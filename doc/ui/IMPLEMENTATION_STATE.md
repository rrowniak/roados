# UI primitives — implementation state

**Purpose:** so a fresh session resumes at the right task and does not redo
finished work. This is working state, not a spec: the task files own the
requirements, and where this file and a task file disagree, the task file wins
and this file gets corrected.

**Spec:** `doc/ui/PRIMITIVES.md`, `doc/ui/PRIMITIVES_ARCHITECTURE.md`, and
`doc/ui/TASK_UI_PRIM_01..32.md`.

**Last updated:** 2026-10-02

## Current position

**Status: task 21 (Chart) reviewed twice, uncommitted, awaiting the operator's
commit.** Task 20 is
committed as `79941cd`, task 19 as `b4a2db8`, tasks 15–18 as `d7240c8`, the
frame-rate readout as `3ddf5fa`. **Task 21 is the first task in this sequence to
go through `reviewer.md` twice** — task 20 was committed without ever being
reviewed, the operator's decision, and this file recorded it as one. The
*No unreviewed advance* gate skipped twice running is **closed as of this
revision**: round one returned 1 blocker and 6 minors, all seven were fixed, and
round two confirmed the blocker closed **by mutation** — removing the fix
reproduces the original four measurements exactly — and left 5 minors, which
the operator **waived with recorded reasons** rather than spend a third round
on. See *The two review rounds, and the five findings the operator waived*.
**"Reviewed" here does not mean "finished": all five are open.**

**Two rows of the task table were stale and are corrected in this revision:**
tasks 19 and 20 both read *implemented, uncommitted* while both have been
committed since 2026-10-02. The rule this file is written under is that it may
not contradict its artefact, so a state file nobody re-reads is worse than no
state file.

## Task 21 — what it decided, and what it found

**Reviewed twice** — see *The two review rounds, and the five findings the
operator waived*, which supersedes the "written to be reviewed" note this
section carried before it.

### The four operator decisions, taken 2026-10-02 before any code was written

Each was put to the operator as a question with the facts behind it, because
the task file requires something this pipeline cannot do as written. The
anti-aliasing one was asked twice: the operator asked for the detail behind two
of the three options before choosing, and the detail is what follows.

1. **Anti-aliasing is now the pipeline's, and it is hardware.** Task 21
   requirement 6 asks for *"Anti-aliased edges"*, and at the time of asking the
   pipeline had none: `Context::new` set four GL attributes and no multisample
   attribute (`render/context.rs:107-111`), there was no FBO anywhere in
   `ui_core`, and the solid shader's only antialiasing branch is a hard
   `discard` on an axis-aligned rounded rectangle. **The operator chose a
   multisampled default framebuffer** over an FBO with a resolve pass — one line
   rather than a change to `begin_frame`, `end_frame` and the resize path — after
   being told that it antialiases the whole app rather than the chart, that it
   may put a hairline seam where two quads share an edge, and that it costs
   measurable frame time. It is `MULTISAMPLE_SAMPLES = 4` with
   `MULTISAMPLE_BUFFERS = 1` in `render/context.rs`.
2. **Area fill is per-segment convex quads.** `DrawCommand::Polygon` — added in
   task 20 for the gauge needle — is **convex only**: the renderer fans `n - 2`
   triangles, which is exact for a convex polygon and a wrong picture for a
   concave one, and the region under a non-monotonic line is concave. The
   operator declined ear-clipping triangulation in the renderer.
3. **The demo gives up the list.** The window **cannot grow**: a 1280×1320
   request comes back **1280×1052**, the window manager's cap, measured on this
   host, and the window is 1020. The operator's answer was *"Remove some
   existing widgets like list or so. (Keep fps label.)"* The chart takes the
   list's column. **What that cost is written down below rather than left in a
   diff.**
4. **Line joins are mitred per-segment quads**, not one `Path` per series —
   `Path` offsets each segment perpendicular, so thickness is exact along a run
   and the joins notch on the outside of a turn.

### The finding that is not this task's: the solid pass does not premultiply

**`ui_core`'s solid-colour path blends as if its colours were premultiplied and
does not premultiply them.** `render.rs:1528` sets
`gl.blend_func(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`, which is the premultiplied
blend; the solid shader's own doc at `render.rs:143` says *"Colors arrive
premultiplied"*; and `quad_color` at `render.rs:617` divides each channel by 255
and **does not multiply rgb by alpha**. So a `Rect`, `RoundedRect`, `Line`,
`Circle`, `Path` or `Polygon` with alpha below 255 composites as
`rgb + dst·(1 − a)` instead of `rgb·a + dst·(1 − a)`, which **brightens over a
lighter destination and is brightest over one of its own colour.**

`Color::to_premultiplied()` exists at `property.rs:330` and has no caller in the
solid path. **The image shader (`render.rs:293`) is correct** and for a stated
reason: the texel is premultiplied at load and an alpha-only scale of a
premultiplied colour stays premultiplied.

**The text pass has the same latent defect, and this record first said it did
not.** `text_quad` (`render.rs:459`) calls the same `quad_color`, and the text
fragment shader (`render.rs:208`) scales by **glyph coverage**, not by alpha, so
a text colour below full alpha composites wrongly exactly as a solid one does.
**It is correct today only because every text colour in the tree is opaque** —
every theme colour is `alpha 255` and no widget builds a translucent one — which
is a fact about the callers, not about the pass. **The fix is one place**
(`quad_color`), and a maintainer who read "the defect is the solid pass alone"
and premultiplied inside the solid shader would fix nothing. Corrected here after
the review found it; the reviewer is right and the first version of this section
was an over-claim of the kind this file exists to catch.

It was found by the chart's author, not by reading the code: the first
measurement of a translucent area fill came back with two unexplained 64-px
bands at the ends of the plot reading `(255,194,255)`. The follow-up's
explanation was its own throwaway harness — a second chart drawn at the same x
and y origin, so a translucent fill lay over an opaque bar of its own colour —
and the arithmetic then predicted the measured value at **three** destinations
exactly. `Color` values above the source's own channels cannot come out of a
composite of that source, which is what ruled the widget out.

**It is pre-existing, it is not task 21's, and it was not fixed here.** Fixing
it changes every translucent primitive in the application, which is a change to
every capture in this file. It is recorded here for the operator as a separate
pipeline task.

### What the widget is, and the two shapes it had to choose between

`ui/src/ui_core/src/widgets/chart.rs` is new — **6 532 lines, 115 unit tests, 23
doctests**, counted after the fix round. `ChartType` is `Line`, `Bar` or `Area`;
`data`,
`x_labels` and `y_labels` are `Property<Vec<…>>`; `chart_type` is a plain field
behind a setter, the gauge's arrangement. **`Series` is two parallel arrays** —
`x` as a share of the plot's width beside the values — and that is load-bearing
rather than tidy: appending re-spaces every existing sample, so a chart that
animated
only the values would draw a correct-looking series in the wrong places.

Both fills are per-segment quads, and both reasons are the same reason: the
renderer's fan is exact for a convex polygon and for nothing else. The line's
quads are **mitred**, and the mitre is taken only while it stays inside
`MITRE_LIMIT · half` — a corner is dropped to a disc rather than allowed to
produce a concave quad. **A first implementation had exactly that defect** (the
along-axis swing can exceed a segment's own length, so a 201-sample chart at a
90° turn went concave) and it was found by the on-screen and fuzz work rather
than by an assertion. It costs nothing on a chart a person would read: the bound
is 6 px, so no corner is affected while the sample pitch stays above 12 px —
**50 samples on a 600-pixel plot**, since `600/(n−1) > 12` holds to n = 50 at
12.245 px and n = 51 is exactly 12. *(Corrected from 49 by the review; this
document had the wrong figure and the widget's own doc now carries the
arithmetic that decides it.)*

`ui/src/ui_core/src/widgets/chart.rs` is **6 532 lines with 115 unit tests and 23
doctests**, and **how each was counted is worth writing down**, because both
sentences here were wrong once and the fix round moved both numbers again:

- **lines** — `wc -l src/ui_core/src/widgets/chart.rs`.
- **unit tests** — `cargo test --lib widgets::chart -- --list | grep -c ': test$'`,
  which counts **registered** tests rather than `#[test]` attributes. It is **115**,
  and `grep -c '^    #\[test\]'` in the file is also **115**, so the two agree and
  there is no attribute without a function and no function registered twice.
- **doctests** — `cargo test --doc -p ui_core -- --list | grep -c 'chart.rs'`,
  which is **23** and was 23 before the fix round: the round added no example, only
  prose and tests. The whole crate has 192 registered doctests.

The previous pair of sentences published **5 896 lines and 112 unit tests**, and
the reviewer's measurement was **6 080 and 113**; all four were stale, because the
counts had been taken before the last edits of the round that wrote them. **A
count published in a document about a file that is still being edited is a
quotation of the past**, and the cheapest guard is to compute it last and put the
command next to it.

### Two claims this task's own docs got wrong, and what fixed them

**Both were found by measuring, which is the point of the `NEVERAGAIN` entry
this task is a second instance of.**

1. **"A translucent fill does not seam" was measured in a harness whose
   surroundings the author had not accounted for.** The claim was true — the
   quads are a tiling and not an overlay, and 1 distinct colour in 27 900 pixels
   says so over the whole region — but the first version of the sentence
   supported "one uniform colour" with nine interior x positions, which is not
   what nine positions establish. It now says what was measured, names the
   harness confound, and rests on two tests that assert the **precondition**
   instead: that consecutive fill quads share exactly one x and that no two
   interiors ever overlap.
2. **`Chart::paint` said nothing it records reaches outside `rect`, "with one
   measured exception, which is the stroke's own half width".** That is false at
   a turn, and the demo subagent found it by measuring the shipped chart: a
   mitred corner is up to `MITRE_LIMIT · half` = 6 px long, and a reading at the
   top of its range puts its data point on the plot's **top** edge — and
   `plot_rect` insets the left and the bottom only, so `plot.y == rect.y`.
   Measured 4.3956 px above the node's top edge one frame into an append, and
   2 px on the settled chart. The passage is now three named overhangs with a
   bound each, and **three tests** fail if it becomes false again. A fourth
   overhang was measured while fixing it: the first y label's line box reaches
   `LABEL_FONT_SIZE / 2` = 6.0 px above the plot's top — **equal to the mitre at
   the defaults and larger than it for any font size above 12 px.**

**A duplicated constant went with the second one.** The demo had carried
`CHART_STROKE_REACH = 6.0`, a private copy of the widget's number that would rot
the moment `MITRE_LIMIT` or `line_width` moved — the shape of the `NEVERAGAIN`
entry *one sibling got the operator's fix; the other with the same constant did
not*. The widget now exposes **`Chart::stroke_reach()`** and the demo calls it
in three places.

### What the list cost, stated rather than left in a diff

The operator's decision removes the demo's **only scrolling viewport**. `Scroll`
— the widget they reported a 6-pixel bar and a ten-to-one drag lag against — is
now **driven by nothing in the demo**, and its only remaining coverage is
`ui_core`'s own tests. `SCROLLBAR_THICKNESS`'s test went with it, and that test
was the only thing standing between the 2026-10-01 report and a repeat of it.
The demo also lost its **on-screen proof of virtualisation** (`first 0, live 10,
free 0`) and the **positional mouse-wheel routing** claim, whose two tests used
the list as their fixture; a positionless `Scroll` — the steering wheel's axis —
is still covered, by the slider's test.

The reasoning behind the deleted constants was **not** deleted with them.
`SCROLLBAR_THICKNESS`'s doc carried the operator's finger complaint verbatim; it
is preserved, dated and attributed, in `KEY_HEIGHT`'s doc, which already argued
from the same judgement. `LIST_FONT`'s is carried by `CHART_READOUT_FONT`'s.

### The three keys, and what a still can and cannot prove

`H` cycles `Line` → `Bar` → `Area` and wraps, so the task file's three separate
rendering criteria are three presses of one key. `A` appends through
`animate_push`; `S` shifts through `animate_shift`. All three were grepped
against `handle_event`'s existing arms first — no collision with `T`, `Space`,
`+`/`-`, `C`, `0`/`1`, `F`, `[`/`]`, `P`, `,`/`.`, `G`, arrows, `Tab`, `Return`.

**Keyboard injection does not reach the window on this host** — the subagent
built the XTEST injector and the positive control (`T`, which changes the whole
window when it lands) moved **212 px**. So *"New data animates in smoothly"* is
covered by **tests through `Demo::handle_event`'s real event path**, asserting
the mid-flight state and the arrival, and **is not claimed on a capture.**

### What is on screen, and how it was got

One capture of the default state, by the stock method in *Verifying a change
that draws* — `cargo build --release`, `setsid ./target/release/ui_demo >
/tmp/demo.log 2>&1 &`, window id `0x100002f` **re-read at the time of the
capture**, `pgrep -a -x ui_demo` in the same call, `magick import -window`, and
`stderr` empty. `rg -c "SEED|PROBE|PREVIEW" ui/src/ui_demo/src/main.rs` is
**0** and `git status` shows no stray file.

Measured, by the demo subagent and re-checked against the pixels:

- **the series lands where its values put it** — every one of the eight readings
  within **2.00 px** (six of eight within 0.41 px), the stroke **3.034 px**
  perpendicular against a requested 3.0 (+1.13 %);
- **the axes and labels** — the y axis a full 433-px column at x=1000, the x axis
  rows 671–672, four grid lines at 326 / 412 / 498 / 585 against the computed
  326.4 / 412.8 / 499.2 / 585.6, and seven x labels inside the gutter with **no
  ink in the node's last two columns** — the newest sample is deliberately
  unlabelled, because a `DrawCommand::Text` carries no width;
- **the bar and area captures came from two temporary releases** whose opening
  shape was the demo's own `CHART_TYPES[1]` and `[2]`, **no new seed variable**,
  reverted before the final capture with a `diff` and the `rg` count above.
  Seven bars for eight readings (the eighth is the range's low and has no
  height), every bar's top within **0.87 px**; **269 of the plot's 270 columns**
  carry a fill run reaching the bottom edge.

### Deliberate breaks — 52 run, 49 killed, 3 no-ops

| writer | mutations | killed |
|---|---|---|
| the chart widget | 19 + 6 + 2 | 27 |
| MSAA (`render/context.rs`) | 4 | 3 + 1 by capture |
| the demo wiring | 18 + 5 | 20 + 2 |

**Three survivors were reported rather than hidden, and the reason is worth
keeping.** Replacing `chart.stroke_reach()` with a hand-written `6.0` passes
every test, because at the defaults **`stroke_reach()` *is* 6.0** — the
substitution is not a different number, it is the same number with the coupling
to `set_line_width` and to a future `MITRE_LIMIT` removed, and no assertion can
distinguish two equal numbers. Every assertion that reads the bound compares it
against a clearance of ten pixels or more, so a bound of 1, of 6 and of 12 all
pass. What the suite *does* police is the **value**: a bound taken from the end
cap's half width (1.5) fails the containment test on the exact geometry that
makes the difference, the mitred corner at y 235.67789 against a node top of
240. **Provenance is a grep, not an assertion** — `grep -c CHART_STROKE_REACH
main.rs` is 0.

### What is NOT claimed

- **No criterion is verified through injected input.** Pointer injection does
  not reach the window on this host and keyboard injection did not arrive
  (`T` moved 212 px). Criteria 1, 2, 3, 4 and 6 are capture-verified and
  measured; criterion 5 is covered by tests through the real event path and is
  **not** claimed on a capture.
- **The animation was not seen mid-flight in a capture** — a still proves what is
  drawn and never how it moves.
- **`y_labels` are deliberately empty**, which is requirement 4's auto-scaling
  default and puts the y axis on the node's own left edge. Criterion 4 is
  therefore proved by the x labels and the two axes, **not by numbers on the y
  axis.**
- **`set_fixed_range` / `clear_fixed_range` are not exercised by the demo.**
  Auto-scaling is what requirement 4 asks for by default; with a fixed range the
  y labels would mean something, and the demo writes none. The operator's call.
- **`cargo audit` was not run** — not installed on this host, the standing tool
  gate. **No dependency changed**, which is the thing it would have checked.
  `ui/Cargo.toml` and `ui/Cargo.lock` are untouched, and the **aarch64
  cross-build passes with no sysroot** — `ELF 64-bit LSB pie executable, ARM
  aarch64`, with the same **four** dynamic dependencies (`libm`, `libgcc_s`,
  `libc`, the loader), so this task added no dependency and changed nothing about
  how the target links.
- **The one-line MSAA change makes every capture in this file historical.** The
  gauge's hard edges are gone, and the gauge's own module doc has been
  superseded in place rather than rewritten.

### The frame rate

`fps-check.sh 12 55`, four release runs across this task: **61.7, 62.1, 62.3**
(the demo wiring) and **62.0** (the follow-up), against **61.8 fps at `f8ba81e`
measured before any of this**. A whole new widget, drawn every frame, plus 4x
multisampling, inside the existing spread. The MSAA author's own interleaved A/B
put the attribute's absence and presence at 61.9/61.0 and 61.8/61.7 — inside
each other's spread, so **the rate is not evidence either way** and the honest
statement is that nothing measurable moved, not that MSAA is free.

### The two review rounds, and the five findings the operator waived

Task 21 went through **`reviewer.md` twice**, in a session separate from the
author's each time — the first time in this task's history, which is not a
standard this sequence can keep skipping. Round one returned **Approve with
required changes**: **1 blocker, 6 minors**. All seven were sent back and fixed.
Round two, on the integrated fix round, returned **Approve with required
changes** again: **the blocker is closed and verified**, and **5 minors remain,
all of which the operator waived on 2026-10-02** after being given the list.

**The blocker, and how it was proved closed.** A non-finite sample erased the
two real series segments either side of it, because `draw_series` built its
`joins` across the whole point vector and `normal_of` answers `None` for a `NaN`
— so both real vertices touching a gap got an undefined join, the segments
were skipped, and the fallback disc is drawn only for a *flat* join, so nothing
replaced them. Measured before the fix: `[1,5,9,NaN,3,7]` drew 1 quad where its
runs hold 3 segments, `[NaN,5,9]` drew 0 of 1, and as an `Area` chart the fill
drew 3 quads against 1 stroke quad, so the outline was missing exactly where
the fill was not. **Four doc sites already said the opposite** — that a missing
sample breaks the run and costs only the segment that spanned it — so the
behaviour was fixed and the docs were left standing.

Round two verified the fix **by mutation rather than by reading it**: removing
the two `.filter(is_finite)` calls reproduces all four of round one's
measurements exactly, so the numbers above are what the fix reverses. It also
established the fix's **blast radius is exactly two tests** — ordinary run-end
flat caps are untouched — and that this follows structurally rather than
empirically: the filter is the identity on all-finite input, and a repeated
point is finite, so it cannot reach the one case it must not change. **A
repeated point is not a gap**, and one test now pins that contrast in a single
assertion.

**The five waived findings.** Each was verified as a real defect before it was
waived, and each is *verifiable* — none is an unverifiable acceptance criterion,
which is the only kind `task-sequence.md` lets a waiver cover without a
struggle. The operator's decision was to stop here rather than run a third
round, and the reasons are recorded so a later session can revive them:

1. **A stale citation.** This file's operator-decision paragraph cites
   `render/context.rs:107-111` for the claim that `Context::new` *"set four GL
   attributes and no multisample attribute"* — and those lines now hold
   `MULTISAMPLE_SAMPLES` itself. The citation names the constant the sentence
   says was absent. **Revive by** citing `context.rs:203-206`, and dropping the
   line number from the "and no multisample attribute" half, which is a claim
   about the state *before* this change and no current line can evidence it.
2. **A cross-reference with nothing behind it.** `paint.rs`'s `DrawCommand::Polygon`
   doc ends by pointing the hairline-seam caveat at `MULTISAMPLE_SAMPLES`'s doc.
   **That caveat is in neither its doc nor `MULTISAMPLE_BUFFERS`'s** — it exists
   only in that one sentence. `gauge.rs`'s identical pointer is correct, because
   both of its claims are in `MULTISAMPLE_SAMPLES`'s doc. **Revive by** moving
   the seam into `MULTISAMPLE_SAMPLES`'s doc, where the operator was told about
   it and where the A/B that looked for it lives.
3. **A coverage claim wider than the assertion.** The nine-row sweep table is
   introduced as one the test *"asserts every cell of"*, exactly. **Only the
   denominator column is asserted for all nine heights**; the `join` and
   `drawn` columns are asserted for 240 and 300 and derived for the other
   seven. Every cell is correct — round two measured all thirty-six — so this is
   an over-statement, not a wrong number, and it is the same shape as the round's
   own new `NEVERAGAIN` entry. **Revive by** hoisting the table into the test's
   loop and asserting `join` and `drawn` per height.
4. **The open defect is not in *What is NOT claimed*.** `chart.rs` calls the
   `Join::Corner((0, 0))` at a small positive `1 + p·q` *"a defect, not a
   degenerate case"* and hands it to *"the integrator"*, with
   `a_full_reversal_depends_on_which_way_f32_rounds` as a tripwire. This file's
   *What is NOT claimed* lists six things task 21 does not establish and does not
   mention it, while the pre-existing premultiply defect gets a subsection, a
   history entry and a bullet. **The omission reads as deliberate because the
   asymmetry is deliberate**, which is why it is recorded here. **Revive by** one
   dated bullet in *What is NOT claimed* plus one history line.
5. **A run count that is one behind.** *The frame rate* above publishes *"four
   release runs across this task"* and lists four. The fix round ran a fifth
   (**62.5 fps**, reproduced by round two) and did not refresh it — eleven lines
   below where the fix round itself wrote that a count about a still-being-edited
   file is a quotation of the past. **Revive by** adding the run and saying
   "five", or by qualifying the list as the pre-fix-round runs.

**What this waiver is not.** It is not a claim that the widget is sound. It is a
decision to stop at a reviewed state rather than to spend a third round on five
citations, one coverage sentence and one history entry — none of which can
change a pixel. **Nothing in it is fixed, and all five are recorded so that
"reviewed" here is not read as "finished".**

## Task 20 — what it decided, and what it found

Written to be reviewed. Nothing here has been through `reviewer.md`.

### The three operator decisions, taken 2026-10-01 before any code was written

Each was put to the operator with the facts behind it, because the task file
requires something the codebase cannot do. They are also under *Ratified by the
operator*.

1. **The arc is not antialiased, and the task file's requirement 5 is not met.**
   Verified before asking: the solid shader's SDF branch is guarded on
   `v_radius > 0` and models **only an axis-aligned rounded rect**
   (`render.rs:154`); `line_quad` hardcodes `radius: 0.0` (`render.rs:670`); the
   GL context sets no multisample attribute (`render/context.rs:107`); there is
   no FBO anywhere. Chosen: hard edges, documented in the widget's own module
   doc, which names the three reasons and what would reverse them. The reviewer
   should treat that doc as the claim and the capture as the evidence — see
   *What is on screen*.
2. **A filled `Polygon` draw command, not a `Line` needle.** Requirement 3 says
   "needle rendered as a triangle" and `Path` *strokes* an outline, so a
   three-point closed `Path` is a hollow triangle. The operator chose to add the
   primitive. It cost one new enum variant and **no shader change**, which was
   not obvious beforehand and is the most transferable fact in this task — see
   *The triangle that fits through a quad-only pipeline*.
3. **The three animation-test buttons are gone**, at the operator's words:
   *"You can remove the first three buttons that were used for testing
   animations."* That freed the head of the right-hand column for the gauge,
   which is the acceptance criterion's only placement — see *Where the gauge
   went, and what it cost*.

### The triangle that fits through a quad-only pipeline

`DrawCommand::Polygon { points, color }` fans a **convex** polygon into `n - 2`
triangles, each emitted as one `Quad` whose **fourth corner repeats its third**.
`quad_indices` addresses four corners as `0,1,2 / 0,2,3` (`render.rs:791`), so
`[a, b, c, c]` draws the triangle `(a,b,c)` and the degenerate `(a,c,c)`, which
encloses no area and covers no fragments.

**That is the whole trick, and it cost nothing.** No new vertex type, no shader,
no change to the index buffer, and the pipeline's quad-only invariant (4
vertices, 6 indices) is preserved and pinned by a test. There is no face
culling and no depth test in `render.rs` at all — grep for `cull`, `front_face`
and `depth_test` returns nothing — so winding does not affect visibility.

Fewer than three points emits no quads, and `command_bounds` returns `None` for
it, **by the same rule the empty `Path` already followed**: a command that draws
nothing has no bounds to clip against. One rule, two arms, not two rules that
happen to agree.

The fan is exact for a **convex** polygon and nothing else; a concave one fans
into overlapping and inverted triangles. Every `match` on `DrawCommand` got a
real arm — none was silenced with `_ =>`.

### The defect this task found in its own work, on a screenshot

**The arc was visibly beaded, and the module doc claimed it was not.** The first
implementation drew the band as a **chain of overlapping filled `Circle`s**, on
a rationale *this session's integrator supplied in the subagent brief*: "a `Path`
shows notches on the outside of the curve, and overlapping circles have no notch
because every circle is round."

**Both halves of that were wrong**, and the demo subagent's capture is what
proved it: sampling the outer edge along rays read **100.0 px at every circle
centre against 94.4–95.8 px at every bisector — a 5.6 px scallop on a 14 px
band.** A chain of circles tangent on their **centre lines** has outer edges
that touch only where `R >> r`, and the gauge's defaults are `r/R = 0.075`.

