# TASK_UI_PRIM_46: Gap `L5` — An Axis on `Scroll`, Momentum, and Snap Points

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Build the mechanism half of row **`L5`** in `doc/ui/DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* — an axis on `Scroll`, momentum, and snap
points — and deliver **none of the three products** its `Blocks` column names: the
card carousel, dock overflow, and the widened wiper segmented control, which are
`TASK_UI_DEMO_n` items. `ui_core` only: the demo constructs no `Scroll` and no
`List`, so the six gallery pages must come out pixel-identical and the frame rate
must not move.

## Context

**The three mechanism halves.** `scroll_offset` becomes `Property<Offset>` and the
axis a private field with a getter and a setter; `Axis` is a new type — not
`slider::Orientation`, which has no `Both` and is the slider's own — because a scalar
plus an axis field cannot express the card carousel's two live offsets. **Momentum**
estimates velocity from the drag stream in `tick` and decays it exponentially with
the closed-form integral to a fixed cutoff, no overshoot, no bounce, hand-rolled
rather than routed through an `Animation`. **Snap points** are a sorted,
de-duplicated `Vec<f32>` of offsets on the scroll's axis, range-clamped where the
viewport is known, settled by a second `AnimationClock`; `List` stays
`Axis::Vertical`, its item boundaries installed by `List::sync`, and
`Scroll::snap_to_state` is not renamed and stays palette snapping.

**Gap row closed: `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in
`ui_core`*, row `L5`.** Severity **High**, not deleted and not marked closed.

