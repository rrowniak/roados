# TASK_UI_PRIM_53: Read glTF/GLB Directly — Delete the Project Format, the Loader, and the Offline Converter

## Goal

Replace the project-owned `ROADOSMF` `.roados` container and its loader
(`render/meshio.rs`) with a **direct glTF 2.0 / GLB read** through the `gltf`
crate, and **remove the offline format translation** — the `glb_to_model.py`
converter, its `model.json` manifest, the baked node translations, and the
committed `.roados` artefacts. Node placement becomes a **runtime** concern:
the loader keeps each node's transform and the demo applies it per part at draw
time, through the `Mat4` that `DrawCommand::Mesh` already carries.

## Context

### This reverses a recorded decision, and the record says it is safe to

`TASK_UI_PRIM_38.md` § *Why a project format and not GLB* chose a
project-specific binary format over `.gltf`/`.glb` for five reasons — no
dependency, no JSON, most of the format unused, baking is correct once, and the
target is embedded. Its closing paragraph says why reversing it is a **planned,
detectable** change rather than a silent misparse:

> *"A project format cannot be opened by any other tool. That is accepted for
> this sequence and reversible — the format's own header carries a magic and a
> version precisely so a future GLB reader, or a `v2` with tangents, is a
> detectable change rather than a silent misparse."*

This task is that future GLB reader. The reversal is deliberate and is the
operator's to confirm (requirement 1); `TASK_UI_PRIM_38.md` and
`TASK_UI_PRIM_39.md` gain a **dated note** recording it (requirement 8), because
a done task's file is its record and a reversal that overwrites the record
destroys the reasoning the next reader needs.

### What exists today, read off the tree

- **The format and its loader.** `ui/src/ui_core/src/render/meshio.rs` (1 166
  lines): the `ROADOSMF` byte table, `MeshError`, `load_from_bytes` /
  `load_from_path`, and 20 unit tests. `render.rs` declares `pub mod meshio;`,
  defines `RenderError::Mesh(String)` and `impl From<meshio::MeshError>`.
- **The offline converter.** `tools/asset-pipeline/glb_to_model.py` (566 lines)
  reads `sedan.glb`, **bakes each node's translation into the positions**
  (`bake_translation`), concatenates the five meshes, rebases indices and writes
  `sedan.roados`. `model.json` is its manifest; `check_assets.py` imports it;
  `README.md` documents it.
- **The committed artefacts.** `ui/src/ui_demo/assets/sedan.roados` (126 425 B),
  `ui/src/ui_core/tests/data/sedan.roados` (81 561 B, a fixture),
  `ui/src/ui_demo/assets/colormap.png`, and the `sedan.roados` row in
  `ui/src/ui_demo/assets/MANIFEST.sha256`.
- **The runtime consumer.** `ui/src/ui_demo/src/main.rs`'s `load_model`,
  `ASSET_MODEL_FILE`/`ASSET_MODEL_RELATIVE`, the `Model { mesh, ranges,
  texture }` struct, `car_commands`, and their tests.
- **The upstream.** `.asset-cache/kenney_car-kit/Models/GLB format/sedan.glb`
  (172 216 B), format-2 GLB, one embedded BIN buffer, five meshes
  (`body` 704 tris, four `wheel-*` 332 each), one primitive each, one material,
  and a node per mesh whose only transform is a **translation**
  (`body T = [0, 0.15, -0.025]`, the four wheels mirrored in x and z at
  `y = 0.3`, `|z| = 0.66`). The texture is a **relative URI**
  (`Textures/colormap.png`), not embedded.

### The dependency this needs, and what it costs

**`gltf` 1.4.1 is the crate.** [A] from crates.io/docs.rs this session:

| fact | value |
|---|---|
| version / released | **1.4.1** / 2024-05-10 |
| licence | **MIT OR Apache-2.0** |
| MSRV / edition | **1.61** / 2021 (the workspace floor is 1.85) |
| downloads | 10.3 M total |
| `unsafe` in lib | **none** |
| direct deps (always) | `gltf-json 1.4.1`, `serde_json 1.0` (raw_value), `byteorder 1.3`, `lazy_static`; transitively `serde`/`serde_derive` |
| `default-features = false` drops | `import` → `base64`, `image`, `urlencoding`; `utils`; `names` |

