# TASK_UI_PRIM_44: `ui_core::widgets::Icon` — a Tintable Image Path, and the Widget That Uses It

## Goal

Close **gap `#4`** of `DEMO_APPLICATION.md` § *Library gaps* — *"**No Icon
widget** — `Image` (task 16) displays textures but icons need vector rendering,
theme tinting, and uniform sizing"*, which the same row calls *"the largest
unsupported item in the design"* — by doing the two of those three things that
are **library** work and leaving the third where it already is.

- **Theme tinting** becomes a capability of the pipeline, once, for every
  textured quad in the crate: `DrawCommand::Image` carries a tint colour and
  `IMAGE_FRAGMENT_SHADER_SRC` multiplies it into the sampled texel.
- **Uniform sizing** becomes the rule a new widget enforces: `ICON_SIZE` is one
  constant, the widget never scales, and the reason is a measurable property of
  the sampler rather than a preference.
- **Vector rendering** stays where it is: offline, in `TASK_UI_PRIM_39`'s
  `resvg` bake. This task adds no SVG, no rasteriser and no curve code, and says
  so in its own *Out of Scope*.

The deliverable is `pub mod icon`, the shader and vertex change that makes a tint
possible, and one widget that consumes both.

## Context

### What exists at `HEAD` (`75a896c` plus the uncommitted diff), established and not re-derived

- **There is no `pub mod icon`.** `ui/src/ui_core/src/widgets/mod.rs` carries
  **fifteen** `pub mod` entries — `button`, `chart`, `container`, `dialog`,
  `gauge`, `image`, `keyboard`, `label`, `list`, `progress`, `scroll`, `slider`,
  `text_input`, `toast`, `toggle` — and nothing else.
- **`widgets/` holds twelve `pub enum`s, not thirteen.** `GaugeType`,
  `ChartType`, `TextAlign`, `WrapMode`, `Truncation`, `Orientation`, `Severity`,
  `Anchor`, `ImageFit`, `ButtonState`, `KeyAction` and `Page` — twelve, across
  eight modules, plus two private enums (`chart`'s `Join` and `keyboard`'s
  `CapAction`). **Stated because a brief for this task said thirteen, and
  `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* row `L6a`
  says twelve**, so the brief's number and the document's number agree with each
  other and not with the tree. The tree wins. `Page` is `keyboard`'s two-page
  toggle and collides by name with `ui_demo`'s `Page`; `Icon` **adds no enum at
  all**, so that collision risk is untouched by this task.
- **`DrawCommand` has nine variants** — `Rect`, `RoundedRect`, `Shadow`, `Text`,
  `Image`, `Line`, `Circle`, `Path`, `Polygon` — and `DrawCommand::Path` is a
  **stroked** polyline: `{ points, width, color, closed }`, expanded by
  `render.rs`'s `command_quads` into **one `line_quad` per segment**. `closed:
  true` joins the last point to the first, and nothing else: there is no cap and
  no join, so a stroked outline drawn this way has butt caps and gaps at every
  corner.
- **`DrawCommand::Polygon` is filled but convex-only** — a triangle fan from the
  first point, with `render.rs`'s own comment naming both rejected alternatives
  (*"The alternative — ear clipping, or a stencil pass — is a rasteriser with its
  own vertex type"*). Ear clipping was **declined by the operator on 2026-10-02**
  and is recorded in `chart.rs`'s module docs and in `IMPLEMENTATION_STATE.md`.
  **That is row `L10`, it is a separate gap, and this task does not re-open it,
  amend it, or route around it.**
- **There is no curve anywhere in `ui_core`.** `bezier`, `cubic`,
  `quadratic`, `curve_to` and `glyph_outline` return **zero** hits across the
  crate. `font.rs` rasterises glyph *coverage* through FreeType and packs the
  bitmaps; there is no outline extraction and no outline API.
- **`DrawCommand::Image` cannot be tinted.** Its fields are `rect`, `texture`,
  `uv`, `opacity: f32` and `radius: f32`, and the `opacity` field's doc says so
  in its own words: *"`0.5` is *the image, half as present* and not *the image,
  tinted*."* `IMAGE_FRAGMENT_SHADER_SRC` ends
  `frag_color = vec4(texel.rgb * opacity, texel.a * opacity);` — a scalar, never
  a colour, and no tint uniform. **Text IS tintable**, by contrast:
  `TEXT_FRAGMENT_SHADER_SRC` multiplies `v_color` (a per-vertex `a_color`) into
  the sampled coverage, and that is the shape this task copies.
- **Textures.** `GL_RGBA8`, premultiplied once at load by `Pixels::premultiply`
  (called from `decode` in `texture.rs`), `GL_LINEAR` for both min and mag,
  **`generate_mipmap` is never called**, and `GL_CLAMP_TO_EDGE`.
  `ATLAS_SIZE` is **2048** and `ATLAS_MAX_IMAGE` is **512**: an image at or under
  512 on both axes shares the atlas, anything larger gets a texture of its own.
  The atlas **evicts by shelf**, and `TextureCache::pin` exists because of it.
- **There is no DPI concept.** The viewport comes from the window size, logical
  points rather than `InPixels`; `dpi`, `hidpi`, `density` and `drawable_size`
  appear only in doc comments. **1 asset pixel = 1 window pixel.**
- **`Painter`'s entire composable vocabulary** is `new`, `rect`, `rounded_rect`,
  `text`, `text_in`, `text_bold`, `image`, `line`, `circle`, `path`, `polygon`,
  `shadow`, `extend`, `finish`. (`text_in` is the family-carrying form of
  `text`; the brief's list omitted it. It is not composable geometry either way.)
- **`Callback<T> = Option<Rc<dyn Fn(T)>>`** behind the `Callback` newtype — `Fn`,
  not `FnMut`.
- `ui/src/ui_core/Cargo.toml` carries **`publish = false`**, so a source-breaking
  change to a `pub` item is internal to this repository. That is what makes
  requirement 2's field addition a decision rather than a negotiation.
- `AGENTS.md`: **no new dependency without the operator**; the approved direct
  dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`
  (`bundled`); edition 2021, `rust-version = "1.85"`. Tests go beside the code;
  **no test that needs a display, a network, a filesystem or the wall clock**.
- `.ai/tools/fps-check.sh` takes `seconds` then `floor`, runs the binary with no
  arguments, and **cannot name a page**; per-page is
  `ROADOS_RUN_SECONDS=<n> … --tab=<page>`. Every run of `ui_demo` an agent
  launches is measured — `.ai/workflows/task-sequence.md` § Gates calls that a
  gate, and `.ai/agents/developer.md` § Phase 3 owns it.
- **The suite stands at 1894** (1450 `ui_core` + 224 `ui_demo` + 220 doctests).
  The `#[test]` attribute count observed in the tree is **1451** in `ui_core` and
  **224** in `ui_demo`, so the two figures differ by one in one binary and the
  handoff resolves it by pasting the real per-binary counts rather than by
  picking one.

### The decision: **(A), a tintable texture path**

**Decision: `DrawCommand::Image` gains a `tint: Color`, the image shader
multiplies it, and a new `Icon` widget consumes that.** Four reasons, and the
third is the one that settles it.

1. **The two rows are one sentence, and (A) is the half that is library work.**
   Row `#4` names three needs — *vector rendering, theme tinting, and uniform
   sizing* — and row `L10` is what makes the first of them impossible at
   runtime: `Polygon` is a convex fan, `Path` is a stroke with no cap and no
   join, and there is no curve primitive in the crate at all. So the three needs
   split cleanly: **tinting and sizing are this task, and vector rendering is
   `TASK_UI_PRIM_39`'s**, which rasterises Lucide's stroked, round-capped, curved
   outlines offline because *"'No SVG' is the gap this task fills from outside
   the library, and the only place it can be filled."* This task therefore adds
   no geometry code and does not touch `L10`.

2. **The tint belongs on the vertex, and that is forced by the batcher.**
   `BatchKey` is `{ texture, blend_mode, shader }`, and an icon atlas is **one
   texture**. Put the tint in the key and every differently-coloured icon becomes
   its own draw call — which defeats the entire reason the atlas exists, and
   turns a 33-glyph dock from one call into thirty-three. So the tint has to
   ride per-vertex, exactly as the text shader's `a_color` already does, and
   **the number of icons on screen stops being a number of draw calls.** This is
   testable without a display (`DrawCommand::batch_key` equality) and it is the
   difference between (A) and a per-icon texture.

