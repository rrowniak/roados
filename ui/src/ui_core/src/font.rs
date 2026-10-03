//! Font loading, glyph rasterization and the glyph atlas.
//!
//! FreeType rasterizes glyphs from a font file, antialiased and at exactly the
//! size they will be drawn; this module packs those coverage bitmaps into an
//! atlas the text shader samples. The pipeline is CPU-side and font-agnostic:
//! the renderer uploads the atlas to a GL texture and draws per-glyph quads.
//!
//! The coverage is carried through as rasterized. It was previously turned into
//! a signed distance field here and reconstructed in the shader, which is how
//! text is drawn when it may be scaled; for text drawn at the size it was
//! rasterized for it threw away the sub-pixel edge position and made the
//! rendering look scanned. See [`GlyphCoverage`].
//!
//! Text *shaping* (ligatures, complex scripts, bidirectional text) is not done
//! here. It needs HarfBuzz, whose safe Rust binding exposes no shaping API —
//! only `unsafe` C calls — and the operator declined `unsafe` (see
//! `doc/ui/IMPLEMENTATION_STATE.md`). Glyphs are placed left-to-right by their
//! advance widths, which is correct for Latin text.
//!
//! **Weight is a second face, not a second pass.** A run says which weight it
//! wants ([`FontWeight`]) and the renderer resolves that against the faces it
//! holds ([`FontSet`]); the two weights of one family are two [`Font`]s, because
//! the advance cache is keyed by character and size alone and a shared one would
//! measure bold with regular's widths. Nothing here synthesises weight, blends
//! two copies of a glyph, or re-rasterizes anything at draw time.

use freetype::face::LoadFlag;
use freetype::{Face, Library};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The transparent margin stored around a glyph's ink, in pixels.
///
/// One pixel, and no more than one is needed: the quad drawn for a glyph covers
/// its margin as well as its ink, so the margin's transparent pixels fall on
/// screen pixels of their own and the ink's coverage lands on the texels it was
/// rasterized into. A wider margin would only waste atlas.
const COVERAGE_PAD: u32 = 1;

/// The gap in pixels kept between neighbouring glyphs in the atlas. The text
/// shader filters with `GL_LINEAR`, which samples texels on both sides of the
/// glyph's own rectangle; without a gap a glyph would interpolate against its
/// neighbour's field and gain a halo.
const ATLAS_PAD: u32 = 1;

/// The smallest vertical span worth keeping when an evicted shelf is split, in
/// pixels. Smaller remainders are handed back whole, because a span too short
/// for any glyph would only waste an entry.
const MIN_SHELF_HEIGHT: u32 = 4;

/// An error from loading a font or rasterizing a glyph.
#[derive(Debug)]
pub enum FontError {
    /// FreeType could not load the font.
    Load(String),
    /// A glyph could not be rasterized.
    Glyph(String),
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontError::Load(msg) => write!(f, "font load error: {msg}"),
            FontError::Glyph(msg) => write!(f, "glyph error: {msg}"),
        }
    }
}

impl std::error::Error for FontError {}

/// Converts a float pixel size to the `u32` the FreeType size setter takes.
///
/// There is no `From`/`TryFrom` between `f32` and any unsigned integer in std,
/// so this is the one place a float-to-integer `as` cast is used, for the same
/// reason and with the same guarantee as the renderer's own `f32_to_i32`: the
/// cast is saturating, so a size outside the range clamps instead of wrapping.
fn f32_to_u32(value: f32) -> u32 {
    value as u32
}

/// Returns the Unicode codepoint of `ch` as the `usize` `load_char` takes.
///
/// `u32` has a `From<char>` (a char is a Unicode scalar value, which a `u32`
/// holds), and `usize` has a `TryFrom<u32>` (a `u32` always fits a `usize` on
/// the targets this runs on), so no `as` cast is needed.
fn codepoint(ch: char) -> usize {
    usize::try_from(u32::from(ch)).unwrap_or(0)
}

/// Converts a 26.6 fractional pixel value to pixels.
///
/// FreeType reports advances and metrics in 26.6 fixed point. There is no
/// `From`/`TryFrom` between `i64` and `f32` in std, so this is an `as` cast
/// like the renderer's own; the value is a small non-negative pixel count that
/// `f32` represents exactly.
fn f266_to_pixels(value: i64) -> f32 {
    value as f32 / 64.0
}

/// Advance widths already measured, keyed by character and pixel size.
///
/// This is the reason the text pipeline is not unusably slow. Measuring one
/// advance costs FreeType a `load_char`, which on the dev host measures about
/// 61 µs — not the cost of the arithmetic, but of the glyph load behind it.
/// Laying out a label asks for one advance per character, and the demo
/// re-lays its labels on every frame, so a frame spent roughly 200 ms here and
/// the whole application ran at about 4 frames per second.
///
/// The cache is shared by every clone, because a clone shares the FreeType face
/// and a private cache would be a cache per label — and every label in a frame
/// asks the same face for the same characters, which is the repetition worth
/// catching. It is keyed by the *rounded* pixel size rather than the `f32`,
/// because that rounded value is the size the face is actually set to: two
/// sizes that round to the same pixel count produce the same advance, so
/// keying on the float would store the same answer twice.
///
/// Separate from [`Font`] so its behaviour can be tested without a font file,
/// which a unit test may not open.
#[derive(Clone, Default)]
struct AdvanceCache(Rc<RefCell<HashMap<(char, u32), f32>>>);

impl AdvanceCache {
    /// Returns the advance measured for `ch` at `pixels`, if it has been.
    fn get(&self, ch: char, pixels: u32) -> Option<f32> {
        self.0.borrow().get(&(ch, pixels)).copied()
    }

    /// Records the advance measured for `ch` at `pixels`.
    fn set(&self, ch: char, pixels: u32, advance: f32) {
        self.0.borrow_mut().insert((ch, pixels), advance);
    }

    /// Returns how many distinct measurements are held.
    ///
    /// Only the tests ask: the count is what shows that a repeated measurement
    /// did not become a second entry.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.0.borrow().len()
    }
}

/// A loaded font face.
///
/// The face is cheap to clone (it shares the underlying FreeType face), so a
/// label clones it to keep its own handle. The advance cache is shared across
/// those clones too, so a character is measured once per size however many
/// labels ask for it.
///
/// **One face, not one family.** Two weights of one family are two `Font`s, and
/// not one `Font` with two faces in it: the advance cache is keyed by character
/// and pixel size, so a second face sharing this one would be measured with the
/// *first* face's advances. [`FontSet`] holds one of these per [`FontWeight`].
#[derive(Clone)]
pub struct Font {
    face: Face,
    advances: AdvanceCache,
}

impl Font {
    /// Loads a font from a file path.
    ///
    /// # Errors
    ///
    /// Returns [`FontError::Load`] if FreeType cannot open the file.
    pub fn from_path(path: &str) -> Result<Self, FontError> {
        let library = Library::init().map_err(|e| FontError::Load(e.to_string()))?;
        let face = library
            .new_face(path, 0)
            .map_err(|e| FontError::Load(e.to_string()))?;
        Ok(Font {
            face,
            advances: AdvanceCache::default(),
        })
    }

    /// Returns the pixel count `size` rounds to, which is the size FreeType is
    /// actually set to and therefore the size a measurement belongs to.
    fn pixels(&self, size: f32) -> u32 {
        let pixels = f32_to_u32(size.max(0.0));
        // A size of zero would make FreeType reject every glyph, so clamp to a
        // single pixel; an empty label has nothing to draw anyway.
        pixels.max(1)
    }

