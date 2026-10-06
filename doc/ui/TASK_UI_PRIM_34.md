# TASK_UI_PRIM_34: A Depth Buffer, and a Policy That Keeps 2D Out of It

## Goal

Give the default framebuffer a depth buffer, clear it once per frame, and record
the depth policy the renderer will follow from now on: **the depth buffer belongs
to the mesh pass, and the 2D passes neither read it nor write it.**

## Context

This is the first of seven tasks (34–40) that add real-time 3D mesh rendering to
`ui_core`, in service of the Tesla-like demo in `DEMO_APPLICATION.md`. It is
first because every task after it needs a depth test that works:

| # | Subject | Depends on 34 for |
|---|---|---|
| 34 | the depth buffer (this task) | — |
| 35 | mesh vertex format and mesh GPU buffers | nothing at run time; it must not be built first, because its first capture would show the car's inside |
| 36 | `MVP`/matrix math and the transform-to-GPU path (**closes gap `L2`**) | a place to put the projected `z` |
| 37 | the mesh draw command, its shader and batching | the depth test that makes submission order irrelevant |
| 38 | the model file format and the loader | nothing |
| 39 | the asset pipeline (offline render and packaging) | nothing |
| 40 | drag-to-rotate (**closes gap `L4`**) | a rotated mesh to be depth-tested |

Gap `L2` in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* is
*"No transform reaches the GPU"* and belongs to task 36; gap `L4` there
(*"`LongPress` and `Swipe` are emitted and consumed by no widget"*) belongs to task
40. Gap `L1` in that same section — the colour capture and the public backdrop API —
is **not** closed by anything in this sequence, and this task must not creep
toward it (see § *Out of Scope*).

**What the source says today, read not inferred:**

