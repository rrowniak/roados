# TASK_UI_PRIM_52: `LayoutMode::Grid` — Equal Columns, Filled Row-Major, Rows as Tall as Their Tallest Cell

## Goal

Give `LayoutMode::Grid` the algorithm it does not have. A grid node reads its
`columns`, splits the box it was given into that many **equal** columns with
**one** gap between them, fills its children **row-major**, sizes **each row to
its tallest cell** and gives **every cell in that row that height**. A grid node
then lays out its children and reports one rect per child, where today it
reports none.

**No new API is needed to reach it, and that is the reason this is a one-file
code change.** `Container::new(nodes, mode: LayoutMode)` and
`Container::set_mode(nodes, mode)` both take a `LayoutMode` today
(`ui/src/ui_core/src/widgets/container.rs`), so `Container::new(nodes,
LayoutMode::Grid { columns: 4 })` is a reachable call once the arm exists. The
demo's tile grids are a `TASK_UI_DEMO_n` and not this task; § *Out of Scope* says
so and names why.

## Context

### The gap is recorded **twice**, and both rows are one gap

**Row `L3`** in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*
reads, verbatim:

> **`LayoutMode::Grid` unimplemented** — `LayoutMode::Grid { .. } => Vec::new()`,
> `columns` declared and never read. `Flex.wrap` accepted and discarded by the
> `..`.

**Row `#2`** in `DEMO_APPLICATION.md` § *Library gaps* reads, verbatim:

> **Grid layout non-functional** — `Grid` mode lays out no children and reports
> no rects; `wrap` is accepted and not honoured.

**These are the same gap recorded in two tables, and they are not two gaps.** The
same clause — a grid lays out nothing — appears in both, with the same evidence
behind it; the second table's prose says explicitly that the first table's rows
are *"the original 2026-09-30 assessment, kept so the original reasoning is
auditable"* and that the `L1..L11` rows are the ones the Layout work exposed.
**Both rows must therefore be amended by this task, and neither is deleted or
marked closed** — § *Library gaps* records that the first eight rows *"keep their
numbering, because four files outside this one cite it by number"*, which is
exactly the constraint `TASK_UI_PRIM_24.1`'s lesson
(`.ai/NEVERAGAIN.md` § *A position API that only one parent mode reads*, in
general: a sweep of one table leaves the other stale) turned on an amendment.

Two differences between the rows are recorded rather than smoothed over, because
a later reader will notice them:

| | row `#2` | row `L3` |
|---|---|---|
| Severity | **High** | **Critical** |
| Blocks | *"App launcher screen"* | *"The Controls tile grid, the app tray grid"* |

**And the row has two clauses, and this task closes one of them.** The Grid
clause closes. **The `Flex.wrap` clause does not** — § *`Flex.wrap`: decided,
and a false promise removed* below is the whole of that decision, and the short
form is that a grid is *already* the wrapped layout with a different line-break
rule, so implementing `wrap` in flex would be a second spelling of the thing this
task builds.

### What exists at the moment of writing, established by reading the source

Every line of this list was read out of the tree, and each is cited by symbol and
path — **never by line number**, because the tree is being modified in parallel
and a line number into `ui/src` is stale within the hour.

- **`fn arrange` in `ui/src/ui_core/src/layout.rs` has a fifth arm**, and it is
  `LayoutMode::Grid { .. } => Vec::new(),` — the `..` discarding `columns`, and
  the arm returning an empty `Vec`. **So a grid node lays out no children and
  reports no rects**, which is the entire of gap `L3`.
- **`LayoutMode::Grid`'s `columns: usize` is declared and read nowhere in
  production.** The only other mention of `Grid` anywhere in `ui/src` is the one
  test `grid_has_no_algorithm_yet` in `layout.rs`'s `#[cfg(test)] mod tests`,
  which constructs `LayoutMode::Grid { columns: 2 }` and asserts
  `rects.is_empty()`. **That test asserts the absence of the behaviour this task
  adds, so it must be replaced by its opposite rather than deleted** —
  requirement 4 and the corresponding acceptance criterion.
- **The variant's doc, verbatim** (this is the sentence that goes away):

  > A grid of `columns` columns. The mode is part of the layout vocabulary but
  > its algorithm is out of scope for this task; a grid node lays out no children
  > and reports no rects.

- **`FlexConfig`** in `layout.rs` is `pub` with three **private** fields —
  `main_axis_alignment`, `cross_axis_alignment`, `spacing` — and three `#[must_use]`
  getters, so a caller reads them and writes them only through the `with_` and
  `set_` builders. **`FlexConfig::with_spacing` clamps a negative spacing to
  zero**, and its doc says so: *"overlapping children are what
  [`LayoutMode::Stack`] is for."*
- **`FlexConfig::spacing`'s doc says *"the gap between children on the main
  axis"*, and there is no cross-axis gap anywhere in the crate.** One `spacing`
  value is the whole of the gap vocabulary: `MainAxisAlignment` has six variants
  and `CrossAxisAlignment` four, and neither carries a second gap.
- **`struct FlexItem`** in `layout.rs` is private and carries `{ handle, flex: f32,
  declared: Constraints, desired: Size, main, cross, tight_cross: bool }`. **It is
  a flex item, in main/cross terms, and a grid reuses none of it** — see § *The
  algorithm*'s note on vocabulary.
- **`fn arrange_flex(nodes, children, inner, direction, config)`** is the flex
  path a grid sits beside, and it runs four steps: measure every child's `desired`
  in `constraints.loosen()`; split the main axis into `rigid_total` plus whatever
  `fn distribute` hands the `flex > 0.0` items; take each item's cross extent from
  `clamp_axis` when it is `tight_cross`; then turn the leftover into a leading
  offset and a gap through `fn alignment_spacing`. **`fn distribute` is the only
  thing in the crate that shares free space between children**, and a grid has no
  free space to share — § *`flex` is not read inside a grid cell* below is why.
- **`fn arrange` applies `Padding` in two steps around the mode call**, and both
  steps are outside every mode's arm: `padding.inset(constraints)` shrinks the box
  the mode is given, and afterwards every placement's `rect.origin` is offset by
  `(padding.left, padding.top)`. **`fn content_size` adds `padding.right` and
  `padding.bottom`** to the extent `fn bounding_box` measured from the node's own
  origin — the far side only, because the placements are already offset by the
  leading side. **`Padding` is a `LayoutState` field, not a `Container` field, and
  this is why: the pass is what applies it.** **A grid therefore gets padding for
  free and needs none of its own.**
- **`LayoutMode::Flex { direction, wrap }`**'s variant doc says, verbatim:
  *"children stay on a single line and an overflowing child is clipped rather
  than wrapped. **Wrapping arrives with the list and scroll widgets.**"* **That
  last sentence is false as of the tree this task is written against**:
  `ui/src/ui_core/src/widgets/mod.rs` carries **fifteen** `pub mod` entries and
  both `pub mod list;` and `pub mod scroll;` are among them, so both widgets
  shipped and neither wraps. **The field's own doc — *"Whether children should
  wrap onto further lines. Not yet honoured."* — is the accurate one**, and the
  variant doc's promise is the defect. Two doc comments asserting the opposite of
  the code beside them is the shape `DEMO_APPLICATION.md`
  § *Corrections to the second gap table* records twice.
- **`LayoutMode` has four variants** — `Flex { direction, wrap }`,
  `Grid { columns }`, `Stack`, `Absolute`, with `#[derive(Clone, Copy, Debug,
  Default, PartialEq)]` and `Stack` as `#[default]` — and the supporting types
  are `Constraints`, `Rect`, `Size`, `Offset` and `Handle`. **`LayoutState`
  carries** `mode`, `flex_config`, `constraints`, `flex`, `position`, `padding`,
  `rect`, `clip`, `dirty`, `placed_under` and `visible`, and **there is no
  margin anywhere in the crate**: no `LayoutMode` field, no `LayoutState` field,
  no `Layout` field.
- **`fn clamp_axis(value, min, max)`** in `layout.rs` is the crate's single clamp,
  and its doc records the rule that makes grid arithmetic safe: **`f32::clamp`
  panics when `min > max`, a constraint set whose minimum exceeds its maximum is
  reachable, and the minimum wins** — *"it is the bound a caller can still act on,
  whereas honouring a lower maximum would silently push a child below the size it
  declared."* It also passes a `NaN` through rather than turning one into a panic.
- **`fn count_to_f32(count: usize) -> f32`** is the file's **one** `usize`-to-`f32`
  cast, and its doc says so. **Every `usize` this task's arithmetic converts is
  converted through it**, which is a reviewable rule rather than a preference.
- **`every_segment_quad_is_convex_over_two_hundred_thousand_geometries`** in
  `ui/src/ui_core/src/widgets/chart.rs` is the house precedent for correctness in
  layout-adjacent arithmetic: a property asserted over a large swept space with the
  count in the test's own name, built on a seeded LCG written out inline in the
  test. **Requirement 9's property test is that shape**, and § *The test suite*
  says why its space is swept exhaustively rather than sampled.
- **`AGENTS.md` and `developer.md` bind the edges**: **no new dependency without
  the operator** — the approved direct dependencies are `sdl3 0.20`, `glow 0.18`
  and `freetype-rs 0.38` and a grid is arithmetic — **edition 2021 and
  `rust-version = "1.85"`**, so `usize::div_ceil` (stable since 1.73) is available
  and nothing newer is. **Tests go in a `#[cfg(test)] mod tests` beside the code,
  with no test that needs a display, a network, a filesystem or the wall clock.**
