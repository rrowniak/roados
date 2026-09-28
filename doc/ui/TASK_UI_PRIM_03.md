# TASK_UI_PRIM_03: SDL3 + OpenGL ES 3.1 Context

## Goal

Initialize SDL3, create a window, and create an OpenGL ES 3.1 context.

## Context

The project uses SDL3 for windowing/input and OpenGL ES 3.1 for rendering. This task establishes the rendering context that all subsequent tasks depend on.

## Requirements

1. Implement `ui_core::render::context` module:
   - `Context` struct holding SDL3 window, GL context, and GLES function pointers
   - `Context::new(title, width, height)` — initializes SDL3 video subsystem, creates window, creates GLES 3.1 context
   - `Context::swap()` — swaps buffers
   - `Context::drop()` — cleans up SDL3 and GL resources

2. SDL3 initialization:
   - `SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS | SDL_INIT_GAMECONTROLLER)`
   - Set `SDL_GL_CONTEXT_PROFILE_ES` with major=3, minor=1
   - Set `SDL_GL_DOUBLEBUFFER`, `SDL_GL_DEPTH_SIZE` (0 — no depth buffer needed for 2D UI)
   - Create window with `SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE`
   - Create GL context with `SDL_GL_CreateContext`
   - Make context current with `SDL_GL_MakeCurrent`

3. GLES function loading:
   - Use `glow` with `SDL_GL_GetProcAddress` as the loader function
   - Store the `GlowContext` in the `Context`

4. Verify GLES 3.1 context is active:
   - Check `glGetString(GL_VERSION)` contains "OpenGL ES 3.1"
   - Check `glGetString(GL_RENDERER)` is non-empty

## Acceptance Criteria

- [ ] `Context::new()` creates a window with GLES 3.1 context
- [ ] `glGetString(GL_VERSION)` returns "OpenGL ES 3.1" or higher
- [ ] `Context::swap()` swaps buffers without error
- [ ] Context is properly cleaned up on drop
- [ ] Demo shows a blank window that can be closed

## Out of Scope

- Any drawing (that comes in TASK_UI_PRIM_06)
- Input handling (that comes in TASK_UI_PRIM_10)
- Error handling beyond basic panic on failure
