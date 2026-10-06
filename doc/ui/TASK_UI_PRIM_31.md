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

> **Read after implementation (2026-10-05).** The paragraph above was measured
> before this task and every claim in it held: the atlas was a fixed
> `GlyphAtlas::new(ATLAS_SIZE)`, `allocate` looped `fit` then `evict_lru_row`,
> and the `None` reached `draw_text_batch` and dropped the character with nothing
> reported. **`render.rs:86` is the line the citation was written against and is
> not one to follow** — `AGENTS.md` cites by section for this reason, and this
> file is now the thing that went stale inside it.
>
> **Two things the Context does not say, and both changed the work.**
>
> 1. **The atlas held no copy of the glyphs' coverage, only their pixels, so a
>    re-pack could only move pixels — which it must, because a UV cannot be
>    recomputed from a UV.** The entry a glyph is filed under now holds the pixel
>    it was written at, and the UVs are derived from the size the atlas is *now*
>    at the moment the placement is handed out. That is one step stronger than
>    requirement 2 below asks for, and it is not decoration: **a batch's quads
>    are built while its glyphs are being packed, so a grow part-way through a
>    batch leaves the quads already built addressing the old texture's
>    coordinates.** The batch is therefore expanded again if the atlas grew
>    during it (`draw_text_batch`), and the derived UVs are what make the second
>    pass correct rather than merely repeated.
> 2. **The starting size was never checked against the driver either.** The atlas
>    was built at `ATLAS_SIZE` whatever `GL_MAX_TEXTURE_SIZE` said, so a driver
>    under 2048 was handed a texture it cannot allocate — as an unchecked GL error
>    nobody reads, which is the same shape of failure requirement 4 is about. Both
>    numbers are bounded now.

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

> **Requirement 2 is met by a different mechanism, and requirement 5's icons half
> is structural.** Both are recorded here because the task file owns the
> requirements and neither is what the sentence above literally says.
>
> - **"updates their UVs"**: a re-pack writes each shelf's pixels at a new `y`
>   and each entry records that `y`; the UVs are computed from it when the
>   placement is handed out, so there is no stored UV that could go stale. The
>   observable the requirement is about — a glyph placed before the grow
>   addressing its own pixels afterwards — is asserted on the re-read pixel data
>   by `a_glyph_packed_before_a_grow_still_addresses_its_own_pixels`.
> - **"icons before text"**: glyphs and images are in **two textures**
>   (`font.rs` and `texture.rs`), so evicting a glyph cannot take an icon in any
>   order, and no scheduling was needed to keep the preference. The ordering the
>   sentence is really about lives in the image atlas, which has LRU shelves and
>   pinned handles. **This task did not touch the image atlas at all**, which is
>   also its *Out of Scope*.
> - **Requirement 6's policy, as implemented, and corrected 2026-10-05 after
>   the review**: a refused glyph is not drawn; the pen still advances by the
>   advance the font reports, so a run keeps its spacing and one missing glyph does
>   not reflow the line; **the count is over the keys, not the events** — a refused
>   glyph *is* asked for again on every frame, because a key that is not in the map
>   is re-rasterized, so an event count would report one missing glyph as thousands
>   — and a key leaves the count the moment it packs again. It is
>   `GlyphAtlas::dropped`, surfaced as
>   `Renderer::dropped_glyphs` and printed by `ui_demo` on its own
>   `roados-glyphs atlas_size=… dropped=…` line on every run. **An eviction is
>   not counted** — an evicted glyph is re-rasterized the next frame the run asks
>   for it — and **a character no font covers is not counted either**: that is a
>   space, or the replacement box, and the caller already knows it asked.

## Acceptance Criteria

- [x] Filling a small atlas past its capacity grows it, and a glyph placed
      before the grow still addresses its own pixels afterwards — asserted on
      the re-read pixel data, not on the size alone —
      `a_glyph_packed_before_a_grow_still_addresses_its_own_pixels`. Each glyph is
      packed as a bitmap whose pixels **all differ from the flat value a wrong
      address would also return**, and the assertion compares **the glyph's whole
      rectangle including its transparent padding** — so a glyph read back from
      anywhere else differs both inside and at its border. **The same test also
      sweeps every key after the grow**, because a survivor and a missing assertion
      look the same.
      **Corrected 2026-10-05 after the review**: this criterion first said *"no two
      pixels carry the same value"*, which is not a property any fixture can have.
      The pattern is `1 + (x·7 + y·13 + seed·29) % 254`, and a 20 × 20 glyph has 400
      pixels and 254 available values, so **146 of its pixels share a value with
      another**. The assertion was strong anyway, for the reason above; the claim
      about the fixture was not.
