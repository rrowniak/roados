# Cross-platform and target — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins.

**Sequence:** `doc/platform/TASK_CROSSPLATFORM_01..04.md` — *Target image and
sysroot*, *head-unit video driver*, *target runtime library audit*, *head-unit
smoke test*, moved out of `doc/ui/` on 2026-10-05 and **not to be started until
the target platform is decided**. `doc/ui/CROSSBUILD.md` stays in `doc/ui` and is
the companion document.

**Updated:** 2026-10-10.

## Current position

**Nothing in flight; nothing dispatched.** All four tasks are `deferred` — not
`pending`: unstarted *and* gated on a decision nobody has made, so a reader
cannot mistake the gate for an absence of work.

**The gate is the operator's.** `TASK_CROSSPLATFORM_01` requirement 1 is
*"Choose the strategy and record the reasoning… **The operator decides; the
developer implements.**"*, and both candidates (Debian multiarch,
Buildroot/Yocto) assume a Linux target. Everything downstream depends on it: 02
needs target `libdrm`/`gbm`, 03 the image 02's driver runs on, 04 a device.

**If the target turns out not to be aarch64 Linux, these four task files need
revision, not just relocation** — the sysroot candidates, the multiarch
discussion and the `aarch64-linux-gnu-pkg-config` requirement are all
aarch64-Linux-specific.

## Measured on this host, 2026-10-05 — the inputs requirements 3 and 4 measure against

Re-measuring these costs a session, so they are recorded. **All of it may have
changed since; check before relying on it.**

- **`pkg-config` is not installed at all** — not the host one, not the
  cross-prefixed one (`command -v` finds neither). This is why
  `CROSSBUILD.md` §8 item 3 calls a target `pkg-config` a hard requirement.
  `aarch64-linux-gnu-gcc` **is** present.
- **No rootfs tool**: no `debootstrap`, `mmdebstrap`, `buildroot` or
  `qemu-aarch64-static`. `dpkg --print-foreign-architectures` reports `i386`
  only, and `/proc/sys/fs/binfmt_misc/` registers no arm64 handler. A strategy
  that builds an arm64 rootfs here needs binfmt or an extract-only variant —
  a cost of the strategy, not an incidental detail.

## The double-sysroot hazard — a hypothesis, not a measurement

**Read from CMake 4.2.3's and SDL's own sources, never tested**, because testing
it needs a sysroot, which needs the decision this sequence is waiting on.
`TASK_CROSSPLATFORM_01` requirement 4 is where it gets settled.

`FindPkgConfig.cmake` never mentions `sysroot`, and `_pkg_create_imp_target`
assigns pkg-config's include dirs **verbatim**, while `GNU.cmake` sets
`CMAKE_<lang>_COMPILE_OPTIONS_SYSROOT "--sysroot="` and SDL's `CheckKMSDRM` goes
through `pkg_check_modules(… IMPORTED_TARGET …)`. So **if `CMAKE_SYSROOT` and
`PKG_CONFIG_SYSROOT_DIR` are the same directory** — which
`cmake/aarch64-toolchain.cmake` does whenever `ROADOS_SYSROOT` is non-empty —
`pkg-config` prefixes `-I/usr/include/libdrm` with the sysroot and `gcc` prefixes
it again: a probe for `<sysroot><sysroot>/usr/include/libdrm`. If it holds, the
fix is in the toolchain file, most likely by leaving `PKG_CONFIG_SYSROOT_DIR`
empty and letting `--sysroot` plus `PKG_CONFIG_LIBDIR` do the work.

## Left over

1. **The target platform** — SoC, board, and what the unit is. The gate on all
   four tasks, and not a question this file can answer.
2. **Then** requirement 1's strategy decision, with its consequences written into
   `doc/ui/CROSSBUILD.md`.