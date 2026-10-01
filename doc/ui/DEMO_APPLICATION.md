# Demo Application — Direction

**Status:** Operator decision, 2026-09-30
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

1. **Tesla look and feel** — dark theme, card-based UI, minimal chrome, large
   touch targets
2. **Map-centric layout** — the map is the centerpiece, with overlays for other
   functions
3. **Bottom dock** — media player and climate controls always accessible
4. **App launcher** — grid of icons for all major functions
5. **Smooth animations** — all transitions animated, 60 FPS
6. **Automotive constraints** — glanceable, one-hand operation, 44dp touch
   targets

## Asset requirements

A dedicated task will inventory all needed assets:

- App icons (navigation, media, climate, vehicle, settings, etc.)
- Status bar icons (time, temperature, connectivity, battery)
- Control icons (play, pause, skip, volume, fan, seat heating, etc.)
- Map elements (roads, route line, car marker, POI icons)
- Vehicle images (for status display)
- Album art (for media player)

## Library gaps

Identified 2026-09-30 by comparing this document's requirements against the
shipped widgets (tasks 1–13: Label, Button, Container) and the planned widgets
(tasks 14–23: Slider, Toggle, Image, Progress, List/Scroll, TextInput, Gauge,
Chart, Dialog, Toast). Each gap must be addressed — by a `TASK_UI_PRIM_n`
amendment or a `TASK_UI_DEMO_n` task — before or during the demo implementation.

| # | Gap | Severity | Blocks |
|---|---|---|---|
| 1 | **Map widget** — no map renderer exists or is planned. The demo's centerpiece. | Critical | Map/navigation screen |
| 2 | **Grid layout non-functional** — `Grid` mode lays out no children and reports no rects; `wrap` is accepted and not honoured. | High | App launcher screen |
| 3 | **No screen/navigation system** — no screen stack, tab controller, or transition system in the library. | High | Multi-screen app structure |
| 4 | **No Icon widget** — `Image` (task 16) displays textures but icons need vector rendering, theme tinting, and uniform sizing. | Medium | Visual quality — "real icons" requirement |
| 5 | **Scissor/clipping not applied** — clip rects are computed but never set on the GPU. | High | Map viewport, scroll view clipping |
| 6 | **No Card widget** — `Container` can be stretched to cover this, but a dedicated card with elevation/shadow matches the Tesla design language better. | Low | Visual polish |
| 7 | **No TabBar/Dock widget** — the bottom dock can be built from `Button` + `Container`, but a dedicated widget with active-state indication and icon+label layout is the right primitive. | Low | Bottom dock implementation |
| 8 | **Transform transitions** — screen transitions need translation, scale, and opacity animation support. The animation system handles property interpolation but transforms are not implemented in the render pipeline. | Medium | Screen transition animations |

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

## Screens

## Layout

Top bar: gear/transmission PRND | Battery icon + percentage | lock icon (car open/locked) | user icon + user name| Time | Weather icon + temperature|
Bottom bar: Car icon | 72 (not sure what's that, maybe tem in F) | Calendar icon | Other icons for apps like spofity, youtube ,etc | Sound icon

### Welcome screen

Left panel: Fasten Seatbelt info + icon/image | automatic lighs on icon | seatbealt not fasten red icon
Main screen: Rendered car with info/buttons about open frunk, open trunk, etc.
Right pane: Map - barely visible

