# TASK_UI_DEMO_02: The Chrome — a Top Status Bar, a Bottom Dock, and a Persistent Car-Status Pane

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Fill the Tesla tab with its persistent chrome: a top status bar, a bottom dock, and the car-status pane as a persistent region of the map screen rather than a panel over it.
Three regions, at absolute rectangles over the map `TASK_UI_DEMO_01` delivered (one image file, full-bleed, nothing on top, in `main.rs`), in `ui_demo`; the chrome is unaffected.
The demo tab and `--tab=demo` exist; this task does not add, rename or change them, nor `Page::DEFAULT`.

## Context

**In flight.** The three regions are drawn over the map and committed as `cb5de91`, *"partially implemented, layout not finished"*; unreviewed, file still in `doc/ui/`, not `done/`. The leftover list is in `doc/ui/IMPLEMENTATION_STATE_DEMO.md` § *Left over*.

**Evidence.** `doc/ui/DEMO_APPLICATION.md` by section, at classes `[A]` manual/photo, `[B]` release notes, `[C]` snippet-level; the corpus is unreachable from this host, so **every `[C]` claim must be re-verified before it is treated as specified.** § *Screens*: one persistent map screen, the pane a persistent region (no dismissal), its drag-to-dismiss `L4`/`TASK_UI_PRIM_40`.

**Rectangles.** No design tokens (first-principles); the pane's **~40 %** is the photo's. `STATUS_BAR_HEIGHT = 48.0`, `CAR_STATUS_PANE_FRACTION = 0.40`, `DOCK_HEIGHT = 96.0`, `PANE_INSET`, `CARDS_HEIGHT = 96.0` and `PAGER_DOT_RADIUS`, all under `CONTENT_TOP`/`WINDOW`. The three rectangles are pairwise disjoint (`64 ≤ 64 < 112 ≤ 924 < 1020`, held by `the_chrome_nodes_do_not_overlap_each_other`) and none is in `Demo::placed_handles`.

**Dependencies.** `TASK_UI_PRIM_41` (`Painter::backdrop`) is hard for the translucent half only; without it the chrome is an opaque `RoundedRect` at alpha 255, a recorded deviation, not a waiver. `TASK_UI_PRIM_43` (`TabBar`) is not required — the dock's items are actions, not screens, and 43's arrival changes nothing.

**The sentence to amend.** `DEMO_APPLICATION.md` § *Relationship to task 24*: *"**It will need one** if a `TASK_UI_DEMO_n` task ever puts a tab bar on a Tesla surface, and that is the sentence to amend when it does."* This task does; the decision is **the gallery's bar stays**, and removing it is `TASK_UI_PRIM_42`'s `Screens`.

**Baseline and rates.** 2026-10-09: **2058 passing, 1 ignored**; baseline **2058** or **2064** if `TASK_UI_DEMO_01` landed. Frame-rate floor **55 fps on a release build**.

## Requirements

1. **`ui/src/ui_demo/src/chrome.rs`, new, `mod chrome;` beside `mod map;`.** Module doc states the three regions and their over-map placement, that the translucency is `TASK_UI_PRIM_41`'s and what is drawn without it, that the dock glyphs are placeholders, and what `premultiplied` is.

2. **`ChromePalette` and `chrome_palette`.** `#[must_use] pub fn chrome_palette(theme: &Theme) -> ChromePalette` via `theme.get(token).as_color().unwrap_or(fallback)`. Fields `surface`, `surface_opaque` (the only two premultiplied; the latter alpha 255), `foreground`/`Text`, `muted`/`TextMuted`, `active`/`Primary`, `inactive`/`Border` (each a `ThemeToken`). Constants `CHROME_ALPHA: u8 = 200` and `BACKDROP_SIGMA: f32 = 2.5`, each with its reason. Tests `chrome_palette_reads_six_values_and_only_two_are_translucent`, `chrome_palette_falls_back_when_a_token_holds_a_number`, `a_translucent_palette_is_strictly_darker_than_its_opaque_twin`, `a_theme_switch_reaches_every_chrome_surface`.

