# TASK_UI_DEMO_02: The Chrome — a Top Status Bar, a Bottom Dock, and a Persistent Car-Status Pane

## Goal

Fill the Tesla tab with its **persistent chrome**: a **top status bar** across the
top of the map, a **bottom dock** across the bottom, and the **car-status pane** as
a **persistent region** of the map screen rather than a panel over it. Three
regions, laid out at absolute rectangles, drawn over the map `TASK_UI_DEMO_01`
delivered, in `ui_demo`.

> **Amended 2026-10-09, when `TASK_UI_DEMO_01` was rewritten.** That task no
> longer draws a procedural vector map: its map is now **one image file**, drawn
> full-bleed as a background, with **nothing on top of it**, and it lives in
> `main.rs` rather than in a `map.rs`. **Four of this task's references to it
> below are amended accordingly** — the `MapSurface::paint` per-frame cost, the
> `map.rs` separability precedent, the *large non-empty vector* control, and the
> *car in the pane is the map's marker* line, which is **now false as written**
> and is corrected in § *Out of Scope*. **The chrome this task builds is not
> affected by any of it**: a translucent bar and dock go over a picture exactly as
> they went over a vector scene.

**The demo tab and `--tab=demo` already exist** — they are `TASK_UI_DEMO_01`'s
deliverable, and this task does not add them, does not rename them and does not
change `Page::DEFAULT`. Its first acceptance criterion **verifies** all three
rather than assuming them, and that criterion is also the check that this task did
not make a second page.

**The two declared dependencies and what happens without them are the substance
of this file, not a footnote.** `TASK_UI_PRIM_41` (`Painter::backdrop`) is **hard
for the translucent half only**, and the fallback is an **opaque** chrome recorded
as a deviation, because `Color` is premultiplied and an alpha-only fill is a flat
wash rather than a frosted one. `TASK_UI_PRIM_43` (`TabBar`) is **not required at
all**, and § *The dock is not a `TabBar`* gives the reason from 43's own design
section: the dock's five items are **actions**, not screens, and a bar that owns
*"which one is selected"* has nothing to own here. **Neither dependency blocks
this task, and both are declared with the fallback written down rather than left
to the implementer.**

## Context

### What the evidence says each region is, cited by section and row

Every layout claim below is from `DEMO_APPLICATION.md` and is cited by its section
name. **The evidence classes are `[A]` manual, `[A]` photo, `[B]` release notes and
`[C]` snippet-level**, per § *Sources and how to read this section*, and § *Sources*
records that **the corpus is unreachable from this host** — `tesla.com` returns
403 and the `rollout-tesla.com` mirror answers a real GUID with Akamai's *Access
Denied*, so **every `[C]` claim below is a claim a task must re-verify before
treating it as specified.**

**§ *Screens* — the structural fact, and it is a table with seven rows.** Tesla
*"does not have 'screens' in the sense this document originally assumed. It has
**one persistent map screen**, and every other surface is a **panel drawn over the
map**."* Three rows carry this task:

| row | what it says | what it means here |
|---|---|---|
| **Map** | *base screen*, over-the-map *—*, dismissed by *—* | the map image from task 01, painted first, never occluded by a chrome node's background at alpha 0 |
| **Car-status pane** | *persistent region of the map screen*, *"part of it"*, dismissed by *"never; resize drag only"* | **the pane is a region of the demo page, not a panel over it** — so it has no open/closed state and no dismissal, and this task builds neither |
| **— (the other five rows)** | *panel*, *popup*, *strip ↔ panel*, *card*, *bottom slot*, all *"yes"* for over-the-map | **all five are later `TASK_UI_DEMO_n` tasks.** § *Screens* is also the section that records *"**Every dismissal is a drag, and the direction is part of the semantics**"* — a drag-to-dismiss gesture, which is `L4`'s half and `TASK_UI_PRIM_40`'s. **This task builds no gesture at all**, and says so |

**§ *The persistent chrome* § *Top bar* — the inventory, and it is state-driven.**
The section's own sentence is the one that decides the structure: *"**The bar is
not a fixed icon set; it is populated by vehicle state.**"* The inventory table
lists **thirteen** rows of which **three are `always`** — the open/closed padlock,
the clock, and the front passenger airbag status badge — and **ten are
conditional**, including *"person + profile name — Displays… only when Model 3 is
parked"* and *"Sentry Mode — Available when Model 3 is parked"*. **This task
builds the three always-on items plus the two parked-only items, and the other
eight are absent.** It does **not** build the state machine that would add and
remove them, and § *Out of Scope* says so.

The photo reading is `[A]`-observational and its order is a real constraint:

```
PRND · [battery 70%]  ………  [open padlock] [👤 Guest] [🔴]  ………  [2:37 pm] [☀ 65°F]  ………  [PASSENGER AIRBAG OFF]
```

**Two decisions are inside that line, and one of them is a warning.**

- **`PRND · battery%` sits at the extreme left, *outside the status-bar band*** —
  the section says so in its own words: *"it belongs to the car-status cluster,
  not to the chrome."* **So the status bar has no `PRND` and no battery, and those
  two readouts go at the top of the car-status pane.** Building them into the bar
  would put a row the evidence explicitly excludes there.
- **The middle is deliberately empty**, and the four visual groups are
  `PRND · battery` · `padlock Guest Sentry` · `clock ambient` · `airbag`.
  **The bar this task builds is three children, not four**, for the reason
  § *Three groups and the alignment that cannot reach the photograph* gives.

**§ *The persistent chrome* § *Bottom dock* — icon-only, and three behaviours
that make it composite.** The photo reading is `[A]`: *"Icon-only, flat black bar,
no labels, **no active-state highlight**"* `[A]` photo `01`. The manual's
authoritative slot list is **13 regions** `[A]` §Touchscreen, in order, of which
**regions 6, 9, 11 and 12 are conditional**, and *"this vehicle shows 6 and not
12, i.e. temperature Split is off."* § *Design principles* records the correction
that this task depends on: *"The media **player surface** … is **not** a dock
item: it is a surface drawn over the map … **The media *shortcut* is a dock
item**"* — and § *Bottom dock* carries the enumeration: region 7 is the *"media
player shortcut"*.

**This task builds five dock slots**, chosen from the 13 regions and named here so
the choice is a decision rather than a default: **Controls** (region 5),
**climate controls (driver)** (region 6, conditional, and this vehicle shows it),
**My Apps** (region 9, conditional), **App Launcher** (region 10), and
**Volume Control** (region 13). **The other eight regions are not built**, and the
three composite behaviours § *Bottom dock* records — recents' width as a function
of My Apps occupancy, the seat-heater override, and destructive overflow — are
**composite widgets 10 and 15** and a later task.

**§ *The car-status pane* — left ~40%, and a five-item vertical structure.** The
section's own sketch puts *"Left ~40% in photo `02`, resizable"*, and the ASCII
block in that section shows the stack from the top: the **drive-mode strip** at the
left screen edge on *"its own layer"*, the **indicator column**, the **map**, and
the **card carousel** at the foot with a 3-dot pager. Two facts from the section
decide what 02 builds:

- **The drive-mode strip is *"Not chrome."*** The section says so in its own
  heading and body: *"a **vertical dotted-grid track** inset against the left
  screen edge"*, with *"a car silhouette, a grey `↑`, a grey `↓`, a bold `P`, and
  the word `HOLD`"* — and it closes with *"**The whole gesture model is `[C]`** and
  rests on that one snippet; it must be re-verified against an opened page before a
  `TASK_UI_DEMO_n` task treats it as specified."* **So the strip's four targets are
  built as read-outs and no gesture is attached to any of them.**
- **The indicator column is *"a severity-ranked list, not a status strip"***, and
  the section enumerates ~20 conditions across **five colours** — red, amber, green,
  blue, grey — with **three distinguished by timing rather than colour** and one
  **latching**. **This task builds five rows, one per colour class, in the section's
  own order.** It does not build the ~20 conditions, the blink semantics or the
  latch, and the latch in particular has an **externally-triggered reset**
  (*"drive over 15 mph (25 km/h) for a short amount of time"*) that is a state
  machine with a second input.

### The four regions, their rectangles, and why the numbers are first-principles

**Tesla publishes no design tokens at all.** `DEMO_APPLICATION.md` § *Could not
verify* answers *"Exact colours, spacing, radii"* with *"**Tesla publishes no
design tokens at all.** Every measurement in the demo is a first-principles
choice, not a transcribed value"*, and § *Asset requirements* repeats it. **So
every rectangle below is a first-principles number, named, in one place, with the
reason it is that number** — and the one number that is *not* first-principles is
the pane's **~40 %**, which the photograph gives.

| region | origin | size | the number is |
|---|---|---|---|
| map (task 01, unchanged) | `(0, CONTENT_TOP)` | `WINDOW.width × WINDOW.height − CONTENT_TOP` | derived from `CONTENT_TOP`, which is `TAB_BAR_HEIGHT` |
| **status bar** | `(0, CONTENT_TOP)` | `WINDOW.width × STATUS_BAR_HEIGHT` | `STATUS_BAR_HEIGHT = 48.0` — the smallest height that holds one line of the theme's `FontSizeMd` with `STATUS_BAR_PADDING` above and below |
| **car-status pane** | `(0, CONTENT_TOP + STATUS_BAR_HEIGHT)` | `WINDOW.width × CAR_STATUS_PANE_FRACTION` wide, down to the dock's top | **`CAR_STATUS_PANE_FRACTION = 0.40`** — the photograph's *"Left ~40%"* |
| **drive-mode strip** | inside the pane, inset `PANE_INSET` | a column of four rows | derived from `SpacingLg` between four targets |
| **card carousel** | inside the pane, at its foot | `WINDOW.width × CARDS_HEIGHT`, three cards | `CARDS_HEIGHT = 96.0` |
| **pager** | below the card row | three dots of `PAGER_DOT_RADIUS` | the photograph's *3-dot pager*, which § *Could not verify* says is *"**unverified**"* for count — **and 02 builds three because the photo `01` shows three**, recorded as observed |
| **bottom dock** | `(0, WINDOW.height − DOCK_HEIGHT)` | `WINDOW.width × DOCK_HEIGHT` | `DOCK_HEIGHT = 96.0` |

