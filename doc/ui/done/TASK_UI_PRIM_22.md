# TASK_UI_PRIM_22: Widget — Dialog

## Goal

Implement the Dialog widget: a modal overlay with title, body, and actions.

## Context

Dialog is a modal overlay that appears on top of the current content. It has a title, body text, and action buttons.

## Requirements

1. Implement `ui_core::widgets::dialog` module:
   - `Dialog` widget node with properties: `title: Property<String>`, `body: Property<String>`, `actions: Vec<DialogAction>`, `visible: Property<bool>`
   - `Dialog::new(title: impl Into<String>, body: impl Into<String>) -> Handle`

2. Visual structure:
   - Overlay: semi-transparent black background (dims content behind)
   - Panel: rounded rectangle, surface color, centered on screen
   - Title: bold text at top of panel
   - Body: regular text below title
   - Actions: row of buttons at bottom (e.g., "OK", "Cancel")

3. Interaction:
   - Dialog is modal: blocks input to content behind it
   - Tap outside dialog dismisses it (optional — configurable)
   - Action buttons fire callbacks and dismiss dialog
   - Escape key dismisses dialog

4. Animation:
   - Dialog fades in (overlay opacity 0 → 0.5)
   - Panel scales in (0.9 → 1.0) with bounce easing
   - Dialog fades out and scales out on dismiss
   - Animation duration from theme tokens

5. Rendering:
   - Overlay rendered first (full screen, semi-transparent)
   - Panel rendered on top (centered)
   - Panel has shadow (rendered as blurred rounded rect behind panel)
   - Content behind dialog is not re-rendered (cached)

## Acceptance Criteria

- [ ] Dialog renders overlay and panel
- [ ] Title and body text render correctly
- [ ] Action buttons render and respond to clicks
- [ ] Dialog dismisses on action button click
- [ ] Dialog dismisses on escape key
- [ ] Dialog animates in and out
- [ ] Demo shows a dialog with "OK" and "Cancel" buttons

## Out of Scope

- Dialog with text input (comes later)
- Dialog with list selection (comes later)
- Stacked dialogs (dialog on top of dialog — comes later)
- Dialog with custom content (comes later)
