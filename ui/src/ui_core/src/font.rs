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

/// The key identifying a glyph in the atlas: the character and its pixel size.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct GlyphKey {
    ch: char,
    size: u32,
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

/// The glyph atlas: a square texture of signed distance fields.
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

    /// Returns the placement of `ch` at `size`, rasterizing and packing it if
    /// it is not already in the atlas.
    ///
    /// Returns `None` if the font has no glyph for `ch`.
    pub fn get_or_insert(&mut self, ch: char, size: f32, font: &Font) -> Option<GlyphPlacement> {
        let key = GlyphKey {
            ch,
            size: f32_to_u32(size.max(0.0)).max(1),
        };
        if let Some(&placement) = self.glyphs.get(&key) {
            self.touch_row(placement.row_y);
            return Some(placement);
        }
        let bitmap = font.rasterize(ch, size)?;
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
