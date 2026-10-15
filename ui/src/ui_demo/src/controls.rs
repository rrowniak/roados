//! The Controls panel: the car overlay's category list and thirteen panes.
//!
//! This is `TASK_UI_DEMO_06`'s module: it fills the `DockPanel::Car` overlay
//! (an empty surface since `TASK_UI_DEMO_02`) with the Controls screen
//! `doc/ui/DEMO_APPLICATION.md` § *The Controls panel* specifies — a
//! scrollable category list on the left with a white-pill selection, and a
//! settings pane on the right. Three panes (**Controls**, **Lights**,
//! **Locks**) are built as specified; the other ten are stated proposals from
//! the section's four cell archetypes (segmented, tile, blue button, slider)
//! plus `Toggle`, every value mocked. No pointer event reaches the window on
//! this host, so `Q`/`W` move the selection, `B` actuates the selected pane's
//! primary control, and every rule is a unit test.
//!
//! Two deviations from the task file, both forced by the tree and recorded
//! here rather than made quietly:
//!
//! - **The tile block is three arithmetic rows, not three `Flex` rows.** A
//!   `Flex` parent lays its children out by flow and ignores a parked
//!   position, so tile buttons in real `Flex` rows could never be parked when
//!   another pane is selected — and twelve hidden panes' tiles would overlap
//!   the selected pane under `no_two_placed_rects_overlap`. [`tile_rect`]
//!   lays 4 + 3 + 3 out arithmetically instead; [`tile_rows`] is what says
//!   which row an index is in, and no grid layout mode is used anywhere in
//!   this file (task 52's grid mode is not built).
//! - **Hidden panes are parked, not hidden.** `touches` counts zero-size rects
//!   at one point as touching, and the overlap tests read layout rects rather
//!   than visibility, so a hidden pane must have *no shared* box, not an
//!   invisible one. [`ControlsPanes::relayout`] gives the selected pane its
//!   real boxes and parks every other pane's leaves as zero-size points on a
//!   diagonal no two share; the paint and hit-test arms read the same geometry
//!   functions, so the three can never disagree about where a control is.

use ui_core::arena::{Arena, Handle};
use ui_core::input::{InputEvent, InputEventKind};
use ui_core::layout::{mark_dirty, Constraints, LayoutState, Offset, Size};
use ui_core::node::{self, WidgetNode};
use ui_core::paint::{DrawCommand, Painter, Rect};
use ui_core::property::{Color, Property};
use ui_core::theme::{Theme, ThemeToken};
use ui_core::widgets::button::{Button, Palette as ButtonPalette};
use ui_core::widgets::label::{layout_text, LayoutOptions, TextAlign, Truncation, WrapMode};
use ui_core::widgets::list::{ItemFactory, List};
use ui_core::widgets::segmented::{Choice, Segmented};
use ui_core::widgets::slider::Slider;
use ui_core::widgets::toggle::Toggle;
use ui_core::widgets::Callback;

use super::{DemoLabel, TextMetrics};

/// How many settings categories the list holds.
pub const CATEGORY_COUNT: usize = 13;

/// How tall one category row is, in pixels.
pub const CATEGORY_ROW_HEIGHT: f32 = 56.0;

/// The list's width share of the overlay rect: a proposal, per the task.
pub const CONTROLS_LIST_FRACTION: f32 = 0.30;

/// The selection pill's corner radius, in pixels.
pub const PILL_RADIUS: f32 = 12.0;

/// The thirteen categories in the photograph's order.
pub const CATEGORIES: [&str; CATEGORY_COUNT] = [
    "Controls",
    "Dynamics",
    "Charging",
    "Autopilot",
    "Locks",
    "Lights",
    "Seats",
    "Display",
    "Schedule",
    "Safety",
    "Service",
    "Software",
    "Navigation",
];

/// The category carrying the `NEW` badge: Seats.
pub const CATEGORY_NEW: usize = 6;

/// Returns the category at `index`, or `None` past the end.
///
/// Public and exercised by the suite; the demo itself names categories through
/// [`CATEGORIES`] and never needs the lookup, so it is allowed to be unused in
/// a non-test build rather than deleted from the spec's surface.
#[must_use]
#[allow(dead_code)]
pub fn category_name(index: usize) -> Option<&'static str> {
    CATEGORIES.get(index).copied()
}

/// Returns the index of the category named `name`, case insensitively, or
/// `None` for a name no category has.
///
/// The reverse of [`category_name`], and the lookup a launch-time capture
/// affordance resolves a category name through.
#[must_use]
pub fn category_index(name: &str) -> Option<usize> {
    CATEGORIES
        .iter()
        .position(|category| category.eq_ignore_ascii_case(name))
}

/// The pad inside the overlay and the pane, in pixels.
const PAD: f32 = 12.0;

/// A standard pane slot's height, in pixels.
const SLOT_HEIGHT: f32 = 56.0;

/// The gap between stacked pane rows, in pixels.
const ROW_GAP: f32 = 12.0;

/// A tile's height, in pixels.
const TILE_HEIGHT: f32 = 52.0;

/// The gap between two tiles, in pixels.
const TILE_GAP: f32 = 10.0;

/// The header label's height, in pixels.
const HEADER_HEIGHT: f32 = 24.0;

/// How many tiles each of the three rows holds: 4 + 3 + 3.
const TILE_ROWS: [usize; 3] = [4, 3, 3];

/// Returns which tile row `index` (0..10) sits in, or `None` past the end.
#[must_use]
pub fn tile_row(index: usize) -> Option<usize> {
    let mut base = 0;
    for (row, count) in TILE_ROWS.iter().enumerate() {
        if index < base + count {
            return Some(row);
        }
        base += count;
    }
    None
}

/// Returns the rows of the tile block: each row's tile count, in order.
#[must_use]
#[allow(dead_code)]
pub fn tile_rows() -> [usize; 3] {
    TILE_ROWS
}

/// Returns the category list's rect inside `overlay`: the left
/// [`CONTROLS_LIST_FRACTION`] share, padded.
#[must_use]
pub fn category_list_rect(overlay: Rect) -> Rect {
    Rect::new(
        overlay.x + PAD,
        overlay.y + PAD,
        overlay.width * CONTROLS_LIST_FRACTION - PAD * 2.0,
        overlay.height - PAD * 2.0,
    )
}

/// Returns the settings pane's rect inside `overlay`: the right share.
#[must_use]
pub fn pane_rect(overlay: Rect) -> Rect {
    let list_width = overlay.width * CONTROLS_LIST_FRACTION;
    Rect::new(
        overlay.x + list_width + PAD,
        overlay.y + PAD,
        overlay.width - list_width - PAD * 2.0,
        overlay.height - PAD * 2.0,
    )
}

/// Returns the header label's rect at the top of `pane`.
#[must_use]
pub fn pane_header_rect(pane: Rect) -> Rect {
    Rect::new(
        pane.x + PAD,
        pane.y + 4.0,
        pane.width - PAD * 2.0,
        HEADER_HEIGHT,
    )
}

/// Returns standard pane slot `slot`'s rect (slots stack below the header).
#[must_use]
pub fn pane_slot_rect(pane: Rect, slot: usize) -> Rect {
    let top = pane.y + 4.0 + HEADER_HEIGHT + 10.0 + count_to_f32(slot) * (SLOT_HEIGHT + ROW_GAP);
    Rect::new(pane.x + PAD, top, pane.width - PAD * 2.0, SLOT_HEIGHT)
}

/// Returns the Controls pane's `row`th segmented rect (row 0 exterior, 1
/// wipers).
#[must_use]
pub fn controls_seg_rect(pane: Rect, row: usize) -> Rect {
    let top = pane.y + 4.0 + HEADER_HEIGHT + 10.0 + count_to_f32(row) * (44.0 + ROW_GAP);
    Rect::new(pane.x + PAD, top, pane.width - PAD * 2.0, 44.0)
}

/// Returns the top of the Controls tile block inside `pane`.
#[must_use]
pub fn controls_tiles_top(pane: Rect) -> f32 {
    pane.y + 4.0 + HEADER_HEIGHT + 10.0 + 2.0 * (44.0 + ROW_GAP) + 14.0
}

/// Returns tile `index`'s rect (0..10) inside `pane`: three arithmetic rows of
/// 4 + 3 + 3, never a grid.
#[must_use]
pub fn tile_rect(pane: Rect, index: usize) -> Option<Rect> {
    let row = tile_row(index)?;
    let count = TILE_ROWS[row];
    let mut base = 0;
    for count in TILE_ROWS.iter().take(row) {
        base += count;
    }
    let column = index - base;
    let inner = pane.width - PAD * 2.0;
    let width = (inner - count_to_f32(count - 1) * TILE_GAP) / count_to_f32(count);
    Some(Rect::new(
        pane.x + PAD + count_to_f32(column) * (width + TILE_GAP),
        controls_tiles_top(pane) + count_to_f32(row) * (TILE_HEIGHT + TILE_GAP),
        width,
        TILE_HEIGHT,
    ))
}

/// Returns the Controls fog button's rect inside `pane`.
#[must_use]
pub fn controls_fog_rect(pane: Rect) -> Rect {
    Rect::new(
        pane.x + PAD,
        controls_tiles_top(pane) + 3.0 * (TILE_HEIGHT + TILE_GAP) + 14.0,
        120.0,
        44.0,
    )
}

