# TASK_UI_PRIM_48: Margin, `flex-shrink`, and the cross-axis gap — closing gap `L7`

## Goal

Close **row `L7`** of `DEMO_APPLICATION.md` § *Gaps this layout exposes in
`ui_core`*, which reads *"**No margin, no `flex-shrink`, no `flex-basis`, no
cross-axis gap**"* and whose **Blocks** column names *"Dense settings rows that
must not overflow"*.

Three of those four are delivered and one is declined with an argument, and this
file says which is which before any code is written:

| clause | outcome | mechanism |
|---|---|---|
| **margin** | **added** | a `Margin` type beside `Padding`, on `LayoutState`, honoured by `LayoutMode::Flex`, `LayoutMode::Stack` and `LayoutMode::Absolute` |
| **`flex-shrink`** | **added** | a `shrink` factor on `LayoutState`, default `1.0`, weighted by the child's own size, clamped by its declared bounds |
| **`flex-basis`** | **declined, by argument** | `Constraints` is already the basis, and a growing child's basis is already zero — **there is nothing left to add** |
| **cross-axis gap** | **narrowed, by decision** | one `spacing`, re-documented as *"the gap between adjacent children"*; **no second parameter**, because a second one is unreachable while `Flex.wrap` is unhonoured |

## Context

### The row, quoted, and what each of its four clauses is actually claiming

Row **`L7`** in `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in
`ui_core`* reads, verbatim:

> **No margin, no `flex-shrink`, no `flex-basis`, no cross-axis gap.** All four
> hold, but **`Padding` does exist** and the previous row omitted it — a
> four-sided inset applied in `arrange` to every mode. A reader counting layout
> vocabulary from this row alone would under-count by the feature the demo uses
> most.

Severity **Medium**; Blocks *"Dense settings rows that must not overflow"*; and
§ *Task structure* places **`L7`** in the **"Low / Medium"** bucket beside `#6`,
`#7`, `L8`, `L9` and `L10` — **not** in the "High" bucket with `#2`/`L3`, `#3`,
`#4`, `#5`, `L4`, `L5` and `L6b`. **So this is a vocabulary task, not a blocker
task, and it is written at that size.** Nothing else in `DEMO_APPLICATION.md`
records this gap; **§ *Library gaps* has no row for it**, and its eight rows keep
their own numbering because four files cite them.

Two things about the row are defects in their own right and requirement 15 fixes
both:

- **Its evidence column cites `ui/src` by line number** — *"`Padding` at
  `layout.rs:579-588`, applied `:1198`, `:1212-1217`; `spacing` is main-axis
  only, `layout.rs:277`"* — which `AGENTS.md` § *Rust* forbids and which
  `TASK_UI_PRIM_52`'s requirement 10 already ruled for row `L3`:
  *"rewritten from line numbers to symbols … because a line number into `ui/src`
  is stale the moment the tree moves."*
- **Its second sentence is a correction of an earlier row and stays true.** `Padding`
  does exist, is a four-sided inset, and is applied by `arrange` to every mode.
  Requirement 15 **keeps that sentence**; what changes is that this task adds the
  three things the row says are missing, so the sentence's *"All four hold"*
  becomes false and the row's note has to say which clause became what.

### What exists at the moment of writing, established by reading the source

Every item was read out of the tree, and each is cited **by symbol and path**,
never by line number, because the tree is being modified in parallel.

- **`rg -c 'margin|flex_shrink|flex_basis|row_gap|column_gap|cross_axis_gap'`
  over `ui/src/ui_core/src/layout.rs` returns exactly one line, and it is a
  comment**: *"`Padding` is the margin …"*, inside the comment explaining the
  padding test. **There is no `margin` field, no `flex_shrink` field, no
  `flex_basis` field and no second gap parameter anywhere in `layout.rs`, and
  none in any other file in `ui/src`.**
- **`struct Padding`** in `layout.rs` is `pub` with four `pub f32` fields —
  `left`, `right`, `top`, `bottom` — and derives
  `#[derive(Clone, Copy, Debug, Default, PartialEq)]`. `impl Padding` carries
  `pub const ZERO`, `pub fn all(f32)` `#[must_use]`,
  `pub fn horizontal(self) -> f32` `#[must_use]`,
  `pub fn vertical(self) -> f32` `#[must_use]`,
  `pub fn inset(&self, Constraints) -> Constraints` `#[must_use]`, and a
  **private** `fn clamped(&self) -> Padding` that floors every side at zero.
- **`struct LayoutState`** carries `mode`, `flex_config`, `constraints`, `flex`,
  `position`, `padding`, `rect`, `clip`, `dirty`, `placed_under`, `visible`.
  **There is no `margin` field and no `shrink` field**, and every one of its
  eleven fields has a `with_` builder and a `set_` method except the three cache
  fields and `dirty` — so the pattern this task follows is set.
- **`struct FlexItem`** is **private** and is exactly
  `{ handle, flex: f32, declared: Constraints, desired: Size, main: f32,
  cross: f32, tight_cross: bool }`. **`flex` is grow-only**: `with_flex`'s own doc
  says *"A factor of zero — the default — means the node is rigid: the parent
  gives it the size it asks for."*
- **`struct Placement`** is **private** and is exactly
  `{ handle: Handle, rect: Rect, constraints: Constraints }`. **`pub struct`
  `FlexConfig`** is `Clone, Copy, Debug, Default, PartialEq` with three
  **private** fields — `main_axis_alignment`, `cross_axis_alignment`,
  `spacing: f32` — and three `#[must_use]` getters, so a caller writes it only
  through `with_main_axis_alignment` / `with_cross_axis_alignment` /
  `with_spacing`. **`with_spacing` clamps a negative to zero** and its doc says
  why: *"overlapping children are what `LayoutMode::Stack` is for."* **`spacing`'s
  doc says *"the gap between children on the main axis"*, and that is the only
  gap the crate has.**
- **`fn arrange(nodes, children, constraints, mode, config, padding)`** applies
  `Padding` in **two steps that sit outside the mode's match**: `padding.inset(
  constraints)` produces `inner`, the box the mode's arm is given; and after the
  match, `if padding != Padding::ZERO` every `placement.rect.origin` is offset by
  `(padding.left, padding.top)`. **`fn content_size`** then adds
  `padding.right` and `padding.bottom` to the extent `fn bounding_box` measured —
  **the far side only**, because the placements are already offset by the near
  side. **`Padding` is not margin**: it shrinks the box a mode arranges in and it
  moves every placement out to where that box starts, and **it never pushes a
  sibling**, because no `padding` is read by any mode's arm.
- **`fn arrange_flex(nodes, children, constraints, direction, config)`** runs
  four numbered steps: (1) build one `FlexItem` per child with
  `desired = sized(nodes, handle, constraints.loosen())` and `flex = node.layout()
  .flex()`, and `Constraints::UNBOUNDED` / `Size::ZERO` for a handle the arena no
  longer holds so the result stays index-aligned; (2) the main axis, where
  `flex > 0.0` sets `item.main = 0.0` and everything else sets
  `item.main = item.desired.main(direction)` and adds it to `rigid_total`, then
  `gaps = spacing * count_to_f32(children.len().saturating_sub(1))` and
  `available_main = max_main` when finite else `rigid_total + gaps`, and
  `fn distribute(&mut items, (available_main - rigid_total - gaps).max(0.0),
  direction)`; (3) the cross axis, where `item.tight_cross = max_cross.is_finite()
  && (item.flex > 0.0 || config.cross_axis_alignment() == Stretch)` and
  `item.cross = clamp_axis(max_cross, min_declared, max_declared)` when tight and
  `item.desired.cross(direction)` otherwise; (4) positions, where
  `alignment_spacing` turns the leftover into a start offset and a gap,
  `cross_offset` turns the cross alignment into an offset,
  `direction_cross_to_xy` maps the pair to `(x, y)`, `Size::from_main_cross`
  builds the size, the child's constraints are `with_main(flex > 0.0 ? (main,
  main) : (0.0, available_main))` and `with_cross(tight_cross ? (cross, cross) :
  (0.0, max_cross))`, and **`cursor += item.main + gap`**.
- **`fn distribute(items, budget, direction)`** is the crate's **only** thing
  that shares free space between children. Its doc's four sentences are the ones
  this task's `fn shrink` mirrors: an item that hits a bound is fixed there and
  its share returns to the pool; **every round either fixes at least one more
  item or spends the budget, so the loop runs at most once per item**; the
  fraction is taken **before** the multiply — *"so two factors near the `f32`
  ceiling overflow to `inf` before the division brings them back down, and the
  child was given an infinite width"* — because `item.flex / total <= 1.0` by
  construction; and **a budget no item's bounds fit into is left unallocated**
  rather than forced on somebody.
- **`fn resolve_box(nodes, node, incoming)`** does
  `incoming.tighten(node.layout().constraints())`, returns it if
  `effective.is_tight()`, and otherwise
  `Constraints::tight(effective.constrain(content_size(nodes, node)))`. **This is
  the "minimum wins" rule's home**: `fn clamp_axis(value, min, max)` returns
  `min` whenever `min > max`, with the doc's reason — *"the minimum is the bound
  a caller can still act on, whereas honouring a lower maximum would silently
  push a child below the size it declared"* — and passes a `NaN` through.
- **`LayoutMode`** has four variants — `Flex { direction, wrap }`,
  `Grid { columns }`, `Absolute`, `Stack` — with
  `#[derive(Clone, Copy, Debug, Default, PartialEq)]` and `Stack` as
  `#[default]`. **`LayoutMode::Grid { .. } => Vec::new()` is the arm in
  `arrange`, and `columns` is read nowhere in production** — task
  **`TASK_UI_PRIM_52`** closes that, and § *Working beside `TASK_UI_PRIM_52`*
  below is about not colliding with it.
- **`Layout::visit`** computes `children_clip = intersect(clip, Some(rect))` once
  per node and hands it to every child, and `an_overflowing_child_keeps_its_size
  _and_computes_a_clip_rect` pins the result. **The clip already exists**, which
  is what makes this task's overflow policy (§ *Overflow*) a policy rather than a
  new mechanism.
- **`every_segment_quad_is_convex_over_two_hundred_thousand_geometries`** in
  `ui/src/ui_core/src/widgets/chart.rs` is the house precedent: a property
  asserted over a large swept space, **the count in the test's own name**, built
  on an inline seeded LCG
  (`seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(
  1_442_695_040_888_963_407)`, `((seed >> 33) as f32 / 2_147_483_648.0) - 1.0`).
  Requirement 14's two property tests are that shape.
