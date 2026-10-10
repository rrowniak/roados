# TASK_UI_DEMO_01: The Map Surface — a Procedural World, a Camera, and Four Draw Commands

> **SUPERSEDED 2026-10-09 — not built, and not to be built as written.**
> Replaced by [`doc/ui/done/TASK_UI_DEMO_01.md`](../done/TASK_UI_DEMO_01.md), which draws
> a **map image as a background** and puts nothing on it. **Its reasoning is
> retained (compacted 2026-10-10)** because it is why the replacement is cheap:
> § *The map approach* proves a raster background was reachable all along
> (`Renderer::load_texture` reads from disk, and `IMPLEMENTATION_STATE.md`
> already records that *"a committed PNG would work today"*), and § *What this
> approach cannot do* is an honest inventory of what the procedural design cost.
> **Read those two sections before rewriting the replacement.** Everything below
> describes a decision that was not taken; the operator's decision and its
> reasons are in § *Decisions taken instead* at the end of this file.

## The map approach

The task would close gap **#1** of `DEMO_APPLICATION.md` § *Library gaps* — row 1, severity Critical, its *Blocks* entry *"Map/navigation screen"* — with **no `ui_core` change**, because § *Task structure* decided *"the map is the demo's own asset and no library owns it."*

The design was a procedural vector map at a fixed camera, generated in `ui/src/ui_demo/src/map.rs` and drawn only from the four existing draw commands `Rect`, `Path`, `Circle` and `Polygon` — no new dependency and no `unsafe`.
It also created the demo's seventh page, `Page::Demo` with `--tab=demo`; the chrome was `TASK_UI_DEMO_02`.

A seeded world generator, `pub fn world(seed: u64) -> World`, uses an LCG written out inline and is a pure function of the seed.
`MAP_METRES = 1600.0` makes a square world; `CAMERA_PIXELS_PER_METRE = 0.7`; `MAP_SEED = 1_812_045`.
`World` holds blocks, roads, a route, POIs and a car: a 9 × 6 block grid, 54 blocks; 20 roads in three `RoadClass` classes — `Service`, `Secondary`, `Arterial` — at widths `ROAD_WIDTH_SERVICE` 2.0, `ROAD_WIDTH_SECONDARY` 4.0 and `ROAD_WIDTH_ARTERIAL` 6.0; a three-point route at `ROUTE_WIDTH` 8.0; three `PoiKind` values (`Charge`, `Food`, `Fuel`); and `Car { at, heading_radians }`.

`Camera { centre, pixels_per_metre }` carries a `project` function, a ten-field `MapPalette`, `rotated` for the car nose, and `MapSurface::paint` returning the command list.
The record order *is* the z-order, because every command is opaque: ground `Rect`, block `Rect`s, roads minor class first, the route, POIs (`Circle` then `Polygon`), the car (`Circle` body then a rotated `Polygon` nose).
Round joins and round caps are one `Circle` per vertex, both ends included.
In `main.rs` the wiring is `const ALL: [Page; 7]`, `Page::Demo => "demo"` in `fn name`, `Page::DEFAULT` left at `Page::Pads`, one `page_members` row and one arm in `Demo::frame`'s paint walk.

### The raster route

§ *The map approach* establishes that a pre-rendered raster tile layer — `Pixels` → `TextureCache` → `DrawCommand::Image` — is not reachable from `ui_demo`: `Renderer`'s only texture entry point is `pub fn load_texture(&mut self, path: &Path)`, which decodes from disk; `Renderer::textures()` returns `&TextureCache` and not `&mut`; `TextureCache::load` needs `&mut self`; and `textures_mut`, `make_texture`, `upload_texture` and `from_pixels` do not exist, nor does any of tasks 41 to 52 add one. `fn stand_in_picture` proves the shape is closed by dropping its cache.

But the same section records a second route to a raster that was reachable all along: commit a PNG to `ui/src/ui_demo/assets/` and call `Renderer::load_texture`.
It was refused for three reasons — it settles the map-source open question by omission, it would need `TASK_UI_PRIM_39`, and a minified texture aliases under `GL_LINEAR` with no mipmaps — and `IMPLEMENTATION_STATE.md` already records that *"a committed PNG would work today."*
This is the route the replacement takes.

A mesh ribbon through `DrawCommand::Mesh` was refused as over-engineering and unavailable: it is task 37's variant, needs tasks 34–37, none built, and a road is a screen-space stroke.

## What this approach cannot do

From § *What this approach cannot do*, condensed:

- No zoom, no pan, no map rotation: the `Camera` is fixed, and a pan needs `TASK_UI_PRIM_46` (`L5`).
- A curve is a polyline; there is no bezier anywhere in `ui/src`.
- A road's width is pixels, not metres, so it is correct at one camera scale; a zoom would re-derive every width.
- A mitre join is not available: `command_quads` gives a `Path` one quad per segment and no joins; a round join's shortfall is `w / 2 · (1 / sin(θ / 2) − 1)`.
- Off-window geometry is submitted, not clipped: `Demo::frame_clips` returns `None` for every node, so a road touching the viewport is recorded whole.
- No raster, so no texture memory and no mipmap problem — and also no satellite imagery, labels, turn restrictions, lane geometry or traffic.
- The car marker rotates by hand, because `DrawCommand` carries no transform and `TASK_UI_PRIM_36` § *Requirements* reserves `Mat4` and `u_model` for `DrawCommand::Mesh`.
- `Polygon` cannot draw a concave silhouette, so the marker is a disc plus a triangle and each POI glyph is the simplest convex shape.

## Decisions taken instead

§ *Decisions taken instead* records the operator's decision of 2026-10-09: the scope was judged too ambitious. The replacement is a background image in `assets/img/`, loaded through the mechanism `demo.png` already uses, drawn full-bleed under the tab bar with nothing on top; `TASK_UI_DEMO_02`'s chrome and `TASK_UI_DEMO_03`'s car-status pane are what put things on it.

Three consequences are recorded: the image's licence is an accepted risk, not a permission held; `ui/src/ui_demo/assets/img/` is deliberately not in the repository, reversing what `TASK_UI_PRIM_39` decided for that directory, so the seventh page's capture is not reproducible from a fresh clone; and gap #1 is not closed by either file, because a picture is not a map widget.

What would bring the file back: a camera that moves (`TASK_UI_PRIM_46`, `L5`) or a pixels-to-texture entry point in `ui_core`. Either makes the procedural design cheaper than the replacement; neither exists today.
