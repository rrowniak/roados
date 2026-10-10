# TASK_UI_PRIM_35: The Mesh Vertex Format and the Mesh GPU Buffers

## Goal

Give `ui_core` a third geometry kind: a triangle mesh with a position, a normal
and a texture coordinate per vertex, uploaded into GPU buffers and addressed by
named index ranges, so that a model with five parts can be drawn from one buffer
pair with one draw call per part.

## Context

This is the second of seven tasks (34–40) that add real-time 3D mesh rendering
for the Tesla-like demo. **It depends on task 34, the depth buffer**, and it
depends on it for one reason only: a mesh's parts are drawn in an order the CPU
does not control, and only a depth buffer makes that order invisible. Nothing
here needs the depth buffer to *work* — buffers upload and index ranges resolve
without it — but this task must not be implemented before 34, or the parts draw
in submission order and the first capture shows the car's inside.

The crate's geometry is quad-only, and the facts below are read off the source,
not inferred:

- **Three vertex types, one pass each.** `Vertex` (`pos`, `local`, `color`,
  `radius`, `size`; 11 `f32`, stride 44), `TextVertex` (`pos`, `uv`, `color`;
  stride 32) and `ImageVertex` (10 `f32`, stride 40). Each is `#[repr(C)]`,
  each is doc-commented field by field, each is laid out against a `const
  …_STRIDE` and `const …_OFFSET` pair that the attribute pointers in
  `Renderer::new` are written from. **The mesh format is a fourth entry in that
  table, not a new pattern.**
- **Four vertex arrays, one per pass.** `vao`, `text_vao` and `image_vao` are
  created in `Renderer::new` beside their own VBO and IBO, their attribute
  pointers are written inside one `unsafe` block with
  `// SAFETY: The GL context is current on this thread…`, and the block ends
  with `bind_vertex_array(None)`. Each VAO binds **its own** element array
  buffer while it is bound.
- **`quad_indices` is a quad function and its doc comment says why.** *"A quad is
  four vertices, so the indices only change when the vertex count per quad
  does."* That is true of the three quad passes and false of a mesh, so the
  function is neither reused nor extended here — a mesh's index buffer is
  whatever the model says it is.
- **Every submission is `draw_elements(GL_TRIANGLES, count, GL_UNSIGNED_INT, 0)`.**
  Three passes, and **all three pass `0` for the offset**, because each draws
  from the start of its own index buffer. A sub-mesh range is the first draw in
  the crate with a non-zero offset, and GL's offset parameter is **in bytes**.
- **`draw_arrays` is used once**, by the separable Gaussian in
  `render/blur.rs`, for the fullscreen quad. Nothing else is non-indexed.
- **The crate declares its own GL constants** (`GL_ARRAY_BUFFER: u32 = 0x8892`
  and its neighbours, each with a one-line doc comment giving the hex), and
  every constant a mesh needs already exists: `GL_ARRAY_BUFFER`,
  `GL_ELEMENT_ARRAY_BUFFER`, `GL_FLOAT`, `GL_STATIC_DRAW`, `GL_TRIANGLES`,
  `GL_UNSIGNED_INT`. **This task adds no new GL constant**, which is itself
  checkable.
- **Textures are `GL_LINEAR` min and mag, `GL_CLAMP_TO_EDGE`, no mipmaps.**
  `generate_mipmap` is never called and `MIN_FILTER` is `GL_LINEAR`. A mesh's
  texture is the texture cache's texture, so a minified car samples by
  `GL_LINEAR` like every other image in the crate; nothing about a mesh changes
  that, and it is recorded because task 39's asset pipeline will otherwise be
  asked to author mip levels nobody generates.
- **No pass enables face culling or a depth test** — `polygon_quad`'s doc
  comment states it as the reason winding is left to the caller. Winding
  therefore decides nothing in this task either, and a mesh's triangle order is
  the loader's to get right, not the renderer's.