- **`layout_walk_cost`** in `layout.rs` is the suite's **one** `#[ignore]`d test,
  a `use std::time::Instant` harness that reads the wall clock. **This task does
  not touch it**, and acceptance criterion 12 says so by name, because its
  numbers in `IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why* are
  reproducible only while the function it measures is unmodified.
- **`.ai/tools/fps-check.sh` takes `seconds` then `floor`, builds release, runs
  `./target/release/ui_demo` with no arguments and no page**, and exits 1 both on
  a missed floor **and on a run that printed no `roados-fps` line at all** —
  *"a missing report line is an aborted measurement, not a fast one."* **It
  cannot name a page**, so the six pages are measured with
  `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>` and the
  `roados-fps` line parsed by hand.
- **The test baseline, measured on this tree, is 1894** — `cargo test
  --all-features` from `ui/` reads **1450** in `ui_core` (**1** ignored, which is
  `layout_walk_cost`), **224** in `ui_demo` and **220** doctests. Every test this
  task adds is additive, and the number is quoted here so that "green" means a
  count.

### What `PRIMITIVES_ARCHITECTURE.md` says, and the answer to "conform or amend"

**`doc/ui/PRIMITIVES_ARCHITECTURE.md` has no § *Layout*.** Its section list is
*Core Design Decisions*, *Dependencies*, *Widget Tree* (with *Arena allocation*,
*Node structure*, *Composition*, *Dynamic lists*), *Property System*, *Theming*,
*Transparency and Compositing*, *Animation System*, *Rendering Pipeline*, *Input
Handling*, *Memory Management*, *Thread Model*, *Module Layout*, *Open
Questions*. **The brief for this task anticipated a § *Layout* that specifies
the intended vocabulary; there is none, and that is a fact about the document
rather than an omission in it.**

Three places in it bear on this task, and the answer is the same for all three:
**this task conforms, and amends nothing.**

1. **§ *Widget Tree* → *Composition*** carries the `LayoutMode` enum in a code
   block as `Flex { direction: FlexDirection, wrap: bool }`, `Grid { columns:
   usize }`, `Absolute`, `Stack`. **This task adds no field to `LayoutMode`.**
   `margin` and `shrink` are `LayoutState` fields, not `LayoutMode` fields, for
   the reasons § *Margin* gives; so the block stays correct, and
   **acceptance criterion 14 makes "the document is untouched" the checkable
   form of that claim**, in the same move as task 52's.
2. **§ *Module Layout*** lists `layout.rs` as *"LayoutMode, layout algorithm"* —
   still true after three new types in it, and the line is not edited.
3. **§ *Theming* → *Theme tokens*** carries the spacing scale
   `SpacingXs, SpacingSm, SpacingMd, SpacingLg, SpacingXl`, which
   `ui/src/ui_core/src/theme.rs` holds at **4, 8, 16, 24 and 32 pixels in both
   themes**. **This is where a caller gets a margin's *value*, and it is why
   `Margin` holds four `f32`s and does not take a `ThemeToken`.** Naming it here
   is what stops a later agent from "improving" `Margin::all` into a token.

### `L7` and its neighbours: what this task does to each clause

**The row is amended, dated and attributed, and it is not deleted and not marked
closed** — requirement 15, in the shape `DEMO_APPLICATION.md`
§ *Corrections to the second gap table* prescribes. Which clause became what is
stated there and repeated here so no reader has to infer it:

- **margin: closed.** A `Margin` exists, `Padding` still exists, and the two
  compose by a stated order (§ *Margin*).
- **`flex-shrink`: closed.** A factor exists, defaults to `1.0`, is weighted by
  the child's own size and clamped by its declared bounds.
- **`flex-basis`: still absent, deliberately.** The clause remains **literally
  true** — there is no `flex_basis` field — and the amendment says *why* in three
  sentences and points at the three tests that make the argument checkable.
  **So the row cannot be marked closed**, exactly as `TASK_UI_PRIM_52` left
  `L3`'s second clause open while the first closed.
- **cross-axis gap: narrowed.** One `spacing` value remains, documented as the
  gap between adjacent children rather than a main-axis-only value; **a second,
  independent gap parameter is still absent**, and the amendment names the
  `Flex.wrap` task as its owner.

### `flex-basis`: closed by argument, and there is nothing left to add

**Decision: no `flex_basis` field is added. The gap item is closed by argument,
and the argument is that the crate already has the two things a basis would
provide.**

1. **`Constraints` is already the basis, and it carries two things a basis does
   not.** `LayoutState::with_constraints`'s doc says the declared constraints are
   *"both the size the node asks for and the bound it will not be squeezed
   below."** That is a basis, a floor and a ceiling in one `pub struct` with four
   named fields and four `with_main`/`with_cross` accessors on `direction`. A
   `flex_basis: f32` would be a **fifth** way to say a main extent, and
   `resolve_box` is the one place that reconciles the ones that exist — so a
   basis would be a field whose interaction with the other four lives in a
   function three call sites away.
2. **The one thing a basis buys over `Constraints` — a base of zero that grows —
   is already implemented, by `flex`.** CSS's `flex-basis: 0%` exists so that a
   `flex-grow` item can absorb free space without an intrinsic size.
   **`arrange_flex`'s step 2 already does exactly that**: `if item.flex > 0.0 {
   item.main = 0.0 }`. **The base of a growing child in this crate is zero, today,
   and it is zero because `flex` said so.** § *`flex-shrink`*'s algorithm
   depends on that fact — a growing item is excluded from the shrink pass because
   it has no base to take anything from — so the two decisions share one
   sentence of arithmetic and neither duplicates the other.
3. **The shrink base is `desired`** (§ *`flex-shrink`*, decision 2), and
   `desired` is `sized(nodes, handle, constraints.loosen())` — **the declared
   constraints, resolved.** That is what a basis resolves to. So the *resolved*
   basis exists (`desired`), the *declared* basis exists (`Constraints`), and a
   separate scalar would be a third copy of the first with a `pub` field.
4. **The two features a basis exists for are both out of scope, and each would
   change the field's type.** `flex-basis: 100px` versus `auto` needs an
   `Option<f32>`; `flex-basis: 50%` needs percentage resolution, and
   **`Constraints` has no percentage form at all** — `f32::INFINITY` on a maximum
   means unbounded and nothing in the crate means "a fraction of the parent".
   § *Out of Scope* names both. **A field whose only remaining use needs an
   `Option` and a length type the crate does not have is a `LayoutMode::Grid {
   columns }`-shaped promise**, which is the shape this sequence exists to end.
5. **Nothing in this repository wants it.** `developer.md` § *Phase 2* refuses an
   abstraction before the second use, and this is not even a first use:
   **zero call sites in `ui/src` would set a basis.** `LayoutMode::Grid {
   columns: usize }` stayed a `usize` by the identical argument in task 52, and
   that task's acceptance criterion is what this one mirrors.

**What replaces the field is a doc sentence and three tests**, and this is the
cheap close rather than a dodge: `LayoutState::with_constraints`'s doc gains the
sentence naming it as the basis; `LayoutState::with_flex`'s doc gains the
sentence naming `flex` as the zero-basis case; and `shrink_weights_by_the_
declared_size`, `a_growing_child_does_not_shrink` and `the_declared_constraints
_are_the_flex_basis` assert the behaviour the argument describes. **A reader who
disagrees can break any of the three and see the claim fail.**

### `Margin`: a second type, on the child, and exactly how it composes with `Padding`

**Decision: `pub struct Margin { pub left, pub right, pub top, pub bottom: f32 }`,
beside `Padding`; a `margin` field on `LayoutState`; the leading margins offset
the child's rect origin and the trailing margins consume space; margins **do not
collapse**; and the composition with `Padding` is a fixed total order, stated
below in one line.**

**Why a second type rather than reusing `Padding`.** Three reasons, and the
second is the one that settles it.

1. **They are not interchangeable, and the compiler is the cheapest thing that
   can say so.** A padding's near edge is **subtracted from a box**; a margin's
   near edge is **added to a position**. A caller who writes
   `with_margin(card.padding())` — which the type system permits if they are one
   type — gets a silently wrong answer in **every** mode, and there is no generic
   parameter in this crate that could distinguish the two directions. That is the
   failure `DEMO_APPLICATION.md` § *Corrections to the second gap table* records
   twice: two doc comments, or two values, asserting the opposite of the code
   beside them.
2. **`Padding::inset` has no `Margin` counterpart, and its absence is the whole
   difference.** Padding's one operation is to shrink a `Constraints`; margin's
   are to shift an `Offset` and to add to a sum. Mirroring `ZERO`, `all`,
   `horizontal`, `vertical` and the private `clamped` is **five small functions
   and four fields**, and duplicating them is cheaper than an alias that would
   give the two the same name in every signature and in every error message.
3. **The crate's own shape is the precedent.** `Padding` is four public `f32`s
   with a `ZERO`, an `all`, two axis sums and a private clamp. `Margin` is that
   same shape, so a reader who knows `Padding::all(16.0)` knows
   `Margin::all(16.0)`, and **the diff is recognisable as the file's own idiom
   rather than as a new one**. `Margin` adds **no `inset`**, and requirement 1
   says so in its doc.

**Where it sits: on the child, in `LayoutState`, read by the parent into
`FlexItem.margin`.** Margin is a relationship between a node and its **siblings**,
so the parent has to be able to read it — and the parent's only handle on a
child's layout inputs is `node.layout()`. That is why `flex`, `position` and
`constraints` are there and why `padding` is too: **the pass is what applies
layout inputs, so an input the pass does not read does not exist.** The
alternative — a field on `Container` — is exactly what `Padding`'s doc says
would be wrong: *"a field only the container read would leave every other node —
every panel, every row, a stacked child — placing its children at the unpadded
origin."*

**How the two compose — one line, and it is a total order:**

```
child.rect.origin = parent.origin
                  + (padding.left, padding.top)      // parent padding, `arrange`
                  + (margin.left, margin.top)        // child margin, the mode's arm
                  + the mode's own cursor            // flex cursor / absolute position