- **`Context::new` in `render/context.rs` calls `gl_attr.set_depth_size(0)` and
  nothing else that touches depth.** The window asks for
  **no** depth buffer, and SDL's own default for that attribute is **16**
  (`sdl3-0.20.0/src/sdl3/video.rs:273`: *"the minimum number of bits in the depth
  buffer; defaults to 16"*). So the `0` is a *removal of a default*, not the
  absence of a decision — which is why the constant it becomes needs the same
  treatment `MULTISAMPLE_BUFFERS` and `MULTISAMPLE_SAMPLES` got: a name, a doc
  comment carrying the reasoning, and a unit test that pins it.
- **There is no depth machinery anywhere in `ui/src/`.**
  `rg -n 'DEPTH_TEST|depth_mask|clear_depth|DEPTH_BUFFER_BIT|CULL_FACE|depth_func|DEPTH_BITS' ui/src/`
  returns **nothing**. No pass enables a depth test, no pass writes depth, no pass
  clears it, and no pass culls a face.
- **The crate declares its own GL constants**, one per line, each with a doc
  comment giving the hex — `render.rs`'s constant block runs from
  `GL_VERTEX_SHADER` down to `GL_CLAMP_TO_EDGE`, and carries `GL_TRIANGLES`,
  `GL_BLEND`, `GL_SCISSOR_TEST`, `GL_COLOR_BUFFER_BIT: u32 = 0x4000` and
  `GL_TEXTURE_2D` among them. This task adds to that block and follows its form.
- **`Renderer::begin_frame` in `render.rs` is where the frame's GL state is
  established.** Today it sets the viewport, calls `gl.disable(GL_SCISSOR_TEST)`,
  `gl.clear_color(0.0, 0.0, 0.0, 1.0)` and `gl.clear(GL_COLOR_BUFFER_BIT)` —
  all four inside its one `unsafe` block — then resets the batcher and clears
  `applied_clip`.
- **`Renderer::end_frame` in `render.rs` is the single submission pass.** It
  walks `self.batcher.submit_order()`'s segments; per segment it draws the opaque
  group with blending **off**, then the three passes in `COMPOSITED_PASSES`
  (`render.rs` — solid, image, text) over the opaque group and then the transparent
  group with blending **on**, then the shadow. Its doc already records one
  deliberate deviation from the batcher's own classification: **the image pass
  blends even for batches the batcher called opaque** — recorded in `end_frame`'s
  own doc, on the grounds that a glyph's coverage and a PNG's transparent corners
  are per-fragment facts the destination has to see. A depth policy that
  contradicts that would have to
  be written the same way — stated where the deviation is, with its reason.
- **All four 2D vertex shaders write the same `gl_Position`.**
  `VERTEX_SHADER_SRC` (solid), `TEXT_VERTEX_SHADER_SRC` (text),
  `IMAGE_VERTEX_SHADER_SRC` (image) and `BLUR_VERTEX_SHADER_SRC` (blur), each in
  `render.rs`, each end
  the assignment `gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);` — **`z` is
  hard-coded to `0.0`**, i.e. every pixel of 2D geometry lands on window depth
  **0.5**.
- **`polygon_quad`'s doc comment in `render.rs` leans on that absence**:
  *"Winding is whatever the fan produced, and nothing depends on it: no pass in
  this module enables face culling or a depth test, so `(a, b, c)` and `(a, c, b)`
  draw the same pixels."* Both halves of that sentence become false the moment a
  depth test exists, so the sentence has to be amended rather than left to rot.
- **MSAA 4× is on the default framebuffer** — `MULTISAMPLE_BUFFERS: u8 = 1`
  (`render/context.rs`), `MULTISAMPLE_SAMPLES: u8 = 4` (`render/context.rs`), both
  applied by the `set_multisample_buffers` / `set_multisample_samples` pair in
  `Context::new`. **The shadow FBO has a colour attachment and nothing
  else**: `ShadowTarget::bind_attach` (`render/target.rs`) attaches to
  `glow::COLOR_ATTACHMENT0` only, and its completeness check would already have
  caught a depth attachment it did not make.

### Why "turn the depth test on" is the wrong answer, in one piece of arithmetic

Window depth is the NDC `z` remapped from `[-1, 1]` to `[0, 1]`, so the 2D passes'
`0.0` becomes **0.5**. Clear depth to `1.0` and enable `GL_DEPTH_TEST` with
`GL_LESS` for the whole frame, and:

1. the first 2D fragment tests `0.5 < 1.0` — it passes, and writes `0.5`;
2. the second 2D fragment tests `0.5 < 0.5` — **false**, and is discarded;
3. so is the third, and the fourth.

**Enabling the depth test over 2D geometry deletes every layer but the first**, and
the result is a window showing the background and nothing else, with no GL error
and a green `cargo test`. That is the failure shape `.ai/NEVERAGAIN.md` records
more than once, and it is the reason this task is a *policy* task and not a
one-line attribute change.

### Why depth cannot order 2D against 3D either

The depth buffer orders geometry **that writes depth**. The 2D passes have one `z`
and, under this policy, no writes — so the buffer contains mesh depths and
nothing else, and no comparison function can express *"the map is behind the car"*.
Ordering between a 2D layer and a 3D layer is therefore **submission order**: the
segment order `end_frame` already walks, which is the same mechanism that decides
where a shadow lands relative to the panel that cast it. Task 37 owns where the
mesh pass sits in that sequence; this task owns the rule and writes it down.

### `glClear` honours the depth writemask

The OpenGL ES 3.1 reference page for `glClear` states that *"the pixel ownership
test, the scissor test, sRGB conversion, dithering, and **the buffer writemasks**
affect the operation of `glClear`"*, and that *"if a buffer is not present, then a
`glClear` directed at that buffer has no effect"*. Two consequences, both
load-bearing here:

- the depth clear must be written with `GL_DEPTH_WRITEMASK` **on**. Under this
  task's resting state (`depth_mask(false)`, below) a clear written without
  re-asserting it silently clears nothing, and a frame that never clears its depth
  buffer is a frame whose mesh pass tests against last frame's depths — a car that
  clips itself, with no GL error and an unchanged frame rate.
- a driver that refused the depth request makes the depth half of the clear a
  documented no-op rather than an error, which is the benign failure.

### `GL_CLEAR_DEPTH_BIT` does not exist, and `0x400` is the stencil bit

The three ES masks for `glClear` are `GL_COLOR_BUFFER_BIT`, `GL_DEPTH_BUFFER_BIT`
and `GL_STENCIL_BUFFER_BIT`. Verified in
`sdl3-src-3.4.16/SDL/src/video/khronos/GLES2/gl2.h:63-65` and in the Khronos
registry's `xml/gl.xml`:

| name | value | where verified |
|---|---|---|
| `GL_DEPTH_BUFFER_BIT` | `0x00000100` | `gl2.h:63`; `gl.xml` `ClearBufferMask` |
| `GL_STENCIL_BUFFER_BIT` | `0x00000400` | `gl2.h:64`; `gl.xml` |
| `GL_DEPTH_TEST` | `0x0B71` | `GLES3/gl3.h:119`; `gl.xml` |
| `GL_LESS` | `0x0201` | `gl3.h:221`; `gl.xml` |
| `GL_LEQUAL` | `0x0203` | `gl3.h:223`; `gl.xml` |
| `GL_GREATER` | `0x0204` | `gl3.h:224`; `gl.xml` |
| `GL_DEPTH_WRITEMASK` | `0x0B72` | `gl3.h:137`; `gl.xml` |
| `GL_DEPTH_COMPONENT16 / 24 / 32` | `0x81A5` / `0x81A6` / `0x81A7` | `gl3.h:336`, `:684`; `glext.h` |

**This task adds `GL_DEPTH_BUFFER_BIT` and does not add `GL_CLEAR_DEPTH_BIT`.** A
constant named that, carrying `0x400`, would clear the *stencil* mask: harmless
while no stencil exists, which is exactly the failure shape
`render/target.rs`'s doc records for the coverage channel — *"nothing errors, the
pipeline reports no GL failure at any step, and the picture is a window with no
shadow on it."* Every value above is also exported by `glow 0.18.0`
(`DEPTH_BUFFER_BIT` at `src/lib.rs:2577`, `DEPTH_TEST` at `:2607`,
`DEPTH_WRITEMASK` at `:2609`, `GREATER` at `:2897`, `LEQUAL` at `:3111`, `LESS` at
`:3113`), so requirement 3 makes the crate's copies and `glow`'s agree rather than
leaving two numbers that can drift.

### Depth size: 24, and what it costs

`SDL_GL_DEPTH_SIZE` is a **minimum** request, so the driver may grant more and the
only way to know is to read it back — which is precisely the discipline
`MULTISAMPLE_SAMPLES`' doc already states for `GL_SAMPLES` (*"a driver that
quietly granted fewer samples than were asked for would be indistinguishable there
from one that granted all four"*). Requirement 11 therefore reads `GL_DEPTH_BITS`
and `GL_SAMPLES` back from a live context and reports both.

- **16** is SDL's own default and the minimum GLES 3.1 requires of a depth buffer.
  It is rejected for the reason it is rejected everywhere else: over a car-sized
  scene with a near plane a few centimetres out, 16 bits quantises `z` coarsely
  enough that two coplanar surfaces — a door skin against a wing, which is what a
  car body is made of — z-fight.
- **32** is not reachable as `GL_DEPTH_COMPONENT32` in the ES 3.1 header this
  build compiles against (it is a desktop-GL/extension sized format there) and is
  not needed: with a near/far ratio task 36 will choose in the range of hundreds,
  24 bits leaves far more precision than the raster can show. It also costs the most
  bandwidth of the three.
- **24** is `GL_DEPTH_COMPONENT24`, the sized format ES 3.1 defines for exactly
  this, and the smallest count that removes the coplanar z-fighting this sequence
  is for. **What would reverse it** is a measurement that 16 is indistinguishable at
  the panel resolution the operator ships — the same "screenshot cannot answer it"
  limit `MULTISAMPLE_SAMPLES` already records, because z-fighting is a *depth*
  artefact and a capture photographs whatever won.

**The MSAA interaction is the real cost, and it is not the bit count.** Depth on a
multisample default framebuffer is stored **per sample**, so the depth allocation
is `samples × bytes`. At this demo's window (`WINDOW` in `ui_demo/src/main.rs` is
1280 × 1020) with 4 samples: **16-bit is 10.4 MB, 24-bit is 15.7 MB, 32-bit is
20.9 MB**, against 5.2 MB of colour today. And there is a second interaction with
the same shape as the one `MULTISAMPLE_BUFFERS`'s doc records: **a driver asked
for four samples *and* a depth buffer may grant fewer samples**, which is why
requirement 11 reports both numbers and requirement 12 makes a dropped sample count
a finding rather than a note.

