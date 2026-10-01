//! Texture loading, decoding and placement.
//!
//! Owns the half of task 16 that is not the widget: getting pixels out of a
//! file, turning them into premultiplied RGBA, deciding where each image lives
//! on the GPU, and remembering which file is already loaded so a second
//! request for it costs nothing.
//!
//! **Nothing here talks to GL.** The atlas is a CPU-side pixel buffer and the
//! renderer uploads it, which is exactly the arrangement
//! [`GlyphAtlas`](crate::font::GlyphAtlas) uses and for the same reason: the
//! decisions about *what* is resident and *where* it sits are the ones worth
//! testing, and none of them need a display.
//!
//! # Where an image goes
//!
//! Requirement 5 asks for two places. A small image is packed into one shared
//! **atlas** texture, so a screen full of icons is one draw call and one
//! texture; an image too big for that gets **its own** texture, because a
//! photograph does not belong in a 2048² shelf allocator next to icons. A
//! [`TextureId`] says which of the two, in its top bit — see [`TextureId`], where
//! the encoding is written down rather than implied.
//!
//! # The decoder is behind a seam
//!
//! [`TextureCache::load`] takes a loader closure rather than reading the file
//! itself, and [`TextureCache::load_from_file`] is the one caller that supplies
//! the real one. That is what makes requirement 2's *"cache loaded textures
//! (don't reload if already loaded)"* a test at all: `AGENTS.md` forbids a test
//! that needs a filesystem, so the test supplies its own loader, counts its
//! calls, and asserts the second request never reaches it. It is also why
//! replacing the decoder later is one function.
//!
//! # Examples
//!
//! ```
//! use ui_core::texture::{Pixels, TextureCache, TextureError};
//!
//! let mut cache = TextureCache::new();
//! let red = Pixels::new(
//!     2,
//!     2,
//!     vec![255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255],
//! );
//!
//! // A loader that is only ever allowed to run once.
//! let mut calls = 0;
//! let mut load = |_: &std::path::Path| {
//!     calls += 1;
//!     Ok(red.clone())
//! };
//! let first = cache
//!     .load(std::path::Path::new("icon.png"), &mut load)
//!     .expect("loaded");
//! let second = cache
//!     .load(std::path::Path::new("icon.png"), &mut load)
//!     .expect("cached");
//! assert_eq!(first, second, "the second request is the first handle back");
//! assert_eq!(calls, 1, "and the loader ran once");
//!
//! // A file that is not there is an error, not a panic.
//! let mut missing = |_: &std::path::Path| {
//!     Err(TextureError::Unreadable("no such file".to_string()))
//! };
//! assert!(cache
//!     .load(std::path::Path::new("gone.png"), &mut missing)
//!     .is_err());
//! ```

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::paint::TextureId;

/// The image atlas texture size in pixels, square.
pub const ATLAS_SIZE: u32 = 2048;

/// The largest image the shared atlas will take, in pixels along either axis.
///
/// An image at or below this in both directions is packed into the atlas with
/// the icons; anything larger gets a texture of its own. The limit is what
/// keeps one photograph from evicting every icon on the screen, and it is a
/// constant rather than a setting because there is no caller that would want to
/// move it and a per-call override would be a second rule to keep honest.
pub const ATLAS_MAX_IMAGE: u32 = 512;

/// The transparent pixel, in a one-by-one [`Pixels`].
///
/// A widget with no texture draws this rather than nothing, so a missing image
/// is a hole the shape of an image rather than a hole the shape of nothing.
const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

/// The bit a [`TextureId`] sets to say the image is a texture of its own rather
/// than a rect inside the shared atlas.
const STANDALONE: u32 = 0x8000_0000;

/// Returns the identifier with the [`STANDALONE`] bit cleared.
///
/// The cache's own maps are keyed by this, not by the identifier a widget
/// holds, so a caller cannot reach into the bookkeeping by guessing a number.
fn low(id: u32) -> u32 {
    id & !STANDALONE
}

/// Returns whether `id` names an image with a texture of its own, rather than a
/// rect inside the shared atlas.
///
/// This is what the renderer asks before it binds a texture: the two cases
/// differ in which GL texture a quad samples and whether it has UVs to clamp, so
/// the bit is the whole of the decision. It is a free function rather than a
/// method on the cache because the renderer sees a bare [`TextureId`] inside a
/// recorded draw command, with no cache in hand.
#[must_use]
pub fn is_standalone(id: TextureId) -> bool {
    id.get() & STANDALONE != 0
}

/// The number a [`TextureId`] is given in the atlas; the first is zero.
const FIRST_ATLAS_ID: u32 = 1;

/// The padding between two images packed into the atlas, in pixels.
///
/// Without it, a linearly filtered texel on an image's edge would interpolate
/// the neighbouring image's pixels into it. It is the same reason
/// [`ATLAS_PAD`](crate::font::ATLAS_PAD) exists, for the same reason.
const ATLAS_PAD: u32 = 2;

/// The shortest span worth keeping after a carve, in pixels.
const MIN_SHELF_HEIGHT: u32 = 8;

/// A decoded image: width, height, and RGBA bytes with the alpha already
/// multiplied into the colour channels.
///
/// Premultiplied, because that is what the whole pipeline blends with
/// (`glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`) and what every other colour in
/// this repository carries; an image that arrived straight-alpha would need
/// converting on every sample to composite correctly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Pixels {
    /// Creates an image from `data`, which is `width` × `height` RGBA bytes.
    ///
    /// The data is taken as it is: it is **not** premultiplied here, because a
    /// caller building a fixture wants to write the bytes it means rather than
    /// the bytes the conversion would produce. [`Pixels::premultiply`] is the
    /// function the loader applies, and it is public so a test can check what it
    /// did.
    #[must_use]
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Pixels {
            width,
            height,
            data,
        }
    }

    /// Creates an image that is entirely transparent.
    #[must_use]
    pub fn transparent(width: u32, height: u32) -> Self {
        let count = usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(usize::try_from(height).ok()?))
            .unwrap_or(0);
        Pixels::new(
            width,
            height,
            TRANSPARENT
                .iter()
                .copied()
                .cycle()
                .take(count * 4)
                .collect(),
        )
    }

    /// Returns the image's width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the image's height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the RGBA bytes, four per pixel, row by row.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the image's size in pixels.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Multiplies every pixel's colour channels by its alpha, in place.
    ///
    /// Premultiplication is `c' = c * a / 255`, and it is done in `u16` rather
    /// than in `u8` so the intermediate product is not truncated before the
    /// divide: `(200 * 200) / 255` is 156.86, and a `u8` multiply would have
    /// said 156 for every value. A pixel that is already premultiplied comes
    /// back slightly different, which is why it is applied once, at load, and
    /// not on every sample.
    pub fn premultiply(&mut self) {
        for pixel in self.data.chunks_exact_mut(4) {
            let alpha = u16::from(pixel[3]);
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * alpha) / 255) as u8;
            }
        }
    }

    /// Returns whether every pixel's colour channels are within its alpha, which
    /// is what "premultiplied" means and what a source image is not.
    #[must_use]
    pub fn is_premultiplied(&self) -> bool {
        self.data.chunks_exact(4).all(|pixel| {
            pixel[3] == 255
                || (pixel[0] <= pixel[3] && pixel[1] <= pixel[3] && pixel[2] <= pixel[3])
        })
    }
}

