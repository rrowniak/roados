# TASK_UI_PRIM_49: Gap `L8` — a Text Run That Knows Its Own Width

## Goal

Put the width of a drawn text run **on the command**, so that
`scroll::command_bounds` can bound a `DrawCommand::Text` instead of answering
`None` for one — which is the mechanism row `L8` in `doc/ui/DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* says is missing when it reads *"so a
half-visible row cannot be clipped"*. **This task clips nothing.** It supplies the
number a clip needs and no clip; `TASK_UI_PRIM_45` supplies the clip and no
number, and the two are separate tasks.

**The headline, and it is the whole design: at every place in the crate that
would set this width, the number is already computed and thrown away.** It is not
a measurement task. It is a plumbing task, and the file says so in those words
because the cost argument and the `AdvanceCache` argument both fall out of it.

## Context

### What is in the crate at `75a896c` plus the uncommitted diff, established and not re-derived

Citations are **by symbol and by file path**. The tree is being modified in
parallel and a line number written today is wrong tomorrow.

| Fact | Where |
|---|---|
| **`DrawCommand::Text` has eight fields, exactly and in this order:** `x: f32`, `y: f32`, `text: String`, `color: Color`, `font_size: f32`, `extra_advance: f32`, `family: FamilyId`, `weight: FontWeight`. **No width.** `y`'s own doc says it is the *"**top** of the line's box, not the baseline"*, and adds that *"a caller that has measured a line does not have to know the ascent"* — **the command's doc already names the box this task needs to bound.** | `ui/src/ui_core/src/paint.rs`, `DrawCommand::Text` |
| **`scroll::command_bounds(&DrawCommand) -> Option<Rect>` returns `None` for a `Text`**, and its doc gives the reason: *"A text run carries the `x` and `y` of its line's top-left and its font size, but **not its width** — that needs the font's advance for every character in the run — so bounding one honestly would mean inventing a width."* **This is the function that has to change and the name of the seam.** | `ui/src/ui_core/src/widgets/scroll.rs`, `command_bounds` |
| **`Painter::text`, `Painter::text_in` and `Painter::text_bold` all route through one private `text_in_weight`**, documented as *"The one place a text command is recorded, so neither the weight nor the family can be a field some of the painters forget."* **The width is added there, once.** | `ui/src/ui_core/src/paint.rs`, `Painter::text`, `Painter::text_in`, `Painter::text_bold`, `Painter::text_in_weight` |
| **`Rect::new` clamps nothing**, and `scroll::intersects` is four inclusive comparisons. **A `NaN` in any coordinate makes every comparison false, so `clip_commands` drops the command.** That is the failure a non-finite width would cause, and it is why requirement 8 filters on `is_finite`. | `ui/src/ui_core/src/paint.rs`, `Rect::new`; `ui/src/ui_core/src/widgets/scroll.rs`, `intersects`, `clip_commands` |
| **`List::paint` is the only production caller of `clip_commands`** in the whole crate, and `scroll::command_bounds`'s other callers are `render/blur.rs`'s shadow reach, `paint.rs`'s own tests and the demo's guards. | `ui/src/ui_core/src/widgets/list.rs`, `List::paint`; `ui/src/ui_core/src/render/blur.rs` |
| **The demo records no `List`**, so no command in the shipped frame passes through the one function whose answer this task changes. This is the mechanism of the pixel-identical criterion, and it is three facts (§ *Scope*). | `ui/src/ui_demo/src/main.rs` — `grep -n 'List::new'` returns nothing |
| **Eight of the ten production `Painter::text*` call sites — in six widgets — already hold the width** and discard it: `Label::paint` records `layout.lines[i].width` one line after computing it; `Button::paint` the same; `Toast::paint` records `measured.lines[i].width` and `Line` is `label::Line`; `Dialog::paint_lines` the same, under a comment saying *"this function measures nothing"*; `Keyboard::paint` computes `text_width` for centring; `TextInput::paint`'s `visible_run` accumulates every advance it walks. | `ui/src/ui_core/src/widgets/label.rs`, `Label::paint`; `button.rs`, `Button::paint`; `toast.rs`, `Toast::paint`; `dialog.rs`, `Dialog::paint_lines`; `keyboard.rs`, `Keyboard::paint`; `text_input.rs`, `TextInput::paint`, `visible_run` |
| **`Chart::paint(&self, rect: Rect)` takes no measurement seam at all** — no font, no `advance`, no family — and `Chart::draw_labels`'s doc says why in its own first words: *"**Placed, never measured**, because [`DrawCommand::Text`] carries no width and nothing outside the text pipeline knows how far a run reaches."* **This is the one caller whose honest answer is `None`, and it is why the field is an `Option`.** | `ui/src/ui_core/src/widgets/chart.rs`, `Chart::paint`, `Chart::draw_labels`, `Chart::x_labels` |
| **`FontSet::advance` answers `replacement_advance(size)` for a character no font in the chain covers**, so a run always has an advance for every character and a missing glyph leaves a hole exactly as wide as the box drawn in it. **`FontSet::measure` sums those advances.** | `ui/src/ui_core/src/font.rs`, `FontSet::advance`, `FontSet::measure`, `replacement_advance` |
| **`AdvanceCache` exists and is keyed by `(char, rounded_pixel_size)`** because an uncached `load_char` costs about **61 µs** — *"a frame spent roughly 200 ms here and the whole application ran at about 4 frames per second"*. **This task does not touch it, does not add a lookup to it, and does not move a measurement into a hotter place.** | `ui/src/ui_core/src/font.rs`, `AdvanceCache`, `Font::advance` |
| **`font.rs` has no outline extraction and no scale-free description of a glyph edge.** Its module doc records that a signed distance field was built here and removed, and why: *"A distance field has to binarize"*. Glyphs rasterise per size into an atlas keyed by `(char, rounded_size, FaceId)`. **Nothing this task needs is behind that decision, and nothing this task does needs it.** | `ui/src/ui_core/src/font.rs`, module docs; `Font::rasterize` |
| **`Font::from_path` is the only font loader** — there is no font-from-memory path — and `FontSet::new` creates an **empty** default family, so `FontSet::measure` on a bare set returns `replacement_advance(size)` per character. **No test may open a font file**, `AGENTS.md` forbids it, and task 30's record says the same about the mutations that needed one. | `ui/src/ui_core/src/font.rs`, `Font::from_path`, `FontSet::new` |
| **Text IS tinted.** The text fragment shader multiplies `coverage` by a per-vertex `v_color`, over a `GL_R8` / `GL_RED` atlas sampled `GL_LINEAR`. **So a fade ramp is reachable without touching the atlas or the blend mode** — which is `TASK_UI_PRIM_32`'s finding, not this task's. | `ui/src/ui_core/src/render.rs`, the text shader sources |
| **Per-node clipping is being closed by `TASK_UI_PRIM_45`**, and `apply_clip` sets a scissor per batch from the node's own `LayoutState::clip`. **A text run that overflows its node's box is not cut today**, and that is the consumer this task enables: with a width on the command, a caller can ask where a run ends, and 45's resolution can be reasoned about rather than assumed. **This task does not clip anything and delivers neither of row `L8`'s two blocked items.** | `doc/ui/TASK_UI_PRIM_45.md`, `Renderer::draw_node_clipped`, `Renderer::apply_clip` |
| **`Truncation` is a `pub enum` with four variants** — `None`, `Clip`, `Ellipsis`, `Fade` — and `Fade`'s own doc says *"Layout-wise identical to [`Truncation::Clip`]: the fade is a rendering effect, and the renderer that will apply it does not exist yet."* **`doc/ui/IMPLEMENTATION_STATE.md` § *Tasks 30–32, the text gaps task 11 left*, row 32** records the state: *"`truncate_line` treats `Clip` and `Fade` identically and `Label::paint` never reads `truncation`, so there is **no fade ramp at all** and `Clip` is a layout cut, not a visual clip."* **Read before writing this file; see § *The fade ramp is out, and what this task owes it instead*.** | `ui/src/ui_core/src/widgets/label.rs`, `Truncation`, `truncate_line`, `Label::paint` |
| **The text pass does not need a width, and that is why adding one costs nothing on the GPU.** `render.rs`'s `text_vertices` destructures `DrawCommand::Text` **exhaustively, with no `..`** — it is the production text pass and it names every field it reads. The pen advances from each glyph's own advance. | `ui/src/ui_core/src/render.rs`, `text_vertices`, `draw_text_batch` |
| **Four places in the tree stop compiling against a ninth field, and each is named by symbol.** **Two exhaustive patterns**: `paint.rs`'s `text_bold_differs_from_text_in_the_weight_and_in_nothing_else`, whose own comment says *"If a second field ever rides along with the weight, this is what catches it"* — so it gains a binding and **not** a `..` — and `render.rs`'s `text_vertices`, the production text pass. **Two literal constructions**: `render.rs`'s `text_and_image_commands_expand_to_no_quads` and `scroll.rs`'s `a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width`. **Every other `DrawCommand::Text` pattern in `ui/src` uses `..` already** and is unaffected. | `ui/src/ui_core/src/paint.rs`, `text_bold_differs_from_text_in_the_weight_and_in_nothing_else`; `ui/src/ui_core/src/render.rs`, `text_vertices`, `text_and_image_commands_expand_to_no_quads`; `ui/src/ui_core/src/widgets/scroll.rs`, `a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width` |
| **Test baseline: 1894** — `ui_core` **1450**, `ui_demo` **224**, doctests **220**. Measured at `75a896c` plus the uncommitted diff, in the session that wrote this file, and re-measured while writing it. | `cargo test --all-features` from `ui/` |
| **`.ai/tools/fps-check.sh` takes `seconds` then `floor` and runs the binary with no arguments**, so it **cannot name a page**; per-page is `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>`. Every run is measured — `.ai/workflows/task-sequence.md` § *Gates*. | `.ai/tools/fps-check.sh` |

### Row `L8`, in the document's own words

`doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*, row
**`L8`**:

> **No offscreen text measurement on the draw command.** `DrawCommand::Text`
> carries no width, so a half-visible row cannot be clipped. **The previous row's
> field list was also incomplete** — there are seven fields, not six:
> `weight: FontWeight` is the seventh and the single place a weight lives.

Severity **Medium**. **Blocks: card content that overflows, the carousel's page
edges.**

**That row's own correction is now one field stale, and it is the third falsity
this task fixes.** The row says seven fields; there are **eight**;
`family: FamilyId` is the eighth and was added after the row was checked. The
`Blocks` column names two products, and **neither exists in this repository**:
there is no card and no pager, and `doc/ui/DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* row `#6` and row `#7` are the card and
the tab bar's open rows. So the row's *claim* is about `ui_core` and this task
closes the mechanism; the row's *Blocks* column stays intact, and requirement 15
says so in those words.

