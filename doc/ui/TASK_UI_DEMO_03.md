# TASK_UI_DEMO_03: The Car-Status Pane — the Pane, the Car, and Three Mutually Exclusive States

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Build the car-status pane as the seventh tab's content: a **~40 %**-wide left pane with a severity-ranked indicator column on its left, a 3-D car on a floor in the middle, a card row at its foot and a page indicator under that, carrying **three mutually exclusive states** — parked, driving and charging — switched by three keys. The ambient rotation, the floor and Track Mode's per-sub-mesh tint are settled here; the pane's width is one argument to one function, so `TASK_UI_DEMO_05`'s two-axis reshape is not a rewrite.

## Context

Builds on `TASK_UI_DEMO_02`'s `ui/src/ui_demo/src/panel.rs` and `chrome.rs`. Library tasks 34 through 52 are specified and not built (`.ai/tools/README.md` § *The task table*), and `TASK_UI_DEMO_01`/`02` are absent, so this file states structural facts rather than citations.

**The pane.** § *The car-status pane*: a severity-ranked indicator column at the left (the ~20 conditions are `TASK_UI_DEMO_04`'s; this task builds the box), the *"3-D car, floor reflection"* at centre, the `Open Frunk`/`Open Trunk` callouts, the lock above the roof and the charge port at rear-left, and the card row plus *"· · · (page indicator)"* at the foot. § *Screens*: *"persistent region of the map screen"*, dismissed *"never; resize drag only"*. § *Asset requirements*: *"Tesla publishes no design tokens at all"*, so every colour and rect is derived.

**Three states.** § *Screen states of the car-status pane* names them mutually exclusive: *"parked"*, *"driving (or ready to drive)"*, *"charging"*; range is under parked and driving, and charging shows the charge state and the range. § *Composite widgets* row 4 demands the state that decides a control's presentation be separate from its value. § *Could not verify* records Track Mode's car-body recolouring rules as `[B]`, *"not a spec the demo should copy until verified"*.

**The car and floor.** `TASK_UI_PRIM_40`'s `Rotator` is the one rotator: one `MeshId`, one ambient term, **one node with a second placement**, at task 40's aspect (`1.412` against `1.400`). `Polygon` is convex-only (`L10`; `TASK_UI_PRIM_51`'s `paint::polygon_is_convex`), no ground mesh exists, and the mirrored reflection cannot be drawn because `TASK_UI_PRIM_37` leaves the front face at `GL_CCW`, a window-space convention, so an orientation-reversing transform culls visible faces. This task draws **two convex `DrawCommand::Rect`s** and one `DrawCommand::Shadow` at the rest pose; the reversal is requirement 10's `L12`.

## Requirements

_Numbering is the source's; its requirement 13 (the `IMPLEMENTATION_STATE.md` record entry) is dropped — superseded by `.ai/workflows/task-sequence.md` § *State*._

1. **Rename `CAR_ORIGIN`→`DATA_CAR_ORIGIN` and `CAR_SIZE`→`DATA_CAR_SIZE` in `ui/src/ui_demo/src/main.rs`, and every site naming them**; this task's pair takes the plain names. `CAR_DISTANCE`, `CAR_TARGET_Y`, `CAR_REST_YAW`, `CAR_REST_PITCH`, `AMBIENT_YAW_RATE`, `CAR_RECT` stay.

2. **Constants in `ui/src/ui_demo/src/main.rs`, each `const`, doc-commented with its derivation and the sentence that it is a proposal where it is one:**

   ```rust
   const PANE_WIDTH: f32 = WINDOW.width * 0.40;
   const PANE_TOP: f32 = CONTENT_TOP;
   const PANE_BOTTOM: f32 = FPS_READOUT_ORIGIN.1;
   const PANE_HEIGHT: f32 = PANE_BOTTOM - PANE_TOP;
   const INDICATOR_WIDTH: f32 = 176.0;
   const CARD_ROW_HEIGHT: f32 = 120.0;
   const PAGE_INDICATOR_HEIGHT: f32 = 24.0;
   const CARD_ROW_TOP: f32 = PANE_BOTTOM - CARD_ROW_HEIGHT - PAGE_INDICATOR_HEIGHT;
   const BODY_HEIGHT: f32 = CARD_ROW_TOP - PANE_TOP;
   const CAR_AREA_WIDTH: f32 = PANE_WIDTH - INDICATOR_WIDTH;
   const CAR_SIZE: Size = Size { width: CAR_AREA_WIDTH, height: 238.0 };
   const CAR_ORIGIN: (f32, f32) = (
       INDICATOR_WIDTH,
       PANE_TOP + (BODY_HEIGHT - CAR_SIZE.height) * 0.5,
   );
   ```

   `CAR_SIZE.height` is the literal `238.0` (ratio `1.4118`, within 1 % of task 40's `1.400`).

3. **`#[must_use] fn pane_rects(width: f32) -> PaneRects`** returns seven rects from one width: `pane` (`Rect::new(0.0, PANE_TOP, width, PANE_HEIGHT)`), `indicators` (`TASK_UI_DEMO_04`'s empty box), `body`, `car_area` (`PANE_TOP..CARD_ROW_TOP`), `card_row` (`TASK_UI_DEMO_05`'s filled host), `dots`, `charge_lamp` (a `Size` 12 × 12 at the car's rear-left, charging only). `width` is unclamped; `pane_rects(0.0)` is a zero-width pane, not an error.

4. **`enum CarState` in `main.rs`, beside `enum Page` and not inside it:** `#[derive(Clone, Copy, PartialEq, Eq, Debug)] enum CarState { Parked, Driving, Charging }`, with `const ALL: [CarState; 3]`, `const DEFAULT: CarState = CarState::Parked` (named, not the first variant), `#[must_use] fn key(self) -> Keycode`, `#[must_use] fn name(self) -> &'static str`, `Demo::car_state: CarState`; no `Default` derive.

5. **Four rows in `GALLERY_SHORTCUTS`** (`[GalleryShortcut; 23]`): `Keycode::Z` (parked), `X` (driving), `V` (charging), `B` (Track Mode), each with a `Some(Page::Demo)` page and a `fn(&mut Demo)` handler; all four keys free, per `no_printable_key_acts_without_a_row_in_the_shortcut_table`.

6. **`Demo::set_car_state(&mut self, state: CarState) -> bool`, one write path, in that order and no other:** `if self.car_state == state { return false; }`, then `self.car_state = state`, then the readouts' visibility sync (requirement 7), then `self.sync_page_visibility()`; it returns whether it moved. `TASK_UI_DEMO_04` adds exactly one line, the self-test arm.

7. **`struct CarReadouts` and the per-state table in `main.rs`:** five `Option<DemoLabel>` fields `drive_mode`, `range_km`, `speed_kmh`, `set_speed_kmh`, `charge_state`, and

   ```rust
   const STATE_READOUTS: [[bool; 5]; 3] = [
       [true,  true,  false, false, false], // Parked:   drive mode, range.
       [false, true,  true,  true,  false], // Driving:  range, speed, set cruising speed.
       [false, true,  false, false, true ], // Charging: range, charge state.
   ];
   ```

   `Demo::readout_visible(&self, index: usize) -> bool` reads `self.car_state`'s row; `Demo::car_readout_rect(index)` places each readout in `PaneRects::body`'s right column, declaration order, `READOUT_PITCH` apart; the unset `set_speed_kmh` em dash is written by a private `fn set_speed_text(&mut self) -> bool`.

8. **The nodes, and a `PageMember` row and a `placed_handles` row for each.** `Demo` gains `pane: DemoPane` = `struct { root, indicators, car, cards, dots, car_node: Handle }` with `impl DemoPane { #[must_use] fn node(&self) -> Handle { self.root } }`, attached to `Demo::root`. Each of the six handles gets a `PageMember { page: <the seventh variant>, focusable: false, always: false }` row, a `("pane …", handle)` row in `Demo::placed_handles`, and a name in `expected_placed_rect_names`. `car_node`'s rect is `Rect::new(CAR_ORIGIN.0, CAR_ORIGIN.1, CAR_SIZE.width, CAR_SIZE.height)`; `Demo::car_at` unchanged.

9. **The pane's paint, seventh page only, in this order** (`TASK_UI_PRIM_37` § *Where the mesh pass sits*): (1) `Painter::rounded_rect` on `PaneRects::pane` in the theme's `Surface`; (2) `Painter::rect` on `PaneRects::body` in `ThemeToken::Background`; (3) floor, two `Painter::rect`s, horizon then ground; (4) one `Painter::shadow`; (5) car, five `Painter::mesh` in `Model::ranges` order — `body`, `wheel-front-left`, `wheel-front-right`, `wheel-back-left`, `wheel-back-right` — all one `mvp`; (6) charge-port lamp, charging only, `Painter::circle` on `PaneRects::charge_lamp`; (7) one `Painter::text_in_measured` caption; (8) readouts, each on its own node; (9) card row and dots band surfaces. The matrix is task 40 requirement 10's with `aspect = CAR_SIZE.width / CAR_SIZE.height`; the `Option` is handled and not unwrapped.

10. **`Row L12` appended to `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*,** dated and attributed, severity **Low**, Blocks *"a mirrored or foreshortened copy of any mesh — a car reflected on a floor, a contact shadow that tracks the yaw"*: no winding flag or cull toggle on `DrawCommand::Mesh`, while `draw_mesh_batch` brackets with `GL_CULL_FACE`/`GL_BACK` and leaves the front face at `GL_CCW` (window-space), so an orientation-reversing transform culls visible faces. Evidence by symbol: `DrawCommand::Mesh` in `ui/src/ui_core/src/paint.rs`, `draw_mesh_batch` in `ui/src/ui_core/src/render.rs`, `TASK_UI_PRIM_34` § *Face culling: off*. Appended after `L11`, no renumber.

11. **Tests, in `main.rs`'s existing `mod tests`, no display/network/filesystem/wall clock; fixture `demo_on(Page::Demo)`:** `pane_rects_agrees_with_every_constant_it_is_built_from`; `pane_rects_moves_every_rect_when_its_width_moves`; `the_pane_reaches_neither_the_tab_bar_nor_the_readout_band`; `the_car_rect_matches_task_forty_aspect_within_one_percent`; `every_state_shows_its_own_readouts_and_shares_only_the_range`; `switching_state_moves_the_gates_and_nothing_else`; `the_pane_records_five_mesh_commands_and_no_more`; `the_car_is_recorded_before_the_chrome_that_sits_over_it`; `the_floor_is_two_convex_rects_and_the_shadow_is_one`; `track_mode_tints_the_body_and_the_wheels_differently`; `the_tint_granularity_caption_names_body_and_wheels`.

12. **`doc/ui/DEMO_APPLICATION.md` gains three dated, attributed notes, none closing or renumbering a row:** § *The car-status pane* — built at `PANE_WIDTH` in three states, the floor is 2-D and the mirrored image is not drawn (`L12`), the shadow is the rest pose's. § *Could not verify*'s Track Mode row — the mechanism and granularity limit, no colour rule. § *Composite widgets* row 4 — the first `ModeScope` consumer of Track Mode, body-against-wheels.

14. **The suite, seven pages and frame rate** per `.ai/agents/developer.md` § *Phase 3*; task-specific: `cargo test --all-features` against baseline **1894 (1450 + 224 + 220)** with requirement 11's eleven tests present by name.

## Acceptance Criteria

- [ ] **The pane is on screen at 40 % of the window, in three states, and the states differ.** `--tab=demo` shows a pane `WINDOW.width × 0.40` wide from `CONTENT_TOP` to `FPS_READOUT_ORIGIN.1` holding the indicator column's empty box, the car on a floor with a contact shadow, and the card row's and dots band's filled surfaces; `Z`/`X`/`V` switch parked/driving/charging and each capture differs beyond the readout's band.

- [ ] **Every pane rect comes from `pane_rects(width)` and nothing else.** `rg -n 'pane_rects' ui/src/ui_demo/src/main.rs` shows the definition, its one paint-path caller and its tests, and no arithmetic outside it builds a pane rect; `pane_rects_agrees_with_every_constant_it_is_built_from` and `pane_rects_moves_every_rect_when_its_width_moves` (at `PANE_WIDTH`, `WINDOW.width`, `0.0`, unclamped) pass; `pane_reaches_neither_the_tab_bar_nor_the_readout_band` holds and the readout is uncovered.

- [ ] **Three states, one write path, and the gates move.** `Z`/`X`/`V` go through `Demo::set_car_state` (the only writer, a no-op for the current state); `switching_state_moves_the_gates_and_nothing_else` asserts `STATE_READOUTS` visibility, `on_show` false for a hidden readout and `Demo::car_rect()` unchanged; `every_state_shows_its_own_readouts_and_shares_only_the_range` asserts the complement.

- [ ] **The car is five mesh commands before the chrome over it.** `the_pane_records_five_mesh_commands_and_no_more` counts exactly five `DrawCommand::Mesh` in `Model::ranges` order; `the_car_is_recorded_before_the_chrome_that_sits_over_it` asserts the first mesh precedes the caption and the card row's surface.

- [ ] **Floor two convex rects, one shadow, `L12` recorded, tint body-against-wheels.** `the_floor_is_two_convex_rects_and_the_shadow_is_one` checks `paint::polygon_is_convex`, exactly one `DrawCommand::Shadow` and no mirrored matrix; § *Gaps this layout exposes in `ui_core`* carries row `L12` by symbol, after `L11`, no renumber. `track_mode_tints_the_body_and_the_wheels_differently` (equal in `Standard`, body = `Warning`, wheel = `Primary` in `Track`) and `the_tint_granularity_caption_names_body_and_wheels` pass, and § *Could not verify*'s Track Mode row and composite row 4 gain dated notes.

- [ ] **No leak, and the suite is green.** No change under `ui/src/ui_core/`; `ui/Cargo.toml`/`ui/Cargo.lock` unchanged (`sdl3 0.20`, `glow 0.18`, `freetype-rs 0.38`); no new `unsafe`, `unwrap`, `expect`, `panic!`, `unimplemented!` or `todo!` outside `demo_on`'s `.unwrap()`; `Page::ALL` `[Page; 7]`, `DEFAULT` `Page::Pads`. `cargo test --all-features` green with all eleven named tests against baseline **1894 (1450 + 224 + 220)**; `cargo audit` recorded as not installed.

- [ ] **Six pages pixel-identical, and the frame rate on all seven.** The six original pages give **AE 0 outside `y ≥ 680`**, because no node on them is a pane node and the ambient term is still one; `every_page_places_every_rect_where_the_gallery_placed_it`, `no_two_placed_rects_overlap`, `placed_handles`, `expected_placed_rect_names` and `assert_placed_handles_is_complete` keep their names, six added. `.ai/tools/fps-check.sh 10 55` and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=demo` per page; all above 55.

- [ ] **The handoff claims no more.** The pane's centre is not a map; the road visualisation, other cars, speed-limit sign and power meter are absent, so driving differs by three readouts. The floor is 2-D (`L12`). Track Mode's tint is body-against-wheels, no colour rule copied. The seventh page's variant name and `--tab=` spelling are `TASK_UI_DEMO_01`'s; `Demo::car_rect()` is unchanged.

## Out of Scope

- **No map, and nothing that claims to be one** — gap `#1` stays a `TASK_UI_DEMO_n` item; task 01's map is a picture, and the pane's centre is a `ThemeToken::Background` rect over it. The road visualisation, other cars (row 7), power meter (row 6), lane markers (row 8) and speed-limit sign are not built.
- **No indicator lights, no glyphs and no column contents** — the ~20 conditions, five colours, three timing rules, the latch and the power-up self-test are `TASK_UI_DEMO_04`'s.
- **No card, no pager dot and no carousel** — both are `TASK_UI_DEMO_05`'s; `N`/`U`/`R`/`E`/`M` are not bound.
- **No callout, no leader line, no lock glyph and no charge-port glyph** — composite row 3 and `TASK_UI_DEMO_05`'s. The charge-port lamp is a `Circle` at `PaneRects::charge_lamp`; 05's hotspot is its icon above it.
- **No reshape, no tier and no zoom** — `TASK_UI_DEMO_05` § *The two-axis reshape* owns the tier, snap, pinch and `[C]` tag.
- **No mirrored geometry, no foreshortened shadow, and no pipeline change** — no `front_face` call, winding flag or cull toggle is added to `ui_core`.
- **No Track Mode colour rule, no per-component temperature and no per-tyre grip tint** — § *Could not verify* binds this task.
- **No translucent chrome and no backdrop** — composite row 5 is `L1`'s; the surface is an opaque `rounded_rect` in the theme's `Surface`.
- **No `Margin`, no `shrink`, no `LayoutMode::Grid`, no `ThemeScope`, no `Snapshot`** — all available and unused; `ModeScope` is used.
- **No `Page` change, no `--tab=` name and no eighth gate** — `Page::ALL` is seven and `Page::DEFAULT` is `pads`, as `TASK_UI_DEMO_01` left them.
- **No new dependency and no `unsafe`** — `ui/Cargo.toml` and `ui/Cargo.lock` are untouched.
- **Found in the tree and deliberately not fixed** — `TASK_UI_DEMO_01` and `TASK_UI_DEMO_02` are specified and not built.