- **Gap `L2` in `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*
  is *"No transform reaches the GPU"*: `Transform` is `Interpolate`-able,
  `DrawCommand` has no transform field and none of its nine variants carries
  one, and there is no matrix and no `u_model`. That gap is **closed by task
  36**, not by this one — this task supplies the buffers a `u_model` would be
  uploaded through, and nothing reaches a shader. Gap `L4`
  (`LongPress` and `Swipe` emitted and consumed by no widget) is closed by
  **task 40**, drag-to-rotate.
- **The car's model has five named meshes** — `body`, `wheel-front-left`,
  `wheel-front-right`, `wheel-back-left`, `wheel-back-right` — sharing **one
  material and one texture**. It also ships `NORMAL` per vertex. Those two
  facts are what decide requirement 3 (the sub-mesh design), requirement 4's
  normalisation and requirement 8 (no eviction).

**Why not extend the quad path.** A triangle is a quad with a duplicated corner
— `polygon_quad`'s own doc comment says so, and it says the duplicated corner is
*"the one way a triangle fits through it without a new vertex type, a new shader
or a change to the index buffer"*. That trick is exact for one filled triangle
in window space and worthless for a closed mesh: a car's body shell shares
vertices between triangles, every triangle needs its own transform, and the
depth order is the whole point. So this task adds a vertex type, and it does not
touch the quad machinery.

**Why `unsafe` is not an operator decision here.** `AGENTS.md` records that
`unsafe` was declined, and this task needs it anyway, because every GL call in
the crate is one. The reviewer's question is never *whether* a GL call is
`unsafe` and always whether the block introduces an invariant beyond *"the GL
context is current on this thread"*. Requirement 10 is that constraint, written
down.

## Requirements

1. **A `MeshVertex` in a new module `ui/src/ui_core/src/render/mesh.rs`**, with
   `pub mod mesh;` declared in `render.rs` beside `pub mod blur;`,
   `pub mod context;` and `pub mod target;`. Three fields, `#[repr(C)]`,
   `#[derive(Clone, Copy, Debug)]`, each doc-commented like `Vertex`'s:

   - `position: [f32; 3]` — object space, right-handed, metres.
   - `normal: [f32; 3]` — object space, **unit length**. The doc says who
     guarantees it (`MeshVertex::new`) and what a non-unit value does to a
     fragment (a `dot(n, l)` that is quietly wrong for every light, and a
     divide by zero for a zero-length one).
   - `uv: [f32; 2]` — into the material's texture, `(0,0)` at its bottom left.

   **No colour field.** One material and one texture cover all five meshes, so
   a per-vertex colour would be 16 bytes of constant data per vertex — a 50 %
   increase in vertex bandwidth to carry a value the shader reads once from a
   uniform. **No padding.** All three fields are `f32`, so `repr(C)` puts every
   field at a multiple of 4 and the struct's size is already 8 × 4 = **32
   bytes**; there is no alignment requirement to satisfy, and `GLES 3.1` imposes
   none on buffer data.
2. **The stride and the offsets, as constants in `render.rs` beside the other
   three tables**, each doc-commented with its arithmetic in the same style as
   `IMAGE_VERTEX_STRIDE`:

   - `MESH_VERTEX_STRIDE: i32 = 32` — eight `f32`, no padding.
   - `MESH_NORMAL_OFFSET: i32 = 12`.
   - `MESH_UV_OFFSET: i32 = 24`.

   And the attribute layout the mesh VAO records, which **task 37's shader must
   declare to match**:

   | location | attribute | components | offset |
   |---|---|---|---|
   | 0 | `a_position` | 3, `GL_FLOAT` | 0 |
   | 1 | `a_normal` | 3, `GL_FLOAT` | `MESH_NORMAL_OFFSET` |
   | 2 | `a_uv` | 2, `GL_FLOAT` | `MESH_UV_OFFSET` |

   Locations **0, 1 and 2 are reused from the other three VAOs deliberately**.
   It is safe because the enabled-array and pointer state belongs to the VAO,
   not to the context, and each VAO binds its own buffers — which is the same
   reason `text_vao` and `image_vao` already reuse 0, 1 and 2 with different
   meanings. This sentence is here because "it reuses locations" reads as a
   hazard, and the reason it is not belongs in the file rather than in a
   reviewer's head.
