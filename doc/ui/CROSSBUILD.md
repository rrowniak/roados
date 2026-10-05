# Crossbuild environment

Build environment for the `ui_core` primitives library: native x86_64 Linux and
cross-compiled aarch64 Linux, both targeting OpenGL ES 3.1.

**Written:** 2026-09-28.
**Applies to:** the dependency set pinned in
`doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Dependencies*.

> **The project exists as of 2026-09-28.** `Cargo.toml`, `ui/src/ui_core/` and
> `ui/src/ui_demo/` were created by `doc/ui/TASK_UI_PRIM_02.md`; the task table
> in `doc/ui/IMPLEMENTATION_STATE.md` says how far each task got. Commands here
> were written before the project existed, so read them as the procedure and
> check them against the current tree. What *was* executed, and what it proved,
> is in [§6 Verification](#6-verification) — read it before trusting a command
> here.

---

## 1. What the vendored SDL3 build actually is

The pinned dependency line is:

```toml
sdl3 = { version = "0.20", features = ["build-from-source"] }
glow = "0.18"
```

`build-from-source` does **not** mean "SDL source lives inside the `sdl3-sys`
crate". It means `sdl3-sys` pulls a second crate, `sdl3-src`, which contains the
SDL C sources and builds them with CMake. The exact chain, read from the
published `.crate` files on 2026-09-28:

| Crate | Version | Note |
|---|---|---|
| `sdl3` | 0.20.0 | `build-from-source` → `sdl3-sys/build-from-source` |
| `sdl3-sys` | `0.7.1+SDL-3.4.16` | build metadata carries the vendored SDL version |
| `sdl3-src` | `=3.4.16` | the SDL C sources; `sdl3-sys` pins it exactly |
| SDL | **3.4.16** | `cmake_minimum_required(VERSION 3.16)`, `project(SDL3 LANGUAGES C VERSION "3.4.16")` |
| `cmake` | `0.1` | resolved to **0.1.58** in `ui/Cargo.lock`; a fresh resolve may pick a later 0.1.x, and §4.1's variable names were read off 0.1.58's source, so re-read them if it moves |
| `glow` | 0.18.0 | pure Rust, no build script |

Two consequences worth internalising before reading further:

1. **Every SDL CMake option name in this document is a 3.4.16 name.** They have
   moved between SDL major versions. Do not carry them to SDL 3.2 or 3.6
   without re-reading `SDL/CMakeLists.txt`.
2. **The subsystem switches are Cargo features on `sdl3-sys`, and `sdl3` does
   not re-export a single one of them.** The pinned dependency line above is
   therefore not sufficient to implement the subsystem configuration in
   `PRIMITIVES_ARCHITECTURE.md`. §5.1.

### Build-script mechanics, because the rest depends on them

`sdl3-sys` 0.7.1's `build.rs` calls `cmake::Config::new(SOURCE_DIR)`, defines
`SDL_EXAMPLES=OFF`, `SDL_TESTS=OFF`, `SDL_REVISION=<rev>`, and then maps a fixed
list of Cargo features to `-D` variables through a `cmake_vars!` macro. It
passes **no toolchain file, no extra `-D`, and no `CMAKE_ARGS`**. `cmake` 0.1.58
has no `CMAKE_ARGS` support at all (no such string in its source), and CMake's
own environment-variable list (`cmake --help-manual cmake-env-variables`, CMake
4.2.3) does not include `CMAKE_PROJECT_INCLUDE` or `CMAKE_PROJECT_INCLUDE_BEFORE`.

So there is exactly one supported way to influence the SDL configure from outside
the crate: **`CMAKE_TOOLCHAIN_FILE` in the environment.** `cmake` 0.1.58 reads it
before it decides anything else and passes it on as a `-D`. The same lookup and
the same behaviour were observed in `cmake` 0.1.58, so this is not a quirk of one
patch release.

---

## 2. Prerequisites

### 2.1 Rust

| Requirement | Value | Why |
|---|---|---|
| cargo / rustc | **>= 1.85** | `sdl3-sys` 0.7.1 is `edition = "2024"`, `rust-version = "1.85"`. `sdl3` 0.20.0 itself is edition 2021 with no `rust-version`; the requirement comes entirely from the `-sys` crate. |
| rustup | any recent | only used for `rustup target add` |
| aarch64 std | `rustup target add aarch64-unknown-linux-gnu` | cross builds only |

> **This machine now meets the Rust requirement.** The operator ran
> `rustup update stable` on 2026-09-28; `cargo` and `rustc` are both **1.98.1**,
> which clears the 1.85 floor. The earlier state, for the record, was 1.80.1 on
> an otherwise-standard unpinned toolchain — `~/.rustup/settings.toml` says
> `default_toolchain = "stable-x86_64-unknown-linux-gnu"`, and the `/snap/bin`
> entries are rustup shims (`rustc -> rustup.rustc`), not a separate compiler.
> The floor is now cleared and the dependency set builds; see
> [§6.6](#66-cargo-build-of-the-pinned-dependency-set--pass).
>
> The installed targets are `x86_64-unknown-linux-gnu` and
> `wasm32-unknown-unknown`; `aarch64-unknown-linux-gnu` is **still not
> installed**, so only the native path has actually been built.

### 2.2 Native C toolchain and build tools

| Tool | Required? | Notes |
|---|---|---|
| `cmake` | **yes** | >= 3.16 for SDL 3.4.16. Measured here: 4.2.3, works. |
| C compiler + `make` | **yes** | SDL is C. Measured here: gcc 15.2.0, GNU make. |
| `pkg-config` | **yes, for the target** | gates KMSDRM, four of the five Linux audio backends (PipeWire, PulseAudio, JACK, sndiod — *not* ALSA, see the correction below), text shaping, Wayland, and the embedded-GPU paths. **Absent on this host**, which silently cost the native build its KMSDRM driver — see the correction below and §6.7. |
| `ninja` | optional | only if you set `CMAKE_GENERATOR=Ninja` |
| `python3` | no | not referenced by SDL 3.4.16's CMake at all, despite `cmake/xxd.py` shipping in the tree |

Measured on this machine (2026-09-28): `cmake` 4.2.3, `gcc` 15.2.0, `make`
present, `ninja` **absent**, `pkg-config` **absent**.

> ### ⚠ Correction: `pkg-config` **is** required, and its absence is silent
>
> An earlier revision of this section said `pkg-config` was "no, not on the
> vendored path" and that installing it would "send the operator to install
> something the build never calls". **That was wrong**, and the error is worth
> recording because it is a scoping mistake rather than a typo: the `pkg_check_modules`
> audit was done against SDL's `CMakeLists.txt` only, where the sole call really
> is line 1980, for `libinotify`. The decisive calls are in
> `cmake/sdlchecks.cmake`, which was not searched.
>
> The counts, stated as counts. In `cmake/sdlchecks.cmake` there are **11**
> `if(PKG_CONFIG_FOUND)` guards, in **11** distinct `Check<X>` macros, wrapping
> **14** `pkg_check_modules` calls — the extra three are `CheckKMSDRM`,
> `CheckWayland` and `CheckRPI` making two calls each. Adding the BSD-only
> `libinotify` call in `CMakeLists.txt:1980` (under the guard at 1943) makes
> **15** calls under **12** guards across SDL's own configure. *This paragraph
> previously said 16, which is no count of anything.*
>
> Two qualifications, so the number is not over-read. A whole-tree grep finds
> two more `pkg_check_modules` call sites that the 15 does not include:
> `cmake/FindLibUSB.cmake:9` and `cmake/FindFFmpeg.cmake:56` (the latter inside a
> `find_component` macro, so one site expanded per component). Both guard on
> `PKG_CONFIG_FOUND` themselves and both keep `find_library`/`find_path`
> fallbacks, so they degrade to `Could NOT find …` rather than failing — which is
> why `SDL_HIDAPI_LIBUSB` is off on this host for a missing `libusb-1.0-0-dev`
> and not for a missing `pkg-config`. A tree grep also turns up two
> `pkg_check_modules(… REQUIRED …)` calls in `src/hidapi/{linux,libusb}/`, which
> look alarming; they are the vendored hidapi's *standalone* build files and
> SDL never `add_subdirectory`s them, so they are unreachable from this build.
>
> | Macro | `.pc` modules requested | Gates |
> |---|---|---|
> | `CheckKMSDRM` | `libdrm`, `gbm` | **the head unit's scanout path** |
> | `CheckPipewire` | `libpipewire-0.3>=0.3.44` | *one of* four **of the five** Linux audio backends |
> | `CheckPulseAudio` | `libpulse>=0.9.15` | |
> | `CheckJACK` | `jack` | |
> | `CheckSNDIO` | `sndio` | |
> | `CheckFribidi` | `fribidi` | complex text shaping |
> | `CheckLibThai` | `libthai` | |
> | `CheckWayland` | `wayland-client>=1.18`, `wayland-egl`, `wayland-cursor`, `egl`, `xkbcommon>=0.5.0`; then `libdecor-0>=0.2.0` | Wayland, *and* the compositor-decoration path |
> | `CheckRPI` | `bcm_host`, `brcmegl` | embedded GPUs |
> | `CheckROCKCHIP` | `mali` | |
> | `CheckLibUnwind` | `libunwind`, `libunwind-generic` | backtrace |
>
> Only `libdrm` and `gbm` matter for the head unit; the rest are listed because
> the error that produced the bad earlier claim was undercounting, and a count
> that is going to be used to justify a hard target requirement should be
> checkable module by module.
>
> **ALSA is the exception, and it is worth being precise about why.** It is the
> fifth Linux audio backend and it is *not* in the table. `CheckALSA` does not
> use `pkg_check_modules`; it calls `find_package(ALSA MODULE)`
> (`cmake/sdlchecks.cmake:101-131`), i.e. SDL's own `FindALSA.cmake`, which
> looks for `asound`. Its `ALSA_PKG_CONFIG_SPEC` is used only to write
> `Requires.private: alsa` into the generated `sdl3.pc`, never to detect
> anything. So a host with `pkg-config` but no `libasound2-dev` loses ALSA, and
> a host with `libasound2-dev` but no `pkg-config` keeps it. That is the
> difference between "the `.pc` file could not be read" and "the library is not
> installed", and §6.7 measures both.
>
> `CheckKMSDRM` (`cmake/sdlchecks.cmake:1363-1374`) has **no `else()` branch**,
> so a missing `pkg-config` does not fail the configure — it just drops the
> driver. Measured on this host, where the build *succeeded*:
>
> ```
> -- Could NOT find PkgConfig (missing: PKG_CONFIG_EXECUTABLE)
> --   SDL_KMSDRM                  (Wanted: ON): OFF
> ```
>
> and the resulting `SDL_build_config.h` contained
> `/* #undef SDL_VIDEO_DRIVER_KMSDRM */`. Full measurement in §6.7 — including
> what that summary line does and does not mean, which is not obvious.

The one true statement in the old text, kept because it is the reason the native
build gets so far without `pkg-config`: `sdl3-sys` itself never needs the binary.
Its only `pkg-config` use is in the `#[cfg(not(feature = "build-from-source"))]`
branch of `build-common.rs`, to find a *prebuilt* system SDL3; with
`build-from-source` that branch is compiled out, and afterwards the crate parses
the `sdl3.pc` that SDL just wrote, using the `rpkg-config` Rust crate. The
absence of the *binary* is a problem for **SDL's** configure, not for the Rust
side — which is exactly why the failure is so easy to misattribute.

For the **target**, `aarch64-linux-gnu-pkg-config` plus `libdrm` and `libgbm` for
aarch64 are hard requirements, not conveniences. Without them the head unit
builds an SDL with no scanout driver and no diagnostic. See §6.7 and §7.5.

### 2.3 Platform development packages

**The short version:** SDL3 loads its platform libraries with `dlopen()` by
default (`SDL_DEPS_SHARED=ON`, the SDL default), so it needs far fewer `-dev`
packages than the upstream README suggests, and it ships its own Khronos
headers, so EGL and GLES headers are not required either. What it does need is
whatever you want compiled in, and the X11 extension libraries are the part
people forget.

Measured on this machine, a vanilla SDL 3.4.16 configure used to fail on the
first missing X11 extension and then on the next two:

```
CMake Error at cmake/macros.cmake:433 (message):
  Couldn't find dependency package for XCURSOR.  Please install the needed
  packages or configure with -DSDL_X11_XCURSOR=OFF
```

That was a **loud** failure, which is the good case. The operator installed
`libxcursor-dev libxrandr-dev libxss-dev` and the native build now configures and
compiles; see §6.6. The three-package list is the complete native delta:
`libx11-dev`, `libxext-dev` and `libxi-dev` were already present.

Present now: `libX11.so`, `/usr/include/X11`, and the three extension packages
above. Absent: `pkg-config` entirely (§2.2), `libgbm-dev`, and the dev packages
for EGL/GLES — which do not matter, see §2.4.

For a development host (X11/Wayland desktop):

```sh
sudo apt-get install build-essential cmake git make python3 \
    libx11-dev libxext-dev libxi-dev libxfixes-dev libxss-dev libxtst-dev \
    libxcursor-dev libxrandr-dev libxkbcommon-dev \
    libdrm-dev libgbm-dev libudev-dev
```

Note `pkg-config` is absent from that list only because it is implicit in
`build-essential` on Debian/Ubuntu; it is called out explicitly in §2.2 because
it is the one whose absence is invisible.

For the head unit the package list is a different question, and it is not
answerable yet: it depends on the target image, which is deferred. What *is*
settled is that the target needs no X11 packages at all (§5.4) and does need
`pkg-config`, `libdrm` and `libgbm` for aarch64 (§6.7).

### 2.4 EGL and GLES: SDL ships the headers

SDL 3.4.16 bundles `EGL/egl.h`, `EGL/eglext.h`, `GLES2/gl2.h` and
`GLES3/gl3.h` under `src/video/khronos/` and puts that directory on the include
path (`CMakeLists.txt:555-559`). `CheckEGL()`
(`cmake/sdlchecks.cmake:861-880`) adds that directory to
`CMAKE_REQUIRED_INCLUDES` and then compiles a translation unit that includes
`<EGL/egl.h>` — so the detection test passes with no system EGL dev package
installed. Confirmed here: this machine has no `/usr/include/EGL`, and the
configure still reported `Performing Test HAVE_OPENGL_EGL - Success`.

What the same configure produced is worth knowing, because it is the shape a
GLES-only head unit wants:

```
-- SDL_OPENGL                  (Wanted: ON): OFF
-- SDL_OPENGLES                (Wanted: ON): ON
```

and in the generated `SDL_build_config.h`:

```
#define SDL_VIDEO_OPENGL_EGL 1
#define SDL_VIDEO_OPENGL_ES2 1
```

`HAVE_OPENGL_GLX` and `HAVE_OPENGL` both failed here, because no desktop GL
development files are installed. SDL therefore came out **EGL + GLES only, no
GLX** — which is exactly the configuration an automotive head unit wants, and it
is reached without touching a single option.

### 2.5 aarch64 cross toolchain

**Installed 2026-09-28** (the operator ran the command below). Verified:
`aarch64-linux-gnu-gcc 15.2.0`, GNU ar/binutils 2.46, `aarch64-linux-gnu-g++
15.2.0`, `-dumpmachine` → `aarch64-linux-gnu`, and
`rustup target list --installed` includes `aarch64-unknown-linux-gnu`. The
cross build in §6.4.2 is real evidence produced with this toolchain, not a
stand-in.

