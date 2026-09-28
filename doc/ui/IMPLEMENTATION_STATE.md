# UI primitives — implementation state

**Purpose:** so a fresh session resumes at the right task and does not redo
finished work. This is working state, not a spec: the task files own the
requirements, and where this file and a task file disagree, the task file wins
and this file gets corrected.

**Spec:** `doc/ui/PRIMITIVES.md`, `doc/ui/PRIMITIVES_ARCHITECTURE.md`, and
`doc/ui/TASK_UI_PRIM_01..24.md`.

**Last updated:** 2026-09-28

## Current position

**Status: 01 done and committed** — operator commit `2f27127`, 2026-09-28,
"doc/ui/TASK_UI_PRIM_01.md done". Both blockers that held 02 are resolved: the
X11 development headers are installed, and the toolchain is 1.98.1.

**Current task: 02 — Project Scaffolding.** It is the first task that writes
Rust source, so it also establishes the repository's conventions.

## Ratified by the operator (2026-09-28)

- The 24 task files are the **confirmed spec**. The
  `.ai/workflows/idea-to-code.md` stage 1 gate is satisfied by the operator for
  the whole sequence — no `idea-evaluator` pass per task. Work enters at
  stage 3, and the reviewer checks the change against the task file, not
  against whether the task was the right idea.
- **An acceptance criterion that cannot be verified on this machine is waived
  with a recorded reason**, not silently dropped and not treated as a blocker.
  It goes in the `AC waived` column and its reason goes in *History*, so a
  waived criterion is never later mistaken for a verified one.
- The operator commits **per task**, and the sequence stops at every task for
  operator review and commit approval. No task starts before the previous one
  is committed.
- **`rustup update stable` executed by the operator, 2026-09-28.** Toolchain is
  now `cargo`/`rustc` 1.98.1. See *Resolved* below.
- **SDL3 builds with its default subsystems, for now.** The operator's decision,
  taken knowing the alternative is blocked. This *contradicts*
  `PRIMITIVES_ARCHITECTURE.md`, which specifies audio, render, camera and
  filesystem disabled at build time — see *Deviations*. Revisit when the
  head unit's audio and rendering needs are known.
- **Native keeps X11, the target build drops it entirely.** The operator's
  decision, 2026-09-28. The dev host runs the task 24 demo in a real window, so
  it needs a windowing driver; the head unit must carry no desktop stack. The
  split is enforced asymmetrically because no single mechanism can serve both —
  see *The native/target X11 split*.
- **aarch64 target libraries are deferred until the target image is decided.**
  The operator's decision, 2026-09-28. Native builds proceed and stay verified;
  aarch64 remains a documented waiver. No sysroot strategy is committed to yet.

## The native/target X11 split

Decided 2026-09-28. Both halves are forced by mechanism, not preference.

**Host, native build — install `libxcursor-dev libxrandr-dev libxss-dev`.**
X11 is the only windowing driver available on this host (the native configure
gave `Video drivers: dummy kmsdrm offscreen x11(dynamic)`, and Wayland's
development libraries are absent), so it is what makes the demo visible.

**Target, cross build — `SDL_X11=OFF SDL_WAYLAND=OFF`, and no X11 packages at
all.** Verified, not assumed: `CheckX11()` opens with `if(SDL_X11)` and
`find_package(X11)` and all nine extension probes are nested inside it, so with
the option off the X11 dev packages are never looked for.

**Installing them introduces no link-time X11 dependency.** `SDL_DEPS_SHARED`
defaults ON (`SDL/CMakeLists.txt:234`) and `SDL_X11_SHARED` defaults ON gated on
`SDL_X11;SDL_DEPS_SHARED` (`:341`), so `cmake/sdlchecks.cmake:398` takes the
dynamic branch and records the soname:

```cmake
if(HAVE_X11_SHARED)
  set(SDL_VIDEO_DRIVER_X11_DYNAMIC_XCURSOR "\"${XCURSOR_LIB_SONAME}\"")
else()
  sdl_link_dependency(xcursor LIBS X11::Xcursor ...)
endif()
```