3. **Sub-meshes are index ranges into one interleaved buffer.** The whole model
   is **one vertex buffer and one index buffer**, and the five named meshes are
   `(first_index, index_count)` ranges into that shared index buffer. The
   alternative — a VBO and an IBO per sub-mesh — is rejected and the reason is
   written in the module doc: five uploads instead of one, and five attribute
   setups for a vertex format that is identical across all five, in exchange for
   the ability to run out of a buffer independently, which a car never does.
   The alternative the choice *buys* is the one that matters: because the five
   parts are ranges rather than buffers, each can be drawn with **its own
   transform** — which is what makes four independently rotating wheels
   possible, and that is tasks 36, 37 and 40.
4. **Three CPU-side types, all in `render/mesh.rs`, all free of GL** so they can
   be tested without a display:

   - `SubMesh { name: String, first_index: u32, index_count: u32 }` — `name` is
     `String`, not a newtype: it is **data read from a model file** (task 38),
     not a parameter of a function, so a newtype would wrap a field rather than
     a signature. `first_index` is an **index position**, not a byte offset.
   - `Mesh { vertices: Vec<MeshVertex>, indices: Vec<u32>, sub_meshes:
     Vec<SubMesh> }` — `indices` are **absolute vertex indices into
     `vertices`**, already offset by the loader; there is no `vertex_base` field
     to add at draw time, because rebasing would need either a per-draw integer
     uniform or a CPU rewrite of the whole index buffer, and a loader that
     offsets once is cheaper than either.
   - `MeshId` — a newtype over `u32`, `Copy`, `Debug`, `PartialEq`, `Eq`,
     `Hash`. It indexes `MeshStore`'s slots and appears in task 37's
     `DrawCommand::Mesh`. `pub(crate)` for the store, `pub` for the three data
     types, so the API surface is what tasks 37 and 38 need and nothing more.

   Plus two pure functions in the same module, both `Result`-returning and both
   testable without a display:

   - `MeshVertex::new(position, normal, uv)` — **normalises `normal`**, and a
     zero-length normal becomes `[0.0, 0.0, 1.0]` rather than `NaN`, because one
     `NaN` in a vertex buffer makes the triangle it belongs to disappear with
     no GL error. Both branches are unit-tested.
   - `sub_mesh_byte_offset(&SubMesh) -> Result<i32, RenderError>` — the
     conversion requirement 3 above turns on:
     `first_index × size_of::<u32>()`, checked, returning
     `RenderError::Gl` rather than wrapping.
5. **The mesh buffers live on `Renderer`, beside the other three, and the CPU
   geometry lives in a `MeshStore` that owns no GL object.** Three new fields on
   `Renderer`, created in `Renderer::new` in the same `unsafe` tuple block shape
   as `text_vao` and `image_vao`:

   - `mesh_vao: glow::VertexArray`, `mesh_vbo: glow::Buffer`,
     `mesh_ibo: glow::Buffer`
   - `mesh_vertex_capacity: usize`, `mesh_index_capacity: usize` — the two
     growth counters the other three passes carry.
   - `meshes: MeshStore` — the CPU-side slots.

   **The split is the answer to "a mesh is CPU data plus GPU state".** The GL
   objects sit where the crate's other GL objects sit, so there is one place
   that knows a context is current. The CPU geometry sits in a plain data
   structure with no GL in it, so the arithmetic that decides whether a mesh is
   valid — range containment, index bounds, byte offsets — is testable on a
   machine with no display, which is the only kind of test `AGENTS.md` permits.
