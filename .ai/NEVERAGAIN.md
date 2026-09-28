# Never again

Failure modes of AI work in this repository, recorded once they have actually
happened. Not a list of things to avoid in theory — a list of things that were
observed, dated, with the fix.

**Written by `developer.md`**, whenever a fix is made because an agent got
something wrong. Also by the operator, whenever an agent's output is rejected.
If you cannot name the mistake, it does not belong here.

Each entry is a trap, the concrete failure it caused, and the rule that
replaces it. Keep entries short. A long entry is a rule nobody reads twice.

---

## 2026-09-28 — Asserting a window opened, from a snapshot of a dead process

Task 02's acceptance criterion is that `cargo run` opens a window. The demo was
launched with `nohup … &` from a tool call that then exceeded its timeout; the
tool killed the process group, so the window was gone a second later. The
evidence — an `xwininfo` line naming the window — was real when it was taken and
useless afterwards, and it would have been quoted as proof.

**Rule:** anything that has to stay alive across tool calls is launched with
`setsid` and checked with `pgrep` in the *same* call as the observation. A GUI
claim is only evidence if the owning process was alive when the window was seen.

## 2026-09-28 — "The window opened" as evidence for a vendored build

Task 02 was reviewed as "fix first" for running the host's SDL 3.5.0 instead of
the 3.4.16 it builds. The handoff had quoted an `xwininfo` line and called the
acceptance criterion verified, without ever establishing *which* `libSDL3.so.0`
opened the window. The gap is not a lie — `cargo run` does load the vendored
library, because Cargo puts the crate's native link-search directory on
`LD_LIBRARY_PATH` — but the same binary executed directly resolves
`/usr/local/lib/libSDL3.so.0`, because `sdl3-sys` emits a plain
`cargo::rustc-link-lib=SDL3` and no rpath. The claim was true of the command
and unverified for the reason that mattered.

**Rule:** on a `build-from-source` dependency, a passing run proves nothing
about which library served it. Check `/proc/<pid>/maps` or `LD_DEBUG=libs` for
the file that was actually mapped, and say which invocation was measured.

## 2026-09-27 — Instructions pointing at files that do not exist

`AGENTS.md` referred to eight artifacts that had never been written:
`.ai/NEVERAGAIN.md`, `.ai/workflows/`, `.ai/skills/`, `.ai/schemas/`,
`architect.md`, `developer.md`, `reviewer.md`, `doc/architecture/overview.md`.

**The failure is silent.** A dangling path is not an error; the agent either
invents plausible contents for it or drops the instruction and proceeds. Both
look like compliance in the output.

**Rule:** never assume the contents of a referenced file — read it, and if it
is absent, say so rather than proceeding. In a document, every reference
resolves to a real file or is listed explicitly as not yet written.

## 2026-09-27 — Two documents each claiming ownership of one definition

`idea-evaluator.md` said the confidence legend was canonically defined in
`researcher.md`; `researcher.md` said it owned the legend and that the
definition travelled with it. A circular reference: each deferred to the other,
and they agreed only because nobody had edited either one yet. The first edit
would have made them disagree silently.

**Rule:** a definition lives in exactly one place. Everything else cites it.
Where two files both claim it, one of them is wrong and the conflict is a bug,
not a stylistic preference.

## 2026-09-27 — Naming a subagent that the harness does not provide

Both agent files described a `scout` subagent, hedged that "availability varies
by harness", and gave a fallback instructing the agent to "clone into the
cache" — a cache convention defined nowhere. The hedge was copied from another
harness and never checked against this one.

**Rule:** state the actual roster. If a tool is unavailable, do not write
contingency prose for it; write what exists. Every instruction must be
executable here, today, or it is decoration.

## 2026-09-27 — An output template that omits the document's own deliverable

`idea-evaluator.md` stated that the rewritten idea paragraph "is the
deliverable", and listed its absence as a failure mode — and its output
template had no field for it, and no verdict field either. The template is what
an agent fills in, so the missing field would have produced reports that
technically complied and were missing the point.

**Rule:** when a document declares something mandatory, the template carries a
slot for it. Check the prose and the template against each other before
believing either.

## 2026-09-27 — Evidence rules defined nowhere they can be found

Both agents tagged every claim `[A]/[B]/[C]` and both told the reader to consult
"prior findings, wherever they live" — with no such place, and with a rule
forbidding file writes. The confidence system was fully specified and entirely
unable to compound.

**Rule:** a mechanism that only produces value over time needs a storage
convention in the same change that introduces it.
