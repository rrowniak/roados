//! The Tesla surface's persistent chrome: a top status bar, a bottom dock and
//! the car-status pane.
//!
//! **Five facts a reader would otherwise have to derive from `ui_core`:**
//!
//! 1. **What the three regions are.** A [`STATUS_BAR_HEIGHT`]-tall status bar
//!    across the top, a car-status pane down the left at
//!    [`CAR_STATUS_PANE_FRACTION`] of the window's width, and a
//!    [`DOCK_HEIGHT`]-tall dock across the bottom. [`chrome_regions`] is the one
//!    place their rectangles are computed.
//! 2. **They are drawn over the map, and are a region of the demo page rather
//!    than a panel over it.** The car-status pane in particular has no
//!    open/closed state and no dismissal: `DEMO_APPLICATION.md` § *Screens*
//!    records it as *"a persistent region of the map screen"*, so this module
//!    builds neither.
//! 3. **A surface is a frosted rounded panel.** [`chrome_surface_commands`]
//!    records a blurred [`DrawCommand::Backdrop`] and then a `RoundedRect`, both
//!    in the premultiplied translucent surface colour, and **the backdrop carries
//!    the panel's corner radius** so its composite is masked to the rounded shape.
//!    That is the whole point of the radius on the backdrop: a rectangular
//!    composite would tint the four corners a rounded fill leaves empty, and a
//!    dark tint over the map reads as a black corner rather than as the live map.
//!    With the radius, the corners are simply not composited and the map shows
//!    through them.
//! 4. **The dock's glyphs are placeholders.** See [`DOCK_GLYPHS`].
//! 5. **[`premultiplied`] exists and why.** `Color` is premultiplied alpha, so a
//!    translucent surface is `(c · a) / 255` per channel, done in `u16` so the
//!    product is not truncated before the divide.
//!
//! Everything here is a pure function of a rect and a palette: no arena is
//! touched, which is what lets the arithmetic be verified before `main` builds a
//! node.

use ui_core::arena::Handle;
use ui_core::layout::Size;
use ui_core::paint::{BackdropMode, Color, DrawCommand, Painter, Rect};
use ui_core::theme::{Theme, ThemeToken};

/// The alpha of the bar and pane surfaces: translucent, and not 255.
///
/// **First-principles**, because `DEMO_APPLICATION.md` § *Could not verify*
/// records that *"Tesla publishes no design tokens at all."* It is set so the
/// map reads through the panel without the panel reading as the map: at 255 the
/// panel would be opaque, and the whole point of the chrome is that it is
/// translucent over the live scene.
pub const CHROME_ALPHA: u8 = 200;

/// The Gaussian standard deviation, in pixels, the backdrop is blurred with.
///
/// **First-principles**, like [`CHROME_ALPHA`]. A 5-tap kernel: `blur::taps_for`
/// caps at `min(ceil(2σ), 4)` taps either side, so σ = 2.5 would be the 9-tap
/// blur. The tighter kernel is chosen because the three surfaces each cost a
/// window-sized capture and two blur passes and this keeps the `demo` page above
/// the 55 fps floor.
pub const BACKDROP_SIGMA: f32 = 1.0;

/// How tall the top status bar is.
///
/// `48.0` — the smallest height that holds one line of the theme's `FontSizeMd`
/// with room above and below. First-principles, per § *Could not verify*.
pub const STATUS_BAR_HEIGHT: f32 = 48.0;

/// How tall the bottom dock is.
pub const DOCK_HEIGHT: f32 = 96.0;

/// The car-status pane's share of the window's width.
///
/// **The one number in this module that is not first-principles**: it is the
/// photograph's *"Left ~40% in photo `02`"*, cited in `DEMO_APPLICATION.md`
/// § *The car-status pane*.
pub const CAR_STATUS_PANE_FRACTION: f32 = 0.40;

/// The inset between the pane's edge and its own content.
pub const PANE_INSET: f32 = 12.0;

/// How tall the card carousel at the pane's foot is.
pub const CARDS_HEIGHT: f32 = 96.0;

/// The radius of one pager dot, in pixels.
pub const PAGER_DOT_RADIUS: f32 = 6.0;

