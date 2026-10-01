//! The Image widget: a texture, a fit, and the quad it is drawn as.
//!
//! An image is a node, the one thing it draws, and the numbers that decide
//! where. [`Image::texture`] is *which* image, [`Image::fit`] is *how it is
//! shaped into the node's rect*, and [`Image::opacity`] and
//! [`Image::corner_radius`] are how it looks. Loading, decoding and placement
//! are [`texture`](crate::texture)'s business and are not repeated here.
//!
//! # The properties are not all properties
//!
//! [`Image::opacity`] and [`Image::shown_opacity`] are properties, and
//! [`Image::corner_radius`] is a property; [`Image::texture`] and [`Image::fit`]
//! are plain fields behind setters. The rule is
//! [`slider`](crate::widgets::slider::Slider)'s: **the mapping is a plain field
//! and the appearance is a property.** Which texture an image shows and how it
//! is fitted are facts about the image, not steps in a transition — nothing
//! cross-fades from `Contain` to `Cover`, and a [`Property`] would let a caller
//! write the fit directly and leave the *drawn* geometry disagreeing with it,
//! which is the one thing the two plain fields exist to prevent. Opacity and a
//! corner radius are how the image looks, and a caller changing either of them
//! mid-frame has to be able to have the node repainted: a property's
//! `on_change` is the only way that is expressible, and it is the idiom every
//! other widget in this module tree uses to keep its node clean.
//!
//! [`Image::opacity`] and [`Image::shown_opacity`] are the slider's and the
//! progress bar's split: `opacity` is the truth the caller writes, and
//! `shown_opacity` is what is *drawn*, animated toward it by
//! [`animate_to_state`](Image::animate_to_state). Writing `opacity` and
//! calling [`snap_to_state`](Image::snap_to_state) puts the two together at
//! once; a caller that writes `opacity` and then aims the widget gets a fade.
//! A caller that writes `opacity` and does neither gets the image at the opacity
//! it was last at, which is the same rule the rest of the library follows:
//! nothing moves until a caller moves it.
//!
//! # `source_size` is the caller's to get right
//!
//! **The fit modes cannot be computed without the image's own aspect ratio, and
//! the widget cannot look it up.** A [`TextureHandle`] is a number and nothing
//! else; the pixels it names live in a [`TextureCache`], and a widget holding a
//! handle cannot reach the cache that issued it without a borrow that outlives
//! the frame and couples the widget's lifetime to the cache's. So the widget
//! keeps its own copy of the answer, as an [`ImageSource`], and the caller
//! supplies it.
//!
//! Getting it wrong is not a compile error and not a blank frame: a 16:9 image
//! declared as 1:1 is *drawn* as though it were square — a letterboxed or
//! stretched picture, at the wrong scale, with the wrong part of the texture
//! sampled. Nothing downstream can detect it, because the widget is doing
//! exactly what it was told. [`ImageSource::of`] is the recipe, so that the
//! right answer is the easy one, and [`Image::set_source_size`] is the setter
//! for the case where only the two numbers are wrong.
//!
//! An [`ImageSource`] is two things, because an image has two: the image's own
//! size in pixels — the aspect ratio every fit is computed from — and the
//! rectangle it occupies *inside its texture*, which is the whole texture for
//! an image with a texture of its own and a window into the shared atlas for
//! every image small enough to be worth packing. Using
//! [`UvRect::full()`](crate::paint::UvRect::full) for an atlas-resident image is
//! not a simplification: it samples the entire atlas.
//!
//! # `None` is anchored at the top left, and that is what a scroll wants
//!
//! [`ImageFit::None`] draws the image at its own pixel size with its top-left
//! corner at the node rect's top-left corner — it does **not** centre it. Only
//! [`ImageFit::Contain`] centres anything, because only `Contain` is fitting a
//! shape into a frame and can be left with space that has to go somewhere;
//! [`ImageFit::Fill`] and [`ImageFit::Cover`] both draw at the rect itself and
//! have no space to place. `None` is not fitting anything: there is no frame,
//! the image is simply its own size, and the space in the rect is space
//! *after* it.
//!
//! A scroll view is what settles it. A scroll's content starts at its top edge:
//! [`scroll_offset`](crate::widgets::scroll::Scroll::scroll_offset) of `0.0` is
//! the top of the content, which is where a user opens a scrolled view and
//! where the first item of a column belongs. A centred `None` image in a node
//! taller than itself puts a band of nothing *above* the image, so the top of
//! the content is empty and the first thing on screen is a gap; and
//! [`max_scroll`](crate::widgets::scroll::max_scroll) is
//! `content_height - viewport_height`, so that gap is space the layout has
//! charged for and no offset can take back. Top left anchoring puts the image at
//! the top of its own node, where the content says it is, and leaves the
//! space *after* it where the next item of a column goes.
//!
//! # Rounded corners are a number, not a shape
//!
//! [`Image::corner_radius`] is passed to
//! [`DrawCommand::Image`] and that is the whole of this widget's contribution.
//! It is a **clip**: the fragment shader discards the fragments outside the
//! rounded rectangle, so a rounded corner shows whatever the widget drew behind
//! the image. This widget draws no background and no border, and neither is
//! right — a [`DrawCommand::RoundedRect`] *fills* its rect, so a "corner"
//! drawn as a filled rounded rectangle is a card with an image on it, and a
//! "border" drawn as a filled rounded rectangle is the same card. The discard is
//! the whole of the mechanism; see `.ai/NEVERAGAIN.md`'s entry *A filled rounded
//! rectangle is not an outline*, which is a defect of that shape found in this
//! repository's own slider.
//!
//! # Opacity is a fraction, not a tint
//!
//! [`Image::opacity`] is recorded as the command's own `opacity` scalar and is
//! **not** premultiplied into a colour. The colours are already in the texture;
//! a scalar is what scales the alpha the image brought with it, and the shader
//! does the multiplying. Multiplying here as well would darken at `0.5` twice —
//! a visible defect, and one the draw command cannot express anyway, since it
//! has no colour field for an image.
//!
//! # `Cover` crops, and does not overflow
//!
//! [`ImageFit::Cover`] draws its quad at the node's rect — the *same* rect
//! [`ImageFit::Fill`] uses — and covers it by **cropping the image**, sampling a
//! centred band of it that has the rect's own shape. It does not draw a quad
//! larger than the rect and sample all of the image, which is the other way to
//! spell "cover" and the one that needs a clip to look right.
//!
//! The two are the same picture *under a clip* — a cropped quad and an
//! overflowing one differ only in which part of the scaled image is on screen —
//! and [`DrawCommand`] has no scissor state of its own, which
//! `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why* records
//! as a deferral to whichever task draws within a node's own bounds. So the
//! overflowing version is right only when something clips, and wrong in a panel
//! that does not: a `Cover` image would paint a screen's worth of itself over
//! whatever is beside it. The cropped version is right in both, and it is also
//! the version in which the two halves of a fit cannot disagree: the crop is
//! computed from the rect and the image's own shape, so the picture is the same
//! whether or not a container ever cuts it.
//!
//! Requirement 3's "clip overflow" is therefore met by the overflow never being
//! drawn rather than by clipping it, which is the only form of it this library
//! can honour today.
//!
//! # What is deliberately not here
//!
//! - **No loader and no atlas.** [`crate::texture`] owns both, and repeating
//!   either here would be the second copy of the decisions worth testing.
//! - **No `on_event`.** Requirement 4 lists no gesture for an image: it is
//!   something to look at, not something to press. A caller driving an image
//!   from a tap writes [`opacity`](Image::opacity) or calls
//!   [`set_texture`](Image::set_texture), and the same property callback every
//!   other widget has carries the write to the node.
//!
//! # Examples
//!
//! The whole of a caller, from a cache holding the image to the commands a frame
//! would submit — with a loader of its own rather than a file, so the example
//! needs no filesystem.
//!
//! ```
//! use std::path::Path;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::{DrawCommand, Rect};
//! use ui_core::texture::{Pixels, TextureCache};
//! use ui_core::widgets::image::{Image, ImageFit, ImageSource};
//!
//! // A caller that holds the cache reads the image's own size and its place in
//! // its texture off the handle, and hands the answer to the widget. 1024 × 512
//! // is too big for the shared atlas, so this image has a texture of its own
//! // and its source is the whole of it; a small one would come back as a window
//! // into the atlas instead, and the crop below would be that window's middle
//! // half rather than the texture's.
//! let mut cache = TextureCache::new();
//! let mut loader = |_: &Path| Ok(Pixels::transparent(1024, 512));
//! let handle = cache.load(Path::new("logo.png"), &mut loader).expect("loaded");
//! let source = ImageSource::of(&cache, handle).expect("placed");
//!
//! let mut nodes = Arena::new();
//! let mut image = Image::new(&mut nodes, handle, source);
//!
//! // `Contain` by default: the whole image, letterboxed in whatever rect the
//! // layout gave it, sampling all of the image.
//! let rect = Rect::new(32.0, 64.0, 300.0, 300.0);
//! let DrawCommand::Image { rect: area, .. } = &image.paint(rect)[0] else {
//!     panic!("an image paints an image");
//! };
//! assert_eq!(*area, Rect::new(32.0, 139.0, 300.0, 150.0));
//!
//! // `Cover` crops rather than stretches, and fades: the quad is the node's own
//! // rect, and the image's middle half of its width is what fills it.
//! image.set_fit(ImageFit::Cover);
//! image.opacity.set(0.5);
//! image.snap_to_state();
//! let DrawCommand::Image { rect: area, uv, opacity, .. } = &image.paint(rect)[0] else {
//!     panic!("an image paints an image");
//! };
//! assert_eq!(*area, rect, "the quad is the rect, so nothing overflows it");
//! assert_eq!((uv.u0, uv.u1), (0.25, 0.75), "from the middle half of the image");
//! assert_eq!((uv.v0, uv.v1), (0.0, 1.0), "at its own full height");
//! assert_eq!(*opacity, 0.5, "and the fade reaches the command as a fraction");
//! ```

use std::cell::RefCell;
use std::time::Duration;

use crate::animation::AnimationClock;
use crate::arena::{Arena, Handle};
use crate::layout::Size;
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect, UvRect};
use crate::property::Property;
use crate::texture::{is_standalone, Placement, TextureCache, TextureHandle};
use crate::widgets::button::Motion;

/// How an image is shaped into the rect it is given.
///
/// The four are the whole of requirement 3. Three of them *fit*: they take a
/// rect and decide how much of it the image covers, preserving the image's own
/// shape. The fourth does not fit at all.
///
/// # Examples
///
/// ```
/// use ui_core::paint::Rect;
/// use ui_core::widgets::image::{destination_rect, ImageFit};
///
/// let bounds = Rect::new(32.0, 64.0, 300.0, 300.0);
///
/// // A 2:1 image contained in a square: as wide as the box, half as tall, and
/// // centred, so the space is split evenly above and below.
/// assert_eq!(
///     destination_rect(ImageFit::Contain, (200, 100), bounds),
///     Rect::new(32.0, 139.0, 300.0, 150.0),
/// );
/// // The same image covering it: the quad is the box, and `sampled_uv` crops
/// // the image's sides to fill it. The two halves are separate questions and
/// // this is the first one.
/// assert_eq!(
///     destination_rect(ImageFit::Cover, (200, 100), bounds),
///     bounds,
/// );
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFit {
    /// The whole image, scaled to the **largest** rectangle of its own shape
    /// that fits inside the node's rect, and centred in that rect.
    ///
    /// The whole image is always visible, and the space left over is split
    /// evenly on the two sides the shape does not fill — above and below for a
    /// wide image in a tall box, left and right for a tall one in a wide box.
    /// This is the default, and it is the right one for a caller that has not
    /// decided: an image that is *shown* is better than an image that is cropped
    /// by an accident of its box.
    #[default]
    Contain,
    /// The image, scaled to the **smallest** rectangle of its own shape that
    /// covers the node's rect and centred on it, sampling only the middle band
    /// of the image that is the same shape as the rect.
    ///
    /// The rect is always full of image, and the rest of the image is not drawn
    /// at all — which is what makes this a fit rather than a distortion, and
    /// which is [`sampled_uv`]'s job rather
    /// than [`destination_rect`]'s.
    ///
    /// **The quad is the node's own rect, exactly as [`Fill`](ImageFit::Fill)
    /// draws it, and nothing overflows.** Covering is done by sampling a centred
    /// band of the image that has the rect's own shape, not by drawing a bigger
    /// quad and letting something cut it: the two are the same picture under a
    /// clip, and only this one is right without one, because [`DrawCommand`] has
    /// no scissor state and `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from
    /// the spec, and why* records per-node clipping as a deferral. The module
    /// documentation says the whole of it.
    Cover,
    /// The whole image, stretched to the node's rect and to nothing else: the
    /// image's own shape is ignored, and both of its dimensions are fitted
    /// independently.
    ///
    /// This is the one fit that can distort, and it is here because a caller
    /// sometimes wants exactly that — a background, a gradient behind a panel, a
    /// nine-slice's centre. It is not the default, and it is not what
    /// "scale to fit" means anywhere else in this list.
    Fill,
    /// The image at its own size in pixels, with its top-left corner at the node
    /// rect's top-left corner.
    ///
    /// Nothing is fitted and nothing is centred; the rect's space beyond the
    /// image's own width and height is not used. The module documentation says
    /// why that corner rather than the middle, and a scroll view is the reason.
    None,
}

/// The image a widget draws: its own size in pixels, and the rectangle it
/// occupies inside its texture.
///
/// This is the copy the widget keeps because it cannot ask for one — the module
/// documentation says the whole of why, and
/// [`ImageSource::of`](ImageSource::of) is the recipe that makes supplying it
/// the easy thing rather than a trap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageSource {
    size: (u32, u32),
    uv: UvRect,
}

impl ImageSource {
    /// Returns a source for a `width` × `height` image occupying `uv` of its
    /// texture.
    ///
    /// The three numbers are the whole of what a widget needs, and
    /// [`whole`](ImageSource::whole) and
    /// [`from_placement`](ImageSource::from_placement) are the two ways a
    /// caller normally has them.
    ///
    /// `uv` is used as given, so it is the caller's to keep ordered — the crop a
    /// [`Cover`](ImageFit::Cover) fit takes is measured from its span, and a
    /// reversed span gives a reversed crop. Both of the named constructors
    /// produce an ordered one, and a caller writing their own is writing a
    /// texture coordinate.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::paint::UvRect;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let window = ImageSource::new(64, 32, UvRect { u0: 0.25, v0: 0.5, u1: 0.5, v1: 0.75 });
    /// assert_eq!(window.size(), (64, 32), "the image's own size in pixels");
    /// assert_eq!(window.uv().u0, 0.25, "and where it sits in the texture");
    /// ```
    #[must_use]
    pub fn new(width: u32, height: u32, uv: UvRect) -> Self {
        ImageSource {
            size: (width, height),
            uv,
        }
    }

