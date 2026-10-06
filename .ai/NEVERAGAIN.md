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

## 2026-09-30 — A distance field has to binarize, and the bit it drops is the edge

Text rendered through a signed distance field looked "scanned", with stair-stepped
diagonals and hard aliased edges. The field itself was innocent: measured against
brute force, the two-pass chamfer transform was accurate to **0.008 px mean,
0.25 px max** on real glyphs. The defect was one line above it —
`bitmap.pixels.iter().map(|&p| p > 127)` — because a distance field is built by
deciding what is *inside*, and that decision throws away the sub-pixel edge
position FreeType had already rasterized. The field's 50% contour then lands on
whole pixels, and an edge on a pixel boundary has no half-covered pixel to sit
in. The stem edge measured `18 18 18 18 255 255`: a hard step from background to
solid with nothing between.

Two fixes were tried and both made it **worse**, and the measurements said so
before the screenshots did: supersampling the threshold, then averaging the field
back down, dilated the glyph and left a halo (background beside the stem went
`18` → `43`, ink `3.26M` → `3.50M`); sampling the supersampled field at pixel
centres instead of averaging gave the same halo. The field was measuring distance
to a mask that had itself grown. The fix that worked was to stop building a field
and carry FreeType's coverage through unchanged — it was already antialiased, at
exactly the size the glyph is drawn, and the field was rebuilding a worse
approximation of it.

**Rule:** a transformation that quantizes cannot be repaired downstream, and the
repair is usually worse than the original — measure the halo, not the intent.
When a pipeline throws away a producer's output and reconstructs it, check
whether the reconstruction is actually more faithful before defending it. And
size a suspected component against a brute-force reference before rewriting it:
two of the three hypotheses here were wrong (the per-call `set_pixel_sizes`
resize, then the chamfer approximation), and only a reference measurement
separated them from the one that was right.

## 2026-09-30 — A deleted `#[test]` attribute is a green suite with a hole in it

Inserting a test by anchoring on a `fn` line rather than on its `#[test]`
attribute leaves the attribute attached to the **new** function and the old one
unregistered. Clippy caught it as `dead_code`, but only after the edit had been
verified once and the count reconciled wrongly: 97 tests became 96, and 96 was
also the count before the new test, so the two errors cancelled. The suite was
green at every step. A *second* symptom pointed at the same cause and was
misread: every test name printed twice, which looked like the library and the
binary both running.

**Rule:** after any edit that adds, moves or removes a test, check the test
**count** against what it was, and treat a name printing twice as a duplicated
registration rather than as two binaries. Anchor new tests on the attribute and
the signature together, never on the signature alone. And a count that goes down
while you are adding something is not a rounding error — it is a test that
stopped running.

## 2026-09-30 — A mutation runner whose reporting pipe is `head` never restores

A deliberate-break loop piped its output through `head`. `head` closed the pipe
once it had read enough lines, the script took `SIGPIPE`, and the process died
**before the restore step**. The next three mutations were therefore applied on
top of the un-restored previous one, and their failure counts described a file
three breaks deep rather than the break being measured. It was caught by
`diff`-ing the file against its pristine snapshot, not by the runner, which
reported those runs as ordinary results.

**Rule:** a mutation runner must not pipe its own output through anything that
can close the pipe — redirect to a file and read the file. A run that dies of
`SIGPIPE` has performed no restore, and **the exit status of the runner is not
evidence that the tree is back**: verify with `diff` against a snapshot taken
immediately before, and re-read the symbols added since. This is the same defect
as the two backup-trap entries above, with a new mechanism — the reporter, not
the backup.

**Second mechanism, found the same day: the replacement text did not match.**
A mutation written as a string replace against a one-line `if handle == list {
rect } else { None }` was a silent no-op, because `cargo fmt` had already
expanded it to five lines. The runner reported it as a survivor, which is
worse than useless — it looks like a weak test and sends the next reader to
strengthen an assertion that was never exercised. The runner now **aborts when
the file it is about to mutate is unchanged**, and says so, rather than
reporting the run. A mutation that did not apply is not a survivor; it is no
result at all.

**Third mechanism, 2026-10-01: a runner that reports a run which never happened,
and a restore that is not on a trap.** A rebuilt runner aborted with *"cargo
never ran any test — NOT a result"* once — good — but only because a guard was
added after the fact; before that it printed an ordinary-looking line for a run
where `cargo` had rejected its own arguments and run nothing. In the same session
a `set -u` abort on an unbound positional parameter fired **between the mutation
and the restore**, so the next command's mutation landed on top of an un-restored
one and `grep` afterwards found `self.thickness = SCROLLBAR_THICKNESS` — a
mutation — sitting in a file whose real body clamps with `.max(0.0)`. It was
caught by `diff` against a snapshot taken *before* any mutation, which is why
that snapshot is the one to take.

**Rule:** two more, on top of the existing ones. **Assert that the run ran** —
`grep -q '^test result'` on the log, and treat a log without one as an abort, not
a survivor, because a filter argument typo and a weak test print the same
nothing. And **put the restore on `trap ... EXIT INT TERM`**, so it cannot be
skipped by an error path; the `diff` afterwards still has to be run, because a
trap proves the restore was attempted and not that it landed. Take the pristine
snapshot before the *first* mutation of a session, not before each one — the
per-mutation snapshot protects the next mutation, and only a from-session-start
one proves the tree was ever clean.

