# TASK_UI_DEMO_01: The Map Surface — a Procedural World, a Camera, and Four Draw Commands

> **SUPERSEDED 2026-10-09 — not built, and not to be built as written.**
> Replaced by [`doc/ui/done/TASK_UI_DEMO_01.md`](../done/TASK_UI_DEMO_01.md), which draws
> a **map image as a background** and puts nothing on it. **This file is kept
> whole** because its reasoning is why the replacement is cheap: § *The map
> approach* proves a raster background was reachable all along
> (`Renderer::load_texture` reads from disk, and `IMPLEMENTATION_STATE.md`
> already records that *"a committed PNG would work today"*), and § *What this
> approach cannot do* is an honest inventory of what the procedural design cost.
> **Read those two sections before rewriting the replacement.** Everything below
> describes a decision that was not taken; the operator's decision and its
> reasons are in § *Decisions taken instead* at the end of this file.

## Goal

Close **gap #1** — `DEMO_APPLICATION.md` § *Library gaps*, row **1**,
*"**Map widget** — no map renderer exists or is planned. The demo's centerpiece.
Critical"* — by giving `ui_demo` a map it can put things on top of: **a ground, a
block layer, three classes of road, a route, three points of interest and a car
marker**, all drawn with primitives `ui_core` already has, with **no new
dependency and no `unsafe`**.

