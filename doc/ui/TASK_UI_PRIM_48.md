# TASK_UI_PRIM_48: Margin, `flex-shrink`, and the cross-axis gap — closing gap `L7`

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Close row `L7` of `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* — *"No margin, no `flex-shrink`, no `flex-basis`, no cross-axis gap"* — whose Blocks entry is *"Dense settings rows that must not overflow"*.
Margin and `flex-shrink` are delivered, the cross-axis gap is narrowed to one re-documented `spacing`, and `flex-basis` is declined by argument.

## Context

Row `L7`'s four clauses resolve as **margin added**, **`flex-shrink` added**, **`flex-basis` declined by argument**, **cross-axis gap narrowed by decision**.

- **`Margin`** — a second type beside `Padding`, never an alias (padding subtracted from a box, margin added to a position), a child-side `LayoutState` input read by `LayoutMode::Flex` in full, by `Stack`/`Absolute` for leading edges only, by `Grid` not at all. Negatives are floored at zero on store; **margins do not collapse** (two adjacent 8s give 16), composing in one total order: parent padding outside the mode's match, the child's margin inside the arm, neither reads the other.
- **`flex-shrink`** — `LayoutState::shrink: f32`, default `1.0`, clamped `max(0.0)`, weighted by the child's **content** size `desired.main(direction)` (not `declared`, a range): `scaled = item.shrink * item.base`, ratio before the multiply. A growing child (`flex > 0.0`) has a zero base and is skipped; the declared minimum wins through `clamp_axis`; a shrunk item gets `tight_main`.
- **cross-axis gap** — just `spacing` re-documented as *"the gap between adjacent children"*; **no second parameter** (unreachable while `Flex.wrap` is unhonoured). **`flex-basis` declined** — `Constraints` is the declared basis, `desired` its resolved base, a growing child's basis already zero, so no field is added.

**Overflow has three cases:** anything shrinkable shrinks to its declared minimum; leftover overflow is not squeezed further (the row runs past the box); it is clipped by `Layout::visit`'s existing `children_clip`. Neither the cross axis nor an unbounded main axis shrinks. `Placement` gains a private `trailing: Size`, because a trailing margin on the last child sits in no placement's rect and `content_size` would otherwise measure the container short. **Beside `TASK_UI_PRIM_52`:** `spacing`'s doc clause is added to 52's, never substituted; if `arrange_grid` exists its push gains `trailing: Size::ZERO`; margin stays unread in a grid cell.

## Requirements

1. `pub struct Margin` in `ui/src/ui_core/src/layout.rs` immediately after `impl Padding`: `#[derive(Clone, Copy, Debug, Default, PartialEq)]`; `pub f32` `left, right, top, bottom`; `pub const ZERO`; `pub fn all(sides: f32) -> Self`, `pub fn horizontal(self) -> f32`, `pub fn vertical(self) -> f32` all `#[must_use]`; private `fn clamped(&self) -> Margin`. **No `inset`** (*"A margin has no inset operation — its near edge is added to a position, not subtracted from a box."*), no `From<Padding>`, no `Deref`. Doc: not `Padding`; do not collapse; which modes read it; negatives floored at zero. One doctest.

2. `LayoutState` gains a `margin` field after `padding`: `with_margin(&mut self, margin: Margin) -> Self` `#[must_use]` (stores `margin.clamped()`), `set_margin(&mut self, margin: Margin)` (stores `margin.clamped()`, calls `mark_dirty()`), `margin(&self) -> Margin` `#[must_use]`. `default` gives `Margin::ZERO`.

3. `ui/src/ui_core/src/widgets/container.rs` gains `Container::set_margin(&self, nodes: &mut Arena<WidgetNode>, margin: Margin)`, in `set_padding`'s shape and doc. Nothing else changes except requirement 11's sentence.

4. `LayoutState` gains a `shrink` field after `flex`: `with_shrink(&mut self, shrink: f32) -> Self` `#[must_use]` (`shrink.max(0.0)`), `set_shrink(&mut self, shrink: f32)` (`.max(0.0)`, `mark_dirty()`), `shrink(&self) -> f32` `#[must_use]`. `default` sets `1.0` in `Default for LayoutState`'s body.

5. `struct FlexItem` gains `margin: Margin`, `shrink: f32`, `base: f32`, `tight_main: bool` after `flex`/`desired`, `tight_main` before `tight_cross`. Both constructions in `arrange_flex` gain all four; the stale-handle one is `Margin::ZERO, 0.0, 0.0, false`.

