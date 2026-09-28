# TASK_UI_PRIM_16: Widget — Image

## Goal

Implement the Image widget: displays a texture with various fit modes.

## Context

Image displays a static image. It supports different fit modes (contain, cover, fill) and opacity.

## Requirements

1. Implement `ui_core::widgets::image` module:
   - `Image` widget node with properties: `texture: TextureHandle`, `fit: ImageFit`, `opacity: Property<f32>`
   - `Image::new(texture: TextureHandle) -> Handle` — create an image widget

2. Image loading:
   - Load images from file (PNG, JPEG, BMP)
   - Decode to RGBA premultiplied alpha
   - Upload to GPU texture
   - Cache loaded textures (don't reload if already loaded)

3. Fit modes:
   - `Contain` — scale to fit within bounds, preserve aspect ratio
   - `Cover` — scale to cover bounds, preserve aspect ratio, clip overflow
   - `Fill` — stretch to fill bounds, ignore aspect ratio
   - `None` — original size, no scaling

4. Rendering:
   - Image rendered as a textured quad
   - Texture coordinates calculated based on fit mode
   - Opacity applied via blending
   - Rounded corners via shader (clip texture to rounded rect)

5. Texture management:
   - Textures stored in texture atlas if small enough
   - Large textures get their own texture
   - Texture eviction when atlas is full (LRU)

## Acceptance Criteria

- [ ] Image loads from file and displays
- [ ] Contain fit mode works
- [ ] Cover fit mode works
- [ ] Fill fit mode works
- [ ] Opacity is applied
- [ ] Rounded corners work
- [ ] Demo shows an image

## Out of Scope

- SVG rendering (comes later — requires vector graphics)
- Animated images (GIF, APNG — comes later)
- Image loading from network (comes later)
