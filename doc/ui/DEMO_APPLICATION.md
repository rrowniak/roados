# Demo Application — Direction

**Status:** Operator decision, 2026-09-30. *Screens* and *Layout* completed
2026-10-01 from Tesla's own Owner's Manual and the operator's own photographs
of a centre display.
**Category:** `TASK_UI_DEMO_n` (new)
**Depends on:** All `TASK_UI_PRIM_n` tasks (12–23) complete

## Goal

Re-implement `ui_demo` as a Tesla-like infotainment interface after all UI
primitive tasks are complete. The demo uses mocked data, focuses on visual
quality, and demonstrates that `ui_core` is fully capable of carrying a real
automotive infotainment application.

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

## Scope

### In scope

- All major Tesla infotainment screens
- Mocked vehicle data (speed, battery, temperature, etc.)
- Real icons and visual assets
- Theme switching (dark/light)
- Smooth animations and transitions
- 60 FPS target
- All widgets from tasks 12–23 working together

### Out of scope

- Real navigation/routing
- Real vehicle bus integration
- Real media playback
- Pixel-exact Tesla replication
- Production-quality error handling

## Relationship to task 24

Task 24 (`TASK_UI_PRIM_24.md`) is superseded by this direction. The existing
task 24 spec describes a widget gallery; the operator has decided to replace it
with a Tesla-like demo application. Task 24 is not started and will not be
started in its current form.

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

- **"Dark theme" was removed as a requirement.** Tesla ships both, and
  `Controls > Display > Appearance` is Dark / Light / Auto [A]. **Both of the
  operator's own photographs are the light theme** — a pale, low-contrast map
  with white cards. The demo must therefore render light credibly too, and the
  theme-switch requirement is a *demonstration of the existing animated theme
  switch*, not a second visual design. Note what the light theme costs: white
  cards on a near-white map, which is only legible because the map is washed
  out to near-invisibility in `01`.
- **"Bottom dock — media player and climate" is right for the wrong reason.**
  The media player is a *card in the car-status pane's carousel*, not a dock
  item; the dock's fixed items are Controls and the climate setpoint.
- **"44dp touch targets" is this project's number, not Tesla's.** Nothing in
  Tesla's documentation states a touch-target size. Keep 44dp as a roados
  decision; do not attribute it to Tesla.
- **Added: chrome is translucent over a live scene.** This is the single
  structural fact the original principles missed, and it is why gap 1 is
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
shipped widgets (tasks 1–13: Label, Button, Container) and the planned widgets
(tasks 14–23: Slider, Toggle, Image, Progress, List/Scroll, TextInput, Gauge,
Chart, Dialog, Toast). Each gap must be addressed — by a `TASK_UI_PRIM_n`
amendment or a `TASK_UI_DEMO_n` task — before or during the demo implementation.

**Re-derived 2026-10-01** against the shipped code and against the completed
*Layout* section below. The first eight rows are the original 2026-09-30
assessment, kept so the original reasoning is auditable; **rows 5 and 8 were
checked against the source and are wrong**, and are marked. Ten further gaps
the Layout work exposed are in *Gaps this layout exposes in `ui_core`*,
further down.

