# Crossbuild environment

Native x86_64 Linux and cross-compiled aarch64 Linux, both GLES 3.1.
Dependency versions live in `PRIMITIVES_ARCHITECTURE.md` § *Dependencies*
only: SDL 3.4.16 via `sdl3-sys` 0.7.1, `sdl3` 0.20
(`build-from-source`, `build-from-source-static`,
`build-from-source-unix-console`, + `image` on `ui_core`), `glow` 0.18,
`freetype-rs` 0.38 (`bundled`), `gltf` 1.4.1. History and measurement logs
of the 2026-09/10 bring-up were removed here to meet the 10 KB cap; git
history of this file is the archive.

## 1. Prerequisites

- Rust `>= 1.85` (`sdl3-sys` floor); `rustup target add
  aarch64-unknown-linux-gnu` for cross builds.
- `cmake >= 3.16`, C compiler + `make`. `ninja` optional.
- `pkg-config`: silent gate (§4). Native loss = no KMSDRM locally; target
  loss = head unit with no scanout driver. Target needs
  `aarch64-linux-gnu-pkg-config` + `libdrm`/`gbm` for aarch64 in the
  sysroot — hard requirement.
- Native X11 `-dev` (`libxcursor-dev libxrandr-dev libxss-dev` plus base
  `libx11-dev libxext-dev libxi-dev`) is dev-host only. SDL links them via
  `dlopen` (`SDL_DEPS_SHARED=ON`), so they add no link dependency; EGL/GLES
  headers ship inside SDL (`src/video/khronos/`), no system EGL needed.
- Cross toolchain: `gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
  binutils-aarch64-linux-gnu`. The `binutils` package provides
  `aarch64-linux-gnu-gcc-ar`, needed only by the IPO archive rule; plain
  builds never consult it. Late failure, not bring-up failure.

## 2. Commands

Native (workspace root is `ui/`):

```sh
cargo build
cargo build --release
cargo test
```

Cross (repository root; `--manifest-path` because the workspace is `ui/`):

```sh
cargo build --target aarch64-unknown-linux-gnu --manifest-path ui/Cargo.toml
```

Linker comes from `.cargo/config.toml`
(`linker = "aarch64-linux-gnu-gcc"`); without it rustc drives host `cc`
and the link dies on `--fix-cortex-a53-843419`. Tuning/defaults:
`ROADOS_ARCH_FLAGS` (default `-march=armv8-a`),
`ROADOS_SYSROOT` (unset = Debian cross layout, not a sysroot; set only for
a real sysroot tree). Always pass `--target`; without it the build is a
quiet native build.

Check the artifact, not the exit status:

```sh
file target/aarch64-unknown-linux-gnu/debug/ui_demo
readelf -h target/aarch64-unknown-linux-gnu/debug/ui_demo | grep -E 'Class|Machine'
```

Expect `ELF64` / `AArch64`. An `x86-64` answer means the host compiler ran.

## 3. How the toolchain file reaches SDL

No `-D` passthrough exists (`cmake` 0.1.58 has no `CMAKE_ARGS`); the
environment is the only channel. Probe order: target-scoped names first,
then kind-scoped (`TARGET_…` cross / `HOST_…` native — kind first), then
bare `CMAKE_TOOLCHAIN_FILE`. `.cargo/config.toml` sets the kind-scoped
pair, so plain builds just work: native reads `cmake/sdl-options.cmake`,
cross reads `cmake/aarch64-toolchain.cmake` (which includes it). The bare
name stays unset on purpose — otherwise a forgotten export silently builds
x86-64 objects under an aarch64 directory. An explicit
`CMAKE_TOOLCHAIN_FILE_aarch64_unknown_linux_gnu` export still wins as an
override.

`cargo clean -p sdl3-sys --manifest-path ui/Cargo.toml` after editing any
`cmake/*.cmake` or `.cargo/config.toml`: `sdl3-sys` emits no
`rerun-if-changed`, so edits otherwise do nothing and report success.

`ROADOS_SYSROOT`, when set, becomes `CMAKE_SYSROOT` + `CMAKE_FIND_ROOT_PATH`
(+ target `PKG_CONFIG_*` paths). `CMAKE_FIND_ROOT_PATH_MODE_*` alone is not
host isolation — measured host-X11 leak stands; what keeps wrong-arch libs
off the link is `SDL_DEPS_SHARED=ON`. `CMAKE_LIBRARY_ARCHITECTURE` cannot
be set from a toolchain file (`project()` overwrites it); the real compiler
supplies the right value.

## 4. SDL configuration

