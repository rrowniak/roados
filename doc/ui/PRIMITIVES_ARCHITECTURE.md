# UI Primitives Architecture

Research date: 2026-09-27

## Core Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Widget tree storage | Arena allocation (generational indices) | Cache-friendly, no refcount overhead, bulk deallocation |
| Ownership | Parent owns children; arena owns all nodes | Tree structure = no cycles = no `Rc`/`RefCell` in the tree |
| Composition | Uniform tree — containers are widgets with children | No special "container" type; everything is a node |
| Theming | Reactive property graph (Slint-style) | Change propagates to dependents automatically; no full rebuild |
| Transparency | Premultiplied alpha throughout | Correct interpolation, simpler blending, no color bleeding |
| Animation | Property interpolation, not tree rebuilds | Animations update values; render tree reads values |
| Rendering | Batched draw calls, opaque/transparent separation | Minimizes GPU state changes |
| SDL3 bindings | `sdl3` crate (v0.20.0, Zlib) | Only actively maintained SDL3 binding |
| GLES 3.1 bindings | `glow` crate (v0.18.0, MIT/Apache-2.0/Zlib) | Only viable GLES 3.1 binding; dynamic function pointer loading |
| SDL3 native lib | Vendored (build-from-source via `sdl3-sys`) | Pins the SDL version and needs no system SDL3. Not reproducible across hosts — see *Dependencies* |

## Dependencies

```toml
[dependencies]
sdl3 = { version = "0.20", features = [
    "build-from-source",
    "build-from-source-static",
    "build-from-source-unix-console",
    "image",
] }
glow = "0.18"
freetype-rs = { version = "0.38", features = ["bundled"] }
```

This is `ui_core`'s list, and `ui_demo` carries the same one minus `image`.
**Do not shorten it.** `build-from-source-static` links SDL statically into the
binary rather than through an rpath, which is unreachable from a manifest;
`build-from-source-unix-console` suppresses a `FATAL_ERROR` at
`cmake/macros.cmake:415` that SDL raises when neither X11 nor Wayland is
available, which is exactly the aarch64 configuration — see `CROSSBUILD.md`
§5.4, which `AGENTS.md` § Rust also cites.

- `sdl3` with `build-from-source` builds SDL3 from vendored source via `sdl3-sys` (cmake crate). No system SDL3 package needed.
- `sdl3` with `image` pulls in `sdl3-image-sys`, which builds **SDL_image 3.4.6** from vendored C source and statically links it. Justified in *SDL_image* below; the operator approved it 2026-09-30.
- `glow` loads GLES 3.1 function pointers via `SDL_GL_GetProcAddress` — compatible with SDL3-created contexts.
- `freetype-rs` with `bundled` builds FreeType 2.13.2 from vendored C source (via `freetype-sys` + `cc`) and statically links it — glyph rasterisation, no system FreeType needed.
- Text *shaping* (ligatures, complex scripts, bidirectional text) would need HarfBuzz. Its safe Rust binding exposes no shaping API — only `unsafe` C calls — so the operator declined `unsafe` and dropped the dependency 2026-09-30; FreeType alone renders Latin text. Revisit when a complex-script or bidi requirement lands.
- **SDL3 subsystems are left at SDL's defaults. None is disabled.** `sdl3`
  0.20.0 re-exports no subsystem feature at all, so the manifest cannot express
  any such configuration; the switches live on `sdl3-sys` 0.7.1 and only for
  the twelve SDL itself declares (`SDL/CMakeLists.txt:238-263`). The operator
  accepted the defaults on 2026-09-28 with that alternative blocked. This bullet
  replaces an earlier one that proposed disabling audio, render, camera and
  filesystem: two of those four were wrong about the product, one was never
  possible, and the fourth is the only accurate one. What the build keeps, and
  why each is right to keep:

  - **video, events, joystick, gamepad, HIDAPI** — `SDL_Init` is asked for
    exactly these three flags (`render/context.rs:190-192`), and a steering wheel
    is a documented input primitive arriving as an SDL gamepad
    (`PRIMITIVES.md:96,200`). HIDAPI is what makes that work for a USB device:
    `SDL_HINT_JOYSTICK_HIDAPI` defaults to `"1"` — *"whether the HIDAPI joystick
    drivers should be used"* — and evdev (`SDL_JOYSTICK_LINUX`) alone would
    leave such a device without named buttons and axes. `SDL_JOYSTICK_VIRTUAL`
    additionally makes a gamepad injectable from code, which is the only
    input-injection route that works on a host with no pointer device.
  - **audio** — a headline feature of the product rather than a preference:
    `IDEA.md` § *Features*, the bold *Audio and media* group asks for zones, source priority, ducking,
    radio and USB playback.
  - **camera** — also a named feature (`IDEA.md:60,107,183`): automatic
    headlights driven from a camera, and recognition behind it. The V4L2 driver
    is compiled in.
  - **haptic** — a listed input primitive, "haptic feedback trigger"
    (`PRIMITIVES.md:54`).
  - **filesystem** — not a choice. SDL declares twelve subsystems and
    filesystem is not one of them; on Unix SDL always compiles it, and the
    generated header carries `SDL_FILESYSTEM_UNIX 1`
    (`SDL/CMakeLists.txt:2112`). It is a few kilobytes of `open`/`stat`
    plumbing, and it is what `SDL_GetPrefPath` and friends need.
    `CROSSBUILD.md` §5.2 has the mechanism.
  - **render, gpu, dialog, tray, power** — compiled, and unused. `render` is
    unused because `ui_core` draws through its own GLES pipeline and never
    reaches `sdl3::render`; the others have no requirement behind them yet.
  - **sensor** — compiled, and the one genuinely undecided: `IDEA.md:60` offers
    "the light sensor or the camera" without saying which, and SDL's sensor
    subsystem covers a device-attached sensor, not a vehicle-bus one.

  The unused ones are left on deliberately, **and not to save space**: the
  operator's 2026-10-05 ruling is that trimming megabytes off a static archive
  is not a reason to narrow what the product can do. Should that change, note
  that naming a subsystem switch means naming a feature on `sdl3-sys`, which
  `sdl3` does not re-export — so it takes a second direct dependency, and
  `CROSSBUILD.md` §5 sets out that channel against the alternative of forcing
  the options from a toolchain file, with their costs.

