# TASK_UI_PRIM_52: `LayoutMode::Grid` — Equal Columns, Filled Row-Major, Rows as Tall as Their Tallest Cell

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Give `LayoutMode::Grid` the algorithm it does not have: read `columns`, split the
given box into that many **equal** columns with **one** gap, fill children
**row-major**, size **each row to its tallest cell** and give **every cell in that
row that height**, then report one rect per child where today it reports none. No
new API is needed — `Container::new(nodes, mode: LayoutMode)` and
`Container::set_mode(nodes, mode)` already take a `LayoutMode`, so
`Container::new(nodes, LayoutMode::Grid { columns: 4 })` is reachable once the arm
exists. The demo's tile grids are a `TASK_UI_DEMO_n`, not this task.

## Context

**The gap is recorded twice, and both rows are one gap.** Row **`L3`** of
`DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* (`columns`
declared and never read; `Flex.wrap` accepted and discarded by the `..`) and row
**`#2`** of § *Library gaps* (`Grid` lays out no children and reports no rects)
carry the same clause with the same evidence — the second table calls the
`L1..L11` rows the ones the Layout work exposed. **Both must be amended and
neither deleted or marked closed.** **This task closes only the `Grid`
clause**: `arrange`'s flex arm stays `LayoutMode::Flex { direction, .. }`, so
`wrap` is still discarded and `L3`'s second clause stays literally true.

**The algorithm.** `columns` is a **column count, not a measurement**: split the
box into `columns` equal columns with `config.spacing()` as the one gap; child `i`
goes to row `i / columns`, column `i % columns` (row-major); each cell is exactly
`column_width × row_heights[r]`, where `row_heights[r]` is the tallest desired
height in row `r` and **every cell in that row gets that height**. Nothing in a
cell's own data decides its size. `columns == 0` lays the children out as **one
column**, not as nothing. A short last row is **left-aligned** and occupies only
the columns its children fill; no cell is emitted for an empty column. A row
**overflows** a too-short box rather than being squeezed. **Neither alignment is
honoured**, and a cell's `flex` factor is **not read**.

**Facts the source does not give.**

- `arrange_grid` hands each cell `Constraints::tight(slot)`; `resolve_box`'s
  existing `tighten`/`clamp_axis` then apply. **A cell declaring a minimum wider
  than its column or taller than its row is larger than its slot and overlaps its
  neighbour** — the grid's one honest limit, in `LayoutMode::Grid`'s doc and a test.
- `config.spacing()` is the gap on **both** axes, one value, **not** a second
  `row-gap`/`column-gap` parameter; `with_spacing` already clamps a negative to zero.
- `count_to_f32` is the file's one `usize`→`f32` cast and every conversion in
  `arrange_grid` goes through it; `clamp_axis` is the crate's one clamp, its rule
  **minimum wins** with a `NaN` passed through. `content_size` measures under
  `Constraints::UNBOUNDED`, so an unconstrained grid must size to its **widest
  row**, not collapse. Both padding steps stay outside `arrange`'s match, so a grid
  gets padding for free; a stale handle is `Size::ZERO` and still occupies its cell.
- `columns` stays `usize`, so `PRIMITIVES_ARCHITECTURE.md` § *Widget Tree*'s
  `Grid { columns: usize }` block stays correct.

Verification is `.ai/agents/developer.md` § *Phase 3* and is not restated here; the
baseline is **1894** (1450 `ui_core` + 224 `ui_demo` + 220 doctests), and
`LayoutMode::Grid { columns: 0 }` is publicly constructible today.

## Requirements

1. **`arrange`'s grid arm calls a new `arrange_grid` with this exact signature:**

   ```rust
   fn arrange_grid(
       nodes: &Arena<WidgetNode>,
       children: &[Handle],
       constraints: Constraints,
       columns: usize,
       config: &FlexConfig,
   ) -> Vec<Placement>
   ```

   and the arm in `ui/src/ui_core/src/layout.rs` becomes
   `LayoutMode::Grid { columns } => arrange_grid(nodes, children, inner, columns, config),`.
   **`inner` is `padding.inset(constraints)`, the same value the other three arms
   are given**, because the two padding steps stay outside the match.