SDL `dlopen`s it at runtime. The Rust binary never links X11, and `glow` resolves
GL through `SDL_GL_GetProcAddress`.

**Enforcement is asymmetric, necessarily.** `SDL_X11`/`SDL_WAYLAND` are not
forwarded by `sdl3-sys`, so they are unreachable from `Cargo.toml` and the
toolchain file is the only channel. `SDL_UNIX_CONSOLE_BUILD` *is* forwarded, and
is required: with both desktop drivers off, `cmake/macros.cmake:415` raises
`FATAL_ERROR` unless it is set, so it belongs in the manifest as
`build-from-source-unix-console` on `sdl3`.

**What the target needs instead of X11, and why it is deferred.**
`CheckKMSDRM` — the head unit's scanout path — requires
`pkg_check_modules(PC_LIBDRM libdrm)` *and* `pkg_check_modules(PC_GBM gbm)`
*and* `HAVE_OPENGL_EGL`. It goes through **pkg-config**, so aarch64 needs a
target sysroot with a working `aarch64-linux-gnu-pkg-config`; Debian's cross
toolchain alone does not provide one. `CheckEGL` is `check_c_source_compiles` —
compile only, against SDL's bundled khronos headers — so no EGL dev package is
needed. This is the point where `CROSSBUILD.md`'s "a sysroot stops being
optional" becomes real, and it is the decision being deferred.

**Runtime consequence to remember:** with `SDL_DEPS_SHARED=ON`, the target's SDL
`dlopen`s libdrm and libgbm, so the *target image* must ship them. Setting
`SDL_DEPS_SHARED=OFF` for static linking would change all of the above and make a
sysroot mandatory.

## Deviations from the spec, and why

- **SDL3 ships with all twelve subsystems enabled.** The architecture doc asks
  for audio, render, camera and filesystem off. Accepted temporarily because
  `sdl3` 0.20.0 re-exports no subsystem features, so honouring the doc needs
  either a second direct `sdl3-sys` dependency or target-asymmetric options
  forced from the toolchain file. Neither is worth the cost while the head
  unit's hardware requirements are still unknown. Consequences to remember: the
  binary carries audio and camera code it will not use, and no choice is being
  made about HID, haptics, or which video driver the target image wants.
  Revisit before `roados_ui`, not before task 02.
- **`ui_core`'s entry point is `src/lib.rs`, not `src/mod.rs`.** Task 02
  specifies `lib.rs`, which is also what Cargo expects, so
  `PRIMITIVES_ARCHITECTURE.md:332` is stale on this one point and was left
  alone. Nothing else in that document's *Module Layout* conflicts with the
  tree: the module list matches, and its `roados_ui/` line is a sibling
  directory rather than a module of `ui_core`, so it is not a workspace member
  at this stage.

## Protocol in force

**Canonical owner: `.ai/workflows/task-sequence.md`.** That file states the
per-task loop and the gates, and is routed to by `AGENTS.md`. It is not restated
here; this section is a pointer so a reader who lands in this file does not
re-derive it.

Short form, for orientation: one task at a time in numeric order — developer
dispatched as `general`, then a reviewer in a **different** session, then fixes
back to the developer, then **stop for the operator**, who commits and reports
the SHA. `developer` and `reviewer` are instruction files, not subagent types;
`.ai/protocols/subagents.md` allows only `explore` and `general`, and their
contents are inlined into each dispatch brief.

## Environment as found (2026-09-28)

Recorded because several acceptance criteria depend on it.

- `cargo`/`rustc` 1.80.1 via snap; `rustup` present with `stable-x86_64-unknown-linux-gnu` active.
- **aarch64 cross toolchain installed 2026-09-28** (operator):
  `aarch64-linux-gnu-gcc` 15.2.0, `aarch64-linux-gnu-g++` 15.2.0, GNU
  binutils 2.46, `-dumpmachine` → `aarch64-linux-gnu`. Cross build verified —
  see `CROSSBUILD.md` §6.4.2.
