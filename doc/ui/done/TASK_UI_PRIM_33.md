# TASK_UI_PRIM_33: Set the SDL Options No Cargo Feature Can Reach

## Goal

Put SDL's build configuration in agreement with the project's recorded
decisions, for the options that `Cargo.toml` cannot express.

## Context

`PRIMITIVES_ARCHITECTURE.md` § *Dependencies* and `PRIMITIVES.md` § *Backend*
agree that this project renders with **OpenGL ES 3.1 through EGL**, and
`PRIMITIVES.md` records Vulkan as **rejected** on the merits. The build did not.

Three SDL options were found **on** in a default configure, and none of them is
reachable from a cargo feature:

- **`SDL_VULKAN`** — a plain `dep_option` at `CMakeLists.txt:371`, default
  **ON**, and `CheckVulkan` (`sdlchecks.cmake:928-938`) probes nothing: it sets
  `SDL_VIDEO_VULKAN 1` and returns. `PRIMITIVES.md` § *Backend* rejects Vulkan,
  and the same paragraph records that **SDL's GPU API does not support OpenGL ES
  at all** — only Vulkan, D3D12 and Metal — while this project uses
  `SDL_GL_CreateContext` and its own GLES 3.1 pipeline. So both the Vulkan
  surface code and the GPU subsystem's only viable backend are compiled and
  unusable.
- **`SDL_OPENGL`** — desktop OpenGL through GLX. Default **ON**, and it was
  `ON` in both `CMakeCache.txt` files. It came out undefined only because
  `FindOpenGLHeaders()` could not compile `<GL/gl.h>` on this host: **`libgl-dev`
  is not installed.** Install that package and a native build silently grows GLX
  code that nothing uses and no test notices — a successful build of a different
  artefact, the same failure shape `CROSSBUILD.md` §6.7 records for a missing
  `pkg-config`.

  **This is the option with the most to prove**, because turning it off must not
  break the GLES 3.1 context the whole project renders through. The argument for
  that is at the **C level**, not the CMake level — and an earlier draft of this
  file got it wrong by claiming `CheckOpenGL` and `CheckOpenGLES` have "no
  dependency between them". They are coupled in the one macro that matters:
  `CheckEGL` opens `if(SDL_OPENGL OR SDL_OPENGLES)` (`sdlchecks.cmake:861-862`).
  The outcome survives, because `SDL_OPENGLES` stays on and the branch still
  runs, but that is the fact to cite. The C-level evidence is in
  `cmake/sdl-options.cmake`'s header: `SDL_VIDEO_OPENGL_EGL` gates
  `SDL_egl.c:23` and the X11 driver's GLES entry points
  (`SDL_x11video.c:217-233`), and with `SDL_VIDEO_OPENGL_GLX` undefined the
  `SDL_HINT_VIDEO_FORCE_EGL` test inside it never runs, so **EGL becomes
  unconditionally preferred** rather than merely unaffected.
- **`SDL_TEST_LIBRARY`** — default **ON** (`CMakeLists.txt:399`), builds a
  static `SDL3_test` nothing in this workspace links, and runs
  `CheckLibUnwind`: a three-stage compile/link/`pkg-config` probe for a test-only
  dependency whose result is `HAVE_LIBUNWIND_H` as a **PRIVATE** definition on
  `SDL3_test`, so it cannot appear in the library's `SDL_build_config.h`
  whatever it finds.

**Why no cargo feature reaches them.** `sdl3-sys` 0.7.1's `cmake_vars!` block
(`build.rs:30-48`) covers the twelve subsystems plus `SDL_ASAN`, `SDL_CCACHE`,
`SDL_LIBC`, `SDL_RPATH` and `SDL_UNIX_CONSOLE_BUILD` — nothing else — and `sdl3`
0.20.0 re-exports only `sdl-unix-console-build` of those. There is no
`no-sdl-vulkan` or `no-sdl-opengl` feature to name on either crate, and no
`CMAKE_ARGS` escape hatch. The environment is the only channel, and
`cmake` 0.1.58 reads `CMAKE_TOOLCHAIN_FILE` from it for any target.