6. **A fourth VAO, built exactly like the other three.** In `Renderer::new`,
   after the image VAO's block: `bind_vertex_array(mesh_vao)`, bind
   `mesh_vbo` to `GL_ARRAY_BUFFER` and `mesh_ibo` to
   `GL_ELEMENT_ARRAY_BUFFER`, enable 0/1/2 and write the three pointers from the
   constants in requirement 2, then `bind_vertex_array(None)`. **The element
   array binding is inside that block and must stay there** — it is VAO state,
   so an IBO bound while a *different* VAO is bound silently replaces that VAO's
   element binding and the solid, text or image pass reads indices out of the
   mesh's buffer. This is the one cross-VAO hazard in the task and it is the
   reason the requirement says "inside that block".
7. **`upload_mesh(&mut self, mesh: Mesh) -> Result<MeshId, RenderError>`**, which
   appends:

   - Validate **before any GL call**, so a malformed mesh costs nothing and
     names itself: `vertices` empty, `indices` empty, or `sub_meshes` empty is
     an error saying which. An upload that returns a handle whose draws are all
     no-ops is the failure shape of a buffer sized wrong — a batch that
     disappears with every GL call reporting success.
   - Validate every sub-mesh: `first_index + index_count <= indices.len()`,
     computed with `checked_add`, naming the sub-mesh in the error. A range past
     the end of the index buffer is what `draw_elements` reads, and without
     robust buffer access that is undefined geometry rather than an error.
   - Validate every index value against the slot's **base**:
     `vertex_base <= index && index < vertex_base + vertices.len()`. The loader
     produced these by offsetting, so this is the check that the offsetting
     agrees with the concatenation order — the one arithmetic in this task that
     nothing in the suite could otherwise catch.
   - Then `ensure_mesh_vertex_capacity` / `ensure_mesh_index_capacity`, then
     `buffer_sub_data` at the two bases with `GL_STATIC_DRAW`, then push the
     `MeshStore` slot and return its `MeshId`.
8. **`MeshStore` is `pub(crate)`, owns no GL, and evicts nothing.**
   `Renderer::upload_mesh` appends and every mesh lives until the renderer
   drops. **No cache, no eviction, no pin** — and the module doc says why, in
   the crate's own terms: the texture cache evicts because a 2048² atlas has a
   hard pixel budget and many small images contend for it, so its unit is a
   *shelf* and LRU is the only policy that fits; a mesh has no shelf, no budget
   and, in this project, one resident model drawn every frame, so an LRU over it
   is a data structure with one element and no event that could ever fire.
   Re-uploading an evicted car every frame is a guaranteed stall, not a saving.
   `Renderer` needs no drop either: `ShadowTarget::drop`'s own doc comment
   already records that *"no GL object is deleted here, and that is the same
   decision `crate::render` makes"* — the buffers live until `Context` tears the
   GL context down. `Drop` on a new mesh type that deletes three GL objects
   while its three siblings do not would be a distinction without a difference,
   and a reviewer should read that sentence before proposing one.
9. **`MeshStore::get(&self, id: MeshId) -> Option<&MeshRecord>` returns `None`
   for an unknown or freed id** rather than panicking, and no path in this task
   uses `unwrap`, `expect`, `panic!`, `unimplemented!` or `todo!`. An unknown id
   is a caller's bug and it is reported as a `None` that a caller can handle, per
   `developer.md`'s rule that library code returns `Result` and `Option`.
10. **Every new `unsafe` block carries a SAFETY comment** naming the two
    invariants the block relies on: *"The GL context is current on this thread"*
    and *"`<buffer>` is a valid buffer/vertex array created above"*, in the shape
    `Renderer::new`'s three existing blocks use. **No new `unsafe` surface
    beyond what a GL call requires** — no raw pointer arithmetic beyond the
    `from_raw_parts` that `submit_vertex_quads` already uses to view a
    `Vec<MeshVertex>` as bytes, no `transmute`, no `static mut`.