| # | Gap | Severity | Blocks |
|---|---|---|---|
| 1 | **Map widget** — no map renderer exists or is planned. The demo's centerpiece. | Critical | Map/navigation screen |
| 2 | **Grid layout non-functional** — `Grid` mode lays out no children and reports no rects; `wrap` is accepted and not honoured. | High | App launcher screen |
| 3 | **No screen/navigation system** — no screen stack, tab controller, or transition system in the library. | High | Multi-screen app structure |
| 4 | **No Icon widget** — `Image` (task 16) displays textures but icons need vector rendering, theme tinting, and uniform sizing. **Raised 2026-10-01 from Medium to High**: the completed Layout section needs an icon for every dock item, every top-bar status item, every tab row and every indicator light — and `Polygon` is convex-only with no bezier, so this is the largest unsupported item in the design. | High | Visual quality — "real icons" requirement |
| 5 | **Clipping has no owner.** The clip rect *is* computed, carried on the batch and set on the GPU as a scissor — but the clip is supplied by the demo's frame loop for the `List` alone, so there is no per-node clipping in the widget system. Two doc comments assert the opposite of the code beside them. **Corrected 2026-10-01; the original "never set on the GPU" claim was false.** | High | Map viewport, scroll view clipping, card page edges |
| 6 | **No Card widget** — `Container` can be stretched to cover this, but a dedicated card with elevation/shadow matches the Tesla design language better. | Low | Visual polish |
| 7 | **No TabBar/Dock widget** — the bottom dock can be built from `Button` + `Container`, but a dedicated widget with active-state indication and icon+label layout is the right primitive. | Low | Bottom dock implementation |
| 8 | **Transform transitions** — screen transitions need translation, scale, and opacity animation support. The animation system handles property interpolation but transforms are not implemented in the render pipeline. **Confirmed 2026-10-01, and worse than stated: `Transform` is `Interpolate`-able, so it can be animated and then never drawn. `DrawCommand` has no transform field and there is no matrix or `u_model` uniform.** | **Critical** | Screen transition animations |

## Task structure

The new category `TASK_UI_DEMO_n` will be defined incrementally. The first
tasks will be:

1. Asset inventory and generation
2. Map emulation approach
3. Screen-by-screen implementation

Each task will follow the same workflow as `TASK_UI_PRIM_n`: developer →
review → operator commit.

Gap closure tasks (see *Library gaps* above) may be interleaved with demo
tasks when a gap blocks a demo screen.

## Open questions

1. What map data source to use for the emulation? (procedural, hand-drawn, or
   simplified real data?)
2. How to handle the Tesla logo and branding? (avoid trademark issues)
3. What vehicle model to display in the status screen? (Passat B5.5 or a generic
   car?)
4. **The car's body is a required asset, not an optional one.** The Layout
   section needs a rendered vehicle that hotspots anchor to, whose regions
   change colour in Track Mode, and that is *reflected on the floor* in the
   operator's own photographs. An SVG of a generic car, or a flat PNG, will not
   carry it. Which?
5. **Photo provenance.** `tmp/tesla_screens/*.png` are the best evidence in
   this repository and are untracked, undated, and their software version is
   not legible. Should they be committed as a reference set, and should the
   demo be pinned to the layout they show rather than to whichever software is
   current when the tasks are written?
6. **Light or dark first?** Both of the operator's photos are light, so the
   demo's first screen is the one that is harder to get right. But Tesla's
   dark theme is the better-known one. See *Design principles*.