**The decision is a procedural vector map, drawn from draw commands, at a fixed
camera.** § *The map approach* argues it against the two alternatives and §
*What this approach cannot do* is the honest half of the same argument: **no
raster tile layer is reachable from `ui_demo` today**, a curve is a polyline, a
road's width is pixels and not metres, and there is no transform in any 2-D pass,
so the car marker is rotated by hand. The map is the thing every later
`TASK_UI_DEMO_n` surface is defined against — `DEMO_APPLICATION.md` § *Design
principles* item 2 (*"the map is the base screen; every other surface is a panel
over the map"*) and § *What this means for the demo's shape* item 1 (*"the demo
must have a map it can put things on top of"*) both say so — and none of it
exists.

**This task also creates the seventh page.** `DEMO_APPLICATION.md` § *Relationship
to task 24* records that the shape of the demo's tab *"is not settled here"* and
that *"a `TASK_UI_DEMO_1.md` has not been written"*; this file is that task, and
`Page::Demo` with `--tab=demo` is the minimum shell needed for the map to be
**seen**, which `developer.md` § Phase 3 requires of any change to what is on
screen. The **chrome** — the top status bar, the bottom dock and the car-status
pane — is `TASK_UI_DEMO_02`, and § *Out of Scope* says so and names why it is not
here.

## Context

### The gap, and why it is this sequence's and not a library task

**Row #1** of `DEMO_APPLICATION.md` § *Library gaps* reads, in its own words:
*"**Map widget** — no map renderer exists or is planned. The demo's centerpiece."*
Severity **Critical**, and the Blocks column names one thing: *"Map/navigation
screen"*.

**§ *Task structure* decides who owns it, and the decision is explicit**: *"Gap
**#1**, the map widget, stays a `TASK_UI_DEMO_n` item — **the map is the demo's
own asset and no library owns it**."* **So there is no `ui_core` change in this
task at all**, and that is the single most load-bearing constraint in the file. A
reviewer looking for a `ui_core` diff will not find one, and its absence is
requirement 12's first acceptance criterion.

**Nothing in `ui_core` renders a map, and the sweep is reproducible.**
`rg -ci 'map_widget|MapView|tile|mercator|latitude|longitude|geojson' ui/src/ui_core/src`
returns **two files**, and both are false positives from `keyboard.rs`'s and
`list.rs`'s own vocabulary — the hits are the word *"tile"* inside comments about
keyboard key tiles and list rows. **Zero** identifiers named `MapView`,
`mercator`, `latitude`, `longitude` or `geojson` exist anywhere in `ui/src`.
**The claim this supports is "no map code exists", not "no word 'tile' exists"** —
the distinction is the one `DEMO_APPLICATION.md` § *Corrections to the second gap
table* insists on, where two rows were false *when written* because the grep was
never run against the tree it described.

### The dependency table: twelve specified library tasks, and which of them bind

**Tasks 41 to 52 are specified and none is built** — `doc/ui/IMPLEMENTATION_STATE.md`
§ *Task table* carries a row for each, every one reading *"specified 2026-10-05,
not started"*. **Not one of the twelve is a prerequisite of a map drawn from draw
commands**, and saying so is part of the deliverable: `DEMO_APPLICATION.md` §
*Task structure* records that closing the library gaps *"may be interleaved with
demo tasks when a gap blocks a demo screen"*, and for **this** screen it does not
block, because the map reaches `DrawCommand` directly.

| task | what it closes | binds this task? | what changes when it lands |
|---|---|---|---|
| **41** | `L1` colour capture + backdrop API | **No** | The map is the scene a backdrop is captured *from*. This task needs no `Backdrop`; it is the **premise** of `TASK_UI_DEMO_02`'s translucent chrome. |
| **42** | `#3` screen/navigation system | **No, but it is amended** | `Screens` replaces `Demo::pending_page`'s six writers with one. This task adds a seventh page, so every count in `TASK_UI_PRIM_42` § *Sub-task 42.2* becomes seven — requirement 10. |
| **43** | `#7` `Button::selected` + `TabBar` | **No, but it is amended** | The demo's hand-built bar becomes one `TabBar`. Same count amendment, § *Sub-task 43.2*, and three of its test names carry the count — requirement 10. |
| **44** | `#4` `Icon`, tintable `Image` | **No** | A POI glyph drawn as `Circle` + `Polygon` is exactly the thing `Icon` replaces. Naming that here is why this task does not wait: `Icon` is a *nicer* glyph, not a *possible* one. |
| **45** | `#5` per-node clip ownership | **No** | Today `Demo::frame_clips` returns `None` for every node and the window's own scissor is the only clip. When 45 lands the map's clip is owned by its node. **No behaviour of the map changes**: it projects *into* its node's rect, so clipping at that rect is correct under either mechanism. |
| **46** | `L5` horizontal scroll, momentum, snap | **No** | A pan needs it. `Camera` is a two-field struct and a pan is one write to it — see § *`Camera` is data, not an abstraction*. |
| **47** | `L6b` `ModeScope` | **No** | The map has one mode (north-up). |
| **48** | `L7` margin, shrink, cross-axis gap | **No** | The map's viewport is an absolute `Constraints::tight` box; the layers inside it are painted, not laid out. |
| **49** | `L8` measured width on `DrawCommand::Text` | **No** | A POI *label* needs it. This task draws POIs without labels — § *Out of Scope*. |
| **50** | `L9` theme scoping, `FocusRing` | **No** | `MapPalette` reads the flat global tokens, which is what `Theme` offers today. When a scope lands, a subtree-scoped map palette becomes possible and is not blocked by this task. |
| **51** | `L10` the `Polygon` convexity precondition | **No, and it is read** | This task records only convex polygons. **51 makes the precondition checkable**; until then `every_polygon_the_map_records_is_convex` is this task's own assertion and it is what makes the constraint true rather than aspirational. |
| **52** | `L3` `#2` `LayoutMode::Grid` | **No** | Nothing here is a grid. |

**Two more specified sequences this task sits behind, and both are stated rather
than assumed.** Tasks 34 to 40 — the depth buffer, the mesh vertex format, `Mat4`
and `u_model`, `DrawCommand::Mesh`, `ROADOSMF`, the offline asset pipeline and
drag-to-rotate — are *"specified 2026-10-05, not started"* in the same table, and
**task 36 is the one that matters here**: `TASK_UI_PRIM_36` § *Requirements* reserves
`Mat4` and `u_model` for **`DrawCommand::Mesh`**, the mesh path only. **There is
no `u_model` in any 2-D pass and none is coming from 36** — which is why
requirement 8 rotates the car marker in the demo rather than asking for a
transform.

### The map approach, and the two alternatives it beats

**The primitives are axis-aligned quads, a stroked polyline and a convex fan.**
`ui_core::paint::DrawCommand` carries **nine** variants today — `Rect`,
`RoundedRect`, `Shadow`, `Text`, `Image`, `Line`, `Circle`, `Path` and `Polygon` —
and a tenth, `Backdrop`, arrives with task 41. What each can do, read out of the
source and not inferred:

- **`fn command_quads` in `ui/src/ui_core/src/render.rs`** is the whole of the
  expansion, and its `DrawCommand::Path` arm is **one `line_quad` per segment with
  no join of any kind**. A polyline therefore *notches on the outside of every
  corner* — the same mechanism the gauge's author measured, at 5.6 px of
  scallop on a 14 px band. **The map's answer is one `Circle` per vertex**
  (requirement 6), because `DrawCommand::Circle`'s arm is a rounded rect of radius
  `r` around a `2r` box, so a disc of radius `w / 2` at a vertex covers precisely
  the region a round join fills. **A round join and a round cap are the same
  command**, which is why endpoints are included.
- **`fn line_quad`** is documented as *"the segment expanded by half its width
  along its normal"*, so a `Path`'s width is **pixels** and is honoured exactly
  per segment.
- **`DrawCommand::Polygon`'s doc** is explicit: *"**Convex only.** The renderer
  fans the points from the first of them into `n - 2` triangles, which is exact
  for a convex polygon and is not a polygon rasteriser"*, and `command_quads`'s
  own comment names both rejected alternatives. **Every filled shape this task
  records is convex by construction**, and `every_polygon_the_map_records_is_convex`
  is the test that says so.
- **There is no DPI concept.** `Renderer::window_size` in
  `ui/src/ui_core/src/render/context.rs` returns `self.window.size()`, which is
  `SDL_GetWindowSize` — **logical points, not `InPixels`**. So **one asset pixel
  is one window pixel** and there is no scale factor to multiply by.
- **There are no mipmaps.** `generate_mipmap` appears **zero** times in `ui/src`,
  and every texture in `render.rs` sets `TEXTURE_MIN_FILTER` and `TEXTURE_MAG_FILTER`
  to `GL_LINEAR` and nothing else. **A minified raster aliases**; drawn geometry
  does not, because there is no texture to minify.
- **Edges are resolved by the framebuffer, not by the primitives.**
  `DrawCommand::Polygon`'s doc records that the default framebuffer is **4x**
  multisampled since 2026-10-02, and that *"an edge landing on a pixel boundary
  resolves to full coverage on one side and none on the other and still reads
  hard"* — which is what a stroked road at a non-integer `y` will look like.

**Alternative 1 — a pre-rendered raster tile layer. Not reachable from `ui_demo`
today, and the reason is a missing public entry point.** The route would be
`Pixels` → `TextureCache` → `DrawCommand::Image`, and the chain breaks at the
first link:

- **`Renderer`'s only texture entry point is `pub fn load_texture(&mut self,
  path: &Path)`**, and its own body is `self.textures.load_from_file(path)` — it
  decodes **from disk**.
- **`Renderer::textures()` returns `&TextureCache`**, not `&mut TextureCache`.
- **`TextureCache::load`** — the closure-taking, decoder-free route the module's
  own doc calls *"the one caller that supplies the real decoder"* — needs
  **`&mut self`**. **There is no `textures_mut`, no `Renderer::make_texture`, no
  `upload_texture` and no `from_pixels`** anywhere in `ui/src/ui_core/src`, and
  **none of tasks 41 to 52 adds one**.
- **`ui/src/ui_demo/src/main.rs`'s `fn stand_in_picture` proves the shape is
  closed**: it builds its own `TextureCache`, calls
  `TextureCache::load(Path::new("stand-in"), ..)`, and **drops the cache on the way
  out**. The handle it returns therefore names a texture the *renderer's* cache has
  no pixels for, and `fn ensure_standalone_texture` in `render.rs` answers `None`
  from `self.textures.standalone_pixels(handle)`. The demo's own doc says the
  result *"draws nothing — an image of transparent pixels is an image of the
  background"*, and the mechanism is that the cache is gone. **So a procedurally
  generated raster cannot be drawn by the demo as the tree stands.**

**There is a second route to a raster — commit a PNG to
`ui/src/ui_demo/assets/` and call `Renderer::load_texture` — and it is refused for
three reasons.** First, it makes the map an **authored asset**, which silently
settles `DEMO_APPLICATION.md` § *Open questions* item 2 (*"What map data source to
use for the emulation? (procedural, hand-drawn, or simplified real data?)"*) by
omission, and that question is the operator's. Second, **the asset would have to
be produced by `TASK_UI_PRIM_39`**, the offline asset pipeline, which the state
table records as not started. Third, **it aliases**: with `GL_LINEAR` and no
mipmaps, a 512-pixel tile drawn at 200 pixels shimmers when the camera moves, and
this map has no camera movement to hide it behind.

**Alternative 2 — a mesh road ribbon through the GPU matrix path. Refused as
over-engineering, and it is unavailable anyway.** `DrawCommand::Mesh` is task 37's
variant and it does not exist; it needs tasks 34, 35, 36 and 37 in front of it,
**none built**. A ribbon is also the wrong shape: a map's roads are *strokes in
screen space*, which is exactly what a stroke primitive is for, and a triangle
strip per road is more geometry and more state than `Path` plus `Circle`.

**The decision, in one line: procedural vector geometry, generated in the demo
from a seeded integer generator, recorded as `Rect`, `Path`, `Circle` and
`Polygon`, at a fixed camera.** No texture, no mesh, no transform, no dependency.

### The visual result, concretely

North-up, one fixed camera, from the top of the window down. Drawn in this order,
and the order is the whole of the z-order because **every command is opaque**:

1. **Ground** — one `Rect` over the map node's own rect, in `palette.ground`.
2. **Blocks** — a 9 × 6 grid of inset rectangles in `palette.block`, one per grid
   cell, each culled against the viewport.
3. **Roads, minor first** — `Service` (2 px), then `Secondary` (4 px), then
   `Arterial` (6 px). Each road is one `Path` **plus one `Circle` per vertex**, so
   every corner is a round join and both ends are round caps.
4. **Route** — one `Path` at 8 px in `palette.route` (the theme's `Primary`), plus
   a `Circle` per vertex, drawn **after every road** so it lies on top of them all.
5. **Points of interest** — for each visible one, a filled `Circle` pip and one
   convex `Polygon` glyph inside it, in `palette.poi_fill` on `palette.ground`.
6. **The car** — a filled `Circle` body with a convex `Polygon` nose on top of it,
   the nose rotated to `car.heading_radians`.

**What that reads as:** a pale flat map with darker blocks, a grey road grid with
two heavier cross-town roads through it, one saturated blue route turning a corner,
three coloured POI pips and a blue marker with a white nose at the corner where
the route begins. **Light and dark both work**, because every colour is read from
the theme (requirement 7) and the dark theme's `Background` is `(18,18,18)` while
its `Surface` is `(30,30,30)` — a 12-point spread, which is why the ground is
built from `Background` and the blocks from `Surface` rather than from two invented
values.

### What this approach cannot do

**This section is the honest half, and it is here rather than in the handoff
because a reader deciding whether to fund a raster tile pipeline needs it before
the code, not after.**

- **No zoom, no pan, no rotation of the map.** The camera is fixed
  (`Camera { centre, pixels_per_metre }`) and there is no event that moves it,
  because there is nothing to move it with: task 46 (`L5`) is what a pan needs and
  it is not built, and a wheel event would be an input criterion this repository
  cannot verify — *pointer injection has never delivered an event to the window
  and keyboard injection delivered exactly one* (§ *Verifying a change that draws — the capture method*).
  **The fixed camera is why the map is capture-verifiable at all**: `--tab=demo`
  reaches it with no input at all.
- **A curve is a polyline.** There is no bezier anywhere in `ui/src` — zero hits
  for `fill_rule`, `nonzero` or `even_odd`, and `DrawCommand::Path`'s points are
  plain `(f32, f32)` pairs with no curve segment. A curved road is many short
  segments, and **at a zoom the faceting is visible**. At `CAMERA_PIXELS_PER_METRE`
  and the generator's minimum segment length it is not, and that is a statement
  about this camera, not about the representation.
- **A road's width is pixels, not metres.** `line_quad` expands by half its `width`
  in pixels and `project` scales metres to pixels, so a width is only correct at
  one camera scale. **A zoom would have to re-derive every width**, and the roads
  would change thickness as the map zooms — which is what a real map does, and
  what this one cannot do without that re-derivation being written.
- **A mitre join is not available.** `command_quads` gives a `Path` one quad per
  segment and no joins, and a round join covers *less* than a mitre at a sharp
  turn by at most `w / 2 · (1 / sin(θ / 2) − 1)`. **No corner in this world is
  sharp enough for that to show**, and `no_road_turns_by_less_than_the_join_can_cover`
  is the test that holds the claim rather than a reader trusting it.
- **Off-window geometry is submitted, not clipped.** `Demo::frame_clips` returns
  `None` for every node today, so the only clip is the window's own scissor.
  Culling (requirement 5) bounds the overhang rather than eliminating it: a road
  whose bounding box crosses the viewport is recorded whole, so up to one segment
  beyond each end is submitted and clipped on the GPU. **That is a cost, not a
  defect**, and it is why the cull test is on *intersection with the viewport* and
  not on *containment within it*.
- **No raster, so no texture memory and no mipmap problem — and also no
  satellite imagery, no labels, no turn restrictions, no lane geometry, no
  traffic.** Every one of those is a data problem, and the data problem is
  `DEMO_APPLICATION.md` § *Open questions* item 2, which this task does **not**
  answer (§ *The map source: a provisional decision, and what would reverse it*).
- **The car marker rotates by hand.** `DrawCommand` carries no transform field and
  none of its variants has one; task 36 § *Requirements* reserves `Mat4` and
  `u_model` for `DrawCommand::Mesh`. So requirement 8 does a two-term rotation in
  the demo. **It is four multiplies and two adds per point, for three points** —
  which is the whole argument for doing it in the demo: the alternative is a
  pipeline change for 12 arithmetic operations.
- **`Polygon` cannot draw a concave silhouette**, so the marker is a disc plus a
  triangle rather than an arrowhead with a notch, and a POI glyph is the simplest
  convex shape that reads. Task 51 § *Requirements* is what makes the convexity
  precondition *checkable*; until it lands this task's own test is the check.

### The map source: a provisional decision, and what would reverse it

**`DEMO_APPLICATION.md` § *Open questions* item 2 asks, verbatim: *"What map data
source to use for the emulation? (procedural, hand-drawn, or simplified real
data?)"*. This task takes a provisional decision and marks it as one, because
`developer.md` § *Phase 1* requires an acceptance test and a task file that
refuses to decide its data cannot have one.**

**The provisional decision: procedural, deterministic, seeded, and no geography
at all.** `pub fn world(seed: u64) -> World` generates blocks, roads, a route,
POIs and a car from a linear congruential generator written out in the module —
the house precedent is `every_segment_quad_is_convex_over_two_hundred_thousand_
geometries` in `ui/src/ui_core/src/widgets/chart.rs`, which writes its LCG inline
and sweeps. **No coordinate in the public surface is a latitude or a longitude**,
no file is read, no network is touched, and the whole world is a function of one
`u64`.

**Four reasons, and the third is the one that settles it.**

1. **It is the only one of the three that has no licence question.**
   `AGENTS.md` gates dependencies, and `developer.md` § *Dependencies* asks whether
   a crate is *"popular, maintained, and well-known in the Rust community"*. Real
   map data is a **data** licence, not a crate licence, and `DEMO_APPLICATION.md`
   § *Open questions* item 3 already carries an open trademark question about the
   same vendor's assets. **Shipping real coordinates would import that question
   into the map** and nobody has answered it.
2. **It is the only one that is testable without a display.** `AGENTS.md` forbids
   a test that needs a display, a network, a filesystem or the wall clock. A
   procedurally generated world is **a value**, and every property this task
   asserts — every route point on a road, no two POIs coincident, the heading
   equal to the first segment's bearing — is arithmetic over that value. Real data
   would make all of them file reads.
3. **It is the only one that keeps the map *the demo's own asset*.**
   `DEMO_APPLICATION.md` § *Task structure* decides *"the map is the demo's own
   asset and no library owns it"*, and a downloaded tile set is not the demo's own
   asset — it is a vendor's, with an attribution requirement.
4. **Hand-drawn is not a source, it is a workload**, and it is the one option that
   cannot be checked by a test: a hand-drawn road network's correctness is a
   judgement, and `reviewer.md` § Phase 2 asks whether a claim was checked.

**What would reverse it, stated so a later task does not have to re-derive it.**
`World` is a plain struct of five public `Vec`s and two public scalars, built by
one function. **Reversing the decision is a second call to the same constructor
with a different argument** — a `World` assembled from real data has the same
type, and everything above it (`Camera`, `project`, `MapPalette`,
`MapSurface::paint`, `Demo::frame`'s arm) is unchanged. **The seam is the data,
not an abstraction**, and `developer.md` § *Phase 2* is why there is no trait:
*"No abstraction before the second use. One caller is a function, not a trait."*
**Reversal becomes real when the second source exists, and not before.**

**And the operator's question stays open.** This task answers *"what does the demo
draw today"*, which is not *"what should the demo draw"*. § *Open questions* item 2
in `DEMO_APPLICATION.md` is **amended, not closed**: it gains this provisional
decision, its four reasons, and the statement that a real or hand-drawn source
would replace `world(seed)` with a second constructor and touch nothing above it.
**The decision is this task's; the question is still the operator's.**

### `Camera` is data, not an abstraction

**`pub struct Camera { pub centre: (f32, f32), pub pixels_per_metre: f32 }`** and
nothing else. It is a struct rather than a trait because a pan is **one write to
`centre`**, and because the honest statement of what this task has is *"a map at a
fixed camera"* rather than *"a camera that cannot move"*. Task 46 (`L5`) supplies
the gesture; the field is already there when it does.

### The seventh page, and the four places a page count lives

**`DEMO_APPLICATION.md` § *What a seventh page costs* is the mechanism, and
`ui/src/ui_demo/src/main.rs` is where it happens.** Every item below is read out
of the source, cited by symbol:

| what | where | what this task does |
|---|---|---|
| **`const ALL: [Page; 6]`** | `enum Page`'s `impl` block | becomes **`[Page; 7]`** — a fixed-size array, so the *type* carries the count and the compiler carries the change |
| **`fn name(self) -> &'static str`** | same `impl`, whose doc says *"The only place the six names are written out"* | gains **`Page::Demo => "demo"`**. `from_name` derives from `ALL` through `name`, and `page_names` and `usage` derive from `ALL`, so one spelling reaches `--tab=`, `--help` and the unknown-name message |
| **`Demo::tabs`** | `struct Tab { page, button }` | gains a seventh row **because it is built by `for &tab_page in &Page::ALL`** — the seventh tab button is not written anywhere |
| **the three gates** | `Demo::on_show`, `Demo::shows`, `Demo::focusables` | the map node gets a `PageMember` row, and every existing row keeps its page. **The complement assertion `tests::always_painted_handles` is what will catch a row that was forgotten** |
| **`Demo::new`'s page table** | the `on(Page, Handle, bool)` closure and its rows | one `on(Page::Demo, map_node, false)` row. **This table has been the site of two mutation-found majors** — `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided, and what it found* records the 24.1 finding (*"one dropped row gave 0 failed / 1814, with the text column drawn on the wrong page"*, and the lesson it drew is *a sweep of a mechanism's call sites is not a sweep of the data it is built from*), and the 24.2 row of the task table records the same class on `placed_handles` at *0 failed / 1817*. **A seventh row is exactly where it recurs** |
| **`Page::DEFAULT`** | the same `impl` | **stays `Page::Pads`**, and requirement 9 says so in a doc comment on the row. `DEMO_APPLICATION.md` § *What a seventh page costs* gives the reason: it is *"the one page every capture taken for tasks 11 to 22 contains, so a capture that used to need no argument is still reproducible"*, and **the demo tab must not become the default** |

