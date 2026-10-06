# TASK_UI_PRIM_45: Gap #5 — Per-Node Clipping Belongs to the Widget System, Not to the Demo's Frame Loop

## Goal

Give the **clip an owner**. `LayoutState::clip` already carries the rect a node
must be cut to, `Batch::clip` already carries it to the GPU, and `apply_clip`
already sets the scissor when it changes — **and nothing in `ui/src` connects the
first to the third.** The demo's `Demo::frame_clips` returns `None` for every
node in paint order, so the shipped frame carries no clip at all, and **nine doc
comments in `ui/src` plus one bullet in `doc/ui/IMPLEMENTATION_STATE.md` assert
that per-node clipping does not exist.** This task makes the renderer read the
node's own clip, deletes the demo's clip table, and deletes the eleven claims.

**This is the cheapest gap left, and the file says so rather than inventing work.**
The mechanism is built, tested and shipped; `apply_clip`'s own doc already reads
*"This is what makes a per-node clip possible at all"*; the operator's on-screen
defect of 2026-09-30 is recorded discharged in
`doc/ui/IMPLEMENTATION_STATE.md` § *A defect the operator found on screen, and
what it was*. **What is missing is one line of wiring, the ownership decision
that says who supplies the value, and eleven sentences that are the opposite of
the code beside them.** Roughly four fifths of the work is already done, and the
task file's honest form is *"finish it and correct the record"*, not *"build
clipping"*.

## Context

### What is in the crate at `75a896c` plus the uncommitted diff, established and not re-derived

Every row of this table was read in the session that wrote this file. **Citations
are by symbol and by file path; the tree is being modified in parallel and a line
number written today is wrong tomorrow.**