3. **`fn premultiplied(color: Color, alpha: u8) -> Color`** — private, two call sites, `alpha` a `u8` not a fraction — channels `× alpha / 255` in `u16`, citing `Pixels::premultiply`. Tests `premultiplied_multiplies_each_channel_by_the_alpha_in_u16` (`(200, 200, 200, 128) → (100, 100, 100, 128)` by value) and control `premultiplied_leaves_an_opaque_colour_unchanged`.

4. **`pub const DOCK_GLYPHS: [&str; 5]`**, one ASCII char each in slot order; the five slots are the manual's regions Controls (5), climate controls (driver) (6), My Apps (9), App Launcher (10), Volume Control (13), of which 6, 9, 11 and 12 are conditional; doc says placeholders replaced by `TASK_UI_PRIM_44`'s `ui_core::widgets::Icon`. `DOCK_GLYPHS.len() == DOCK_SLOTS` asserted by `the_dock_is_a_row_of_five_slots_and_the_glyph_table_has_five_entries`; `every_dock_glyph_is_one_non_empty_character`.

5. **The three regions in `Demo::new`, in order, each absolute.** `status_bar`: `Container::new(..., LayoutMode::row())`, `set_flex_config` `MainAxisAlignment::SpaceBetween` + `CrossAxisAlignment::Center`, `Padding::all(STATUS_BAR_PADDING)`, `LayoutMode::Absolute` at `Offset::new(0.0, CONTENT_TOP)`, `Constraints::tight(Size::new(WINDOW.width, STATUS_BAR_HEIGHT))`; three row groups `status_left` (padlock, profile, sentry labels), `status_right` (clock, ambient), `status_airbag`, with no `PRND` or battery. `car_status_pane`: `LayoutMode::column()`, `Offset::new(0.0, CONTENT_TOP + STATUS_BAR_HEIGHT)`, tight to `Size::new(WINDOW.width * CAR_STATUS_PANE_FRACTION, dock_top − CONTENT_TOP − STATUS_BAR_HEIGHT)` (`dock_top = WINDOW.height − DOCK_HEIGHT`); children `prnd_row`, `indicator_column`, `card_row`. `bottom_dock`: `LayoutMode::row()` at `Offset::new(0.0, WINDOW.height − DOCK_HEIGHT)`, tight to `Size::new(WINDOW.width, DOCK_HEIGHT)`, five `Button`s labelled `DOCK_GLYPHS[i]`, each given `font_size`, `padding_h` and `border_radius` before measuring, **no `set_palette`**.

6. **The pane's non-label parts.** `indicator_column`: five `Label`s in order red, amber, green, blue, grey, coloured by `fn themed_color` from `Error`, `Warning`, `Success`, `Primary`, `TextMuted` (`the_indicator_column_is_five_rows_in_red_amber_green_blue_grey_order`). Pager: three `Circle` dots of `PAGER_DOT_RADIUS`, `PAGER_DOT_GAP` apart, first `palette.active`, other two `palette.inactive`. Drive-mode strip: four `Label` rows `↑`, `P`, `↓`, `HOLD` in a `LayoutMode::column()` container inset in the pane (`↑`/`↓` `[C]`, `P`/`HOLD` `[A]`); no gesture.

