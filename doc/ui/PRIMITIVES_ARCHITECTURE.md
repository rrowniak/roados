# UI Primitives Architecture

Research date: 2026-09-27

## Core Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Widget tree storage | Arena allocation (generational indices) | Cache-friendly, no refcount overhead, bulk deallocation |
| Ownership | Parent owns children; arena owns all nodes | Tree structure = no cycles = no need for Rc/RefCell |
| Composition | Uniform tree — containers are widgets with children | No special "container" type; everything is a node |
| Theming | Reactive property graph (Slint-style) | Change propagates to dependents automatically; no full rebuild |
| Transparency | Premultiplied alpha throughout | Correct interpolation, simpler blending, no color bleeding |
| Animation | Property interpolation, not tree rebuilds | Animations update values; render tree reads values |
| Rendering | Batched draw calls, opaque/transparent separation | Minimizes GPU state changes |
| SDL3 bindings | `sdl3` crate (v0.20.0, Zlib) | Only actively maintained SDL3 binding |
| GLES 3.1 bindings | `glow` crate (v0.18.0, MIT/Apache-2.0/Zlib) | Only viable GLES 3.1 binding; dynamic function pointer loading |
| SDL3 native lib | Vendored (build-from-source via `sdl3-sys`) | Control version, disable unneeded subsystems, reproducible builds |

## Dependencies

```toml
[dependencies]
sdl3 = { version = "0.20", features = ["build-from-source"] }
glow = "0.18"
```

- `sdl3` with `build-from-source` builds SDL3 from vendored source via `sdl3-sys` (cmake crate). No system SDL3 package needed.
- `glow` loads GLES 3.1 function pointers via `SDL_GL_GetProcAddress` — compatible with SDL3-created contexts.
- SDL3 subsystems disabled at build time: audio, render, camera, filesystem — only video, events, input, joystick/gamepad needed.

## Widget Tree

### Arena allocation

All widget nodes live in a single arena (`Vec<WidgetNode>` + generational indices). Each node has a `Handle` (index + generation) that remains valid across insertions and deletions.

```
Handle { index: u32, generation: u32 }
```

- No `Rc<RefCell<T>>`: no runtime borrow-checking overhead, no scattered allocation, cache-friendly.
- No `Box`: parent-owned children make tree restructuring expensive.
- Arena: nodes are created/destroyed together (frame-level), iteration is cache-friendly, bulk deallocation on scene switch.

### Node structure

Every widget is a `WidgetNode` in the arena:

```rust
struct WidgetNode {
    kind: WidgetKind,           // Button, Label, Container, etc.
    children: Vec<Handle>,      // Child nodes (empty for leaf widgets)
    parent: Option<Handle>,     // Parent node
    properties: PropertySet,    // Reactive properties
    layout: LayoutState,        // Computed layout rect, constraints
    paint: PaintState,          // Cached paint data, dirty flags
    flags: WidgetFlags,         // Visible, enabled, focused, etc.
}
```

### Composition

No separate "container" type. A container is any `WidgetNode` with `children.len() > 0`. Layout is determined by the node's `layout` field:

```rust
enum LayoutMode {
    Absolute,
    Flex { direction: FlexDirection, wrap: bool },
    Grid { columns: usize },
    Stack,
}
```

A `Button` is a node with a `Label` child. A `ListView` is a node with N children. A `Dialog` is a node with children. The tree is uniform.

### Dynamic lists

For scrolling lists with thousands of items, the arena uses a slab allocator pattern: visible items are allocated, scrolled-out items return to a free list. No per-item heap allocation.

## Property System

### Reactive properties

Every visual aspect of a widget is a `Property<T>`. Properties form a directed acyclic graph (DAG) of dependencies.

```rust
struct Property<T> {
    value: T,
    tracker: PropertyTracker,   // Who depends on this property
}
```

When a property changes, all dependent properties and render nodes are marked dirty. Same model as Slint.

### Property sources

- **Literal**: `Property::new(Color::RED)`
- **Bound**: `Property::bind(|| theme.background)` — recomputed when dependencies change
- **Animated**: `Property::animate(from, to, duration, easing)` — driven by animation clock

