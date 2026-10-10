# TASK_UI_PRIM_39: The Asset Pipeline — Two Pinned Upstreams In, Committed Bytes Out

## Goal

Give the repository an **offline** path from two pinned third-party sources to
the committed bytes the demo will load: a converter that turns Kenney's
`sedan.glb` into exactly the model file task 38 defines, and a baker that turns
33 Lucide SVGs into 66 baked PNGs — one per theme, at the exact on-screen size —
with the licences recorded, the upstream hashes pinned, and a guard that makes
it impossible for a Tesla mark to reach the repository.

**Nothing here runs inside `ui_demo`.** There is no `build.rs`, no runtime
generation, no code path from these files into a frame. **Zero Rust files
change.** The deliverable is a directory of committed bytes, the tooling that
reproduces them, and — for one run after this task lands — the disappearance of
the line *"car model not loaded"* that task 38 put there on purpose.

## Context

This is the sixth of seven tasks (34–40) in the sequence that bring real-time 3D
mesh rendering to this project, and **the only one of the seven that adds no Rust
at all**. Tasks 34–37 and task 40 change `ui_core`; task 38 changes `ui_core` and
`ui_demo`. This one changes a scripts directory, one `.gitignore` line, and a set
of image and geometry files.

### What it is, and what it is not — and the narrowing of task 34's row

Task 34's sequence table gives task 39 the subject **"the asset pipeline (offline
render and packaging)"**. **This task takes the "packaging" half and refuses the
"offline render" half**, for a reason that is a fact about the upstream rather
than a preference:

- **Kenney's Car Kit ships no `.blend` source.** The archive is `Models/FBX
  format/` (50 `.fbx`), `Models/GLB format/` (50 `.glb`) and
  `Models/GLB format/Textures/colormap.png` — a 512×512 fully opaque palette —
  and nothing else. There is no modeler file, so **re-lighting, re-texturing or
  re-authoring the car is impossible without a modeller**, and this task will not
  install one.
- **An offline render of the car would not be an asset this pipeline loads
  anyway.** The engine draws the car from geometry and a colormap (tasks 35, 37,
  38); a pre-rendered PNG of a car is a *decoration*, and
  `DEMO_APPLICATION.md` § *Open questions* item 5 is explicit that *"An SVG of
  a generic car, or a flat PNG, will not carry it"*. Shipping one would answer
  the wrong question.

So "offline render" becomes "offline **convert**", and the rasteriser prototype
discussed below is **not** promoted into this task. `DEMO_APPLICATION.md`
§ *Open questions* items 4 and 5 stay answered-but-unratified by this task: the
pipeline ships a generic Kenney sedan and the operator's decision on which
vehicle the demo shows is still owed.

### What exists today, read off the tree

- **There is no build tooling in this repository, and that is a fact not an
  omission.** The only non-source directories at the root are `cmake/` (build
  configuration: `sdl-options.cmake`, `aarch64-toolchain.cmake`) and `.ai/`
  (AI engineering configuration). `.ai/tools/` holds `fps-check.sh` and
  `topy.py` — an agent's instruments, documented by `.ai/tools/README.md` with
  what each **may not** be used for, and explicitly *"not an authority"*.
  **`AGENTS.md` forbids modifying `.ai/` "unless the task explicitly concerns the
  AI engineering system"**, and a build tool that produces committed assets is
  not that. So the pipeline goes in a new top-level `tools/`, beside `cmake/`,
  and not in `.ai/tools/`.
- **The convention for committed assets does exist, and is one file.**
  `ui/src/ui_demo/assets/demo.png` (320×192, `const ASSET_SIZE`), resolved by
  three constants in `ui/src/ui_demo/src/main.rs` — `ASSET_FILE = "demo.png"`,
  `ASSET_RELATIVE = "src/ui_demo/assets/demo.png"` and the
  `ROADOS_ASSET_DIR` override. **Nothing names a directory under `assets/` other
  than that one file's parent**, which is the mechanism that makes this task's
  "the six pages are pixel-identical" criterion achievable by construction
  rather than by luck.
- **`.gitignore` has three entries** — `*.swp`, `*.context.*`, `/ui/target/`,
  and `/tmp/`. A download cache needs one more, and the addition is named in
  requirement 3 rather than left to whoever notices.

### The dependency question, answered explicitly

`AGENTS.md` approves exactly three direct dependencies — `sdl3 0.20`, `glow 0.18`,
`freetype-rs 0.38` — and `.ai/agents/developer.md` § *Dependencies* says *"No new
dependency without operator approval."*

**This task adds no workspace dependency, so it does not breach that rule.** A
build-time tool under `tools/` is not a member of the `ui` workspace, appears in
no `Cargo.toml`, is never linked into `ui_core` or `ui_demo`, and is absent from
`ui/Cargo.lock`. `ui/Cargo.toml` and `ui/Cargo.lock` are **unchanged** by this
task, and that is an acceptance criterion rather than a hope.

**Two things are nevertheless the operator's call, and this file does not
pretend otherwise:**

1. **The host packages this task relies on** — `python3` with `numpy` and
   Pillow, plus the `resvg` binary — are dependencies of the *build*, not of the
   *workspace*. They are not approved by `AGENTS.md` because `AGENTS.md` governs
   the workspace. **That a build-time dependency escapes the dependency rule is a
   consequence of where the rule is written, not a precedent anyone has
   established.** It is stated here so the operator can overrule it rather than
   discover it.
2. **`resvg` 0.48.1 is already installed on this host by `cargo install`** — it
   is at `~/.cargo/bin/resvg`, with its crate in the shared registry
   (`resvg-0.48.1.crate`). So the Rust source of a build tool is *already* on
   this machine, outside any manifest. **Recorded, not endorsed.**

### Host tooling, verified 2026-10-05, and one trap

| tool | state | note |
|---|---|---|
| `python3` | **3.14.4** | |
| `PIL` (Pillow) | **12.1.1** | the PNG writer; § *Determinism* is about this |
| `numpy` | **2.3.5** | the float work and the GLB arrays |
| `resvg` | **0.48.1 at `~/.cargo/bin/resvg`** | **not on `PATH`** in a plain shell |
| `inkscape` | `/usr/bin/inkscape` | present; **not used** — see *Out of Scope* |
| `magick`, `convert` | `/usr/bin/magick`, `/usr/bin/convert` | the comparison and contact-sheet tools |
| `curl` | `/usr/bin/curl` | the fetcher |
| `blender` | **absent** (`apt` offers 5.0.1) | and it is not needed, per the no-`.blend` fact |
| `trimesh` | **absent** from the host, downloads fine from PyPI | and it is not needed either |

**The `resvg` path is a trap worth naming.** `~/.cargo/bin` is not on this
shell's `PATH`, so `command -v resvg` returns nothing while the binary works
perfectly when invoked by absolute path. A pipeline that shells out to a bare
`resvg` therefore fails on a host where it is installed and will send the next
agent looking for a missing dependency. Requirement 6 handles it.

### Upstream one — Kenney "Car Kit" 3.1, CC0

- **URL**:
  `https://kenney.nl/media/pages/assets/car-kit/1a312ec241-1775131960/kenney_car-kit.zip`
- **Size**: 4,814,237 B. **SHA-256**:
  `fac7dacac5c7874348cf19729af3ef205f3d366493edaf0a827d93f4fdf3d0c4`
  (computed on this host on 2026-10-05; re-verify before recording — see
  requirement 3, which is a stop condition, not a warning)
- **Licence**, `License.txt` verbatim: *"License: (Creative Commons Zero, CC0)
  http://creativecommons.org/publicdomain/zero/1.0/ — You can use this content
  for personal, educational, and commercial purposes."* The file adds *"Support
  by crediting 'Kenney' or 'www.kenney.nl' (this is not a requirement)"*.
  **So: no obligation, credit optional** — recorded in requirement 4 anyway,
  because an uncredited CC0 asset in a GPLv3 repository is indistinguishable
  from an unlicensed one to the next reader.
- **Contents**: 50 `.glb` under `Models/GLB format/`, 50 `.fbx` under
  `Models/FBX format/`, one `Models/GLB format/Textures/colormap.png` at
  **512×512, RGBA, alpha 255 on every pixel**. **No `.blend`, no `.obj`, no
  `.fbx` with materials worth re-authoring.**

### What `sedan.glb` actually contains, read out of the file

Everything in this subsection was read from the archive, not inferred, and a
developer must re-derive it rather than trust it:

- **Five meshes, one primitive each, 2,032 triangles total.** `body` 704,
  `wheel-front-left` / `-right` / `wheel-back-left` / `-right` 332 each.
  Vertices: `body` 1,072, each wheel 528 — **3,184 vertices and 6,096 indices**
  after the merge.
- **`meshes[]` order is `wheel-front-right`, `body`, `wheel-back-left`,
  `wheel-back-right`, `wheel-front-left`** — *not* a logical order. Requirement 8
  makes the merge order an explicit manifest list for exactly this reason.
- **Attributes per primitive: `POSITION` `VEC3` f32, `NORMAL` `VEC3` f32,
  `TANGENT` `VEC4` f32, `TEXCOORD_0` `VEC2` f32.** **`TANGENT` is present and is
  dropped.** Task 35's `MeshVertex` is three fields at stride 32 and task 37's
  shader declares `layout(location = 0..2)` with no tangent, so carrying one
  would be a stride change and nothing else. It is recorded here because
  "the GLB has a tangent" is exactly the kind of thing a future agent adds "just
  in case".
- **Indices are `UNSIGNED_SHORT` (componentType 5123), not `UNSIGNED_INT`.** Task
  35's buffers are `GL_UNSIGNED_INT` and 37's draw call passes
  `GL_UNSIGNED_INT`, so the converter **widens u16 → u32**. 6,096 values, all
  below 3,184 after rebasing.
- **One material `colormap`, one texture, one image, `uri =
  "Textures/colormap.png"`** — a **relative URI, not embedded**, which is why the
  archive must be unzipped rather than the `.glb` copied out of it.
- **`samplers[0]` is `{ "minFilter": 9987 }` — `GL_LINEAR_MIPMAP_LINEAR`.** The
  upstream *asks for mipmaps*. Task 35 records that this engine's `MIN_FILTER` is
  `GL_LINEAR` and that `generate_mipmap` is never called. **The request is
  dropped**, and requirement 9 makes the drop an assertion rather than a silence.
  `magFilter` is absent, so it is `GL_LINEAR`; the wrap modes are absent, so they
  are `GL_REPEAT`.
- **`GL_REPEAT` versus this engine's `GL_CLAMP_TO_EDGE` is harmless here, and
  only because of a fact that must be asserted.** The UV ranges are
  `[0.09375, 0.84375] × [0.275, 0.975]` for the body and
  `[0.34375, 0.71875] × [0.525, 0.725]` for every wheel — entirely inside
  `[0, 1]`. Under `REPEAT` and under `CLAMP_TO_EDGE` a UV in `[0, 1]` samples the
  same texel. Requirement 9 asserts it per file, because an asset whose UVs left
  `[0, 1]` would sample a clamped edge and draw a stripe, and nothing in the
  runtime would say so.
- **`KHR_texture_transform` is declared and carries only `{"texCoord": 0}`** — no
  `offset`, no `scale`, so the transform is the identity. This matters because
  `MeshVertex.uv` has no transform and 37's vertex shader is `v_uv = a_uv` with
  no offset/scale uniform: a non-identity transform would silently change the
  texture. Requirement 9 refuses it.
- **`doubleSided: true`.** Task 37 enables `GL_CULL_FACE` with
  `cull_face(GL_BACK)` and leaves front face at `GL_CCW` per task 34's policy.
  **`doubleSided` and back-face culling are contradictory in general** — and for
  *this* asset the culling is right, for two measured reasons: the winding is
  consistent (**2,032 of 2,032 triangles have their face normal agreeing with
  their vertex normals**; zero disagree), and every shell is **closed** (after
  welding positions, **every edge has valence exactly 2** — no boundary edges, no
  non-manifold edges, across all five meshes). So there is no interior to see
  through, and `doubleSided: true` is a flag from the exporter rather than a
  requirement of the geometry. Requirement 9 records the measurement in the
  manifest so a future asset swap cannot rely on it by accident.
- **`asset.generator` is `"UnityGLTF"`, `asset.version` `"2.0"`** — provenance for
  the manifest.
