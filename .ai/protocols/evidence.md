# Evidence protocol

This file owns the confidence scale for this repository. The definition lives
here rather than inside any one agent so that grader and reader cannot
disagree about what a tag means. `researcher.md` and `idea-evaluator.md` cite
it and must not restate it.

## The confidence ladder

- **[A]** — read directly from a primary source: the upstream repository file,
  the `LICENSE`, the release feed, the official specification, the crates.io /
  GitHub API response, the mailing-list post by the author. The evidence line
  names the file and the line.
- **[B]** — a reputable secondary source: conference talk, engineering blog by
  a practitioner, well-maintained project documentation, journalism that names
  its sources. Not re-verified against the primary document.
- **[C]** — unverified, or snippet-level. A search result, a forum post, a
  README's claim about itself, an inference.

Two orthogonal qualifiers, inline, not as new tags:

- **staleness** — for volatile fact classes, the date and the re-verify
  horizon from the table below
- **support** — how many *independent* sources agree. One source is
  `support: 1`, and that is the normal, honest case. Independence is the
  question: five blogs citing one press release is `support: 1`.

### Inheriting a tag

A tag records that *somebody* read the primary source, not that you did.
Inheriting one is legitimate; laundering one is not.

- A tag read back from a document is `[A]` if its evidence line is present and
  checkable and its date is inside its staleness horizon. It is **not** "read
  in this session" — you have not read it this session.
- **Never upgrade a tag without opening the source yourself.** The usual
  corruption is `[C]` in a subagent, restated as `[B]` in a synthesis, cited
  as `[A]` in a document three weeks later.
- **Two snippets agreeing is one snippet cited twice.** Never launder a `[C]`
  into an `[A]` by repeating it in a second place.
- If you assert a claim fresh, read its source fresh, or tag it `[C]`.
- Prior `[A]` findings are not free to redo. Re-verify only what is stale
  inside its horizon, or what the decision it feeds has changed underneath.

## The staleness horizon

Different facts expire at different speeds. State the horizon with the fact, so
the reader knows which rows to re-verify and which are settled.

| Fact class | Re-verify after |
|---|---|
| Crate / package version, download count | 1 week — moves constantly |
| Last commit, release date, contributor count | 1 month — slow, but real |
| GitHub stars, issue counts | 1 month — vanity metric; a trend only |
| Patent litigation posture, pool membership | 1 month — moving fast in AV1 and HEVC |
| Licence of a standard, patent pool rate | 3 months — royalty schedules get renegotiated |
| OS or distro default, vendor roadmap | 6 months — slow, and announced quietly |
| Licence terms of an active project | 1 year, or on any release — rare, but catastrophic |
| A blog post or conference talk | never — one person's belief, one day |

## The depth ladder

For any load-bearing claim, climb at least to **L2**. For any claim about
production suitability, reach **L5**. If the ladder cannot be climbed, that is
the finding — and the tag is `[C]` no matter how authoritative the top of the
ladder looks.

- **L0 — someone said it.** Search snippet, forum post, social media. Tag
  ceiling `[C]`.
- **L1 — what the project intends.** Official documentation, marketing page.
  `[A]` for a specification, `[B]` for a capability claim.
- **L2 — what the project does.** The source file, the `LICENSE`, the API
  response. Tag ceiling `[A]`.
- **L3 — what is verified.** Its tests, CI config, packaging recipes. `[A]`,
  and the strongest evidence available.
- **L4 — what worked and still does.** History of the relevant path: `git
  log`, reverted commits, blame. `[A]`.
- **L5 — what survives contact.** Field reports: issues, mailing lists, talks
  by third parties. `[B]`, and the only source for this.
- **L6 — what is missing.** The gap between what the docs promise and what the
  repository contains. `[A]`, earned by search.

Two of these carry most of the weight:

- **L3 is the honest test of support.** Documentation says a backend exists. A
  CI job that runs it on the target says it exists. This single check
  separates most "supported" claims from most real ones.
- **L6 is where the research effort actually pays.** Read for the gap: absent
  tests, a backend listed in the README but not in the build matrix, a config
  option with no consumer, an issue closed as stale for a year.

## Banned inferences

These are the moves that produce a confident, wrong report. They are banned
regardless of how good the source sounds.

- **"X is maintained" from "X has commits".** One hobbyist is a maintenance
  stream, not a project. *Read instead:* contributor count, release cadence,
  bus factor, time-to-first-response on issues.
- **"X is fast" from a benchmark run on other hardware.** Benchmarks do not
  port, and the numbers that matter on a Pi are memory and thermal, not
  throughput. *Read instead:* a benchmark on the target, or an explicit
  statement that none exists.
- **"X is licensed Y" from a badge or a repository sidebar.** Badges are
  self-reported and routinely stale. *Read instead:* the `LICENSE` file, the
  licence field in the crate manifest, the SPDX identifier.
- **"X works on our target" from "X supports Linux".** Support claims mean a
  mainstream distro on a desktop GPU. *Read instead:* the build matrix, the CI
  target list, open issues naming your hardware.
- **"There is no alternative" from having not found one.** Absence of
  evidence. *Read instead:* state the search performed, name the databases
  queried, tag `[C]`.
- **"X will exist in N years" from present activity.** Activity is not
  continuity; projects get acquired, relicensed, and abandoned. *Read
  instead:* governance, funding, ownership, licence history, bus factor.
- **"13 MiB binary means 13 MiB of RAM."** Binary size is on-disk stripped
  code; runtime cost is resident memory plus allocator overhead, per subsystem.
  *Read instead:* runtime RSS on target, per subsystem. Report both numbers,
  never one standing in for the other.
- **"High stars means healthy."** Stars are a one-time vote and cheap to
  accumulate. *Read instead:* stars against recent contributors, issue close
  rate.
- **"The docs say it is stable."** Docs are written by the people who want you
  to adopt it. *Read instead:* release policy, deprecation history, breaking
  changes per year.
- **"It is cleaner than the alternative" without a benchmark.** Taste is not a
  measurement. *Read instead:* the benchmark, or admit it is a preference.

A shorter form of the same rule: **if the sentence would survive if the
evidence were deleted, delete the sentence.** "The maintainers are responsive"
survives deleting the citation. It is not a finding.

## Null results

"No evidence found" is a result, and it is reported as one. It carries:

- the question, restated
- **what you searched** — exact queries, databases, repos, issue trackers
- the fact that it is a negative result, tagged `[C]`
- what *would* settle it

A null result without the search that produced it is useless, because nobody
can tell it apart from not having looked.

## Reading a fact back

Documentation and code routinely disagree, and for the questions this project
asks they disagree often. Before reporting a capability, one of these is
usually true and cheap to establish:

- a doc statement with no corresponding code, test, or build-matrix entry
- a config option, interface, or tier with no consumer
- a claim that is settled in prose but was never tagged when it was first made

The last one is the common case in this repository. A tagged, dated finding
compounds; an untagged assertion in a document has to be re-derived by every
agent that reads it, and they will derive it differently.