    /// Sets the face's pixel size, so glyphs rasterize at `size` pixels.
    fn set_size(&self, size: f32) {
        let pixels = self.pixels(size);
        let _ = self.face.set_pixel_sizes(pixels, pixels);
    }

    /// Returns the advance width of `ch` at `size` pixels, in pixels.
    ///
    /// The advance is how far the pen moves after drawing the glyph, so a run
    /// of characters is laid out by summing advances.
    ///
    /// A measured advance is kept in the face's shared cache, so a character
    /// is only ever paid for once per pixel size however many labels ask for
    /// it. This is the hottest call in the text pipeline and was, uncached, the
    /// reason the demo ran at about 4 frames per second.
    pub fn advance(&self, ch: char, size: f32) -> f32 {
        let pixels = self.pixels(size);
        if let Some(cached) = self.advances.get(ch, pixels) {
            return cached;
        }
        self.set_size(size);
        if self
            .face
            .load_char(codepoint(ch), LoadFlag::DEFAULT)
            .is_err()
        {
            return 0.0;
        }
        let glyph = self.face.glyph();
        let advance = f266_to_pixels(glyph.advance().x);
        self.advances.set(ch, pixels, advance);
        advance
    }

    /// Returns the distance from a line's top edge to the baseline at `size`
    /// pixels.
    ///
    /// This is what turns a line box into a baseline: a draw command positions
    /// the top of the line, and the font decides where inside it the glyphs sit.
    /// A face that reports no metrics falls back to `size` itself, which places
    /// the baseline at the bottom of the line — low, but never off the top.
    pub fn ascent(&self, size: f32) -> f32 {
        self.set_size(size);
        self.face
            .size_metrics()
            .map_or(size, |metrics| f266_to_pixels(metrics.ascender))
    }

    /// Returns the recommended distance between consecutive baselines at `size`
    /// pixels — the face's own line height, not the sum of ascent and descent.
    pub fn line_height(&self, size: f32) -> f32 {
        self.set_size(size);
        self.face
            .size_metrics()
            .map_or(size, |metrics| f266_to_pixels(metrics.height))
    }

    /// Measures a run of text at `size` pixels: the sum of its advances.
    pub fn measure(&self, text: &str, size: f32) -> f32 {
        self.set_size(size);
        text.chars().map(|ch| self.advance(ch, size)).sum()
    }

    /// Rasterizes `ch` at `size` pixels into a top-down coverage bitmap.
    ///
    /// Returns `None` for a character the font has no glyph for (it rasterizes
    /// to nothing). The bitmap is 8-bit coverage: 0 is outside the glyph, 255 is
    /// fully covered.
    pub fn rasterize(&self, ch: char, size: f32) -> Option<GlyphBitmap> {
        self.set_size(size);
        self.face.load_char(codepoint(ch), LoadFlag::RENDER).ok()?;
        let glyph = self.face.glyph();
        let bitmap = glyph.bitmap();
        let width = bitmap.width();
        let rows = bitmap.rows();
        if width <= 0 || rows <= 0 {
            return None;
        }
        let pitch = bitmap.pitch();
        let buffer = bitmap.buffer();
        let w = usize::try_from(width).unwrap_or(0);
        let h = usize::try_from(rows).unwrap_or(0);
        let stride = usize::try_from(pitch.unsigned_abs()).unwrap_or(w);
        if w == 0 || h == 0 {
            return None;
        }
        // FreeType stores rows bottom-up for a negative pitch and top-down for
        // a positive one; the atlas is always top-down, so flip when needed.
        let mut pixels = vec![0u8; w * h];
        for row in 0..h {
            let src = row * stride;
            let dst = if pitch >= 0 { row } else { h - 1 - row };
            let dst = dst * w;
            let end = src + w;
            if end <= buffer.len() && dst + w <= pixels.len() {
                pixels[dst..dst + w].copy_from_slice(&buffer[src..end]);
            }
        }
        Some(GlyphBitmap {
            // FreeType reports the extents as `i32`, and both are known to be
            // positive here, so the `u32` the bitmap keeps is a checked
            // conversion rather than a cast that could wrap.
            width: u32::try_from(width).unwrap_or(0),
            height: u32::try_from(rows).unwrap_or(0),
            bearing_x: glyph.bitmap_left(),
            bearing_y: glyph.bitmap_top(),
            advance: f266_to_pixels(glyph.advance().x),
            pixels,
        })
    }
}

/// The number of weights a [`FontSet`] holds a face for.
///
/// A constant rather than a growing map because the set of weights is closed: the
/// two variants of [`FontWeight`] are the whole of what a draw command can ask
/// for, and an open set would mean a request that names a face the renderer has
/// no slot for — which is the case this design does not have to handle at all.
pub const FACE_COUNT: usize = 2;

/// The slot [`FontWeight::Regular`] lives in, and the slot a request for a
/// weight with no face of its own falls back to.
///
/// It is [`FontWeight::Regular`]'s own slot rather than a second number, so the
/// fallback cannot drift away from the weight it falls back *to*: there is one
/// definition and it names the variant.
const REGULAR_SLOT: usize = FontWeight::Regular.slot();

/// The weight a text run asks for, and the only thing a draw command says about
/// the face it is drawn with.
///
/// Two variants and no more, and that is what makes the whole of the weight
/// mechanism total: [`FontSet`] holds one face per variant, so every value here
/// names a slot that exists. There is no string to miss, no integer to range-
/// check at draw time and therefore nothing to report as an error — see
/// [`resolve_slot`] for what happens when the weight asked for is not loaded.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FontWeight {
    /// The face every ordinary run is drawn with.
    ///
    /// The **default**, so a command that says nothing about weight is the run it
    /// was before there was a second face: a renderer holding one font draws and
    /// measures exactly what it always did, and `Painter::text` says so by
    /// construction rather than by a rule somewhere else.
    #[default]
    Regular,
    /// The heavier face a heading is drawn with.
    ///
    /// A real second face, rasterized from a second file. Not a synthetic
    /// double-strike of the regular one: two overlapping copies of a glyph
    /// thicken it *and* widen it at every join, which is not what bold is, and
    /// the operator chose the real face over it.
    Bold,
}

impl FontWeight {
    /// Every weight, in slot order.
    ///
    /// A public list rather than a private one so a caller that has to iterate
    /// the weights — a test asserting every one of them resolves, a caller
    /// pre-loading a face per weight — does not write the list out a second time
    /// and get it out of step with the enum.
    pub const ALL: [FontWeight; FACE_COUNT] = [FontWeight::Regular, FontWeight::Bold];

    /// The slot this weight's face lives in, which is the index into a
    /// [`FontSet`]'s faces and ids.
    const fn slot(self) -> usize {
        match self {
            // 0 rather than `REGULAR_SLOT`, which is defined from this function
            // and cannot therefore be used inside it.
            FontWeight::Regular => 0,
            FontWeight::Bold => 1,
        }
    }
}

