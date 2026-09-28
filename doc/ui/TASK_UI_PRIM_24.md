# TASK_UI_PRIM_24: Demo Application

## Goal

Build a comprehensive demo application that showcases all UI primitives in action.

## Context

The demo application is the final task. It uses all widgets implemented in previous tasks to create a realistic automotive infotainment interface.

## Requirements

1. Create `ui/src/ui_demo/src/main.rs` with a full demo application:

2. Demo screens:
   - **Home screen**: App launcher grid with icons for each demo section
   - **Controls screen**: Button, Toggle, Slider, Progress widgets
   - **Text screen**: Label (various sizes), TextInput
   - **Navigation screen**: List with scrolling, Tab bar
   - **Information screen**: Gauge, Chart, Vehicle status display
   - **Overlays screen**: Dialog, Toast notifications
   - **Theme switcher**: Toggle between dark and light themes

3. Layout:
   - Top status bar (time, connectivity icons)
   - Bottom navigation bar (tabs for each screen)
   - Content area between status bar and nav bar

4. Interactions:
   - All buttons respond to clicks
   - All toggles switch state
   - All sliders respond to drag
   - List scrolls
   - Text input accepts text
   - Dialog opens and closes
   - Toast appears on button click
   - Theme switcher changes theme with animation

5. Visual quality:
   - Consistent spacing and sizing (from theme)
   - Smooth animations on all interactions
   - Proper touch target sizes (44x44dp minimum)
   - High contrast text
   - Automotive-appropriate styling

6. Performance:
   - 60 FPS target
   - No frame drops during animations
   - Efficient rendering (batched draw calls)

## Acceptance Criteria

- [ ] Demo compiles and runs
- [ ] All widgets are displayed and functional
- [ ] Theme switching works with animation
- [ ] All interactions work (click, drag, scroll, type)
- [ ] 60 FPS is maintained
- [ ] No visual glitches
- [ ] Demo is usable as a reference for roados_ui implementation

## Out of Scope

- Production-quality styling (comes with roados_ui)
- Actual vehicle data integration (comes with roados_ui)
- Navigation system integration (comes with roados_ui)