- **`layout_walk_cost` in `layout.rs` is the suite's one `#[ignore]`d test** — a
  `use std::time::Instant` harness, so it reads the wall clock and is not a test.
  **This task does not touch it**, and the acceptance criterion says so by name,
  because its numbers in `IMPLEMENTATION_STATE.md` § *Deviations from the spec,
  and why* are reproducible only while the function it measures is unmodified.
- **`.ai/tools/fps-check.sh` takes `seconds` then `floor`, builds release, runs
  `./target/release/ui_demo` with no arguments and no page**, and exits 1 both on
  a missed floor **and on a run that printed no `roados-fps` line at all** —
  *"a missing report line is an aborted measurement, not a fast one."* **It cannot
  name a page**, which is why per-page measurement is
  `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>` with the line
  parsed by hand. This is the gate in `task-sequence.md` § *Gates* and
  `developer.md` § Phase 3, and it is not optional.
- **The test baseline, measured on this tree, is 1894** — `cargo test
  --all-features` from `ui/` reads **1450** in `ui_core` (`1 ignored`, which is
  `layout_walk_cost`), **224** in `ui_demo` and **220** doctests. **Every test
  this task adds is additive**, and the count is quoted here so that "green" means
  a number rather than a feeling.

### The gap has no upstream dependency, and that is stated rather than left blank

`TASK_UI_PRIM_40.md` § *Context* opens with a seven-row dependency table. **This
task has no such table**, and the absence is deliberate: nothing in
`ui/src` constructs `LayoutMode::Grid`, nothing reads it, and the algorithm's
inputs (`Constraints`, `FlexConfig::spacing`, `clamp_axis`, `count_to_f32`,
`sized`, `bounding_box`, `content_size`) are all present and unchanged. **The
table would be empty, and an empty table in that file's shape is a claim that
there is nothing to get wrong.**

### The algorithm: equal columns, row-major, rows as tall as their tallest cell

**The rule in one line.** `columns` is a **column count**, not a measurement:
the grid splits its box into `columns` equal columns, places child `i` at row
`i / columns` and column `i % columns`, and every cell in row `r` is exactly
`column_width × row_heights[r]`. **Nothing in the cell's own data decides its
size**, which is what makes a grid a tiling rather than a table.

Six decisions are inside that sentence, and each is argued because each has a
defensible alternative.

1. **Equal columns, not content-sized ones.** Four reasons, the third the one
   that settles it. First, *"a grid of `columns` columns"* is a statement about
   columns, and content-sized columns make the count a label for whatever the
   data happened to be. Second, content-sized columns need a per-column width
   *and* a rule for what happens when the widest column and the box disagree,
   which is a second mechanism — and this task adds one mechanism. Third, **the
   demo is the consumer and it needs alignment**: `DEMO_APPLICATION.md`
   § *The Controls panel* specifies a right pane whose tiles are a **4-wide row**
   and then a **3-wide, two-row block captioned *"2×3 grid, tiles carry state"*,
   and a grid of unequal columns puts a visible seam down the middle of that
   block at every column boundary where the seam is not drawn. Fourth, equal
   columns make `columns` load-bearing in the arithmetic itself, which is what
   makes "the field is read in production" true rather than nominal.
2. **Row-major flow.** Child `i` goes to row `i / columns`, column `i % columns`.
   It is the only flow expressible without a new field: the variant carries no
   row index, no `grid-row` and no ordering key, and **row-major is the flow
   `Grid { columns }` means** — a caller that wants a different order attaches
   its children in that order, which is the mechanism every container in this
   crate already uses.
3. **`columns == 0` lays the children out as **one column**, not as nothing.**
   Three reasons. First, **silently dropping children is the failure this task
   exists to remove**: today a caller writing `LayoutMode::Grid { columns: 0 }`
   gets `Vec::new()` and no error, and there is no diagnostic to notice. Second,
   a column count is usually *derived* — `columns = (available_width /
   MIN_TILE_WIDTH) as usize` is two lines, and `available_width < MIN_TILE_WIDTH`
   yields **zero** — and a derived count that loses every child is a trap. Third,
   **clamping a numeric layout input is this crate's own idiom**: `with_spacing`
   clamps to `max(0.0)` and `with_flex`/`set_flex` clamp to `max(0.0)`, so
   `columns.max(1)` is the same move with a different bound, and the clamp is
   **also what keeps `usize::div_ceil` from panicking** on a zero divisor — a
   public input reaching a panicking stdlib method, which is a `developer.md`
   § *Panics* concern and not a style note.
4. **A short last row is left-aligned: the row occupies only the columns its
   children fill, and no cell is emitted for a column nothing landed in.** The
   alternatives are left-aligned, stretched and centred, and the reason is that
   **only one of the three is a decision and the other two are vacuous under
   equal columns.** A cell's width is already its column's width, so *stretching*
   a leftover cell is not a different rule — it is the same rect. And *centring*
   would move the row's own origin, which is the one free choice left, and it
   would make every row's left edge depend on the item count: a 5-item grid over
   4 columns would have its second row's first tile start at a different x from
   its first row's, which is precisely the raggedness the Controls pane's caption
   rules out. **Left is also the crate's origin convention**: `MainAxisAlignment`
   and `CrossAxisAlignment` both default to `Start`, and `arrange_stack` and
   `arrange_absolute` both put an unpositioned child at `Offset::ZERO`. Emitting a
   cell for an empty column is not on the list because `struct Placement` carries
   `handle: Handle`, not `Option<Handle>` — **an empty cell would need a handle
   that means nothing**, which is the span case § *Out of Scope* declines.
5. **Rows stack from the top of the padded box, and neither alignment is
   honoured.** `MainAxisAlignment`'s own doc says *"How a flex container
   distributes the space left over once its children are sized"*, and
   `FlexConfig`'s says *"How a flex container arranges its children, beyond the
   direction"* — **both are documented as properties of a flex container**, and a
   grid is not one. More decisively, **honouring them would mean honouring two of
   six main-axis values**: `Start`/`End`/`Center` are meaningful for a grid block
   on the cross axis, but `SpaceBetween`/`SpaceAround`/`SpaceEvenly` have no
   meaning there at all — they would either change the column spacing, which
   defeats equal columns, or do nothing. **A feature that honours three of four
   cross-axis values and none of three main-axis values is one a reader has to
   discover by trying them**, so the grid honours neither and says so.
6. **A row overflows the box rather than being squeezed.** Rows keep
   `row_heights[r]` and the grid's last row may run past the bottom of the box.
   There is **no `flex-shrink` in the crate** (gap `L7`, a separate task), a grid
   has no distribution step at all, and squeezing a row would push a tile's
   content out of its own tile — the failure at the wrong level — rather than
   pushing the tile out of the grid, which is what an overflowing child does
   everywhere else in this crate. **The clip already exists**:
   `Layout::visit` computes `children_clip = intersect(clip, Some(rect))` for
   every child of a node, and `an_overflowing_child_keeps_its_size_and_computes_a
   _clip_rect` already pins that the overflow is confined.

### `columns` stays `usize`. Not `Option<usize>`, and not auto-fill.

**Decision: `columns` stays `usize`; no `Option`; no auto-fill; no minimum-column
width; no second field.** Four reasons, and the third is the one that makes this
a decision rather than an omission.

1. **Auto-fill needs a target column width, and this crate has no source for
   one.** The Controls tile grid is icon-plus-label and roughly square; the app
   tray is icon-only; the app launcher in `DEMO_APPLICATION.md` § *Design
   principles* is *"grid of icons"*. **A `MIN_GRID_COLUMN_WIDTH` would be a
   number with no measurement behind it**, which is the failure
   `DEMO_APPLICATION.md` § *Asset requirements* records for colours nobody could
   verify, and the same shape as `TASK_UI_PRIM_40`'s rejection of a `Swipe`
   magnitude. **An invented constant in a layout pass is worse than an invented
   one in a widget**, because every node on screen inherits it.
2. **Auto-fill makes a grid's column count a function of its box, and that
   breaks the crate's one stated sizing principle.** `layout.rs`'s module doc:
   *"Asking and placing are the same code (`resolve_box` decides the box a node
   lays its children out in, and the parent measures the child with the same
   call), so a parent and a child never disagree about a size."*
   `fn content_size` measures a node's children under **`Constraints::UNBOUNDED`**
   and the doc on it says why: *"the content is measured unbounded so that a
   node's size stays a property of its children rather than of how it arranges
   them."* **Under auto-fill a grid measured unbounded fills one column and the
   same grid placed in a wide box fills eight**, so its own size becomes a
   function of the box it was put in — the exact disagreement the module doc
   says must never happen, reached by a different road.
3. **The seam is already there and it costs a caller one line.** A caller that
   wants auto-fill computes `columns` itself and writes
   `LayoutMode::Grid { columns: computed }`. **No crate change is needed for
   that**, and the caller owns the constant it would otherwise hide — which is
   also why `Container::new(nodes, LayoutMode::Grid { columns: n })` needs nothing
   from this task.
4. **`Option<usize>` widens a `pub` enum for a capability nothing needs.**
   `LayoutMode` is `pub` and derives `PartialEq`, and its constructors
   `LayoutMode::row()` and `LayoutMode::column()` are in its own doctests. A
   `Some`/`None` on a field of a public enum is a semver-visible surface bought
   for a feature this task does not ship. **If `columns` becomes optional later,
   the field type changes then, when something needs it to.**

`doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Widget Tree* carries the enum in a code
block as `Grid { columns: usize }`. **This decision means that block stays
correct**, and the acceptance criterion says so — a semver-visible API change
would have made a second document wrong.

### Cross-axis sizing: a row is as tall as its tallest cell, and its cells share that height

**Decision: `row_heights[r] = max(desired[i].height for i in row r)`, and every
cell in row `r` is exactly that tall. Rows do not share a height with each
other.** Each cell is also exactly one column wide, so **a cell's rect is its
slot** — see the caveat in § *The test suite* on what a cell may then do to it.

Three reasons, and the first is the spec's own words.

1. **`DEMO_APPLICATION.md` § *The Controls panel* makes tiles of different
   heights by design.** Its four cell archetypes include *"Action tile — icon +
   label, sometimes with a state dot (Recording, Sentry) and sometimes with a
   **secondary line** (`Child Lock / off`, `Neutral / Hold`)"*. **A tile with a
   secondary line is taller than its neighbour by construction**, and the ASCII
   in that section draws the 3×2 block with its two rows sharing vertical
   boundaries. Without a shared row height the block is a staircase: the tiles
   with a secondary line extend below their neighbours' bottom edges and the
   horizontal rules the sketch draws do not exist.
2. **Row heights are per row, and deliberately not all equal.** Forcing every
   row to the height of the tallest child in the grid would make a 3-row grid
   with one tall tile as tall as three tall tiles. **A per-row height is what
   "each row is as tall as its own tallest cell" means**, and the two-row
   Controls block is the case that shows the difference between the readings.
3. **This is `CrossAxisAlignment::Stretch` with the choice taken away.** In flex,
   a parent decides whether its children fill the cross axis; a grid has no
   parent to ask, so the grid decides. **Every cell in a row being the same
   height is also what makes a tile grid read as tiles rather than as a column of
   differently-sized boxes.**

**A cell's size is its slot, tightened by its own declared constraints, and that
is the crate's existing behaviour rather than a new rule.** `arrange_grid` hands
each cell **`Constraints::tight(Size::new(column_width, row_heights[row]))`**,
and `Layout::visit` passes that straight down; `fn resolve_box` then does
`incoming.tighten(node.layout().constraints())` and, if the result is tight,
returns it. So:

- **a cell that declares nothing takes the whole slot** — the tiling rule, and
  the one the demo's tiles rely on;
- **a cell that declares a tight size keeps its own size** on that axis, because
  `tighten` of two tight boxes of one size is that size;
- **a cell whose declared minimum exceeds the slot is larger than the slot**,
  because `clamp_axis` returns the minimum when `min > max` — the crate's
  documented rule, the same rule `arrange_flex` already applies, and the reason
  the minimum is the crate's single clamp and not four `.max(0.0)` calls.

**The last clause is the grid's one honest limit and it is stated rather than
removed.** A grid is a tiling **exactly when no cell declares a minimum wider
than its column or taller than its row**; a cell that does overlaps its
neighbour, because the cell's rect is larger than its slot. **The alternative —
forcing the rect to the slot whatever the child declared — is not available at
this layer**: `arrange_grid` hands constraints down and `resolve_box` is what
turns them into a rect, which is the module doc's *"asking and placing are the
same code"* principle. Making a grid override a child would mean writing the
child's rect after its own visit, which breaks that principle and would have to
be undone by the next mode. **The consequence is named by a test**
(`a_cell_whose_declared_minimum_exceeds_the_column_overflows_its_slot`) and is
in § *The test suite* as the reason the property test's space is what it is.

### `Flex.wrap`: decided — it stays a flex-only no-op, and a false promise comes out of the doc

**Decision: this task implements no wrapping, in flex or in grid. `Flex.wrap`
remains declared and unhonoured, `LayoutMode::row()` and `::column()` keep
writing `false`, and the change is to the documentation: the promise comes out
and the honest statement takes its place.** Five reasons.

1. **A grid is the wrapped layout with the line-break rule replaced by a column
   count, and this task builds the second half.** Multi-line placement means
   three things: a break rule, a per-line cross size, and cross-axis alignment
   **per line**. `arrange_grid` provides all three — the break rule is
   `i / columns`, the per-line cross size is `row_heights[r]`, and the
   cross-alignment is the shared height. **What is left over for a flex `wrap` is
   the only hard part: deciding where to break.** So the two are not competing
   implementations of one feature; **grid is the placement and flex wrap is the
   policy, and this task ships the placement.**
2. **Flex wrap is a second algorithm, not a boolean threaded through.**
   `arrange_flex`'s four steps are all single-line: `rigid_total` is one line's
   worth, `fn distribute` shares one line's budget, `fn alignment_spacing`
   returns one leading offset and one gap for one line, and `fn cross_offset`
   aligns one child against one box. Honouring `wrap` means: accumulate lines
   until the next child does not fit, give every item in the line the line's own
   cross size, run the alignment **per line** with the per-line count for
   `SpaceBetween`'s `count - 1.0` divisor, and then place the lines against the
   cross axis. **That is a rewrite of steps 2 through 4**, and it is a task with
   its own boundary cases — a child wider than the box, an exactly-divisible
   count, whether the gap applies at a break — none of which this task's
   property test would reach.
3. **Nothing in the repository sets `wrap: true`.** The only writers of the
   field are `LayoutMode::row()` and `LayoutMode::column()`, both `false`. So
   implementing it would ship a branch **no node in production can enter** —
   which is the `columns` situation this task exists to end, re-created one
   variant over.
4. **Removing the field is not the answer either.** `LayoutMode` is `pub` with
   named-field variants, `LayoutMode::row()`'s doctest asserts
   `LayoutMode::Flex { direction: FlexDirection::Column, wrap: false }`, and
   dropping a field breaks every construction of it. **That is a semver break
   bought for tidiness**, and `developer.md` § *API design* requires discussion,
   not a quiet edit.
5. **So the field stays, and the field's doc is the accurate one — which makes
   the *variant* doc the defect.** *"Not yet honoured"* is true and stays.
   *"Wrapping arrives with the list and scroll widgets"* is **false**, because
   both shipped, and it must be replaced with the statement of where multi-line
   placement actually lives: **`LayoutMode::Grid`, whose row-major flow is
   wrapping with a column count instead of a break rule.** A doc comment that
   promises a feature two delivered widgets did not bring is the kind of thing
   `reviewer.md` § Phase 2 looks for first.

**Consequence for the gap rows, stated so that neither row is closed by
accident: after this task `L3`'s second clause — "`Flex.wrap` accepted and
discarded by the `..`" — is still literally true.** `arrange`'s flex arm is
`LayoutMode::Flex { direction, .. } =>`, and it still discards `wrap`. The row is
amended to say **which clause closed and which did not**, and it is **not
deleted and not marked closed**. Row `#2` says the same thing in prose and is
amended to match. **A grid that lays its children out and a `wrap` that is still
discarded are two different facts, and a row that claims both without
distinguishing them is the defect `DEMO_APPLICATION.md` § *Corrections to the
second gap table* exists to prevent.**

### `flex` is not read inside a grid cell

**Decision: `arrange_grid` never reads `node.layout().flex()`. A cell's flex
factor is ignored, silently and by design, and that is written down in the places
a caller will look.** Three reasons.

1. **`LayoutState::flex`'s own doc defines the factor in terms a grid does not
   have**: *"Sets this node's share of the free space on its parent's **main
   axis**."* **A grid has no main axis.** It has two axes, both named, and its
   column width is decided by `columns` and the box — so a flex factor has
   nothing to be a share *of*.
2. **A cell's width is already decided when the factor is read.** Even if the
   grid wanted to honour it, the factor would have to redistribute a column
   among the cells in it, which is `fn distribute`'s job — **and `distribute`
   works in main/cross terms over `FlexItem`, which a grid does not build.**
   Routing grid cells through it would mean constructing `FlexItem`s for a
   flexbox the grid is not, and the uniformity of the tiling would be gone.
3. **This is the exact failure `.ai/NEVERAGAIN.md` § *A position API that only
   one parent mode reads* records, and that entry supplies the remedy**: *"a
   setter that a **parent** consumes is not honoured by every parent, and the
   compiler will not say so. Before writing a placement, read **the parent's
   `LayoutMode` arm** rather than the setter's name."* `position` is honoured by
   `arrange_absolute` and not by `arrange_stack`; `flex` after this task is
   honoured by `arrange_flex` and not by `arrange_grid`. **The remedy that entry
   prescribes is documentation in the settler's own doc plus a test**, and both
   are requirements 6 and 9 below. **Nothing here is a defect: a field that one
   parent mode reads is the normal shape of this API, provided the field says so
   and a test holds it.**

### What a grid takes from `FlexConfig`, and what it does not

**`config.spacing()` is the grid's gap, on **both** axes. Neither alignment is
read.** This is the one place where the answer is "yes, and the docs must change",
so it is stated three times: here, in requirement 5, and in the gap-row amendment
to `L7`.

- **The gap is `config.spacing()` and it goes between columns *and* between
  rows.** One value, two axes. Three reasons. First, `spacing` is the only gap
  the crate has and a grid that ignores it would place cells flush against each
  other, which is a tiling of backgrounds rather than a grid of tiles — and a
  caller who wants no gap writes `with_spacing(0.0)`, which is one call and no
  new parameter. Second, a single value applied to two axes keeps `FlexConfig`
  the same type: **no `row_spacing`, no `column_spacing`, no `GridConfig`, and no
  new field on any public struct.** Third, `with_spacing` already clamps a
  negative to zero, so a grid cannot be handed a negative gap and produce
  overlapping columns — the guard is the existing one, not a new one.
