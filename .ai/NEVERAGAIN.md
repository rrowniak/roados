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