- **Normals are already unit length** — measured minimum and maximum `‖n‖` are
  both exactly `1.0` on all five meshes. `MeshVertex::new` normalises anyway, and
  requirement 9 says the converter must **not** rely on that, because a
  non-uniform node scale would need the inverse transpose and 35's `new` does not
  know about scale.

### Upstream two — Lucide 1.52.0, ISC

- **URL**:
  `https://github.com/lucide-icons/lucide/archive/refs/tags/1.52.0.zip`
- **Size**: 7,347,333 B. **SHA-256**:
  `f54137c1f9a08eb452624e13a087a8f759e5e36eeac089fce920ed5940c51034`
  (same date and same caveat as the Kenney hash)
- **1,866 `.svg`** files under `icons/`.
- **`LICENSE` is ISC, verbatim: *"Copyright (c) 2026 Lucide Icons and
  Contributors"*, and ISC's grant is conditioned on *"this copyright notice and
  this permission notice appear in all copies."*** **That is a hard obligation
  and it is discharged in-file, in requirement 4** — not by naming the licence in
  a manifest, which travels with nothing.
- **Style, and it is uniform across all 33 selected icons**: `width="24"`,
  `height="24"`, `viewBox="0 0 24 24"`, `fill="none"`,
  `stroke="currentColor"`, `stroke-width="2"`, `stroke-linecap="round"`,
  `stroke-linejoin="round"`. **All 33 verified individually.** The uniformity is
  what makes the stylesheet mechanism and the input guard in requirements 6 and
  12 possible at all.
- **33 icons verified present and relevant to a car HMI**: `car`, `car-front`,
  `battery-charging`, `gauge`, `plug-zap`, `zap`, `thermometer`, `snowflake`,
  `lock`, `lock-open`, `sun`, `clock`, `calendar`, `bell`, `download`,
  `navigation`, `play`, `pause`, `skip-back`, `skip-forward`, `volume-2`,
  `speaker`, `fan`, `settings`, `wrench`, `circle-alert`, `triangle-alert`,
  `power`, `map-pin`, `radio`, `wifi`, `smartphone`, `armchair`, `circle-dot`.
- **One miss: `gauge-metric` does not exist in this tag.** This matters far more
  than a missing glyph, and requirement 13 is entirely about it.

### `resvg`: the tint mechanism, verified empirically on 2026-10-05

`resvg` 0.48.1 rasterises and tints. Measured on `icons/car.svg` at
`--width 32 --height 32`, taking the mean of the 307 pixels with non-zero alpha:

| stylesheet | measured mean |
|---|---|
| `svg{color:#1a1a1a}` + `*{color:#1a1a1a}` | **(26.64, 26.64, 26.64)** |
| `svg{color:#eceff4}` + `*{color:#eceff4}` | **(236.16, 239.00, 244.26)** |
| **no stylesheet** | **(0, 0, 0)** |

Three consequences, and the third is the reason the tint check in requirement 12
is not optional:

1. **A stylesheet is how the colour is chosen.** The two hex values are the
   theme's ink colours and they live in the manifest, once, not in a `.css` file
   that can drift from them.
2. **Without a stylesheet the output is black**, because `usvg` has resolved
   `currentColor` to black since 0.9.0. That is a *successful* `resvg` run with a
   zero exit status and a valid 24×24 PNG.
3. **A black icon on the dark theme is nearly invisible and still passes a file
   count.** This is the same failure shape `DEMO_APPLICATION.md`
   § *Corrections to the second gap table* records — *"a bare `grep -i FBO`
   returns two hits, both of them comments denying an FBO exists, so '2 hits'
   reads as evidence of a partial implementation and is not"* — and the fix is the
   same: assert on the **value**, not on the artefact's existence. Requirement 12
   turns the measurement above into a per-PNG assertion.

**The `fill` trap, and why it does not apply here.** A `*{color:…}` rule does
nothing for an SVG set that uses a plain `fill` with no colour attribute, and
`{fill:…}` is required for those. **That is true, and it is the wrong tool for
this set**: all 33 Lucide files carry `fill="none"` on the **root `<svg>`**, so a
`{fill:…}` rule would *fill the glyph outlines* rather than tint them. Requirement
12 therefore uses `*{color:…}` **and** asserts `fill="none"` on every input root,
which is what makes the rule correct for this set and impossible to apply to a
set it is wrong for. `resvg` also supports `--export-id` for sprite slicing; it is
named here so the next agent knows it exists, and it is **not used** — the
runtime atlas is the packer (*Out of Scope*).

### The engine constraints that dictate the authoring rules

Four facts in `ui_core` decide what an asset may be. Each is cited by symbol
rather than by line number, because `ui/src/ui_core/src/render.rs`,
`paint.rs` and `texture.rs` are modified in the tree this task is written
against and a line number into them is already stale.

- **No texture tinting at draw time.** `IMAGE_FRAGMENT_SHADER_SRC` ends
  `frag_color = vec4(texel.rgb * opacity, texel.a * opacity);` — a **scalar**,
  never a colour — and `DrawCommand::Image`'s `opacity` field says so in its own
  doc: *"`0.5` is *the image, half as present* and not *the image, tinted*."*
  **Therefore every icon needs a separate baked PNG per theme.** A one-file
  icon set tinted at runtime is unreachable in this pipeline. **Text is
  tintable** — `TextVertex` carries a per-vertex `a_color` written to `v_color`
  and `TEXT_FRAGMENT_SHADER_SRC` outputs it — which is why this is about images
  and not about everything.
- **No mipmaps.** `MIN_FILTER` is `GL_LINEAR` and `generate_mipmap` is never
  called. Minification aliases. **Author at the exact on-screen pixel size.**
- **No DPI concept.** `Renderer::begin_frame` sets the viewport from the window
  size — logical points, not `InPixels` — and `rg -i 'dpi|hidpi|density|drawable_size'
  ui/src/` hits only doc comments. **1 asset pixel = 1 window pixel**, so a
  24-pixel icon is a 24-pixel PNG and not "24 points at some scale factor".
- **`ATLAS_MAX_IMAGE = 512`** in `texture.rs`. An image at or under that in both
  directions is packed into the atlas with the icons; anything larger gets a
  texture of its own. **Every icon this task bakes is 24×24, so all 66 share the
  atlas.** The atlas **evicts by shelf** — `evict_least_recently_used` removes
  the oldest *shelf*, which is why `TextureCache::pin` exists. **This task's
  output is not pinned by the pipeline**: pinning is a *runtime* decision made by
  whoever draws an icon, and requirement 15 keeps this task out of the runtime.
  What the pipeline owes is the size bound: **an asset over 512 in either
  direction escapes the atlas and gets its own texture**, which is a silent cost
  change at draw time, so requirement 9 refuses to emit one.

### Why the icons are rasterised images and not vector draws

This is the reason a build-time SVG rasteriser exists at all, and it is three
rows of `DEMO_APPLICATION.md` rather than a preference.

- **Row `L10` in § *Gaps this layout exposes in `ui_core`*** — *"`Polygon` is
  convex-only … no bezier, no fill rules … Note also that `DrawCommand::Path` is
  a **stroked** polyline with an explicit width, so it cannot serve as a filled
  icon outline either."* **A Lucide glyph is a stroked outline with round caps and
  joins, and this engine cannot draw one.** Not its geometry, not its curve, not
  its stroke.
- **Row `#4` in § *Library gaps*** — *"No Icon widget — `Image` (task 16)
  displays textures but icons need vector rendering, theme tinting, and uniform
  sizing"*, and after the Layout work *"this is the largest unsupported item in
  the design"*. The three things that row names are exactly the three this
  pipeline supplies offline: **the vector rendering** (`resvg`), the **theme
  tinting** (two baked sets), and the **uniform sizing** (one size, declared).
- **§ *Corrections to the first gap table*** puts the same fact in one sentence:
  *"There is no path or vector-glyph support anywhere: `DrawCommand::Path` is
  stroked, fonts rasterize to a glyph atlas, and there is no outline extraction,
  no SVG and no curve tessellation."* **"No SVG" is the gap this task fills from
  outside the library**, and the only place it can be filled.

**And § *Asset requirements* bounds what the baker may be asked for**, so the
manifest is not an open-ended wish list:

- *"**The icon inventory is much larger than the list above.** Every dock item,
  every conditional top-bar item, every Controls tab row, and ~20 indicator lights
  each need a glyph. The Controls tab list alone carries a distinct icon per row
  (car, lightning bolt, steering wheel, padlock, lamp, seat, screen, clock,
  warning circle, wrench, download arrow, navigation arrow)."* **The 33 names in
  `icons.json` are this row's list, read once and closed** — they are the
  Controls tab rows, the top-bar statuses, the media transport, the dock, the
  alerts and the climate surfaces, and nothing else.
- *"**Some widgets need *generated* glyphs, not icon files.** The seat widget's
  squiggle count encodes the level and its colour encodes heating vs cooling; the
  indicator column uses colour to encode severity and *timing* to encode
  fault-vs-condition. Those are drawn from state, not selected from an atlas."*
  **So the baker is not asked for a seat glyph, a severity ramp or a timing
  variant**, and requirement 13's closed list is the mechanism: a name that is not
  in the manifest cannot be requested, which keeps *"the seat squiggle"* from
  becoming *"one Lucide file, tinted at draw time"* — which the first bullet above
  has already established is impossible.
- The same section's governing principle applies to the two hexes in `icons.json`:
  *"Tesla publishes **no design tokens at all** … the asset inventory must
  produce them rather than transcribe them."* **They are first-principles
  choices, and `icons.json` is where they are recorded so a later change is a
  diff.**

### The prototype, and the two bugs it hit

A working offline rasteriser exists in this session at
`/tmp/opencode/carproto/render.py` (~150 lines, `numpy` + Pillow). It reads a GLB,
applies node transforms, z-buffers an orthographic projection, applies one
Lambert term and samples the texture through the UVs, and **it renders
`sedan.glb` correctly**. **It is not promoted into this task** — it rasterises,
and a rasteriser's output is not an asset this engine loads (see the narrowing
above). It is here for two reasons only, and both are negative:

1. **It proves the data is loadable with `numpy` and Pillow and nothing else.**
   No `trimesh`, no Blender. That is the whole justification for the toolchain in
   the host table.
2. **It hit two bugs that the real tooling must not repeat**, and both are
   recorded in the prototype's own comments:

   > *"A GLB separates mesh geometry from node placement: the wheels are authored
   > at the origin and moved by their node's translation. Reading meshes alone
   > puts all four wheels inside the body, so node transforms must be applied."*

   > *"Camera must be derived from the WHOLE model, not the selected subset, or an
   > isolated part is auto-framed to fill the frame and no two layers align."*

   **Both are silent, both look like a rendering bug, and neither raises an
   error.** Requirement 7 is the first; requirement 8 is the second, restated for
   a converter that has no camera: **every quantity the tool derives from
   geometry is derived from the whole model, and the tool has no subset mode.**

   The measured version of the first is stronger than the comment says: **all
   five nodes carry a translation, not only the four wheels** — `body` is
   translated `[0.0, 0.15, -0.025]` — so a converter that special-cases "the
   wheels are the offset ones" gets the body's ride height wrong by 15 cm, which
   is not a visible catastrophe and is exactly why it is specified rather than
   remembered.

### The trademark guard, and why it is a pipeline property

**Do not ship the Tesla T, shield or wordmark.** The facts:

- Wikimedia tags `Tesla_T_symbol.svg` public-domain via `PD-textlogo` **and**
  carries `{{Trademarked}}`. A public-domain **copyright** tag is not a
  **trademark** licence, and an asset set that answers "is it public domain?"
  without answering "is it a registered mark?" has answered the wrong question.
- Tesla counsel sent a §512(c) DMCA notice on **2024-09-12** asserting registered
  copyright and trademarks for *"TESLA, TESLA (Stylized), and T Logo"*.
- **`simple-icons` is CC0 and ships `tesla.svg`** — the same trap again, one
  dependency away, and the reason "it's CC0" is not a defence that reaches
  anything.

