# TASK_UI_DEMO_06: The Controls Panel — a Category List and Three Built Panes in the Car Overlay

> **2026-10-15 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE_DEMO.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Fill the car overlay panel (`DockPanel::Car`, an empty surface since
`TASK_UI_DEMO_02`) with the Controls screen `doc/ui/DEMO_APPLICATION.md`
§ *The Controls panel* specifies: a scrollable category list on the left with
a white-pill selection, and a settings pane on the right. All thirteen
categories get a pane — **Controls**, **Lights**, **Locks** exactly as
specified below, the other ten as stated proposals from the section's four
cell archetypes (segmented, tile, blue button, slider) plus `Toggle`, every
value mocked. No pointer event reaches this window on this host, so `Q`/`W`
move the selection, `B` actuates the selected pane's primary control, and
every rule is a unit test.

## Context

- **Hard prerequisite `TASK_UI_PRIM_47`, not built.** The two segmented rows
  (exterior lights `Off │ Parking │ On │ Auto`, wipers
  `Off │ Auto │ I │ II │ III │ IIII`) are `Segmented` with arities 4 and 6 —
  the case § *The Controls panel* calls out as needing one widget at two
  arities. **If 47 has not landed this task cannot be started.** No other
  library task is needed: `List` scrolls vertically already (task 46 is the
  horizontal half), `List::clip_rect` bounds the rows, `Button`/`Toggle`/`Slider`
  exist, and the overlay gate (`open_panel == Some(Car)`, tap-outside closes)
  is built. **This task is ordered behind `TASK_UI_DEMO_05`**, which it does
  not depend on.
- **First demo use of `List`.** `List::new(nodes, item_count, item_height)` plus
  `ItemFactory` plus `sync`/`paint`/`on_event` are the whole of it; the file
  defers to `list.rs`'s docs for the mechanics and specifies only the demo's
  rows.
