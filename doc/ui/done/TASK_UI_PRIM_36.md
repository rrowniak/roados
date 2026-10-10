# TASK_UI_PRIM_36: A 4×4 Matrix Type, and the Uniform the Mesh Pass Will Read

## Goal

Give `ui_core` the arithmetic a 3D pass needs and nothing else: a column-major
`Mat4`, a perspective and an orthographic projection, and one conversion from the
crate's existing `Transform` into it. Reserve the single uniform name the mesh
pass will read. **No shader is edited, no draw command is added, and no matrix
reaches a GL context in this task** — the program that would declare the uniform
does not exist until task 37.

## Context

This is the third of seven tasks (34–40) that add real-time 3D mesh rendering to
`ui_core`, in service of the Tesla-like demo in `DEMO_APPLICATION.md`:

| # | Subject | Depends on 34 for | Depends on 35 for |
|---|---|---|---|
| 34 | the depth buffer, and the depth policy | — | — |
| 35 | `MeshVertex` and the mesh GPU buffers | a place to put the projected `z`; parts drawn in an order the CPU does not control | — |
| 36 | matrix math and the transform-to-GPU path (**gap `L2`'s matrix half**) | the `z` this task's projection produces has somewhere to land | the `(first_index, index_count)` ranges that each carry their own matrix |
| 37 | the mesh draw command, its shader and batching | the depth test that makes submission order irrelevant | the buffers this task's uniform is uploaded alongside |
| 38 | the model file format and the loader | — | the `Mesh` it has to produce |
| 39 | the asset pipeline | — | — |
| 40 | drag-to-rotate (**closes gap `L4`**) | a rotated mesh to be depth-tested | four independently transformable wheel ranges |

**Both dependencies are declared, not incidental.** Task 35 must have landed,
because requirement 1 declares this module beside `pub mod mesh;` and
requirement 5 makes "one vertex buffer, five matrices" the point of 35's
sub-mesh design; if `render/mesh.rs` is not there when this task starts, that is
a dependency violation and not a variant of this task. Task 34 must have landed,
because `TASK_UI_PRIM_34.md` § *Requirement 1* writes the near/far ratio task 36
will choose into the `DEPTH_BITS` doc comment, and requirement 6 pays that
promise with two named numbers.

### What gap `L2` says, and what of it this task closes

Gap **`L2`** in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*
makes three claims: `Transform` is `Interpolate`-able so it can be animated;
`DrawCommand` has no transform field, none of its nine variants carrying one; and
there is no matrix and no `u_model` uniform. It is Critical, and it blocks
"Panel slide-ins, card paging, the drive-mode strip's drag, any resize animation".

**This task closes the matrix and the bridge, and it does not close the other
two halves.** Stated here rather than in the handoff because the alternative is
a row in a gap table marked closed on the strength of a type nobody calls:

- **Closed here:** *there is no matrix*. `Mat4` exists, is tested against
  hand-computed values, and `Transform` has a conversion into it — so the type
  an animatable `Transform` can become is now the type the GPU path consumes.
- **Closed by task 37:** *no `DrawCommand` variant carries a transform*, for the
  mesh path, and *no `u_model` uniform*. The uniform cannot be uploaded by this
  task because the mesh **program** does not exist; a uniform added to a program
  whose shader ignores it is stripped by the GLSL compiler and
  `get_uniform_location` returns `None`, which makes every upload a silent
  no-op (see § *Why no existing shader changes*).
- **Not closed by anything in 34–40:** the 2D half — a transform field on one of
  the nine 2D `DrawCommand` variants, applied by one of the four 2D vertex
  shaders. That cannot be done without editing all four, and § *Why no existing
  shader changes* is the arithmetic for why it is not done here.

So requirement 12 **amends row `L2`, dated and attributed**, rather than deleting
it or leaving it to rot. That is the discipline `DEMO_APPLICATION.md`
§ *Corrections to the second gap table* exists to record: row `L1` was rewritten
2026-10-05 after the previous row was found **false at the time it was written**,
and the section's own sentence — *"a doc comment asserting the opposite of the
code beside it"* is the failure it names twice.

### The source, read and not inferred

- **`Transform` is a minimal 2D transform** (in `ui/src/ui_core/src/property.rs`):
  `pub struct Transform { tx, ty, sx, sy, rotation }`, five `f32`, all public,
  `#[derive(Clone, Debug, PartialEq)]`, a module doc calling it *"A minimal 2D
  transform"*, `rotation: f32` doc-commented *"Rotation in radians"*, and
  `identity()` with every field at its default.
- **It is animatable and undrawable.** `impl Interpolate for Transform`
  (in `ui/src/ui_core/src/animation.rs`) interpolates all five fields one
  by one, and `rg -n 'Transform' ui/src -l` returns **exactly two files**:
  `property.rs` and `animation.rs`. **No widget, no paint call site, no shader,
  no demo page.** `rg -n 'rotation' ui/src/ui_core/src/` outside those two files
  returns one hit, and it is a doc comment in `chart.rs` about chart segments.
  **No rotation is drawn anywhere in the crate.**
- **`DrawCommand` has nine variants and none carries a transform** — `Rect`,
  `RoundedRect`, `Shadow`, `Text`, `Image`, `Line`, `Circle`, `Path` and
  `Polygon`, in that order, in `ui/src/ui_core/src/paint.rs`.
- **There is no matrix anywhere.** `rg -n 'u_model|mat4|mat3|projection' ui/src/`
  returns **nothing** — not zero hits in `render.rs`, zero in the whole `ui/src`
  tree.
- **The complete set of uniform names in the renderer** is ten: `u_atlas`,
  `u_color`, `u_direction`, `u_image`, `u_resolution`, `u_size`, `u_source`,
  `u_taps`, `u_texel`, `u_weights`, spread over **16 `get_uniform_location`
  sites** in `Renderer::new` in `render.rs` because a location belongs to its own
  program and the shadow and blur passes each need their own `u_resolution`. Any
  name this task reserves must not collide with those ten.
- **All four 2D vertex shaders write the same `gl_Position`.**
  `VERTEX_SHADER_SRC` (solid), `TEXT_VERTEX_SHADER_SRC` (text),
  `IMAGE_VERTEX_SHADER_SRC` (image) and `BLUR_VERTEX_SHADER_SRC` (blur), each in
  `render.rs`, each end
  `gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);`, built from
  `vec2 normalized = a_pos / u_resolution; vec2 clip = normalized * 2.0 - 1.0;`.
  **`z` is hard-coded `0.0` in all four**, and a test already pins the blur
  one's spelling — `the_blur_vertex_shader_flips_v_so_the_target_is_read_the_right_way_up`.
- **The only transform-like effect in the crate is on the CPU.** `Button`'s
  press "scale" is `fn scaled(rect: Rect, scale: f32)`
  (in `ui/src/ui_core/src/widgets/button.rs`), which recomputes a smaller centred
  rect. Uniform-only, axis-aligned, no rotation, and its doc-adjacent arithmetic
  is about `rect.width` and `rect.height` rather than a matrix.
- **`Rect.y` is "Y offset from the top edge of the window"** (`ui/src/ui_core/src/paint.rs`),
  so window space is **y-down**, and the four shaders' `-clip.y` is what turns it
  into NDC's y-up.
- **`ShaderKind` has four variants** — `Solid`, `Text`, `Image`, `Shadow`
  (in `ui/src/ui_core/src/batch.rs`) — and `glow 0.18` carries
  `HasContext::uniform_matrix_4_f32_slice(&self, location: Option<&Self::UniformLocation>, transpose: bool, v: &[f32])`
  (`src/lib.rs:1453-1458`), which takes a plain `&[f32]` and a `transpose` flag.

### The decision: `Transform` keeps its five fields, and the mesh path gets a second type

Weighed four ways, and the fourth is what settles it.

1. **They are different rotations in different handedness.** `Transform.rotation`
   is *"Rotation in radians"* and its axis is documented **nowhere**. A 2D
   rotation has exactly one possible axis — z — because a rotation about x or y
   in the plane is a reflection or a mirror, which is not what a field named
   "rotation" means, so the axis is derivable even though it is unwritten. The
   car's placement needs **yaw about y** and its wheels need **spin about x**.
   Worse than the axis is the sense: window space is y-down (`Rect.y` from the
   top edge), so a positive z-rotation built from `Transform` and consumed
   *without* a flip turns x toward +y, which is **clockwise on screen**, and
   consumed *with* the four shaders' `-clip.y` it is **counter-clockwise**. The
   same struct would carry two opposite directions depending on the consumer, and
   nothing in the crate decides which a 2D transform should be because **no 2D
   transform is drawn**.
2. **Field-by-field interpolation is right for one axis and wrong for three.**
   `impl Interpolate for Transform` in `animation.rs` interpolates the five fields
   individually, and its doc
   says *"Rotation is interpolated as the number of radians it is, not along the
   short way round: the two angles are values, and taking the difference is the
   caller's decision"*. That reasoning is sound for a scalar and does not extend
   to three Euler angles, where interpolating the components does not follow the
   rotation between two orientations. Growing `Transform` would make this task
   edit an animation-semantics decision, which is not maths.
3. **`Transform` is a `Property` value and a `Mat4` is a per-draw value.**
   `Transform` is `Clone`, `Debug`, `PartialEq`, interpolated per frame per
   animation. A `Mat4` is `Copy`, composed per draw, and never compared. Adding
   `tz`, `sz`, `yaw` to `Transform` grows the per-frame cost of every animation
   for fields no animation sets.
4. **Growing it is a public API change, and this task's whole deliverable is
   maths.** `Transform` is a `pub struct` with five public fields, so adding a
   field breaks every struct literal — including the ones in this crate's own
   tests — and `developer.md` § *API design* requires explicit discussion of a
   breaking public change. A task whose scope is "matrix maths and uniform
   plumbing" is not where that discussion happens.

**Decision: `Transform` is unchanged, and the mesh path carries a `Mat4`.** The
bridge is one conversion, `Transform::to_matrix(&self) -> Mat4`, **declared in
`render/matrix.rs` and not in `property.rs`** so that the lowest layer of the
crate does not gain a dependency on `render`. `Transform`'s five fields, its
`identity()`, its `Default`, and `Interpolate` are untouched — which is
checkable by `git diff`.

**The honest limit, and the rule that names it.** `to_matrix` is the first edge
out of `Transform` in the crate's history, and **no widget's paint path calls
it**. **A test of a helper cannot see a call site that stopped using it** is
the exact rule: a test of `to_matrix` proves `to_matrix`
and nothing else, and a caller that never calls it is invisible to the suite.
So the acceptance criterion greps for the edge and **states what it does not
prove**. What would close it for the 2D layers is named in § *Out of Scope*.

### `rotation`'s axis, and the one thing about it a capture cannot settle

Two facts belong in the file rather than in a reviewer's head, because both look
like the other:

- `Transform`'s plane rotation is about **z**, and the crate's window space is
  **y-down**, so a matrix built from a positive `Transform.rotation` rotates in
  the opposite screen direction from the same matrix consumed with a y-flip.
- **Neither direction is "what the crate does today"**, because the crate draws
  no `Transform`. The choice is 37's and 40's to make, and the module docs must
  say that rather than implying a convention exists. What makes it checkable is
  a labelled rect turned a quarter turn and captured, which is a demo asset this
  task does not add.

### Perspective **and** orthographic, and why the orthographic one is the shipped 2D mapping

Both are exposed, and the reason the orthographic one is not optional is
arithmetic rather than taste: **the mapping the four 2D shaders already perform
is exactly an orthographic projection**, and the task can *prove* it with a test
instead of asserting it in prose.

With `w = width`, `h = height`, the shaders compute
`normalized = a_pos / u_resolution` then `clip = normalized * 2.0 - 1.0`. The
orthographic matrix with `left = 0`, `right = w`, `bottom = 0`, `top = h`,
`near = -1.0`, `far = 1.0` maps a point to

```
x' = 2x/(right - left) - (right + left)/(right - left) = 2x/w - 1
y' = 2y/(top - bottom) - (top + bottom)/(top - bottom) = 2y/h - 1
z' = -2z/(far - near) - (far + near)/(far - near) = -z - 0        (z = 0 → 0.0)
w' = 1
```

which is `clip.x`, `clip.y`, **and `z' = 0.0` — the very value all four shaders
hard-code** — with the single sign of `y` left to the shader's `-clip.y`. So
`Mat4::orthographic(0.0, w, 0.0, h, -1.0, 1.0)` **reproduces the shipped
pipeline's projection exactly, and the test that says so is the strongest single
argument this task has for closing `L2` without editing a shader.** The flip
stays in the shader for the reason it is there: all four shaders agree on it, and
`TASK_UI_PRIM_34.md`'s depth arithmetic is stated in terms of that shared `z`.

**The near plane is negative and that is load-bearing.** Passing the positive,
obvious-looking `near = 0.1, far = 100.0` to the same call maps `z = 0` to
`z' = -(far + near)/(far - near) = -100.1/99.9 ≈ -1.002`, i.e. to the **near
plane** — every 2D fragment at window depth ≈ 0.002 instead of 0.5, which is
precisely the value task 34's depth arithmetic depends on. Requirement 6 requires
the sign convention stated in the doc comment and pinned by the test above.

**Perspective is exposed for the car, and its `z` convention is OpenGL's, not
D3D's or Vulkan's.** GLES 3.1 clip space is `z ∈ [-1, 1]`, which is what makes
34's *"NDC `z` remapped from `[-1, 1]` to `[0, 1]`, so the 2D passes' `0.0`
becomes **0.5**"* correct, so the projection must produce `[-1, 1]`. The
`[0, 1]` D3D/Vulkan form is a real mistake available here and requirement 6 names
it.

**The near/far ratio, and it pays task 34's promise.** `TASK_UI_PRIM_34.md`
§ *Requirement 1* wrote that 24-bit depth "leaves far more precision than the
raster can show **with a near/far ratio task 36 will choose in the range of
hundreds**". This task chooses **`near = 0.1` m and `far = 20.0` m, a ratio of
200** — squarely in the promised range, and appropriate to the scene: a
4.8 m car under a camera that never comes closer than about 2 m, on a floor with
nothing else in it. 20 m is not arbitrary — beyond it the car is off the panel
anyway. The **field of view is `fov_y = π/4` (45°), the mild end**: its
vertical half-extent at distance *d* is `d · tan(π/8) ≈ 0.414 d`, so at 6 m the
frustum is about 5 m tall against a body roughly 1.4 m tall — a visible
convergence without the wide-angle distortion. **Task 40 rotates the camera and
must not move `near` or `far`**, and requirement 6 records that.

**Rejected and named, so 37 does not re-decide one of them by accident:** a single
projection serving both (which is why both are exposed, and neither is a special
case); an oblique or axonometric projection, which for a car the demo anchors
callout hotspots to body parts reads as a diagram where the photographs read as a
photograph, and which is three lines of matrix that would need their own tests;
and every depth-precision device 34 already refused — reverse-Z, logarithmic
depth, `glClipControl`, separate near/far objects — none of which this task
reopens.

### One matrix or three: `u_mvp`, and the count is not what decides it

**Decision: a single pre-multiplied `u_mvp`**, with `u_view` and `u_projection`
**not** reserved as names.

**The count, measured honestly, does not decide it.** The car is five sub-meshes,
so three uniforms would be 15 `glUniformMatrix4fv` calls per frame against 5 —
915 against 305 per second at 61 fps, and the same for the two extra matrix
products on the CPU, 10 per frame against 5. Both are noise against a frame that
already issues thousands of calls, and claiming otherwise would be a false
economy argument that a reviewer can and should refute.

**What decides it is the error surface, and this is the whole argument.** With
three uniforms the shader must multiply them in the right order, and the two
plausible wrong orders — `M · V · P` and `V · P · M` — both compile, both run,
and produce a plausible-looking wrong image. **`cargo test` cannot see it,
because the shader is task 37's and no test in this crate compiles GLSL.** One
pre-multiplied `u_mvp` has exactly one order, written in one Rust expression:

```rust
let mvp = projection.multiply(&view).multiply(&model);
```

and that expression is unit-testable with no display, against
`transform_vec4` composition, by requirement 11's `multiply_is_the_matrix_product_and_its_order_is_the_glu_one`.

**What one matrix costs, and it is a real cost.** The mesh **fragment** shader
will need the model→world normal transform separately, and `mat3(u_mvp)` is not
one: the projection scales x by `1/aspect` and z by the near/far terms, so
`P · R · n` normalised is not `R · n`. This task **does not add that uniform**,
because `developer.md` § *Phase 2* says *"No abstraction before the second use.
One caller is a function, not a trait"* and there is not one caller — there is
zero. The requirement on **37** is recorded in the module docs with that
arithmetic, so 37 inherits the fact instead of rediscovering it. **The one
matrix a uniform scale would make sufficient is `u_model` itself**, which is the
honest reason a future task may reverse this decision and ship two uniforms
beside `u_mvp`: with a non-uniform scale there is no cheaper answer, and a
non-uniform scale is a caller's choice — a model fitted to the panel is scaled
per axis — not something task 35 or 38 imposes.

**The transpose flag is the trap, and it is named.** `Mat4`'s array is already
column-major and GLSL's `mat4` is stored column-major, so the upload is

```rust
// SAFETY: The GL context is current on this thread and `u_mvp` was queried
// from `mesh_program`.
gl.uniform_matrix_4_f32_slice(u_mvp, false, mvp.as_slice());
```

with **`transpose: false`**. Setting it `true` — which reads as a harmless
"correcting" argument, and which a GLSL programmer reaches for out of habit from
row-major maths libraries — **silently transposes the matrix**: no GL error, every
call reporting success, and a scene that is mirrored and/or rotated wrongly. That
is the same failure shape as `TASK_UI_PRIM_34.md` § *The shadow FBO does not need
a depth attachment*, which records *"nothing errors, the pipeline reports no GL
failure at any step, and the picture is a window with no shadow on it"*.

**Two uniforms are deferred on a measurement, not by default.** Recomposing
`P · V` per sub-mesh costs 128 multiply-adds per draw, 640 per frame — nothing at
this scale. If a future frame rate ever asks for it, the change is to cache
`P · V` on `Renderer`, and requirement 7 records that in the file so the next agent
knows the decision was taken rather than forgotten.

### Why no existing shader changes, and what pixel-identity is verified against

All four 2D vertex shaders write `gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);`
from `a_pos / u_resolution`. Editing any of them changes every pixel of every
page, and an identity matrix that is *not* exactly identity — one bit wrong in one
of sixteen floats — is a page that is subtly wrong with a fully green suite,
because no test in this crate compiles GLSL. **So: none of the four vertex
shaders and none of the four fragment shaders is edited.** That is a requirement
(7), it is checkable by `git diff` on `render.rs`, and requirement 11 adds a
test that asserts all four sources still contain the identical `gl_Position`
spelling and none contains `u_mvp`, so the guarantee is in code rather than in a
reviewer's memory.

**Two tempting alternatives, both rejected, and both are the same failure twice:**

- **Declare `u_mvp` in the *solid* program and multiply `gl_Position` by it,
  identity every frame.** Identity makes it pixel-identical, which is what makes
  it tempting. It is rejected because (a) setting a uniform and *then* asking for
  the draw writes it to whichever program the previous pass left bound — the
  failure `Renderer::submit_vertex_quads`'s own doc in `render.rs` records in as
  many words (*"a caller that sets `u_resolution` and then asks for the draw
  writes the uniform to whichever program the previous pass left bound — which is
  what the two shadow passes did first"*), where the two shadow passes lost their
  shadows to it; (b) it adds a per-frame GL call for no visible effect; and (c) it
  puts a matrix into a shader whose `z` is fixed, which would silently give the 2D
  passes a `z` that is no longer the `0.0` task 34's depth arithmetic is written
  against.
- **Declare `u_mvp` in the solid program and *not* use it.** **GLSL strips an
  unused uniform**, so `get_uniform_location` returns `None` and every upload is
  a silent no-op. The result is a uniform that appears to be wired, passes a
  grep, and does nothing — the most expensive kind of "green".

**Pixel-identity is verified, not asserted.** Release build,
`setsid ./target/release/ui_demo > log 2>&1 &`, window id **re-read at the time
of each capture** with `xwininfo -root -tree` (a root capture and
`ffmpeg x11grab` both return black for a GL window), `pgrep -a -x ui_demo` in the
same call as each `magick import -window <id>`, then
`magick compare -metric AE before.png after.png null:` per page, over all six of
`Page::ALL`'s pages (`pads`, `text`, `input`, `controls`, `data`, `overlays`) —
the method of `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the
capture method*, used verbatim.

**The band is part of the method.** Two captures of an unchanged frame differ
inside the fps readout — `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it
decided, and what it found* records **405 pixels, and AE 0 over y 80–680** — so
the criterion is **AE 0 outside `y ≥ 680`, with every differing pixel inside that
band**.

**`.ai/tools/fps-check.sh` cannot name a page.** Verified in
`.ai/tools/fps-check.sh`: it reads exactly two positional arguments — `seconds`
then `floor` — and then runs the binary with **no
arguments**. Per-page measurement is therefore
`ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` with the
`roados-fps` line parsed by hand — the same amendment
`IMPLEMENTATION_STATE.md` § *Current position* records for task 24.2's criterion
6, and **an acceptance criterion that names an instrument which cannot produce
the evidence is not met by producing the evidence another way** is why it is
written that way here rather than as an instruction to use the
script for six pages.

### Per-mesh transforms compose, and where the composition happens

Task 35 made the car's five named meshes — `body`, `wheel-front-left`,
`wheel-front-right`, `wheel-back-left`, `wheel-back-right` — into
`(first_index, index_count)` ranges into **one** vertex buffer and **one** index
buffer. That is what makes each independently transformable: **one upload, five
draws, five matrices.** The composition is

```rust
let mvp = projection.multiply(&view).multiply(&car_placement).multiply(&wheel_placement);
```

so a wheel spins about its own axle, rides with the body, and the body is placed
by one matrix — four wheels turning independently, which is the case 35's
requirement 3 says the range design exists to enable. It is pure arithmetic with
no GL in it, so it is a unit test, not a capture.

**Where the per-draw matrix lives is task 37's decision and this task does not
pre-empt it.** 36 supplies `multiply` and the order, and the module docs record
the two candidates: a `transform: Mat4` field on `DrawCommand::Mesh` (expected,
because a wheel's matrix changes every frame and a `MeshStore` slot that is
mutated mid-frame would make `end_frame`'s walk of the batcher read state that
moved under it — the same hazard `DEMO_APPLICATION.md`'s row **`L6a`** records for
a mode that must not change in the middle of a frame) versus a `Vec<Mat4>` in
`MeshStore` (rejected for that reason, and because a batch key built from it
would have to compare sixteen floats to decide whether two draws can merge).
35's requirement 3 put the batching-key question out of scope and in 37's hands;
so it stays there, and 36 records the expectation rather than freezing it.

### Neither `Mat4` nor the projections need `unsafe`, and this task adds none at all

`[f32; 16]` arithmetic, `f32::sin_cos`, `f32::tan` and comparisons are all safe
Rust. **This task adds zero `unsafe` blocks** — stronger than 34's and 35's,
because there is no GL call in it either: no `uniform_matrix_4_f32_slice` (no
mesh program), no GL object, no buffer, no FBO. The only `unsafe` in the mesh
matrix path is task 37's uniform upload, and it inherits its SAFETY comment's
shape from `Renderer::draw_shadow_quads` and 34's and 35's blocks — *"The GL
context is current on this thread"*, plus which program the location came from.

**And no dependency.** Per `AGENTS.md` the approved direct dependencies are
`sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; `glam`, `cgmath` and `nalgebra`
are a licence decision against GPLv3 that nobody has asked for. What one would buy
here is SIMD, and the approved dependency set contains no SIMD, so the crate
would buy the same sixteen floats with a supply-chain tail. Requirement 10 makes
`ui/Cargo.toml` and `ui/Cargo.lock` unchanged, which is checkable in one command.

### Degenerate inputs: `Option`, not `Result`, and not a panic

`developer.md` § *Panics and unwrap* forbids `unwrap`, `expect`, `panic!`,
`unimplemented!` and `todo!` in production code, and prefers `Result` and
`Option` over a panicking path.

A `perspective(fov_y = 0.0, …)` divides by `tan(0.0) = 0.0`. **In Rust that is
`inf`, not a panic** — which is worse, not better: the matrix uploads, every
triangle in the car vanishes, and there is no GL error. It is the same failure
shape `TASK_UI_PRIM_35.md` § *Requirement 4* records for a zero-length vertex
normal, where `NaN` in a vertex buffer "makes the triangle it belongs to
disappear with no GL error".

So **both constructors are total by construction and return `Option<Mat4>`**,
`None` on every violated precondition, `#[must_use]`:

- `perspective`: `None` unless `0.0 < fov_y_radians < PI`, `aspect > 0.0`,
  `0.0 < near < far`, and every argument finite.
- `orthographic`: `None` unless `right != left`, `top != bottom`, `far != near`,
  and every argument finite.

**`Option` and not `Result`** because there is no error to describe and no I/O: a
violated precondition is a value the caller handles or does not want, and
wrapping it in an error type would imply a recoverable condition.

**No `assert!` and no `debug_assert!`** — and the reason is specific rather than
dogmatic: a `debug_assert!` fires in the test build and not in the release build,
so the preconditions this task's tests exercise would not be the ones the demo's
release binary runs under, and a capture taken in release would be verifying a
different function than the suite tested. `git diff` on the new file is the
criterion.

**`transform_point` returns `None` only when `w` is zero or non-finite.** A point
at the camera plane has `w = 0`, and dividing by it is a division by zero. **A
point *behind* the camera has `w < 0` and returns `Some`** — that is not an
error, it is clipping: GL discards a primitive whose clip coordinates fail
`-w ≤ x, y, z ≤ w`, and a caller that returned `None` there would drop a vertex
GL would have handled. Getting this "helpfully" wrong is named in requirement 4
because it is the kind of defensive-looking change that is a defect.

### Float comparison, and why the tests must not assert one `Mat4` against another

`f32::cos(PI/2)` is `6.12e-17`, not `0.0`. "Unit-tested against hand-computed
values" plus a derived `PartialEq` is a suite that fails on the last bit of a
trigonometric function and gets deleted. So requirement 11 requires **one private
epsilon helper** (`close(got, want)` at `1e-5`, plus `assert_matrix_eq`) used by
every matrix test, and — the part that matters more — **assertions against
hand-written `[f32; 16]` literals, never against another `Mat4` method's output.**
Asserting `a.multiply(&b)` against `a.translated(..).rotated_z(..)` is a
tautology: it compares two implementations of the same arithmetic, so it would
have caught nothing.

### Scope, measured against `developer.md` § *Scope check*

**Four files, three components, each at or under the threshold and the third
exactly on it.** Files: `render/matrix.rs` (new), `render.rs` (module
declaration, the reserved constant, the module-doc section, tests),
`DEMO_APPLICATION.md` (row `L2`), `IMPLEMENTATION_STATE.md` (the record).
Components: the matrix and its two projections; the `Transform` bridge; the
reserved uniform name and the docs — the third is four lines and a doc comment.
**No fan-out is needed**, and if the implementer finds themselves touching a third
*code* file, that is a stop condition rather than an expansion.

## Requirements

1. **A new module `ui/src/ui_core/src/render/matrix.rs`**, with `pub mod matrix;`
   declared in `render.rs`'s module block beside `pub mod blur;`,
   `pub mod context;`, `pub mod mesh;` (task 35's) and `pub mod target;`. A
   missing `render/mesh.rs` when this task starts is a dependency violation, not a
   variant of it.

2. **`Mat4`, one private field, column-major.** `#[derive(Clone, Copy, Debug,
   PartialEq)] pub struct Mat4 { cols: [f32; 16] }` — the field is **private**,
   and `cols: [[f32; 4]; 4]` is **rejected** because `&[[f32; 4]; 4]` does not
   coerce to `&[f32]`, so reaching the upload's required `&[f32]` would need a
   `from_raw_parts` and this task adds no `unsafe`. **Column-major**, with
   `cols[c * 4 + r]` at column `c`, row `r`, and the doc comment says why in the
   crate's voice: GLSL's `mat4` is stored column-major and `glUniformMatrix4fv`
   with `transpose = GL_FALSE` takes its 16 floats in that order, so the array is
   uploaded **verbatim** and no transpose flag can silently swap it. Accessors:
   `pub fn as_slice(&self) -> &[f32; 16]` — the one way out, and it coerces to
   `&[f32]` at the call site with no copy and no cast. **No second accessor**:
   `reviewer.md` § *Overly public API* applies to a matrix more than to anything.

3. **The operations, all `#[must_use]`, all post-multiplying** (each is `self ×
   rhs`, so `identity().translated(t).rotated_z(r).scaled(s)` is `T · R · S` and a
   point is scaled, then rotated, then translated — the order a caller means):

   ```rust
   pub fn identity() -> Mat4;
   pub fn multiply(&self, rhs: &Mat4) -> Mat4;
   pub fn translated(&self, offset: [f32; 3]) -> Mat4;
   pub fn scaled(&self, factor: [f32; 3]) -> Mat4;
   pub fn rotated_x(&self, radians: f32) -> Mat4;
   pub fn rotated_y(&self, radians: f32) -> Mat4;
   pub fn rotated_z(&self, radians: f32) -> Mat4;
   pub fn transform_vec4(&self, v: [f32; 4]) -> [f32; 4];
   pub fn transform_point(&self, point: [f32; 3]) -> Option<[f32; 3]>;
   pub fn perspective(fov_y_radians: f32, aspect: f32, near: f32, far: f32) -> Option<Mat4>;
   pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Option<Mat4>;
   ```

   `identity()` is a **function, not an associated constant**, because
   `Transform::identity()` in `property.rs` is one and the nearest precedent in
   the crate wins. `rotated_x` and `rotated_y` are required **in addition to**
   `rotated_z`: `rotated_z` is what makes `Transform` expressible at all, and a
   `Mat4` with only a z-rotation cannot place the car (yaw is about y) or spin a
   wheel (about x), which would leave task 40 hand-multiplying an axis-angle
   matrix with no test. All three are the same one-line standard rotation, and
   three methods for three axes is not a seam.

4. **`transform_point` divides, `transform_vec4` does not, and the difference is
   the reason both exist.** `transform_vec4` is total — no division, no `Option`.
   `transform_point` is `transform_vec4` with `w` divided out, returning **`None`
   only when `w` is zero or non-finite**, and returning **`Some` for `w < 0`**:
   a point behind the camera is clipping, not an error, and GL discards it from the
   clip coordinates rather than from a `None` a caller would act on. The doc
   comment names that case explicitly, because returning `None` there is a
   defensive-looking change that is a defect.

5. **The `Transform` bridge, in this module and not in `property.rs`** —
   `impl Transform { pub fn to_matrix(&self) -> Mat4 }`, building
   `T · R_z · S` in the plane `Transform` already describes. The doc comment
   records, in this order: that `Transform` is **unchanged** by this task; that
   its `rotation` axis is **z** and is documented nowhere in the crate, which is
   why this conversion is the z-rotation; that the crate's window space is
   **y-down** (`Rect.y` is an offset from the top edge), so a positive rotation
   turns **clockwise on screen without a flip and counter-clockwise with the four
   shaders' `-clip.y`**; that **neither direction is a behaviour the crate has
   today**, because no 2D transform is drawn anywhere and the choice is task 37's
   and 40's; and that **the y-flip is deliberately not baked into the matrix**,
   because all four existing shaders apply it themselves and a matrix that carried
   it would be wrong for every one of them. `property.rs` is **not edited**, so
   `property` does not gain a dependency on `render`.

6. **Both projections, with the conventions stated and the near plane's sign
   load-bearing.** The doc comments state, each in as many words:

   - `orthographic` takes **signed** `near` and `far` as view-space distances
     along **-z**, and **the 2D box is `near = -1.0, far = 1.0`**. Passing
     `0.1, 100.0` maps `z = 0` to `z' ≈ -1.000` — the near plane — instead of
     `0.0`, which is the value task 34's `0.5 < 0.5` arithmetic is written
     against. The convention is stated in the doc comment, and
     `orthographic_with_positive_distances_puts_z_zero_on_the_near_plane` asserts
     the trap rather than leaving it as a warning nobody reads
   - `perspective` produces **OpenGL** clip space, `z ∈ [-1, 1]`, because GLES
     3.1's clip volume is `[-1, 1]` and it is that which makes window depth
     `0.5` for `z_ndc = 0.0`. The `[0, 1]` D3D/Vulkan form is named as the
     mistake it is.
   - The module docs record the decided defaults — **`fov_y = π/4` (45°),
     `aspect` from the viewport, `near = 0.1` m, `far = 20.0` m, ratio 200** —
     with the `0.414 d` half-extent arithmetic that makes 45° the mild end, and
     with **task 34's promise quoted**: 24-bit depth "leaves far more precision
     than the raster can show with a near/far ratio task 36 will choose in the
     range of hundreds". **Task 40 rotates the camera and must not move `near` or
     `far`**, and the file says so.
   - Both preconditions, and both `Option` returns, as in § *Degenerate inputs*.

7. **No existing shader is edited.** None of the four vertex shader sources and
   none of the four fragment shader sources in `render.rs` changes by one
   character, and no uniform is added to any of the **seven** existing programs
   (solid, text, image, shadow mask, shadow composite, blur, shadow composite
   over the default target). **Five builders in `render.rs` produce them, and the
   count is five and not seven for a reason worth stating:** `create_shadow_programs`
   and `create_blur_programs` each return a *pair*, and `create_program_with_fragment`
   is a shared helper rather than a program of its own. The builders are
   `create_program`, `create_text_program`, `create_image_program`,
   `create_shadow_programs` and `create_blur_programs`; a test asserting the
   program count is against the *seven programs*, not the five builders.
   `git diff ui/src/ui_core/src/render.rs`
   shows no edit inside any `*_SHADER_SRC`. `render.rs`'s module docs gain a
   **`## Transforms and the mesh matrix`** section recording, each item because a
   later task will need it and none of it is visible in the code:

   - the GL-convention `P · V · M` order and why the whole chain is uploaded as
     one `u_mvp` — **not three uniforms**, with the two wrong orders named and the
     `mat3(u_mvp)`-is-not-a-normal-transform arithmetic for the requirement on
     **task 37**;
   - the upload call shape, with **`transpose: false`** and the reason, and the
     silent-transpose failure spelled out;
   - that `P · V` may be cached on `Renderer` **if a measurement ever asks**, and
     that deciding so now would be optimising 640 multiply-adds a frame;
   - that this task's projection reproduces the four 2D shaders' mapping exactly
     (§ *Perspective **and** orthographic*), so a future 2D transform needs the
     matrix and the shaders' `-clip.y`, not a different projection;
   - that `Transform` is unchanged, and why.

8. **One reserved uniform name, and no GL call uses it.**
   `pub(crate) const MESH_MVP_UNIFORM: &str = "u_mvp";` in `render.rs`'s constant
   block, doc-commented: what it is, that **task 37** queries the location from
   the mesh program and uploads through
   `gl.uniform_matrix_4_f32_slice(location, false, mvp.as_slice())`, that
   **nothing in this crate uploads it**, and that `u_view` and `u_projection` are
   **deliberately not reserved** — a reserved name nothing sets is a name a later
   task sets by accident. `rg -c get_uniform_location ui/src/ui_core/src/render.rs`
   is **still 16**, and `rg -n 'uniform_matrix_4_f32_slice' ui/src` returns
   **nothing**.

9. **`DrawCommand` and `ShaderKind` are untouched** — nine variants and four,
   with no transform field on any of them. Which transforms compose, and the
   `car_placement.multiply(&wheel_placement)` chain four independently rotating
   wheels need, is recorded in `render.rs`'s module docs as **the expectation for
   task 37, not a decision**: a `transform: Mat4` field on `DrawCommand::Mesh`,
   with the reasons against a `Vec<Mat4>` in `MeshStore` (a matrix that changes
   every frame must not be mutated while `end_frame` walks the batcher — the
   hazard row **`L6a`** records for a mode that must not change mid-frame — and a
   batch key built from it would compare sixteen floats to decide a merge).
   `render/mesh.rs` is **not edited**.

10. **No `unsafe`, and no dependency.** Zero new `unsafe` blocks: `grep -c SAFETY`
    on `render.rs` is **unchanged**, `grep -c unsafe render/matrix.rs` is **0**,
    and `rg -n 'from_raw_parts|transmute|static mut|unwrap\(\)|expect\(|panic!|unimplemented!|todo!|as f32|as i32' render/matrix.rs`
    returns **nothing**. `ui/Cargo.toml` and `ui/Cargo.lock` are **unchanged** —
    no maths crate, no GL-constants crate. No `assert!` and no `debug_assert!`
    (a precondition checked in the test build and not in the release build is a
    precondition the demo's capture never ran).

11. **The tests, named, with no display, network, filesystem or wall clock.**
    Every one is a pure function of the matrix type, which is the only reason
    `AGENTS.md` permits them at all. **One private epsilon helper** —
    `close(got: f32, want: f32) -> bool` at `1e-5`, and
    `assert_matrix_eq(got: &Mat4, want: [f32; 16])` — used by every one, because
    `f32::cos(PI/2)` is `6.12e-17` and an exact `PartialEq` on `[f32; 16]` is a
    suite that fails on the last bit of a `sin_cos`. **Every assertion is against
    a hand-written literal, never against another `Mat4` method's output** —
    asserting `multiply` against `translated().rotated_z()` is a tautology and
    would have caught nothing. The named tests:

    - `identity_leaves_a_point_where_it_was` — `identity().transform_point([1.0,
      2.0, 3.0])` is `[1.0, 2.0, 3.0]`, and `cols` is the identity literal.
    - `translate_puts_the_translation_in_the_last_column` — `cols[12..15]` is
      `[tx, ty, tz, 1.0]` and columns 0–2 are the identity, which is what
      GLSL's `m[c][r]` indexing means and what catches a row-major mistake.
    - `a_quarter_turn_about_z_takes_x_onto_y` — `rotated_z(FRAC_PI_2)` maps
      `[1.0, 0.0, 0.0]` to `[0.0, 1.0, 0.0]`.
    - `a_quarter_turn_about_y_takes_z_onto_x` and
      `a_quarter_turn_about_x_takes_y_onto_z` — the other two axes, same shape,
      so all three rotations are pinned right-handed.
    - `scale_multiplies_each_axis` and
      `a_zero_scale_collapses_the_axis_it_was_given` — the second exists because a
      zero scale makes a matrix non-invertible and the future normal-matrix
      requirement on 37 must know it can happen.
- `multiply_is_the_matrix_product_and_its_order_is_the_glu_one` — for a
     perspective, a rotation and a translation, `mvp.transform_vec4(point)`
     equals
     `point.transform_vec4(view.transform_vec4(model.transform_vec4(point)))`
     for the same `point`, which is the only test in this task that can catch
     `P · V · M` written in the wrong order — and it needs `transform_vec4`
     precisely because `transform_point` would divide by `w` at each step
    - `the_transforms_compose_translate_rotate_scale_in_that_order` — a point is
      scaled, then rotated, then translated.
    - `transform_to_matrix_agrees_with_hand_computed_window_space_values` — a
      `Transform` with `tx = 10.0, sx = 2.0, sy = 2.0` sends `[1.0, 1.0, 0.0]` to
      `[12.0, 2.0, 0.0]`.
    - `perspective_puts_the_near_plane_at_minus_one_and_the_far_plane_at_one` —
      with `near = 1.0, far = 2.0`, `transform_point([0.0, 0.0, -1.0])` has
      `z = -1.0` and `transform_point([0.0, 0.0, -2.0])` has `z = 1.0`. **These two
      values are the ones that fail if the `[0, 1]` convention is used by
      mistake.**
    - `perspective_is_none_for_every_degenerate_argument` — `fov_y` of `0.0`,
      `PI` and `-1.0`, `aspect` of `0.0`, `near` of `0.0`, `near == far`, and one
      non-finite argument; each `None`.
    - `orthographic_reproduces_the_window_to_clip_mapping_the_four_shaders_write`
      — with `left = 0, right = 640, bottom = 0, top = 480, near = -1, far = 1`:
      `[0, 0, 0]` → `[-1, -1, 0]`, `[320, 240, 0]` → `[0, 0, 0]`,
      `[640, 480, 0]` → `[1, 1, 0]`, **including `z == 0.0`**. This is the test
      that ties the maths to the shipped pipeline.
    - `orthographic_with_positive_distances_puts_z_zero_on_the_near_plane` — the
      `near = 0.1, far = 100.0` trap, asserted as a fact about the sign convention
      rather than left as a warning nobody reads.
    - `orthographic_is_none_for_an_empty_box` — `right == left`, `top == bottom`,
      `far == near`.
    - `transform_point_is_none_at_the_camera_plane_and_keeps_points_behind_it` —
      `w == 0` is `None`; `w < 0` is `Some` with a finite result.
    - `the_sixteen_floats_are_column_major` — `as_slice().len() == 16`,
      `size_of::<Mat4>() == 64`, and `translated([1.0, 2.0, 3.0])`'s whole array
      is the hand-written column-major literal.
    - In `render.rs`: `the_four_2d_vertex_shaders_still_write_the_same_clip_position`
      — all four sources contain
      `gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);` and **none** contains
      `u_mvp` — and `the_mesh_mvp_uniform_is_not_one_the_existing_programs_already_use`,
      which asserts `MESH_MVP_UNIFORM` against **all ten** existing names
      (`u_atlas`, `u_color`, `u_direction`, `u_image`, `u_resolution`, `u_size`,
      `u_source`, `u_taps`, `u_texel`, `u_weights`).
    - **Mutation evidence in the handoff**, for at least
      `multiply_is_the_matrix_product_and_its_order_is_the_glu_one` and
      `orthographic_reproduces_the_window_to_clip_mapping_the_four_shaders_write`:
      break each deliberately, watch it fail for that reason, restore it, watch it
      pass. A test that has never failed is a hypothesis (`developer.md`
      § Phase 3).

12. **The gap row is amended, dated and attributed.** Row **`L2`** in
    `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* gains a dated
    note stating that **task `TASK_UI_PRIM_36` closed the "no matrix" half** — the
    `Mat4`, both projections, and the `Transform` → `Mat4` conversion — and that
    **the uniform is task 37's** and **the 2D half (a transform field on one of
    the nine 2D variants, applied by one of the four 2D shaders) is closed by no
    task in 34–40**. The row is not deleted and not marked closed; the previous
    text stays, and the correction follows
    `DEMO_APPLICATION.md` § *Corrections to the second gap table* in form. Its
    **Blocks** column keeps naming panel slide-ins, card paging, the drive-mode
    strip's drag and resize animations, because those are still open — they are
    the 2D half.

13. **The decisions are recorded where the next agent finds them.** An entry in
    `doc/ui/IMPLEMENTATION_STATE.md` carrying: column-major and why; one
    pre-multiplied `u_mvp` and the three-uniform alternative with the wrong-order
    argument; `Transform` unchanged and the bridge's location; the projection
    defaults (`fov_y = π/4`, `near = 0.1`, `far = 20.0`, ratio 200) and that 40
    must not move `near`/`far`; the fps readout's band for the capture criterion;
    the six pages' frame rates; and **the honest limit, stated rather than
    implied** — *a matrix exists and is tested; no shader declares the uniform, no
    draw command carries a matrix, and no page shows a transformed pixel. `cargo
    test` and a capture are both green on a pipeline whose matrix path has never
    run.* `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`task-sequence.md` § *State*); it points at the code.

14. **The suite, the capture and the frame rate are all produced.** From `ui/`:
    `cargo fmt --check`, `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings`,
    `cargo test --all-features` with the per-binary counts pasted and **no test
    deleted, renamed away or weakened**, `cargo doc --no-deps` clean, and
    `cargo audit` **recorded as not installed on this host, not passed**. Then
    the six-page before/after capture of § *Why no existing shader changes*, and
    then the frame rate: `.ai/tools/fps-check.sh 10 55` on the default page and
    `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
    `Page::ALL`'s six, with the **script's own line pasted** rather than the
    number expected, per `task-sequence.md` § Gates and `developer.md` § Phase 3.
    **The expected result is no measurable change**, and the reason is stated in
    the handoff rather than left as a coincidence: this task adds **no GL call,
    no uniform, no buffer and no per-frame work** — the only new runtime thing is a
    type in a module, and nothing calls it.

## Acceptance Criteria

- [ ] **`u_mvp` is reserved in `render.rs` and used by nothing.**
      `rg -n 'MESH_MVP_UNIFORM' ui/src/ui_core/src/render.rs` shows the constant
      with its doc comment; `rg -c get_uniform_location ui/src/ui_core/src/render.rs`
      is **still 16**; `rg -n 'u_mvp' ui/src/ui_core/src/render/mesh.rs` returns
      **nothing**; and `rg -n 'uniform_matrix_4_f32_slice' ui/src` returns
      **nothing** — no GL call was added, because no mesh program exists
- [ ] **`Transform` now has an edge out of it, and its own file is untouched.**
      `git diff --stat` shows **no change to `ui/src/ui_core/src/property.rs`**;
      `rg -n 'pub struct Transform' -A 12 ui/src/ui_core/src/property.rs` still
      shows exactly `tx`, `ty`, `sx`, `sy`, `rotation` and no more;
      `rg -n 'to_matrix' ui/src` shows the definition in
      `ui/src/ui_core/src/render/matrix.rs` and its call sites in that file's
      tests; and `rg -l Transform ui/src` returns **three** files —
      `property.rs`, `animation.rs`, `render/matrix.rs` (was two).
      **The handoff states what this does not prove**: no widget's paint path
      calls `to_matrix`, and a green test of the conversion is
      not evidence that anything uses it
- [ ] **`Mat4` is 64 bytes, column-major, and reachable without `unsafe`.**
      `the_sixteen_floats_are_column_major` asserts `size_of::<Mat4>() == 64`,
      `as_slice().len() == 16`, and the whole 16-float literal for
      `translated([1.0, 2.0, 3.0])`;
      `translate_puts_the_translation_in_the_last_column` asserts
      `cols[12..15] == [tx, ty, tz, 1.0]`.
      **Mutation evidence:** transpose `translated`'s write into `cols[3], [7],
      [11]` and `the_sixteen_floats_are_column_major` and
      `translate_puts_the_translation_in_the_last_column` fail; restore and they
      pass
- [ ] **`cargo test --all-features` is green with the named matrix tests**, and the
      handoff **lists each of them by name** with its count:
      `identity_leaves_a_point_where_it_was`,
      `translate_puts_the_translation_in_the_last_column`,
      `a_quarter_turn_about_z_takes_x_onto_y`,
      `a_quarter_turn_about_y_takes_z_onto_x`,
      `a_quarter_turn_about_x_takes_y_onto_z`,
      `scale_multiplies_each_axis`,
      `a_zero_scale_collapses_the_axis_it_was_given`,
      `multiply_is_the_matrix_product_and_its_order_is_the_glu_one`,
      `the_transforms_compose_translate_rotate_scale_in_that_order`,
      `transform_to_matrix_agrees_with_hand_computed_window_space_values`,
      `perspective_puts_the_near_plane_at_minus_one_and_the_far_plane_at_one`,
      `perspective_is_none_for_every_degenerate_argument`,
      `orthographic_reproduces_the_window_to_clip_mapping_the_four_shaders_write`,
      `orthographic_with_positive_distances_puts_z_zero_on_the_near_plane`,
      `orthographic_is_none_for_an_empty_box`,
      `transform_point_is_none_at_the_camera_plane_and_keeps_points_behind_it`,
      `the_sixteen_floats_are_column_major`,
      `the_four_2d_vertex_shaders_still_write_the_same_clip_position`,
      `the_mesh_mvp_uniform_is_not_one_the_existing_programs_already_use`.
      **No test was deleted, renamed away or weakened.**
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is **recorded**, not passed
- [ ] **The matrix product's order is tested, and the test can fail.**
      `multiply_is_the_matrix_product_and_its_order_is_the_glu_one` is the only
      test here that can catch `P · V · M` written the wrong way round, and
      **mutation evidence is in the handoff**: change
      `projection.multiply(&view).multiply(&model)` to
      `model.multiply(&view).multiply(&projection)` and watch it fail for that
      reason; restore and watch it pass
- [ ] **The orthographic projection reproduces the shipped 2D mapping, in a test.**
      `orthographic_reproduces_the_window_to_clip_mapping_the_four_shaders_write`
      asserts `[0,0,0] → [-1,-1,0]`, `[320,240,0] → [0,0,0]`,
      `[640,480,0] → [1,1,0]` with `near = -1.0, far = 1.0`, **including
      `z == 0.0`** — which is the value all four shaders hard-code.
      **Mutation evidence:** swap the call's near and far to
      `orthographic(0.0, 640.0, 0.0, 480.0, 1.0, -1.0)` — the natural reading when
      `near`/`far` are imagined as unsigned distances — and watch
      `orthographic_reproduces_the_window_to_clip_mapping_the_four_shaders_write`
      fail on `z == 0.0`, then restore and watch it pass
- [ ] **No existing shader changed, by `git diff` and by a test.**
      `git diff ui/src/ui_core/src/render.rs` shows **no edit inside any
      `*_SHADER_SRC`**, and the four verbatim `gl_Position` spellings in
      `VERTEX_SHADER_SRC`, `TEXT_VERTEX_SHADER_SRC`, `IMAGE_VERTEX_SHADER_SRC`
      and `BLUR_VERTEX_SHADER_SRC` are unchanged;
      `the_four_2d_vertex_shaders_still_write_the_same_clip_position` asserts all
      four still contain
      `gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);` and none contains `u_mvp`;
      `DrawCommand` still has **nine** variants and `ShaderKind` still **four**
- [ ] **The six gallery pages are pixel-identical.** All six of `Page::ALL`'s
      pages (`pads`, `text`, `input`, `controls`, `data`, `overlays`), captured
      **before and after**, release build, `setsid ./target/release/ui_demo >
      log 2>&1 &`, then the commands of `IMPLEMENTATION_STATE.md`
      § *Verifying a change that draws — the capture method* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture, and `ffmpeg x11grab` too, return black for a GL window),
      `pgrep -a -x ui_demo` in the same call as each `magick import -window <id>`,
      and `magick compare -metric AE before.png after.png null:` per page.
      **`AE 0` outside the fps readout's band `y ≥ 680`**, and every differing
      pixel inside it — the band `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it
      decided, and what it found* records as the one thing two captures of an
      unchanged frame differ in (405 pixels there, **AE 0 over y 80–680**). The
      rect-level half is `every_page_places_every_rect_where_the_gallery_placed_it`
      in the demo's suite
- [ ] **The frame rate is measured and reported**, with the **script's own line
      pasted** rather than the number expected.
      `.ai/tools/fps-check.sh 10 55` on the default page, and per page
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` with the
      `roados-fps` line parsed — **`fps-check.sh` cannot name a page**, verified in
      `.ai/tools/fps-check.sh` — it reads `seconds` and `floor` and nothing else,
      and runs the binary with no arguments — which is the amendment
      `IMPLEMENTATION_STATE.md` § *Current position* records for task 24.2's
      criterion 6. Every page is inside the recorded **61.1–63.9** band in
      `IMPLEMENTATION_STATE.md` § *The frame rate, measured* and above the floor
      of 55. **The handoff states the expected result and its reason**: no
      measurable change, because this task adds no GL call, no uniform and no
      per-frame work
- [ ] **Nothing from tasks 34, 35, 37, 38, 39 or 40 leaked in.** `git diff --stat`
      shows **no change to `render/mesh.rs`, `paint.rs`, `batch.rs`, `property.rs`
      or anything under `ui/src/ui_demo/`**; `rg -c Mesh ui/src/ui_core/src/paint.rs`
      and `rg -c Mesh ui/src/ui_core/src/batch.rs` are both **0**; no
      `DrawCommand::Mesh` and no `ShaderKind::Mesh` exist; and
      `rg -n 'set_depth_size|GL_DEPTH_BUFFER_BIT' ui/src` shows task 34's values
      **unchanged** — this task does not re-decide the depth policy
- [ ] **No `unsafe`, no panic path, and no dependency.**
      `grep -c unsafe ui/src/ui_core/src/render/matrix.rs` is **0** and
      `grep -c SAFETY ui/src/ui_core/src/render.rs` is **unchanged from the
      branch point** (this task adds no GL call, so it needs no SAFETY comment);
      `rg -n 'from_raw_parts|transmute|static mut|as f32|as i32|as u32|unwrap\(\)|expect\(|panic!|unimplemented!|todo!|debug_assert|assert!' ui/src/ui_core/src/render/matrix.rs`
      returns **nothing**; and `git diff ui/Cargo.toml ui/Cargo.lock` is **empty** —
      no maths crate (`glam`, `cgmath`, `nalgebra`) was approved for this
- [ ] **`Transform`'s shape is untouched, and `animation.rs` is untouched.**
      `git diff` shows no change to `property.rs` or `animation.rs`; the
      `Interpolate for Transform` impl still interpolates **five** fields
- [ ] **The public API is documented and `#[must_use]`d.** Every `pub` item in
      `render/matrix.rs` has a doc comment, every operation returning a value has
      `#[must_use]`, `cargo doc --no-deps` is warning-free, and the doc comments
      carry what a caller cannot derive — the column-major reason, the
      `transpose: false` reason, the signed-near reason, the `w < 0` is-clipping
      reason, and the `mat3(u_mvp)`-is-not-a-normal-transform requirement on task
      37
- [ ] **The gap row and the state file are updated, dated and attributed.** Row
      **`L2`** in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*
      carries a dated note naming `TASK_UI_PRIM_36` for the "no matrix" half,
      task 37 for the uniform, and **"closed by no task in 34–40"** for the 2D
      half — with the **Blocks** column still naming panel slide-ins, card paging,
      the drive-mode strip's drag and resize animations, because those are open.
      `IMPLEMENTATION_STATE.md` carries the entry from requirement 13, including
      **the honest limit**: a matrix exists and is tested, and nothing draws it
- [ ] **The handoff says what was not done.** Explicitly: no shader was edited, no
      uniform was uploaded, no draw command carries a matrix, no page shows a
      transformed pixel, and `Transform` is unchanged — so `L2` is **not yet
      closed**, only its "no matrix" half is, and the handoff must not read as
      though a transform now reaches a GPU

## Out of Scope

- **No mesh draw command, no mesh shader, no mesh program, no uniform upload.**
  `DrawCommand::Mesh`, `ShaderKind::Mesh`, the GLSL sources, the program link, the
  `get_uniform_location` call and `gl.uniform_matrix_4_f32_slice` are task 37's.
  `DrawCommand` keeps its nine variants, `ShaderKind` its four, and
  `rg -c get_uniform_location ui/src/ui_core/src/render.rs` stays 16. **The buffers
  task 35 uploads are still uploaded and not drawn**, and the matrix task 36 adds
  is still not multiplied into anything: `cargo test` and a capture are both green
  on a pipeline where no fragment has ever seen a matrix
- **No normal matrix, and no `u_normal`, `u_model`, `u_view` or `u_projection`.**
  The model→world normal transform is genuinely needed by 37's fragment shader —
  `mat3(u_mvp)` is not one, because the projection scales x by `1/aspect` and z by
  the near/far terms — and it is **task 37's**, on the rule that a second
  uniform with zero callers is a name a later task sets by accident. This task's
  only deliverable here is the arithmetic written into the module docs so 37
  inherits it. **What would reverse the one-`u_mvp` decision** is a non-uniform
  per-sub-mesh scale: then no upper-left 3×3 of any already-uploaded matrix is a
  valid normal transform, and 37 must add `u_model` or `u_normal` **beside**
  `u_mvp` rather than replacing it. Nothing in tasks 35 or 38 imposes that scale;
  it is a caller's choice, and the sequence has not made it
- **No change to any of the nine 2D `DrawCommand` variants.** **The 2D half of
  gap `L2` therefore stays open**, and that is deliberate rather than
  accidental: putting a `Transform` on one of them means one of the four 2D
  vertex shaders has to apply it, which means all four change to keep them
  consistent, which forfeits the pixel-identity criterion that is the only
  evidence this task touches no rendering. What that task would take is named
  here so it can be written: one transform field (or a `Batch`-level transform,
  since a batch is where `clip` already lives), `u_mvp` multiplied into the four
  identical `gl_Position` writes, the four locations queried, and the y-flip
  question in § *`rotation`'s axis* settled by a captured quarter turn
- **No `Mat4` inverse, determinant, transpose, adjoint, normalise, `inverse_transpose`,
  `look_at`, `look_to`, `Mat3`, `Vec3`, `Vec4`, `Quat`, Euler extraction, TRS
  decomposition, or a `push`/`pop` stack.** Each is a real answer to a real problem
  this sequence does not have yet, and naming them is what keeps task 37 or 40
  from re-deciding one of them by accident
- **No camera type and no depth-precision analysis.** `near`, `far` and `fov_y`
  are decided as constants in the docs; where the camera *is*, and how its matrix
  is built, is 37's and 40's. No `glClipControl`, no reverse-Z, no logarithmic
  depth, no separate near and far objects, no depth prepass — all already refused
  by task 34 and not reopened here
- **No vertex colour, no skinning, no instancing, no per-vertex or per-material
  uniforms, no texture sampling.** One material and one texture cover all five of
  the car's meshes, which is task 35's fact and 37's shader
- **No model file format and no loader.** Task 38 reads a file and produces a
  `Mesh`; this task touches no filesystem, and every test here runs with no
  display, no network, no filesystem and no wall clock — which is the only kind
  `AGENTS.md` permits and the reason the whole of this task is testable at all
- **No asset pipeline.** Task 39 owns texture authoring and packaging
- **No drag-to-rotate, no pointer or gesture handling.** Gap `L4` is task 40's.
  The four independently transformable wheel ranges this task's `multiply` exists
  to serve are task 35's, and what rotates them is 40's
- **No colour capture and no public backdrop API.** Gap `L1` in
  `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* is a different
  task and a different decision
- **No new dependency, and no `unsafe` beyond what a GL call requires.** Per
  `AGENTS.md` the approved direct dependencies are `sdl3 0.20`, `glow 0.18` and
  `freetype-rs 0.38`; a maths crate for a 4×4 multiply, three axis rotations and
  two projections is a licence decision against GPLv3 that nobody has asked for,
  and what it would buy here is SIMD the approved set does not contain. This task
  adds **zero** `unsafe` blocks, because it makes **zero** GL calls
- **No change to `ui_demo`.** No page, no widget, no `--tab=` name, no new asset.
  The six pages must be pixel-identical afterwards, and a demo that drew
  anything new would make that criterion unverifiable rather than merely demanding