# TASK_UI_PRIM_37: The Mesh Draw Command, Its Shader, and Where It Sits in the Frame

## Goal

Give `ui_core` a way to say *"draw this sub-mesh of this mesh, through this
transform, in this colour, at this opacity"* — as a recorded draw command, a
fifth shader kind with its own program, and a pass that draws at its **recorded
position** rather than at a fixed place in the frame's layering.

## Context

This is the fourth of seven tasks (34–40) that add real-time 3D mesh rendering
to `ui_core`. It depends on task 34 (the depth buffer), task 35 (the vertex
format and the GPU buffers) and task 36 (the matrix maths and the
transform-to-GPU path), and it closes **no gap on its own**: gap `L2` in
`DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*
(*"No transform reaches the GPU"*) is closed by **task 36**, and gap `L4`
(*"`LongPress` and `Swipe` are emitted and consumed by no widget"*) by **task
40**. What this task owns is the third thing in that row of 34's table — the
depth test that makes submission order irrelevant *among meshes* — and the one
question 34 explicitly left open: **where the mesh pass sits in the frame.**

### What exists today, read off the source

- **`DrawCommand` has nine variants and not one of them carries a transform or a
  mesh.** In `ui/src/ui_core/src/paint.rs` the nine are `Rect`, `RoundedRect`,
  `Shadow`, `Text`, `Image`, `Line`, `Circle`, `Path` and `Polygon`, in that
  order. The enum's own doc says *"Colors are premultiplied alpha,
  matching the rest of the pipeline"*, which is the contract the tint field
  inherits.
- **`ShaderKind` has four variants** (`ui/src/ui_core/src/batch.rs`): `Solid`,
  `Text`, `Image`, `Shadow`.
- **`BatchKey` is `{ texture, blend_mode, shader }`** and `is_singleton()` returns
  true **only** for `ShaderKind::Shadow`. Its doc comment gives the reason, and
  the reason is about *where* a command draws rather than what it draws:
  *"The only such key is the shadow's… `Batcher::submit_order` splits a frame into
  `Segment`s at every shadow."*
- **`Segment`'s boundary is `shadow: Option<Batch>`**, and `submit_order` splits
  the frame there. A frame with no shadow is one segment.
- **`end_frame` in `render.rs`** reads `self.batcher.submit_order()` and, per
  segment, dispatches one pass per `ShaderKind` over `COMPOSITED_PASSES`
  (`Pass` and `COMPOSITED_PASSES`, both in `render.rs`, the order a test asserts
  in `the_composited_passes_are_drawn_solid_then_image_then_text`).
- **Three vertex structs, four VAOs, four programs, and every submission is
  `draw_elements(GL_TRIANGLES, count, GL_UNSIGNED_INT, 0)`** — in
  `Renderer::submit_vertex_quads` (solid), `Renderer::draw_image_batch` (image)
  and `Renderer::draw_text_batch` (text), all three in `render.rs`. Task 35 adds
  `MeshVertex` (stride 32: `position
  [f32;3]`, `normal [f32;3]`, `uv [f32;2]`, no colour, `#[repr(C)]`, doc-commented
  field by field), one interleaved VBO/IBO, and five named sub-meshes as
  `(first_index, index_count)` ranges with **absolute** indices into that shared
  buffer.
- **Textures are `GL_RGBA8`, premultiplied on load** (`Pixels::premultiply`,
  called from `decode` in `ui/src/ui_core/src/texture.rs`),
  `GL_LINEAR` min and mag, `GL_CLAMP_TO_EDGE`, **no mipmaps**. So a mesh's
  colormap is an ordinary image through the ordinary cache, and **the texel the
  shader samples is already premultiplied** — which is what makes the shader's
  arithmetic checkable rather than hopeful.

### The batching question, and the answer

**A mesh command is a singleton key, for the same reason a shadow is and for one
more reason besides.**

The first reason is the shadow's, and it is decisive on its own. 34 decided that
**2D passes neither test nor write depth**, so the depth buffer orders mesh
geometry and nothing else, and *"Ordering between a 2D layer and a 3D layer is
therefore **submission order**: the segment order `end_frame` already walks,
which is the same mechanism that decides where a shadow lands relative to the
panel that cast it."* A mesh command that did **not** seal the segment would let
a command recorded *after* it merge into a batch recorded *before* it — and that
batch is submitted in an earlier segment, so the chrome would end up **under** the
car. `Batcher::add_clipped`'s doc already states the failure for the shadow case
(*"the panel would be dimmed, which is the defect `Segment` exists to fix,
arrived at by a different road"*), and this is that road again.

The second reason is independent of ordering, and it is why the answer is not
merely *"a boundary"*: **two mesh commands cannot be one `draw_elements` call at
all.** The three existing passes put their per-command state in the **vertex
buffer** — `ImageVertex` carries `a_radius`, `a_opacity`, `a_size`, so a whole
batch draws with no per-command uniform. A mesh's per-command state is a `Mat4`
and an index range, which cannot ride in a vertex buffer without re-uploading the
model once per transform. So merging two mesh commands would save nothing (they
are already one draw call each) and would cost correctness.

`BatchKey::is_singleton()` therefore becomes
`matches!(self.shader, ShaderKind::Shadow | ShaderKind::Mesh)`, and its doc
comment is **amended rather than replaced**: the sentence *"The only such key is
the shadow's"* becomes false the moment a mesh exists, and a doc comment asserting
the opposite of the code beside it is the defect `DEMO_APPLICATION.md`
§ *Corrections to the second gap table* records twice.

**`Segment` gains a second boundary slot, `mesh: Option<Batch>`, beside
`shadow`.** Not a `Boundary` sum type: the invariant is *"at most one of the two
is `Some`"*, it already holds by construction (a seal consumes exactly one
singleton command, so a segment ends with exactly one), and a sum type would
rename a `pub` field that twenty-three of `batch.rs`'s own tests and two of its
doc examples name, for an invariant that needs a test rather than a type. The
invariant gets that test.

### Where the mesh pass sits, and the ordering problem it solves

**After the segment's opaque and transparent groups, beside the shadow, before
the next segment's anything.** And a mesh and a shadow can never be in the same
segment, so their relative order in the code is unobservable — which is stated in
the code rather than left for a reader to work out.

That position *is* the answer to "the car is over the map but the map is 2D with
no depth". The resolution is not a z-order hack and not a depth trick. It is:

- **mesh against mesh: the depth buffer.** The mesh pass enables `GL_DEPTH_TEST`
  with 34's `GL_LESS` and writes depth when the command is opaque, so five
  sub-meshes and anything else in the scene sort themselves. This is what task 34
  bought and it is the only thing the depth buffer can do.
- **mesh against 2D: submission order, and nothing else.** A 2D command recorded
  before the mesh is submitted in an earlier segment and is therefore *under* it; a
  2D command recorded after it is submitted later and is *over* it, whatever the
  mesh's depth says — because the 2D passes do not test depth. So the contract
  is a **recording contract**, and it is the caller's:

  > record the map before the mesh and the chrome after it.

  This is not a limitation worked around; it is 34's recorded policy, and this
  task's contribution is that the policy is now **expressible**. Today there is no
  way to say "car over map" in this pipeline at all.

  The honest edge of it: a mesh recorded *before* the map is under the map. That
  is correct behaviour for submission order and not a defect, and the doc comment
  on `DrawCommand::Mesh` says so in those words so the next reader does not go
  looking for a depth flag that will never exist.

### The lighting model, stated as flat/Lambert and defended

**One directional light plus ambient. No specular, no normal map, no shadow
mapping, no fog, no tone mapping, no second light.** Four reasons, each a fact
rather than a preference:

- **The data supports exactly this and no more.** The model ships `NORMAL` per
  vertex (35 records it), so a per-fragment Lambert on an interpolated normal
  costs one `normalize` and one `dot`. A specular term needs a view vector and a
  gloss value; the material is **one texture with regions distinguished by UV**
  and there is no channel anywhere that says which parts are glossy.
- **A normal map is unreachable without changing 35's decision.** A normal map
  needs a tangent frame, and `MeshVertex` is three fields at stride 32 by task
  35's requirement 1. Adding a tangent is a **vertex-format change**, so it
  belongs with whoever changes the format, not here.
- **Shadow mapping needs a depth-only FBO, and 34 says the shadow FBO gets no
  depth attachment.** 34's *Context* is explicit that a 3D backdrop needing one
  is gap `L1`'s work — a separate task with a separate decision. A mesh pass that
  cast shadows would be that task, done badly and early.
- **The demo's need is shape legibility, not realism.** A car body at 1280×1020
  with one light reads as a solid object from the Lambert term alone. Every
  shading term past that is a term whose *parameters* this project does not have,
  and `DEMO_APPLICATION.md` § *Asset requirements* already records the governing
  principle for missing parameters — first-principles choices, explicitly not
  transcribed values.

**The light is a fixed direction in view space**, and the normal is transformed by
the **inverse transpose of the model-view 3×3**, not by the MVP. Two consequences
worth writing into the shader's doc comment:

- Transforming a normal by the MVP is wrong for any perspective projection, which
  is non-uniform by construction. So the mesh shader takes **two** matrix
  uniforms, `u_mvp` (mat4) and `u_normal_matrix` (mat3).
- The light being view-space means **rotating the car changes its shading**. That
  is what makes task 40's drag-to-rotate legible: a rotating flat-shaded object
  under a fixed light shows the rotation, where a rotating object under a
  camera-attached light would not change at all.

`u_light_dir` is normalized on the CPU and `u_ambient` is a scalar in `0.0..=1.0`,
so `shade = ambient + (1 - ambient) * lambert` is in `0.0..=1.0` by construction —
which is what the premultiplication proof in the next section needs.

### Premultiplied alpha: what this task gets right, and what it does not fix

**The mesh shader keeps the fragment premultiplied, and the CPU tint is an
alpha-only scale.** The proof is short and it is a property of the arithmetic
rather than a hope:

- `Pixels::premultiply`, called from `decode` in `texture.rs`, premultiplies on
  load, so `texel.rgb <= texel.a`.
- `u_tint` is premultiplied by `mesh_tint` (below), so `tint.rgb <= tint.a`.
- The composition of two premultiplied colours is
  `vec4(a.rgb * b.rgb, a.a * b.a)`, and `a.rgb * b.rgb <= a.a * b.a` follows from
  both operands — so the **base is premultiplied**.
- `frag_color = vec4(base.rgb * shade, base.a)` with `shade <= 1` keeps it so, and
  **alpha is deliberately not shaded**: shading alpha would make a lit surface
  more transparent, which is backwards.

**This does not copy the defect `chart.rs` § *What made the first attempt at this
measurement wrong* records, and it does not fix it either.** `quad_color` in
`render.rs` returns `[r/255, g/255, b/255, a/255]` from a `Color`
whose own doc says the components are already premultiplied — so given a
contract-honouring input it is faithful, and the defect is upstream: that same
section of `chart.rs` measures `Palette::fill` handing the solid pass a **straight**
colour, and records the three predicted-and-measured results. So:

- `mesh_tint(tint, opacity)` **scales alpha only**: `rgb` is `r/255` untouched and
  `a` is `a/255 * opacity`. It never multiplies `rgb` by `a`, because `Color`'s r,
  g and b are already premultiplied and doing it twice darkens every honest
  caller. An alpha-only scale is exactly the operation that preserves the
  invariant, and it is the operation the image fragment shader already uses
  (`vec4(texel.rgb * opacity, texel.a * opacity)`).
- The consequence for a caller that passes a **straight** colour is the same wrong
  picture `chart.rs` measured, and it is recorded here rather than papered over.
  Fixing `Color`'s contract is **not** this task: it changes all nine existing
  variants' pixels and is a separate decision with its own measurements.

**The proof is a test, not a comment.** A pure CPU mirror of the fragment
arithmetic, `mesh_fragment(texel, tint, shade)`, lets a fixture assert
`r <= a && g <= a && b <= a` componentwise across a grid of contract-honouring
texels, tints and shades — **with no display**, which is the only kind of test
`AGENTS.md` permits.

### What is decided here and what is inherited

Inherited and **not re-decided** here: `MeshVertex` and the 32-byte stride and
its `layout(location = 0..2)` table (35); the depth policy, the resting state,
`GL_LESS`, and the blend/depth rule (34); `Mat4` and the uniform names (36).

**Task 36's file is not in the tree** (`doc/ui/TASK_UI_PRIM_36.md` does not
exist), so the two matrix uniforms are named here as `u_mvp` and
`u_normal_matrix` with the rule stated: **if 36 named them otherwise, 37 uses 36's
names and nothing else in this file changes.** The requirement that survives
either way is the *number* and *type* of them — one `mat4` and one `mat3`, and a
`mat3` rather than a `mat4` because a normal is a direction.