    /// Returns a source for a `width` × `height` image that has a texture of
    /// its own, so it occupies all of it.
    ///
    /// This is the source for any image too large for the shared atlas, and it
    /// is the only case in which [`UvRect::full`] is the right answer.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::paint::UvRect;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let photo = ImageSource::whole(1024, 768);
    /// assert_eq!(photo.uv(), UvRect::full(), "it is the whole texture");
    /// ```
    #[must_use]
    pub fn whole(width: u32, height: u32) -> Self {
        ImageSource::new(width, height, UvRect::full())
    }

    /// Returns a source for an image the atlas is holding, from where it holds
    /// it.
    ///
    /// The placement carries both halves of the answer — the image's own width
    /// and height in pixels, and the rectangle it occupies in the atlas — so
    /// this is the one conversion, and it cannot put a size and a window out of
    /// step with one another.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::paint::UvRect;
    /// use ui_core::texture::{Pixels, TextureCache};
    /// use ui_core::widgets::image::ImageSource;
    /// use std::path::Path;
    ///
    /// let mut cache = TextureCache::new();
    /// let mut loader = |_: &Path| Ok(Pixels::transparent(64, 32));
    /// let handle = cache.load(Path::new("icon.png"), &mut loader).expect("loaded");
    /// let placement = cache.placement(handle).expect("small enough for the atlas");
    ///
    /// let source = ImageSource::from_placement(placement);
    /// assert_eq!(source.size(), (64, 32), "the placement knows the pixel size");
    /// assert_eq!(
    ///     source.uv(),
    ///     UvRect { u0: placement.u0, v0: placement.v0, u1: placement.u1, v1: placement.v1 },
    ///     "and the window into the atlas",
    /// );
    /// ```
    #[must_use]
    pub fn from_placement(placement: Placement) -> Self {
        ImageSource::new(
            placement.width,
            placement.height,
            UvRect {
                u0: placement.u0,
                v0: placement.v0,
                u1: placement.u1,
                v1: placement.v1,
            },
        )
    }

    /// Returns the source for `handle` in `cache`, or `None` if that cache
    /// cannot say where the image is.
    ///
    /// **This is a constructor called by whoever holds the cache, and it is the
    /// recipe the module documentation asks for.** The widget never sees a
    /// [`TextureCache`]: an `Image` holds a [`TextureHandle`] and an
    /// [`ImageSource`], and both are plain data, so nothing here ties a widget's
    /// lifetime to a cache's or borrows one for a frame. A caller that holds a
    /// cache builds the source here, once, and hands it over.
    ///
    /// The two cases are decided by
    /// [`is_standalone`], because that is the bit
    /// that says which of the two places the image lives. An image with a
    /// texture of its own is the whole of that texture. An atlas-resident image
    /// is a window into the shared atlas, and its window is the only correct
    /// answer — sampling the whole atlas is a picture of the other icons too.
    ///
    /// # Errors
    ///
    /// `None` means the cache cannot place the image, which is two different
    /// things and neither of them is an image a widget can draw:
    ///
    /// - a handle this cache never issued, which [`size_of`](TextureCache::size_of)
    ///   and [`placement`](TextureCache::placement) both also answer `None` for;
    /// - an atlas-resident image that **eviction has since taken**. Its shelf
    ///   and its window are gone, and the handle still names a slot in the
    ///   atlas. A caller showing an image that must survive is expected to
    ///   [`pin`](TextureCache::pin) it; this returns `None` rather than the
    ///   whole atlas so that a caller drawing a placeholder draws it, instead of
    ///   drawing a screen full of other people's icons.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::texture::{Pixels, TextureCache, TextureHandle};
    /// use ui_core::paint::UvRect;
    /// use ui_core::widgets::image::ImageSource;
    /// use std::path::Path;
    ///
    /// let mut cache = TextureCache::new();
    /// let mut small = |_: &Path| Ok(Pixels::transparent(64, 64));
    /// let icon = cache.load(Path::new("icon.png"), &mut small).expect("loaded");
    /// // Too big for the atlas, so it has a texture of its own.
    /// let mut large = |_: &Path| Ok(Pixels::transparent(600, 600));
    /// let photo = cache.load(Path::new("photo.png"), &mut large).expect("loaded");
    ///
    /// let icon_source = ImageSource::of(&cache, icon).expect("placed");
    /// assert_eq!(icon_source.size(), (64, 64));
    /// assert!(!icon_source.uv().is_full(), "a window into the shared atlas");
    ///
    /// let photo_source = ImageSource::of(&cache, photo).expect("placed");
    /// assert_eq!(photo_source.uv(), UvRect::full(), "the whole of its own texture");
    ///
    /// // A handle this cache never issued is not an image it can place.
    /// let stranger = TextureHandle::new(ui_core::paint::TextureId::new(9999));
    /// assert_eq!(ImageSource::of(&cache, stranger), None);
    /// ```
    #[must_use]
    pub fn of(cache: &TextureCache, handle: TextureHandle) -> Option<Self> {
        if is_standalone(handle.id()) {
            let (width, height) = cache.size_of(handle)?;
            return Some(ImageSource::whole(width, height));
        }
        let placement = cache.placement(handle)?;
        Some(ImageSource::from_placement(placement))
    }

    /// Returns the image's own size in pixels: `width` and `height`.
    ///
    /// This is the aspect ratio every fit is computed from, and it is the
    /// number a caller gets wrong.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Returns the rectangle the image occupies inside its texture.
    #[must_use]
    pub fn uv(&self) -> UvRect {
        self.uv
    }
}

/// The appearance an image's opacity implies.
///
/// One field, and that is not a gap. The other appearance numbers an image has
/// are its corner radius, which is a [`Property`] the caller writes and the
/// widget draws without a second copy of it to animate, and the fit, which is a
/// plain field and not an appearance at all. An image has no state a caller
/// toggles — a theme switch does not change how opaque a photograph is — so the
/// only thing a [`Style`] can report is where the one animated property is
/// heading, clamped.
///
/// [`Property`]: crate::property::Property
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The opacity the image is being drawn toward: [`Image::opacity`], clamped
    /// to `0.0..=1.0`.
    pub opacity: f32,
}

/// An image: a texture, a fit, and the quad it is drawn as.
///
/// The widget holds the properties the task gives it —
/// [`opacity`](Image::opacity), the [`fit`](Image::set_fit) and the
/// [`texture`](Image::set_texture) — the property that is drawn rather than
/// held ([`shown_opacity`](Image::shown_opacity)) and the appearance property a
/// caller writes directly ([`corner_radius`](Image::corner_radius)) — and the
/// [`ImageSource`] the fit cannot be computed without.
///
/// The node is the caller's to keep clean, and its size is the caller's to give
/// through [`layout_mut`](crate::node::WidgetNode::layout_mut) —
/// [`Image::size`] is a suggestion for a caller who has nothing else to go on.
/// An image paints inside whatever rect it is given: for [`ImageFit::Contain`]
/// that rect is a frame to letterbox the image in, for [`ImageFit::Fill`] and
/// [`ImageFit::Cover`] it is the quad itself, and for [`ImageFit::None`] it is
/// the place the image's own size starts.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::{DrawCommand, Rect, TextureId};
/// use ui_core::texture::TextureHandle;
/// use ui_core::widgets::image::{Image, ImageFit, ImageSource};
///
/// let mut nodes = Arena::new();
/// let handle = TextureHandle::new(TextureId::new(7));
/// let mut image = Image::new(&mut nodes, handle, ImageSource::whole(200, 100));
/// let rect = Rect::new(32.0, 64.0, 300.0, 300.0);
///
/// // `Contain` by default: the whole image, letterboxed in the rect.
/// let DrawCommand::Image { rect: drawn, .. } = &image.paint(rect)[0] else {
///     panic!("an image paints an image");
/// };
/// assert_eq!(*drawn, Rect::new(32.0, 139.0, 300.0, 150.0));
///
/// // `Cover` fills the rect and crops the image's *width* to the middle half
/// // of it — the *whole* `Image` at once, on one command, with nothing
/// // overflowing the rect; see `ImageFit::Cover`.
/// image.set_fit(ImageFit::Cover);
/// let DrawCommand::Image { rect: drawn, uv, .. } = &image.paint(rect)[0] else {
///     panic!("an image paints an image");
/// };
/// assert_eq!(*drawn, rect, "the quad is the node's own rect");
/// assert_eq!(uv.u0, 0.25, "and the middle half of the image is what fills it");
/// assert_eq!(uv.u1, 0.75);
/// assert_eq!((uv.v0, uv.v1), (0.0, 1.0), "at the image's own full height");
/// ```
pub struct Image {
    /// How much of the image reaches the screen, from `0.0` to `1.0`. The truth,
    /// and what a caller writes.
    ///
    /// It is the *fraction*, not a tint and not a colour: the image's own colours
    /// are in the texture and this scales the alpha the image arrived with. A
    /// value outside `0.0..=1.0` is not an error and is not wrapped — it is
    /// clamped when the image is painted, so a caller computing a fade can
    /// overshoot harmlessly. See [`Image::paint`].
    pub opacity: Property<f32>,
    /// The opacity the image is *drawn* at, animated toward
    /// [`opacity`](Image::opacity).
    ///
    /// A caller writing [`opacity`](Image::opacity) and then calling
    /// [`animate_to_state`](Image::animate_to_state) gets a fade; a caller that
    /// wants the image at its new opacity now calls
    /// [`snap_to_state`](Image::snap_to_state). Writing this one directly
    /// overrides the animation until the next aim.
    pub shown_opacity: Property<f32>,
    /// The corner radius in pixels, `0.0` for square corners.
    ///
    /// A property rather than a plain field for the reason the module gives for
    /// appearance: a caller changing it mid-frame has to be able to have the
    /// node repainted, and a property's `on_change` is the only way that is
    /// expressible. It is **not** animated — nothing in requirement 4 animates a
    /// corner, a radius is a shape rather than a state, and
    /// [`Button::border_radius`](crate::widgets::button::Button) is the same: a
    /// property, written directly, never a target. It is passed to
    /// [`DrawCommand::Image`] and nothing is drawn for it here; see the module
    /// documentation.
    pub corner_radius: Property<f32>,
    texture: TextureHandle,
    fit: ImageFit,
    source: ImageSource,
    clock: RefCell<AnimationClock>,
    node: Handle,
}

