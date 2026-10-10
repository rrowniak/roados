# TASK_CROSSPLATFORM_01: Target Image and Sysroot

## Goal

Decide and build the aarch64 target image: the root filesystem, its headers, and
its `.pc` files, so that a real cross build has something target-shaped to
compile and link against.

## Context

This is the decision deferred by the operator on 2026-09-28 while task 01 was
implemented. `doc/ui/CROSSBUILD.md` §8 records it as open.

The cross build **already succeeds without a sysroot** — §6.4 configures and
builds SDL for the target with `SDL_X11=OFF`, `SDL_WAYLAND=OFF`,
`SDL_UNIX_CONSOLE_BUILD=ON`, and no sysroot at all, because every platform
dependency degrades to `OFF` rather than failing. What a sysroot buys is not a
passing build. It is:

- `pkg-config` resolution for the target, which is the only route to
  `CheckKMSDRM` (task 02 depends on this).
- Correct multiarch library search paths, so a library is not silently resolved
  to the host's x86_64 copy — the failure mode `cmake/aarch64-toolchain.cmake`
  warns about at its `CMAKE_FIND_ROOT_PATH_MODE_*` section.
- The rootfs the head unit actually boots.

## Requirements

1. **Choose the strategy and record the reasoning.** Two candidates:
   - Debian multiarch: `dpkg --add-architecture arm64`, target `-dev` packages,
     `aarch64-linux-gnu-pkg-config` pointed at them via `PKG_CONFIG_LIBDIR`.
     Least work; mixes arm64 packages into the host.
   - Buildroot or Yocto: a real rootfs with target headers and `.pc` files. More
     work up front; this is the production path for a head unit.
   Write the decision and its consequences into `doc/ui/CROSSBUILD.md` and this
   sequence's state file, `doc/platform/IMPLEMENTATION_STATE.md`. **The operator
   decides; the developer implements.**

2. **Produce the sysroot** at a known path, with a documented way to select it
   (`ROADOS_SYSROOT` already exists in the toolchain file and is the hook).

3. **Give the target a working `pkg-config`**, and prove it: a
   `pkg_check_modules` for `libdrm` and `gbm` against the sysroot must succeed
   where it currently fails.

4. **Verify the toolchain file's sysroot branch**, which has never been exercised
   against a real sysroot. `CMAKE_SYSROOT`, `CMAKE_FIND_ROOT_PATH`,
   `PKG_CONFIG_SYSROOT_DIR` and `PKG_CONFIG_LIBDIR` are read-verified but never
   run against a real one.

5. **Record what the sysroot changes.** In particular the interaction with
   `CMAKE_LIBRARY_ARCHITECTURE`, which `project()` derives from the compiler and
   a toolchain file cannot override — see the correction in `CROSSBUILD.md` §8.

## Acceptance Criteria

- [ ] The strategy decision is recorded with its reasoning, by the operator.
- [ ] A sysroot exists at a documented path.
- [ ] `pkg-config` resolves `libdrm` and `gbm` for aarch64.
- [ ] `ROADOS_SYSROOT` set to that path, the cross configure succeeds, and
      `grep CMAKE_LIBRARY_ARCHITECTURE <build>/CMakeCache.txt` reports
      `aarch64-linux-gnu`.
- [ ] No host x86_64 library appears anywhere in the target build's link line.
- [ ] `CROSSBUILD.md` updated; its "sysroot stops being optional" warning
      replaced by what is now true.

## Out of Scope

- Forcing `SDL_KMSDRM=ON` — that is task 02.
- Booting the image — task 04.
- Any change to the Rust crates, which are unaffected by the sysroot choice.