### Scope check

`developer.md` § *Scope check* asks for a file count and a component count before
committing to a shape. **Four files**: `paint.rs`, `batch.rs`, `render.rs`, and
`render/mesh.rs` (one `Copy` range type, which belongs to the batching change).
**Three components**: the command and its `Painter` method; the batching change
(`ShaderKind`, `Segment`, `is_singleton`, `batch_key`, `submit_order`); and the
shader, program, uniforms and draw path. Under both thresholds. The three are
**coupled, not splittable** — `batch_key` matches on the new `DrawCommand` variant
and `draw_mesh_batch` dispatches on the new `ShaderKind` — so
`.ai/protocols/subagents.md` § *Implementation fan-out* does not apply and this is
one developer's change.

## Requirements

1. **`DrawCommand::Mesh`, the tenth variant, in `paint.rs` after `Polygon`**,
   `#[derive(Clone, Debug, PartialEq)]` like its nine siblings, with every field
   doc-commented in the house voice:

   ```rust
   /// A triangle mesh: one named sub-mesh of one uploaded mesh, through one
   /// transform, in one tint.
   ///
   /// **The transform is on the command and not on the mesh**, because the five
   /// sub-meshes of one car share one buffer and differ only in the transform
   /// they are drawn through — which is why task 35 made them index ranges into
   /// one interleaved pair rather than five buffer pairs.
   ///
   /// **Where it lands in the frame is the caller's recording order, and there is
   /// no flag that changes it.** A command recorded before this one is submitted
   /// first and is under it; a command recorded after it is submitted later and is
   /// over it, whatever this mesh's depth says — the 2D passes neither test nor
   /// write depth, so the depth buffer cannot express "the map is behind the car"
   /// and no comparison function on it ever will. Record the map first and the
   /// chrome second. A mesh recorded *before* the map is under the map: that is
   /// correct for submission order, not a defect.
   Mesh {
       /// The mesh to draw from, as returned by `Renderer::upload_mesh`.
       mesh: MeshId,
       /// Which part of that mesh: one sub-mesh's `(first_index, index_count)`
       /// range, with **absolute** indices into the shared index buffer.
       range: SubMeshRange,
       /// Model-view-projection, from the renderer's transform path.
       mvp: Mat4,
       /// Material tint, premultiplied, like every other colour on this enum.
       tint: Color,
       /// How much of the mesh reaches the screen, `0.0..=1.0`, clamped when
       /// drawn.
       ///
       /// **A scalar and not part of `tint`,** because it decides the blend mode
       /// and the blend mode must not depend on how the caller chose to express
       /// the alpha: a tint of alpha 0 draws nothing and is transparent, while
       /// `opacity: 1.0` says the draw is opaque whatever the tint. This is the
       /// same split `DrawCommand::Image`'s `opacity` makes against the texture.
       opacity: f32,
       /// The material's colormap, sampled by the mesh pass.
       ///
       /// **On the command rather than on the mesh**, because `BatchKey`'s
       /// `texture` is what the pass reads the sampler from — the same field
       /// `draw_image_batch` reads — and because one model may be drawn with a
       /// different colormap without re-uploading its geometry.
       texture: TextureId,
   },
   ```

   **`Painter::mesh(&mut self, mesh: MeshId, range: SubMeshRange, mvp: Mat4,
   tint: Color, opacity: f32, texture: TextureId)`**, matching the signature and
   the recording shape of the other nine methods, with a `# Examples` doc test in
   their form — **which is a test that needs no display**, because it records and
   pattern-matches and never opens a window.

2. **`SubMeshRange` in `render/mesh.rs`, declared by this task**, and the
   arithmetic that goes with it:

   ```rust
   /// One sub-mesh's range in the shared index buffer.
   #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
   pub struct SubMeshRange {
       /// First index of the range, absolute into `Mesh::indices`.
       pub first_index: u32,
       /// How many indices the range holds; a multiple of three.
       pub index_count: u32,
   }
   ```

   with `SubMesh::range(&self) -> SubMeshRange` on task 35's `SubMesh`, and
   **`SubMeshRange::byte_offset(&self) -> Result<i32, RenderError>`** as the
   *single* place the conversion `first_index × size_of::<u32>()` happens.
   `sub_mesh_byte_offset` from task 35 **becomes a one-line delegate** to it, and
   **a test asserts the two return the same value for a fixture of ranges** — the
   crate's own rule that a duplicate with no test is two numbers that can drift.
   Doc comment on the type records why a newtype and not two `u32` fields on the
   command: the two cannot be transposed by accident, and `first_index: 1` is
   **4 bytes**, not 1.

   `MeshId` and `Mat4` must both be `Clone + Debug + PartialEq` for
   `DrawCommand`'s derives, and `MeshId`'s `Copy` is what lets a `Painter` method
   take it by value without a clone. **That is a requirement on task 36's `Mat4`**
   and is stated here rather than assumed.

3. **`ShaderKind::Mesh`, the fifth variant**, in `batch.rs` beside `Shadow`, with
   a doc comment that says what the kind *is* rather than what the shader looks
   like, in `Shadow`'s voice — including the sentence that it can never merge into
   another batch, and **why**: *"a mesh's per-command state is a transform and an
   index range, and neither can ride in a vertex buffer the way a rounded
   rectangle's radius and size do, so two mesh commands are two draw calls however
   their keys compare."*

4. **`BatchKey::is_singleton()` returns true for `Mesh` as well as `Shadow`** —
   `matches!(self.shader, ShaderKind::Shadow | ShaderKind::Mesh)` — and **its doc
   comment is amended, not replaced**. The amendment keeps the existing reasoning
   (the reason is about *where* a command draws) and adds: *"Two keys are
   singletons and they are singletons for the same reason, which is that
   `submit_order` splits the frame at each of them: a command recorded after a
  mesh or a shadow must not merge into a batch recorded before it, or it lands on
  the wrong side of it. They differ in the second reason — a shadow needs an
  offscreen target, a mesh needs a per-command transform — and in neither does
  merging buy anything: each is one draw call already."*

   `DrawCommand::batch_key()` gains the `Mesh` arm:
   `texture: Some(*texture)`, `shader: ShaderKind::Mesh`, and a blend mode decided
   by `opacity` **exactly as `DrawCommand::Image`'s arm decides it** —
   `BlendMode::Opaque` only when `*opacity == 1.0`, not "at least", for 35 lines
   of the same reason. The arm's doc comment says so by reference.