### SDL_image

Task 16 requires loading PNG, JPEG and BMP from file. **What is needed and why
it is a dependency rather than code:** decoding a JPEG is a baseline DCT, a
Huffman decoder and a chroma upsampler — several thousand lines of
signal-processing code that no part of this repository wants to own. BMP is
built into SDL3 core; PNG needs zlib, which SDL3 also carries. JPEG is the one
format with no decoder anywhere in the tree, and task 16's requirement 2 names
it.

**Alternatives considered:**

| Route | Why not |
|---|---|
| SDL3 core only — `Surface::load_bmp`, `Surface::load_png` | Needs **no new dependency** and would have covered BMP and PNG. It cannot decode JPEG, so requirement 2 would be partially unmet, and the demo's image would be a format the head unit's own assets may not use. |
| `image` crate (pure Rust) | No C toolchain and a single decoder API, but a large transitive tree and a second licence (MIT/Apache-2.0) alongside SDL3's own zlib/libpng path. |
| `sdl3-image` Rust binding | Chosen — see below. |

**The crate.** `sdl3-image-sys 0.7.0+SDL-image-3.4.6`, from the same author as
`sdl3` and `sdl3-sys` (`vspace`), released alongside them and part of the same
versioned series. SDL_image itself is a long-standing, actively maintained
library in its own right, under the **zlib** licence — the same terms the
vendored zlib that FreeType already pulls in is under, so the licence audit this
cost is one this project has already paid.

**What portion is used.** One function: load a file into a decoded `Surface`,
which `ui_core::texture` then converts to premultiplied RGBA and uploads. No
scaling, no colour conversion, no animated formats — the widget scales on the
GPU and GIF/APNG are out of scope in task 16.

**Build model.** `build-from-source`, exactly like SDL3 and FreeType: the C is
compiled from vendored sources by `cc`/`cmake` and statically linked. **No system
library, and therefore no sysroot for the aarch64 cross build** — which is the
property that decided it, given the sysroot question is still open
(`CROSSBUILD.md` §8 item 1).

**Cost if it had to be replaced.** The only thing this project would have to
redo is one `load` call: the decoder hands back pixels, and everything
downstream — premultiplication, the texture cache, the fit modes — is this
repository's own code and does not know where the pixels came from. That is an
argument for keeping `texture.rs` decoder-agnostic, and it is why the decoder is
behind a small seam rather than called from the widget.