**Two places the map node must NOT be added, and both are load-bearing.**

- **`Demo::placed_handles` must not gain a row.** That list is built by
  `tests::no_two_placed_rects_overlap` and by `tests::every_page_places_every_rect
  _where_the_gallery_placed_it`, and the map's rect is **full-bleed — it overlaps
  every gallery widget's rect by construction**. The list is of *leaves the demo
  places*, and the tab bar is already absent from it for a neighbouring reason.
  **The map is covered by the paint gate instead** —
  `a_page_records_no_command_on_a_node_that_is_not_its_own` — which is exactly
  what `Demo::page_rects`'s own doc says of the `overlays` page: *"The page is
  checked by the paint gate instead."*
- **`tests::expected_placed_rect_names` must not gain a row**, and
  `tests::assert_placed_handles_is_complete` must still assert its length
  unchanged. Both are named in `TASK_UI_PRIM_52` § *Acceptance Criteria* as rows
  *"not added or loosened"*, and the same rule holds here.

### What the tests already know, and the two counts that must not move

**The measured baseline, re-measured on the tree this file is written against, is
1894** — `cargo test --all-features` from `ui/` reads **1450** in `ui_core`
(`1 ignored`, which is `layout_walk_cost`), **224** in `ui_demo` and **220**
doctests. Every test this task adds is additive, and the number is quoted so that
"green" means a number rather than a feeling.

**Two existing counts are asserted by value and must not move.** `tests::
assert_placed_handles_is_complete` asserts that **five** rows of `Demo::
page_members` are `Tab` stops, and the map node is **not focusable** — a map is
not a control `Tab` can move focus to, on `PageMember::focusable`'s own argument
that *"a stop where nothing lights up is a stop a reader cannot see"*.
`tests::every_page_lists_at_least_one_node_and_no_node_is_on_two_pages` asserts
that **no handle is on two pages** and that **every row's handle is in
`Demo::order`**, so the map node must be appended to `order` — which the `Layout`
walk does automatically if it is attached to the root, and which requirement 8
makes explicit rather than incidental.

### A dependency this sequence does not have: `task-sequence.md` does not name it

**`.ai/workflows/task-sequence.md` § *Scope* lists the sequences the workflow
applies to — `doc/ui/TASK_UI_PRIM_*.md` and `doc/platform/TASK_CROSSPLATFORM_
*.md` — and `doc/ui/TASK_UI_DEMO_*.md` is not among them.** That is a real gap,
not a formality: § *State* says each sequence's `IMPLEMENTATION_STATE.md` records
its progress, and **this sequence would have nowhere to record anything**. So
requirement 11 amends that section and creates `doc/ui/IMPLEMENTATION_STATE_DEMO.md`.
**This is an `.ai/` edit, and `AGENTS.md` § *AI Engineering System* permits one
when the task *"explicitly concerns the AI engineering system — reviewing,
extending, or fixing it counts"*; this does, and no other `.ai/` file is touched.**

### Scope, measured against `developer.md` § *Scope check*

**Seven files and three components. The file count is over the threshold of five,
so this task is split into three sub-tasks**, per `developer.md` § *Scope check*
and `.ai/protocols/subagents.md` § *Implementation fan-out*.

| file | what changes |
|---|---|
| `ui/src/ui_demo/src/map.rs` | **new.** The world, `project`, `MapPalette`, `MapSurface`, `rotated`, `intersects`, and its `#[cfg(test)] mod tests` |
| `ui/src/ui_demo/src/main.rs` | `mod map;`, `Page::Demo`, `Page::ALL` at seven, `fn name`'s seventh arm, the `Demo::map` and `Demo::map_node` fields, `Demo::new`'s node and its one `page_members` row, one arm in `Demo::frame`'s paint walk, and the four new tests of § *Testing* |
| `doc/ui/DEMO_APPLICATION.md` | row #1 amended and **not closed**; § *Open questions* item 2 amended and **still open**; § *Open questions* item 7 answered; § *What a seventh page costs* gains a dated note |
| `doc/ui/done/TASK_UI_PRIM_42.md` | every count in § *Sub-task 42.2* that names six becomes seven |
| `doc/ui/TASK_UI_PRIM_43.md` | the same in § *Sub-task 43.2*, plus the three test names that carry the count |
| `doc/ui/IMPLEMENTATION_STATE_DEMO.md` | **new.** The task-table row, the record section, the waivers, the frame rate |
| `.ai/workflows/task-sequence.md` | § *Scope* gains `doc/ui/TASK_UI_DEMO_*.md` and names this sequence's state file |

| sub-task | files it owns | acceptance test it can pass alone |
|---|---|---|
| **01.A** — `map.rs` | `ui/src/ui_demo/src/map.rs` | `cargo test -p ui_demo --lib map::` is not a thing (this is a binary), so: `cargo test --all-features map::tests` passes, and `cargo build --all-targets` is green with `mod map;` declared and **no caller yet** — the module compiles and is tested standalone |
| **01.C** — the documents | the four `.md` files | `git diff --stat` shows the four, and every grep in requirement 11 and the amendments of requirement 10 is checkable by reading the diff. **It has no code and needs no build** |
| **01.B** — the wiring | `ui/src/ui_demo/src/main.rs` | the whole suite green, the seven-page capture, and the frame rate |

**01.A and 01.C are file-disjoint and have no data dependency, so they dispatch in
parallel; 01.B depends on `MapSurface` existing, so it is sequential after 01.A**
— `.ai/protocols/subagents.md` § *Parallel or sequential*, and its rule *"Do not
dispatch a subagent whose brief depends on code that does not exist yet"*. **01.B
is one field, one node creation, one `page_members` row and one arm in a paint
walk; splitting it further would be a split along no boundary at all, and the
protocol says *"If a sub-task is too large for one subagent, the developer split
wrongly."***

**Three components is at the threshold and not over it** — `developer.md` says
*"more than 3 independent components"* — so **the split is a consequence of the
file count alone**, and that is stated rather than dressed up as a design.

## Requirements

1. **`ui/src/ui_demo/src/map.rs`, new, and `mod map;` beside `mod fps;`** in
   `ui/src/ui_demo/src/main.rs`. The module doc carries, in this order: **what
   the map is**; **that it is procedural, seeded and contains no geography**;
   **the six things it records and the order it records them in**; **that every
   filled shape it records is convex and why**; and **§ *What this approach
   cannot do* in full, in the module's own voice rather than as a pointer to a
   task file** — because the module is the file a future agent opens first, and
   a confident claim in exactly that position is what this guards against.