## Requirements

1. **A CMake file carrying the three options, and no target identity.**
   `cmake/sdl-options.cmake`, setting `SDL_VULKAN`, `SDL_OPENGL` and
   `SDL_TEST_LIBRARY` to `OFF`. It must set no compiler, no `CMAKE_SYSTEM_NAME`,
   no `-march` and no sysroot default — that is what lets one file serve both
   targets. Its header states the reason for each option individually.
2. **The same options on both targets.** `cmake/aarch64-toolchain.cmake`
   `include()`s it; the native build reads it directly. Override discipline
   matches the existing file: `-D` wins, then `ROADOS_SDL_*` from the
   environment, then the default.
3. **Reach the native build without a toolchain file being mandatory.** The
   `cmake` crate probes `CMAKE_TOOLCHAIN_FILE_<triple>`,
   `CMAKE_TOOLCHAIN_FILE_<triple underscored>`, then
   `TARGET_CMAKE_TOOLCHAIN_FILE` when cross-compiling or
   `HOST_CMAKE_TOOLCHAIN_FILE` when not, then the bare name. Set the two
   kind-scoped names in `.cargo/config.toml` § `[env]`. **Do not set the bare
   name**: an aarch64 build with the toolchain file forgotten would then fall
   back to a file that sets no `CMAKE_SYSTEM_NAME`, and SDL would configure with
   the host compiler and produce x86-64 objects under an aarch64 target
   directory. `CROSSBUILD.md` §4.1 carries the full resolution order and §4.1.1
   the `cargo clean -p sdl3-sys` step this requires.
4. **Document, in `CROSSBUILD.md`, what the options do and what they cost** — a
   new section with the greps that verify them, the measured before/after, and
   the note that five `*vulkan*.o` files remain and are empty translation units
   (`nm --defined-only` returns nothing for each).
5. **Do not change the subsystem policy.** The twelve subsystems stay at SDL's
   defaults. That is a separate decision, recorded in
   `CROSSBUILD.md` §5.2.1, and this
   task is not where it is revisited.

## Acceptance Criteria

- [ ] `grep -E '^SDL_(VULKAN|OPENGL|TEST_LIBRARY):' <build>/CMakeCache.txt` shows
      `OFF` for all three, **on the native build**
- [ ] The same three are `OFF` **on the aarch64 build**, and the artifact is
      still `ELF 64-bit … ARM aarch64`
- [ ] `SDL_X11`, `SDL_WAYLAND` and `SDL_UNIX_CONSOLE_BUILD` are **unchanged** —
      the target keeps X11 off and the console build on
- [ ] `SDL_VIDEO_OPENGL_EGL` and `SDL_JOYSTICK_HIDAPI` are still `1` — this task
      changes three options and must not disturb the GLES path
- [ ] `libSDL3_test.a` is not built
- [ ] `nm --defined-only` on every remaining `*vulkan*.o` reports zero symbols
- [ ] `cargo test` in `ui/` is green, and `.ai/tools/fps-check.sh` reports no
      frame-cost regression against the recorded baseline
- [ ] `CROSSBUILD.md` carries the option list, the verification greps, and the
      measured sizes labelled by artifact

## Out of Scope

- **The twelve subsystems.** Unchanged, deliberately — §5.2.1.
- **The X11 extension libraries** (`libxcursor-dev` and friends). A separate
  problem with a separate cost, and *not* solved by this task: those options are
  reachable from neither crate. `IMPLEMENTATION_STATE.md` carries it.
- **`SDL_KMSDRM`.** Left alone on purpose — it needs target `libdrm` and `gbm`
  `.pc` files, and the target image is undecided.
  `doc/platform/TASK_CROSSPLATFORM_02.md` owns it.
- **A second direct `sdl3-sys` dependency.** The operator chose the toolchain
  route, which needs no dependency and therefore no dependency approval.
- **Stripping the binary.** The operator declined it 2026-10-05: 825 KB of
  `.symtab`/`.strtab` is kept deliberately, for a usable head-unit backtrace.