**Fourth mechanism, 2026-10-01: a failure count parsed off the whole log.**
`grep -o '[0-9]* failed' | head -1` — a runner's own idea of how to count
failures — matched a **bare `" failed"`** in cargo's `error: test failed, to
rerun pass …` trailer before it ever reached `4 failed`. `head -1` took the
empty-number match, `cut -d' ' -f1` produced nothing, and `${failed:-0}` turned
that into zero. Every mutation was reported **SURVIVED** while the log showed
**4 failed**. Three mutations in a row were declared weak tests on the strength
of a parse bug; had the runner been believed, the next reader would have gone
to strengthen assertions that were never weak — the exact harm the third
mechanism was written about.

The same session had already shipped the `grep -q '^test result'` guard and the
`trap`, and **the guard did not catch it**, because the log *did* contain a
`test result:` line — a `FAILED` one. Asserting that a line exists is not
asserting what it says. Parse it: `sed -n 's/.*[^0-9]\([0-9]\+\) failed.*/\1/p'`
against the `test result:` line specifically.

**Rule:** a guard that checks *presence* is not a guard on *content*. When a
runner's verdict is parsed out of a log, read the field it means and nothing
else, and make the parser's failure mode **loud** — an unparseable log must
abort, never default to "0 failures", because "0" is the one value that turns a
broken runner into a confident report that the code is untested.

**Extended 2026-10-06 (task 32.1): the log was missing, and "missing" parsed as
"0 failed" — and a guard that fired was outvoted below it.** The first
deliberate-break sweep wrote each run's log to the wrong path. `grep` on a
missing file exits non-zero, so the failure count came back empty, and `awk
'{s+=$1} END {print s+0}'` over no input prints `0` — so **six rows printed
`SURVIVED` and none of them had survived.** Six false results in the direction
that makes the work look finished. The same sweep's second defect fired on three
of the six rows at once: rustfmt had wrapped the search anchor across two lines
since the row was written, so the guard printed `ANCHOR NOT FOUND — not a
result` **and the runner printed `SURVIVED` anyway**, because the missing log
defaulted to zero below the guard. Two contradictory verdicts on one row, and
the one that survived on the page was the false one.

**The rule is the same one, from the other direction.** *A mutation the runner
refused to apply was recorded as a kill* and *A guard that reported why only
when the why was the easy one* are the two neighbours, and all three mechanisms
are one bug: **a runner that cannot prove a result must not print one.** A guard
that fires and is then outvoted by a default is not a guard; a count that
defaults to zero on an empty input is not a count. The fix is the one above —
parse the field, and abort on an unparseable log — plus this: **the abort must
sit on the path that produces the verdict, not on a guard above it**, and an
empty input to a summing parser is an error, not zero.

## 2026-10-01 — A drawn control with nothing behind it

The operator reported the list's scrollbar two ways — *"is too narrow, I have
issues with pointing on it with my mouse, so doing that on tablet with a finger is
impossible"* and *"when I click it and drag - it doesn't follow my mouse cursor
exactly, it's like something was keeping it from moving faster"* — and the
second half was not a feel problem at all: **there was no thumb dragging in the
code.** `Scroll::on_event` handled every `Drag` by scrolling the content by its
delta wherever the pointer was, so the scrollbar was drawn geometry and nothing
else. The thumb travelled a run of 252 against a maximum offset of 2 520, so it
moved **a tenth** of the distance the cursor did — the "something keeping it from
moving faster" was the ratio, and no tuning of that drag could have fixed it
because the drag was the wrong mapping.

Every test in the module was green. All of them asserted on recorded draw
commands or on the offset after an event, and **not one asked whether the control
could be operated at all**: a widget that draws a thumb and has no code path that
reads the thumb is indistinguishable from one that works, to a suite that only
looks at what was recorded.

**Rule:** for every control drawn, name the gesture that operates it and assert
that the gesture moves it — a test that presses where the control is drawn, not
one that presses on a fixture near it. And when a control's geometry is drawn
from one number (a thickness, a radius, a thickness of a groove), assert that
**hit testing and drawing read the same number**, because they are two consumers
of one constant and a change to it that misses one of them produces a target
nobody can see or a hit area nothing is drawn in.

## 2026-10-01 — One sibling got the operator's fix; the other with the same constant did not

On 2026-09-30 the operator called the slider's 6-pixel track unusable with a
finger, and `ui_demo` answered it by asking for **12** through the widget's
existing `set_track_thickness`. A day later the operator reported the same thing
about the **scrollbar**, whose default was also 6 — and there the answer did not
exist, because **`Scroll` had no thickness setter at all**. The demo had nothing
to call. The report was true, it was predictable from the previous round's own
numbers, and the previous round had written them down.

**Rule:** when the operator rejects a **sizing constant**, grep every widget for
that constant before declaring the fix done, and ask which of them is reachable
by a setter. A fix that only reaches the widget in front of you is half a fix,
and the other half fails later, in front of the same person, and costs a round.
A rejected number is a property of the **device and the finger**, not of the
control that happened to be on screen — so the next control with the same number
is the same defect, already reported.

## 2026-09-30 — A draw-command assertion cannot see where a command *lands*

The operator reported the list jumping about while scrolling. Every one of the
list's 88 unit tests passed, and so did the demo's, because the list was
recording exactly the right commands: rows are laid out in the content's own
coordinates and drawn at `viewport.y + index * item_height - offset`, so at any
offset that is not a whole number of rows **the top and bottom rows are drawn
outside the viewport by design** — that is what makes a scroll smooth rather
than a row popping in. And nothing clipped them. A row's text was drawn on top
of the window background above the panel, and then vanished as it scrolled
away.

Nothing in the suite could see it. A draw-command assertion asks *what was
recorded*; every assertion in this repository asks exactly that. The defect was
not in the commands and could not have been fixed there.

It could not be fixed in the widget either, and the reason is worth keeping: a
`DrawCommand::Text` carries an `x`, a `y` and a string and **no width**, so
nothing outside the text pipeline can tell how far a run reaches, and
`scroll::clip_commands` can therefore only drop a command that is *wholly*
outside. Half a row needs the GPU.

**Rule:** assert on where a command lands, not only that it was recorded, and
expect to need a scissor for anything that is meant to cross a boundary. A
widget whose content is *designed* to overflow its own box — a scrolling
viewport, a marquee, a shadow — is drawing something no recorded-command
assertion can see. When a widget's geometry deliberately puts a primitive
outside its bounds, that is the moment to ask who clips it, and the answer
cannot be "the test suite is green".

## 2026-09-30 — A test of a helper cannot see a call site that stopped using it

The regression test for the clipping defect called `Demo::clip_for(..)` directly
and was green. A mutation that made `clip_for` return `None` was caught. A
mutation that **inlined the same logic into the frame loop instead**, so the
loop stopped calling the helper and inverted the rule — clips everything *except*
the list — passed every test written against the helper.

The fix was structural rather than another assertion: the frame loop and the
tests now both read one function, `Demo::frame_clips`, which returns a clip for
every node in paint order. There is no second place to put the logic, so there
is nothing to bypass.

**Rule:** when a test exercises a helper, ask what a caller could do *instead of*
calling it. A helper that is called from one place and tested from another has
two places to be wrong, and only one of them is under test. Put the decision in
a function the production loop also calls, and test that.

## 2026-10-01 — Two captures of a moving number can be identical

The frame-rate readout landed, and the first check of it was two `magick import`
captures of the window two seconds apart with `magick compare -metric AE` — which
came back **0**. That reads as "the readout is not updating", and it is the
reading a session will stop on: the number on screen is the whole point of the
change, so an unchanged picture is the one thing that would say it is broken. It
was updating. Six captures half a second apart read `fps 50, avg 52.8`, `51,
52.5`, `50, 52.1`, `51, 52.0`, **`51, 52.0`**, `50, 51.6` — a formatted average
lands on the same tenth twice about as often as it moves, so *consecutive* samples
are routinely the same string and the pair matches.

The process was verified as alive, and its own report was read, so the reading was
safe to dismiss — but only because something else in the session had already
established it. That is luck, not method.

**Rule:** a `metric AE` of 0 on a window that is supposed to be *changing* is
ambiguous between "nothing happened" and "the same value twice", and the two need
different responses. Sample **five or six times** and compare the set of readings,
not a pair; and when the thing being watched is a number, read the number rather
than the pixels. This is the `a still proves what is drawn, never how fast it is
drawn` entry with the other half of the trap: a still cannot tell you a counter
is frozen either, and a counter's format is what decides the two apart.

## 2026-10-01 — A brief's rationale becomes the widget's doc comment, and nobody re-checks it

Task 20's gauge draws its arc as a band. There were three ways to draw one, and
the integrator **chose one and wrote the reason into the subagent's brief**:

*"a `Path` shows notches on the outside of the curve, and overlapping circles have
no notch because every circle is round."*

The subagent implemented it, wrote a module doc repeating the claim as settled
fact, and shipped 73 green tests. The demo subagent then took a screenshot,
because that is what the demo sub-agent is for, and **measured the outer edge
along rays: 100.0 px at every circle centre against 94.4 px at every bisector — a
5.6 px scallop on a 14 px band.** A visibly beaded dial.

Both halves of the rationale were wrong. Circles tangent on their **centre lines**
have outer edges that touch only where `R >> r`, and `r/R` was 0.075. And the
`Path` was worse for a reason the brief never mentioned: `line_quad` offsets each
segment **perpendicular**, so its outer corner lands at `sqrt(R² + r²)` rather
than `R + r` — the band is ~6.7 px too thin *everywhere*, before any scalloping.
The fix was a third option neither had considered, and it used a primitive that
had landed hours earlier: one convex four-point `Polygon` per segment, corners on
`R ± thickness/2`. **29x** less error, half the primitives.

The 73 tests could not catch it because **every one of them asserted that a
command was recorded, and the defect was in where the recorded commands landed** —
the same trap as the `a draw-command assertion cannot see where a command lands`
entry above, reached from a new direction. And the module doc made it worse: a
confident false claim in the one file every future reader opens is worse than no
claim, because the next agent trusts it and does not measure.

**Rule:** when a brief states *why* an approach was chosen, that rationale is
**unverified input**, not a decision to pass down — and it must never be laundered
into a doc comment as fact. Reproduce the number that justifies the choice before
anyone writes code against it, and if the choice has a visible consequence, put
the measurement of that consequence in the tests. A geometry claim like "tangent
circles have no notch" is one line of arithmetic; check it rather than reason
about it. **And measure a rendered thing in pixels before its doc comment says it
is smooth** — the doc is the claim, and the capture is the evidence.

**Second mechanism, 2026-10-05: the false belief is a recollection of the design
rather than an inherited brief, and it lands in a prose paragraph nobody re-checks.**
Task 24.3's `press_pads_if_unfocused` was given a doc paragraph recording a
shortcut interaction — which page wins, whether the pads press, where the page is
drained — and **all four factual claims in it were wrong while every one read
plausibly**: the row's page does not win, the pads do not press, the property is
written once, and the drain is in `handle_event` rather than `frame`. They were
wrong because the paragraph was written from memory of the design instead of from
the three sites in execution order, and the reviewer's own test contradicted the
comment two thousand lines away. **`cargo test` was green throughout**, because
`ui_demo` has no doctests, `cargo doc --no-deps` cannot see inside
`#[cfg(test)]`, and clippy and rustfmt parse these lines without reading them —
**so for a prose claim about behaviour, the test count is not evidence at all.**
A plausible comment is worse than a missing one, because it removes the reader's
reason to check. **The worked artefact is the corrected paragraph itself, at
`ui/src/ui_demo/src/main.rs:7433`-`7478`: a numbered list of the interaction's
sites in execution order — the key `match`, the routing loop, the drain — each step
naming the call and citing the test that reads the outcome, and the paragraph
saying which of its earlier claims were wrong.** That is a form better than a rule
sentence, because a reviewer can check each step against the line it names. So:
when a doc comment describes an interaction,
sequence the interaction's sites in execution order and cite each one by line, and
when the claims are behavioural, say in the handoff that the test suite does not
cover them.

## 2026-10-02 — A debug build reads as a performance regression

