# TASK_UI_PRIM_15: Widget — Toggle

## Goal

Implement the Toggle (switch) widget: a binary state control with animated transition.

## Context

Toggle is a binary on/off control. It has an animated thumb that slides between positions.

## Requirements

1. Implement `ui_core::widgets::toggle` module:
   - `Toggle` widget node with properties: `checked: Property<bool>`, `on_change: Callback<bool>`
   - `Toggle::new() -> Handle` — create a toggle

2. Visual structure:
   - Track: rounded rectangle (pill shape)
   - Thumb: circle that slides left/right
   - Track color changes: off (muted) / on (primary)
   - Thumb color: white or on-primary

3. Interaction:
   - Tap/click toggles the state
   - Keyboard: Enter/Space toggles when focused
   - Gamepad: A button toggles when focused
   - Toggle consumes tap events

4. Animation:
   - Thumb slides from left to right (or vice versa) over `DurationFast`
   - Track color interpolates between off/on colors
   - Thumb has slight scale bounce on toggle
   - All animations use theme easing curves

5. Rendering:
   - Track: pill-shaped rounded rectangle
   - Thumb: circle with shadow
   - Smooth animation between states
   - Respects theme tokens for sizing

## Acceptance Criteria

- [ ] Toggle renders track and thumb
- [ ] Tap toggles the state
- [ ] Thumb slides animated between positions
- [ ] Track color changes between off/on
- [ ] Keyboard/gamepad toggles when focused
- [ ] Demo shows a toggle that responds to clicks

## Out of Scope

- Toggle with label (comes with demo app)
- Tri-state toggle (comes later)
- Toggle group (comes later)
