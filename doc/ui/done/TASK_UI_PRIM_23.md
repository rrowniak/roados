# TASK_UI_PRIM_23: Widget — Toast

## Goal

Implement the Toast widget: a non-blocking notification that auto-dismisses.

## Context

Toast is a small notification that appears temporarily and auto-dismisses. It does not block input to other widgets.

## Requirements

1. Implement `ui_core::widgets::toast` module:
   - `Toast` widget node with properties: `message: Property<String>`, `duration: Duration`, `visible: Property<bool>`
   - `Toast::show(message: impl Into<String>, duration: Duration) -> Handle`

2. Visual structure:
   - Background: rounded rectangle, surface color with slight transparency
   - Icon: optional icon (info, warning, error, success)
   - Message: text content
   - Position: bottom of screen (or top — configurable)

3. Behavior:
   - Toast appears with fade-in animation
   - Toast auto-dismisses after `duration`
   - Toast dismisses with fade-out animation
   - Toast does not block input to other widgets
   - Multiple toasts stack vertically (newest at bottom)

4. Animation:
   - Fade in: opacity 0 → 1 over `DurationFast`
   - Slide in: translateY(20px) → translateY(0) over `DurationFast`
   - Fade out: opacity 1 → 0 over `DurationFast`
   - Slide out: translateY(0) → translateY(20px) over `DurationFast`

5. Rendering:
   - Toast rendered as overlay (on top of all other content)
   - Toast has shadow
   - Toast respects theme tokens for sizing and color

## Acceptance Criteria

- [ ] Toast appears with message
- [ ] Toast auto-dismisses after duration
- [ ] Toast animates in and out
- [ ] Toast does not block input
- [ ] Multiple toasts stack correctly
- [ ] Demo shows a toast notification

## Out of Scope

- Toast with action button (comes later)
- Toast with progress bar (comes later)
- Toast queue management (comes later)
