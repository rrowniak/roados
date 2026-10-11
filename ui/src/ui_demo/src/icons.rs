//! The demo's one icon: a white airbag master tinted at draw time.
//!
//! **Fill-based source, and its `{fill}` rule.** The source,
//! `assets/icons/airbag-svgrepo-com.svg`, carries `fill="#000000"` on its
//! root and no stroke: it is a filled silhouette, not a Lucide-style outline.
//! The bake's stylesheet is therefore `svg{fill:#ffffff}*{fill:#ffffff}` —
//! the `{fill:…}` rule wins over the presentation attribute — and baking it
//! with the pipeline's `*{color:…}` rule instead produces a valid black PNG
//! with exit status 0, which the white-mean guard rejects. The input guard is
//! `fill` on the root, not `stroke`.
//!
//! **Baked at size, and that is `TASK_UI_PRIM_39`'s rule.** There are no
//! mipmaps and no DPI: `MIN_FILTER` is `GL_LINEAR`, 1 asset px is 1 window
//! px, and "dynamic size" means baked at the exact on-screen pixel size and
//! drawn 1:1. A widget never scales; a smaller cell crops.
//!
//! **White master plus tint, and that is `TASK_UI_PRIM_44`'s pipeline.**
//! One baked white master per size is loaded through
//! `Renderer::load_texture`, and the caller-chosen colour arrives as the
//! shader tint (`DrawCommand::Image::tint`): switching colour re-tints the
//! same texture, and no per-colour PNG variant exists. The neutral tint is
//! opaque white, which leaves non-icon images unchanged.
//!
//! **Drawn 1:1.** [`paint`] records one tinted-image command with
//! [`ImageFit::Fill`](ui_core::widgets::image::ImageFit::Fill) on matching
//! shapes — the identity — over the full texture, at the centred square
//! intersected with the node's rect. An empty intersection records nothing.
//!
//! **Composed as a sibling in the status bar.** The icon is a sibling `Image`
//! node in the airbag group beside its two-line text, painted through
//! [`paint`] by its own arm. (2026-10-11: the operator moved it here from the
//! dock — an airbag badge belongs to the status bar, not to a dock slot — so
//! the task file's "beside a `Button`" no longer holds and this paragraph
//! records the redirect.)
//!
//! Where the task file names `paint(spec, rect)`, this module ships
//! [`paint`] with the texture it names: a pure painter cannot borrow the
//! cache, so the GPU image arrives as a [`TextureId`]
//! — a test fixture in the suite, the loaded handle on screen — and the
//! colour never touches the filesystem. [`paint_with_opacity`] is the same
//! command with the opacity the caller's image state holds.

use ui_core::paint::{Color, DrawCommand, Painter, Rect, TextureId, UvRect};
use ui_core::property::Property;
use ui_core::theme::{Theme, ThemeToken};

/// The sizes baked offline, in pixels per side: one white master each.
///
/// Both dimensions equal, both within `1..=512` (`ATLAS_MAX_IMAGE`).
pub const BAKED_SIZES: [u32; 2] = [24, 48];

/// The size the demo draws, in pixels per side: the small master, so the
/// glyph sits beside the airbag text inside the 48-pixel status bar.
pub const ICON_SIZE_PX: u32 = 24;

/// What the demo's icon is: how large it is drawn, and in what colour.
///
/// `size_px` selects the baked file — never the colour: two specs differing
/// only in `color` resolve to the same bytes, and the colour becomes the
/// shader tint at paint time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconSpec {
    /// The drawn size in pixels per side, `1..=512`. Unknown sizes resolve to
    /// the nearest baked file; arbitrary sizes are a re-bake, still offline.
    pub size_px: u32,
    /// The colour the master is tinted with at draw time, premultiplied by
    /// contract. Opaque tints are unaffected either way.
    pub color: Color,
}

/// Returns the baked file for `size_px`: the nearest of [`BAKED_SIZES`].
///
/// Keyed by size only — the colour never touches the filesystem. A tie
/// resolves to the smaller file: 24 and 48 resolve to their own masters,
/// 25 to the small one, 47 and 100 to the large one, and 1 stays low.
#[must_use]
pub fn baked_file(size_px: u32) -> &'static str {
    let mut nearest = BAKED_SIZES[0];
    let mut distance = size_px.abs_diff(nearest);
    for size in BAKED_SIZES {
        let candidate = size_px.abs_diff(size);
        if candidate < distance {
            nearest = size;
            distance = candidate;
        }
    }
    if nearest == BAKED_SIZES[0] {
        "airbag_24.png"
    } else {
        "airbag_48.png"
    }
}