6. `arrange_flex` steps 2–4 are rewritten. Step 1 reads `node.layout().margin()` and `.shrink()`. Step 2 sets `item.base = item.desired.main(direction)`, gives a growing item `item.main = 0.0` and `item.tight_main = true`, and otherwise adds the margins to `item.base`; then, after `distribute`, **`shrink(&mut items, (rigid_total + gaps - available_main).max(0.0), direction)`**. New private helpers, each one caller: `fn main_axis_leading(margin: &Margin, direction: FlexDirection) -> f32` (`left`/`Row`, `top`/`Column`), `fn main_axis_trailing` (`right`/`bottom`), `fn cross_axis_leading`/`fn cross_axis_trailing` (pairs swapped), `fn occupied_main(item: &FlexItem, direction: FlexDirection, gap: f32) -> f32`. Steps 3 and 4 subtract the cross margins inside `clamp_axis`, offset the cross position by the leading margin, and set `cursor += occupied_main(...)`; `main_bounds = if item.tight_main { (item.main, item.main) } else { (0.0, available_main) }`. `tight_main` is set true only for a growing item and for a shrunk one.

7. `fn shrink(items: &mut [FlexItem], overflow: f32, direction: FlexDirection)`, private, immediately after `fn distribute`, a mirror of it. It returns on `overflow <= 0.0` or a zero total weight, with `fixed = vec![false; items.len()]` and `pinned = 0.0f32`; the weight is the sum of `item.shrink * item.base` over `!fixed && flex <= 0.0 && shrink > 0.0`; per item `scaled = item.shrink * item.base`, `share = free * (scaled / total_scaled)` (**ratio before the multiply**), `taken = clamp_axis(item.base - share, min_main, max_main)`, and the margins are added back after the clamp; a `taken != item.base - share` sets `tight_main = true`. A non-finite `scaled` is handled by `clamp_axis`, not a guard.

8. `struct Placement` gains private `trailing: Size` last; `fn bounding_box` reads `right = right.max(corner.x + placement.trailing.width)` and `bottom = bottom.max(corner.y + placement.trailing.height)`. `arrange_stack`'s and `arrange_absolute`'s pushes gain `trailing: Size::ZERO`; if `arrange_grid` exists, its one push gains it too.

9. `arrange_stack` and `arrange_absolute` apply the leading margin only, via one shared helper `fn node_layout_margin(nodes: &Arena<WidgetNode>, handle: Handle) -> Margin` (stale handle → `Margin::ZERO`); `arrange_absolute`'s origin is `position + (margin.left, margin.top)`, its `unwrap_or(Offset::ZERO)` becoming `position.map(|p| Offset::new(p.x + margin.left, p.y + margin.top)).unwrap_or(Offset::new(margin.left, margin.top))`.

10. No `flex_basis` field. Recorded in `LayoutState::with_constraints`'s doc (*"this is the basis a flex container shrinks against"*), `LayoutState::with_flex`'s doc (*"a growing child's basis is zero … there is no separate basis to set"*), and row `L7` (requirement 15).

11. `spacing` stops being documented as main-axis only, in four `layout.rs` sites (`FlexConfig`'s doc, `with_spacing`'s, `spacing`'s, `LayoutState::with_flex_config`'s) and one in `container.rs` (`Container::set_flex_config`'s): each says *"the gap between **adjacent** children. In a one-line `LayoutMode::Flex` container that is the main axis; in a `LayoutMode::Grid` container it is both axes, from this one value."* Task 52's sentence is added to, never substituted. No type, field or getter changes.

12. Doc comments gain who-reads-what: `with_flex`/`with_shrink`/`set_flex`/`set_shrink` say `flex` is honoured by `LayoutMode::Flex` only and `shrink` by `Flex` on the main axis alone; `with_shrink` gains the reachability sentence (*"A child shrinks from its own content size, so a child with no content has nothing to shrink; a child that declares a `tight` size declares a floor equal to its size and does not shrink at all."*) and the overflow consequence; `with_flex` gains the CSS-shorthand warning (*"this is a grow factor … use `with_shrink` to make a child unshrinkable"*); `margin`/`set_margin`/`Margin` name the modes; `flex_config` names `Flex`/`Grid` as readers of `spacing` and `Stack`/`Absolute` as ignorers; `LayoutMode::Flex`'s variant doc gains the margins, the floor, the clipping and the zero basis.

13. `arrange`'s doc gains the composition order in one sentence (margin read inside the mode's arm, padding first then margin, neither reading the other); signature, `match` and the `if padding != Padding::ZERO` fast path are untouched.