The operator reported *"some performance degradation, sometimes the fps drops to
~30 fps"* against task 20, the gauge. It was not a degradation and the gauge was
not involved: `ui/target/debug/ui_demo` had been rebuilt that morning and
`cargo run` defaults to debug. Measured on the same commit — **debug 32.7 fps,
release 51.2** — and an interleaved CPU comparison against `b4a2db8` (pre-gauge)
gave 136/144/135 jiffies before against 137/131/149 after, indistinguishable.

The interesting part is what the *number* meant. The recorded baseline already
had a debug row — **34.2 fps** — sitting in the same table as the release figures,
so a debug reading was not a mystery, it was a row nobody connected to the number
in front of them. **A performance report names a rate, and a rate is meaningless
without the build it came from.** Six runs of a release build spanning 4.4 fps
was already the standing caveat; the missing half was that the *floor* of 40 was
a release floor and a debug build sits below it by construction.

**Rule:** a frame-rate claim is a claim about **a build**. State the profile with
the rate, and before treating a report as a regression, check the binary's mtime
against the source and run both profiles — `ls -la ui/target/{debug,release}/ui_demo`
settles in one command which one is on screen. And a baseline table that records a
debug number is a trap for the reader who finds the row and not the cause; say so
next to the number.

## 2026-10-02 — A flat wait before the work is a frame-rate ceiling wearing a frame-time costume

The demo loop called `wait_event_timeout(16 ms)` and *then* drew, so a frame was
`16 ms + work` whatever the work was. On this host that is `16 + 3.9 = 19.9 ms`,
**50.2 fps**, for a frame whose own work was 3.9 ms — and an infinitely fast
frame would still have capped it at 62.5 fps. The UI cost **0.18 ms**; the whole
of the 4 ms was GL submission. The number looked like a hardware limit and was
purely an artefact of where the wait sat.

It survived every measurement the repository had, because every one of them
reported an *average*, and 50 is a stable average: the loop was not slow, it was
exactly as slow as it had been written to be. Only a per-phase breakdown — wait /
update / record / submit — shows a term that should not be there, and the frame
budget the wait was supposed to be had become a wait that *preceded* the frame
instead of bounding it.

**Rule:** in a frame loop, **measure the phases, not just the rate.** An average
that has never moved is evidence the loop is doing what it was told, not evidence
it is doing the right thing; and a pacing constant that is applied *before* the
work rather than *around* it converts every future performance improvement into a
smaller number that never reaches the screen. Budget the frame
(`FRAME_BUDGET - work`), never wait a fixed slice and then work. Where such a
constant is fixed rather than read from the display, say which display rate it
assumes — this one hard-codes 60 Hz and no `GL_SetSwapInterval` is ever called, so
it is not vsync-locked and a 30 Hz cluster panel would run it at half refresh.

## 2026-10-02 — On a shared tree, the suite you ran is not your suite

Task 21 ran three subagents at once: one writing `chart.rs`, one rewiring the
demo, one adding MSAA to the GL context. The MSAA agent's `cargo fmt --check`,
`cargo clippy --all-targets --all-features -- -D warnings` and
`cargo test --all-features` were run from `ui/` as the rules say, and **all
three came back red on `chart.rs`** — four clippy `neg_cmp_op_on_partial_ord`
errors, a `never read` field, forty-odd `cargo fmt` diffs and three to seven
doctest failures whose *count changed between runs* because the other agent was
still typing. `ui_core` does not compile while `chart.rs` has those errors, so
the lints for the file that **was** clean were never emitted: the gate produced
no information at all about the change under test.

The two wrong answers were both available and both look like compliance. Report
the red suite as your own failure, or scope `chart.rs` out silently and call the
green one *the* suite. The second is the expensive one: nothing in the output
says the tree was different from the one you were told to verify.

**Rule:** when another agent is working in the same tree, **name the tree the
number came from.** The shape that worked: copy the workspace to `/tmp`, delete
the other agent's file *and its `mod` line* in the copy, `diff` your own files
against the copy to prove they are identical, run the gate there with its own
`CARGO_TARGET_DIR`, and report both — *"green on a copy with `chart.rs` absent,
your files byte-identical; red in the repo on `chart.rs`, which is not mine"* —
and then say which of the two the operator should believe about **their**
tree. Never delete or move another agent's file in the real tree to make your
own gate pass; the file they are writing is the one they will write next.

## 2026-10-02 — A constant named for an axis is not a deduction about a dimension

Task 21's review produced a should-fix finding that was wrong, and the fix author
disagreed with it correctly. `Chart::plot_rect` reserves two gutters:

```rust
let left   = if self.y_labels.get().is_empty() { 0.0 } else { Y_LABEL_GUTTER };
let bottom = if self.x_labels.get().is_empty() { 0.0 } else { X_LABEL_GUTTER };
```

