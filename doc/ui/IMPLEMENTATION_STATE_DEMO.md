# Demo application — implementation state

**Purpose:** a status board for a fresh session — what is in flight, what is
next, what is left over. **Nothing else.** A finished task is not here: its file
in `doc/ui/done/` is its record. Where this file and a task file disagree, the
task file wins.

**Sequence:** `doc/ui/TASK_UI_DEMO_01..05.md`, designed by
`doc/ui/DEMO_APPLICATION.md`. The library sequence beside it is
`doc/ui/IMPLEMENTATION_STATE.md`. **Updated:** 2026-10-10.

## Current position

**`TASK_UI_DEMO_02` is in progress** — the three chrome regions (status bar,
car-status pane, dock) are drawn over the map and committed as `cb5de91`,
*"partially implemented, layout not finished"*. Not reviewed
(`task-sequence.md` step 2). Its file is still in `doc/ui/`, not `done/`.

**Next: DEMO-03** — the car-status pane's three states, then 04 and 05. It
builds on DEMO-02's page object model (`ui/src/ui_demo/src/panel.rs`) and
chrome (`ui/src/ui_demo/src/chrome.rs`).

## Left over

- **DEMO-02's layout is unfinished**, and the commit says so rather than saying
  which parts. What it does **not** do, from its own record: the pane does not
  resize, the carousel does not page, the dock's five slots open nothing, and the
  drive-mode strip's four targets have no gesture.
- **The dock's five glyphs are placeholders** (`DOCK_GLYPHS`), against the
  operator's 2026-09-30 *"real icons, no placeholder shapes"* decision.
  `TASK_UI_PRIM_44.md`'s `ui_core::widgets::Icon` is the fix.
- **No screenshot was taken** for DEMO-02: neither the demo page nor the
  six-gallery-page `AE 0` comparison. Frame rate was measured (61.1–62.6 fps,
  floor 55, 2026-10-09).
- **DEMO-01's capture criterion is waived** — the operator's map image
  (`/workspace/img/map_demo.png`) is not in the repository, so a fresh clone
  cannot reproduce it. Its licence was accepted for **local, undistributed** use
  on 2026-10-09 and is void if the image is ever committed or shipped.
- **Gap `#1` is not closed.** The map is a picture: no camera, no pan, no zoom,
  no route, no marker, no POI.

## What DEMO-03 inherits

- `Page::Demo` is reachable at `--tab=demo`; the map is its full-bleed base
  layer, and every chrome node is a non-focusable `page_members` row on that
  page, so the paint, hit-test and focus gates already hide the chrome on the six
  gallery pages. It is attached to `root` **after** the controls layer: anything
  drawn over it must come later in paint order.
- `Chrome` surfaces live in `Demo::chrome_nodes`, buttons in `Demo::chrome_buttons`,
  labels in `Demo::chrome_labels`, the pager node in `Demo::chrome_pager`.
- `Panel` (`panel.rs`) owns a rect in its parent's coordinates plus a widget
  subtree; `set_rect` / `translate` / `resize` move the object and the subtree
  follows. Use `Panel`, not window coordinates — it is what makes the pane
  resizable.
- A surface is `BackdropMode::Blur(σ = 1.0)` plus a matching rounded fill,
  `CHROME_RADIUS` 12 in both themes. σ = 1.0 rather than 2.5 because the `demo`
  page needs it for the frame rate; the capture behind each backdrop is
  window-sized, so each surface costs two blur passes.