14. The twenty-six tests, named, beside the code in `layout.rs`'s `#[cfg(test)] mod tests` — no display, no network, no filesystem, no wall clock, all through `layout_constraints` or `Layout::layout`, none naming a private function, using the existing `leaf`/`container`/`padded`/`layout_in`/`laid_out_by` helpers.

   Margin — ten: `a_leading_margin_pushes_a_child_past_its_siblings_start`, `a_trailing_margin_pushes_the_next_sibling`, `a_trailing_margin_on_the_only_child_is_included_in_the_containers_measured_size`, `two_adjacent_margins_do_not_collapse`, `a_margin_is_clamped_to_zero_when_it_is_stored`, `padding_is_applied_before_margin_and_the_two_add`, `a_margin_offsets_an_absolutely_placed_child_from_the_position_it_declared`, `a_margin_moves_a_stacked_child_off_the_parents_origin`, `a_margin_on_the_cross_axis_centres_a_child_inside_its_own_margins`, `a_margin_is_not_read_by_a_grid_cell` (conditional on `arrange_grid`).

   `flex-shrink` — eleven: `shrink_weights_by_the_declared_size` (220×20 and 110×20 children in a 165-wide box shrink to **110** and **55**), `a_shrink_factor_of_zero_leaves_the_row_overflowing_exactly_as_it_did`, `a_row_whose_children_all_declare_a_tight_size_does_not_shrink`, `a_growing_child_does_not_shrink`, `flex_zero_does_not_mean_immovable`, `an_item_is_never_shrunk_below_its_declared_minimum`, `an_unbounded_main_axis_never_shrinks`, `a_non_finite_shrink_factor_is_handled_by_the_clamp_and_not_by_a_panic`, `shrink_does_nothing_on_the_cross_axis`, `a_shrunk_row_is_shorter_than_it_was_and_never_shorter_than_the_sum_of_the_minima`, `the_default_shrink_factor_is_one`.

   Property — two, in `chart.rs`'s swept shape: `every_flex_child_either_fits_its_parents_box_or_is_clipped_to_it` (`FlexDirection` × child count `1..=6` exhaustively with no seed, geometry from an inline seeded LCG; one rect per child in order and, per child, inside the box to `LAYOUT_EPSILON` or clipped; `const LAYOUT_EPSILON: f32 = 1e-3;` doc-commented with its arithmetic), `shrink_never_oscillates_and_never_produces_a_negative_size` (monotonicity over descending widths `400.0, 300.0, 200.0, 120.0, 60.0, 20.0`, plus non-negativity and finiteness, with `assert!(shrunk_at_least_once)`).

   Gap, basis and integration — three: `spacing_is_the_gap_between_adjacent_children_and_a_stack_ignores_it`, `the_declared_constraints_are_the_flex_basis`, `margin_and_shrink_compose_in_one_pass`.

   Plus three doctests on `Margin`, `LayoutState::with_margin`, `LayoutState::with_shrink`.

15. Row `L7` in `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* is amended, dated, attributed and not closed, per § *Corrections to the second gap table*: one dated note naming `TASK_UI_PRIM_48` giving each clause — margin closed; `flex-shrink` closed; `flex-basis` still absent, deliberately, with the reason; cross-axis gap narrowed with the `Flex.wrap` task named as owner of a second parameter. **Its evidence column is rewritten from line numbers to symbols** (`Margin`, `LayoutState::margin`, `Padding::inset`, `arrange`'s two padding steps, `fn shrink`, `LayoutState::shrink`, `FlexConfig::spacing`, and the two tests that pin who reads it). The row is not deleted or closed, Severity stays **Medium**, Blocks keeps *"Dense settings rows that must not overflow"*, its second sentence (`Padding` does exist) is unchanged, no other row changes, task 52's sentence quoted not restated.

16. The suite is green against the measured baseline of **1894** (1450 `ui_core` + 224 `ui_demo` + 220 doctests): 1450 + 26 = 1476, 224 unchanged, 220 + 3 = 223 doctests; `layout_walk_cost` still the one `#[ignore]`d test and unmodified, `cargo audit` recorded as not installed rather than passed. The 26 is conditional in one place: `a_margin_is_not_read_by_a_grid_cell` exists only if `arrange_grid` does. Verification, capture and frame rate are `.ai/agents/developer.md` § *Phase 3* and `.ai/tools/README.md` § *Capturing a window* / § *Frame-rate baseline*.

## Acceptance Criteria

- [ ] **`Margin` is a separate declaration** from `Padding`, with `impl From<Padding> for Margin|Deref` returning nothing; `set_margin` calls `mark_dirty()`; `LayoutState::default` gives `Margin::ZERO`.

- [ ] **Both property tests pass, are non-vacuous, and fail on the named mutations.** `every_flex_child_either_fits_its_parents_box_or_is_clipped_to_it` uses `LAYOUT_EPSILON = 1e-3` justified in its doc; `shrink_never_oscillates_and_never_produces_a_negative_size` asserts `shrunk_at_least_once`. Failing mutations: `clamp_axis(item.base - share, …)` → `item.base - share`; `item.main = taken` → `item.main -= taken`; `bounding_box`'s `corner.x + placement.trailing.width` → `corner.x`.

