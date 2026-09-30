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

## 2026-09-28 — Backgrounding a long-running process without redirecting output

The developer agent ran `setsid cargo run --bin ui_demo &` to verify the demo
launched. The background process inherited the shell's stdout/stderr, and since
`ui_demo` is a GUI app that runs indefinitely, those file descriptors never
closed. The bash tool waits for all output to close before returning, so the
command hung forever.

**Rule:** when backgrounding a long-running process, always redirect stdout and
stderr to a file (`>/tmp/app.log 2>&1`) and separate `cargo build` from
execution so compilation time does not race with `sleep`.

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

## 2026-09-28 — `pkill -f <pattern>` killing the invoking shell

While stopping background demo processes, the developer ran
`pkill -f ui_demo`. The pattern matched the bash tool's own command line
(which contained the string `ui_demo`), so pkill killed the shell running
the command: the rest of the command never executed, the persistent
session's working directory reset to the workspace root, and the next
command failed with "No such file or directory" for a relative path that
had worked minutes earlier.

**Rule:** never `pkill -f` a pattern that appears in the invoking command
line. Collect PIDs with `pgrep -a` and `kill` them by number, or use
`setsid` + a pidfile from the start.

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

## 2026-09-29 — A deliberate break's backup silently reverted the fix, and the report described the intent

Task 07, round 3. A mutation test proved a fix worked, but a later deliberate
break of the same line left the suite green. Cause: the `cp file /tmp/x.bak`
backup was taken *before* the test was improved, and restored *after* — so the
restore put the pre-improvement file back, dropping the edit that made the test
discriminate. The report for the round in between described the test as it was
written, not as it stood in the tree, and the reviewer found the divergence by
running the same mutation.

**Rule:** a deliberate break is a three-step ritual — snapshot, mutate, **diff
the restore against the snapshot you intend to keep**, and re-read the file that
was mutated before reporting on it. Never report a file's intended state; read
its state. If a backup is taken before an edit and restored after, the edit is
gone, and nothing but a fresh `grep` will say so.

## 2026-09-29 — The same backup trap, one task later

Task 09 repeated the entry above exactly: a whole-file snapshot taken before the
demo's dirty-queue test was written was restored after the test existed, and
the restore deleted the test. It survived one green suite — the suite that ran
during the restore simply did not contain the test yet.

**Rule:** refresh the snapshot *immediately before each* mutation, not once per
session. After every restore, `grep` for the symbols that were added since the
snapshot was taken; a test count is not enough, because the count is printed by
the build that no longer has them.

## 2026-09-29 — Expectations remembered instead of derived

Six spring, bounce and composition tests failed on their first run. Every one
asserted something about the curve that was plausible and wrong: that a spring
oscillates past *zero* (it oscillates about its target, so it swings between
0.72 and 1.53 and never below 0), and that a bounce overshoots (it touches its
target and comes back, never passing it).

**Rule:** write down the closed form and get the number from it before writing
the assertion. When a test fails on its first run, the expectation is the
suspect, not the code.

## 2026-09-29 — An assertion that cannot fail, in a test named for what it checks

`a_color_component_is_clamped_rather_than_wrapped` checked
`value.r == value.r.clamp(0, 255)` on a `u8` — true for every value the type
can hold. The test passed through a mutation that replaced the clamp with a
wrap, and was only noticed when the mutation check was run.

**Rule:** an assertion that cannot fail for reasons of type cannot test
anything. The only way to know is to run the mutation it was written to catch.


## 2026-09-30 — A buffer sized for one vertex per quad

Both `ensure_vertex_capacity` and `ensure_text_vertex_capacity` allocated
`capacity * size_of::<Vertex>()` bytes for a capacity counted in *quads*. A
quad is four vertices, so every buffer was a quarter of the size it promised.
`glBufferSubData` then failed with `GL_INVALID_VALUE` and the batch was
silently dropped: the text pipeline drew nothing at all, while the solid
pipeline kept drawing because the demo had fewer quads than the slack allowed.

Nothing in the test suite could see it, because the size arithmetic is GL-side
and there is no GL context in a test. The only reason it was found is that the
window was screenshotted and looked empty. The first guess — that the crop was
wrong, then that the labels were not painted, then that the ellipsis glyph was
missing from the font — was each wrong; `gl.get_error()` after each call named
the failing one immediately.

**Rule:** count a buffer's contents in the unit its capacity is counted in, and
when a GL call is added, read `gl.get_error()` after it once to find out which
call is unhappy. A rendering change is not verified until the pixels have been
looked at.

## 2026-09-30 — A strength clamped to 0..=1, used directly as an effect's size

`Button::paint` derived a press overlay's alpha from the button's scale —
`((1.0 - scale) / (1.0 - PRESSED_SCALE)).clamp(0.0, 1.0)` — and passed it
straight to a colour interpolator. A full press therefore meant a **fully
opaque black** rectangle inset 2 px into the background: a pressed button was a
black box, and its label was drawn on top in the theme's `OnPrimary`, which is
black in the dark theme, so the label vanished too. The task asked for a
"slight inner shadow".

46 unit tests passed throughout, and they could not have caught it. Every one of
them asserts on the *recorded draw commands*, and a rounded rect of near-black at
alpha 71 and the same rect at alpha 255 are the same shape and the same colour;
"is it black and is it inset" is the whole of what a draw-command assertion can
ask. The number that distinguishes them is the one thing not asserted on.