11. **Growth re-uploads from the retained CPU bytes.** `buffer_data_size` keeps
    the buffer *object* and its VAO's attribute pointers valid, which is why one
    shared mesh VAO survives growth at all; the re-upload itself is a single
    `buffer_data_u8_slice` of the concatenated arrays from `MeshStore`, because
    a mesh buffer is uploaded once and never edited, so there is no per-frame
    path to preserve. `INITIAL_CAPACITY` is not reused: a mesh counts
    **vertices and indices**, not quads, and the `VERTS_PER_QUAD` factor in
    `vertex_buffer_size` is the arithmetic that went wrong once already.

## Acceptance Criteria

- [ ] **The four VAOs exist.** `grep -c create_vertex_array ui/src/ui_core/src/render.rs`
      is **4** (was 3), and `grep -n "mesh_vao\|mesh_vbo\|mesh_ibo" render.rs`
      shows three fields on `Renderer`, three `create_*` calls in
      `Renderer::new`, and three bindings written **inside** the mesh VAO's
      `unsafe` block with the element array binding among them
- [ ] **The stride is asserted by a test, and so is every offset.**
      `size_of::<MeshVertex>() == MESH_VERTEX_STRIDE as usize`, plus
      `offset_of!(MeshVertex, normal) == MESH_NORMAL_OFFSET as usize` and
      `offset_of!(MeshVertex, uv) == MESH_UV_OFFSET as usize`. `offset_of!` is
      stable since Rust 1.77 and the crate's floor is 1.85, so this needs no
      dependency and no `unsafe`. The three existing offsets are **also**
      asserted with it — the existing `vertex_layout_matches_offsets` checks
      only the size, which is why the offsets were never checked
- [ ] **A test kills a wrong stride.** Change `MESH_VERTEX_STRIDE` to 36 and the
      suite fails; restore it and it passes. A test that has never failed is a
      hypothesis (`.ai/agents/developer.md` § Phase 3)
- [ ] **The five-mesh fixture is a pure-data test, with no display.**
      A fixture with five `SubMesh` ranges asserts they are contiguous,
      non-overlapping, cover `indices.len()` exactly, and that every index value
      is `< vertices.len()`
- [ ] **`sub_mesh_byte_offset` is unit-tested on the conversion GL actually
      wants** — `first_index: 1` is **4**, not 1 — and returns `Err` on an offset
      past `i32`
- [ ] **`MeshVertex::new` normalises, and a zero-length normal becomes
      `[0.0, 0.0, 1.0]`** rather than a `NaN`; both branches tested
- [ ] **`upload_mesh`'s three validations are tested with the GL side
      unreachable** — an empty `vertices`, an empty `sub_meshes`, an out-of-range
      sub-mesh, and an index value below the slot's base each return `Err`
      naming what was wrong. Every one of them is a pure predicate over the
      mesh, so no test in this task needs a window
- [ ] **`cargo test --all-features` is green, the per-binary counts are reported,
      and no test was deleted, renamed-away or weakened.** `cargo fmt --check`,
      `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is recorded, not passed
- [ ] **The six gallery pages are pixel-identical.** `Page::ALL`'s six names,
      captured **before and after** the change, release build,
      `setsid ./target/release/ui_demo > log 2>&1 &`, then the commands of
      `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
      method* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab` too, return
      black for a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`. `magick compare -metric AE before.png
      after.png null:` reports **0 outside the fps readout's band `y ≥ 680`**, and
      every differing pixel is inside it — which is the band
      `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided, and what it
      found* already records as the one thing two captures of an unchanged frame
      differ in (405 pixels there, **AE 0 over y 80–680**). The rect-level half is
      `every_page_places_every_rect_where_the_gallery_placed_it` in the demo's
      suite, which compares all six pages
- [ ] **The frame rate is measured and reported**, with the script's own line
      pasted rather than the number expected. `.ai/tools/fps-check.sh 10 55` on
      the default page, and per page
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` with the
      `roados-fps` line parsed — **`fps-check.sh` cannot name a page**, which
      `IMPLEMENTATION_STATE.md` § *Current position* records as the reason task
      24.2's criterion 6 was amended rather than met by the script. Every page
      is inside the recorded **61.1–63.9** band in
      `IMPLEMENTATION_STATE.md` § *The frame rate, measured* and above the floor
      of 55. **The expected result is no per-frame cost at all**: no page draws a
      mesh in this task, so the only new work is three GL object creations in
      `Renderer::new`