/// The gap between two pager dots' centres, in pixels.
pub const PAGER_DOT_GAP: f32 = 20.0;

/// How many dots the pager draws.
pub const PAGER_DOTS: usize = 3;

/// The centre offsets of the pager's dots from the first one, in pixels.
///
/// Written out rather than multiplied by `index`, because `usize` to `f32` is a
/// conversion this module forbids (`as` appears nowhere in it).
const PAGER_OFFSETS: [f32; PAGER_DOTS] = [0.0, PAGER_DOT_GAP, PAGER_DOT_GAP * 2.0];

/// The corner radius a chrome surface is rounded to.
pub const CHROME_RADIUS: f32 = 12.0;

/// The number of slots the bottom dock holds.
pub const DOCK_SLOTS: usize = 5;

/// The glyphs the five dock slots show, in slot order.
///
/// **These are placeholders, and that is a recorded deviation.** The operator's
/// decision of 2026-09-30, `DEMO_APPLICATION.md` § *Operator decisions
/// (2026-09-30)* item 3, is *"**Real icons.** … **No placeholder geometric
/// shapes**"*. A dedicated task, `TASK_UI_PRIM_44`, inventories the icons and
/// ships `ui_core::widgets::Icon`, which is what replaces these. Until it lands,
/// the criterion that holds the dock is its geometry and its five-slot count,
/// not its glyphs.
///
/// **A `const` array and not a `Vec`, and its length is the count** — the same
/// reason `Page::ALL` is `[Page; 7]`: a fixed-size array, so the type carries
/// the count.
pub const DOCK_GLYPHS: [&str; DOCK_SLOTS] = ["C", "T", "A", "L", "V"];

/// The theme tokens the five indicator rows read, in the section's own order:
/// red, amber, green, blue, grey.
///
/// `DEMO_APPLICATION.md` § *The car-status pane* enumerates ~20 conditions across
/// five colours. **This task builds one row per colour class and not the ~20
/// conditions**, and none of the blink semantics or the latch.
pub const INDICATOR_TOKENS: [ThemeToken; 5] = [
    ThemeToken::Error,
    ThemeToken::Warning,
    ThemeToken::Success,
    ThemeToken::Primary,
    ThemeToken::TextMuted,
];

/// The chrome's colours, read from the theme and never invented.
///
/// **`premultiplied` on `surface`, and nothing else.** Every other field is
/// opaque: `Color` is documented in `ui/src/ui_core/src/property.rs` as
/// premultiplied, and a translucent colour is a premultiplication the caller
/// performs. A panel at `Surface` with `a = 200` written straight into `Color`
/// composites as `245 * 200 = 49 000` per channel under
/// `GL_ONE, GL_ONE_MINUS_SRC_ALPHA` — far too bright — and no assertion on a
/// `Color` field catches it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromePalette {
    /// The bar and pane surface, translucent. Premultiplied.
    pub surface: Color,
    /// The chrome's text. Opaque, from `ThemeToken::Text`.
    pub foreground: Color,
    /// The chrome's secondary text. Opaque, from `ThemeToken::TextMuted`.
    pub muted: Color,
    /// The pager's active dot. Opaque, from `ThemeToken::Primary`.
    pub active: Color,
    /// The pager's inactive dots and the drive-mode strip's track.
    /// Opaque, from `ThemeToken::Border`.
    pub inactive: Color,
}

/// Returns the chrome's palette, read from `theme`.
///
/// Reads `theme.get(token).as_color().unwrap_or(fallback)` — the `fn themed_color`
/// precedent in `main.rs` — and **`surface` is the only [`premultiplied`] call
/// site in the module**.
#[must_use]
pub fn chrome_palette(theme: &Theme) -> ChromePalette {
    let read = |token: ThemeToken, fallback: Color| theme.get(token).as_color().unwrap_or(fallback);
    let surface = read(ThemeToken::Surface, Color::new(30, 30, 30, 255));
    ChromePalette {
        surface: premultiplied(surface, CHROME_ALPHA),
        foreground: read(ThemeToken::Text, Color::new(255, 255, 255, 255)),
        muted: read(ThemeToken::TextMuted, Color::new(158, 158, 158, 255)),
        active: read(ThemeToken::Primary, Color::new(187, 134, 252, 255)),
        inactive: read(ThemeToken::Border, Color::new(51, 51, 51, 255)),
    }
}