**What it does not do:** animated images, SVG, and image loading from the
network — all three are out of scope in task 16.

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
    children: Vec<Handle>,      // Child nodes (empty for leaf widgets)
    parent: Option<Handle>,     // Parent node
    layout: LayoutState,        // Computed layout rect, constraints
    paint: PaintState,          // Cached paint data, dirty flags
}
```

This is the whole type, and it is smaller than an earlier draft of this
document. There is no `kind`, no `properties` and no `flags` field, and no
`PropertySet` type exists. A node is a **place in a tree** with a layout cache
and a paint cache; what a node *is* lives in `widgets`, and what it *shows*
lives in the properties the widget holds a handle to. Visibility is
`LayoutState::visible`, and **only hit testing consults it** — `layout.rs` notes
that the layout pass places every node it reaches, visible or not, so hiding a
node leaves its rect and its siblings' rects untouched and stops it taking
input, and nothing more. Fields are private behind `children`, `parent`,
`layout`/`layout_mut` and `paint`/`paint_mut`.

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

`wrap` is accepted and **not yet honoured**: a `Flex` parent keeps its children
on one line and clips an overflowing child rather than wrapping it. Every other
mode behaves as described.

A `Button` is a node with a `Label` child. A `ListView` is a node with N children. A `Dialog` is a node with children. The tree is uniform.

### Dynamic lists

For scrolling lists with thousands of items, the arena uses a slab allocator pattern: visible items are allocated, scrolled-out items return to a free list. No per-item heap allocation.

## Property System

### Reactive properties

Every visual aspect of a widget is a `Property<T>`. Properties form a directed acyclic graph (DAG) of dependencies.

```rust
struct Property<T: 'static> {
    // A shared handle, not an inline value: several widgets bind to one value,
    // so it lives behind an Rc and is reached through RefCell.
    inner: Rc<PropertyInner<T>>,
}

struct PropertyInner<T: 'static> {
    value: RefCell<T>,
    recompute: RefCell<Option<Rc<dyn Fn()>>>,
    dependencies: RefCell<Vec<Weak<dyn PropertyBase>>>,  // what this reads
    dependents: RefCell<Vec<Weak<dyn PropertyBase>>>,    // who reads this
}
```

**The widget tree is `Rc`-free; the property graph is not, and cannot be.** A
bound property has to be readable from a widget that does not own it, so the
value is shared. The arena's no-`Rc` argument is about ownership and cycles in
the *tree* — nodes owned by the arena, parents holding `Handle`s — and it holds
there without exception.

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
      lib.rs                 — module declarations, the public surface
      arena.rs               — arena allocator, Handle
      node.rs                — WidgetNode
      property.rs            — Property, PropertyTracker, PropertyGraph
      theme.rs               — Theme, ThemeToken, ThemeData
      font.rs                — FreeType faces, glyph rasterisation, the glyph atlas
      texture.rs             — the image atlas, and the image-decoder seam
      layout.rs              — LayoutMode, layout algorithm
      paint.rs               — PaintState, draw command recording
      batch.rs               — draw call batching
      render.rs              — GPU submission, frame lifecycle
      render/
        context.rs           — the SDL window and the GLES 3.1 context
        target.rs            — the offscreen target a blurred shadow is drawn into
        blur.rs              — separable Gaussian, for shadows
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
        keyboard.rs
        image.rs
        progress.rs
        gauge.rs
        chart.rs
        dialog.rs
        toast.rs
    ui_demo/                — demo/test harness for ui_core
      main.rs
      fps.rs                 — frame-rate reporting for the demo
    roados_ui/               — production UI for the vehicle (planned)
    ...                      — future tools, side projects
```

## Open Questions

| Question | Impact | Settlement path |
|---|---|---|
| ~~Single vs multi-threaded rendering~~ | — | **Settled: single-threaded.** See *Thread Model*. Revisit only if profiling shows the frame saturated. |
| ~~Vulkan backend later?~~ | — | **Recorded, not deferred.** `PRIMITIVES.md` § *Backend* rejects Vulkan: open-driver support on Mali/VideoCore is immature, so GLES 3.1 is the baseline. An earlier draft of this table said Vulkan was "additive" — that was wrong and contradicted that decision. **Trigger to revisit, recorded 2026-10-05:** a target SoC with a conformant open Vulkan driver. |
| ~~Very long lists (10k+ items)?~~ | — | **Settled: virtualised.** `widgets::list` recycles a fixed row set; a hundred rows cost three nodes. |
| Scripting language for UI? | Designer workflow | Start with Rust DSL; evaluate later |

The first three rows were open when this table was written and have since been
answered — two by the work itself, one by a decision recorded in another
document. They are struck through rather than deleted so a reader can see what
was settled and which document holds the why, and so the two documents that
once disagreed about Vulkan cannot drift apart again unnoticed.