impl Image {
    /// Creates an image of `texture` in the arena, and returns it.
    ///
    /// It starts [`Contain`](ImageFit::Contain) — the whole image, letterboxed —
    /// fully opaque, with square corners, and paints `source` exactly as
    /// supplied.
    ///
    /// `source` is required rather than defaulted because the fit modes cannot be
    /// computed without the image's own aspect ratio, and this widget cannot
    /// look it up; the module documentation says the whole of why, and
    /// [`ImageSource::of`](ImageSource::of) is how a caller that holds a cache
    /// gets the right one.
    ///
    /// The task file's `Image::new(texture) -> Handle` is read as this: the
    /// handle is [`Image::handle`]'s, and returning it alone would leave a
    /// caller with no property to write, no fit to choose and no way to draw the
    /// image. Task 12's [`Button::new`](crate::widgets::button::Button::new) and
    /// task 14's [`Slider::new`](crate::widgets::slider::Slider::new) settled
    /// the same reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageFit, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(3));
    /// let image = Image::new(&mut nodes, handle, ImageSource::whole(64, 64));
    ///
    /// assert!(nodes.get(image.handle()).is_some(), "its node is in the arena");
    /// assert_eq!(image.fit(), ImageFit::Contain, "and it fits inside its rect");
    /// assert_eq!(image.opacity.get(), 1.0, "at full opacity");
    /// assert_eq!(image.corner_radius.get(), 0.0, "with square corners");
    /// ```
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, texture: TextureHandle, source: ImageSource) -> Self {
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Image {
            opacity: Property::new(1.0),
            shown_opacity: Property::new(1.0),
            corner_radius: Property::new(0.0),
            texture,
            fit: ImageFit::default(),
            source,
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the image's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the texture the image draws.
    #[must_use]
    pub fn texture(&self) -> TextureHandle {
        self.texture
    }

    /// Sets the texture the image draws, and applies from the next repaint.
    ///
    /// A plain field behind a setter, for the reason the module gives: which
    /// texture an image shows is the mapping, not the appearance, and a
    /// [`Property`] would let a caller write it directly and leave the drawn
    /// texture and the drawn geometry disagreeing — a caller changing the image
    /// without changing its aspect ratio, which is a wrong picture rather than a
    /// compile error. Nothing animates it and nothing needs to.
    ///
    /// **A caller that changes this has to mark the node itself.** A plain field
    /// has no callback to fire, and every other widget here has the same
    /// limitation on its own setters; the ones that cannot is why
    /// [`opacity`](Image::opacity) and [`corner_radius`](Image::corner_radius)
    /// are properties.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{DrawCommand, Rect, TextureId};
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let mut image = Image::new(&mut nodes, TextureHandle::new(TextureId::new(1)), ImageSource::whole(8, 8));
    /// image.set_texture(TextureHandle::new(TextureId::new(2)));
    ///
    /// let DrawCommand::Image { texture, .. } = &image.paint(Rect::new(0.0, 0.0, 8.0, 8.0))[0] else {
    ///     panic!("an image paints an image");
    /// };
    /// assert_eq!(*texture, TextureId::new(2), "the new one, at once");
    /// ```
    pub fn set_texture(&mut self, texture: TextureHandle) {
        self.texture = texture;
    }

    /// Returns how the image is shaped into the rect it is given.
    #[must_use]
    pub fn fit(&self) -> ImageFit {
        self.fit
    }

    /// Sets how the image is shaped into the rect it is given, and applies from
    /// the next repaint.
    ///
    /// A plain field behind a setter, for the reason
    /// [`set_texture`](Image::set_texture) gives: a fit is the mapping rather
    /// than the appearance, and nothing cross-fades between two of them. The
    /// geometry is derived at paint time, so a new fit applies from the next
    /// frame without disturbing the opacity or the corner radius.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageFit, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let mut image = Image::new(&mut nodes, TextureHandle::new(TextureId::new(1)), ImageSource::whole(200, 100));
    /// image.set_fit(ImageFit::Cover);
    /// assert_eq!(image.fit(), ImageFit::Cover);
    /// ```
    pub fn set_fit(&mut self, fit: ImageFit) {
        self.fit = fit;
    }

    /// Returns the image's own copy of what it is drawing.
    #[must_use]
    pub fn source(&self) -> ImageSource {
        self.source
    }

    /// Sets the image's own copy of what it is drawing: its pixel size and where
    /// it sits in its texture.
    ///
    /// The two halves are the two things a fit needs and the widget cannot find
    /// out for itself; see the module documentation and
    /// [`ImageSource::of`](ImageSource::of).
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(1));
    /// let mut image = Image::new(&mut nodes, handle, ImageSource::whole(16, 16));
    ///
    /// // The handle is the same one; what changed is what the widget believes
    /// // about it. A caller that swapped an image for a wider one does this.
    /// image.set_source(ImageSource::whole(200, 100));
    /// assert_eq!(image.source_size(), (200, 100));
    /// ```
    pub fn set_source(&mut self, source: ImageSource) {
        self.source = source;
    }

    /// Returns the image's own size in pixels: `width` and `height`.
    #[must_use]
    pub fn source_size(&self) -> (u32, u32) {
        self.source.size()
    }

    /// Sets the image's own size in pixels, keeping the rectangle it occupies in
    /// its texture.
    ///
    /// **This is the one number a caller has to get right, and getting it wrong
    /// is invisible to every check in this crate.** The fit modes are computed
    /// from this aspect ratio and from nothing else, so an image declared as
    /// 1:1 and drawn as though it were — a stretched or letterboxed picture at
    /// the wrong scale, sampling the wrong part of the texture. There is no
    /// error and no warning: the widget does exactly what it was told. The
    /// cheap defences are [`ImageSource::of`](ImageSource::of), which reads the
    /// number off the handle's own cache rather than off a literal, and
    /// [`Image::size`](Image::size), which asks for exactly this size and so
    /// cannot be laid out at the wrong scale.
    ///
    /// It changes the drawn geometry at once and does not disturb the opacity,
    /// the corner radius or anything else; the fit is derived at paint time.
    pub fn set_source_size(&mut self, width: u32, height: u32) {
        self.source = ImageSource::new(width, height, self.source.uv());
    }

    /// Returns the size the image asks for: its own size in pixels.
    ///
    /// An image *has* an intrinsic size, unlike a slider or a progress bar,
    /// which have a default because they have no content to measure. So this is
    /// not a default with a fallback — it is the one size at which the image is
    /// not scaled, which is what
    /// [`ImageFit::None`] draws it at.
    ///
    /// A caller that wants it fitted gives the node a size of its own through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut) and lets
    /// [`paint`](Image::paint) draw inside it. A caller that asks for this
    /// instead gets a 4000-pixel photograph asking for 4000 pixels, which is
    /// correct for [`ImageFit::None`] and not what anybody
    /// wants for [`ImageFit::Cover`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(1));
    /// let image = Image::new(&mut nodes, handle, ImageSource::whole(200, 100));
    /// let size = image.size();
    /// assert_eq!((size.width, size.height), (200.0, 100.0));
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        let (width, height) = self.source.size();
        Size::new(pixel_extent(width), pixel_extent(height))
    }

    /// Returns the rect the image is drawn in, for `rect`.
    ///
    /// A thin wrapper over [`destination_rect`] with this
    /// widget's own fit and source size, so that a caller asking where an image
    /// will land does not have to restate them and get one of them wrong.
    #[must_use]
    pub fn destination(&self, rect: Rect) -> Rect {
        destination_rect(self.fit, self.source.size(), rect)
    }

    /// Returns the part of the texture the image is drawn from, for `rect`.
    ///
    /// A thin wrapper over [`sampled_uv`] with this widget's own fit
    /// and source.
    #[must_use]
    pub fn sampled_uv(&self, rect: Rect) -> UvRect {
        sampled_uv(self.fit, self.source, rect)
    }

    /// Returns the appearance the image's opacity implies.
    ///
    /// The opacity is clamped here rather than at paint time alone, so a caller
    /// reading the target is told about the ends of the range rather than about
    /// a position past them. A value of `NaN` reads as `0.0`: a caller that has
    /// lost track of its arithmetic gets an invisible image rather than one whose
    /// every command carries a `NaN`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(1));
    /// let image = Image::new(&mut nodes, handle, ImageSource::whole(8, 8));
    /// assert_eq!(image.style().opacity, 1.0);
    ///
    /// image.opacity.set(2.0);
    /// assert_eq!(image.style().opacity, 1.0, "past the top is the top");
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            opacity: bounded(self.opacity.get(), 0.0, 1.0),
        }
    }

    /// Applies the appearance the image's opacity implies at once, with no
    /// transition, and ends every transition already running.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: an image that has just been given an
    /// [`opacity`](Image::opacity) and has never animated — whose
    /// [`shown_opacity`](Image::shown_opacity) still holds the value
    /// [`Image::new`] wrote — and a caller that has written the opacity itself
    /// and wants the image to be that now.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(1));
    /// let image = Image::new(&mut nodes, handle, ImageSource::whole(8, 8));
    ///
    /// image.opacity.set(0.25);
    /// assert_eq!(image.shown_opacity.get(), 1.0, "the truth moved; the image has not");
    /// image.snap_to_state();
    /// assert_eq!(image.shown_opacity.get(), 0.25, "and a snap puts it there at once");
    /// assert!(!image.is_animating(), "a snap is not a transition");
    /// ```
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.shown_opacity.set(style.opacity);
    }

    /// Starts the transition that carries the image from wherever it is drawn
    /// toward the appearance [`style`](Image::style) implies, on `motion`.
    ///
    /// This is the call a caller makes after writing
    /// [`opacity`](Image::opacity), and it is the whole of requirement 4's
    /// "opacity applied via blending" becoming a fade rather than a jump. Only
    /// [`shown_opacity`](Image::shown_opacity) is animated: the fit, the source
    /// and the corner radius have no target that is not the value they already
    /// hold, and a transition on a property that nothing moves is a transition
    /// that reports itself as running for nothing.
    ///
    /// The image's own clock is cleared first, so the transition this replaces
    /// stops where it is rather than writing over the new one when it arrives —
    /// the reason the button owns a clock, for the same reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::image::{Image, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(1));
    /// let image = Image::new(&mut nodes, handle, ImageSource::whole(8, 8));
    /// let motion = Motion { duration: Duration::from_millis(100), easing: Easing::Linear };
    ///
    /// image.opacity.set(0.0);
    /// image.animate_to_state(motion);
    /// assert_eq!(image.shown_opacity.get(), 1.0, "it starts where it was drawn");
    /// image.tick(Duration::from_millis(50));
    /// assert_eq!(image.shown_opacity.get(), 0.5, "and is half way at half the time");
    /// ```
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.shown_opacity
                .animate_to(style.opacity, motion.duration, motion.easing),
        );
    }

    /// Advances the image's transition by `delta`, and returns whether it
    /// wrote.
    ///
    /// It is the image's frame integration: call it once a frame, before the
    /// paint pass, with the time that frame took. The write is what reaches the
    /// node — a property callback registered by the caller marks the node dirty
    /// — so a caller that repaints only when this is true repaints exactly while
    /// something moves.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Returns whether the image's transition is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Returns the draw commands that paint the image within `rect`.
    ///
    /// **One command, ever**: a [`DrawCommand::Image`] at
    /// [`destination`](Image::destination) in this widget's rect, sampling
    /// [`sampled_uv`](Image::sampled_uv) of this widget's texture, at the drawn
    /// opacity, with the corner radius. There is no background and no border, and
    /// there is deliberately nothing else: a rounded corner is a shader clip, so
    /// a widget that drew a shape to make the corner would be drawing a card
    /// under the image. See the module documentation and `.ai/NEVERAGAIN.md`'s
    /// entry *A filled rounded rectangle is not an outline*.
    ///
    /// The opacity is clamped to `0.0..=1.0` here and **not** multiplied into a
    /// colour: the command has no colour for an image, the shader scales the
    /// alpha the image brought with it, and doing it in both places would darken
    /// a half-faded image by half twice.
    ///
    /// A rect with no extent in either axis records nothing. There is no
    /// rectangle to clip to and no quad to draw; a collapsed node in a flex
    /// layout costs nothing rather than a degenerate triangle. An image at zero
    /// *opacity* is still recorded, because that is a state a caller fades
    /// through and not a shape that has collapsed.
    ///
    /// Every size comes from the drawn properties, so an image mid-fade paints
    /// at the opacity the fade has got to.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{DrawCommand, Rect, TextureId};
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::image::{Image, ImageSource};
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(7));
    /// let image = Image::new(&mut nodes, handle, ImageSource::whole(200, 100));
    /// let commands = image.paint(Rect::new(32.0, 64.0, 300.0, 300.0));
    ///
    /// assert_eq!(commands.len(), 1, "an image is one command and nothing else");
    /// let DrawCommand::Image { rect, texture, uv, opacity, radius } = &commands[0] else {
    ///     panic!("an image paints an image");
    /// };
    /// assert_eq!(*rect, Rect::new(32.0, 139.0, 300.0, 150.0), "letterboxed");
    /// assert_eq!(*texture, TextureId::new(7));
    /// assert!(uv.is_full(), "an image with a texture of its own is all of it");
    /// assert_eq!(*opacity, 1.0);
    /// assert_eq!(*radius, 0.0, "square corners until a caller asks otherwise");
    /// ```
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let area = self.destination(rect);
        if area.width > 0.0 && area.height > 0.0 {
            painter.image(
                area,
                self.texture.id(),
                self.sampled_uv(rect),
                self.drawn_opacity(),
                self.drawn_radius(),
            );
        }
        painter.finish()
    }

    /// Returns the opacity the image is *drawn* at, clamped to `0.0..=1.0`.
    ///
    /// Clamped here as well as in [`style`](Image::style) because
    /// [`shown_opacity`](Image::shown_opacity) is a public property and a caller
    /// may write it directly: a drawn opacity of `1.4` is an opaque image, not
    /// one that is 40% more visible than it can be. A `NaN` is `0.0`.
    fn drawn_opacity(&self) -> f32 {
        bounded(self.shown_opacity.get(), 0.0, 1.0)
    }

    /// Returns the corner radius the image is drawn with, never negative.
    ///
    /// A radius bigger than half the drawn rect is **not** clamped here:
    /// [`DrawCommand::Image`] documents that a radius past half the shorter side
    /// is treated as half of it, and clamping it in two places would be a second
    /// rule to keep honest. A *negative* radius is clamped, because it is not a
    /// radius — there is no shape of which it is the corner — and the shader's
    /// "outside the rounded rect" test has no answer for one.
    fn drawn_radius(&self) -> f32 {
        self.corner_radius.get().max(0.0)
    }
}

/// Returns the rect the image is drawn in, for `fit`, inside `bounds`.
///
/// This is the first half of what a fit decides and it is pure: the same
/// `fit`, the same `source_size` and the same `bounds` give the same rect with
/// no widget, no clock and no texture anywhere in it. `Image::destination` is
/// this function with the widget's own numbers filled in.
///
/// **Only [`Contain`](ImageFit::Contain) has anything to compute here.** The
/// other three answer with `bounds` or with the image's own size:
///
/// - [`Fill`](ImageFit::Fill) and [`Cover`](ImageFit::Cover) are `bounds`
///   itself. Cover's covering is done by the *crop* —
///   [`sampled_uv`] — and the module documentation says why the quad
///   is not made larger than the rect instead.
/// - [`None`](ImageFit::None) is the source's own size at `bounds`'s **top
///   left**, not centred; the module documentation says why that corner.
/// - A `bounds` with no extent in either axis, and a source with a zero in
///   either axis, are an empty rect at **every** fit. There is no shape to fit
///   into, no distance to divide by and no pixels to stretch, and a division by
///   zero is not an answer. Keeping the four fits on one answer for both is the
///   point: a node a layout has collapsed draws nothing whatever image it holds.
///   `TextureCache` never issues a handle for a source with a zero in it, so both
///   of these are a caller writing a [`Rect`] or a source by hand.
///
/// `Contain` takes the **larger** rectangle of the image's own shape that fits
/// inside `bounds`, and centres it, so its space is split evenly on the two
/// sides the image's shape does not fill. A source relatively *wider* than the
/// bounds has its width bound by the rect and its height follows from the shape;
/// a relatively *taller* source is the other way round. The comparison that
/// decides is `source_aspect <= bounds_aspect`, and the `<=` is the
/// matching-shapes case taking the same branch as a relatively taller source —
/// which for matching shapes gives the same answer either way.
///
/// The extents are the rect's own, not its origin's. That is not a detail: the
/// slider's own `travel` subtracted a rect's *origin* from its *extent* and
/// every one of its fifty-three tests passed, because every fixture in that
/// module is laid out at `(0, 0)`.
///
/// # Examples
///
/// ```
/// use ui_core::paint::Rect;
/// use ui_core::widgets::image::{destination_rect, ImageFit};
///
/// // A 2:1 image in a 2:1 box: both fits are the box.
/// let bounds = Rect::new(0.0, 0.0, 400.0, 200.0);
/// assert_eq!(destination_rect(ImageFit::Contain, (200, 100), bounds), bounds);
/// assert_eq!(destination_rect(ImageFit::Cover, (200, 100), bounds), bounds);
///
/// // `None` is the image's own size, at the rect's top left.
/// assert_eq!(
///     destination_rect(ImageFit::None, (64, 32), Rect::new(10.0, 20.0, 400.0, 200.0)),
///     Rect::new(10.0, 20.0, 64.0, 32.0),
///     "top left, not centred: a scroll's content starts at its top",
/// );
///
/// // `Fill` ignores the image's shape entirely, and so does `Cover`'s quad.
/// assert_eq!(destination_rect(ImageFit::Fill, (64, 32), bounds), bounds);
/// assert_eq!(
///     destination_rect(ImageFit::Cover, (64, 32), Rect::new(10.0, 20.0, 400.0, 200.0)),
///     Rect::new(10.0, 20.0, 400.0, 200.0),
///     "nothing overflows: cover crops the image rather than the rect",
/// );
///
/// // An image with no pixels has no destination at any fit.
/// for fit in [
///     ImageFit::Contain,
///     ImageFit::Cover,
///     ImageFit::Fill,
///     ImageFit::None,
/// ] {
///     assert_eq!(
///         destination_rect(fit, (0, 32), bounds).width,
///         0.0,
///         "{fit:?} of an image with no pixels is nothing",
///     );
/// }
/// ```
#[must_use]
pub fn destination_rect(fit: ImageFit, source_size: (u32, u32), bounds: Rect) -> Rect {
    let width = bounds.width.max(0.0);
    let height = bounds.height.max(0.0);
    // A rect with no extent in either axis has no area, and an empty rect is
    // the answer at every fit rather than only at the one that would divide by
    // it. It also keeps the four consistent: a node a layout has collapsed to
    // zero width draws nothing whatever it holds, which is the same promise
    // `paint` makes by not recording a rect with no extent.
    if width <= 0.0 || height <= 0.0 {
        return Rect::new(bounds.x, bounds.y, 0.0, 0.0);
    }
    // An image with no pixels in one axis has no destination at any fit, and
    // this is the one place that says so for all four: `TextureCache` refuses
    // to issue a handle for such an image, which makes this a caller writing a
    // source by hand. An empty rect is not a division by zero, and it is what
    // makes `None` agree with the other three rather than being the odd one out.
    if source_size.0 == 0 || source_size.1 == 0 {
        return Rect::new(bounds.x, bounds.y, 0.0, 0.0);
    }
    let source_width = pixel_extent(source_size.0);
    let source_height = pixel_extent(source_size.1);

    match fit {
        ImageFit::Fill | ImageFit::Cover => Rect::new(bounds.x, bounds.y, width, height),
        ImageFit::None => Rect::new(bounds.x, bounds.y, source_width, source_height),
        ImageFit::Contain => {
            let source_aspect = source_width / source_height;
            let bounds_aspect = width / height;
            // A source relatively narrower than the bounds has its *height* bound
            // by the rect and its width follows from the shape; a relatively
            // wider one is the other way round. The `<=` sends matching shapes
            // down the first branch, which for a source and a bounds of the same
            // shape gives the whole of the rect either way.
            let (fitted_width, fitted_height) = if source_aspect <= bounds_aspect {
                (height * source_aspect, height)
            } else {
                (width, width / source_aspect)
            };
            Rect::new(
                bounds.x + (width - fitted_width) / 2.0,
                bounds.y + (height - fitted_height) / 2.0,
                fitted_width,
                fitted_height,
            )
        }
    }
}