**The three chrome rectangles are pairwise disjoint, and that is a property rather
than a hope.** `64 ≤ 64 < 112 ≤ 924 < 1020`, so the status bar, the pane and the
dock cannot overlap, and `the_chrome_nodes_do_not_overlap_each_other` is the test
that holds it. **All three are drawn over the map on purpose** — they are the
"panel over the map" model of § *Screens* applied to the regions that are *always*
there, and `the_chrome_nodes_do_not_overlap_each_other` is deliberately **not**
`no_two_placed_rects_overlap`: the chrome overlaps the map by construction, which
is why neither list is in `Demo::placed_handles` (§ *Four things this task must NOT
do*).

### The three groups and the alignment that cannot reach the photograph

**The status bar is three children in a `row()` with
`MainAxisAlignment::SpaceBetween`, and that does not put the airbag where the
photograph puts it.** Three children and `SpaceBetween` divide the free space into
**two** equal gaps, so the left group is at `x = 0`, the clock/ambient group at the
**midpoint**, and the airbag badge at the **right edge**. The photograph reads
`PRND · battery` · `padlock Guest Sentry` · `clock ambient` · `airbag` with
`……` between the sentry and the clock and **another `……` before the airbag**, so
the airbag sits at the **right edge** and the clock group at about **two thirds**.

**Reaching that needs two independent gaps, and there is one.** `FlexConfig` in
`ui/src/ui_core/src/layout.rs` has **three private fields** —
`main_axis_alignment`, `cross_axis_alignment` and `spacing` — and
`MainAxisAlignment` has **six** variants, **none of which puts the middle child at
two thirds of a three-child row**: `Start`, `End`, `Center`, `SpaceBetween`,
`SpaceAround` and `SpaceEvenly`. **Getting two unequal gaps out of three children
needs a second gap value**, which is gap `L7`'s *"no cross-axis gap"* clause and
therefore `TASK_UI_PRIM_48`'s deliverable. **The honest statement is that the bar's
three-group layout is correct in structure and differs from photograph `01` in the
midpoint's position**, and the alternative — hard-coding one child's `position` —
is refused because `developer.md` § *Phase 2* refuses an invented layout rule in a
pass every node inherits, on the same argument `TASK_UI_PRIM_52` § *`columns` stays
`usize`* makes about `MIN_GRID_COLUMN_WIDTH`: **a constant with no source is the
failure § *Could not verify* records.**

### `Color` is premultiplied, and a translucent chrome needs arithmetic this repository has exactly once

**`ui/src/ui_core/src/property.rs` documents `Color` as *"A minimal RGBA color with
premultiplied alpha"***, and every `DrawCommand` that carries a colour carries a
premultiplied one. **So a translucent surface is `(c · a) / 255` per channel, done
in `u16` so the product is not truncated before the divide** — and that arithmetic
already exists in this repository, in exactly one place: `fn premultiply` in
`ui/src/ui_core/src/texture.rs`, whose doc says *"(200 * 200) / 255 is 156.86, and
a `u8` multiply would have said 156 for every value"*, and which does the
conversion in `u16` for that reason.

**So this task writes one private `fn premultiplied(color: Color, alpha: u8) ->
Color` in `chrome.rs`, with that arithmetic in its doc and that reasoning cited**,
and
`premultiplied_multiplies_each_channel_by_the_alpha_in_u16` asserts it **by value**
— `(200, 200, 200, 128) → (100, 100, 100, 128)` — because **a test that asserts
only that the channels got *smaller* cannot catch a `u8` multiply**, and
**an assertion that cannot fail** is exactly that trap.

**Why this is in the task rather than left to the implementer:** a card drawn at
`Surface` with `a = 200` and **not** premultiplied composites as if it were `(245 ·
200, 245 · 200, 245 · 200, 200)` under `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`
— **far too bright**, and it is the same class as
a strength clamped to `0..=1` and used directly as an effect's
size: **a number that looks like the right kind of number and is not.**

### The two declared dependencies, and what happens without them

**`TASK_UI_PRIM_41` (`L1`) — hard for the translucent half, and the fallback is
opaque.** What 41 delivers, by name: `pub struct ColourTarget` beside
`ShadowTarget` in `render/target.rs`; **`DrawCommand::Backdrop { rect, mode, tint }`**
as the eleventh `DrawCommand` variant; **`pub fn Painter::backdrop(&mut self, rect:
Rect, mode: BackdropMode, tint: Color)`**; **`pub enum BackdropMode { Sharp, Blur(f32) }`**
where `Blur`'s payload is *"the Gaussian's standard deviation in pixels"*; and
`Segment { .., backdrop: Some(..) }` from `Batcher::submit_order`. **41's own §
*Out of Scope* records the honest size of that**: its `ColourTarget` is
**window-sized**, not rect-scoped, because `L1`(c) — *"Rect-scoped capture"* — is
the thing 41 does **not** close, and *"capturing full-window RGBA and blurring it
every frame is a different order of cost, and the target is aarch64."*

**So the decision is: `TASK_UI_DEMO_02` uses `Painter::backdrop` if and only if 41
has landed, with `BackdropMode::Blur(BACKDROP_SIGMA)` and the tint being
`premultiplied(Surface, CHROME_ALPHA)`; and if 41 has not landed, the chrome is an
opaque `RoundedRect` in the theme's `Surface` at alpha 255.**

**Why opaque and not "the same alpha without the blur".** Because that is the one
answer that would be dishonest. A flat `Surface` wash over a map is a **different
picture** from a blurred one, and `DEMO_APPLICATION.md` § *Design principles*
records the added principle *"**chrome is translucent over a live scene**"* and
calls it *"the single structural fact the original principles missed"*. **Shipping
an alpha wash and describing it as the translucent chrome would be a false support
statement**, which is the first thing `reviewer.md` § Phase 2 asks about.
**So the fallback is opaque, and it is recorded as a deviation with a reason — not
as a waiver, because `task-sequence.md` § *Gates* says a verifiable criterion may
not be waived.**

**Two further costs of 41 that the handoff must state, because they are arithmetic
and not opinion.** One `Backdrop` is a **window-sized `GL_RGBA8` capture plus two
blur passes**, so **five chrome nodes is five captures a frame** — or, better,
**one capture and five composites**, which is `L1`(c)'s missing rect scoping and
therefore **not available**. **And the target is aarch64** — one of the two
architectures `AGENTS.md` names — so this is a bandwidth decision on the weaker of
the two, and **the frame-rate criterion below is where it shows up if it shows up
at all.**

**`TASK_UI_PRIM_43` (`#7`) — not required, and the reason is 43's own design
section.** 43's § *`TabBar`'s shape, and how it composes with task 42's `Screens`*
decides: *"**`TabBar` owns the buttons and the selection. It owns no content, no
screen, no history and no transition.**"* and *"The bar draws a row of buttons;
`Screens` holds a subtree per name and says which one is on show."*

**The dock's five items are actions, not screens, and there is no selection to own.**

| dock slot | what pressing it does | is it a screen? |
|---|---|---|
| Controls | opens a **panel over the map** | no — § *Screens* records *"The Controls screen appears over the map"* `[A]`, and it is **dismissed by a drag**, so it has no selected state on the dock |
| climate | opens a **popup** | no |
| My Apps | opens the **app tray**, a **card** over the map | no |
| App Launcher | opens the launcher grid | no — and the grid is `L3`/`#2`, **task 52, not built** |
| Volume | adjusts volume | no |

**And the photograph says so independently:** § *Bottom dock* reads *"[A] photo
`01`: Icon-only, flat black bar, no labels, **no active-state highlight**"*. **A
`TabBar` exists to know which tab is selected; a dock with no active-state
highlight has nothing to tell it.** **So the dock is a `Container` in
`LayoutMode::row()` holding five `Button`s, which is precisely what gap `#7`'s row
prescribes** — *"the bottom dock can be built from `Button` + `Container`, but a
dedicated widget with active-state indication and icon+label layout is the right
primitive"* — **with the active-state half declined for this task and recorded.**

**And if 43 has landed, this task does not rewire anything.** The **gallery's**
tab bar becomes a library `TabBar` in 43's sub-task 43.2; the **dock** is not a
`TabBar` either way. **So 43's arrival changes nothing in this task's diff**, and
the criterion that says so is `the_gallery_tab_bar_is_still_present_on_the_demo_page`
— **one assertion, in the file that owns the seven buttons, about a widget this
task never touches.**

### The sentence `DEMO_APPLICATION.md` told this sequence to amend

**§ *Relationship to task 24* ends its tension paragraph with a standing
instruction, and this task is what it was waiting for.** Verbatim:

> **It will need one** if a `TASK_UI_DEMO_n` task ever puts a tab bar on a Tesla
> surface, and that is the sentence to amend when it does.

