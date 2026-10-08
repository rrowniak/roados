# Demo Application — Direction

**Status:** Operator decision, 2026-09-30. *Screens* and *Layout* completed
2026-10-01 from Tesla's own Owner's Manual and the operator's own photographs
of a centre display. **Revised 2026-10-03** — see § *Relationship to task 24*.
**Revised 2026-10-05** — the source-verified gap table was found to be **wrong
in two rows at the time it was written**, the two numbered gap tables collided,
and two operator decisions were taken. See § *Operator decisions (2026-10-05)*
and § *Corrections to the second gap table*.
**Category:** `TASK_UI_DEMO_n` (new)
**Depends on:** `TASK_UI_PRIM_11..23` — the shipped widgets, which is **Label**
(11), Button, Container, Slider, Toggle, Image, Progress, List/Scroll, TextInput
**and the on-screen keyboard** (19), Gauge, Chart, Dialog and Toast. Plus
`TASK_UI_PRIM_24.1..3`, **done** in `e567634`: they built the page shell this
application becomes a tab inside — `enum Page` and the three gates (paint, hit
test, focus) from 24.1, `CONTENT_TOP` and the page-local content band from
24.2, and the top tab bar from 24.3.

### What a seventh page costs

Recorded here rather than in § *Relationship to task 24*, because it is the
mechanism a `TASK_UI_DEMO_n` task has to touch and the header is where a task
author looks first. The six pages are `Pads` · `Text` · `Input` · `Controls` ·
`Data` · `Overlays` — the card of pads and the `Space` cascade; the label column
with its sizes, alignments, truncation and colour token; the text field and the
on-screen keyboard; the slider, toggle and progress bar; the gauge, chart and
image; and the dialog with the toast host.

Adding a seventh is **not** a free addition, and `ui_demo/src/main.rs` says why
in its own comments:

- **`const ALL: [Page; 6]`** (`main.rs:1700`) — a fixed-size array, so the *type*
  carries the count. A seventh changes it to `[Page; 7]`.
- **`Page::name()`** (`main.rs:1728`) — its doc comment says *"The only place the
  six names are written out"* (`:1722`). `from_name` and everything printable
  derive from it, so one misspelling is a name that `--tab=` cannot resolve.
- **`Demo::tabs`** — pairs the buttons with their pages, and the source calls the
  order load-bearing three times: `--help`, the unknown-name message, and the
  tab bar's own button order.
- **The three gates** — paint, hit test and focus each need a row.
- **The page-membership table in `Demo::new`** — **mutation-tested.** Task 24.1's
  review produced four majors that were all one finding, *a mechanism with no
  test*, found by mutation rather than by reading; the sharpest instance is
  recorded in `IMPLEMENTATION_STATE.md` as *"`Demo::new`'s page table had no
  completeness assertion at all"* — one dropped row gave **0 failed / 1814**, with
  the text column drawn on the wrong page. **24.2 found the same class again** on
  the table 24.2 introduced (`placed_handles`, 0 failed / 1817). A seventh row is
  exactly where it recurs.
- **`Page::DEFAULT` stays `Pads`** — deliberately, because it is *"the one page
  every capture taken for tasks 11 to 22 contains, so a capture that used to need
  no argument is still reproducible"* (`main.rs:1712-1716`). The demo tab must not
  become the default.

## Goal

Re-implement `ui_demo` as a Tesla-like infotainment interface after all UI
primitive tasks are complete. The demo uses mocked data, focuses on visual
quality, and demonstrates that `ui_core` is fully capable of carrying a real
automotive infotainment application.

**Revised 2026-10-03.** Not a *re*-implementation: **the operator's decision of
that date is that this application becomes one more tab of the widget gallery**,
so the gallery's page shell is built first and this direction is a page inside
it rather than a replacement for it. See § *Relationship to task 24*.

## Operator decisions (2026-09-30)

1. **All screens.** The demo includes all major Tesla screens: map/navigation,
   media player, climate controls, vehicle status, app launcher, and any others
   identified during implementation.
2. **Near-Tesla visual fidelity.** The look is "almost exact" — not pixel-exact,
   but not drifting into a simplified view. The demo should be immediately
   recognizable as Tesla-like.
3. **Real icons.** A dedicated task inventories needed icons and obtains or
   generates missing assets. No placeholder geometric shapes.
4. **New task category.** This is not one task but a new category:
   `TASK_UI_DEMO_n.md`. Most likely 30+ tasks.
5. **Map emulation.** If a real map is too difficult, emulate it in a visually
   appealing way that preserves the Tesla look.

## Operator decisions (2026-10-05)

Taken while correcting this document. Both are recorded here rather than
in-place at their point of use so that the point of use can cite them.

1. **Dark theme ships first, and both themes are required.** § *Design
   principles* had recorded dark as *removed*; that is retired. What survives it
   is kept: the theme switch is a demonstration of the **existing animated
   switch**, not a second visual design, and **both of the operator's
   photographs are the light theme** — so light is where the first-hand evidence
   is and dark is not. That asymmetry is now a cost to state rather than a
   reason to drop a theme. See § *Design principles*.
2. **Every library gap must be closed.** The 2026-10-03 decision to build the
   page mechanism in `ui_demo` and leave gaps **#3** and **#7** open in
   `ui_core` is **withdrawn**. Those two gaps are library work that blocks the
   demo like any other. § *Library gaps* carries the consequence; the four files
   that still record the withdrawn decision are listed in § *Relationship to
   task 24*.

## Scope

### In scope

- All major Tesla infotainment screens
- Mocked vehicle data (speed, battery, temperature, etc.)
- Real icons and visual assets
- **Both themes, with dark built first** — `Controls > Display > Appearance` is
  Dark / Light / Auto [A], and the operator's decision of 2026-10-05 is that both
  ship and dark is the one the demo leads with. See § *Operator decisions
  (2026-10-05)* and § *Design principles* for why that is the harder order.
- Smooth animations and transitions
- 60 FPS target
- All widgets from tasks 11–23 working together, **the on-screen keyboard
  included**

### Out of scope

- Real navigation/routing
- Real vehicle bus integration
- Real media playback
- Pixel-exact Tesla replication
- Production-quality error handling

## Relationship to task 24

**Revised 2026-10-03. This section superseded itself; both decisions are kept.**

**The 2026-09-30 decision, as recorded then.** Task 24
(`TASK_UI_PRIM_24.md`) is superseded by this direction. The existing task 24
spec describes a widget gallery; the operator has decided to replace it with a
Tesla-like demo application. Task 24 is not started and will not be started in
its current form.

**The 2026-10-03 decision, which supersedes that one.** Task 24 is **kept**,
amended, and split into `TASK_UI_PRIM_24.1..3.md`. The reasoning that made the
first decision sound has not changed — a widget gallery is not a demo
application — but **the gallery is what tasks 11–23 have been building, widget
by widget, and it is finished rather than replaced.** So the gallery gets a page
shell, and:

- **this application becomes one more tab**, not a replacement for the gallery.
  That is the operator's decision and **the shape of that tab is not settled
  here** — a `TASK_UI_DEMO_1.md` has not been written, and nothing below has
  been re-derived against a tabbed shell;
- **two library gaps stay open by that decision**: gap #3 (*no screen/navigation
  system*) and gap #7 (*no `TabBar` widget*). The gallery's page shell is built
  **in `ui_demo`**, out of `Container` + `Button`, which is what gap #7
  prescribes for the dock. A `ui_core` tab controller is a library task with
  its own cycle and was declined for now. **Withdrawn 2026-10-05** — see
  *Operator decisions (2026-10-05)* item 2. Both gaps must now be closed in
  `ui_core`, so what `24.1..3` built is the demo-level mechanism and **not a
  substitute** for the library one;
- **`--tab=<name>`** lands on a page without a click, which is the only route to
  a capture of one page on this host: pointer injection has never delivered an
  event to the window, and keyboard injection delivered exactly one in this
  project's history.

**One tension to settle when the `TASK_UI_DEMO_n` tasks are written.** The
*Controls panel* section below records, from photograph `02`, *"There is no
underline or tab bar anywhere"*. **That is evidence about Tesla's own interface
and it remains true of Tesla.** The gallery's tab bar is a demo affordance for
switching between widget pages and says nothing about how the Tesla screens are
navigated — so no override is needed here, and none is recorded. **It will need
one** if a `TASK_UI_DEMO_n` task ever puts a tab bar on a Tesla surface, and that
is the sentence to amend when it does.

**Four places still record the withdrawn 2026-10-03 decision.** Recorded here so
the divergence is visible rather than silent, per the repository's rule that the
artifact a claim contradicts is the one that is wrong. **None of them is
amended by this pass** — this direction is the owner and the others are history:

| File | Records |
|---|---|
| `TASK_UI_PRIM_24.md:263-266`, *Out of Scope* | *"`DEMO_APPLICATION.md` gaps #3 and #7 stay open. Closing #3 is a library task with its own cycle, and the operator chose the demo-level route on 2026-10-03"* |
| `TASK_UI_PRIM_24.3.md:13-21` and `:212-214`, *Context* and *Out of Scope* | *"The operator chose the demo-level route and gaps #3 and #7 stay open"* |
| `IMPLEMENTATION_STATE.md:4336` and `:5606` | *"Two library gaps stay open by this decision"* / *"Three library gaps stay open by that decision"* |
| `ui/src/ui_demo/src/main.rs:3247`, a comment on the tab bar | *"`DEMO_APPLICATION.md` gaps #3 and #7 stay open and this is what gap #7 prescribes"* |

The last is **known-stale** and is left for a code task: it is a source comment
asserting a decision that has been withdrawn.

## Design principles

1. **Tesla look and feel** — card-based UI, minimal chrome, large touch targets
2. **Map-centric layout** — the map is the base screen; every other surface is a
   panel *over the map*, dismissed by dragging down
3. **Bottom dock** — climate and volume always accessible, with a
   user-configurable middle
4. **App launcher** — grid of icons for all major functions
5. **Smooth animations** — all transitions animated, 60 FPS
6. **Automotive constraints** — glanceable, one-hand operation, 44dp touch
   targets

**Revised 2026-10-01** against the research recorded below. Three of these
were assumptions, and two of them were wrong:

- **"Dark theme" is required, and it ships first.** This reverses the
  2026-10-01 correction recorded here before, which had removed it — see §
  *Operator decisions (2026-10-05)* item 1. Tesla ships both, and
  `Controls > Display > Appearance` is Dark / Light / Auto [A]. **What the
  earlier correction got right survives it:** the theme switch is a
  *demonstration of the existing animated theme switch*, not a second visual
  design. And the evidence is still asymmetric — **both of the operator's own
  photographs are the light theme**, a pale, low-contrast map with white cards —
  so light is the theme with first-hand evidence behind it and dark is not. That
  is a cost to state, not a reason to drop a theme: every value in a dark theme
  is a first-principles choice, on the same footing as *"Tesla publishes no
  design tokens at all"* below. Note too what the light theme costs, because it
  is the one the photographs show: white cards on a near-white map, legible only
  because the map is washed out to near-invisibility in `01`.
- **"Bottom dock — media player and climate" is right for the wrong reason, and
  the correction itself needed one.** The media **player surface** — the
  strip ↔ panel — is *not* a dock item: it is a surface drawn over the map,
  which § *Screens* already records, and its miniplayer is reached from the
  car-status pane's carousel. **The media *shortcut* is a dock item**, though,
  and the first version of this correction denied it: the manual's own 13 dock
  regions name **region 7 as the "media player shortcut"**, and photograph `01`
  shows the `▶` glyph in the dock — both reproduced in § *Bottom dock* below.
  So the distinction this correction draws is **surface versus shortcut**, and
  the enumeration of the dock's fixed items is the manual's 13 regions, not
  *"Controls and the climate setpoint"*. Amended 2026-10-05; the original claim
  is kept here because a demo that built the player into the dock would be
  wrong, and the correction is what says so.
- **"44dp touch targets" is this project's number, not Tesla's.** Nothing in
  Tesla's documentation states a touch-target size. Keep 44dp as a roados
  decision; do not attribute it to Tesla.
- **Added: chrome is translucent over a live scene.** This is the single
  structural fact the original principles missed, and it is why **L1** is
  critical. See composite widget 5.

## Asset requirements

A dedicated task will inventory all needed assets:

- App icons (navigation, media, climate, vehicle, settings, etc.)
- Status bar icons (time, temperature, connectivity, battery)
- Control icons (play, pause, skip, volume, fan, seat heating, etc.)
- Map elements (roads, route line, car marker, POI icons)
- Vehicle images (for status display)
- Album art (for media player)

**Added 2026-10-01, from the completed Layout section.** Tesla publishes **no
design tokens at all** — no colours, no spacing, no radii, in any manual or
release note. Every visual value in the demo is therefore a first-principles
choice, and the asset inventory must produce them rather than transcribe them.
Two consequences for this task:

- **The icon inventory is much larger than the list above.** Every dock item,
  every conditional top-bar item, every Controls tab row, and ~20 indicator
  lights each need a glyph. The Controls tab list alone carries a distinct icon
  per row (car, lightning bolt, steering wheel, padlock, lamp, seat, screen,
  clock, warning circle, wrench, download arrow, navigation arrow — all
  readable in `tmp/tesla_screens/02_car_screen.png`).
- **Some widgets need *generated* glyphs, not icon files.** The seat widget's
  squiggle count encodes the level and its colour encodes heating vs cooling;
  the indicator column uses colour to encode severity and *timing* to encode
  fault-vs-condition. Those are drawn from state, not selected from an atlas.

## Library gaps

Identified 2026-09-30 by comparing this document's requirements against the
shipped widgets (tasks 11–13: Label, Button, Container — tasks 1–10 are the
arena, property system, render pipeline, layout, theme, animation and input) and
the planned widgets (tasks 14–23: Slider, Toggle, Image, Progress, List/Scroll,
TextInput **and the on-screen keyboard**, Gauge, Chart, Dialog, Toast). Each gap
must be addressed — by a `TASK_UI_PRIM_n` amendment or a `TASK_UI_DEMO_n` task —
before or during the demo implementation, **and as of 2026-10-05 none may be left
open**: see § *Operator decisions (2026-10-05)* item 2.

**Re-derived 2026-10-01** against the shipped code and against the completed
*Layout* section below. The first eight rows are the original 2026-09-30
assessment, kept so the original reasoning is auditable; **rows 5 and 8 were
checked against the source and are wrong**, and are marked. **Rows 3 and 7 were
open by decision on 2026-10-03 and are no longer** — see § *Operator decisions
(2026-10-05)*. Eleven further gaps the Layout work exposed are in *Gaps this
layout exposes in `ui_core`*, further down.

**These eight rows keep their numbering**, because four files outside this one
cite it by number: `TASK_UI_PRIM_24.md:142` and `:270` cite rows 7 and 8,
`TASK_UI_PRIM_24.3.md:15`, `:17`, `:21`, `:216` and `:218` cite rows 7, 3, 8
and 4, `IMPLEMENTATION_STATE.md:4336` and `:5606` cite rows 3 and 7, and
`ui/src/ui_demo/src/main.rs:3247` cites rows 3 and 7. The second table is
therefore cited as **L1..L11** instead, so that a bare "gap N" in this document
cannot mean two different rows — which it did, five times, before this pass.

| # | Gap | Severity | Blocks |
|---|---|---|---|
| 1 | **Map widget** — no map renderer exists or is planned. The demo's centerpiece. | Critical | Map/navigation screen |
| 2 | **Grid layout non-functional** — `Grid` mode lays out no children and reports no rects; `wrap` is accepted and not honoured. | High | App launcher screen |
| 3 | **No screen/navigation system** — no screen stack, tab controller, or transition system in the library. **Was left open by decision on 2026-10-03; that decision is withdrawn 2026-10-05** — this gap must now be closed in `ui_core`, so it blocks the demo like any other. What `TASK_UI_PRIM_24.1` built is the *demo-level* mechanism — `enum Page`, a page-membership table, and three gates (paint, hit test, focus), inside one binary — and **that was never this gap closed**, as the row itself said at the time. | High | Multi-screen app structure |
| 4 | **No Icon widget** — `Image` (task 16) displays textures but icons need vector rendering, theme tinting, and uniform sizing. **Raised 2026-10-01 from Medium to High**: the completed Layout section needs an icon for every dock item, every top-bar status item, every tab row and every indicator light — and `Polygon` is convex-only with no bezier (**L10**), so this is the largest unsupported item in the design. | High | Visual quality — "real icons" requirement |
| 5 | **Clipping has no owner.** The clip rect *is* computed, carried on the batch and set on the GPU as a scissor — but the clip is supplied by the demo's frame loop for the `List` alone, so there is no per-node clipping in the widget system. Two doc comments assert the opposite of the code beside them. **Corrected 2026-10-01; the original "never set on the GPU" claim was false.** | High | Map viewport, scroll view clipping, card page edges |
| 6 | **No Card widget** — `Container` can be stretched to cover this, but a dedicated card with elevation/shadow matches the Tesla design language better. | Low | Visual polish |
| 7 | **No TabBar/Dock widget** — the bottom dock can be built from `Button` + `Container`, but a dedicated widget with active-state indication and icon+label layout is the right primitive. **Was left open by decision on 2026-10-03; withdrawn 2026-10-05** — the gap must now be closed in `ui_core`. What `TASK_UI_PRIM_24.3` built is the gallery's top tab bar from exactly `Button` + `Container`, which is the prescription this row states and **not a closure of it**. **The "active-state indication" half has no widget behind it either** — `Button` carries `hovered`, `pressed`, `disabled`, `focused` and `activatable` (the fifth exists because one bit cannot answer both "does it draw the ring" and "may it be activated") and **no `selected`**, so the selected tab is a `background`/`foreground` swap the demo owns. Checked in `ui_core/src/widgets/button.rs` on 2026-10-05. A dedicated widget is still the right primitive for a dock that wants icon+label layout. | Low | Bottom dock implementation |
| 8 | **Transform transitions** — screen transitions need translation, scale, and opacity animation support. The animation system handles property interpolation but transforms are not implemented in the render pipeline. **Confirmed 2026-10-01, and worse than stated: `Transform` is `Interpolate`-able, so it can be animated and then never drawn. `DrawCommand` has no transform field and there is no matrix or `u_model` uniform.** | **Critical** | Screen transition animations |