- `rustup` targets installed: `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`,
  `wasm32-unknown-unknown`.
- `libEGL` and `libGLESv2` present (Mesa); `DISPLAY=:0`; `/dev/dri/card{1,2}`.
  GLES 3.1 support itself is unconfirmed.
- No code in the repository yet, so there are no existing conventions to match
  until task 02 establishes them.

## Task table

`AC waived` records acceptance criteria that were met with a reason instead of
verified. A blank cell is unknown, not "none".

| # | Task | Status | Commit | Review | AC waived |
|---|---|---|---|---|---|
| 01 | Crossbuild Environment Setup | done | `2f27127` | 3 review passes, 4 fix rounds | 9 open, 2 closed |
| 02 | Project Scaffolding | in progress | | 3 review passes, 3 fix rounds | |
| 03 | SDL3 + OpenGL ES 3.1 Context | pending | | | |
| 04 | Arena Allocator | pending | | | |
| 05 | Property System | pending | | | |
| 06 | Rendering Pipeline | pending | | | |
| 07 | Layout System | pending | | | |
| 08 | Theme System | pending | | | |
| 09 | Animation System | pending | | | |
| 10 | Input Handling | pending | | | |
| 11 | Widget — Label | pending | | | |
| 12 | Widget — Button | pending | | | |
| 13 | Widget — Container | pending | | | |
| 14 | Widget — Slider | pending | | | |
| 15 | Widget — Toggle | pending | | | |
| 16 | Widget — Image | pending | | | |
| 17 | Widget — Progress | pending | | | |
| 18 | Widget — List/Scroll | pending | | | |
| 19 | Widget — TextInput | pending | | | |
| 20 | Widget — Gauge | pending | | | |
| 21 | Widget — Chart | pending | | | |
| 22 | Widget — Dialog | pending | | | |
| 23 | Widget — Toast | pending | | | |
| 24 | Demo Application | pending | | | |

Status values: `pending` · `in progress` · `implemented` (developer done,
awaiting review) · `in review` (reviewer running) · `changes requested` ·
`approved` (operator approved, awaiting commit) · `done` (committed) ·
`blocked` (see History for the reason).

Numeric order is a valid dependency order: the cross-references in the task
files place 03 before 06 before 07, 05 before 08 and 09, 10 before the input
widgets, and 24 last.

## Tasks 25–29, added after the demo

Created 2026-09-28, **pending the operator's ratification**. They exist so the
deferred work is not lost, not because anything in 02–24 needs them.

The trigger was a review question worth answering explicitly: *does the missing
KMSDRM driver block tasks 02–24, given a successful cross build is required?*
**No.** `CROSSBUILD.md` §6.4 configures and builds SDL for the target with
`SDL_X11=OFF`, `SDL_WAYLAND=OFF`, `SDL_UNIX_CONSOLE_BUILD=ON` and **no sysroot
at all**, exit 0. Every platform dependency degrades to `OFF` rather than
`FATAL_ERROR`; the only fatal gate is no-X11-no-Wayland, which the toolchain file
already handles. What the missing driver breaks is *runtime video on the device*,
not the build.

| # | Task | Needs |
|---|---|---|
| 25 | Target Image and Sysroot | operator strategy decision; unblocks 26 |
| 26 | Head-Unit Video Driver | 25 |
| 27 | Target Runtime Library Audit | 25, 26 |
| 28 | Reconcile the SDL Configuration | none — but item 5 must land **before task 04** |
| 29 | Head-Unit Smoke Test | 25, 26, 27 |

**Three things were recorded as gating the cross-build requirement**, and they
were not all in these tasks. All three are closed as of task 02, 2026-09-28.
What remains open is the sysroot — `CROSSBUILD.md` §8 item 1 — which is a runtime
question, not a build one.

