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
//!
//! **A family is a chain, and the chain is the fallback.** A run also says which
//! family it wants ([`FamilyId`]), and a family holds a face per weight plus a
//! list of fallbacks; the first font in that list with a glyph for a character
//! draws it. The chain is built once, when the family is defined, and a lookup
//! walks it — it is never assembled per character.
//!
//! **A character no font in the chain covers is drawn, not dropped.** It becomes
//! [`replacement_bitmap`]: a hollow box, synthesized rather than taken from a font
//! because the fonts this repository's demo loads have no glyph at U+FFFD, which
//! was measured against each file's own character map rather than assumed. The box
//! is a [`GlyphBitmap`] like any other, so it is packed into the atlas and drawn
//! by the text shader with no second draw path, and it advances the pen by
//! [`replacement_advance`] — the same function the measuring half asks, so the
//! hole in the layout is exactly as wide as the box drawn in it.
//!
//! **The atlas grows, and a refusal is counted rather than silent.** A full
//! atlas doubles — to the next power of two, no past the ceiling the renderer
//! read from the driver — and re-packs the glyphs already in it, so every glyph
//! placed before the grow still addresses its own pixels afterwards. Only at the
//! ceiling does it evict, and a glyph that fits nowhere even there is reported
//! through [`GlyphAtlas::dropped`] instead of drawn as a hole nobody is told
//! about.

use freetype::face::LoadFlag;
use freetype::{Face, Library};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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

    /// Returns whether this font has a glyph for `ch`.
    ///
    /// **A character it has no glyph for is a normal answer, not an error**, and
    /// this is the question the fallback chain asks: a run's first font says no
    /// and the next one is asked, and a chain in which every font says no draws
    /// the replacement rather than nothing.
    ///
    /// It asks FreeType's character map and **does not rasterize or load a glyph**,
    /// which is the whole reason the chain can be walked per character per frame:
    /// the answer is a lookup in the face's own table of what it covers, where
    /// [`Font::advance`] and [`Font::rasterize`] are a glyph load each. Asking by
    /// `load_char` instead — which also answers correctly — would cost a load per
    /// font per character, and the load is the 61 µs the advance cache exists to
    /// avoid.
    ///
    /// **Index `0` counts as "no glyph"**, because FreeType returns `0` for a
    /// character the face maps to `.notdef`, and `.notdef` is the empty box every
    /// font carries rather than a drawing of the character asked for. Counting it
    /// as covered would make every font claim every character and the chain would
    /// never move past the first font — a silent hole where the fallback should
    /// have been.
    #[must_use]
    pub fn has_glyph(&self, ch: char) -> bool {
        self.face.get_char_index(codepoint(ch)).is_some()
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
/// places would be two rules.** A family's chain reads it to find the face a run
/// starts from — in `Family::primary_id`, which is where the chain's own two
/// clauses live — and the renderer's text pass reads the set's answer; neither
/// decides anything of its own.
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

/// One installed font's identity: what a chain names and the atlas keys on.
///
/// The atlas is shared by every font the set holds, so a glyph's cache key has to
/// say which font rasterized it — see `GlyphKey`, which is where the atlas's
/// cache key spells it out. This is that third part of the key, and it is
/// deliberately **not** the weight: it is handed out every time a font is
/// installed, so a renderer that swaps its bold file cannot serve the glyphs of
/// the file it no longer holds. A newtype rather than a bare `u32` because the
/// two are not interchangeable, and this is the one value whose confusion draws a
/// letter in the wrong weight with no error anywhere.
///
/// **It names a file, not a family.** A [`FamilyId`] names the *chain* — the
/// ordered list of fonts a run is looked for in — and this names one font inside
/// one; a chain of three fonts has three of these and one [`FamilyId`]. The two
/// are not interchangeable either, and the value of confusing them is a chain
/// walked in the wrong order, which is a letter drawn from a font the caller did
/// not ask for.
///
/// **The number comes from `FontIds::issue` and there is no public constructor
/// for it.** That is what keeps a caller from naming a font the set does not hold:
/// the id is meaningless outside the set that issued it, which is also why a set
/// handed to a renderer must be the same set the caller measured with — two sets
/// built by the same calls in the same order agree, and two built independently
/// do not, with nothing to say so.
///
/// **It is also the font's own index in the set's table**, which is what makes the
/// identity and the lookup one number rather than two that could drift — see
/// `FontSet::install`, which issues once and pushes once.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FontId(u32);

impl FontId {
    /// Returns the number this id wraps.
    ///
    /// For a caller keeping an id beside its own bookkeeping; nothing in this
    /// crate needs the number rather than the id.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// One installed font as the glyph atlas needs it: the [`Font`] to rasterize
/// from, and the [`FontId`] its glyphs are cached under.
///
/// A pair rather than two arguments to [`GlyphAtlas::get_or_insert`] because the
/// two must not be able to disagree: the atlas keys on the id and rasterizes from
/// the font, so a caller that passed one font's id with another font's outlines
/// would cache a glyph under the wrong key and draw the wrong glyphs for every
/// later run — a defect invisible until two fonts are on screen at once. There
/// are exactly two ways to make this value, [`FontSet::pick`] and
/// [`FontSet::primary`], and both are on the set.
#[derive(Clone, Copy)]
pub struct FontRef<'a> {
    font: &'a Font,
    id: FontId,
}

impl std::fmt::Debug for FontRef<'_> {
    /// Prints the font's id and nothing else.
    ///
    /// Not derived because `Font` is not `Debug` — it holds a FreeType handle,
    /// which has nothing to print that means anything — and a `Debug` that could
    /// not be derived would have to leave the field out anyway. The id is the half
    /// that says which font this is.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontRef")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<'a> FontRef<'a> {
    /// Returns the font to rasterize from.
    #[must_use]
    pub fn font(&self) -> &'a Font {
        self.font
    }

    /// Returns the identity this font's glyphs are cached under.
    #[must_use]
    pub fn id(&self) -> FontId {
        self.id
    }
}

/// The counter `FontSet::install` draws the next font's identity from.
///
/// **Its own type, and the reason is a test.** Before task 30 a `Font` could only
/// be put in the set by installing a *file*, and `AGENTS.md` forbids a test to
/// open one — so the rule worth testing here, *"every install is a new identity,
/// so a weight that is reinstalled cannot be served the glyphs of the file that
/// has just been dropped"*, had no seam to be tested through. The counter is that
/// seam: it is a number, and two numbers can be compared without a font.
///
/// **It is also the whole of what makes the id unique**, and it agrees with
/// `FontSet::fonts`' own length by construction — `install` pushes exactly one
/// font per issue — so there is one number in the set rather than an index in a
/// vector beside a counter that could drift from it. `the_identity_and_the_fonts_
/// own_length_are_the_same_number` is the test that pins that, and it is the
/// assertion this type exists to make possible.
#[derive(Clone, Debug, Default)]
struct FontIds {
    /// The id the next installed font is given, which is also how many fonts have
    /// been installed.
    issued: u32,
}

impl FontIds {
    /// Returns an identity no font in this set has held, and counts it.
    ///
    /// A counter rather than anything derived from a slot because the slot is not
    /// the identity: a font installed for one weight may be replaced by a
    /// different file later, and the first file's glyphs must not be found for the
    /// second. Wrapping after `u32::MAX` installs is not a hazard any process will
    /// reach, and a wrapped id would be a *repeat* rather than a wrong one.
    fn issue(&mut self) -> FontId {
        let id = FontId(self.issued);
        self.issued = self.issued.wrapping_add(1);
        id
    }
}

/// The name the default family is defined under.
///
/// The default family is **not** named `"sans-serif"` on purpose: a name that
/// looks like a promise about fontconfig's generic families is one a reader would
/// take as "the font the platform picks", and this set does no such thing. The
/// name is what [`FontSet::define_family`] would find it under, so it is written
/// down once and read from one place.
const DEFAULT_FAMILY: &str = "default";

/// A named chain of fonts: what a text run asks for, and what a
/// [`crate::paint::DrawCommand::Text`] carries.
///
/// **A family is a chain, not a face.** One [`FontId`] names one installed file;
/// this names the *order* a run looks for glyphs in — the face for the run's
/// weight first, then the fallbacks — so `font_family` can be a property that
/// changes which file a character is drawn from.
///
/// The default is [`FontSet::default_family`]'s id, which is `0`, so a value
/// built by [`Default`] is the family every program gets without asking for one.
/// A [`Property<FamilyId>`](crate::property::Property) therefore needs no
/// special case for "no family was chosen", which is what made this the shape
/// rather than an `Option<FamilyId>`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FamilyId(u32);

impl FamilyId {
    /// Returns the number this id wraps.
    ///
    /// For a caller keeping an id beside its own bookkeeping; nothing in this
    /// crate needs the number rather than the id.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }

    /// Wraps a family's index in a [`FontSet`].
    ///
    /// Private for the reason [`FontId::u32_from`] is: the number is the set's
    /// own table index, and a handle that could name a family that is not there
    /// would be indistinguishable from one that is.
    fn u32_from(index: u32) -> Self {
        FamilyId(index)
    }
}

/// One family: a face per weight, and the fallbacks tried after them.
///
/// **The chain is built once, here, when the family is defined** — and a family
/// is only ever appended to, so it is never rebuilt: a glyph lookup walks the
/// entries below and does not assemble anything. That is the whole of what
/// "the chain is built once per family, not per glyph" asks for, and the
/// alternative — deriving the chain inside every lookup — would be the same
/// vector rebuilt once per character of every frame.
///
/// The two lists are separate because they answer different questions. `faces`
/// is *this family's own face for each weight*, and a weight asks for exactly one
/// of them, so a missing entry means "this family has no face of that weight"
/// and the chain moves on to the fallbacks. `fallbacks` is *the files to try after
/// that face*, in order, and it is shared by every weight in the family: a bold
/// run that the bold face cannot draw has no more business reaching for the
/// family's regular face than it has for any other font it was not given.
#[derive(Clone, Debug, Default)]
struct Family {
    /// The name this family was defined under. The set keeps it beside the chain
    /// so that `family(name)` can answer, and because a chain a reader cannot
    /// name is a chain nobody can check.
    name: String,
    /// The family's own face for each weight, if it has one.
    faces: [Option<FontId>; FACE_COUNT],
    /// The fonts tried after the face for the weight asked for, in order.
    fallbacks: Vec<FontId>,
}

/// The family a set with no families at all answers with, which
/// [`FontSet::new`] makes unreachable — kept because the accessor that would need it
/// is otherwise a function with an arm that cannot be written.
static EMPTY_FAMILY: Family = Family {
    name: String::new(),
    faces: [None, None],
    fallbacks: Vec::new(),
};

impl Family {
    /// Returns whether the family holds no font at all — no face for any weight
    /// and no fallback.
    fn is_empty(&self) -> bool {
        self.faces.iter().all(Option::is_none) && self.fallbacks.is_empty()
    }

    /// Returns the id of the font that starts this family's chain for `weight`.
    ///
    /// **The weight's own face, else the regular one, else the first fallback** —
    /// the first two clauses are [`resolve_slot`]'s rule unchanged, and the third
    /// is what makes a family that defines no face of its own work at all: a
    /// chain of fallbacks alone is a family whose first font is its primary.
    ///
    /// `None` when the family has no face for the weight, no regular face, and no
    /// fallback: there is nothing to draw the run with, which is the one case the
    /// caller can see.
    fn primary_id(&self, weight: FontWeight) -> Option<FontId> {
        resolve_slot(&self.faces, weight)
            .and_then(|slot| self.faces[slot])
            .or_else(|| self.fallbacks.first().copied())
    }
}

/// What a chain says about one character: an installed font that covers it, or
/// nothing.
///
/// **Neither variant is an error.** A character no font in the chain covers is an
/// ordinary state — it is drawn as the replacement glyph — so there is no
/// `Result`, and the caller that cannot act on an error has one action to take
/// anyway.
#[derive(Clone, Copy, Debug)]
pub enum PickedFont<'a> {
    /// The first font in the chain that has a glyph for the character.
    Covered(FontRef<'a>),
    /// No font in the chain has one; draw the replacement.
    Replacement,
}

/// Picks the first font of a chain that covers `ch`, given the chain's primary,
/// its fallbacks and a way to ask each one.
///
/// A free function over a `covers` callback rather than a method on [`FontSet`]
/// for the reason [`resolve_slot`] is: **the rule is worth testing without a
/// font file, and a `Font` cannot be made without one.** The set's own
/// [`FontSet::pick`] supplies the callback and is otherwise this function.
///
/// The primary is asked first and then the fallbacks, with the primary skipped in
/// the second pass: a chain whose only fallback is the font that already answered
/// asks each font twice, which is free but reads as two rules when it is one.
///
/// Total: every chain answers for every character, including the empty chain,
/// which answers [`PickedFont::Replacement`].
#[must_use]
pub fn pick_in_chain(
    primary: Option<FontId>,
    fallbacks: &[FontId],
    covers: &mut dyn FnMut(FontId, char) -> bool,
    ch: char,
) -> Option<FontId> {
    if let Some(id) = primary {
        if covers(id, ch) {
            return Some(id);
        }
    }
    fallbacks
        .iter()
        .copied()
        .find(|&id| Some(id) != primary && covers(id, ch))
}

/// The fonts a renderer draws text with: an installed file per weight, the chains
/// built from them, and the identity each file's glyphs are cached under.
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
    /// Every installed font, in install order, and **the index is the [`FontId`]'s
    /// number** — a chain names a font and this is where it is found.
    fonts: Vec<Font>,
    /// The identities handed out, which `install` draws from and which agree with
    /// `fonts`' own length by construction. Its own type so the rule can be tested
    /// without a font file; see [`FontIds`].
    ids: FontIds,
    /// The families, by name. The first is the default family, and there is
    /// always at least one, so a [`FamilyId`] of `0` names a family that exists.
    families: Vec<Family>,
}

