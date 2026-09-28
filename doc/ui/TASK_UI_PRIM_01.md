# TASK_UI_PRIM_01: Crossbuild Environment Setup

## Goal

Document and set up the build environment for both native x86_64 and cross-compiled aarch64 targets.

## Context

The project targets aarch64 and x86_64 Linux with OpenGL ES 3.1. SDL3 is built from vendored source via `sdl3-sys`. Cross-compilation to aarch64 requires a CMake toolchain file and proper environment variables.

## Requirements

1. Create `doc/ui/CROSSBUILD.md` with:
   - Prerequisites (rustup, cross-compilation toolchain, cmake, pkg-config)
   - Native x86_64 build instructions
   - aarch64 cross-compilation instructions (toolchain file, env vars, CMake flags)
   - SDL3 vendored build configuration (which subsystems to disable)
   - Troubleshooting common issues

2. Create `cmake/aarch64-toolchain.cmake` for cross-compilation

3. Verify the setup works:
   - `cargo build` succeeds for x86_64
   - `cargo build --target aarch64-unknown-linux-gnu` succeeds (once project exists)

## Acceptance Criteria

- [ ] `doc/ui/CROSSBUILD.md` exists with complete instructions
- [ ] `cmake/aarch64-toolchain.cmake` exists and is correct
- [ ] Native build instructions are tested and working
- [ ] Cross-compilation instructions are tested and working (can be verified with a minimal test crate if project scaffolding not yet done)

## Out of Scope

- Docker/container setup (may be added later)
- CI pipeline configuration
- Windows/macOS build instructions