5. **`Segment` gains `mesh: Option<Batch>`** beside `shadow`, both doc-commented
   in the existing field's voice, both documented as *"the boundary the segment
   ends with"* with the invariant stated on the struct: **`at most one of `shadow`
   and `mesh` is `Some`, because a seal consumes exactly one singleton command`.
   `Batcher::submit_order`'s trailing-run `Segment` literal sets `mesh: None`
   alongside `shadow: None`, and its sealed-run branch classifies the popped
   batch by `key.shader` — a `match` with a `ShaderKind::Shadow` arm, a
   `ShaderKind::Mesh` arm, and an arm that **pushes the batch back** rather than
   dropping it, in the shape the existing code already uses and in as many words.

   **Three tests, all display-free:** a frame with no mesh and no shadow still
   yields one segment whose two boundary slots are `None`; a fixture of
   alternating shadows and meshes yields `n` boundaries in recorded order across
   `n + 1` segments; and **every segment of that fixture has at most one boundary
   slot filled**, which is the invariant a two-`Option` type cannot say for itself.

6. **The mesh program, its uniforms, and the shader sources** in `render.rs`,
   beside the four existing programs and their uniform locations:

   - `MESH_VERTEX_SHADER_SRC` and `MESH_FRAGMENT_SHADER_SRC`, as `const &str`
     raw literals in the existing form.
   - `create_mesh_program(gl)`, following `create_image_program`'s shape exactly:
     compile both, create, attach, link, detach, delete both shaders, and on a
     failed link `delete_program` and return `Err(RenderError::ProgramLink(log))`.
   - `Renderer` gains `mesh_program: glow::Program` and six
     `Option<glow::UniformLocation>` fields — `u_mvp`, `u_normal_matrix`,
     `u_colormap`, `u_light_dir`, `u_ambient`, `u_tint` — each queried in
     `Renderer::new` beside the others and each carrying the same doc note the
     existing locations carry about a location belonging to the program it was
     queried from.

   **The vertex shader:**

   ```glsl
   #version 300 es
   precision highp float;
   layout(location = 0) in vec3 a_position;
   layout(location = 1) in vec3 a_normal;
   layout(location = 2) in vec2 a_uv;
   uniform mat4 u_mvp;
   uniform mat3 u_normal_matrix;
   out vec3 v_normal;
   out vec2 v_uv;
   void main() {
       gl_Position = u_mvp * vec4(a_position, 1.0);
       v_normal = u_normal_matrix * a_normal;
       v_uv = a_uv;
   }
   ```

   `layout(location = 0..2)` with those names and component counts **must match
   task 35's requirement 2 table exactly** — `a_position` 3×`GL_FLOAT` at 0,
   `a_normal` 3×`GL_FLOAT` at `MESH_NORMAL_OFFSET`, `a_uv` 2×`GL_FLOAT` at
   `MESH_UV_OFFSET`. **A test asserts the three declarations are present in the
   source**, which is the only check that catches a mismatch between two files
   that are otherwise free to disagree, and it costs three greps.

   `precision highp float;` — **not** `mediump` as the four 2D shaders use, and
   the difference is stated in the source's doc comment: those shaders work in
   window coordinates, `0.0` to `1280.0`, which `mediump` spans comfortably,
   while a 4.5 m car at a near plane 36 chooses in the range of hundreds needs
   more than `mediump`'s roughly three decimal digits. GLES 3.1 **requires**
   `highp` support in fragment shaders, so this asks for nothing the target may
   refuse.

   **The fragment shader:**

   ```glsl
   #version 300 es
   precision highp float;
   in vec3 v_normal;
   in vec2 v_uv;
   uniform sampler2D u_colormap;
   uniform vec3 u_light_dir;
   uniform float u_ambient;
   uniform vec4 u_tint;
   out vec4 frag_color;
   void main() {
       vec3 normal = normalize(v_normal);
       float lambert = max(dot(normal, u_light_dir), 0.0);
       vec4 texel = texture(u_colormap, v_uv);
       vec4 base = vec4(texel.rgb * u_tint.rgb, texel.a * u_tint.a);
       float shade = u_ambient + (1.0 - u_ambient) * lambert;
       frag_color = vec4(base.rgb * shade, base.a);
   }
   ```

   Four lines of that are **contracts rather than taste**, and each is stated as
   such above and in the source's doc comment: the texel is **premultiplied**
   (`Pixels::premultiply`, called from `decode` in `texture.rs`) and the tint is
   **premultiplied** (`mesh_tint`), so the
   product is premultiplied; **`shade` is in `0.0..=1.0`** because `u_ambient` is
   a scalar in that range and `lambert` is a clamped `dot`; **`base.a` is not
   shaded**, because a lit surface must not become more transparent; and there is
   **no specular, no reflection vector, no tangent attribute, no second sampler,
   and no shadow lookup** — which is the lighting model requirement 6 defends and
   acceptance criterion 8 greps for.

7. **`mesh_tint(tint: Color, opacity: f32) -> [f32; 4]`, private to
   `render.rs`**, in `mesh_tint`'s place beside `quad_color`:

   ```rust
   [r/255, g/255, b/255, (a/255) * opacity.clamp(0.0, 1.0)]
   ```

   **`rgb` untouched by `opacity`; `a` scaled by it.** The doc comment says why in
   the first line: *"an alpha-only scale is the operation that preserves
   premultiplication, and it is the operation the image fragment shader already
   uses."* And it says what it deliberately does **not** do — *"it never
   multiplies `rgb` by `a`, because `Color`'s r, g and b are already
   premultiplied; doing it twice darkens every honest caller"* — with a pointer to
   `chart.rs`'s recorded measurements for what a *straight* input costs, and a
   sentence that fixing `Color`'s contract is a separate decision that would
   change all nine existing variants.

8. **`mesh_fragment(texel: [f32; 4], tint: [f32; 4], shade: f32) -> [f32; 4]`**,
   a pure CPU mirror of the fragment shader's three arithmetic lines, private to
   `render.rs`, carrying the doc comment that it exists **so the premultiplication
   invariant can be asserted without a display** and that the GLSL string and this
   function **must be edited together** — the same obligation
   `IMAGE_FRAGMENT_SHADER_SRC`'s doc already states for the corner-clipping
   expression it shares with the solid shader.

   **`depth_state_for` is widened from `fn depth_state_for(pass: Pass) -> PassDepth`
   to `fn depth_state_for(pass: Pass, blend: BlendMode) -> PassDepth`**, which is
   an amendment to task 34's requirement 5 and is required rather than optional:
   34's own blend/depth rule — *"a pass that writes depth blends off; a pass that
   blends writes no depth"* — makes a mesh's depth state depend on its blend
   mode, so a `Pass`-only signature **cannot express the correct answer** for the
   pass 34 said task 37 would add. The values:

   | pass | blend | test | writes |
   |---|---|---|---|
   | `Solid` / `Image` / `Text` | either | `false` | `false` |
   | `Mesh` | `Opaque` | `true` | `true` |
   | `Mesh` | `Transparent` | `true` | `false` |

   **A test iterates all four passes × both blend modes** — eight rows, so a fifth
   pass added later fails the suite until someone decides its depth state, which
   is the whole reason 34 made the policy data.

9. **`draw_mesh_batch(&mut self, batch: &Batch) -> Result<(), RenderError>`**,
   beside `draw_image_batch`, with this order of operations and each step's reason
   in the doc comment:

   1. **Skip a batch that is not `ShaderKind::Mesh`** — `return Ok(())`, the shape
      all three existing `draw_*_batch` functions use.
   2. **`self.apply_clip(batch.clip)`**, before anything else, because a draw call
      has one scissor and a mesh clipped to a scrolling viewport must be clipped
      in window space like every other batch.
   3. **Resolve the mesh**: `self.meshes.get(mesh)` from task 35's `MeshStore`, and
      **`None` is a skip, not a panic** — 35's requirement 9's rule, honoured at
      the draw site as well as the store.
   4. **Bounds-check `range` against the record's `indices.len()`** with
      `checked_add`, and **skip an out-of-range range rather than issuing the
      draw** — with the doc comment naming 35's reasoning: a range past the end of
      the index buffer *"is what `draw_elements` reads, and without robust buffer
      access that is undefined geometry rather than an error."*
   5. **`gl.use_program(Some(self.mesh_program))`**, then
      `uniform_2_f32`-equivalent for nothing — **the mesh shader reads no
      `u_resolution`**, because it does not work in window coordinates — then
      `uniform_matrix_4_f32_slice(self.u_mvp.as_ref(), false, mvp_slice)`,
      `uniform_matrix_3_f32_slice(self.u_normal_matrix.as_ref(), false,
      normal_slice)`, `gl.active_texture(GL_TEXTURE0)`,
      `gl.bind_texture(GL_TEXTURE_2D, Some(texture))`,
      `uniform_1_i32(self.u_colormap.as_ref(), 0)`,
      `uniform_3_f32(self.u_light_dir.as_ref(), …)`,
      `uniform_1_f32(self.u_ambient.as_ref(), …)`,
      `uniform_4_f32(self.u_tint.as_ref(), …)`. **`transpose` is `false`** — GLSL
      matrices are column-major — and 36's accessor is used whatever it is called,
      with the column-major requirement stated as a requirement **on 36**.
   6. **Bracket the draws with the depth and culling state, and restore it after**,
      in this order and no other: `gl.enable(GL_CULL_FACE)`,
      `gl.cull_face(GL_BACK)`, `gl.enable(GL_DEPTH_TEST)`,
      `gl.depth_func(GL_LESS)`, `gl.depth_mask(writes)`; then
      `gl.bind_vertex_array(Some(self.mesh_vao))`,
      `gl.draw_elements(GL_TRIANGLES, index_count, GL_UNSIGNED_INT, byte_offset)`,
      `gl.bind_vertex_array(None)`; then on the way out `gl.depth_mask(false)`,
      `gl.disable(GL_DEPTH_TEST)`, `gl.disable(GL_CULL_FACE)`.

      **Front face is left at GL's default `GL_CCW`** and no `front_face` call is
      made, per 34's policy. **The element array buffer is not bound here** — it is
      35's requirement 6's hazard, and the mesh VAO's own binding is what makes
      this draw read the right indices. **Blending** is set from
      `batch.key.blend_mode`: `gl.disable(GL_BLEND)` for `Opaque`,
      `gl.enable(GL_BLEND)` + `gl.blend_func(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)` for
      `Transparent`, in the image pass's shape.
   7. **`MESH_LIGHT_DIR` and `MESH_AMBIENT` are `Renderer` constants**, uploaded
      in step 5 on every mesh batch rather than once per frame. The doc comment
      says why: the mesh program is only current inside this function, there is no
      "mesh pass begins here" hook that runs once per frame, and five sub-meshes
      cost three extra GL calls each — while a per-frame upload would need
      `use_program` during `begin_frame` for no measurable gain.

10. **The GL constants this task adds, in `render.rs`'s own block**, each one line
    with a doc comment giving the hex and naming where it was verified, in the
    form `/// GL_CULL_FACE constant (0x0B44).`:

    - `GL_CULL_FACE: u32 = 0x0B44`
    - `GL_CULL_FACE_MODE: u32 = 0x0B45`
    - `GL_BACK: u32 = 0x0405`

    **34's requirement 2 declared the six depth constants and not these three**,
    because 34's requirement 8 and § *Face culling: off, and the reason recorded*
    decided the policy and left the code here: *"culling is a mesh-pass property —
    `GL_CULL_FACE` on, `GL_CULL_FACE_MODE` = `GL_BACK`, front face left at GL's
    default `GL_CCW`… and task 37 inherits it rather than inventing it."* Using
    `glow::CULL_FACE` and friends directly is rejected in the same breath, for
    34's reason: the crate's block is where a reader looks, and a duplicate with
    no test is two numbers that can drift.

    **A test pins all three against `glow::CULL_FACE`, `glow::CULL_FACE_MODE` and
    `glow::BACK`** (all three exported by `glow` 0.18.0), and the existing six
    depth pins are re-asserted rather than assumed to still hold.