| Fact | Where |
|---|---|
| **`Batch` already carries the clip, and carries it *outside* the key.** `Batch { key, clip: Option<Rect> }`, with `BatchKey`'s fields being only `texture`, `blend_mode` and `shader`. Its doc is explicit: *"**It is not part of [`BatchKey`].** A clip is not a property of the material — every batch inside a scrolling viewport shares one — so keying on it would split a single list into one batch per command and defeat the batching. It rides along on the batch instead, and the renderer sets the scissor when the clip *changes* between batches, which is once per viewport rather than once per command."* | `ui/src/ui_core/src/batch.rs`, `Batch`, `BatchKey` |
| **`Batcher::add_clipped(command, clip)` merges only under an equal clip.** Its search predicate is `batch.key == key && batch.clip == clip`, and `Batcher::add(command)` is `add_clipped(command, None)`. A command whose key is a `BatchKey::is_singleton` — the shadow — seals the open segment and starts the next. | `ui/src/ui_core/src/batch.rs`, `Batcher::add_clipped`, `Batcher::add` |
| **`Renderer::apply_clip` exists, is correct, and says so.** It compares against a cached `applied_clip`, returns early when equal, and calls `Renderer::set_scissor`. Its own doc opens *"**This is what makes a per-node clip possible at all**"* and records the old obstruction by name: setting a scissor while recording applied it at the wrong moment and the last one won for the whole frame. | `ui/src/ui_core/src/render.rs`, `Renderer::apply_clip` |
| **`apply_clip` is already called per batch, on every path that draws.** `Renderer::draw_pass` calls it once per batch; `Renderer::draw_shadow_batch` calls it before branching into the no-blur and offscreen paths; `Renderer::bind_default_target` clears `applied_clip` and then re-applies through it. `Renderer::begin_frame` disables the scissor directly and therefore clears the cache too. | `ui/src/ui_core/src/render.rs`, `Renderer::draw_pass`, `Renderer::draw_shadow_batch`, `Renderer::bind_default_target`, `Renderer::begin_frame` |
| **`Renderer::draw_node_clipped(handle, nodes, clip)` exists and is the only recording path.** It takes the node's commands with `PaintState::take_commands` and records each through `Batcher::add_clipped`. `Renderer::draw_node(handle, nodes)` is `draw_node_clipped(handle, nodes, None)`. It returns early on a stale handle and on `!state.is_dirty()`. | `ui/src/ui_core/src/render.rs`, `Renderer::draw_node_clipped`, `Renderer::draw_node` |
| **`LayoutState::clip` is already the right value, computed by the layout pass.** `Layout::visit` computes `children_clip = intersect(clip, Some(rect))` for a node's children and `LayoutState::place(rect, clip, …)` stores the *ancestors'* intersection on the node itself. `LayoutState::clip()` is the getter, and `visit`'s cache key already compares `node.layout().clip() == clip`. **So the pass computes the intersection, the renderer has the hook, and nothing joins them.** | `ui/src/ui_core/src/layout.rs`, `LayoutState::clip`, `LayoutState::place`, `Layout::visit`, `layout::intersect` |
| **`ui_demo` supplies no clip anywhere.** `Demo::frame_clips(&self) -> Vec<Option<Rect>>` is `self.order.iter().map(|_| None).collect()`, and `Demo::draw` zips it against `self.order` and calls `Renderer::draw_node_clipped` per handle. **Its doc says the function *"survives with no decision left in it"* and calls it *"the whole of the frame's clipping, in one place"* — which is the honest description of a function that computes nothing, and is why the decision has to move somewhere that can be tested.** | `ui/src/ui_demo/src/main.rs`, `Demo::frame_clips`, `Demo::draw` |
| **The demo has no scrolling viewport left, so there is nothing whose clip was ever wanted.** The doc on `TEXT_PANEL` and the body of `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect` both record it: the list *"was the only clipped node this demo ever had and it is gone as of 2026-10-02"*. **The mechanism is present and unused in the shipped frame — which is exactly what row #5 says.** | `ui/src/ui_demo/src/main.rs`, `TEXT_PANEL`, `tests::no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect` |
| **The layout pass reaches two more roots the paint walk can therefore not reach through the gallery.** `Demo::frame` lays out `self.root`, `self.dialog.handle()` and `self.toasts.handle()`, each with `Constraints::tight(size)`. A separate pass is not a different clip; the same arithmetic runs. | `ui/src/ui_demo/src/main.rs`, `Demo::frame` |
| **MSAA 4× is on the default framebuffer only.** `render::context`'s `MULTISAMPLE_BUFFERS = 1` and `MULTISAMPLE_SAMPLES = 4` are window attributes set before context creation; `MULTISAMPLE_SAMPLES`'s own doc records that **an edge landing on a pixel boundary resolves to full coverage on one side and none on the other**, which is the arithmetic requirement 10's capture number rests on. `ShadowTarget` attaches a colour texture and nothing else, so the offscreen passes are not multisampled. | `ui/src/ui_core/src/render/context.rs`, `MULTISAMPLE_BUFFERS`, `MULTISAMPLE_SAMPLES`; `ui/src/ui_core/src/render/target.rs`, `ShadowTarget` |
| **`ShadowTarget::bind_for_write` turns the scissor off**, on the stated ground that the offscreen passes are whole-window, and its own doc names the renderer as the re-applier: *"the renderer re-applies the clip before the composite, which is the pass that puts the shadow on the screen"*. `Renderer::bind_default_target` is that re-applier and its doc records that **an earlier version claimed the clip was applied once and the composite ran with the scissor test off**. | `ui/src/ui_core/src/render/target.rs`, `ShadowTarget::bind_for_write`; `ui/src/ui_core/src/render.rs`, `Renderer::bind_default_target`, `Renderer::draw_shadow_batch` |
| **`scroll::clip_commands` drops what is wholly outside and keeps a straddling command whole**, and its doc says why trimming is not clipping: a `RoundedRect` fills its rect, a `Circle` cannot express a partial disc, and a `Text` run carries no width. `scroll::command_bounds(command) -> Option<Rect>` is public and returns `None` for a command it cannot bound. | `ui/src/ui_core/src/widgets/scroll.rs`, `clip_commands`, `command_bounds` |
| **No shadow is recorded anywhere in `ui/src/ui_demo/src/main.rs`.** `grep -n '\.shadow(' ui/src/ui_demo/src/main.rs` returns nothing. **The demo's only shadows are inside `Dialog` and `Toasts`, and both of their nodes are laid out as roots with `Constraints::tight(size)`, so their clips are the window.** Consequence in § *The shadow path, and what this task can and cannot show about it*. | `ui/src/ui_demo/src/main.rs`; `ui/src/ui_core/src/widgets/dialog.rs`, `ui/src/ui_core/src/widgets/toast.rs` |
| **Test baseline: 1894** — `ui_core` **1450**, `ui_demo` **224**, doctests **220**. Measured at `75a896c` plus the uncommitted diff, in the session that wrote this file. | `cargo test --all-features` from `ui/` |
| **The demo cannot receive a pointer event on this host.** XTEST pointer injection has never delivered one; keyboard has delivered exactly one. **No acceptance criterion here may require a pointer-driven interaction, and none does.** | `doc/ui/IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture method* |
| **`.ai/tools/fps-check.sh` takes `seconds` then `floor` and runs the binary with no arguments**, so it **cannot name a page**; per-page is `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>`. Every run is measured — `.ai/workflows/task-sequence.md` § *Gates*. | `.ai/tools/fps-check.sh` |

### Row #5, in the document's own words

`doc/ui/DEMO_APPLICATION.md` § *Library gaps*, row **#5**:

> **Clipping has no owner.** The clip rect *is* computed, carried on the batch
> and set on the GPU as a scissor — but the clip is supplied by the demo's frame
> loop for the `List` alone, so there is no per-node clipping in the widget
> system. Two doc comments assert the opposite of the code beside them.
> **Corrected 2026-10-01; the original "never set on the GPU" claim was false.**

Severity **High**. **Blocks: map viewport, scroll view clipping, card page
edges.** The *second sentence of that block of text* is this task's whole
deliverable, and so is the count in the third: **nine doc comments, not two.**

`doc/ui/DEMO_APPLICATION.md` § *Corrections to the second gap table* names the
same failure from the other side, in the bullet *"Two doc comments assert the
opposite of the code beside them"*, and closes with the diagnosis this task
exists to finish:

> **The live problem is narrower and worse than "the clip is never set":** the
> clip is supplied by the demo's frame loop for **one widget only**, so clipping
> works and is **not owned by the widget system**. That is row **#5** of the
> first table.

### The eleven claims this task deletes, each by file and symbol

Nine are doc comments in `ui/src`, one is a code comment on a constant in
`ui/src`, and one is a bullet in a document. **Requirement 8 is the work; this
list is the acceptance criterion a reviewer greps.**

| # | File | Symbol / place | The claim, and what is true instead |
|---|---|---|---|
| 1 | `ui/src/ui_core/src/render.rs` | `Renderer::set_scissor` | *"The scissor applies to the whole frame … so there is no per-node clip to hook a scissor to yet … applying it per node is deferred, because a recorded command has no scissor state of its own to carry."* **Every clause is false.** The clip rides on the batch, and `Renderer::apply_clip` sets the scissor per batch already. |
| 2 | `ui/src/ui_core/src/widgets/list.rs` | module docs, § *Clipping* | *"**Real per-node clipping is not here and no scissor is set.**"* A scissor is set. |
| 3 | `ui/src/ui_core/src/widgets/list.rs` | `List::clip_rect` | *"It is **not** applied … [`Renderer::set_scissor`] applies to the whole frame … which … records as a deferral."* It is applied — not by the list, but by the renderer's draw walk, from the node's own `LayoutState::clip`, which is the same rect this method returns. |
| 4 | `ui/src/ui_core/src/widgets/scroll.rs` | module docs, the clipping paragraph | *"`DrawCommand` has **no scissor state of its own** — [`Renderer::set_scissor`] applies to the whole frame … So a scroll cannot clip at the GL level, and nothing here pretends to."* The first clause stays true and is kept, with its reason restated; the conclusion is false. |
| 5 | `ui/src/ui_core/src/widgets/scroll.rs` | `Scroll::clip_rect` | *"**Applying this rect is that task's job** … until then the viewport's own box is the only thing keeping a scrolled list inside it."* That task is this one. |
| 6 | `ui/src/ui_core/src/widgets/scroll.rs` | `clip_commands` | *"Real clipping needs the scissor … until that scissor is applied, this function is the part of the job that can be done without one, and doing the other half wrongly would look worse than not doing it."* **The behaviour stays and the reason changes**: the scissor now cuts the straddler, which is why keeping it whole is right — trimming it would *and* the scissor would cut it twice into something neither intended. |
| 7 | `ui/src/ui_core/src/widgets/scroll.rs` | module docs, a second sentence | *"Real clipping needs the scissor; the honest thing for a function that has none is to drop what is wholly gone and leave what is partly there for the scissor to cut."* The scissor exists; the sentence's premise is withdrawn and the choice it defends is kept. |
| 8 | `ui/src/ui_core/src/widgets/image.rs` | module docs | *"[`DrawCommand`] has no scissor state of its own, which … records as a deferral to whichever task draws within a node's own bounds."* First clause kept, deferral withdrawn. |
| 9 | `ui/src/ui_core/src/widgets/image.rs` | `ImageFit::Cover` | *"…only this one is right without one, because [`DrawCommand`] has no scissor state."* `Cover`'s behaviour **does not change** — see § *What this task does not change about `ImageFit`*, because the reason it survives is arithmetic and not the absence of a scissor. |
| 10 | `ui/src/ui_demo/src/main.rs` | `TEXT_PANEL`'s doc comment | *"The height is the column's height at [`TEXT_SIZE_START`]; **the column is not clipped to it, so a larger `+` size overflows the window bottom by design.**"* **This is a code comment, not a doc comment, and it is an eleventh instance found while writing this file rather than one of the nine.** It becomes false the moment the scissor is applied, because `text_column` is a child of `text_panel` and the panel's box is the child's clip. |
| 11 | `doc/ui/IMPLEMENTATION_STATE.md` | § *Deviations from the spec, and why*, the bullet *"A node's clip rect is computed, not applied."* | Contradicted by that same file's § *A defect the operator found on screen, and what it was*, which records the discharge in detail. **The bullet is amended, not deleted** — see requirement 9. |

`doc/ui/DEMO_APPLICATION.md` § *Corrections to the second gap table* carries the
same failure **with line numbers in it** (`render.rs:1920-1925`, `list.rs:122-126`,
`scroll.rs:21`, `image.rs:117`, `image.rs:258`, plus a correction of its own wrong
citation). Requirement 9 amends that bullet too: **the count becomes nine, and
the citations become symbols**, because a line number into a file being edited in
parallel is stale within the hour — which is the rule `AGENTS.md` states for
`AGENTS.md` itself and which applies here for the same reason.

## The ownership decision

**Decision: the renderer derives the clip from the node's own `LayoutState::clip`,
and the caller may only *narrow* it. `Demo::frame_clips` is deleted.**

Five reasons. The second is the one that settles it, and the fourth is the cost.

1. **It is the shape that makes the row true rather than nearly true.** Row #5
   says *"the clip is supplied by the demo's frame loop … so there is no per-node
   clipping in the widget system."* An opt-in API leaves the supply with the
   caller and changes the sentence to *"the demo may supply a clip"*, which is the
   same defect one level down. **A caller who has to remember to clip is the
   failure mode the row is about** — it is the one that produced the operator's
   2026-09-30 report, where the list's rows drew over the panel and eighty-eight
   tests passed, because every assertion asked *what was recorded* and not *where
   it landed*.

2. **The value already exists, is already computed by a pass nobody asked for,
   and is already the right one.** `Layout::visit` computes the intersection of
   every ancestor's box and stores it, and it already uses it as a cache key, so
   the number is maintained correctly for free and would rot the moment a second
   implementation of the intersection appeared. `Scroll::clip_rect`'s own doc
   already refuses to recompute it — *"a second copy of that arithmetic would be a
   second thing to keep in step with the pass, and the wrong answer here is a
   scissor in the wrong place"*. **This task therefore adds no arithmetic: it
   consumes one value and calls one function.**

3. **Every widget in the crate that has a viewport already asks for the same
   number.** `List::clip_rect`, `Scroll::clip_rect`, `layout::intersect` and
   `LayoutState::clip` are four readings of one thing. Making the renderer the
   fourth reader is one call site; making every widget push its own clip is a
   change to every widget's paint signature, which is the change
   `developer.md` § *Phase 2* (*"No abstraction before the second use"*) and §
   *Stop conditions* both warn against in a different direction.

4. **What it changes for existing callers, weighed honestly: one line in one
   function, in the direction of correctness.** Every current call site is
   `draw_node_clipped(handle, nodes, None)` or `draw_node(handle, nodes)`, so the
   resolved clip changes from `None` to `Some(node's ancestors' boxes)` for every
   node in the frame. **The consequence is measurable and is not "nothing":** two
   things could change, and each of them is a named test that must pass before the
   change lands — the pre-flight of requirement 6 for the first, and
   `the_frame_uses_a_bounded_number_of_distinct_clips` for the second.

   - **A command that currently draws outside its ancestors' boxes would be cut.**
     § *The pre-flight, and why it runs before anything else* names the one place
     the repository's own docs say this happens (`TEXT_PANEL`).
   - **The batch count can rise, because `add_clipped`'s predicate compares
     clips.** The cost is bounded by the number of *distinct clip values* in the
     frame, which is the number of container boxes with painted children and not
     the number of nodes — a node's clip is its **parent's** box, so siblings under
     one parent share a clip and still merge into one batch. The named test
     `the_frame_uses_a_bounded_number_of_distinct_clips` puts a number on it, and
     it is the test that kills the mutation which intersects a node's *own* rect
     as well.

   Neither is a reason to choose the alternative. It is the reason the task is
   written as a gated change with a pre-flight rather than as a one-liner.

5. **The caller's parameter keeps its meaning, and narrows.** `clip: Option<Rect>`
   becomes *"an additional constraint, intersected with the node's own"* rather
   than *"the clip"*. Two reasons: a caller that must draw a widget outside its
   ancestors' boxes can still say so; and a caller can never **widen** past the
   ancestors, because widening is how an overflow escapes the box that was
   supposed to contain it. `Scroll::clip_rect`'s wording is the model — the
   intersection is the pass's arithmetic, and the caller adds to it.

**What automatic derivation does *not* decide.** It does not clip a node to its
**own** box. `Layout::visit` stores the *ancestors'* intersection on the node and
its own box on its children, so a button's focus ring, a `Slider`'s glow and a
chart's 6 px `Chart::stroke_reach` overhang are not cut by the widget's own rect —
which is what keeps `tests::the_chart_keeps_its_geometry_inside_its_own_rect`
(a half of today's `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect`)
true for a second reason. Requirement 6's `a_node_is_not_clipped_to_its_own_box`
pins that, because it is the single most tempting wrong implementation of this
task and the one that silently eats every focus ring in the crate.

**One thing this decision assumes and records rather than solves.** The clip must
be in **window** coordinates, and `Layout::Layout`'s `visit` starts every pass at
`Offset::ZERO`. That is already an assumption of the whole pipeline — every paint
rect in `ui/src` is absolute window geometry — so this task adds no new
requirement. It is written down in `LayoutState::clip`'s and
`Renderer::draw_node_clipped`'s docs because the renderer's new dependence on it
is the first place where the assumption would bite: a caller who lays out a
subtree at a non-zero origin gets the wrong scissor, and every paint rect in that
subtree is wrong too.

## The call path, end to end

Five hops, and **exactly one of them is new**:

```
1.  Layout::visit                      (ui_core/src/layout.rs)
      children_clip = intersect(clip, Some(rect));          // unchanged
      node.layout_mut().place(rect, clip, incoming);        // unchanged — stores
                                                            //   the ANCESTORS' box