2. **The algorithm, inside `arrange_grid`, in this order:** `if children.is_empty()
   { return Vec::new(); }` (makes `rows >= 1` and a `row_heights[0]` index safe);
   `columns.max(1)`; `gap = config.spacing()` and `column_gaps = count_to_f32(columns
   - 1)`; one `desired` per child via `nodes.get(handle).is_none_or(|_| sized(nodes,
   handle, loose))` under `constraints.loosen()` (a stale handle is `Size::ZERO` and
   still occupies its cell, keeping the result index-aligned with `children`);
   `rows = children.len().div_ceil(columns)`; available width = finite
   `constraints.max_width`, else `max over rows of (Σ desired widths + gap * (count -
   1))`; `column_width = grid_column_width(available, columns, gap)` and
   `row_heights = grid_row_heights(&desired, columns)`. Then, for `row` in `0..rows`
   and `index` in `first..last` (`first = row * columns`, `last = (first +
   columns).min(children.len())`):

   ```rust
   let x = count_to_f32(column) * (column_width + gap);
   let y = <running sum of this row's and every earlier row's height, plus one gap each>;
   let size = Size::new(column_width, row_heights[row]);
   ```

   **`x` is multiplicative, not an accumulated cursor**; origins are in the box's own
   coordinates. Push `Placement { handle: children[index], rect: Rect::new(
   Offset::new(x, y), size), constraints: Constraints::tight(size) }` — **`tight` of
   the cell's own size is what makes the child's rect its slot**; no new sizing rule
   is introduced.

3. **Two private helpers, one caller each, doc-commented, not reachable from a
   test:**

   ```rust
   /// The width each of `columns` equal columns gets in a box `available` wide,
   /// with `gap` between them.
   fn grid_column_width(available: f32, columns: usize, gap: f32) -> f32

   /// The height of each row of `columns`: the tallest desired height in it.
   fn grid_row_heights(desired: &[Size], columns: usize) -> Vec<f32>
   ```

   `grid_column_width`'s body is `(available - gap * count_to_f32(columns - 1))
   .max(0.0) / count_to_f32(columns)`; `grid_row_heights` allocates with
   `Vec::with_capacity(rows)`, one entry per row that has a child.

4. **`columns == 0` lays the children out as one column**, via `columns.max(1)` at
   the top of `arrange_grid`. **`LayoutMode::Grid { columns: 0 }` is publicly
   constructible today**, so the degenerate case cannot be an internal assertion;
   the clamp also keeps `usize::div_ceil` from panicking.

5. **The `spacing` docs change in `layout.rs` and `container.rs`.** `FlexConfig`'s
   doc, `with_spacing`'s doc, `spacing`'s doc and the field's own doc each gain the
   sentence that **`LayoutMode::Grid` uses `config.spacing()` as its gap on both
   axes, from one value, and that this is not a second gap parameter.**
   `Container::set_flex_config`'s doc, which says *"how a **flex mode** arranges
   this container's children"*, gains the same sentence. **No type changes, no new
   field, no new getter; `MainAxisAlignment` and `CrossAxisAlignment` untouched.**

6. **`LayoutState`'s `flex` docs name the modes that honour it.** `LayoutState::flex`'s
   doc, `with_flex`'s and `set_flex`'s each gain: **`LayoutMode::Flex` honours the
   factor and no other mode does; a grid cell's is ignored**, because a grid has no
   main axis and its column width is decided by `columns` and the box. **Nothing
   else in `LayoutState` changes**, and `position` is not touched.

7. **`LayoutMode::Grid`'s doc is rewritten and carries a `# Examples` doctest.**
   The out-of-scope sentence is deleted, not qualified. The replacement states, in
   order: **`columns` equal columns filled row-major; a row as tall as its tallest
   cell, every cell in that row that tall; `config.spacing()` as the gap between
   columns and between rows; `columns: 0` as one column; a short last row
   left-aligned filling only the columns its children occupy; rows overflowing
   rather than squeezed; the alignments not honoured; a cell's flex factor not
   read.** The example builds a two-column grid of four leaves in an `Arena`
   through `node::create` and `Layout::new(...).layout(...)`, the shape the module's
   own doc example and `LayoutMode::row`'s doctest use, and asserts the four rects
   by value.