**So the pipeline is designed so that no Tesla mark can enter, rather than so
that a reviewer has to notice one.** Five mechanisms, all in § Requirements:
a **closed 33-entry allow-list** of semantic names (requirement 13); a
**host allow-list** of exactly two URLs (requirement 3); a **root-attribute
guard** that accepts only Lucide's exact shape (requirement 12); a
**`grep -ri tesla` over the committed assets and the manifests** as a hard
criterion (acceptance criterion 7); and — the one that does the real work — the
**recorded treatment of the one miss**, `gauge-metric` (requirement 13). An
absent glyph is exactly what tempts an agent to go looking for a set that has
it, and `simple-icons` is the set that has it and is CC0 and therefore looks
perfectly safe.

**What this task does not decide.** `DEMO_APPLICATION.md` § *Open questions*
item 3 — *"How to handle the Tesla logo and branding? (avoid trademark issues)"* —
**stays open.** What is decided here is only that the pipeline **cannot ingest
one**. The operator's decision on the question itself is still owed, and a task
that answered it would be a task that answered it without being asked.

### Vendoring: **not vendored**, fetched on demand, verified by hash

The decision, with its arithmetic:

| | bytes | in git? |
|---|---|---|
| Kenney archive | 4,814,237 | **no** |
| Lucide archive | 7,347,333 | **no** |
| model file (3,184 × 32 B + 6,096 × 4 B + header) | ~126 KB | **yes** |
| `colormap.png` | 12,371 | **yes** |
| 66 baked icons at 24×24 RGBA | ~45 KB | **yes** |
| **committed total** | **~183 KB** | |

**Vendoring 11.7 MB of upstream into a git repository whose committed derived
output is 183 KB buys nothing.** The reasoning, in the order that decided it:

1. **A hash is a stronger guarantee than a blob.** A vendored archive can be
   edited, re-zipped or replaced and nothing downstream notices; a recorded
   SHA-256 either matches or the fetch is refused. Vendoring trades a checkable
   identity for an unchecked copy.
2. **Git keeps deleted blobs forever.** Dropping the archives in a later commit
   would not reclaim the 11.7 MB from history — it would leave the repository
   carrying the cost and losing the benefit, which is the worst of both.
3. **The upstream is already pinned by a tag and a URL.** `lucide-1.52.0.zip` is
   a GitHub tag archive; it is immutable and re-fetchable byte-for-byte. The
   Kenney URL carries a content hash in its own path segment. Neither needs a
   copy to be reproducible.
4. **The offline story does not depend on the vendoring choice**, and this is the
   point. **The build never needs the network**: `ui_demo` reads committed bytes,
   and the committed bytes are in the repository. Only *regenerating* them needs
   the upstream, and regeneration is a deliberate act by a human with a cache.
   Requirement 3 says what happens when the network is absent: the cache is used,
   and **a missing cache with no network is a hard failure naming the URL** — not
   a skip, not an empty output directory, not a half-written asset set.
5. **A cache is the right shape for the 12 MB.** `.asset-cache/` is one
   `.gitignore` line, and after the first run it makes the second run offline.

### Determinism, stated precisely — and one thing that is **not** byte-stable

This is the part most likely to be got wrong, so it is separated out.

- **The model file is byte-identical across runs, and the mechanism is that it
  contains no encoder.** It is a 40-byte header, four blocks, and raw
  little-endian `f32` and `u32` arrays written by `struct.pack`. The only
  arithmetic between the GLB and the output is elementwise translation and
  scaling of `POSITION` — no dot products, no reductions, no order-dependent
  summation — so there is nothing in it that a different CPU, a different BLAS or
  a different SIMD width could reorder. **This is what `sha256sum -c` may be
  used on.**
- **The PNGs are byte-identical *on a given host* and are not guaranteed to be
  across hosts, and the difference is not carelessness.** Measured here: writing
  the same 32×32 RGBA array twice with Pillow 12.1.1 and zlib 1.3.1 produces
  **identical bytes**, and the chunk list is `IHDR` then `IDAT` — **no `tIME`
  chunk**, so there is no timestamp to drift. But the `IDAT` stream is deflate
  output from whichever zlib built the Pillow, and deflate output is not a
  specified function of the input. **So: `sha256sum -c` on a PNG verifies that
  *this host's encoder* has not changed its mind, and it does not verify
  reproducibility.** The cross-machine check is **pixel comparison** —
  `magick compare -metric AE` — and that is what **acceptance criterion 2(b)**
  uses, where criterion 3 is the one `sha256sum -c` actually serves.
- **`colormap.png` is copied, not re-encoded**, so it *is* byte-stable, and
  requirement 9 says to copy it rather than round-trip it through Pillow.

### Scope check

`developer.md` § *Scope check* asks for a file count and a component count, and
`developer.md` § *Stop conditions* asks whether the task needs splitting before
either number is read as a pass.

**The honest count is eleven**, not the ten an earlier draft of this file
claimed: one `.gitignore` line; **eight** new files under
`tools/asset-pipeline/`; one `LICENSES.md`; and three committed artefacts under
`ui/src/ui_demo/assets/` counted as **one each** rather than as 69 files — the
`.roados`, the colormap and the icon tree. **Eleven is over `developer.md`'s
five-file threshold**, and the file says what happens when it is: *"If the change
touches more than 5 files or has more than 3 independent components, it is too
large for one agent. Split it per `.ai/protocols/subagents.md` § *Implementation
fan-out` before writing code."*

**It is split, and the split is three sub-tasks, not one task pretending:**

1. **`glb_to_model.py` + `model.json` + `sedan.roados` + `colormap.png`** — the
   3D converter and its two outputs. Depends on 38's byte table.
2. **`bake_icons.py` + `icons.json` + the 66 PNGs + `icons/LICENSE`** — the icon
   baker. **Depends on nothing from sub-task 1** and could land first or in
   parallel.
3. **`fetch_upstream.sh` + `upstream.sha256` + `check_assets.py` + `LICENSES.md` +
   the `.gitignore` line** — the guard, and **it depends on both of the others**
   because it reads what they write.

**Four independent components**: converter, baker, fetcher, checker — the fetcher
was folded into sub-task 3 rather than counted separately, which is the one place
this count is generous rather than exact, and it is the fetcher that is a shell
script of thirty lines. **`.ai/protocols/subagents.md` § *Implementation
fan-out* therefore applies**, sub-task 3 is dispatched after 1 and 2, and the
operator gets three handoffs and three diffs instead of one unreviewable
eleven-file change.

**Zero Rust files** in all three. The components are coupled only through two
manifest paths and one output directory, which is what makes the fan-out safe: no
sub-task needs another's intermediate result except the checker, which runs last
by construction.

### Task 38's file, and the six decisions it hands this task

**This task depends on task 38**, which task 34's sequence table says it does not
(*"39 | the asset pipeline (offline render and packaging) | nothing"*). That row
was written before either file existed, and **it is now false in the only way that
matters**: this task writes 38's byte table and puts its output where 38's
`ASSET_MODEL_RELATIVE` and colormap `const` look for it, so it cannot be written
before 38 and a file written against the wrong table would be a **silent misparse**
— 38's requirement 4's exact phrase, *"a format whose reader guesses is a format
that draws a plausible wrong car"*. The dependency is on 38's **document**, not on
38's **code**: the writer is Python, the reader is Rust, and nothing here imports
`meshio`.

`doc/ui/TASK_UI_PRIM_38.md` **is** in the tree, so **this task writes 38's format
and does not propose one.** Requirement 9 restates 38's byte table rather than
inventing a second, because 38 § *Out of Scope* is explicit: *"the format is
specified so 39 can write it; 39 writing it is not this task"* — and two byte
tables is two things to keep honest, which is the shape of defect
`DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice. The
differences that mattered to an earlier draft of this file, and 38's answers:

| | this task would have written | **38 decides** |
|---|---|---|
| magic | `ROADOM01` | **`ROADOSMF`**, *ROADOS Mesh Format* |
| header | 64 B, with `bounds_min` / `bounds_max` | **40 B**, and **no bounding box at all** — *"a file that carries its own pivot is a second answer to a question the transform path answers"* |
| version | none | **`version: u32` = 1**, refused if not 1 |
| name table | fixed `name[24]` per entry | **a variable `names` block of NUL-terminated UTF-8**, after the vertices, indices and `(first_index, index_count)` pairs |
| file name | `sedan.bin` | **`sedan.roados`** |
| where | a `models/` subdirectory | **`ui/src/ui_demo/assets/sedan.roados`**, which is 38's `ASSET_MODEL_RELATIVE`, with **`colormap.png` beside it** |
| merge order | an explicit manifest list | *"concatenated in **file order**"* — see the contradiction below |

