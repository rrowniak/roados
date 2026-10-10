# Cross-platform and target — implementation state

**Purpose:** so a fresh session resumes the target-platform work at the right
task and does not re-derive what is already settled. This is working state, not
a spec: the task files own the requirements, and where this file and a task file
disagree, the task file wins and this file gets corrected.

**Spec:** `doc/platform/TASK_CROSSPLATFORM_01..04.md`.

**Created:** 2026-10-05, by moving four tasks out of `doc/ui/`.

## Current position

**Status: all four tasks are `pending`, and the sequence is deferred. Nothing is
in flight, and nothing has been dispatched.** The operator's 2026-10-05 decision
was to move this work out of the UI sequence and **not to begin it until the
target platform is decided**.

That ordering is not a convenience — it is a dependency. `TASK_CROSSPLATFORM_01`
requirement 1 is *"Choose the strategy and record the reasoning… **The operator
decides; the developer implements.**"*, and its two named candidates (Debian
multiarch, Buildroot/Yocto) are both stated in terms of a Linux target. Every
later task depends on that answer: 02 needs target `libdrm`/`gbm`, 03 needs the
image 02's driver runs on, and 04 needs a device. So the target-platform decision
is upstream of all four, and `01` is the first place its consequences land.

**This file, not `doc/ui/IMPLEMENTATION_STATE.md`, is where this work is
resumed from.** `doc/ui`'s file records the move in its *History* and no longer
claims these tasks as its own.

## Tasks

| # | Task | Was | Status | Needs |
|---|---|---|---|---|
| 01 | Target Image and Sysroot | `TASK_UI_PRIM_25.md` | **deferred** | operator target-platform decision, then the strategy decision |
| 02 | Head-Unit Video Driver | `TASK_UI_PRIM_26.md` | **deferred** | 01 |
| 03 | Target Runtime Library Audit | `TASK_UI_PRIM_27.md` | **deferred** | 01, 02 |
| 04 | Head-Unit Smoke Test | `TASK_UI_PRIM_29.md` | **deferred** | 01, 02, 03 |

The three status words differ from `doc/ui`'s table on purpose: **`deferred` is
not `pending`.** `pending` means unstarted; `deferred` means unstarted *and*
gated on a decision that has not been made, so a reader cannot mistake the gate
for a lack of work. `doc/ui`'s status vocabulary is defined in its *Task table*
section and does not carry this word; the sequence's own vocabulary is here.

**Task 28 stayed in `doc/ui`.** `TASK_UI_PRIM_28.md` (*Reconcile the SDL
Configuration*) was part of the same 2026-09-28 batch and did **not** move: its
goal is making `doc/ui/PRIMITIVES_ARCHITECTURE.md` agree with reality, and its
requirement 5 is a seam inside the UI sequence (task 04/05). Its *Out of Scope*
now cites the new paths.

## Measured on this host, 2026-10-05 — before any of this work starts

These were established while scoping `TASK_CROSSPLATFORM_01` and they are
recorded here because they are the inputs requirement 3 and requirement 4 will be
measured against, and re-measuring them costs a session.

**`pkg-config` is not installed at all** — not the host one, not the
cross-prefixed one. `command -v pkg-config` and
`command -v aarch64-linux-gnu-pkg-config` both find nothing. This is the state
that made §6.7's finding, and it is why `CROSSBUILD.md` §8 item 3 calls a target
`pkg-config` a hard requirement rather than a convenience. `aarch64-linux-gnu-gcc`
**is** present.

**Neither rootfs tool is installed**: no `debootstrap`, no `mmdebstrap`, no
`buildroot`, no `qemu-aarch64-static`. `dpkg --print-foreign-architectures`
reports `i386` only, so no arm64 packages are enabled on the host, and
`/proc/sys/fs/binfmt_misc/` registers no arm64 handler — only `python3.14`.
**A strategy that needs an arm64 rootfs built here will need binfmt or an
extract-only variant**, and that is a cost of the strategy, not an incidental
detail.

## The double-sysroot hazard — a hypothesis, stated as one

`TASK_CROSSPLATFORM_01` requirement 4 asks for the toolchain file's sysroot
branch to be verified against a real sysroot. This is the most likely thing that
verification will surface, and it is recorded now so a later session measures it
rather than re-derives it.

**Reading CMake 4.2.3's own sources:**

