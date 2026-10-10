# TASK_UI_PRIM_17: Widget — Progress

## Goal

Implement the Progress bar widget: shows completion progress with animation.

## Context

Progress bar displays a value as a filled portion of a track. It supports determinate and indeterminate modes.

## Requirements

1. Implement `ui_core::widgets::progress` module:
   - `Progress` widget node with properties: `value: Property<f32>`, `orientation: Orientation`, `indeterminate: bool`
   - `Progress::new() -> Handle` — create a progress bar

2. Visual structure:
   - Track: rounded rectangle, muted color
   - Fill: rounded rectangle, primary color, width proportional to value
   - Indeterminate: animated fill that slides across the track

3. Modes:
   - Determinate: fill width = value * track_width (value in [0, 1])
   - Indeterminate: fill slides left to right repeatedly (animation loop)

4. Animation:
   - Fill width animates when value changes (smooth transition)
   - Indeterminate: continuous slide animation
   - Animation duration from theme tokens

5. Rendering:
   - Track: rounded rectangle
   - Fill: rounded rectangle, clipped to track bounds
   - Smooth animation between values
   - Respects theme tokens for sizing and color

## Acceptance Criteria

- [ ] Progress bar renders track and fill
- [ ] Fill width reflects value
- [ ] Value changes animate smoothly
- [ ] Indeterminate mode shows sliding animation
- [ ] Demo shows a progress bar at 50%

## Out of Scope

- Circular progress indicator (comes later)
- Progress with label (comes with demo app)
- Segmented progress (comes later)