On Debian/Ubuntu, all three packages are required:

```sh
sudo apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu \
                        binutils-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu
```

`g++` is not strictly needed for SDL, which is C, nor for `glow`, which is Rust.
It is listed because C++ build scripts are common in transitive dependencies and
installing it later is more disruptive than installing it now.

**The Rust target alone is not sufficient, and the failure is late.** With
`aarch64-unknown-linux-gnu` installed but no cross compiler, compilation
proceeds and the *link* fails. Install all of it together, and confirm with
`aarch64-linux-gnu-gcc -dumpmachine` before trusting a cross build.

`binutils-aarch64-linux-gnu` is not optional in practice, and it is worth being
precise about *which file* and *which rule* — there are two archiver variables
and they are used in different places.

The plain archive rule uses `CMAKE_AR`
(`/usr/share/cmake-4.2/Modules/CMakeCInformation.cmake:155`):

```cmake
set(CMAKE_C_ARCHIVE_CREATE "<CMAKE_AR> qc <TARGET> <LINK_FLAGS> <OBJECTS>")
```

The **IPO** archive rules use `CMAKE_<LANG>_COMPILER_AR` instead — IPO
(interprocedural optimization, of which LTO is the usual case) —
(`/usr/share/cmake-4.2/Modules/Compiler/GNU.cmake:110-127`), which is why:

```cmake
  # Need to use version of 'ar'/'ranlib' with plugin support.
  # Quote from [documentation][1]:
  #   To create static libraries suitable for LTO,
  #   use gcc-ar and gcc-ranlib instead of ar and ranlib
  set(CMAKE_${lang}_ARCHIVE_CREATE_IPO
    "\"${CMAKE_${lang}_COMPILER_AR}\" qc <TARGET> <LINK_FLAGS> <OBJECTS>")
```

CMake populates that variable with its own `find_program`, and the search is
narrow on purpose
(`/usr/share/cmake-4.2/Modules/Compiler/GNU-FindBinUtils.cmake:14-27`, CMake
4.2.3):

```cmake
# Try to find tools in the same directory as GCC itself
get_filename_component(__gcc_hints "${CMAKE_${_CMAKE_PROCESSING_LANGUAGE}_COMPILER}" DIRECTORY)

# http://manpages.ubuntu.com/manpages/wily/en/man1/gcc-ar.1.html
find_program(CMAKE_${_CMAKE_PROCESSING_LANGUAGE}_COMPILER_AR NAMES
    "${_CMAKE_TOOLCHAIN_PREFIX}gcc-ar-${__version_x_y}"
    "${_CMAKE_TOOLCHAIN_PREFIX}gcc-ar-${__version_x}"
    "${_CMAKE_TOOLCHAIN_PREFIX}gcc-ar${__version_x}"
    "${_CMAKE_TOOLCHAIN_PREFIX}gcc-ar${_CMAKE_COMPILER_SUFFIX}"
    HINTS ${__gcc_hints}
    NO_CMAKE_PATH NO_CMAKE_ENVIRONMENT_PATH
    DOC "A wrapper around 'ar' adding the appropriate '--plugin' option for the GCC compiler"
)
```

Three things follow, and each is a trap:

- The names are **`aarch64-linux-gnu-gcc-ar`**, not `ar` and not
  `aarch64-linux-gnu-ar`. `gcc-ar` is a wrapper that adds the LTO `--plugin`
  option plain `ar` cannot accept.
- `HINTS` is the compiler's own directory, and `NO_CMAKE_PATH
  NO_CMAKE_ENVIRONMENT_PATH` removes the default search paths, so the file is
  looked for *next to the compiler* and not on `PATH` generally.
- When the search fails, the cache records
  `CMAKE_C_COMPILER_AR:FILEPATH=CMAKE_C_COMPILER_AR-NOTFOUND` and **configure
  still succeeds**; the failure is deferred to the first archive step that uses
  the IPO rule. Measured in the cross cache on 2026-09-28:

```
CMAKE_C_COMPILER_AR:FILEPATH=CMAKE_C_COMPILER_AR-NOTFOUND
```

Since `CMAKE_INTERPROCEDURAL_OPTIMIZATION` is off by default, a build that does
not enable IPO never consults it — so this bites when someone turns IPO on (LTO
being the usual reason), not at bring-up. That timing is what makes it expensive
to diagnose. Measured, with IPO on and no LTO property set, the rule is still
the one that needs `gcc-ar`:

```
-- IPO req: [TRUE]
-- IPO rule: ["/tmp/opencode/r1/HAND-SET-AR" qc <TARGET> <LINK_FLAGS> <OBJECTS>]
-- LTO prop: []
```

Two consequences for this file: it sets `CMAKE_AR` explicitly (for the plain
rule), and it does **not** try to set `CMAKE_<LANG>_COMPILER_AR`.

That second decision needs the reasoning, because the obvious reason is wrong.
CMake's `find_program` **does not re-search when the variable already holds a
value**, so a hand-set value is *not* discarded — it sticks, it propagates into
the generated IPO rule, and it suppresses the search entirely. Verified on
2026-09-28 by injecting one line into a copy of this file and configuring:

```
# with set(CMAKE_C_COMPILER_AR "...") injected after the CMAKE_AR block
-- PROBE: CMAKE_C_COMPILER_AR=[/tmp/opencode/r1/HAND-SET-AR]
-- PROBE: CMAKE_C_ARCHIVE_CREATE_IPO=["/tmp/opencode/r1/HAND-SET-AR" qc <TARGET> <LINK_FLAGS> <OBJECTS>]
-- PROBE: CMAKE_C_ARCHIVE_CREATE=[<CMAKE_AR> qc <TARGET> <LINK_FLAGS> <OBJECTS>]

# control, unmodified
-- PROBE: CMAKE_C_COMPILER_AR=[CMAKE_C_COMPILER_AR-NOTFOUND]
-- PROBE: CMAKE_C_ARCHIVE_CREATE_IPO=["CMAKE_C_COMPILER_AR-NOTFOUND" qc <TARGET> <LINK_FLAGS> <OBJECTS>]
```

The search really is skipped, not merely overridden — in the hand-set build
`CMakeCache.txt` has **no** `CMAKE_C_COMPILER_AR` entry at all, where the control
has the `-NOTFOUND` one.

So it is left to CMake's search for two reasons, neither of which is "it would
not stick":

- A hand-set value is a **normal variable, not a cache entry**, so it never
  appears in `CMakeCache.txt` — which is the first place §7.3 tells you to look.
  A workaround that hides the variable it is working around makes the
  diagnostic useless.
- A hardcoded path gives up CMake's prefix- and version-aware search
  (`gcc-ar-13`, `gcc-ar-12`, …).

Installing `binutils-aarch64-linux-gnu` — which provides
`/usr/bin/aarch64-linux-gnu-gcc-ar` next to the compiler — is the supported fix.
Setting `CMAKE_C_COMPILER_AR` in this file **is** a working last resort for
someone stuck with a toolchain that ships no `gcc-ar` at all; it just should not
be the first thing you reach for.

> **Not reproduced here.** The archive failure itself was never reached on this
> machine: the cross build stopped earlier, at the `dbus/dbus.h` error in
> §6.4.1. The mechanism above is read from the cited CMake sources, not observed.

---

## 3. Native x86_64 build

No toolchain file. CMake detects the host, and `sdl3-sys` builds SDL with
`/usr/bin/cc`.

```sh
# from ui/ — the workspace root, and the only place Cargo.toml lives
cargo build
cargo build --release
cargo test
```

For x86_64 that is the whole story. The directory matters only because the
cross build in §4.2 is the one command that must be issued from the repository
root: it addresses `cmake/aarch64-toolchain.cmake` as `$PWD/…`, so §4.2 carries
a `--manifest-path` and this section does not. To see what SDL itself
does underneath — useful when a subsystem is missing at runtime and you need to
know whether it was even compiled in — configure the vendored source directly.
This is the exact command whose output is quoted below and summarised in §6.5,
run on 2026-09-28 against SDL 3.4.16 with CMake 4.2.3:

```sh
cmake -S <path-to>/sdl3-src-3.4.16/SDL -B /tmp/opencode/sdlverify \
  -DCMAKE_BUILD_TYPE=Release \
  -DSDL_EXAMPLES=OFF -DSDL_TESTS=OFF -DSDL_REVISION=SDL-3.4.16 \
  -DSDL_AUDIO=OFF -DSDL_RENDER=OFF -DSDL_CAMERA=OFF \
  -DSDL_X11_XCURSOR=OFF -DSDL_X11_XRANDR=OFF -DSDL_X11_XSCRNSAVER=OFF
```

Three of those flags are not optional for the *vendored* tree:

- `SDL_TESTS=OFF` — the `sdl3-src` crate ships SDL without its `test/`
  directory, and `CMakeLists.txt:4363` does `add_subdirectory(test)` whenever
  `SDL_TESTS` is on (the guard is `CMakeLists.txt:4360`). Without the flag:
  `add_subdirectory given source "test" which is not an existing directory.`
  `sdl3-sys` passes it unconditionally, so the cargo build is fine; a
  hand-rolled configure is not.
- `SDL_EXAMPLES=OFF` — likewise passed unconditionally by `sdl3-sys`.
- `SDL_REVISION=<rev>` — cosmetic; `sdl3-sys` sets it to `sdl3_src::REVISION`.

The three `SDL_X11_X*=OFF` flags are **machine-specific**, not project
requirements. They are what this host needs; a host with the X11 extension
`-dev` packages installed does not need them. Below is the tail of that
configure's output, with `...` marking elided regions and nothing else changed:

```
...
-- Subsystems:
--   Audio:    OFF
--   Video:    ON
--   GPU:      ON
--   Render:   OFF
--   Camera:   OFF
--   Joystick: ON
--   Haptic:   ON
--   Hidapi:   ON
--   Power:    ON
--   Sensor:   ON
--   Dialog:   ON
--   Tray:     ON
...
--   SDL_X11_XCURSOR             (Wanted: OFF): OFF
--   SDL_X11_XRANDR              (Wanted: OFF): OFF
--   SDL_X11_XSCRNSAVER          (Wanted: OFF): OFF
...
-- Enabled backends:
--   Video drivers: dummy offscreen x11(dynamic)
--   X11 libraries: xdbe xfixes xinput2 xshape xsync xtest
--   Render drivers: ogl_es2
--   GPU drivers: vulkan
--   Audio drivers: dummy
--   Joystick drivers: hidapi linux virtual
--   Camera drivers: dummy
...
-- Configuring done (27.8s)
-- Generating done (0.1s)
-- Build files have been written to: /tmp/opencode/sdlverify
```

That `Subsystems:` block is the single most useful diagnostic this build
produces — it is the whole SDL feature surface on twelve lines, and §5.1 is
about which of those twelve you want.

The `Enabled backends:` lines need reading carefully, because the `Audio
drivers: dummy` and `Camera drivers: dummy` entries survive subsystems being
`OFF`. A backend name here means "this code is in the binary", not "this
subsystem works"; the `Subsystems:` block is the authority on the latter.

The `(dynamic)` suffix on `x11` is the `SDL_DEPS_SHARED=ON` mechanism visible in
the build output: SDL recorded the *soname* `libX11.so.6` and will `dlopen` it on
the target.

### Confirming a built artifact

```sh
file target/debug/<binary>
readelf -h target/debug/<binary> | grep -E 'Class|Machine'
```

`ELF64` / `Advanced Micro Devices X86-64` for native, `AArch64` for a target
build. Checking this is cheap and catches the failure mode that nothing else
does: a "cross" build that quietly produced host objects.

---

## 4. aarch64 cross-compilation

### 4.1 How the toolchain file reaches the SDL build

There is no way to add a `-DCMAKE_TOOLCHAIN_FILE=` to the `sdl3-sys` configure
step from a `Cargo.toml`. The environment is the only channel, and `cmake`
0.1.58 resolves it target-aware, in this order:

1. `CMAKE_TOOLCHAIN_FILE_aarch64-unknown-linux-gnu`
2. `CMAKE_TOOLCHAIN_FILE_aarch64_unknown_linux_gnu`
3. `TARGET_CMAKE_TOOLCHAIN_FILE` (when cross-compiling) or
   `HOST_CMAKE_TOOLCHAIN_FILE` (when the host is the target)
4. `CMAKE_TOOLCHAIN_FILE`

**Note the direction of form 3: the kind comes first.** It is
`format!("{}_{}", kind, var_base)` at `cmake-0.1.58/src/lib.rs:958`, not the
other way round. An earlier draft of this list had
`CMAKE_TARGET_CMAKE_TOOLCHAIN_FILE` and `CMAKE_HOST_CMAKE_TOOLCHAIN_FILE`, which
are neither spelling. The mistake is quiet rather than loud: cargo delivers
whichever name you export, the crate never asks for that one, and the build
carries on with SDL's defaults — a successful build of the wrong configuration.
The list above was read off `cargo build -vv`, which prints all four lookups
and their answers (`cmake`'s `getenv_os` does a `println!` per probe):

```
[sdl3-sys 0.7.1+SDL-3.4.16] CMAKE_TOOLCHAIN_FILE_x86_64-unknown-linux-gnu = None
[sdl3-sys 0.7.1+SDL-3.4.16] CMAKE_TOOLCHAIN_FILE_x86_64_unknown_linux_gnu = None
[sdl3-sys 0.7.1+SDL-3.4.16] HOST_CMAKE_TOOLCHAIN_FILE = Some("…/cmake/sdl-options.cmake")
[sdl3-sys 0.7.1+SDL-3.4.16] CMAKE_TOOLCHAIN_FILE = None
```

Forms 3 and 4 are now **set for you** by `.cargo/config.toml` § `[env]`, one per
kind, so a plain `cargo build` gets `cmake/sdl-options.cmake` and a plain
`cargo build --target aarch64-unknown-linux-gnu` gets
`cmake/aarch64-toolchain.cmake` with no export at all. §4.2's manual export is
now an *override* rather than a requirement, and still wins, because form 2 is
probed before form 3.

Form 4 is deliberately left unset, and that is a decision rather than an
oversight. Setting the bare name would mean an aarch64 build whose operator
forgot to export anything falls back to a file with no `CMAKE_SYSTEM_NAME`, so
SDL configures with the host compiler and produces x86-64 objects under an
aarch64 target directory — the quiet failure §4.2 and §7.8 exist to catch. With
no bare name there is nothing to fall back to, and the mistake stays loud.

### 4.1.1 Editing a CMake file does not rebuild SDL

`sdl3-sys`'s `build.rs` emits **no `rerun-if-changed` directive at all**, so
cargo's fallback applies: the build script re-runs when a file *inside the
package* changes. `cmake/sdl-options.cmake`, `cmake/aarch64-toolchain.cmake` and
`.cargo/config.toml` are all outside it. Editing any of them and running
`cargo build` therefore does nothing to SDL, and reports success.

**After changing any of them, force the reconfigure:**

```sh
cargo clean -p sdl3-sys --manifest-path ui/Cargo.toml
```

Then check the artefact rather than trusting the exit status — §5.5 has the
three greps that tell you whether the options took.

### 4.2 The commands

```sh
# the repo root
export CMAKE_TOOLCHAIN_FILE_aarch64_unknown_linux_gnu="$PWD/cmake/aarch64-toolchain.cmake"

