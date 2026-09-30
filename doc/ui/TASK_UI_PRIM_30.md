# TASK_UI_PRIM_30: Font fallback chain

## Goal

Resolve `font_family` to a real font, and draw a character no font in the
chain covers as something visible instead of nothing.

## Context

Task 11 shipped a single font. `Label::font_family` exists as a property and
**nothing reads it** — it is set to `"sans-serif"` and never used; the demo
hardcodes `FONT_PATH = /usr/share/fonts/truetype/lato/Lato-Medium.ttf` and hands
the one `Font` to `Renderer::set_font`. `DrawCommand::Text` carries no font.

A character the font does not have is **silently dropped**:
`GlyphAtlas::get_or_insert` returns `None` when `Font::rasterize` returns `None`,
and `draw_text_batch` does `continue` on that. The glyph leaves a hole in the
word with no tofu, no log line, and no error. U+2026 was checked by hand during
task 11 and *is* in Lato, so the case is easy to miss.

`GlyphKey` is `{ ch, size }`. Two fonts would therefore collide in the atlas on
the same character at the same size, which must be fixed before a second font can
be added at all.

Task 11 requirement 2 listed "Font fallback chain". It is unmet, and this task
is where it lands.

## Requirements

1. A font set owned by `ui_core`: several `Font`s, each with a family name, and
   a `FontId` newtype that the rest of the text path can carry.
2. `font_family` resolves through that set to a chain of fonts, tried in order
   until one covers the character. The chain is built once per family, not per
   glyph.
3. Decide, record, and implement **where** the family is resolved: either
   `DrawCommand::Text` carries a `FontId` and the renderer owns the set, or the
   `Painter` resolves the family at record time. Record the choice and its
   reason in `IMPLEMENTATION_STATE.md`; both are defensible and the decision
   belongs to the operator.
4. A character that **no** font in the chain covers draws a visible replacement
   — a tofu box from the primary font, or a documented substitute glyph. It must
   never be a silent hole. Whatever it is, it advances the pen.
5. Metrics follow the font actually used: a run that spans two fonts sums each
   glyph's own advance, and ascent and line height come from the primary font,
   with the choice recorded.
6. `GlyphKey` includes the `FontId`, so the same character at the same size from
   two fonts is two atlas entries.
7. `ui_demo` gains a label whose text contains a character the primary font does
   not cover, and `font_family` bound to a property the demo can change.

## Acceptance Criteria

- [ ] A run mixing two fonts lays out with each glyph's own advance, measured
      through the same callback `Label` lays out with
- [ ] A character absent from every font in the chain draws a visible
      replacement, not a hole — asserted on the recorded draw commands
- [ ] The atlas keys a glyph by font as well as character and size, and two
      fonts' glyphs for the same character do not collide
- [ ] Setting `font_family` changes the font a label draws with
- [ ] The demo shows the fallback glyph, in a captured screenshot

## Out of Scope

- Shaping, ligatures, complex scripts and bidi — waived with HarfBuzz on
  2026-09-30; see `AGENTS.md` § Rust
- Discovering fonts through fontconfig, or any system font enumeration
- Variable fonts, font loading UI, web fonts
- A per-widget font weight or style axis
