# Demo application — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins.

**Sequence:** `doc/ui/TASK_UI_DEMO_01..06.md`, designed by
`doc/ui/DEMO_APPLICATION.md`. The library sequence beside it is
`doc/ui/IMPLEMENTATION_STATE.md`. **Updated:** 2026-10-15.

## Current position

**In flight: `TASK_UI_DEMO_06`** (the Controls panel) — implemented, suite green,
**awaiting review and the operator's commit**. Its file is still in `doc/ui/`.
It was built after `TASK_UI_PRIM_47`'s `Segmented` was added to `ui_core`
(`ui/src/ui_core/src/widgets/segmented.rs`) — the hard prerequisite the task
names — but **47 is not otherwise built**: `ModeScope`, `Snapshot`, the climate
strip and its `DEMO_APPLICATION.md` closure are not there, so 47's file is not
closed either. **DEMO-05 was skipped** by operator direction; it remains
`doc/ui/TASK_UI_DEMO_05.md`.

**Next: DEMO-05**, or the operator's call. PRIM-43/45/46/48/49/50/51/52 and 47's
rest are `doc/ui/TASK_UI_PRIM_*.md`; `PRIM-42` never landed despite its file
sitting in `doc/ui/done/` (no `nav.rs` in the tree).

## Left over

- **The dock's five slots open nothing.**
- **Four dock glyphs are still placeholders**, against the operator's
  2026-09-30 *"real icons"* decision. The 33-glyph inventory is a later task's.
- **Captures taken 2026-10-11** (release, `/tmp`, uncommitted): icon white at
  48 px on the demo page; five gallery pages `AE = 0` above the fps band; tab
  strips and the data image region `AE = 0`. The car's edges speckle run to
  run (same-binary `AE = 5854` beats old-vs-new `2995`) — pre-existing noise.
  Frame rate 60.3–64.2 fps on all seven pages (floor 55).
- **`check_assets.py --all` was already red** before the two PNGs (no
  `icons.json`, no `assets/icons/LICENSE`) and is unchanged after them: no
  manifest covers the masters.
- **DEMO-01's capture criterion is waived** — the operator's map image is not
  in the repository. Its licence was accepted for **local, undistributed** use
  on 2026-10-09 and is void if the image is ever committed or shipped.
- **Gap `#1` is not closed.** The map is a picture: no camera, no pan, no zoom,
  no route, no marker, no POI.
- **DEMO-06's deviations** (2026-10-15): tiles are three arithmetic rows, not
  `Flex` rows (a `Flex` parent ignores a parked position); hidden panes park as
  disjoint zero-size points (`touches` counts coincident zero-size boxes as
  touching).
- **DEMO-06's rendering caveat** (2026-10-15): the panel's opaque children and
  the surface's translucent fill land in different passes (opaque before
  transparent), so the surface composites over them and the white selection pill
  reads frosted grey — legible, not white. The chrome's, not the panel's.
  `ROADOS_OPEN_PANEL=<category>` opens the panel at launch: no pointer here.

(End of file - total 58 lines)
