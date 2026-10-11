# Demo application — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins.

**Sequence:** `doc/ui/TASK_UI_DEMO_01..05.md`, designed by
`doc/ui/DEMO_APPLICATION.md`. The library sequence beside it is
`doc/ui/IMPLEMENTATION_STATE.md`. **Updated:** 2026-10-11.

## Current position

**`TASK_UI_DEMO_02` is in progress** — the three chrome regions are drawn over
the map and committed as `cb5de91`, *"partially implemented, layout not
finished"*. Not reviewed (`task-sequence.md` step 2). Its file is still in
`doc/ui/`, not `done/`.

**`TASK_UI_DEMO_02.1` is done** (2026-10-11, unreviewed): the airbag icon —
48 px white master, theme-tinted — sits in the status bar's airbag badge
beside its two-line text (operator redirect from the task file's dock cell).
Only the airbag exists. Built on task 44, landed the same day.

**Next: DEMO-03** — the car-status pane's three states, then 04 and 05. It
builds on DEMO-02's `panel.rs` page object model and `chrome.rs`.

## Left over

- **DEMO-02's layout is unfinished.** The pane does not resize, the carousel
  does not page, the dock's five slots open nothing, and the drive-mode
  strip's four targets have no gesture.
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
- **For DEMO-03:** `Page::Demo` is `--tab=demo`; the map is its full-bleed
  base, every chrome node a non-focusable `page_members` row hidden off-page
  by the three gates. Surfaces in `Demo::chrome_nodes`, buttons in
  `Demo::chrome_buttons`, labels in `Demo::chrome_labels`, pager in
  `Demo::chrome_pager`, icon in `Demo::icon_image` + `icon_spec`/`icon_tint`.
  `Panel` owns a rect plus a subtree (`set_rect`/`translate`/`resize` move
  both). A surface is `Blur(σ = 1.0)` + matching rounded fill at radius 12.

(End of file - total 58 lines)