11. **`end_frame` gains the mesh draw, and `Pass` gains the variant.**
    `Pass::Mesh` is a fifth variant of the private enum, doc-commented as *"a
    triangle mesh, through the mesh program, with the depth test on."*
    `COMPOSITED_PASSES` **stays `[Pass; 3]`** — a mesh is **not** a composited
    pass, and the doc comment says why: *"a composited pass is drawn over the
    opaque and transparent groups at a fixed place in the frame's layering, and a
    mesh must be drawn where it was recorded, not at a fixed place."*

    The boundary dispatch in `end_frame`'s segment loop becomes two `if let`s
    after the `COMPOSITED_PASSES` walk:

    ```rust
    // The shadow and the mesh both come after everything the segment recorded
    // and before everything the next one will, which is the whole of what the
    // segmentation is for. They cannot both be here — a seal consumes exactly
    // one singleton command — so their order relative to each other is not
    // observable; the shadow is drawn first because a composited full-window
    // effect belongs above everything else in its segment.
    if let Some(shadow) = &segment.shadow {
        self.draw_shadow_batch(shadow)?;
    }
    if let Some(mesh) = &segment.mesh {
        self.draw_mesh_batch(mesh)?;
    }
    ```

    `draw_pass` gains a `Pass::Mesh` arm routing to `draw_mesh_batch`, so the
    dispatch is exhaustive over the enum and a future fifth pass cannot be added
    without the compiler asking where it draws.