### The falsities this task fixes, each by file and symbol

Three, and one of them is a test whose name is the claim. **Requirement 14 is the
work; this list is what a reviewer greps.**

| # | File | Symbol / place | The claim, verbatim, and what is true instead |
|---|---|---|---|
| 1 | `ui/src/ui_core/src/widgets/text_input.rs` | module docs, § *Clipping, and why the widget has to do it itself* | *"A [`DrawCommand::Text`] carries an `x`, a `y`, a `String`, a colour, a size and a tracking, and **no width**: `scroll::command_bounds` returns `None` for one, because bounding a run honestly would mean inventing a width it does not carry, and `scroll::clip_commands` therefore *keeps* every text run whatever the viewport is"* — **the enumeration is two fields stale** (`family` and `weight`), **and every conclusion after it is false** after this task. Two further sites in the same file make the same claim: `TextInput::paint`'s doc (*"There is exactly one text run whatever the text's length, because the run carries no width and therefore cannot be trimmed"*) and `visible_run`'s (*"This is the clip, and it is a walk rather than a measurement because a [`DrawCommand::Text`] carries no width"*). |
| 2 | `ui/src/ui_core/src/widgets/scroll.rs` | `command_bounds`, and `clip_commands`' three-outcome list | *"...but **not its width** — that needs the font's advance for every character in the run — so bounding one honestly would mean inventing a width."* And *"...a [`DrawCommand::Text`] run does not carry its width at all."* **Both become false, and the second is the sentence that has to change with the behaviour**: a measured run wholly outside a viewport is now **dropped**. |
| 3 | `ui/src/ui_core/src/widgets/chart.rs` | `Chart::draw_labels`, `Chart::x_labels`, and the module docs § *Labels are placed, never measured* | *"**Placed, never measured**, because [`DrawCommand::Text`] carries no width and nothing outside the text pipeline knows how far a run reaches."* **The first clause stays true for the chart and the reason changes owner**: the command *can* carry a width, and **the chart's is `None`, because `Chart::paint` takes no measurement seam**. **Six production doc sites name the old reason** — the module docs' § *Labels are placed, never measured*, `Y_LABEL_GUTTER`, `Chart::stroke_reach`, `Chart::paint`'s § *What reaches outside `rect`, and how far* (whose table row reads *"**an x label's right-hand end** \| **unbounded, and unknowable** \| none: [`DrawCommand::Text`] carries no width"*), `Chart::draw_labels` and `series_label` — **and three more inside `#[cfg(test)]`.** |

**One existing test is the claim too, and requirement 12 renames it rather than
leaving a green test whose name is false:**

`ui/src/ui_core/src/widgets/scroll.rs`,
`a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width`, whose
assertion message is *"there is no width in the command to bound it with"*. It
constructs a `DrawCommand::Text` **literally with all eight fields**, so it does
not compile against a ninth — and its whole claim is this task's deliverable.

**Three more sentences in `paint.rs` are wrong about the argument counts**, and
this task adds an argument, so a reader will reconcile them against the new
signature whether or not the file says so. `Painter::text_in`'s comment says *"The
same **eight** arguments [`Painter::text`] takes, plus the family"* — `text` takes
**six** and `text_in` takes **seven**. `Painter::text_in_weight`'s says *"**Nine**
arguments is the command's **eight** fields plus the weight"* — the function takes
**eight**, and both the weight and the family are already among the command's
fields, so nothing is being added. **Requirement 14.4 corrects all three counts to
the counts the code has.**

## The width field is `Option<f32>`, and `None` is a fact rather than a gap

**Decision: a ninth field, `width: Option<f32>`, appended after `weight` — and the
three recording methods that set it are new, not changed.**

### Why `Option<f32>` and not `f32` with a sentinel

1. **`Chart` is the reason, and it is not hypothetical.** `Chart::paint(&self,
   rect: Rect)` takes no font, no `advance` and no family, and it records two
   label runs per axis per sample. It has **no way to know a width**, so a field
   it cannot fill needs an absent answer. A widget whose signature cannot carry a
   measurement is an ordinary state in this crate — it is the same state
   `FontSet::ascent` and `FontSet::line_height` answer with `None`, and
   `Keyboard::advance` exists because `Keyboard::paint` takes no font either.
