# TASK_UI_PRIM_05: Property System

## Goal

Implement the reactive property system that powers theming, animation, and data flow.

## Context

Every visual aspect of a widget is a `Property<T>`. Properties form a DAG of dependencies. When a property changes, all dependent properties and render nodes are marked dirty. This is the core of the theming and animation systems.

## Requirements

1. Implement `ui_core::property` module:
   - `Property<T>` struct with value and dependency tracking
   - `Property::new(value: T) -> Property<T>` — literal property
   - `Property::bind<F>(f: F) -> Property<T> where F: Fn() -> T` — computed property
   - `Property::get(&self) -> T` — get current value
   - `Property::set(&self, value: T)` — set value, mark dependents dirty
   - `Property::animate(from, to, duration, easing) -> Property<T>` — animated property
   - `PropertyTracker` — tracks which properties depend on which
   - `PropertyHandle` — reference to a property (for use in widget nodes)

2. Dependency tracking:
   - When `Property::bind` is called, it registers a dependency on any properties accessed during evaluation
   - When a property changes, all properties that depend on it are re-evaluated
   - Cycles are detected and panic (or return error)

3. Change notification:
   - `Property::set` marks all dependent render nodes as dirty
   - Uses a callback or observer pattern (e.g., `on_change: Vec<Callback>`)
   - Callbacks are called after the value is updated

4. Property types needed:
   - `Property<f32>` — numeric values (opacity, size, position)
   - `Property<Color>` — colors (with premultiplied alpha)
   - `Property<bool>` — boolean states
   - `Property<String>` — text content
   - `Property<Transform>` — 2D transforms

5. Unit tests:
   - Literal property get/set
   - Bound property recomputes when dependency changes
   - Multiple levels of binding (A depends on B depends on C)
   - Cycle detection
   - Change notification fires

## Acceptance Criteria

- [ ] All unit tests pass
- [ ] `cargo test` in `ui_core` succeeds
- [ ] Property graph correctly propagates changes
- [ ] No memory leaks (properties don't hold stale references)
- [ ] Documentation comments on all public types and methods

## Out of Scope

- ~~Widget node structure (uses properties but doesn't define them)~~ —
  **corrected 2026-10-05.** True as far as it goes — this task did not define the
  node structure — and misleading because it leaves the impression that some
  other task did. Task 04 pointed at *this* task for it, and this task pointed at
  nothing, so two consecutive tasks each excluded it and none claimed it.
  **`ui_core/src/node.rs` came from TASK_UI_PRIM_02** (commit `89b67b7`), which
  specified the module structure; task 04 is `a8f3147` and this task is
  `8c3657b`, both later. The actual structure is four fields — `children`,
  `parent`, `layout`, `paint` — with no `kind`, no `properties` and no `flags`,
  and no `PropertySet` type exists. `PRIMITIVES_ARCHITECTURE.md` carried a
  seven-field sketch until 2026-10-05; see its *Widget Tree* section for the
  corrected one and why the type is smaller than the design suggested.
- Theme system (uses properties, comes in TASK_UI_PRIM_08)
- Animation clock (drives animated properties, comes in TASK_UI_PRIM_09)
