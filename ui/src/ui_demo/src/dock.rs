//! The bottom dock's state: climatronic setpoints, heater levels, volume and
//! the overlay panel one opener at a time shows.
//!
//! **Pure functions of values, no arena.** Like [`crate::chrome`], everything
//! here is arithmetic over numbers and rects, which is what lets the suite
//! assert the step, the clamp and the geometry without a window. The widgets
//! themselves — the dock's buttons, the nested icon images, the five overlay
//! panels — live in `main.rs` and read this module; nothing here names a node.
//!
//! The dock is nine slots in two groups: left `[car][debug][< temp >][seat
//! heat][defrost][< sound n >]` and right `[seat heat][< temp >]`, with a
//! flexible spacer between them. Three behaviours make it more than a row of
//! buttons, and all three are pure:
//!
//! 1. **Steppers clamp.** [`step_temp_up`]/[`step_temp_down`] move in
//!    [`TEMP_STEP`] within [`TEMP_MIN`]..=[`TEMP_MAX`];
//!    [`step_volume_up`]/[`step_volume_down`] in [`VOLUME_STEP`] within
//!    `0..=100`. A step past the end stays at the end.
//! 2. **Heaters cycle.** [`cycle_heater`] walks `0 → 1 → 2 → 0`; anything
//!    outside `0..=2` resets to `0` rather than preserving a state no tint
//!    exists for.
//! 3. **One panel at a time.** [`toggle_open`] opens the clicked opener and
//!    closes whatever was open; clicking the open one closes it, and a tap
//!    outside closes all ([`close_all`]).

use ui_core::layout::Size;
use ui_core::paint::Rect;

use crate::chrome::{self, CAR_STATUS_PANE_FRACTION, DOCK_HEIGHT};

/// The driver and passenger setpoints at launch, in °C. Mocked vehicle data —
/// no bus drives them.
pub const TEMP_INIT: f32 = 22.0;

/// The climatronic range, in °C: a Passat unit's own end stops.
pub const TEMP_MIN: f32 = 16.0;
/// The climatronic range, in °C: a Passat unit's own end stops.
pub const TEMP_MAX: f32 = 28.0;

/// One arrow press, in °C.
pub const TEMP_STEP: f32 = 0.5;

/// The volume at launch, in percent. Mocked like the setpoints.
pub const VOLUME_INIT: u8 = 50;

/// One volume-arrow press, in percent.
pub const VOLUME_STEP: u8 = 5;

/// The highest seat-heater level. `0` is off.
pub const HEATER_MAX: u8 = 2;

/// The dock's button counts per group, left then right: the six left control
/// groups (car, debug, driver climatronic, driver seat heat, defrost, sound)
/// span `1 + 1 + 3 + 1 + 1 + 3` buttons, the two right ones (passenger seat
/// heat, passenger climatronic) `1 + 3`. Read at construction for the button
/// table's capacity and by the suite for the groups' child counts.
pub const DOCK_BUTTONS: [usize; 2] = [10, 4];

/// Returns `current` one [`TEMP_STEP`] up, clamped to [`TEMP_MAX`].
#[must_use]
pub fn step_temp_up(current: f32) -> f32 {
    (current + TEMP_STEP).min(TEMP_MAX)
}

/// Returns `current` one [`TEMP_STEP`] down, clamped to [`TEMP_MIN`].
#[must_use]
pub fn step_temp_down(current: f32) -> f32 {
    (current - TEMP_STEP).max(TEMP_MIN)
}

/// Returns the setpoint text: one decimal with the degree sign, no unit
/// letter — `"22.0°"`, the Tesla dock's unitless style with the mark that
/// says it is a temperature.
///
/// **A proposal.** The `°` is U+00B0, which Lato covers (see `main.rs`'s
/// ambient-temperature comment); `"22.0°C"` is the alternative the operator
/// can pick without touching the step logic.
#[must_use]
pub fn format_temp(value: f32) -> String {
    format!("{value:.1}\u{b0}")
}

/// Returns `current` one [`VOLUME_STEP`] up, clamped to `100`.
#[must_use]
pub fn step_volume_up(current: u8) -> u8 {
    current.saturating_add(VOLUME_STEP).min(100)
}

/// Returns `current` one [`VOLUME_STEP`] down, clamped to `0`.
#[must_use]
pub fn step_volume_down(current: u8) -> u8 {
    current.saturating_sub(VOLUME_STEP)
}