7. **Card carousel depth.** Two cards are visible side by side in photo `01`
   with a 3-dot pager. Should the demo implement paging (the real behaviour)
   or a swipe-with-peek (cheaper, and what the photo's evidence supports
   equally well)?

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
  but not read off a page I opened; they are marked `[A]` for being Tesla's
  words and `[A*]` where the distinction matters. Nothing in this section is
  third-party prose about Tesla's UI.
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
| Top bar: `PRND` first | **Right that it is a horizontal letter row and it is at the left. Wrong that it is a top-bar *control*.** The `P R N D` row is a **readout in the car-status cluster**, not chrome. The *selector* is a separate **vertical drive-mode strip on the left screen edge**, edge-swipe summoned, hidden at highway speed, and **Neutral is not on it** — it is a press-and-hold inside Controls. | [A*] manual §Shifting: *"Swipe up for Drive, swipe down for Reverse, or press the drive mode strip for Park… To shift into Neutral, open Controls, then press and hold the Neutral icon… the drive mode strip is hidden when driving at highway speeds."* [A] photo `02`: the strip is a dotted vertical track with a grey `↑`, visible at the left edge |
| Bottom bar: `72` = "maybe temp in F" | **A temperature, but the cabin setpoint, not outside.** Outside temperature is a different widget in the top bar. `72` is setpoint; `65°F` in the same frame is ambient. The dock shows **no unit and no degree sign**. | [A] manual §Touchscreen: *"Climate controls (driver): Use the left and right arrows to decrease/increase cabin temperature."* [A] photo `01`: dock reads `< 72 >`, top bar reads `65°F` |
| Bottom bar: calendar icon | **True of this vehicle, false as a fixed slot.** Calendar is an app the user pinned into *My Apps*. The dock's fixed slots do not include it. | [A] manual §Touchscreen's 13-region layout: slot 9 is *My Apps*, and Calendar is never a named region. [A] photo `01` shows it among the pinned app icons |
| "Car screen gets squashed into left pane" | **Correct, and worth keeping — but it is the Controls *panel* that appears, and it is reached by the car icon or by an edge swipe.** | [A] manual §Controls Overview: *"Touch Controls on the bottom corner of the touchscreen… The Controls screen appears over the map… You can also access Controls by touching anywhere on the side of the touchscreen closest to the driver and swiping open."* [A] photo `02` |
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
`↑`, a grey `↓`, a bold `P`, and the word `HOLD` — [A*] the manual quotes for
the gestures, [A] photo `02` for the form: a dotted vertical track with a grey
`↑` against the left edge. **One axis, three targets, three different
gestures** — swipe up, swipe down, press — plus press-and-hold for the
emergency stop, plus a fourth gear reached somewhere else entirely.

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
You can pinch to zoom in or out."* [A*] — Tesla's own wording, read in the
indexed text of `tesla.com/ownersmanual/model3/en_us/Owners_Manual.pdf`
rather than off an opened page. The **behaviour** is independently visible in
photo `02`: the pane is narrower there than in photo `01`, because Controls is
open.

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
display on the right side of the screen. 2. Settings area."* [A] Model Y
manual §Controls Overview. The 3-D car is in the *car-status pane*, not here —
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
| 4 | **Mode-dependent controls** | Nine documented instances where one control changes meaning: the fan slider's **enumeration** changes in Auto [A]; the defroster is a 3-state button whose exit **restores three other controls** [A]; the map-orientation icon becomes route-overview *while navigating* [A]; the Maps dock icon hides the map when already on it [B]; the visualisation drag is a manual toggle **and** a persisted setting **and** an automatic FSD behaviour [B]; Track Mode recolours the car body by component temperature and tire grip [A] | The state that decides a control's *presentation* must be separate from the control's value. This is the single deepest structural gap — see gap 5 |
| 5 | **Translucent chrome over a live scene** | The whole interface floats on the map; the car visualisation is a *lighting* context for everything above it; the media player's background is *"translucent, instead of a solid color… the vehicle animations subtly shine through"* [B] | An alpha-blended overlay layer over a continuously-rendering scene. **Stated as a product decision, not a source fact** — see gap 4 |
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
| 16 | **The two-axis reshape** | Horizontal drag = a **discrete snap that also changes the content tier** (split ↔ full-screen reveals road markings and objects); pinch = continuous zoom [A] | One axis discrete-and-tiered, one axis continuous |
| 17 | **Spatial audio pad** | Balance is *"drag the center circle to the location… where you want to focus the sound"* [A] | A 2-D drag target inside a fixed frame |

### Gaps this layout exposes in `ui_core`

Verified against the source, not inferred. **Two of the eight gaps already
recorded in this document are wrong** — see the corrected table below.

