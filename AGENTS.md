# AGENTS.md

## Project

Read `doc/IDEA.md`.

## Architecture

Read `doc/architecture/overview.md` before making architectural changes,
when that document exists.

Target architectures: aarch64 and x86_64. OpenGL ES 3.1.

## Rust

The Rust workspace is `ui/`: the `ui_core` library, the `ui_demo` binary, and
`ui/Cargo.lock`, which is committed because the workspace ships a binary.
Build output is `/ui/target/` and is ignored.

Before changing a manifest, read `doc/ui/CROSSBUILD.md` §5.4 for the SDL
features and the video-driver split, and `doc/ui/IMPLEMENTATION_STATE.md` for
the operator's decisions. Dependency **versions** are in
`doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Dependencies*, and only there — that
section's **feature list is known to be incomplete**;
`doc/ui/IMPLEMENTATION_STATE.md:346` records it as wrong in two places, and its
`features = ["build-from-source"]` would delete
`build-from-source-unix-console`, which
`doc/ui/IMPLEMENTATION_STATE.md:221` makes mandatory for the aarch64 target.

The list below is a pointer document, not a second owner of any rule.
`.ai/agents/developer.md` owns the agent rules and must be read before
non-trivial work. Its **verification suite**, its **panic and `unsafe` policy**
and its **doc-comment and `#[must_use]`** rules are not restated here, because a
second copy of a rule is a divergence waiting to happen; only what
`developer.md` does not say appears below, and where this section records an
exception it names the rule it excepts.

- **Edition 2021, `rust-version = "1.85"`** — the floor `sdl3-sys` declares.
- **No new dependency** without the operator, per `.ai/agents/developer.md`.
  The approved **direct** dependencies are `sdl3 0.20`, `glow 0.18` and
  `freetype-rs 0.38` (`bundled`); the resolved graph is larger and is not
  gated. The font crate was approved by the operator 2026-09-30 for the
  task 11 text rendering pipeline; it builds its vendored C from source and
  statically links, like SDL3. HarfBuzz was approved then dropped the same
  day: its safe binding exposes no shaping API (only `unsafe` C calls), and
  the operator declined `unsafe`. Revisit when a complex-script or bidi
  requirement lands.
- **Error handling deviates from `developer.md` on purpose.** It prescribes
  `thiserror` and `anyhow`; neither is an approved dependency, so a binary
  returns `Result<(), Box<dyn std::error::Error>>` and a library returns the
  underlying error — `sdl3::Error` — and lets `?` carry it. Revisit when an
  error-type dependency is approved.
- **Tests** go in a `#[cfg(test)] mod tests` beside the code, or in
  `<crate>/tests/` when they test the public API. No test framework, no
  `dev-dependencies`. No test that needs a display, a network, a filesystem or
  the wall clock. `developer.md`'s suite is run from `ui/`.

## Never again

Common AI pitfalls for this project are recorded in `.ai/NEVERAGAIN.md` —
observed failures, dated, with the rule that replaces each one. Read it before
non-trivial work; add to it whenever a fix is made because an agent got
something wrong.

## Working context

A working document may have a sidecar: `<file>.context.md`, beside the file it
describes. It holds live working state — status, what the operator ratified, what
an agent added without approval, caveats, next step, dated history.

Four rules keep it from becoming a shadow spec:

- **A sidecar is not a source.** Nothing in it may be cited as evidence, and no
  confidence tag is earned by reading it.
- **The artifact wins.** A sidecar that contradicts its file is wrong, and gets
  fixed.
- **No rules live here.** If it is a rule, it belongs in the file. A sidecar
  describes the state of a rule; it never extends one.
- **Superseded, not deleted**, and every entry dated and attributed.

Write one only when there is live state. An empty sidecar is noise, exactly as an
empty directory is.

## AI Engineering System

This repository uses `.ai/` as its AI engineering configuration.

Before performing non-trivial work, inspect:

- `.ai/agents/` — specialized agent instructions
- `.ai/protocols/` — shared mechanics: subagent dispatch, evidence and
  confidence. These are canonical; an agent file that disagrees with one is
  the one that is wrong.
- `.ai/schemas/` — structured artifact definitions
- `.ai/tools/` — small utilities, each documented with what it may not be used
  for
- `.ai/workflows/` — engineering workflows
- `doc/findings/` — research already done; check it before re-researching
- `doc/platform/` — the target-platform and cross-compilation sequence,
  `TASK_CROSSPLATFORM_01..04.md`, **deferred until the target platform is
  decided**; its `IMPLEMENTATION_STATE.md` says so and is the place to resume it

The `.ai/` directory is part of the project's engineering infrastructure.
Do not modify its contents unless the task explicitly concerns the AI
engineering system — reviewing, extending, or fixing it counts.

### Harness

opencode. The Task tool provides two subagent types:

- `explore` — read-only facts about this repository
- `general` — anything outside it: web, upstream sources, licences, standards

### Agent selection

Use the relevant instructions from `.ai/agents/` according to the task:

- Idea evaluation, critique, alternatives → `.ai/agents/idea-evaluator.md`
- Research → `.ai/agents/researcher.md`
- Implementation → `.ai/agents/developer.md`
- Code review → `.ai/agents/reviewer.md`

### Workflow

For non-trivial changes, follow the applicable workflow in `.ai/workflows/`,
starting with `idea-to-code.md`. Do not skip its gates — an unconfirmed
verdict, an unverified build, or a waived finding are each a decision, not an
oversight.

For a `doc/ui/TASK_UI_PRIM_*.md` or `doc/platform/TASK_CROSSPLATFORM_*.md`
sequence, `task-sequence.md` is canonical and additionally owns the per-task loop
and the operator-commit gate. It is not restated here.

## Planned, not yet written

Referenced by the design, absent from the repository. Do not assume their
contents; read this file again when they land.

- `.ai/skills/` — reusable capabilities
- `.ai/agents/architect.md` — architecture decisions

Skills provide specialized instructions and workflows for specific tasks.
