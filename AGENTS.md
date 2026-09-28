# AGENTS.md

## Project

Read `doc/IDEA.md`.

## Architecture

Read `doc/architecture/overview.md` before making architectural changes,
when that document exists.

Target architectures: aarch64 and x86_64. OpenGL ES 3.1.

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

For a `doc/ui/TASK_UI_PRIM_*.md` sequence, `task-sequence.md` is canonical and
additionally owns the per-task loop and the operator-commit gate. It is not
restated here.

## Planned, not yet written

Referenced by the design, absent from the repository. Do not assume their
contents; read this file again when they land.

- `.ai/skills/` — reusable capabilities
- `.ai/agents/architect.md` — architecture decisions

Skills provide specialized instructions and workflows for specific tasks.
