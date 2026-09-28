# TASK_UI_PRIM_20: Widget — Gauge

## Goal

Implement the Gauge widget: a circular or arc indicator for displaying values.

## Context

Gauge displays a value as a filled arc or circular progress. Common in automotive for speed, RPM, fuel, temperature.

## Requirements

1. Implement `ui_core::widgets::gauge` module:
   - `Gauge` widget node with properties: `value: Property<f32>`, `min: f32`, `max: f32`, `start_angle: f32`, `end_angle: f32`, `gauge_type: GaugeType`
   - `Gauge::new(min: f32, max: f32) -> Handle` — create a gauge

2. Gauge types:
   - `Arc` — partial circle (e.g., 270-degree arc for speedometer)
   - `Circle` — full circle progress
   - `Needle` — arc with a needle indicator

3. Visual structure:
   - Background track: arc showing full range
   - Fill: arc from start to current value position
   - Optional: tick marks, value labels, needle
   - Colors from theme tokens

4. Animation:
   - Fill animates when value changes (smooth transition)
   - Needle animates with spring physics
   - Animation duration from theme tokens

5. Rendering:
   - Arc rendered as a thick line (triangle strip or quad strip)
   - Fill arc rendered with primary color
   - Track rendered with muted color
   - Needle rendered as a triangle or line
   - Anti-aliased edges via SDF or MSAA

## Acceptance Criteria

- [ ] Gauge renders arc with fill proportional to value
- [ ] Value changes animate smoothly
- [ ] Needle type renders and animates
- [ ] Tick marks render correctly
- [ ] Demo shows a gauge at 50%

## Out of Scope

- Digital value display inside gauge (comes with demo app)
- Multiple needles (comes later)
- Gauge with colored zones (red zone, green zone — comes later)
