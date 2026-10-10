# TASK_UI_PRIM_40: Drag to Rotate — a Two-Axis `Rotator` in `ui_core`, and the First Mesh on Screen

## Goal

Give `ui_core` a widget that turns a drag into **two angles** — a yaw about y and
a pitch about x, with the pitch clamped — and wire it to the car the demo already
loads, so that the rotation is exercised by the frame loop whether or not a pointer
event ever arrives.

## Context

This is the **seventh and last** of the seven tasks (34–40) that add real-time 3D
mesh rendering to `ui_core`, and the only one of them whose deliverable is an
**interaction** rather than a mechanism.

| # | Subject | Depends on 34 for | Depends on 35 for | Depends on 36 for | Depends on 37 for | Depends on 38 for | Depends on 39 for |
|---|---|---|---|---|---|---|---|
| 34 | the depth buffer, and the depth policy | — | — | — | — | — | — |
| 35 | `MeshVertex` and the mesh GPU buffers | a place to put the projected `z` | — | — | — | — | — |
| 36 | matrix maths and the transform-to-GPU path | the `z` this task's angles become | — | — | — | — |
| 37 | the mesh draw command, its shader and batching | the depth test | the buffers | `Mat4` | — | — | — |
| 38 | the model file format and the loader | — | `Mesh` | — | — | — | — |
| 39 | the asset pipeline | — | — | — | — | the bytes | — |
| 40 | **drag-to-rotate (this task)** | **a rotated mesh to be depth-tested** | **four transformable wheel ranges** | **`Mat4`, and the promise not to move `near`/`far`** | **`DrawCommand::Mesh` and `ShaderKind::Mesh`** | **the model and the colormap the demo already loads** | **the asset that makes a car exist** |

**Every dependency is load-bearing and none of them is a nicety.** A missing
`render/mesh.rs`, `render/matrix.rs`, `DrawCommand::Mesh` or `assets/sedan.roados`
when this task starts is a **dependency violation, not a variant of this task** —
and that is the *only* acceptable reading of the table, because § *The demo must
draw something, or the mesh pass is still untested* below explains why this task
cannot be written against a tree where the model does not load.

### What `Drag` is, and what no widget does with it in **two** axes today

- **`InputEventKind` has nine variants** in `input.rs`: `Tap`, `LongPress`,
  `Swipe { direction }`, `Pinch { scale }`, `Drag { delta }`, `KeyDown { key,
  keymod }`, `KeyUp { key, keymod }`, `Scroll { delta }` and `Text { text }`.
  `SwipeDirection` has **four** variants — `Left`, `Right`, `Up`, `Down`.
- **`Drag` is consumed today**, by `Scroll` (`gesture_delta`), by `TextInput`
  (selection extension) and by `Keyboard` (slide-out, and only after the pointer
  has already moved). **So gap `L4` is not about `Drag`, and this task does not
  close it** — see § *What this task does to gap `L4`* below, which is the
  careful half of this file.
- **Every one of those consumers is one-axis.** `Scroll`'s `scroll_offset` is a
  **scalar** `Property<f32>`, and left/right are explicitly refused: its `on_event`
  carries the comment *"A drag with no vertical component is not a vertical scroll,
  and leaving it unconsumed lets a horizontal scroller above take it"*, returns
  `false` unconsumed for it, and `max_scroll`, `visible_rect`, `gesture_delta` and
  `wheel_delta` are all scalar in y. **There is no two-axis
  drag-driven value anywhere in the crate.** That is the space this task fills,
  and it is a space rather than an extension: adding a second axis to `Scroll`
  would have meant giving a vertical scroller a horizontal one, which is gap `L5`
  (*"No horizontal scrolling"*) and a different decision with a different
  justification.
- **There is no momentum, no fling, no inertia and no snap anywhere in the crate.**
  No `inertia`, `momentum` or `flick` identifier exists. `Scroll::snap_to_state` is
  **palette** snapping — jumping instantly to a new colour set — not scroll
  snapping, and a reader who takes its name for a mechanic is misreading it.
- **The animation idiom is "aim once, tick per frame"**, and
  `Button::animate_to_state(motion)` with `Button::tick(delta) -> bool` is the
  named pattern. The demo's `sync_toggle_state` is the recorded reason a **per-frame
  aim** is wrong: a transition restarted every frame creeps toward its target and
  never arrives.
- **`LayoutMode::Grid` lays out nothing** — `LayoutMode::Grid { .. } => Vec::new()`
  in `layout.rs`'s arrange match, `columns` declared and never read — and
  `Flex.wrap` is accepted and discarded by the same `..`. **The rotator must
  therefore depend on neither**: it is placed absolutely like every other node in
  the demo, and it has one child-free box.

### The decision: a `ui_core` widget, and the demo owns the matrix

**Decision: `Rotator`, a new `ui_core` widget, and the demo builds the `Mat4`.**
Four reasons, and the third is the one that settles it.

1. **`Scroll` is the precedent for a drag-driven `Property`, and `Slider` is the
   closer one.** `Slider` is a scalar `Property<f32>` a drag writes, with a
   `dragging: Property<bool>` **written by the caller from a press** (because the
   gesture recogniser reports a tap on the *release*, so the widget has no
   "the finger went down on me"), an `on_event` whose `KeyDown` arm requires
   `focused`, and a private `adjustment(&key, orientation)` mapping arrows and a
   d-pad to a signed step. **`Rotator` is that shape with two properties and a
   clamp**, and reusing it means a reviewer has one shape to check rather than a
   new one.
2. **A rotator with no 3D in it is the reusable part, and it is most of the
   thing.** Its whole content is: read a delta, scale it, clamp, write. There is
   no projection, no matrix, no `MeshId`, no `GL_` call — which means **every line
   of it is unit-testable**, and unit-testability is the only verification this
   host can offer (see § *The central constraint*). A `ui_core` widget is also the
   only place a test of that arithmetic can live under `AGENTS.md`'s rule that tests
   go beside the code.
3. **A demo-local struct cannot close a row of a table about `ui_core`.** Row `L4`
   in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*
   is phrased about **widgets**: *"`LongPress` and `Swipe` are emitted and consumed
   by no widget"*, and its evidence is *"`widgets/mod.rs` lists fifteen `pub mod`
   entries"* and *"`InputEventKind::Swipe` appears in no `on_event` body at all"*.
   A struct in `ui_demo` is not a widget, `input::route` can never reach it, and
   **the row would still read true afterwards** — a gap closed on paper and open in
   the library, which is the exact shape `DEMO_APPLICATION.md`
   § *Corrections to the second gap table* exists to have been caught twice.
4. **`Mat4` is the other half and it is the demo's.** Task 36's own module docs
   record that the composition is `P · V · M`, that `Mat4` has **no `look_at`**, and
   that a 2D transform's rotation axis is documented nowhere. The orbit is a
   camera placement — a distance, a target and two angles — and **where the camera
   stands relative to the car is a property of the screen, not of the crate.** So
   `Rotator` knows about radians and a clamp and nothing else, and the demo owns
   the eye, the projection and the five matrices.

### Yaw, pitch, and the clamp

**`pitch` is clamped; `yaw` is not.** Each with a reason, because the asymmetry is
the decision.

- **`ROTOR_PITCH_LIMIT = std::f32::consts::FRAC_PI_6` — 30°, symmetric about
  zero.** The measured model (task 39's `model.json` `measured.bounds_min` /
  `bounds_max`) is **1.50 m wide × 2.55 m long × 1.30 m tall**, so its projected
  silhouette is square at `atan(1.30 / (2.55 / 2)) ≈ 45.5°`. **A pitch at or
  above that angle reads as a plan view of the roof, and at a plan view the four
  wheels are four indistinguishable discs** — so a limit below it is a limit on
  whether the object stays legible as a car. **30° is comfortably below 45.5° and
  is a round number in degrees**, and it is the angle at which the roof and both
  flanks are visible at once, which is the whole reason a second axis exists.
  **`atan` is the arithmetic, not an assertion about how the car looks**: a
  reviewer who disagrees with the number can compute it from the extents.
- **Symmetric, not `0.0..=30°`.** Two reasons, and the second is the one that
  matters. First, a drag whose two directions are not equivalent is a drag that
  feels broken, and `Scroll`'s asymmetric `[0, max_scroll]` is right *for a scroll*
  because the top of a document is a real place. Second, and this is the
  geometry: **task 39 recorded `Y = 0` as the ground plane, and this pipeline draws
  no floor** — a backdrop is gap `L1`'s work and it is not this task's. So the
  negative end of the pitch puts the camera *below the ground plane*, in a region
  of the world where nothing is drawn except the car itself — and **the model's
  shells are closed** (39 measured `shells_closed: true`, every welded edge at
  valence exactly 2), so an underbody view shows geometry rather than a hole. The
  negative end is not out of the world; it is under it.
