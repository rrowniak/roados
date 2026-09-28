# TASK_UI_PRIM_10: Input Handling

## Goal

Implement input handling: SDL3 event processing, hit testing, focus, and gesture recognition.

## Context

Input flows from SDL3 events through hit testing to widget dispatch. The system supports touch, mouse, keyboard, and gamepad input.

## Requirements

1. Implement `ui_core::input` module:
   - `InputEvent` enum: `Tap`, `LongPress`, `Swipe { direction }`, `Pinch { scale }`, `Drag { delta }`, `KeyDown`, `KeyUp`, `Scroll { delta }`
   - `InputEvent` has `position: Option<Offset>` and `consumed: bool`

2. SDL3 event mapping:
   - `SDL_EVENT_FINGER_DOWN` → start tracking potential tap/long-press/drag
   - `SDL_EVENT_FINGER_MOTION` → update drag/swipe
   - `SDL_EVENT_FINGER_UP` → finalize tap/long-press/drag
   - `SDL_EVENT_MOUSE_BUTTON_DOWN/UP` → same as finger
   - `SDL_EVENT_MOUSE_MOTION` → hover tracking
   - `SDL_EVENT_MOUSE_WHEEL` → scroll
   - `SDL_EVENT_KEY_DOWN/UP` → key events
   - `SDL_EVENT_GAMEPAD_BUTTON_DOWN/UP` → gamepad button events
   - `SDL_EVENT_GAMEPAD_AXIS_MOTION` → gamepad axis events

3. Hit testing:
   - `hit_test(root: Handle, position: Offset) -> Option<Handle>` — find deepest node whose bounds contain the point
   - Walk tree top-down (reverse child order for overlapping)
   - Skip invisible nodes

4. Focus system:
   - `Focus` struct tracking currently focused widget
   - `focus_next()`, `focus_prev()` — move focus between focusable widgets
   - Tab key moves focus forward, Shift+Tab backward
   - Steering wheel scroll maps to focus navigation

5. Gesture recognition:
   - Tap: down + up within 300ms, minimal movement (< 10px)
   - Long press: down for > 500ms
   - Swipe: move > 50px in one direction
   - Pinch: two fingers moving apart/together
   - Drag: down + move

6. Event dispatch:
   - `dispatch_event(root: Handle, event: InputEvent)` — route event to hit-tested node
   - Node may consume the event (stop propagation) or pass to parent
   - Bubbling: if not consumed, event goes to parent

## Acceptance Criteria

- [ ] Tap events are dispatched to the correct widget
- [ ] Long press events fire after 500ms
- [ ] Swipe gestures are recognized
- [ ] Focus navigation works with Tab/Shift+Tab
- [ ] Steering wheel scroll moves focus
- [ ] Events bubble to parent if not consumed
- [ ] Unit tests for hit testing and gesture recognition

## Out of Scope

- Multi-touch beyond pinch (e.g., three-finger gestures)
- Voice input (separate system)
- Haptic feedback output (platform-specific)
