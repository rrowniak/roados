# TASK_UI_PRIM_32: Fade and clip truncation, drawn

**Status, 2026-10-06 (sub-task 32.3):** implemented as three sub-tasks —
**32.1** the demo rows (`ui_demo` only), **32.2** the mechanism (`ui_core`),
**32.3** this record — and verified on the tree. **Not reviewed:** review is
`.ai/workflows/task-sequence.md` step 2, in a session separate from the
implementer's. The full record is `doc/ui/IMPLEMENTATION_STATE.md` § *Task 32
— what it decided, and what it found*. This file owns the requirements; where
it and the record disagree, this file wins and the record is corrected.

**Amended 2026-10-06 (sub-task 32.3):** two false premises in Context corrected,
requirement 6 decision recorded, requirement 4 mechanism and task-45 boundary
written, multi-line policy resolved, acceptance criteria ticked honestly, split
rationale preserved — superseded, not deleted, every entry dated and attributed.

## Goal

Make `Truncation::Fade` fade and `Truncation::Clip` clip at paint time, so the
two modes differ on screen as their names claim.

## Context

`truncate_line` handles `Clip` and `Fade` **identically**: both cut the text to
`max_width` and return a `(String, bool)` whose `bool` is folded into one
layout-wide `TextLayout::truncated` by `layout_text`. Nothing downstream reads
the folded flag. `Label::paint` re-runs `layout_text` itself rather than being
handed a layout, so a caller cannot pre-compute a ramp and hand it in — the
decision has to be made where the line is recorded. It iterates the lines and
paints each with one flat `color`; it never reads `options.truncation` and never
looks at a line's `truncated` flag (because `Line` carried only `text`, `width`,
`x_offset` and `word_gap` — no per-line flag existed). So:

- **Fade** is a misnomer. There is no ramp, no alpha gradient, nothing.
- **Clip** is a *layout* cut — whole characters are dropped — not a visual clip.
  Nothing clips a glyph that overhangs `max_width`, and `Renderer::set_scissor`
  documents that per-node clip is deferred, with `LayoutState::clip` carrying the
  rect and no hook to apply it.

Ellipsis, the third mode, does work: task 11's screenshot shows the three dots.
Task 11 requirement 4 listed "Text truncation: ellipsis, clip, fade"; one third
of it is met.

**Two premises in the original version of this section were false, corrected here
in place, 2026-10-06, against the source by the implementer's own reading** —
the same correction pattern `IMPLEMENTATION_STATE.md` § *Three premises that
measurement refuted* records for tasks 24.1–24.3, and the reason this file
carries a correction rather than a rewrite is that a task file owns its
requirements:

1. **There was no per-line `truncated` flag.** `truncate_line` returns a bare
   `(String, bool)`; `layout_text` folds that `bool` into one layout-wide
   `TextLayout::truncated` and discards it per line. `Line` itself carried only
   `text`, `width`, `x_offset` and `word_gap`. So *"set the line's `truncated`
   flag"* and *"never looks at a line's `truncated` flag"* both described a
   mechanism this pipeline did not have — the flag existed only as a folded
   `||` over every line's answer, which cannot say *which* line was cut.
2. **`Label::paint` re-runs `layout_text` itself** rather than being handed a
   layout, so a caller cannot pre-compute a ramp and hand it in — the decision
   has to be made where the line is recorded. This is why the fade rides on
   `DrawCommand::Text` from the paint site, and why the multi-line policy
   (below) needed a new per-line `Line::truncated` field.

The section's third premise holds: requirement 3's clip at the *label's own*
rect is **not** task 45's per-node clip, which is the ancestors' intersection
and explicitly refuses the node's own box — see requirement 4's note below.

## Requirements

1. **Fade.** The truncated line's glyphs ramp to zero alpha across a documented
   width at the cut edge.
2. The ramp follows the **text's** cut edge, not the container's: with
   `TextAlign::Right` or `Center` the fade is at the end of the drawn run, which
   is where the overflow is.
3. **Clip.** A real paint-time clip at the label's rect, per node.
4. Per-node clip must not leak: two labels with different clip rects in one frame
   must not clip each other. `LayoutState::clip` carries the rect; the batch key
   or the scissor state must carry it through to `end_frame`.