- [ ] **Nothing from tasks 36, 37 or 38 leaked in.** `git diff --stat` shows no
      change to `paint.rs`, `batch.rs` or `lib.rs`; `grep -c Mesh
      ui/src/ui_core/src/paint.rs` and `grep -c Mesh ui/src/ui_core/src/batch.rs`
      are both **0** — no `DrawCommand::Mesh`, no `ShaderKind::Mesh`, no
      `u_model`, no `mat4`, no `Transform` field. `ui/Cargo.toml` and
      `ui/Cargo.lock` are **unchanged**: no dependency was added
- [ ] **Every new `unsafe` block has a SAFETY comment**, and
      `grep -n "SAFETY" render.rs mesh.rs | wc -l` is at least the number of
      new `unsafe` blocks. `grep -rn "transmute\|static mut" render/ ` returns
      nothing
- [ ] **`mesh.rs` has a module doc that records the rejected alternatives in
      full** — per-sub-mesh buffers, a non-indexed mesh, and
      `quad_indices` reuse — because each is a dead end the next reader would
      otherwise walk. And `render/mesh.rs`'s tests pass **with no display, no
      network, no filesystem and no wall clock**, which is what makes the
      fixture above a legal test at all

## Out of Scope

- **No shader.** No vertex source, no fragment source, no program, no uniform
  location. Task 37 writes the mesh shader and it declares
  `layout(location = 0..2)` to match requirement 2's table. **The buffers this
  task uploads are uploaded and not drawn**: `cargo test` and a capture can both
  be green with a buffer nothing submits, and that is the expected state here
- **No `DrawCommand::Mesh` variant and no `ShaderKind::Mesh`.** `batch.rs` and
  `paint.rs` are not touched. `DrawCommand` has nine variants and this task
  leaves it with nine. The batching key question — whether a mesh command can
  merge with another mesh command — is task 37's, and it is unanswerable until
  the shader exists
- **No matrix math and no transform to the GPU.** No `Mat4`, no `u_model`, no
  `Transform` field on anything. **Gap `L2` is task 36's to close.** The
  per-sub-mesh transform this design exists to enable is why the sub-meshes are
  ranges, and the transform itself arrives later
- **No depth buffer, no depth test, no `GL_DEPTH_TEST`, no face culling.** Task
  34 owns the depth buffer; this task does not enable it, does not clear it and
  does not depend on it at run time. Face culling stays off because
  `polygon_quad`'s doc comment already states that no pass in the module enables
  it, and enabling it here would change winding requirements for every pass
- **No model file format and no loader.** No `.obj`, no `.gltf`, no parser, no
  filesystem read. Task 38 reads the file and produces a `Mesh`; this task
  defines what it has to produce, which is why `Mesh`, `SubMesh` and `MeshId`
  are `pub` here and the loader is not
- **No asset pipeline.** Task 39 owns texture authoring, and the `GL_LINEAR`
  / `GL_CLAMP_TO_EDGE` / no-mipmap state recorded in *Context* is a fact it must
  plan around, not one this task changes
- **No drag-to-rotate, no pointer or gesture handling.** Gap `L4` is task 40's.
  This task's per-sub-mesh ranges are what a rotation will rotate
- **No new dependency, and no `unsafe` beyond a GL call.** Per `AGENTS.md` the
  approved direct dependencies are `sdl3 0.20`, `glow 0.18` and
  `freetype-rs 0.38`; a GL loader, an OBJ parser or a maths crate for this
  task is a licence decision against GPLv3 that nobody has asked for
- **No mesh in the demo.** `ui_demo` gains nothing. The six pages must be
  pixel-identical afterwards, and a demo that drew a mesh would make that
  criterion unverifiable rather than merely demanding