**Facts not available from the source:** `InputEvent` carries no timestamp, so the
only duration is the frame delta `tick` is handed; `layout::Offset` carries no
arithmetic; `Scroll::snap_to_state` is a crate-wide convention
(`grep -l 'pub fn snap_to_state' ui/src/ui_core/src/widgets/` returns ten files,
`ui_demo` calls it eleven times); the demo builds no `Scroll` and no `List`
(`grep -n 'widgets::scroll\|widgets::list' ui/src/ui_demo/src/main.rs` returns
nothing); the two refusals stay (`Scroll::on_event`'s `delta.y == 0.0` early return,
`scroll_key`'s refusal of left/right). **Baseline `cargo test --all-features` from
`ui/`: 1894** (1450 + 224 + 220), this file projecting **1935**.

**Files:** `ui/src/ui_core/src/widgets/scroll.rs`, `ui/src/ui_core/src/animation.rs`,
`ui/src/ui_core/src/widgets/list.rs`, `doc/ui/DEMO_APPLICATION.md`. 46.1 runs before
46.2, which reads `Axis`, `Scroll::set_snap_points` and `Scroll::tick`'s signature.

## Requirements

### Sub-task 46.1 — the mechanism

1. **`pub scroll_offset: Property<Offset>`** replaces `Property<f32>`; `Scroll::new`
   writes `Property::new(Offset::ZERO)`. Doc rewritten: `rect.y - scroll_offset` →
   `rect.origin - scroll_offset`; *"never animated"* → only a snap animates.
2. **`pub enum Axis { Vertical, Horizontal, Both }`** in `scroll.rs`, deriving
   `Clone, Copy, Debug, Default, PartialEq, Eq`, `Vertical` `#[default]`; doc: a
   **caller's claim**; doctest drives a horizontal wheel and drag through
   `Scroll::on_event`.
3. **Private `axis`**; `pub fn axis(&self) -> Axis` `#[must_use]`; `pub fn
   set_axis(&mut self, axis: Axis) -> Axis` writes `content_width` when the axis
   admits `x`, else `0.0`.
4. **Private `content_width: f32`**; `pub fn content_width(&self) -> f32`; `pub fn
   set_content_width(&mut self, width: f32) -> f32`; `sync_content` reads
   `content_size(nodes)` once and writes both.
5. **Five free functions, names kept:** `max_scroll`, `clamp_scroll`, `visible_rect`
   (**no `min` between the axes**), `gesture_delta(delta, axis)` (three arms),
   `wheel_delta` (negated per component; a zero component contributes `0.0`);
   doctests add `NaN`, horizontal, `Both`, zero-component.
6. **Geometry stays scalar:** `thumb_length`, `thumb_run`, `track_rect`,
   `track_rect_h(&self, rect: Rect) -> Rect` = `Rect::new(rect.x, rect.y +
   rect.height - SCROLLBAR_MARGIN - self.thickness, rect.width, self.thickness)`,
   `thumb_rect`/`thumb_rect_h`, `offset_under`/`offset_under_h`;
   `SCROLLBAR_MARGIN`/`SCROLLBAR_MIN_THUMB` doc notes.
7. **`Scroll::scroll_by(&self, rect: Rect, delta: Offset) -> Offset`** `#[must_use]`;
   **`Offset::ZERO` is the re-clamp**. `Scroll::max_scroll_for(&self, rect: Rect) ->
   Offset` `#[must_use]`.
8. **`Scroll::apply_offset`**: `let offset = self.scroll_offset.get(); let target =
   Offset::new(-offset.x, -offset.y);`.
9. **`Scroll::content_rect`** shifts both components; doctest gains a horizontal
   assertion.
10. **Three private `Cell`s**: `drag_pending: Cell<Offset>`, `velocity: Cell<Offset>`
    (px/s; zero is not coasting), `snap_pending: Cell<bool>`.
11. **`snap_points: Vec<f32>`** (private): `snap_points(&self) -> &[f32]`,
    `set_snap_points(&mut self, points: &[f32]) -> usize` (drops non-finite, sorts,
    de-dupes, returns count kept, no clamp), `has_snap_points(&self) -> bool`, all
    `#[must_use]`, and private `nearest_point(&self, offset: f32, rect: Rect) ->
    Option<f32>` (nearest in `0.0..=max_scroll` to the already-clamped offset, tie
    earlier, `None` if empty or all out). Under `Axis::Both`, the larger-magnitude
    velocity axis, tie to `y`.
12. **`Scroll::snap_to_points(&self, rect: Rect, motion: Motion) -> bool`**
    `#[must_use]`: clamps, asks `nearest_point`; **`None` → `false`, nothing
    written**; else clears the offset clock, zeroes velocity, adds
    `self.scroll_offset.animate_to(target, motion.duration, motion.easing)`, `true`;
    clears `velocity`/`drag_pending`/`snap_pending` on entry. Automatic settle uses
    private `SNAP_MOTION`; doc recommends `Easing::EaseOut`.
13. **`offset_clock: RefCell<AnimationClock>`** = `AnimationClock::new()` in
    `Scroll::new`.
14. **`Scroll::tick(&self, rect: Rect, delta: Duration) -> bool`** `#[must_use]`:
    (1) this frame's drag becomes a velocity, replacing armed momentum; (2) else one
    exact-decay step per component, `factor = (-MOMENTUM_DECAY_PER_SECOND *
    seconds).exp()`, `travel = Offset::new(v.x / MOMENTUM_DECAY_PER_SECOND * (1.0 -
    factor), v.y / MOMENTUM_DECAY_PER_SECOND * (1.0 - factor))`, clamp, write, then
    `v *= factor`, a pinned component zeroed and `snap_pending` set; (3) `snap_pending`
    arms `snap_to_points(rect, SNAP_MOTION)`, flag cleared that frame; (4) offset clock
    ticks; (5) appearance clock ticks; `true` if 1/2/4/5 wrote.
15. **`Scroll::is_coasting(&self) -> bool`, `Scroll::momentum_velocity(&self) ->
    Offset`**, both `#[must_use]`; `is_animating` doc widened, signature unchanged.
16. **`Scroll::on_event`, axis-aware.** `Drag` — grabbed-thumb branch first on its own
    axis (`offset_under`/`offset_under_h`, both for `Both`), then `gesture_delta(delta,
    self.axis)`; **`Offset::ZERO` returns `false` unconsumed**; the applied delta goes
    into `drag_pending`; zeroes `velocity`. `Scroll` — `wheel_delta`; `Offset::ZERO`
    returns `false` unconsumed; a hit consumes, clears `drag_pending`/`velocity`.
    `KeyDown` — `if !self.focused.get() { return false; }`, then `scroll_key(&key,
    self.axis)`; `None` returns `false` unconsumed; a hit calls `scroll_by`, clears
    both. `Tap`, `LongPress`, `Swipe`, `Pinch`, `KeyUp`, `Text` reach `_ => false`.
17. **`fn scroll_key(key: &Key, axis: Axis) -> Option<Offset>`**, private: four arrows
    and four D-pad directions mapped to the component of their axis, `None` on
    `Vertical` for left/right.
18. **`Scroll::paint`** draws the vertical bar, then the horizontal bar, then the focus
    rings and thumbs, returns `Vec::new()` only when neither axis has anywhere to go;
    `scrollbar_rect`'s doctest gains both axes. No `DrawCommand` variant.
19. **`impl Interpolate for Offset`** in `animation.rs`, four lines through
    `f32::interpolate`; doctest: midpoint of `Offset::new(0.0, 0.0)` and
    `Offset::new(100.0, 200.0)` is `Offset::new(50.0, 100.0)`.
20. **`scroll.rs`'s module doc is rewritten**: axis model; momentum model with its
    three constants and the exact integral; why `Easing` is the snap's; snap model;
    paging limit and unbuilt spacer; the two refusals. **"Horizontal scrolling"** and
    **"Momentum."** deleted from *What is deliberately not here*.
21. **Correct `scroll.rs`'s stale clipping claims** (`Scroll::clip_rect`,
    `clip_commands`); if task 45 has landed, say so and move on.