impl Default for FontSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FontSet {
    /// Creates a set holding no font, which draws no text.
    ///
    /// **The default family exists from here on.** It holds nothing, so it
    /// resolves to nothing and a run in it is not drawn — which is what a renderer
    /// that has installed no font does. What it buys is that
    /// [`FamilyId::default`] and [`FontSet::default_family`] are the same family
    /// without either of them having to be special-cased anywhere else.
    #[must_use]
    pub fn new() -> Self {
        FontSet {
            fonts: Vec::new(),
            ids: FontIds::default(),
            families: vec![Family {
                name: DEFAULT_FAMILY.to_string(),
                ..Family::default()
            }],
        }
    }

    /// Installs `font` as the default family's face for `weight`, replacing
    /// whatever was there.
    ///
    /// The font is given a **fresh** [`FontId`] every time it is installed,
    /// including the first: the id is *this file's* identity in the atlas, not
    /// the slot's name, so replacing a weight re-rasterizes rather than serving
    /// glyphs rasterized from the file that has just been dropped. Which number
    /// that is depends on the order the fonts were installed in, and nothing
    /// depends on the number — the atlas only ever compares ids.
    pub fn set(&mut self, weight: FontWeight, font: Font) {
        let id = self.install(font);
        self.default_family_mut().faces[weight.slot()] = Some(id);
    }

    /// Installs `font` in the default family's chain, after the face for the
    /// weight a run asks for, and returns the identity it was given.
    ///
    /// This is the whole of the fallback mechanism's entry point: a run whose
    /// primary face has no glyph for a character asks this list next, in the order
    /// fonts were added, and the first one that covers it draws that character.
    /// The chain is **appended to and never rebuilt**, so a family defined once
    /// answers the same way for the rest of the program's life.
    pub fn add_fallback(&mut self, font: Font) -> FontId {
        let id = self.install(font);
        self.default_family_mut().fallbacks.push(id);
        id
    }

    /// Defines a family named `name` with nothing in it yet, and returns its id.
    ///
    /// An empty family draws nothing, exactly like a set with no font: it is a
    /// name a caller can put in a `font_family` property and fill with
    /// [`FontSet::add_fallback_to`], and **an existing name is not replaced** —
    /// defining a family twice returns the one that is there, so a second call is
    /// not a way to empty a chain that something else is drawing with.
    pub fn define_family(&mut self, name: &str) -> FamilyId {
        if let Some(id) = self.family_id(name) {
            return id;
        }
        let id = FamilyId::u32_from(u32::try_from(self.families.len()).unwrap_or(0));
        self.families.push(Family {
            name: name.to_string(),
            ..Family::default()
        });
        id
    }

    /// Appends `font` to `family`'s chain and returns the identity it was given.
    ///
    /// The font is installed afresh rather than shared with another family, so
    /// naming the same file in two families costs two identities and therefore two
    /// sets of atlas entries for its glyphs. That is deliberate: a shared entry
    /// would need the two families to agree that the file never changes, and an
    /// id per install is the one thing that makes a replaced file's glyphs
    /// unreachable.
    pub fn add_fallback_to(&mut self, family: FamilyId, font: Font) -> FontId {
        let id = self.install(font);
        if let Some(family) = self.family_mut(family) {
            family.fallbacks.push(id);
        }
        id
    }

    /// Returns the id of the family named `name`, or the default family's.
    ///
    /// **An unknown name is the default family and not an error**, because this is
    /// where a `font_family` string becomes a handle and a property's default is
    /// `"sans-serif"` in every program that has not defined one. Refusing it would
    /// mean every program that had not been told about families had to be taught
    /// about them first, and a family that quietly resolved to nothing would draw
    /// no text at all.
    #[must_use]
    pub fn family(&self, name: &str) -> FamilyId {
        match self.family_id(name) {
            Some(id) => id,
            None => self.default_family(),
        }
    }

    /// Returns the id of the family named `name`, if it is defined.
    #[must_use]
    pub fn family_id(&self, name: &str) -> Option<FamilyId> {
        self.families
            .iter()
            .position(|family| family.name == name)
            .and_then(|index| u32::try_from(index).ok())
            .map(FamilyId::u32_from)
    }

    /// Returns the default family's id, which is [`FamilyId::default`].
    #[must_use]
    pub fn default_family(&self) -> FamilyId {
        FamilyId::default()
    }

    /// Returns whether the set holds no font at all.
    ///
    /// The renderer's text pass asks this before it walks a batch, because a
    /// renderer with no font draws no text — which is what it did before there
    /// were two weights, and asking here is what keeps that path as free as it
    /// was rather than free modulo a per-command resolution.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// Returns what the chain of `family` says about `ch` in `weight`.
    ///
    /// This is the whole of the fallback rule at the call site: the family's primary
    /// for the weight and its fallbacks, asked in that order, and the first font
    /// with a glyph for the character wins. [`pick_in_chain`] is that walk, as a
    /// function that can be tested without a font.
    ///
    /// **The family is read through `FontSet::drawable`**, so an empty family
    /// answers from the default one rather than from nothing — see that method for
    /// why an empty family must not be a dropped run.
    ///
    /// `PickedFont::Replacement` means **no font in the chain has a glyph**, which
    /// is not a failure and must not be treated as one: the caller draws
    /// [`replacement_bitmap`] and the run continues.
    #[must_use]
    pub fn pick(&self, family: FamilyId, weight: FontWeight, ch: char) -> PickedFont<'_> {
        let entry = self.drawable(family);
        let primary = entry.primary_id(weight);
        let picked = pick_in_chain(
            primary,
            &entry.fallbacks,
            &mut |id, ch| self.covers(id, ch),
            ch,
        );
        match picked.and_then(|id| self.font_ref(id)) {
            Some(font) => PickedFont::Covered(font),
            None => PickedFont::Replacement,
        }
    }

    /// Returns the face that starts `family`'s chain for `weight`: the font every
    /// metric of the run comes from.
    ///
    /// **Ascent and line height are this font's and not the drawn glyphs'.** A run
    /// that spans two fonts has no single face to take its line's height from —
    /// taking it from whichever glyph happened to be first would make the line
    /// box depend on the text's first character — so the primary is the one whose
    /// metrics the layout uses, and the fallbacks contribute glyphs, bearings and
    /// advances and nothing else.
    ///
    /// `None` only for a set with **no font at all**, which is the one state with
    /// nothing to measure from and nothing to draw with; see `FontSet::drawable`
    /// for the empty *family*, which is not that state.
    #[must_use]
    pub fn primary(&self, family: FamilyId, weight: FontWeight) -> Option<FontRef<'_>> {
        let primary = self.drawable(family).primary_id(weight)?;
        self.font_ref(primary)
    }

    /// Returns the family `id` names **when it holds a font**, and the default
    /// family otherwise.
    ///
    /// **An empty family resolves to the default one, and that is what stops a run
    /// from vanishing.** The review of task 30 found the gap: `draw_text_batch`
    /// asks for a run's primary and `continue`s when there is none, so a family
    /// defined with nothing in it — which `define_family` accepts, and which a
    /// `Property<FamilyId>` can be written to before its fonts are installed —
    /// dropped every character of every run in it. Requirement 4 says a character
    /// no font covers *"must never be a silent hole"*, and a whole run is the
    /// largest hole there is.
    ///
    /// **The same rule as [`FontSet::family`]'s, applied one step later:** an
    /// unknown *name* resolves to the default family there, and an empty family
    /// resolves to it here. One rule with two arms rather than two rules, and a
    /// handle that cannot name a font-less family rather than one that can.
    ///
    /// A set whose **default** family is also empty has nothing to offer at all,
    /// and this returns the requested family — `primary` then answers `None` and
    /// the renderer draws nothing, which is correct: **a renderer with no font is
    /// the one state that draws no text.**
    ///
    /// **No allocation and no `Option`**, because this is called once per character
    /// per run and a chain lookup that cloned a family would be the first
    /// per-frame allocation in the text path — the shape of cost `.ai/NEVERAGAIN.md`
    /// records as invisible until a frame rate is measured.
    fn drawable(&self, id: FamilyId) -> &Family {
        let entry = self.entry(id);
        let default = self.default_family();
        if !entry.is_empty() || default == id {
            return entry;
        }
        self.entry(default)
    }

    /// Returns the advance width of `ch` at `size` pixels in `family`, in pixels.
    ///
    /// **This is the measuring half of the text path and it answers what
    /// [`FontSet::pick`] answers**, which is the only reason it can be trusted by a
    /// layout: a run laid out with one chain's advances and drawn with another's
    /// is a line whose words do not land under their glyphs. It asks the chain
    /// rather than being told the answer, and the two callers cannot drift.
    ///
    /// A character no font in the chain covers is
    /// [`replacement_advance`] — the width the replacement is drawn at, so the
    /// hole the caller leaves in the layout is exactly as wide as the box drawn
    /// in it.
    #[must_use]
    pub fn advance(&self, family: FamilyId, weight: FontWeight, ch: char, size: f32) -> f32 {
        match self.pick(family, weight, ch) {
            PickedFont::Covered(face) => face.font().advance(ch, size),
            PickedFont::Replacement => replacement_advance(size),
        }
    }

    /// Returns the distance from a line's top edge to the baseline at `size`
    /// pixels, from the chain's primary face.
    ///
    /// `None` for a family with no font to ask; a caller that has no baseline has
    /// no text to draw either.
    #[must_use]
    pub fn ascent(&self, family: FamilyId, weight: FontWeight, size: f32) -> Option<f32> {
        self.primary(family, weight)
            .map(|face| face.font().ascent(size))
    }

    /// Returns the recommended distance between consecutive baselines at `size`
    /// pixels, from the chain's primary face.
    ///
    /// The primary face's and not a maximum over the chain, for the reason
    /// [`FontSet::primary`] gives.
    #[must_use]
    pub fn line_height(&self, family: FamilyId, weight: FontWeight, size: f32) -> Option<f32> {
        self.primary(family, weight)
            .map(|face| face.font().line_height(size))
    }

    /// Measures a run of text at `size` pixels in `family`: the sum of the
    /// advances of its characters, each from the font that covers it.
    #[must_use]
    pub fn measure(&self, family: FamilyId, weight: FontWeight, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|ch| self.advance(family, weight, ch, size))
            .sum()
    }

    /// Installs `font` and returns the identity it was given.
    ///
    /// **One issue and one push, which is what makes the id the font's own index.**
    /// The two are separate statements because the id is what the atlas keys on and
    /// the index is where the face is found, and a set in which they could differ
    /// would cache one file's glyphs under another's identity.
    fn install(&mut self, font: Font) -> FontId {
        let id = self.ids.issue();
        self.fonts.push(font);
        id
    }

    /// Returns the family `id` names, or the default family when there is no such
    /// family.
    ///
    /// An id this set never handed out resolves to the default family rather than
    /// to nothing, and for the same reason [`FontSet::family`] does: a handle from
    /// another set, or from before a family was defined, must draw *something*.
    ///
    /// **Total**, and that is deliberate — `FontSet::new` creates the default
    /// family and nothing removes it, so both arms have something to return. A
    /// caller that wants to know whether a family holds anything asks
    /// [`Family::is_empty`] through [`FontSet::drawable`].
    fn entry(&self, id: FamilyId) -> &Family {
        let index = usize::try_from(id.get()).unwrap_or(usize::MAX);
        self.families
            .get(index)
            .or_else(|| self.families.first())
            .unwrap_or(&EMPTY_FAMILY)
    }

    /// Returns the family `id` names for writing, or `None` when the set has no
    /// such family and no default to write into.
    fn family_mut(&mut self, id: FamilyId) -> Option<&mut Family> {
        let index = usize::try_from(id.get())
            .ok()
            .filter(|&index| index < self.families.len())
            .or(if self.families.is_empty() {
                None
            } else {
                Some(0)
            })?;
        self.families.get_mut(index)
    }

    /// Returns the default family for writing.
    ///
    /// A set always has one, because [`FontSet::new`] creates it, so this cannot
    /// fail and the callers do not each re-check it.
    fn default_family_mut(&mut self) -> &mut Family {
        self.families.first_mut().unwrap_or_else(|| {
            unreachable!("FontSet::new creates the default family, so there is one")
        })
    }

    /// Returns whether the installed font `id` names has a glyph for `ch`.
    fn covers(&self, id: FontId, ch: char) -> bool {
        self.font(id).is_some_and(|font| font.has_glyph(ch))
    }

    /// Returns the installed font `id` names.
    fn font(&self, id: FontId) -> Option<&Font> {
        usize::try_from(id.get())
            .ok()
            .and_then(|index| self.fonts.get(index))
    }

    /// Returns `id` paired with the font it names, which is what the atlas caches
    /// its glyphs under.
    fn font_ref(&self, id: FontId) -> Option<FontRef<'_>> {
        self.font(id).map(|font| FontRef { font, id })
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

/// The width of the replacement box as a fraction of the em.
///
/// The em rather than a pixel count because the box has to sit with the text it
/// is standing in for: a fixed 8 pixels would be a narrow box beside 24-pixel
/// type and a wide one beside 14-pixel type, and the *same* box is what tells a
/// reader that one character is missing.
///
/// It is also the box's **advance**, through [`replacement_advance`] — the hole a
/// layout leaves for the character is exactly as wide as the box drawn in it.
const REPLACEMENT_ADVANCE_EM: f32 = 0.6;