- **The clamp is mandatory rather than cosmetic, and this is the failure it
  prevents.** `Mat4::rotated_x` is finite at every angle — including exactly
  `±π/2`, where the car's length projects to nothing and the picture is a
  1.50 m sliver — so an unclamped pitch produces a **silently wrong picture with
  no GL error and a green suite**, which is the failure shape tasks 34, 35, 36 and
  37 each name in their own words. The clamp is what stops "the camera can point
  anywhere" from being one of them.
- **`yaw` is unbounded, and deliberately not wrapped.** Yaw is periodic and
  wrapping it introduces a discontinuity at `±π` that an animated value would
  cross and an equality assertion would see; **nothing needs a bounded yaw**,
  because the model is closed in yaw, so there is no "wrong side" for it to be on.
  **A test asserts a drag past a full turn does not snap back**, because a wrap is
  the one thing that looks correct and is not.
- **The clamp lives in one place.** `clamp_pitch`, a private free function beside
  the module's other helpers, in the shape `Scroll` puts `clamp_scroll` and
  `gesture_delta` — and `Scroll`'s `gesture_delta` doc records **why that shape is
  the point**: the direction rule written out in two arms and restated in eleven
  tests meant eleven places to find when the convention changed. Here, the three
  producers (a drag, a key press and the demo's ambient term) all reach the clamp
  through `Rotator::rotate_by`, so there is **one** copy of it.
- **`Scroll`'s `bounded` is private and stays private.** It is `0.0..=1.0` for a
  *fraction*, and `clamp_scroll` is a scroll-specific two-value clamp. Importing
  either would be a shared module for one caller, which
  `developer.md` § *Phase 2* (*"No abstraction before the second use"*) refuses.

### The gesture mapping, in full

- **Horizontal drag → yaw, vertical drag → pitch, and the signs are the
  grab-and-turn convention.** `yaw` **decreases** as `delta.x` increases (drag
  right and the car's nose goes right, which from above is clockwise, which means
  the camera orbits the other way), and `pitch` **increases** as `delta.y`
  increases (window space is **y-down** — `Rect.y` is an offset from the top edge —
  so a drag down raises the camera and shows more roof). **Both signs are pinned
  by named tests that state the direction in the test's name**, which is the
  `gesture_delta` discipline applied to a convention this file is choosing.
- **Why grab-and-turn, and why `Scroll`'s operator decision does not transfer.**
  `gesture_delta`'s doc records that the operator chose **"down is later"** on
  2026-09-30 and rejected content-follows-the-finger — **because the wheel arm
  disagreed with it and one control with two directions is worse than either
  convention alone.** The rotator has **no second input** (see below), so the tie
  that decided `Scroll` does not exist here, and grab-and-turn is the only
  convention every 3D viewer on earth uses. **Stated because a reviewer will see
  one gesture rule go one way and the other the other way and assume one of them is
  a bug.**
- **A diagonal drag moves both axes. No axis locking, no dominant-axis selection.**
  And this is the **deliberate opposite** of `Scroll`, whose `on_event` returns
  `false` on a drag whose `delta.y == 0.0` so that *"leaving it unconsumed lets a
  horizontal scroller above take it"* — the right answer for a one-axis control
  **and the wrong answer for a two-axis one**, since refusing the horizontal axis
  is refusing the feature. **The two rules are both right, for different
  reasons, and this file names both** so that neither looks like an inconsistency.
- **A drag with `delta.x == 0.0 && delta.y == 0.0` is not consumed.** It is
  `Scroll`'s rule and the right one: a zero-delta drag changes nothing, and a
  widget that swallows one is a widget something behind it can no longer have.
- **`Scroll` (the wheel event) is not consumed.** There is no documented wheel
  gesture for the visualisation, and adding one would be a **fourth producer for
  two properties with no source** — the exact "invented parameters" problem
  `DEMO_APPLICATION.md` § *Asset requirements* records for colours it could not
  verify.
- **`Pinch` is not consumed, and this is the interesting one.** The manual's
  `[C]` sentence in `DEMO_APPLICATION.md` (the visualisation-resize quote) says
  *"You can pinch to zoom in or out"* — so pinch **is** a documented gesture on
  this surface. **A pinch is a zoom and a zoom is a projection change** (task 36's
  `Mat4`, and `near`/`far` or `fov_y`), **not a rotation**, and this task's
  deliverable is a rotation. So `Pinch` is **declined by name** and the reason is
  recorded; a zoom task is a different task with a different property.
- **`Tap`, `LongPress`, `Swipe`, `KeyUp` and `Text` are not consumed**, and § *What
  this task does to gap `L4`* says why `LongPress` and `Swipe` are the two a later
  task wants.
- **`KeyDown` arrows and the d-pad nudge both axes, and only while `focused`.**
  Three reasons: `Slider` and `Scroll` both do this, and a drag-driven control
  that cannot be driven from a physical control is broken on a head unit that has
  one; it is the **only** interaction route on this host with any recorded
  delivery at all (see below); and it costs about twenty lines plus one test. The
  step is **`ROTOR_KEY_STEP = 0.0873 rad` (5°)** — `Slider`'s own step is `5.0` of
  `0..=100`, so "five" is the number this crate already reaches for — and it goes
  through the same `rotate_by` and therefore the same clamp.

### Two producers, one value

**The rule: the gesture owns the yaw while a drag is in progress; the ambient
producer owns it the rest of the time; and the handover is neither a blend nor a
snap — the ambient term is an accumulation that resumes from wherever the gesture
left the value.**

- **One write path.** `Rotator::rotate_by(dyaw, dpitch) -> bool` is the only thing
  that writes `yaw` or `pitch` from inside the crate. A drag calls it, a key calls
  it, and **the demo's ambient term calls it**. That is what makes "one value, two
  producers" a **structural** fact rather than a sentence: there is no second path
  to the property, so the two producers cannot bypass the clamp and cannot be
  reordered by accident.
- **The ambient producer is the demo's, not the widget's**, and the split is
  **yaw only.** The demo's frame loop, once per frame, does
  `if !rotator.dragging.get() { rotator.rotate_by(AMBIENT_YAW_RATE * seconds, 0.0); }`.
  **Pitch is never ambient-driven**, and the reason is that an ambient pitch would
  integrate one way until it hit the clamp and then stick there — a car that tilts
  forward forever and stops. **So of the two values, one has two producers and one
  has exactly one, and that is the honest shape of the interaction rather than a
  simplification.**
- **The handover is an accumulation, not a cross-fade.** Because the ambient term
  is `yaw += rate · dt` rather than `yaw = rate · t`, a gesture's contribution
  **stays in the value**: drag the car 90°, release, and it resumes turning from
  90°, not from zero. **No blend, because two producers writing one value with
  different weights is a second thing to get right and nothing here needs it**, and
  **no snap-back, because "return to the rest pose on release" needs a rest pose
  and this demo has none** — `CAR_REST_YAW` is an arbitrary choice this file makes
  for a flattering first frame, not a pose the product has.
- **`dragging` is written by the demo from a press and a release**, in exactly the
  way `Slider::dragging`'s doc says it must be: the gesture recogniser reports a
  tap on the *release* and a drag only once the pointer has moved, so the flag has
  to come from `MouseButtonDown` / `FingerDown` and its counterparts. **The widget
  never writes `dragging` itself.**
- **`AMBIENT_YAW_RATE = 0.35 rad/s`** — 20°/s, a turn in about eighteen seconds.
  Slow enough that the car is clearly turning in a capture rather than strobing,
  fast enough that two captures a second apart differ visibly in the car rect.

### The central constraint: no pointer event reaches this window

**This is the shape of the whole task, and it is a measured fact about this host
rather than an assumption.**

`.ai/tools/README.md` § *Capturing a window* records, dated and attributed, that **XTEST pointer injection has never
delivered an event to the window**, across several sessions by several agents: a
drag along the slider's track gave `magick compare -metric AE` = **0** against the
capture before it; two presses on the button task 12 verified on screen twice gave
**AE = 0** with the counter still reading `0 clicks`; and `XQueryPointer` after a
fake motion reported the pointer at window `0x0`. Keyboard injection has delivered
**exactly one** event in this project's history — the positive control `T`, which
moved 212 px — and `IMPLEMENTATION_STATE.md` records the task 14 outcome as a
**waiver on five of eight acceptance criteria**, every one of which needed a
pointer or a key.

**So this task is designed to be verifiable without a pointer event, and says so
rather than claiming a capture proves it:**

1. **The gesture logic is in `ui_core` and is fully unit-tested through the crate's
   own event path.** A test constructs an `InputEvent` carrying
   `InputEventKind::Drag { delta }` and calls `Rotator::on_event(&mut event, rect)`
   directly, in the shape `Slider`'s and `Scroll`'s own doc examples already use.
   **This is legal under `AGENTS.md`'s no-display rule and it is the technique
   tasks 14 to 23 used for their unwinnable criteria.**
2. **The demo drives the rotation from whatever the gesture produced, and the
   ambient producer runs whether or not a drag ever arrives** — so the mesh pass,
   the per-frame matrix composition and the model are exercised on every run, on
   this host, with no input at all.
3. **The label on every claim is the one it earns.** What is verified by test is
   named as verified by test. What is verified by capture is named as verified by
   capture. What is **not** verified at all — *that a pointer event reaches this
   window and a finger turns the car* — is **not claimed, and an acceptance
   criterion here does not require it.** The gates in `.ai/workflows/task-sequence.md`
   § Gates forbid evidence by assertion, so a criterion that cannot be met is not
   written; **an acceptance criterion that names an instrument which cannot
   produce the evidence is not met by producing the evidence another way** is the
   rule this file obeys by leaving the criterion out and recording the
   gap instead.

### What this task does to gap `L4`, and the honest limit

**It closes no part of row `L4`, and this file must not be read as saying it does.**

Row `L4` in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* reads
*"**`LongPress` and `Swipe` are emitted and consumed by no widget**"* and its
**Blocks** column names *"Dock edit mode, card paging, alert dismissal, the
drive-mode strip"*. **Neither of those two variants is touched here**, and none of
those four blocked items is delivered.

What **is** true afterwards is narrower and is worth exactly this much:

- **`Drag` gains its first two-axis consumer.** Today the drag-driven values in
  the crate are all one-axis scalars (`Scroll::scroll_offset`, `Slider::value`).
- **One more widget module exists**, so `widgets/mod.rs` carries **sixteen**
  `pub mod` entries where row `L4`'s evidence sentence says fifteen, and there are
  **nine** `on_event` impls in the crate where its evidence says eight.
- **`Swipe` and `LongPress` remain unconsumed, and the row remains true.**

So requirement 13 **amends row `L4`'s evidence line and adds a dated note** — in
the form `DEMO_APPLICATION.md` § *Corrections to the second gap table* prescribes
— rather than marking the row closed or deleting it. **And the amendment is
enforced by a test, not by prose**: requirement 12's
`the_widget_consumes_nothing_but_a_drag` asserts that a `Swipe` and a `LongPress`
offered to `Rotator::on_event` are **not** consumed, so a later task that does
consume them must break that test deliberately rather than drift past a claim in a
document.

**Why this task does not consume `Swipe`, decided and named.** `Swipe` carries a
`SwipeDirection` — `Left`, `Right`, `Up`, `Down` — and **no magnitude and no
duration**; a rotation needs an angle. Mapping `Swipe` onto the rotator would
therefore require inventing a magnitude (from what? elapsed time? a fixed step?)
and a direction-to-axis mapping (`Up` → pitch? `Right` → yaw? which sign?), **and
every one of those would be a parameter with no source.** Three consumers want
`Swipe` and **none of them is this one**: dock edit mode, alert dismissal and card
paging are all row `L4`'s own three *Blocks* entries, and all three are
**discrete, threshold-based decisions** where the gesture's semantics ("travelled
far enough in one direction") is exactly what is wanted — which is not true of a
continuous orbital camera. **Leaving `Swipe` for the task that needs it is the
cheaper decision and the more honest one.**

### The demo must draw something, or the mesh pass is still untested

**Tasks 34, 35, 37 and 38 each recorded "no page records a mesh command", and 39
recorded that it has no runtime code at all.** So **this is the first task in the
sequence that records a `DrawCommand::Mesh` anywhere in this repository** — and
that is a scope consequence to state rather than an addition to apologise for:
a rotation nobody can see is a rotation that does not exist.

It has one consequence the earlier tasks never had, and it is the reason this
file's capture criterion is worded the way it is:

- **The six gallery pages cannot all be pixel-identical, and this file does not
  claim they are.** The car is drawn on **`Page::Data`**, in **one new rect**,
  and the criterion is restated as *"identical outside the fps readout's band and
  outside that one rect"*. **The mechanism, stated and true, is three facts and not
  one comfortable one:**
  1. **No rect the gallery placed moves.** `CAR_ORIGIN` and `CAR_SIZE` are new
     constants in the `data` page's free band, and
     `every_page_places_every_rect_where_the_gallery_placed_it` keeps its name and
     every one of its assertions.
  2. **Five of the six pages record no mesh command at all**, so on those five the
     criterion is exactly the one tasks 34 to 39 inherited: **AE 0 outside
     `y ≥ 680`**, every differing pixel inside the fps band.
  3. **On the sixth page the car rect differs by construction, every frame** —
     the ambient rotation is *supposed* to move the pixels there. So the band is
     the union of `y ≥ 680` and the car rect, and the criterion says so with the
     rect **named as a constant** so a reviewer can compare against the number the
     code uses rather than against an eyeballed region.
- **And the scissor is what makes the third fact a rect rather than the window.**
  `Demo::frame_clips` currently returns `None` for **every** node, so the demo
  clips nothing today (`DEMO_APPLICATION.md` § *Corrections to the second gap
  table* records that this was narrowed to "one widget" when the list still
  existed, and the list was removed on 2026-10-02). **Task 37's `draw_mesh_batch`
  calls `apply_clip(batch.clip)` — a clip that does not exist is no clip** — so
  requirement 8 makes `frame_clips` return `Some(CAR_RECT)` for the car's node and
  `None` for everything else. **Without that, a car drawn unclipped would spill
  across the tab bar and the chrome**, which is both a visible defect and a
  violation of the recording-order contract task 37 recorded. **This is the first
  thing in the pipeline that a scissor actually matters to**, which is why it is
  this task's and not a leftover.

### Scope, measured against `developer.md` § *Scope check*

**Five files, three components, at both thresholds and neither over.** Files:
`ui/src/ui_core/src/widgets/rotator.rs` (new), `ui/src/ui_core/src/widgets/mod.rs`
(one `pub mod` line), `ui/src/ui_demo/src/main.rs`, `doc/ui/DEMO_APPLICATION.md`
(row `L4`), `doc/ui/IMPLEMENTATION_STATE.md` (the record). Components: the
gesture-to-angle arithmetic and the clamp in `ui_core`; the demo's orbit, matrix
and page wiring; the documentation amendment and the record. **No fan-out is
needed** — `.ai/protocols/subagents.md` § *Implementation fan-out* does not apply —
**and if the implementer finds themselves editing a fourth *code* file, that is a
stop condition rather than an expansion** (`developer.md` § *Stop conditions*).

## Requirements

1. **A new widget `ui/src/ui_core/src/widgets/rotator.rs`, declared with
   `pub mod rotator;` in `ui/src/ui_core/src/widgets/mod.rs`** beside the fifteen
   that are there, so the list carries **sixteen** entries and the new name is
   **alphabetical** (`button`, `chart`, `container`, `dialog`, `gauge`, `image`,
   `keyboard`, `label`, `list`, `progress`, **`rotator`**, `scroll`, `slider`,
   `text_input`, `toast`, `toggle`).

   ```rust
   /// Two angles — a yaw about y and a clamped pitch about x — and nothing else.
   #[derive(Clone, Debug)]
   pub struct Rotator {
       /// Yaw in radians, **unbounded and never wrapped**.
       pub yaw: Property<f32>,
       /// Pitch in radians, **clamped** to `-ROTOR_PITCH_LIMIT..=ROTOR_PITCH_LIMIT`.
       pub pitch: Property<f32>,
       /// Whether a pointer is down on this rotator. **Written by the caller**
       /// from a press and a release, never by the widget.
       pub dragging: Property<bool>,
       /// Whether the rotator holds focus. Written by the caller from
       /// [`input::Focus`](crate::input::Focus), as `Slider::focused`'s doc says.
       pub focused: Property<bool>,
       sensitivity: f32,
       pitch_limit: f32,
       node: Handle,
   }
   ```

   - `pub fn new(nodes: &mut Arena<WidgetNode>) -> Self` — creates its node through
     `node::create` the way `Slider::new` does, with `yaw` at `YAW_REST`,
     `pitch` at `PITCH_REST`, `dragging` and `focused` **false**, `sensitivity` at
     `ROTOR_SENSITIVITY` and `pitch_limit` at `ROTOR_PITCH_LIMIT`. **`new` takes no
     tuning arguments**: a widget whose limits are arguments is a widget whose
     limits two callers disagree about, and this project has one rotator.
   - `pub fn handle(&self) -> Handle`, `#[must_use]`, in `Slider`'s and `Scroll`'s
     shape.
   - **`pub fn yaw_limit(&self) -> f32` and `pub fn pitch_limit(&self) -> f32`,
     both `#[must_use]`**, because a caller placing a needle next to the rotator
     needs to know where the ends are and there is no other way to ask.
   - **No `paint`.** A rotator has **nothing of its own to draw** — the car is drawn
     by `DrawCommand::Mesh` through the demo's `Mat4`, and a two-axis value with no
     surface is a state holder. **`paint` returning an empty `Vec<DrawCommand>`
     would be a method whose body is `Vec::new()`**, which is the shape a reader
     checks for content and finds none.
   - **No `AnimationClock`, no `tick`, no `animate_to_state`.** Not an omission:
     every one of those exists to drive a **transition**, and this widget has none
     (see requirement 4 and § *Out of Scope* on why). **A `tick` over an empty
     clock is a method that returns `false` forever**, and adding one because every
     sibling has one is a shape with no content. `Property` with no animation on it
     is a value, and that is what this is.

2. **The constants, `pub`, in the module's own block, each doc-commented with the
   arithmetic in the same voice as `MULTISAMPLE_SAMPLES` and `IMAGE_VERTEX_STRIDE`:**

   - `pub const ROTOR_SENSITIVITY: f32 = 0.0025;` — radians per pixel of drag
     (≈ 0.143°/px). **Doc comment carries the derivation:** across the demo's
     `WINDOW` width of 1280 px a full-width drag turns the car `1280 × 0.0025 =
     3.2 rad ≈ 183°`, so a comfortable quarter turn is about **630 px**, a bit
     under half the panel — the reach a thumb covers on a centre display — and
     **209 px** of vertical travel reaches the pitch limit, which is **why the
     clamp is reachable in well under a third of a half-height drag and therefore
     not a corner case**.
   - `pub const ROTOR_PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_6;` — 30°. Doc
     comment carries the `atan(1.30 / 1.275) ≈ 45.5°` silhouette-square argument
     from § *Yaw, pitch, and the clamp*, the symmetry, and what would reverse it
     (a measurement that 30° hides a surface the demo needs to show).
   - `pub const ROTOR_KEY_STEP: f32 = 0.0873;` — 5° per key press, with `Slider`'s
     own `SLIDER_STEP = 5.0` of `0..=100` named as the precedent.

   **A test asserts all three are finite and positive** and that
   `ROTOR_PITCH_LIMIT < FRAC_PI_4`, because a limit at or above the
   silhouette-square angle defeats the reason it exists — the same shape as task
   34's `DEPTH_BITS` test, which admits `16`, `24`, `32` and **not `0`**.

3. **`Rotator::on_event(&self, event: &mut InputEvent, rect: Rect) -> bool`**,
   the signature `Slider::on_event`, `Scroll::on_event` and `Toggle::on_event` all
   use, with exactly two arms and a `_ => false` for the other seven variants:

   - **`InputEventKind::Drag { delta }`** — **no focus required**, because a drag
     is positional and `Scroll` takes one from an unfocused scroller. **A drag with
     `delta.x == 0.0 && delta.y == 0.0` returns `false` unconsumed**, `Scroll`'s
     rule and for `Scroll`'s reason. Otherwise: `event.consume()`, then
     `self.rotate_by(-self.sensitivity * delta.x, self.sensitivity * delta.y)`,
     then `true`. **`rect` is not read by this arm** — a rotation is a delta, not a
     position — and the doc comment says so, because an unused parameter reads as
     an oversight and a reviewer will grep for the use.
   - **`InputEventKind::KeyDown { key, .. }`** — `if !self.focused.get() { return
     false; }`, then a **private `fn rotation_key(key: &Key) -> Option<(f32, f32)>`**
     in `Slider::adjustment`'s shape, mapping
     `Key::Keyboard(Keycode::Left/Right)` and
     `Key::Gamepad(Button::DPadLeft/DPadRight)` to `(±ROTOR_KEY_STEP, 0.0)` and
     `Key::Keyboard(Keycode::Up/Down)` and `DPadUp/DPadDown` to
     `(0.0, ±ROTOR_KEY_STEP)`; `None` returns `false` unconsumed, and a hit
     consumes and calls `rotate_by`. **The sign convention is stated in the
     helper's doc comment in one sentence and pinned by one test**, which is
     `gesture_delta`'s discipline: a sign written out in two arms and restated in
     several tests is eleven places to look when the convention changes.
   - **Seven variants reach `_ => false`:** `Tap`, `LongPress`, `Swipe`, `Pinch`,
     `KeyUp`, `Scroll` and `Text`. **Each of the seven is named in the arm's doc
     comment with why it is declined**, and `Pinch`, `LongPress` and `Swipe` get
     the reasons from § *The gesture mapping, in full* — a pinch is a zoom and a
     zoom is a projection change; `LongPress` and `Swipe` belong to the discrete
     decisions row `L4`'s **Blocks** column names.

4. **`pub fn rotate_by(&self, delta_yaw: f32, delta_pitch: f32) -> bool`,
   `#[must_use]`, and it is the ONLY thing that writes either property.** It
   clamps the pitch through the private `clamp_pitch`, writes both, and **returns
   whether either value moved**. Its doc comment states the three callers — a drag,
   a key press, and a caller's own integration such as the demo's ambient turn —
   and says in the first line that this is the single write path, because
   **"one value, two producers" is only a structural fact while there is one way in.**
   **`Scroll::scroll_by` is the precedent and its shape is the model**: it clamps,
   writes the settled value and returns the new offset.

5. **`fn clamp_pitch(pitch: f32, limit: f32) -> f32`**, private to the module, in
   `Scroll::bounded`'s shape and **not imported from `scroll.rs`** (requirement: see
   § *Yaw, pitch, and the clamp*'s last bullet). It clamps with `f32::clamp` on a
   finite `limit` and **returns `0.0` for a non-finite input** — the arithmetic that
   task 36 requires of `perspective` and `orthographic`, where a non-finite angle
   is `inf` and not a panic, so a non-finite pitch is `NaN` and a `NaN` in a matrix
   makes the geometry it belongs to disappear with no GL error. **Both branches are
   unit-tested.**

6. **`rotator.rs`'s module doc**, in the form `render/meshio.rs`'s and
   `render/mesh.rs`'s carry, recording: what the widget is and **what it is not**
   (no matrix, no projection, no mesh, no `GL_` call, no paint); the decision to be
   a crate widget rather than demo-local state, with the four reasons from
   § *The decision*; **why pitch is clamped and yaw is not**, with the `atan`
   arithmetic; the grab-and-turn convention **and that `Scroll`'s "down is later"
   does not transfer**, with the reason; **that there is no inertia, no fling, no
   snap and no release animation, and why each is absent** (the crate has none
   anywhere; a snap needs a preset set the demo does not have; a release animation
   needs a rest pose there is no source for); and the fact that **`rect` is
   unused by the drag arm**.

7. **The demo's placement constants, all in `ui/src/ui_demo/src/main.rs`, all
   doc-commented in the voice the neighbouring constants carry:**

   - `const CAR_ORIGIN: (f32, f32) = (60.0, 240.0);` and
     `const CAR_SIZE: Size = Size { width: 560.0, height: 400.0 };` — **on the
     `data` page's free band**, chosen so the rect **overlaps nothing the gallery
     placed**: the gauge begins at `GAUGE_ORIGIN.0 = 664`, the chart at
     `CHART_ORIGIN.0 = 1000`, the image at `IMAGE_ORIGIN.0 = 800`, and the fps
     readout is at `y = 684`, so a rect ending at `y = 640` is clear of the fps
     band the capture criterion names. **The implementer confirms the choice
     against `no_two_placed_rects_overlap`, which is what pins it** — and that
     test, plus the completeness assertion, are requirement 10's obligation.
     **The numbers are this file's proposal, not a measurement of free space**, and
     the doc comment says so.
   - `const CAR_TARGET_Y: f32 = 0.65;` — the model's centre in y, from 39's
     `bounds_min.y = 0.0` and `bounds_max.y = 1.3`.
   - `const CAR_DISTANCE: f32 = 2.6;` — **metres, and derived in the doc comment**:
     with 36's `fov_y = π/4` the frustum's vertical half-extent at distance *d* is
     `0.414 d`, so at `d = 2.6` the visible height is 2.15 m (the 1.30 m car is
     **60 %** of the 400 px rect) and, at `aspect = 560/400 = 1.4`, the visible
     width is 3.02 m (the 2.55 m car is **84 %** of the 560 px rect). **36's
     `near = 0.1` and `far = 20.0` are used unchanged**, which is the promise 36's
     module docs made to this task by name.
   - `const CAR_REST_YAW: f32 = 0.6;` and `const CAR_REST_PITCH: f32 = 0.21;` —
     radians, ≈ 34.4° and ≈ 12°, **a three-quarter-from-above resting view**. The
     doc comment states plainly that **the model's facing is recorded nowhere in
     this repository** — 38 and 39 record extents and node translations, not which
     way the nose points — so **which yaw sign produces a front three-quarter is
     settled from the first capture and the handoff records the value it used.**
     This is the honest form of the sentence: the file does not assert a direction
     it cannot verify.
   - `const AMBIENT_YAW_RATE: f32 = 0.35;` — rad/s, with § *Two producers, one
     value*'s derivation.

8. **The clip, and this is the first thing in the pipeline that needs one.**
   `Demo::frame_clips` currently returns `None` for every node; it must return
   **`Some(CAR_RECT)` for the car's node and `None` for every other node**, which
   is a one-line change to a function whose doc comment says it is *"the whole of
   the frame's clipping, in one place"*. The doc comment gains the reason in its
   first lines: **task 37's `draw_mesh_batch` calls `apply_clip(batch.clip)`, a
   clip that does not exist is no clip, and an unclipped car would draw across the
   tab bar and the chrome** — a visible defect and a violation of the recording
   order task 37 made the contract. **`frame_clips` keeps its signature, keeps its
   one-function shape, and the existing tests on it keep their names and their
   assertions**; the new row is added to the expected set rather than the old set
   loosened.

9. **The demo's press, release and route arms, all inside the existing
   `handle_event` chains and none of them outside the modal guard.** `Demo` gains a
   **`car: DemoCar`** in `DemoSlider`'s exact shape — a two-field wrapper whose
   `node()` delegates to `widget.handle()` — so `self.car.node()` and
   `self.slider.node()` are the same kind of thing and `placed_handles`,
   `on_show` and `focus_navigation` all read it the way they already read the
   slider's. Then:

   - **`Demo::car_at(&self, x: f32, y: f32) -> Option<()>`**, in `Demo::slider_at`'s
     exact shape — `if !self.on_show(self.car.node()) { return None; }` then
     `self.car_rect().is_some_and(|rect| over_rect(rect, x, y)).then_some(())`.
     **The page gate is not optional**: without it a drag on the `data` page's car
     would still be reachable from a page that does not draw it.
   - **`Demo::car_rect(&self) -> Option<Rect>`**, in `Demo::slider_rect`'s shape.
   - **A `press_car` / `release_car` pair**, added as **one `else if` in the
     existing `MouseButtonDown` chain and one line in each of the three release
     arms** (`MouseButtonUp`, `FingerUp`, `FingerCanceled`). **The press arm is
     guarded by the same `dialog_is_modal()` chain as the four existing press
     arms**, on their recorded argument: a press that reaches the gallery directly
     bypasses `Demo::route_input_event`'s tap-side filter, and a press answered
     under a modal scrim is the defect the guard exists for. **The release arms are
     unguarded, like every other release in the file**, on the `release_all`
     argument the file already states.
   - **The chain position is stated and its premise is asserted.** The new arm goes
     **after the four existing press arms**; because `CAR_RECT` intersects none of
     `pad_at`, `slider_at`, `keyboard_at` or `tab_at`, the position is not
     load-bearing — **and a test asserts that premise by name** (the same move as
     `nothing_the_demo_places_reaches_into_the_strip`, which pins the premise the
     tab bar's fallback guard rests on rather than the guard).
   - **One arm in `Demo::route_input_event`**, beside the slider's: *"a drag goes
     to the rotator being dragged first, whether or not the pointer is still over
     it"* — `if self.car_dragging && matches!(event.kind(),
     InputEventKind::Drag { .. }) { … rotator.on_event(event, rect) … }`. **The
     reason is the file's own and it is quoted in the comment:** the routed chain
     finds the node under the pointer, and a finger that has travelled past the
     end of the car rect is outside it, which is exactly when the car most needs
     to hear about the drag.

10. **`Demo::frame`'s ambient term, in this order and no other:** after the existing
    per-widget ticks and **before the paint pass**, one block that reads
    `if !self.rotator.dragging.get() { self.rotator.rotate_by(
    AMBIENT_YAW_RATE * delta.as_secs_f32(), 0.0); }`. **The doc comment carries
    the two reasons this is a write and not an `animate_to`:** the crate's idiom is
    "aim once, tick per frame" and a per-frame aim would creep toward its target
    and never arrive (`Demo::sync_toggle_state`'s argument, cited by name), and an
    *accumulation* rather than an absolute assignment is what lets a gesture's
    contribution survive the handover.
    **There is no `tick` for the rotator to call** (requirement 1), and the
    handoff says so rather than leaving a reader looking for one.

11. **The paint, on the `data` page only, recorded before the page's chrome and
    after the page's background** — the ordering contract task 37 recorded, which
    is *"record the map before the mesh and the chrome after it"*, and which for
    this page means the car is recorded **before** the gauge, chart, image and
    readouts so the chrome is above it. **Five commands, one per sub-mesh** in
    `Model::ranges`' file order, because task 35's ranges are what selects a part
    and `DrawCommand::Mesh` carries one range per command. **All five carry the
    same `mvp`** — the model's own placement is task 39's baked node transforms, so
    no per-sub-mesh transform is applied and **there is no wheel spin** (§ *Out of
    Scope*). `tint` is the premultiplied white the colormap needs and `opacity` is
    `1.0`, so the mesh batches `Opaque` and writes depth per task 37's table.

    The matrix composition, in `Demo`'s own private helper so it is a pure function:

    ```rust
    let [eye_x, eye_y, eye_z] = orbit(yaw, pitch, CAR_DISTANCE);
    let view = Mat4::identity()
        .rotated_x(pitch)
        .rotated_y(yaw)
        .translated([-eye_x, -eye_y, -eye_z]);
    let model = Mat4::identity();
    let mvp = projection.multiply(&view).multiply(&model);
    ```

    **`Mat4`'s post-multiplying convention is what makes that order right**, and
    the doc comment says so: `rotated_x` then `rotated_y` then `translated` is
    `R_x · R_y · T`, which is the one order that aims the camera **at the target**
    — and 36's `Mat4` has **no `look_at`**, so this composition is the camera's
    only construction. `projection` is
    `Mat4::perspective(std::f32::consts::FRAC_PI_4, CAR_SIZE.width / CAR_SIZE.height,
    0.1, 20.0)`, and **the `Option` is handled, not unwrapped** — 36 returns `None`
    on every violated precondition, so the helper returns early with a printed
    reason rather than drawing with a stale matrix.

    **`fn orbit(yaw: f32, pitch: f32, distance: f32) -> [f32; 3]`, private to the
    demo and free of GL and of `Mat4`**, so the geometry is unit-testable:

    ```rust
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    [
        CAR_TARGET_Y + distance * sp - distance * sy * cp,
        distance * cp * cy,
    ]
    ```

    **The two facts the doc comment must state, because both are invisible in the
    arithmetic and both are what a reader will get wrong:** with `pitch > 0` the
    **eye rises above the target** and looks down — so *drag down shows more roof*
    — and **the `-` on the eye is `target − distance · direction`, not `+`**, which
    is what puts yaw 0 with the camera on `+z` looking toward `−z` and makes the yaw
    sign the convention it is rather than an accident. **`CAR_TARGET_Y` is the
    target's own y and is *added* to `distance · sin(pitch)`**, so a helper that
    returned a world-space eye by forgetting the target would sit 0.65 m low and
    show the car from below the beltline.

    **`u_normal_matrix` is the identity 3×3, and the reason is in the comment.**
    Task 37 requires a `mat3` normal matrix because a mesh fragment's normal must
    not be transformed by the MVP. Here the model matrix **is** the identity and the
    view is a rigid rotation plus a translation, so the model→world 3×3 is the
    identity and the demo uploads nine ones. **And the honest limit is stated in
    the same comment**: if a later task rotates the *car* rather than orbiting the
    camera, the normal matrix stops being the identity and needs a real inverse
    transpose, which `Mat4` does not have.

12. **The tests, named, with no display, no network, no filesystem and no wall
    clock** — the only kind `AGENTS.md` permits, and the only verification this
    host can offer for the interaction. In `rotator.rs`'s `#[cfg(test)] mod tests`:

    - `a_horizontal_drag_turns_the_yaw_and_leaves_the_pitch_alone` — a synthetic
      `InputEventKind::Drag { delta: Offset::new(60.0, 0.0) }` driven through
      `on_event`; asserts it returned true, the event was consumed, the yaw moved
      by **exactly** `-60.0 × ROTOR_SENSITIVITY` and the pitch is **bit-identical**
      to its value before.
    - `a_vertical_drag_tilts_the_pitch_and_leaves_the_yaw_alone`, and
      `a_diagonal_drag_moves_both_axes` — the third is the contrast with `Scroll`'s
      horizontal refusal, and its name says what it is for.
    - **`the_pitch_cannot_exceed_its_clamp_under_an_extreme_drag`** — the required
      one, in two halves: a **single** `Drag` of `Offset::new(0.0, 100_000.0)`, and
      **one thousand** successive 1.0-px drags in the same direction. Both assert
      `pitch.get() <= ROTOR_PITCH_LIMIT` and `>= -ROTOR_PITCH_LIMIT` **exactly**,
      and that **a further drag in the same direction leaves it unchanged** — so
      the assertion catches a clamp that is a slow leak as well as one that is
      absent.
    - `the_pitch_is_clamped_at_both_ends` — the negative end, with the same
      two-half shape.
    - `clamp_pitch_returns_zero_for_a_non_finite_pitch` — both branches.
    - `a_drag_with_no_component_is_not_consumed` — `Offset::new(0.0, 0.0)`
      returns false and leaves the event unconsumed.
    - `the_yaw_is_not_wrapped` — a drag long enough to pass a full turn asserts the
      yaw is **still past** `2π` rather than back near zero.
    - **`the_widget_consumes_nothing_but_a_drag`** — a table over **all nine**
      `InputEventKind` variants: only `Drag` and a focused `KeyDown` are consumed,
      and **`Swipe` and `LongPress` are asserted not consumed by name**. This is
      the test that makes § *What this task does to gap `L4`*'s claim checkable
      rather than documentary.
    - `the_arrow_keys_nudge_both_axes_and_stop_at_the_same_clamp` — driven through
      `on_event`, requires `focused`, and drives the pitch past the limit in
      thirty presses to prove the key path clamps identically.
    - `an_unfocused_rotator_declines_a_key_and_lets_it_travel_on`, in
      `Slider::on_event`'s own doc-example shape.
    - `the_three_constants_are_positive_finite_and_under_a_quarter_turn`.
    - `the_drag_arm_needs_no_focus` — a drag acts on an unfocused rotator, with
      `Scroll`'s one-axis behaviour named as the precedent.

    In the demo's suite:

    - `a_drag_over_the_car_turns_it_and_touches_nothing_else` — a synthetic
      `Drag` offered through **`Demo::offer_to`**, the demo's own dispatcher, so the
      event travels the crate's route rather than a test's; asserts the yaw moved
      and that **`placed_rects` is unchanged**.
    - `the_ambient_turn_holds_the_car_when_nothing_is_dragging` — a `frame` with a
      delta and no drag, asserting `yaw` advanced by
      `AMBIENT_YAW_RATE × seconds`.
    - **`the_gesture_stops_the_ambient_turn_and_the_ambient_resumes_from_where_it
      _left_off`** — the precedence test: drag, release, one frame; the yaw is
      where the drag left it **plus** one ambient step, not reset.
    - `the_ambient_turn_never_moves_the_pitch` — the one-producer fact.
    - `the_car_node_is_a_member_of_the_data_page_and_of_no_other`.
    - `the_car_rect_overlaps_nothing_the_gallery_placed` — the premise requirement 7
      rests on, by name, so the chain position and the clip region are both pinned
      to a number rather than to a reading of the layout.
    - `the_view_puts_the_orbit_target_on_the_camera_axis` — **the sign test**, and
      the one that kills the whole class of orbit errors: `view.transform_point(
      [0.0, CAR_TARGET_Y, 0.0])` is `[0.0, 0.0, -CAR_DISTANCE]` within the demo's
      epsilon, for **yaw = 0, pitch = 0** and for **a non-zero yaw and pitch**.
      **A single assertion a reviewer should break first**, because an orbit with
      the wrong sign or the wrong order produces a plausible wrong picture and no
      error anywhere.
    - `the_car_records_five_mesh_commands_each_with_a_range_and_one_mvp` — the
      command-level half, matching task 34's rect-level half for the 2D pages.

13. **The `L4` amendment, dated and attributed.** Row **`L4`** in `DEMO_APPLICATION.md`
    § *Gaps this layout exposes in `ui_core`* gains a dated note recording that
    **task `TASK_UI_PRIM_40` added `Rotator`, the crate's first two-axis
    drag-driven value**, that **`LongPress` and `Swipe` remain unconsumed and the
    row's claim is unchanged**, and **why** (this task's § *What this task does to
    gap `L4`*: a `Swipe` has no magnitude, and the three consumers row `L4` names
    are discrete decisions rather than a continuous camera). **Its evidence line's
    two counts are corrected in place** — *"`widgets/mod.rs` lists sixteen `pub mod`
    entries"* and *"zero match arms in all nine `on_event` impls"* — because a
    count that is wrong is the failure `DEMO_APPLICATION.md`
    § *Corrections to the second gap table* was written about. **The row is not
    deleted, not marked closed, and its Blocks column keeps all four entries**,
    because none of them is delivered. **`LongPress` and `Swipe` consumption is
    named as the next task's, and `Pinch` is named as a zoom task's.**