2. **`0.0` is not available as a sentinel, and the reason is arithmetic rather
   than taste.** **An empty run measures exactly `0.0`** — `layout_text` produces
   empty `x_labels` entries by design (`Chart::x_labels`' own rule: *"a caller
   labelling every third sample writes an empty string at the others' indices"*) —
   so `0.0` cannot mean "unmeasured" without making a measured empty run
   indistinguishable from an unmeasured one.
3. **A zero-width sentinel is not merely ambiguous; it is a behaviour regression,
   and this is the decisive argument.** `clip_commands` **drops** what a bound
   says is wholly outside the viewport. A `Text` bound to a **zero-width** box at
   `x` intersects a clip whose right edge is at or past `x` and **is kept** — but
   a `Text` whose `x` is past the clip's right edge is **dropped**. **So a
   sentinel would convert "a text run on screen is never dropped for want of a
   measurement this layer has no way to take" — `clip_commands`' own doc — into
   the opposite, for exactly the commands `Chart` records.** `Option<f32>` is what
   keeps that sentence true.
4. **Consistency with the crate's existing answer for a missing glyph, stated
   carefully because the two cases are *not* the same.** `FontSet::advance`
   answers `replacement_advance(size)` for a character no font covers — a real,
   non-zero number — because the character *will be drawn* as a replacement box
   and the hole in the layout must be as wide as the box in it. Here the case is
   different in kind: **no font was consulted at all, and nothing narrower than
   the truth is available.** The rule that transfers is not "always return a
   number" but *"a substitute is allowed only where it is as wide as what is
   drawn"* — and for `Chart` no such number exists, so the honest answer is none.
5. **`f32::NAN` is not a sentinel either**, for the reason in the facts table: a
   `NaN` coordinate fails every one of `intersects`' four comparisons, so
   `clip_commands` drops the command. Requirement 8's `is_finite` filter is what
   makes a `NaN` width behave like `None` rather than like a zero-width box at
   infinity.

### Where the field sits, and why appended

**Ninth, after `weight`.** Two reasons. First, **the diff**: appending is one
added line and no reordered line, so `git diff` on `paint.rs` shows the addition
and nothing else — and `render.rs`'s exhaustive `text_vertices` destructuring
takes one added binding rather than a reordering. Second, the reading order is
right: `family` and `weight` say **which face** the run is drawn with, and the
width is **what that face did to the pen**, so it belongs after them.

### The three methods are new, not changed — and the repo's own convention says so

**Decision: `Painter::text_measured`, `Painter::text_in_measured` and
`Painter::text_bold_measured` are added beside their unmeasured twins; the three
existing signatures are untouched.**

1. **It is not a breaking public API change.** `Painter::text`,
   `Painter::text_in` and `Painter::text_bold` are `pub` on a library that
   `developer.md` § *API design* protects with *"Respect semver. Public API changes
   that break downstream users require explicit discussion."* Adding three
   methods is additive; adding a seventh parameter to three existing methods is a
   break, and this task's brief does not authorise a break.
2. **A second route into a text command is what the crate already refuses.** The
   comment at `Dialog::paint_lines` states the rule in the crate's own words:
   *"One branch and two calls rather than a weight carried into the command by
   hand: `Painter` keeps the two faces apart on purpose, and a third route into a
   text command is a third place for the two of them to disagree about anything
   else."* A new **variant** of a text run is a new method in this codebase, not a
   new argument — that is how `text_bold` and `text_in` came to be.
3. **Three, not six.** The width is one dimension; family and weight stay where
   they are. The three new methods are the same three variants with the width in
   front, next to its twin, and **the discriminating argument leads** — which is
   `Painter::text_in`'s own convention with `family` first.
4. **The footgun is named, bounded, and then counted, because counting it turns a
   worry into a fact.** Six methods can record a text run. The mitigation is not a
   lint; it is that **the unmeasured triple is documented as the chart's path**,
   and requirement 11's `a_chart_label_records_no_width_because_the_chart_cannot_measure_one`
   makes using it for anything else a deliberate break. **After this task the
   production callers are exactly:** `text_measured` — `Button::paint`,
   `Toast::paint`, `Keyboard::paint`, `TextInput::paint`; `text_in_measured` —
   `Label::paint`; `text_bold_measured` — `Dialog::paint_lines`; `text` —
   **`Chart::draw_labels` and nothing else**.
5. **Two of the six lose their last production caller, and they stay.** `text_in`
   was `Label::paint`'s only production caller and `text_bold` was
   `Dialog::paint_lines`'s, so both are unused afterwards. **They stay because they
   are `pub` on a library, and deleting a public method is the exact break this
   design exists to avoid** — and because this crate has already shipped exactly
   this shape: `text_bold` and `text_in` were both added while every call site in
   the tree used `text`. **The honest limit, and it is one a reviewer should
   record:** `text_in` and `text_bold` are now *unmeasured* twins with no
   production user, so a reader looking for a way to record a run in a family or
   in bold **without** a measurement has two methods and no example in the tree.
   Their docs gain one sentence naming `text_in_measured` and `text_bold_measured`
   as the twins callers usually want.

```rust
/// Records a text run whose width the caller has measured, in the renderer's
/// regular face and its **default family**: every argument as
/// [`Painter::text`], plus `width` in front.
///
/// **`width` is the pen's travel and this method does not check it.** It is the
/// number the caller measured with the chain it drew with, and recording it is
/// not this method's decision to second-guess — a non-finite width is recorded as
/// it was handed in and
/// [`crate::widgets::scroll::command_bounds`] declines to bound the run, so
/// there is **one** place where a bad width is answered and not six.
#[allow(clippy::too_many_arguments)]
pub fn text_measured(
    &mut self,
    width: f32,
    x: f32,
    y: f32,
    text: &str,
    color: Color,
    font_size: f32,
    extra_advance: f32,
) {
    self.text_in_weight(
        FamilyId::default(),
        Some(width),
        x, y, text, color, font_size, extra_advance,
        FontWeight::Regular,
    );
}
```

`text_in_measured` takes `family` first and `text_bold_measured` is
`text_measured` with `FontWeight::Bold`, both documented as *"every argument as
[`Painter::text_measured`], one family / one weight different"*. **All three route
to `text_in_weight`, which gains `width: Option<f32>` as its second parameter and
remains `#[allow(clippy::too_many_arguments)]`.** Its doc's first line keeps its
promise — *"The one place a text command is recorded, so neither the weight nor
the family nor the width can be a field some of the painters forget"* — and
requirement 14.4 corrects its argument count.

## The measurement comes from the caller, and that is the caching decision

**Decision: `Painter` does not measure. The caller passes the width it already
has. `AdvanceCache` is not consulted by this task at all.**

Five reasons, and the first is the whole design.

1. **Eight of the ten production call sites already computed the number.** The
   table in § *What is in the crate* names each one. `Label::paint` calls
   `layout_text`, which ends every line with `measure(&text, letter_spacing,
   advance)` and stores the result in `Line::width` — **one line above the
   `painter.text_in` call that discards it.** `Button::paint`, `Toast::paint` and
   `Dialog::paint_lines` are the same shape. `Keyboard::paint` computes
   `text_width` because it has to centre on it. `TextInput::paint`'s `visible_run`
   accumulates `advance(ch) + letter_spacing` per character and returns
   `(String, f32)` — the offset it needs and the sum it does not yet return.
   **So this task adds no measurement, moves none, and repeats none.** The
   `AdvanceCache` question answers itself: the measurement happens where it
   already happened, **at layout time, upstream of the painter**, and the cache
   that serves it is untouched.
2. **A widget cannot measure text without a font, and the crate's seam for that
   is `advance: &dyn Fn(char) -> f32`, not `Painter`.** `Label::paint`,
   `Label::layout`, `Button::paint`, `Toast::paint` and `TextInput::paint` all
   take it; `layout_text` takes it; `FontSet::advance` is *"the measuring half of
   the text path"*. **A `Painter` that measured would be a second, different
   measurement seam**, and *two documents each claiming
   ownership of one definition* is the rule about two.
3. **Weighing a parameter against a handle, since the brief asks.** A `&Font` or a
   `&FontSet` on `Painter::text` is a seventh argument too — and the **wrong** one,
   because `Font::measure` does not know the **family chain** or the **weight**,
   and `FontSet::measure` needs both. A measurement *handle* (a `FontSet` held by
   the `Painter`, or a measuring closure on it) makes `Painter` stateful per
   frame, changes `Painter::new()`, and moves a font into a type whose entire
   content is a `Vec<DrawCommand>`. **The caller already holds the closure; the
   handle would be a second copy of it.**
4. **What the caller must guarantee, and where it is already written down.**
   `Label::paint`'s doc: *"**The `advance` callback and the recorded family must
   be the same chain**, and the caller is the only thing that can see both: this
   method knows the family (from the property) and is handed the advances (from
   `advance`), and a run laid out with one chain's advances and drawn in another's
   is a line whose words do not land under their glyphs."* **The invariant this
   task establishes is that sentence applied to the number**: *a recorded width is
   the pen's travel, measured with the chain that drew the run.* The new field's
   doc says exactly this and cites `FontSet::advance` for it.
5. **The one caller whose number is not the font's true width says so.**
   `Keyboard::paint` takes no font and centres on `advance * count` from a
   `Property<f32>` whose doc already says a caller with a font should put a real
   one in it. Its recorded width is therefore **the number it centred on**, and
   that is the number that matters: `x + width` is the pen's rest, consistent with
   where the run is drawn. The field's doc states the invariant as *consistency
   with the drawing*, not as *truth about the typeface*, because a keyboard key's
   box must agree with its own centring even on a host with no font file.

### The recorded width includes the trailing `extra_advance`

**Decided, and it is a decision a reviewer will check.** `extra_advance`'s own doc:
*"Pixels added after every glyph, **including the last**"*, and `Label`'s `measure`
adds `letter_spacing` after every character including the last — the same
arithmetic. So **the recorded width is the pen's final `x` minus this run's `x`,
the trailing advance included, and `x + width` is where the pen comes to rest
rather than where the ink stops.** Requirement 11's `Label` test asserts it at a
**non-zero tracking** as well as at zero, so that a width which forgot the last
glyph's advance fails.

### What else consumes the width, and what deliberately does not

| Consumer | This task's answer |
|---|---|
| **`scroll::command_bounds`** | **Gains a text arm** — requirement 8. This is the seam. |
| **`scroll::clip_commands`** | **Its behaviour changes for a measured run and not for an unmeasured one** — a measured run wholly outside a viewport is now dropped. Requirement 9 pins both halves. **Its only production caller is `List::paint`, and the demo records no `List`**, so this is observable in the crate's tests and nowhere in the shipped frame. |
| **`TextInput`'s selection and caret** | **Untouched.** `position_of`, `offset_at_position`, `caret_x`, `selection_rect` and `ensure_caret_visible` all read the same `advance` closure and none of them reads the recorded width. **Using the new field to shorten `visible_run` would be circular**: the run's width is a *consequence* of the walk, and `to` is the inner box's right edge, not a function of the width. Requirement 6 says so in those words. |
| **`Truncation::Ellipsis`** | **Untouched.** `truncate_line` already cuts by measurement and `Label::paint` already produces the width of the cut line — this task adds the field, not the ellipsis. `LayoutOptions::truncation`, `LayoutOptions`' `Default` and the ellipsis budget arithmetic in `truncate_line` are byte-identical. |
| **`render.rs`'s text pass** | **Does not read it, and must not.** `text_vertices` binds the field and does not use it: the pen advances from each glyph's own advance, and a second source of pen positions in one function is *two owners of one definition* in the GPU path. Requirement 10 states it and the pixel-identical criterion is the check. |
| **`ui_demo`'s `inked_box`** | **Untouched, and its `Text` arm keeps its zero width.** It answers *"does this put ink above the tab bar"*, over every node on every page — a **y** question. Widening it would make a tab-bar guard assert something about the width of a text panel. Its doc's reason is corrected (falsity 3's sibling), nothing else. |
| **`Label`'s `Line::width`**, `layout_text`, `truncate_line` | **Untouched.** The number is read, not recomputed. |

## The fade ramp is out, and what this task owes it instead

**Plainly: this task implements no fade ramp, no alpha gradient, no per-glyph
alpha factor and no shader term.** `Truncation::Fade` remains layout-identical to
`Truncation::Clip`, `truncate_line` keeps the arm it has, `Label::paint` still
never reads `truncation`, and `doc/ui/IMPLEMENTATION_STATE.md`
§ *Tasks 30–32, the text gaps task 11 left*, row 32, keeps reading exactly as it
reads today: **there is no fade ramp at all.** `TASK_UI_PRIM_32` owns it and this
task does not touch it.

Three things are said here so that the relationship is checkable rather than
implied.

1. **This task is upstream of task 32 and supplies the one number 32's
   requirement 2 needs.** Task 32's requirement 2 is *"The ramp follows the
   **text's** cut edge, not the container's: with `TextAlign::Right` or `Center`
   the fade is at the end of the drawn run, which is where the overflow is."* **The
   drawn run's cut edge is `x + width`**, and today nothing in the repository can
   read it off a recorded command — which is why task 32's file, written on
   2026-09-30, could state the requirement and not the mechanism. Requirement 16
   records this in `IMPLEMENTATION_STATE.md` in one sentence, so the next agent
   does not have to re-derive the ordering.
2. **The width is necessary and not sufficient, and saying so is what stops this
   task being read as closing 32.** Task 32's requirement 6 is *"Decide and record
   whether the ramp is a per-glyph alpha factor or a shader term"* — a decision
   about the recorder and the fragment shader that no width settles. Task 32's
   requirement 4 (per-node clip not leaking between two labels) is **task 45's**
   work, not this task's. **So after this task, 32 has a datum it did not have and
   still has all three of its own decisions to take.**
3. **The text shader can tint per vertex today, which is 32's finding and this
   task's confirmation of it.** The text fragment shader multiplies `coverage` by
   `v_color` over a `GL_R8` atlas. **So a ramp needs no atlas change, no blend-mode
   change and no new draw path** — and equally, **nothing in this task's field is a
   ramp.** `width` is a rectangle extent. Requirement 16 says: *this task closes
   none of `TASK_UI_PRIM_32`.*

## Scope, measured against `developer.md` § *Scope check*

**Thirteen files and three components — over the file threshold, at the component
threshold**, so this is split into three sub-tasks per
`.ai/protocols/subagents.md` § *Implementation fan-out*. **Sequential**, because
49.2 is the only sub-task that changes an observable answer and 49.3 records what
it measured.

| Sub-task | Files | Components | Acceptable alone because |
|---|---|---|---|
| **49.1 — the datum and every producer that already holds it** | `ui/src/ui_core/src/paint.rs`, `ui/src/ui_core/src/render.rs`, `ui/src/ui_core/src/widgets/label.rs`, `ui/src/ui_core/src/widgets/text_input.rs`, `ui/src/ui_core/src/widgets/button.rs`, `ui/src/ui_core/src/widgets/toast.rs`, `ui/src/ui_core/src/widgets/dialog.rs`, `ui/src/ui_core/src/widgets/keyboard.rs` | 1: the field and the eight call sites in six widgets that set it | **`command_bounds` still answers `None`**, so **nothing observable changes**: the recorded commands gain a number no reader reads. That is the strongest acceptance test available — **AE 0 on all six pages**, plus the fps number, plus every existing placement test still green. |
| **49.2 — the reader** | `ui/src/ui_core/src/widgets/scroll.rs`, `ui/src/ui_core/src/widgets/chart.rs` | 1: the one function whose answer changes, and the one caller whose `None` is correct | `command_bounds` gains a text arm and its doctest; `clip_commands`' behaviour change is pinned by two named tests; the chart's nine doc claims are re-owned. `chart.rs` is here rather than in 49.1 because **its production code does not change at all** — only its reasons do. |
| **49.3 — the record** | `ui/src/ui_demo/src/main.rs`, `doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md` | 1: the record | **No production line in the demo changes**, which is the test-level statement of "this task moves no pixels", and the same fact the `AE 0` states at the pixel level. Three files, one component. |

**49.1 is over `developer.md`'s five-file threshold and is not split, and the
reason is stated rather than waved at.** Three facts, and the second is
decisive.

1. **`paint.rs` and `render.rs` cannot land apart.** `render.rs`'s `text_vertices`
   destructures `DrawCommand::Text` **exhaustively**, so adding the field breaks
   the build until the renderer binds it. That is two files minimum, and it is not
   a choice.
2. **The producers cannot land apart from the field.** A field no producer
   sets is `developer.md` § *Phase 2*'s *"No dead code"* — and it would also mean a
   commit whose field is unreadable and unset, which is *a
   field carried by nobody* wearing a new type.
3. **The split that would satisfy the threshold is the wrong one.** Separating 49.1
   into "the field and the renderer" and "the producers" puts `label.rs` and
   `text_input.rs` — **the two files whose module docs are the *subject* of the
   claim being closed** — in a different sub-task from the field, so `git log
   --stat` would show the claim corrected in one commit and the thing that
   corrects it added in another. **One sub-task, one commit, eight files, and the
   file count is reported as over the threshold in the handoff rather than
   argued away.**

**`list.rs` is out of scope entirely**, and the reason is worth stating because a
reader would expect it in: its two `Painter::text` calls are inside
`#[cfg(test)]`, they compile unchanged under the additive design, and the
`command_bounds(&moved[0])` assertion in `an_empty_path_is_kept_rather_than_dropped`
is about a **`Path`**, not a `Text`. `list.rs`'s two remaining `carries no …`
claims are about **scissor state** and are task 45's.

**`widgets/image.rs`, `input.rs`, `node.rs`, `layout.rs`, `batch.rs`,
`render/target.rs`, `render/context.rs`, `render/blur.rs` and `theme.rs` are out.**
If the implementer finds themselves editing one of them, that is a **stop
condition** (`developer.md` § *Stop conditions*).

### The six gallery pages must be pixel-identical, and the mechanism is three facts

**Stated here rather than hoped for at review**, and it is checkable rather than
asserted.

1. **No demo production line changes.** `git diff ui/src/ui_demo/src/main.rs`
   shows **`#[cfg(test)]` blocks and doc comments, and nothing else** — no
   constant, no placement, no page, no paint call. The demo reaches the width
   through `Label::paint` and `Chart::draw_labels`, and the first of those changes
   in `ui_core` while the second does not change at all. **This is the strongest single mechanism available and it is
   verifiable with `git diff` alone.**
2. **The only function whose *answer* changes is `command_bounds`, and no command
   in the shipped frame passes through its one production consumer.** `List::paint`
   is the sole caller of `clip_commands` (`grep -rn 'clip_commands(' ui/src`
   returns one call outside `scroll.rs` and its own tests), and
   `grep -n 'List::new' ui/src/ui_demo/src/main.rs` returns **nothing**. So the
   newly-droppable case — a measured text run wholly outside a viewport — has no
   instance in any frame the demo draws.
3. **The width is data on the GPU path, not behaviour.** `render.rs`'s
   `text_vertices` does not read it; the pen still advances from each glyph's own
   advance. **No GL call, no uniform, no vertex computation and no blend state
   moves**, which is checkable by `git diff --stat` on `render.rs`.

Plus the rect-level half keeps its name and every one of its assertions —
`every_page_places_every_rect_where_the_gallery_placed_it`,
`no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`,
`inked_box_bounds_a_shadow_by_the_blur_modules_own_reach`,
`the_highest_ink_on_any_page_is_the_card_of_pads_at_the_tab_bar` — and the
capture criterion is **AE 0 outside `y ≥ 680`** on all six pages, every differing
pixel inside the fps readout's band, per
`doc/ui/IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided, and what it
found*.

## Requirements

### Sub-task 49.1 — the datum and its producers

1. **A ninth field on `DrawCommand::Text`, appended after `weight`, in
   `ui/src/ui_core/src/paint.rs`:**

   ```rust
   /// How far the pen travels across this run, in pixels, **when the caller
   /// measured it**; `None` when nobody did.
   ///
   /// **The pen's final `x` minus this run's `x`**, which is
   /// [`Self::extra_advance`] and everything after it included: the last
   /// glyph's advance is part of the run's travel, and `extra_advance`'s own
   /// doc says it is added after *every* glyph including the last. So
   /// `x + width` is where the pen comes to rest, not where the ink stops.
   ///
   /// **A caller-measured number and nothing else.** The renderer does not
   /// read it — the pen advances from each glyph's own advance in
   /// `crate::render::Renderer::draw_text_batch` — and the width is here so
   /// that `crate::widgets::scroll::command_bounds` can bound a run instead
   /// of inventing one, which is what it declines to do for a `Text` not
   /// carrying this today.
   ///
   /// **It must be measured with the chain that drew the run**, for the same
   /// reason `crate::font::FontSet::advance` is the measuring half of the text
   /// path: a run laid out with one chain's advances and drawn with another's
   /// is a line whose words do not land under their glyphs, and a width from
   /// the wrong chain is a box around the wrong picture. Where the crate
   /// cannot know the font — `crate::widgets::chart::Chart` takes no
   /// measurement seam at all — the answer is `None`, **which is a fact and
   /// not a gap**.
   ///
   /// **`None` is not `0.0`, and the difference is load-bearing.** An empty run
   /// measures exactly `0.0`, and a zero-width box around an unmeasured run
   /// would let `crate::widgets::scroll::clip_commands` *drop* a run that today
   /// it always keeps.
   width: Option<f32>,
   ```

   **The field's doc carries five claims and each is load-bearing**: the pen's
   travel including the trailing advance; a caller-measured number the renderer
   does not read; measured with the chain that drew it, citing `FontSet::advance`;
   `None` for `Chart` as a fact; and `None` ≠ `0.0` with the `clip_commands`
   consequence named. **`git diff` on the variant shows one added line and no
   reordered line.**

2. **Three new `Painter` methods, beside their twins, in `ui/src/ui_core/src/paint.rs`:**
   `pub fn text_measured(&mut self, width: f32, x: f32, y: f32, text: &str, color: Color, font_size: f32, extra_advance: f32)`,
   `pub fn text_in_measured(&mut self, family: FamilyId, width: f32, x: f32, y: f32, text: &str, color: Color, font_size: f32, extra_advance: f32)`, and
   `pub fn text_bold_measured(&mut self, width: f32, x: f32, y: f32, text: &str, color: Color, font_size: f32, extra_advance: f32)` —
   each `#[allow(clippy::too_many_arguments)]`, each documented as *"every
   argument as [`Painter::text_measured`], one family / one weight different"*,
   and each routing to `text_in_weight`.

   - **`text_measured` carries the doctest**, in the shape of `Painter::text_in`'s:
     a `FontSet::new`, two runs of `"Settings"` at different sizes through
     `text_measured`, an `assert_eq!` on the two recorded `Some` widths against the
     numbers handed in, **and an `assert_eq!` on a run recorded through the
     unmeasured `Painter::text` being `None`** — so the doctest is the pair, and
     the additive design's distinction is what a reader learns from it.
   - **`text_measured`'s doc says three things** the requirement's own prose says:
     `width` is the pen's travel; **this method does not check it**, so a
     non-finite width is recorded as handed in and `command_bounds` declines to
     bound the run — **one** place answers a bad width and not six; and the
     unmeasured twin is `Painter::text`, whose doc gains one clause naming
     `text_measured` as what a caller that has measured the run should use.
   - **`Painter::text`, `Painter::text_in` and `Painter::text_bold` keep their
     signatures byte-for-byte** and gain one sentence each, so an existing caller
     compiles and a new reader knows the alternative exists.

3. **`Painter::text_in_weight` gains `width: Option<f32>` as its second
   parameter** and records it. **It stays `#[allow(clippy::too_many_arguments)]`
   and stays the only place a `Text` is pushed.** Its doc's first line is extended
   from *"so neither the weight nor the family can be a field some of the painters
   forget"* to *"so neither the weight, nor the family, nor the width can be a
   field some of the painters forget"*, and requirement 14.4 fixes its argument
   count. **The unmeasured three pass `None`; the measured three pass `Some`.**

4. **`ui/src/ui_core/src/render.rs`, two sites, and no GL call.**
   - **`text_vertices` binds the field and does not use it**, keeping its
     **exhaustive** destructuring: the pattern gains `width: _` and nothing else.
     A comment beside it states why the pass does not read it — *"the pen advances
     from each glyph's own `advance`, and a second source of pen positions in this
     function is a second thing to keep in step with the glyphs"* — and the
     comment names `scroll::command_bounds` as the reader the field is for.
     **Adding `..` instead of `width: _` is forbidden**: this destructuring is the
     crate's declaration of which fields the text pass depends on.
   - **`text_and_image_commands_expand_to_no_quads`'s literal `DrawCommand::Text`
     gains `width: None`**, and nothing else about that test changes.

5. **The eight call sites in six widgets adopt the measured methods, each passing
   the number it already computed, and each doc comment naming what the number
   is.**

   | File | Symbol | What it passes |
   |---|---|---|
   | `label.rs` | `Label::paint` | **`line.width`** in both branches — the justified word-by-word branch passes **`measure(word, options.letter_spacing, advance)` per word**, not the line's width, and requirement 11's `a_justified_line_records_one_run_per_word_each_at_its_own_width` is the test that catches the difference |
   | `button.rs` | `Button::paint` | **`line.width`** |
   | `toast.rs` | `Toast::paint` | **`line.width`** (its `Line` is `label::Line`) |
   | `dialog.rs` | `Dialog::paint_lines` | **`line.width`**, in both the `Regular` and the `Bold` branch; **its comment *"this function measures nothing"* is corrected** to *"this function computes no advance — the width it records is `layout_text`'s own, measured by the caller this widget was handed"* |
   | `keyboard.rs` | `Keyboard::paint` | **`text_width`**, the same `advance * count` it centred on; `Keyboard::advance`'s doc gains one sentence that the property's number is now also the recorded width |
   | `text_input.rs` | `TextInput::paint` | **the sum `visible_run` already accumulates**, which requirement 6 changes it to return |

   **`Chart::draw_labels` is not in this table and does not change** — requirement 7.

6. **`visible_run` returns its width, in `ui/src/ui_core/src/widgets/text_input.rs`.**
   Its signature becomes `-> (String, f32, f32)` — the kept run, the offset of the
   first kept character, and **the sum of the advances of exactly the characters
   kept**. The body gains one accumulator and one `return`; **the walk, the
   `from`/`to` comparisons and the keep/drop rule are byte-identical**, so
   `only_the_characters_that_fit_inside_the_field_are_drawn` and every other
   `TextInput` test keep their names and their assertions. `TextInput::paint`
   passes the third element to `text_measured`.

   **The doc says why the new field does not shorten the walk**, because a reader
   will try: *"the run's width is a **consequence** of this walk and `to` is the
   inner box's right edge, so using the recorded width to choose the characters
   would be the width deciding the width."*

7. **`Chart` records no width, and its reasons are re-owned rather than deleted.**
   `Chart::draw_labels` keeps both `painter.text` calls **byte-for-byte**, and its
   doc's first sentence is **replaced** with: *"**Placed, never measured** — and
   that is now this widget's own limit rather than the command's: a
   [`DrawCommand::Text`] can carry a width, and `Chart::paint` takes no
   measurement seam to produce one with, so its labels are recorded with none."*
   **The same re-owning is applied to every other site that gives the old reason,
   and there are eight more of them: five production doc comments** — the module
   docs' § *Labels are placed, never measured*, `Y_LABEL_GUTTER`,
   `Chart::stroke_reach`'s *an x label's right-hand end* bullet,
   `Chart::paint`'s reachability table and `series_label`
   — **and three comments inside `#[cfg(test)]`.** Each keeps its conclusion and
   changes only the owner of the reason. **Not one of the nine is deleted**,
   because in every one of them the conclusion is still true of the chart, and a
   deletion would have removed a true sentence about `Chart` along with a false
   sentence about `DrawCommand::Text`.

8. **`scroll::command_bounds` gains a text arm, and the arm is where the cost is
   bounded.** In `ui/src/ui_core/src/widgets/scroll.rs`:

   ```rust
   // A text run's box is its **line box**: `x` to `x + width`, and `font_size`
   // tall, because `DrawCommand::Text`'s own `y` doc says `y` is the *top of
   // the line's box* and not the baseline. The width is the pen's travel as
   // the caller measured it, `extra_advance` and the last glyph's advance
   // included, so `x + width` is where the pen came to rest.
   //
   // **`None` and a non-finite coordinate are the same answer for the same
   // reason**: a bound invented from a number nobody measured is worse than no
   // bound, because `clip_commands` *drops* what a bound says is outside — and
   // `intersects` is four comparisons, every one of which a `NaN` fails, so a
   // `NaN` width would **drop** the run rather than keep it. One filter, one
   // place.
   //
   // A negative width is zero: a measured run of no extent is a real answer
   // and not an absence, and `Rect::new` clamps nothing. A negative
   // `font_size` is left alone for the same reason — `command_bounds` reports
   // what the command says and does not correct it.
   DrawCommand::Text {
       x,
       y,
       font_size,
       width,
       ..
   } => width
       .copied()
       .filter(|w| w.is_finite() && font_size.is_finite())
       .map(|w| Rect::new(*x, *y, w.max(0.0), *font_size)),
   ```

   - **`x` and `y` are not filtered.** They are the caller's and were already the
     renderer's problem; this task's arm reads them and does not second-guess
     them. **The filter covers exactly the two numbers this task's arm newly forms
     a box from**, and the doc says so.
   - **`command_bounds`'s doc is rewritten** so the `None` paragraph names **three**
     cases rather than one — an empty `Path`, a `Polygon` of fewer than three
     points, **and a `Text` recorded with no width** — and states that the third is
     `Chart`'s case by name. Its doctest gains one text assertion with
     `command_bounds(...) == Some(Rect::new(...))`.
   - **The doctest's text example is placed off the origin** —
     `Rect::new(41.0, 17.0, 90.0, 18.0)` from `x = 41.0`, `y = 17.0`,
     `font_size = 18.0`, `width = Some(90.0)` — because
     *a rect's origin and a rect's extent are different
     numbers* — a geometry fixture at the origin cannot see an origin
     being read as an extent.

9. **`clip_commands` keeps its three outcomes and its straddler rule, and one of
   its sentences is replaced.** Dropping a wholly-outside command is now reachable
   for a `Text`; the straddler is still kept **whole**, and its reason gains the
   new one: *"and a measured [`DrawCommand::Text`] is **not** trimmed either —
   cutting a run's string would leave a different string, not a clipped one, and
   the scissor cuts the pixels."* The doc's sentence *"A command
   [`command_bounds`] cannot bound is kept, so a text run on screen is never
   dropped for want of a measurement this layer has no way to take"* is **kept and
   made narrower**: *"…for want of a measurement this layer has no way to take —
   which is now `Chart`'s labels and no one else's."*

10. **The text pass is untouched, and this requirement's diff is the check.** No GL
    call, uniform, vertex computation, blend state or shader source moves.
    `git diff ui/src/ui_core/src/render.rs` shows `width: _` in `text_vertices`'s
    exhaustive pattern, one `width: None` in a test fixture, and nothing else.

11. **The tests for 49.1**, named, with no display, no network, no filesystem and
    no wall clock — the only kind `AGENTS.md` permits:

    In `ui/src/ui_core/src/paint.rs`:
    - **`a_measured_painter_records_the_width_its_caller_hands_in`** — the three
      `*_measured` methods record `Some(w)` for a caller-supplied number and the
      three unmeasured methods record `None`, for **the same string at the same
      size**. **The mutation:** `text_in_weight` dropping the parameter, or
      `text_in_measured` routing to `FontWeight::Regular`'s twin — both make a
      measured run read `None`.
    - **`an_unmeasured_run_and_a_measured_one_of_the_same_string_differ_in_the
      _width_and_in_nothing_else`** — the deliberate mirror of
      `text_bold_differs_from_text_in_the_weight_and_in_nothing_else`, and the
      reason requirement 14.5 exists. **Its mutation:** the width changing
      something else on the way in, which the existing test cannot see once its
      pattern gains a binding.
    - **`a_non_finite_width_is_recorded_rather_than_corrected`** — `NaN` and
      `inf` reach the command unchanged. **Its mutation:** the painter clamping or
      dropping a bad width, which would put a second answer beside
      `command_bounds`'s filter and make the two disagree.

    In `ui/src/ui_core/src/widgets/label.rs`:
    - **`a_label_records_the_width_of_the_line_it_paints`** — **the required
      "width equals the sum of advances for a known string, without a font file at
      a path" test.** `mono` (`fn mono(_: char) -> f32 { 5.0 }`, the crate's own
      stand-in, already in this file) at `letter_spacing = 0.0` and again at
      `1.5`; for each recorded run, `width == Some(5.0 * text.chars().count() +
      spacing * text.chars().count())`. **The expected number is arithmetic over
      the test's own input** — see § *Testing the width without a font file*.
    - **`a_justified_line_records_one_run_per_word_each_at_its_own_width`** — the
      word-by-word branch, where the line's width is the wrong number. **Its
      mutation:** passing `line.width` to every word's run, which is a box three
      times too wide per word and is invisible on screen.

    In `ui/src/ui_core/src/widgets/text_input.rs`:
    - **`the_field_records_the_width_of_exactly_the_characters_it_drew`** — with
      `mono`, the recorded `width` equals the advances of exactly the characters
      the existing test `only_the_characters_that_fit_inside_the_field_are_drawn`
      names. **Its mutation:** the whole string's width on a run of seventeen
      characters, or the run's width on the text it does not draw.

    In `ui/src/ui_core/src/widgets/button.rs`, `toast.rs`, `dialog.rs`, `keyboard.rs`:
    - **`the_button_records_the_width_of_the_run_it_paints`**,
      **`the_toast_records_the_width_of_each_line_it_paints`**,
      **`the_dialog_records_the_width_of_every_line_it_paints_in_both_weights`**,
      **`a_key_records_the_advance_width_it_centred_on`** — one named test each,
      because a one-line change whose whole content is *"pass the number you
      already computed"* cannot otherwise be reviewed for **picking the right
      number**, and the dialog's is the only one with a two-branch body.

    In `ui/src/ui_core/src/widgets/chart.rs`:
    - **`a_chart_label_records_no_width_because_the_chart_cannot_measure_one`** —
      **the guard that keeps the chart's honest `None` from becoming an
      oversight.** **Its mutation:** `draw_labels` gaining a `text_measured` call
      with a number it did not measure, which would be the one edit in this task
      that puts a false box on screen.

### Sub-task 49.2 — the reader and its tests

12. **`scroll.rs`'s tests, named**, and the **one rename**:

    - **`command_bounds_answers_for_a_text_run_that_carries_its_width`** — **the
      required test.** A `Text` at `x = 41.0`, `y = 17.0`, `font_size = 18.0`,
      `width = Some(90.0)` bounds to `Rect::new(41.0, 17.0, 90.0, 18.0)`, and
      **`width = None` on the same command with every other field identical bounds
      to `None`**. **The mutation:** the arm returning `Some` for the `None` case,
      which is the whole falsity this task closes.
    - **`a_text_run_is_bounded_by_its_line_box_and_not_by_its_baseline`** — the
      bound's top is `y`, **not** `y − ascent` and **not** `y + font_size`; and its
      height is `font_size`, **not** `font_size × 1.2`. **Off the origin** — a
      fixture at the origin cannot see an origin read as an extent. **The mutation:**
      a bound built from the baseline rather
      than the line box, which is a box shifted up by the ascent and drops a run
      that starts inside a clip.
    - **`a_non_finite_or_negative_width_leaves_a_text_run_unbounded`** — `NaN`,
      `f32::INFINITY`, `f32::NEG_INFINITY` and `-4.0` over the same fixture.
      **`NEG_INFINITY` and `NaN` bound to `None`; `-4.0` bounds to a zero-width
      rect at `x`.** **Its mutation:** `filter(|w| w.is_finite())` dropped, which
      makes `clip_commands` **drop** a run whose width is `NaN` — the regression
      the filter exists to prevent, and the one the `intersects` arithmetic in the
      facts table predicts.
    - **`a_measured_text_run_outside_the_clip_is_dropped_and_an_unmeasured_one_is
      _kept`** — two commands, identical but for the width, against a viewport that
      contains neither: the measured one is dropped and the unmeasured one is
      returned whole. **Its mutation:** `clip_commands`'s `is_none_or` inverted,
      which would drop every unmeasurable command in the crate.
    - **`an_unmeasured_text_run_is_kept_because_the_command_carries_no_width`** —
      **the rename of `a_text_run_is_kept_because_a_draw_command_does_not_carry_its
      _width`.** Its **fixture gains `width: None`** (it constructs the variant
      literally, so it does not compile otherwise), its `None` assertion **and its
      `clip_commands` half are kept unchanged**, and its assertion message
      *"there is no width in the command to bound it with"* becomes *"the run
      carries no width, so there is nothing to bound it with"* — **which is still
      true of this fixture and is the point of keeping it.** **It is renamed, not
      deleted, and no assertion of it is weakened**; the `y = 5000.0` placement is
      kept so the straddler-and-drop halves stay off the origin.

    In `ui/src/ui_core/src/widgets/chart.rs`, requirement 11's chart test.

### Sub-task 49.3 — the record

13. **`ui/src/ui_demo/src/main.rs` gains one test and changes no production line.**

    - **`no_recorded_text_run_would_be_dropped_where_it_drew`** — **the pre-flight,
      and it is this task's version of task 45's.** All six `Page::ALL` pages, plus
      **`Page::Overlays` with the dialog open and a toast raised**; every node in
      `Demo::order`; every `Text` command of its `PaintState` **that
      `command_bounds` can now bound**, asserted inside that node's
      `LayoutState::clip`, with `command_bounds`'s own `None` treated as its doc
      says. **Mutations killed:** any new bound that cuts a run the demo draws —
      a width that forgot the last glyph's advance, a box built from the baseline,
      or a `clip_commands` predicate that dropped a straddler. **And the criterion
      is one number a reviewer can check:** the handoff pastes the count of
      bounded runs per page, and **a non-zero count outside a node's clip is a
      stop condition**, because the fix would be in the producer's arithmetic and
      never in `command_bounds`.
    - **It also strengthens `TASK_UI_PRIM_45`'s
      `no_recorded_command_is_cut_by_its_own_nodes_clip`**, which walks the same
      nodes and answers only where `command_bounds` does — **so after this task
      that test covers every text run on every page, and this one is the narrower
      text-specific statement of the same fact. Which of the two owns the
      assertion is recorded here rather than left to two tests claiming it**, and
      if 45.1 has landed its assertion **stays**: the two are not the same
      question, and deleting one of them would delete a mutation the other cannot
      see.

14. **The falsities are corrected, by symbol, and no correction is a deletion
    where the conclusion still holds.**

    - **14.1 — `ui/src/ui_core/src/widgets/text_input.rs`.** Three sites. The
      module docs' § *Clipping, and why the widget has to do it itself* has its
      six-field enumeration **replaced by the nine fields' names**, and its
      conclusion **replaced** by the present tense: a run that overflows is drawn
      whole, the scissor is what stops it, **and the run now carries the width of
      exactly the characters `visible_run` kept** — which is why that function is a
      walk and not a measurement. `TextInput::paint`'s doc's *"because the run
      carries no width and therefore cannot be trimmed"* becomes *"because a run's
      recorded string is what is drawn and a shorter string is not this run"*.
      `visible_run`'s doc keeps its walk and loses its reason. **Its module doc
      also says plainly that the field is not used by the selection or the caret,
      and why a run's width cannot shorten its own walk.**
    - **14.2 — `ui/src/ui_core/src/widgets/scroll.rs`.** `command_bounds`'s doc and
      `clip_commands`' three-outcome list, per requirements 8 and 9. **The module
      docs' § *The second decision is clipping* keeps** *"[`DrawCommand`] has **no
      scissor state of its own**"* **and its reason untouched, because that
      paragraph makes no field enumeration and this task moves no scissor claim** —
      the conclusion in it is task 45's, and 45 keeps it. **And if `TASK_UI_PRIM_45`
      has landed first, one clause is this task's rather than 45's:** 45's own
      requirement 8.4 rewrites that paragraph's reason as *"[`DrawCommand`] … it
      carries one `x`, one `y` and a string, and no width"*, **and that
      enumeration is five fields short on the day it is written**, so this task
      corrects it to the nine fields' names. **Which task owns that clause depends
      on which landed first, and the handoff says which — `.ai/protocols/subagents.md`
      § *Splitting* rule 1 applies to the implementation and `git log --stat` is
      the check.**
    - **14.3 — `ui/src/ui_core/src/widgets/chart.rs`.** The nine sites, per
      requirement 7: every conclusion kept, every reason re-owned.
    - **14.4 — `ui/src/ui_core/src/paint.rs`'s argument-count arithmetic.**
      `Painter::text_in`'s *"The same **eight** arguments [`Painter::text`] takes,
      plus the family"* and `Painter::text_in_weight`'s *"**Nine** arguments is the
      command's **eight** fields plus the weight"* are both **corrected to the
      counts the code has** — six, seven and nine respectively, with the ninth being
      the command's **eight** fields plus `width`. **This is not a drive-by:** a
      reader reconciling `text_in_weight`'s signature against its comment is doing
      exactly what this task makes necessary, and leaving two wrong counts there
      would be the defect `doc/ui/DEMO_APPLICATION.md`
      § *Corrections to the second gap table* records twice.
    - **14.5 — `ui/src/ui_core/src/paint.rs`'s
      `text_bold_differs_from_text_in_the_weight_and_in_nothing_else`.** Its two
      exhaustive patterns gain **`width: regular_width`** and
      **`width: bold_width`**, **not `..`** — because the test's own comment says
      *"If a second field ever rides along with the weight, this is what catches
      it"*, and a `..` would end the catching. It then asserts
      `regular_width == bold_width`, **which is the truth** (`text` and
      `text_bold` both record `None`) **and is the assertion that fails if a future
      field is ever recorded differently by the two twins.**

15. **`doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*, row
    `L8`, gains a dated note** and is **not deleted, not marked closed**. The note
    records: that **`TASK_UI_PRIM_49` added `DrawCommand::Text`'s ninth field,
    `width: Option<f32>`, and `scroll::command_bounds` gained a text arm**; that
    **the row's own field count is corrected from seven to eight, and the command
    now has nine** — *the row corrected six to seven and was overtaken by the
    eighth, which is the shape `doc/ui/DEMO_APPLICATION.md`
    § *Corrections to the second gap table* exists to record*; that **the width is
    a caller-measured pen travel and `None` is `Chart`'s honest answer**; that
    **the row's two `Blocks` entries — card content that overflows, the carousel's
    page edges — are not delivered**, because there is no card and no pager in this
    repository; and **that the fade ramp remains `TASK_UI_PRIM_32`'s, with this
    task supplying only the number its requirement 2 reads.**

16. **`doc/ui/IMPLEMENTATION_STATE.md` gains one dated entry** carrying: the field
    and the three methods, and the additive-not-breaking decision in three
    sentences; **the headline — that eight of the ten production call sites already
    computed the number and discarded it, and `AdvanceCache` is untouched**; the
    `Option<f32>` decision and the `clip_commands`-drop reason for it; the
    `font_size`-is-the-height limit; the pre-flight's measured count per page;
    the fps numbers; **and the honest limits, in that section's own register** —
    *"no card and no pager exist, so nothing in this repository consumes the width
    for the product reason row `L8` gives; the consumers are
    `scroll::command_bounds`, `TASK_UI_PRIM_45`'s clip pre-flight and
    `TASK_UI_PRIM_32`'s ramp requirement 2, which needs the cut edge and has three
    decisions of its own left to take. `Truncation::Fade` still has no ramp and
    this task closed none of task 32."*
    `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`.ai/workflows/task-sequence.md` § *State*); it points at the code.