## Task structure

The new category `TASK_UI_DEMO_n` will be defined incrementally. The first
tasks will be:

1. Asset inventory and generation
2. Map emulation approach
3. Screen-by-screen implementation

Each task will follow the same workflow as `TASK_UI_PRIM_n`: developer →
review → operator commit.

Gap closure tasks (see *Library gaps* above) may be interleaved with demo tasks
when a gap blocks a demo screen — but since 2026-10-05 they are **mandatory
rather than discretionary**, and there are more of them than this document first
suggested. Merging the duplicates across both gap tables (`#2` with **L3**,
`#8` with **L2**), the library work this makes a precondition is roughly **ten
`TASK_UI_PRIM_n` tasks**, five of them Critical or High:

| Severity | Gaps | Nature |
|---|---|---|
| **Critical** | `#8`/`L2` transform, **L1** colour capture + backdrop API | render pipeline |
| **High** | `#2`/`L3` grid, `#3` screen system, `#4` icon, `#5` clip ownership, **L4** `LongPress`/`Swipe`, **L5** horizontal scroll/momentum/snap, **L6b** cross-widget mode | layout, input, widget system |
| **Low / Medium** | `#6` card, `#7` `TabBar`, **L7** margin/gap, **L8** text width, **L9** theme scoping, **L10** `Polygon` | widget vocabulary |

Gap **#1**, the map widget, stays a `TASK_UI_DEMO_n` item — the map is the demo's
own asset and no library owns it.

## Open questions

1. **Photo provenance — the first question because everything below rests on
   it.** `tmp/tesla_screens/*.png` are the best evidence in this repository and
   are **untracked**, undated, and their software version is not legible. Should
   they be committed as a reference set, and should the demo be pinned to the
   layout they show rather than to whichever software is current when the tasks
   are written? Until this is answered no future agent can reproduce the primary
   evidence for the whole *Layout* section below, and a re-derivation would
   produce a different document.
2. What map data source to use for the emulation? (procedural, hand-drawn, or
   simplified real data?)
3. How to handle the Tesla logo and branding? (avoid trademark issues)
4. What vehicle model to display in the status screen? (Passat B5.5 or a generic
   car?)
5. **The car's body is a required asset, not an optional one.** The Layout
   section needs a rendered vehicle that hotspots anchor to, whose regions
   change colour in Track Mode, and that is *reflected on the floor* in the
   operator's own photographs. An SVG of a generic car, or a flat PNG, will not
   carry it. Which?
6. **What is the backdrop task's category?** Closing **L1** means an `GL_RGBA8`
   colour attachment, a public entry point and a rect-scoped capture. That is
   **render-pipeline work**, of exactly the kind task 22 already did once when it
   built the FBO blur — so **`TASK_UI_PRIM_n` is the recommendation**, and
   `TASK_UI_DEMO_n` only if the operator would rather keep it inside the demo
   task. Since 2026-10-05 the gap cannot be deferred either way, so this is a
   question of *which sequence owns it*, not *whether*.
7. **What is the seventh page called, and its `--tab=` value?** What a seventh
   page has to touch is recorded in the header; the name is a decision this
   direction has no authority to make, and `Page::name()` is the single place the
   spelling lives.
8. **Card carousel depth.** Two cards are visible side by side in photo `01`
   with a 3-dot pager. Should the demo implement paging (the real behaviour)
   or a swipe-with-peek (cheaper, and what the photo's evidence supports
   equally well)?

**Answered 2026-10-05 and removed from this list: *Light or dark first?*** Dark is
required and ships first. See § *Operator decisions (2026-10-05)* item 1.

## Screens

Tesla does not have "screens" in the sense this document originally assumed.
It has **one persistent map screen**, and every other surface is a **panel
drawn over the map**. The manual states this for Controls verbatim: *"The
Controls screen appears over the map."* [A] Closing is always *"drag it
downward"* [A] — a dismiss gesture, not a route change.

The car-status pane is the other half of that model. It is **always on
screen**, is not created by any tap, and has three mutually exclusive states
the manual names explicitly: **parked**, **driving (or ready to drive)**, and
**charging** [A].

| Surface | Kind | Over the map? | Dismissed by |
|---|---|---|---|
| Map | base screen | — | — |
| Car-status pane | persistent region of the map screen | part of it | never; resize drag only |
| Controls | panel | yes | *"Swipe to close"* [A] |
| Climate popup | popup | yes | not documented; implied tap-outside |
| Media player | strip ↔ panel, one object | yes | drag *down* to minimise — the other direction is expand [A] |
| App tray | card | yes | *"To close an app, drag it downward"* [A] |
| Popup message / alert | bottom slot | yes | *"swipe it downward"* [A] |
| Theater, Browser | panel, may go full-screen | yes | minimise button [B] |

**Every dismissal is a drag, and the direction is part of the semantics.**
Controls closes on a swipe; the media player's downward drag *minimises* it
while upward *expands* it; an alert is swiped down and gone. The demo needs a
drag-to-dismiss gesture with a direction that means something, not a close
button — which is also the one gesture `ui_core` currently has no consumer for.

### Screen states of the car-status pane

Read from the manual's three-state split [A], and cross-checked against
`tmp/tesla_screens/01_welcome_screen.png` and `02_car_screen.png`, which the
operator captured from a real centre display.

- **Parked** — drive mode, estimated range, and a view of the car with
  tappable buttons for the trunks and charge-port door. Indicator lights
  **flash briefly at power-up** as a self-test and then go out [A].
- **Driving** — speed, a real-time road visualisation from the Autopilot
  cameras, a power meter, detected other cars, the speed-limit sign, range,
  and the set cruising speed [A].
- **Charging** — the charge-port lamp protocol and charge state [A].

## Layout

### Sources and how to read this section

Three classes of evidence, tagged per `.ai/protocols/evidence.md`:

- **[A] manual** — Tesla's own Model 3 / Model Y Owner's Manual, read during
  this session via the `rollout-tesla.com` mirror of `tesla.com/ownersmanual`
  (Tesla's CDN 403s from this host). The mirror serves the same GUID-addressed
  pages as the canonical site; the Manual is © Tesla and is the vendor's own
  description of its own product. **Within this class, a distinction is worth
  keeping:** §Touchscreen, §Car Status, §Operating Climate Controls, §Media,
  §Tire Care and the Model Y §Controls Overview were **opened and read**. The
  §Shifting quotes and the visualisation-resize sentence came from
  **tesla.com's own PDF text as surfaced in a search index**, quoted verbatim
  but not read off a page I opened. Nothing in this section is third-party prose
  about Tesla's UI.

  **Two corrections to how those two sentences are tagged, made 2026-10-05.**

  First, the §Shifting quotes and the visualisation-resize sentence were
  formerly marked **`[A*]`**, a tag defined nowhere in
  `.ai/protocols/evidence.md` and used nowhere else in this repository. The
  protocol defines `[A]`, `[B]` and `[C]`, and says the only permitted extra
  qualifiers are inline — `staleness` and `support` — so a fourth letter-tag is
  not available. `[A]` does not fit either, because it requires that *"the
  evidence line names the file and the line"* and the rule adds *"Never upgrade a
  tag without opening the source yourself"*, which was not done. **They are now
  `[C]`**, whose definition begins *"unverified, or snippet-level. A search
  result"*, and the protocol anticipates the case exactly: *"If the ladder cannot
  be climbed, that is the finding — and the tag is `[C]` no matter how
  authoritative the top of the ladder looks."*

  Second, the **Model Y §Controls Overview was read through a third-party
  mirror**, not from Tesla. It was tagged `[A]` and is now **`[B]`**, which is
  defined as *"well-maintained project documentation … not re-verified against
  the primary document"*. Its two citations are the *Corrections* row on the
  car icon and the *Controls panel* section.

  **The demotion is structural, not an oversight, and cannot be repaired from
  this host.** Re-tested 2026-10-05: `tesla.com` returns **403**; and
  `rollout-tesla.com` is a passthrough to the same CDN, so its directory paths
  return **403** and a real GUID page returns **Akamai's "Access Denied"** page.
  An *unknown* GUID path returns HTTP 200 and ~39 KB of HTML — but that content
  is an **unrelated third-party site**, not the manual, which is a trap worth
  recording: a reachability probe that reports 200 may have found nothing of
  Tesla's at all. **There is no way to open these pages from here**, so any claim
  resting on them stays `[C]` until someone with different network egress reads
  them, or the corpus is mirrored locally.
- **[A] photo** — `tmp/tesla_screens/01_welcome_screen.png` (1136×715) and
  `02_car_screen.png` (1141×642). **Photographs of a real display, not
  screen captures**: perspective-distorted, and no software version is
  legible in either frame. They are first-hand evidence of layout and of
  what exists, at `[A]`-observational, and are **not** evidence of exact
  colour, exact pixel metrics, or which software build produced them.
- **[B] release notes** — every claim about *what changed when* comes from
  third-party transcriptions of in-car artefacts. `doc/findings/tesla-release-aggregator-reliability.md`
  already established at `[A]` that those transcriptions are unreliable **in
  their gating fields**, so dates and hardware applicability here are `[B]`
  even where the feature text is `[A]`-as-transcribed. Version attributions
  in this section are indicative, not authoritative.

### Corrections to the first sketch

The operator's original notes were mostly right, and two were confidently
wrong. Both corrections change what has to be built, so they are recorded
rather than silently overwritten.

| First sketch | What it is | Evidence |
|---|---|---|
| Top bar: `PRND` first | **Right that it is a horizontal letter row and it is at the left. Wrong that it is a top-bar *control*.** The `P R N D` row is a **readout in the car-status cluster**, not chrome. The *selector* is a separate **vertical drive-mode strip on the left screen edge**, edge-swipe summoned, hidden at highway speed, and **Neutral is not on it** — it is a press-and-hold inside Controls. | [C] manual §Shifting, snippet-level: *"Swipe up for Drive, swipe down for Reverse, or press the drive mode strip for Park… To shift into Neutral, open Controls, then press and hold the Neutral icon… the drive mode strip is hidden when driving at highway speeds."* Quoted verbatim from Tesla's own PDF text but **read in a search index, not off an opened page** — see § *Sources*. **The gesture semantics below rest on this alone.** [A] photo `02`: the strip is a dotted vertical track with a grey `↑`, visible at the left edge |
| Bottom bar: `72` = "maybe temp in F" | **A temperature, but the cabin setpoint, not outside.** Outside temperature is a different widget in the top bar. `72` is setpoint; `65°F` in the same frame is ambient. The dock shows **no unit and no degree sign**. | [A] manual §Touchscreen: *"Climate controls (driver): Use the left and right arrows to decrease/increase cabin temperature."* [A] photo `01`: dock reads `< 72 >`, top bar reads `65°F` |
| Bottom bar: calendar icon | **True of this vehicle, false as a fixed slot.** Calendar is an app the user pinned into *My Apps*. The dock's fixed slots do not include it. | [A] manual §Touchscreen's 13-region layout: slot 9 is *My Apps*, and Calendar is never a named region. [A] photo `01` shows it among the pinned app icons |
| "Car screen gets squashed into left pane" | **Correct, and worth keeping — but it is the Controls *panel* that appears, and it is reached by the car icon or by an edge swipe.** | [B] manual §Controls Overview (Model Y, **read through a third-party mirror, so not re-verified against the primary document** — see § *Sources*): *"Touch Controls on the bottom corner of the touchscreen… The Controls screen appears over the map… You can also access Controls by touching anywhere on the side of the touchscreen closest to the driver and swiping open."* [A] photo `02` |
| Car-screen tabs, 12 names, no icons | **The set is close; the order is not the sketch's; every row has an icon; the list scrolls.** Observed order: Controls · Dynamics · Charging · Autopilot · Locks · Lights · Seats *(NEW badge)* · Display · Schedule · Safety · Service · Software · **Navigation** *(cut off — the list continues)*. The manual's own chapter ordering confirms Dynamics, Autopilot, Locks, Lights, Seats, Display, Schedule, Safety, Service, Software are all real top-level categories, and adds Mirrors, Navigation, Wi-Fi, Bluetooth, Audio, Outlets & Mods, Trips. | [A] photo `02`, read icon by icon. [A] manual cross-references. Renames worth knowing: *Pedals & Steering → Dynamics* (2024.14), *Autopilot → Self-Driving* (2026.2) — both [B] |
| Left panel: "automatic lights on icon" | **There is no "automatic lights armed" indicator.** Exterior lights default to Auto every drive with no persistent tell-tale. The glyph in the photo is one of the *high-beam* states — blue beams + `A` = Adaptive Headlights armed and high beams on; grey = armed, dimmed because light is ahead. | [A] manual §Car Status lists the five lighting states; §Lights: *"Exterior lights… are set to AUTO each time you start Model 3… If you change to a different setting, lights always revert to AUTO on your next drive."* |
| Left panel: "seatbelt not fasten red icon" | **Right, and it is one of three channels, not one.** A red occupant-with-belt indicator light in the cluster, **plus** a bottom popup whose text and per-seat tap-to-mute are separate, **plus** the "Fasten Seatbelt" label under the seatbelt graphic. Tapping the offending seat on the popup disables the reminder for the drive and **replaces the icon with a seat glyph**. | [A] manual §Car Status indicator list, §Touchscreen §Popup Messages, §Seat Belts |

### The persistent chrome

#### Top bar

**Order, parked, current software.** The middle is deliberately empty; the
driver-side items are grouped left of it and the clock/ambient group right of
it. [A] photo `01`, read left to right:

```
PRND · [battery 70%]  ………  [open padlock] [👤 Guest] [🔴]  ………  [2:37 pm] [☀ 65°F]  ………  [PASSENGER AIRBAG OFF]
```

- `PRND · battery%` sits at the **extreme left, outside the status-bar band**
  — it belongs to the car-status cluster, not to the chrome.
- **The bar is not a fixed icon set; it is populated by vehicle state.**
  Full documented inventory [A], all conditional except three:

  | Always | Condition | Icon |
  |---|---|---|
  | yes | always | open/closed padlock — *"Touch to lock/unlock all doors and trunks"* |
  | yes | always | clock — *"Your vehicle automatically updates the time"* |
  | yes | always | front passenger airbag status badge |
  | no | **parked only** | person + profile name — *"Displays… only when Model 3 is parked"* |
  | no | **parked only** | Sentry Mode — *"Available when Model 3 is parked"* |
  | no | HomeLink in range | house |
  | no | poor AQI only | AQI reading |
  | no | on Wi-Fi | Wi-Fi |
  | no | on cellular | signal bars — *"Touch this icon for quick access to Wi-Fi settings"* |
  | no | cellular unavailable | signal bars with a slash |
  | no | update available | amber clock, replaced by a green download icon while downloading [B] |
  | no | app accessing GPS | phone with an arrow |
  | no | emergency | SOS |

  The red circle in photo `01` sits where Sentry is documented and reads as
  recording-active; it is **not separately documented** and is recorded here
  as observed, not as a specified state.

- **Profile switching is parked-only in the bar** and available from the top
  of *any* Controls screen [A]. Save confirmation is *"a green check mark
  appears next to the driver profile icon"* [A] — an annotation on the chrome
  that reports a state change two screens away.

#### Bottom dock

Icon-only, flat black bar, no labels, **no active-state highlight** [A] photo
`01`. Observed left to right:

```
[car]  < 72 >   [📅27] [Spotify] [▶] [🎬] [📹•]  […]  [🎲•]  ………  ‹ 🔊 ›
        ↑          └──── pinned apps (My Apps) ────┘  ↑  └ recents ┘   volume
     Controls   climate            App Launcher
```

The manual's authoritative slot list is 13 regions [A] §Touchscreen, in
order: status bar · navigation · car status · drive-mode strip · **Controls**
· **climate controls (driver)** · media player shortcut · full-screen Park
view · **My Apps** · **App Launcher** · **Recent App(s)** · **climate controls
(passenger)** · **Volume Control**. Regions 6, 9, 12 and 11 are conditional;
this vehicle shows 6 and not 12, i.e. temperature Split is off.

Three behaviours here are the ones that make the dock a composite widget
rather than a row of buttons:

1. **Recent Apps is a function of My Apps occupancy.** *"The number of recent
   apps displayed here depends on how many apps have been added to My Apps. If
   you add the maximum number of apps to My Apps, only the most recent app
   displays."* [A] The recents strip's width is computed from the pinned
   strip's fullness, and full occupancy collapses it to one.
2. **One control type refuses to live in the dock at all.** *"Seat heaters
   selected from the app tray appear next to the temperature, instead of in
   the My Apps area."* [A] A drag that works for every icon is overridden for
   one.
3. **Overflow is destructive, not rejected.** *"When you've added the maximum
   number of apps… adding an additional app removes the rightmost app."* [A]
   The demo should copy the eviction *mechanic* only if it is deliberately
   modelling the failure.

#### Drive-mode strip — the left edge

Not chrome. A **vertical dotted-grid track** inset against the left screen
edge, summoned by an edge swipe *"from the edge of the touchscreen towards
the passenger"*, auto-hiding at highway speed, with a car silhouette, a grey
`↑`, a grey `↓`, a bold `P`, and the word `HOLD` — [C] the manual quotes for
the gestures, **snippet-level, not read off an opened page** — and [A] photo
`02` for the form: a dotted vertical track with a grey `↑` against the left
edge. **One axis, three targets, three different gestures** — swipe up, swipe
down, press — plus press-and-hold for the emergency stop, plus a fourth gear
reached somewhere else entirely. **The whole gesture model is `[C]`** and rests
on that one snippet; it must be re-verified against an opened page before a
`TASK_UI_DEMO_n` task treats it as specified.

#### The alert channel — the bottom slot

*"Popup messages appear at the bottom of the touchscreen. For example, a seat
belt reminder appears if a seat belt is unfastened in an occupied seat, an
alert appears to notify you of an incoming phone call, a text message appears
(when applicable), and voice commands appear when in use. If applicable, touch
options from these popup messages… To dismiss a popup message, swipe it
downward."* [A]

One slot, four structurally different producers, dismissal by gesture only, and
an archive behind a bell: *"You can view a list of vehicle alerts and
notifications by touching the bell icon at the top of Controls."* [A] Stacking
behaviour is **not documented** — see *Could not verify*.

### The car-status pane

Left ~40% in photo `02`, resizable: *"You can expand/condense the
visualization by dragging the car status area from side to side. Expanding the
visualization displays more details about the roadway and its surroundings,
including road markings, stop lights, objects (such as trash cans and poles).
You can pinch to zoom in or out."* **[C]** — Tesla's own wording, read in the
indexed text of `tesla.com/ownersmanual/model3/en_us/Owners_Manual.pdf`
rather than off an opened page, and **the corpus is unreachable from this host**,
so the demotion is structural; see § *Sources*. This sentence is the **only**
source for **composite widget 16**, the two-axis reshape, which is therefore
snippet-level rather than specified. The **behaviour** is independently visible
at `[A]` in photo `02`: the pane is narrower there than in photo `01`, because
Controls is open.

**Vertical structure**, from photo `01` and `02`:

```
┌ drive-mode strip (left edge, own layer) ────────────────────────────┐
│  ┌ indicator column ─┐                                              │
│  │ seatbelt graphic  │   ┌───────── map, full-bleed ───────────┐   │
│  │ "Fasten Seatbelt" │   │                                      │   │
│  │ 🟢 high-beam A    │   │        3-D car, floor reflection      │   │
│  │ 💡 low beam       │   │   ┌──Open Frunk──┐    ┌─Open Trunk─┐   │   │
│  │ 🔴 seatbelt       │   │   │  callouts w/  │    │ callouts  │   │   │
│  └────────────────────┘   │   │  leader lines│    │            │   │   │
│                           │   🔓 (lock, floating above roof)      │   │
│                           │   ⚡ (charge port, rear-left)         │   │
│                           └──────────────────────────────────────┘   │
│  ┌ Tire Pressure card ─┐┌ Navigate card ┐┌ Start FSD (greyed) ┐     │
│  └────────── ──────────┘└───────────────┘└────────────────────┘      │
│                        · · ·   (page indicator)                      │
└──────────────────────────────────────────────────────────────────────┘
```

**The indicator column is a severity-ranked list, not a status strip.** The
manual enumerates ~20 conditions with their colours and their *timing*
semantics [A] §Car Status:

- **Red** — brake fault, parking brake applied, **seat belt unfastened in an
  occupied seat**, airbag fault, door or trunk open, system failure
- **Amber** — brake booster, ABS (brief flash at startup), parking-brake
  electrical, **tire pressure out of range**, ESC active (flashing), ESC off,
  power limited
- **Green** — parking lights, low beam, ready to drive, battery low
- **Blue** — high beam, high beam with Adaptive Headlights armed, snowflake
  (battery too cold)
- **Grey** — Adaptive Headlights armed but dimmed, Vehicle Hold, pedestrian
  warning paused

Three of these are distinguished **by timing, not colour**: ABS flashes once at
startup then faults if it stays; ESC flashes *while actively* correcting and
goes solid if it is a fault; the tire-pressure tell-tale is **steady for low
pressure and flashing for a sensor fault**. The tire light also **latches**: it
does not clear when you inflate, it clears only once you *"drive over 15 mph
(25 km/h) for a short amount of time to activate the TPMS"* [A]. That is a
state machine with an externally-triggered reset, drawn in the chrome.

### The Controls panel

**Two panes inside the panel, and no car in it.** *"1. List of available
settings. When you select an item from this list, its associated settings
display on the right side of the screen. 2. Settings area."* [B] Model Y
manual §Controls Overview — **read through a third-party mirror**, so `[B]` and
not `[A]`; see § *Sources*. The 3-D car is in the *car-status pane*, not here —
the two are separate regions and conflating them is the mistake the first
sketch made.

```
┌ Controls header ──────────────────────────────────────────────────┐
│ 🔍 Search Settings                    👤 Guest  ⌂  🔔  ᛒ  ▂▄▆ │  [A] photo 02
├──────────────────┬─────────────────────────────────────────────────┤
│ ▣ Controls  ◀sel │  [segmented] Off │ Parking │ On ◀sel │ Auto  [ (D) │  fog = solid blue
│ ▤ Dynamics       │  ┌────────┬────────┬────────┬────────┐             │
│ ⚡ Charging       │  │Fold Mir│Child Lk│Window  │Glovebox│            │
│ ⊙ Autopilot      │  └────────┴────────┴────────┴────────┘             │
│ 🔒 Locks         │  [segmented] Off◀sel │ Auto │ I │ II │ III │ IIII   │
│ ☀ Lights         │  ┌────────┬────────┬────────┐                     │
│ 🪑 Seats  [NEW]  │  │Mirrors │Recordng│Car Wash│  2×3 grid,           │
│ ▭ Display        │  ├────────┼────────┼────────┤  tiles carry state   │
◷ Schedule        │  │Steering│Sentry• │Neutral │  (Sentry shows a dot)│
│ ⓘ Safety         │  └────────┴────────┴────────┘                     │
│ 🔧 Service       │  ──────────────────────────  ☀        [ Auto ]    │  brightness
│ ⬇ Software       │                                                │
│ ▲ Navigation …   │                                                │
└──────────────────┴─────────────────────────────────────────────────┘
```

The layout grammar of that right pane is **four cell archetypes and nothing
else** — and the demo needs all four:

1. **Segmented control**, selected cell = grey fill, e.g. exterior lights
   `Off │ Parking │ On │ Auto` and wipers `Off │ Auto │ I │ II │ III │ IIII`.
   Two of these sit on one screen with *different* arities (4 and 6).
2. **Action tile** — icon + label, sometimes with a state dot (Recording,
   Sentry) and sometimes with a **secondary line** (`Child Lock / off`,
   `Neutral / Hold`).
3. **Solid-colour action button** — Tesla blue, white glyph, for *on* actions
   (fog lights, `Auto`). Visually distinct from a selected segment on purpose.
4. **Slider** — one, wide, with an inline glyph.

The list scrolls, the selected row is a **white rounded pill**, and a row may
carry a **badge** (`Seats` → `NEW`). There is no underline or tab bar anywhere.

### Climate — three surfaces

The single most instructive control cluster in the whole interface, because
the *same* controls appear at three different levels of detail.

- **Dock** — `< 72 >`, chevrons only, setpoint only.
- **Popup** — *"Touch the temperature arrows on the bottom of the touchscreen
  to display a popup"* [A]. Four items [A] §Operating Climate Controls: a gear
  icon into the full screen; a seat control that is *"Enable or disable heated
  or cooled front seats"*; front and rear defroster; and *"Modify the cabin
  temperature by dragging the slider. You can also enable temperature
  splitting."* So the popup's slider is **blue→red-graded**, draggable, with a
  round thumb, and carries a **`Split`** toggle at its end — while the number
  stays behind it on the dock. Split is what causes regions 12 of the dock to
  appear at all.
- **Full screen** — 16 documented controls [A], including a power button,
  three-way airflow distribution, a `Front │ Rear` cabin selector,
  `Schedule`, and a fan-speed slider.

The fan slider is the cleanest example of a control that is *not itself*
mode-stable: *"Use the slider to adjust the fan speed. **When in Auto, the fan
speed levels change to Low/Medium/High.**"* [A] Same widget, same gesture,
different enumeration. And its side effects are coupled: *"Adjusting the fan
speed may change the selected setting for how air is drawn into Model 3."* [A]

The defroster is a three-state button that restores other controls:
*"Touch once to defog the windshield (the icon turns blue). Touch a second
time to defrost… Touch a third time to turn off and **restore the air
distribution, heating, and fan to their previous settings**."* [A]

The seat widget encodes level in the *count of squiggles* and direction in
their *colour*: *"The seat operates at three levels from 3 (highest) to 1
(lowest). The seat icon displays twisting lines that turn red (heating) or
blue (cooling) corresponding with the set level."* [A] No numeric readout, no
cabin diagram.

### Media — two states, and a strip that outlives its app

*"You can drag Media Player upward to expand it (allowing you to browse), and
downward to minimize it so that just the Miniplayer displays. The convenient
Miniplayer, which occupies the least amount of space on the touchscreen,
displays what's currently playing and provides only the basic functions
associated with what's playing."* [A]

- The **source picker is a dropdown inside the player**, not a screen:
  *"Instead of launching a different media app, you can change the source from
  within the Media Player screen by choosing a source from the dropdown list."*
  [A]
- **Volume is chevrons**, read directly: *"Touch the `<>` arrows associated
  with the speaker icon on the bottom corner of the touchscreen."* [A] A
  press-and-hold overlay slider also exists [B] — but the manual documents only
  the chevrons, so the chevrons are the specified path.
- **Hiding a source removes it from two places at once**: *"Once hidden, the
  media source does not appear on the drop down list in Media Player, nor will
  it appear in the app tray when you touch the App Launcher."* [A]
- **Audio balance is a spatial pad, not a slider**: *"Balance: Drag the center
  circle to the location in Model 3 where you want to focus the sound."* [A]
- **The minimized player is still the transport control for another app**:
  *"When you play audio through the web browser and then minimize the browser,
  Model 3 continues the browser audio in the background. You can pause or play
  the browser audio through the media Miniplayer. If there was media playing
  before the browser audio began, the media resumes after you pause or end
  browser audio."* [A] A one-slot resume stack underneath an app that is not
  the media app.

## Composite widgets — the part worth rebuilding

The design principle above is that each of these **is not one primitive**. Each
is several, wired so that they behave as a unit, and each one is a place where
a straightforward widget library's assumptions stop holding. This is the list
the demo should be built to *satisfy*, because satisfying it is what would
demonstrate `ui_core` can carry a real infotainment UI.

| # | Widget | What makes it composite | What it demands |
|---|---|---|---|
| 1 | **Card carousel** | Swipeable pages at the foot of the car-status pane; a dot pager; **each card is itself two-level** (the Media card reveals a source list on swipe-up [B]); the carousel's *default page* is mode-dependent — *"the G-Meter displays as the default card whenever you engage Track Mode"* [B]; **the whole strip is dismissible and its recovery lives on a different affordance** [B] | Horizontal paging with snap + inertia; a page indicator; per-card internal scroll; a card host that can hand off its own gesture |
| 2 | **Per-field staleness** | The Tire Pressure card carries **four readings positioned at the wheel they belong to, each with its own independent timestamp** — photo `01` shows `42 psi / 16 minutes ago` beside `42 psi / 15 minutes ago` on the same card, plus a `Recommended Front: 42 / Rear: 42` block with no timestamp | A value+unit+relative-time tuple rendered as one unit, repeated with independent state, at four positions |
| 3 | **Callout hotspots** | Leader lines drawn from a rendered object out to labelled buttons (`Open Frunk`, `Open Trunk`), plus a free-floating lock glyph above the roof and a charge-port glyph at the rear-left | Hit targets anchored to positions on a picture, with leader lines and labels, revealed by a gesture on the picture |
| 4 | **Mode-dependent controls** | Nine documented instances where one control changes meaning: the fan slider's **enumeration** changes in Auto [A]; the defroster is a 3-state button whose exit **restores three other controls** [A]; the map-orientation icon becomes route-overview *while navigating* [A]; the Maps dock icon hides the map when already on it [B]; the visualisation drag is a manual toggle **and** a persisted setting **and** an automatic FSD behaviour [B]; Track Mode recolours the car body by component temperature and tire grip [A] | The state that decides a control's *presentation* must be separate from the control's value. **The per-widget half of this is already solved** — see **L6a** — so what is demanded here is the **cross-widget** half: see **L6b** |
| 5 | **Translucent chrome over a live scene** | The whole interface floats on the map; the car visualisation is a *lighting* context for everything above it; the media player's background is *"translucent, instead of a solid color… the vehicle animations subtly shine through"* [B] | An alpha-blended overlay layer over a continuously-rendering scene. **Stated as a product decision, not a source fact** — see **L1**, which is where the pipeline work sits |
| 6 | **Bidirectional power meter** | A bar with a **fixed zero in the middle**; draw above, regen below; *"Power being fed back to the Battery displays in green whereas power used by the regular braking system displays in gray"*; the draw half is *"black (or white if the display is dark)"* [A] — **so one half inverts with the theme and the other does not**. Four semantic states on one axis, no numeric scale | A signed axis whose two directions have independently theme-polarised colours |
| 7 | **Proximity ramp** | *"Colored lines radiate from the image of your Model 3 as objects are detected… The location of the lines correspond to the location of the detected object. The color of the lines (white, yellow, orange, or red) represents the object's proximity"* [A] | Position and colour carrying two independent variables, emitted radially |
| 8 | **Lane marker with changing identity** | One colour, four forms: a filled blue lane region, *"a single blue line"* under Navigate on Autopilot, a blue indicator line for Lane Departure Avoidance, and *"highlights the lane marking in red"* on Emergency Lane Departure [A] | The same visual channel switching representation class with a mode |
| 9 | **The indicator-light column** | ~20 conditions across five colours, **three distinguished by timing rather than colour**, one **latching** until an external event (driving 25 km/h) | A severity-ordered column of status glyphs with blink semantics and an externally-cleared fault |
| 10 | **Dock edit mode** | Long-press to enter; an `X` badge appears; drag from a tray whose **contents change in edit mode** [B]; overflow silently evicts; one control type bypasses the dock [A] | A container that switches into an editing layout, with drag-out, drag-in, and per-item affordance |
| 11 | **Tri-state button with external restore** | Defroster: touch once → defog, twice → defrost, thrice → off **and restore the air distribution, heating and fan to their previous settings** [A] | A control that must snapshot and restore other controls' values |
| 12 | **The seat widget** | Level encoded as **squiggle count**, direction as **squiggle colour**, an `Auto` state, and a 30-minute auto-shutoff [A]; and it is the one dock item that relocates itself [A] | A glyph whose *content* is generated from a state, not chosen from an atlas |
| 13 | **Heterogeneous alert slot** | One slot, four different producers; dismissal by swipe only; `Learn More` **presence is data-dependent** (*"Not all alerts provide additional information at this time"* [A]); a bell-indexed archive behind it | A toast host that takes heterogeneous payloads and whose action affordance is conditional |
| 14 | **Segmented + blue-action composite row** | A segmented control and a solid-colour action button in one row, e.g. exterior lights next to the fog-light button [A] photo `02` | Two control archetypes that must align on one baseline without merging |
| 15 | **Coupled dock regions** | Recents' width is a **function of** My Apps occupancy, collapsing to one at full [A] | Two sibling regions whose layout depends on each other's content |
| 16 | **The two-axis reshape** | Horizontal drag = a **discrete snap that also changes the content tier** (split ↔ full-screen reveals road markings and objects); pinch = continuous zoom **[C]** | One axis discrete-and-tiered, one axis continuous. **Its only source is the visualisation-resize sentence, which is `[C]`** — Tesla's own words read in a search index, not off an opened page, and the corpus is unreachable from this host. **Snippet-level, not specified**: a task must re-verify it before treating the tier change or the pinch as a requirement |
| 17 | **Spatial audio pad** | Balance is *"drag the center circle to the location… where you want to focus the sound"* [A] | A 2-D drag target inside a fixed frame |

### Gaps this layout exposes in `ui_core`

Verified against the source, not inferred — **a claim this section made and then
got wrong twice.** Rows **L1** and **L6** were checked against the source on
2026-10-05 and found **false when written**, which is a different failure from
being overtaken by a later commit; see § *Corrections to the second gap table*.
The other nine rows hold.

**These rows are cited as `L1..L11`, not `1..11`.** This document has two numbered
gap tables and the first keeps its own numbering because four files cite it, so a
bare "gap N" here is ambiguous. The prefix is the fix. Note that `L6` is split
into **L6a** and **L6b**, which is why eleven rows are cited in eleven slots with
one row removed and one added.

| # | Gap | Severity | Blocks | Evidence |
|---|---|---|---|---|
| L1 | **Colour capture and a public backdrop API now exist; rect-scoped capture does not.** **Amended 2026-10-08 by `TASK_UI_PRIM_41`**, which added `render::target::ColourTarget` (`GL_RGBA8`, four channels — sub-item (a) closed) and `Painter::backdrop` / `DrawCommand::Backdrop` as a `Segment` boundary (sub-item (b) closed), and recorded sub-item (d)'s bandwidth decision in numbers. **Sub-item (c), rect-scoped capture, is still open** and is why the row is neither closed nor downgraded: `glBlitFramebuffer` from a multisampled read framebuffer requires the source and destination rectangles to have identical bounds (OpenGL ES 3.1), so the only legal capture from this pipeline's default framebuffer is a full-window blit into a full-window texture, and the rect on a `DrawCommand::Backdrop` sizes the composite rather than the capture. **The original claim, for the record:** **Rewritten 2026-10-05 — the previous row was false.** An FBO, a sigma-weighted separable Gaussian blur and a composite/resolve pass **all exist and run in production**: `render/target.rs` (`ShadowTarget`, `create_framebuffer`, two ping-ponged `GL_R8` textures, `check_framebuffer_status` completeness), `render/blur.rs` (`MAX_TAPS = 9`, `taps_for`, `gaussian`, `kernel`, `reach`), and in `render.rs` four programs plus `bind_default_target`. `Dialog` uses the whole path today. **What is actually missing is four things, and none is "no FBO":** (a) a **colour attachment** — `ShadowTarget` is `GL_R8`, one channel of coverage, chosen because a shadow's colour is one constant (`target.rs:109-116`), so it **cannot hold a scene**; the demo needs `GL_RGBA8`. (b) A **public entry point** — `draw_shadow_offscreen` is reachable only from `draw_shadow_batch`, i.e. only from a `DrawCommand::Shadow` segment, so no widget can ask for a backdrop. (c) **Rect-scoped capture** — `bind_for_write` disables the scissor test because it draws a window-sized quad. (d) A **bandwidth decision that inverts**: the `GL_R8` choice is an optimisation *because* a shadow is one colour; capturing full-window RGBA and blurring it every frame is a different order of cost, and the target is aarch64. | **Critical** | Every overlay: Controls, climate, media, app tray, alerts | `ColourTarget`, `ColourTarget::capture` and `allocate_texture` in `render/target.rs`; `MAX_TAPS`, `rect_quad` and `reach` in `render/blur.rs`; `Pass::Backdrop`, `draw_backdrop_batch`, `draw_backdrop_offscreen`, `backdrop_fragment`, `BLUR_COLOUR_FRAGMENT_SHADER_SRC`, `BACKDROP_COMPOSITE_FRAGMENT_SHADER_SRC` and the `## Backdrops` section in `render.rs`; `BackdropMode`, `DrawCommand::Backdrop` and `Painter::backdrop` in `paint.rs`; `ShaderKind::Backdrop` and `Segment::backdrop` in `batch.rs`. **No line numbers**, because a line number into a source file is stale within days of being written. Sub-item (c)'s constraint is enforced by `a_colour_capture_blits_the_same_rectangle_twice`, so the row's remaining gap is a test rather than a note. |
| L2 | **No transform reaches the GPU.** `Transform` is `Interpolate`-able so it can be animated, but `DrawCommand` has no transform field — none of its nine variants carries one — and there is no matrix or `u_model` uniform. **2026-10-07 note (`TASK_UI_PRIM_36`): the "no matrix" half is closed** — `Mat4` (column-major, `render/matrix.rs`), both projections (`perspective` in OpenGL `[-1, 1]` clip space, `orthographic`), and the `Transform` → `Mat4` conversion (`to_matrix`, declared in `render/matrix.rs`; `property.rs` untouched) exist and are unit-tested against hand-computed values. **The uniform is task 37's** (`u_mvp` reserved in `render.rs`; no program declares it and nothing uploads it), **and the 2D half — a transform field on one of the nine 2D variants, applied by one of the four 2D shaders — is closed by no task in 34–40.** Severity and Blocks unchanged: those are still open because they are the 2D half. | **Critical** | Panel slide-ins, card paging, the drive-mode strip's drag, any resize animation | `property.rs:337`, `animation.rs:110-122`; zero hits for `u_model`/`mat4`/matrix in `render.rs` or `paint.rs` |
| L3 | **`LayoutMode::Grid` unimplemented** — `LayoutMode::Grid { .. } => Vec::new()`, `columns` declared and never read. `Flex.wrap` accepted and discarded by the `..`. | **Critical** | The Controls tile grid, the app tray grid | `layout.rs:1205`, `:1200`, `columns` at `:510` |
| L4 | **`LongPress` and `Swipe` are emitted and consumed by no widget.** Checked per-widget over `fn on_event`: zero match arms in **all sixteen** widget modules. `InputEventKind::Swipe` appears in no `on_event` body at all; the only mentions are doc comments saying a widget *deliberately ignores* them, and **tests that assert non-consumption** — `list.rs:3846`, `keyboard.rs:2670` — plus `toggle.rs:748`, a doc comment. **2026-10-08 note (`TASK_UI_PRIM_40`):** `Rotator` (`widgets/rotator.rs`) is the crate's first two-axis drag-driven value — a yaw plus a clamped pitch through one write path — and the module count below is sixteen for it. **`LongPress` and `Swipe` remain unconsumed and the row's claim is unchanged**: a `Swipe` carries a direction and no magnitude, so mapping it onto a rotation means inventing one, and the three consumers this row names — dock edit mode, alert dismissal, card paging — are discrete threshold-based decisions where a swipe's semantics is exactly what is wanted, which is not true of a continuous orbital camera. `Pinch` is declined by the same widget for the same reason and belongs to a zoom task. | **High** | Dock edit mode, card paging, alert dismissal, the drive-mode strip | per-widget `awk` over `on_event` → 0 in all 9 `on_event` impls; `widgets/mod.rs:176-191` lists sixteen `pub mod` entries |
| L5 | **No horizontal scrolling, no momentum, no snap.** `Scroll` and `List` are vertical-only — `scroll_offset` is a scalar `Property<f32>`, and left/right are explicitly rejected. No inertia identifier exists anywhere. `Scroll::snap_to_state` is **palette** snapping, not scroll snapping. | **High** | The card carousel, dock overflow, the widened wiper segmented control | `scroll.rs:378`, `:1178`, `:1362`, `:1498`; `list.rs:132` states momentum is out of scope; `list.rs:748` |
| **L6a** | **A control's presentation *can* depend on a mode — SATISFIED, per widget.** The previous row claimed *"nothing carries 'which mode am I in' as a separate input"*, and that is false. `GaugeType` is declared in the crate's own words as *"a mode and not an appearance"*, deliberately behind a **setter rather than a `Property`** so it cannot change mid-frame and leave the mode disagreeing with the properties drawn. It selects three different primitive sets. `Severity` is a second such input and it changes **layout as well as colour** — no disc means the message starts at the panel's padding. **Twelve `pub enum`s exist in `widgets/`**, of which these two are the clearest; the rest are `TextAlign`, `WrapMode`, `Truncation`, `KeyAction`, `Page`, `Anchor`, `Orientation`, `ImageFit`, `ButtonState` and `ChartType`. | **—** | — | `gauge.rs:420-440`, `toast.rs:456-470`, `toast.rs:919-925`; twelve `pub enum`s across eight widget modules |
| **L6b** | **But a mode cannot cross a widget boundary.** Three things are genuinely absent: (a) **no mode dimension in the theme** — `ThemeToken` is a flat 33-variant enum with no `Focus`, `Hover`, `Pressed`, `Shadow`, `ZOrder`, `Active` or `Selected` variant, so a mode cannot restyle a subtree; (b) **no cross-widget propagation** — all twelve enums are a field on one widget read at paint time from its own state, and the demo needs one mode to reach widgets it is not on: the defroster's third touch restores three *other* controls, Track Mode recolours the car body; (c) **no mode-dependent enumeration of a stored value** — `Slider` does let the domain change at runtime via `set_range`/`set_step`, but it is one linear value → one position with **no label or option set**, so a stored value cannot *survive* an enumeration change and be presented through the new one. | **High** | The defroster's restore, Track Mode's recolouring, the fan slider's Auto enumeration — composite row 4 | `theme.rs:78-148`; `slider.rs:160`, `:263`, `:486`, and `:481-485` for the step constraint |
| L7 | **No margin, no `flex-shrink`, no `flex-basis`, no cross-axis gap.** All four hold, but **`Padding` does exist** and the previous row omitted it — a four-sided inset applied in `arrange` to every mode. A reader counting layout vocabulary from this row alone would under-count by the feature the demo uses most. | Medium | Dense settings rows that must not overflow | `Padding` at `layout.rs:579-588`, applied `:1198`, `:1212-1217`; `spacing` is main-axis only, `layout.rs:277` |
| L8 | **No offscreen text measurement on the draw command.** `DrawCommand::Text` carries no width, so a half-visible row cannot be clipped. **The previous row's field list was also incomplete** — there are seven fields, not six: `weight: FontWeight` is the seventh and the single place a weight lives. | Medium | Card content that overflows, the carousel's page edges | `paint.rs:203-238`, `weight` at `:237` |
| L9 | **No theme scoping or inheritance.** One flat global token map; no tokens for focus, hover, pressed, shadow or z-order — focus rings and hover/pressed are per-widget properties instead. | Medium | Any subtree that needs to differ from the global theme | `ThemeToken` at `theme.rs:78-148` (`TOKEN_COUNT` = 33 at `:148`), `Theme` at `theme.rs:356-359`; the per-widget states at `button.rs:346`, `:355-357` |
| L10 | **`Polygon` is convex-only** — triangle fan, no ear-clipping, no stencil; the source names both rejected alternatives in as many words. No bezier, no fill rules — zero hits for `fill_rule`/`nonzero`/`even_odd` anywhere. **Note also that `DrawCommand::Path` is a *stroked* polyline with an explicit width**, so it cannot serve as a filled icon outline either. | Medium | Any concave silhouette; the proximity ramp; instrument arcs | the fan is `render.rs:960-980` (`// A fan from the first point… the alternative — ear clipping, or a stencil pass — is a rasteriser`); `paint.rs:249`; `Path` at `paint.rs:303-312` |

### Corrections to the second gap table

Two rows of the table above were checked against the source on **2026-10-05** and
found **false at the time they were written**. That is a different failure from
being overtaken by a later commit, and the distinction is what makes it a
*verification* failure rather than ordinary staleness:

- **L1 was false when written.** `render/target.rs` (483 lines) and
  `render/blur.rs` (702 lines) were added in **`22356f6`** (task 22, 2026-10-03),
  and this file's last previous edit is **`1ea7e79`** — which is *later* in the
  history. So the table was written on top of a tree that already had both
  files, and the grep it offered as evidence could not have produced its answer:
  `git grep -ic framebuffer 1ea7e79` hits **eight files**, `target.rs` alone
  29 times. **The lesson — a sweep of a mechanism's call sites is not a sweep
  of what exists.** Here it is worse:
  the absence claim was made by a grep that was never run against the right tree.
- **L6 was false when written**, and in the crate's own words. The row denied
  any mode or enumeration concept; `GaugeType` is a `pub enum` documented as
  *"A mode and not an appearance"* (`gauge.rs:420-426`), and `Severity` changes
  layout as well as colour (`toast.rs:456-457`). The row has been split into
  **L6a** (satisfied) and **L6b** (what is genuinely missing), and **L6a's
  satisfaction is worth more to the demo than the gap was**: writing nine
  per-widget modes now inherits the crate's own reason for making the mode a
  setter, which is that a mode must not change in the middle of a frame.

Three further stale citations were found in this pass and are recorded rather
than fixed here, since fixing them is a code change:

- **Two doc comments assert the opposite of the code beside them.** The
  mechanism works: `Batch` carries `clip` outside the batch key and
  `apply_clip` sets the scissor per batch. The stale text is at
  **`render.rs:1920-1925`** (on `set_scissor`) and **`list.rs:122-126`**. **The
  correction below cited `render.rs:1459` for the first, which is wrong** —
  that line is inside `Renderer::new`'s doc block. Three more instances of the
  same claim exist: `scroll.rs:21`, `image.rs:117`, `image.rs:258`.
- **The live problem is narrower and worse than "the clip is never set":** the
  clip is supplied by the demo's frame loop for **one widget only**, so clipping
  works and is **not owned by the widget system**. That is row **#5** of the
  first table.

**Corrections to the first gap table.** Two of its rows were checked against the
source and are wrong:

- *"Scissor/clipping not applied — clip rects are computed but never set on
  the GPU"* is **false**, as above.
- *"No Icon widget"* is still true, and it is now a larger problem than it was:
  **L10** plus the absence of any vector path mean icons are the *first* thing
  the demo will need and the least supported thing in the crate. There is no
  path or vector-glyph support anywhere: `DrawCommand::Path` is stroked, fonts
  rasterize to a glyph atlas, and there is no outline extraction, no SVG and no
  curve tessellation.

**A note on how these were checked.** Every row above was verified by reading
the source in this session — grep for the absence, and read the surrounding
lines for the presence. **Two of the claims a first pass produced were wrong
about the method** and are recorded because the failure is the interesting
part: a bare `grep -i FBO` returns two hits, both of them comments *denying*
an FBO exists, so "2 hits" reads as evidence of a partial implementation and
is not. The FBO claim was only safe after reading what the hits said. The
same applies to `LongPress`, which appears in three widget files and is a
doc comment in all three.

### Could not verify

Named with what was searched, per `.ai/protocols/evidence.md`. **These are
absences, and each one is a place the demo would be inventing.**

| Question | Searched | Result |
|---|---|---|
| **Exact top-bar icon order** | §Touchscreen icon list, photo `01`, 2024.14 release notes | Order is verified for **this** vehicle by photo `01`. Tesla's own illustration disagrees with the photo and the manual never states an order. The photo wins for the demo; the disagreement is recorded rather than resolved |
| **How many cards the carousel holds, and their order** | §Car Status §Cards, photo `01` | The manual names *"Media, tire pressure data, trip information, and more"* — in that order. Photo `01` shows a **3-dot pager** with Tire Pressure and Navigate visible, and a greyed `Start FSD` card at a different height. Pager count, total card count and page order are **unverified** |
| **Popup stacking, ordering, queue depth** | Both manual editions §Popup Messages, photo `01` | Undocumented. Four producer types share one slot; the photo shows one. **The demo must choose a policy; Tesla's is unknown** |
| **Exact colours, spacing, radii** | Three manual editions, the release-note corpus | **Tesla publishes no design tokens at all.** Every measurement in the demo is a first-principles choice, not a transcribed value |
| **Dock drop-target visual during a drag** | §Customizing My Apps, photo `01` | The manual says only *"Drag any app or control from the app tray onto the My Apps area"*. **No highlight, caret or ghost is documented.** The blind-spot camera's drag *does* document shaded valid regions [A], so the affordance exists in the product — just not here |
| **Software version behind either photo** | Both PNGs, full frame | Not legible. Both layouts are consistent with a recent build, but that is inference |
| **Whether the status bar persists in full-screen Theater** | Release notes, three manual editions | The top bar is *repurposed* to HVAC shortcuts rather than removed [B]; dock persistence is **stated neither way** |
| **Track Mode's car-body recolouring rules** | §Track Mode, §Monitoring Vehicle Health | Named but not enumerated per component in anything read; treated here as [B] and **not** a spec the demo should copy until verified |
| **Lane-guidance / junction-view rendering** | Release-note corpus, three manual editions | The feature is documented as region-gated and no mechanics are given. **No NA/EU availability confirmed.** Excluded from the demo's scope |

**What was searched to produce the negative results above:** Tesla Model 3 and
Model Y Owner's Manual via the `rollout-tesla.com` mirror — §Touchscreen,
§Top Status Bar Icons, §Customizing My Apps, §Popup Messages, §Car Status,
§Driving Status, §Controls Overview, §Operating Climate Controls, §Climate
Popup, §Media, §Media Settings, §Volume Controls, §Tire Care; plus web search
for the release-note corpus. **`tesla.com` itself 403s from this host**
(Akamai challenge page), and `rollout-tesla.com/ownersmanual/modely/` returns
Access Denied — so **Model Y is covered only where its manual text is
identical to Model 3's**, and the Model Y §Controls Overview was read through
a third-party mirror, which is why it is tagged `[B]`. That is a real coverage
limit on the primary corpus, not a formality.

**Re-tested 2026-10-05 and unchanged**, with one trap added to the record: the
`tesla.com` 403 and the mirror's *Access Denied* both still hold, but an
**unknown GUID path on the mirror returns HTTP 200 and ~39 KB of HTML that is an
unrelated third-party site.** A reachability probe that reports 200 may have
found nothing of Tesla's at all. § *Sources* carries the consequence — the two
snippet-derived claims stay `[C]` and cannot be repaired from this host.

### What this means for the demo's shape

Three decisions the layout forces, which the operator should settle before
any `TASK_UI_DEMO_n` is written:

1. **The demo must have a map it can put things on top of.** Not a
   placeholder — the translucency, the panes and the overlays are all
   *defined* against it, and **gap #1** (the map widget, in the first table) is
   untestable without one. Distinct from **L1**, which is the pipeline work for
   drawing over it.
2. **The car visualisation and the Controls panel are different screens, not
   one screen with a tab.** The first sketch merged them. Every layout
   decision downstream depends on keeping them apart.
3. **Mode-dependence is a demo *design* problem, not a library gap — and the
   honest half is cross-widget.** The previous version of this item said
   *"nothing in the crate models it"*, which was false and is corrected in §
   *Corrections to the second gap table*. What the crate has: a per-widget mode
   concept (`GaugeType`), a documented position on why it is a setter and not a
   `Property`, and ten more presentation-bearing enums. **So the demo writes
   nine per-widget modes and inherits that reasoning for free.** What the crate
   has not: a mode that crosses a widget boundary, a mode dimension in the
   theme, or a value whose presentation through an enumeration can change — that
   is **L6b**, and it is where the real work is.
   **The counter-argument, recorded because it cuts against the split above:**
   cross-widget propagation is arguably the half that *does* deserve a library
   concept, since a demo-local answer means every future consumer reinvents it,
   and the defroster's snapshot-and-restore (composite row 11) is exactly the
   kind of mechanism that wants to be shared. Since 2026-10-05 the gap cannot be
   deferred either way — see § *Operator decisions (2026-10-05)* item 2 — so the
   open choice is **whether L6b is a `TASK_UI_PRIM_n` or a `TASK_UI_DEMO_n`**, and
   this document does not settle it.


