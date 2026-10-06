# TASK_UI_PRIM_41: A `GL_RGBA8` Colour-Capture Target, and the Public Backdrop API

## Goal

Give `ui_core` a **four-channel offscreen target** that can hold the scene, and a
**public** way for a caller to say *"blur what is behind this rect"* — so that the
first half of *translucent chrome over a live scene* exists in the library rather
than being a thing the demo wishes for.

## Context

This closes **three of the four** things row **`L1`** in `DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* names as missing, and names the fourth
as the reason the row was rewritten:

> **What is actually missing is four things, and none is "no FBO":** (a) a
> **colour attachment** … (b) A **public entry point** … (c) **Rect-scoped
> capture** … (d) A **bandwidth decision that inverts**: the `GL_R8` choice is an
> optimisation *because* a shadow is one colour; capturing full-window RGBA and
> blurring it every frame is a different order of cost, and the target is aarch64.

**(a), (b) and (d) are this task. (c) is not**, and § *The capture, and the one
rule that decides its shape* below says why with a citation rather than an
opinion: **rect-scoped capture is not reachable by the only cheap copy GL offers,
from a multisampled default framebuffer.** The row therefore stays open with (c)
still open inside it, amended rather than deleted — the same form
`DEMO_APPLICATION.md` § *Corrections to the second gap table* prescribes and the
same form task 40 used for row `L4`.

`DEMO_APPLICATION.md` § *Design principles* records why this is *"the single
structural fact the original principles missed, and it is why **L1** is
critical"* — the bullet *"Added: chrome is translucent over a live scene"* — and
composite widget **5** in § *Composite widgets — the part worth rebuilding* names
the requirement it comes from: the media player's background is *"translucent,
instead of a solid color… the vehicle animations subtly shine through"*, which is
a picture the pipeline must be able to *put somewhere* before any of it is worth
drawing. **"An alpha-blended overlay layer over a continuously-rendering scene",
composite widget 5's own `Gap` cell — and there is no way to blur a scene into an
overlay layer in this crate today.**

### What exists today, read off the source

- **`ShadowTarget` is the only offscreen target in the crate.** In
  `ui/src/ui_core/src/render/target.rs` it is
  `pub struct ShadowTarget { framebuffer: glow::Framebuffer, textures:
  [glow::Texture; 2], read: usize, written: usize, size: Option<(u32, u32)> }` —
  one framebuffer and two single-channel textures, because a separable blur
  cannot read and write the same texture. Its public surface is exactly
  `new`, `size`, `ensure_size`, `bind_for_write`, `bind_for_read` and `swap`,
  plus the free functions `allocation`, `resize_decision` and
  `pub(crate) fn max_texture_size`, which the uncommitted diff widened from
  private to crate-public for the glyph atlas.
- **Its texture is `GL_R8` / `GL_RED`, and the module says why in its own words.**
  `ShadowTarget`'s doc: *"The texture is `GL_R8` and holds **one channel:
  coverage**. It is not `GL_RGBA8`, and the reason is arithmetic rather than
  taste."* The arithmetic is three lines: the shadow's colour is one constant,
  convolution is linear, so `blur(rgb · a) = rgb · blur(a)` — **a blurred
  premultiplied colour and a blurred coverage tinted once are the same image, and
  the second is a quarter of the bandwidth.** The same doc then says what the
  module does not do: *"What this module does **not** do is tell the renderer
  what to tint with."* That sentence is the whole of `L1`(a): the target has no
  colour to hold because nothing was ever meant to hold colour.
- **`draw_shadow_offscreen` is private, and there is exactly one route in.** In
  `ui/src/ui_core/src/render.rs` it is
  `fn draw_shadow_offscreen(&mut self, vertices: &[Vertex], color: Color, sigma:
  f32, clip: Option<Rect>) -> Result<(), RenderError>`, called from
  `fn draw_shadow_batch(&mut self, batch: &Batch)`, which matches only
  `DrawCommand::Shadow { .. }` and is itself called from **one** place: the
  `if let Some(shadow) = &segment.shadow` arm in `Renderer::end_frame`'s segment
  loop. **There is no other caller and no public entry point.** This is `L1`(b),
  verbatim, and it is why "the mechanism already exists" and "no widget can ask
  for a backdrop" are both true at once.
- **`ShadowTarget::bind_for_write` turns the scissor test off**, and says why:
  *"Scissoring is disabled here because the offscreen passes are whole-window"*.
  The composite restores it through `Renderer::bind_default_target`, which writes
  the scissor through `Renderer::apply_clip` and invalidates the renderer's
  `applied_clip` cache first. That is `L1`(c)'s mechanical half — the passes are
  window-sized because **nothing ever told the target which rect to hold.**
- **The offscreen path is mask → FBO, two blur passes, composite.**
  `SHADOW_MASK_FRAGMENT_SHADER_SRC` writes `vec4(v_color.a, 0.0, 0.0, 0.0)`;
  `BLUR_FRAGMENT_SHADER_SRC` accumulates `texture(u_source, …).r * u_weights[i]`
  into a `float` and writes `vec4(total, 0.0, 0.0, 0.0)`;
  `SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC` reads `.r` once and writes
  `vec4(u_color.rgb * coverage, coverage)`. **All three read or write one
  channel, and all three are `const &str` GLSL in `render.rs`.** The blur's own
  module doc is the separable-Gaussian argument: `2 · 2r+1` texture fetches per
  pixel of the target, where `r` is the tap radius.
- **MSAA 4× is on the default framebuffer and only there.**
  `render/context.rs` declares `MULTISAMPLE_BUFFERS: u8 = 1` and
  `MULTISAMPLE_SAMPLES: u8 = 4` and `Context::new` applies both, and the shadow
  FBO has **no multisample attachment** — `ShadowTarget::bind_attach` attaches to
  `glow::COLOR_ATTACHMENT0` and nothing else, which its own completeness check
  would already have caught. **So the only framebuffer that holds the scene is
  the one that is multisampled, and every framebuffer this task can read from it
  into is not.** That sentence is the shape of § *The capture* below.
- **The frame is a list of segments and each boundary slot is a `Batch`.**
  `BatchKey::is_singleton` in `ui/src/ui_core/src/batch.rs` is
  `matches!(self.shader, ShaderKind::Shadow)` and task 37 widens it to
  `ShaderKind::Shadow | ShaderKind::Mesh`; `Batcher::add_clipped` seals the open
  segment and starts the next for a singleton key; `Batcher::submit_order`
  drains the sealed runs into `Segment { opaque, transparent, shadow }` with
  task 37's second slot `mesh` beside `shadow`, on the invariant task 37 records as
  *"at most one of `shadow` and `mesh` is `Some`, because a seal consumes exactly
  one singleton command."*
- **Composited passes are a fixed three-slot list.** `Pass` is the private enum
  in `render.rs` and `COMPOSITED_PASSES: [Pass; 3] =
  [Pass::Solid, Pass::Image, Pass::Text]`; `Renderer::end_frame` walks that list
  over the segment's opaque group and then its transparent group, then the
  shadow. Task 37 adds `Pass::Mesh` **as a boundary and not a fourth member**,
  and keeps `COMPOSITED_PASSES` at three.
- **Every texture in the crate is `GL_RGBA8`, premultiplied once at load.**
  `Pixels::premultiply`, called from `decode` in `ui/src/ui_core/src/texture.rs`,
  and `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)` in `Renderer::end_frame`.
- **No dependency may be added.** Per `AGENTS.md` the approved direct
  dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38` (`bundled`).
  Everything this task needs is a GL call `glow` already exposes and an
  arithmetic function.
- **`glow 0.18.0` exports `blit_framebuffer` on `HasContext`**, with the
  signature
  `unsafe fn blit_framebuffer(&self, src_x0: i32, src_y0: i32, src_x1: i32,
  src_y1: i32, dst_x0: i32, dst_y0: i32, dst_x1: i32, dst_y1: i32, mask: u32,
  filter: u32)`, and exports `READ_FRAMEBUFFER` (`0x8CA8`), `DRAW_FRAMEBUFFER`
  (`0x8CA9`) and `NEAREST` (`0x2600`).

### The decision: a second type, `ColourTarget`, and why not one type with a format

**Decision: a second `pub struct` in `render/target.rs`, and the format is a
literal at the one place the texture is allocated — not a field, not a type
parameter, not an enum.** Three reasons, and the second is the one that settles
it.

1. **The format is a property of the shaders, and the shaders are compile-time
   text.** `SHADOW_MASK_FRAGMENT_SHADER_SRC`, `BLUR_FRAGMENT_SHADER_SRC` and
   `SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC` are three `const &str` GLSL literals
   whose `.r` reads and `vec4(a, 0, 0, 0)` writes **are** the coverage contract.
   A format parameter would move `GL_R8` out of the agreement between those three
   strings and into a runtime value, and the invariant "the mask writes what the
   blur reads" would stop being provable and become an assertion about a value.
   **The failure that makes this more than tidiness: it does not fail.** A
   `GL_RGBA8` target driven by the coverage shaders still *works* — the mask
   writes coverage into `r`, the blur reads `r`, the composite reads `r` — so a
   caller who builds the wrong instance gets a correct-looking picture with three
   quarters of its bandwidth wasted and no error anywhere. There is nothing to
   notice.
2. **`GL_R8` cannot represent a backdrop, and that is not a matter of degree.**
   The argument in `ShadowTarget`'s doc is a *saving*: the shadow's colour is one
   constant, so blurring one channel and tinting at the composite is the same
   image for a quarter of the cost. **A backdrop has no such constant** — its
   `rgb` varies per pixel, which is what "what is behind this rect" means — and
   its alpha channel carries the coverage the composite blends against, which
   the `GL_R8` mask shader has nowhere to put. So one channel is not a cheaper
   backdrop, it is a **backdrop with three of its four channels discarded**, and
   a shared type whose `ensure_size` can be handed `GL_R8` is a type whose
   compiler will not say so.
3. **A shared `ensure_size` would have to answer a question only one of the two
   has.** A colour capture from a multisampled default framebuffer is legal only
   under a condition the shadow target never faces (§ *The capture* below), so
   `ColourTarget` carries a one-shot capability probe and a documented
   "draws nothing" degradation and `ShadowTarget` carries neither. Merging the
   types merges two unrelated error stories into one method and buys about
   seventy lines of identical `swap`, `bind_for_read`, `bind_for_write` and
   `bind_attach`.

**The duplication this accepts is named, because `developer.md` § *Phase 2*
(*"No abstraction before the second use"*) points the other way and the operator
is entitled to see the trade made rather than the rule assumed.** This *is* the
second use, so the letter of the rule says factor the ping-pong protocol into one
private type. **The cheapest reversal is recorded and cheap: if a third offscreen
target appears, the four bookkeeping methods move into a private
`PingPongTarget` and neither public type's signature, semantics nor doc comment
changes** — because the format was never a field, the move is a mechanical one
and there is nothing to unpick. This file resolves the tension in favour of the
compile-time fact, and § *Out of Scope* records that the operator may resolve it
the other way without reopening anything else.

**What `ColourTarget` reuses from its neighbour, and what it does not.** It
reuses `allocation`, `resize_decision` and `max_texture_size` as they stand —
**all three are already format-free**, and `resize_decision`'s own doc says the
pair exists so *"a window that alternates between `0` and `1` on one side
allocates once instead of every frame"*, which is a property of the extents and
not of what is in them. `ColourTarget`'s own `ensure_size` calls them unchanged,
so the normalise-then-compare invariant has **one** implementation and not two.
`ShadowTarget`'s public contract is **not touched**: the same six methods, the
same signatures, the same semantics, and the same `Drop` that deletes no GL
object because `Context::drop` tears the context down immediately after. What
changes in `target.rs` around it is **doc text and one new private free
function**, `allocate_texture`, which both `ensure_size` bodies call with their
own format pair — so the `glTexImage2D` call and its four `tex_parameter_i32`
calls exist once and the format is a literal beside the type whose name says what
it holds.

### The capture, and the one rule that decides its shape

**The scene is in the default framebuffer, and the default framebuffer is
multisampled. Everything else follows from that one fact.**

You cannot sample the framebuffer you are rendering into, so a capture is a
*copy*, and GL offers two: `glBlitFramebuffer` and `glReadPixels`. The blit is
the right one; **but its legality from a multisampled read framebuffer is
narrower than it looks, and the narrowing is the whole reason `L1`(c) stays
open.** From the OpenGL ES 3.1 reference page for `glBlitFramebuffer`:

> `GL_INVALID_OPERATION` is generated if `GL_SAMPLE_BUFFERS` for the read buffer
> is greater than zero and the formats of draw and read buffers are not
> identical, **or the source and destination rectangles are not defined with the
> same (X0, Y0) and (X1, Y1) bounds.**