/// The slot a run asking for `weight` is drawn with: the weight's own slot when a
/// face is installed there, and the regular slot otherwise.
///
/// **This is the whole of the fallback rule, and it is in one place because two
/// places would be two rules.** [`FontSet::resolve`] reads it to find a face, and
/// the renderer's text pass reads the set's answer; neither decides anything of
/// its own.
///
/// The fallback is **the regular face**, and it is deliberate: a heading that
/// asks for bold on a renderer that was given one font must still be *drawn*,
/// and drawn in the regular weight is a picture a reader can read, while not
/// drawn is a hole in the layout. There is no `Result` because there is nothing
/// to report — a weight with no face installed is an ordinary state, not a
/// failure, and a caller cannot act on an error except by falling back to exactly
/// what this does.
///
/// "The regular face" and not "some face": a set holding **only** a bold face
/// draws its bold runs and draws **no regular ones**, because a regular run
/// quietly set in bold would be a surprise with nothing to point at it, whereas a
/// regular run that is not drawn is a hole the caller can see. Every application
/// calls [`crate::render::Renderer::set_font`] — it is the one line a program
/// needs to draw text at all — so a set without a regular face is a caller that
/// deliberately has none.
///
/// Total: every one of [`FontWeight::ALL`] resolves without panicking whether or
/// not either slot is filled. `None` means the set has no face for the weight
/// asked for *and* no regular face to fall back to, which is the one case with
/// nothing to draw with.
///
/// # Examples
///
/// ```
/// use ui_core::font::{resolve_slot, FontWeight};
///
/// // Nothing installed: no run can be drawn at all.
/// let empty: [Option<u8>; 2] = [None, None];
/// assert_eq!(resolve_slot(&empty, FontWeight::Regular), None);
/// assert_eq!(resolve_slot(&empty, FontWeight::Bold), None);
///
/// // Only the regular face, which is what a renderer given one font holds: both
/// // weights draw with it, and neither is dropped.
/// let regular_only: [Option<u8>; 2] = [Some(0), None];
/// assert_eq!(resolve_slot(&regular_only, FontWeight::Regular), Some(0));
/// assert_eq!(resolve_slot(&regular_only, FontWeight::Bold), Some(0));
///
/// // Both installed: each weight gets the face it asked for.
/// let both: [Option<u8>; 2] = [Some(0), Some(1)];
/// assert_eq!(resolve_slot(&both, FontWeight::Regular), Some(0));
/// assert_eq!(resolve_slot(&both, FontWeight::Bold), Some(1));
/// ```
#[must_use]
pub fn resolve_slot<F>(faces: &[Option<F>; FACE_COUNT], weight: FontWeight) -> Option<usize> {
    if faces[weight.slot()].is_some() {
        Some(weight.slot())
    } else if faces[REGULAR_SLOT].is_some() {
        Some(REGULAR_SLOT)
    } else {
        None
    }
}

/// One installed face's identity in the glyph atlas.
///
/// The atlas is shared by every face the renderer holds, so a glyph's cache key
/// has to say which face rasterized it — see `GlyphKey`, which is where the
/// atlas's cache key spells it out. This is that third
/// part of the key, and it is deliberately **not** the weight: it is handed out by
/// [`FontSet::set`] every time a face is installed, so a renderer that swaps its
/// bold file cannot serve the glyphs of the file it no longer holds. A newtype
/// rather than a bare `u32` because the two are not interchangeable, and this is
/// the one value whose confusion draws a letter in the wrong weight with no error
/// anywhere.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FaceId(u32);

impl FaceId {
    /// Returns the number this id wraps.
    ///
    /// For a caller keeping an id beside its own bookkeeping; nothing in this
    /// crate needs the number rather than the id.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// One face as the glyph atlas needs it: the [`Font`] to rasterize from, and the
/// [`FaceId`] its glyphs are cached under.
///
/// A pair rather than two arguments to [`GlyphAtlas::get_or_insert`] because the
/// two must not be able to disagree: the atlas keys on the id and rasterizes from
/// the font, so a caller that passed one face's id with another face's outlines
/// would cache a glyph under the wrong key and draw the wrong weight for every
/// later run — a defect invisible until two weights are on screen at once. There
/// is exactly one way to make this value, [`FontSet::resolve`].
#[derive(Clone, Copy)]
pub struct FaceRef<'a> {
    font: &'a Font,
    id: FaceId,
}

impl std::fmt::Debug for FaceRef<'_> {
    /// Prints the face's id and nothing else.
    ///
    /// Not derived because `Font` is not `Debug` — it holds a FreeType handle,
    /// which has nothing to print that means anything — and a `Debug` that could
    /// not be derived would have to leave the field out anyway. The id is the half
    /// that says which face this is.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FaceRef")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<'a> FaceRef<'a> {
    /// Returns the face to rasterize from.
    #[must_use]
    pub fn font(&self) -> &'a Font {
        self.font
    }

    /// Returns the identity this face's glyphs are cached under.
    #[must_use]
    pub fn id(&self) -> FaceId {
        self.id
    }
}

/// The identities the faces of a [`FontSet`] are cached under, one per slot.
///
/// Separate from `FontSet` for the reason [`AdvanceCache`] is: a `Font` cannot be
/// made without a font file, which a unit test may not open, and the rule worth
/// testing here — **every install is a new identity**, so replacing a weight
/// cannot leave the atlas serving the glyphs of the file that has just been
/// dropped — is a rule about ids and not about FreeType.
#[derive(Clone, Debug)]
struct FaceIds {
    /// The id the next installed face is given.
    next: u32,
    /// The id each slot holds, which is meaningless for a slot nothing has been
    /// installed in and is never read for one.
    slots: [FaceId; FACE_COUNT],
}

impl FaceIds {
    fn new() -> Self {
        FaceIds {
            next: 0,
            slots: [FaceId(0), FaceId(0)],
        }
    }

    /// Returns an identity no face in this set has held.
    ///
    /// A counter rather than the slot's index because the slot is not the
    /// identity: a face installed in one slot may be replaced by a different file
    /// later, and the glyphs of the first one must not be found for the second.
    /// It only has to differ from the identities already handed out, so it is
    /// never read back and wrapping after `u32::MAX` installs is not a hazard any
    /// process will reach.
    fn issue(&mut self) -> FaceId {
        let id = FaceId(self.next);
        self.next = self.next.wrapping_add(1);
        id
    }

    /// Records `id` as the identity of the face in `slot`.
    fn install(&mut self, slot: usize, id: FaceId) {
        self.slots[slot] = id;
    }

    /// Returns the identity of the face installed in `slot`.
    fn get(&self, slot: usize) -> FaceId {
        self.slots[slot]
    }
}

/// The faces a renderer draws text with: one per [`FontWeight`], and the
/// identity each is cached under in the glyph atlas.
///
/// **One [`Font`] per weight, not one font with two faces inside it.** The
/// advance cache is keyed by character and pixel size, so a second face sharing
/// one `Font` would be measured with the *first* face's advances — bold drawn
/// with regular spacing, which is a defect in the opposite direction from bold
/// drawn with regular ink, and one that only shows where two runs of different
/// weight sit side by side. Two `Font`s means two caches, each holding its own
/// face's measurements, and a face nothing asks for is never measured at all.
///
/// The cost of holding a face nothing uses is two handles: a FreeType library
/// and a face. Neither rasterizes anything until a run asks, so a renderer that
/// installs only the regular face measures and draws exactly what it did before,
/// and [`resolve_slot`] never looks at a slot it does not need.
#[derive(Clone)]
pub struct FontSet {
    faces: [Option<Font>; FACE_COUNT],
    /// The identity each slot's glyphs are cached under, handed out by `set`.
    ids: FaceIds,
}

