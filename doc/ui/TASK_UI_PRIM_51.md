# TASK_UI_PRIM_51: Gap `L10` — the `Polygon` Precondition, Made Checkable, and the Reversal Asked For

## Goal

Take row **`L10`** of `doc/ui/DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* and do the only two things that are
honest about it: **put the renderer's convexity precondition where a caller can
ask whether it meets it**, and **put the dated decision that keeps the
capability out of the renderer to the operator in writing**, with the evidence
assembled and the reversal specified well enough to be built if they say yes.

**This task does not teach the renderer to fill a concave polygon.** It does not
reverse the operator's decision of **2026-10-02**, and § *The decision* says so
in the first line rather than in a footnote.

## Context

### The row, quoted, and what each of its four clauses is claiming

Row **`L10`** in `doc/ui/DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* reads, verbatim:

> **`Polygon` is convex-only** — triangle fan, no ear-clipping, no stencil; the
> source names both rejected alternatives in as many words. No bezier, no fill
> rules — zero hits for `fill_rule`/`nonzero`/`even_odd` anywhere. **Note also
> that `DrawCommand::Path` is a *stroked* polyline with an explicit width**, so
> it cannot serve as a filled icon outline either.

Severity **Medium**; Blocks *"Any concave silhouette; the proximity ramp;
instrument arcs"*. § *Task structure* places **`L10`** in the **"Low / Medium"**
bucket beside `#6`, `#7`, `L7`, `L8` and `L9` — **not** in the "High" bucket
with `#2`/`L3`, `#3`, `#4`, `#5`, `L4`, `L5` and `L6b`.

**The row is four clauses wearing one title, and that is the whole difficulty:**

1. **The fan is exact for a convex polygon and for nothing else.** A capability
   claim about the renderer.
2. **There is no stencil pass.** A second capability claim about the renderer,
   for a different mechanism.
3. **No bezier, no fill rules.** **Not a convexity question at all** — it is a
   missing curve primitive and a missing winding rule, and neither is what
   `command_quads`'s fan does or does not do.
4. **`DrawCommand::Path` is stroked.** A fact about a *different* variant, true,
   and relevant to `#4` rather than to `Polygon`.

**Clause 1 is the only clause this task can act on without reversing a
decision**, and it is the clause the row is named after.

### The two dated decisions, and the third date that is not one of them

**Three dates are in play and only two of them are about ear clipping. Mixing
them up is the failure this section exists to prevent.**

- **2026-10-01 — the operator *added* the primitive.** `paint.rs`'s
  `DrawCommand::Polygon` doc says, in the variant's own words: *"The Gauge's
  needle is a filled triangle, and the operator decided on 2026-10-01 to add this
  primitive rather than settle for the outline."* **This is not a declination and
  must not be cited as one.** It is the decision that created the gap.
- **2026-10-02 — the operator *declined both renderer-side answers*.** Verified in
  three places, all consistent:
  - `ui/src/ui_core/src/widgets/chart.rs`'s module doc, in the section on the area
    fill: *"Ear clipping and a stencil pass were both considered and the operator
    declined both on 2026-10-02."*
  - `doc/ui/IMPLEMENTATION_STATE.md` § *Task 21 — what it decided, and what it
    found*, item 2 of the four operator decisions taken that date: *"Area fill is
    per-segment convex quads. `DrawCommand::Polygon` — added in task 20 for the
    gauge needle — is **convex only**: the renderer fans `n - 2` triangles, which
    is exact for a convex polygon and a wrong picture for a concave one, and the
    region under a non-monotonic line is concave. **The operator declined
    ear-clipping triangulation in the renderer.**"*
  - `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why*, in the
    entry on task 21's line and area rendering: *"**The fan is exact for a convex
    polygon and for nothing else**, so a single polygon for the whole series would
    be a wrong picture rather than a rough one."*
- **2026-10-05 — "every library gap must be closed."** `DEMO_APPLICATION.md`
  § *Operator decisions (2026-10-05)* item 2, verbatim: *"**Every library gap
  must be closed.** The 2026-10-03 decision to build the page mechanism in
  `ui_demo` and leave gaps **#3** and **#7** open in `ui_core` is **withdrawn**.
  Those two gaps are library work that blocks the demo like any other."*

**What that instruction does and does not say, read as carefully as the row
itself.** **Its explicit withdrawal targets the 2026-10-03 deferral decision, by
name, and it names gaps `#3` and `#7`.** It does not name `L10`, it does not
mention 2026-10-02, and it says nothing about any technical declination. So the
two statements *"every library gap must be closed"* and *"ear clipping was
declined on 2026-10-02"* are reconcilable, and they reconcile in **exactly one**
way: **`L10` must be closed by some means, and 2026-10-02 says which means are
excluded.**

**That reading is an interpretation and this file labels it as one.** An
operator who meant *"and every earlier technical declination with it"* would say
so, and § *The decision* is where that question is put to them with everything
it needs to answer it. **This file does not assume the answer, and it does not
build the reversal.**

### What is in the crate at `75a896c` plus the uncommitted diff, established and not re-derived

Cited **by symbol and path**, never by line number, because the tree is being
modified in parallel — the rule `AGENTS.md` § *Rust* states and
`IMPLEMENTATION_STATE.md` § *Tasks 34–40 — the mesh-rendering sequence* records
as *"Citations in these seven files are by symbol, not by line number … one line
number went stale **inside a single drafting session**."*