Both clauses are disjunctive, so **either** one rejects the call. This pipeline's
default framebuffer has four samples, so both are live:

- **The rectangles must have identical bounds.** A blit of the window region
  `(x0, y0, x1, y1)` into a rect-sized texture at `(0, 0, w, h)` is an
  `GL_INVALID_OPERATION`, and so is a **half-scale** blit, and so is any
  offset blit. **The only legal colour capture from this pipeline's default
  framebuffer is a full-window blit into a full-window texture.** Rect-scoping
  and half-resolution both become unavailable *through this route*, which is why
  both are named as the follow-up and neither is implemented here.
- **The formats must be identical.** `Context::new` sets no
  `SDL_GL_RED_SIZE`/`GREEN`/`BLUE`/`ALPHA`, so the default framebuffer's colour
  format is **the driver's**, and it is not known to be `GL_RGBA8`. A format
  read-back would be a necessary condition and not a sufficient one — the
  *internal formats* must match, and a driver whose default framebuffer is
  `GL_SRGB8_ALPHA8` reports eight bits per channel and still is not `GL_RGBA8` —
  **so the probe this task uses is the only sufficient one: do the blit, then read
  `gl.get_error()`.**

**And the rule cuts the other way once, in our favour, for free.** The same page:

> If `SAMPLE_BUFFERS` for the read framebuffer is greater than zero and
> `SAMPLE_BUFFERS` for the draw framebuffer is zero, the samples corresponding to
> each pixel location in the source are converted to a single sample before
> being written to the destination.

**So the resolve is the specification's, the driver's, and free** — which is why
§ *Out of Scope* can say *no MSAA resolve for the new target* honestly: there is no
multisample renderbuffer to allocate and no resolve code to write.
**What it costs is stated rather than assumed, and the reference page is careful
about it**: it says only that the samples *"are converted to a single sample"*, and
does not say how — a driver may average, may take one, may weight them. **So the
capture's antialiasing is whatever the driver's resolve is, and it is not the
coverage integral `chart.rs`'s module docs measured for this pipeline** (a 3-pixel
line at 3.0113 px, a 2-pixel axis at exactly 2.0000 px). **And a blurred edge is
softer than a sharp one in any case, so this is a limit recorded rather than a
defect to fix.**

**`glReadPixels` is the rejected alternative and is named rather than left
unconsidered.** It is unconditionally legal, it *is* rect-scopable, and it is a
pipeline stall followed by a CPU round trip — at 1280 × 1020 a full-window
read-back moves 5.2 MB down and 5.2 MB up **per frame**, on the CPU, with the
pipeline waiting. It is the route a task that cannot use a blit would take, and
this one can.

**So the capture is: full-window `GL_RGBA8` blit, one-shot legality probe, and
nothing clever.** What the probe does when the answer is no is in requirement 15,
and the shape of it is chosen deliberately: **the first backdrop on a driver that
refuses the blit returns `Err(RenderError::Gl(…))`** — loud, printed, in the
handoff — **and every later one draws nothing**, because the loud one already
happened and repeating it every frame would cost a `get_error` on every backdrop
forever. `Renderer::colour_capture_legal()` is a `#[must_use]` accessor so a
caller can ask, which is what keeps the quiet half from being silent.

### The public API: how a widget says *"blur what is behind me"*

**Decision: a new `DrawCommand::Backdrop` variant recorded through a new
`Painter::backdrop`, which becomes a singleton `BatchKey` and therefore a third
`Segment` boundary slot.** The mechanism is the shadow's, and it is the shadow's
because three separate facts already make it the only one available:

1. **`draw_shadow_offscreen` is private and one route in.** A method on
   `Renderer` called from a widget would not be a route in at all — widgets do
   not hold a `Renderer`, they record commands during paint and the renderer
   submits them later. **The recording is the only channel a widget has**, so the
   request has to be a command.
2. **Where a command lands is submission order, and only submission order.** Task
   34 recorded that the 2D passes neither test nor write depth, so ordering
   between layers **is** the segment order `end_frame` walks; task 37 recorded
   the same for a 3D layer. A request that composites "where it was recorded"
   must therefore be a **boundary**, or a command recorded after it merges into a
   batch recorded before it and lands on the wrong side.
3. **A backdrop and a shadow cannot share a boundary slot.** A seal consumes
   exactly one singleton command, so a segment ends with exactly one boundary.
   A backdrop that merged with a shadow, or rode in the same batch, would take
   its neighbour across the boundary with it.

**The command, in full, and every field earns its place:**

```rust
/// The scene behind a rect, captured, optionally blurred, and composited
/// over what is already on screen where it was recorded.
///
/// **It captures what is *already* on screen**, so a command recorded before
/// this one is inside the backdrop and one recorded after it is over it. That
/// is the recording contract, it is the caller's, and it is the same contract
/// task 37 recorded for a mesh: *record the scene first and the chrome second.*
/// `.ai/NEVERAGAIN.md` § *A shadow lands on whatever was recorded before it, not
/// on whatever comes next* is the same rule for the same reason — the layer
/// lands on everything before it.
///
/// **It replaces its rect, by default, and that is arithmetic rather than
/// taste.** The capture carries the default framebuffer's own alpha, which is
/// 1.0 everywhere an opaque primitive drew and the composited coverage
/// everywhere a translucent one did. `tint.a == 255` over an opaque capture is
/// `src + dst · (1 − 1) = src`: the backdrop *is* the picture in its rect, and
/// the sharp scene under it is gone. A caller wanting a frost rather than a
/// window passes `tint.a < 255`.
Backdrop {
    /// The rect of the window to capture, in window coordinates, **before** the
    /// blur's reach is added.
    ///
    /// **The capture is window-sized whatever this says**, because
    /// `glBlitFramebuffer` from a multisampled read framebuffer requires the
    /// source and destination rectangles to have identical bounds. This rect
    /// sizes the *composite*, and nothing else. See the type's own doc for why
    /// that is a limitation and not a design.
    rect: Rect,
    /// How the capture is blurred, or that it is not.
    ///
    /// **An enum and not a sigma, because `sigma == 0.0` does not mean the same
    /// thing here as it does for a shadow.** `blur::SOLID_BLUR` is 0.0 and the
    /// shadow path takes it as *"draw the shape directly"* — the offscreen round
    /// trip would redraw precisely what drawing it directly draws. **A backdrop
    /// cannot be drawn directly at all**, because the destination is a framebuffer
    /// rather than a shape, so sigma 0 is not a no-op: it is the same picture
    /// with a copy in front of it. Two modes and no overlap.
    mode: BackdropMode,
    /// The colour the captured scene is multiplied by, premultiplied, like
    /// every other colour on this enum.
    ///
    /// **Its alpha is the backdrop's opacity, and that is a deliberate departure
    /// from `Image` and from task 37's `Mesh`.** Both of those carry a separate
    /// `opacity` scalar, and task 37 gives the reason for its own: *"it decides
    /// the blend mode and the blend mode must not depend on how the caller chose
    /// to express the alpha."* **A backdrop's blend mode does not come from its
    /// tint at all** — it is always a blend, whatever the tint says, for the
    /// reason `DrawCommand::batch_key`'s arm gives — so there is nothing for a
    /// second field to decide, and one field instead of two is the honest shape.
    /// `Color::new(255, 255, 255, 255)` is "as captured"; there is **no `Color`
    /// constant for it** because `Color::new` is not `const`, so the value is
    /// named in `Painter::backdrop`'s doc and pinned by a test.
    tint: Color,
},
```

**`BackdropMode` is two variants, and `Sharp` is the one that has to exist:**

```rust
/// How a captured scene is blurred before it is composited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackdropMode {
    /// The capture is composited as it was taken: one FBO copy, no blur pass.
    ///
    /// **It is not the same picture as no backdrop at all**, which is why this
    /// is a variant and not an optimisation away: it replaces the sharp scene in
    /// its rect with the same pixels, which is what a card with a cut-out wants
    /// and what a caller gets for free once the target exists.
    Sharp,
    /// The capture is blurred with `sigma`, the Gaussian's standard deviation
    /// in pixels, with `blur::taps_for`'s cap and no wider kernel.
    Blur(f32),
}
```

**`Painter::backdrop(&mut self, rect: Rect, mode: BackdropMode, tint: Color)`**,
in the signature and recording shape of `Painter`'s other methods, with a
`# Examples` doc test in their form — which is a test that **needs no display**,
because it records and pattern-matches and never opens a window.

**The route, end to end, and each hop is a place a test can stand:**

```
Painter::backdrop            paint.rs        records DrawCommand::Backdrop
  → DrawCommand::batch_key   paint.rs        ShaderKind::Backdrop, texture None,
                                                blend_mode Transparent, ALWAYS
  → BatchKey::is_singleton   batch.rs        true — seals the segment
  → Batcher::submit_order    batch.rs        Segment { .., backdrop: Some(..) }
  → Renderer::end_frame      render.rs       if let Some(backdrop) = &segment.backdrop
  → draw_pass(Pass::Backdrop, batch)         render.rs   (exhaustive arm)
  → draw_backdrop_batch      render.rs       clip, rect, mode, tint
  → draw_backdrop_offscreen  render.rs       ensure_size, resting state,
                                                capture, 2 blur passes, composite
```

**Two decisions inside that chain are not the shadow's, and both are named:**

- **`blend_mode` is `BlendMode::Transparent` unconditionally**, refusing
  `BlendMode::from_color`'s rule. A backdrop at `tint.a == 255` is still a
  *blend*, because the capture's alpha is the framebuffer's accumulated coverage
  and a translucent primitive recorded before the backdrop left that below 1.0
  somewhere — an `Opaque` batch would overwrite it and put a hard edge where a
  blur has none. **The `DrawCommand::batch_key` arm's doc comment says so by
  reference to the rule it declines**, the same way task 37's `Mesh` arm names
  `Image`'s.
- **`Pass::Backdrop` is a boundary and not a fourth `COMPOSITED_PASSES` member**,
  for task 37's stated reason: *"a composited pass is drawn over the opaque and
  transparent groups at a fixed place in the frame's layering, and a mesh must be
  drawn where it was recorded, not at a fixed place."* A backdrop has the same
  property. **`COMPOSITED_PASSES` stays `[Pass; 3]`.**

### Bandwidth, and the decision, in numbers

**This is the section the original row got wrong by claiming nothing existed, so
the arithmetic is written out and every figure is derived from a number in the
tree.** Window: `WINDOW` in `ui/src/ui_demo/src/main.rs` is 1280 × 1020 =
**1,305,600 pixels**. `GL_RGBA8` is 4 bytes per pixel (5.22 MB); `GL_R8` is 1
(1.31 MB). One blur pass is `2r+1` fetches with `r = taps_for(sigma)`, and
`blur::MAX_TAPS` is 9, so the two passes together are **18 fetches per target
pixel**.

| stage | reads / window px | writes | bytes / window px | per frame |
|---|---|---|---|---|
| blit, 4 samples → 1 | 4 | 1 | 20 B | 26.1 MB |
| blur H, 9 taps | 9 | 1 | 40 B | 52.2 MB |
| blur V, 9 taps | 9 | 1 | 40 B | 52.2 MB |
| composite, over the rect | 2 | 1 | 12 B | 15.7 MB full, **1.4 MB** rect-limited |
| **total, rect-limited composite** | | | **112 B** | **146.2 MB** un-limited, **131.9 MB** as implemented |

**Every byte in that table assumes the default framebuffer is `GL_RGBA8`, and
`Context::new` does not ask for it** — it sets no `SDL_GL_RED_SIZE` family, so the
format is the driver's. **If it is not `GL_RGBA8` the table is not smaller, it is
moot**: a three-channel `GL_RGB8` default framebuffer and an `GL_RGBA8` texture
have *different formats*, which is the first disjunct of the ES 3.1 rule quoted
above, so the blit is refused, requirement 15's probe says so, and **no backdrop
draws at all**. **This is why the probe is a blit and not a format read-back, and
why its answer is an acceptance criterion rather than a note.**

**131.9 MB per frame at 60 fps is 7.9 GB/s of texture traffic.** The same table
for the shadow that runs today, at `GL_R8`: 23 single-sample accesses at 1 B plus
3 RGBA8 accesses at 4 B is **31 B per window pixel, 40.5 MB per frame, 2.4
GB/s.** So **a backdrop is 3.3× the shadow's bandwidth** — and 3.3 rather than
4 because the composite and the mask do not scale with all four channels.

**The decision, and the four mitigations and what happened to each.** Each row is
a real number, not an adjective:

| mitigation | per frame | vs 131.9 MB | implemented? |
|---|---|---|---|
| **window capture + window blur + rect-limited composite** | **131.9 MB** | — | **yes, this is the deliverable** |
| 5-tap blur instead of 9 (`Blur(sigma)` with `sigma ≤ 1.0`) | 90.2 MB | −32 % | **yes, free** — `blur::taps_for` already caps it, and no caller can widen it |
| rect-scoped capture *and* blur, a 400 × 300 rect | 13.4 MB | −90 % | **no** — `GL_INVALID_OPERATION`; § *The capture* |
| half-resolution tier | 34.1 MB | −74 % | **no** — a half-scale blit is the same rule, so it needs the row above first |
| dirty flag: skip when nothing behind the rect changed | n/a | n/a | **the weak form is free and is what being a segment boundary means**; the strong form is **not implementable here** |

**Why each rejection is the decision it is, and not a deferral:**

- **Rect scoping and half-resolution are one task, not two, and both are blocked
  on the same rule.** Once the capture is a re-submission of the already-recorded
  commands into a scissored target (§ *The capture*), rect
  scoping and a half-size target both become ordinary — a scissor is GL state the
  renderer already sets per batch, and a half-size viewport is a number. **Until
  then a blit is the only copy, and the blit is the copy that cannot be
  rect-scoped.** Naming them here is what keeps a later task from re-deriving it.
- **The strong dirty flag is not implementable in this pipeline and saying so is
  more useful than promising it.** "Nothing behind the rect changed" needs a
  content identity per pixel or per region; the batcher holds batches and the
  `Painter` is called every frame with no dirty protocol, no retained previous
  frame, and no way for a caller to say "this rect is unchanged". **The weak form
  — a backdrop costs nothing on a frame that records none — is true by
  construction**, because a backdrop is a segment boundary and a frame with none
  has one segment and one `if let`.
- **The blur radius is the only knob that costs nothing, and it is already
  capped.** `blur::MAX_TAPS`'s doc is explicit that seventeen taps was
  measured as indistinguishable from nine on this host and that the constant
  rests on the kernel's *width* argument — an 8-pixel ramp, no plateau. **So this
  task cannot widen it, and a backdrop that wants a wider frost needs a different
  kernel, which is a different decision with its own measurement.**

**Why the decision is made by arithmetic and not by this host's frame rate.**
`target.rs`'s module doc and `blur::MAX_TAPS`'s doc both record the same measured
fact: interleaved release runs at 1280 × 1020 with one shadow read 171, 182 and
211 jiffies at nine taps and 212, 202 and 188 at seventeen, against 191, 172 and
158 with no shadow at all — **the three sets overlap and the frame rate is
62.0–62.2 fps in all of them.** This host cannot tell nine taps from seventeen.
**It therefore cannot tell a 2.4 GB/s shadow from a 7.9 GB/s backdrop either**,
and a decision made on a number this host cannot produce would be the same
category of mistake as the row this task amends. The arithmetic is what decides;
**the measurement is a regression gate, and requirement 20 says so in those
words.**

**The aarch64 argument, as a threshold rather than a part number.** `AGENTS.md`
names aarch64 as a target and this file will not quote a fill rate for a part it
has not measured. **What can be derived is the threshold:** a backdrop costs **28
pixel-accesses per window pixel**, so 1,305,600 × 28 = **36.6 million
accesses per frame, 2.19 Gaccesses per second at 60 fps** — and a window of that
size at 1,305,600 pixels is a small head-unit panel by aarch64 automotive
standards, not a large one. **Any part whose RGBA8 fill rate is below about
2 GPix/s cannot afford a full-window backdrop on every frame it is asked for**,
which is a threshold an operator holding the part can answer in one number, and a
much better thing to hand them than a guess. **And the cost of being wrong about
the target is bounded by construction:** no gallery page records a backdrop, so
the six pages' measured cost of this task is **zero**, and the 131.9 MB figure is
what a caller pays *when it records one*, not what this task costs.

### Where it sits: submission order, depth, and the mesh pass

**Submission order.** The backdrop is a `Segment` boundary, so it composites
**after everything its own segment recorded and before everything the next one
will** — the position the shadow already occupies, for the reason task 37 wrote
down for the shadow. **The contract that follows is the caller's and it is one
sentence: record the scene, then the backdrop, then the chrome.** A backdrop
recorded *before* the scene captures an empty rect; a backdrop recorded after the
chrome blurs the chrome.

**Its `if let` goes first, before the shadow's and before task 37's mesh's, and
the relative order of all three is not observable** — a seal consumes exactly one
singleton command, so a backdrop, a shadow and a mesh each end their own segment
and are never adjacent. Task 37 already states this for the shadow and the mesh; this
task states it for three. **And the one case where it *is* observable is the
`NEVERAGAIN` case**: a shadow recorded immediately after a backdrop lands **on**
the backdrop, which is what that entry is about, and it is the caller's ordering
again. `draw_backdrop_batch`'s doc names it.

**Depth: no depth attachment, and it is state rather than storage, which is task
34's own reason.** `ShadowTarget` gets none because a shadow's mask is one quad
into a target cleared immediately before it, so there is nothing to occlude. **A
backdrop has the same answer for the same reason plus one more**: the capture is
a `GL_COLOR_BUFFER_BIT` blit and nothing else, so a depth attachment would have
to be written by a second blit whose formats must match and whose bounds must be
identical — *more* constraints for a buffer nothing reads. And task 34 recorded
the policy that makes it unnecessary: **the depth buffer orders mesh geometry
only, and the 2D passes neither test nor write it.** The blur and the composite
are 2D passes. So:

- **`depth_state_for` gains a `Backdrop` row: `{ test: false, writes: false }`**
  for **both** blend modes, in task 34's shape — the policy is data, and a pass
  added later without a row fails the suite.
- **`draw_backdrop_offscreen` asserts the resting state locally**, exactly as
  task 34's requirement 7 has `draw_shadow_offscreen` do before its clear:
  `gl.disable(GL_DEPTH_TEST)` and `gl.depth_mask(false)` immediately before the
  capture. Belt to task 34's requirement 6's braces, and the reason is 34's own:
  **depth test state is global and not a property of a framebuffer**, so a mesh
  pass that left it enabled would leak it into the offscreen passes.
- **`Renderer::bind_default_target` is the restore**, and it does not change: it
  already restores the framebuffer, the viewport and the scissor, and task 34's
  requirement 6 adds the depth restore to it. **This task adds no fourth thing to
  it**, which is worth saying because a fourth thing is how that function grows
  into the place the policy stops being visible.

**The mesh pass.** `Pass::Mesh` (task 37) is a boundary, writes depth when opaque,
and **leaves `depth_mask(false)` / `GL_DEPTH_TEST` disabled on the way out** — so
a backdrop boundary that follows a mesh boundary in the frame inherits the
resting state rather than a stale one, and the assertion in the bullet above is
belt to that. And under task 34's policy **a backdrop cannot be depth-tested
against a mesh anyway**: the composite draws with no depth test, so a mesh
recorded *after* a backdrop is over it whatever its `z` says. **That is not a
limitation worked around; it is task 34's recorded policy and task 37's
recording contract, and this task inherits both by name.**

### Premultiplied alpha, and the defect this path must not inherit

**The composite is premultiplied, and the arithmetic is three lines that do not
need a comment to be true.**

```glsl
vec4 texel = texture(u_source, v_uv);
frag_color = vec4(texel.rgb * u_tint.rgb, texel.a * u_tint.a);
```

`texel` is the captured scene, which is premultiplied by the mesh shader's own
contract (`base.rgb * shade`, `base.a` unshaded) and by `Pixels::premultiply` for
anything that came from a texture; `u_tint` is `quad_color(tint)`, premultiplied
because `Color`'s own doc says its components are; and the composition of two
premultiplied colours, `vec4(a.rgb · b.rgb, a.a · b.a)`, is premultiplied
whenever both operands are — `a.rgb · b.rgb ≤ a.a · b.a` follows from
`a.rgb ≤ a.a` and `b.rgb ≤ b.a`. **`backdrop_fragment(texel, tint)`, a pure CPU
mirror of that one line, is unit-tested over a grid of contract-honouring inputs
and asserts `r ≤ a && g ≤ a && b ≤ a` componentwise** — with no display, which
is the only kind of test `AGENTS.md` permits, and the same move task 37 made for
`mesh_fragment`.

**It must not multiply `texel.rgb` by `texel.a`**, and the reason is a recorded
defect. `ui/src/ui_core/src/widgets/chart.rs`'s module docs
§ *What made the first attempt at this measurement wrong* measured it: the solid
fragment shader writes `frag_color = v_color`, straight alpha, while
`Renderer::end_frame` blends with `GL_ONE, GL_ONE_MINUS_SRC_ALPHA`, which expects
a premultiplied source, so a translucent solid primitive composites as
`rgb + dst · (1 − a)` and reads brighter over a lighter destination — predicted
and measured at three destinations, `(187,134,252)` over black, `(255,205,255)`
over `TextMuted`, `(255,194,255)` over `Primary`. `doc/ui/IMPLEMENTATION_STATE.md`
§ *The finding that is not this task's: the solid pass does not premultiply* is
where that finding lives. **The backdrop's answer is neither to inherit the
defect nor to fix it:** it multiplies by `u_tint`, which is premultiplied by
contract, exactly as the image fragment shader already does with
`vec4(texel.rgb * opacity, texel.a * opacity)` and as `SHADOW_FRAGMENT_SHADER_SRC`
does with `vec4(v_color.rgb * v_color.a, v_color.a)`.

**And the honest limit, which is the sentence the reviewer needs:** **the
backdrop is exactly as premultiplied as what it captured.** If the scene behind
a backdrop was drawn by the solid pass, the capture holds straight alpha and the
composite's output inherits that, which is the defect arriving through a
different door. **This task cannot fix it and does not pretend to**: fixing
`Color`'s contract or the solid shader's would change the pixels of **every**
existing `DrawCommand` variant — nine at this tree, eleven where task 37 has
landed — and is its own task with its own before-and-after captures. **What this task promises is only that it introduces
no second instance of it**, and the test that proves that is the premultiplied
mirror above.

**The blur is on premultiplied colour, and that is the correct order.** Blurring
a premultiplied value is linear convolution of premultiplied data and stays
premultiplied. **Blurring straight colour would not be**, and at every edge of a
translucent primitive it would fringe — the same arithmetic `target.rs`'s doc
uses to justify `GL_R8`, one level up: `blur(rgb · a) ≠ blur(rgb) · blur(a)`. The
colour blur shader's doc comment states it.

### A self-contained draw, not an input to a composited pass

**Decision: the blurred backdrop is a self-contained draw — capture, two blur
passes, composite, and then the frame carries on. Nothing downstream samples
it.** The precedent is the shadow's, and the argument is the two points below.

- **A composited pass is drawn at a fixed place in the frame's layering.**
  `COMPOSITED_PASSES` is `[Pass::Solid, Pass::Image, Pass::Text]` and `end_frame`
  walks it over **every** batch in the segment. Making the backdrop an *input*
  to the image and text passes would mean **every batch recorded after the
  backdrop samples it** — a second sampler and a second uniform in two existing
  programs, and a source that is a copy of the frame rather than an image, for a
  picture the shadow's composite already draws correctly in one draw call.
- **It cannot be one anyway.** `COMPOSITED_PASSES` is walked per segment over
  the opaque group and then the transparent group; a backdrop captured from the
  frame **so far** is a different texture for every segment boundary, so a
  single member of a fixed list would have to be re-captured once per segment.

**What this costs, stated rather than discovered at review:** a caller cannot
have the text pass pick up the blurred backdrop's tint without drawing the text
twice. **That is the right trade and the reason is that a caller who wants that
asks for a second backdrop, which composites after the text and which is
already expressible.**

### Scope check

`developer.md` § *Scope check* asks for a file count and a component count, and
says what to do with a number over its threshold. **This file reports both and the
overshoot plainly, because the rule's own remedy is a decision and not a
silence.**

**Files: seven** — `ui/src/ui_core/src/render/target.rs`,
`ui/src/ui_core/src/render.rs`, `ui/src/ui_core/src/paint.rs`,
`ui/src/ui_core/src/batch.rs`, `ui/src/ui_core/src/render/blur.rs`,
`doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md`. **That is two over
the five-file threshold, and it is stated rather than resolved by counting the
documentation differently.** The seventh file is `blur.rs`, and it is not padding:
`rect_quad` belongs beside `full_quad` because it is the same six-vertex
window-space quad with a different origin, and putting it anywhere else would
split a module whose whole argument is that the blur's geometry is one shape.

**Components: three** — the colour target and its capture, the recorded request
and its `Segment` boundary, and the offscreen blur-and-composite draw. **Three is
at the threshold and not over it**, and the three are **coupled rather than
splittable**: `batch_key` matches on the new `DrawCommand` variant,
`draw_pass` routes on the new `ShaderKind`, and `draw_backdrop_offscreen` reads
the `BackdropMode` the command carries.

