# TASK_UI_PRIM_07: Layout System

## Goal

Implement the constraint-based layout system.

## Context

Layout is constraint-based (Flutter-style). Parent imposes constraints, child decides size within them. Supports Row, Column, Stack, and manual positioning.

## Requirements

1. Implement `ui_core::layout` module:
   - `Constraints` struct: `min_width`, `max_width`, `min_height`, `max_height`
   - `Size` struct: `width`, `height`
   - `Offset` struct: `x`, `y`
   - `Rect` struct: `origin: Offset`, `size: Size`
   - `LayoutMode` enum: `Absolute`, `Flex { direction, wrap }`, `Grid { columns }`, `Stack`

2. Layout algorithms:
   - `layout_constraints(children: &[Handle], constraints: Constraints, mode: LayoutMode) -> Vec<Rect>`
   - Row: horizontal flex layout
   - Column: vertical flex layout
   - Stack: overlapping children (all at same position)
   - Absolute: manual positioning (no automatic layout)

3. Layout pass:
   - `Layout::layout(root: Handle, constraints: Constraints)` — recursively layout the tree
   - Only layout dirty subtrees (tracked via dirty flags)
   - Cache layout results in `WidgetNode::layout`

4. Flex layout:
   - Distribute available space among children based on flex factors
   - Handle overflow (clip or scroll — clip for now)
   - Support `main_axis_alignment`, `cross_axis_alignment`, `spacing`

5. Unit tests:
   - Row layout with fixed-size children
   - Column layout with flex children
   - Stack layout (all children at origin)
   - Constraint enforcement (min/max)
   - Nested layouts (row inside column)

## Acceptance Criteria

- [ ] All unit tests pass
- [ ] Row, Column, Stack layouts produce correct rects
- [ ] Constraints are enforced (children respect min/max)
- [ ] Flex factors distribute space correctly
- [ ] Nested layouts work correctly

## Out of Scope

- Grid layout (can be added later, similar to flex)
- Animated layout transitions (comes with animation system)
- Virtualized list layout (comes in TASK_UI_PRIM_18)