| Fact | Where |
|---|---|
| **`DrawCommand::Polygon { points: Vec<(f32, f32)>, color: Color }`** — two fields, and the variant's doc carries the **bold** sentence *"**Convex only.**"* and the obligation *"a caller that cannot promise convexity must decompose the shape into convex pieces itself before recording it."* | `ui/src/ui_core/src/paint.rs`, the `DrawCommand::Polygon` variant |
| **The expansion is a triangle fan from the first point.** `if points.len() < 3 { return Vec::new(); }`, then `quads.reserve(points.len() - 2)` and `for point in points[1..].windows(2) { quads.push(polygon_quad(points[0], point[0], point[1], *color)); }`. The comment above the loop is the gap row's own evidence, verbatim: *"A fan from the first point. This is exact for a convex polygon and nothing else, which is what `DrawCommand::Polygon`'s docs say it is; a concave one fans into overlapping and inverted triangles. The alternative — ear clipping, or a stencil pass — is a rasteriser with its own vertex type, and neither a gauge needle nor anything else recorded so far needs one."* | `ui/src/ui_core/src/render.rs`, `command_quads`'s `DrawCommand::Polygon` arm |
| **`Painter::polygon` and `Painter::path` are the two public drawing methods for a point list**, and `polygon`'s doc already points at the contract: *"`points` must describe a **convex** polygon, and fewer than three of them draw nothing … this method only records; it neither measures the points nor rejects them, because the recorder's contract is to store what it was handed."* | `ui/src/ui_core/src/paint.rs`, `Painter::polygon`, `Painter::path` |
| **A triangle is a quad with a duplicated corner** — `polygon_quad` emits four corners and `quad_indices` addresses `0,1,2 / 0,2,3`, so a three-point polygon is one quad whose fourth corner repeats its third. The test that pins it is `a_triangle_is_one_quad_with_its_third_corner_repeated`. | `ui/src/ui_core/src/render.rs`, `polygon_quad`, `quad_indices`, and that test |
| **`DrawCommand::Path` is stroked**: `{ points, width, color, closed }`, expanded by `command_quads` into **one `line_quad` per segment**, with `closed: true` adding the last-to-first segment and nothing else — no cap, no join. | `ui/src/ui_core/src/render.rs`, `command_quads`'s `DrawCommand::Path` arm; `ui/src/ui_core/src/paint.rs`, the `DrawCommand::Path` variant |
| **`chart.rs` solved a concave area fill caller-side, with per-segment convex quads.** Its area fill is *"**one convex quad per segment**, each dropping from its segment's two points to the plot's own bottom edge"*, and the module doc says the same. | `ui/src/ui_core/src/widgets/chart.rs`, `Chart::draw_fill`, and the module section *The area fill is per-segment quads too, and it has to be* |
| **`gauge.rs` solved a concave band caller-side, the same way.** The arc is *"**one convex four-point `DrawCommand::Polygon` per segment**, with corners on the inner and outer radii at the segment's two endpoint angles"*, and the module doc names *"both rejected alternatives"* in the section titled *The arc is a band of quads, and both rejected alternatives are measured*. | `ui/src/ui_core/src/widgets/gauge.rs`, the module section of that name |
| **The convexity rule already exists — twice, privately, and only in test code.** `chart.rs` and `gauge.rs` each carry a private `fn assert_convex(points, what)` whose bodies are the same algorithm: a cross product per consecutive triple, counting `> 0` against `< 0`, and asserting one count is zero. **Neither is reachable by a caller**, and **there is no third copy.** | `ui/src/ui_core/src/widgets/chart.rs` and `ui/src/ui_core/src/widgets/gauge.rs`, each in its own `#[cfg(test)] mod tests` |
| **Zero `fill_rule`, `nonzero` or `even_odd` anywhere in the crate.** `grep -rn 'fill_rule\|nonzero\|even_odd' ui/src/` returns nothing. | measured over `ui/src/` |
| **Zero stencil code anywhere in the crate — three hits, all prose declining it.** `paint.rs`'s `DrawCommand::Polygon` doc, `render.rs`'s `command_quads` comment, and `chart.rs`'s module doc. **`gl_attr.set_stencil_size` is never called**: `ui/src/ui_core/src/render/context.rs`'s `Context::new` sets the profile, the version, the double buffer, `set_depth_size(0)`, `set_multisample_buffers` and `set_multisample_samples` — **and nothing else.** | `ui/src/ui_core/src/render/context.rs`, `Context::new`; `ui/src/ui_core/src/paint.rs`, `ui/src/ui_core/src/render.rs`, `ui/src/ui_core/src/widgets/chart.rs` |
| **The renderer's degenerate answer is already decided and is not an error.** `command_quads` returns `Vec::new()` for a `Polygon` of fewer than three points, and the test `a_polygon_of_fewer_than_three_points_expands_to_no_quads` pins it: *"Not an error and not a panic: a point list that cannot enclose an area has nothing to draw."* | `ui/src/ui_core/src/render.rs`, `command_quads` and that test |
| **`ui_core/src/paint.rs` is a single 1 327-line file**, not a directory — `ui/src/ui_core/src/lib.rs` declares thirteen `pub mod`s and `render` is the only one with children (`blur.rs`, `context.rs`, `target.rs`). **So a new public predicate about a point list belongs in `paint.rs`, beside the doc that states the obligation.** | `ui/src/ui_core/src/lib.rs`, `ui/src/ui_core/src/paint.rs`, `ui/src/ui_core/src/render/` |
| **`ui_core` carries `publish = false`**, so a source-breaking change to a `pub` item is internal to this repository. | `ui/src/ui_core/Cargo.toml` |
| **The mesh path is specified but not landed.** `grep -c 'DrawCommand::Mesh\|MeshVertex'` returns **0** in both `paint.rs` and `render.rs`, and `ui/src/ui_core/src/render/` holds only `blur.rs`, `context.rs` and `target.rs` — **no `mesh.rs`, no `matrix.rs`, no `meshio.rs`**. Tasks 34 to 40 are written and unstarted (`IMPLEMENTATION_STATE.md` § *Tasks 34–40 — the mesh-rendering sequence*: *"Created 2026-10-05. **Nothing in it is started.**"*). | measured over `ui/src/`, and the state file by section |
| **Test baseline: 1894** — `ui_core` **1450**, `ui_demo` **224**, doctests **220**, recorded in `IMPLEMENTATION_STATE.md` § *Task 31 — what it decided, and what it found* and restated in `TASK_UI_PRIM_42.md` § *What is in the crate at `75a896c`*. **The `#[test]` attribute count observed in the tree is 1451 in `ui_core` and 225 in `ui_demo`** — `grep -rc '#\[test\]' --include=*.rs`, and the two figures differ from the passed-test counts by the one ignored test and by the doctests, exactly as `TASK_UI_PRIM_44.md` § *Context* records. **The handoff pastes the real per-binary numbers rather than picking one.** | `cargo test --all-features` from `ui/`; the attribute count by grep |
| **The demo cannot receive a pointer event on this host**, so no criterion here may require one — and none does. This task's deliverable is a pure function and two test migrations. | `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture method* |
| **`cargo audit` is not installed on this host.** Tasks 07, 08, 09, 10 and 13 each carry it as *recorded, not passed*, in the task table. | `IMPLEMENTATION_STATE.md` § *Task table* |
| **`.ai/tools/fps-check.sh` takes `seconds` then `floor` and runs the binary with no arguments**, so it **cannot name a page**; per-page is `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>`. Every run of `ui_demo` an agent launches is measured — `.ai/workflows/task-sequence.md` § Gates, *"No unmeasured run of the demo."* | `.ai/tools/fps-check.sh`; the gate |
| **`ui/Cargo.toml` and `ui/Cargo.lock` are not touched by this task**, and no dependency is proposed — see § *Out of Scope*. | `AGENTS.md` § *Rust* |

### The decision: **(D) — keep the row open, escalate in writing, and ship the enforcement instrument**

**Decision: the capability stays out of the renderer, the row stays open, the
reversal is put to the operator as a dated question with its specification
already written, and the caller-side contract becomes something a caller can
actually check.** Five reasons, and the third is the one that settles it.

1. **No caller is blocked, and that is checkable rather than asserted.** Each of
   the row's three Blocks entries has an answer already in this repository, named
   below in § *Is any existing caller actually blocked*. **A capability built for
   a caller that does not exist is a rasteriser with no acceptance test that can
   fail**, and `developer.md` § *Phase 1* asks for the acceptance test first.
2. **The row's own severity says what kind of task this is.** `L10` sits in the
   **"Low / Medium"** bucket and not in the "High" one, on a scale the same
   document applies. **What `#4` needed from `L10` was icons, and icons are
   answered offline** — `TASK_UI_PRIM_39.md` § *Why the icons are rasterised
   images and not vector draws* says it in one line: *"'No SVG' is the gap this
   task fills from outside the library, and the only place it can be filled"* —
   and `TASK_UI_PRIM_44.md` § *The decision: (A), a tintable texture path*
   decided `Icon` is a texture. **So the High-severity consequence of `L10`
   shipped as an asset pipeline, not as renderer geometry.**
3. **The specific question is not this file's to decide.** The operator declined
   ear clipping **on a named shape**, in a dated decision recorded in three
   places, and the later general instruction does not name it. `.ai/agents/developer.md`
   § *Failure modes* lists **"Re-opening the decision"** — *"You were handed a
   verdict. Second-guessing it is the evaluator's job, and doing it silently
   undoes the evaluation"* — and `.ai/agents/reviewer.md` § *Phase 0* refuses to
   *"re-litigate a confirmed verdict."* **A task file is not the agent that
   reopens it either; the honest instrument is a dated question with the
   evidence attached**, and `.ai/workflows/task-sequence.md` § Gates makes the
   skipped decision explicit rather than silent. **What this file must not do is
   write the reversal into a requirements list and let the operator's commit
   ratify it after the code exists.**