/// Returns the tint the icon follows the theme with: the theme's ink.
///
/// Bound over `theme.property(Text)`, so a theme switch animates the tint
/// with the theme's own clock and nothing here owns one. The fallback is
/// opaque white — the neutral tint — so a token holding a number leaves the
/// master unchanged rather than tinting it black.
#[must_use]
pub fn theme_tint(theme: &Theme) -> Property<Color> {
    let ink = theme.property(ThemeToken::Text);
    Property::bind(move || {
        ink.get()
            .as_color()
            .unwrap_or(Color::new(255, 255, 255, 255))
    })
}

/// Returns the `side`-pixel square centred in `rect`, intersected with it.
///
/// [`Icon::destination`](ui_core::widgets::icon::Icon) is to a fixed-size
/// widget what this is to a caller-chosen size: a smaller cell crops the
/// middle rather than scaling the glyph.
#[must_use]
fn destination(rect: Rect, side: f32) -> Rect {
    Rect::new(
        rect.x + (rect.width - side) / 2.0,
        rect.y + (rect.height - side) / 2.0,
        side,
        side,
    )
    .intersection(rect)
}

/// Returns `size_px` as an `f32` side length.
///
/// `as` is banned by policy and no float-to-int round trip is needed going
/// the other way: every baked size is far below `u16`'s ceiling, exactly.
/// The demo lays the icon's node out with this; [`paint_with_opacity`]
/// draws with it.
#[must_use]
pub fn drawn_side(size_px: u32) -> f32 {
    f32::from(u16::try_from(size_px).unwrap_or(u16::MAX))
}

/// Returns the commands that paint `spec` within `rect`, sampling `texture`.
///
/// Exactly one tinted-image command with full UV and square corners, at
/// `spec.color` and full opacity; an empty intersection records nothing.
#[must_use]
pub fn paint(spec: &IconSpec, rect: Rect, texture: TextureId) -> Vec<DrawCommand> {
    paint_with_opacity(spec, rect, texture, 1.0)
}