### Face culling: off, and the reason recorded

Decision: **this task enables no face culling anywhere.** The reasons are that (a)
nothing is culled until task 37 has a closed mesh, and (b) `GL_CULL_FACE` is
*global* state, so enabling it for one pass changes the pixels every other pass
draws — and `polygon_quad`'s doc says in as many words that the fan's winding is
whatever it is *because* nothing culls. The policy is still decided here, in the
file, so task 37 inherits it rather than inventing it: **culling is a mesh-pass
property** — `GL_CULL_FACE` on, `GL_CULL_FACE_MODE` = `GL_BACK` (`0x0405`),
front face left at GL's default `GL_CCW` (`0x0901`), and the `polygon_quad`
sentence amended to say *the 2D passes* leave culling off, which is what will
remain true after 37.

### Blend and depth: a pass that writes depth does not blend

Stated once, in `render.rs`'s module docs, because it is the rule task 37 will
trip over:

- **a pass that writes depth blends off.** A blended fragment that also writes
  depth puts a partially transparent pixel into the depth buffer, and everything
  behind it is then rejected against that pixel — a hard silhouette where the
  material asked for a soft edge.
- **a pass that blends writes no depth** (`GL_DEPTH_TEST` on, `GL_DEPTH_WRITEMASK`
  off): it tests against what the opaque pass wrote and leaves the buffer alone.
  Such a pass is order-dependent within itself and must be drawn back-to-front,
  which is the caller's problem, not the renderer's.

The 2D passes are the degenerate case of the second rule: they blend, and under
this task's policy they neither test nor write, so they are order-dependent among
themselves and are already ordered by the batcher and by
`COMPOSITED_PASSES`.