```

**They never read each other, and that is the design rather than a
coincidence.** `padding.inset(constraints)` runs **before** the mode's match and
takes room **off the outside** of the box the arm is handed; the margin runs
**inside** the arm and pushes from **within** that box. Three consequences, each
of which is a test:

1. **Padding is already applied when the arm sees the box, so a child's margin
   never fights its parent's padding.** A container with `Padding::all(12.0)` and
   a child with `Margin::all(8.0)` puts the child at `(20.0, 20.0)` in the
   parent's coordinates — the two **add**, they do not compose as a difference
   and there is no rule that could make them.
2. **A child's own `Padding` insets *its* children and is untouched by its
   `Margin`.** A card with `Margin::all(8.0)` and `Padding::all(12.0)` measures
   itself 20 pixels taller than its content (its own padding, plus the 12) and
   its parent puts it 8 pixels further away. **Padding is what a node leaves
   between its own bounds and its children; margin is what its siblings leave
   between it and them.**
3. **The far side needs one thing the near side does not, and `bounding_box` is
   where it lives.** A leading margin is inside some placement's `rect.origin`, so
   `bounding_box` already sees it. **A trailing margin on the last child is in
   no placement's rect at all**, so `fn content_size` would measure a container
   whose only child has `Margin::all(8.0)` **eight pixels too narrow**, the child
   would overflow it, and `Layout::visit`'s clip would take eight pixels off the
   child's far edge — **a silent visible defect with a green suite.**
   `struct Placement` therefore gains **one private field, `trailing: Size`**, and
   `bounding_box` adds it. Requirement 8.

**Margins do not collapse, and § *Out of Scope* says why.** Two adjacent
8-pixel margins give a **16**-pixel gap. Three reasons:

1. **Collapsing is defined for block flow and this crate has no block flow.**
   Collapsing is about *adjoining vertical margins of block-level boxes in normal
   flow* — it is a rule about a container's content edge and its neighbours'
   content edges, with self-collapsing blocks and clearance as its two special
   cases. This crate has two named axes, one line, and a four-variant
   `LayoutMode` enum. **None of collapsing's preconditions is expressible here.**
2. **CSS flexbox does not collapse, and flexbox is the model.** `FlexConfig`,
   `MainAxisAlignment`, `CrossAxisAlignment`, `FlexDirection`, a grow factor and
   a gap are flexbox's vocabulary, and this crate's flexbox deliberately follows
   flexbox rather than block flow. **Following flexbox on the gap and block flow
   on the margin would be the odd one out.**
3. **Collapsing makes a child's rect depend on its siblings' declared values.**
   `content_size`'s doc states the property this crate holds: *"the content is
   measured unbounded so that a node's size stays a property of its children
   rather than of how it arranges them."* Collapsing replaces
   `leading_i + trailing_{i-1}` with `max(leading_i, trailing_{i-1})`, which is a
   function of the **neighbours**, so a node's measured size would stop being a
   property of its own children. **That is one sentence of arithmetic against a
   property the file already commits to.**

**Which modes read it, and which do not.**

- **`LayoutMode::Flex`: fully.** Leading margins offset the child's main cursor
  and its cross offset; trailing margins are consumed by the row's
  `rigid_total` and by the cross total.
- **`LayoutMode::Absolute`: leading margins only**, added to the `position` the
  child declares. That is CSS's rule for an absolutely positioned box and it is
  the one place a leading margin is unambiguously meaningful with no siblings in
  play. A trailing margin consumes nothing because nothing is packed.
- **`LayoutMode::Stack`: leading margins only**, and this is the fix for a real
  gap: **`LayoutMode::Stack` places every child at `Offset::ZERO`, so today there
  is no way to inset one stacked child from its container's origin except a
  wrapper node with a position.** That is the same condition
  `.ai/NEVERAGAIN.md` § *A position API that only one parent mode reads* records
  for `set_position`, and this task closes it for margins.
- **`LayoutMode::Grid`: not read**, if `arrange_grid` exists when this task
  lands — and requirement 9 makes the condition explicit rather than assuming an
  ordering. The reason is task 52's: **a grid cell's rect *is* its slot**,
  `arrange_grid` hands `Constraints::tight(slot)` down, and insetting a slot by
  margins would make a cell smaller than the slot, which is the decision task 52
  made and documented. **This is the same shape as `a_grid_cell_ignores_its_
  flex_factor` and gets the same remedy**: the field's own doc names the modes,
  and `a_margin_is_not_read_by_a_grid_cell` holds it there.

**Negative margins are refused**, clamped to zero by a private
`fn clamped(&self) -> Margin` applied in `with_margin` and `set_margin`, for
`Padding::clamped`'s own stated reason: *"A negative side would otherwise pull a
child out of its parent's box, and there is no arrangement that means it."* **The
default is `Margin::ZERO`, so every node laid out before this field existed is
arithmetically unchanged** — the same guarantee `with_padding`'s doc already
makes.

### `flex-shrink`: default `1.0`, base size `desired`, weighted, and clamped

Six decisions, each argued.

1. **The factor is `LayoutState::shrink: f32`, default `1.0`, clamped to
   `max(0.0)`.** `1.0` is CSS's `flex-shrink` initial value, so a reader porting
   a specification gets the number they expect. **The stronger reason is the
   opposite default being inert**: `0.0` would make the feature a no-op for every
   caller who does not set it, and a field no reachable arrangement reads is the
   `LayoutMode::Grid { columns }` defect this sequence exists to end.
   `the_default_shrink_factor_is_one` asserts the default so that changing it is
   a deliberate act.
2. **The base size is `desired.main(direction)` — the child's main-axis
   *content* size, the number `arrange_flex`'s step 2 already computes for a rigid
   child — and the margins are added to the occupied extent rather than to the
   base.** This is the decision the brief asked to be justified, and here is the
   whole argument.
   - **`desired` and not `declared`.** `declared` is a **`Constraints`**, which is
     a pair of bounds per axis, not a size. A child declaring
     `Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY)` — a bare
     `LayoutState::new()`, which every leaf in this crate is — has a declared
     main extent of *"anything"*, and weighting by it would give every
     unconstrained child a base of **zero**, so it could never shrink
     proportionally to the content it actually has. **That is the
     `flex-basis: 0%` trap**, and it is the trap a reader importing "weight by
     the base size" walks into.
   - **`desired` and not a fresh measurement.** `desired` **is already in
     `FlexItem`**, computed by `sized(nodes, handle, constraints.loosen())`. So
     the shrink pass is arithmetic over a number the pass already holds: **no
     extra `sized` call, no extra traversal, no new per-frame cost shape.**
   - **`desired` and not a new `flex_basis` field.** `desired` is the declared
     constraints **resolved**, which is what a basis resolves to — §
     *`flex-basis`*'s decision 3.
   - **The margins are in the weight's denominator's neighbour, not in the
     weight.** A child's occupied main extent is `leading + content + trailing`,
     and shrinking must reduce the **content**, because a margin is not a size and
     clamping it below zero would overlap the neighbour. **CSS's flex base size
     excludes the margin for the same reason**, so the share is subtracted from
     `desired.main` and the margins are added back afterwards, untouched —
     requirement 6's step 2 and requirement 7's clamp both rest on that
     separation, and `margin_and_shrink_compose_in_one_pass` is the test that
     fails if it is collapsed.
3. **A growing child does not shrink, and the two passes are separate.**
   `flex > 0.0` sets `item.main = 0.0` in step 2, so a growing item's base is
   already zero and **there is nothing for the shrink pass to take from it**.
   Including it would divide by a zero base and produce a `NaN` share. **So
   `fn shrink` skips every item with `flex > 0.0`, `fn distribute` is not touched,
   and `a_growing_child_does_not_shrink` pins the exclusion.** This is also what
   keeps the two distributions independent, which is why this is two components
   and not one (§ *Scope*).
4. **`flex: 0.0` does *not* mean immovable — and this is a divergence from the
   CSS shorthand that must be written down.** In CSS, `flex: 0` is shorthand for
   `flex-grow: 0; flex-shrink: 1; flex-basis: 0%`, **so CSS's `flex: 0` does
   shrink.** In this crate `flex` is **grow only** and `0.0` means *"do not grow"*,
   while the separate `shrink` keeps its own default of `1.0`. **So a caller who
   writes `flex: 0.0` expecting this crate's `flex` to mean the CSS shorthand gets
   a different answer**, and the fix is documentation in the crate's own words:
   `LayoutState::with_flex`'s doc gains *"this is a **grow** factor. It is not the
   CSS `flex` shorthand, which also sets `flex-shrink` and `flex-basis`; use
   [`with_shrink`] to make a child unshrinkable."* A caller who wants a rigid
   child writes `.with_shrink(0.0)`. `flex_zero_does_not_mean_immovable` asserts
   the behaviour **and names the collision in its own name**, which is the
   discipline `gesture_delta`'s sign convention uses.
5. **A shrunken child is handed tight main constraints, through a new
   `tight_main: bool` mirroring `tight_cross`.** Today a placement's main bounds
   are `(item.main, item.main)` for a flexible child and `(0.0, available_main)`
   for a rigid one. **A shrunken child is exactly sized too**, and handing it
   `(0.0, available_main)` would let `resolve_box` measure it straight back down
   to `desired` — **undoing the shrink inside the child's own visit, which no test
   that only reads the placement's rect would see.** `tight_main` is the fix, and
   it is the same field and the same reason as `tight_cross`, whose doc already
   says *"Whether the cross extent is the whole of a bounded box, so the child has
   to be handed a tight cross constraint rather than the box to measure in."*
6. **`fn shrink` is `fn distribute`'s mirror, term for term**, including the
   ratio-before-the-multiply guard, the `fixed`/`pinned` pool, the
   "newly fixed" return, and the *"a budget no item's bounds fit into is left
   unallocated"* ending. Requirement 7 writes it out.
7. **A tight declaration is a hard floor on both sides, so a leaf that declares
   a size cannot shrink — and this is the fact that makes the whole feature's
   reachable set narrow, so it is stated here rather than discovered by a caller
   who sets `shrink` and sees nothing happen.** `Constraints::tight(Size::new(w,
   h))` sets `min_width == max_width == w`, and `Constraints::new(w, w, h, h)`
   is the same four numbers. **`fn clamp_axis(x, w, w) == w` for every `x`**, by
   construction, so a tight item is returned at its declared size whatever share
   it is offered. Three consequences, each of which is a test:

   - **A leaf with a `tight` size never shrinks**, whatever `shrink` says.
     `a_shrink_factor_of_zero_leaves_the_row_overflowing_exactly_as_it_did` and its
     default-`1.0` twin are both built from tight leaves **on purpose**: they are
     the demo's exact case.
   - **A leaf that declares a *range* still has a base of zero**, because
     `desired = sized(nodes, handle, constraints.loosen())` and
     `fn content_size` measures a childless node as `fn bounding_box`'s result
     over no placements, which is `Size::ZERO`. `Constraints::loose(Size::new
     (100.0, 20.0))` therefore yields `desired == Size::ZERO` — **`loose` means
     *"at most this, with no minimum"*, so it asks the node to measure itself
     down, and a node with no content measures down to nothing.**
   - **So the child that can shrink is a node with content** — a container
     holding children, or a leaf carrying `Padding`, because `fn content_size`
     adds `padding.right` and `padding.bottom` to the content it measured. **This
     is not a defect and it is not changed here**: `content_size`'s doc commits the
     crate to *"a node's size stays a property of its children rather than of how
     it arranges them"*, and a base of zero for a node with no children **is**
     that property. **What this task owes the caller is the sentence**, in
     `LayoutState::with_shrink`'s doc: *"A child shrinks from its own content size,
     so a child with no content has nothing to shrink; a child that declares a
     `tight` size declares a floor equal to its size and does not shrink at all."*
     Requirement 12 puts it there and requirement 15's `L7` note records it.

   **And the base-size decision stands on this ground rather than on the
   alternative**: weighting by `declared` would give a contentless child a base of
   zero *and* a contentful child one too, because `declared` is a range — so the
   `flex-basis: 0%` trap would be universal rather than partial.

### Overflow: what a container smaller than its content does

**The outcome is defined, and it has three cases, in this order.**

1. **If anything can shrink, it does, down to its declared minimum.** `fn shrink`
   stops at `clamp_axis(base - share, min_declared, max_declared)`, and
   `clamp_axis`'s documented rule is that **the minimum wins** when
   `min > max`. So the floor is a number the caller declared, not one this task
   invented.
2. **The leftover overflow is not squeezed further.** When every shrinkable item
   has hit its declared minimum and the budget is still positive,
   **`fn shrink` returns with the budget unspent**, `used` stays above
   `available_main`, `extra` goes negative, and `fn alignment_spacing` does what
   it already does with a negative `extra` — `Start` puts the row at `0.0` and
   `Center` at `extra / 2.0`, which is a **negative** leading offset and
   therefore **centres the overflowing row on the box**. That is today's
   behaviour for an overflowing centred row and it is unchanged.
   **So: children keep their sizes, and the row runs past the box.**
3. **The overflow is clipped, because the clip already exists.**
   `Layout::visit` computes `children_clip = intersect(clip, Some(rect))` for every
   child of a node, and `an_overflowing_child_keeps_its_size_and_computes_a_clip
   _rect` already pins it. **Nothing is scaled to fit, no child's rect is
   rewritten, and there is no `transform: scale`** — a layout pass that rescaled
   children would make a parent and a child disagree about a size, which
   `layout.rs`'s module doc forbids by name: *"Asking and placing are the same
   code … so a parent and a child never disagree about a size."*

**Two axes are excluded from shrinking, and both exclusions are stated rather
than assumed:**

- **The cross axis.** `flex-shrink` is main-axis only, in this crate and in CSS.
  Cross-axis overflow is clipped, exactly as it is today.
  `shrink_does_nothing_on_the_cross_axis` pins it.
- **An unbounded main axis.** `arrange_flex`'s step 2 already sets
  `available_main = rigid_total + gaps` when `max_main` is not finite, which makes
  the overflow `0.0` — **so there is nothing to shrink against**, and this is the
  same reasoning `arrange_flex`'s own comment gives for not *growing* there.
  **`content_size` measures under `Constraints::UNBOUNDED`, so this is the path a
  container with no declared constraints takes every time it is measured**, and
  `an_unbounded_main_axis_never_shrinks` says so.

**The consequence a caller must be told, in `with_shrink`'s doc and in
`LayoutMode::Flex`'s variant doc:** *"an item is never shrunk below its declared
minimum; whatever is still too wide after that runs past the container and is
clipped."*

### Working beside `TASK_UI_PRIM_52`

**`TASK_UI_PRIM_52` implements `LayoutMode::Grid` and amends row `L7`'s `spacing`
claim in the same file this task amends.** Four overlaps, and each one's rule is
stated rather than left to the ordering of two agents:

| overlap | this task's rule |
|---|---|
| **`FlexConfig`'s documentation.** 52's requirement 5 changes four doc sites to say **`LayoutMode::Grid` uses `spacing` as its gap on both axes, from one value**. 48's requirement 11 changes the same sites to say **`spacing` is the gap between *adjacent children***. | **Both sentences must be present.** 48 adds its clause to the existing text and **does not delete 52's**; and if 52 has not landed, 48's wording is written so that 52's sentence can be appended without a rewrite. Neither task's doc diff may revert the other's. |
| **`struct Placement`.** 52 adds `arrange_grid`, which constructs one `Placement` in its placement loop. 48 adds a private `trailing: Size` field to `Placement`. | **`trailing` is private**, so this is not a `pub` API change. **If `arrange_grid` exists when 48 lands, requirement 8's one added argument at that one push is `Size::ZERO` and nothing else about the grid.** If it does not exist, 52 adds the field to its own construction when it does, and requirement 8 says so in the task file so it is not a surprise. |
| **A field one parent mode reads.** 52 documents that `flex` is unread inside a grid cell and pins it with `a_grid_cell_ignores_its_flex_factor`. 48's `margin` is unread inside a grid cell and gets `a_margin_is_not_read_by_a_grid_cell`. | **48 does not modify `arrange_grid`, `grid_column_width`, `grid_row_heights`, `LayoutMode::Grid`'s doc, or any of 52's twenty-one tests.** The grid-facing test is **conditional on `arrange_grid` existing** and the requirement says so, so an implementer on a tree without it does not invent the mode. |
| **Row `L7`.** Both tasks append a dated, attributed note. | **The notes are appended in task order, 52's first.** 48's note **quotes 52's sentence** rather than restating it in its own words, so the two cannot disagree — § *Corrections to the second gap table* exists because a sweep of one table leaves the other stale, and the same failure inside one row has the same fix. **Neither note is deleted when the other is written.** |

### Scope, measured against `developer.md` § *Scope check*

**Four files, two independent components, both inside both thresholds.**
`developer.md` § *Scope check* says a change touching **more than 5 files** or
having **more than 3 independent components** is too large for one agent and must
be split per `.ai/protocols/subagents.md` § *Implementation fan-out*.

| file | what changes |
|---|---|
| `ui/src/ui_core/src/layout.rs` | `Margin`; `LayoutState`'s `margin` and `shrink` fields with their six methods; `FlexItem`'s four new fields; `arrange_flex`'s steps 2 to 4; `fn shrink`; `Placement::trailing` and `bounding_box`; `arrange_stack`'s and `arrange_absolute`'s leading-margin offsets; `FlexConfig`'s and `LayoutMode::Flex`'s and `arrange`'s docs; the twenty-six tests and three doctests |
| `ui/src/ui_core/src/widgets/container.rs` | `Container::set_margin`, in `set_padding`'s exact shape, plus the `set_flex_config` doc sentence 52's requirement 5 also touches |
| `doc/ui/DEMO_APPLICATION.md` | row `L7` amended — its four clauses, its evidence column rewritten to symbols, and 52's sentence quoted |
| `doc/ui/IMPLEMENTATION_STATE.md` | the task-table row, the record section, and the `Current position` update |

**Components: two, and the honest count is two because two of the four gap items
are closed by argument rather than by code.**

1. **Margin** — the type, the field, the composition with `Padding`, the three
   modes that read it, `Placement::trailing` and `bounding_box`. Testable on its
   own: every one of requirement 14's ten margin tests reaches it without setting
   a shrink factor.
2. **`flex-shrink`** — the factor, `fn shrink`, the base-size rule, the
   `tight_main` change, the overflow policy. Testable on its own: every one of
   requirement 14's ten shrink tests reaches it without setting a margin.

**`flex-basis` and the cross-axis gap are not components: neither produces a line
of code.** The first is § *`flex-basis`*'s argument plus three tests; the second is
four doc sentences plus one test. **Counting them as components would be the
dishonest way to pass a threshold**, and it is stated here so that a reviewer can
check the count rather than take it.

**The two components share one function.** `arrange_flex` is where margin enters
the main-axis sum and where shrink enters the size of each item, so the two diffs
land in the same forty lines. **That is a reason for care, not a reason to
split**: `developer.md`'s threshold is about files and independent components,
and this is one file and two independently reasoned and independently testable
changes. **The integration is what the twenty-fifth test
(`margin_and_shrink_compose_in_one_pass`) exists for**, because a change that
proves each part and never both together has not been integrated.

**No fan-out is needed** — `.ai/protocols/subagents.md` § *Implementation
fan-out* does not apply — **and the implementer should say so rather than
splitting a two-component change. If the implementer finds themselves editing a
second *code* file other than `container.rs`, that is a stop condition, not an
expansion** (`developer.md` § *Stop conditions*).

## Requirements

1. **A new `pub struct Margin` in `ui/src/ui_core/src/layout.rs`, immediately
   after `impl Padding`**, in `Padding`'s exact shape and voice:

   ```rust
   /// The gap a node's siblings leave between it and them.
   ///
   /// A margin is the mirror of [`Padding`] and not a synonym for it: padding
   /// is taken **off** the box a node lays its children out in, and a margin is
   /// **added** to the position a parent puts a node at. …
   ///
   /// **Margins do not collapse.** …
   ///
   /// # Examples
   /// ```
   /// use ui_core::layout::Margin;
   ///
   /// let card = Margin::all(8.0);
   /// assert_eq!(card.horizontal(), 16.0);
   /// assert_eq!(card.vertical(), 16.0);
   /// assert_eq!(Margin::ZERO.horizontal(), 0.0);
   /// ```
   #[derive(Clone, Copy, Debug, Default, PartialEq)]
   pub struct Margin {
       /// The gap before the node's left edge.
       pub left: f32,
       /// The gap after the node's right edge.
       pub right: f32,
       /// The gap before the node's top edge.
       pub top: f32,
       /// The gap after the node's bottom edge.
       pub bottom: f32,
   }
   ```

   - `pub const ZERO: Margin`, `pub fn all(sides: f32) -> Self` `#[must_use]`,
     `pub fn horizontal(self) -> f32` `#[must_use]`,
     `pub fn vertical(self) -> f32` `#[must_use]`, and a **private**
     `fn clamped(&self) -> Margin` flooring each side at zero, each with a doc
     comment in `Padding`'s voice.
   - **`Margin` has no `inset`.** Its doc says why in one sentence: *"A margin has
     no inset operation — its near edge is added to a position, not subtracted
     from a box."*
   - **The type's doc carries the four facts a caller gets wrong**, in this
     order: **it is not `Padding`**; **margins do not collapse**, with the
     CSS-flexbox precedent and `content_size`'s property named; **it is read by
     `LayoutMode::Flex` in full and by `LayoutMode::Stack` and
     `LayoutMode::Absolute` for its leading edges only, and not at all by
     `LayoutMode::Grid`**; **a negative side is floored at zero when it is
     stored**.
   - **No `Margin::from(Padding)`, no `impl From<Padding> for Margin`, and no
     `Deref`.** `developer.md` § *API design* asks for newtypes, and §
     *`flex-basis`*'s decision 1 is the reason a conversion between the two is
     refused.

2. **`LayoutState` gains a `margin` field**, declared immediately after
   `padding`, and the three methods that go with every other input:
   `LayoutState::with_margin(&mut self, margin: Margin) -> Self` `#[must_use]`
   storing `margin.clamped()`; `LayoutState::set_margin(&mut self, margin: Margin)`
   storing `margin.clamped()` and calling `self.mark_dirty()`; and
   `LayoutState::margin(&self) -> Margin` `#[must_use]`. **`LayoutState::default`
   initialises it to `Margin::ZERO`.** Their doc comments state, in the first
   line, that **the pass is what applies it** — `Padding`'s doc says the same
   about itself and the wording is shared deliberately — and **`set_margin`'s doc
   carries the dirty-flag sentence `set_padding`'s does**: *"the children were
   placed in the unpadded box, and only a pass over this node moves them."*