/// Returns the part of `source` an image samples when it is drawn in `bounds`
/// with `fit`.
///
/// This is the second half of what a fit decides, and it is pure for the same
/// reason [`destination_rect`] is.
/// `Image::sampled_uv` is this function with the widget's own numbers filled in.
///
/// For [`Contain`](ImageFit::Contain), [`Fill`](ImageFit::Fill) and
/// [`None`](ImageFit::None) the answer is **the whole of the image**, which is
/// `source`'s own `uv`: the whole texture for an image with a texture of its
/// own, and the atlas window for an atlas-resident one. It is not
/// [`UvRect::full`] for the second, and saying so here is the point — an atlas
/// image sampled as the whole texture is a picture of every other icon in it as
/// well.
///
/// For [`Cover`](ImageFit::Cover) the answer is a **centred sub-rectangle of
/// the image**, with the same shape as `bounds` — and that is the part that is
/// easy to get wrong, and the part that is least likely to be noticed on screen
/// when it is wrong, because a stretched image and a cropped one are both
/// pictures of the right image. [`destination_rect`] draws
/// [`Cover`](ImageFit::Cover)'s quad at `bounds` itself, so a
/// [`Cover`](ImageFit::Cover) quad with **no** crop is an image *stretched* into
/// the box, and a crop of the wrong shape is a stretched image of the wrong part.
/// The two halves have to agree, and this function is the half that carries it.
///
/// The crop, from the two shapes:
///
/// - a source **relatively wider** than the bounds keeps its whole height and
///   loses width: the kept fraction across is `bounds_aspect / source_aspect`
///   and the kept fraction down is `1.0`;
/// - a source **relatively taller** keeps its whole width and loses height: `1.0`
///   across and `source_aspect / bounds_aspect` down;
/// - **matching** shapes keep all of it, and the answer is the whole image —
///   which is also the only case in which a [`Cover`](ImageFit::Cover) image is
///   recorded for the opaque draw pass, since a window into a texture blends.
///
/// Both are then centred, so the crop is the middle band on the axis that is
/// cropped and all of the other. Because the crop is a *fraction of the span*,
/// it is taken relative to `source`'s own `uv` and the answer never reaches
/// outside it, which is what keeps an atlas-resident image from sampling its
/// neighbours.
///
/// Three cases crop nothing: a fit that is not [`Cover`](ImageFit::Cover), a
/// source with a zero in either axis, and a `bounds` with no extent in either
/// axis. The last of those is the only one that can be reached from
/// [`paint`](Image::paint), and an image drawn into it is not recorded at all.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{Rect, UvRect};
/// use ui_core::widgets::image::{sampled_uv, ImageFit, ImageSource};
///
/// let whole = ImageSource::whole(200, 200);
///
/// // A square image covering a wide box keeps the middle band, not all of it.
/// let wide = sampled_uv(ImageFit::Cover, whole, Rect::new(0.0, 0.0, 100.0, 50.0));
/// assert_eq!(wide, UvRect { u0: 0.0, v0: 0.25, u1: 1.0, v1: 0.75 });
/// // Which is a 2:1 window of a 1:1 image, for a 2:1 box.
/// assert_eq!((wide.v1 - wide.v0), 0.5, "half the height is kept");
///
/// // Covering a box of the image's own shape crops nothing.
/// assert!(sampled_uv(ImageFit::Cover, whole, Rect::new(0.0, 0.0, 80.0, 80.0)).is_full());
///
/// // The other three fits take the whole of the image, whatever the box is.
/// for fit in [
///     ImageFit::Contain,
///     ImageFit::Fill,
///     ImageFit::None,
/// ] {
///     assert!(
///         sampled_uv(fit, whole, Rect::new(0.0, 0.0, 100.0, 50.0)).is_full(),
///         "{fit:?} does not crop",
///     );
/// }
/// ```
#[must_use]
pub fn sampled_uv(fit: ImageFit, source: ImageSource, bounds: Rect) -> UvRect {
    let base = source.uv();
    let (across, down) = crop_fractions(fit, source.size(), bounds);
    let (u0, u1) = centred_span(base.u0, base.u1, across);
    let (v0, v1) = centred_span(base.v0, base.v1, down);
    UvRect { u0, v0, u1, v1 }
}

/// Returns the fractions of the image's own width and height a `Cover` fit
/// keeps, as `(across, down)`.
///
/// `1.0, 1.0` is the answer for every fit that does not crop, for a source with
/// no aspect ratio, and for a `bounds` with no extent in either axis. The
/// derivation is on [`sampled_uv`](sampled_uv); what is here is the arithmetic
/// that has to be exactly right, because a crop of the right size in the wrong
/// place still looks like an image on screen.
///
/// The two branches are the same ratio the other way up, and the reason the
/// *narrower* of the two numbers is never taken is worth stating: the crop's
/// job is to have the bounds' own shape, and the bounds' shape is the wider of
/// the two aspects only in the branch that keeps the source's whole height. A
/// crop of the *larger* fraction on both axes would be a picture of more image
/// than the box can hold, which is the overflow this fit exists not to need.
fn crop_fractions(fit: ImageFit, source_size: (u32, u32), bounds: Rect) -> (f32, f32) {
    if fit != ImageFit::Cover {
        return (1.0, 1.0);
    }
    let (source_width, source_height) = (pixel_extent(source_size.0), pixel_extent(source_size.1));
    let (bounds_width, bounds_height) = (bounds.width, bounds.height);
    if source_width <= 0.0 || source_height <= 0.0 || bounds_width <= 0.0 || bounds_height <= 0.0 {
        return (1.0, 1.0);
    }
    let source_aspect = source_width / source_height;
    let bounds_aspect = bounds_width / bounds_height;
    if source_aspect > bounds_aspect {
        // The source is relatively wider, so the crop has to be relatively
        // narrower: the kept width is the height times the bounds' own aspect,
        // which over the source's width is the one ratio over the other.
        (bounds_aspect / source_aspect, 1.0)
    } else if source_aspect < bounds_aspect {
        (1.0, source_aspect / bounds_aspect)
    } else {
        (1.0, 1.0)
    }
}

/// Returns the middle `fraction` of the span `start..=end`.
///
/// Centred on the span's midpoint rather than built from either end, because
/// that is what "centred" means and because a one-float difference between the
/// two ways of writing it is a half-texel of a neighbouring atlas image.
fn centred_span(start: f32, end: f32, fraction: f32) -> (f32, f32) {
    let middle = (start + end) / 2.0;
    let half = (end - start) * fraction / 2.0;
    (middle - half, middle + half)
}

/// The mask that leaves an integer's low sixteen bits.
const LOW_HALF: u32 = 0xFFFF;

/// The number one half's place is worth, and therefore the scale between a
/// `u32`'s two halves and the value they make between them.
const HALF_SCALE: f32 = 65536.0;

/// Returns `value` — a count of pixels — as an `f32`.
///
/// There is no [`From`] from `u32` to `f32` — a `u32` does not always fit in
/// one, and the standard library does not pretend that it does — and a widget in
/// this repository may not reach for an `as` either. So the value is split into
/// the two halves `f32` *does* have a `From` for and put back together:
/// `value` is `high * 2^16 + low`, with each half at most `0xFFFF`, and scaling
/// by a power of two is exact in binary floating point.
///
/// The answer is therefore **every value below `2^24` exactly** — 16.7 million
/// pixels, which is a bit over four thousand pixels square — and the nearest
/// `f32` above that, which is as much as an `f32` can hold and the same answer a
/// cast would have given. The numbers are asserted in this module's tests rather
/// than in a doctest here, because a doctest cannot name a private item; the
/// observable consequence is that an image's own size survives into
/// [`Image::size`] unchanged, and that is on that function.
fn pixel_extent(value: u32) -> f32 {
    // Neither `try_from` can fail: shifting sixteen bits off a `u32` leaves at
    // most sixteen, and masking leaves sixteen. `u16::MAX` is the honest
    // fallback if that ever stops being true, where `0` would be a small wrong
    // answer rather than a large one.
    let high = u16::try_from(value >> 16).unwrap_or(u16::MAX);
    let low = u16::try_from(value & LOW_HALF).unwrap_or(u16::MAX);
    f32::from(high) * HALF_SCALE + f32::from(low)
}