14. **`doc/ui/IMPLEMENTATION_STATE.md` gains one entry**, carrying: the widget name
    and the three constants with their arithmetic; **the widget-vs-demo-local
    decision and the row `L4` amendment's date**; the two-producer precedence rule
    in one sentence; the grab-and-turn convention and that `Scroll`'s does not
    transfer; **the drag arm not reading `rect`**; the **no-inertia / no-snap /
    no-release-animation** statement; the model's **facing not being recorded
    anywhere** and which yaw the capture settled on; the clip change to
    `Demo::frame_clips`; the six pages' frame rates and the `data` page's separately;
    and **the honest limit, in the section's own register** — *the gesture logic is
    unit-tested through the crate's own event path and the demo's ambient turn
    moves the car on every run, and **no pointer event has ever been observed
    reaching this window**, so nothing here is evidence that a finger turns it.*
    `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`.ai/workflows/task-sequence.md` § *State*); it points at the code.

15. **The suite, the capture and the frame rate are all produced.** From `ui/`:
    `cargo fmt --check`, `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings`,
    `cargo test --all-features` with the per-binary counts pasted and **no test
    deleted, renamed away or weakened**, `cargo doc --no-deps` clean, and
    `cargo audit` **recorded as not installed on this host, not passed**. Then the
    six-page before/after capture of § *The demo must draw something*, and then
    **the frame rate on all six pages**. **`gl.get_error()` is read once after the
    first frame that draws the mesh** and the result pasted with the instrument's
    code quoted: `bind_attach`'s doc records that a rejected call with nobody reading it dropped
    a whole pass, **and a mesh pass that silently draws nothing is the largest
    version of that failure this sequence has produced.**

## Acceptance Criteria

- [ ] **`Rotator` exists and is the sixteenth `pub mod`.**
      `grep -c 'pub mod' ui/src/ui_core/src/widgets/mod.rs` returns **16** (was
      **15**, quoted here so the number means something), and
      `grep -n 'pub mod rotator' ui/src/ui_core/src/widgets/mod.rs` shows the line.
      `awk '/^pub struct Rotator/,/^}/' ui/src/ui_core/src/widgets/rotator.rs`
      shows **`yaw`, `pitch`, `dragging`, `focused`** and the three private fields,
      and nothing else

- [ ] **The three constants are named, documented and pinned by a test.**
      `the_three_constants_are_positive_finite_and_under_a_quarter_turn` asserts
      `ROTOR_SENSITIVITY == 0.0025`,
      `ROTOR_PITCH_LIMIT == std::f32::consts::FRAC_PI_6` and
      `ROTOR_KEY_STEP == 0.0873`, that all three are finite and positive, and that
      `ROTOR_PITCH_LIMIT < std::f32::consts::FRAC_PI_4` — **the last clause is the
      silhouette-square argument as an assertion**, so a future edit that widens the
      limit past `atan(1.30 / 1.275) ≈ 45.5°` fails rather than passing quietly.
      **Mutation evidence in the handoff:** change `ROTOR_PITCH_LIMIT` to
      `FRAC_PI_4` and watch the suite fail for that reason; restore it and watch it
      pass. A test that has never failed is a hypothesis
      (`developer.md` § Phase 3)

- [ ] **A synthetic drag moves the yaw, and only the yaw.**
      `a_horizontal_drag_turns_the_yaw_and_leaves_the_pitch_alone` constructs
      `InputEventKind::Drag { delta: Offset::new(60.0, 0.0) }`, drives
      **`Rotator::on_event`**, and asserts it returned `true`, the event was
      consumed, the yaw moved by **exactly** `-60.0 * ROTOR_SENSITIVITY`, and the
      pitch is **bit-identical** to its value before the call.
      `a_vertical_drag_tilts_the_pitch_and_leaves_the_yaw_alone` is the same shape
      transposed, and `a_diagonal_drag_moves_both_axes` asserts both moved.
      **The mechanism is named because it is what makes the claim legal:** no
      display, no window, no event loop — `AGENTS.md` forbids all three and
      requirement 12's tests use none of them

- [ ] **The pitch cannot exceed its clamp under an extreme drag, and the test can
      fail.** `the_pitch_cannot_exceed_its_clamp_under_an_extreme_drag` runs **both**
      halves — a **single** `Drag` of `Offset::new(0.0, 100_000.0)` and **one
      thousand** successive 1.0-px drags in the same direction — and asserts in each
      that `pitch.get() <= ROTOR_PITCH_LIMIT` and `>= -ROTOR_PITCH_LIMIT`
      **exactly**, and that **a further drag in the same direction leaves the value
      unchanged**, so a clamp that leaks is caught as well as one that is absent.
      `the_pitch_is_clamped_at_both_ends` is the same shape on the negative end.
      **Mutation evidence is in the handoff:** make `clamp_pitch` return its input
      unchanged and watch both tests fail for that reason; restore it and watch them
      pass. The extreme-drag half is the one a reviewer should break first, because
      a clamp tested only with a small drag passes for a clamp that is off by a
      little

- [ ] **`rotate_by` is the only write path, and the two producers both go through
      it.** `grep -n 'yaw.set\|pitch.set' ui/src/ui_core/src/widgets/rotator.rs`
      returns **exactly the two lines inside `rotate_by`**, and the same grep over
      the demo returns no write to either property. `the_gesture_stops_the_ambient
      _turn_and_the_ambient_resumes_from_where_it_left_off` asserts the precedence
      end to end: a drag turns the car, the ambient term is suppressed while
      `dragging` is true, and one frame after the release the yaw is **where the
      drag left it plus one ambient step** — not reset, not blended.
      `the_ambient_turn_never_moves_the_pitch` pins the one-producer half

- [ ] **`Swipe` and `LongPress` are asserted unconsumed, so the `L4` claim is
      enforced by a test.** `the_widget_consumes_nothing_but_a_drag` iterates **all
      nine** `InputEventKind` variants and asserts only `Drag` and a **focused**
      `KeyDown` are consumed; `Swipe`, `LongPress`, `Tap`, `Pinch`, `KeyUp`,
      `Scroll` and `Text` each leave the event unconsumed. **`InputEventKind::Swipe`
      must therefore appear in exactly one `on_event` body in the whole crate —
      `Rotator`'s `_ => false`, which does not name it — and `grep -rn
      'InputEventKind::Swipe' ui/src/ui_core/src/widgets/` returns the existing
      non-consumption tests in `list.rs` and `keyboard.rs` and nothing new.**
      **The mechanism is stated and true: this criterion is what a later task that
      consumes `Swipe` must break deliberately**, which is why § *What this task
      does to gap `L4`* is checkable rather than documentary

- [ ] **The orbit's signs are pinned, because nothing else can see them.**
      `the_view_puts_the_orbit_target_on_the_camera_axis` asserts
      `view.transform_point([0.0, CAR_TARGET_Y, 0.0])` is `[0.0, 0.0,
      -CAR_DISTANCE]` within the demo's epsilon, for **yaw = 0, pitch = 0** and for
      **a non-zero yaw and a non-zero pitch**. **Mutation evidence:** swap the
      `rotated_x` and `rotated_y` lines in the composition and watch it fail; then
      drop the `-eye` negation and watch it fail. **An orbit with the wrong order or
      the wrong sign produces a plausible wrong picture and no error anywhere** —
      that is why this is the test a reviewer should break first

- [ ] **`rect` is unused by the drag arm, and the module doc says so.**
      `grep -n 'rect' ui/src/ui_core/src/widgets/rotator.rs` shows `rect` in the
      signature and in the doc comment that explains why the drag arm does not read
      it, and **nowhere in `rotate_by` or `clamp_pitch`**

- [ ] **The demo records five mesh commands on the `data` page and none anywhere
      else.** `the_car_records_five_mesh_commands_each_with_a_range_and_one_mvp`
      asserts five `DrawCommand::Mesh` with `opacity: 1.0`, one per range of
      `Model::ranges`, **and that all five carry the same `mvp`**, which is the
      honest statement that no wheel spin exists. **A test asserts no other page
      records one**, so the criterion below is demanding rather than merely met.
      **And the recording order is the contract task 37 recorded:** the car is
      recorded **before** the `data` page's chrome, which is asserted by the
      position of the first mesh command in the paint order

- [ ] **The car is clipped to its rect, and `frame_clips` is where that lives.**
      `grep -n 'frame_clips' ui/src/ui_demo/src/main.rs` shows the one function
      with a new `Some(CAR_RECT)` row for the car's node and `None` for every
      other, **the signature unchanged**, and **its existing tests keep their names
      and their assertions** with the new row **added** to the expected set rather
      than the old set loosened. The handoff says in one sentence what happens
      without it: `apply_clip` is a call on a `None` clip, and an unclipped car
      draws across the tab bar

- [ ] **`cargo test --all-features` is green with every named test present**, and the
      handoff **lists each by name**: in `rotator.rs` —
      `a_horizontal_drag_turns_the_yaw_and_leaves_the_pitch_alone`,
      `a_vertical_drag_tilts_the_pitch_and_leaves_the_yaw_alone`,
      `a_diagonal_drag_moves_both_axes`,
      `the_pitch_cannot_exceed_its_clamp_under_an_extreme_drag`,
      `the_pitch_is_clamped_at_both_ends`,
      `clamp_pitch_returns_zero_for_a_non_finite_pitch`,
      `a_drag_with_no_component_is_not_consumed`, `the_yaw_is_not_wrapped`,
      `the_widget_consumes_nothing_but_a_drag`,
      `the_arrow_keys_nudge_both_axes_and_stop_at_the_same_clamp`,
      `an_unfocused_rotator_declines_a_key_and_lets_it_travel_on`,
      `the_three_constants_are_positive_finite_and_under_a_quarter_turn`,
      `the_drag_arm_needs_no_focus`; in the demo —
      `a_drag_over_the_car_turns_it_and_touches_nothing_else`,
      `the_ambient_turn_holds_the_car_when_nothing_is_dragging`,
      `the_gesture_stops_the_ambient_turn_and_the_ambient_resumes_from_where_it_left_off`,
      `the_ambient_turn_never_moves_the_pitch`,
      `the_car_node_is_a_member_of_the_data_page_and_of_no_other`,
      `the_car_rect_overlaps_nothing_the_gallery_placed`,
      `the_view_puts_the_orbit_target_on_the_camera_axis`,
      `the_car_records_five_mesh_commands_each_with_a_range_and_one_mvp`.
      **No test was deleted, renamed away or weakened.**
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host; that
      is **recorded, not passed`