12. **Every new `unsafe` block carries a SAFETY comment** naming the two
    invariants the block relies on — *"The GL context is current on this thread"*
    and, where a call takes one, *"`<object>` is a valid … created in
    `Renderer::new`"* — in the shape `draw_image_batch`'s existing block uses.
    **No new `unsafe` surface beyond what a GL call requires**: no `transmute`, no
    `static mut`, no raw pointer arithmetic, no `unwrap`, no `expect`, no
    `panic!`/`unimplemented!`/`todo!` in any production path. `glow`'s
    `cull_face`, `depth_mask`, `depth_func`, `uniform_matrix_4_f32_slice` and
    `uniform_matrix_3_f32_slice` are already `unsafe fn`s on `HasContext`, so this
    adds **no** `unsafe` the crate did not already have. Every fallible path
    returns `Err(RenderError::Gl(…))` naming what was wrong.

13. **The handoff reports measurements, not expectations**, and pastes the
    script's own line rather than the number it expected:

    - **`gl.get_error()` is read once after the first mesh draw** and the result
      pasted: `bind_attach`'s doc records that a rejected call with nobody reading it
      dropped a whole pass, and a mesh draw that silently does nothing is the
      largest version of that failure this sequence has produced.
    - **The frame rate is measured on every page and reported against task 34's
      baseline**, with `.ai/tools/fps-check.sh 10 55` on the default page and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
      `Page::ALL`'s six, the `roados-fps` line parsed by hand — **`fps-check.sh`
      reads `seconds` then `floor` and nothing else, and runs the binary with no
      arguments (`.ai/tools/fps-check.sh`), so it cannot name a page**, which
      `.ai/tools/README.md` § *Frame-rate baseline* records as the reason task
      24.2's criterion 6 was amended rather than met by the script.
      **If task 34 has not landed when this task runs, the baseline is the band
      recorded in `.ai/tools/README.md` § *Frame-rate baseline* — 61.1 to
      63.9 across six pages — and the handoff says so in those words rather than
      claiming a comparison it did not make.** Every page above the floor of 55.
    - **The expected result is no measurable change, and the reason is stated in
      the handoff rather than left as a coincidence**: no page records a mesh
      command in this task (there is no loader — that is 38 — and no demo change),
      so the per-frame work added is one `if let Some(mesh)` per segment and one
      program link in `Renderer::new`. **What the measurement is for is the
      regression the added `if` could not cause and a future task's first mesh
      would** — and the per-sub-mesh cost of a real car (five draw calls, five
      uniform sets, five `use_program`s) is stated in the handoff as the number
      task 38's first capture will be measured against.

14. **The decision is written down where the next agent finds it.**
    `render.rs`'s module docs gain a `## Meshes` section: the vertex format is
    35's and the matrix maths is 36's, cited by name; **`Pass::Mesh` is a
    boundary and not a composited pass, and why**; the ordering contract
    (*record the map first, the chrome second*) stated as the answer to "the car
    is over the map"; the lighting model stated as flat Lambert with its four
    reasons; the premultiplication proof and the `chart.rs` defect it does **not**
    fix; the culling policy as 34 recorded it; and the uniform-count requirement
    on 36. `draw_pass`'s doc and `COMPOSITED_PASSES`'s are amended, not replaced.
    `doc/ui/IMPLEMENTATION_STATE.md` gains an entry recording the ten and five
    counts, the six pages' frame rates against 34's baseline, and the fact that
    **no page draws a mesh yet**.

## Acceptance Criteria

- [ ] **`DrawCommand` has ten variants and `ShaderKind` five.** Verified on the
      pre-change tree and quoted so the numbers mean something:
      `grep -cE '^    [A-Z][A-Za-z]+ \{$' ui/src/ui_core/src/paint.rs` returns
      **9** today and must return **10**; and
      `awk '/^pub enum ShaderKind/,/^}/' ui/src/ui_core/src/batch.rs | grep -cE '^    [A-Z][A-Za-z]+,?$'`
      returns **4** today and must return **5**. **The stronger check is the
      compiler**: a test in `batch.rs` with one `match` over all ten variants and
      one over all five `ShaderKind`s — a missing variant is a non-exhaustive
      match, which is a compile error rather than a silent pass
- [ ] **A mesh command is a singleton, and the two singleton kinds are
      distinguished.** `grep -n 'is_singleton' ui/src/ui_core/src/batch.rs` shows
      `matches!(self.shader, ShaderKind::Shadow | ShaderKind::Mesh)`, a test
      asserts `mesh().batch_key().is_singleton()` is true, and the existing
      control assertions (`!rect().is_singleton()`, `!image().is_singleton()`) are
      **still present and unweakened**. **Mutation evidence:** delete `|
      ShaderKind::Mesh` from the `matches!` and the suite fails
- [ ] **The batch key carries the colormap and the blend mode follows
      `opacity`.** A test asserts a mesh at `opacity: 1.0` batches `Opaque` and one
      at `0.999` batches `Transparent`, with `Image`'s own
      `an_opacity_a_hair_below_one_blends` named beside it as the precedent.
      **Mutation:** change `== 1.0` to `>= 0.999` and it fails
- [ ] **`Segment` has two boundary slots and the invariant holds.** A fixture of
      `n` alternating shadows and meshes yields `n + 1` segments in recorded
      order, and a test asserts that **no segment has both slots `Some`**.
      `grep -n 'mesh: None' ui/src/ui_core/src/batch.rs` shows the trailing
      segment's literal sets both. **Mutation:** make the sealed-run `match`
      classify a mesh batch as a shadow and the invariant test fails