- `Modules/FindPkgConfig.cmake` contains **zero** case-insensitive occurrences of
  `sysroot`. There is no de-duplication to inherit.
- `_pkg_create_imp_target` (`FindPkgConfig.cmake:791`) assigns
  `${${_prefix}_INCLUDE_DIRS}` to `INTERFACE_INCLUDE_DIRECTORIES` **verbatim** —
  whatever `pkg-config` printed, unrewritten.
- `Modules/Compiler/GNU.cmake:37` sets
  `CMAKE_${lang}_COMPILE_OPTIONS_SYSROOT "--sysroot="`, so the compiler receives
  `--sysroot=<dir>`.
- SDL's `CheckKMSDRM` (`cmake/sdlchecks.cmake:1370-1371`) uses
  `pkg_check_modules(… IMPORTED_TARGET …)`, so it goes through exactly that
  imported-target machinery.

**Therefore, if `CMAKE_SYSROOT` and `PKG_CONFIG_SYSROOT_DIR` are set to the same
directory** — which is what `cmake/aarch64-toolchain.cmake:129-158` does
whenever `ROADOS_SYSROOT` is non-empty — then `pkg-config` prefixes
`-I/usr/include/libdrm` with the sysroot, and `gcc` prefixes it again with
`--sysroot=`. The expected result is a probe for
`<sysroot><sysroot>/usr/include/libdrm`.

**This is a reading of the sources, not a measurement.** It was not tested,
because testing it requires a sysroot, which requires the decision this sequence
is waiting on. `TASK_CROSSPLATFORM_01` requirement 4 is where it gets settled,
and if it holds, the fix is in the toolchain file — most likely by leaving
`PKG_CONFIG_SYSROOT_DIR` empty and letting `--sysroot` plus
`PKG_CONFIG_LIBDIR` do the work.

## Decisions

- **2026-10-05 — the strategy decision is deferred, not made.** No choice between
  Debian multiarch and Buildroot/Yocto has been recorded, and none is implied by
  this file. `TASK_CROSSPLATFORM_01` requirement 1 stands.
- **2026-10-05 — the four tasks moved here rather than being rewritten.** The
  task text is verbatim aarch64-Linux prose and says nothing about "the target
  platform" in the abstract. The operator's instruction was to move and defer,
  not to re-scope, so nothing was reworded. **If the target platform turns out
  not to be aarch64 Linux, these task files need revision, not just
  relocation** — the sysroot candidates, the multiarch discussion and the
  `aarch64-linux-gnu-pkg-config` requirement are all aarch64-Linux-specific.
- **2026-10-05 — `doc/ui/CROSSBUILD.md` stayed in `doc/ui`.** It reads as
  platform documentation, but `cmake/aarch64-toolchain.cmake` and `AGENTS.md` §
  Rust both cite it at that path, and a code file citing a moved document is a
  broken reference. Relocating it is a separate, larger change.

## What this sequence still needs from the operator

1. **The target platform.** SoC, board, and what the unit actually is — this is
   the gate on all four tasks, and it is not a question this file can answer.
2. **Then** `TASK_CROSSPLATFORM_01` requirement 1's strategy decision, with its
   consequences written into `doc/ui/CROSSBUILD.md` and
   `doc/ui/IMPLEMENTATION_STATE.md` as that requirement specifies.

## History

- 2026-10-05 — **the sequence was created by moving four tasks out of
  `doc/ui/`.** `TASK_UI_PRIM_25`, `_26`, `_27` and `_29` became
  `TASK_CROSSPLATFORM_01`–`_04` via `git mv`, so each keeps its history. Every
  cross-reference to the old numbers was corrected in the moved files, in
  `doc/ui/done/TASK_UI_PRIM_28.md`, in `doc/ui/CROSSBUILD.md` and in
  `doc/ui/IMPLEMENTATION_STATE.md`. `doc/ui/IMPLEMENTATION_STATE.md`'s *History*
  records the move; its 2026-09-28 entry saying *"tasks 25–29 drafted"* was left
  as it stood, because it was true when written.
- 2026-10-05 — **the three findings above were measured while answering the
  operator's question about what task 25 was**, before any file was moved. They
  are recorded here so the next session does not re-run the same probes. No code
  was written, no build was run, and no task was implemented for this entry: it
  records a move and a deferral.