2.  LayoutState::clip(&self) -> Option<layout::Rect>       // unchanged
3.  Renderer::draw_node_clipped(handle, nodes, clip)       // CHANGED (requirement 3)
      let extra  = clip.map(layout::Rect::from);           //   caller narrows
      let own    = node.layout().clip();                   //   the widget system
      let resolved = layout::intersect(extra, own)          //   supplies the rect
                             .map(paint::Rect::from);      //   -> NEW LINE
      for command in node.paint_mut().take_commands() {
          self.batcher.add_clipped(command, resolved);
      }
4.  Batcher::add_clipped(command, clip)                   // unchanged — the
      //  merge predicate compares `batch.clip == clip`      //   predicate is the
                                                            //   thing under test
5.  Renderer::draw_pass -> apply_clip(batch.clip)          // unchanged — the
      -> Renderer::set_scissor                            //   scissor is already
                                                            //   set per batch
```

**The function that changes is `Renderer::draw_node_clipped`, and the only new
public surface it needs is `layout::intersect` made public plus a
`From<paint::Rect> for layout::Rect` impl.** `Renderer::draw_node` keeps its
signature and its delegation; its *doc* changes, because "no clip" is no longer
what a `None` means.

**The order inside `draw_node_clipped` is specified because it is load-bearing on
cost.** The `!state.is_dirty()` early return stays **first**, ahead of any clip
resolution: the toast host is a clean empty node and a clip lookup for it is a
wasted arena read every frame. `Demo::frame`'s comment on that node — *"Empty and
**not dirty**, and the reason is arithmetic rather than a rendering one"*, because
the pipeline keeps no per-node command cache — is the reason the check stays
first.

## The pre-flight, and why it runs before anything else

**Automatic derivation can cut something the demo draws today. That is not a
hypothesis to reason about; it is a question with an instrument.**

`doc/ui/IMPLEMENTATION_STATE.md` § *A defect the operator found on screen, and
what it was* records the shape: *"every assertion in them asked **what was
recorded** and not **where it landed**"*, and `.ai/NEVERAGAIN.md` § *A survivor is
a missing assertion, and only a sweep finds it* is the rule. So the question
*"would anything the demo records be cut by its node's own clip?"* gets a test
rather than a paragraph, and **that test is written and run against the
unmodified tree before `Renderer::draw_node_clipped` is edited.**

- `tests::no_recorded_command_is_cut_by_its_own_nodes_clip` — for **all six**
  `Page::ALL` pages, plus **`Page::Overlays` with the dialog open and a toast
  raised**, walk `Demo::order`; for each node read
  `nodes.get(handle).layout().clip()` and each command of
  `nodes.get(handle).paint().commands()`; where `scroll::command_bounds` answers,
  assert the bounds lie inside the clip.
- **It is expected to find at least one cut, and the file says what to do about
  each.** `TEXT_PANEL`'s doc says it outright: the panel is
  `Constraints::tight(TEXT_PANEL)` at the window's own origin and
  `text_column` is its child, so **every label in the text column is clipped to
  `y < 380`**, and `TEXT_SIZE_START`'s own comment says a larger `+` size
  *"overflows the window bottom by design"*. Under this task it is cut at the
  panel's edge instead, which **is the panel's box doing what a box is for** —
  and the decision is recorded either way:
  - **A cut at a container's own edge is accepted** and the doc comment is
    corrected (requirement 8.10). It is the feature.
  - **A cut anywhere else — inside a rect, across a control, through the middle
    of a widget's own reach — is a stop condition** (`developer.md` § *Stop
    conditions*), and the fix is in **the demo's geometry**, never in the
    renderer: a container whose box is smaller than what legitimately draws under
    it is given the box it needs. Requirement 6's test is the gate on both halves.
- **The pre-flight's result is pasted into the handoff whatever it is**, on the
  unmodified tree, as the number of nodes cut per page. A pre-flight that reports
  zero is as much a finding as one that reports four, and both are recorded.

## The shadow path, and what this task can and cannot show about it

**The mechanism is already correct. This task's obligation is three things, and
none of them is "fix the scissor".**

1. **Add no writer of the scissor that forgets the cache.** `apply_clip`'s doc
   states the invariant: *"**The cache is only as good as every writer of the GL
   state it mirrors.** Anything that changes the scissor behind this function's
   back has to clear [`Self::applied_clip`] too, or the next `apply_clip` with an
   equal clip returns early and the box is never written."* This task's change
   adds **no** GL call and **no** new writer — it only chooses a different value
   to pass to a function that already existed.
2. **Make the invariant unit-testable, because nothing else about it can be.**
   `applied_clip: Option<Rect>` is a field on a type whose constructor needs a
   GL context, and `AGENTS.md` forbids a test that needs a display. Requirement 5
   extracts the cache into a private `ClipCache` in `render.rs` — **an extraction,
   not a change** — and pins the contract with two tests. What that proves and what
   it does not is stated plainly in requirement 5: it proves the cache's
   arithmetic, and it **cannot** prove that every call site honours it, because
   every call site needs a context.
3. **Say out loud that no page exercises the shadow path under a non-trivial
   clip.** `grep -n '\.shadow(' ui/src/ui_demo/src/main.rs` is empty: the demo's
   only shadows are inside `Dialog` and `Toasts`, and **both nodes are layout
   roots under `Constraints::tight(size)`, so their clips are the window.** With
   no shadow anywhere under a container box, the requirement 4 capture on
   `Page::Overlays` is a **no-regression** measurement — **AE 0**, the dialog's
   shadow and both toasts unchanged — and **not** a positive demonstration that a
   clipped shadow composites inside its scissor. This file does not claim the
   latter, per `.ai/workflows/task-sequence.md` § *Gates* (*"No evidence by
   assertion"*). The limit is recorded in `doc/ui/IMPLEMENTATION_STATE.md` in the
   register that file uses.

**And the reason the mechanism is right, quoted rather than re-derived:**
`Renderer::bind_default_target`'s doc records that *"The scissor is put back here
rather than by the caller, and that is the whole of this function's existence in
the shadow path"*, that `ShadowTarget::bind_for_write` *"turns the scissor test
**off** — it has to, because the offscreen passes cover the whole window"*, and
that an earlier version of the surrounding doc *"claimed the clip was applied
**once** for all of it. That was wrong, and the composite ran with the scissor test
off."* The operator found that on screen. **Requirement 7 leaves it alone and
asserts it did not move.**

## What this task does not change about `ImageFit::Cover`

Instances 8 and 9 assert that `Cover` is right *"because `DrawCommand` has no
scissor state"*. **The scissor now exists, and `Cover` still draws the node's own
rect — and the reason it must is arithmetic, not the absence of a clip.**

The two are the same picture *under a clip*, as `image.rs`'s module docs already
say. But "the overflowing version is right only when something clips, and wrong
in a panel that does not" remains exactly true **after this task**: a `Container`
with no `LayoutMode` that places a smaller box than its child has no clip for the
child beyond that box, and a caller that places an `ImageFit::Cover` node
directly under the root — which the demo does — has the window as its clip, and
the window does not stop an image painting over its neighbour. **So the
corrections 8.8 and 8.9 restate the reason as the geometry argument it
always was and drops the deferral clause; it does not reintroduce an overflowing
quad.** Requirements 8.8 and 8.9 say so in those words, because "the scissor
exists now, let us change `Cover`" is the sentence a reader will expect this file
to license and it does not.

## Scope, measured against `developer.md` § *Scope check`

