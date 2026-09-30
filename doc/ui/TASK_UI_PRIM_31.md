# TASK_UI_PRIM_31: Dynamic atlas growth

## Goal

Grow the glyph atlas instead of dropping glyphs when it fills, and never lose a
glyph in silence.

## Context

The atlas is a fixed `ATLAS_SIZE: u32 = 2048` square
(`render.rs:86`, `GlyphAtlas::new(ATLAS_SIZE)`). `GlyphAtlas::allocate` loops
`fit`, then `evict_lru_row`, and returns `None` when `evict_lru_row` cannot free
a row. `None` propagates: `get_or_insert` → `None`, and `draw_text_batch`
`continue`s — so the character is **not drawn and nothing is reported**. LRU
eviction is a legitimate policy, but it is only legitimate once the atlas is as
large as it is allowed to get, and the drop must not be silent.

The glyphs already in the atlas also have to survive a resize: every placement
carries UVs into the old texture, and every row carries a `row_y` the LRU order
depends on. A grow is a re-pack, not a re-alloc.

Task 11 requirement 3 listed "Dynamic atlas growth". It is unmet.

## Requirements

1. When `allocate` cannot fit a glyph, grow the atlas to the next power of two,
   up to a documented maximum, before falling back to eviction.
2. Growing re-packs the live glyphs into the new atlas and **updates their UVs**
   and row bookkeeping. A glyph placed before the grow must still address its own
   pixels afterwards.
3. The GL texture is reallocated and the dirty region covers the new texture, so
   the first frame after a grow uploads everything.
4. The maximum is checked against `GL_MAX_TEXTURE_SIZE`; the atlas never asks GL
   for a texture larger than the driver allows.
5. LRU eviction is what happens *after* growth is no longer possible, and it
   keeps the "icons before text" preference task 11's spec asks for.
6. A glyph that cannot be placed even at the maximum atlas size is **reported**,
   not dropped in silence: a `RenderError`, or a counted drop with a documented
   policy. Silence is the defect this task exists to remove.

## Acceptance Criteria

- [ ] Filling a small atlas past its capacity grows it, and a glyph placed
      before the grow still addresses its own pixels afterwards — asserted on
      the re-read pixel data, not on the size alone
- [ ] Growth doubles to a power of two and stops at the documented maximum
- [ ] The dirty region after a grow covers the whole new atlas
- [ ] A glyph too large for the maximum atlas is refused by the recorded policy,
      and the refusal is observable rather than silent
- [ ] Eviction still happens at the maximum, and still frees a row
- [ ] The demo at the largest `+` size, with every label on screen, shows no
      missing glyph — in a captured screenshot

## Out of Scope

- Evicting to a smaller atlas again, or any memory ceiling across atlases
- Compressing the atlas, or an external texture array
- A runtime atlas-size setting in the demo
- Task 30's per-font atlas keying, except that this task's tests must not assume
  a single font
