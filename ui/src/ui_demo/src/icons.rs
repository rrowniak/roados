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
//! drawn 1:1. **Downscaling through the sampler is the one exception** (see
//! [`destination`]): the operator's 2026-10-11 airbag call maps the whole 48
//! master into a 0.85 node rather than baking a 41-pixel master for it, and a
//! 0.85 minification is inside the window where `GL_LINEAR` downsampling is
//! visually lossless. Anything much smaller wants its own master instead —
//! no new bake per size was the point of the exception, and resampling the
//! master offline every time a size moves is what it removes.
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
//! shapes — the identity — over the full texture, at the centred square the
//! node's rect allows (see [`destination`]: the square is capped to the rect's
//! smaller dimension so a non-square node can never stretch the glyph). An
//! empty intersection records nothing.
//!
//! **Composed inside the badge's button in the status bar.** The icon is an
//! `Image` node **nested in the airbag badge's `Button`**, stacked at the
//! button's content origin so the button's own background wraps it; it is
//! painted through [`paint`] by its own arm, and the arm for a tap is the
//! button's, since an `Image` has no handler of its own: the badge is one
//! target to a finger. (2026-10-11: the operator moved it here from the dock —
//! an airbag badge belongs to the status bar, not to a dock slot — and then
//! asked for the button to wrap it, which is why it is a child rather than a
//! sibling.)
//!
//! Where the task file names `paint(spec, rect)`, this module ships
//! [`paint`] with the texture it names: a pure painter cannot borrow the
//! cache, so the GPU image arrives as a [`TextureId`]
//! — a test fixture in the suite, the loaded handle on screen — and the
//! colour never touches the filesystem. [`paint_with_opacity`] is the same
//! command with the opacity the caller's image state holds.

use ui_core::paint::{Color, DrawCommand, Painter, Rect, TextureId, UvRect};

/// The sizes baked offline, in pixels per side: one white master each.
///
/// Both dimensions equal, both within `1..=512` (`ATLAS_MAX_IMAGE`).
pub const BAKED_SIZES: [u32; 2] = [24, 48];

/// Returns the baked size that fits a status bar of `bar_height`.
///
/// The icon is drawn 1:1 — a widget never scales — so a taller bar cannot
/// stretch the small master. It picks the nearest of [`BAKED_SIZES`] to half
/// the bar's height, the share the 24-pixel master held of the old 48-pixel
/// bar. Half of the new 102-pixel bar is 51, so it draws the 48 master; a tie
/// resolves to the smaller file, like [`baked_file`].
#[must_use]
pub fn size_for_height(bar_height: f32) -> u32 {
    let target = bar_height / 2.0;
    let width = |size: u32| f32::from(u16::try_from(size).unwrap_or(u16::MAX));
    let mut nearest = BAKED_SIZES[0];
    let mut distance = (target - width(nearest)).abs();
    for size in BAKED_SIZES {
        let candidate = (target - width(size)).abs();
        if candidate < distance {
            nearest = size;
            distance = candidate;
        }
    }
    nearest
}

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

/// Returns the `side`-pixel square centred in `rect`, intersected with it.
///
/// **The drawn square is always square, whatever shape `rect` is.** The glyph
/// is square, so mapping the whole master into a cell whose width and height
/// differ would stretch it; `side` is therefore capped at the rect's *smaller*
/// dimension before it is centred. A cell smaller than `side` does **not**
/// crop the middle: the cap makes the whole master minify into the cell, so the
/// sampler scales rather than the bake multiplying. That is what the airbag's
/// tall containing node relies on — it is an `f32` cell the height of the
/// badge's inner box, and the glyph is a centred square within it. It is a
/// downscale-only licence: an upscale would magnify through `GL_LINEAR` with
/// no mipmaps to hide behind.
///
/// [`Icon::destination`](ui_core::widgets::icon::Icon) is to a fixed-size
/// widget what this is to a caller-chosen size.
#[must_use]
fn destination(rect: Rect, side: f32) -> Rect {
    let side = side.min(rect.width).min(rect.height);
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
    fn the_bar_height_picks_the_master_that_fits_it() {
        // Half the bar's height, nearest baked: the old 48-pixel bar drew the
        // 24 master, and the new 102-pixel bar draws the 48 one. A tie keeps
        // the smaller file, like `baked_file`.
        assert_eq!(size_for_height(48.0), 24, "the old bar, the small master");
        assert_eq!(
            size_for_height(102.0),
            48,
            "half of 102 is 51, nearest the large master"
        );
        assert_eq!(
            size_for_height(72.0),
            24,
            "half of 72 is 36, a tie, and ties stay small"
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
    fn a_non_square_cell_never_stretches_the_icon() {
        // The glyph is square, so the square it is drawn in must be too: a tall
        // cell (the airbag badge's, whose node is the height of the button's
        // inner box) keeps the cell's *smaller* dimension on both sides, centred.
        // Without the cap the full master maps into a `width`-by-`side` rect and
        // the glyph skews — which is the defect this pins.
        let tall = Rect::new(32.0, 64.0, 40.0, 100.0);
        let drawn = drawn_rect(&paint(&white(48), tall, fixture())).expect("an image");
        assert_eq!(
            drawn,
            Rect::new(32.0, 94.0, 40.0, 40.0),
            "a 40-wide square centred in the tall cell"
        );
        assert!(
            (drawn.width - drawn.height).abs() < f32::EPSILON,
            "and it is square: {drawn:?}"
        );

        // The wide case is the same rule the other way round.
        let wide = Rect::new(10.0, 20.0, 100.0, 30.0);
        let drawn = drawn_rect(&paint(&white(48), wide, fixture())).expect("an image");
        assert_eq!(drawn, Rect::new(45.0, 20.0, 30.0, 30.0));
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
}
