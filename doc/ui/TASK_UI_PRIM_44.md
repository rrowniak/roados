# TASK_UI_PRIM_44: `ui_core::widgets::Icon` — a Tintable Image Path, and the Widget That Uses It

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Close gap **`#4`** of `doc/ui/DEMO_APPLICATION.md` § *Library gaps* by doing the two of its three named needs that are **library** work — **theme tinting** becomes a pipeline capability once for every textured quad, and **uniform sizing** becomes the rule a new `Icon` widget enforces — while **vector rendering** stays offline in `TASK_UI_PRIM_39`'s resvg bake. The deliverable is `pub mod icon`, the shader and vertex change that makes a tint possible, and one widget that consumes both.

## Context

**Decision: (A), a tintable texture path** — `DrawCommand::Image` gains a `tint: Color`, the image shader multiplies it, and a new `Icon` consumes it; **(B), a procedural glyph path, and (C), a font-glyph-backed icon set, are declined**. (A) is source-breaking only to `DrawCommand::Image` construction and exhaustive destructuring (`paint.rs`, `batch.rs`, `widgets/image.rs`'s doc-test); `ui_core` is `publish = false`. The tint rides per-vertex, not in `BatchKey` (`{ texture, blend_mode, shader }`), so thirty-three coloured glyphs stay one draw call; its opaque-white identity leaves existing images unchanged.

**Gap `#4`'s needs split:** tinting and sizing here; vector rendering in `TASK_UI_PRIM_39`'s offline resvg bake. Its bytes are absent (`ui/src/ui_demo/assets/` holds `demo.png`, no `icons/`), so the demo draws no icon and `ui/src/ui_demo/src/main.rs` is untouched — a placeholder is forbidden by operator decision 3 of **2026-09-30**: *"Real icons. … No placeholder geometric shapes."*

**`ICON_SIZE = 24.0`**, authored at the drawn size: `MIN_FILTER` is `GL_LINEAR` with no `generate_mipmap`, so a 256-pixel master (**10.67 : 1**) aliases; the widget never scales and a smaller cell crops it. `Icon::tint` is a `pub Property<Color>` the caller assigns, so `Theme::switch_to` animates it with no clock in `Icon`. Verification is `.ai/agents/developer.md` § *Phase 3*; task-specific: **no gallery page is expected to move**, and `gl.get_error()` is read once after the first frame that draws the `data` page's image.

## Requirements

Requirements **11–12** (state-file record-keeping) are dropped and **13** is folded into § *Context*'s verification line; survivors are renumbered.

1. **`pub mod icon;`** in `ui/src/ui_core/src/widgets/mod.rs`, **alphabetically** between `gauge` and `image`, making **sixteen** entries; nothing else changes and `Callback<T>` is untouched.

2. **`DrawCommand::Image` gains a `tint: Color` last field** in `ui/src/ui_core/src/paint.rs`, documented as a premultiplied modulate, **not in `BatchKey`** (see `Batch::clip`), `opacity` unchanged (clamped `0.0..=1.0`). Neutral tint: **`pub fn untinted() -> Color`**, `#[must_use]`, `Color::new(255, 255, 255, 255)`, with a doc-test — `pub` because `Icon::new` and `Icon::reset_tint` need it.

3. **`Painter::image` keeps its five-argument signature** and delegates to `pub fn tinted_image(&mut self, rect: Rect, texture: TextureId, uv: UvRect, opacity: f32, radius: f32, tint: Color)`. **`Image::paint` is the crate's only production caller and is untouched** (`list.rs`'s `painter.image(` is a `#[cfg(test)]` fixture); `tinted_image`'s doc gives the premultiply arithmetic.

4. **`render.rs`.** `ImageVertex` gains `tint: [f32; 4]` last; `const IMAGE_VERTEX_STRIDE: i32 = 56;`, `const IMAGE_TINT_OFFSET: i32 = 40;`. `image_quad` takes `tint: Color` after `radius` and writes `quad_color(tint)`; `draw_image_batch` passes it. A new `gl.enable_vertex_attrib_array(6)` + `gl.vertex_attrib_pointer_f32(6, 4, GL_FLOAT, false, IMAGE_VERTEX_STRIDE, IMAGE_TINT_OFFSET)` sits beside the other five; the SAFETY comment names the new field.

5. **`render.rs` shaders.** `IMAGE_VERTEX_SHADER_SRC` gains `layout(location = 6) in vec4 a_tint;`, `out vec4 v_tint;`, `v_tint = a_tint;`; `IMAGE_FRAGMENT_SHADER_SRC`'s last line is exactly `frag_color = vec4(texel.rgb * v_tint.rgb * opacity, texel.a * v_tint.a * opacity);`, doc'd unclamped, not `flat`, old line at opaque white.

