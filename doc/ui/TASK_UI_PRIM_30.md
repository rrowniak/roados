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

> **Amended 2026-10-05, after implementation.** The paragraph above is a
> **measurement taken before task 22, and task 22 answered it.** `GlyphKey` has
> carried a third field — `FaceId`, since renamed `FontId` — since 2026-09-30,
> because the dialog's bold title needs a real second face and that face's glyphs
> are keyed apart from the regular one's. **The two fonts this paragraph warns
> about, Lato-Medium and Lato-Bold, have been packing separately for a year.**
>
> Requirement 6 below is therefore **met before this task began**, and its test —
> that two fonts' glyphs for one character do not collide — is
> `the_same_letter_in_two_faces_is_two_atlas_entries`, written by task 22 and
> still passing unchanged. What this task did to the key was different and is
> recorded in `IMPLEMENTATION_STATE.md` § *Task 30*: the **replacement** glyph
> needed a key of its own, because it belongs to no font and names no character,
> and a key carrying a character would have had to fabricate one. `GlyphKey` is
> an enum for that reason.

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

> **Requirement 4's first clause is unreachable on this host, and the operator
> chose the second.** *"a tofu box from the primary font"* needs a font with a
> glyph at U+FFFD, and **there is not one**: Lato-Medium, Lato-Bold,
> LiberationSans and NotoSansDevanagari were each checked against their own
> character map on this host and none has one. A rule "use U+FFFD when the
> primary has it" would take its fallback branch in the one place it can be seen.
> The replacement is **synthesized** — a hollow rectangle, one pixel of ink per
> edge, on the baseline — and its advance and its bitmap's `advance` are the same
> function's answer so the hole and the box cannot disagree.

## Acceptance Criteria

- [x] A run mixing two fonts lays out with each glyph's own advance, measured
      through the same callback `Label` lays out with — `FontSet::advance` asks
      the chain per character and the demo's fallback label is measured through
      it, and `the_fallback_label_is_measured_through_its_own_family_and_not_the_
      panels` measures one 22-character string through both of the demo's
      measurement paths and gets **316.8** and **264.0** out of them.
      **The two-font clause itself is capture-only, and this is the honest half of
      the criterion:** the fixture's set holds no fonts, so every character in
      every run the suite measures is uncovered and **no run in the suite mixes
      two fonts** — which needs a font file, and `AGENTS.md` forbids a test to
      open one. What the capture shows is the two families of the demo drawing
      the same sentence at **247 px and 234 px**, and the review of task 30 is
      what caught that this criterion claimed numbers (`259.2`/`216.0`) belonging
      to the 18-character text that shipped before `" end"` was appended
- [x] A character absent from every font in the chain draws a visible
      replacement, not a hole — asserted on the recorded draw commands by
      `a_replacement_is_packed_and_returned_rather_than_dropped`, and **measured
      on screen** at 14 × 17 hollow pixels
- [x] The atlas keys a glyph by font as well as character and size, and two
      fonts' glyphs for the same character do not collide — **met by task 22**
      before this task began; see the amendment above
- [x] Setting `font_family` changes the font a label draws with — through the
      property, in the recorded command, and **in the pixels**: the same sentence
      is 247 px wide in the default family and 234 px in `lato-only`
- [x] The demo shows the fallback glyph, in a captured screenshot — one line,
      three characters, three outcomes, with the arithmetic of the box checked
      against the two constants

## Out of Scope

- Shaping, ligatures, complex scripts and bidi — waived with HarfBuzz on
  2026-09-30; see `AGENTS.md` § Rust
- Discovering fonts through fontconfig, or any system font enumeration
- Variable fonts, font loading UI, web fonts
- A per-widget font weight or style axis