1. **`build-from-source-unix-console` must be in the manifest.** Without it the
   cross configure dies at `cmake/macros.cmake:415`. It is the one option that
   makes the target build possible, it lives in the file task 02 creates, and
   task 01 only documents it. **Task 02's brief must carry this.** — carried, in
   both manifests, and no longer removable; see `AGENTS.md` § Rust.
2. **The aarch64 cross toolchain is now installed** (2026-09-28), and the real
   cross build passes — `CROSSBUILD.md` §6.4.2. A
   `cargo build --target aarch64-unknown-linux-gnu` **now passes as well**, with
   no sysroot: the manifest task 02 created exists and carries
   `build-from-source-unix-console`, and the artifact is
   `ELF 64-bit LSB pie executable, ARM aarch64` — `CROSSBUILD.md` §4.2 records
   the command and the `readelf -h` output. Nothing mechanical is left between
   this tree and a target build.
3. The earlier caveat that every cross run used an x86_64 `gcc` symlink is
   superseded by item 2 and by §6.4.2: a real aarch64 compile happened, twice —
   once as a direct CMake configure and once through Cargo. What a sysroot would
   still add is the *runtime* side, which is item 1.

## Resolved since task 01 was reviewed

- **The Rust toolchain blocker is gone.** `rustup update stable` was executed by
  the operator on 2026-09-28; the toolchain is now `cargo`/`rustc` 1.98.1, above
  the 1.85 floor `sdl3-sys` 0.7.1 needs. Closes waivers 2 and 3.
- **The pinned dependency set now builds natively.** With the operator's
  `libxcursor-dev libxrandr-dev libxss-dev` install, `cargo build` of
  `sdl3 = { version = "0.20", features = ["build-from-source"] }` plus `glow 0.18`
  completes, exit 0. Closes waiver 3 for the native path.
- **The X11 link-time dependency does not exist — measured, not argued.** `ldd`
  on the built binary reports four dynamic dependencies and nothing else:
  `linux-vdso.so.1`, `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`. No
  `libX11`, no `libGL`, no `libEGL`. X11 support is entirely `dlopen`, recorded
  as sonames in the generated `SDL_build_config.h`:
  `SDL_VIDEO_DRIVER_X11_DYNAMIC "libX11.so.6"`,
  `..._DYNAMIC_XCURSOR "libXcursor.so.1"`, and six more.
  `SDL_VIDEO_OPENGL_EGL 1` and `SDL_VIDEO_OPENGL_ES2 1` are both set, with
  `SDL_VIDEO_OPENGL` and `SDL_VIDEO_OPENGL_GLX` undefined — EGL and GLES only,
  which is the shape the head unit wants.

## New finding — KMSDRM is silently disabled without pkg-config

Found 2026-09-28 while verifying the native build. **Not a build failure: the
build succeeds and produces an SDL with no scanout driver.** On a head unit that
means `SDL_Init` finds no video driver at all, with no error and no warning.

`CheckKMSDRM` in `SDL/cmake/sdlchecks.cmake` requires three things, and the first
two are reached only through pkg-config:

```cmake
if(PKG_CONFIG_FOUND)
  pkg_check_modules(PC_LIBDRM IMPORTED_TARGET ${PKG_CONFIG_LIBDRM_SPEC})
  pkg_check_modules(PC_GBM   IMPORTED_TARGET ${PKG_CONFIG_GBM_SPEC})
endif()
if(PC_LIBDRM_FOUND AND PC_GBM_FOUND AND HAVE_OPENGL_EGL)
  set(HAVE_KMSDRM TRUE)
  set(SDL_VIDEO_DRIVER_KMSDRM 1)
```

On this host `pkg-config` is **absent**, so `PKG_CONFIG_FOUND` is false, so
`PC_LIBDRM_FOUND` and `PC_GBM_FOUND` are false, so `HAVE_KMSDRM` is never true.
The generated header confirms it:

```
/* #undef SDL_VIDEO_DRIVER_KMSDRM */
/* #undef SDL_VIDEO_DRIVER_KMSDRM_DYNAMIC */
/* #undef SDL_VIDEO_DRIVER_KMSDRM_DYNAMIC_GBM */
```

