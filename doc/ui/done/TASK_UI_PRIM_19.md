# TASK_UI_PRIM_19: Widget — TextInput

## Goal

Implement the TextInput widget: single-line text entry with cursor and selection.

## Context

TextInput allows the user to enter and edit text. It includes a cursor, text selection, and an on-screen keyboard.

## Requirements

1. Implement `ui_core::widgets::text_input` module:
   - `TextInput` widget node with properties: `text: Property<String>`, `placeholder: Property<String>`, `focused: Property<bool>`, `on_change: Callback<String>`, `on_submit: Callback<String>`
   - `TextInput::new() -> Handle` — create a text input

2. Visual structure:
   - Background: rounded rectangle, surface color
   - Border: highlighted when focused
   - Text: rendered using Label's text rendering pipeline
   - Cursor: blinking vertical line at insertion point
   - Placeholder: shown when text is empty (muted color)

3. Interaction:
   - Tap to focus and position cursor
   - Keyboard input inserts characters
   - Backspace deletes character before cursor
   - Delete deletes character after cursor
   - Left/right arrows move cursor
   - Home/End move to start/end
   - Enter submits (fires `on_submit`)

4. On-screen keyboard:
   - Custom automotive-optimized keyboard layout
   - Large touch targets (minimum 44x44dp)
   - QWERTY layout with numbers and symbols
   - Backspace, enter, space keys
   - Keyboard appears when TextInput is focused

5. Cursor animation:
   - Cursor blinks (visible for 500ms, hidden for 500ms)
   - Cursor position animates when moved
   - Cursor color from theme

6. Rendering:
   - Text rendered with SDF glyph atlas (same as Label)
   - Cursor rendered as a thin rectangle
   - Selection highlighted with rounded rectangle
   - Clipping: text is clipped to input bounds

## Acceptance Criteria

- [ ] TextInput renders with placeholder
- [ ] Tap focuses the input
- [ ] Keyboard input inserts characters
- [ ] Backspace deletes characters
- [ ] Cursor blinks
- [ ] Cursor moves with arrow keys
- [ ] On-screen keyboard appears when focused
- [ ] Demo shows a text input that accepts text

## Out of Scope

- Multi-line text input (comes later)
- Rich text editing (comes later)
- Copy/paste (comes later)
- Input validation (comes later)
