# TASK_UI_PRIM_04: Arena Allocator

## Goal

Implement the arena allocator for widget tree storage.

## Context

All widget nodes live in a single arena. Each node has a `Handle` (index + generation) that remains valid across insertions and deletions. This is the foundation for the entire widget tree.

## Requirements

1. Implement `ui_core::arena` module:
   - `Handle` struct: `{ index: u32, generation: u32 }`
   - `Arena<T>` struct: `Vec<T>`, free list, generation tracking
   - `Arena::new()` — empty arena
   - `Arena::insert(value: T) -> Handle` — insert a value, return its handle
   - `Arena::get(handle: Handle) -> Option<&T>` — get a reference if handle is valid
   - `Arena::get_mut(handle: Handle) -> Option<&mut T>` — get a mutable reference
   - `Arena::remove(handle: Handle) -> Option<T>` — remove and return value
   - `Arena::iter() -> impl Iterator<Item = &T>` — iterate all valid entries
   - `Arena::len() -> usize` — number of valid entries
   - `Arena::is_valid(handle: Handle) -> bool` — check if handle is still valid

2. Handle validity:
   - After removal, old handles become invalid (generation mismatch)
   - New insertions at the same index get a new generation
   - `get`/`get_mut` return `None` for invalid handles

3. Growth:
   - Arena grows geometrically (doubling) when full
   - Handles remain valid across growth (indices don't change)

4. Unit tests:
   - Insert and retrieve
   - Remove and verify handle invalidation
   - Generation increment on reuse
   - Iteration skips removed entries
   - Growth preserves existing handles

## Acceptance Criteria

- [ ] All unit tests pass
- [ ] `cargo test` in `ui_core` succeeds
- [ ] No unsafe code (or minimal, well-commented unsafe)
- [ ] Documentation comments on all public types and methods

## Out of Scope

- Widget node structure (that comes in TASK_UI_PRIM_05)
- Slab allocator for dynamic lists (that comes in TASK_UI_PRIM_18)
- Thread safety (single-threaded by design)