17. **The suite, the capture and the frame rate are all produced.** From `ui/`:
    `cargo fmt --check`, `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings`,
    `cargo test --all-features` with the per-binary counts pasted and **no test
    deleted, renamed away or weakened** beyond requirement 12's single named
    rename, `cargo doc --no-deps` clean, and `cargo audit` **recorded as not
    installed on this host, not passed**. Then the six-page before/after capture
    with the commands of `IMPLEMENTATION_STATE.md`
    § *Verifying a change that draws — the capture method* verbatim, and then
    **the frame rate on all six pages**.

## Testing the width without a font file at a path

**The mechanism, named, because `AGENTS.md` forbids a test that needs a filesystem
and a font is a file.**

1. **The crate already has the stand-in, and this task uses it rather than
   inventing one.** `ui/src/ui_core/src/widgets/label.rs`'s test module opens with
   `fn mono(_: char) -> f32 { 5.0 }` — *"A monospace advance of 5 pixels per
   character, spaces included"* — and **every** layout assertion in that file is
   driven through it. `ui/src/ui_demo/src/main.rs`'s `TextMetrics::sharing` doc
   records the same rule from the other side: *"The tests build the closures
   directly instead, from a monospace stand-in, because a set cannot be given a
   font without a font file and a test may not open one."*
