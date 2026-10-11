//! The Icon widget: a tinted image drawn at one fixed size.
//!
//! An icon is a node, one tinted quad, and the numbers that decide where. It
//! is an [`Image`](crate::widgets::image::Image) in one thing only — it
//! records a [`DrawCommand::Image`] — and
//! it is not an image in everything else: it has no fit, no corner radius, no
//! clock and no transition, and its colour is a tint the shader multiplies
//! rather than an opacity the pipeline blends.
//!
//! # What an icon is not
//!
//! - **Not a vector renderer.** There is no SVG parser, no curve tessellation
//!   and no runtime rasterisation here: the bytes are baked offline (a white
//!   master PNG at the exact on-screen size) and this widget draws the
//!   texture it is given. Vector rendering stays an offline bake.
//! - **Not a button.** It answers no event, carries no `selected`, no `state`
//!   and no callback. A caller driving an icon from a tap writes
//!   [`tint`](Icon::tint) or swaps the texture, the same way an image is
//!   driven from outside.
//! - **Not an image.** [`Image::paint`](crate::widgets::image::Image::paint)
//!   records the neutral tint and this widget records its own: sharing the
//!   method would make one of the two draw the other's colour.
//!
//! # The tint rides per vertex, not in the batch key
//!
//! [`DrawCommand::Image`]'s `tint` is a
//! premultiplied modulate the image shader multiplies per fragment, and it is
//! deliberately **not** part of [`BatchKey`](crate::batch::BatchKey): thirty-
//! three coloured glyphs off one texture stay one draw call, where a keyed
//! tint would make them thirty-three. The neutral tint is
//! [`untinted`] — opaque white — which leaves any
//! image unchanged.
//!
//! # Authored at the drawn size, and cropped rather than scaled
//!
//! [`ICON_SIZE`] is 24, and the widget never scales: `MIN_FILTER` is
//! `GL_LINEAR` with no mipmaps, so a master baked at another size aliases,
//! and a cell smaller than the icon crops it. [`destination`](Icon::destination)
//! is the centred `ICON_SIZE` square intersected with the node's rect, so an
//! icon in a smaller box draws the middle of itself rather than a squashed
//! version of it.
//!
//! # Glyphs are the caller's to name
//!
//! Like [`Image`](crate::widgets::image::Image), this widget holds a
//! [`TextureHandle`] and an [`ImageSource`]
//! and cannot look either up: which texture an icon shows and where it sits in
//! it are facts about the icon the caller supplies. There is no glyph-name
//! enum here and no inventory; the bake names them.
//!
//! No icon is drawn yet: this is the widget and the pipeline, and the demo's
//! composition is a later task's.
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::{DrawCommand, Rect, TextureId};
//! use ui_core::texture::TextureHandle;
//! use ui_core::widgets::icon::{Icon, ICON_SIZE};
//! use ui_core::widgets::image::ImageSource;
//!
//! let mut nodes = Arena::new();
//! let mut icon = Icon::new(
//!     &mut nodes,
//!     TextureHandle::new(TextureId::new(7)),
//!     ImageSource::whole(24, 24),
//! );
//!
//! assert!(nodes.get(icon.handle()).is_some(), "its node is in the arena");
//! let commands = icon.paint(Rect::new(26.0, 26.0, 24.0, 24.0));
//! assert_eq!(commands.len(), 1, "an icon is one command and nothing else");
//! let DrawCommand::Image { rect, .. } = &commands[0] else {
//!     panic!("an icon paints an image");
//! };
//! assert_eq!(*rect, Rect::new(26.0, 26.0, ICON_SIZE, ICON_SIZE));
//! ```