/// Returns the brightness row's rects inside `pane`: the label, the slider and
/// the blue `Auto` button.
#[must_use]
pub fn controls_brightness_rects(pane: Rect) -> (Rect, Rect, Rect) {
    let top = controls_tiles_top(pane) + 3.0 * (TILE_HEIGHT + TILE_GAP) + 14.0 + 44.0 + 14.0;
    let label = Rect::new(pane.x + PAD, top, 90.0, 44.0);
    let button = Rect::new(pane.x + pane.width - PAD - 90.0, top, 90.0, 44.0);
    let slider = Rect::new(
        label.x + label.width + TILE_GAP,
        top,
        button.x - (label.x + label.width + TILE_GAP) - TILE_GAP,
        44.0,
    );
    (label, slider, button)
}

/// Returns `count` as an `f32`, saturating at `u16::MAX`.
///
/// There is no `From<usize>` for `f32` in std, so this goes through `u16`
/// rather than an `as` cast. A pane with more than 65,535 rows is not a pane.
fn count_to_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

/// The category rows' font size, in pixels: a proposal.
const ROW_FONT_SIZE: f32 = 20.0;

/// The `NEW` badge's font size, in pixels: a proposal.
const BADGE_FONT_SIZE: f32 = 13.0;

/// The category list: thirteen rows, one pill, no glyphs.
///
/// The rows are text-only with the pill — the 33 Lucide masters cover dock
/// and chrome needs, not thirteen settings categories, so category glyphs are
/// an asset task. The pill adapts to both themes: its fill reads
/// [`ThemeToken::Text`]'s value and the row text [`ThemeToken::Background`]'s,
/// both at paint time.
pub struct CategoryList {
    list: List,
    selected: Property<usize>,
}

impl CategoryList {
    /// Creates the thirteen-row list in the arena, with `Controls` selected.
    ///
    /// The rows are bare nodes the [`List`] recycles for its scroll
    /// bookkeeping; the row texts are painted by [`CategoryList::paint`],
    /// which is why the factory builds nodes that carry no commands of their
    /// own.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>) -> Self {
        let mut list = List::new(nodes, CATEGORY_COUNT, CATEGORY_ROW_HEIGHT);
        list.set_item_factory(
            nodes,
            ItemFactory::new(|nodes: &mut Arena<WidgetNode>| {
                node::create(nodes, LayoutState::new())
            }),
        );
        CategoryList {
            list,
            selected: Property::new(0),
        }
    }

    /// Returns the list's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.list.handle()
    }

    /// Returns the selected category index.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.selected.get()
    }

    /// Selects `index`, and reports whether the selection moved.
    ///
    /// `false` past the end, clamping nothing.
    pub fn select(&self, index: usize) -> bool {
        if index >= CATEGORY_COUNT {
            return false;
        }
        if self.selected.get() == index {
            return false;
        }
        self.selected.set(index);
        true
    }

    /// Moves the selection by `delta`, clamping at both ends and never
    /// wrapping, and reports whether it moved.
    pub fn step(&self, delta: isize) -> bool {
        let current = isize::try_from(self.selected.get()).unwrap_or(0);
        let last = isize::try_from(CATEGORY_COUNT - 1).unwrap_or(0);
        let next = (current + delta).clamp(0, last);
        self.select(usize::try_from(next).unwrap_or(0))
    }

    /// Syncs the list's scroll bookkeeping against `rect`.
    pub fn sync(&mut self, nodes: &mut Arena<WidgetNode>, rect: Rect) {
        let _ = self.list.sync(nodes, rect);
    }

    /// Handles `event` inside `rect`: a tap selects the row under it through
    /// the list's own `item_at`, every other event travels on.
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        if !matches!(event.kind(), InputEventKind::Tap) {
            return false;
        }
        let Some(position) = event.position() else {
            return false;
        };
        if position.x < rect.x
            || position.x > rect.x + rect.width
            || position.y < rect.y
            || position.y > rect.y + rect.height
        {
            return false;
        }
        let Some(index) = self.list.item_at(position, rect) else {
            return false;
        };
        if !self.select(index) {
            return false;
        }
        event.consume();
        true
    }

    /// Returns the draw commands that paint the list within `rect`: the
    /// white pill behind the selected row's text, one text run per row, and
    /// the `NEW` badge on row 6.
    #[must_use]
    pub fn paint(
        &self,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
        theme: &Theme,
    ) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let pill_fill = token_color(theme, ThemeToken::Text);
        // **The selected row's text is `Background`, the rest are `Text`** —
        // the pill is `Text`'s value, so the row on it takes the colour that is
        // legible against it, and a row with no pill takes the ink the surface
        // is. The task's *"fill `Text`, row text `Background`"* is the pair the
        // pill and its own row swap; a list that drew **every** row in
        // `Background` would be dark-on-dark in the dark theme and invisible,
        // which a capture showed.
        let selected_ink = token_color(theme, ThemeToken::Background);
        let row_ink = token_color(theme, ThemeToken::Text);
        for (index, name) in CATEGORIES.iter().enumerate() {
            let selected = index == self.selected.get();
            // `item_rect` carries the scroll offset already: the row's `y` is
            // where the row is drawn, not where it sits in the content.
            let y = self.list.item_rect(index, rect).y;
            if selected {
                painter.rounded_rect(
                    Rect::new(rect.x + 4.0, y, rect.width - 8.0, CATEGORY_ROW_HEIGHT),
                    PILL_RADIUS,
                    pill_fill,
                );
            }
            let options = LayoutOptions {
                max_width: rect.width - 32.0,
                line_height: line_height.max(0.0),
                align: TextAlign::Left,
                wrap: WrapMode::None,
                truncation: Truncation::Ellipsis,
                ..LayoutOptions::default()
            };
            let layout = layout_text(name, &options, advance);
            let top = y + (CATEGORY_ROW_HEIGHT - layout.total_height) / 2.0;
            let ink = if selected { selected_ink } else { row_ink };
            for line in &layout.lines {
                painter.text(
                    rect.x + 16.0 + line.x_offset,
                    top,
                    &line.text,
                    ink,
                    ROW_FONT_SIZE,
                    0.0,
                );
            }
            if index == CATEGORY_NEW {
                painter.text(
                    rect.x + rect.width - 52.0,
                    top,
                    "NEW",
                    ink,
                    BADGE_FONT_SIZE,
                    0.0,
                );
            }
        }
        painter.finish()
    }
}

/// Returns the colour a theme holds for `token`, or white if it holds
/// something else.
///
/// White rather than black: the pill is drawn over a translucent surface over
/// a map, and an unreadable token must fail light rather than dark. It is the
/// button's own helper, repeated rather than imported: it is four lines, and
/// a shared module for one four-line helper is a module.
fn token_color(theme: &Theme, token: ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(255, 255, 255, 255))
}

/// Builds the palette a segmented row in a pane is drawn in: the track and
/// the ring read the chrome ink, the selected column reads the pill —
///
/// **deliberately not [`Primary`](ThemeToken::Primary).** The solid-blue
/// action buttons fill `Primary` with `OnPrimary` text, and a selected segment
/// in the same pair would read as a button. The segment reads `Text` with
/// `Background` text instead, so the two never agree.
fn segmented_palette(theme: &Theme) -> ui_core::widgets::segmented::Palette {
    let mut palette = ui_core::widgets::segmented::Palette::from_theme(theme);
    palette.selected_fill = token_color(theme, ThemeToken::Text);
    palette.selected_text = token_color(theme, ThemeToken::Background);
    palette
}

/// The values [`ControlsPanes::sync_demo`] writes labels from, as one
/// comparable record.
///
/// `PartialEq` and `Copy`, so the caller can hold the last one and call the
/// sync only when a write moved something.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlsState {
    /// Whether `Demo::locked` is set.
    pub locked: bool,
    /// The Sentry state, shared by the tile and the Safety toggle.
    pub sentry: bool,
    /// The child-lock state, shared by both tiles.
    pub child: bool,
    /// The driver heater level.
    pub seat_driver: u8,
    /// The passenger heater level.
    pub seat_passenger: u8,
    /// The charge-limit percentage the readout names.
    pub charge: f32,
    /// Screen Clean Mode.
    pub screen_clean: bool,
    /// Car Wash Mode.
    pub car_wash: bool,
    /// Tow Mode.
    pub tow: bool,
    /// Whether the update check has run.
    pub check: bool,
}