### The shadow FBO does not need a depth attachment — and the reason is state, not storage

`ShadowTarget` attaches one `GL_R8` colour texture and nothing else, and a
framebuffer with no depth attachment is complete. It does not need a depth
attachment here: a shadow's mask is **one rounded-rect quad**, drawn into a target
cleared immediately before it, so there is nothing for it to occlude; and a
window-sized depth attachment plus a clear per shadowed frame would be paid on
every frame for zero pixels. **A future 3D backdrop would need one, and that is gap
`L1`'s work** — a `GL_RGBA8` colour attachment and a public entry point, neither of
which exists and neither of which is this task's.

The hazard the FBO *does* create is that **depth test state is global, not a
property of a framebuffer**. A mesh pass that left `GL_DEPTH_TEST` enabled would
leak it into the offscreen passes, where it would do nothing useful (no attachment
to read) and could mask a fragment against a stale value if a depth attachment is
added later. So requirement 6 makes `bind_default_target` restore the depth state
for exactly the reason its own doc already gives for the scissor, and requirement 7
makes the mask pass assert it locally.

### How "the 2D pages are unchanged" would be verified

`developer.md` § Phase 3: *"A change that alters what is on screen is not verified
until it has been seen"*, and `AGENTS.md` forbids wall-clock tests — so the
non-regression claim needs a mechanism, not a promise. The method is
`doc/ui/IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
method*, used verbatim: release build, `setsid ./target/release/ui_demo > log 2>&1 &`,
window id **re-read at the time of each capture** with `xwininfo -root -tree` (a
root capture and `ffmpeg x11grab` both return black for a GL window),
`pgrep -a -x ui_demo` in the same call as each `magick import -window <id>`, then
`magick compare -metric AE before.png after.png null:` per page.

**The band is part of the method.** Two captures of an unchanged frame differ
inside the fps readout — `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided,
and what it found* records **405 pixels, and AE 0 over y 80–680** — because the
readout is a moving number. So the criterion is **AE 0 outside `y ≥ 680`**, with
every differing pixel inside that band, on all six of `Page::ALL`'s pages.

**`.ai/tools/fps-check.sh` cannot name a page**, verified: it reads exactly two
positional arguments — `seconds` then `floor` — and then runs
`./target/release/ui_demo` with no arguments. Per-page measurement is
therefore `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` with the
`roados-fps` line parsed by hand — the same amendment
`IMPLEMENTATION_STATE.md` § *Current position* records for task 24.2's criterion 6.

**Why `unsafe` is not an operator decision here.** `AGENTS.md` records that
`unsafe` was declined, and this task needs it anyway, because every GL call in the
crate is one. The reviewer's question is never *whether* a GL call is `unsafe` and
always whether the block introduces an invariant beyond *"The GL context is current
on this thread"*. Requirement 10 is that constraint, written down.

## Requirements

1. **`DEPTH_BITS: u8 = 24` in `render/context.rs`**, declared beside
   `MULTISAMPLE_BUFFERS` and `MULTISAMPLE_SAMPLES` and doc-commented in their
   voice: what it is, why 24 and not 16 or 32 (the three-way trade, with the
   near/far ratio task 36 will pick and the coplanar z-fighting a car body is made
   of), that `SDL_GL_DEPTH_SIZE` is a **minimum** so the granted value is read back
   and not assumed, and what would reverse it. The `gl_attr.set_depth_size(0)` in
   `Context::new` becomes `gl_attr.set_depth_size(DEPTH_BITS)`, and **the `0`
   disappears** — a literal depth size is the failure mode
   `MULTISAMPLE_BUFFERS`'s own doc describes for a sample count with no buffer to
   hold it.
2. **The new GL constants in `render.rs`**, **appended to the end of the existing
   GL-constant block** rather than inserted between two named constants — the
   block is a flat run of `///` doc plus `const`, and naming two members of it as
   an insertion point breaks the moment a third lands between them. Each new
   constant is one line with a doc comment giving the hex and naming where the
   value was verified, in the form `/// GL_DEPTH_TEST constant (0x0B71).`:

   - `GL_DEPTH_BUFFER_BIT: u32 = 0x00000100`
   - `GL_DEPTH_TEST: u32 = 0x0B71`
   - `GL_LESS: u32 = 0x0201`
   - `GL_LEQUAL: u32 = 0x0203`
   - `GL_GREATER: u32 = 0x0204`
   - `GL_DEPTH_WRITEMASK: u32 = 0x0B72`

   `GL_LEQUAL` and `GL_GREATER` are declared in this task and **not used by it**:
   the test function is `GL_LESS` and one unused constant for a comparison the
   policy rejects is noise. They are here because the rejected alternative is
   written in the module docs (see requirement 8) and a doc that names a constant
   the file does not have is the kind of thing a reviewer has to go and check.
   **No `GL_CLEAR_DEPTH_BIT`** — the table in *Context* is the reason, and the
   reason belongs in the module docs where the next reader meets the constant list.