**Nine files and three components — over both thresholds**, so this is split into
three sub-tasks per `.ai/protocols/subagents.md` § *Implementation fan-out*.
**Sequential**, because 45.2's pre-flight is the gate on 45.1, and because 45.3
amends documents whose subject is the code 45.1 and 45.2 change.

| Sub-task | Files | Components | Acceptable alone because |
|---|---|---|---|
| **45.1 — the mechanism** | `ui/src/ui_core/src/render.rs`, `ui/src/ui_core/src/layout.rs`, `ui/src/ui_core/src/batch.rs` (**tests only** — requirement 12 adds two tests beside the predicate, and no production line in that file moves) | 1: clip resolution at submission, plus its cache | `draw_node_clipped` resolves the clip itself, so the demo's `None` becomes the node's own rect **with no demo change at all** — verified by the requirement 10 capture and AE number, not by a unit test |
| **45.2 — the demo stops owning the clip** | `ui/src/ui_demo/src/main.rs` | 1: the paint walk's call and its tests | **Its own change must move no pixels**, which is the strongest acceptance test in this file: AE 0 on all six pages outside the fps band, plus the fps number, plus `every_page_places_every_rect_where_the_gallery_placed_it` and its neighbours still green |
| **45.3 — the eleven claims** | `ui/src/ui_core/src/widgets/list.rs`, `ui/src/ui_core/src/widgets/scroll.rs`, `ui/src/ui_core/src/widgets/image.rs`, `doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md` | 1: the record | Five files, at the threshold and not over; one component; and its acceptance test is **a named grep returning no instance**, which needs no build at all |

**`LayoutState::hits` is task 42's and this task does not touch it.** Task 42
adds the flag to `LayoutState`, which is the struct requirement 3 reads
`clip` from; the two are different fields, the two tasks are different files'
edits of `layout.rs` only in the sense that **if 42.1 lands first, its
`set_visible`/`hits` doc amendments and this task's `clip` accessor must not be
conflated in one commit** — `.ai/protocols/subagents.md` § *Splitting* rule 1
(file isolation) applies to the *implementation*, and `git diff --stat` is the
check. **If 42.1 has not landed, this task does not wait for it and does not
implement any part of it.**