cargo build --target aarch64-unknown-linux-gnu --manifest-path ui/Cargo.toml
```

**The export is no longer required.** `.cargo/config.toml` § `[env]` sets
`TARGET_CMAKE_TOOLCHAIN_FILE` to the same path, so the cross build picks the
toolchain file up with no environment set up at all — verified 2026-10-05, with
the variable deliberately unset and the artifact still `ARM aarch64`. The export
above is kept because it is an explicit override that outranks the config file,
and because it is what a reader should try first when the cross build misbehaves.

`--manifest-path` is needed because the workspace root is `ui/Cargo.toml`, not
the repository root; the two snippets below add an environment variable to the
last command and keep its arguments.

**This command has been run, 2026-09-28, and it succeeds.** From scratch, into a
throwaway target directory, with no sysroot: exit 0 in 46.7 s, and the artifact
is the architecture it claims to be.

```
$ aarch64-linux-gnu-readelf -h …/aarch64-unknown-linux-gnu/debug/ui_demo \
    | grep -E 'Class|Machine|Type'
  Class:   ELF64
  Type:    DYN (Position-Independent Executable file)
  Machine: AArch64
```

The build carries this tree's thirteen source files — twelve in
`libui_core.rlib`, one in the binary itself — and the SDL it links is the
vendored 3.4.16. What it does **not** show is a target that runs: no sysroot, no
runtime, no video driver (§6.7) — and the artifact still asks for
`libSDL3.so.0` at load time, with no `RPATH` or `RUNPATH` to say where that
should come from. That is the open linkage question, recorded in
`doc/ui/IMPLEMENTATION_STATE.md` § *History*.

**The linker comes from `.cargo/config.toml`,** which sets
`[target.aarch64-unknown-linux-gnu] linker = "aarch64-linux-gnu-gcc"`. It is at
the repository root and not under `ui/`, because Cargo reads config from the
working directory and its ancestors and never from beside a manifest. Without it
rustc drives the host `cc`, an x86_64 compiler, and the link fails with
`rust-lld: error: --fix-cortex-a53-843419 is only supported on AArch64` — an
aarch64 hardening flag rustc always passes, rejected by the host linker. Nothing
else in the target build is a Cargo setting; the SDL options come from the
toolchain file and from the manifest's features, so a native build gets X11 and
the target does not.

`ROADOS_ARCH_FLAGS` defaults to `-march=armv8-a`. If the head unit SoC supports
more and the renderer can use it:

```sh
ROADOS_ARCH_FLAGS="-march=armv8.2-a+crc" \
  cargo build --target aarch64-unknown-linux-gnu --manifest-path ui/Cargo.toml
```

With a real sysroot — Buildroot, Yocto, a hand-made rootfs:

```sh
ROADOS_SYSROOT=/opt/sysroot-aarch64 \
  cargo build --target aarch64-unknown-linux-gnu --manifest-path ui/Cargo.toml
```

### 4.3 The toolchain file

`cmake/aarch64-toolchain.cmake`. Every value in it is overridable, in this order:
`-D<var>=<value>` on the cmake command line, then the environment variable of
the same name (`ROADOS_*` only), then the file's default.

| Variable | Default | Effect |
|---|---|---|
| `ROADOS_TARGET_TRIPLE` | `aarch64-linux-gnu` | prefixes every tool; also the multiarch dir name |
| `ROADOS_SYSROOT` | *(unset)* | when set: becomes `CMAKE_SYSROOT` + `CMAKE_FIND_ROOT_PATH` |
| `ROADOS_PREFIX_cc` / `_cxx` / `_ar` / `_ranlib` / `_strip` | `<triple>-` | tool name prefixes |
| `ROADOS_ARCH_FLAGS` | `-march=armv8-a` | appended to `CMAKE_C_FLAGS` and `CMAKE_CXX_FLAGS` |
| `ROADOS_TRY_COMPILE_TARGET_TYPE` | `EXECUTABLE` | set to `STATIC_LIBRARY` to configure without linking |

Three decisions in that file are worth stating outright, because each of them
was a bug before it was a comment:

- **`CMAKE_SYSROOT` is not set by default.** On Debian/Ubuntu,
  `gcc-aarch64-linux-gnu` already knows its target layout
  (`/usr/aarch64-linux-gnu/{include,lib}` plus multiarch search paths), and that
  directory is a target *library* root, not a sysroot — it has no
  `<sysroot>/usr/include` inside it. Pointing `CMAKE_SYSROOT` at it breaks the
  build. Set `ROADOS_SYSROOT` only when the target really is a sysroot tree.
- **`-march` is appended to `CMAKE_C_FLAGS`, not seeded through
  `CMAKE_C_FLAGS_INIT`.** `cmake` 0.1.58 always passes
  `-DCMAKE_C_FLAGS=<flags derived from cc::Build>` on the command line, so
  `CMAKE_C_FLAGS` is already a cache entry before the toolchain file is read and
  `CMAKE_C_FLAGS_INIT` is ignored. Verified: with the pre-seeded value present,
  the result is ` -ffunction-sections -fdata-sections -fPIC -w -march=armv8-a` —
  the flag survives, and is not duplicated on the re-reads the toolchain file
  gets for every `try_compile`.
- **The `CMAKE_FIND_ROOT_PATH_MODE_*` settings are not a host-isolation
  guarantee.** Measured: with an empty `CMAKE_FIND_ROOT_PATH`, SDL's configure
  still resolved the host's `/usr/include/X11/XKBlib.h` and host `libX11`. What
  actually keeps host libraries off the link line is SDL's own
  `SDL_DEPS_SHARED=ON` default, which records sonames and `dlopen`s them on the
  target. The moment you turn that off — `SDL_DEPS_SHARED=OFF`, statically
  linked platform dependencies — a sysroot stops being optional.

### 4.4 Using the toolchain file outside cargo

Same option set as §3, since §6.4(b) is run this way:

```sh
cmake -S <sdl-src> -B /tmp/sdl-aarch64 \
  -DCMAKE_TOOLCHAIN_FILE="$PWD/cmake/aarch64-toolchain.cmake" \
  -DCMAKE_BUILD_TYPE=Release \
  -DSDL_EXAMPLES=OFF -DSDL_TESTS=OFF -DSDL_REVISION=SDL-3.4.16 \
  -DSDL_AUDIO=OFF -DSDL_RENDER=OFF -DSDL_CAMERA=OFF \
  -DSDL_X11_XCURSOR=OFF -DSDL_X11_XRANDR=OFF -DSDL_X11_XSCRNSAVER=OFF
```

The three `SDL_X11_X*=OFF` flags are this host's, not the target's — see §2.3 and
§3. A cross build against a real sysroot that has the X11 packages does not need
them, and a target with neither X11 nor Wayland needs the opposite
(`-DSDL_UNIX_CONSOLE_BUILD=ON`, §4.5).

`CMAKE_PREFIX_PATH` also works, and is read by `cmake` 0.1.58 the same
target-aware way, so `CMAKE_PREFIX_PATH_aarch64_unknown_linux_gnu=<target
prefix>` points `find_package` at target libraries without touching the toolchain
file.

### 4.5 Two things that will bite

- **Always pass `--target` to cargo.** Without it, `TARGET` is the host triple,
  `cc::Build` derives host flags, and `cmake` 0.1.58 bakes them into
  `-DCMAKE_C_FLAGS= -ffunction-sections -fdata-sections -fPIC -m64 -w`. `-m64` on
  an aarch64 build is a hard error from the compiler, which is at least loud.
  The `cc` version to check this against is the one `sdl3` 0.20.0's own
  `Cargo.lock` pins — **`cc` 1.2.41** — where the emission is guarded at
  `src/lib.rs:2177-2178`:

  ```rust
  } else if target.arch == "x86_64" || target.arch == "powerpc64" {
      cmd.args.push("-m64".into());
  ```

  so aarch64 does not get `-m64` even so. The same guard exists in `cc` 1.5.1 at
  `src/lib.rs:2483-2484`, so the conclusion is not an artefact of one version.
  Either way this flag should not appear once `--target` is correct; the point
  of naming it is the underlying habit.
- **The target-scoped environment variable (§4.2) is silently ignored without
  `--target`.** `cmake` 0.1.58 looks up
  `CMAKE_TOOLCHAIN_FILE_aarch64_unknown_linux_gnu` only when the target triple
  is aarch64; building for the host, it never consults the variable, so you get
  a plain native build and no warning. §4.2 recommends the scoped form precisely
  because it does not leak into native builds — and this is the same property
  seen from the other side: the mistake is *quieter* than exporting the
  unscoped `CMAKE_TOOLCHAIN_FILE`, which at least fails loudly. If a build
  produced x86-64 objects after you set the variable, check for a missing
  `--target` before anything else (§7.8).
- **A bare cross build has no X11 or Wayland in the sysroot, and SDL treats that
  as an error** (`cmake/macros.cmake:413-427`): a `FATAL_ERROR` unless
  `SDL_UNIX_CONSOLE_BUILD` is on. The console-build switch is reachable from
  `Cargo.toml` as the `sdl-unix-console-build` feature on `sdl3-sys`, exposed on
  `sdl3` as `build-from-source-unix-console`. It is not a good fit for a head
  unit, which is the other reason to build a sysroot rather than lean on the
  Debian cross toolchain.

---

## 5. SDL3 vendored build configuration

### 5.1 How subsystems are switched — and why `sdl3` alone cannot do it

`PRIMITIVES_ARCHITECTURE.md` used to say audio, render, camera and filesystem are
disabled at build time, and that a named set of subsystems stays. **It no longer
says that** — the paragraph was reconciled on 2026-10-05, and the reason is in
§5.4. Two things still have to be established for anyone who wants to act on the
old text, and the first is a blocker for the second.

**`sdl3` 0.20.0 re-exports no subsystem features at all.** Its complete feature
table is: `ash`, `build-from-source`, `build-from-source-static`,
`build-from-source-unix-console`, `debug-impls`, `default`, `gfx`, `hidapi`,
`image`, `link-framework`, `main`, `mixer`, `net`, `raw-window-handle`,
`static-link`, `test-mode`, `ttf`, `unsafe_textures`, `use-pkg-config`,
`use-vcpkg`. Not one of them is an SDL subsystem, and none of them forwards one.
Adding `no-sdl-audio` to `sdl3 = { ... }` is a hard error, not a no-op:

```text
error: the package `roados` depends on `sdl3`, with features: `no-sdl-audio`
but `sdl3` does not have these features.
```

The subsystem features live one level down, on `sdl3-sys` 0.7.1, which maps them
to SDL CMake variables in one place — the `cmake_vars!` block,
`build.rs:31-48` — such that for each of `SDL_AUDIO`, `SDL_VIDEO`, `SDL_GPU`,
`SDL_RENDER`, `SDL_CAMERA`, `SDL_JOYSTICK`, `SDL_HAPTIC`, `SDL_HIDAPI`,
`SDL_POWER`, `SDL_SENSOR`, `SDL_DIALOG`, `SDL_TRAY` it emits `-D<NAME>=ON` when
feature `sdl-<name>` is on and `-D<NAME>=OFF` when `no-sdl-<name>` is. The same
block carries `SDL_UNIX_CONSOLE_BUILD`, `SDL_RPATH`, `SDL_LIBC`, `SDL_ASAN` and
`SDL_CCACHE`.

`SDL_STATIC` is **not** one of them, and is worth calling out because it looks
like it should be. It is set separately, directly from the `link-static`
feature at `build.rs:26-27`:

```rust
} else if cfg!(feature = "link-static") {
    config.define("SDL_STATIC", "ON");
}
```

So there is no `sdl-static` cargo feature to look for; the feature is called
`link-static`, and it does not go through the `no-`/`sdl-` naming scheme.
There is no other way to pass an SDL option in — see §7.7.

So the subsystem configuration the architecture doc asks for requires **a second
direct dependency** on `sdl3-sys`. Cargo unifies features across the graph, so
this works, and the crate is already in the build:

```toml
[dependencies]
sdl3     = { version = "0.20",   features = ["build-from-source"] }
sdl3-sys = { version = "0.7.1", features = [
    "no-default-subsystems",   # all twelve off; an allowlist, not a denylist
    "sdl-video",
    "sdl-joystick",
    "sdl-hidapi",
] }
glow     = "0.18"
```

Note the two version requirements are now both live, and they must be kept
compatible by hand: `sdl3` 0.20.0 requires `sdl3-sys ^0.7.1`, and
`0.7.1+SDL-3.4.16` satisfies it, so the two requirements resolve to the same
crate. That is a maintenance cost this document cannot make disappear, and it is
why the alternative below exists.

What the allowlist does and does not cover:

- `no-default-subsystems` is the right shape: a subsystem added by a future SDL
  release is off by default rather than silently on.
- `sdl-video` covers video, the event queue, keyboard and mouse. They are not
  separate subsystems — keyboard and mouse are parts of `SDL_VIDEO`, so there is
  no `sdl-keyboard` to keep.
- `sdl-joystick` and `sdl-hidapi` are **independent** features on `sdl3-sys`
  (all three of `sdl-joystick`, `sdl-hidapi`, `sdl-haptic` are declared `= []`
  — no implied edges), and `SDL_JOYSTICK_HIDAPI` is a *sub-option* of
  `SDL_HIDAPI` in SDL's own CMake. So omitting `sdl-hidapi` does not cost you
  HID joysticks as a side effect of `sdl-joystick`; it turns the HID driver off
  outright, and USB gamepads and steering wheels stop enumerating. evdev
  (`SDL_JOYSTICK_LINUX`) survives on its own. Whether HID is wanted is a
  hardware question, so it is left in the example and flagged in §8.
- `sdl-haptic` is left out. `PRIMITIVES_ARCHITECTURE.md` does not mention
  haptics; a steering wheel with force feedback would need it.
- `sdl-gpu` stays out. It is not in the architecture doc's keep-list and it pulls
  Vulkan into the dependency surface. Note that `SDL_VULKAN` is a *separate*,
  independently-defaulted-on option and it is on in a default configure — that
  is the `GPU drivers: vulkan` line in §3, and it is unaffected by
  `no-sdl-gpu`.
- `sdl-audio` is deliberately **not** in the allowlist. With the architecture
  doc's list, SDL ends up with no audio backend at all, which is presumably the
  intent; see §8 if the head unit is expected to make sound.

**The alternative: force the options from a toolchain file** — see §7.7. That
needs no second dependency and keeps the configuration in one place, but an
earlier draft of this paragraph claimed it was inherently target-asymmetric,
"a native x86_64 build passes no toolchain file and would get a *full* SDL".
**That was wrong, and it was wrong about the mechanism rather than about taste.**
A native build does pass no toolchain file *by default* — but the `cmake` crate
reads one from the environment for any target (§4.1), so there is no asymmetry
in the mechanism, only in the file that was there. §5.5 records the fix: the
options that do not differ per target moved into `cmake/sdl-options.cmake`,
which **both** targets read, the cross target through
`cmake/aarch64-toolchain.cmake` and the native one directly.

The subsystems themselves are still on SDL's defaults, and that is the
operator's decision of 2026-09-28 with the channel blocked — see §5.4. What
changed on 2026-10-05 is that three options which no cargo feature can reach are
now set on both targets. The allowlist above remains the way to reach the other
twelve, and it remains undecided; the reason it is still undecided is not a
mechanism problem and is recorded as such.

### 5.1.1 What `no-default-subsystems` costs

For the record, because it is invisible in the generated header: turning a
subsystem off with `-D<NAME>=OFF` sets `<NAME>_DISABLED 1` in
`SDL_build_config.h`, which compiles the subsystem out, and flips its line in
the `Subsystems:` block (§3) from `ON` to `OFF`. Nothing in the Rust API changes
shape, so the failure appears at runtime as an `SDL_Init` error rather than at
compile time.

Note that the subsystem options do *not* use the `(Wanted: ...)` summary format
that `dep_option()` options do — `grep 'SDL_AUDIO '` over the configure output
finds nothing. Only the `Subsystems:` block and the generated header report
them.

### 5.2 Filesystem cannot be disabled

**Settled 2026-10-05: it stays, and the architecture document now says so.** This
subsection is kept because the mechanism is the reason, and because "there is no
switch" is a claim that is easy to re-derive wrongly.

The architecture document used to list filesystem among the subsystems to
disable. It is not possible with this dependency set, and the reason is
structural rather than a matter of finding the right flag:

- SDL 3.4.16 declares its subsystems through a `define_sdl_subsystem()` macro
  (`CMakeLists.txt:238-263`), and the list is Audio, Video, GPU, Render, Camera,
  Joystick, Haptic, Hidapi, Power, Sensor, Dialog, Tray. There is no filesystem
  entry, and no `SDL_FILESYSTEM` option anywhere in the file. The `Subsystems:`
  block in §3 is the configure output rendering that same list — twelve entries,
  and filesystem is not among them.
- `sdl3-sys` 0.7.1 correspondingly has no `sdl-filesystem` / `no-sdl-filesystem`
  feature.
- The generated `SDL_build_config.h` from the configure in §3 shows the other
  three switches working and the fourth having no switch at all:

```c
#define SDL_AUDIO_DISABLED 1
#define SDL_RENDER_DISABLED 1
#define SDL_CAMERA_DISABLED 1
#define SDL_FILESYSTEM_UNIX 1
```

On Linux, SDL 3.4.16 always compiles its Unix filesystem implementation. It is a
few kilobytes of `open`/`stat` plumbing, and it is also what `SDL_GetPrefPath`
and friends need — worth knowing before anyone trades it away. Measured, the
object subtree is **72 KB** of the 9.3 MB `src/` tree in a release build.

**That measurement is object bytes, not binary bytes**, and the difference is not
academic: the linker drops unreferenced members of the static archive, so the
binary saving from any of these options is strictly smaller than the object
figure and cannot be derived from it. Any future claim about what a subsystem
costs has to come from a link, not from `du` on an object tree.

### 5.2.1 The subsystem policy, decided

**All twelve subsystems are on, at SDL's defaults. The operator decided this on
2026-09-28**, knowing the alternative was blocked by §5.1, and it is recorded as
a deviation in `IMPLEMENTATION_STATE.md`.

Two of the four subsystems the architecture document proposed to disable turned
out to be **product requirements**, which is a stronger reason than the blocked
channel:

- **audio** — `IDEA.md` § *Audio and media* asks for zones, source priority,
  ducking, FM/DAB+ and USB playback. It is a headline feature.
- **camera** — `IDEA.md:60,107,183`: automatic headlights driven from a camera,
  and recognition behind it. The V4L2 driver compiles in.

Of the other two, **filesystem** has no switch (§5.2) and **render** is unused
because `ui_core` draws through its own GLES pipeline and never reaches
`sdl3::render`.

**And the unused ones stay on by decision, not by oversight.** `render`, `gpu`,
`dialog`, `tray` and `power` are compiled and nothing calls them. The operator's
ruling of 2026-10-05 is that trimming megabytes off a static archive is not a
reason to narrow what the product can do, and that `power` in particular is a
battery API (`SDL_GetPowerInfo`, the whole of the category) which a head unit on
the vehicle's supply has no use for. `sensor` is the one genuinely undecided
option; see §5.4.

### 5.3 What the options come out as

The configure in §3, with `SDL_AUDIO=OFF SDL_RENDER=OFF SDL_CAMERA=OFF`, produced
`SDL_AUDIO_DISABLED`, `SDL_RENDER_DISABLED` and `SDL_CAMERA_DISABLED` in the
generated header, and kept:

```
#define SDL_JOYSTICK_HIDAPI 1
#define SDL_JOYSTICK_LINUX 1
#define SDL_JOYSTICK_VIRTUAL 1
#define SDL_VIDEO_DRIVER_DUMMY 1
#define SDL_VIDEO_DRIVER_OFFSCREEN 1
#define SDL_VIDEO_DRIVER_X11 1
#define SDL_VIDEO_OPENGL_ES2 1
#define SDL_VIDEO_OPENGL_EGL 1
```

Read `SDL_JOYSTICK_LINUX` + `SDL_JOYSTICK_HIDAPI` in
`SDL_build_config.h` as the two joystick backends that matter on a head unit:
evdev and HID. `SDL_VIDEO_DRIVER_DUMMY` and `SDL_VIDEO_DRIVER_OFFSCREEN` are
always compiled in and cost nothing.

### 5.4 Video drivers: native keeps X11, the target is framebuffer-only

**This is a design decision, not a footnote.** The two builds have deliberately
different *video driver* configurations. The mechanism that expresses it is
asymmetric by content — the target has a target identity, an `-march` and a
sysroot branch that mean nothing to x86_64 — but **not** by kind: §4.1 records
that the same channel serves either target, and §5.5 is the shared file that
makes "the same options on both targets" true rather than aspirational.

| | native (dev host) | aarch64 (head unit) |
|---|---|---|
| X11 | **on** | **off** |
| Wayland | off | **off** |
| KMSDRM | not detected here (§6.7) | **required** |
| `SDL_UNIX_CONSOLE_BUILD` | on, harmlessly | on, **mandatory** |
| `SDL_VULKAN`, `SDL_OPENGL`, `SDL_TEST_LIBRARY` | **off** | **off** |

The last row is the shared part: `cmake/sdl-options.cmake`, reached natively
through `HOST_CMAKE_TOOLCHAIN_FILE` and on the target through
`cmake/aarch64-toolchain.cmake`'s `include()`. §5.5 has the measured effect.

Native keeps X11 because the dev host runs the task 24 demo in a real window.
The head unit must not carry a desktop stack at all: it scans out to a panel
through KMSDRM, and X11 or Wayland on it would mean depending on a compositor
that does not exist.

**Why the target needs no X11 packages.** Not because they are "harmless if
unused" — because SDL never looks for them. `CheckX11()`
(`cmake/sdlchecks.cmake:273-564`) opens with a check-state push and then
`if(SDL_X11)`, and `find_package(X11)` plus all nine `SDL_X11_X<NAME>` extension
probes are nested inside that one condition:

```
macro(CheckX11)                                  <- sdlchecks.cmake:273
  cmake_push_check_state()                        <- 274
  if(SDL_X11)                                    <- 275, first thing tested
    ...
    find_package(X11)                            <- nested, never runs when OFF
    ...
      if(SDL_X11_XCURSOR) ... SDL_missing_dependency(XCURSOR SDL_X11_XCURSOR)
      if(SDL_X11_XDBE)    ...
      if(SDL_X11_XINPUT)  ...
      if(SDL_X11_XFIXES)  ...
      if(SDL_X11_XRANDR)  ...
      if(SDL_X11_XSCRNSAVER) ...
      ... and xshape, xsync, xtest