/// The thirteen panes' widgets and the shared state between them.
///
/// Every value is mocked (the `TEMP_INIT` precedent). Three panes are built
/// as specified — **Controls**, **Lights**, **Locks** — and the other ten are
/// stated proposals from the four cell archetypes plus `Toggle`.
///
/// Shared state, not duplicated state: the fog buttons in Controls and Lights
/// flip one [`ControlsPanes::fog_on`]; the door-lock toggle writes
/// `Demo::locked` itself; the Sentry tile and the Sentry toggle flip one
/// [`ControlsPanes::sentry_on`].
#[allow(clippy::too_many_lines)]
pub struct ControlsPanes {
    /// One state for the Sentry tile and the Safety toggle.
    pub sentry_on: Property<bool>,
    /// One write path for both fog buttons.
    #[allow(dead_code)]
    pub fog_on: Property<bool>,
    /// One write path for both child-lock tiles.
    pub child_on: Property<bool>,
    /// Two seat heaters, three levels each (`0` is off).
    pub seat_driver: Property<u8>,
    /// Two seat heaters, three levels each (`0` is off).
    pub seat_passenger: Property<u8>,
    /// Screen Clean Mode's own state, shown in its label.
    pub screen_clean_on: Property<bool>,
    /// Car Wash Mode's own state, shown in its label.
    pub car_wash_on: Property<bool>,
    /// Tow Mode's own state, shown in its label.
    pub tow_on: Property<bool>,
    /// Whether the update check has run; the button reads `Up to date` after.
    pub check_done: Property<bool>,
    /// The Controls exterior-lights row.
    pub controls_ext: Segmented,
    /// The Controls wiper row.
    pub controls_wipers: Segmented,
    /// The ten action tiles, in 4 + 3 + 3 row order.
    pub tiles: Vec<Button>,
    /// The Controls blue fog button.
    pub controls_fog: Button,
    /// The brightness slider.
    pub brightness: Slider,
    /// The brightness blue `Auto` button.
    pub brightness_auto: Button,
    /// The Lights exterior row: its own value, same four choices.
    pub lights_ext: Segmented,
    /// The Lights blue fog button: the fog path's second writer.
    pub lights_fog: Button,
    /// Auto high beam, mocked off.
    pub high_beam: Toggle,
    /// The door lock: writes `Demo::locked`, the status-bar lock's property.
    pub locks_lock: Toggle,
    /// Walk-away lock, mocked on.
    pub walk_away: Toggle,
    /// The Locks child-lock tile: the child path's second writer.
    pub locks_child: Button,
    /// Acceleration: Chill, Standard.
    pub accel: Segmented,
    /// Steering: Comfort, Standard, Sport.
    pub steering: Segmented,
    /// Slip Start, off.
    pub slip: Toggle,
    /// The charge-limit slider, 50–100%.
    pub charge_limit: Slider,
    /// Scheduled charging, off.
    pub sched_charge: Toggle,
    /// Autosteer, mocked on.
    pub autosteer: Toggle,
    /// Navigate on Autopilot, off.
    pub navigate_ap: Toggle,
    /// Follow distance, seven points.
    pub follow: Segmented,
    /// The driver heater button.
    pub seat_driver_btn: Button,
    /// The passenger heater button.
    pub seat_passenger_btn: Button,
    /// Easy Entry, mocked on.
    pub easy_entry: Toggle,
    /// Appearance: Auto, Day, Night.
    pub appearance: Segmented,
    /// Screen Clean Mode's blue button.
    pub screen_clean_btn: Button,
    /// 24-hour time, off.
    pub clock_24: Toggle,
    /// Precondition for departure, off.
    pub precondition: Toggle,
    /// Off-peak charging, off.
    pub off_peak: Toggle,
    /// The Safety Sentry toggle: the sentry path's second writer.
    pub safety_sentry: Toggle,
    /// PIN to Drive, off.
    pub pin_drive: Toggle,
    /// Speed Limit Mode, off.
    pub speed_limit: Toggle,
    /// Car Wash Mode's blue button.
    pub wash_btn: Button,
    /// Tow Mode's blue button.
    pub tow_btn: Button,
    /// Check for updates' blue button.
    pub check_btn: Button,
    /// The advanced-update toggle, off.
    pub advanced_updates: Toggle,
    /// Online Routing, mocked on.
    pub online_routing: Toggle,
    /// Trip Planner, mocked on.
    pub trip_planner: Toggle,
    /// The voice-guidance slider, 0–10.
    pub voice: Slider,
    /// The thirteen header labels, one per pane.
    pub headers: Vec<DemoLabel>,
    /// The charge-limit readout beside its slider.
    pub charge_label: DemoLabel,
    /// The mocked departure time: editing a time needs text entry, a later
    /// task's item.
    pub schedule_time: DemoLabel,
    /// The mocked odometer.
    pub odometer: DemoLabel,
    /// The mocked software version.
    pub version: DemoLabel,
    /// Every leaf in a fixed order, for parking and for the placed-rows
    /// assertion. The list node is not here; it is [`CategoryList`]'s.
    parked: Vec<Handle>,
}

/// The ten action tiles' placed names, in 4 + 3 + 3 row order.
#[cfg(test)]
const TILE_NAMES: [&str; 10] = [
    "controls tile fold mirrors",
    "controls tile child lock",
    "controls tile window lock",
    "controls tile glovebox",
    "controls tile mirrors",
    "controls tile recording",
    "controls tile car wash",
    "controls tile steering",
    "controls tile sentry",
    "controls tile neutral",
];

/// The thirteen pane headers' placed names, in category order.
#[cfg(test)]
const HEADER_NAMES: [&str; CATEGORY_COUNT] = [
    "controls header controls",
    "controls header dynamics",
    "controls header charging",
    "controls header autopilot",
    "controls header locks",
    "controls header lights",
    "controls header seats",
    "controls header display",
    "controls header schedule",
    "controls header safety",
    "controls header service",
    "controls header software",
    "controls header navigation",
];

/// The tile labels in 4 + 3 + 3 row order: the first row of four, then three,
/// then three. Stateful tiles carry their secondary line after a newline and
/// never a third widget.
const TILE_LABELS: [&str; 10] = [
    "Fold\nMirrors",
    "Child Lock\noff",
    "Window\nLock",
    "Glovebox",
    "Mirrors",
    "Recording\noff",
    "Car Wash",
    "Steering",
    "Sentry\noff",
    "Neutral\nHold",
];

/// Cycles a seat heater `0 → 1 → 2 → 3 → 0`: three levels per the manual's
/// seat widget — a demo-local function, since `dock::cycle_heater` tops out
/// at 2.
#[must_use]
pub fn seat_cycle(current: u8) -> u8 {
    match current {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 0,
    }
}

/// Returns a heater button's label for `level`: `"Driver / Off"` through
/// `"Driver / Level 3"`.
#[must_use]
pub fn seat_label(side: &str, level: u8) -> String {
    if level == 0 {
        format!("{side} / Off")
    } else {
        format!("{side} / Level {level}")
    }
}

impl ControlsPanes {
    /// Builds all thirteen panes' widgets in the arena, mocked throughout.
    ///
    /// Nothing is attached here: the caller parents every leaf under the Car
    /// overlay panel's node and calls [`ControlsPanes::relayout`]. `locked`
    /// is `Demo::locked` itself — the status-bar lock's property — so the
    /// Locks toggle and the bar flip one state.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(nodes: &mut Arena<WidgetNode>, theme: &Theme, locked: Property<bool>) -> Self {
        let _ = locked;
        let seg_palette = segmented_palette(theme);
        let exterior = |nodes: &mut Arena<WidgetNode>, labels: Vec<Choice>, initial: f32| {
            let mut segmented = Segmented::new(nodes, labels);
            segmented.set_palette(seg_palette);
            segmented.value.set(initial);
            segmented
        };
        let blue = |nodes: &mut Arena<WidgetNode>, text: &str| {
            let mut button = Button::new(nodes, text);
            button.set_palette(ButtonPalette::from_theme(theme));
            button.snap_to_state();
            button
        };
        let tile = |nodes: &mut Arena<WidgetNode>, text: &str| {
            let mut button = Button::new(nodes, text);
            button.set_palette(ButtonPalette::default());
            button.snap_to_state();
            button
        };
        let toggle = |nodes: &mut Arena<WidgetNode>, checked: bool| {
            let mut widget = Toggle::new(nodes);
            widget.set_palette(ui_core::widgets::toggle::Palette::from_theme(theme));
            widget.checked.set(checked);
            widget.snap_to_state();
            widget
        };
        let slider = |nodes: &mut Arena<WidgetNode>, min: f32, max: f32, initial: f32| {
            let mut widget = Slider::new(nodes, min, max);
            widget.set_palette(ui_core::widgets::slider::Palette::from_theme(theme));
            widget.value.set(initial);
            widget.thumb.set(initial);
            widget.snap_to_state();
            widget
        };

        let sentry_on = Property::new(false);
        let fog_on = Property::new(false);
        let child_on = Property::new(false);
        let seat_driver = Property::new(0);
        let seat_passenger = Property::new(0);
        let screen_clean_on = Property::new(false);
        let car_wash_on = Property::new(false);
        let tow_on = Property::new(false);
        let check_done = Property::new(false);

        let controls_ext = exterior(
            nodes,
            vec![
                Choice::new("Off", 0.0),
                Choice::new("Parking", 1.0),
                Choice::new("On", 2.0),
                Choice::new("Auto", 3.0),
            ],
            3.0,
        );
        let controls_wipers = exterior(
            nodes,
            vec![
                Choice::new("Off", 0.0),
                Choice::new("Auto", 1.0),
                Choice::new("I", 2.0),
                Choice::new("II", 3.0),
                Choice::new("III", 4.0),
                Choice::new("IIII", 5.0),
            ],
            1.0,
        );
        let mut tiles: Vec<Button> = Vec::with_capacity(TILE_LABELS.len());
        for label in TILE_LABELS {
            tiles.push(tile(nodes, label));
        }
        // The two shared write paths, wired after both writers exist: each
        // closure flips one property and refreshes both labels, so the two
        // tiles cannot disagree about the state they share.
        {
            let child = child_on.clone();
            let first = tiles[1].label.clone();
            let sentry = sentry_on.clone();
            let ninth = tiles[8].label.clone();
            let recording = tiles[5].label.clone();
            tiles[1].on_click = Callback::new(move || {
                child.set(!child.get());
                let state = if child.get() { "on" } else { "off" };
                first.set(format!("Child Lock\n{state}"));
            });
            tiles[8].on_click = Callback::new(move || {
                sentry.set(!sentry.get());
                let state = if sentry.get() { "on" } else { "off" };
                ninth.set(format!("Sentry\n{state}"));
                recording.set(if sentry.get() {
                    "Recording\n●".to_string()
                } else {
                    "Recording\noff".to_string()
                });
            });
        }
        let mut controls_fog = blue(nodes, "Fog");
        {
            let fog = fog_on.clone();
            controls_fog.on_click = Callback::new(move || fog.set(!fog.get()));
        }
        let mut brightness = slider(nodes, 0.0, 100.0, 80.0);
        brightness.set_step(Some(1.0));
        let brightness_auto = blue(nodes, "Auto");