impl Default for FontSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FontSet {
    /// Creates a set holding no face, which draws no text.
    #[must_use]
    pub fn new() -> Self {
        FontSet {
            faces: [None, None],
            ids: FaceIds::new(),
        }
    }

    /// Installs `font` as the face for `weight`, replacing whatever was there.
    ///
    /// The face is given a **fresh** [`FaceId`] every time it is installed,
    /// including the first: the id is *this file's* identity in the atlas, not
    /// the slot's name, so replacing a weight re-rasterizes rather than serving
    /// glyphs rasterized from the file that has just been dropped. Which number
    /// that is depends on the order the faces were installed in, and nothing
    /// depends on the number — the atlas only ever compares ids.
    pub fn set(&mut self, weight: FontWeight, font: Font) {
        let id = self.ids.issue();
        let slot = weight.slot();
        self.faces[slot] = Some(font);
        self.ids.install(slot, id);
    }

    /// Returns whether the set holds no face at all.
    ///
    /// The renderer's text pass asks this before it walks a batch, because a
    /// renderer with no font draws no text — which is what it did before there
    /// were two weights, and asking here is what keeps that path as free as it
    /// was rather than free modulo a per-command resolution.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.faces.iter().all(Option::is_none)
    }

    /// Returns the face a run asking for `weight` is drawn with.
    ///
    /// `None` only for a set with no face at all; a weight with no face of its
    /// own resolves to the regular one, per [`resolve_slot`].
    #[must_use]
    pub fn resolve(&self, weight: FontWeight) -> Option<FaceRef<'_>> {
        let slot = resolve_slot(&self.faces, weight)?;
        let font = self.faces[slot].as_ref()?;
        Some(FaceRef {
            font,
            id: self.ids.get(slot),
        })
    }
}

/// A rasterized glyph: a top-down coverage bitmap and its metrics, in pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphBitmap {
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Horizontal bearing: offset from the pen to the bitmap's left edge.
    pub bearing_x: i32,
    /// Vertical bearing: offset from the baseline to the bitmap's top edge.
    pub bearing_y: i32,
    /// Advance width: how far the pen moves after this glyph, in pixels.
    pub advance: f32,
    /// Coverage pixels, top-down, `width * height` of them.
    pub pixels: Vec<u8>,
}

/// One glyph's coverage, as the atlas stores it and the text shader reads it.
///
/// This is FreeType's own 8-bit antialiased coverage, carried through unchanged.
/// The module used to build a signed distance field here instead, and recover a
/// coverage from it in the shader with a `smoothstep`; that round trip is what
/// made the text look scanned rather than drawn, and the reason is one line:
/// **a distance field has to binarize.** Deciding what is inside the glyph
/// throws away the sub-pixel position of the edge that FreeType rasterized, so
/// the field's 50% contour lands on whole pixels, and an edge on a pixel
/// boundary has no half-covered pixel to sit in. It renders as a hard aliased
/// step. Recovering the lost precision by supersampling the threshold was tried
/// and made it worse — it dilates the glyph and leaves a halo of low-alpha
/// pixels around every one — because the distance transform then measures to a
/// mask that has itself grown.
///
/// Carrying the coverage needs none of that. FreeType already antialiases, with
/// sub-pixel accuracy, and at exactly the size the glyph will be drawn; the
/// field threw that away and rebuilt a worse approximation of it. What is given
/// up is resolution independence: a magnified glyph will blur, because there is
/// no longer a scale-free description of the edge. Nothing magnifies text today
/// — the atlas is keyed by size, so a new size re-rasterizes — and an
/// unblurred edge at the size actually drawn is worth more than the ability to
/// scale one that is not.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphCoverage {
    /// Coverage width in pixels, including the transparent margin.
    pub width: u32,
    /// Coverage height in pixels, including the transparent margin.
    pub height: u32,
    /// Horizontal bearing: offset from the pen to the coverage's left edge.
    pub bearing_x: i32,
    /// Vertical bearing: offset from the baseline to the coverage's top edge.
    pub bearing_y: i32,
    /// Advance width in pixels.
    pub advance: f32,
    /// Coverage, top-down, `width * height` of them. 255 is solid ink.
    pub coverage: Vec<u8>,
}

/// Wraps a glyph's coverage bitmap as the atlas stores it, with no scaling or
/// filtering of the values.
///
/// # Examples
///
/// ```
/// use ui_core::font::{make_coverage, GlyphBitmap};
///
/// let bitmap = GlyphBitmap {
///     width: 2,
///     height: 1,
///     bearing_x: 0,
///     bearing_y: 1,
///     advance: 2.0,
///     // A half-covered pixel: the edge runs through the middle of it, which
///     // is exactly the sub-pixel information a distance field would discard.
///     pixels: vec![255, 128],
/// };
/// let coverage = make_coverage(&bitmap);
/// assert_eq!(coverage.coverage, vec![255, 128], "carried through unchanged");
/// assert_eq!(coverage.width, 2);
/// ```
pub fn make_coverage(bitmap: &GlyphBitmap) -> GlyphCoverage {
    GlyphCoverage {
        width: bitmap.width,
        height: bitmap.height,
        bearing_x: bitmap.bearing_x,
        bearing_y: bitmap.bearing_y,
        advance: bitmap.advance,
        coverage: bitmap.pixels.clone(),
    }
}

/// Returns a copy of `bitmap` with `pad` transparent pixels on every side.
///
/// A glyph's bitmap from FreeType is the **tight** box around its ink, so its
/// border pixels are edge pixels with partial coverage. A distance field needs
/// room to develop on both sides of that edge, and there is none: the field is
/// cut off mid-gradient at the bitmap border, so it never reaches the flat
/// "fully outside" value the shader's `smoothstep` needs to draw a crisp edge.
/// Worse, the quad drawn for the glyph is exactly the bitmap, so the truncated
/// border is also the quad's geometric edge, and linear filtering blends it
/// with the zero-valued atlas padding beside it. Every stroke then loses its
/// outer half-pixel and the text reads soft and slightly eaten.
///
/// Padding fixes both: the field develops across the padding and saturates at
/// the new border, and the cut-off edge moves inside the quad where the shader
/// can resolve it.
///
/// The bearings move with the ink: the bitmap's left edge is `pad` further
/// left of the pen (`bearing_x -= pad`) and its top `pad` higher above the
/// baseline (`bearing_y += pad`), so a padded glyph lands in the same place at
/// the same size, with the padding falling outside the ink.
pub fn pad_bitmap(bitmap: &GlyphBitmap, pad: u32) -> GlyphBitmap {
    let p = usize::try_from(pad).unwrap_or(0);
    let src_w = usize::try_from(bitmap.width).unwrap_or(0);
    let src_h = usize::try_from(bitmap.height).unwrap_or(0);
    if p == 0 {
        return bitmap.clone();
    }
    let w = src_w + 2 * p;
    let h = src_h + 2 * p;
    let mut pixels = vec![0u8; w * h];
    for y in 0..src_h {
        for x in 0..src_w {
            let Some(&coverage) = bitmap.pixels.get(y * src_w + x) else {
                continue;
            };
            let dst = (y + p) * w + (x + p);
            if let Some(slot) = pixels.get_mut(dst) {
                *slot = coverage;
            }
        }
    }
    let pad_i32 = i32::try_from(pad).unwrap_or(0);
    GlyphBitmap {
        width: u32::try_from(w).unwrap_or(0),
        height: u32::try_from(h).unwrap_or(0),
        bearing_x: bitmap.bearing_x.saturating_sub(pad_i32),
        bearing_y: bitmap.bearing_y.saturating_add(pad_i32),
        advance: bitmap.advance,
        pixels,
    }
}