- Subsystems: all twelve on, SDL defaults (operator 2026-09-28; two of the
  proposed disables were product requirements — audio, camera — filesystem
  has no switch, render is unused). `sdl3` re-exports no subsystem feature;
  trimming needs a second direct `sdl3-sys` dependency. Deliberately
  undecided: nothing.
- Shared off on both targets (`cmake/sdl-options.cmake`, unreachable from
  Cargo): `SDL_VULKAN`, `SDL_OPENGL`, `SDL_TEST_LIBRARY`. GLES is
  unaffected (`CheckOpenGLES` hangs off `SDL_OPENGLES`; EGL preferred with
  GLX off). Verify with the three greps (cache, generated header, no
  `*vulkan*.o` under `SDL3-static.dir`).
- Video split (design, not oversight): native keeps X11 (demo needs a
  window); target is framebuffer-only — `SDL_X11=OFF`, `SDL_WAYLAND=OFF`
  from the toolchain file (unreachable from Cargo), plus
  `build-from-source-unix-console` in the manifest (the one SDL option
  `sdl3` re-exports; suppresses the no-X11-no-Wayland `FATAL_ERROR`).
  `SDL_KMSDRM` defaults ON; it activates only with target `libdrm`/`gbm`
  `.pc` files, i.e. with the sysroot (§6). Target needs no X11 packages:
  `CheckX11()` never runs with `SDL_X11=OFF`.

Silent trap: `CheckKMSDRM` has no `else()` — missing `pkg-config` or
`libdrm`/`gbm` succeeds with `/* #undef SDL_VIDEO_DRIVER_KMSDRM */`.
`(Wanted: ON): OFF` means option on, driver not found. Diagnose via
`SDL_build_config.h` (`SDL_VIDEO_DRIVER_*`, `SDL_AUDIO_DRIVER_*`), not the
exit code.

## 5. Bench hardware

CM4 wireless (eMMC) on CM4IO + Waveshare 15.6" 1920x1080 HDMI + USB-HID
touch (`doc/platform/TARGET_PLATFORM.md`). SoC BCM2711 / VideoCore VI,
open V3D GLES 3.1 — inside the `IDEA.md` GPU filter. Bench panel is
driverless HDMI+HID; the industrial car panel keeps the same interface, so
the KMSDRM path configured here does not change. UI renders logical
(1280x720 dp) scaled to physical; a panel swap is EDID + touch matrix only.

## 6. Troubleshooting

- `edition2024 required` (names `sdl3-ttf-sys`): toolchain older than 1.85.
  `rustup update stable`. Red herring crate; floor comes from `sdl3-sys`.
- Cross compiler not found: install §1 toolchain; confirm
  `aarch64-linux-gnu-gcc -dumpmachine`.
- `CMAKE_C_COMPILER_AR-NOTFOUND` at archive step: install
  `binutils-aarch64-linux-gnu`. `CMAKE_AR` is a different variable and does
  not help; hand-setting `COMPILER_AR` hides the cache entry — last resort.
- `Couldn't find dependency package for XCURSOR/...`: native only —
  install X11 `-dev` or `-DSDL_X11_X<NAME>=OFF`.
- Builds, no video driver on device: §4 trap — target `pkg-config` +
  `libdrm`/`gbm` in sysroot.
- `could not find X11 or Wayland`: cross without sysroot needs
  `SDL_UNIX_CONSOLE_BUILD` (already in manifest) or a sysroot.
- `sdl3-sys` panic `cmake dir not found`: stale `OUT_DIR`; `cargo clean -p
  sdl3-sys` and rebuild.
- Vendored source: `~/.cargo/registry/src/*/sdl3-src-3.4.16/SDL/`.

## 7. Open items

Strategy decided 2026-10-16 (`TASK_CROSSPLATFORM_01` req 1): Pi OS Lite 64-bit,
pinned release, bench-only and revisable; SSH provision + app deployer, no image
baking; sysroot extracted from the same release's arm64 debs; boot target under
7 s power-to-first-touch-responsive-frame. Remaining gate: sysroot build, target
`libdrm`/`gbm` (activates KMSDRM), runtime audit, smoke test on the CM4.

## 8. Old-section map

Compacted 2026-10-16 from 118 KB; older citations resolve here: §1 chain →
intro; §2.2 `pkg-config` → §1 + §4 trap; §2.3 X11 dev, §2.4 EGL headers,
§2.5 toolchain → §1; §3 native, §4.2 cross commands → §2; §4.1/4.1.1/4.3
wiring → §3; §5.1 subsystems, §5.2 filesystem, §5.5 shared options, §5.4
video split → §4; §6.7 KMSDRM finding → §4 trap; §6 rest → git history;
§7 → §6; §8 → §7.