**Six decisions 38 assigns here, each answered below rather than deferred:**
**compression** (38's requirement 2: *"Compression is task 39's decision"*);
**the seven conversion-time operations** — normals, welding, tangent generation,
smoothing, re-indexing, degenerate-triangle removal, winding normalisation (38's
*Out of Scope*: *"All of it is 39's, at conversion time, precisely because all of
it is once-per-asset work"*); **the origin convention** (38's *Out of Scope*:
*"where the car sits is 39's"*); **whether to split the glass into a sixth
sub-mesh** (*"the decision to do so is 39's"*); and, from 38's requirement 2,
**that the file carries no texture reference**, which is why requirement 9 emits
the colormap as a second file and the manifest names it.

**Three contradictions between the sibling files, named here because 39 sits
downstream of all of them and a reviewer will find them:**

1. **38 contradicts itself about merge order, and 39 picks a side.**
   Requirement 3 says the parts are *"concatenated in **file order**"*, and its
   very next bullet says *"The five names, in the file, are `body`,
   `wheel-front-left`, `wheel-front-right`, `wheel-back-left`,
   `wheel-back-right`"*. **Those are not the same sequence**: the GLB's
   `meshes[]` order is `wheel-front-right`, `body`, `wheel-back-left`,
   `wheel-back-right`, `wheel-front-left`, measured. Read as "GLB file order" the
   sentence names `wheel-front-right` first; read as 38's stated list, `body` is
   first. **39 writes 38's stated list**, because 38's demo side carries
   `ranges` *"in file order"* and two documents that disagree about which order
   that is would be the defect 38 exists to prevent. **Nothing breaks either
   way**, and 38 says why: *"nothing in the loader knows that. The loader does
   not recognise a name, does not require a wheel to exist and does not order the
   parts."* The loader is order-agnostic; only the two documents have to agree,
   and requirement 5 makes 39's `merge_order` the single place that says so.
2. **37 and 38 disagree about `std::fs` in `ui_core`.** 37's acceptance
   criterion asserts `grep -rc 'std::fs\|File::open\|read_to_string'
   ui/src/ui_core/src/` is **empty** — *"it is empty today and must stay so, since
   no `ui_core` API takes a file path and nothing here may begin to."* 38's
   requirement 7 puts `load_from_path` in `ui_core` and calls it *"the only place
   `std::fs` appears in `ui_core`"*, on the precedent of `TextureCache::load_from_file`
   and `Font::from_path`. **One of those two criteria cannot pass once both tasks
   have landed, and it is not 39's to settle.** It is recorded here because 39
   writes the file that function reads, and because **39's own criterion must not
   repeat the claim** — so this task's acceptance criterion greps for `std::fs`
   in **its own** new files and says nothing about `ui_core`, and the
   disagreement is referred to the operator.
3. **38 assigns "where the car sits" to 39, and 37 and 40 say otherwise.** 37
   puts the transform **on the draw command** and makes the recording order the
   contract; 40 owns the rotation and closes gap `L4`. **39 has no runtime code,
   so it cannot place anything.** The only reading that leaves 39 consistent with
   both is: **39 owns the object-space origin convention baked into the file, and
   the `Mat4` that puts it on screen is the demo's.** Requirement 9 says what that
   convention is.

### The seven conversion-time operations, each decided with a reason

38 hands all seven to this task. Each is a decision a general-purpose converter
would make silently, which is why each is named, decided, and recorded in the code
that declines or performs it.

| operation | decision | why |
|---|---|---|
| **Normals recomputed** | **No — keep the file's** | The file's normals are already **unit length on all five meshes** (measured: min ‖n‖ = max ‖n‖ = 1.0). Recomputing them from face normals would replace the smooth body with a faceted car, because the body has averaged normals across its creases (measured: face-normal-versus-vertex-normal `dot` goes as low as **0.0000** on `body` while the wheels stay at **0.9700–1.0000**). The file's normals *are* the model's shading |
| **Welding** | **No** | Measured: each wheel carries **528 vertices that are only 168 distinct positions**, and the body **1,072 that are only 358** — and after welding, **every edge has valence exactly 2** across all five shells. So the duplication is **not** sloppy topology; it is how the export stores split normals, and welding would collapse them. A general-purpose GLB converter's most celebrated optimisation is the wrong move here |
| **Tangent generation** | **No** | `MeshVertex` is three fields at stride 32 (task 35 requirement 1) and 37's shader declares `layout(location = 0..2)` with no tangent. A tangent is a **stride change**, and 38 puts a vertex-format change on *"whoever changes the format"*. The GLB's own `TANGENT` (VEC4, 528/1072 per mesh) is **read and discarded**, never carried |
| **Smoothing / normal averaging** | **No** | Same evidence as welding, and doing it twice is worse than doing it once |
| **Re-indexing** | **No** | The output is the concatenation, so two runs on two machines produce the same bytes **by construction** rather than by a canonicalisation pass that has to be right |
| **Degenerate-triangle removal** | **No — and this one is measured, not obvious** | **`body` contains 6 zero-area triangles out of 704** (triangle indices 296, 308, 593, 594, 654, 655 — three **collinear** triples, all at constant `x` and `y` with only `z` varying). They cost **18 of 6,096 indices, 0.3 %**, and they rasterise **nothing**: a zero-area triangle has no interior, so it draws no fragments. **Removing them opens the shell**: with all 704 triangles the body's welded edge-valence histogram is `{2: 1056}` — watertight — and with the six gone it becomes **`{1: 14, 2: 1040}`**, i.e. **14 boundary edges and fourteen holes**. Task 37 enables `GL_CULL_FACE` with `cull_face(GL_BACK)`, so **an open shell is a visible defect and a closed one is not.** Keeping six invisible triangles is strictly better than trading a watertight shell for a 0.3 % index saving, and the `measured` block in `model.json` records `degenerate_triangles: 6` so a future asset swap that changes the count is noticed |
| **Winding normalisation** | **No** | Measured: **2,032 of 2,032 triangles have a face normal agreeing with their vertex normals; zero disagree.** Winding is already consistent and already matches `GL_CCW` with `cull_face(GL_BACK)` as task 37 requires and task 34's policy fixed. "Normalising" it would change bytes for no gain and, worse, make the converter's output depend on a re-orientation rule that nothing needs |

**The material's `doubleSided: true` is not a contradiction of this table, and the
reason is measured.** Task 37 culls back faces; the material asks for both sides.
For *this* asset culling is right, because the shells are **closed** (every welded
edge valence exactly 2) and consistently wound (2,032/2,032 agree), so every back
face is genuinely hidden. `model.json`'s `measured` block records
`double_sided: true`, `winding_consistent: true` and `shells_closed: true`
**together**, because the third is what makes the first survivable: an asset that
kept `doubleSided` and lost closure would be a different pipeline and a new
decision.

### Compression: **no**, and why

38 § *Out of Scope* refuses compression in the **format** and its requirement 2
assigns the decision here: *"An indexed 2,032-triangle car is on the order of
200 KB of vertices and 25 KB of indices … Compression is task 39's decision, and
it would turn requirement 6's bounds arithmetic into a decompressor's problem."*

**Decision: no compression. The model file is the raw interleaved arrays.**
Measured, uncompressed: **101,888 B of vertices** (3,184 × 32) + **24,384 B of
indices** (6,096 × 4) + a 40-byte header + a 40-byte sub-mesh table (5 × 8) +
a 73-byte name block = **126,425 B ≈ 123 KB**. Four reasons:

- **It is read once, at start-up, and stays resident.** 38's own framing. A
  compressed payload would need a decompressor resident for the run's lifetime to
  save 124 KB of a head unit's RAM — which is not a trade this project has
  measured.
- **It would make 38's validation a decompressor's problem**, exactly as 38 says:
  every count in 38's requirement 6 is bounds-checked against the bytes actually
  present, and a compressed file's `vertex_count` describes bytes that are not
  there yet. **A hostile-file argument gets harder to state the moment the bytes
  are not the payload**, and that argument is 38's Phase-2 review ground.
- **`flags` and `reserved` are 38's two spare words**, and 38 says they are *"the
  two words that let a future v1 carry something (a LOD hint, **a checksum**)
  without a version bump"*. **Spending them on a compression flag would close the
  checksum door for a 124 KB saving.** That is the wrong trade in a format whose
  loader is a security surface.
- **A compressor would be a build-time dependency that could change output
  bytes**, which is the one thing *Determinism* below is built on: a zlib version
  bump would rewrite a committed binary asset with no source change. **The raw
  writer's only arithmetic is a translation, so its output is a function of the
  input.** That property is worth more than 40 KB.

**The consequence, stated rather than left:** the `.roados` file **is** the
geometry, byte for byte, and `check_assets.py` can verify it against
`model.json`'s `measured` block by reading the bytes with no decompression and no
format knowledge beyond 38's table.

### Where the car sits, and what this task may decide about it

38 § *Out of Scope* says *"No camera, no placement, no `Mat4` construction … and
where the car sits is 39's."* **37 and 40 say otherwise** — 37 puts the transform
on the draw command and makes the recording order the contract, 40 owns the
rotation — and **39 has no runtime code, so it cannot place anything.**

**What 39 decides: the object-space origin convention, baked into the file.** The
measured, post-bake extents are:

| axis | min | max | who sets it |
|---|---|---|---|
| X | **−0.75** | **0.75** | the body — half-width 1.5 m |
| Y | **0.00** | **1.30** | **Y = 0 is the wheels' contact with the ground**, and the roof is at 1.30 |
| Z | **−1.30** | **1.25** | the body plus its `−0.025` translation — **asymmetric, and deliberately not corrected** |

So: **`Y = 0` is the ground plane**, `X = 0` is the car's centreline, and the
model is **not** re-centred on Z. Three reasons not to "tidy" the Z asymmetry:
1.30 and 1.25 differ by 25 mm because Kenney's `body` mesh carries a
`−0.025` translation, and **undoing it would mean undoing the node transform that
requirement 7 exists to bake** — the two bugs would cancel and the requirement
would look like it worked. **The asymmetry is the evidence.** It is recorded in
`model.json`'s `measured` block and asserted, which is what makes the node baking
checkable at all now that 38's format carries no bounds.

**What 39 does not decide:** the `Mat4`, the scale, where on screen the car sits,
and any rotation. The demo builds the `Mat4` from 36's type and 37's command, and
task 40 rotates it. **The ground-plane convention is what makes that `Mat4`
writable by someone who has never opened a GLB**: a placement is
"translate up by 0, back by some metres, scale by some factor", and Y = 0 being the
ground is the fact that tells them which way is up.

### The glass: **no sixth sub-mesh**, and why

38's requirement 2 records that *"glass and lamps are not separable meshes — they
are UV regions in `colormap.png`"*, 37's *Out of Scope* records that finer
recolour needs *"more sub-meshes in the model file (38's decision, and it changes
the file format)"*, and 38's *Out of Scope* says **the decision to split it *"is
39's"*.**

**Decision: no. Five sub-meshes, `body` and the four wheels.**

Three reasons, and the third is the one that settles it:

1. **38's format makes it possible** — a variable `sub_mesh_count` and a variable
   name block, which 38 says is exactly what it is for. **This is not a reason to
   do it now.**
2. **Nothing can use it.** 37 delivers **per-sub-mesh** tint, and
   `DEMO_APPLICATION.md` § *Could not verify* lists *"Track Mode's car-body
   recolouring rules"* as *"Named but not enumerated per component in anything
   read; treated here as `[B]` and **not a spec the demo should copy until
   verified**"*. **There is no rule to implement**, so a sixth sub-mesh would be a
   capability with no caller — and the project's own rule
   (`developer.md` § Phase 2, *"No abstraction before the second use"*) applies to
   asset granularity as much as to Rust types.
3. **Splitting by UV region is a guess about intent, and it would be baked into a
   committed file.** The glass is `(56, 56, 61)` occupying one contiguous box in a
   512×512 palette of ~2,330 flat colours; deciding that this particular dark swatch
   is glass and that the adjacent dark swatch is not is a judgement about the
   model that **cannot be re-derived from the file**. If it is wrong, the wrong
   answer is committed, reviewed as a diff of binary bytes, and expensive to
   change. **A capability that can be added in ten minutes when a rule exists
   should not be added now on a guess.**

**And the honest limit this leaves**, which is 37's and not restated as a
resolution: on this asset, per-sub-mesh tint reaches `body` against the four
wheels **and nothing finer**. That is a property of the **asset**, not of the
reader — 38 says so in as many words.

### Task 38's test fixture is not this task's asset, and must not be overwritten

38 § *Out of Scope* records that **"no file is committed to
`ui/src/ui_demo/assets/`"** in that task, that the only encoder in the tree is
*"a private one in `mod tests`"*, and that **a fixture ships at
`ui/src/ui_core/tests/data/sedan.roados`** — *"a fixture rather than an asset"*.

**So there are two writers of `ROADOSMF` bytes in this repository after 39 lands,
and they are independent on purpose.** The fixture is hand-built in a test to
exercise the loader against a *hostile* input; this task's output is built from a
*real* model. **Requirement 14 forbids the pipeline from touching
`ui/src/ui_core/tests/`**, and the criterion that checks it is a `git diff --stat`
over that path — because a converter that "helpfully" regenerated the fixture from
the GLB would **delete the loader's test coverage of the malformed cases**, and
the diff would show one file changed and nothing about it.

## Requirements

1. **`tools/asset-pipeline/`, and not `.ai/tools/`.** Eight files, each with a
   header saying what it is, what it reads, what it writes, and **what it must
   not be used for** — `.ai/tools/README.md`'s shape, and the reason for the
   choice is in *Context*: `AGENTS.md` protects `.ai/` unless the task is about
   the AI engineering system, and a build tool is not.

   - `tools/asset-pipeline/README.md` — the four commands, in order, and the
     decisions below written down.
   - `tools/asset-pipeline/fetch_upstream.sh` — requirement 3.
   - `tools/asset-pipeline/upstream.sha256` — the two recorded hashes.
   - `tools/asset-pipeline/model.json` — requirement 5.
   - `tools/asset-pipeline/icons.json` — requirements 11 and 13.
   - `tools/asset-pipeline/glb_to_model.py` — requirements 6, 8, 9.
   - `tools/asset-pipeline/bake_icons.py` — requirements 10, 11, 12.
   - `tools/asset-pipeline/check_assets.py` — requirements 13, 14.

   **No executable bit is required on any of the eight** — every invocation in
   this file is `bash tools/asset-pipeline/fetch_upstream.sh` or `python3
   tools/asset-pipeline/<name>.py`, so the command line in a reviewer's shell is
   the command line that ran, and a `chmod` cannot become part of the diff.
2. **`.gitignore` gains exactly one line: `/.asset-cache/`.** One line, anchored,
   with a two-line comment saying what it holds and that the committed assets
   under `ui/src/ui_demo/assets/` are **not** covered by it — because a reader
   seeing a `.gitignore` entry about assets has to be able to tell that the
   assets are in the tree. **Nothing else in `.gitignore` changes.**
3. **Pinning, fetching and the offline path.** `fetch_upstream.sh` takes no
   arguments and does four things, in order, each failing loudly:

   1. **Refuses any URL not in its allow-list** — exactly two entries:
      `https://kenney.nl/media/pages/assets/car-kit/1a312ec241-1775131960/kenney_car-kit.zip`
      and
      `https://github.com/lucide-icons/lucide/archive/refs/tags/1.52.0.zip`.
      **The allow-list is the trademark guard at the network edge**: an agent who
      wants to point the pipeline at `simple-icons` must edit this file, which is
      a reviewable one-line diff. The refusal message names the offending URL and
      the two permitted ones.
   2. **Downloads into `.asset-cache/` with `curl -sSfL --retry 3`** — a file
      already present with the right hash is **not** re-downloaded, so the second
      run is offline.
   3. **Verifies with `sha256sum -c upstream.sha256` and refuses on mismatch.**
      This is a **stop condition**, not a warning: a hash mismatch means the
      upstream moved under a pinned URL, and the correct response is to re-read
      the licence and re-record the hash deliberately, not to update the file
      and continue.
   4. **Unzips into `.asset-cache/kenney_car-kit/` and `.asset-cache/lucide-1.52.0/`,
      preserving the archive's internal paths** — the GLB references
      `Textures/colormap.png` relatively, so a tree that does not have that path
      next to it cannot be loaded.

   **When the network is unavailable**: if the cached file is present and hashes
   correctly, the script prints that it is running from cache and exits 0. **If it
   is absent, it exits non-zero naming the URL and `.asset-cache/`** — it does not
   fall through to an empty tree, and it does not write one. **A partial
   extraction is removed before the extraction begins**, so an interrupted run
   cannot leave a half-tree that a later run treats as complete.

   **`upstream.sha256` carries the two hashes measured on 2026-10-05** (given in
   *Context*). **The developer re-runs the fetch and records what the download
   actually produced; if it differs, the run stops and the difference is
   reported.** A hash this task file carries is a fact about one host on one day,
   and the acceptance criterion is that `sha256sum -c` passes against the file in
   the repository.
