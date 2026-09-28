# Finding schema

One finding, one file, in `doc/findings/`. A finding is a single falsifiable
question that was researched and answered, with the evidence attached. It is
the durable output of `researcher.md` and the input every later agent reads
before re-researching anything.

Tags are defined in `.ai/protocols/evidence.md` and are not redefined here.

## Naming

`doc/findings/<question-slug>.md`

The slug comes from the question, not the answer, so that a later agent can
find the file by the thing it wants to know rather than by the conclusion it
has not reached yet. Lowercase, hyphenated, no dates, no numbers.

- `slint-embedded-backend-licence.md` — "What licence governs Slint's
  embedded Linux backend?"
- `can-bus-frame-id-arbitration.md` — "How is the Passat B5.5 comfort bus
  arbitrated?"

Do not slug a compound question. Split it, and dispatch both.

## Frontmatter

Required, in this order:

```yaml
---
question: <the falsifiable sentence that was researched>
answer:   <one to three sentences, written so it stands alone>
tag:      [A] | [B] | [C]
support:  <N independent sources; 1 is the normal honest case>
evidence: <file path and line, API field, or quoted sentence — not a URL alone>
read:     <YYYY-MM-DD, the date the source was read>
decay:    <horizon from the staleness table, or "stable">
unblocks: <the decision this finding feeds, in one line>
---
```

Optional, only when they apply:

```yaml
supersedes: <slug of the finding this one replaces>
```

### Field notes

- **`tag`** is the tag the *source* supports, at the lowest level any source
  supports. Never upgrade it here. If you re-opened the source yourself, that
  is a re-verification, recorded in the log below, not a silent edit.
- **`support`** counts *independent* sources. Five blogs citing one press
  release is `support: 1`.
- **`evidence`** is what makes `[A]` falsifiable. A finding with an `[A]` tag
  and no line-level evidence is malformed — retag it `[C]`.
- **`decay`** is copied from the staleness table in
  `.ai/protocols/evidence.md`. A `[C]` finding about a fast-moving subject
  still gets a date; that is what tells the reader it was never a fact.
- **`unblocks`** is what makes the file worth reading. A finding no decision
  will consult is not a finding, it is a blog post.

## Body

```markdown
# <the question, as a question>

<Detail: mechanism, quoted clause, measurement. Two or three sentences.>

**Implication:** <what this changes for the project>

**Reverses if:** <the cheapest observation that would overturn it, and
roughly what it would cost to make that observation>

## Searched

<Only for a null result: exact queries, databases, repos, issue trackers. A
null result without this section is indistinguishable from not having looked.>

## Revisit log

- 2026-09-27 — initial finding, [A] from <locator>
- 2027-02-14 — re-verified inside horizon, unchanged
- 2027-06-01 — horizon passed, licence had changed; now superseded by
  <slug>
```

## Rules

- **A finding is immutable in substance.** Re-verification appends to the
  revisit log and may bump `read`. It does not rewrite the answer in place,
  because a finding that quietly changed its mind is worse than no finding —
  nobody can tell it was ever wrong.
- **Superseding, not deleting.** When a finding is falsified, write the new
  one, set `supersedes:` on it, and add a `status: superseded` line to the old
  file's body. A corpus that hides its own corrections will be trusted after
  it stops being true.
- **Never launder a tag.** `[C]` in a subagent, `[B]` in a synthesis, `[A]`
  in a document three weeks later is the canonical failure, and this is the
  document where it happens.
- **Cite only what you fetched.** A failed fetch is a null result, reported
  as one, with the failure named.
- **One question per file.** A file with two answers has no answer.

## What does not go here

- **Decisions.** A finding is evidence. A decision is an ADR, or a paragraph
  in the document that owns the question.
- **Repository facts that are one grep away.** If `explore` can answer it in
  one read, it is not a finding and it will never be looked up again.
- **Anything `[C]` that is not load-bearing.** A tag on everything is a tag on
  nothing. Write the ones that feed a decision; let the rest decay in the
  conversation.