4. **What *is* unambiguously broken is checkable, and fixing it is free of every
   dated decision.** The crate's contract is *"a caller that cannot promise
   convexity must decompose the shape into convex pieces itself"* — and **a
   caller cannot currently find out whether it can promise it.** The rule exists
   twice, in two `#[cfg(test)]` modules, unreachable from production code, and a
   third caller has to write it a third time or hope. **That is a real defect and
   the cheap fix is a `pub fn`**, with **two existing uses** already (so
   `developer.md` § *Phase 2*'s *"No abstraction before the second use"* is
   satisfied by the tree, not by this file's preference), **no change to any
   pixel**, and no file that four other in-flight tasks own.
5. **The alternative that is not chosen is still specified.** § *What (A) would
   cost, and what it would need* answers the four questions an operator needs in
   order to say yes — worst case and failure mode, self-intersection, winding and
   fill rule, the degenerate cases, and the cost on the target. **An escalation
   that arrives without a specification is a deferral dressed as a question.**

### Why not (A), ear clipping — declined here for the same reasons it was declined there

**It is not declined on merit, and this file does not argue that it does not
work.** It is declined because **nothing recorded so far needs it**, which is
`render.rs`'s own stated reason at the moment the row was written — *"neither a
gauge needle nor anything else recorded so far needs one"* — and because
reversing it is the operator's call. **What has changed since 2026-10-02 is
recorded rather than argued:** the cost of *not* having it has been **paid
twice in this crate**, in two different widget modules, as two different
bespoke decompositions — `chart.rs`'s per-segment quads and `gauge.rs`'s
per-segment band — **and paid a third time outside the library**, as task 39's
offline rasteriser. **That is a fact the operator did not have on 2026-10-02,
and it is the strongest argument available for the reversal.** It is an argument,
and this file puts it in the escalation rather than acting on it.

**Three concrete costs are named here rather than discovered at review:**

- **`render.rs` is the file four unstarted tasks own.** Tasks 34, 35, 36, 37 and
  40 are each specified to change it, and its `command_quads` `Polygon` arm is
  the expansion **every shipped `Polygon` in the crate goes through** — the
  gauge's 33 band quads per arc, the needle, the chart's line and its fill. **A
  defect there is a wrong picture on five of six gallery pages**, and the
  sequence's own gate is *"A change that alters what is on screen is not verified
  until it has been seen."*
- **It adds a failure mode the status quo does not have.** A caller who today
  gets a wrong picture gets a *visible* wrong picture. A caller of an ear
  clipper who hands it degenerate input gets **`Vec::new()` — a hole in the
  chrome**, which `render.rs`'s `DrawCommand::Shadow` arm explicitly considers
  the better of two evils (*"a missing shadow rather than a wrong picture"*) and
  which is not obviously the better evil for an icon.
- **It is per-frame CPU work.** `command_quads` runs inside the batching path
  every frame (`batch_vertices` → `command_quads`), so an ear clipper is
  O(n²) work repeated 60 times a second per concave polygon on screen.

### Why not (C), a stencil pass — declined, and it is the most expensive of the four

- **The context requests no stencil buffer at all.** `Context::new` in
  `ui/src/ui_core/src/render/context.rs` sets six attributes and a stencil size is
  not among them; `set_stencil_size` appears nowhere in `ui/src/`. **(C) is
  therefore not a renderer change, it is a context change** — a new attachment on
  the default framebuffer, a per-frame `glClear` of the stencil bits, and a
  clear value that has to be correct for every existing pass.
- **It does not compose with the batcher.** `BatchKey` is
  `{ texture, blend_mode, shader }`, and a stencil fill needs *two* passes over
  the same geometry — front faces incrementing, back faces decrementing, then a
  third draw testing `!= 0`. **Each concave polygon would need its own draw
  sequence out of a batcher whose whole design is that a batch is one
  `use_program` and one vertex upload**, which is why `chart.rs`'s and `gauge.rs`'s
  decompositions produce *batches* rather than special draws.
- **On the target it is the option most likely to be refused or to cost most.**
  The default framebuffer is **4× multisampled** — an operator decision recorded
  in `IMPLEMENTATION_STATE.md` § *Task 21 — what it decided, and what it found*
  item 1 — and a stencil attachment on a multisampled default framebuffer is
  bandwidth on an aarch64 target. **And `L1`'s own row** already records that
  *"`ShadowTarget` is `GL_R8`, one channel of coverage"* and that capturing a
  scene needs `GL_RGBA8` — **so the crate has one FBO, it is single-channel, it
  is owned by shadows, and no stencil buffer exists anywhere in it.**

### Why not (B) alone — the row cannot be closed by argument, and saying so is the point

**(B) is the option this task is closest to, and it is not taken alone because it
fails this repository's own test for a closed gap.** The test is `DEMO_APPLICATION.md`
§ *Corrections to the second gap table*, which exists because a row *"was found
**false when written**"* twice — and `TASK_UI_PRIM_40.md`
§ *What this task does to gap `L4`, and the honest limit* puts the general rule
in one sentence: *"a gap closed on paper and open in the library."*

**After this task the sentence *"`Polygon` is convex-only"* is still true.** The
renderer still fans, still cannot fill a concave polygon, still produces
overlapping and inverted triangles for one. **What changes is that a caller can
now find out whether it is about to be wrong** — and that is an improvement to a
contract, not the removal of a gap.

**So the row is amended, not closed, and requirement 9 says so in the row's own
cell.** The instrument is the one `TASK_UI_PRIM_52.md` requirement 10 used on
row `L3`, which is the same situation exactly: one clause closed, the other not,
the row **"not deleted and not marked closed"**, its Blocks column unchanged.
**This file declines the `L6a`/`L6b` split instrument for `L10`, and says why:**
splitting would renumber the second gap table, and every row in it is cited by
slot from `L1` to `L11` — including by **`TASK_UI_PRIM_44.md` § *Out of Scope***,
which names `L10` as *"a separate task"* (this one) and **does not amend it**. A
row that three other task files cite by number is the wrong place to invent a new
slot.

### What the mesh path does and does not do to this row — **and the claim to strike**

`TASK_UI_PRIM_37.md`, `TASK_UI_PRIM_38.md` and `TASK_UI_PRIM_39.md` do change the
picture, and **the way they change it is the opposite of the way it is usually
argued.**

- **What is true.** A model is **already triangulated in its file**. Task 38's
  format carries indices, and task 39 recorded `triangles: 2032`,
  `indices: 6096`, and **`degenerate_triangles: 6`** for `sedan.glb` — so a mesh
  needs no polygon triangulation at draw time, ever, and **`DrawCommand::Mesh`
  bypasses `command_quads` entirely** as task 37 specifies it. That is a real
  reason not to build ear clipping for the 3D case.
- **What is false, and it is the argument usually made.** **A mesh is not one of
  this row's three Blocks entries.** Not *"any concave silhouette"* (the icons),
  not *"the proximity ramp"*, not *"instrument arcs"*. **All three are 2-D UI, and
  none of them is a model.** So *"the mesh path makes the renderer-side solution
  unnecessary"* is **false for `L10`**, and **no requirement, acceptance criterion
  or doc comment in this task may repeat it.**
- **And the mesh path is not even in the tree.** `render/mesh.rs`,
  `render/matrix.rs` and `render/meshio.rs` do not exist and
  `DrawCommand::Mesh` is not a variant. **So the mesh path's effect on this row is
  a specification, not an artefact** — the same *false when written* shape
  `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice, and
  the reason this task cites the specification by file and section rather than by
  source. **A reviewer checking this section against `ui/src` will find nothing,
  and that is correct.**
- **What it does mean for urgency: `L10` is not raised, and it is not lowered
  either.** Its severity stays **Medium**, and `DEMO_APPLICATION.md`
  § *Task structure* keeps it in the **"Low / Medium"** bucket — which requirement
  9 forbids editing. **The mesh sequence removes the strongest hypothetical need
  for ear clipping (a 3-D model) and none of the three recorded ones.**

### Is any existing caller actually blocked? — **No, and here is the accounting, entry by entry**

**This is the section the operator's decision turns on, so each answer carries the
source that answers it rather than a judgement.**

| Blocks entry | Answered by | Evidence |
|---|---|---|
| **"instrument arcs"** | **Shipped, in 2026-10-01, as a caller-side decomposition.** The gauge's arc is a concave band — an annulus sector — and `gauge.rs` draws it as **one convex four-point `DrawCommand::Polygon` per segment**, corners on the inner and outer radii. | `ui/src/ui_core/src/widgets/gauge.rs`, the module section *The arc is a band of quads, and both rejected alternatives are measured* — which also records why a chain of circles and a `Path` were both worse, **with measurements** |
| **"any concave silhouette"** (i.e. the icons, which is why `#4` was raised to **High** on 2026-10-01) | **Outside the library, offline, by design.** Lucide's SVGs are rasterised by `resvg` in task 39's asset pipeline, hash-verified and committed as bytes; task 44's `Icon` is a `TextureHandle` plus a `tint` and a fixed size. **A concave icon silhouette is a PNG at `ICON_SIZE`.** | `TASK_UI_PRIM_39.md` § *Why the icons are rasterised images and not vector draws*; `TASK_UI_PRIM_44.md` § *Why not (B), a procedural glyph path* and § *Why not (C)* |
| **"the proximity ramp"** | **It is not a fill, in the source it comes from.** `DEMO_APPLICATION.md` § *Composite widgets*, row 7, quotes the manual: *"**Colored lines** radiate from the image of your Model 3 as objects are detected … The color of the lines (white, yellow, orange, or red) represents the object's proximity."* **A radiating line is `DrawCommand::Line` or a `Path`, both of which exist** — and what that row demands is *"Position and colour carrying two independent variables, emitted radially"*, which is a state problem and not a fill problem. | `DEMO_APPLICATION.md` § *Composite widgets*, row **7**, tagged `[A]` |

**So: the crate has two shipped decompositions of concave shapes
(`chart.rs`, `gauge.rs`), one shape the design wanted that was answered offline
(task 39 + task 44), and one named Block that turns out not to want a fill at
all. Zero callers are blocked.** **This file states that as a fact about the tree
and stops there** — it is not an argument that concave fills are unneeded in
principle, because § *What (A) would cost* is the argument for needing one.

### What evidence would settle the reversal, in the operator's hands

**Four observations, each of which would change this file's recommendation, and
each of which has a mechanism already written down in this repository:**

1. **A caller that cannot decompose cheaply.** The decomposition is affordable
   where the shape is a *chain* (a chart fill, a band) and expensive where it is
   *many-holed with a smooth boundary*. **The design has no such shape today** —
   the ~20 indicator lights and the ~30 icon glyphs are images after task 39. **A
   caller that reaches one is the trigger**, and the shape of that call is what
   would decide (A) over (C): many holes means the stencil's nonzero rule and a
   fill rule, which is **clause 3** and a separate row.
2. **A measurement that the decomposition seams.** `chart.rs`'s module section
   *A translucent fill does not seam* is the measurement: a five-sample area chart
   at `Palette::fill` alpha 140, through the real renderer with 4× MSAA, reads
   **one distinct colour in 27 900 pixels**. `gauge.rs`'s is the chord: **0.255 px**
   on the outer edge at the defaults. **If a future caller finds a fill where
   those two do not hold, the contract is refuted by a number** — which is the
   only kind of refutation this repository accepts
   (`.ai/agents/developer.md` § *Phase 3*, *"Measured, not asserted"*, is the
   register; `TASK_UI_PRIM_21`'s measured seams are the precedent).
3. **A frame-cost measurement.** Ear clipping's cost is per-frame CPU in the
   batching path. `.ai/tools/fps-check.sh` before and after, on the page that
   carries the concave polygon, would settle affordability in one run each.
4. **Nothing in this repository will ever settle it,** because
   `ui_core`'s `Cargo.toml` carries `publish = false` — **there is no downstream
   user whose need is a fact the operator holds and this repository does not.**
   That is an argument for deciding it on the evidence above, not for deferring.

### What (A) would cost, and what it would need — the escalation, pre-specified

**Written so that "yes" is an instruction rather than another question. It is a
specification for a task that does not exist unless the operator asks for one,
and none of it is built by this task.**

- **Where the code goes.** One arm of `command_quads`'s `DrawCommand::Polygon` in
  `ui/src/ui_core/src/render.rs`, plus a `pub(crate)`-or-private triangulator
  beside `polygon_quad` and `line_quad`. **No new vertex type, no new shader, no
  index-buffer change** — a triangle is already a quad with a duplicated corner,
  which is `render.rs`'s recorded reason and the reason the pipeline is quad-only.
- **A convexity pre-test, and it is what makes the change pixel-neutral.** An
  ear clipper applied to a convex polygon produces a *different triangulation of
  the same region*. **A same-colour retiling does not seam** — `chart.rs`
  measured one colour in 27 900 pixels across three shared edges at alpha 140,
  and that measurement is what licenses it. **So the O(n) convexity test runs
  first, a convex polygon keeps today's fan, and every existing capture in this
  repository's history stays valid by construction.** Without that pre-test the
  change moves pixels on the gauge's 33 band quads and the chart's whole fill.
- **Worst case and failure mode.** Ear clipping is **O(n²)**: `n − 2` ears, each
  tested for containment against the `n − 3` remaining vertices. **The
  two-ears theorem says an ear exists for every simple polygon of at least four
  vertices, but it assumes no degenerate vertex** — no repeated point, no
  collinear triple — **and this crate cannot assume either.** So: a vertex whose
  cross product is exactly zero is **skipped as a candidate ear** (and a collinear
  vertex is *removed* rather than clipped, which is the standard repair); a
  candidate ear whose triangle has **zero area is rejected**; and **if no ear is
  found, the whole polygon expands to `Vec::new()`** — nothing drawn. **That is
  the answer `command_quads` already gives for fewer than three points, and it is
  the right one for the same reason: a wrong picture is what this row calls a
  wrong picture, and a hole is visible.** A `panic!`, an `unwrap` or a
  `loop`-forever is excluded by `developer.md` § *Panics and unwrap* and
  § *Stop conditions*.
- **Degenerate triangles, and why the evidence is real rather than hypothetical.**
  **Zero-area triangles are emitted or skipped, and it must be one of those, not
  "whatever falls out".** `TASK_UI_PRIM_39.md` § *The pipeline* records
  **`body` contains 6 zero-area triangles out of 704** — indices 296, 308, 593,
  594, 654, 655, three collinear triples — and *also* records why they are
  **kept** rather than removed: removing them opens the shell (the welded
  edge-valence histogram goes from `{2: 1056}` to `{1: 14, 2: 1040}`, i.e. **14
  holes**), because task 37 enables `GL_CULL_FACE` with `cull_face(GL_BACK)`.
  **So the repository's measured position on a degenerate triangle is "it
  rasterises nothing, and removing it can cost more than it saves"** — which is
  the same answer here, and it is an answer rather than an accident.
- **Self-intersection.** Ear clipping assumes a **simple** polygon. A
  self-intersecting one has no consistent winding, so the containment test is
  meaningless. **Detection is required and its cost must be stated**: an O(n²)
  segment-pair test is enough for the point counts a 2-D UI shape has (an icon
  outline is tens of points, not thousands) and a rejected polygon expands to
  `Vec::new()` on the same grounds as a failed ear search. **The fast
  Bentley–Ottmann O(n log n) is not worth it here and should be named as declined
  rather than left unmentioned.**
- **Winding and `fill_rule`.** **Neither a `fill_rule` field nor a `nonzero`
  argument is needed, and adding either is out of scope.** For a **simple**
  polygon, an ear clipping that respects the input's own winding produces a
  triangulation of the interior, which *is* the nonzero rule. **Even-odd needs
  explicit hole handling** — a second, different feature, and one this task's
  clause 3 is about. **Both windings must be accepted**: window space is **y-down**
  (`Rect::y` is an offset from the top edge), so "counter-clockwise" is a footgun,
  and `gauge.rs` already carries a `winding` helper because *"the winding is a
  property of the whole band"*. **A rule that only accepts one winding would be a
  defect the first y-down caller found.**
- **Cost on the target.** `command_quads` is on the per-frame batching path, so
  this is 60 repeats a second. **A 200-point concave polygon is ~40 000
  containment tests, tens of microseconds** — affordable on aarch64 and
  x86_64 alike. **The cost that matters is not the arithmetic, it is the risk**:
  a bug in this arm is a wrong picture on five of six gallery pages, and the
  sequence's gate is that such a change is *"not verified until it has been
  seen."*
- **The test that would carry it**, in `chart.rs`'s shape rather than a raster:
  **a property test over many random concave polygons asserting that the union
  of the emitted triangles covers the polygon and stays inside it** — checked
  **without a display**, by asking the same question twice in two ways: *is a
  point inside the polygon* (a crossing-count ray cast) against *is it inside
  some emitted triangle* (a barycentric test), over a lattice of sample points
  per polygon. **That is geometry, not pixels, and it is the precedent
  `every_segment_quad_is_convex_over_two_hundred_thousand_geometries` set** —
  whose own finding, recorded in the same module doc, is that **a proof can be
  wrong and a search finds it** (*"a search over two hundred thousand geometries
  found the counterexample on its thirty-eighth"*).

### Scope, measured against `developer.md` § *Scope check*

**Five files, three components, at both thresholds and neither over.** Files:
`ui/src/ui_core/src/paint.rs` (one new `pub fn`, one new constant, two doc
paragraphs, one test module addition); `ui/src/ui_core/src/widgets/chart.rs`
(its test helper becomes a wrapper over the new `pub fn`, one new named test,
one module-doc sentence); `ui/src/ui_core/src/widgets/gauge.rs` (the same wrapper,
one new named test, one module-doc sentence); `doc/ui/DEMO_APPLICATION.md` (row
`L10`, § *Corrections to the second gap table*); `doc/ui/IMPLEMENTATION_STATE.md`
(the record and the task-table row).

Components: **the predicate and its own tests**, in `paint.rs`; **the two widget
test migrations and the gauge's new test**; **the row amendment and the record.**
**No fan-out is needed** — `.ai/protocols/subagents.md` § *Implementation
fan-out* does not apply.

**And the deliberate consequence, which is the reason the diff is this small:
`ui/src/ui_core/src/render.rs` is not in the list and must not become the fifth.**
It is the file four unstarted tasks specify changes to, and **`git diff --stat`
for this task's commit must show it absent.** **If the implementer finds
themselves editing it, that is a stop condition rather than an expansion**
(`developer.md` § *Stop conditions*).

## Requirements

1. **The decision, stated where the operator reads it, and dated.**
    `doc/ui/IMPLEMENTATION_STATE.md` gains a section **§ *Task 51 — what it
    decided, and what it found*** whose first paragraph is the decision and its
    date-interaction in this form: **ear clipping and a stencil pass remain
    declined; the operator's decision of 2026-10-02 stands; the operator's
    instruction of 2026-10-05 that every library gap must be closed does **not**
    reverse it, because that instruction names gaps `#3` and `#7` and does not
    mention any technical declination; and the reversal question is now on the
    table in writing with § *What (A) would cost, and what it would need* beside
    it.** The section is **dated and attributed to `TASK_UI_PRIM_51`**, and it
    carries **the three dates distinguished** — 2026-10-01 (the primitive added),
    2026-10-02 (both alternatives declined), 2026-10-05 (every gap must be
    closed) — **because conflating the first with the second is the mistake this
    task exists to prevent.** It carries **no claim that the capability changed**,
    and it carries **no statement that the operator has answered the reversal
    question.**

2. **A new public predicate in `ui/src/ui_core/src/paint.rs`, beside the doc that
    states the obligation.**

    ```rust
    /// Whether `points`, in the order given, turns the same way at every corner.
    ///
    /// The precondition [`DrawCommand::Polygon`] states and the renderer's fan
    /// needs, made askable. ...
    #[must_use]
    pub fn polygon_is_convex(points: &[(f32, f32)]) -> bool
    ```

    - **`pub` and module-level**, so the call site reads
      `paint::polygon_is_convex(&points)`; **not** a method on `Painter`, which
      holds state this does not read and which a caller checking a decomposition
      does not have.
    - **`#[must_use]`**, because a caller that calls it and ignores the answer
      has written a bug — `developer.md` § *API design*.
    - **The algorithm, exactly, and it is the algorithm already in the tree twice:
      for each consecutive triple `(a, b, c)`, the last wrapping to the first,
      form `(b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)`; the polygon is
      convex when the positive crosses and the negative crosses are not both
      present.** This is `chart.rs`'s and `gauge.rs`'s `assert_convex`, moved —
      **so replacing them is behaviour-preserving and their 200 000-geometry
      search is the evidence that it is.**
    - **A cross product of exactly zero is neither**, so a collinear triple and a
      repeated point are **not** concavity. **There is deliberately no
      tolerance**, and the doc says why in its own paragraph: the crate's one
      tolerance is `render/blur.rs`'s `EPSILON: f32 = 1e-5`, a kernel weight
      tolerance in a module with one natural scale, and **a point list has none** —
      a `Rect` is pixels, a gauge is pixels, and a model in metres is not pixels.
      **A tolerance here would be a silent unit assumption.** The doc names the
      consequence: **at large coordinates a nearly-collinear triple's cross
      product is rounding noise**, so a caller working in metres must scale, and
      that is the caller's decision to make.
    - **`points.len() < 3` answers `true`**, and the doc says why: nothing turns,
      and `command_quads` draws nothing for such a list, so `true` is not a
      promise about a shape that will be drawn. **`Painter::polygon`'s doc already
      says the two facts are the same fact.**
    - **A non-finite coordinate answers `false`.** A `NaN` cross product is
      neither `> 0.0` nor `< 0.0`, so without an explicit finiteness check a
      `NaN` would read as "all turns zero" and answer `true` — **a false
      "convex" on a shape the fan will draw wrongly.** `"I cannot promise
      convexity"` is the honest answer, and it is the answer `clamp_pitch`
      returning `0.0` for a non-finite pitch gives in `TASK_UI_PRIM_40.md`
      requirement 5.
    - **Both windings answer `true`.** Window space is y-down, so the sign of a
      turn is not a thing a caller should have to think about; the rule is
      sign-agnostic by construction.
    - **The doc states the limit, in its own paragraph, because it is a real one:
      this is necessary and not sufficient.** For a **simple** polygon, every
      turn the same way **is** convexity. For a **self-intersecting** one it is
      not: a five-pointed star drawn in star order turns the same way five times
      and self-intersects twice, and the fan still draws it wrongly. **The
      predicate answers the shape's *turns*, not its *intersections*, and the doc
      says so by name**, together with what a caller who needs sufficiency must
      pay for and where that is out of this crate's scope.
    - **No new type, no new enum, no `Result`, and no allocation.** The answer is
      one `bool` from one pass; a caller with a hundred points gets no heap
      traffic.

3. **`Painter::polygon`'s doc and the `DrawCommand::Polygon` variant's doc each
    gain one sentence naming `polygon_is_convex`, as an intra-doc link.** **No
    signature change and no behaviour change to either**: `polygon` still records
    what it was handed, because *"the recorder's contract is to store what it was
    handed"* and that sentence is right. **What changes is that the obligation
    has a name and a way to be discharged**, so the sentence *"a caller that
    cannot promise convexity must decompose the shape into convex pieces itself
    before recording it"* is followed by **"and
    [`polygon_is_convex`] is how a caller finds out whether it can."**
    **This is the discoverability half of the deliverable and it is the half that
    makes the predicate a contract rather than a utility.**

4. **`chart.rs`'s and `gauge.rs`'s private `assert_convex` helpers become
    two-line wrappers over `paint::polygon_is_convex`, and each keeps its failure
    message.** The message is not decoration: `chart.rs`'s names the geometry
    (*"random geometry {attempts} segment {index}"*) and `gauge.rs`'s says what a
    failure would mean (*"so the fan overlaps itself"*), and **a search over
    200 000 geometries that reports *which* one is worth having**. **So the
    wrapper stays and the algorithm moves** — the rule has one home, the labels
    stay where they are read, and **the crate has one convexity rule instead of
    two.** Neither helper is deleted and neither test is renamed.

5. **One new named test in `gauge.rs`, because a shipped decomposition that only
    proves itself with its own copy of the rule is the defect this task is
    fixing.** **`every_polygon_the_gauge_records_is_convex_by_the_library_rule`**
    sweeps all three `GaugeType` values and a swept value space over both a wide
    and a square rect, collects every `DrawCommand::Polygon` the gauge records,
    and asserts `paint::polygon_is_convex` on each. **The name states the library
    rule on purpose**: a test named only `..._is_convex` would pass unchanged if
    someone reintroduced a private copy of the check.

6. **One new named test in `chart.rs`, and it is the mirror of the mutation
    evidence.** **`the_predicate_rejects_the_counterexample_this_module_recorded`**
    builds the non-convex segment quad out of the arithmetic in that module's own
    doc — the case where the two offsets' displacement along the segment differs
    by the segment's own length, which the doc says *"a search over two hundred
    thousand geometries found … on its thirty-eighth"* — **and asserts the
    shared predicate rejects it.** A library-wide rule that no test ever catches
    rejecting anything is a rule nothing has checked; this is the cheapest
    possible check that the move from a private helper to a public one preserved
    the *sensitivity* as well as the *soundness*.

7. **The tests in `paint.rs`, named, with no display, no network, no filesystem
    and no wall clock** — the only kind `AGENTS.md` permits:

    - `polygon_is_convex_accepts_a_triangle_and_a_convex_quadrilateral` — the
      positive control, off the origin so an origin fixture cannot see a
      coordinate read as an extent.
    - `polygon_is_convex_rejects_an_l_a_chevron_and_a_star_shaped_list` — the
      negative control on three concave shapes whose names say what they are.
    - **`polygon_is_convex_calls_a_collinear_point_list_convex_because_it_encloses_no_area`** —
      **the degenerate case, and its name states the reason rather than the
      verdict.** Three collinear points, five collinear points, a rectangle with
      one edge split by a point on it, and a list with a point repeated
      (`a, b, b, c`). **All four answer `true`, and the test's comment says why
      that is not a false promise: a collinear point list encloses no area, and
      `command_quads` draws nothing for it.**
    - `polygon_is_convex_reads_both_windings_as_convex` — the same quadrilateral
      given both ways round, because window space is y-down.
    - `polygon_is_convex_refuses_a_non_finite_point` — `f32::NAN` and
      `f32::INFINITY` in each of x and y, each answering `false`, **with the
      comment naming the trap it closes**: without the finiteness check a `NaN`
      cross product answers `true`.
    - `polygon_is_convex_accepts_fewer_than_three_points` — `[]`, one point, two
      points.
    - `polygon_is_convex_reports_by_its_turns_and_does_not_test_for_intersections`
      — a five-pointed star **in star order** answers `true` while
      self-intersecting, **asserted on purpose**, because the limitation is in
      the predicate's doc and a limitation nothing asserts is a limitation the
      next edit will forget.
    - **`polygon_is_convex_agrees_with_a_half_plane_test_over_two_hundred_thousand_simple_polygons`** —
      **the property test, and it is `chart.rs`'s search pointed at a stronger
      question.** Two *independent formulations* of the same fact must agree:
      the shipped O(n) turn rule, and an O(n²) oracle that asks, for each edge,
      whether every other vertex lies on one side of the line through it — which
      is convexity for a simple polygon and is not the same computation. **The
      generator is the one thing that has to be right: random points **sorted by
      angle about their own centroid**, which yields a polygon star-shaped about
      that centroid and therefore **simple** — and without that the oracle is not
      valid and the test would be measuring two different questions.** Radius is
      varied so the lists run from convex to deeply concave. As in the chart's
      search, **it asserts the search actually ran (`checked >= 200_000`) and that
      it saw both verdicts**, so it cannot pass by everything being convex.
      **The LCG seed is a named constant in the test**, `chart.rs`'s shape, and
      **the count is in the test's name** so a reader knows what was searched.

8. **The degenerate-input answer is recorded in the source, not only in the
    tests.** `paint.rs`'s module-level docs for `polygon_is_convex` and
    `render.rs`'s `command_quads` `Polygon` arm are left **byte-identical** by
    this task — **the renderer's arm is not edited at all** — and the *new* prose
    about degeneracy lives only in `paint.rs`'s doc and in
    `IMPLEMENTATION_STATE.md`. **What that means, stated so a reviewer can check
    it: `render.rs` gains nothing from this task, including a comment, and the
    six gallery pages are unaffected by construction rather than by luck.**

9. **Row `L10` is amended, dated, attributed, and is not closed.**
    `doc/ui/DEMO_APPLICATION.md` carries **four** edits and the first three are
    the same fact in the three places it is recorded:

    - **Row `L10`** in § *Gaps this layout exposes in `ui_core`* gains a dated
      note naming `TASK_UI_PRIM_51` and stating, **per clause**:
      - **clause 1 (`Polygon` is convex-only) — still true, still open.** The
        fan is unchanged, the renderer is unchanged, and **the sentence at the
        head of the row is kept verbatim**, because it is true after this task.
      - **clause 1's contract — closed.** `paint::polygon_is_convex` now exists,
        is public and `#[must_use]`, and **the two decompositions this crate
        actually ships assert through it**, so *"every polygon this crate records
        is convex"* is a claim with a test behind it where before it had a
        comment behind it.
      - **clause 2 (no stencil pass) — still true**, declined 2026-10-02, and
        **now with the reason strengthened**: `Context::new` requests no stencil
        buffer at all.
      - **clause 3 (no bezier, no fill rules) — untouched, and named as not this
        row's question.** **No curve primitive is added and no `fill_rule` field
        is added**, and the note says so, because a reader who bundled "no
        bezier" into "convex-only" is reading one row as two gaps.
      - **clause 4 (`Path` is stroked) — still true**, and `#4` answers it
        offline.
      - **Each of the three Blocks entries gains the source that answers it**, by
        row and section — **not deleted**: *"instrument arcs"* → `gauge.rs`'s
        per-segment band; *"the proximity ramp"* → `DEMO_APPLICATION.md`
        § *Composite widgets* row **7**'s own words, *lines*; *"any concave
        silhouette"* → task 39's offline rasterisation plus task 44's `Icon`.
      - **The row is not deleted, not marked closed, its severity stays Medium,
        and its Blocks column keeps all three entries.** § *Task structure*'s
        **"Low / Medium"** line is **not edited**, because an open row in the
        Medium bucket must still say Medium.
    - **Its evidence column is rewritten from line numbers to symbols** — the
      `command_quads` `DrawCommand::Polygon` arm in
      `ui/src/ui_core/src/render.rs`, the `DrawCommand::Polygon` variant,
      `Painter::polygon`, `DrawCommand::Path` and `polygon_is_convex` in
      `ui/src/ui_core/src/paint.rs`, the two `assert_convex` wrappers, and the
      test names — **because a line number into `ui/src` is stale the moment the
      tree moves**, which is the reason `TASK_UI_PRIM_52.md` requirement 10 did
      the same for row `L3`.
    - **§ *Corrections to the second gap table* gains one dated paragraph**: the
      interaction of the three dates; **what the 2026-10-05 instruction does and
      does not reach**; **that this task did not reverse the 2026-10-02 decision
      and did not close the row**; and **that the escalation is written down
      rather than left in a session**. **That section is where this belongs**, and
      § *Gaps this layout exposes in `ui_core`*'s own preamble is amended only if
      its sentence about rows `L1` and `L6` being false-when-written needs it —
      **and it does not**, because this task's row was checked against the source
      and holds.
    - **No other row changes.** **Not `L1`**, though the stencil half of clause 2
      touches the same neighbourhood — `L1` is Critical, unstarted, and owned by
      `TASK_UI_PRIM_41.md`.

10. **`doc/ui/IMPLEMENTATION_STATE.md` gains the record**, in the shape
    requirement 1 sets out and with the rest of it:

    - **A task-table row for 51** naming the file, its review count and its
      waivers-or-none, and its frame rate.
    - **§ *Task 51 — what it decided, and what it found*** carrying: the decision
      and the three dates; **`polygon_is_convex`'s contract and its two stated
      limits** (no tolerance and why; turns and not intersections); **the
      per-clause status of `L10`**; **the three Blocks accounted for**; **that
      the crate had two private copies of the rule and now has one public one**;
      **the test count before and after**; **the frame rate for all six pages**;
      and **the honest limit in that file's own register** — *the rule is proved
      over two hundred thousand generated simple polygons against an independent
      formulation, **the renderer still cannot fill a concave polygon**, and
      **nothing in this repository is blocked by that**.*
    - **§ *Current position* updated** to name task 51 and what state `L10` is in.
    - **§ *Task table*'s existing rows are not edited**, and in particular the
      rows for tasks 34 to 40 keep saying *"specified 2026-10-05, not started"* —
      **this task does not start any of them**, and `IMPLEMENTATION_STATE.md` is
      not a source of evidence (`.ai/workflows/task-sequence.md` § *State*); it
      points at the code.

11. **The suite, the six pages and the frame rate are all produced.** From `ui/`:
    `cargo fmt --check`; `cargo build --all-targets --all-features`;
    `cargo clippy --all-targets --all-features -- -D warnings`;
    `cargo test --all-features` with **the three per-binary counts pasted and
    every test in requirements 6 and 7's list present by name**, against a
    baseline of **1894** (1450 `ui_core` + 224 `ui_demo` + 220 doctests) — **and
    no test deleted, renamed away or weakened**; `cargo doc --no-deps` clean; and
    `cargo audit` **recorded as not installed on this host, not passed**. Then
    the six-page before/after capture of the criterion below, and then **the frame
    rate on all six pages**.

## Acceptance Criteria

- [ ] **The decision and its date-interaction are stated in the record, in those
      words, and nowhere is a reversal claimed.** `IMPLEMENTATION_STATE.md`
      § *Task 51 — what it decided, and what it found* states that **ear clipping
      and a stencil pass remain declined and the operator's 2026-10-02 decision
      stands**; that **the 2026-10-05 instruction that every library gap must be
      closed does not reverse it, because it names gaps `#3` and `#7` and does not
      mention any technical declination**; that **2026-10-01 is the date the
      primitive was added and must not be cited as a declination**; and that
      **the operator has not been asked a question this task answers for them**.
      **And the mechanism is grep-able, because the failure this task exists to
      prevent is a quiet reversal:** `grep -rn '2026-10-02' ui/src/` returns the
      **three pre-existing hits and no new one**, so the crate's own record of the
      declination is unchanged and only `paint.rs` gains prose that **links** it;
      and no file changed by this task contains the words *"reverses the
      operator's decision"*, *"supersedes the 2026-10-02 decision"* or any
      equivalent claim.

- [ ] **`polygon_is_convex` exists, is public, is `#[must_use]`, and is where the
      obligation is documented.** `grep -n 'pub fn polygon_is_convex' ui/src/ui_core/src/paint.rs`
      shows the function and its `#[must_use]`; `awk '/pub fn polygon_is_convex/,/^}/'`
      shows **one pass, no allocation and no `Result`**; and both
      `Painter::polygon`'s doc and the `DrawCommand::Polygon` variant's doc carry
      an intra-doc link to it, which `cargo doc --no-deps` resolving without a
      warning is what proves. **Its doc states the two limits by name** — *no
      tolerance, because a point list has no scale* and *turns and not
      intersections, so a self-intersecting star reads convex* — and
      `polygon_is_convex_reports_by_its_turns_and_does_not_test_for_intersections`
      asserts the second one.

- [ ] **The crate has one convexity rule, not two, and the two searches still
      pass at the same counts.** `grep -c 'b.0 - a.0) \* (c.1 - a.1) - (b.1 - a.1)' ui/src/ui_core/src/widgets/chart.rs`
      and the same over `gauge.rs` return **0 each**, because both helpers now
      call `paint::polygon_is_convex`; and
      `every_segment_quad_is_convex_over_two_hundred_thousand_geometries` and
      `every_polygon_a_chart_records_is_convex` **keep their names, their seed,
      their 200 000 count and their assertions**. **Mutation evidence in the
      handoff, both directions, because this is the criterion a reviewer should
      break first:** (a) make `polygon_is_convex` return `true` unconditionally
      and watch **both** searches and `the_predicate_rejects_the_counterexample_this
      _module_recorded` fail; (b) restore it, then delete the finiteness check and
      watch `polygon_is_convex_refuses_a_non_finite_point` fail. **A rule that has
      only ever agreed with itself has proved nothing**
      (`developer.md` § *Phase 3*, *"A test that has never failed is not a test."*).

- [ ] **The degenerate-input case has a named test and it names its reason.**
      `polygon_is_convex_calls_a_collinear_point_list_convex_because_it_encloses_no_area`
      exists, covers **four** degenerate lists — three collinear points, five
      collinear points, a rectangle with one edge split by a point on it, and
      `a, b, b, c` — and asserts `true` on each **with the reason in the test's
      comment and in its name**: *a collinear list encloses no area, and
      `command_quads` draws nothing for it.* **It is paired with the renderer's
      own degenerate answer, which is unchanged and still pinned:**
      `a_polygon_of_fewer_than_three_points_expands_to_no_quads` keeps its name
      and its assertions, and `git diff --stat` for this task's commit shows
      `ui/src/ui_core/src/render.rs` **absent**.

- [ ] **The property test ran, and the two formulations agreed on two hundred
      thousand of something.** `polygon_is_convex_agrees_with_a_half_plane_test_over_two_hundred_thousand_simple_polygons`
      passes, and its own body asserts **both** that `checked >= 200_000` **and**
      that it saw at least one concave list, **so it cannot pass by every
      generated polygon being convex.** The generator is **sorted by angle about
      the centroid**, and a reviewer should check that first: without it the
      polygons are not guaranteed simple and the half-plane oracle is not valid.
      **The mutation that must be shown:** make the predicate answer `true` for
      any list of five or more points and watch this test fail with the list index
      in the message — **that is the evidence it is searching rather than
      restating**.

- [ ] **A shipped decomposition proves itself against the library's rule, and the
      chart's counterexample is rejected by it.**
      `every_polygon_the_gauge_records_is_convex_by_the_library_rule` exists in
      `gauge.rs`, sweeps **all three `GaugeType` values** and both rect shapes, and
      asserts `paint::polygon_is_convex` on **every `DrawCommand::Polygon` the
      gauge records** — **and its name states "the library rule" on purpose**,
      because a test named only `..._is_convex` would pass unchanged against a
      reintroduced private copy of the check.
      `the_predicate_rejects_the_counterexample_this_module_recorded` exists in
      `chart.rs` and builds the non-convex quad out of that module's own recorded
      arithmetic. **Neither is a new private helper:** the handoff pastes
      `grep -c 'fn assert_convex' ` for both widget modules and the number is
      **1 each**, which is the wrapper, not a second algorithm.

- [ ] **`cargo test --all-features` is green against 1894, with every named test
      present.** The handoff **lists each by name**: in `paint.rs` —
      `polygon_is_convex_accepts_a_triangle_and_a_convex_quadrilateral`,
      `polygon_is_convex_rejects_an_l_a_chevron_and_a_star_shaped_list`,
      `polygon_is_convex_calls_a_collinear_point_list_convex_because_it_encloses_no_area`,
      `polygon_is_convex_reads_both_windings_as_convex`,
      `polygon_is_convex_refuses_a_non_finite_point`,
      `polygon_is_convex_accepts_fewer_than_three_points`,
      `polygon_is_convex_reports_by_its_turns_and_does_not_test_for_intersections`,
      `polygon_is_convex_agrees_with_a_half_plane_test_over_two_hundred_thousand_simple_polygons`;
      in `gauge.rs` —
      `every_polygon_the_gauge_records_is_convex_by_the_library_rule`; in
      `chart.rs` — `the_predicate_rejects_the_counterexample_this_module_recorded`,
      beside the two that keep their names. **Three per-binary counts pasted, and
      the total is 1894 plus the new tests with none removed.**
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is **recorded, not passed**.

- [ ] **The six gallery pages are pixel-identical outside the fps band — the
      full criterion, with no `CAR_RECT` restatement — and the mechanism is
      stated rather than hoped for.** `Page::ALL`'s six names, release build,
      captured **before and after** with the commands of `IMPLEMENTATION_STATE.md`
      § *Verifying a change that draws — the capture method* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture and `ffmpeg x11grab` both return black for a GL window),
      `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, then
      `magick compare -metric AE before.png after.png null:` per page.
      **The criterion is `AE = 0` outside `y ≥ 680` on all six pages, every
      differing pixel inside the fps readout's band** — which is exactly what
      tasks 34 to 39 inherited, and **task 40's `CAR_RECT` restatement does not
      apply to this task**, because this task moves no pixel anywhere.
      **The mechanism is three facts and not one comfortable one:**
      1. **The only production item this task adds is one `pub fn` in `paint.rs`,
         and no frame path calls it.** It is a query, not an expansion: nothing in
         `command_quads`, in `batch.rs`, in `batch_vertices` or in
         `draw_solid_batch` references it.
      2. **The two widget files change `#[cfg(test)]` code and doc comments
         only.** Their `paint` methods, their constants, their properties and
         their recorded commands are untouched — **so the gauge's 33 band quads
         per arc, the needle and the chart's line and fill are byte-identical
         inputs to a byte-identical expansion.**
      3. **`git diff --stat` for this task's commit shows
         `ui/src/ui_core/src/render.rs` absent**, and `batch.rs`, `font.rs`,
         `render/target.rs`, `render/context.rs`, `render/blur.rs` and
         `ui/src/ui_demo/src/main.rs` absent. **The commit's own diff is the only
         honest form of this check**, because `render.rs`, `font.rs`,
         `render/target.rs` and `ui/src/ui_demo/src/main.rs` are **already
         modified in the uncommitted diff at `75a896c`**, and a diff against
         `HEAD` would attribute another task's work to this one — which is the
         stale-evidence failure `.ai/workflows/task-sequence.md` § Gates names as
         *"No evidence by assertion."*
      **And the rect-level tests keep their names and every one of their
      assertions:** `every_page_places_every_rect_where_the_gallery_placed_it`,
      `no_two_placed_rects_overlap`, and `assert_placed_handles_is_complete` in
      the demo's suite.

- [ ] **The frame rate is measured on every page and reported, and no page is
      expected to move.** With the script's own line pasted rather than the number
      expected: `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of the
      six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `IMPLEMENTATION_STATE.md` § *Current position* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. Every page above the floor of **55**.
      **And the handoff states what was expected, so a good number is not a
      surprise and a bad one is a finding: every page is expected inside the
      recorded 61.1–63.9 band in `IMPLEMENTATION_STATE.md`
      § *The frame rate, measured*, because nothing on any frame path changed.**
      **A page outside that band is reported as a finding with its number pasted,
      not as noise** — because `task-sequence.md` § Gates records that a four-fps
      regression survived three reviews in this repository, and that gate exists
      precisely because no other check here can see one.

- [ ] **`L10` is amended, dated, attributed, and open — with the claim that was
      false when written caught if this task makes one.** Row `L10` in
      `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* carries a
      dated note naming `TASK_UI_PRIM_51`; **the sentence at the head of the row
      is kept verbatim, because it is still true**; each of the four clauses has a
      stated status; each of the three Blocks entries names the source that answers
      it; **the row is not deleted, not marked closed, its severity stays Medium
      and its Blocks column keeps all three entries**; § *Task structure*'s
      **"Low / Medium"** line is not edited; and its **evidence column cites
      symbols and test names, with zero `render.rs:`-style line numbers left in
      the row.** § *Corrections to the second gap table* gains the dated paragraph
      requirement 9 names, and **no other row changes.**
      **And the mechanism that keeps the row honest is stated: this task's claim
      about the row was checked against the source and holds, so nothing here is
      a stale citation being repeated** — which is the defect that section records
      twice, and the reason the evidence column had to be rewritten rather than
      carried across.

- [ ] **No doc comment in any changed file asserts the opposite of the code beside
      it**, which is the defect `DEMO_APPLICATION.md`
      § *Corrections to the first gap table* records twice. **Three claims are
      named in advance, because each is a sentence this task could have written and
      did not:** that the renderer fills concave polygons (**false** — requirement
      8 keeps `render.rs` untouched); that a caller's decomposition obligation is
      **enforced** (**false** — it is *askable*, and `Painter::polygon` still
      *"neither measures the points nor rejects them"*); and that the mesh path
      removed the need for this (**false** — § *What the mesh path does and does
      not do to this row*). **A reviewer should grep for all three.**

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      **Five files and no more**, per § *Scope, measured against `developer.md`
      § *Scope check*; `ui/Cargo.toml` and `ui/Cargo.lock` are **unchanged** — the
      approved direct dependencies remain `sdl3 0.20`, `glow 0.18` and
      `freetype-rs 0.38`, and **a polygon-triangulation crate is a licence
      decision against GPLv3 that nobody has asked for**; **`grep -c unsafe`
      over `paint.rs` is unchanged from before this task**, because the predicate
      is arithmetic on two `f32`s; `grep -rn 'unwrap()\|expect(\|panic!\|todo!\|unimplemented!' `
      over the diff is empty; and **no task from the 34–40 sequence is started**,
      which `IMPLEMENTATION_STATE.md` § *Tasks 34–40 — the mesh-rendering
      sequence* must still say *"Nothing in it is started"* after this task.

## Out of Scope

- **No ear clipping, no concave fill in the renderer, and no reversal of the
    operator's 2026-10-02 decision.** This is the whole of the task's decision,
    restated where it cannot be missed. **`command_quads`'s
    `DrawCommand::Polygon` arm is not edited — not even a comment** (requirement
    8), `DrawCommand` gains no variant, no triangulation function is written, and
    `git diff --stat` for this task's commit shows
    `ui/src/ui_core/src/render.rs` absent. **§ *What (A) would cost, and what it
    would need* is the specification for a task that does not exist unless the
    operator asks for one, and it is written so that "yes" is an instruction**
- **No stencil pass, and no stencil buffer.** Declined on 2026-10-02, and
    `Context::new`'s attribute list is **not** touched — `set_stencil_size`,
    `glClear(GL_STENCIL_BUFFER_BIT)` and every other stencil call remain absent
    from `ui/src/`, and requirement 9 records the strengthened reason rather than
    the code
- **No bezier, no curve, no `Path` tessellation, no stroke-to-fill conversion and
    no outline extraction.** These are **clause 3 and clause 4 of the row** —
    *a missing curve primitive and a stroked `Path`* — and **neither is a
    convexity question**. `grep -rn 'bezier\|cubic\|quadratic\|curve_to' ui/src/`
    stays empty. **Requirement 9 records clause 3 as open and untouched rather
    than folded into clause 1**, because one row reading as two gaps is how
    `L10` came to be rated Medium with a High-severity consequence attached
- **No fill rule, no `nonzero`, no `even_odd`, and no winding argument.** Zero
    hits for all three before this task and zero after. The predicate's
    *sign-agnostic* reading of winding is not a fill rule, and **even-odd would
    need explicit hole handling**, which is a separate feature
- **No multi-subpath, no holes, and no `DrawCommand` change of any kind.** The
    nine variants stay nine. `PaintState`, `Painter::extend`, `Painter::finish`
    and every other method keep their signatures
- **No mesh or GLB triangulation, and nothing from tasks 34–40.**
    `render/mesh.rs`, `render/matrix.rs` and `render/meshio.rs` **are not
    created**, `MeshVertex` and `DrawCommand::Mesh` are not added, and
    `DEPTH_BITS`, the depth policy, the culling policy, the mesh shaders, the
    loader and the byte format are untouched. **§ *What the mesh path does and
    does not do to this row* is analysis of three task files, and nothing in this
    task implements any of them**
- **No `Icon`, no icon asset, no SVG, and no edit to task 39 or task 44's files.**
    They are cited for what they decided. `TASK_UI_PRIM_44.md`
    § *Out of Scope* names `L10` as *"a separate task"* and says *"`L10` is not
    amended, not narrowed and not re-opened here"* — **and this task does not
    amend task 44's file either**, so the two records agree rather than one
    contradicting the other
- **No change to the six pages, no new page, and no `ui/src/ui_demo/src/main.rs`
    edit at all.** `Page::ALL` stays `[Page; 6]`, `Page::DEFAULT` stays `Pads`, the
    `--help` text is unchanged and no new `--tab=` name exists. **The capture
    criterion above is the full one precisely because this task moves nothing**
- **No new dependency, no `unsafe`, and no `as` cast.** Per `AGENTS.md` the
    approved direct dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs
    0.38`; the predicate is two subtractions, two multiplications and one
    subtraction on `f32`s, **so there is nothing here a dependency would shorten
    and nothing here that needs one**; and `developer.md`'s ban on `as` casts for
    numeric conversions is untouched because **the predicate converts nothing**
- **No change to any other gap row**, and none to
    `doc/ui/PRIMITIVES_ARCHITECTURE.md`. **`L1` in particular is untouched**, and
    `TASK_UI_PRIM_41.md` owns it