```

With `SDL_X11=OFF` the whole block is skipped, so `libxcursor-dev`,
`libxrandr-dev`, `libxss-dev` and the rest are not consulted for the target at
all. This is the asymmetry's whole point: it is what makes "no desktop stack"
a build-time fact rather than a promise.

**Why native X11 costs nothing at link time.** Also measured, not assumed.
`SDL_DEPS_SHARED=ON` is the SDL default, so the X11 driver is compiled in and
its libraries are reached by `dlopen()` at runtime, not linked. The native
binary built on this host has exactly four dynamic dependencies:

```
$ ldd target/debug/roados-native-probe | sort
	/lib64/ld-linux-x86-64.so.2
	libc.so.6 => /usr/lib/x86_64-linux-gnu/libc.so.6
	libgcc_s.so.1 => /usr/lib/x86_64-linux-gnu/libgcc_s.so.1
	linux-vdso.so.1
```

No `libX11`, no `libXcursor`, no `libGL`, no `libEGL`. The sonames live in the
generated header instead and are resolved at startup:

```c
#define SDL_VIDEO_DRIVER_X11 1
#define SDL_VIDEO_DRIVER_X11_DYNAMIC "libX11.so.6"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XCURSOR "libXcursor.so.1"
/* ... 7 more extension sonames ... */
```

So installing those X11 development packages on the dev host does **not** create
an X11 dependency in the artifact — the direct answer to the question that made
§2.3's package list look onerous. The `ldd` output is in §6.6.

**How the split is enforced, and why it is asymmetric.** `SDL_X11` and
`SDL_WAYLAND` are **not** in `sdl3-sys`'s `cmake_vars!` list
(`build.rs:30-48`), so there is no way to reach them from `Cargo.toml` — no
feature, no `-D` passthrough, no environment variable. The toolchain file is the
only channel, and a toolchain file is cross-only by construction. Hence:

```cmake
# in cmake/aarch64-toolchain.cmake
if(NOT DEFINED SDL_X11)      # -D wins; the block only supplies a default
  set(SDL_X11 OFF CACHE BOOL "" )
endif()
```

`SDL_UNIX_CONSOLE_BUILD` is the exception, and it goes in the **manifest**
instead — see below. Putting the two in different places is not tidiness: it
follows from which ones the crate re-exports.

> **A reader who "fixes" the asymmetry by making native match cross will break
> the demo.** Removing those two `OFF` lines, or adding `SDL_X11=ON` for
> symmetry, does not make the builds agree — it stops the dev host opening a
> window. The asymmetry is the design, not an oversight to be tidied away.

**The matched pair: `SDL_UNIX_CONSOLE_BUILD` is mandatory for the target.**
Turning both desktop drivers off is not sufficient on its own. SDL then reaches
this (`cmake/macros.cmake:413-414`):

```cmake
if(NOT (HAVE_X11 OR HAVE_WAYLAND))
  if(NOT SDL_UNIX_CONSOLE_BUILD)
    message(FATAL_ERROR
      "SDL could not find X11 or Wayland development libraries on your system. "
      ...)
```

so the configure dies unless `SDL_UNIX_CONSOLE_BUILD` is set. One without the
other fails, in both directions. It belongs in `Cargo.toml` because, unlike the
two above, it **is** re-exported by `sdl3`
(`Cargo.toml: build-from-source-unix-console = ["sdl3-sys/sdl-unix-console-build"]`):

```toml
sdl3 = { version = "0.20", features = [
    "build-from-source",
    "build-from-source-unix-console",
] }
```

**A single manifest serves both targets**, which an earlier revision of this
document said was impossible ("it applies to the whole build, not per target, so
native and aarch64 would need different manifests"). That is wrong, and the
reason is that `SDL_UNIX_CONSOLE_BUILD` is read at exactly **one** place in the
whole of SDL 3.4.16 — the `if` quoted above — and enables no driver, no source
file and no `SDL_BUILD_CONFIG` entry. Natively the check it suppresses would
have passed anyway; cross it suppresses a `FATAL_ERROR`. Turning it on
unconditionally costs nothing and is required. The X11/Wayland forcing is
target-asymmetric precisely because those two are unreachable from the manifest.

**KMSDRM is required on the target and is not configured here yet.**
`SDL_KMSDRM` is likewise absent from the `cmake_vars!` list, so it too would need
forcing from the toolchain file — and it already defaults to `ON` on Unix
(`CMakeLists.txt:375`, `dep_option(... ${UNIX_SYS} ...)`), so the default is not
the problem. The problem is that enabling it needs `libdrm` and `gbm` `.pc`
files **for the target**, and the target image is deferred (§8). Forcing it on
before those exist would add a step that cannot be verified, so the toolchain
file deliberately leaves it alone. When the sysroot is settled this is the
mechanism, identical to the two `OFF` lines above.

**A note on the earlier reading of this section.** A previous revision claimed
`SDL_KMSDRM` had been switched on in the cross configure and off in the native
one by the difference in `pkg-config`, and drew the lesson that
`CMAKE_FIND_ROOT_PATH_MODE_*` are not a host-isolation guarantee. That was
correct about the leak, and the stub-`pkg-config` evidence for it stands
(§6.4.1). What it got wrong was leaving the impression that KMSDRM was a choice
this document was making. It was never a choice: `SDL_KMSDRM` defaults to `ON`
on Unix, and the native build simply failed to satisfy it — silently, for the
reason in §6.7.

---

### 5.5 What changed on 2026-10-05, and how to check it

Three SDL options that **no cargo feature can reach** are now off on both
targets. They are `SDL_VULKAN`, `SDL_OPENGL` and `SDL_TEST_LIBRARY`, and none of
them is in `sdl3-sys`'s `cmake_vars!` list (`build.rs:30-48`), which covers the
twelve subsystems plus `SDL_ASAN`, `SDL_CCACHE`, `SDL_LIBC`, `SDL_RPATH` and
`SDL_UNIX_CONSOLE_BUILD` — nothing else. The channel is therefore the
environment, via `cmake/sdl-options.cmake`; the reasoning for each option is in
that file's header, and the wiring is §4.1.

**Why each one, in one line each.** Vulkan was rejected on the merits
(`PRIMITIVES.md` § *Backend`), and SDL's GPU API cannot do OpenGL ES at all.
Desktop GL is the GLX path, this project renders GLES 3.1 through EGL, and the
two are independent SDL options — but `SDL_OPENGL` defaulted **on** and was only
being defeated by `libgl-dev` being absent on this host, so the artefact
depended on the dev machine's package list. `SDL_TEST_LIBRARY` builds a static
`SDL3_test` nothing links and runs a `libunwind` probe whose result cannot reach
the library header (§6.7).

**The check.** These three greps are the whole verification, and they are worth
more than the exit status, because a CMake file edit does not trigger a rebuild
(§4.1.1):

```sh
d=$(dirname "$(dirname "$(dirname "$(ls ui/target/release/build/sdl3-sys-*/out/build/include-config-release/build_config/SDL_build_config.h | head -1)")")")

# 1. the options took, in the cache
grep -E '^SDL_(VULKAN|OPENGL|TEST_LIBRARY):' "$d/CMakeCache.txt"
#   SDL_OPENGL:BOOL=OFF
#   SDL_TEST_LIBRARY:BOOL=OFF
#   SDL_VULKAN:BOOL=OFF

# 2. and in the generated header
grep -E 'SDL_(VIDEO_VULKAN|GPU_VULKAN|VIDEO_RENDER_VULKAN|VIDEO_OPENGL)\b' \
    "$d/../include-config-release/build_config/SDL_build_config.h"
#   /* #undef SDL_VIDEO_RENDER_VULKAN */
#   /* #undef SDL_VIDEO_OPENGL */
#   /* #undef SDL_VIDEO_VULKAN */
#   /* #undef SDL_GPU_VULKAN */

# 3. no Vulkan code survived, and the test library is gone
find "$d/CMakeFiles/SDL3-static.dir" -iname '*vulkan*' -name '*.o' | wc -l   # 5, all empty
ls "$d/libSDL3_test.a" 2>/dev/null || echo "libSDL3_test.a: GONE"           # GONE
```