        let lights_ext = exterior(
            nodes,
            vec![
                Choice::new("Off", 0.0),
                Choice::new("Parking", 1.0),
                Choice::new("On", 2.0),
                Choice::new("Auto", 3.0),
            ],
            3.0,
        );
        let mut lights_fog = blue(nodes, "Fog");
        {
            let fog = fog_on.clone();
            lights_fog.on_click = Callback::new(move || fog.set(!fog.get()));
        }
        let high_beam = toggle(nodes, false);

        // The door-lock toggle writes `Demo::locked` on tap rather than its
        // own state: one state, two writers, and the frame syncs the toggle
        // back. Its own `on_event` is never called; see `press_locks_toggle`.
        let locks_lock = toggle(nodes, false);
        let walk_away = toggle(nodes, true);
        let mut locks_child = tile(nodes, "Child Lock\noff");
        {
            let child = child_on.clone();
            let locks = locks_child.label.clone();
            locks_child.on_click = Callback::new(move || {
                child.set(!child.get());
                let state = if child.get() { "on" } else { "off" };
                locks.set(format!("Child Lock\n{state}"));
            });
        }

        let accel = exterior(
            nodes,
            vec![Choice::new("Chill", 0.0), Choice::new("Standard", 1.0)],
            1.0,
        );
        let steering = exterior(
            nodes,
            vec![
                Choice::new("Comfort", 0.0),
                Choice::new("Standard", 1.0),
                Choice::new("Sport", 2.0),
            ],
            1.0,
        );
        let slip = toggle(nodes, false);

        let mut charge_limit = slider(nodes, 50.0, 100.0, 80.0);
        charge_limit.set_step(Some(1.0));
        let sched_charge = toggle(nodes, false);

        let autosteer = toggle(nodes, true);
        let navigate_ap = toggle(nodes, false);
        let follow = exterior(
            nodes,
            vec![
                Choice::new("1", 0.0),
                Choice::new("2", 1.0),
                Choice::new("3", 2.0),
                Choice::new("4", 3.0),
                Choice::new("5", 4.0),
                Choice::new("6", 5.0),
                Choice::new("7", 6.0),
            ],
            2.0,
        );

        let mut seat_driver_btn = tile(nodes, &seat_label("Driver", 0));
        {
            let level = seat_driver.clone();
            let label = seat_driver_btn.label.clone();
            seat_driver_btn.on_click = Callback::new(move || {
                level.set(seat_cycle(level.get()));
                label.set(seat_label("Driver", level.get()));
            });
        }
        let mut seat_passenger_btn = tile(nodes, &seat_label("Passenger", 0));
        {
            let level = seat_passenger.clone();
            let label = seat_passenger_btn.label.clone();
            seat_passenger_btn.on_click = Callback::new(move || {
                level.set(seat_cycle(level.get()));
                label.set(seat_label("Passenger", level.get()));
            });
        }
        let easy_entry = toggle(nodes, true);

        let appearance = exterior(
            nodes,
            vec![
                Choice::new("Auto", 0.0),
                Choice::new("Day", 1.0),
                Choice::new("Night", 2.0),
            ],
            0.0,
        );
        let mut screen_clean_btn = blue(nodes, "Screen Clean\nOff");
        {
            let clean = screen_clean_on.clone();
            let label = screen_clean_btn.label.clone();
            screen_clean_btn.on_click = Callback::new(move || {
                clean.set(!clean.get());
                let state = if clean.get() { "On" } else { "Off" };
                label.set(format!("Screen Clean\n{state}"));
            });
        }
        let clock_24 = toggle(nodes, false);

        let precondition = toggle(nodes, false);
        let off_peak = toggle(nodes, false);

        let safety_sentry = toggle(nodes, false);
        let pin_drive = toggle(nodes, false);
        let speed_limit = toggle(nodes, false);

        let mut wash_btn = blue(nodes, "Car Wash\nOff");
        {
            let wash = car_wash_on.clone();
            let label = wash_btn.label.clone();
            wash_btn.on_click = Callback::new(move || {
                wash.set(!wash.get());
                let state = if wash.get() { "On" } else { "Off" };
                label.set(format!("Car Wash\n{state}"));
            });
        }
        let mut tow_btn = blue(nodes, "Tow Mode\nOff");
        {
            let tow = tow_on.clone();
            let label = tow_btn.label.clone();
            tow_btn.on_click = Callback::new(move || {
                tow.set(!tow.get());
                let state = if tow.get() { "On" } else { "Off" };
                label.set(format!("Tow Mode\n{state}"));
            });
        }

        let mut check_btn = blue(nodes, "Check for updates");
        {
            let done = check_done.clone();
            let label = check_btn.label.clone();
            check_btn.on_click = Callback::new(move || {
                done.set(true);
                label.set("Up to date".to_string());
            });
        }
        let advanced_updates = toggle(nodes, false);

        let online_routing = toggle(nodes, true);
        let trip_planner = toggle(nodes, true);
        let voice = slider(nodes, 0.0, 10.0, 5.0);

        let header = |nodes: &mut Arena<WidgetNode>, text: &str| DemoLabel {
            label: ui_core::widgets::label::Label::new(nodes, text),
            options: LayoutOptions {
                max_width: 600.0,
                ..LayoutOptions::default()
            },
        };
        let mut headers: Vec<DemoLabel> = Vec::with_capacity(CATEGORIES.len());
        for name in CATEGORIES {
            headers.push(header(nodes, name));
        }
        let charge_label = header(nodes, "80%");
        let schedule_time = header(nodes, "Weekdays 07:30");
        let odometer = header(nodes, "12,480 mi");
        let version = header(nodes, "2026.20.1");

        let mut parked = Vec::new();
        let mut leaf = |handle: Handle| parked.push(handle);
        leaf(controls_ext.handle());
        leaf(controls_wipers.handle());
        for button in &tiles {
            leaf(button.handle());
        }
        leaf(controls_fog.handle());
        leaf(brightness.handle());
        leaf(brightness_auto.handle());
        leaf(lights_ext.handle());
        leaf(lights_fog.handle());
        leaf(high_beam.handle());
        leaf(locks_lock.handle());
        leaf(walk_away.handle());
        leaf(locks_child.handle());
        leaf(accel.handle());
        leaf(steering.handle());
        leaf(slip.handle());
        leaf(charge_limit.handle());
        leaf(sched_charge.handle());
        leaf(autosteer.handle());
        leaf(navigate_ap.handle());
        leaf(follow.handle());
        leaf(seat_driver_btn.handle());
        leaf(seat_passenger_btn.handle());
        leaf(easy_entry.handle());
        leaf(appearance.handle());
        leaf(screen_clean_btn.handle());
        leaf(clock_24.handle());
        leaf(precondition.handle());
        leaf(off_peak.handle());
        leaf(safety_sentry.handle());
        leaf(pin_drive.handle());
        leaf(speed_limit.handle());
        leaf(wash_btn.handle());
        leaf(tow_btn.handle());
        leaf(check_btn.handle());
        leaf(advanced_updates.handle());
        leaf(online_routing.handle());
        leaf(trip_planner.handle());
        leaf(voice.handle());
        for demo_label in headers
            .iter()
            .chain([&charge_label, &schedule_time, &odometer, &version])
        {
            leaf(demo_label.label.handle());
        }

