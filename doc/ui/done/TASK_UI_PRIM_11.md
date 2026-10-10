# TASK_UI_PRIM_11: Widget — Label

## Goal

Implement the Label widget: text rendering with SDF glyph atlas.

## Context

Label is the simplest widget — it displays text. It uses FreeType for glyph rasterization, SDF for scalable text, and HarfBuzz for text shaping.

## Requirements

1. Implement `ui_core::widgets::label` module:
   - `Label` widget node with properties: `text: Property<String>`, `font_size: Property<f32>`, `color: Property<Color>`, `font_family: Property<String>`
   - `Label::new(text: impl Into<String>) -> Handle` — create a label in the arena

2. Text rendering pipeline:
   - FreeType for glyph rasterization (load TTF/OTF fonts)
   - SDF (Signed Distance Field) glyph atlas for scalable text
   - HarfBuzz for text shaping (complex scripts, ligatures)
   - Bidirectional text support (Arabic, Hebrew)
   - Font fallback chain

3. Glyph atlas:
   - Pack glyphs into a texture atlas (e.g., 2048x2048)
   - SDF rendering: each glyph stored as distance field
   - Atlas eviction: LRU when full, glyphs evicted before icons
   - Dynamic atlas growth

4. Text layout:
   - Single-line and multi-line text
   - Text wrapping (word wrap, character wrap)
   - Text alignment: left, center, right, justify
   - Line height, letter spacing
   - Text truncation: ellipsis, clip, fade

5. Rendering:
   - Label renders as a draw command using the text shader
   - SDF text shader: samples distance field, applies smoothstep for anti-aliasing
   - Color from `color` property
   - Opacity from inherited opacity

## Acceptance Criteria

- [ ] Label renders text correctly
- [ ] Text wraps correctly
- [ ] Text alignment works (left, center, right)
- [ ] Font size changes are reflected
- [ ] Color changes are reflected
- [ ] SDF text is crisp at all sizes
- [ ] Demo shows a label with "Hello, World!"

## Out of Scope

- Rich text (multiple styles in one label — comes later)
- Text selection/editing (comes with TextInput in TASK_UI_PRIM_19)
- Animated text (comes with animation system integration)