The parse/read path needs **no C, no `unsafe`, no `image`, no `base64`**. It is
compatible with `rust-version = "1.85"`. The unavoidable cost is `serde` +
`serde_json` (any JSON glTF parser has them), and the crate's known negative:
**it contains internal `unwrap`/`panic!` paths** (75 + 8 in 1.4.1) that a
malformed GLB can reach — see requirement 2's risk note. `cargo audit` shows no
advisory for `gltf` or `gltf-json`, and the transitive `base64` advisory applies
only below 0.5.2 (the crate uses 0.13) and is `import`-only besides.

**Alternatives, with their cost:** `easy-gltf` wraps `gltf` **with default
features** and adds `cgmath`, so it is strictly larger; `gltf-json` is the
JSON model only and would leave the GLB container and accessor decoding
hand-rolled; `russimp`/`meshopt` are C++ FFI (`unsafe`, `cc`, a C toolchain);
a hand-rolled GLB reader still needs a JSON parser and re-implements tested
accessor decoding. `gltf` is the crate a senior Rust developer would pick.

### Scope, measured against `developer.md` § *Scope check*

**Well over five files, four components — it fans out.** Components:
(A) the crate dependency + the new `render/gltf.rs` loader and its unit tests;
(B) the `ui_demo` consumer (`Model`, `load_model`, `car_commands`, tests);
(C) the deletions and edits across `render.rs`, `mesh.rs`, `check_assets.py`,
`MANIFEST.sha256`, the asset tree; (D) the documentation records. Per
`.ai/protocols/subagents.md` § *Implementation fan-out*, A–D are dispatched as
isolated sub-agents and integrated; A and B share the loader's API and must be
sequenced A → B.

## Requirements

1. **Operator gate first: the `gltf` dependency is approved, or the task stops.**
   `AGENTS.md` § Rust lists the approved **direct** dependencies as `sdl3 0.20`,
   `glow 0.18` and `freetype-rs 0.38`, and `developer.md` § *Dependencies* says
   *"No new dependency without operator approval."* This task **cannot start**
   until the operator approves `gltf`. The justification the operator is owed:
   it is the maintained, widely used (10 M downloads), MIT/Apache-2.0, no-C,
   no-`unsafe` glTF parser; the alternatives are larger (`easy-gltf`) or feel
   (`russimp`, `meshopt`) or re-implement tested code (hand-rolled).

   **The feature set is exact:** `gltf = { version = "1.4.1",
   default-features = false, features = ["utils", "names"] }`, in
   `ui/src/ui_core/Cargo.toml` only. `utils` is `Primitive::reader` and the
   `read_*` helpers; `names` is `Mesh::name()`/`Node::name()`. **`import` stays
   off**, so `base64`, `image` and `urlencoding` are not pulled, and this task
   reads a GLB **with one embedded buffer** and no external files. `AGENTS.md`'s
   approved-dependency list and `PRIMITIVES_ARCHITECTURE.md` § *Dependencies*
   (the sole owner of versions) are updated in the same change.
   `cargo audit` runs before the add.