Five `*vulkan*.o` files remain and **that is correct**: SDL globs whole
directories (`sdlchecks.cmake:330` takes all of `src/video/x11/*.c`), and each of
those files is internally `#ifdef SDL_VIDEO_VULKAN`. With the option off they
compile to empty translation units. The check that matters is
`nm --defined-only` on them — **0 defined symbols each**, against 716 KB of real
objects before.

**Measured, release, x86_64, same machine and toolchain:**

| | before | after | delta |
|---|---|---|---|
| `libSDL3.a` | 7 623 342 | 6 819 806 | **−803 536 (−10.5 %)** |
| `ui_demo` | 6 546 728 | 6 137 376 | **−409 352 (−6.3 %)** |
| `libSDL3_test.a` | 228 180 | gone | −228 180 |
| `*vulkan*.o` | 716 K | 20 K, 0 symbols | −696 K |

The aarch64 cross artifact went 6 013 800 → 5 759 648 (−254 152). Note the two
figures are not proportional — 803 KB off the archive is 409 KB off the binary —
which is the object-tree-versus-linker point from §5.2 in the only terms that
settle it.

**Also verified, and the reason to believe any of it:** 1 839 tests green
(1 404 + 217 + 218), and `.ai/tools/fps-check.sh` at **61.9 fps, worst frame
24.3 ms, 0 frames over 33 ms** over 10 s — against a recorded baseline of
61.6–61.9 fps, so no frame-cost regression. The cross build was re-run with
`CMAKE_TOOLCHAIN_FILE` deliberately unset to prove §4.1's `[env]` wiring stands
on its own, and the artifact is `ELF 64-bit LSB pie executable, ARM aarch64`
with `SDL_X11` off, `SDL_UNIX_CONSOLE_BUILD` on, `SDL_VIDEO_OPENGL_EGL` and
`SDL_JOYSTICK_HIDAPI` on — every one of those unchanged by this work.

---

## 6. Verification

Everything below was run on 2026-09-28 on Ubuntu 26.04.1, CMake 4.2.3, with **no
aarch64 cross toolchain installed**. Commands are given verbatim; output is
quoted, not summarised.

The Rust version differs between subsections, because the operator ran
`rustup update stable` partway through and the evidence on either side of that
has to be attributed correctly:

- **§6.1–§6.5: cargo/rustc 1.80.1.** These are `cmake`-only runs, so the Rust
  version does not affect them; §6.3 is the one that invokes `cargo build`, and
  it did so on 1.80.1.
- **§6.6–§6.7: cargo/rustc 1.98.1**, after the update. These are the runs that
  build the real dependency set, so the version is load-bearing rather than
  incidental.
- **§6.8: version-independent** (a `glxinfo` result).

### 6.1 Toolchain file parses and sets what it claims — **pass**

```sh
cmake -DPROBE_TOOLCHAIN=.../cmake/aarch64-toolchain.cmake -P probe-toolchain.cmake
```

```
-- ROADOS_TARGET_TRIPLE = aarch64-linux-gnu
-- ROADOS_ARCH_FLAGS = -march=armv8-a
-- CMAKE_SYSTEM_NAME = Linux
-- CMAKE_SYSTEM_PROCESSOR = aarch64
-- CMAKE_C_COMPILER = aarch64-linux-gnu-gcc
-- CMAKE_CXX_COMPILER = aarch64-linux-gnu-g++
-- CMAKE_AR = aarch64-linux-gnu-ar
-- CMAKE_RANLIB = aarch64-linux-gnu-ranlib
-- CMAKE_STRIP = aarch64-linux-gnu-strip
-- CMAKE_SYSROOT = <unset>
-- CMAKE_FIND_ROOT_PATH_MODE_PROGRAM = NEVER
-- CMAKE_FIND_ROOT_PATH_MODE_LIBRARY = ONLY
-- CMAKE_FIND_ROOT_PATH_MODE_INCLUDE = ONLY
-- CMAKE_FIND_ROOT_PATH_MODE_PACKAGE = ONLY
-- CMAKE_C_FLAGS =  -march=armv8-a
-- CMAKE_CXX_FLAGS =  -march=armv8-a
-- CMAKE_TRY_COMPILE_TARGET_TYPE = EXECUTABLE
-- PKG_CONFIG_EXECUTABLE = <unset>
-- PKG_CONFIG_SYSROOT_DIR = <unset>
-- PKG_CONFIG_LIBDIR = <unset>
-- SDL_X11 = OFF
-- SDL_WAYLAND = OFF
```

The last two lines are the video-driver block of §5.4, and they are here so the
everyday check covers it. With `ROADOS_SDL_X11=ON` exported, the same probe
reports `SDL_X11 = ON`, `SDL_WAYLAND = OFF` — the documented override, one
variable at a time.

With `ROADOS_SYSROOT=/opt/sysroot` and
`ROADOS_ARCH_FLAGS="-march=armv8.2-a+crc"` exported, the same probe reports
`CMAKE_SYSROOT = /opt/sysroot`, `CMAKE_FIND_ROOT_PATH = /opt/sysroot`,
`CMAKE_LIBRARY_ARCHITECTURE = aarch64-linux-gnu`, and
`PKG_CONFIG_LIBDIR = /opt/sysroot/usr/lib/aarch64-linux-gnu/pkgconfig;...`.

The `CMAKE_LIBRARY_ARCHITECTURE` line there is a script-mode result and holds
only because `project()` never runs here — in a real configure `project()`
overwrites it from the compiler it probed. §8, item 1 has that measurement;
`PKG_CONFIG_LIBDIR` value, which is built from `ROADOS_TARGET_TRIPLE`, is
unaffected either way.

With `-DPROBE_PREFLAGS=' -ffunction-sections -fdata-sections -fPIC -w'`
pre-seeding the cache the way the `cmake` crate does, and the toolchain file
included twice to simulate the `try_compile` re-reads:

```
-- CMAKE_C_FLAGS =  -ffunction-sections -fdata-sections -fPIC -w -march=armv8-a
```

Composed once, not twice.

### 6.2 CMake reads the file, and uses the cross compiler — **pass**

```sh
cmake -S trivial -B b1 -DCMAKE_TOOLCHAIN_FILE=.../cmake/aarch64-toolchain.cmake
```

```
-- The C compiler identification is unknown
CMake Error at CMakeLists.txt:2 (project):
  The CMAKE_C_COMPILER:

    aarch64-linux-gnu-gcc

  is not a full path and was not found in the PATH.
```

This is the expected result on a host with no cross toolchain, and it is the
proof that the file is read and that `CMAKE_C_COMPILER` is what it claims.

### 6.3 The `cmake` crate picks the toolchain file out of the environment — **pass**

A throwaway crate whose `build.rs` does what `sdl3-sys` does — `cmake::Config`,
`-DSDL_EXAMPLES=OFF -DSDL_TESTS=OFF`, no toolchain argument — built with:

```sh
CMAKE_TOOLCHAIN_FILE=.../cmake/aarch64-toolchain.cmake cargo build
```

```
[xprobe 0.0.0] HOST_CMAKE_TOOLCHAIN_FILE = None
[xprobe 0.0.0] CMAKE_TOOLCHAIN_FILE = Some(".../cmake/aarch64-toolchain.cmake")
[xprobe 0.0.0] CMAKE_PREFIX_PATH = None
[xprobe 0.0.0] running: cd ".../out/build" && CMAKE_PREFIX_PATH="" LC_ALL="C" \
  "cmake" "/tmp/opencode/xprobe/native" \
  "-DSDL_EXAMPLES=OFF" "-DSDL_TESTS=OFF" \
  "-DCMAKE_TOOLCHAIN_FILE=.../cmake/aarch64-toolchain.cmake" \
  "-DCMAKE_INSTALL_PREFIX=.../out" \
  "-DCMAKE_C_FLAGS= -ffunction-sections -fdata-sections -fPIC -m64 -w" \
  "-DCMAKE_BUILD_TYPE=Debug"
[xprobe 0.0.0] CMake Error at CMakeLists.txt:2 (project):
[xprobe 0.0.0]     aarch64-linux-gnu-gcc
[xprobe 0.0.0]   is not a full path and was not found in the PATH.
[xprobe 0.0.0] -- Configuring incomplete, errors occurred!
```

The environment variable reached the build script, the build script handed it to
`cmake` as a `-D`, and CMake read it. This is the whole crossbuild mechanism,
executed. Two incidental confirmations in the same trace: the `CMAKE_PREFIX_PATH`
lookups that follow the toolchain lookup (§4.4), and the `-DCMAKE_C_FLAGS=`
value seeded by `cc` — the reason §4.3 appends rather than initialises.

### 6.4 Full configure through the toolchain file, with a stand-in compiler — **partial**

**Superseded for the purposes of "does the cross build work": see §6.4.2, which
does it for real with the installed aarch64 toolchain and passes.** The runs below
predate that toolchain, and their x86_64 symlinks are the reason the verdict is
only partial. They are kept because they isolate two things §6.4.2 cannot show
separately — what the toolchain file does when the compiler *rejects* the arch
flag, and how a stubbed `pkg-config` fabricates availability. Read §6.4.2 first.

The host has no aarch64 compiler, so `aarch64-linux-gnu-gcc` was faked with a
symlink to the host's x86_64 `gcc`. That lets the file's CMake logic run for
real, but it splits the result, because the committed file's *default* is an
aarch64 flag that x86_64 `gcc` rejects. Three runs: (a) shows the default
reaching the compiler, (b) shows a completing configure, and (b′) shows the same
command failing for a different and intended reason.

**A note on the fake `PATH`, because it changes the output.** Two directories
are used. `fakebin` holds the x86_64 toolchain symlinks *plus* an
`aarch64-linux-gnu-pkg-config` stub that is a two-line `exit 0` — it answers
every query successfully, which fabricates library availability. `fakebin-nopc`
holds the same symlinks with no `pkg-config` at all, so SDL's
`find_package(PkgConfig)` genuinely fails. Runs (a) and the pre-amendment (b)
used `fakebin`; the runs quoted below use `fakebin-nopc`, because they exist to
show what the file does *and* what a host without `pkg-config` does, and a stub
that always succeeds would answer neither question honestly. See §6.4.1 for
what the stub was masking.

**(a) Default path — the arch flag reaches the compiler, and the configure then
fails because the compiler is x86_64.** Command:

```sh
env -u ROADOS_ARCH_FLAGS -u ROADOS_SYSROOT \
  PATH="/tmp/opencode/fakebin:$PATH" \
  cmake -S crates/sdl3-src-3.4.16/SDL -B sdlcross-clean \
    -DCMAKE_TOOLCHAIN_FILE=.../cmake/aarch64-toolchain.cmake \
    -DCMAKE_BUILD_TYPE=Release \
    -DSDL_EXAMPLES=OFF -DSDL_TESTS=OFF -DSDL_REVISION=SDL-3.4.16 \
    -DSDL_AUDIO=OFF -DSDL_RENDER=OFF -DSDL_CAMERA=OFF
```

The cache is written before the compiler check completes, and it holds the
default:

```
CMAKE_C_FLAGS:STRING= -march=armv8-a
```

and the compiler check shows the flag on the command line:

```
/tmp/opencode/fakebin/aarch64-linux-gnu-gcc   -march=armv8-a  -o \
  CMakeFiles/cmTC_216d6.dir/testCCompiler.c.o -c .../testCCompiler.c
cc1: error: bad value 'armv8-a' for '-march=' switch
CMakeLists.txt:8 (project)
-- Configuring incomplete, errors occurred!
```

That is the *correct* outcome: the file asked for aarch64 and got an x86_64
compiler. It is also, incidentally, the loud version of the mistake §4.5 warns
about.

**(b) With an x86-valid stand-in for the arch flag, and the video drivers the
toolchain file forces, the configure completes.** Same command plus
`ROADOS_ARCH_FLAGS='-mtune=generic'`, and — standing in for the manifest feature
`build-from-source-unix-console` of §5.4 — `-DSDL_UNIX_CONSOLE_BUILD=ON`:

```sh
env -u ROADOS_SYSROOT \
  PATH="/tmp/opencode/fakebin-nopc:$PATH" \
  ROADOS_ARCH_FLAGS='-mtune=generic' \
  cmake -S crates/sdl3-src-3.4.16/SDL -B sdlcross-x86-new \
    -DCMAKE_TOOLCHAIN_FILE=.../cmake/aarch64-toolchain.cmake \
    -DCMAKE_BUILD_TYPE=Release \
    -DSDL_EXAMPLES=OFF -DSDL_TESTS=OFF -DSDL_REVISION=SDL-3.4.16 \
    -DSDL_AUDIO=OFF -DSDL_RENDER=OFF -DSDL_CAMERA=OFF \
    -DSDL_UNIX_CONSOLE_BUILD=ON
```

> `fakebin-nopc` is described in the note at the top of this section: the same
> x86_64 symlinks with the `exit 0` `pkg-config` stub removed. The stub answered
> every query with success, which is why an earlier run of this subsection
> reported `kmsdrm` in the driver list and a host `libdrm.so` in the cache
> (§6.4.1). It is not used here, so the run reflects a host that genuinely has
> no `pkg-config` — which is the state §6.7 describes, and the state the target
> will be in unless waiver 10 is closed.

Exit status 0, and the drivers are what the toolchain file asked for:

```
-- Could NOT find PkgConfig (missing: PKG_CONFIG_EXECUTABLE)
-- Enabled backends:
--   Video drivers: dummy offscreen
--   Render drivers: ogl_es2
--   GPU drivers: vulkan
--   Audio drivers: dummy
--   Joystick drivers: hidapi linux virtual
--   Camera drivers: dummy
...
-- Configuring done (25.8s)
-- Generating done (0.1s)
```

**No `x11`, and no `kmsdrm`.** Both are the point:

```
$ grep -E '^SDL_X11:|^SDL_WAYLAND:|^SDL_UNIX_CONSOLE_BUILD:|^CMAKE_C_FLAGS:' \
    sdlcross-x86-new/CMakeCache.txt
CMAKE_C_FLAGS:STRING= -mtune=generic
SDL_UNIX_CONSOLE_BUILD:UNINITIALIZED=ON
SDL_WAYLAND:BOOL=OFF
SDL_X11:BOOL=OFF
```

`x11` is gone because the toolchain file forces it off, which is the
enforcement this subsection exists to show. `kmsdrm` is gone for the
independent reason of §6.7 — no `pkg-config` — which on a real target is the
failure the operator must not ship.

**The strongest single piece of evidence is an absence.** The nine X11
extension options do not appear in the cache at all:

```
$ grep -cE '^SDL_X11_X' sdlcross-x86-new/CMakeCache.txt
0
```

They *are* still printed by SDL's backends summary, but as `Wanted: OFF`:

```
--   SDL_X11_XCURSOR             (Wanted: OFF): OFF
--   SDL_X11_XRANDR              (Wanted: OFF): OFF
...
```