3. **A test pinning every one of those six against `glow`'s own constant of the
   same name.** The crate duplicates `glow`'s numbers on purpose (the block starts
   at `GL_VERTEX_SHADER` and never imports one), and a duplicate with no test is two
   numbers that can drift. This is the same shape as the existing
   `the_image_texture_format_is_rgba_and_not_the_glyph_atlases_red`, which pins a
   format against the neighbour it must not be confused with.
4. **`Renderer::begin_frame` establishes the frame's depth state, in this order,
   inside its existing one `unsafe` block** (the one that today holds `viewport`,
   `disable(GL_SCISSOR_TEST)`, `clear_color` and `clear`):

   1. `gl.bind_framebuffer(glow::FRAMEBUFFER, None)` — **new, and deliberate.**
      `begin_frame`'s clear now targets a buffer whose contents matter, and
      `draw_shadow_offscreen`'s composite restores the default framebuffer through
      `bind_default_target` at the end of every shadowed frame. Relying on a
      previous frame's restore from inside a new frame's setup is the coupling
      `bind_default_target`'s own doc warns about, and it now has a consequence:
      a depth clear aimed at a shadow framebuffer clears nothing visible and the
      mesh pass inherits last frame's depths.
   2. `gl.disable(GL_SCISSOR_TEST)` — already there, and it must stay *before* the
      clear, because `glClear` honours the scissor box.
   3. `gl.depth_mask(true)` — required before the clear, per the `glClear`
      writemask rule in *Context*. It is **not** redundant with GL's default,
      because step 5 of this same list is what leaves the writemask off.
   4. `gl.clear_depth_f32(1.0)` — the far plane. Stated rather than inherited: GL
      defaults the depth clear value to `1.0`, and a constant that means "the far
      plane" should be visible at the call site next to the mask that names it.
   5. `gl.clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT)` — **one** call, not
      two, for the reason the blur's per-axis comment already uses elsewhere in
      this crate.
   6. `gl.depth_func(GL_LESS)` and `gl.disable(GL_DEPTH_TEST)` and
      `gl.depth_mask(false)` — **the frame's resting depth state**: no pass tests
      depth, no pass writes it. The doc comment on the group states the whole
      policy, the `0.5 < 0.5` arithmetic that makes it mandatory, and that task
      37's mesh pass brackets its own draws with `enable(GL_DEPTH_TEST)` and
      `depth_mask(true)`.
5. **The policy is data, so it can be asserted without a display.** A
   `#[derive(Clone, Copy, Debug, Eq, PartialEq)] struct PassDepth { test: bool,
   writes: bool }` and `fn depth_state_for(pass: Pass) -> PassDepth`, private to
   `render.rs`, with `depth_state_for(Pass::Solid)`,
   `depth_state_for(Pass::Image)` and `depth_state_for(Pass::Text)` all returning
   `PassDepth { test: false, writes: false }`. **One function for three callers is
   a function, not a trait** — `developer.md` § Phase 2, and the seam exists here
   because the *policy* is the deliverable and a policy nothing can assert is a
   comment. A test iterates `Pass`'s three variants and asserts the struct for
   each, so a fourth pass added later fails the suite until someone decides what its
   depth state is. `Pass` itself needs no change: task 37 adds the variant.
6. **`Renderer::bind_default_target` restores the depth state as well as the
   framebuffer, viewport and scissor**, and its doc comment gains the second
   reason beside the first — *"The scissor is put back here rather than by the
   caller, and that is the whole of this function's existence in the shadow path"* —
   namely that the offscreen passes are a second writer of global GL state, that
   `apply_clip`'s cache is the model for how this crate handles that, and that
   depth test state has no cache and therefore has to be re-asserted
   unconditionally. It is called with `clip` and re-applies the clip through
   `apply_clip` today; the depth restore goes in the same place and does not touch
   `applied_clip`.
7. **The offscreen mask pass asserts the resting state where it runs.** In
   `draw_shadow_offscreen`, immediately before the existing
`gl.clear_color(0.0, 0.0, 0.0, 0.0)` / `gl.clear(GL_COLOR_BUFFER_BIT)` /
    `gl.disable(GL_BLEND)` block, `gl.disable(GL_DEPTH_TEST)`
   and `gl.depth_mask(false)` — belt to requirement 6's braces, and the two lines
   are cheap. **The colour clear there stays colour-only**: the FBO has no depth
   attachment, and per `glClear`'s own note a clear directed at an absent buffer
   has no effect, so adding the depth bit would be a lie about what the target
   holds. The doc comment beside it says so, and says why no depth attachment is
   added — the arithmetic in *Context*.
