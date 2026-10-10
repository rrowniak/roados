# TASK_UI_PRIM_38: A Model File Format, and the Loader That Reads It

## Goal

Give `ui_core` a mesh that comes from **a file**, in a format this project owns:
a small binary container for interleaved vertices, absolute indices and named
sub-mesh ranges, and a **safe-Rust loader** that parses it from a `&[u8]`,
produces task 35's `Mesh`, and **returns a specific error for every way a file
can be wrong** — no panic, no silently empty mesh, no oversized allocation.

## Context

This is the fifth of seven tasks (34–40) that add real-time 3D mesh rendering to
`ui_core`, in service of the Tesla-like demo in `DEMO_APPLICATION.md`. It
depends on **task 35 alone** — for the `Mesh`, `SubMesh` and `MeshId` types the
loader has to produce — and on nothing else:

| # | Subject | Depends on 34 for | Depends on 35 for |
|---|---|---|---|
| 34 | the depth buffer, and the depth policy | — | — |
| 35 | `MeshVertex` and the mesh GPU buffers | a place to put the projected `z`; parts drawn in an order the CPU does not control | — |
| 36 | matrix maths and the transform-to-GPU path (**gap `L2`'s matrix half**) | the `z` this task's geometry produces has somewhere to land | — |
| 37 | the mesh draw command, its shader and batching | the depth test that makes submission order irrelevant | the buffers and the ranges this task fills |
| 38 | **the model file format and the loader (this task)** | nothing | **`Mesh`, `SubMesh`, `MeshId`, the absolute index convention** |
| 39 | the asset pipeline (the offline tooling that produces these files) | nothing | nothing |
| 40 | drag-to-rotate (**closes gap `L4`**) | a rotated mesh to be depth-tested | four independently transformable wheel ranges |

**It closes no gap.** Gap `L2` in `DEMO_APPLICATION.md` § *Gaps this layout
exposes in `ui_core`* (*"No transform reaches the GPU"*) is closed by tasks 36 and
37; gap `L4` there (*"`LongPress` and `Swipe` are emitted and consumed by no
widget"*) by task 40; gap `L1` there (the colour capture and the public backdrop
API) by **no task in this sequence**. What this task does is make the mesh
pipeline's **input** exist, which is the precondition for every one of those.

### What exists today, read off the source and not inferred

- **There is no glTF/GLB crate available, and adding one needs the operator.**
  `ui/src/ui_core/Cargo.toml`'s `[dependencies]` block — the entire dependency
  list is `sdl3`
  (features `build-from-source`, `build-from-source-static`,
  `build-from-source-unix-console`, `image`), `glow` and `freetype-rs`
  (`bundled`). `AGENTS.md` records the approved **direct** dependencies as
  `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38` and the rule *"No new
  dependency without operator approval"*. So the alternative to writing a reader
  is not a crate lookup; it is a licence decision against GPLv3 that nobody has
  asked for, and the reader this task writes is a few hundred lines of bounds
  arithmetic.
- **Images load from a filesystem path only.** `TextureCache::load_from_file` in
  `ui/src/ui_core/src/texture.rs` takes a `&Path` and nothing else;
  `Renderer::load_texture` in `render.rs`
  (`pub fn load_texture(&mut self, path: &Path) -> Result<TextureHandle,
  RenderError>`) is its only caller. **There is no `from_bytes`, `from_memory`,
  `include_bytes!` or `from_io` anywhere in `ui/src/`** — the only `include!` in
  the crate is a *test* in `blur.rs`. So the crate's own precedent for loading
  anything is a path, and this task follows it: `load_from_path` is the entry
  point, and the byte-parsing core sits underneath it.
- **`TextureCache::load` takes a caller-supplied loader closure** —
  in `ui/src/ui_core/src/texture.rs`,
  `pub fn load<F>(&mut self, path: &Path, loader: &mut F) ->
  Result<TextureHandle, TextureError> where F: FnMut(&Path) -> Result<Pixels,
  TextureError>` — and its doc comment says what that seam is for: *"Returns
  whatever `loader` returns, unchanged, and nothing else: an image that failed to
  decode is not remembered, so a caller that fixes the file and asks again gets a
  second attempt rather than a cached failure."* **That is the same split this
  task's API has**: `load_from_bytes` is the whole of the logic and
  `load_from_path` is a wrapper over it. The precedent is not accidental — a
  closure seam is this crate's established way of keeping a decoder swappable.
- **`TextureError::Unreadable` deliberately collapses missing-file and
  unsupported-format** (in `ui/src/ui_core/src/texture.rs`): *"The message is the decoder's
  own; SDL_image names the format it could not read, and a caller that wants to
  tell a missing file from an unsupported one is given a library that reports both
  the same way."* **A format this project owns can do better and is required to.**
  A `ROADOSMF` file with the wrong version, a file truncated at 40 of 64 bytes,
  a vertex count of 4 000 000 000 in a 200-byte file, and a sub-mesh range past
  the end of the index buffer are **four different faults**, and the loader
  distinguishes all four by name. That is the honesty discipline of
  `TextureError::TooLarge` and `decode`'s own `pitch < row` refusal, both in
  `texture.rs` and both naming what was wrong rather than folding it
  into one arm.
- **Textures are `GL_RGBA8`, premultiplied once at load**
  (`Pixels::premultiply`, called from `decode` in `texture.rs`),
  `GL_LINEAR` min and mag, `GL_CLAMP_TO_EDGE`,
  **no mipmaps**; `ATLAS_MAX_IMAGE = 512` (`texture.rs`), so the colormap —
  512×512 — sits **exactly at** the threshold and takes the shared atlas.
  `TextureCache::pin` (`texture.rs`) exists. **None of this is this task's to
  change**: the colormap is loaded through `Renderer::load_texture` on 37's
  recording order, by the same call the demo already makes for `demo.png`.
- **The demo's one existing asset load** is
  `fn load_picture(renderer: &mut Renderer) -> Option<Picture>`
  (`ui/src/ui_demo/src/main.rs`); the search-path resolution is `asset_candidates`
  in the same file, walking
  **up** from `std::env::current_exe()` appending
  `ASSET_RELATIVE = "src/ui_demo/assets/demo.png"`, with
  `ROADOS_ASSET_DIR` (`ASSET_DIR_VAR`) as the override, and the split
  between the arithmetic and the filesystem is `asset_candidates_from`
  so the ordering is a function a test can call. **Failure is
  non-fatal and explicit** — the function's doc says *"A missing asset must not
  take the window down"*, it prints one `eprintln!` naming where it looked
  (`join_paths` joins the candidates into one line), and the
  stand-in `stand_in_picture` is an image of the asset's own shape with
  a label saying it is standing in. **Requirement 8 copies that discipline
  exactly**, and says where a mesh stand-in cannot.
- **`Font::from_path` (`ui/src/ui_core/src/font.rs`) is the font loader**, and there is no
  font-from-memory path either. **Precedent, not obligation**: a font is named by
  the platform's own lookup rules, whereas this format is one this project writes
  and can therefore accept a buffer for.

### The upstream model this format serves

**Kenney "Car Kit" 3.1** — `License.txt` reads *"License: (Creative Commons Zero,
CC0) http://creativecommons.org/publicdomain/zero/1.0/ — You can use this content
for personal, educational, and commercial purposes."* **[A]**, read from the
downloaded archive. 4 814 237 bytes from
`https://kenney.nl/media/pages/assets/car-kit/1a312ec241-1775131960/kenney_car-kit.zip`.
It ships `Models/GLB format/` with 50 `.glb` files plus
`Models/GLB format/Textures/colormap.png` (512×512 RGBA, 2 330 distinct colours
in flat clusters). **No `.blend` source is distributed**, so a `.glb` is the only
machine-readable form of a model this project can obtain.