4. **`LICENSES.md` at the repository root, and one `LICENSE` beside the icons.**
   Two obligations, discharged in two places, and the reason is in *Context*:

   - **`LICENSES.md`** — a table with one row per upstream: name, exact version,
     the URL fetched, the SHA-256, the licence, **what that licence obliges**, and
     **what was done about it**. The Kenney row records **CC0, no obligation,
     credit optional — and this repository credits it anyway**. The Lucide row
     records **ISC, and the verbatim notice**, which is quoted in full:
     *"Copyright (c) 2026 Lucide Icons and Contributors"* plus the permission
     sentence and the warranty disclaimer. The file also carries the project's own
     `LICENSE` (GPLv3) as a row, so a reader looking for "what licence is this?"
     finds all three in one place rather than two.
   - **`ui/src/ui_demo/assets/icons/LICENSE`** — the **ISC notice verbatim**, so
     the notice travels **with any copy of the icon directory**. ISC conditions
     the grant on the notice appearing *in all copies*; a licence recorded in a
     file at the repository root does not travel with `cp -r icons/ somewhere/`.
     This is the same reasoning `.ai/tools/topy.py`'s README states for licence
     footers being among the things a text-extraction tool drops.
   - **`colormap.png` gets its provenance in `model.json`**, and the Kenney row
     in `LICENSES.md` names it, because a derived image with no stated origin is
     an unlicensed image with extra steps.
5. **`model.json` — the model manifest, and every number in it is a decision or a
   guard, not a discovery.** Fields: `upstream` (the archive name from
   `upstream.sha256`), `model` (`Models/GLB format/sedan.glb`), `texture`
   (`Models/GLB format/Textures/colormap.png`), `out_model`
   (`ui/src/ui_demo/assets/sedan.roados` — 38's `ASSET_MODEL_RELATIVE`, the
   literal string, so there is one spelling of where the demo looks),
   `out_texture` (`ui/src/ui_demo/assets/colormap.png` — 38's colormap const,
   beside the model in the same directory, and **not named by the format**, which
   carries no texture reference), **`merge_order`** — the explicit ordered list
   `["body", "wheel-front-left", "wheel-front-right", "wheel-back-left",
   "wheel-back-right"]`, which is **38's stated name sequence** and resolves 38's
   own *"file order"* ambiguity as recorded in *Context* — **`up_axis` (`"y"`)**
   and **`origin` (`{"ground_plane": "y=0", "centreline": "x=0", "z":
   "not re-centred"}`)**, `units` (`"metres"`, matching task 35's `MeshVertex::position`
   doc), the two hex colours are **not** here (they are an icon concern,
   requirement 11), and a `measured` block carrying the numbers from *Context* —
   `meshes: 5`, `triangles: 2032`, `triangles_per_mesh: {"body": 704, "wheel-…":
   332}`, `degenerate_triangles: 6`, `vertices: 3184`, `indices: 6096`,
   `index_component_type: 5123`, `glb_mesh_order: ["wheel-front-right", "body",
   "wheel-back-left", "wheel-back-right", "wheel-front-left"]`,
   `node_translations: {"body": [0.0, 0.15, -0.025], "wheel-front-left": [0.3,
   0.3, 0.66], …}`, `tangent_dropped: true`, `min_filter: 9987`,
   `double_sided: true`, `texture_transform: identity`,
   `uv_within_unit_square: true`, `winding_agreement: "2032/2032"`,
   `shells_closed: true`, `welded_edge_valence_2: true`,
   `bounds_min: [-0.75, 0.0, -1.3]`, `bounds_max: [0.75, 1.3, 1.25]`,
   `bytes: {header: 40, vertices: 101888, indices: 24384, sub_meshes: 40,
   names: 73, total: 126425}`.
   **A `measured` block that disagrees with what the tool reads is a hard
   failure**, which is what makes it a guard rather than documentation: change the
   upstream model and the pipeline refuses rather than quietly emitting a
   different car.
   **`merge_order` is explicit for two reasons, and the second is the one that
   matters.** The first: the GLB's `meshes[]` order is not logical — it begins
   with `wheel-front-right`. The second: **task 38's format carries no bounding
   box**, so there is nowhere in the file to record what the geometry looks like,
   and `model.json`'s `bounds_min` / `bounds_max` / `bytes` are the **only**
   description of the emitted file's shape that survives the commit. They are what
   makes requirement 7's node baking checkable at all.
6. **`glb_to_model.py` reads the GLB subset, and refuses everything else.**
   Reads, and only these: the chunk table, the JSON chunk, the `BIN` chunk,
   `scenes` → `nodes` → `meshes`, and per primitive `POSITION`, `NORMAL`,
`TEXCOORD_0`, `indices`, **honouring `bufferViews[i].byteOffset` and
    `bufferViews[i].byteStride`**. In *this* file both are harmless and that is
    worth saying precisely, because it is the kind of reader that looks like a
    guard and is not one: **no accessor carries its own `byteOffset`, and all ten
    strided `bufferViews` have `byteStride == 12`, which is exactly the natural
    size of a `VEC3` of `f32`** — so a packed reader would coincidentally produce
    correct output here. The reader honours both fields anyway, because the next
    asset is not this one and a silently mis-strided accessor yields **wrong
    numbers rather than an error**: a `NORMAL` read at the wrong stride is still
    unit-length, so it changes the shading and raises nothing.
    **`TANGENT` is read and discarded, never carried.**
   **It must fail, naming the mesh and the field, on**: a primitive with no
   `POSITION` or no `TEXCOORD_0`; a `mode` other than 4 (`TRIANGLES`); an index
   `componentType` other than 5121 or 5123; an attribute `componentType` other
   than 5126 (`FLOAT`); a `normalized` attribute; a `KHR_texture_transform` with
   an `offset` or `scale` that is not the identity; an accessor whose `count`
   disagrees with its `min`/`max`; **a mesh in `meshes[]` that no node in the
   scene references** (an orphan the scene walk drops silently, and the prototype's
   class of bug); and a node whose `mesh` index is out of range.
7. **`glb_to_model.py` bakes node transforms, and the silent-failure rules are
   the requirement.** For each node reached through `scenes[0].nodes`, in scene
   order: apply `translation`, then `rotation` (quaternion, xyzw), then `scale`,
   composing as `T · R · S`; apply the composed 4×4 to `POSITION` as a point
   (`w = 1`) and to `NORMAL` as a direction (`w = 0`, renormalised). If a node
   carries a `matrix`, decompose or refuse — **refusing is allowed and preferred**
   over a silent ignore. **If a node carries a `rotation`, a `scale` other than
   `[1,1,1]`, or a `matrix`, the tool refuses with a message naming the node** —
   unless it implements that case, in which case `model.json` says so and the
   normal path uses the inverse transpose for a non-uniform scale.
   **Measured on this asset**: all five nodes carry only a `translation` —
   `body [0.0, 0.15, -0.025]`, and the four wheels at
   `[0.3, 0.3, 0.66]`, `[-0.3, 0.3, 0.66]`, `[0.3, 0.3, -0.66]`,
   `[-0.3, 0.3, -0.66]` — and no `rotation`, no `scale`, no `matrix`. **The
   requirement is written for the general case anyway, because the failure is
   silent**: a converter that ignores an unknown node field produces a car with
   one wheel slightly wrong and no message. **The prototype's own comment is the
   specification**: *"A GLB separates mesh geometry from node placement: the wheels
   are authored at the origin and moved by their node's translation. Reading meshes
   alone puts all four wheels inside the body."*
8. **The merge, and the seven conversion-time operations, each declined by
   decision.** `merge_order` from `model.json`, in order; each mesh's vertices
   appended to one `float32` array in **stride-32 interleaved** order —
   `position[3]`, `normal[3]`, `uv[2]`, matching task 35's `MeshVertex` field
   order and its `MESH_NORMAL_OFFSET = 12` / `MESH_UV_OFFSET = 24` — and its
   indices appended to one `uint32` array with **every value offset by the running
   vertex base**, matching task 35's requirement 4 (*"`indices` are absolute
   vertex indices … already offset by the loader"*). Sub-mesh `first_index` is the
   **index position**, not a byte offset; `index_count` is the mesh's index count,
   and it is **a multiple of 3** for every mesh — including `body`'s **2,112**,
   which is 704 × 3, **because the six degenerate triangles are kept** (*Context*,
   the conversion-time table). 38's requirement 6 checks that multiple and would
   refuse a file where it did not hold.

   Then the seven operations 38 assigns here, each present in the code as a
   **named no-op with its reason in the comment**, and each listed in
   `model.json` so a later reader can see the decision without reading the source:

   - **Normals: the file's, renormalised, never recomputed.** Measured: already
     unit length on all five meshes, and the body's face-versus-vertex-normal `dot`
     goes as low as **0.0000** while the wheels sit at **0.9700–1.0000** — the body
     carries averaged normals across its creases, and recomputing from faces would
     facet it. Renormalising is belt-and-braces: 38's loader calls 35's
     `MeshVertex::new`, which normalises anyway.
   - **Welding: no.** 528 vertices per wheel are only **168** distinct positions;
     1,072 for the body are only **358**; and after welding, **every edge has
     valence exactly 2** across all five shells — so the duplication is how the
     export stores split normals, not sloppy topology. Welding would collapse them.
   - **Tangent generation: no**, and the GLB's own `TANGENT` (VEC4) is read and
     discarded. A tangent is a stride change to 35's vertex format, and 38 puts
     that on *"whoever changes the format"*.
   - **Smoothing / normal averaging: no** — the same evidence as welding, and doing
     it after not welding is the same damage twice.
   - **Re-indexing, vertex reordering, index sorting, deduplication: no.** The
     output is the concatenation, so two runs on two machines produce the same
     bytes **by construction** rather than by a canonicalisation pass that has to
     be right.
   - **Degenerate-triangle removal: no**, and the reason is the sharpest of the
     seven. **`body` has 6 zero-area triangles out of 704** (three collinear
     triples, indices 296, 308, 593, 594, 654, 655), 18 of 6,096 indices, and they
     rasterise nothing. **Removing them opens the shell**: the body's welded
     edge-valence histogram is `{2: 1056}` with them and `{1: 14, 2: 1040}`
     without — **fourteen boundary edges, fourteen holes** — and task 37 enables
     `GL_CULL_FACE` with `cull_face(GL_BACK)`, so **a hole is visible and a
     watertight shell is not.** The `measured` block records `degenerate_triangles:
     6` so an asset swap that changes the count is noticed.
   - **Winding normalisation: no.** Measured: **2,032 of 2,032** triangles have a
     face normal agreeing with their vertex normals, zero disagree — already
     consistent, and already matching `GL_CCW` + `cull_face(GL_BACK)` as 37 draws
     it under 34's policy. Normalising would change bytes for no gain and would
     make the output depend on a re-orientation rule nothing needs.

   Plus one narrowing the tool must refuse rather than perform: **no `u32` → `u16`
   narrowing on the way out**, and an assertion `max(index) < vertex_count` after
   the merge, which is the arithmetic the prototype already got right and the one
   that would be silently wrong if the base offset were wrong.

   **Every quantity is derived from the whole model. The tool has no subset mode
   and no `--only` flag** — it converts all five meshes or it fails. That is the
   generalisation of the prototype's second bug: *"Camera must be derived from the
   WHOLE model, not the selected subset, or an isolated part is auto-framed to fill
   the frame and no two layers align."* A converter has no camera, so the rule
   that prevents the bug's **class** is to have no way to ask for a part. **And
   because task 38's format carries no bounding box**, `model.json`'s
   `bounds_min` / `bounds_max` and the byte counts are the only description of the
   emitted file's shape — **whole-model quantities, in the one file that records
   them.**