The `Path` alternative is worse, and for a **different** reason than the brief
gave: `line_quad` offsets each segment **perpendicular**, so a segment's outer
corner lands at `sqrt(R² + r²)`, not `R + r` — a band of the requested thickness
is drawn about **6.7 px too thin everywhere**, before any scalloping. Both
numbers were reproduced from the geometry before the fix was written.

**The fix uses the primitive that had just landed, and nothing new.** One convex
four-point `Polygon` per segment, corners on `R ± thickness/2` at the segment's
two endpoint angles. The corners sit exactly on the two radii, so the band is
the requested thickness with **no perpendicular-offset error at all**, and the
only error left is the chord between corners:

| | outer edge error | cost, track + fill |
|---|---|---|
| chain of circles (shipped, then reverted) | **7.26 px** | 132 quads |
| `Path` quad strip | 6.74 px thin *everywhere*, plus 0.26 px of sag | 66 quads |
| **annular quad per segment (shipped)** | **0.255 px** | **66 quads** |

About **29x** better than the chain, at half the primitives. Measured on screen,
not asserted: the band's pixel deficit against the ideal 270° annulus went from
**1064 px (17.4%) to 17 px (0.28%)**. The beading is gone.

The sagitta is `step²` in the small-angle limit, so the tessellation degrades
gracefully where the chain's error scaled with `r` — a test asserts the `step²`
signature by halving the segment count and checking the error quadruples, which
is the property that makes a coarse tessellation safe.

### Where the gauge went, and what it cost

**At (664, 240) in a 200×200 box**, in the column the removed button row
occupied, entirely above `BAND_TOP = 720`. Nothing above the band moved, so
every capture of tasks 11–19 is still a capture of the same pixels.

**The operator should know what the button removal took with it.** The buttons
were task 12's only on-screen proof, and 22 tests went with them — **accounted
for exactly**, 22 removed and 17 gauge tests added, so `ui_demo` moved 140 → 139
only because of the net. What is **no longer demonstrable** is that a click
reaches a widget and something happens; the demo's remaining pointer-driven
controls are the slider, the toggle and the list, and `Tab` still reaches the
field.

**One gap was opened and has been closed.** The removal took the demo's only
**Tab-order walk test** with it, leaving focus order entirely unasserted — the
remaining test checks that `Tab` reaches *one* widget, so any order would pass.
`tab_walks_every_focusable_control_in_order_and_wraps` and its `Shift+Tab`
mirror now pin the full six-stop order (**slider, image, toggle, progress bar,
list, text field**), plus `the_gauge_is_not_in_the_focus_order` — a gauge has
no `on_event` and no `focused` property, so it is a display and not a stop. A
survivor found this: dropping `("gauge", …)` from `placed_rects` passed every
test, because each neighbour claim named its neighbour by hand, so the test now
asserts **membership** as well.

**`SLIDER_THUMB_RADIUS = 18` was left alone**, but a cheap assertion now says
something its comment did not: the clearance under the slider's readout is
**exactly 8 px** and a 22-pixel knob is **8 px taller**, so 22 still does not fit
and the ceiling holds. The looser column after the buttons went is *vertical*
headroom the removal did not create.

### What is on screen, and how it was got

One capture of the default state, by the stock method, **re-verified by this
session independently of the subagents' own claims**:

```
cargo build --release
setsid ./target/release/ui_demo > /tmp/verify-demo.log 2>&1 &
pgrep -a -x ui_demo                                  # 3115423, same call as the capture
xwininfo -root -tree | rg '"roados ui_demo"'         # 0x100002f, 1280x1020
magick import -window 0x100002f /tmp/verify-gauge.png
```

Window id re-read at the time of the capture, process confirmed alive in the
same call, `stderr` empty. **No seed, no environment variable, no rebuilt
binary**: `rg -c "SEED|PROBE|PREVIEW" ui/src/ui_demo/src/main.rs` is **0**.

Cropped at 250 % and looked at, and then **measured** — the numbers above come
from sampling the outer edge along 181 rays inside the node rect and from a
band-area count that cancels pixel quantisation. Measured, because a crop at
2500 % shows a 5 px scallop and a 2 px one equally well:

- **two distinct bands, fill over track** — the fill is `Primary` on top of the
  `Border`-coloured track, both in the same annulus, and the fill's segments
  begin after the track's;
- **the band is smooth** — 0.28 % area deficit against the ideal annulus;
- **the needle is a filled triangle** with a hub disc covering its base, and the
  needle's rows widen linearly from tip to base;
- **eleven tick marks** sit inside the track's inner edge and clear of the band;
- **the edges are hard** — stair-stepped, most visibly at nine and three
  o'clock where the tangent is vertical. This is decision 1 above, **seen**
  rather than read about;
- **the readout shows `120 of 240, 50%, Needle`**, so "50 %" is legible without
  inferring it from the picture.

### Deliberate breaks — 58 run, 56 killed, 2 no-ops

| writer | mutations | killed |
|---|---|---|
| `Polygon` primitive | 21 | 21 |
| the gauge widget | 25 | 25 |
| the demo wiring | 12 | 12 |
| the arc-geometry fix | 20 | 18 (+2 aborted as no-ops) |

**Two survivors were real and both were fixed.** Removing `clear()` from
`animate_to_state` survived because **two aims of the same length arrive on the
same frame** and race invisibly; the test now makes the first aim longer than
the second. Removing the `(radius - half).max(0.0)` floor survived because **no
fixture had a box narrow enough to give a negative inner radius** while the arc
radius was still positive — the same class as the origin/extent entry below.

### What is NOT claimed

- **No criterion was verified through injected input.** Pointer injection does
  not reach the window on this host. The gauge is driven by `,` and `.`, and the
  needle's spring is asserted by tests that measure the overshoot, **not** seen
  mid-flight in a capture.
- **`cargo audit` was not run** — not installed on this host, the standing tool
  gate. **No dependency changed**, which is the thing it would have checked.
- **The frame rate is one reading.** See below.
- **`DEMO_APPLICATION.md` is modified in the working tree and was not touched by
  any of this task's sub-agents** (mtime 18:45, before this task's writes at 19:23
  and 19:44). It appears to be the operator's own edit from a parallel session.
  Flagged rather than reconciled.

### The frame rate

`fps-check.sh 12 40`, this session's own run:

```
fps-check: 587 frames in 12.006s
fps-check: average 48.9 fps, worst frame 51.6 ms, 1 frame(s) over 33 ms
fps-check: PASS — 48.9 fps is at or above the 40 fps floor.
```

**48.9 fps against a recorded baseline of 49.7–54.1**, inside the spread, and it
should not have risen: the shipped arc draws **fewer** primitives than the chain
it replaced. The intermediate beaded build measured 50.1 and the button removal
freed what the gauge cost, so the CPU comparison is confounded in the gauge's
favour — **the honest statement is that no regression is visible at the floor,
not that the gauge is free.** The aarch64 cross-build passes with no sysroot,
`Machine: AArch64`, and the same four dynamic dependencies.

**Two things a reviewer should weigh.** The fill's segments are cut from the
*fill's* sweep, so a half-value fill has 17 segments where the track has 33 and
**the two bands' corners do not line up along the fill** — the fill is drawn
over the track in the same annulus so the seam is between two overlapping bands,
but that is a judgement and it is the one thing in the geometry a human should
look at closely. And **a polygon's extreme point is not always a corner**: at a
step that lands no corner on twelve o'clock, the topmost corner sits one sagitta
inside the topmost drawn edge.

## Task 19 — what it decided, and what it found

**Three operator decisions shaped this task, taken 2026-10-01 before any code was
written**, each because the task file did not authorise it:

1. **The on-screen keyboard is a `ui_core::widgets` module** — not demo-local code,
   and not a second type inside `text_input.rs`. `PRIMITIVES_ARCHITECTURE.md`'s
   § *Module Layout* is amended to list `keyboard.rs`.
2. **`InputEventKind::Text { text: String }` was added** to `input.rs`, with a
   `GestureRecognizer::process` arm over SDL's `EVENT_TEXT_INPUT`. Without it
   *"Keyboard input inserts characters"* cannot be met: no event the crate
   produced carried a character, and `process` ended in `_ => {}`.
3. **The window grew rather than the demo being re-laid-out**, and the band is
   laid out **side by side** rather than stacked. Both are measured below, and the
   second one was not the plan.

**1053 `ui_core` unit tests (1 ignored) + 140 demo tests + 154 doctests**, all
green; `cargo fmt --check` clean, `cargo clippy --all-targets --all-features -D
warnings` clean, `cargo doc --no-deps` with no warnings, and the **aarch64
cross-build passes with no sysroot** — artifact `Machine: AArch64`, and the same
**four** dynamic dependencies, so task 19 added no dependency and changed nothing
about how the target links.

**The frame rate is 50.0 fps** against a recorded baseline of 49.7, and the two
new widgets cost about **1.1 points of a core**, measured interleaved against
`HEAD` rather than against a number from another session.

**The work was split** per `.ai/protocols/subagents.md` § *Implementation fan-out*,
because it touched six files and four independent components: `input.rs` first,
since both widgets depend on its new variant; then `keyboard.rs` and
`text_input.rs` as two **file-isolated** subagents that do not import each other;
then the demo wiring, here. See *Task 19 — what it decided*.

**Last task before this: the frame-rate readout** — not a `TASK_UI_PRIM_n` task,
an operator request of 2026-10-01 to make performance measurable after every run
an agent launches. `ui/src/ui_demo/src/fps.rs` is new; `ui_demo` gained three
constants, the readout and the bounded run. **838 `ui_core` unit tests (1
ignored) + 122 demo tests + 129 doctests**, all green.

**Task 18 — Widgets: List and Scroll**, with 15, 16 and 17 in the same changeset.
Six new files (`texture.rs`, `widgets/toggle.rs`, `image.rs`, `progress.rs`,
`scroll.rs`, `list.rs`), `paint.rs` and `batch.rs` extended, `render.rs` given a
third pass, `ui_demo` enlarged to 1280×720 and given all four widgets, and one new
dependency: `sdl3`'s `image` feature. The two scrollbar defects the operator
reported against the list — a 6-pixel bar too narrow to aim at, and a drag that
lagged the cursor by about ten to one — are fixed inside that same commit; see
*The fourth operator report*.

**Task 13 — Widget: Container.** `widgets/container.rs` is new (18 tests),
`layout.rs` gained `Padding` and the pass now honours it, and `ui_demo`'s
private `container()` helper is **gone**: the demo's tree is built out of
`Container` widgets, and the row of pads is drawn on a card — a container with
the theme's `Surface` behind them and a padding of 12. 326 `ui_core` unit tests
+ 49 demo tests + 49 doctests.

**Task 12 — Widget: Button.** `widgets/button.rs` is new (2168 lines),
`input.rs` gained `Focus::focus` and `route` plus a **unit fix that is not part
of the button** (below), `ui_demo` gained a band of three buttons over a click
counter. 301 `ui_core` unit tests + 44 demo tests + 44 doctests.

**The blocker the review found: the gesture recogniser compared nanoseconds
against millisecond thresholds, so no `Tap` was ever produced.** SDL stamps every
event with `SDL_GetTicksNS()` — `SDL_events.h:300`, *"In nanoseconds, populated
using SDL_GetTicksNS()"*, in the vendored 3.4.16 this project builds against.
`input.rs` read those stamps unchanged and compared them against
`TAP_MAX_DURATION_MS = 300`, which made the tap window 300 **nanoseconds** and
fired a long press at 500 of them. `check_long_press` runs before the release is
evaluated and `release_pointer` returns as soon as one has fired, so **every
press was a long press**: a button's `on_click` could not fire from any pointer,
and task 12's acceptance criteria 2 and 7 were genuinely unmet. This is task 10's
code, and the unit tests did not catch it because every one of them used small
timestamps — `200` for "quick" — which is under a 300-nanosecond window. Fixed
by making the two thresholds [`std::time::Duration`], so the mistake cannot be
written: `held <= TAP_MAX_DURATION` does not compile across units where
`held <= 300` compiles happily and is wrong by a factor of a million. Two tests
now fail on the old behaviour, one of them a realistic 100 ms click.

**Task 12: what was reviewed, and what was not — stated precisely, because the
first version of this note was too strong.** A *dedicated* re-review of task
12's fixes never happened, so the workflow's *No unreviewed advance* gate was
formally skipped: the operator committed `9973185` with the blocker and the
three majors fixed, each fix mutation-verified only by the session that wrote
it. But "unverified" overstates it, and the task 13 review corrected that:

- **The blocker has been independently confirmed twice.** The task 13 reviewer
  read the source and found the unit fix genuinely in place — the thresholds are
  `Duration`s at `input.rs:93`/`:97`, the tap comparison at `:879` is on a
  `Duration`, and the tests convert at the boundary at `:1053` — and then drove
  a **real click** on the live demo and took the counter from `0 clicks` to
  `1 clicks`, so the tap recogniser works end to end on real input. That is the
  part that mattered, and it is no longer one agent's word.
- **What is still one agent's word:** the three majors' fixes — the `shapes()`
  ordering tests and the reordering mutation that now fails, the removal of the
  vacuous `shadow.a <= round(PRESS_SHADOW_ALPHA * 255)` assertion, and the
  retitled demo test — and the minors: the two stale counts, the
  `THEME_SPACING_*`/`THEME_RADIUS_MD` constants with their token-reading test,
  and the `focus_navigation` doc correction.

A reviewer dispatched against `9973185` needs no working tree, so closing this is
cheap whenever it is wanted. The order that would be least wasteful is to let
task 14 land first and review the two together, since the unexamined fixes are
test and comment changes with no behavioural surface.

## Task 19 — what it decided, and what it found

Written to be reviewed. Nothing here has been through `reviewer.md`.

### The three decisions that were the operator's, not the agent's

Each of the three was put to the operator as a question with the facts behind it
**before any code was written**, because the task file requires something it does
not describe. They are also under *Ratified by the operator* with their dates.

1. **`widgets/keyboard.rs` is a module of its own.** The alternatives were a
   second type inside `text_input.rs` — which puts two widgets in a module named
   after the first, the argument `widgets/mod.rs` already makes against
   one-module-per-shared-type — and demo-local code, which would have proven
   task 19's keyboard criterion against code the next vehicle cannot reach. The
   Module Layout is amended.
2. **`InputEventKind::Text { text: String }`, and a `process` arm over
   `Event::TextInput`.** The alternative the operator declined was deriving
   characters from `KeyDown` keycodes, which needs no `input.rs` change and is the
   wrong mechanism: a keycode names a physical key and says nothing about which
   character that key produces on the layout in use, so a widget doing it would
   re-implement, wrongly, what `EVENT_TEXT_INPUT` exists to deliver.
3. **The window grows; nothing in the gallery moves.** The band goes below
   `BAND_TOP = 720`, which is where task 14 recorded the window was already full.

### What `InputEventKind::Text` cost, and it is a public API change

**`InputEventKind` is no longer `Copy`.** It was
`#[derive(Clone, Copy, Debug, PartialEq)]`, and a `String` payload cannot live in
a `Copy` enum. The alternatives were a `char`, which cannot carry the
multi-character run SDL delivers for an IME composition commit, and a fixed
buffer, which truncates. `InputEvent::kind()` therefore clones.

The whole-repository cost was **two call sites**, both in this repository:
`InputEvent::kind` (`input.rs:248`, which now clones and says so in its doc) and
one `list.rs` test that formatted a `kind` *after* moving it into
`InputEvent::new` (`list.rs:3790`, which now renders the message first). That is
the number to check a claim like "removing `Copy` is invasive" against — the
build finds both of them, and nothing else in the tree needed changing.

**An empty run produces no event.** SDL delivers `""` when an IME composition is
cleared, and an event carrying `""` would be one every consumer has to learn to
ignore — which is exactly what a widget with a text buffer would fail to do.

### The window is 1020 tall, and 1160 was what the first attempt asked for

**The plan was a stacked band and a 1280×1160 window.** The window came back
**1052 pixels tall**. Measured, on this host: two stacked displays —
`eDP-1` at 1920x1080 at `+0+1200` and `HDMI-A-1` at 1920x1200 at `+0+0`, a root
of 1920x2280 — and the window manager capped the height where the window landed
(`xwininfo`: `Absolute upper-left Y: 78`, `Height: 1052`).

**This is why the band is side by side and not stacked, and it is worth stating as
a general fact rather than as this task's story.** A window taller than the cap is
not merely awkward: **the bottom of the keyboard never reaches the screen, so the
capture that is supposed to prove the widget draws cannot see it.** The band gets
[`BAND_HEIGHT`] = 300 pixels. A field over a keyboard needs 64 + a gap + a
260-tall keyboard = 344, which does not fit; a field beside one needs the height
of the keyboard alone, which does. For a car the side-by-side shape is also the
better one — a driver reaches the keys beside the field without the field moving
under their hand.

`the_whole_band_fits_in_the_space_below_the_gallery` checks the budget, and
`Demo::new` **also** checks it at build time and returns the offending name as the
error, because this is the class of failure that is invisible until somebody looks
at a screen.

**Nothing above `BAND_TOP` moved**, which is the whole argument for growing the
window rather than re-laying the demo.
`the_gallery_above_the_band_is_where_it_was` asserts every non-band rect ends
above 720 and every band rect starts below it, and pins the progress bar at y 668
and the frame-rate readout at (60, 684) so "nothing moved" is a claim about two
numbers rather than about the absence of a failure.

### The three sizing numbers, and one of them is a floor

The demo asks the field for **420×64** against the widget's **240×44**, and the
keyboard for **44-tall keys** against the widget's **52**.

The field is the third control in a series the operator has already judged twice:
a 6-pixel slider track, then a 6-pixel scrollbar. Task 19's review asked for a
door so that a rejection could be answered without a code change, and
`TextInput::width` / `TextInput::height` are that door. **`the_size_is_settable_and
_size_reads_what_was_written` fails if `size()` goes back to reading two
constants**, which is the `.ai/NEVERAGAIN.md` entry *one sibling got the operator's
fix* made concrete.

The key height is the opposite case and the distinction is worth keeping: 44 is
the **touch floor**, the widget's 52 is a *default*, and the band was too short
for the default. So the demo asks for the floor and `the_demo_lowers_the_keys_to
_the_floor_and_never_below_it` says so.

### Three defects this task found, all fixed

1. **An `Rc` around the field made it un-rethemeable.** The first wiring shared
   `TextInput` behind an `Rc` so the keyboard's `on_key` callback could reach it.
   `TextInput::set_palette` takes `&mut self`, so an `Rc` with a live clone can
   never be re-themed — `Rc::get_mut` returns `None` and **the palette silently
   stayed the old one across a theme switch**. The demo was sharing state for the
   wrong reason: it now uses the repository's own pattern, the one `List`'s
   `on_item_click` uses — the callback writes a `Property<Option<KeyAction>>` and
   `Demo::offer_to` drains it — and owns the field plainly. Caught by
   `a_theme_switch_reaches_the_field_and_the_keyboard`, which is why that test
   exists.
2. **The palettes were read from the theme *after* `switch_to` had consumed it.**
   Every other widget in `toggle_theme` reads its palette from `new_theme` first,
   for a reason the first draft of this wiring missed: `switch_to` **animates the
   theme's own tokens**, so a palette read afterwards is the palette the theme is
   leaving, and every widget is re-aimed at what it already had. The transition
   goes nowhere and every test that does not wait for it stays green. Both are
   mutation-checked — reading from `self.theme` instead fails
   `a_theme_switch_reaches_the_field_and_the_keyboard`.
3. **A readout bound to an empty field printed an empty line.** The field's text
   readout is bound to `text`, so with nothing in the field it showed nothing at
   all — a readout that cannot be seen, and a driver could not tell a missing
   readout from an empty field. This is the third time this round's reasoning
   landed in the same place (`PLACEHOLDER_TEXT`, `NOTHING_SUBMITTED`), which is
   why `NOTHING_ENTERED` exists as a named constant rather than as a literal.

### Deliberate breaks — 38 run, 38 killed

| writer | mutations | killed |
|---|---|---|
| `input.rs` (the `Text` variant) | 4 | 4 |
| `keyboard.rs` (subagent) | 12 | 12 |
| `text_input.rs` (subagent, plus the two size setters added here) | 15 | 15 |
| the integrator: the demo wiring and the theme palettes | 7 | 7 |

**One further attempt was vacuous and is not counted as a result**: a
replacement that applied cleanly and changed no behaviour at all, which the runner
faithfully reported as a survivor. That is the third mechanism in the
`.ai/NEVERAGAIN.md` entry on mutation runners, caught for the second time in one
session; the real mutation it stood in for was written and killed.

### What is NOT claimed

- **No acceptance criterion was verified through injected input.** Every criterion
  that needs a tap, a key or a drag was verified **through the demo's own event
  path** — `MouseButtonDown`/`MouseButtonUp` into `Demo::handle_event`, the
  recogniser, `input::route` — and **through the pixels** for what is drawn.
  Whether XTEST injection reaches the window on this host is still unknown; see
  *Verifying a change that draws*, and no criterion is claimed on injection.
- **The blink was not seen mid-cycle in a capture.** It is covered by tests that
  drive explicit deltas, and the caret is visible in the capture because the
  field is **not** focused, which is the correct resting state.
- **The symbols page was not captured.** `set_page` is the widget's own API and
  its own tests; reaching it needs a key press on `?123`.
- **`cargo audit` was not run** — not installed on this host, the standing tool
  gate. **No dependency changed**, which is the thing it would have checked.

### What is on screen, and how it was got — no instrument

**One capture of the default state, by the stock method** in *Verifying a change
that draws*: `cargo build --release`, `setsid ./target/release/ui_demo >
/tmp/demo.log 2>&1 &`, the window id from `xwininfo -root -tree | rg '"roados
ui_demo"'`, and `magick import -window <id>`.

**Stated explicitly because the entry exists:** **no seed, no temporary
environment variable, and no rebuilt binary.** Nothing in `ui_demo`'s `main.rs`
was modified to produce this picture — `rg -c "PREVIEW|LIST_OFFSET|SEED"
ui/src/ui_demo/src/main.rs` is **0** — and the field is shown in its **resting,
unfocused** state, which is the state the demo opens in. Every one of the task
file's four clipped-or-instrumented acceptance criteria is therefore either
visible in this capture or covered by a test through the real event path, and
none of them is evidenced by a build that was altered to produce it.

What the capture shows, checked by cropping and scaling rather than by looking at
a whole window:

- **the whole gallery unchanged** — the three pads, the seven text-panel labels,
  the button band, the slider, the toggle, the progress bar, the list and both
  readouts are where they were before the window grew.
- **the field**, cropped at 250 %: rounded corners, a two-pixel border reading as
  an **outline** rather than a filled card, and the placeholder in the theme's
  muted grey.
- **the keyboard**, cropped at 300 %: every label **centred inside its own key**,
  which is the subagent's `0.6 × font_size` guess replaced by a measurement from
  the demo's real `TextMetrics`. `Shift`, `Bksp` and `Space` — the three widest —
  all fit inside their keys.
- **both readouts visible while empty** (`text: -`, `submitted: -`), which is
  defect 3 above and the reason the crop was taken.

`stderr` was empty and the process was confirmed alive by `pgrep` in the same
call as the capture, so the asset and the stand-in path were not involved.

### Two findings offered to the operator rather than acted on

- **`label::measure` should be public.** Raised by the `text_input.rs` subagent
  and not acted on here. `label`'s `measure` and `fit` are private, so a caret's
  per-character positions cannot be reached from another module; the subagent
  reported that `text_input.rs`'s own tests assert its character walk against its
  **own copy** of the `advance(ch) + letter_spacing` rule rather than against
  `label`, so a change to `measure` that stopped adding the trailing letter spacing
  would not be caught there. Three widgets now walk characters the same way and a
  fourth will. Exposing the two functions is a change to a module task 11
  shipped, and a reviewer should check the subagent's claim against the source
  before treating it as established.
- **The demo's keyboard label centring needed a real number.** `Keyboard` centres
  a keycap's label with one average `advance` and ships `0.6 × font_size` as a
  guess. The demo overwrites it with a measurement from its own `TextMetrics`,
  which is why the labels in the capture are centred on what they are drawn with.

## The frame rate, measured

**This section is what `.ai/agents/developer.md` § Phase 3, `.ai/workflows/task-sequence.md`
§ Gates and `.ai/tools/README.md` point at when they ask for a rate.** The
numbers and the mechanism are here; the rule that it must be produced after every
run lives with the agent that has to run it.

### What exists

| | |
|---|---|
| the meter | `ui/src/ui_demo/src/fps.rs` — `FrameRate`, ticked once per frame with the delta the loop already computes for the animation clocks. **It holds no clock**, which is what lets its ten tests run without a wall clock |
| the readout | a `Label` at `(60, 684)`, 400 px wide, at the foot of the window and clear of the text column and the button column. `fps 61, avg 60.8, worst 34 ms` — the current 500 ms window, the run's average, and the longest single frame |
| the report | one line on **stdout** when the demo stops: `roados-fps frames=498 duration_s=10.020 average_fps=49.7 worst_frame_ms=48.5 long_frames=1` |
| ending a run | `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo`, or `kill -TERM` — SDL installs SIGINT and SIGTERM handlers by default (`SDL/src/events/SDL_quit.c:117` and `:118`) and turns either into the quit event the loop already breaks on. **Both paths were measured**: a bounded run printed its report, and a `kill -TERM` of an 89-second run printed `frames=4409 duration_s=88.799 average_fps=49.7` |
| one command | `.ai/tools/fps-check.sh [seconds] [minimum-fps]` — builds release, runs, parses, and exits 1 on a missed floor **or on a run that produced no report at all** |