**Why the fan-out `.ai/protocols/subagents.md` § *Implementation fan-out* asks for
is not available here, stated so the operator can overturn it.** The first half —
`paint.rs` and `batch.rs` — *is* independently landable: `ShaderKind::Backdrop`
with no `Pass::Backdrop` compiles, because `ShaderKind` and `Pass` are separate
enums and nothing makes the compiler ask. **And a request that seals the frame and
then draws nothing is a hole in the picture, not a smaller deliverable** — which
is why it is not a candidate, and why `developer.md` § *Stop conditions* (*"the
change would silently do less than the spec says, and the smaller version is not
an acceptable outcome"*) is the rule this file lands on rather than fan-out.

**The shrink that would be honest, offered rather than taken: land `target.rs`,
`blur.rs` and `render.rs` with a renderer-level
`pub fn Renderer::draw_backdrop(&mut self, rect: Rect, mode: BackdropMode, tint:
Color) -> Result<(), RenderError>` and defer `DrawCommand::Backdrop`,
`Painter::backdrop`, `ShaderKind::Backdrop` and `Segment::backdrop` to a second
task.** That is three files and two components, inside both thresholds — **and it
leaves `L1`(b) open, which is *"**A public entry point** —
`draw_shadow_offscreen` is reachable only from `draw_shadow_batch`, i.e. only from
a `DrawCommand::Shadow` segment, so **no widget can ask for a backdrop**"*.**
**So the shrink is not smaller, it is a different deliverable**, and the operator
is the one who decides whether that is the one wanted. **This file recommends the
full seven**, and records the overshoot rather than disguising it.

## Requirements

1. **`ColourTarget` in `ui/src/ui_core/src/render/target.rs`**, `pub`, beside
   `ShadowTarget` and with its own module-level doc section named
   `## The colour target, and why it is a second type`. Its fields, in
   `ShadowTarget`'s order and with its own doc comments:

   ```rust
   pub struct ColourTarget {
       /// The framebuffer the capture writes into and the blur passes between.
       framebuffer: glow::Framebuffer,
       /// The two `GL_RGBA8` textures the separable blur passes between, and an
       /// array for the reason `ShadowTarget`'s is one.
       textures: [glow::Texture; 2],
       /// Which of `textures` holds what the next pass reads. **Not both zero**,
       /// for `ShadowTarget`'s reason: the two are never the same index.
       read: usize,
       /// Which of `textures` the next pass — the capture included — writes to.
       written: usize,
       /// The size the textures are allocated at, or `None` before the first
       /// allocation.
       size: Option<(u32, u32)>,
   }
   ```

   The type's doc records, in this order: what it is (**four channels, not one**)
   and its single caller; **why it is not `ShadowTarget` with a format argument**,
   in the three-part form § *The decision* gives; **why it is window-sized**,
   citing the ES 3.1 identical-bounds rule and naming `L1`(c) as the consequence
   rather than pretending the rect is the caller's to choose; that it allocates
   **lazily**, so a frame with no backdrop allocates nothing; and **what would
   reverse the decision** — a capture route that can be scoped to a rect, which
   § *The capture* names.

2. **`ColourTarget`'s public surface is `ShadowTarget`'s six names, with **one**
   method added, one `Drop`, and nothing else:**

   - `pub fn new(gl: &glow::Context) -> Result<Self, RenderError>` — allocates no
     storage, `read: 1` and `written: 0`, `size: None`, in `ShadowTarget::new`'s
     shape and for its stated reason.
   - `pub fn size(&self) -> (u32, u32)`, `#[must_use]`.
   - `pub fn ensure_size(&mut self, gl: &glow::Context, width: u32, height: u32)
     -> Result<(), RenderError>` — **window-sized, lazily, and
     `allocation`-decided**: it calls `allocation` and `resize_decision`
     unchanged, returns `Ok(())` immediately when `resize_decision` says no, and
     **refuses an extent past `max_texture_size(gl)` with `Err(RenderError::Gl)`
     before binding anything** — `ShadowTarget::ensure_size`'s rule and its
     recorded reason, *a rejected call with nobody reading it dropped a whole
     pass*. It then allocates with **`GL_RGBA8` internal and `GL_RGBA` format**,
     attaches with `bind_attach`, and **refuses an incomplete framebuffer with
     `Err(RenderError::Gl)`** naming the status — the same two-step check, and the
     same reason: on a GLES 3.1 implementation it can only happen if the driver
     disagrees about `GL_RGBA8` being colour-renderable, which is worth an error
     rather than a silently absent backdrop.
   - `pub fn capture(&mut self, gl: &glow::Context) -> Result<(), RenderError>` —
     **the addition, and it is one blit.** Doc comment in the order the code
     runs: the write texture is already attached and the viewport already set by
     `bind_for_write`, so the call binds `glow::READ_FRAMEBUFFER` to `None`
     **after** `bind_attach` — *the order is load-bearing, and it is a
     measurement: `glow::FRAMEBUFFER` binds both, so a blit issued before the
     read binding is a texture-to-itself blit, which is a documented
     `GL_INVALID_OPERATION`* — then `gl.blit_framebuffer(0, 0, width, height, 0,
     0, width, height, GL_COLOR_BUFFER_BIT, GL_NEAREST)`. **Both rectangles are
     the whole window and the doc comment says why they have to be**: the ES 3.1
     rule quoted in § *The capture*, and the fact that `GL_SAMPLE_BUFFERS` for
     this pipeline's read framebuffer is four.
   - `pub fn bind_for_write(&mut self, gl: &glow::Context)`, `pub fn bind_for_read
     (&self, gl: &glow::Context)`, `pub fn swap(&mut self)` — **`ShadowTarget`'s
     three bodies, `ShadowTarget`'s three doc comments, with `bind_for_write`'s
     scissor sentence restated for this target** and `swap`'s *"three swaps: one
     for the capture, one per blur pass, one for the composite"* — **the capture
     is a write, so it consumes a `swap` exactly as the mask draw does.**
   - **`impl Drop for ColourTarget`** with a body identical in decision to
     `ShadowTarget`'s: no GL object is deleted, because `Context::drop` tears the
     context down immediately after.

3. **One new private free function in `target.rs`,
   `fn allocate_texture(gl: &glow::Context, texture: glow::Texture, width: u32,
   height: u32, internal_format: u32, format: u32)`**, and **both**
   `ensure_size` bodies call it. It does the four `tex_parameter_i32` calls
   (`GL_TEXTURE_MIN_FILTER` and `GL_TEXTURE_MAG_FILTER` to `GL_LINEAR`,
   `GL_TEXTURE_WRAP_S` and `GL_TEXTURE_WRAP_T` to `GL_CLAMP_TO_EDGE`) and the one
   `tex_image_2d` with `Slice(None)`, carries `ShadowTarget::ensure_size`'s SAFETY
   comment verbatim — including *"`Slice(None)` allocates storage and leaves its
   contents undefined, which is what a target that is cleared or captured before
   every use wants"* — **and its format pair is an argument rather than a field,
   which is the whole of § *The decision*'s first reason.** `ShadowTarget`'s
   public behaviour is unchanged and its doc comments say nothing new; **requirement
   19's test asserts the two bodies' formats**, which is the only check that can
   catch the wrong one being handed to the wrong body.

4. **`target.rs`'s module doc gains a paragraph in the existing section
   `## Why the whole window, and not the shadow's own box`,** recording that the
   colour target **inherited the same decision for a different reason** and
   cannot do otherwise: *the shadow could have been sized to its caster's box;
   a backdrop cannot be sized to anything but the window, because the blit that
   fills it requires identical source and destination bounds.* The existing
   paragraph's *"the difference is the whole of the cost this module adds"* is
   **not** amended, because it is still true — and the new paragraph says what it
   costs in the numbers § *Bandwidth* gives, and names the measured fact that
   **this host cannot measure it** (`171/182/211` jiffies at nine taps against
   `191/172/158` with no shadow, 62.0–62.2 fps in all three sets) so the next
   reader does not try.

5. **The three new GL constants in `render.rs`'s existing block**, appended at
   the end rather than inserted, each one line with a doc comment giving the hex
   and naming where it was verified:

   - `GL_READ_FRAMEBUFFER: u32 = 0x8CA8`
   - `GL_DRAW_FRAMEBUFFER: u32 = 0x8CA9`
   - `GL_NEAREST: u32 = 0x2600`

   **A test pins all three against `glow::READ_FRAMEBUFFER`,
   `glow::DRAW_FRAMEBUFFER` and `glow::NEAREST`**, in task 34's requirement 3
   shape — *a duplicate with no test is two numbers that can drift*. **No
   `GL_BLIT_*` alias and no `GL_DEPTH_BUFFER_BIT` is added here**: task 34
   already declares the depth bit, and `GL_DEPTH_BUFFER_BIT` is **not** in this
   task's blit mask because the colour target has no depth attachment — the mask
   is `GL_COLOR_BUFFER_BIT` and only that, and the doc comment on `capture`
   says why a second bit would be a lie about what the target holds.

6. **`BackdropMode` in `ui/src/ui_core/src/paint.rs`**, immediately above
   `DrawCommand`, `#[derive(Clone, Copy, Debug, PartialEq)]`, two variants
   `Sharp` and `Blur(f32)`, each doc-commented in the house voice with the reason
   from § *The public API* — **`Blur(f32)` is a newtype's worth of argument and is
   still a bare `f32`, because `DrawCommand::Shadow`'s `blur` is one and two
   spellings of the same field is one thing to be wrong about.**

7. **`DrawCommand::Backdrop`, the eleventh variant**, placed **after `Polygon`**
   and before task 37's `Mesh`, with `#[derive(Clone, Debug, PartialEq)]`
   unchanged and every field doc-commented exactly as § *The public API* gives
   them — **the capture semantics (`tint.a == 255` replaces the rect), the
   recording contract (*scene, then backdrop, then chrome*) with
   `.ai/NEVERAGAIN.md` § *A shadow lands on whatever was recorded before it, not
   on whatever comes next* cited by name, and the fact that `rect` sizes the
   composite and not the capture, with the ES 3.1 rule beside it.** No
   `radius`: the composite shader is the shadow composite's shape with a sampler
   argument changed, and a rounded backdrop is § *Out of Scope*.

8. **`Painter::backdrop(&mut self, rect: Rect, mode: BackdropMode, tint: Color)`**
   in `ui/src/ui_core/src/paint.rs`, pushing `DrawCommand::Backdrop { rect, mode,
   tint }` in the other methods' shape, with a `# Examples` doc test in their
   form that constructs a `Painter`, calls `backdrop`, and pattern-matches — **a
   test that needs no display, because it records and never opens a window.**
   The doc comment names **`Color::new(255, 255, 255, 255)` as "as captured"**,
   says there is no constant for it because `Color::new` is not `const`, and
   points at `blur::MAX_TAPS` for the cap on `Blur`'s sigma.

9. **`ShaderKind::Backdrop` in `ui/src/ui_core/src/batch.rs`**, beside `Shadow`
   and `Mesh`, doc-commented for **what the kind is** and **why it can never
   merge**, in the two-part form task 37's requirement 3 established: a backdrop
   needs an offscreen target and a per-command rect, neither of which can ride
   in a vertex buffer, so two backdrops are two draw calls however their keys
   compare.

10. **`BatchKey::is_singleton` returns true for it** —
    `matches!(self.shader, ShaderKind::Shadow | ShaderKind::Mesh |
    ShaderKind::Backdrop)` — and **its doc comment is amended, not replaced**,
    keeping task 37's sentence about `submit_order` splitting the frame and
    updating *"The only such key is the shadow's"* and task 37's *"Two keys are
    singletons"* to three, with the shared reason stated once and each kind's own
    reason beside it. **A doc comment that says two where the code says three is
    the defect `DEMO_APPLICATION.md` § *Corrections to the second gap table*
    records twice**, so this is a requirement and not a nicety.

11. **`DrawCommand::batch_key` gains the `Backdrop` arm, and it decides the blend
    mode by rule rather than by `BlendMode::from_color`: `texture: None`,
    `shader: ShaderKind::Backdrop`, `blend_mode: BlendMode::Transparent`, always.**
    The arm's doc comment gives the reason from § *The public API* — *the
    capture's alpha is the framebuffer's accumulated coverage, and a translucent
    primitive recorded before the backdrop left it below 1.0 somewhere, which an
    `Opaque` batch would overwrite with a hard edge* — and names
    `BlendMode::from_color`'s rule as the one it declines.

12. **`Segment` gains `backdrop: Option<Batch>` beside `shadow` and `mesh`,** in
    the same shape as `mesh`'s: *"the boundary the segment ends with"*, and the
    struct's invariant restated as **at most one of the three is `Some`, because a
    seal consumes exactly one singleton command**. `Batcher::submit_order`'s
    trailing-run literal sets all three `None`, and its sealed-run `match` gains a
    **`ShaderKind::Backdrop` arm** — classified, not pushed back.
    **Not a `Boundary` sum type**, for task 37's stated reason: the invariant
    needs a test rather than a type, and a sum type would rename two `pub` fields
    that task 37's own tests and two of its doc examples name.
    **The existing test asserting no segment has two slots filled keeps its name
    and its assertion and gains the third slot.**

13. **`Pass::Backdrop` in `render.rs`**, the **fourth** variant of the private enum
    at this tree and the **fifth** where task 37 has landed — **the implementer
    counts, states the count they started from, and quotes it** — doc-commented as *"a captured
    scene, blurred and composited where it was recorded, with no depth test."*
    **`COMPOSITED_PASSES` stays `[Pass; 3]`**, and its doc comment gains task 37's
    sentence generalised: a backdrop is drawn at its recorded position and not at
    a fixed one. **`Renderer::draw_pass` gains the arm**, so the dispatch stays
    exhaustive and a future pass cannot be added without the compiler asking where
    it draws.

14. **`Renderer::end_frame`'s segment loop gains one `if let`,** beside the
    shadow's and task 37's mesh's, and the comment above them is amended to name
    three boundaries:

    ```rust
    // The backdrop, the shadow and the mesh each come after everything their
    // own segment recorded and before everything the next will. They cannot be
    // in the same segment — a seal consumes exactly one singleton command — so
    // their order relative to each other is not observable. The backdrop comes
    // first because it is the layer a caller wants *under* everything the rest
    // of the frame draws: the recording contract is scene, backdrop, chrome.
    if let Some(backdrop) = &segment.backdrop {
        self.draw_backdrop_batch(backdrop)?;
    }
    ```

15. **`Renderer` gains the state and the two draw functions.** Fields:
    `colour_target: ColourTarget`, `blur_colour_program: glow::Program`,
    `backdrop_composite_program: glow::Program`, `u_backdrop_blur_size`,
    `u_backdrop_blur_texel`, `u_backdrop_blur_direction`,
    `u_backdrop_blur_weights`, `u_backdrop_blur_taps`, `u_backdrop_blur_source`,
    `u_backdrop_size`, `u_backdrop_source`, `u_backdrop_tint` — **nine
    `Option<glow::UniformLocation>` fields, each queried in `Renderer::new`
    against the program it belongs to**, because a uniform location belongs to the
    program it was queried from and the existing six shadow/blur locations say so
    in their own field doc — and `colour_capture_legal: Option<bool>`, `None`
    until the first capture, per the accessor below.
    `pub fn colour_capture_legal(&self) -> Option<bool>`, `#[must_use]`,
    documented as *"whether the one-shot probe has run and what it said; `None`
    before the first backdrop"*.

    - **`fn draw_backdrop_batch(&mut self, batch: &Batch) -> Result<(), RenderError>`**,
      beside `draw_shadow_batch`, in its shape: `find_map` over `batch.commands`
      for `DrawCommand::Backdrop { rect, mode, tint }`, **`None` is
      `return Ok(())`** rather than a panic — the shape every `draw_*_batch` uses —
      then `self.apply_clip(batch.clip)`, then `self.draw_backdrop_offscreen(rect,
      mode, tint, batch.clip)`.
    - **`fn draw_backdrop_offscreen(&mut self, rect: Rect, mode: BackdropMode,
      tint: Color, clip: Option<Rect>) -> Result<(), RenderError>`**, beside
      `draw_shadow_offscreen`, in its shape and in its step order, and **the doc
      comment states every step's reason**:

      1. `colour_target.ensure_size(gl, width, height)?`, where `(width, height)`
         is `self.viewport`.
      2. **The resting state asserted locally**, immediately before the capture:
         `gl.disable(GL_DEPTH_TEST)`, `gl.depth_mask(false)`,
         `gl.disable(GL_BLEND)` — task 34's requirement 7's move, for 34's reason
         that depth test state is global and not a property of a framebuffer.
      3. `colour_target.bind_for_write(gl)` — attaches, sets the viewport,
         **disables the scissor**, in `ShadowTarget`'s shape and with its
         scissor sentence restated.
      4. **`colour_target.capture(gl)?`** — the blit, and **the probe**: on the
         first capture (`colour_capture_legal == None`), `gl.get_error()` is
         drained **before** the blit and read **after** it, and a non-zero result
         **returns `Err(RenderError::Gl(format!("colour capture is refused by this
         driver: 0x{code:04X} after glBlitFramebuffer from the default framebuffer —
         this context's default framebuffer holds {samples} samples, so ES 3.1
         requires the source and destination rectangles to have identical bounds
         and the two formats to match")))`** and records `Some(false)`;
         `NO_ERROR` records `Some(true)`, **with `{samples}` being
         `MULTISAMPLE_SAMPLES` interpolated, so the message names this host's
         number rather than a guess**. **Every
         later capture with `Some(false)` returns `Ok(())` without drawing**, and
         the doc comment says why that is not a silent failure: the first one
         errored, and repeating the probe every frame would cost a pipeline
         round trip on every backdrop forever.
      5. **Two blur passes**, in a `for direction in [(1.0_f32, 0.0_f32),
         (0.0_f32, 1.0_f32)]` loop exactly as `draw_shadow_offscreen`'s is, with
         `swap` + `bind_for_write` + `bind_for_read` per pass and **the colour
         program** and its own six locations. `u_backdrop_blur_texel`'s value is
         `1.0 / target.0.max(1.0), 1.0 / target.1.max(1.0)` — the target's, not
         the window's, because the target is what is sampled.
         **There is no `swap` between the capture and this loop, and the call-site
         comment says why in `draw_shadow_offscreen`'s own words: the capture went
         in through the *write* side and nothing reads it until the first pass, so
         the loop's first `swap` is spent turning the write target into the first
         pass's source. The two swaps inside the loop and the one after it are
         three, not two** — the same count, for the same reason, as the shadow's.
      6. `colour_target.swap()`, then **`self.bind_default_target(clip)`** — the
         framebuffer, the viewport, the scissor and (where task 34 has landed)
         the depth state, in one call, for the reason `bind_default_target`'s own
         doc gives: `bind_for_write` turned the scissor off and the composite is
         the pass that puts it on the screen.
      7. **The composite, over the rect and not the window**: the rect grown by
         `blur::reach(sigma)` (`0.0` for `Sharp`), intersected with the window, a
         `blur::rect_quad` of it uploaded through the existing `BlurQuad`, the
         backdrop program, `u_backdrop_size` set to the **window** size — the
         vertex shader divides *window* coordinates by it and a rect quad's UVs
         land over the full-window texture — `u_backdrop_source` to 0, and
         `u_backdrop_tint` to `quad_color(tint)`. **`gl.enable(GL_BLEND)` and
         `gl.blend_func(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`**, because the composite
         reads a destination.
      8. **The clip is the composite's, twice over, and the doc says so**: it is
         in `bind_default_target(clip)` and it is the rect, which is a second
         bound on the same pixels. **A caller whose clip is smaller than its
         `rect` gets the smaller one**, which is `draw_shadow_batch`'s recorded
         lesson applied.

16. **The two new shaders and the two new programs in `render.rs`,** each beside
    its shadow-path neighbour, each `const &str`, each doc-commented with the
    arithmetic it encodes:

    - `BLUR_COLOUR_FRAGMENT_SHADER_SRC` — the same `BLUR_FRAGMENT_SHADER_SRC`
      with the accumulator widened from `float total` to `vec4 total` and the
      write from `vec4(total, 0.0, 0.0, 0.0)` to `frag_color = total`. **The
      doc comment states the two things that are contracts:** the weights arrive
      from `blur::kernel` already normalised to one, so this is a plain dot
      product and a shader that normalised again would be a second place where
      the normalisation could be wrong; and **the blur runs on premultiplied
      colour, because `blur(rgb · a) ≠ blur(rgb) · blur(a)` and blurring straight
      colour fringes at every translucent edge.** `precision highp float;`, for
      `BLUR_FRAGMENT_SHADER_SRC`'s stated reason — a coverage sum over nine taps
      banding at `mediump`, and a colour sum is four times the range.
    - `BACKDROP_COMPOSITE_FRAGMENT_SHADER_SRC` — **the two lines of
      § *Premultiplied alpha*, verbatim**, over `BLUR_VERTEX_SHADER_SRC`, **which
      is reused unchanged**: its positions
      are window coordinates and its `v_uv` flip is correct for a rect quad as it
      is for a window quad, because `normalized` is `a_pos / u_size` either way.
      `create_program_with_fragment` is the linking shape, and its doc comment
      says why **no new vertex source is needed**, which is a third fewer
      `GL_` constant's worth of surface.

    **`fn create_backdrop_programs(gl: &glow::Context) -> Result<(glow::Program,
    glow::Program), RenderError>`**, beside `create_blur_programs` and in its exact
    shape — two `create_program_with_fragment` calls against the same
    `BLUR_VERTEX_SHADER_SRC`, wrapped in `Ok((…, …))` — and `Renderer::new` calls it
    beside `create_blur_programs`' call. **A new helper and not a four-tuple out of
    `create_blur_programs`**, for the reason the pair-returning helpers already
    exist: each names one layer's programs, and a caller that reads
    `create_blur_programs`' doc knows what it got.

17. **`fn backdrop_fragment(texel: [f32; 4], tint: [f32; 4]) -> [f32; 4]`**,
    private to `render.rs`, beside `mesh_fragment`'s place and for its reason: a
    pure CPU mirror of the composite's one arithmetic line, **so the
    premultiplication invariant can be asserted without a display**, carrying the
    obligation that the GLSL string and this function **must be edited together**.

18. **`fn rect_quad(rect: Rect) -> [blur::BlurVertex; blur::BLUR_QUAD_SIZE]` in
    `render/blur.rs`**, beside `full_quad`, in its exact shape — the same
    `[[0,0], [w,0], [w,h], [0,h]]` corner order, offset by `rect.x`/`rect.y`, six
    vertices, no index buffer — with a `# Examples` doc test in `full_quad`'s form
    asserting the six positions, **and with a doc comment stating the two things
    that are easy to get wrong**: the order is the same one `quad_indices` produces
    because the triangle winding is the same, and **`pos` is in window coordinates
    with y increasing downward**, which is why `BLUR_VERTEX_SHADER_SRC` negates
    `clip.y`.

19. **The tests, named, with no display, no network, no filesystem and no wall
    clock** — the only kind `AGENTS.md` permits. In `render/target.rs`'s
    existing `#[cfg(test)] mod tests`:

    - **`the_colour_target_is_rgba_and_the_shadow_target_is_red`** — the required
      one. It has two halves, and the first is the arithmetic rather than the
      search: `GL_RGBA8 == glow::RGBA8`, `GL_RGBA == glow::RGBA`,
      `GL_R8 == glow::R8`, `GL_RED == glow::RED`, **`GL_RGBA8 != GL_R8`** and
      **`GL_RGBA != GL_RED`**. The second half is the source-string assertion in
      `render.rs`'s and `blur.rs`'s own established shape: `include_str!
      ("target.rs")`, split at `"#[cfg(test)]"` so the assertion cannot be
      satisfied by its own words, scoped to each `ensure_size` body, asserting
      that `ColourTarget`'s contains `GL_RGBA8` and `GL_RGBA` and **does not
      contain `GL_R8` or `GL_RED`**, that `ShadowTarget`'s contains `GL_R8` and
      `GL_RED` and **does not contain `GL_RGBA8` or `GL_RGBA`**, and **a control**
      that both function names really are spelled the way the search expects, so a
      rename fails rather than quietly passing. **Mutation evidence in the
      handoff:** change `ColourTarget::ensure_size`'s pair to `GL_R8`/`GL_RED`
      and watch it fail for that reason; restore it and watch it pass. **A test
      that has never failed is a hypothesis** (`developer.md` § Phase 3).
    - `a_colour_capture_blits_the_same_rectangle_twice` — the § *The capture* rule
      as an assertion, in the same source-string shape, scoped to
      `ColourTarget::capture`'s body: it contains one `blit_framebuffer` whose
      four source arguments and four destination arguments are the same four
      numbers, `GL_COLOR_BUFFER_BIT`, and `GL_NEAREST`; and it contains **no**
      `GL_DEPTH_BUFFER_BIT`. **Mutation:** make the destination start at `0, 0`
      with a `rect.x`-derived offset, or drop one argument to a constant, and
      watch it fail — **this is the test that keeps `L1`(c) open on purpose**,
      because a blit that violates the rule is a capture that does not happen and
      a picture with no error.
    - `the_colour_target_allocates_lazily_and_resizes_with_the_window` — reuses
      `allocation` and `resize_decision` on `ColourTarget`'s behalf and asserts
      the `None` sentinel is what makes a 1×1 window allocate, in
      `a_zero_sentinel_for_the_allocation_would_hide_a_one_pixel_window`'s exact
      shape and with its control assertion kept.

    In `paint.rs`:
    - **`a_backdrop_records_one_command_and_names_its_rect_mode_and_tint`** — the
      doc test of requirement 8, plus a `mode` round-trip over **both**
      `BackdropMode` variants, and an assertion that `Blur(2.5)` and `Blur(0.0)`
      are **not** equal, which is the reason the mode is an enum.

    In `batch.rs`:
    - **`a_public_backdrop_request_reaches_the_renderer_as_a_segment_boundary`**
      — the required one. It drives the **public** `Painter::backdrop` and then
      the crate's own `Batcher` and `Batcher::submit_order`, asserting the request
      arrives as `Segment { .., backdrop: Some(batch) }` with
      `batch.key.shader == ShaderKind::Backdrop`, `batch.key.texture.is_none()`,
      `batch.key.blend_mode == BlendMode::Transparent`, and **`batch.clip` equal
      to the clip the command was recorded under** — because a clip is a draw
      call's scissor and a composite is a draw call, which is
      `Segment::shadow`'s own recorded reason. It also asserts **a command
      recorded after the backdrop lands in a later segment**, which is the property
      the singleton exists for.
      **The label on this criterion is exact and the handoff must repeat it: it
      proves the request reaches the renderer's submission structure, which is as
      far as a display-free test can reach.** The GL half is proved by the one-shot
      capture in requirement 21 and by nothing else.
    - `a_backdrop_batches_transparent_even_when_its_tint_is_opaque` — the
      unconditional `Transparent` of requirement 11, with `BlendMode::from_color`
      on the same tint named as the control that says no.
    - `no_segment_has_two_boundaries_filled` — task 37's invariant over **three**
      slots, with a fixture of `n` alternating shadows, meshes and backdrops
      asserting `n` boundaries across `n + 1` segments.
    - `a_backdrop_key_is_a_singleton_and_its_neighbours_still_are_not` — the
      required singleton assertion **with the existing controls
      `!rect().is_singleton()` and `!image().is_singleton()` kept present and
      unweakened.**

    In `render.rs`:
    - **`depth_state_for_the_backdrop_pass_is_no_test_and_no_write`** — both blend
      modes, added to task 34's table, **which is six rows there, eight where task
      37 has landed, and ten after this task**; a test iterating every pass × every
      blend mode keeps asserting all of them.
      **Mutation:** flip the `Backdrop` + `Opaque` row's `writes` and the suite
      fails.
    - **`the_backdrop_composite_stays_premultiplied`** — `backdrop_fragment` over a
      grid of contract-honouring texels and tints in `0.0..=1.0`, asserting `r ≤ a
      && g ≤ a && b ≤ a` componentwise. **Mutation evidence:** change the mirror to
      `vec4(texel.rgb, texel.a * tint.a)` — the straight-alpha mistake the solid
      pass makes — and watch it fail; then to `texel.rgb * texel.a * tint.rgb` —
      double-premultiplying, which is the other mistake — and watch it fail.
      **And a test that the shader string contains `u_tint.rgb` and does not
      contain `texel.a *`**, in the same source-string shape, because a CPU mirror
      that has drifted from its GLSL is a mirror of nothing.
    - **`the_composited_passes_are_drawn_solid_then_image_then_text` keeps its
      name and its assertion** and gains `!COMPOSITED_PASSES.contains(&Pass::Backdrop)`
      and, where task 37 has landed, `!COMPOSITED_PASSES.contains(&Pass::Mesh)`.
    - `the_backdrop_shaders_reuse_the_blur_vertex_source` — `BLUR_VERTEX_SHADER_SRC`
      is named by both new programs' creation path and neither new fragment source
      contains `gl_Position`, so no new vertex stage can drift in unannounced.

    In `render/blur.rs`:
    - `a_rect_quad_covers_the_rect_it_is_given` — six positions, both triangles,
      for an off-origin rect, with `full_quad`'s own doc test still passing
      unchanged.

    In `ui/src/ui_demo/src/main.rs`, **asserting the negative**:
    - **`no_gallery_page_records_a_backdrop`** — for each of `Page::ALL`'s six
      pages, `laid_out_on(page)`, one `frame(WINDOW, …)`, and every handle in
      `demo.order` asserted to record **no** `DrawCommand::Backdrop`. **With
      `a_page_records_no_command_on_a_node_that_is_not_its_own`'s vacuity control
      copied in**, because a sweep over an empty `order` proves nothing —
      `.ai/NEVERAGAIN.md` § *A sweep of a mechanism's call sites is not a sweep of
      the data it is built from*. **This test is what makes the pixel-identical
      criterion demanding rather than merely met.**

20. **The GL error, the capture and the frame rate are produced, and the handoff
    pastes the instruments' own output.**

    - **`gl.get_error()` is read once after the first backdrop** — once *per
      process*, because requirement 15's probe is the only reader — and the result
      pasted **with the instrument's code quoted**, per `.ai/NEVERAGAIN.md`
      § *A buffer sized for one vertex per quad*: `bind_attach`'s doc records that
      a rejected call with nobody reading it dropped a whole pass, and a **backdrop
      that silently does not draw is the largest version of that failure this
      sequence has produced.** **A capture is not a substitute** — a missing
      backdrop and a backdrop over a solid background look different, and only one
      of them is right.
    - **The frame rate on every page**, with `.ai/tools/fps-check.sh 10 55` on the
      default page and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo
      --tab=<page>` for each of `Page::ALL`'s six, the `roados-fps` line parsed by
      hand — **`fps-check.sh` reads `seconds` then `floor` and nothing else and
      runs the binary with no arguments**, verified in `.ai/tools/fps-check.sh`, so
      it cannot name a page. Every page above the floor of **55** and inside the
      baseline, which is **task 34's if task 34 has landed** and otherwise the
      **61.1–63.9 fps across six pages** band in `doc/ui/IMPLEMENTATION_STATE.md`
      § *The frame rate, measured* — **and the handoff says in which of those two
      registers it compared.** **The expected result is no measurable change, and
      the reason is stated rather than left as a coincidence:** `no_gallery_page_
      records_a_backdrop` passes, so the per-frame work added is **one `if let
      Some(backdrop)` per segment**, two program links in `Renderer::new`, and
      **zero** captures — because there is no caller, and § *Out of Scope* says
      no widget gets one in this task. **What the measurement is for** is the
      regression the added `if` could not cause and the first real backdrop's
      first capture will, and **the 131.9 MB figure is the number that first
      capture is measured against.**
    - **`Renderer::colour_capture_legal()`'s answer is recorded**, and so is the
      one-shot probe's `gl.get_error()` code, in `doc/ui/IMPLEMENTATION_STATE.md`.

21. **A temporary instrument, used once and reverted, because a GL pipeline passes
    every unit test in this file and still draws nothing.** `.ai/agents/developer.md`
    § Phase 3: *"A change that alters what is on screen is not verified until it
    has been seen."* After the instrumented capture below is inspected, the handoff
    says **in those words that the instrument is not in the tree**, and
    `git diff --stat` shows `ui/src/ui_demo/` unchanged. **This is task 34's
    requirement 11's arrangement applied to a whole pass rather than to a
    read-back.**

22. **`doc/ui/DEMO_APPLICATION.md` row `L1` is amended, dated, and kept.** It
    gains a dated note recording that `TASK_UI_PRIM_41` added
    `render::target::ColourTarget` (`GL_RGBA8`, four channels) and
    `Painter::backdrop` / `DrawCommand::Backdrop` as a `Segment` boundary, and
    **that sub-item (c), rect-scoped capture, is still open**, with the ES 3.1
    identical-bounds rule as the reason. **Its `Evidence` cell is rewritten by
    symbol name and file path** — `target.rs`'s `ColourTarget`,
    `render/blur.rs`'s `MAX_TAPS`, `render.rs`'s `draw_backdrop_offscreen` — and
    **not by line number**, for `AGENTS.md`'s reason that a line number is stale
    within days of being written. **The row is not deleted and not marked closed;
    its `Severity` stays `Critical` because (c) is open; and its `Blocks` column
    still names all five of every overlay — Controls, climate, media, app tray,
    alerts — because no widget consumes a backdrop and none of those five is
    delivered.** `L1`'s `Gap` cell's first sentence, *"No colour capture and no
    public backdrop API"*, is amended to name what now exists, so the row does not
    read as a claim about a tree that no longer matches it.

23. **`doc/ui/IMPLEMENTATION_STATE.md` gains one entry**, carrying: the type name,
    the four-channel arithmetic against `target.rs`'s one-channel arithmetic, the
    `GL_RGBA8` / `GL_RGBA` pair and the fact that it is a literal at the
    allocation; the public API's four-hop route; the capture's shape and the ES
    3.1 rule with the reason `(c)` stays open; **the bandwidth table's own numbers**
    — 131.9 MB per frame as implemented, 13.4 MB if rect-scoped, 40.5 MB for the
    shadow it is compared against — **and the recorded fact that this host cannot
    measure the difference**; the recording contract in one sentence; the
    premultiplication proof and **the honest limit that the backdrop is exactly as
    premultiplied as what it captured**; the probe's answer and its
    `gl.get_error()` code; the six pages' frame rates against the baseline the
    handoff names; **and the statement that no widget requests a backdrop yet and
    the first pixels of one are a later task's capture.**
    `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`.ai/workflows/task-sequence.md` § *State*); it points at the code.

