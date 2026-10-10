# TASK_UI_PRIM_08: Theme System

## Goal

Implement the theme system with runtime switching and animated transitions.

## Context

A theme is a struct of `Property` values. Widgets reference theme properties through the property graph. Changing the theme updates all dependent widgets automatically.

## Requirements

1. Implement `ui_core::theme` module:
   - `ThemeToken` enum (all tokens from PRIMITIVES_ARCHITECTURE.md)
   - `Theme` struct with a `Property` for each token
   - `Theme::new()` — default theme (dark)
   - `Theme::get(&self, token: ThemeToken) -> PropertyValue`
   - `Theme::set(&self, token: ThemeToken, value: PropertyValue)`
   - `Theme::switch_to(&self, new_theme: Theme, duration_ms: u32)` — animated transition

2. Theme tokens:
   - Colors: `Background`, `Surface`, `Primary`, `OnPrimary`, `Text`, `TextMuted`, `Border`, `Error`, `Warning`, `Success`
   - Spacing: `SpacingXs`, `SpacingSm`, `SpacingMd`, `SpacingLg`, `SpacingXl`
   - Typography: `FontFamily`, `FontSizeXs`..`FontSizeXl`, `FontWeightNormal`, `FontWeightBold`
   - Shape: `BorderRadiusSm`, `BorderRadiusMd`, `BorderRadiusLg`, `BorderWidth`
   - Motion: `DurationFast`, `DurationNormal`, `DurationSlow`, `EasingStandard`, `EasingDecelerate`, `EasingAccelerate`

3. Runtime theme switching:
   - `Theme::switch_to` animates each token from old to new value
   - Uses `Property::animate` for each token
   - Property graph propagates changes to all dependent widgets

4. Theme variants:
   - `Theme::dark()` — default dark theme
   - `Theme::light()` — light theme
   - Both define all tokens

5. Unit tests:
   - Theme creation and token access
   - Theme switching updates all tokens
   - Animated transition produces intermediate values

## Acceptance Criteria

- [ ] All unit tests pass
- [ ] Dark and light themes are defined with all tokens
- [ ] Theme switching updates all dependent properties
- [ ] Animated transition interpolates values correctly
- [ ] Demo can switch between dark and light themes at runtime

## Out of Scope

- Custom theme creation UI (comes with demo app)
- Theme persistence (save/load from file)
- CSS-like stylesheet parsing
