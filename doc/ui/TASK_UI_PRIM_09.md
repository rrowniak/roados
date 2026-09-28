# TASK_UI_PRIM_09: Animation System

## Goal

Implement the animation system: property interpolation, easing, and animation clock.

## Context

Animations update `Property` values over time. They do not rebuild the widget tree. The animation clock ticks each frame, updating all active animations.

## Requirements

1. Implement `ui_core::animation` module:
   - `Animation` struct: `property`, `from`, `to`, `start_time`, `duration`, `easing`
   - `AnimationClock` — tracks elapsed time, updates all active animations
   - `Easing` enum: `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`, `Spring { damping, stiffness }`, `Bounce`
   - `Easing::apply(&self, t: f32) -> f32` — apply easing function

2. Animation types:
   - `NumberAnimation` — interpolates `f32` values
   - `ColorAnimation` — interpolates `Color` values (in premultiplied alpha space)
   - `TransformAnimation` — interpolates `Transform` values
   - `Sequence` — chained animations (A then B)
   - `Parallel` — simultaneous animations (A and B together)
   - `Stagger` — delayed sequence across children

3. Animation API:
   - `Property::animate_to(&self, to: T, duration: Duration, easing: Easing)` — start animation
   - `Property::animate_from_to(&self, from: T, to: T, duration: Duration, easing: Easing)`
   - `AnimationClock::tick(&mut self, delta: Duration)` — update all animations
   - `AnimationClock::is_animating(&self) -> bool` — check if any animation is active

4. Frame integration:
   - Animation clock is updated once per frame
   - After ticking, dirty nodes are re-rendered
   - Animations automatically mark dependent nodes dirty

5. Unit tests:
   - Linear easing
   - Ease-in, ease-out, ease-in-out
   - Spring physics
   - Number animation interpolation
   - Color animation interpolation
   - Sequence and parallel composition
   - Stagger timing

## Acceptance Criteria

- [ ] All unit tests pass
- [ ] Animations interpolate values correctly
- [ ] Easing functions produce expected curves
- [ ] Sequences and parallels compose correctly
- [ ] Animations mark dependent nodes dirty
- [ ] Demo shows animated transitions (e.g., button press animation)

## Out of Scope

- Animation curves beyond the defined set
- Physics-based animations (beyond spring)
- Animation serialization/deserialization