/// Returns the volume text: the number with a percent sign — `"50%"`.
///
/// **A proposal**, like [`format_temp`]: explicit where the temperature is
/// bare, because a bare number beside a speaker reads as a track number.
#[must_use]
pub fn format_volume(value: u8) -> String {
    format!("{value}%")
}

/// Returns the heater level after one tap: `0 → 1 → 2 → 0`. A value outside
/// `0..=HEATER_MAX` resets to `0`: no tint exists for it, so keeping it would
/// draw a level the legend cannot name.
#[must_use]
pub fn cycle_heater(current: u8) -> u8 {
    match current {
        0 => 1,
        1 => HEATER_MAX,
        _ => 0,
    }
}

/// The five dock controls that open an overlay panel: the car and debug icon
/// buttons, the two setpoint buttons (the arrows beside them step, the number
/// itself opens) and the sound button (the arrows step the volume, the icon
/// with its number opens).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPanel {
    /// The car icon's panel.
    Car,
    /// The debug icon's panel.
    Debug,
    /// The driver setpoint's panel.
    DriverTemp,
    /// The sound button's panel.
    Sound,
    /// The passenger setpoint's panel.
    PassengerTemp,
}

/// Returns the open panel after `clicked` is tapped while `current` is open:
/// the clicked one, unless it is already open — then nothing. That is the
/// whole of the toggle: one panel at a time, and a second tap closes it.
#[must_use]
pub fn toggle_open(current: Option<DockPanel>, clicked: DockPanel) -> Option<DockPanel> {
    if current == Some(clicked) {
        None
    } else {
        Some(clicked)
    }
}

/// Returns no open panel for `open`: the answer to a tap outside the open
/// panel. Takes the open state so the call site assigns the result over it —
/// and so the suite can assert every opener closes, not just none.
#[must_use]
pub fn close_all(open: Option<DockPanel>) -> Option<DockPanel> {
    let _ = open;
    None
}

/// A dock icon: the five white masters beside the airbag and lock ones, one
/// 48-pixel file each, tinted at draw time through `icons::paint`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockIcon {
    /// The car button's glyph.
    Car,
    /// The debug button's glyph.
    Debug,
    /// The sound button's glyph.
    Sound,
    /// Both seat-heater buttons' glyph.
    SeatHeater,
    /// The defrost button's glyph.
    Defrost,
}

/// Returns the baked white master for `icon`.
///
/// Keyed by glyph only — never by heater level, defrost state or volume,
/// which become the shader tint at paint time like every other icon in the
/// demo.
#[must_use]
pub fn dock_baked_file(icon: DockIcon) -> &'static str {
    match icon {
        DockIcon::Car => "car_48.png",
        DockIcon::Debug => "debug_48.png",
        DockIcon::Sound => "sound_48.png",
        DockIcon::SeatHeater => "seat_heater_48.png",
        DockIcon::Defrost => "defrost_48.png",
    }
}

