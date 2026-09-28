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