/// Why an image could not be made available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextureError {
    /// The file could not be opened, or is not an image this build can decode.
    ///
    /// The message is the decoder's own; SDL_image names the format it could
    /// not read, and a caller that wants to tell a missing file from an
    /// unsupported one is given a library that reports both the same way.
    Unreadable(String),
    /// The image decoded but its size cannot be addressed: a zero in either
    /// axis, or a pixel count that does not fit in a `usize`.
    TooLarge {
        /// The width the decoder reported.
        width: u32,
        /// The height the decoder reported.
        height: u32,
    },
}

impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextureError::Unreadable(message) => write!(f, "could not read image: {message}"),
            TextureError::TooLarge { width, height } => {
                write!(f, "image of {width}x{height} has an addressable size")
            }
        }
    }
}

impl std::error::Error for TextureError {}

/// A handle to an image the cache is holding.
///
/// It is a [`TextureId`] and nothing more, so a widget that has one can record
/// a draw command without reaching the cache; what the *number* means is
/// [`TextureId`]'s business.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextureHandle(TextureId);

impl TextureHandle {
    /// Wraps a raw [`TextureId`].
    #[must_use]
    pub fn new(id: TextureId) -> Self {
        TextureHandle(id)
    }

    /// Returns the raw identifier the renderer looks the image up by.
    #[must_use]
    pub fn id(self) -> TextureId {
        self.0
    }
}

impl From<TextureHandle> for TextureId {
    fn from(handle: TextureHandle) -> Self {
        handle.0
    }
}

/// Where an image sits inside the shared atlas, in normalised coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// Left edge, `0.0..=1.0` across the atlas.
    pub u0: f32,
    /// Top edge, `0.0..=1.0` down the atlas.
    pub v0: f32,
    /// Right edge, `0.0..=1.0` across the atlas.
    pub u1: f32,
    /// Bottom edge, `0.0..=1.0` down the atlas.
    pub v1: f32,
    /// The image's width in pixels.
    pub width: u32,
    /// The image's height in pixels.
    pub height: u32,
    /// The shelf this image was packed into, for eviction.
    shelf_y: u32,
}

/// One shelf of the image atlas: a row of images with a shared height.
#[derive(Clone, Debug, PartialEq)]
struct Shelf {
    /// The shelf's top edge in the atlas.
    y: u32,
    /// The shelf's height: the tallest image it holds.
    height: u32,
    /// The first free column, past the last image and its padding.
    x: u32,
    /// The identifiers packed here, for eviction.
    images: Vec<u32>,
}

/// A vertical span left behind by an evicted shelf, in pixels: `y` and `height`.
type FreeSpan = (u32, u32);

/// One image too large for the atlas, held in pixels until the renderer makes
/// it a texture of its own.
#[derive(Clone, Debug, PartialEq)]
struct Standalone {
    pixels: Pixels,
    /// When the image was last asked for, as a monotonically rising counter, so
    /// the least recently used is the smallest. A wall clock would be untestable
    /// and `AGENTS.md` forbids one.
    used: u64,
}

/// The shared atlas of small images, held in CPU pixels.
///
/// Shelf-packed like the glyph atlas, and for the same reason: shelves are
/// cheap to allocate from, and evicting one frees a contiguous band rather than
/// a scattered handful of rectangles. What is *not* the same is the eviction
/// policy's effect — see [`ImageCache::evict_least_recently_used`].
#[derive(Clone, Debug, PartialEq)]
struct ImageCache {
    size: u32,
    pixels: Vec<u8>,
    /// Shelves in recency order: the front is the least recently used.
    shelves: Vec<Shelf>,
    /// Vertical spans freed by eviction.
    free: Vec<FreeSpan>,
    /// The next unused pixel row, below every shelf ever allocated.
    next_y: u32,
    /// Where each atlas-resident image sits.
    placements: HashMap<u32, Placement>,
    /// Whether [`ImageCache::pixels`] has changed since the last upload.
    dirty: bool,
}

impl ImageCache {
    /// Creates an empty `size` × `size` atlas.
    fn new(size: u32) -> Self {
        let area = usize::try_from(size)
            .ok()
            .and_then(|size| size.checked_mul(size))
            .unwrap_or(0);
        ImageCache {
            size,
            pixels: vec![0; area * 4],
            shelves: Vec::new(),
            free: Vec::new(),
            next_y: 0,
            placements: HashMap::new(),
            dirty: false,
        }
    }

    /// Returns the placement without touching recency, for a lookup that must
    /// not change the eviction order.
    fn placement_peek(&self, id: u32) -> Option<Placement> {
        self.placements.get(&id).copied()
    }

    /// Moves the shelf at `y` to the back of the recency order.
    fn touch_shelf(&mut self, y: u32) {
        if let Some(index) = self.shelves.iter().position(|shelf| shelf.y == y) {
            let shelf = self.shelves.remove(index);
            self.shelves.push(shelf);
        }
    }