2. **A new module `ui_core/src/render/gltf.rs` replaces `meshio.rs`.** It reads
   a GLB from bytes and returns the scene's mesh instances, each with its node
   transform. `pub mod meshio;` in `render.rs` becomes `pub mod gltf;`.

   ```rust
   /// One glTF mesh instanced by one node: its geometry and its node transform.
   pub struct GltfMesh {
       pub name: String,
       pub mesh: Mesh,        // task 35's Mesh, one SubMesh covering all indices
       pub transform: Mat4,   // the node's placement
   }

   pub fn load_from_bytes(bytes: &[u8]) -> Result<Vec<GltfMesh>, GltfError>;
   pub fn load_from_path(path: &Path) -> Result<Vec<GltfMesh>, GltfError>;
   ```

   - **Reading.** `gltf::Gltf::from_slice(bytes)` parses the GLB and keeps the
     BIN chunk; `Gltf::blob` is the single embedded buffer. Each scene node with
     a mesh becomes one `GltfMesh`: `node.transform()`, `node.mesh()`, and the
     mesh's one primitive read through `primitive.reader(|buffer| …)` with
     `read_positions()`, `read_normals()`, `read_tex_coords(0)`, `read_indices()`
     (`.into_u32()`). Vertices are built with `MeshVertex::new`, so the file's
     normals are renormalised here exactly as `meshio` did, and a zero-length
     normal becomes `[0.0, 0.0, 1.0]` rather than `NaN`.
   - **The subset, refused by name rather than approximated.** A `.gltf` (JSON)
     container, a non-GLB input, a second/external buffer, a mesh with more than
     one primitive, a primitive missing `POSITION`/`NORMAL`/`TEXCOORD_0`, a
     non-`TRIANGLES` mode, a 64-bit or `normalized` attribute, a node whose
     transform is a **matrix, a rotation, or a non-unit scale**, and an index
     that escapes its vertex array are each `GltfError::Unsupported(String)`
     naming what was found. **This is the same subset `bake_translation`
     enforced**, moved from conversion time to load time and no wider.
   - **`GltfError`** in the crate's idiom (`Clone, Debug, PartialEq, Eq`, a
     hand-written `Display` and `std::error::Error`): `Io(String)` (path only,
     as `MeshError::Io` was), `Parse(String)` (the crate's own message, so the
     parser stays the authority on JSON/GLB faults), `Unsupported(String)`.
   - **Risk, stated rather than hidden.** `gltf` 1.4.1 contains internal
     `unwrap`/`panic!` reachable from malformed input; `MeshError` had none. The
     asset this demo loads is **committed and trusted**, so the demo path does
     not exercise it, but `load_from_bytes` is public and a caller may hand it
     untrusted bytes. **The task records this as an accepted limit and does not
     claim `MeshError`'s no-panic guarantee**; the alternative is a hand-rolled
     GLB reader, which requirement 1's justification rejects.
   - `render.rs`'s module doc, `RenderError::Mesh`'s doc, and the
     `impl From<GltfError> for RenderError` replace every `meshio`/`ROADOSMF`
     reference. `RenderError::Mesh` keeps its name (a `.glb` is still a mesh
     source) and its `"mesh error: …"` `Display`.

3. **Node transforms are applied at draw time, and they are translation-only.**
   The loader converts `node.transform()` to a `Mat4`: `Transform::Decomposed`
   with an identity rotation and unit scale becomes
   `Mat4::identity().translated(translation)`; anything else is
   `Unsupported`. **This is correct and deliberate**: the renderer uploads
   `u_normal_matrix` as the constant `IDENTITY_MAT3`
   (`render.rs`'s mesh draw pass), which is only true while no node carries a
   rotation or a non-uniform scale. `Mat4` has `translated`/`rotated_x`/
   `rotated_y`/`scaled` but no inverse-transpose, and task 36's doc already puts
   *"if a later task rotates the car it needs a real normal matrix"* on that
   task. Supporting a rotating node is **out of scope**; the loader refuses it
   rather than shading it wrong.

4. **The demo records one command per node instance, through that node's own
   `mvp`.** In `ui/src/ui_demo/src/main.rs`:

   - `ASSET_MODEL_FILE = "sedan.glb"` and
     `ASSET_MODEL_RELATIVE = "src/ui_demo/assets/sedan.glb"`, with the doc
     comments rewritten: the asset is Kenney's GLB **copied byte-for-byte**,
     not a converted form, and the demo names `colormap.png` beside it because
     the draw command carries the texture and the GLB's own relative URI is not
     followed (requirement 6).
   - `Model { parts: Vec<ModelPart>, texture: TextureId }` where
     `ModelPart { mesh: MeshId, range: SubMeshRange, transform: Mat4 }`, one
     per `GltfMesh` the loader returned. `load_model` uploads each part's `Mesh`
     (`renderer.upload_mesh`), takes its single sub-mesh's `range()`, and keeps
     the node `transform`. All the non-fatal-search and `eprintln!` discipline
     of the current `load_model` is unchanged (the same `model_candidates`,
     `asset_candidates_from`, `join_paths`, `ROADOS_ASSET_DIR`).
   - `car_commands` composes `base = shift · projection · view` once (as
     `car_mvp` does today) and, for each part, records
     `painter.mesh(part.mesh, part.range, base.multiply(&part.transform),
     Color::new(255,255,255,255), 1.0, model.texture)`. **The five parts no
     longer share one matrix** — each wheel now carries its node translation, so
     the car assembles from geometry authored at the origin. The doc comment
     that says *"the model's own placement is task 39's baked node transforms"*
     is deleted; placement is this function's.

5. **The removed code is removed whole, and nothing is left asserting the
   opposite.** Delete:

   - `ui/src/ui_core/src/render/meshio.rs`, its `pub mod meshio;`,
     `RenderError::Mesh`'s `meshio` doc, and `impl From<meshio::MeshError>`.
   - `ui/src/ui_core/tests/model_file.rs` and
     `ui/src/ui_core/tests/data/sedan.roados` (requirement 7 replaces them).
   - `ui/src/ui_demo/assets/sedan.roados`.
   - `tools/asset-pipeline/glb_to_model.py` and `tools/asset-pipeline/model.json`.

   Edit: `render/mesh.rs`'s doc references to *"read from the model file (task
   38)"* and the `Mesh`/`SubMesh` docs that name `meshio`; `check_assets.py`'s
   `import glb_to_model`, its model-measured check, its `sedan.roados` stability
   row and its `sedan.roados` hash list entry; `tools/asset-pipeline/README.md`'s
   whole model/`ROADOSMF` section. **Nothing in `render.rs`, `mesh.rs` or the
   demo's doc comments may still describe a project format or a baked
   conversion after this task** — a doc asserting the code beside it was removed
   is the defect this repository records twice.

