# Subagent protocol

This protocol is canonical for this repository. `researcher.md` and
`idea-evaluator.md` dispatch by it and must not restate it. If either agent
disagrees with this file, this file wins.

## Roster

The Task tool in this harness provides two subagent types. Do not name a
third.

- **`explore`** — facts about this repository: what exists, what a module
  does, what a file already assumes. Fast, read-only. The default.
- **`general`** — anything outside the repository: upstream projects, licence
  texts, hardware documentation, standards, competitors, prior art, field
  reports. Give it web access needs explicitly.

There is no `scout`. Reading a dependency's actual implementation is
`general`'s job, escalated per the ladder below.

## Fan-out

- **One question per subagent.** A subagent given two questions answers one
  and hand-waves the other, and you cannot tell which.
- **Three to six is the useful range.** Fewer wastes the parallelism; more
  produces a pile nobody reads.
- **All dispatches go in one message**, so they run concurrently.
- **Do not let subagents fan out further.** Nesting multiplies cost and makes
  results unattributable. If a subagent reports that its question depends on
  another question, dispatch that as a new top-level subagent.
- **Do not stall.** Research on one question is not a blocker on the synthesis
  of the others. Keep working while subagents run.
- **Do not research what needs no research.** A claim already settled at
  `[A]` is not a subquestion. Checking whether it is *still* settled is, and
  for a volatile fact class that check is cheap — so make it one.
- **Do not research decisions.** "Is Slint a good UI toolkit" is a judgement.
  "What licence governs Slint's embedded Linux backend" is a fact. Convert
  judgements into their factual components first.

## Implementation fan-out

Research fan-out is read-only and answers questions. Implementation fan-out
writes code. The rules are different.

### When to use it

The developer agent estimates scope per `.ai/agents/developer.md` § *Scope
check*. When a task exceeds the threshold, the developer splits it into
isolated sub-tasks and dispatches one `general` subagent per sub-task.

**The developer is the orchestrator, not a peer.** It splits the work, briefs
each subagent, collects the handoffs, and integrates. Subagents do not see
each other's work and do not coordinate directly.

### Splitting

A valid split has three properties:

1. **File isolation.** Each subagent works on a disjoint set of files. Two
   subagents editing the same file is a failed split — the second write wins
   and the first subagent's work is silently lost.
2. **Testable independently.** Each sub-task has an acceptance test that
   passes without the other sub-tasks landing. A sub-task that cannot be
   verified alone is not a sub-task; it is a phase of a monolithic change.
3. **No hidden dependencies.** If sub-task B requires a type or function from
   sub-task A, they are not independent. Either merge them or make A a
   prerequisite that lands first.

**Split along module boundaries, not arbitrary chunks.** A file is the unit of
isolation. If a task requires two changes to the same file, it is one sub-task,
not two.

### Parallel or sequential

The developer decides the execution order:

- **Parallel** when sub-tasks touch disjoint files and have no data
  dependencies. All dispatches go in one message, per the research fan-out
  rules.
- **Sequential** when sub-task B reads a type or function sub-task A creates.
  A lands, is verified, and B is briefed against the now-existing code.
- **Mixed** when some sub-tasks are independent and others depend on them.
  Dispatch the independent ones in parallel, then the dependent ones in
   sequence.

**Do not dispatch a subagent whose brief depends on code that does not exist
yet.** A subagent told to "use the Foo type from the other subagent" will
invent it. Brief against the codebase as it stands, or wait for the prerequisite
to land.

### Briefing contract

Each implementation subagent gets a self-contained brief containing:

- **The sub-task**, in one or two sentences: what to build, and why it matters
- **The files it owns** — the disjoint set it may create or modify
- **The acceptance test** — how the subagent knows it is done
- **The constraints** — the project's conventions, the error-handling rules,
  the dependency policy, the coding standards from `developer.md`
- **What it must not touch** — files owned by other subagents, and the
  boundaries of its sub-task
- **The return format** — a handoff, not a research answer
- **Read-write access** — unlike research, implementation subagents write to
  the repository. State this explicitly.

### Return format

Implementation subagents return a handoff, not a research answer:

```
SUB-TASK: <the sub-task, restated>
DONE: <what was built, in one paragraph>
FILES: <the files created or modified>
VERIFIED: <the commands run, with real output — build, test, lint>
LEFT OUT: <what was not done, and why>
RISKS: <decisions made that the spec left open, or places where you guessed>
```