3. **Text already proves the shader shape, so this is a copy rather than a
   design.** `TEXT_FRAGMENT_SHADER_SRC` samples a single coverage channel and
   multiplies a per-vertex colour into it. An icon texture is the same thing with
   RGBA instead of one channel, and the premultiplied arithmetic works out
   exactly: the texel is premultiplied, the tint is a `Color` and every `Color`
   in this crate is premultiplied (`Color::new`'s own doc says *"Creates a new
   color with premultiplied alpha"*), so `vec4(texel.rgb * tint.rgb,
   texel.a * tint.a)` is a correct premultiplied modulate — the output's alpha
   is `a·A` and its colour is the tint at that alpha, which is what premultiplied
   blending requires. **The identity is opaque white**, so every image already in
   the pipeline is unchanged, which is what makes the six-page capture criterion
   in *Acceptance Criteria* achievable at all.

4. **(A) is not icon-specific and that is most of its value.** The change lands
   on the one path every textured quad in the crate uses — `Image`'s paint and
   the `data` page's `demo.png` — and each of them gets tinting for the cost of
   a field and a multiply. A `TabBar` that wants a selected dock item in the
   theme's `Primary`, a disabled item dimmed, a hover item lifted: all three are
   a colour the caller already owns.

### What (A) costs, stated in full

- **It is a source-breaking change to `DrawCommand::Image`, and to nothing else.**
  The variant gains a field, so every construction and every *exhaustive*
  destructuring of it must be updated. In this repository that is
  `ui/src/ui_core/src/paint.rs` (the recorder and its own tests),
  `ui/src/ui_core/src/batch.rs` (the key and its fixtures) and
  `ui/src/ui_core/src/widgets/image.rs` (one **doc-test** that destructures all
  five fields with no `..`). Because `ui_core` is `publish = false` this breaks
  no downstream user; `reviewer.md` asks for it to be discussed rather than
  silent, and this section is the discussion.
- **The other eight `DrawCommand` variants cost nothing and change nothing.**
  `Rect`, `RoundedRect`, `Shadow`, `Text`, `Line`, `Circle`, `Path` and
  `Polygon` are untouched; `BatchKey::is_singleton` still names only
  `ShaderKind::Shadow`; the solid, text, shadow-mask and shadow-composite shaders
  are untouched.
- **`Painter::image` keeps its exact signature** and records opaque white;
  `Painter::tinted_image` is the six-argument form. **`Image::paint` is the
  crate's only production caller of `Painter::image`, and it is unaffected** —
  `list.rs`'s one `painter.image(` is inside its `#[cfg(test)]` module and is a
  fixture rather than a call site. The reason for keeping the signature is
  compatibility, and it is stated as such rather than dressed up as a principle:
  a colourless image is the overwhelmingly common case (every photograph, and the
  demo's one image), and a trailing `Color` after two `f32`s is a positional
  argument a caller can transpose. One implementation, two names: `image` is a
  one-line delegation to `tinted_image` with the neutral tint, so there is no
  second rule to keep honest.
- **`ImageVertex` grows from ten `f32` to fourteen**, so `IMAGE_VERTEX_STRIDE`
  goes **40 → 56** and a new `IMAGE_TINT_OFFSET` sits at 40, bound as
  `location = 6`, `size 4`. **Every image quad costs 64 more bytes** of vertex
  data: the `data` page records **one** `DrawCommand::Image` today, so the frame
  pays 64 bytes, and the fps criterion is stated to expect no measurable change
  rather than to hope for one.
- **`BatchKey` gains one clause and no field.** The image arm's opaque test
  becomes `uv.is_full() && *opacity == 1.0 && tint.a == u8::MAX`, because a
  translucent tint makes the quad translucent even at full opacity and full UV,
  and submitting it with blending off would **drop whatever is behind it** — a
  wrong picture rather than a slow one, which is the trade `batch.rs`'s own
  comment says it is making deliberately. The tint itself is **not** in the key,
  for the batching reason in decision point 2.
- **`Image` itself gains nothing.** It keeps its three public properties
  (`opacity`, `shown_opacity`, `corner_radius`), its four `ImageFit` variants,
  its own `AnimationClock` and its `opacity`/`shown_opacity` split. It records
  the neutral tint and is otherwise unchanged, **so the six gallery pages draw
  exactly what they drew before** — which is the criterion, and it is load-bearing
  rather than vacuous because the `data` page's image goes through the changed
  vertex layout, the changed attribute set and the changed fragment shader.
- **`theme.rs` gains nothing.** No `ThemeToken`, no `ThemeToken::all` entry, and
  `TOKEN_COUNT` stays at 33. An icon is ink and `ThemeToken::Text` is the ink
  colour; a new token is `L9`'s business and `L9` is not this task.
- **`ui/src/ui_demo/src/main.rs` is not touched at all.** See § *The demo does
  not change*.

### Why not (B), a procedural glyph path

**Declined, on `L10`'s own evidence and on a structural argument.**

- A Lucide glyph is a **stroked outline with round caps and round joins and
  bezier curves**. `DrawCommand::Path` **is** a stroke, which is the one part
  that fits — and it is not enough: `command_quads` expands it to **one
  `line_quad` per segment**, so there is no cap and no join, and a 2 px
  round-capped corner comes out butt-capped with a notch at the vertex. There is
  no curve primitive, so every curve in every glyph would have to be
  hand-tessellated into straight segments, at which point the glyph is a
  hand-authored polygon table in a library source file.
- Fills are worse than strokes here: `DrawCommand::Polygon` is a convex fan, so
  any concave silhouette — a chevron, a lightning bolt, a padlock shackle, a
  `W`, an `M` — renders as overlapping and inverted triangles, which is **a
  wrong picture rather than a rough one** by `DrawCommand::Polygon`'s own doc.
- **And the structural argument is the one that settles it.** (B) needs a closed
  `pub enum` of glyph names *inside the library*. `DEMO_APPLICATION.md`
  § *Asset requirements* says the inventory is *"much larger than the list
  above"* — every dock item, every conditional top-bar item, every Controls tab
  row and ~20 indicator lights — and it says explicitly that some of those
  *"are drawn from state, not selected from an atlas"*. A library enum is a list
  the library cannot extend without a breaking change, and the demo's real list
  is a directory of files on disk that `TASK_UI_PRIM_39`'s `icons.json` owns. **A
  texture handle is an open set; an enum is a closed one.**

### Why not (C), a font-glyph-backed icon set

**Declined, and here is what it would actually cost — each item read off the
source rather than assumed.**

- **The two-weight limit does *not* bite, and it is worth saying so rather than
  reaching for it as a reason.** `font.rs` declares `FACE_COUNT: usize = 2`,
  `FontWeight` has exactly the two variants `Regular` and `Bold`, and
  `resolve_slot` falls back to the regular slot when a family has no face for the
  weight asked for. An icon font as **its own family** with one face therefore
  works: every icon is drawn at `Regular`, and a request for `Bold` — which
  nothing makes — falls back to that same face. **A third weight cannot be asked
  for at all, because `FontWeight` is a closed two-variant enum**, so this
  limitation is not a constraint an icon set runs into.
- **The atlas is not the problem either.** `GlyphAtlas` is a separate structure
  from the image atlas, grows by doubling to the ceiling the renderer reads from
  `GL_MAX_TEXTURE_SIZE`, and 33 glyphs at 24 px is nothing beside the text.
  `GlyphKey::Glyph { ch, size, font }` would address an icon as a Private Use
  Area codepoint at one integer size, and the "per-glyph rasterisation at exactly
  the drawn size" property that `font.rs`'s module docs make a virtue of is
  genuinely the *right* answer to the no-mipmap rule.
- **What does bite is eviction, and it is disqualifying.** The image cache has
  `TextureCache::pin`, and its own doc says why: *"a widget that must be showing
  an image — a photograph, a logo — pins it, and the cache stops evicting shelves
  that would drop a pinned image."* **The glyph atlas has no `pin` at all.** It
  evicts by shelf, and at its ceiling it *refuses* a glyph and reports it through
  `GlyphAtlas::dropped`. So under (C) a dock icon — the most "must be showing"
  thing on the screen — is **unpinnable**, and it competes for shelf space with
  every glyph of body text. A dropped body glyph reflows a line; a dropped dock
  icon is a hole in the chrome with a count attached to it.
- **And the asset is a fresh licence decision.** (C) needs a new committed font
  file, which under `AGENTS.md` is the same class of decision as a new
  dependency, and it duplicates in a second format what `TASK_UI_PRIM_39`
  already bakes offline from a pinned, hash-verified ISC source.
- **The resolution independence it buys is worth nothing in this engine.** There
  is no DPI concept and one on-screen size, `ICON_SIZE`; `GL_LINEAR` with no
  mipmaps means an icon drawn at anything other than the size it was rasterised
  at is *worse*, not better. **(C) is (A) with a font loader and a licence
  attached, and it buys a scaling capability the engine cannot use.**

### `Icon`: what it is, and what it is deliberately not

**`Icon` is `Image` minus three decisions, plus one colour.** It is:

- a **`TextureHandle`** and an **`ImageSource`**, exactly as `Image` holds them —
  so the caller resolves a glyph the way it resolves any image, through
  `TextureCache::load`, and gets its uv and its pixel size from
  `ImageSource::of`;
- a **`tint`**, which is the one thing `DrawCommand::Image` could not carry;
- **a fixed size it does not scale from**, which is the one thing `Image`'s four
  fits are for and which an icon must not have.

It is **not** a control. **No `on_event`, no `state()`, no `hit`-test
participation, no `on_change`, and no `selected`.** Each of those is declined for
a stated reason rather than left out:

- **`state()` is `Button::state()`'s instrument, and its doc says what it is
  for**: the primary state *"in the order one overrides another: disabled, then
  pressed, then hovered, then focused, then at rest"*. A state enum answers
  *"which of these mutually exclusive appearances is current"*, and an icon has
  exactly one appearance per `(texture, tint, opacity)` — every bit that would
  make it more than one belongs to the `Button` drawn around it. So `Icon` has no
  `state()`, and the crate's count of `pub fn on_event` impls stays at **eight**.
- **`selected` is gap `#7`'s field, to be defined once.** `DEMO_APPLICATION.md`
  § *Library gaps* row `#7` records that `Button` carries `hovered`, `pressed`,
  `disabled`, `focused` and `activatable` and **no `selected`**, and that the
  selected tab is a `background`/`foreground` swap the demo owns. A dock's
  selected icon follows from the same decision, so a `selected` on `Icon` would
  be the same concept in a second place — the failure `Button::activatable`'s own
  doc records when it says *"One bit cannot answer both"*.
- **`style()` is `Image::style()`'s shape, one field wider**: `{ tint, opacity }`,
  where `opacity` is clamped to `0.0..=1.0` and a `NaN` is `0.0`, because
  `shown_opacity`'s counterpart is public and a caller may write it directly.
  `tint` is drawn as it holds, because a `Color` cannot be out of range — its
  channels are four `u8`s.

**`Icon` has no clock.** No `AnimationClock`, no `snap_to_state`, no
`animate_to_state`, no `tick`, no `is_animating`, and no `shown_*` property. That
is the sharpest difference between it and `Image`, and it is the right one:
`Image` splits `opacity` from `shown_opacity` so a **caller-aimed** transition has
something to carry, and an icon's only appearance is its tint — **and the one
thing that moves an icon's tint is the theme, which animates its own tokens.**
Adding a second clock to `Icon` would be two animations of one colour, and a
`tick` over an empty clock is a method that returns `false` forever.

**How a caller names the glyph: it does not, inside `ui_core`.** There is no
`IconName` and no string. The name lives where the inventory is — `TASK_UI_PRIM_39`
§ *Asset requirements*' `icons.json`, which `bake_icons.py` reads and which
requirement 13 of that task makes a **closed** list — and the caller turns it into
a `TextureHandle` through its own loader, exactly as it does for every image
today. **`Icon::new` takes the handle, in `Image::new`'s exact signature**, and
that is the whole of the naming story. Putting a glyph name in the library would
put the demo's inventory in the library, and the direction document's own list is
open at both ends.

### Uniform sizing: one number, authored at the size it is drawn

**The rule: `ICON_SIZE = 24.0`, one constant, no setter, no property, and the
asset is 24×24 because 24 is what it is drawn at.** Gap `#4` names *"uniform
sizing"* as one of its three needs and this is the whole of it.

**Why a 256-pixel master would alias, in numbers.** `MIN_FILTER` is `GL_LINEAR`
and `generate_mipmap` is never called, so **there is no level to average over**:
a minifying `GL_LINEAR` sample reads **four** texels regardless of how small the
destination pixel is. At 256 → 24 the reduction is **10.67 : 1**, so each
destination pixel's footprint is **113.8 source texels** and four of them are
read — **3.5 % of the footprint**. The consequence is not softness, it is
absence: Lucide's `stroke-width="2"` on a 24 viewBox is **2.0 screen pixels** at
24 and **0.1875 screen pixels** at 24 when authored at 256, which is *below one
destination pixel*, so a stroke either misses entirely or lands in one pixel and
not its neighbour, and it moves between the two as the icon shifts by a pixel.
This is the same failure `font.rs`'s module docs record for a signed distance
field — *"a good way to draw text at a size other than the one it was rasterized
for"* — arrived at from the other direction, and
a distance field has to binarize, and the bit it drops is
the edge.

**And the atlas would not hold them.** `ATLAS_SIZE` is **2048**, so the atlas is
**4 194 304** pixels. Thirty-three icons at 24×24 are **19 008** pixels —
**0.45 %** of it. The same thirty-three at 256×256 are **2 162 688** pixels —
**51.6 %** of it on their own, and a **second** theme's set beside them would need
**4 325 376**, which does not fit and starts shelf eviction. That is the second
reason, and it is arithmetic rather than taste. **And note what § *Theme
tinting* removes**: once one white set is enough, the doubling is gone — which is
the atlas argument and the licensing argument arriving at the same number from
two directions.

**What happens when the cell is smaller than the icon: the icon is cropped, not
scaled and not shrunk.** `Icon::destination(rect)` is the intersection of the
centred `ICON_SIZE` square with `rect`, and `paint` records nothing when that
intersection is empty on either axis. The alternatives were weighed:

- **Scaling down** is the thing the sampler cannot do, and it is forbidden above.
- **Letting it overflow** makes a 16-px dock cell paint 24 px over its
  neighbour, and **nothing clips it**: `DrawCommand` has no scissor state, and
  `frame_clips` in `ui_demo` is the demo's, per gap `#5`. An overlap is invisible
  in the source and corrupts something else on screen.
- **Cropping** bounds the damage to the icon's own cell and is visible. It is
  also the invariant every other widget in the crate already keeps: `Image::paint`
  records nothing for a collapsed rect (*"a collapsed node in a flex layout costs
  nothing rather than a degenerate triangle"*), and `DrawCommand::Image`'s radius
  is a clip rather than an overflow. **`every_icon_records_a_rect_inside_the_rect
  _it_was_given` is the test that holds the invariant**, and `Icon::size()` is the
  number a caller lays the cell out from.

**Centred, not top-left anchored, and `Image`'s rule does not transfer.**
`ImageFit::None` anchors at the top left because `Scroll`'s content starts at its
top edge — its module doc argues that at length, and the argument is correct *for
a scroll*. An icon is never the first thing in a scrolling column; it is a mark
inside a cell that is always larger than it, and a ragged left edge on thirty-three
dock glyphs is a defect a reader would photograph. So `Icon::destination` centres,
and its doc says so **with `Image`'s reason named as the reason it does not
apply** — the `gesture_delta` discipline, applied to an anchoring rule.

### Theme tinting: it follows the theme, and the theme animates it

- **`Icon::tint` is a `pub Property<Color>`**, and **the caller assigns it**, in
  the demo's existing idiom: `Label::color = cycling_color(…)` and
  `Container::background = themed_color(…)` are both whole-property assignments,
  and `ui_core/src/theme.rs`'s `Theme::property` doc is what makes them possible
  (*"a widget holding the theme's background property is bound to the theme's
  background, and follows it through every change and every transition"*).
- **`Theme::switch_to` therefore animates the icon with no code in `Icon` at
  all.** It animates every token property, the bound property recomputes on every
  frame of that transition, and the tint follows. **This is the mechanism behind
  the acceptance criterion *"a named test proving a theme switch reaches the
  icon"*, and it is worth stating that it is the mechanism the crate already uses
  for the pads and the text panel** — `Icon` is the first *widget* to get it for
  free, and the reason it can is that it has no clock of its own to fight with.
- **The default is opaque white and the fallback is `ThemeToken::Text`.** White is
  the identity modulate, so an icon nobody has themed is the icon the texture
  already is; `Text` is the ink colour, and
  `DEMO_APPLICATION.md` § *Could not verify* records that *"Tesla publishes no
  design tokens at all"*, so this is a first-principles choice like every other
  colour in the demo. `Icon::reset_tint()` is the one call that gets the default
  back, and its doc says it is for a caller that overwrote the field rather than
  for a themed icon.
- **A muted or accent-coloured icon is a caller binding a different token.** No
  second field, no enum, no token.

### Interaction with `Button`, and what a future `TabBar` needs

**`Icon` needs nothing from a future `TabBar`, and constrains it not at all.** The
direction document's dock is icon-plus-label, so here is the whole of the
contract in both directions, stated now so the task that builds `TabBar` does not
have to guess:

- **What `TabBar` will use.** `Button` for the hit target and the label,
  `Icon` beside it, `Label` under or after it. `Icon` supplies a writable
  `tint: Property<Color>` and a writable `opacity: Property<f32>`, so `TabBar`
  writes the selected icon's colour from **the same decision** that writes the
  button's `background` and `foreground` — which is what the demo's
  `tab_palette` already does for the button half.
- **What it must not want from `Icon`.** It must not want a `selected` flag, an
  `on_event`, a hit rect of its own, or a size knob. All four are declined above,
  each for a stated reason, and the first is gap `#7`'s to define.
- **And `L7` is the constraint that bites the row, not `Icon`.** Row `L7` in
  `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* reads
  *"**No margin, no `flex-shrink`, no `flex-basis`, no cross-axis gap**"* while
  recording that `Padding` *does* exist. An icon-plus-label row therefore has a
  **main-axis** `spacing` and a four-sided `Padding`, and **nothing else**: there
  is no margin and no cross-axis gap to put between a glyph and its text.
  **`Icon` cannot fix that and does not try** — it is a separate gap with a
  separate task, and this one says so rather than inventing a spacing property.
- **`Button` is untouched.** No field added, no setter, no doc edit beyond
  pointing at `Icon`. Gap `#7` owns whatever `Button` needs.

### The demo does not change, and why that is stated rather than assumed

**`ui/src/ui_demo/src/main.rs` is not edited by this task, and the six gallery
pages draw exactly what they drew before.** Three facts, and the third is the one
that makes the criterion worth anything:

1. **`TASK_UI_PRIM_39`'s icon bytes are not in the tree.** `ui/src/ui_demo/
   assets/` holds `demo.png` and nothing else; there is no `icons/` directory and
   no `tools/` directory, because task 39 has not run. **So the demo has no icon
   to draw, and putting one on a page would mean inventing a placeholder** —
   which operator decision 3 of 2026-09-30 forbids in as many words: *"Real
   icons. … No placeholder geometric shapes."*
2. **`Painter::image` keeps its signature and records the identity tint**, so the
   `data` page's `demo.png` is multiplied by opaque white and lands on the same
   pixels it landed on before.
3. **And that makes the capture criterion load-bearing rather than vacuous.** The
   `data` page records `DrawCommand::Image`, so it goes through the **changed**
   `ImageVertex`, the **changed** `IMAGE_VERTEX_STRIDE`, the **new** attribute
   pointer at `location = 6`, and the **changed** `IMAGE_FRAGMENT_SHADER_SRC`. A
   wrong `IMAGE_TINT_OFFSET`, a missed `vertex_attrib_pointer_f32`, an
   `IMAGE_VERTEX_STRIDE` that does not match `size_of::<ImageVertex>()`, or a
   shader that dropped the multiply **all change or blank that image**, and the
   capture sees it. **This task's only on-screen evidence is a regression test
   for the pipeline change, and the honest limit is that no icon is seen at all.**

### Testing: what a widget that only draws pixels can and cannot be asked for

`AGENTS.md` forbids a test that needs a display, a network, a filesystem or the
wall clock, and the honest consequence is that **almost nothing about this
widget's appearance is testable, and the things that are testable are about its
recorded commands.** Stated before the tests rather than after, so the test list
is not read as coverage of the tint.

**Testable without a display, and tested:**

- the recorded command stream: one `DrawCommand::Image`, at the centred
  `ICON_SIZE` square intersected with `rect`, sampling the `uv` it was given, at
  the clamped opacity, with `radius: 0.0`, carrying the tint it was given;
- **`a_theme_switch_reaches_the_icon`** — the load-bearing one, and it is a real
  end-to-end property: bind a `Theme::dark()` token, build an `Icon`, assign the
  bound property, read the recorded tint, `Theme::switch_to(Theme::light(), 300)`,
  tick, and assert the recorded tint has moved and is *between* the two themes'
  ink colours — and again at the end of the transition, where it is exactly the
  light theme's;
- `BatchKey` behaviour: two icons with different tints share one batch, an
  untinted full-texture image at opacity 1 is still `Opaque`, and a translucent
  tint is not;
- the shader **as a string**: the fragment source declares `v_tint` and multiplies
  by it, and the vertex source declares `a_tint` at `location = 6`. This is the
  crate's established mechanism for shaders — `render.rs` already asserts on its
  sources this way and those assertions have teeth;
- the vertex layout: `size_of::<ImageVertex>() == IMAGE_VERTEX_STRIDE` and each
  field at its `IMAGE_*_OFFSET`, through `offset_of!`, which is the existing
  `image_vertex_layout_matches_the_offsets_the_attributes_are_bound_with`;
- the arithmetic: `ICON_SIZE` is finite, positive, `== 24.0`, square, and at or
  under `ATLAS_MAX_IMAGE`; the destination is the intersection and is inside
  `rect`; a collapsed rect records nothing.

**Not testable without a display, and not claimed:**

- **that the multiply reaches the framebuffer.** No unit test distinguishes
  `texel.rgb * tint.rgb` from `texel.rgb`; they differ only in what a fragment
  writes. The capture in *Acceptance Criteria* is evidence that the image *pass*
  still works, and it is **not** evidence that a tint lands in the right colour,
  because no tinted quad is ever drawn by this task.
- **that an icon looks like an icon.** Nothing in this task draws one.
- **anything about aliasing.** The 10.67 : 1 arithmetic in § *Uniform sizing* is
  an argument from `GL_LINEAR` and the absence of `generate_mipmap`, both read
  off the source. It is not a measurement, and this file does not claim it is one.

**The mutations that must be killed, named before the tests so a reviewer knows
which to break first:**

| mutation | killed by |
|---|---|
| drop `* v_tint.rgb` from the fragment shader's `frag_color` | `the_image_shader_multiplies_the_sampled_texel_by_the_tint` |
| put `tint` into `BatchKey` | `two_icons_of_different_tints_share_one_batch` |
| drop `&& tint.a == u8::MAX` from the image key's opaque test | `a_tinted_image_with_a_translucent_tint_blends` |
| `Painter::image` records `Color::default()` instead of opaque white | `painter_image_records_the_neutral_tint` |
| `IMAGE_TINT_OFFSET` moved to 32 | `image_vertex_layout_matches_the_offsets_the_attributes_are_bound_with` |
| `Icon::destination` anchors top-left instead of centring | `an_icon_is_centred_in_its_rect` |
| `Icon::destination` returns the square un-intersected | `every_icon_records_a_rect_inside_the_rect_it_was_given` |
| `Icon::paint` records `radius: 6.0` | `an_icon_paints_square_corners` |

**Every one of the eight must be run and its kill reported**, per
`.ai/agents/developer.md` § Phase 3 (*"A test that has never failed is not a
test"*) and *"Break it deliberately, watch it fail for the right reason, then fix
it back."* A mutation the runner refused to apply is **not a kill**, and
a mutation the runner refused to apply being recorded as a kill is that failure
in this repository's own memory.

### Scope, measured against `developer.md` § *Scope check*

**Eight files, three components, and the file count is over the threshold — so
this task splits, per `.ai/protocols/subagents.md` § *Implementation fan-out*.**

| sub-task | files it owns | acceptance test that passes alone |
|---|---|---|
| **A — the tint pipeline** | `ui/src/ui_core/src/paint.rs`, `ui/src/ui_core/src/render.rs`, `ui/src/ui_core/src/batch.rs`, `ui/src/ui_core/src/widgets/image.rs` | `painter_image_records_the_neutral_tint`, `two_icons_of_different_tints_share_one_batch`, `image_vertex_layout_matches_the_offsets_the_attributes_are_bound_with`, `the_image_shader_multiplies_the_sampled_texel_by_the_tint`, and `cargo test` green with the demo's `demo.png` still drawn |
| **B — the `Icon` widget** | `ui/src/ui_core/src/widgets/icon.rs` (new), `ui/src/ui_core/src/widgets/mod.rs` | `an_icon_paints_exactly_one_tinted_image_command`, `a_theme_switch_reaches_the_icon`, `every_icon_records_a_rect_inside_the_rect_it_was_given`, and every other test in requirement 9 |
| **C — the record** | `doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md` | row `#4` carries its dated note and the state file's entry exists |

**B depends on A** — it needs `Painter::tinted_image` and
`DrawCommand::Image`'s `tint` — so under
`subagents.md` § *Implementation fan-out*'s **"no hidden dependencies"** clause
this is a **sequential** split, **A then B**, and **C may run in parallel with
either** because it edits two documents no other sub-task owns. **B is briefed
against the tree as it stands after A lands**; a subagent told to use a type that
does not exist yet invents one. The developer is the orchestrator, integrates the
three handoffs, and runs the whole verification suite **once, at the end** — a
suite run per sub-task measures three trees and none of them is the change.

**If an implementer finds themselves editing a fifth *code* file, that is a stop
condition rather than an expansion** (`developer.md` § *Stop conditions*).

## Requirements

1. **`pub mod icon;` in `ui/src/ui_core/src/widgets/mod.rs`**, **alphabetically**
   between `gauge` and `image`, so the list carries **sixteen** entries:
   `button`, `chart`, `container`, `dialog`, `gauge`, **`icon`**, `image`,
   `keyboard`, `label`, `list`, `progress`, `scroll`, `slider`, `text_input`,
   `toast`, `toggle`. **Nothing else in that file changes** — `Callback<T>` is
   untouched.

2. **`DrawCommand::Image` gains a `tint: Color`, as its last field**, in
   `ui/src/ui_core/src/paint.rs`, with a doc comment that says, in this file's
   § *The demo does not change* arithmetic and not in the negative:

   - **`tint` is premultiplied, and the multiply is a modulate, not a
     replacement.** `vec4(texel.rgb * tint.rgb, texel.a * tint.a)` against a
     premultiplied texel and a premultiplied `Color` is exact: for a **white**
     glyph at coverage *a* and a tint *c* at alpha *A*, the output is
     `a·c·A` per channel with alpha `a·A`, which unpremultiplies to *c* at
     `a·A` — the correct premultiplied result. **A texture that is not white
     does not become the tint; it becomes the tint times its own colour, and
     that is the contract.** The doc says this in one sentence because a caller
     who bakes a coloured glyph and wonders why it is muddy needs the answer
     before they rebake it.
   - **`tint` is NOT in `BatchKey`.** The field's doc names the reason — a screen
     of thirty-three differently-coloured glyphs sharing one atlas texture is
     **one draw call**, and a key that carried the tint would make it
     thirty-three — and cross-references `Batch::clip`, which is the crate's
     existing precedent for something a batch carries without being keyed on.
   - **`opacity` keeps its own doc, and its meaning does not change.** It is
     still *"the image, half as present"* and not a colour; the two fields are
     independent, and `opacity` is still clamped `0.0..=1.0` at draw time.
   - **The neutral tint is opaque white, and it is one named public function.**
     `paint.rs` gains **`pub fn untinted() -> Color`, `#[must_use]`**, returning
     `Color::new(255, 255, 255, 255)`, with a doc comment giving the identity
     argument and a doc-test. It is **`pub` and not private** because
     `icon.rs`'s `Icon::new` and `Icon::reset_tint` both need it, and a private
     helper in one module with a copy in another is the second rule this
     repository keeps not having. It is `pub` and not a `pub const` because
     `Color::new` is not `const`. **The name is load-bearing: "untinted" is the
     claim, and a reviewer can grep it and find every place that means
     "unchanged".**

3. **`Painter::image` keeps its exact five-argument signature** and delegates:

   ```rust
   pub fn image(&mut self, rect: Rect, texture: TextureId, uv: UvRect, opacity: f32, radius: f32) {
       self.tinted_image(rect, texture, uv, opacity, radius, untinted());
   }

   pub fn tinted_image(
       &mut self,
       rect: Rect,
       texture: TextureId,
       uv: UvRect,
       opacity: f32,
       radius: f32,
       tint: Color,
   ) {
       self.commands.push(DrawCommand::Image {
           rect, texture, uv, opacity, radius, tint,
       });
   }
   ```

   **`Image::paint` is the crate's only production caller of `Painter::image`,
   and it is untouched** — `list.rs`'s one `painter.image(` is a fixture inside
   its `#[cfg(test)]` module, not a call site. The doc comment on `image` gains
   one sentence saying that the neutral tint is the identity and that
   `tinted_image` is the form that colours, and the doc on `tinted_image` gives
   the premultiply arithmetic.

4. **`render.rs`: the vertex gains the tint, and the offsets follow.**

   ```rust
   /// `pos` 0, `local` 8, `uv` 16, `radius` 24, `opacity` 28, `size` 32, `tint` 40.
   const IMAGE_VERTEX_STRIDE: i32 = 56;
   const IMAGE_TINT_OFFSET: i32 = 40;
   ```

   `ImageVertex` gains `tint: [f32; 4]` as its **last** field (the field order
   is the shader's attribute order and the offsets are derived from it, which
   `ImageVertex`'s own doc says). `image_quad` takes a `tint: Color` after
   `radius` and writes `quad_color(tint)` into all four corners; `draw_image_batch`
   destructures `tint` out of the command and passes it. A new
   `gl.enable_vertex_attrib_array(6)` +
   `gl.vertex_attrib_pointer_f32(6, 4, GL_FLOAT, false, IMAGE_VERTEX_STRIDE,
   IMAGE_TINT_OFFSET)` is added beside the other five, in the same block, and
   **the SAFETY comment above the vertex upload is extended to name the new
   field** — the existing one says the slice is a valid
   `IMAGE_VERTEX_STRIDE`-strided array because the struct is `repr(C)` with no
   padding, and a stride that no longer matches the struct is exactly the defect
   *a buffer sized for one vertex per quad* is about.

5. **`render.rs`: the two shader sources, exactly.**

   `IMAGE_VERTEX_SHADER_SRC` gains, in the attribute order:

   ```glsl
   layout(location = 6) in vec4 a_tint;
   // …
   out vec4 v_tint;
   // …
   v_tint = a_tint;
   ```

   and `IMAGE_FRAGMENT_SHADER_SRC`'s body becomes, with its last line the only
   line that changes:

   ```glsl
   in vec4 v_tint;
   // …
   vec4 texel = texture(u_image, v_uv);
   frag_color = vec4(texel.rgb * v_tint.rgb * opacity, texel.a * v_tint.a * opacity);
   ```

   **Four decisions in that one line, each with its reason, and all four go in
   the shader's own doc comment**, which already carries this shader's contracts
   and is where the next reader looks:

   - **The tint is not clamped.** No colour in this pipeline is clamped —
     `quad_color` divides by 255 and stops — and clamping one here would be a
     second rule. The opacity clamp stays, because `DrawCommand::Image` promises
     it *twice*, in the CPU and in the shader, and that promise is load-bearing.
   - **The tint is not `flat`.** It is constant across a quad, so interpolation
     is exact, and **`TEXT_FRAGMENT_SHADER_SRC`'s `v_color` is not `flat`
     either** — matching it is worth more than the qualifier is worth.
   - **The multiply is component-wise on a premultiplied pair**, which is the
     arithmetic in requirement 2, written out where the shader is.
   - **With an opaque-white tint the line is the old line**, which is the
     six-page capture criterion stated as an equation.

6. **`batch.rs`: one clause, no field.** `DrawCommand::batch_key`'s image arm
   becomes `if uv.is_full() && *opacity == 1.0 && tint.a == u8::MAX`, with the
   comment extended in place: a translucent tint makes the quad translucent even
   at full opacity and full UV, and *"a command that **might** show what is
   behind it must not be submitted with blending off"* — the argument the arm
   already makes, now applied to a fifth input. `tint` is **not** destructured
   into the key.

7. **`widgets/image.rs` changes in exactly one place, and it is a doc-test.**
   `Image::paint` calls `Painter::image`, which requirement 3 has already made
   supply the neutral tint, so **`Image::paint` does not change at all** — and
   neither does `Image`'s struct, its three public properties, its four
   `ImageFit` variants, its `AnimationClock` or its module doc. The **only** edit
   is the doc-test on `Image::paint`, which destructures all five fields of
   `DrawCommand::Image` with no `..` and therefore stops compiling once a sixth
   exists; it gains a `..` or names `tint` and keeps every assertion it already
   makes. **`git diff --stat ui/src/ui_core/src/widgets/image.rs` showing one
   changed hunk is the check, and the hunk is the criterion.**

8. **`ui/src/ui_core/src/widgets/icon.rs`, new.** Module doc first, in the form
   `image.rs`'s and `font.rs`'s carry — the two module docs in this crate that
   argue a mechanism rather than describe one — recording: what an icon is
   (**`Image` minus a fit, a radius and a clock, plus a colour**); **what it is
   not** (no `on_event`, no `state()`, no `selected`, no `fit`, no `radius`, no
   `AnimationClock`, no `tick`, no `animate_to_state`) with the four reasons from
   § *`Icon`: what it is, and what it is deliberately not*; **why the tint is on
   the vertex and not in the batch key**; **why a 256-pixel master would
   alias**, with the 10.67 : 1 and four-texel arithmetic; **why a cropped icon
   beats a scaled one and beats an overflow**, and that `L7` is why the
   icon-plus-label gap is a `spacing` and not a margin; **that the glyph is named
   by the caller's asset manifest and not by anything in this crate**; and **that
   no icon is drawn by anything in this repository yet**, because
   `TASK_UI_PRIM_39`'s bytes are not in the tree.

   ```rust
   /// The width and height every icon is drawn at, in pixels.
   pub const ICON_SIZE: f32 = 24.0;

   /// A single glyph: a texture, a tint, and a size it does not scale from.
   pub struct Icon {
       /// The colour the glyph is drawn in, premultiplied.
       pub tint: Property<Color>,
       /// How much of the glyph reaches the screen, `0.0..=1.0`.
       pub opacity: Property<f32>,
       texture: TextureHandle,
       source: ImageSource,
       node: Handle,
   }
   ```

   **No `#[derive]` on `Icon`**, in `Image`'s exact shape — and the reason is
   `Property<T>`, which is `#[derive(Clone)]` alone and has no `Debug`, so a
   `Debug` on any widget holding one would not compile. `Clone` is left off for
   the same reason `Image` leaves it off: nothing in this crate clones a widget,
   and a derive with no caller is an obligation nobody meets.

   - `pub fn new(nodes: &mut Arena<WidgetNode>, texture: TextureHandle, source: ImageSource) -> Self`
     — **in `Image::new`'s exact signature**, with `tint` at
     `paint::untinted()`, `opacity` at `1.0`, and a doc example asserting the
     node is in the arena and the drawn rect is the centred square.
   - `pub fn handle(&self) -> Handle`, `#[must_use]`.
   - `pub fn set_tint(&mut self, tint: Property<Color>)` and
     **`pub fn reset_tint(&mut self)`** — the second restores
     `paint::untinted()` and exists so a caller that overwrote the field can get
     it back without reconstructing the widget. Both doc comments name
     `Label::color = cycling_color(…)` and `Theme::property` as the idiom being
     served, and `reset_tint`'s says plainly that it is **not** what a themed
     icon wants.
   - `pub fn texture(&self) -> TextureHandle` and
     `pub fn set_texture(&mut self, texture: TextureHandle)` — `Image`'s pair,
     `Image::set_texture`'s doc reason (*"a plain field behind a setter … the
     mapping rather than the appearance"*) carried across.
   - `pub fn source(&self) -> ImageSource` and
     `pub fn set_source(&mut self, source: ImageSource)`, and
     `pub fn source_size(&self) -> (u32, u32)`.
   - **`pub fn size(&self) -> Size`**, `#[must_use]`, returning
     `Size::new(ICON_SIZE, ICON_SIZE)` **every time** — `Image::size`'s exact
     shape and its exact argument (*"the one size at which the image is not
     scaled"*), and it is how a caller lays out the cell so the icon is never
     cropped. `Image::size`'s doc notes an image's size depends on its content
     and this one does not; that difference is the whole of the sizing rule.
   - **`pub fn destination(&self, rect: Rect) -> Rect`**, `#[must_use]` — the
     centred `ICON_SIZE` square **intersected with `rect`**, in a private free
     function `fn centred_square(rect: Rect, side: f32) -> Rect` beside the
     module's helpers, so the geometry is testable with no widget in it. Its doc
     carries the centring argument, `Image`'s top-left rule with the reason it
     does not transfer, and the crop-not-scale-not-overflow rule.
   - **`pub fn style(&self) -> Style`**, `#[must_use]`, over
     `pub struct Style { pub tint: Color, pub opacity: f32 }` — `Image::style`'s
     shape, one field wider, with `opacity` clamped by the module's own private
     `fn bounded(value: f32, low: f32, high: f32) -> f32` and **`NaN` mapped to
     `0.0`**, because `Image`'s own `drawn_opacity` does and a caller may write
     `opacity` directly. **A fifth private copy of that one-line clamp, and the
     convention says so**: `chart.rs`, `gauge.rs`, `image.rs` and `progress.rs`
     each already carry their own private `bounded`, and five callers have still
     not produced a shared module — which is
     `developer.md` § *Phase 2* (*"No abstraction before the second use"*)
     read the way this crate reads it, which is *at* the second use and not
     above it. `icon.rs` writes its own and does not import `image.rs`'s.
   - **`pub fn paint(&self, rect: Rect) -> Vec<DrawCommand>`**, `#[must_use]` —
     **one `Painter::tinted_image` and nothing else**, at
     `self.destination(rect)`, `self.source.uv()`, `self.style().opacity`,
     `radius: 0.0` (the literal, not a property: an icon has no rounded corners,
     and *a filled rounded rectangle is not an outline* is
     why a rounded icon would be a card), and `self.style().tint`. **A
     destination empty on either axis records nothing**, `Image::paint`'s rule
     and its reason. The doc-test asserts one command, the five-plus-one fields,
     and the square's corners.
   - **No `on_event`, no `state`, no `fit`, no `corner_radius`, no `clock`, no
     `tick`, no `snap_to_state`, no `animate_to_state`, no `is_animating`, no
     `size_hint`, no `layout_mut` wrapper.** Each absence is named in the module
     doc with its reason; none of them is an oversight and a reviewer will grep
     for them.

9. **The tests, named, with no display, no network, no filesystem and no wall
   clock** — the only kind `AGENTS.md` permits. **In
   `ui/src/ui_core/src/widgets/icon.rs`'s `#[cfg(test)] mod tests`:**

   - `an_icon_is_twenty_four_pixels_square` — `ICON_SIZE == 24.0`, finite,
     positive, and `icon.size() == Size::new(24.0, 24.0)`, **and**
     `icon.size() == icon.size()` twice, because a size that varied would be a
     scale.
   - `icon_size_fits_the_shared_image_atlas` — with
     **`u32::try_from(ICON_SIZE).unwrap_or(u32::MAX)`** (no `as` cast, per
     `developer.md` § *Code quality*) bound to one local, asserting it is at or
     under `texture::ATLAS_MAX_IMAGE` **and** at or under
     `texture::ATLAS_SIZE / 64`. **The second clause is the § *Uniform sizing*
     arithmetic in a form a mutation cannot pass by being merely smaller**:
     `ATLAS_SIZE / 64` is **32**, so `24` passes and `256` fails, and the failure
     message names both numbers.
   - `an_icon_paints_exactly_one_tinted_image_command` — the length is 1 and it
     is a `DrawCommand::Image`, in `Image::paint`'s destructuring shape.
   - `the_icon_is_centred_in_its_rect` — a 100×100 rect draws a 24×24 quad at
     `(38.0, 38.0)`, **and a 24×24 rect draws it at `(0.0, 0.0)`**, so the
     assertion has both a centring case and an exact-fit case and cannot pass on
     a top-left anchor.
   - `every_icon_records_a_rect_inside_the_rect_it_was_given` — a table of rects
     including **a 16×16 cell, a 24×1 cell and a collapsed 0×0**, each asserting
     the recorded rect is inside the given rect and that a collapsed destination
     records nothing.
   - `the_icon_carries_the_tint_it_was_given` — a tint through `set_tint`, and
     the neutral tint through `reset_tint`.
   - **`a_theme_switch_reaches_the_icon`** — **the required one.** A
     `Theme::dark()` and a `Theme::light()`; `themed_color`-shaped binding of
     `ThemeToken::Text` (the same `Property::bind` over `theme.property(token)`
     the demo's own `themed_color` helper builds, inlined here because
     `ui_core` does not have that helper); the bound property assigned with
     `set_tint`; then:
     1. the recorded tint is the dark theme's `Text`;
     2. `theme.switch_to(Theme::light(), 300)`, one `theme.tick(Duration::from_millis(150))`,
        and the recorded tint is **neither** the dark one **nor** the light one —
        it is strictly between them, which is the animated half;
     3. ticking to the end, the recorded tint is **exactly** the light theme's
        `Text`, which is the arrived half;
     4. **and the second assertion runs with no `Icon` clock and no
        `animate_to_state` call anywhere**, because there is none — the tint moved
        because the theme's own token animation moved it.
   - `an_untinted_icon_paints_opaque_white` — a fresh icon's recorded tint is
     `Color::new(255, 255, 255, 255)`, which is the identity modulate.
   - `an_icon_follows_a_written_tint_at_once` — `icon.tint.set(…)` changes the
     recorded tint **with no snap and no tick**, because `Icon` has no clock.
     This is the testable form of "no `AnimationClock`", and it is the one a
     reviewer should break by adding a `shown_tint` split.
   - `an_icon_paints_square_corners` — the recorded `radius` is exactly `0.0`.
   - `the_icon_samples_the_uv_window_it_was_given` — an `ImageSource::new(w, h,
     uv)` with a non-full `uv` reaches the command unchanged, so a caller that
     hands over an atlas placement gets that placement.
   - `an_icon_in_a_translucent_state_records_the_opacity_it_holds` —
     `opacity.set(1.4)` records `1.0` and `opacity.set(f32::NAN)` records `0.0`,
     in `Image`'s own two-clause shape.
   - `centred_square_is_pure` — the free function over a table of rects and
     sides, including a negative side clamped to `0.0`, with no widget and no
     arena in the test.

   **In `ui/src/ui_core/src/paint.rs`:** `painter_image_records_the_neutral_tint`
   and `painter_tinted_image_records_the_tint_it_was_given`.

   **In `ui/src/ui_core/src/batch.rs`:**
   `two_icons_of_different_tints_share_one_batch`,
   `a_tinted_image_with_a_translucent_tint_blends`, and
   `an_untinted_full_texture_image_still_batches_as_opaque`.

   **In `ui/src/ui_core/src/render.rs`:** the two **existing** tests
   `image_vertex_layout_matches_the_offsets_the_attributes_are_bound_with` and
   `the_image_shader_reads_every_attribute_the_vertex_buffer_supplies` gain the
   new field and the new attribute **and keep their names and every assertion
   they already make**; `the_image_shader_multiplies_the_sampled_texel_by_the_tint`
   and `a_tinted_image_quad_carries_the_tint_it_was_given` are new, and the
   first asserts on the **whole assignment**, not a substring —
   *an unknown-name rule and a mutation of it can be the same value* is why a
   bare `contains("v_tint")` will not do.

10. **`DEMO_APPLICATION.md` § *Library gaps*, row `#4`, gains a dated note**,
    recording: that **`TASK_UI_PRIM_44` added `Icon` and a tint on
    `DrawCommand::Image`**, that **the row is not closed and not marked closed**,
    that **the row's evidence sentence *"no texture tinting at draw time"* is
    now false and is corrected in place** the way § *Corrections to the first gap
    table* prescribes, and **what remains open**: *vector rendering* is
    `TASK_UI_PRIM_39`'s and has not run, so **no glyph exists in this repository
    yet** and `Icon` draws nothing anywhere until it does; and the `L10`
    cross-reference stands, unamended, because ear clipping remains declined.
    **The row is not deleted, its Blocks column keeps "Visual quality — 'real
    icons' requirement", and no note anywhere claims a rendered icon.**

11. **`doc/ui/IMPLEMENTATION_STATE.md` gains one entry**, carrying: the
    architecture and the two rejected ones with their costs; the breaking change
    and that `ui_core` is `publish = false`; the stride change 40 → 56 and the
    new attribute; **`ICON_SIZE = 24.0` and the aliasing arithmetic**; **that the
    tint is on the vertex and not in `BatchKey`, and why**; **that the demo is
    unchanged and no icon is drawn**; **the eight mutations and their kills**;
    the six pages' frame rates; and **the honest limit in the section's own
    register** — *the tint path is exercised by the demo's existing image, whose
    pixels are unchanged, and **no tinted quad is drawn by anything in this
    repository**, so nothing here is evidence that a tint lands in the right
    colour.*
    `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`.ai/workflows/task-sequence.md` § *State*); it points at the code.

12. **The consequence for `TASK_UI_PRIM_39` is recorded, not applied.**
    `TASK_UI_PRIM_39`'s § *The engine constraints that dictate the authoring
    rules* states, as a load-bearing premise, *"**No texture tinting at draw
    time.** … **Therefore every icon needs a separate baked PNG per theme.** A
    one-file icon set tinted at runtime is unreachable in this pipeline."* **This
    task falsifies that premise, and task 39 must be amended before it runs.**
    **This task does not edit task 39's file** — it is a different task's
    specification, it has its own review, and `developer.md` § *Phase 2* says not
    to restructure what you were not asked to touch — so requirement 11's state
    entry carries the amendment **verbatim**, in the form a fresh session can act
    on:

    - **whichever lands second adjusts**, and the state entry says so, so the
      order cannot strand the two halves;
    - **if 44 lands first**, `icons.json` bakes **one set of white icons** — its
      `themes` list becomes a single theme whose hex is `#ffffff`, its `sizes`
      list stays `[24]`, and `len(icons) × len(sizes) × len(themes)` becomes
      **33 files instead of 66** — and task 39's per-PNG tint check becomes a
      check that **every icon is white** rather than a check against two hexes;
    - **if 39 lands first**, its 66 PNGs are **still correct** — a per-theme set
      is a valid input to an untinted path — and 44's tint simply is not used by
      them until a demo task rebakes;
    - **and the amendment is not a simplification of anything**, because the
      tint path is a *pipeline* capability: `Painter::tinted_image` is reachable
      from a `Painter` by any caller, and the demo's `demo.png` is the proof that
      it draws.

13. **The suite, the capture, the `gl.get_error()` read and the frame rate are
    all produced.** From `ui/`: `cargo fmt --check`,
    `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings`,
    `cargo test --all-features` **with the per-binary counts pasted and no test
    deleted, renamed away or weakened**, `cargo doc --no-deps` clean, and
    `cargo audit` **recorded as not installed on this host, not passed**. Then the
    six-page before/after capture of `IMPLEMENTATION_STATE.md`
    § *Verifying a change that draws — the capture method*, verbatim. Then
    **`gl.get_error()` read once after the first frame that draws the `data`
    page's image**, with the instrument's code quoted — because a
    vertex layout that is wrong in a way GL accepts **draws the wrong picture or
    nothing**, and this is the task that changes one. Then the frame rate on all
    six pages.

## Acceptance Criteria

- [ ] **`Icon` exists and is the sixteenth `pub mod`.**
      `grep -c 'pub mod' ui/src/ui_core/src/widgets/mod.rs` returns **16** (was
      **15**), `grep -n 'pub mod icon' ui/src/ui_core/src/widgets/mod.rs` shows
      the line **between `gauge` and `image`**, and
      `git diff -- ui/src/ui_core/src/widgets/mod.rs` shows **that one line and
      nothing else** — `Callback<T>` is untouched.

- [ ] **`DrawCommand::Image` carries a `tint`, and the tint is on the vertex and
      not in the batch key.** `awk '/^    Image \{/,/^    \},/' ui/src/ui_core/
      src/paint.rs` shows `tint: Color` after `radius`. `grep -n 'tint' ui/src/
      ui_core/src/batch.rs` shows the field destructured **only** for the
      `tint.a == u8::MAX` clause of the opaque test and **never** written into a
      `BatchKey` literal. **Two named tests hold both halves:**
      `two_icons_of_different_tints_share_one_batch` adds two
      `DrawCommand::Image` on one texture with tints `(255,0,0,255)` and
      `(0,255,0,255)` and asserts **`batcher.finish().transparent.len() == 1`
      with 2 commands in it**, and
      `a_tinted_image_with_a_translucent_tint_blends` asserts a full-UV,
      opacity-1.0 image whose tint has `a == 128` is keyed
      `BlendMode::Transparent`, while `an_untinted_full_texture_image_still_
      batches_as_opaque` asserts the neutral tint keeps it `Opaque`.
      **Mutation evidence in the handoff:** put `tint` into the key and watch
      the first fail; drop the `tint.a` clause and watch the second fail; restore
      both and watch them pass.

- [ ] **The tint uniform appears in the image fragment shader, and the whole
      assignment is asserted.** `IMAGE_FRAGMENT_SHADER_SRC` contains
      `in vec4 v_tint;` and its last line is exactly
      `frag_color = vec4(texel.rgb * v_tint.rgb * opacity, texel.a * v_tint.a * opacity);`,
      asserted by **`the_image_shader_multiplies_the_sampled_texel_by_the_tint`
      on the whole assignment string**, not on `v_tint` alone.
      `IMAGE_VERTEX_SHADER_SRC` contains `layout(location = 6) in vec4 a_tint;`
      and `out vec4 v_tint;`. The **existing** test
      `the_image_shader_reads_every_attribute_the_vertex_buffer_supplies` **keeps
      its name and every assertion it already makes** and covers `a_tint`.
      **Mutation evidence:** delete `* v_tint.rgb` and watch the new test fail
      for that reason; restore it and watch it pass. A test that asserts a
      substring is not evidence, which is why this criterion names the
      assignment.

- [ ] **The vertex layout matches the offsets, and the stride is 56.**
      `IMAGE_VERTEX_STRIDE == 56`, `IMAGE_TINT_OFFSET == 40`,
      `size_of::<ImageVertex>() == 56`, and `offset_of!(ImageVertex, tint) == 40`
      — the **existing** test
      `image_vertex_layout_matches_the_offsets_the_attributes_are_bound_with`
      extended in place, **name and every prior assertion kept**.
      `grep -c 'vertex_attrib_pointer_f32(6' ui/src/ui_core/src/render.rs` is
      **1**, and `grep -c 'enable_vertex_attrib_array(6)'` is **1**.
      **Mutation evidence:** move `IMAGE_TINT_OFFSET` to 32 and watch the layout
      test fail; restore it and watch it pass.

- [ ] **`Painter::image` is source-compatible and its one production caller is
      untouched.** `git diff` shows `Painter::image`'s **signature unchanged** and
      `Painter::tinted_image` added beside it, with `image` delegating to it.
      `grep -n 'painter.image(' ui/src/ui_core/src/widgets/image.rs` shows **no
      change** — `Image::paint` still calls the five-argument form — and
      `painter_image_records_the_neutral_tint` asserts `image` records
      `Color::new(255, 255, 255, 255)`. **The reason for the delegation is stated
      in the handoff in one sentence: every existing painting call site compiles
      unchanged, and one implementation means there is no second rule to keep
      honest.**

- [ ] **`Image` gained nothing.** `git diff --stat ui/src/ui_core/src/widgets/
      image.rs` shows **exactly one changed hunk**, and it is the doc-test's
      destructuring pattern gaining a `..` or naming `tint`.
      `grep -c 'pub fn ' ui/src/ui_core/src/widgets/image.rs` is **unchanged** from
      the pre-task count, `Image`'s three public properties (`opacity`,
      `shown_opacity`, `corner_radius`) and `ImageFit`'s four variants are
      byte-identical, and `grep -n 'pub fn paint' ui/src/ui_core/src/widgets/
      image.rs` shows the same body with **no new argument** — **`Image::paint`
      still calls `Painter::image` and not `tinted_image`.**

- [ ] **A theme switch reaches the icon, by a named test.**
      `a_theme_switch_reaches_the_icon` runs **all four** clauses: the recorded
      tint is the dark theme's `Text`; after `switch_to(light, 300)` and one
      150 ms tick it is **strict between** the two themes' `Text` colours; after
      ticking to the end it is **exactly** the light theme's `Text`; and
      **no `Icon` clock and no `animate_to_state` call appears anywhere in the
      test**, because there is none to call.
      `an_icon_follows_a_written_tint_at_once` asserts a direct
      `tint.set(…)` changes the recorded tint **with no snap and no tick**, which
      is what "no `AnimationClock`" means as a test rather than as a sentence.
      **Mutation evidence:** break the bind so it reads the token once
      (`let ink = theme.get(token); Property::new(ink)`) and watch clause 2 and
      clause 3 both fail; restore it and watch them pass.

- [ ] **The sizing rule holds and its arithmetic is testable.**
      `ICON_SIZE == 24.0`, `icon.size() == Size::new(24.0, 24.0)` on two
      successive calls, and `icon_size_fits_the_shared_image_atlas` binds
      `u32::try_from(ICON_SIZE).unwrap_or(u32::MAX)` **once** and asserts it is at
      or under `texture::ATLAS_MAX_IMAGE` **and** at or under
      `texture::ATLAS_SIZE / 64`. **The second clause is the § *Uniform sizing*
      arithmetic in a form a "merely smaller" mutation cannot pass**: the bound is
      **32**, so `24` passes and `256` fails, **and the assertion message names
      both numbers and the two percentages** — 0.45 % at 24×24, 51.6 % at
      256×256 — so a failing run says which half moved.
      `grep -n 'fn size' ui/src/ui_core/src/widgets/icon.rs` shows **one** size
      method and **no** `set_size`, no `size_hint` and no `scale` — **the widget
      cannot be scaled, and that is checkable by grep.** The doc comment on
      `ICON_SIZE` carries the 10.67 : 1 / four-texel / 3.5 %-of-footprint
      arithmetic and the `stroke-width="2"` → 0.1875 screen-pixel derivation.

- [ ] **An icon is cropped, never scaled and never overflowing.**
      `every_icon_records_a_rect_inside_the_rect_it_was_given` covers a 100×100
      cell, a 24×24 cell, a **16×16** cell, a **24×1** cell and a **0×0** cell,
      asserting containment on the first four and **no recorded command at all**
      on the last. `the_icon_is_centred_in_its_rect` asserts `(38.0, 38.0)` in a
      100×100 rect **and `(0.0, 0.0)` in a 24×24 rect**, so a top-left anchor
      cannot pass it. `an_icon_paints_square_corners` asserts `radius == 0.0`.
      **Mutation evidence:** return the un-intersected square and watch the
      containment test fail on the 16×16 case; anchor top-left and watch the
      centring test fail.

- [ ] **`Icon` declares no interaction and no state.**
      `grep -c 'pub fn on_event' ui/src/ui_core/src/widgets/icon.rs` is **0**,
      `grep -rn 'pub fn on_event' ui/src/ui_core/src/` returns **8** —
      **unchanged**, the count observable in the tree now, and this task adds
      none — `grep -c 'selected' ui/src/ui_core/src/widgets/icon.rs` is **0**
      outside the module doc that explains why there is none, and there is no
      `pub fn state`, no `pub fn tick`, no `pub fn animate_to_state`, no
      `pub fn snap_to_state`, no `pub fn is_animating` and no `AnimationClock`.
      **The module doc names every one of those absences with its reason**, so a
      reviewer reading the diff finds the reasoning rather than the hole.

- [ ] **`Icon` adds no enum, and the crate's enum count is unmoved.**
      `grep -c 'pub enum ' ui/src/ui_core/src/widgets/icon.rs` is **0**, and
      `grep -rn 'pub enum ' ui/src/ui_core/src/widgets/` returns **12** — the
      twelve of § *What exists*, which is the count `DEMO_APPLICATION.md` row
      `L6a` states. **A brief for this task said thirteen; the tree and the
      document agree on twelve and this criterion makes that disagreement
      checkable rather than settled by assertion.** No glyph-name enum exists,
      and `Icon::new`'s doc says where the names live instead.

- [ ] **`cargo test --all-features` is green against 1894, and the handoff lists
      each new test by name.** Against the baseline of **1894** (1450 `ui_core`
      + 224 `ui_demo` + 220 doctests), the handoff **pastes the real per-binary
      counts** — and **resolves the one-test discrepancy** between the recorded
      1450 and the 1451 `#[test]` attributes observable in the tree, by saying
      which is which rather than by picking one. Every test named in requirement
      9 is listed as present, **and no test was deleted, renamed away or
      weakened.** `cargo fmt --check`, `cargo build --all-targets
      --all-features`, `cargo clippy --all-targets --all-features -- -D
      warnings` and `cargo doc --no-deps` clean. `cargo audit` is not installed
      on this host; that is **recorded, not passed**, and
      `git diff --stat ui/Cargo.toml ui/Cargo.lock` is **empty** — the approved
      direct dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`,
      and this task adds none.

- [ ] **All eight mutations are killed, and the kills are reported.**
      The table in § *Testing* is run end to end, one mutation at a time, and the
      handoff reports the **failing test names and counts** for each, plus the
      restored green. **A mutation the runner refused to apply is recorded as
      *not a result*, not as a kill** —
      *a mutation the runner refused to apply was recorded
      as a kill*. **And every run is made against a tree that was rebuilt** —
      *a build that reports `Finished in 0.0xs` did not
      rebuild* and *`tar -x` restores the archived mtime* are both the reason
      the handoff pastes the build line beside each mutation result.

- [ ] **The six gallery pages are pixel-identical outside the fps band, and the
      mechanism is three stated facts rather than one comfortable one.**
      `Page::ALL`'s six names, release build, captured **before and after** with
      the commands of `IMPLEMENTATION_STATE.md`
      § *Verifying a change that draws — the capture method* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture, and `ffmpeg x11grab`, return black for a GL window),
      `pgrep -a -x ui_demo` in the same call as each `magick import -window <id>`,
      then `magick compare -metric AE before.png after.png null:` per page.

      1. **The criterion is task 34's, unchanged: AE 0 outside `y ≥ 680` on all
         six pages**, every differing pixel inside the fps readout's band, which
         `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided, and what it
         found* records as the one thing two captures of an unchanged frame
         differ in (405 pixels there, **AE 0 over y 80–680**).
      2. **`git diff --stat ui/src/ui_demo/src/main.rs` is empty**, and the handoff
         says why in one sentence: **`TASK_UI_PRIM_39`'s icon bytes are not in
         the tree** — `ui/src/ui_demo/assets/` holds `demo.png` and no `icons/`
         directory — so the demo has no icon to draw, and putting one there would
         mean a placeholder, which operator decision 3 of 2026-09-30 forbids.
      3. **And the criterion is load-bearing rather than vacuous, which is the
         third fact and the one a reviewer should check.** The `data` page records
         a `DrawCommand::Image`, so it is drawn through the **changed**
         `ImageVertex`, the **changed** stride, the **new** attribute at
         `location = 6` and the **changed** fragment shader. **The identity tint
         is what makes AE 0 the right answer rather than a surprise**: with
         `tint = (1, 1, 1, 1)` the new line is the old line, so *any*
         difference on the `data` page is a defect in this change and not a
         change in the picture. **If the `data` page is not AE 0 outside the
         band, this criterion has caught a real break in the vertex layout or
         the shader and the task is not done.**

- [ ] **No GL error on the first frame that draws the image, read once.**
      `gl.get_error()` is read after the first frame that draws the `data` page's
      image and pasted into the handoff **with the instrument's code quoted**.
      **A capture is not a substitute in either direction**: a wrong vertex layout
      can draw the wrong picture, and the recorded command count is asserted by a
      test while the pixels are asserted only by the capture above.

- [ ] **The frame rate is measured on every page and reported.** With the
      script's own line pasted rather than the number expected:
      `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
      the six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `IMPLEMENTATION_STATE.md` § *Current position*
      records as the reason task 24.2's criterion 6 was amended rather than met
      by the script. Every page at or above the floor of **55**, and **the
      handoff says in one sentence why no page is expected to move**: the demo
      draws no icon, records exactly one `DrawCommand::Image` as it did before,
      and the change adds **64 bytes to that one quad's four vertices** —
      a vertex-buffer upload the frame was already making.

- [ ] **What the handoff does not claim, in those words.** It states that **no
      tinted quad is drawn by anything in this repository**, because no glyph
      exists here until `TASK_UI_PRIM_39` runs, and therefore that **no
      acceptance criterion here is evidence that a tint reaches the framebuffer
      in the right colour.** The shader is verified **as a source string and
      through a compile-and-draw of the neutral case**; the pipeline is verified
      **by the six-page capture of the `data` page's untinted image**; the
      widget is verified **by its recorded commands**; and **the aliasing
      argument in § *Uniform sizing* is an argument from `GL_LINEAR` and the
      absence of `generate_mipmap`, not a measurement**, and is labelled as one
      in the code, in this file and in the state file's entry.

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows changes to **exactly** the eight files this task
      names and no others: `ui/src/ui_core/src/widgets/icon.rs` (new),
      `ui/src/ui_core/src/widgets/mod.rs`,
      `ui/src/ui_core/src/paint.rs`, `ui/src/ui_core/src/render.rs`,
      `ui/src/ui_core/src/batch.rs`, `ui/src/ui_core/src/widgets/image.rs`,
      `doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md`.
      `grep -rn 'bezier\|cubic\|quadratic\|curve_to\|glyph_outline' ui/src/`
      returns **nothing**, which is `L10`'s claim still holding;
      `grep -n 'ear\|tessellat\|stencil' ui/src/ui_core/src/widgets/icon.rs`
      returns **nothing** — the two files that legitimately *discuss* ear
      clipping are `paint.rs` and `render.rs`, and they discuss it to **decline**
      it, which is why the grep is scoped to the new widget and not to the crate.
      `git diff --stat` is **empty** for `ui/src/ui_core/src/theme.rs`,
      `layout.rs`, `font.rs`, `texture.rs`, `property.rs`, `animation.rs`,
      `input.rs`, `node.rs`, `arena.rs`, `lib.rs`, `render/context.rs`,
      `render/target.rs`, `render/blur.rs`, `ui/src/ui_demo/src/main.rs`,
      `ui/src/ui_demo/src/fps.rs`, `ui/Cargo.toml` and `ui/Cargo.lock` —
      **in particular `theme.rs` does not change, so `TOKEN_COUNT` stays at 33
      and no `ThemeToken` is added.** **And `render/matrix.rs`,
      `render/mesh.rs` and `render/meshio.rs` do not exist and must not be
      created**: they belong to tasks 35, 37 and 38, `git ls-files` returns
      nothing for them at `HEAD`, and their appearance here would be a different
      task's work in this diff.

- [ ] **`#4` is amended, dated, and its claim is narrowed rather than closed.**
      Row `#4` in `DEMO_APPLICATION.md` § *Library gaps* carries a dated note
      naming `TASK_UI_PRIM_44`, stating that **`Icon` and a tint on
      `DrawCommand::Image` now exist**, that **its evidence sentence *"no
      texture tinting at draw time"* is corrected in place** because it is now
      false, and **what remains open**: *vector rendering* is
      `TASK_UI_PRIM_39`'s, has not run, and **no glyph is in the repository**.
      **The row is not deleted and not marked closed, its Blocks column keeps
      "Visual quality — 'real icons' requirement", and `L10` is not amended.**
      **And no doc comment in any changed file asserts the opposite of the code
      beside it** — the defect `DEMO_APPLICATION.md`
      § *Corrections to the first gap table* records twice — which here means, in
      particular, that **no comment anywhere claims an icon is on screen.**

- [ ] **The decisions are written down where the next agent finds them.**
      `icon.rs`'s module doc carries requirement 8's content, including the
      aliasing arithmetic, the crop-not-scale rule, **every** interaction absence
      with its reason, and the "no icon is drawn yet" statement; `paint.rs`,
      `batch.rs` and `render.rs` each carry their half of requirements 2 to 7; and
      `doc/ui/IMPLEMENTATION_STATE.md` carries requirement 11, **including
      requirement 12's `TASK_UI_PRIM_39` amendment verbatim**, so a fresh session
      running task 39 finds the falsified premise without reading this task file.

## Out of Scope

- **No ear clipping, no concave fill, no stencil pass, no fill rule — row `L10`,
  a separate task, and declined by the operator on 2026-10-02.** `Icon` does not
  need one and does not add a geometry primitive of any kind: it records a
  `DrawCommand::Image` and nothing else. **`L10` is not amended, not narrowed
  and not re-opened here**, and `grep -n 'ear\|tessellat\|stencil' ui/src/
  ui_core/src/widgets/icon.rs` returns nothing — the two files that legitimately
  *discuss* ear clipping are `paint.rs` and `render.rs`, and they discuss it in
  order to **decline** it.
- **No `TabBar` (gap `#7`), no `Button::selected`, and no edit to
  `ui/src/ui_core/src/widgets/button.rs` at all.** `Button` is untouched: no
  field, no setter, no `state()` change, no doc edit beyond a cross-reference if
  one is wanted. What `Icon` owes a future `TabBar` is written out in §
  *Interaction with `Button`* — a writable `tint` and `opacity` — and what it
  refuses it is written out in the same place.
- **No SVG, and no runtime rasterisation of anything.** No XML, no path parser,
  no curve tessellation, no image decoder. `TASK_UI_PRIM_39` bakes Lucide's SVGs
  **offline, outside the binary**, and requirement 12 records what this task does
  to its authoring rules without editing its file.
- **No icon authoring tooling, and no icon inventory.** Not a manifest, not a
  script, not a directory of PNGs, not a glyph-name enum. The names live in
  `TASK_UI_PRIM_39`'s `icons.json`; `ui_core` takes a `TextureHandle`.
- **No demo page, no new page, and no change to any of the six.**
  `DEMO_APPLICATION.md` § *What a seventh page costs* enumerates what one costs,
  and `ui/src/ui_demo/src/main.rs` is not edited at all — `git diff --stat` on
  it is empty, `Page::ALL` stays `[Page; 6]`, `Page::DEFAULT` stays `Pads`, the
  `--help` text is unchanged and no new `--tab=` name exists. **The demo placing
  an icon is a `TASK_UI_DEMO_n` item and it needs task 39's bytes first.**
- **No generated glyphs.** The seat widget's squiggle count, the indicator
  column's severity ramp and its blink timing are *"drawn from state, not
  selected from an atlas"* by `DEMO_APPLICATION.md` § *Asset requirements*, and
  `Icon` draws **one authored glyph in one colour**. It has no severity, no
  blink, no level and no `Severity`/`GaugeType`-style mode — a widget whose
  content is generated from state is a different widget with a different paint.
- **No per-node clipping, and no fix to gap `#5`.** `DrawCommand` still carries
  no scissor state; `Icon`'s crop rule bounds its own damage to its own cell
  precisely because nothing clips it. Clip **ownership** is gap `#5` and a
  separate task.
- **No layout change, and no reliance on anything unimplemented.**
  `LayoutMode::Grid` still lays out nothing and `Flex.wrap` is still discarded,
  so an icon is placed absolutely like every other node in the demo.
  `Padding` and main-axis `spacing` are all the icon-plus-label row has, per
  `L7`.
- **No theme token, and no edit to `ui/src/ui_core/src/theme.rs`.**
  `TOKEN_COUNT` stays at **33** and `ThemeToken::all` is unchanged. An icon is
  ink, `ThemeToken::Text` is ink, and a token for "not selected" or a scoped
  token is `L9`'s and `L6b`'s — both separate tasks, and re-opening either here
  would be exactly the "two documents each claiming ownership of one definition"
  failure.
- **No font change and no icon font.** `ui/src/ui_core/src/font.rs` is untouched:
  no `FACE_COUNT` change, no third weight, no `GlyphKey` change, no glyph in the
  glyph atlas, no `Font` added to a `FontSet`. Option (C) is declined above with
  its four costs.
- **No new dependency and no `unsafe`.** `ui/Cargo.toml` and `ui/Cargo.lock` are
  unchanged — the approved direct dependencies remain `sdl3 0.20`, `glow 0.18`
  and `freetype-rs 0.38`. And `grep -c unsafe ui/src/ui_core/src/widgets/icon.rs`
  is **0**: `Icon` contains no GL at all, and the pipeline change adds no new
  `unsafe` block — it extends the attribute-pointer block and the upload's
  existing SAFETY comment.