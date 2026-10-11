# Asset pipeline

Offline path from two pinned third-party sources to the committed bytes the
demo loads: the pipeline **byte-copies** Kenney's `sedan.glb` and its
`colormap.png` into the demo's `assets/`, and bakes 33 Lucide SVGs into 66
baked PNGs — one per theme, at the exact on-screen size.

**There is no model converter.** `ui_core::render::gltf` reads the GLB directly
(task 53), keeping each node's transform and applying it at draw time, so the
committed model is the upstream file itself and the only thing the pipeline does
to it is copy it. The old project-owned model format, its converter, its
manifest and the `.roados` artefacts are gone.

Nothing here runs inside `ui_demo`. There is no `build.rs`, no runtime
generation, no code path from these files into a frame. This directory
produces bytes and is not a runtime dependency of anything.

## The three commands, in order

Paths are relative to the repository root:

```sh
bash tools/asset-pipeline/fetch_upstream.sh
python3 tools/asset-pipeline/bake_icons.py --manifest tools/asset-pipeline/icons.json
python3 tools/asset-pipeline/check_assets.py --all
```

`fetch_upstream.sh` downloads the two archives into `.asset-cache/` (skipping
what is already there with the right hash, so the second run is offline) and
extracts them. `bake_icons.py` writes the 66 PNGs under
`ui/src/ui_demo/assets/icons/`. `check_assets.py --all` re-derives every claim
from the committed bytes and the cache, and stops at the first failure naming
it. The model and the colormap are copied from the cache by hand (or by the
same `fetch_upstream.sh` plus a `cp`); `check_assets.py --all` verifies the
committed copies are byte-identical to the cached upstream files.

## Host tools: what must be installed, and why

These are dependencies of the *build*, not of the *workspace*: nothing here
is a member of the `ui` workspace, appears in any `Cargo.toml`, or is linked
into `ui_core`/`ui_demo`. `ui/Cargo.toml` and `ui/Cargo.lock` are untouched
by this pipeline, and that is an acceptance criterion, not a hope. If you
are reviewing dependency policy, that is the sentence to overrule — not a
line in a manifest, because there is none.

| Tool | Pinned / seen version | Why it is needed | How to install | What breaks without it |
|---|---|---|---|---|
| `python3` | 3.13 (any 3.10+ works; scripts use stdlib + two packages only) | The pipeline's language: PNG assertions and the gate | System package | Nothing runs |
| `numpy` | 2.x (`pip install --user numpy`) | The pixel means behind the icon colour assertions | `python3 -m pip install --user numpy` | `bake_icons.py` and `check_assets.py` refuse with a message naming it |
| `Pillow` | 12.x (`pip install --user pillow`) | The PNG writer behind the value assertions and the committed-file re-checks; the colormap is deliberately *not* round-tripped through it (byte copy) | `python3 -m pip install --user pillow` | `bake_icons.py` and `check_assets.py` refuse with a message naming it |
| `resvg` | **0.48.1** | Rasterises **and tints** the SVGs. Without a stylesheet its output is valid, black, exit-0 — which is why the colour-mean assertion exists | `cargo install resvg --version 0.48.1`; the binary lands at `~/.cargo/bin/resvg`, outside this repository | `bake_icons.py` exits non-zero naming every path it tried (`$RESVG`, `PATH`, `$HOME/.cargo/bin/resvg`) |
| `curl` | any | The fetcher (`-sSfL --retry 3`) | System package | `fetch_upstream.sh` cannot download; with a warm cache it is never called |
| `magick` | any | Review-only: contact sheets and `compare -metric AE`. Not part of the pipeline and never a gate | System package | No contact sheets; `check_assets.py` does not need it |

The `resvg` path is a trap worth naming: `~/.cargo/bin` is not on `PATH`
in a plain shell, so `command -v resvg` returns nothing while the binary
works by absolute path. `bake_icons.py` resolves `$RESVG`, then `PATH`,
then `$HOME/.cargo/bin/resvg`, else a hard error — a bare `resvg` would
fail on a host where it is installed and send the next reader looking for
a missing dependency. This host provisions it with
`cargo install resvg --version 0.48.1` (seen: 38 s compile); a fresh clone
needs the same one-time install per machine.