24. **The suite is green and the counts are reported.** From `ui/`:
    `cargo fmt --check`, `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings`,
    `cargo test --all-features` with the per-binary counts pasted against the
    baseline — **`ui_core` 1 450 unit, `ui_demo` 224 unit, 220 doctests, 1 894 in
    all** — **and no test deleted, renamed away or weakened**; `cargo doc --no-deps`
    clean; `cargo audit` **recorded as not installed on this host, not passed**.
    **Every new `unsafe` block carries a SAFETY comment** naming *"The GL context
    is current on this thread"* and, where a call takes one, what the object is —
    the form `ShadowTarget::bind_attach`'s already uses. **No new `unsafe`
    surface beyond what a GL call requires**: no `transmute`, no `static mut`, no
    raw pointer arithmetic, no `unwrap`, no `expect`, no `panic!`/`unimplemented!`/
    `todo!` in any production path; `glow`'s `blit_framebuffer`,
    `bind_framebuffer`, `framebuffer_texture_2d` and `get_error` are already
    `unsafe fn`s on `HasContext`, so this adds **no** `unsafe` the crate did not
    already have — **and no format query, because the probe is the blit** (§ *The
    capture*), which is why `get_framebuffer_attachment_parameter_i32` appears
    nowhere in the change.

## Acceptance Criteria