That is `dep_option(… SDL_X11 OFF)` (`CMakeLists.txt:342-350`) doing its job: with
the parent option off the dependent options are never created, so
`find_package(X11)` and every extension probe inside `CheckX11()` are skipped
and no X11 package is looked for. Compare the pre-amendment run of this
subsection, where the same cache held all nine:

```
SDL_X11_XCURSOR:BOOL SDL_X11_XDBE:BOOL SDL_X11_XFIXES:BOOL SDL_X11_XINPUT:BOOL \
SDL_X11_XRANDR:BOOL SDL_X11_XSCRNSAVER:BOOL SDL_X11_XSHAPE:BOOL SDL_X11_XSYNC:BOOL \
SDL_X11_XTEST:BOOL
```

and the flags actually used for the library, from the generated
`CMakeFiles/SDL3-shared.dir/flags.make`:

```
C_FLAGS =  -mtune=generic -O3 -DNDEBUG -fPIC -fvisibility=hidden -Wall -Wundef \
  -Wfloat-conversion -fno-strict-aliasing -Wshadow \
  -Wno-unused-local-typedefs -Wimplicit-fallthrough -fdiagnostics-color=always \
  -idirafter/tmp/opencode/crates/sdl3-src-3.4.16/SDL/src/video/khronos \
  -D_REENTRANT
```

No X11 include path and no X11 library in that line. The only two `X11`-bearing
tokens in the whole of `flags.make` are `-DEGL_NO_X11` and
`-DMESA_EGL_NO_X11_HEADERS`, which are SDL's own Khronos-header guards and
disable X11 *inside* `EGL/egl.h` — not X11 dependencies. The unambiguous measure
is the source list, where the X11 driver is simply not built:

```
$ grep -c 'src/video/x11/' sdlcross-x86-new/CMakeFiles/SDL3-shared.dir/build.make
0
```

against 360 matches in the pre-amendment run of this subsection, one per
translation unit of the same driver.

**(b′) Without the manifest feature, the same command fails — by design.** This
is the matched pair of §5.4, and it is worth quoting because "the build works"
and "the build works for the right reason" are different claims:

```sh
# identical to (b), minus -DSDL_UNIX_CONSOLE_BUILD=ON
```

```
--   SDL_X11                     (Wanted: OFF): OFF
...
CMake Error at cmake/macros.cmake:415 (message):
  SDL could not find X11 or Wayland development libraries on your system.
...
-- Configuring incomplete, errors occurred!
```

Exit status 1, and the cache still records `SDL_X11:BOOL=OFF` and
`SDL_WAYLAND:BOOL=OFF` with zero `SDL_X11_X*` entries. So the forcing works
independently of the manifest, and SDL's own sanity check fires exactly when the
manifest feature is missing. That is why `SDL_UNIX_CONSOLE_BUILD` is a manifest
requirement and not merely a convenience: it is load-bearing, and a build that
omits it stops with a diagnostic rather than producing a quietly driverless SDL.
The way to supply it is §5.1, not a `-D` on the cmake command line.

**What this does and does not prove.** It proves `CMAKE_SYSTEM_NAME`,
`CMAKE_SYSTEM_PROCESSOR`, `CMAKE_CROSSCOMPILING`, the toolchain resolution
order, the video-driver forcing of §5.4 including the `dep_option` cascade, the
`SDL_UNIX_CONSOLE_BUILD` dependency, and — via (a) — that the default arch flag
reaches the compiler through a real `project()`. It does not prove the result is
aarch64 code: the compiler was x86_64 throughout, and in (b) the arch flag was
deliberately replaced by an x86-valid one, which is why `CMAKE_C_FLAGS` there
shows no `-march`. It also does not prove the target configuration is *sufficient*
— (b) has no scanout driver at all, for the reason in §6.7.

**Why (a) is here, and why §6.1 is not enough on its own.** §6.1's probe is
`cmake -P`, *script* mode: it never calls `project()`, so it never runs a
compiler check. It can show that the file sets a variable; it cannot show that
the variable survives being overridden by a real configure, or that it reaches
a compiler command line. `CMAKE_C_FLAGS` is the variable to watch for exactly
that reason — `cc` seeds it on the command line (§4.3), so it is a cache entry
before the file is even read, and only a real configure will show what the file
does to it. Run (a) is what supplies that evidence: the flag appears in the cache
*and* on the `cc1` command line.

### 6.4.1 SDL's own build does not link here — a harness artefact, not a finding

Building run (b) fails:

```
SDL/src/core/linux/SDL_dbus.h:29:10: fatal error: dbus/dbus.h: No such file or directory
make[3]: *** [CMakeFiles/SDL3-shared.dir/build.make:96: .../src/SDL.c.o] Error 1
```

This looks like the host-isolation failure §4.3 and the toolchain file's own
warning describe — a cross configure picking up host state. The warning is the
comment beginning `The ONLY modes are NOT, on their own, a host-isolation
guarantee` in `cmake/aarch64-toolchain.cmake` (lines 116-121 as of
2026-09-28; locate it by that text, not by line number, since the file changes).
**It is not a host-isolation failure**, and the reason is worth recording so
nobody chases it:

- `dbus-1.pc` does not exist on this host (`find / -name 'dbus-1.pc'` → nothing),
  and neither does `dbus/dbus.h`. The *native* build of the same source
  succeeds, because with no `pkg-config` at all `PKG_CONFIG_FOUND` is false and
  SDL skips the whole `if(PKG_CONFIG_FOUND)` block, leaving `HAVE_DBUS_DBUS_H`
  unset.
- In run (b) my toolchain file's `find_program` found
  `/tmp/opencode/fakebin/aarch64-linux-gnu-pkg-config` — a 17-byte stub whose
  entire body is `exit 0`. That made `PKG_CONFIG_FOUND` true, and
  `pkg_search_module(DBUS dbus-1 dbus)` inherited the stub's success:

```
DBUS_FOUND:INTERNAL=1
DBUS_CFLAGS:INTERNAL=
DBUS_INCLUDE_DIRS:INTERNAL=
```

so SDL set `HAVE_DBUS_DBUS_H TRUE` and included a header it had no include path
for.

A real cross toolchain with a real `aarch64-linux-gnu-pkg-config` and no dbus
development files in its sysroot would report `DBUS_FOUND` false and skip the
include. The lesson for anyone reproducing this: **a stubbed `pkg-config` that
always succeeds manufactures `*_FOUND` results**, and they then fail much later
and somewhere unrelated.

### 6.4.2 Real aarch64 cross build — **pass**

§6.4 ran the toolchain file with an **x86_64 `gcc` symlink** and a stand-in arch
flag, because no cross compiler was installed. That proved the CMake plumbing and
nothing about the target. The toolchain of §2.5 is now installed, so the same
procedure was re-run with **no fake `PATH`, no stub `pkg-config`, and no
`ROADOS_ARCH_FLAGS` override** — the real `aarch64-linux-gnu-gcc` and the file's
own default `-march=armv8-a`.

```sh
env -u ROADOS_SYSROOT -u ROADOS_ARCH_FLAGS \
  cmake -S /tmp/opencode/crates/sdl3-src-3.4.16/SDL -B /tmp/opencode/sdlcross-real \
    -DCMAKE_TOOLCHAIN_FILE=.../cmake/aarch64-toolchain.cmake \
    -DCMAKE_BUILD_TYPE=Release \
    -DSDL_EXAMPLES=OFF -DSDL_TESTS=OFF -DSDL_REVISION=SDL-3.4.16 \
    -DSDL_AUDIO=OFF -DSDL_RENDER=OFF -DSDL_CAMERA=OFF \
    -DSDL_UNIX_CONSOLE_BUILD=ON
cmake --build /tmp/opencode/sdlcross-real -j"$(nproc)"
```

`-DSDL_UNIX_CONSOLE_BUILD=ON` stands in for the manifest feature
`build-from-source-unix-console` of §5.4, which did not exist when this was
written and now does (task 02, 2026-09-28; §4.2 runs the real cargo build with
it). Without it, configure fails at `cmake/macros.cmake:415`; see §7.6.

**Configure: exit 0** (26.9 s). **Build: exit 0** (33.4 s, `user 2m50s`),
`Linking C shared library libSDL3.so`.

The compiler is genuinely the cross one, and the flag the file chose is accepted
by it — the pair that was impossible before:

```
$ grep -E '^(CMAKE_C_COMPILER:|CMAKE_C_FLAGS:)' CMakeCache.txt
CMAKE_C_FLAGS:STRING= -march=armv8-a
$ grep -m1 'CMAKE_C_COMPILER ' CMakeFiles/*/CMakeCCompiler.cmake
set(CMAKE_C_COMPILER "/usr/bin/aarch64-linux-gnu-gcc")
$ aarch64-linux-gnu-gcc -dumpmachine
aarch64-linux-gnu
```

**The artifact is AArch64.** This is the check that catches a "cross" build that
quietly produced host objects, and nothing else in the pipeline catches it:

```
$ file libSDL3.so.0
libSDL3.so.0: ELF 64-bit LSB shared object, ARM aarch64, ...
$ aarch64-linux-gnu-readelf -h libSDL3.so.0 | grep -E 'Class|Machine'
  Class:   ELF64
  Machine: AArch64
$ find . -name '*.o' | wc -l
251
$ file $(find . -name '*.o' | head -3)
  ...: ELF 64-bit LSB relocatable, ARM aarch64, version 1 (SYSV), not stripped
```

Every one of the 251 objects is aarch64. No host object leaked in.

**The KMSDRM failure is now proven in the artifact, not just in a summary line.**
The cache and the generated header disagree, in the most consequential way
possible:

```
$ grep -E '^SDL_(X11|WAYLAND|KMSDRM|UNIX_CONSOLE_BUILD):' CMakeCache.txt
SDL_X11:BOOL=OFF
SDL_WAYLAND:BOOL=OFF
SDL_KMSDRM:BOOL=ON
SDL_UNIX_CONSOLE_BUILD:UNINITIALIZED=ON

$ grep SDL_VIDEO_DRIVER_KMSDRM include-config-release/build_config/SDL_build_config.h
/* #undef SDL_VIDEO_DRIVER_KMSDRM */
```

`SDL_KMSDRM` **is** on — `dep_option(SDL_KMSDRM … ${UNIX_SYS} "SDL_VIDEO" OFF)`
at `CMakeLists.txt:375` defaults it ON for Unix, so no forcing is needed to reach
it. What fails is the *backend test*, because there is no target `pkg-config`
and therefore no target `libdrm`/`gbm`. The build exits **0** and produces a
library with no scanout driver at all:

```
--   Video drivers: dummy offscreen
```

That is §6.7's finding, now confirmed on a real target artifact. It is also why
the forcing in `doc/platform/TASK_CROSSPLATFORM_02.md` is probably unnecessary —
see that task's requirement 2, which asks for this to be checked before anything
is changed.

**The target links no runtime platform libraries, and the corrected count is
zero.** `SDL_DEPS_SHARED=ON` is the default, so the interesting question is which
sonames SDL recorded for `dlopen`. The answer here is none:

```
$ grep -cE '^#define SDL_.*DYNAMIC' include-config-release/build_config/SDL_build_config.h
0
$ for s in libX11.so.6 libwayland-client.so.0 libdrm.so.2 libgbm.so.1; do
      strings libSDL3.so.0 | grep -qx "$s" && echo "$s PRESENT" || echo "$s absent"; done
  libX11.so.6              absent
  libwayland-client.so.0   absent
  libdrm.so.2              absent
  libgbm.so.1              absent
```

A counting trap worth recording, because it produces the opposite answer: a
plain `grep -oE 'SDL_[A-Z0-9_]*DYNAMIC[A-Z0-9_]*'` reports **26** matches, which
looks like 26 sonames to ship. All 26 are `#undef` lines — the macro *names*
appear in the comment, and the *values* are not defined. Count `^#define`, or
check the binary's strings, or the two disagree. The `#undef` block is the
mechanism, not a list.

**The `dbus` failure of §6.4.1 is gone, as predicted.** With no fake `pkg-config`
on `PATH`, `PKG_CONFIG_FOUND` is false, the `if(PKG_CONFIG_FOUND)` block is
skipped, and `HAVE_DBUS_DBUS_H` is never set. That confirms the §6.4.1 diagnosis
on the real toolchain rather than leaving it a hypothesis.

**What this closes, and what it does not.** It closes the last task-01
cross-compilation waiver: the build runs, the compiler is cross, the flag is
valid for aarch64, and the artifact is `AArch64`. The one thing it did not
close — "a `cargo build --target aarch64-unknown-linux-gnu` succeeds" — was
closed the same day once the manifest existed (task 02); see §4.2. What it
still does **not** give the target is a video driver (§6.7,
`doc/platform/TASK_CROSSPLATFORM_02.md`). No
runtime, no device, and no sysroot — this build needed none, and §6.4 explains
why that is expected.

### 6.5 Native SDL 3.4.16 configure — **pass**

The §3 command completes with exit status 0:

```
-- Configuring done (27.8s)
-- Generating done (0.1s)
-- Build files have been written to: /tmp/opencode/sdlverify
```

EGL and GLES 2+ detected, desktop GL not detected, and in the generated
`SDL_build_config.h`:

```
#define SDL_AUDIO_DISABLED 1
#define SDL_RENDER_DISABLED 1
#define SDL_CAMERA_DISABLED 1
#define SDL_VIDEO_OPENGL_EGL 1
#define SDL_VIDEO_OPENGL_ES2 1
```

No `SDL_VIDEO_OPENGL` and no `SDL_FILESYSTEM_DISABLED` — the first because no
desktop GL is installed, the second because there is no such option (§5.2).

### 6.6 `cargo build` of the pinned dependency set — **pass**

Run on 2026-09-28 after the operator's two fixes (`rustup update stable`, and
`libxcursor-dev libxrandr-dev libxss-dev`). This is the first time the pinned set
has actually built here, so the full result is recorded.

```sh
# /tmp/opencode/nativebuild/Cargo.toml
[dependencies]
sdl3  = { version = "0.20", features = ["build-from-source"] }
glow = "0.18"
```

```
   Compiling sdl3-src v3.4.16
   Compiling sdl3-sys v0.7.1+SDL-3.4.16
   Compiling sdl3 v0.20.0
   Compiling roados-native-probe v0.0.0 (/tmp/opencode/nativebuild)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.70s
```

Exit 0. SDL's own configure phase reports what it ended up with:

```
--   Video drivers: dummy offscreen x11(dynamic)
--   X11 libraries: xcursor xdbe xfixes xinput2 xrandr xscrnsaver xshape xsync xtest
--   Render drivers: gpu ogl_es2 vulkan
--   GPU drivers: vulkan
--   Audio drivers: disk dummy
--   Joystick drivers: hidapi linux virtual
--   Camera drivers: dummy v4l2
```

Three things to read off that list. `x11(dynamic)` confirms the driver is
compiled in but reached by `dlopen`. `Audio drivers: disk dummy` is the
pkg-config casualty of §6.7. And **there is no `kmsdrm`**, which is the
subject of the next section.

**`ldd`: installing the X11 development packages did not create an X11
dependency.**