2. **The expected number is arithmetic over the test's own input.** For a string
   of `n` characters at `letter_spacing = s`, the sum of the advances is
   `5.0 * n + s * n` — computed in the test from `text.chars().count()` and the
   `LayoutOptions` the test itself built. **No font, no path, no FreeType, no
   filesystem and no display.**
3. **What that proves, and what it does not, stated here rather than discovered at
   review.** It proves that **`Label::paint` hands `layout_text`'s own
   `line.width` to the command, unchanged** — which is the plumbing this task is,
   and the only thing this task changed. **It does not prove that a font's
   advances are correct.** That needs a font file, it was already untested before
   this task, and `doc/ui/IMPLEMENTATION_STATE.md` § *Task 30 — what it decided,
   and what it found* records the same limit for `has_glyph`: *"one FreeType call
   that needs a real font file, which `AGENTS.md` forbids a test to open"*. **So
   no acceptance criterion here claims anything about a typeface's metrics, and the
   handoff says so.**

**The task-30 fixture rule, applied as a rule and not as a reminder.**
`doc/ui/IMPLEMENTATION_STATE.md` § *Task 30* records that the demo's test fixture
**called `define_family` for a family `main` never defined**, so every test saw two
families and passed while the shipped binary resolved the unknown name to the
default — *"a test fixture that builds what
production does not define"*. **Therefore, in this task:**