2. **The world's types, named and complete, in `map.rs`.** Every one is `pub`,
   every field is `pub`, and `World`, `Road`, `Poi` and `Car` derive
   `Clone, Debug, PartialEq` — `PartialEq` because `world_is_deterministic_in_its
   _seed` compares two worlds and cannot without it.

   ```rust
   /// The three road widths, and the order they are drawn in.
   ///
   /// **Minor first**, so a heavier road lies on top of a lighter one where they
   /// cross, and the draw order is a list rather than a comparison: three `match`
   /// arms would be a second answer to "which class is drawn first" that a new
   /// class could disagree with.
   pub enum RoadClass {
       Service,
       Secondary,
       Arterial,
   }

   pub const ROAD_WIDTH_SERVICE: f32 = 2.0;
   pub const ROAD_WIDTH_SECONDARY: f32 = 4.0;
   pub const ROAD_WIDTH_ARTERIAL: f32 = 6.0;
   pub const ROUTE_WIDTH: f32 = 8.0;

   /// One road, in map metres, as the polyline its centreline follows.
   pub struct Road {
       pub class: RoadClass,
       pub points: Vec<(f32, f32)>,
   }

   /// What a point of interest is. Three, and no more: three is enough to prove
   /// the glyph dispatch and few enough that every one is in the first capture.
   pub enum PoiKind { Charge, Food, Fuel }

   pub struct Poi {
       pub at: (f32, f32),
       pub kind: PoiKind,
   }

   /// One city block, as its centre and its extent in map metres.
   pub struct Block {
       pub at: (f32, f32),
       pub size: (f32, f32),
   }

   /// The car: where it is and which way it points, in radians, north-up.
   pub struct Car {
       pub at: (f32, f32),
       pub heading_radians: f32,
   }

   pub struct World {
       pub blocks: Vec<Block>,
       pub roads: Vec<Road>,
       pub route: Vec<(f32, f32)>,
       pub pois: Vec<Poi>,
       pub car: Car,
   }

   /// How the world is turned into window pixels.
   pub struct Camera {
       pub centre: (f32, f32),
       pub pixels_per_metre: f32,
   }
   ```

3. **Two constants and a camera, and the arithmetic in requirement 4 depends on
   both.** `pub const MAP_METRES: f32 = 1600.0` is the width **and** the height of
   the world in map metres, so the world is square and the camera's aspect
   handling is the map node's aspect — which is 1280 × 956, so **a square world in
   a non-square viewport is stretched**, and that is stated in `Camera`'s doc
   rather than corrected, because a fitted scale would need a `min` of two numbers
   and a decision about which axis gives. `pub const CAMERA_PIXELS_PER_METRE:
   f32 = 0.7` puts 1600 m across 1120 px, inside the 1280-wide node. **Both are
   named, and neither is written into an expression anywhere else in the module.**

4. **`pub fn world(seed: u64) -> World` — procedural, deterministic, and total.**
   In order:

   - **The generator is an LCG written out in the module**, not a dependency and
     not `std`'s — `state = state · 6364136223846793005 + 1442695040888963407`,
     and a draw is `(state >> 33) as f32 / (1u64 << 31) as f32`. **`state >> 33`
     is 31 bits**, so the quotient is in `0.0..=1.0` by construction and needs no
     clamp; and the two `as` casts are the file's **only** two, with a doc comment
     on the function naming them and the arithmetic that makes them exact —
     `developer.md` § *Code quality* forbids `as` casts for numeric conversions,
     and `ui/src/ui_core/src/layout.rs`'s `fn count_to_f32` is the crate's answer
     for `usize`, which does not cover a `u64` shift.
   - **Blocks**: a **9 × 6 grid** over `MAP_METRES`, each cell inset by a quarter
     of its pitch on every side, so the road grid shows between them. **54
     blocks**, and the count is in `blocks_are_one_per_cell_of_a_nine_by_six_grid`.
   - **Roads**: **two arterial and three secondary lines on each axis**, snapped
     to grid-line coordinates, plus **ten service roads**, each a two-segment L
     hanging off a secondary. **20 roads in all**, and
     `the_world_has_twenty_roads_in_three_classes` asserts the per-class counts
     (10 / 6 / 4) rather than the total alone.
   - **The route**: **a three-point L** from one grid intersection to another two
     cells away, and **every point of it lies on a road's polyline** —
     `every_route_point_lies_on_some_road_polyline` is the property test and the
     one a reviewer should break first.
   - **The car**: at the route's first point, with
     `heading_radians` **the bearing of the route's first segment**, so the marker
     points the way the route goes.
   - **POIs**: **three, one per `PoiKind`**, at grid intersections that are
     neither the car's nor each other's.
   - **`seed` is not special-cased.** `world(0)` and `world(1)` both return a
     full world and neither panics — `world_zero_is_not_special` says so, because a
     generator that guards `0` and a generator that divides by a seed are the same
     class of bug.

   **No file is read, no clock is read, no environment is read, no thread is
   spawned, and there is no `unsafe`** — which is what makes every test in §
   *Testing* legal under `AGENTS.md` § *Rust*.

5. **`#[must_use] pub fn project(camera: &Camera, point: (f32, f32), viewport:
   Rect) -> (f32, f32)` — two multiplications and four additions, and no divisor
   anywhere.** The body is the closed form

   ```
   x = viewport.origin.x + viewport.size.width / 2.0 + (point.0 - camera.centre.0) * camera.pixels_per_metre
   y = viewport.origin.y + viewport.size.height / 2.0 + (point.1 - camera.centre.1) * camera.pixels_per_metre
   ```

   **Three decisions are in that, and each is argued because each has a defensible
   alternative.**

   - **The camera's centre is the middle of the viewport, not its origin.** An
     origin-pinned camera is the simpler function and it puts the world's centre in
     the top-left corner, which puts a third of the map off-screen on the first
     capture for no reason a reader could see.
   - **`y` is not negated.** World-south is down on screen and world-north is up,
     so the identity is correct and the temptation is to write a Mercator-style
     flip because that is what a map API looks like. **The doc says why there is
     no flip**, and `project_scales_metres_into_pixels_and_does_not_flip_y` is the
     test: projecting `(0, 0)` and `(0, +100)` must put the second **below** the
     first.
   - **There is no divisor, and that is the reason a zero-sized viewport is safe.**
     `viewport.size.width / 2.0` divides by a literal; nothing divides by a value
     the caller supplied. So a viewport of `0 × 0` projects every point to the
     origin, records no division and cannot panic —
     `paint_records_a_ground_rect_and_nothing_else_at_a_zero_sized_viewport` is
     the test, and **it is the evidence that the absence of a divisor is real
     rather than asserted**. A `NaN` handed in propagates to a `NaN` out, which
     `a_nan_point_projects_to_a_nan_rather_than_to_a_panic` records.

6. **Round joins and round caps are one `Circle` per vertex, and the arithmetic
   that makes it correct is in the doc.** For a stroke of width `w`:

   - `DrawCommand::Circle`'s arm in `fn command_quads` is a rounded rect of radius
     `r` around a `2r × 2r` box, so `Circle { center: v, radius: w / 2.0 }` is a
     disc of radius `w / 2` centred on `v`;
   - `fn line_quad` is *"the segment expanded by half its width along its normal"*,
     so each segment covers exactly the band within `w / 2` of itself;
   - **the union of those bands leaves a wedge at every interior vertex, and a disc
     of radius `w / 2` at that vertex is the round join that fills it** — a round
     join is *by definition* the disc of radius `w / 2` at the vertex;
   - **the same disc at an endpoint is a round cap**, so endpoints are included and
     there is no second rule.

   `MapSurface::paint` therefore records, per stroke, **one `Path` and one `Circle`
   per point including both ends** — never one fewer. The alternative (relying on
   the segments alone) is the notch measured on a gauge, and it is a visible
   defect on a road grid.

   **The limit, in the same doc: a mitre join covers more.** At a turn of `θ` a
   mitre extends `w / 2 · cot(θ / 2)` from the vertex against the disc's `w / 2`,
   so the shortfall is `w / 2 · (1 / sin(θ / 2) − 1)` — **and
   `no_road_turns_by_less_than_the_join_can_cover` is a test over every vertex of
   every road that holds the shortfall under one pixel at the widest road**, so the
   claim is arithmetic rather than a promise.

7. **`pub struct MapPalette` with ten fields, and `pub fn map_palette(&Theme) ->
   MapPalette`, and the six values that have no token behind them are named.**
   The ten fields, in declaration order, with the token each reads or the literal
   it falls back to:

   | field | token | note |
   |---|---|---|
   | `ground` | `Background` | the whole map's base |
   | `block` | `Surface` | dark `(30,30,30)` on ground `(18,18,18)`; light `(245,245,245)` on `(255,255,255)` |
   | `water` | `Surface` | **unused in the first capture** and kept because the block layer reserves a channel for it; the doc says so rather than leaving a field no reader can account for |
   | `road_service` | `Border` | dark `(51,51,51)`, light `(224,224,224)` |
   | `road_secondary` | `TextMuted` | |
   | `road_arterial` | `TextMuted` | **the same token as `road_secondary`, and a distinct fallback literal** — the *width* carries the class, not the colour, which is what a real map does |
   | `route` | `Primary` | dark `(187,134,252)`, light `(98,0,238)` |
   | `car_body` | `Primary` | the disc |
   | `car_nose` | `OnPrimary` | the nose, so it is legible against the disc in both themes |
   | `poi_fill` | `Warning` | one colour for three kinds in the first capture |

   **`map_palette` reads `theme.get(token).as_color().unwrap_or(fallback)`**, the
   `fn themed_color` precedent in `main.rs`, so a theme that ever holds a number
   where a colour belongs yields the literal rather than a panic.
   `map_palette_reads_every_value_from_a_theme_and_every_colour_is_opaque` asserts
   the ten `a` values are all `255`, and `map_palette_falls_back_when_a_token_holds
   _a_number` covers the fallback.

   **Why all ten are opaque, in the doc:** `Color` is documented in
   `ui/src/ui_core/src/property.rs` as *"A minimal RGBA color with **premultiplied
   alpha**"*, so a translucent colour is a premultiplication the caller performs —
   `(c · a) / 255` in `u16`, `Pixels::premultiply`'s arithmetic — and the map
   needs none. **Ten opaque colours also mean every command is opaque, which means
   the record order *is* the z-order**, and that is requirement 9's whole argument.

