# TASK_UI_PRIM_06: Rendering Pipeline

## Goal

Implement the rendering pipeline: draw command recording, batching, and GLES submission.

## Context

The rendering pipeline takes dirty widget nodes, records draw commands, batches them by material/blend mode, and submits to GPU. This is the core of the visual output.

## Requirements

1. Implement `ui_core::paint` module:
   - `DrawCommand` enum: `Rect`, `RoundedRect`, `Text`, `Image`, `Line`, `Circle`, `Path`
   - `PaintState` per widget node: cached draw commands, dirty flags
   - `Painter` — records draw commands for a widget subtree

2. Implement `ui_core::batch` module:
   - `Batch` — a group of draw commands with the same material/blend mode
   - `Batcher` — groups draw commands into batches
   - Batching keys: texture atlas, blend mode, shader type
   - Opaque/transparent separation (two passes)

3. Implement `ui_core::render` module (GLES backend):
   - `Renderer` — submits batched draw commands to GPU
   - Shader programs: solid color, gradient, image (texture), text (SDF)
   - Vertex buffer management with geometry retention
   - Texture atlas management
   - `glBlendFunc` setup for premultiplied alpha: `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`
   - Scissor test for clipping

4. Frame lifecycle:
   - `Renderer::begin_frame()` — clear screen, reset batches
   - `Renderer::draw_node(handle)` — record draw commands for a node
   - `Renderer::end_frame()` — submit batches to GPU, swap buffers

5. Transparency:
   - Opaque pass: front-to-back, no blending
   - Transparent pass: back-to-front, blending enabled
   - Layer boundaries for opacity < 1.0, transforms, clips

## Acceptance Criteria

- [ ] Solid color rectangles render correctly
- [ ] Rounded rectangles render correctly
- [ ] Transparent rectangles blend correctly (premultiplied alpha)
- [ ] Multiple draw calls with same material are batched
- [ ] Opaque and transparent passes are separated
- [ ] `cargo test` passes (unit tests for batching logic)
- [ ] Demo shows colored rectangles on screen

## Out of Scope

- Text rendering (needs font atlas, comes with Label widget in TASK_UI_PRIM_11)
- Image rendering (comes with Image widget in TASK_UI_PRIM_16)
- Texture atlas population (comes with widgets that use it)
- Layout (comes in TASK_UI_PRIM_07)