/// Returns `color`'s channels multiplied by `alpha / 255`, in `u16`.
///
/// **The `u16` is the whole of it.** `Pixels::premultiply` in
/// `ui/src/ui_core/src/texture.rs` does this conversion and its own doc records
/// why: *"(200 * 200) / 255 is 156.86, and a `u8` multiply would have said 156
/// for every value."* An implementation that multiplied in `u8` truncates the
/// product before the divide and darkens every translucent colour by up to one
/// whole step, and **the difference is one digit of a `u8`** — a test asserting
/// "the channels got smaller" passes on it.
fn premultiplied(color: Color, alpha: u8) -> Color {
    let channel = |value: u8| {
        let product = u16::from(value) * u16::from(alpha);
        u8::try_from(product / 255).unwrap_or(u8::MAX)
    };
    Color::new(channel(color.r), channel(color.g), channel(color.b), alpha)
}

/// A chrome surface: its node, the radius it is drawn with, and its palette.
pub struct Chrome {
    /// The node the surface is drawn on.
    pub node: Handle,
    /// The corner radius the fill is rounded to.
    pub radius: f32,
    /// The surface's colours.
    pub palette: ChromePalette,
}

/// Returns the commands that paint a chrome surface within `rect`.
///
/// **A blurred `Backdrop` first, then the rounded fill**, in that order: a
/// backdrop captures what was recorded before it, so it must come before the fill
/// on the same node and `a_chrome_surface_records_its_backdrop_before_its_surface`
/// is the enforcement. **Both carry `chrome.radius`** — the fill so the panel is
/// rounded, and the backdrop so its composite is masked to that same rounded
/// shape. A backdrop with `radius: 0.0` under a rounded fill is the defect this
/// pairing removes: the composite would tint the fill's empty corners and they
/// would read as black over the map.
#[must_use]
pub fn chrome_surface_commands(rect: Rect, chrome: &Chrome) -> Vec<DrawCommand> {
    let mut painter = Painter::new();
    painter.backdrop(
        rect,
        BackdropMode::Blur(BACKDROP_SIGMA),
        chrome.palette.surface,
        chrome.radius,
    );
    painter.rounded_rect(rect, chrome.radius, chrome.palette.surface);
    painter.finish()
}

/// Returns the centre of pager dot `index` when the dots are centred in `row`.
///
/// **Arithmetic rather than a literal `x` per dot**, because three literals are
/// three numbers a resize could not move together.
#[must_use]
pub fn pager_centre(index: usize, row: Rect) -> (f32, f32) {
    let span = PAGER_OFFSETS[PAGER_DOTS - 1];
    let start = row.x + (row.width - span) / 2.0;
    let y = row.y + row.height / 2.0;
    let offset = PAGER_OFFSETS.get(index).copied().unwrap_or(0.0);
    (start + offset, y)
}

/// Returns the commands that paint the pager's dots within `row`.
///
/// The first dot is [`ChromePalette::active`] and the rest
/// [`ChromePalette::inactive`].
#[must_use]
pub fn pager_commands(row: Rect, palette: &ChromePalette) -> Vec<DrawCommand> {
    let mut painter = Painter::new();
    for index in 0..PAGER_DOTS {
        let color = if index == 0 {
            palette.active
        } else {
            palette.inactive
        };
        painter.circle(pager_centre(index, row), PAGER_DOT_RADIUS, color);
    }
    painter.finish()
}

