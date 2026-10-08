# TASK_UI_PRIM_24.1: `Page`, the `--tab=` argument, and the three gates

## Goal

Give `ui_demo` a **page** — a named subset of its widgets — and make a page
selectable from the command line. Switching pages hides everything on the other
pages: not painted, not hit, not in the `Tab` order.

**Nothing moves on screen.** Every rectangle stays exactly where it is; this
sub-task changes which rectangles are drawn, not where they are. That is what
makes it separable from 24.2, and it is why every capture taken for tasks
11–22 is still a capture of the same pixels when this lands.

## Context

Parent: `doc/ui/TASK_UI_PRIM_24.md`. Read its Context first — the three
measured facts there are the whole of why this task has the shape it has.

**What already exists to copy.** `ui_demo` reads two environment variables and
**no arguments at all**: `main()` takes none (`ui/src/ui_demo/src/main.rs`),
and `run_seconds_from(raw: Option<String>) -> Option<Duration>`
(`run_seconds_from`) is the existing shape for "read a setting, parse it in a pure
function, unit-test the function". **The argument parser is written the same
way** — a pure function over a slice of strings, no I/O, no filesystem, no clock
— because a parser that reads `std::env` directly can only be tested by
mutating the process's environment, and one test doing that is one test that
cannot run beside another.

**Why three gates, restated as code.** The parent's Context carries the
citations; what belongs here is what each gate is *for*.

- `LayoutState::set_visible` is hit-testing only
  (`LayoutState::visible` and `LayoutState::set_visible` in
  `ui/src/ui_core/src/layout.rs`), and `hit_test` honours it by skipping the
  whole subtree (`hit_test_from`, in `ui/src/ui_core/src/input.rs`). So **one
  gate is free** — and it is free only for events that go through
  `Demo::route_input_event`.
- **Three helpers bypass that path**: `Demo::pad_at`, `Demo::slider_at` and
  `Demo::keyboard_at` are called straight from the `MouseButtonDown` and
  `FingerDown` arms of `Demo::handle_event`, because the demo needs the
  *press* and the gesture recogniser only reports a tap on the *release*. They
  ask their own rectangles and get an answer for a widget on another page.
- **Paint needs nothing from the library and everything from the demo.** The
  frame loop writes `PaintState::from_commands` for **every** handle in
  `self.order` with no `is_dirty` test (the `for handle in self.order` walk in
  `Demo::frame`), and `Demo::draw` sends every node in `order`.

**The one trap in this task, and it is a silent one.** An off-page node's
commands must be replaced with `PaintState::from_commands(Vec::new())` and
**not** with `PaintState::new()`: the latter is `dirty: false`
(`PaintState::new`, `ui/src/ui_core/src/paint.rs`), so the renderer keeps the batch it
already has and **the old page stays on screen** with every test that reads a
recorded command still green, because the commands are only *stale*, not
*absent*. **A cache invalidated in the wrong order is a cache that lies** — the
same failure one layer down.

**The test migration is the largest part of this task, and it should be counted
before anything else is written.** The three test helpers — `tests::demo()`,
`tests::laid_out()` and `tests::dialog_closed()` — are what every `#[test]`
function in the file goes through. Any test whose subject is off the default page
needs the page activated before its first frame.

**The count is a measurement with a date on it, because the file is being
edited.** At **2026-10-03 16:36**, `grep -c '^    \[test\]'
ui/src/ui_demo/src/main.rs` was **157**. Grouping the test *names* by subject —
an `awk` over the file, collecting the `fn` above each `#[test]` — accounts for
**99** of them: slider 18, dialog 16, pads 15, gauge 12, chart 10, label 8,
keyboard 7, toggle 6, image 6, progress 4, band 3. The rest are theme, frame,
layout, focus and route tests, of which the two collision tests change **scope**
rather than gaining a line (see 24.2). **So the migration is ≈100 tests, and
both figures must be re-measured at the start of this task** — they were still
moving while this file was written.

## Requirements

1. **`enum Page`** with one variant per page from the parent's requirement 1,
   in that order, plus:

   - `Page::ALL` — the six, in order. One list, so the `--help` text, the
     unknown-name error, the tab bar's buttons and the tests cannot disagree.
   - `Page::name(&self) -> &'static str` — the lowercase name, which is the
     `--tab=` value and the error message's word.
   - `Page::from_name(&str) -> Option<Page>`.
   - `Page::DEFAULT`, named rather than left to enum order.

2. **Page membership is one table, not a `match` per call site.** A
   `fn shows(&self, page: Page, handle: Handle) -> bool` over a per-page list
   of handles, built once in `Demo::new`. **A `match` here would be a second
   list** — the hazard the 2026-10-03 review named about `GALLERY_SHORTCUTS`
   and the reason that table *is* the dispatch.

3. **`--tab=<name>` selects the initial page, `--help` prints the usage and the
   six names and exits 0, an unknown name exits non-zero and prints the six
   valid names.** Parsed by a pure function over a slice — the shape
   `run_seconds_from` already has — and `Demo::new` **takes the page**, so
   there is no frame in which the wrong page is painted. The default page is
   the parent's, named in the task file.