5. The alpha ramp composites correctly with the text batch's blend
   (`ONE, ONE_MINUS_SRC_ALPHA`, premultiplied output) and with inherited opacity.
6. Decide and record whether the ramp is a per-glyph alpha factor or a shader
   term. Per-glyph alpha quantises the ramp to glyph boundaries and leaves the
   batcher alone; a shader term is smooth and costs a uniform. Both are
   defensible — record the choice and its reason.
7. `ui_demo` shows all three modes on comparable text, so the difference is
   visible in one screenshot.

**Requirement 4, as built (2026-10-06).** The clip is a **command-level** rect:
`DrawCommand::Text` carries a new `clip` field, `Batcher::add_clipped` intersects
it with the batch's own clip, the existing `batch.clip == clip` merge predicate
splits the batch for free, and the cut is made by the scissor that already
existed (`Batch::clip` → `Renderer::apply_clip` → `set_scissor`). **A clip
deliberately does not go in `BatchKey`** — `Batch::clip`'s own doc says why: a
clip is not a property of the material, and keying on it would split a single
list into one batch per command.

**The boundary with task 45: two different clips, and neither subsumes the
other.** Task 45's clip is the **node's own ancestors' intersection**, resolved
in `Renderer::draw_node_clipped`, and task 45 explicitly does *not* clip a node
to its own box. This task's clip is one command's own rect, intersected with
the batch's. They answer different questions — *what is this command's box*
versus *what may this node draw inside* — and neither mechanism can express the
other's answer.

**One overlap is left open deliberately, and it is task 45's requirement 1.**
`paint::Rect::intersection` is total and takes one rect; `layout::intersect` is
private, takes two `Option<layout::Rect>`s and has a different answer for empty.
`Rect::intersection`'s doc names the overlap and says merging the two is task
45's requirement 1's decision, with both call sites in front of it.

**Requirement 6, resolved (2026-10-06, the operator's choice, taken before any
code).** The ramp is a **per-corner vertex alpha**: `paint::FadeRamp { start_x,
end_x }` in window coordinates rides `DrawCommand::Text` as a new `fade` field,
and `render::text_quad` asks `text_corner_color(ramp, x, color)` for **each
corner's own x**. Because the rasteriser already interpolates `v_color`, the ramp
is smooth *inside* a glyph — **no shader change, no vertex-format change
(`TextVertex` stays 32 bytes, `TEXT_VERTEX_STRIDE = 32`), no batching change.**

The two rejected alternatives, and why each was rejected:

- **Per-glyph alpha** — quantises the ramp to glyph boundaries, which is the
  defect requirement 6 names; it leaves the batcher alone, but a ramp that steps
  at every glyph edge is not a fade.
- **A shader term** — a uniform is per draw call, and one text batch holds every
  run on the frame, so a per-run ramp needs either one draw per ramp or a new
  per-vertex attribute. The first costs a draw call per run; the second costs a
  `[f32; 4]` on the command that nothing here needs.

**Premultiplication.** `paint::faded_color` interpolates the colour toward
transparent black, which scales all four channels — the one line
`button::with_opacity` writes. Scaling only the alpha gives text that keeps its
full brightness and lets the background through, which reads as a wrong colour
rather than a fade.

## Acceptance Criteria

- [x] **AC1** A fade-truncated line's last glyphs are less opaque than its first,
      and the ramp is monotonic — asserted on the recorded draw commands' alpha
- [x] **AC2** With `Right` and `Center` alignment the ramp sits at the drawn
      run's cut edge, not at the container's
- [ ] **AC3** A glyph that overhangs `max_width` is clipped away, and a glyph
      inside it is not — **half met, and the half that is missing is the
      instrument.** The gate is proved by font-free tests
      (`the_cut_leaves_the_last_glyph_inside_the_box_and_the_clip_is_what_would_cut_it`,
      `a_commands_own_clip_is_intersected_…`,
      `two_text_runs_whose_own_clips_differ_do_not_share_a_batch`,
      `a_clip_that_cuts_a_run_out_of_its_own_box_leaves_a_batch_that_draws_nothing`),
      and the cut was **never observed on screen**: measured with a scratch crate
      outside the repository (a test may not open a font file), Lato-Medium at
      24 px overhangs on **4 of the sentence's 101 characters, always by exactly
      1 px**, and the demo's own cut lands on `'p'` with the last kept glyph's
      ink ending **9 px inside `max_width`** — so the Clip row is AE 0 against
      its own before-capture, and the clip cut nothing. A criterion whose
      instrument cannot produce the evidence is not met by producing the evidence
      another way. **No temporary seed was
      used to manufacture an overhanging cut.**