9. **`sedan.roados`, in task 38's format exactly, uncompressed.** **38's byte
   table is the specification and is not restated here** — 38 § *Out of Scope*
   says *"the format is specified so 39 can write it; 39 writing it is not this
   task"*, and 38's requirement 9 puts that same table **verbatim** into
   `render/meshio.rs`'s module doc *"so a writer in task 39 reads one table, not
   two"*. **This requirement therefore names the fields the writer must emit and
   nothing about their layout**, because a second byte table in this repository is
   a second thing to keep honest:

   - **`magic = b"ROADOSMF"`, `version = 1`, `header_bytes = 40`, `flags = 0`,
     `reserved = 0`** — the four values 38's requirement 4 defines and the values
     `meshio::MESH_MAGIC`, `MESH_VERSION`, `MESH_HEADER_BYTES` already carry. The
     tool **imports none of them** — it is Python and `ui_core` is Rust — so
     `model.json` records all four **as literals**, and `check_assets.py` compares
     the emitted file's first 40 bytes against 38's table **and** against
     `model.json`, so a change to either side is a failure rather than a silent
     divergence between a Rust constant and a Python literal.
   - **`vertex_count = 3184`, `index_count = 6096`, `sub_mesh_count = 5`,
     `name_bytes = 73`** — and `name_bytes` is **computed, not written down**:
     `5 + 17 + 18 + 16 + 17`, one NUL-terminated name each in `merge_order`
     (`body`, `wheel-front-left`, `wheel-front-right`, `wheel-back-left`,
     `wheel-back-right`), comfortably inside 38's `sub_mesh_count × 64` ceiling.
   - **The four blocks in 38's order**: interleaved vertices, `u32` indices,
     `(first_index, index_count)` pairs, then the name block. **Little-endian
     throughout, no padding**, written with `struct.pack` on four-byte windows.
   - **No bounding box, no pivot, no texture reference, no node transforms, no
     material, no draw order, no per-sub-mesh colour, no compression** — every one
     of 38's negative list, and each is a value the writer does not emit rather
     than a check the writer performs.
   - **The colormap is a second file and is not embedded**: `colormap.png`,
     **beside the model in `ui/src/ui_demo/assets/`**, per 38's requirement 8. It is
     a **byte copy** of the archive's `Textures/colormap.png`, never re-encoded
     through Pillow, because a re-encode is a byte change for no benefit (see
     *Determinism*). **The format names it nowhere** — 38 is explicit that the
     demo names it, because 37's `DrawCommand::Mesh` already carries
     `texture: TextureId`.
   - **`--print-layout` writes 38's table as `model.json` records it and exits 0**,
     so a reviewer of 38 and a reviewer of 39 diff two files rather than compare
     two prose descriptions with their eyes.
10. **`bake_icons.py` reads `icons.json` and fails on a short set.** Per
    `icons.json`'s `icons` list, per `sizes` entry, per theme: locate
    `<icons-dir>/<file>`, **assert the file exists** (a missing entry is a hard
    failure naming the semantic name, the file it wanted, the pinned tag, and the
    `icons.json` path — **never a skipped icon**), then run the two `resvg`
    invocations of requirement 11.
    **The whole set is written into a temporary directory and only then moved into
    place**, so a run that fails half way leaves the committed set untouched
    rather than short. **After the move, the emitted file set is compared against
    the manifest's expected set and a single missing or extra file is a
    non-zero exit** — `len(icons) × len(sizes) × len(themes)` files, and 33 × 1 ×
    2 = **66** today.
11. **The two `resvg` invocations, exactly.** For one icon at size `S` and theme
    `T`, with `C_T` the hex in `icons.json`:

    ```sh
    resvg --stylesheet "$tmp/dark.css" --width "$S" --height "$S" \
          --shape-rendering geometricPrecision \
          "$icons_dir/<file>" "$out/<name>/dark/<name>.png"
    ```

    ```sh
    resvg --stylesheet "$tmp/light.css" --width "$S" --height "$S" \
          --shape-rendering geometricPrecision \
          "$icons_dir/<file>" "$out/<name>/light/<name>.png"
    ```

    where the two stylesheets are written from the manifest's hexes and contain
    `svg{color:<hex>}` **and** `*{color:<hex>}` — the first because the colour is
    inherited by the shapes, the second because `usvg`'s cascade reaches the
    shapes through the root and this is what was measured.

    **Six decisions in those lines, each with a reason**:

    - **`--width S --height S`, once, at the exact on-screen size.** No `-z`, no
      `--dpi`, no supersample-then-downsample. **1 asset pixel = 1 window pixel**
      (there is no DPI concept in this engine) and `S = 24` is Lucide's own
      `viewBox`, so at `S = 24` the rasteriser performs **no rescale at all** —
      the strongest correctness position available, and the reason `24` is the
      only size this task ships. `GL_LINEAR` min and mag with no mipmaps means a
      shipped asset that is not the on-screen size is wrong on screen and cannot
      be fixed at draw time.
    - **`--shape-rendering geometricPrecision`** is `resvg`'s default, named
      explicitly so that a future `resvg` with a different default is not a
      silent diff across all 66 files.
    - **`*{color:…}`, never `{fill:…}`.** All 33 inputs carry `fill="none"` on
      the **root `<svg>`**, so a `{fill:…}` rule would fill the glyph outlines
      rather than tint the strokes. Requirement 12's input guard is what makes
      this rule safe to state absolutely rather than "usually".
    - **`resvg` is invoked by absolute path**, resolved from `RESVG` in the
      environment if set, else `command -v resvg`, else
      `$HOME/.cargo/bin/resvg`, else a hard error naming all three. **On this host
      `~/.cargo/bin` is not on `PATH`** and `command -v resvg` returns nothing
      while the binary works — so a bare `resvg` fails on a host where it is
      installed, which is the exact shape of a missing-dependency investigation
      that is really a `PATH` one.
    - **`--export-id` is not used.** The runtime atlas is the packer.
    - **Output is RGBA, 8-bit.** The engine premultiplies on load
      (`texture.rs`), and a fully transparent icon is `TRANSPARENT`, one pixel.
12. **The input guard and the value assertion — the two checks that make a
    successful `resvg` run insufficient.**
    Before each invocation, assert of the input SVG: root `viewBox="0 0 24 24"`,
    `fill="none"`, `stroke="currentColor"`, `stroke-width="2"`. **A file that
    fails any of these is refused with the file name and the attribute that
    failed.** This is the guard that makes a third-party icon set unreachable: the
    33 entries are the only inputs that pass, and the shapes are Lucide's.
    After each invocation, assert of the output PNG: `mode == "RGBA"`, size
    exactly `S × S`, **at least one pixel with non-zero alpha** (an empty
    rasterisation is a failure, not a transparent icon), and **the mean of the
    non-transparent pixels within ±2 per channel of the requested hex.**
    **That last assertion is the point of the whole subsection.** Measured means
    were (26.64, 26.64, 26.64) against `#1a1a1a` and (236.16, 239.00, 244.26)
    against `#eceff4` — inside ±0.3 — and **without a stylesheet the mean is
    (0, 0, 0)** with a zero exit status and a valid PNG. **Black icons on the dark
    theme are nearly invisible and would pass every other check in this task**:
    the file count is right, the size is right, the mode is right, and the shape
    is right. Only the value is wrong. Asserting on the artefact's existence is
    what `DEMO_APPLICATION.md` § *Corrections to the second gap table* records as
    having been got wrong twice.
13. **The one miss, and the guard that is really about it.** `icons.json` carries
    the 33 mappings **and** an `unmapped` array recording **`gauge-metric` does not
    exist in Lucide 1.52.0, `gauge` is the substitute, and the reason no other set
    is acceptable.** The mapping is semantic name → Lucide file, never
    semantic name → a glyph that happens to look right, so **`gauge` is the
    semantic name the demo asks for and `gauge.svg` is the file that answers it.**
    The `unmapped` record carries the reason in as many words as the trademark
    section needs: *an absent glyph is the moment an agent reaches for a set that
    has it, and the set that has it is CC0 and therefore looks safe; `simple-icons`
    ships `tesla.svg`.* **That requirement's own 33-entry closed list**, and
    requirement 3's host allow-list, are what make reaching impossible rather
    than discouraged.
    `check_assets.py` additionally **denylists** the strings `tesla`,
    `simple-icons`, `brandfetch`, `worldvectorlogo`, `svglogo` and `wikimedia`
    across `model.json`, `icons.json`, `LICENSES.md` and
    `ui/src/ui_demo/assets/` — with **one documented exemption**, because
    `LICENSES.md` may *name* what it refuses in order to record the decision, and
    a denylist that forbids the record of the refusal forbids the record.
14. **`check_assets.py` — the gate, and it is one command.** `python3
    tools/asset-pipeline/check_assets.py --all` runs, in order, and **stops at the
    first failure with the failing check named**:

    1. `model.json`'s `measured` block against what `glb_to_model.py` reads.
    2. The model file against `sha256sum -c` in a mode that distinguishes
       **byte-stable** artefacts (the `.roados`, the copied colormap) from
       **encoder-dependent** ones (the PNGs), and **says which is which in its
       output** — a gate that quietly treats a PNG hash as a reproducibility claim
       is worse than no gate.
    3. `LICENSES.md` names **both** upstreams, with the ISC notice present
       **verbatim**, and `ui/src/ui_demo/assets/icons/LICENSE` carries the same
       notice.
    4. `grep -ri tesla ui/src/ui_demo/assets/ tools/asset-pipeline/ *.md` returns
       **nothing** beyond the documented exemption of requirement 13.
    5. The icon count: files present == `len(icons) × len(sizes) × len(themes)`,
       **no file in the directory absent from the manifest and none in the
       manifest absent from the directory**, and **no icon file over
       `ATLAS_MAX_IMAGE` in either direction**.
    6. Per-PNG: size, `RGBA`, non-empty, **and the ±2 mean assertion re-run from
       the committed file** — so a hand-edited PNG is caught, not just a bad
       run.
    7. **Determinism**, by the mechanism in the *Determinism* subsection below,
       and `--changed`, which prints
       `changed / added / removed` by name against the committed
       `MANIFEST.sha256`.
15. **What is committed, what is generated, and how a regenerated set is
    reviewed.**
    - **Committed**: `tools/asset-pipeline/` in full (the tools, the manifests,
       the hashes, the README), `LICENSES.md`,
       `ui/src/ui_demo/assets/sedan.roados` and
       `ui/src/ui_demo/assets/colormap.png` — **the two paths task 38's
       `ASSET_MODEL_RELATIVE` and colormap const name, so there is one spelling
       of where the demo looks** — and `ui/src/ui_demo/assets/icons/{dark,light}/`
       (**66 PNGs**), `ui/src/ui_demo/assets/icons/LICENSE`, and
       `ui/src/ui_demo/assets/MANIFEST.sha256` — **one sorted
       `<sha256>  <path>` file listing every generated artefact**, regenerated by
       the tool. **Not committed**: `.asset-cache/`, and nothing else.
    - **The review mechanism, and it is three commands, because a regenerated
      asset set is a diff nobody can read by eye**: `check_assets.py --changed`
      **names** what changed; `MANIFEST.sha256` lets a reviewer confirm that
      everything else did **not**
      (`git diff --stat ui/src/ui_demo/assets/` shows one line per changed file,
      and `sha256sum -c` on the previous manifest passes for the rest); and two
      **contact sheets** make the change visible —
      `magick montage ui/src/ui_demo/assets/icons/dark/*.png -tile 11x -geometry +2+2 -background '#1a1a1a' /tmp/dark.png`
      and the same for `light`, before and after, compared with
      `magick compare -metric AE`. **The contact sheet is the review artefact**:
      66 near-identical 24×24 files in a `git diff` is a diff nobody looks at, and
      one image whose differing region names the icon is one everybody does.
      `README.md` carries all three commands.
    - **`README.md` also records the decisions where the next agent finds them**:
      the vendoring decision and its arithmetic; why `DrawCommand::Image`'s
      `opacity` doc forces one PNG per theme; why there are no mipmaps and
      therefore why 24 is
      the only size; the **`~/.cargo/bin` is not on `PATH`** trap in *Context*'s
      host table; that
      `gauge-metric` is a recorded miss and **not** an invitation; and **that this
      directory produces bytes and is not a runtime dependency of anything.**