- [x] Growth doubles to a power of two and stops at the documented maximum —
      `growth_doubles_to_a_power_of_two_and_stops_at_the_maximum`, from a start
      that is **not** a power of two (100) to a ceiling that is not one either
      (200), so the sequence it asserts is `[100, 128, 200]` and not "the ceiling".
      The maximum is **4096** (`ATLAS_MAX_SIZE`), 16 MB of `GL_R8`, and it is
      bounded by the driver below that.
- [x] The dirty region after a grow covers the whole new atlas —
      `a_grow_dirties_the_whole_new_texture`, which isolates the flag by making
      the atlas clean and then growing it **without blitting anything**, so the
      flag cannot be the blit's. On the GL side `upload_atlas` reads
      `self.atlas.size()` per upload and hands the whole buffer over, so a grow
      is a reallocation at the new size with every texel written.
- [x] A glyph too large for the maximum atlas is refused by the recorded policy,
      and the refusal is observable rather than silent —
      `a_glyph_too_large_for_the_ceiling_is_refused_and_counted` (which also
      asserts the atlas **did not grow three times** to arrive at a refusal its
      own width already predicted) and
      `a_glyph_the_font_has_no_bitmap_for_is_not_a_refusal` for the two `None`s
      being one counter apart, and
      `a_glyph_refused_every_frame_is_counted_once_and_not_asked_about_again` for
      the count being idempotent. Observable: `dropped`, `Renderer::dropped_glyphs`,
      and the demo's report line. **Corrected 2026-10-05 after the review**: the
      first version counted *refusal events*, and a refused glyph is asked for again
      on every frame — so the number grew with the frame count for a glyph that is
      not being drawn at all, and the policy paragraph claimed the opposite of what
      the code did. **It counts keys now**, and a key leaves the report the moment
      it packs again.
- [x] Eviction still happens at the maximum, and still frees a row —
      `eviction_still_frees_a_shelf_at_the_ceiling`, which measures which shelf
      each glyph was on rather than assuming the oldest was alone on it.
- [x] The demo at the largest `+` size, with every label on screen, shows no
      missing glyph — in a captured screenshot. **Two captures, and the `+` keys
      were not used**: this host injects no events, so `TEXT_SIZE_START` was
      seeded to 64 (the `+` ceiling) and to 40 (the largest size at which the
      text page's labels all fit), built, captured and **reverted**. Every run
      printed `roados-glyphs atlas_size=2048 dropped=0`, and the 40-pixel capture
      shows all nine labels including task 30's `Fallback ⚠ ✓ and □ end` — three
      fonts, one line, no hole.

> **The seventh thing — a capture that requirement 2 could not otherwise have —
> and what it does *not* cover.** The demo's atlas never fills, so growth was
> exercised by seeding `ATLAS_SIZE` to 64 as well. The run grew to **256** and
> reported it, and **the text page's label rows came out pixel-identical to the
> same page drawn from a 2048 atlas** — `magick compare -metric AE -crop
> 1280x380+0+230` reads **0**, the whole window's 2 199 differing pixels being the
> frame-rate readout and the padding. That is requirement 2's guarantee measured
> through GL, on a texture reallocated under it.
>
> **It is not evidence for the batch's second pass, and the record used to say it
> was.** The review tested that: it built the demo twice, once with the
> re-expansion and once with it broken, and compared the same crop at 0.3 s, 1 s,
> 3 s and 8 s — **AE = 0 at every one**. The defect occupies exactly the frame the
> grow happens on, and every frame after it is identical either way, so no capture
> can see it. **The rule is a free function now** — `expand_until_settled` beside
> `text_vertices`, both in `render.rs` — and two tests cover it, one of which
> asserts the batch is expanded **once** when nothing grew.

## Out of Scope

- Evicting to a smaller atlas again, or any memory ceiling across atlases
- Compressing the atlas, or an external texture array
- A runtime atlas-size setting in the demo
- Task 30's per-font atlas keying, except that this task's tests must not assume
  a single font