- **This is not gap `L7`.** `L7` reads *"No margin, no `flex-shrink`, no
  `flex-basis`, **no cross-axis gap**"*, and its evidence sentence is that
  *"`spacing` is main-axis only"*. **What `L7` means by a cross-axis gap is a
  second, independent gap parameter** — the `row-gap` / `column-gap` pair a
  wrapped flex container needs — and **after this task there still is not one.**
  There is one gap value, and `LayoutMode::Grid` applies it to two axes because
  a grid's two axes are symmetric. **So `L7` is amended with that sentence and is
  not closed**: margin, shrink, basis and the absence of a second gap parameter
  all still hold, and the amendment says which sentence of the row the grid
  touched and which it did not.
- **Neither alignment is read**, for the reason in § *The algorithm* decision 5.
  A consequence worth writing down: **`FlexConfig::main_axis_alignment` and
  `cross_axis_alignment` on a grid node are inert**, so `Container::set_flex_config`
  is still the right call for the `spacing` on a grid and the caller should not
  expect the alignments to do anything.

### Scope, measured against `developer.md` § *Scope check*

**Four files, two components, both inside the thresholds.**
`developer.md` § *Scope check* says a change touching **more than 5 files** or
having **more than 3 independent components** is too large for one agent, and
that it must then be split per `.ai/protocols/subagents.md` § *Implementation
fan-out*.

| file | what changes |
|---|---|
| `ui/src/ui_core/src/layout.rs` | the `arrange` arm, `arrange_grid` and its two helpers, `LayoutMode::Grid`'s doc, `LayoutMode::Flex`'s `wrap` doc, the `FlexConfig` spacing docs, the `LayoutState::flex` docs, the tests and one doctest |
| `ui/src/ui_core/src/widgets/container.rs` | **one doc sentence** on `Container::set_flex_config`, whose doc says *"how a **flex mode** arranges this container's children"* and is therefore wrong the moment a grid reads its `spacing` |
| `doc/ui/DEMO_APPLICATION.md` | rows `L3` and `#2` amended — the same gap, twice — and row `L7`'s spacing sentence |
| `doc/ui/IMPLEMENTATION_STATE.md` | the task-table row, the record section, and the `Current position` update |

