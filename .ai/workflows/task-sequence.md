# Workflow: task sequence

**Owner: this file.** It is the single canonical description of the per-task
implementation loop. If any other document states the loop, that document is
wrong — including `AGENTS.md`, which routes here rather than restating it, and
each sequence's `IMPLEMENTATION_STATE.md`, which records progress and points
here.

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

   If the task exceeds the scope threshold in `.ai/agents/developer.md` §
   *Scope check*, the developer splits it into isolated sub-tasks per
   `.ai/protocols/subagents.md` § *Implementation fan-out*. Each sub-task is
   dispatched as a separate `general` subagent. The developer integrates their
   work and returns a single handoff for the whole task.

2. **Review.** Dispatch `.ai/agents/reviewer.md` as `general`, in a **different
   session from the developer.** A reviewer in the author's session inherits the
   author's reasoning and reviews its own conclusions. Do not reuse the
   developer's session.

   If the task fanned out, the review covers the integrated result, not each
   sub-task separately. A sub-task that was reviewed in isolation and then
   integrated is reviewed again as part of the whole — integration can break
   what the parts proved.
3. **Fix, or waive.** Findings go back to the developer until each is fixed. A
   finding the operator declines to fix is **waived with a recorded reason** — a
   waiver is a decision, not a skipped step, and an unrecorded one is a bug.
4. **Stop for the operator.** Present the handoff, the verdict, and the diff
   scope. Do not proceed to the next task.
5. **Operator commits.** Per task, not per batch. The operator reviews, commits,
   and reports the SHA.
6. **Record and advance.** Move the task file to `doc/ui/done/`, record what is
   left over on the sequence's state file — and only that — then begin the next
   task. § *State* below owns the size cap and says what does not belong there.

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
- **No unmeasured run of the demo.** Every run of `ui_demo` an agent launches is
  measured with `.ai/tools/fps-check.sh`, and the numbers go into the handoff
  whether they are good or bad. This is a gate because the only other checks in
  the sequence are blind to a frame-cost regression: no test can see it, and a
  capture cannot either — a still of a 4 fps application is pixel-identical to a
  still of a 60 fps one, which is how a four-fps regression survived three
  reviews in this repository. **The rule itself is
  `.ai/agents/developer.md` § Phase 3**, which owns what is run and what is
  reported; it is not restated here.

## State

Each sequence's `IMPLEMENTATION_STATE.md` is a **status board, not a log**. It
answers three questions for a fresh session and carries nothing else:

- **Current position** — what is in flight, what the next task is, and the last
  measured suite figure.
- **Left over** — todos, waived or unmet acceptance criteria, tasks whose review
  or commit never happened, and the decisions that are the operator's to make.
- **Where the sequence's task files are.** That is all.

**The cap is 3 KB.** This file's rule, and the operator's on 2026-10-10: the
`UI_PRIM` state file was 527 KB of per-task narrative — fifteen sections of "what
it decided and what it found" — and a fresh session could not find the next task
in it. `doc/ui/IMPLEMENTATION_STATE.md` and
`doc/ui/IMPLEMENTATION_STATE_DEMO.md` are the two rewritten; this rule is why
they stay that size.

**A finished task is not in the state file at all.** Its task file moves to
`doc/ui/done/` and that file is its record. Do not summarise a done task in the
state file, do not keep a task-table row for it, and do not give it a record
section — the state file's silence *is* the record of "this is finished", and a
duplicate is a second copy to keep in step with the first. What survives the
task is the *leftover*: an acceptance criterion not met, a review or commit
still owed, a defect found and not fixed. Those go on *Left over*, dated.

**A pending task file asks for the opposite** — a task-table row, a
§ *Task NN — what it decided* section, a dated entry. Those requirements are
**superseded** by this section, and each pending file carries a dated marker
saying so. Put the durable fact where it belongs: the code's own doc for what
the code does, the task file for what the task decided, *Left over* for what is
still owed.

A state file is **not** the source of the workflow, and it is not a source of
evidence. A sidecar or state file that contradicts its artifact is wrong and
gets fixed — see `AGENTS.md` on working context.

There are three: `doc/ui/IMPLEMENTATION_STATE.md` (`TASK_UI_PRIM_*`),
`doc/ui/IMPLEMENTATION_STATE_DEMO.md` (`TASK_UI_DEMO_*`, created 2026-10-09 by
`doc/ui/done/TASK_UI_DEMO_01.md` requirement 7) and
`doc/platform/IMPLEMENTATION_STATE.md` (`TASK_CROSSPLATFORM_*`). This workflow
applies to all three.

**`doc/ui/backlog/` is not a sequence and is not covered by this workflow.** A
task file there is superseded: it has no state-file row, no review and no
operator-commit gate, because nothing in it is being built. What moving a file
there means is recorded in that directory's own `README.md`.

## Scope

Applies to a task-file sequence: `doc/ui/TASK_UI_PRIM_*.md`,
`doc/ui/TASK_UI_DEMO_*.md` and `doc/platform/TASK_CROSSPLATFORM_*.md`.

The `DEMO` sequence is `doc/ui/TASK_UI_DEMO_*.md` — the seven-page demo
application — with `doc/ui/IMPLEMENTATION_STATE_DEMO.md` as its state file.
**It was added to this list on 2026-10-09 by `TASK_UI_DEMO_01`**, which found
that the sequence had no workflow and therefore nowhere to record progress;
before that, the five `DEMO_*` task files specified the demo application with no
loop over them. Its task files were specified 2026-10-05 and none was started
when the sequence was added.

The `CROSSPLATFORM` sequence was created
2026-10-05 by moving tasks 25, 26, 27 and 29 out of the `UI_PRIM` sequence, and
is deferred until the operator decides the target platform; nothing in it is in
flight. A single self-contained change does not need a loop; it needs a build and
a test.