### The baseline, and the 60 fps target it now reaches

**Superseded on 2026-10-02 — the demo now runs at ~62 fps.** This section is kept
because the numbers in it are how the ceiling was found, and because the *debug*
row below is still the trap it was. What follows the historical table is the
change; see *The frame budget, and the 60 fps it reaches*.

`doc/ui/DEMO_APPLICATION.md` lists **60 FPS target** in scope and *"Smooth
animations and transitions — 60 FPS"* as a design principle. The demo does not
reach it, and before this change nobody could tell:

| build | run | average | worst frame | frames over 33 ms |
|---|---|---|---|---|
| **release** | `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo` | **49.7 fps** | 48.5 ms | 1 |
| release | the same binary, stopped with `kill -TERM` after 137 s | 50.0 fps | 46.8 ms | 1 |
| **debug** | `ROADOS_RUN_SECONDS=10 ./target/debug/ui_demo` | **34.2 fps** | 78.4 ms | 10 |

**Six release runs on this host, taken while writing this section, read 49.7,
50.0, 51.0, 52.5, 53.7 and 54.1 fps** — a spread of 4.4 fps on a machine that is
also running a browser and a compositor, and the reason the floor a check should
use was around **40** rather than the best number above. A floor at 49.7 would
fail on a build that has not changed.

**That floor is now too loose, and the reason is worth stating.** A loop paced by
a **frame budget** absorbs jitter in its work term: the wait is whatever is left,
so a slow frame is followed by a short wait rather than adding to a fixed 16 ms.
The consequence is that release no longer wanders — **61.9, 62.0 and 62.2 fps**
across three runs, a spread of **0.3 fps** where the unpaced loop spread 4.4. **A
release floor around 55 is now defensible** and a floor at 40 would miss a
regression from 62 to 45, which is a third of the budget gone. The **debug** row
above stays where it is, and a floor is a **release** floor: debug sits below any
sensible one by construction, which is exactly the trap recorded in `NEVERAGAIN.md`.

**Task 19 added two widgets and one text-event variant, and the rate did not
move: 50.0 fps** (`fps-check.sh 12 40`, 601 frames in 12.014 s, worst frame
63.2 ms, 1 frame over 33 ms). Two new widgets and a whole keyboard of ~50
rounded rectangles, drawn every frame, for nothing — which is what the batching
task 06 bought and is worth one sentence.

### The frame cost, measured against `HEAD` in the same session

The table above is not a comparison, and **a number from another session is not
one either**: this host is quieter now than it was, so the whole-machine figures
have moved and only an interleaved pair means anything.

`git worktree` at `3ddf5fa`, built release, three interleaved rounds of ten
seconds each, `utime + stime` from `/proc/<pid>/stat`:

| build | round 1 | round 2 | round 3 | median |
|---|---|---|---|---|
| `HEAD` (tasks 15–18 + the fps readout) | 19.1 % | 18.9 % | 18.9 % | **18.9 %** |
| task 19 (field + keyboard + band) | 21.4 % | 20.0 % | 19.3 % | **20.0 %** |

**About 1.1 points of a core, roughly 0.2 ms a frame at the loop's 16 ms pace.**
The *absolute* numbers are about half the 34.6 % and 43.6 % recorded above, which
is the host and not the code — and is exactly why the comparison is against
`HEAD` in the same session and not against that table.

**The ceiling was in the shape of the loop, not in the interface — and on
2026-10-02 that ceiling was removed.** This subsection is the record of the
finding; the paragraph immediately after it is what happened next.

The loop called `wait_event_timeout(EVENT_WAIT)` with `EVENT_WAIT = 16 ms` and
*then* drew, so the wait and the frame's own cost were **serialised**: a frame was
`16 ms + work` whatever the work was. Measured per-phase on 2026-10-02, a frame
was **19.9 ms — 50.2 fps** — of which the wait was 15.7 ms and the work 3.9 ms.
Even an infinitely fast frame would have given 62.5 fps. The operator's decision
on 2026-10-01 was **not** to change this in that round, *measure first*; these
are the numbers the decision was waiting for.

### The frame budget, and the 60 fps it reaches

The operator authorised the loop change on 2026-10-02, after being shown the
breakdown above. `EVENT_WAIT` is replaced by **`FRAME_BUDGET = 16_666_667 ns`**,
and the loop now waits only what is **left** of the budget after the previous
frame's work — so the work lands *inside* the frame rather than after it.

The arithmetic is extracted as **`frame_wait(spent)`** and the loop calls it,
rather than inlining the subtraction, for the reason `.ai/NEVERAGAIN.md` § *a
test of a helper cannot see a call site that stopped using it* gives: a helper
tested from one place and inlined in another has two places to be wrong and only
one under test.

| build | before | after |
|---|---|---|
| release | 50.2 fps | **61.9 / 62.0 / 62.2 fps** (three runs) |
| debug | 32.7 fps | unchanged — and **cannot** be rescued |

**Task 21 added a whole widget and 4x multisampling, and the rate did not
move.** Four release runs across that task read **61.7, 62.1, 62.3, 62.0**,
against **61.8 fps at `f8ba81e`** measured before any of it — inside the spread,
and the floor of 55 that these numbers defend still holds. The MSAA author's own
interleaved A/B — the attribute absent and present, alternating, restoring the
file between every build — put the two builds at **61.9 / 61.0 without** and
**61.8 / 61.7 with**, so **the measurement does not separate them** and the
honest statement is that nothing measurable moved, not that 4x MSAA is free. On a
fill-rate-bound target it would not be free, which is what the constant's own doc
says.

**The debug build is the operator's other half of this story**, and it is the
more misleading number: `ui/target/debug/ui_demo` had been rebuilt on the morning
of the report and `cargo run` defaults to debug, so *"sometimes the fps drops to
~30"* was a debug build, not a regression. An interleaved CPU comparison against
`b4a2db8` (pre-gauge) gave **136 / 144 / 135 jiffies before against 137 / 131 /
149 after** — task 20 is free, and the gauge draws *fewer* primitives than the
circle chain it replaced. **Frame rates are claims about a build**; this table
already had a debug row (34.2 fps) and it is the row nobody connects to the
number in front of them. Recorded in `NEVERAGAIN.md`.

**Pacing cannot rescue a frame that costs more than the budget.** Debug work is
18.7 ms against a 16.67 ms budget, so it runs at ~33 fps whatever the loop does.
That is not a loop defect, it is unoptimised Rust being slower than a frame.

**What is NOT fixed, and is the next thing that will bite.** The work is
**0.18 ms of UI and ~3.9 ms of GL submission** — 22 draw calls a frame against
Mesa on an Intel HD 530. Ruled out by measurement, not by assumption: it is
**not vsync** (`swap()` costs 0.18 ms and no `SDL_GL_SetSwapInterval` is ever
called), **not** opaque geometry being drawn twice (the unblended pre-pass is
0.07 ms), **not** uncached font metrics (`ascent` is 0.01 ms over 70 calls), and
the glyph atlas is uploaded only when dirty. So there is real headroom at 60 fps,
but a car UI with ten times the widgets would not have it, and the text pass is
where the next investigation should start.

**One honest caveat on the 62.** The loop is **not vsync-locked** — it free-runs
at its budget and reads slightly *over* 60. `FRAME_BUDGET` is a hard-coded rate,
not a query of the display's refresh, so a 30 Hz cluster panel would run it at
half refresh and waste half its budget. Reading the monitor's rate, or setting
the swap interval to match it, is the change that would fix that; the constant's
doc says so.

The 15 fps between the two builds is why `fps-check.sh` builds **release**: a
debug build's rate is a fact about unoptimised Rust, not about the interface, and
a regression measured against the wrong build is a regression against nothing.

### What is NOT claimed

- **Not vsync, and not a display rate.** There is no frame pacing and no
  `SDL_GL_SetSwapInterval`; the number is the loop's.
- **Not one sample.** Every number above is one run on a shared machine, and the
  six release runs span 4.4 fps. That is enough for a floor at 40 and not enough
  to quote 49.7 to a decimal as a property of the interface. Repeating a run
  three times before and three after is what the review's benchmark tables do, and
  what a regression claim should do.
- **Not a rendering verification.** A fast wrong picture passes it; a correct
  slow one fails it. Both halves are checked by the things in
  *Verifying a change that draws*.

### Two capture traps this feature walked into

**A pair of captures can be byte-identical while the demo is animating.** Two
`magick import` captures of the window two seconds apart came back `AE = 0`, which
reads as "the readout is not updating" — and the readout *was* updating. Six
captures half a second apart read `fps 50, avg 52.8`, `51, 52.5`, `50, 52.1`,
`51, 52.0`, `51, 52.0`, `50, 51.6`: two consecutive samples are the same string
when the average lands on the same tenth twice. **A live readout needs several
samples compared as a set, not a pair.** Recorded in `NEVERAGAIN`.

**The window's position is not the root's, and the id has to be re-read.** The
window id here was `0x120002f`; the standing capture method in *Verifying a
change that draws* is unchanged and was used as written.

## Tasks 15–18 — what was decided, and what the operator should look at

One changeset, four tasks. Everything below is the integrator's record; nothing
here has been through a review, so it is written to be reviewed.

### Gates skipped, named rather than glossed

`task-sequence.md` puts a review between the implementation and the operator's
commit, and step 5 is "**Operator commits. Per task, not per batch.**" The
operator decided on 2026-09-30 to batch all four into one changeset and to review
afterwards. The gates that goes around, recorded so they are decisions:

- **No self-review and no review at all.** Step 2 did not happen. Each task's
  `Review` cell above reads `none`, which is a fact and not a shrug.
- **One commit instead of four.** Step 5 was per task; this is one. There is
  **no per-task revert point**: a defect found in task 17 has to be fixed in a
  commit that also contains tasks 15, 16 and 18.
- **The per-task operator stop** (step 4) did not happen between tasks.
- What *did* hold: the whole verification suite was run after integration, and
  the four sub-tasks were file-isolated and independently tested, per
  `.ai/protocols/subagents.md` § *Implementation fan-out*.

**And then, on 2026-10-01, the operator committed the batch as `d7240c8`** — still
without the review step 2 asks for, which was the decision and remains one. What
that changes for the next reader is only *where* a review would start: a reviewer
dispatched against `d7240c8` needs no working tree, and the four tasks are still
one changeset with no per-task revert point.

**The cheapest way to review this is by file, not by task.** `texture.rs` and
`render.rs` are task 16's, `widgets/toggle.rs` is task 15's, and so on; each is
self-contained.

### The dependency, and it is the operator's

`sdl3`'s `image` feature, which pulls in `sdl3-image-sys` and builds **SDL_image
3.4.6** from vendored C source. Approved by the operator 2026-09-30 after being
shown the alternatives. `PRIMITIVES_ARCHITECTURE.md` § *Dependencies* now owns
the justification, the alternatives table, the licence (**zlib**, the same terms
the vendored zlib FreeType already pulls in is under) and the replacement cost.
The feature comment in `ui_core/Cargo.toml` says why it must not be removed.

Measured, not assumed: the aarch64 cross-build still passes **with no sysroot**,
the artifact is `AArch64`, and the binary's dynamic dependencies are unchanged —
SDL_image, like SDL3 and FreeType, is statically linked.

### Decisions the operator may want to reverse

1. **`ui_demo`'s window went from 1024×600 to 1280×720.** This is the one
   change that is visible in a screenshot and is not additive. It was necessary:
   task 14 recorded that the window was already full, and four more widgets — one
   of them a **100-row list needing a tall viewport** — do not fit in the free
   `788..1024 × 0..164` and `660..1024 × 396..600`. Nothing existing moved:
   every widget's position is absolute, and only the root and background
   constraints read `WINDOW`. The subagent verified that by building `HEAD` and
   the new demo and diffing the pixel runs of the pads card, the button row and
   the slider track — byte-identical. **Reversing it means shrinking something.**
2. **`Image::ImageFit::Cover` crops rather than overflows.** The quad *is* the
   node's rect and the source is cropped to its shape. The task file says
   "scale to cover bounds, preserve aspect ratio, clip overflow", and a crop is
   the same picture under a clip — which matters because `DrawCommand` carries no
   scissor state, so an overflowing quad could not be clipped at all. The
   subagent's argument, which is the reason to agree: with an overflowing quad
   the destination's aspect equals the source's, so "a crop matching the
   destination's aspect" is the whole image in every case.
3. **`List::scroll_offset` is reached through `List::scroll()`**, not as a field
   on `List`. Two properties would be two truths to write in step.
4. **`Progress`'s indeterminate leg is `2 × DurationFast`.** Requirement 4 says
   the duration comes from theme tokens and a single leg is 3.3 Hz of shimmer.
   One constant, `INDETERMINATE_LEG_MULTIPLIER`, if the operator wants
   Material's 1.4 s period.
5. **`Toggle`'s and `Progress`'s sizing are named constants, not theme tokens**,
   for the reason `slider.rs` and `button.rs` give, and each constant's doc says
   what would reverse it.
6. **`List` rows are read as row-local and translated by `List::paint`.** The
   demo skips the row handles in its draw order, and the widget's docs say so.
   A caller that paints a row at absolute coordinates and then also hands its
   handle to `draw_node` will draw it twice.

### A stale claim this round caught by measuring instead of repeating it

The *Resolved since task 01* section claimed that `ui_demo`'s "only dynamic
dependencies are `libm`, `libgcc_s`, `libc` and the loader". It is **false for the
native build**: `readelf -d` reports five, with **`libz.so.1`** among them,
because `freetype-sys`'s vendored `libpng.a` leaves `inflate`/`deflate`/`crc32`
undefined and links the *system* zlib on this host. It is not SDL_image's doing
— `nm -u` on the native `libSDL3_image.a` finds no undefined zlib symbol — and
it is **pre-existing**, confirmed by a `git worktree` build of `11f4134`. The
aarch64 build has no system zlib to find, builds the vendored one and links it,
so the target really does carry four.

Task 16's SDL_image adds no dynamic dependency on either target, which is the
claim worth making about it, and the justification in
`PRIMITIVES_ARCHITECTURE.md` says "no system library" — true, because a system
zlib being linked is not a *library SDL_image needs installed*, but the honest
wording is that SDL_image resolves zlib the same way FreeType's bundled libpng
already did. The stale sentence is corrected in place and marked superseded,
not deleted.

### The frame cost, measured

The `NEVERAGAIN` entry about a still of a 4 fps app looking exactly like a 60 fps
one applies to four new widgets and a new render pass, so the rate was measured
rather than inferred — `/proc/<pid>/stat` `utime + stime` over 10 seconds, both
builds on the same host:

| build | CPU over 10 s | of a core |
|---|---|---|
| `HEAD` (`11f4134`), the three-widget demo | 3.46 core-seconds | **34.6 %** |
| tasks 15–18, all four widgets | 4.36 core-seconds | **43.6 %** |

So the four widgets and the image pass cost **about 9 points of a core**, roughly
1.4 ms a frame at the loop's 16 ms pace. Reproduced independently: the demo
subagent measured 43.4 % against the same 34 % baseline before this round did.
It is not the 4 fps defect; it is also not free, and it is the number to re-take
if a fifth widget is added.

### What is on screen, and how it was got

One capture, of the **default state only**, taken by this round after
integration. Window id `0x100002f`, 1280×720, the process verified alive by
`pgrep` in the same call, and `stderr` empty — so the asset was found and the
stand-in path was not taken.

- **Image** (top right, 220×160): the PNG decoded through SDL_image, drawn with
  the source's crosshair and centre circle intact, **letterboxed top and bottom**
  so `Contain` is visibly a fit, and **rounded corners visibly clipped** — the
  corners show the window's background through a curve. That was checked by
  cropping and scaling 300 %, because a full-window still cannot show a corner
  clip, and because a filled corner instead of a discarded one is the exact
  failure `NEVERAGAIN` records for `Slider`.
- **Toggle** (664, 584): grey pill, white thumb hard left, its shadow ring,
  labelled `off, 0 changes`.
- **Progress** (664, 668): filled to exactly half of the track, both ends
  rounded, labelled `50%, determinate`.
- **List** (1000, 396): `item 0` … `item 9` on a Surface panel, a scrollbar thumb
  one tenth of the groove at the top — which is right for 100 rows in a ten-row
  viewport — and the readout `first 0, live 10, free 0, tap -`, so **the
  virtualisation is visible on screen** rather than only in a test.

**No injected input reached the app in this round**, and the control that says so
is task 12's button: `XTestFakeMotionEvent` returned success and the pointer did
not move, so a synthetic click landed outside the window and the click counter
was byte-identical before and after. Per the standing rule, **no rebuilt binary
with a seed was used** to manufacture the states that need a key — the `A capture
whose only route was instrumented` entry is why. Every criterion above that needs
a pointer or a key is in the `AC waived` column and is not claimed as verified.

### A defect this round found in its own work, and fixed

**A deleted `#[test]` attribute was a green suite with a hole in it.** Adding a
test by anchoring on a `fn` line rather than on its attribute left the attribute
attached to the new function and **unregistered the old one** — so
`the_four_new_widgets_follow_the_theme_switch` silently stopped running, and the
suite stayed green at every step. It was caught because the count went 97 → 96
and 96 was also the count *before* the new test: two errors cancelling. Clippy's
`dead_code` found it too, after the edit had already been verified once. Recorded
in `.ai/NEVERAGAIN.md`.

**One test expectation was wrong on its first run, and the code was right.** A
new test asserted that the list's scrollbar has *left* the dark theme's colour
one 10 ms frame into a 150 ms switch. It had not: `EasingStandard` is `EaseInOut`,
which starts quadratically, and `Color::interpolate` rounds each channel to a
`u8` — so the first frame writes a value **byte-identical** to the one it started
from. Measured, not reasoned: 158, 158, 157, 155, 152, 149 over six frames. The
test now asserts those measured numbers, and its point is the mid-transition
value, which is what distinguishes an animated palette from a property write.

### A defect the operator found on screen, and what it was

Reported 2026-09-30: *"The list/scroll doesn't work. I can't drag down the
slider, it doesn't move when clicking and dragging. It moves when I use my
mouse's wheel. But then, I see some artifacts like the position of the first
items jumps by one row up and down, kinda glitch."*

**One real defect, and one thing that is not a defect.**

**The artefact was real, and no test could see it.** Rows are drawn at
`viewport.y + index * item_height - offset`, so at any offset that is not a whole
number of rows **the top and bottom rows are drawn outside the viewport by
design** — that is what makes a scroll smooth instead of a row popping in — and
**nothing clipped them**. Measured before the fix: at offsets 10, 20 and 48 the
list recorded exactly one text run *above its own top edge* (`y = 384` against a
list whose top is `396`). It was drawn on the window background above the panel,
and it vanished as it scrolled away. That is the "first item jumps about".

It could not be fixed in the widget, and the reason is worth keeping: a
`DrawCommand::Text` carries an `x`, a `y` and a string and **no width**, so
nothing outside the text pipeline can say how far a run reaches, and
`scroll::clip_commands` can only drop a command that is *wholly* outside. Half a
row needs the GPU.

**The fix discharges a deferral rather than inventing a mechanism.**
`doc/ui/IMPLEMENTATION_STATE.md` § *Deviations* recorded that a per-node clip is
"deferred to the task that draws within a node's own bounds". That task is now.
`Batch` carries a `clip: Option<Rect>` — **not** in `BatchKey`, because a clip is
not a property of the material and keying on it would split one list into one
batch per command — and `Renderer::end_frame` sets the scissor when the clip
*changes between batches*, through `Renderer::apply_clip`. The old obstruction is
named in the code: a scissor set while recording was applied at the wrong moment
and the last one won for the whole frame. `begin_frame` clears the renderer's
idea of what is applied, because it disables the scissor directly and the next
clipped batch would otherwise be skipped as "unchanged". `Renderer::draw_node`
still exists and is `draw_node_clipped(.., None)`; the demo's `Demo::frame_clips`
returns one clip per node and the list is the only one that is not `None`.

**Verified on screen, and the capture is what proves it.** The method is the one
in *Verifying a change that draws*, plus a **temporary seed in `Demo::new`**
reading `LIST_OFFSET` from the environment — the same six-line technique task 14
used and recorded, needed because no injected event reaches the app. **The seed
is reverted**: `rg -c "LIST_PROBE|LIST_OFFSET" ui/src/ui_demo/src/main.rs` is
**0**. Before the fix, captures at offsets 0/48/96 showed whole evenly-spaced
rows. After it, offset 48 shows **`item 1` sliced at the top edge and `item 5`
sliced at the bottom**, with the 41 px of window above the panel clean — which is
the half-row being cut where it should be cut.

**Dragging is a convention, not a fault, and the demo is unchanged by it.**
`Scroll::on_event` maps a `Drag` to `scroll_by(rect, -delta.y)`: drag the content
**down** to reveal what is **above**, which is the direct-manipulation
convention every native scroll view uses. At offset 0 the offset cannot go below
zero, so **dragging down at the top does nothing, and must** — you cannot scroll
past the start. Dragging **up** works, and that was verified through the demo's
real event path (`MouseButtonDown` / `MouseMotion` / `MouseButtonUp` into
`Demo::handle_event`, not a hand-built `InputEvent`): from offset 400, 60 px up
gives 460 and 120 px up gives 520. The wheel works in both directions because
its sign is taken from the event and is not clamped by the gesture's own start.

**What this costs the operator's workflow, stated plainly:** the demo still has
no pointer-free way to move the list — arrow keys reach a **focused** scroll, and
`Tab` reaches the list — so a pointer is not required to scroll, but it is
required to *reach* it first.

### The fourth operator report: the scrollbar is a hairline, and it lagged

Reported 2026-10-01, against the list: *"Is too narrow, I have issues with
pointing on it with my mouse, so doing that on tablet with a finger is
impossible"* and *"when I click it and drag - it doesn't follow my mouse cursor
exactly, it's like something was keeping it from moving faster."* Both halves are
the scrollbar, which is why this section is about one widget and two numbers.

**The width was a missing door, not a wrong number.** `Scroll` drew a **6-pixel**
bar — the same six the operator rejected for the slider's track a day earlier,
which `ui_demo` had answered with `SLIDER_TRACK_THICKNESS = 12.0` through the
widget's `set_track_thickness`. **`Scroll` had no equivalent setter**, so the demo
had nothing to call and the report was true for a reason the slider's was not.
`Scroll::set_thickness` now exists, the demo asks for **12**, and the widget's
constant is still 6 for the reason its own doc gives: it is a documented baseline,
and the operator's number is the operator's call. `List::set_scrollbar_thickness`
is the matching door, because `List::scroll()` hands out a `&Scroll` and the
setter needs a `&mut` — the same argument `List::set_palette` already makes.

**The lag was not a tuning problem. There was no thumb dragging at all.**
`Scroll::on_event` handled **every** `Drag` by scrolling the content by the
delta, wherever the pointer was — the scrollbar was drawn geometry and nothing
else. So the content tracked the finger 1:1 in *content* pixels while the thumb
travelled a *shorter* run, and on the demo's own numbers (a 280 viewport over
2 800 of rows: a 28-tall thumb on a run of **252**, against a maximum offset of
**2 520**) the thumb moved **a tenth** of the distance the cursor did. No tuning
of that drag could have fixed it, because the drag was the wrong mapping.

The fix tracks the pointer's **position** against the groove, which is 1:1 by
construction, and records **where inside the thumb** the pointer landed, so the
thumb does not jump its own width sideways on the first frame of the drag.
`grab_thumb` is a **call, not a match arm**, because the gesture recogniser has no
press to give: it reports a `Tap` on the *release* and a `Drag` only after the
pointer has moved. That is the same reason `Slider::dragging` is written by its
caller, and the demo now calls `grab_thumb` from `MouseButtonDown`/`FingerDown`
and `release_thumb` from the matching releases.

**A third defect found while fixing these, and fixed with them.** `List::item_at`
tested the whole viewport rect, so a tap on the scrollbar named **whichever row
was behind it** — 12 pixels of a 270-wide list silently activating row 0. A
scrollbar is a control drawn over the rows, so `item_at` now asks the embedded
`Scroll` for the strip (`scrollbar_rect`) rather than recomputing it: a second copy
of the scrollbar's geometry in the list would be a second thing to keep in step
with the thickness the operator just changed, and it would have been wrong the
moment they changed it again.

**Verified.** The width was verified **on screen**, because that is a claim about
what is drawn: `magick import -window <id>` on the running demo measures the
thumb at **12 pixels** wide at abs x 1256..1267 and the groove at 12 as well,
with the 28-pixel thumb starting at the list's top edge as offset zero says it
must. The **drag was not verified by a capture**, and could not be: a still
proves what is drawn and never how it moves, and pointer injection is unreliable
on this host (*Verifying a change that draws*). It is verified instead by
`a_drag_on_the_scrollbars_thumb_moves_the_thumb_and_not_the_drags_delta`, which
drives a real `FingerDown`/`FingerMotion`/`FingerUp` through `Demo::handle_event`
and the recogniser — **400** pixels of offset for a 40-pixel drag of a run of 252,
against the **40** the delta path gave.

**Seven mutations, all killed**, and two of them are the two reports themselves:
ignoring the recorded grab offset, and disabling the grabbed branch entirely (the
old behaviour) which is caught by three unit tests and the end-to-end demo test.
The other five: `set_thickness` as a no-op, the scrollbar exclusion removed from
`item_at`, `release_thumb` as a no-op, `run` and `max` transposed, and the clamp
at the two ends removed.

