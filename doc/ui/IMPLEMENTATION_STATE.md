# UI primitives — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins and this file is corrected.

**Sequence:** `doc/ui/TASK_UI_PRIM_*.md`, designed by `PRIMITIVES.md` and
`PRIMITIVES_ARCHITECTURE.md`. Sibling state files:
`IMPLEMENTATION_STATE_DEMO.md` (the `TASK_UI_DEMO_*` sequence) and
`doc/platform/IMPLEMENTATION_STATE.md` (`TASK_CROSSPLATFORM_*`, deferred).

**Updated:** 2026-10-10.

## Current position

**Nothing in flight** — no task of this sequence is half-implemented. The
`DEMO` sequence is the one with work open; see `IMPLEMENTATION_STATE_DEMO.md`.

**Next: task 43** — `doc/ui/TASK_UI_PRIM_43.md`, `TabBar` + `Button::selected`,
closing gap `#7`. Then 44–50 and 52 in order; **51 waits on the operator** (see
*Left over*). Ten task files in `doc/ui/` are the queue.

**Suite:** `2085 passing / 1 ignored` as last measured at `cb5de91`; the ignored
one is `layout_walk_cost`. Re-measure before quoting it.

## Left over

- **Nine tasks never reviewed** — `task-sequence.md` step 2, in a session
  separate from the implementer's, was never run for **32, 34, 35, 36, 37, 38,
  39, 40, 41**. All are implemented, verified and committed. This is the largest
  unpaid debt in the sequence.
- **Task 51 needs the operator.** Gap `L10` (concave fill): ear clipping and a
  stencil pass were both declined on 2026-10-02. Task 51 ships
  `paint::polygon_is_convex` and escalates the reversal with its cost already
  specified, so that "yes" is an instruction rather than another round.
  `DEMO_APPLICATION.md` § *Library gaps* row `L10` stays open until it is.
- **Task 41 left two acceptance criteria open**, both demo-side and both chosen
  against the operator's instruction not to touch the demo: the named negative
  test `no_gallery_page_records_a_backdrop`, which does not exist in
  `ui/src/ui_demo/src/main.rs` (the property it would have held is covered for
  the chrome by `every_chrome_node_records_commands_on_the_demo_page_and_on_no_other_page`),
  and requirement 21's one-shot capture instrument — **no capture has ever shown
  a backdrop drawing.** Both are in `doc/ui/done/TASK_UI_PRIM_41.md`.
- **Two `cargo doc` warnings** stand in `ui_core`, pre-existing and unfixed:
  `render.rs` links the private `IDENTITY_MAT3`, `render/mesh.rs` has a redundant
  explicit link target. Recorded 2026-10-09, not re-measured since.