- [x] **AC4** Two labels with different clip rects in one frame do not clip each
      other
- [x] **AC5** Fade alpha survives the premultiplied blend and inherited opacity —
      **met arithmetically, and the blend-state half is argued, not measured:**
      `end_frame` disables blending for the solid pass only and enables it once
      with `GL_ONE, GL_ONE_MINUS_SRC_ALPHA`, so a ramped run keyed
      `BlendMode::Opaque` still composites correctly and no faded run was routed
      into `segment.transparent`. Pinned by `a_faded_text_batch_is_still_drawn_with_blending_on`,
      a source-string assertion scoped to `end_frame`, because **a capture
      cannot see blend state** — and the premultiplied claim in `batch_key`'s
      `Text` arm is a claim about a GL state machine, which `cargo test` is not
      evidence for.
- [x] **AC6** The demo shows ellipsis, clip and fade on comparable text, in a
      captured screenshot

**AC summary (2026-10-06):** AC1, AC2, AC4, AC6 met; AC3 half (gate proved by
font-free tests, on-screen half not observed — overhang 4×1 px, demo cut 9 px
inside `max_width`); AC5 arithmetic met, blend-state argued (source-string
assertion pins `end_frame` blend state, capture cannot measure it).

## Out of Scope

- Rich text, and per-glyph animation of the ramp
- Gradients other than the truncation fade
- Fading on the vertical axis, or a fade on any other widget
- Multi-line fade policy: fading every truncated line, or only the last, is a
  decision to record in `IMPLEMENTATION_STATE.md`, not a requirement here
  — **resolved 2026-10-06: every truncated line ramps, at its own cut edge.**
  That needed a per-line `Line::truncated`, because the layout-wide flag was an
  `||` over the same answers and could not say which line was cut.

## The split into 32.1 / 32.2 / 32.3, and why

`developer.md` § *Scope check* put this task over both thresholds — more than 5
files and more than 3 independent components — so it was split per
`.ai/protocols/subagents.md` § *Implementation fan-out*. **Rule 1 of *Splitting*
is what fixed the shape:** two agents cannot both add the same field to
`DrawCommand::Text`, so the `fade` and `clip` fields and every constructing
site that has to learn them are **one** sub-task; a split that landed the field
first would leave the crate not building. 32.1 (the demo rows, `ui_demo` only)
therefore landed first and 32.2 (the mechanism, `ui_core`) second, each
verified on its own tree, and 32.3 is this record.

**The split is preserved because the code landed in that order and the record
must reflect the actual history**, not an idealised one. 32.1 (+292/−45 in
`ui_demo/src/main.rs`) extended the text page to 9 rows (6 original + 3 shared
sentence via `TRUNCATION_ROWS = [Ellipsis, Clip, Fade]`), extended completeness
assertions (`expected_placed_rect_names` 7→9, count 26→28, `labels.len()` 7→9),
added 2 tests and renamed 2 (no assertion removed), and captured the "before"
image at `/tmp/opencode/task32_before.png`. 32.2 (+743 `paint.rs`, +240
`batch.rs`, +341 `render.rs`, +789 `widgets/label.rs`, +40 `widgets/list.rs`,
+2 `widgets/scroll.rs`) implemented the three operator decisions, added
`Line::truncated`, `FadeRamp`, `DrawCommand::Text::{fade,clip}`, `Painter::text_run`,
`paint::TextRun`, `paint::Rect::intersection`, and verified with 1482 lib + 226
demo + 225 doctest = 1933 tests (+39 from 1894 baseline: +32 lib, +5 doctest, +2
demo, 0 removed, 4 extended). Both sub-tasks passed fmt, build, clippy, doc (0
warnings), `cargo audit` exit 0, and their respective mutation sweeps.