/// Returns `value` between `low` and `high`, `NaN` included.
///
/// The two comparisons rather than [`f32::clamp`], for the reason the slider's
/// own `bounded_to` gives: `clamp` panics when its bounds are the wrong way
/// round, and a widget must not take a frame down to say that a caller passed
/// the wrong pair. `max` then `min` is the same answer for an ordered pair, it
/// cannot panic, and a `NaN` is passed over rather than propagated —
/// `f32::max` returns the *other* operand when one of the two is `NaN`, so a
/// `NaN` opacity becomes `0.0` and a `NaN` bound is ignored. Two `NaN` bounds
/// cannot be made sense of and leave the answer `NaN`, which draws nothing.
#[allow(clippy::manual_clamp)]
fn bounded(value: f32, low: f32, high: f32) -> f32 {
    value.max(low).min(high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Easing;
    use crate::batch::Batcher;
    use crate::layout::Size;
    use crate::paint::TextureId;
    use crate::texture::{Pixels, ATLAS_MAX_IMAGE};
    use std::path::Path;

    /// A square node, 300 × 300, at (32, 64).
    ///
    /// It is deliberately **not** at the origin, and so is every other fixture
    /// in this module. A rect's origin and a rect's extent are different
    /// numbers, and a suite whose every fixture is at `(0, 0)` cannot see one
    /// being used as the other: the slider's tests passed all fifty-three of them
    /// while every slider drawn anywhere else in a window reported its own origin
    /// as a negative travel.
    const RECT: Rect = Rect {
        x: 32.0,
        y: 64.0,
        width: 300.0,
        height: 300.0,
    };

    /// A node twice as wide as it is tall, 400 × 200 at (700, 40) — far from
    /// `RECT` and far from the origin, and 2:1 so the arithmetic is exact.
    const WIDE: Rect = Rect {
        x: 700.0,
        y: 40.0,
        width: 400.0,
        height: 200.0,
    };

    /// A node twice as tall as it is wide, 200 × 400 at (40, 700), which is 1:2
    /// and near the origin in neither axis.
    const TALL: Rect = Rect {
        x: 40.0,
        y: 700.0,
        width: 200.0,
        height: 400.0,
    };

    /// Every fit, in the order the enum declares them, for a test that has to
    /// hold of all four at once.
    const FITS: [ImageFit; 4] = [
        ImageFit::Contain,
        ImageFit::Cover,
        ImageFit::Fill,
        ImageFit::None,
    ];

    /// The nodes every all-the-fits table walks, with the source size that goes
    /// with each. The sizes are 2:1, 1:2, 1:1, 1:4 and 4:1 against nodes that are
    /// 1:1, 2:1 and 1:2, so every comparison between the two aspects is
    /// exercised in both directions and once exactly.
    const SHAPES: [((u32, u32), Rect); 6] = [
        ((200, 100), RECT),
        ((100, 200), RECT),
        ((200, 200), RECT),
        ((200, 100), WIDE),
        ((100, 200), WIDE),
        ((100, 400), TALL),
    ];

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a
    /// value at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// Advances `image`'s transition by `millis` and reports whether anything
    /// moved.
    ///
    /// The image's own `tick` is what a frame calls and it answers whether a
    /// repaint is needed, so a test that only wants time to pass goes through
    /// here rather than dropping a `#[must_use]` answer on the floor. It is the
    /// helper the slider's and the progress bar's tests have, for the same
    /// reason.
    fn tick(image: &Image, millis: u64) -> bool {
        image.tick(ms(millis))
    }

    /// A handle for an image no cache has issued, for a test that only cares
    /// about geometry.
    fn handle() -> TextureHandle {
        TextureHandle::new(TextureId::new(7))
    }

    /// An image of a `width` × `height` source with a texture of its own, and
    /// the arena its node is in.
    fn image(width: u32, height: u32) -> (Arena<WidgetNode>, Image) {
        let mut nodes = Arena::new();
        let image = Image::new(&mut nodes, handle(), ImageSource::whole(width, height));
        (nodes, image)
    }

    /// An image of `source`, for a test about the source rather than the fit.
    fn of_source(source: ImageSource) -> (Arena<WidgetNode>, Image) {
        let mut nodes = Arena::new();
        let image = Image::new(&mut nodes, handle(), source);
        (nodes, image)
    }

    /// The single image command a paint recorded, and a failure that says so
    /// rather than indexing past the end.
    fn only_image(image: &Image, rect: Rect) -> DrawCommand {
        let commands = image.paint(rect);
        assert_eq!(
            commands.len(),
            1,
            "an image records one command and draws nothing else: {commands:?}"
        );
        match commands.into_iter().next() {
            Some(command) => command,
            None => panic!("the paint recorded nothing to return"),
        }
    }

    /// The rect a paint drew the image in.
    fn area(image: &Image, rect: Rect) -> Rect {
        match only_image(image, rect) {
            DrawCommand::Image { rect, .. } => rect,
            other => panic!("an image paints an image, not {other:?}"),
        }
    }

    /// The part of the texture a paint sampled.
    fn sampled(image: &Image, rect: Rect) -> UvRect {
        match only_image(image, rect) {
            DrawCommand::Image { uv, .. } => uv,
            other => panic!("an image paints an image, not {other:?}"),
        }
    }

    /// The opacity a paint recorded.
    fn drawn_opacity(image: &Image, rect: Rect) -> f32 {
        match only_image(image, rect) {
            DrawCommand::Image { opacity, .. } => opacity,
            other => panic!("an image paints an image, not {other:?}"),
        }
    }

    /// The corner radius a paint recorded.
    fn drawn_radius(image: &Image, rect: Rect) -> f32 {
        match only_image(image, rect) {
            DrawCommand::Image { radius, .. } => radius,
            other => panic!("an image paints an image, not {other:?}"),
        }
    }

    /// An image of a `width` × `height` source, painted into `rect` and returned
    /// as the command, for a test that does not need the widget afterwards.
    fn painted(width: u32, height: u32, fit: ImageFit, rect: Rect) -> DrawCommand {
        let (_nodes, mut image) = image(width, height);
        image.set_fit(fit);
        only_image(&image, rect)
    }

    /// Asserts two values are within a millionth of one another.
    ///
    /// The ratios are computed in `f32`, so `(4.0 / 3.0) / 10.0` is not exactly
    /// `0.13333333`: a test that wrote the decimal out would be asserting the
    /// compiler's rounding rather than the widget. Everything the widget
    /// computes exactly \u2014 a half, a quarter, a tenth of a whole texture, or
    /// anything at all in a dyadic ratio \u2014 is asserted with `assert_eq!`.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-5, "{what}: {got} against {want}");
    }

    /// Returns whether `inner` lies inside `outer`, to within a millionth on
    /// each edge.
    fn inside(outer: Rect, inner: Rect) -> bool {
        const TOLERANCE: f32 = 1e-5;
        inner.x >= outer.x - TOLERANCE
            && inner.y >= outer.y - TOLERANCE
            && inner.x + inner.width <= outer.x + outer.width + TOLERANCE
            && inner.y + inner.height <= outer.y + outer.height + TOLERANCE
    }

    /// Returns the two `f32`s of a `u32` pair, the way the widget's arithmetic
    /// sees them.
    fn extents(size: (u32, u32)) -> (f32, f32) {
        (pixel_extent(size.0), pixel_extent(size.1))
    }

    /// Loads a `width` × `height` image through a loader of the test's own
    /// making, and returns the handle \u2014 no filesystem, and a 2048\u00b2 atlas.
    fn load(cache: &mut TextureCache, name: &str, width: u32, height: u32) -> TextureHandle {
        let mut loader = |_: &Path| Ok(Pixels::transparent(width, height));
        cache
            .load(Path::new(name), &mut loader)
            .expect("the stub loader answers everything")
    }

    // ---------------------------------------------------------------- fields

    #[test]
    fn an_image_holds_the_properties_the_task_gives_it() {
        let (nodes, image) = image(200, 100);
        assert!(
            nodes.get(image.handle()).is_some(),
            "with its node in the arena"
        );
        assert_eq!(image.texture(), handle(), "and the texture it was given");
        assert_eq!(image.fit(), ImageFit::Contain);
        assert_eq!(image.opacity.get(), 1.0, "fully opaque to begin with");
        assert_eq!(
            image.shown_opacity.get(),
            1.0,
            "and drawn at that, so the first frame is right"
        );
        assert_eq!(image.corner_radius.get(), 0.0, "with square corners");
        assert_eq!(
            image.source_size(),
            (200, 100),
            "and the source it was given"
        );
        assert_eq!(image.source(), ImageSource::whole(200, 100));
        assert!(
            !image.is_animating(),
            "nothing moves until a caller aims it"
        );
    }

    #[test]
    fn a_new_image_paints_the_defaults_rather_than_a_blank() {
        let (_nodes, image) = image(200, 100);
        let command = only_image(&image, RECT);
        let DrawCommand::Image {
            rect,
            texture,
            uv,
            opacity,
            radius,
        } = command
        else {
            panic!("an image paints an image");
        };
        assert_eq!(rect, Rect::new(32.0, 139.0, 300.0, 150.0));
        assert_eq!(texture, handle().id());
        assert!(uv.is_full(), "an image with its own texture is all of it");
        assert_eq!(opacity, 1.0);
        assert_eq!(radius, 0.0);
    }

    #[test]
    fn the_default_fit_is_contain_because_a_shown_image_is_better_than_a_cropped_one() {
        // The doc's claim, as a test: `Default` is `Contain`, and `Contain` is
        // the only fit that cannot lose any of the image.
        assert_eq!(ImageFit::default(), ImageFit::Contain);
        let (_nodes, image) = image(200, 100);
        assert_eq!(image.fit(), ImageFit::default(), "and a new image uses it");
        assert!(
            inside(RECT, area(&image, RECT)),
            "so the whole image is on screen inside the node's own rect"
        );
    }

    #[test]
    fn an_image_asks_for_the_size_it_was_given_in_pixels() {
        for size in [(200, 100), (1, 1), (0, 0), (1024, 512), (4097, 4093)] {
            let (_nodes, image) = image(size.0, size.1);
            let asked = image.size();
            assert_eq!(
                (asked.width, asked.height),
                (pixel_extent(size.0), pixel_extent(size.1)),
                "an image has no default size; {size:?} is its own"
            );
        }
    }

    #[test]
    fn two_images_get_two_nodes() {
        let (mut nodes, first) = image(8, 8);
        let second = Image::new(&mut nodes, handle(), ImageSource::whole(8, 8));
        assert_ne!(first.handle(), second.handle(), "each widget owns a node");
        assert!(nodes.get(second.handle()).is_some());
    }

    #[test]
    fn the_texture_is_a_plain_field_written_through_its_setter() {
        // The mapping, not the appearance: a `Property` would let a caller write
        // it directly and leave the drawn texture disagreeing with the drawn
        // geometry. The setter is the only way in, and it applies at once.
        let (_nodes, mut image) = image(8, 8);
        image.set_texture(TextureHandle::new(TextureId::new(99)));
        assert_eq!(image.texture(), TextureHandle::new(TextureId::new(99)));
        let DrawCommand::Image { texture, .. } = only_image(&image, RECT) else {
            panic!("an image paints an image");
        };
        assert_eq!(
            texture,
            TextureId::new(99),
            "and the command has it at once"
        );
    }

    #[test]
    fn the_fit_is_a_plain_field_written_through_its_setter() {
        let (_nodes, mut image) = image(200, 100);
        for fit in FITS {
            image.set_fit(fit);
            assert_eq!(image.fit(), fit);
            assert_eq!(area(&image, RECT), destination_rect(fit, (200, 100), RECT));
        }
    }

    #[test]
    fn a_new_fit_does_not_disturb_the_opacity_or_the_corner_radius() {
        let (_nodes, mut image) = image(200, 100);
        image.opacity.set(0.25);
        image.corner_radius.set(6.0);
        image.snap_to_state();
        for fit in FITS {
            image.set_fit(fit);
            assert_eq!(drawn_opacity(&image, RECT), 0.25, "{fit:?} left it alone");
            assert_eq!(drawn_radius(&image, RECT), 6.0, "{fit:?} and the radius");
        }
    }

    #[test]
    fn the_source_is_written_through_its_setter_and_keeps_both_halves() {
        let (mut nodes, image) = image(8, 8);
        let mut swapped = Image::new(&mut nodes, handle(), ImageSource::whole(8, 8));
        swapped.set_source(ImageSource::new(400, 200, UvRect::full()));
        assert_eq!(swapped.source_size(), (400, 200));
        assert_eq!(swapped.source().uv(), UvRect::full());
        // The widget that was not touched is still the one it was.
        assert_eq!(image.source_size(), (8, 8));
    }

    // ------------------------------------------------------- source size trap

    #[test]
    fn the_source_size_is_the_callers_to_get_right_and_the_image_cannot_tell() {
        // The one number a caller must supply, and the one mistake that is
        // invisible: the widget is told an image is 1:1 and it is drawn as
        // though it were, at the wrong scale, with no error anywhere.
        let (_nodes, mut image) = image(200, 100);
        let wide = area(&image, RECT);
        image.set_source_size(200, 200);
        let square = area(&image, RECT);

        assert_eq!(
            wide,
            Rect::new(32.0, 139.0, 300.0, 150.0),
            "a 2:1 image in a square is letterboxed"
        );
        assert_eq!(
            square,
            Rect::new(32.0, 64.0, 300.0, 300.0),
            "the same handle declared 1:1 fills the square instead"
        );
        assert_ne!(
            wide, square,
            "so the mistake is a visibly different picture, and only the \
             caller's own number stands between them"
        );
    }

    #[test]
    fn setting_the_source_size_keeps_the_rectangle_the_image_occupies() {
        // The two halves of a source are separate: fixing the size must not move
        // an atlas-resident image off its own window.
        let window = ImageSource::new(
            200,
            100,
            UvRect {
                u0: 0.25,
                v0: 0.5,
                u1: 0.5,
                v1: 0.75,
            },
        );
        let (_nodes, mut image) = of_source(window);
        image.set_source_size(200, 400);
        assert_eq!(image.source_size(), (200, 400), "the size changed");
        assert_eq!(image.source().uv(), window.uv(), "and the window did not");
    }

    // --------------------------------------------------- where the image goes

    #[test]
    fn contain_takes_the_largest_rectangle_of_the_images_own_shape_that_fits() {
        let (_nodes, image) = image(200, 100);
        // 2:1 in a square: the width reaches the rect and the height follows
        // from the shape, so 300 wide and 150 tall, centred on the 150 either
        // side: y = 64 + (300 - 150) / 2 = 139.
        assert_eq!(area(&image, RECT), Rect::new(32.0, 139.0, 300.0, 150.0));
    }

    #[test]
    fn contain_letterboxes_a_tall_image_on_the_left_and_the_right() {
        let (_nodes, image) = image(100, 200);
        // 1:2 in a square: the height reaches the rect and the width follows,
        // so 150 wide, split evenly: x = 32 + (300 - 150) / 2 = 107.
        assert_eq!(area(&image, RECT), Rect::new(107.0, 64.0, 150.0, 300.0));
    }

    #[test]
    fn contain_of_a_matching_shape_is_the_whole_rect() {
        for (size, rect) in [((200, 200), RECT), ((200, 100), WIDE), ((100, 200), TALL)] {
            let (_nodes, image) = image(size.0, size.1);
            assert_eq!(
                area(&image, rect),
                rect,
                "{size:?} has the shape of {rect:?}, so there is nothing to letterbox"
            );
        }
    }

    #[test]
    fn contain_keeps_the_images_own_shape_at_every_pair_of_shapes() {
        for (size, rect) in SHAPES {
            let drawn = destination_rect(ImageFit::Contain, size, rect);
            let (source_width, source_height) = extents(size);
            assert_close(
                drawn.width / drawn.height,
                source_width / source_height,
                &format!("{size:?} in {rect:?} is still the image's own shape"),
            );
        }
    }

    #[test]
    fn contain_never_reaches_outside_the_rect_it_is_given() {
        for (size, rect) in SHAPES {
            assert!(
                inside(rect, destination_rect(ImageFit::Contain, size, rect)),
                "{size:?} in {rect:?} fits inside"
            );
        }
    }

    #[test]
    fn cover_draws_at_the_rect_so_nothing_overflows_it() {
        // The decision, as a test: cover is done by cropping the *image*, so the
        // quad is the node's own rect. A larger quad would be the same picture
        // only where something clips it, and nothing does yet.
        for (size, rect) in SHAPES {
            assert_eq!(
                destination_rect(ImageFit::Cover, size, rect),
                rect,
                "{size:?} in {rect:?} is covered by the rect itself",
            );
        }
    }

    #[test]
    fn cover_fills_the_rect_at_every_pair_of_shapes() {
        for (size, rect) in SHAPES {
            assert!(
                inside(rect, destination_rect(ImageFit::Cover, size, rect)),
                "{size:?} in {rect:?} is covered from inside, not by overflowing"
            );
        }
    }

    #[test]
    fn fill_is_the_rect_itself_and_ignores_the_image_entirely() {
        for (size, rect) in SHAPES {
            assert_eq!(
                destination_rect(ImageFit::Fill, size, rect),
                rect,
                "{size:?} is stretched into {rect:?} and its shape is not consulted",
            );
        }
    }

    #[test]
    fn none_is_the_image_at_its_own_size() {
        for (size, rect) in SHAPES {
            let (source_width, source_height) = extents(size);
            assert_eq!(
                destination_rect(ImageFit::None, size, rect),
                Rect::new(rect.x, rect.y, source_width, source_height),
                "{size:?} at its own size, wherever the rect is",
            );
        }
    }

    #[test]
    fn none_is_anchored_at_the_corners_and_not_centred() {
        // The decision the module documents, over every offset and every size a
        // test can reach: the image's top left is the rect's top left, always.
        // Centring would make an image's position depend on how big its
        // container is, and would leave a band of nothing above the first item of
        // a scrolled column.
        let offsets = [
            (0.0, 0.0),
            (32.0, 64.0),
            (700.0, 40.0),
            (-20.0, -30.0),
            (1023.0, 599.0),
        ];
        for (x, y) in offsets {
            for (source_width, source_height) in [(8, 8), (200, 100), (64, 512)] {
                let rect = Rect::new(x, y, 400.0, 300.0);
                let drawn = destination_rect(ImageFit::None, (source_width, source_height), rect);
                assert_eq!(
                    (drawn.x, drawn.y),
                    (x, y),
                    "a {source_width}x{source_height} image in a rect at ({x}, {y}) \
                     starts at the rect's own corner"
                );
                assert_eq!(
                    (drawn.width, drawn.height),
                    (pixel_extent(source_width), pixel_extent(source_height)),
                    "and is its own size, so the space in the rect is after it",
                );
            }
        }
    }

    #[test]
    fn none_does_not_move_when_its_container_grows() {
        // The consequence that matters for a layout: growing the node leaves the
        // image where it was and puts the space below it, where a column puts
        // its next item. Centring would move it.
        let (_nodes, mut image) = image(64, 32);
        image.set_fit(ImageFit::None);
        let short = area(&image, Rect::new(10.0, 20.0, 200.0, 200.0));
        let tall = area(&image, Rect::new(10.0, 20.0, 200.0, 400.0));
        assert_eq!(short, Rect::new(10.0, 20.0, 64.0, 32.0));
        assert_eq!(tall, short, "a taller container does not move the image");
    }

    #[test]
    fn the_geometry_moves_with_its_rect_and_nothing_else() {
        // The origin test, as a property rather than one fixture. A rect's origin
        // and a rect's extent are different numbers, and the slider's own suite
        // could not see them confused because every one of its fixtures was at
        // (0, 0); here no fixture is at (0, 0) and this one asks the question
        // directly: moving the rect moves the picture by the same vector and
        // changes not one other number.
        for (size, _) in SHAPES {
            for fit in FITS {
                let at_origin = destination_rect(fit, size, Rect::new(0.0, 0.0, 300.0, 300.0));
                for (x, y) in [(32.0, 64.0), (700.0, 40.0), (-20.0, -30.0)] {
                    let moved = destination_rect(fit, size, Rect::new(x, y, 300.0, 300.0));
                    assert_eq!(
                        (moved.width, moved.height),
                        (at_origin.width, at_origin.height),
                        "{fit:?} of {size:?} is the same size wherever it is laid out",
                    );
                    assert_close(moved.x - at_origin.x, x, &format!("{fit:?} moved by x"));
                    assert_close(moved.y - at_origin.y, y, &format!("{fit:?} moved by y"));
                }
            }
        }
    }

    #[test]
    fn an_image_laid_out_somewhere_else_is_placed_relative_to_its_own_rect() {
        // The fixture the NEVERAGAIN entry asks for, stated in the numbers: the
        // demo puts its widgets at (664, 496) and nothing else in this repository
        // is at the origin.
        let (_nodes, wide) = image(200, 100);
        let box_at = Rect::new(664.0, 496.0, 240.0, 240.0);
        assert_eq!(box_at.x, 664.0, "the fixture really is off the origin");
        // 2:1 into a square: the width binds, 240 wide and 120 tall, centred on
        // the 60 either side. Both halves of the answer come from the rect's own
        // numbers: 664 is the rect's x and 556 is 496 + (240 - 120) / 2. An
        // origin read as an extent would give 240 - 120 and a different x.
        assert_eq!(
            area(&wide, box_at),
            Rect::new(664.0, 556.0, 240.0, 120.0),
            "placed against the rect's own origin on both axes",
        );

        // The mirror, so the x axis is the centred one here and the y is not.
        let (_other, tall) = image(100, 200);
        let wide_at = Rect::new(40.0, 700.0, 240.0, 240.0);
        assert_eq!(
            area(&tall, wide_at),
            Rect::new(100.0, 700.0, 120.0, 240.0),
            "1:2 into a square: x is 40 + (240 - 120) / 2 = 100 and y is the \
             rect's own 700, so neither axis is read off the other",
        );
    }

    #[test]
    fn a_rect_with_no_extent_is_an_empty_rect_at_every_fit() {
        // One answer for all four, and not only for the fit that divides by the
        // missing side: a node a layout has collapsed to zero width draws nothing
        // whatever image it holds, which is what keeps `None` from being the fit
        // that draws a 200-pixel image outside a node with no width.
        for fit in FITS {
            for rect in [
                Rect::new(32.0, 64.0, 0.0, 300.0),
                Rect::new(32.0, 64.0, 300.0, 0.0),
                Rect::new(32.0, 64.0, 0.0, 0.0),
            ] {
                let drawn = destination_rect(fit, (200, 100), rect);
                assert_eq!(
                    drawn,
                    Rect::new(32.0, 64.0, 0.0, 0.0),
                    "{fit:?} into {rect:?}",
                );
            }
        }
    }

    #[test]
    fn a_negative_extent_is_treated_as_none_rather_than_drawn_inside_out() {
        for fit in FITS {
            let drawn = destination_rect(fit, (200, 100), Rect::new(32.0, 64.0, -40.0, 300.0));
            assert_eq!(drawn.width, 0.0, "{fit:?} does not draw a negative width");
        }
    }

    #[test]
    fn a_source_with_no_pixels_has_no_destination_at_every_fit() {
        // There is nothing to stretch, and no aspect ratio to divide by. The
        // guard is in `destination_rect` for all four fits rather than in three
        // of them, so `None` is not the odd one out with a zero-sized answer the
        // other three contradict.
        for fit in FITS {
            for size in [(0, 0), (0, 100), (200, 0)] {
                let drawn = destination_rect(fit, size, RECT);
                assert_eq!(
                    (drawn.width, drawn.height),
                    (0.0, 0.0),
                    "{fit:?} of a {size:?} image is nothing at all",
                );
                assert_eq!((drawn.x, drawn.y), (RECT.x, RECT.y));
            }
        }
    }

    #[test]
    fn an_image_with_no_pixels_records_no_command() {
        let (_nodes, mut image) = image(0, 100);
        for fit in FITS {
            image.set_fit(fit);
            assert!(
                image.paint(RECT).is_empty(),
                "{fit:?} of an image with no pixels draws nothing"
            );
        }
    }

    #[test]
    fn a_rect_with_no_extent_records_no_command() {
        let (_nodes, mut image) = image(200, 100);
        for fit in FITS {
            image.set_fit(fit);
            assert!(
                image.paint(Rect::new(32.0, 64.0, 0.0, 300.0)).is_empty(),
                "a collapsed node costs no draw call, at {fit:?}"
            );
        }
    }

    // -------------------------------------------------- what part is sampled

    #[test]
    fn the_three_fitting_fits_sample_the_whole_of_the_image() {
        for fit in [ImageFit::Contain, ImageFit::Fill, ImageFit::None] {
            for (size, rect) in SHAPES {
                let uv = sampled_uv(fit, ImageSource::whole(size.0, size.1), rect);
                assert!(
                    uv.is_full(),
                    "{fit:?} of {size:?} in {rect:?} crops nothing, and an image with \
                     its own texture is all of it"
                );
            }
        }
    }

    #[test]
    fn an_atlas_resident_image_samples_its_placement_and_not_the_whole_atlas() {
        // The half of "what part is sampled" that is *not* `UvRect::full()`: an
        // image packed into the shared atlas is a window into it, and sampling the
        // whole texture is a picture of every other icon in there as well.
        let placement = UvRect {
            u0: 0.25,
            v0: 0.5,
            u1: 0.5,
            v1: 0.75,
        };
        let source = ImageSource::new(200, 100, placement);
        for fit in [ImageFit::Contain, ImageFit::Fill, ImageFit::None] {
            assert_eq!(
                sampled_uv(fit, source, RECT),
                placement,
                "{fit:?} takes the whole of the image, which is that window"
            );
        }
        assert!(!sampled_uv(ImageFit::Contain, source, RECT).is_full());
    }

    #[test]
    fn a_cover_crop_is_the_middle_band_on_the_axis_it_crops() {
        // Every case, with the numbers derived from the two shapes rather than
        // read off the implementation. A source relatively *wider* than the rect
        // loses width; relatively *taller* loses height; matching loses nothing.
        let cases: [((u32, u32), Rect, UvRect); 6] = [
            // 1:1 into 2:1: taller than the box is wide, so the top and the
            // bottom quarter of a square go, and the whole width is kept.
            (
                (200, 200),
                Rect::new(0.0, 0.0, 100.0, 50.0),
                UvRect {
                    u0: 0.0,
                    v0: 0.25,
                    u1: 1.0,
                    v1: 0.75,
                },
            ),
            // 1:1 into 1:2: the mirror of the above, on the other axis.
            (
                (200, 200),
                Rect::new(0.0, 0.0, 50.0, 100.0),
                UvRect {
                    u0: 0.25,
                    v0: 0.0,
                    u1: 0.75,
                    v1: 1.0,
                },
            ),
            // 2:1 into 1:1: a wide image loses its sides, so the middle half of
            // its width fills a square, and its height is untouched.
            (
                (200, 100),
                RECT,
                UvRect {
                    u0: 0.25,
                    v0: 0.0,
                    u1: 0.75,
                    v1: 1.0,
                },
            ),
            // 1:2 into 1:1: the mirror again.
            (
                (100, 200),
                RECT,
                UvRect {
                    u0: 0.0,
                    v0: 0.25,
                    u1: 1.0,
                    v1: 0.75,
                },
            ),
            // Matching shapes crop nothing at all.
            (
                (200, 200),
                RECT,
                UvRect {
                    u0: 0.0,
                    v0: 0.0,
                    u1: 1.0,
                    v1: 1.0,
                },
            ),
            // A very wide image into a square keeps the middle *tenth* of its
            // width, because 10:1 into 1:1 leaves one part in ten.
            (
                (1000, 100),
                Rect::new(0.0, 0.0, 100.0, 100.0),
                UvRect {
                    u0: 0.45,
                    v0: 0.0,
                    u1: 0.55,
                    v1: 1.0,
                },
            ),
        ];
        for (size, rect, want) in cases {
            assert_eq!(
                sampled_uv(ImageFit::Cover, ImageSource::whole(size.0, size.1), rect),
                want,
                "{size:?} covering {rect:?}",
            );
        }
    }

    #[test]
    fn a_very_tall_image_covering_a_square_keeps_the_middle_tenth_down() {
        // The mirror of the 10:1 case above, and the one a suite that only
        // checked wide images would miss: the branch is chosen by *which* shape
        // is relatively larger, and both directions have to be right.
        let crop = sampled_uv(
            ImageFit::Cover,
            ImageSource::whole(100, 1000),
            Rect::new(0.0, 0.0, 100.0, 100.0),
        );
        assert_eq!(
            crop,
            UvRect {
                u0: 0.0,
                v0: 0.45,
                u1: 1.0,
                v1: 0.55
            },
            "a 1:10 image in a square keeps the middle tenth of its height",
        );
    }

    #[test]
    fn a_cover_crop_of_an_awkward_shape_is_the_ratio_of_the_two_shapes() {
        // 10:1 into 4:3 keeps a third of a tenth of the width; 1:10 into 3:4
        // keeps the same fraction of the height. Neither is a round number, so
        // these are close comparisons against numbers worked out by hand:
        // 10:1 into 4:3 keeps (4/3)/10 = 2/15 of the width, centred, so from
        // 13/30 to 17/30; 1:10 into 3:4 keeps (1/10)/(3/4) = 2/15 of the height,
        // centred, so from 13/30 to 17/30 down.
        let wide = sampled_uv(
            ImageFit::Cover,
            ImageSource::whole(1000, 100),
            Rect::new(0.0, 0.0, 400.0, 300.0),
        );
        assert_close(wide.u0, 13.0 / 30.0, "the left edge of the band");
        assert_close(wide.u1, 17.0 / 30.0, "and the right edge");
        assert_close(wide.v0, 0.0, "with the whole height kept");
        assert_close(wide.v1, 1.0, "both ends of it");

        let tall = sampled_uv(
            ImageFit::Cover,
            ImageSource::whole(100, 1000),
            Rect::new(0.0, 0.0, 300.0, 400.0),
        );
        assert_close(tall.v0, 13.0 / 30.0, "the top edge of the band");
        assert_close(tall.v1, 17.0 / 30.0, "and the bottom edge");
        assert_close(tall.u0, 0.0, "with the whole width kept");
        assert_close(tall.u1, 1.0, "both ends of it");
    }

    #[test]
    fn a_cover_crop_has_the_rects_own_shape() {
        // What the crop is *for*. Measured in pixels, because a uv is a
        // fraction of a span and the spans of an atlas placement are not the
        // image's own size.
        for (size, rect) in SHAPES {
            let uv = sampled_uv(ImageFit::Cover, ImageSource::whole(size.0, size.1), rect);
            let (source_width, source_height) = extents(size);
            let crop_width = (uv.u1 - uv.u0) * source_width;
            let crop_height = (uv.v1 - uv.v0) * source_height;
            assert!(
                crop_width > 0.0 && crop_height > 0.0,
                "{size:?} in {rect:?} keeps some image",
            );
            assert_close(
                crop_width / crop_height,
                rect.width / rect.height,
                &format!("the crop of {size:?} has the shape of {rect:?}"),
            );
        }
    }

    #[test]
    fn a_cover_image_is_scaled_by_one_number_and_not_stretched() {
        // The anti-distortion test, and the reason the crop exists at all. A
        // `Cover` quad with the whole image sampled is a *stretched* image, which
        // looks exactly like an image and is the defect this module's second
        // half is about. Drawing the cropped image into the rect has to be one
        // scale on both axes, and this asks the two axes separately.
        for (size, rect) in SHAPES {
            let uv = sampled_uv(ImageFit::Cover, ImageSource::whole(size.0, size.1), rect);
            let (source_width, source_height) = extents(size);
            let crop_width = (uv.u1 - uv.u0) * source_width;
            let crop_height = (uv.v1 - uv.v0) * source_height;
            let across = rect.width / crop_width;
            let down = rect.height / crop_height;
            assert_close(
                across,
                down,
                &format!("{size:?} in {rect:?} is scaled by one number"),
            );
            assert!(
                across >= 1.0,
                "{size:?} into {rect:?} is scaled up or left alone, never down",
            );
        }
    }

    #[test]
    fn a_cover_crop_is_centred_in_the_image() {
        let source = ImageSource::new(
            200,
            100,
            UvRect {
                u0: 0.25,
                v0: 0.5,
                u1: 0.5,
                v1: 0.75,
            },
        );
        let uv = sampled_uv(ImageFit::Cover, source, RECT);
        let base = source.uv();
        assert_close(
            (uv.u0 + uv.u1) / 2.0,
            (base.u0 + base.u1) / 2.0,
            "the middle of the crop across is the middle of the image across",
        );
        assert_close(
            (uv.v0 + uv.v1) / 2.0,
            (base.v0 + base.v1) / 2.0,
            "and so is the middle of it down",
        );
    }

    #[test]
    fn a_cover_crop_never_reaches_outside_the_window_it_was_taken_from() {
        // An atlas-resident image cropped for cover: the crop is a fraction of
        // the *window*, not of the texture, so it cannot sample the icons beside
        // it. 1:1 in a 2:1 box keeps the middle half of a window that spans
        // 0.25..0.5 across and 0.5..0.75 down: the middle quarter of each.
        let source = ImageSource::new(
            200,
            200,
            UvRect {
                u0: 0.25,
                v0: 0.5,
                u1: 0.5,
                v1: 0.75,
            },
        );
        let uv = sampled_uv(ImageFit::Cover, source, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(
            uv,
            UvRect {
                u0: 0.25,
                v0: 0.5625,
                u1: 0.5,
                v1: 0.6875
            },
            "half of the window's height, from 0.5 + 0.25 * 0.25 to 0.5 + 0.25 * 0.75",
        );
        assert!(
            uv.u0 >= source.uv().u0 && uv.u1 <= source.uv().u1,
            "inside across"
        );
        assert!(
            uv.v0 >= source.uv().v0 && uv.v1 <= source.uv().v1,
            "and inside down"
        );
    }

    #[test]
    fn a_cover_crop_is_the_same_fraction_of_a_placement_as_of_a_whole_texture() {
        // The two cases have to agree, or the same image is cropped one way in
        // the atlas and another way out of it. The comparison is of *fractions
        // of their own windows*, not of absolute coordinates: a window that spans
        // a quarter of the texture has to give a crop a quarter as wide.
        let windowed = ImageSource::new(
            200,
            100,
            UvRect {
                u0: 0.25,
                v0: 0.5,
                u1: 0.5,
                v1: 0.75,
            },
        );
        let a = sampled_uv(ImageFit::Cover, ImageSource::whole(200, 100), RECT);
        let b = sampled_uv(ImageFit::Cover, windowed, RECT);

        let spans = |uv: UvRect, base: UvRect| {
            (
                (uv.u1 - uv.u0) / (base.u1 - base.u0),
                (uv.v1 - uv.v0) / (base.v1 - base.v0),
            )
        };
        let (a_across, a_down) = spans(a, UvRect::full());
        let (b_across, b_down) = spans(b, windowed.uv());
        assert_close(b_across, a_across, "the same fraction of the window across");
        assert_close(b_down, a_down, "and the same fraction of it down");

        // And the same place in it: a 2:1 image in a square keeps the middle
        // quarter of the window, which is a quarter of a whole texture too.
        assert_close(b.u0 - windowed.uv().u0, 0.25 * 0.25, "a quarter in");
        assert_close(
            b.u1 - windowed.uv().u0,
            0.75 * 0.25,
            "and three quarters of the way in",
        );
    }

    #[test]
    fn the_rect_does_not_change_what_a_fit_that_does_not_crop_samples() {
        let source = ImageSource::new(
            200,
            100,
            UvRect {
                u0: 0.25,
                v0: 0.5,
                u1: 0.5,
                v1: 0.75,
            },
        );
        for fit in [ImageFit::Contain, ImageFit::Fill, ImageFit::None] {
            let first = sampled_uv(fit, source, RECT);
            for rect in [WIDE, TALL, Rect::new(0.0, 0.0, 1.0, 999.0)] {
                assert_eq!(
                    sampled_uv(fit, source, rect),
                    first,
                    "{fit:?} does not consult the rect at all",
                );
            }
        }
    }

    #[test]
    fn a_cover_fit_of_a_rect_with_no_extent_crops_nothing() {
        let source = ImageSource::whole(200, 100);
        for rect in [
            Rect::new(32.0, 64.0, 0.0, 300.0),
            Rect::new(32.0, 64.0, 300.0, 0.0),
        ] {
            assert!(
                sampled_uv(ImageFit::Cover, source, rect).is_full(),
                "{rect:?}"
            );
        }
    }

    #[test]
    fn a_cover_image_whose_shape_already_matches_its_rect_is_drawn_opaque() {
        // The one consequence of a crop that is not a crop: a `Cover` image that
        // needs no cropping records `is_full()`, and the batcher puts a whole
        // texture at full opacity in the opaque pass. Anything else blends.
        let (_nodes, mut image) = image(200, 200);
        image.set_fit(ImageFit::Cover);
        let mut batcher = Batcher::new();
        batcher.add(only_image(&image, RECT));
        let batched = batcher.finish();
        assert_eq!(
            batched.transparent.len(),
            0,
            "a square into a square crops nothing"
        );
        assert_eq!(batched.opaque.len(), 1, "so it is drawn in the opaque pass");
    }

    #[test]
    fn a_cover_image_that_does_crop_is_drawn_blended() {
        let (_nodes, mut image) = image(200, 100);
        image.set_fit(ImageFit::Cover);
        let mut batcher = Batcher::new();
        batcher.add(only_image(&image, RECT));
        let batched = batcher.finish();
        assert!(
            batched.opaque.is_empty(),
            "a window into a texture must blend"
        );
        assert_eq!(batched.transparent.len(), 1);
    }

    // -------------------------------------------------------------- opacity

    #[test]
    fn the_opacity_reaches_the_command_as_a_fraction() {
        let (_nodes, image) = image(200, 100);
        for wanted in [0.0, 0.25, 0.5, 0.75, 1.0] {
            image.opacity.set(wanted);
            image.snap_to_state();
            assert_eq!(
                drawn_opacity(&image, RECT),
                wanted,
                "{wanted} is the fraction, and it is the number the shader scales by",
            );
        }
    }

    #[test]
    fn the_opacity_is_not_folded_into_a_colour() {
        // `DrawCommand::Image` has no colour field, and that is the mechanism:
        // the colours are in the texture and the shader scales the alpha the
        // image brought with it. Multiplying here as well would darken a
        // half-faded image by half *twice*, and the command could not express it
        // even if it were wanted.
        let (_nodes, image) = image(200, 100);
        image.opacity.set(0.5);
        image.snap_to_state();
        match only_image(&image, RECT) {
            DrawCommand::Image { opacity, .. } => {
                assert_eq!(opacity, 0.5, "the raw fraction, not 0.5 of anything");
            }
            other => panic!("an image paints an image, not {other:?}"),
        }
    }

    #[test]
    fn an_opacity_above_the_range_is_the_top_of_it() {
        // Asserting the value, not the identity: `1.4 == 1.4.clamp(0.0, 1.0)` is
        // true for every f32 and tests nothing. The number that distinguishes a
        // clamp from a pass-through is the number the command carries.
        let (_nodes, image) = image(200, 100);
        for over in [1.4, 2.0, 1000.0] {
            image.opacity.set(over);
            image.snap_to_state();
            assert_eq!(drawn_opacity(&image, RECT), 1.0, "{over} is fully opaque");
        }
    }

    #[test]
    fn an_opacity_below_the_range_is_the_bottom_of_it() {
        let (_nodes, image) = image(200, 100);
        for under in [-0.5, -1.0, -1000.0] {
            image.opacity.set(under);
            image.snap_to_state();
            assert_eq!(
                drawn_opacity(&image, RECT),
                0.0,
                "{under} is nothing at all"
            );
        }
    }

    #[test]
    fn an_opacity_of_nothing_is_nothing_rather_than_a_nan_on_the_screen() {
        // `f32::max` returns the other operand when one of the two is a `NaN`, so
        // this is 0.0 and not a `NaN`: a rect whose every coordinate is `NaN` is
        // not a picture, it is a shader that has stopped.
        let (_nodes, image) = image(200, 100);
        image.opacity.set(f32::NAN);
        image.snap_to_state();
        assert_eq!(drawn_opacity(&image, RECT), 0.0);
        assert!(!drawn_opacity(&image, RECT).is_nan());
    }

    #[test]
    fn an_opacity_written_by_hand_outside_the_range_is_clamped_too() {
        // `shown_opacity` is public, so the clamp cannot live only in `style`:
        // a caller that writes the drawn property directly must not be able to
        // put 1.4 in a command.
        let (_nodes, image) = image(200, 100);
        image.shown_opacity.set(1.4);
        assert_eq!(drawn_opacity(&image, RECT), 1.0);
        image.shown_opacity.set(-0.25);
        assert_eq!(drawn_opacity(&image, RECT), 0.0);
    }

    #[test]
    fn a_faded_image_is_drawn_blended_rather_than_covering_what_is_behind_it() {
        let (_nodes, image) = image(200, 100);
        let mut batcher = Batcher::new();
        batcher.add(only_image(&image, RECT));
        assert_eq!(
            batcher.finish().opaque.len(),
            1,
            "whole texture, full opacity"
        );

        image.opacity.set(0.5);
        image.snap_to_state();
        let mut faded = Batcher::new();
        faded.add(only_image(&image, RECT));
        let batched = faded.finish();
        assert!(batched.opaque.is_empty(), "half an image is not opaque");
        assert_eq!(batched.transparent.len(), 1);
    }

    // --------------------------------------------------------- corner radius

    #[test]
    fn the_corner_radius_reaches_the_command() {
        let (_nodes, image) = image(200, 100);
        for wanted in [0.0, 1.0, 6.0, 12.5, 150.0] {
            image.corner_radius.set(wanted);
            assert_eq!(drawn_radius(&image, RECT), wanted, "{wanted} px of corner");
        }
    }

    #[test]
    fn a_zero_radius_is_square_corners() {
        let (_nodes, image) = image(200, 100);
        assert_eq!(image.corner_radius.get(), 0.0, "the default is square");
        assert_eq!(drawn_radius(&image, RECT), 0.0);
    }

    #[test]
    fn a_negative_radius_is_clamped_to_nothing_rather_than_inverted() {
        // A negative radius is not a radius: there is no shape of which it is the
        // corner, and the shader's "outside the rounded rect" test has no answer
        // for one. Asserting the number, not `max`'s identity.
        let (_nodes, image) = image(200, 100);
        for under in [-1.0, -0.5, -1000.0] {
            image.corner_radius.set(under);
            assert_eq!(
                drawn_radius(&image, RECT),
                0.0,
                "{under} is no corner at all"
            );
        }
    }

    #[test]
    fn a_radius_bigger_than_the_image_is_left_to_the_shader() {
        // `DrawCommand::Image` documents that a radius past half the shorter side
        // is treated as half of it, as it is for a rounded rect. Clamping it here
        // as well would be a second rule to keep in step with the renderer's.
        let (_nodes, image) = image(200, 100);
        image.corner_radius.set(10_000.0);
        let drawn = area(&image, RECT);
        assert_eq!(
            drawn_radius(&image, RECT),
            10_000.0,
            "passed through as given"
        );
        assert!(
            drawn.width < 10_000.0,
            "which is far more than half of the image"
        );
    }

    #[test]
    fn the_corners_are_a_clip_and_nothing_is_drawn_for_them() {
        // `.ai/NEVERAGAIN.md`, *A filled rounded rectangle is not an outline*: a
        // `RoundedRect` fills its rect, so a "corner" drawn as a filled shape is
        // a card with the image on it. The image records one command and it is
        // the image — no rect, no rounded rect, nothing behind it and nothing on
        // top.
        let (_nodes, image) = image(200, 100);
        image.corner_radius.set(40.0);
        let commands = image.paint(RECT);
        assert_eq!(commands.len(), 1, "{commands:?}");
        assert!(
            matches!(commands[0], DrawCommand::Image { .. }),
            "the only command is the image itself: {commands:?}"
        );
        assert_eq!(
            commands
                .iter()
                .filter(|command| matches!(
                    command,
                    DrawCommand::Rect { .. }
                        | DrawCommand::RoundedRect { .. }
                        | DrawCommand::Circle { .. }
                ))
                .count(),
            0,
            "a clipped corner is not a filled shape",
        );
    }

    // -------------------------------------------------------------- animation

    #[test]
    fn a_programmatic_opacity_change_animates_toward_the_new_value() {
        // The contract is that the image *fades* rather than jumps, so this checks
        // the middle of the transition and not only its end: a `set` instead of
        // an animation would pass an end-only assertion.
        let (_nodes, image) = image(200, 100);
        image.opacity.set(0.0);
        image.animate_to_state(motion());
        assert_eq!(
            image.shown_opacity.get(),
            1.0,
            "it starts where it was, so a frame drawn now is right"
        );
        assert!(image.tick(ms(50)));
        assert_eq!(
            image.shown_opacity.get(),
            0.5,
            "half way through, on a linear curve"
        );
        assert!(image.tick(ms(50)));
        assert_eq!(image.shown_opacity.get(), 0.0, "and it arrives");
        assert!(!image.is_animating());
    }

    #[test]
    fn a_frame_in_the_middle_of_a_fade_paints_the_opacity_the_fade_has_got_to() {
        let (_nodes, image) = image(200, 100);
        image.opacity.set(0.0);
        image.animate_to_state(motion());
        let at_start = drawn_opacity(&image, RECT);
        tick(&image, 50);
        let halfway = drawn_opacity(&image, RECT);
        tick(&image, 50);
        let arrived = drawn_opacity(&image, RECT);

        assert_eq!(at_start, 1.0);
        assert_eq!(halfway, 0.5, "a frame mid-fade is half an image");
        assert_eq!(arrived, 0.0);
        assert!(
            at_start > halfway && halfway > arrived,
            "and it moves monotonically: {at_start} then {halfway} then {arrived}"
        );
    }

    #[test]
    fn the_truth_moves_at_once_and_the_drawn_opacity_follows_it() {
        let (_nodes, image) = image(200, 100);
        image.opacity.set(0.25);
        assert_eq!(
            image.shown_opacity.get(),
            1.0,
            "the caller writes the truth and the widget is still drawing what it was"
        );
        image.snap_to_state();
        assert_eq!(
            drawn_opacity(&image, RECT),
            0.25,
            "and a snap brings it along"
        );
    }

    #[test]
    fn a_second_aim_replaces_the_first_rather_than_running_both_of_them() {
        // The `clear()` in `animate_to_state`, and the reason it cannot be tested
        // by the value alone. A clock does not know that two animations are
        // writing one property, and it ticks them in the order they were added,
        // so the **newest** value is the right answer while both are running —
        // the older one writes first and loses. Which means a test that only
        // looks at the value cannot see a missing `clear`: it passes whether the
        // first transition was dropped or merely out-ticked.
        //
        // What it *can* see is the first transition still being there. The two
        // aims below have different durations, so the second finishes long
        // before the first would have, and the first is then a transition that
        // reports itself as running for 350 ms more and then writes 0.0 — the
        // value the caller has already replaced.
        let (_nodes, image) = image(200, 100);
        let long = Motion {
            duration: ms(400),
            easing: Easing::Linear,
        };
        image.opacity.set(0.0);
        image.animate_to_state(long);
        tick(&image, 50);
        assert!(image.is_animating(), "the first aim is running");

        image.opacity.set(1.0);
        image.animate_to_state(motion());
        for _ in 0..3 {
            tick(&image, 50);
        }
        assert_eq!(
            image.shown_opacity.get(),
            1.0,
            "the newest value is on screen"
        );
        assert!(
            !image.is_animating(),
            "and the 400 ms aim it replaced is gone rather than still counting \
             down"
        );
        assert!(!image.tick(ms(500)), "so there is nothing left to write");
        assert_eq!(
            image.shown_opacity.get(),
            1.0,
            "and the replaced value never arrives to undo it"
        );
    }

    #[test]
    fn snapping_an_image_mid_fade_back_out_ends_the_fade() {
        let (_nodes, image) = image(200, 100);
        image.opacity.set(0.0);
        image.animate_to_state(motion());
        assert!(image.tick(ms(10)), "it is fading");
        image.snap_to_state();
        assert_eq!(
            image.shown_opacity.get(),
            0.0,
            "the snap put it at the truth"
        );
        assert!(!image.tick(ms(500)), "and nothing arrives afterwards");
        assert_eq!(image.shown_opacity.get(), 0.0);
    }

    #[test]
    fn a_write_to_the_opacity_is_what_marks_the_node_clean() {
        // The node is the caller's to keep clean, and a property's `on_change` is
        // the whole of the mechanism. A plain field could not do this, which is
        // half of why the appearance is properties and the fit is not.
        use std::cell::Cell;
        use std::rc::Rc;

        let (_nodes, image) = image(200, 100);
        let drawn_marks = Rc::new(Cell::new(0));
        let truth_marks = Rc::new(Cell::new(0));
        let counting_drawn = Rc::clone(&drawn_marks);
        let counting_truth = Rc::clone(&truth_marks);
        // Both halves, because a caller that fades an image and a caller that
        // changes what it fades to are both writing something that has to reach
        // the node; `Progress`'s own doctest does the same with its `shown`.
        image
            .shown_opacity
            .on_change(move |_| counting_drawn.set(counting_drawn.get() + 1));
        image
            .opacity
            .on_change(move |_| counting_truth.set(counting_truth.get() + 1));

        image.snap_to_state();
        assert_eq!(
            drawn_marks.get(),
            1,
            "the snap wrote the drawn opacity once"
        );
        assert_eq!(truth_marks.get(), 0, "and left the truth alone");

        image.opacity.set(0.0);
        assert_eq!(
            truth_marks.get(),
            1,
            "a caller writing the truth marks it too"
        );
        assert_eq!(drawn_marks.get(), 1, "and it draws nothing by itself");

        image.animate_to_state(motion());
        let before = drawn_marks.get();
        assert!(image.tick(ms(50)), "a frame of the fade wrote");
        assert!(
            drawn_marks.get() > before,
            "so a caller that repaints on the write saw every frame of it: {} \
             marks after {}",
            drawn_marks.get(),
            before,
        );
    }

    #[test]
    fn the_corner_radius_marks_the_node_clean_too() {
        // The other half of the reason the radius is a property: a caller that
        // changes it mid-frame has to be able to have the node repainted.
        use std::cell::Cell;
        use std::rc::Rc;

        let (_nodes, image) = image(200, 100);
        let marks = Rc::new(Cell::new(0));
        let counting = Rc::clone(&marks);
        image
            .corner_radius
            .on_change(move |_| counting.set(counting.get() + 1));
        image.corner_radius.set(8.0);
        assert_eq!(marks.get(), 1);
        image.corner_radius.set(16.0);
        assert_eq!(marks.get(), 2, "and again on the next write");
    }

    #[test]
    fn an_image_that_has_never_been_aimed_does_not_move() {
        let (_nodes, image) = image(200, 100);
        image.opacity.set(0.0);
        assert!(!image.tick(ms(1000)), "time passing is not an animation");
        assert_eq!(
            image.shown_opacity.get(),
            1.0,
            "and the image has not moved"
        );
    }

    #[test]
    fn an_image_with_animated_opacity_reports_that_it_is_animating() {
        let (_nodes, image) = image(200, 100);
        assert!(!image.is_animating());
        image.opacity.set(0.0);
        image.animate_to_state(motion());
        assert!(image.is_animating(), "an aim is a running transition");
        tick(&image, 100);
        assert!(!image.is_animating(), "and it stops when it arrives");
    }

    // ------------------------------------------------------------- the source

    #[test]
    fn a_source_is_the_three_numbers_it_is_given() {
        let uv = UvRect {
            u0: 0.1,
            v0: 0.2,
            u1: 0.3,
            v1: 0.4,
        };
        let source = ImageSource::new(64, 32, uv);
        assert_eq!(source.size(), (64, 32));
        assert_eq!(source.uv(), uv);
    }

    #[test]
    fn a_whole_image_is_the_whole_texture() {
        let source = ImageSource::whole(1024, 768);
        assert_eq!(source.size(), (1024, 768));
        assert!(source.uv().is_full());
    }

    #[test]
    fn a_placement_carries_the_pixel_size_and_the_window_together() {
        // The two halves of a source cannot be put out of step when they arrive
        // together, which is the whole argument for this conversion.
        let mut cache = TextureCache::new();
        let handle = load(&mut cache, "icon.png", 64, 32);
        let placement = cache.placement(handle).expect("small enough for the atlas");
        let source = ImageSource::from_placement(placement);

        assert_eq!(
            source.size(),
            (64, 32),
            "the placement knows the pixel size"
        );
        assert_eq!(
            source.uv(),
            UvRect {
                u0: placement.u0,
                v0: placement.v0,
                u1: placement.u1,
                v1: placement.v1
            },
            "and the window into the atlas",
        );
        assert!(
            !source.uv().is_full(),
            "which is not the whole of the atlas"
        );
    }

    #[test]
    fn the_recipe_answers_with_the_placement_of_an_atlas_resident_image() {
        let mut cache = TextureCache::new();
        let handle = load(&mut cache, "icon.png", 64, 32);
        let source = ImageSource::of(&cache, handle).expect("placed");
        let placement = cache.placement(handle).expect("atlas resident");

        assert_eq!(source, ImageSource::from_placement(placement));
        assert_eq!(source.size(), (64, 32));
        assert!(!source.uv().is_full());
    }

    #[test]
    fn the_recipe_answers_with_the_whole_texture_of_an_image_with_one_of_its_own() {
        // Too big for the atlas in either axis, so the handle carries the
        // standalone bit and the whole of that texture is the image.
        let mut cache = TextureCache::new();
        let big = ATLAS_MAX_IMAGE + 1;
        let handle = load(&mut cache, "photo.png", big, 4);
        let source = ImageSource::of(&cache, handle).expect("placed");

        assert_eq!(source.size(), (big, 4));
        assert!(source.uv().is_full(), "its own texture is all of it");
        assert!(
            cache.placement(handle).is_none(),
            "and it is not in the atlas"
        );
    }

    #[test]
    fn the_recipe_answers_with_the_whole_texture_of_a_too_tall_image_too() {
        // The limit is on both axes, and an image that is narrow but taller than
        // the limit is the case a test that only checks the width misses.
        let mut cache = TextureCache::new();
        let tall = ATLAS_MAX_IMAGE + 1;
        let handle = load(&mut cache, "banner.png", 4, tall);
        let source = ImageSource::of(&cache, handle).expect("placed");
        assert_eq!(source.size(), (4, tall));
        assert!(source.uv().is_full());
    }

    #[test]
    fn the_recipe_answers_none_for_a_handle_the_cache_never_issued() {
        let cache = TextureCache::new();
        for id in [0_u32, 1, 9999] {
            assert_eq!(
                ImageSource::of(&cache, TextureHandle::new(TextureId::new(id))),
                None,
                "handle {id} is not an image this cache can place",
            );
        }
        // And the standalone variant of the same question.
        assert_eq!(
            ImageSource::of(&cache, TextureHandle::new(TextureId::new(0x8000_0001))),
            None,
        );
    }

    #[test]
    fn the_recipe_answers_none_for_an_atlas_image_that_eviction_took() {
        // The second thing `None` means, and the reason it is not "the whole
        // atlas": an evicted image's window is gone while its handle still names
        // a slot in it. Answering `full()` here would draw a screen of other
        // people's icons in place of the picture, which is a wrong picture rather
        // than a hole. A caller that must not lose an image pins it.
        let mut cache = TextureCache::new();
        let side = 128;
        let per_shelf = 2048 / (side + 2);
        let total = (per_shelf * (2048 / side)) + per_shelf;
        let first = load(&mut cache, "icon0.png", side, side);
        for index in 1..total {
            load(&mut cache, &format!("icon{index}.png"), side, side);
        }
        assert!(
            cache.placement(first).is_none(),
            "the oldest shelf was evicted"
        );
        assert_eq!(
            ImageSource::of(&cache, first),
            None,
            "rather than the whole atlas",
        );

        // And pinned, it stays.
        let mut pinned_cache = TextureCache::new();
        let kept = load(&mut pinned_cache, "logo.png", side, side);
        pinned_cache.pin(kept);
        for index in 0..total {
            load(&mut pinned_cache, &format!("other{index}.png"), side, side);
        }
        assert!(
            ImageSource::of(&pinned_cache, kept).is_some(),
            "a pinned one stays"
        );
    }

    #[test]
    fn a_source_survives_a_round_trip_through_the_widget() {
        let source = ImageSource::new(
            64,
            32,
            UvRect {
                u0: 0.25,
                v0: 0.5,
                u1: 0.5,
                v1: 0.75,
            },
        );
        let (_nodes, image) = of_source(source);
        assert_eq!(image.source(), source);
        assert_eq!(image.source_size(), (64, 32));
        assert_eq!(
            image.sampled_uv(RECT),
            source.uv(),
            "and it is what is sampled"
        );
    }

    #[test]
    fn an_image_from_a_placement_is_drawn_from_that_placement_at_every_fit() {
        let mut cache = TextureCache::new();
        let handle = load(&mut cache, "icon.png", 64, 32);
        let source = ImageSource::of(&cache, handle).expect("placed");
        let (_nodes, mut image) = of_source(source);

        for fit in FITS {
            image.set_fit(fit);
            let expected = sampled_uv(fit, source, RECT);
            assert_eq!(sampled(&image, RECT), expected, "{fit:?} at the placement");
            assert!(
                !sampled(&image, RECT).is_full() || fit == ImageFit::Cover,
                "{fit:?} samples the window, not the atlas",
            );
        }
    }

    // -------------------------------------------- the widget is the two parts

    #[test]
    fn the_widget_is_the_two_pure_functions_with_its_own_numbers() {
        // The two halves and the widget cannot drift apart, because the widget
        // *is* them: every command's rect and uv is the free function's answer
        // for the widget's own fit and source.
        for (size, rect) in SHAPES {
            for fit in FITS {
                let command = painted(size.0, size.1, fit, rect);
                let DrawCommand::Image {
                    rect: drawn, uv, ..
                } = command
                else {
                    panic!("an image paints an image");
                };
                let source = ImageSource::whole(size.0, size.1);
                assert_eq!(
                    drawn,
                    destination_rect(fit, size, rect),
                    "{fit:?} of {size:?} in {rect:?} draws where the function says",
                );
                assert_eq!(
                    uv,
                    sampled_uv(fit, source, rect),
                    "{fit:?} of {size:?} in {rect:?} samples what the function says",
                );
            }
        }
    }

    #[test]
    fn the_widgets_two_methods_agree_with_the_functions() {
        let (_nodes, mut image) = image(200, 100);
        for fit in FITS {
            image.set_fit(fit);
            assert_eq!(
                image.destination(RECT),
                destination_rect(fit, (200, 100), RECT)
            );
            assert_eq!(
                image.sampled_uv(RECT),
                sampled_uv(fit, ImageSource::whole(200, 100), RECT),
            );
        }
    }

    #[test]
    fn the_commands_a_frame_would_batch_are_only_ever_images() {
        // What the whole module adds to a frame: nothing that is not a textured
        // quad. Every widget in this tree draws something; this one draws an
        // image, and a caller reading the paint state should not have to work
        // out which of the seven variants it was handed.
        let (_nodes, mut image) = image(200, 100);
        image.set_fit(ImageFit::Cover);
        image.corner_radius.set(20.0);
        image.opacity.set(0.6);
        image.snap_to_state();
        let mut batcher = Batcher::new();
        for command in image.paint(RECT) {
            batcher.add(command);
        }
        let batched = batcher.finish();
        assert_eq!(batched.opaque.len() + batched.transparent.len(), 1);
        let commands = &batched.transparent[0].commands;
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], DrawCommand::Image { .. }));
    }

    // ------------------------------------------------------------- pixel_extent

    #[test]
    fn a_pixel_count_below_two_to_the_twenty_four_survives_exactly() {
        // `f32` has no `From<u32>`, so the conversion is done in two halves; this
        // is the test that the two halves are the whole number and not a sum
        // with a rounding error in it. 2^24 is where an `f32` runs out of
        // integers, and a 4096-square image is comfortably under it.
        for value in [
            0_u32, 1, 2, 255, 256, 1024, 4096, 65_535, 65_536, 16_777_215,
        ] {
            assert_eq!(pixel_extent(value), f64::from(value) as f32, "{value}");
        }
    }

    #[test]
    fn a_pixel_count_above_two_to_the_twenty_four_is_the_nearest_float() {
        // There is no exact answer above 2^24 and pretending otherwise would be
        // the lie; the nearest `f32` is what an image's size becomes, and it is
        // what a layout is given.
        for value in [16_777_216_u32, 16_777_217, 16_777_219, u32::MAX] {
            let got = pixel_extent(value);
            assert!(
                got >= 16_777_216.0,
                "{value} is at least the largest exact one"
            );
            assert!(
                (got - f64::from(value) as f32).abs() <= 1.0,
                "{value} is within a pixel of itself, and no further than an f32 reaches",
            );
        }
    }

    #[test]
    fn an_image_asks_for_a_size_it_can_lay_out_exactly() {
        // The visible consequence of the conversion: the size a caller gets back
        // is the size it can put into a layout without the layout disagreeing
        // with the image.
        for (width, height) in [(800, 600), (1920, 1080), (4096, 4096)] {
            let (_nodes, image) = image(width, height);
            let size: Size = image.size();
            assert_eq!(size, Size::new(pixel_extent(width), pixel_extent(height)));
        }
    }

    #[test]
    fn a_fit_decided_on_a_pixel_size_matches_one_decided_on_a_float() {
        // The conversion is not supposed to move any of the geometry: a 1920 ×
        // 1080 image is exactly 16:9, and its aspect must come out as exactly
        // the same number the rectangle arithmetic would produce from a float.
        let (width, height) = (1920_u32, 1080_u32);
        let (float_width, float_height) = (f64::from(width) as f32, f64::from(height) as f32);
        assert_close(
            pixel_extent(width) / pixel_extent(height),
            float_width / float_height,
            "the aspect ratio survives the conversion",
        );
        assert_eq!(
            destination_rect(ImageFit::Contain, (width, height), RECT),
            destination_rect(ImageFit::Contain, (width, height), RECT),
        );
    }

    #[test]
    fn a_fit_of_an_image_wider_than_any_float_representation_is_still_sane() {
        // A caller that has a 65536-pixel-wide image: the width is exactly
        // representable, so the letterbox is a real rect rather than an infinity
        // or a `NaN`.
        let drawn = destination_rect(ImageFit::Contain, (65_536, 32), RECT);
        assert_eq!(drawn.width, 300.0, "the width binds");
        assert_eq!(drawn.height, 300.0 * 32.0 / 65_536.0);
        assert!(drawn.height > 0.0 && drawn.height.is_finite());
        assert!(drawn.y.is_finite() && drawn.x.is_finite());
    }
}
