# TASK_UI_PRIM_13: Widget — Container

## Goal

Implement the Container widget: a layout-only node that groups children.

## Context

Container is the fundamental composition primitive. It has no visual appearance of its own — it only lays out its children. All other widgets are built on top of Container.

## Requirements

1. Implement `ui_core::widgets::container` module:
   - `Container` widget node with properties: `layout_mode: LayoutMode`, `padding: Padding`, `background: Option<Property<Color>>`, `border_radius: Option<Property<f32>>`
   - `Container::new(mode: LayoutMode) -> Handle` — create a container
   - `Container::add_child(&self, child: Handle)` — add a child widget
   - `Container::remove_child(&self, child: Handle)` — remove a child

2. Layout modes:
   - `Flex { direction: Row | Column, wrap: bool }` — flex layout
   - `Stack` — overlapping children
   - `Absolute` — manual positioning (children position themselves)

3. Padding:
   - `Padding` struct: `left`, `right`, `top`, `bottom`
   - Padding is applied inside the container's bounds
   - Children are laid out within the padded area

4. Optional background:
   - If `background` is set, container renders a rounded rectangle behind children
   - Border radius for rounded corners
   - Background color animates with theme changes

5. Composition:
   - Container can contain any widget (including other containers)
   - Container is the building block for all complex layouts
   - Container itself is invisible unless background is set

## Acceptance Criteria

- [ ] Container lays out children in Row mode
- [ ] Container lays out children in Column mode
- [ ] Container lays out children in Stack mode
- [ ] Padding is applied correctly
- [ ] Background renders behind children when set
- [ ] Border radius rounds the background
- [ ] Nested containers work correctly
- [ ] Demo shows a container with multiple children laid out

## Out of Scope

- Scrolling container (comes in TASK_UI_PRIM_18)
- Animated layout transitions (comes with animation system)
- Container constraints (max/min size — comes with layout system refinement)
