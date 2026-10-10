# TASK_UI_DEMO_01: The Demo Page — a Seventh Tab, and the Map Image Under It

## Goal

**Add the seventh page**, `Page::Demo`, and give it the one thing every other
thing on it stands on: **a map image, full-bleed, under the tab bar.**

**The page is the infotainment demonstration and the map is its base layer, not
the other way round.** `DEMO_APPLICATION.md` § *Design principles* item 2 is
*"Map-centric layout — the map is the base screen; every other surface is a
panel over the map"*, and § *What this means for the demo's shape* item 1 is
*"the demo must have a map it can put things on top of"* — **which is a
statement about what the page is for, not a statement about what the page
contains.** This task supplies the base layer and nothing else. **The chrome is
`TASK_UI_DEMO_02`, the car-status pane is `TASK_UI_DEMO_03`, the indicator
column is `TASK_UI_DEMO_04` and the card carousel is `TASK_UI_DEMO_05`** — four
tasks, each of which is a panel over this page, and **the page exists for all
five of them.** This one is first because it is the only one that needs nothing
but the tab bar that already exists.

**The map is a picture, and this task says so wherever it says "map".**
`ui/src/ui_demo/assets/img/` will hold `map_demo.png`, the demo will load it
through the mechanism `demo.png` already uses, and `Page::Demo` will draw it
full-bleed. **No geometry, no camera, no projection, no marker, no route, no
POI** — and none of those is out of scope by taste; each is listed in § *Out of
Scope* with the task that owns it.

**What it closes: the visible half of gap `#1`** — `DEMO_APPLICATION.md`
§ *Library gaps*, row **1**, *"**Map widget** — no map renderer exists or is
planned. The demo's centerpiece"*, Blocks **"Map/navigation screen"**.

**This replaces a task, and the replacement is the whole of its scope.** The
previous `TASK_UI_DEMO_01` specified a procedural vector map — a seeded world,
a fixed camera, three road classes, a route, POI glyphs and a rotated car
marker, 26 tests in a new `ui/src/ui_demo/src/map.rs`. **The operator judged that
too ambitious on 2026-10-09.** That file is at
[`doc/ui/backlog/TASK_UI_DEMO_01.md`](backlog/TASK_UI_DEMO_01.md), whole, with
its reasoning — **read its § *The map approach* before writing anything here,
because it is what proves the picture route was reachable all along.**

**What this task does not close.** **Gap `#1` is not closed by this task, and
its row is not deleted and not marked closed.** A picture is not a map widget:
there is no camera, nothing to pan or zoom, no route, no marker, and nothing
addressable in the picture's own coordinates. The row keeps its numbering
because four files outside `DEMO_APPLICATION.md` cite it by number, and it
keeps its Blocks entry because **no navigation screen is built.**

## Context

### The load-bearing fact: a picture was always reachable from `ui_demo`

**`Renderer::load_texture` reads from disk, and the demo already calls it.**
`ui/src/ui_core/src/render.rs:3109` is
`pub fn load_texture(&mut self, path: &Path) -> Result<TextureHandle, RenderError>`,
its body is `TextureCache::load_from_file` plus one
`ensure_standalone_texture` call, and its **only** call
site is `ui/src/ui_demo/src/main.rs:2571` in `load_picture`. The chain the
previous task's § *The map approach* says is broken is not broken **for a file
that already exists**; what does not exist is a route from `Pixels` to a
texture, which is a different question. `doc/ui/IMPLEMENTATION_STATE.md`
§ *Tasks `DEMO_01`–`DEMO_05`* records the conclusion in one sentence: *"no task in
41–52 adds `textures_mut`/`from_pixels`, **so a PNG would work today**."*

**`ui_core` has no diff in this task**, and that is the first bullet of §
*Acceptance Criteria*.

**The three APIs this uses, each read out of the source:**

- **`Image::new(&mut nodes, texture, source)`** — `ui/src/ui_core/src/widgets/image.rs`,
  the same constructor `main.rs:4373` builds the gallery's image with, and the
  same `ImageSource::of(renderer.textures(), texture)` beside it.
- **`Image::paint(rect) -> Vec<DrawCommand>`** — `widgets/image.rs:1011`. It
  records **one** `Painter::image` and nothing else, and it is
  `main.rs:7620`'s existing widget arm with the map's handle added.
- **`ImageFit::Cover`** — `widgets/image.rs:260`. Its own doc is the reason it
  is the fit and not `Contain` or `Fill`: *"The quad is the node's own rect,
  exactly as `Fill` draws it, and nothing overflows… only this one is right
  without one, because `DrawCommand` has no scissor state."* **A background
  that overflowed its node would be a background over the tab bar.**

**Two facts about the pipeline that make the image cheap.** A map image is
larger than `ATLAS_MAX_IMAGE` (512, `ui/src/ui_core/src/texture.rs:85`), so
`TextureCache::insert` (`texture.rs:775`) routes it to `insert_standalone` and
it gets a GL texture of its own at `GL_RGBA8` with `GL_LINEAR` and
`GL_CLAMP_TO_EDGE` — **one upload, once, at load** (`render.rs:4350`). And the
image pass draws with blending on regardless of what the batcher decided
(`render.rs:3315`), so **a PNG with no alpha channel is still drawn correctly**
and a stand-in of transparent pixels is drawn as nothing.

### The asset is not in this repository, and that is the operator's decision

**The image is not in this repository, and where it sits today is a live
hazard.** It was supplied on 2026-10-09 as **`img/map_demo.png` in the repository
root**, and **that path is not ignored by anything** — `git check-ignore -v
img/map_demo.png` exits 1 and `git status` reports `?? img/`, **so the next
`git add -A` in this repository would commit 2.8 MB of captured Google Maps
imagery and void the acceptance recorded below.** Requirement 3 adds the
`.gitignore` lines and this section is why it is a requirement and not a
preference.

**The rest of this section is about the design's consequence, not its
plumbing.** The image is added to `.gitignore` and is not committed.
Every other asset in `ui/src/ui_demo/assets/` is tracked — 71 files, **68 of
them recorded with hashes in `MANIFEST.sha256`**, the manifest naming exactly
the pipeline's outputs and saying so (*"`demo.png` is a pre-existing committed
asset and is not listed"*, `check_assets.py:271`) — because
`TASK_UI_PRIM_39` built them from a two-URL allow-list and the gate
`tools/asset-pipeline/check_assets.py` verifies them. **This reverses that for
one directory, by operator decision on 2026-10-09.** § *Out of Scope* records
what that costs; the two consequences a reader has to know are:

1. **The seventh page's capture is not reproducible from a fresh clone.** The
   page renders, the node lays out, the tab bar grows a button, and the map's
   own pixels are not there. `check_assets.py`'s `denylist_hits` walks the whole
   assets tree and **skips `.png` contents** (`check_assets.py:197-203`), so an
   untracked image in `img/` is invisible to all six checks — **and the guard
   that was built to be structural rather than a review instruction does not
   cover this directory.** `TASK_UI_PRIM_39.md` § *Out of Scope* says so of the
   network allow-list in its own words (*"the guard is structural rather than a
   review instruction"*), and `IMPLEMENTATION_STATE.md` § *Task 39* records that
   task as *not reviewed* — **so nothing in this repository has yet checked what
   its guard does with a hand-dropped binary.** Widening it is not this task's
   job and is named in § *Out of Scope*.
2. **The licence of the image is the operator's decision and it is a risk
   accepted, not a permission held.** § *The licence, and what was accepted*
   states what was accepted and on what grounds.

### The seventh page, and the four places a page count lives

**`DEMO_APPLICATION.md` § *What a seventh page costs* is the mechanism, and
`ui/src/ui_demo/src/main.rs` is where it happens.** Each item below is read out
of the source, cited by symbol:

| what | where | what this task does |
|---|---|---|
| **`const ALL: [Page; 6]`** | `main.rs:2012` | becomes **`[Page; 7]`** — a fixed-size array, so the *type* carries the count and the compiler carries the change |
| **`fn name(self) -> &'static str`** | `main.rs:2040`, whose doc says *"The only place the six names are written out"* | gains **`Page::Demo => "demo"`**. `from_name`, `page_names` and `usage` all derive from `ALL` through `name`, so one spelling reaches `--tab=`, `--help` and the unknown-name message |
| **`Demo::tabs`** | the `Tab { page, button }` row, built by `for &tab_page in &Page::ALL` (`main.rs:3973`) | gains a seventh row **because it is built by the loop** — the seventh tab button is not written anywhere |
| **`Demo::page_members`** | `main.rs:5462` | one row: `on(Page::Demo, map_node, false)`. **This table has been the site of two mutation-found majors** — `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided, and what it found* records a dropped row giving *"0 failed / 1814, with the text column drawn on the wrong page"*, and the task-table's row **24.2** records the same class on `placed_handles` at *0 failed / 1817*. **A seventh row is exactly where it recurs** |
| **`Page::DEFAULT`** | `main.rs:2029` | **stays `Page::Pads`**, and requirement 4 says so in a doc comment on the row |
| **`root`'s child list** | `main.rs:5230` | the map node goes in **immediately after `background`**, for the reason in requirement 4's `root` bullet |

**Two places the map node must NOT be added, and both are load-bearing.**

- **`Demo::placed_handles` must not gain a row.** It is built by
  `tests::no_two_placed_rects_overlap` and by
  `tests::every_page_places_every_rect_where_the_gallery_placed_it`, and the
  map's rect is **full-bleed — it overlaps every gallery widget's rect by
  construction.** The list is of leaves the demo places, and § *Where the map is
  covered instead* says which gate holds it down.
- **`tests::expected_placed_rect_names` must not gain a row**, and
  `tests::assert_placed_handles_is_complete` (`main.rs:13907`) must still
  assert its length unchanged — **29 with a model, 30 without one.**
  `TASK_UI_PRIM_52.md` § *Acceptance Criteria* states the same rule in its own
  words: *"**No row of any of them is added or loosened.**"

### Where the map is covered instead, and the one place it must be added

**Three existing gates hold the map down without naming it, and a fourth one
needs it added by name.** Both halves are here because a future reader who
cannot find the map in `placed_handles` will assume it is uncovered, and because
**the fourth is the one that fails if this task is implemented and this section
is ignored** — `tests::rects_by_page` reaches its assertion on `Page::Demo` and
the map is in neither of the two lists it compares.

**The three that hold it down:**

1. **The paint gate**, `tests::a_page_records_no_command_on_a_node_that_is_not_its_own`
   (`main.rs:19781`). It loops `Page::ALL` and asserts every node that recorded
   a command is either one of that page's own rows or on
   `tests::always_painted_handles`. **The map node is a row of `Page::Demo` and
   of no other page**, which is the whole of what keeps it off the six gallery
   pages.
2. **`tests::every_page_lists_at_least_one_node_and_no_node_is_on_two_pages`**
   (`main.rs:19706`), which asserts every node in `Demo::order` that is not a
   row of the table is one of the always-painted set — **so the map node being a
   row is what keeps this from failing**, and `Demo::order` reaches it because
   it is attached to `root` before `paint_order` runs at `main.rs:5422`.
3. **`tests::every_page_places_every_rect_where_the_gallery_placed_it`**, which
   compares the six pages with each other — **and cannot see the map at all**,
   because the map is in no page's `placed_handles`. § *What does not detect a
   change to the map* names that as its limitation rather than as coverage.

**The fourth, and the one that needs a row added.** `tests::rects_by_page`
(`main.rs:13813`) builds every page and calls
`tests::assert_every_drawn_leaf_is_named_or_excused` on it. That assertion walks
`Demo::order` for nodes that **have extent and record commands** — and the map
node has both, 1280 × 956 and one command — and asserts the drawn set minus
`placed_handles()` **equals** the exemption list `tests::undrawn_leaf_exemptions`
(`main.rs:13999`) intersected with the drawn set. **The map is in neither list,
so on `Page::Demo` that `assert_eq!` sees `unseen == [map]` and fails.**

**Three tests go red without it**, and none of them is about the map:
`tests::no_two_placed_rects_overlap` (`main.rs:14208`),
`tests::every_placed_rect_is_inside_the_window` (`main.rs:14174`) and
`tests::nothing_the_demo_places_reaches_into_the_strip` (`main.rs:22262`) — all
three iterate `rects_by_page`, which iterates `Page::ALL`. **So requirement 4
adds the map node to `tests::undrawn_leaf_exemptions`**, beside the background and
the bar, which is the same list those two occupy and for the same reason: **they
cover the window and are not placed leaves.** **This is the one place this task
adds a row to a list it is told not to touch, and § *Out of Scope* says why the
two lists are different.**

**The distinction that makes both rules true at once.** `placed_handles` is the
list of **leaves the demo places, and its tests are about rectangles colliding** —
so a full-bleed rect there would fail `no_two_placed_rects_overlap` for being
large, which is the correct answer for a placed leaf and the wrong one for a
background. `undrawn_leaf_exemptions` is the list of **nodes that draw without
being placed**, and its assertion asks only whether each drawn node is accounted
for. **The background is already in it and is already full-bleed, which is the
precedent the map follows rather than a rule invented for it.**

### What does not detect a change to the map, and is therefore tested directly

**Three things this change can get wrong that no existing gate sees**, which is
the reason requirement 5 has tests of its own rather than leaning on the
gallery's:

- **The node's rect.** Nothing measures it — the map is in no `page_rects`, and
  a node laid out at `y: 0` would be *behind the tab bar*, which is a plausible
  failure and looks like a styling choice in a capture.
- **That the image is `Cover` and not `Fill`.** A `Fill` stretches; a `Contain`
  letterboxes. Both draw, both pass every gate above, and both look like a
  deliberate choice to a reader who did not know.
- **That the page is reachable by name.** `--tab=demo` is the only route in and
  the paint gate does not know the spelling exists.

### The licence, and what was accepted

**Recorded 2026-10-09, by the operator, in full. This is the honest half of the
decision and it belongs in the task rather than in a conversation.**

The image was captured from Google Maps. **The operator's stated use is local
and not distributed.** Read against the published terms, that use sits outside
what Google grants:

- **Maps End User Additional Terms** (last modified 2026-01-27,
  `maps.google.com/help/terms_maps`) §2.2 — *"copy the content (unless you are
  otherwise permitted to do so by the Using Google Maps, Google Earth, and
  Street View permissions page or applicable intellectual property law,
  including 'fair use')"*. §1's grant is to *view*, *annotate*, and *"publicly
  display content with proper attribution"* — none of which is shipping a
  rendering as one's own asset.
- **The Brand Resource Centre Geo Guidelines** are the permission grant, and the
  one clause that mentions screenshots is scoped to *"add custom labels or
  graphics using third-party software"* — an annotation intermediate. The same
  section forbids *"significant[ly] alter[ing] how Google Maps … would look
  online"* and states *"we're not able to grant exceptions"*.
- **The trademark guidelines** (`about.google/brand-resource-center/rules`)
  cover *"trade dress, including the look and feel of Google web design
  properties … distinctive color combinations, typography, graphic designs,
  product icons, or imagery associated with Google"*, and the Geo Guidelines name
  the **red pin** and the **Google Maps wordmark** as marks.
- **Attribution does not cure it.** Attribution is a condition of the permitted
  uses; this use is not among them. *"All uses … must provide attribution …
  We do not approve of any use of content without proper attribution, in any
  circumstances"* — which makes it necessary, not sufficient.

**What was accepted, and what it does not accept.** The operator accepted the
risk for a **local, undistributed** run, which is not the use the guidelines
permit either but is not a distribution. **The image is not committed
(requirement 3), so no GPLv3 downstream distributor inherits it** — which is
the specific harm a committed image would cause, and the reason requirement 3 is
not a preference. **If this image is ever committed, published, or shipped, the
acceptance is void and the terms apply as written.** The task does not argue
this further: it is the operator's decision, made with the clauses in front of
them, and `DEMO_APPLICATION.md` § *Open questions* item 3 (**the Tesla mark**,
also open) is the same kind of question recorded the same way.

### What the tests already know, and the baseline

**Re-measured on the tree this file is written against, 2026-10-09:
`cargo test --all-features` from `ui/` reports **2058 passed, 0 failed, 1
ignored** — 1587 in `ui_core` **passing of 1588 registered, the one ignored
being `layout_walk_cost`** — plus 3 in `tests/model_file.rs`, 1 in
`tests/test_painter.rs`, 236 in `ui_demo`, and 231 doctests. Every test this
task adds is additive, so the total to reach is **2064 passing / 2065
registered**. **The number the superseded task quoted, 1894, is stale and is
recorded as stale here** rather than left in a document a reader would take as
current.

**Two existing counts are asserted by value and must not move, and they live in
two different tests.** `tests::assert_placed_handles_is_complete`
(`main.rs:13907`) asserts that `expected_placed_rect_names()`'s length is **29
without the model-status line and 30 with it**, and that `placed_handles()` equals
it — **and that is the assertion the map node must not disturb.**
`tests::every_page_lists_at_least_one_node_and_no_node_is_on_two_pages`
(`main.rs:19706`) asserts that **five** rows of `Demo::page_members` are `Tab`
stops, and the map node is **not focusable** — a map is not a control `Tab` can
move focus to, on `PageMember::focusable`'s own argument that *"a stop where
nothing lights up is a stop a reader cannot see"*.

### A dependency this sequence did not have, and now has half of

**`.ai/workflows/task-sequence.md` § *Scope* did not name this sequence.** It
listed `doc/ui/TASK_UI_PRIM_*.md` and `doc/platform/TASK_CROSSPLATFORM_*.md` and
nothing else, so the workflow these five tasks run under did not apply to them:
§ *State* says each sequence's `IMPLEMENTATION_STATE.md` records its progress,
and **this sequence had nowhere to record anything.** The superseded task found
this and made fixing it a requirement of its own implementation.

**It was fixed earlier than that, on 2026-10-09, when this file was written.**
`task-sequence.md` § *Scope* now lists `doc/ui/TASK_UI_DEMO_*.md` and § *State*
names this sequence's state file. **A workflow that covers a task file being
re-specified is the right order**: the sequence had no workflow while it was
being rewritten, and has one now.

**The half that is still outstanding is the state file itself.**
`doc/ui/IMPLEMENTATION_STATE_DEMO.md` **does not exist**, and requirement 7
creates it **as part of the implementation, not of this specification** — so
until this task lands, the five `DEMO_*` tasks still have no state file and
`IMPLEMENTATION_STATE.md` § *Tasks `DEMO_01`–`DEMO_05`* is still the only place
that says so. **Two halves and not one is deliberate**: writing a state file
about a task that has not run would record nothing, and creating it at
specification time would be a file whose every row said *not started*.

**This is an `.ai/` edit, and `AGENTS.md` § *AI Engineering System* permits one
when the task *"explicitly concerns the AI engineering system — reviewing,
extending, or fixing it counts"*; this does, and **no other `.ai/` file is
touched.** `git diff --stat -- .ai/` must name exactly one line.

### Scope, measured against `developer.md` § *Scope check`

**Nine files in all and two components — the file count is over the threshold of
five, so the argument for not splitting is written down rather than assumed.**
What shrank when the procedural map went to the backlog is the *code*: **there
is no new Rust file**, because a background image needs a loader, a widget and a
node, and those three are already written for `demo.png` and `sedan.roados`.

| file | what changes |
|---|---|
| `ui/src/ui_demo/src/main.rs` | the map constants, `load_map_picture`, `stand_in_map_picture`, `map_candidates`, the `Demo::map` field, the node and its one `page_members` row, `root`'s child list, one arm in `Demo::frame`'s paint walk, `Page::Demo`, `Page::ALL` at seven, `fn name`'s seventh arm, and the six new tests of § *Testing* |
| `.gitignore` | the `ui/src/ui_demo/assets/img/` line |
| `doc/ui/DEMO_APPLICATION.md` | row `#1` gains a dated note and is **not closed**; § *Open questions* item 2 amended and **still open**; item 7 answered and removed; § *What a seventh page costs* gains a dated note |
| `doc/ui/TASK_UI_PRIM_42.md` | every count in § *Sub-task 42.2* that names six becomes seven |
| `doc/ui/TASK_UI_PRIM_43.md` | the same in § *Sub-task 43.2*, plus the three test names that carry the count |
| `doc/ui/IMPLEMENTATION_STATE_DEMO.md` | **new.** The task-table row, the record, the waivers, the frame rate |
| `.ai/workflows/task-sequence.md` | § *Scope* gains `doc/ui/TASK_UI_DEMO_*.md` and names this sequence's state file |
| `doc/ui/backlog/README.md` | **new, already written** 2026-10-09: what the directory is, and why a superseded task is kept rather than deleted |
| `doc/ui/backlog/TASK_UI_DEMO_01.md` | **already moved**, 2026-10-09, with its superseded note and § *Decisions taken instead* |
| `doc/ui/IMPLEMENTATION_STATE.md` | **already amended** 2026-10-09: the `DEMO-01` table row, and the two paragraphs of § *Tasks `DEMO_01`–`DEMO_05`* this rewrite makes false |
| `doc/ui/TASK_UI_DEMO_02.md` | **already amended** 2026-10-09: four references to the map's shape — § *Testing*'s control, and § *Out of Scope*'s asset line, car-marker line and per-frame cost |
| `doc/ui/TASK_UI_DEMO_03.md` | **already amended** 2026-10-09: three places that say there is no map in this repository, which was already false and is now false differently |

**Two components is under the threshold of three, eight of the nine files are
`.md`, and the whole code change is three constants, one loader, one field, one
node, one row and one arm in a file this repository's own `main.rs` already holds
all of those for two other assets.** `developer.md` § *Scope check* splits on
files *or* components, so the file count is the one that bites and the argument
against fanning out is the one above. **The superseded task split into three
sub-tasks and one of them was four `.md` files with no build at all** — which is
the shape of work that does not need a subagent and cannot be reviewed faster by
one.

**Six of the nine rows are already done, and the table marks each one.** They
landed with this file on 2026-10-09, because **a task file that contradicts the
documents around it is wrong on arrival** (`AGENTS.md` § *Working context*:
*"The artifact wins"*). **What is left for the implementation is `main.rs`,
`.gitignore` and `IMPLEMENTATION_STATE_DEMO.md`** — three files, and the first is
the only one with code in it.

## Requirements

1. **Three constants and one loader, beside the three the demo already has.**
   `MAP_IMAGE_FILE: &str = "map_demo.png"`, `MAP_IMAGE_RELATIVE: &str =
   "src/ui_demo/assets/img/map_demo.png"` and `MAP_IMAGE_SIZE: (u32, u32) =
   (1359, 970)` — and nothing else.

   **`MAP_IMAGE_SIZE` is the operator's image's own size, measured 2026-10-09:**
   `1359 × 970`. It is a **different aspect from the node's `1280 × 956`**, which
   is what makes it the right constant and not a copy of the node's — **and it is
   the reason the fit is observable at all.** `ImageFit::Cover` on a source
   *wider* than its node keeps its whole height and crops the sides, so the
   sampled `uv` is a band **1359 → 1359 · (1280/956) = 1267 px wide**, about
   **6.8 % cropped off each side**; under `ImageFit::Fill` the whole texture is
   sampled and the 6.8 % is stretched instead. **Those are different pictures and
   a test can tell them apart**, which is why § *Testing*'s first break fails on
   `uv` and not only on `fit()`. **Had the two shapes matched, `Cover` and `Fill`
   would have been indistinguishable** (§ *Where the map node's size comes from*
   carries that argument); **this constant is load-bearing, not a stand-in
   formality.** It is also what keeps the image out of the shared atlas: both
   dimensions exceed `ATLAS_MAX_IMAGE` (512), so `TextureCache::insert` routes
   it to `insert_standalone` and it gets a GL texture of its own —
   **5.0 MB at `GL_RGBA8`, once, at load.** **`fn map_candidates() ->
   Vec<PathBuf>`** is `model_candidates`' ten lines (`main.rs:2700`) with
   `MAP_IMAGE_RELATIVE`, because it is `fn asset_candidates_from(exe, dir,
   relative)` (`main.rs:2720`) that walks — and `model_candidates`' own doc names
   the cost of writing another: *"a second search-path walker would be a second
   answer to one question"* (`main.rs:2699`). **Three callers of one walker is the arrangement the
   demo already has** — `load_picture` and `load_model` both go through it — and
   a third is one different argument, not a second function.

   **`fn load_map_picture(renderer: &mut Renderer) -> Option<Picture>`** is
   `load_picture` (`main.rs:2561`) line for line: `map_candidates()`, the first
   `is_file()`, `renderer.load_texture(path)`, `ImageSource::of`, and **three
   `eprintln!`s in the same message shapes** — not found, load failed, cache
   could not place it. **A missing map must not take the window down, and the
   demo must say where it looked**, because *"an unexplained blank rectangle is
   what two of this repository's defects were mistaken for"*.

2. **`fn stand_in_map_picture() -> Option<Picture>` — the transparent stand-in,
   built at `MAP_IMAGE_SIZE`, and why it must be.** It is `stand_in_picture`
   (`main.rs:2752`) with the map's own constants, and **it builds its own
   `TextureCache` and drops it**, which is what the superseded task's § *The map
   approach* records as the shape a pixels-to-texture route cannot escape.

   **The stand-in exists because the file may be absent, and it is not a
   nicety.** `ui/src/ui_demo/assets/img/` is not in this repository
   (§ *The asset is not in this repository*), so **a demo run here, and any run
   on a fresh clone, has no map** — and a map node that recorded nothing would
   make the demo page empty, which `a_page_records_no_command_on_a_node_that_is_
   not_its_own` reads as a defect rather than as an absent file.

   **It must be `MAP_IMAGE_SIZE` and not an arbitrary shape**, because the tests
   assert geometry: `the_map_is_an_image_at_the_node_s_own_rect_with_the_cover_fit`
   reads the recorded `uv`, and **a stand-in of the wrong shape would make the
   crop the wrong one and the assertion would be measuring the fixture.**
   `ASSET_SIZE`'s own argument covers it — *"a stand-in has to be the shape of
   the thing it stands in for"* — **and `ASSET_SIZE`'s second clause is the one
   that applies: every fit's geometry is computed from the source's own width and
   height**, so a stand-in of a different shape would make `Cover` crop a
   different amount from the one on screen.

3. **`.gitignore` gains the image's path, and no byte of it is committed.**
   The line carries the reason in the comment above it, the way the existing
   `/.asset-cache/` entry does.

   **Two paths, because the operator's file is at neither of the ones originally
   specified, and the *correct* path is a decision this task should not make for
   them.** The image was supplied on 2026-10-09 at **`img/map_demo.png` in the
   repository root** — and **`git check-ignore` says that path is *not* ignored
   and `git status` reports `?? img/`**, so a `git add -A` would commit 2.8 MB of
   captured Google Maps imagery, which is the one outcome § *The licence, and
   what was accepted* says voids the acceptance. **Two `.gitignore` lines, both
   anchored, and the handoff says which is live:**

   ```
   /img/
   /ui/src/ui_demo/assets/img/
   ```

   **The demo finds the file either way with no code change**, because
   `asset_candidates_from` (`main.rs:2720`) takes the path relative to a
   directory it walks **up from the executable**, and `ui/target/release/ui_demo`'s
   ancestors include `/workspace`. `ROADOS_ASSET_DIR` also works, since it names a
   **directory** and the demo joins `MAP_IMAGE_FILE` to it — so
   `ROADOS_ASSET_DIR=/workspace/img` is a third route. **Which path is
   canonical is the operator's call and the task file does not pick**: the
   `.gitignore` excludes both, `MAP_IMAGE_RELATIVE` names one as the default,
   and **the acceptance criterion is `git status --porcelain` staying empty with
   the file in place**, which is true of both.

4. **`ui/src/ui_demo/src/main.rs`: `Page::Demo`, the seventh page, and the map
   node under it.** In order, each step naming what it is:

   - **`Page::Demo`**, a variant whose doc comment says **what the page is
     for before naming what it holds today**, because the two are different
     questions and the page is the answer to the first. It reads: the
     infotainment screen this repository's direction describes, reached at
     `--tab=demo`; **the map is its base layer and the chrome goes over it**;
     **today it holds the map image and the tab bar and nothing else**, and
     `TASK_UI_DEMO_02` through `_05` fill it in, one panel each. **The
     one-line rule: the doc names the five tasks before it names the map**, or
     a reader meets a picture and concludes the page is a picture.
   - **`const ALL: [Page; 7]`**, `Page::Demo` **last**, after `Page::Overlays`.
     The order is load-bearing three times (`--help`, the unknown-name message,
     the tab bar's button order) and `Page::Demo`'s doc says so.
   - **`fn name`'s seventh arm, `Page::Demo => "demo"`.** It is the **only** new
     spelling, and `Page::name`'s doc keeps its claim that it is *"The only
     place the six names are written out"* with **"six" changed to "seven"** and
     a dated note saying which task changed it.
   - **`const DEFAULT` stays `Page::Pads`** and its doc gains one sentence: *"The
     seventh page does not become the default. `Page::DEFAULT` is the page every
     capture taken for tasks 11 to 22 contains, so a capture that used to need no
     argument is still reproducible."* **One line of `main.rs` is the whole of
     that criterion.**
   - **`Demo::map: Image`** as a field, not a local, for the reason
     `always_painted_handles` is written out: the paint walk reaches it through
     `Demo::order` and needs its handle on every frame.
   - **`Demo::new`**: `load_map_picture(&mut renderer)` is passed in beside
     `picture` and `model`; a missing one becomes `stand_in_map_picture()`, and
     **the fallback is recorded, not silent** — a `map_is_stand_in: bool` beside
     the existing `image_is_stand_in`, used by one test.
   - **The widget**: `Image::new(&mut nodes, map_picture.texture,
     map_picture.source)`, `set_fit(ImageFit::Cover)`, `snap_to_state()`, and
     **no corner radius** — a map is not a card, and `IMAGE_CORNER_RADIUS`'s own
     argument for it is *"the corner is a shader feature, and a square corner is
     not evidence it ran"*, which applies to the gallery's 220 × 160 picture and
     not to this.
   - **The node**: `Constraints::tight(Size::new(WINDOW.width, WINDOW.height -
     CONTENT_TOP))` and `layout_mut().set_position(Some(Offset::new(0.0,
     CONTENT_TOP)))` — **`CONTENT_TOP` added at the position and never to the
     constant**, which is the rule five other positions in that function already
     follow.
   - **`root`'s child list gains the map node, immediately after `background`
     and before `tab_bar`.** **The reason is paint order and nothing else**:
     attached second, the map paints over the window background and under the
     bar and everything the gallery draws.

     **Hit testing is not affected by this position, and the reason is worth
     writing down because the opposite is easy to assume.** `hit_test_from`
     (`ui/src/ui_core/src/input.rs:333`) walks a node's children with `.rev()`
     and returns the **first** hit, and `root`'s last child is the controls layer
     — `Constraints::tight(WINDOW)`, and **not** a `page_members` row, so
     `sync_page_visibility` never writes `set_visible` on it and it is visible on
     every page. **It therefore contains every point in the window**, so
     `hit_test_from` returns one of its children or the layer itself and
     **never reaches any `root` child below it — the map included, wherever it
     is in the list.** `LayoutState::hits`, which is what would let a host opt out
     of being a target, **does not exist yet** (`grep -c 'hits' ui/src/ui_core/src/layout.rs`
     is 0; it is `TASK_UI_PRIM_42` requirement 8). **So the position in the child
     list is a paint-order decision, and a claim about hit testing made from it
     would be false.**
   - **One `page_members` row**: `on(Page::Demo, map_node, false)`, with a
     comment naming the two mutation-found majors as the reason the row is
     written where a reader sees it.
   - **One row in `tests::undrawn_leaf_exemptions`**, beside the background and
     the tab bar — **required, not optional**, for the reason
     § *Where the map is covered instead, and the one place it must be added*
     gives: `tests::assert_every_drawn_leaf_is_named_or_excused` compares the drawn
     set against that list and the map is in neither of the two sets otherwise,
     **so three tests that are not about the map go red without this row.**
   - **One arm in `Demo::frame`'s paint walk**, in the existing six-widget arm at
     `main.rs:7620` rather than a new one — the arm is
     `if handle == self.gauge.handle() || … || self.image.handle() || …`, and
     the map is the seventh such widget. **On the six gallery pages it records
     nothing**, because `Demo::sync_page_visibility` (`main.rs:5964`) writes
     `set_visible(false)` on every row whose page is not the one on show, and
     `Demo::empty_off_page_paint` (`main.rs:6024`) empties the recorded commands
     of exactly those nodes. **Both already exist and neither is touched**; a
     second mechanism would be a second answer to one question.

5. **The tests: six in `ui/src/ui_demo/src/main.rs`'s `mod tests`, named.**
   Every one drives the public surface, and **none needs the image file** —
   `AGENTS.md` § *Rust* forbids a test that needs a filesystem, which is the
   whole reason the stand-in exists.

   - **`the_seventh_page_is_named_demo_and_the_default_did_not_move`** —
     `Page::DEFAULT == Page::Pads`, `Page::ALL.len() == 7`,
     `Page::ALL[6] == Page::Demo`, and `Page::Demo.name() == "demo"`.
   - **`the_seventh_tab_button_lays_out_inside_the_window`** — the bar is laid
     out and the last button's right edge is at or before `WINDOW.width`.
     **"Seven buttons fit" is a number, not an estimate**, and this is the
     number. **It reads `layout().rect()` and adds the width rather than calling
     `far_corner()`**: that method is on `layout::Rect` (`layout.rs:159`) and
     `Demo::node_rect` hands back a `paint::Rect`, which has no corners.
   - **`the_map_node_is_full_bleed_below_the_tab_bar`** — the laid-out rect is
     `(0, CONTENT_TOP)` and is `WINDOW.width` × `WINDOW.height - CONTENT_TOP`.
     **This is the test for the failure in § *What does not detect a change to
     the map* that no existing gate can see**: a node at `y: 0` would be behind
     the tab bar and would pass everything else in this list.
   - **`the_demo_page_draws_the_map_and_the_gallery_pages_do_not`** — the paint
     gate over the map node on all seven pages: **at least one command on
     `Page::Demo`, and none on each of the six.** **Not *more than one*, which is
     what the superseded task's version of this sentence said:** `Image::paint`
     records **exactly one** command and nothing else (`widgets/image.rs:1000`
     asserts it), so the number has to be one. This is the criterion that the map
     does not leak.
   - **`the_map_is_an_image_at_the_node_s_own_rect_with_the_cover_fit`** — three
     assertions:
     1. the map node records **exactly one** command and it is a
        `DrawCommand::Image`;
     2. `demo.map.fit() == ImageFit::Cover` — **`Image::fit()`
        (`widgets/image.rs:680`) is a public getter, so the fit is read as well
        as checked, and `ImageFit` derives `PartialEq`**;
     3. that command's `rect` is the node's whole laid-out rect, **and its `uv`
        is a centred band narrower than the full texture** — `u0 > 0.0` and
        `u1 < 1.0`, by about **3.4 % each side**.

     **Assertion 3 is the one that says `Cover` rather than `Fill`, and it only
     works because the operator's image is a different shape from the node.**
     `map_demo.png` is `1359 × 970` and the node is `1280 × 956`, so `Cover`
     keeps the source's whole height and crops its sides to `1280 · (970/1359) ·
     …` — the kept fraction across is `bounds_aspect / source_aspect` =
     `1.3389 / 1.4010` = **0.9556**, and `sampled_uv`'s own doc gives exactly
     that rule (*"a source **relatively wider** than the bounds keeps its whole
     height and loses width"*). Under `Fill` the whole texture is sampled and the
     4.4 % is stretched instead: **a different picture, and `u0 == 0.0` is how a
     test sees it.**

     **Why the shape difference is recorded rather than left implicit.** At
     **matching** shapes `Cover` and `Fill` are *indistinguishable* —
     `destination_rect` gives both the node's rect (`widgets/image.rs:1151`) and
     `sampled_uv` says *"matching shapes keep all of it"* — **so had the image
     been `1280 × 956`, assertion 3 would have failed on correct code and
     `Fill` would have passed unnoticed.** `MAP_IMAGE_SIZE` is therefore not
     only a stand-in's dimensions; **it is the number that makes the fit
     testable**, and requirement 1 says so where the constant is written.
   - **`the_map_node_is_not_in_placed_handles_and_the_two_counts_did_not_move`**
     — `placed_handles().len()` and
     `expected_placed_rect_names(demo.model_status.is_some()).len()` are both
     **unchanged from their pre-task values** (**29 and 30**,
     `main.rs:13861`), and the map's handle is in neither. **A count is the checkable form of "the map was not added to a list
     its rect overlaps"**, and the mutation a reviewer should make is to add the
     row and watch this fail.

   **These six are additive** — `ui_demo`'s 236 becomes **242** — and **no
   existing test is deleted, renamed or weakened.** The three `TASK_UI_PRIM_43`
   test *names* that carry a page count — `tab_walks_the_six_buttons_before_the_
   pages_own_controls`, `the_tab_bar_is_still_first_in_the_route_chain_for_each_of_
   its_six_buttons` and `the_demo_bar_has_one_tab_per_page_and_no_tab_beyond_the_
   six` — **were renamed to seven on 2026-10-09 in that task file, not in any
   binary**, so no test in `ui/src` is renamed by this task.

6. **`ui/Cargo.toml` and `ui/Cargo.lock` are unchanged, and the manifest rule is
   the reason.** The approved direct dependencies are `sdl3 0.20`, `glow 0.18`
   and `freetype-rs 0.38`, and **a background image is one
   `Renderer::load_texture` call on a path — none of them is needed and no
   fourth is earned.** Edition 2021 and `rust-version = "1.85"` hold; no stdlib
   feature newer than the floor is used.

7. **`doc/ui/IMPLEMENTATION_STATE_DEMO.md` is created**, with the task-table row
   for **01** (file, review count, waivers-or-none, frame rate), a
   § *Task DEMO 01* record carrying **the map approach and its honest limits in
   the record's own register** — **that the map is a picture, that it does not
   pan, zoom or rotate, that nothing is drawn on it, and that gap `#1` is not
   closed** — **the licence acceptance**, **the waiver of the capture criterion
   with its reason**, **the test count before and after**, **the frame rate for
   all seven pages**, and **what the handoff does not claim.**

   **The waiver is the one thing in this requirement that must be written down
   rather than discovered.** § *Acceptance Criteria*'s capture bullet cannot be
   met from a fresh clone, because the image is not in the repository
   (§ *The asset is not in this repository*); `task-sequence.md` § *Gates* says
   *"An unverifiable acceptance criterion may be waived with a reason. A
   verifiable one may not, and a waiver outliving its reason becomes a fiction."*
   **The reason is that the file is not in the tree, and that reason outlives the
   waiver exactly as long as that is true** — so **if the image is ever committed,
   this waiver must be withdrawn and the criterion met**, and requirement 3 is
   what would change.

   **No other file changes at implementation time.** `DEMO_APPLICATION.md`,
   `TASK_UI_PRIM_42.md`, `TASK_UI_PRIM_43.md`, `TASK_UI_DEMO_02.md`,
   `TASK_UI_DEMO_03.md`, `IMPLEMENTATION_STATE.md` and
   `.ai/workflows/task-sequence.md` were amended on 2026-10-09 **with this file**
   — § *Scope* says which and why — and **this task does not amend them again**.
   A second amendment about the same seventh page would be a second place to look
   for it.

## Testing

**Every test is named above, every one is legal under `AGENTS.md` § *Rust*, and
every one asserts a contract rather than an implementation detail.**

**The measured baseline, re-measured on the tree this file is written against,
is 2058 passing and 1 ignored**, and the six tests above bring it to **2064
passing / 2065 registered** (§ *What the tests already know, and the baseline*
carries the split between passing and registered), each pasted in the handoff.

**With one existing test changed, and it is named rather than counted as
additive.** Requirement 4 adds a row to `tests::undrawn_leaf_exemptions`, which
`tests::assert_every_drawn_leaf_is_named_or_excused` compares against — **so the
data that test asserts changes, while the assertion itself does not.** **Nothing
is deleted, renamed or weakened**; a test's fixture list grows by one row is not a
weakened assertion, and the alternative — leaving it out — is three red tests
(§ *Where the map is covered instead, and the one place it must be added*).

**Two deliberate breaks the handoff must paste**, because `developer.md`
§ Phase 3 says *"A test that has never failed is not a test"*:

1. **Change the map's fit to `ImageFit::Fill`** and re-run
   `the_map_is_an_image_at_the_node_s_own_rect_with_the_cover_fit` — **assertion 3
   fails on `uv`** (`u0` becomes `0.0` instead of `≈ 0.034`), **and the capture
   shows the whole 1359 px of the map squeezed into 1280, a 4.4 % horizontal
   stretch.** **This is the break with a visible consequence**, which is the rule
   *a fill rule cannot be repaired downstream* is about.

   **`Fill` rather than `Contain`, and the reason is the image's shape.** With a
   source *wider* than its node, `Contain` shrinks the quad to fit the whole
   image inside the box (`destination_rect`, `widgets/image.rs:1157`) **and
   letterboxes it**, so `Contain` is caught by the `rect` half of assertion 3 and
   is a weaker break: **it proves `Contain` is wrong, not that `Cover` is right.**
   `Fill` is caught by the `uv` half, **which is the assertion that separates
   `Fill` from `Cover` and from nothing else** — the two fits that agree on the
   quad and disagree on the picture. **A break that proves one fit is not another
   is not a test of the one chosen.**

   **The second break below is the one whose failure is positional rather than
   optical.**
2. **Remove the map node's `set_position`** and re-run
   `the_map_node_is_full_bleed_below_the_tab_bar` — it fails on `y`, **and the
   capture shows the map tucked under the tab bar**, which reads as a styling
   choice rather than as a defect. **The test is what tells the two apart**, and
   it is the only thing in this task that can: no other gate measures the map's
   rect (§ *What does not detect a change to the map*).

## Acceptance Criteria

- [ ] **`ui_core` has no diff and the map sweep is still empty.**
      `git diff --stat` names **no** file under `ui/src/ui_core/src/`, and
      `grep -rE 'map_widget|MapView|mercator|latitude|longitude|geojson'
      ui/src/ui_core/src` returns **nothing** — the `-c` form prints a `path:0`
      line for every file in the tree, which is 36 lines of noise a reviewer has
      to read past. **The mechanism is the
      point**: `DEMO_APPLICATION.md` § *Task structure* decided *"the map is the
      demo's own asset and no library owns it"*, and a criterion that only
      asserted "a map shows on the demo page" would pass on a diff that closed
      gap `#1` in the crate instead.

- [ ] **`Page::ALL` is `[Page; 7]`, `demo` is the seventh spelling, and the
      default did not move.** `grep -n 'const ALL: \[Page; ' ui/src/ui_demo/src/main.rs`
      reads **`const ALL: [Page; 7]`**; `grep -c 'Page::Demo => "demo"'` finds
      **exactly one** occurrence, inside `fn name`;
      `./target/release/ui_demo --tab=demo` opens the demo page and
      `--tab=dta` refuses naming **seven** pages; `--help` prints seven; and
      `./target/release/ui_demo` with no argument opens **`pads`**.
      **`the_seventh_page_is_named_demo_and_the_default_did_not_move` asserts all
      four.**

- [ ] **The image is ignored at every path it might sit, and no byte of it is in
      git.** `git check-ignore -v img/map_demo.png` and
      `git check-ignore -v ui/src/ui_demo/assets/img/map_demo.png` **both** name a
      `.gitignore` line; `git ls-files` finds it at neither; and **with the
      operator's file in place, `git status --porcelain` is empty.** That last one
      is the criterion that would have caught this file being staged by accident,
      and it is checked with the file present rather than absent.

      **Measured before the `.gitignore` line existed, 2026-10-09:**
      `git check-ignore -v img/map_demo.png` exits **1** and `git status` reports
      **`?? img/`** — **the file was one `git add -A` from being committed.** The
      handoff names the file by its path and nothing else: **no bytes of it are
      in this repository and none may be.**

- [ ] **The page is on screen, on the demo page only, and the capture says
      which picture it is of.** Release build, `--tab=demo`, captured with the
      stock method of `IMPLEMENTATION_STATE.md` § *Verifying a change that draws
      — the capture method* verbatim — window id from
      `DISPLAY=:0 xwininfo -root -tree | grep '"roados ui_demo"'` and
      `DISPLAY=:0 magick import -window <id>` — with `pgrep -a -x ui_demo` in the
      same call as each capture.

      **The capture is read for four things, and the fourth is a waiver.** Three
      are evidence: the **seven-button tab bar** with `demo` the seventh; **the
      map node's area below it**; and **the six gallery pages unchanged**, by
      `magick compare -metric AE before.png after.png null:` per page giving
      **AE 0 outside `y < TAB_BAR_HEIGHT` and the fps readout's band** — not AE
      0, because the seventh tab button is on every page.

      **The fourth is: the map image itself is not in this repository, so this
      task cannot produce a capture of the operator's map.** The capture is taken
      **with the stand-in**, which proves the page, the layout and the paint
      order and **proves nothing about the picture**. **That is recorded as a
      waiver with its reason, per `task-sequence.md` § *Gates* (*"An
      unverifiable acceptance criterion may be waived with a reason"*), and the
      operator's own capture of the supplied image is what closes it.** Stating
      this here is the point: a handoff that reported the stand-in as "the map is
      on screen" would be reporting a test fixture as evidence.

- [ ] **`cargo test --all-features` is green with every named test present.**
      From `ui/`: `cargo fmt --check`;
      `cargo build --all-targets --all-features`;
      `cargo clippy --all-targets --all-features -- -D warnings`;
      `cargo test --all-features` with **each of the six tests named in
      requirement 5 present by name**, against the baseline of **2058**, so the
      total is **2064 or higher with nothing removed**; `cargo doc --no-deps`
      clean; and `cargo audit` **recorded as not installed on this host, not
      passed**. `layout_walk_cost` is **still `#[ignore]`d and unmodified**.
      **And the handoff lists the two deliberate breaks with their failure
      output.**

- [ ] **The frame rate is measured on all seven pages and reported with the line
      pasted.** `.ai/tools/fps-check.sh 10 55` on the default page — **the only
      thing the script can do**, since it takes `seconds` then `floor`, builds
      release and runs `./target/release/ui_demo` with **no arguments and no
      page** — and then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo
      --tab=<page>` for **each of the seven**. **Every page above the floor of
      55**; the six gallery pages inside the recorded 61.1–63.9 band, **because
      nothing this task does runs on them**; and the `demo` page's rate is the
      number this task produces and is reported **whatever it is**. **This is the
      gate in `task-sequence.md` § *Gates* (*"No unmeasured run of the demo"*)**,
      and a still of a 4 fps application is pixel-identical to a still of a
      60 fps one.

- [ ] **Nothing leaked in, and the dependency rule holds.** `git diff --stat`
      names **no** file under `ui/src/ui_core/src/` and **no** file under
      `ui/src/ui_demo/assets/` **and none under `/img/`**. **`ui/Cargo.toml` and
      `ui/Cargo.lock` are unchanged**. `grep -c unsafe ui/src/ui_demo/src/main.rs` is **2** — both in
      doc comments — **and `git diff` adds none**, and `grep -n 'unwrap(\|expect(\|panic!\|unimplemented!\|todo!'`
      over the new functions returns **nothing**. **No `pub` item is added to
      `ui_core`** — the new code is in a binary, so `pub` there is crate-visible
      and not a semver question. **`TASK_UI_PRIM_39`'s pipeline is not touched
      and not started**; nothing in `ui/src` calls it.

- [ ] **The documents carry the decision, and none is closed.**
      **`doc/ui/backlog/TASK_UI_DEMO_01.md` exists with its superseded note and
      its § *Decisions taken instead*, and `doc/ui/backlog/README.md` says what
      the directory is.** **Row `#1` of `DEMO_APPLICATION.md` § *Library gaps*
      gains a dated note that the seventh page exists and that its map is an
      image, and the row is neither deleted nor marked closed** — its Blocks entry stays
      *"Map/navigation screen"* because **no navigation screen is built**. **§
      *Open questions* item 2 is amended and stays on the list**, carrying the
      decision, the licence acceptance, the reason the image is not committed,
      and the sentence that *"the decision is this task's; the question is still
      the operator's"*. **Item 7 is answered and removed**, with the answer
      `demo` and the date. **`TASK_UI_PRIM_42.md` § *Sub-task 42.2` and
      `TASK_UI_PRIM_43.md` § *Sub-task 43.2`: every count that names the demo's
      six pages becomes seven**, dated and attributed, and the three of 43.2's
      test names that carry the count are renamed to match — `git grep -n 'six'
      doc/ui/TASK_UI_PRIM_4[23].md` is pasted so every line the amendment touched
      is listed. **`task-sequence.md` § *Scope* names `doc/ui/TASK_UI_DEMO_*.md`
      and its state file, and no other `.ai/` file has a diff** —
      `git diff --stat -- .ai/` names one line.

- [ ] **What the handoff does not claim, in those words.** It states that **the
      map is a picture and not a map widget**; that **it does not pan, does not
      zoom, does not rotate and carries no route, marker or POI**; that **gap
      `#1` is not closed**; that **the image is not in the repository, so the
      demo page's capture in this handoff shows the stand-in and not the map**;
      that **the image's licence was accepted as a local-use risk by the
      operator on 2026-10-09 and is void if the image is ever distributed**; and
      that **`check_assets.py`'s denylist does not cover `assets/img/`, because
      it skips `.png` contents**.

## Out of Scope

- **Nothing on top of the map, and almost nothing on the page.** No car marker,
  no route, no POI, no compass, no scale bar, no attribution line, no zoom
  controls — and **no chrome, no car-status pane, no indicator column and no card
  carousel either**, because the page this task adds is the infotainment screen
  and those four are its content. **`TASK_UI_DEMO_02` through `_05` are those
  four panels**, one each, and **this task's page is the thing they all go on**
  — which is why the page is created here and not there. **The honest summary is
  that the seventh page ships with a picture on it and stays that way until task
  02 lands**, and `Page::Demo`'s doc says so in the same words rather than
  describing a finished screen.
  `TASK_UI_DEMO_05` already records that road markings and stop lights are map
  layers and are not built — § *The `[C]` sentence, and what survives it* says
  *"**They are map layers and there is no map**"*, and its § *Out of Scope* says
  *"No map, and therefore no road markings, no stop lights and no objects"* — and
  this task does not change either.

- **No coordinates, no camera, no projection, no pan, no zoom.** The picture is
  a rectangle with a texture in it. **The superseded task's § *`Camera` is data,
  not an abstraction* and the eight bullets of its § *What this approach cannot
  do* do not apply here** — and
  neither does its reversal path: **there is no `World` and no seam.** What a
  later task needs to reach over this one is the closing paragraph of §
  *Decisions taken instead* in `doc/ui/backlog/TASK_UI_DEMO_01.md`.

- **No raster tile layer, no zoom levels, no pan, no tile cache.** **The image
  is one file, loaded once, uploaded once, and drawn with the node's rect.**
  There is no camera to move, so there is nothing to re-derive and **no
  minification aliasing to hide** — `GL_LINEAR` with no mipmaps is a real
  limitation for a map that zooms and is not one for a picture drawn at the size
  it was captured at.

- **No new `ui_core` API, of any kind.** No `DrawCommand` variant, no `Painter`
  method, no `ThemeToken`, no `LayoutMode`, no pixels-to-texture entry point.
  **`Painter::image` and `ImageFit::Cover` already exist and are what a full-
  bleed textured rect needs** — which is the fact that makes this task a day
  rather than the pipeline change the superseded task refused to make.

- **The licence question is recorded, not settled.** § *The licence, and what
  was accepted* states what the terms say and what the operator accepted.
  **`DEMO_APPLICATION.md` § *Open questions* item 2 stays on the list** and item
  3 — **the Tesla mark** — was already open and stays open. **This task does not
  answer either**, and the fact that the image is not committed is the reason
  the first of them is a bounded risk rather than a defect in a release.

- **The asset guard is not widened, and that is a known gap rather than an
  oversight.** `tools/asset-pipeline/check_assets.py`'s `denylist_hits` walks
  the whole assets tree but **skips `.png` contents**
  (`check_assets.py:197-203`) and `check_icons` is scoped to `icons/`
  (`check_assets.py:220-240`), so **`assets/img/` is not covered by any of the
  six checks.** Widening the walk to name the directory is **one line of
  Python**, but it is a change to a tool `TASK_UI_PRIM_39` owns and delivered,
  its record says *"not reviewed"*, and **this task is not the task that should
  review it.** It is named here so the next reader knows it was seen.

- **`MAP_IMAGE_SIZE` is written down, not read from the file**, for the reason
  `ASSET_SIZE` (`main.rs:2315`) is: *"a stand-in has to be the shape of the
  thing it stands in for"* — **and `ImageSource::of` reads the real size off the
  cache anyway**, so a stand-in built at a different shape still lays out; what
  the constant buys is that **the geometry a test asserts is the geometry the
  operator's file produces.** It is measured (`1359 × 970`), and **if the image is
  ever replaced with one of a different shape, the constant and
  `the_map_is_an_image_at_the_node_s_own_rect_with_the_cover_fit`'s `uv`
  assertion change together** — which is the arrangement, because a constant that
  silently stops describing the file is the thing this task is avoiding.

  **What is out of scope is a `MAP_IMAGE_SIZE` derived from a file read.** The
  image is not in the repository, so a test that read it would be a test that
  cannot run here (`AGENTS.md` § *Rust*), and the constant is the honest
  alternative. **The operator's own capture is what confirms the constant still
  describes the file** — and that is one of the things the waived capture
  criterion is waived *for*.

- **No second fit, no cycling key, no fit readout.** `F` cycles the gallery
  image's four fits and **the map is not in that cycle** — a background that
  letterboxes when a key is pressed is a defect, not a demonstration.

- **No `ui/src/ui_demo/assets/img/README.md` and no placeholder file.** The
  directory is not in the repository and nothing describes it there. **What the
  directory is and why it is empty is this task file's job**, and a README in a
  directory `.gitignore` excludes would be a file the next agent cannot find.

- **Found in the tree and deliberately not fixed.** `ui/src/ui_demo/src/main.rs`
  carries a comment on the tab bar's construction that records the **withdrawn**
  2026-10-03 decision, which `DEMO_APPLICATION.md` § *Relationship to task 24*
  lists as *"**known-stale** and is left for a code task"*.
  **`TASK_UI_PRIM_43` rewrites that whole block when it lands** and
  `developer.md` § Phase 2 refuses drive-by cleanups, so this task does not fix
  it. **Recorded so the next reader knows it was seen and not missed.**