3. **`ui/src/ui_core/src/widgets/container.rs` gains
   `Container::set_margin(&self, nodes: &mut Arena<WidgetNode>, margin: Margin)`**,
   in `Container::set_padding`'s exact shape — `if let Some(node) = nodes.get_mut
   (self.node) { node.layout_mut().set_margin(margin); }` — with a doc comment
   carrying `set_padding`'s argument: *"the margin is a layout input, so it lands
   on the node's `LayoutState` and the pass applies it — the same margin any other
   node with children could declare, and the reason a container is not special to
   the pass."* **A container that is itself a child of another container is how a
   margin reaches a panel**, so the setter is what makes the field usable and not
   only reachable. **Nothing else in `container.rs` changes**, except the one doc
   sentence requirement 11 names.

4. **`LayoutState` gains a `shrink` field**, declared immediately after `flex`,
   with `LayoutState::with_shrink(&mut self, shrink: f32) -> Self` `#[must_use]`
   storing `shrink.max(0.0)`; `LayoutState::set_shrink(&mut self, shrink: f32)`
   storing `shrink.max(0.0)` and calling `self.mark_dirty()`; and
   `LayoutState::shrink(&self) -> f32` `#[must_use]`.
   **`LayoutState::default` initialises it to `1.0`, not `0.0`.** The
   initialisation is written out in `Default for LayoutState`'s body rather than
   left to a derive, and **`with_shrink`'s doc says, in its first line**, that
   `1.0` is the default and why: *"the CSS initial value, and `0.0` would leave
   the factor unread by every arrangement for every caller who did not set it —
   a field with no consumer, which is what `LayoutMode::Grid { columns }` was."*
   **The clamp is `max(0.0)` and nothing more, exactly as `with_flex`'s is**, so
   an `f32::INFINITY` factor is stored and requirement 7's arithmetic and
   requirement 14's non-finite test say what it does.

5. **`struct FlexItem` gains four fields**, declared after `flex` and `desired`:
   `margin: Margin`, `shrink: f32`, `base: f32`, and
   `tight_main: bool` **immediately before `tight_cross`** so the two `tight_*`
   flags sit together. Each carries a doc comment:

   - `margin` — *"The margin this child declares. It is a child-side input read
     by the parent's arm, and `Margin::ZERO` contributes nothing to any sum."*
   - `shrink` — *"How much of its base this child gives up when the row does not
     fit. Zero means it keeps the base, which is not the same as growing and is
     set separately."*
   - `base` — *"The child's main-axis **content** size: `desired.main(direction)`.
     This is the number shrink is weighted by, and CSS's flex base size excludes
     the margin, so this does too — the margin is added to `main` after the share
     is taken off, and is therefore never itself shrunk. Held rather than
     recomputed so grow and shrink agree on one number."*
   - `tight_main` — *"Whether the main extent is exactly this item's share, so the
     child is handed tight main constraints rather than the box to measure in."*
     The wording mirrors `tight_cross`'s existing doc. **It is written `true` in
     step 2 for a growing item and inside `fn shrink` for an item that was
     actually shrunk, and nowhere else** — **which is why it needs no
     cross-term against `base` and why it cannot be set for an item that did not
     change.**

   **The two `FlexItem` constructions in `arrange_flex` — the live one and the
   stale-handle one — both gain all four**, and the stale one is
   `margin: Margin::ZERO, shrink: 0.0, base: 0.0, tight_main: false`. **`shrink:
   0.0` and not `1.0` for a handle that no longer resolves is deliberate**: a
   dead handle has `desired = Size::ZERO`, so a base of zero would exclude it from
   the shrink pass anyway, and `shrink: 0.0` makes the exclusion explicit rather
   than incidental.

6. **`arrange_flex`'s steps 2, 3 and 4 are rewritten, and the whole of the new
   arithmetic is here because "implementable without questions" is the
   requirement and this is where the questions would otherwise live.**

   **Step 1 is unchanged except that each pushed `FlexItem` reads the two new
   inputs**: `margin: node.layout().margin()` and `shrink: node.layout().shrink()`.

   **Step 2 — the base, the occupied extent, and the overflow.** For each item,
   in this order:

   ```rust
   item.base = item.desired.main(direction);
   if item.flex > 0.0 {
       item.main = 0.0;
       item.tight_main = true;
   } else {
       item.main = item.base
           + main_axis_leading(&item.margin, direction)
           + main_axis_trailing(&item.margin, direction);
   }
   rigid_total += item.main;
   ```

   **`item.base` is the child's content size and the margins are added
   separately, and that separation is load-bearing.** CSS's flex base size
   excludes the margin, so the shrink weight excludes it too; and **a share taken
   off `item.main` would eat a margin**, which is the one thing a margin must
   never do — § *`flex-shrink`*'s decision 2's third clause. **`rigid_total`
   therefore carries the margins, because the margins are real occupied space on
   the row's main axis**, and it no longer needs an axis-conditional: `item.main`
   is already axis-agnostic.

   **The two axis helpers are named, private, doc-commented and one caller each:**
   `fn main_axis_leading(margin: &Margin, direction: FlexDirection) -> f32`
   returns `margin.left` for `Row` and `margin.top` for `Column`;
   `fn main_axis_trailing(margin: &Margin, direction: FlexDirection) -> f32`
   returns `margin.right` for `Row` and `margin.bottom` for `Column`. **They
   exist so that "which pair of sides is the main axis" is written once**, the
   same reason `fn direction_cross_to_xy` and `fn Size::main` exist; and their
   doc comments say that `Constraints::main` is the precedent for the same
   question. **`count_to_f32` is not involved and no `usize` is converted here.**

   Then, unchanged in shape:

   ```rust
   let gaps = spacing * count_to_f32(children.len().saturating_sub(1));
   let available_main = if max_main.is_finite() { max_main } else { rigid_total + gaps };
   distribute(&mut items, (available_main - rigid_total - gaps).max(0.0), direction);
   ```

   **and then the new line**, immediately after `distribute` and before step 3:

   ```rust
   shrink(&mut items, (rigid_total + gaps - available_main).max(0.0), direction);
   ```

   **The overflow is computed from `rigid_total + gaps`, which already contains
   every child's margins**, so the two are consistent by construction and the
   shrink pass cannot be asked to remove space that is not there. **When
   `available_main` was the unbounded branch, `rigid_total + gaps - available_main`
   is exactly `0.0`,** which is § *Overflow*'s second exclusion, arrived at
   arithmetically rather than by an `is_finite` test.

   **Step 3 — the cross axis, with margins.** `item.tight_cross` keeps its
   existing expression verbatim. `item.cross` becomes:

   ```rust
   item.cross = if item.tight_cross {
       clamp_axis(
           max_cross
               - cross_axis_leading(&item.margin, direction)
               - cross_axis_trailing(&item.margin, direction),
           min_declared,
           max_declared,
       )
   } else {
       item.desired.cross(direction)
   };
   ```

   with `fn cross_axis_leading` / `fn cross_axis_trailing` the same two helpers
   with the pairs swapped (`top`/`bottom` for `Row`, `left`/`right` for
   `Column`). **The subtraction is inside the `clamp_axis`, so a margin larger
   than the box produces a negative candidate that `clamp_axis` floors at the
   child's declared minimum — never a negative cross extent**, which is the same
   guard `grid_column_width`'s `.max(0.0)` is required to have by task 52's
   requirement 3 and for the same reason.

   **Step 4 — positions, with margins.** Inside the existing `for item in &items`
   loop:

   ```rust
   let cross_offset = cross_offset(
       config.cross_axis_alignment(),
       max_cross,
       item.cross
           + cross_axis_leading(&item.margin, direction)
           + cross_axis_trailing(&item.margin, direction),
   ) + cross_axis_leading(&item.margin, direction);
   let (x, y) = direction_cross_to_xy(direction, cursor + main_axis_leading(&item.margin, direction), cross_offset);
   ```

   **and `cursor += occupied_main(&item, direction, gap);`**, where

   ```rust
   /// How far the main-axis cursor moves past one item, gap included.
   ///
   /// `item.main` is already the item's occupied main extent — its content plus
   /// its main-axis margins — so this is one addition, and putting it in a
   /// function is what stops the cursor and the rect from being two expressions
   /// that could disagree.
   fn occupied_main(item: &FlexItem, direction: FlexDirection, gap: f32) -> f32
   ```

   **Requirement: `cursor` advances by the item's occupied main extent plus the
   gap, and the item's rect origin is the cursor plus its leading margin.** That
   is the whole of margin's effect on a flex placement, and `occupied_main` is
   what makes the two expressions one.

   **The two `main_bounds` lines change**, and this is the subtle one:

   ```rust
   let main_bounds = if item.tight_main { (item.main, item.main) } else { (0.0, available_main) };
   ```

   with **`item.tight_main` written `true` in exactly two places** — in step 2 for
   a growing item, and inside `fn shrink` for an item that was actually shrunk —
   **and nowhere else.** So `tight_main` needs no cross-term against `base`, and
   an item whose factor is `1.0` but whose share was zero is **not** tight, which
   is right: it kept its base and `resolve_box` may still measure it inside
   `available_main`, exactly as it does today. **This is the fix for the silent
   un-shrink**: a child whose main extent is smaller than the box would otherwise
   be handed the box to measure in, and `resolve_box` would measure it straight
   back down to `desired` — **undoing the shrink inside the child's own visit,
   which no test reading only the parent's placement would see.**

   **`tight_cross` is untouched, and the `cross_bounds` expression keeps its
   existing shape** — `(item.cross, item.cross)` when tight, `(0.0, max_cross)`
   otherwise — because the cross margins are already inside `item.cross`.

7. **`fn shrink(items: &mut [FlexItem], overflow: f32, direction: FlexDirection)`,
   private to `layout.rs`, placed immediately after `fn distribute`** and
   written in its shape:

   ```rust
/// Takes `overflow` pixels off the main axis, in proportion to each item's
    /// `shrink × base`, and within each item's own declared bounds.
    ///
    /// The mirror of `distribute`, with three differences that are all decisions
    /// rather than omissions: an item with `flex > 0.0` is skipped, because its
    /// base is zero and there is nothing to take from it; the share is taken off
    /// the base and the margins are added back afterwards, so a margin is never
    /// itself shrunk; and the whole call is a no-op when `overflow` is zero or
    /// when no item's weight is positive, which is every frame of every page on
    /// which nothing overflows.
    fn shrink(items: &mut [FlexItem], overflow: f32, direction: FlexDirection)
   ```

   - **`if overflow <= 0.0 { return; }` is its first line**, before the
     `fixed`/`pinned` setup, for the reason its own doc's last sentence gives.
   - `let mut fixed = vec![false; items.len()];` and `let mut pinned = 0.0f32;`
     — **the same two lines `distribute` opens with, and the same one
     `Vec` allocation.** `distribute`'s doc already says the loop runs at most
     once per item, and this inherits the argument.
   - The loop's `total` is
     `items.iter().enumerate().filter(|(i, item)| !fixed[*i] && item.flex <= 0.0
     && item.shrink > 0.0).map(|(_, item)| item.shrink * item.base).sum::<f32>()`
     — **the weighted factor, and `if total <= 0.0 { return; }`**, which is what
     makes a row of unshrinkable children overflow exactly as it does today.
   - `let free = (overflow - pinned).max(0.0);` — the same `free` as
     `distribute`'s, over the overflow rather than the budget.
   - Inside, for each unfixed, non-growing item: **`let share = free * (scaled /
     total_scaled);` where `scaled = item.shrink * item.base`**, and **the ratio
     is taken before the multiply for `distribute`'s stated reason**, quoted in
     this function's doc: *"two factors near the `f32` ceiling overflow to `inf`
     before the division brings them back down, and the child was given an
     infinite width."* The ratio is at most `1.0` by construction.
- `let (min_main, max_main) = item.declared.main(direction);`
      `let taken = clamp_axis(item.base - share, min_main, max_main);`
      `item.main = taken + main_axis_leading(&item.margin, direction)
      + main_axis_trailing(&item.margin, direction);` and
      `if taken != item.base - share { item.tight_main = true; newly_fixed.push(index); }`
      — **an assignment, never an accumulation**, which is the property
      requirement 14's degeneracy test checks from outside, and **the margins
      added after the clamp, never before it**, so a margin is never shrunk and
      `item.main` can never be negative because `taken` cannot be.
    - `if newly_fixed.is_empty() { return; }` then the pin loop, `pinned +=
      items[index].main;` — **which is `distribute`'s line verbatim**, because
      `item.main` already carries the margins from the line above and adding them
      again would be the double count requirement 6's separation exists to
      prevent.

    - **A non-finite `scaled` is handled by `clamp_axis`, not by a guard.**
      `with_shrink` accepts `f32::INFINITY`, so `total` can be `inf` and
      `scaled / total` a `NaN`; `clamp_axis`'s doc says it *"passes the non-NaN
      operand through, so a NaN in a constraint cannot turn into a panic"*, so the
      child lands on its declared minimum and is fixed. **Requirement 14's
      non-finite test asserts that outcome by name**, because "it does not panic"
      is a weaker claim than "here is what it does".

