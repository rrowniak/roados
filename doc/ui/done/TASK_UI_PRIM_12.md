# TASK_UI_PRIM_12: Widget — Button

## Goal

Implement the Button widget with press/hover/focus states and animated transitions.

## Context

Button is the primary interactive control. It has visual states (default, hovered, pressed, disabled, focused) with animated transitions between them.

## Requirements

1. Implement `ui_core::widgets::button` module:
   - `Button` widget node with properties: `label: Property<String>`, `background: Property<Color>`, `foreground: Property<Color>`, `border_radius: Property<f32>`, `on_click: Callback`
   - `Button::new(label: impl Into<String>) -> Handle` — create a button

2. Visual states:
   - `Default` — normal appearance
   - `Hovered` — mouse/touch hover (slightly lighter background)
   - `Pressed` — actively pressed (darker background, slight scale down)
   - `Disabled` — grayed out, no interaction
   - `Focused` — keyboard/gamepad focus indicator (border highlight)

3. Animated transitions:
   - Background color animates between states (duration from theme `DurationFast`)
   - Scale animates on press (0.95x) and release (1.0x)
   - Opacity animates for disabled state
   - All transitions use theme easing curves

4. Interaction:
   - Tap/click triggers `on_click` callback
   - Button consumes tap events (doesn't propagate to parent)
   - Button is focusable (can receive keyboard/gamepad focus)
   - Enter/Space/Gamepad-A activates focused button

5. Layout:
   - Button sizes to fit label text + padding
   - Padding from theme tokens (`SpacingSm` horizontal, `SpacingXs` vertical)
   - Minimum touch target size: 44x44dp (from theme)

6. Rendering:
   - Rounded rectangle background (border radius from theme)
   - Label centered inside button
   - Focus indicator: border or glow
   - Pressed state: slight inner shadow or darker overlay

## Acceptance Criteria

- [ ] Button renders with label centered
- [ ] Button responds to tap/click (on_click fires)
- [ ] Visual states are distinct (default, hovered, pressed, disabled, focused)
- [ ] State transitions are animated
- [ ] Button is focusable and activatable via keyboard
- [ ] Button respects minimum touch target size
- [ ] Demo shows a button that responds to clicks

## Out of Scope

- Icon buttons (icon + label — comes later)
- Toggle buttons (comes with Toggle in TASK_UI_PRIM_15)
- Button groups (radio buttons, checkbox groups)