8. **`LayoutMode::Flex`'s `wrap` doc is corrected.** *"Wrapping arrives with the
   list and scroll widgets."* is **deleted** and replaced by the statement that
   **wrapping is not honoured and `LayoutMode::Grid` is where a node places children
   on more than one line**, because its row-major flow is wrapping with a column
   count in place of a break rule. The field's own doc — *"Whether children should
   wrap onto further lines. Not yet honoured."* — **stays, because it is true.**
   `arrange`'s flex arm stays `LayoutMode::Flex { direction, .. }`,
   `LayoutMode::row()`/`::column()` keep writing `false`, and **no wrapping is
   implemented in this task.**

9. **The tests, named, in `layout.rs`'s `#[cfg(test)] mod tests`, with no display,
   no network, no filesystem and no wall clock.** Every assertion goes through
   `layout_constraints` or `Layout::layout`; **no test names a private function**;
   the existing helpers `leaf`, `container`, `padded` and `layout_in` are used as
   they are.

   - **Property test.** `every_grid_cell_is_inside_the_grid_and_no_two_cells_overlap`
     sweeps `columns` in `1..=8` and a count in `0..=columns * 3 + 2` (**216 pairs,
     exhaustively, no seed**), sizes from a seeded inline LCG in `chart.rs`'s
     `every_segment_quad_is_convex_over_two_hundred_thousand_geometries` shape,
     asserts one rect per child in order, containment to `GRID_EPSILON`, and no
     interiors intersecting (touching excluded). `const GRID_EPSILON: f32 = 1e-3;`
     is doc-commented with the ulp arithmetic that forces it.
   - `grid_places_children_instead_of_reporting_nothing` — the replacement for
     `grid_has_no_algorithm_yet`: same `LayoutMode::Grid { columns: 2 }` and
     `layout_constraints` call, `assert!(rects.is_empty())` inverted to a
     **50 × 100 rect at the origin**.
   - **Flow:** `grid_places_children_row_major_into_equal_columns`;
     `zero_columns_lays_the_children_out_as_one_column`;
     `one_child_gets_one_column_and_the_rest_of_the_row_is_nothing`;
     `an_empty_grid_produces_no_placements`.
   - **Row heights:** `a_grid_row_is_as_tall_as_its_tallest_cell_and_every_cell_shares_it`;
     `rows_do_not_share_a_height_with_each_other`;
     `a_cell_whose_declared_minimum_exceeds_the_column_overflows_its_slot` (a
     `Constraints::new(400.0, f32::INFINITY, 0.0, f32::INFINITY)` cell is 400 wide
     in a 100-wide column).
   - **Spacing:** `a_grid_uses_the_configured_spacing_between_columns_and_between_rows`
     (`with_spacing(8.0)`, `column_width = (100 - 8) / 2 = 46`, second column at
     `54`); `zero_spacing_makes_a_grid_abut_without_overlapping` (`with_spacing(-4.0)`).
   - **Ignored config:** `a_grid_ignores_both_alignments_and_stacks_rows_from_the_top`;
     `a_grid_cell_ignores_its_flex_factor`;
     `wrap_is_not_honoured_and_a_wrapped_row_stays_on_one_line`.
   - **Measurement and padding:** `a_grid_measures_to_the_widest_row_when_the_box_is_unbounded`;
     `a_grid_too_narrow_for_its_columns_gives_zero_width_cells_and_overflows`;
     `a_padded_grid_offsets_every_cell_by_the_leading_edges`;
     `a_padded_grid_is_measured_as_its_content_plus_the_padding`.
   - **Structure:** `a_grid_inside_a_grid_cell_lays_out_in_that_cell`;
     `a_stale_grid_child_keeps_the_result_index_aligned`;
     `an_overflowing_grid_row_keeps_its_height_and_is_clipped`.