        ControlsPanes {
            sentry_on,
            fog_on,
            child_on,
            seat_driver,
            seat_passenger,
            screen_clean_on,
            car_wash_on,
            tow_on,
            check_done,
            controls_ext,
            controls_wipers,
            tiles,
            controls_fog,
            brightness,
            brightness_auto,
            lights_ext,
            lights_fog,
            high_beam,
            locks_lock,
            walk_away,
            locks_child,
            accel,
            steering,
            slip,
            charge_limit,
            sched_charge,
            autosteer,
            navigate_ap,
            follow,
            seat_driver_btn,
            seat_passenger_btn,
            easy_entry,
            appearance,
            screen_clean_btn,
            clock_24,
            precondition,
            off_peak,
            safety_sentry,
            pin_drive,
            speed_limit,
            wash_btn,
            tow_btn,
            check_btn,
            advanced_updates,
            online_routing,
            trip_planner,
            voice,
            headers,
            charge_label,
            schedule_time,
            odometer,
            version,
            parked,
        }
    }

    /// Returns every leaf in the fixed parking order.
    #[must_use]
    pub fn leaves(&self) -> &[Handle] {
        &self.parked
    }

    /// Lays the selected pane out in the pane area and parks every other
    /// pane's leaves as zero-size points no two share.
    ///
    /// Parked points run diagonally from the pane area's bottom-right corner:
    /// inside the window, inside the overlay, and disjoint from the list, the
    /// fps readout and each other. Call after every selection change and once
    /// at construction; the frame's layout pass applies it. The boxes written
    /// are [`ControlsPanes::layout_of`]'s, so this cannot disagree with the
    /// paint and hit-test arms.
    pub fn relayout(&self, nodes: &mut Arena<WidgetNode>, overlay: Rect, selected: usize) {
        let pane = pane_rect(overlay);
        let mut slot = 0;
        for (handle, rect) in self.layout_of(overlay, selected) {
            match rect {
                Some(rect) => place_leaf(nodes, handle, rect),
                None => park_leaf(nodes, pane, handle, &mut slot),
            }
        }
    }

    /// Returns the pane `handle` belongs to, or `None` for a handle this
    /// panel does not own.
    #[must_use]
    pub fn header_of(&self, handle: Handle) -> Option<usize> {
        self.headers
            .iter()
            .position(|header| header.label.handle() == handle)
    }

    /// Returns every pane leaf with the rect it holds while `selected` is
    /// on: `Some` for the selected pane's boxes, `None` for a parked leaf.
    ///
    /// The one mapping the three readers share — [`ControlsPanes::relayout`]
    /// writes it into the tree, the paint arm and the hit-test arm read the
    /// same numbers back — so a control is never drawn in one place and hit
    /// in another. Headers are parked leaves here; their painted rect is
    /// [`pane_header_rect`], read by the paint arm alone.
    #[must_use]
    pub fn layout_of(&self, overlay: Rect, selected: usize) -> Vec<(Handle, Option<Rect>)> {
        let pane = pane_rect(overlay);
        let mut out = Vec::with_capacity(self.parked.len());
        let mut one = |handle: Handle, rect: Option<Rect>| out.push((handle, rect));
        let live = |index: usize| selected == index;
        one(
            self.controls_ext.handle(),
            live(0).then(|| controls_seg_rect(pane, 0)),
        );
        one(
            self.controls_wipers.handle(),
            live(0).then(|| controls_seg_rect(pane, 1)),
        );
        for (index, button) in self.tiles.iter().enumerate() {
            one(
                button.handle(),
                live(0).then(|| tile_rect(pane, index)).flatten(),
            );
        }
        one(
            self.controls_fog.handle(),
            live(0).then(|| controls_fog_rect(pane)),
        );
        let (_, slider_rect, auto_rect) = controls_brightness_rects(pane);
        one(self.brightness.handle(), live(0).then_some(slider_rect));
        one(self.brightness_auto.handle(), live(0).then_some(auto_rect));
        let panes: [(usize, [Handle; 3]); 12] = [
            (
                5,
                [
                    self.lights_ext.handle(),
                    self.lights_fog.handle(),
                    self.high_beam.handle(),
                ],
            ),
            (
                4,
                [
                    self.locks_lock.handle(),
                    self.walk_away.handle(),
                    self.locks_child.handle(),
                ],
            ),
            (
                1,
                [
                    self.accel.handle(),
                    self.steering.handle(),
                    self.slip.handle(),
                ],
            ),
            (
                2,
                [
                    self.charge_limit.handle(),
                    self.sched_charge.handle(),
                    self.charge_label.label.handle(),
                ],
            ),
            (
                3,
                [
                    self.autosteer.handle(),
                    self.navigate_ap.handle(),
                    self.follow.handle(),
                ],
            ),
            (
                6,
                [
                    self.seat_driver_btn.handle(),
                    self.seat_passenger_btn.handle(),
                    self.easy_entry.handle(),
                ],
            ),
            (
                7,
                [
                    self.appearance.handle(),
                    self.screen_clean_btn.handle(),
                    self.clock_24.handle(),
                ],
            ),
            (
                8,
                [
                    self.precondition.handle(),
                    self.off_peak.handle(),
                    self.schedule_time.label.handle(),
                ],
            ),
            (
                9,
                [
                    self.safety_sentry.handle(),
                    self.pin_drive.handle(),
                    self.speed_limit.handle(),
                ],
            ),
            (
                10,
                [
                    self.wash_btn.handle(),
                    self.tow_btn.handle(),
                    self.odometer.label.handle(),
                ],
            ),
            (
                11,
                [
                    self.check_btn.handle(),
                    self.advanced_updates.handle(),
                    self.version.label.handle(),
                ],
            ),
            (
                12,
                [
                    self.online_routing.handle(),
                    self.trip_planner.handle(),
                    self.voice.handle(),
                ],
            ),
        ];
        for (index, handles) in panes {
            for (position, handle) in handles.iter().enumerate() {
                one(*handle, live(index).then(|| pane_slot_rect(pane, position)));
            }
        }
        for demo_label in &self.headers {
            one(demo_label.label.handle(), None);
        }
        out
    }

    /// Returns the draw commands that paint `handle`'s widget at `rect`,
    /// or `None` for a handle this panel does not own.
    ///
    /// The caller gates on the open panel and the paint gate on the page;
    /// this answers only what the widget draws. Headers are not here — see
    /// [`ControlsPanes::paint_header`], which paints them at an explicit
    /// rect — but the static slot labels are, at their own slot's rect.
    #[must_use]
    pub fn paint_leaf(
        &self,
        handle: Handle,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
        metrics: &TextMetrics,
    ) -> Option<Vec<DrawCommand>> {
        for segmented in [
            &self.controls_ext,
            &self.controls_wipers,
            &self.lights_ext,
            &self.accel,
            &self.steering,
            &self.follow,
            &self.appearance,
        ] {
            if segmented.handle() == handle {
                return Some(segmented.paint(rect, advance, line_height));
            }
        }
        for button in self.tiles.iter().chain([
            &self.controls_fog,
            &self.brightness_auto,
            &self.lights_fog,
            &self.locks_child,
            &self.seat_driver_btn,
            &self.seat_passenger_btn,
            &self.screen_clean_btn,
            &self.wash_btn,
            &self.tow_btn,
            &self.check_btn,
        ]) {
            if button.handle() == handle {
                return Some(button.paint(rect, advance, line_height));
            }
        }
        for toggle in [
            &self.high_beam,
            &self.locks_lock,
            &self.walk_away,
            &self.slip,
            &self.sched_charge,
            &self.autosteer,
            &self.navigate_ap,
            &self.easy_entry,
            &self.clock_24,
            &self.precondition,
            &self.off_peak,
            &self.safety_sentry,
            &self.pin_drive,
            &self.speed_limit,
            &self.advanced_updates,
            &self.online_routing,
            &self.trip_planner,
        ] {
            if toggle.handle() == handle {
                return Some(toggle.paint(rect));
            }
        }
        for slider in [&self.brightness, &self.charge_limit, &self.voice] {
            if slider.handle() == handle {
                return Some(slider.paint(rect));
            }
        }
        for demo_label in [
            &self.charge_label,
            &self.schedule_time,
            &self.odometer,
            &self.version,
        ] {
            if demo_label.label.handle() == handle {
                return Some(paint_demo_label(
                    demo_label,
                    metrics,
                    STATIC_FONT_SIZE,
                    rect,
                ));
            }
        }
        None
    }

    /// Returns the draw commands that paint pane `index`'s header.
    #[must_use]
    pub fn paint_header(
        &self,
        index: usize,
        overlay: Rect,
        metrics: &TextMetrics,
    ) -> Vec<DrawCommand> {
        let Some(header) = self.headers.get(index) else {
            return Vec::new();
        };
        paint_demo_label(
            header,
            metrics,
            HEADER_FONT_SIZE,
            pane_header_rect(pane_rect(overlay)),
        )
    }

    /// Offers `event` to one pane leaf at its box: the two bound toggles write
    /// shared state — `Demo::locked` and `sentry_on` — through the same setters
    /// the `B` key uses, and every other control is offered to itself.
    ///
    /// **The one place a routed handle becomes a pane action**, so the two
    /// bound toggles and every other pane control cannot disagree about which
    /// property a toggle writes or how a control is offered.
    pub fn offer_handle(
        &self,
        event: &mut InputEvent,
        handle: Handle,
        rect: Rect,
        locked: &Property<bool>,
    ) -> bool {
        if handle == self.locks_lock.handle() {
            self.press_locks_toggle(locked);
            event.consume();
            return true;
        }
        if handle == self.safety_sentry.handle() {
            self.press_safety_sentry();
            event.consume();
            return true;
        }
        self.offer_widget(event, handle, rect)
    }

    /// Offers `event` to the widget at `handle`, or declines for a handle
    /// that is a label rather than a control.
    fn offer_widget(&self, event: &mut InputEvent, handle: Handle, rect: Rect) -> bool {
        for segmented in [
            &self.controls_ext,
            &self.controls_wipers,
            &self.lights_ext,
            &self.accel,
            &self.steering,
            &self.follow,
            &self.appearance,
        ] {
            if segmented.handle() == handle {
                return segmented.on_event(event, rect);
            }
        }
        for button in self.tiles.iter().chain([
            &self.controls_fog,
            &self.brightness_auto,
            &self.lights_fog,
            &self.locks_child,
            &self.seat_driver_btn,
            &self.seat_passenger_btn,
            &self.screen_clean_btn,
            &self.wash_btn,
            &self.tow_btn,
            &self.check_btn,
        ]) {
            if button.handle() == handle {
                return button.on_event(event);
            }
        }
        for toggle in [
            &self.high_beam,
            &self.locks_lock,
            &self.walk_away,
            &self.slip,
            &self.sched_charge,
            &self.autosteer,
            &self.navigate_ap,
            &self.easy_entry,
            &self.clock_24,
            &self.precondition,
            &self.off_peak,
            &self.safety_sentry,
            &self.pin_drive,
            &self.speed_limit,
            &self.advanced_updates,
            &self.online_routing,
            &self.trip_planner,
        ] {
            if toggle.handle() == handle {
                return toggle.on_event(event, rect);
            }
        }
        for slider in [&self.brightness, &self.charge_limit, &self.voice] {
            if slider.handle() == handle {
                return slider.on_event(event, rect);
            }
        }
        false
    }

    /// Returns every leaf with the name [`Demo::placed_handles`] gives it, in
    /// one fixed order.
    ///
    /// The demo's `assert_placed_handles_is_complete` compares a sorted answer
    /// against [`Demo::expected_placed_rect_names`], so the names here and the
    /// names there must agree with multiplicity. The order is the parking
    /// order, so a leaf added here without a matching name there is a failure
    /// rather than a silent row.
    #[must_use]
    #[cfg(test)]
    pub fn placed_names(&self) -> Vec<(&'static str, Handle)> {
        let mut names: Vec<(&'static str, Handle)> = vec![
            ("controls exterior", self.controls_ext.handle()),
            ("controls wipers", self.controls_wipers.handle()),
        ];
        for (index, button) in self.tiles.iter().enumerate() {
            // Ten names written out, so a tile that is dropped or reordered
            // changes the sorted list the assertion compares.
            names.push((TILE_NAMES[index], button.handle()));
        }
        names.extend([
            ("controls fog", self.controls_fog.handle()),
            ("controls brightness", self.brightness.handle()),
            ("controls brightness auto", self.brightness_auto.handle()),
            ("controls lights exterior", self.lights_ext.handle()),
            ("controls lights fog", self.lights_fog.handle()),
            ("controls auto high beam", self.high_beam.handle()),
            ("controls lock", self.locks_lock.handle()),
            ("controls walk-away", self.walk_away.handle()),
            ("controls locks child", self.locks_child.handle()),
            ("controls acceleration", self.accel.handle()),
            ("controls steering", self.steering.handle()),
            ("controls slip start", self.slip.handle()),
            ("controls charge limit", self.charge_limit.handle()),
            ("controls scheduled charging", self.sched_charge.handle()),
            ("controls autosteer", self.autosteer.handle()),
            ("controls navigate on autopilot", self.navigate_ap.handle()),
            ("controls follow distance", self.follow.handle()),
            ("controls driver heater", self.seat_driver_btn.handle()),
            (
                "controls passenger heater",
                self.seat_passenger_btn.handle(),
            ),
            ("controls easy entry", self.easy_entry.handle()),
            ("controls appearance", self.appearance.handle()),
            ("controls screen clean", self.screen_clean_btn.handle()),
            ("controls 24-hour", self.clock_24.handle()),
            ("controls precondition", self.precondition.handle()),
            ("controls off-peak", self.off_peak.handle()),
            ("controls safety sentry", self.safety_sentry.handle()),
            ("controls pin to drive", self.pin_drive.handle()),
            ("controls speed limit", self.speed_limit.handle()),
            ("controls car wash", self.wash_btn.handle()),
            ("controls tow mode", self.tow_btn.handle()),
            ("controls check updates", self.check_btn.handle()),
            ("controls advanced updates", self.advanced_updates.handle()),
            ("controls online routing", self.online_routing.handle()),
            ("controls trip planner", self.trip_planner.handle()),
            ("controls voice guidance", self.voice.handle()),
            ("controls charge readout", self.charge_label.label.handle()),
            ("controls schedule time", self.schedule_time.label.handle()),
            ("controls odometer", self.odometer.label.handle()),
            ("controls version", self.version.label.handle()),
        ]);
        for (index, header) in self.headers.iter().enumerate() {
            names.push((HEADER_NAMES[index], header.label.handle()));
        }
        names
    }

    /// Returns every leaf with an empty label and a box for the demo's
    /// completeness assertion, in [`ControlsPanes::placed_names`] order.
    ///
    /// The same list, reduced to names, is what `expected_placed_rect_names`
    /// must carry; this is the demo-side accessor so the two cannot be built on
    /// two orderings.
    #[must_use]
    #[cfg(test)]
    pub fn placed_name_list() -> Vec<&'static str> {
        let mut names = vec!["controls exterior", "controls wipers"];
        names.extend(TILE_NAMES);
        names.extend([
            "controls fog",
            "controls brightness",
            "controls brightness auto",
            "controls lights exterior",
            "controls lights fog",
            "controls auto high beam",
            "controls lock",
            "controls walk-away",
            "controls locks child",
            "controls acceleration",
            "controls steering",
            "controls slip start",
            "controls charge limit",
            "controls scheduled charging",
            "controls autosteer",
            "controls navigate on autopilot",
            "controls follow distance",
            "controls driver heater",
            "controls passenger heater",
            "controls easy entry",
            "controls appearance",
            "controls screen clean",
            "controls 24-hour",
            "controls precondition",
            "controls off-peak",
            "controls safety sentry",
            "controls pin to drive",
            "controls speed limit",
            "controls car wash",
            "controls tow mode",
            "controls check updates",
            "controls advanced updates",
            "controls online routing",
            "controls trip planner",
            "controls voice guidance",
            "controls charge readout",
            "controls schedule time",
            "controls odometer",
            "controls version",
        ]);
        names.extend(HEADER_NAMES);
        names
    }

    /// Presses the Locks door-lock toggle: the one write path the status-bar
    /// lock shares. The frame syncs the toggle's own state back.
    pub fn press_locks_toggle(&self, locked: &Property<bool>) {
        locked.set(!locked.get());
    }

    /// Presses the Safety Sentry toggle: the sentry path's second writer.
    pub fn press_safety_sentry(&self) {
        self.sentry_on.set(!self.sentry_on.get());
    }

    /// Steps the exterior-lights row of the Lights pane forward one choice and
    /// clamps at `Auto`: the `B` key's actuation for Lights.
    pub fn step_lights(&self) {
        let current = self.lights_ext.selected().unwrap_or(0);
        let _ = self.lights_ext.select(current + 1);
    }

    /// Steps the appearance row forward one choice and clamps at `Night`: the
    /// `B` key's actuation for Display.
    pub fn step_appearance(&self) {
        let current = self.appearance.selected().unwrap_or(0);
        let _ = self.appearance.select(current + 1);
    }

    /// Actuates the selected pane's primary control: the `B` key, one write
    /// path per pane through the same setters the widgets themselves use.
    pub fn act(&self, selected: usize) {
        match selected {
            0 => self.press_sentry_tile(),
            1 => self.flip(&self.slip),
            2 => self.flip(&self.sched_charge),
            3 => self.flip(&self.autosteer),
            4 => self.press_child_tile(),
            5 => self.step_lights(),
            6 => self.press_seat_driver(),
            7 => self.step_appearance(),
            8 => self.flip(&self.precondition),
            9 => self.press_safety_sentry(),
            10 => self.press_wash(),
            11 => self.press_check(),
            _ => self.flip(&self.online_routing),
        }
    }

    /// Flips a pane toggle and snaps it, so the state the `B` key wrote is the
    /// state drawn on the same frame. A pane toggle does not animate — see
    /// [`ControlsPanes::sync_demo`] — so a snap is the whole of making a
    /// programmatic flip visible.
    fn flip(&self, toggle: &Toggle) {
        toggle.checked.set(!toggle.checked.get());
        toggle.snap_to_state();
    }

    /// Flips the sentry state: the Controls tile's write path.
    pub fn press_sentry_tile(&self) {
        self.sentry_on.set(!self.sentry_on.get());
    }

    /// Flips the child-lock state: both tiles' write path.
    pub fn press_child_tile(&self) {
        self.child_on.set(!self.child_on.get());
    }

    /// Cycles the driver heater: the Seats `B` path.
    pub fn press_seat_driver(&self) {
        self.seat_driver.set(seat_cycle(self.seat_driver.get()));
    }

    /// Flips Car Wash Mode: the Service `B` path.
    pub fn press_wash(&self) {
        self.car_wash_on.set(!self.car_wash_on.get());
    }

    /// Runs the update check: the Software `B` path.
    pub fn press_check(&self) {
        self.check_done.set(true);
    }

    /// Syncs the bound widgets back from the shared state: the door-lock
    /// toggle follows `Demo::locked`, and every label that names a state is
    /// refreshed from the property it names.
    pub fn sync_demo(&self, locked: bool) {
        self.locks_lock.checked.set(locked);
        self.safety_sentry.checked.set(self.sentry_on.get());
        let child = if self.child_on.get() { "on" } else { "off" };
        self.tiles[1].label.set(format!("Child Lock\n{child}"));
        self.locks_child.label.set(format!("Child Lock\n{child}"));
        let sentry = if self.sentry_on.get() { "on" } else { "off" };
        self.tiles[8].label.set(format!("Sentry\n{sentry}"));
        self.tiles[5].label.set(if self.sentry_on.get() {
            "Recording\n●".to_string()
        } else {
            "Recording\noff".to_string()
        });
        self.seat_driver_btn
            .label
            .set(seat_label("Driver", self.seat_driver.get()));
        self.seat_passenger_btn
            .label
            .set(seat_label("Passenger", self.seat_passenger.get()));
        self.charge_label
            .label
            .text
            .set(format!("{:.0}%", self.charge_limit.value.get()));
        let clean = if self.screen_clean_on.get() {
            "On"
        } else {
            "Off"
        };
        self.screen_clean_btn
            .label
            .set(format!("Screen Clean\n{clean}"));
        let wash = if self.car_wash_on.get() { "On" } else { "Off" };
        self.wash_btn.label.set(format!("Car Wash\n{wash}"));
        let tow = if self.tow_on.get() { "On" } else { "Off" };
        self.tow_btn.label.set(format!("Tow Mode\n{tow}"));
        if self.check_done.get() {
            self.check_btn.label.set("Up to date".to_string());
        }
        for toggle in [
            &self.high_beam,
            &self.locks_lock,
            &self.walk_away,
            &self.slip,
            &self.sched_charge,
            &self.autosteer,
            &self.navigate_ap,
            &self.easy_entry,
            &self.clock_24,
            &self.precondition,
            &self.off_peak,
            &self.safety_sentry,
            &self.pin_drive,
            &self.speed_limit,
            &self.advanced_updates,
            &self.online_routing,
            &self.trip_planner,
        ] {
            toggle.snap_to_state();
        }
    }

    /// Returns the shared state [`ControlsPanes::sync_demo`] writes labels
    /// from: one value per property a label names, so the caller can tell a
    /// frame that changed something from one that did not.
    ///
    /// `Property::set` fires its dependents unconditionally and the demo's
    /// `on_change` callbacks mark nodes dirty, so a per-frame `sync_demo` would
    /// repaint all sixty-eight leaves every frame. This is the record that says
    /// whether anything moved — the same idiom `sync_toggle_state`'s
    /// `toggle_aimed` is.
    #[must_use]
    pub fn state(&self, locked: bool) -> ControlsState {
        ControlsState {
            locked,
            sentry: self.sentry_on.get(),
            child: self.child_on.get(),
            seat_driver: self.seat_driver.get(),
            seat_passenger: self.seat_passenger.get(),
            charge: self.charge_limit.value.get(),
            screen_clean: self.screen_clean_on.get(),
            car_wash: self.car_wash_on.get(),
            tow: self.tow_on.get(),
            check: self.check_done.get(),
        }
    }

    /// Re-themes every pane widget from `theme` and snaps each to it, so a
    /// theme switch re-aims nothing in flight: no pane widget animates.
    pub fn apply_theme(&mut self, theme: &Theme) {
        let seg_palette = segmented_palette(theme);
        for segmented in [
            &mut self.controls_ext,
            &mut self.controls_wipers,
            &mut self.lights_ext,
            &mut self.accel,
            &mut self.steering,
            &mut self.follow,
            &mut self.appearance,
        ] {
            segmented.set_palette(seg_palette);
        }
        for button in self.tiles.iter_mut().chain([
            &mut self.controls_fog,
            &mut self.brightness_auto,
            &mut self.lights_fog,
            &mut self.locks_child,
            &mut self.seat_driver_btn,
            &mut self.seat_passenger_btn,
            &mut self.screen_clean_btn,
            &mut self.wash_btn,
            &mut self.tow_btn,
            &mut self.check_btn,
        ]) {
            if button.label.get() == "Fog"
                || button.label.get() == "Auto"
                || button.label.get().starts_with("Screen Clean")
                || button.label.get().starts_with("Car Wash")
                || button.label.get().starts_with("Tow Mode")
                || button.label.get() == "Check for updates"
                || button.label.get() == "Up to date"
            {
                button.set_palette(ButtonPalette::from_theme(theme));
            } else {
                button.set_palette(ButtonPalette::default());
            }
            button.snap_to_state();
        }
        for toggle in [
            &mut self.high_beam,
            &mut self.locks_lock,
            &mut self.walk_away,
            &mut self.slip,
            &mut self.sched_charge,
            &mut self.autosteer,
            &mut self.navigate_ap,
            &mut self.easy_entry,
            &mut self.clock_24,
            &mut self.precondition,
            &mut self.off_peak,
            &mut self.safety_sentry,
            &mut self.pin_drive,
            &mut self.speed_limit,
            &mut self.advanced_updates,
            &mut self.online_routing,
            &mut self.trip_planner,
        ] {
            toggle.set_palette(ui_core::widgets::toggle::Palette::from_theme(theme));
            toggle.snap_to_state();
        }
        for slider in [
            &mut self.brightness,
            &mut self.charge_limit,
            &mut self.voice,
        ] {
            slider.set_palette(ui_core::widgets::slider::Palette::from_theme(theme));
            slider.snap_to_state();
        }
    }
}