/// Returns the overlay panel's rectangle for a window of `window` whose
/// content starts at `content_top`.
///
/// The right map area: right of the car-status pane, below the status bar,
/// above the dock. Derived from [`crate::chrome`]'s own numbers — the pane
/// fraction, the bar height, the dock height — so the panel and the chrome
/// cannot disagree about where the map area is.
#[must_use]
pub fn overlay_rect(window: Size, content_top: f32) -> Rect {
    let bar_bottom = content_top + chrome::status_bar_height(window);
    let pane_right = window.width * CAR_STATUS_PANE_FRACTION;
    Rect::new(
        pane_right,
        bar_bottom,
        window.width - pane_right,
        window.height - DOCK_HEIGHT - bar_bottom,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_steps_move_half_a_degree_and_clamp_at_both_ends() {
        assert_eq!(step_temp_up(22.0), 22.5, "one press up");
        assert_eq!(step_temp_down(22.0), 21.5, "one press down");
        assert_eq!(step_temp_up(TEMP_MAX), TEMP_MAX, "past the top stays");
        assert_eq!(step_temp_down(TEMP_MIN), TEMP_MIN, "past the bottom stays");
        assert_eq!(step_temp_up(27.8), TEMP_MAX, "27.8 + 0.5 clamps to 28");
        assert_eq!(step_temp_down(16.2), TEMP_MIN, "16.2 - 0.5 clamps to 16");
    }

    #[test]
    fn temp_text_is_one_decimal_with_a_degree_sign() {
        assert_eq!(format_temp(22.0), "22.0\u{b0}");
        assert_eq!(format_temp(TEMP_INIT), "22.0\u{b0}");
        assert_eq!(format_temp(TEMP_MIN), "16.0\u{b0}");
    }

    #[test]
    fn volume_steps_move_five_percent_and_clamp_at_both_ends() {
        assert_eq!(step_volume_up(50), 55);
        assert_eq!(step_volume_down(50), 45);
        assert_eq!(step_volume_up(100), 100, "past full stays full");
        assert_eq!(step_volume_down(0), 0, "past mute stays mute");
        assert_eq!(step_volume_up(98), 100, "98 + 5 clamps to 100");
        assert_eq!(step_volume_down(3), 0, "3 - 5 saturates to 0");
    }

    #[test]
    fn volume_text_carries_a_percent_sign() {
        assert_eq!(format_volume(50), "50%");
        assert_eq!(format_volume(VOLUME_INIT), "50%");
    }

    #[test]
    fn the_heater_cycles_off_one_two_and_back_to_off() {
        assert_eq!(cycle_heater(0), 1, "off warms to level 1");
        assert_eq!(cycle_heater(1), 2, "level 1 warms to level 2");
        assert_eq!(cycle_heater(2), 0, "level 2 switches off");
        assert_eq!(cycle_heater(HEATER_MAX), 0, "the top level switches off");
    }

    #[test]
    fn a_heater_level_with_no_tint_resets_to_off() {
        assert_eq!(cycle_heater(7), 0, "no fourth level exists");
        assert_eq!(cycle_heater(u8::MAX), 0, "garbage in, off out");
    }

    #[test]
    fn tapping_an_opener_opens_it_and_tapping_it_again_closes_it() {
        assert_eq!(toggle_open(None, DockPanel::Car), Some(DockPanel::Car));
        assert_eq!(toggle_open(Some(DockPanel::Car), DockPanel::Car), None);
    }

    #[test]
    fn tapping_another_opener_switches_panels_never_stacks_them() {
        assert_eq!(
            toggle_open(Some(DockPanel::Car), DockPanel::Sound),
            Some(DockPanel::Sound),
            "the car panel gives way to the sound panel"
        );
        assert_eq!(
            toggle_open(Some(DockPanel::DriverTemp), DockPanel::PassengerTemp),
            Some(DockPanel::PassengerTemp)
        );
    }

    #[test]
    fn tapping_outside_closes_whatever_is_open() {
        for panel in [
            DockPanel::Car,
            DockPanel::Debug,
            DockPanel::DriverTemp,
            DockPanel::Sound,
            DockPanel::PassengerTemp,
        ] {
            assert_eq!(
                close_all(Some(panel)),
                None,
                "{panel:?} closes on a tap outside it"
            );
        }
        assert_eq!(close_all(None), None, "nothing open stays nothing open");
    }

    #[test]
    fn every_dock_icon_has_its_own_baked_file() {
        let files = [
            dock_baked_file(DockIcon::Car),
            dock_baked_file(DockIcon::Debug),
            dock_baked_file(DockIcon::Sound),
            dock_baked_file(DockIcon::SeatHeater),
            dock_baked_file(DockIcon::Defrost),
        ];
        assert_eq!(files[0], "car_48.png");
        assert_eq!(files[3], "seat_heater_48.png");
        let mut sorted = files.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), files.len(), "no two glyphs share a file");
    }

    #[test]
    fn the_overlay_is_the_right_map_area_and_nothing_else() {
        let window = Size::new(1280.0, 1020.0);
        let content_top = 64.0;
        let overlay = overlay_rect(window, content_top);
        let regions = chrome::chrome_regions(window, content_top);
        let bar_bottom = content_top + chrome::status_bar_height(window);
        assert_eq!(overlay.x, window.width * CAR_STATUS_PANE_FRACTION);
        assert_eq!(overlay.y, bar_bottom, "below the status bar");
        assert_eq!(
            overlay.y + overlay.height,
            window.height - DOCK_HEIGHT,
            "above the dock"
        );
        assert_eq!(
            overlay.x + overlay.width,
            window.width,
            "to the window edge"
        );
        let _ = regions;
    }

    #[test]
    fn the_overlay_never_covers_the_chrome_it_floats_between() {
        let window = Size::new(1280.0, 1020.0);
        let overlay = overlay_rect(window, 64.0);
        let regions = chrome::chrome_regions(window, 64.0);
        let overlaps = |a: Rect, b: Rect| {
            a.x < b.x + b.width
                && b.x < a.x + a.width
                && a.y < b.y + b.height
                && b.y < a.y + a.height
        };
        for region in regions {
            assert!(!overlaps(overlay, region), "{overlay:?} covers {region:?}");
        }
    }
}