- [ ] **The six gallery pages are pixel-identical outside two named bands, and the
      mechanism is stated rather than hoped for.** `Page::ALL`'s six names,
      release build, captured **before and after** with the commands of
      `.ai/tools/README.md` § *Capturing a window* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab`, return black for
      a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, then `magick compare -metric AE before.png
      after.png null:` per page.

      - **On the five pages that record no mesh command — `pads`, `text`, `input`,
        `controls`, `overlays` — the criterion is exactly task 34's: AE 0 outside
        `y ≥ 680`**, every differing pixel inside the fps readout's band, which
        `.ai/tools/README.md` § *Capturing a window* records as the one thing two captures of an unchanged frame differ
        in (405 pixels there, **AE 0 over y 80–680**).
      - **On `data` the criterion is AE 0 outside `y ≥ 680` *and* outside
        `CAR_RECT`**, and every differing pixel inside one of those two bands.
        **This is a restatement, not the criterion tasks 34 to 39 inherited, and
        the reason is stated rather than discovered at review: this is the first
        task that records a mesh command, and the ambient rotation is *supposed* to
        move the pixels inside `CAR_RECT` every frame.** The rect is the **named
        constant** `CAR_SIZE` at `CAR_ORIGIN`, so the band is compared against the
        number the code uses and not against an eyeballed region.
      - **The rect-level half keeps its name and every one of its assertions:**
        `every_page_places_every_rect_where_the_gallery_placed_it`, plus
        `no_two_placed_rects_overlap` with the car's rect added to
        `placed_handles` and to `expected_placed_rect_names`, plus
        `assert_placed_handles_is_complete`. **The mechanism that makes all
        three pass is three facts and not one: no rect the gallery placed moves;
        five of six pages record no mesh command; and the clip confines the sixth
        page's change to the one rect the criterion names.**

- [ ] **The frame rate is measured on every page and reported, and the `data` page
      is expected to cost something.** With the script's own line pasted rather
      than the number expected: `.ai/tools/fps-check.sh 10 55` on the default page,
      and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
      the six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `.ai/tools/README.md` § *Frame-rate baseline* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. Every page above the floor of **55**.
      **And the handoff reports `data`'s number separately, whatever it is, with the
      reason stated rather than left as a coincidence:** this is the first page in
      this project that draws a mesh, so it is the first to pay for one —
      **2 032 triangles in five draw calls, five uniform sets and five
      `use_program`s per frame**, on top of a frame that already issues thousands of
      calls. **The five other pages are expected to be inside the recorded 61.1–63.9
      band in `.ai/tools/README.md` § *Frame-rate baseline* and to have
      cost nothing**, because nothing about them changed.

- [ ] **No GL error on the first frame that draws the mesh**, read once with
      `gl.get_error()` and pasted into the handoff **with the instrument's code
      quoted**. **A capture is not a substitute:** a mesh that silently draws nothing looks
      exactly like a mesh that drew a car, and the count of mesh commands in the
      recorded stream is asserted by a test while the pixels are not asserted by
      anything

- [ ] **What the handoff does not claim, in those words.** It states that
      **no pointer event has ever been observed reaching this window** on this
      host — `.ai/tools/README.md` § *Capturing a window* records the drag, the
      two presses on task 12's button, the counter and the `AE = 0`, and
      `XQueryPointer` reporting window `0x0` — and therefore that **no acceptance
      criterion here is verified by a pointer-driven capture, and none asks for
      one.** The gesture logic is verified **by test through the crate's own event
      path**; the car on screen is verified **by capture with the ambient turn
      driving it**; **the two together are not evidence that a finger turns the
      car**, and the entry added to `IMPLEMENTATION_STATE.md` says so

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `grep -c 'pub fn on_event' ui/src/ui_core/src/` over the crate returns **9**
      (was **8**), and the ninth is `Rotator`'s. `git diff --stat` shows **no
      change** to `ui/src/ui_core/src/render.rs`, `render/mesh.rs`,
      `render/matrix.rs`, `render/meshio.rs`, `paint.rs` or `batch.rs` — this task
      adds a widget and does not touch the pipeline. `grep -rn 'inertia\|momentum\|
      flick' ui/src/ui_core/src/widgets/` returns **nothing**, which is
      `L5`'s claim still holding. **`ui/Cargo.toml` and `ui/Cargo.lock` are
      unchanged** — the approved direct dependencies remain `sdl3 0.20`, `glow
      0.18` and `freetype-rs 0.38`, and per `AGENTS.md` a gesture crate or a maths
      crate for two clamped floats is a licence decision against GPLv3 that nobody
      has asked for