**What was NOT changed.** A press on the **empty groove** still does nothing
rather than paging or jumping the thumb there — a conventional behaviour, and one
the operator did not ask for. The bar is 12 wide rather than the 40-plus a
fingertip covers; a *wider invisible hit strip* around a bar drawn at 12 was
offered and declined in favour of widening the bar itself, so a tablet user is
better served than before and not yet served ideally.

### The third operator report: both directions inverted, and the slider resized

Reported 2026-09-30: *"when I drag down the slider, the list goes up — this
direction has to be reversed"* and *"the slider is very narrow, can't imagine how
I could use it in a car with my finger"*.

**The diagnostic settled the first half in one reading.** A temporary probe in
`Demo::handle_event` appended every event to `/tmp/roados-input.log`, and the
operator reproduced against it. **1223 lines, three `MouseButtonDown` events, all
of them at x ≈ 1265 — inside the list, which spans 1000..1270. Zero presses
anywhere near the slider at 664..904.** So there was no slider-drag defect to find:
the control had not been touched, and *"the slider"* meant the list. The probe is
removed (`rg -c "INPUT_PROBE" ui/src/ui_demo/src/main.rs` is **0**). What it did
establish is that the pointer arrives with `state=1` set — **SDL does populate
`mousestate`** (`SDL/src/events/SDL_mouse.c:839`,
`event.motion.state = SDL_GetMouseButtonState(...)`) — so the recogniser's drag
path is sound on real hardware, not only in the synthetic events the tests build.

**Both directions are now inverted, and they are one rule.** The wheel was
inverted first and the drag was left following the finger, which put one control
answering two directions — and that is the state the operator reported. The drag
was then inverted too, so **down is later for both**: a finger travelling down the
screen advances the list, and so does a wheel rolled towards the user.

The rule is now **`scroll::gesture_delta`**, and it is the only copy. This was not
the shape it started in: the sign was written out in both arms of
`Scroll::on_event` and restated in **eleven tests**, so inverting the convention
meant finding and rewriting eleven places across two modules — four of them with
the old direction in the *test's name*, which cannot be silently flipped without
the name and the body drifting apart. Now there is one named function, a doctest
that states the convention, and **one test**
(`down_is_later_and_a_wheel_agrees_with_a_finger`) that pins it as a fact about
numbers it chooses. Every other test derives its expectation from the arithmetic,
which is direction-agnostic. `wheel_delta` sits beside it and holds the one
asymmetry: **only the wheel is negated**, because SDL's positive `y` is *away from*
the user and so is *up* the document, while a finger's positive `y` really is
travelling down.

Two tests written the day before to pin that the wheel and the drag *disagree*
were **rewritten, not deleted**, to pin that they agree — the matched-magnitude
form, because *a 48 px notch and a 48 px drag now land in the same place* is the
sharpest statement of the new invariant.

### The slider is finger-sized, and the column is what stopped it

The widget's defaults are a **6-pixel** track and a 24-pixel knob. The demo now
asks for **12 and 18** through the widget's own setters — a 36-pixel knob and a
**52-tall** node, which is the hit target, against 44 before. It is also **300
wide rather than 240**, because a finger wants a longer swipe. The widget's own
constants are unchanged: task 14's record says each states what would reverse it,
and this is the setter doing what it exists for, which also puts the two numbers
side by side.

**Why not larger, and this is the ceiling.** A 22 radius gives a 44-pixel knob and
a 62-tall node, and the right-hand column then stops fitting:
`no_two_placed_rects_overlap` reported the toggle against the progress bar's
readout. Six controls, of which three are now finger-sized, need more than the
**324 pixels** between the button row at y = 396 and the bottom of the window.
**A bigger slider means moving the progress bar out of that column** — there is
free space under the image at x 788..1000 — and that is the operator's trade to
make, not an agent's. The column is currently: buttons 396..440, counter 456..480,
slider 496..548, its readout 560..584, toggle 592..636, the progress bar's readout
640..664, the bar 668..712.

Two constants had to follow the slider and one had to stop repeating a number:
`SLIDER_READOUT_DROP` 52 → 64, `TOGGLE_ORIGIN` 584 → 592, and
`TOGGLE_READOUT_ORIGIN` was a **literal 592** that stayed behind at the old
position and collided with the slider's new readout — it is now derived from
`TOGGLE_ORIGIN`. Three geometry tests that quoted the old thumb run (216 px of
240 minus 24) were re-derived from the new one (264 px of 300 minus 36), and one
expected value changed with it: a 40-pixel drag from the middle lands at **65**,
not 70, because 40 of 264 is 15 points and 40 of 216 was 20.

**Verified on screen.** The track is visibly a bar rather than a hairline and the
knob is visibly larger; the column is clear from the button row to the progress
bar with the readout lines between them.

### What was NOT changed, and why

**The toggle and the progress bar still have 6-pixel tracks.** The same complaint
is waiting for both of them and neither was in the report. The toggle's pill is 28
tall and the progress bar's track is 6, and fixing them needs the same column
space this round ran out of — which is the argument for moving the progress bar
rather than for enlarging one control at a time.

**`Slider`'s hit target is its node, not its track.** Every position test measures
the node's laid-out rect, so the enlarged slider answers a press anywhere in a
52-pixel band rather than only on the 12-pixel line.

### The second operator report: the wheel, and the slider

Reported 2026-09-30, after the clipping fix: *"the mouse wheel seems to work in
reversed direction"* and *"clicking and dragging the slider doesn't work — this
is the main functionality as in a car you don't have mouse"*.

**The slider works, and there were no mouse tests for it at all.** Every slider
drag test in the demo used `drag_on`, which builds a
`FingerDown`/`FingerMotion`/`FingerUp` sequence — **not a mouse**. So the exact
route the operator used had zero coverage, and "the tests pass" said nothing
about it. Four tests now cover it through the real event path:
`MouseButtonDown` / `MouseMotion` / `MouseButtonUp` into `Demo::handle_event`,
with nanosecond stamps and SDL's real button mask
(`SDL_BUTTON_LEFT = 1`, and the binding computes `1 << (button as u32 - 1)`, so
bit 0). Measured behaviour:

| motion | value | thumb |
|---|---|---|
| press at x+10 | 0 | 0 |
| x+40 | 15 | 15 |
| x+90 | 35 | 35 |
| x+140 | 60 | 60 |
| x+190 | 80 | 80 |
| after release + a frame | 80 | 80 |

and a 900 ms hold before moving changes nothing and then drags normally, which is
the case where a long press could plausibly have eaten it.

**The demo has two independent routes for a slider drag, and only one is
load-bearing.** Disabling `Demo::slider_dragging`'s fast path leaves every test
green, because the drag then falls through to the positional chain, finds the
slider under the pointer and is handled there. That is a robustness property —
a drag that starts on the slider and wanders off is what the fast path is *for*,
and `a_drag_past_the_end_of_the_slider_clamps_at_its_maximum` covers that — but it
also means a mutation of the flag is unobservable, and worth knowing before
anyone writes a test that claims to cover it.

**The wheel's direction is a product decision this repository has not made, and
it is recorded here rather than changed.** What is implemented is
content-follows-the-gesture: a `Drag` and a wheel notch both move the offset so
that **the content moves the way the gesture did**, which is what every
touch-first interface does and the only convention that makes a wheel and a
finger agree. So SDL's positive `y` (the wheel rolling *away* from the user)
takes the content up and shows later rows.

**SDL already normalises that sign, and it is worth writing down before anyone
"fixes" it.** SDL3 documents a mouse wheel's `y` as positive for scrolling away
from the user and reports `MouseWheelDirection::Flipped` to say the *device* is
inverted relative to that. `GestureRecognizer` drops the `direction` field, and
that is correct: **negating on `Flipped` would double the inversion.** The
information is not lost for any purpose that matters here.

**The wheel was inverted on the operator's decision, 2026-09-30.** The list now
uses the **scrollbar convention** — SDL reports the wheel rolling *towards* the
user as a negative `y`, and that is the notch that advances *down* the document
— while the **drag keeps the touch convention**, so the content follows the
finger. A `Drag` and a `Scroll` on the same axis therefore mean opposite things,
which is now stated at the line that decides it and pinned by
`a_wheel_notch_and_a_drag_travel_opposite_ways_on_one_scroll` and
`a_drag_down_and_a_wheel_notch_move_the_list_opposite_ways`. **The drag was
deliberately not touched**: the operator's report named the wheel, and inverting
both would break the agreement between a wheel and a finger on one control.

Six tests across `scroll.rs` and `list.rs` encoded the old sign and were rewritten
rather than flipped — each carries the date and the reason, because a flipped
sign with no explanation is indistinguishable from a typo. Reverting the
inversion fails all six; the restore was verified byte-identical.

**The consequence, which the operator should confirm:** `Slider`'s wheel keeps the
*other* convention, so the wheel now goes **down** the list and **up** the
slider. They are two different controls and the operator named one of them, but
the asymmetry is real and is one line in `Slider::on_event` if it should change
too.

**What the operator is most likely to have hit was an asymmetry at the top of the
list, not a reversal.** From offset 0 the offset cannot go below zero, so **one
notch direction is a no-op** — measured: `dy = +1` moves the offset 0 → 48 and
`dy = -1` leaves it at 0. A caller who scrolls *down* the list first sees a list
that ignores the wheel, exactly as a caller who drags down sees a list that does
not move. Both complaints have the same root: **at the top of a list, the
direction most people try first is the one that cannot work.** Two tests now pin
both halves, and the fix for the confusion is the operator's to choose:

- leave the convention and make the demo start the list part way down, so both
  directions do something on launch; or
- invert the wheel only (not the drag), which breaks agreement between a wheel
  and a finger on the same control.

### Two test defects this fix round found in itself

**An assertion that could not fail.** The first version of the regression test
looped over every other node and asserted `clip_for(handle, list, None) == None`
— passing `None` in and comparing against `None`. A mutation that clipped
*everything* sailed through it. The loop now passes each node's **own real
rect**, so clipping everything fails.

**A test of a helper that could not see its call site.** The test called
`Demo::clip_for(..)`; a mutation that inlined the logic into the frame loop
instead — inverting the rule, so the *list* is the one thing not clipped — passed
every test. The loop and the tests now both read `Demo::frame_clips`, so there is
no second place to put the logic.

Both were found by a mutation runner that **aborts when the mutation does not
apply**: an earlier runner reported two such runs as survivors, because
`cargo fmt` had expanded the one-line text the replacement was written against.
A mutation that did not apply is no result at all, and reporting it as a survivor
is worse than useless. All three mutations are now caught — the revert, clip
everything, and the inversion — with the restore verified byte-identical.
Recorded in `.ai/NEVERAGAIN.md`.

### The integration gap the demo subagent found, and the fix

`List` owns its `Scroll`, `List::scroll()` hands out `&Scroll`, and
`Scroll::set_palette` takes `&mut self` — so **a caller holding a `List` had no
route at all to a themed scrollbar**, and the demo reached past the widget and
wrote two properties. That themes the scrollbar but takes the animation out of a
theme switch, because there is no palette for the transition to aim at.

Fixed by adding `List::set_palette`, which forwards. The demo now uses it plus
`scroll().animate_to_state`, so the scrollbar animates with everything else, and
`the_lists_scrollbar_animates_to_the_new_theme_rather_than_jumping` pins the
mid-transition value. That test was mutation-checked: reverting to the two direct
property writes fails it.

## Task 14 — what it decided, and what it found

Decisions the task file left open or contradicted, and where each one is recorded
in the code. None of them belong to the operator.

- **`Callback` is `widgets::Callback<T>`, and `widgets/mod.rs` owns it.** The
  button's own doc said it would move when a second widget needed one, and task 14
  is that second widget: a slider's `on_change` carries the value it moved to, and
  a type that cannot carry a payload could not be it. `button::Callback` is a
  **type alias** for `Callback<()>`, so `button::Callback` still names what it
  named and `Callback::new(move || …)` still reads as it did.
- **The two constructors are named apart, which is what made the alias possible.**
  A closure of no arguments does not implement `Fn(())` and no bound can make it,
  so one `new` cannot serve both shapes. `Callback::new` is the payload-free one
  and `Callback::from_fn` is the general one. Had they been one function, every
  button's click handler in the repository would have become `move |_|`.
- **`call` takes the payload, so the button calls `call(())`.** The only
  source-level consequence, and it is inside `button.rs`; `Button::activate` is
  what a caller uses.
- **`Slider::on_event` takes the slider's `rect` as an argument.** A node cannot
  reach the arena that holds it — the same reason `Container::add_child` takes
  `&mut Arena` — and a slider has to know where along its own track a pointer is.
  `Button::on_event` does not need one: a tap anywhere on a button is a click.
- **`dragging` is a property the caller writes from a press and a release**, and
  `on_event` does not touch it. This is `Button::pressed`'s arrangement, for the
  same reason: the gesture recogniser reports a tap on the *release*, so the
  pressed appearance has to be on screen while the pointer is down, which is
  before any tap exists — and a short drag produces no event at all on release, so
  a flag maintained from the event stream would stay set.
- **`on_change` fires when the value *moved*, and only for an interaction.** A
  finger resting past the end of the track moves nothing and reports nothing; a
  caller writing `value` itself is itself and reads the property.
- **`min`, `max`, `step`, `orientation` and the three sizes are plain fields
  behind `&mut self` setters**, not properties. They are the *mapping* rather than
  the appearance, nothing animates a slider's minimum, and a caller writing them
  would need the setters anyway to re-clamp the value against them. The colours
  and the two animated numbers — the thumb's value and its scale — are
  properties, and they are what `snap_to_state`, `animate_to_state` and `paint`
  read.
- **`set_range` and `set_step` move the thumb at once rather than animating it.** A
  range or a grid that has just changed has no transition to run, and a thumb
  animating into a track that has not been drawn yet is a frame of nonsense.
- **The vertical case is in the geometry and the keyboard, not in the demo.** The
  task file lists `orientation` as a property and puts "Vertical slider" under
  *Out of Scope* with the note "(add orientation support)", which is read as: the
  vertical slider as a shipped feature is out of scope, and the support for it is
  what this task adds. `Orientation` therefore exists, is honoured by
  `thumb_center`, `value_at`, `track_rect`, the fill's side and the key and scroll
  mapping, and the demo shows **one** horizontal slider — the window's room below
  the counter is 360 by 144 and a second control there is a collision waiting to
  happen, which tasks 12 and 13 each found by eye. `a_vertical_slider_is_larger_at
  _the_top_than_at_the_bottom` and `a_vertical_slider_takes_up_and_down_and
  _ignores_left_and_right` are what make the enum non-decorative.
- **The sizing is named constants, not theme tokens**, for the reason
  `MIN_TOUCH_TARGET` and `THEME_SPACING_SM` in `button.rs` give: the theme has no
  token for a slider's parts, and adding one would change `ThemeToken::all`, both
  theme tables, the token count and the animation every token takes part in during
  a switch, for values a switch does not change. Each constant says what would
  reverse it.
- **The focus ring is not in the task file and is drawn anyway.** The keyboard
  criterion needs an on-screen indication of where arrows will land, and
  `Button`'s ring is the precedent; without it, focus on a slider is invisible. It
  is drawn **around the track**, not around the node — see the defects below.
- **The gamepad half of requirement 3 is half met, and the half that is not is
  recorded rather than faked.** Gamepad *buttons* are already mapped into `Key`,
  so the d-pad adjusts the value with no change to `input.rs` and is tested. The
  **left stick is not implemented**: `input.rs` maps exactly one gamepad axis,
  `STEERING_WHEEL_SCROLL_AXIS = Axis::RightX`, and nothing maps `Axis::LeftX` or
  `LeftY` to any event. What the widget does do is handle
  `Scroll { delta }` — the event an axis *would* produce — using the component
  along its own axis and the direction alone, and `ui_demo` gives the focused
  control first refusal on a positionless `Scroll` so the steering wheel's axis
  already reaches a focused slider. Adding `LeftX → Scroll` is a change to a
  module task 10 shipped, and it is not made here; it is one `match` arm in
  `GestureRecognizer::process` whenever a stick is wanted.
- **`Motion` is still `button::Motion`,** imported by `slider.rs`. It is the
  theme's fast duration and standard curve and a slider's press follows the same
  motion as a button's, but it is one line of import rather than a second move
  that would touch the button's doctests, its tests and the demo's import. It
  moves when a module of its own is warranted.
- **The demo's slider is a fourth child of the band**, not a new panel, so
  `every_parent_in_the_demo_is_a_container_widget` still counts six containers and
  still fails if a bare parent reappears. It sits at `(664, 496)` — the band's own
  column, below the click counter — with its readout `SLIDER_READOUT_DROP` below
  it, and a test asserts it is clear of the counter, the text column and both
  edges of the window.
- **`0` and `1` put the slider at its two ends**, which is the demo's
  *changes programmatically* case: the value property is written directly and the
  thumb is carried there, so the one thing a drag cannot show is on screen. The
  readout's count of adjustments deliberately does **not** move when they are
  pressed, which is the visible difference between the two paths.
- **One existing demo test grew a step.**
  `tab_steps_over_the_disabled_button` walked three `Tab` presses and expected to
  wrap; the slider is a fourth stop, so it now walks four and still asserts that
  the disabled button is never visited.
- **A value whose step does not divide its range cannot reach its top.** "Snaps to
  the nearest step" says nothing about the ends being on the grid: a slider from 0
  to 1 with a step of 0.3 snaps to 0.9 at the top, and that is what the word
  means. The demo's step is 5 of 100, which does divide it.

### Defects found while implementing, and fixed

1. **The focus ring was a white card, not an outline.** `Slider::paint` drew the
   ring as a rounded rectangle grown around the slider's whole rect, which is what
   the button's ring is — and the button works only because it draws its background
   over the ring's middle. A slider has no background to draw with, so a focused
   slider rendered as a 240×44 white panel with a track lying on it. **Found by
   looking at the pixels**, after a unit test had been written and passed that
   asserted the ring's rect: a filled rounded rectangle of the right size, colour
   and place satisfies every draw-command assertion there is. Fixed by drawing the
   ring around the **track** and letting the track cover it, which is the same
   trick the button uses; the test now asserts both the grown rect and the fact
   that the track is recorded after it, and a mutation that puts the ring back
   around the node fails it. Recorded in `.ai/NEVERAGAIN.md`.
2. **Every slider drawn away from the origin was broken.** `Slider::travel`
   computed the thumb's run as `extent - origin - radius * 2`, where `extent` was
   already a length. All 53 unit tests laid their slider out at `(0, 0)`, where
   subtracting the origin subtracts nothing, and every one passed; the run came
   out negative for any real position, both ends pinned to the slider's centre,
   and every pointer position read as the minimum. **Found by the demo**, whose
   slider is at `(664, 496)`, within an hour of the widget landing. Fixed, and
   `a_slider_away_from_the_origin_maps_positions_to_the_same_values` was added
   with a fixture that is not at the origin — the fixture the suite was missing is
   recorded in `.ai/NEVERAGAIN.md`.

## Task 13 — what it decided, and what it found

Decisions the task file left open, and where each one is recorded in the code.
The three that belong to the operator are in *Ratified by the operator* and are
not restated here.

- **`Container` holds no `layout_mode` field and no `padding` field.** The task
  file lists both as properties, and both live on the node's `LayoutState`
  instead, which is what the layout pass reads. A second copy on the widget would
  be a value nothing reads, and a padding only the container held would leave
  every other node's children laid out at the unpadded origin. The widget's
  `set_mode`, `set_padding` and `set_flex_config` are the doors to them; the
  size of a container is set the way every other node is sized, through
  `layout_mut()`.
- **`background` and `border_radius` are plain `Property`s, not
  `Option<Property<…>>`.** The task file's optionality is read as a transparent
  default: `Color::new(0, 0, 0, 0)` is how this repository says "not drawn", and
  it is the only shape a *themed* background can take, because a colour that
  follows the theme is a `Property::bind` and a bind cannot produce an `Option`.
  Requirement 4 — "background color animates with theme changes" — is
  unsatisfiable with an `Option` under the property graph as it stands.
  `Container::paint` records **no command at all** while the background is
  transparent, so a container with no background costs nothing.
- **The background is behind the children by tree order, not by drawing it.**
  A node's commands are recorded parent first, and the demo's `order` is a
  pre-order walk, so a container's rect is recorded before its children's. Two
  tests pin the order, because task 12's review found that moving the label above
  the press overlay passed every assertion in that module.
- **The task file's requirement 2 is stale about `LayoutMode`.** What exists is
  `Flex { direction, wrap, flex_config }`, `Grid { columns }`, `Stack` and
  `Absolute`, with `LayoutMode::row()` and `::column()` as constructors. `wrap`
  is accepted and **not honoured** — documented at `layout.rs` and arriving with
  the list widget — and `Grid` lays out no children and reports no rects. Neither
  is implemented here: neither is a container, and both are already recorded
  where they were shipped.
- **`layout_constraints` did not gain a `padding` parameter.** It is a
  measurement helper with no caller outside `layout.rs`'s own tests, and its
  signature is task 07's public API. Its doc now says what to pass instead: the
  padded box is `padding.inset(constraints)`, which is why `Padding::inset` is
  public.
- **The two bare parent nodes in the demo became `Container`s too.** The text
  panel and the button band were `node::create` with a mode and children — the
  same thing the retired helper did, written out. Leaving them would have kept
  two ways to build a parent in one repository, and
  `every_parent_in_the_demo_is_a_container_widget` fails if one reappears.
- **The card is the row of pads, and it bleeds off two edges of the window.**
  The demo's window is full: the text panel's rect is 900×380 at the origin, so a
  card there would cover the pads, and the button band is the window, so a card
  there would cover everything. The pads' row is the one container whose rect
  covers nothing but its own children. Its left and top edges are the window's
  own corner, because a `Stack` places every child at the origin and the pads
  are already flush there, so the padding shows on the right and bottom only.
  Measured on screen: the card is 788×164 with the pads inset by exactly 12, and
  `CARD_PADDING` is 12 rather than 16 because 16 would have put the card's bottom
  edge two pixels into the first label's line box.
- **`ui_demo`'s click counter has a 0×0 rect, and that is pre-existing.** It is
  an `Absolute` child with no declared constraints, so the pass measures its
  content — and a leaf's content is nothing. The text still draws, because
  `Label::paint` lays out from its own options rather than from the rect's size.
  Not touched: it is task 12's code and no criterion here depends on it.
- **The `BUTTON_ORIGIN` comment says the pads are centred, and they are not.**
  It claims they "reach from x = 130 to x = 894"; they are laid out at 0, 272 and
  544, because the row is a `Stack` child and a `Stack` sizes a child from its
  own constraints, which measures it to its content — 764 wide. The comment's
  conclusion still holds on the other axis: the band is below the pads at
  y = 396, which is what `the_band_does_not_overlap_a_pad` checks. Left as found
  rather than corrected in passing.

### The benchmark, before and after

`layout_walk_cost` is a best-of-N on a shared host. **These figures are the
reviewer's, not the developer's** — the first version of this table reported +6 %
and +10 % and overstated the regression by roughly 5–10×, because one baseline
sample of 164 ms sat far below its own run and the comparison was not
interleaved. Three interleaved rounds of three, median of each:

| shape | before (`9973185`) | after | delta |
|---|---|---|---|
| flat-2041 clean pass | 9 739 ns (9079–11100) | 8 667 ns (8653–8765) | **−11 %** |
| depth-1000 declaring, one dirty leaf | 4.26 ms (4.04–4.40) | 4.34 ms (4.33–4.49) | +1.7 % |
| depth-1000 bare, cold pass | 187 ms (186.2–192.5) | 189 ms (188.5–188.9) | +1.2 % |

**The conclusion the developer reached is right and the numbers were not:** the
move is inside the noise rather than outside it. The clean pass is very slightly
*faster* and the two deep-chain shapes are up about 1 %, and the O(n·d) argument
in *Deviations* is about ratios, which do not move: 22.9× before and 23.0× after
on the declaring chain, 1.0× on the bare one. The developer's 14 % baseline
spread does not reproduce either; it measures 9 %.

The likeliest real cost is the 16 bytes `Padding` adds to `LayoutState`, which
`visit` reads once per node. A guard skipping the inset for the zero case was
tried, measured, and reverted rather than kept as speculative complexity.

**Read this table as "about 1 %", not as "+10 %".** A single unreplicated sample
on a shared host is not a measurement, and quoting one as a headline is the
2026-09-30 `NEVERAGAIN` entry about derived numbers arriving unverified.

## Task 12 — what it decided, and what it found

Decisions the task file left open, and where each one is recorded in the code.
The two that belong to the operator are flagged.

- **`on_click: Callback` is a new public newtype in `widgets::button`**, not
  `property.rs`'s `Callback<T>`, which is a private `Rc<dyn Fn(&T)>` and is the
  notification a property fires rather than an action a widget performs. The
  task file names a `Callback` that does not exist as a public type; this is the
  smallest thing that satisfies it. Its own doc says it moves somewhere shared
  when a second widget needs one — task 15's Toggle and task 19's TextInput both
  will.
- **The minimum touch target is a constant, not a theme token.** The theme has no
  token for it, and adding one would change `ThemeToken::all`, both theme
  tables, the token count, and the animation every token joins during a switch —
  for a value a switch does not change. `MIN_TOUCH_TARGET` documents the
  trade-off and says what would reverse it.
- **The states are four boolean properties, not one enum, plus a `ButtonState`
  enum that resolves them.** The states overlap: a button can be focused *and*
  hovered, and an enum holds one. `Button::style` is the pure resolution and is
  what is drawn; `Button::state` is the primary state for a caller that wants
  one name.