- [ ] **Values are pinned by value, not prose** (requirements 4, 6, 7, 14); `fn distribute` is byte-for-byte unchanged and `flex_zero_does_not_mean_immovable` pins CSS's `flex: 0`; overflow has its three tests; `rg -c 'flex_basis'` returns nothing; `spacing` has no main-axis-only scoping and `spacing_is_the_gap_between_adjacent_children_and_a_stack_ignores_it` passes.

- [ ] **The suite is green with every named test present** (the twenty-six, less the conditional grid test if `arrange_grid` is absent, plus three doctests); `cargo fmt --check`, `build --all-targets --all-features`, `clippy … -D warnings`, `doc --no-deps` clean; `layout_walk_cost` unmodified.

- [ ] **The six pages are pixel-identical, the mechanism the criterion.** Release build, captured before and after per `.ai/tools/README.md` § *Capturing a window*; **AE 0 outside `y ≥ 680`**, every differing pixel inside the fps band. Mechanism: `set_margin|with_margin` and `set_shrink|with_shrink` are absent from `ui/src/ui_demo/src/main.rs`; every demo flex child declares `Constraints::tight(..)` so `clamp_axis(base - share, w, w) == w`; the card of pads overflows by 12 (`3×220 + 2×52 = 764` against `752`) but each share `12×(220/660)=4` clamps back to 220; `Margin::ZERO` leaves `arrange_stack`/`arrange_absolute`/`content_size` identical. The rect-level tests (`every_page_places_every_rect_where_the_gallery_placed_it`, `no_two_placed_rects_overlap`, `placed_handles`, `expected_placed_rect_names`, `assert_placed_handles_is_complete`) keep every row.

- [ ] **The frame rate is measured on every page** with `.ai/tools/fps-check.sh 10 55` and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of the six, above 55 and inside the recorded band; the one three-element `vec![false; …]` allocation and the two `f32` adds per placement are named.

- [ ] **Nothing leaked.** No change to `render.rs`, `render/`, `paint.rs`, `batch.rs`, `node.rs`, `arena.rs`, `property.rs`, `animation.rs`, `input.rs`, `theme.rs`, any `widgets/` module but `container.rs`, or `ui/src/ui_demo/src/main.rs`; `ui/Cargo.toml`/`ui/Cargo.lock` unchanged; no new `unsafe`, `unwrap`, `expect`, `panic!`, `unimplemented!`, `todo!` or `as f32` beyond `count_to_f32`; no `pub` item added, no signature changed; Edition 2021 / `rust-version = "1.85"`. `doc/ui/PRIMITIVES_ARCHITECTURE.md` untouched; row `L7` amended, dated, attributed and not closed, evidence column in symbols, Severity Medium, Blocks kept, no other row changed.

## Out of Scope

- **No `LayoutMode::Grid` change and no grid algorithm** — `TASK_UI_PRIM_52`'s; `arrange_grid`, `grid_column_width`, `grid_row_heights`, `LayoutMode::Grid`'s doc and 52's twenty-one tests are untouched, the only interaction being `Placement`'s private field.
- **No absolute or `auto` margins** (negatives floored at zero on store; an auto main margin would be a fourth free-space consumer and an auto cross margin duplicates `CrossAxisAlignment`), and **no percentage resolution** for a margin or a basis.
- **No nested-margin collapsing** (it would make a node's measured size a function of its siblings); adjacent margins add.
- **No `flex_basis` field, no `flex-basis: auto`, no `min-width` basis, no second length type.**
- **No second gap parameter** (`row_gap`, `column_gap`, `cross_axis_gap`, `GridConfig`) and **no `Flex.wrap`**.
- **No change to `LayoutMode`, `Constraints`, `Rect`, `Size`, `Offset` or `Handle`**; `FlexConfig` and `Padding` change in comments only; `fn distribute`, `fn alignment_spacing`, `fn intersect`, `fn clamp_axis`, `fn count_to_f32`, `fn direction_cross_to_xy`, `fn cross_offset`, `fn resolve_box` and `fn sized` are byte-for-byte unchanged.
- **No scroll integration and no `List` change** (row `L5` is open and not this task's), and **no `Container` change beyond `set_margin`.**
- **No new dependency and no `unsafe`** — the approved direct dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`.
- **One confusing comment found and deliberately not fixed** — `layout.rs`'s padding-test comment *"Padding is the margin …"*; recorded, not edited, per `developer.md` § *Phase 2*.
- **No new page, no `--tab=` name, and no change to any of the six pages.**