This does not affect the native build, where X11 supplies the window and the
demo is visible. It affects the **target**, whose scanout path *is* KMSDRM, and
it does so silently. `pkg-config` and `libdrm-dev`/`libgbm-dev` — or their
target-sysroot equivalents — are therefore hard requirements for any aarch64
build, not optional extras. Carried into the aarch64 decision below, and into
the task 01 amendment.

## Blocking task 02 — the host is missing the X11 development headers

**Status: resolved 2026-09-28.** The operator ran

```sh
sudo apt-get install -y libxcursor-dev libxrandr-dev libxss-dev
```

and the pinned dependency set then built natively, exit 0 — `CROSSBUILD.md`
§6.6. No longer a blocker. The finding is kept below because it is what the
package list came from, not because it is open.

Found by running the build, not by reading. The vendored SDL3 configure fails on
this host:

```
CMake Error at cmake/sdlchecks.cmake:405 (SDL_missing_dependency)
  Couldn't find dependency package for XCURSOR.  Please install the needed
  packages or configure with -DSDL_X11_XCURSOR=OFF
```

Measured: `/usr/include/X11/Xlib.h` and `X11/XKBlib.h` are present;
`X11/extensions/Xcursor/Xcursor.h`, `Xrandr.h` and `scrnsaver.h` are **absent**,
and there are no `libX*.so` development symlinks. ALSA headers are absent too,
which is a warning rather than an error.

**The awkward part, and the reason this needs a decision rather than a fix.**
`CROSSBUILD.md` §7.6 establishes that with `build-from-source` there is no `-D`
escape hatch: `cmake` 0.1.58 has no `CMAKE_ARGS`, CMake does not read
`CMAKE_PROJECT_INCLUDE` from the environment, and `sdl3-sys`'s `build.rs` exposes
no X11 options at all — verified, there is no `X11`/`XCURSOR` string in it. So
`-DSDL_X11_XCURSOR=OFF` is unreachable from `Cargo.toml`.

That leaves exactly two routes on a **native** build, and a native build passes no
toolchain file:

1. Install `libxcursor-dev`, `libxrandr-dev`, `libxss-dev` on the host.
2. Route the native build through a toolchain file that forces the options —
   which then makes the native and aarch64 builds asymmetric, and puts the
   configuration somewhere `Cargo.toml` does not describe.

Note this is **independent of the subsystem decision** above: the X11
sub-options are not subsystems, so accepting default subsystems does not bring
this closer to fixed, and disabling the documented four would not have fixed it
either. It was always going to be needed.


- Task 04 places "widget node structure" out of scope, deferring it to task 05;
  task 05 is titled *Property System* and does not list a widget node type
  among its requirements. Whoever reaches 04/05 should resolve this rather than
  both agents guessing differently. Not yet settled by the operator.
- **`PRIMITIVES_ARCHITECTURE.md` § Dependencies is wrong in two places**, found
  while doing task 01. Both need an operator decision; neither is mine to amend.
  1. It says SDL subsystems are disabled by build configuration, but `sdl3`
     0.20.0 **re-exports no subsystem features at all** — its 20 features
     include none, and none forwards one. The switches live on `sdl3-sys` 0.7.1
     (`sdl-<name>` / `no-sdl-<name>`). So the pinned dependency line cannot
     implement the documented configuration. Task 02 is blocked on the choice
     between a second direct `sdl3-sys` dependency, or options forced from the
     toolchain file — which is target-asymmetric and invisible in `Cargo.toml`.
  2. It lists **filesystem** among the subsystems to disable. There is no
     `SDL_FILESYSTEM` option in SDL 3.4.16 and no `sdl-filesystem` feature in
     `sdl3-sys`; SDL always compiles its Unix filesystem implementation.
     The doc and reality disagree.

## Waivers, task 01

Transcribed from the developer's handoff and confirmed by the reviewer. Per the
operator's rule, none of these is treated as satisfied.

