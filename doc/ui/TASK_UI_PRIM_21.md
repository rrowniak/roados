# TASK_UI_PRIM_21: Widget — Chart

## Goal

Implement the Chart widget: displays time-series data as a line or bar chart.

## Context

Chart displays data visually. Common in automotive for energy consumption, speed history, temperature trends.

## Requirements

1. Implement `ui_core::widgets::chart` module:
   - `Chart` widget node with properties: `data: Vec<f32>`, `chart_type: ChartType`, `x_labels: Vec<String>`, `y_labels: Vec<String>`
   - `Chart::new(chart_type: ChartType) -> Handle` — create a chart

2. Chart types:
   - `Line` — connected line segments
   - `Bar` — vertical bars
   - `Area` — line with filled area below

3. Visual structure:
   - Axes: x and y axis lines
   - Grid: optional grid lines
   - Data: line/bars/area rendered with primary color
   - Labels: x and y axis labels (optional)
   - Colors from theme tokens

4. Data handling:
   - Data is a `Vec<f32>` of values
   - Y-axis auto-scales to data range (or fixed range)
   - X-axis is evenly spaced
   - Data can be updated (append new value, shift old values)

5. Animation:
   - New data points animate in (line grows, bar rises)
   - Data updates are smooth (no jarring jumps)
   - Animation duration from theme tokens

6. Rendering:
   - Line: triangle strip or line strip
   - Bars: individual rectangles
   - Area: filled polygon below line
   - Anti-aliased edges

## Acceptance Criteria

- [ ] Line chart renders data correctly
- [ ] Bar chart renders data correctly
- [ ] Area chart renders data correctly
- [ ] Axes and labels render
- [ ] New data animates in smoothly
- [ ] Demo shows a chart with sample data

## Out of Scope

- Multiple data series (comes later)
- Interactive zoom/pan (comes later)
- Real-time scrolling chart (comes later)