    /// Returns the position of a `width` × `height` image if a shelf can take
    /// it.
    ///
    /// Tries a shelf with room to spare, then a span freed by eviction, then
    /// unused space at the bottom. Images are separated by [`ATLAS_PAD`] pixels
    /// so linear filtering never interpolates one into its neighbour.
    fn fit(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        if width == 0 || height == 0 || width > self.size || height > self.size {
            return None;
        }
        for shelf in self.shelves.iter_mut() {
            if shelf.height >= height && shelf.x + width + ATLAS_PAD <= self.size {
                let x = shelf.x;
                let y = shelf.y;
                shelf.x += width + ATLAS_PAD;
                return Some((x, y));
            }
        }
        if let Some(index) = self.free.iter().position(|&(_, span)| span >= height) {
            let (y, span) = self.free.swap_remove(index);
            if span - height >= MIN_SHELF_HEIGHT {
                self.free.push((y + height, span - height));
            }
            self.shelves.push(Shelf {
                y,
                height,
                x: width + ATLAS_PAD,
                images: Vec::new(),
            });
            return Some((0, y));
        }
        if self.next_y + height > self.size {
            return None;
        }
        let y = self.next_y;
        self.next_y += height;
        self.shelves.push(Shelf {
            y,
            height,
            x: width + ATLAS_PAD,
            images: Vec::new(),
        });
        Some((0, y))
    }

    /// Evicts the least recently used shelf: the front of `shelves`, which
    /// [`ImageCache::touch_shelf`] keeps ordered oldest-first. Its images are
    /// dropped, its pixels zeroed and its vertical span returned to the free
    /// list.
    ///
    /// This is the one place a *live* image can be pulled out from under a
    /// widget, and it is a real limitation rather than a rounding error: the
    /// shelf is the unit, so a screen of 256-pixel icons loses every icon on the
    /// oldest shelf when one more is asked for. The alternatives are to give an
    /// image that does not fit a shelf its own rectangle — which is a second
    /// allocator — or to grow the atlas, which is a fixed-size design the
    /// architecture document already chose. A caller that cannot lose an image
    /// holds it itself; see [`TextureCache::pin`].
    fn evict_least_recently_used(&mut self) -> Option<()> {
        if self.shelves.is_empty() {
            return None;
        }
        let shelf = self.shelves.remove(0);
        for id in &shelf.images {
            self.placements.remove(id);
        }
        self.clear_rect(0, shelf.y, self.size, shelf.height);
        self.free.push((shelf.y, shelf.height));
        self.dirty = true;
        Some(())
    }

    /// Copies `image` into the atlas at `(x, y)`.
    fn blit(&mut self, x: u32, y: u32, image: &Pixels) {
        for row in 0..image.height {
            let from =
                usize::try_from(row).unwrap_or(0) * 4 * usize::try_from(image.width).unwrap_or(0);
            let to = usize::try_from(y + row)
                .ok()
                .and_then(|y| y.checked_mul(usize::try_from(self.size).ok()?))
                .and_then(|row| row.checked_add(usize::try_from(x).ok()?))
                .and_then(|row| row.checked_mul(4))
                .unwrap_or(0);
            let width = 4 * usize::try_from(image.width).unwrap_or(0);
            if from + width > image.data.len() || to + width > self.pixels.len() {
                continue;
            }
            self.pixels[to..to + width].copy_from_slice(&image.data[from..from + width]);
        }
        self.dirty = true;
    }

    /// Zeroes a rectangle of the atlas.
    fn clear_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        let row_bytes = 4 * usize::try_from(width).unwrap_or(0);
        for row in 0..height {
            let start = usize::try_from(y + row)
                .ok()
                .and_then(|y| y.checked_mul(usize::try_from(self.size).ok()?))
                .and_then(|row| row.checked_add(usize::try_from(x).ok()?))
                .and_then(|row| row.checked_mul(4))
                .unwrap_or(0);
            if start + row_bytes > self.pixels.len() {
                continue;
            }
            self.pixels[start..start + row_bytes].fill(0);
        }
    }
}

/// The decoded images this process is holding, and where each of them lives.
///
/// A cache rather than a bare load, because requirement 2 asks for it and
/// because a list that draws the same icon a hundred times should decode it
/// once: the key is the path, and the value is where the pixels went.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureCache {
    atlas: ImageCache,
    /// Every path asked for, and the handle it was given.
    loaded: HashMap<PathBuf, TextureHandle>,
    /// Images too large for the atlas, by handle.
    standalone: HashMap<u32, Standalone>,
    /// The next handle to hand out.
    next_id: u32,
    /// The tick [`TextureCache::load`] stamps on an image it is asked for again.
    clock: u64,
    /// The edges of the image, so [`TextureCache::load`] does not depend on the
    /// decoder having produced one.
    sizes: HashMap<u32, (u32, u32)>,
    /// Handles the caller has asked to keep, which eviction may not take.
    pinned: Vec<u32>,
}

impl Default for TextureCache {
    fn default() -> Self {
        TextureCache::new()
    }
}

impl TextureCache {
    /// Creates a cache with an empty [`ATLAS_SIZE`] atlas.
    #[must_use]
    pub fn new() -> Self {
        TextureCache {
            atlas: ImageCache::new(ATLAS_SIZE),
            loaded: HashMap::new(),
            standalone: HashMap::new(),
            next_id: FIRST_ATLAS_ID,
            clock: 0,
            sizes: HashMap::new(),
            pinned: Vec::new(),
        }
    }

    /// Loads the image at `path` from disk, decoding it with SDL_image.
    ///
    /// This is the one caller that supplies the real decoder; everything else
    /// goes through [`TextureCache::load`], which is what makes the cache
    /// testable without a filesystem.
    ///
    /// A path that is not valid UTF-8 is an error rather than a decode attempt:
    /// the binding's loader takes a `CString` and unwraps, and a path that
    /// cannot be one is a caller error worth reporting rather than a panic to
    /// provoke.
    ///
    /// # Errors
    ///
    /// Returns [`TextureError::Unreadable`] when the file cannot be opened or
    /// decoded, and [`TextureError::TooLarge`] when it decodes to a size with
    /// no addressable pixel count.
    pub fn load_from_file(&mut self, path: &Path) -> Result<TextureHandle, TextureError> {
        self.load(path, &mut |path: &Path| decode(path))
    }

    /// Returns the handle for the image at `path`, decoding it with `loader` the
    /// first time it is asked for and not at all afterwards.
    ///
    /// The closure is called at most once per path, and not at all for a path
    /// already in the cache. That is the whole of requirement 2's "don't reload
    /// if already loaded", and it is what the module's example asserts.
    ///
    /// # Errors
    ///
    /// Returns whatever `loader` returns, unchanged, and nothing else: an image
    /// that failed to decode is not remembered, so a caller that fixes the file
    /// and asks again gets a second attempt rather than a cached failure.
    pub fn load<F>(&mut self, path: &Path, loader: &mut F) -> Result<TextureHandle, TextureError>
    where
        F: FnMut(&Path) -> Result<Pixels, TextureError>,
    {
        if let Some(&handle) = self.loaded.get(path) {
            self.touch(handle);
            return Ok(handle);
        }
        let pixels = loader(path)?;
        let handle = self.insert(pixels)?;
        self.loaded.insert(path.to_path_buf(), handle);
        Ok(handle)
    }