4. **Paint gate.** The frame loop writes empty commands for an off-page node,
   with `from_commands(Vec::new())`. Two consequences to hold: `Demo::draw` and
   `Demo::frame_clips` need **no** change, because an empty command list draws
   nothing; and a test that reads `Demo::commands_at` for an off-page node gets
   an empty vector, which is the honest answer.

5. **Hit-test gate.** `set_visible(false)` on every off-page node, refreshed on
   a switch — it does not mark dirty (`LayoutState::set_visible`), so a switch costs
   nothing and **no rect moves**. Plus a page guard in `Demo::pad_at`,
   `Demo::slider_at` and `Demo::keyboard_at`.

6. **Focus gate.** `Demo::focus_navigation`'s hardcoded five-element array
   becomes the active page's focusables **plus the six tab buttons** — which do
   not exist until 24.3, so until then it is the active page's focusables and a
   note saying where the buttons arrive.
   `Demo::set_focus(None)` on every switch: a control on the page just left
   must not keep its focus ring, and `Demo::focused` is the record the paint
   arms read.

7. **Every shortcut activates its own page first, then acts** — requirement 6 of
   the parent. The page is a field on the `GALLERY_SHORTCUTS` row, so the table
   remains the single dispatch, and the row count and its arity change with it.

8. **The frame-rate readout and the background are on every page.** The
   operator's 2026-10-02 instruction was *"Keep fps label"*, so it is not page
   content; it belongs to the always-painted set.

## Acceptance Criteria

- [ ] `--tab=<name>` opens on that page for all six names
- [ ] `--tab=nope` exits non-zero and its message names all six valid pages
- [ ] `--help` lists the six pages and exits 0
- [ ] A page records **no** draw command on a node that is not its own —
      asserted for every page, over the commands of every node in
      `Demo::order`
- [ ] `Demo::commands_at` returns **empty**, not stale, for an off-page node —
      asserted by switching away and back and reading the node twice
- [ ] A press on an off-page pad, slider, toggle, field or key changes nothing,
      through `Demo::handle_event` and not through a helper called directly
- [ ] `Tab` walks only the active page's controls, forward and backward, for
      every page
- [ ] A switch retires focus: no control on the page just left has
      `focused` set
- [ ] All eighteen shortcuts work from every page, and each lands on the page
      its widget is on
- [ ] `WINDOW`, every `*_ORIGIN` constant and every placed rect are
      **byte-identical** to the values before this task — the assertion is that
      no pixel moved
- [ ] `fps-check.sh` at or above the recorded baseline, with the default page
- [ ] The whole suite is green with no assertion weakened, and every test the
      migration touched is listed in the handoff by name

## Deliberate breaks

Four, each expected to fail before it is fixed back:

1. Paint gate removed → the off-page command test fails.
2. `from_commands(Vec::new())` swapped for `PaintState::new()` → **the whole
   suite stays green**, which is the point of the requirement and the reason
   the capture is also required. Reported as a surviving mutation, not as a
   pass.
3. `set_visible` removed → the routed-tap test fails, and the three direct
   helper tests still pass, which is the asymmetry requirement 5 exists for.
4. `set_focus(None)` removed from the switch → the focus-retirement test fails.

**Item 2's second clause is void, and it is void because the premise above it
did not hold.** Measured 2026-10-04 from the source, after three review rounds:
`Renderer::begin_frame` (`render.rs:1843`) does `gl.clear(GL_COLOR_BUFFER_BIT)`
**and** `self.batcher.reset()`; `Batcher::reset` (`batch.rs:257`) clears `open`
and `sealed`; `draw_node_clipped` (`render.rs:1895`) returns early on a
non-dirty node and otherwise `take_commands()`s into **this frame's** batcher;
and `Renderer`'s field list holds **no per-node command cache**. **A non-dirty
node therefore contributes nothing and nothing stale survives the clear**, so
the two forms are equivalent **on screen** as well as in every recorded-command
assertion — `PaintState::commands` is empty either way, and `commands_at` reads
`commands()` and not `take_commands()`. **No capture distinguishes them**, which
is the opposite of what the Context's *"the commands would be stale, not absent"*
argument requires. Requirement 4's literal form was kept; what would close the
difference is a renderer caching commands per node between frames, which is a
`ui_core` change and out of scope. **The capture is still required — for the page
gate and for proving no rect moved, both of which it does do.**

## Out of Scope

- **Moving anything.** 24.2 owns the geometry; this task's criterion is that it
  does not.
- **The tab bar.** No button, no row, no selected appearance — 24.3.
- **`ui_core`.** No visibility change, no page widget, no new module. If a
  library change turns out to be necessary, that is a stop condition and a
  report, not a widening of scope (`.ai/agents/developer.md` § *Do not silently
  widen scope*).
- **A `--widget=` alias.**
- **A page-switch animation.** See the parent's out-of-scope list.