8. **`struct Placement` gains one private field, `trailing: Size`**, declared
   last, and `fn bounding_box` reads it:

   ```rust
   /// The margin on this placement's far edge on each axis.
   ///
   /// A leading margin is inside `rect.origin`, and this is the half that is in
   /// no rect at all: `content_size` measures a node from the extent its
   /// placements reach, so without this a container whose last child declares a
   /// trailing margin measures short by that margin and clips its own child.
   /// `arrange_flex` fills it with the main- and cross-axis trailing margins in
   /// x and y terms; every other mode fills it with `Size::ZERO`.
   trailing: Size,
   ```

   and `bounding_box`'s loop becomes

   ```rust
   let corner = placement.rect.far_corner();
   right = right.max(corner.x + placement.trailing.width);
   bottom = bottom.max(corner.y + placement.trailing.height);
   ```

   **This is the same loop `layout_walk_cost`'s first shape measures**, so the
   change is two `f32` adds inside an existing `max` pair, not a new traversal,
   and the handoff reports the harness's numbers **unchanged** because the harness
   is not to be modified. **`arrange_stack`'s and `arrange_absolute`'s single
   `Placement` expressions each gain `trailing: Size::ZERO`**, and **`if
   `arrange_grid` exists when this task lands, its one push in its placement loop
   gains `trailing: Size::ZERO` as well — that is the whole of the interaction
   with task 52, and nothing else about the grid changes.**

9. **`arrange_stack` and `arrange_absolute` apply the leading margin, and only
   the leading margin.** Each gains, in its `map` closure:

   ```rust
   let margin = node_layout_margin(nodes, handle);
   // … origin.x + margin.left, origin.y + margin.top
   ```

   with one private helper, shared by both, because two call sites is the second
   use `developer.md` § *Phase 2* asks for and because a helper is what keeps the
   two arms from disagreeing:

   ```rust
   /// The margin `handle` declares, or [`Margin::ZERO`] for a stale handle.
   fn node_layout_margin(nodes: &Arena<WidgetNode>, handle: Handle) -> Margin
   ```

   **`arrange_absolute`'s origin is `position + (margin.left, margin.top)`, and
   its `unwrap_or(Offset::ZERO)` becomes
   `position.map(|p| Offset::new(p.x + margin.left, p.y + margin.top)).unwrap_or(
   Offset::new(margin.left, margin.top))`** — **so an unpositioned child in an
   `Absolute` parent moves off the origin when it has a margin, and that is a
   behaviour change from today, which is the point of
   `a_margin_moves_a_stacked_child_off_the_parents_origin` and
   `absolute_places_an_unpositioned_child_at_the_origin` read together.** The
   existing test keeps its name and gains the no-margin case it already covers:
   **`Margin::ZERO` is arithmetically inert, so `absolute_places_an_unpositioned
   _child_at_the_origin` and `stack_places_every_child_at_the_origin` keep
   passing unchanged, and requirement 14 says so by name so the suite does not
   look like it lost a test.**

   **`arrange_flex` reads the child's margin from `node.layout().margin()` inside
   its own step 1 loop and does not call `node_layout_margin`** — it already has
   the `node` in hand, and a second lookup would be a second `nodes.get`.

10. **No `flex_basis` field is added, and the decision is recorded in three
    places so the next agent does not re-open it.** The three are
    **`LayoutState::with_constraints`'s doc** (which gains *"this is the basis a
    flex container shrinks against; see [`LayoutState::with_shrink`]"*),
    **`LayoutState::with_flex`'s doc** (which gains *"a growing child's basis is
    zero, which is what `flex` is for; there is no separate basis to set"*), and
    **row `L7`'s amendment** (requirement 15). **The `LayoutState` field list in
    `PRIMITIVES_ARCHITECTURE.md` § *Node structure* does not change** — it
    carries `layout: LayoutState` as one line and names no field.

11. **`spacing` stops being documented as main-axis only, in four places in
    `layout.rs` and one in `container.rs`.** `FlexConfig`'s doc,
    `FlexConfig::with_spacing`'s doc, `FlexConfig::spacing`'s doc and
    `LayoutState::with_flex_config`'s doc each currently scope the value to the
    main axis; each becomes: *"the gap between **adjacent** children. In a
    one-line `LayoutMode::Flex` container that is the main axis; in a
    `LayoutMode::Grid` container it is both axes, from this one value."* **If
    task 52's requirement 5 sentence is present, it is kept and this clause is
    added to it — never substituted for it.** **`Container::set_flex_config`'s
    doc says *"how a **flex mode** arranges this container's children"* and
    becomes false the moment a grid reads `spacing`, so it gains the same clause
    plus the sentence naming the modes that read `spacing` and the two that do
    not.**
    **No type changes, no new field, no new getter, and `MainAxisAlignment` and
    `CrossAxisAlignment` are untouched.**

12. **Four doc comments gain the sentences that name who reads what**, in the
    shape `.ai/NEVERAGAIN.md` § *A position API that only one parent mode reads*
    prescribes — *"a setter that a **parent** consumes is not honoured by every
    parent, and the compiler will not say so"*:

    - **`LayoutState::with_flex`, `LayoutState::with_shrink`,
      `LayoutState::set_flex` and `LayoutState::set_shrink`** each gain:
      **`flex` is honoured by `LayoutMode::Flex` only; `shrink` is honoured by
      `LayoutMode::Flex`, on the main axis, and by no other mode and no other
      axis.** `with_shrink` additionally gains **two** sentences — the overflow
      one from § *Overflow*, and the reachability one from § *`flex-shrink`*'s
      decision 7, quoted there verbatim: *"A child shrinks from its own content
      size, so a child with no content has nothing to shrink; a child that
      declares a `tight` size declares a floor equal to its size and does not
      shrink at all."* **`with_flex` gains the CSS-shorthand warning** quoted in
      § *`flex-shrink`*'s decision 4 verbatim. **Both sentences are on
      `with_shrink` and not on the field, because a caller reads the builder's
      doc and not the field's.**
    - **`LayoutState::margin`, `set_margin` and `Margin`'s own doc** each name
      the modes, per § *Margin*.
    - **`LayoutState::flex_config`** names `LayoutMode::Flex` and
      `LayoutMode::Grid` as the modes that read `spacing` and
      `LayoutMode::Stack` and `LayoutMode::Absolute` as the modes that ignore it
      — **which is a fact no doc comment states today and no test covers.**
    - **`LayoutMode::Flex`'s variant doc** reads *"Children run along `direction`
      on one line, sharing the space that is left after the rigid ones are
      sized."* **That sentence becomes incomplete** — the space left may be
      negative — so it gains: a child's **leading** and **trailing** margins
      offset and consume space; **an item is never shrunk below its declared
      minimum**; **whatever is still too wide after that runs past the container
      and is clipped**; and **a growing child has a basis of zero and does not
      shrink**.

13. **`arrange`'s own doc gains the margin's place in the order, in one
    sentence.** It currently says *"The children are arranged in the box
    `padding` leaves, and their rects are then moved out to where that box
    starts."* The addition: *"A child's own `Margin` is read **inside** the mode's
    arm, from the child, and is added to the placement the arm produces — so the
    two compose in a fixed order, padding first and then margin, and neither
    reads the other."* **`fn arrange`'s signature, its `match`, and the
    `if padding != Padding::ZERO` fast path are all untouched** — and the handoff
    says so, because that fast path is the branch `layout_walk_cost`'s
    `flat-2041 clean pass` figure is measured against and **moving it would make
    that number unreproducible.**

14. **The twenty-six tests, named, beside the code in `layout.rs`'s
    `#[cfg(test)] mod tests`, with no display, no network, no filesystem and no
    wall clock** — the only kind `AGENTS.md` permits. **Every assertion goes
    through `layout_constraints` or `Layout::layout`, and no test names a private
    function**: `shrink`, `main_axis_leading`, `main_axis_trailing`,
    `cross_axis_leading`, `cross_axis_trailing`, `node_layout_margin` and
    `occupied_main` are reached only through the public path, because
    `reviewer.md` § Phase 1 asks whether a test asserts the contract or the
    implementation. **The existing helpers are used as they are** — `leaf`,
    `container`, `padded`, `layout_in`, `laid_out_by`.

    **Margin — ten.**

    - **`a_leading_margin_pushes_a_child_past_its_siblings_start`** — two leaves
      and a container with `with_spacing(0.0)`; the first child carries
      `Margin { left: 10.0, ..Margin::ZERO }` and the second's origin is asserted
      **by value**.
    - **`a_trailing_margin_pushes_the_next_sibling`** — the mirror, and the two
      halves of `Margin::horizontal()` are asserted so a change to the sum fails
      here.
    - **`a_trailing_margin_on_the_only_child_is_included_in_the_containers_
      measured_size`** — the `bounding_box` half, and **the test that fails if
      requirement 8's `trailing` field is dropped.** A container with one child at
      `Margin { right: 8.0, bottom: 8.0, ..Margin::ZERO }` and no declared
      constraints, measured through `laid_out_by` or the child's own rect after a
      `loose` offer, asserting the container's width is content **+ 8** and its
      height content **+ 8**.
    - **`two_adjacent_margins_do_not_collapse`** — two children with
      `right: 8.0` and `left: 8.0`, asserting a **16**-pixel gap. **The name
      states the decision**, so a future task that implements collapsing must
      break it deliberately.
    - **`a_margin_is_clamped_to_zero_when_it_is_stored`** — the mirror of
      `a_negative_padding_is_clamped_to_zero_when_it_is_stored`, and it asserts
      through `margin()` that a stored negative reads back as zero.
    - **`padding_is_applied_before_margin_and_the_two_add`** — `Padding::all(12.0)`
      on the container, `Margin::all(8.0)` on the first child, asserting the
      child's absolute origin is `(20.0, 20.0)`. **This is § *Margin*'s total
      order, and the test's name says the order.**
    - **`a_margin_offsets_an_absolutely_placed_child_from_the_position_it_declared`**
      — `with_position(Offset::new(10.0, 10.0))` and `Margin::all(8.0)`, asserting
      the child's origin is `(18.0, 18.0)`.
    - **`a_margin_moves_a_stacked_child_off_the_parents_origin`** — a `Stack` with
      two children, the second with `Margin::all(6.0)`; the second is at `(6, 6)`
      and the first is still at `(0, 0)`. **`stack_places_every_child_at_the_
      origin` keeps its name and its assertions**, because its children declare no
      margin.
    - **`a_margin_on_the_cross_axis_centres_a_child_inside_its_own_margins`** —
      `CrossAxisAlignment::Center`, a child **40** wide in a **100**-wide box,
      `Margin { left: 4.0, right: 12.0, ..Margin::ZERO }`: its margin box is
      `4 + 40 + 12 = 56`, so `cross_offset(Center, 100.0, 56.0) = 22.0` and the
      child's origin is `22.0 + 4.0 = 26.0`, **its centre at 46 against the box's
      50** — **4 pixels left, which is `(right - left) / 2`.** The number in the
      test's body is the assertion; the arithmetic is in its comments.
    - **`a_margin_is_not_read_by_a_grid_cell`** — **conditional on `arrange_grid`
      existing when this task lands**; three cells in one row, the second with a
      margin, **all three the same width**. The mirror of task 52's
      `a_grid_cell_ignores_its_flex_factor`.

    **`flex-shrink` — eleven.**

    - **`shrink_weights_by_the_declared_size`** — **the worked example, by value**,
      and it is also § *`flex-basis`*'s test. **Both children are containers, and
      that is not incidental**: § *`flex-shrink`*'s decision 7 is why a childless
      leaf has a base of zero. Each holds one leaf — **220 × 20** and **110 × 20** —
      and each declares `Constraints::new(0.0, f32::INFINITY, 0.0,
      f32::INFINITY)`, so each is shrinkable and its base is its content size.
      `with_spacing(0.0)`, a **165**-wide box, `shrink` left at its `1.0` default
      on both. `rigid_total` is `330`, the gaps are `0`, the overflow is `330 -
      165 = 165`, the weight is `220 + 110 = 330`, and the shares are
      `165 × (220 / 330) = 110.0` and `165 × (110 / 330) = 55.0` — so the two
      children are **110** and **55** wide, at origins `0` and `110`, with the
      second's far edge at exactly `165`. **The arithmetic is in the test's
      comments** so a reader can check it, and the exact-rect assertion is what a
      mutation that swaps the two weights, or that weights by the factor alone,
      fails.
    - **`a_shrink_factor_of_zero_leaves_the_row_overflowing_exactly_as_it_did`** —
      two tight children of **100** each in a **150**-wide box with
      `with_shrink(0.0)` on both: origins `0` and `100`, both children **100**
      wide, and the second's far edge at `200`, i.e. past the box. **This is the
      regression test for the demo's pixels**, and requirement 13's
      `a_row_whose_children_all_declare_a_tight_size_does_not_shrink` is its
      default-`1.0` twin.
    - **`a_row_whose_children_all_declare_a_tight_size_does_not_shrink`** — **the
      same geometry with the shrink factor left at its `1.0` default**, asserting
      the identical numbers. **The pair is the point**: a test with the factor set
      to zero proves the pass is wired up and proves nothing about what a caller
      who sets nothing gets, **and the demo is a caller who sets nothing.**
    - **`a_growing_child_does_not_shrink`** — `with_flex(1.0)` **and**
      `with_shrink(4.0)` in a box too small: the child's main extent is its share,
      is **`>= 0.0`**, and the row's total equals `available_main`. **The name
      states the exclusion** rather than the outcome.
    - **`flex_zero_does_not_mean_immovable`** — `with_flex(0.0)`, `shrink` left at
      its default, and a **container** declaring
      `Constraints::new(0.0, 100.0, 0.0, f32::INFINITY)` that holds one leaf of
      100 × 20, in a **60**-wide box: `rigid_total` is 100, the overflow is 40,
      the weight is 100, and `clamp_axis(100 - 40, 0.0, 100.0)` is **60** — **the
      child shrinks to 60**, which is CSS's `flex: 0` behaviour, and **the test's
      doc comment names the collision** and points at `with_shrink(0.0)` as the way
      to be immovable.
    - **`an_item_is_never_shrunk_below_its_declared_minimum`** — **two
      containers, both holding one leaf** — one declaring
      `Constraints::new(80.0, f32::INFINITY, 0.0, f32::INFINITY)` around a
      200 × 20 leaf, the other
      `Constraints::new(30.0, f32::INFINITY, 0.0, f32::INFINITY)` around a
      100 × 20 leaf — with `with_spacing(0.0)` in a **zero**-wide box. Bases are
      `200` and `100`, the overflow is `300`, the weight is `300`, and the shares
      are `200` and `100`, so the un-clamped results would be `0` and `0`; **`80`
      and `30` are what come out**, because `clamp_axis` returns the minimum.
      **The row is `110` long inside a box of zero**, so both are at their floors
      and the remaining `190` of overflow is left unallocated — which is §
      *Overflow*'s case 2 in one assertion.
    - **`an_unbounded_main_axis_never_shrinks`** — `Constraints::loose(Size::new
      (0.0, 0.0))` and a `Stack`'s unbounded offer, over a container holding a
      leaf: the children keep their content sizes and the container's own rect is
      its content, **because `rigid_total + gaps - available_main` is exactly
      `0.0` on the unbounded branch** rather than because a test asks for it.
    - **`a_non_finite_shrink_factor_is_handled_by_the_clamp_and_not_by_a_panic`**
      — **two containers of base `100` each**, in a **60**-wide box, the first
      with `with_shrink(f32::INFINITY)` and the second with `with_shrink(0.0)`:
      the total weight is `inf`, `inf / inf` is a `NaN`, `clamp_axis` passes it
      through as the **minimum**, and the first lands at **`0`** while the second
      keeps **`100`** — **so the row is `100` in a 60-wide box with 40 pixels
      unallocated, and that is the assertion.** Repeated with `f32::MAX`, which
      `MAX * 100.0` does not overflow at these magnitudes. **"It does not panic"
      would be the weaker claim** and is not what this test asserts.
    - **`shrink_does_nothing_on_the_cross_axis`** — a `Row` whose children are
      **140** tall in a **100**-tall box: heights unchanged at **140**, and the
      child's `LayoutState::clip` is the intersection. The mirror of
      `an_overflowing_child_keeps_its_size_and_computes_a_clip_rect`.
    - **`a_shrunk_row_is_shorter_than_it_was_and_never_shorter_than_the_sum_of_the
      _minima`** — the row-level property, over the containment sweep's
      configurations: for each one, the total occupied main extent after
      `fn shrink` is **`<=` the total before it** and **`>= the sum of every
      item's main-axis margins plus its declared minimum.** **The second half is
      what catches a share that eats a margin** — a margin that was shrunk would
      put the total below the first clause's floor of `Σ margins`, and
      `margin_and_shrink_compose_in_one_pass` would catch it too; **this test is
      the one that holds when nothing else in the case is a shrinkable child.**
    - **`the_default_shrink_factor_is_one`** — `LayoutState::new().shrink() ==
      1.0` and `set_flex` does not move it. **The initial-value test**, the same
      shape task 34's `DEPTH_BITS` test is.

    **Property — two, both in `chart.rs`'s swept shape.**

    - **`every_flex_child_either_fits_its_parents_box_or_is_clipped_to_it`** —
      **the required containment property.** It sweeps **exhaustively over
      `FlexDirection` in `[Row, Column]` and a child count in `1..=6`** — twelve
      cases — **and varies the geometry with an inline seeded LCG in
      `chart.rs`'s `every_segment_quad_is_convex_over_two_hundred_thousand
      _geometries` shape**, drawing for each child a declaration (bare `LayoutState
      ::new()`, `Constraints::tight(..)`, `Constraints::loose(..)`, or one whose
      minimum is drawn **above** the box), a `Margin` from `{0, 4, 12}` on each
      side, and a shrink factor from `{0.0, 1.0, 4.0}`; and for the container a
      `spacing` from `{0.0, 8.0}` and a box from `40.0` to `400.0`. **No seed for
      the discrete cases, because a full sweep of small integers is strictly
      stronger than a sample of it.** Each case asserts, on the rects
      `layout_constraints` returns: **one rect per child, in order**; and **per
      child, either the rect is inside the parent's box on all four edges to
      `LAYOUT_EPSILON`, or the child's `LayoutState::clip` is the parent's box** —
      **the disjunction is the honest form and the name says so**, because a child
      whose declared minimum exceeds the box is **larger than its box by design**
      (`clamp_axis`'s "minimum wins", which `a_child_is_clamped_by_its_declared
      _minimum` already pins) and the guarantee that matters is that it is
      **clipped to the parent**. **`const LAYOUT_EPSILON: f32 = 1e-3;` in the
      test module**, doc-commented with the arithmetic that forces it — a child's
      far edge is `cursor + margin + size` summed in one rounding and the parent's
      is `box` in another, differing by at most a few `f32` ulps, of the order of
      `1e-7` pixels at these magnitudes — **and the epsilon is stated rather than
      absorbed, because a tolerance with no arithmetic behind it is a test that
      passes for the wrong reason.**
    - **`shrink_never_oscillates_and_never_produces_a_negative_size`** — **the
      required degeneracy test**, in two halves. **Half one, monotonicity:** the
      same tree is laid out at **descending** box widths — `400.0, 300.0, 200.0,
      120.0, 60.0, 20.0` — and for every child every recorded main extent is
      asserted **non-decreasing** as the box shrinks. **A hand-rolled shrink that
      reallocates what a previous round gave back produces a child that **grows**
      when the box shrinks, and that is the only way to see it from outside the
      pass**, because the pass is called once per layout and its rounds cannot be
      observed directly. **Half two, non-negativity and finiteness:** over the
      whole of the containment sweep **plus** the degenerate inputs — every
      `shrink` of `0.0`, `f32::INFINITY` and `f32::MAX`, a base of `0.0`, a child
      count of one, a `spacing` of `0.0` — every recorded rect satisfies
      `rect.size.width >= 0.0`, `rect.size.height >= 0.0`, and both components of
      `rect.origin` are `is_finite()`. **And the sweep is asserted non-vacuous**:
      `assert!(shrunk_at_least_once, …)`, so the monotonicity half cannot pass
      because nothing ever shrank.

    **The gap, the basis and the integration — three.**

    - **`spacing_is_the_gap_between_adjacent_children_and_a_stack_ignores_it`** —
      the same `with_spacing(12.0)` on a `row`, a `column`, a `stack` and an
      `absolute`: **the first two separate the children, the last two do not.**
      **The second half exists because `with_spacing` on a `Stack` is silently
      ignored today and no test says so**, which is `.ai/NEVERAGAIN.md`
      § *A position API that only one parent mode reads* with nothing standing
      between it and the next caller.
    - **`the_declared_constraints_are_the_flex_basis`** — **the base-size
      decision, by value, and the test that makes § *`flex-basis`*'s argument
      falsifiable.** A `row` with `with_spacing(0.0)` in a **100**-wide box holds
      **two containers, both declaring
      `Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY)`** — the widest
      declaration there is — **holding leaves of 200 × 20 and 60 × 20
      respectively**. `rigid_total` is `260`, the overflow is `260 - 100 = 160`,
      the weight is `200 + 60 = 260`, and the shares are
      `160 × (200 / 260) ≈ 123.077` and `160 × (60 / 260) ≈ 36.923`, so the two
      children are **≈ 76.923** and **≈ 23.077** wide. **Both are asserted to
      `LAYOUT_EPSILON`, and the expected values are written out in the test's
      comments** so a reader can recompute them. **The assertion is the one that
      matters: the second child shrinks from its content size of 60, not from
      0.** Weighting by `declared` would give both children a base of zero, the
      total weight would be `0.0`, `fn shrink` would return at its
      `if total <= 0.0 { return; }`, and **both would keep `200` and `60`** — so
      this test fails on the alternative implementation and not only on a
      mis-weighted one.
    - **`margin_and_shrink_compose_in_one_pass`** — the integration test, and it
      is why the list ends with an integration test rather than with the
      property tests: a `row` with
      `with_spacing(10.0)` in a **150**-wide box holds **three containers, none
      of them declaring a tight size**, so **all three of the components are live
      at once**. Each holds one leaf; `shrink` is left at its `1.0` default
      except on the second, which is set to `2.0`:

      | | declares | margin | leaf | base | occupied after |
      |---|---|---|---|---|---|
      | 1 | `Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY)` | `{ left: 8.0, right: 4.0 }` | 100 × 20 | `100` | `70 + 12 = 82` |
      | 2 | `Constraints::new(0.0, 120.0, 0.0, f32::INFINITY)`, **holding no leaf** | `{ left: 6.0 }` | — | `0` | `0 + 6 = 6` |
      | 3 | `Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY)` | none | 60 × 20 | `60` | `42` |

      `rigid_total` is `112 + 6 + 60 = 178`, the gaps are `20`, the overflow is
      `198 - 150 = 48`, and the total weight is `100 + 0 + 60 = 160` **because the
      second child's base is zero and `2.0 × 0.0 = 0.0` therefore contributes
      nothing** — **§ *`flex-shrink`*'s decision 7 in a single number.** The shares
      are `48 × (100 / 160) = 30.0` and `48 × (60 / 160) = 18.0`, so the two
      shrinkable children land on `70 + 12 = 82` and `42`, and the contentless
      one keeps its `0 + 6 = 6`.

      **Every rect is asserted by value: origins `8`, `98` and `108`, widths
      `82`, `6` and `42`, and the last child's far edge at exactly `150`** — the
      box's own width, with `used = 82 + 6 + 42 + 20 = 150` and `extra = 0.0`.
      **One set of numbers therefore carries margin's leading offset, margin's
      trailing space being consumed rather than shrunk, the gap, the shrunk
      children's tight main constraints, the zero-base child keeping its margin,
      and the row landing on the box exactly** — and a change to either component
      that the other does not mask fails here rather than in two
      single-component tests that each pass.

    **Plus three doctests**, which `cargo test` counts in the doctest total and
    which `developer.md` § *API design* requires: on **`Margin`** (quoted in
    requirement 1), on **`LayoutState::with_margin`**, and on
    **`LayoutState::with_shrink`** — the last two in the shape the module's own doc
    example at the top of `layout.rs` and `mark_dirty`'s already use, each
    building a small `Arena` through `node::create` and asserting the rects by
    value.

15. **Row `L7` is amended, dated, attributed, and not closed.**
    `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* carries
    a dated note recording, **in this order and one clause at a time**:

    - **margin — closed.** A `Margin` exists beside `Padding`, is an input on
      `LayoutState`, and is read by `LayoutMode::Flex` in full and by
      `LayoutMode::Stack` and `LayoutMode::Absolute` for its leading edges;
      margins do not collapse.
    - **`flex-shrink` — closed.** A factor exists on `LayoutState`, defaults to
      `1.0`, is weighted by the child's own size, and is clamped by the child's
      declared minimum; a growing child does not shrink.
    - **`flex-basis` — still absent, deliberately, with the reason.** The declared
      basis is `Constraints`, the resolved base is `desired`, and a growing child's
      basis is already zero in `arrange_flex`. **And, per § *`flex-shrink`*'s
      decision 7, the base of every shrinkable child in this crate is already its
      content size** — so the row's clause names a field whose value the pass
      already computes. **The row's own second sentence — that `Padding` does
      exist — stays exactly as it is.**
    - **cross-axis gap — narrowed.** `spacing` is documented as the gap between
      adjacent children rather than a main-axis-only value; **a second,
      independent gap parameter is still absent**, and the `Flex.wrap` task owns
      it. **If task 52's sentence is in the row, it is quoted, not restated.**

    **Its evidence column is rewritten from line numbers to symbols** — `Margin`
    and `LayoutState::margin`, `Padding::inset` and `fn arrange`'s two padding
    steps, `fn shrink`, `LayoutState::shrink`, `FlexConfig::spacing` and the two
    tests that pin who reads it — **because a line number into `ui/src` is stale
    the moment the tree moves** and `AGENTS.md` § *Rust* records the rule. **The
    row is not deleted and not marked closed**, its Severity stays **Medium**,
    and **its Blocks column keeps *"Dense settings rows that must not overflow"* —
    because no settings row exists in the demo to be fixed, and closing a row
    whose blocked item is undelivered is the defect
    `DEMO_APPLICATION.md` § *Corrections to the second gap table* exists to
    prevent.** **No other row in either table changes.**