/// The height of the replacement box as a fraction of the em.
///
/// Slightly taller than it is wide, which is the proportion of the empty box
/// convention goes back on: a square reads as a filled square and a box twice as
/// tall as it is wide reads as a container that lost its contents, which is what
/// it is standing in for.
const REPLACEMENT_HEIGHT_EM: f32 = 0.72;

/// The smallest box either dimension may have, in pixels.
///
/// At the small sizes a label's truncation marker or a badge is drawn at, the
/// fractions round to zero or one, and a 1×1 box is one solid pixel that could be
/// mistaken for debris.
///
/// **Three and not two, and the review of task 30 is what found the two.** This
/// constant's own doc used to promise that the floor is "the smallest rectangle
/// that still reads as an outline", while **a 2×2 rectangle has no interior
/// column at all**: `y == 0 || x == 0 || y + 1 == h || x + 1 == w` is true for
/// every one of its four pixels, so every size from 1 to 4 pixels drew a solid
/// block — exactly the failure the sentence says the floor prevents. **Three is
/// the smallest width with a column that is not on the outline**, so it is the
/// smallest size at which "hollow" is true rather than merely intended.
/// `the_replacement_is_a_hollow_box_that_sits_on_the_baseline` now runs its pixel
/// loop over three sizes rather than one, which is what makes this a checked claim
/// instead of a comment.
const REPLACEMENT_MIN: u32 = 3;

/// The advance width of the replacement glyph at `size` pixels.
///
/// **The one number both halves of the replacement agree on**, and the reason it
/// is a function rather than a field read back from the bitmap: the layout's
/// width for an uncovered character comes from here and the drawn box's own
/// `advance` comes from [`replacement_bitmap`], so a caller that measured with
/// one and drew with the other would leave a gap or an overlap. Two callers, one
/// function — the rule `.ai/NEVERAGAIN.md`'s *two documents each claiming
/// ownership of one definition* is about, in code.
#[must_use]
pub fn replacement_advance(size: f32) -> f32 {
    REPLACEMENT_ADVANCE_EM * size.max(0.0)
}