- **No test defines a family, calls `FontSet::family("…")`, or asserts anything
  about which family a run was recorded in.** Every width assertion is made in
  `FamilyId::default()` and is **family-agnostic**: it compares the recorded width
  against a number the same test computed from the same `advance` closure.
- **No demo test touches a font, a family or an advance at all.** The pre-flight
  walks recorded commands and asks `command_bounds` — it never asks for a
  measurement.
- **`Keyboard`'s test is the one place a stand-in number is the production
  number**, and that is stated as the invariant rather than as a fixture: a key's
  recorded width is the advance it centred on, **which is what the box in
  `command_bounds` must agree with**, and a caller that installs a real advance
  gets a real width with no code change. **That is the fixture rule's positive
  form — the stand-in is the production value here, not a substitute for it.**

## Acceptance Criteria

- [ ] **`DrawCommand::Text` has nine fields and the ninth is `width: Option<f32>`.**
      `awk '/^pub struct DrawCommand/,/^}/'` is not the instrument;
      `grep -n -A 60 'Text {' ui/src/ui_core/src/paint.rs` shows the nine fields
      **in this order** — `x`, `y`, `text`, `color`, `font_size`, `extra_advance`,
      `family`, `weight`, `width` — and `git diff` on the variant shows **one added
      line and no reordered line**. The field's doc carries all five claims from
      requirement 1, including **`None` is not `0.0` with the `clip_commands`
      consequence named**

- [ ] **The three existing signatures are untouched and three new methods exist.**
      `git diff ui/src/ui_core/src/paint.rs` shows **no change to the parameter
      lists of `Painter::text`, `Painter::text_in` or `Painter::text_bold`** — the
      additive decision, and the reason no caller outside this task's eight call
      sites in six widgets had to be edited. `grep -n 'pub fn text_measured\|pub fn text_in_measured\|pub fn text_bold_measured' ui/src/ui_core/src/paint.rs`
      returns **three** lines, each `#[allow(clippy::too_many_arguments)]`, each
      routing to `text_in_weight`, and `text_measured` carries the doctest from
      requirement 2 with **both** halves of the pair asserted. **The handoff quotes
      the `git diff` of the three old signatures as proof that a `pub` API was not
      broken** — `developer.md` § *API design*

- [ ] **`Painter::text_in_weight` remains the only place a `Text` is pushed.**
      `grep -n 'DrawCommand::Text {' ui/src/ui_core/src/paint.rs` returns **one**
      production line — the `self.commands.push` inside `text_in_weight` — and
      `grep -c 'self.commands.push(DrawCommand::Text' ui/src/ui_core/src/paint.rs`
      is **1**. Its doc's first line names **the weight, the family and the
      width**. **A second place to push a text command is the mutation this
      criterion exists to kill**, and it is the failure the function's own doc was
      written to prevent