`resvg` 0.48.1 reached this host by `cargo install`, so its Rust source
sits in the shared cargo registry outside any manifest. Recorded, not
endorsed: it is a host tool like `curl`, not a workspace dependency.

## Decisions, where the next agent finds them

- **Vendoring: not vendored, fetched on demand, verified by hash.**
  4.8 MB + 7.3 MB of upstream against ~230 KB of committed output
  (172,216 B model + 12,371 B colormap + ~45 KB icons). A hash is a
  stronger guarantee than a blob (a vendored archive can be re-zipped
  silently); git keeps deleted blobs forever, so dropping archives later
  would not reclaim the cost. The build never needs the network — only
  *regenerating* does — and `.asset-cache/` makes the second run offline.
- **The model is the upstream GLB, byte for byte.** `ui_core::render::gltf`
  reads it at runtime and the demo applies each node's translation at draw
  time, so no offline translation exists to be wrong. `check_assets.py`
  verifies the committed `sedan.glb` and `colormap.png` are byte-identical to
  their cached upstream files.
- **One PNG per theme, baked, because tinting at draw time is unreachable.**
  `IMAGE_FRAGMENT_SHADER_SRC` ends `texel.rgb * opacity` — a scalar, never
  a colour — and `DrawCommand::Image`'s `opacity` doc says `0.5` is *the
  image, half as present*, not *tinted*. Text is tintable (per-vertex
  colour); images are not. Hence two baked sets, tinted by resvg
  stylesheets written from the manifest hexes (`#1a1a1a` dark, `#eceff4`
  light), and `*{color:...}` rather than `{fill:...}` because every input
  root carries `fill="none"`.
- **24 px is the only size, because there are no mipmaps and no DPI.**
  `MIN_FILTER` is `GL_LINEAR`, `generate_mipmap` is never called, and
  1 asset pixel is 1 window pixel. 24 is Lucide's own viewBox, so the
  rasteriser performs no rescale at all. Adding a size is a manifest edit
  reviewed like any other, never an ad-hoc invocation.
- **The colour assertion is coverage-weighted by alpha.** A flat mean over
  painted pixels fails `plug-zap` dark ((28.11, ...) vs 26 — straight-RGBA
  quantisation biases faint edge pixels bright) while the icon is the
  right colour (weighted: 26.73). Weighting counts each pixel as the
  compositor sees it; a missing stylesheet still fails loudly (0 vs 26).
- **The icon list has 33 entries; the task text enumerates 34 names.**
  `TASK_UI_PRIM_39` states "33" eight times with the arithmetic 33 x 1 x 2
  = 66, and lists 34 names once (the extra is `circle-dot`). The
  arithmetic wins: 33 entries, 66 files. If `circle-dot` belongs, adding
  it is a manifest edit, not a re-read of this paragraph.
- **`gauge-metric` is a recorded miss, not an invitation.** It does not
  exist in Lucide 1.52.0; `gauge.svg` answers the gauge semantic, and
  `icons.json`'s `unmapped` array records why no other set is acceptable.
- **Lucide's licence is ISC *plus* MIT.** The task text says ISC; the
  archive's `LICENSE` carries an MIT section for Feather-derived icons,
  several of which are baked here. Both halves are quoted in
  `LICENSES.md` and both travel in `icons/LICENSE` (a verbatim copy).

## Reviewing a regenerated set

A regenerated asset set is a diff nobody can read by eye. Two commands:

```sh
git diff --stat ui/src/ui_demo/assets/                   # one line per changed file
magick montage ui/src/ui_demo/assets/icons/dark/*.png -tile 11x -geometry +2+2 -background '#ffffff' -type TrueColorAlpha /tmp/dark.png
magick montage ui/src/ui_demo/assets/icons/light/*.png -tile 11x -geometry +2+2 -background '#121212' -type TrueColorAlpha /tmp/light.png
magick compare -metric AE /tmp/dark_before.png /tmp/dark_after.png null:
```

Two corrections to these commands are load-bearing, and both were found
by running them rather than reading them:

- **Each set is mounted on the surfaces it serves, not on its own ink.**
  The `dark` set is dark ink for the light theme's white surfaces, the
  `light` set light ink for the dark theme's near-black ones (the set is
  named for the theme whose *text* shares its brightness). The task text's
  backgrounds (`#1a1a1a` under dark ink, `#eceff4` under light ink) match
  the ink, not the surface, and both render blank sheets — measured
  all-(26) and all-(239). A blank sheet cannot name a changed icon, so the
  commands above use `#ffffff` (light `Background`) and `#121212` (dark
  `Background`, 18,18,18).