/// Where a glyph lives in the atlas, in UV coordinates and pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphPlacement {
    /// Left edge of the glyph in the atlas, in UV.
    pub u0: f32,
    /// Top edge of the glyph in the atlas, in UV.
    pub v0: f32,
    /// Right edge of the glyph in the atlas, in UV.
    pub u1: f32,
    /// Bottom edge of the glyph in the atlas, in UV.
    pub v1: f32,
    /// The top edge of the glyph's shelf in the atlas, in pixels. Used to
    /// track the shelf for LRU eviction.
    pub row_y: u32,
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Horizontal bearing: offset from the pen to the bitmap's left edge.
    pub bearing_x: i32,
    /// Vertical bearing: offset from the baseline to the bitmap's top edge.
    pub bearing_y: i32,
    /// Advance width in pixels.
    pub advance: f32,
}

/// The key identifying a glyph in the atlas: the character, its pixel size, and
/// the face that rasterized it.
///
/// **The face is part of the key because the atlas is shared by every face the
/// renderer holds.** Everything the atlas stores about a glyph is that face's own
/// — the coverage bitmap, the bearings that place it against the pen, and the
/// advance the next glyph starts after — so a key of character and size alone
/// would hand a bold run the *regular* glyph's quad: one letter in the wrong
/// weight, drawn from the right UVs, with no error anywhere. `face` is the
/// [`FaceId`] rather than the [`FontWeight`] because it is the identity of the
/// file that was rasterized, which is what changes when a weight is reinstalled
/// and is not what the run asked for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct GlyphKey {
    ch: char,
    size: u32,
    face: FaceId,
}

/// One shelf of the atlas: a horizontal row of glyphs with a shared height.
struct Row {
    /// The row's top edge in the atlas.
    y: u32,
    /// The row's height: the tallest glyph it holds.
    height: u32,
    /// The first free column, past the last glyph and its padding.
    x: u32,
    /// The glyphs in this row, for eviction.
    glyphs: Vec<GlyphKey>,
}

/// A free vertical span left behind by an evicted shelf, in pixels: `y` and
/// `height`.
type FreeSpan = (u32, u32);

/// The glyph atlas: a square texture of glyph coverage, for every face the
/// renderer holds.
///
/// Glyphs are packed into shelves (rows). When the atlas fills up, the least
/// recently used shelf is evicted — its glyphs are dropped, its pixels cleared
/// and its vertical span returned to the free list for reuse — which keeps the
/// hot glyphs resident. The atlas is CPU-side; the renderer uploads
/// [`GlyphAtlas::take_dirty_pixels`] to a GL texture.
pub struct GlyphAtlas {
    size: u32,
    pixels: Vec<u8>,
    /// Shelves in recency order: the front is the least recently used.
    rows: Vec<Row>,
    /// Vertical spans freed by eviction, in pixels.
    free: Vec<FreeSpan>,
    /// The next unused pixel row, below every shelf ever allocated.
    next_y: u32,
    glyphs: HashMap<GlyphKey, GlyphPlacement>,
    dirty: bool,
}

impl GlyphAtlas {
    /// Creates an empty `size` × `size` atlas.
    #[must_use]
    pub fn new(size: u32) -> Self {
        GlyphAtlas {
            size,
            pixels: vec![
                0;
                usize::try_from(size).unwrap_or(0) * usize::try_from(size).unwrap_or(0)
            ],
            rows: Vec::new(),
            free: Vec::new(),
            next_y: 0,
            glyphs: HashMap::new(),
            dirty: false,
        }
    }