- **Thirteen categories** (§ *Corrections to the first sketch*, car-screen tabs
  row, `[A]` photo `02` + manual cross-references): Controls, Dynamics,
  Charging, Autopilot, Locks, Lights, Seats (`NEW` badge), Display, Schedule,
  Safety, Service, Software, Navigation (cut off — the list continues, so
  thirteen is the count and the order is the photo's). Renames worth knowing
  (*Pedals & Steering → Dynamics*, *Autopilot → Self-Driving*) stay out: the
  rows carry the photo's names.
- **No baked masters for these glyphs.** The 33 Lucide PNGs cover dock and
  chrome needs, not thirteen settings categories — so rows are text-only with
  the pill, and category glyphs are an asset task. The pill adapts to both
  themes: fill `ThemeToken::Text`'s value, row text `ThemeToken::Background`'s.
- **No `LayoutMode::Grid`.** Task 52 is not built, so the tile block is three
  `Flex` rows (4 + 3 + 3), never a grid.
- **Shared state, not duplicated state.** The fog button writes the same
  one-shot path in Controls and Lights; the door-lock toggle binds
  `Demo::locked`, the property the status-bar lock button already flips; the
  Sentry tile (Controls) and the Sentry toggle (Safety) flip one
  `sentry_on: Property<bool>`.

## Requirements

### 06-1 — the category list

1. New module `ui/src/ui_demo/src/controls.rs`, declared `mod controls;` in
   `ui/src/ui_demo/src/main.rs` beside `mod dock;`. Every public item
   doc-commented; every value-returning function `#[must_use]`.
2. `pub const CATEGORY_COUNT: usize = 13`, `CATEGORY_ROW_HEIGHT: f32 = 56.0`,
   `CONTROLS_LIST_FRACTION: f32 = 0.30` (list width share of the overlay rect,
   a proposal), `PILL_RADIUS: f32 = 12.0`. `pub const CATEGORIES:
   [&str; 13]` in the photo's order — Controls, Dynamics, Charging, Autopilot,
   Locks, Lights, Seats, Display, Schedule, Safety, Service, Software,
   Navigation; `pub const CATEGORY_NEW: usize = 6`
   (Seats). `#[must_use] pub fn category_name(index: usize) -> Option<&'static
   str>` (`None` past the end).
3. `pub struct CategoryList`: one `List` (`CATEGORY_COUNT` rows,
   `CATEGORY_ROW_HEIGHT`), `selected: Property<usize>`, pill fill/text colours
   read from the theme at paint time. `select(&self, index: usize) -> bool`
   (false past the end, clamps nothing); `step(&self, delta: isize) -> bool`
   (clamps at both ends, never wraps). Selection paint is one
   `DrawCommand::RoundedRect` behind the selected row's text — the white pill —
   plus one `DrawCommand::Text` per row and a `NEW` badge label on row 6.
4. `Demo` gains `controls_list: CategoryList` and `controls_panes` (06-2/06-3
   below), all nodes children of the Car overlay panel's node, so the
   `chrome_all` walk gives every leaf its `Page::Demo, focusable: false` row
   and `placed_handles`/`expected_placed_rect_names` grow by the same count,
   none loosened. One paint arm per new widget type (`List`, `Segmented`);
   `Button`/`Toggle`/`Slider` reuse their arm shapes. Nothing paints unless
   `open_panel == Some(DockPanel::Car)` — the existing gate, unchanged.

### 06-2 — the Controls pane

5. Two `Segmented`s: exterior (`["Off", "Parking", "On", "Auto"]` @
   `0.0..=3.0`) and wipers (`["Off", "Auto", "I", "II", "III", "IIII"]` @
   `0.0..=5.0`), task 47's `Choice`/`set_choices` API, values initialised
   `3.0` (Auto — exterior lights revert to Auto every drive, `[A]` §Lights)
   and `1.0`.
6. Ten action tiles in three `Flex` rows (4 + 3 + 3): `Fold Mirrors`,
   `Child Lock` (secondary `off`), `Window Lock`, `Glovebox` /
   `Mirrors`, `Recording` (dot while on), `Car Wash` /
   `Steering`, `Sentry` (dot while on), `Neutral` (secondary `Hold`). Tiles
   are `Button`s with two-line labels; stateful tiles append the secondary
   line, never a third widget.
7. Two solid-blue action buttons (`Fog`, `Auto`): fill the theme's `Primary`,
   text `OnPrimary` — visually distinct from a selected segment on purpose.
   `Fog` shares its one-shot write path with the Lights pane's fog button.
8. Brightness row: `DemoLabel` + `Slider` (`0.0..=100.0`, initialised `80.0`)
   + blue `Auto` button. No sun glyph — deferred with the category glyphs.

### 06-3 — the Lights and Locks panes

9. Lights: one exterior `Segmented` (same four choices, its own value,
   initialised `3.0`), the shared `Fog` blue button, one `Toggle`
   (`Auto high beam`, off).
10. Locks: one `Toggle` bound to `Demo::locked` (the status-bar lock's
    property — one state, two writers), one `Toggle` (`Walk-away lock`,
    mocked on), one `Child Lock` tile with secondary `off` sharing the
    Controls tile's write path.

### 06-4 — the other ten panes, each a stated proposal

Every value below is mocked (the `TEMP_INIT` precedent); each pane is a header
label with the category name plus the controls named, built from the four
cell archetypes and `Toggle`, nothing else.

11. Dynamics (index 1): `Segmented` Acceleration `["Chill", "Standard"]`,
    initialised `1.0`; `Segmented` Steering `["Comfort", "Standard",
    "Sport"]`, initialised `1.0`; `Toggle` Slip Start, off.
12. Charging (2): `Slider` charge limit `50.0..=100.0`,
    `set_step(Some(1.0))`, initialised `80.0`, with a `DemoLabel` reading
    `"80%"` beside it; `Toggle` Scheduled charging, off.
13. Autopilot (3): `Toggle` Autosteer, mocked on; `Toggle` Navigate on
    Autopilot, off; `Segmented` Follow Distance `["1" … "7"]`, initialised
    `2.0` (distance 3).
14. Seats (6): two heater `Button`s, driver and passenger, each cycling
    `0 → 1 → 2 → 3 → 0` (three levels per `[A]` § seat widget — a demo-local
    `seat_cycle`, since `dock::cycle_heater` tops out at 2), labels
    `"Driver / Off"` through `"Driver / Level 3"`; `Toggle` Easy Entry,
    mocked on.
15. Display (7): `Segmented` Appearance `["Auto", "Day", "Night"]`,
    initialised `0.0`; blue `Button` Screen Clean Mode (press toggles its own
    `Property<bool>`, label shows the state); `Toggle` 24-hour time, off.
16. Schedule (8): `Toggle` Precondition for departure, off; `Toggle`
    Off-peak charging, off; static `DemoLabel` `"Weekdays 07:30"` (mocked —
    editing a time needs text entry, which is a `TASK_UI_DEMO_n` item).
17. Safety (9): `Toggle` Sentry bound to `sentry_on` — the Controls tile's
    property, one state in two panes; `Toggle` PIN to Drive, off; `Toggle`
    Speed Limit Mode, off.
18. Service (10): blue `Button` Car Wash Mode and blue `Button` Tow Mode, each
    toggling its own `Property<bool>` with the label showing the state;
    static `DemoLabel` `"12,480 mi"` (mocked odometer).
19. Software (11): static `DemoLabel` `"2026.20.1"` (mocked version); blue
    `Button` Check for updates (one-shot → label `"Up to date"`); `Toggle`
    Advanced update preference, off.
20. Navigation (12): `Toggle` Online Routing, mocked on; `Toggle` Trip
    Planner, mocked on; `Slider` voice guidance `0.0..=10.0`, initialised
    `5.0`.
21. `GALLERY_SHORTCUTS` gains three rows, array `24 + 3`: **`Q`** previous
    category, **`W`** next category (both `Some(Page::Demo)`, through
    `CategoryList::step`), **`B`** actuates the selected pane's primary
    control — Sentry tile (Controls), Slip Start (Dynamics), Scheduled
    charging (Charging), Autosteer (Autopilot), child-lock tile (Locks),
    exterior-lights step (Lights), driver heater cycle (Seats), appearance
    step (Display), precondition (Schedule), Sentry toggle (Safety), Car
    Wash (Service), check updates (Software), online routing (Navigation) —
    one write path per pane through the existing setters. The
    `no_printable_key_acts_without_a_row_in_the_shortcut_table` gate covers
    all three.
22. Tests, beside the code, no display/network/filesystem/wall clock:
    `the_thirteen_categories_are_the_photo_order_with_new_on_seats`,
    `select_refuses_an_index_past_the_end_and_step_clamps_at_both_ends`,
    `the_pill_reads_text_and_background_tokens_and_no_literal_colour`,
    `the_two_segmented_rows_have_arities_four_and_six_with_auto_selected`,
    `the_tile_block_is_three_flex_rows_and_no_grid`,
    `fog_shares_one_write_path_between_controls_and_lights`,
    `the_door_lock_toggle_binds_the_status_bar_lock_property`,
    `dynamics_offers_two_segmented_rows_and_a_slip_start_toggle`,
    `charging_pairs_an_80_percent_slider_with_a_schedule_toggle`,
    `autopilot_pairs_two_toggles_with_a_seven_point_follow_distance`,
    `seats_cycle_two_heaters_through_three_levels_plus_easy_entry`,
    `display_pairs_appearance_with_screen_clean_and_24_hour`,
    `schedule_pairs_two_toggles_with_a_static_mocked_time`,
    `sentry_is_one_property_in_two_panes`,
    `service_pairs_two_blue_buttons_with_a_mocked_odometer`,
    `software_pairs_a_mocked_version_with_check_and_advanced`,
    `navigation_pairs_two_toggles_with_a_voice_slider`,
    `the_b_key_map_covers_all_thirteen_panes`,
    `the_q_w_and_b_keys_reach_the_panel_through_handle_event`,
    `every_controls_leaf_has_a_page_row_and_a_placed_handles_row`,
    `the_panel_paints_nothing_while_closed`.
23. `doc/ui/DEMO_APPLICATION.md` § *The Controls panel* gains one dated,
    attributed note naming what 06 built (list + thirteen panes, the ten
    proposals of 06-4, shared-state list, glyph deferral) and what it did not
    (departure-time editing, category glyphs, renames). No row closed or
    renumbered.

## Acceptance Criteria

- [ ] **Thirteen rows in the photo's order, `NEW` on Seats, pill on the
      selection in both themes** (theme switch flips pill and text; no
      literal colour in `controls.rs`).
- [ ] **Thirteen panes built: three specified, ten stated proposals**
      (06-4; every value mocked, each pane header-labelled; no pane claims
      manual backing beyond Controls/Lights/Locks).
- [ ] **47's API is the only new-widget route** (`rg -n 'Segmented'
      ui/src/ui_demo/src/controls.rs` non-empty; no demo-local segmented
      reimplementation; no `LayoutMode::Grid`).
- [ ] **Shared state is single** (fog one path, `Demo::locked` one property,
      `sentry_on` one property in two panes; toggling the status-bar lock
      flips the Locks toggle and back).
- [ ] **`Q`/`W`/`B` reach only their rows** (`Some(Page::Demo)`; the
      shortcut-table gates hold; **no criterion requires a pointer event**).
- [ ] **No leakage:** `git diff --stat` shows one new file
      (`ui/src/ui_demo/src/controls.rs`) plus `main.rs` and
      `DEMO_APPLICATION.md`, nothing under `ui/src/ui_core/`,
      `ui/Cargo.toml`/`ui/Cargo.lock` unchanged; no
      `unsafe`/`unwrap`/`expect`/`panic!`/`unimplemented!`/`todo!`;
      `Page::ALL` seven, `Page::DEFAULT` `Page::Pads`, no `--tab=`.
- [ ] **Suite green, all twenty-one tests named in the handoff**, counts pasted
      against the figure in `doc/ui/IMPLEMENTATION_STATE.md` re-measured at
      build time; verification is `.ai/agents/developer.md` § *Phase 3*.
- [ ] **Captures:** all pages `AE = 0` with the panel closed; one
      open-panel capture on `--tab=demo` (list + Controls pane) and one with
      the Lights pane; fps on all seven pages above 55
      (`.ai/tools/fps-check.sh 10 55`).
- [ ] **The handoff states, in those words:** ordered behind 47 and 05;
      first demo use of `List`; category glyphs deferred for lack of masters;
      ten panes are stated proposals with mocked values, not manual-backed;
      `Demo::locked` and `sentry_on` shared, not duplicated;
      nothing verified by a tap.

## Out of Scope

- **No departure-time editing in Schedule** — the `"Weekdays 07:30"` label is
  static and mocked; editing needs text entry, a `TASK_UI_DEMO_n` item.
- **No category glyphs and no sun glyph** — text rows; masters are an asset
  task.
- **No `Autopilot → Self-Driving` / `Pedals & Steering → Dynamics` renames**
  — photo's names stand.
- **No edge-swipe summoning of Controls** (§ *Corrections* names it; this
  host has no edge gesture) — car icon only.
- **No seatbelt popup, no per-seat mute, no charge-port lamp protocol.**
- **No new theme token, no `ThemeScope`, no `ModeScope`, no `Margin`, no
  `LayoutMode::Grid`, no `polygon_is_convex`, no measured text runs.**
- **No new dependency and no `unsafe`.**
- **Found in the tree and deliberately not fixed:** `GALLERY_SHORTCUTS`'s doc
  still says "eighteen" over a nineteen-row table; the count beside the array
  length is 06's to correct.
