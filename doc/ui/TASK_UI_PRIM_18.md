# TASK_UI_PRIM_18: Widget — List/Scroll

## Goal

Implement the List and Scroll widgets: scrollable containers with virtualized rendering.

## Context

List displays a scrollable set of items. Only visible items are rendered (virtualization). Scroll supports touch-drag, mouse wheel, and keyboard.

## Requirements

1. Implement `ui_core::widgets::list` and `ui_core::widgets::scroll` modules:
   - `List` widget node with properties: `item_count: usize`, `item_height: f32`, `scroll_offset: Property<f32>`, `on_item_click: Callback<usize>`
   - `List::new(item_count: usize, item_height: f32) -> Handle`
   - `Scroll` widget node with properties: `content: Handle`, `scroll_offset: Property<f32>`
   - `Scroll::new(content: Handle) -> Handle`

2. Virtualization:
   - Only visible items are allocated in the arena
   - Items scrolling out of view are returned to free list
   - Items scrolling in are allocated from free list
   - Scroll offset determines which items are visible

3. Scrolling:
   - Touch-drag scrolls content
   - Mouse wheel scrolls content
   - Keyboard: up/down arrows scroll
   - Gamepad: left stick or d-pad scrolls
   - Scroll offset is clamped to [0, max_scroll]
   - Smooth scrolling with momentum (optional)

4. Layout:
   - List lays out items vertically (or horizontally)
   - Each item has fixed height (or variable height — stretch goal)
   - Scroll offset shifts item positions

5. Rendering:
   - Only visible items are rendered
   - Clipping: items outside viewport are not drawn
   - Scissor test for efficient clipping

## Acceptance Criteria

- [ ] List renders visible items
- [ ] Scrolling works via touch-drag
- [ ] Scrolling works via mouse wheel
- [ ] Virtualization: only visible items are allocated
- [ ] Items recycle when scrolling
- [ ] Scroll offset is clamped
- [ ] Demo shows a scrollable list of 100 items

## Out of Scope

- Variable item heights (comes later)
- Grid list (multi-column — comes later)
- Pull-to-refresh (comes later)
- Sticky section headers (comes later)