/// Places `handle` at `rect`: the selected pane's boxes, written as layout
/// inputs for the frame's pass to apply.
fn place_leaf(nodes: &mut Arena<WidgetNode>, handle: Handle, rect: Rect) {
    if let Some(node) = nodes.get_mut(handle) {
        node.layout_mut()
            .set_constraints(Constraints::tight(Size::new(rect.width, rect.height)));
        node.layout_mut()
            .set_position(Some(Offset::new(rect.x, rect.y)));
        mark_dirty(nodes, handle);
    }
}

/// Parks `handle` as a zero-size point no other leaf shares: `slot` counts
/// leaves parked so far, and the diagonal stays inside the pane area's
/// bottom-right corner.
fn park_leaf(nodes: &mut Arena<WidgetNode>, pane: Rect, handle: Handle, slot: &mut usize) {
    let count = count_to_f32(*slot);
    *slot += 1;
    let at = Offset::new(
        pane.x + pane.width - 140.0 + (count % 32.0) * 4.0,
        pane.y + pane.height - 30.0 + (count / 32.0).floor() * 4.0,
    );
    if let Some(node) = nodes.get_mut(handle) {
        node.layout_mut()
            .set_constraints(Constraints::tight(Size::new(0.0, 0.0)));
        node.layout_mut().set_position(Some(at));
        mark_dirty(nodes, handle);
    }
}