- [ ] **`Pass` has four variants and the mesh is not one of the composited three.**
      `awk '/^enum Pass/,/^}/' ui/src/ui_core/src/render.rs | grep -cE '^    [A-Z]'`
      returns **3** today and must return **4**. The existing
      `the_composited_passes_are_drawn_solid_then_image_then_text` **keeps its name
      and its assertion** and gains a second one that
      `!COMPOSITED_PASSES.contains(&Pass::Mesh)`, with the doc sentence saying a
      mesh is drawn at its recorded position rather than a fixed one. **No test
      renamed, no assertion weakened**
- [ ] **`depth_state_for` is asserted for four passes × two blend modes.** Eight
      rows: the three 2D passes `{ test: false, writes: false }` for both blend
      modes, `Mesh` + `Opaque` `{ test: true, writes: true }`, `Mesh` +
      `Transparent` `{ test: true, writes: false }`. **Mutation evidence:** flip
      the `Mesh` + `Opaque` row's `writes` to `false` and the suite fails for that
      reason; a test that has never failed is a hypothesis
      (`developer.md` § Phase 3)
- [ ] **The mesh shader's attribute declarations match task 35's table.** A test
      asserts `MESH_VERTEX_SHADER_SRC` contains
      `layout(location = 0) in vec3 a_position`,
      `layout(location = 1) in vec3 a_normal` and `layout(location = 2) in vec2 a_uv`
      — the two whose component counts and offsets a silent mismatch would hide —
      and that the same source does **not** contain `u_resolution`, because the
      mesh shader does not work in window coordinates. **A grep over the whole file
      would not do this**: `grep -cE 'layout\(location = [012]\) in vec[23]'
      ui/src/ui_core/src/render.rs` already returns **11** from the four existing
      2D shaders, so the assertion has to be against the constant's own text
- [ ] **The shader has no shading term past Lambert.** `grep -in 'specular\|reflect(\|pow(\|textureLod\|normal_map\|tangent\|u_shadow' ` over the two mesh source constants in
      `render.rs` returns **nothing**, and the fragment source contains `max(dot(`
      and `u_ambient` and `u_light_dir`. This criterion is the lighting model, and
      it is checkable with one grep
- [ ] **Premultiplication survives, and it is proved without a display.**
      `mesh_fragment` is unit-tested over a grid of contract-honouring texels,
      tints and shades in `0.0..=1.0`, asserting `r <= a && g <= a && b <= a`
      componentwise on every result. **Mutation evidence:** change the CPU mirror's
      `base.rgb * shade` to `base.rgb` and to a straight-alpha `vec4(base.rgb,
      base.a) * tint`, and the invariant test fails for each. Separately, a test
      asserts `mesh_tint(Color::new(255, 0, 0, 128), 0.5)` is
      `[1.0, 0.0, 0.0, 128/255 * 0.5]` — **`rgb` untouched by `opacity`** — and
      **mutation: multiply `rgb` by `a` in `mesh_tint` and that test fails**, which
      is the specific thing `chart.rs`'s recorded defect would look like here
- [ ] **`sub_mesh_byte_offset` and `SubMeshRange::byte_offset` agree.** A test
      asserts both return the same value for a fixture of ranges, and both return
      **`4` for `first_index: 1`, not `1`**, and `Err` on an offset past `i32`
- [ ] **The six gallery pages are pixel-identical.** `Page::ALL`'s six names,
      captured **before and after** the change, release build, the commands of
      `.ai/tools/README.md` § *Capturing a window* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab` too, return
      black for a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, then `magick compare -metric AE before.png
      after.png null:` per page. **AE 0 outside `y ≥ 680`**, and every differing
      pixel inside the fps readout's band — which
      `.ai/tools/README.md` § *Capturing a window*
      records as the one thing two captures of an unchanged frame differ in (405
      pixels there, **AE 0 over y 80–680**). The rect-level half is
      `every_page_places_every_rect_where_the_gallery_placed_it` in the demo's
      suite. **The mechanism that makes this achievable is stated and true: no
      page records a mesh command in this task**, so the added pass is unreachable
      from the six pages and the criterion is demanding rather than merely met
- [ ] **The frame rate is measured and reported against task 34's baseline**, with
      the script's own line pasted rather than the number expected:
      `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for all six
      with the `roados-fps` line parsed — **`fps-check.sh` cannot name a page**,
      verified in `.ai/tools/fps-check.sh`. Every page above the
      floor of 55 and inside 34's band if 34 has landed, or inside the recorded
      **61.1–63.9** band in `.ai/tools/README.md` § *Frame-rate baseline* with the handoff saying which baseline it used.
      **The handoff also states the five-sub-mesh per-frame cost** — five draw
      calls, five uniform sets, five `use_program`s — as the number task 38's first
      capture is measured against
- [ ] **No GL error, read once after the first mesh draw**, pasted into the
      handoff with the instrument's code quoted. The three new
      constants are pinned against `glow::CULL_FACE`, `glow::CULL_FACE_MODE` and
      `glow::BACK`, and the six depth pins still hold
- [ ] **Nothing from tasks 38, 39 or 40 leaked in.** `git diff --stat` shows **no
      change under `ui/src/ui_demo/`**. `grep -rc 'std::fs\|File::open\|read_to_string' ui/src/ui_core/src/ | grep -v ':0'`
      is **empty** — it is empty today and must stay so **for this task**, since
      no `ui_core` API takes a file path and nothing here may begin to.
      **This criterion is superseded once task 38 lands and is not to be "fixed"
      then.** Task 38 § *The loader's entry point* adds
      `Mesh::load_from_path` to `ui_core` — the crate's first `std::fs` call, and
      deliberately so, since it follows the `load_picture` / `load_texture`
      precedent already in the tree. The check that stays true across both is the
      narrower one: **this task introduces no filesystem access.** A reviewer who
      sees this grep fail at task 38 should read task 38's own
      `Out of Scope` and the amendment recorded there, not treat it as a
       regression. No image decoder, no
       pointer or gesture handling, no rotation state. (`Drag` and `Rotation` are
       **not** usable as the check here: `InputEventKind::Drag` and
       `property.rs`'s `Transform::rotation` already exist and predate this
       sequence.) **Amended 2026-10-07, on task 38's landing: the entry point is
       `meshio::load_from_path` in `render/meshio.rs`, not `Mesh::load_from_path`
       as the paragraph above names it** — task 38's requirement 7 puts the two
       functions and four constants in `meshio` beside task 35's `Mesh` rather
       than on it, and the `rg -n 'std::fs|File::open' ui/src/` check task 38
       records returns exactly that one line. The criterion above is otherwise
       confirmed as written: superseded, not failed. **No new dependency**: `ui/Cargo.toml` and `ui/Cargo.lock` are
      unchanged, and the approved direct dependencies remain `sdl3 0.20`, `glow
      0.18` and `freetype-rs 0.38` — per `AGENTS.md`, a maths crate for a 4×4
      multiply that task 36 already supplies is a licence decision against GPLv3
      that nobody has asked for
