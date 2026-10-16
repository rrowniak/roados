# TASK_CROSSPLATFORM_01: Target Image and Sysroot

## Goal

Decide and build the aarch64 target image: the root filesystem, its headers, and
its `.pc` files, so that a real cross build has something target-shaped to
compile and link against.

## Context

This is the decision deferred by the operator on 2026-09-28 while task 01 was
implemented. `doc/ui/CROSSBUILD.md` §7 records it as open.

The cross build **already succeeds without a sysroot** — §2 configures and
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

1. **Choose the strategy and record the reasoning.** Three candidates:
   - Debian multiarch: `dpkg --add-architecture arm64`, target `-dev` packages,
     `aarch64-linux-gnu-pkg-config` pointed at them via `PKG_CONFIG_LIBDIR`.
     Least work; mixes arm64 packages into the host.
   - Buildroot or Yocto: a real rootfs with target headers and `.pc` files. More
     work up front; this is the production path for a head unit.
   - Raspberry Pi OS 64-bit image (added 2026-10-16): the Pi-native route for
     the decided CM4 target.
   Write the decision and its consequences into `doc/ui/CROSSBUILD.md` and this
   sequence's state file, `doc/platform/IMPLEMENTATION_STATE.md`. **The operator
   decides; the developer implements.**

   **Decision (operator, 2026-10-16): Pi OS Lite 64-bit, pinned release,
   bench-only and revisable.** Lite ships no X/Wayland — the KMSDRM-only target
   needs nothing removed, and nothing desktop-shaped gets installed "just in
   case". The release is one variable (image URL + deb snapshot); changing it
   later is the flexibility mechanism, not vagueness now.
   - Provisioning is SSH-only: an idempotent provision script (runtime debs,
     `config.txt`, root-run console service, boot-time service cuts) plus an app
     deployer (binary + assets + config, restart). No offline image baking; until
     safe updates are designed, recovery is reflash stock Lite + rerun provision.
   - Boot target: under 7 s from power to first touch-responsive frame, measured
     with `systemd-analyze`; suspend/resume is the fallback if panel modeset
     blows the budget.
   - The app runs as root (DRM master without logind; bench-only) in one
     versioned directory under one systemd unit — the hook a future A/B update
     scheme hangs onto. Safe updates themselves are postponed, waived with reason.
   - The sysroot is extracted on the host from the same release's arm64 `.deb`s
     (`dpkg-deb -x`, no qemu); a version manifest checked against the device's
     `dpkg -l` guards skew.

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
   a toolchain file cannot override — see the correction in `CROSSBUILD.md` §3.

## Acceptance Criteria

- [x] The strategy decision is recorded with its reasoning, by the operator.
      (2026-10-16: Pi OS Lite 64-bit — see requirement 1.)
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