- **A button owns its transition clock.** `AnimationClock::clear` is whole-clock,
  so a shared clock would strand other widgets' transitions when one button is
  re-aimed. The button owning one is what makes a state change *replace* a
  transition rather than fight it. This is the same reasoning as `Theme`'s own
  clock, and it is the fix for the per-pad `clear` limitation the demo's
  `press_pad` documents at length.
- **Focus activation lives in the button, not in `input`.** `Focus` knows the
  focus *order*; `Button::on_event` knows what Enter means. `input.rs` gained
  only `Focus::focus` (focus a known node) and `route` (below). The alternative
  was teaching `Focus` the activation keys, which would put a widget's meaning
  in the input module.
- **`input::route` exists because `dispatch_event` cannot be used for this.**
  `dispatch_event` holds `&Arena` for its whole bubbling walk, and this
  repository reaches widgets through property callbacks — an `on_change` that
  marks a node dirty. A button's click writes a property, so the callback fires
  while the dispatch still holds a `Ref` on the arena, which is a `RefCell`
  double borrow and panics. Five demo tests fail on that revert. `dispatch_event`
  is unchanged and is still right for callers whose handlers cannot re-enter;
  `route` resolves the chain, the borrow drops, and the caller then handles.

**Defects found while implementing, and fixed:**

1. **A themed button started out grey.** `Button::new` seeds its colour
   properties from the *default* palette, and `set_palette` deliberately leaves
   the appearance alone so a theme switch can be animated — so a button that was
   given a palette and never aimed painted the neutral grey. Fixed by adding
   `Button::snap_to_state`, which applies the current state's appearance at once
   and clears any running transition first. Three tests, one of which mutates
   the `clear` away.
2. **The pressed overlay was opaque black, and the label vanished on it.** Found
   by screenshotting, not by any test — see the 2026-09-30 entry in
   `.ai/NEVERAGAIN.md`. The press amount was clamped to `0.0..=1.0` and used
   directly as the overlay's alpha, so a full press meant alpha 255. Now capped
   at `PRESS_SHADOW_ALPHA = 0.28`, which is what "a slight inner shadow" means.
   Measured on screen: the button's mean luminance goes 0.515 hovered → 0.316
   pressed → 0.515 released, where it was 0.094 pressed before the fix. The
   reviewer then found the *other* half: no test pinned the **order** the
   commands are recorded in, and moving the label above the overlay — the change
   that made the label invisible in the first place — left all 340 tests green,
   because every helper in the module filters and so cannot see order. `shapes()`
   now returns the recorded sequence and three tests assert on it.
3. **The button band landed on top of the pads.** `arrange_stack` places every
   child at the origin and ignores the position it declares, so an offset put on
   the band was ignored while the band still covered the centred pads. The offset
   belongs on the row inside it. **Four** tests catch the revert, not two.
4. **"right aligned" ran under the buttons.** Also only visible on screen. The
   text panel is 900 wide from an origin of 60, so a right-aligned label ended at
   960 while the band starts at 664. The column is now laid out at
   `TEXT_COLUMN_WIDTH = 594`, which puts its right edge at 654.

## Verifying a change that draws — the capture method

Task 11 recorded that the demo "was captured and inspected" without saying how,
and rediscovering it cost several steps. It is:

```sh
# The window id, not the root: ffmpeg's x11grab and ImageMagick's root capture
# both return black for a GL window, because the compositor does not put the
# window in the root pixmap. Capturing the window by id works.
DISPLAY=:0 xwininfo -root -tree | rg '"roados ui_demo"' | rg -o '0x[0-9a-f]+' | head -1
DISPLAY=:0 magick import -window <that id> /tmp/shot.png
```

`import` on this machine reports `missing an image filename` for a filename it
was given, and `ffmpeg -f x11grab -i :0+X,Y` returns black, so neither is the
tool. `magick import -window <id>` is. The window's position is also not the
origin: the root here is 1920×2280 and the window sits at +480+1468.

**Injecting input.** There is no `xdotool` or `xte` on this machine, so keys and
pointer events were injected through a throwaway C program linked against
`libXtst` (`XTestFakeKeyEvent` and friends; the headers *are* installed, under
`/usr/include/X11/extensions/XTest.h`). It lives in `/tmp`, not in the
repository.

**Injected input is unreliable on this host, and a failure to inject is not
evidence of a product defect.** What is actually known, measured 2026-09-30
across several attempts by two agents:

- On runs where injection **worked**, a synthetic **click** was required first.
  Before any click, injecting `T` changed **0** pixels and moving the pointer
  changed 0; `XSetInputFocus` returning `Success`, confirmed by
  `XGetInputFocus`, changed 0. After a click on a button, one `T` changed
  **614,400** pixels — the whole 1024×600 window — with the background going
  (18,18,18) → (255,255,255) and a themed card's fill (30,30,30) →
  (245,245,245); a second `T` returned to a pixel-identical image (`AE` = 0).
  Clicks on empty window space do not grant it.
- On other runs it delivers **nothing at all** — not clicks, not keys — including
  a fresh launch with a six-second settle. The counter stayed at `0 clicks` and
  `T` changed 0 pixels. That is not explained by focus, the extension or the key
  mapping, and the reviewer independently found `XTestFakeMotionEvent` having no
  effect in their session too.

So the click-first rule is a **necessary condition on the runs that worked, not
a sufficient one**, and it is recorded as such rather than as a recipe. When
checking whether a change works, **verify the injection reached the app before
concluding anything about the change**: compare captures with
`magick compare -metric AE a.png b.png null:` and read the on-screen counter, and
treat "0 pixels changed" as ambiguous between "the change is broken" and "the
input never arrived". The task 12 blocker was found by an agent that took the
second reading for granted in the *other* direction — a misread timestamp, a
waiver, and a defect that was real all the same.

**A pointer tap, and the waiver that was wrongly raised against it.** The first
attempt at this section recorded a waiver on task 12's "responds to tap/click",
on the reasoning that SDL3 on X11 does not take its event timestamp from the X
event, so an XTEST-injected press and release were stamped 14 hours apart and
every injected click read as a long press. **The reasoning was wrong in a way
that mattered.** The stamps were 4,552,618,936 and 4,604,053,201, and the
difference is 51,434,265 — read as milliseconds that is 14.3 hours, and read as
the **nanoseconds** they actually are it is 51.4 ms, an ordinary click. The
misreading turned a product defect into a tooling excuse: the recogniser's
thresholds were in the wrong unit, so a real click failed exactly as the injected
one did. The waiver is withdrawn and the defect is fixed; see *Current position*.

Keyboard injection through `libXtst` does work — once the window has been
clicked — and is how the focus ring and the activation key were seen on screen.
Pointer injection is unreliable on this host and is no longer relied on for
anything.

**2026-09-30, task 13: keyboard injection did not work this time, and the
statement above no longer holds for this X session.** The demo window was mapped
(`map_state=2`), it was already `_NET_ACTIVE_WINDOW`, `XSetInputFocus` returned
`Success` and `XGetInputFocus` confirmed it, `XTestQueryExtension` reported
XTEST 2.2, and the keyboard mapping does carry `T` on keycode 28. XTEST key
events were then injected at the window with `KeyPressMask` selected and **no
`KeyPress` was delivered at all** — a probe that selected the mask and waited
half a second saw zero events, and two captures taken after injecting `T` were
pixel-identical (`compare -metric AE` = 0) to the capture before it. Whatever
changed, the cause is not the injector, the focus, the extension or the mapping.
Task 13's theme-switch criterion is therefore covered by a unit test
(`the_card_follows_a_theme_switch` runs `toggle_theme` and four 100 ms frames and
asserts the card's recorded colour) and **not** by a capture. Re-probe before
trusting keyboard injection again; the pointer caveat above still holds.

**Superseded 2026-09-30 by the task 13 review — kept, not deleted, per the
sidecar rule in `AGENTS.md`.** The conclusion is wrong and the diagnosis in the
paragraph above is the opposite of the cause. Nothing was broken: the window
simply had not been **clicked**. Measured, after a synthetic click on the
"Press me" button, one `T` changes **614,400 pixels** — the whole 1024×600
window — with the background going (18,18,18) → (255,255,255) and the card's
fill (30,30,30) → (245,245,245); a second `T` returns to a pixel-identical image
(`AE` = 0). The clicks that fail to grant focus are the ones on empty window
space, which is what the original attempt was doing. The waiver this paragraph
justifies is withdrawn, and **the task 13 theme-switch criterion is verified on
screen on the reviewer's measurement** — the card's fill changing with the theme
— as well as by `the_card_follows_a_theme_switch`. Read the injection section
above before relying on it: click-first is necessary, not sufficient.

**2026-09-30, task 14: neither keyboard nor pointer injection reached the app in
this session, and the positive control says so.** The injector is the same shape
as before — `XTestFakeMotionEvent`, `XTestFakeButtonEvent`,
`XTestFakeKeyEvent` through `libXtst`, built in `/tmp` — and
`XTestQueryExtension` reports XTEST 2.2. What was measured:

- A drag along the slider's track from 50% to 90% of it: `compare -metric AE`
  = **0** against the capture before it.
- **Two presses on "Press me"**, the button task 12 verified on screen twice:
  `AE` = **0**, the counter still reads `0 clicks`, and the button still paints
  its resting fill rather than its hover tint. This is the control that makes the
  reading safe: the failure is not slider-specific, it is the whole input path.
- A `1` key press, which the demo answers by putting the slider at its maximum
  with the thumb travelling there: `AE` = **0**.
- `XQueryPointer` after a fake motion reports the pointer inside window
  **`0x0`** — not inside the demo window — while `xwininfo` reports the window
  `IsViewable` at `+480+1468`, 1024×600. So the pointer is not where the window
  is, whatever the injection does to the X server's idea of it.

So the "0 pixels changed" above is the ambiguous reading the earlier paragraphs
warn about, and it is resolved here by the *control*: task 12's button, known to
work, is equally dead, so nothing here is evidence about the slider. **Five of the
task file's eight acceptance criteria are therefore covered by tests and not by a
capture, and that is recorded as a waiver** rather than as a defect. They are
criteria **2 (dragging the thumb), 3 (tapping the track), 4 (keyboard and gamepad),
7 (the thumb animating) and 8 (the demo responding to a drag)** — every one of
them needs a pointer or a key to happen at all. Criteria **1** (track, fill and
thumb render) was capture-verified, and **5** (step snapping) and **6** (clamping)
are properties of the widget's arithmetic with no input and no GPU in the way, so
a test is the whole of their verification and no capture is owed them. Re-probe
with a button click as the control before trusting injection again, and treat a
button that does not count a click as the same failure rather than as the widget
under test.

**The capture method for task 14, since the paragraph above ends the input route
and something still has to say how the pixels were got.** Three captures were
taken and only the first is reachable from the demo as it stands:

1. **The resting slider, value 0, unfocused** — no input at all. `cargo build`,
   then `setsid ./target/debug/ui_demo > /tmp/demo.log 2>&1 &`, the window id
   from `xwininfo -root -tree | rg '"roados ui_demo"'`, and
   `magick import -window <id> shot.png`, cropped and scaled with
   `magick shot.png -crop … +repage -scale 400%`. This is the stock method above
   and needs nothing but a built binary.
2. **Values 25 and 70, and the focused state** — a **rebuilt binary with a
   temporary seed in `Demo::new`**, since no key and no click reached the app and
   the demo has no other route to either a value or a focus. The seed was six
   lines, read from the environment, and has been reverted:

   ```rust
   // SLIDER_PREVIEW: temporary capture aid, reverted immediately after.
   {
       let preview: f32 = std::env::var("SLIDER_PREVIEW")
           .ok()
           .and_then(|v| v.parse().ok())
           .unwrap_or(0.0);
       widget.value.set(preview);
       widget.focused
           .set(std::env::var("SLIDER_FOCUS").is_ok());
       widget.snap_to_state();
   }
   ```

   It ran as `SLIDER_PREVIEW=70 SLIDER_FOCUS=1 setsid ./target/debug/ui_demo` and
   `SLIDER_PREVIEW=25 setsid ./target/debug/ui_demo`, on builds that had it.
   `rg -c SLIDER_PREVIEW ui/src/ui_demo/src/main.rs` is **0** now, and the file's
   `md5sum` matches the snapshot taken before the seed was added.

   **What this means for the record, stated plainly:** the focus ring was seen in
   a capture produced this way — the widget's `focused` property was written at
   construction, not by `Demo::set_focus` — and the reader may re-apply the seed
   above to reproduce it. It is *not* evidence that `Tab` focuses a slider on
   screen, because no `Tab` ever arrived. The first version of this file said "seen
   on screen at 0, 25 and 70 with its fill, thumb, readout and focus ring" without
   saying that two of the three came from a seeded build, which read as though the
   demo's own keys had produced them; they cannot have, because `0` and `1` write
   0 and 100 and not 25 and 70.

**2026-09-30, tasks 15–18: the capture method is the one above, and two things
about it are new.** The window id is still read with `xwininfo` and captured with
`magick import -window <id>`, and the pointer is still at the window rather than
the origin — this round it was at root **+352+1408** and 1280×720, because the
demo's window grew. **Input injection did not work again**, in the developer's
session or the demo subagent's: `XTestFakeMotionEvent` returned 1, the pointer
did not move (`XQueryPointer` reported it at root (1464, 1468) before and after a
request for (1064, 1826)), and `magick compare -metric AE` between captures
before and after a synthetic click on "Press me" was **0** with the counter still
reading `0 clicks`. That is task 12's button failing to count a click, which is
the control that makes the reading safe: the failure is the whole input path, not
anything about the four new widgets.

**The asset path is resolved from the executable, not the working directory.**
`ui_demo/assets/demo.png` is looked for in `$ROADOS_ASSET_DIR` first and then by
walking up from `std::env::current_exe()`, because `cargo run` and `cargo test`
have different working directories. A missing asset prints one line to stderr and
stands in a transparent 320×192 image rather than taking the window down.

## Ratified by the operator (2026-09-28, 2026-09-29, 2026-09-30)

- **Task 09 lands before task 08**, decided 2026-09-29. Task 08's
  `Theme::switch_to` requires `Property::animate` per token and lists
  `EasingStandard`/`Decelerate`/`Accelerate` as theme token values — both are
  task 09's. The task files place 09's API inside 08, an inversion. 09 does not
  depend on 08, so 09 runs first. The numeric order is broken at this one point;
  every other cross-reference in the task files is honoured.
- The 24 task files, `01`–`24`, are the **confirmed spec**. The
  `.ai/workflows/idea-to-code.md` stage 1 gate is satisfied by the operator for
  the whole sequence — no `idea-evaluator` pass per task. Work enters at
  stage 3, and the reviewer checks the change against the task file, not
  against whether the task was the right idea. Tasks `25`–`29` and `30`–`32`
  were added afterwards, each in its own section below with its own status;
  they are **not** covered by that stage 1 waiver and each needs the operator's
  ratification before work starts on it.
- **An acceptance criterion that cannot be verified on this machine is waived
  with a recorded reason**, not silently dropped and not treated as a blocker.
  It goes in the `AC waived` column and its reason goes in *History*, so a
  waived criterion is never later mistaken for a verified one.
- The operator commits **per task**, and the sequence stops at every task for
  operator review and commit approval. No task starts before the previous one
  is committed.
- **`rustup update stable` executed by the operator, 2026-09-28.** Toolchain is
  now `cargo`/`rustc` 1.98.1. See *Resolved* below.
- **SDL3 builds with its default subsystems, for now.** The operator's decision,
  taken knowing the alternative is blocked. This *contradicts*
  `PRIMITIVES_ARCHITECTURE.md`, which specifies audio, render, camera and
  filesystem disabled at build time — see *Deviations*. Revisit when the
  head unit's audio and rendering needs are known.
- **Native keeps X11, the target build drops it entirely.** The operator's
  decision, 2026-09-28. The dev host runs the task 24 demo in a real window, so
  it needs a windowing driver; the head unit must carry no desktop stack. The
  split is enforced asymmetrically because no single mechanism can serve both —
  see *The native/target X11 split*.
- **Task 13's `Container` API takes `&mut Arena` on every method that needs it**,
  decided 2026-09-30. The task file's `Container::add_child(&self, child:
  Handle)` is not implementable: a node cannot reach the arena that holds it,
  because the arena owns the node, and `node::attach` already needs
  `&mut Arena<WidgetNode>`. This is the shape task 12's `Button` settled on and
  it is precedent, not a new decision — the task file's `-> Handle` is read as
  `Container::handle()`.
- **`ui_demo`'s private `container()` helper is replaced by the widget**, decided
  2026-09-30. The demo has had a helper doing this task's exact job since task
  06, in four call sites; building `ui_core::widgets::container` without retiring
  it would leave two implementations of a composition primitive in one
  repository, and the widget's acceptance criteria would be proven against a
  demo that does not use it.
- **`Padding` lives on `LayoutState` and the layout pass honours it**, decided
  2026-09-30. `layout.rs` had no padding at all, so this is a change to a module
  that task 07 shipped and that this sequence has since built four widgets on top
  of. It is chosen over keeping padding on the `Container` widget because a
  field only the container reads makes padding impossible on every other node,
  and a stacked child would still be placed by the pass at the unpadded origin.
  **Consequence to carry:** the layout suite and the committed
  `layout_walk_cost` benchmark both have to be re-run, because the pass itself
  changes and the benchmark is what the O(n·d) walk argument in *Deviations*
  rests on.
- **Tasks 15–18 are implemented as ONE changeset and reviewed afterwards**, the
  operator's decision of 2026-09-30, taken after being shown what it costs: the
  per-task review (step 2) does not happen, there is one commit instead of four,
  and **no per-task revert point** exists. The gates that goes around are named
  in *Gates skipped*. This supersedes the earlier ratified rule "the operator
  commits **per task** … no task starts before the previous one is committed" —
  not the rule's intent, which was a reviewable revert point per task, but its
  letter, which this round traded away knowingly.
- **`sdl3`'s `image` feature is approved** (2026-09-30), which pulls in
  `sdl3-image-sys` and builds SDL_image 3.4.6 from vendored source. Chosen over
  SDL3 core's own `load_bmp`/`load_png`, which need no new dependency and cannot
  decode JPEG, and over the pure-Rust `image` crate. Justification, alternatives
  and licence in `PRIMITIVES_ARCHITECTURE.md` § *Dependencies*.
- **Task 19's three gaps are the operator's decisions, 2026-10-01**, each taken
  after being shown the facts and before any code was written, because the task
  file requires something it does not describe. (i) **The on-screen keyboard is a
  `ui_core::widgets` module**, `keyboard.rs`, rather than demo-local code or a
  second type inside `text_input.rs`; `PRIMITIVES_ARCHITECTURE.md`'s
  § *Module Layout* is amended accordingly. (ii) **`InputEventKind::Text` is
  added to `input.rs`** with a `process` arm over `EVENT_TEXT_INPUT`, rather than
  deriving characters from `KeyDown` keycodes — which is the mechanism SDL's text
  event exists to replace. (iii) **The demo's window grows and the band is laid
  out side by side**, rather than the demo being re-laid-out or the band stacked.
  The reason for the third half of (iii) is measured, not chosen: this host's
  window manager caps the window at 1052 pixels, so a stacked band's keyboard
  would never have reached the screen. See *Task 19 — what it decided*.
- **Task 21's four gaps are the operator's decisions, 2026-10-02**, each taken
  after being shown the facts and before any code was written, because the task
  file requires something this pipeline could not do as written. (i)
  **Anti-aliasing is a multisampled default framebuffer** — `MULTISAMPLE_SAMPLES
  = 4`, `MULTISAMPLE_BUFFERS = 1` in `render/context.rs` — chosen over an FBO
  with a resolve pass after being told that it antialiases the whole app rather
  than the chart, that two quads sharing an edge may gain a hairline seam, and
  that it costs measurable frame time. (ii) **The area fill is per-segment
  convex quads**; ear-clipping triangulation in the renderer was declined.
  (iii) **The demo gives up the list and its readout** so the chart can have the
  column — the window cannot grow (a 1280×1320 request comes back 1280×1052,
  measured) and the operator's words were *"Remove some existing widgets like
  list or so. (Keep fps label.)"* (iv) **Line joins are mitred per-segment
  quads** rather than one `Path` per series. See *Task 21 — what it decided*.
- **aarch64 target libraries are deferred until the target image is decided.**
  The operator's decision, 2026-09-28. Native builds proceed and stay verified;
  aarch64 remains a documented waiver. No sysroot strategy is committed to yet.

## The native/target X11 split

Decided 2026-09-28. Both halves are forced by mechanism, not preference.

**Host, native build — install `libxcursor-dev libxrandr-dev libxss-dev`.**
X11 is the only windowing driver available on this host (the native configure
gave `Video drivers: dummy kmsdrm offscreen x11(dynamic)`, and Wayland's
development libraries are absent), so it is what makes the demo visible.

**Target, cross build — `SDL_X11=OFF SDL_WAYLAND=OFF`, and no X11 packages at
all.** Verified, not assumed: `CheckX11()` opens with `if(SDL_X11)` and
`find_package(X11)` and all nine extension probes are nested inside it, so with
the option off the X11 dev packages are never looked for.

**Installing them introduces no link-time X11 dependency.** `SDL_DEPS_SHARED`
defaults ON (`SDL/CMakeLists.txt:234`) and `SDL_X11_SHARED` defaults ON gated on
`SDL_X11;SDL_DEPS_SHARED` (`:341`), so `cmake/sdlchecks.cmake:398` takes the
dynamic branch and records the soname:

```cmake
if(HAVE_X11_SHARED)
  set(SDL_VIDEO_DRIVER_X11_DYNAMIC_XCURSOR "\"${XCURSOR_LIB_SONAME}\"")
else()
  sdl_link_dependency(xcursor LIBS X11::Xcursor ...)