    /// Returns the atlas texture size in pixels.
    #[must_use]
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Returns the atlas pixel data, ready to upload to a GL texture.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns the pixel data if it changed since the last call, and marks it
    /// clean. `None` means the caller's copy is still current.
    ///
    /// The renderer calls this after rasterizing a batch's glyphs and before
    /// drawing it: taking it earlier would upload a texture without the glyphs
    /// that batch is about to sample.
    pub fn take_dirty_pixels(&mut self) -> Option<&[u8]> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        Some(&self.pixels)
    }

    /// The key `ch` at `size` is cached under in `face`: the character, the
    /// pixel count the size rounds to, and the face's identity.
    ///
    /// One function builds every key, so a cache that cannot tell two faces apart
    /// would have to be wrong here rather than in one lookup: the size is the
    /// rounded pixel count because that is the size the face is actually set to,
    /// and the face is part of the key because the atlas is shared by all of them.
    fn key_for(ch: char, size: f32, face: FaceId) -> GlyphKey {
        GlyphKey {
            ch,
            size: f32_to_u32(size.max(0.0)).max(1),
            face,
        }
    }

    /// Returns the placement of `ch` at `size` in `face`, rasterizing and
    /// packing it if it is not already in the atlas.
    ///
    /// The same character at the same size in a different face is a **different**
    /// glyph — different coverage, different bearings, different advance — so it
    /// is packed separately and looked up separately; see `GlyphKey`.
    ///
    /// Returns `None` if the font has no glyph for `ch`.
    pub fn get_or_insert(
        &mut self,
        ch: char,
        size: f32,
        face: FaceRef<'_>,
    ) -> Option<GlyphPlacement> {
        let key = Self::key_for(ch, size, face.id());
        if let Some(&placement) = self.glyphs.get(&key) {
            self.touch_row(placement.row_y);
            return Some(placement);
        }
        let bitmap = face.font().rasterize(ch, size)?;
        let padded = pad_bitmap(&bitmap, COVERAGE_PAD);
        let glyph = make_coverage(&padded);
        let (x, y) = self.allocate(glyph.width, glyph.height)?;
        self.blit(x, y, &glyph);
        let placement = GlyphPlacement {
            u0: u32_to_f32(x) / u32_to_f32(self.size),
            v0: u32_to_f32(y) / u32_to_f32(self.size),
            u1: u32_to_f32(x + glyph.width) / u32_to_f32(self.size),
            v1: u32_to_f32(y + glyph.height) / u32_to_f32(self.size),
            row_y: y,
            width: glyph.width,
            height: glyph.height,
            bearing_x: glyph.bearing_x,
            bearing_y: glyph.bearing_y,
            advance: glyph.advance,
        };
        self.glyphs.insert(key, placement);
        if let Some(row) = self.row_at_mut(y) {
            row.glyphs.push(key);
        }
        Some(placement)
    }

    /// Finds a shelf for a `w` × `h` glyph, evicting the least recently used
    /// row until one fits, and returns its top-left corner.
    fn allocate(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        loop {
            if let Some(found) = self.fit(w, h) {
                return Some(found);
            }
            self.evict_lru_row()?;
        }
    }

    /// Returns the position of a `w` × `h` glyph if a shelf can take it.
    ///
    /// Tries, in order: a shelf with room to spare, a span freed by eviction,
    /// then unused space at the bottom. Each glyph is separated from its
    /// neighbours by [`ATLAS_PAD`] pixels, so that linear filtering never
    /// interpolates a glyph's field into the next one.
    fn fit(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if w > self.size || h > self.size || w + ATLAS_PAD > self.size {
            return None;
        }
        for row in self.rows.iter_mut() {
            if row.height >= h && row.x + w + ATLAS_PAD <= self.size {
                let x = row.x;
                let y = row.y;
                row.x += w + ATLAS_PAD;
                return Some((x, y));
            }
        }
        if let Some(index) = self.free.iter().position(|&(_, span)| span >= h) {
            let (y, span) = self.free.swap_remove(index);
            if span - h >= MIN_SHELF_HEIGHT {
                self.free.push((y + h, span - h));
            }
            self.rows.push(Row {
                y,
                height: h,
                x: w + ATLAS_PAD,
                glyphs: Vec::new(),
            });
            return Some((0, y));
        }
        if self.next_y + h > self.size {
            return None;
        }
        let y = self.next_y;
        self.next_y += h;
        self.rows.push(Row {
            y,
            height: h,
            x: w + ATLAS_PAD,
            glyphs: Vec::new(),
        });
        Some((0, y))
    }

    /// Evicts the least recently used shelf — the front of `rows`, which
    /// [`touch_row`] keeps ordered oldest-first — dropping its glyphs, clearing
    /// its pixels and returning its span to the free list.
    fn evict_lru_row(&mut self) -> Option<()> {
        if self.rows.is_empty() {
            return None;
        }
        let row = self.rows.remove(0);
        for key in &row.glyphs {
            self.glyphs.remove(key);
        }
        self.clear(row.y, row.height);
        self.free.push((row.y, row.height));
        Some(())
    }

    /// Moves the shelf at `row_y` to the back of `rows`, marking it most
    /// recently used.
    fn touch_row(&mut self, row_y: u32) {
        if let Some(index) = self.rows.iter().position(|row| row.y == row_y) {
            let row = self.rows.remove(index);
            self.rows.push(row);
        }
    }

    /// Returns the shelf at `row_y`, if any.
    fn row_at_mut(&mut self, row_y: u32) -> Option<&mut Row> {
        self.rows.iter_mut().find(|row| row.y == row_y)
    }

    /// Zeroes `height` pixel rows starting at `y`, so an evicted shelf leaves
    /// no field behind for a later glyph to sample.
    fn clear(&mut self, y: u32, height: u32) {
        let size = usize::try_from(self.size).unwrap_or(0);
        let first = usize::try_from(y).unwrap_or(0).min(size);
        let last = usize::try_from(y)
            .unwrap_or(0)
            .saturating_add(usize::try_from(height).unwrap_or(0))
            .min(size);
        for row in first..last {
            let start = row * size;
            self.pixels[start..start + size].fill(0);
        }
        self.dirty = true;
    }

    /// Copies a glyph's coverage into the atlas at `(x, y)`.
    fn blit(&mut self, x: u32, y: u32, glyph: &GlyphCoverage) {
        let w = usize::try_from(glyph.width).unwrap_or(0);
        let h = usize::try_from(glyph.height).unwrap_or(0);
        let size = usize::try_from(self.size).unwrap_or(0);
        for row in 0..h {
            let dst_y = usize::try_from(y).unwrap_or(0) + row;
            if dst_y >= size {
                break;
            }
            let src = row * w;
            let dst = dst_y * size + usize::try_from(x).unwrap_or(0);
            if dst + w <= self.pixels.len() && src + w <= glyph.coverage.len() {
                self.pixels[dst..dst + w].copy_from_slice(&glyph.coverage[src..src + w]);
            }
        }
        self.dirty = true;
    }
}

/// Converts a `u32` to `f32` for UV math.
///
/// `f32` has no `From<u32>` in std — its `From` impls stop at 16-bit integers
/// — so this is an `as` cast like the renderer's `u32_to_f32`. It is
/// well-defined for every `u32`: the result rounds to the nearest `f32`.
fn u32_to_f32(value: u32) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3×3 bitmap with the centre pixel set, and an edge pixel at partial
    /// coverage so the sub-pixel information is visible to the assertions.
    fn edge_bitmap() -> GlyphBitmap {
        GlyphBitmap {
            width: 3,
            height: 3,
            bearing_x: 0,
            bearing_y: 3,
            advance: 3.0,
            pixels: vec![0, 0, 0, 0, 255, 0, 0, 96, 0],
        }
    }

    #[test]
    fn coverage_is_carried_through_unchanged() {
        // The whole point of storing coverage rather than a distance field: a
        // half-covered pixel stays half covered. A distance field had to
        // binarize to be built, which is what made the text look scanned.
        let coverage = make_coverage(&edge_bitmap());
        assert_eq!(coverage.width, 3);
        assert_eq!(coverage.height, 3);
        assert_eq!(coverage.coverage, edge_bitmap().pixels);
    }

    #[test]
    fn a_partly_covered_pixel_is_not_rounded_to_whole_or_empty() {
        let coverage = make_coverage(&edge_bitmap());
        assert_eq!(
            coverage.coverage[7], 96,
            "an edge pixel between 1 and 254 must survive as itself, not become \
             0 or 255 — that rounding is precisely the defect"
        );
        assert!(coverage.coverage[7] > 0 && coverage.coverage[7] < 255);
    }

    #[test]
    fn coverage_carries_the_metrics_through() {
        let coverage = make_coverage(&edge_bitmap());
        assert_eq!(coverage.bearing_x, 0);
        assert_eq!(coverage.bearing_y, 3);
        assert_eq!(coverage.advance, 3.0);
    }

    #[test]
    fn an_empty_bitmap_produces_no_coverage() {
        let bitmap = GlyphBitmap {
            width: 0,
            height: 0,
            bearing_x: 0,
            bearing_y: 0,
            advance: 0.0,
            pixels: Vec::new(),
        };
        assert!(make_coverage(&bitmap).coverage.is_empty());
    }

    #[test]
    fn atlas_starts_empty() {
        // A real font is needed to rasterize, so this checks the atlas
        // geometry directly: a `size`×`size` texture of zeroed coverage.
        let atlas = GlyphAtlas::new(64);
        assert_eq!(atlas.size(), 64);
        assert_eq!(atlas.pixels().len(), 64 * 64);
        assert!(atlas.pixels().iter().all(|&p| p == 0));
    }

    #[test]
    fn f32_to_u32_clamps_to_the_u32_range() {
        assert_eq!(f32_to_u32(0.0), 0);
        assert_eq!(f32_to_u32(16.0), 16);
        assert_eq!(f32_to_u32(-1.0), 0);
        // 4 billion fits a u32; 5 billion saturates to u32::MAX.
        assert_eq!(f32_to_u32(4_000_000_000.0), 4_000_000_000);
        assert_eq!(f32_to_u32(5_000_000_000.0), u32::MAX);
    }

    #[test]
    fn u32_to_f32_roundtrips_small_values() {
        assert_eq!(u32_to_f32(0), 0.0);
        assert_eq!(u32_to_f32(1024), 1024.0);
    }
}

#[cfg(test)]
mod atlas_tests {
    use super::*;

    /// A glyph's coverage of `w` × `h` at the origin, fully covered — the
    /// tests here are about where the atlas puts a glyph, not what is in it.
    fn coverage(w: u32, h: u32) -> GlyphCoverage {
        GlyphCoverage {
            width: w,
            height: h,
            bearing_x: 0,
            bearing_y: 0,
            advance: 0.0,
            coverage: vec![255; usize::try_from(w).unwrap_or(0) * usize::try_from(h).unwrap_or(0)],
        }
    }