```
$ ldd target/debug/roados-native-probe | sort
	/lib64/ld-linux-x86-64.so.2 (0x0000704d3c1a2000)
	libc.so.6 => /usr/lib/x86_64-linux-gnu/libc.so.6 (0x0000704d3be00000)
	libgcc_s.so.1 => /usr/lib/x86_64-linux-gnu/libgcc_s.so.1 (0x0000704d3c0ec000)
	linux-vdso.so.1 (0x0000704d3c1a0000)
```

Four dependencies. `grep -cE 'X11|GL|EGL'` over that output returns `0`. The
generated `SDL_build_config.h` shows where the X11 libraries went — recorded as
sonames for runtime `dlopen()`, which is what `SDL_DEPS_SHARED=ON` means:

```c
#define SDL_VIDEO_DRIVER_X11 1
#define SDL_VIDEO_DRIVER_X11_DYNAMIC "libX11.so.6"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XCURSOR "libXcursor.so.1"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XEXT "libXext.so.6"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XFIXES "libXfixes.so.3"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XINPUT2 "libXi.so.6"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XRANDR "libXrandr.so.2"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XSS "libXss.so.1"
#define SDL_VIDEO_DRIVER_X11_DYNAMIC_XTEST "libXtst.so.6"
#define SDL_VIDEO_OPENGL_EGL 1
#define SDL_VIDEO_OPENGL_ES2 1
/* #undef SDL_VIDEO_OPENGL */
/* #undef SDL_VIDEO_OPENGL_GLX */
```

This is the direct answer to the question §2.3's package list raises: the X11
dev packages are a *configure-time* requirement of the native build and cost
nothing at link time. It also confirms the `SDL_DEPS_SHARED=ON` mechanism that
the toolchain file relies on to keep host X11 headers out of a target link line
— and that reliance is why a sysroot stops being optional if anyone ever sets
`SDL_DEPS_SHARED=OFF`.

`SDL_VIDEO_OPENGL_GLX` is undefined, matching §2.4: SDL took the EGL path and
rejected GLX.

### 6.7 The build succeeds, and `SDL_video.c` has no scanout driver — **the important finding**

`SDL_VIDEO_DRIVER_KMSDRM` is undefined in the native build above, along with
`SDL_VIDEO_DRIVER_KMSDRM_DYNAMIC` and `SDL_VIDEO_DRIVER_KMSDRM_DYNAMIC_GBM`. The
build **succeeded**. There is no error and no warning. On a head unit, the
resulting SDL finds no video driver, and `SDL_Init` fails at runtime with
nothing in the build log to explain why.

The cause is `CheckKMSDRM` (`cmake/sdlchecks.cmake:1363-1374`):

```cmake
macro(CheckKMSDRM)
  if(SDL_KMSDRM)
    set(PKG_CONFIG_LIBDRM_SPEC libdrm)
    set(PKG_CONFIG_GBM_SPEC gbm)
    set(PC_LIBDRM_FOUND FALSE)
    set(PC_GBM_FOUND FALSE)
    if(PKG_CONFIG_FOUND)                                    # <- the gate
      pkg_check_modules(PC_LIBDRM IMPORTED_TARGET ${PKG_CONFIG_LIBDRM_SPEC})
      pkg_check_modules(PC_GBM   IMPORTED_TARGET ${PKG_CONFIG_GBM_SPEC})
    endif()
    if(PC_LIBDRM_FOUND AND PC_GBM_FOUND AND HAVE_OPENGL_EGL)
      set(HAVE_KMSDRM TRUE)
      set(SDL_VIDEO_DRIVER_KMSDRM 1)
      ...
    endif()
  endif()
endmacro()
```

There is **no `else()` branch**. The macro falls off the end of the `if` and the
configure continues. The cause is `PKG_CONFIG_FOUND` being false, and this host
has no `pkg-config`:

```
-- Could NOT find PkgConfig (missing: PKG_CONFIG_EXECUTABLE)
```

**The silence is not total, and the one trace is easy to miss.** SDL's backends
summary does list the subsystem as unsatisfied, among about two hundred lines:

```
--   SDL_KMSDRM                  (Wanted: ON): OFF
--   SDL_KMSDRM_SHARED           (Wanted: ON): OFF
```

So it is findable — but it is a line in a long list, not a diagnostic, and
nothing anywhere says "you have no scanout driver". The build is not degraded,
it is missing a subsystem, and it looks exactly like success.

**What that line actually reports, because getting this backwards is easy.** It
is worth being exact, since the obvious reading is wrong. `message_tested_option`
(`cmake/macros.cmake:48-60`) is the whole of the mechanism:

```cmake
macro(message_tested_option _NAME)
  set(_REQVALUE ${${_NAME}})                          # the option value
  ...
  message(STATUS "  ${_NAME}${_PAD}(Wanted: ${_REQVALUE}): ${HAVE_${_STRIPPEDNAME}}")
endmacro()
```

Two different variables: `Wanted:` is the **option** (`SDL_KMSDRM`, which is
`ON`), and the value after the colon is `HAVE_KMSDRM`, the **detection result**
(`OFF`). So `SDL_KMSDRM (Wanted: ON): OFF` does not mean the option was turned
off; it means the option was on and the driver was not found. Measured, on the
native build above:

```
$ grep -E '^SDL_KMSDRM:' <build>/CMakeCache.txt
SDL_KMSDRM:BOOL=ON
```

The option is on, the cache agrees, and the driver is still missing. This is the
same distinction the reviewer drew between option state and backend
availability, applied to the one line of SDL's own output that is most likely to
be misread as evidence of the former.

**This is not KMSDRM-specific.** The same `if(PKG_CONFIG_FOUND)` gate wraps **14**
`pkg_check_modules` calls in `cmake/sdlchecks.cmake`, across 11 macros (§2.2).
That is the number this paragraph used to state as 16, which was wrong; with
the BSD-only `libinotify` call in `CMakeLists.txt:1980` the tree-wide total is
15 calls under 12 guards.

The native configure emitted **31** `(Wanted: ON): OFF` lines — not 32, as an
earlier revision of this document claimed. But the count is the least interesting
part, because the 31 are not 31 disabled options. Checked against the cache:

| | count |
|---|---|
| `(Wanted: ON): OFF` lines | 31 |
| …of those, `BOOL=ON` in `CMakeCache.txt` | 29 |
| …with no cache entry at all | 2 (`SDL_EXAMPLES_LINK_SHARED`, `SDL_TESTS_LINK_SHARED`) |
| …actually `BOOL=OFF` | **0** |

Sorted by cause, all 31:

- **18 lines from the `pkg-config` gate** — `SDL_KMSDRM`, `SDL_WAYLAND`,
  `SDL_WAYLAND_LIBDECOR` (each with a `_SHARED` twin), and `SDL_PIPEWIRE`,
  `SDL_PULSEAUDIO`, `SDL_JACK`, `SDL_SNDIO`, `SDL_FRIBIDI`, `SDL_LIBTHAI`
  (each with a `_SHARED` twin). These are the ones a target `pkg-config` fixes.
- **8 lines from other `find_package` mechanisms, absent for missing dev
  packages, not for `pkg-config`** — `SDL_ALSA` and `SDL_ALSA_SHARED`
  (`find_package(ALSA MODULE)`, no `libasound2-dev`), `SDL_HIDAPI_LIBUSB` and
  its `_SHARED` twin (`find_package(LibUSB)`, no `libusb-1.0-0-dev`),
  `SDL_DBUS`, `SDL_IBUS`, `SDL_LIBUDEV`, `SDL_LIBURING`. The configure log
  names them and says so:
  ```
  -- Could NOT find ALSA (missing: ALSA_LIBRARY ALSA_INCLUDE_DIR)
  -- Could NOT find LibUSB (missing: LibUSB_LIBRARY LibUSB_INCLUDE_PATH) (found version "LibUSB_VERSION-NOTFOUND")
  ```
- **1 line that is a decision, not a dependency** — `SDL_OPENGL`:
  `-- Could NOT find OpenGL (missing: OPENGL_opengl_LIBRARY OPENGL_glx_LIBRARY OPENGL_INCLUDE_DIR)`.
  §2.4 chooses EGL over desktop GL deliberately, so this one is off by design.
- **4 lines that are not backends at all** — `SDL_DEPS_SHARED`, `SDL_INSTALL`,
  `SDL_EXAMPLES_LINK_SHARED` and `SDL_TESTS_LINK_SHARED`, none of which
  describes a runtime subsystem. The last two have no cache entry at all.

So "31 options came out `(Wanted: ON): OFF`" was wrong twice over: the number was
31 rather than 32, and no option came out *anything* — every one was on, and 18
backends went missing underneath them. The sentence that survives is narrower
and true: *this host, with no `pkg-config`, silently lost 18 pkg-config-gated
subsystems, one of which is the only scanout path available to the head unit.*

Two of the 11 gated macros do not appear in the list at all, and the reasons
are worth carrying because they bound how far the summary can be trusted as an
inventory:

- `CheckLibUnwind` has no `dep_option` and sets no `HAVE_` variable on the
  library — it only adds `HAVE_LIBUNWIND_H` as a **PRIVATE** definition on the
  `SDL3_test` target (`sdlchecks.cmake:1418-1481`, inside
  `if(TARGET SDL3_test)`), so its outcome is invisible in the backends summary
  *and cannot appear in the library's `SDL_build_config.h` whatever it finds*.
  It is also a *test* dependency, and its gate is not exclusive anyway: it tries
  a plain compile test, then linking `unwind`, and only then `pkg-config`.
  `SDL_TEST_LIBRARY` defaults `ON` (`CMakeLists.txt:399`), so the target really
  was being built — 228 KB of archive and one three-stage probe per configure.
  **`SDL_TEST_LIBRARY=OFF` now, on both targets**; see §5.5. That is the honest
  resolution of "use it or stop checking it": this project cannot use it, and it
  can now stop being checked.
- `CheckRPI`/`CheckROCKCHIP` (`bcm_host`, `brcmegl`, `mali`) are on the embedded
  paths and stay off here regardless of `pkg-config`, so the gate is not the
  operative constraint for them.

**Contrast with §7.4.** A missing X11 extension package is a `FATAL_ERROR` at
configure time; a missing `pkg-config` is a successful build. The first is
annoying, the second is dangerous, because the artefact is wrong rather than
absent and nothing downstream will catch it until the device boots.

**Consequences, stated as requirements rather than advice:**

- For **any** aarch64 build: `aarch64-linux-gnu-pkg-config` is required, and it
  is not optional in the same way `pkg-config` is on the host.
- `libdrm` and `libgbm` for aarch64 are required, not merely wanted, for the
  scanout path.
- With `ROADOS_SYSROOT` unset, a host `pkg-config` satisfies the `if` and then
  resolves the **host's** `libdrm`, recording its path — which is exactly what
  §6.4.1 observed with the stub. A sysroot is not optional for this target.
- The check to run on any target build, which catches the whole class:

```sh
grep -E 'SDL_VIDEO_DRIVER_KMSDRM|SDL_AUDIO_DRIVER' <build-dir>/SDL_build_config.h
```

Not reproduced here: an aarch64 build with these in place. That is waiver 1.

### 6.8 OpenGL ES on this machine — **observed, not a gate**

```sh
glxinfo -B
```

```
OpenGL core profile version string: 4.6 (Core Profile) Mesa 26.0.8-1ubuntu0.3
Max GLES[23] profile version: 3.2
OpenGL ES profile version string: OpenGL ES 3.2 Mesa 26.0.8-1ubuntu0.3
OpenGL ES profile shading language version string: OpenGL ES GLSL ES 3.20
```

`eglinfo` reports EGL 1.5 with `EGL_KHR_create_context` and
`EGL_EXT_platform_device`/`_wayland`/`_x11`/`_gbm` across GBM, Wayland, X11 and
surfaceless platforms. The EGL stack is present and usable. SDL's own configure
selected EGL and rejected GLX (§2.4).

This is a *host* result and says nothing about the head unit's GPU. It is
recorded because it removes "does this machine have GLES at all" from the list of
unknowns for task 03.

---

## 7. Troubleshooting

### 7.1 `feature 'edition2024' is required` — **resolved, kept for the diagnosis**

**Closed on 2026-09-28.** The operator ran `rustup update stable`; the toolchain
is now **1.98.1** and the pinned dependency set builds (see §6.6). The entry
stays because the diagnosis is the reusable part, and because the symptom is
misleading enough to be worth recognising.

**The symptom points at the wrong crate.** Cargo reported:

```
error: failed to parse manifest at `.../sdl3-ttf-sys-0.7.1+SDL-ttf-3.2.2/Cargo.toml`

Caused by:
  feature `edition2024` is required
```

Nothing in this project depends on `sdl3-ttf-sys`. It is an optional `ttf`
feature that nothing enables, and cargo still has to *parse* its manifest to
resolve the dependency graph at all. The real constraint is one crate up:
`sdl3-sys` 0.7.1 is `edition = "2024"` / `rust-version = "1.85"`, which
`sdl3` 0.20.0 does not impose itself (it is edition 2021 with no
`rust-version`). So when a build dies on `sdl3-ttf-sys`, the floor to check is
`sdl3-sys`'s, and the crate named in the error is a red herring.

**The toolchain was not pinned — it was just old.** That distinction decided
how much this cost:

```
$ rustup check
stable-x86_64-unknown-linux-gnu - update available: 1.80.1 (3f5fd8dd4 2024-08-06)
                                 -> 1.98.1 (48a229cea 2026-09-01)

$ cat ~/.rustup/settings.toml
default_toolchain = "stable-x86_64-unknown-linux-gnu"
profile = "default"
version = "12"

$ ls -la /snap/bin/rustc
lrwxrwxrwx 1 root root 12 Jun 18 14:29 /snap/bin/rustc -> rustup.rustc
```

`stable` was already active and already the default, the `/snap/bin` entries were
rustup shims rather than a separate compiler, and the toolchain lived in
user-owned `~/.rustup/toolchains/`. One command, nothing installed, no pin
changed:

```sh
rustup update stable
cargo --version    # 1.98.1
```

If the machine ever has to stay on an older toolchain for other work, the
fallback is **not** an older `sdl3`: `0.20` resolves to `sdl3` 0.20.0 and
nothing earlier, because `^0.20` does not admit 0.19. The pin in
`PRIMITIVES_ARCHITECTURE.md` would have to change, which is a far larger
decision than updating a toolchain.

### 7.2 `aarch64-linux-gnu-gcc is not a full path and was not found in the PATH`

The cross toolchain is not installed. §2.5.

### 7.3 `no such file or directory` at the archive step, with `CMAKE_C_COMPILER_AR-NOTFOUND`

Check the cache first:

```sh
grep CMAKE_C_COMPILER_AR <build-dir>/CMakeCache.txt
```

If it reads `CMAKE_C_COMPILER_AR-NOTFOUND`, the archiver is missing and that is
the whole story. CMake looks for **`aarch64-linux-gnu-gcc-ar`**, in the
compiler's own directory, via its own `find_program` with
`NO_CMAKE_PATH NO_CMAKE_ENVIRONMENT_PATH`. Install `binutils-aarch64-linux-gnu`,
which provides it next to the compiler. §2.5 has the quoted source for both the
search and the two different archive rules.

Three dead ends to avoid:

- **`gcc -print-prog-name=ar` is the wrong probe.** It answers `ar`, and plain
  `ar` is not the file CMake wants for the IPO rule — `gcc-ar` is a wrapper that
  adds the LTO `--plugin` option. Reading this symptom through that command is
  what makes it look unfixable.