endif()
```

SDL `dlopen`s it at runtime. The Rust binary never links X11, and `glow` resolves
GL through `SDL_GL_GetProcAddress`.

**Enforcement is asymmetric, necessarily.** `SDL_X11`/`SDL_WAYLAND` are not
forwarded by `sdl3-sys`, so they are unreachable from `Cargo.toml` and the
toolchain file is the only channel. `SDL_UNIX_CONSOLE_BUILD` *is* forwarded, and
is required: with both desktop drivers off, `cmake/macros.cmake:415` raises
`FATAL_ERROR` unless it is set, so it belongs in the manifest as
`build-from-source-unix-console` on `sdl3`.

**What the target needs instead of X11, and why it is deferred.**
`CheckKMSDRM` — the head unit's scanout path — requires
`pkg_check_modules(PC_LIBDRM libdrm)` *and* `pkg_check_modules(PC_GBM gbm)`
*and* `HAVE_OPENGL_EGL`. It goes through **pkg-config**, so aarch64 needs a
target sysroot with a working `aarch64-linux-gnu-pkg-config`; Debian's cross
toolchain alone does not provide one. `CheckEGL` is `check_c_source_compiles` —
compile only, against SDL's bundled khronos headers — so no EGL dev package is
needed. This is the point where `CROSSBUILD.md`'s "a sysroot stops being
optional" becomes real, and it is the decision being deferred.

**Runtime consequence to remember:** with `SDL_DEPS_SHARED=ON`, the target's SDL
`dlopen`s libdrm and libgbm, so the *target image* must ship them. Setting
`SDL_DEPS_SHARED=OFF` for static linking would change all of the above and make a
sysroot mandatory.

## Deviations from the spec, and why

- **`Chart::new` takes the arena and returns `Self`, not a `Handle`.** Task 21's
  requirement 1 writes `Chart::new(chart_type: ChartType) -> Handle`, which is
  stale about this repository in the same way task 13's requirement 2 was stale
  about `LayoutMode`: every widget here is constructed against
  `&mut Arena<WidgetNode>` and hands out its `Handle` separately, because the
  node has to exist before its properties do. So it is
  `Chart::new(nodes: &mut Arena<WidgetNode>, chart_type: ChartType) -> Self`
  plus `Chart::handle() -> Handle`, which is `Gauge`'s shape. `chart_type` is a
  plain field behind `set_chart_type` rather than a `Property`, following
  `Gauge`'s `gauge_type`: it is the *shape* rather than the appearance, and
  nothing animates it. The deviation and its reason are in the widget's own
  module doc.
- **Task 21's line and area rendering are per-segment quads, not a strip.**
  Requirement 6 asks for *"triangle strip or line strip"*; the pipeline is
  quad-only, and the index buffer's quad-only invariant (4 vertices, 6 indices)
  is pinned by a test. A line is therefore one convex `DrawCommand::Polygon` per
  segment with mitred corners, and an area is one convex quad per segment from
  the line down to the plot's bottom edge. **The fan is exact for a convex
  polygon and for nothing else**, so a single polygon for the whole series would
  be a wrong picture rather than a rough one — which is why the requirement's
  literal *"filled polygon below line"* is not what ships.
- **Task 21's requirement 6 asks for anti-aliased edges, and this task is what
  made that true** — 4x MSAA on the default framebuffer, the operator's decision.
  Before it, the pipeline had no antialiasing on a geometric edge at all: the
  solid shader's only branch is a hard `discard` on an axis-aligned rounded
  rectangle, and a polygon or a line takes the plain-colour path because
  `line_quad` and `polygon_quad` both hardcode `radius: 0.0`.
- **`input::dispatch_event` is unusable for any handler that reaches the arena,
  and `input::route` was added beside it.** `dispatch_event` holds `&Arena` for
  its whole bubbling walk. This repository reaches widgets through property
  callbacks — an `on_change` that marks a node dirty — so a handler that writes
  a property re-enters the arena while the dispatch still holds a `Ref` on it,
  which is a `RefCell` double borrow and panics rather than misbehaves. Task 10
  shipped `dispatch_event` without a caller that could hit this; the button is
  the first. `dispatch_event` itself is unchanged and remains correct for
  handlers that cannot re-enter.
- **Task 12's `on_click: Callback` is a type alias for `widgets::Callback<()>`, and
  the type itself moved to `widgets/mod.rs` in task 14.** The task file names a
  `Callback` that does not exist as a public type: `property.rs`'s is private and
  is `Fn(&T)`, the notification a property fires, not an action a widget performs.
  Task 14 needed the payload a `property.rs` callback already had — an `on_change`
  that reports the value it moved to — and the button's own doc said it would move
  when a second widget needed one. So it moved, and what moved is
  `widgets::Callback<T>`; tasks 15 and 19 now reach the same type. `button::Callback`
  is an alias, so no button's spelling changed.
- **SDL3 ships with all twelve subsystems enabled.** The architecture doc asks
  for audio, render, camera and filesystem off. Accepted temporarily because
  `sdl3` 0.20.0 re-exports no subsystem features, so honouring the doc needs
  either a second direct `sdl3-sys` dependency or target-asymmetric options
  forced from the toolchain file. Neither is worth the cost while the head
  unit's hardware requirements are still unknown. Consequences to remember: the
  binary carries audio and camera code it will not use, and no choice is being
  made about HID, haptics, or which video driver the target image wants.
  Revisit before `roados_ui`, not before task 02.
- **`ui_core`'s entry point is `src/lib.rs`, not `src/mod.rs`.** Task 02
  specifies `lib.rs`, which is also what Cargo expects, so
  `PRIMITIVES_ARCHITECTURE.md:332` is stale on this one point and was left
  alone. Nothing else in that document's *Module Layout* conflicts with the
  tree: the module list matches, and its `roados_ui/` line is a sibling
  directory rather than a module of `ui_core`, so it is not a workspace member
  at this stage.
- **`WidgetNode` has `children`, `parent`, `layout` and `paint`, not the
  `kind`/`properties`/`flags` of `PRIMITIVES_ARCHITECTURE.md:51-59`.** A node
  is a place in a tree with a layout cache and a paint cache; what it *is* and
  what properties it holds are not needed until a widget that has them is
  written, and an empty `kind` would be a lie until then. The reviewer's
  finding is the same one the note further down this file already makes about
  tasks 04 and 05 — that block is the one to settle, and this line only records
  what task 07 did in the meantime.
- **A node's clip rect is computed, not applied.** `LayoutState::clip()` holds
  the rect a renderer would scissor to, and the layout pass fills it in
  correctly, but nothing sets a scissor per node yet: `Renderer::set_scissor`
  applies to the whole frame, and a recorded draw command has no scissor state
  of its own to carry. Applying the rect is deferred to the task that draws
  within a node's own bounds, not to the layout pass, which is where the rect
  belongs either way.
- **Skipping a clean node costs a walk of its subtree, so a pass is
  O(n·d).** `visit` cannot skip a node that is clean, placed under the same box
  and clipped the same unless it also knows nothing below it is dirty, and the
  only way it can know is to look. It looks once per level on the path down to
  a change, so the worst case is the whole tree read once per level, not once.
  This is a **known and accepted cost, not an oversight**, and it was measured
  rather than guessed.

  **The shapes are the numbers.** A ratio here is a property of the tree, not
  of the code, so the fixture is part of the claim. Measured on `x86_64` dev
  host, 2026-09-29, release, best of 1000 (flat) or 20 (deep) passes, by the
  `#[ignore]`d harness `layout_walk_cost` in
  `ui/src/ui_core/src/layout.rs` — run it, do not trust this table:

  ```text
  cargo test -p ui_core --release --all-features --lib \
      layout_walk_cost -- --ignored --nocapture
  ```

  **Re-measured 2026-09-30, after task 13 added `Padding` to the pass:** the flat
  clean pass is unchanged (9 975 ns → 9 750 ns, medians of three) and the two
  1000-deep figures are 4–10 % higher, which is at the edge of this benchmark's
  own run-to-run spread. The ratios are the same — 4.5×, 27.0×, 1.0× — and the
  argument here is about ratios. Full numbers and the noise, in *Task 13 — what it
  decided, and what it found*.

  | fixture | one dirty leaf under a clean root | cold pass | ratio |
  | --- | --- | --- | --- |
  | 2041 nodes in one row, all clean | — | 27 ns without the walk, ~10 µs with it | ~370× |
  | 200-deep chain, links declare `tight(10,10)` | 172 µs | 39.6 µs | 4.3× |
  | 1000-deep chain, links declare `tight(10,10)` | 4.43 ms | 198 µs | 22× |
  | 200-deep chain, links declare nothing | 5.02 ms | 4.77 ms | 1.1× |
  | 1000-deep chain, links declare nothing | 146 ms | 142 ms | 1.0× |

  Two things follow, and both were got wrong in earlier drafts of this entry.
  The **clean pass is the real cost**: a frame where nothing changed — the
  common case, and the one the 27 ns was bought for — now reads the tree, and
  no other row here matters as much. The **honest ratio range is 1.0× to
  22×**, not one number: a chain whose links declare constraints is linear
  apart from the walk, so the walk is nearly the whole cost; a chain whose links
  declare none is *already* O(n·d) through `resolve_box` → `content_size`, and
  the walk adds about 1% on top of it. Quoting the second shape as "the" cost
  overstates the walk by 20×; quoting the first as the only shape understates
  the clean-pass regression. Quote the range.

  There is no "before" number for the dirty-leaf rows to be compared against,
  and that is the point: without the walk the pass skipped the change entirely
  and returned the wrong answer, so it was fast by being broken.

  **The alternative, and why it is not taken.** A per-subtree "has a dirty
  descendant" bit maintained in `place` is refused for a reason that is a fact
  rather than a judgement: `LayoutState::set_mode`, `set_flex_config`,
  `set_constraints`, `set_flex` and `set_position` are `pub` and reachable
  through the `pub layout_mut()`, and they mark only `self.dirty` — never the
  parent links. A cached bit therefore says "clean" when a descendant is
  dirty, which is the exact bug the walk exists to prevent. Making the bit
  correct means giving those setters a way to reach the parent links, i.e.
  arena-taking setters, which is a public API change declined in this task.

  **A global mutation epoch is a different matter, and this entry should not be
  read as refusing it.** An epoch closes that gap without touching the setters,
  makes the guard O(1), and costs exactly one recomputation after an edit —
  after which the tree is fast again. It is better than the walk on most
  workloads, and on any workload that does not edit on every frame it is
  strictly better. It is declined *only* for the per-frame case: an edit on
  every frame invalidates every bit on every frame, so there the epoch buys
  nothing over the walk while adding a field and a global, and task 09's
  animation clock is exactly that workload. **That per-frame claim is argued,
  not measured** — no epoch prototype exists, and building one to measure it is
  the work it would save. What would settle it is a prototype run over the same
  three shapes as the harness above, once an animation actually exists to
  measure against. **Revisit in task 09**: re-evaluate the epoch on the
  non-per-frame case, which is most of a UI, rather than on the reasoning in
  earlier drafts of this entry, which was wrong about what an epoch does.

## Protocol in force

**Canonical owner: `.ai/workflows/task-sequence.md`.** That file states the
per-task loop and the gates, and is routed to by `AGENTS.md`. It is not restated
here; this section is a pointer so a reader who lands in this file does not
re-derive it.

Short form, for orientation: one task at a time in numeric order — developer
dispatched as `general`, then a reviewer in a **different** session, then fixes
back to the developer, then **stop for the operator**, who commits and reports
the SHA. `developer` and `reviewer` are instruction files, not subagent types;
`.ai/protocols/subagents.md` allows only `explore` and `general`, and their
contents are inlined into each dispatch brief.

## Environment as found (2026-09-28)

Recorded because several acceptance criteria depend on it.

- `cargo`/`rustc` 1.80.1 via snap; `rustup` present with `stable-x86_64-unknown-linux-gnu` active.
- **aarch64 cross toolchain installed 2026-09-28** (operator):
  `aarch64-linux-gnu-gcc` 15.2.0, `aarch64-linux-gnu-g++` 15.2.0, GNU
  binutils 2.46, `-dumpmachine` → `aarch64-linux-gnu`. Cross build verified —
  see `CROSSBUILD.md` §6.4.2.
- `rustup` targets installed: `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`,
  `wasm32-unknown-unknown`.
- `libEGL` and `libGLESv2` present (Mesa); `DISPLAY=:0`; `/dev/dri/card{1,2}`.
  GLES 3.1 support itself is unconfirmed.
- No code in the repository yet, so there are no existing conventions to match
  until task 02 establishes them.

## Task table

`AC waived` records acceptance criteria that were met with a reason instead of
verified. A blank cell is unknown, not "none".

| # | Task | Status | Commit | Review | AC waived |
|---|---|---|---|---|---|
| 01 | Crossbuild Environment Setup | done | `2f27127` | 3 review passes, 4 fix rounds | 9 open, 2 closed |
| 02 | Project Scaffolding | done | `89b67b7` | 4 review passes, 3 fix rounds | — |
| 03 | SDL3 + OpenGL ES 3.1 Context | done | `5e564c7` | 1 review pass, 1 fix round | — |
| 04 | Arena Allocator | done | `a8f3147` | 1 review pass, 0 fix rounds | — |
| 05 | Property System | done | `8c3657b` | 0 review passes, 1 fix round | — |
| 06 | Rendering Pipeline | done | `0b1c3e7` | 1 review pass, 0 fix rounds | — |
| 07 | Layout System | done | `58957d8` | 4 review passes, 3 fix rounds | 1 (`cargo audit` not installed) |
| 08 | Theme System | done | `d9041f9` | 0 review passes, 0 fix rounds | — |
| 09 | Animation System | done | `6726e21` | 4 review passes, 3 fix rounds | 1 (`cargo audit` not installed) |
| 10 | Input Handling | done | `4e51b09` | 2 review passes, 1 fix round | 1 (`cargo audit` not installed) |
| 11 | Widget — Label | done | `ffbb4d6` | **none — committed without review** | 0 |
| 12 | Widget — Button | done | `9973185` | 1 pass, *fix first* — findings fixed, **not re-reviewed** | 0 |
| 13 | Widget — Container | done | `2dc9193` | 1 pass, *approve with minor findings* — findings fixed | 1 (`cargo audit` not installed) |
| 14 | Widget — Slider | done | `11f4134` | 1 pass, *approve with required changes* — 2 majors, 4 minors, all fixed before the commit | **ACs 2, 3, 4, 7, 8** — each needs a pointer or a key, and XTEST injection delivered no event to the app in either session (see *Verifying a change that draws*). AC 1 was capture-verified; ACs 5 and 6 are the widget's own arithmetic and a test is their whole verification. *Tool gate, not an AC:* `cargo audit` is not installed |
| 15 | Widget — Toggle | done, **batched** | `d7240c8` | **none** — see *Gates skipped* | **ACs 2, 3, 4, 5, 6** — every one needs a pointer or a key to happen at all, and no injected event reached the app. AC 1 (track and thumb render) was **capture-verified** |
| 16 | Widget — Image | done, **batched** | `d7240c8` | **none** — see *Gates skipped* | **ACs 2, 3, 4, 5, for the picture only.** The *geometry* of all four fits and the opacity are asserted as recorded draw commands, and AC 6's corner clip and AC 7's image on screen were **capture-verified**; only `Contain` has been *seen*, because reaching the other three needs a key |
| 17 | Widget — Progress | done, **batched** | `d7240c8` | **none** — see *Gates skipped* | **ACs 3, 4** — a value animating and the indeterminate slide both need frames with something moving, and moving them needs a key. ACs 1, 2 and 5 were **capture-verified**, the bar on screen at 50% |
| 18 | Widget — List/Scroll | done, **batched**, then **fixed on screen** | `d7240c8` | **none** — see *Gates skipped* | **ACs 2, 3**, and the "scrollable" half of **AC 7** — all three need a pointer or a wheel on this host. **AC 2 is not a defect**: dragging *up* scrolls, through the demo's real event path, and dragging *down* at offset 0 cannot move a list past its own start. ACs 1, 4, 5, 6 and AC 7's *100 rows on screen* are **capture-verified and unit-tested**, and the clipping defect the operator reported is fixed — see *A defect the operator found* |
| 19 | Widget — TextInput + On-screen Keyboard | done | `b4a2db8` | **none** | **none waived** — every criterion is covered by the demo's own event path or by a capture. What is *not* claimed is anything about XTEST injection, which was not used; see *Task 19 — what it decided* |
| — | Frame-rate readout, stdout report, `fps-check.sh` | done | `3ddf5fa` | none yet | n/a — an operator request, not a task with criteria. Verified: the suite is green, six mutations killed, the readout seen on screen, and both run-end paths measured — see *The frame rate, measured* |
| 20 | Widget — Gauge | done | `79941cd` | **none — committed without review** | **Requirement 5's anti-aliasing half was NOT met at the time and was not waived** — the renderer had no SDF for curves and no MSAA; the widget's module doc said so and the hard edges were seen in a capture. **That is no longer true**: 4x MSAA landed with task 21 and the gauge's doc has been superseded in place. **AC 3's "needle as a triangle"** required a new filled `Polygon` draw command, which the operator approved. The needle's spring is asserted by tests, not seen mid-flight. ACs 1, 2, 4 and 5 are capture-verified and unit-tested — see *Task 20 — what it decided* |
| 21 | Widget — Chart | **reviewed, uncommitted — awaiting the operator's commit** | — | **2 passes**, both in a session separate from the author's. Round 1: *approve with required changes*, 1 blocker + 6 minors, all 7 fixed. Round 2: *approve with required changes*, blocker **closed and verified by mutation**, **5 minors waived 2026-10-02 with recorded reasons** — not "fixed"; see *The two review rounds* | **AC 5 is covered by tests through the demo's real event path, not by a capture** — keyboard injection does not reach the window on this host (the positive control `T` moved 212 px) and pointer injection never did. ACs 1, 2, 3, 4 and 6 are capture-verified **and measured**, the bar and area ones through two reverted temporary releases. `y_labels` are empty by design, so AC 4's labels are proved by the x labels and the two axes. **No acceptance criterion is waived**; the 5 waived findings are review findings, not criteria — two stale citations, one coverage claim, one omission and one run count, none of which can change a pixel. See *Task 21 — what it decided* |
| 22 | Widget — Dialog | pending | | | |
| 23 | Widget — Toast | pending | | | |
| 24 | Demo Application | **superseded** | | | |
| — | Tesla-like demo application | pending | | | see `doc/ui/DEMO_APPLICATION.md` |

**Task 24 is superseded and will not be started in its current form.** The
operator's decision of 2026-09-30 replaces the widget-gallery demo with a
Tesla-like infotainment application, in a new `TASK_UI_DEMO_n` task category
begun after task 23. That document owns the decision, the scope and the open
questions; nothing about it is restated here. The line is in this table so a
fresh session resuming from this file does not start task 24, and so the last
task of the `PRIM` sequence is 23 rather than 24.

Status values: `pending` · `in progress` · `implemented` (developer done,
awaiting review) · `in review` (reviewer running) · `changes requested` ·
`approved` (operator approved, awaiting commit) · `done` (committed) ·
`blocked` (see History for the reason).

Numeric order is a valid dependency order: the cross-references in the task
files place 03 before 06 before 07, 05 before 08 and 09, 10 before the input
widgets, and 24 last.

## Tasks 25–29, added after the demo

Created 2026-09-28, **pending the operator's ratification**. They exist so the
deferred work is not lost, not because anything in 02–24 needs them.

The trigger was a review question worth answering explicitly: *does the missing
KMSDRM driver block tasks 02–24, given a successful cross build is required?*
**No.** `CROSSBUILD.md` §6.4 configures and builds SDL for the target with
`SDL_X11=OFF`, `SDL_WAYLAND=OFF`, `SDL_UNIX_CONSOLE_BUILD=ON` and **no sysroot
at all**, exit 0. Every platform dependency degrades to `OFF` rather than
`FATAL_ERROR`; the only fatal gate is no-X11-no-Wayland, which the toolchain file
already handles. What the missing driver breaks is *runtime video on the device*,
not the build.

| # | Task | Needs |
|---|---|---|
| 25 | Target Image and Sysroot | operator strategy decision; unblocks 26 |
| 26 | Head-Unit Video Driver | 25 |
| 27 | Target Runtime Library Audit | 25, 26 |
| 28 | Reconcile the SDL Configuration | none — but item 5 must land **before task 04** |
| 29 | Head-Unit Smoke Test | 25, 26, 27 |

**Three things were recorded as gating the cross-build requirement**, and they
were not all in these tasks. All three are closed as of task 02, 2026-09-28.
What remains open is the sysroot — `CROSSBUILD.md` §8 item 1 — which is a runtime
question, not a build one.

1. **`build-from-source-unix-console` must be in the manifest.** Without it the
   cross configure dies at `cmake/macros.cmake:415`. It is the one option that
   makes the target build possible, it lives in the file task 02 creates, and
   task 01 only documents it. **Task 02's brief must carry this.** — carried, in
   both manifests, and no longer removable; see `AGENTS.md` § Rust.
2. **The aarch64 cross toolchain is now installed** (2026-09-28), and the real
   cross build passes — `CROSSBUILD.md` §6.4.2. A
   `cargo build --target aarch64-unknown-linux-gnu` **now passes as well**, with
   no sysroot: the manifest task 02 created exists and carries
   `build-from-source-unix-console`, and the artifact is
   `ELF 64-bit LSB pie executable, ARM aarch64` — `CROSSBUILD.md` §4.2 records
   the command and the `readelf -h` output. Nothing mechanical is left between
   this tree and a target build.
3. The earlier caveat that every cross run used an x86_64 `gcc` symlink is
   superseded by item 2 and by §6.4.2: a real aarch64 compile happened, twice —
   once as a direct CMake configure and once through Cargo. What a sysroot would
   still add is the *runtime* side, which is item 1.

## Tasks 30–32, the text gaps task 11 left

Created 2026-09-30, **pending the operator's ratification**. Task 11 is
implemented and renders, but three of its own requirements are not met. They are
named here so that "implemented" is not read as "the whole spec landed", and
each has a task file so the work is not carried in prose.

| # | Task | Unmet requirement in task 11 | Symptom today |
|---|---|---|---|
| 30 | Font fallback chain | §2 *Font fallback chain* | `Label::font_family` is a property nothing reads; the demo hardcodes one `FONT_PATH`. A character the font lacks is **silently dropped** — `get_or_insert` returns `None`, `draw_text_batch` `continue`s, and the word has a hole in it. `GlyphKey` is `{ ch, size }`, so a second font would collide with the first. |
| 31 | Dynamic atlas growth | §3 *Dynamic atlas growth* | The atlas is a fixed `ATLAS_SIZE = 2048`. `allocate` evicts LRU rows and returns `None` when it cannot, and `None` is a **silent** dropped glyph. Live glyphs' UVs and row bookkeeping must survive a re-pack. |
| 32 | Fade and clip truncation, drawn | §4 *Text truncation: ellipsis, clip, fade* | `truncate_line` treats `Clip` and `Fade` identically and `Label::paint` never reads `truncation`, so there is **no fade ramp at all** and `Clip` is a layout cut, not a visual clip. Only the ellipsis third works. |

Task 11's §2 also lists HarfBuzz shaping and bidi. Those are **waived, not
deferred**: the operator dropped HarfBuzz on 2026-09-30 because its safe binding
exposes no shaping API and `unsafe` was declined. They have no task file, and
that is deliberate — the trigger for revisiting is a complex-script or bidi
requirement landing, recorded in `AGENTS.md` § Rust and
`PRIMITIVES_ARCHITECTURE.md` § *Dependencies*. A task file would imply work
nobody has asked for.

## Resolved since task 01 was reviewed

- **The Rust toolchain blocker is gone.** `rustup update stable` was executed by
  the operator on 2026-09-28; the toolchain is now `cargo`/`rustc` 1.98.1, above
  the 1.85 floor `sdl3-sys` 0.7.1 needs. Closes waivers 2 and 3.
- **The pinned dependency set now builds natively.** With the operator's
  `libxcursor-dev libxrandr-dev libxss-dev` install, `cargo build` of
  `sdl3 = { version = "0.20", features = ["build-from-source"] }` plus `glow 0.18`
  completes, exit 0. Closes waiver 3 for the native path.
- **The X11 link-time dependency does not exist — measured, not argued.** `ldd`
  on the built binary reports four dynamic dependencies and nothing else:
  `linux-vdso.so.1`, `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`. No
  `libX11`, no `libGL`, no `libEGL`. X11 support is entirely `dlopen`, recorded
  as sonames in the generated `SDL_build_config.h`:
  `SDL_VIDEO_DRIVER_X11_DYNAMIC "libX11.so.6"`,
  `..._DYNAMIC_XCURSOR "libXcursor.so.1"`, and six more.
  `SDL_VIDEO_OPENGL_EGL 1` and `SDL_VIDEO_OPENGL_ES2 1` are both set, with
  `SDL_VIDEO_OPENGL` and `SDL_VIDEO_OPENGL_GLX` undefined — EGL and GLES only,
  which is the shape the head unit wants.

## New finding — KMSDRM is silently disabled without pkg-config

Found 2026-09-28 while verifying the native build. **Not a build failure: the
build succeeds and produces an SDL with no scanout driver.** On a head unit that
means `SDL_Init` finds no video driver at all, with no error and no warning.

`CheckKMSDRM` in `SDL/cmake/sdlchecks.cmake` requires three things, and the first
two are reached only through pkg-config:

```cmake
if(PKG_CONFIG_FOUND)
  pkg_check_modules(PC_LIBDRM IMPORTED_TARGET ${PKG_CONFIG_LIBDRM_SPEC})
  pkg_check_modules(PC_GBM   IMPORTED_TARGET ${PKG_CONFIG_GBM_SPEC})
endif()
if(PC_LIBDRM_FOUND AND PC_GBM_FOUND AND HAVE_OPENGL_EGL)
  set(HAVE_KMSDRM TRUE)
  set(SDL_VIDEO_DRIVER_KMSDRM 1)
```

On this host `pkg-config` is **absent**, so `PKG_CONFIG_FOUND` is false, so
`PC_LIBDRM_FOUND` and `PC_GBM_FOUND` are false, so `HAVE_KMSDRM` is never true.
The generated header confirms it:

```
/* #undef SDL_VIDEO_DRIVER_KMSDRM */
/* #undef SDL_VIDEO_DRIVER_KMSDRM_DYNAMIC */
/* #undef SDL_VIDEO_DRIVER_KMSDRM_DYNAMIC_GBM */
```

This does not affect the native build, where X11 supplies the window and the
demo is visible. It affects the **target**, whose scanout path *is* KMSDRM, and
it does so silently. `pkg-config` and `libdrm-dev`/`libgbm-dev` — or their
target-sysroot equivalents — are therefore hard requirements for any aarch64
build, not optional extras. Carried into the aarch64 decision below, and into
the task 01 amendment.

## Blocking task 02 — the host is missing the X11 development headers

**Status: resolved 2026-09-28.** The operator ran

```sh
sudo apt-get install -y libxcursor-dev libxrandr-dev libxss-dev
```

and the pinned dependency set then built natively, exit 0 — `CROSSBUILD.md`
§6.6. No longer a blocker. The finding is kept below because it is what the
package list came from, not because it is open.

Found by running the build, not by reading. The vendored SDL3 configure fails on
this host:

```
CMake Error at cmake/sdlchecks.cmake:405 (SDL_missing_dependency)
  Couldn't find dependency package for XCURSOR.  Please install the needed
  packages or configure with -DSDL_X11_XCURSOR=OFF
```

Measured: `/usr/include/X11/Xlib.h` and `X11/XKBlib.h` are present;
`X11/extensions/Xcursor/Xcursor.h`, `Xrandr.h` and `scrnsaver.h` are **absent**,
and there are no `libX*.so` development symlinks. ALSA headers are absent too,
which is a warning rather than an error.

**The awkward part, and the reason this needs a decision rather than a fix.**
`CROSSBUILD.md` §7.6 establishes that with `build-from-source` there is no `-D`
escape hatch: `cmake` 0.1.58 has no `CMAKE_ARGS`, CMake does not read
`CMAKE_PROJECT_INCLUDE` from the environment, and `sdl3-sys`'s `build.rs` exposes
no X11 options at all — verified, there is no `X11`/`XCURSOR` string in it. So
`-DSDL_X11_XCURSOR=OFF` is unreachable from `Cargo.toml`.

That leaves exactly two routes on a **native** build, and a native build passes no
toolchain file:

1. Install `libxcursor-dev`, `libxrandr-dev`, `libxss-dev` on the host.
2. Route the native build through a toolchain file that forces the options —
   which then makes the native and aarch64 builds asymmetric, and puts the
   configuration somewhere `Cargo.toml` does not describe.

Note this is **independent of the subsystem decision** above: the X11
sub-options are not subsystems, so accepting default subsystems does not bring
this closer to fixed, and disabling the documented four would not have fixed it
either. It was always going to be needed.

- Task 04 places "widget node structure" out of scope, deferring it to task 05;
  task 05 is titled *Property System* and does not list a widget node type
  among its requirements. Whoever reaches 04/05 should resolve this rather than
  both agents guessing differently. Not yet settled by the operator.
