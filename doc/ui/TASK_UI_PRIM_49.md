# TASK_UI_PRIM_49: Gap `L8` — a Text Run That Knows Its Own Width

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Put a drawn text run's width on the command, so `scroll::command_bounds` can bound a `DrawCommand::Text` instead of answering `None` for one.
That is the mechanism row `L8` of `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* says is missing.
This task clips nothing; it supplies the number a clip needs, which is `TASK_UI_PRIM_45`'s separate work.

## Context

**Decision: a ninth field, `width: Option<f32>`, appended after `weight` on `DrawCommand::Text`, supplied by the caller.** Three new `Painter` methods carry it; the existing three signatures stay byte-for-byte.

Eight of the ten production `Painter::text*` call sites, in six widgets, already hold the width and discard it.
`Chart::paint` takes no measurement seam, so `None` must be expressible — `None` is a fact, not a gap; `0.0` and `NaN` are no sentinels (an empty run measures `0.0`; a `NaN` fails every `scroll::intersects` comparison and would make `clip_commands` drop the run).
The new methods are additive (`Painter::text*` are `pub`; `.ai/agents/developer.md` § *API design*), and `AdvanceCache` is untouched — measurement stays at layout time, upstream of the painter.
A recorded width is the pen's travel, the trailing `extra_advance` included.

Four sites stop compiling against a ninth field: the exhaustive patterns `text_bold_differs_from_text_in_the_weight_and_in_nothing_else` and `text_vertices`, and the literal constructions `text_and_image_commands_expand_to_no_quads` and `a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width`.

No test may open a font file; `label.rs`'s test module has `fn mono(_: char) -> f32 { 5.0 }`.
Row `L8` says seven fields, there are eight, and this task makes nine. Baseline **1894** (`ui_core` 1450, `ui_demo` 224, doctests 220); this task projects **1912**.

## Requirements

_Numbering is the source's; its requirement 16 (an `IMPLEMENTATION_STATE.md` entry) and 17 (the `cargo`/capture/frame-rate run) are dropped — verification is `.ai/agents/developer.md` § *Phase 3*._

1. **A ninth field on `DrawCommand::Text`, appended after `weight`, in `ui/src/ui_core/src/paint.rs`:** `width: Option<f32>`, documenting the pen's travel incl. the trailing `extra_advance`; that `crate::render::Renderer::draw_text_batch` does not read it; that it is measured with the chain that drew the run (`crate::font::FontSet::advance`); that `None` for `Chart` is a fact; and that `None` ≠ `0.0` (the `crate::widgets::scroll::clip_commands` drop). `git diff` one added line, no reorder.

2. **Three new `Painter` methods in `paint.rs`, beside their twins:** `pub fn text_measured(&mut self, width: f32, x: f32, y: f32, text: &str, color: Color, font_size: f32, extra_advance: f32)`; `pub fn text_in_measured(&mut self, family: FamilyId, width: f32, x: f32, y: f32, text: &str, color: Color, font_size: f32, extra_advance: f32)`; `pub fn text_bold_measured(&mut self, width: f32, x: f32, y: f32, text: &str, color: Color, font_size: f32, extra_advance: f32)` — each `#[allow(clippy::too_many_arguments)]`, documented *"every argument as [`Painter::text_measured`], one family / one weight different"*, routing to `text_in_weight`. `text_measured` carries the doctest (a `FontSet::new`, two `"Settings"` runs, the `Some` widths asserted, and unmeasured `Painter::text` asserted `None`); its doc says `width` is not checked, so a non-finite width is recorded and `command_bounds` declines to bound the run. The old three gain one sentence naming their measured twin.

3. **`Painter::text_in_weight` gains `width: Option<f32>` as its second parameter** and records it; it stays `#[allow(clippy::too_many_arguments)]` and the only place a `Text` is pushed; its doc's first line names the weight, the family and the width. Unmeasured pass `None`, measured pass `Some`.

4. **`render.rs`, two sites, no GL call:** `text_vertices` binds `width: _` in its exhaustive destructuring (no `..`), with a comment naming `scroll::command_bounds`; `text_and_image_commands_expand_to_no_quads`'s literal gains `width: None`.

