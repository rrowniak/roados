# UI Primitives

Research date: 2026-09-27

## Backend

**OpenGL ES 3.1 + SDL3.** No better option for this project.

- SDL3 (stable since Jan 2025, currently 3.4.16 as of Sep 2026) supports GLES 3.1 context creation via `SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_ES, ...)` + major/minor version. [A]
- SDL3 KMSDRM backend works on embedded Linux without X11/Wayland (GBM + atomic modesetting). [A]
- SDL3 is zlib-licensed. [A]
- SDL3 GPU API does NOT support OpenGL ES — only Vulkan, D3D12, Metal. This project uses `SDL_GL_CreateContext` + own GLES 3.1 rendering, not the GPU API. [A]
- SDL3 input: touch (`SDL_EVENT_FINGER_*`), mouse, keyboard, gamepad. [A]
- Alternative considered: SDL3 + Vulkan. Rejected — Vulkan open-driver support on Mali/VideoCore is immature; GLES 3.1 is the safe baseline per IDEA.md. [A]

## Reference Libraries

Read for reference only. None are dependencies.

| Library | Licence | Relevance | Key takeaway |
|---|---|---|---|
| **LVGL** | MIT | Highest — full widget set, DRM/EGL backend, FreeType + bitmap fonts, Flex/Grid layouts | Proof that a complete embedded UI library fits in MIT-licensed C with GLES backend |
| **Qt Quick** | LGPLv3 / Commercial | High — ~50+ controls, QML, EGLFS, animations, styles | The animation/type system and QML declarative approach is the gold standard for complexity |
| **Slint** | GPLv3 / Commercial | High — LinuxKMS backend, declarative DSL, FemtoVG/Skia renderers | Shows retained-mode declarative UI can run on DRM/KMS without compositor |
| **Dear ImGui** | MIT | Medium — immediate mode, no layout engine, no animation | Minimal reference for what a thin immediate-mode layer looks like; not a widget library |
| **Nuklear** | Public domain | Medium — single-header immediate mode | Same category as ImGui |
| **Flutter** | BSD-3-Clause | Medium — Impeller GLES backend, Material/Cupertino catalogs | Widget catalog is the most comprehensive; constraint-based layout is elegant |
| **Crank Storyboard** | Proprietary | Low — automotive-specific but closed | Confirms automotive market for GPU-accelerated UI |
| **Embedded Wizard** | Proprietary | Low — automotive-specific but closed | Same |

### What we adopt from reference

- **From LVGL**: DRM/KMS integration pattern, font atlas approach (with FreeType), style/state system (parts + states)
- **From Qt Quick**: Animation type system (number/color/rotation/sequential/parallel), property-based transitions
- **From Flutter**: Constraint-based layout model (simpler than CSS Flexbox for our needs)
- **From ImGui/Nuklear**: Immediate-mode overlay pattern for debug/development tools only

## Primitive Taxonomy

### 1. Basic Controls

| Primitive | Notes |
|---|---|
| Button | Push, with pressed/hover/disabled states |
| Toggle / Switch | Binary state, animated transition |
| Slider | Horizontal/vertical, continuous value, touch-drag |
| Rotary knob | Circular drag input, optional detents |
| Scroll wheel | Steering wheel input mapped to scroll events |
| Tap / press | Basic touch/click |
| Swipe / slide | Directional gesture |
| Long press / hold | Time-thresholded press |
| Pinch-to-zoom | Two-finger scale |
| Drag | Move element or scroll content |
| Haptic feedback trigger | Platform haptic on input events |

### 2. Display Elements

| Primitive | Notes |
|---|---|
| Text / label | Single-line, multi-line, styled spans |
| Icon | SVG-rasterized or font-based |
| Image | Static, with opacity/blend modes |
| Progress bar | Horizontal/vertical, determinate/indeterminate |
| Gauge / dial | Circular or arc, with needle/fill |
| Sparkline / trend | Time-series mini chart |
| 3D vehicle model | Rotating car visualization |
| Map (2D/3D) | Navigation display |
| Video | Media playback surface |
| Camera feed | Rear-view / 360 camera |

### 3. Navigation Elements

| Primitive | Notes |
|---|---|
| Tab bar | Bottom or top, with badge support |
| App launcher / grid | Icon grid with labels |
| List | Vertical scrollable, with sections |
| Grid | Multi-column, scrollable |
| Scrollable container | Generic scroll view |
| Drawer / side panel | Slide-in overlay |
| Top status bar | Persistent status icons |
| Bottom navigation | Fixed bottom bar |
| Search bar | Text input with results |
| Section header | Sticky or inline |
| Pagination dots | Page indicator |

### 4. Input Elements

| Primitive | Notes |
|---|---|
| Text input | Single-line, with cursor, selection |
| Multi-line text | Paragraph input |
| On-screen keyboard | Custom layout, automotive-optimized |
| Voice input indicator | Shows listening state |
| Gesture area | Recognizes swipe/pinch patterns |
| Steering wheel input | Scroll wheel + buttons via SDL |
| Touchpad / rotary | Center console input |

### 5. Information Display