7. **Four paint arms and the chrome registry.** `pub struct Chrome { pub node: Handle, pub radius: f32, pub palette: ChromePalette }` in `Demo::chrome_nodes: Vec<Chrome>`; `Demo::chrome_of(handle: Handle) -> Option<&Chrome>` (a search, `fn tab_button`'s shape). The arm sits **after the map arm and before the container arm**: `Painter::backdrop(rect, BackdropMode::Blur(BACKDROP_SIGMA), palette.surface)` (only if 41 landed) then `Painter::rounded_rect(rect, chrome.radius, palette.surface)` (or `surface_opaque`). Dock buttons: an arm keyed by `Demo::chrome_button(handle) -> Option<&Button>` calling `button.paint(rect.into(), &chrome_advance(&self.metrics), chrome_line_height(&self.metrics))`. Labels: the existing `fn record_label`, from `chrome_labels: Vec<(DemoLabel, Handle)>` (not `Demo::labels`/`Demo::label_nodes`). Pager: three `Circle`s and nothing else. The **backdrop precedes the surface fill**. Tests `the_backdrop_carries_the_palette_s_premultiplied_surface_and_blur_two_point_five`, `a_pager_paint_arm_records_three_circles_and_nothing_else`, `the_map_node_is_still_underneath_the_chrome_on_the_demo_page`, and `the_chrome_surfaces_draw_their_background_and_their_children_on_top_of_it`.

8. **`page_members` rows, no `focusable`.** `on(Page::Demo, .., false)` for the three surfaces plus the three group nodes (strip, indicator column, card row), because `hit_test` skips an invisible node's subtree; `focusable` stays five. Gates `the_chrome_is_on_the_demo_page_and_on_no_other_page`, `every_chrome_node_records_commands_on_the_demo_page_and_on_no_other_page`.

9. **`fn pager_centre(index: usize, row: Rect) -> (f32, f32)`** — `#[must_use]`, three dots `PAGER_DOT_GAP` apart centred in `row`; `the_pager_places_three_dots_pager_dot_gap_apart_and_centred` asserts the three pairs by value.

10. **The documents.** `DEMO_APPLICATION.md` § *Relationship to task 24*'s standing instruction is **replaced by a dated note** (this task amended it; the gallery's bar stays; removing it is `TASK_UI_PRIM_42`'s `Screens`); § *The persistent chrome* and § *The car-status pane* each gain a dated note of what 02 built and did not; § *Open questions* items 8 (card-carousel depth) and 5 (the car's body) stay. `TASK_UI_PRIM_41.md` and `TASK_UI_PRIM_43.md` untouched.

11. **The dependency as code.** One `if` on `ui_core::paint::BackdropMode` and one `use` in `chrome.rs`, behind a named constant or function; `fn chrome_surface_commands(rect: Rect, chrome: &Chrome, backdrop: bool) -> Vec<DrawCommand>`, `backdrop` from the one `Demo::frame` call site (an argument, not a `cfg!`). Tests `a_chrome_surface_records_its_backdrop_before_its_surface`, `a_chrome_surface_without_a_backdrop_records_only_its_surface`.

## Acceptance Criteria