**Rule:** a quantity clamped to `0.0..=1.0` is a *position in a range*, not a
magnitude. "How pressed" and "how opaque" are different quantities, and scaling
one by the other needs a ceiling written down as a named constant with a reason.
When an effect has a word like *slight* or *subtle* in its spec, assert the
number that word fixes — here `shadow.a < 128` — because a shape assertion will
not.

## 2026-09-30 — A filled rounded rectangle is not an outline

`Slider::paint` drew its focus ring as a rounded rectangle grown around the
slider's whole rect, and every draw-command assertion in the module called it
correct: a real rect, the right colour, recorded first, with the track and the
thumb after it. But `DrawCommand::RoundedRect` *fills* its rect, and a slider has
no background of its own to draw over the middle of one — so on screen a focused
slider was a white card with a track lying on it, 240 by 44 of it. The `Button`
never hits this because it draws its background over its own ring; a slider has
nothing to draw with.

The capture is what found it. The unit test that covers it was asserting
`grow(rect, ring)` — the right shape, the right colour, the right place, and a
filled rectangle rather than an outline, which is a difference no
draw-command assertion can express.

*How that capture was taken, since it is the evidence the entry rests on:* no
injected event reached the app in either the developer's session or the reviewer's
— see `doc/ui/IMPLEMENTATION_STATE.md` § *Verifying a change that draws* — so the
focused state was produced by a build that wrote the widget's `focused` property
directly at construction, from an environment variable, in a temporary seed that
has since been reverted. The seed is quoted in that section and is reproducible.
That is a real capture of the real widget and it found a real defect; what it is
**not** is evidence that `Tab` focuses a slider on screen.

**Rule:** a filled primitive only reads as an outline if something is drawn over
its middle. Decide what that something is *before* drawing the outline, and assert
on the pair — the grown shape **and** the fact that the shape covering it is
recorded after it — because the shape alone is satisfied by a filled rectangle of
exactly the right size, colour and place. This is the third defect in this
repository found only by looking at the screen, after the undersized vertex
buffer and the opaque press overlay, and the second that no draw-command
assertion could see.

## 2026-09-30 — A rect's origin and a rect's extent are different numbers

`Slider::travel` computed how far the thumb could move as
`extent - origin - radius * 2`, where `extent` was already a *length*. Every unit
test in the module laid its slider out at `(0, 0)`, where subtracting the origin
subtracts nothing, so all 53 of them passed — while every slider drawn anywhere
else in a window reported a negative run, pinned its two ends to its own centre,
and put every pointer position at the minimum. The demo found it within an hour of
landing the widget, because the demo's slider is at `(664, 496)` and nothing else
in the repository is at the origin.

**Rule:** a geometry fixture at the origin cannot see an origin being read as an
extent. Give a geometry test suite one rect that is *not* at the origin, and when a
fixture is easy to place at `(0, 0)`, place it somewhere else instead. The same
applies to any two same-typed numbers that are not the same number — a size and an
origin, a width and a left edge, a count and an index.

## 2026-09-30 — A capture whose only route was instrumented

The task 14 record said the slider was "seen on screen at 0, 25 and 70". The
captures were real and the seed that produced two of them was reverted — but the
record did not say so, and the demo's own keys write **0 and 100**, so they could
not have produced 25 and 70. Those two came from a rebuilt binary whose
`Demo::new` read a value and a focus out of the environment, because no key and
no click had reached the app and the demo had no other route to either. A reader
therefore took pixels for the demo's input, and *A filled rounded rectangle is
not an outline* rests on one of them.

**Rule:** when a capture needed a seed, a rebuild or any other instrument, name it
in the record that cites it, and check the method's own arithmetic against the
numbers claimed *before* writing either down — the same class of error as task
12's withdrawn tap waiver, where two timestamps read in the wrong unit turned a
real defect into a tooling excuse.

## 2026-09-30 — A still screenshot of a 4 fps application looks exactly like a 60 fps one

Task 11 added `font.rs`, and `Font::advance` called FreeType's `load_char` once
per character, per label, per frame — about 61 µs each. The demo re-lays its
labels every frame, so a frame cost ~200 ms and the whole application ran at
about 4 frames per second at 91% of a core. It survived **three reviews and four
captures** because every capture method in `IMPLEMENTATION_STATE.md` is a single
still, and a still of a 4 fps application is pixel-identical to a still of a
60 fps one. No test could see it either: `AGENTS.md` forbids wall-clock tests, so
there was nothing to fail.

The tell that was available the whole time and was read as a tooling problem
three times: the window existed, the pixels were right, and *nothing responded*.
Reviewers concluded "XTEST injection delivered no event" and recorded a waiver.
The app was not ignoring input — it was too slow to answer it. Two of the three
reviewers had a working control in front of them (task 12's button, verified on
screen twice) that was *equally* dead, which is what a machine-level problem
looks like, and it was read as evidence about the injector.

**Rule:** a still proves what is drawn, never how fast it is drawn. When a
change touches a per-frame cost, measure the rate (`/proc/<pid>/stat` utime
over a fixed interval, or frames counted in the loop) and state it, because
"it looks right" and "it responds" are different claims and only the second one
is what a user notices. And bisect before theorising: `git worktree` at the last
known-good commit and compare the same measurement across the three commits
around the suspect change — here task 10 read 5/300 jiffies and task 11 read
272/300, which named the commit in one step. When a report says "input did not
arrive", check `XTestFakeMotionEvent` actually moves the pointer before
believing it; on this host it does not, and a click sent to a pointer that never
moved lands outside the window and proves nothing.