- [ ] **The type exists, is a second type, and its texture is `GL_RGBA8` and not
      `GL_R8`.** `grep -c 'pub struct ColourTarget' ui/src/ui_core/src/render/target.rs`
      is **1**, and
      `awk '/^pub struct ColourTarget/,/^}/' ui/src/ui_core/src/render/target.rs`
      shows **`framebuffer`, `textures`, `read`, `written`, `size`** and nothing
      else — **no format field, no `Option<Format>`, no type parameter**, which is
      what § *The decision* decided and what makes the format a compile-time fact.
      `the_colour_target_is_rgba_and_the_shadow_target_is_red` asserts both halves
      and can fail: **mutation evidence in the handoff** — change
      `ColourTarget::ensure_size`'s pair to `GL_R8` / `GL_RED`, watch the suite
      fail for that reason, restore it, watch it pass. **A test that has never
      failed is a hypothesis** (`developer.md` § Phase 3)

- [ ] **`ShadowTarget`'s public contract is unchanged, and the two types do not
      share a format.**
      `awk '/^pub struct ShadowTarget/,/^}/' ui/src/ui_core/src/render/target.rs`
      shows the same five fields, and `grep -n 'pub fn ' ui/src/ui_core/src/render/target.rs`
      shows `ShadowTarget`'s six methods with **unchanged signatures** beside
      `ColourTarget`'s. `grep -n 'GL_RGBA8' ui/src/ui_core/src/render/target.rs`
      returns hits in **`ColourTarget::ensure_size` and in the doc comments only** —
      **no hit inside `ShadowTarget::ensure_size`'s body**, which
      `the_colour_target_is_rgba_and_the_shadow_target_is_red` asserts. And the
      handoff states in one sentence what the accepted duplication is: **four
      bookkeeping methods, and the cheapest reversal is named** — a third target
      moves them into a private `PingPongTarget` and neither public signature
      changes