6. **`batch.rs`.** `DrawCommand::batch_key`'s image arm becomes `if uv.is_full() && *opacity == 1.0 && tint.a == u8::MAX`; `tint` is **not** destructured into the key.

7. **`widgets/image.rs`:** only the `Image::paint` doc-test changes (gains a `..` or names `tint`, keeping every assertion); `Image`'s struct, three public properties, four `ImageFit` variants and `AnimationClock` are unchanged and `paint` still calls `Painter::image`.

8. **`ui/src/ui_core/src/widgets/icon.rs`, new**, no `#[derive]`. Module doc records what an icon is and is not, the per-vertex tint rationale, the sizing/aliasing and crop-not-scale rules, caller-named glyphs, and that no icon is drawn yet.

   `pub const ICON_SIZE: f32 = 24.0;` · `pub struct Icon { pub tint: Property<Color>, pub opacity: Property<f32>, texture: TextureHandle, source: ImageSource, node: Handle }` · `pub struct Style { pub tint: Color, pub opacity: f32 }`

   ```
   new(nodes: &mut Arena<WidgetNode>, texture: TextureHandle, source: ImageSource) -> Self   Image::new's signature; tint = paint::untinted(), opacity = 1.0; doc example asserts the node is in the arena and the centred drawn rect
   handle(&self) -> Handle / set_tint(&mut self, tint: Property<Color>) / reset_tint(&mut self)   #[must_use]; reset_tint restores paint::untinted()
   texture(&self) -> TextureHandle / set_texture(&mut self, TextureHandle)
   source(&self) -> ImageSource / set_source(&mut self, ImageSource) / source_size(&self) -> (u32, u32)
   size(&self) -> Size                                              #[must_use]   Size::new(ICON_SIZE, ICON_SIZE)
   destination(&self, rect: Rect) -> Rect                           #[must_use]   centred ICON_SIZE square ∩ rect, via private centred_square(rect: Rect, side: f32) -> Rect
   style(&self) -> Style                                            #[must_use]   opacity via private bounded(value: f32, low: f32, high: f32) -> f32, NaN -> 0.0
   paint(&self, rect: Rect) -> Vec<DrawCommand>                     #[must_use]   one Painter::tinted_image; empty destination records nothing
   ```

   **No** `on_event`, `state`, `selected`, `fit`, `corner_radius`, `clock`, `tick`, `snap_to_state`, `animate_to_state`, `is_animating`, `size_hint` or `layout_mut` wrapper.

9. **The tests, named, with no display, network, filesystem or wall clock.** In `icon.rs`: `an_icon_is_twenty_four_pixels_square`; `icon_size_fits_the_shared_image_atlas` (binds `u32::try_from(ICON_SIZE).unwrap_or(u32::MAX)`, asserts `<= texture::ATLAS_MAX_IMAGE` and `<= texture::ATLAS_SIZE / 64`); `an_icon_paints_exactly_one_tinted_image_command`; `the_icon_is_centred_in_its_rect`; `every_icon_records_a_rect_inside_the_rect_it_was_given`; `the_icon_carries_the_tint_it_was_given`; **`a_theme_switch_reaches_the_icon`**; `an_untinted_icon_paints_opaque_white`; `an_icon_follows_a_written_tint_at_once`; `an_icon_paints_square_corners`; `the_icon_samples_the_uv_window_it_was_given`; `an_icon_in_a_translucent_state_records_the_opacity_it_holds`; `centred_square_is_pure`. `a_theme_switch_reaches_the_icon` binds `ThemeToken::Text` with `Property::bind` over `theme.property(token)`, assigns `set_tint`, and proves the tint is dark `Text`, then strictly between after `switch_to(Theme::light(), 300)` and `tick(Duration::from_millis(150))`, then exactly light `Text` — with no `Icon` clock. In `paint.rs`: `painter_image_records_the_neutral_tint`, `painter_tinted_image_records_the_tint_it_was_given`. In `batch.rs`: `two_icons_of_different_tints_share_one_batch`, `a_tinted_image_with_a_translucent_tint_blends`, `an_untinted_full_texture_image_still_batches_as_opaque`. In `render.rs`: existing `image_vertex_layout_matches_the_offsets_the_attributes_are_bound_with` and `the_image_shader_reads_every_attribute_the_vertex_buffer_supplies` gain the field and attribute **keeping their names and assertions**; new `the_image_shader_multiplies_the_sampled_texel_by_the_tint` (asserts the **whole assignment**) and `a_tinted_image_quad_carries_the_tint_it_was_given`.

10. **`doc/ui/DEMO_APPLICATION.md` § *Library gaps*, row `#4`, gains a dated note:** `TASK_UI_PRIM_44` added `Icon` and a tint on `DrawCommand::Image`; the row is **not closed**; its evidence sentence *"no texture tinting at draw time"* is **corrected in place** per § *Corrections to the first gap table*; *vector rendering* remains `TASK_UI_PRIM_39`'s, so **no glyph exists here yet**. The Blocks column keeps **"Visual quality — 'real icons' requirement"**, `L10` stands unamended, and **no note claims a rendered icon**.