- **`-type TrueColorAlpha` works around a montage canvas quirk.** All 33
  dark icons are true-gray pixels (R=G=B), so a montage of only dark
  icons builds a grayscale canvas whose alpha compositing drops every
  tile: a one-file and a 33-file dark montage both render pure white,
  while adding a single non-gray tile "fixes" the whole sheet (verified
  tile by tile). The committed PNGs are valid TrueColorAlpha that Pillow,
  `convert`, `flatten`, `append` and the engine's loader all read
  correctly — only `montage`'s all-gray fast path misreads them, and the
  flag forces the RGB canvas it should have built.

The contact sheet before and after makes the change visible — 66
near-identical 24x24 files in a `git diff` is a diff nobody looks at, and
one image whose differing region names the icon is one everybody does.

## Files

- `README.md` — this file: the three commands and the decisions.
- `fetch_upstream.sh` — pinning, fetching, the offline path, the
  two-URL allow-list (the trademark guard at the network edge).
- `upstream.sha256` — the two recorded hashes; a mismatch is a stop
  condition, not a warning.
- `icons.json` — the icon manifest: 33 mappings, one size, two hexes,
  and the `unmapped` record.
- `bake_icons.py` — the icon baker. Input guard, two `resvg`
  invocations per icon, value assertions, atomic move into place.
- `check_assets.py` — the gate (`--all`).

The model artefacts (`sedan.glb`, `colormap.png`) have no script: they are
copied from the extracted cache. `check_assets.py --all` is what makes that a
verifiable claim rather than a convention.

No executable bit is required on any of these: every invocation is
`bash ...` or `python3 ...`, so the command line in a reviewer's shell is
the command line that ran.

## The airbag master (TASK_UI_DEMO_02.1, 2026-10-11)

One fill-based SVG outside the Lucide set —
`ui/src/ui_demo/assets/icons/airbag-svgrepo-com.svg` — baked to two white
masters beside it (`airbag_24.png`, `airbag_48.png`). The demo loads the 24 px
one for the status bar's airbag badge and tints it at draw time (`TASK_UI_PRIM_44`'s shader tint), so there is one file
per size and no per-colour variant. Exact commands, run from the repository
root (`~/.cargo/bin` is not on `PATH`, so `resvg` is the absolute path):

```sh
printf 'svg{fill:#ffffff}*{fill:#ffffff}' > /tmp/airbag_white.css
/home/dev/.cargo/bin/resvg --stylesheet /tmp/airbag_white.css \
  --width 24 --height 24 --shape-rendering geometricPrecision \
  ui/src/ui_demo/assets/icons/airbag-svgrepo-com.svg \
  ui/src/ui_demo/assets/icons/airbag_24.png
/home/dev/.cargo/bin/resvg --stylesheet /tmp/airbag_white.css \
  --width 48 --height 48 --shape-rendering geometricPrecision \
  ui/src/ui_demo/assets/icons/airbag-svgrepo-com.svg \
  ui/src/ui_demo/assets/icons/airbag_48.png
```

Why `{fill:…}` and not the pipeline's `*{color:…}` rule: the root carries
`fill="#000000"` and no stroke, so a `color` rule changes nothing and resvg
exits 0 with a valid black PNG (measured coverage-weighted mean 0.0 on the
24 px bake). The guard is the pixels, not the exit status: RGBA at the exact
size, corner alpha 0 (the SVG holds no background element — three paths, no
`<rect>`), coverage-weighted mean ≈ 255 on all channels (white, never 0),
and at least one pixel with `0 < alpha < 255` (anti-aliasing survived; resvg
stores straight RGBA and the bake must not threshold it). Measured on the
committed masters: 24 px — 301/576 painted, mean 255.0, 177 edge pixels;
48 px — 1031/2304 painted, mean 255.0, 356 edge pixels.

`check_assets.py --all` does not cover these files: it reads
`tools/asset-pipeline/icons.json`, which is absent from this tree, and stops
at `check_licences` (missing `assets/icons/LICENSE`) before reaching any
icon check — the same failure with and without the two PNGs. No manifest
was amended because there is none covering them.