### Determinism, as a checkable requirement

**The model file, byte-identical on a second run, in place.** `sha256sum -c
ui/src/ui_demo/assets/MANIFEST.sha256` passes for `sedan.roados` and
`colormap.png`, and then:

```sh
bash tools/asset-pipeline/fetch_upstream.sh                # from cache, no network
python3 tools/asset-pipeline/glb_to_model.py --manifest tools/asset-pipeline/model.json
python3 tools/asset-pipeline/bake_icons.py --manifest tools/asset-pipeline/icons.json \
        --icons .asset-cache/lucide-1.52.0/icons \
        --resvg "$HOME/.cargo/bin/resvg" --out ui/src/ui_demo/assets/icons
git status --porcelain ui/src/ui_demo/assets/             # MUST BE EMPTY
```

**Paths in these commands are relative to the repository root**, and the cache is
`.asset-cache/` at that root — requirement 3's directory, not an environment
variable, so that there is exactly one spelling of where a download lives and it
is the one `.gitignore` carries.

**`git status --porcelain` on the asset tree must be empty.** That is the whole
check and it is stronger than a hash comparison, because it compares the
**committed** bytes against a **fresh run of the tool** — which is the claim
"these files are what the tool produces" rather than the weaker "these two runs
agreed".

**The PNGs, pixel-identical, because their bytes are not guaranteed to be.**
Same command sequence, and then:

```sh
magick compare -metric AE /tmp/dark_before.png /tmp/dark_after.png null:   # 0
magick compare -metric AE /tmp/light_before.png /tmp/light_after.png null: # 0
```

per icon as well as per contact sheet. **The reason for the split is in
*Determinism* above and it is not optional hand-waving**: the `IDAT` stream is
zlib output, zlib is not pinned by anything in this repository, and a byte-hash of
a PNG would be a claim about this host's encoder rather than about the pipeline.
**A tool that hashed PNGs and called it reproducibility would be making a
statement it cannot support**, and the split is recorded so the next agent does
not "tidy" it into one number.

## Acceptance Criteria

- [ ] **The pipeline runs end-to-end from a clean checkout**, with **no network**,
      after exactly one `fetch_upstream.sh`. From `git clone`, `rm -rf
      .asset-cache/`, then: fetch once; run `glb_to_model.py`; run `bake_icons.py`;
      run `check_assets.py --all`; and `git status --porcelain` on
      `ui/src/ui_demo/assets/` **is empty**. The sequence and its output are
      pasted into the handoff
- [ ] **Determinism is checked the two ways this file specifies, and both are
      reported as results.** (a) The model file: `sha256sum -c` against the
      committed `MANIFEST.sha256` passes for `sedan.roados` and
      `colormap.png`, **and** the in-place second run leaves
      `git status --porcelain ui/src/ui_demo/assets/` **empty**. (b) The PNGs:
      `magick compare -metric AE` on the before and after contact sheets reports
      **0** for both themes, and per-icon for all 66. **The handoff states plainly
      that the PNG *bytes* are not the reproducibility claim and the pixel
      comparison is** — it does not report a PNG hash as if it settled something
      it does not
- [ ] **`sha256sum -c` passes against `tools/asset-pipeline/upstream.sha256`.**
      Both archives download, hash to the recorded values
      (`fac7daca…f3d0c4` and `f54137c1…c51034`), and are placed. **A mismatch is
      a stop condition and is reported, not absorbed by editing the file.**
      **The handoff states whether the two hashes this task file carried were
      confirmed or corrected**, since a hash measured on one host on one day is a
      fact about that host and that day
- [ ] **`fetch_upstream.sh` refuses a URL outside its two-entry allow-list**, and
      the refusal message names the offending URL and both permitted ones.
      **Demonstrated in the handoff by pointing it at a third URL**, with the
      non-zero exit pasted
- [ ] **The offline path is demonstrated, not asserted.** `rm -rf .asset-cache/`
      then `fetch_upstream.sh` **with the network unavailable** exits **non-zero**
      naming the URL and `.asset-cache/` — **and no directory tree is left
      behind**. The handoff pastes the message and shows the empty tree
- [ ] **`LICENSES.md` names both upstreams with their licences**, with the
      **ISC notice verbatim** for Lucide — *"Copyright (c) 2026 Lucide Icons and
      Contributors"* plus the permission sentence and the disclaimer — and states
      for Kenney that **CC0 carries no obligation and the credit is optional**.
      `ui/src/ui_demo/assets/icons/LICENSE` carries the same ISC notice, so the
      notice travels with the directory
- [ ] **`grep -ri tesla` over the committed assets returns nothing.**
      `grep -ril tesla ui/src/ui_demo/assets/ | wc -l` is **0**, and the same over
      `tools/asset-pipeline/` is **0** modulo the one documented exemption in
      requirement 13, which the handoff quotes. `check_assets.py`'s denylist
      (`tesla`, `simple-icons`, `brandfetch`, `worldvectorlogo`, `svglogo`,
      `wikimedia`) finds nothing. **And the `colormap.png` has been looked at** —
      the handoff says so and pastes `magick
      ui/src/ui_demo/assets/colormap.png /tmp/colormap.png`: it is a
      grid of flat colour swatches, **fully opaque on every pixel, with no badge,
      no wordmark and no logo**, which is why this asset carries no trademark at
      all. A criterion about a pixel the reviewer has not looked at is not a
      criterion
- [ ] **The generated icon count matches the manifest.** `find
      ui/src/ui_demo/assets/icons -name '*.png' | wc -l` is **66** =
      33 × 1 size × 2 themes, and **the set in the directory equals the set the
      manifest names** — no extra, none missing. Removing one entry from
      `icons.json` and re-running **fails**, and the failure message names the
      semantic name, the file it wanted and `icons.json`. **A run that emits 65
      files is not a pass**
- [ ] **Every baked icon is the right colour, checked on the value.** For all 66
      committed PNGs, the mean of the non-transparent pixels is within **±2 per
      channel** of the manifest hex — `(26, 26, 26)` for dark, `(236, 239, 244)`
      for light. **Mutation evidence:** delete the `--stylesheet` argument from one
      invocation and the assertion fails for that file, naming the measured mean
      against the requested hex — because the no-stylesheet output is `(0,0,0)`
      with a **zero exit status**, and every other check in this task passes on it
- [ ] **Every input passes the root-attribute guard.** Each of the 33 inputs
      carries `viewBox="0 0 24 24"`, `fill="none"`, `stroke="currentColor"` and
      `stroke-width="2"`. **Mutation evidence:** point one entry at a file with a
      plain `fill` and the guard refuses it, naming the file and the attribute
- [ ] **`sedan.roados` is byte-compatible with task 38's loader, and the two
      documents are diffable rather than comparable by eye.** The emitted file's
      first 40 bytes decode, field by field, to `magic = b"ROADOSMF"`, `version =
      1`, `header_bytes = 40`, `vertex_count = 3184`, `index_count = 6096`,
      `sub_mesh_count = 5`, `name_bytes = 73`, `flags = 0`, `reserved = 0` — **the
      same values as `meshio::MESH_MAGIC`, `MESH_VERSION` and `MESH_HEADER_BYTES`**,
      which `check_assets.py` compares against the file and against `model.json`.
      **The strongest check is the loader itself**: the handoff runs
      `meshio::load_from_path` on the emitted file through the demo's own
      `load_model` path and pastes the resulting vertex, index and sub-mesh counts
      — **a green `ui_demo` run that no longer prints *"car model not loaded"*,**
      which is the one sentence 38 made this repository print on purpose.
      `python3 tools/asset-pipeline/glb_to_model.py --print-layout` prints the table
      and the handoff **diffs it against 38's requirement 2**, and the handoff says
      which way any difference went.
- [ ] **Every property task 38's requirement 6 validates holds in the emitted
      file**, because those are the checks a loader will actually run:
      `vertex_count`, `index_count` and `sub_mesh_count` all `> 0`;
      `index_count % 3 == 0` (6,096 = 2,032 × 3); the five sub-meshes are
      **contiguous, non-overlapping and tile the index array exactly**, in
      `merge_order` — `(0, 2112), (2112, 996), (3108, 996), (4104, 996),
      (5100, 996)`, and `2112 + 4 × 996 = 6096`; every `index_count` is itself a
      multiple of 3; every index value is `< vertex_count`; the name block splits on
      `0` into **exactly five** non-empty names with no interior NUL, and
      `name_bytes (73) ≤ sub_mesh_count × 64`; and **there are no trailing bytes
      after the name block**, so 38's requirement 4 cannot raise `Malformed` on the
      file this task writes. `check_assets.py` performs all of these from the
      committed bytes without decompressing anything
- [ ] **The colormap is a byte copy, and the model's `measured` block is a guard.**
      `cmp ui/src/ui_demo/assets/colormap.png '.asset-cache/kenney_car-kit/Models/GLB
      format/Textures/colormap.png'` is silent. **Mutation evidence:** change one
      number in `model.json`'s `measured` block and `check_assets.py` fails naming
      the field and both values — and one number in particular, `degenerate_triangles:
      6`, is the one that fails if a converter starts dropping triangles
- [ ] **The two prototype bugs are demonstrably not repeated, and the evidence is
      `model.json` because task 38's format carries no bounds.**
      (a) **Node transforms are baked**, proven by the per-mesh extents the tool
      measures and writes into `measured`, against the unbaked values: `body`
      `y ∈ [0.150, 1.300]` baked and `y ∈ [0.000, 1.150]` unbaked;
      `body` `z ∈ [-1.300, 1.250]` baked and `z ∈ [-1.275, 1.275]` unbaked;
      `wheel-front-left` `z ∈ [0.360, 0.960]` baked and `z ∈ [-0.300, 0.300]`
      unbaked. **A converter that ignored node translations would emit
      `Y ∈ [-0.30, 1.15]` and `Z ∈ [-0.30, 1.275]`** — the wheels 30 cm under the
      ground and inside the body, exactly as the prototype's comment describes.
      **Two of the six discriminating numbers come from the `body` node, not the
      wheels**: its `[0.0, 0.15, -0.025]` is what puts the roof at 1.30 rather than
      1.15 and leaves **Z asymmetric by 25 mm**. **That asymmetry is preserved and
      asserted, not corrected** — undoing it would cancel the very transform
      requirement 7 exists to bake, and the requirement would look like it worked.
      The handoff pastes the baked and unbaked pairs side by side.
      (b) **No subset mode exists:** `glb_to_model.py --help` has **no `--only`,
      no `--mesh`, no `--part`**, and an unknown flag exits non-zero. The handoff
      says in words that this is the generalisation of the shared-camera bug
- [ ] **The seven conversion-time decisions are in the code as named no-ops with
      reasons, and the measured numbers behind three of them are pasted.**
      `grep -c 'def \|# NO ' tools/asset-pipeline/glb_to_model.py` is reported, and
      the handoff quotes the comments for **no-weld** (168 unique positions from 528
      wheel vertices; 358 from 1,072 for the body; every welded edge at valence
      exactly 2), **no-degenerate-removal** (6 zero-area triangles; histogram
      `{2: 1056}` with them and `{1: 14, 2: 1040}` without — **fourteen holes**),
      and **no-winding-normalisation** (2,032 of 2,032 face normals agreeing with
      their vertex normals). **Mutation evidence:** enable welding and
      `check_assets.py` fails on `vertices: 3184`; enable degenerate removal and it
      fails on `triangles: 2032`, `indices: 6096` **and** on `degenerate_triangles:
      6` — three fields, because the decision has three observable consequences