1. ~~**No real aarch64 build or artifact**~~ — **resolved 2026-09-28.** The
   operator installed `gcc-aarch64-linux-gnu`, `g++-aarch64-linux-gnu` and
   `binutils-aarch64-linux-gnu`; the Rust target was already present. A **real**
   cross configure and build now succeed — no fake `PATH`, no stub `pkg-config`,
   no arch-flag override, the file's own `-march=armv8-a` — and
   `readelf -h libSDL3.so.0` reports `Machine: AArch64` across all 251 objects.
   Full transcript in `CROSSBUILD.md` §6.4.2. Closed.
2. ~~**`rustup update stable` not executed**~~ — **resolved 2026-09-28.** The
   operator ran it; the toolchain is 1.98.1. Closed.
3. ~~**`cargo build` of the pinned dependency set fails**~~ — **resolved
   2026-09-28** for the native path, after the X11 dev packages were installed.
   `cargo build` of the pinned set now completes, exit 0. The *aarch64* build
   remains unbuilt and is covered by waiver 1.
4. **The §6.4 cross configure is not an aarch64 build** — the compiler was a
   symlink to host x86_64 `gcc`. The document states this rather than claiming
   otherwise.
5. **The archive-step failure is not reproduced** — the cross build dies earlier,
   at dbus. The mechanism is cited from CMake's sources, not observed.
6. **The dbus and libdrm results depend on a stub `pkg-config`.** A 17-byte
   `exit 0` stub made `*_FOUND` come back true. Re-run with a real
   `aarch64-linux-gnu-pkg-config`.
7. **Host OpenGL ES 3.2 is not the target's GLES 3.1.** `glxinfo -B` is a GLX host
   result; no EGL 3.1 context was created. Verifying it needs the head unit.
8. **The subsystem configuration in §5.1 is untested in practice.** The
   feature-rejection error is cited from `sdl3`'s feature table, not reproduced,
   because cargo 1.80.1 cannot parse the manifest. Needs (2) first.
9. **`ROADOS_SYSROOT` never exercised against a real target.** Ran in script mode
   and in real `project()` configures, but only with a hand-made fake sysroot and
   a stand-in compiler. `CMAKE_SYSROOT`, `CMAKE_FIND_ROOT_PATH`,
   `PKG_CONFIG_SYSROOT_DIR` and `PKG_CONFIG_LIBDIR` are verified; whether
   *library* search paths resolve is not.

## History

- 2026-09-28 — file created before the first dispatch, so an interrupted task is
  recoverable. No task started.
- 2026-09-28 — **task 01 implemented.** Review round 1: `fix first`, 1 blocking
  (a stray code fence that rendered §6.4 through §8 — 198 lines, including the
  unblocking instructions — as literal code), 4 should-fix, 5 nits. All fixed.
- 2026-09-28 — review round 2: `fix first` again, delta small. All 11 original
  findings confirmed fixed; both disputes the developer raised were **conceded in
  the developer's favour** — the reviewer had invented a `gcc -print-prog-name`
  mechanism, then partly overcorrected into treating a missing
  `CMAKE_<LANG>_COMPILER_AR` as a general archiver problem when it only feeds the
  IPO archive rules, and its dbus "host contamination" finding turned out to be
  the reviewer's own 17-byte `pkg-config` stub. Round 2 raised 5 new findings, all
  introduced by the fix round; all fixed in round 3.
- 2026-09-28 — fix round 3 also produced a self-correction worth recording: the
  developer had earlier reported `CMAKE_LIBRARY_ARCHITECTURE` coming out wrong and
  proposed a guard. The guard was inert; `project()` derives the value from the
  compiler it probed and overwrites whatever a toolchain file set. No defect
  existed — the wrong value only ever appeared because a host compiler stood in
  for aarch64. The comment now documents that the line is not load-bearing.
- 2026-09-28 — two operator decisions now block task 02: the `rustup update
  stable` machine change, and how SDL subsystems get configured.