A subagent that returns prose instead of this is a **failed dispatch**.
Re-send the brief. Do not retrofit its prose into the format yourself: the
verification and the risks are the subagent's to report, and they have to be
the subagent's, or the apparatus is decoration.

### Coordination rules

- **No nesting.** Implementation subagents do not fan out further. If a
  sub-task is too large for one subagent, the developer split it wrong.
- **No shared state.** Subagents do not communicate through files, git
  branches, or any mechanism other than the developer's integration.
- **The developer integrates.** After all subagents return, the developer
  merges their work, resolves conflicts, and runs the full verification suite
  before handing off to review.
- **A failed subagent is re-dispatched, not patched.** If a subagent returns
  incomplete work, the developer re-dispatches it with a corrected brief. The
  developer does not finish a subagent's work itself — that would make the
  developer the author of code it did not design, and the review would lose
  its meaning.

## Briefing contract

Subagents start with no context: anything you leave out, they will invent.
Every dispatch gets a self-contained brief containing:

- the question, in one or two sentences, and **why the answer matters**
- the constraints that shape the answer: licence, target hardware, project
  goals, the operator's real budget or time
- the specific sources to prefer, and **what counts as an acceptable source**
- which step of the escalation ladder is expected to suffice
- **read-only**: no writes to this repository. If it must build, clone, or
  install, confine it to a temp directory and say so
- the required return format
- today's date, so a stale answer is visibly stale

## Return format

```
QUESTION: <the question, restated>
ANSWER: <one to three sentences>
CONFIDENCE: [A] primary read | [B] reputable secondary | [C] unverified
EVIDENCE: <the exact file path and line, API field, or quoted sentence that
          supports the answer — not the URL alone>
SOURCES: <url or file path, one per line, with the date read>
NEGATIVE: <what you looked for and did not find, or "none">
CAVEATS: <what would change this answer, or "none">
```

A subagent that returns prose instead of this is a **failed dispatch**.
Re-send the brief. Do not retrofit its prose into the format yourself: the
confidence tag and the evidence line are judgements, and they have to be the
subagent's, or the whole apparatus is decoration.

`EVIDENCE` is what makes `[A]` falsifiable. `NEGATIVE` is what makes an empty
result reportable rather than indistinguishable from not having looked.

## Reaching the source

L2 means reading the artefact. It does **not** mean cloning the repository.
Cloning is a late and often unnecessary step, and reaching for it by reflex
spends a minute of wall clock and a repository's entire history to answer a
one-line question. Escalate only as far as the question requires:

1. **An API field.** crates.io, GitHub, a release feed. Machine-readable,
   already parsed, one request. Answers most version, licence, and activity
   questions outright.
2. **One file over HTTP.** A raw source file, a `docs.rs` source view, a
   vendored licence text, a CI config, a build manifest. This is L2, it is one
   request, and it is the correct answer far more often than a clone is.
   For a large HTML page, save it and reduce it with `.ai/tools/topy.py` to
   locate the section — then cite the saved HTML, never the reduced text. The
   tool's README states why, and what it may not be used for.
3. **A release tarball.** The published artefact — what users actually
   install — as distinct from `main`, which they do not. Divergence between
   the two is itself worth reporting.
4. **A shallow clone** (`--depth 1`), and `git log` scoped to one path for L4.
5. **A full clone.** Only for archaeology across many revisions, and only when
   a shallow history has already proven insufficient.

Steps 1 and 2 resolve the large majority of questions. Two rules make this
stick:

- **Say in the brief which step is expected to suffice.** "The CI workflow
  file under `.github/workflows/` is enough; do not clone."
- **A subagent that cannot answer within the briefed escalation reports `[C]`
  and stops.** It does not quietly escalate, and it does not substitute a
  search snippet for the source it was told to read. A gap the operator can
  see is worth more than an answer assembled from the wrong tier.

Banned outright: reaching for a clone when step 1 or 2 would have answered the
question, and citing a repository's `master` when the question was about the
released version.

## Failure modes

- **Fan-out theatre.** Six subagents whose answers nobody cross-checked. The
  fan-out buys parallelism, not truth; the verification is where the truth
  comes from.
- **Cloning by reflex.** Reaching for a full clone when one HTTP request would
  have answered the question.