    #[test]
    fn a_glyph_is_separated_from_its_neighbour_by_the_padding() {
        let mut atlas = GlyphAtlas::new(64);
        let (x0, _) = atlas.allocate(8, 8).unwrap();
        let (x1, _) = atlas.allocate(8, 8).unwrap();
        assert_eq!(x1, x0 + 8 + ATLAS_PAD, "the gap is a whole padding wide");
    }

    #[test]
    fn an_evicted_shelf_clears_its_pixels() {
        let mut atlas = GlyphAtlas::new(64);
        let (x, y) = atlas.allocate(16, 16).unwrap();
        atlas.blit(x, y, &coverage(16, 16));
        assert!(
            atlas.pixels().iter().any(|&pixel| pixel != 0),
            "the blitted glyph is in the atlas"
        );
        assert!(atlas.evict_lru_row().is_some());
        assert!(
            atlas.pixels().iter().all(|&pixel| pixel == 0),
            "and the eviction took it out again, so a later glyph cannot sample it"
        );
    }

    #[test]
    fn an_evicted_shelf_is_reused_rather_than_appended_to() {
        let mut atlas = GlyphAtlas::new(64);
        let (_, first_y) = atlas.fit(16, 16).unwrap();
        // Fill the atlas, without evicting, so a new shelf cannot be appended.
        while atlas.fit(16, 16).is_some() {}
        assert!(atlas.fit(16, 16).is_none(), "the atlas is full");
        assert!(atlas.evict_lru_row().is_some());
        let (x, y) = atlas.fit(16, 16).expect("the freed shelf takes it");
        assert_eq!(y, first_y, "the glyph went back into the evicted space");
        assert_eq!(x, 0);
    }

    #[test]
    fn a_glyph_too_big_for_the_atlas_is_refused_rather_than_looping() {
        let mut atlas = GlyphAtlas::new(16);
        assert!(atlas.fit(17, 8).is_none(), "wider than the atlas");
        assert!(atlas.fit(8, 17).is_none(), "taller than the atlas");
        // Every shelf is evicted in turn, and then there is nothing left to
        // evict: allocation gives up rather than looping forever.
        assert!(atlas.fit(8, 8).is_some());
        assert!(atlas.allocate(15, 15).is_none());
    }

    #[test]
    fn the_pixels_are_offered_once_per_change() {
        let mut atlas = GlyphAtlas::new(64);
        assert!(atlas.take_dirty_pixels().is_none(), "a new atlas is clean");
        let (x, y) = atlas.allocate(8, 8).unwrap();
        atlas.blit(x, y, &coverage(8, 8));
        assert!(atlas.take_dirty_pixels().is_some(), "a blit dirties it");
        assert!(
            atlas.take_dirty_pixels().is_none(),
            "and it is clean again until the next blit, so the renderer does not \
             re-upload a texture that has not changed"
        );
    }

    /// The advance cache is the fix for the demo running at about 4 frames per
    /// second, and it is invisible to a test that only checks the *values* an
    /// advance returns: an uncached `advance` returns exactly the same numbers.
    /// These tests are therefore about the cache's own behaviour, and they are
    /// possible without a font file only because the cache is its own type.

    #[test]
    fn a_measured_advance_is_returned_again_rather_than_measured_again() {
        let cache = AdvanceCache::default();
        assert_eq!(cache.get('a', 20), None, "nothing measured yet");
        cache.set('a', 20, 7.5);
        assert_eq!(cache.get('a', 20), Some(7.5));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn a_second_measurement_of_the_same_character_and_size_is_not_a_second_entry() {
        let cache = AdvanceCache::default();
        cache.set('a', 20, 7.5);
        cache.set('a', 20, 7.5);
        assert_eq!(
            cache.len(),
            1,
            "the repeat a frame makes is the whole point: it must not grow the \
             cache, because growing it means the measurement ran again"
        );
    }

    #[test]
    fn the_size_is_part_of_the_key() {
        let cache = AdvanceCache::default();
        cache.set('a', 20, 7.5);
        assert_eq!(
            cache.get('a', 21),
            None,
            "an advance at 21 pixels is a different measurement, so the size \
             cannot be left out of the key"
        );
        assert_eq!(cache.get('b', 20), None, "and so is the character");
    }

    #[test]
    fn a_clone_shares_the_one_cache() {
        // A `Font` is cloned per label, and a clone shares the FreeType face.
        // If the cache were per-clone it would be a cache per label, and every
        // label in a frame would pay for the same characters again.
        let cache = AdvanceCache::default();
        let clone = cache.clone();
        cache.set('a', 20, 7.5);
        assert_eq!(
            clone.get('a', 20),
            Some(7.5),
            "a clone sees what the original measured"
        );
        clone.set('b', 20, 3.0);
        assert_eq!(cache.get('b', 20), Some(3.0), "and the other way round");
        assert_eq!(cache.len(), 2, "one cache, not two");
    }
}

/// The tests below are about the second weight: which face a request resolves
/// to, and what the atlas then does with the same letter at two weights.
///
/// Neither needs a font file, and neither could: rasterizing needs a real face,
/// and a unit test may not open one. What *can* be tested is everything the
/// weight decides before FreeType is called — the slot a request lands in, and
/// the key the atlas files a glyph under — and that is where both mistakes this
/// feature invites would have to be made.
#[cfg(test)]
mod weight_tests {
    use super::*;

    /// A glyph's placement, off the origin and with a size the assertions can
    /// tell apart: an atlas placement at `(0, 0)` with the default metrics cannot
    /// be told from any other, which is the same reason `.ai/NEVERAGAIN.md` § *A
    /// rect's origin and a rect's extent are different numbers* exists.
    fn placement(x: f32, advance: f32) -> GlyphPlacement {
        GlyphPlacement {
            u0: x / 1024.0,
            v0: 0.0,
            u1: (x + 8.0) / 1024.0,
            v1: 8.0 / 1024.0,
            row_y: 16,
            width: 8,
            height: 8,
            bearing_x: 1,
            bearing_y: 7,
            advance,
        }
    }

    /// The two faces a renderer holds once a regular and a bold one are installed,
    /// as the stand-ins `resolve_slot` reads: the weight's slot is what matters,
    /// not what the face is.
    fn both_faces() -> [Option<&'static str>; FACE_COUNT] {
        [Some("regular"), Some("bold")]
    }

    #[test]
    fn the_weight_a_run_asks_for_is_the_slot_it_resolves_to() {
        let faces = both_faces();
        assert_eq!(resolve_slot(&faces, FontWeight::Regular), Some(0));
        assert_eq!(resolve_slot(&faces, FontWeight::Bold), Some(1));
    }