**If the implementer finds themselves editing a file outside the three rows above,
that is a stop condition** (`developer.md` § *Stop conditions`) — in particular
`ui/src/ui_core/src/paint.rs`, `input.rs`, `node.rs`,
`ui/src/ui_core/src/render/target.rs`, `ui/src/ui_core/src/render/context.rs` and
every other `widgets/*.rs` are out. **`batch.rs` is in scope for two tests and
nothing else**: if a production line of it moves, this task has grown a component
it did not have.

## Requirements

### Sub-task 45.1 — the mechanism

1. **`layout::intersect` becomes public, and it stays the only intersection in
   the crate.** In `ui/src/ui_core/src/layout.rs`, change

   ```rust
   /// Intersects two optional clip rectangles.
   ///
   /// An unset clip means no ancestor clips the node, so the other one — which may
   /// also be unset — passes through.
   fn intersect(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
   ```

   to `pub fn intersect(a: Option<Rect>, b: Option<Rect>) -> Option<Rect>` with
   `#[must_use]`, its doc extended with **why it is public** — *"it is the
   arithmetic [`crate::render::Renderer::draw_node_clipped`] composes a caller's
   constraint with the node's own clip with, and a second copy of it would be a
   scissor in the wrong place"* — and **its existing tests keep their names and
   their assertions.** The body does not change by one character.

2. **`impl From<crate::paint::Rect> for crate::layout::Rect`** in
   `ui/src/ui_core/src/layout.rs`, beside the existing
   `impl From<Rect> for paint::Rect`. **Identity on the four coordinates, in the
   reverse direction**, and the doc says that in one sentence: *"the same four
   numbers, and the conversion is spelled out rather than a helper so that the one
   place where a clip crosses between the layout's geometry and the draw command's
   is visible in the API."* **It carries a doctest** with an `assert_eq!` on all
   four fields, in the shape of the existing `From`'s doctest.

3. **`Renderer::draw_node_clipped` resolves the clip itself. This is the whole
   mechanism.** In `ui/src/ui_core/src/render.rs`, the body becomes:

   ```rust
   pub fn draw_node_clipped(
       &mut self,
       handle: Handle,
       nodes: &mut Arena<WidgetNode>,
       clip: Option<Rect>,
   ) {
       let Some(node) = nodes.get_mut(handle) else {
           return;
       };
       // The dirty check is first, before any clip is resolved: a clean node
       // records nothing, and `Demo::frame`'s toast host is a clean empty node
       // that would otherwise cost an arena read per frame for a value nobody
       // draws with.
       if !node.paint_mut().is_dirty() {
           return;
       }
       // **The widget system supplies the clip.** `clip` is the caller's
       // additional constraint, not the clip; the node's own `LayoutState::clip`
       // is the pass's own intersection of every ancestor's box with it, and the
       // two compose through `layout::intersect` — the pass's arithmetic, not a
       // second copy of it. A caller narrows and cannot widen.
       let extra = clip.map(layout::Rect::from);
       let own = node.layout().clip();
       let resolved = layout::intersect(extra, own).map(paint::Rect::from);
       let commands = node.paint_mut().take_commands();
       for command in commands {
           self.batcher.add_clipped(command, resolved);
       }
   }
   ```

   - **`clip` keeps its name and its type.** No signature change, so no call site
     in the tree stops compiling, which is the point: the change is what the
     caller *means* by `None`, not the caller's code.
   - **The doc comment is rewritten, and it says three things the present one
     does not**: that `None` means *"no additional constraint"* and **not** *"no
     clip"*; that the node's own clip is read from
     `crate::layout::LayoutState::clip`; and that the clip is in **window**
     coordinates, which holds because `Layout::layout` starts every pass at
     `Offset::ZERO` and because every paint rect in this pipeline is absolute
     window geometry.
   - **The early returns keep their order and their reasons** — a stale handle and
     a clean node — and the existing doc lines about them stay.

4. **`Renderer::draw_node`'s doc is corrected**, because it currently delegates to
   a `None` that used to mean "the whole window" and now means "the node's own
   ancestors' boxes". One sentence, naming `Renderer::draw_node_clipped` and
   `crate::layout::LayoutState::clip`. **No code change.** `Renderer::set_scissor`
   and `Renderer::apply_clip` keep their bodies; only `set_scissor`'s stale
   paragraph is replaced (requirement 8.1).

5. **`applied_clip` becomes a private `ClipCache`, so the scissor invariant is
   unit-testable at all.** In `ui/src/ui_core/src/render.rs`:

   ```rust
   /// The scissor the renderer believes is set, so [`Renderer::apply_clip`] can
   /// skip a write that would change nothing.
   ///
   /// **This is the renderer's mirror of GL state, and it is only as good as every
   /// writer of that state.** Anything that disables or changes the scissor
   /// directly has to [`ClipCache::clear`] it, or the next `apply_clip` with an
   /// equal clip returns early and the box is never written —
   /// [`Renderer::begin_frame`] and [`Renderer::bind_default_target`] are the two
   /// such writers today.
   #[derive(Clone, Copy, Debug, Default)]
   struct ClipCache {
       applied: Option<Rect>,
   }

   impl ClipCache {
       /// Returns whether the scissor has to be written to get `clip` set.
       #[must_use]
       fn wants(&self, clip: Option<Rect>) -> bool { clip != self.applied }
       /// Records that `clip` is now set.
       fn record(&mut self, clip: Option<Rect>) { self.applied = clip; }
       /// Forgets what is set, for a writer that has changed the GL state itself.
       fn clear(&mut self) { self.applied = None; }
   }
   ```

   - `Renderer`'s field `applied_clip: Option<Rect>` becomes
     `clip_cache: ClipCache`; `Renderer::new` initialises it with
     `ClipCache::default()`. **Three call sites change** — `Renderer::begin_frame`,
     `Renderer::apply_clip`, `Renderer::bind_default_target` — and nothing else in
     the crate names the field.
   - **This is an extraction and not a change.** `git diff` on `begin_frame`,
     `apply_clip` and `bind_default_target` shows the same statements with a
     different receiver, and requirement 12's criterion says so.
   - **What the two tests prove, and what they cannot**, is written into
     `ClipCache`'s doc: they pin the arithmetic — a write is asked for when the
     clip changes, **not** when it is merely re-asked for, and **a cleared cache
     re-asks for a clip it had already applied**. They **cannot** prove that every
     caller clears the cache, because every caller is behind a GL context. That
     limit is recorded, not glossed.

6. **The pre-flight is a gate on this sub-task, not a phase of it.**
   `tests::no_recorded_command_is_cut_by_its_own_nodes_clip` — named and specified
   in requirement 12, living in `ui/src/ui_demo/src/main.rs` — **exists and its
   result is pasted before requirement 3 is edited.** It is written against the
   **unmodified**
   renderer, which is what makes its answer meaningful: on the current tree the
   clip is applied nowhere, so every cut it reports is a cut *this task would
   introduce*. **A cut at a container's own edge is accepted and requirement 8.10 is
   amended; a cut anywhere else is a stop condition and the fix is the demo's
   geometry, never the renderer's.**

### Sub-task 45.2 — the demo stops owning the clip

7. **`Demo::draw` calls `draw_node`, and `Demo::frame_clips` is deleted.** In
   `ui/src/ui_demo/src/main.rs`:

   ```rust
   /// Hands the recorded commands to the renderer, in paint order.
   fn draw(&mut self, renderer: &mut Renderer) {
       let mut nodes = self.nodes.borrow_mut();
       for handle in self.order.iter().copied() {
           renderer.draw_node(handle, &mut nodes);
       }
   }
   ```

   - **`Demo::frame_clips` is deleted with its doc comment**, all of it — including
     the paragraph recording that a `clip_for` helper *"sailed through every test,
     because the tests were calling `clip_for` and the defect was in a caller of
     it"*. **That paragraph's lesson is not lost: it is inherited by
     `Renderer::draw_node_clipped`'s doc (requirement 3), which now carries the
     whole of the frame's clipping decision and whose one caller has no decision
     left in it to get wrong.**
   - **`Demo::draw`'s own doc gains one sentence** saying that the clip comes from
     each node's `LayoutState::clip` inside the renderer and that this walk has no
     clipping decision in it.
   - **No other demo function changes.** `Demo::frame`'s paint walk, its three
     layout passes and its order are untouched, and no page is added, moved or
     resized.

8. **The ten stale claims are deleted or rewritten, one by one, and no tenth is
   left for a reader to find.** Each is a rewrite rather than a deletion, because
   **the surrounding text is mostly right and only the conclusion is stale** —
   `scroll.rs`'s module docs argue that a `DrawCommand` carries no scissor state,
   which is still true; what is false is what follows from it.

   - **8.1 — `ui/src/ui_core/src/render.rs`, `Renderer::set_scissor`.** The four
     sentences from *"The scissor applies to the whole frame"* to *"...a recorded
     command has no scissor state of its own to carry"* are **deleted**, and
     replaced with three sentences that are true: the scissor is set **per batch**,
     at submission, by `Renderer::apply_clip`; the clip rides on
     `crate::batch::Batch::clip` because a scissor is GL state rather than a
     property of a command; and `Renderer::set_scissor` is the **writer**, not the
     decision — the decision is the node's `LayoutState::clip`, made in
     `Renderer::draw_node_clipped`. The flip-the-Y paragraph above it stays
     untouched, and so does the whole `# Errors`-free signature.
   - **8.2 — `ui/src/ui_core/src/widgets/list.rs`, module docs § *Clipping*.**
     *"**Real per-node clipping is not here and no scissor is set.**"* is
     **replaced** with: the list does not scissor anything itself, and it does not
     need to — the scissor is set by the renderer from the list node's own
     `LayoutState::clip`, and `List::clip_rect` is the same rect read by a
     different route. **The half that is the list's own — `List::paint` dropping
     what is wholly outside — keeps its paragraph and its reason**, because it is
     a *saving*, not a substitute for the clip.
   - **8.3 — `ui/src/ui_core/src/widgets/list.rs`, `List::clip_rect`.** *"It is
     **not** applied"* and its two following sentences are **replaced** with:
     **the rect is applied, by the renderer, at submission, and it is applied to
     the list node's commands because that node's `LayoutState::clip` is this same
     rect.** The `# Examples` block below is unchanged and still passes.
   - **8.4 — `ui/src/ui_core/src/widgets/scroll.rs`, module docs, the clipping
     paragraph.** Keep *"[`DrawCommand`] has **no scissor state of its own**"* and
     its reason — *it carries one `x`, one `y` and a string, and no width* — and
     **delete the inference from it**: *"So a scroll cannot clip at the GL level,
     and nothing here pretends to."* Replaced with: the scissor **does** clip the
     content at the GL level, through the content node's own
     `LayoutState::clip`; what this module cannot do is **trim** a command, which
     is a different operation. The three bullets under the paragraph stay, and the
     first gains one clause: **`clip_rect` is what the renderer ends up with, read
     from the same place.**
   - **8.5 — `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::clip_rect`.** *"**Applying
     this rect is that task's job** … until then the viewport's own box is the
     only thing keeping a scrolled list inside it"* is **replaced** with: that job
     is done, by `Renderer::apply_clip` at submission, from this node's own
     `LayoutState::clip`. **The paragraph above it — the one refusing to recompute
     the intersection — is kept verbatim**, because it is now the reason the whole
     design works rather than a piece of politeness.
   - **8.6 — `ui/src/ui_core/src/widgets/scroll.rs`, `clip_commands`.** *"Real
     clipping needs the scissor, and [`Scroll::clip_rect`] is where the rect for it
     comes from; until that scissor is applied, this function is the part of the
     job that can be done without one, and doing the other half wrongly would look
     worse than not doing it"* is **replaced** with: the scissor exists and cuts
     the straddler, which is precisely why the straddler is kept whole — **a
     trimmed `RoundedRect` trimmed *and* scissored is a smaller rounded rectangle
     with its own corners twice over**, and a `Text` run has no width to trim to
     in the first place. **The three-outcome list above it and the doctest below
     it are unchanged.**
   - **8.7 — `ui/src/ui_core/src/widgets/scroll.rs`, module docs, the second
     sentence.** *"Real clipping needs the scissor; the honest thing for a function
     that has none…"* is **deleted and replaced** with the same reasoning restated
     in the present tense: the function has no scissor of its own and never will,
     because a scissor is set once for a batch; the scissor is the renderer's, it
     runs at submission, and this function's job is the part that saves work
     before then.
   - **8.8 — `ui/src/ui_core/src/widgets/image.rs`, module docs.** *"…which …
     records as a deferral to whichever task draws within a node's own bounds"* is
     **replaced** with the geometry argument of § *What this task does not change
     about `ImageFit::Cover`*, in one sentence: *"the overflowing version is right
     only when something clips, and a caller that places an image directly under
     the root has the window for a clip, which does not stop it painting over its
     neighbour."*
   - **8.9 — `ui/src/ui_core/src/widgets/image.rs`, `ImageFit::Cover`.** *"…only
     this one is right without one, because [`DrawCommand`] has no scissor state …
     records per-node clipping as a deferral"* is **replaced** with: **the quad is
     the node's own rect because a cropped quad is right with or without a clip,
     and an overflowing one is right only with one — which is a statement about the
     two pictures, not about what the pipeline can currently do.** The variant's
     behaviour is untouched and the requirement's own doctest still passes.
   - **8.10 — `ui/src/ui_demo/src/main.rs`, `TEXT_PANEL`'s doc comment.** *"The
     height is the column's height at [`TEXT_SIZE_START`]; the column is not
     clipped to it, so a larger `+` size overflows the window bottom by design."*
     is **replaced** with: the column **is** clipped to the panel's box, by the
     scissor the renderer sets from the column's own `LayoutState::clip`, so a
     larger `+` size now stops at the panel's bottom edge rather than running past
     it. **And it says which of the two was the design and which was the accident**:
     a `Text` run carries no width, so the panel's height was never going to make
     the column *wrap*; the overflow was visible because nothing cut it, and the
     cut is the panel's box doing what a box is for.

9. **The two documents are amended, dated and attributed, and the bullet about the
   clip is amended rather than deleted.**

   - **`doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why`** —
     the bullet *"**A node's clip rect is computed, not applied.**"* is
     **rewritten in place** into a **resolved** entry, keeping its heading shape
     so the bullet's history is auditable. It carries: that the deferral it
     recorded was discharged by the operator's on-screen defect of 2026-09-30 and
     again by this task; **`Batch::clip` outside `BatchKey`, and
     `Renderer::apply_clip` at submission, both already existing and both
     unchanged**; **what was missing and what this task changed — one line in
     `Renderer::draw_node_clipped`, and the deletion of `Demo::frame_clips`**;
     the decision that the renderer derives the clip from
     `LayoutState::clip` and the caller narrows it; the pre-flight's measured
     result; the batch-count bound; **and the honest limit — that no page draws a
     shadow under a container box, so the scissored-shadow composite is not
     demonstrated on screen, only asserted not to have regressed.**
   - **`doc/ui/DEMO_APPLICATION.md` § *Library gaps*, row #5** gains a dated note
     naming `TASK_UI_PRIM_45`, stating that the clip now comes from
     `LayoutState::clip` inside `Renderer::draw_node_clipped` and from nowhere
     else; **that the row's own count is corrected from two doc comments to
     nine**, plus the tenth code comment on `TEXT_PANEL`; and **that the row's
     `Blocks` column names three products, none of which is delivered by this
     task** — the map viewport is gap #1's work, scroll view clipping needs a
     scrolling widget the demo no longer has, and card page edges need the card
     and the pager. **The row's mechanism claim is discharged; the row is not
     deleted and the three blocked items stay in its `Blocks` column**, because
     § *Corrections to the second gap table* is a record of what happens when a row
     is closed by a mechanism with no consumer, and **this one has a consumer —
     the renderer's own draw walk, on every node of every frame.**
   - **`doc/ui/DEMO_APPLICATION.md` § *Corrections to the second gap table`** — the
     bullet *"Two doc comments assert the opposite of the code beside them"* gains
     a dated closing clause: **nine, not two; cited by symbol; all deleted or
     rewritten by `TASK_UI_PRIM_45`; and its own line-number citations are
     withdrawn for the same reason every other line-number citation in this
     repository is.** Its diagnosis paragraph — *"the clip is supplied by the demo's
     frame loop for **one widget only**"* — gains the sentence recording that it is
     no longer supplied by the frame loop at all.

10. **The capture method, the seed, and the number.** Recorded here in full because
    requirement 12's tests cannot produce this evidence and `.ai/NEVERAGAIN.md` §
    *An acceptance criterion that names an instrument which cannot produce the
    evidence is not met by producing the evidence another way* applies.

    - **The seed.** A **temporary** block in `Demo::new`, keyed off the
      environment variable **`CLIP_PROBE`**, in the shape
      `doc/ui/IMPLEMENTATION_STATE.md` § *A defect the operator found on screen,
      and what it was* records for `LIST_OFFSET` — *"the same six-line technique
      task 14 used and recorded"* — and for the same reason: **no injected event
      reaches this app, so without a seed the demo has nothing on screen whose
      clip is non-trivial.**
    - **What the seed builds.** One `Container` with `LayoutMode::Absolute` and a
      flat, fully opaque `background`, `border_radius` left at **zero** so the
      solid shader's corner branch is skipped and the quad's edges are hard,
      attached to an existing **`LayoutMode::Absolute`** container whose box is
      smaller than the probe's — **`text_panel` or the controls layer**, and the
      implementer picks the one whose free band the named test below clears —
      positioned so a known number of the probe's pixels lie outside that box.
      **Two constraints on the placement, both asserted by a test rather than by
      eye:** the probe's box and the **spill band** overlap nothing
      `Demo::placed_rects()` reports, and the spill band sits at `y < 680` so the
      fps readout's band is outside the crop by construction.
    - **The number, and why it is a number.** The spill band's width and height are
      **constants derived from the probe's own laid-out rect and its parent's**,
      and the expected `AE` is their product:

      ```sh
      # before: the unmodified tree, plus the seed
      DISPLAY=:0 xwininfo -root -tree | rg '"roados ui_demo"' | rg -o '0x[0-9a-f]+' | head -1
      DISPLAY=:0 magick import -window <id> /tmp/clip-before.png
      # after: the same seed on the tree with requirement 3 landed
      DISPLAY=:0 pgrep -a -x ui_demo
      DISPLAY=:0 magick import -window <id> /tmp/clip-after.png
      magick compare -metric AE -crop 1280x680+0+0 +repage \
        /tmp/clip-before.png /tmp/clip-after.png null:
      ```

      **`AE` over the `1280x680` crop must equal `SPILL_WIDTH * SPILL_HEIGHT`
      exactly** — computed from the laid-out rects and **pasted beside the
      measured number**, because a one-number criterion with a computed
      expectation is checkable and *"it looks clipped"* is not. The arithmetic
      behind "exactly" is `MULTISAMPLE_SAMPLES`'s own recorded statement: an edge
      landing on a pixel boundary resolves to full coverage on one side and none
      on the other, and with `border_radius` at zero the probe's quad has no
      antialiased corner to soften the count.
    - **The seed is reverted**, and the revert is proved:
      `rg -c "CLIP_PROBE" ui/src/ui_demo/src/main.rs` returns **0**. The seed's
      second edit — one row in `tests::undrawn_leaf_exemptions`, whose own doc
      names the nodes that draw, have a box and are not in `placed_handles` —
      reverts with it.
    - **What this cannot show, stated here rather than discovered at review.** No
      scissored *shadow*, because no page has one (§ *The shadow path*); and no
      `Page::Overlays` dialog or toast under a container box, because both are
      layout roots. **The `Page::Overlays` criterion is therefore AE 0 outside the
      fps band**, which is a no-regression measurement and is labelled as one.

11. **`clip_commands` and `List::paint` keep their behaviour.** Dropping what is
    wholly outside a viewport is a **saving** — an off-screen item costs nothing at
    all — and it is now a saving on top of a clip rather than a substitute for
    one. **No arm of either function changes; no existing test of either is
    renamed, weakened or deleted.** This is requirement 8's scope boundary: the
    task rewrites what those functions' docs claim about the *pipeline*, and
    nothing about what they do.

### Sub-task 45.2 — the tests

12. **The tests, named, with no display, no network, no filesystem and no wall
    clock** — the only kind `AGENTS.md` permits. **Each one names the mutation it
    kills**, per `developer.md` § *Phase 3* (*"A test that has never failed is not
    a test"*).

    In `ui/src/ui_core/src/layout.rs`:

    - **`a_node_is_not_clipped_to_its_own_box`** — a fixture of a parent and a
      child: the parent's `clip()` is `None` at the root and the child's is
      `Some(parent's rect)`, while the child's **own** `rect()` is strictly larger
      than that clip. **The mutation it kills:** resolving a node's clip as
      `intersect(own_clip, Some(node_rect))`, which is the single most tempting
      wrong reading of this task and would eat every focus ring in the crate.
    - **`a_node_is_clipped_to_the_intersection_of_its_ancestors_and_not_its_parents_own_box`** —
      a three-deep fixture: the grandchild's clip is the **parent's** box exactly,
      which is what makes siblings share a clip and is the whole of the
      batching-cost argument in the decision.

    In `ui/src/ui_core/src/batch.rs`:

    - **`a_batch_under_a_different_clip_does_not_merge_even_with_the_same_key`** —
      **the required one, and its mutation is the predicate itself**: delete
      `&& batch.clip == clip` from `Batcher::add_clipped`'s `find`, so two batches
      under two clips merge into one draw call and the second clip is silently
      lost for half the commands. **It states the same fact as the existing
      `commands_under_different_clips_do_not_share_a_batch`, which keeps its name
      and its assertions**, in the vocabulary of the mutation — *do not merge* is
      the predicate; *do not share a batch* is its consequence — so the two differ
      in what a reader is asked to hold rather than in what they assert. **No new
      coverage is invented to duplicate an existing test: this one exists because
      the acceptance criterion names it, and its doc comment says that**
    - **`the_same_clip_after_a_different_one_is_a_third_batch`** — `A`, then `B`,
      then `A` again is **three** batches, which is the ordering `Batcher::add`'s
      search-only-the-open-list rule implies. **Its mutation:** a predicate of
      `batch.clip != clip`, or a search over the sealed segments as well, either of
      which merges a command into a batch it must not join.

    In `ui/src/ui_core/src/render.rs`:

    - **`the_clip_cache_asks_for_a_write_only_when_the_clip_changed`** — record
      `Some(r)`, then `wants(Some(r))` is false and `wants(None)` and
      `wants(Some(other))` are true. **The mutation:** dropping the early return.
    - **`a_cleared_clip_cache_reapplies_a_clip_it_had_already_applied`** — record
      `Some(r)`, `clear()`, and `wants(Some(r))` is **true** again. **This is the
      shadow invariant in isolation and the reason requirement 5 exists.**
      **Its stated limit:** it does not prove `Renderer::bind_default_target`
      clears the cache, because that needs a context — which is why requirement 10
      captures `Page::Overlays`.

    In `ui/src/ui_demo/src/main.rs`:

    - **`no_recorded_command_is_cut_by_its_own_nodes_clip`** — **the pre-flight and
      the standing guard.** All six `Page::ALL` pages, plus `Page::Overlays` with
      the dialog open and a toast raised; every node in `Demo::order`; every
      command of its `PaintState`; every command `scroll::command_bounds` can
      bound asserted inside its `LayoutState::clip`, with `command_bounds`'s own
      `None` treated as its doc says. **Mutations killed:** any narrowing that cuts
      a rect the demo draws, and the own-rect mistake of
      `a_node_is_not_clipped_to_its_own_box` seen from the consumer's side.
    - **`the_frame_uses_a_bounded_number_of_distinct_clips`** — the distinct
      `Option<Rect>` values over `Demo::order`, per page, asserted against a named
      bound, **and the per-page count pasted into the handoff.** **The mutation:**
      resolving to each node's own rect as well, which turns forty-odd nodes into
      forty-odd clips and is invisible everywhere else. **This test is the frame
      cost of the whole task, in one assertion.**
    - **`the_clip_a_child_will_be_scissored_to_is_its_parents_box`** — for the
      demo's placed leaves, the clip the renderer will resolve equals the parent's
      laid-out rect. **The mutation:** a resolution that intersects the wrong
      ancestor, or one that is off by the `CONTENT_TOP` the placement adds at the
      position rather than to the constant.
    - **`the_chart_keeps_its_geometry_inside_its_own_rect`** — **the second half of
      today's `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect`,
      renamed, with every assertion kept**: all three `ChartType`s, twice each,
      mid-transition and settled, the `Chart::stroke_reach` allowance, and the
      eleven-pixel clearance. **The old test's first half — `demo.frame_clips()`
      returning one `None` per node — is deleted**, because **this task makes it
      false**, and it is superseded by
      `no_recorded_command_is_cut_by_its_own_nodes_clip`. **The rename and the
      deletion are stated here rather than left for a reviewer to infer from a
      diff**, and no assertion of the chart half is weakened.
    - **`the_probe_spill_band_touches_nothing_the_demo_places`** — **with the seed
      and only with the seed**, asserting that the probe's box and its spill band
      overlap no rect `Demo::placed_rects()` reports. It is what makes requirement
      10's `AE` an exact product rather than a number somebody liked.

### Sub-task 45.3 — the record

13. **`doc/ui/IMPLEMENTATION_STATE.md` gains one dated entry** under its own task
    heading, carrying: the decision and its five reasons in three sentences; the
    one line that changed, in `Renderer::draw_node_clipped`, and the fact that
    **nothing in `batch.rs`, `render/target.rs`, `render/context.rs` or `input.rs`
    moved**; `Demo::frame_clips`'s deletion; **the eleven corrected claims, listed
    by file and symbol**; the pre-flight's measured result; the distinct-clip count
    and the fps numbers; **and the honest limit in that section's own register** —
    *"no page draws a shadow under a container box, so the shadow's scissored
    composite is not demonstrated on screen; `Page::Overlays` is a no-regression
    measurement and nothing more."*
    `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`.ai/workflows/task-sequence.md` § *State*); it points at the code.

## Acceptance Criteria

- [ ] **The clip comes from the node, and the caller's parameter narrows it.**
      `git diff --stat` shows `ui/src/ui_core/src/render.rs`,
      `ui/src/ui_core/src/layout.rs` and `ui/src/ui_core/src/batch.rs` changed
      and **`ui/src/ui_core/src/render/target.rs`,
      `ui/src/ui_core/src/render/context.rs`, `ui/src/ui_core/src/paint.rs`,
      `ui/src/ui_core/src/input.rs`, `ui/src/ui_core/src/node.rs` and every other
      `widgets/*.rs` unchanged** — the shadow FBO, the multisample attributes and
      `DrawCommand` are untouched by this task, which is checkable rather than
      asserted. **`batch.rs`'s diff is `#[cfg(test)]` and nothing else** — the
      merge predicate, `Batcher::add_clipped`'s `find`, `Batch::clip`'s field and
      `BatchKey` are byte-identical, which is the criterion the two tests in
      requirement 12 pin.
      `Renderer::draw_node_clipped`'s body contains one
      `layout::intersect(extra, own)` and one `.map(paint::Rect::from)`, and
      **its signature is unchanged**, so no call site in the tree stopped
      compiling. **Its doc says that `None` means "no additional constraint" and
      not "no clip"**, in those words

- [ ] **The eleven stale claims are gone, and a named grep proves it.** This
      returns **no output**:

      ```sh
      grep -rn \
        -e 'no scissor state' \
        -e 'no scissor is set' \
        -e 'Real clipping needs the scissor' \
        -e 'Real per-node clipping is not here' \
        -e 'computed, not applied' \
        -e 'there is no per-node clip' \
        -e 'applies to the whole frame' \
        -e 'per-node clipping as a deferral' \
        -e 'until then the viewport' \
        -e 'until that scissor is applied' \
        -e 'not\*\* applied' \
        ui/src/ doc/ui/IMPLEMENTATION_STATE.md
      ```

      **It returns sixteen lines at `75a896c` plus the uncommitted diff** — quoted
      here so that "no output" means something: three in
      `Renderer::set_scissor`'s doc, one in `list.rs`'s module docs, one in
      `List::clip_rect`, three in `scroll.rs`'s module docs, one in
      `Scroll::clip_rect`, one in `clip_commands`, two in `image.rs`'s module docs
      and `ImageFit::Cover`, and two in `doc/ui/IMPLEMENTATION_STATE.md` §
      *Deviations from the spec, and why*. **And the ten `ui/src` sites are
      amended by symbol, not deleted:** the handoff lists each of the eleven by
      file and symbol as in § *The eleven claims this task deletes*, and
      **grep -n 'The scissor applies to the whole frame' ui/src/ui_core/src/render.rs
      and grep -n 'Real per-node clipping is not here' ui/src/ui_core/src/widgets/list.rs
      both return nothing**, while
      `grep -n 'This is what makes a per-node clip possible at all' ui/src/ui_core/src/render.rs`
      still shows the line it has always shown — **the mechanism's own doc was
      right and is untouched**

- [ ] **The stale bullet in `IMPLEMENTATION_STATE.md` is amended, and the two
      sections that contradicted each other now agree.** § *Deviations from the
      spec, and why* carries no bullet reading *"A node's clip rect is computed,
      not applied"*, and `grep -n 'computed, not applied' doc/ui/IMPLEMENTATION_STATE.md`
      returns nothing. **The bullet is not deleted** — it is rewritten as a
      resolved entry, in the same list, naming `TASK_UI_PRIM_45`, naming
      `Renderer::draw_node_clipped` and `Demo::frame_clips`, and carrying the honest
      limit from requirement 9. **§ *A defect the operator found on screen, and
      what it was* is not edited**: it was right when it was written, and this
      task makes it right-er, not wrong

- [ ] **`add_clipped`'s merge predicate is pinned by a named test, and the test
      can fail.** `commands_under_different_clips_do_not_share_a_batch` (its
      existing name, its existing assertions, two clips and one unclipped batch
      out of three) **and the new**
      `a_batch_under_a_different_clip_does_not_merge_even_with_the_same_key`
      between them pin both directions: two clips must not merge, and the same
      clip must. **Mutation evidence in the handoff:** delete
      `&& batch.clip == clip` from `Batcher::add_clipped`'s `find` in
      `ui/src/ui_core/src/batch.rs`, run
      `cargo test --all-features -p ui_core batch::` from `ui/`, and watch the
      count of batches in the two-clip case go from **3 to 2** with the suite
      failing; restore it and watch it pass. **A wrong predicate is the mutation,
      because a predicate that ignores the clip loses the clip for every command
      merged into the wrong batch and no other assertion in the crate can see
      it** — the clip is GPU state, and `Batch::clip`'s own doc says so

- [ ] **The pre-flight ran before the mechanism, and its result is pasted.** The
      handoff carries `cargo test --all-features -p ui_demo
      no_recorded_command_is_cut_by_its_own_nodes_clip` **run against the
      unmodified renderer**, with the number of nodes reported cut per page. **If
      that number is zero, the criterion is met and the zero is recorded; if it is
      not, every cut is listed with the decision taken on it**, and **a cut
      anywhere other than a container's own edge is a stop condition rather than
      something to fix in the renderer.** The same test, run **after**
      requirement 3, is green

- [ ] **A capture shows a node's commands clipped, with `AE` as the measure and
      the number named in advance.** On `Page::DEFAULT`, release build, with the
      `CLIP_PROBE` seed applied to **both** sides:

      ```sh
      magick compare -metric AE -crop 1280x680+0+0 +repage \
        /tmp/clip-before.png /tmp/clip-after.png null:
      ```

      **reports exactly `SPILL_WIDTH * SPILL_HEIGHT`**, computed from the probe's
      laid-out rect and its parent's and **pasted beside the measured number** —
      the crop excludes the fps band by construction, so the whole figure is the
      spill. **Every differing pixel lies inside the spill band, and none lies
      outside it**, which is what an exact figure means. The window id is re-read
      with `xwininfo -root -tree` at the time of **each** capture, `pgrep -a -x
      ui_demo` is run in the same call as each `magick import -window <id>`, and
      the commands are those of `IMPLEMENTATION_STATE.md`
      § *Verifying a change that draws — the capture method* verbatim.
      `rg -c "CLIP_PROBE" ui/src/ui_demo/src/main.rs` is **0** afterwards, and so
      is `rg -c "probe" ui/src/ui_demo/src/main.rs`

- [ ] **The other five pages and the shadow path are measured, and the shadow
      claim is the narrow one.** `Page::Overlays` **with the dialog open and a
      toast raised**: `magick compare -metric AE` over the `1280x680` crop is
      **0** — the no-regression measurement requirement 10 says it is, and **the
      handoff does not describe it as demonstrating a clipped shadow**, because
      `grep -n '\.shadow(' ui/src/ui_demo/src/main.rs` is empty and both the
      dialog's and the toasts' nodes are layout roots under
      `Constraints::tight(size)`. **`Page::pads`, `text`, `input`, `controls` and
      `data`: AE 0 over the same crop**, with the seed applied to both sides and
      the same exact spill figure inside the band on each

- [ ] **The demo's own clip table is gone and nothing of it survives in a
      weakened form.** `grep -n 'frame_clips' ui/src/ui_demo/src/main.rs` returns
      **nothing**, `grep -n 'fn clip_for' ui/src/ui_demo/src/main.rs` returns
      **nothing**, and `Demo::draw` is the six-line loop of requirement 7.
      **`tests::no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect`
      is renamed to `the_chart_keeps_its_geometry_inside_its_own_rect` with every
      one of its chart assertions kept verbatim**, its first half — the
      `frame_clips()` list — deleted because this task makes it false, and
      **its doc comment rewritten to say which half went and what replaced it.**
      **`every_page_places_every_rect_where_the_gallery_placed_it`,
      `no_two_placed_rects_overlap` and `assert_placed_handles_is_complete` keep
      their names and every one of their assertions** — which is the
      test-level statement of "this sub-task moves no pixels", and it is the same
      fact the `AE 0` above states at the pixel level

- [ ] **The batch cost is bounded by a test and by a number.**
      `the_frame_uses_a_bounded_number_of_distinct_clips` passes on all six pages
      and **the per-page count is pasted into the handoff**, beside the sentence
      explaining why it is the count of *container boxes* and not the count of
      nodes: a node's clip is its **parent's** box, so siblings under one parent
      share a clip and still merge into one batch.
      `the_frame_rate_is_measured_on_every_page` below is the frame-level half,
      and neither substitutes for the other

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: in `layout.rs` —
      `a_node_is_not_clipped_to_its_own_box`,
      `a_node_is_clipped_to_the_intersection_of_its_ancestors_and_not_its_parents_own_box`;
      in `batch.rs` — `a_batch_under_a_different_clip_does_not_merge_even_with_the_same_key`,
      `the_same_clip_after_a_different_one_is_a_third_batch`;
      in `render.rs` — `the_clip_cache_asks_for_a_write_only_when_the_clip_changed`,
      `a_cleared_clip_cache_reapplies_a_clip_it_had_already_applied`;
      in the demo —
      `no_recorded_command_is_cut_by_its_own_nodes_clip`,
      `the_frame_uses_a_bounded_number_of_distinct_clips`,
      `the_clip_a_child_will_be_scissored_to_is_its_parents_box`,
      `the_chart_keeps_its_geometry_inside_its_own_rect`.
      **Baseline 1894 (1450 + 224 + 220), and this file projects 1903** — eight new
      tests and one new doctest, plus one rename and one deletion inside a single
      test — **so a measured number other than 1903 is corrected here rather than
      argued about.** **No test was deleted, renamed away or weakened**, and the
      single deletion inside a test is the one named in the criterion above.
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is **recorded, not passed**

- [ ] **The frame rate is measured on all six pages and reported.**
      `.ai/tools/fps-check.sh 10 55` on the default page with the script's own
      line pasted rather than the rate expected, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of the
      six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `IMPLEMENTATION_STATE.md` § *Current position* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. Every page above the floor of **55**, and **the report compares each
      page against its own pre-change number, not against the other pages** —
      which is the only comparison that isolates the cost of this task, since a
      regression here is *more batches*, not more pixels, and it is invisible to
      every other check in this list

- [ ] **`ClipCache` is an extraction and not a change.**
      `git diff ui/src/ui_core/src/render.rs` shows
      `Renderer::begin_frame`, `Renderer::apply_clip` and
      `Renderer::bind_default_target` making the **same three statements** against
      a different receiver — `self.applied_clip = None` becomes
      `self.clip_cache.clear()`, and `apply_clip`'s compare-then-write becomes
      `self.clip_cache.wants(clip)` then `self.clip_cache.record(clip)` then
      `self.set_scissor(clip)`. **No GL call is added, removed or reordered**, and
      `a_cleared_clip_cache_reapplies_a_clip_it_had_already_applied` is the
      assertion that the shadow invariant survived the move

- [ ] **Row #5 is amended, dated, and its `Blocks` column is intact.**
      `doc/ui/DEMO_APPLICATION.md` § *Library gaps* row **#5** carries a dated note
      naming `TASK_UI_PRIM_45`, stating that the clip now comes from
      `LayoutState::clip` inside `Renderer::draw_node_clipped` and from nowhere
      else, **correcting its own count from two doc comments to nine** (plus the
      code comment on `TEXT_PANEL`), and stating that **none of the three items in
      its `Blocks` column is delivered by this task**: the map viewport is gap #1's
      work, scroll view clipping needs a scrolling widget the demo no longer has,
      and card page edges need the card and the pager. **The row is not deleted.**
      § *Corrections to the second gap table*'s *"Two doc comments"* bullet gains
      its dated closing clause, **with its line-number citations withdrawn**, and
      the diagnosis paragraph records that the frame loop supplies nothing

- [ ] **`LayoutState::hits` is task 42's and this task did not implement any of
      it.** `git diff` over `ui/src/ui_core/src/layout.rs` shows **no** `hits`
      field, **no** `set_hits`, and **no** change to `LayoutState::visible` or
      `LayoutState::set_visible`, whatever the state of task 42 in the tree when
      this task starts. **If 42.1 has landed, the two tasks' edits to `layout.rs`
      are in separate commits** and `git log --stat` shows it

- [ ] **No new dependency, no new `unsafe`, no new page, no change to any of the
      six.** `ui/Cargo.toml` and `ui/Cargo.lock` are unchanged — the approved
      direct dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`,
      and **per `AGENTS.md` a clipping crate for a rectangle intersection is a
      licence decision against GPLv3 that nobody has asked for.** No `unsafe`
      block is added: requirement 3 changes which `Option<Rect>` reaches an existing
      `gl.scissor` call and adds no call. `Page::ALL` still holds six names,
      `Page::DEFAULT` is still `Pads`, and `--help` is unchanged.
      `DEPTH_BITS`, `MULTISAMPLE_SAMPLES`, `MULTISAMPLE_BUFFERS`, `SOLID_BLUR`,
      `DrawCommand`'s variants and `BatchKey::is_singleton` are all untouched, and
      checkable by `git diff --stat`

## Out of Scope

- **No shadow-map depth.** The offscreen passes are coverage and a separable
  blur; a clip is a scissor and says nothing about how far a shadow reaches.
  `ShadowTarget` is not given a depth attachment by this task and
  `SOLID_BLUR`'s own threshold is not moved
- **No rounded or soft clip edges.** A scissor is a rectangle, so a scissored edge
  is a hard edge. **Rounded clipping is an SDF change and this task does not
  attempt it**: `DrawCommand::Image`'s `radius` is already documented as a **hard
  `discard`** in the fragment shader rather than an analytic edge, and requirement 8
  does not change one word of it. **If a card's page edge is wanted as a rounded
  one, that is a shader task and the scissor is orthogonal to it** — a scissor cuts
  *outside* a rect and the SDF decides what happens *inside* one
- **No clip animation, and no clip interpolation.** `Transform` is `Interpolate`-able
  and never read at draw time — that is gap #8's row and this file does not touch
  it. A clip that moved between frames would be a batch key that changed every
  frame, which `Batch::clip`'s own doc says is exactly what keying on it would do
- **No nested-clip intersection beyond what falls out.** The pass **already**
  intersects every ancestor's box and this task **consumes** that value; there is
  no second intersection pass, no nesting stack, and no per-command clip depth.
  The only new composition is requirement 3's one-line intersection of the
  caller's constraint with the node's own — and it is the same `layout::intersect`
  for a reason, not a new one
- **No per-pixel scissor, no discard-based clipping, no stencil pass.** A scissor
  test is per-fragment on the box; "per-pixel" would mean sampling the clip shape
  in the fragment shader, which is the SDF change above under another name
- **No change to what `List::paint` and `clip_commands` do** — requirement 11.
  Dropping what is wholly outside is a saving that now sits on top of a clip
- **No new widget, no new page, no new UI of any kind.** The `CLIP_PROBE` seed is
  **temporary and reverted**, and `rg -c "CLIP_PROBE" ui/src/ui_demo/src/main.rs`
  is **0** in the handoff. Nothing from requirement 10's capture survives into the
  tree
- **No change to the shadow FBO, to `bind_for_write`, or to `bind_default_target`'s
  behaviour.** Requirement 5 moves the cache out from behind the field name and
  changes no GL call. **What this task does not do is prove on screen that a
  scissored shadow composites inside its scissor**, because no page has one
- **No change to `LayoutState::visible`, `LayoutState::hits`, `hit_test_from` or
  `Focus`.** Task 42 owns all four
- **No change to the three layout passes in `Demo::frame`.** The dialog and the
  toast host are laid out as roots today and stay roots; this task only reads what
  those passes already store
- **No `ImageFit` change, and no new fit.** § *What this task does not change
  about `ImageFit::Cover`* is the whole of it