22. In `list.rs`: `pub fn tick`, `snap_to_items(&self) -> bool` and
    `set_snap_to_items(&mut self, snap: bool) -> bool` (`#[must_use]`, setter in
    `List::set_palette`'s shape), private `snap_to_items: bool` **false** in
    `List::new`. `List::sync` writes the snap points every call, after its re-clamp
    and before `visible_range` (`i · item_height` clamped into `0..=max_scroll`),
    **only when `snap_to_items` is true, clearing the set when it is false.**
    `List::offset`, `max_scroll_for`, `visible_range`, `item_rect`, `item_at`,
    `clip_rect`, `sync`'s signature and all four free functions keep `f32`
    signatures; bodies change to `.y` off the clamped `Offset` and
    `scroll::max_scroll(rect.size(), …).y`. `list.rs`'s module doc gains the snap
    points as its **seventh** borrowed thing and swaps **"Momentum."** for **"A
    horizontal axis, and a variable item height"**.

### Sub-task 46.2 tests, and the document

23. **The tests, named** — no display, network, filesystem, wall clock or pointer
    event; driven by an injected `InputEvent` into `Scroll::on_event`, by
    `Scroll::tick` stepped with a test-chosen `Duration`, and by `Arena::new()`.

    `scroll.rs` **Axis**: `a_horizontal_scroll_scrolls_on_x_and_leaves_y_bit_identical`,
    `a_two_axis_scroll_moves_both_components_of_one_drag`,
    `a_horizontal_scroll_refuses_a_vertical_drag_and_leaves_it_unconsumed`,
    `a_horizontal_wheel_moves_a_horizontal_scroll_the_way_a_vertical_wheel_moves_a_vertical_one`,
    `a_horizontal_scroll_clamps_its_x_at_both_ends`,
    `a_horizontal_scrollbar_is_drawn_along_the_bottom_edge_and_a_vertical_one_is_not`,
    `apply_offset_writes_the_negated_vector_and_a_horizontal_offset_moves_the_content_sideways`,
    `a_content_rect_shifts_on_both_axes`.

    **Momentum**: **`a_momentum_travels_the_same_distance_at_ten_milliseconds_a_frame_and_at_forty`**,
    `a_momentum_that_crosses_the_cutoff_stops_at_the_same_offset_on_both_frame_rates`,
    `the_velocity_is_the_drag_this_frame_carried_over_the_frame`,
    `the_momentum_travels_the_closed_form_and_stops_within_a_pixel_of_it`,
    `a_momentum_into_the_bound_is_pinned_there_and_ends`,
    `a_wheel_notch_and_a_key_press_cancel_a_momentum_and_start_none`,
    `a_drag_on_a_grabbed_thumb_arms_the_same_momentum_a_drag_on_the_content_does`,
    `a_zero_tick_neither_arms_a_velocity_nor_moves_the_content`,
    `the_velocity_is_capped_at_the_maximum_and_the_cap_is_not_reachable_from_a_real_drag`,
    `tick_reports_true_while_the_content_coasts_and_false_the_frame_after_it_stops`.

    **Snap**: **`a_snap_settles_on_the_nearest_point_when_the_momentum_stops`**,
    `a_snap_lands_exactly_on_its_point_after_its_duration`,
    `set_snap_points_keeps_the_sorted_distinct_finite_ones_and_drops_the_rest`,
    `a_scroll_with_no_snap_points_stops_where_its_momentum_stopped`,
    `the_nearest_point_ties_go_to_the_earlier_one`,
    `a_snap_point_past_the_end_of_the_content_is_not_offered`,
    `an_ease_out_snap_never_leaves_the_range_and_a_spring_one_does`,
    `a_snap_and_a_palette_transition_can_run_at_once_because_they_have_two_clocks`,
    `a_snap_to_points_can_be_asked_for_directly_with_no_momentum_at_all`.

    **Keys**: `left_and_right_scroll_a_horizontal_scroll_and_are_refused_by_a_vertical_one`,
    `a_two_axis_scroll_takes_all_four_arrows_each_on_its_own_axis`,
    `an_unfocused_scroll_refuses_a_horizontal_key_as_it_does_a_vertical_one`.

    **Event surface**: `the_scroll_consumes_only_a_drag_a_wheel_and_a_focused_key` —
    all nine `InputEventKind` variants on an `Axis::Both` scroll; only `Drag`,
    `Scroll` and a **focused** `KeyDown` consumed.

    **Constants**: `the_momentum_constants_and_the_snap_motion_are_finite_and_positive`.

    **Preserved, asserted by a diff**: `a_drag_that_is_entirely_horizontal_is_left_for_a_horizontal_scroller`,
    `a_horizontal_wheel_is_not_a_vertical_scroll`,
    `a_wheels_size_is_not_used_only_its_direction`,
    `a_wheel_notch_and_a_drag_of_the_same_size_move_a_scroll_alike`,
    `a_drag_is_clamped_at_both_ends`,
    `a_drag_moves_the_offset_the_content_rect_and_the_node_together`,
    `visible_rect_is_the_band_of_content_the_viewport_shows`,
    `down_is_later_and_a_wheel_agrees_with_a_finger` — every name and assertion kept,
    `.get()` gaining `.y`, `set(60.0)` gaining `Offset::new(0.0, 60.0)`.

    `list.rs`: `a_list_with_snap_to_items_settles_on_a_row_boundary` (100 rows of 40
    in a 200 × 300 viewport; lands on a **multiple of 40** inside `0..=3_680`,
    exactly), `a_list_without_snap_to_items_stops_where_its_momentum_stopped`,
    `a_list_ticks_the_scroll_it_embeds_and_touches_nothing_of_its_own`,
    `turning_item_snapping_off_clears_the_points_rather_than_leaving_them`,
    `a_list_is_vertical_and_still_refuses_a_horizontal_wheel`.

    **Doctests:** `impl Interpolate for Offset`, `Axis`, `Scroll::tick`,
    `Scroll::snap_to_points`. **Count:** thirty-two in `scroll.rs` (8 axis, 10
    momentum, 9 snap, 3 keys, 1 event table, 1 constants) + 5 in `list.rs` = 37, plus
    4 doctests; 1894 + 41 = **1935**.
24. `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*, row `L5`,
    gains a dated note naming `TASK_UI_PRIM_46`: the offset is a `Property<Offset>`,
    the axis is `scroll::Axis`, the momentum is a closed-form exponential with its
    three constants, the snap points are `Vec<f32>`, **`snap_to_state` is still
    palette snapping and was not renamed**, **`List` is still vertical** so the row's
    *"and `List`"* half is unchanged, and **all three `Blocks` entries remain and none
    is delivered**. **The row is not deleted and not marked closed.** Its four
    line-number citations become symbols — `Scroll::scroll_offset`,
    `Scroll::on_event`, `max_scroll`, `visible_rect`, `List::item_rect`'s doc, the
    module's *What is deliberately not here*.

## Acceptance Criteria

- [ ] `awk '/^pub struct Scroll/,/^}/' ui/src/ui_core/src/widgets/scroll.rs` shows
      `pub scroll_offset: Property<Offset>`; `pub enum Axis { Vertical, Horizontal,
      Both }` is in `scroll.rs` with `Vertical` `#[default]`; the handoff lists the
      nine changed and fourteen added public items by name; `main.rs` is unchanged.
- [ ] `grep -n 'Orientation' ui/src/ui_core/src/widgets/scroll.rs` and
      `grep -n 'Both' ui/src/ui_core/src/widgets/slider.rs` both return **nothing**;
      the four refusal tests keep their names and every assertion.
- [ ] `a_momentum_travels_the_same_distance_at_ten_milliseconds_a_frame_and_at_forty`
      asserts **64 × 10 ms against 16 × 40 ms agree within 0.01 px**; **mutation
      evidence:** `travel = v * dt` in `Scroll::tick` fails it by about **28 px**.
      `a_momentum_that_crosses_the_cutoff_stops_at_the_same_offset_on_both_frame_rates`
      asserts **1.1 px** (`MOMENTUM_STOP_SPEED / MOMENTUM_DECAY_PER_SECOND` = 1.0 px
      plus a margin).
- [ ] `a_snap_settles_on_the_nearest_point_when_the_momentum_stops` settles on
      **100** from a landing near **70**, where a `floor`-based implementation picks
      **0**. `an_ease_out_snap_never_leaves_the_range_and_a_spring_one_does` **must**
      find the offset out of range at some frame.
- [ ] `Scroll::snap_to_state` is byte-identical; `grep -n 'pub fn snap_to_state\|pub
      fn snap_to_points\|pub fn set_snap_points' ui/src/ui_core/src/widgets/scroll.rs`
      shows **three** distinct names; `git diff --stat` over `button.rs`, `chart.rs`,
      `gauge.rs`, `image.rs`, `keyboard.rs`, `progress.rs`, `slider.rs`,
      `text_input.rs`, `toggle.rs` and `main.rs` shows no change;
      `rg -c 'snap_to_state\(\);' ui/src/ui_demo/src/main.rs` still returns **11**.
- [ ] `git diff ui/src/ui_core/src/widgets/list.rs` shows `pub fn tick`, `pub fn
      snap_to_items`, `pub fn set_snap_to_items` as the only new `pub fn`;
      `turning_item_snapping_off_clears_the_points_rather_than_leaving_them` and
      `a_list_is_vertical_and_still_refuses_a_horizontal_wheel` pass.
- [ ] `grep -n 'std::time::Instant\|SystemTime\|thread::sleep'
      ui/src/ui_core/src/widgets/scroll.rs ui/src/ui_core/src/widgets/list.rs` returns
      **nothing**; no criterion here is met by a pointer-driven capture.
- [ ] `cargo test --all-features` is green with every named test present; **baseline
      1894 (1450 + 224 + 220), projected 1935**; no test deleted, renamed away or
      weakened. Verification is `.ai/agents/developer.md` § *Phase 3*; specific to
      this task, the six gallery pages are the only surface to capture.
- [ ] Captures before and after per `.ai/tools/README.md` § *Capturing a window* give
      **`AE 0` over the `1280x680` crop on all six**, every differing pixel inside the
      fps readout's band; `grep -n 'widgets::scroll\|widgets::list\|Scroll::new\|List::new'
      ui/src/ui_demo/src/main.rs` returns nothing before and after.
- [ ] `.ai/tools/fps-check.sh 10 55` on the default page and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` per page — every
      page above the floor of **55**, each against its own pre-change number; a page
      that moves is a finding, reported with the run's page named.
- [ ] Row `L5` carries the note of requirement 24, not deleted and not marked closed,
      its `Blocks` column still naming **all three** with **none delivered** in those
      words; `grep -n 'scroll.rs:378' doc/ui/DEMO_APPLICATION.md` returns **nothing**;
      `git diff --stat` shows exactly the four files of § *Context*; `ui/Cargo.toml`,
      `ui/Cargo.lock`, `LayoutState::clip`, `LayoutState::hits` and `Batch::clip`
      unchanged; `grep -c unsafe` over the diff is **0**.
- [ ] Both suites were reported against **a `/tmp` copy with the other agent's file
      and its `mod` line removed** — a single green number with no tree named does not
      meet this criterion.

## Out of Scope

- No rubber-banding, no elastic overscroll, no bounce.
- No pull-to-refresh, no sticky headers, no overscroll affordance.
- No nested scrolling and no scroll chaining.
- No grid, no `LayoutMode::Grid`, no `Flex.wrap` — row `L3`, owned by `TASK_UI_PRIM_52`.
- No card carousel, no dock overflow, no widened wiper segmented control; the carousel
  also owns the unbuilt paged trailing spacer.
- No snap outside the content's extent, and no trailing spacer.
- No snap-point callback: `Callback<T>` is `Rc<dyn Fn(T)>` returning `()`.
- No caller-facing momentum tuning: the three constants stay private, no `set_momentum`.
- No vertical `Scroll` behaviour change, no `List` API break, no `Axis::Both` snap on
  two lattices.
- No change to `Scroll::snap_to_state`, `Palette`, `Style`, `animate_to_state`,
  `style`, `grab_thumb`, `release_thumb`, `is_thumb_grabbed`, `clip_rect`,
  `command_bounds`, `clip_commands`, `LayoutState::clip`, `LayoutState::hits`,
  `LayoutState::visible`, `Batch::clip`, `BatchKey`, `Renderer::draw_node_clipped` or
  any GL call — tasks 45 and 42 own those.
- No demo change of any kind, no new dependency, no `unsafe`.