**This task does put the gallery's tab bar on screen over a Tesla surface** — the
demo page *is* a page of the gallery, so the gallery's `TAB_BAR_HEIGHT` strip is
drawn above the map on every frame the demo tab shows. **So the sentence is
amended**, dated and attributed, with the decision and its reason: **the gallery's
bar stays**, because `DEMO_APPLICATION.md` § *Relationship to task 24* records the
2026-10-03 operator decision that *"this application becomes one more tab, not a
replacement for the gallery"*, and a page of a gallery does not remove the
gallery's own chrome; **and the amendment says which bar is which** — the top strip
is the *gallery's* affordance for switching pages and carries the label `demo`, and
the status bar **inside** it is the Tesla top bar. **Removing the gallery's bar
while the demo tab is showing is a screen-level decision, and screen-level decisions
belong to `TASK_UI_PRIM_42`'s `Screens`.**

### Four things this task must NOT do, each with the reason

1. **No chrome node goes in `Demo::placed_handles`.** `tests::no_two_placed_rects_overlap` is asserted over **`placed_rects()`**, which is the **union** of every
   placed leaf and not page-filtered — `Demo::page_rects`'s own doc is the evidence
   (*"The union said that twenty-five placed leaves coexist in one window without
   touching"*, and that is what the pages dissolved). **The dock at
   `y 924..1020` overlaps the on-screen keyboard and the frame-rate readout, and the
   pane at `x 0..512` overlaps the card of pads.** Adding a row would break an
   assertion that is right, for a reason that is the feature. `task 01` made the
   same call for the map and for the same reason.
2. **`tests::expected_placed_rect_names` gains no row**, and
   `tests::assert_placed_handles_is_complete`'s asserted length does not move.
   **Both are named in `TASK_UI_PRIM_52` § *Acceptance Criteria* as rows *"not
   added or loosened"*.**
3. **The five-row `focusable` count does not move.** `Demo::page_members`'s
   `focusable` flag is asserted at exactly **five** by
   `tests::every_page_lists_at_least_one_node_and_no_node_is_on_two_pages`, and the
   chrome contributes **no `Tab` stop** — the pane's indicator rows and the drive
   mode's four targets are **read-outs**, on `PageMember::focusable`'s own argument
   that *"a stop where nothing lights up is a stop a reader cannot see"*.
4. **`ui_core` has no diff, whichever way 41 went.** The task either calls
   `Painter::backdrop`, which 41 already added, or draws an opaque `RoundedRect`,
   which `Painter` already has. **Neither path needs a crate change**, and that is
   the whole reason this task can be written before 41 lands.

### The baseline, and the counts that must not move

**Measured on the tree this file is written against, `cargo test --all-features`
from `ui/` read 1894 — 1450 in `ui_core` (`1 ignored`, `layout_walk_cost`), 224 in
`ui_demo` and 220 doctests.** **Re-measured 2026-10-09 on the same tree: 2058
passing and 1 ignored** — 1587 in `ui_core` passing of 1588 registered, plus 3 in
`tests/model_file.rs`, 1 in `tests/test_painter.rs`, 236 in `ui_demo` and 231
doctests. **The 1894 is stale and is kept only so the number this file was written
against is still on the record**; § *Scope* in `TASK_UI_PRIM_41` and `TASK_UI_PRIM_42`
record the work that grew the tree between the two measurements.

**So this task has two possible baselines and the handoff must state which it
started from**, because **"the suite is green" is not a number** and
`task-sequence.md` § *Gates* says *"No evidence by assertion. A claim about a file,
a count, a version, or a build result is checked against the file or the command
output."* **2058** if `TASK_UI_DEMO_01` has not landed, **2064** if it has — **six
tests, not the thirty-two its predecessor specified**, because the rewritten
task 01 adds six and the procedural map went to `doc/ui/backlog/`.

**The frame-rate floor is 55 fps on a release build**, and `IMPLEMENTATION_STATE.md`
§ *The frame rate, measured* records the band the six gallery pages sit in. **The
demo page's rate is the number this task produces**, and the chrome is a real
per-frame cost: **one `DrawCommand::Image` from task 01 — not the roughly one
thousand `Rect`/`Path`/`Circle` commands the superseded procedural map cost,
which is the cheapest thing this rewrite bought** — plus five chrome surfaces
plus the demo page's own label and button commands.

### Scope, measured against `developer.md` § *Scope check*

**Four files and three components — inside both thresholds, so no fan-out.**

| file | what changes |
|---|---|
| `ui/src/ui_demo/src/chrome.rs` | **new.** `pub struct Chrome` and its node table, `pub struct ChromePalette`, `fn map_palette`-alike `pub fn chrome_palette(&Theme) -> ChromePalette`, `pub const DOCK_GLYPHS: [&str; 5]`, `fn premultiplied`, the three `fn paint_*` bodies, and its `#[cfg(test)] mod tests` |
| `ui/src/ui_demo/src/main.rs` | `mod chrome;`, `STATUS_BAR_HEIGHT`, `DOCK_HEIGHT`, `CAR_STATUS_PANE_FRACTION`, `CHROME_ALPHA`, `BACKDROP_SIGMA` and the six geometry constants, the five chrome `Container`s and their children in `Demo::new`, `Demo::chrome_nodes: Vec<Chrome>`, one `page_members` row per chrome node, one arm in `Demo::frame`'s paint walk, `Demo::chrome_of(handle)`, and the five new tests |
| `doc/ui/DEMO_APPLICATION.md` | **the § *Relationship to task 24* sentence this task exists to amend**, dated; a dated note in § *The persistent chrome* and § *The car-status pane* recording what 02 built and what it did not; § *Open questions* item 8 **left on the list** |
| `doc/ui/IMPLEMENTATION_STATE_DEMO.md` | the task-table row for **02**, the § *Task DEMO 02* record, the **dependency outcome for 41 and 43 in the record's own words**, the frame rate, and the deviations |

| component | what it is | why it is separable |
|---|---|---|
| 1 | `chrome.rs`'s palette, the premultiply arithmetic and the three paint bodies | **its tests drive `chrome_palette`, `premultiplied` and the `paint_*` functions with no arena**, so it is verifiable before `main.rs` builds a node — the arrangement task 01's superseded `map.rs` used, and which task 01 no longer needs because it has no module of its own |
| 2 | `main.rs`'s node construction, its `page_members` rows and its one paint arm | **four rows and one arm; it needs component 1's `Chrome` to exist and nothing else** |
| 3 | the four documents | **no code and no build** |

**Three components is at the threshold, not over it** — `developer.md` § *Scope
check* says *"more than 5 files"* or *"more than 3 independent components"* — so
**no split is required and none is offered.** **If the implementer finds themselves
editing a file not in this table, that is a stop condition**
(`developer.md` § *Stop conditions*), and the two files most likely to tempt them
are `ui/src/ui_core/src/paint.rs` — where `Painter::backdrop` already exists if 41
has landed and does not need writing if it has not — and `ui/src/ui_demo/src/map.rs`, which is task 01's and is **not** edited here.

## Requirements

1. **`ui/src/ui_demo/src/chrome.rs`, new, and `mod chrome;` beside `mod map;`.**
   The module doc carries, in this order: **what the three regions are**; **that
   they are drawn over the map and are a region of the demo page rather than panels
   over it**; **that the translucency is `TASK_UI_PRIM_41`'s and what this module
   draws when it has not landed**; **that the dock's glyphs are placeholders**; and
   **that `premultiplied` exists and why** — the five facts a reader of the module
   would otherwise have to derive from `ui_core`.

2. **The palette, read from the theme, and the alpha that makes it translucent.**

   ```rust
   /// The chrome's colours, read from the theme and never invented.
   ///
   /// **`premultiplied` on `surface` and `surface_opaque`, and nothing else.**
   /// Every other field is opaque: `Color` is documented in
   /// `ui/src/ui_core/src/property.rs` as premultiplied, and a translucent colour
   /// is a premultiplication the caller performs. A card at `Surface` with
   /// `a = 200` written straight into `Color` composites as
   /// `245 * 200 = 49 000` per channel under `GL_ONE, GL_ONE_MINUS_SRC_ALPHA` —
   /// far too bright — and no assertion on a `Color` field catches it.
   pub struct ChromePalette {
       /// The bar and pane surface, translucent. Premultiplied.
       pub surface: Color,
       /// The same colour opaque, for the no-backdrop path. Alpha 255.
       pub surface_opaque: Color,
       /// The chrome's text. Opaque, from `ThemeToken::Text`.
       pub foreground: Color,
       /// The chrome's secondary text. Opaque, from `ThemeToken::TextMuted`.
       pub muted: Color,
       /// The pager's active dot. Opaque, from `ThemeToken::Primary`.
       pub active: Color,
       /// The pager's inactive dots and the drive-mode strip's track.
       /// Opaque, from `ThemeToken::Border`.
       pub inactive: Color,
   }

   #[must_use]
   pub fn chrome_palette(theme: &Theme) -> ChromePalette
   ```

   `chrome_palette` reads `theme.get(token).as_color().unwrap_or(fallback)` — the
   `fn themed_color` precedent in `main.rs` — and **the two `surface` values are the
   only `premultiplied` call sites in the module**. `CHROME_ALPHA: u8 = 200` and
   `BACKDROP_SIGMA: f32 = 2.5` are named constants in `chrome.rs`, and each carries
   its reason: **`CHROME_ALPHA` is first-principles** because *Tesla publishes no
   design tokens at all*; **`BACKDROP_SIGMA` is first-principles for the same
   reason**, and 2.5 px is `DrawCommand::Shadow`'s own example blur in this
   repository's docs, chosen so the two translucencies on screen match.

   **`CHROME_ALPHA = 200` is not 255 and that is the whole point.** At 255 the
   backdrop is captured, blurred, composited and then **completely hidden behind an
   opaque card** — a real cost for no visible difference. The alpha's own doc says
   so, because a number that looks like the right kind of number and is not the
   right number is the failure this guards against.

3. **`fn premultiplied(color: Color, alpha: u8) -> Color` — the crate's arithmetic,
   written once, with its reason cited.**

   ```rust
   /// Returns `color`'s channels multiplied by `alpha / 255`, in `u16`.
   ///
   /// **The `u16` is the whole of it.** `Pixels::premultiply` in
   /// `ui/src/ui_core/src/texture.rs` does this conversion and its own doc records
   /// why: *"(200 * 200) / 255 is 156.86, and a `u8` multiply would have said 156
   /// for every value."* An implementation that multiplied in `u8` truncates the
   /// product before the divide and darkens every translucent colour by up to one
   /// whole step, and **the difference is one digit of a `u8`** — a test asserting
   /// "the channels got smaller" passes on it.
   fn premultiplied(color: Color, alpha: u8) -> Color
   ```

   **It returns `alpha: u8` and takes `alpha: u8`, not a fraction**, so there is no
   `f32`-to-`u8` conversion to get wrong at the call site. **It is private** — one
   caller shape (`chrome_palette`) and two call sites, which is the seam, not an
   abstraction. **`developer.md` § *Phase 2* is the reason it is a function and not
   inlined twice**: *"No abstraction before the second use"* — there are two call
   sites.

4. **`pub const DOCK_GLYPHS: [&str; 5]` — five entries, and the doc says they are
   placeholders.** One character each, from the ASCII range the default face
   certainly has, in `DOCK_GLYPHS`'s declaration order matching the five slots of
   § *The two declared dependencies*. **The constant's doc says, in the module's
   own voice, that these are placeholders and that `TASK_UI_PRIM_44`'s
   `ui_core::widgets::Icon` is what replaces them** — because the operator's
   decision of 2026-09-30, `DEMO_APPLICATION.md` § *Operator decisions (2026-09-30)*
   item 3, is *"**Real icons.** A dedicated task inventories needed icons and
obtains or generates missing assets. **No placeholder geometric shapes**"* — **and
   this task ships five placeholder glyphs, which is a deviation from an operator
   decision and is recorded as one in the handoff and in the state file rather than
   described as done.**

   **`DOCK_GLYPHS` is a `const` array and not a `Vec`, and its length is the
   count** — the same reason `Page::ALL` is `[Page; 6]` in `main.rs`: *"a
   fixed-size array, so the type carries the count."* **`DOCK_GLYPHS.len() ==
   DOCK_SLOTS` is a compile-time fact and a test**, and the test
   `the_dock_is_a_row_of_five_slots_and_the_glyph_table_has_five_entries` is where
   it is asserted.

5. **The three regions, their nodes, and their rectangles — in `Demo::new`, in this
   order, each with an explicit position.**

   1. **`status_bar`**: `Container::new(&mut nodes, LayoutMode::row())`,
      `set_flex_config` with `MainAxisAlignment::SpaceBetween` and
      `CrossAxisAlignment::Center`, `set_padding(Padding::all(STATUS_BAR_PADDING))`,
      `LayoutMode::Absolute` at `Offset::new(0.0, CONTENT_TOP)`,
      `Constraints::tight(Size::new(WINDOW.width, STATUS_BAR_HEIGHT))`.
      **Three child groups**, each a `Container` in `LayoutMode::row()`:
      **`status_left`** (padlock label, profile label, sentry label),
      **`status_right`** (clock label, ambient label), **`status_airbag`** (one
      label). **No `PRND` and no battery in either group** — § *The four regions*
      gives the photograph's own words for why.
   2. **`car_status_pane`**: `LayoutMode::column()`, at
      `Offset::new(0.0, CONTENT_TOP + STATUS_BAR_HEIGHT)`, tight to
      `Size::new(WINDOW.width * CAR_STATUS_PANE_FRACTION, dock_top -
      CONTENT_TOP - STATUS_BAR_HEIGHT)` where `dock_top = WINDOW.height -
      DOCK_HEIGHT`. **`CAR_STATUS_PANE_FRACTION` is the one number in this file that
      is not first-principles**, and its doc says so and cites the photograph.
      Three children: **`prnd_row`** (two labels — `PRND` and the battery
      percentage), **`indicator_column`** (a `Container` in `LayoutMode::column()`
      of five labels), and **`card_row`** (a `Container` in `LayoutMode::row()` of
      three card `Container`s, each with one label, plus a `pager` `Container`
      holding three 6-pixel-radius `Circle` labels' worth of geometry — see
      requirement 6).
   3. **`bottom_dock`**: `LayoutMode::row()` at
      `Offset::new(0.0, WINDOW.height - DOCK_HEIGHT)`, tight to
      `Size::new(WINDOW.width, DOCK_HEIGHT)`, with five `Button`s whose `label` is
      `DOCK_GLYPHS[i]`, each given `font_size`, `padding_h` and `border_radius`
      **before it is measured**, on the `Demo::new` tab-bar button's own argument:
      *"`Button::content_size` reads `padding_h`, `font_size` and the label, and a
      width measured before they are set is a width for the widget's defaults."*
      **No `set_palette` call on any of them**, because the dock has **no
      active-state highlight** and the default palette is the rest pair — that is
      § *The two declared dependencies*'s whole argument, and a comment on the loop
      says so rather than leaving the absence looking like an oversight.

   **Every one of the eight new containers is created with `Container::new` and
   given `snap_to_state`-equivalent treatment only where it owns a property**, and
   **the three chrome surfaces are not in `Demo::containers`** — see requirement 7.

6. **The three things inside the pane that are not labels, and how they are
   recorded.**

   - **The indicator column is five rows, one per colour class, in
     `DEMO_APPLICATION.md` § *The car-status pane*'s own order: red, amber, green,
     blue, grey.** Each row is a `Label` whose colour is a `Property<Color>` built
     by `fn themed_color` from the token named in that section's list — `Error`,
     `Warning`, `Success`, `Primary`, `TextMuted`. **The five rows are a fixed
     five and not a table of ~20 conditions**, and the column's doc says so: the
     section enumerates ~20 with *"three of these distinguished **by timing, not
     colour**"* and one latching, and **none of that is built here**.
   - **The pager is three dots and three commands.** Each dot is a `Circle` of
     radius `PAGER_DOT_RADIUS` at a position `PAGER_DOT_GAP` apart, in
     `palette.active` for the first and `palette.inactive` for the other two.
     **A dot is a `Circle` and not a `Label`**, so the pager needs a paint arm of
     its own — and that arm is the reason requirement 7's list has three entries and
     not two.
   - **The drive-mode strip is four rows of one `Label` each — `↑`, `P`, `↓`,
     `HOLD` — in a `Container` in `LayoutMode::column()` inset inside the pane.**
     **Its four glyphs are `↑` and `↓` from the manual's `[C]` snippet and `P` and
     `HOLD` from photograph `02`'s `[A]` reading**, and the strip's doc says which
     is which and repeats that § *Drive-mode strip — the left edge* calls the whole
     gesture model `[C]` and *"must be re-verified against an opened page before a
     `TASK_UI_DEMO_n` task treats it as specified."* **No gesture is attached to
     any of the four**, so nothing here depends on a claim that cannot be verified
     from this host.

7. **Four paint arms — the chrome's three surfaces, its five buttons, its labels
   and its pager's dots — and the reason each exists is a widget seam.**
   **`Demo::chrome_nodes: Vec<Chrome>`** where
   `pub struct Chrome { pub node: Handle, pub radius: f32, pub palette: ChromePalette }`,
   and `Demo::chrome_of(handle: Handle) -> Option<&Chrome>` in `fn tab_button`'s
   shape — a **search over the vector**, not an arithmetic position, for the reason
   `Demo::tab_button`'s own doc gives (*"a search … rather than a position, because
   a handle is what routing and the paint walk arrive with"*).

   The arm sits in `Demo::frame`'s walk **after the map arm and before the container
   arm**, and for a matched handle records, in this order:

   - **`Painter::backdrop(rect, BackdropMode::Blur(BACKDROP_SIGMA),
     palette.surface)` — only if `TASK_UI_PRIM_41` has landed.** The `cfg` is
     **on one `if` and one `use`**, and **the criterion is the `git grep` for
     `BackdropMode` in `chrome.rs`**, so "41 has landed" is a fact about the tree and
     not a claim.
   - **`Painter::rounded_rect(rect, chrome.radius, palette.surface)`** — or
     `palette.surface_opaque` on the no-41 path.

**And the pager's arm records three `Circle`s and nothing else.** It has no
   widget behind it — a dot is not a label, a button or a container — so it is the
   third arm, and `a_pager_paint_arm_records_three_circles_and_nothing_else` is its
   test.

   **The chrome's own children need three more arms, and each exists because of a
   widget seam rather than a convenience.** `Demo::frame`'s walk already has to
   branch per widget shape, and adding the chrome's twenty-odd nodes to the wrong
   arm is the defect:

   - **The dock's five `Button`s get an arm of their own**, keyed by
     `Demo::chrome_button(handle) -> Option<&Button>` in `fn tab_button`'s shape,
     calling `button.paint(rect.into(), &chrome_advance(&self.metrics),
     chrome_line_height(&self.metrics))`. **`Button::paint` measures its label**, so
     it takes an advance closure and a line height, and the walk has no such
     parameter — `Demo::frame`'s own comment on the tab-button arm says exactly
     this: *"`Button::paint` measures its label, so it takes an advance closure and
     a line height, and adding two parameters to the rect-only arm below would be a
     change to five widgets' code for one widget's seam."* **Five buttons are one
     widget's seam.**
   - **The chrome's labels are painted through the existing `fn record_label`**, from
     a **`chrome_labels: Vec<(DemoLabel, Handle)>` on `Demo`**, in one loop beside
     the panel's own. **They are not appended to `Demo::labels` and
     `Demo::label_nodes`**, and the reason is concrete: `fn placed_handles` pushes
     `("text panel label", handle)` for **every** entry of `label_nodes`, so a
     chrome label added there would be *named* a text-panel label, would enter
     `expected_placed_rect_names`, and would put a chrome rect into
     `no_two_placed_rects_overlap` — **which it would break, because the status
     bar's labels sit inside a rect that overlaps nothing but the pane's labels do
     not and the map's does.** **Reusing `record_label` rather than writing a second
     label painter is the whole of the reason this is a `Vec` and not a new
     function.**
   - **The pager's three dots have no widget at all** and are the third arm, as
     above.

   **The arm order is the z-order and it is stated, because the two paths differ.**
   **A shadow lands on whatever was recorded before it, not on whatever comes
   next**: a `Shadow` is composited **after everything its
   own segment recorded** (`render.rs`), and `TASK_UI_PRIM_41` § *Requirements*
   gives `Backdrop`'s `BatchKey::is_singleton` the same property — it **seals** the
   segment. **So the `Backdrop` must be recorded before the surface fill on the same
   node, and a chrome node's own children are on other nodes and are recorded after
   it in the `order` walk**, which is why the content of a translucent card lands on
   the card. `a_chrome_surface_records_its_backdrop_before_its_surface` is the test,
   and **it is the enforcement for a mechanism no draw-command assertion can see by
   position alone** — the rule is that a comment claiming a state transition happens
   *"once, up front"* is a testable claim, and this is it.

8. **Five `page_members` rows, one per chrome surface, and no `focusable` flag.**
   `on(Page::Demo, status_bar.handle(), false)`, and the same for the pane and the
   dock; **plus one row each for the three chrome *groups* that are nodes in their
   own right** — the strip, the indicator column and the card row — **because
   `hit_test` skips the whole subtree of an invisible node** and a container on no
   page's list would leave its children drawn on every page. That is `Demo::new`'s
   own written reason for the `text_column` row (*"hiding the labels alone would
   leave a container that is on no page drawing nothing over a page that is not
   showing"*), and **the six rows are written where
   `tests::every_page_lists_at_least_one_node_and_no_node_is_on_two_pages` can see
   them.**

   **No row is `focusable`**, and the count stays **five** — § *Four things this
   task must NOT do* item 3.

9. **`DOCK_GLYPHS`, `CHROME_ALPHA` and the pager's geometry are one shared
   constant, and the pager's dot positions are arithmetic rather than literals.**
   `fn pager_centre(index: usize, row: Rect) -> (f32, f32)` — `#[must_use]`, three
   dots, `PAGER_DOT_GAP` apart, centred in `row` — and
   `the_pager_places_three_dots_pager_dot_gap_apart_and_centred` asserts the three
   `(x, y)` pairs by value. **A literal `x` per dot would be three numbers that a
   resize could not move together**, which is the `developer.md` § *Phase 2*
   *"Read the surrounding code first"* hazard in its smallest form.

10. **The documents, three files, dated and attributed.**

    - **`DEMO_APPLICATION.md` § *Relationship to task 24**: the standing
      instruction — *"**It will need one** if a `TASK_UI_DEMO_n` task ever puts a tab
      bar on a Tesla surface, and that is the sentence to amend when it does"* — is
      **replaced by a dated note recording that this task did**, with the decision
      (**the gallery's bar stays**) and its reason (**§ *Relationship to task 24*'s
      2026-10-03 decision that the demo *becomes one more tab*, so a page of the
      gallery does not remove the gallery's chrome**), and with the sentence that
      **removing it is a screen-level decision belonging to `TASK_UI_PRIM_42`'s
      `Screens`.** **The rest of that section is untouched**, and in particular the
      four-file table of places recording the withdrawn 2026-10-03 decision is
      neither edited nor re-listed.
    - **`DEMO_APPLICATION.md` § *The persistent chrome* and § *The car-status pane*
      each gain a dated note** recording what 02 built — three regions, three
      always-on top-bar items, two parked-only, five indicator rows, three cards,
      three pager dots, four drive-mode targets, five dock slots — **and what it did
      not**, naming: no panels, no popups, no app tray, no gesture, no iconography,
      no 3-D car, no per-field staleness, no latch, no blinking tell-tale, no resize
      drag, no paging.
    - **`DEMO_APPLICATION.md` § *Open questions* item 8** (*"Card carousel depth"*
      — *"Should the demo implement paging (the real behaviour) or a swipe-with-peek
      (cheaper, and what the photo's evidence supports equally well)?"*) is
      **left on the list, unchanged**, and the carousel's three-card static row
      records which way it leans and why. **`item 5`** (*"The car's body is a
      required asset"*) is **left on the list too**, and § *Out of Scope* says what
      stands in for it.
    - **`doc/ui/IMPLEMENTATION_STATE_DEMO.md`** gains the task-table row for **02**
      (file, review count, waivers-or-none, frame rate) and a § *Task DEMO 02*
      record carrying **the chrome's three regions and their rectangles**,
      **the dependency outcome for 41 and 43 in the record's own words**,
      **the deviation for the placeholder dock glyphs**,
      **the deviation for the opaque chrome if 41 had not landed**,
      **the test count before and after**, and **the frame rate for all seven
      pages**.
    - **No other document changes**, and in particular `TASK_UI_PRIM_41.md` and
      `TASK_UI_PRIM_43.md` are **untouched** — this task *uses* their deliverables
      and does not *edit* their specifications.

11. **The dependency handling as code, not as prose.** **One `if` on
    `ui_core::paint::BackdropMode`'s availability, and one `use` line**, in
    `chrome.rs`; and **both are behind a named constant or a named function, never
    a literal.** Concretely: **`fn chrome_surface_commands(rect: Rect, chrome: &
    Chrome, backdrop: bool) -> Vec<DrawCommand>`**, whose `backdrop` argument is
    **supplied by the one call site from `Demo::frame`**, and
    `a_chrome_surface_records_its_backdrop_before_its_surface` and
    `a_chrome_surface_without_a_backdrop_records_only_its_surface` are **its two
    tests, both calling it directly with a `true` and a `false`**.

    **Why the flag is an argument and not a `cfg!`:** a `cfg!` makes the fallback
    **untestable**, because a tree where 41 has landed can never exercise the
    no-41 branch. **An argument means both branches are covered by two tests on any
    tree**, which is what `TASK_UI_PRIM_41`'s own § *Out of Scope* calls *"the
    shrink that would be honest"* being refused in the opposite direction — and the
    one call site is the only place the flag comes from, so a caller cannot get it
    wrong twice.

## Testing

**Every test is named, every one is legal under `AGENTS.md` § *Rust*, and every one
asserts a contract.** **Fourteen in `chrome.rs`, five in `main.rs`.**

**In `chrome.rs`'s `#[cfg(test)] mod tests` — fourteen:**

| # | test | what it holds |
|---|---|---|
| 1 | `premultiplied_multiplies_each_channel_by_the_alpha_in_u16` | **`(200, 200, 200, 128) → (100, 100, 100, 128)` by value**, and the doc cites `Pixels::premultiply`'s own arithmetic |
| 2 | `premultiplied_leaves_an_opaque_colour_unchanged` | `alpha == 255` is the identity — **the control for 1**, without which 1 could pass on a function that always returns its input |
| 3 | `a_translucent_palette_is_strictly_darker_than_its_opaque_twin` | `surface.r < surface_opaque.r` on both channels, and `surface.a == CHROME_ALPHA` — **the assertion that a `u8` multiply survives**, since truncating the product darkens it |
| 4 | `chrome_palette_reads_six_values_and_only_two_are_translucent` | six fields, and `surface`/`surface_opaque` are the only two with `a != 255` |
| 5 | `chrome_palette_falls_back_when_a_token_holds_a_number` | the `as_color().unwrap_or(..)` path, as in task 01 |
| 6 | **`a_chrome_surface_records_its_backdrop_before_its_surface`** | with `backdrop: true`, the recorded commands are **`[Backdrop, RoundedRect]`** and the `RoundedRect`'s rect equals the one the `Backdrop` carries. **This is the enforcement for a mechanism no other assertion can see** |
| 7 | **`a_chrome_surface_without_a_backdrop_records_only_its_surface`** | with `backdrop: false`, **exactly one** `RoundedRect` at `surface_opaque`. **The control for 6**, and the fallback path is covered on any tree, which is the point of requirement 11 |
| 8 | `the_backdrop_carries_the_palette_s_premultiplied_surface_and_blur_two_point_five` | the tint is `palette.surface`, not `surface_opaque`, and the mode is `Blur(BACKDROP_SIGMA)` — **a card whose blur is then hidden under an opaque fill is the "right kind of number" defect** |
| 9 | `the_dock_is_a_row_of_five_slots_and_the_glyph_table_has_five_entries` | `DOCK_GLYPHS.len() == DOCK_SLOTS` and both are 5 |
| 10 | `every_dock_glyph_is_one_non_empty_character` | each entry is exactly one char and not whitespace — **an empty label draws nothing and cannot be seen**, which is the **drawn control with nothing behind it** shape |
| 11 | `a_pager_paint_arm_records_three_circles_and_nothing_else` | **exactly three** `Circle`s, and no other variant in the vector |
| 12 | `the_pager_places_three_dots_pager_dot_gap_apart_and_centred` | the three `(x, y)` by value, and the middle one is the row's own midpoint |
| 13 | `the_indicator_column_is_five_rows_in_red_amber_green_blue_grey_order` | five, and the order is the section's |
| 14 | `the_chrome_nodes_do_not_overlap_each_other` | **the four-sided separation test** — the three rectangles' interiors do not intersect. **The row is not at the origin** — a geometry fixture at the origin cannot see an origin being read as an extent |

**In `main.rs`'s `mod tests` — five:**

- **`the_demo_tab_exists_from_task_01_and_three_gates_hold_for_it`** —
  `Page::ALL.len() == 7`, `Page::Demo.name() == "demo"`,
  `Page::DEFAULT == Page::Pads`, `Page::from_name("demo") == Some(Page::Demo)`,
  and `Page::from_name("dta") == None`. **This is task 01's deliverable asserted
  rather than assumed**, and `Page::DEFAULT` is the assertion that matters: a
  capture with no argument must still open on `pads`.
- **`the_chrome_is_on_the_demo_page_and_on_no_other_page`** — for each of the six
  chrome nodes, `Demo::shows(node)` is true on `Page::Demo` and false on all six
  gallery pages; and **every row's handle is in `Demo::order`**. **The complement
  assertion `tests::always_painted_handles` is what catches a row that was
  forgotten**, and this test is the same shape for the six new rows.
- **`the_map_node_is_still_underneath_the_chrome_on_the_demo_page`** — after
  `Demo::frame` on `Page::Demo`, the map node's commands and the chrome nodes' are
  read from `Demo::commands_at`, and the map's are **recorded earlier in `order`**
  than every chrome node's. **The z-order is a claim about two nodes and it is
  asserted as one.**
- **`every_chrome_node_records_commands_on_the_demo_page_and_on_no_other_page`** —
  the paint gate over **every** chrome node, the three surfaces *and* their children
  including the five dock buttons and the pager: **at least one command on
  `Page::Demo`, empty on each of the six.**
  **Corrected 2026-10-09 from *more than one*, and the correction is the same one
  `TASK_UI_DEMO_01` needed for the map:** `Label::paint` records one `text_run` per
  line (`widgets/label.rs:581`), so a single-word chrome label records **exactly
  one** command and a `> 1` assertion fails on correct code. **The surfaces are
  the ones that must record more than one**, and they do — a surface with children
  and a background — so the number belongs to the surfaces if anywhere, which is
  why the assertion is `>= 1` over every node and the surfaces' multiplicity is
  `the_chrome_surfaces_draw_their_background_and_their_children_on_top_of_it`
  instead. This is the criterion that the chrome
  does not leak onto the gallery, and **covering the children is the point** —
  a chrome surface that records commands while its own labels record none is a
  bar with nothing on it, and a drawn control with nothing behind it is a picture
  no test could see the difference in.
  **Its control is the demo page's own map node** — **amended 2026-10-09**:
  task 01's map will be **one `DrawCommand::Image`**, not the large non-empty
  vector this sentence described. **A single command is still a control, and the
  assertion is unchanged** — `more than one command` is what the chrome's own
  node must record on `Page::Demo` and not on the six gallery pages, and **the
  map's one command is the non-vacuity floor that stops the whole loop from
  passing on a demo that paints nothing at all.** What a control is for here is
  not the size of the vector but that *some* node on the page records a command
  while the six gallery pages record none. **The two numbers are different and
  both are right**: `more than one` is about the chrome's own node, which is a
  surface with children and records several; `at least one` is about the map,
  which is one image and records one (`ui_demo`'s own
  `the_demo_page_draws_the_map_and_the_gallery_pages_do_not` says so, and a
  `> 1` there would fail on correct code).
- **`a_theme_switch_reaches_every_chrome_surface`** — `toggle_theme()` and then all
  three chrome surfaces' palettes differ in at least `surface`, and the whole
  gallery's six pages are pixel-different afterwards. **It is the test that the
  chrome is bound to the property graph rather than to a captured colour**, on the
  `Demo::new` argument that *"the property graph carries a theme switch to a widget
  that is holding a token"*.

**Three deliberate breaks the handoff must paste**, because `developer.md` §
Phase 3 says *"A test that has never failed is not a test"*:

1. **Swap the `backdrop` and `rounded_rect` calls** in
   `chrome_surface_commands` and re-run test 6 — it fails on the order, **and the
   capture shows the surface painted over its own blur.** This break has a visible
   consequence, and *a shadow lands on whatever was recorded before it* predicts it.
2. **Premultiply with a `u8` multiply** instead of a `u16` one and re-run test 1 —
   it fails by value, on the number `Pixels::premultiply`'s doc predicted.
3. **Give the dock's third `Button` an empty label** and re-run test 10 — it fails.
   **Its control is test 11**, where three dots recording three circles *is*
   asserted, so a reader can tell an empty label from a missing one.

## Acceptance Criteria

- [ ] **The seventh tab and `--tab=demo` are verified, not re-added.**
      `rg -n 'Page::Demo' ui/src/ui_demo/src/main.rs` finds it in `enum Page`, in
      `const ALL`, in `fn name` and in **at least one `on(Page::Demo, ..)` row**;
      `rg -n 'const ALL: \[Page; ' ui/src/ui_demo/src/main.rs` reads `[Page; 7]`;
      **and `git diff` for this task shows no line adding `Page::Demo`, no change to
      `const ALL` and no change to `const DEFAULT`** — which is the check that this
      task used task 01's page rather than making a second one. Then, on a release
      build: `--tab=demo` opens the demo page; `--tab=dta` refuses and names **seven**
      pages; `--help` prints seven; **and `--tab=demo` with no argument opens
      `pads`**, checked by the capture below and not by the parser alone.

- [ ] **The chrome is on screen, in the right places, and does not leak.**
      Release build, `--tab=demo`, captured by the stock method of
      `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
      method* verbatim — window id from
      `DISPLAY=:0 xwininfo -root -tree | rg '"roados ui_demo"'`, then
      `DISPLAY=:0 magick import -window <id>` — **with `pgrep -a -x ui_demo` in the
      same call as each capture and no seed, no environment variable and no rebuilt
      binary**, which is stated because a picture whose method was not given is the
      failure this guards against. **The capture is read for seven things, each stated in the handoff:**
      the map is visible across the whole area below the gallery's tab bar; **a
      status bar across the top with the padlock, the profile name and the airbag
      badge and nothing else**; **a dock across the bottom with five slots and no
      labels**; **a pane on the left at roughly 40 % with `PRND` and the battery
      percentage at its top, five indicator rows in red/amber/green/blue/grey, and
      three cards above a three-dot pager with the first dot active**; **the
      drive-mode strip's four targets inset at the pane's left**; **the gallery's
      seven-button tab bar still above the status bar**; and **the whole thing in
      the light theme as well**, because § *Design principles* records that *"both
      of the operator's own photographs are the light theme"*.

      **And the six gallery pages are unchanged**, compared with
      `magick compare -metric AE before.png after.png null:` — **AE 0 outside the
      fps readout's band**, which is the criterion tasks 34 to 39 inherited and what
      `IMPLEMENTATION_STATE.md` records as *"the one thing two captures of an
      unchanged frame differ in."* **Unlike `TASK_UI_DEMO_01`, which added a tab
      button to every page, this task adds nothing to any page but the demo's** —
      which is what makes AE 0 the criterion rather than a region of it.

      **The mechanism is four facts, and saying so is the criterion.**
      **`rg -n 'map_node|chrome_nodes' ui/src/ui_demo/src/main.rs` shows neither
      reaching `fn placed_handles`** — so `every_page_places_every_rect_where_the
      _gallery_placed_it` and `no_two_placed_rects_overlap` hold with **every
      assertion untouched**, which is what makes them free: **the pane at
      `x 0..512` overlaps the card of pads and the dock at `y 924..1020` overlaps
      the keyboard**, so a row in `placed_handles` would break a correct assertion.
      **`tests::expected_placed_rect_names` gains no row** and its asserted length
      does not move. **The chrome is on no gallery page**, because every row is
      `Page::Demo` and `Demo::on_show` is the predicate both gates read —
      **asserted by `every_chrome_node_records_commands_on_the_demo_page_and_on_no
      _other_page` over every chrome node and its children, not only the three
      surfaces.** **And `CONTENT_TOP`, `WINDOW` and `TAB_BAR_HEIGHT` are
      unchanged**, so no gallery rect moved. A change that cannot move a pixel is
      **demonstrated not to**, which is why the greps are part of this criterion and
      not a remark.

- [ ] **The geometry is asserted, not eyeballed.**
      `the_chrome_nodes_do_not_overlap_each_other` runs the four-sided separation
      test over the three rectangles and **none is at the origin** — a geometry
      fixture at the origin cannot see an origin being read as an extent;
      `the_pager_places_three_dots_pager_dot_gap_apart_and_centred` asserts the
      three centres by value;
      `the_indicator_column_is_five_rows_in_red_amber_green_blue_grey_order`
      asserts the count and the order;
      `the_dock_is_a_row_of_five_slots_and_the_glyph_table_has_five_entries`
      asserts **both counts, and they are compile-time facts** — `DOCK_GLYPHS` is a
      `[&str; 5]` and `DOCK_SLOTS` is a `usize`, so a mismatch is a compile error
      and the test is the second reader.
      **`the_seventh_tab_button_lays_out_inside_the_window` from task 01 still
      passes**, because the status bar's `WINDOW.width` is not the bar's: **seven
      dock slots and five bar buttons are different rows and neither may push the
      other out of the window.**

- [ ] **`TASK_UI_PRIM_41`'s outcome is a fact about the tree and is recorded either
      way.** `git grep -n 'BackdropMode' ui/src/ui_demo/src/chrome.rs` returns the
      `use` and the one call site **if 41 has landed**, and returns **nothing** if it
      has not. **Either way, `a_chrome_surface_records_its_backdrop_before_its
      _surface` and `a_chrome_surface_without_a_backdrop_records_only_its_surface`
      are both present and both pass** — which is requirement 11's whole point, that
      the flag is an argument and not a `cfg!` so **both branches are covered on any
      tree.**

      **If 41 has landed**, the capture shows **a blurred map visible through the
      three surfaces**, `the_backdrop_carries_the_palette_s_premultiplied_surface_
      and_blur_two_point_five` asserts the tint is `palette.surface` and the mode is
      `Blur(BACKDROP_SIGMA)`, and **the handoff states the frame cost of five
      window-sized `GL_RGBA8` captures a frame** — one per chrome node — because
      41's § *Out of Scope* records that `ColourTarget` is **window-sized** and that
      *"capturing full-window RGBA and blurring it every frame is a different order
      of cost, and the target is aarch64."*

      **If 41 has not landed**, the handoff states, in those words, that **the
      chrome is opaque, that this is a deviation and not a waiver, and that an
      alpha-only wash was refused because `DEMO_APPLICATION.md` § *Design
      principles* calls translucency *"the single structural fact the original
      principles missed"* and a flat wash is not it.** `git diff --stat` shows
      **no change to `ui/src/ui_core/src/`** either way.

- [ ] **`TASK_UI_PRIM_43`'s outcome is that nothing changes, and it is asserted.**
      `git grep -n 'TabBar\|Button::selected' ui/src/ui_demo/src/chrome.rs` returns
      **nothing**, and the dock is a `Container` in `LayoutMode::row()` holding five
      `Button`s with **no `set_palette` call on any of them** — because § *The two
      declared dependencies*'s argument is the photograph's own *"no active-state
      highlight"*, and a `TabBar` that owns *"which one is selected"* has nothing to
      own over a dock whose five items open panels.
      **`the_gallery_tab_bar_is_still_present_on_the_demo_page` passes** — asserted
      in the file that owns the seven buttons, about a widget this task never
      touches — **which is the criterion that 43's arrival changes nothing in this
      task's diff.**
      And **`doc/ui/TASK_UI_PRIM_43.md` has no diff**, because this task *uses* 43's
      deliverable and does not *edit* its specification.

- [ ] **The sentence `DEMO_APPLICATION.md` said to amend is amended, dated, and
      attributed.** `git diff` on `doc/ui/DEMO_APPLICATION.md` shows
      § *Relationship to task 24*'s *"**It will need one** if a `TASK_UI_DEMO_n` task
      ever puts a tab bar on a Tesla surface, and that is the sentence to amend when
      it does"* **replaced by a dated note** recording that this task did, the
      decision (**the gallery's bar stays**), the reason (§ *Relationship to task
      24*'s 2026-10-03 decision that the demo *becomes one more tab*), and that
      **removing it is `TASK_UI_PRIM_42`'s decision**. **The four-file table of
      places recording the withdrawn 2026-10-03 decision in that section is neither
      edited nor re-listed**, and **§ *Open questions* item 8 is still on the list**
      — because *"Should the demo implement paging (the real behaviour) or a
      swipe-with-peek?"* is the operator's question and a static three-card row does
      not answer it. **The three-region capture above is what makes the amendment
      true rather than asserted.**

- [ ] **The premultiplied arithmetic is asserted by value and the mutation is
      killed.** `premultiplied_multiplies_each_channel_by_the_alpha_in_u16` asserts
      **`(200, 200, 200, 128) → (100, 100, 100, 128)`**, with
      `premultiplied_leaves_an_opaque_colour_unchanged` as its control;
      `a_translucent_palette_is_strictly_darker_than_its_opaque_twin` is the
      assertion a `u8` multiply survives. **Mutation evidence in the handoff:**
      multiply in `u8` instead of `u16` and watch the first fail on the number
      `Pixels::premultiply`'s own doc predicted; make `premultiplied` return its
      input and watch tests 1 and 3 fail together. **The reason the test is written
      on a value at all** is that **a "it got smaller" assertion passes on a
      number that is wrong.**

- [ ] **`cargo test --all-features` is green with every named test present, and the
      three counts are pasted.** From `ui/`: `cargo fmt --check`;
      `cargo build --all-targets --all-features`;
      `cargo clippy --all-targets --all-features -- -D warnings`;
      `cargo test --all-features` with **each of the fourteen `chrome.rs` tests and
      each of the five new `main.rs` tests listed by name in the handoff**, from a
      stated baseline of **2058** or **2064** depending on whether task 01 landed,
      **so the total is the baseline plus nineteen and nothing is removed**;
      `cargo doc --no-deps` clean; `cargo audit` **recorded as not installed on this
      host, not passed**. **`layout_walk_cost` is still `#[ignore]`d** and
      `ui/src/ui_core/src/layout.rs` has **no diff at all**. **And the three
      deliberate breaks are pasted with their failure output**, including break 1's
      visible consequence in the capture.

- [ ] **The frame rate is measured on all seven pages and reported with the line
      pasted.** `.ai/tools/fps-check.sh 10 55` on the default page — **the only
      thing the script can do**, since it takes `seconds` then `floor`, builds
      release and runs `./target/release/ui_demo` with **no arguments and no page** —
      and then `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for
      **each of the seven**, with the `roados-fps` line parsed by hand. **Every page
      above the floor of 55.** The six gallery pages inside the recorded band,
      **because nothing this task does runs on them**; **and the `demo` page's rate
      is the real number this task produces and is reported whatever it is** — with
      the command count the demo page records per frame stated beside it, and **with
      41 landed, the five captures named**. **This is the gate in `task-sequence.md`
      § *Gates* (*"No unmeasured run of the demo"*)**, and nothing else in this
      checklist can see a frame-cost regression, because **a still of a 4 fps
      application is pixel-identical to a still of a 60 fps one** — which is how a
      four-fps regression survived three reviews in this repository. **And the
      numbers name their build**: `ls -la ui/target/release/ui_demo` settles in one
      command which binary was measured — **a debug build reads as a performance
      regression**.

- [ ] **Nothing leaked in, and the manifest rule holds.** `git diff --stat` names
      **no file under `ui/src/ui_core/src/`**, **no file under
      `ui/src/ui_demo/assets/`**, **no file but `main.rs` under
      `ui/src/ui_demo/src/`**, and **no file but `chrome.rs` is new there** —
      `map.rs` is task 01's and is untouched.
      **`ui/Cargo.toml` and `ui/Cargo.lock` are unchanged**: the approved direct
      dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, and
      **the chrome is `Container` + `Button` + `Label` + two draw commands.**
      **`rg -c unsafe ui/src/ui_demo/src/chrome.rs` is zero** and
      `rg -n 'unwrap\(|expect\(|panic!|unimplemented!|todo!'` over `chrome.rs`
      returns **nothing**; `as` appears **nowhere in `chrome.rs`** — every numeric
      conversion in it is `u8 → u16` or `f32 → f32`, which needs none.
      **Edition 2021 and `rust-version = "1.85"` hold**: nothing newer than 1.85 is
      used, and `MainAxisAlignment::SpaceBetween` is a variant that exists today.
      **No new `pub` item in `ui_core`**, so there is no semver question at all —
      `chrome.rs` is a module of a binary.

- [ ] **What the handoff does not claim, in those words.** It states that **the
      dock's five glyphs are placeholders** and that this **departs from the
      operator's decision of 2026-09-30** — *"**Real icons.** … **No placeholder
      geometric shapes**"* — with `TASK_UI_PRIM_44` named as what replaces them;
      that **the chrome is translucent only if `TASK_UI_PRIM_41` has landed**, and
      says which branch the tree took; that **the dock's five slots open nothing**
      — no Controls panel, no climate popup, no app tray, no launcher grid, no
      volume overlay, and **no gesture on any of them**; that **the status bar's
      clock and ambient readings are static strings**, because *"Your vehicle
      automatically updates the time"* `[A]` is a vehicle behaviour this demo does
      not have, and **a `std::time::SystemTime` in the chrome would be a wall-clock
      read in a paint path**; that **the indicator column is five rows and not the
      ~20 conditions**, with **no blink semantics and no latch**; that **the car in
      the pane is the pane's own picture and not a 3-D car** — **amended 2026-10-09:
      the sentence this replaces said *the car in the pane is the map's marker*, and
      that was true of the superseded procedural map, which drew one. Task 01's map
      is a picture with nothing on it, so there is no map marker to reuse and the
      pane draws its own** — `DEMO_APPLICATION.md` § *Open questions* item 5 records
      that the car's body *"is a required asset, not an optional one"* and it stays
      open; that **the carousel does not page and does
      not swipe**, so § *Open questions* item 8 stays open; that **the pane does not
      resize**, because the sentence that specifies the two-axis reshape is `[C]`
      snippet-level and the corpus is unreachable from this host; and that **the
      drive-mode strip's four targets have no gesture**, for the same `[C]` reason.

## Out of Scope

- **No panels, no popups, no app tray, no app launcher, no media player, no alert
  slot, no theater, no browser.** `DEMO_APPLICATION.md` § *Screens*'s table has
  **seven surfaces over the map** and this task builds **none** of them — only the
  three regions that § *Screens* records as *"always on screen"* or as chrome. **The
  dock's five buttons are wired to nothing** and that is stated in the dock loop's
  comment, because a button with no callback is not an oversight to be discovered.

- **No gesture at all: no drag-to-dismiss, no swipe, no long press, no pinch, no
  two-axis reshape.** § *Screens* closes with *"**Every dismissal is a drag, and
  the direction is part of semantics**"* and names it *"the one gesture `ui_core`
  currently has no consumer for"* — which is `L4`, and `TASK_UI_PRIM_40` § *Goal*
  says it *"partially closes"* `L4` and is **not started**. **`InputEventKind::
  Swipe` and `LongPress` are consumed by no widget in `ui_core` today**, so a drag
  on this task's chrome would go nowhere. **And no acceptance criterion could
  verify one**: *pointer injection has never delivered an event to the window on
  this host and keyboard injection delivered exactly one* — `IMPLEMENTATION_STATE.md`
  § *Verifying a change that draws — the capture method* — and
  `TASK_UI_PRIM_42`'s § *Context* states the same rule as *"**no acceptance criterion
  here may require a pointer-driven interaction**"*. **A gesture-free chrome is a
  capture-verifiable chrome, and that is not a coincidence.**

- **No `Icon` widgets, and the five dock glyphs are placeholders.** `TASK_UI_PRIM_44`
  closes gap `#4` with `ui_core::widgets::Icon` and a tintable
  `DrawCommand::Image`, and **when it lands `DOCK_GLYPHS` becomes five `Icon`
  widgets.** **The operator's decision of 2026-09-30 forbids the placeholders as a
  final answer**, so the departure is recorded in `Out of Scope`, in the constant's
  own doc and in the handoff. **The criterion that holds the dock meanwhile is its
  geometry and its five-slot count, not its glyphs** — and saying so is what keeps
  the placeholder from being read as an asset.

- **No `Screens`, no screen stack, no transitions, and no panel dismissal.**
  `TASK_UI_PRIM_42` closes gap `#3` and is **not started**; `L2`'s transform half
  is task 36's and is given to the mesh path. **A chrome that cannot be dismissed
  is a persistent region, which is what § *Screens* says the car-status pane is**,
  so this task is internally consistent — but the five panels over it are not.

- **No status-bar state machine: no ten conditional items, no blink, no latch, no
  severity timing.** § *Top bar* lists thirteen rows of which ten are conditional,
  and § *The car-status pane* records that **three of the indicator conditions are
  distinguished by timing and one latches until an externally-triggered reset**.
  **This task builds three always-on items, two parked-only items and five
  indicator rows, and none of the state that adds or removes them** — because a
  latching tell-tale cleared by *"drive over 15 mph for a short amount of time"* is
  a state machine with **two** inputs, and `developer.md` § *Phase 2* says *"No
  abstraction before the second use. One caller is a function, not a trait."* **There
  is one caller and no input it would read.**

- **No 3-D car visualisation, no floor reflection, no hotspot callouts, no leader
  lines, no charge-port or lock glyphs.** `DEMO_APPLICATION.md` § *Open questions*
  item 5 records that the car's body *"is a required asset, not an optional one"* —
  an SVG will not carry it and *"a flat PNG will not either"* — **and the four tasks
  that could draw one (34 to 38: depth buffer, mesh buffers, `Mat4`, `Mesh`,
  `ROADOSMF`) are all not started.** **The pane's middle is therefore the map's
  surface**, which is what § *Design principles* item 2 asks for anyway (*"the map
  is the base screen; every other surface is a panel over the map"*), and the
  stand-in is named in the pane's doc.

- **No per-field staleness, no carousel paging, no per-card internal scroll, no
  drag-to-dismiss of the card strip.** These are **composite widgets 1, 2 and 3**
  in § *Composite widgets*, and composite 1's "What it demands" column names
  *"Horizontal paging with snap + inertia"* — which is `L5`, **task 46, not
  built**. **A static three-card row with a three-dot pager is the whole of the
  carousel here**, and § *Open questions* item 8 — which asks paging versus
  swipe-with-peek and records that the photo's *"3-dot pager"* evidence is
  *"**unverified**"* for count — **is left on the list, unchanged.**

- **No light/dark theme switch inside the demo tab, and no second visual design.**
  `T` switches the whole window between the two themes and the chrome follows
  through the property graph, which is what `a_theme_switch_reaches_every_chrome
  _surface` asserts. **The chrome does not carry its own appearance setting**, and
  `DEMO_APPLICATION.md` § *Design principles* records the operator's 2026-10-05
  decision that the theme switch *"is a demonstration of the existing animated
  switch, not a second visual design"*.

- **No dock edit mode, no drag between the tray and the dock, no eviction.**
  **Composite widget 10**, and § *Bottom dock* records the three behaviours that make
  the dock composite — *"Overflow is destructive, not rejected"*, *"the number of
  recent apps displayed here depends on how many apps have been added to My Apps"*,
  and *"Seat heaters … appear next to the temperature, instead of in the My Apps
  area"* — **none of which is built, and § *Bottom dock* itself says the demo should
  copy the eviction mechanic *"only if it is deliberately modelling the failure."***

- **No status-bar `PRND` selector, no drive-mode gesture, no Neutral press-and-hold.**
  § *Corrections to the first sketch* records that the `PRND` row is *"a **readout**
  in the car-status cluster, not chrome"* and that **the selector is a separate
  vertical drive-mode strip, edge-swipe summoned, hidden at highway speed**, whose
  gesture model the same row tags `[C]` — *"read in a search index, not off an opened
  page"* — and whose corpus § *Sources* records as **unreachable from this host**.
  **This task builds the readout and the strip's four targets and no gesture on
  either**, and the strip's doc repeats the `[C]` tag so a later task does not
  inherit it as a spec.

- **No clock.** The status bar's clock and ambient readings are **static strings**
  bound to a `Property<String>`, and `a_theme_switch_reaches_every_chrome_surface`
  is the test that reaches them. **A `SystemTime` read in a paint path would be a
  wall-clock read in the demo**, and `AGENTS.md` § *Rust* forbids a wall-clock
  *test* while **a still screenshot of a 4 fps application** hides what a
  per-frame cost does to a build nobody measured. **A
  moving clock also has no capture evidence**: `magick compare -metric AE` of two
  captures of a number that ticks is that **two captures of a moving number can be
  identical**, and the cure there is to read the number rather than the
  pixels — **which a wall clock would make impossible to photograph.**

- **No change to the gallery, to the tab bar, to `Page`, or to any other page.**
  `git diff` on `main.rs` adds `mod chrome;`, the constants, the nodes, the rows and
  the arm — **and touches `enum Page`, `const ALL`, `const DEFAULT` and
  `fn name` not at all**, which criterion 1 asserts. **No gallery rect moves**,
  which is what criterion 2's AE 0 checks.

- **No `ui_core` change, and `task 41`'s deliverable is used, not written.** Both
  branches of requirement 11 go through APIs that exist today:
  `Painter::backdrop` if 41 landed, `Painter::rounded_rect` always. **There is no
  third branch in which this task edits `paint.rs`**, and an implementer who finds
  themselves adding a `DrawCommand` variant is past the task's boundary
  (`developer.md` § *Stop conditions*).

- **No asset file, and no `TASK_UI_PRIM_39` pipeline. Amended 2026-10-09**, because
  the sentence was written when task 01's map drew nothing from disk: **task 01
  now reads exactly one image file**, `ui/src/ui_demo/assets/img/map_demo.png`, and it
  is **not in this repository** and not the pipeline's. **This task still adds no
  asset and reads no file** — nothing is decoded, nothing is added under
  `ui/src/ui_demo/assets/`, and **every colour in the chrome is a theme token or a
  first-principles literal** — because
  `DEMO_APPLICATION.md` § *Could not verify* records that *"**Tesla publishes no
  design tokens at all.**"* and § *Asset requirements* repeats it.

- **Found in the tree and deliberately not fixed.** `ui/src/ui_demo/src/main.rs`'s
  comment on the tab bar's construction records the **withdrawn** 2026-10-03
  decision — `DEMO_APPLICATION.md` § *Relationship to task 24* lists it as
  *"**known-stale** and is left for a code task"*, and **`TASK_UI_PRIM_43` rewrites
  that whole block when it lands**. `developer.md` § *Phase 2* refuses drive-by
  cleanups (*"Do not restructure what you were not asked to touch"*), and §
  *Relationship to task 24*'s own table says the amendment surface for that comment
  is the block, not a task file. **Recorded here so the next reader knows it was
  seen.**