- **`PRIMITIVES_ARCHITECTURE.md` § Dependencies is wrong in two places**, found
  while doing task 01. Both need an operator decision; neither is mine to amend.
  1. It says SDL subsystems are disabled by build configuration, but `sdl3`
     0.20.0 **re-exports no subsystem features at all** — its 20 features
     include none, and none forwards one. The switches live on `sdl3-sys` 0.7.1
     (`sdl-<name>` / `no-sdl-<name>`). So the pinned dependency line cannot
     implement the documented configuration. Task 02 is blocked on the choice
     between a second direct `sdl3-sys` dependency, or options forced from the
     toolchain file — which is target-asymmetric and invisible in `Cargo.toml`.
  2. It lists **filesystem** among the subsystems to disable. There is no
     `SDL_FILESYSTEM` option in SDL 3.4.16 and no `sdl-filesystem` feature in
     `sdl3-sys`; SDL always compiles its Unix filesystem implementation.
     The doc and reality disagree.

## Waivers, task 01

Transcribed from the developer's handoff and confirmed by the reviewer. Per the
operator's rule, none of these is treated as satisfied.

1. ~~**No real aarch64 build or artifact**~~ — **resolved 2026-09-28.** The
   operator installed `gcc-aarch64-linux-gnu`, `g++-aarch64-linux-gnu` and
   `binutils-aarch64-linux-gnu`; the Rust target was already present. A **real**
   cross configure and build now succeed — no fake `PATH`, no stub `pkg-config`,
   no arch-flag override, the file's own `-march=armv8-a` — and
   `readelf -h libSDL3.so.0` reports `Machine: AArch64` across all 251 objects.
   Full transcript in `CROSSBUILD.md` §6.4.2. Closed.
2. ~~**`rustup update stable` not executed**~~ — **resolved 2026-09-28.** The
   operator ran it; the toolchain is 1.98.1. Closed.
3. ~~**`cargo build` of the pinned dependency set fails**~~ — **resolved
   2026-09-28** for the native path, after the X11 dev packages were installed.
   `cargo build` of the pinned set now completes, exit 0. The *aarch64* build
   remains unbuilt and is covered by waiver 1.
4. **The §6.4 cross configure is not an aarch64 build** — the compiler was a
   symlink to host x86_64 `gcc`. The document states this rather than claiming
   otherwise.
5. **The archive-step failure is not reproduced** — the cross build dies earlier,
   at dbus. The mechanism is cited from CMake's sources, not observed.
6. **The dbus and libdrm results depend on a stub `pkg-config`.** A 17-byte
   `exit 0` stub made `*_FOUND` come back true. Re-run with a real
   `aarch64-linux-gnu-pkg-config`.
7. **Host OpenGL ES 3.2 is not the target's GLES 3.1.** `glxinfo -B` is a GLX host
   result; no EGL 3.1 context was created. Verifying it needs the head unit.
8. **The subsystem configuration in §5.1 is untested in practice.** The
   feature-rejection error is cited from `sdl3`'s feature table, not reproduced,
   because cargo 1.80.1 cannot parse the manifest. Needs (2) first.
9. **`ROADOS_SYSROOT` never exercised against a real target.** Ran in script mode
   and in real `project()` configures, but only with a hand-made fake sysroot and
   a stand-in compiler. `CMAKE_SYSROOT`, `CMAKE_FIND_ROOT_PATH`,
   `PKG_CONFIG_SYSROOT_DIR` and `PKG_CONFIG_LIBDIR` are verified; whether
   *library* search paths resolve is not.

## History

- 2026-10-02 — **task 21 reviewed twice, the first reviewed task in this
  sequence.** Round 1: *approve with required changes* — **1 blocker** (a
  non-finite sample erased the two real series segments either side of it,
  against four doc sites that said it cost only the segment it spanned) and
  **6 minors**. All seven fixed. Round 2: blocker **closed, and proved closed by
  mutation** rather than by reading the diff — removing the two
  `.filter(is_finite)` calls reproduces round 1's four measurements exactly.
  Round 2 left **5 minors, all waived by the operator on 2026-10-02** with
  recorded reasons instead of a third round. **Also recorded here:** two
  self-labelled `THROWAWAY` harnesses (`zz_dot.rs`, `zz_verify.rs`) were found
  in `ui/src/ui_demo/examples/` from an interrupted investigation into exactly
  the reversal branch; they were run once to capture their answer, then deleted,
  because a scratch `examples/` file fails `cargo clippy --all-targets` and they
  were the only clippy failures in the tree. **The open defect round 2 declined
  to fix — `Join::Corner((0,0))` at a small positive `1 + p·q`, called "a defect,
  not a degenerate case" by the widget's own doc — is still open**, with
  `a_full_reversal_depends_on_which_way_f32_rounds` as its tripwire.
- 2026-09-28 — file created before the first dispatch, so an interrupted task is
  recoverable. No task started.
- 2026-09-28 — **task 01 implemented.** Review round 1: `fix first`, 1 blocking
  (a stray code fence that rendered §6.4 through §8 — 198 lines, including the
  unblocking instructions — as literal code), 4 should-fix, 5 nits. All fixed.
- 2026-09-28 — review round 2: `fix first` again, delta small. All 11 original
  findings confirmed fixed; both disputes the developer raised were **conceded in
  the developer's favour** — the reviewer had invented a `gcc -print-prog-name`
  mechanism, then partly overcorrected into treating a missing
  `CMAKE_<LANG>_COMPILER_AR` as a general archiver problem when it only feeds the
  IPO archive rules, and its dbus "host contamination" finding turned out to be
  the reviewer's own 17-byte `pkg-config` stub. Round 2 raised 5 new findings, all
  introduced by the fix round; all fixed in round 3.
- 2026-09-28 — fix round 3 also produced a self-correction worth recording: the
  developer had earlier reported `CMAKE_LIBRARY_ARCHITECTURE` coming out wrong and
  proposed a guard. The guard was inert; `project()` derives the value from the
  compiler it probed and overwrites whatever a toolchain file set. No defect
  existed — the wrong value only ever appeared because a host compiler stood in
  for aarch64. The comment now documents that the line is not load-bearing.
- 2026-09-28 — two operator decisions now block task 02: the `rustup update
  stable` machine change, and how SDL subsystems get configured.
- 2026-09-28 — **both settled by the operator.** `rustup update stable` executed
  (toolchain now 1.98.1); SDL3 to build with default subsystems for now, the
  deviation from `PRIMITIVES_ARCHITECTURE.md` recorded above rather than
  silently adopted.
- 2026-09-28 — running the build after the toolchain update exposed a **new**
  blocker for task 02: the host lacks the X11 extension development headers, and
  SDL's X11 sub-options are unreachable from `Cargo.toml` for the reason
  `CROSSBUILD.md` §7.6 documents. Open for the operator.
- 2026-09-28 — **the operator installed the X11 dev packages.** The pinned
  dependency set then built natively, exit 0. The X11 link-time dependency was
  measured rather than argued: `ldd` reports four dependencies and no X11.
- 2026-09-28 — **the operator's three decisions** (native keeps X11, target
  drops it, aarch64 libraries deferred) were implemented as a task 01 amendment.
  Review round 3: `fix first`, but the substance confirmed and **all six of the
  developer's self-declared flags adjudicated in the developer's favour** —
  including the reviewer reversing its own earlier position on
  `SDL_UNIX_CONSOLE_BUILD`. Six findings remained, all mechanical: stale
  evidence, two wrong counts, two stale statements, two nits.
- 2026-09-28 — fix round 4 cleared them, and produced a correction the reviewer
  had missed: `message_tested_option` (`cmake/macros.cmake:55`) prints
  `(Wanted: ${_REQVALUE}): ${HAVE_<name>}` — **two different variables**. So
  `(Wanted: ON): OFF` means the option was on and the *backend test* failed, not
  that the option was off. Verified: of 31 such lines, 29 are `BOOL=ON`, two
  have no entry, and **none** is `BOOL=OFF`. This reframes the whole
  missing-`pkg-config` finding and is now documented as such.
- 2026-09-28 — **task 01 ready for operator commit.** Three review passes, four
  fix rounds. The toolchain file and the document are untracked; nothing is
  staged or committed. Task 01 still awaits the operator's approval.
- 2026-09-28 — **the operator installed the aarch64 cross toolchain**, and it
  changes task 01's standing. The last open waiver — "no real aarch64 build or
  artifact" — is now **closed with evidence** rather than carried: a real cross
  configure and build, real `aarch64-linux-gnu-gcc`, the file's own
  `-march=armv8-a`, `Machine: AArch64` across all 251 objects. Full transcript
  in `CROSSBUILD.md` §6.4.2, and §6.4 is retitled so its stand-in runs no longer
  read as the verdict.
- 2026-09-28 — **the real build also corrected two standing claims.** (i)
  `SDL_KMSDRM` is `BOOL=ON` in the cache without any forcing, because
  `dep_option` defaults it ON for Unix; what fails is the *backend test*, so task
  26's fix is the sysroot and its requirement 2 is now answered. (ii) The target
  records **zero** `dlopen` sonames — `#define …DYNAMIC` count is 0 and none of
  `libX11.so.6`, `libwayland-client.so.0`, `libdrm.so.2`, `libgbm.so.1` appears in
  the binary. A naive `grep -oE '…DYNAMIC…'` reports 26 and is wrong, because it
  matches the macro names in `#undef` lines. Tasks 26 and 27 were updated.
- 2026-09-28 — **the operator asked for the workflow to be written down**, and it
  is now `.ai/workflows/task-sequence.md`, canonical, with pointers from
  `AGENTS.md` and this file. It does not reopen the 2026-09-27 decision to keep
  `reviewer.md` the only review document; it owns the sequence and the gates
  between steps, which no agent file owned.
- 2026-09-28 — **tasks 25–29 drafted** to carry the deferred work, pending
  ratification. Triggered by the reviewer's question about whether the missing
  KMSDRM driver blocks the sequence: it does not, because a cross build needs
  neither the driver nor a sysroot — `CROSSBUILD.md` §6.4.2 now proves the
  cross build works with no sysroot at all.
- 2026-09-28 — **task 01 committed by the operator**, `2f27127`, and task 02
  started. The `Pending` statuses of 25–29 are unchanged: they are still outside
  the confirmed 24-task spec, and 02–24 do not depend on them — except that item
  5 of task 28 must land before task 04, which is the next place that bites.
- 2026-09-28 — **task 02, review round 1: six findings, five fixed, one
  escalated to the operator.** Fixed: `AGENTS.md` duplicated rules
  `developer.md` owns; `CROSSBUILD.md` §4.2 documented a command with no
  evidence and never said where `.cargo/config.toml` comes from;
  `IMPLEMENTATION_STATE.md` never recorded that `PRIMITIVES_ARCHITECTURE.md:332`
  is stale on the `lib.rs` entry point; `ui_core/src/lib.rs` claimed the module
  tree follows that document when the task file lists the modules; and the
  `ui_demo` manifest comment did not say the feature list is a mirror. All five
  held on re-review.
- 2026-09-28 — **task 02, review round 1, escalated blocker: SDL is linked
  dynamically and nothing says where the target's copy comes from.** Measured,
  not inferred: `readelf -d` on both artifacts gives `NEEDED libSDL3.so.0` and
  no `RPATH`/`RUNPATH`; run directly, the host's `/usr/local/lib` SDL 3.5.0
  wins over the vendored 3.4.16, and `cargo run` only gets the vendored one
  because Cargo puts the crate's link-search directory on `LD_LIBRARY_PATH`. The
  static route works but is not free: `SDL_DEPS_SHARED` stays ON under
  `SDL_STATIC`, so the statically linked binary still carries the X11 chain's
  `dlopen` sonames — measured on this host, `libX11.so.6`, `libXcursor.so.1`,
  `libXrandr.so.2`, `libX11-xcb.so.1` — and what the target `dlopen`s is whatever
  its `pkg-config` supplies, which today is nothing (§6.4.2 records zero
  sonames). And rpath or install-prefix packaging is unreachable while SDL is a
  cross build, because `sdl3-sys` is written in Rust and SDL's CMake never sees
  the manifest.
  **Not the developer's to decide and not fixed:** the operator owns the
  linkage strategy. Unchanged by either review round, and no acceptance
  criterion is waived over it — all four were verified.
- 2026-09-28 — **task 02, review round 2: six findings, all documentation or
  one-sentence corrections.** The escalation above was accepted as correctly
  handled, and all five round-1 fixes were confirmed to hold. Fixed here: the
  three lossy restatements of `developer.md` rules that round 1 had left
  standing; two documents that still claimed no aarch64 cargo build had ever
  been run, when the artifact and a from-scratch build prove otherwise; the
  `CROSSBUILD.md` preamble that still said the project did not exist, and §3's
  missing working directory; two manifest comments that stated the feature
  mechanics wrongly; one stale `cmake/macros.cmake` line number; and the
  review-rounds bookkeeping — two History lines and the `Review` cell — that
  this round's own finding 4 asked for. The
  reviewer's own three settled facts — the `sdl3` licence, `SDL_RPATH`
  reachability, and `wait_event_timeout`'s `None` on error — were offered for
  elsewhere and deliberately not taken here.
- 2026-09-28 — **task 02, review round 3: four findings, all fixed.** All four
  were documentation corrections, and the reviewer confirmed the change is
  otherwise ready for commit. Fixed here: §8 item 3's stale reason — "no aarch64
  build exists yet" replaced with the measured one, that the aarch64 builds so
  far run with no sysroot and no target `pkg-config`; the round-2 History
  undercount — "five findings" corrected to six and the review-rounds
  bookkeeping added to the list; §4.2's pronoun — the binary's DWARF references
  one tree source, the other twelve are in `libui_core.rlib`, so the sentence now
  says the build carries thirteen with the split named — and its timing figure,
  46.8 s adjusted to the reproducible 46.7 s; and one off-by-one citation,
  `cmake/macros.cmake:56` → `:55`, the `message(STATUS …)` line rather than the
  `endmacro()` below it.
- 2026-09-28 — **task 02 committed by the operator**, `89b67b7`, and task 03
  started. Four review passes, three fix rounds; every defect was in
  documentation, and three of the four across the last two rounds were
  self-invalidating line-number citations. The Rust and the manifests were
  clean from round 1 onward. The `libSDL3.so.0` linkage question is carried
  open into task 03 — see *Current position*.
- 2026-09-28 — **task 03 committed by the operator**, `5e564c7`, and task 04
  started. One review pass, *Approve* with two minor findings (a suffix-less
  version-string edge case and a discarded `sdl3::Error` type), both fixed.
  Static linking applied and verified: the binary has no `libSDL3.so.0`
  dependency, only `libm`, `libgcc_s`, `libc`, `ld-linux`.
- 2026-09-28 — **task 04 committed by the operator**, `a8f3147`, and task 05
  started. One review pass, *Approve*, no findings. The reviewer independently
  mutation-tested the code (9 mutations, 6 caught, 3 correctly uncaught as
  implementation details).
- 2026-09-28 — **task 05 committed by the operator**, `8c3657b`, and task 06
  started. Implemented directly after the developer agent returned empty twice.
  One fix round: clippy `type_complexity` (type alias for callbacks), cascade
  propagation in `recompute`, and a doc test that moved a non-`Copy` property.
- 2026-09-29 — **task 06 committed by the operator**, `0b1c3e7`, and task 07
  started. Implemented by a developer subagent (first attempt returned empty;
  second attempt with a focused prompt succeeded). One review pass, *Approve
  with minor findings* — all non-blocking: stale state file, gradient shader
  deferral, layer boundary deferral, no automated GL test, O(n) batching.
  Two bugs found and fixed during verification: VAO not re-bound in
  `draw_solid_batch`, and demo loop's `continue` on event timeout skipping all
  drawing. Demo verified live with GPU capture showing correct premultiplied
  alpha blending.
- 2026-09-29 — **task 07 implemented and reviewed; awaiting the operator's
  commit.** Four review rounds, three fix rounds, no finding outstanding and
  none waived except `cargo audit`, which is not installed here. The reviewer
  mutation-tested every fix rather than trusting the reported counts, and
  three times found a test that did not discriminate the change it was written
  for — the attach/detach cache-invalidation fix took four attempts before a
  test actually killed the mutant. Four deviations recorded above: the node
  struct, the clip rect, the O(n·d) walk cost, and the walk's revisit point.
  Two things a reader should know before starting task 08:
  1. **The dirty-flag contract is the tightest constraint in this change.** A
     `LayoutState` setter marks only its own node; the pass reaches a dirty
     descendant through `subtree_is_dirty` because the five setters are
     reachable through `pub layout_mut()` without the arena, so a cached
     dirty-descendant bit cannot be kept honest. Any new way to dirty a node
     must keep that walk's cost in view.
   2. **`layout_walk_cost` is a committed `#[ignore]`d benchmark**, not a test.
      Run it before and after changing the pass, not during `cargo test`.
- 2026-09-29 — **the operator decided task 09 runs before task 08**, because 08's
  spec requires `Property::animate` and `Easing`, which 09 owns. Recorded under
  *Ratified by the operator*.
- 2026-09-29 — **task 09 implemented and approved; awaiting the operator's
  commit.** Four review rounds, three fix rounds, no finding outstanding and
  none waived except `cargo audit`, which is not installed here. The reviewer
  mutation-tested the fixes and found the suite genuinely discriminating. Three
  pre-existing `property.rs` defects were found and fixed: `set` dropped the
  callback list (`mem::take`), a bound property recomputed without notifying,
  and `bind`'s recompute closure captured `Rc<PropertyInner>` and leaked every
  bound property. The animation module has no dependency on the node arena; the
  demo wires `on_change` to `layout::mark_dirty`.
- 2026-09-29 — **a bad revert during fix round 1 lost the uncommitted work in
  `property.rs` and `main.rs`.** `git checkout --` restored both from HEAD. A
  fresh developer rebuilt both files on top of the intact `animation.rs`, and
  the reviewer confirmed the rebuild was correct. The lesson — never revert
  uncommitted work with `git checkout --` when a targeted edit will do — is in
  `.ai/NEVERAGAIN.md`.
- 2026-09-29 — **task 08 committed by the operator**, `d9041f9`, and task 10
  started. The implementation was already in the working tree from the previous
  session; two clippy warnings were fixed before commit. Doctests require
  `TMPDIR` on the main filesystem — `/tmp` is a tmpfs with a user quota that
  causes `Disk quota exceeded` during linking.
- 2026-09-30 — **task 10 implemented and approved; awaiting the operator's
  commit.** Two review rounds, one fix round, no finding outstanding and none
  waived except `cargo audit`, which is not installed here. Three gesture
  defects found and fixed: two-finger hold firing long presses, canceled touch
  leaving a stuck pointer, pinch with coincident start never arming. The
  reviewer mutation-tested each fix. `layout.rs` gained a `visible` flag on
  `LayoutState` for hit testing.
- 2026-09-30 — **task 10 committed by the operator**, `4e51b09`, and task 11
  started.
- 2026-09-30 — **task 11 implemented; awaiting review.** The developer
  subagent returned empty three times, so the task was implemented directly,
  as task 05 was. `widgets/label.rs` is new (+683): the `Label` widget node
  with its four properties, and a pure text-layout engine (word/character
  wrap, left/center/right/justify alignment, line height, letter spacing,
  ellipsis/clip/fade truncation, vertical truncation) measured through an
  advance-width callback. 228 unit + 13 integration + 32 doctests pass; fmt,
  clippy and doc clean. Two mutations (wrap off-by-one, ellipsis budget) were
  each caught by the test named for them and restored clean.
- 2026-09-30 — **the operator approved the font dependency, then narrowed it
  to FreeType alone.** `freetype-rs 0.38` (`bundled`) is added to `ui_core`;
  it compiles its vendored C from source via `cc` and statically links — the
  same from-source, no-system-library model SDL3 uses, so the aarch64
  cross-build needs no sysroot for it. Recorded in
  `PRIMITIVES_ARCHITECTURE.md` § *Dependencies* and `AGENTS.md`.
  **HarfBuzz was approved then dropped the same day.** Its safe Rust binding
  (`harfbuzz` 0.8) exposes no shaping API — only `unsafe` C calls — and the
  operator declined `unsafe`. FreeType alone renders Latin text; ligatures,
  complex scripts and bidirectional text wait for a future `unsafe` decision.
  **Build verified, both targets, 2026-09-30.** Native: `libfreetype2.a` and
  `libpng.a` are produced from the vendored source. aarch64 cross-build: exit 0
  in 57 s with **no sysroot**, the artifact is `ELF 64-bit … ARM aarch64`, and
  FreeType and zlib are statically linked.
  **Superseded 2026-09-30 by the tasks 15–18 round, which measured this rather
  than repeating it: the "same four dynamic dependencies" claim is true of the
  aarch64 build and FALSE of the native one.** `readelf -d` on the native
  `ui_demo` reports **five** `NEEDED` entries — `libz.so.1`, `libm`, `libgcc_s`,
  `libc` and the loader — and `libz.so.1` is there because
  `freetype-sys`'s vendored `libpng.a` leaves `inflate`, `deflate` and `crc32`
  undefined and resolves them against the **system** zlib. Verified rather than
  inferred: it is not SDL_image's doing (`nm -u` on the native
  `libSDL3_image.a` finds no undefined zlib symbol), and it is **pre-existing** —
  a `git worktree` build of `11f4134` reports the same five. The aarch64 build
  has no system zlib to find, builds the vendored one, and links it: four
  dependencies, as claimed. So the vendored-zlib claim was right for the target
  and wrong for the host, and the native image path is one shared library wider
  than this file said for two tasks.
- 2026-09-30 — **the text pipeline renders; the demo shows it.** The text
  renderer is no longer the remaining work: the glyph atlas (shelf packing,
  LRU eviction with span reuse, atlas-owned dirty tracking), the SDF
  generator, the text shader and the `u_text_resolution` uniform are all in,
  and `Label::paint` feeds the laid-out lines to the renderer as one command
  per line, justified lines word by word. `ui_demo` paints a seven-label panel
  — greeting, a wrapping paragraph, the three alignments, letter spacing and a
  line cut with an ellipsis — with `+`/`-` for size, `C` for colour and `T` for
  the theme.
  **A real bug the unit tests could not see:** both vertex buffers were sized
  for one vertex per quad, so `glBufferSubData` failed with `GL_INVALID_VALUE`
  past 64 quads and the text was silently dropped. It is fixed in
  `vertex_buffer_size`, which is now unit-tested, and the demo was captured and
  inspected to confirm it.
  **Known gaps, deliberately left:** no font fallback chain (`font_family` does
  not resolve fonts) and no dynamic atlas growth; both wait on a decision. Fade
  truncation lays out but does not draw a fade. Shaping, ligatures, complex
  scripts and bidi remain waived with HarfBuzz. Each of the first three is now a
  task file — **30**, **31**, **32** — see § *Tasks 30–32*; the HarfBuzz items
  stay waived with no task.
  **Verified 2026-09-30:** fmt, clippy, 246 unit + 22 demo + 33 doctests, doc,
  aarch64 cross-build, and FreeType still statically linked with no dynamic
  freetype dependency on either target.
  **One verification limit, stated rather than glossed:** the `+`/`-` size keys
  are covered by a unit test, not by a captured screenshot — this machine has no
  `xdotool` or `xte` to inject a key event. Size-dependent *layout* was
  confirmed visually by capturing the demo built at a 40px default instead, which
  re-wraps the panel. Injecting synthetic input into a running demo is a tooling
  gap, not a task; SDL's own event path is already covered by task 10.
- 2026-09-30 — **task 11 committed by the operator, `ffbb4d6`, with the review
  step skipped.** The commit carries the label widget, the text pipeline, the
  `ui_demo` panel, the three gap tasks (30–32) and the docs. The
  `task-sequence.md` loop puts the review **before** the operator's commit —
  step 2 dispatches `reviewer.md` in a different session from the developer, and
  the *No self-review* gate says the developer does not clear its own work. That
  did not happen here, so the task table records the review column as **none**
  rather than as a pass. The tree is clean, the suite is green at the commit
  (246 unit + 22 demo + 33 doctests, fmt, clippy, doc, aarch64), and the commit
  is a sound review target: a reviewer can be dispatched against `ffbb4d6`
  without a revert point being at risk. What is lost until that happens is the
  second pair of eyes on ~3200 lines, and the recorded fact that nobody has
  looked for the findings a reviewer would look for.
- 2026-09-30 — **task 12 implemented; awaiting review.** The developer subagent
  wrote `widgets/button.rs` and half of `ui_demo`'s wiring, then died on a
  provider rate limit with no handoff, so the demo half was finished directly and
  the result verified from scratch rather than reported. `button.rs` is +1988
  lines: the four state properties, the four animated ones, a `Palette` and a
  `Motion` resolved from the theme, a `Callback` newtype, a per-button
  `AnimationClock`, and a pure `style()` that both `animate_to_state` and
  `paint` read. `input.rs` gained `Focus::focus` and `route`. `ui_demo` routes
  the band through `GestureRecognizer` and `input::route` rather than raw events,
  so a button consumes what is aimed at it.
  **Four defects, two of them only findable by looking at pixels:** the press
  overlay was opaque black and swallowed the label (recorded in
  `.ai/NEVERAGAIN.md`); the band's offset was on the node a `Stack` ignores, so
  it landed on the pads; "right aligned" ran under the buttons; and a themed
  button started grey, which needed `Button::snap_to_state` to exist at all.
  **Five mutation checks, each seen to fail for the right reason** before being
  restored: reverting `route` to `dispatch_event` (5 tests, the `RefCell`
  double borrow), moving the band offset back onto the band (3), dropping the
  `clear` from `snap_to_state` (1), removing the 44 px floor (6), and letting a
  disabled button into the focus order (1). Verified: fmt, clippy `-D warnings`,
  296 `ui_core` + 44 demo + 44 doctests, `cargo doc` clean, and the aarch64
  cross-build (AArch64, no sysroot, the same four dynamic dependencies, so
  FreeType and SDL are still statically linked).
  **`cargo audit` is not installed here and was not run** — the standing waiver,
  as on tasks 07, 09 and 10. **One acceptance criterion is waived:** the pointer
  tap, for the timestamp reason recorded under *Verifying a change that draws*.
  The other six were seen on screen: all five visual states, the press animation
  measured at 0.515 → 0.316 → 0.515, the focus ring on `Tab`, and the click
  counter incrementing on `Enter`.