**Components: two.** The layout pass's grid algorithm, and the documentation of
the decision. **`No fan-out is needed** — `.ai/protocols/subagents.md`
§ *Implementation fan-out* does not apply — and the implementer should say so
rather than splitting a two-component change. **If the implementer finds
themselves editing a second *code* file other than `container.rs`'s doc comment,
that is a stop condition, not an expansion** (`developer.md` § *Stop
conditions*).

## Requirements

1. **`arrange`'s grid arm calls a new function, with this exact signature:**

   ```rust
   fn arrange_grid(
       nodes: &Arena<WidgetNode>,
       children: &[Handle],
       constraints: Constraints,
       columns: usize,
       config: &FlexConfig,
   ) -> Vec<Placement>
   ```

   and the arm in `arrange` in `ui/src/ui_core/src/layout.rs` becomes
   `LayoutMode::Grid { columns } => arrange_grid(nodes, children, inner, columns, config),`.
   **`inner` is `padding.inset(constraints)`, the same value the other three arms
   are given**, because the two padding steps stay outside the match exactly as
   they are today — a grid gets padding for free and must not take it twice.
   `arrange_stack` and `arrange_absolute` are the shape: a mode's arm is one line
   and its function is named after it.

2. **The algorithm, in full, because "implementable without questions" is the
   requirement and this is where the questions would otherwise live.** Inside
   `arrange_grid`, in this order:

   - `if children.is_empty() { return Vec::new(); }` — **`arrange_flex`'s own
     first line, and it is what makes `rows >= 1` and a `row_heights[0]` index
     below safe.** Do not drop it on the grounds that the loop would produce
     nothing anyway; the index is the reason.
   - `let columns = columns.max(1);` — requirement 4's clamp, and the reason
     `usize::div_ceil` cannot panic.
   - `let gap = config.spacing();` and
     `let column_gaps = count_to_f32(columns - 1);` — **`columns - 1` cannot
     underflow because `columns >= 1`,** and every `usize` conversion in this
     function is `count_to_f32`, per that helper's own doc.
   - `let loose = constraints.loosen();` and one `desired` size per child:
     `nodes.get(handle).is_none_or(|_| sized(nodes, handle, loose))` — the
     `is_none_or` form `Layout::moved` already uses, so a stale handle is
     `false` and the cell takes `Size::ZERO`. **A handle that no longer resolves
     is `Size::ZERO` and still occupies its cell**, which is `arrange_flex`'s rule
     stated for a function that has no `FlexItem` for it: *"so the result stays
     index-aligned with `children`."*
   - `let rows = children.len().div_ceil(columns);`
   - **Available width.** `let available = if constraints.max_width.is_finite()
     { constraints.max_width } else { /* the widest row's own need */ };` — the
     unbounded branch is `arrange_flex`'s treatment of an unbounded main axis
     (`available_main = rigid_total + gaps`) with **max over rows** in place of
     the sum, because a grid's rows are its lines and a grid's width is its
     widest one. Computed as `max over rows of (Σ that row's desired widths +
     gap * (that row's count - 1))`, which for a row of equal cells is exactly
     `columns * w + (columns - 1) * gap`. **`content_size` calls `arrange` with
     `Constraints::UNBOUNDED`, so this branch is not an edge case — it is the
     path a grid with no declared constraints takes every time it is measured**,
     and it is why a grid sizes itself to its cells instead of collapsing to
     zero.
   - `let column_width = grid_column_width(available, columns, gap);` — requirement 3.
   - `let row_heights = grid_row_heights(&desired, columns);` — requirement 3.
   - **The placement loop**, over `row` in `0..rows`, and inside it `index` in
     `first..last` where `first = row * columns` and
     `last = (first + columns).min(children.len())`:

     ```rust
     let x = count_to_f32(column) * (column_width + gap);
     let y = <the running sum of this row's and every earlier row's height, plus one gap each>;
     let size = Size::new(column_width, row_heights[row]);
     ```

     **`x` is computed multiplicatively — `column * (column_width + gap)` — and
     not by accumulating a cursor.** The property § *The test suite* asserts is
     that no two cells overlap, and a cursor's error grows with the column index
     while a product's does not. **Cell origins are in the box's own coordinates**,
     because `Layout::visit` adds the node's origin to each placement and says so
     where it does it.

   - `placements.push(Placement { handle: children[index], rect: Rect::new(Offset::new(x, y), size),
     constraints: Constraints::tight(size) });`
     **The placement's `constraints` is `tight` of the cell's own size, which is
     what makes the child's rect its slot** — a loose box would let `resolve_box`
     measure the child down to its natural size inside the slot, and a grid whose
     cells do not fill their slots is not a tiling. **No new sizing rule is
     introduced here at all**: the slot is arithmetic and every override on top of
     it is `resolve_box`'s existing `tighten` and `clamp_axis`, which is why
     § *Cross-axis sizing* can state the override rules as consequences rather
     than as new behaviour.

3. **Two private helpers, one caller each, each doc-commented, and neither
   reachable from a test.** `developer.md` § *Phase 2* allows a function with one
   caller; what it refuses is an abstraction with one, and the two reasons these
   exist are that each is the one piece of arithmetic a reviewer checks by hand
   and that each contains a place the arithmetic can go wrong:

   ```rust
   /// The width each of `columns` equal columns gets in a box `available` wide,
   /// with `gap` between them.
   ///
   /// `columns` is clamped by the caller and is at least 1, so the division has
   /// no zero divisor; the subtraction is floored at zero so a box too narrow to
   /// hold the gaps gives cells of zero width rather than of negative width,
   /// which is the same rule `fn clamp_axis` applies to a constraint set whose
   /// minimum exceeds its maximum.
   fn grid_column_width(available: f32, columns: usize, gap: f32) -> f32

   /// The height of each row of `columns`: the tallest desired height in it.
   ///
   /// One entry per row that has a child, so `row_heights.len()` is
   /// `desired.len().div_ceil(columns)` and indexing it with a row index from the
   /// placement loop is in bounds. Rows do not share a height with each other.
   fn grid_row_heights(desired: &[Size], columns: usize) -> Vec<f32>
   ```

   `grid_column_width`'s body is
   `(available - gap * count_to_f32(columns - 1)).max(0.0) / count_to_f32(columns)`
   and `grid_row_heights` allocates with `Vec::with_capacity(rows)`. **The single
   `Vec<f32>` is consistent with what the flex path already allocates** —
   `arrange_flex` builds `Vec::with_capacity(children.len())` items and
   `distribute` builds `vec![false; items.len()]` — so this is not a new per-frame
   cost shape.

4. **`columns == 0` lays the children out as one column.** The clamp is
   `columns.max(1)` at the top of `arrange_grid`, for the three reasons in
   § *The algorithm* decision 3 — chiefly that **silently dropping children is
   the failure this task exists to remove**, and that a column count is usually
   derived and can derive to zero. **`LayoutMode::Grid { columns: 0 }` is
   publicly constructible today**, so the degenerate case is reachable from
   outside the crate and cannot be an internal assertion.

5. **The `spacing` documentation changes, in four places in `layout.rs` and one in
   `container.rs`.** `FlexConfig`'s doc, `FlexConfig::with_spacing`'s doc,
   `FlexConfig::spacing`'s doc and the `spacing` field's own doc all currently
   scope the value to the main axis; each gains the sentence that
   **`LayoutMode::Grid` uses it as its gap on both axes, from one value, and that
   this is not a second gap parameter**. **`Container::set_flex_config`'s doc says
   *"how a **flex mode** arranges this container's children"* and becomes false
   the moment the grid arm reads `spacing`**, so it gains the same sentence.
   **No type changes, no new field, no new getter, and `MainAxisAlignment` and
   `CrossAxisAlignment` are untouched.**

6. **`LayoutState`'s `flex` documentation names the modes that honour it.**
   `LayoutState::flex`'s doc, `with_flex`'s doc and `set_flex`'s doc each say
   today that the factor is a share of the free space on the **parent's** main
   axis, and each gains: **`LayoutMode::Flex` honours it and no other mode does;
   a grid cell's factor is ignored, because a grid has no main axis and its
   column width is decided by `columns` and the box.** **This is
   `.ai/NEVERAGAIN.md` § *A position API that only one parent mode reads*'s
   remedy applied to the other setter** — the entry's rule is *"read the parent's
   `LayoutMode` arm rather than the setter's name"*, and the two things that make
   the answer findable are the doc and the test. **Nothing else in
   `LayoutState` changes**, and `position` is not touched by this task.

7. **`LayoutMode::Grid`'s doc is rewritten and carries a doctest.** The sentence
   *"its algorithm is out of scope for this task; a grid node lays out no children
   and reports no rects"* is **deleted**, not qualified. The replacement states,
   in this order: **`columns` equal columns filled row-major; one row as tall as
   its tallest cell, with every cell in that row that tall; `config.spacing()` as
   the gap between columns and between rows; `columns: 0` laid out as one column;
   a short last row left-aligned, occupying only the columns its children fill;
   rows overflowing the box rather than being squeezed; the alignments not
   honoured; a cell's flex factor not read.** **Each clause is one of a decision
   above and none is new here.**
   **A `# Examples` doctest** — `developer.md` § *API design*: *"Doc comments on
   all public APIs. Doc tests count as tests — write them."* The example builds a
   two-column grid of four leaves in an `Arena` through `node::create` and
   `Layout::new(...).layout(...)`, exactly the shape the module's own doc example
   at the top of `layout.rs` and `LayoutMode::row`'s doctest already use, and
   asserts the four rects by value. **A doctest on an enum variant's doc comment
   runs**, so this is a real test and it is counted in the doctest total.

8. **`LayoutMode::Flex`'s `wrap` documentation is corrected.** The variant doc's
   *"Wrapping arrives with the list and scroll widgets."* is **deleted**, because
   `pub mod list;` and `pub mod scroll;` are both in
   `ui/src/ui_core/src/widgets/mod.rs` and neither wraps. It is replaced by the
   statement of where multi-line placement lives: **wrapping is not honoured, and
   `LayoutMode::Grid` is where a node places children on more than one line**,
   because its row-major flow is wrapping with a column count in place of a break
   rule. The field's own doc — *"Whether children should wrap onto further lines.
   Not yet honoured."* — **stays as it is, because it is true.** `arrange`'s
   flex arm stays `LayoutMode::Flex { direction, .. }`, `LayoutMode::row()` and
   `::column()` keep writing `false`, and **no wrapping is implemented anywhere in
   this task** — § *`Flex.wrap`* gives the five reasons.

9. **The tests, named, beside the code in `layout.rs`'s `#[cfg(test)] mod tests`,
   with no display, no network, no filesystem and no wall clock** — the only kind
   `AGENTS.md` permits. **Every assertion goes through `layout_constraints` or
   `Layout::layout`, and no test names a private function**: `arrange_grid`,
   `grid_column_width` and `grid_row_heights` are reached only through the public
   path, because `reviewer.md` § Phase 1 asks whether a test asserts the contract
   or the implementation and a test that calls a private helper answers that by
   construction. **The existing helpers are used as they are**: `leaf` for a
   declared tight size, `container` and `padded` for a node with children in a
   mode with and without padding, `layout_in` for a tight box.

   - **`every_grid_cell_is_inside_the_grid_and_no_two_cells_overlap`** — the
     required property test, and the one a reviewer should break first. **It
     sweeps the space exhaustively rather than sampling it, and the reason is
     that the space is small**: `columns` in `1..=8` and a child count in
     `0..=columns * 3 + 2` — 216 (columns, count) pairs — and **no seed, because
     a full sweep of small integers is strictly stronger than a sample of the
     same set.** Cell *sizes* are varied by a seeded LCG written out inline in
     `chart.rs`'s own `every_segment_quad_is_convex_over_two_hundred_thousand
     _geometries` shape, so the geometry is varied while the space is covered.
     **Cells declare nothing** — a bare `LayoutState::new()` — **or declare
     `Constraints::loose(..)`**, and both fill their slot; the `loose` half is in
     the sweep because its maximum is *below* the slot and the slot's own minimum
     still wins, which is `clamp_axis`'s documented rule exercised for free.
     Each pair asserts, on the rects `layout_constraints` returns:
     **one rect per child, in order**; **every rect inside the grid's own content
     box on all four edges** — which for `columns == 1` is the box itself, and
     which is asserted with the tolerance in the next paragraph; and **no two
     rects' interiors intersect**, written as the four-sided separation test
     (`a.far_corner().x <= b.origin.x || b.far_corner().x <= a.origin.x ||` the
     same on y) so that cells which *touch* are not called overlapping.
   - **`const GRID_EPSILON: f32 = 1e-3;` in the test module**, doc-commented with
     the arithmetic that forces it: **a cell's far edge is `x + column_width` in
     one rounding and its neighbour's origin is `(column + 1) * column_width` in
     another, and the two differ by at most a few `f32` ulps** — of the order of
     `1e-7` pixels at these magnitudes, several orders below the epsilon. **The
     epsilon is on the containment and the separation assertions and is stated
     rather than absorbed**, because a tolerance with no arithmetic behind it is a
     test that passes for the wrong reason.
   - **`grid_places_children_instead_of_reporting_nothing`** — the replacement for
     `grid_has_no_algorithm_yet`, and **the literal opposite of it**: the same
     `LayoutMode::Grid { columns: 2 }` and the same
     `layout_constraints(&nodes, &[child], Constraints::tight(Size::new(100.0,
     100.0)), ..)` call, with `assert!(rects.is_empty())` replaced by an assertion
     that the result has one entry and that the entry is a rect of `50 × 100` at
     the origin — **the child in column 0 of 2, filling its slot**. The name
     states the inversion so that the two names can be read against each other.
     **The old test is replaced, not deleted**, because it asserted the absence of
     the behaviour this task adds and a silently deleted test is how a sequence
     loses a count.
   - **`zero_columns_lays_the_children_out_as_one_column`** — three children,
     `LayoutMode::Grid { columns: 0 }`, asserting three stacked rects of the full
     box width and the claim in the name.
   - **`one_child_gets_one_column_and_the_rest_of_the_row_is_nothing`** —
     `columns: 3` with one child: **one rect, not three**, and its width is a
     third of the box. The name states the leftover decision, which is the one a
     reviewer will otherwise read as a dropped placement.
   - **`an_empty_grid_produces_no_placements`** — the `children.is_empty()` early
     return, which `no_children_produce_no_rects` covers for flex and stack and
     which nothing covers for grid today.
   - **`grid_places_children_row_major_into_equal_columns`** — the ordering, with
     a child count that is not a multiple of `columns`, asserting each child's
     exact rect by value so row-major and column-major cannot both pass.
   - **`a_grid_row_is_as_tall_as_its_tallest_cell_and_every_cell_shares_it`** —
     three cells of three different declared heights in one row, asserting one
     height for all three and that it is the largest of the three.
   - **`rows_do_not_share_a_height_with_each_other`** — two rows whose tallest
     cells differ, asserting the second row's `y` is the first row's height plus
     the gap and that neither row is the other's height.
   - **`a_grid_uses_the_configured_spacing_between_columns_and_between_rows`** —
     `FlexConfig::new().with_spacing(8.0)` on a 2×2 grid of a 100-wide box, with
     the four offsets asserted by value: `column_width` is
     `(100 - 8) / 2 = 46`, the second column starts at `46 + 8 = 54`, and the
     second row starts `row_heights[0] + 8` down. **The same test with
     `with_spacing(-4.0)` is the clamp**: `zero_spacing_makes_a_grid_abut_without
     _overlapping`, and its name says both halves.
   - **`a_grid_ignores_both_alignments_and_stacks_rows_from_the_top`** —
     `with_main_axis_alignment(MainAxisAlignment::End)` **and**
     `with_cross_axis_alignment(CrossAxisAlignment::Center)` on a grid with two
     rows in a box twice as tall as they need: rows still start at the top and
     still at the left. **This is the test that makes decision 5 checkable
     rather than documentary**, and it is the mirror of
     `alignment_shifts_the_leftover_space`.
   - **`a_grid_cell_ignores_its_flex_factor`** — three cells in one row, the first
     with `with_flex(1.0)` and `with_flex(7.0)` on a third: **all three get the
     same width**, and `distribute` is not reached. The mirror of
     `flex_factors_distribute_the_free_space`.
   - **`a_grid_measures_to_the_widest_row_when_the_box_is_unbounded`** — a grid
     with no declared constraints laid out under `Constraints::loose(Size::new(0.0,
     0.0))` (or under a stack's unbounded offer), asserting the grid's own rect is
     its widest row's content and **not zero** — the regression `content_size`'s
     `Constraints::UNBOUNDED` call would otherwise be.
   - **`a_grid_too_narrow_for_its_columns_gives_zero_width_cells_and_overflows`** —
     `columns: 8` in a box `40` wide with `with_spacing(20.0)`: the subtraction
     floors, every cell is `0` wide, and the origins still march rightwards past
     the box. The name states that the overflow is the answer, because
     `grid_column_width`'s `.max(0.0)` is a decision and a test should say which.
   - **`a_padded_grid_offsets_every_cell_by_the_leading_edges`** — built with the
     existing `padded` helper, `Padding::all(20.0)` on a 2×2 grid in a 200-wide
     box: the grid works in the 160-wide inset and every cell's origin carries the
     `+20, +20`. **The mirror of `a_padded_row_offsets_its_children_by_the_leading
     _edges`**, and it is free in the implementation and worth a test anyway
     because the two padding steps sit outside the match and a future edit that
     moves one inside would break exactly this mode.
   - **`a_padded_grid_is_measured_as_its_content_plus_the_padding`** — `Padding::all`
     around a grid under `Constraints::loose`, asserting the grid's own rect, which
     is the `content_size` half: the far-side gaps are added and the near-side
     ones are not, because the placements are already offset by them.
   - **`a_grid_inside_a_grid_cell_lays_out_in_that_cell`** — a two-column grid
     holding a `Container` in `LayoutMode::Grid { columns: 2 }` mode as one of its
     four cells, asserting the inner grid's four cells are placed inside the outer
     cell's rect. **This pins what falls out** — a cell is a box and a grid in a
     box lays out in it — and it exists because a coordinate-space bug in nested
     containers is invisible to every other test in the module; see
     `nested_row_inside_a_column_is_positioned`, which exists for exactly that.
   - **`a_stale_grid_child_keeps_the_result_index_aligned`** — a removed handle in
     the middle of four children, asserting four rects back, the dead one at zero
     size in **its own cell** rather than skipped.
   - **`wrap_is_not_honoured_and_a_wrapped_row_stays_on_one_line`** —
     `LayoutMode::Flex { direction: FlexDirection::Row, wrap: true }` with children
     wider than the box: **one line, and the overflow clipped**. **This is the
     enforcement mechanism for § *`Flex.wrap`*, in the same shape as
     `the_widget_consumes_nothing_but_a_drag`** — a documented behaviour nobody
     honours must be asserted, or a later task that changes it drifts past a
     sentence in a doc comment instead of breaking a test.
   - **`an_overflowing_grid_row_keeps_its_height_and_is_clipped`** — a grid whose
     rows are taller than the box, asserting the last row's rect runs past the
     bottom **and** that its child's `LayoutState::clip` is the intersection. The
     mirror of `an_overflowing_child_keeps_its_size_and_computes_a_clip_rect`.
   - **`a_cell_whose_declared_minimum_exceeds_the_column_overflows_its_slot`** — a
     cell declaring `Constraints::new(400.0, f32::INFINITY, 0.0, f32::INFINITY)` in
     a 100-wide two-column grid: **the cell is 400 wide**, it overlaps its
     neighbour, and the test **says so in its failure message and in
     `LayoutMode::Grid`'s doc**. This is § *Cross-axis sizing*'s honest limit
     pinned, so a reviewer reading the doc finds a test that agrees with it.

10. **The gap rows are amended, dated, attributed, and neither is closed.**
    **`DEMO_APPLICATION.md` carries three edits, and the first two are the same
    gap in two tables** — § *The gap is recorded twice* is why a task that closes
    it must touch both:

    - **Row `L3`** in § *Gaps this layout exposes in `ui_core`* gains a dated note
      naming `TASK_UI_PRIM_52`, stating that the **`Grid` clause is closed** — the
      variant's algorithm, that `columns` is read in production, and that a grid
      node reports one rect per child — **and that the `Flex.wrap` clause is not**,
      because `arrange`'s flex arm still discards `wrap`. **Its evidence column is
      rewritten from line numbers to symbols** — the arm, the field, the variant,
      the test — because a line number into `ui/src` is stale the moment the tree
      moves and `AGENTS.md` § *Rust* records that rule for the documents in this
      repository. **The row is not deleted and not marked closed**, and its Blocks
      column keeps *"The Controls tile grid, the app tray grid"*, because neither
      tile grid exists.
    - **Row `#2`** in § *Library gaps* gains the same dated note in the prose its
      row uses, so the two tables do not disagree. **It keeps its numbering**,
      because § *Library gaps* records that four files cite rows 1–8 by number.
    - **Row `L7`** gains one sentence: **the cross-axis-gap clause holds for
      `LayoutMode::Flex`, and `LayoutMode::Grid` applies the one `spacing` value
      to both of its axes, which is not a second gap parameter** — so the row is
      **amended and still open**, and margin, `flex-shrink` and `flex-basis` are
      untouched by this task.
    - **No other row changes, and `doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Widget
      Tree* changes not at all** — its `Grid { columns: usize }` stays correct
      precisely because `columns` stayed a `usize`, which is the observable
      reason for that decision.

11. **`doc/ui/IMPLEMENTATION_STATE.md` gains the record**: a task-table row for
    **52** naming the file, its review count and its waivers-or-none, and its
    frame rate; a task section carrying **the algorithm in one paragraph**, **the
    seven decisions and the reason for each** (equal columns, row-major,
    `columns == 0` as one column, a left-aligned short last row, no alignments, no
    row squeezing, and `columns` staying `usize`); **the `wrap` decision and the
    deleted promise, by name**; **that `flex` is unread inside a grid cell and
    that this is `NEVERAGAIN`'s "one parent mode" entry with the remedy applied
    to the other setter**; **that `spacing` is the grid's gap on both axes and
    that `L7` is amended rather than closed**; **the test count before and
    after**; the frame rate for all six pages; and **the honest limit in the
    section's own register** — *the algorithm is unit-tested over a swept space
    and nothing in the repository constructs a grid, so no tile has been seen laid
    out on screen, and this task's evidence is that the geometry is right rather
    than that it looks right.* `IMPLEMENTATION_STATE.md` is not a source of
    evidence (`task-sequence.md` § *State*); it points at the code. The
    `Current position` section is updated to name task 52.

12. **The suite, the six pages and the frame rate are all produced.** From `ui/`:
    `cargo fmt --check`; `cargo build --all-targets --all-features`;
    `cargo clippy --all-targets --all-features -- -D warnings`;
    `cargo test --all-features` with **the three per-binary counts pasted and every
    test in requirement 9's list present by name**, against a baseline of
    **1894 (1450 + 224 + 220)** measured on this tree, **with no test deleted,
    renamed away or weakened and `layout_walk_cost` still `#[ignore]`d and
    unmodified**; `cargo doc --no-deps` clean; and `cargo audit` **recorded as not
    installed on this host rather than passed**. Then the six-page capture and then
    the frame rate on all six pages, per § *Acceptance Criteria*.

## Acceptance Criteria

- [ ] **`LayoutMode::Grid` no longer appears with `Vec::new()` in `arrange`, and
      `columns` is read in production.** `rg -n 'LayoutMode::Grid' ui/src/ui_core/src/layout.rs`
      returns the variant declaration and the **new arm** and nothing that returns
      an empty `Vec`; the arm is
      `LayoutMode::Grid { columns } => arrange_grid(nodes, children, inner, columns, config),`
      **which is the check that matters**: `columns` is bound and passed, so the
      field is read in production and not only in tests. `rg -n 'columns' ui/src/ui_core/src/layout.rs`
      shows `columns` in `grid_column_width`'s signature, in `grid_row_heights`'s,
      in `div_ceil(columns)` and in the row loop's `row * columns`, and the
      replacement test's name. **The mechanism is stated and true: nothing in the
      repository constructs `LayoutMode::Grid` outside `layout.rs`'s own tests, so
      no node's mode changed and no rect moved — which is what makes the
      pixel-identity criterion below free rather than lucky.**

- [ ] **The property test exists, sweeps, and its tolerance is justified.**
      `every_grid_cell_is_inside_the_grid_and_no_two_cells_overlap` sweeps
      `columns` in `1..=8` and a child count in `0..=columns * 3 + 2` — **216
      (columns, count) pairs, exhaustively and with no seed**, with cell sizes from
      a seeded LCG in `chart.rs`'s inline shape — and asserts one rect per child in
      order, every rect inside the grid's content box to `GRID_EPSILON = 1e-3`, and
      no two interiors intersecting, with touching edges not counted as overlap.
      **The epsilon's arithmetic is in `GRID_EPSILON`'s own doc comment**: a cell's
      far edge is `x + column_width` in one rounding and its neighbour's origin is
      `(column + 1) * column_width` in another, differing by at most a few ulps.
      **Mutation evidence in the handoff:** change `i / columns` to `i % columns`
      and watch it fail; replace `count_to_f32(column) * (column_width + gap)`
      with a cursor and re-run the sweep with `columns == 8` to see whether the
      separation assertion still holds; make `gap` negative and watch it fail. **A
      test that has never failed is a hypothesis** (`developer.md` § Phase 3), and
      **the row-major mutation is the one a reviewer should break first** — the
      other seven column orders all produce one rect per child inside the box, so
      only the exact-rect assertion in
      `grid_places_children_row_major_into_equal_columns` kills the transposed
      one, and that is why that test exists.

- [ ] **The placeholder test is replaced by its opposite, not deleted.**
      `grid_has_no_algorithm_yet` does not exist; `grid_places_children_instead_of
      _reporting_nothing` does, and it makes the same
      `LayoutMode::Grid { columns: 2 }` / `layout_constraints` call and asserts a
      **50 × 100 rect at the origin for the child in column 0 of 2** —
      `assert!(rects.is_empty())` inverted into a size. **The count of `#[test]`
      functions in `layout.rs` is one higher than it was, and the handoff lists
      requirement 9's twenty tests by name** so a reviewer can pair the old name
      with the new one.

- [ ] **`columns == 0` and one child are both pinned.** `zero_columns_lays_the
      _children_out_as_one_column` asserts three full-width stacked rects from
      `LayoutMode::Grid { columns: 0 }`; `one_child_gets_one_column_and_the_rest_of
      _the_row_is_nothing` asserts **one** rect and not three, of a third of the
      box's width. **Mutation evidence:** replace `columns.max(1)` with `columns`
      and watch the first test panic or fail; **and the panic is the point** —
      `usize::div_ceil(0)` is the reason the clamp is a `developer.md` § *Panics*
      requirement and not a tidiness one.

- [ ] **The cross-axis rules are pinned by three tests, not by prose.**
      `a_grid_row_is_as_tall_as_its_tallest_cell_and_every_cell_shares_it` asserts
      one height for all cells in a row of three different declared heights;
      `rows_do_not_share_a_height_with_each_other` asserts the two rows differ and
      the second starts at the first's height plus the gap; and
      **`a_cell_whose_declared_minimum_exceeds_the_column_overflows_its_slot`
      asserts the documented limit** — a cell declaring a 400-pixel minimum in a
      100-pixel column is 400 wide and overlaps its neighbour, because
      `arrange_grid` hands `Constraints::tight(slot)` down and `resolve_box`'s
      existing `clamp_axis` lets a declared minimum beat it. **That limit is in
      `LayoutMode::Grid`'s doc with the same words the test uses**, which is the
      check that no doc comment in a changed file asserts the opposite of the code
      beside it.

- [ ] **`spacing` is the gap on both axes, and `L7` is amended rather than
      closed.** `a_grid_uses_the_configured_spacing_between_columns_and_between_rows`
      asserts the four offsets by value with `column_width = (100 - 8) / 2 = 46`
      and the second column at `54`, and `zero_spacing_makes_a_grid_abut_without
      _overlapping` asserts that `with_spacing(-4.0)` clamps to zero and that the
      cells touch without overlapping. **Row `L7` in `DEMO_APPLICATION.md`
      § *Gaps this layout exposes in `ui_core`* carries the sentence saying the
      cross-axis-gap clause holds for `LayoutMode::Flex`, that `LayoutMode::Grid`
      applies the one value to both axes, and **that this is not a second gap
      parameter** — and **the row is not closed, and its margin / `flex-shrink` /
      `flex-basis` clauses are unchanged.**

- [ ] **`flex` is unread inside a grid cell, and the setter says so.**
      `a_grid_cell_ignores_its_flex_factor` asserts three cells in one row get the
      same width with factors `0.0`, `1.0` and `7.0` and the box wide enough that a
      factor would show. **`LayoutState::flex`'s doc, `with_flex`'s doc and
      `set_flex`'s doc each name the modes that honour the factor** and say that a
      grid cell's is ignored, because a grid has no main axis and its column width
      is decided by `columns` and the box. **This is
      `.ai/NEVERAGAIN.md` § *A position API that only one parent mode reads*'s
      remedy applied to the other setter**, and the mechanism is what makes it
      checkable: a caller who reads the setter's name is told which parent's arm
      consumes it.

- [ ] **`wrap` is still not honoured, and that is asserted rather than documented.**
      `wrap_is_not_honoured_and_a_wrapped_row_stays_on_one_line` builds
      `LayoutMode::Flex { direction: FlexDirection::Row, wrap: true }` with children
      wider than the box and asserts **one line and a clipped overflow**. **And the
      false promise is gone:** `rg -n 'Wrapping arrives' ui/src/ui_core/src/`
      returns **nothing**, because `pub mod list;` and `pub mod scroll;` are both in
      `ui/src/ui_core/src/widgets/mod.rs` and neither wraps — a doc comment promising
      a feature two delivered widgets did not bring. **`LayoutMode::Flex`'s doc now
      says where multi-line placement lives** (`LayoutMode::Grid`, whose row-major
      flow is wrapping with a column count instead of a break rule), and the field's
      *"Not yet honoured"* stays, because it is true. **`arrange`'s flex arm still
      discards `wrap`, so `L3`'s second clause is still literally true and the row
      says so.**

- [ ] **Both copies of the gap are amended, dated, and neither is closed.**
      `DEMO_APPLICATION.md` carries the note in **row `L3`** (saying the `Grid`
      clause is closed and the `wrap` clause is not) **and in row `#2`** (the same
      note in that table's prose), so **the two rows cannot disagree** — which is
      the whole point of amending both, since § *The gap is recorded twice* in the
      task file is the finding. **`L3`'s evidence column names symbols and paths,
      not line numbers** (`arrange`'s arm, `LayoutMode::Grid`'s `columns`, the two
      tests), per `AGENTS.md` § *Rust*. **Neither row is deleted, neither is marked
      closed, and `L3`'s Blocks column keeps *"The Controls tile grid, the app
      tray grid"*, because neither tile grid exists.** And **no other row in either
      table changed.**

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: in `layout.rs` —
      `every_grid_cell_is_inside_the_grid_and_no_two_cells_overlap`,
      `grid_places_children_instead_of_reporting_nothing`,
      `zero_columns_lays_the_children_out_as_one_column`,
      `one_child_gets_one_column_and_the_rest_of_the_row_is_nothing`,
      `an_empty_grid_produces_no_placements`,
      `grid_places_children_row_major_into_equal_columns`,
      `a_grid_row_is_as_tall_as_its_tallest_cell_and_every_cell_shares_it`,
      `rows_do_not_share_a_height_with_each_other`,
      `a_grid_uses_the_configured_spacing_between_columns_and_between_rows`,
      `zero_spacing_makes_a_grid_abut_without_overlapping`,
      `a_grid_ignores_both_alignments_and_stacks_rows_from_the_top`,
      `a_grid_cell_ignores_its_flex_factor`,
      `a_grid_measures_to_the_widest_row_when_the_box_is_unbounded`,
      `a_grid_too_narrow_for_its_columns_gives_zero_width_cells_and_overflows`,
      `a_padded_grid_offsets_every_cell_by_the_leading_edges`,
      `a_padded_grid_is_measured_as_its_content_plus_the_padding`,
      `a_grid_inside_a_grid_cell_lays_out_in_that_cell`,
      `a_stale_grid_child_keeps_the_result_index_aligned`,
      `wrap_is_not_honoured_and_a_wrapped_row_stays_on_one_line`,
      `an_overflowing_grid_row_keeps_its_height_and_is_clipped`,
      `a_cell_whose_declared_minimum_exceeds_the_column_overflows_its_slot` —
      **twenty-one in all**, and the list above is the whole of what requirement 9
      adds; **plus `LayoutMode::Grid`'s new doctest**, which `cargo test` counts in the
      doctest total and which `developer.md` § *API design* requires (*"Doc tests
      count as tests — write them"*). **Against the measured baseline of 1894**
      (1450 `ui_core` + 224 `ui_demo` + 220 doctests): the three counts are
      pasted, each is **higher than the baseline by the number of tests added in
      it**, and **no test was deleted, renamed away or weakened except
      `grid_has_no_algorithm_yet`, which requirement 4 replaces with its opposite.**
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. **And `layout_walk_cost` is still `#[ignore]`d
      and unmodified** — `git diff` over `layout.rs` shows no change to it, because
      its numbers in `IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why*
      are reproducible only while it is. `cargo audit` is **recorded as not
      installed on this host, not passed.**

- [ ] **The six gallery pages are pixel-identical, and the mechanism is the
      criterion rather than the result.** `Page::ALL`'s six names, release build,
      captured **before and after** with the commands of `IMPLEMENTATION_STATE.md`
      § *Verifying a change that draws — the capture method* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture, and `ffmpeg x11grab`, return black for a GL window), `pgrep -a -x
      ui_demo` in the same call as each `magick import -window <id>`, then
      `magick compare -metric AE before.png after.png null:` per page.

      - **The criterion on all six pages is AE 0 outside `y ≥ 680`**, every
      differing pixel inside the fps readout's band, which is the criterion tasks
        34 to 39 inherited and what `IMPLEMENTATION_STATE.md` § *Task 24.1 — what
        it decided, and what it found* records as the one thing two captures of an
        unchanged frame differ in.
      - **The mechanism is four facts, and saying so is the criterion:**
        **no node in either binary constructs `LayoutMode::Grid`** — the grep is in
        the first item above and its result is part of this claim; **so no node's
        `LayoutMode` changed and no rect moved**; **`arrange_flex`,
        `arrange_stack` and `arrange_absolute` are byte-for-byte unchanged**, so
        every existing placement is computed by the same code as before; and the
        only new allocation on any frame this demo takes is none, because the new
        function is never called. **A change that cannot move a pixel is
        demonstrated not to, not asserted not to** — which is why the grep is an
        acceptance criterion rather than a remark.
      - **The rect-level half keeps its name and every one of its assertions:**
        `every_page_places_every_rect_where_the_gallery_placed_it`,
        `no_two_placed_rects_overlap`, `placed_handles`,
        `expected_placed_rect_names` and `assert_placed_handles_is_complete`. **No
        row of any of them is added or loosened.**

- [ ] **The frame rate is measured on every page and reported with the script's own
      line pasted.** `.ai/tools/fps-check.sh 10 55` on the default page, **which
      is the only thing the script can do** — it takes `seconds` then `floor`,
      builds release and runs `./target/release/ui_demo` with **no arguments and no
      page**, per `IMPLEMENTATION_STATE.md` § *Current position* recording as the
      reason task 24.2's criterion 6 was amended rather than met by the script —
      and then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for
      each of the six, with the `roados-fps` line parsed by hand. **Every page
      above the floor of 55**, and **every page expected to land inside the
      recorded 61.1–63.9 band in `IMPLEMENTATION_STATE.md` § *The frame rate,
      measured* and to have cost nothing** — with the reason stated rather than
      left as a coincidence: **the new function is never called on any page, so no
      frame does anything this task added.** This is the gate in
      `task-sequence.md` § *Gates* (*"No unmeasured run of the demo"*) and no
      other check here can see a frame-cost regression, because **a still of a 4 fps
      application is pixel-identical to a still of a 60 fps one** — which is how a
      four-fps regression survived three reviews in this repository.

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **no change** to `ui/src/ui_core/src/render.rs`,
      `render/`, `paint.rs`, `batch.rs`, `node.rs`, `arena.rs`, `property.rs`,
      `animation.rs`, `input.rs`, `theme.rs`, `widgets/label.rs`,
      `widgets/container.rs`'s **code** — **its only diff is the one doc sentence
      requirement 5 names** — or `ui/src/ui_demo/src/main.rs`. **`ui/Cargo.toml` and
      `ui/Cargo.lock` are unchanged**: the approved direct dependencies remain
      `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, and a grid is arithmetic
      over four `f32`s, which is not a licence decision against GPLv3. **No `unsafe`
      is added** (`rg -c unsafe ui/src/ui_core/src/layout.rs` unchanged),
      **no `unwrap`, no `expect`, no `panic!`, no `unimplemented!`, no `todo!`**, and
      **the only `usize`-to-`f32` conversion in `arrange_grid` is through
      `count_to_f32`** — which is what that helper's own doc says it is for. **No
      `pub` item is added and no existing `pub` item's signature changes**, so
      there is no semver question to raise; **and
      `doc/ui/PRIMITIVES_ARCHITECTURE.md` is untouched, its `Grid { columns: usize }`
      staying correct because `columns` stayed a `usize`.**

- [ ] **What the handoff does not claim, in those words.** It states that **no
      grid appears on screen**: no node in either binary constructs one, so **no
      tile has been laid out by this algorithm in a frame anyone looked at**, and
      the evidence here is **that the geometry is right over a swept space**, not
      that it looks right. **`developer.md` § Phase 3 requires a rendering change
      to be seen on screen before it is reported as done, and this task does not
      change what is on screen** — which is a claim the first criterion makes
      checkable rather than a thing to apologise for. It also states that **the
      demo's Controls tile grid and app tray grid are not built**, that
      `Container::new(nodes, LayoutMode::Grid { columns: n })` is how a caller
      reaches the mode, and that **building those grids is `TASK_UI_DEMO_n`** with
      `L3`'s Blocks column naming them. And it states the one place a reader is
      most likely to be misled: **a cell may be larger than its slot** if it
      declares a minimum that exceeds the column or the row, **which is `clamp_axis`
      and not a grid defect**, and which two doc comments and one test now say so.

## Out of Scope

- **No margin, no `flex-shrink`, no `flex-basis`, and no second gap parameter —
  gap `L7`, a separate task, and not this one.** All four hold after this change
  except in the one direction § *What a grid takes from `FlexConfig`* states, and
  that direction is **not any of the four**: `LayoutMode::Grid` applies the single
  `FlexConfig::spacing` value to both of its axes, which is **one parameter used
  twice and not a `row-gap` / `column-gap` pair**. **No `margin` field is added to
  `LayoutState`, no `flex_shrink` field to it, no `flex_basis`, and no
  `GridConfig` type.** A grid has no distribution step, so there is nothing for a
  shrink factor to share and nothing for a basis to be the base of — and § *The
  test suite* names the overflow that results from having neither. **This task
  does not depend on `L7` being closed and does not solve any part of it beyond
  that one sentence.**
- **No flex `wrap`, in flex or in grid, and no wrapping anywhere.** Decided in §
  *`Flex.wrap`* with five reasons, the load-bearing one being that **a grid is the
  wrapped layout with a column count in place of a break rule**, so this task
  ships the placement and flex wrap is the policy. **A grid's row-major flow is
  not wrapping** and must not be described as such. `LayoutMode::Flex { wrap }`
  stays a declared, unhonoured, documented field; `wrap_is_not_honoured_and_a_
  wrapped_row_stays_on_one_line` holds it there. **Honouring it — line breaking,
  a per-line cross size, and per-line `MainAxisAlignment` — is its own task with
  its own boundary cases** (a child wider than the box, an exactly-divisible
  count, whether the gap applies at a break), and `arrange_flex`'s steps 2 through
  4 are a rewrite rather than a flag.
- **No auto-fill, no `Option<usize>`, and no minimum-column-width constant.**
  `columns` stays `usize` for the four reasons in § *`columns` stays `usize`*, of
  which the decisive one is that **auto-fill makes a grid's column count a function
  of its box, and `content_size` measures under `Constraints::UNBOUNDED` for the
  stated reason that a node's size must be a property of its children.** A caller
  that wants auto-fill computes the count and writes
  `LayoutMode::Grid { columns: computed }`; no crate change is needed. **Revisit
  when a measured minimum tile width exists in this repository** — a constant with
  no source is the failure `DEMO_APPLICATION.md` § *Asset requirements* records for
  values nobody could verify.
- **No row or column spans, no explicit row or column index, and no
  track-based sizing.** `Grid { columns }` gains no second field and no placement
  field, so a child cannot say "span two columns" or "start at row 3". **An empty
  cell is why**: `struct Placement` carries `handle: Handle` and not
  `Option<Handle>`, so a span would need a node for the covered columns and an
  empty cell would need a handle that means nothing. **A cell's rect is exactly
  its slot** and a leftover column in a short last row holds nothing — § *The
  algorithm* decision 4.
- **No alignment in a grid, and no content-sized or weighted columns.** Neither
  `MainAxisAlignment` nor `CrossAxisAlignment` is read by `arrange_grid`, and there
  is no `1fr`, no `auto` track and no per-column weight. `a_grid_ignores_both
  _alignments_and_stacks_rows_from_the_top` pins the first; the second is decision
  1 and `grid_places_children_row_major_into_equal_columns` pins it.
- **No change to any other mode, and none to the flex path.**
  `arrange_flex`, `arrange_stack`, `arrange_absolute`, `distribute`,
  `alignment_spacing`, `cross_offset`, `direction_cross_to_xy`, `bounding_box`,
  `content_size`, `resolve_box`, `sized`, `intersect`, `clamp_axis` and
  `count_to_f32` are **byte-for-byte unchanged**, and so are `LayoutMode::Stack`,
  `LayoutMode::Absolute` and `LayoutMode::Flex`'s `direction`. **The two padding
  steps stay outside the match in `arrange`**, which is why a grid gets padding
  for free and two tests pin that it does. **`struct FlexItem` is not reused** — a
  grid has no main or cross axis and builds no `FlexItem` — and **`FlexConfig`
  gains no field and changes no signature.**
- **No demo page, no tile, and no Controls or app-tray grid.** `L3`'s Blocks
  column names *"The Controls tile grid, the app tray grid"* and **neither is
  delivered**; both are `TASK_UI_DEMO_n`, and the way in is
  `Container::new(nodes, LayoutMode::Grid { columns: n })`, which exists today. **No
  page is added, `Page::ALL` is unchanged, no `--tab=` name is added, and
  `ui/src/ui_demo/src/main.rs` has no diff** — which is the other half of the
  pixel-identity criterion.
- **No scroll-container integration, no grid inside `List`, no virtualisation.**
  `List` scrolls vertically and lays its items out itself; a grid inside it is a
  later task and its `row_gap` question is a `Scroll` one. **No item recycling, no
  windowing, and no partial-grid measure.** A grid's whole box is laid out in one
  pass, which is what `Layout::visit`'s dirty-subtree skip already provides.
- **No nested-grid semantics beyond what falls out.** A cell is a box, and a grid
  in a box lays its children out in it — that is arithmetic, not a feature, and
  `a_grid_inside_a_grid_cell_lays_out_in_that_cell` pins the coordinate space once
  because a nested-container bug is invisible to every other test in the module.
  **There is no row-alignment, no column-alignment and no subgrid.**
- **No new dependency and no `unsafe`.** Per `AGENTS.md` the approved direct
  dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; **a grid is
  arithmetic over `f32` and `usize` and needs nothing**, so `ui/Cargo.toml` and
  `ui/Cargo.lock` are untouched. **Edition 2021 and `rust-version = "1.85"` are
  respected** — `usize::div_ceil` is stable since 1.73 and nothing newer is used.
  **Zero new `unsafe` blocks**, because there is no GL, no FFI and no pointer in
  this change.
- **Found in the tree and deliberately not fixed:**
  `IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why* writes
  `LayoutMode`'s variants as `Flex { direction, wrap, flex_config }`, and
  **`flex_config` is a `LayoutState` field, not a `LayoutMode` field**. **It is
  recorded here rather than fixed**, because that section is the record of what
  task 24 decided and rewriting a past section's text is the drive-by cleanup
  `developer.md` § *Phase 2* refuses (*"Do not restructure what you were not asked
  to touch"*), and because the same file is where this task's own record goes and a
  second edit to a neighbouring paragraph would make the diff unreviewable.
  **Equally not fixed:** the historical sentences in
  `TASK_UI_PRIM_24.md` § *Context*, `TASK_UI_PRIM_40.md` § *Context* and
  `TASK_UI_PRIM_40.md` § *Out of Scope* that say a grid lays out nothing. **They
  were true when written and they are the record of what those tasks decided**;
  the amendment surface for a *gap claim* is the gap table, and the two are
  different artefacts.