- [ ] **The six gallery pages are pixel-identical, and the mechanism is stated
      because it is what makes the criterion achievable.** `Page::ALL`'s six
      names, release build, captured **before and after** with the commands of
      `.ai/tools/README.md` § *Capturing a window* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab`, return black
      for a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`. `magick compare -metric AE before.png
      after.png null:` reports **AE 0 outside the fps readout's band `y ≥ 680`**
      and every differing pixel inside it — which
      `.ai/tools/README.md` § *Capturing a window*
      records as the one thing two captures of an unchanged frame differ in. The
      rect-level half is `every_page_places_every_rect_where_the_gallery_placed_it`.
      **The mechanism, and it is two facts rather than the comfortable one.**
      First: the only change under `ui/src/` is **added files** under
      `ui/src/ui_demo/assets/` — no modified source file, no page, no widget, no
      `--tab=` name. Second, and this is the part worth being careful about:
      **once task 38 has landed, this task's model file IS found and loaded.**
      38's `load_model` searches `ASSET_MODEL_RELATIVE` and `ROADOS_ASSET_DIR`, and
      38 deliberately made a missing model non-fatal so the demo would print
      *"car model not loaded — see the log"*. **This task removes that line**: the
      demo will parse the file, upload the mesh and hold a `Model`, on every run.
      **It still draws nothing**, because 38 records that *"no page records a mesh
      command in this task"* and *"the first pixels of a car are task 39's
      capture"* — and **task 39 is not that task**: this one produces bytes, and
      the capture criterion above is exactly the reason a task that loads a model
      must still leave the six pages pixel-identical. **So the expected state after
      this task is: a model loaded and uploaded, zero mesh commands recorded, six
      pages pixel-identical.** The handoff pastes the stderr line that says so, and
      **its absence is the criterion** — a run that still prints *"car model not
      loaded"* means the file is in the wrong place or the demo was not rebuilt.
- [ ] **The frame rate is measured and reported**, with the script's own line
      pasted rather than the number expected: `.ai/tools/fps-check.sh 10 55` on
      the default page, and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo
      --tab=<page>` for each of the six pages with the `roados-fps` line parsed by
      hand — **`fps-check.sh` takes `seconds` then `floor` and runs the binary
      with no arguments**, so it cannot name a page. Every page is above the floor
      of 55 and inside the recorded **61.1–63.9** band in
      `.ai/tools/README.md` § *Frame-rate baseline*. **The expected
      result is no change at all, and the handoff says why rather than treating a
      coincidence as a result**: this task changes **no Rust** and loads nothing,
      so the measurement is for the fact that nothing was accidentally wired in
- [ ] **The suite is green and unchanged.** `cargo test --all-features` in `ui/`
      with the per-binary counts pasted; `cargo fmt --check`,
      `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. **No test deleted, renamed away or weakened.**
      `cargo audit` is not installed on this host; that is **recorded**, not
      passed
- [ ] **No dependency was added, and the workspace is untouched.**
      `git diff --exit-code ui/Cargo.toml ui/Cargo.lock` is clean, the approved
      direct dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`,
      and `find . -name build.rs -not -path './ui/target/*'` finds **none**. **The
      handoff states explicitly, in the file, that the build-time dependencies
      (`python3`+`numpy`+Pillow, the `resvg` binary) sit outside the workspace
      dependency rule rather than inside an exemption from it, and that
      `resvg` 0.48.1 is already installed on this host by `cargo install` outside
      any manifest** — because a reviewer should be able to overrule that, not
      discover it
- [ ] **The review mechanism works on a real change.** `check_assets.py --changed`
      names what changed after a deliberate edit to one icon (re-tint `gauge.svg`
      light from `#eceff4` to `#ff0000`, regenerate, and the report names exactly
      one file); the two contact sheets are regenerated and their `AE` difference
      is reported with the region it falls in. **Mutation evidence:** a regeneration
      that changes nothing reports zero changed, zero added, zero removed, and
      `git status --porcelain ui/src/ui_demo/assets/` is empty
- [ ] **No runtime code appeared, and nothing from task 40 leaked in.**
      `git diff --stat ui/src/` shows **no modified file** under `ui/src/` — only
      additions under `ui/src/ui_demo/assets/`. `git diff --exit-code
      ui/src/ui_core/src/ ui/src/ui_demo/src/` is **clean**: no `Mat4`, no
      `DrawCommand::Mesh`, no `u_model`, no placement transform, no pointer or
      gesture handling, no rotation state. `rg -n 'unsafe|unwrap\(\)|expect\(|panic!|
      unimplemented!|todo!' tools/asset-pipeline/*.py` returns **nothing**, and
      `rg -n 'transmute|static mut' ui/src/` returns nothing.
      **And task 38's test fixture is untouched**: `git diff --exit-code
      ui/src/ui_core/tests/` is clean, because a converter that regenerated
      `ui/src/ui_core/tests/data/sedan.roados` from the GLB would replace a
      hand-built **hostile-input** fixture with a **well-formed** one and delete
      the loader's malformed-case coverage — **a diff of one file that shows
      nothing about what it destroyed**, which is why it gets its own `diff --exit-code`.
      **This task's criterion deliberately says nothing about `std::fs` in
      `ui_core`**: task 37 requires that grep to be empty and task 38 requires it
      to contain `load_from_path`, **one of those two criteria cannot pass once
      both have landed, it is not this task's to settle, and repeating either
      claim here would put a third copy of a contested fact into the repository.**
      The disagreement is reported to the operator in the handoff
- [ ] **The decisions are written down where the next agent finds them.**
      `tools/asset-pipeline/README.md` carries requirement 15's list; `LICENSES.md`
      carries both upstreams; and `doc/ui/IMPLEMENTATION_STATE.md` gains an entry
      recording the vendoring decision, the two hashes, the 66-icon count, the
      single size, the `gauge-metric` miss, the fact that **`sedan.glb` carries no
      badge and no wordmark**, and the fact that **task 38's format is the one
      this pipeline emits** — including, if it differs, which way the difference
      went. **No doc comment in the changed files asserts the opposite of the code
      beside it**, which is the defect `DEMO_APPLICATION.md`
      § *Corrections to the second gap table* records twice

## Out of Scope

- **No runtime code in `ui_core` or `ui_demo`.** Not one line. No `build.rs`, no
  `include_bytes!`, no generated Rust, no loader, no path-taking API, no
  `std::fs` anywhere under `ui/src/`. **Task 38 reads the model file** — in
  `ui_demo`, because task 37's criterion holds that no `ui_core` API takes a file
  path — and this task's only relationship to that is having produced the bytes.
  **Nothing produced by this task runs inside `ui_demo`.**
- **No `resvg` in the build, and no `cargo install`.** `resvg` is a **host tool
  this task invokes**, already present at `~/.cargo/bin/resvg`. The pipeline does
  not build it, does not vendor it and does not add it to any manifest; if a host
  lacks it, `fetch_upstream.sh` and `bake_icons.py` say so with the path they
  looked at. **Installing it is the operator's call and is not assumed**
- **No Blender, and no `.blend` work.** Blender is **not installed** (`apt`
  offers 5.0.1) and is not needed, because **Kenney's Car Kit distributes no
  `.blend` source**. Requirement 3's host allow-list admits two URLs and this is
  not one of them
- **No re-modelling, re-lighting, re-texturing or UV re-authoring.** Every one of
  these needs a modeler and a source file, and the archive has 50 `.fbx`, 50
  `.glb`, one colormap and no `.blend`. **The transform between the upstream and
  this pipeline is exactly: read, bake node transforms, concatenate, write.** The
  lighting is task 37's Lambert term at run time; nothing is baked about light
- **No offline render, no preview renderer, no contact sheet of the car.** Task
  34's table called this task *"offline render and packaging"* and this file takes
  only the **packaging** half — see the narrowing in *Context*. The rasteriser at
  `/tmp/opencode/carproto/render.py` is **not** promoted: it lives in `/tmp`, it
  rasterises, and a pre-rendered PNG of a car is the thing
  `DEMO_APPLICATION.md` § *Open questions* item 5 says will not carry the Layout
  section. **Per-layer output with a shared camera works and composites
  correctly** — that is recorded as a fact about the prototype, not as a feature
  of this pipeline
- **No runtime asset generation.** Nothing is generated when `ui_demo` runs. There
  is no first-run bake, no lazy bake, no shader-side tint. `paint.rs`'s recorded
  `0.5`-is-opacity-not-tint is the reason one baked PNG per theme exists at all,
  and this task does not attempt to work around it
- **No runtime texture pinning.** `ATLAS_MAX_IMAGE = 512` and the shelf eviction
  in `evict_least_recently_used` are facts this task **plans around** — every icon
  is 24×24, so all 66 share the atlas — but `TextureCache::pin` is a **runtime**
  call made by whoever draws an icon. **This task's output is not pinned**, and
  requirement 15 keeps this task out of the runtime
- **No LOD, no instancing, no morph targets, no skinning, no animation.** 2,032
  triangles and one car. Task 37's *Out of Scope* already names each of these as
  a real answer to a real problem this project does not have yet, and none of them
  is a conversion-time decision
- **No compression** — decided, not deferred. 38's requirement 2 assigns it here
  and *Context* answers it: **126,425 B raw, no compressor, and the `flags` and
  `reserved` words left free** for the checksum 38 says they are for. A
  compressor would also be the one build-time dependency whose version could change
  a committed binary asset's bytes, which is the property *Determinism* is built
  on
- **No welding, no tangent generation, no normal recomputation, no smoothing, no
  re-indexing, no degenerate-triangle removal, no winding normalisation** —
  **all seven are decided in *Context*'s table with measured reasons**, and this
  entry exists so a reader does not mistake seven decisions for seven omissions.
  The two with the least obvious answers: **no welding**, because the wheels'
  528 vertices are only **168** positions and the shells are closed, so the
  duplication is split normals; **no degenerate removal**, because the 6
  zero-area triangles cost 0.3 % of the indices and removing them opens **14
  holes** in a shell that task 37's `cull_face(GL_BACK)` would show through
- **No sixth sub-mesh for the glass** — decided, not deferred. 38's *Out of Scope*
  says the decision is 39's; it is **no**, because `DEMO_APPLICATION.md`
  § *Could not verify* records Track Mode's recolouring rules as *"not a spec the
  demo should copy until verified"*, so there is no rule to implement, and because
  guessing which dark swatch in a 512×512 palette is glass would bake an
  unreviewable judgement into a committed file. **38's variable `sub_mesh_count`
  makes it a ten-minute change when a rule exists**
- **No placement and no `Mat4`** — decided, not deferred. 38 says *"where the car
  sits is 39's"*; **37 and 40 say otherwise and 39 has no runtime code, so the
  only reading consistent with all three** is that 39 owns the **object-space
  origin convention** baked into the file (`Y = 0` is the ground plane, `X = 0` the
  centreline, Z deliberately not re-centred) and the demo owns the matrix. The
  `Mat4`, the scale, the on-screen position and every rotation are the demo's and
  task 40's
- **No second model.** One `.glb`, one model file, one colormap. The archive's
  other 49 `.glb` are not candidates here: `DEMO_APPLICATION.md`
  § *Open questions* items 4 and 5 are the operator's, and this task answers
  neither
- **No third icon set and no missing-icon hunt.** `gauge-metric` stays missing and
  `gauge.svg` answers it. **`simple-icons` is CC0 and ships `tesla.svg`** — the
  same trap as the Wikimedia `PD-textlogo` tag, and the reason requirement 13 is
  about the miss and not about the glyph. **No asset from any set other than the
  two pinned upstreams may enter, and the guard is structural rather than a
  review instruction**
- **No decision on the branding question.** `DEMO_APPLICATION.md`
  § *Open questions* item 3 — *"How to handle the Tesla logo and branding?"* —
  **stays open.** What is decided here is only that the pipeline **cannot ingest
  a mark**. The demo is Tesla-*like*, the car is a generic Kenney sedan with no
  badge, and a task that answered the branding question would be a task that
  answered it without being asked
- **No change to the icon set's size policy beyond `[24]`.** The manifest carries
  a `sizes` array with **one** entry, because no page draws an icon in this task
  and the layout's icon sizes are not fixed. **Adding a size is a manifest edit
  whose diff is reviewed like any other** — never an ad-hoc `resvg` invocation,
  because there are no mipmaps and a wrongly sized asset is wrong on screen and
  cannot be fixed at draw time
- **No sprite sheets and no `--export-id`.** `resvg` supports it and the runtime
  atlas is the packer. Two packers would be two layouts to keep honest
- **No change to `ui/src/ui_demo/assets/demo.png`,** to `ASSET_FILE`,
  `ASSET_RELATIVE`, `ROADOS_ASSET_DIR` or `ASSET_SIZE`. **The one committed asset
  the demo already has stays exactly as it is**, and the new assets sit beside it in
  directories no constant names
- **No change to anything under `ui/src/` except additions under
  `ui/src/ui_demo/assets/`.** No modified source file, no new page, no widget, no
  `--tab=` name, no dependency. The six pages must be pixel-identical afterwards,
  and a demo that loaded any of this would make that criterion unverifiable rather
  than merely demanding