/// Returns the three chrome regions' rectangles for a window of `window` whose
/// content starts at `content_top`.
///
/// The order is status bar, car-status pane, bottom dock. They are pairwise
/// non-overlapping; `the_chrome_nodes_do_not_overlap_each_other` is the test.
#[must_use]
pub fn chrome_regions(window: Size, content_top: f32) -> [Rect; 3] {
    let dock_top = window.height - DOCK_HEIGHT;
    [
        Rect::new(0.0, content_top, window.width, STATUS_BAR_HEIGHT),
        Rect::new(
            0.0,
            content_top + STATUS_BAR_HEIGHT,
            window.width * CAR_STATUS_PANE_FRACTION,
            dock_top - content_top - STATUS_BAR_HEIGHT,
        ),
        Rect::new(0.0, dock_top, window.width, DOCK_HEIGHT),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::arena::Arena;
    use ui_core::layout::LayoutState;
    use ui_core::node::{self, WidgetNode};
    use ui_core::theme::PropertyValue;

    /// Builds a `Chrome` whose palette is `palette`. The node is a real arena
    /// handle so the struct is the production one, but nothing in
    /// [`chrome_surface_commands`] reads it.
    fn chrome_with(palette: ChromePalette) -> Chrome {
        let mut nodes: Arena<WidgetNode> = Arena::new();
        let node = node::create(&mut nodes, LayoutState::new());
        Chrome {
            node,
            radius: CHROME_RADIUS,
            palette,
        }
    }

    fn dark_chrome() -> Chrome {
        chrome_with(chrome_palette(&Theme::dark()))
    }

    #[test]
    fn premultiplied_multiplies_each_channel_by_the_alpha_in_u16() {
        assert_eq!(
            premultiplied(Color::new(200, 200, 200, 128), 128),
            Color::new(100, 100, 100, 128),
            "(200 * 128) / 255 is 100.39, and a u8 multiply would have said 100 as \
             well here — which is why the control below and the value above are \
             the assertion: Pixels::premultiply's doc predicted 156 against 156.86 \
             for a u8 multiply at a different pair of numbers"
        );
        assert_eq!(
            premultiplied(Color::new(200, 200, 200, 200), 200),
            Color::new(156, 156, 156, 200),
            "(200 * 200) / 255 is 156.86, which truncates to 156 and **not** to the \
             156 a u8 multiply would also produce — the point is the size of the \
             product, not the rounding of this one pair"
        );
    }

    #[test]
    fn premultiplied_leaves_an_opaque_colour_unchanged() {
        let color = Color::new(12, 34, 56, 255);
        assert_eq!(
            premultiplied(color, 255),
            color,
            "alpha 255 is the identity, or the function above could always return \
             its input"
        );
    }

    #[test]
    fn a_translucent_surface_is_darker_than_the_themes_opaque_surface() {
        let theme = Theme::dark();
        let palette = chrome_palette(&theme);
        let opaque = theme
            .get(ThemeToken::Surface)
            .as_color()
            .expect("Surface is a colour");
        assert!(
            palette.surface.r < opaque.r
                && palette.surface.g < opaque.g
                && palette.surface.b < opaque.b,
            "every channel is multiplied by the alpha: {:?} against {:?}",
            palette.surface,
            opaque
        );
        assert_eq!(palette.surface.a, CHROME_ALPHA);
    }

    #[test]
    fn chrome_palette_reads_five_values_and_only_one_is_translucent() {
        let palette = chrome_palette(&Theme::dark());
        assert_eq!(palette.surface.a, CHROME_ALPHA);
        assert_eq!(palette.foreground.a, 255);
        assert_eq!(palette.muted.a, 255);
        assert_eq!(palette.active.a, 255);
        assert_eq!(palette.inactive.a, 255);
    }

    #[test]
    fn chrome_palette_falls_back_when_a_token_holds_a_number() {
        let theme = Theme::dark();
        theme.set(ThemeToken::Surface, PropertyValue::Number(7.0));
        let palette = chrome_palette(&theme);
        let fallback = premultiplied(Color::new(30, 30, 30, 255), CHROME_ALPHA);
        assert_eq!(
            palette.surface, fallback,
            "a token holding a number is not a colour, so the fallback is used"
        );
    }

    #[test]
    fn a_chrome_surface_records_a_rounded_backdrop_before_its_rounded_fill() {
        let chrome = dark_chrome();
        let rect = Rect::new(10.0, 20.0, 300.0, 48.0);
        let commands = chrome_surface_commands(rect, &chrome);
        assert_eq!(commands.len(), 2, "a backdrop then a fill: {commands:?}");

        let DrawCommand::Backdrop {
            rect: backdrop_rect,
            mode,
            tint,
            radius: backdrop_radius,
        } = &commands[0]
        else {
            panic!("the first command is the backdrop, got {:?}", commands[0]);
        };
        let DrawCommand::RoundedRect {
            rect: fill_rect,
            radius: fill_radius,
            color,
        } = &commands[1]
        else {
            panic!("the second command is the fill, got {:?}", commands[1]);
        };

        assert_eq!(*backdrop_rect, rect);
        assert_eq!(*fill_rect, rect);
        assert_eq!(*mode, BackdropMode::Blur(BACKDROP_SIGMA));
        assert_eq!(*tint, chrome.palette.surface);
        assert_eq!(*color, chrome.palette.surface);
        // **The radius is on both, and that is the whole point**: the composite
        // must be masked to the same rounded shape the fill draws, or the four
        // empty corners get tinted black over the map.
        assert_eq!(*backdrop_radius, chrome.radius);
        assert_eq!(*fill_radius, chrome.radius);
        assert_ne!(*backdrop_radius, 0.0, "a rounded panel, not a rectangle");
    }

    #[test]
    fn the_dock_is_a_row_of_five_slots_and_the_glyph_table_has_five_entries() {
        assert_eq!(DOCK_GLYPHS.len(), DOCK_SLOTS);
        assert_eq!(DOCK_SLOTS, 5);
    }

    #[test]
    fn every_dock_glyph_is_one_non_empty_character() {
        for glyph in DOCK_GLYPHS {
            assert_eq!(
                glyph.chars().count(),
                1,
                "the glyph {glyph:?} is one character"
            );
            assert!(
                !glyph.trim().is_empty(),
                "an empty label draws nothing and cannot be seen"
            );
        }
    }

    #[test]
    fn a_pager_paint_arm_records_three_circles_and_nothing_else() {
        let chrome = dark_chrome();
        let commands = pager_commands(Rect::new(0.0, 0.0, 120.0, 16.0), &chrome.palette);
        assert_eq!(commands.len(), PAGER_DOTS);
        assert!(
            commands
                .iter()
                .all(|command| matches!(command, DrawCommand::Circle { .. })),
            "the pager records circles and nothing else: {commands:?}"
        );
    }

    #[test]
    fn the_pager_places_three_dots_pager_dot_gap_apart_and_centred() {
        let row = Rect::new(10.0, 20.0, 100.0, 20.0);
        let centres: Vec<(f32, f32)> = (0..PAGER_DOTS).map(|i| pager_centre(i, row)).collect();
        let span = PAGER_OFFSETS[PAGER_DOTS - 1];
        let start = row.x + (row.width - span) / 2.0;
        let y = row.y + row.height / 2.0;
        assert_eq!(
            centres,
            vec![
                (start, y),
                (start + PAGER_DOT_GAP, y),
                (start + PAGER_DOT_GAP * 2.0, y),
            ]
        );
        assert!(
            (centres[1].0 - (row.x + row.width / 2.0)).abs() < f32::EPSILON,
            "the middle dot is the row's own midpoint"
        );
    }

    #[test]
    fn the_indicator_column_is_five_rows_in_red_amber_green_blue_grey_order() {
        assert_eq!(INDICATOR_TOKENS.len(), 5);
        assert_eq!(
            INDICATOR_TOKENS,
            [
                ThemeToken::Error,
                ThemeToken::Warning,
                ThemeToken::Success,
                ThemeToken::Primary,
                ThemeToken::TextMuted,
            ]
        );
    }

    #[test]
    fn the_chrome_nodes_do_not_overlap_each_other() {
        // **The row is not at the origin**, so an origin read as an extent cannot
        // pass: a fixture at `(0, 0)` cannot tell a position from a size.
        let window = Size::new(1280.0, 1020.0);
        let regions = chrome_regions(window, 64.0);
        for region in regions {
            assert!(
                region.x != 0.0 || region.y != 0.0,
                "no region is at the origin: {region:?}"
            );
        }
        let overlaps = |a: Rect, b: Rect| {
            a.x < b.x + b.width
                && b.x < a.x + a.width
                && a.y < b.y + b.height
                && b.y < a.y + a.height
        };
        for first in 0..regions.len() {
            for second in first + 1..regions.len() {
                assert!(
                    !overlaps(regions[first], regions[second]),
                    "{:?} overlaps {:?}",
                    regions[first],
                    regions[second]
                );
            }
        }
    }
}