use crate::arena::{Arena, Handle};
use crate::layout::{LayoutState, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{untinted, DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::texture::TextureHandle;
use crate::widgets::image::ImageSource;

/// How large an icon is drawn, in pixels, on each side.
///
/// Authored at the drawn size: the renderer filters linearly with no mipmaps,
/// so a master baked larger aliases down and a smaller cell crops. The widget
/// never scales, whatever rect it is given.
pub const ICON_SIZE: f32 = 24.0;

/// The appearance an icon is drawn with: its tint and its opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The colour the sampled texel is multiplied by, premultiplied.
    pub tint: Color,
    /// How much of the tinted image reaches the screen, `0.0..=1.0`.
    pub opacity: f32,
}

/// An icon: a texture drawn at [`ICON_SIZE`], tinted at draw time.
///
/// The widget holds the properties the task gives it —
/// [`tint`](Icon::tint) and [`opacity`](Icon::opacity) — the texture and the
/// source they are drawn from, and the node. There is no fit: the icon is
/// always [`ICON_SIZE`] square, centred in whatever rect it is given. There
/// is no clock: writing [`tint`](Icon::tint) repaints at once, and a theme
/// switch animates the bound property with the theme's own clock rather than
/// one here.
pub struct Icon {
    /// The colour the sampled texel is multiplied by. The truth, and what a
    /// caller writes — bound to a theme token for an icon that follows the
    /// theme, or set directly for one that does not.
    pub tint: Property<Color>,
    /// How much of the tinted image reaches the screen, from `0.0` to `1.0`.
    /// The truth, and what a caller writes; clamped when drawn.
    pub opacity: Property<f32>,
    texture: TextureHandle,
    source: ImageSource,
    node: Handle,
}

impl Icon {
    /// Creates an icon of `texture` in the arena, and returns it.
    ///
    /// Untinted and fully opaque: the master reaches the screen unchanged
    /// until a caller writes [`tint`](Icon::tint) or
    /// [`opacity`](Icon::opacity).
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{Rect, TextureId};
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::{Icon, ICON_SIZE};
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let icon = Icon::new(
    ///     &mut nodes,
    ///     TextureHandle::new(TextureId::new(3)),
    ///     ImageSource::whole(24, 24),
    /// );
    ///
    /// assert!(nodes.get(icon.handle()).is_some(), "its node is in the arena");
    /// assert_eq!((icon.size().width, icon.size().height), (ICON_SIZE, ICON_SIZE));
    /// ```
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, texture: TextureHandle, source: ImageSource) -> Self {
        let node = node::create(nodes, LayoutState::new());
        Icon {
            tint: Property::new(untinted()),
            opacity: Property::new(1.0),
            texture,
            source,
            node,
        }
    }

    /// Returns the icon's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Sets the colour the sampled texel is multiplied by, applying from the
    /// next repaint.
    ///
    /// A property write, so a bound tint follows its source with no clock
    /// here: a theme switch animates the theme's own property and this one
    /// recomputes on every frame of it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{Color, TextureId};
    /// use ui_core::property::Property;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::Icon;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let mut icon = Icon::new(
    ///     &mut nodes,
    ///     TextureHandle::new(TextureId::new(3)),
    ///     ImageSource::whole(24, 24),
    /// );
    /// icon.set_tint(Property::new(Color::new(255, 0, 0, 255)));
    /// assert_eq!(icon.tint.get(), Color::new(255, 0, 0, 255));
    /// ```
    pub fn set_tint(&mut self, tint: Property<Color>) {
        self.tint = tint;
    }

    /// Restores the neutral tint, so the master reaches the screen unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{Color, TextureId, untinted};
    /// use ui_core::property::Property;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::Icon;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let mut icon = Icon::new(
    ///     &mut nodes,
    ///     TextureHandle::new(TextureId::new(3)),
    ///     ImageSource::whole(24, 24),
    /// );
    /// icon.set_tint(Property::new(Color::new(255, 0, 0, 255)));
    /// icon.reset_tint();
    /// assert_eq!(icon.tint.get(), untinted());
    /// ```
    pub fn reset_tint(&mut self) {
        self.tint = Property::new(untinted());
    }

    /// Returns the texture the icon draws.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::Icon;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let handle = TextureHandle::new(TextureId::new(5));
    /// let icon = Icon::new(&mut nodes, handle, ImageSource::whole(24, 24));
    /// assert_eq!(icon.texture(), handle);
    /// ```
    #[must_use]
    pub fn texture(&self) -> TextureHandle {
        self.texture
    }

    /// Sets the texture the icon draws, applying from the next repaint.
    ///
    /// A plain field behind a setter: which texture an icon shows is the
    /// mapping, not the appearance, and nothing cross-fades between two of
    /// them.
    pub fn set_texture(&mut self, texture: TextureHandle) {
        self.texture = texture;
    }

    /// Returns the icon's own copy of what it is drawing.
    #[must_use]
    pub fn source(&self) -> ImageSource {
        self.source
    }

    /// Sets the icon's own copy of what it is drawing: its pixel size and
    /// where it sits in its texture.
    pub fn set_source(&mut self, source: ImageSource) {
        self.source = source;
    }

    /// Returns the image's own size in pixels: `width` and `height`.
    #[must_use]
    pub fn source_size(&self) -> (u32, u32) {
        self.source.size()
    }

    /// Returns the size the icon asks for: [`ICON_SIZE`] square.
    ///
    /// One size, and it is a constant: an icon has no content to measure and
    /// no fit to choose, so there is no `set_size` and no `size_hint`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::TextureId;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::{Icon, ICON_SIZE};
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let icon = Icon::new(
    ///     &mut nodes,
    ///     TextureHandle::new(TextureId::new(1)),
    ///     ImageSource::whole(24, 24),
    /// );
    /// let size = icon.size();
    /// assert_eq!((size.width, size.height), (ICON_SIZE, ICON_SIZE));
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(ICON_SIZE, ICON_SIZE)
    }

    /// Returns the rect the icon is drawn in: the centred [`ICON_SIZE`]
    /// square intersected with `rect`.
    ///
    /// A smaller cell crops rather than scales: the intersection keeps the
    /// middle of the icon that fits and drops the rest.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{Rect, TextureId};
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::Icon;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let icon = Icon::new(
    ///     &mut nodes,
    ///     TextureHandle::new(TextureId::new(1)),
    ///     ImageSource::whole(24, 24),
    /// );
    /// assert_eq!(
    ///     icon.destination(Rect::new(0.0, 0.0, 100.0, 100.0)),
    ///     Rect::new(38.0, 38.0, 24.0, 24.0),
    ///     "centred in the rect it was given"
    /// );
    /// ```
    #[must_use]
    pub fn destination(&self, rect: Rect) -> Rect {
        centred_square(rect, ICON_SIZE).intersection(rect)
    }

    /// Returns the appearance the icon is drawn with: its tint, and its
    /// opacity clamped to `0.0..=1.0`.
    ///
    /// A `NaN` opacity reads as `0.0`: a caller that has lost track of its
    /// arithmetic gets an invisible icon rather than one whose command
    /// carries a `NaN`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{TextureId, untinted};
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::Icon;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let icon = Icon::new(
    ///     &mut nodes,
    ///     TextureHandle::new(TextureId::new(1)),
    ///     ImageSource::whole(24, 24),
    /// );
    /// assert_eq!(icon.style().tint, untinted());
    /// assert_eq!(icon.style().opacity, 1.0);
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            tint: self.tint.get(),
            opacity: bounded(self.opacity.get(), 0.0, 1.0),
        }
    }

    /// Returns the draw commands that paint the icon within `rect`.
    ///
    /// **One command, ever**: a tinted [`DrawCommand::Image`] at
    /// [`destination`](Icon::destination), sampling the source's own UV
    /// window at the drawn tint and opacity, with square corners. An empty
    /// destination records nothing: there is no rectangle to draw in.
    ///
    /// Every value comes from the drawn properties, so an icon whose tint is
    /// bound to the theme paints at the tint the theme has got to.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{Color, DrawCommand, Rect, TextureId};
    /// use ui_core::property::Property;
    /// use ui_core::texture::TextureHandle;
    /// use ui_core::widgets::icon::Icon;
    /// use ui_core::widgets::image::ImageSource;
    ///
    /// let mut nodes = Arena::new();
    /// let mut icon = Icon::new(
    ///     &mut nodes,
    ///     TextureHandle::new(TextureId::new(7)),
    ///     ImageSource::whole(24, 24),
    /// );
    /// icon.set_tint(Property::new(Color::new(255, 0, 0, 255)));
    /// let commands = icon.paint(Rect::new(0.0, 0.0, 100.0, 100.0));
    ///
    /// assert_eq!(commands.len(), 1, "an icon is one command and nothing else");
    /// let DrawCommand::Image { tint, opacity, radius, .. } = &commands[0] else {
    ///     panic!("an icon paints an image");
    /// };
    /// assert_eq!(*tint, Color::new(255, 0, 0, 255));
    /// assert_eq!(*opacity, 1.0);
    /// assert_eq!(*radius, 0.0, "square corners");
    /// ```
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let area = self.destination(rect);
        if area.width > 0.0 && area.height > 0.0 {
            let style = self.style();
            painter.tinted_image(
                area,
                self.texture.id(),
                self.source.uv(),
                style.opacity,
                0.0,
                style.tint,
            );
        }
        painter.finish()
    }
}