5. **The eight call sites in six widgets pass the number each already computed:** `Label::paint` (`line.width` in both branches; the justified branch passes `measure(word, options.letter_spacing, advance)` per word), `Button::paint` and `Toast::paint` (`line.width`), `Dialog::paint_lines` (`line.width`, both the `Regular` and `Bold` branches), `Keyboard::paint` (`text_width`, the `advance * count` it centred on), `TextInput::paint` (the `visible_run` sum of requirement 6). `Chart::draw_labels` does not change — requirement 7. `Dialog::paint_lines`'s comment *"this function measures nothing"* is corrected; `Keyboard::advance`'s doc gains one sentence.

6. **`visible_run` returns `-> (String, f32, f32)` in `ui/src/ui_core/src/widgets/text_input.rs`** — the kept run, the first kept offset, and the sum of the advances of exactly the characters kept. One accumulator and one `return` are added; the walk, the `from`/`to` comparisons and the keep/drop rule are byte-identical.

7. **`Chart` records no width, and its reasons are re-owned rather than deleted.** `Chart::draw_labels` keeps both `painter.text` calls byte-for-byte; its doc's *"Placed, never measured, because [`DrawCommand::Text`] carries no width"* becomes *Placed, never measured — now this widget's own limit: the command **can** carry a width, and `Chart::paint` takes no measurement seam.* Eight more sites get the same re-owning, each keeping its conclusion: module docs' § *Labels are placed, never measured*, `Y_LABEL_GUTTER`, `Chart::stroke_reach`, `Chart::paint`'s reachability table, `series_label`, three `#[cfg(test)]` comments.

8. **`scroll::command_bounds` gains a text arm, in `ui/src/ui_core/src/widgets/scroll.rs`:**

   ```rust
   DrawCommand::Text { x, y, font_size, width, .. } => width
       .copied()
       .filter(|w| w.is_finite() && font_size.is_finite())
       .map(|w| Rect::new(*x, *y, w.max(0.0), *font_size)),
   ```

   The line box is `x` to `x + width`, `font_size` tall; `None` and a non-finite coordinate are one answer; a negative width is zero. The `None` paragraph names three cases — an empty `Path`, a `Polygon` under three points, a `Text` with no width (`Chart`'s). Its doctest gains one off-origin assertion: `x = 41.0`, `y = 17.0`, `font_size = 18.0`, `width = Some(90.0)` → `Rect::new(41.0, 17.0, 90.0, 18.0)`.

9. **`clip_commands` keeps its three outcomes and its straddler rule; a measured `Text` wholly outside is now dropped.** A straddler is not trimmed — a shorter string is a different picture and the scissor cuts the pixels — and the missing-width sentence is narrowed to *"which is now `Chart`'s labels and no one else's."*

10. **The text pass is untouched:** `git diff ui/src/ui_core/src/render.rs` shows `width: _`, `width: None`, and nothing else; no GL call, uniform, vertex computation, blend state or shader source moves.

11. **Tests for 49.1,** no display, network, filesystem or wall clock. `paint.rs`: `a_measured_painter_records_the_width_its_caller_hands_in`, `an_unmeasured_run_and_a_measured_one_of_the_same_string_differ_in_the_width_and_in_nothing_else`, `a_non_finite_width_is_recorded_rather_than_corrected`. `label.rs`: `a_label_records_the_width_of_the_line_it_paints` — **the width-without-a-font-file test** — through `mono` at `letter_spacing` `0.0` and `1.5`, `width == Some(5.0 * text.chars().count() + spacing * text.chars().count())` — and `a_justified_line_records_one_run_per_word_each_at_its_own_width`. `text_input.rs`: `the_field_records_the_width_of_exactly_the_characters_it_drew`. `button.rs`/`toast.rs`/`dialog.rs`/`keyboard.rs`: `the_button_records_the_width_of_the_run_it_paints`, `the_toast_records_the_width_of_each_line_it_paints`, `the_dialog_records_the_width_of_every_line_it_paints_in_both_weights`, `a_key_records_the_advance_width_it_centred_on`. `chart.rs`: `a_chart_label_records_no_width_because_the_chart_cannot_measure_one`.

12. **`scroll.rs`'s tests, and the one rename:** `command_bounds_answers_for_a_text_run_that_carries_its_width` (`Some(90.0)` at `x = 41.0`, `y = 17.0`, `font_size = 18.0` → `Rect::new(41.0, 17.0, 90.0, 18.0)`, and `None` with every other field identical); `a_text_run_is_bounded_by_its_line_box_and_not_by_its_baseline`; `a_non_finite_or_negative_width_leaves_a_text_run_unbounded` (`NaN`, `f32::INFINITY`, `f32::NEG_INFINITY` → `None`; `-4.0` → a zero-width rect at `x`); `a_measured_text_run_outside_the_clip_is_dropped_and_an_unmeasured_one_is_kept`. Rename `a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width` to `an_unmeasured_text_run_is_kept_because_the_command_carries_no_width`; its fixture gains `width: None`, its `None` assertion and `clip_commands` half are unchanged, its message becomes *"the run carries no width, so there is nothing to bound it with."*

13. **`ui/src/ui_demo/src/main.rs` gains one test and changes no production line:** `no_recorded_text_run_would_be_dropped_where_it_drew` — over all six `Page::ALL` pages plus `Page::Overlays` with the dialog open and a toast raised, every node in `Demo::order`, every boundable `Text` asserted inside its `LayoutState::clip`. Its doc states it strengthens `TASK_UI_PRIM_45`'s `no_recorded_command_is_cut_by_its_own_nodes_clip`.

14. **The falsities are corrected, by symbol.** `text_input.rs`: module docs' § *Clipping, and why the widget has to do it itself* (enumeration → the nine fields' names, conclusion present-tense), `TextInput::paint`'s and `visible_run`'s docs. `scroll.rs`: `command_bounds`'s doc and `clip_commands`' three-outcome list; module docs' § *The second decision is clipping* keeps its no-scissor-state sentence; correct `TASK_UI_PRIM_45`'s five-short enumeration if it landed first. `chart.rs`: the nine sites of requirement 7. `paint.rs`: the stale counts corrected to six, seven, nine; `text_bold_differs_from_text_in_the_weight_and_in_nothing_else` gains `width: regular_width`/`width: bold_width` (not `..`) and asserts equality.