- 2026-09-28 — **both settled by the operator.** `rustup update stable` executed
  (toolchain now 1.98.1); SDL3 to build with default subsystems for now, the
  deviation from `PRIMITIVES_ARCHITECTURE.md` recorded above rather than
  silently adopted.
- 2026-09-28 — running the build after the toolchain update exposed a **new**
  blocker for task 02: the host lacks the X11 extension development headers, and
  SDL's X11 sub-options are unreachable from `Cargo.toml` for the reason
  `CROSSBUILD.md` §7.6 documents. Open for the operator.
- 2026-09-28 — **the operator installed the X11 dev packages.** The pinned
  dependency set then built natively, exit 0. The X11 link-time dependency was
  measured rather than argued: `ldd` reports four dependencies and no X11.
- 2026-09-28 — **the operator's three decisions** (native keeps X11, target
  drops it, aarch64 libraries deferred) were implemented as a task 01 amendment.
  Review round 3: `fix first`, but the substance confirmed and **all six of the
  developer's self-declared flags adjudicated in the developer's favour** —
  including the reviewer reversing its own earlier position on
  `SDL_UNIX_CONSOLE_BUILD`. Six findings remained, all mechanical: stale
  evidence, two wrong counts, two stale statements, two nits.
- 2026-09-28 — fix round 4 cleared them, and produced a correction the reviewer
  had missed: `message_tested_option` (`cmake/macros.cmake:55`) prints
  `(Wanted: ${_REQVALUE}): ${HAVE_<name>}` — **two different variables**. So
  `(Wanted: ON): OFF` means the option was on and the *backend test* failed, not
  that the option was off. Verified: of 31 such lines, 29 are `BOOL=ON`, two
  have no entry, and **none** is `BOOL=OFF`. This reframes the whole
  missing-`pkg-config` finding and is now documented as such.
- 2026-09-28 — **task 01 ready for operator commit.** Three review passes, four
  fix rounds. The toolchain file and the document are untracked; nothing is
  staged or committed. Task 01 still awaits the operator's approval.
- 2026-09-28 — **the operator installed the aarch64 cross toolchain**, and it
  changes task 01's standing. The last open waiver — "no real aarch64 build or
  artifact" — is now **closed with evidence** rather than carried: a real cross
  configure and build, real `aarch64-linux-gnu-gcc`, the file's own
  `-march=armv8-a`, `Machine: AArch64` across all 251 objects. Full transcript
  in `CROSSBUILD.md` §6.4.2, and §6.4 is retitled so its stand-in runs no longer
  read as the verdict.
- 2026-09-28 — **the real build also corrected two standing claims.** (i)
  `SDL_KMSDRM` is `BOOL=ON` in the cache without any forcing, because
  `dep_option` defaults it ON for Unix; what fails is the *backend test*, so task
  26's fix is the sysroot and its requirement 2 is now answered. (ii) The target
  records **zero** `dlopen` sonames — `#define …DYNAMIC` count is 0 and none of
  `libX11.so.6`, `libwayland-client.so.0`, `libdrm.so.2`, `libgbm.so.1` appears in
  the binary. A naive `grep -oE '…DYNAMIC…'` reports 26 and is wrong, because it
  matches the macro names in `#undef` lines. Tasks 26 and 27 were updated.
- 2026-09-28 — **the operator asked for the workflow to be written down**, and it
  is now `.ai/workflows/task-sequence.md`, canonical, with pointers from
  `AGENTS.md` and this file. It does not reopen the 2026-09-27 decision to keep
  `reviewer.md` the only review document; it owns the sequence and the gates
  between steps, which no agent file owned.
- 2026-09-28 — **tasks 25–29 drafted** to carry the deferred work, pending
  ratification. Triggered by the reviewer's question about whether the missing
  KMSDRM driver blocks the sequence: it does not, because a cross build needs
  neither the driver nor a sysroot — `CROSSBUILD.md` §6.4.2 now proves the
  cross build works with no sysroot at all.