- [ ] **`L4` is amended, dated, and its claim is unchanged.** Row `L4` in
      `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* carries a
      dated note naming `TASK_UI_PRIM_40`, stating that **`LongPress` and `Swipe`
      remain unconsumed and the row is not closed**, and correcting its two
      evidence counts to **sixteen** `pub mod` entries and **nine** `on_event`
      impls. **The row is not deleted, not marked closed, and its Blocks column
      still names all four** — dock edit mode, card paging, alert dismissal, the
      drive-mode strip — because none of them is delivered. **And no doc comment in
      any changed file asserts the opposite of the code beside it**, which is the
      defect `DEMO_APPLICATION.md` § *Corrections to the second gap table* records
      twice

- [ ] **The decisions are written down where the next agent finds them.**
      `rotator.rs`'s module doc carries requirement 6's content;
      `rotator.rs`, `ui/src/ui_demo/src/main.rs` and `doc/ui/IMPLEMENTATION_STATE.md`
      each carry their half of requirement 14, including **the model's facing being
      recorded nowhere in this repository** and which yaw the capture settled on,
      **the drag arm not reading `rect`**, the **no-inertia / no-snap /
      no-release-animation** statement, and the **honest limit** in
      `IMPLEMENTATION_STATE.md`'s own register

## Out of Scope

- **Tier 3: skeletal animation is a BACKLOG item and is NOT this task. It was not
  forgotten.** **No joints, no skinning, no bone matrices in the vertex shader, no
  animated wheels, no animated suspension, no wheel spin, no steering.** Stating it
  here in those words is the point: **all five sub-meshes carry the same `mvp`**,
  because task 38 baked every node transform into the file at conversion time and
  a wheel that turns is a second transform on top of a baked one. Tier 3 is the
  work that makes task 35's per-sub-mesh ranges matter beyond placement, and it is
  a sequence of its own with its own format questions — 38's format carries **no
  joints, no weights and no second UV set**, by decision — and it has **no place in
  this task's file at all**
- **No inertia, no fling, no momentum, no decay, no snap, no snap-to-preset and no
  release animation.** Decided, not deferred. The crate has **none of them
  anywhere** — no `inertia`, `momentum` or `flick` identifier exists — and row
  **`L5`** in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*
  records the absence as still open, **so this task does not change `L5`
  either**. A snap needs a preset set the demo has no source for: the manual's
  `[C]` sentence describes a discrete snap that *"also changes the content tier"*,
  and the tier is a `TASK_UI_DEMO_n` concept this task does not have. A release
  animation needs a rest pose, and `CAR_REST_YAW` is an arbitrary flattering angle
  rather than a pose the product has. **The car stops where the finger stopped, and
  that is stated in `rotator.rs`'s module doc rather than left to be discovered by
  a user**
- **No `LongPress` and no `Swipe` consumption.** Decided and named in § *What this
  task does to gap `L4`*: a `Swipe` carries a direction and **no magnitude**, so
  mapping it onto a rotation means inventing one; and the three consumers row `L4`
  names — dock edit mode, alert dismissal, card paging — are **discrete
  threshold-based decisions** where a swipe's semantics is exactly what is wanted.
  **Row `L4` stays open**, and this task's contribution to it is one more widget
  module and a corrected evidence count
- **No pinch, and no zoom.** `Pinch` is declined **by name** in
  `Rotator::on_event`'s doc comment: the manual's own words make pinch a **zoom**,
  and a zoom is a projection change — `Mat4`'s `fov_y`, or `near`/`far`, both of
  which **36 recorded and 40 must not move**. A zoom task is a different task with
  a different property on the same widget, and this task does not start it
- **No multi-touch of any other kind**, no rotation inertia from a fling, no
  two-finger orbit, no `Gesture` type
- **No rotation of the car itself — only an orbit of the camera.** The model
  matrix is the identity and stays it, which is why `u_normal_matrix` is nine ones
  (requirement 11). **A future task that rotates the car needs a real inverse
  transpose, which `Mat4` does not have**, and that is recorded in the demo's
  comment rather than left for the next agent to find out at the shader
- **No map, and no backdrop.** `DEMO_APPLICATION.md` § *What this means for the
  demo's shape* item 1 records that the demo must have a map to put things on, and
  gap **`L1`** in § *Gaps this layout exposes in `ui_core`* is the pipeline work for
  drawing over it. **Neither is this task's.** What this task says about them is
  the negative one that matters for the clip: **nothing is drawn below
  `Y = 0`**, so a camera under the ground plane sees the car's closed underbody
  against the cleared background, and that is a consequence of `L1` being open and
  not a design
- **No chrome, and no new UI of any kind.** No speed readout, no angle readout, no
  compass, no reset button, no label naming the yaw, no focus ring on the car. The
  rotator draws nothing and the demo adds nothing around it
- **No new page, and no change to any of the six.** `DEMO_APPLICATION.md`
  § *What a seventh page costs* enumerates what one costs — `Page::ALL` becomes
  `[Page; 7]`, `Page::name`, `Demo::tabs`, the three gates, and the
  **mutation-tested** page-membership table — and **this task puts the car on an
  existing page precisely so that none of that is touched.** `Page::DEFAULT` stays
  `Pads`, the `--help` text is unchanged, and no new `--tab=` name exists
- **No wheel spin, no animated ride height, no glass transparency, no per-component
  recolour.** Track Mode's recolouring rules are in `DEMO_APPLICATION.md`
  § *Could not verify* as *"not a spec the demo should copy until verified"*, and
  task 37's *Out of Scope* already records that per-sub-mesh tint reaches `body`
  against the four wheels **and nothing finer**. This task draws the colormap it
  was given
- **No change to any pipeline mechanism.** `DEPTH_BITS`, the depth policy, the
  resting state, `GL_LESS`, the culling policy, `MeshVertex` and its stride, the
  `MESH_VERTEX_STRIDE` / `MESH_NORMAL_OFFSET` / `MESH_UV_OFFSET` constants,
  `Mat4` and both projections, `MESH_MVP_UNIFORM`, `DrawCommand`'s ten variants,
  `ShaderKind`'s five, `Segment`'s two boundary slots, `depth_state_for`,
  `DrawCommand::Mesh`, `BatchKey::is_singleton`, the mesh shader sources, the
  loader, the byte format and every constant in it — **all untouched by this task,
  and checkable by `git diff --stat`.** Task 36's promise that **task 40 does not
  move `near` or `far`** is paid in requirement 7
- **No change to `LayoutMode`, and no reliance on anything unimplemented.**
  `LayoutMode::Grid` lays out nothing and `Flex.wrap` is discarded, so **the car's
  node is placed absolutely like every other node in the demo** and this task adds
  no child, no flex container and no grid. `Padding` is a four-sided inset and is
  not used either; there is **no margin, no `flex-shrink`, no `flex-basis` and no
  cross-axis gap** in the crate, and this task needed none of them
- **No new dependency.** Per `AGENTS.md` the approved direct dependencies are
  `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; a gesture library, a maths
  crate or an easing crate for two clamped floats is a licence decision against
  GPLv3 that nobody has asked for. **And no `unsafe` beyond what a GL call
  requires** — this task adds **zero** new `unsafe` blocks, because the demo's new
  arithmetic is `[f32; 3]` trigonometry and `Rotator` contains no GL at all.
  `grep -c unsafe ui/src/ui_core/src/widgets/rotator.rs` is **0**