16. **`doc/ui/IMPLEMENTATION_STATE.md` gains the record**, and the suite, the six
    pages and the frame rate are all produced. The record carries: a task-table
    row for **48** naming the file, its review count and its waivers-or-none, and
    its frame rate; a task section carrying **the four decisions and the reason
    for each**, **the composition order in one line**, **`flex-basis` declined by
    argument**, **the shrink base-size decision**, **the overflow policy in three
    cases**, **the `margin` / `shrink` / `flex-basis` / cross-axis outcome per
    clause of row `L7`**, **that no margin and no shrink factor is set anywhere in
    the demo** and the six-fact mechanism that keeps the pages pixel-identical,
    the **test count before and after**, the **frame rate for all six pages**, and
    **the honest limit in the section's own register** — *the vocabulary is
    unit-tested over a swept space and no node in either binary sets a margin or a
    shrink factor, so nothing here has been seen laid out on screen, and this
    task's evidence is that the arithmetic is right rather than that it looks
    right.* `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`task-sequence.md` § *State*); it points at the code. `Current position` is
    updated to name task 48. **Then, from `ui/`:** `cargo fmt --check`;
    `cargo build --all-targets --all-features`;
    `cargo clippy --all-targets --all-features -- -D warnings`;
    `cargo test --all-features` with the three per-binary counts pasted and
    **every one of requirement 14's twenty-six tests present by name**, against a
    baseline of **1894 (1450 + 224 + 220)**, **with no test deleted, renamed away
    or weakened and `layout_walk_cost` still `#[ignore]`d and unmodified**;
    `cargo doc --no-deps` clean; and `cargo audit` **recorded as not installed on
    this host rather than passed**.

## Acceptance Criteria

- [ ] **`Margin` exists, is a second type rather than a reuse of `Padding`, and
      lives on the child.** `rg -n 'pub struct Margin' ui/src/ui_core/src/layout.rs`
      shows the declaration with its four `pub f32` fields, and
      `rg -n 'pub struct Padding' ui/src/ui_core/src/layout.rs` shows a **separate**
      declaration. **`rg -n 'impl From<Padding> for Margin|Deref' ui/src/ui_core/
      src/layout.rs` returns nothing**, so the compiler is what stops a caller
      passing one where the other is meant. `rg -n 'margin' ui/src/ui_core/src/
      layout.rs` shows `LayoutState`'s field, `with_margin`, `set_margin`,
      `margin()`, `Margin::ZERO` as `LayoutState::default`'s value, `FlexItem`'s
      field, and the two `FlexItem` constructions in `arrange_flex` — **and
      `set_margin` calls `mark_dirty()`**, because the dirty flag is the only
      thing that makes the change visible before the next pass.

