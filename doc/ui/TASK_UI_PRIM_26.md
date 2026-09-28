# TASK_UI_PRIM_26: Head-Unit Video Driver

## Goal

Make the head unit's scanout path actually work: `SDL_VIDEO_DRIVER_KMSDRM`
compiled in, with a real window on a real panel.

## Context

`SDL_KMSDRM` is the head unit's video path and it is currently **unreachable**.
It is not in `sdl3-sys`'s `cmake_vars!` list, so it cannot be set from
`Cargo.toml`; and `cmake/aarch64-toolchain.cmake` does not set it either. The
operator's 2026-09-28 decision settled the policy — target gets KMSDRM, no
desktop stack — but only the X11/Wayland half of it is implemented.

The failure this task fixes is **silent**, which is why it is a task rather than a
note. With no target `pkg-config`, `CheckKMSDRM` never sets `HAVE_KMSDRM`, the
generated header carries `/* #undef SDL_VIDEO_DRIVER_KMSDRM */`, the build exits
**0**, and on the device `SDL_Init` finds no video driver. Nothing warns.

`message_tested_option` prints `(Wanted: ${_REQVALUE}): ${HAVE_<name>}` — two
different variables — so the summary line reads `SDL_KMSDRM (Wanted: ON): OFF`.
The option is on; the backend test failed. Do not misread it.

## Requirements

1. **Do not add a forcing rule for `SDL_KMSDRM` yet.** The real cross build of
   `CROSSBUILD.md` §6.4.2 settles this, and it goes against the obvious fix:

   ```
   SDL_KMSDRM:BOOL=ON                          # in the cache, no forcing
   /* #undef SDL_VIDEO_DRIVER_KMSDRM */        # in SDL_build_config.h
   ```

   `dep_option(SDL_KMSDRM … ${UNIX_SYS} "SDL_VIDEO" OFF)` at `CMakeLists.txt:375`
   already defaults the option `ON` for any Unix target, so it is **not** an
   unreachable-option problem. What fails is the backend test, for want of a
   target `pkg-config` and target `libdrm`/`gbm`. A forcing rule would change
   nothing observable and would add a channel the option does not need — and it
   would be the kind of change that looks like progress while fixing nothing.
   **Verify this still holds after the sysroot lands**, then implement the
   sysroot, and only then consider whether any forcing is wanted at all.

2. **Make the backend test pass**, which means a target `pkg-config` that
   resolves `libdrm` and `gbm` — that is task 25. Re-run the §6.4.2 procedure and
   require `SDL_VIDEO_DRIVER_KMSDRM` to become a `#define` and `kmsdrm` to appear
   in the `Video drivers:` line.

3. **Understand the trap before diagnosing.** `message_tested_option` prints
   `(Wanted: ${_REQVALUE}): ${HAVE_<name>}` — two different variables. So
   `SDL_KMSDRM (Wanted: ON): OFF` means the option is **on** and the *backend
   test* failed. An agent that reads this as "the option is off" will reach for a
   forcing rule and find that it changes nothing.

4. **Verify the generated header**, not just the summary line. §6.4.2 is the
   model: the cache and `SDL_build_config.h` disagreed, and only the header is
   what the compiled library obeys.

5. **Account for every `(Wanted: ON): OFF` line in the target configure.** 31 of
   them occur on the current host. For each, record whether its absence is
   correct for a head unit or a gap. Beware the trap already documented: some are
   genuinely `if(PKG_CONFIG_FOUND)`-gated, others merely lack a `-dev` package —
   ALSA uses `find_package(ALSA)` and is off because `libasound2-dev` is absent,
   not because of `pkg-config`. Conflating the two causes a future reader to
   "confirm" the wrong story.

6. **Record the `SDL_DEPS_SHARED` tradeoff.** The default is ON, so the target
   `dlopen`s libdrm and libgbm at runtime and the *image* must ship them — see
   task 27. Turning it OFF for static linking changes all of this and makes a
   sysroot mandatory. If anyone proposes that, it needs a decision, not a
   commit.

## Acceptance Criteria

- [ ] `SDL_VIDEO_DRIVER_KMSDRM` is `#define` in the target's
      `SDL_build_config.h`.
- [ ] `kmsdrm` appears in the target configure's `Video drivers:` line.
- [ ] A record of whether any forcing rule was added and why — with the §6.4.2
      finding quoted, so the next agent does not re-derive it.
- [ ] Every remaining `(Wanted: ON): OFF` line is accounted for, each with its
      real cause.
- [ ] `CROSSBUILD.md` §5.4 and §8 no longer describe the driver half of the
      policy as unimplemented, and no longer imply it was ever verified.
- [ ] If the backend test still fails, that is reported as a hard blocker with
      the evidence — not waived, and not worked around.

## Out of Scope

- The sysroot itself — task 25.
- Which runtime libraries the image ships — task 27.
- Anything about the aarch64 CPU flags; `-march=armv8-a` is set already and the
  head unit SoC is not yet decided.
