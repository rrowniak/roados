# TASK_UI_DEMO_04: The Indicator-Light Column — ~20 Conditions, Five Colours, Three Timing Rules, and One Latch

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Build the **indicator-light column** `TASK_UI_DEMO_03` left as an empty box: a **severity-ranked list of ~20 conditions** painted as coloured discs down the pane's left column.
Five colours are slots, three conditions are distinguished by timing rather than colour, one light latches and clears only on an external event, and every light flashes at power-up as a self-test then goes out.
The blink is a pure function of an elapsed `Duration` and the latch a one-writer `bool`, so every rule is unit-testable with literal durations.

## Context

- **Hard prerequisite `TASK_UI_DEMO_03`, absent from the tree:** it defines `PaneRects::indicators` via `pane_rects(width) -> PaneRects` (the box this column paints into) and makes `Demo::set_car_state` the one write path with room for one added line. **If it has not landed this task cannot be started.** No other task is needed.
- **23 conditions across five slots** (the document's "~20"): **Red (6)** brake fault, parking brake applied, seat belt unfastened in an occupied seat, airbag fault, door or trunk open, system failure; **Amber (7)** brake booster, ABS (brief flash at startup), parking-brake electrical, tire pressure out of range, ESC active (flashing), ESC off, power limited; **Green (4)** parking lights, low beam, ready to drive, battery low; **Blue (3)** high beam, high beam with Adaptive Headlights armed, snowflake (battery too cold); **Grey (3)** Adaptive Headlights armed but dimmed, Vehicle Hold, pedestrian warning paused.
- **Timing, then a latch.** ABS flashes once at startup then faults if it stays; ESC flashes while actively correcting and goes solid if it is a fault; the tyre tell-tale is steady for low pressure and flashing for a sensor fault; the other 20 are steady. The tyre light **latches** — it does not clear when you inflate, only once you drive over **15 mph (25 km/h) for a short time**.
- **Self-test:** every light flashes briefly at power-up then goes out.
- **The three lighting conditions are mutually exclusive** — High beam, High beam with Adaptive Headlights armed, Adaptive Headlights armed but dimmed — three rows in one slot (§ *Corrections to the first sketch*).
- **The slots are the manual's; the RGB is this repository's** (Tesla publishes no design tokens — § *Could not verify*, Exact colours row), and colour and timing are drawn from state, not an atlas (§ *Asset requirements*). § *The car-status pane*'s diagram shows a green high-beam circle but its bullet list says **Blue**; the list wins.
- **Latch reading:** the source names no cause; implement **the union** (either reason latches) and record the alternative (only a sensor fault latches).

## Requirements

_Numbering is the source's; its requirement 13 (the `IMPLEMENTATION_STATE.md` record entry) is dropped — superseded by `.ai/workflows/task-sequence.md` § *State*._

1. **New module `ui/src/ui_demo/src/indicators.rs`**, declared `mod indicators;` in `ui/src/ui_demo/src/main.rs` beside `mod fps;`. Every public item doc-commented; every value-returning function `#[must_use]`.

2. **`pub enum Severity`**, `#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]`, rank order `Red, Amber, Green, Blue, Grey` — declaration order *is* the order. `#[must_use] pub fn token(self) -> ThemeToken` maps `Red → ThemeToken::Error`, `Amber → ThemeToken::Warning`, `Green → ThemeToken::Success`, `Blue → ThemeToken::Primary`, `Grey → ThemeToken::TextMuted` (this repository's); `#[must_use] pub fn label(self) -> &'static str`. **No new token; `ThemeToken::TOKEN_COUNT` does not move.**

3. **`pub enum Timing`**, four variants, none carrying data: `Steady` (20 of the 23), `FlashWhileActive` (ESC), `FlashOnceThenSolid` (ABS), `LatchedPressure` (tyre pressure — the only rule with memory).

4. **`pub struct Condition`** — `name: &'static str`, `severity: Severity`, `timing: Timing`, `active: bool`, `faulted: bool`. **`Indicators::demo()` builds the 23 rows** in the five bullets' order, so `indicators[0]` is *Brake fault* and the index is the identity. `Indicators::index_of(name) -> Option<usize>`. Two names use the source's exact wording: *"Seat belt unfastened in an occupied seat"* and *"Parking brake applied"*.

5. **`pub struct Indicators`**, five fields: `conditions: Vec<Condition>`; `flashed: Vec<bool>` (one bit per `FlashOnceThenSolid` row); `latched: bool` (written by `tick` and `drive_for` only); `power_up: Duration`; `dwell: Duration`. `#[must_use] pub fn demo() -> Self` (fresh, `power_up` zero, in its self-test). `#[must_use] pub fn len(&self) -> usize`; **no `is_empty`**.

6. **Constants**, `pub`, each doc-commented with its arithmetic and source:

   ```rust
   pub const BLINK_HALF_MS: u32 = 400;
   pub const FLASH_ONCE_MS: u32 = 2 * BLINK_HALF_MS;
   pub const SELF_TEST_MS: u32 = 4 * BLINK_HALF_MS;
   pub const TPMS_CLEAR_KMH: u32 = 25;
   pub const TPMS_DWELL_MS: u32 = 3000;
   pub const INDICATOR_ROW_HEIGHT: f32 = 28.0;
   pub const INDICATOR_DISC: f32 = 16.0;
   pub const INDICATOR_PAD: f32 = 10.0;
   pub const INDICATOR_GAP: f32 = 8.0;
   pub const INDICATOR_NAME_X: f32 = INDICATOR_PAD + INDICATOR_DISC + INDICATOR_GAP;
   pub const INDICATOR_FONT_SIZE: f32 = 16.0;
   ```

   The two multiply out of `BLINK_HALF_MS`, `INDICATOR_NAME_X` out of the other three; `SELF_TEST_MS` is four halves, so the self-test ends dark. **`TPMS_DWELL_MS` has no source at all** — a first-principles choice.

7. **`#[must_use] pub fn flash_lit(elapsed: Duration) -> bool`** = `(elapsed / BLINK_HALF_MS) % 2 == 0`, pure. **`#[must_use] pub fn row_rect(rect: Rect, rank: usize) -> Rect`** — two readers (the disc painter and the demo's label paint); an overflowing row is **not clamped**, the node's own clip cuts it.

8. **Five mutators and four readers, in this order:**

   ```rust
   pub fn arm_self_test(&mut self)
   pub fn set(&mut self, index: usize, active: bool, faulted: bool) -> bool
   pub fn drive_for(&mut self, speed_kmh: u32, delta: Duration) -> bool
   pub fn tick(&mut self, delta: Duration) -> bool
   #[must_use] pub fn phase(&self, index: usize) -> Option<Phase>
   #[must_use] pub fn showing(&self) -> Vec<usize>
   #[must_use] pub fn paint(&self, rect: Rect, theme: &Theme) -> Vec<DrawCommand>
   ```

   `drive_for` accumulates `dwell` while `speed_kmh >= TPMS_CLEAR_KMH`, else zeroes it, and clears `latched` (nothing else) when `dwell` first reaches `TPMS_DWELL_MS`. `arm_self_test` zeroes `power_up` (the only route back), from `Demo::set_car_state` on every entry into `CarState::Parked`. `set` is the only writer of a condition; `tick` the only writer of a `Phase` (once per frame, arming the latch on the frame the tell-tale lights); `phase` returns `None` for a bad index; `showing` is most severe first; `paint` returns discs only. **`Phase` is `Off`, `On`, `Flashing`, `Latched` with `pub fn lit(self) -> bool`** (`Flashing` on the clock, the other two unconditional); while `power_up < SELF_TEST_MS`, `lit` is `flash_lit(power_up)` for every row while `phase` still reports the row's own.

9. **`Demo` gains `indicators: indicators::Indicators` and `indicator_labels: Vec<DemoLabel>`**, the latter 23 built with the existing `read_only_label`, theme `TextMuted`, a tight constraint at `INDICATOR_NAME_X`, `INDICATOR_ROW_HEIGHT` wide minus the left inset. All 23 nodes live under `DemoPane::indicators`, each carrying a `PageMember` row and a `placed_handles` row.

10. **Three rows in `GALLERY_SHORTCUTS`, making the array `[GalleryShortcut; 26]`: `I` toggles the ABS row, `J` the ESC row's `active`, `L` the tyre-pressure row's `active`**; each row's page is `Page::Demo`, each handler writes through `Indicators::set` only, so a key without a row is caught by `no_printable_key_acts_without_a_row_in_the_shortcut_table`.

11. **Fifteen tests**, in `indicators.rs`'s own `#[cfg(test)] mod tests` plus `main.rs`'s `mod tests`; no display, network, filesystem or wall clock, every test a literal `Duration`:
    `flash_lit_alternates_every_blink_half_ms`, `the_duty_cycle_is_exactly_one_half`, `the_column_constants_are_consistent`, `the_twenty_three_conditions_are_the_five_slots_the_section_enumerates`, `the_column_runs_its_whole_self_test_and_ends_dark`, `an_abs_condition_flashes_once_and_then_goes_solid`, `esc_flashes_while_active_and_goes_solid_when_faulted`, `the_tyre_telltale_latches_and_nothing_else_clears_it`, `a_sensor_fault_flashes_and_the_flash_survives_the_fault_clearing`, `showing_is_ranked_and_showing_and_phase_never_disagree`, `the_three_lighting_conditions_are_mutually_exclusive_in_the_defaults`, `the_column_clips_rather_than_overflowing_onto_the_card_row`, `arming_the_self_test_is_the_only_thing_that_restarts_it`, `every_indicator_label_has_a_page_row_and_a_placed_handles_row`, `the_i_j_and_l_keys_reach_the_column_through_handle_event`.

12. **`doc/ui/DEMO_APPLICATION.md` gains two dated, attributed notes and no new row:** one in § *The car-status pane* recording the 23 conditions across the slots, the colour conflict's resolution, the three timing rules, the latch's union arming with the alternative, and the self-test's two triggers; one in § *Could not verify*, the Exact colours, spacing, radii row, that all five colours are theme tokens and every pixel figure a proposal.

14. **The suite, the seven pages and the frame rate are all produced**, as `TASK_UI_DEMO_03` requirement 14 spells out, against a baseline of **1894 (1450 + 224 + 220)**.

## Acceptance Criteria

- [ ] **23 conditions, per-slot 6 / 7 / 4 / 3 / 3 asserted in the failure message;** § *The car-status pane* reconciles *"~20"*; the two exact names appear once; `Severity::token` returns the five tokens, `ThemeToken::TOKEN_COUNT` does not move, and the three lighting conditions are never two at once.
- [ ] **The blink is pure** (`BLINK_HALF_MS - 1` lit and `BLINK_HALF_MS` dark, the duty sweep beside a control at a second half-length, `rg -n 'Cell|RefCell' ui/src/ui_demo/src/indicators.rs` empty); **the timing rules are pinned with second clauses** (ABS does not resume, ESC reads `On` not `Flashing` when `faulted`, the sensor fault reads `On` not `Off`; `rg -n 'FLASH_ONCE_MS|SELF_TEST_MS' ui/src/ui_demo/src/indicators.rs` shows two multiplications and no literals).
- [ ] **The latch clears only by `drive_for(TPMS_CLEAR_KMH, TPMS_DWELL_MS)`** (`drive_for(0, …)` ×100 as control); the self-test restarts only via `arm_self_test`; `main.rs` carries one line in `Demo::set_car_state` on entry into `CarState::Parked`; every row is lit at `Duration::ZERO` and dark at `SELF_TEST_MS`.
- [ ] **`showing`/`phase` agree over all `1 << 23` masks, and the column clips rather than overflowing** (the mirror of `an_overflowing_child_keeps_its_size_and_computes_a_clip_rect`; `TASK_UI_PRIM_45`, `L8`); the three keys reach only their own rows; **no criterion requires a pointer event.**
- [ ] **23 labels, 23 page rows, 23 placed-handles rows, a count beside the membership** (`expected_placed_rect_names` gains all 23; no row of `every_page_places_every_rect_where_the_gallery_placed_it`, `no_two_placed_rects_overlap`, `placed_handles` or `assert_placed_handles_is_complete` loosened).
- [ ] **No leakage:** `git diff --stat` shows one new file (`ui/src/ui_demo/src/indicators.rs`) plus `main.rs` and `DEMO_APPLICATION.md`, nothing under `ui/src/ui_core/`, `ui/Cargo.toml`/`ui/Cargo.lock` unchanged; no `unsafe`/`unwrap`/`expect`/`panic!`/`unimplemented!`/`todo!`; `Page::ALL` seven, `Page::DEFAULT` `Page::Pads`, no `--tab=`. **The suite is green, all fifteen tests named in the handoff**, counts pasted against **1894 (1450 + 224 + 220)**; `cargo audit` recorded as not installed; verification is `.ai/agents/developer.md` § *Phase 3*.
- [ ] **The six unchanged pages are pixel-identical** (AE 0 outside `y ≥ 680`); the seventh shows a disc in two slots, the self-test in a run shorter than `SELF_TEST_MS` (`ROADOS_RUN_SECONDS` whole seconds, `SELF_TEST_MS` 1600), the tyre tell-tale lit after `L` and after an unaccumulated driving dwell; fps measured on all seven (`.ai/tools/fps-check.sh 10 55`, then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=demo`), all above 55, the six inside the band, the seventh's arithmetic reported.
- [ ] **The handoff states, in those words:** discs not glyphs (the 33 baked Lucide PNGs do not cover 23 automotive conditions); the union latch reading with the alternative; `TPMS_DWELL_MS` unsourced; no `ui_core` widget with the promotion condition; the demo cannot receive a pointer event on this host.

## Out of Scope

- **No `StatusLight` widget and no change to `ui_core` of any kind** — demo-local, one consumer; `TASK_UI_PRIM_47`'s `Snapshot` answers the neighbouring need; promotion condition named, not met.
- **No glyph inside the disc and no use of the 33 baked icons** — the disc carries the state; a condition-specific glyph is an asset task.
- **No seatbelt popup, no per-seat tap-to-mute, no "Fasten Seatbelt" label** — § *Corrections to the first sketch* makes these three separate channels; the popup is composite row 13's.
- **No lighting control, no switch, no `Segmented`** — choosing between the lighting conditions is `Controls > Lights`'s.
- **No charge-port lamp protocol** — § *Screen states* puts it in the charging state's content.
- **No blink on anything but these rows; no animation, easing or spring on a light.**
- **No new theme token, no `ThemeScope`, no `ModeScope`, no `Snapshot`, no `Margin`, no `LayoutMode::Grid`, no `polygon_is_convex`, no measured text runs** (tasks 44, 47, 48, 50, 51, 52, 49; `TOKEN_COUNT` does not move).
- **No new dependency and no `unsafe`** — `ui/Cargo.toml`/`ui/Cargo.lock` untouched; approved direct dependencies remain `sdl3 0.20`, `glow 0.18`, `freetype-rs 0.38`; no FFI, no GL, no pointer.
- **Found in the tree and deliberately not fixed:** `main.rs` already imports `ui_core::widgets::toast::Severity` unqualified; the mitigation is that `main.rs` writes `indicators::Severity` and never imports it. Whether the crate's `Severity` should widen to five variants is a `TASK_UI_PRIM_n` question, not opened here.