### Inheritance

Some properties inherit from parent to child unless overridden: `font_family`, `font_size`, `color`, `opacity`, `visibility`. Similar to CSS inheritance. A child that sets its own value breaks the chain.

## Theming

### Theme as property source

A theme is a struct of `Property` values. Widgets reference theme properties through the property graph:

```rust
let background = Property::bind(|| theme.get(ThemeToken::ButtonBackground));
```

When the theme changes, the property graph automatically marks all dependent widgets dirty. No widget tree rebuild needed.

### Theme tokens

```rust
enum ThemeToken {
    Background, Surface, Primary, OnPrimary, Text, TextMuted, Border,
    Error, Warning, Success,
    SpacingXs, SpacingSm, SpacingMd, SpacingLg, SpacingXl,
    FontFamily, FontSizeXs, FontSizeSm, FontSizeMd, FontSizeLg, FontSizeXl,
    FontWeightNormal, FontWeightBold,
    BorderRadiusSm, BorderRadiusMd, BorderRadiusLg, BorderWidth,
    DurationFast, DurationNormal, DurationSlow,
    EasingStandard, EasingDecelerate, EasingAccelerate,
}
```

### Runtime theme switching

1. Theme stored as `Property<Theme>` at the root.
2. Switching theme = replacing the `Theme` struct's property values.
3. Property graph propagates changes to all dependent widgets.
4. Only dirty widgets re-render.

### Animated theme transitions

```rust
fn switch_theme(new_theme: Theme) {
    for token in ThemeToken::all() {
        let old = current_theme.get(token);
        let new = new_theme.get(token);
        current_theme.set(token, Property::animate(old, new, 300, Easing::Standard));
    }
}
```

Interpolates colors, sizes, and other numeric values over 300ms.

## Transparency and Compositing

### Premultiplied alpha

All colors and textures use premultiplied alpha throughout:

```
R' = R * A,  G' = G * A,  B' = B * A
```

Blending: `C' = S + (1 - αs) * D`

Avoids color bleeding during interpolation. Correct model for compositing.

### Opacity nodes

Opacity multiplies down the tree: `effective_opacity = parent.opacity * own.opacity`

When `effective_opacity < 1.0`, the node renders to an offscreen framebuffer (layer), then composites with blending. Same as Flutter's `OpacityLayer`, Qt's `QSGOpacityNode`.

### Render pass separation

1. **Opaque pass**: `effective_opacity == 1.0`. Front-to-back sorting. No blending. Freely reorderable by batching.
2. **Transparent pass**: `effective_opacity < 1.0`. Back-to-front sorting. Blending enabled.

Critical for performance: opaque nodes can be batched aggressively without draw order concerns.

### Layer boundaries

A node gets its own composited layer when:
- `opacity < 1.0`
- `transform` is non-identity
- `clip` is set
- `blend_mode` is not normal

Matches CSS stacking context rules.

## Animation System

### Property-driven

Animations do not rebuild the widget tree. They update `Property` values:

```rust
struct Animation {
    property: PropertyHandle,
    from: f32,
    to: f32,
    start_time: Duration,
    duration: Duration,
    easing: EasingFn,
}
```

Each frame: compute `t`, apply easing, interpolate, set property, mark dependents dirty.

### Animation types

| Type | Use case |
|---|---|
| `NumberAnimation` | Position, size, opacity |
| `ColorAnimation` | Color transitions |
| `TransformAnimation` | Rotation, scale, translation |
| `Sequence` | Chained animations |
| `Parallel` | Simultaneous animations |
| `Stagger` | Delayed sequence across children |

### Easing functions

```rust
enum Easing {
    Linear,
    EaseIn, EaseOut, EaseInOut,
    Spring { damping: f32, stiffness: f32 },
    Bounce,
}
```

### State transitions

Widget state changes trigger animations automatically:

```rust
fn on_state_change(old: State, new: State) {
    let target_bg = match new {
        State::Pressed => theme.button_background_pressed,
        State::Hovered => theme.button_background_hovered,
        State::Default => theme.button_background,
    };
    background.animate_to(target_bg, theme.duration_fast, theme.easing_standard);
}
```