## Acceptance Criteria

- [ ] **Requirements 1–9's deliverables exist as specified:** `Icon` is the sixteenth `pub mod` between `gauge` and `image`; `DrawCommand::Image` carries `tint`; `IMAGE_VERTEX_STRIDE == 56`, `IMAGE_TINT_OFFSET == 40`, `offset_of!(ImageVertex, tint) == 40`, one attribute-6 pair; `Painter::image` is unchanged; and `icon.rs` has requirement 8's surface and none of its forbidden items (`on_event` count **0** there, **8** crate-wide; no `selected`/`state`/`tick`/`AnimationClock`; `pub enum` in `widgets/` is **12**).
- [ ] **The tint is not in the key:** `two_icons_of_different_tints_share_one_batch` (one texture, tints `(255,0,0,255)` and `(0,255,0,255)`) gives one transparent batch of 2 commands; a `tint.a == 128` full-UV image is `BlendMode::Transparent`; the neutral tint stays `Opaque`.
- [ ] **The shader's last line is asserted whole** by `the_image_shader_multiplies_the_sampled_texel_by_the_tint`, and the sizing/crop tests hold: `ICON_SIZE == 24.0`, `size()` constant, the `ATLAS_SIZE / 64 == 32` bound rejects `256`, one `size` with no `set_size`/`size_hint`/`scale`, containment over 100×100, 24×24, **16×16**, **24×1**, **0×0**, centre `(38.0, 38.0)` and `(0.0, 0.0)`, `radius == 0.0`.
- [ ] **All named tests pass** — including `a_theme_switch_reaches_the_icon`'s four clauses — the suite is green against **1894** with per-binary counts pasted and the 1450/1451 discrepancy resolved, none deleted, renamed away or weakened, and `Cargo.toml`/`Cargo.lock` unchanged.
- [ ] **Kills are reported per `.ai/agents/developer.md` § *Phase 3*;** task-specific: deleting `* v_tint.rgb`, keying the tint, dropping the `tint.a` clause, moving `IMAGE_TINT_OFFSET` to 32, and returning an un-intersected or top-left-anchored destination each fail a named test.
- [ ] **Six pages `AE = 0` outside `y ≥ 680`**, `main.rs` diff empty, the `data` image traversing the changed vertex, stride, `location = 6` attribute and shader; no `gl.get_error()` on that frame; **no changed doc comment claims an icon is on screen.**
- [ ] **Row `#4` is amended, dated and narrowed, not closed**, Blocks column kept and `L10` unamended.
- [ ] **Nothing leaked:** only `widgets/mod.rs`, `paint.rs`, `render.rs`, `batch.rs`, `widgets/image.rs`, `icon.rs` and `DEMO_APPLICATION.md` change; no `bezier`/`cubic`/`quadratic`/`curve_to`/`glyph_outline`; `theme.rs` unchanged (`TOKEN_COUNT` **33**); no `render/matrix.rs`, `render/mesh.rs`, `render/meshio.rs`.

## Out of Scope

- **No ear clipping, no concave fill, no stencil pass, no fill rule — row `L10`, declined by the operator on 2026-10-02;** `Icon` records a `DrawCommand::Image` and nothing else.
- **No `TabBar` (gap `#7`), no `Button::selected`, no edit to `widgets/button.rs`.**
- **No SVG and no runtime rasterisation** (no XML, path parser, curve tessellation or image decoder); `TASK_UI_PRIM_39` bakes offline.
- **No icon tooling or inventory** — no manifest, script, PNG directory or glyph-name enum; `ui_core` takes a `TextureHandle` (`TASK_UI_PRIM_39`'s `icons.json` names them).
- **No demo page and no change to the six**; `main.rs` is not edited, `Page::ALL` stays `[Page; 6]`, `Page::DEFAULT` stays `Pads`.
- **No generated glyphs** (no severity, blink, level or mode) and **no per-node clipping** (gap `#5`; `DrawCommand` carries no scissor state).
- **No layout change** — `LayoutMode::Grid` lays out nothing and `Flex.wrap` is discarded; `Padding` and main-axis `spacing` are all the row has, per `L7`.
- **No theme token** — `theme.rs` unchanged, `TOKEN_COUNT` **33**, `ThemeToken::all` unchanged.
- **No font change and no icon font** — (C) is declined; no `FACE_COUNT`, weight, `GlyphKey` or `FontSet` change.
- **No new dependency and no `unsafe`** — `Cargo.toml`/`Cargo.lock` unchanged (`sdl3 0.20`, `glow 0.18`, `freetype-rs 0.38`); `grep -c unsafe ui/src/ui_core/src/widgets/icon.rs` is 0.