/// Builds the glyph drawn for a character no font in a chain covers: a hollow
/// rectangle, one pixel of ink on each of its four edges.
///
/// **Synthesized rather than taken from a font, and that is a measurement rather
/// than a preference.** The obvious form is `U+FFFD REPLACEMENT CHARACTER`
/// rasterized from the primary font, and the font this repository's demo loads has
/// **no glyph at it** — `Font::has_glyph('\u{fffd}')` is false for Lato-Medium,
/// Lato-Bold, LiberationSans and NotoSansDevanagari on this host, checked against
/// each file's own character map rather than against a list of what those fonts
/// are supposed to have. A rule "use U+FFFD if the primary has one" would therefore
/// take its second branch in the one place it can be seen, and would look like a
/// feature in the fonts that do have it.
///
/// The box is the conventional empty box, so a reader sees *a character is
/// missing here* rather than *something was drawn*. It is a [`GlyphBitmap`] like
/// any other, which is what lets it be packed into the atlas and drawn by the text
/// shader with no second draw path: **one batch, one shader, one quad**.
///
/// The box sits **on the baseline**: `bearing_y` is the box's own height, so the
/// bottom edge is at the baseline and the top edge is where a capital letter's top
/// would be. The advance is [`replacement_advance`] and **not the box's rounded
/// width**, so the pen moves by the same fraction of the em whatever the size —
/// a rounded width would make the box and the gap disagree by up to half a pixel.
///
/// # Examples
///
/// ```
/// use ui_core::font::{replacement_advance, replacement_bitmap};
///
/// let box_ = replacement_bitmap(24.0);
/// assert_eq!(box_.advance, replacement_advance(24.0));
/// // A hollow box: every border pixel is ink and the middle is not.
/// let w = usize::try_from(box_.width).unwrap_or(0);
/// let h = usize::try_from(box_.height).unwrap_or(0);
/// assert_eq!(box_.pixels[0], 255, "the top-left corner is ink");
/// assert_eq!(box_.pixels[w + 1], 0, "and the middle of the top edge is not");
/// assert_eq!(
///     box_.pixels[(h / 2) * w + (w / 2)],
///     0,
///     "and neither is the middle of the box"
/// );
/// ```
#[must_use]
pub fn replacement_bitmap(size: f32) -> GlyphBitmap {
    let em = size.max(0.0);
    let width = (REPLACEMENT_ADVANCE_EM * em)
        .round()
        .max(u32_to_f32(REPLACEMENT_MIN));
    let height = (REPLACEMENT_HEIGHT_EM * em)
        .round()
        .max(u32_to_f32(REPLACEMENT_MIN));
    let w = width as u32;
    let h = height as u32;
    let mut pixels = vec![0u8; usize::try_from(w).unwrap_or(0) * usize::try_from(h).unwrap_or(0)];
    for y in 0..h {
        for x in 0..w {
            let edge = y == 0 || x == 0 || y + 1 == h || x + 1 == w;
            if !edge {
                continue;
            }
            let index = usize::try_from(y).unwrap_or(0) * usize::try_from(w).unwrap_or(0)
                + usize::try_from(x).unwrap_or(0);
            if let Some(slot) = pixels.get_mut(index) {
                *slot = 255;
            }
        }
    }
    GlyphBitmap {
        width: w,
        height: h,
        // The box starts at the pen, so its left edge is the pen's own position
        // and nothing is drawn to the left of it.
        bearing_x: 0,
        // Its bottom edge sits on the baseline.
        bearing_y: i32::try_from(h).unwrap_or(0),
        advance: replacement_advance(size),
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

/// The key identifying an entry in the atlas.
///
/// **Two variants rather than a key with an optional character**, because the
/// replacement glyph is not a character of a font and keying it as one would mean
/// either a fabricated `char` — a real character some font does have, whose entry
/// a reader would then believe was that character's — or an identity reserved for
/// it, which is a second naming scheme beside this one. The variant says what the
/// entry is, and both arms of it say it with fields they mean.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum GlyphKey {
    /// A character of one installed font, at a pixel size.
    ///
    /// **The font is part of the key because the atlas is shared by every font the
    /// set holds.** Everything the atlas stores about a glyph is that font's own
    /// — the coverage bitmap, the bearings that place it against the pen, and the
    /// advance the next glyph starts after — so a key of character and size alone
    /// would hand a run the *first* font's quad: one letter from the wrong file,
    /// drawn from the right UVs, with no error anywhere. `font` is the
    /// [`FontId`] rather than the [`FontWeight`] because it is the identity of the
    /// file that was rasterized, which is what changes when a weight is reinstalled
    /// and is not what the run asked for.
    Glyph {
        /// The character the glyph draws.
        ch: char,
        /// The pixel count the size rounds to, which is the size the font was
        /// actually set to.
        size: u32,
        /// The font that rasterized it.
        font: FontId,
    },
    /// The replacement glyph at a pixel size.
    ///
    /// **No character, and no font**, because it is drawn for every character no
    /// font covers and looks the same for all of them. One entry per size rather
    /// than one per uncovered character: a run of five characters nothing covers
    /// packs one box and draws it five times, where a key carrying the character
    /// would pack five identical copies of it and evict five shelves' worth of
    /// real glyphs.
    Replacement {
        /// The pixel count the size rounds to.
        size: u32,
    },
}

/// One glyph's entry in the atlas: where its pixels are, and the metrics its
/// quad is drawn from.
///
/// **Pixels, not UVs, and that is what makes a grow safe.** A UV is only
/// meaningful against the size it was divided by: a glyph packed at `x = 8` in a
/// 2048 texture has `u0 = 0.00390625`, and after the atlas grows to 4096 that
/// same number addresses `x = 16` — the wrong glyph's first pixel, from a run
/// that looks correctly laid out. So the atlas keeps the pixel it wrote and
/// [`GlyphAtlas::placement_of`] derives the UVs from the size it is *now*, which
/// makes a stale UV unrepresentable rather than merely unlikely.
///
/// The alternative — keeping UVs and recomputing all of them at the moment of the
/// grow — needs the pixel back to recompute them from, which is the number this
/// entry is.
#[derive(Clone, Copy, Debug)]
struct Entry {
    /// Left edge of the glyph's coverage in the atlas, in pixels.
    x: u32,
    /// Top edge of the glyph's coverage in the atlas, in pixels: the top edge of
    /// its shelf.
    y: u32,
    /// Coverage width in pixels, including the padding.
    width: u32,
    /// Coverage height in pixels, including the padding.
    height: u32,
    /// Horizontal bearing: offset from the pen to the coverage's left edge.
    bearing_x: i32,
    /// Vertical bearing: offset from the baseline to the coverage's top edge.
    bearing_y: i32,
    /// Advance width in pixels.
    advance: f32,
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
/// Glyphs are packed into shelves (rows). When the atlas fills up it **grows** —
/// to the next power of two, and no past [`GlyphAtlas::new`]'s ceiling — and the
/// shelves are re-packed into the larger texture; only when it is as large as it
/// is allowed to get does it evict, dropping the least recently used shelf, its
/// glyphs, its pixels, and returning its vertical span to the free list for
/// reuse. Growth comes first because a glyph the atlas cannot hold is a glyph
/// that is not drawn, and a bigger texture is cheaper than a hole; eviction
/// comes second because it is the only mechanism here that loses a glyph.
///
/// A glyph that still cannot be placed at the ceiling is **counted**, not
/// dropped in silence: see [`GlyphAtlas::dropped`]. The atlas is CPU-side; the
/// renderer uploads [`GlyphAtlas::take_dirty_pixels`] to a GL texture.
pub struct GlyphAtlas {
    /// The current texture size, square. Doubles on a grow.
    size: u32,
    /// The size a grow may not pass, and therefore where growth stops.
    max: u32,
    pixels: Vec<u8>,
    /// Shelves in recency order: the front is the least recently used.
    rows: Vec<Row>,
    /// Vertical spans freed by eviction, in pixels.
    free: Vec<FreeSpan>,
    /// The next unused pixel row, below every shelf ever allocated.
    next_y: u32,
    glyphs: HashMap<GlyphKey, Entry>,
    dirty: bool,
    /// The glyphs the atlas has refused, as keys. A set and not a count — see
    /// [`GlyphAtlas::dropped`]. `HashSet` because the neighbouring map is one
    /// and this is never iterated, so nothing wants an order.
    refused: HashSet<GlyphKey>,
}

impl GlyphAtlas {
    /// Creates an empty `size` × `size` atlas that may grow to `max` × `max`.
    ///
    /// **`max` is a ceiling, not a target, and `max` below `size` means no
    /// growth at all** — which is how the caller says "this driver, or this
    /// deployment, has no room for a bigger atlas" without a second constructor
    /// and a second name for the same atlas. The renderer reads the ceiling from
    /// `GL_MAX_TEXTURE_SIZE` and from its own documented maximum, so the atlas
    /// cannot be asked to grow into a texture the driver would refuse.
    #[must_use]
    pub fn new(size: u32, max: u32) -> Self {
        GlyphAtlas {
            size,
            max,
            pixels: vec![
                0;
                usize::try_from(size).unwrap_or(0) * usize::try_from(size).unwrap_or(0)
            ],
            rows: Vec::new(),
            free: Vec::new(),
            next_y: 0,
            glyphs: HashMap::new(),
            dirty: false,
            refused: HashSet::new(),
        }
    }

    /// Returns the atlas texture size in pixels.
    #[must_use]
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Returns how many distinct glyphs the atlas is refusing to draw because it
    /// is at its ceiling and could not free a shelf for them.
    ///
    /// **This is the report requirement 6 of `TASK_UI_PRIM_31` asks for, and the
    /// number counts glyphs rather than events — which is the whole policy, and
    /// the difference between a number that means something and one that does
    /// not.** A refused glyph *is* asked for again: `get_or_insert` is a cache
    /// miss followed by a pack, a refused key is not in the map, and the text
    /// pass re-expands the batch every frame — so a counter incremented per
    /// refusal would report one unrasterizable glyph as 3 600 after three
    /// seconds at 60 fps, and an operator could not tell that from a page of
    /// holes. **So the keys are what is kept**, and asking for a refused glyph
    /// again changes nothing about the number.
    ///
    /// **A key leaves the set the moment it can be packed.** The number answers
    /// *what is not being drawn now*, not *what ever went wrong*: an eviction can
    /// free the shelf a large glyph wanted, and a glyph that packs afterwards is
    /// being drawn, so leaving it in the set would make the report lie about the
    /// screen.
    ///
    /// The rest of the policy: a refused glyph is not drawn; the pen still
    /// advances by the advance the font reports for that character, so a run keeps
    /// its spacing and one missing glyph does not reflow the rest of the line;
    /// nothing is queued, because a shelf that could not be freed once will not be
    /// freed by a queue; and the renderer surfaces this count through
    /// [`crate::render::Renderer::dropped_glyphs`], so a run that lost glyphs
    /// says so rather than drawing a page of holes and looking like a font bug.
    ///
    /// **An eviction is not a refusal.** A glyph whose shelf is evicted is not
    /// lost: it is still in the run that asked for it next frame, and the atlas
    /// rasterizes it again. Counting those would make the number report the
    /// atlas working as designed.
    #[must_use]
    pub fn dropped(&self) -> u32 {
        u32::try_from(self.refused.len()).unwrap_or(u32::MAX)
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
    /// One function builds every key, so a cache that cannot tell two fonts apart
    /// would have to be wrong here rather than in one lookup: the size is the
    /// rounded pixel count because that is the size the font is actually set to,
    /// and the font is part of the key because the atlas is shared by all of them.
    fn key_for(ch: char, size: f32, font: FontId) -> GlyphKey {
        GlyphKey::Glyph {
            ch,
            size: f32_to_u32(size.max(0.0)).max(1),
            font,
        }
    }

    /// The key the replacement glyph at `size` is cached under.
    ///
    /// The same rounding and the same floor as [`GlyphAtlas::key_for`], written out
    /// rather than borrowed, because the two are different variants of one key: a
    /// function that took a character and returned a `GlyphKey` for both would
    /// have to invent one for the replacement, which is the fabrication the enum
    /// exists to avoid.
    fn replacement_key(size: f32) -> GlyphKey {
        GlyphKey::Replacement {
            size: f32_to_u32(size.max(0.0)).max(1),
        }
    }

    /// Returns the placement of `ch` at `size` in `font`, rasterizing and
    /// packing it if it is not already in the atlas.
    ///
    /// The same character at the same size in a different font is a **different**
    /// glyph — different coverage, different bearings, different advance — so it
    /// is packed separately and looked up separately; see `GlyphKey`.
    ///
    /// Returns `None` if the font has no glyph for `ch`, **which is what a space
    /// always returns**: a space rasterizes to nothing, so there is no quad to
    /// draw and no placement to read an advance from, while the space still
    /// occupies width. The caller reads the advance from the font instead; see
    /// `crate::render`'s `advance_for`.
    pub fn get_or_insert(
        &mut self,
        ch: char,
        size: f32,
        font: FontRef<'_>,
    ) -> Option<GlyphPlacement> {
        let key = Self::key_for(ch, size, font.id());
        self.cached(key)
            .or_else(|| self.pack(key, font.font().rasterize(ch, size)))
    }

    /// Returns the placement of the replacement glyph at `size`, packing
    /// [`replacement_bitmap`] if it is not already in the atlas.
    ///
    /// **This is what a character no font in a chain covers is drawn from**, and it
    /// returns a placement like any other rather than a `None`: the caller has
    /// already asked the chain and every font said no, so a `None` here would put
    /// the hole straight back that the replacement exists to fill. The only way it
    /// returns `None` is the atlas being at its ceiling and unable to free a
    /// shelf for the box, in which case the character is dropped, the run
    /// continues at the advance the font reports, and the refusal is counted in
    /// [`GlyphAtlas::dropped`] rather than left to be noticed as a hole on the
    /// screen.
    pub fn get_or_insert_replacement(&mut self, size: f32) -> Option<GlyphPlacement> {
        let key = Self::replacement_key(size);
        self.cached(key)
            .or_else(|| self.pack(key, Some(replacement_bitmap(size))))
    }

    /// Returns the placement stored under `key`, marking its shelf most recently
    /// used.
    fn cached(&mut self, key: GlyphKey) -> Option<GlyphPlacement> {
        let entry = *self.glyphs.get(&key)?;
        self.touch_row(entry.y);
        Some(self.placement_of(&entry))
    }

    /// Returns where `entry`'s pixels are in the atlas **as it is now**.
    ///
    /// The one place a UV is computed, which is what keeps a grow from having to
    /// find every stale one: an entry holds pixels, and the size they are
    /// divided by is read here, at the moment the placement is handed out.
    fn placement_of(&self, entry: &Entry) -> GlyphPlacement {
        let size = u32_to_f32(self.size);
        GlyphPlacement {
            u0: u32_to_f32(entry.x) / size,
            v0: u32_to_f32(entry.y) / size,
            u1: u32_to_f32(entry.x + entry.width) / size,
            v1: u32_to_f32(entry.y + entry.height) / size,
            row_y: entry.y,
            width: entry.width,
            height: entry.height,
            bearing_x: entry.bearing_x,
            bearing_y: entry.bearing_y,
            advance: entry.advance,
        }
    }

    /// Pads, packs and records `bitmap` under `key`, and returns where it went.
    ///
    /// One function for both arms of [`GlyphKey`], because padding, packing,
    /// blitting and shelf bookkeeping are four steps that have to happen in that
    /// order for every entry, and a second copy of them is a second copy of the
    /// atlas's only state machine. `None` for a bitmap the font did not produce
    /// and one the atlas cannot fit.
    ///
    /// **The two `None`s are different events and only one of them is counted.**
    /// A font with no glyph for a character — which is what a space always is —
    /// has nothing to store and nothing to report, because the caller already
    /// knows it asked for a character nothing covers; a bitmap the atlas refuses
    /// at its ceiling is a glyph this renderer failed to draw, and that is what
    /// [`GlyphAtlas::dropped`] counts.
    fn pack(&mut self, key: GlyphKey, bitmap: Option<GlyphBitmap>) -> Option<GlyphPlacement> {
        let padded = pad_bitmap(&bitmap?, COVERAGE_PAD);
        let glyph = make_coverage(&padded);
        let Some((x, y)) = self.allocate(glyph.width, glyph.height) else {
            self.refused.insert(key);
            return None;
        };
        // A glyph that fits again is being drawn, so it is no longer refused —
        // see `dropped`.
        self.refused.remove(&key);
        self.blit(x, y, &glyph);
        let entry = Entry {
            x,
            y,
            width: glyph.width,
            height: glyph.height,
            bearing_x: glyph.bearing_x,
            bearing_y: glyph.bearing_y,
            advance: glyph.advance,
        };
        self.glyphs.insert(key, entry);
        if let Some(row) = self.row_at_mut(y) {
            row.glyphs.push(key);
        }
        Some(self.placement_of(&entry))
    }

    /// Finds a shelf for a `w` × `h` glyph and returns its top-left corner,
    /// **growing the atlas first and evicting only when it cannot grow.**
    ///
    /// The order is the whole of the change this task exists for. Evicting a
    /// shelf that holds a hundred glyphs to make room for one is the right answer
    /// to an atlas that is as large as it is allowed to get and no earlier, and
    /// a grow is a re-pack rather than a re-alloc, so the glyphs already in the
    /// atlas survive it. Each of the three outcomes ends the loop: `fit`
    /// succeeds, `grow` makes `self.size` strictly larger and it is bounded by
    /// the ceiling, and an eviction removes a shelf and there are finitely many.
    fn allocate(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        loop {
            if let Some(found) = self.fit(w, h) {
                return Some(found);
            }
            if self.grow(w, h) {
                continue;
            }
            self.evict_lru_row()?;
        }
    }

    /// Grows the atlas one step if a `w` × `h` glyph could fit in a bigger one,
    /// and returns whether it did.
    ///
    /// **To the next power of two above the current size, and no further than the
    /// ceiling.** A power of two because every size in the sequence divides the
    /// next, so the sizes the atlas has held divide the size it is growing into
    /// and nothing has to be rounded on the way; the ceiling because a texture
    /// that grows without one grows until the driver refuses the upload, and a GL
    /// error nothing reads is the failure mode `.ai/NEVERAGAIN.md` has already
    /// recorded twice.
    ///
    /// **It rounds the current size up to the next power of two, and not up its own
    /// double.** `next_power_of_two` is idempotent on a power of two — it answers
    /// 64 with 64 — so asking it for "the next power of two" of a 64-pixel atlas
    /// asks for the size the atlas already is, and growth is refused forever at
    /// the size it started: every size this atlas holds after the first step *is*
    /// a power of two, so the second step would be the one that never happened.
    /// Asking for the power of two above `size + 1` is the same question with the
    /// idempotence taken out, and it is what puts the 128 between a 100-pixel
    /// start and a 200-pixel ceiling.
    ///
    /// **A glyph that would not fit the ceiling does not grow the atlas at all.**
    /// It is refused now rather than after allocating 16 MB per doubling to learn
    /// what its own width already says — which is the difference between a
    /// refusal and a 16 MB allocation on the way to the same refusal.
    fn grow(&mut self, w: u32, h: u32) -> bool {
        if w.saturating_add(ATLAS_PAD) > self.max || h > self.max {
            return false;
        }
        // `checked_next_power_of_two` is `None` only for a size that would
        // overflow a `u32`, which no texture is; the ceiling is the answer then,
        // and `min` bounds it either way.
        let next = self
            .size
            .saturating_add(1)
            .checked_next_power_of_two()
            .unwrap_or(self.max)
            .min(self.max);
        if next <= self.size {
            return false;
        }
        self.repack(next);
        true
    }

    /// Re-packs every live shelf into a `size` × `size` atlas.
    ///
    /// **A re-pack, not a re-allocation, and the difference is every glyph
    /// already in the texture.** Each one carries UVs into the texture it was
    /// written to and each shelf carries the `y` that the LRU order and
    /// [`GlyphAtlas::evict_lru_row`] address it by, so dropping the pixels and
    /// keeping the bookkeeping would leave a run sampling a texture that no
    /// longer exists.
    ///
    /// **The shelves are laid out again, tight against the top, in the order
    /// they were in before** — which is recency order, oldest first, so the
    /// eviction order survives the move as well. They fit because they fitted
    /// before: their bands are disjoint spans of `[0, size)`, so their heights
    /// sum to less than the old size, which is smaller than the new one. The
    /// pixels come out of the old texture rather than out of a copy of every
    /// glyph, because the atlas holds coverage only as pixels and a shelf's band
    /// can be copied whole — a shelf's glyphs are packed contiguously from its
    /// left edge and nothing else is ever written into it.
    ///
    /// **The free spans are handed back rather than translated.** A span is a
    /// hole an eviction left, and once the shelves have moved, its coordinates no
    /// longer describe one: in the layout above they can land on top of a shelf
    /// that was moved there, and the next glyph to take it would be drawn into
    /// another glyph's coverage. What the spans bought is space, and the layout
    /// above is that space contiguous.
    fn repack(&mut self, size: u32) {
        let old_size = self.size;
        let old = std::mem::replace(
            &mut self.pixels,
            vec![0; usize::try_from(size).unwrap_or(0) * usize::try_from(size).unwrap_or(0)],
        );
        let mut next_y: u32 = 0;
        let mut packed = Vec::with_capacity(self.rows.len());
        for row in std::mem::take(&mut self.rows) {
            let y = next_y;
            next_y = next_y.saturating_add(row.height);
            self.copy_shelf(&old, old_size, size, y, &row);
            for key in &row.glyphs {
                if let Some(entry) = self.glyphs.get_mut(key) {
                    entry.y = y;
                }
            }
            packed.push(Row { y, ..row });
        }
        self.rows = packed;
        self.next_y = next_y;
        self.free.clear();
        self.size = size;
        // The whole new texture is new as far as the renderer is concerned: every
        // shelf may have moved, and `take_dirty_pixels` hands out the whole
        // buffer rather than a region, so the flag is the only thing that says
        // the upload has to happen.
        self.dirty = true;
    }

    /// Copies `row`'s packed band from the `old_size` texture into the `size` one
    /// at `y`.
    ///
    /// The band is `row.x` wide and `row.height` tall: everything the shelf has
    /// written, and nothing it has not, because `fit` only ever advances a
    /// shelf's cursor and only ever blits inside it.
    fn copy_shelf(&mut self, old: &[u8], old_size: u32, size: u32, y: u32, row: &Row) {
        let from_stride = usize::try_from(old_size).unwrap_or(0);
        let to_stride = usize::try_from(size).unwrap_or(0);
        let width = usize::try_from(row.x.min(size)).unwrap_or(0);
        let height = usize::try_from(row.height).unwrap_or(0);
        for band in 0..height {
            let from = usize::try_from(row.y).unwrap_or(0) + band;
            let to = usize::try_from(y).unwrap_or(0) + band;
            if from >= from_stride || to >= to_stride {
                break;
            }
            let src = from * from_stride;
            let dst = to * to_stride;
            if src + width <= old.len() && dst + width <= self.pixels.len() {
                self.pixels[dst..dst + width].copy_from_slice(&old[src..src + width]);
            }
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
        let atlas = GlyphAtlas::new(64, 64);
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
        let mut atlas = GlyphAtlas::new(64, 64);
        let (x0, _) = atlas.allocate(8, 8).unwrap();
        let (x1, _) = atlas.allocate(8, 8).unwrap();
        assert_eq!(x1, x0 + 8 + ATLAS_PAD, "the gap is a whole padding wide");
    }

    #[test]
    fn an_evicted_shelf_clears_its_pixels() {
        let mut atlas = GlyphAtlas::new(64, 64);
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
        let mut atlas = GlyphAtlas::new(64, 64);
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
        let mut atlas = GlyphAtlas::new(16, 16);
        assert!(atlas.fit(17, 8).is_none(), "wider than the atlas");
        assert!(atlas.fit(8, 17).is_none(), "taller than the atlas");
        // Every shelf is evicted in turn, and then there is nothing left to
        // evict: allocation gives up rather than looping forever.
        assert!(atlas.fit(8, 8).is_some());
        assert!(atlas.allocate(15, 15).is_none());
    }

    #[test]
    fn the_pixels_are_offered_once_per_change() {
        let mut atlas = GlyphAtlas::new(64, 64);
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
        cache.set('b', 20, 3.0);
        assert_eq!(clone.get('b', 20), Some(3.0), "and the other way round");
        assert_eq!(cache.len(), 2, "one cache, not two");
    }
}

/// The tests below are about the atlas growing: what happens to the glyphs that
/// were already in it when it does, where it stops, and what it says when it
/// cannot take a glyph at all.
///
/// **No font file, and none is needed** — which is the point of testing growth
/// here rather than through the renderer. A glyph reaches the atlas as a
/// [`GlyphBitmap`] and everything after that is the atlas's own state machine,
/// so [`GlyphAtlas::pack`] can be called with a bitmap a test builds. That is
/// also why the coverage in these tests is per-pixel rather than per-rect: the
/// acceptance criterion is that a glyph placed before the grow **still addresses
/// its own pixels** afterwards, and "the size went up" is not that.
#[cfg(test)]
mod growth_tests {
    use super::*;

    /// A bitmap of `w` × `h` pixels whose every pixel differs from 0, shifted by
    /// `seed`.
    ///
    /// **The pattern is the assertion, and what makes it work is that it is not
    /// flat.** A glyph packed as a solid 255 would read back correctly from *any*
    /// address inside any glyph-sized rectangle, so a re-pack that moved a glyph,
    /// or a UV that divided by the wrong size, would return 255 and pass. A
    /// per-pixel pattern cannot.
    ///
    /// **It is not injective, and this file does not claim it is.** The values run
    /// `1 + (x·7 + y·13 + seed·29) % 254`, so a 20 × 20 glyph — 400 pixels against
    /// 254 available values — has 146 pixels sharing a value with another. What
    /// carries the assertion is that `read_back` compares the glyph's **whole
    /// rectangle including its transparent padding**: a glyph addressed one pixel
    /// off returns a different border as well as a different middle, and the
    /// padding is a row and a column of zeros nothing else in the atlas has.
    fn bitmap(w: u32, h: u32, seed: u32) -> GlyphBitmap {
        let mut pixels = Vec::new();
        for y in 0..h {
            for x in 0..w {
                // Never 0: the padding the atlas stores around a glyph is
                // transparent, so a 0 could not be told from a pixel that was
                // never written.
                let value = 1 + (x * 7 + y * 13 + seed * 29) % 254;
                pixels.push(u8::try_from(value).unwrap_or(0));
            }
        }
        GlyphBitmap {
            width: w,
            height: h,
            bearing_x: 0,
            bearing_y: i32::try_from(h).unwrap_or(0),
            advance: u32_to_f32(w),
            pixels,
        }
    }

    /// The coverage `pack` stores for `source`: the same bitmap with the atlas's
    /// padding around it, as [`pad_bitmap`] and [`make_coverage`] produce it.
    fn padded_pixels(source: &GlyphBitmap) -> Vec<u8> {
        pad_bitmap(source, COVERAGE_PAD).pixels
    }

    /// Reads back the pixels `placement` addresses, **through its UVs**, the way
    /// the text shader does.
    ///
    /// The UVs rather than the atlas's private entry, because the UVs are what
    /// the renderer samples with and the whole of what a re-pack can get wrong.
    /// The sizes these tests use are powers of two, for which the division and
    /// the multiplication back are exact — so the rounding here cannot hide a
    /// placement that is a pixel off.
    fn read_back(atlas: &GlyphAtlas, placement: &GlyphPlacement) -> Vec<u8> {
        let size = u32_to_f32(atlas.size());
        let x0 = usize::try_from(f32_to_u32((placement.u0 * size).round())).unwrap_or(0);
        let y0 = usize::try_from(f32_to_u32((placement.v0 * size).round())).unwrap_or(0);
        let w = usize::try_from(placement.width).unwrap_or(0);
        let h = usize::try_from(placement.height).unwrap_or(0);
        let stride = usize::try_from(atlas.size()).unwrap_or(0);
        let mut out = Vec::new();
        for row in 0..h {
            let start = (y0 + row) * stride + x0;
            out.extend_from_slice(&atlas.pixels()[start..start + w]);
        }
        out
    }

    /// A key for the `index`th glyph of a run of tests, distinct from every
    /// other **because the size differs**.
    ///
    /// **The size and not the character, because 26 letters is not enough.** The
    /// tests below pack up to 80 glyphs, and a key that repeats is worse than a
    /// key that is missing: `pack` is only reached on a cache miss, so packing
    /// the same key twice would file one entry under it in two shelves, and the
    /// atlas's two copies of a glyph's shelf would disagree — which is the kind
    /// of corruption that only shows up as a glyph that disappears much later.
    fn key(index: u32) -> GlyphKey {
        GlyphAtlas::key_for('a', u32_to_f32(16 + index), FontId(0))
    }

    /// Packs `count` glyphs of `w` × `h`, from `from` upwards, and returns the
    /// atlas's size after each one.
    ///
    /// **A bounded loop, and that is not a convenience.** `allocate` grows
    /// before it evicts, so a "fill the atlas" loop written as
    /// `while allocate(..).is_some() {}` does not end when the atlas is full —
    /// it ends when the *ceiling* is reached, and at the ceiling `allocate`
    /// evicts a shelf and succeeds again, so the loop never ends at all. The
    /// sizes are returned rather than asserted inside, because the sequence of
    /// them is what two of the tests below are about.
    fn pack_from(atlas: &mut GlyphAtlas, w: u32, h: u32, from: u32, count: u32) -> Vec<u32> {
        let mut sizes = Vec::new();
        for index in from..from + count {
            let packed = atlas.pack(key(index), Some(bitmap(w, h, index)));
            assert!(packed.is_some(), "glyph {index} is packed");
            sizes.push(atlas.size());
        }
        sizes
    }

    /// The sizes in `sizes` with the repetitions removed, in order.
    fn steps(sizes: &[u32]) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for size in sizes {
            if out.last() != Some(size) {
                out.push(*size);
            }
        }
        out
    }

    /// The glyphs on each shelf, in the order the atlas keeps its shelves — which is
    /// the eviction order, oldest first.
    fn order_of_shelves(atlas: &GlyphAtlas) -> Vec<GlyphKey> {
        atlas
            .rows
            .iter()
            .flat_map(|row| row.glyphs.iter().copied())
            .collect()
    }

    /// The `y` of each shelf, in the order the atlas keeps them.
    fn ys_of_shelves(atlas: &GlyphAtlas) -> Vec<u32> {
        atlas.rows.iter().map(|row| row.y).collect()
    }

    #[test]
    fn a_glyph_packed_before_a_grow_still_addresses_its_own_pixels() {
        // The acceptance criterion this task exists for, and the one that can only
        // be met by reading pixels back: **a grow is a re-pack, and every glyph
        // already in the atlas has to come out of it addressing its own
        // coverage.**
        //
        // 20 × 20 bitmaps become 22 × 22 of coverage, so a 64-pixel atlas holds
        // two per shelf and two shelves — four glyphs — and the fifth is the one
        // that cannot fit. The count was measured rather than reasoned about in
        // the prose above, because it is what makes "the fifth glyph grew it"
        // true: three shelves would need 66 pixels and the atlas has 64.
        let mut atlas = GlyphAtlas::new(64, 256);
        let mut packed: Vec<(GlyphKey, Vec<u8>)> = Vec::new();
        for index in 0..4_u32 {
            let source = bitmap(20, 20, index);
            let expected = padded_pixels(&source);
            let placement = atlas
                .pack(key(index), Some(source))
                .unwrap_or_else(|| panic!("glyph {index} is packed"));
            assert_eq!(
                read_back(&atlas, &placement),
                expected,
                "glyph {index} reads back its own coverage the moment it is packed, \
                 at row_y {} of a {}-pixel atlas",
                placement.row_y,
                atlas.size()
            );
            packed.push((key(index), expected));
        }
        assert_eq!(atlas.size(), 64, "four of them fit, and it did not grow");

        let after = pack_from(&mut atlas, 20, 20, 4, 2);
        assert_eq!(
            after,
            vec![128, 128],
            "the fifth glyph could not fit in 64, so the atlas doubled and the sixth \
             went into the larger one"
        );
        assert_eq!(
            atlas.glyphs.len(),
            6,
            "a grow loses no entries: it re-packs, it does not re-allocate"
        );
        for (key, expected) in &packed {
            let placement = atlas
                .cached(*key)
                .unwrap_or_else(|| panic!("{key:?} survived the grow"));
            assert_eq!(
                read_back(&atlas, &placement),
                *expected,
                "{key:?} still addresses its own pixels after the grow — UVs ({}, {}) \
                 in a {}-pixel atlas",
                placement.u0,
                placement.v0,
                atlas.size()
            );
        }
    }

    #[test]
    fn growth_doubles_to_a_power_of_two_and_stops_at_the_maximum() {
        // Both halves of the criterion at once, and with **the whole sequence**
        // rather than one observation of it: a starting size that is not a power
        // of two, a ceiling that is not one either, and the two steps between
        // them. Each number here is one the growth rule has to produce.
        let mut atlas = GlyphAtlas::new(100, 200);
        let sizes = pack_from(&mut atlas, 20, 20, 0, 80);
        assert_eq!(
            steps(&sizes),
            vec![100, 128, 200],
            "the sizes it passed through, starting at the size it was built at: it \\
             grows to the next power of two (128) rather than straight to the \\
             ceiling, and then to the ceiling (200) rather than past it — {sizes:?}"
        );
        // And it is at the ceiling for good: asking again changes nothing, which
        // is the half of the criterion that "it stopped growing once" would miss
        // if the atlas grew on the *next* ask rather than this one.
        assert!(
            !atlas.grow(20, 20),
            "a grow at the ceiling is refused, not deferred"
        );
        assert_eq!(atlas.size(), 200);
        for _ in 0..8 {
            assert!(!atlas.grow(20, 20), "and it stays refused");
        }
        assert_eq!(atlas.size(), 200, "so it never moves again");
    }

    #[test]
    fn a_grow_dirties_the_whole_new_texture() {
        // The flag is the only thing that carries a grow to the GPU, and this is
        // the one case where it cannot be blitted into being true: the atlas is
        // clean, `allocate` grows it, and nothing is blitted — so the flag has to
        // be the grow's own doing.
        let mut atlas = GlyphAtlas::new(64, 256);
        pack_from(&mut atlas, 20, 20, 0, 4);
        assert!(
            atlas.take_dirty_pixels().is_some(),
            "four glyphs were placed"
        );
        assert!(
            atlas.take_dirty_pixels().is_none(),
            "and the atlas is clean again, so what dirties it below is the grow alone"
        );
        let placed = atlas.allocate(22, 22);
        assert!(placed.is_some(), "the fifth glyph grows rather than fails");
        assert_eq!(atlas.size(), 128, "and it doubled");
        let pixels = atlas
            .take_dirty_pixels()
            .expect("a grow dirties the atlas even though nothing was blitted");
        assert_eq!(
            pixels.len(),
            128 * 128,
            "and what is offered for upload is the whole new texture, not the part of \
             it that moved: the renderer's upload is given the whole buffer and the \
             current size, and this flag is the only thing that says it has to run"
        );
    }

    #[test]
    fn a_glyph_too_large_for_the_ceiling_is_refused_and_counted() {
        // Requirement 6, and the half of it that is not a test of a counter: the
        // refusal must not cost four re-packs to arrive at.
        let mut atlas = GlyphAtlas::new(64, 512);
        assert_eq!(atlas.dropped(), 0, "nothing has been refused yet");
        // **Past the ceiling, not merely past the current size** — a glyph that
        // only does not fit *yet* is a grow, and these two are the refusal.
        assert!(
            atlas.pack(key(0), Some(bitmap(600, 20, 0))).is_none(),
            "600 pixels will not fit a 512-pixel ceiling"
        );
        assert_eq!(atlas.dropped(), 1, "and it is counted");
        assert_eq!(
            atlas.size(),
            64,
            "**and the atlas did not grow to find out.** Three doublings from 64 is \
             a 512-pixel texture allocated and re-packed to arrive at the same \
             refusal, and the glyph's own width already said so"
        );
        assert!(
            atlas.pack(key(1), Some(bitmap(20, 600, 1))).is_none(),
            "and a glyph too tall for it is refused on the same count rather than \
             growing until it is tall enough"
        );
        assert_eq!(
            atlas.dropped(),
            2,
            "and the count is a count, not a flag: two refusals are two"
        );
        assert!(
            atlas.glyphs.is_empty() && atlas.rows.is_empty(),
            "and a refused glyph is not half-stored — no entry and no shelf — or the \
             next glyph would be drawn into it"
        );
    }

    #[test]
    fn a_glyph_the_font_has_no_bitmap_for_is_not_a_refusal() {
        // The two `None`s of `pack` are one counter apart, and conflating them
        // would make the counter report spaces as lost glyphs — a space is one per
        // word, on every screen, and it is not a loss.
        let mut atlas = GlyphAtlas::new(64, 64);
        assert!(atlas.pack(key(0), None).is_none(), "a space");
        assert_eq!(
            atlas.dropped(),
            0,
            "which is not drawn and not counted: it has no quad, and the pen is \
             given the font's own advance instead"
        );
        assert!(
            atlas.glyphs.is_empty(),
            "and nothing about it was stored either"
        );
    }

    #[test]
    fn a_glyph_refused_every_frame_is_counted_once_and_not_asked_about_again() {
        // **The review's finding, as a test.** A refused glyph *is* asked for
        // again on every frame — `get_or_insert` is a cache miss followed by a
        // pack, and a refused key is not in the map — so a counter incremented
        // per refusal reported one unrasterizable glyph as thousands after a few
        // seconds at 60 fps, and the number requirement 6 exists to produce could
        // not tell that apart from a page of holes.
        // A ceiling above the start, so the second half of the test has somewhere to grow to
        let mut atlas = GlyphAtlas::new(64, 128);
        let too_big = bitmap(600, 20, 0);
        for _ in 0..5 {
            assert!(atlas.pack(key(0), Some(too_big.clone())).is_none());
        }
        assert_eq!(
            atlas.dropped(),
            1,
            "**five refusals of one glyph is one glyph that is not being drawn**, and \
             a number an operator reads has to mean that"
        );
        assert!(
            atlas.pack(key(1), Some(too_big)).is_none(),
            "a second glyph"
        );
        assert_eq!(
            atlas.dropped(),
            2,
            "while a *different* glyph refused is a second glyph missing from a line, \
             which is what the operator is being told"
        );
        // And the number is a live reading rather than a ledger: a glyph that can
        // be placed again is being drawn, so it must leave the report.
        assert!(atlas.grow(20, 20), "the atlas grows");
        assert!(atlas.pack(key(0), Some(bitmap(20, 20, 0))).is_some());
        assert_eq!(
            atlas.dropped(),
            1,
            "so the glyph that now packs is off the report, and the one that still \
             does not fit stays on it"
        );
    }

    #[test]
    fn eviction_still_frees_a_shelf_at_the_ceiling() {
        // Growth is not a replacement for eviction, and at the ceiling the atlas
        // has to go back to throwing glyphs away — otherwise a long session either
        // grows without bound or refuses everything.
        let mut atlas = GlyphAtlas::new(64, 64);
        let mut keys = Vec::new();
        for index in 0..4_u32 {
            atlas
                .pack(key(index), Some(bitmap(20, 20, index)))
                .unwrap_or_else(|| panic!("glyph {index} is packed"));
            keys.push(key(index));
        }
        assert_eq!(atlas.size(), 64, "four glyphs fill it, and it did not grow");
        // **Which shelf each glyph is in, measured rather than assumed.** Two per
        // shelf at 22 pixels in a 64-pixel atlas, so the first two share one and
        // an eviction takes both — and a test that assumed the oldest glyph was
        // alone on its shelf would be asserting a packing this atlas does not do.
        let shelf_of: Vec<u32> = keys
            .iter()
            .map(|key| {
                atlas
                    .cached(*key)
                    .unwrap_or_else(|| panic!("{key:?} is packed"))
                    .row_y
            })
            .collect();
        assert_eq!(
            shelf_of,
            vec![0, 0, 22, 22],
            "two glyphs per shelf, two shelves"
        );
        let seventh = atlas
            .pack(key(4), Some(bitmap(20, 20, 4)))
            .expect("at the ceiling the atlas evicts rather than refuses");
        assert_eq!(
            atlas.size(),
            64,
            "and it is still 64: a ceiling is not a suggestion"
        );
        for (index, key) in keys.iter().enumerate() {
            let gone = atlas.cached(*key).is_none();
            assert_eq!(
                gone,
                index < 2,
                "glyph {index} is on the least recently used shelf, so it went with \
                 it, and the two on the shelf above stayed: a shelf is the unit of \
                 eviction, which costs two glyphs here and not one"
            );
        }
        assert_eq!(
            atlas.dropped(),
            0,
            "and an eviction is not a refusal: the glyph is rasterized again the \
             next frame the run asks for it, which is what LRU is for"
        );
        let (x, y) = (
            usize::try_from(f32_to_u32((seventh.u0 * u32_to_f32(atlas.size())).round()))
                .unwrap_or(0),
            usize::try_from(seventh.row_y).unwrap_or(0),
        );
        assert_eq!(
            (x, y),
            (0, 0),
            "and it landed at the origin of the shelf the eviction returned to the \
             free list, which is what reuse means"
        );
        assert_eq!(
            read_back(&atlas, &seventh),
            padded_pixels(&bitmap(20, 20, 4)),
            "with its own coverage under it: the eviction cleared the shelf before it \
             was handed back, so the new glyph sampled nothing of the old one"
        );
    }

    #[test]
    fn the_least_recently_used_shelf_is_still_the_first_one_after_a_grow() {
        // The other half of a re-pack. A shelf is addressed by its `y`, the
        // recency order *is* the order shelves sit in `rows`, and a grow moves
        // every one of them. If the move lost the order, the next eviction would
        // take a hot shelf: invisible on screen — the text is redrawn, all of it
        // correct — and it costs a shelf's worth of rasterization per eviction.
        let mut atlas = GlyphAtlas::new(64, 256);
        // Tall, thin glyphs: one per shelf, so each key names its own shelf.
        let mut keys = Vec::new();
        for index in 0..4_u32 {
            atlas
                .pack(key(index), Some(bitmap(60, 4, index)))
                .unwrap_or_else(|| panic!("glyph {index} is packed"));
            keys.push(key(index));
        }
        assert_eq!(atlas.rows.len(), 4, "four shelves, one glyph each");
        // **The middle shelf is the one touched, and that is the whole difficulty
        // of this test.** Looking up the newest shelf changes nothing — it is
        // already at the back — so the recency order would go on being the order
        // the shelves sit in, and a re-pack that wrote each shelf back to its own
        // `y` would look identical to one that re-flowed them correctly. Touching
        // the second shelf makes the two orders differ, so the re-pack has to
        // *move* a shelf for the assertion below to mean anything.
        assert!(
            atlas.cached(keys[1]).is_some(),
            "the second glyph is looked up, which makes its shelf the most recent"
        );
        let recency_before: Vec<GlyphKey> = order_of_shelves(&atlas);
        let ys_before: Vec<u32> = ys_of_shelves(&atlas);
        assert_eq!(
            recency_before,
            vec![keys[0], keys[2], keys[3], keys[1]],
            "so the recency order is no longer the order the shelves sit in, which is \
             what makes the next three assertions able to fail"
        );

        // **The grow is asked for directly**, because `allocate` would take a
        // fresh shelf before it grew and that is a different test: this one is
        // about what the move does to the shelves that are already there.
        assert!(atlas.grow(60, 6), "and the atlas grows");
        assert_eq!(atlas.size(), 128);
        assert_eq!(
            atlas.rows.len(),
            4,
            "no shelf was invented or lost by the move"
        );
        assert_eq!(
            order_of_shelves(&atlas),
            recency_before,
            "**and the recency order came through it unchanged**, read as the glyphs \
             on each shelf rather than as a `y`: an order that survived by accident — \
             because every shelf went back to where it was — is not an order that \
             survived"
        );
        assert_ne!(
            ys_of_shelves(&atlas),
            ys_before,
            "which is a claim about the move and not about the order: the shelves did \
             move, so the assertion above was not the identity"
        );
        assert_eq!(
            atlas.rows[0].glyphs,
            vec![keys[0]],
            "and the least recently used glyph is still on the front shelf"
        );

        // **Every placement's `row_y` has to be the `y` of the shelf that holds
        // that glyph** — not merely some shelf that exists. The entry and the shelf
        // list are two copies of one fact, and eviction frees a shelf by the `y`
        // the entry names: an entry left on its old `y` still points at *a* shelf,
        // so an assertion that only asks "is there a shelf at this `y`" passes
        // while the glyph is on a different one, and the shelf an eviction frees is
        // a shelf nothing is in. Membership is not correspondence.
        for key in &keys {
            let placement = atlas
                .cached(*key)
                .unwrap_or_else(|| panic!("{key:?} survived the grow"));
            let shelf = atlas
                .rows
                .iter()
                .find(|row| row.glyphs.contains(key))
                .unwrap_or_else(|| panic!("{key:?} is on a shelf"));
            assert_eq!(
                placement.row_y, shelf.y,
                "{key:?} says it is at row_y {} and the shelf holding it is at y {}",
                placement.row_y, shelf.y
            );
        }

        let front = atlas.rows[0].glyphs.clone();
        assert!(atlas.evict_lru_row().is_some(), "an eviction still happens");
        for key in front {
            assert!(
                atlas.cached(key).is_none(),
                "{key:?} was on the least recently used shelf and went with it"
            );
        }
        assert_eq!(
            atlas.rows.len(),
            3,
            "and only one shelf went, so the rest of the atlas is still addressable"
        );
    }

    #[test]
    fn a_glyph_packed_after_a_grow_does_not_land_in_a_span_the_move_invalidated() {
        // **The free spans are handed back rather than moved**, and this is what
        // that costs if it is got wrong. A span is a hole an eviction left, and
        // the re-flow writes a shelf where the hole was — so a span kept across
        // the move is not a hole any more, and the next glyph to take it is
        // blitted over a live one.
        let mut atlas = GlyphAtlas::new(64, 128);
        // Four 22-pixel glyphs are two full shelves of a 64-pixel atlas.
        let mut expected: Vec<(GlyphKey, Vec<u8>)> = Vec::new();
        for index in 0..4_u32 {
            let source = bitmap(20, 20, index);
            expected.push((key(index), padded_pixels(&source)));
            atlas
                .pack(key(index), Some(source))
                .unwrap_or_else(|| panic!("glyph {index} is packed"));
        }
        assert_eq!(atlas.rows.len(), 2, "two shelves, two glyphs each");
        assert!(
            atlas.evict_lru_row().is_some(),
            "an eviction frees the first shelf's span and takes its two glyphs"
        );
        // The two on that shelf are gone, and the sweep below is over what is left.
        expected.drain(..2);
        // The re-flow now has one shelf left, and it moves down to the origin —
        // which is where the freed span is.
        assert!(atlas.grow(22, 22), "and the atlas grows to 128");
        assert_eq!(atlas.size(), 128);
        assert_eq!(atlas.next_y, 22, "the one shelf is written at the top");
        // Three of the four fill the rest of that shelf's width — it holds two, so
        // its cursor is at 46 and three more take it to 115 — and the fourth is the
        // one that has to go somewhere else.
        for index in 4..8_u32 {
            let source = bitmap(20, 20, index);
            expected.push((key(index), padded_pixels(&source)));
            atlas
                .pack(key(index), Some(source))
                .unwrap_or_else(|| panic!("glyph {index} is packed"));
        }
        let placed = atlas.rows.last().expect("a shelf").glyphs.clone();
        assert_eq!(
            placed,
            vec![key(7)],
            "so the fourth went on a shelf of its own, and the other three stayed on \
             the one the re-flow wrote"
        );
        let last = atlas.rows.last().expect("a shelf");
        assert_eq!(
            (last.y, last.glyphs.len()),
            (22, 1),
            "**at y 22, below the shelf the re-flow wrote — not into the span the \
             eviction left, which is where that shelf now is.** A glyph placed into \
             the stale span is blitted over the glyph that is already there"
        );
        // **And every glyph in the atlas still reads back its own coverage**, which
        // is what catches the blit over a neighbour. Written as a sweep over the
        // keys rather than as one named glyph because the sweep is what cannot miss
        // a shelf.
        for (key, pixels) in &expected {
            let placement = atlas
                .cached(*key)
                .unwrap_or_else(|| panic!("{key:?} is still packed"));
            assert_eq!(
                read_back(&atlas, &placement),
                *pixels,
                "{key:?} reads back its own coverage after the grow and after four \
                 more glyphs were packed around it"
            );
        }
    }

    #[test]
    fn a_grow_leaves_the_new_texture_empty_where_nothing_was() {
        // The half of the re-pack nothing else looks at: **the new texture is
        // zeroed**, so a moved glyph cannot sample the coverage of one that did
        // not move into its place, and a shelf above the moved ones is not
        // carrying whatever the old buffer held at those coordinates.
        let mut atlas = GlyphAtlas::new(64, 256);
        pack_from(&mut atlas, 20, 20, 0, 4);
        assert!(atlas.grow(20, 20), "it grows");
        assert_eq!(atlas.size(), 128);
        assert_eq!(
            atlas.pixels().len(),
            128 * 128,
            "and the texture it hands to the upload is the new one"
        );
        let ink = atlas.pixels().iter().filter(|&&pixel| pixel != 0).count();
        assert_eq!(
            ink,
            4 * 20 * 20,
            "exactly the four glyphs' own ink, and nothing else: {ink} inked pixels \
             in a 128 × 128 texture that was 64 × 64 before"
        );
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

    /// A glyph's entry, off the origin and with a size the assertions can tell
    /// apart: an entry at `x = 0` with the default metrics cannot be told from
    /// any other, which is the same reason `.ai/NEVERAGAIN.md` § *A rect's
    /// origin and a rect's extent are different numbers* exists.
    ///
    /// **An `Entry` and not a `GlyphPlacement`, because that is what the atlas
    /// stores** — the placement is derived from it, and a test that builds one
    /// by hand would be testing a struct the atlas never holds.
    fn entry(x: u32, advance: f32) -> Entry {
        Entry {
            x,
            y: 16,
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
        let mut atlas = GlyphAtlas::new(1024, 1024);
        let regular = FontId(0);
        let bold = FontId(1);
        let key_regular = GlyphAtlas::key_for('a', 20.0, regular);
        let key_bold = GlyphAtlas::key_for('a', 20.0, bold);
        assert_ne!(
            key_regular, key_bold,
            "'a' at 20 pixels in Lato-Medium and 'a' at 20 pixels in Lato-Bold \
             are different glyphs and the key has to say which face rasterized it"
        );

        atlas.glyphs.insert(key_regular, entry(8, 10.0));
        atlas.glyphs.insert(key_bold, entry(64, 12.0));

        // **Read back through `cached`, not out of the map**, because `cached` is
        // what a renderer asks and it is where the UV is derived from the entry's
        // pixel and the atlas's current size. Reading the map would skip the one
        // step this task changed, and the `u0` assertion below is about that step.
        let found_regular = atlas.cached(key_regular);
        let found_bold = atlas.cached(key_bold);
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
        let face = FontId(3);
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
    fn the_size_in_the_key_is_the_pixel_count_the_font_is_set_to() {
        // Unchanged by the font, and pinned because the atlas's own tests would
        // not notice: a float key would store the same glyph twice for two sizes
        // that round to the same pixel count.
        let GlyphKey::Glyph { size, .. } = GlyphAtlas::key_for('a', 20.4, FontId(0)) else {
            panic!("a character's key names its size");
        };
        assert_eq!(size, 20);
        let GlyphKey::Glyph { size, .. } = GlyphAtlas::key_for('a', 0.0, FontId(0)) else {
            panic!("a character's key names its size");
        };
        assert_eq!(
            size, 1,
            "and a size of zero floors to one, as it always has"
        );
    }

    #[test]
    fn the_replacement_key_is_one_size_and_names_no_character() {
        // The shape of the second arm, and the reason it is a variant rather than
        // a key with an empty character: a key built per uncovered character would
        // pack five identical boxes for a run of five.
        let key = GlyphAtlas::replacement_key(20.0);
        assert_eq!(key, GlyphAtlas::replacement_key(20.0), "one entry per size");
        assert_ne!(key, GlyphAtlas::replacement_key(21.0), "and one per size");
        assert_ne!(
            key,
            GlyphAtlas::key_for('a', 20.0, FontId(0)),
            "and it is not a character's entry, or the box would be drawn where a \
             letter was asked for and the two would evict each other"
        );
    }

    #[test]
    fn the_replacement_key_is_not_any_characters_key_and_says_so_in_its_variant() {
        // **The assertion the first version of this file did not have**, and the
        // mutation `replacement-is-keyed-per-character` is what found it: making
        // the replacement's key a `Glyph` under some character changes nothing
        // about the numbers — it is still one entry per size, and it is still
        // distinct from the keys of 'a', ' ' and U+4E2D — so both of the tests
        // above stayed green while the enum's reason for existing was gone.
        //
        // The character a hand-written key would fabricate is **U+FFFD**, because
        // that is the character the replacement *stands in for*. So this asks about
        // that one specifically, and asserts the variant as well: a key that is not
        // a `Replacement` is the defect, whatever it collides with.
        let key = GlyphAtlas::replacement_key(20.0);
        assert!(
            matches!(key, GlyphKey::Replacement { size: 20 }),
            "the replacement's key says what it is: {key:?}"
        );
        for font in 0..4_u32 {
            assert_ne!(
                key,
                GlyphAtlas::key_for('\u{fffd}', 20.0, FontId(font)),
                "and it is not U+FFFD's key in any font — the character the box \\
                 stands in for is the one a fabricated key would collide with, and \\
                 a collision there would draw the replacement *over* a font's own \\
                 replacement character wherever that font has one"
            );
        }
    }

    /// **The two tests below were deleted by task 30 and are restored here**, and
    /// the review of task 30 is what found them: `FaceIds` was removed when the
    /// identity became a font's own index, and with it the only seam through which
    /// *"every install is a new identity"* could be tested — a `Font` needs a font
    /// file, which `AGENTS.md` forbids a test to open. `FontIds` is that seam back,
    /// and the mutation `install()` returning a constant id survived the whole
    /// suite while the property was untested, which is precisely what a gate with
    /// no test looks like.
    #[test]
    fn each_installed_font_is_given_an_identity_of_its_own() {
        let mut ids = FontIds::default();
        let first = ids.issue();
        let second = ids.issue();
        assert_ne!(
            first, second,
            "two fonts cached under one identity would share every glyph in the \
             atlas, whichever weight they were rasterized from"
        );
    }

    #[test]
    fn a_reinstalled_font_gets_an_identity_the_previous_one_never_had() {
        // The rule the atlas key depends on when a renderer swaps its bold file:
        // the glyphs of the file that has just been dropped must not be found for
        // the file that is there now, and the only thing that can separate them is
        // a new identity. **An id is never reused**, so replacing a weight cannot
        // hand back the identity the dropped file had — and the identity an
        // untouched font still holds is still that font's, which is the half a
        // chain still naming it depends on.
        let mut ids = FontIds::default();
        let regular = ids.issue();
        let bold = ids.issue();
        let bold_again = ids.issue();

        assert_ne!(bold_again, bold, "the replaced font is a different font");
        assert_ne!(
            bold_again, regular,
            "and its identity is not the other one's"
        );
        assert_eq!(
            regular.get(),
            0,
            "while the identity the first font still holds is the one it was given: \\
             a counter that rewrote an old id would take it away from a chain still \\
             naming that font"
        );
        assert_eq!(bold.get(), 1);
        assert_eq!(bold_again.get(), 2, "three installs, three numbers");
    }

    #[test]
    fn the_identity_and_the_fonts_own_length_are_the_same_number() {
        // **New in task 30, and it is the assertion the index-based identity
        // introduces.** `FontSet::install` pushes one font per issue, so the id
        // and the index agree by construction — and "by construction" is a comment,
        // not a check. If a future install path ever pushed without issuing, or
        // issued without pushing, the atlas would key one file's glyphs where
        // another file is looked up: a bold run drawn from the regular glyph's
        // quad, with no error anywhere.
        let mut ids = FontIds::default();
        let mut installed = Vec::new();
        for _ in 0..3 {
            let id = ids.issue();
            installed.push(id);
        }
        for (index, id) in installed.iter().enumerate() {
            assert_eq!(
                id.get(),
                u32::try_from(index).unwrap_or(0),
                "the {index}th font installed is at index {index} and carries that \
                 number, so an id is where the face is found"
            );
        }
        assert_eq!(
            ids.issued,
            u32::try_from(installed.len()).unwrap_or(0),
            "and the counter is exactly how many were installed"
        );
    }

    #[test]
    fn the_two_arms_of_the_key_do_not_collide_at_the_same_size() {
        // Every size has both a character's entries and the replacement's, and the
        // atlas holds them in one map: an arm that could be reached from the other
        // would return the wrong placement with no error.
        for size in [1.0_f32, 14.0, 20.4, 24.0] {
            let mut keys = std::collections::HashSet::new();
            for ch in ['a', '\u{4e2d}', ' '] {
                keys.insert(GlyphAtlas::key_for(ch, size, FontId(0)));
            }
            keys.insert(GlyphAtlas::replacement_key(size));
            assert_eq!(keys.len(), 4, "four distinct entries at {size} pixels");
        }
    }
}

/// The chain: which font a character is looked for in, and what happens when none
/// of them has it.
///
/// Neither half needs a font file and neither could: rasterizing needs a real
/// face, and a unit test may not open one. What *can* be tested is everything the
/// chain decides before FreeType is asked anything — the order fonts are asked in,
/// and the width an uncovered character is measured at — because the chain's rule
/// is written as a free function over a coverage callback for exactly this
/// reason. What is **not** here is anything that reads a font's own character map:
/// whether Lato covers a warning sign is a fact about a file on this host, checked
/// against the file and recorded in the module docs, not something a unit test can
/// assert.
#[cfg(test)]
mod chain_tests {
    use super::*;

    /// Coverage as a table: one row per font id and the characters that font
    /// covers. **A font with no row covers nothing**, and that is the whole of how
    /// two fonts in one chain are made to differ — written as a table rather than
    /// as one set of characters because a callback answering the same for every
    /// font cannot tell a chain from its first entry.
    fn covers_in<'a>(table: &'a [(u32, &'a str)]) -> impl FnMut(FontId, char) -> bool + 'a {
        move |id: FontId, ch: char| {
            table
                .iter()
                .any(|&(font, covered)| font == id.get() && covered.contains(ch))
        }
    }

    /// The primary of a chain and the fallbacks after it, as the arguments
    /// `pick_in_chain` takes.
    fn chain(primary: Option<u32>, fallbacks: &[u32]) -> (Option<FontId>, Vec<FontId>) {
        (
            primary.map(FontId),
            fallbacks.iter().copied().map(FontId).collect(),
        )
    }

    #[test]
    fn the_first_font_that_covers_the_character_is_the_one_that_draws_it() {
        // The mechanism: the primary says no, the first fallback says yes, and the
        // answer is that fallback's identity — which is what the atlas then keys
        // the glyph under and what the advance is measured from.
        let (primary, fallbacks) = chain(Some(0), &[1, 2]);
        let picked = pick_in_chain(
            primary,
            &fallbacks,
            &mut covers_in(&[(1, "\u{26a0}"), (2, "\u{26a0}")]),
            '\u{26a0}',
        );
        assert_eq!(picked, Some(FontId(1)));
    }

    #[test]
    fn the_primary_is_asked_before_any_fallback() {
        // A fallback that covers everything would make this invisible if the order
        // were reversed, and reversing it is the defect: the whole point of a
        // chain is that the font the caller asked for draws what it has.
        let (primary, fallbacks) = chain(Some(0), &[1]);
        let picked = pick_in_chain(
            primary,
            &fallbacks,
            &mut covers_in(&[(0, "a"), (1, "a")]),
            'a',
        );
        assert_eq!(
            picked,
            Some(FontId(0)),
            "the primary covered it, so the primary drew it"
        );
    }

    #[test]
    fn a_character_no_font_covers_is_not_a_font() {
        // The other direction, and the reason this returns an `Option` at all: no
        // font is a normal answer and it has to be distinguishable from "the first
        // font that covered it".
        let (primary, fallbacks) = chain(Some(0), &[1, 2]);
        let picked = pick_in_chain(
            primary,
            &fallbacks,
            &mut covers_in(&[(0, "a"), (1, "a"), (2, "a")]),
            '\u{4e2d}',
        );
        assert_eq!(
            picked, None,
            "no font covers it, which the caller draws as the replacement"
        );
    }

    #[test]
    fn a_chain_of_one_font_answers_for_every_character() {
        // Total in both directions, because a caller walks this per character per
        // frame and an early return would be a hole rather than an answer.
        let (primary, fallbacks) = chain(Some(0), &[]);
        let mut covers_everything = |_id: FontId, _ch: char| true;
        assert_eq!(
            pick_in_chain(primary, &fallbacks, &mut covers_everything, 'a'),
            Some(FontId(0)),
            "one font that covers everything is a whole chain"
        );
        assert_eq!(
            pick_in_chain(primary, &fallbacks, &mut covers_in(&[]), 'a'),
            None,
            "and one that covers nothing draws no font at all"
        );
    }

    #[test]
    fn an_empty_chain_draws_no_font_and_asks_no_font() {
        // The state a renderer is in before any font is installed, and the one the
        // renderer's own empty-set gate exists to avoid paying for per character.
        let empty: Vec<FontId> = Vec::new();
        let mut asked = 0_u32;
        let picked = pick_in_chain(
            None,
            &empty,
            &mut |_id: FontId, _ch: char| {
                asked += 1;
                true
            },
            'a',
        );
        assert_eq!(picked, None);
        assert_eq!(
            asked, 0,
            "and it asked nobody, so there is nothing to draw with"
        );
    }

    #[test]
    fn the_primary_is_asked_once_when_it_is_also_a_fallback() {
        // A chain built by appending the primary's own font — which is what
        // `add_fallback` with the same file gives — must not ask it twice. Asking
        // twice is free but it is two readings of one chain and the next reader
        // would have to work out whether the two can disagree.
        //
        // **The primary has to decline, or this test cannot see anything**: the
        // walk returns as soon as a font covers the character, so with a primary
        // that covers it the fallback pass never runs and the duplicate is
        // invisible. The first version of this test had the primary covering and
        // asserted one question, which passed with the filter deleted — the
        // mutation named `chain-asks-the-primary-twice` is what found it.
        let (primary, fallbacks) = chain(Some(0), &[0, 1]);
        let mut asked = Vec::new();
        let picked = pick_in_chain(
            primary,
            &fallbacks,
            &mut |id: FontId, ch: char| {
                asked.push((id, ch));
                // The primary has no glyph; the font that is also a fallback has.
                id.get() == 1 && ch == 'a'
            },
            'a',
        );
        assert_eq!(
            picked,
            Some(FontId(1)),
            "the second font in the list covers it, and it is the same font the \\
             chain already asked once"
        );
        assert_eq!(
            asked,
            vec![(FontId(0), 'a'), (FontId(1), 'a')],
            "two questions, one per font: the primary is asked first and **not** \\
             again in the fallback pass, and the third entry of the chain is never \
             reached"
        );
    }

    #[test]
    fn the_chain_is_walked_in_order_and_stops_at_the_first_hit() {
        // The order is the mechanism, so it is pinned as an order: three fallbacks
        // all covering the character, and the first one is the answer.
        let (primary, fallbacks) = chain(None, &[7, 8, 9]);
        let mut asked = Vec::new();
        let picked = pick_in_chain(
            primary,
            &fallbacks,
            &mut |id: FontId, ch: char| {
                asked.push(id.get());
                ch == 'x'
            },
            'x',
        );
        assert_eq!(picked, Some(FontId(7)));
        assert_eq!(asked, vec![7], "and the two after it were never asked");
    }

    #[test]
    fn a_family_with_no_face_of_its_own_starts_at_its_first_font() {
        // A chain of fallbacks alone is a family whose primary is its first entry —
        // which is what makes `define_family` + `add_fallback_to` a family at all.
        let family = Family {
            faces: [None, None],
            fallbacks: vec![FontId(4), FontId(5)],
            ..Family::default()
        };
        assert_eq!(family.primary_id(FontWeight::Regular), Some(FontId(4)));
        assert_eq!(
            family.primary_id(FontWeight::Bold),
            Some(FontId(4)),
            "and the weight asked for does not change it, because the family has \
             no face of either"
        );
    }

    #[test]
    fn a_familys_primary_is_the_weights_own_face_then_the_regular_one() {
        // The two clauses `resolve_slot` already owns, read through a family: they
        // are unchanged, and this says so where the chain now reads them.
        let family = Family {
            faces: [Some(FontId(1)), Some(FontId(2))],
            fallbacks: vec![FontId(3)],
            ..Family::default()
        };
        assert_eq!(family.primary_id(FontWeight::Regular), Some(FontId(1)));
        assert_eq!(family.primary_id(FontWeight::Bold), Some(FontId(2)));

        let regular_only = Family {
            faces: [Some(FontId(1)), None],
            fallbacks: vec![FontId(3)],
            ..Family::default()
        };
        assert_eq!(
            regular_only.primary_id(FontWeight::Bold),
            Some(FontId(1)),
            "a weight with no face of its own falls back to the regular one"
        );
    }

    #[test]
    fn a_family_with_nothing_in_it_has_no_primary_and_no_fallbacks() {
        // The empty chain, and `None` rather than a panic: a family id from another
        // set resolves to the default family, and the default family of an empty
        // set is empty.
        let empty = Family::default();
        assert_eq!(empty.primary_id(FontWeight::Regular), None);
        assert!(empty.fallbacks.is_empty());
        assert!(
            empty.is_empty(),
            "and it says so, which is what `FontSet::drawable` asks before handing \\
             the default family over in its place"
        );
    }

    #[test]
    fn a_family_that_holds_a_font_is_not_empty_even_with_no_face_of_its_own() {
        // The other side of the same question, and the reason the check is
        // `is_empty` rather than *"has a face for this weight"*: a family of
        // fallbacks alone is a real family, and asking for a regular face in it
        // must not classify it as empty and send the run to the default family.
        let chain_only = Family {
            faces: [None, None],
            fallbacks: vec![FontId(9)],
            ..Family::default()
        };
        assert!(!chain_only.is_empty());
        assert!(
            !Family {
                faces: [Some(FontId(1)), None],
                fallbacks: Vec::new(),
                ..Family::default()
            }
            .is_empty(),
            "and a family with one face of its own is not empty either"
        );
    }

    #[test]
    fn an_empty_family_is_replaced_by_the_default_one_where_there_is_a_font() {
        // The rule that stops a whole run from vanishing, and the review of task 30
        // is what found it missing: `draw_text_batch` `continue`s when a run's
        // family has no primary, so an empty family dropped every character of
        // every run in it — requirement 4's "never a silent hole", at the largest
        // possible size.
        //
        // **What is testable here and what is not, stated rather than blurred.**
        // Which family is *chosen* needs a font in the default family, and a test
        // may not open one — so this asserts the two halves that do not: that an
        // empty family is recognised as empty, and that `drawable` hands back the
        // family it was given when **that** family is the default one (which is
        // the case a set with no fonts at all is in, and the one that must stay a
        // dropped run). **The substitution itself is verified by the capture
        // recorded in `IMPLEMENTATION_STATE.md`, not by a test**, and
        // `an_empty_family_resolves_to_the_default_rather_than_to_nothing_is_not_
        // unit_testable` is not written here because a test that asserts a
        // substitution it cannot set up is a test that passes either way.
        let mut set = FontSet::new();
        let empty_family = set.define_family("later");
        assert_eq!(set.family("later"), empty_family);
        assert!(
            set.entry(empty_family).is_empty(),
            "a family defined and never filled is empty, which is the state the \\
             rule is about"
        );
        assert!(
            set.primary(empty_family, FontWeight::Regular).is_none(),
            "and with nothing installed at all there is genuinely nothing to draw \
             with, so the run is dropped — the one state that may be"
        );
        assert!(
            matches!(
                set.pick(empty_family, FontWeight::Regular, 'a'),
                PickedFont::Replacement
            ),
            "and a character in it is uncovered, which is a box rather than a gap"
        );
    }

    #[test]
    fn the_default_family_exists_from_the_first_moment_and_is_the_default_one() {
        // Why `FamilyId::default` needs no `Option`: `FontSet::new` makes the
        // default family, so a property built with `Default` names a family that
        // exists rather than one that has to be checked for.
        let set = FontSet::new();
        assert_eq!(set.default_family(), FamilyId::default());
        assert_eq!(
            Some(set.entry(FamilyId::default()).name.as_str()),
            Some(DEFAULT_FAMILY)
        );
    }

    #[test]
    fn a_family_named_twice_is_defined_once() {
        // Defining a family twice returns the one that is there rather than
        // replacing it: emptying a chain something else is drawing with, by
        // accident, is not a thing a caller can do by asking twice.
        let mut set = FontSet::new();
        let first = set.define_family("heading");
        let second = set.define_family("heading");
        assert_eq!(first, second);
        assert_eq!(set.family("heading"), first);
    }

    #[test]
    fn a_family_name_that_is_not_defined_is_the_default_family() {
        // The rule that keeps every existing program working: nothing here has
        // heard of families, its labels carry `FamilyId::default()`, and the
        // default family is what they are drawn in.
        let set = FontSet::new();
        assert_eq!(set.family("sans-serif"), set.default_family());
        assert_eq!(set.family(""), set.default_family());
    }

    #[test]
    fn a_family_id_this_set_never_handed_out_is_the_default_family() {
        // The handle can come from another set, or from before a family was
        // defined. Either way the run is drawn, in the default family, because a
        // run that is not drawn is a hole in the layout.
        let mut set = FontSet::new();
        let defined = set.define_family("heading");
        let never_handed_out = FamilyId::u32_from(defined.get() + 7);
        assert_eq!(
            Some(set.entry(never_handed_out).name.as_str()),
            Some(DEFAULT_FAMILY),
            "an id from beyond the table lands on the default family"
        );
    }

    #[test]
    fn the_replacement_advance_is_the_advance_the_box_is_drawn_at() {
        // The one number both halves agree on, and the assertion that holds them
        // together: the layout's width for an uncovered character comes from
        // `replacement_advance` and the drawn box's own advance comes from the
        // bitmap. Two callers, one function — and this is the test that says so,
        // at several sizes, because a rounding difference at one size can be a
        // half-pixel gap at another.
        for size in [8.0_f32, 12.0, 14.0, 18.0, 24.0, 48.0] {
            assert_eq!(
                replacement_bitmap(size).advance,
                replacement_advance(size),
                "at {size} pixels the box and the pen agree"
            );
        }
    }

    #[test]
    fn the_replacement_is_a_hollow_box_that_sits_on_the_baseline() {
        // Shape, because "draws a visible replacement" is a claim about pixels and
        // the pixels are the bitmap: every border pixel ink, the middle empty, and
        // the bearings putting it on the baseline at the pen.
        //
        // **Three sizes, and the two small ones are the point.** At 24 pixels every
        // assertion below passes whatever `REPLACEMENT_MIN` is, because the fractions
        // are far from the floor — which is how `REPLACEMENT_MIN = 2` survived: a 2×2
        // box is *solid*, and no test looked at a size where the floor binds.
        for size in [3.0_f32, 8.0, 24.0] {
            let bitmap = replacement_bitmap(size);
            let w = usize::try_from(bitmap.width).unwrap_or(0);
            let h = usize::try_from(bitmap.height).unwrap_or(0);
            assert!(w >= 3 && h >= 3, "at {size} px it is {w}x{h}, not a dot");
            for y in 0..h {
                for x in 0..w {
                    let edge = y == 0 || x == 0 || y + 1 == h || x + 1 == w;
                    let expected = if edge { 255 } else { 0 };
                    assert_eq!(
                        bitmap.pixels[y * w + x],
                        expected,
                        "at {size} px, pixel ({x}, {y}) is {}",
                        if edge { "on the outline" } else { "inside" }
                    );
                }
            }
        }
        let bitmap = replacement_bitmap(24.0);
        assert!(bitmap.pixels.contains(&255), "and there is ink in the box");
        assert!(
            bitmap.pixels.iter().all(|&p| p == 0 || p == 255),
            "which is whole pixels, because it is not a font's edge and has no \
             antialiasing to carry"
        );
        assert_eq!(bitmap.bearing_x, 0, "the box starts at the pen");
        assert_eq!(
            bitmap.bearing_y,
            i32::try_from(bitmap.height).unwrap_or(0),
            "and its bottom edge sits on the baseline"
        );
    }

    /// The numbers the two constants fix, at one size, written out.
    ///
    /// **A design choice is not a derived number, so nothing can prove it — which
    /// is exactly why it is pinned.** `REPLACEMENT_ADVANCE_EM` is 0.6 and
    /// `REPLACEMENT_HEIGHT_EM` is 0.72, and at 24 pixels that is a box 14 wide and
    /// 17 tall with an advance of 14.4. Every *other* test in this module can only
    /// show that the two halves of the replacement agree, which they would agree
    /// about at 0.06 as readily as at 0.6; a mistyped fraction would leave the
    /// suite green and put a box half the width of the character it stands in for
    /// on the screen. `.ai/NEVERAGAIN.md`'s *a strength clamped to 0..=1* entry is
    /// the same shape: assert the number the word in the spec fixes, because a
    /// shape assertion will not.
    #[test]
    fn the_replacement_box_is_the_size_its_two_constants_fix() {
        let bitmap = replacement_bitmap(24.0);
        assert_eq!(
            (bitmap.width, bitmap.height),
            (14, 17),
            "0.6 and 0.72 of 24 pixels, rounded: a box a little taller than it is \
             wide, which is the proportion that reads as 'empty'"
        );
        // **Within an ulp, and the reason is written down rather than the
        // tolerance being a number someone picked.** `0.6_f32 * 24.0` is
        // 14.400001, so an exact comparison against the decimal a reader would
        // write is an assertion about f32's arithmetic rather than about this
        // constant. The property being pinned is "0.6 of the em", and 14.4 is that
        // to the nearest representable value.
        assert!(
            (bitmap.advance - 14.4).abs() < 1e-4,
            "and an advance of 0.6 em — which is 14.4 and not 14, because the pen \
             moves by the fraction and the box is the rounded drawing of it \
             (got {})",
            bitmap.advance
        );
    }

    #[test]
    fn a_replacement_is_packed_and_returned_rather_than_dropped() {
        // **The gate this module would otherwise have none of.** Everything else
        // here is about *which* font draws a character; this is about the one path
        // that draws something no font drew, and it can only answer for itself:
        // `get_or_insert_replacement` takes no font, so a test can call it.
        //
        // The mutation it kills is one line — returning `None` — and that is the
        // original defect task 30 exists to remove: a character nothing covers,
        // answered with "no glyph", drawn as nothing. A `None` here is a silent
        // hole wearing the fix's own type.
        let mut atlas = GlyphAtlas::new(1024, 1024);
        let placement = atlas.get_or_insert_replacement(24.0);
        let placement = placement.expect("the replacement is packed, not dropped");
        // **The padded size, not the bitmap's**, and the difference is the point:
        // the atlas stores `COVERAGE_PAD` transparent pixels around every glyph so
        // linear filtering never interpolates a neighbour into it, and this entry
        // goes through the same `pad_bitmap` a rasterized glyph does. A reader who
        // expected 14 × 17 here would be right about the box and wrong about what
        // the atlas holds — the quad drawn on screen is the placement, so it is two
        // pixels wider than the ink and lands 14.4 pixels along the pen.
        assert_eq!(placement.width, 16, "14 of box plus a pixel either side");
        assert_eq!(
            placement.height, 19,
            "17 of box plus a pixel above and below"
        );
        assert_eq!(placement.advance, replacement_advance(24.0));
        assert_eq!(placement.bearing_x, -1, "moved a pixel left of the pen");
        assert_eq!(
            placement.bearing_y, 18,
            "and its bottom edge still sits on the baseline, because the padding \
             moves with the ink"
        );
        assert!(
            atlas.pixels().contains(&255),
            "and there are inked pixels in the atlas, which is what makes this a \
             drawn box rather than a placement of nothing"
        );
    }

    #[test]
    fn the_replacement_is_packed_once_per_size_however_many_characters_need_it() {
        // The other direction, and the reason the key has no character in it: five
        // uncovered characters in a row draw five boxes out of **one** atlas entry,
        // so a page of text in a script no font covers cannot evict every real
        // glyph in the atlas to store identical copies of a box.
        let mut atlas = GlyphAtlas::new(1024, 1024);
        let first = atlas
            .get_or_insert_replacement(24.0)
            .expect("the replacement is packed");
        let second = atlas.get_or_insert_replacement(24.0).expect("and again");
        assert_eq!(
            first, second,
            "the same entry, found rather than packed again"
        );
        assert_eq!(
            atlas.glyphs.len(),
            1,
            "and the atlas holds one entry for both, not two"
        );
        assert_ne!(
            atlas.get_or_insert_replacement(25.0),
            Some(first),
            "a different size is a different entry, because a glyph is rasterized \
             at the size it is drawn"
        );
        assert_eq!(atlas.glyphs.len(), 2, "which is two");
    }

    #[test]
    fn the_replacement_is_never_a_blank_or_a_single_pixel() {
        // The two sizes where a fraction of the em rounds away, and where a box
        // built from `round()` alone would be 1x1 — one solid pixel that reads as
        // debris rather than as a missing character.
        for size in [1.0_f32, 2.0, 3.0, 6.0, 0.0, -4.0] {
            let bitmap = replacement_bitmap(size);
            assert!(
                bitmap.width >= REPLACEMENT_MIN && bitmap.height >= REPLACEMENT_MIN,
                "at {size} pixels the box is {}x{}, not a dot",
                bitmap.width,
                bitmap.height
            );
            assert!(
                !bitmap.pixels.is_empty(),
                "and it has pixels at all, at {size} pixels"
            );
        }
    }

    #[test]
    fn the_replacement_advances_the_pen_rather_than_disappearing() {
        // "Whatever it is, it advances the pen", stated as the property: the hole
        // a layout leaves is as wide as the box, so the words after it do not
        // close over the gap.
        assert!(
            replacement_advance(24.0) > 0.0,
            "an uncovered character is as wide as the box drawn in it"
        );
        // **The review of task 30 deleted a line here** that read
        // `assert_eq!(replacement_advance(24.0), replacement_advance(24.0) * 3.0 / 3.0)`
        // and claimed to show "a function of the size rather than a constant". Both
        // sides are the same `f32` bits, so it was true by construction and it is
        // **not even generally true of `f32` here** — 229 of the 1999 half-pixel
        // sizes fail `x * 3.0 / 3.0 == x`. What it actually asserted was that one
        // value round-trips.
        assert_ne!(
            replacement_advance(8.0),
            replacement_advance(24.0),
            "and it is a function of the size rather than a constant, which is the \
             property the deleted line claimed and could not test"
        );
        assert!(
            replacement_advance(48.0) > replacement_advance(24.0),
            "and it grows with the text, as a character's width does"
        );
    }
}