/// Returns the `side`-pixel square centred in `rect`.
///
/// Pure: the same rect and the same side give the same square, with no widget
/// anywhere in it. [`Icon::destination`] intersects the answer with the rect,
/// so a square hanging off a smaller box is cropped to what fits.
fn centred_square(rect: Rect, side: f32) -> Rect {
    Rect::new(
        rect.x + (rect.width - side) / 2.0,
        rect.y + (rect.height - side) / 2.0,
        side,
        side,
    )
}

/// Returns `value` between `low` and `high`, `NaN` included.
///
/// The two comparisons rather than [`f32::clamp`]: `clamp` panics when its
/// bounds are the wrong way round, and a widget must not take a frame down.
/// A `NaN` becomes `low` — the `max` arm keeps the other operand — so a lost
/// opacity draws nothing rather than a `NaN`.
#[allow(clippy::manual_clamp)]
fn bounded(value: f32, low: f32, high: f32) -> f32 {
    value.max(low).min(high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::TextureId;
    use crate::texture::{ATLAS_MAX_IMAGE, ATLAS_SIZE};
    use std::time::Duration;

    /// An icon of texture 7, `24 × 24` with a texture of its own, in a fresh
    /// arena. Off-atlas, so its source is the whole texture.
    fn icon() -> (Arena<WidgetNode>, Icon) {
        let mut nodes = Arena::new();
        let icon = Icon::new(
            &mut nodes,
            TextureHandle::new(TextureId::new(7)),
            ImageSource::whole(24, 24),
        );
        (nodes, icon)
    }

    /// A node 100 × 100 at (26, 26): off the origin, so a fixture cannot
    /// mistake an origin for an extent.
    fn rect() -> Rect {
        Rect::new(26.0, 26.0, 100.0, 100.0)
    }

    #[test]
    fn an_icon_is_twenty_four_pixels_square() {
        assert_eq!(ICON_SIZE, 24.0);
        let (_nodes, icon) = icon();
        assert_eq!(icon.size(), Size::new(24.0, 24.0));
        assert_eq!(icon.size(), Size::new(ICON_SIZE, ICON_SIZE));
    }

    #[test]
    fn icon_size_fits_the_shared_image_atlas() {
        // The icon is baked at its drawn size and packed with the other small
        // images: both bounds have to hold, or the widget names a size the
        // atlas refuses and the icon gets a texture of its own per frame.
        //
        // No float-to-int `TryFrom` exists in std, `f32::from` stops at
        // `u16`, and `as` is banned by policy, so the small atlas bounds
        // arrive through `u16`: both are far below its ceiling, exactly.
        assert_eq!(ICON_SIZE.fract(), 0.0, "the icon side is whole pixels");
        assert_eq!(ICON_SIZE, 24.0, "the icon's side in pixels");
        let max_image = f32::from(u16::try_from(ATLAS_MAX_IMAGE).unwrap_or(u16::MAX));
        let quota = f32::from(u16::try_from(ATLAS_SIZE / 64).unwrap_or(u16::MAX));
        assert!(
            ICON_SIZE <= max_image,
            "an icon fits the shared image atlas"
        );
        assert!(
            ICON_SIZE <= quota,
            "thirty-two of them fit across one atlas side"
        );
    }

    #[test]
    fn an_icon_paints_exactly_one_tinted_image_command() {
        let (_nodes, icon) = icon();
        let commands = icon.paint(rect());
        assert_eq!(commands.len(), 1, "an icon is one command and nothing else");
        assert!(
            matches!(commands[0], DrawCommand::Image { .. }),
            "and that command is a tinted image"
        );
    }

    #[test]
    fn the_icon_is_centred_in_its_rect() {
        let (_nodes, icon) = icon();
        assert_eq!(
            icon.destination(rect()),
            Rect::new(64.0, 64.0, 24.0, 24.0),
            "centred in a 100-pixel box at (26, 26)"
        );
        assert_eq!(
            icon.destination(Rect::new(26.0, 26.0, 24.0, 24.0)),
            Rect::new(26.0, 26.0, 24.0, 24.0),
            "a box of its own size is itself, centred at (38, 38)"
        );
        assert_eq!(
            icon.destination(Rect::new(-12.0, -12.0, 24.0, 24.0)),
            Rect::new(-12.0, -12.0, 24.0, 24.0),
            "centred at the origin"
        );
    }

    #[test]
    fn every_icon_records_a_rect_inside_the_rect_it_was_given() {
        let (_nodes, icon) = icon();
        let inside = |outer: Rect, inner: Rect| {
            inner.x >= outer.x
                && inner.y >= outer.y
                && inner.x + inner.width <= outer.x + outer.width
                && inner.y + inner.height <= outer.y + outer.height
        };
        for given in [
            Rect::new(26.0, 26.0, 100.0, 100.0),
            Rect::new(26.0, 26.0, 24.0, 24.0),
            Rect::new(26.0, 26.0, 16.0, 16.0),
            Rect::new(26.0, 26.0, 24.0, 1.0),
        ] {
            let commands = icon.paint(given);
            assert_eq!(commands.len(), 1, "a box with extent draws: {given:?}");
            let DrawCommand::Image { rect: drawn, .. } = &commands[0] else {
                panic!("an icon paints an image");
            };
            assert!(
                inside(given, *drawn),
                "{drawn:?} is inside the rect it was given, {given:?}"
            );
        }
        assert!(
            icon.paint(Rect::new(26.0, 26.0, 0.0, 0.0)).is_empty(),
            "a box of no extent draws nothing"
        );
    }

    #[test]
    fn the_icon_carries_the_tint_it_was_given() {
        let (_nodes, mut icon) = icon();
        icon.set_tint(Property::new(Color::new(255, 0, 0, 255)));
        let commands = icon.paint(rect());
        assert_eq!(commands.len(), 1);
        let DrawCommand::Image { tint, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*tint, Color::new(255, 0, 0, 255));
        assert_ne!(*tint, untinted());
    }

    #[test]
    fn a_theme_switch_reaches_the_icon() {
        use crate::theme::{Theme, ThemeToken};

        // The tint follows the theme's ink through the property graph, with no
        // clock in the icon: the theme owns the transition.
        let theme = Theme::dark();
        let ink = theme.property(ThemeToken::Text);
        let (_nodes, mut icon) = icon();
        icon.set_tint(Property::bind(move || {
            ink.get().as_color().unwrap_or(Color::new(0, 0, 0, 255))
        }));

        let dark_ink = Color::new(255, 255, 255, 255);
        let light_ink = Color::new(0, 0, 0, 255);
        assert_eq!(icon.tint.get(), dark_ink, "the dark theme's ink");

        theme.switch_to(Theme::light(), 300);
        theme.tick(Duration::from_millis(150));
        let between = icon.tint.get();
        assert_ne!(between, dark_ink, "it left where it was");
        assert_ne!(between, light_ink, "and it has not arrived yet");
        for (channel, name) in [
            (between.r, "red"),
            (between.g, "green"),
            (between.b, "blue"),
        ] {
            assert!(
                channel > light_ink.r.min(dark_ink.r) && channel < light_ink.r.max(dark_ink.r),
                "the {name} channel is strictly between the two inks: {channel}"
            );
        }

        theme.tick(Duration::from_millis(150));
        assert_eq!(icon.tint.get(), light_ink, "exactly the light theme's ink");
    }

    #[test]
    fn an_untinted_icon_paints_opaque_white() {
        let (_nodes, icon) = icon();
        assert_eq!(icon.tint.get(), untinted());
        let commands = icon.paint(rect());
        let DrawCommand::Image { tint, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*tint, Color::new(255, 255, 255, 255));
    }

    #[test]
    fn an_icon_follows_a_written_tint_at_once() {
        // No snap, no tick, no clock: a property write is the next frame's
        // tint, which is what lets a theme switch carry it.
        let (_nodes, mut icon) = icon();
        icon.set_tint(Property::new(Color::new(0, 255, 0, 255)));
        let commands = icon.paint(rect());
        let DrawCommand::Image { tint, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*tint, Color::new(0, 255, 0, 255));

        icon.reset_tint();
        let commands = icon.paint(rect());
        let DrawCommand::Image { tint, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*tint, untinted(), "reset is the neutral tint again");
    }

    #[test]
    fn an_icon_paints_square_corners() {
        let (_nodes, icon) = icon();
        let commands = icon.paint(rect());
        let DrawCommand::Image { radius, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*radius, 0.0, "an icon clips no corner");
    }

    #[test]
    fn the_icon_samples_the_uv_window_it_was_given() {
        use crate::texture::{Pixels, TextureCache};
        use std::path::Path;

        // An atlas-resident icon samples its placement window, not the whole
        // texture: sampling the atlas whole would draw other people's icons.
        let mut cache = TextureCache::new();
        let mut loader = |_: &Path| Ok(Pixels::transparent(64, 64));
        let handle = cache
            .load(Path::new("icon.png"), &mut loader)
            .unwrap_or_else(|_| TextureHandle::new(TextureId::new(7)));
        let source = ImageSource::of(&cache, handle).unwrap_or_else(|| ImageSource::whole(64, 64));
        let mut nodes = Arena::new();
        let icon = Icon::new(&mut nodes, handle, source);

        let commands = icon.paint(rect());
        let DrawCommand::Image { uv, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*uv, source.uv(), "the placement window it was given");
        assert!(
            !uv.is_full(),
            "an atlas-resident icon is a window into the shared texture"
        );
    }

    #[test]
    fn an_icon_in_a_translucent_state_records_the_opacity_it_holds() {
        let (_nodes, icon) = icon();
        icon.opacity.set(0.5);
        let commands = icon.paint(rect());
        let DrawCommand::Image { opacity, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*opacity, 0.5);

        icon.opacity.set(f32::NAN);
        let commands = icon.paint(rect());
        let DrawCommand::Image { opacity, .. } = &commands[0] else {
            panic!("an icon paints an image");
        };
        assert_eq!(*opacity, 0.0, "a lost opacity draws nothing, not a NaN");
    }

    #[test]
    fn centred_square_is_pure() {
        // No widget in it: the same rect and side give the same square, off
        // the origin so neither coordinate can hide.
        assert_eq!(
            centred_square(Rect::new(26.0, 26.0, 100.0, 100.0), 24.0),
            Rect::new(64.0, 64.0, 24.0, 24.0)
        );
        assert_eq!(
            centred_square(Rect::new(-12.0, -12.0, 24.0, 24.0), 24.0),
            Rect::new(-12.0, -12.0, 24.0, 24.0)
        );
        assert_eq!(
            centred_square(Rect::new(26.0, 26.0, 16.0, 16.0), 24.0),
            Rect::new(22.0, 22.0, 24.0, 24.0),
            "a smaller box centres a larger square, and the intersection crops it"
        );
    }
}
