# TASK_UI_PRIM_14: Widget — Slider

## Goal

Implement the Slider widget: a continuous value selector with touch-drag interaction.

## Context

Slider allows selecting a continuous value within a range. It supports touch-drag, keyboard, and gamepad input.

## Requirements

1. Implement `ui_core::widgets::slider` module:
   - `Slider` widget node with properties: `value: Property<f32>`, `min: f32`, `max: f32`, `step: Option<f32>`, `orientation: Orientation`, `on_change: Callback<f32>`
   - `Slider::new(min: f32, max: f32) -> Handle` — create a slider

2. Visual structure:
   - Track: horizontal or vertical line showing the range
   - Fill: colored portion from min to current value
   - Thumb: draggable handle indicating current value
   - Optional: tick marks, value label

3. Interaction:
   - Touch-drag on thumb updates value
   - Touch-drag on track jumps to that position
   - Keyboard: left/right (horizontal) or up/down (vertical) arrows adjust value
   - Gamepad: left stick or d-pad adjusts value
   - Step: if set, value snaps to nearest step
   - Value is clamped to [min, max]

4. Animation:
   - Thumb position animates smoothly when value changes programmatically
   - Track fill animates to match thumb position
   - Thumb scales up slightly when pressed/dragged

5. Rendering:
   - Track: rounded rectangle, muted color
   - Fill: rounded rectangle, primary color
   - Thumb: circle, primary color with border
   - All elements use theme tokens for sizing and color

## Acceptance Criteria

- [ ] Slider renders track, fill, and thumb
- [ ] Dragging thumb updates value
- [ ] Tapping track jumps to that position
- [ ] Keyboard/gamepad adjusts value
- [ ] Step snapping works
- [ ] Value is clamped to [min, max]
- [ ] Thumb position animates smoothly
- [ ] Demo shows a slider that responds to drag

## Out of Scope

- Range slider (two thumbs — comes later)
- Vertical slider (add orientation support)
- Value label (comes with demo app)
