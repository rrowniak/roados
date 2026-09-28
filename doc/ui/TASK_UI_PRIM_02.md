# TASK_UI_PRIM_02: Project Scaffolding

## Goal

Create the Cargo workspace with `ui_core` and `ui_demo` crates.

## Context

The UI library lives in `ui/src/ui_core/`. The demo application lives in `ui/src/ui_demo/`. Both are part of a Cargo workspace.

## Requirements

1. Create `ui/Cargo.toml` workspace root:
   ```toml
   [workspace]
   members = ["src/ui_core", "src/ui_demo"]
   resolver = "2"
   ```

2. Create `ui/src/ui_core/Cargo.toml`:
   - Dependencies: `sdl3` (with `build-from-source`), `glow`
   - Module structure per PRIMITIVES_ARCHITECTURE.md

3. Create `ui/src/ui_demo/Cargo.toml`:
   - Dependencies: `ui_core`, `sdl3`, `glow`
   - Binary target

4. Create `ui/src/ui_core/src/lib.rs` with module declarations:
   - `arena`, `node`, `property`, `theme`, `layout`, `paint`, `batch`, `render`, `animation`, `input`
   - `widgets` module

5. Create `ui/src/ui_demo/src/main.rs` with a minimal "Hello World" that initializes SDL3 and creates a window (placeholder for future tasks)

## Acceptance Criteria

- [ ] `cargo build` succeeds in `ui/` directory
- [ ] `cargo run` in `ui/src/ui_demo/` opens a blank SDL3 window
- [ ] Module structure matches PRIMITIVES_ARCHITECTURE.md
- [ ] No compiler warnings

## Out of Scope

- Any actual UI functionality (that comes in later tasks)
- CMake integration (handled by sdl3-sys build-from-source)
