# Demo application — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins.

**Sequence:** `doc/ui/TASK_UI_DEMO_01..05.md`, designed by
`doc/ui/DEMO_APPLICATION.md`. The library sequence beside it is
`doc/ui/IMPLEMENTATION_STATE.md`. **Updated:** 2026-10-12.

## Current position

**`TASK_UI_DEMO_02` is in progress** — the three chrome regions are drawn over
the map, *"partially implemented, layout not finished"*. Not reviewed. Its
file is still in `doc/ui/`, not `done/`.

**`TASK_UI_DEMO_03` was built and then removed** (2026-10-12): the pane, its
three states and its 3-D car landed uncommitted, the operator rejected the
looks, and then the rest of the left panel's contents followed — task 02's
chrome pane subtree (PRND row, drive strip, indicators, card labels, pager)
is gone too. What stands: the pane's bare translucent surface (empty, as the
operator expects), the `DATA_*` rename, the status bar, the dock, and the
map. The task files stay in `doc/ui/` as the rebuild spec. Not reviewed.

**Next: DEMO-03 rebuilt** — the car-status pane from scratch on the clean
base (map + translucent chrome), then 04 and 05. First row landed 2026-10-12:
`PRND` with Park green, `70%` + level icon, three tests green.

## Left over

- **DEMO-02's chrome is now bar, bare pane surface, and dock.** The pane's
  contents are gone (see *Current position*): no PRND row, drive-mode strip,
  indicator column, cards or pager. The dock's five slots open nothing.
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
- **For the DEMO-03 rebuild:** `Page::Demo` is `--tab=demo`; the map is its full-bleed
  base, every chrome node a non-focusable `page_members` row hidden off-page
  by the three gates. Surfaces in `Demo::chrome_nodes`, buttons in
  `Demo::chrome_buttons`, labels in `Demo::chrome_labels`, pager in
  `Demo::chrome_pager`, icon in `Demo::icon_image` + `icon_spec`/`icon_tint`.
  `Panel` owns a rect plus a subtree (`set_rect`/`translate`/`resize` move
  both). A surface is `Blur(σ = 1.0)` + matching rounded fill at radius 12.

(End of file - total 58 lines)
