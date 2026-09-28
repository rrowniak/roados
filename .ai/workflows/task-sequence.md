# Workflow: task sequence

**Owner: this file.** It is the single canonical description of the per-task
implementation loop. If any other document states the loop, that document is
wrong — including `AGENTS.md`, which routes here rather than restating it, and
`doc/ui/IMPLEMENTATION_STATE.md`, which records progress and points here.

The 2026-09-27 operator decision to keep `reviewer.md` as the only review
document still holds. This file does not restate what a review is or how one is
conducted; `.ai/agents/reviewer.md` owns that. What lives here is the thing no
single agent file owns: **the order of the steps, and the gates between them.**

## The loop

One task at a time, in numeric order. Not overlapping, not batched.

1. **Developer.** Dispatch `.ai/agents/developer.md` as a `general` subagent
   (`.ai/protocols/subagents.md` allows only `explore` and `general`; `developer`
   and `reviewer` are instruction files, not subagent types). It implements the
   task and returns a handoff: what it did, what it verified, what it left out.
2. **Review.** Dispatch `.ai/agents/reviewer.md` as `general`, in a **different
   session from the developer.** A reviewer in the author's session inherits the
   author's reasoning and reviews its own conclusions. Do not reuse the
   developer's session.
3. **Fix, or waive.** Findings go back to the developer until each is fixed. A
   finding the operator declines to fix is **waived with a recorded reason** — a
   waiver is a decision, not a skipped step, and an unrecorded one is a bug.
4. **Stop for the operator.** Present the handoff, the verdict, and the diff
   scope. Do not proceed to the next task.
5. **Operator commits.** Per task, not per batch. The operator reviews, commits,
   and reports the SHA.
6. **Record and advance.** Record the SHA in the state file, then begin the next
   task.

## Gates

These are the things that get skipped, so they are named explicitly. A skipped
gate is a decision made silently, which is the failure this workflow exists to
prevent.

- **No self-review.** A developer does not clear its own work. Step 2 is not
  optional because the developer felt confident.
- **No unreviewed advance.** A verdict of `fix first` that is never re-reviewed
  is not a cleared verdict. Step 3 loops back to step 2.
- **No evidence by assertion.** A claim about a file, a count, a version, or a
  build result is checked against the file or the command output. A quotation
  that no longer matches what it was evidence for is a defect, and this has been
  caught twice in this sequence: a stale `-march` flag, and stale option state.
- **No blanket waivers.** An unverifiable acceptance criterion may be waived
  with a reason. A verifiable one may not, and a waiver outliving its reason
  becomes a fiction.
- **No uncommitted advance.** Step 5 precedes step 6. An uncommitted task leaves
  no revert point for the next one.

## State

`doc/ui/IMPLEMENTATION_STATE.md` records status, decisions, waivers, blockers and
history for a multi-task sequence, so a fresh session can resume without
re-deriving it. It is **not** the source of the workflow, and it is not a source
of evidence. A sidecar or state file that contradicts its artifact is wrong and
gets fixed — see `AGENTS.md` on working context.

## Scope

Applies to a task-file sequence: `doc/ui/TASK_UI_PRIM_*.md`. A single
self-contained change does not need a loop; it needs a build and a test.