    /// Returns whether the cache already holds the image at `path`, without
    /// loading it.
    #[must_use]
    pub fn is_loaded(&self, path: &Path) -> bool {
        self.loaded.contains_key(path)
    }

    /// Returns the handle for `path` if it is already loaded, and `None`
    /// otherwise.
    ///
    /// It is the non-fallible half of [`TextureCache::load`], for a caller that
    /// draws a placeholder when the image is not there yet rather than
    /// synchronously blocking on the decode.
    #[must_use]
    pub fn get(&self, path: &Path) -> Option<TextureHandle> {
        self.loaded.get(path).copied()
    }

    /// Returns the size in pixels of the image `handle` names, or `None` for a
    /// handle this cache never issued.
    ///
    /// A widget asks for this to work out a fit, and a widget holding a handle
    /// it was given must be able to ask without a borrow of the cache that
    /// outlives the frame.
    #[must_use]
    pub fn size_of(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.sizes.get(&low(handle.id().get())).copied()
    }

    /// Marks `handle` as one eviction may not take.
    ///
    /// A widget that must be showing an image — a photograph, a logo — pins it,
    /// and the cache stops evicting shelves that would drop a pinned image.
    ///
    /// The cost is the shelf being the unit of eviction, which
    /// the eviction policy spells out: a pinned image protects its whole
    /// shelf. When the atlas is full and the only shelf left
    /// to give up is a protected one, the image being loaded does **not** fail —
    /// it is given a texture of its own instead, which is a slower draw and a
    /// little more memory, and is not something the caller is told about. That
    /// is the right trade for a pinned image: the alternative is a hole.
    pub fn pin(&mut self, handle: TextureHandle) {
        let id = low(handle.id().get());
        if !self.pinned.contains(&id) {
            self.pinned.push(id);
        }
    }

    /// Returns the atlas's size in pixels.
    #[must_use]
    pub fn atlas_size(&self) -> u32 {
        self.atlas.size
    }

    /// Returns the atlas pixels if they changed since the last call, and marks
    /// them clean. `None` means the caller's copy is still current.
    ///
    /// The renderer calls this after it has read what it needs for the frame and
    /// before it draws: taking it earlier would upload a texture without the
    /// images that frame is about to sample.
    pub fn take_dirty_atlas_pixels(&mut self) -> Option<&[u8]> {
        if !self.atlas.dirty {
            return None;
        }
        self.atlas.dirty = false;
        Some(&self.atlas.pixels)
    }

    /// Returns where `handle` sits in the atlas, or `None` if it is not
    /// atlas-resident or has been evicted.
    ///
    /// A lookup here does **not** mark the image as used: the renderer asks on
    /// every frame, and touching recency every frame would pin everything. The
    /// recency a pixel gets is stamped by [`TextureCache::load`], which runs
    /// when an image is asked for rather than when it is drawn.
    #[must_use]
    pub fn placement(&self, handle: TextureHandle) -> Option<Placement> {
        self.atlas.placement_peek(low(handle.id().get()))
    }

    /// Returns the pixels of an image that has a texture of its own, or `None`
    /// if it is atlas-resident or unknown.
    ///
    /// The renderer reads this to create the GL texture, and keeps it thereafter,
    /// so it is read once per image rather than once per frame.
    #[must_use]
    pub fn standalone_pixels(&self, handle: TextureHandle) -> Option<&Pixels> {
        self.standalone
            .get(&low(handle.id().get()))
            .map(|entry| &entry.pixels)
    }

    /// Returns the handles that have their own texture, in no particular order.
    #[must_use]
    pub fn standalone_handles(&self) -> Vec<TextureHandle> {
        self.standalone
            .keys()
            .map(|id| TextureHandle::new(TextureId::new(STANDALONE | id)))
            .collect()
    }

    /// Returns how many images the cache is holding.
    #[must_use]
    pub fn len(&self) -> usize {
        self.loaded.len()
    }

    /// Returns whether the cache is holding nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.loaded.is_empty()
    }

    /// Places `pixels` and returns the handle, packing it into the atlas when it
    /// is small enough and giving it a texture of its own when it is not.
    ///
    /// # Errors
    ///
    /// Returns [`TextureError::TooLarge`] when the image has no addressable
    /// size, and when it is atlas-sized and the atlas is full of images that
    /// are pinned or that the request made eviction unable to free a shelf.
    fn insert(&mut self, pixels: Pixels) -> Result<TextureHandle, TextureError> {
        let (width, height) = pixels.size();
        let addressable = usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(usize::try_from(height).ok()?));
        if width == 0 || height == 0 || addressable.is_none() {
            return Err(TextureError::TooLarge { width, height });
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(FIRST_ATLAS_ID);
        self.sizes.insert(id, (width, height));

        if width <= ATLAS_MAX_IMAGE && height <= ATLAS_MAX_IMAGE {
            if let Some((x, y)) = self.atlas_at(width, height) {
                self.atlas.blit(x, y, &pixels);
                let size = self.atlas.size;
                let placement = Placement {
                    u0: x as f32 / size as f32,
                    v0: y as f32 / size as f32,
                    u1: (x + width) as f32 / size as f32,
                    v1: (y + height) as f32 / size as f32,
                    width,
                    height,
                    shelf_y: y,
                };
                if let Some(shelf) = self.atlas.shelves.iter_mut().find(|it| it.y == y) {
                    shelf.images.push(id);
                }
                self.atlas.placements.insert(id, placement);
                // No [`STANDALONE`] bit: this image is a window into the shared
                // texture, and the renderer binds the atlas for it.
                return Ok(TextureHandle::new(TextureId::new(id)));
            }
        }
        self.standalone.insert(
            id,
            Standalone {
                pixels,
                used: self.clock,
            },
        );
        // With the bit set, because this image is a GL texture in its own right
        // and the renderer has to know that before it binds anything.
        Ok(TextureHandle::new(TextureId::new(STANDALONE | id)))
    }

    /// Finds room in the atlas for a `width` × `height` image, evicting until
    /// there is some and giving up rather than evicting a pinned image.
    ///
    /// The fit-then-evict loop lives here rather than on [`ImageCache`] because
    /// the pin check has to happen **between** the two, and eviction is
    /// destructive: an image noticed to be pinned after its shelf is gone is an
    /// image that has already been dropped.
    fn atlas_at(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        loop {
            if let Some(found) = self.atlas.fit(width, height) {
                return Some(found);
            }
            // The check is *before* the eviction, not after: eviction is
            // destructive, and a pinned image noticed afterwards is an image
            // that has already been dropped.
            let shelf_y = self.atlas.shelves.first()?.y;
            if self.shelf_holds_pinned(shelf_y) {
                return None;
            }
            self.atlas.evict_least_recently_used()?;
        }
    }

    /// Returns whether the shelf at `y` holds an image eviction may not take.
    fn shelf_holds_pinned(&self, y: u32) -> bool {
        self.atlas
            .shelves
            .iter()
            .find(|shelf| shelf.y == y)
            .is_some_and(|shelf| shelf.images.iter().any(|id| self.pinned.contains(id)))
    }

    /// Marks the image a handle names as the most recently used, which is what
    /// decides which shelf the next eviction takes.
    fn touch(&mut self, handle: TextureHandle) {
        self.clock += 1;
        let id = low(handle.id().get());
        if let Some(entry) = self.standalone.get_mut(&id) {
            entry.used = self.clock;
            return;
        }
        if let Some(shelf_y) = self.atlas.placement_peek(id).map(|it| it.shelf_y) {
            self.atlas.touch_shelf(shelf_y);
        }
    }
}

