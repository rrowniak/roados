# UI primitives — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins and this file is corrected.

**Sequence:** `doc/ui/TASK_UI_PRIM_*.md`, designed by `PRIMITIVES.md` and
`PRIMITIVES_ARCHITECTURE.md`. Sibling state files:
`IMPLEMENTATION_STATE_DEMO.md` (the `TASK_UI_DEMO_*` sequence) and
`doc/platform/IMPLEMENTATION_STATE.md` (`TASK_CROSSPLATFORM_*`, deferred).

**Updated:** 2026-10-15.

## Current position

**Nothing in flight.** Task 44 (`Icon` + image tint) landed 2026-10-11, early:
`TASK_UI_DEMO_02.1` needed it and it was self-contained.

**Next: task 43** — `doc/ui/TASK_UI_PRIM_43.md`, `TabBar` + `Button::selected`,
closing gap `#7`. Then 45–50 and 52 in order; **51 waits on the operator** (see
*Left over*). Nine task files in `doc/ui/` are the queue.

**Suite:** `2121 passing / 1 ignored` as measured 2026-10-11 for tasks 44 +
DEMO-02.1 (`cargo test --all-features`); the ignored one is `layout_walk_cost`.
Re-measure before quoting it.

## Left over

- **Task 44's two spec deviations** (2026-10-11). (a) `list.rs`, `scroll.rs`
  and two exhaustive `Image` destructures each gained a `tint` line: the
  task's file list assumed only the doc-test destructured exhaustively, and
  the compiler disagreed. (b) `u32::try_from(ICON_SIZE)` does not exist in
  std, so the atlas-fit test asserts in the float domain through `u16`
  (`as` is banned).
- **Task 53's limit and deviations** (2026-10-10). Limit: `gltf`'s internal
  `unwrap`/`panic!` are reachable, so `load_from_bytes` claims no no-panic
  guarantee; the demo asset is trusted. Deviations: one uploaded `Mesh` with
  five ranges/transforms (single shared VBO/IBO); wheels compare against the
  body's own box, not whole-model bounds.
- **Task 51 needs the operator.** Gap `L10` (concave fill) was declined on
  2026-10-02. Task 51 ships `paint::polygon_is_convex` and escalates the
  reversal with its cost specified. Row `L10` stays open until then.
- **Task 41 left two criteria open**, both demo-side: the negative test
  `no_gallery_page_records_a_backdrop` (property covered for the chrome by
  another gate) and requirement 21's one-shot capture — **no capture has ever
  shown a backdrop drawing.** Both are in `doc/ui/done/TASK_UI_PRIM_41.md`.
- **Two `cargo doc` warnings** stand in `ui_core`, pre-existing and unfixed:
  `render.rs` links the private `IDENTITY_MAT3`, `render/mesh.rs` has a redundant
  explicit link target. Recorded 2026-10-09, not re-measured since.

(End of file - total 61 lines)