- [ ] **The seventh tab and `--tab=demo` are verified, not re-added:** `git diff` adds no `Page::Demo` and changes neither `const ALL` nor `const DEFAULT`; `the_demo_tab_exists_from_task_01_and_three_gates_hold_for_it` passes; on release `--tab=demo` opens the demo page, `--tab=dta` names seven pages and refuses, `--help` prints seven, no argument opens `pads`.
- [ ] **The chrome is on screen, in the right places, and does not leak:** a release `--tab=demo` capture (`.ai/tools/README.md` § *Capturing a window*) shows the map, the status bar, five-slot dock and ~40 % pane (PRND, battery, five indicator rows, three cards, three-dot pager, drive-mode strip) as specified, the gallery bar above, in the light theme; the six gallery pages differ by `magick compare -metric AE` = 0 outside the fps band; `the_chrome_nodes_do_not_overlap_each_other`, `the_pager_places_three_dots_pager_dot_gap_apart_and_centred`, `the_indicator_column_is_five_rows_in_red_amber_green_blue_grey_order` and `the_dock_is_a_row_of_five_slots_and_the_glyph_table_has_five_entries` assert the geometry, and `the_seventh_tab_button_lays_out_inside_the_window` still passes.
- [ ] **41 and 43 are facts about the tree, recorded either way:** `git grep -n 'BackdropMode' ui/src/ui_demo/src/chrome.rs` returns the `use` and call site if 41 landed, nothing if not, both backdrop tests pass either way, the branch is stated (landed: a blurred map in the capture; not-landed: the opaque chrome is a deviation, not a waiver, `ui/src/ui_core/src/` unchanged); `git grep -n 'TabBar\|Button::selected'` in `chrome.rs` returns nothing, no `set_palette` on the dock, `the_gallery_tab_bar_is_still_present_on_the_demo_page` passes, `TASK_UI_PRIM_43.md` has no diff.
- [ ] **The `DEMO_APPLICATION.md` amendment is dated and attributed,** the withdrawn-2026-10-03 four-file table neither edited nor re-listed, and § *Open questions* item 8 stays.
- [ ] **The premultiplied arithmetic is asserted by value and the mutation is killed:** `premultiplied_multiplies_each_channel_by_the_alpha_in_u16` asserts `(200, 200, 200, 128) → (100, 100, 100, 128)` with `premultiplied_leaves_an_opaque_colour_unchanged` as control, and a `u8` multiply fails both; paste the three deliberate breaks' failure output (swap the `backdrop`/`rounded_rect` calls, premultiply in `u8`, empty the dock's third label).
- [ ] **Verification is `.ai/agents/developer.md` § *Phase 3* plus the task-specifics:** `cargo test --all-features` green from baseline **2058** or **2064** with every named test listed and none removed or renamed; no file under `ui/src/ui_core/src/` or `ui/src/ui_demo/assets/`, only `chrome.rs` new under `ui/src/ui_demo/src/`; manifests unchanged with direct deps `sdl3 0.20`, `glow 0.18`, `freetype-rs 0.38`; no `unsafe`, `unwrap()`, `as` or new `pub` in `chrome.rs`/`ui_core`; `Demo::placed_handles` and `tests::expected_placed_rect_names` gain no row, `tests::assert_placed_handles_is_complete` and the `focusable` count of five unmoved; `cargo audit` not installed; `layout_walk_cost` still ignored and `layout.rs` unchanged.
- [ ] **The frame rate is measured on all seven pages** (`.ai/tools/fps-check.sh`, then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>`); every page above 55, the six gallery pages in the recorded band, the demo page's rate reported whatever it is.
- [ ] **The handoff states its deviations:** placeholder dock glyphs (replaced by `TASK_UI_PRIM_44`), the opaque-chrome fallback if 41 did not land, and that no slot, drive-mode target, clock reading, carousel or pane resize is implemented.

## Out of Scope

- No panels, popups, app tray, app launcher, media player, alert slot, theater or browser; the dock's five buttons are wired to nothing.
- No gesture of any kind (drag-to-dismiss, swipe, long press, pinch, two-axis reshape): `InputEventKind::Swipe` and `LongPress` have no consumer, so no criterion may require pointer interaction.
- No `Icon` widgets, no asset file and no `TASK_UI_PRIM_39` pipeline; the dock glyphs become `Icon`s when `TASK_UI_PRIM_44` lands, and task 01's image `ui/src/ui_demo/assets/img/map_demo.png` is not in this repository.
- No `Screens`, screen stack, transitions or panel dismissal (`TASK_UI_PRIM_42`, gap `#3`, not started).
- No state machine (ten conditional top-bar items, blink, latch, severity timing) and no clock — the readings are static `Property<String>` strings.
- No 3-D car, floor reflection, hotspot callouts, leader lines, charge-port or lock glyphs; the pane's middle is the map's surface.
- No composite-widget behaviour (per-field staleness, carousel paging, per-card scroll, card-strip drag-to-dismiss; composites 1–3, `L5`/task 46) or dock edit mode, tray-to-dock drag or eviction (composite 10).
- No light/dark switch inside the demo tab and no second visual design; `T` switches the whole window and the chrome follows through the property graph. No `PRND` selector, drive-mode gesture or Neutral press-and-hold.
- No change to the gallery, tab bar, `Page` or any other page; no `ui_core` change (requirement 11 uses `Painter::backdrop` if 41 landed, `Painter::rounded_rect` always).