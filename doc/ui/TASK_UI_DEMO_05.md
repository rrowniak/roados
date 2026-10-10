# TASK_UI_DEMO_05: The Card Carousel, the Callout Hotspots, and the Two-Axis Reshape

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Finish the car-status pane's contents in three sub-tasks: 05-1 the swipeable card
carousel with a dot pager and per-field staleness, 05-2 the callout hotspots
anchored in model space, 05-3 the two-axis reshape.
No pointer event reaches this window on this host, so nothing here is verified by
a tap: every gesture has a key that drives the same write path, and a unit test
over literal numbers.

## Context

**Dependencies, none of it built.** `TASK_UI_DEMO_03`'s `PaneRects`: `card_row`
and `dots` are the bands 05-1 fills, `pane_rects(width) -> PaneRects` is 05-3's
seam, and the car matrix must be one composition named `car_mvp(&self) ->
Option<Mat4>`. `TASK_UI_PRIM_44` supplies `Icon`, `DrawCommand::Image`'s tint, and
`lock` and `plug-zap`; `TASK_UI_PRIM_45` the per-node clip; `TASK_UI_PRIM_49` the
`DrawCommand::Text` width; task `36`'s `Mat4::transform_point`, `perspective`,
`Mat4::scaled`; task `38`'s model bounds 05-2's anchors use. `TASK_UI_PRIM_46`
supplies `Scroll`; 05-1's pager **is** a `Scroll` with three snap points.
`TASK_UI_PRIM_47` is unused.

**05-1 — the carousel and the per-field staleness.** The card count and order are
unverified: the manual-as-transcribed names `Media`, `Tire Pressure`, `Trip`, and
this task ships those three in that order, photo `01`'s 3-dot pager agreeing on
the count. § *Composite widgets* row 1's three `[B]` claims — two-level Media
card, Track Mode default page, dismissible strip — are declined for their tag.