10. **The gap rows are amended, dated, attributed, and neither is closed.**
    `DEMO_APPLICATION.md` carries three edits:

    - **Row `L3`** in § *Gaps this layout exposes in `ui_core`* gains a dated note
      naming `TASK_UI_PRIM_52`, stating that the **`Grid` clause is closed** (the
      variant's algorithm, `columns` read in production, one rect per child) **and
      that the `Flex.wrap` clause is not**, because `arrange`'s flex arm still
      discards `wrap`. **Its evidence column is rewritten from line numbers to
      symbols** — the arm, the field, the variant, the test. **The row is not
      deleted and not marked closed**, and its Blocks column keeps *"The Controls
      tile grid, the app tray grid"*.
    - **Row `#2`** in § *Library gaps* gains the same dated note in the prose its
      row uses, and **keeps its numbering**, because four files cite rows 1–8 by
      number.
    - **Row `L7`** gains one sentence: **the cross-axis-gap clause holds for
      `LayoutMode::Flex`, and `LayoutMode::Grid` applies the one `spacing` value to
      both of its axes, which is not a second gap parameter** — amended and still
      open; margin / `flex-shrink` / `flex-basis` untouched.
    - **No other row changes, and `doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Widget
      Tree* changes not at all**, its `Grid { columns: usize }` staying correct.

## Acceptance Criteria

- [ ] **Every deliverable of requirements 1–10 is present:** the `arrange_grid`
      signature and the
      `LayoutMode::Grid { columns } => arrange_grid(nodes, children, inner, columns, config),`
      arm; `grid_column_width` and `grid_row_heights`; the `columns.max(1)` clamp;
      the four `spacing` doc sentences in `layout.rs` plus the one in
      `container.rs`; the `flex` doc sentences; the rewritten `LayoutMode::Grid` doc
      with its `# Examples` doctest; the corrected `wrap` doc; the twenty-one named
      tests; and the three `DEMO_APPLICATION.md` edits.

- [ ] **`LayoutMode::Grid` no longer appears with `Vec::new()` in `arrange`** and
      `columns` is read in production, so no node's mode changed and no rect moved;
      `grid_has_no_algorithm_yet` is replaced by
      `grid_places_children_instead_of_reporting_nothing`, which asserts a 50 × 100
      rect at the origin; and replacing `columns.max(1)` with `columns` panics in
      `usize::div_ceil(0)`.

- [ ] **The property test sweeps its 216 pairs**, and the handoff records that
      `i / columns`→`i % columns`, a cursor for
      `count_to_f32(column) * (column_width + gap)`, and a negative `gap` each fail
      it; `rg -n 'Wrapping arrives' ui/src/ui_core/src/` returns nothing; and the
      `flex` setters' docs name the modes that honour the factor.

- [ ] **`DEMO_APPLICATION.md` rows `L3` and `#2` carry the dated, attributed note**
      (Grid clause closed, `wrap` clause not; neither deleted nor marked closed),
      `L3`'s evidence column names symbols not line numbers and its Blocks keeps
      *"The Controls tile grid, the app tray grid"*, row `L7` gains the one spacing
      sentence and stays open, and no other row changed.

- [ ] **`cargo test --all-features` is green with every named test present**, against
      the baseline **1894**, `layout_walk_cost` still `#[ignore]`d and unmodified;
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean; **`cargo audit` recorded as not installed on this
      host rather than passed** (`.ai/agents/developer.md` § *Phase 3*).

- [ ] **No page moves and every page's frame rate is measured.** `Page::ALL`'s six,
      release build, before/after per `.ai/tools/README.md` § *Capturing a window*,
      with **AE 0 outside `y ≥ 680`** on all six; `.ai/tools/fps-check.sh 10 55`
      then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` per page,
      each above **55** and inside the **61.1–63.9** band. The rect-level gallery
      tests keep their names and assertions.

- [ ] **Nothing from another task leaked in:** `render.rs`, `render/`, `paint.rs`,
      `batch.rs`, `node.rs`, `arena.rs`, `property.rs`, `animation.rs`, `input.rs`,
      `theme.rs`, `widgets/label.rs`, `widgets/container.rs`'s code except the one
      doc sentence, `ui/src/ui_demo/src/main.rs`, `ui/Cargo.toml` and
      `ui/Cargo.lock` all absent; no new `unsafe` or
      `unwrap`/`expect`/`panic!`/`unimplemented!`/`todo!`; the only `usize`→`f32` in
      `arrange_grid` is `count_to_f32`; no `pub` signature changed; and
      `doc/ui/PRIMITIVES_ARCHITECTURE.md` untouched.

- [ ] **The handoff does not claim a grid appears on screen** (the demo's Controls
      and app-tray grids are `TASK_UI_DEMO_n`; the way in is
      `Container::new(nodes, LayoutMode::Grid { columns: n })`; the evidence is
      geometry over a swept space, not that it looks right) and says a cell may
      exceed its slot only via `clamp_axis`.

## Out of Scope

- **No margin, no `flex-shrink`, no `flex-basis`, and no second gap parameter — gap
  `L7`, a separate task.** No `margin`/`flex_shrink`/`flex_basis` field on
  `LayoutState` and no `GridConfig` type. This task does not depend on `L7` closing.
- **No flex `wrap`, in flex or in grid, and no wrapping anywhere** — a grid is the
  wrapped layout with a column count in place of a break rule. `LayoutMode::Flex {
  wrap }` stays declared and unhonoured. Honouring it (line breaking, a per-line
  cross size, per-line `MainAxisAlignment`, and the boundary cases of a child wider
  than the box and an exactly-divisible count) is its own task.
- **No auto-fill, no `Option<usize>`, and no minimum-column-width constant.**
  `columns` stays `usize`; a caller that wants auto-fill computes the count and
  writes `LayoutMode::Grid { columns: computed }`. Revisit when a measured minimum
  tile width exists in this repository.
- **No row or column spans, no explicit row or column index, no track-based sizing,
  no alignment, and no content-sized or weighted columns.** `Grid { columns }` gains
  no second field; `struct Placement` carries `handle: Handle`, not
  `Option<Handle>`; neither `MainAxisAlignment` nor `CrossAxisAlignment` is read; no
  `1fr`, no `auto` track, no per-column weight.
- **No change to any other mode, and none to the flex path:** `arrange_flex`,
  `arrange_stack`, `arrange_absolute`, `distribute`, `alignment_spacing`,
  `cross_offset`, `direction_cross_to_xy`, `bounding_box`, `content_size`,
  `resolve_box`, `sized`, `intersect`, `clamp_axis` and `count_to_f32` are
  byte-for-byte unchanged; the two padding steps stay outside the match; `struct
  FlexItem` is not reused; `FlexConfig` gains no field and changes no signature.
- **No demo page, no tile, and no Controls or app-tray grid.** `L3`'s Blocks names
  both and neither is delivered; both are `TASK_UI_DEMO_n`. No page is added,
  `Page::ALL` is unchanged, and `ui/src/ui_demo/src/main.rs` has no diff.
- **No scroll-container integration, no grid inside `List`, no virtualisation, and no
  nested-grid semantics beyond what falls out.** A cell is a box and a grid in a box
  lays its children out in it; a grid's whole box is laid out in one pass, which
  `Layout::visit`'s dirty-subtree skip already provides; there is no row-alignment,
  column-alignment or subgrid.
- **No new dependency and no `unsafe`.** The approved direct dependencies are
  `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; a grid is arithmetic over `f32`
  and `usize`, so `ui/Cargo.toml` and `ui/Cargo.lock` are untouched. Edition 2021
  and `rust-version = "1.85"` are respected — `usize::div_ceil` is stable since
  1.73 and nothing newer is used.