| Primitive | Notes |
|---|---|
| Navigation map | Turn-by-turn overlay |
| Turn-by-turn card | Direction, distance, lane guidance |
| Media player | Controls, progress, album art, track info |
| Vehicle status | Speed, RPM, fuel, temperature, doors, lights |
| Climate controls | Temperature, fan, zone, seat heating |
| Speedometer | Digital or analog gauge |
| Battery / range | Charge level, estimated range |
| Tire pressure | Per-tire display |
| Trip computer | Consumption, distance, time, speed |
| Energy chart | Consumption over time |
| Driver assist visualization | Lane keeping, blind spot, collision warning |
| Charging status | Rate, time to complete, range added |
| Widget (live) | Home screen widget container |
| Calendar | Month view, events |
| Weather | Current + forecast |
| Contacts / phone | Call log, favorites |
| Messages | Conversation view |

### 6. Notification Elements

| Primitive | Notes |
|---|---|
| Alert / warning | Modal or banner, severity levels |
| Toast | Auto-dismissing, non-blocking |
| Status bar icons | Connectivity, time, notifications |
| Notification card | Rich content, actions |
| Badge / indicator | Unread count on tabs/icons |
| Banner | Persistent or timed |
| Dialog / modal | With title, body, actions |
| Context menu | Right-click or long-press menu |
| Tooltip | Hover or focus hint |

### 7. Animation / Transition Primitives

| Primitive | Notes |
|---|---|
| Fade in/out | Opacity transition |
| Slide | Directional translate |
| Scale / zoom | Size transform |
| Morph | Shape interpolation (Tesla patent US2011/0082627) |
| Parallax | Layered translate on scroll |
| Bounce | Spring physics |
| Stagger | Delayed sequence |
| Ripple | Touch feedback |
| Skeleton / shimmer | Loading placeholder |
| Page transition | Full-screen transition |
| 3D transition | Depth/rotation transition |
| Scroll animation | Scroll-linked transforms |

### 8. System-Level Components

| Primitive | Notes |
|---|---|
| Status bar (top) | Time, connectivity, vehicle state |
| Navigation bar (bottom) | Primary actions |
| Lock screen | Vehicle state summary |
| User switcher | Profile selection |
| Volume UI | Overlay slider |
| Loading indicator | Spinner, progress |
| Empty state | No data message |
| Error state | Error message + retry |
| Splash screen | Boot animation |
| Setup wizard | First-run configuration |
| Keyboard (system) | On-screen, automotive layout |
| Contextual overlay | Floating panel |

## Layout System

**Constraint-based** (Flutter-style), not CSS Flexbox.

- Each widget declares constraints (min/max width/height, alignment, padding)
- Parent imposes constraints, child decides size within them
- Simpler than full Flexbox/Grid for embedded use
- Supports: Row, Column, Stack, Padding, Align, ConstrainedBox

## Text Rendering

- **FreeType** for glyph rasterization (TTF/OTF fonts)
- **SDF (Signed Distance Field)** glyph atlas for scalable text with crisp edges
- **HarfBuzz** for text shaping (complex scripts, ligatures)
- **Bidirectional text** support (Arabic, Hebrew)
- Font fallback chain

## Theming

- Style properties per widget (colors, fonts, sizes, spacing)
- State-dependent styles (default, pressed, hover, disabled, focused)
- Dark/light theme variants
- Runtime theme switching
- CSS-inspired property inheritance

## Input Handling

- SDL3 events mapped to UI input events
- Touch: finger down/motion/up → tap, swipe, pinch, drag
- Mouse: click, move, wheel → hover, scroll
- Keyboard: key down/up → focus navigation, shortcuts
- Gamepad: steering wheel controls
- Focus system for keyboard/gamepad navigation
- Hit testing (top-most widget receives event)

## Animation System

- Property-based animations (interpolate any numeric property)
- Easing functions: linear, ease-in, ease-out, ease-in-out, spring
- Animation types: number, color, rotation, scale, offset
- Composers: sequential, parallel, staggered
- Transitions: state-change animations (e.g., button press → color change)
- Duration + delay per animation
- Animation curves (bezier)

## Automotive-Specific Constraints

| Constraint | Requirement | Source |
|---|---|---|
| Max single glance | 2 seconds | NHTSA |
| Max cumulative glance | 12 seconds | NHTSA |
| Occlusion test | 1.5s glances, max 9s total open time | NHTSA / ISO 16673 |
| One hand only | No two-hand visual-manual tasks | NHTSA |
| Touch target size | Minimum 44-48dp (~15mm) | AAOS / CarPlay HIG |
| Interaction depth | Max 2-3 levels | Industry standard |
| Glanceable | Readable in <2s | All OEMs |
| Safety locks | Text input, video disabled while driving | Industry standard |

## Gaps

- No primary-source documentation of Tesla's internal UI component library (proprietary). Tesla UI primitives are inferred from teardowns, patents, and owner manuals. [C]
- No existing library documents OpenGL ES 3.1 specifically; most say "ES 2.0 or newer". If ES 3.1-specific features (compute shaders, etc.) are needed, all reference libraries require custom rendering. [C]
- No benchmark data for SDL3 GLES 3.1 on Mali/VideoCore hardware. [C]
- Crank Storyboard and Embedded Wizard widget details are behind proprietary documentation. [C]