**05-2 — the callout hotspots.** § *Composite widgets* row 3 names four anchors:
leader-lined `Open Frunk` and `Open Trunk`, a lock glyph above the roof, a
charge-port glyph at the rear-left. The model's facing is recorded nowhere
(`TASK_UI_PRIM_40`'s `CAR_REST_YAW`), so anchors are fractions of the model's own
bounding box.

**05-3 — the two-axis reshape.** The only source is a `[C]` sentence in § *The
car-status pane* (Tesla's manual, corpus unreachable from this host). Three
repository-owned facts survive its demotion: the pane is *"narrower"* in photo `02`
than photo `01`; *"Left ~40 %"* is `PANE_WIDTH`; § *Composite widgets* row 16
demands *"One axis discrete-and-tiered, one axis continuous"*. The tier's content
— road markings, stop lights, objects — is map layers and there is no map (gap
`#1`). The pinch is declined: `[C]`, and `TASK_UI_PRIM_40` requirement 3 already
lists `InputEventKind::Pinch` among the variants `Rotator::on_event` declines.

## Requirements

### 05-1 — the carousel and the per-field staleness

1. New module `ui/src/ui_demo/src/carousel.rs`, declared `mod carousel;` in
   `ui/src/ui_demo/src/main.rs`, so the demo carries **three** modules.

2. `pub struct Pages` (one field `viewport: Rect`); `#[must_use] pub fn
   page_width(&self) -> f32` (`viewport.width`), `content_width(&self) -> f32`
   (`viewport.width * PAGE_COUNT as f32`), `snap_points(&self) -> [f32; 3]`;
   **`PAGE_COUNT: usize = 3`**, a `pub const` from the two sources' count
   agreement. Doc: no peek — `set_snap_points` drops points outside
   `0.0..=max_scroll` (`max_scroll.x = content_width - viewport_width`).

3. `#[must_use] pub fn page_of(offset: f32, page_width: f32) -> usize` =
   `(offset + page_width * 0.5) / page_width` clamped into `0..PAGE_COUNT`, a tie
   to the earlier page, per `TASK_UI_PRIM_46`'s `nearest_point`.

4. `PAGE_MEDIA: usize = 0`, `PAGE_TYRES: usize = 1`, `PAGE_TRIP: usize = 2`,
   `PAGE_LABELS: [&str; 3] = ["Media", "Tire Pressure", "Trip"]`; the doc cites §
   *Could not verify*'s carousel row.

5. One `Scroll`: `Scroll::new`, `set_axis(Axis::Horizontal)`,
   `set_content_width(pages.content_width())`, `set_snap_points(&pages.snap_points())`,
   `Scroll::tick(rect, delta)` in `Demo::frame` beside the current ticks; the
   settle is `Scroll::snap_to_points(rect, SNAP_MOTION)`. Task 46's `SNAP_MOTION`
   is private, so write `Motion { duration: Duration::from_millis(200), easing:
   Easing::EaseOut }`, asserted `assert_ne!`-style against it. `GALLERY_SHORTCUTS`
   rows **`N`** (next) and **`U`** (previous), each `Scroll::scroll_by(rect,
   Offset::new(±page_width, 0.0))` then `snap_to_points`.

6. `pub fn dots(rect: Rect, page: usize, theme: &Theme) -> Vec<DrawCommand>`, one
   `DrawCommand::Circle` per page, selected `DOT_ACTIVE_RADIUS`, unselected
   `DOT_RADIUS`, filled `Severity::Grey`/`Severity::Blue` (the tokens
   `TASK_UI_DEMO_04`'s `Severity::token` returns); **`DOT_RADIUS = 3.0`,
   `DOT_ACTIVE_RADIUS = 5.0`**, proposals per § *Asset requirements*.

7. `pub enum Corner`; `TyreReading { pub corner: Corner, pub psi: u32, pub age:
   Duration }`; `pub struct Tyres`; the age is **stored**. `Tyres::demo()`,
   `Tyres::age(&mut self, delta: Duration) -> bool` (all four advance together,
   seeded `16`, `15`, `16`, `17` minutes), `#[must_use] fn reading(&self, corner:
   Corner) -> Option<&TyreReading>`, `#[must_use] fn line(&self, corner: Corner)
   -> Option<String>`, `recommended(&self) -> String` = `"Recommended Front: 42 /
   Rear: 42"`.

8. `pub fn relative_age(age: Duration) -> String`, `#[must_use]`, pure: `"just
   now"` under a minute, `"N minutes ago"` to 59, `"N hours ago"` to 23, `"N days
   ago"` beyond; thresholds are named `pub const`s; seconds never shown.

9. One card host, three card bodies, each its own node under the pager's node:
   four `DemoLabel`s at the readings' quadrant positions, one for the recommended
   block, two per non-tyre card; each label carries a `PageMember` row and a
   `placed_handles` row, `assert_placed_handles_is_complete` the instrument.

10. Tests in `carousel.rs`'s `mod tests` and `main.rs`'s:
    `the_three_pages_snap_to_three_points_and_the_last_is_reachable` (`[0.0, 512.0,
    1024.0]`, `max_scroll_for` `1024.0`);
    `a_peeking_pager_would_put_its_last_page_out_of_range` (`PEEK` `1..=64`);
    `page_of_rounds_a_half_drag_to_the_earlier_page` (ties earlier, `w` a literal
    `512.0`); `two_equal_pressures_with_different_ages_render_different_lines`
    (`42 psi`, 16 and 15 minutes); `the_recommended_block_carries_no_timestamp`;
    `relative_age_is_a_pure_function_of_a_duration` (`0`, `59 s`, `60 s`, `16 min`,
    `59 min`, `60 min`, `23 h`, `24 h`);
    `the_n_and_u_keys_move_the_pager_one_page_and_stop_at_the_ends` (`0 1 2 2 1 0
    0`); `the_dot_pager_draws_three_circles_and_the_selected_one_is_larger`.

### 05-2 — the callout hotspots

11. New module `ui/src/ui_demo/src/hotspots.rs`, declared `mod hotspots;`. `pub
    struct Hotspot { pub id: HotspotId, pub at: [f32; 3] }` — fractions of the
    model's own bounding box, `0.0` at `bounds_min`, `1.0` at `bounds_max`.
    `pub enum HotspotId { Frunk, Trunk, Lock, ChargePort }`; `HOTSPOTS: [Hotspot;
    4]`; `#[must_use] fn world_point(&self, bounds: ([f32; 3], [f32; 3])) ->
    [f32; 3]` (lerp per component); **`#[must_use] fn project(mvp: &Mat4,
    car_area: Rect, world: [f32; 3]) -> Option<Offset>`** = `mvp.transform_point(world)`,
    `None` behind the eye, else `car_area.x + (ndc.x + 1.0) * 0.5 * car_area.width`
    and `car_area.y + (1.0 - ndc.y) * 0.5 * car_area.height`.

12. Two leader-lined buttons, two free-floating icons: `Hotspot` gains `leader:
    bool`; `Frunk`/`Trunk` have it, `Lock`/`ChargePort` do not. The leader is
    `Painter::path(&[(x0, y0), (x1, y1)], LEADER_WIDTH, color, false)`,
    `DrawCommand::Path` *stroked*. Buttons are `Button`s (`Open Frunk`, `Open
    Trunk`), glyphs are `Icon`s over `lock` and `plug-zap`, the charge-port glyph
    offset above `PaneRects::charge_lamp`, with a test asserting the two rects do
    not intersect.

13. `Hotspots::set_revealed(bool)` is the only write path. `#[must_use] pub fn
    revealed(&self) -> bool`, `at(&self, position: Offset) -> Option<HotspotId>`,
    `paint(&self, car_area: Rect, mvp: &Mat4) -> Vec<DrawCommand>`; `at` is
    `Demo::slider_at`'s shape with the page gate first (`if !self.on_show(node) {
    return None; }`, then unrevealed, then the rect test). `GALLERY_SHORTCUTS` row
    **`R`** toggles the reveal.

14. Tests: `world_point_lands_inside_the_bounding_box_for_every_hotspot`;
    `project_puts_a_point_behind_the_eye_nowhere`;
    `project_maps_the_ndc_corners_onto_the_car_area`;
    `a_leader_is_a_stroked_path_and_not_a_polygon`;
    `the_lock_and_the_charge_port_have_no_leader_and_the_two_buttons_have_one`;
    `the_charge_port_glyph_and_the_charge_lamp_do_not_intersect`;
    `at_finds_each_hotspot_at_its_own_anchor_and_nothing_else`;
    `at_answers_none_while_the_hotspots_are_hidden`;
    `the_r_key_reveals_them_through_handle_event`.

### The two-axis reshape

This is sub-task **05-3**; the labels `05-1`/`05-2`/`05-3` in § *Context* name its three parts.

15. New module `ui/src/ui_demo/src/reshape.rs`, declared `mod reshape;`. `pub enum
    Tier { Split, Full }`; `#[must_use] fn width(self) -> f32` (`Split` =
    `PANE_WIDTH`, *"~40 %"*; `Full` = `WINDOW.width`), `label(self) -> &'static
    str`.

16. `pub struct Reshape`: `width: Property<f32>`, `target: Option<Tier>`,
    `origin: f32`, `zoom: Property<f32>`. `#[must_use] pub fn new(nodes: &mut
    Arena<WidgetNode>) -> Self` at `Tier::Split.width()`, zoom `1.0`;
    `#[must_use] pub fn tier(&self) -> Tier`; `pub fn begin_drag(&self, at: f32)
    -> bool`; `drag_to(&self, at: f32) -> bool`; `end_drag(&self, motion: Motion)
    -> bool`; `settle_to(&self, tier: Tier, motion: Motion) -> bool`;
    `step_tier(&self, motion: Motion) -> bool`; `#[must_use] pub fn zoom(&self) ->
    f32`; `set_zoom(&self, zoom: f32) -> bool`; `step_zoom(&self, up: bool) ->
    bool`.

17. `DRAG_GAIN: f32 = 1.0` and `ZOOM_STEPS: [f32; 5] = [0.70, 0.80, 0.90, 1.00,
    1.10]`, both derived in their docs. `DRAG_GAIN` 1.0 because the sentence says
    drag side to side (a 768-pixel drag reaches `Full` from `Split`).
    `ZOOM_STEPS`' upper bound: visible width `2 · d · tan(fov_y/2) · aspect`, `d =
    CAR_DISTANCE / zoom = 2.6 / zoom`, `fov_y = π/4`, `aspect = 1.4118`; the car is
    2.55 m, inside for `zoom ≤ 3.041 / (2.55 · 1.05) = 1.135`; `1.10` is inside,
    5 % margin; a test asserts `≥ 2.55` per step.

18. `end_drag` settles to the nearer tier, a tie to the narrower, on
    `TASK_UI_PRIM_46`'s `nearest_point` rule cited by name; it calls
    `width.animate_to(target, motion.duration, motion.easing)` — the crate's *"aim
    once, tick per frame"* idiom, not a per-frame assignment.

19. `GALLERY_SHORTCUTS` rows **`E`** (`step_tier`) and **`M`** (`step_zoom`), plus
    an arm in `Demo::route_input_event` beside task 40's rotator arm, so a `Drag`
    reaches `begin_drag`/`drag_to`/`end_drag` and key and drag share the three
    calls.

20. `Demo::frame`'s per-frame block grows two lines: `pane_rects(self.reshape.width.get())`
    where task 03 passes the constant, and `orbit(yaw, pitch, CAR_DISTANCE /
    self.reshape.zoom.get())` where task 03 passes `CAR_DISTANCE`; task 03's
    `CAR_DISTANCE` and `orbit`'s signature are unchanged.

21. `#[must_use] pub fn tier(&self) -> Tier` on `Demo` — the tier's published
    value for whichever task draws the map, named in § *Out of Scope*.

22. Tests: `a_drag_to_the_right_settles_into_the_wide_tier_and_a_short_one_does_not`;
    `a_tie_goes_to_the_narrower_tier`; `drag_to_writes_nothing_while_no_drag_is_down`;
    `every_zoom_step_keeps_the_car_inside_its_rect`;
    `set_zoom_rejects_a_value_outside_the_steps_and_step_zoom_never_leaves_them`;
    `the_readout_stays_visible_at_both_tiers`;
    `the_e_key_steps_the_tier_and_the_m_key_steps_the_zoom_through_handle_event`.

23. A row added to `DEMO_APPLICATION.md` § *Could not verify*, named *"The
    car-status pane's two-axis reshape"*, its *Searched* column *"nothing further
    — `tesla.com` returns 403 and `rollout-tesla.com`'s GUID pages return Akamai's
    Access Denied, both re-tested 2026-10-05 per § *Sources*; this task ran no
    search of its own"*, its *Result* column the `[C]` demotion, the two
    `[A]`-observational facts that survived it, and which half this task built and
    which half it declined. No row closed or renumbered; § *Composite widgets* row
    16 gains a dated note naming the same split, and the two cannot disagree.

## Acceptance Criteria

- [ ] **The tests named in requirements 10, 14 and 22 all pass**, the handoff
      lists all twenty-four by name against a baseline of **1894 (1450 `ui_core` +
      224 `ui_demo` + 220 doctests)**, each count pasted, none deleted, renamed
      away or weakened, the three sub-tasks' counts separate. Verification is
      `developer.md` § *Phase 3*.

- [ ] **Five rows reach `GALLERY_SHORTCUTS`, none a pointer.** `N`, `U`, `R`, `E`
      and `M` with `Some(Page::Demo)`, the array at **28** entries;
      `no_printable_key_acts_without_a_row_in_the_shortcut_table` and
      `the_gallery_shortcut_list_holds_every_key_the_table_has` still hold.

- [ ] **The `DEMO_APPLICATION.md` amendments hold.** The § *Could not verify*
      carousel row is annotated with the choice, the two sources' count agreement,
      `Navigate`/`Start FSD` as the declined alternative, and that a greyed `Start
      FSD` is a disabled control; the new reshape row and the dated note on §
      *Composite widgets* row 16 carry the same `[C]` split; no row closed or
      renumbered.

- [ ] **A hotspot stays on the thing it names while the car turns, and the reveal
      is one write path.** A second capture after the ambient rotation advances
      shows the four still on the car; a capture before `R` shows none and one
      after shows four.

- [ ] **The seven pages are captured and the frame rate measured.** AE 0 outside
      `y ≥ 680` on the six unchanged pages, 05-2's reveal a before/after pair;
      `.ai/tools/fps-check.sh 10 55` and `ROADOS_RUN_SECONDS=10
      ./target/release/ui_demo --tab=demo`, every page above 55.

- [ ] **Nothing from another task leaked in.** `git diff --stat` shows the three
      new files and changes to `main.rs` and `DEMO_APPLICATION.md`, and no change
      under `ui/src/ui_core/`; `ui/Cargo.toml` and `ui/Cargo.lock` unchanged;
      `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; no `unsafe`, `unwrap`,
      `expect`, `panic!` or `todo!`; `Page::ALL` seven, `Page::DEFAULT`
      `Page::Pads`, no new `--tab=`; `CAR_DISTANCE`, `CAR_TARGET_Y`,
      `CAR_REST_YAW`, `CAR_REST_PITCH` and `AMBIENT_YAW_RATE` unchanged.

- [ ] **The handoff states what is not claimed**: the count and order unverified
      and the three pages a choice; the two-level Media card, Track Mode's default
      page and the strip's dismissibility all `[B]` and declined; half of composite
      row 16's content claim not met; the mechanism the demo's own construction
      justified by three named facts; the continuous axis continuous in type and
      not in input; nothing verified by a tap; the three sub-tasks sequential
      because all three edit `main.rs`.

## Out of Scope

- **No map, so no road markings, no stop lights and no objects.** Gap **#1** stays
  a `TASK_UI_DEMO_n` item per § *What this means for the demo's shape* item 1;
  05-3's tier changes the pane's width and publishes `Reshape::tier()` only.
- **No pinch gesture.** `[C]`, and it needs two pointers this host has none of.
- **No two-level card, no swipe-up, no mode-dependent default page, no G-Meter,
  no dismissible strip.** All `[B]`; the Media card shows a title and a disabled
  source line, there is no G-Meter page among the three, and a strip with no
  specified way back is a defect on this host.
- **No `Navigate` card, no `Start FSD` card, no per-card internal scroll.** A card
  one page wide does not overflow.
- **No unequal page widths and no peek.** Held down by
  `a_peeking_pager_would_put_its_last_page_out_of_range`; the fix is a `Scroll`
  change, not a demo change.
- **No wheel steering and no per-wheel rotation.**
- **No hotspot for the tyres, the doors or the windows.** Four, because §
  *Composite widgets* row 3 names four.
- **No `Segmented`, `Snapshot`, `ThemeScope`, `Margin`, `LayoutMode::Grid`,
  `polygon_is_convex` or `Backdrop`.**
- **No new dependency and no `unsafe`.** `ui/Cargo.toml` and `ui/Cargo.lock` are
  untouched; zero new `unsafe` blocks.
- **Found in the tree and deliberately not fixed.** `TASK_UI_DEMO_03`'s matrix
  composition is described as *"in `Demo`'s own private helper"* without naming
  it; the resolution is a one-line rename to `car_mvp`. `TASK_UI_DEMO_01`'s page
  variant name and `--tab=` spelling remain this task's open input.
