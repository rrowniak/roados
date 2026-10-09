# Demo application — implementation state

**Purpose:** so a fresh session resumes at the right task in the `TASK_UI_DEMO_01..05`
sequence and does not redo finished work. This is working state, not a spec: the
task files own the requirements, and where this file and a task file disagree, the
task file wins and this file gets corrected.

**Spec:** `doc/ui/DEMO_APPLICATION.md` and `doc/ui/TASK_UI_DEMO_01..05.md`.

**Last updated:** 2026-10-09 (`TASK_UI_DEMO_01` implemented, verified, record written,
not yet reviewed).

## Current position

**Status: `TASK_UI_DEMO_01` is done — implemented, verified, record written on
2026-10-09.** The seventh page (`Page::Demo`, `--tab=demo`) exists, with a
full-bleed map image drawn under the tab bar and over the window background. The
image is loaded through the existing `Renderer::load_texture` path; a missing file
falls back to a transparent stand-in of the operator's image's own shape
(`1359 × 970`).

**A fix after the first run:** the operator's image sits at `/workspace/img/map_demo.png`
(the repository root), which the crate-relative [`MAP_IMAGE_RELATIVE`]
(`src/ui_demo/assets/img/map_demo.png`) does not reach when walking up from
`/workspace/ui/target/release/ui_demo`. `map_candidates` now also checks an
`img/<file>` path at every ancestor, so the file is found without requiring
`ROADOS_ASSET_DIR`.

**What this closes:** the visible half of gap `#1` — the demo now has a page to put
panels on, and that page has a base layer. **What it does not close:** gap `#1`
itself, because the map is a picture and not a map widget — no camera, no pan, no
zoom, no route, no marker, no POI, and nothing addressable in the picture's own
coordinates. `DEMO_APPLICATION.md` § *Library gaps* row 1 keeps its numbering and
its *"Map/navigation screen"* Blocks entry.

**The next task after DEMO-01 is DEMO-02** (`doc/ui/TASK_UI_DEMO_02.md`: the tab
shell and the persistent chrome). DEMO-03, DEMO-04 and DEMO-05 are specified and
not started.

## Task table

`AC waived` records acceptance criteria that were met with a reason instead of
verified. A blank cell is unknown, not "none".

| # | Task | Status | Commit | Review | AC waived |
|---|---|---|---|---|---|
| DEMO-01 | The demo page — a seventh tab, and the map image under it; **gap `#1`, partly** | **implemented 2026-10-09, verified, record written, not yet reviewed** | `—` — awaiting the operator's commit (`.ai/workflows/task-sequence.md` step 5) | **none yet** — review is step 2, in a session separate from the implementer's | **The capture criterion is waived** — the operator's map image is not in this repository, so a fresh clone cannot reproduce the capture of the supplied image. The waiver's reason is that the file is not in the tree; if it is ever committed, the waiver must be withdrawn and the criterion met. See § *Task DEMO 01 — what it decided* |
| DEMO-02 | The tab shell and the persistent chrome | specified 2026-10-05, not started | `doc/ui/TASK_UI_DEMO_02.md` | — | — |
| DEMO-03 | The car-status pane and its three states | specified 2026-10-05, not started | `doc/ui/TASK_UI_DEMO_03.md` | — | — |
| DEMO-04 | The indicator-light column | specified 2026-10-05, not started | `doc/ui/TASK_UI_DEMO_04.md` | — | — |
| DEMO-05 | Card carousel, callout hotspots, the two-axis reshape | specified 2026-10-05, not started | `doc/ui/TASK_UI_DEMO_05.md` | — | — |

## Task DEMO 01 — what it decided, and what it does not claim

**The map is a picture, not a map widget.** `TASK_UI_DEMO_01.md` § *Context* records
that `Renderer::load_texture` already reads from disk and that the demo already
calls it for `demo.png`; the only missing piece was a page to draw it on. The
result is one `Image` widget with `ImageFit::Cover`, full-bleed below the tab bar.

**What the page is for.** `Page::Demo` is the infotainment screen this repository's
direction describes. Today it holds the map image and the tab bar and nothing else;
`TASK_UI_DEMO_02` through `TASK_UI_DEMO_05` fill it in, one panel each. The page's
doc comment says so in those words rather than describing a finished screen.