/// Decodes an image file into premultiplied RGBA pixels.
///
/// Everything below this line is the decoder, and it is the only part of the
/// module that knows SDL exists. It is here, rather than in
/// [`TextureCache::load_from_file`], so the seam has something on the far side
/// of it: a caller replacing the decoder replaces this function and nothing
/// else.
///
/// # Errors
///
/// Returns [`TextureError::Unreadable`] for anything SDL_image will not read,
/// and [`TextureError::TooLarge`] for a decoded surface with an unusable size.
fn decode(path: &Path) -> Result<Pixels, TextureError> {
    use sdl3::pixels::PixelFormat;

    let Some(text) = path.to_str() else {
        return Err(TextureError::Unreadable(
            "the path is not valid UTF-8".to_string(),
        ));
    };
    let loaded: Result<sdl3::surface::Surface<'static>, sdl3::Error> =
        sdl3::image::LoadSurface::from_file(text);
    let Ok(mut surface) = loaded else {
        return Err(TextureError::Unreadable(format!(
            "no image decoder accepted {text}"
        )));
    };
    // `Surface::with_lock` panics rather than returning an error when SDL's
    // surface lock fails, and the one case that fails is an RLE-encoded
    // surface whose RLE data will not lock. A decoded BMP can be one, so it is
    // turned off before anything reads the pixels.
    surface.disable_RLE();
    let rgba = PixelFormat::RGBA32;
    let converted = surface
        .convert(&rgba)
        .map_err(|error| TextureError::Unreadable(error.to_string()))?;
    let (width, height) = converted.size();
    let pitch = usize::try_from(converted.pitch()).unwrap_or(0);
    let row = 4 * usize::try_from(width).unwrap_or(0);
    if width == 0 || height == 0 || row == 0 || pitch < row {
        return Err(TextureError::TooLarge { width, height });
    }
    let mut data = Vec::with_capacity(row * usize::try_from(height).unwrap_or(0));
    converted.with_lock(|pixels| {
        for line in 0..usize::try_from(height).unwrap_or(0) {
            let start = line * pitch;
            let end = start + row;
            match pixels.get(start..end) {
                Some(slice) => data.extend_from_slice(slice),
                // A surface whose pitch does not account for its own height has
                // told us something untrue; taking what is there beats
                // indexing past the end of it.
                None => break,
            }
        }
    });
    let expected = row * usize::try_from(height).unwrap_or(0);
    if data.len() != expected {
        return Err(TextureError::Unreadable(format!(
            "the decoder returned {} of {expected} bytes for {text}",
            data.len()
        )));
    }
    let mut pixels = Pixels::new(width, height, data);
    pixels.premultiply();
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// A solid image `width` × `height` of `color`, premultiplied.
    fn solid(width: u32, height: u32, color: [u8; 4]) -> Pixels {
        let count = usize::try_from(width * height).unwrap_or(0);
        Pixels::new(width, height, color.repeat(count))
    }

    /// A loader that counts how often it ran.
    ///
    /// The count is behind a shared cell rather than read off the struct
    /// directly, because a test wants to assert on it *while* the loader
    /// closure is still alive — and a closure borrowing `&mut self` holds the
    /// borrow for as long as it exists.
    struct Counting {
        calls: Rc<Cell<usize>>,
        image: Pixels,
        fail: bool,
    }

    impl Counting {
        fn new(width: u32, height: u32) -> Self {
            Counting {
                calls: Rc::new(Cell::new(0)),
                image: solid(width, height, [200, 100, 50, 255]),
                fail: false,
            }
        }

        fn loader(&mut self) -> impl FnMut(&Path) -> Result<Pixels, TextureError> + '_ {
            let calls = Rc::clone(&self.calls);
            let image = self.image.clone();
            let fail = self.fail;
            move |_: &Path| {
                calls.set(calls.get() + 1);
                if fail {
                    Err(TextureError::Unreadable("no such file".to_string()))
                } else {
                    Ok(image.clone())
                }
            }
        }
    }

    /// Loads one image through `counter` and returns the handle and the call
    /// count, so a test can assert on both without holding the closure's borrow.
    fn load_once(
        cache: &mut TextureCache,
        counter: &mut Counting,
        name: &str,
    ) -> Result<(TextureHandle, Rc<Cell<usize>>), TextureError> {
        let calls = Rc::clone(&counter.calls);
        let mut loader = counter.loader();
        let handle = cache.load(&path(name), &mut loader)?;
        Ok((handle, calls))
    }

    fn path(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn pixels_report_their_own_size() {
        let pixels = solid(3, 5, [1, 2, 3, 255]);
        assert_eq!(pixels.size(), (3, 5));
        assert_eq!(pixels.width(), 3);
        assert_eq!(pixels.height(), 5);
        assert_eq!(pixels.data().len(), 3 * 5 * 4);
    }

    #[test]
    fn a_transparent_image_is_fully_transparent() {
        let pixels = Pixels::transparent(2, 2);
        assert_eq!(pixels.size(), (2, 2));
        assert_eq!(pixels.data(), &[0; 16]);
        assert!(pixels.is_premultiplied());
    }

    #[test]
    fn premultiply_scales_the_colour_by_the_alpha() {
        // 200 * 128 / 255 is 100.39, and the u16 intermediate is what makes the
        // answer 100 rather than the 100 a u8 multiply gives for a different
        // reason. The value a test would be wrong about is the rounding, so it
        // is written out rather than computed from the function under test.
        let mut pixels = Pixels::new(1, 1, vec![200, 255, 0, 128]);
        pixels.premultiply();
        assert_eq!(pixels.data(), &[100, 128, 0, 128]);
    }

    #[test]
    fn premultiply_leaves_an_opaque_pixel_alone() {
        let mut pixels = Pixels::new(1, 1, vec![200, 100, 50, 255]);
        pixels.premultiply();
        assert_eq!(pixels.data(), &[200, 100, 50, 255]);
    }

    #[test]
    fn premultiply_leaves_a_fully_transparent_pixel_at_zero() {
        let mut pixels = Pixels::new(1, 1, vec![200, 100, 50, 0]);
        pixels.premultiply();
        assert_eq!(pixels.data(), &[0, 0, 0, 0]);
    }

    #[test]
    fn is_premultiplied_tells_a_strong_colour_from_a_weak_alpha() {
        let weak = Pixels::new(1, 1, vec![200, 100, 50, 128]);
        assert!(
            !weak.is_premultiplied(),
            "200 red under alpha 128 is not premultiplied"
        );
        let strong = Pixels::new(1, 1, vec![200, 100, 50, 255]);
        assert!(strong.is_premultiplied());
    }

    #[test]
    fn a_second_request_for_a_path_does_not_decode_again() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(8, 8);

        let (first, calls) = load_once(&mut cache, &mut counter, "icon.png").expect("loaded");
        let mut loader = counter.loader();
        let second = cache.load(&path("icon.png"), &mut loader).expect("cached");

        assert_eq!(
            first, second,
            "the cache answers with the handle it gave before"
        );
        assert_eq!(calls.get(), 1, "and the decoder ran exactly once");
    }

    #[test]
    fn two_paths_get_two_handles() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(8, 8);

        let (a, calls_a) = load_once(&mut cache, &mut counter, "a.png").expect("loaded");
        let (b, _) = load_once(&mut cache, &mut counter, "b.png").expect("loaded");

        assert_ne!(a, b, "two files are two images");
        assert_eq!(calls_a.get(), 2);
        assert_eq!(cache.len(), 2);
        assert!(!cache.is_empty());
    }

    #[test]
    fn a_failed_load_is_not_remembered() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(8, 8);
        counter.fail = true;

        assert!(load_once(&mut cache, &mut counter, "gone.png").is_err());
        assert!(
            !cache.is_loaded(&path("gone.png")),
            "a failure is not a cache entry"
        );
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn the_error_a_loader_returns_is_passed_on_unchanged() {
        let mut cache = TextureCache::new();
        let wanted = TextureError::Unreadable("decoder said no".to_string());
        let mut loader = |_: &Path| Err(wanted.clone());

        let got = cache
            .load(&path("x.png"), &mut loader)
            .expect_err("the loader failed");
        assert_eq!(got, wanted);
        assert_eq!(got.to_string(), "could not read image: decoder said no");
    }

    #[test]
    fn is_loaded_and_get_answer_without_decoding() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(8, 8);
        let calls = Rc::clone(&counter.calls);

        assert!(!cache.is_loaded(&path("icon.png")));
        assert_eq!(cache.get(&path("icon.png")), None);
        assert_eq!(calls.get(), 0, "asking is not loading");

        let handle = load_once(&mut cache, &mut counter, "icon.png")
            .expect("loaded")
            .0;
        assert!(cache.is_loaded(&path("icon.png")));
        assert_eq!(cache.get(&path("icon.png")), Some(handle));
    }

    #[test]
    fn a_small_image_is_packed_into_the_atlas() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(ATLAS_MAX_IMAGE, ATLAS_MAX_IMAGE);
        let handle = load_once(&mut cache, &mut counter, "icon.png")
            .expect("loaded")
            .0;

        let placement = cache.placement(handle).expect("atlas resident");
        assert_eq!(placement.width, ATLAS_MAX_IMAGE);
        assert_eq!(placement.height, ATLAS_MAX_IMAGE);
        assert_eq!(
            placement.u0, 0.0,
            "the first image is at the atlas's corner"
        );
        assert_eq!(placement.v0, 0.0);
        assert!(
            cache.standalone_pixels(handle).is_none(),
            "an atlas image has no texture of its own"
        );
    }

    #[test]
    fn an_atlas_placement_is_inside_the_atlas() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(64, 32);
        let handle = load_once(&mut cache, &mut counter, "icon.png")
            .expect("loaded")
            .0;
        let placement = cache.placement(handle).expect("atlas resident");
        let size = cache.atlas_size() as f32;

        assert!(placement.u0 >= 0.0 && placement.u0 <= placement.u1 && placement.u1 <= 1.0);
        assert!(placement.v0 >= 0.0 && placement.v0 <= placement.v1 && placement.v1 <= 1.0);
        assert_eq!((placement.u1 - placement.u0) * size, 64.0);
        assert_eq!((placement.v1 - placement.v0) * size, 32.0);
    }

    #[test]
    fn a_large_image_gets_a_texture_of_its_own() {
        let mut cache = TextureCache::new();
        let big = ATLAS_MAX_IMAGE + 1;
        let mut counter = Counting::new(big, 4);
        let handle = load_once(&mut cache, &mut counter, "photo.png")
            .expect("loaded")
            .0;

        assert!(
            cache.placement(handle).is_none(),
            "too big for the atlas, so not in it"
        );
        let pixels = cache.standalone_pixels(handle).expect("has its own pixels");
        assert_eq!(pixels.size(), (big, 4));
        assert_eq!(cache.standalone_handles(), vec![handle]);
    }

    #[test]
    fn an_image_too_tall_for_the_atlas_also_gets_its_own_texture() {
        // The limit is on *both* axes, and an image that is narrow but taller
        // than the limit is the case a test that only checks the width misses.
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(4, ATLAS_MAX_IMAGE + 1);
        let handle = load_once(&mut cache, &mut counter, "tall.png")
            .expect("loaded")
            .0;

        assert!(cache.placement(handle).is_none());
        assert!(cache.standalone_pixels(handle).is_some());
    }

    #[test]
    fn the_size_of_a_handle_is_known_without_the_pixels() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(40, 20);
        let handle = load_once(&mut cache, &mut counter, "icon.png")
            .expect("loaded")
            .0;

        assert_eq!(cache.size_of(handle), Some((40, 20)));
        assert_eq!(
            cache.size_of(TextureHandle::new(TextureId::new(9999))),
            None
        );
    }

    #[test]
    fn the_atlas_pixels_are_offered_once_per_change() {
        let mut cache = TextureCache::new();
        assert!(
            cache.take_dirty_atlas_pixels().is_none(),
            "an empty atlas has nothing to upload"
        );

        let mut counter = Counting::new(16, 16);
        load_once(&mut cache, &mut counter, "icon.png").expect("loaded");

        let first = cache
            .take_dirty_atlas_pixels()
            .expect("the image changed the atlas");
        assert_eq!(first.len(), (ATLAS_SIZE * ATLAS_SIZE * 4) as usize);
        assert!(
            cache.take_dirty_atlas_pixels().is_none(),
            "and then nothing"
        );
    }

    #[test]
    fn the_image_reaches_the_atlas_pixels_the_upload_would_send() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(4, 4);
        counter.image = solid(4, 4, [10, 20, 30, 255]);
        let handle = load_once(&mut cache, &mut counter, "icon.png")
            .expect("loaded")
            .0;
        let placement = cache.placement(handle).expect("atlas resident");
        let size = cache.atlas_size() as usize;
        let pixels = cache.take_dirty_atlas_pixels().expect("dirty");
        let x = (placement.u0 * size as f32) as usize;
        let y = (placement.v0 * size as f32) as usize;
        let at = (y * size + x) * 4;
        assert_eq!(&pixels[at..at + 4], &[10, 20, 30, 255]);
    }

    #[test]
    fn filling_the_atlas_evicts_and_the_evicted_image_stops_being_there() {
        let mut cache = TextureCache::new();
        // Shelve-size images, so a full shelf is a handful of them and the
        // eviction path is reached without allocating a 2048² texture per test.
        let side = 128;
        let per_shelf = ATLAS_SIZE / (side + ATLAS_PAD);
        let shelves = ATLAS_SIZE / side;
        let total = (per_shelf * shelves) + per_shelf;
        let mut handles = Vec::new();
        let mut counter = Counting::new(side, side);
        for index in 0..total {
            let (handle, _) =
                load_once(&mut cache, &mut counter, &format!("icon{index}.png")).expect("loaded");
            handles.push(handle);
        }

        let first = handles[0];
        assert!(
            cache.placement(first).is_none(),
            "the oldest shelf was evicted to make room, so its images are gone"
        );
        let last = *handles.last().expect("there is a last image");
        assert!(
            cache.placement(last).is_some(),
            "and the newest image is resident"
        );
    }

    #[test]
    fn an_evicted_image_is_answered_from_the_cache_rather_than_decoded_again() {
        let mut cache = TextureCache::new();
        let side = 128;
        let per_shelf = ATLAS_SIZE / (side + ATLAS_PAD);
        let total = (per_shelf * (ATLAS_SIZE / side)) + per_shelf;
        let mut counter = Counting::new(side, side);
        let (first, calls) = load_once(&mut cache, &mut counter, "icon0.png").expect("loaded");
        for index in 1..total {
            load_once(&mut cache, &mut counter, &format!("icon{index}.png")).expect("loaded");
        }
        assert!(
            cache.placement(first).is_none(),
            "it was evicted to make room"
        );
        let before = calls.get();

        // Asking again finds the path in the cache and answers with the same
        // handle, so an evicted image is *not* silently re-decoded into a second
        // copy — which is the cost of caching by path, stated as a fact rather
        // than discovered on screen.
        let again = load_once(&mut cache, &mut counter, "icon0.png")
            .expect("cached")
            .0;
        assert_eq!(again, first);
        assert_eq!(
            calls.get(),
            before,
            "the decoder did not run again for a path it had already seen"
        );
    }

    #[test]
    fn a_pinned_image_is_not_evicted() {
        let mut cache = TextureCache::new();
        let side = 128;
        let per_shelf = ATLAS_SIZE / (side + ATLAS_PAD);
        let total = (per_shelf * (ATLAS_SIZE / side)) + per_shelf;
        let mut counter = Counting::new(side, side);
        let (first, _) = load_once(&mut cache, &mut counter, "icon0.png").expect("loaded");
        cache.pin(first);
        for index in 1..total {
            load_once(&mut cache, &mut counter, &format!("icon{index}.png")).expect("loaded");
        }

        assert!(
            cache.placement(first).is_some(),
            "a pinned image survives an atlas that had to keep evicting"
        );
        assert!(
            cache.standalone_handles().len() >= usize::try_from(per_shelf).unwrap_or(usize::MAX),
            "and the images after it got textures of their own rather than \
             pushing the pinned one out"
        );
    }

    #[test]
    fn a_pinned_image_is_not_pinned_twice() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(8, 8);
        let (handle, _) = load_once(&mut cache, &mut counter, "icon.png").expect("loaded");
        cache.pin(handle);
        cache.pin(handle);
        // The behaviour is "pinning is idempotent"; the count is private, so
        // what is asserted is the effect: nothing throws and the image is still
        // there.
        assert!(cache.placement(handle).is_some());
    }

    #[test]
    fn an_image_with_no_addressable_size_is_an_error_and_not_a_handle() {
        let mut cache = TextureCache::new();
        let mut loader = |_: &Path| Ok(Pixels::new(0, 8, Vec::new()));
        let got = cache
            .load(&path("empty.png"), &mut loader)
            .expect_err("no pixels at all");
        assert_eq!(
            got,
            TextureError::TooLarge {
                width: 0,
                height: 8
            }
        );
        assert_eq!(cache.len(), 0, "and it is not remembered as loaded");
    }

    #[test]
    fn an_image_too_wide_for_the_multiply_is_an_error() {
        // Only a 32-bit target can reach this branch, and only because
        // `u32::MAX * u32::MAX` is below `usize::MAX` on a 64-bit one — so a
        // test written for this host would be asserting a number it invented.
        // Derived: `(2^32-1)^2 = 2^64 - 2^33 + 1 < 2^64 - 1`, which is
        // `usize::MAX`. There is no 64-bit fixture that overflows, so the guard
        // is asserted where it can fire.
        #[cfg(target_pointer_width = "32")]
        {
            let mut cache = TextureCache::new();
            let mut loader = |_: &Path| Ok(Pixels::new(u32::MAX, u32::MAX, Vec::new()));
            let got = cache
                .load(&path("huge.png"), &mut loader)
                .expect_err("unaddressable");
            assert!(matches!(got, TextureError::TooLarge { .. }));
        }
        #[cfg(target_pointer_width = "64")]
        {
            // Two `u32`s always fit in a 64-bit `usize`, so what actually
            // rejects an image here is the zero dimension — which
            // `an_image_with_no_addressable_size_is_an_error_and_not_a_handle`
            // covers. This test exists to say the other guard is unreachable on
            // this target rather than to leave it looking tested.
            let mut cache = TextureCache::new();
            let big = u32::MAX;
            let mut loader = |_: &Path| Ok(Pixels::new(big, big, Vec::new()));
            assert!(
                cache.load(&path("huge.png"), &mut loader).is_ok(),
                "a 64-bit target can address {big}x{big} of empty pixels"
            );
        }
    }

    #[test]
    fn the_error_displays_the_size_it_rejected() {
        let error = TextureError::TooLarge {
            width: 7,
            height: 9,
        };
        assert_eq!(error.to_string(), "image of 7x9 has an addressable size");
    }

    #[test]
    fn a_handle_carries_the_raw_identifier_the_renderer_looks_up() {
        let handle = TextureHandle::new(TextureId::new(17));
        assert_eq!(handle.id(), TextureId::new(17));
        assert_eq!(TextureId::from(handle), TextureId::new(17));
    }

    #[test]
    fn the_standalone_bit_distinguishes_the_two_places_an_image_can_live() {
        // The encoding the module doc promises, checked so the promise cannot
        // be edited out from under the renderer.
        assert_eq!(STANDALONE, 0x8000_0000);
        assert_eq!(FIRST_ATLAS_ID, 1);
        assert!(
            !is_standalone(TextureId::new(5)),
            "an atlas id has the bit clear"
        );
        assert!(is_standalone(TextureId::new(STANDALONE | 5)));
    }

    #[test]
    fn an_atlas_image_carries_no_standalone_bit_and_a_big_one_does() {
        // The renderer binds the shared atlas for the first and a texture of
        // its own for the second, and the only thing telling it which is the
        // bit — so a handle that does not carry it is a black square, not a
        // compile error.
        let mut cache = TextureCache::new();
        let mut small = Counting::new(64, 64);
        let small = load_once(&mut cache, &mut small, "icon.png")
            .expect("loaded")
            .0;
        let mut big = Counting::new(ATLAS_MAX_IMAGE + 1, 4);
        let big = load_once(&mut cache, &mut big, "photo.png")
            .expect("loaded")
            .0;

        assert!(!is_standalone(small.id()), "packed into the atlas");
        assert!(is_standalone(big.id()), "too big for the atlas");
    }

    #[test]
    fn the_bit_does_not_stop_the_cache_finding_either_image() {
        // The cache's own maps are keyed by the low identifier, so a standalone
        // handle still reaches its pixels and its size. A lookup that forgot to
        // strip the bit would answer `None` for both.
        let mut cache = TextureCache::new();
        let big = ATLAS_MAX_IMAGE + 1;
        let mut counter = Counting::new(big, 7);
        let (handle, _) = load_once(&mut cache, &mut counter, "photo.png").expect("loaded");

        assert!(is_standalone(handle.id()));
        assert_eq!(cache.size_of(handle), Some((big, 7)));
        assert_eq!(
            cache.standalone_pixels(handle).map(|it| it.size()),
            Some((big, 7))
        );
        assert_eq!(cache.standalone_handles(), vec![handle]);
    }

    #[test]
    fn asking_for_a_standalone_image_twice_still_gives_one_handle() {
        let mut cache = TextureCache::new();
        let big = ATLAS_MAX_IMAGE + 1;
        let mut counter = Counting::new(big, 4);
        let (first, calls) = load_once(&mut cache, &mut counter, "photo.png").expect("loaded");
        let before = calls.get();
        let mut loader = counter.loader();
        let second = cache.load(&path("photo.png"), &mut loader).expect("cached");

        assert_eq!(first, second);
        assert_eq!(calls.get(), before, "the decoder did not run again");
    }

    #[test]
    fn the_decoder_rejects_a_path_that_is_not_text() {
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            let raw = OsStr::from_bytes(b"bad\xffname.png");
            let got = decode(Path::new(raw)).expect_err("not valid UTF-8");
            assert_eq!(
                got,
                TextureError::Unreadable("the path is not valid UTF-8".to_string())
            );
        }
    }

    #[test]
    fn the_decoder_reports_a_file_that_is_not_there_as_unreadable() {
        let got = decode(Path::new("/nonexistent/roados/nothing-here.png"))
            .expect_err("there is no such file");
        assert!(
            matches!(got, TextureError::Unreadable(_)),
            "and says so rather than panicking: {got}"
        );
    }

    #[test]
    fn a_handle_names_an_image_the_cache_can_size_and_a_stranger_cannot() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(9, 11);
        let mut loader = counter.loader();
        let handle = cache.load(&path("icon.png"), &mut loader).expect("loaded");

        assert_eq!(cache.size_of(handle), Some((9, 11)));
        assert_eq!(cache.size_of(TextureHandle::new(TextureId::new(0))), None);
    }

    #[test]
    fn a_shelf_is_reused_before_the_atlas_grows_downward() {
        let mut cache = TextureCache::new();
        let mut counter = Counting::new(64, 64);
        let first = load_once(&mut cache, &mut counter, "a.png").expect("a").0;
        counter.image = solid(16, 16, [1, 2, 3, 255]);
        let second = load_once(&mut cache, &mut counter, "b.png").expect("b").0;

        let one = cache.placement(first).expect("atlas resident");
        let two = cache.placement(second).expect("atlas resident");
        assert_eq!(one.v0, two.v0, "the small image joined the same shelf");
        assert!(
            two.u0 > one.u0,
            "to the right of the first, so its u0 is further along"
        );
    }
}