- [ ] **The capture blits the whole window to the whole window, and a test says
      so.** `a_colour_capture_blits_the_same_rectangle_twice` asserts
      `ColourTarget::capture`'s body carries one `blit_framebuffer` whose source
      and destination rectangles are the same four numbers, `GL_COLOR_BUFFER_BIT`
      and `GL_NEAREST`, **and no `GL_DEPTH_BUFFER_BIT`**. **Mutation evidence:**
      offset the destination by `rect.x` / `rect.y`, or swap `GL_NEAREST` for
      `GL_LINEAR`, and watch it fail. **The mechanism is named because it is what
      makes the criterion meaningful: the ES 3.1 reference page for
      `glBlitFramebuffer` raises `GL_INVALID_OPERATION` when the read framebuffer
      is multisampled and the source and destination bounds differ**, and this
      pipeline's read framebuffer has four samples

- [ ] **The capture is full-window, and the doc comments in three files say so
      rather than the reader finding out from a blank picture.**
      `DrawCommand::Backdrop`'s `rect` field doc in `paint.rs`, `ColourTarget`'s
      type doc in `target.rs` and `capture`'s doc comment in `target.rs` **each**
      name that `rect` sizes the composite and not the capture, and **each cite the
      ES 3.1 rule**. **The handoff says what a reader who believed the field
      comment would get**: a blit the driver rejects, a backdrop that does not
      draw, and no GL error anybody read

- [ ] **A public backdrop request reaches the renderer, and a test names how far.**
      `a_public_backdrop_request_reaches_the_renderer_as_a_segment_boundary`
      constructs a `Painter`, calls the **public** `Painter::backdrop`, feeds the
      command through the crate's own `Batcher` and `Batcher::submit_order`, and
      asserts the request arrives as `Segment { .., backdrop: Some(batch) }` with
      `shader == ShaderKind::Backdrop`, `texture.is_none()`,
      `blend_mode == BlendMode::Transparent`, the recorded `clip` carried on the
      batch, **and a command recorded afterwards landing in a later segment**.
      **The label on this criterion is exact: this proves the request reaches the
      renderer's submission structure, which is as far as a display-free test can
      reach, and the handoff repeats it in those words.** The GL half is the
      capture below and is not claimed by this criterion

- [ ] **The route is eight hops and every one is named in a doc comment.**
      `grep -n 'Backdrop' ui/src/ui_core/src/paint.rs ui/src/ui_core/src/batch.rs
      ui/src/ui_core/src/render.rs` shows `BackdropMode`, `DrawCommand::Backdrop`,
      `Painter::backdrop`, `ShaderKind::Backdrop`, the `batch_key` arm,
      `is_singleton`, `Segment::backdrop`, `Pass::Backdrop`, `draw_pass`'s arm,
      `end_frame`'s `if let`, `draw_backdrop_batch`, `draw_backdrop_offscreen` —
      **and the handoff pastes the route as the chain it is.** `grep -c 'pub fn
      backdrop' ui/src/ui_core/src/paint.rs` is **1**, so there is one way in and
      not two

- [ ] **The ordering contract is stated, and the segment structure enforces it.**
      `Segment` has **three** boundary slots and **at most one is `Some`** —
      `no_segment_has_two_boundaries_filled` over a fixture of `n` alternating
      shadows, meshes and backdrops, asserting `n` boundaries across `n + 1`
      segments and no segment with two filled. **`COMPOSITED_PASSES` stays `[Pass;
      3]`**: `the_composited_passes_are_drawn_solid_then_image_then_text` keeps its
      name and its assertion and gains
      `!COMPOSITED_PASSES.contains(&Pass::Backdrop)`. **`DrawCommand::Shadow`'s doc
      and `DrawCommand::Backdrop`'s doc both state the recording contract** —
      *record the scene, then the backdrop, then the chrome* — with
      `.ai/NEVERAGAIN.md` § *A shadow lands on whatever was recorded before it,
      not on whatever comes next* cited by name in `Backdrop`'s

- [ ] **Depth: no attachment, no test, no write, and the policy is data.**
      `grep -rn 'depth\|DEPTH' ui/src/ui_core/src/render/target.rs` shows
      `ColourTarget` **declaring no renderbuffer and no depth attachment**, and the
      `ColourTarget` type doc says why in task 34's own terms. `capture`'s blit
      mask is `GL_COLOR_BUFFER_BIT` and only that. `draw_backdrop_offscreen`
      contains `gl.disable(GL_DEPTH_TEST)` and `gl.depth_mask(false)` immediately
      before the capture. **`depth_state_for` is asserted for every pass × every
      blend mode — six rows at this tree, ten where task 37 has landed —**,
      `Backdrop` + either mode being `{ test: false, writes: false }`, **and the three 2D rows are asserted identically before and
      after this change.** **Mutation:** flip the `Backdrop` row's `writes` and the
      suite fails. And `git diff --stat` shows **no change to `DEPTH_BITS`, the
      resting state, `GL_LESS`, or `bind_default_target`** — task 34's four are
      inherited, not edited

- [ ] **Premultiplied throughout, and the solid pass's defect is neither
      inherited nor fixed.** `the_backdrop_composite_stays_premultiplied` asserts
      `r ≤ a && g ≤ a && b ≤ a` componentwise over a grid of contract-honouring
      inputs, **mutation evidence in the handoff** for both mistakes — the
      straight-alpha form `vec4(texel.rgb, texel.a * tint.a)` and the
      double-premultiplying form `texel.rgb * texel.a * tint.rgb` — and a test that
      the GLSL contains `u_tint.rgb` and does **not** contain `texel.a *`.
      **And the handoff carries the honest limit in the project's own register:**
      the backdrop is exactly as premultiplied as what it captured, a translucent
      **solid** primitive recorded before it contributes straight alpha to the
      capture, **and this task neither introduces a second instance of that defect
      nor fixes the first** — citing `chart.rs`'s module docs
      § *What made the first attempt at this measurement wrong* and
      `doc/ui/IMPLEMENTATION_STATE.md`
      § *The finding that is not this task's: the solid pass does not premultiply*.
      **`git diff --stat` shows no change to `FRAGMENT_SHADER_SRC`,
      `SHADOW_FRAGMENT_SHADER_SRC`, `SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC`,
      `BLUR_FRAGMENT_SHADER_SRC`, `quad_color` or `Color`**

- [ ] **The colour blur is on premultiplied colour, and the shader says why.**
      `BLUR_COLOUR_FRAGMENT_SHADER_SRC` differs from `BLUR_FRAGMENT_SHADER_SRC` in
      the accumulator (`vec4 total`) and the write (`frag_color = total`) and in
      **nothing else**, a test asserts that by substring, **and its doc comment
      carries `blur(rgb · a) ≠ blur(rgb) · blur(a)` as the reason.** `grep -c
      'precision highp float;' ui/src/ui_core/src/render.rs` is **at least** the
      count it is today plus one

- [ ] **The composite is a self-contained draw, and the rejected alternative is
      written down.** `COMPOSITED_PASSES` has three members and `Pass::Backdrop` is
      not one of them; `draw_backdrop_offscreen` ends by drawing the composite and
      returning, and **no shader in the file samples the backdrop for a later
      pass** — `grep -n 'u_backdrop' ui/src/ui_core/src/render.rs` shows the **nine**
      new locations, queried in `Renderer::new` and **written only inside
      `draw_backdrop_offscreen`**.
      `BACKDROP_COMPOSITE_FRAGMENT_SHADER_SRC`'s doc comment names the rejected
      arrangement (a second sampler in the image and text programs, one captured
      texture per segment) with both of § *A self-contained draw*'s reasons

- [ ] **The composite is rect-limited and the target is not, and the dependency
      between them is written down.** `rect_quad` exists in `render/blur.rs` with a
      doc test, `draw_backdrop_offscreen` calls it with the rect grown by
      `blur::reach(sigma)`, **and the two facts are stated as one dependency:**
      *a rect-limited composite is only correct because the capture was
      full-window, which is what gives the blur texels the rect's edges need.*
      **Mutation:** composite the window-sized quad instead and the rect's edge
      texels come from outside it — **a test cannot see that, and the handoff
      says so** rather than claiming a criterion for it

- [ ] **The bandwidth reasoning is in the file, in numbers, with the decision
      named.** `render.rs` gains a `## Backdrops` module section carrying
      § *Bandwidth*'s table: **131.9 MB per frame as implemented** (26.1 capture +
      104.4 two 9-tap blurs + 1.4 rect-limited composite, at
 1 305 600 px), **13.4 MB if rect-scoped — 90 % cheaper and not implemented**,
  **34.1 MB at half resolution**, **40.5 MB for the `GL_R8` shadow it is compared
  against**, **3.3×**, **2.19 Gpixel-accesses per second at 60 fps**, and the
  threshold *"any part below about 2 GPix/s of RGBA8 fill cannot afford a
  full-window backdrop every frame"*. **And the section states the four numbers
  that make this a decision rather than a guess**: `171/182/211` jiffies at nine
  taps, `212/202/188` at seventeen, `191/172/158` with no shadow, **62.0–62.2 fps
  in all three sets** — so this host cannot measure it and the arithmetic decides.
      **The implemented subset is named as a list**: rect-limited composite,
  `taps_for`'s cap, and "not paid at all on a frame that records none"

- [ ] **`cargo test --all-features` is green against the 1 894 baseline and the
      handoff lists every new test by name.** Baseline: **`ui_core` 1 450 unit,
      `ui_demo` 224 unit, 220 doctests — 1 894 in all.** **No test was deleted,
      renamed away or weakened.** **The three tests this change amends**
      — `the_composited_passes_are_drawn_solid_then_image_then_text` in
      `render.rs` and
      `a_zero_sentinel_for_the_allocation_would_hide_a_one_pixel_window` in
      `target.rs`, and `a_page_records_no_command_on_a_node_that_is_not_its_own`
      in `ui_demo` —
      **keep their names and their assertions and gain clauses rather than losing
      any**, and **the handoff names the fourth amended test — the at-most-one
      invariant task 37's requirement 5 adds for `Segment` — by the name it was
      given there, which task 37 does not fix**, so this task amends a name rather
      than a string.
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host;
      that is **recorded, not passed**

- [ ] **Every new `unsafe` block has a SAFETY comment, and there is no new
      `unsafe` beyond a GL call.** `grep -c 'SAFETY' ui/src/ui_core/src/render/target.rs`
      and the same over `render.rs` are **at least** the number of new `unsafe`
      blocks on top of their current counts. `grep -rn 'transmute\|static mut'
      ui/src/` returns **nothing**, and `grep -n 'unwrap()\|expect(\|panic!\|unimplemented!\|todo!'
      ` over the new code returns **nothing**. **No new dependency**:
      `ui/Cargo.toml` and `ui/Cargo.lock` are **unchanged** and the approved direct
      dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38` — per
      `AGENTS.md`, an image-processing crate or a FBO helper for a blit and an
      `f32` clamp is a licence decision against GPLv3 that nobody has asked for

- [ ] **The six gallery pages are pixel-identical, and the mechanism is stated
      rather than hoped for.** `Page::ALL`'s six names, release build, captured
      **before and after** with the commands of `doc/ui/IMPLEMENTATION_STATE.md`
      § *Verifying a change that draws — the capture method* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture, and `ffmpeg x11grab`, return black for a GL window),
      `pgrep -a -x ui_demo` in the same call as each `magick import -window <id>`,
      then `magick compare -metric AE before.png after.png null:` per page.
      **AE 0 outside `y ≥ 680`** on all six, every differing pixel inside the fps
      readout's band — which `doc/ui/IMPLEMENTATION_STATE.md`
      § *Task 24.1 — what it decided, and what it found* records as the one thing
      two captures of an unchanged frame differ in (405 pixels there, **AE 0 over
      y 80–680**). The rect-level half keeps its names and its assertions:
      `every_page_places_every_rect_where_the_gallery_placed_it` and
      `no_two_placed_rects_overlap`.
      **The mechanism that makes it achievable is two facts and one is a test:**
      **`no_gallery_page_records_a_backdrop` asserts no page records one**, with
      `a_page_records_no_command_on_a_node_that_is_not_its_own`'s vacuity control
      copied in — so the added `if let Some(backdrop)` is unreachable from all six
      pages and the criterion is **demanding rather than merely met**; and the
      `Capture` never allocates, because `ColourTarget::new` allocates no storage
      and `ensure_size` is only reached from `draw_backdrop_offscreen`

- [ ] **The frame rate is measured on every page and reported against a named
      baseline.** With the script's own line pasted rather than the number
      expected: `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
      `Page::ALL`'s six with the `roados-fps` line parsed by hand — **`fps-check.sh`
      takes `seconds` then `floor` and runs the binary with no arguments, so it
      cannot name a page**, verified in `.ai/tools/fps-check.sh`. Every page above
      the floor of **55**.
      **The baseline is named explicitly and the handoff says which one it used:**
      **task 34's, if task 34 has landed; otherwise the 61.1–63.9 fps across six
      pages band in `doc/ui/IMPLEMENTATION_STATE.md`
      § *The frame rate, measured*.**
      **The expected result is no measurable change, and the reason is stated
      rather than left as a coincidence**: no page records a backdrop, so the
      per-frame work added is one `if let` per segment and two program links in
      `Renderer::new`, and **zero captures**
      — **and 131.9 MB per frame is the number a real backdrop is measured against,
      not the number this task costs**