## Rendering Pipeline

### Frame lifecycle

```
1. Process input → update state → mark dirty
2. Update animations → interpolate → mark dirty
3. Layout pass → compute sizes/positions for dirty subtrees
4. Paint pass → record draw commands for dirty nodes
5. Batch pass → group by material/blend mode
6. Submit pass → execute on GPU
```

### Batching strategy

Draw calls grouped by: texture atlas, blend mode, shader.

Target: **< 50 draw calls** per frame for a typical screen. Achieved through:
- Texture atlas: all icons, glyphs, small images packed together
- Geometry batching: multiple quads with same material merged into one vertex buffer
- Index buffering: shared index buffer for quad geometry

### Geometry retention

Unchanged geometry stays in GPU memory between frames. Only dirty nodes re-upload. Same strategy as Qt Quick's renderer.

### Scissor optimization

Clipped widgets use `glScissor` to avoid overdraw. Scissor rect = intersection of all ancestor clip rects.

## Input Handling

### Event flow

```
SDL3 event → InputEvent → Hit test (top-most widget) → Event dispatch
```

1. SDL3 produces `SDL_Event`
2. Convert to `InputEvent` (unified format)
3. Hit test: walk tree top-down, find deepest node whose bounds contain the point
4. Dispatch to that node
5. Node may consume or pass to parent

### Focus system

- Tab/arrow keys move focus between focusable widgets
- Focus stored as `Handle` at root level
- Focused widget gets `State::Focused` with visual highlight
- Steering wheel scroll maps to focus navigation

### Gesture recognition

| Gesture | Recognition | Action |
|---|---|---|
| Tap | Down + up within 300ms, minimal movement | Click/activate |
| Long press | Down for > 500ms | Context menu |
| Swipe | Move > 50px in one direction | Scroll/dismiss |
| Pinch | Two fingers apart/together | Zoom |
| Drag | Down + move | Scroll content |

## Memory Management

### Arena growth

Arena grows geometrically (doubling) when full. Handles remain valid across growth.

### Widget recycling

Scrolling lists: widgets scrolling out of view return to a free list. New items allocate from free list. No per-item allocation.

### Texture atlas eviction

Fixed maximum size (e.g., 2048x2048). When full, LRU entries evicted. Glyphs evicted before icons.

## Thread Model

Single-threaded. UI runs on one thread: event polling, input, animation, layout, paint, GPU submission. Simpler than Qt's multi-threaded renderer. If profiling shows saturation, a render thread can be added later.

## Module Layout

```
ui/
  src/
    ui_core/                 — the UI primitives library (this project)
      mod.rs                 — public API, UiContext
      arena.rs               — arena allocator, Handle
      node.rs                — WidgetNode, WidgetKind
      property.rs            — Property, PropertyTracker, PropertyGraph
      theme.rs               — Theme, ThemeToken, ThemeData
      layout.rs              — LayoutMode, layout algorithm
      paint.rs               — PaintState, draw command recording
      batch.rs               — draw call batching
      render.rs              — GPU submission, frame lifecycle
      animation.rs           — Animation, Easing, animation clock
      input.rs               — InputEvent, hit test, gesture recognition
      widgets/
        mod.rs
        button.rs
        label.rs
        container.rs
        slider.rs
        toggle.rs
        list.rs
        scroll.rs
        text_input.rs
        image.rs
        progress.rs
        gauge.rs
        chart.rs
        dialog.rs
        toast.rs
    ui_demo/                — demo/test harness for ui_core
    roados_ui/               — production UI for the vehicle
    ...                      — future tools, side projects
```

## Open Questions

| Question | Impact | Settlement path |
|---|---|---|
| Single vs multi-threaded rendering | Performance on target | Benchmark; start single-threaded |
| Vulkan backend later? | Future-proofing | GLES 3.1 first; Vulkan is additive |
| Scripting language for UI? | Designer workflow | Start with Rust DSL; evaluate later |
| Very long lists (10k+ items)? | Memory and perf | Virtualized list with slab allocator |