/// Returns the commands that paint `spec` within `rect`, at `opacity`.
///
/// [`paint`] at the opacity the caller's image state holds — the drawn
/// opacity, already the fraction (not a tint) the command carries — clamped
/// to `0.0..=1.0` by the pipeline when drawn.
#[must_use]
pub fn paint_with_opacity(
    spec: &IconSpec,
    rect: Rect,
    texture: TextureId,
    opacity: f32,
) -> Vec<DrawCommand> {
    let mut painter = Painter::new();
    let area = destination(rect, drawn_side(spec.size_px));
    if area.width > 0.0 && area.height > 0.0 {
        painter.tinted_image(area, texture, UvRect::full(), opacity, 0.0, spec.color);
    }
    painter.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn spec(size_px: u32, color: Color) -> IconSpec {
        IconSpec { size_px, color }
    }

    fn white(size_px: u32) -> IconSpec {
        spec(size_px, Color::new(255, 255, 255, 255))
    }

    /// A texture id the commands name without any bytes behind it: every
    /// assertion here is over rect math and the recorded tint, never over
    /// pixels.
    fn fixture() -> TextureId {
        TextureId::new(7)
    }

    /// A node 100 × 100 at (32, 64): off the origin, so a fixture cannot
    /// mistake an origin for an extent.
    fn rect() -> Rect {
        Rect::new(32.0, 64.0, 100.0, 100.0)
    }

    /// Reads the icon's rect off a painted command, failing without `panic!`.
    fn drawn_rect(commands: &[DrawCommand]) -> Option<Rect> {
        if let DrawCommand::Image { rect, .. } = &commands[0] {
            Some(*rect)
        } else {
            None
        }
    }

    #[test]
    fn baked_file_resolves_every_baked_size() {
        assert_eq!(baked_file(24), "airbag_24.png");
        assert_eq!(baked_file(48), "airbag_48.png");
    }

    #[test]
    fn unknown_size_maps_to_nearest_baked_file() {
        assert_eq!(baked_file(25), "airbag_24.png", "one past the small master");
        assert_eq!(
            baked_file(47),
            "airbag_48.png",
            "one short of the large one"
        );
        assert_eq!(baked_file(100), "airbag_48.png", "past the top stays at it");
        assert_eq!(
            baked_file(1),
            "airbag_24.png",
            "and below the bottom stays low"
        );
        assert_eq!(
            baked_file(36),
            "airbag_24.png",
            "a tie resolves to the smaller file"
        );
    }

    #[test]
    fn icon_color_never_selects_a_file() {
        // Two specs differing only in colour resolve to the same bytes: the
        // colour becomes the shader tint at paint time, never a second file.
        let red = spec(48, Color::new(255, 0, 0, 255));
        let green = spec(48, Color::new(0, 255, 0, 255));
        assert_eq!(baked_file(red.size_px), baked_file(green.size_px));
        assert_eq!(baked_file(24), "airbag_24.png");
    }

    #[test]
    fn the_icon_is_centred_in_its_rect() {
        let commands = paint(&white(24), rect(), fixture());
        assert_eq!(commands.len(), 1);
        assert_eq!(
            drawn_rect(&commands),
            Some(Rect::new(70.0, 102.0, 24.0, 24.0)),
            "a 24-pixel icon centred in a 100-pixel box at (32, 64)"
        );
    }

    #[test]
    fn every_icon_records_a_rect_inside_the_rect_it_was_given() {
        let inside = |outer: Rect, inner: Rect| {
            inner.x >= outer.x
                && inner.y >= outer.y
                && inner.x + inner.width <= outer.x + outer.width
                && inner.y + inner.height <= outer.y + outer.height
        };
        for given in [
            Rect::new(32.0, 64.0, 100.0, 100.0),
            Rect::new(32.0, 64.0, 48.0, 48.0),
            Rect::new(32.0, 64.0, 16.0, 16.0),
        ] {
            let commands = paint(&white(48), given, fixture());
            assert_eq!(commands.len(), 1, "a box with extent draws: {given:?}");
            assert!(
                drawn_rect(&commands).is_some_and(|drawn| inside(given, drawn)),
                "the icon stays inside the rect it was given: {given:?}"
            );
        }
        assert!(
            paint(&white(48), Rect::new(32.0, 64.0, 0.0, 0.0), fixture()).is_empty(),
            "a box of no extent draws nothing"
        );
    }

    #[test]
    fn an_icon_paints_exactly_one_tinted_image_command() {
        let commands = paint(&white(48), rect(), fixture());
        assert_eq!(commands.len(), 1, "an icon is one command and nothing else");
        assert!(
            matches!(commands[0], DrawCommand::Image { .. }),
            "and that command is a tinted image"
        );
    }

    #[test]
    fn the_icon_carries_the_tint_it_was_given() {
        let tint = Color::new(255, 0, 0, 255);
        let commands = paint(&spec(48, tint), rect(), fixture());
        assert_eq!(commands.len(), 1);
        if let DrawCommand::Image {
            tint: recorded,
            uv,
            radius,
            opacity,
            ..
        } = &commands[0]
        {
            assert_eq!(*recorded, tint);
            assert!(uv.is_full(), "an icon samples the whole master");
            assert_eq!(*radius, 0.0, "square corners");
            assert_eq!(*opacity, 1.0);
        } else {
            assert_eq!("image", "other", "an icon paints an image");
        }
    }

    #[test]
    fn an_icon_in_a_translucent_state_records_the_opacity_it_holds() {
        // The opacity rides the command's own scalar — clamped by the
        // pipeline, never multiplied into the tint.
        let commands = paint_with_opacity(&white(48), rect(), fixture(), 0.5);
        assert_eq!(commands.len(), 1);
        if let DrawCommand::Image { opacity, tint, .. } = &commands[0] {
            assert_eq!(*opacity, 0.5);
            assert_eq!(*tint, Color::new(255, 255, 255, 255));
        } else {
            assert_eq!("image", "other", "an icon paints an image");
        }
    }

    #[test]
    fn a_theme_switch_reaches_the_icon() {
        // The demo binds the icon's tint to the theme's ink: dark ink, then
        // strictly between mid-transition, then exactly light ink — with no
        // clock in this module.
        let theme = Theme::dark();
        let tint = theme_tint(&theme);
        let dark_ink = Color::new(255, 255, 255, 255);
        let light_ink = Color::new(0, 0, 0, 255);
        assert_eq!(tint.get(), dark_ink, "the dark theme's ink");

        theme.switch_to(Theme::light(), 300);
        theme.tick(Duration::from_millis(150));
        let between = tint.get();
        assert_ne!(between, dark_ink, "it left where it was");
        assert_ne!(between, light_ink, "and it has not arrived yet");
        for (channel, name) in [
            (between.r, "red"),
            (between.g, "green"),
            (between.b, "blue"),
        ] {
            assert!(
                channel > 0 && channel < 255,
                "the {name} channel is strictly between the two inks: {channel}"
            );
        }

        theme.tick(Duration::from_millis(150));
        assert_eq!(tint.get(), light_ink, "exactly the light theme's ink");
    }
}