- [ ] **The property test exists, sweeps, and its tolerance is justified.**
      `every_flex_child_either_fits_its_parents_box_or_is_clipped_to_it` sweeps
      `FlexDirection` in `[Row, Column]` and a child count in `1..=6`
      **exhaustively and with no seed for the discrete cases**, draws each child's
      declaration, margin and shrink factor and the container's spacing and box
      from an inline LCG in `chart.rs`'s shape, and asserts one rect per child in
      order and, per child, **inside the parent's box on all four edges to
      `LAYOUT_EPSILON = 1e-3` or clipped to it**.
      **`LAYOUT_EPSILON`'s arithmetic is in its own doc comment** — a summation
      order difference of a few `f32` ulps, of the order of `1e-7` pixels.
      **Mutation evidence in the handoff, three mutations and the reason for the
      order:** (a) drop `cross_axis_leading` from `cross_offset`'s argument and
      watch the cross-axis cases fail; (b) replace `bounding_box`'s
      `corner.x + placement.trailing.width` with `corner.x` and watch
      `a_trailing_margin_on_the_only_child_is_included_in_the_containers_
      measured_size` fail — **which is the one that proves requirement 8's field
      is load-bearing**; (c) replace `clamp_axis(item.base - share, …)` with
      `item.base - share` and watch the minimum cases fail. **A test that has
      never failed is a hypothesis** (`developer.md` § Phase 3).

- [ ] **The degeneracy test exists, and both halves can fail.**
      `shrink_never_oscillates_and_never_produces_a_negative_size` lays one tree
      out at **descending** widths `400.0, 300.0, 200.0, 120.0, 60.0, 20.0` and
      asserts every child's main extent is **non-decreasing** as the box shrinks,
      and over the whole sweep plus `shrink` of `0.0`, `f32::INFINITY` and
      `f32::MAX`, a base of `0.0` and a child count of one, asserts every rect's
      size is `>= 0.0` and both origin components are finite. **Mutation
      evidence:** change `item.main = taken` to `item.main -= taken` and watch the
      monotonicity half fail **on the second width** — **that mutation is the one
      a reviewer should try first, because it is the classic hand-rolled-flexbox
      failure and nothing else in the suite distinguishes it**; then change
      `clamp_axis` to `f32::clamp` and watch the minimum cases fail or panic.
      **The sweep asserts `shrunk_at_least_once`**, so neither half can pass
      vacuously.

- [ ] **Margin's three components are pinned by value, not by prose.**
      `a_leading_margin_pushes_a_child_past_its_siblings_start` and
      `a_trailing_margin_pushes_the_next_sibling` assert the second child's origin
      by value; **`two_adjacent_margins_do_not_collapse` asserts a 16-pixel gap**
      from two 8-pixel margins, so a future task that implements collapsing must
      break it deliberately; `padding_is_applied_before_margin_and_the_two_add`
      asserts `(20.0, 20.0)` from `Padding::all(12.0)` plus `Margin::all(8.0)`;
      `a_margin_offsets_an_absolutely_placed_child_from_the_position_it_declared`
      asserts `(18.0, 18.0)`; `a_margin_moves_a_stacked_child_off_the_parents_
      origin` asserts the first child is still at `(0, 0)` and the second at
      `(6, 6)`; and `a_margin_on_the_cross_axis_centres_a_child_inside_its_own
      _margins` asserts the centre offset is **4** pixels. **Mutation evidence:**
      swap `main_axis_leading` for `cross_axis_leading` and watch the first fail;
      delete the `item.main = item.base` line and watch the third fail.

- [ ] **`bounding_box` counts a trailing margin, which is the one thing only it
      can see.** `a_trailing_margin_on_the_only_child_is_included_in_the_
      containers_measured_size` asserts the container's width and height are
      content **+ 8** each. `rg -n 'trailing' ui/src/ui_core/src/layout.rs` shows
      `Placement`'s field with its doc comment, `arrange_flex`'s push filling it
      from the main- and cross-axis trailing margins, `arrange_stack`'s and
      `arrange_absolute`'s pushes filling it with `Size::ZERO`, and
      `bounding_box`'s two `max` calls reading it. **And `git diff` over
      `layout.rs` shows no change to `layout_walk_cost`**, because the two added
      adds are inside the loop that harness measures and modifying the harness
      would make `IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why*
      unreproducible.

- [ ] **`flex-shrink`'s default, its weighting and its floor are pinned by five
      tests.** `the_default_shrink_factor_is_one` asserts
      `LayoutState::new().shrink() == 1.0` and that `set_flex` does not move it;
      `shrink_weights_by_the_declared_size` asserts the worked example's rects
      **by value** — a 220- and a 110-wide child in a 165-wide box shrinking to
      **110** and **55** — with the arithmetic in the test's comments;
      `an_item_is_never_shrunk_below_its_declared_minimum` asserts an 80-pixel
      declared minimum in a zero-width box; `a_non_finite_shrink_factor_is_
      handled_by_the_clamp_and_not_by_a_panic` asserts the **defined outcome** for
      `f32::INFINITY` and `f32::MAX`, not merely the absence of a panic; and
      `a_shrunk_row_is_shorter_than_it_was_and_never_shorter_than_the_sum_of_the
      _minima` is the row-level property.
      **Mutation evidence:** weight by `item.shrink` alone and watch
      `shrink_weights_by_the_declared_size` fail; change the default to `0.0` and
      watch `the_default_shrink_factor_is_one` fail **and**
      `flex_zero_does_not_mean_immovable` fail.

- [ ] **Grow and shrink are one exclusion, not two mechanisms.** `fn distribute`
      is **byte-for-byte unchanged** — `git diff` shows no hunk in it — and
      `a_growing_child_does_not_shrink` asserts that a child with
      `with_flex(1.0)` **and** `with_shrink(4.0)` in a box too small gets its
      share, is `>= 0.0`, and leaves the row exactly `available_main` long.
      **The reason is in `shrink`'s doc and in `with_flex`'s:** a growing child's
      base is zero, so there is nothing to take from it. `flex_zero_does_not_
      mean_immovable` asserts the **other** half — `flex: 0.0` with the default
      `shrink` **does** shrink, which is CSS's `flex: 0` behaviour and a
      divergence this crate documents rather than adopts silently.

- [ ] **`flex_basis` does not exist, and its absence is argued in three places.**
      `rg -c 'flex_basis' ui/src/ui_core/src/layout.rs` returns **nothing**, and
      `rg -n 'the basis|basis a flex' ui/src/ui_core/src/layout.rs` shows the two
      doc sentences — `LayoutState::with_constraints`'s and
      `LayoutState::with_flex`'s. `shrink_weights_by_the_declared_size` and
      `the_declared_constraints_are_the_flex_basis` assert the behaviour the
      argument describes, and `a_growing_child_does_not_shrink` asserts the
      zero-basis case. **A reader who disagrees with § *`flex-basis`* can break
      any of those three and see the claim fail** — which is what makes the row's
      `flex-basis` clause an argument rather than an omission.

- [ ] **Overflow has a defined outcome and a test for each of its three cases.**
      `shrink_does_nothing_on_the_cross_axis` asserts heights unchanged at 140 in a
      100-tall box with the child's `clip` the intersection;
      `an_unbounded_main_axis_never_shrinks` asserts a `loose` offer and a
      `Stack`'s unbounded offer leave the children at their declared sizes;
      `a_shrink_factor_of_zero_leaves_the_row_overflowing_exactly_as_it_did`
      asserts the children keep their sizes and the row runs past the box. **And
      `with_shrink`'s doc, `LayoutMode::Flex`'s variant doc and `arrange_flex`'s
      doc each say that nothing is scaled to fit** — `rg -n 'scale' ui/src/
      ui_core/src/layout.rs` returns nothing new, because a layout pass that
      rescaled children would break the module doc's *"asking and placing are the
      same code"*.

- [ ] **`spacing` is documented as the gap between adjacent children, and who
      reads it is a test rather than a sentence.** `rg -n 'main axis' ui/src/
      ui_core/src/layout.rs` shows **no remaining scoping of `spacing` to the main
      axis**; the four doc sites in `layout.rs` and `set_flex_config`'s in
      `container.rs` say *"between adjacent children"* and name `LayoutMode::Grid`
      if task 52's sentence is present. **`spacing_is_the_gap_between_adjacent
      _children_and_a_stack_ignores_it` asserts the same `with_spacing(12.0)`
      separates children in a `row` and a `column` and does nothing in a `stack`
      or an `absolute`** — **the second half is a fact no test covered before this
      task**, which is `.ai/NEVERAGAIN.md`
      § *A position API that only one parent mode reads* with nothing standing
      between it and the next caller. **No `FlexConfig` field is added, no getter,
      and `MainAxisAlignment` and `CrossAxisAlignment` are untouched** —
      `git diff --stat` shows `FlexConfig` changed in comments only.

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: in `layout.rs` — margin:
      `a_leading_margin_pushes_a_child_past_its_siblings_start`,
      `a_trailing_margin_pushes_the_next_sibling`,
      `a_trailing_margin_on_the_only_child_is_included_in_the_containers_
      measured_size`, `two_adjacent_margins_do_not_collapse`,
      `a_margin_is_clamped_to_zero_when_it_is_stored`,
      `padding_is_applied_before_margin_and_the_two_add`,
      `a_margin_offsets_an_absolutely_placed_child_from_the_position_it_declared`,
      `a_margin_moves_a_stacked_child_off_the_parents_origin`,
      `a_margin_on_the_cross_axis_centres_a_child_inside_its_own_margins`,
      `a_margin_is_not_read_by_a_grid_cell` *(conditional on `arrange_grid`
      existing; if it does not, the handoff says the test is absent and why)*;
      `flex-shrink`: `shrink_weights_by_the_declared_size`,
      `a_shrink_factor_of_zero_leaves_the_row_overflowing_exactly_as_it_did`,
      `a_row_whose_children_all_declare_a_tight_size_does_not_shrink`,
      `a_growing_child_does_not_shrink`, `flex_zero_does_not_mean_immovable`,
      `an_item_is_never_shrunk_below_its_declared_minimum`,
      `an_unbounded_main_axis_never_shrinks`,
      `a_non_finite_shrink_factor_is_handled_by_the_clamp_and_not_by_a_panic`,
      `shrink_does_nothing_on_the_cross_axis`,
      `a_shrunk_row_is_shorter_than_it_was_and_never_shorter_than_the_sum_of_the
      _minima`, `the_default_shrink_factor_is_one`;
      property: `every_flex_child_either_fits_its_parents_box_or_is_clipped_to_it`,
      `shrink_never_oscillates_and_never_produces_a_negative_size`; and
      `spacing_is_the_gap_between_adjacent_children_and_a_stack_ignores_it`,
      `the_declared_constraints_are_the_flex_basis`,
      `margin_and_shrink_compose_in_one_pass` — **twenty-six in all**, **less
      `a_margin_is_not_read_by_a_grid_cell` if `arrange_grid` does not exist when
      this task lands, which the handoff states**; **plus three doctests**, on
      `Margin`, on `LayoutState::with_margin` and on `LayoutState::with_shrink`.
      **Against the measured baseline of 1894** (1450 `ui_core` + 224 `ui_demo` +
      220 doctests): the three counts are pasted, and **the arithmetic is
      stated here so "green" means a number rather than a feeling** —
      **1450 + 26 = 1476** in `ui_core`, **224 unchanged** in `ui_demo`,
      **220 + 3 = 223** doctests, **1476 + 224 + 223 = 1923**, which is
      **1894 + 29**, and `layout_walk_cost` is still the **one** ignored test.
      **The 26 is conditional in exactly one place**: `a_margin_is_not_read_by_a
      _grid_cell` exists only if `arrange_grid` exists when this task lands, and
      **if it does not, `ui_core` is 1475 and the total is 1922, and the handoff
      says which side of task 52 it was written on.** **No test was deleted,
      renamed away or weakened.** `cargo fmt --check`, `cargo build --all-targets
      --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`
      and `cargo doc --no-deps` clean. **`layout_walk_cost` is still `#[ignore]`d
      and unmodified** — `git diff` over `layout.rs` shows no change to it.
      `cargo audit` is **recorded as not installed on this host, not passed.**

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
        34 to 39 inherited and what `IMPLEMENTATION_STATE.md`
        § *Task 24.1 — what it decided, and what it found* records as the one
        thing two captures of an unchanged frame differ in.
      - **The mechanism is six facts, and the first two are greps whose results
        are part of this claim rather than a remark:**
        1. **`rg -c 'set_margin|with_margin' ui/src/ui_demo/src/main.rs` returns
           nothing** — no node in either binary sets a margin, so `Margin::ZERO` is
           every node's margin and **`Margin::ZERO` contributes `0.0` to every sum
           it appears in.**
        2. **`rg -c 'set_shrink|with_shrink' ui/src/ui_demo/src/main.rs` returns
           nothing** — so every node takes the `1.0` default, and the whole
           question is whether the pass *does* anything.
        3. **Every child of every demo flex container declares
           `Constraints::tight(..)`**, and **`fn clamp_axis` returns `min`
           whenever `min > max`** — which is the crate's single documented clamp.
           A child whose declared minimum is its declared maximum therefore
           **cannot shrink at all**: `clamp_axis(base - share, w, w) == w` for any
           share, **and § *`flex-shrink`*'s decision 7 says the same thing from the
           other side — a tight declaration is a floor on both sides.** The three
           flex containers are the card of pads, the tab bar and the text column,
           and their children are three `PAD_SIZE` pads, six tab buttons sized by
           `Button::content_size`, and the labels — **all tight, and all
           childless**, so every base is zero besides.
        4. **The card of pads genuinely overflows**, so the pass is **not** simply
           skipped: `3 × 220 + 2 × 52 = 764` against an inner box of
           `776 − 24 = 752`, an overflow of **12** pixels that
           `MainAxisAlignment::Center` resolves as a start offset of `−6`.
           **This task therefore changes a number that is read on a page**, and
           the reason nothing moves is that the shrink pass allocates nothing:
           each item's share is `12 × (220 / 660) = 4`, `clamp_axis(216, 220,
           220)` returns **220**, every item is fixed at its minimum in the first
           round, and the loop returns on the second. **The test that guards this
           is `a_row_whose_children_all_declare_a_tight_size_does_not_shrink`,
           which leaves the factor at its `1.0` default** — a test with the factor
           set to zero would pass whether or not the default were `1.0` and would
           prove nothing about a caller who sets nothing.
        5. **`arrange_stack`, `arrange_absolute` and `content_size` produce
           identical numbers when every margin is `Margin::ZERO`** — the leading
           offset adds `0.0`, `bounding_box`'s trailing adds `0.0`, and
           `absolute_places_an_unpositioned_child_at_the_origin` and
           `stack_places_every_child_at_the_origin` keep their names and their
           assertions unchanged.
        6. **`spacing`'s value does not change and no second parameter is added**,
           so `alignment_spacing` is untouched and `cross_offset` gains at most a
           zero term.
      - **The rect-level half keeps its name and every one of its assertions:**
        `every_page_places_every_rect_where_the_gallery_placed_it`,
        `no_two_placed_rects_overlap`, `placed_handles`,
        `expected_placed_rect_names` and `assert_placed_handles_is_complete`. **No
        row of any of them is added or loosened**, and `ui/src/ui_demo/src/main.rs`
        has **no diff at all** — which is the other half of the mechanism.

- [ ] **The frame rate is measured on every page and reported with the script's
      own line pasted.** `.ai/tools/fps-check.sh 10 55` on the default page,
      **which is the only thing the script can do** — it takes `seconds` then
      `floor`, builds release and runs `./target/release/ui_demo` **with no
      arguments and no page**, per `IMPLEMENTATION_STATE.md` § *Current position*
      recording as the reason task 24.2's criterion 6 was amended rather than met
      by the script — and then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo
      --tab=<page>` for each of the six, with the `roados-fps` line parsed by hand.
      **Every page above the floor of 55**, and **every page expected to land
      inside the recorded 61.1–63.9 band** in `IMPLEMENTATION_STATE.md`
      § *The frame rate, measured*. **The handoff names the one place this task
      costs something and does not hand-wave it:** `fn shrink` allocates one
      `vec![false; items.len()]` per flex container whose row overflows, which on
      the demo is **the card of pads alone and is a three-element `Vec`**, and
      `bounding_box`'s loop gains two `f32` adds per placement. **Neither is free
      and both are named**, because a frame-cost regression is invisible to every
      other check here — **a still of a 4 fps application is pixel-identical to a
      still of a 60 fps one** — and the allocation is avoidable if a reviewer
      decides it should be, which is a reviewer's call and not this task's.

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **no change** to `ui/src/ui_core/src/render.rs`,
      `render/`, `paint.rs`, `batch.rs`, `node.rs`, `arena.rs`, `property.rs`,
      `animation.rs`, `input.rs`, `theme.rs`, `widgets/chart.rs`, any `widgets/`
      module other than `container.rs`, or `ui/src/ui_demo/src/main.rs` —
      **`container.rs`'s only diff is `set_margin` plus the one doc sentence
      requirement 11 names.** **And `git diff` shows no change to
      `arrange_grid`, `grid_column_width`, `grid_row_heights`,
      `LayoutMode::Grid`'s doc, or any of task 52's twenty-one tests** — **unless
      `arrange_grid` does not exist when this task lands, in which case there is
      nothing to show and the handoff says which side of task 52 it was written
      on.** `ui/Cargo.toml` and `ui/Cargo.lock` are unchanged: the approved direct
      dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, and a
      margin, a shrink factor and a weighted loop are arithmetic over `f32`,
      which is not a licence decision against GPLv3. **No `unsafe` is added**
      (`rg -c unsafe ui/src/ui_core/src/layout.rs` unchanged), **no `unwrap`, no
      `expect`, no `panic!`, no `unimplemented!`, no `todo!`**, and **the only
      `usize`-to-`f32` conversion added is through `count_to_f32`** —
      `rg -n 'as f32' ui/src/ui_core/src/layout.rs` shows the existing ones and
      nothing new. **No `pub` item is added and no existing `pub` item's
      signature changes**, so there is no semver question to raise. **Edition 2021
      and `rust-version = "1.85"` are respected** and nothing newer than it is
      used.