- [ ] **`command_bounds` returns `Some` for a `Text` that carries its width, and
      `None` for the same run without one.**
      **`command_bounds_answers_for_a_text_run_that_carries_its_width`** asserts
      `width = Some(90.0)` at `x = 41.0`, `y = 17.0`, `font_size = 18.0` gives
      `Rect::new(41.0, 17.0, 90.0, 18.0)`, and that `width = None` with every
      other field identical gives `None`. `grep -n 'DrawCommand::Text' ui/src/ui_core/src/widgets/scroll.rs`
      shows the new arm with its `is_finite` filter. **Mutation evidence in the
      handoff:** make the arm ignore `width` and return `None` unconditionally, and
      watch the suite fail; restore it and watch it pass.
      **The fixture is off the origin** — `x = 41.0`, not `0.0` — because
      *a rect's origin and a rect's extent are different
      numbers* — a geometry fixture at the origin cannot see an origin
      read as an extent

- [ ] **The bound is the line box, not the baseline, and its height is
      `font_size`.**
      `a_text_run_is_bounded_by_its_line_box_and_not_by_its_baseline` asserts the
      top is `y` and the height is exactly `font_size`. **Mutation evidence:** build
      the rect from `y − font_size` and watch it fail; then build the height as
      `font_size * 1.2` and watch it fail. **A box shifted up by the ascent drops a
      run that starts inside its own clip**, which is the failure the arm's `y` doc
      exists to prevent. **The honest limit is in the arm's doc:** `font_size` is
      the em box and not `FontSet::line_height`, so the bound can be a little
      shorter than the ink for a face with a large line gap — **this task adds no
      tenth field for it** (§ *Out of Scope*)

- [ ] **A non-finite width leaves the run unbounded rather than dropping it.**
      `a_non_finite_or_negative_width_leaves_a_text_run_unbounded` asserts `NaN`,
      `f32::INFINITY` and `f32::NEG_INFINITY` all bound to `None`, and `-4.0` bounds
      to a zero-width rect at `x`. **Mutation evidence:** delete
      `.filter(|w| w.is_finite() && font_size.is_finite())` and watch
      `a_measured_text_run_outside_the_clip_is_dropped_and_an_unmeasured_one_is_kept`
      fail for a `NaN` fixture — **because `intersects` is four comparisons and
      every one of them a `NaN` fails, so an unfiltered `NaN` width makes
      `clip_commands` drop the run.** That regression is the reason the filter
      exists and it is the one mutation this criterion insists on being seen to
      fail

- [ ] **`clip_commands` drops a measured run that is wholly outside and keeps an
      unmeasured one.**
      `a_measured_text_run_outside_the_clip_is_dropped_and_an_unmeasured_one_is_kept`
      runs both halves against one viewport. **`clip_commands`' straddler rule is
      unchanged** — its three-outcome list, its doctest and every existing test of
      it keep their names and their assertions, and the only sentence that changes
      is the one naming a `Text`'s missing width. **And its doc's promise is
      narrowed rather than dropped:** *"a text run on screen is never dropped for
      want of a measurement this layer has no way to take — which is now `Chart`'s
      labels and no one else's"*

- [ ] **The one renamed test is renamed, and no test was deleted or weakened.**
      `a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width` is
      **`an_unmeasured_text_run_is_kept_because_the_command_carries_no_width`**, its
      fixture gains `width: None` (it constructs the variant literally), **its
      `None` assertion and its `clip_commands` half are kept unchanged**, its
      assertion message becomes *"the run carries no width, so there is nothing to
      bound it with"*, and its `y = 5000.0` placement is kept. **`grep -n
      'a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width' ui/src/`
      returns nothing**, and the new name returns one line. **The handoff states
      this as the single deletion-of-a-name in the task and gives the reason**
      (`developer.md` § Phase 3: a test whose name is the claim this task closes
      cannot keep its name)

- [ ] **The recorded width is the sum of advances for a known string, with no font
      file at a path, and the mechanism is stated.**
      `a_label_records_the_width_of_the_line_it_paints` drives `Label::paint`
      through `label.rs`'s own `fn mono(_: char) -> f32 { 5.0 }` and asserts each
      recorded `width` equals **`5.0 * text.chars().count() +
      letter_spacing * text.chars().count()`**, at `letter_spacing` `0.0` and again
      at `1.5`. **The expected number is arithmetic over the test's own input** — no
      font, no path, no FreeType, no filesystem, no display — and the
      non-zero-tracking half is what fails a width that forgot the last glyph's
      advance, which requirement 1's field doc says is included.
      `the_field_records_the_width_of_exactly_the_characters_it_drew` is the same
      mechanism in `TextInput`. **The handoff states what this does not prove: that
      a font's advances are correct.** That needs a font file, `AGENTS.md` forbids a
      test to open one, and `doc/ui/IMPLEMENTATION_STATE.md` § *Task 30* records the
      identical limit for `has_glyph`. **No criterion in this file claims anything
      about a typeface's metrics**

- [ ] **No test depends on a definition production is supposed to make.**
      `grep -rn 'define_family\|FontSet::family(' ui/src/ui_core/src/widgets/label.rs ui/src/ui_core/src/widgets/text_input.rs ui/src/ui_core/src/widgets/scroll.rs ui/src/ui_core/src/widgets/button.rs ui/src/ui_core/src/widgets/toast.rs ui/src/ui_core/src/widgets/dialog.rs ui/src/ui_core/src/widgets/keyboard.rs ui/src/ui_core/src/widgets/chart.rs`
      returns **nothing new** in any `#[cfg(test)]` module this task touched, and
      **every width assertion is made in `FamilyId::default()`** and compares the
      recorded width against a number the same test computed from the same closure.
      **The rule is *a test fixture that builds
      what production does not define***, and the defect it records — a fixture
      defining a family `main` did not, so every test passed while the shipped
      binary did nothing — is the reason. **`keyboard.rs`'s stand-in is the positive
      form and is stated as such:** a key's recorded width is the advance it
      centred on, and a caller that installs a real advance gets a real width with
      no code change

- [ ] **The one honest `None` is the chart's, and a test says why.**
      `a_chart_label_records_no_width_because_the_chart_cannot_measure_one` asserts
      `Chart::draw_labels`' recorded `Text` commands carry `width == None`.
      **Mutation evidence:** give `draw_labels` a `text_measured` call with a
      number it did not measure and watch that test fail; restore and watch it
      pass. **This is the guard that stops the `None` from becoming an oversight**,
      and it is why the field is an `Option` rather than a `f32`

- [ ] **The existing exhaustive destructurings keep catching, and the argument
      counts are right.**
      `text_bold_differs_from_text_in_the_weight_and_in_nothing_else`'s two patterns
      in `ui/src/ui_core/src/paint.rs` bind **`width: regular_width`** and
      **`width: bold_width`** and **no `..`**, and it asserts they are equal — the
      assertion that fires if a future field is ever recorded differently by
      `Painter::text` and `Painter::text_bold`. `text_vertices` in
      `ui/src/ui_core/src/render.rs` binds **`width: _`** and **no `..`**.
      `grep -n 'The same eight arguments' ui/src/ui_core/src/paint.rs` and
      `grep -n 'Nine arguments is the command' ui/src/ui_core/src/paint.rs` both
      return **nothing**, and the corrected counts — six, seven, nine — are in the
      file. **The reason is in the criterion:** this task adds an argument, so a
      reader reconciling `text_in_weight`'s signature against its comment is doing
      what the task makes necessary, and two wrong counts left there would be the
      defect `doc/ui/DEMO_APPLICATION.md` § *Corrections to the second gap table*
      records twice

- [ ] **The eight call sites in six widgets record the number they already
      computed, and the one that must not have the line's width does not.**
      `a_label_records_the_width_of_the_line_it_paints`,
      `a_justified_line_records_one_run_per_word_each_at_its_own_width`,
      `the_button_records_the_width_of_the_run_it_paints`,
      `the_toast_records_the_width_of_each_line_it_paints`,
      `the_dialog_records_the_width_of_every_line_it_paints_in_both_weights` and
      `a_key_records_the_advance_width_it_centred_on` all pass.
      **The justified-label test is the one a reviewer should break first:**
      `line.width` on every word's run is a box three times too wide per word and is
      **invisible on screen** — which is why it needs a test rather than a capture

- [ ] **The GPU path is untouched, and that is checkable by diff.**
      `git diff ui/src/ui_core/src/render.rs` shows **`width: _`** in `text_vertices`'s
      exhaustive pattern, **`width: None`** in `text_and_image_commands_expand_to_no
      _quads`'s fixture, **and nothing else**. **No GL call, no uniform, no vertex
      computation, no blend state and no shader source moves**; the text fragment
      shader still multiplies `coverage` by `v_color` over a `GL_R8` atlas, and
      `ui/src/ui_core/src/font.rs` is **unchanged** — no atlas change, no outline
      extraction, no distance field, and `AdvanceCache` untouched

- [ ] **The pre-flight ran and its number is pasted.**
      `no_recorded_text_run_would_be_dropped_where_it_drew` passes on all six
      `Page::ALL` pages **and on `Page::Overlays` with the dialog open and a toast
      raised**, and the handoff pastes **the count of newly-bounded runs per page**.
      **A non-zero count outside a node's own `LayoutState::clip` is a stop
      condition**, and the fix is in the producer's arithmetic, never in
      `command_bounds`. The test also states in its own doc that it **strengthens**
      `TASK_UI_PRIM_45`'s `no_recorded_command_is_cut_by_its_own_nodes_clip`, that
      both stay, and **which of the two owns the assertion** — two tests claiming
      one fact is the defect `AGENTS.md` § *Working context* warns about, in code