**What is not built.** No chrome, no car-status pane, no indicator column, no card
carousel, no car marker, no route, no POI, no compass, no scale bar, no attribution
line, no zoom controls. No coordinates, no camera, no projection, no pan, no zoom,
no raster tile layer, no zoom levels, no tile cache. `ui_core` has no new API: no
`DrawCommand` variant, no `Painter` method, no `ThemeToken`, no `LayoutMode`, no
pixels-to-texture entry point.

**The honest limits in the record's own register.** The map does not pan, does not
zoom, does not rotate, carries no route, marker or POI, and gap `#1` is not closed.

**The licence acceptance.** The image was captured from Google Maps. The operator
accepted the risk for a **local, undistributed** run on 2026-10-09. The image is not
committed; if it is ever committed, published, or shipped, that acceptance is void
and the terms apply as written. `TASK_UI_DEMO_01.md` § *The licence, and what was
accepted* states the clauses.

**The capture criterion, and the file that changed it.** `task-sequence.md` § *Gates*
says *"An unverifiable acceptance criterion may be waived with a reason."* The
reason was that the image is not in the tree. After the `map_candidates` fix the
operator's file at `/workspace/img/map_demo.png` is found by the release binary, so
the capture taken for this handoff shows the actual map image. The waiver remains
in force for a fresh clone, because the file is still not committed.

**Test count.** Suite `2058 passing / 1 ignored` before this task; `2064 passing /
1 ignored` after it. The six new tests are in `ui/src/ui_demo/src/main.rs`'s
`mod tests` and are named:

- `the_seventh_page_is_named_demo_and_the_default_did_not_move`
- `the_seventh_tab_button_lays_out_inside_the_window`
- `the_map_node_is_full_bleed_below_the_tab_bar`
- `the_demo_page_draws_the_map_and_the_gallery_pages_do_not`
- `the_map_is_an_image_at_the_node_s_own_rect_with_the_cover_fit`
- `the_map_node_is_not_in_placed_handles_and_the_two_counts_did_not_move`

No existing test was deleted, renamed or weakened.

**Frame rate, measured on all seven pages.** Floor 55 fps. `fps-check.sh 10 55` on
the default page, and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>`
for each of the seven:

| page | frames | duration (s) | average fps | worst frame (ms) | long frames |
|---|---|---|---|---|---|---|
| pads | 632 | 10.013 | 63.1 | 17.2 | 0 |
| text | 625 | 10.011 | 62.4 | 24.2 | 0 |
| input | 628 | 10.007 | 62.8 | 19.7 | 0 |
| controls | 624 | 10.014 | 62.3 | 23.5 | 0 |
| data | 622 | 10.014 | 62.1 | 42.3 | 1 |
| overlays | 616 | 10.015 | 61.5 | 29.4 | 0 |
| demo | 627 | 10.016 | 62.6 | 31.5 | 0 |

Default page (`pads`) from `fps-check.sh`: **63.1 fps**, worst frame 22.4 ms, 0 long
frames. Every page is above the 55 fps floor.

**Files changed.** `ui/src/ui_demo/src/main.rs` and `.gitignore` only. `ui_core` has
no diff. `ui/Cargo.toml` and `ui/Cargo.lock` are unchanged. No asset is committed;
`/img/` and `/ui/src/ui_demo/assets/img/` are ignored.

**What the handoff does not claim.** The map is a picture and not a map widget; it
does not pan, does not zoom, does not rotate and carries no route, marker or POI;
gap `#1` is not closed; the image is not in the repository, so a fresh clone's
capture shows the stand-in and not the map; the image's licence was accepted as a
local-use risk by the operator on 2026-10-09 and is void if the image is ever
distributed; and `check_assets.py`'s denylist does not cover `assets/img/`, because
it skips `.png` contents.

## What DEMO-02 inherits

- `Page::Demo` exists and is reachable at `--tab=demo`.
- The map node is attached to `root` immediately after `background` and before
  `tab_bar`; anything painted on top of the map must be added after it in paint
  order.
- The map node is a `page_members` row on `Page::Demo` and is not focusable.
- The map's rect is `(0, CONTENT_TOP)` to `(WINDOW.width, WINDOW.height)`, i.e.
  `1280 × 956`.
- The map image is not in the repository; any capture that needs the operator's
  picture must be taken with the file in place.