**`X_LABEL_GUTTER` is subtracted from the plot's _height_**, because x labels sit
*below* the plot. The review read it as the gutter that comes off the *width*,
computed a plot of `270 − 18 = 252` where the plot is 270, and reported that the
demo's published pitch (24.5) and its sample ceiling (23) were both wrong. They
were not. Three checks killed it: the constant's own doc ("How far the **bottom**
gutter is"), the widget's recorded paint (the x axis drawn `1000 → 1270`), and the
capture (that axis's row spans 270 px).

The demo's own sentence had invited the misreading, and that half was real:
*"the plot is 270 wide and 432 tall, the difference being the `X_LABEL_GUTTER`'s 18
pixels"* never said **which dimension** the 18 came off. It now says so, and a
test measures both edges out of the widget's recorded paint instead of publishing
a derived number nobody can check in one command.

**Rule:** a name that contains an axis tells you which *labels* a constant is
about, not which *side of a rect* it is subtracted from — read the line that
subtracts it, and where a number is derived, publish the derivation. And **when a
reviewer and an author disagree on arithmetic, settle it by running both** before
either side is written down: a finding that survives as "the author was wrong"
costs a round, and one that is quietly fixed costs the next reader the same
misreading.

## 2026-10-02 — Four documents agreeing is one belief, counted four times

Task 21's chart module stated in **four places** that a non-finite sample only
breaks a line's run, and all four were right. The code did something else.
`draw_series` built its joins for *every* point, so the vertex before a `NaN` asked
`join_at` about a direction of `NaN`, `normal_of` answered `None`, and **a `None`
join is read by the segments on both sides of it** — so `[1, 5, 9, NaN, 3, 7]`
drew **1** segment where its two runs hold 3, `[1, 5, NaN, 3]` and `[NaN, 5, 9]`
drew **0** of 1, `[1, 5, ∞, 3, 7]` drew **0** of 2, and as an `Area` the same data
recorded 3 fill quads and **1** stroke quad: a body drawn across a gap its own
outline was missing.

The four documents — `Chart::data`, `Series::values`, `each_run` and `paint`'s
degenerate-case table — were written from **the same belief about what a gap does**.
They agreed with each other exactly as far as the belief was right and **could not
disagree with each other at all**, so their agreement carried no information. Two
findings in one review round, the second the same shape: `join_at`'s own doc said
`None` came back "only from a segment of no length", which `normal_of`
contradicts — it answers `None` when **either** side has no length, and again for
a full reversal. The file already stated the correct rule three lines away, three
times, and two tests pinned it.

**And the suite had a test for the gap that could not see it.**
`a_nan_reading_breaks_the_run_rather_than_dividing_by_it` used `[0.0, NaN, 10.0]`:
two runs of **one sample**, which contain no segments, so no join arithmetic ran
at all. It passed before and after, and its name says the property that was never
under test.

**Rule:** a property asserted in several documents is **one assumption counted
several times**, not several checks — and when the code behind them shares the
assumption they pass and fail together. Read what the code does, not how many
places agree. When a test claims a property of a degenerate case, **check that the
case is where the property is non-vacuous**: a run of one sample has no segments,
so nothing about joins is exercised. The cheapest thing that would have caught all
of it is a **count through the public API with its control beside it** — "3
polygons where this data holds 3 segments", and the same data without the `NaN` —
and that is one line per case. A count next to a control is what turns a fixture
from a smoke test into a measurement.

## 2026-10-03 — A test filtered to one binary reported a survivor that the whole suite kills

Task 22's review mutated `Dialog::is_drawn` to ignore its transition, ran
`cargo test --lib`, got **1364 passed / 0 failed**, and recorded a **survivor**.
Re-run against the whole suite the same mutation is killed by one assertion in
`ui_demo` (`main.rs:11697`). The ui_core suite was green throughout; the demo's
was the one that failed, and `--lib` had hidden it.

**Rule:** a mutation verdict is a claim about **the suite the failing test is in**,
so run the mutation against **every binary that has tests** and report **every
`test result:` line**, not the last one. This is the *On a shared tree* entry one
level down: a filter argument prints the same nothing as a weak test, and the fix
is the same — parse each line, and if only one binary ran, that is not a result.

## 2026-10-03 — A cache invalidated in the wrong order is a cache that lies

Task 22 added a blurred shadow that composites through an offscreen target.
`bind_default_target` restored the framebuffer and the viewport, and the code
carried a comment saying the clip was "applied once, before all of it". It was
not: `bind_for_write` does `gl.disable(GL_SCISSOR_TEST)`, nothing re-enabled it,
and **the composite — the only pass that puts the shadow on screen — ran
unclipped**. The `blur <= 0` path *was* clipped, because it bound the default
target immediately after `apply_clip`, so the two paths disagreed and only the
unused one was right.

**And the fix has an ordering to it.** The first repair re-applied the clip at
the composite call site, which leaves `apply_clip`'s early-return cache able to
skip a later `set_scissor` as "unchanged". The repair that is actually sound
writes it **through `apply_clip`, after invalidating the cache**.

**Rule:** a cache of GL or driver state is only as good as **every writer** of
that state, and a "I already set that" cache makes a direct write silently
ineffective. When restoring state you did not set, **invalidate the cache first,
then write through the setter** — never past it. And **a comment claiming a state
transition happens "once, up front" is a testable claim**: the only assertion in
the tree read the batch's `clip` field, which is precisely a shape of assertion
that cannot see whether the GPU was ever told.

## 2026-10-03 — `open(path, "w").write(expr)` truncates before `expr` runs

An agent's edit script did `open(path, "w").write(src.replace(start, new + src[end:], 1))`.
**`open(path, "w")` truncates the file the instant it is called — before the
argument is evaluated.** The expression raised `TypeError`, so the file was left
at **0 bytes**. It was recovered from a snapshot of the *previous* round, so an
hour of work was lost and re-applied.

**Rule:** in a script that rewrites a file, **open it for reading first, build the
whole new content in a variable, and only then open for writing** — or write to
`.new` and `mv` it into place. `open(p, "w").write(f(x))` has the argument
evaluated *after* the truncation, which is the opposite of what it looks like. This
is the same family as the three backup-trap entries above with a new mechanism:
not a bad restore, but a **destructive open standing in for a write**.

## 2026-10-03 — A shadow lands on whatever was recorded before it, not on
## whatever comes next

**Nothing shipped wrong here.** Task 23's toast is the first translucent surface
in the repository with opaque content drawn on it, and the order the task file's
reasoning implies — `surface → Shadow → text` — is wrong in a way nothing could
see: a shadow is composited **after everything its own segment recorded**
(`render.rs:2011`), so a shadow between a surface and its content lands **on the
surface**. Had it shipped, a black 0.5 shadow over its own caster's whole
footprint would have drawn the card at 15 instead of 30 — with every draw command
recorded, in the right order, with the right colour, and the picture wrong. The
three operator decisions were taken before any code and the arrangement was
derived from the pipeline's rules, so **this entry is a rule that was applied, not
a defect that was shipped**, and it is here because the mistake is available to
the next agent rather than because it was made here.

The three facts that decide it are all one layer down and none is visible from a
command list: a segment's opaque batches are submitted **before** its translucent
ones, so a translucent surface and an opaque run in one segment are drawn in the
wrong order; the translucent group is submitted **reversed**, so to draw A then B
inside one segment a caller must record **B then A**; and commands sharing a
`BatchKey` **merge**, so two commands that must not swap cannot be separated by
recording order alone. The toast's answer — every colour multiplied by its own
`surface_opacity` so all of them share one group, recorded
`shadow, text, surface, disc` — is in `toast.rs`'s module doc, and
`the_shadow_lands_behind_the_surface_and_the_text_lands_on_it_in_submission`
runs the recorded commands through the **real** `Batcher` and reads the order
they are *submitted* in.

**Rule:** when a draw order is load-bearing, assert it through the batcher and the
renderer's own traversal, not on the recorded list — and when a `Shadow` is the
thing separating two primitives, ask which of the two it is composited over.
`Painter::shadow`'s doc says "record it before the thing casting the shadow",
which is right for an **opaque** caster and silently wrong for a translucent one.

## 2026-10-03 — A paint order computed once does not contain a node created later

Task 23's demo wiring computed `order` once in `Demo::new` — with the comment
"the tree never changes shape, so the order is computed once" — and a toast
raised by a key press is a node that did not exist then. `Toasts::paint_toast`
recorded the card's commands onto that node every frame and **nothing was ever
painted**, because `Demo::frame` walks `order` and the fall-through
`let Some(pad) = … else { continue; }` never saw the node.

The frame loop's fall-through already warns about a node that is in the order with
no arm; this is the same silence one step further out, and neither is visible to a
test that asks the widget what it records. What caught it was a test that asserted
on the **recorded paint of the demo**, which is empty for a node the loop never
touched.

**Rule:** a node created after the paint order is computed has to be appended to
it, at the position its own semantics ask for — and a widget that **creates a
node per instance** (a notification, a row, a popped item) makes that a permanent
obligation rather than a one-off. The widget's own tests cannot catch it; a test
that reads what the frame loop recorded can.

## 2026-10-03 — `cp -p` restores the file's mtime, so cargo never rebuilds

Task 23's review round ran a deliberate-break loop whose restore was
`cp -p "$SNAP" "$FILE"`. **`-p` preserves the snapshot's timestamps**, so the
restored source came back looking untouched to cargo: cargo fingerprints
`mtime`, decided `ui_core` was fresh, and linked the demo against the **rlib the
previous mutation had produced**. **Eight failures came out of mutations that had
nothing to do with what they changed**, and two mutation runs had to be discarded
and repeated from scratch.

Nothing about it looks like a bug when it happens. The file is byte-identical to
the snapshot — `diff` says so — and the suite *is* red, so a runner reports it
as a result: eight clean mutations "survived" for reasons that had nothing to do
with their assertions. The tell is that a mutation of `toast.rs` fails tests in
`ui_demo`, and the only way that happens with a fresh `ui_core` is a stale
artifact. `.ai/NEVERAGAIN.md` already has four backup-trap entries — a backup
taken before an edit and restored after it, a snapshot refreshed per session
rather than per mutation, a `SIGPIPE` from `head` killing the restore, and
`open(path, "w")` truncating before its argument is evaluated — and **this is the
same family with a new mechanism: not a bad restore but a restore that preserves
metadata and thereby lies to the build cache.**

**Rule:** a mutation runner's restore must make the tree look *newer* than the
build, not older. **`cp` without `-p` is the right restore**; if a snapshot has to
be taken with metadata preserved for some reason, `touch` the restored file
afterwards. And **when a mutation in one crate fails tests in another, suspect the
runner before the code** — cargo's own fingerprint is the thing that was lied
to. The stronger check, which catches this and a wrong restore alike, is to
`touch` the mutated file *before* building as well: a deliberate break is a
change nobody else made, and nothing needs its old timestamp kept.

## 2026-10-04 — One log path for a loop of runs is one run of evidence, and the
## total it fed cannot be re-derived

Task 23's deliberate-break runner sent every run to a single fixed path —
`run_suite() { (cd ui && cargo test …) >"$LOG" 2>&1; }`, `LOG=/tmp/opencode/mutation.log` —
and all thirteen mutations shared it. **`>` truncates, so each run destroyed the only
record of the one before it.** A second script, `mutate12.sh`, has the same shape and
kept the same shape's damage. **Twelve-plus runs of evidence are gone.** What survived
is two `test result:` triples, and the hand-over's total — sixteen breaks, fifteen
killed, one survivor — was a console summary line (`### $RUN run: $KILLED killed, $SURVIVED
survived, $NOOPS no-ops`) that existed only on a terminal. **The scripts on disk hold
thirteen and one invocations: fourteen against a reported sixteen, and nothing left
settles which two are unaccounted for.** Found on 2026-10-04, when a recount tried to
re-derive that total from the logs it claimed to come from.

Losing the logs is the smaller half. **What remains looks like proof**: two logs with
complete `test result:` lines are indistinguishable from a whole record, so the record
went on citing them beside a total they cannot support — a hand-over whose figures no
file on disk accounts for. **The tell is a total no run on disk adds up to**, and it is
invisible precisely because the surviving logs are real. Two later runners get this
right and are the reason 27 of the triples this task quotes are re-derivable at all:
`mut.sh` and `mut3.sh` write **one log per mutation, named for it**
(`mutlog/m18_no_clock_clear.log`, `mutlog2/r3_from_rest.log`), which is why the single
unrecoverable gap in that record is recorded as a gap instead of being guessed at.

**Rule:** a loop writes **one log path per iteration, named for what produced it** —
`log="$WORK/$NAME.log"` — never a fixed path shared by every run; `>` keeps exactly one
and `>>` produces one file nobody can read. Print each run's three `test result:` lines
into the transcript as well as into the file, so the tally lives somewhere the loop does
not overwrite. And **a reported total is only reportable if every part of it is
re-derivable from a file that still exists** — where one is not, the honest output is
the per-run evidence plus the gap, which is what
`doc/ui/IMPLEMENTATION_STATE.md` § *What was measured and how* now does.

## 2026-10-04 — Hiding a widget makes every test that crosses the boundary vacuous

Task 24.1 gave `ui_demo` pages, and the migration moved 119 of its 164 tests. Nineteen
of those changed by more than a fixture line, and the split is **6 + 5 + 2 + 6 = 19**:

- **six** were the `GALLERY_SHORTCUTS` tuple gaining a page field, so a destructuring
  pattern's arity changed and nothing else:
  `no_printable_key_acts_without_a_row_in_the_shortcut_table`,
  `the_gallery_shortcut_list_holds_every_key_the_table_has`,
  `a_gallery_shortcut_acts_again_once_the_dialog_is_closed`,
  `a_gallery_shortcut_does_nothing_while_the_dialog_is_showing`,
  `a_toast_is_raised_by_its_own_key_in_both_states`,
  `the_two_plus_keys_are_both_rows_and_do_the_same_thing`;
- **five** were focus walks that lost their one five-stop order, because the five
  focusables now sit on three pages and no page has all of them:
  `tab_walks_every_focusable_control_in_order_and_wraps` and
  `shift_tab_walks_the_same_order_backwards` (both rewritten to walk every page's own
  written-out order), `the_two_widgets_with_no_focus_state_say_where_focus_is`
  (one demo per widget's page), `a_key_switches_the_toggle_once_it_holds_focus` (its
  `Tab` count 3 → 2, now `controls`' second stop), and
  `the_dialog_replaces_the_tab_order_while_it_is_showing` (its "closed, the background
  controls are back" half moved to `controls`);
- **two** were not focus walks at all: `the_gauge_is_not_in_the_focus_order` and
  `the_chart_is_not_in_the_focus_order` each changed **two lines** — the fixture's
  page and one extra argument to a helper that already existed — and were moved to
  `controls` because a page with three focusables gives a longer lap for "never lands
  on it" than `data`'s one;
- **six** were tests of a **relationship between two things on different pages**, and
  those are the subset that went *weak*:
  `a_pointer_press_behind_the_scrim_operates_nothing`,
  `a_tap_outside_the_panel_does_not_reach_the_gallery_behind_it`,
  `a_tap_on_a_key_behind_the_scrim_inserts_nothing`,
  `a_tap_inside_a_drawn_toast_reaches_the_control_under_it`,
  `the_demo_never_offers_an_event_outside_the_dialog_while_it_is_showing`,
  `the_demo_has_the_widgets_the_later_tasks_added`.

**An earlier version of this entry said "nine", and it was the one number that could
not be reconciled with anything**: nineteen changed by more than a fixture line,
**thirteen** of them for reasons with nothing to do with pages, and **six** of them
because pages changed what they were testing. Carrying 19, 13 and 9 for one thing is
how the error survived a whole review round — the next reader went looking for nine,
found nineteen, and had no way to tell which set the nine was meant to be.

**Why the last six went weak**, and this is the part the rule below is about. The
paint gate, the `set_visible` gate and the page guards all answer "nothing
happened" for a cross-page relationship, so those assertions went on passing — **for a
second reason that has nothing to do with what they were written to check.** `a_tap_inside_a_drawn_toast_
reaches_the_control_under_it` could not be repaired at all: a toast is on `overlays` and
the keyboard is on `input`, so there is no demo in which a card covers a routed control.

The trap is that the suite was *green* throughout, so there is no failure to notice.
What exposed it was counting rather than running: grouping the tests by subject showed
three dozen whose subject was off the default page while their fixture named it, and
each of those needed reading, not running.

**Rule:** when a change makes part of the tree unreachable, **every test whose subject
crosses that boundary has to be re-read, and one that cannot be expressed any more has
to be replaced rather than kept.** A test that asserts "nothing happened" needs a
**positive** half on the far side of the boundary — the same gesture succeeding with the
boundary elsewhere, or the whole chain compared with and without the hidden thing — or
an assertion about the mechanism that is page-independent (a modal *dismissing* proves it
was reached; a press that did nothing does not). And **say which assertions were
weakened in the hand-over**, because "the suite is green" is exactly what a batch of
quietly vacuous tests looks like.

## 2026-10-04 — A build that reports `Finished in 0.0xs` did not rebuild

Task 24.1's round-2 reviewer built `HEAD`'s `main.rs` into `ui/target` to compare two
captures, then ran `cargo build --release` on the working tree. It printed **`Finished
in 0.03s`** and left HEAD's binary in place — **two source trees sharing one
`CARGO_TARGET_DIR`**, and the second build's fingerprint matched the first's because
nothing it depended on had changed as far as cargo was concerned. Two captures taken
in that window were **HEAD's launch state, not the pages**.

It was caught by the symptom rather than by the log: **`--tab=pads` showed the whole
gallery instead of one page**, which is what a binary that never read the argument
looks like. That is the shape of a run that is not talking to you — the argument was
accepted and ignored, exactly as an injector that delivered nothing was read three
reviews ago as a product defect.

The rule belongs beside the `cp -p` entry above, because it is the same mechanism
(**a build cache lied about the state of the tree**) and the same family as *On a
shared tree, the suite you ran is not your suite*. What is new here is that the lie
was in a **build**, not in a restore, and so it survived `diff` against a pristine
snapshot: the source was right and the binary was not.

**Rule:** before capturing, **force the rebuild and confirm the binary is the one you
built** — `cargo build --release` and read what it printed, and if the time is under
about a second, assume nothing was rebuilt until something has changed. `touch` the
source, or `cargo clean -p <crate>`, and watch the compile happen. **A `Finished in
0.0xs` line is a claim about the cache, not about the tree**, and a capture is evidence
about a binary.

## 2026-10-04 — A sweep of a mechanism's call sites is not a sweep of the data it is
## built from

Task 24.1 gave `ui_demo` six pages, and the mutation sweep that closed task 24.1's
round 3 ran **34 mutations across every call site of all three gates** — every
`sync_page_visibility`, every `empty_off_page_paint`, every page guard, the focus
filter, the shortcut dispatch, the argument parser. **Four findings, one per round,
and every one of them was a gate or a call site with no test.** And then the round-3
reviewer deleted **one line of `Demo::new`**, the `for &handle in &label_nodes` loop
that adds the seven text labels to the page table, and the whole suite stayed green
while the entire text column was drawn on the `pads` page.

The four, in order, because the shape is the lesson and not the instances:

1. **`Demo::raise_toast`'s `page_members.push`** — a node created after the table was
   built, with no row. A card raised by `K` was painted on every page.
2. **`Demo::show_page`'s `sync_page_visibility`** — the same gate, a second call site.
   Every fixture reached it through `Demo::new`'s, so no test went through a *switch*.
3. **`on_show`'s always-painted clause** — defensible, unexercised, argued rather than
   tested (that one was a survivor, not a finding; it is listed because the sweep
   found it and I had to answer for it).
4. **The `page_members` table's own construction** — a row that was never added.

**The rule:** a sweep answers *"is each call site of this mechanism reached by a
test?"*, and a mechanism built from a table is only as complete as the table. **Sweep
the data as well as the call sites** — for a table, that means mutating rows and not
only the code that reads them, and it means an assertion over the *complement* (what is
**not** in the table) rather than one over its members, because a row that is missing
is trivially "not one of the members" and every assertion phrased over membership
passes. `a_page_records_no_command_on_a_node_that_is_not_its_own` is the exact shape:
it asked whether a node with commands was `own || always`, and a missing row is
`always` by definition.

**And all four were found by mutation and none by reading**, across four rounds of a
change whose prose was argued line by line. A gate's *absence* is not something a
reviewer reads off a diff: what a diff shows is the code that exists.

## 2026-10-04 — A guard built from a name and a size, described as a guard on the bytes

The row-deletion sweep's no-op detector hashed each candidate test binary's **filename
and byte length**, and the report called it a byte-level guard that rejects a
semantically neutral mutation before running it. It was neither. Fixed to hash the
binary's bytes, the "no-op" mutation `X || false || Y` was **not** rejected: it
compiled to a different binary and survived, exactly as a textual comparison would have
reported.

Measured, on the two expressions alone:

- **opt-level 0** — identical instruction sequences, differing only in basic-block
  label numbers (`.LBB0_*` vs `.LBB1_*`).
- **opt-level 1** — byte-identical after normalising labels.
- **opt-level ≥ 2** — LLVM merges the two functions into a single symbol, `mutd = base`.

**Rule:** a fingerprint of a compiled artifact must hash the artifact's **bytes**, and
**byte identity is not behavioural identity** — it tracks the optimiser's decisions
about symbols, labels and dedup, so it is not a no-op detector at any opt level. Never
infer "this mutation changed nothing" from "this binary did not change". The sound
substitute is a declared expectation per row plus a differential comparison against the
base build's own failure set, so "0 failed" is read against a **measured baseline** and a
run that disagrees with what the row was supposed to be is the finding. A name-and-size
hash is worse than no guard: a same-length binary collides with the base and hides a
kill.

## 2026-10-04 — One snapshot file asked to be two things, and the round's work was deleted

The mutation harness kept a snapshot and restored from it on exit. In rounds 1–4
the snapshot was the *final* state, so restoring it was right. The operator then asked
for a **start-of-round** snapshot to measure against, and the same file was reused for
that — so the harness's exit handler overwrote the working tree with the state from
**before** the round's eight edits. One `md5sum` found it; all eight edits were lost
and had to be replayed. The abort-if-not-applied guard is what surfaced it, by refusing
to report results for anchors that had stopped matching.

**Rule:** a snapshot used to *measure* a round and a snapshot used to *restore* the
tree answer different questions, and they must be different files — `r5.base.rs` for
"what did this round change", `r5.work.rs` for "put the tree back". They coincided only
as long as the round's fix was also its final state, which is a coincidence with an
expiry date. **Take the measurement snapshot at the start of a round and the restore
snapshot immediately before running any mutation**, and check the restore target's
`md5sum` against the tree after the sweep — which is what the sweep already prints.

## 2026-10-04 — A guard that reported *why* only when the why was the easy one

The same harness reports `NO BINARY CHANGE` when a mutation's compiled binary matches
the base's. That verdict is wrong in two ways that both presented as it, and both
happened here:

- **cargo's output was discarded, so its exit status was never checked.** A mutation
  that fails to compile leaves the *previous* binary as the newest file in `deps/`, and
  an unchanged fingerprint is then read as "this mutation changed nothing".
- **A row whose edits cancel is indistinguishable from a row that changed nothing.**
  A hoisted copy plus a positional deletion does exactly that, because `replace(x, y,
  1)` takes the *first* match — so the deletion removed the copy the insertion had just
  made, the file came out equal to the base, and the binary matched. This row reported
  `NO BINARY CHANGE` twice before it was found.

**Rule:** a verdict must name its own cause, and a negative result needs its own
alternatives excluded rather than assumed away. **Check the build's exit status rather
than discarding its output, and assert that the mutated text differs from the base
before fingerprinting it** — the second check is two lines and it is the one that
separates "the mutation is a no-op" from "my row did nothing".

## 2026-10-04 — A position API that only one parent mode reads

`TASK_UI_PRIM_24.2.md` said, as the reason the card of pads needed work, that it
is *"a `Stack` child with no `set_position`, so it sits at the origin by
default"*, and asked for an explicit `Offset::new(0.0, CONTENT_TOP)`. **`set_position`
on a `Stack` child does nothing**: `arrange_stack` places every child at
`Offset::ZERO` and never reads `position`, and `LayoutState::position`'s own doc
(`layout.rs:835`) says *"Returns the position an `Absolute` parent places this node
at, if it declares one."* — **quoted exactly**, because the second version of this
entry had it as a paraphrase in quotation marks and the third review caught that,
which is the same class of error as the wrong citation above it.
The write compiled, the demo ran, and the card sat at `y: 0.0` — caught only
because a test asserted the strip was empty.

**The first version of this entry cited the wrong proof**, and the review caught
it: `stack_places_every_child_at_the_origin` places two **unpositioned** leaves,
so it shows that a `Stack` puts its children at the origin — not that the setter
is ignored. The claim is proved by reading the two arms against each other:
`arrange_stack` never reads `position`, `arrange_absolute` (`layout.rs:1380`)
does, and `absolute_places_a_child_where_it_asked` shows the setter working.

**Rule:** a setter that a *parent* consumes is not honoured by every parent, and
the compiler will not say so. Before writing a placement, read **the parent's
`LayoutMode` arm** rather than the setter's name — one grep of `arrange_*` settles
it — and where the parent is the wrong mode, ask whether changing the mode is
cheaper than adding a wrapper node. `LayoutMode::Absolute` places an unpositioned
child at the parent's origin exactly as `Stack` does, so for a root whose other
children declare no position the switch is one token and moves nothing else.

## 2026-10-04 — A survivor is a missing assertion, and only a sweep finds it

Task 24.2's shift is written at six sites. The mutation sweep ran all six, and
**dropping `CONTENT_TOP` from the text column's own `set_position` left all 1817
tests green**: the column's labels only ever claimed to be *below the frame-rate
readout*, and 501 is below 748 as surely as 565 is. The other five sites were
each killed by two to eight tests. Nothing about the surviving mutation looked
like a weak test — it looked like a site nobody had asked about.

**Rule:** a surviving mutation is a finding about **the assertions**, and the fix
is an assertion, not a wider sweep. When a change is written at several sites,
the sweep's survivors name the sites nothing holds down, one for one — so run it
even when the change "obviously" moves everything together, and treat a survivor
as a hole in the suite rather than as a fact about the code. Writing the missing
anchor down (a rect's own `y` beside its constant) is usually one line.

## 2026-10-04 — An acceptance criterion that names an instrument which cannot
## produce the evidence is not met by producing the evidence another way

`TASK_UI_PRIM_24.2.md` requirement 6 and its acceptance criterion both say the
frame rate is measured *with `.ai/tools/fps-check.sh` on all six pages*. **The
script cannot select a page**: `.ai/tools/fps-check.sh:71-72` runs
`./target/release/ui_demo` with no `"$@"`, and the demo's page comes from
`--tab=` with no environment variable that reaches it — so the tool can only ever
measure `pads`. The five other pages' numbers in the hand-over are real and were
produced by running the same command with the page named, **which is not the
named instrument**, and forwarding `"$@"` is an `.ai/` change a task may not make.

This has been sitting in the task files since task 24.

**Rule:** an acceptance criterion names **an instrument**, and the evidence has to
come from that instrument — a criterion met by a *different* measurement is met by
nothing, however good the number is. So when a criterion names a tool, **read the
tool's argument handling before quoting its output as the criterion's evidence**,
and when it cannot produce what the criterion asks for, say so plainly in the
hand-over and let the criterion be amended rather than quietly satisfied beside.
The companion is `No evidence by assertion`: a number in a report is not a
measurement until the thing that produced it is the thing that was named.

## 2026-10-04 — A multi-edit patch script that writes once at the end loses every
## edit before the one that failed

Task 24.2's third review found a doc comment still asserting the opposite of the
failure message three lines below it. Both had been edited in the same session,
and the cause was in how the edits were applied, not in either text: the patch
was a Python script holding three replacements, it **`open(p, 'w')`-ed once at the
end**, and its `assert s.count(old) == 1` failed on the **second** edit — so the
script raised, the write never happened, and the **first edit was discarded with
it**. The reduced rerun I then wrote covered edits two and three, printed `ok`, and
I reported all three as done.

**Nothing in the session said so.** The edit that vanished was a doc comment, so
`cargo test` was green, `cargo fmt --check` was clean, and `cargo clippy` had
nothing to say: **`ui_demo` has no doctests and `cargo doc --no-deps` cannot see
inside `#[cfg(test)]`, so for this file clippy and rustfmt are the only gates that
read a doc comment at all** — and neither of them checks whether a doc comment is
*true*. A reviewer comparing the doc against the code found it in one round.

**Rule:** a patch script that makes **several** edits to one file must **write
after each edit**, or **assert every anchor before touching anything** and abort
loudly with the count of edits that would be lost. A bare `ok` from a script that
raised partway through is not a result — and when a rerun is a *reduction* of the
original, **the omitted edits are the ones nobody looks at.** If the text being
edited is a claim rather than code, the check that finds out is a reader, not a
tool: grep the old sentence before reporting the edit done.

The sibling entry — `open(path, "w").write(expr)` truncating before `expr` runs —
is the same hazard at the level of one write. This one is at the level of the
**whole script**: N correct edits, one bad anchor, zero applied.

## 2026-10-05 — A container that covers the window swallows every tap aimed at
## anything behind it

Task 24.3 put a tab bar at the top of the window, as the root's **second** child —
which is what `TASK_UI_PRIM_24.3.md` requirement 1 asks for, *"as the first child
so it paints over the background and under everything else"*. **`hit_test_from`
(`ui_core::input`) walks a node's children in reverse**, because the later child
covers the earlier — and two of the root's other children are boxes that start at
the window's own origin: the controls layer is `tight(WINDOW)` and the text panel is
`tight(TEXT_PANEL)`. So a tap over a bar button came back with the chain
`[controls layer, root]`, `offer_to` answered `false` for both, and **the bar could
not be clicked at all**: no page switch, `pending_page` still `None`, every unit
test green until one asked.

**Moving the bar does not fix it, and that is the part worth keeping.** The two
readers of one child list run in opposite directions — `input::route` and
`Focus::focus_order` walk it backwards and forwards respectively — so attaching the
bar *last* makes it hit-tested first and puts the six buttons **last** in the `Tab`
order. And trimming the controls layer's box cannot help at all: its origin has to
stay at `0, 0` for the offsets inside it to be window coordinates, and **a box that
starts at `y 0` covers the strip whatever its height is**. What was left was a
second hit test, after `input::route` had declined — a fallback rather than a
bypass, so anything a control in the tree consumed never reaches it.

**Rule:** **a full-window (or origin-anchored) grouping node is a hit-test
region, not an invisible one**, and putting a control behind one hides it from the
pointer while leaving it painted. When a new control has to be *clickable* rather
than merely drawn, **print the chain `input::route` returns for a press on it**
before believing the tree is right — `input::route(&nodes, root, &tap)` in a test is
two lines and it is the whole diagnosis. And where two mechanisms read one ordering
in opposite directions, **check that they can both be satisfied before moving
anything**: here the answer was that they could not, which is a fact the operator's
brief did not contain and only the measurement produced.

The sibling is *a drawn control with nothing behind it* in the other direction: a
control with no code path that reads it. This one is a control with a code path that
reads it and a tree that never delivers the event.

## 2026-10-05 — An environment variable the build never asks for fails silently

*(task 33 — `cmake/sdl-options.cmake`; the variables are named in
`CROSSBUILD.md` §4.1.)*

`cmake` 0.1.58 resolves the SDL toolchain file from the environment, in four
steps, and the third one is `format!("{}_{}", kind, var_base)` — so the name is
**`HOST_CMAKE_TOOLCHAIN_FILE` or `TARGET_CMAKE_TOOLCHAIN_FILE`, with the kind
first**. `CROSSBUILD.md` had it as `CMAKE_TARGET_CMAKE_TOOLCHAIN_FILE` and
`CMAKE_HOST_CMAKE_TOOLCHAIN_FILE`; neither is a name the crate looks for. Task 28
inherited the wrong spelling, and my first fix guessed a third one,
`CMAKE_TOOLCHAIN_FILE_HOST` — also wrong, in the opposite direction.

It failed **silently, three times over**, and each layer agreed with the wrong
one:

- `cargo build` succeeded.
- `.cargo/config.toml` § `[env]` did deliver the variable — correctly, and
  `relative = true` resolved it to an absolute path, so *that* half was verifiably
  working and looked like the whole thing.
- SDL configured, compiled 900-odd objects, linked, and produced a binary.
- `SDL_VIDEO_VULKAN`, `SDL_GPU_VULKAN` and `SDL_TEST_LIBRARY` were all still
  **on**. The configuration was SDL's defaults, not the requested one.

What finally settled it was `cmake`'s own diagnostics: `getenv_os` does a
`println!` per probe, so `cargo build -vv` prints all four lookups and their
answers. Reading the code told me the *shape* (`{}_{}`); reading the output told
me the *direction*.

**Rule:** a variable name is not a name until something has read it back. When a
build ignores an environment variable, do not conclude the variable is not
delivered — conclude that delivery and lookup are different claims and prove
each. And when a `format!` composes a name from a discriminator, the argument
order is part of the interface: `"{}_{}"` with `(kind, base)` and
`"{}_{}"` with `(base, kind)` produce two plausible, mutually incompatible
spellings, and only one is ever exercised. Prefer the form the program can print
back to you; if it cannot print, find the substitute that can.

This is the same shape as `A build that reports Finished in 0.0xs did not
rebuild`: **a success signal that is not connected to the thing you changed.**
`cargo clean -p <pkg>` is the reflex when a build's configuration changes and
nothing happens.

## 2026-10-05 — A crate with no rerun-if-changed ignores every file you changed

*(task 33 — `sdl3-sys`; the remedy is in `CROSSBUILD.md` §4.1.1.)*

`sdl3-sys` 0.7.1's `build.rs` emits **no `rerun-if-changed` directive at all**.
Cargo's fallback for a build script with no directives is "re-run if any file in
the *package* changed" — and `cmake/sdl-options.cmake`,
`cmake/aarch64-toolchain.cmake` and `.cargo/config.toml` are all outside the
package. Editing all three and running `cargo build` produced **no rebuild of
SDL and exit status 0**. The two SDL configure trees that mattered were unchanged
and nothing said so.

The trap is specific to configuration that lives outside the package. Rust source
edits are fine, because they are in the package. A build script that reads
`CARGO_MANIFEST_DIR/../../..` has declared a dependency on a file cargo cannot
see, and it has no way to declare it.

**Rule:** after changing anything a build script reads from outside its own
package — a toolchain file, a `.cargo/config.toml`, an env var the crate reads —
**force the reconfigure and verify the artefact, not the exit status**:

```sh
cargo clean -p <pkg>          # then build, then grep the artefact
```

"Build succeeded" is not evidence that the configuration took. The evidence is a
value in the artefact: a cache entry, a generated header, a symbol count.

## 2026-10-05 — An object-tree size is not a binary-size saving

*(task 28 — the near-miss was writing `du` figures into a design document;
the figures that settled it were linked in `CROSSBUILD.md` §5.5.)*

Task 28 measured the vendored SDL build per subsystem with `du` over
`CMakeFiles/SDL3-static.dir/src/<subsystem>` — `render` 1.2 M, `gpu` 708 K,
`audio` 344 K, and so on, against 9.3 M of `src/` objects in total. Those numbers
are real and they are what a subsystem costs **to compile**.

They are not what it costs **to ship**, and the gap is not a rounding error:
`libSDL3.a` was 7 623 342 bytes against 9.3 M of objects, because the linker
drops unreferenced members of a static archive. The measured truth, once both
sides were linked: forcing Vulkan, desktop GL and SDL's test library off took
**803 536 bytes off the archive and 409 352 off the binary** — the same change,
two numbers, neither derivable from the other.

An agent writing this into a design document would have produced a confident
figure that was wrong by half, and `NEVERAGAIN`'s *"a guard built from a name and
a size, described as a guard on the bytes"* is the same mistake one level up: a
measurement of a proxy, described as a measurement of the thing.

**Rule:** name the artifact a number was measured on, in the same sentence —
*object bytes*, *archive bytes*, *binary bytes* — and never let the second stand
in for the third. `du` on a build tree is a **build-time** cost. A shipping cost
needs a link, before and after, on the same machine. If the change has not been
made, the shipping figure is unknown and "unknown" is the answer.

The corollary bit here too: an argument whose whole force is a number the
operator has just rejected is an argument to drop, not to soften. The subsystem
question was settled on the merits — audio and camera are product requirements —
and only then did the sizes get measured, as a footnote to what had already been
decided.

## 2026-10-05 — `tar -x` restores the archived mtime, so a restored tree is older
## than the build and cargo keeps the binary the mutation produced

Task 30's deliberate-break sweep left a mutation in the tree, and the tell was not
in the runner at all. The restore was `tar -xf "$PRISTINE"` on a trap, which is
correct as a restore and **lies to cargo afterwards**: `tar` puts back the mtimes
the archive recorded, and those were taken before the session's first mutation —
so every source file came back *older* than the build a mutation had just
produced. Cargo fingerprinted them as unchanged, kept the mutated rlib, and the
suite reported **two failures against a source file that was provably correct**:
`the text holds U+26A0` failing on a file that contained `\u{26a0}`, and a
character count reading 12 for a string of 18.

**`diff` said the tree was clean and the suite was red, and neither was a lie** —
that is what makes this one new. `.ai/NEVERAGAIN.md` already has the `cp -p`
entry ("a restore that preserves metadata and thereby lies to the build cache"),
and this is that mechanism reached by a different command. **`touch` after the
restore is the fix**, and it is two words, so the cost of leaving them out is a
session's worth of chasing a defect that was not in the tree.

**Rule:** a mutation runner's restore must leave the tree looking **newer** than
any build that ran during the sweep. `cp` without `-p` does it; `tar -x` does
not; so after either, `touch` the files. And **when a test fails, read the source
before believing the test** — a failure that contradicts the file is either a
stale binary or a wrong expectation, and `cargo build` printing what it rebuilt
is what tells the two apart in one command.

## 2026-10-05 — A test fixture that builds what production does not define

Task 30's demo label is switched between two font families by a property, and the
demo's test fixture **defines the second family** so the toggle has something to
toggle to. `main` builds its font set in a different place, and it **did not
define that family** — and `FontSet::family` resolves an unknown name to the
default family *by design*, so `Y` wrote the family the label was already in. It
did nothing. Every test passed, because every test ran against the fixture.

**The capture found it and nothing else could.** The two states were photographed
to show the same sentence drawn two ways, and the two images came out
byte-identical across 71 sampled columns of the label's row — the second was a
photograph of the first. `magick compare -metric AE` reports 1958 differing
pixels over the whole window, which is the fps readout and the padding; the
*label's* row is what is identical, and it is identical because nothing happened.

This is task 24.1's missing-row finding **mirrored**. There, a production row was
absent from a table the tests read and a completeness assertion caught it. Here, a
production **definition** is absent from a set the tests *constructed themselves*,
so the tests and the artefact were built from two different sources and the
assertions agreed with the fixture. The sweep's row
`the-fallback-font-file-is-not-installed` — deleting
`fonts.add_fallback(Font::from_path(FALLBACK_FONT_PATH)?)` from `main` — is the
same defect measured from the other side, and **it survived**, because `main`
opens a window and no test touches it.

**Rule:** when a fixture supplies a value the product is supposed to supply, the
fixture is **a second copy of that decision** and the two can differ without
anything failing. Prefer extracting the construction into a function the tests can
call; where the value needs a file or a window and cannot be, **say so in the
fixture's doc and count the wiring as unverified**, because "the tests pass" then
covers the fixture and nothing else. And **photograph a change and compare the
two states rather than each against nothing**: one capture of a thing that did not
change looks exactly like one capture of a thing that did not change, and only a
pair answers it.

## 2026-10-05 — An unknown-name rule and a mutation of it can be the same value

A deliberate-break row replaced `self.default_family()` with
`FamilyId::default()` in `FontSet::family`, and with
`self.family_id(DEFAULT_FAMILY).unwrap_or(...)` in a second attempt. **Neither is
a mutation.** `FontSet::new` pushes the default family under `DEFAULT_FAMILY` as
its first entry and `default_family()` *is* `FamilyId::default()`, so all three
spellings name the same family. Both edits applied, the binary changed, the
behaviour did not, and the runner's guard — which compares the mutated text
against the base before building, and the binary's bytes — passed both.

This is the *same* failure as *a guard built from a name and a size*, one level up:
**a fingerprint proves the artefact changed and says nothing about whether the
behaviour did.** The remedy that file gives is right and is worth restating
because it was not applied here: **a declared expectation per row**, and a
mutation that is unkillable must be *argued* as unkillable rather than reported
as a survivor. The row that actually tests the rule is
`an-unknown-name-resolves-to-something-else` — resolving an unknown name to a
family that is **not** the default — and it is killed.

**Rule:** when a mutation survives, ask whether it *can* be killed before writing
it down as a weak test. Two ways to spell the same value, a constant and its own
definition, a default and the thing it defaults to — a change between them is a
rename, and a renamed row has tested nothing. The cheap check is to ask what the
edit was *for*, and if the answer is "to see whether this path is reached", the
edit needs to change a **value**, not a spelling.

## 2026-10-05 — `next_power_of_two` is idempotent on a power of two, so "grow to
## the next power of two" is a grow that never happens

Task 31's atlas growth asked `next_power_of_two` for the next power of two above
the atlas's size, exactly as the task file words it — and growth never fired once.
The atlas starts at 2048 and every size it holds after a grow **is** a power of
two, so the answer was always the size it already had: `min(64, max) = 64`, the
`next <= size` guard returned `false`, and `allocate` fell straight through to
eviction — which is precisely the behaviour the task exists to remove, reached
through the code that was supposed to prevent it.

**The suite caught it in one run and it would never have been caught on screen.**
Six tests failed on the first execution, all of them saying the same thing: the
atlas had not grown. Nothing about the demo shows it: 2048² holds about 9 500
glyphs and the demo has 200, so *no run of the product ever grows the atlas at
all* — the growth path is unreachable from a capture unless a build is seeded to
reach it.

**Rule:** a "round up to the next power of two" is not a doubling, and the two
are written differently: ask for the power of two **above `size + 1`**, not above
`size`, because the rounding function is a fixed point at a power of two. This
is the same shape as *an unknown-name rule and a mutation of it can be the same
value*: **a function applied to a value the function is idempotent on returns the
value**, so any growth, padding or rounding keyed on "the next X" is a no-op on
every value it has already produced. And when a task's mechanism is unreachable
from the product's own screens, **assert the mechanism's own numbers in a test
before believing the code that computes them** — the numbers are cheap and the
screen is not.

## 2026-10-05 — A mutation the runner refused to apply was recorded as a kill

Task 31's sweep claimed 13 of 14 deliberate breaks killed. **Two of them had never
run.** The runner compares its search string against the source before it builds —
the guard from *an unknown-name rule and a mutation of it can be the same value* —
and rustfmt had wrapped `atlas_sizes`'s return tuple across four lines since the
row was written, so the guard matched nothing, refused the mutation, and printed
`GUARD`. The output line was read as a kill when it was re-run, because the row
above it said `KILLED` and nobody counted the rows.

**The guard is right and the record was wrong.** A guard that lets a no-op mutation
through is worse than no guard at all: it certifies a behaviour change that was never
compiled. But a `GUARD` line is *not* a kill, and a summary written from the rows
rather than from the run will call it one.

**Rule:** read the verdict column, not the table above it, and **a sweep's summary
is written from the run's own count of its verdicts** — `grep -c KILLED` on the
output, not the number of rows in the table. Where the two disagree the table is
wrong, and here it was wrong twice in two places at once (the heading said 14 rows,
the table had 13, the script had 15). Related: *four documents agreeing is one
belief, counted four times*.

## 2026-10-05 — A counter incremented per event, beside a doc saying it was not

Task 31 counted the glyphs the atlas refused so a missing glyph would stop being
silent, incremented the counter on every refusal, and wrote in the same doc comment
that **"the character is not queued, retried or re-rasterized"**. All three halves
of that were wrong in the same direction: a refused key is not in the map, so
`get_or_insert` re-rasterizes and re-refuses it on every frame, and the text pass
re-expands each batch per frame — so one unrasterizable glyph reported 3 600 after
three seconds at 60 fps. The reviewer found it by reading the doc against the code,
which is the only tool that compares them.

**Rule:** a counter that answers "how much is wrong" must be keyed on **what is
wrong**, not on how often the question was asked — `HashSet` before `usize`. And
**when a doc comment states a mechanism as strongly as "not retried", write the
sentence from the call path rather than from the intention**: the intent was to
avoid pointless work, and the honest statement of it is "a retry costs a cache
miss", which does not change what the number means.

## 2026-10-06 — A brief's list of what a change touches is a list somebody has to compile

Task 32.2's orchestrator brief listed the files the mechanism would touch:
`paint.rs`, `batch.rs`, `render.rs`, `widgets/label.rs`, `widgets/list.rs`. **It
missed `widgets/scroll.rs`** — a test literal there (`fade: None, clip: None`) on
one of the eleven `DrawCommand::Text` constructing sites. The compiler found it,
the fix was two fields on one literal, and nothing was lost — but the enumeration
was an assertion made from an earlier session's reading rather than a command.
This is the same failure mode as *a sweep of a mechanism's call sites is not a
sweep of the data it is built from* (2026-10-04) and *a survivor is a missing
assertion, and only a sweep finds it* (2026-10-04): **a brief that claims "these
are the files" without a command that proves it is a hypothesis, not a fact**.
The blast-radius entries (`A drawn control with nothing behind it` /
`A draw-command assertion cannot see where a command lands`) are the same trap in
the widget — the code that *must* change is not the code the brief *says* changes.

**Rule:** before writing a brief that lists files, **run the command that proves
the list** — `grep -r "DrawCommand::Text" --include="*.rs" | grep -v test |
cut -d: -f1 | sort -u` — and paste its output into the brief. A list that was
read from a previous session's output is stale the moment the tree changes, and
the cost of missing one is a build failure the reviewer finds, not the author.

## 2026-10-06 — A log that cannot be parsed must abort

Task 32.1's first deliberate-break sweep wrote each run's log to the wrong path.
`grep` on a missing file exits non-zero, so the failure count came back empty,
and `awk '{s+=$1} END {print s+0}'` over no input prints `0` — so **six rows
printed `SURVIVED` and none of them had survived**. Six false results in the
direction that makes the work look finished. The same sweep's second defect fired
on three of the six rows at once: rustfmt had wrapped the search anchor across
two lines since the row was written, so the guard printed `ANCHOR NOT FOUND — not
a result` **and the runner printed `SURVIVED` anyway**, because the missing log
defaulted to zero below the guard — two contradictory verdicts on one row, and
the one that survived on the page was the false one.

This extends the existing guard/runner entries (*A mutation runner whose
reporting pipe is `head` never restores*, *A failure count parsed off the whole
log*, *A guard that reported why only when the why was the easy one*). All three
mechanisms are one bug: **a runner that cannot prove a result must not print
one**. A guard that fires and is then outvoted by a default is not a guard; a
count that defaults to zero on an empty input is not a count.

**Rule:** the abort must sit on the path that produces the verdict, not on a
guard above it. Parse the `test result:` line's failure field specifically —
`sed -n 's/.*[^0-9]\([0-9]\+\) failed.*/\1/p'` — and **an unparseable log (missing,
no `test result:` line, or the field not a number) must abort the runner with a
loud error, never default to "0 failures"**. An empty input to a summing parser
is an error, not zero.