15. **Row `L8` of `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* gains a dated note, and is not deleted or closed:** the ninth field and the text arm; the field count seven→eight→nine; caller-measured width, `None` as `Chart`'s answer; the two `Blocks` entries (card content that overflows, the carousel's page edges) not delivered (no card, no pager); the fade ramp remains `TASK_UI_PRIM_32`'s.

_Verification is `.ai/agents/developer.md` § *Phase 3*; task-specific: six pages pixel-identical (`AE 0` outside `y ≥ 680`), every page above floor **55**, suite green against baseline **1894**, `cargo audit` recorded as not installed, not passed._

## Acceptance Criteria

- [ ] **`DrawCommand::Text` has the ninth field**, `width: Option<f32>`, in the order `x`, `y`, `text`, `color`, `font_size`, `extra_advance`, `family`, `weight`, `width`; `git diff` one added line, no reorder; the doc carries its five claims.
- [ ] **The old three signatures are untouched and the three new methods exist**, each `#[allow(clippy::too_many_arguments)]` and routing to `text_in_weight`; `text_measured`'s doctest asserts both halves; `text_in_weight` is the only `Text` push (`grep -c 'self.commands.push(DrawCommand::Text' ui/src/ui_core/src/paint.rs` is **1**).
- [ ] **`command_bounds` answers for a carried width and for none**, with the `is_finite` filter and the off-origin fixture; making the arm ignore `width` fails the suite, and deleting the filter makes the `NaN` drop test fail.
- [ ] **The one rename is done and no test deleted or weakened:** the old name greps zero and the new name one line; its `y = 5000.0` placement is kept.
- [ ] **The width equals the sum of advances with `mono`, no font file at a path**, at `letter_spacing` `0.0` and `1.5`; `grep -rn 'define_family\|FontSet::family('` over the eight touched widget files returns nothing new, and assertions are made in `FamilyId::default()`.
- [ ] **Exhaustive destructurings keep catching and argument counts are right:** `width: regular_width`/`width: bold_width` and `width: _`, no `..`; `grep -n 'The same eight arguments'` and `grep -n 'Nine arguments is the command'` over `paint.rs` return nothing, and six, seven, nine are in the file.
- [ ] **The GPU path is untouched:** `git diff ui/src/ui_core/src/render.rs` shows `width: _`, `width: None` and nothing else; `font.rs` is unchanged — no atlas change, no outline extraction, no distance field, `AdvanceCache` untouched.
- [ ] **The pre-flight ran and its count is pasted**; a non-zero count outside a node's own `LayoutState::clip` on any page is a stop condition, fixed in the producer, never in `command_bounds`.
- [ ] **The six gallery pages are pixel-identical:** `AE 0` outside `y ≥ 680`, every differing pixel inside the fps band. Mechanisms: the demo diff is `#[cfg(test)]` and doc comments only; no shipped command reaches `command_bounds`' one production consumer `List::paint` (`grep -n 'List::new' ui/src/ui_demo/src/main.rs` returns nothing); `text_vertices` does not read the width. The rect-level tests keep every name and assertion — `every_page_places_every_rect_where_the_gallery_placed_it`, `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`, `inked_box_bounds_a_shadow_by_the_blur_modules_own_reach`, `the_highest_ink_on_any_page_is_the_card_of_pads_at_the_tab_bar`.
- [ ] **`cargo test --all-features` is green with every test named in requirements 11–13 present**; baseline **1894**, projected **1912** (seventeen new tests, one new doctest, one rename), and every page above floor **55** against its own pre-change number.
- [ ] **Nothing outside the width field moves:** row `L8` is amended, dated and open with its `Blocks` column intact; the fade ramp is untouched (`Truncation`, `truncate_line`, `LayoutOptions::truncation`, `Label::paint`'s truncation handling and every shader source), this task supplying only task 32's requirement-2 cut edge (`x + width`); `git diff --stat` shows no change to `font.rs`, `layout.rs`, `batch.rs`, `input.rs`, `node.rs`, `theme.rs`, `render/target.rs`, `render/context.rs`, `render/blur.rs` or any `widgets/*.rs` this task does not name; `scroll.rs`'s clipping half keeps its bytes; `ui/Cargo.toml` and `ui/Cargo.lock` unchanged; no `unsafe` or `unwrap`/`expect`/`panic!` in production code; no changed doc asserts the opposite of its code.

## Out of Scope

- **No fade ramp, alpha gradient, per-glyph alpha factor or shader term.** `TASK_UI_PRIM_32` owns it; `Truncation::Fade` stays layout-identical to `Truncation::Clip`.
- **No `LayoutMode::Grid`** (`LayoutMode::Grid { .. } => Vec::new()` in `layout.rs`; `columns` never read). `TASK_UI_PRIM_52` owns it.
- **No ellipsis redesign:** `truncate_line`'s `Truncation::Ellipsis` arm, its budget arithmetic and `LayoutOptions::truncation` are untouched.
- **No font fallback work:** no family or chain rule added; `Font::has_glyph`, `PickedFont` and `replacement_advance` unchanged. `TASK_UI_PRIM_30` owns the chain.
- **No glyph-atlas change:** `GlyphAtlas`, its `(char, rounded_size, FaceId)` key, its power-of-two growth and `REPLACEMENT_MIN` are untouched.
- **No outline extraction and no signed distance field.** `font.rs` records an SDF was built and removed — *"A distance field has to binarize"*.
- **No tenth field and no `line_height`.** The bound's height is `font_size`, the em box.
- **No change to `TextInput`'s selection, caret, scrolling or event handling:** `position_of`, `offset_at_position`, `caret_x`, `selection_rect`, `ensure_caret_visible` and `on_event` are untouched; the recorded width is not used to shorten the walk that produced it.
- **No change to `Chart`'s production code and no measurement seam added to it.** Its labels stay `None`.
- **No change to `ui_demo`'s `inked_box`, `placed_rects`, `Demo::order`, any page, placement constant or paint call.**
- **No trimming of a straddling text run:** `clip_commands` keeps a straddler whole.
- **No new widget, no new page, no new UI:** `Page::ALL` still holds six names, `Page::DEFAULT` is still `Pads`, `--help` unchanged, no new `--tab=` name.
- **No new dependency, no new `unsafe`, no new `unwrap`/`expect`/`panic!` in production code.** Approved direct dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; Edition 2021 and `rust-version = "1.85"`.