- 2026-09-28 — **task 01 committed by the operator**, `2f27127`, and task 02
  started. The `Pending` statuses of 25–29 are unchanged: they are still outside
  the confirmed 24-task spec, and 02–24 do not depend on them — except that item
  5 of task 28 must land before task 04, which is the next place that bites.
- 2026-09-28 — **task 02, review round 1: six findings, five fixed, one
  escalated to the operator.** Fixed: `AGENTS.md` duplicated rules
  `developer.md` owns; `CROSSBUILD.md` §4.2 documented a command with no
  evidence and never said where `.cargo/config.toml` comes from;
  `IMPLEMENTATION_STATE.md` never recorded that `PRIMITIVES_ARCHITECTURE.md:332`
  is stale on the `lib.rs` entry point; `ui_core/src/lib.rs` claimed the module
  tree follows that document when the task file lists the modules; and the
  `ui_demo` manifest comment did not say the feature list is a mirror. All five
  held on re-review.
- 2026-09-28 — **task 02, review round 1, escalated blocker: SDL is linked
  dynamically and nothing says where the target's copy comes from.** Measured,
  not inferred: `readelf -d` on both artifacts gives `NEEDED libSDL3.so.0` and
  no `RPATH`/`RUNPATH`; run directly, the host's `/usr/local/lib` SDL 3.5.0
  wins over the vendored 3.4.16, and `cargo run` only gets the vendored one
  because Cargo puts the crate's link-search directory on `LD_LIBRARY_PATH`. The
  static route works but is not free: `SDL_DEPS_SHARED` stays ON under
  `SDL_STATIC`, so the statically linked binary still carries the X11 chain's
  `dlopen` sonames — measured on this host, `libX11.so.6`, `libXcursor.so.1`,
  `libXrandr.so.2`, `libX11-xcb.so.1` — and what the target `dlopen`s is whatever
  its `pkg-config` supplies, which today is nothing (§6.4.2 records zero
  sonames). And rpath or install-prefix packaging is unreachable while SDL is a
  cross build, because `sdl3-sys` is written in Rust and SDL's CMake never sees
  the manifest.
  **Not the developer's to decide and not fixed:** the operator owns the
  linkage strategy. Unchanged by either review round, and no acceptance
  criterion is waived over it — all four were verified.
- 2026-09-28 — **task 02, review round 2: six findings, all documentation or
  one-sentence corrections.** The escalation above was accepted as correctly
  handled, and all five round-1 fixes were confirmed to hold. Fixed here: the
  three lossy restatements of `developer.md` rules that round 1 had left
  standing; two documents that still claimed no aarch64 cargo build had ever
  been run, when the artifact and a from-scratch build prove otherwise; the
  `CROSSBUILD.md` preamble that still said the project did not exist, and §3's
  missing working directory; two manifest comments that stated the feature
  mechanics wrongly; one stale `cmake/macros.cmake` line number; and the
  review-rounds bookkeeping — two History lines and the `Review` cell — that
  this round's own finding 4 asked for. The
  reviewer's own three settled facts — the `sdl3` licence, `SDL_RPATH`
  reachability, and `wait_event_timeout`'s `None` on error — were offered for
  elsewhere and deliberately not taken here.
- 2026-09-28 — **task 02, review round 3: four findings, all fixed.** All four
  were documentation corrections, and the reviewer confirmed the change is
  otherwise ready for commit. Fixed here: §8 item 3's stale reason — "no aarch64
  build exists yet" replaced with the measured one, that the aarch64 builds so
  far run with no sysroot and no target `pkg-config`; the round-2 History
  undercount — "five findings" corrected to six and the review-rounds
  bookkeeping added to the list; §4.2's pronoun — the binary's DWARF references
  one tree source, the other twelve are in `libui_core.rlib`, so the sentence now
  says the build carries thirteen with the split named — and its timing figure,
  46.8 s adjusted to the reproducible 46.7 s; and one off-by-one citation,
  `cmake/macros.cmake:56` → `:55`, the `message(STATUS …)` line rather than the
  `endmacro()` below it.