- [ ] **The six gallery pages are pixel-identical outside the fps band, and the
      mechanism is three facts and not one.** `Page::ALL`'s six names, release
      build, captured **before and after** with the commands of
      `doc/ui/IMPLEMENTATION_STATE.md`
      § *Verifying a change that draws — the capture method* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture, and `ffmpeg x11grab`, return black for a GL window),
      `pgrep -a -x ui_demo` in the same call as each `magick import -window <id>`,
      then `magick compare -metric AE before.png after.png null:` per page.
      **AE 0 outside `y ≥ 680`** on all six, every differing pixel inside the fps
      readout's band, which `IMPLEMENTATION_STATE.md`
      § *Task 24.1 — what it decided, and what it found* records as the one thing
      two captures of an unchanged frame differ in.

      - **1. No demo production line changes.**
        `git diff ui/src/ui_demo/src/main.rs` shows **`#[cfg(test)]` and doc comments
        only** — no constant, no placement, no page, no paint call. **This is the
        strongest single mechanism available and it is verifiable with `git diff`
        alone.**
      - **2. The only function whose *answer* changes is `command_bounds`, and no
        shipped command reaches its one production consumer.** `List::paint` is the
        sole caller of `clip_commands` (`grep -rn 'clip_commands(' ui/src` returns
        one call outside `scroll.rs` and its own tests) and
        `grep -n 'List::new' ui/src/ui_demo/src/main.rs` returns **nothing**, so the
        newly-droppable case — a measured text run wholly outside a viewport — has
        no instance in any frame the demo draws.
      - **3. The width is data on the GPU path, not behaviour.** `text_vertices`
        does not read it; the pen still advances from each glyph's own advance.
      - **The rect-level half keeps every name and every assertion:**
        `every_page_places_every_rect_where_the_gallery_placed_it`,
        `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`,
        `inked_box_bounds_a_shadow_by_the_blur_modules_own_reach` and
        `the_highest_ink_on_any_page_is_the_card_of_pads_at_the_tab_bar` — and
        `ui_demo`'s `inked_box` keeps its zero-width `Text` arm, because it answers
        a **y** question over every node on every page

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: in `paint.rs` —
      `a_measured_painter_records_the_width_its_caller_hands_in`,
      `an_unmeasured_run_and_a_measured_one_of_the_same_string_differ_in_the_width_and_in_nothing_else`,
      `a_non_finite_width_is_recorded_rather_than_corrected`; in `label.rs` —
      `a_label_records_the_width_of_the_line_it_paints`,
      `a_justified_line_records_one_run_per_word_each_at_its_own_width`; in
      `text_input.rs` — `the_field_records_the_width_of_exactly_the_characters_it
      _drew`; in `button.rs`, `toast.rs`, `dialog.rs` and `keyboard.rs` —
      `the_button_records_the_width_of_the_run_it_paints`,
      `the_toast_records_the_width_of_each_line_it_paints`,
      `the_dialog_records_the_width_of_every_line_it_paints_in_both_weights`,
      `a_key_records_the_advance_width_it_centred_on`; in `chart.rs` —
      `a_chart_label_records_no_width_because_the_chart_cannot_measure_one`; in
      `scroll.rs` — `command_bounds_answers_for_a_text_run_that_carries_its_width`,
      `a_text_run_is_bounded_by_its_line_box_and_not_by_its_baseline`,
      `a_non_finite_or_negative_width_leaves_a_text_run_unbounded`,
      `a_measured_text_run_outside_the_clip_is_dropped_and_an_unmeasured_one_is_kept`,
      `an_unmeasured_text_run_is_kept_because_the_command_carries_no_width`
      (**renamed**); in the demo —
      `no_recorded_text_run_would_be_dropped_where_it_drew`.

      **Baseline 1894 (1450 + 224 + 220), and this file projects 1912** —
      **seventeen new tests and one new doctest**, plus **one rename** with no
      assertion weakened — **so a measured number other than 1912 is corrected here
      rather than argued about.**
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host; that
      is **recorded, not passed**

- [ ] **The frame rate is measured on all six pages and reported.**
      `.ai/tools/fps-check.sh 10 55` on the default page with the script's own line
      pasted rather than the rate expected, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of the
      six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `doc/ui/IMPLEMENTATION_STATE.md` § *Current position*
      records as the reason task 24.2's criterion 6 was amended rather than met by
      the script. Every page above the floor of **55**, and **each page is compared
      against its own pre-change number, not against the other pages.**

      **What is expected, and stated as an expectation rather than a promise:**
      **no measurable change**, because no measurement is added anywhere — the
      eight call sites in six widgets already had the number — and the only
      per-frame growth is
      `Option<f32>` per recorded text run and the `Rect` arithmetic `clip_commands`
      now performs for a run it previously skipped. **A regression is reported as a
      regression and not explained away**, because
      *a still screenshot of a 4 fps application looks exactly
      like a 60 fps one* is the reason this criterion exists at all

- [ ] **Row `L8` is amended, dated, and its `Blocks` column is intact.**
      `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* row
      **`L8`** carries a dated note naming `TASK_UI_PRIM_49`, stating that a
      `DrawCommand::Text` now carries its measured width and that
      `scroll::command_bounds` bounds one, **correcting the row's own field count
      from seven to eight and recording that the command now has nine** —
      *the row corrected six to seven and was overtaken by the eighth, which is the
      shape § *Corrections to the second gap table* exists to record* — and stating
      that **neither of the row's two `Blocks` entries is delivered**: there is no
      card and no pager in this repository, and rows `#6` and `#7` are the card and
      the tab bar's open rows. **The row is not deleted, not marked closed, and its
      `Blocks` column still names both** — *card content that overflows, the
      carousel's page edges*. **And no doc comment in any changed file asserts the
      opposite of the code beside it**, which is the defect
      `doc/ui/DEMO_APPLICATION.md` § *Corrections to the second gap table* records
      twice

- [ ] **The fade ramp is untouched, and the relationship to task 32 is written
      down where the next agent finds it.** `git diff` shows **no change** to
      `Truncation`, `truncate_line`, `LayoutOptions::truncation`,
      `Label::paint`'s truncation handling, or any shader source.
      `doc/ui/IMPLEMENTATION_STATE.md` § *Tasks 30–32, the text gaps task 11 left*,
      row **32**, still reads that **there is no fade ramp at all** — **it is not
      edited by this task** — and the new entry says in one sentence that this task
      **supplies the cut edge task 32's requirement 2 reads** (`x + width`) and
      **closes none of task 32**, whose requirement 6 (per-glyph factor or shader
      term) and requirement 4 (per-node clip not leaking, which is task 45's) are
      both still open. **A reader who takes the width for a ramp has read the wrong
      document, and this criterion is what stops them**

- [ ] **Nothing from another task leaked in.** `git diff --stat` shows **no change**
      to `ui/src/ui_core/src/font.rs` (task 30's chain and task 31's atlas), to
      `ui/src/ui_core/src/layout.rs`, `batch.rs`, `input.rs`, `node.rs`, `theme.rs`,
      `render/target.rs`, `render/context.rs`, `render/blur.rs`, or to
      `ui/src/ui_core/src/widgets/image.rs`, `list.rs`, `slider.rs`, `toggle.rs`,
      `gauge.rs`, `chart.rs`'s **production code**, or any `widgets/*.rs` outside
      the ones this task names. **`scroll.rs`'s clipping half keeps its bytes** —
      `Scroll::clip_rect`, its doc and `clip_commands`' straddler rule are task
      45's, and only `command_bounds` and `clip_commands`' text sentence move here,
      plus the module docs' enumeration **if and only if** `TASK_UI_PRIM_45` landed
      first (requirement 14.2, which says which task owns that clause).
      **`ui/Cargo.toml` and `ui/Cargo.lock` are unchanged** — the approved direct
      dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, and per
      `AGENTS.md` a text-measurement crate for a sum of cached advances is a
      licence decision against GPLv3 that nobody has asked for. **Edition 2021 and
      `rust-version = "1.85"` are untouched**, and **no `unsafe` block is added**:
      `git diff` shows no `unsafe`, and this task adds no FFI call, no GL call and
      no `unwrap`/`expect`/`panic!` in production code

## Out of Scope

- **No fade ramp, no alpha gradient, no per-glyph alpha factor, no shader term.**
  `TASK_UI_PRIM_32` owns it. `Truncation::Fade` stays layout-identical to
  `Truncation::Clip`, `truncate_line` keeps the arm it has, `Label::paint` still
  never reads `truncation`, and `doc/ui/IMPLEMENTATION_STATE.md` § *Tasks 30–32*
  row 32 keeps reading that **there is no fade ramp at all**. **This task supplies
  the one number task 32's requirement 2 needs and closes none of it** — see
  § *The fade ramp is out, and what this task owes it instead*, which says so in
  three checkable places rather than once
- **No `LayoutMode::Grid`.** `LayoutMode::Grid { .. } => Vec::new()` in
  `layout.rs`'s arrange match, `columns` declared and never read.
  `TASK_UI_PRIM_52` owns it
- **No ellipsis redesign.** `truncate_line`'s `Truncation::Ellipsis` arm, its
  budget arithmetic and `LayoutOptions::truncation` are untouched. **This task adds
  the field, not the ellipsis**, and the cut line's width was already computed
- **No font fallback work.** No family is added, no chain rule is added,
  `Font::has_glyph` and `PickedFont` are untouched, and
  `replacement_advance`'s role is unchanged. `TASK_UI_PRIM_30` owns the chain
- **No glyph-atlas change.** `GlyphAtlas`, its `(char, rounded_size, FaceId)` key,
  its power-of-two growth and `REPLACEMENT_MIN` are untouched
- **No outline extraction and no signed distance field.** `font.rs` records that an
  SDF was built here and removed — *"A distance field has to binarize"* — and the
  bound this task needs is the pen's travel, which is arithmetic over advances and
  needs no description of a glyph edge
- **No tenth field, and no `line_height`.** The bound's height is `font_size`, the
  em box, and the honest limit is that it can be shorter than the ink for a face
  with a large line gap. **A tenth field is a second geometry a caller can get
  wrong**, and every caller in the crate treats `font_size` as the line box
  (`Keyboard::paint` centres on it; `Label`'s tests use it as the line height)
- **No change to `TextInput`'s selection, caret, scrolling or event handling.**
  `position_of`, `offset_at_position`, `caret_x`, `selection_rect`,
  `ensure_caret_visible` and `on_event` are untouched. **The recorded width is not
  used to shorten the walk that produced it** — that would be circular
- **No change to `Chart`'s production code, and no measurement seam added to it.**
  Its labels stay `None`. **Adding an `advance` parameter to `Chart::paint` is
  task 45's or another task's decision**, and this task records the limit instead of
  fixing it
- **No change to `ui_demo`'s `inked_box`, `placed_rects`, `Demo::order`, any page,
  any placement constant or any paint call.** The demo's edit is `#[cfg(test)]` and
  doc comments, and that is the mechanism of the pixel-identical criterion
- **No trimming of a straddling text run.** `clip_commands` keeps a straddler
  **whole** — trimming a run's string is a different picture, not a clipped one,
  and the scissor cuts the pixels — which is requirement 9's reason restated and
  not a change of behaviour
- **No new widget, no new page, no new UI of any kind.** `Page::ALL` still holds
  six names, `Page::DEFAULT` is still `Pads`, `--help` is unchanged, and no new
  `--tab=` name exists
- **No new dependency, no new `unsafe`, no new `unwrap`/`expect`/`panic!` in
  production code.** Per `AGENTS.md` the approved direct dependencies are `sdl3
  0.20`, `glow 0.18` and `freetype-rs 0.38`. **Edition 2021 and
  `rust-version = "1.85"` are respected** — no stdlib feature newer than the
  floor is used, and `Option::copied`, `Option::filter` and `f32::is_finite` are
  all long-standing
