# Demo application — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins.

**Sequence:** `doc/ui/TASK_UI_DEMO_01..05.md`, designed by
`doc/ui/DEMO_APPLICATION.md`. The library sequence beside it is
`doc/ui/IMPLEMENTATION_STATE.md`. **Updated:** 2026-10-15.

## Current position

**Nothing in flight.** `TASK_UI_DEMO_02` (chrome) and `TASK_UI_DEMO_03`
(car-status pane) landed 2026-10-15: their files are in `doc/ui/done/` and
are the record.

**Next: DEMO-04** — `doc/ui/TASK_UI_DEMO_04.md`, the indicator-light column;
then 05. Two task files in `doc/ui/` are the queue.

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

(End of file - total 58 lines)