8. **`#[must_use] pub fn rotated(marker: &[(f32, f32)], heading_radians: f32,
   about: (f32, f32)) -> Vec<(f32, f32)>` — the marker's rotation, done in the
   demo.** `(x, y)` becomes `(c + dx · cos θ − dy · sin θ, c + dy · cos θ + dx ·
   sin θ)` where `dx = x − about.0` and `dy = y − about.1`. **It takes the
   marker as a slice and returns a `Vec`**, so the same function rotates the nose
   and any glyph; and **it returns rather than mutating** so a caller cannot
   rotate the constant it drew from.

   **The reason it exists here and not as a uniform, in the doc:** `DrawCommand`
   carries no transform field and none of its nine variants has one;
   `TASK_UI_PRIM_36` § *Requirements* gives `Transform::to_matrix` and the `u_model`
   uniform to **the mesh path**, because *"the mesh **program**"* is where a
   uniform can be uploaded. **So the cost of asking the pipeline for a transform is
   four tasks, and the cost of not asking is twelve arithmetic operations on three
   points.** Rotation also preserves convexity, which is why rotating the nose
   keeps it a legal `Polygon` — `rotating_a_marker_preserves_convexity` asserts it
   over a sweep of headings rather than leaving it to a reader.

9. **`MapSurface`, its `paint`, and the command order — the load-bearing
   requirement.**

   ```rust
   pub struct MapSurface {
       world: World,
       camera: Camera,
       palette: MapPalette,
   }

   impl MapSurface {
       #[must_use]
       pub fn new(world: World, theme: &Theme) -> Self
       /// The surface's whole picture, for `viewport` in window coordinates.
       #[must_use]
       pub fn paint(&self, viewport: Rect) -> Vec<DrawCommand>
   }
   ```

   **`paint` records, in this order and no other:**

   1. **`Rect`** over `viewport` in `palette.ground` — **exactly one, always, even
      at a zero-sized viewport and even for an empty `World`**. That is what makes
      `a_page_records_no_command_on_a_node_that_is_not_its_own` meaningful on the
      demo page: the map is never the page that draws nothing.
   2. **`Rect`** per visible `Block`, in `World::blocks` order, in `palette.block`.
   3. **`Path` + one `Circle` per point** per visible `Road`, **grouped by class in
      `Service`, `Secondary`, `Arterial` order** and within a class in
      `World::roads` order — requirement 6's rule and the two orders.
   4. **`Path` + one `Circle` per point** for `World::route` at `ROUTE_WIDTH`, in
      `palette.route`. **The route is never culled**: it is three points, and a
      route that vanished at the edge of the viewport would be a bug in a feature
      nobody asked for.
   5. **`Circle` then `Polygon`** per visible `Poi`, in `World::pois` order — the
      pip, then the glyph.
   6. **`Circle` then `Polygon`** for the car — the body, then the nose, the nose
      from `rotated(NOSE, car.heading_radians, car.at)`.

   **The order is the z-order and it is a list, because every command is opaque.**
   `ui/src/ui_core/src/batch.rs` groups commands into batches and
   `ui/src/ui_core/src/render.rs` submits them in the order they were recorded, so
   the last recorded opaque command is the top one. **The doc cites that rather
   than restating it as a rule**: that is the same mechanism, and the reason the
   ordering here is checkable is that
   `roads_are_recorded_minor_class_first` and `the_car_is_recorded_last` assert the
   *positions* of commands rather than their presence.

   **The glyphs are named constants, and each is convex by construction:**
   `MARKER_NOSE: [(f32, f32); 3]` — a triangle whose tip is `MARKER_NOSE_LENGTH`
   beyond the disc and whose base sits inside it, so the nose reads against the
   disc in both themes; and a private `fn poi_glyph(kind: PoiKind) ->
   &'static [(f32, f32)]` returning a three- or four-point convex polygon per kind
   — a triangle for `Food`, a quad for `Fuel`, a narrow quad for `Charge`. **A
   `match` on the kind, not an index into a table**, on the same
   `GALLERY_SHORTCUTS` argument `main.rs` makes about a list beside a `match`.