/// The pane headers' font size, in pixels: a proposal.
const HEADER_FONT_SIZE: f32 = 22.0;

/// The static labels' font size, in pixels: a proposal.
const STATIC_FONT_SIZE: f32 = 16.0;

/// Paints one header or static label's commands: the label's own paint at an
/// explicit rect, the way `record_label` paints the demo's own labels.
fn paint_demo_label(
    demo_label: &DemoLabel,
    metrics: &TextMetrics,
    font_size: f32,
    rect: Rect,
) -> Vec<DrawCommand> {
    let mut options = demo_label.options;
    options.line_height = metrics.line_height(font_size);
    demo_label
        .label
        .paint(rect, &options, &|ch: char| metrics.advance(ch, font_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::input::Key;

    /// A monospace advance of 5 pixels per character.
    fn mono(_: char) -> f32 {
        5.0
    }

    /// A line 20 pixels tall.
    const LINE: f32 = 20.0;

    /// The overlay the geometry tests lay out: the demo window's own right
    /// map area.
    fn overlay() -> Rect {
        Rect::new(512.0, 100.0, 768.0, 856.0)
    }

    /// Thirteen panes' widgets plus the shared state, mocked throughout.
    fn panes() -> (Arena<WidgetNode>, ControlsPanes) {
        let mut nodes = Arena::new();
        let panes = ControlsPanes::new(&mut nodes, &Theme::dark(), Property::new(false));
        (nodes, panes)
    }

    #[test]
    fn the_thirteen_categories_are_the_photo_order_with_new_on_seats() {
        assert_eq!(
            CATEGORIES,
            [
                "Controls",
                "Dynamics",
                "Charging",
                "Autopilot",
                "Locks",
                "Lights",
                "Seats",
                "Display",
                "Schedule",
                "Safety",
                "Service",
                "Software",
                "Navigation",
            ]
        );
        assert_eq!(CATEGORY_COUNT, 13);
        assert_eq!(category_name(0), Some("Controls"));
        assert_eq!(category_name(12), Some("Navigation"));
        assert_eq!(category_name(13), None);
        assert_eq!(CATEGORY_NEW, 6);
        assert_eq!(category_name(CATEGORY_NEW), Some("Seats"));
    }

    #[test]
    fn select_refuses_an_index_past_the_end_and_step_clamps_at_both_ends() {
        let mut nodes = Arena::new();
        let list = CategoryList::new(&mut nodes);
        assert_eq!(list.selected(), 0);
        assert!(!list.select(13), "past the end changes nothing");
        assert_eq!(list.selected(), 0);
        assert!(list.select(12));
        assert!(!list.step(1), "never wraps past Navigation");
        assert_eq!(list.selected(), 12);
        assert!(list.step(-12));
        assert_eq!(list.selected(), 0);
        assert!(!list.step(-1), "never wraps past Controls");
        assert_eq!(list.selected(), 0);
    }

    #[test]
    fn the_pill_reads_text_and_background_tokens_and_no_literal_colour() {
        let mut nodes = Arena::new();
        let list = CategoryList::new(&mut nodes);
        let rect = category_list_rect(overlay());
        for theme in [Theme::dark(), Theme::light()] {
            let commands = list.paint(rect, &mono, LINE, &theme);
            let fills: Vec<Color> = commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::RoundedRect { color, .. } => Some(*color),
                    _ => None,
                })
                .collect();
            assert_eq!(
                fills,
                vec![theme
                    .get(ThemeToken::Text)
                    .as_color()
                    .unwrap_or(Color::new(255, 255, 255, 255))],
                "the one fill is the Text token's value"
            );
            let inks: Vec<Color> = commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { color, .. } => Some(*color),
                    _ => None,
                })
                .collect();
            assert_eq!(inks.len(), CATEGORY_COUNT + 1, "thirteen rows and a badge");
            // The selected row (and its badge, were it the NEW row — it is not)
            // takes `Background`; every other row takes `Text`.
            let text = theme
                .get(ThemeToken::Text)
                .as_color()
                .unwrap_or(Color::new(255, 255, 255, 255));
            let background = theme
                .get(ThemeToken::Background)
                .as_color()
                .unwrap_or(Color::new(0, 0, 0, 255));
            assert_eq!(
                inks[0], background,
                "the selected row's text contrasts the pill"
            );
            for (index, ink) in inks.iter().enumerate().skip(1) {
                let _ = index;
                assert_eq!(*ink, text, "an unselected row is the surface's ink");
            }
        }
        // The pill fill is asserted equal to the `Text` token's value and the
        // row ink to `Background`'s, in both themes, above — which is what
        // "no literal colour" means: a literal would have to equal the token's
        // value in **both** themes to pass, and the dark and light tables do
        // not share either. No source scan is needed, and one would read this
        // test's own text back.
    }

    #[test]
    fn the_two_segmented_rows_have_arities_four_and_six_with_auto_selected() {
        let (_nodes, panes) = panes();
        assert_eq!(panes.controls_ext.choices().len(), 4);
        assert_eq!(panes.controls_wipers.choices().len(), 6);
        assert_eq!(
            panes.controls_ext.value.get(),
            3.0,
            "exterior reverts to Auto"
        );
        assert_eq!(panes.controls_wipers.value.get(), 1.0);
        assert_eq!(panes.lights_ext.choices().len(), 4);
        assert_eq!(panes.lights_ext.value.get(), 3.0);
        // The selected segment reads the pill, not the blue buttons' Primary.
        assert_ne!(
            panes.controls_ext.palette().selected_fill,
            ButtonPalette::from_theme(&Theme::dark()).background
        );
    }

    #[test]
    fn the_tile_block_is_three_flex_rows_and_no_grid() {
        let pane = pane_rect(overlay());
        let rows: Vec<usize> = (0..10)
            .map(|index| tile_row(index).expect("ten tiles"))
            .collect();
        assert_eq!(rows, vec![0, 0, 0, 0, 1, 1, 1, 2, 2, 2]);
        assert_eq!(tile_row(10), None);
        assert_eq!(tile_rows(), [4, 3, 3]);
        // Three bands, top to bottom, each inside the pane.
        let tops: Vec<f32> = (0..10)
            .map(|index| tile_rect(pane, index).expect("ten tiles").y)
            .collect();
        assert!(tops[0] < tops[4] && tops[4] < tops[7]);
        for index in 0..10 {
            let rect = tile_rect(pane, index).expect("ten tiles");
            assert!(
                rect.x >= pane.x
                    && rect.x + rect.width <= pane.x + pane.width
                    && rect.y >= pane.y
                    && rect.y + rect.height <= pane.y + pane.height,
                "tile {index} stays in its pane"
            );
        }
        assert_eq!(tile_rect(pane, 10), None);
        let source = include_str!("controls.rs");
        // The needle is assembled from parts, so this test's own source — which
        // `include_str!` folds in — does not contain the literal it forbids.
        let grid = ["LayoutMode", "::Grid"].concat();
        assert!(
            !source.contains(&grid),
            "the tile block uses no grid layout mode"
        );
        assert!(
            source.contains("Segmented") && source.contains("tile_rect"),
            "the segmented rows are `Segmented`, and the tiles are arithmetic"
        );
    }

    #[test]
    fn fog_shares_one_write_path_between_controls_and_lights() {
        let (_nodes, panes) = panes();
        assert!(!panes.fog_on.get());
        panes.controls_fog.on_click.call(());
        assert!(panes.fog_on.get());
        panes.lights_fog.on_click.call(());
        assert!(
            !panes.fog_on.get(),
            "the second writer flips the same state"
        );
    }

    #[test]
    fn the_door_lock_toggle_binds_the_status_bar_lock_property() {
        let mut nodes = Arena::new();
        let locked = Property::new(false);
        let panes = ControlsPanes::new(&mut nodes, &Theme::dark(), locked.clone());
        panes.press_locks_toggle(&locked);
        assert!(locked.get(), "the toggle writes Demo::locked");
        panes.sync_demo(locked.get());
        assert!(
            panes.locks_lock.checked.get(),
            "and the frame syncs the toggle back"
        );
        panes.press_locks_toggle(&locked);
        assert!(!locked.get());
    }

    #[test]
    fn dynamics_offers_two_segmented_rows_and_a_slip_start_toggle() {
        let (_nodes, panes) = panes();
        assert_eq!(panes.accel.choices().len(), 2);
        assert_eq!(panes.accel.value.get(), 1.0);
        assert_eq!(panes.steering.choices().len(), 3);
        assert_eq!(panes.steering.value.get(), 1.0);
        assert!(!panes.slip.checked.get());
    }

    #[test]
    fn charging_pairs_an_80_percent_slider_with_a_schedule_toggle() {
        let (_nodes, panes) = panes();
        assert_eq!(panes.charge_limit.value.get(), 80.0);
        assert_eq!(panes.charge_limit.step(), Some(1.0));
        assert!(!panes.sched_charge.checked.get());
        panes.sync_demo(false);
        assert_eq!(panes.charge_label.label.text.get(), "80%");
    }

    #[test]
    fn autopilot_pairs_two_toggles_with_a_seven_point_follow_distance() {
        let (_nodes, panes) = panes();
        assert!(panes.autosteer.checked.get());
        assert!(!panes.navigate_ap.checked.get());
        assert_eq!(panes.follow.choices().len(), 7);
        assert_eq!(panes.follow.value.get(), 2.0, "distance 3");
    }

    #[test]
    fn seats_cycle_two_heaters_through_three_levels_plus_easy_entry() {
        let (_nodes, panes) = panes();
        assert_eq!(seat_cycle(0), 1);
        assert_eq!(seat_cycle(1), 2);
        assert_eq!(seat_cycle(2), 3);
        assert_eq!(seat_cycle(3), 0);
        assert_eq!(seat_cycle(9), 0, "garbage in, off out");
        panes.press_seat_driver();
        panes.press_seat_driver();
        assert_eq!(panes.seat_driver.get(), 2);
        panes.sync_demo(false);
        assert_eq!(
            panes.seat_driver_btn.label.get(),
            "Driver / Level 2",
            "the label names the level"
        );
        assert!(panes.easy_entry.checked.get());
    }

    #[test]
    fn display_pairs_appearance_with_screen_clean_and_24_hour() {
        let (_nodes, panes) = panes();
        assert_eq!(panes.appearance.value.get(), 0.0);
        assert!(!panes.clock_24.checked.get());
        panes.screen_clean_btn.on_click.call(());
        assert!(panes.screen_clean_on.get());
        panes.sync_demo(false);
        assert_eq!(
            panes.screen_clean_btn.label.get(),
            "Screen Clean\nOn",
            "the label shows the state"
        );
    }

    #[test]
    fn schedule_pairs_two_toggles_with_a_static_mocked_time() {
        let (_nodes, panes) = panes();
        assert!(!panes.precondition.checked.get());
        assert!(!panes.off_peak.checked.get());
        assert_eq!(
            panes.schedule_time.label.text.get(),
            "Weekdays 07:30",
            "mocked: editing a time needs text entry"
        );
    }

    #[test]
    fn sentry_is_one_property_in_two_panes() {
        let (_nodes, panes) = panes();
        assert!(!panes.sentry_on.get());
        panes.tiles[8].on_click.call(());
        assert!(panes.sentry_on.get(), "the tile flips it");
        panes.press_safety_sentry();
        assert!(!panes.sentry_on.get(), "the toggle flips it back");
        panes.sync_demo(false);
        assert_eq!(panes.safety_sentry.checked.get(), panes.sentry_on.get());
    }

    #[test]
    fn service_pairs_two_blue_buttons_with_a_mocked_odometer() {
        let (_nodes, panes) = panes();
        panes.wash_btn.on_click.call(());
        panes.tow_btn.on_click.call(());
        assert!(panes.car_wash_on.get());
        assert!(panes.tow_on.get());
        panes.sync_demo(false);
        assert_eq!(panes.wash_btn.label.get(), "Car Wash\nOn");
        assert_eq!(panes.tow_btn.label.get(), "Tow Mode\nOn");
        assert_eq!(panes.odometer.label.text.get(), "12,480 mi");
    }

    #[test]
    fn software_pairs_a_mocked_version_with_check_and_advanced() {
        let (_nodes, panes) = panes();
        assert_eq!(panes.version.label.text.get(), "2026.20.1");
        assert!(!panes.advanced_updates.checked.get());
        panes.check_btn.on_click.call(());
        assert!(panes.check_done.get());
        panes.sync_demo(false);
        assert_eq!(panes.check_btn.label.get(), "Up to date");
    }

    #[test]
    fn navigation_pairs_two_toggles_with_a_voice_slider() {
        let (_nodes, panes) = panes();
        assert!(panes.online_routing.checked.get());
        assert!(panes.trip_planner.checked.get());
        assert_eq!(panes.voice.value.get(), 5.0);
    }

    #[test]
    fn tapping_a_row_selects_it_and_nothing_outside_does() {
        use ui_core::input::InputEventKind;
        use ui_core::layout::Offset;
        let mut nodes = Arena::new();
        let list = CategoryList::new(&mut nodes);
        let rect = category_list_rect(overlay());
        let row = |index: usize| {
            Offset::new(
                rect.x + 10.0,
                rect.y + count_to_f32(index) * CATEGORY_ROW_HEIGHT + 10.0,
            )
        };
        let mut tap = ui_core::input::InputEvent::new(InputEventKind::Tap, Some(row(4)));
        assert!(list.on_event(&mut tap, rect));
        assert_eq!(list.selected(), 4);
        let mut away =
            ui_core::input::InputEvent::new(InputEventKind::Tap, Some(Offset::new(0.0, 0.0)));
        assert!(!list.on_event(&mut away, rect));
        assert_eq!(list.selected(), 4);
        // A key is not a tap and travels on.
        let mut key = ui_core::input::InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(sdl3::keyboard::Keycode::W),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        );
        assert!(!list.on_event(&mut key, rect));
    }
}