- 2026-09-30 — **task 12 reviewed; one blocker and three majors found, all
  fixed.** The verdict was *fix first*. The blocker is the important one and it
  is not a button defect: the reviewer's finding was that
  `GestureRecognizer` compared SDL's **nanosecond** event stamps against
  **millisecond** thresholds, so every press fired a long press, `release_pointer`
  returned before it could emit a `Tap`, and **no widget acting on a tap could
  ever fire**. Task 12's acceptance criteria 2 and 7 were unmet because of task
  10's code. The unit tests missed it because every one of them used small
  timestamps — `200` for "quick" — which is under a 300-nanosecond window.
  **The developer had already written a waiver for the criterion, on a
  misreading**: the two stamps were 4,552,618,936 and 4,604,053,201, which read
  as milliseconds is 14.3 hours and read as the nanoseconds they are is 51.4 ms.
  The misreading turned a product defect into a tooling excuse, and it was only
  the review that separated them. The waiver is withdrawn. Fixed by making both
  thresholds `Duration`, so `held <= TAP_MAX_DURATION` cannot compile across
  units; two tests fail on the old behaviour, one a realistic 100 ms click.
  **Three majors, all "the test cannot fail" shape.** (i) The new alpha
  assertion was `shadow.a <= round(PRESS_SHADOW_ALPHA * 255)` — computed *from*
  the constant it was checking, so it survived setting that constant to 1.0. The
  number is now written out. (ii) The reviewer moved the label above the press
  overlay — the exact change that had made the label invisible — and all 340
  tests stayed green, because every helper in the module filters and cannot see
  order; `shapes()` and three tests now pin the recorded sequence. (iii) A demo
  test named `a_tap_on_a_button_does_not_reach_the_node_behind_it` claimed the
  counter would go up by two if the tap fell through, and nothing behind the
  band handles a `Tap`, so it could not have; retitled to what it establishes,
  with a pointer to where the consumption contract really is tested. Also fixed:
  two stale counts in this file, the padding and radius constants that asserted
  themselves rather than reading the theme tokens, and a doc comment that
  overstated which scrolls reach focus navigation.
  **All seven acceptance criteria are now met and all seven were seen on
  screen**, the last of them after the fix: three real pointer clicks on
  "Press me" took the counter to "3 clicks" with the hover tint and the focus
  ring both visible. Re-verified: fmt, clippy `-D warnings`, 301 `ui_core` + 44
  demo + 44 doctests, `cargo doc` clean, and the aarch64 cross-build.
- 2026-09-30 — **task 12 committed by the operator, `9973185`, with the review
  step completed but not repeated.** The verdict was *fix first*; the blocker and
  the three majors were fixed and each fix mutation-verified, but **no reviewer
  has looked at the fixes**, so the *No unreviewed advance* gate was skipped. The
  task table records that rather than a pass. The commit is a sound target: the
  tree was clean at `9973185` except for `doc/ui/DEMO_APPLICATION.md`, which is
  untracked, so a reviewer can work against the commit with no revert point at
  risk. What is unverified is the fixes themselves — chiefly the `Duration`
  conversion in `input.rs`, the `shapes()` ordering tests, and the retitled demo
  test.
- 2026-09-30 — **`doc/ui/DEMO_APPLICATION.md` arrived untracked during task 12
  and is not committed.** It is the operator's: a Tesla-like infotainment demo
  in a new `TASK_UI_DEMO_n` category, superseding task 24. It is referenced from
  the task table so a session resuming here does not start task 24, and is
  otherwise untouched. Whether it belongs in the repository is the operator's
  call, not this file's.
- 2026-09-30 — **task 13 implemented: the Container widget, `Padding` in the
  layout pass, and `ui_demo`'s `container()` helper retired.** Five files, two
  independent components by the scope check's count (the widget, the layout
  change it consumes) plus the demo migration, which touches none of the
  library's behaviour. `widgets/container.rs` is new; `layout.rs` gained the
  `Padding` type, the `LayoutState` field and the three places the pass applies
  it; the demo's tree is built out of `Container`s and the pads sit on a card.
  325 `ui_core` + 49 demo + 49 doctests, fmt clean, clippy `-D warnings` clean,
  `cargo doc` clean, aarch64 cross-build clean (`AArch64`, the same four
  dynamic dependencies), and the card seen on screen at 788×164 with the pads
  inset by 12. Six deliberate breaks were run and each failed for the right
  reason, and two of them found a gap first: removing the `inset` from `arrange`
  was caught by nothing until `a_padded_container_measures_its_children_in_the_padded_box`
  was written, which is now the test that says a padded container gives its
  children a smaller box and not only a smaller offset. **Waived, with the
  reason recorded in *Verifying a change that draws*: the theme switch of the
  card could not be seen on screen, because XTEST key injection delivered no
  event to the window in this X session.** `cargo audit` was not run; it is not
  installed on this host and no dependency changed.
- 2026-09-30 — **the operator decided to proceed to task 13 without the task 12
  re-review**, having been told what was unverified and that the
  *No unreviewed advance* gate was being skipped. The gate is skipped, not
  satisfied: nothing about the fixes has had a second pair of eyes. Task 13
  therefore builds on a task whose review findings are fixed but unchecked, and
  if task 13's review turns up something in `input.rs` or `button.rs` that
  belongs to task 12, it belongs to task 12's history and not to task 13's.
- 2026-09-30 — **task 13 implemented, reviewed *approve with minor findings*, all
  findings fixed; awaiting the operator's commit.** `widgets/container.rs` is
  new (18 tests) and the demo's private `container()` helper is gone, with all
  six of the demo's parents being `Container` widgets and a test that fails if a
  bare parent reappears. `layout.rs` gained `Padding`, applied to the box
  (`layout.rs:1189`), the placements (`:1200`) and the measurement (`:1527`) —
  applied, not merely stored, which is the failure this repository has already
  recorded once for the clip rect.
  **The review's five findings, all fixed.** (i) A comment credited
  `Padding::inset`'s four `.max(0.0)` floors with behaviour `clamp_axis` actually
  provides; the reviewer removed the floors and got byte-identical output on
  eleven arrangements. The floors are gone and both comments now name the clamp.
  (ii) The demo's `on_change → mark_dirty` link for the card did nothing and its
  comment said otherwise — the demo rebuilds every paint state each frame, and
  `mark_dirty` dirties *layout* for a paint-only change. Deleted, with a comment
  saying why the card is the one node that needs no link. (iii) A test was added
  for `set_padding` *after* a settled pass, which every other padding test
  missed; it is caught by removing `set_padding`'s `mark_dirty`. (iv) The
  benchmark table overstated the regression by 5–10× on one unreplicated
  baseline sample; replaced with the reviewer's interleaved figures, which put
  the clean pass 11 % *faster* and the two deep shapes up about 1 %. (v) A
  `"1 clicks"` string, from task 12, that the reviewer's own capture showed on
  screen; now pluralised.
  **A fifth finding was the reviewer's, and it corrected a claim of mine**: the
  developer had waived *the card's theme switch, not seen on screen*, on the
  evidence that XTEST delivered no `KeyPress`. That was wrong, and I had already
  written the opposite claim into this file. On the runs where injection works, a
  synthetic click is required first; after one, `T` changes 614,400 pixels and the
  card's fill goes (30,30,30) → (245,245,245). The waiver is withdrawn, the
  developer's note is marked superseded rather than deleted, and *Verifying a
  change that draws* now records the whole of it — including that injection
  delivers **nothing at all** on other runs, which is why "0 pixels changed" is
  ambiguous and has to be checked against the on-screen counter before it is read
  as a defect. **I over-claimed in the other direction while fixing it** and the
  file says so.
  **Requirement 2 is partially unmet and recorded as such:** `wrap: bool` is
  accepted by `LayoutMode` and never read by the pass, which is task 07's
  pre-existing state, documented at `layout.rs:495` as arriving with the list
  widget. Container supplies the mode; it does not supply wrapping.
  Re-verified after the fixes: fmt, clippy `-D warnings`, 326 `ui_core` + 49
  demo + 49 doctests, `cargo doc` clean, aarch64 cross-build. `cargo audit` is
  still not installed and remains the only waiver.
- 2026-09-30 — **task 13 committed by the operator, `2dc9193`**, reviewed
  *approve with minor findings* with all five fixed before the commit. The
  operator's `5722fb1` separately committed `doc/ui/DEMO_APPLICATION.md`, so the
  demo-application direction is now tracked rather than untracked.
- 2026-09-30 — **task 14 implemented: the Slider widget, the action `Callback`
  moved somewhere two widgets can reach it, and a slider in the demo.** Five
  files: `widgets/slider.rs` is new (54 tests), `widgets/mod.rs` gains the
  parameterised `Callback<T>` that task 12's own doc promised would move,
  `widgets/button.rs` keeps its `Callback` name as a type alias for it, and
  `ui_demo` gains a slider and a value readout under the button band. The widget
  holds the value, the drawn thumb, the track, the fill, the thumb and its border
  as properties, `min`/`max`/`step`/`orientation`/the three sizes as setters, and
  a `Palette` read from the theme the way `Button`'s is.
  **Two defects, and only one of them is the kind the suite can catch.** The
  focus ring was drawn as a filled rounded rectangle around the whole slider,
  which on screen is a white card with a track on it — a `RoundedRect` fills its
  rect, and a slider has no background to draw over the ring's middle the way a
  button does. Every draw-command assertion in the module called it correct, and
  the capture is what found it; both are recorded in `.ai/NEVERAGAIN.md` with the
  rules they replace. Separately, `Slider::travel` subtracted a rect's *origin*
  from its *extent*, which every unit test missed because every one of them lays
  its slider out at `(0, 0)`; the demo found it within the hour because the
  demo's slider is at `(664, 496)`. Both are fixed, and a fixture away from the
  origin now exists.
  **Eleven deliberate breaks, each seen to fail for the right reason and each
  restored from a snapshot taken immediately before it**: dropping the step snap
  (3 tests), dropping the interaction's write of the drawn thumb (2), firing
  `on_change` on an interaction that moved nothing (3), letting the keyboard act
  on an unfocused slider (1), putting the focus ring back around the whole node
  (1), removing the too-small-for-its-thumb guard (1), drawing the thumb's border
  over the thumb (2), and four in the demo — not offering a drag to the slider
  being dragged (1), and reordering the positionless-event precedence (3).
  A twelfth mutation, dropping the clamp inside `fraction`, was **not** caught by
  `cargo test --lib` at all: it was caught by the doctest on `fraction`, and the
  clamp is now asserted in the unit suite as well, because every path the widget
  takes clamps first and so nothing else in the module could see it missing.
  Verified: fmt, clippy `-D warnings`, 380 `ui_core` + 65 demo + 57 doctests,
  `cargo doc` clean, aarch64 cross-build (`AArch64`, statically linked, the same
  shape as every previous task), and the slider **seen on screen three times**: at
  its resting 0 with nothing but a built binary, and at 25 unfocused and 70 focused
  on builds carrying a temporary seed in `Demo::new` that has since been reverted —
  the whole method, and why two of the three came from a seeded build rather than
  from the demo's own keys, is in *Verifying a change that draws*.
  **Five acceptance criteria are waived: 2, 3, 4, 7 and 8** — the drag, the tap, the
  keyboard and gamepad, the visible animation and the demo's response to a drag,
  every one of which needs a pointer or a key. XTEST injection delivered nothing to
  the app in both the developer's session and the reviewer's, and task 12's button
  is the control that says so. `cargo audit` is a **tool gate, not a criterion**:
  it is not installed and remains the standing waiver from tasks 07, 09, 10 and 13.
- 2026-09-30 — **task 14 reviewed: *approve with required changes* — the widget
  itself stands, and every finding is about the record or a doc comment.** The
  reviewer re-ran the whole suite and reproduced all four of the developer's input
  measurements exactly, tried three mutations against the widget and could not
  break it, and upheld both `NEVERAGAIN` entries, the control comparison behind
  the XTEST waiver, all six flagged scope risks, the left-stick reading,
  `Orientation`'s being load-bearing, the `Button` alias removing no public path,
  the no-collision claim (reproduced on the reviewer's own capture) and
  `DEMO_APPLICATION.md` not being the developer's. Two majors and four minors, all
  fixed here and none of them touching the widget's behaviour.
  **Major 1 — a capture claim with no route in the code.** The file said the
  slider was seen on screen "at 0, 25 and 70 … and focus ring", while the same
  file recorded that no key and no click reached the app, and the demo's only
  route to a focus is `set_focus`, which only a `Tab` or a positionless `Scroll`
  reaches. The reviewer did the arithmetic the developer had not: **the demo's
  `0` and `1` keys write 0 and 100, not 25 and 70**, so those two captures cannot
  have come from them. They did not. They came from **a rebuilt binary with a
  temporary six-line seed in `Demo::new` reading `SLIDER_PREVIEW` and
  `SLIDER_FOCUS` from the environment**, taken because nothing else could put a
  value or a focus on the screen, and reverted afterwards
  (`rg -c SLIDER_PREVIEW ui/src/ui_demo/src/main.rs` is 0, and the file's md5
  matches the pre-seed snapshot). The seed is now quoted verbatim in *Verifying a
  change that draws*, with an explicit statement of what the focused capture is
  and is not evidence for, and the `NEVERAGAIN` entry that rests on it carries the
  same note. The claim was true; it was undocumented, which is the finding.
  **Major 2 — three different counts for one waiver.** The table cell said `2`, the
  History said "one acceptance criterion", and the *Verifying* section said "the
  drag and key criteria", all against a ratified operator rule that requires a
  criterion which cannot be verified here to be *named* so it is never later
  mistaken for a verified one. All three now read the same: **ACs 2, 3, 4, 7 and
  8**, with AC 1 recorded as capture-verified and ACs 5 and 6 explained as the
  widget's own arithmetic, where a test *is* the whole of the verification.
  `cargo audit` is now labelled a **tool gate, not a criterion**, in the cell and
  in History, because the column header says criteria.
  **Minor 3** — two doc comments (`FOCUS_RING` and `Palette::ring`) still described
  the ring as drawn around the slider's whole rect, which is the pre-fix
  description of the defect this change fixed. **Minor 4** —
  `a_slider_paints_its_thumb_over_its_fill_and_its_border_under_itself` asserted
  only the shape sequence, and `shapes` maps two `Circle`s to one word, so
  swapping the thumb's circle and its border left it green; it now asserts the two
  radii, `shapes`'s doc says what it cannot see, and the reviewer's swap was
  re-run and took that test red with "12 against 14" before the restore. **Minor
  5** — `.ai/NEVERAGAIN.md.context.md` still said five entries and "Last touched:
  2026-09-27" against a file with 17; the sidecar is corrected and its own history
  extended. **Minor 6** — the task 13 record said `container.rs` was "new (17
  tests)" and it has 18, a count that went stale inside task 13's own fix round;
  corrected here because this file was already being edited, and
  `container.rs` itself is unchanged.
  Re-verified after the fixes: fmt, clippy `-D warnings`, 380 `ui_core` + 65 demo
  + 57 doctests, `cargo doc` clean. `cargo audit` is still not installed and
  remains a tool gate.
- 2026-09-30 — **the "task 12 is unverified" claim narrowed, because the operator
  asked what it rested on and the answer was partly weaker than stated.** A
  dedicated re-review of task 12's fixes never happened, so the gate was skipped
  — that part stands. But the note said "nobody has checked the fixes", and that
  was too strong: the task 13 review independently confirmed the **blocker**
  twice, once by reading the unit fix in the source and once by driving a real
  click that took the live counter from 0 to 1. What remains unexamined is the
  three majors' fixes and the minors, which are test and comment changes with no
  behavioural surface. The note above now says that instead.

- 2026-09-30 — **task 14 committed by the operator, `11f4134`,** reviewed *approve
  with required changes* with all six findings fixed before the commit. Task 14's
  fixes — the `NEVERAGAIN` entries on the filled-rounded-rect focus ring and the
  origin-read-as-extent travel, the capture-method paragraph, and the three
  counts that had to agree — are therefore no longer one agent's word.
- 2026-09-30 — **tasks 15, 16, 17 and 18 implemented as one changeset, awaiting
  the operator's review.** Four task files, one changeset, on the operator's
  decision recorded under *Ratified*. Implementation was fanned out per
  `.ai/protocols/subagents.md` § *Implementation fan-out* in two waves of
  file-isolated sub-tasks, integrated and verified here:
  - **Wave 1** (4 parallel, disjoint files): `paint.rs` + `batch.rs`
    (`DrawCommand::Image` extended with `UvRect`, `opacity`, `radius`);
    `widgets/toggle.rs` (task 15); `widgets/progress.rs` (task 17);
    `widgets/scroll.rs` (task 18's `Scroll`).
  - **Wave 2** (3 parallel, disjoint files): `render.rs` (the image pass — its
    shader, its buffers, the RGBA8 atlas, per-image textures, and
    `Renderer::load_texture`); `widgets/image.rs` (task 16);
    `widgets/list.rs` (task 18's `List`).
  - **Integration**: module registration, `texture.rs`'s `STANDALONE` bit — which
    the first subagent's handoff correctly reported as documented but **not
    actually set**, so a renderer could not have told an atlas image from one with
    its own texture — the demo wiring, and the `List::set_palette` gap.
  Six new files, `paint.rs`, `batch.rs`, `render.rs` and `ui_demo` extended.
  **820 `ui_core` unit tests + 102 demo + 126 doctests**, `cargo fmt --check`
  clean, `cargo clippy --all-targets --all-features -D warnings` clean,
  `cargo doc --no-deps` with no warnings, and the aarch64 cross-build exit 0 with
  `Machine: AArch64` and SDL_image statically linked alongside SDL3 and FreeType.
  **`cargo audit` was not run — it is not installed on this host**, the standing
  tool gate from tasks 07, 09, 10 and 13, unchanged by this round.
  **Five subagents each ran 10–30 deliberate breaks and reported 0 survivors**;
  the integrator's own integration work (the `STANDALONE` bit, the palette gap,
  the scrollbar animation) was mutation-checked here.
  **Four acceptance criteria are waived per task** and named in the table; all
  of them need a pointer or a key, and the control that says so is task 12's
  button. **Nothing was rebuilt with a seed to reach a state the demo cannot get
  to** — the standing rule, and the reason the captures cover only the default
  state.
  **Two defects this round found in its own work**, both recorded in
  `NEVERAGAIN`: a deleted `#[test]` attribute that left the suite green with a
  test unregistered, and a test expectation that was wrong where the code was
  right (an `EaseInOut` colour transition's first frame rounds back to its start).
- 2026-10-01 — **the operator committed tasks 15–18 as `d7240c8`**, all four in
  one changeset as decided on 2026-09-30. The `Review` column reads `none` and
  that is still accurate: the batch was never reviewed by anybody, and the
  per-task revert point the workflow asks for does not exist for any of the four.
  A reviewer dispatched against `d7240c8` needs no working tree.
- 2026-10-01 — **the demo measures its own frame rate.** Operator request, not a
  task file: a readout at the foot of the window, a `roados-fps …` line on stdout
  when the demo stops, `ROADOS_RUN_SECONDS` to end a run, and
  `.ai/tools/fps-check.sh` to run all of it and judge the result. **The finding is
  the number: 49.7 fps on a release build, against a 60 fps target** — and the
  cause is the loop's 16 ms event wait plus the frame's own cost being serialised,
  so 62.5 fps is the ceiling of the loop's present shape. The operator decided
  this round measures and does not retime the loop. See *The frame rate, measured*.
- 2026-10-01 — **task 19 implemented — the text field, the on-screen keyboard, a
  typed-text event, and a demo band. Awaiting review, uncommitted.** Two new
  widgets (`widgets/text_input.rs`, 115 tests; `widgets/keyboard.rs`, 96 tests),
  one new `InputEventKind::Text` variant in `input.rs` with a `process` arm over
  SDL's `EVENT_TEXT_INPUT`, and the demo wired to both. **1053 `ui_core` + 140
  demo + 154 doctests**, fmt clean, clippy `-D warnings` clean, `cargo doc` clean,
  aarch64 cross-build clean with the same four dynamic dependencies.
  **Split per `.ai/protocols/subagents.md` § *Implementation fan-out*** —
  `input.rs` first as the shared prerequisite, then the two widgets as
  file-isolated subagents that do not import each other, then the demo here.
  **38 deliberate breaks run, 38 killed**, including one vacuous mutation of the
  integrator's own that was reported as a survivor and re-run properly.
  **The frame rate is 50.0 fps**, unchanged, and the two widgets cost about **1.1
  points of a core** measured interleaved against `HEAD`.
  **Three defects found and fixed**, two of them in the integrator's own wiring:
  an `Rc` around the field that made `set_palette` permanently unreachable so a
  theme switch did nothing to it; the palettes read from the theme *after*
  `switch_to` had begun animating it, which re-aims every widget at the palette it
  already had; and a readout bound to an empty field printing an empty line. The
  first two were caught by one test, which is now the reason it exists.
  **The window is 1020 tall because 1160 was not seeable**: this host caps the
  window at 1052 pixels, so the band's first arrangement — field over keyboard —
  put the keyboard's bottom off the bottom of the screen. Measured, and the band
  is now laid out side by side. **No acceptance criterion is waived**; none was
  verified through XTEST injection, which was not used.
  **`cargo audit` was not run** — not installed, unchanged, and no dependency
  changed, which is what it would have checked.
- 2026-10-01 — **a mutation runner's failure count was parsed off the whole log**,
  and every mutation was reported SURVIVED while the log showed 4 failures: the
  regex also matched a bare `" failed"` in cargo's `error: test failed` trailer,
  `head -1` took it, `cut` produced nothing, and `${failed:-0}` turned that into
  zero. The `grep -q '^test result'` guard did **not** catch it, because the log
  did contain a `test result:` line — a `FAILED` one. Added to `.ai/NEVERAGAIN.md`
  as a fourth mechanism: **a guard that checks presence is not a guard on
  content**, and an unparseable log must abort rather than default to "0 failures",
  because "0" is the one value that turns a broken runner into a confident report
  that the code is untested.
- 2026-10-01 — **the rule is wired into the AI system**, once and in one place:
  `.ai/agents/developer.md` § Phase 3 owns it, `.ai/workflows/task-sequence.md`
  § Gates and `.ai/agents/reviewer.md` § *Performance and idioms* point at it, and
  `.ai/tools/README.md` documents the tool with what it may not be used for.
- 2026-10-02 — **task 20 committed as `79941cd` without ever being reviewed**, and
  task 21 implemented in the same working tree, uncommitted. *No unreviewed
  advance* has now been skipped twice; see *Current position*. Task 21's four
  operator decisions are in *Ratified by the operator*.
- 2026-10-02 — **4x MSAA on the default framebuffer**, one line plus a constant
  in `render/context.rs`, on the operator's decision after being shown the three
  routes. The driver honoured it (`GL_SAMPLE_BUFFERS=1`, `GL_SAMPLES=4` read
  back from a live context) and the seam hunt found **no new dark pit anywhere in
  the window** — 19 before, 19 after, at the same coordinates. Every capture in
  this file is now historical, because every geometric edge in the application
  is antialiased. The gauge's *"There is no anti-aliasing"* section is
  superseded in place rather than rewritten.
- 2026-10-02 — **the solid pass does not premultiply**, found while measuring
  task 21 and **not fixed**: `render.rs:1528` blends `GL_ONE,
  GL_ONE_MINUS_SRC_ALPHA` and `quad_color` at `render.rs:617` normalises without
  scaling rgb by alpha, so a translucent solid primitive brightens over a lighter
  destination. **The text pass has the same latent defect** — `text_quad` at
  `render.rs:459` calls the same `quad_color` and the text shader scales by glyph
  coverage rather than by alpha — and is correct only because every text colour
  in the tree is opaque, which is a fact about the callers. The image shader is
  correct and stays. **The fix is one place, `quad_color`, and covers both.**
  *(This line first said the text and image shaders were both correct; the review
  caught it and it is corrected above as well as here.)* Pre-existing,
  whole-pipeline, and **recorded rather than fixed** because fixing it changes
  every translucent pixel in the application. See *Task 21*.
- 2026-10-02 — **the demo gives up the list and its readout** so the chart can
  have the column; the window cannot grow (1280×1320 requested, 1280×1052
  returned, measured). The demo loses its only scrolling viewport, its on-screen
  proof of virtualisation and the positional wheel-routing claim. All three are
  written into *Task 21 — what it decided* rather than left in a diff.
- 2026-10-02 — **two documentation claims in `chart.rs` were false and are
  corrected**, both found by measuring rather than by reading: a translucent
  fill's "no seam" was measured in a harness that drew a second chart underneath
  it, and `Chart::paint` claimed nothing reaches outside the node rect "with one
  measured exception, which is the stroke's own half width" when a mitred corner
  reaches `MITRE_LIMIT · half`. `Chart::stroke_reach()` now exists so the demo
  does not keep a private copy of the number.