8. **`render.rs`'s module docs carry the depth policy**, as a new `## Depth`
   section above `## Frame lifecycle`-equivalent material: the resting state; the
   `0.5 < 0.5` arithmetic that makes a global depth test impossible for 2D;
   **that the depth buffer orders mesh geometry only and that 2D-against-3D
   ordering is submission order** (task 37's decision, recorded here as the rule);
   the rejected alternatives, each named — a global `GL_DEPTH_TEST` with the 2D
   passes pushed to `z = 1.0`, and `GL_LEQUAL` (which would let every 2D layer
   through against another at the same `z`, at the cost of a depth test on every
   fragment of every UI pixel and a rule that only holds because every 2D vertex
   shader happens to write the same `z`); the blend/depth rule (writes depth ⇒
   blend off; blends ⇒ writes no depth); and **the face-culling policy**: culling
   is a mesh-pass property, `GL_CULL_FACE` on with `GL_BACK` and the default
   `GL_CCW` front face, task 37.
9. **`polygon_quad`'s doc comment is amended, not deleted.** Its winding sentence
   (the *"Winding is whatever the fan produced…"* paragraph in `render.rs`)
   becomes true of the passes that still exist — *no 2D pass
   enables face culling or a depth test* — with one sentence added that the reason
   a mesh pass *will* depend on winding is recorded in `render.rs`'s module docs, so
   the next reader of a `Polygon` knows where the rule moved to. The alternative
   (leaving the sentence and letting it become false) is the failure
   `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice: a
   doc comment asserting the opposite of the code beside it.
10. **Every new `unsafe` block carries a SAFETY comment** naming the two
    invariants the block relies on — *"The GL context is current on this thread"*
    and, where a call takes one, what the object is — in the shape
    `begin_frame`'s existing block uses. **No new `unsafe` surface beyond what a GL
    call requires**: no `transmute`, no `static mut`, no raw pointer arithmetic,
    no `unwrap`, no `expect`, no `panic!`/`unimplemented!`/`todo!` in production
    paths. `glow`'s `depth_mask`, `depth_func` and `clear_depth_f32` are already
    `unsafe fn`s on `HasContext`, so this adds **no** `unsafe` the crate did not
    already have.
11. **The measurement is taken and reported, not asserted.** In the handoff, from a
    live context on this host: `GL_DEPTH_BITS` read back with
    `gl.get_parameter_i32(glow::DEPTH_BITS)` and `GL_SAMPLES` with
    `glow::SAMPLES`, both **after** `Context::new`, and both recorded in
    `doc/ui/IMPLEMENTATION_STATE.md` — as a temporary instrument if need be,
    reverted immediately, and with the code that read them quoted. **A `GL_DEPTH_BITS`
    below 24 or a `GL_SAMPLES` below 4 is a finding to report, not a note**, for
    the reason `MULTISAMPLE_SAMPLES`' doc gives: there is no GL context in the test
    harness, so a driver that quietly granted less than it was asked for is
    invisible to every check `cargo test` can make. `gl.get_error()` is read once
    after each new GL call the first time it runs, per `.ai/NEVERAGAIN.md` § *A
    buffer sized for one vertex per quad* — `bind_attach`'s doc records that a
    rejected call with nobody reading it dropped a whole pass.
12. **The frame rate is measured on every page**, and the numbers go in the
    handoff whether they are good or bad, with the script's own line pasted rather
    than the rate expected: `.ai/tools/fps-check.sh 10 55` for the default page,
    and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
    `Page::ALL`'s six, parsed by hand because the script cannot name a page. Every
    page is inside the **61.1–63.9** band in `IMPLEMENTATION_STATE.md` § *The frame
    rate, measured* and above the floor of 55. **The expected result is no
    measurable change**, and the reason is stated in the handoff rather than left as
    a coincidence: no pass tests depth and no pass writes it, so the only new
    per-frame work is one extra bit in a clear that already happened and the
    driver's own per-frame clear of the depth buffer — which is a cost the driver
    pays whether or not anything reads it.

## Acceptance Criteria

- [ ] **`set_depth_size` is non-zero and named.**
      `rg -n 'set_depth_size' ui/src/ui_core/src/render/context.rs` shows
      `gl_attr.set_depth_size(DEPTH_BITS)`, `rg -n 'DEPTH_BITS' ui/src/ui_core/src/render/context.rs`
      shows `const DEPTH_BITS: u8 = 24` with its doc comment, and
      `rg -n 'set_depth_size\(0\)' ui/src` returns **nothing**
- [ ] **`DEPTH_BITS` is pinned by a test, and the test can fail.** A test in
      `render/context.rs`'s existing `#[cfg(test)] mod tests`, in the shape of
      `the_multisample_request_is_four_samples_in_one_buffer`, asserts it is `24`
      and that it is one of `16`/`24`/`32` — 0 is what this change removes and a
      test that admits 0 admits the defect back. **Mutation evidence in the
      handoff:** change it to `16`, watch the suite fail for that reason, restore
      it, watch it pass. A test that has never failed is a hypothesis
      (`developer.md` § Phase 3)
- [ ] **`GL_DEPTH_TEST` and `GL_DEPTH_BUFFER_BIT` are in `render.rs`, and
      `GL_CLEAR_DEPTH_BIT` is not anywhere.**
      `rg -n 'GL_DEPTH_TEST|GL_DEPTH_BUFFER_BIT|GL_LESS|GL_LEQUAL|GL_GREATER|GL_DEPTH_WRITEMASK' ui/src/ui_core/src/render.rs`
      returns all six definitions with their hex doc comments;
      `rg -rn 'GL_CLEAR_DEPTH_BIT' ui/src` returns **nothing**, and the handoff
      says why in one sentence (`0x400` is `GL_STENCIL_BUFFER_BIT`)
- [ ] **All six constants agree with `glow`'s.** The test from requirement 3
      asserts each against `glow::DEPTH_BUFFER_BIT`, `glow::DEPTH_TEST`,
      `glow::LESS`, `glow::LEQUAL`, `glow::GREATER` and `glow::DEPTH_WRITEMASK`, and
      **a mutation that changes one crate constant's hex value fails it**
- [ ] **`begin_frame` clears depth, and clears it with the writemask on.**
      `rg -n 'GL_DEPTH_BUFFER_BIT|depth_mask|clear_depth' ui/src/ui_core/src/render.rs`
      shows `gl.clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT)` in a single
      `clear` call, a `gl.depth_mask(true)` **before** it, `gl.clear_depth_f32(1.0)`
      before that, `gl.bind_framebuffer(glow::FRAMEBUFFER, None)` before all three,
      and the resting `gl.disable(GL_DEPTH_TEST)` / `gl.depth_mask(false)` /
      `gl.depth_func(GL_LESS)` after. Reading the order is the criterion — the
      three orderings are load-bearing for different reasons and a reviewer is
      asked to check them as a sequence
- [ ] **The policy is asserted, not only documented.**
      `depth_state_for(Pass::Solid)`, `depth_state_for(Pass::Image)` and
      `depth_state_for(Pass::Text)` each equal `PassDepth { test: false, writes: false }`,
      asserted by a test that iterates `Pass`'s variants. **Mutation evidence:**
      flipping one entry to `writes: true` fails the suite. And
      `rg -n 'GL_CULL_FACE|CULL_FACE_MODE' ui/src` returns **nothing** — culling is
      policy in this task, code in task 37
- [ ] **The 2D pass state is unchanged, by capture.** `Page::ALL`'s six pages,
      captured **before and after** the change, release build, the commands of
      `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
      method* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab` too, return black
      for a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, and `magick compare -metric AE before.png
      after.png null:` per page. **AE 0 outside `y ≥ 680`**, and every differing
      pixel inside the fps readout's band — which
      `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided, and what it found*
      records as the one thing two captures of an unchanged frame differ in (405
      pixels there, AE 0 over y 80–680). The rect-level half is
      `every_page_places_every_rect_where_the_gallery_placed_it` in the demo's
      suite
- [ ] **The frame rate is measured and reported**, with the script's own line
      pasted rather than the number expected: `.ai/tools/fps-check.sh 10 55` on the
      default page, and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo
      --tab=<page>` for all six pages with the `roados-fps` line parsed —
      **`fps-check.sh` cannot name a page**, verified in `.ai/tools/fps-check.sh`
      — it reads `seconds` and `floor` and nothing else, and runs the binary with
      no arguments — and the reason
      `IMPLEMENTATION_STATE.md` § *Current position* amended task 24.2's criterion
      6 rather than meeting it with the script. Every page inside the recorded
      **61.1–63.9** band and above the floor of 55
- [ ] **What the driver granted is read back and recorded.**
      `GL_DEPTH_BITS ≥ 24` and `GL_SAMPLES == 4`, both read from a live context
      after `Context::new` and both pasted into
      `doc/ui/IMPLEMENTATION_STATE.md` **with the instrument's code quoted** — as
      `MULTISAMPLE_SAMPLES`' doc already does for `GL_MAX_SAMPLES`. A number below
      the request is reported as a finding with its consequence, not filed as a
      note
- [ ] **Nothing from tasks 35 to 38 leaked in.** `git diff --stat` shows no change
      to `paint.rs`, `batch.rs`, `lib.rs` or anything under `ui/src/ui_demo/`;
      `DrawCommand` has **nine** variants and `ShaderKind` its four (`Solid`,
      `Text`, `Image`, `Shadow`);
      `rg -c Mesh ui/src/ui_core/src/paint.rs` and the same for `batch.rs` are
      both **0**; `rg -n 'u_model|mat4|MeshVertex' ui/src` returns nothing.
      `ui/Cargo.toml` and `ui/Cargo.lock` are **unchanged** — no dependency was
      added
- [ ] **The suite is green and the counts are reported.**
      `cargo test --all-features` with the per-binary counts pasted, and **no test
      deleted, renamed away or weakened**; `cargo fmt --check`,
      `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is **recorded**, not passed
- [ ] **Every new `unsafe` block has a SAFETY comment**, and
      `rg -n 'SAFETY' ui/src/ui_core/src/render.rs | wc -l` is at least the number
      of new `unsafe` blocks. `rg -rn 'transmute|static mut' ui/src/` returns
      nothing. And `rg -n 'unwrap\(\)|expect\(|panic!|unimplemented!|todo!'` over the
      new code returns nothing — the depth work adds no panic path
- [ ] **The decision is written down where the next agent finds it.** `render.rs`
      has the `## Depth` module section with requirement 8's content, and an entry
      in `doc/ui/IMPLEMENTATION_STATE.md` recording the depth-size decision, the
      granted `GL_DEPTH_BITS` and `GL_SAMPLES`, the six pages' frame rates, and the
      fact that the depth buffer is cleared per frame and owned by the mesh pass

## Out of Scope

- **No mesh draw command, no vertex format, no mesh shader, no batching.** Task 35
  adds `MeshVertex` and the mesh buffers; task 37 adds the draw command, the
  program and the batch kind. `DrawCommand` keeps its nine variants,
  `ShaderKind` its four, and `ui_core` draws nothing new. **This task enables
  them and nothing more** — and the honest limit of that is worth stating: the
  depth buffer this task adds is cleared, tested by nothing and written by nothing,
  so `cargo test` and a capture are both green on a pipeline with a depth buffer
  no fragment has ever touched. That is the expected state, and the first
  evidence that the buffer works is task 37's first capture
- **No matrix math, no `MVP`, no `u_model`, no transform to the GPU.** Gap `L2` is
  task 36's to close. Nothing here computes a projection, and the `GL_LESS` /
  `GL_LEQUAL` / `GL_GREATER` constants this task declares are the depth **compare**
  functions, not a matrix
- **No model file format and no loader.** Task 38 reads a file and produces a mesh.
  This task touches no filesystem
- **No asset pipeline, no offline render, no packaging.** Task 39's, and nothing in
  this task touches a texture format or an authoring tool
- **No drag-to-rotate, no pointer or gesture handling.** Gap `L4` is task 40's
- **No colour attachment on the shadow FBO and no public backdrop API.** Gap `L1`
  in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* is a different
  task and a different decision: `ShadowTarget` is `GL_R8` **coverage** because a
  shadow's colour is one constant, and a scene needs `GL_RGBA8`. The one thing this
  task says about it is the negative one — the shadow FBO gets **no depth
  attachment** either, and says why
- **No face culling enabled.** Decided and documented (requirement 8), coded in
  task 37. `GL_CULL_FACE`, `GL_CULL_FACE_MODE` and `GL_CW`/`GL_CCW` are not called
  or declared here
- **No stencil, no reverse-Z, no logarithmic depth, no `glClipControl`, no
  separate near and far plane objects, no depth prepass, no `GL_SAMPLE_...`
  coverage-based alpha.** Each of these is a real answer to a real problem this
  sequence does not have yet; naming them here is what keeps task 37 from
  re-deciding one of them by accident
- **No change to MSAA.** `MULTISAMPLE_BUFFERS` and `MULTISAMPLE_SAMPLES` stay at 1
  and 4. Requesting a depth buffer alongside four samples is the one interaction
  that could cost samples, and requirement 11 **measures** it rather than
  pre-empting it with a smaller request
- **No resize handling for depth.** The default framebuffer's depth buffer belongs
  to the window and follows a resize, so there is nothing to reallocate and nothing
  to mirror — which is the one thing `ShadowTarget` has to do for its own textures
  and the reason this task touches no FBO
- **No new dependency, and no `unsafe` beyond a GL call.** Per `AGENTS.md` the
  approved direct dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs
  0.38`; a maths crate, a GL-constants crate or an image-processing crate for
  this task is a licence decision against GPLv3 that nobody has asked for, and
  three constants and a comparison function are not worth one
- **No change to `ui_demo`.** No page, no widget, no `--tab=` name. The six pages
  must be pixel-identical afterwards, and a demo that drew anything new would make
  that criterion unverifiable rather than merely demanding