- **Setting `CMAKE_AR` does not help the IPO rule.** It is a different variable:
  the plain archive rule uses `CMAKE_AR`, the IPO rule uses
  `CMAKE_<LANG>_COMPILER_AR`. This toolchain file sets the former on purpose.
- **Setting `CMAKE_<LANG>_COMPILER_AR` works but is a last resort.** It does
  stick — `find_program` skips its search when the variable is already set — but
  it becomes a normal variable that never reaches `CMakeCache.txt`, so the
  `grep` at the top of this section stops finding it, and it hardcodes a path
  instead of using CMake's version-aware search. §2.5 has the measurements.
- **It may not bite at all.** `CMAKE_INTERPROCEDURAL_OPTIMIZATION` is off by
  default, so the rule that needs `gcc-ar` is not selected unless IPO is enabled
  (LTO being the usual reason). If the failure appears only after someone turned
  IPO on, this is why.

### 7.4 `Couldn't find dependency package for XCURSOR` (or XRANDR, XSCRNSAVER)

Missing X11 extension development packages. Either install them or add
`-DSDL_X11_X<NAME>=OFF` — §2.3, §3. In a cargo build the flag is not reachable
from `Cargo.toml`; see §7.7.

### 7.5 The build succeeds and there is no video driver on the device

**No error, no warning, exit 0.** This is the hardest failure mode in the whole
pipeline, because every signal says it worked.

```c
/* #undef SDL_VIDEO_DRIVER_KMSDRM */   /* in SDL_build_config.h */
```

`SDL_Init(SDL_INIT_VIDEO)` on the head unit then fails to find a driver, and the
only clue is that error at runtime.

Cause: `CheckKMSDRM` reaches libdrm and gbm **only** through `pkg-config`
(`cmake/sdlchecks.cmake:1363-1374`), and has no `else()` branch. Missing
`pkg-config`, or missing `libdrm`/`gbm`, means the driver is dropped silently.
Full measurement in §6.7.

Diagnose it by asking for the drivers rather than reading the log:

```sh
grep -E 'SDL_VIDEO_DRIVER_|SDL_AUDIO_DRIVER_' <build-dir>/SDL_build_config.h
```

Compare against what you asked for. Or read SDL's own backends summary, which
does record it, in among everything else:

```sh
grep '(Wanted: ON): OFF' <build-dir>/../build/CMakeFiles/CMakeOutput.log
# or just: cargo build -vv 2>&1 | grep -A400 'Enabled video backends'
```

Fix, for the target:

```sh
sudo apt-get install aarch64-linux-gnu-pkg-config   # or the sysroot equivalent
# plus libdrm and libgbm for aarch64, in the sysroot
```

**Contrast with §7.4, which is the same class of mistake with the opposite
outcome.** A missing X11 extension package aborts the configure with
`FATAL_ERROR: Couldn't find dependency package for XCURSOR` — loud, immediate,
and impossible to miss. A missing `pkg-config` produces a *successful* build
that is missing a subsystem. When triaging, reach for this entry whenever the
symptom is "it builds but does not work on the device", and treat every
`(Wanted: ON): OFF` line in the backends summary as a defect to be explained
rather than noise to be skimmed.

### 7.6 `SDL could not find X11 or Wayland development libraries on your system`

`cmake/macros.cmake:413-427`. On a cross build with no sysroot this is expected:
there is no X11 or Wayland in scope. Provide a sysroot, or set
`SDL_UNIX_CONSOLE_BUILD=ON` — §4.5.

### 7.7 An SDL option that `sdl3-sys` does not expose

There is no `-D` escape hatch: no `CMAKE_ARGS` in `cmake` 0.1.58, and CMake's
environment-variable list has no `CMAKE_PROJECT_INCLUDE`. The options you can
reach are exactly the `cmake_vars!` list in `sdl3-sys`'s `build.rs`. The
remaining escape hatch is `CMAKE_TOOLCHAIN_FILE` itself, which is CMake code read
before `project()`, so it can supply a default for any option. It can also
overwrite one, but see below before you write that:

```cmake
# appended to cmake/aarch64-toolchain.cmake, or to a copy of it
if(NOT DEFINED SDL_X11)              # supplies a default; -D still wins
  set(SDL_X11 OFF CACHE BOOL "")
endif()
if(NOT DEFINED SDL_UNIX_CONSOLE_BUILD)
  set(SDL_UNIX_CONSOLE_BUILD ON CACHE BOOL "")
endif()
```

The obvious form of this — `set(SDL_X11 OFF CACHE BOOL "" FORCE)` — is **not**
what the file does, and an earlier revision of this section printed it as the
recipe while a later paragraph correctly said `FORCE` was wrong. The `FORCE` form
is wrong because it overwrites a `-DSDL_X11=ON` and takes away the operator's
ability to opt out for a one-off build. Use `if(NOT DEFINED ...)`, for the reason
given at the end of this section. If you genuinely must win over a `-D`, that is
a different intent and belongs in a local uncommitted edit, not in a recipe
copied out of a document.

Two things to know before using it: it is **target-asymmetric** — there is no
toolchain file on the native build, so the native SDL would not get the same
option — and it is invisible to anyone reading `Cargo.toml`. Prefer the Cargo
features in §5.1; use the toolchain file only for options that have no feature
and accept the asymmetry deliberately.

**Three options are in exactly this position**, and the distinction between them
is the useful part. From `sdl3-sys`'s `cmake_vars!` list (`build.rs:30-48`):

| Option | In `cmake_vars!`? | Consequence |
|---|---|---|
| `SDL_UNIX_CONSOLE_BUILD` | **yes** | reach it from `Cargo.toml` — `build-from-source-unix-console` |
| `SDL_X11`, `SDL_WAYLAND` | no | toolchain file only |
| `SDL_KMSDRM` | no, **but already `ON`** | needs no channel — see below |

**`SDL_KMSDRM` is the one row that is better than the table's pattern suggests,
and §6.4.2 is the evidence.** `dep_option(SDL_KMSDRM … ${UNIX_SYS} "SDL_VIDEO"
OFF)` at `CMakeLists.txt:375` defaults it `ON` for any Unix target, so the cache
holds `SDL_KMSDRM:BOOL=ON` without the toolchain file touching it. The reason it
still does not work is not an unreachable option — it is the *backend test*
failing for want of a target `pkg-config` and target `libdrm`/`gbm`, which is
§6.7. So the fix is the sysroot (`doc/platform/TASK_CROSSPLATFORM_01.md`), not a
forcing. `TASK_CROSSPLATFORM_02.md`'s requirement 2 says to check this before
changing anything, and this is that check's result.

`cmake_vars!` is the whole reachable set. Everything else needs a channel that
does not exist — there is no `CMAKE_ARGS` in `cmake` 0.1.58 and no
`CMAKE_PROJECT_INCLUDE` in CMake's environment-variable list.

**`cmake/aarch64-toolchain.cmake` now uses this for `SDL_X11` and
`SDL_WAYLAND`**, with the reasoning and the measurements in §5.4. The override
discipline it uses is worth copying, because the obvious `FORCE` is wrong:

```cmake
if(NOT DEFINED SDL_X11)                 # -D wins
  set(SDL_X11 OFF CACHE BOOL "")
endif()
```

`FORCE` would overwrite a `-DSDL_X11=ON` and take away the operator's ability to
opt out for a one-off build. `if(NOT DEFINED ...)` is correct because a toolchain
file is read *before* `project()`, so `-DSDL_X11=ON` is already a cache entry
when this runs, while the no-`-D` case leaves the variable undefined and lets
the block supply the default. It is also idempotent across the repeated reads
that `try_compile` causes.

Re-verified against the committed file on 2026-09-28 with the §6.1 probe, four
branches plus the precedence rule:

| branch | `SDL_X11` | `SDL_WAYLAND` |
|---|---|---|
| no `-D`, no environment | `OFF` | `OFF` |
| `-DSDL_X11=ON` | **`ON`** | `OFF` |
| `ROADOS_SDL_X11=ON` | **`ON`** | `OFF` |
| `ROADOS_SDL_X11=OFF` *and* `-DSDL_X11=ON` | **`ON`** | `OFF` |
| `ROADOS_SYSROOT=/opt/sysroot ROADOS_ARCH_FLAGS="-march=armv8.2-a+crc"` | `OFF` | `OFF` |

Row 2 is the row `FORCE` would fail: the block must leave a `-D` alone. Row 4
pins the order — `-D` beats the environment variable, which beats the default.

### 7.8 The build works and produces something that will not run on the target

`file target/aarch64-unknown-linux-gnu/debug/<binary>` should say `AArch64`. If it
says `x86-64`, the compiler was the host's — see §4.5, first bullet. Nothing
else in the pipeline catches this.

### 7.9 `sdl3-sys` panics: `cmake dir not found in ...`

`find_and_output_cmake_dir_metadata` looks for `SDL3Config.cmake` in
`lib/cmake/SDL3`, `lib64/cmake/SDL3` or `cmake` under the build output. It means
SDL configured but did not install its CMake package. Usually a configure that
"finished" but was actually reconfigured, or a stale `OUT_DIR`. `cargo clean -p
sdl3-sys` and rebuild.

### 7.10 Reading the vendored source

`~/.cargo/registry/src/*/sdl3-src-3.4.16/SDL/` after a build, or download the
crate directly:

```sh
curl -sSLo sdl3-src.crate \
  https://static.crates.io/crates/sdl3-src/sdl3-src-3.4.16.crate
tar xzf sdl3-src.crate
```

---

## 8. Open items for the operator

Ordered by what blocks the next task.

1. **`ROADOS_SYSROOT` is untested against a real target.** The branch is
   exercised in §6.1's script-mode probe and in real `project()` configures on
   this host, but only ever with a hand-made fake sysroot and the host's x86_64
   compiler standing in for aarch64. What that did establish is that the three
   settings the branch is responsible for all land:

```
-- SYSROOT=[/tmp/opencode/r1/fakesys]
-- FIND_ROOT=[/tmp/opencode/r1/fakesys]
-- PKG_CONFIG_SYSROOT_DIR=[/tmp/opencode/r1/fakesys]
-- Configuring done (0.2s)
```

   `PKG_CONFIG_LIBDIR` is built from `ROADOS_TARGET_TRIPLE` rather than from
   anything the compiler reports, so the pkg-config search paths are already
   target-correct. What the fake compiler cannot tell us is whether the *library*
   search paths resolve against a real sysroot, since it derives a different
   multiarch triplet than the target.

   One thing worth recording because it is a property of CMake rather than of
   this file: `CMAKE_LIBRARY_ARCHITECTURE` **cannot** be set from a toolchain
   file. The file sets it, and the setting is correct at that moment, but
   `project()` derives the variable from the compiler it probed and overwrites
   it. Printing it on both sides of `project()` on 2026-09-28:

```
-- IN-FILE-AFTER-SET: [aarch64-linux-gnu]     <- immediately after this file's set()
-- AT-PROJECT-BODY:   [x86_64-linux-gnu]       <- after project() ran
```

   So the `if(NOT CMAKE_LIBRARY_ARCHITECTURE)` block is documentation of intent
   and takes effect only in script mode; in a real configure the value is
   whatever the real `aarch64-linux-gnu-gcc` reports, which is the right one.
   No fix is needed, and none of the alternatives (forcing a cache entry, or
   injecting code via `CMAKE_PROJECT_INCLUDE`) were adopted: they cannot beat
   `project()` either, and the `-DCMAKE_LIBRARY_ARCHITECTURE=` escape hatch is
   unnecessary once the compiler is real. Once a real toolchain and sysroot
   exist, the check is one line:

```sh
grep CMAKE_LIBRARY_ARCHITECTURE <build-dir>/CMakeCache.txt
```
2. **How SDL subsystems get configured.** `sdl3` 0.20.0 exposes no subsystem
   features, so the configuration `PRIMITIVES_ARCHITECTURE.md` asks for needs
   either a direct `sdl3-sys` dependency carrying the features, or options forced
   from the toolchain file. §5.1. **Decided** for now: default subsystems,
   recorded as a deviation in `doc/ui/IMPLEMENTATION_STATE.md`, to be revisited
   before `roados_ui` rather than before task 02 — which is why task 02 was not
   blocked on it.
3. **`pkg-config`, `libdrm` and `libgbm` are hard requirements for any aarch64
   build.** §6.7, §2.2. This is the highest-value item on the list because the
   failure it causes is silent: the build succeeds, and the head unit ends up
   with an SDL that has no scanout driver. On the target that means
   `aarch64-linux-gnu-pkg-config` plus `libdrm` and `libgbm` for aarch64, inside
   the sysroot — and with `ROADOS_SYSROOT` unset a host `pkg-config` will satisfy
   the check and then record the *host's* `libdrm`, so item 1 gates this one.
   Verified on the host that the absence really is silent; not verified that
   installing them fixes an aarch64 build, because the aarch64 builds so far run
   with no sysroot and no target `pkg-config` (§6.4.2).
4. **aarch64 cross toolchain — installed and in use.** §2.5,
   `rustup target add aarch64-unknown-linux-gnu` included, verified
   2026-09-28. The builds in §6.4.2 and §4.2 ran on it. `binutils-aarch64-linux-gnu`
   is present as well as the compiler, for the `gcc-ar` reason in §2.5. What
   remains on the target side is the runtime half: item 1.
5. **Filesystem subsystem.** Cannot be disabled with SDL 3.4.16 +
   `sdl3-sys` 0.7.1. §5.2. `PRIMITIVES_ARCHITECTURE.md` and reality disagree.
6. **Haptics, HID and audio.** §5.1. Whether `sdl-hidapi` stays on (USB gamepads
   and steering wheels), and whether the head unit is expected to make sound at
   all. The native build has `Audio drivers: disk dummy` — no real backend —
   which is item 3's doing, not a decision.
7. **Target video drivers — settled, with one part deferred.** §5.4. X11 and
   Wayland are **off** for the target and `SDL_UNIX_CONSOLE_BUILD` is **on**;
   `cmake/aarch64-toolchain.cmake` forces the first two and the manifest **does**
   carry the third, since task 02. KMSDRM is **required** but not yet configured,
   because it needs libdrm and gbm `.pc` files for the target and the target image
   is undecided. What remains open is only the sysroot story — which is item 1.
8. **A real `cargo build --target aarch64-unknown-linux-gnu` has been run on
   this machine**, and it succeeds, with no sysroot (§4.2 records the command
   and `readelf -h`). The §6.4 configures below it remain *not* aarch64 builds
   in the narrower sense that they configure SDL's CMake directly instead of
   going through Cargo — but they no longer stand as the only cross evidence
   there is. What they establish is that the committed toolchain file's variables
   and its default `-march` reach a real `project()` configure, and the new
   `SDL_X11` / `SDL_WAYLAND` forcing has been verified on all four of its
   branches in script mode and in a real configure with a stand-in compiler
   (§5.4, §7.7).
9. **The `pkg-config` harness artefact.** §6.4.1 and §5.4. Any cross
   verification done with a stub `pkg-config` will report host libraries as
   found. When items 1 and 4 are settled, re-run the cross configure with a real
   `aarch64-linux-gnu-pkg-config` before trusting anything the previous runs
   said about optional platform libraries. This is also the only way to check
   item 3's fix.
