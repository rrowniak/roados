# Workflow: idea to code

Sequence and gates for a non-trivial change. The craft and judgement live in
`.ai/agents/developer.md`; the evaluation lives in `.ai/agents/idea-evaluator.md`;
the facts live in `.ai/agents/researcher.md`. This file is the order and the
stops.

## Stages

```
0  Frame        operator or agent   is this worth a workflow at all?
1  Evaluate     idea-evaluator      verdict + rewritten idea + open questions
   ── gate ──   operator            confirms the verdict. No build without it.
2  Research     researcher          only the claims the verdict left [C]
   ── gate ──   operator            a new [A] contradicts the verdict? back to 1
3  Shape        developer           smallest diff that tests the idea
4  Implement    developer           the change
5  Verify       developer           build, tests, lint — real output
   ── gate ──   reviewer            findings fixed or waived, then merged
6  Feed back    developer            NEVERAGAIN.md, and doc/findings/ if a
                                     factual question was settled
```

Stages 2 and 3 are not always both present. A bug fix with a known cause
enters at stage 3, not stage 0. A change whose only question is "does this
compile" is stage 3 and nothing else.

## Stage 0 — Frame

Is this non-trivial? A change that touches one file, deletes a comment, or
fixes a typo does not need six stages and a verdict. Say so and just do it.

Everything else continues.

## Stage 1 — Evaluate

Run `.ai/agents/idea-evaluator.md`. It ends at a verdict, a rewritten idea
paragraph, and an ordered list of open questions.

**Gate:** the operator confirms. The evaluator does not proceed on its own
authority, and neither does the developer. A verdict nobody confirmed is an
idea, not a spec.

## Stage 2 — Research

Dispatch research only for the claims the verdict left open and load-bearing.
Check `doc/findings/` first; most questions are already answered there, and a
finding inside its `decay` horizon is not to be re-researched.

**Gate:** if a finding comes back `[A]` and contradicts the verdict, return to
stage 1. Do not resolve a contradiction between the idea and its own evidence
by quietly choosing one.

## Stage 3 — Shape

Write down, before code:

- the acceptance test
- what is explicitly out of scope
- whether the codebase has conventions to match or ones to establish

In an empty repository, this stage is where the conventions get set, and they
get recorded where the next agent will find them.

## Stages 4 and 5 — Implement and verify

Per `.ai/agents/developer.md` Phases 2 and 3. Verification is performed, not
reported on. The output of these stages is a handoff: what was built, the
commands run with their real output, what was left out, and what the reviewer
needs to know.

## Stage 5 gate — Review

`reviewer.md` is a separate agent, deliberately. An author reviewing their own
change is checking it against the intention they already hold, which is the one
thing review cannot do.

Findings are fixed or explicitly waived with a reason. Not waived silently —
a waived finding is a decision, and decisions get written down.

## Stage 6 — Feed back

Two things, both cheap, both skipped constantly:

- **`.ai/NEVERAGAIN.md`** — anything an agent got wrong and you had to fix.
- **`doc/findings/`** — any factual question the build settled. Offer once,
  after the merge, not during the work.

## Failure of a gate

A failed gate is not a failure of the workflow; it is the workflow working. Say
which gate failed, what the state is, and what the cheapest way forward is.

Do not proceed on a gate you could not evaluate. "I could not check whether the
build passes, so I will assume it does" is the gate being skipped, and it is
worse than a hard stop.
