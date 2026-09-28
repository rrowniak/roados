# Findings

Durable research output. One finding per file: a single falsifiable question,
answered, with the evidence attached and dated.

The format is defined in `.ai/schemas/finding.md`. The confidence tags are
defined in `.ai/protocols/evidence.md`. Read both before writing a file here.

**Before researching anything, check this directory.** A finding that already
answers the question is cheaper than a subagent, and a `[A]` finding inside
its staleness horizon is not to be redone. A finding whose `read` date is
older than its `decay` horizon is re-verified, not trusted.

**When research reaches `[A]` on a finding that feeds a live decision, ask the
operator once whether to write it here**, and on yes, write it. The question
is asked once per round, after synthesis — not per finding, mid-flight.