| # | Gap | Severity | Blocks | Evidence |
|---|---|---|---|---|
| 1 | **No offscreen target, no layer, no alpha-blended chrome.** No FBO, renderbuffer or resolve pass anywhere in the crate; layering means draw order only. So there is no way to draw a translucent panel over a live scene and no way to blur one. | **Critical** | Every overlay: Controls, climate, media, app tray, alerts | `grep -i 'framebuffer\|renderbuffer\|FBO'` hits only two prose comments that say the pipeline *lacks* it (`render.rs:2701`, `gauge.rs:75`) |
| 2 | **No transform reaches the GPU.** `Transform` is `Interpolate`-able so it can be animated, but `DrawCommand` has no transform field and there is no matrix or `u_model` uniform. | **Critical** | Panel slide-ins, card paging, the drive-mode strip's drag, any resize animation | `property.rs:337`; zero hits for `u_model`/`mat4`/matrix uniform in `render.rs` |
| 3 | **`LayoutMode::Grid` unimplemented** — `LayoutMode::Grid { .. } => Vec::new()`, `columns` never read. `Flex.wrap` accepted and discarded by the `..`. | **Critical** | The Controls tile grid, the app tray grid | `layout.rs:1205`, `:1200` |
| 4 | **`LongPress` and `Swipe` are emitted and consumed by no widget.** Checked per-widget over `fn on_event`: zero match arms in all thirteen widget modules. The only mentions are doc comments saying a widget *deliberately ignores* them, and tests that construct them. | **High** | Dock edit mode, card paging, alert dismissal, the drive-mode strip | per-widget `awk` over `on_event` → 0 everywhere; `toggle.rs:748` is a doc comment |
| 5 | **No horizontal scrolling, no momentum, no snap.** `Scroll` and `List` are vertical-only; inertia is explicitly out of scope. | **High** | The card carousel, dock overflow, the widened wiper segmented control | `scroll.rs:90` |
| 6 | **A control's presentation cannot depend on a mode.** Every widget's appearance derives from its own value; nothing carries "which mode am I in" as a separate input. | **High** | The fan slider, the defroster, the map icon, the car body colouring — composite row 4 is **nine widgets**, not one | no mode/enumeration concept anywhere in `widgets/` |
| 7 | **No margin, no `flex-shrink`, no `flex-basis`, no cross-axis gap.** | Medium | Dense settings rows that must not overflow | `layout.rs` |
| 8 | **No offscreen text measurement on the draw command.** `DrawCommand::Text` carries `x`, `y`, `text`, `color`, `font_size`, `extra_advance` — **no width** — so a half-visible row cannot be clipped. | Medium | Card content that overflows, the carousel's page edges | `paint.rs:148-165` |
| 9 | **No theme scoping or inheritance.** One flat global token map; no tokens for focus, hover, pressed, shadow or z-order. | Medium | Any subtree that needs to differ from the global theme | `theme.rs:357` |
| 10 | **`Polygon` is convex-only** — triangle fan, no ear-clipping, no stencil. No bezier, no fill rules. | Medium | Any concave silhouette; the proximity ramp; instrument arcs | `paint.rs:249`, `render.rs:786` |

**Corrections to the gap table earlier in this document.** Two rows were
checked against the source and are wrong:

- *"Scissor/clipping not applied — clip rects are computed but never set on
  the GPU"* is **false**. `Batch` carries `clip` outside the batch key and
  `apply_clip` sets the scissor per batch. **The live problem is narrower and
  worse: the clip is supplied by the demo's frame loop for one widget only**,
  so clipping works and is not owned by the widget system. Two doc comments
  (`render.rs:1459`, `list.rs:124`) still assert the opposite of the code they
  sit next to.
- *"No Icon widget"* is still true, and it is now a larger problem than it was:
  gap 10 above plus the absence of any vector path mean icons are the *first*
  thing the demo will need and the least supported thing in the crate.

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
a third-party mirror. That is a real coverage limit on the primary corpus, not
a formality.

### What this means for the demo's shape

Three decisions the layout forces, which the operator should settle before
any `TASK_UI_DEMO_n` is written:

1. **The demo must have a map it can put things on top of.** Not a
   placeholder — the translucency, the panes and the overlays are all
   *defined* against it, and gap 1 is untestable without one.
2. **The car visualisation and the Controls panel are different screens, not
   one screen with a tab.** The first sketch merged them. Every layout
   decision downstream depends on keeping them apart.
3. **Mode-dependence (composite gap 4) is the highest-value thing to build
   and the least supported.** Nine real widgets need it; nothing in the crate
   models it. If the demo demonstrates only that, it has demonstrated the
   thing the library is missing.