- [ ] **The suite is green and the counts are reported.**
      `cargo test --all-features` with the per-binary counts pasted, and **no test
      deleted, renamed away or weakened**; `cargo fmt --check`,
      `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is **recorded**, not passed
- [ ] **Every new `unsafe` block has a SAFETY comment.** `grep -c 'SAFETY'
      ui/src/ui_core/src/render.rs` is **63** today and must be **at least** the
      number of new `unsafe` blocks on top of that — the count is stated as a
      floor, not a target, because the file's existing 63 is not this task's to
      explain. `rg -n 'transmute|static mut' ui/src/` returns nothing, and
      `rg -n 'unwrap\(\)|expect\(|panic!|unimplemented!|todo!'` over the new code
      returns nothing — an unknown `MeshId` and an out-of-range `SubMeshRange` are
      both **skips**, and requirement 12's paths all return `Err`
- [ ] **The decision is written down where the next agent finds it.** `render.rs`
      has the `## Meshes` module section with requirement 14's content;
      `BatchKey::is_singleton`, `Segment`, `COMPOSITED_PASSES`, `Pass`, `DrawCommand`
      and the enum's own "Colors are premultiplied alpha" doc are all consistent
      with the code beside them — **no doc comment in the changed files asserts the
      opposite of the code**, which is the defect
      `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice.
      And `doc/ui/IMPLEMENTATION_STATE.md` carries the ten/five counts, the pass
      order, the six pages' rates against 34's baseline, and the statement that
      **no page draws a mesh yet and the first pixels of a mesh are task 38's
      capture**

## Out of Scope

- **No model file format and no loader.** Task 38's. Nothing here reads a file,
  and `DrawCommand::Mesh` is therefore **recorded by no widget in this
  repository** — the honest state of this task, and the same one 35 and 34 record
  in their own § *Out of Scope*. The first pixels of a mesh in this codebase are
  task 38's first capture, not this task's
- **No asset pipeline, no offline render, no texture authoring, no mipmaps.**
  Task 39's. The colormap is loaded through the existing cache with the existing
  `GL_LINEAR` / `GL_CLAMP_TO_EDGE` / no-mipmap state, and **this task does not
  change it**
- **No drag-to-rotate, no pointer or gesture handling, no animation clock.**
  Gap `L4` is task 40's. The per-sub-mesh transform this task's command carries is
  what a rotation will rotate; the rotation itself is 40's
- **No shadow mapping.** Named and rejected in requirement 8's fourth reason, and
  the reason is 34's: a shadow map needs a depth-only FBO, the shadow FBO is
  `GL_R8` coverage with **no depth attachment** by 34's decision, and a 3D
  backdrop that needs one is **gap `L1`'s** work
- **No specular, no normal map, no PBR, no IBL, no second light, no fog, no tone
  mapping, no alpha-to-coverage.** Each is a real answer to a real problem this
  project does not have yet; naming them is what keeps a later task from
  re-deciding one of them by accident
- **No tangents, no vertex-format change.** `MeshVertex` is task 35's: three
  fields, stride 32, no padding. A normal map needs a tangent and a tangent is a
  stride change, so it belongs to whoever changes the format
- **No skinning, no morph targets, no LOD, no instancing, no frustum culling, no
  occlusion culling, no sort-by-depth on the CPU.** The depth buffer is the sort,
  and it is the reason task 34 came first in this sequence
- **No mesh in the demo.** `ui_demo` gains nothing: no page, no widget, no
  `--tab=` name. The six pages must be pixel-identical afterwards, and a demo that
  drew anything new would make that criterion unverifiable rather than merely
  demanding
- **No change to the depth policy, the resting state, `GL_LESS`, or the blend/depth
  rule.** All four are task 34's, and this task's requirement 8 **widens the
  signature** of the function that encodes the policy — it does not change a
  single value in it, and the three 2D rows are asserted identically before and
  after
- **No new face culling anywhere but the mesh pass**, and no `front_face` call at
  all — front face stays at GL's default `GL_CCW`, as 34 recorded. `polygon_quad`'s
  doc comment, already amended by 34 to say *the 2D passes* enable no culling,
  stays true
- **No colour attachment on the shadow FBO, no public backdrop API, no rect-scoped
  capture.** Gap `L1`'s, and a different decision: `ShadowTarget` is `GL_R8`
  coverage because a shadow's colour is one constant
- **No new dependency, and no `unsafe` beyond a GL call.** Per `AGENTS.md` the
  approved direct dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs
  0.38`; a maths crate for a 4×4 multiply, a GL-constants crate for three
  constants, or a GL loader this crate does not have is a licence decision against
  GPLv3 that nobody has asked for, and 36's `Mat4` is already the maths
- **No `Color` contract fix and no fix for the `quad_color` defect.** `chart.rs`
  records the measurements and this task cites them; changing what
  `Color::new(r, g, b, a)` means would change all nine existing variants' pixels
  and is its own task with its own before-and-after captures
- **No per-component recolour.** Track Mode *"recolours the car body by component
  temperature and tire grip"* (composite widget 4 and gap `L6b` in
  `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*). What this task
  delivers is **per-sub-mesh** tint, which on this model means **`body` against the
  four `wheel-*` meshes and nothing finer**. The model has **one material and one
  texture with regions distinguished by UV**, so per-sub-mesh tint *cannot* reach
  a region inside the body, and no amount of tint plumbing in this task would
  change that. Finer granularity needs one of three things, **none of them this
  task**: more sub-meshes in the model file (38's decision, and it changes the
  file format), a UV-region test in the fragment shader (a second sampler and a
  mask the asset does not have), or a baked recolour in the colormap (39's).
  And the honest limit is stated in the project's own record:
  `DEMO_APPLICATION.md` § *Could not verify* lists *"Track Mode's car-body
  recolouring rules"* as *"Named but not enumerated per component in anything
  read; treated here as `[B]` and **not a spec the demo should copy until
  verified**"* — so the rules are not known yet, and a task that invented them
  would be worse than one that does not have them