    #[test]
    fn a_weight_with_no_face_of_its_own_is_drawn_with_the_regular_one() {
        // The fallback, stated as a fact rather than as an absence of errors: a
        // bold run on a renderer given one font is *drawn*, in the regular face.
        let regular_only: [Option<&'static str>; FACE_COUNT] = [Some("regular"), None];
        assert_eq!(
            resolve_slot(&regular_only, FontWeight::Bold),
            Some(0),
            "a heading that asked for bold and got the regular face is readable; \
             one that resolved to nothing is a hole in the layout"
        );
    }

    #[test]
    fn every_weight_resolves_and_none_of_them_panics() {
        // Total is the whole claim: there is no `Result` to return from a
        // resolution and nothing for a caller to handle, so every one of the
        // weights has to answer whatever is installed, including nothing.
        let empty: [Option<&'static str>; FACE_COUNT] = [None, None];
        let regular_only: [Option<&'static str>; FACE_COUNT] = [Some("regular"), None];
        let both = both_faces();
        for weight in FontWeight::ALL {
            assert_eq!(
                resolve_slot(&empty, weight),
                None,
                "{weight:?}, nothing held"
            );
            assert_eq!(
                resolve_slot(&regular_only, weight),
                Some(0),
                "{weight:?}, only the regular face held"
            );
            assert!(
                resolve_slot(&both, weight).is_some(),
                "{weight:?}, both faces held"
            );
        }
    }

    #[test]
    fn a_set_with_only_a_bold_face_draws_its_bold_runs_and_none_of_the_others() {
        // The one set that answers differently per weight, and the reason the
        // fallback is the *regular* face rather than any face at all: a regular
        // run quietly set in bold would be a surprise with nothing to point at,
        // while a regular run that is not drawn is a hole the caller can see.
        let bold_only: [Option<&'static str>; FACE_COUNT] = [None, Some("bold")];
        assert_eq!(resolve_slot(&bold_only, FontWeight::Bold), Some(1));
        assert_eq!(resolve_slot(&bold_only, FontWeight::Regular), None);
    }

    #[test]
    fn a_set_with_no_face_at_all_resolves_to_nothing_rather_than_a_wrong_one() {
        // The one case that is `None`, and it is the case the renderer had before
        // there was a second weight: no font, no text.
        let empty: [Option<&'static str>; FACE_COUNT] = [None, None];
        for weight in FontWeight::ALL {
            assert_eq!(resolve_slot(&empty, weight), None);
        }
    }

    #[test]
    fn the_default_weight_is_the_regular_face() {
        // What keeps "a caller that never asks for bold gets today's behaviour"
        // true without a rule anywhere: `Default` is what a command built without
        // a weight gets, and a `Painter::text` names it explicitly.
        assert_eq!(FontWeight::default(), FontWeight::Regular);
        assert_eq!(FontWeight::ALL[0], FontWeight::default());
    }

    #[test]
    fn the_regular_slot_is_the_slot_the_fallback_lands_on() {
        // The two are one definition: `REGULAR_SLOT` is written from
        // `FontWeight::Regular.slot()`, and this says the enum's own numbering
        // has not moved out from under it.
        assert_eq!(REGULAR_SLOT, 0);
        assert_eq!(FontWeight::Regular.slot(), REGULAR_SLOT);
        assert_ne!(FontWeight::Bold.slot(), REGULAR_SLOT);
    }

    #[test]
    fn every_weight_names_a_slot_the_set_has_room_for() {
        // A weight whose slot was outside the array would be an index panic in the
        // middle of a frame; the array length and the enum are two numbers that
        // have to agree, which is what this is.
        assert_eq!(FontWeight::ALL.len(), FACE_COUNT);
        for weight in FontWeight::ALL {
            assert!(weight.slot() < FACE_COUNT, "{weight:?} is addressable");
        }
    }

    #[test]
    fn the_same_letter_in_two_faces_is_two_atlas_entries() {
        // The feature and its only silent failure: 'a' at 20 pixels, once
        // rasterized from the regular face and once from the bold one, must be
        // found separately — a key without the face would hand the bold run the
        // regular glyph's quad, one letter in the wrong weight with no error.
        let mut atlas = GlyphAtlas::new(1024);
        let regular = FaceId(0);
        let bold = FaceId(1);
        let key_regular = GlyphAtlas::key_for('a', 20.0, regular);
        let key_bold = GlyphAtlas::key_for('a', 20.0, bold);
        assert_ne!(
            key_regular, key_bold,
            "'a' at 20 pixels in Lato-Medium and 'a' at 20 pixels in Lato-Bold \
             are different glyphs and the key has to say which face rasterized it"
        );

        atlas.glyphs.insert(key_regular, placement(8.0, 10.0));
        atlas.glyphs.insert(key_bold, placement(64.0, 12.0));

        let found_regular = atlas.glyphs.get(&key_regular).copied();
        let found_bold = atlas.glyphs.get(&key_bold).copied();
        assert_eq!(
            found_regular.map(|p| p.advance),
            Some(10.0),
            "the regular one"
        );
        assert_eq!(
            found_bold.map(|p| p.advance),
            Some(12.0),
            "and the bold one is the bold one's glyph, not the regular glyph found \
             under a second key"
        );
        assert_eq!(atlas.glyphs.len(), 2, "two entries, not one overwritten");
        assert_eq!(
            found_bold.map(|p| p.u0),
            Some(64.0 / 1024.0),
            "each face's glyph is packed where it was put"
        );
    }

    #[test]
    fn the_face_is_part_of_the_key_and_the_character_and_size_still_are() {
        // The other direction: adding the face must not make the key *only* the
        // face, or every glyph would be one entry per face and the atlas would
        // hold one letter.
        let face = FaceId(3);
        assert_ne!(
            GlyphAtlas::key_for('a', 20.0, face),
            GlyphAtlas::key_for('b', 20.0, face)
        );
        assert_ne!(
            GlyphAtlas::key_for('a', 20.0, face),
            GlyphAtlas::key_for('a', 21.0, face)
        );
        assert_eq!(
            GlyphAtlas::key_for('a', 20.0, face),
            GlyphAtlas::key_for('a', 20.0, face)
        );
    }

    #[test]
    fn the_size_in_the_key_is_the_pixel_count_the_face_is_set_to() {
        // Unchanged by the face, and pinned because the atlas's own tests would
        // not notice: a float key would store the same glyph twice for two sizes
        // that round to the same pixel count.
        assert_eq!(GlyphAtlas::key_for('a', 20.4, FaceId(0)).size, 20);
        assert_eq!(GlyphAtlas::key_for('a', 0.0, FaceId(0)).size, 1);
    }

    #[test]
    fn each_installed_face_is_given_an_identity_of_its_own() {
        let mut ids = FaceIds::new();
        let first = ids.issue();
        let second = ids.issue();
        assert_ne!(
            first, second,
            "two faces cached under one identity would share every glyph in the \
             atlas, whichever weight they were rasterized from"
        );
    }

    #[test]
    fn a_reinstalled_face_gets_an_identity_the_previous_one_never_had() {
        // The rule the atlas key depends on when a renderer swaps its bold file:
        // the glyphs of the file that has just been dropped must not be found for
        // the file that is there now, and the only thing that can separate them is
        // a new identity.
        let mut ids = FaceIds::new();
        let regular = ids.issue();
        let bold = ids.issue();
        ids.install(FontWeight::Regular.slot(), regular);
        ids.install(FontWeight::Bold.slot(), bold);

        let bold_again = ids.issue();
        ids.install(FontWeight::Bold.slot(), bold_again);

        assert_ne!(bold_again, bold, "the replaced face is a different face");
        assert_eq!(
            ids.get(FontWeight::Bold.slot()),
            bold_again,
            "and the slot holds the one that is installed now"
        );
        assert_eq!(
            ids.get(FontWeight::Regular.slot()),
            regular,
            "while the other slot is untouched by one slot being replaced"
        );
    }
}
