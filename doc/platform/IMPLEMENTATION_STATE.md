# Cross-platform and target — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins.

**Sequence:** `doc/platform/TASK_CROSSPLATFORM_01..04.md` — *Target image and
sysroot*, *head-unit video driver*, *target runtime library audit*, *head-unit
smoke test*, moved out of `doc/ui/` on 2026-10-05; gated on the target platform
until the operator decided it on 2026-10-16 (`TARGET_PLATFORM.md`).
`doc/ui/CROSSBUILD.md` stays in `doc/ui` and is the companion document.

**Updated:** 2026-10-16.

## Current position

**Nothing in flight; nothing dispatched.** All four tasks are `pending` —
unstarted and ungated since 2026-10-16.

**The next decision is still the operator's.** `TASK_CROSSPLATFORM_01`
requirement 1 — *"Choose the strategy and record the reasoning… **The operator
decides; the developer implements.**"* — and everything downstream depends on
it: 02 needs target `libdrm`/`gbm`, 03 the image 02's driver runs on, 04 a
device. The target is aarch64 Linux (BCM2711), so the sequence stands as
written.

## Measured on this host, 2026-10-16

Re-measured for requirements 3 and 4; supersedes the 2026-10-05 block. Check
before relying on it.

- **Host `pkg-config` present** (host `libdrm` 2.4.124 resolves);
  `aarch64-linux-gnu-pkg-config` still missing — the hard requirement in
  `CROSSBUILD.md` § *Prerequisites*. `aarch64-linux-gnu-gcc` **is** present.
- **Still no rootfs tool**: no `debootstrap`, `mmdebstrap` or
  `qemu-aarch64-static`. No foreign architectures; no arm64 handler under
  `/proc/sys/fs/binfmt_misc/`. A strategy that builds an arm64 rootfs here needs
  binfmt or an extract-only variant — a cost of the strategy, not an incidental
  detail.

## The double-sysroot hazard — a hypothesis, not a measurement

**Read from CMake 4.2.3's and SDL's own sources, never tested**, because testing
it needs a sysroot, which task 01 has not yet produced.
`TASK_CROSSPLATFORM_01` requirement 4 is where it gets settled.

`FindPkgConfig.cmake` never mentions `sysroot`, and `_pkg_create_imp_target`
assigns pkg-config's include dirs **verbatim**, while `GNU.cmake` sets
`CMAKE_<lang>_COMPILE_OPTIONS_SYSROOT "--sysroot="` and SDL's `CheckKMSDRM` goes
through `pkg_check_modules(… IMPORTED_TARGET …)`. So **if `CMAKE_SYSROOT` and
`PKG_CONFIG_SYSROOT_DIR` are the same directory** (what the toolchain file does
with non-empty `ROADOS_SYSROOT`), the prefix applies twice: a probe for
`<sysroot><sysroot>/usr/include/libdrm`. Suspected fix: leave
`PKG_CONFIG_SYSROOT_DIR` empty; let `--sysroot` plus `PKG_CONFIG_LIBDIR` do the
work.

## Left over

1. **Sysroot + provision scripts** — strategy decided 2026-10-16 (Pi OS Lite
   64-bit pinned, bench-only; SSH provision, no baking; deb-extracted sysroot;
   < 7 s boot target). Next: build both scripts, then 02.