`sedan.glb`: **2 032 triangles**; meshes `body` (704 tris), `wheel-front-left`,
`wheel-front-right`, `wheel-back-left`, `wheel-back-right` (332 each). **One
material (`colormap`), one texture**, referenced by **relative URI**
(`"uri": "Textures/colormap.png"`), not embedded bytes. Node translations move
the wheels: `wheel-front-right T = [-0.3, 0.3, 0.66]`, `body T = [0, 0.15, -0.025]`
and the rest by the same rule. **Coordinate system: Y-up** — body X = width
(±0.75), Y = height (0 → 1.15), Z = length (±1.28).

**Three of those facts decide the format, and each is a requirement rather than
context:**

1. **A GLB separates geometry from node placement, and a naive reader puts all
   four wheels at the origin inside the body.** So **node transforms are baked at
   conversion time** (requirement 3), which is the single most important thing
   this format does *not* store.
2. **Body and wheels share one material, so glass and lamps are not separable
   meshes — they are UV regions in `colormap.png`.** The glass colour
   `(56, 56, 61)` occupies one contiguous bounding box; white is another. **The
   colours and the box are stated here because a reviewer will ask what a finer
   granularity would even mean**, and the answer is that the boundary is in a PNG,
   not in the geometry.

   `TASK_UI_PRIM_37.md` § *Out of Scope* records the limit this must not
   contradict: what task 37 delivers is **per-sub-mesh** tint, which on this model
   means `body` against the four `wheel-*` meshes **and nothing finer**, and
   finer granularity needs more sub-meshes in the model file (**this task's
   decision, and it changes the file format**), a UV-region test in the fragment
   shader, or a baked recolour in the colormap (task 39's). **This format
   therefore carries a variable-length name table and a variable sub-mesh count,
   not five hard-coded slots** — that is what makes 37's first option reachable
   later without a format version bump, and it is the one place where the
   minimalism argument below has to bend.
3. **The texture is a separate file, by relative URI.** The mesh format carries
   **no texture reference at all** (requirement 2): 37's `DrawCommand::Mesh`
   already carries `texture: TextureId` on the **command**, precisely so one model
   can be drawn with a different colormap without re-uploading its geometry, and a
   URI in the file would be a second answer to a question the draw command
   already answers.

### Why a project format and not GLB

**The decision: a project-specific binary format, not `.gltf`, not `.glb`, not
`.obj`.** Five reasons, each of which would be sufficient and all of which hold:

- **No dependency.** A glTF reader is a crate; `AGENTS.md` gates new dependencies
  behind the operator, and the approved direct set is three crates, none of which
  parses anything. A hand-written reader for **this** subset needs no approval and
  no licence audit.
- **No JSON.** `.gltf` is JSON, so reading it is reading JSON: a parser, an error
  taxonomy for a language this project would otherwise not have, and a stream
  state machine. `.glb` embeds the JSON in a binary chunk, which removes the
  parser and **keeps the accessor indirection** — a buffer view, a byte offset, a
  stride, a component type and a count per attribute, per primitive, resolved
  through a table of tables. The whole of that resolves to *"here are 1 000
  interleaved vertices and here are the index triples"* at conversion time.
- **The GLB machinery this project has no use for is most of the format.**
  Scenes, nodes, skins, animations, cameras, samplers, materials, extensions,
  `extras`, and the joint matrix inverse-baking that a skinned mesh needs. Task
  37's shader samples one `sampler2D` per draw from a uniform, and 34/37's depth
  policy has no joint in it. Reading fields that can only ever be ignored is a
  validation surface with no consumer.
- **The runtime should not be the thing that decides geometry.** Baking node
  transforms and merging sub-meshes into one interleaved pair is work that is
  correct exactly once and wrong if it is wrong per frame. Doing it at conversion
  time is not an optimisation; it is the correctness argument for one shared index
  buffer with **absolute** indices, which task 35 already decided.
- **The target is an embedded aarch64 head unit**, and a binary file with fixed
  field sizes is a file the reader can bounds-check without a schema.

**What the minimalism argument costs, stated because it is real:** a project
format cannot be opened by any other tool. That is accepted for this sequence and
reversible — **the format's own header carries a magic and a version precisely so
that a future GLB reader, or a `v2` with tangents, is a detectable change rather
than a silent misparse.** Requirement 4 specifies that.

### `MeshStore` compatibility, and what "returns a `Mesh`" means

Task 35's split is the one this task must not blur: **`MeshStore` holds the CPU
geometry and the GPU objects live on `Renderer`.** So the loader produces a
**`Mesh`** — `vertices: Vec<MeshVertex>`, `indices: Vec<u32>`, `sub_meshes:
Vec<SubMesh>` — and **knows nothing about `Renderer`, `MeshStore` or GL**. It is
in `render/meshio.rs` (requirement 1), beside task 35's `Mesh` rather than inside
it, it is testable with no display, and `Renderer::upload_mesh` is the caller's
next step. **`load_from_bytes` returns `Mesh`; it does not return `MeshId` and it
does not upload anything**, and `grep -c unsafe render/meshio.rs` must be **0**
for the whole file — which is a stronger statement than 35's, because 35's file
does contain `unsafe` for the GL calls.

### Scope, measured against `developer.md` § *Scope check*

**Six files, three components, under both thresholds.** Files:
`render/meshio.rs` (new), `render.rs` (module declaration, `RenderError::Mesh`,
doc section, tests), `ui_demo/src/main.rs` (`load_model`, two constants, one
struct field, one signature change on a helper), `ui_core/tests/model_file.rs`
(new), the fixture it includes, `IMPLEMENTATION_STATE.md` (the record). **Three
components**: the byte format and its reader; the demo's search path and its
failure message; the tests and the record. **No fan-out is needed** —
`.ai/protocols/subagents.md` § *Implementation fan-out* does not apply, and if the
implementer finds themselves editing a **fourth code file**, that is a stop
condition rather than an expansion.

## Requirements

1. **A new module `ui/src/ui_core/src/render/meshio.rs`**, with `pub mod meshio;`
   declared in `render.rs`'s module block beside `pub mod mesh;` (35's),
   `pub mod matrix;` (36's), `pub mod blur;`, `pub mod context;` and
   `pub mod target;`. **Its module doc records the rejected alternatives in
   full** — GLB, glTF, OBJ, and a "just embed the vertex bytes in the binary"
   alternative — because each is a dead end the next reader would otherwise walk,
   and the reasons are § *Why a project format and not GLB* above.

2. **The format, exactly.** Little-endian throughout; **no padding anywhere**;
   every field is naturally aligned for its own width from an offset that is a
   multiple of that width. Header is **40 bytes**, then three variable-length
   blocks in this order.

   | offset | size | type | field | meaning |
   |---|---|---|---|---|
   | 0 | 8 | `[u8; 8]` | `magic` | `b"ROADOSMF"` |
   | 8 | 4 | `u32` | `version` | **1**. A reader that sees anything else refuses |
   | 12 | 4 | `u32` | `header_bytes` | **40**. Size of this header |
   | 16 | 4 | `u32` | `vertex_count` | vertices in the merged array |
   | 20 | 4 | `u32` | `index_count` | indices in the merged array, **a multiple of 3** |
   | 24 | 4 | `u32` | `sub_mesh_count` | named sub-meshes, **≥ 1** |
   | 28 | 4 | `u32` | `name_bytes` | total length of the name block |
   | 32 | 4 | `u32` | `flags` | **0** in v1; a non-zero value is refused |
   | 36 | 4 | `u32` | `reserved` | **0** in v1; a non-zero value is refused |
   | 40 | `vertex_count × 32` | bytes | `vertices` | interleaved, stride 32, **task 35's layout verbatim** |
   | 40 + V·32 | `index_count × 4` | `u32` | `indices` | **absolute** vertex indices into `vertices` |
   | 40 + V·32 + I·4 | `S × 8` | `u32` pair | `sub_meshes` | `(first_index, index_count)` per sub-mesh |
   | 40 + V·32 + I·4 + S·8 | `name_bytes` | UTF-8 | `names` | `sub_mesh_count` NUL-terminated strings, in the same order |

   `V`, `I` and `S` are `vertex_count`, `index_count` and `sub_mesh_count`. **The
   `header_bytes` field exists and the reader refuses any value but 40** — it is
   checked, never used to seek; see requirement 4.

   The vertex block is **`position [f32;3]`, `normal [f32;3]`, `uv [f32;2]`** —
   32 bytes, no padding, `MESH_VERTEX_STRIDE`, `MESH_NORMAL_OFFSET` = 12 and
   `MESH_UV_OFFSET` = 24 as task 35's requirement 2 declared them. **The reader
   decodes field by field, not by `from_ne_bytes` over a struct**: a `[f32; 8]`
   copy followed by a manual split is readable, bounds-checked, and needs no
   `unsafe`, and the bytes are built with `f32::from_le_bytes` on four-byte
   windows rather than by a transmute-shaped reinterpretation.

   **The name block is one `Vec<u8>` of NUL-terminated UTF-8 strings, not a
   directory of offsets** — an offsets table is 4 bytes per name to save a scan
   of a block that is read **once**, at load, by code that has the strings in
   hand. `sub_mesh_count` names, `name_bytes` bytes, each name **at least one
   byte** (the NUL), and **no interior NUL** in any name, which is what makes
   splitting on `0` unambiguous. Requirement 2's reason for this exception in the
   minimalism argument is the second paragraph of § *Why a project format and not
   GLB* item 2 above: 35's five names are **data**, and a format that could only
   carry exactly five would make 37's "more sub-meshes in the model file" option a
   version bump.

   **What the format deliberately does not carry, each with its reason:**

   - **Node transforms, hierarchies, or a scene graph.** Baked at conversion
     time. The reader is a reader of one flat mesh, and the "all four wheels at the
     origin" failure is a **converter's** bug and not a runtime one — which is the
     whole argument, because a converter is edited by a human with a diff and a
     runtime scene graph is a bug class that has to be re-tested every time a
     model changes.
   - **A texture reference, a material, or a URI.** `DrawCommand::Mesh` carries
     `texture: TextureId`; one model may be drawn with a different colormap.
   - **Tangents, colours, joints, weights, or a second UV set.** One UV set and
     one normal per vertex is 35's vertex format; a second UV set or a tangent is a
     **stride change**, and it belongs to whoever changes the format.
   - **A bounding box, an extent, or a pivot.** The demo places the car with a
     `Mat4`; a file that carries its own pivot is a second answer to a question
     the transform path answers.
   - **Compression.** An indexed 2 032-triangle car is on the order of 200 KB of
     vertices and 25 KB of indices — about a quarter of a megabyte, read once, at
     start-up, into memory that stays resident for the run. Compression is task
     39's decision, and it would turn requirement 6's bounds arithmetic into a
     decompressor's problem.
   - **A draw order, a material id, or a per-sub-mesh colour.** All of it is 37's
     tint on the command, and a file that carried it would be a second source for
     a decision the recording order makes. **`L6a` in
     `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* is the
     project's recorded reason to want exactly one source:** a value that must not
     change mid-frame is a **setter** and not a `Property`, and a file field the
     pipeline reads per frame is a `Property` in everything but name.
   - **Anything requiring JSON, base64, or a text encoding.**

3. **Node transforms are baked, and the sub-meshes are absolute ranges into one
   interleaved pair.** The loader's doc comment states the contract and the
   assumption it rests on:

   - The converter applies every node's translation (and rotation and scale, if a
     future model has them) to the **positions**, transforms the **normals** by the
     rotation part alone (a normal is a direction, so a translation must not move
     it and a scale must not lengthen it beyond what the reader normalises), and
     emits one vertex array with the five parts **concatenated in file order**.
   - Indices are **rebased once, at conversion**, so the file's indices are
     **absolute into the merged `vertices`** — which is exactly task 35's
     requirement 4's convention (`"there is no `vertex_base` field to add at draw
     time"`), and **the loader adds nothing**. A reader that added a base would
     double-count.
   - `sub_meshes[i].first_index` is an **index position**, not a byte offset, and
     `sub_meshes[i].index_count` is a **multiple of 3** because every submission
     in this crate is `draw_elements(GL_TRIANGLES, …)`
     (`TASK_UI_PRIM_35.md` § *Context*), which consumes three indices per
     triangle.
   - **The sub-mesh ranges must cover the index array exactly and in order** —
     `sub_meshes[0].first_index == 0`, each subsequent `first_index` equals the
     previous `first_index + previous index_count`, and the last range ends at
     `index_count`. The loader **enforces** this (requirement 6), because a gap
     means triangles no `draw_elements` call will ever draw and an overlap means
     some of them draw twice. This is the loader half of 35's requirement 7's
     range-containment check; **35 validates what a caller hands it and this
     validates what a file says**, and a fixture proves the two agree.
   - The five names, in the file, are `body`, `wheel-front-left`,
     `wheel-front-right`, `wheel-back-left`, `wheel-back-right` — and **nothing in
     the loader knows that**. The loader does not recognise a name, does not
     require a wheel to exist and does not order the parts.

4. **Versioning: the magic and the version exist to make a future change
   detectable, and a mismatch is an error, never a warning.** **The magic is
   `ROADOSMF`** — *ROADOS Mesh Format* — because a bare `ROADOS` would collide
   with a future container that is not a mesh, and because the eight bytes are
   what a reader checks first and what an operator sees in a hex dump when the
   demo says *"not a mesh"*.

   - **Wrong magic** → `MeshError::NotAMesh { found }`, carrying the **eight bytes
     as read**, and its `Display` renders them as sixteen hex digits. **An empty
     file is not `NotAMesh`**: check 1 of requirement 6 runs first and calls a
     file shorter than the header `Truncated`, so the message can never say
     *"the magic was not `ROADOSMF`"* about a zero-byte file. A PNG and a text
     file are both `NotAMesh`, and the hex says which — **which is the opposite of
     `TextureError::Unreadable`'s deliberate collapse, and the reason is that here
     the project knows the format and can therefore tell the cases apart.**
   - **`version != 1`** → `MeshError::Version { found, expected: 1 }`, whose
     `Display` says *"model file version 2, this build reads version 1"*.
     **Not** a "try anyway", and **not** an attempt to skip fields it does not
     understand. A format whose reader guesses is a format that draws a plausible
     wrong car.
   - **`header_bytes != 40`** → `MeshError::Malformed`, because a reader that
     trusts the header's own size field to skip to the payload can be pointed
     anywhere in the file. **The reader checks it and does not use it to seek.**
   - **`flags != 0` or `reserved != 0`** → `MeshError::Malformed`, naming which.
     These are the two words that let a **future v1** carry something (a LOD hint,
     a checksum) without a version bump, and refusing them is what keeps "v1 means
     exactly this layout" true.
   - **Trailing bytes after the name block** → `MeshError::Malformed` with the
     surplus count. A file that is longer than it says it is is either a
     concatenated pair or a different format, and both are worth refusing rather
     than ignoring: the alternative is a writer that appends a block the reader
     silently drops, and nobody notices until the block matters.

5. **`MeshError`, in the crate's error idiom, and one variant per repair.**
   `#[derive(Clone, Debug, PartialEq, Eq)]` like `TextureError`, a hand-written
   `Display` and `std::error::Error`, in `render/meshio.rs`. **Six variants, and
   the reason there are six is that six different mistakes deserve six different
   repairs** — which is the discipline `TextureError::TooLarge` states for an
   image (*"the image decoded but its size cannot be addressed"*) and the one
   `TextureError::Unreadable` deliberately declines for a decoder it does not
   control:

   ```rust
   pub enum MeshError {
       /// The path could not be opened or read.
       Io(String),
       /// The first eight bytes are not `ROADOSMF`. Carries them, and the
       /// `Display` renders them as hex, so a caller can tell a PNG from a
       /// zero-byte file from an HTML error page.
       NotAMesh {
           /// The eight bytes that were there instead.
           found: [u8; 8],
       },
       /// The version is not one this build reads.
       Version { found: u32, expected: u32 },
       /// The header says something the layout cannot be: a bad
       /// `header_bytes`, a non-zero `flags` or `reserved`, a zero count, an
       /// `index_count` that is not a multiple of three, a sub-mesh range that
       /// does not tile the index array, or bytes left over at the end.
       Malformed(String),
       /// A declared size does not fit the bytes that are actually there.
       Truncated { needed: u64, have: u64 },
       /// A payload this reader must build does not fit in memory on this
       /// target. Never a huge allocation.
       TooLarge { bytes: u64 },
   }
   ```

   - **`Truncated` and `TooLarge` are distinct, and the distinction is the
     point.** A 60-byte file declaring 100 vertices is **truncated** — re-copy it;
     a 200-byte file declaring 4 000 000 000 vertices is **oversized** — produce a
     smaller model. **`TooLarge` is reserved for one case only: a payload that
     fits the file byte-for-byte and still cannot be addressed on this target**,
     which in practice is a 32-bit build handed a mesh whose byte count exceeds
     `isize::MAX`. `TextureError::TooLarge` already draws the same line for an
     image (`texture.rs`), and this variant is the mesh reader's version
     of that check, **not a fourth name for a bad count** — a count that does not
     fit its own file is `Truncated`, full stop.
   - **`Malformed` carries a `String` naming the field**, because there are
     fourteen distinct header facts and one variant each would be fourteen arms of
     the same shape. `TextureError::TooLarge` is the precedent for a *structured*
     variant where the structure earns itself — it carries `width` and `height`
     because a caller places them; here **no caller places anything from a
     `Malformed`**, so the structure would be decoration. **The message is the
     specification, which makes "a specific error, not a generic one" a testable
     claim and not a doc sentence**: the refusal tests each assert that the
     `Malformed` message **contains the offending field's name**, so a future
     message that says only *"malformed mesh file"* fails the suite rather than
     shipping.
   - **`Io` is for `load_from_path` only.** `load_from_bytes` **cannot** produce
     it, and the module doc says so: a missing file and a malformed file are
     different answers here, which is the one thing `TextureError::Unreadable` gave
     up and this format need not.
   - **`impl From<MeshError> for RenderError`** mapping to a new
     `RenderError::Mesh(String)` variant, beside `RenderError::Texture`, with the
     same doc treatment — because the demo's `load_model` sits beside
     `upload_mesh` and must not have to flatten a `MeshError` into a string by
     hand. **`RenderError::Mesh` is a second answer to nothing**: it is the same
     relationship `RenderError::Texture` already has to `TextureError`, one layer
     up, and the alternative — folding the mesh fault into
     `RenderError::Texture` — would put *"the car model's magic was `89504E47`"*
     into a variant whose own doc says it carries an image's message.

6. **Validation, exact and in this order, with every count bounds-checked against
   the remaining byte length before anything is allocated.** This is a hard
   requirement, not a code-review preference: `.ai/agents/reviewer.md` § Phase 2
   lists *"panics on input from a bus or a file"* and *"anything whose behaviour
   under a hostile or absent input is unexamined"* as ordinary review ground, and a
   mesh file is untrusted input. **The order is the requirement; each check's
   reason is stated where the check is.**

   1. **`bytes.len() < 40` → `Truncated { needed: 40, have: len }`.** Before the
      magic is even read: a 12-byte file cannot carry a magic **and** a version,
      and a reader that reads the magic first reports `NotAMesh` for a file that
      was merely short. **Shortness is checked before content, because it is the
      only check that cannot itself be wrong.**
   2. **`magic != b"ROADOSMF"` → `NotAMesh { found }`.**
   3. **`version != 1` → `Version`; `header_bytes != 40`, `flags != 0`,
      `reserved != 0` → `Malformed` naming the field.**
   4. **`vertex_count`, `index_count`, `sub_mesh_count`, `name_bytes` each
      `usize::try_from`ed**, and a `None` → `Malformed` naming the field. On a
      32-bit target a `u32` count is not automatically a `usize`, and the
      conversion is the honest one per `developer.md` § *Code quality* rather than
      an `as` cast.
   5. **`vertex_count > 0`, `index_count > 0`, `sub_mesh_count > 0`, and
      `index_count % 3 == 0`** → else `Malformed` naming which. A zero count is
      task 35's requirement 7's first validation, and the loader is the place it
      first bites.
   6. **Every length is computed with `checked_mul` and `checked_add`** into a
      `u64`. **The arithmetic is `u64` and never `u32`, and that is the whole
      hostile-file defence**: `0x0800_0000 × 32` is `2^32`, which a `u32` multiply
      wraps to **exactly zero**, so a reader computing in `u32` concludes a
      40-byte file needs no payload and accepts it.
   7. **The three payload lengths are checked against `bytes.len()` before any
      allocation**: `vertices_bytes = vertex_count × 32`,
      `indices_bytes = index_count × 4`, `sub_mesh_bytes = sub_mesh_count × 8`,
      `header + all three + name_bytes <= bytes.len()`, each with `checked_add`,
      and the shortfall → `Truncated { needed, have: bytes.len() }` naming which
      block was short. **A reader that allocated `Vec::with_capacity(vertex_count)`
      before this check would let a 40-byte file ask for 128 GB**, and that is the
      single most important line in this task.
   8. **`name_bytes <= sub_mesh_count × 64`**, the latter by `checked_mul` → else
      `Malformed`. Every name in this model is `wheel-front-left`, 16 bytes; 64 is
      a **stated ceiling**, not a measurement, and the doc says so and says what
      raises it (requirement 4's version bump).
   9. **`name_bytes` is decoded as UTF-8** and a `str::from_utf8` failure →
      `Malformed` naming the offset. **Then the block is split on `0`**, and the
      number of pieces must equal `sub_mesh_count`, each piece **non-empty** (a
      name may not be the empty string). A mismatch → `Malformed` naming how many
      names were found against how many were declared.
   10. **Every index value is `< vertex_count`**, checked **as it is read**, with
        the offending value and its position in the error. This is task 35's
        requirement 7's third validation, run here against the file rather than
        against a caller.
   11. **Every `(first_index, index_count)` pair is validated in file order**:
        `first_index == running` (starting at 0), `index_count > 0`,
        `index_count % 3 == 0`, and `first_index + index_count <= index_count of
        the file` with `checked_add`. Any failure → `Malformed` naming the
        sub-mesh's index and what it said.
   12. **`running == index_count` at the end** — the ranges tile the index array
        exactly — else `Malformed`.
   13. **Only then** is anything allocated, and the allocation is
        `Vec::with_capacity` of a count that has already been proven to fit the
        file. **`Vec::with_capacity` is not a promise that it succeeds**: an
        allocation failure on an aarch64 with little RAM is an abort, not an
        `Err`, so the ceiling that matters is the one in check 7 — a count that
        does not exceed the file's own length — and **the format therefore cannot
        describe a mesh larger than the bytes it occupies**. That sentence is the
        whole of the "hostile file" argument. `TooLarge` is checked once, on the
        two `usize`-converted byte counts against `isize::MAX`.

7. **The loader is safe Rust, `&[u8]` first and `&Path` a thin wrapper.** Three
   `pub` items in `render/meshio.rs`, all `#[must_use]` on the ones that return a
   value, all doc-commented in the house voice, **no fourth**:

   - **`pub fn load_from_bytes(bytes: &[u8]) -> Result<Mesh, MeshError>`** —
     every check in requirement 6, no filesystem, no allocation before check 13,
     no `unsafe`, **no `unwrap`/`expect`/`panic!`/`unimplemented!`/`todo!`**. It
     builds `Vec<MeshVertex>` with `MeshVertex::new(position, normal, uv)` from
     task 35, so **normalisation happens here for the file's data too** and a
     zero-length normal in a corrupt file becomes `[0.0, 0.0, 1.0]` rather than a
     `NaN` that makes a triangle vanish with no GL error.
   - **`pub fn load_from_path(path: &Path) -> Result<Mesh, MeshError>`** — a
     wrapper, and **the only place `std::fs` appears in `ui_core`**. It reads the
     whole file with `std::fs::read` and calls `load_from_bytes`, mapping an
     `io::Error` to `MeshError::Io`. Its doc comment says in the first line that
     the buffer form is the one that exists, and names the crate-wide consequence
     honestly: **this is the first `ui_core` function that opens a file itself**,
     while every prior asset loader delegates to a library — `Font::from_path`
     hands the path to FreeType and `TextureCache::load_from_file` reaches
     SDL_image — so the precedent is the crate's and not this task's invention.
   - **The constants**, `pub` so a writer (task 39) and the tests can name them:
     `pub const MESH_MAGIC: [u8; 8] = *b"ROADOSMF";`, `pub const MESH_VERSION:
     u32 = 1;`, `pub const MESH_HEADER_BYTES: u32 = 40;`, and
     `pub const VERTEX_STRIDE_BYTES: u32 = 32;`. **A test asserts the last one
     against `MESH_VERTEX_STRIDE` from `render.rs`**, so the file's byte layout
     and task 35's `#[repr(C)]` struct cannot drift apart — the same reasoning
     35's requirement 1 gave for the stride and offset table, applied to the one
     place where the two are written twice.

   **The `&Path` wrapper is thin on purpose, and the doc comment says what that
   costs:** every test in this task calls `load_from_bytes`, because `AGENTS.md`
   permits no test that needs a filesystem. **That is the whole design constraint
   behind the API shape**, and the fixture test in the acceptance criteria is its
   consequence. **`load_from_path` is one function and one `?`, not a trait and not
   a generic over a reader** — `developer.md` § *Phase 2* (*"No abstraction before
   the second use"*), and `TextureCache::load`'s closure is the seam that already
   exists if a second caller ever needs one.

8. **`ui_demo` reaches the loader the way it reaches `load_picture`, and a missing
   mesh is non-fatal.** In `ui/src/ui_demo/src/main.rs`:

   - **`load_model(renderer: &mut Renderer) -> Option<Model>`**, beside
     `load_picture` and in its shape: build the candidate list from the
     **same** `asset_candidates_from` helper with a different relative path
     (`ASSET_MODEL_RELATIVE = "src/ui_demo/assets/sedan.roados"`) and the **same**
     `ROADOS_ASSET_DIR` override, pick the first that `is_file()`, and on no
     candidate print one `eprintln!` naming every path it looked at through the
     existing `join_paths` — **the same message shape, the same helper, the same
     override.** **This requires one signature change and it is required rather
     than optional:** `asset_candidates_from(exe: &Path, dir: Option<&OsStr>)`
     currently bakes `ASSET_RELATIVE` in, so it becomes
     `asset_candidates_from(exe: &Path, dir: Option<&OsStr>, relative: &str)`.
     `load_picture` passes `ASSET_RELATIVE` and **its existing tests keep their
     assertions** — a second search-path walker would be a second answer to one
     question, and that is the mechanism that lets two copies of a decision
     disagree without anything failing.
   - **`ASSET_MODEL_RELATIVE` and the colormap's own name are two new `const`s**
     beside `ASSET_RELATIVE`, each doc-commented in that constant's
     voice. The colormap is **`colormap.png`, beside the model in the same
     `assets/` directory**, and **the model file does not name it** — the format
     carries no texture reference (requirement 2), so the demo names it, which is
     the arrangement requirement 2's third reason argues for.
   - On a candidate found: `meshio::load_from_path(path)`, then
     `renderer.upload_mesh(mesh)`, each error printed with `path.display()` and
     the word *"standing in"*, exactly as `load_picture` does at its own two
     `eprintln!` sites.
   - **A mesh stand-in is harder than a picture's, and the decision is: there is
     none, and the demo says so.** `stand_in_picture` can hand the widget a
     transparent image of the asset's own shape because `Image` needs only a
     handle and a source; **`DrawCommand::Mesh` needs a `MeshId`, a
     `SubMeshRange`, a `Mat4` and a `TextureId`**, and there is no "no mesh" value
     any of those four can take that is not a lie — a zero-length range is skipped
     by 37's step 4, an unknown `MeshId` is skipped by `MeshStore::get`'s `None`,
     and a unit-cube stand-in is **a picture of a mesh pretending to be the car**.
     So **the demo draws no car and prints why**, and the reason is one line on
     stderr and one line in the page's own text: *"car model not loaded — see the
     log"*. **The alternative, and why it is rejected:** a wireframe box is a shape
     a defect looks like — which is `load_picture`'s own argument for a labelled
     stand-in, turned against the stand-in itself — and a car-shaped `Mesh` built
     in Rust is an asset pipeline in `ui_demo`, which is task 39's and not this
     task's.
   - **`Option<Model>` flows into `Demo::new` beside `picture`**, and the field is
     documented in the same voice `Picture`'s is: *"The uploaded model, or `None` —
     which means the page shows the line saying so, and never a shape pretending to
     be the car."*
   - **`Model { mesh: MeshId, ranges: Vec<SubMeshRange>, texture: TextureId }`** —
     the colormap is `renderer.load_texture` on the **same** search, and `ranges`
     carries **all five** sub-meshes **in file order**, with no field selecting
     one, because task 37's per-command `range` is what selects one and this
     struct must not pre-empt it.
   - **No page records a mesh command in this task.** `Model` is stored, the ranges
     are available, and **nothing draws them** — the mesh pass is 37's and the
     placement is 39's, and the first pixels of a car in this codebase are **39's**
     first capture. The six gallery pages must be pixel-identical afterwards, and a
     demo that drew a mesh would make that criterion unverifiable rather than
     merely demanding.

9. **The decisions go where the next agent finds them, and no doc comment is left
   asserting the opposite of the code beside it** — the defect
   `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice in
   this repository:

   - `render/meshio.rs`'s module doc gains a `## Model files` section with the
     **byte table from requirement 2 verbatim** (so a writer in task 39 reads one
     table, not two), the **six `MeshError` variants and the fault each names**,
     the **thirteen numbered checks of requirement 6 in order**, and the negative
     list — no scene graph, no texture URI, no transforms, no tangent, no LOD, no
     compression, no pivot — **each with its reason in one line.**
   - `TASK_UI_PRIM_37.md` § *Out of Scope*'s record that **finer recolour needs
     "more sub-meshes in the model file (38's decision, and it changes the file
     format)"** is **confirmed by this file's variable `sub_mesh_count` and name
     table**, and the confirmation is a dated note in that section of 37's file —
     amended in place, not replaced. **The record that the model has one material
     and one texture is not contradicted by anything here**: this format stores
     five named sub-meshes of that one material, and the granularity limit 37
     recorded is a property of the **asset**, not of the reader.
   - `ui/src/ui_demo/src/main.rs` carries **one paragraph of module doc** on the
     model and its search path, in the shape `ASSET_RELATIVE`'s own doc has: what
     `sedan.roados` is, that **it is not in this repository** (39 writes it), and
     that `ROADOS_ASSET_DIR` is how a caller supplies one. **The demo's help text
     is unchanged** — `--help` lists the six pages, and a mesh that is not drawn
     is not a page.
   - An entry in `doc/ui/IMPLEMENTATION_STATE.md` carrying: the magic and version;
     **the fact that no model file ships in `ui/src/ui_demo/assets/` in this task**
     (the asset is 39's, so the demo's search finds nothing and it prints its
     one-line reason — which is the *expected* state, and the reason the six pages
     are pixel-identical); **that a test fixture does ship, at
     `ui/src/ui_core/tests/data/sedan.roados`, and that it is a fixture rather than
     an asset**; the validation order; the test names and the suite delta; the six
     pages' frame rates; and the honest limit — **a loader is parsed and tested
     against a fixture of the asset's shape, and no car has ever been drawn**,
     which is stated rather than left to be discovered in 39.

## Acceptance Criteria

- [ ] **The magic and the version are asserted by tests, and the version is `1`.**
      `the_magic_is_roados_mfs_and_the_version_is_one` asserts
      `meshio::MESH_MAGIC == *b"ROADOSMF"`, `meshio::MESH_VERSION == 1` and
      `meshio::MESH_HEADER_BYTES == 40`, and a fixture built by the test's own
      encoder loads back. **Mutation evidence in the handoff:** change one byte of
      `MESH_MAGIC` and watch both this test and
      `a_file_whose_magic_is_wrong_is_not_a_mesh` fail for that reason; bump
      `MESH_VERSION` to 2 and watch
      `a_file_of_a_version_this_build_does_not_read_is_refused` fail. **A test
      that has never failed is a hypothesis** (`.ai/agents/developer.md` § Phase 3)
- [ ] **The file layout is asserted against the code, not against a comment.**
      A test asserts `meshio::VERTEX_STRIDE_BYTES == MESH_VERTEX_STRIDE as u32`
      and `MESH_NORMAL_OFFSET as u32 == 12` / `MESH_UV_OFFSET as u32 == 24` —
      **the three constants from task 35, from `render.rs`, in the same
      direction** — plus `size_of::<MeshVertex>() == 32`, and a test that builds a
      two-vertex fixture and asserts the loader's `MeshVertex` fields equal the
      `f32`s written at the two 32-byte strides, **including a negative test** that
      a fixture whose second vertex starts at offset 31 rather than 32 loads a
      *different* position. The file's layout is proved by the reader rather than
      by agreement between two documents
- [ ] **The refusal paths are tested by name, one fault each, and no panic path
      exists.** Each of these returns the named error, each is a separate
      `#[test]` so a failure names the check, and **every `Malformed` row
      additionally asserts its message contains the offending field's name**:

      | test | fixture | error |
      |---|---|---|
      | `a_file_shorter_than_the_header_is_truncated` | 39 bytes, and separately 0 bytes | `Truncated { needed: 40, have }` — **both sizes**, because "empty" is the case a reader is likeliest to mishandle into `NotAMesh` |
      | `a_file_whose_magic_is_wrong_is_not_a_mesh` | `"ROADOSM\x00"` | `NotAMesh { found }`, and the `found` bytes asserted individually |
      | `a_file_of_a_version_this_build_does_not_read_is_refused` | `version: 2` | `Version { found: 2, expected: 1 }` |
      | `a_file_whose_header_bytes_field_is_not_forty_is_refused` | `header_bytes: 64` | `Malformed` naming `header_bytes` |
      | `a_file_with_a_set_flag_bit_is_refused` | `flags: 1` | `Malformed` naming `flags` |
      | `a_file_whose_reserved_word_is_not_zero_is_refused` | `reserved: 1` | `Malformed` naming `reserved` |
      | `a_file_with_no_vertices_is_refused` | `vertex_count: 0` | `Malformed` naming `vertex_count` |
      | `a_file_whose_index_count_is_not_a_multiple_of_three_is_refused` | `index_count: 7` | `Malformed` naming `index_count` |
      | `a_file_whose_payload_is_shorter_than_its_counts_is_truncated` | 40-byte header, no payload | `Truncated` naming the block, `needed` asserted |
      | `a_file_whose_index_reaches_past_the_vertex_array_is_refused` | `index: 9` with 3 vertices | `Malformed` naming the value |
      | `a_file_whose_sub_mesh_ranges_do_not_tile_the_index_array_is_refused` | two ranges with a gap | `Malformed` naming the sub-mesh |
      | `a_file_with_bytes_left_over_at_the_end_is_refused` | one trailing byte | `Malformed` naming the surplus |
      | `a_file_whose_names_are_not_utf8_is_refused` | `0xFF` in the name block | `Malformed` naming the offset |
      | `a_file_with_fewer_names_than_sub_meshes_is_refused` | 2 names, 3 sub-meshes | `Malformed` naming both counts |
      | `a_file_with_a_name_longer_than_the_declared_ceiling_is_refused` | one 200-byte name | `Malformed` naming `name_bytes` |

      `rg -n 'unwrap\(\)|expect\(|panic!|unimplemented!|todo!|unreachable!'` over
      `render/meshio.rs` outside its `mod tests` returns **nothing**, and
      `grep -c unsafe render/meshio.rs` is **0**. **Every one of these runs with
      no display, no network, no filesystem and no wall clock**, which is the only
      kind `AGENTS.md` permits and the reason the whole task is testable at all
- [ ] **Truncated-file and hostile-count tests prove no oversized allocation.**
      Two named tests carry that burden, and **both assert a number rather than a
      variant**, which is what makes them killable:

      - `a_hostile_vertex_count_cannot_allocate_more_than_the_file_holds` writes a
        **40-byte** header claiming `vertex_count: 0x0800_0000` — 134 217 728 —
        and asserts
        `Err(Truncated { needed: 4_294_967_336, have: 40 })`. **`needed` is the
        assertion that matters.** `0x0800_0000 × 32` is `2^32`, which a `u32`
        multiply wraps to **exactly zero**, so a reader that computes the payload
        length in `u32` sees a 40-byte file whose vertices occupy no bytes and
        **accepts it** — followed by a `with_capacity(134_217_728)` and a 4 GB
        allocation from a 40-byte input. **Mutation evidence in the handoff:**
        compute the three payload lengths as `u32` and watch this test fail on the
        `needed` value while **the fourteen refusal tests stay green** — a test
        asserting only "it was an error" would survive that mutation, and one
        asserting only the variant would survive a reader that returned
        `Truncated` with a wrapped `needed`.
      - `an_index_count_that_overflows_a_thirty_two_bit_multiply_is_truncated_not_accepted`
        is the same trap on the index block: `index_count: 0x4000_0000`, whose
        `u32` product is `2^32` wrapping to zero. **Two rows rather than one
        because they are two multiplications in the code**, and requirement 6's
        check is written per block on purpose.

      The structural half is
      **`vec_with_capacity_is_not_called_before_the_byte_length_is_proven`**,
      checkable by reading: `rg -n 'with_capacity' render/meshio.rs` returns lines
      that are **all** after the truncation check, and the handoff quotes that
      output beside the `checked_mul` lines. **This criterion is the one a reviewer
      should break first**, because a reader that allocates before it measures is
      correct on every other fixture in this file and a denial of service on a
      file it has not seen
- [ ] **The real asset's *shape* is exercised without a filesystem, and the public
      API is what makes that possible.** The constraint is `AGENTS.md`'s — **no
      test that needs a filesystem** — and the resolution is requirement 7's API
      shape: **`load_from_bytes` is the whole of the loader and `load_from_path`
      is a wrapper over it**, so a test holds the bytes in memory. Three named
      tests:

      - `a_mesh_of_the_sedans_shape_loads_and_its_wheels_are_outside_its_body` —
        a **committed test fixture** at
        `ui/src/ui_core/tests/data/sedan.roados`, built by this task's
        **test-side** encoder (the private one from the round-trip criterion
        below) to the asset's **verified counts**: 2 032 triangles, five sub-meshes
        named `body`, `wheel-front-left`, `wheel-front-right`, `wheel-back-left`,
        `wheel-back-right` with **704 and 332 triangles** respectively, and the
        four wheel placements offset by the node translations the GLB carries
        (`wheel-front-right` at `T = [-0.3, 0.3, 0.66]`, `body` at
        `T = [0, 0.15, -0.025]`). It is loaded with
        **`include_bytes!("data/sedan.roados")`** from
        `ui/src/ui_core/tests/model_file.rs` — an **integration** test, because
        `AGENTS.md` puts public-API tests in `<crate>/tests/` and because a binary
        fixture is not something a unit test should reach sideways to find. It
        asserts the counts, the five names **in file order**, the ranges **tiling
        the index array**, every index `< vertices.len()`, every normal unit length
        to within `1e-5`, and — the property that makes the baked-transform
        requirement load-bearing rather than asserted — **the four wheel
        sub-meshes' centroids are four distinct points, pairwise more than 0.1 m
        apart, each outside the body's bounding box in at least one axis.** A
        reader that added a base to the indices, or a converter that forgot to bake
        a node translation, puts all four wheels at the origin inside the body, and
        **this is the test that sees it.** The handoff records the four centroids
        it measured and **the fixture's size on disk**, so an unexpected binary in
        the tree is announced rather than discovered.
      - `the_fixtures_five_wheel_sub_meshes_are_not_one_sub_mesh` — the
        **anti-vacuity check**, and it exists because
        **four documents agreeing is one belief, counted four times** is exactly
        this trap: the five names are asserted in this file, in
        task 37's, and in the fixture, so their agreement carries no information
        unless something checks the fixture is **not** a single sub-mesh. The test
        asserts `sub_meshes.len() == 5`, that the four wheel ranges are **pairwise
        disjoint**, and that the body range's triangle count differs from each
        wheel's — so the fixture cannot pass by being one part five times.
      - `the_public_api_offers_the_byte_form_the_tests_use` — a test that calls
        `meshio::load_from_bytes` with a `Vec<u8>`'s `as_slice()`. **This is the
        criterion's real content**: it is what stops a later refactor from folding
        `load_from_bytes` into `load_from_path` and quietly making every test in
        this task illegal, since **a test of a helper cannot see a call site
        that stopped using it** runs the other way here — a test
        that needs a filesystem is a test that cannot run at all.

      **What these three tests do and do not prove, stated here rather than left
      to a reviewer.** They prove the **reader** against a file of the real asset's
      exact shape and counts. **They prove nothing about the converter**, which
      does not exist in this task and is 39's: the fixture is built by the test's
      own encoder from figures read out of `sedan.glb`, so a converter that
      mis-bakes a transform or mis-counts a triangle is invisible here and will be
      caught by **39** writing the same assertions against the real `sedan.roados`
      — **which is why the fixture lives under `ui/src/ui_core/tests/data/` and
      not in the demo's `assets/`: it is a test input, and putting it where the
      runtime looks would make this task's demo find it, which is a car on screen
      and therefore 39's capture.** **The handoff must say this in those words**,
      because a green fixture test reads like a green asset pipeline and is not
      one — and a fixture that supplies the asset is **a second
      copy of a decision 39 will make differently**, and the honest handling is to
      say so in the file rather than count the green as evidence about the
      pipeline. **The handoff also names the `include_bytes!` as the crate's
      first** — the survey recorded none anywhere in `ui/src` — and states why it
      is right here when `Font::from_path`'s precedent is the opposite: a font is
      named by the platform's lookup rules, whereas this file is one the project
      writes and can therefore embed for its own tests **without changing what the
      loader is**, and requirement 8 keeps the path wrapper for the demo
- [ ] **A round-trip fixture proves the loader and an encoder agree**, even though
      the production encoder is task 39's:
      `a_fixture_the_test_built_loads_back_to_what_it_wrote` writes the header,
      two vertices at stride 32, six indices, two sub-mesh ranges and two names
      with **the test's own encoder** — **a private helper in `mod tests`, not
      production code, so this task ships no writer** — and asserts every field
      back, including the exact `f32` bit patterns at the two strides. **This is
      what makes the byte table a specification rather than a description**, and
      the two-vertex fixture is what the truncation and hostile-count tests
      mutate. **And the same encoder is what builds the `sedan.roados` test
      fixture**, which is why that fixture's counts are a statement about this
      format and not about any converter
- [ ] **The six gallery pages are pixel-identical.** `Page::ALL`'s six names
      (`pads`, `text`, `input`, `controls`, `data`, `overlays`), captured **before
      and after** the change, release build, the commands of
      `.ai/tools/README.md` § *Capturing a window* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab` too, return
      black for a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, then `magick compare -metric AE before.png
      after.png null:` per page. **AE 0 outside `y ≥ 680`**, and every differing
      pixel inside that band — the band `.ai/tools/README.md` § *Capturing a window* records as the one thing
      two captures of an unchanged frame differ in (405 pixels there, **AE 0 over
      y 80–680**). The rect-level half is
      `every_page_places_every_rect_where_the_gallery_placed_it` in the demo's
      suite, which compares all six pages. **The mechanism that makes this
      achievable is stated and true: no page records a mesh command in this task**
      — `Model` is stored and its ranges are available, and nothing draws them —
      so the criterion is demanding rather than merely met
- [ ] **The frame rate is measured and reported**, with the **script's own line
      pasted** rather than the number expected: `.ai/tools/fps-check.sh 10 55` on
      the default page, and per page `ROADOS_RUN_SECONDS=10
      ./target/release/ui_demo --tab=<page>` with the `roados-fps` line parsed —
      **`fps-check.sh` cannot name a page** (it takes `seconds` then `floor` and
      nothing else), which `.ai/tools/README.md` § *Frame-rate baseline* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. Every page is inside the recorded **61.1–63.9** band in
      `.ai/tools/README.md` § *Frame-rate baseline* and above the floor
      of 55, and inside task 37's band if 37 has landed, with the handoff saying
      which baseline it used. **The expected result is no measurable change**, and
      the reason is stated in the handoff rather than left as a coincidence: this
      task adds no per-frame work at all — the loader runs **once**, at start-up,
      before the first frame, and no page draws its result
- [ ] **Nothing from tasks 39 or 40 leaked in, and nothing from 34–37 moved.**
      `git diff --stat` shows **no change to `paint.rs`, `batch.rs`, `render.rs`'s
      shader constants, `render/mesh.rs`, `render/matrix.rs`, `render/context.rs`,
      `property.rs` or `animation.rs`** beyond the `pub mod meshio;` declaration,
      `RenderError::Mesh`, and the doc sections the change requires; `DrawCommand`
      and `ShaderKind` keep the counts 37 gave them; **no page records a mesh
      command** (`rg -c 'painter\.mesh|\.mesh\(' ui/src/ui_demo/src/main.rs` is
      **0**); and **no asset is committed** — `git status --short
      ui/src/ui_demo/assets/` names no new file, because the `.roados` asset is
      task 39's. **`ui/Cargo.toml`, `ui/Cargo.lock` and
      `ui/src/ui_demo/Cargo.toml` are unchanged**: no glTF crate, no OBJ crate, no
      maths crate, and the approved direct dependencies remain `sdl3 0.20`,
      `glow 0.18` and `freetype-rs 0.38` per `AGENTS.md`
- [ ] **`load_from_path` is the crate's only place that opens a file, and that is
      recorded as the change of fact it is.** `rg -n 'std::fs|File::open' ui/src/`
      returns **exactly one line**, `render/meshio.rs`’s `load_from_path`, where
      it returns **empty** today — `Font::from_path` hands the path to FreeType
      rather than opening it, `TextureCache::load_from_file` reaches SDL_image, and
      neither uses `std::fs`. **The handoff states plainly that this is a new
      property of `ui_core`**: no function here opened a file itself before this
      one, and the crate's other two asset loaders delegate to a library.
      **`TASK_UI_PRIM_37.md`'s acceptance criterion asserting
      `rg -rc 'std::fs\|File::open\|read_to_string' ui/src/ui_core/src/` is empty
      is no longer satisfiable after this task** — it was written to keep task 37
      from reading a file, and this task is the one that legitimately does. **That
      is why it is named here to be amended in 37's file, dated and attributed,
      before the sequence reaches 37**, rather than discovered as a failed
      criterion by whoever runs 37 next. The amendment's wording follows
      `DEMO_APPLICATION.md` § *Corrections to the second gap table*'s form: the
      criterion becomes **"`std::fs` appears only in `render/meshio.rs`’s
      `load_from_path`, and no `ui_core` code draws a mesh from a path"** — still
      checkable, and still ruling out what 37's criterion was for
- [ ] **The suite is green and the counts are reported.** `cargo test
      --all-features` with the per-binary counts pasted, and **no test deleted,
      renamed away or weakened**; `cargo fmt --check`, `cargo build --all-targets
      --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`
      and `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is **recorded**, not passed
- [ ] **The public API is documented, `#[must_use]`d, and minimal.**
      `cargo doc --no-deps` is warning-free; every `pub` item in `render/meshio.rs`
      has a doc comment carrying what a caller cannot derive — the byte order, the
      baked-transform contract, the absolute-index convention, the six error
      variants and which repair each one asks for, and the `&Path` wrapper's reason
      for existing; every value-returning function is `#[must_use]`;
      `load_from_bytes` has a **doc test** that builds a 40-byte fixture in memory
      and parses it, which is a test that needs no filesystem. **`MeshError` is a
      concrete type, not `Box<dyn Error>`**, per `reviewer.md` § *Error handling*,
      and there is **no fourth `pub` item** in the module beyond the two functions
      and the four constants requirement 7 lists
- [ ] **The decision is written down where the next agent finds it, and no doc
      comment asserts the opposite of the code beside it** — the defect
      `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice.
      `render/meshio.rs` has the `## Model files` section with requirement 9's
      content; `TASK_UI_PRIM_37.md` carries the dated note confirming its
      "more sub-meshes in the model file (38's decision)" record;
      `IMPLEMENTATION_STATE.md` carries requirement 9's entry **including the
      honest limit that no car has ever been drawn**; and the handoff says
      explicitly that **the first pixels of a mesh in this codebase are task 39's
      capture, not this task's**

## Out of Scope

- **No asset *production* tooling.** The converter that reads `sedan.glb`, bakes
  the node transforms, merges the five meshes into one interleaved pair, rebases
  the indices and writes `sedan.roados` is **task 39's**, and this task writes no
  writer in production code — the only encoder in the tree is a private one in
  `mod tests`. **No `.blend`, no `.obj`, no glTF reader, no Blender, no
  command-line tool, no build script, and no file is committed to
  `ui/src/ui_demo/assets/`.** The format is specified so 39 can write it; 39
  writing it is not this task
- **No GLB, glTF, JSON or OBJ parsing at runtime**, and no text format of any
  kind. The five reasons are in § *Why a project format and not GLB* and the
  module doc repeats them
- **No compression, no LOD, no instancing, no octree, no streaming, no progressive
  load, no async I/O.** An indexed 2 032-triangle car is on the order of 200 KB of
  vertices and 25 KB of indices; compression would turn requirement 6's bounds
  arithmetic into a decompressor's problem, and LOD needs a second mesh format or
  a second file and is a performance question this project has not asked
- **No multiple models, no model registry, no named-model lookup, no hot reload.**
  One car, loaded once, before the first frame. `Model` holds one `MeshId` and its
  ranges, and a scene graph of models is a different task with a different
  lifecycle question
- **No animation, no skinning, no morph targets, no joint hierarchy.** The format
  carries no node transforms at all, so none of these has anywhere to live; **and
  the decision that this is so is the decision to reverse first** if a skinned model
  ever lands — at which point the version field exists
- **No normals recomputed, no welding, no tangent generation, no smoothing, no
  re-indexing, no degenerate-triangle removal, no winding normalisation.** All of
  it is 39's, at conversion time, precisely because all of it is once-per-asset
  work. **The one exception is normalisation, and it is 35's function, not a new
  one:** `MeshVertex::new` is what the loader calls, so the file's normals are
  normalised through the same code path 35's tests already cover
- **No mesh in the demo — no page draws one, and no `DrawCommand::Mesh` is recorded
  anywhere.** `ui_demo` gains a loader, an upload, a stored `Model`, a search path
  and one line of text saying why there is no car. The six pages must be
  pixel-identical afterwards, and a demo that drew a mesh would make that criterion
  unverifiable rather than merely demanding. **The first pixels of a car are task
  39's capture**
- **No `TextureCache` change, no `Renderer::load_texture` change, no atlas or
  sampler change, and no mipmaps.** The colormap is an ordinary image through the
  ordinary path with `GL_LINEAR` / `GL_CLAMP_TO_EDGE` / no-mipmap state, and 512×512
  sits exactly at `ATLAS_MAX_IMAGE`, so it shares the atlas — **which this task
  neither arranges nor needs to**
- **No `DrawCommand::Mesh` field, no `ShaderKind::Mesh` change, no shader, no
  uniform, no GL constant, and no change to the depth policy.** All are 37's and
  34's. `render/mesh.rs` is not edited
- **No camera, no placement, no `Mat4` construction, no rotation, no gesture or
  pointer handling.** Gap `L4` is task 40's, and where the car sits is 39's
- **No `Material`, no per-sub-mesh colour, no per-vertex colour, no vertex-format
  change.** Task 37 records that this model has one material and one texture with
  regions distinguished by UV, so per-sub-mesh tint reaches `body` against the four
  wheels **and nothing finer** — **this task does not contradict that and does not
  try to**: the format's variable sub-mesh count is what would let a future
  converter split the glass out, and the decision to do so is 39's, with
  `DEMO_APPLICATION.md` § *Could not verify* recording that Track Mode's recolouring
  rules are *"named but not enumerated per component in anything read"* and are
  *"not a spec the demo should copy until verified"*
- **No new dependency, and no `unsafe`.** Per `AGENTS.md` the approved direct
  dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; a glTF reader,
  an OBJ parser or a byte-order crate is a licence decision against GPLv3 that
  nobody has asked for, and the crate's own `u32::from_le_bytes` covers byte order.
  **This task adds zero `unsafe` blocks** — stronger than 34's and 35's, because it
  makes zero GL calls and zero pointer conversions
- **No `Color` contract fix, no `quad_color` defect fix, and no `chart.rs`
  measurement.** `TASK_UI_PRIM_37.md` § *Out of Scope* records those; this task
  touches neither