10. **`ui/src/ui_demo/src/main.rs`: `Page::Demo`, the seventh page, and the map
    node.** In order, and each step names what it is:

    - **`Page::Demo`**, a variant with a doc comment saying what the page holds
      **today** — the map, and nothing else — and pointing at `TASK_UI_DEMO_02`
      for the chrome.
    - **`const ALL: [Page; 7]`**, `Page::Demo` **last**, after `Page::Overlays`.
      The order is load-bearing three times (`--help`, the unknown-name message,
      and the tab bar's button order) and `Page::Demo`'s doc says so.
    - **`fn name`'s seventh arm, `Page::Demo => "demo"`.** It is the **only** new
      spelling, and `Page::name`'s doc keeps its claim that it is *"The only place
      the six names are written out"* with **"six" changed to "seven"** and a dated
      note saying which task changed it.
    - **`const DEFAULT` stays `Page::Pads`** and its doc gains one sentence: *"The
      seventh page does not become the default. `Page::DEFAULT` is the page every
      capture taken for tasks 11 to 22 contains, so a capture that used to need no
      argument is still reproducible — `DEMO_APPLICATION.md` § *What a seventh page
      costs*."* **This is the criterion, and it is checkable by reading one line.**
    - **Two fields on `Demo`**: `map: map::MapSurface` and
      `map_node: Handle`, each with a doc comment saying why it is a field and not
      a local.
    - **`Demo::new`**: `mod map`'s `MapSurface::new(map::world(MAP_SEED), &theme)`
      where `pub const MAP_SEED: u64 = 1_812_045;` — a named constant, because a
      bare literal in a constructor is a seed nobody can reproduce from the
      capture. The node is created with `node::create`, its
      `LayoutMode::Absolute` root puts it at `Offset::new(0.0, CONTENT_TOP)` — the
      same reason the tab bar writes its own position out — and
      `Constraints::tight(Size::new(WINDOW.width, WINDOW.height - CONTENT_TOP))`.
      **The map is full-bleed below the tab bar**, which is what makes
      `TASK_UI_DEMO_02`'s chrome translucent over a live scene rather than over a
      flat colour.
    - **One `page_members` row**: `on(Page::Demo, map_node, false)`, with a comment
      naming `IMPLEMENTATION_STATE`'s two mutation-found majors as the reason the
      row is written where a reader can see it.
    - **One arm in `Demo::frame`'s paint walk**, **immediately after the background
      arm and before the container arm**, painting from the node's own laid-out
      rect: `*node.paint_mut() = PaintState::from_commands(self.map.paint(rect));`
      — `PaintState::from_commands` marks the state dirty, which is what
      `Renderer::draw_node` reads. **The arm is placed where it is because the walk
      is parent-first and `order` is the z-order**, so a map painted after the
      gallery's own background is above it and below everything the gallery paints
      in the same page — and on the six gallery pages it is **not painted at all**,
      because `Demo::on_show` hides a node in no page's list.

11. **The documents, four files, each dated and attributed and none closed.**
    - **`DEMO_APPLICATION.md` row #1** gains a dated note naming this task: the map
      **exists**, in `ui/src/ui_demo/src/map.rs`, drawn from `Rect`, `Path`,
      `Circle` and `Polygon`; **the row is not deleted and not marked closed**,
      because § *Library gaps* records that its rows *"keep their numbering,
      because four files outside this one cite it by number"*, and its Blocks column
      keeps *"Map/navigation screen"* because **no navigation screen is built**.
    - **`DEMO_APPLICATION.md` § *Open questions* item 2** gains the provisional
      decision of § *The map source*, its four reasons, and the reversal path — and
      **the item stays on the list**, marked as answered-provisionally rather than
      removed. The section's existing precedent is the *"Light or dark first?"*
      item, which was **answered and removed**, and this one is **not** answered,
      and the difference is stated so a reader does not read the presence of an
      answer as its removal.
    - **`DEMO_APPLICATION.md` § *Open questions* item 7** is **answered and
      removed** — *"What is the seventh page called, and its `--tab=` value?"* — with
      the answer `demo`, the date, and the sentence that `Page::name` remains the
      single place the spelling lives.
    - **`DEMO_APPLICATION.md` § *What a seventh page costs** gains a dated note
      recording that all six of its items are now done, that `Page::DEFAULT` did
      not move, and that the **`placed_handles` row is deliberately absent** — which
      is the row § *Two places the map node must NOT be added* is about.
    - **`.ai/workflows/task-sequence.md` § *Scope** gains
      `doc/ui/TASK_UI_DEMO_*.md` to the list of sequences the workflow applies to,
      and names `doc/ui/IMPLEMENTATION_STATE_DEMO.md` as this sequence's state
      file. **No other `.ai/` file changes.**
    - **`doc/ui/IMPLEMENTATION_STATE_DEMO.md` is created** with the task-table row
      for **01** (file, review count, waivers-or-none, frame rate), a § *Task
      DEMO 01* record carrying **the map approach and its honest limits in the
      record's own register**, **the provisional map-source decision and the four
      reasons**, **the test count before and after**, **the frame rate for all
      seven pages**, and **what the handoff does not claim** — no pan, no zoom, no
      raster, no geography, no POI labels.
    - **`TASK_UI_PRIM_42` § *Sub-task 42.2** and **`TASK_UI_PRIM_43` §
      *Sub-task 43.2***: **every count that names the demo's six pages becomes
      seven**, dated and attributed, and **three of 43.2's test names that carry
      the count in the name are renamed to match** —
      `tab_walks_the_six_buttons_before_the_pages_own_controls`,
      `the_tab_bar_is_still_first_in_the_route_chain_for_each_of_its_six_buttons`
      and `the_demo_bar_has_one_tab_per_page_and_no_tab_beyond_the_six`.
      **The handoff pastes `git grep -n 'six' doc/ui/TASK_UI_PRIM_4[23].md`**, so
      every line the amendment touched is listed. **No other part of either task
      file changes**, and in particular 42's and 43's `## Requirements` for
      `ui_core` are untouched, because **nothing in `ui_core` changed**.

12. **`ui/Cargo.toml` and `ui/Cargo.lock` are unchanged, and the manifest rule is
    the reason.** The approved direct dependencies are `sdl3 0.20`, `glow 0.18`
    and `freetype-rs 0.38`, and **a map drawn from four draw commands needs none
    of them and no fourth**. Edition 2021 and `rust-version = "1.85"` hold:
    `usize::div_ceil` is stable since 1.73 and is not used here, `f32::mul_add` is
    not used, and `let-else` (stable 1.65) is not needed because `project` has no
    branch.

## Testing

**Every test is named, every test is legal under `AGENTS.md` § *Rust*, and every
one asserts a contract rather than an implementation detail** — `reviewer.md` §
Phase 1 asks that question and `developer.md` § Phase 1 answers it here: **every
test drives `world`, `project`, `map_palette`, `rotated` or `MapSurface::paint`
through their public signatures, and no test names a private helper.** The one
private function is `fn intersects`, and its behaviour is asserted through
`MapSurface::paint` by the two cull tests rather than directly.

**In `map.rs`'s `#[cfg(test)] mod tests` — twenty-six tests:**

| # | test | what it holds |
|---|---|---|
| 1 | `project_puts_the_camera_centre_at_the_middle_of_the_viewport` | the closed form, at a non-origin viewport — **a geometry fixture at the origin cannot see an origin being read as an extent** |
| 2 | `project_scales_metres_into_pixels_and_does_not_flip_y` | `(0, 0)` and `(0, +100)` land with the second **below** the first |
| 3 | `a_nan_point_projects_to_a_nan_rather_than_to_a_panic` | propagation, not a panic |
| 4 | `world_is_deterministic_in_its_seed` | `world(7) == world(7)`, by `PartialEq` |
| 5 | `a_different_seed_makes_a_different_world` | the generator is not constant — `world(1) != world(2)` |
| 6 | `world_zero_is_not_special` | `world(0)` returns a full world and does not panic |
| 7 | `blocks_are_one_per_cell_of_a_nine_by_six_grid` | 54, on the grid |
| 8 | `the_world_has_twenty_roads_in_three_classes` | **10 service, 6 secondary, 4 arterial**, asserted per class rather than as a total |
| 9 | `every_road_has_at_least_two_points_and_every_point_is_inside_the_map_bounds` | the `Path` precondition, since a one-point road draws nothing |
| 10 | **`every_route_point_lies_on_some_road_polyline`** | **the property test, the one to break first.** For every route point, the perpendicular distance to some road's polyline is under `1e-3` m. It is what makes the route a route rather than a line drawn near the roads |
| 11 | `the_route_starts_at_the_car_and_ends_at_a_poi` | the two endpoints |
| 12 | `the_car_heading_is_the_bearing_of_the_first_route_segment` | the marker's rotation is derived, not invented |
| 13 | `no_two_pois_share_a_position_and_one_is_of_each_kind` | three POIs, three kinds |
| 14 | `map_palette_reads_every_value_from_a_theme_and_every_colour_is_opaque` | ten values, ten alphas of 255 — the opacity that makes the record order the z-order |
| 15 | `map_palette_falls_back_when_a_token_holds_a_number` | the `as_color().unwrap_or(..)` path |
| 16 | `paint_records_one_ground_rect_and_nothing_else_when_the_world_is_empty` | **the vacuity guard.** Check that the case is where the property is non-vacuous: an empty world must record **exactly one** command, not zero, or every other test's "at least one command" would pass on an empty picture |
| 17 | `paint_records_a_path_and_one_circle_per_vertex_for_every_road` | **the count through the public API** — for each road, one `Path` whose point count equals the road's, and one `Circle` per point **including both ends**. **The control beside it is test 18**: the same call with an empty `World`, where there is no road to count |
| 18 | `a_world_with_no_roads_records_no_path_and_no_circle` | the control for 17 |
| 19 | `roads_are_recorded_minor_class_first` | **the positions**, not the presence: every `Service` road's `Path` precedes every `Secondary` one's, which precedes every `Arterial` one's |
| 20 | `the_route_is_recorded_after_every_road` | its `Path`'s index in the returned `Vec` is above every road's |
| 21 | `the_car_is_recorded_last_and_is_a_circle_then_a_polygon` | the last two commands, in that order |
| 22 | `every_polygon_the_map_records_is_convex` | **the signed-area sign test over all three consecutive triples**, over every `Polygon` in the returned commands and for the three POI kinds and three headings. **This is task 51's precondition asserted by the consumer until 51 lands** |
| 23 | `a_block_outside_the_viewport_records_no_rect` and `a_road_outside_the_viewport_records_nothing` and `a_poi_outside_the_viewport_records_nothing` — **three tests, and each keeps the ground `Rect` asserted** | the cull, and each is paired with its **positive control** in the same test: a camera moved so the item is inside records it. **A cull test without the control cannot tell "culled" from "absent"** |
| 24 | `paint_records_a_ground_rect_and_nothing_else_at_a_zero_sized_viewport` | **the evidence that `project` has no divisor** |
| 25 | `rotating_a_marker_preserves_convexity` | convexity is rotation-invariant — asserted over a sweep of headings in `0.0..=2π` |
| 26 | `paint_is_a_pure_function_of_the_world_and_the_viewport` | two calls, equal `Vec`s — **the assertion that catches a map that drifts because it accumulates state** |

*(The numbering counts the three cull tests of row 23 as three, so the list is
twenty-six `#[test]` functions: 1–22, then 23's three, then 24–26.)*

**In `ui/src/ui_demo/src/main.rs`'s `mod tests` — six new tests, named:**

- **`the_default_page_is_still_pads_and_the_demo_tab_is_the_seventh`** —
  `Page::DEFAULT == Page::Pads`, `Page::ALL.len() == 7`,
  `Page::ALL[6] == Page::Demo`, and `Page::Demo.name() == "demo"`.
- **`the_seventh_tab_button_lays_out_inside_the_window`** — the bar is laid out
  and the last button's `far_corner().x <= WINDOW.width`. **"Seven buttons fit" is
  a number, not an estimate**, and this is the number.
- **`the_demo_page_draws_the_map_and_the_gallery_pages_do_not`** — the paint gate
  over the map node on all seven pages: **more than one command on `Page::Demo`,
  and empty on each of the six**. This is the criterion that the map does not leak.
- **`the_map_node_is_not_in_placed_handles_and_the_two_counts_did_not_move`** —
  `placed_handles().len()` and `expected_placed_rect_names().len()` are both
  **unchanged from their pre-task values**, and the map's handle is in neither.
  **A count is the checkable form of "the map was not added to a list it overlaps"**,
  and it is the mutation a reviewer should make: add the row, watch this fail.
- **`the_map_node_is_on_no_page_but_the_demo_page_and_is_not_focusable`** — the
  existing invariant restated for the new row: no handle on two pages, every row's
  handle in `Demo::order`, and the `focusable` count **still five**.
- **`a_theme_switch_reaches_the_map`** — `toggle_theme()` and then
  `map_palette(&theme)` differs in at least `ground`, proving the palette is read
  per-paint rather than captured once.

**These six are additive.** `main.rs`'s 224 tests and `map.rs`'s 26 arrive from a
baseline of **1894 = 1450 + 224 + 220**, each pasted in the handoff, and **no
existing test is deleted, renamed or weakened** — with **one exception named in
advance**: the three task-43 test names listed in requirement 11 are renamed **in
`TASK_UI_PRIM_43.md`**, which is a task file, **not in any binary**, so no test in
`ui/src` is renamed by this task.

**Three deliberate breaks the handoff must paste**, because
`developer.md` § Phase 3 says *"A test that has never failed is not a test"*:

1. **Delete the per-vertex `Circle`s** from `MapSurface::paint` and re-run test 17
   — it fails on the count, and the capture shows the notches. **This is the break
   that has a visual consequence**, and the rule that *a fill rule cannot be
   repaired downstream* says the consequence is the evidence.
2. **Move `road_arterial` before `road_service`** in the record order and re-run
   test 19 — it fails, and the capture shows the thin roads on top of the heavy
   ones where they cross.
3. **Replace `palette.ground` with `palette.block`** and re-run test 16 — the
   count does not change, which is **the point**: the test cannot see it and the
   capture can. **A break that survives a test is not a weak test here, it is a
   test on the wrong layer**, and the handoff says which is which rather than
   reporting a survivor as a pass.

## Acceptance Criteria

- [ ] **`ui_core` has no diff and the map sweep is still empty.**
      `git diff --stat` names **no** file under `ui/src/ui_core/src/`, and
      `rg -ci 'map_widget|MapView|mercator|latitude|longitude|geojson'
      ui/src/ui_core/src` returns **no file at all**, while
      `rg -n 'pub fn paint' ui/src/ui_demo/src/map.rs` returns `MapSurface::paint`
      and `rg -c 'mod map;' ui/src/ui_demo/src/main.rs` is non-zero. **The
      mechanism is the point**: `DEMO_APPLICATION.md` § *Task structure* decided
      *"the map is the demo's own asset and no library owns it"*, and a criterion
      that only asserted "a map module exists" would pass on a diff that closed
      gap #1 in the crate instead.

- [ ] **`Page::ALL` is `[Page; 7]`, `demo` is the seventh spelling, and the default
      did not move.** `rg -n 'const ALL: \[Page; ' ui/src/ui_demo/src/main.rs`
      reads **`const ALL: [Page; 7]`**; `rg -n 'Page::Demo => "demo"'` finds
      **exactly one** occurrence, inside `fn name`;
      `./target/release/ui_demo --tab=demo` opens the demo page and
      `./target/release/ui_demo --tab=dta` refuses and names **seven** pages;
      `./target/release/ui_demo --help` prints seven; and
      `./target/release/ui_demo` with no argument opens **`pads`** — checked by
      the capture in the next-but-two criterion, not by the argument parser alone.
      **`the_default_page_is_still_pads_and_the_demo_tab_is_the_seventh` asserts
      all four.** `Page::DEFAULT`'s doc carries the sentence from
      `DEMO_APPLICATION.md` § *What a seventh page costs*, and **one line of
      `main.rs` is the whole of the "the demo tab must not become the default"
      criterion.**

- [ ] **`map.rs` is a new file with a module doc that carries the limits, not a
      pointer to them.** `ls ui/src/ui_demo/src/map.rs` exists;
      `rg -n '^mod ' ui/src/ui_demo/src/main.rs` shows `mod fps;` and `mod map;`;
      and the module doc contains, in its own words and not as a link, **each of:
      procedural and seeded; no geography; every filled shape convex; no camera
      movement; a curve is a polyline; a road's width is pixels; no mitre join; no
      raster and therefore no mipmaps; the marker is rotated by hand because no
      2-D pass carries a transform.** **A module doc that says "see the task file"
      fails this criterion**: *"a confident false claim in the one file every
      future reader opens is worse than no claim"*, and a pointer is the same
      failure with better manners.

- [ ] **The world is deterministic, total, and every route point is on a road.**
      `every_route_point_lies_on_some_road_polyline` is present and passes; the
      other five world tests in the § *Testing* table are present by name;
      `world(7) == world(7)` and `world(1) != world(2)` and `world(0)` does not
      panic. **Mutation evidence in the handoff:** make the route's second point a
      free coordinate rather than a grid intersection and watch test 10 fail;
      make the LCG's draw constant and watch test 5 fail. **Test 10 is the one a
      reviewer should break first** — every other world test passes on a world
      whose route is a random diagonal, and only test 10 notices that the route no
      longer follows a road.

- [ ] **The round-join rule is a count through the public API, with its control.**
      `paint_records_a_path_and_one_circle_per_vertex_for_every_road` asserts, for
      each road, **one `Path` whose `points.len()` equals the road's, and exactly
      one `Circle` per point — including both endpoints** — and
      `a_world_with_no_roads_records_no_path_and_no_circle` is the control that
      says zero roads records zero of each. **Mutation evidence:** drop the
      endpoint `Circle`s and watch the count fall by two per road; drop them all
      and watch the capture show the notches, which is the
      mechanism with a visible consequence. **The arithmetic is in the requirement
      and in the module doc**: `Circle`'s arm is a rounded rect of radius `r`
      around a `2r` box, and `fn line_quad` is the segment expanded by half its
      width, so the disc at the vertex *is* the round join and *is* the round cap.

- [ ] **The record order is asserted by position, and every polygon is convex.**
      `roads_are_recorded_minor_class_first` indexes the returned `Vec` and
      compares the positions of the `Service`, `Secondary` and `Arterial` `Path`
      commands; `the_route_is_recorded_after_every_road` and
      `the_car_is_recorded_last_and_is_a_circle_then_a_polygon` do the same for
      the route and the marker; `every_polygon_the_map_records_is_convex` runs the
      signed-area sign test over **every** `Polygon` in the output, for all three
      POI kinds and a sweep of headings. **The reason the order is a list and not a
      rule** is in the requirement: every one of the ten palette values is opaque,
      and `command_quads` + the batch order mean the last opaque command recorded
      is the top one. **Mutation evidence:** record the arterial class first and
      watch the three position tests fail.

- [ ] **The map is on screen, on the demo page only, and it is a real map.**
      Release build, `--tab=demo`, captured with the stock method of
      `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture method* verbatim — window
      id from `DISPLAY=:0 xwininfo -root -tree | rg '"roados ui_demo"'`, and
      `DISPLAY=:0 magick import -window <id>` — with `pgrep -a -x ui_demo` in the
      same call as each capture, **and no seed, no environment variable and no
      rebuilt binary**, which is stated because a capture that is described as
      evidence without saying what produced it is the failure this guards against.
      **The capture is read
      for six things and each is stated in the handoff:** a full-bleed ground with
      blocks, three weights of road, a blue route that turns a corner and lies on
      the road grid, three POI pips, the car marker with a nose pointing along the
      route, and **the seven-button tab bar** with `demo` the seventh. **It is also
      captured in the light theme**, because `DEMO_APPLICATION.md` § *Design
      principles* records that *"both of the operator's own photographs are the
      light theme"*, and a map that reads only in one theme has not been seen.

- [ ] **The six gallery pages changed only inside the tab-bar strip.** Captured
      **before and after** on all six, by the same commands, and compared with
      `magick compare -metric AE before.png after.png null:` per page. **The
      criterion is AE 0 outside `y < TAB_BAR_HEIGHT` and outside the fps readout's
      band** — *not* AE 0, because **the seventh tab button is drawn on every page
      and the bar is on every page**, which is the one thing this task cannot avoid
      and the reason the criterion is a region rather than a number.

      **The mechanism is four facts, and saying so is the criterion.**
      **`placed_handles` did not gain a row** — `rg -n 'map_node' ui/src/ui_demo/src/main.rs` shows it in `Demo::new`, in the `page_members` row and in the
      paint arm, and **nowhere in `fn placed_handles`**, and the test
      `the_map_node_is_not_in_placed_handles_and_the_two_counts_did_not_move`
      asserts both lengths unchanged. **So `every_page_places_every_rect_where
      _the_gallery_placed_it` and `no_two_placed_rects_overlap` hold with every
      assertion untouched**, which is what makes them free rather than lucky:
      **the map's rect is full-bleed and would overlap every gallery rect.** The
      gallery's geometry is absolute constants and the root is `LayoutMode::
      Absolute`, so **no gallery rect moved**; **`CONTENT_TOP` is unchanged**, so no
      page shifted; and the only node whose recorded commands changed on a
      gallery page is **the tab bar's row**, by one button. A change that cannot
      move a pixel outside a strip is **demonstrated not to**, which is why the
      greps are part of this criterion and not a remark.

- [ ] **`cargo test --all-features` is green with every named test present, and the
      three counts are pasted.** From `ui/`: `cargo fmt --check`;
      `cargo build --all-targets --all-features`;
      `cargo clippy --all-targets --all-features -- -D warnings`;
      `cargo test --all-features` with **each of the twenty-six `map.rs` tests and
      each of the six new `main.rs` tests listed by name in the handoff**, against
      a baseline of **1894 (1450 + 224 + 220)**, so the total is **1894 + 32 = 1926**
      or higher with **nothing removed**; `cargo doc --no-deps` clean; and
      `cargo audit` **recorded as not installed on this host, not passed**.
      `layout_walk_cost` is **still `#[ignore]`d and unmodified** —
      `git diff` over `ui/src/ui_core/src/layout.rs` shows no change to it, which
      is moot here because **this task does not touch `ui_core` at all**. **And
      the handoff lists the three deliberate breaks with their failure output**,
      including the third one's **survivor**, because a break that survives is
      either a weak test or a test on the wrong layer and the handoff says which.

- [ ] **The frame rate is measured on all seven pages and reported with the line
      pasted.** `.ai/tools/fps-check.sh 10 55` on the default page — **the only
      thing the script can do**, since it takes `seconds` then `floor`, builds
      release and runs `./target/release/ui_demo` with **no arguments and no page**
      — and then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for
      **each of the seven**, with the `roados-fps` line parsed by hand. **Every page
      above the floor of 55**, and the six gallery pages inside the recorded
      61.1–63.9 band in `IMPLEMENTATION_STATE.md` § *The frame rate, measured*,
      **because nothing this task does runs on them** — the map node is not in
      `order`'s painted set on those pages and its commands are not recorded. **The
      `demo` page's rate is the real number this task produces and is reported
      whatever it is**; the map's cost is roughly `roads × (1 + points)` commands per
      frame and the handoff states the count it measured. **This is the gate in
      `task-sequence.md` § *Gates* (*"No unmeasured run of the demo"*)**, and no
      other check here can see a frame-cost regression, because **a still of a
      4 fps application is pixel-identical to a still of a 60 fps one** — which is
      how a four-fps regression survived three reviews in this repository.

- [ ] **Nothing leaked in, and the dependency rule holds.** `git diff --stat`
      names **no** file under `ui/src/ui_core/src/`, **no** file under
      `ui/src/ui_demo/assets/`, and **no** file but `main.rs` under
      `ui/src/ui_demo/src/`. **`ui/Cargo.toml` and `ui/Cargo.lock` are unchanged**
      — the approved direct dependencies remain `sdl3 0.20`, `glow 0.18` and
      `freetype-rs 0.38`, and **a map drawn from four draw commands is not a
      licence decision against GPLv3**. **`rg -c unsafe ui/src/ui_demo/src/map.rs`
      is zero**, and `rg -n 'unwrap\(|expect\(|panic!|unimplemented!|todo!'` over
      `map.rs` returns **nothing**; `as` appears **exactly twice**, both inside
      `world`'s LCG draw, both with the arithmetic that makes them exact. **No `pub`
      item is added to `ui_core`** — `map.rs` is a module of a binary, so `pub`
      there is crate-visible and **not a semver question at all**, which is the
      second reason gap #1 is not a library task. **No asset was added, so
      `TASK_UI_PRIM_39`'s pipeline is not needed and not started.**

- [ ] **The four documents carry the decision, and none is closed.**
      `git diff --stat` names exactly `doc/ui/DEMO_APPLICATION.md`,
      `doc/ui/done/TASK_UI_PRIM_42.md`, `doc/ui/TASK_UI_PRIM_43.md` and
      `.ai/workflows/task-sequence.md`, plus the new
      `doc/ui/IMPLEMENTATION_STATE_DEMO.md`. **Row #1 of `DEMO_APPLICATION.md` §
      *Library gaps* gains a dated note saying the map exists and where it lives,
      and the row is neither deleted nor marked closed**, because its Blocks column
      names a navigation screen that does not exist. **§ *Open questions* item 2 is
      amended and stays on the list**, carrying the provisional decision, its four
      reasons, the reversal path, and the sentence that *"the decision is this
      task's; the question is still the operator's"*. **Item 7 is answered and
      removed**, with the answer `demo` and the date. **§ *Task structure*'s
      paragraph placing gap #1 in this sequence gains no change and is cited by 42's
      and 43's amendments.** **`task-sequence.md` § *Scope* names
      `doc/ui/TASK_UI_DEMO_*.md`** and its state file, and **no other `.ai/` file
      has a diff** — `git diff --stat -- .ai/` names one line.

- [ ] **What the handoff does not claim, in those words.** It states that **the map
      does not pan, does not zoom and does not rotate**; that **there is no raster
      tile layer and no real geography** and that the procedural decision is
      **provisional and reversible in one constructor call**; that **no POI carries
      a label**; that **a road's width is pixels and correct only at one camera
      scale**; that **off-window geometry is submitted and clipped on the GPU rather
      than culled**; and that **`Polygon`'s convexity is asserted by this task's own
      test until `TASK_UI_PRIM_51` makes the precondition checkable library-wide**.
      `developer.md` § Phase 3 requires a rendering change to be seen before it is
      reported as done, and this task **does** change what is on screen — the
      seventh tab and a full-bleed map — so the capture above is the evidence
      rather than an apology.

## Out of Scope

- **No raster tile layer, and no texture of any kind.** § *The map approach*
  gives the reason in the source: `Renderer`'s only texture entry point is
  `pub fn load_texture(&mut self, path: &Path)`, `Renderer::textures()` returns
  `&TextureCache` and not `&mut`, `TextureCache::load` needs `&mut self`, **and no
  task from 41 to 52 adds `textures_mut`, `make_texture`, `upload_texture` or
  `from_pixels`**. `ui/src/ui_demo/src/main.rs`'s `fn stand_in_picture` is the
  proof that the shape is closed: it drops its cache and the handle it returns
  names a texture the renderer's cache has no pixels for. **This is the one place a
  reviewer is most likely to think the task under-delivered**, and the answer is
  that the alternative is not "harder", it is **not reachable without a `ui_core`
  change, and `DEMO_APPLICATION.md` § *Task structure* forbids one here.**
  **Revisit when a pixels-to-texture entry point exists**, and the second `World`
  constructor is then the only other change.

- **No pan, no zoom, no map rotation.** `Camera { centre, pixels_per_metre }` is
  two fields and nothing writes them. **The fixed camera is the reason the map is
  capture-verifiable on a host where no pointer event has ever reached the
  window**, and a pannable map would have its only acceptance evidence behind an
  input this repository cannot inject. **Revisit with `TASK_UI_PRIM_46` (`L5`)**,
  which is what a pan needs — the field is already there and the gesture is the
  whole of the missing half.

- **No roads outside the viewport, but no perfect clipping either.** `MapSurface::
  paint` records a road whose projected bounding box **intersects** the viewport,
  and one whose box does not intersect records nothing. **A recorded road is
  recorded whole**, so up to one segment beyond each end is submitted and clipped
  by the GPU — `Demo::frame_clips` returns `None` for every node today. **No
  viewport-space clipping of the point list, and no partial `Path`**, because a
  `Path` has no clip field and clipping its points would be a second projection
  with its own round-off. **`TASK_UI_PRIM_45` (`#5`) makes the clip the node's own
  and changes nothing here**; a *tighter* answer is a later task and not this one.

- **No navigation, no routing, no arrival, no turn-by-turn, no lane guidance.**
  `DEMO_APPLICATION.md` § *Scope* § *Out of scope* names *"Real navigation/
  routing"* as out of scope for the whole demo, and this task draws **a route that
  exists**, not a route that is computed. `DEMO_APPLICATION.md` § *Could not
  verify* also excludes *"Lane-guidance / junction-view rendering"* outright, with
  no availability confirmed.

- **No satellite imagery, no labels, no POI names, no street names, no scale bar,
  no attribution line, no compass, no zoom controls, no traffic, no turn
  restrictions.** A POI is a pip and a glyph. **The scale bar and the compass are
  where task 02's dock goes**, and a POI *label* needs `TASK_UI_PRIM_49` (`L8`),
  because `DrawCommand::Text` carries no width and a half-visible label cannot be
  clipped without it. **Adding a POI name is therefore not a two-line change and
  the reason is a gap row, not taste.**

- **No `Icon` widgets for the POI glyphs, and no tintable `Image`.**
  `TASK_UI_PRIM_44` closes gap `#4` with `ui_core::widgets::Icon` and a tintable
  `DrawCommand::Image`, and **when it lands these three polygons are what it
  replaces.** The glyphs are `Polygon`s because `Polygon` exists today and
  `Icon` does not, and **the map does not wait**: a tintable raster icon is a
  *nicer* glyph, not a *possible* one. **`every_polygon_the_map_records_is_convex`
  becomes `Icon::paint`'s contract rather than this task's** — a class of change
  that is a migration, not a rewrite.

- **No real geography, no simplified real data, and the map-source question is
  left open.** § *The map source* takes a **provisional** decision and states the
  reversal, and `DEMO_APPLICATION.md` § *Open questions* item 2 stays on the list
  with the decision attached and **not** removed. **This is the honest reading of
  the instruction not to settle it silently**: the task is implementable because
  the decision is written down, dated, marked provisional and paired with the one
  constructor call that reverses it.

- **No `Screens`, no `TabBar`, no `Button::selected`, and no change to the demo's
  existing tab bar beyond the seventh button that `Page::ALL` produces.**
  `TASK_UI_PRIM_42` and `TASK_UI_PRIM_43` are **not started** and this task
  **rewires nothing they will rewire**. What this task does is amend the **counts**
  in their § *Sub-task 42.2* and § *Sub-task 43.2*, dated, because a task file
  written against six pages and implemented against seven is a file whose named
  tests cannot be written as named. **No `ui_core` requirement in either file
  changes.**

- **No chrome of any kind on the demo page: no top status bar, no bottom dock, no
      car-status pane, no translucent surface, no backdrop.** That is
  `TASK_UI_DEMO_02`, and it is a separate task because it depends on
  `TASK_UI_PRIM_41`'s `Painter::backdrop` and on a decision about
  `TASK_UI_PRIM_43`'s `TabBar`, neither of which has landed. **The demo page after
  this task is a map and a tab bar, and nothing else** — and it is honest about
  that, because `Page::Demo`'s doc says what the page holds today and points at the
  task that fills it in.

- **No demo page shortcut, no hidden key, no second way in.** `--tab=demo` is the
  **only** route to the map, and that is deliberate: a picture whose method was
  not stated is the failure this guards against, and **an acceptance criterion
  that needs a key is an acceptance criterion that cannot be met on this host.**

- **No `ui_core` change of any kind — no `DrawCommand` variant, no `Painter`
      method, no `ThemeToken`, no `LayoutMode`.** Four of these are tempting and
      all four are refused: **a `ThemeToken` for map colours** would put a demo's
      invented values into the crate's global palette, where `Theme::from_table`
      requires both themes to define every token
      (`dark_and_light_defines_every_token`) and a map colour is not a property of
      the UI library; **a mitre-join `Path`** would be a pipeline change for a
      corner the demo does not have; **a transform on `DrawCommand`** is gap `L2`,
      which `TASK_UI_PRIM_36` owns and deliberately gives to the mesh path; and
      **a convexity precondition on `Polygon`** is `TASK_UI_PRIM_51`'s deliverable.
      **`MapPalette` reading the flat global tokens is the whole of the theme
      coupling, and `TASK_UI_PRIM_50`'s scoping is what would change it.**

- **No test needing a display, a network, a filesystem or the wall clock.**
  `AGENTS.md` § *Rust* forbids all four, and every one of the thirty-two tests is
  arithmetic over values the module produced itself. **No `Instant::now()` in
  `map.rs`**, so the map is not the second instance of the trap that **a still
  screenshot of a 4 fps application looks exactly like a 60 fps one** — that
  one's evidence was a `load_char` per character
  per frame, and **a generator called per frame with no clock is the shape of that
  bug with a different name.**

- **Found in the tree and deliberately not fixed.** `ui/src/ui_demo/src/main.rs`
  has a comment on the tab bar's construction that records the **withdrawn**
  2026-10-03 decision — `DEMO_APPLICATION.md` § *Relationship to task 24* lists it
  as *"**known-stale** and is left for a code task: it is a source comment asserting
  a decision that has been withdrawn."* **This task does not fix it**, because
  `TASK_UI_PRIM_43` rewrites that whole block when it lands and
  `developer.md` § *Phase 2* refuses drive-by cleanups (*"Do not restructure what
  you were not asked to touch"*). **It is recorded here so the next reader knows it
  was seen and not missed.**

## Decisions taken instead

**Recorded 2026-10-09, by the operator.** This section is why the file is here
rather than deleted, and it is the only part of it that describes anything that
is still true.

**The scope was judged too ambitious.** The task file specifies a seeded world
generator, a fixed camera, three road classes with a draw-order rule, a route
that provably follows a road, three POI glyphs, a rotated car marker, a
ten-field palette, 26 tests in a new module, 6 more in `main.rs`, four amended
documents and a created state file. **The operator's judgement is that this is
more than the demo needs to have a map**, and the argument was not about the code
— `map.rs` is an LCG, a grid, a projection and a paint walk — but about the
whole apparatus.

**The replacement is a background image.** `assets/img/` holds a map picture,
loaded through the mechanism `demo.png` already uses, drawn full-bleed under the
tab bar, with **nothing on top of it**. `TASK_UI_DEMO_02`'s chrome and
`TASK_UI_DEMO_03`'s car-status pane are what put things on it, and they were
going to anyway.

**Three consequences, recorded here rather than argued anywhere else.**

1. **The licence of the image is the operator's decision and it is a risk
   accepted, not a permission held.** The replacement task file names the terms
   clauses and states the acceptance in full; a reader who disagrees can see
   exactly what was accepted and on what grounds.
2. **`ui/src/ui_demo/assets/img/` is not in this repository**, by operator
   decision, which reverses what `TASK_UI_PRIM_39` decided for every other asset
   in that directory. The consequence is recorded rather than solved: **the
   seventh page's capture is not reproducible from a fresh clone.**
3. **Gap #1 is not closed by either file.** A picture is not a map widget. The
   row in `DEMO_APPLICATION.md` § *Library gaps` keeps its numbering, its
   severity and its *Blocks* entry, and the replacement task file says so.

**What would bring this file back.** A camera that moves — `TASK_UI_PRIM_46`
(`L5`) is what a pan needs — or a pixels-to-texture entry point in `ui_core`,
which no task in 41 to 52 adds and which this file's § *The map approach*
establishes is the only thing standing between a procedural world and a raster
one. **Either of those makes this design cheaper than the replacement, and
neither exists today.**