- [ ] **`doc/ui/PRIMITIVES_ARCHITECTURE.md` is untouched, and the claim is
      checkable rather than asserted.** `git diff --stat` shows **no change** to
      `doc/ui/PRIMITIVES_ARCHITECTURE.md`. **The reason is stated because the
      brief expected a § *Layout* that this document does not have:** the closest
      artefact is § *Widget Tree* → *Composition*, which carries the `LayoutMode`
      enum in a code block, and **this task adds no field to `LayoutMode`** —
      `margin` and `shrink` are `LayoutState` fields — so that block stays
      correct. § *Theming* → *Theme tokens* carries the `SpacingXs..SpacingXl`
      scale at 4/8/16/24/32, **which is where a caller gets a margin's value and
      is why `Margin` holds four `f32`s and takes no `ThemeToken`.** **Nothing in
      the amended row and nothing in the new doc comments contradicts the
      document**, which is the check `reviewer.md` § Phase 2 makes first.

- [ ] **Row `L7` is amended, dated, attributed, and not closed.** The row carries
      a dated note naming `TASK_UI_PRIM_48` and stating **four clauses, one at a
      time**: margin closed; `flex-shrink` closed; **`flex-basis` still absent,
      deliberately, with the reason**; cross-axis gap narrowed with the
      `Flex.wrap` task named as the owner of a second parameter. **If task 52's
      sentence is in the row, it is quoted rather than restated.** **Its evidence
      column names symbols and paths, not line numbers** — `Margin`,
      `LayoutState::margin`, `Padding::inset` and `fn arrange`'s two padding
      steps, `fn shrink`, `LayoutState::shrink`, `FlexConfig::spacing`, and the
      two tests that pin who reads it — per `AGENTS.md` § *Rust*. **The row is
      not deleted and not marked closed, its Severity stays Medium, its Blocks
      column keeps *"Dense settings rows that must not overflow"* because no
      settings row exists in the demo, and its second sentence — that `Padding`
      does exist — is unchanged. And no other row in either gap table changed.**

- [ ] **What the handoff does not claim, in those words.** It states that **no
      margin and no shrink factor is set by any node in either binary**, so **no
      node on any of the six pages has had a margin applied or been shrunk by
      anything a human declared**, and the evidence here is **that the geometry is
      right over a swept space**, not that it looks right.
      **`developer.md` § Phase 3 requires a rendering change to be seen on screen
      before it is reported as done, and this task does not change what is on
      screen** — which is what criterion 8's greps make checkable rather than a
      thing to apologise for. It also states that **`flex-basis` is declined, not
      forgotten** — the reader who wanted a `flex_basis` field should start from
      § *`flex-basis`*'s argument, and the cheapest way to disagree with it is to
      break one of the three tests it names. And it states the one place a reader
      is most likely to be misled: **`flex: 0.0` does not make a child immovable,
      and that is CSS's `flex: 0` behaviour** — a divergence this crate adopts
      deliberately, writes down in `with_flex`'s doc, and names in the test
      `flex_zero_does_not_mean_immovable`.

## Out of Scope

- **No `LayoutMode::Grid` change, and no grid algorithm.** Gap `L3` is task
  **`TASK_UI_PRIM_52`'s**, and § *Working beside `TASK_UI_PRIM_52`* is the whole
  of the coordination. **`arrange_grid`, `grid_column_width`, `grid_row_heights`,
  `LayoutMode::Grid`'s doc and task 52's twenty-one tests are untouched.** The
  **only** interaction is that `struct Placement` gains a private field, so if
  `arrange_grid` exists when this task lands its one push gains
  `trailing: Size::ZERO` — **one argument, no arithmetic, no decision.** **A
  margin is not read inside a grid cell**, and the field's doc says so and a test
  holds it, which is the remedy `.ai/NEVERAGAIN.md`
  § *A position API that only one parent mode reads* prescribes.
- **No absolute margins and no `auto` margins.** **Negative** margins are refused
  and clamped to zero at store, for `Padding::clamped`'s own reason: *"a negative
  side would otherwise pull a child out of its parent's box, and there is no
  arrangement that means it."* **An `auto` margin is refused on cost grounds and
  not on taste**: an auto **main** margin absorbs free space, which is
  `MainAxisAlignment::SpaceBetween` spelled per child, and it would have to
  interact with `fn distribute` — a fourth consumer of the free-space pool, in a
  task whose whole subject is two others. **An `auto` cross margin is centre or
  end alignment for one child**, which `CrossAxisAlignment` already is. **Both
  would be a second mechanism for something two mechanisms already do.**
  Revisit when a caller needs one child centred with its siblings at the ends.
- **No percentage resolution, for a margin or for a basis.** `Constraints` is four
  `f32`s; `f32::INFINITY` on a maximum means unbounded and **nothing in the crate
  means "a fraction of the parent"**. A percentage margin needs a parent box at
  resolution time, and `resolve_box` runs before the mode's arm — **the box is
  there, but the length type is not, and adding one is a `LayoutMode`-shaped
  promise this task does not need to make.** Revisit with a measured reason.
- **No nested-margin collapsing, and this is decided rather than deferred.** §
  *Margin*'s fourth paragraph is the whole argument: **collapsing is defined for
  block flow, CSS flexbox does not do it, and collapsing would make a node's
  measured size a function of its siblings'** — which `content_size`'s doc
  commits the crate not to do. **Does it apply here? No, and the reason is that
  its preconditions are not expressible**: this crate has two named axes, one
  line, and no block flow, so there is no "adjoining margins in normal flow" for
  the rule to be about, and no self-collapsing block or clearance for its two
  special cases. **Adjacent margins add.** `two_adjacent_margins_do_not_collapse`
  asserts it, so a future task that changes it must break the test rather than
  drift past a sentence.
- **No `flex_basis` field, no `flex-basis: auto`, no `min-width` basis, and no
  second length type.** § *`flex-basis`* is the whole of the argument and it is
  closed **by argument plus three tests**, not by a field. **The row's clause
  stays literally true** and the amendment says so.
- **No second gap parameter — no `row_gap`, no `column_gap`, no `cross_axis_gap`,
  no `GridConfig`.** § *Working beside `TASK_UI_PRIM_52`* gives the reason: **a
  one-line flex container has one line, `CrossAxisAlignment` has four variants
  and none of them distributes children along the cross axis, and
  `Flex.wrap` is declared and unhonoured** — so a second value would be a field no
  reachable arrangement can read, which is the `LayoutMode::Grid { columns }`
  failure this sequence exists to end. **What ships instead is four doc sentences
  that stop scoping `spacing` to the main axis, and one test that says which modes
  read it.** **A wrapped flex container's second gap value is the `Flex.wrap`
  task's**, and task 52 § *`Flex.wrap`* names the break rule as the hard part of
  it. Revisit with that task, not before.
- **No `Flex.wrap`, and no wrapping anywhere.** Decided by task 52 with five
  reasons, and this task does not reopen it: **`arrange_flex`'s steps 2 through 4
  are all single-line**, and honouring `wrap` means a rewrite of them, not a flag.
  **A grid's row-major flow is not wrapping** and is not described as such here
  either. What this task *does* say is that a cross-axis gap has no reachable
  effect without it, which is the reason the second parameter waits.
- **No change to `LayoutMode`, to `Constraints`, to `Rect`, to `Size`, to
  `Offset` or to `Handle`.** `FlexConfig` changes in comments only. `Padding`
  changes in comments only. `LayoutMode::Flex`'s `direction` and `wrap` are
  untouched, as is `LayoutMode::Stack`, `LayoutMode::Absolute` and
  `LayoutMode::Grid`'s `columns`. **`fn distribute`,
  `fn alignment_spacing`, `fn intersect`, `fn clamp_axis`, `fn count_to_f32`,
  `fn direction_cross_to_xy`, `fn cross_offset`, `fn resolve_box` and `fn sized`
  are byte-for-byte unchanged** — **`distribute` in particular, because grow and
  shrink are one exclusion and not two mechanisms.**
  **`arrange`'s signature, its `match` and the `if padding != Padding::ZERO`
  fast path are unchanged**, which is what keeps `layout_walk_cost`'s
  `flat-2041 clean pass` figure reproducible.
- **No scroll integration and no `List` change.** `Scroll` and `List` lay their
  items out themselves, a margin inside a scrolling container is a `Scroll`
  question, and row **`L5`** in `DEMO_APPLICATION.md`
  § *Gaps this layout exposes in `ui_core`* — *"No horizontal scrolling, no
  momentum, no snap"* — is open and is not this task's. **No item recycling, no
  windowing, and no partial measure.**
- **No `Container` change beyond `set_margin`.** `Container::new`,
  `Container::set_mode`, `Container::handle`, `Container::add_child`,
  `Container::remove_child` and `Container::paint` are untouched, and no new
  `pub` item is added to the widget.
- **No new dependency and no `unsafe`.** Per `AGENTS.md` the approved direct
  dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; **a margin,
  a shrink factor and a weighted loop are arithmetic over `f32` and need
  nothing**, so `ui/Cargo.toml` and `ui/Cargo.lock` are untouched. **Edition 2021
  and `rust-version = "1.85"` are respected** and nothing newer than it is used.
  **Zero new `unsafe` blocks**, because there is no GL, no FFI and no pointer in
  this change.
- **Found in the tree and deliberately not fixed.** `rg` over
  `ui/src/ui_core/src/layout.rs` finds **one** occurrence of any of `margin`,
  `flex_shrink`, `flex_basis`, `row_gap`, `column_gap`, `cross_axis_gap`, and it
  is a **comment** inside the padding test — *"Padding is the margin …"*. **The
  wording is confusing now that a real margin exists and it is wrong to leave it
  as a reader's first impression of the vocabulary.** **It is recorded here rather
  than fixed**, for the reason `TASK_UI_PRIM_52`'s last bullet records for the
  same class of thing: a drive-by comment edit inside a test this task does not
  otherwise touch is the `developer.md` § *Phase 2* refusal (*"Do not restructure
  what you were not asked to touch"*), and requirement 14 already adds a margin
  test in the same module that says what the difference is. **If the reviewer
  prefers the comment rewritten, it is one sentence and belongs in this task's
  diff rather than in a later one.**
- **No new page, no `--tab=` name, and no change to any of the six.** `Page::ALL`
  is unchanged and `ui/src/ui_demo/src/main.rs` has no diff — which is the other
  half of the pixel-identity criterion. **No demo change of any kind**, so
  `every_page_places_every_rect_where_the_gallery_placed_it`,
  `no_two_placed_rects_overlap`, `placed_handles`,
  `expected_placed_rect_names` and `assert_placed_handles_is_complete` keep every
  one of their rows.
