# TASK_UI_PRIM_32: Fade and clip truncation, drawn

## Goal

Make `Truncation::Fade` fade and `Truncation::Clip` clip at paint time, so the
two modes differ on screen as their names claim.

## Context

`truncate_line` handles `Clip` and `Fade` **identically**: both cut the text to
`max_width` and set the line's `truncated` flag. Nothing downstream reads it.
`Label::paint` iterates the lines and paints each with one flat `color`; it never
reads `options.truncation` and never looks at a line's `truncated` flag. So:

- **Fade** is a misnomer. There is no ramp, no alpha gradient, nothing.
- **Clip** is a *layout* cut — whole characters are dropped — not a visual clip.
  Nothing clips a glyph that overhangs `max_width`, and `Renderer::set_scissor`
  documents that per-node clip is deferred, with `LayoutState::clip` carrying the
  rect and no hook to apply it.

Ellipsis, the third mode, does work: task 11's screenshot shows the three dots.
Task 11 requirement 4 listed "Text truncation: ellipsis, clip, fade"; one third
of it is met.

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
5. The alpha ramp composes correctly with the text batch's blend
   (`ONE, ONE_MINUS_SRC_ALPHA`, premultiplied output) and with inherited opacity.
6. Decide and record whether the ramp is a per-glyph alpha factor or a shader
   term. Per-glyph alpha quantises the ramp to glyph boundaries and leaves the
   batcher alone; a shader term is smooth and costs a uniform. Both are
   defensible — record the choice and its reason.
7. `ui_demo` shows all three modes on comparable text, so the difference is
   visible in one screenshot.

## Acceptance Criteria

- [ ] A fade-truncated line's last glyphs are less opaque than its first, and the
      ramp is monotonic — asserted on the recorded draw commands' alpha
- [ ] With `Right` and `Center` alignment the ramp sits at the drawn run's cut
      edge, not at the container's
- [ ] A glyph that overhangs `max_width` is clipped away, and a glyph inside it
      is not
- [ ] Two labels with different clip rects in one frame do not clip each other
- [ ] Fade alpha survives the premultiplied blend and inherited opacity
- [ ] The demo shows ellipsis, clip and fade on comparable text, in a captured
      screenshot

## Out of Scope

- Rich text, and per-glyph animation of the ramp
- Gradients other than the truncation fade
- Fading on the vertical axis, or a fade on any other widget
- Multi-line fade policy: fading every truncated line, or only the last, is a
  decision to record in `IMPLEMENTATION_STATE.md`, not a requirement here