6. **The asset is the GLB itself, and the pipeline checks it, not a conversion.**
   Commit `ui/src/ui_demo/assets/sedan.glb` as a **byte-for-byte copy** of
   `.asset-cache/kenney_car-kit/Models/GLB format/sedan.glb` (172 216 B). Update
   `ui/src/ui_demo/assets/MANIFEST.sha256` (its `sedan.roados` row becomes the
   `sedan.glb` row with that file's sha256), and `check_assets.py` so it verifies
   the committed `sedan.glb` and `colormap.png` are **byte-identical to their
   cached upstream files** — the same byte-stability the `.roados` row enforced,
   now with nothing in between. `colormap.png` is still the committed texture
   and the demo still names it; the GLB's `Textures/colormap.png` URI and the
   material are **not** followed.

7. **Tests move with the code.** The `meshio` unit tests go with `meshio.rs`;
   their hostile-input coverage (`Truncated`, checked-multiply, `TooLarge`) has
   no equivalent because the parser is the crate's. The new coverage:

   - **Unit tests in `render/gltf.rs`** build a **minimal GLB in bytes** — the
     analogue of `meshio`'s `encode_file` — and assert: the magic/version and a
     two-node fixture parse to two `GltfMesh`; each node's translation becomes
     the expected `Mat4`; a mesh with no `POSITION`, a non-`TRIANGLES` mode, a
     rotation/scale/matrix node, and a `.gltf` JSON input each return the named
     `Unsupported`. **Every one runs with no display, no filesystem, no network
     and no wall clock.**
   - **An integration test `ui/src/ui_core/tests/gltf_file.rs`** includes the
     **committed asset** (not a second fixture — a duplicate binary is a second
     thing to drift) via `include_bytes!("../../../ui_demo/assets/sedan.glb")`,
     and asserts the shape task 39's fixture asserted, now against real GLB
     bytes: five parts named `body` and the four `wheel-*`, 2 032 triangles
     (704 + 4×332), every normal unit length, every index `< vertices.len()`,
     and — the load-bearing one — **the wheel transforms place the four wheels
     at four distinct points more than 0.1 m apart, each reaching outside the
     body's `[−0.75, 0, −1.3]..[0.75, 1.3, 1.25]` box in at least one axis**.
     With geometry authored at the origin, a loader that dropped or
     mis-converted a node transform puts all four wheels at the origin inside
     the body, and this is the test that sees it. **Mutation evidence in the
     handoff:** return `Mat4::identity()` for every node and watch this test
     fail for the right reason.
   - **The demo tests update**: `the_car_records_five_mesh_commands_each_with_a_range_and_one_mvp`
     (and its doc) becomes *one command per part / node, five commands, sharing
     tint, opacity and texture, with the four wheels' `mvp`s differing from the
     body's and from each other*; the hand-built `Model` fixture and the
     `model_candidates`/`ASSET_MODEL_RELATIVE` assertions change
     `sedan.roados` → `sedan.glb`. `the_car_records_…`'s former single-matrix
     assertion is the one that must be **deliberately inverted**, not deleted.

8. **The records are amended, not overwritten.** `TASK_UI_PRIM_38.md` and
   `TASK_UI_PRIM_39.md` (both in `doc/ui/done/`) gain a **dated note** naming
   this task, that the format/converter they specified is removed here, and the
   reason; `DEMO_APPLICATION.md` § *Library gaps* rows that cite the model
   format are checked and corrected if they now dangle; `PRIMITIVES_ARCHITECTURE.md`
   § *Dependencies* gains `gltf`; `AGENTS.md` § Rust's approved-dependency list
   gains it; and the sequence state file's *Left over* records the accepted
   no-panic limit and any acceptance criterion this task waives.

## Acceptance Criteria

- [ ] **The operator has approved `gltf`, and the reversal is recorded.**
      `ui_core/Cargo.toml` carries `gltf = { version = "1.4.1",
      default-features = false, features = ["utils", "names"] }`; `cargo tree`
      shows **no `base64`, `image` or `urlencoding`**; `cargo audit` is clean;
      `TASK_UI_PRIM_38.md` and `TASK_UI_PRIM_39.md` carry the dated note.
- [ ] **`meshio` and the converter are gone.** `render/meshio.rs`,
      `tools/asset-pipeline/glb_to_model.py`, `tools/asset-pipeline/model.json`,
      both `sedan.roados` files and `tests/model_file.rs` do not exist.
      `rg -n 'meshio|ROADOSMF|gi?lb_to_model|sedan\.roados'` over `ui/`,
      `tools/` and `doc/ui/` returns **only** the dated reversal notes in
      `doc/ui/done/` and this task — no live code or doc reference survives.
- [ ] **The loader's contract is tested on a hand-built GLB and the real
      asset.** `render/gltf.rs`'s unit tests cover the parse, the transform
      conversion and each `Unsupported` refusal with no filesystem; the
      integration test includes the committed `sedan.glb` and asserts counts,
      names, unit normals, index bounds and the four distinct wheel placements.
      **Mutation evidence in the handoff** that identity-ing the node transform
      fails `…wheels_are_outside…`. `cargo test --all-features` passes with the
      suite delta reported.
- [ ] **A change that alters the screen is seen.** The seven pages are captured
      before and after (`.ai/tools/README.md` § *Capturing a window* verbatim),
      the car is on the `data` page with the four wheels in the GLB's own
      places, and the six gallery pages are **pixel-identical**. A GL pipeline
      passes unit tests and still draws nothing, so
      the capture is the criterion, not a nicety.
- [ ] **The frame rate is measured, not asserted.** Every run of `ui_demo`
      is measured with `.ai/tools/fps-check.sh` and the numbers are in the
      handoff whether good or bad (`.ai/agents/developer.md` § Phase 3).
- [ ] **Verification is performed.** `cargo fmt --check`, `cargo build
      --all-targets --all-features`, `cargo clippy --all-targets --all-features
      -- -D warnings`, `cargo test --all-features`, `cargo doc --no-deps` and
      `cargo audit` — real output, pasted.

## Out of Scope

- **Rotation, scale or matrix node transforms.** Refused by name
  (requirement 3); supporting them needs a real `u_normal_matrix`, which is
  task 36's recorded follow-up and not this task's.
- **`.gltf` (JSON) files, external buffers, and embedded or URI textures.** A
  GLB with one embedded buffer only; the demo names `colormap.png`.
- **Skins, animations, cameras, morph targets, `KHR_*` extensions, multiple
  primitives per mesh, and materials.** Unread and refused where a refusal is
  cheap.
- **A no-panic guarantee on malformed GLB input.** The crate's, not this
  task's; recorded as an accepted limit (requirement 2).
- **Wheel spin or per-wheel animation.** Task 40's, and it composes with a
  `ModelPart` transform rather than replacing it.
- **Any change to tasks 34, 35, 36, 37, 40 or 43's code.** The vertex layout,
  the `Mesh`/`SubMesh`/`MeshId` types, the matrices, the draw command, the
  shader and the batchers are untouched.