- [ ] **One capture shows a backdrop on screen, with the instrument reverted.**
      A **temporary** change to the demo's `overlays` page records exactly one
      `Painter::backdrop(Rect::new(…), BackdropMode::Blur(2.0), Color::new(255,
      255, 255, 255))`; the page is captured by the method above, **the pixels are
      looked at**, and the handoff says what they show — a blurred copy of the
      dialog host inside the rect, and the chrome drawn after it still sharp above
      it. **Then the instrument is reverted and `git diff --stat` shows
      `ui/src/ui_demo/` unchanged**, which is what makes the six-page criterion
      above a property of the final tree and not of the instrumented one.
      **The mechanism is `developer.md` § Phase 3's own:** *"a GL pipeline passes
      every unit test and still draws nothing"*, and **a capture of a page that
      records no backdrop is a capture of a page that records no backdrop**

- [ ] **No GL error, read once after the first backdrop, and the probe's answer
      recorded.** `gl.get_error()` is drained before the blit and read after it,
      the code pasted into the handoff **with the instrument's code quoted**, and
      **`Renderer::colour_capture_legal()`'s answer recorded in
      `doc/ui/IMPLEMENTATION_STATE.md`**. **A capture is not a substitute** — the
      absence of a backdrop and a backdrop that drew nothing are the same picture,
      which is the whole of `.ai/NEVERAGAIN.md`
      § *A buffer sized for one vertex per quad* in one sentence

- [ ] **`L1` is amended, dated, and still open.** Row **`L1`** in
      `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* carries a
      dated note naming `TASK_UI_PRIM_41`, stating that
      `render::target::ColourTarget` (`GL_RGBA8`) and `Painter::backdrop` now
      exist, **that sub-item (c), rect-scoped capture, is still open** with the ES
      3.1 identical-bounds rule as the reason, and that the row's `Gap` sentence is
      amended to match. **Its `Evidence` cell is rewritten by symbol name and file
      path** — `ColourTarget` and `capture` in `target.rs`, `MAX_TAPS` and
      `rect_quad` in `blur.rs`, `draw_backdrop_offscreen` and `Pass::Backdrop` in
      `render.rs` — **and carries no line number**, for `AGENTS.md`'s reason that a
      line number into a document or a source file is stale within days of being
      written. **The row is not deleted, not marked closed, its `Severity` stays
      `Critical`, and its `Blocks` column still names all five blocked overlays**
      — Controls, climate, media, app tray, alerts — **because no widget requests
      a backdrop and none of the five is delivered**

- [ ] **The decisions are written down where the next agent finds them.**
      `render/target.rs` carries `## The colour target, and why it is a second
      type` and a paragraph in `## Why the whole window, and not the shadow's own
      box`; `render.rs` carries `## Backdrops` with the bandwidth table and the
      measured fact that this host cannot measure it; `render/blur.rs`'s module doc
      gains the premultiplied-convolution sentence;
      `ui/src/ui_core/src/render.rs`'s `## Depth` section (task 34's) gains the
      `Backdrop` row; and `doc/ui/IMPLEMENTATION_STATE.md` carries requirement 23.
      **And no doc comment in any changed file asserts the opposite of the code
      beside it** — which is the defect `DEMO_APPLICATION.md`
      § *Corrections to the second gap table* records twice, and which three
      specific comments in this change would otherwise produce: `is_singleton`'s
      *"two keys"*, `Segment`'s invariant, and `ShadowTarget`'s *"The texture is
      `GL_R8`"* if it were left unqualified beside a second target

- [ ] **Nothing from a later task leaked in, and the sibling counts are
      reconciled.** `git diff --stat` shows **no change** under
      `ui/src/ui_demo/` (requirement 21), **no change to `layout.rs`** and **no
      `LayoutMode::Grid` row** — `LayoutMode::Grid { .. } => Vec::new()` stands.
      The handoff records **the counts this change started from and ended at** —
      `DrawCommand` variants, `ShaderKind` variants, `Pass` variants,
      `COMPOSITED_PASSES`' length, `Segment`'s boundary slots — **and, if tasks 34
      through 40 have landed, a dated amendment to the counts their own files
      quote**, because task 40's § *Out of Scope* names *"`DrawCommand`'s ten
      variants, `ShaderKind`'s five, `Segment`'s two boundary slots"* as things it
      does not touch, and **a number that is wrong is the failure
      `DEMO_APPLICATION.md` § *Corrections to the second gap table* was written
      about**

## Out of Scope

- **No widget requests a backdrop, and that is the deliberate limit of this task.**
  **No change to `ui/src/ui_core/src/widgets/mod.rs` at all** — not one `pub mod`
  entry added, renamed or removed — and no `Dialog` change, no `Toast`, no new
  widget, **and no `DrawCommand::Backdrop` recorded anywhere in the tree**
  afterwards. `Painter::backdrop` and `DrawCommand::Backdrop` exist, are tested
  through the crate's own `Painter` and `Batcher`, and **have no caller**, which
  is the same state tasks 34, 35, 37 and 38 recorded in their own words — *"the
  first pixels of a mesh in this codebase are task 38's first capture"* — restated
  for a backdrop. **The widget that consumes it — a card, a media panel, a climate
  surface — is a later task, and it is the reason `L1`'s `Blocks` column keeps all
  five entries.** Requirement 21's one-shot instrument is **not** a widget change
  and **is reverted**.

- **No rect-scoped capture and no rect-scoped blur — `L1`(c), still open.** Not
  deferred for want of time: **it is not reachable by `glBlitFramebuffer` from
  this pipeline's multisampled default framebuffer**, because the ES 3.1 reference
  page makes differing source and destination bounds an
  `GL_INVALID_OPERATION` whenever `GL_SAMPLE_BUFFERS` for the read buffer is
  greater than zero, and ours is four. **The route that would make it possible is
  a re-submission of the already-recorded commands into a scissored target rather
  than a copy of the default framebuffer**, and that is a task with its own
  decision. **A test asserts the constraint holds** so the row's remaining gap is
  enforced rather than documentary

- **No half-resolution tier and no multi-resolution tiering at all.** A half-scale
  blit is the *same* `GL_INVALID_OPERATION` rule, so a tier cannot be built on the
  blit either — it needs the re-submission capture above first. **Not implemented,
  and the 34.1 MB it would cost is in § *Bandwidth*'s table so the next agent
  starts from the number rather than from the idea.** A tier also needs its own
  question answered — what a backdrop at half resolution is allowed to be, given
  the text drawn over it is not tiered — and that is a design question, not a
  constant

- **No dirty-flag, and the strong form is not implementable here.** "Skip the
  capture when nothing behind the rect changed" needs a content identity per
  region; **the batcher holds batches, `Painter` is called every frame, and there
  is no retained previous frame and no dirty-rect protocol anywhere in the crate**
  — so there is nothing to compare against and promising it would be promising a
  mechanism this task does not have. **The weak form — a backdrop costs nothing on
  a frame that records none — is true by construction**, because a backdrop is a
  `Segment` boundary and a frame with none has one segment

- **No MSAA resolve for the new target, and no multisample attachment.** **The
  resolve is the specification's**: the ES 3.1 `glBlitFramebuffer` page says a
  multisampled read framebuffer's samples *"are converted to a single sample
  before being written to the destination"*, so there is **no multisample
  renderbuffer to allocate and no resolve code to write.** What the resolve costs
  is stated in § *The capture* rather than hidden: the reference page says only
  that the samples *"are converted to a single sample"* and does not say how, so
  the capture's antialiasing is whatever the driver's resolve is — **not** the
  coverage integral `chart.rs`'s module docs measured for this pipeline — and a
  blurred edge is softer than a sharp one in any case. `MULTISAMPLE_BUFFERS` and
  `MULTISAMPLE_SAMPLES` are **not** changed, the shadow FBO is **not** given
  samples, and **no `Renderbuffer` object appears in the crate** as a result

- **No shadow mapping, and no depth attachment on either target.** A shadow map
  needs a depth-only FBO; task 34 decided the shadow FBO gets no depth attachment
  and recorded that *"a future 3D backdrop would need one, and that is gap `L1`'s
  work"* — **and this task answers that sentence: it does not.** A backdrop's
  capture is a `GL_COLOR_BUFFER_BIT` blit and the blur and the composite are 2D
  passes, which under task 34's recorded policy neither test nor write depth, so a
  depth attachment would have to be written by a second blit whose formats must
  match and whose bounds must be identical — more constraints for a buffer nothing
  reads. **Task 34's four decisions are inherited by name and none is edited**:
  `DEPTH_BITS`, the resting state, `GL_LESS`, and the blend/depth rule

- **No layout change and no `Grid`.** `LayoutMode::Grid { .. } => Vec::new()` in
  `layout.rs`'s arrange match stands, `columns` is still declared and never read,
  `Flex.wrap` is still accepted and discarded by the same `..`, and **a backdrop's
  `rect` is a caller-supplied window-space rect and not a layout result** — which
  is what makes it composable with `L3` when that task lands without this one
  knowing. **No margin, no `flex-shrink`, no `flex-basis`, no cross-axis gap** is
  added or used

- **No rounded backdrop and no per-pixel mask.** `DrawCommand::Shadow` carries a
  `radius` and `SHADOW_MASK_FRAGMENT_SHADER_SRC` clips it with a signed distance;
  **`DrawCommand::Backdrop` has no `radius`**, so a backdrop is a rectangle, and
  `BACKDROP_COMPOSITE_FRAGMENT_SHADER_SRC` has no SDF and no `discard`. A rounded
  backdrop needs the mask shader's distance expression in the composite — which is
  a second fragment source and a second branch in the pass that costs 131.9 MB a
  frame — and is named as later work rather than half-built

- **No change to `ShadowTarget`'s sizing decision, and none of its arithmetic.**
  Its `GL_R8`, its window-sized target, its `MIN_TARGET_EXTENT` of 1, its
  `resize_decision` pair and its `max_texture_size` refusal are **all inherited,
  none amended**, and its module doc § *Two textures, one framebuffer* is still
  true. **What § *Why the whole window, and not the shadow's own box* gains is one
  paragraph saying the colour target inherited the decision for a different reason
  and cannot do otherwise** — the shadow could have been sized to its caster's
  box, and a backdrop cannot

- **No new dependency, and no `unsafe` beyond a GL call.** Per `AGENTS.md` the
  approved direct dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs
  0.38`; an image crate for a `tex_image_2d`, a FBO helper for four GL calls, or a
  maths crate for two clamps is a licence decision against GPLv3 that nobody has
  asked for. **`glow`'s `blit_framebuffer` already exists and is already
  `unsafe`**, so this adds **no** `unsafe` the crate did not already have:
  `grep -c unsafe ui/src/ui_core/src/render/target.rs` grows by the number of GL
  calls `ColourTarget::capture` and its `ensure_size` make and by nothing else

- **No change to `ui_demo`, and no change to any of the six pages.**
  `Page::ALL` stays `[Page; 6]`, `Page::DEFAULT` stays `Pads`, the `--help` text is
  unchanged, and no new `--tab=` name exists. **Requirement 21's instrument is
  reverted before the commit**, and the six pages must be pixel-identical
  afterwards — **and the six-page criterion would be unverifiable rather than
  merely demanding if the instrument were left in, which is why requirement 21
  ends with `git diff --stat`**

- **No change to the shadow path, and none of task 34's or task 37's decisions
  re-opened.** `SHADOW_MASK_FRAGMENT_SHADER_SRC`,
  `SHADOW_FRAGMENT_SHADER_SRC`, `SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC`,
  `BLUR_VERTEX_SHADER_SRC`, `BLUR_FRAGMENT_SHADER_SRC`, `blur::MAX_TAPS`,
  `blur::SOLID_BLUR`, `blur::taps_for`, `blur::kernel`, `blur::reach`,
  `blur::full_quad`, `draw_shadow_batch`, `draw_shadow_offscreen`,
  `bind_default_target`, `apply_clip`, `COMPOSITED_PASSES`' three members and
  `Renderer::end_frame`'s three composited passes — **all untouched by this task,
  and
  checkable by `git diff --stat`.** The only thing this change adds next to them is
  one more blur program and one more composite program, both from
  `BLUR_VERTEX_SHADER_SRC`