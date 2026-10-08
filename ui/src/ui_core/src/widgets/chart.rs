//! The Chart widget: a series of readings drawn over two axes.
//!
//! A chart is a node, a truth, a drawn series, and a pile of quads.
//! [`Chart::data`] is the truth — a `Vec<f32>` of readings, oldest first — and
//! [`Chart::shown`] is what is *drawn*, animated toward it, and the distance
//! between the two is the transition. That split is
//! [`slider`](crate::widgets::slider::Slider)'s and
//! [`gauge`](crate::widgets::gauge::Gauge)'s and
//! [`progress`](crate::widgets::progress::Progress)'s, for the reason all three
//! give: a caller writing `data` from a sensor wants the line to *arrive* at the
//! reading rather than jump to it, and requirement 4's "data updates are smooth
//! (no jarring jumps)" is the drawn half of that pair.
//!
//! Three types, one widget. [`ChartType::Line`] is connected segments,
//! [`ChartType::Bar`] is one rectangle per sample, and [`ChartType::Area`] is a
//! line with the region under it filled.
//!
//! **The type is a plain field behind a setter**, though requirement 1 lists
//! `chart_type` among the properties, and for [`GaugeType`](crate::widgets::gauge::GaugeType)'s reason:
//! nothing animates *which* shape a chart is in. A chart does not cross-fade from
//! a line into bars; it is one or the other, and which one is a fact about the
//! data rather than a step in a transition. A `Property<ChartType>` would be one
//! whose only writes are a caller's, read through `get()`, and free to change on
//! its own in the middle of a frame — leaving the mode disagreeing with the
//! property actually drawn.
//!
//! # The drawn series is two arrays, and why it cannot be one
//!
//! [`Series`] carries an `x` beside every value, and that is the whole reason
//! an append is smooth. Appending a sample to evenly spaced data **re-spaces
//! every sample already there**: three samples across a plot put the second at
//! `0.5` and four put it at `0.333`. A chart that animated only the *values*
//! would therefore show a correct-looking series in the wrong places, from the
//! first frame of the transition to the last. Animating the positions as well is
//! what makes the existing three *stay put* and the line grow to the right:
//!
//! ```text
//!   data  [a, b, c]        x  [0.0, 0.5, 1.0]
//!   data  [a, b, c, d]     x  [0.0, 0.333, 0.667, 1.0]
//!   the new sample enters at x = 1.0 — where the old run's last sample was —
//!   and everything slides into place underneath it.
//! ```
//!
//! A **shift** — the other half of requirement 4's "append a new value, shift
//! old values" — needs no geometry at all. `[a,b,c,d]` becoming `[b,c,d,e]` is
//! four samples at four fixed positions whose values glide from each one's own to
//! its neighbour's, which is a series sliding one slot to the left: the picture
//! at the start of the transition is the picture before it, to the float.
//!
//! # The line is mitred per segment, and the number behind it
//!
//! Requirement 6 asks for "triangle strip or line strip". Neither is reachable
//! without a new vertex type, a new shader and a change to the index buffer, so
//! the series is drawn as **one convex four-point [`DrawCommand::Polygon`] per
//! segment**, with the two ends of each segment offset from the segment's own
//! direction. [`DrawCommand::Path`] would be one command for the whole series
//! and is wrong for this, and the reason is worth being exact about because it is
//! *not* the reason it is wrong for the gauge.
//!
//! A `Path` gets the **thickness right**. `line_quad` offsets each segment
//! **perpendicular** to it, so a stroke of `w` is `w` across every segment —
//! there is no error in the width of a run at any angle. What it gets wrong is
//! the **joins**: two flat caps meeting at an angle leave the outer side of a
//! turn with an empty wedge, and the depth of that wedge is
//! `half·(1/cos(φ/2) − cos(φ/2))` where `φ` is the turn. Measured on a 3-pixel
//! line, from the closed form rather than from a capture, and it is small for a
//! gentle turn and catastrophic for a sharp one:
//!
//! | turn | wedge missing | share of a 3 px line |
//! |---:|---:|---:|
//! | 15° | 0.026 px | 0.9% |
//! | 30° | 0.104 px | 3.5% |
//! | 45° | 0.238 px | 7.9% |
//! | 60° | 0.433 px | 14.4% |
//! | 90° | 1.061 px | 35.4% |
//! | 120° | 2.250 px | 75.0% |
//!
//! Near zero it is second order — `half·φ²/4` in radians, which is 0.12743 px
//! against the same formula's exact 0.12932 px at 33.4°, a gap of 0.0019 px — so a chart of gentle readings is
//! nearly right on a `Path` and a chart of any real spikiness is not. The gauge's
//! number is a different defect and a different geometry: an arc band wants a
//! *radial* thickness, and a chord offset perpendicular to itself puts its outer
//! corner at `sqrt(R² + r²)` rather than `R + r`. A line chart's centre line is
//! the data, and perpendicular is exactly right for it.
//!
//! **The mitre is `w = half·(p + q)/(1 + p·q)`**, where `p` and `q` are the two
//! unit normals either side of the vertex. It is the offset that sits `half` from
//! *both* segments, and its length is `half/cos(φ/2)`: `half` on a straight run,
//! `1.414·half` at a right angle, and at the two angles quoted below this paragraph
//! — **`9.5509·half` at 167.98° and `9.5668·half` at 168°**, which agree to two
//! figures and were once quoted as one. **What would reverse this
//! choice** is a real stroked-polyline primitive — a join rule, a cap, and a
//! vertex type that carries a direction — which would draw the run in one pass
//! with neither the fan nor the fan's convexity precondition.
//!
//! # A mitre past `MITRE_LIMIT` becomes a disc, and the number behind that
//!
//! `half/cos(φ/2)` is unbounded as `φ` approaches a full reversal, and a chart's
//! slopes are not bounded: a 20-sample chart on a 600×300 plot has a 31.6 px
//! x-spacing, so a peak whose two sides rise the plot's full 300 px turns
//! **167.98°**, whose mitre is `9.55·half` — a 14.3-pixel spike on a 3-pixel
//! line. So the mitre is taken only while it stays inside `MITRE_LIMIT`, and
//! past that the join falls back to **each segment's own perpendicular offset
//! plus a [`DrawCommand::Circle`] of radius `half` at the vertex**.
//!
//! **The disc fills the corner exactly and adds nothing outside it.** The wedge
//! the flat caps leave empty is the triangle `v, v + half·p, v + half·q`, and
//! both of its outer vertices are at distance `half` from `v` while its third
//! vertex is `v` itself — so the triangle lies inside the disc of radius `half`
//! about `v`, touching its boundary at exactly those two points. Those two points
//! are also where each segment's outer edge line touches the disc, so the disc
//! never reaches past either edge. That is SVG's own rule for a miter limit, and
//! it keeps **every segment at exactly four points**, which is what
//! [`DrawCommand::Polygon`]'s convex fan wants.
//!
//! `MITRE_LIMIT` is 4, which is SVG's default, and the turn at which it bites
//! is `2·acos(1/4)` = **151.045°** — a turn so sharp that each side rises
//! `tan(75.522°)·slot` = **3.873 slots**. What would reverse it is a measurement
//! that a spikier chart than that ships.
//!
//! # Every quad is convex, and the rule that makes it so
//!
//! [`DrawCommand::Polygon`] fans from its first point and the fan is exact for a
//! convex polygon, so convexity is the primitive's precondition rather than a
//! nicety. Each segment's four corners are `a + A`, `b + B`, `b − B`, `a − A`,
//! where `A` and `B` are the joins at its two ends, and **both offsets sit `half`
//! from the segment's own centre line** — the mitre by construction
//! (`w·p = half(p·p + q·p)/(1 + p·q) = half`) and the flat offset because it *is*
//! `half·n`. In a frame whose `y` axis is that normal the four corners are
//! `(t_A, half)`, `(L + t_B, half)`, `(L − t_B, −half)`, `(−t_A, −half)`, and
//!
//! - the first and last turns carry the factor `−2·half·(L + t_B − t_A)`;
//! - the two in between carry `−2·half·(L + t_A − t_B)`.
//!
//! **Those two expressions are not the same, which is the whole of it.** They
//! agree in sign — which is convexity — exactly when **`|t_A − t_B| < L`**: the
//! two offsets' displacement *along* the segment must differ by less than the
//! segment's own length. The first draft of this module claimed they were the
//! same expression and therefore that every quad was convex for any offsets at
//! all; a search over two hundred thousand geometries found the counterexample on
//! its thirty-eighth, and the claim was wrong.
//!
//! The counterexample is not exotic. A mitre's along-axis swing is
//! `half·tan(φ/2)`, and a chart's segment length is `pitch·√(1 + slope²)`, so on
//! a 600-pixel plot a **201-sample** chart has a 3-pixel pitch and at a 90-degree
//! turn the swing is `1.5` px — **half the segment's own length**, at which the
//! two factors are already opposite. `MITRE_LIMIT` does not catch it: that mitre
//! is `1.414·half`, comfortably inside the limit.
//!
//! **So the rule is `|t| < L/2` per side of a vertex**, which is stricter than
//! the `|t_A − t_B| < L` it needs and is what lets the decision be made **per
//! vertex, in one pass**: the triangle inequality gives `|t_A − t_B| ≤ |t_A| +
//! |t_B| < L`, and a segment's two ends are the two ends of one of a vertex's
//! sides. A vertex whose corner either limit or this rule rejects gets
//! `Join::Flat` — each segment's own perpendicular plus a disc — and its quad is
//! a rectangle, which is convex for any length.
//!
//! It costs nothing on a chart a person would read: `|t| ≤ |corner| ≤
//! MITRE_LIMIT·half = 4·half = 6` px, so no corner is affected while the pitch
//! stays above **12 px — 50 samples on a 600-pixel plot**, which is `600/(n−1) >
//! 12`, so `n = 50` at 12.245 px and `n = 51` at exactly 12, which is not above
//! it. Below that the
//! segments really are shorter than the mitres, and rounding those corners is the
//! right answer rather than a fallback.
//! `every_segment_quad_is_convex_over_two_hundred_thousand_geometries` asserts
//! the rule against the search that found the defect.
//!
//! # The area fill is per-segment quads too, and it has to be
//!
//! The shape under an area chart's line is concave — it is a curve with a
//! baseline along the bottom of it — so one [`DrawCommand::Polygon`] for the
//! whole region fans into triangles that overlap it and triangles outside it.
//! Requirement 6's "filled polygon below line" is therefore drawn as **one
//! convex quad per segment**, each dropping from its segment's two points to the
//! plot's own bottom edge. Ear clipping and a stencil pass were both considered
//! and the operator declined both on 2026-10-02.
//!
//! **A translucent fill does *not* seam, and that was measured over the whole
//! region rather than at a few columns.** The first draft of this section claimed
//! that neighbouring quads sharing a vertical edge cover those pixels twice and
//! so put a seam down every sample. The quads are a **tiling, not an overlay**:
//! they abut, so a pixel on a shared edge is covered once and composites once.
//!
//! The measurement is a capture of a flat five-sample area chart with
//! [`Palette::fill`] at alpha 140, through the real renderer with 4× MSAA, **with
//! that chart alone in the frame and the grid switched off**
//! ([`Chart::set_grid_visible`]). The fill's interior — 310 by 90 pixels, which
//! contains all three shared edges at x = 1120, 1200 and 1280 and both of the
//! plot's own edges — reads **one single colour**: `(187,134,252)`, **1 distinct
//! colour in 27,900 pixels**.
//!
//! **The grid was off, and that is load-bearing rather than incidental.** It
//! defaults to on, [`Chart::paint`] draws it *under* the fill, and a fill at alpha
//! 140 is translucent, so grid lines underneath would show through as a second
//! colour. Four lines cross any interior taller than a division
//! (`GRID_DIVISIONS` is 5), and on a 300-pixel plot that is a 60-pixel gap, so a
//! 90-pixel interior **cannot** fit between two of them — one always crosses. A
//! one-colour reading over a 90-pixel interior is therefore itself the evidence
//! that the grid was off, and stating it is what makes the number comparable: with
//! the grid on, the correct claim is "the seams are invisible", not "one colour".
//!
//! **What made the first attempt at this measurement wrong, and why it is worth
//! writing down:** the harness drew a *second* chart — a bar chart — at **the same
//! x and the same y origin** as the translucent one, 400 tall where it was 250.
//! Over the two opaque bars the same fill read `(255,194,255)`, and that is **not**
//! a seam and not two fill layers: it is one fill layer over an opaque bar of the
//! same colour. The solid fragment shader writes `frag_color = v_color`, straight
//! alpha, while [`end_frame`](crate::render::Renderer::end_frame) blends with
//! `GL_ONE, GL_ONE_MINUS_SRC_ALPHA`, which expects a premultiplied source — so a
//! translucent solid primitive composites as `rgb + dst·(1 − a)` and is brighter
//! over a brighter destination. Predicted and measured, at three destinations:
//!
//! | destination | predicted | measured |
//! |---|---|---|
//! | black `(0,0,0)` | `(187,134,252)` | `(187,134,252)`, the whole interior |
//! | `TextMuted` axis `(158,158,158)` | `(255,205,255)` | `(255,205,255)`, 419 px |
//! | `Primary` bar `(187,134,252)` | `(255,194,255)` | `(255,194,255)`, 12,765 px |
//!
//! **The renderer row of that table is a finding about the renderer and not about
//! this module**, and it is the integrator's to act on: *any* translucent solid
//! primitive in this pipeline composites brighter over a lighter destination, and
//! [`Color::to_premultiplied`](crate::property::Color::to_premultiplied) exists
//! and is not what the solid quad carries. What it did here was make a harness
//! artefact look like a property of the widget.
//!
//! So the fill is **not** required to be opaque and [`Palette::fill`] says so
//! rather than claiming a reason it does not have. What remains true is the
//! ordinary one: a translucent fill over a varying background is a design
//! decision, and a caller who makes it should know it is theirs.
//!
//! # Antialiasing is the pipeline's, and the widget's job is to give it edges
//!
//! Requirement 6's "anti-aliased edges" is met by **drawing real geometry with
//! real edges**, and not by anything in this module: the default framebuffer is
//! multisampled at four samples, so every boundary recorded here — a segment's
//! outer edge, a bar's corner, the top of a fill — is a real edge for the hardware
//! to resolve, and this module adds no shader branch, no distance field, no
//! `fwidth` and no `smoothstep` to get there.
//!
//! **Measured, not asserted.** A capture of a 3-pixel line through the real
//! renderer reads **3.0113 px** of thickness perpendicular to every segment over
//! 156 samples — an error of **+0.0113 px, +0.38%** — with the coverage integral
//! resolving a 2-pixel axis as **exactly 2.0000 px**, so the estimator itself is
//! unbiased. The boundary pixels are visibly intermediate: the capture carries
//! `(93,67,126)`, `(140,100,189)` and `(47,33,63)` beside the theme's
//! `(187,134,252)`, which are blends of the primary with the background and are
//! what four samples per pixel look like.
//!
//! That is also why nothing here bakes softness into a colour or an alpha: **an
//! edge that is already geometry cannot be improved by a colour**, and a
//! half-transparent border drawn by the widget to fake an edge is a colour where a
//! multisample resolves one, and a worse one.
//!
//! What this module owes the pipeline is *edges to resolve*: the series is
//! mitred rather than built from overlapping capsules, so a turn is a corner
//! rather than a scalloped seam, and every recorded shape is a convex polygon or
//! an axis-aligned rectangle because those are the two the rasteriser fills
//! exactly.
//!
//! # There is no `on_event`, and nothing here can be touched
//!
//! Requirement 4 lists no gesture for a chart, and a chart is a display rather
//! than a control: there is no value a drag would set, no action a tap would
//! report, and a caller driving one writes [`data`](Chart::data). **A drawn
//! series, a grid and a cursor's worth of hairlines all look grabbable, and
//! nothing in this module reads a pointer over any of them.** A caller that wants
//! a chart to be pannable or selectable has to build that gesture itself, and
//! should not assume the existence of a series implies one — a drawn control
//! with nothing behind it is what happens when that assumption is made in the
//! other direction.
//!
//! # Colours, and what is not a theme token
//!
//! The colours follow the progress bar's, the slider's and the gauge's rule
//! rather than adding theme tokens: a [`Palette`] names the five a chart draws
//! with, the properties hold them so a theme switch can be animated into them,
//! and [`snap_to_state`](Chart::snap_to_state) puts a themed chart on its theme
//! at once. The *sizing* — the plot's gutters, the line's width, a bar's width,
//! the grid's spacing, the labels' font size — is named constants rather than
//! tokens, for the reason `progress.rs`'s and `gauge.rs`'s own module documents
//! at length: the theme has no token for a chart's parts, and adding one would
//! change [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables,
//! the token count and the transition every token takes part in during a switch,
//! for values a switch does not change. Each constant below says what would
//! reverse it.
//!
//! Requirement 4's "animation duration from theme tokens" is satisfied the way
//! every other widget satisfies it: the caller hands a [`Motion`], and
//! [`Motion::from_theme`] reads the theme's `DurationFast` and `EasingStandard`.
//! The widget picks no duration of its own.
//!
//! # Labels are placed, never measured
//!
//! [`DrawCommand::Text`] carries an `x`, a `y`, a string and a font size and **no
//! width**, so nothing outside the text pipeline knows how far a run reaches. A
//! chart therefore cannot centre a label, cannot right-align one, cannot tell
//! whether two labels collide, and cannot tell whether the last one overhangs the
//! plot. What it *can* do is reserve a gutter and place each label from what it
//! knows:
//!
//! - a **y** label is drawn left-aligned in a left gutter of `Y_LABEL_GUTTER`,
//!   its line box's top [`LABEL_FONT_SIZE / 2`] above the mark it belongs to,
//!   which centres it to within the difference between the line height and the
//!   font size;
//! - an **x** label is drawn left-aligned at its own sample's x, and **it
//!   overhangs the plot's right edge by its own width**, which the widget cannot
//!   know. The caller's answers are fewer labels and shorter ones.
//!
//! # Examples
//!
//! A themed line chart with a fixed range, painted off the origin:
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::Rect;
//! use ui_core::theme::Theme;
//! use ui_core::widgets::chart::{Chart, ChartType, Palette};
//!
//! let mut nodes = Arena::new();
//! let mut chart = Chart::new(&mut nodes, ChartType::Line);
//! chart.set_palette(Palette::from_theme(&Theme::dark()));
//! chart.set_fixed_range(0.0, 100.0);
//! chart.data.set(vec![10.0, 40.0, 70.0]);
//! chart.snap_to_state();
//!
//! // The rect is deliberately not at the origin: an x and a width are two
//! // different numbers, and a fixture at (0, 0) cannot see one read as the
//! // other.
//! let rect = Rect::new(300.0, 120.0, 480.0, 240.0);
//! assert_eq!(
//!     chart.paint(rect).len(),
//!     8,
//!     "four grid lines, two axes and two segments for three samples"
//! );
//! ```
//!
//! A new reading arrives and the line grows toward it:
//!
//! ```
//! use std::time::Duration;
//! use ui_core::animation::Easing;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::widgets::button::Motion;
//! use ui_core::widgets::chart::{Chart, ChartType};
//!
//! let mut nodes = Arena::new();
//! let chart = Chart::new(&mut nodes, ChartType::Line);
//! chart.data.set(vec![1.0, 2.0]);
//! chart.snap_to_state();
//! let before = chart.shown.get();
//! assert_eq!(before.x, vec![0.0, 1.0], "two samples at the two ends");
//!
//! chart.data.set(vec![1.0, 2.0, 3.0]);
//! chart.animate_to_state(Motion {
//!     duration: Duration::from_millis(100),
//!     easing: Easing::Linear,
//! });
//! // Aiming changes nothing on the frame it happens: the drawn series is
//! // still the one that was already on screen.
//! assert_eq!(chart.shown.get().x, before.x);
//!
//! chart.tick(Duration::from_millis(100));
//! let arrived = chart.shown.get();
//! assert_eq!(arrived.x, vec![0.0, 0.5, 1.0], "and the run is evenly spaced at last");
//! assert_eq!(arrived.values, vec![1.0, 2.0, 3.0]);
//! ```
//!
//! The same append, one frame in, is where the **line grows** rather than
//! re-spacing: the sample that was in the middle is still where it was, and the
//! new one has arrived at the far edge reading what the last one read, so the
//! first frame of the transition draws the old line plus a flat stub of it.
//!
//! ```
//! use std::time::Duration;
//! use ui_core::animation::Easing;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::widgets::button::Motion;
//! use ui_core::widgets::chart::Chart;
//!
//! let mut nodes = Arena::new();
//! let chart = Chart::new(&mut nodes, Default::default());
//! chart.data.set(vec![1.0, 2.0]);
//! chart.snap_to_state();
//! let before = chart.shown.get();
//!
//! chart.data.set(vec![1.0, 2.0, 3.0]);
//! chart.animate_to_state(Motion {
//!     duration: Duration::from_millis(100),
//!     easing: Easing::Linear,
//! });
//! chart.tick(Duration::from_millis(50));
//!
//! let started = chart.shown.get();
//! assert_eq!(started.x[0], before.x[0], "the oldest has not moved at all");
//! assert_eq!(
//!     started.x[2], 1.0,
//!     "and the new one is still at the far edge the old run ended on"
//! );
//! assert_eq!(
//!     started.x[1], 0.75,
//!     "while the middle one slides from the middle towards its even place"
//! );
//! assert_eq!(
//!     started.values[2], 2.5,
//!     "and the new reading is between the last one and its own"
//! );
//! ```

use std::cell::RefCell;
use std::time::Duration;

use crate::animation::{AnimationClock, Easing};
use crate::arena::{Arena, Handle};
use crate::layout::Size;
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::button::Motion;

/// Half: the middle of anything from `0.0` to `1.0`.
///
/// Named because two places divide by a count that is one short of itself for a
/// single sample — a series' even spacing and a set of labels' spread — and both
/// of them have the same answer for one sample: the middle, not an edge and not a
/// division by zero. It is a constant because it is arithmetic rather than a
/// decision; what would make it a decision is a caller that wants a chart of one
/// reading drawn against its axis rather than in the middle of its plot.
const HALF: f32 = 0.5;

/// How wide and how tall a chart asks to be when its caller gives it no size of
/// its own.
///
/// Four hundred and eighty by two hundred and forty, which is the slider's
/// [`DEFAULT_LENGTH`](crate::widgets::slider::DEFAULT_LENGTH) across and the
/// gauge's [`DEFAULT_SIZE`](crate::widgets::gauge) of 200 up, so a caller laying a
/// chart out among the other widgets gets one family. It is a constant rather
/// than a theme token for the reason the module documents. What would reverse it
/// is a theme token for a preferred plot size.
const DEFAULT_WIDTH: f32 = 480.0;

/// The height a chart asks for when its caller gives it no size of its own.
///
/// The other half of `DEFAULT_WIDTH`; see it for the reason.
const DEFAULT_HEIGHT: f32 = 240.0;

/// How thick a line or area chart's stroke is, in pixels.
///
/// Three, which is the weight of a plot line rather than of a border: a 1 px
/// series on a 480-px plot is a hairline that disappears against a grid line of
/// `GRID_WIDTH`, and a 6 px one covers three samples of a crowded chart. It is
/// the *full* width of the stroke — half on each side of the centre line — and
/// not the gauge's chord, because a chart's centre line is the data and a chord
/// has no radius to sag from. **What it costs** is read back at
/// `MITRE_LIMIT`, where the mitre length is a multiple of it, and a thinner
/// series mitres over a larger range of turns for exactly the same picture. It
/// is a constant for the reason `DEFAULT_WIDTH` is.
const LINE_WIDTH: f32 = 3.0;

/// How much of its slot a bar fills across, from `0.0` to `1.0`.
///
/// Three fifths, which leaves a fifth of a slot's pitch of gap on each side: at
/// half the pitch the bars touch and a chart of a dozen samples reads as one
/// block, and at four fifths a chart of fifty samples reads as a solid area with
/// no bars in it. **It is a fraction of the pitch rather than a pixel count**
/// because the pitch is the chart's own width over its own sample count, and a
/// fixed count would be a fixed count of pixels wide on one plot and invisible
/// bars on the next. What would reverse it is a caller that wants its bars
/// touching, which is [`Chart::set_bar_fraction`].
const BAR_WIDTH_FRACTION: f32 = 0.6;

/// The narrowest a bar may be, in pixels.
///
/// One, because a bar is a rectangle and a rectangle of no width draws nothing:
/// a chart of six hundred samples on a 600-pixel plot has a pitch of a pixel and
/// three fifths of that, and a bar thinner than half a pixel begins to drop out
/// of a rasteriser's coverage rather than to fade. It does **not** stop bars from
/// overlapping when the pitch is under a pixel: a bar is at least this wide
/// whatever the pitch, so a chart with more samples than the plot has pixels draws
/// bars that overlap, and the caller's answer is fewer samples or a wider chart.
/// A constant for the reason `DEFAULT_WIDTH` is.
const MIN_BAR_WIDTH: f32 = 1.0;

/// How many parts the y range is cut into for the grid.
///
/// Five, which is four interior lines: enough that a reader can see where a
/// reading sits between two values, few enough that the grid does not compete
/// with the series for the eye. It is a **count** rather than a pixel spacing
/// because a chart's height is its caller's, and a spacing in pixels would give a
/// short chart two lines and a tall one eleven for the same number. What would
/// reverse it is a caller that wants its own divisions drawn, which is what
/// `set_fixed_range` plus a caller-drawn grid is.
const GRID_DIVISIONS: usize = 5;

/// How thick the two axis lines are, in pixels.
///
/// Two, which is twice `GRID_WIDTH` and the same order as
/// [`gauge::TICK_THICKNESS`](crate::widgets::gauge)'s: an axis is a frame the
/// series is read against and has to survive being read against the series. A
/// constant for the reason `DEFAULT_WIDTH` is.
const AXIS_WIDTH: f32 = 2.0;

/// How thick a grid line is, in pixels.
///
/// One, the theme's own [`BorderWidth`] and a hairline by definition: a grid is
/// present but not emphasised, and at the same weight as the axes it stops being
/// a grid. A constant for the reason `DEFAULT_WIDTH` is.
const GRID_WIDTH: f32 = 1.0;

/// How far the left gutter is, in pixels, when the chart has y labels.
///
/// Thirty-six, which holds the widest reading a chart writes itself at
/// `LABEL_FONT_SIZE` — a three-character number and its sign — with a few
/// pixels of clearance before the axis. **The gutter is reserved only when there
/// are labels to put in it**, so an unlabelled chart gets the whole of its rect.
/// It is a constant rather than a token for the reason the module documents; and
/// it is a constant rather than a measurement of the widest string because this
/// widget cannot measure a string at all — [`DrawCommand::Text`] carries no
/// width — so a gutter computed from the labels would be a guess dressed as a
/// measurement. What would reverse it is a caller that lays the chart out with
/// its own gutter and wants to say so, which is what
/// [`Chart::set_fixed_range`] and a caller-drawn chart are.
const Y_LABEL_GUTTER: f32 = 36.0;

/// How far the bottom gutter is, in pixels, when the chart has x labels.
///
/// Eighteen, which is `X_LABEL_GAP` plus `LABEL_FONT_SIZE` plus two pixels of
/// slack: the label's line box is at least its font size tall, and the gutter has
/// to hold the gap above it as well as the box. **Reserved only when there are
/// labels**, for the reason `Y_LABEL_GUTTER` is. A constant for the reason
/// `DEFAULT_WIDTH` is.
const X_LABEL_GUTTER: f32 = 18.0;

/// How far above the x axis an x label's line box starts, in pixels.
///
/// Four: enough to see that the label belongs to the axis rather than to it. It
/// is a gap rather than a fraction of the font size so that a caller changing the
/// label font size changes the gutter's contents and not the space between the
/// axis and the text. A constant for the reason `DEFAULT_WIDTH` is.
const X_LABEL_GAP: f32 = 4.0;

/// The size the axis labels are drawn at, in pixels.
///
/// Twelve, which is what both themes hold for
/// [`FontSizeSm`](crate::theme::ThemeToken::FontSizeSm). It is written down here
/// rather than read from the token, and the reason is the gutter it sits in:
/// `Y_LABEL_GUTTER` and `X_LABEL_GUTTER` are constants, so a theme that
/// lengthened the label would give the widget a longer label in a gutter sized
/// for a shorter one — a theme *can* change a font size, so this is the one
/// sizing value a switch could plausibly move, and it is the one whose container
/// a switch cannot. What would reverse it is a caller that lays its own labels
/// out over the chart and sets the font size itself, which is a caller-drawn
/// chart. A constant for the reason `DEFAULT_WIDTH` is.
const LABEL_FONT_SIZE: f32 = 12.0;

/// The largest a mitre is allowed to be, as a multiple of the stroke's half
/// width.
///
/// Four, which is SVG's `stroke-miterlimit` and the turn it corresponds to is
/// `2·acos(1/4)` = **151.045°**. **What it costs, computed rather than assumed:**
/// the mitre length is `half/cos(φ/2)`, so at 151° it is exactly `4·half` — 6.0
/// pixels on a 3-pixel line — and past it the join falls back to each segment's
/// own offset plus a disc of radius `half` at the vertex. A chart reaches that
/// turn when each side of a peak rises `3.873` slots, which on a 600-pixel plot
/// with 20 samples (a 31.6-pixel pitch) is 122 pixels, and the plot is 300 tall,
/// so **an ordinary spiky chart does**. Lowering it to 2 would put the limit at
/// 120° and fill more corners with discs; raising it to 8 would allow a
/// 165.638° turn and a mitre of exactly `8·half` — 12.0 pixels on a 3-pixel line.
/// **All three of those figures are the same number computed twice**, which is why
/// they are written as one: the mitre at the limit's own turn is `half·MITRE_LIMIT`
/// by definition, so "8-half" and "12 px" cannot disagree and `165.96°` cannot be
/// either. A constant for the reason `DEFAULT_WIDTH` is.
const MITRE_LIMIT: f32 = 4.0;

/// The shape a chart draws its data in.
///
/// A mode and not an appearance, and behind a setter rather than a
/// `Property<ChartType>` for the reason the module documents: nothing animates
/// *which* shape a chart is in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChartType {
    /// Connected line segments, one mitred quad each. The default.
    #[default]
    Line,
    /// One rectangle per sample, rising from the chart's baseline.
    Bar,
    /// A line with the region under it filled to the bottom of the plot.
    Area,
}

/// A series as a chart draws it: where each sample sits, and what it reads.
///
/// **Two parallel arrays rather than one, and that is load-bearing.** An even
/// spacing is `i / (len − 1)`, so appending a sample *re-spaces every sample
/// already in the series*: three samples across a plot put the middle one at
/// `0.5` and four put it at `0.333`. A chart that animated only the values would
/// show a correct-looking series in the wrong places for the whole of the
/// transition, so the positions are part of what is drawn and are animated with
/// the readings. The module document has the arithmetic.
///
/// `x` is a **share of the plot's width**, `0.0..=1.0`, not a pixel: the plot's
/// width is the caller's, and a series that stored pixels would need rewriting
/// every time the window was resized.
///
/// The two arrays are the same length by construction, and the widget does not
/// police it: [`Chart::paint`] reads them as far as the **shorter** one goes, so a
/// caller that writes a [`shown`](Chart::shown) of two different lengths gets the
/// series as far as it is whole rather than an index out of bounds.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::chart::Series;
///
/// let empty = Series::default();
/// assert!(empty.is_empty(), "a series with no samples is one value pair short");
///
/// let run = Series { x: vec![0.0, 1.0], values: vec![3.0, 4.0] };
/// assert!(!run.is_empty());
/// assert_eq!(run.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Series {
    /// Where each sample sits across the plot's width, from `0.0` at its left
    /// edge to `1.0` at its right.
    ///
    /// A caller writing this by hand gets the order the samples arrived in and
    /// the spacing it wants; a widget reading it bounds every value into
    /// `0.0..=1.0`, so a sample outside the plot is drawn at the plot's edge
    /// rather than off it.
    pub x: Vec<f32>,
    /// What each sample reads, in the same units as [`Chart::data`].
    ///
    /// **A non-finite value is a gap, not a reading.** `NaN` breaks a line's run
    /// and draws no bar, which is what a missing sample means in a series; see
    /// [`Chart::paint`].
    pub values: Vec<f32>,
}

impl Series {
    /// Returns whether the series has no samples at all.
    ///
    /// It asks about **both** halves rather than about `x`, because the two can
    /// disagree only if a caller wrote [`shown`](Chart::shown) by hand with
    /// different lengths, and then the drawn series is the shorter one — so
    /// "empty" means "no sample is drawn", whichever array that turns out to be.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.x.is_empty() && self.values.is_empty()
    }

    /// Returns how many samples the series draws.
    ///
    /// The length of the **shorter** of the two halves, for the reason
    /// [`Series`] gives: that is as far as [`Chart::paint`] reads it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::widgets::chart::Series;
    ///
    /// let uneven = Series { x: vec![0.0, 0.5, 1.0], values: vec![1.0, 2.0] };
    /// assert_eq!(uneven.len(), 2, "as far as it is whole");
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.x.len().min(self.values.len())
    }
}

/// The colours a chart draws with.
///
/// Five: the axes, the grid, the series, an area chart's body and the labels.
/// They are not tokens of their own — the theme has none per part, and adding
/// one per part would put five more tokens in every theme table and in every
/// theme switch — so a chart is themed with the theme's own and
/// [`Palette::from_theme`] says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The two axis lines.
    pub axis: Color,
    /// The grid lines behind the series.
    pub grid: Color,
    /// The series: the stroke of a line or area chart, the bars of a bar chart.
    pub series: Color,
    /// The body an area chart fills under its line.
    ///
    /// **Any alpha works: the per-segment quads do not seam.** They are a tiling
    /// rather than an overlay — neighbouring quads abut, so a pixel on a shared
    /// edge is covered once — and a capture of a translucent fill at alpha 140
    /// through the real renderer, **with that chart alone in the frame**, reads
    /// **one single colour across the whole fill interior: 1 distinct colour in
    /// 27,900 pixels**, spanning all three of a flat chart's shared edges. The
    /// module document has the measurement, the fixture it was taken with, and the
    /// harness artefact that confounded the first attempt at it. What a
    /// translucent fill *is*, is a design decision over whatever is behind it.
    ///
    /// [`Palette::from_theme`] returns the theme's own
    /// [`Primary`](crate::theme::ThemeToken::Primary), which is opaque in both
    /// themes, so a caller has to lower it deliberately.
    ///
    /// **It is the same colour as [`series`](Palette::series) out of the theme**,
    /// which has one consequence worth writing down: an area chart's fill quads
    /// and its stroke quads cannot be told apart by colour, and a caller reading a
    /// recorded paint has to use [`Chart::paint`]'s documented order — the fill is
    /// recorded before the stroke. A caller that wants them apart sets this field,
    /// which is what it is for.
    pub fill: Color,
    /// The axis labels.
    pub label: Color,
}

impl Default for Palette {
    /// Returns a neutral grey chart: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    ///
    /// The greys are the progress bar's and the gauge's, so a chart and a bar
    /// look like one family before either is themed.
    fn default() -> Self {
        Palette {
            axis: Color::new(110, 110, 110, 255),
            grid: Color::new(64, 64, 64, 255),
            series: Color::new(160, 160, 160, 255),
            fill: Color::new(96, 96, 96, 255),
            label: Color::new(158, 158, 158, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The series is [`Primary`](crate::theme::ThemeToken::Primary), because a
    /// plotted reading *is* the data and the theme's primary is the colour this
    /// repository uses for the value being shown — the progress bar's fill, the
    /// gauge's arc, the slider's fill.
    ///
    /// The axes are [`TextMuted`](crate::theme::ThemeToken::TextMuted), which is
    /// what that token is for — present but not emphasised — and which is what
    /// keeps an axis legible *over* a series drawn in the primary colour. The
    /// grid is [`Border`](crate::theme::ThemeToken::Border), the theme's hairline
    /// and the only one of its nine that is muted by definition: a grid is the
    /// part of a chart that is not the data and is not the frame, and putting it
    /// two tokens below the axes is what makes the two read as different things.
    ///
    /// The labels are [`TextMuted`](crate::theme::ThemeToken::TextMuted) as
    /// well, for the gauge's tick marks' reason, and the body an area chart fills
    /// is [`Primary`](crate::theme::ThemeToken::Primary) — the same colour as the
    /// series, so an area chart reads as one solid region whose top edge is the
    /// line. A caller that wants the outline to read separately sets
    /// [`Palette::fill`] to something else, which is what the field is for.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::theme::{Theme, ThemeToken};
    /// use ui_core::widgets::chart::Palette;
    ///
    /// let theme = Theme::dark();
    /// let palette = Palette::from_theme(&theme);
    /// let color = |token| theme.get(token).as_color().unwrap();
    /// assert_eq!(palette.series, color(ThemeToken::Primary));
    /// assert_eq!(palette.axis, color(ThemeToken::TextMuted));
    /// assert_eq!(palette.grid, color(ThemeToken::Border));
    /// assert_eq!(palette.fill, color(ThemeToken::Primary));
    /// assert_eq!(palette.label, color(ThemeToken::TextMuted));
    ///
    /// // And the fill is opaque, which is what per-segment quads require.
    /// assert_eq!(palette.fill.a, 255);
    /// ```
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            axis: token_color(theme, crate::theme::ThemeToken::TextMuted),
            grid: token_color(theme, crate::theme::ThemeToken::Border),
            series: token_color(theme, crate::theme::ThemeToken::Primary),
            fill: token_color(theme, crate::theme::ThemeToken::Primary),
            label: token_color(theme, crate::theme::ThemeToken::TextMuted),
        }
    }
}

/// The appearance the chart's data and its palette imply.
///
/// Every field is a target, not a value in flight:
/// [`animate_to_state`](Chart::animate_to_state) animates the chart's properties
/// toward this and [`paint`](Chart::paint) draws whatever the properties have
/// reached, which is a [`Style`] part way through on a frame where something is
/// moving.
///
/// The grid's visibility, the line's width and a bar's width are **not** in here:
/// they are the mapping from a sample to a place, and no transition runs toward
/// any of them.
///
/// It is not `Copy`, unlike every other widget's, because [`Series`] holds two
/// vectors and is not `Copy` either.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    /// The series the chart is drawn at, arrived: [`data`](Chart::data) evenly
    /// spaced across the plot, with every sample where its value puts it.
    ///
    /// The **scale is not in here** — that is [`Chart::y_range`], and it depends
    /// on the rect the chart is painted into rather than on its data and its
    /// palette alone.
    pub shown: Series,
    /// The colour of the axes.
    pub axis: Color,
    /// The colour of the grid lines.
    pub grid: Color,
    /// The colour of the series.
    pub series: Color,
    /// The colour of an area chart's body.
    pub fill: Color,
    /// The colour of the labels.
    pub label: Color,
}

/// The series transition the chart is running, when it is running one.
///
/// Private, and a plain struct rather than an [`Animation`] because
/// [`Animation`] needs its value type to be
/// [`Interpolate`](crate::animation::Interpolate) and a `Series` — two vectors of
/// `f32` — is not a type anything in this repository can interpolate towards.
/// Implementing `Interpolate` for `Vec<f32>` would have put a rule about series in
/// a module about interpolable values, and it would have had to say something
/// about a `NaN` in the values, which is the one case where the answer is not
/// "the `t` of the way between".
///
/// So the chart integrates the series itself, which is why
/// [`tick`](Chart::tick) has a second job beyond driving the colours.
#[derive(Clone, Debug)]
struct Glide {
    /// Where the run was when the glide was aimed.
    from: Series,
    /// Where it is going: [`data`](Chart::data) as it was when it was aimed.
    to: Series,
    /// How far through the motion it has got.
    elapsed: Duration,
    /// How long the whole of it takes.
    duration: Duration,
    /// The curve it follows.
    easing: Easing,
    /// What the newest bar's height starts at, from `0.0` to `1.0`.
    ///
    /// Only [`ChartType::Bar`] draws it; a line and an area chart leave it alone,
    /// for the reason [`Gauge::animate_to_state`] gives — a transition on a
    /// property nothing draws is one that reports itself as running for nothing.
    from_reveal: f32,
}

/// A chart: a series of readings drawn over two axes.
///
/// The widget holds the properties the task gives it — [`data`], [`x_labels`],
/// [`y_labels`] — the two that are drawn rather than held ([`shown`],
/// [`reveal`]) and the five colour properties a theme switch moves ([`axis`],
/// [`grid`], [`series`], [`fill`], [`label`]) — and the plain fields that define
/// the shape: [`chart_type`] through [`set_chart_type`](Chart::set_chart_type),
/// the scale through [`set_fixed_range`](Chart::set_fixed_range) and
/// [`clear_fixed_range`](Chart::clear_fixed_range), the grid through
/// [`set_grid_visible`](Chart::set_grid_visible), and the sizing through
/// [`set_line_width`](Chart::set_line_width) and
/// [`set_bar_fraction`](Chart::set_bar_fraction).
///
/// The shape is plain fields rather than properties for the reason
/// [`ChartType`] documents: nothing animates a chart's mode, its scale or its
/// grid, and a property the caller could write directly would let the mode and
/// the drawn series disagree, which is what the setters exist to prevent.
///
/// The node is the caller's to keep clean, and its size is the caller's to give
/// through [`layout_mut`](crate::node::WidgetNode::layout_mut) —
/// [`Chart::size`] is a suggestion for a caller who has nothing else to go on. A
/// chart draws inside whatever rect it is given, which is what decides the size of
/// its plot once the label gutters are taken off it.
///
/// [`data`]: Chart::data
/// [`x_labels`]: Chart::x_labels
/// [`y_labels`]: Chart::y_labels
/// [`shown`]: Chart::shown
/// [`reveal`]: Chart::reveal
/// [`axis`]: Chart::axis
/// [`grid`]: Chart::grid
/// [`series`]: Chart::series
/// [`fill`]: Chart::fill
/// [`label`]: Chart::label
/// [`chart_type`]: Chart::chart_type
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::chart::{Chart, ChartType};
///
/// let mut nodes = Arena::new();
/// let chart = Chart::new(&mut nodes, ChartType::Bar);
/// chart.data.set(vec![1.0]);
/// chart.snap_to_state();
///
/// // One sample is one bar: the whole chart, on a rect off the origin.
/// let rect = Rect::new(700.0, 40.0, 320.0, 180.0);
/// let commands = chart.paint(rect);
/// assert!(
///     commands
///         .iter()
///         .any(|command| matches!(command, ui_core::paint::DrawCommand::Rect { .. })),
///     "a bar chart draws a rectangle per sample, and one sample is one bar"
/// );
/// ```
pub struct Chart {
    /// The truth: what the chart is plotting, oldest sample first.
    ///
    /// The caller may write it and the widget never does: nothing a chart
    /// receives is a *new series*, only a new mode, a new scale or a new colour.
    /// A value outside a fixed range is not an error and is not wrapped; see
    /// [`Chart::y_range`] and [`Chart::paint`]. A `NaN` is a **gap** rather than a
    /// reading, and the widget documents what it draws for one.
    pub data: Property<Vec<f32>>,
    /// The labels along the x axis, one per sample at the same index.
    ///
    /// Fewer labels than samples is ordinary — a chart of sixty readings a
    /// second does not label all sixty — and **a sample with no label at its own
    /// index is drawn without one**, rather than borrowing its neighbour's.
    /// Labels beyond the last sample are not drawn at all, for the same reason:
    /// there is nothing under them.
    pub x_labels: Property<Vec<String>>,
    /// The labels down the left of the y axis, spread evenly over the plot's
    /// height from its top to its bottom.
    ///
    /// The widget does not interpret these numbers and does not draw its own:
    /// with an **auto-scaled** range the scale moves as the data moves, and
    /// numbers a widget wrote for itself would change on every frame of a
    /// transition. A caller that wants numbers on the axis fixes the range with
    /// [`set_fixed_range`](Chart::set_fixed_range) and writes the labels to match,
    /// which is the door the task file's "or fixed range" is for.
    pub y_labels: Property<Vec<String>>,
    /// The series the chart is *drawn* at, animated toward [`data`](Chart::data).
    ///
    /// Requirement 5's "data updates are smooth" is this property, and it is what
    /// the geometry is built from rather than from the truth: a chart painted
    /// from the truth would show the new series on the first frame of a
    /// transition. Writing it directly overrides the animation until the next
    /// aim.
    pub shown: Property<Series>,
    /// How much of the newest bar has risen, from `0.0` to `1.0`.
    ///
    /// Requirement 5's "a bar rises" is this property, and **only
    /// [`ChartType::Bar`] draws it**: a line chart's new sample arrives as a
    /// position and a reading rather than as a height, and an area chart's fill
    /// arrives with its line. A bar's height is its value's share of the plot,
    /// times this; at `0.0` the bar is not recorded at all, so a chart's newest
    /// bar is genuinely not there rather than a rectangle of no height.
    pub reveal: Property<f32>,
    /// The colour of the two axis lines.
    pub axis: Property<Color>,
    /// The colour of the grid lines.
    pub grid: Property<Color>,
    /// The colour of the series: a line or area chart's stroke, a bar chart's
    /// bars.
    ///
    /// Named for the drawn series and for nothing else; the readings themselves
    /// are [`data`](Chart::data).
    pub series: Property<Color>,
    /// The colour of the body an area chart fills under its line.
    ///
    /// Read as an opaque colour; [`Palette::fill`] says why, and the module
    /// document has the arithmetic.
    pub fill: Property<Color>,
    /// The colour of the axis labels.
    pub label: Property<Color>,
    chart_type: ChartType,
    fixed_range: Option<(f32, f32)>,
    show_grid: bool,
    line_width: f32,
    bar_fraction: f32,
    palette: Palette,
    glide: RefCell<Option<Glide>>,
    clock: RefCell<AnimationClock>,
    node: Handle,
}

impl Chart {
    /// Creates a chart of `chart_type` in the arena, and returns it.
    ///
    /// It starts with no data, no labels, an **auto-scaled** y axis, a grid, a
    /// [`ChartType::Line`] series and the colours of [`Palette::default`], until a
    /// caller gives it a [`Palette`](Chart::set_palette) and calls
    /// [`snap_to_state`](Chart::snap_to_state) or
    /// [`animate_to_state`](Chart::animate_to_state). A chart starts
    /// *arrived* rather than empty-but-arriving: a caller that writes
    /// [`data`](Chart::data) and then reads [`shown`](Chart::shown) without
    /// aiming anything gets an empty series rather than a series of zeroes.
    ///
    /// The task file's `Chart::new(chart_type) -> Handle` is read as this: the
    /// handle is [`Chart::handle`]'s, and returning it alone would leave a caller
    /// with no property to write and no way to draw the chart. Task 20's
    /// [`Gauge::new`](crate::widgets::gauge::Gauge::new), task 18's
    /// [`Progress::new`](crate::widgets::progress::Progress::new) and task 16's
    /// [`Slider::new`](crate::widgets::slider::Slider::new) settled the same
    /// reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::{Chart, ChartType};
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, ChartType::Area);
    /// assert!(nodes.get(chart.handle()).is_some(), "its node is in the arena");
    /// assert_eq!(chart.chart_type(), ChartType::Area);
    /// assert!(chart.data.get().is_empty(), "and it starts with nothing to plot");
    /// ```
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, chart_type: ChartType) -> Self {
        let palette = Palette::default();
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Chart {
            data: Property::new(Vec::new()),
            x_labels: Property::new(Vec::new()),
            y_labels: Property::new(Vec::new()),
            shown: Property::new(Series::default()),
            reveal: Property::new(1.0),
            axis: Property::new(palette.axis),
            grid: Property::new(palette.grid),
            series: Property::new(palette.series),
            fill: Property::new(palette.fill),
            label: Property::new(palette.label),
            chart_type,
            fixed_range: None,
            show_grid: true,
            line_width: LINE_WIDTH,
            bar_fraction: BAR_WIDTH_FRACTION,
            palette,
            glide: RefCell::new(None),
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the chart's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the chart draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the chart draws with, and leaves the current ones where
    /// they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](Chart::animate_to_state) or
    /// [`snap_to_state`](Chart::snap_to_state): a theme switch is animated, and a
    /// theme switch is the caller announcing a new palette and then moving the
    /// chart toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the chart chasing a palette that is still
    /// moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the shape the chart draws its data in.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::{Chart, ChartType};
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, ChartType::Line);
    /// assert_eq!(chart.chart_type(), ChartType::Line);
    /// chart.set_chart_type(ChartType::Bar);
    /// assert_eq!(chart.chart_type(), ChartType::Bar);
    /// ```
    #[must_use]
    pub fn chart_type(&self) -> ChartType {
        self.chart_type
    }

    /// Sets the shape the chart draws its data in, and puts the revealed part of
    /// a bar chart's newest bar where a bar about to rise expects to find it.
    ///
    /// Switching **to** [`ChartType::Bar`] writes [`reveal`](Chart::reveal) at
    /// once to `1.0`, so the first frame with bars on it is those bars at their
    /// full heights rather than at whatever a line chart's last transition left
    /// there. Switching **away** writes nothing, because nothing outside
    /// [`ChartType::Bar`] draws it.
    ///
    /// The mode is a plain field and this setter is the only way to write it,
    /// which is what keeps the mode and the drawn series from disagreeing.
    /// [`ChartType`] documents why it is not a `Property<ChartType>`.
    pub fn set_chart_type(&mut self, chart_type: ChartType) {
        self.chart_type = chart_type;
        if chart_type == ChartType::Bar {
            self.reveal.set(1.0);
        }
    }

    /// Returns the range the chart draws its y axis over, or `None` when there is
    /// nothing to draw a scale for.
    ///
    /// It is the fixed range when one is set — ordered, so
    /// [`set_fixed_range`](Chart::set_fixed_range) with the two the wrong way
    /// round is a range — and otherwise the extent of the **drawn** series
    /// ignoring any non-finite value. `None` means every sample is non-finite.
    ///
    /// **It is the drawn series rather than the truth**, and that is what makes a
    /// scale change smooth: a scale read from [`data`](Chart::data) moves on the
    /// frame the caller wrote it, so every reading on the chart shifts at once.
    /// Read from [`shown`](Chart::shown) it moves continuously with the
    /// transition, and the axis never outruns the series it is a scale for.
    ///
    /// **The range collapses** when every finite sample is the same number, and
    /// `Chart::paint` draws the series through the middle of the plot for that
    /// case rather than dividing by zero. A range whose two ends are the same
    /// number is a caller's range rather than the widget's, and gets the same
    /// answer.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, Default::default());
    /// assert_eq!(chart.y_range(), None, "nothing to scale");
    ///
    /// chart.data.set(vec![-4.0, 0.0, 8.0]);
    /// chart.snap_to_state();
    /// assert_eq!(chart.y_range(), Some((-4.0, 8.0)));
    ///
    /// chart.set_fixed_range(0.0, 240.0);
    /// assert_eq!(chart.y_range(), Some((0.0, 240.0)), "a fixed range wins");
    /// ```
    #[must_use]
    pub fn y_range(&self) -> Option<(f32, f32)> {
        if let Some((low, high)) = self.fixed_range {
            return Some(ordered(low, high));
        }
        let series = self.shown.get();
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for value in series.values.iter().take(series.len()) {
            if value.is_finite() {
                low = low.min(*value);
                high = high.max(*value);
            }
        }
        (low.is_finite() && high.is_finite()).then_some((low, high))
    }

    /// Returns the range the chart draws its y axis over regardless of its data,
    /// or `None` when it is auto-scaling.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, Default::default());
    /// assert_eq!(chart.fixed_range(), None, "it auto-scales by default");
    ///
    /// chart.set_fixed_range(240.0, 0.0);
    /// assert_eq!(chart.fixed_range(), Some((0.0, 240.0)), "and orders the pair");
    /// chart.clear_fixed_range();
    /// assert_eq!(chart.fixed_range(), None);
    /// ```
    #[must_use]
    pub fn fixed_range(&self) -> Option<(f32, f32)> {
        self.fixed_range.map(|(low, high)| ordered(low, high))
    }

    /// Fixes the range the chart draws its y axis over, whatever its data says.
    ///
    /// This is the door a caller uses to say what a reading *means*: with a fixed
    /// range a chart of temperatures from 18 to 22 fills its plot, with an
    /// auto-scaled one it is a flat line at the bottom with the axis stretched to
    /// fit. It is also what makes [`y_labels`](Chart::y_labels) mean anything,
    /// which [`Chart::y_labels`] says.
    ///
    /// The two are ordered, so `set_fixed_range(240.0, 0.0)` is a range from 0 to
    /// 240 rather than one with a negative span. **Nothing is drawn at once**: a
    /// range that has just changed is a new mapping from a reading to a place,
    /// and the series gliding from where it was to where the new mapping puts it
    /// is a transition the caller asks for with
    /// [`animate_to_state`](Chart::animate_to_state), exactly as for a new
    /// reading. Nothing else moves — no sample is dropped, none is clamped into
    /// the new range, and the scale is applied at paint time.
    ///
    /// A sample **outside** the range is pinned to the plot's edge rather than
    /// dropped, which is how a caller sees that the data does not fit the range
    /// they chose: a reading above the top draws along the top, and one below the
    /// bottom along the bottom.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, Default::default());
    /// chart.set_fixed_range(-40.0, 60.0);
    ///
    /// chart.data.set(vec![-40.0, 0.0, 60.0]);
    /// chart.snap_to_state();
    /// assert_eq!(chart.y_range(), Some((-40.0, 60.0)));
    /// ```
    pub fn set_fixed_range(&mut self, min: f32, max: f32) {
        self.fixed_range = Some(ordered(min, max));
    }

    /// Returns the chart to auto-scaling, and drops the fixed range.
    ///
    /// Nothing is drawn at once, for the reason
    /// [`set_fixed_range`](Chart::set_fixed_range) gives: the scale is a mapping
    /// applied at paint time, so the next frame is already on the new one.
    pub fn clear_fixed_range(&mut self) {
        self.fixed_range = None;
    }

    /// Returns whether the chart draws its grid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, Default::default());
    /// assert!(chart.grid_visible(), "a chart with no grid cannot be read");
    /// chart.set_grid_visible(false);
    /// assert!(!chart.grid_visible());
    /// ```
    #[must_use]
    pub fn grid_visible(&self) -> bool {
        self.show_grid
    }

    /// Sets whether the chart draws its grid, and puts the grid colour where
    /// lines about to appear expect to find it.
    ///
    /// **On** is the default, because a chart with a y axis and no grid cannot be
    /// read between two readings, and off is what a caller reaches for on a
    /// small plot where four lines are four lines too many. Turning it **on**
    /// writes [`grid`](Chart::grid) at once, for the reason
    /// [`set_chart_type`](Chart::set_chart_type) writes the revealed bar: the
    /// first frame with a grid on it is that grid in the current palette rather
    /// than in whatever the colour was before the caller had one. Turning it
    /// **off** leaves the colour alone, because nothing draws it.
    ///
    /// The grid is **horizontal lines only**: `GRID_DIVISIONS` parts of the y
    /// range with the interior boundaries drawn. A vertical line per sample would
    /// be a picket fence as soon as a chart had more than a dozen samples, and
    /// the x positions are already named by [`x_labels`](Chart::x_labels) and by
    /// the series itself.
    pub fn set_grid_visible(&mut self, visible: bool) {
        self.show_grid = visible;
        if visible {
            self.grid.set(self.palette.grid);
        }
    }

    /// Returns how thick the line or area chart's stroke is, in pixels.
    #[must_use]
    pub fn line_width(&self) -> f32 {
        self.line_width
    }

    /// Returns how far the chart's **stroke** reaches from the sample it belongs
    /// to, in pixels: `MITRE_LIMIT · line_width / 2`.
    ///
    /// **This is the number a caller needs and must not re-derive.** A segment's
    /// geometry is offset from its samples, so a mitred corner sits *beside* the
    /// vertex rather than on it, and the offset is longer the harder the turn: a
    /// flat cap at the end of a run is `line_width / 2` away, and a mitre at a
    /// turn of `φ` is `line_width / 2 / cos(φ/2)` away — up to
    /// `MITRE_LIMIT` times that, and no further, because that is where the join
    /// gives up its corner for a disc. **6 px at the defaults**, which is the
    /// number `ui_demo` was carrying a private copy of.
    ///
    /// **It moves when either of its two inputs moves**, which is why it is a
    /// method and not a constant: [`set_line_width`](Chart::set_line_width) moves
    /// it, and `MITRE_LIMIT` is a private constant a future change to the
    /// geometry would move. A caller that copied the number would go stale on the
    /// first of the two.
    ///
    /// **What it does not bound**, and each of these is a separate number:
    ///
    /// - the **first y label's line box**, whose top is
    ///   `LABEL_FONT_SIZE / 2` above the plot's top edge — **also 6 px at the
    ///   defaults, and larger than this one for any font size above 12 px**;
    /// - the **axes**, which are `AXIS_WIDTH` / 2 about their own positions —
    ///   1 px, on the y axis's left and the x axis's bottom;
    /// - an **x label's right-hand end**, which overhangs by its own width and
    ///   which this widget cannot know, because
    ///   [`DrawCommand::Text`] carries no width ([`Chart::x_labels`]).
    ///
    /// [`Chart::paint`] names each of those with its own bound. **What would
    /// reverse this method** is a caller that wants the reach of the *widget*
    /// rather than of the stroke — which is the maximum of this and the label and
    /// axis bounds above, and is a question this method does not answer because
    /// the answer is unbounded the moment a caller supplies an x label.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, Default::default());
    ///
    /// // MITRE_LIMIT (4) times half a 3-pixel line.
    /// assert_eq!(chart.stroke_reach(), 6.0);
    ///
    /// // And it follows the line width, which is why it is not a constant.
    /// chart.set_line_width(10.0);
    /// assert_eq!(chart.stroke_reach(), 20.0);
    /// ```
    #[must_use]
    pub fn stroke_reach(&self) -> f32 {
        MITRE_LIMIT * self.line_width / 2.0
    }

    /// Sets how thick the line or area chart's stroke is, in pixels.
    ///
    /// A negative width is no width: a negative one puts a segment's two inner
    /// corners outside its two outer ones, which is a quad of nothing drawn
    /// inside out. Zero is also drawn as no series at all, so a chart in
    /// [`ChartType::Bar`] keeps its bars.
    ///
    /// The width is the stroke's only extent. It does **not** decide how the
    /// series is cut up — it is one quad per sample pair whatever it is — and it
    /// does not decide where the mitre gives up, which is
    /// `MITRE_LIMIT` *this* number: a thinner stroke mitres over a larger
    /// range of turns and looks the same where it does not. What would bring the
    /// two together again is a caller that wants a constant *ratio* of corner
    /// error rather than a constant corner size.
    pub fn set_line_width(&mut self, line_width: f32) {
        self.line_width = line_width.max(0.0);
    }

    /// Returns how much of its slot a bar fills across, from `0.0` to `1.0`.
    #[must_use]
    pub fn bar_fraction(&self) -> f32 {
        self.bar_fraction
    }

    /// Sets how much of its slot a bar fills across, from `0.0` to `1.0`.
    ///
    /// A fraction outside that range is clamped rather than rejected, for the
    /// reason [`Progress::set_track_thickness`](crate::widgets::progress::Progress::set_track_thickness)
    /// gives for a negative thickness: a caller error must not put a bar outside
    /// its own slot. A fraction of zero is not a bar of no width —
    /// `Chart::bar_width` holds it to `MIN_BAR_WIDTH` — and one of `1.0`
    /// makes neighbouring bars touch, which is a caller's decision and is drawn.
    pub fn set_bar_fraction(&mut self, fraction: f32) {
        self.bar_fraction = bounded(fraction, 0.0, 1.0);
    }

    /// Returns the size a chart asks for: `DEFAULT_WIDTH` by
    /// `DEFAULT_HEIGHT`.
    ///
    /// A chart has no content to measure — its labels are the caller's strings
    /// and this widget cannot measure a string at all — so this is only for a
    /// caller that has nothing else to go on; a caller that lays the chart out
    /// itself gives the node whatever rect it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](Chart::paint) draws inside whatever it is given. Note that the
    /// plot is *smaller* than this whenever the chart has labels, because
    /// `Y_LABEL_GUTTER` and `X_LABEL_GUTTER` come off it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, Default::default());
    /// let size = chart.size();
    /// assert_eq!((size.width, size.height), (480.0, 240.0));
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }

    /// Returns the appearance the chart's data and its palette imply.
    ///
    /// The series is the truth evenly spaced across the plot, arrived: the same
    /// thing [`snap_to_state`](Chart::snap_to_state) writes to
    /// [`shown`](Chart::shown), which is why a caller reading the target and a
    /// caller reading the property after a snap read the same numbers. The
    /// **scale is not in here** — that is [`Chart::y_range`], and it depends on
    /// the rect as well as on the data.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::{Chart, Palette};
    /// use ui_core::theme::Theme;
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, Default::default());
    /// chart.data.set(vec![2.0, 4.0, 6.0]);
    /// chart.set_palette(Palette::from_theme(&Theme::dark()));
    ///
    /// let style = chart.style();
    /// assert_eq!(style.shown.values, vec![2.0, 4.0, 6.0]);
    /// assert_eq!(style.shown.x, vec![0.0, 0.5, 1.0], "evenly spaced across the plot");
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            shown: self.target_series(),
            axis: self.palette.axis,
            grid: self.palette.grid,
            series: self.palette.series,
            fill: self.palette.fill,
            label: self.palette.label,
        }
    }

    /// Applies the appearance the data and the palette imply at once, with no
    /// transition, and ends every transition already running.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a chart that has just been given a
    /// [`Palette`](Chart::set_palette) and has never animated — whose colour
    /// properties still hold the neutral defaults [`Chart::new`] wrote, so without
    /// this a themed chart starts out grey — and a caller that has written
    /// [`data`](Chart::data) itself and wants the chart to be that state now.
    ///
    /// It writes [`reveal`](Chart::reveal) at `1.0` only in
    /// [`ChartType::Bar`], for the reason
    /// [`Gauge::snap_to_state`](crate::widgets::gauge::Gauge::snap_to_state)
    /// leaves a property nothing draws alone.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::chart::{Chart, Palette};
    ///
    /// let mut nodes = Arena::new();
    /// let mut chart = Chart::new(&mut nodes, Default::default());
    /// let themed = Palette::from_theme(&Theme::dark());
    /// chart.set_palette(themed);
    /// chart.data.set(vec![1.0, 2.0]);
    /// chart.snap_to_state();
    /// assert_eq!(chart.series.get(), themed.series);
    /// assert_eq!(chart.shown.get().values, vec![1.0, 2.0]);
    /// ```
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        *self.glide.borrow_mut() = None;
        self.shown.set(style.shown);
        if self.chart_type == ChartType::Bar {
            self.reveal.set(1.0);
        }
        self.axis.set(style.axis);
        self.grid.set(style.grid);
        self.series.set(style.series);
        self.fill.set(style.fill);
        self.label.set(style.label);
    }

    /// Starts the transitions that carry the chart from wherever it is toward the
    /// appearance [`style`](Chart::style) implies, on `motion`.
    ///
    /// This is the call a caller makes after writing [`data`](Chart::data), and it
    /// is what requirement 5's "data updates are smooth (no jarring jumps)" is
    /// made of. **The widget picks no curve of its own.** A caller who wants the
    /// series to settle passes an
    /// [`Easing::Spring`]; a caller who wants a
    /// plain glide passes `Easing::Linear`; and the duration comes from the theme
    /// through [`Motion::from_theme`], which is how requirement 4's "animation
    /// duration from theme tokens" is satisfied.
    ///
    /// **The series glides rather than jumping, and the way it glides is what
    /// the requirement is about.** Every sample moves from where it was to where
    /// the new even spacing puts it, a sample that is new arrives **at the far
    /// edge of the plot** — where the old run's last sample was — and a sample
    /// that has gone is not animated out at all: the run ends where the new data
    /// ends, from the next frame. A shift — `[a,b,c,d]` becoming `[b,c,d,e]` — is
    /// four samples at four fixed places whose values glide from each one's own to
    /// its neighbour's, which is a series sliding one slot to the left.
    ///
    /// The chart's own transitions are cleared first, so the ones this replaces
    /// stop where they are rather than writing over the new ones when they
    /// arrive.
    ///
    /// **A glide that would change nothing is not started**, so a caller calling
    /// this on a frame where the data and the drawn series already agree is not
    /// left with a transition that reports itself as running for nothing.
    ///
    /// [`reveal`](Chart::reveal) is aimed only in [`ChartType::Bar`], and it is
    /// aimed from `0.0` **only when the series grew**: a bar chart's newest bar
    /// rises from its baseline when a sample arrives, and does not re-rise when
    /// the readings that are already there move.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::chart::{Chart, ChartType};
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, ChartType::Bar);
    /// chart.data.set(vec![1.0]);
    /// chart.snap_to_state();
    ///
    /// chart.data.set(vec![1.0, 2.0]);
    /// chart.animate_to_state(Motion {
    ///     duration: Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    /// // Aiming writes nothing: the first frame is still the chart that was
    /// // already on screen, with its one bar at its full height.
    /// assert_eq!(chart.reveal.get(), 1.0);
    ///
    /// chart.tick(Duration::from_millis(50));
    /// assert_eq!(chart.reveal.get(), 0.5, "and the new bar is half way up");
    /// chart.tick(Duration::from_millis(50));
    /// assert_eq!(chart.reveal.get(), 1.0);
    /// ```
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        {
            let mut clock = self.clock.borrow_mut();
            clock.clear();
            clock.add(
                self.axis
                    .animate_to(style.axis, motion.duration, motion.easing),
            );
            clock.add(
                self.grid
                    .animate_to(style.grid, motion.duration, motion.easing),
            );
            clock.add(
                self.series
                    .animate_to(style.series, motion.duration, motion.easing),
            );
            if self.chart_type == ChartType::Area {
                clock.add(
                    self.fill
                        .animate_to(style.fill, motion.duration, motion.easing),
                );
            }
            // `label` is animated for **every** type, and it used to be skipped
            // for bars. Nothing else here treats a bar specially: `fill` is
            // genuinely `Area`-only because no other type has a fill, but
            // `Chart::snap_to_state` sets `label` for every type and
            // `Chart::draw_labels` draws with it for every type, so a bar chart
            // skipped by this arm was left on the *old* theme's muted text while
            // its axis, grid and series all moved — grey labels on a light window,
            // with nothing recording that they had not arrived.
            // `every_colour_a_chart_draws_reaches_every_chart_type` covers it.
            clock.add(
                self.label
                    .animate_to(style.label, motion.duration, motion.easing),
            );
        }
        *self.glide.borrow_mut() = self.aim_glide(motion);
    }

    /// Appends `value` to the truth, and starts nothing.
    ///
    /// It is the plain write: a caller that drives the chart itself follows it
    /// with [`animate_to_state`](Chart::animate_to_state), and a caller that wants
    /// the two in one call uses [`animate_push`](Chart::animate_push).
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, Default::default());
    /// chart.push(1.0);
    /// chart.push(2.0);
    /// assert_eq!(chart.data.get(), vec![1.0, 2.0]);
    /// ```
    pub fn push(&self, value: f32) {
        let mut series = self.data.get();
        series.push(value);
        self.data.set(series);
    }

    /// Appends `value` to the truth and glides the drawn series to it on
    /// `motion`.
    ///
    /// The two calls [`push`](Chart::push) and
    /// [`animate_to_state`](Chart::animate_to_state) make, in the order they have
    /// to be made in, which is the pair a scrolling chart calls once per sample.
    pub fn animate_push(&self, value: f32, motion: Motion) {
        self.push(value);
        self.animate_to_state(motion);
    }

    /// Drops the oldest sample, appends `value`, and returns what was dropped.
    ///
    /// **`None` when the series was empty**, in which case `value` is appended and
    /// nothing was dropped: there was no oldest sample to shift out. It is not an
    /// error and not a panic, because a chart whose first sample arrives before
    /// its second is ordinary.
    ///
    /// The series length does not change, so nothing re-spaces: this is the shift
    /// half of requirement 4, and it is what makes a scrolling chart's reading
    /// travel left rather than jump. Nothing is drawn at once; see
    /// [`animate_shift`](Chart::animate_shift).
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, Default::default());
    /// assert_eq!(chart.shift(9.0), None, "nothing to shift out of an empty series");
    /// assert_eq!(chart.data.get(), vec![9.0]);
    ///
    /// chart.push(8.0);
    /// assert_eq!(chart.shift(7.0), Some(9.0));
    /// assert_eq!(chart.data.get(), vec![8.0, 7.0], "the oldest went, the newest arrived");
    /// ```
    #[must_use]
    pub fn shift(&self, value: f32) -> Option<f32> {
        let mut series = self.data.get();
        let dropped = if series.is_empty() {
            None
        } else {
            Some(series.remove(0))
        };
        series.push(value);
        self.data.set(series);
        dropped
    }

    /// Drops the oldest sample, appends `value`, glides the drawn series to it on
    /// `motion`, and returns what was dropped.
    ///
    /// The three calls [`shift`](Chart::shift) and
    /// [`animate_to_state`](Chart::animate_to_state) make, in the order they have
    /// to be made in. **`reveal` is not aimed**: a shift does not make the series
    /// longer, so there is no new bar to rise and the bars already on the plot
    /// keep their heights while their readings slide.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, Default::default());
    /// chart.data.set(vec![1.0, 2.0]);
    /// chart.snap_to_state();
    ///
    /// let motion = Motion { duration: Duration::from_millis(100), easing: Easing::Linear };
    /// assert_eq!(chart.animate_shift(3.0, motion), Some(1.0));
    /// assert_eq!(chart.shown.get().values, vec![1.0, 2.0], "the drawn series has not moved");
    /// chart.tick(Duration::from_millis(50));
    /// assert_eq!(
    ///     chart.shown.get().values,
    ///     vec![1.5, 2.5],
    ///     "half way, each reading sliding toward its neighbour's"
    /// );
    /// ```
    #[must_use]
    pub fn animate_shift(&self, value: f32, motion: Motion) -> Option<f32> {
        let dropped = self.shift(value);
        self.animate_to_state(motion);
        dropped
    }

    /// Advances the chart's transitions by `delta`, and returns whether any of
    /// them wrote.
    ///
    /// It is the chart's frame integration: call it once a frame, before the paint
    /// pass, with the time that frame took. The write is what reaches the node — a
    /// property callback registered by the caller marks the node dirty — so a
    /// caller that repaints only when this is true repaints exactly while
    /// something moves.
    ///
    /// **It advances the series glide as well as the colour transitions**, which
    /// is why the series is not a plain [`Property`] animation: a `Vec<f32>`
    /// cannot be interpolated by this repository's
    /// [`Interpolate`](crate::animation::Interpolate), and the chart integrates
    /// the run itself rather than putting a rule about series in a module about
    /// interpolable values. `Glide` says so.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::chart::Chart;
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, Default::default());
    /// chart.data.set(vec![0.0]);
    /// chart.snap_to_state();
    /// chart.data.set(vec![0.0, 100.0]);
    /// chart.animate_to_state(Motion {
    ///     duration: Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    ///
    /// assert!(chart.tick(Duration::from_millis(50)), "the series is moving");
    /// assert!(chart.is_animating());
    /// assert_eq!(chart.shown.get().values[1], 50.0, "half way");
    /// assert!(chart.tick(Duration::from_millis(50)));
    /// assert_eq!(chart.shown.get().values[1], 100.0, "arrived on the truth exactly");
    /// assert!(!chart.is_animating(), "so nothing is left running");
    /// ```
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        let mut wrote = self.clock.borrow_mut().tick(delta);
        wrote |= self.advance_glide(delta);
        wrote
    }

    /// Returns whether any of the chart's transitions is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating() || self.glide.borrow().is_some()
    }

    /// Returns the draw commands that paint the chart within `rect`.
    ///
    /// The commands are, in order: the **grid**, four horizontal lines across the
    /// plot, when [`grid_visible`](Chart::grid_visible); then the **data** — the
    /// fill quads of an [`ChartType::Area`], then one rectangle per sample of a
    /// [`ChartType::Bar`], then the disc of every over-limit join and one mitred
    /// quad per segment of a [`ChartType::Line`] or
    /// [`ChartType::Area`]; then the two **axes**; then the **labels**, the y
    /// labels in the left gutter and the x labels in the bottom one.
    ///
    /// # What reaches outside `rect`, and how far
    ///
    /// **Some of what this method records does reach outside `rect`, and the
    /// furthest it reaches is the larger of two numbers, not one of them:
    /// [`stroke_reach`](Chart::stroke_reach) — `MITRE_LIMIT` ·
    /// `line_width / 2`, **6 px at the defaults** — and the first y label's line
    /// box at `LABEL_FONT_SIZE / 2`, **also 6 px at the defaults, on a different
    /// lever**.** Both are documented properties rather than defects, and a caller
    /// who sizes a node by this has to leave room for the larger of the two.
    ///
    /// **They move on different setters, which is why the headline cannot name
    /// one.** The reach follows [`set_line_width`](Chart::set_line_width), which is
    /// public, so a caller who thins the line to 1 px gets a reach of **2.0 px** —
    /// while the label's 6 px does not move at all, because the label's font size is
    /// a private constant rather than a setter. **Measured**: one y label and
    /// `set_line_width(1.0)` puts its line box's top at `y = 234` against a node top
    /// of 240, **6.0 px above the node**, with `stroke_reach()` at **2.0**. A
    /// headline naming only the reach is reachable-false: the search for it has to
    /// write a label, and every assertion below it is about a chart with none.
    /// `nothing_but_the_stroke_reach_or_a_label_line_box_leaves_the_node_rect`
    /// measures the larger of the two over a battery that now includes labelled
    /// charts on a thin line. Every overhang, with its own bound and where
    /// the bound comes from:
    ///
    /// | what | how far outside | the bound |
    /// |---|---|---|
    /// | **a mitred corner at a turn** | up to 6.0 px, and **6.0 px measured** with a reading on the plot's top edge and a turn at the limit | `MITRE_LIMIT` · `line_width / 2`, because the corner is `line_width / 2 / cos(φ/2)` from its vertex and `MITRE_LIMIT` is where that stops being a corner |
    /// | **the first y label's line box** | exactly 6.0 px, measured | `LABEL_FONT_SIZE / 2`, because the box's top is placed that far above the mark and the first mark is on the plot's top edge |
    /// | **a cap at either end of a run** | up to `line_width / 2`; measured 0.67 px and 1.34 px | `line_width / 2` in the direction perpendicular to the run's first or last segment, so it is less than that whenever the run is not along the axis |
    /// | **the disc at an over-limit vertex** | up to `line_width / 2`; measured 1.5 px | `line_width / 2`, the disc's own radius |
    /// | **the axes** | `AXIS_WIDTH / 2`; 1 px | `AXIS_WIDTH` / 2, the axis being that wide *about* its own position: the y axis's left and the x axis's bottom |
    /// | **the grid lines** | `GRID_WIDTH / 2`; 0.5 px, and inside the plot anyway | `GRID_WIDTH` / 2 |
    /// | **an x label's right-hand end** | **unbounded, and unknowable** | none: [`DrawCommand::Text`] carries no width, so this widget cannot know it ([`Chart::x_labels`]) |
    ///
    /// **The first two rows are the ones that matter, and the earlier version
    /// of this claim got both wrong.** It said the exception was "the stroke's own
    /// half width" and named the two end caps, which is the *third* row: a cap is
    /// at most `line_width / 2`, and a mitred corner is up to `MITRE_LIMIT` times
    /// that — four times further at the defaults. A reading at the top of the
    /// range lands on the plot's top edge, and `plot.y` is `rect.y` because the
    /// gutters come off the left and the bottom only, so the corner is drawn
    /// outside `rect`. The doc also said "every one of these overhangs is inside
    /// it", which is the opposite of what happens. **And it then named
    /// [`stroke_reach`](Chart::stroke_reach) alone as the bound**, which is the row
    /// above's error in the other direction: the label's own 6 px is not smaller
    /// than the reach's, and the two are only ever equal at the defaults.
    ///
    /// **What is genuinely inside `rect`**, at any value, any scale and any point
    /// of any transition: the plot is `rect` less the gutters the labels need;
    /// every sample's `x` is bounded into `0.0..=1.0` before it is scaled by the
    /// plot's width; every reading is bounded into the range before it is scaled by
    /// the plot's height; the **bars** are pulled in until their outer edges land
    /// on the plot's edges; the **area fill**'s quads reach the plot's own bottom
    /// edge and no further; and the **fill and the samples** are inside the plot
    /// because every coordinate is bounded first.
    /// `nothing_but_the_stroke_reach_or_a_label_line_box_leaves_the_node_rect`
    /// asserts all of it over a battery of shapes, modes, rects and label sets, and
    /// `the_reach_of_a_peak_at_the_high_is_the_mitre_limit_times_the_half_width`
    /// pins the number.
    ///
    /// **Nothing is overdrawn by any of it** on a chart laid out with any
    /// clearance, which is the caller's business and the reason this is documented
    /// rather than clipped: bounding the corner to the rect would remove exactly
    /// the corner the mitre exists to draw.
    ///
    /// **Every size here comes from the *drawn* series**, so a chart mid-transition
    /// paints its geometry where the transition has got and not where the data is
    /// travelling to.
    ///
    /// What each degenerate case draws, which is what a caller has to be able to
    /// rely on rather than to discover:
    ///
    /// | the data | what is drawn |
    /// |---|---|
    /// | empty | the grid, the two axes and the labels; **no series at all** |
    /// | one sample, [`ChartType::Line`] or [`ChartType::Area`] | the grid, the axes and the labels; **no series**, because a segment needs two points |
    /// | one sample, [`ChartType::Bar`] | the grid, the axes, the labels and **one bar**, at `BAR_WIDTH_FRACTION` of the plot's width, centred |
    /// | every finite sample the same number | the series **through the middle of the plot**: a range with no room in it has no place to put a reading, and both edges of a plot are edges rather than positions |
    /// | a sample outside a fixed range | the reading **pinned to the plot's edge**, so an out-of-range sample is visible as a clipped reading |
    /// | a `NaN` or infinite sample | **no series through it**, and **the two runs either side of it keep all of their own segments**: a line or area chart's run is broken there and each side is capped flat at the sample beside the gap, and a bar chart draws no bar for it. A gap is an *absent* neighbour rather than a neighbour of no direction, so it costs the segment that would have spanned it and nothing else — `a_gap_ends_a_run_and_the_segments_beside_it_are_still_drawn` |
    /// | **two samples at the same point** — the same `x` *and* the same reading | **no segment**, because a stroke is `line_width / 2` either side of a *direction*, and two identical points have none |
    /// | **two samples at the same `x`, at different readings** | **a vertical segment**, `line_width` wide, spanning the two readings: they are different points, so the direction is straight up and the join at each end is a flat cap. **This row used to claim the opposite** — it read "a series of one distinct `x` twice" and said the result was "a segment of no area", which conflated this with the row above it |
    /// | **a run that doubles back on itself**, so a middle sample's turn is a full reversal | **no rule at all — it is a floating-point accident**, and the two outcomes are measured in the paragraph below. A reversal has no mitre, so whether the join is `None` comes down to whether `1 + p·q` is *exactly* zero in `f32`, and that depends on the coordinates rather than on the turn |
    /// | fewer x labels than samples | the labels for the samples that have one, **unshifted** — sample `i` is labelled by `x_labels[i]` or by nothing |
    /// | more x labels than samples | the labels for the samples that exist; the rest are **not drawn**, for there is nothing under them |
    ///
    /// A chart with a plot of no width or no height draws its labels and nothing
    /// else: there is nowhere for a grid line, an axis or a sample to go.
    ///
    /// **Those three rows are one predicate, and it is not the one the table used
    /// to name.** What is skipped is a segment whose **direction** is zero, because
    /// `normal_of` answers `None` for it, `join_at` turns that into a join of
    /// `None`, and a segment needs a join at **both** of its ends. It is emphatically
    /// not "two samples at the same `x`": that draws a vertical rule.
    ///
    /// **A repeated *point* is the deterministic half of this, and it is the one a
    /// caller can rely on.** A repeat between vertices `i` and `i+1` leaves
    /// `joins[i]` undefined — its *leaving* direction is zero — and `joins[i + 1]`
    /// undefined, because its *arriving* direction is zero. Each undefined join is
    /// read by the segments on **both** sides of it, so **three consecutive
    /// segments are lost**, `i - 1`, `i` and `i + 1`, and drawing resumes at
    /// `i + 2`: the run is **broken there, not truncated**. Measured: one repeat in
    /// five samples draws **1 of 4**; two repeats in five,
    /// `x = [0, 0.5, 0.5, 1, 1]` over `values = [1, 5, 5, 1, 1]`, draw **0 of 4**,
    /// because the pairs of lost segments overlap. Reaching any of it needs `x`
    /// written by hand, because every supported path derives `x` from the sample
    /// index and so makes it strictly increasing.
    ///
    /// **A full reversal is the other half, and it is not a rule.** Whether the
    /// join comes back `None` is decided by `1 + p·q`, and the two operands of that
    /// are `f32`:
    ///
    /// - **axis-aligned**, so the normals are exactly `±(1, 0)` and the dot is
    ///   exactly `-1.0`: `1 + p·q` is exactly `0.0`, the join is `None`, and the
    ///   segments on both sides are dropped;
    /// - **oblique**, so the dot is a product of two divisions and lands a rounding
    ///   step either side of `-1` **according to the coordinates** — and the plot's
    ///   height is one of the coordinates, so the branch can be changed by resizing
    ///   the rect and nothing else.
    ///
    /// The second branch is a **defect**, not a degenerate case, and it is the
    /// integrator's to schedule rather than this round's to change. With the
    /// denominator small and positive, `scale = half / (1 + p·q)` is enormous and the
    /// corner is that scale times `p + q`, which **cancels to exactly zero** because
    /// the two normals are computed as exact negatives. The corner is then `(0, 0)`,
    /// its length is `0`, and **`0 <= MITRE_LIMIT · half` accepts it as a *shorter*
    /// join**: `join_at` returns `Join::Corner((0, 0))` and the segment quad inherits
    /// a zero offset, so **two of its four corners are the same point**. The
    /// mitre-limit test cannot tell a short corner from no corner at all, and that is
    /// the bug in one sentence. Measured at `x = [0, 0.5, 1, 0.5, 1]` over
    /// `values = [1, 5, 9, 5, 1]`: **2 of 4 segments at a 300-pixel plot height and
    /// 4 of 4 at 240, the second of the four degenerate.** Two heights, one fixture,
    /// and the difference between them is which way the multiply rounded.
    /// `a_repeated_x_draws_a_vertical_rule_and_a_repeated_point_draws_nothing` pins
    /// the deterministic half and `a_full_reversal_depends_on_which_way_f32_rounds`
    /// pins all three rows of this, through the public API.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{DrawCommand, Rect};
    /// use ui_core::widgets::chart::{Chart, ChartType};
    ///
    /// let mut nodes = Arena::new();
    /// let chart = Chart::new(&mut nodes, ChartType::Line);
    /// let rect = Rect::new(300.0, 120.0, 480.0, 240.0);
    ///
    /// // Nothing to plot: the grid and the two axes, and no segment.
    /// assert_eq!(charts(&chart.paint(rect)), 0);
    ///
    /// chart.data.set(vec![0.0, 1.0, 0.0]);
    /// chart.snap_to_state();
    /// assert_eq!(charts(&chart.paint(rect)), 2, "two segments for three samples");
    ///
    /// fn charts(commands: &[DrawCommand]) -> usize {
    ///     commands
    ///         .iter()
    ///         .filter(|c| matches!(c, DrawCommand::Polygon { .. }))
    ///         .count()
    /// }
    /// ```
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let plot = self.plot_rect(rect);
        let range = self.y_range();
        if plot.width > 0.0 && plot.height > 0.0 {
            self.draw_grid(&mut painter, plot);
            match self.chart_type {
                ChartType::Bar => self.draw_bars(&mut painter, plot, range),
                ChartType::Area => {
                    self.draw_fill(&mut painter, plot, range);
                    self.draw_series(&mut painter, plot, range);
                }
                ChartType::Line => self.draw_series(&mut painter, plot, range),
            }
            self.draw_axes(&mut painter, plot);
        }
        self.draw_labels(&mut painter, plot);
        painter.finish()
    }

    /// Returns the series the truth maps to when it has arrived: every sample of
    /// [`data`](Chart::data) at its own even place across the plot.
    ///
    /// It is what [`style`](Chart::style) reports and what
    /// [`snap_to_state`](Chart::snap_to_state) writes, and it is *not* what
    /// [`paint`](Chart::paint) reads: a frame mid-transition paints
    /// [`shown`](Chart::shown).
    ///
    /// **One sample sits in the middle of the plot.** `i / (len − 1)` is `0/0` for
    /// a single sample, and the two ends of a plot are both edges rather than
    /// positions — so a chart of one reading puts it where a reader's eye already
    /// is, and a bar chart's one bar lands in the middle of its plot rather than
    /// against its y axis. A line and an area chart draw no series at all for one
    /// sample, so the same choice costs them nothing.
    fn target_series(&self) -> Series {
        let values = self.data.get();
        let gaps = one_fewer(values.len());
        let x = (0..values.len())
            .map(|index| {
                if gaps == 0 {
                    HALF
                } else {
                    count_to_f32(index) / count_to_f32(gaps)
                }
            })
            .collect();
        Series { x, values }
    }

    /// Returns the transition to run toward [`style`](Chart::style)'s series on
    /// `motion`, or `None` when there is nothing to run.
    ///
    /// `None` is the answer for a chart whose drawn series already *is* the
    /// target, and it matters: a glide that changes nothing would still return
    /// `true` from every [`tick`](Chart::tick) for the whole of its duration,
    /// which is a caller repainting forever for a frame in which nothing moved.
    ///
    /// The new sample's **starting reading is the one before it**, not zero and
    /// not the target. A sample that arrives at zero draws a spike down to the
    /// plot's bottom on its first frame, which is the "jarring jump" requirement 5
    /// is about; a sample that arrives at the previous one's reading draws a flat
    /// stub where the line already ended, and glides from there. Its starting
    /// **place** is `1.0` for the same reason and not `0.0`: `0.0` would put the
    /// new sample on top of the oldest one.
    ///
    /// A sample whose index is past the drawn series gets the last drawn reading
    /// and the far edge; two new samples get the same two, so they are stacked at
    /// the end of the run until they have places of their own.
    fn aim_glide(&self, motion: Motion) -> Option<Glide> {
        let to = self.target_series();
        let shown = self.shown.get();
        let count = to.len();
        let drawn = shown.len();
        // The last reading the chart is currently showing, which is where a new
        // sample comes from. A chart showing nothing has no such reading, and its
        // first sample starts at its own target — there is nothing else for it to
        // have come from.
        let last_shown = shown
            .values
            .get(one_fewer(shown.values.len()))
            .copied()
            .filter(|_| !shown.values.is_empty());
        let from = Series {
            x: (0..count)
                .map(|index| if index < drawn { shown.x[index] } else { 1.0 })
                .collect(),
            values: (0..count)
                .map(|index| {
                    if index < drawn {
                        shown.values[index]
                    } else {
                        last_shown.unwrap_or(to.values[index])
                    }
                })
                .collect(),
        };
        let grew = count > drawn;
        let from_reveal = if self.chart_type != ChartType::Bar || !grew {
            1.0
        } else {
            0.0
        };
        if from_reveal >= 1.0 && from.x == to.x && from.values == to.values {
            return None;
        }
        Some(Glide {
            from,
            to,
            elapsed: Duration::ZERO,
            duration: motion.duration,
            easing: motion.easing,
            from_reveal,
        })
    }

    /// Advances the series glide by `delta`, writes what it has reached, and
    /// returns whether it wrote.
    ///
    /// The last frame writes the target **exactly** rather than the last lerp's
    /// float: a transition that finishes on `from + (to − from)` lands a few
    /// millionths away from the truth on some curves, and a chart whose drawn
    /// series never quite reaches its own data is a chart whose next transition
    /// starts from a number nothing else holds.
    ///
    /// `reveal` travels from `from_reveal` to `1.0` rather than from `0.0`, which
    /// is the difference between a bar chart whose newest bar rises when a sample
    /// arrives and a bar chart whose **every** bar vanishes and comes back on any
    /// change to the data. `from_reveal` is `0.0` only when the series grew; see
    /// [`Chart::aim_glide`].
    ///
    /// It is bounded rather than eased raw, because a
    /// [`Spring`](crate::animation::Easing::Spring) passes `1.0` on its way and a
    /// bar that rose past its own height and came back would be a bar drawn outside
    /// the plot.
    fn advance_glide(&self, delta: Duration) -> bool {
        let Some(mut glide) = self.glide.borrow_mut().take() else {
            return false;
        };
        glide.elapsed += delta;
        // A zero-length motion is over rather than infinitely long, so the division
        // below is not reached — the same guard
        // [`Animation::progress_at`](crate::animation::Animation::progress_at)
        // makes, for the same reason.
        let raw = if glide.duration.is_zero() {
            1.0
        } else {
            (glide.elapsed.as_secs_f32() / glide.duration.as_secs_f32()).clamp(0.0, 1.0)
        };
        let bar_mode = self.chart_type == ChartType::Bar;
        if raw >= 1.0 {
            self.shown.set(glide.to);
            if bar_mode {
                self.reveal.set(1.0);
            }
            // Taken out of the slot above and **not** put back: a glide that has
            // arrived is finished, and leaving it there would make every later
            // `tick` write the target again and report that it moved.
            return true;
        }
        let eased = glide.easing.apply(raw);
        let count = glide.to.len();
        self.shown.set(Series {
            x: (0..count)
                .map(|index| {
                    glide.from.x[index] + (glide.to.x[index] - glide.from.x[index]) * eased
                })
                .collect(),
            values: (0..count)
                .map(|index| {
                    glide.from.values[index]
                        + (glide.to.values[index] - glide.from.values[index]) * eased
                })
                .collect(),
        });
        if bar_mode {
            self.reveal
                .set((glide.from_reveal + (1.0 - glide.from_reveal) * eased).clamp(0.0, 1.0));
        }
        *self.glide.borrow_mut() = Some(glide);
        true
    }

    /// Returns the part of `rect` the series is drawn in: `rect` less the gutter
    /// each labelled axis needs.
    ///
    /// The two gutters are the rect's **own** origin and **own** extents taken
    /// away, and the distinction is the slider suite's whole lesson: a rect's
    /// origin and a rect's extent are different numbers, and every one of them
    /// has to be read. Both results are `.max(0.0)`, because a rect narrower than
    /// its own y gutter leaves a plot of no width rather than a negative one.
    ///
    /// A gutter is reserved **only when there are labels for it**, so an
    /// unlabelled chart gets the whole of its rect.
    fn plot_rect(&self, rect: Rect) -> Rect {
        let left = if self.y_labels.get().is_empty() {
            0.0
        } else {
            Y_LABEL_GUTTER
        };
        let bottom = if self.x_labels.get().is_empty() {
            0.0
        } else {
            X_LABEL_GUTTER
        };
        Rect::new(
            rect.x + left,
            rect.y,
            (rect.width - left).max(0.0),
            (rect.height - bottom).max(0.0),
        )
    }

    /// Returns where on `plot` a reading of `value` is drawn, in window
    /// coordinates.
    ///
    /// The value is **bounded into the range** first, which is what pins an
    /// out-of-range reading to the plot's edge and is why an area chart's fill
    /// quads can be convex by construction: every point on the line is at or
    /// above the bottom edge, so no segment can straddle the edge the fill drops
    /// to.
    ///
    /// **A range with no room in it puts every reading in the middle of the plot.**
    /// `high − low` is zero whenever every finite sample is the same number, or
    /// whenever a caller fixed the range at one value, and dividing by it is the
    /// one thing this widget must not do to say so. Both edges of a plot are edges
    /// rather than positions, and the middle is the one place that is inside the
    /// plot for every stroke width — so a flat series is drawn through it and a
    /// bar of one value in an unmeasurable range rises to it.
    ///
    /// A `NaN` range gives the middle as well: the question `high > low` asks is
    /// whether the chart has any distance along its axis to divide by, and `NaN`
    /// does not answer it. A `NaN` *value* never arrives, because
    /// [`Chart::paint`] draws no sample that has no reading.
    fn y_of(&self, plot: Rect, range: Option<(f32, f32)>, value: f32) -> f32 {
        let Some((low, high)) = range else {
            return plot.y + plot.height / 2.0;
        };
        if !spanned(low, high) {
            return plot.y + plot.height / 2.0;
        }
        let bounded = bounded(value, low, high);
        plot.y + (high - bounded) / (high - low) * plot.height
    }

    /// Returns the y a [`ChartType::Bar`] chart's bars grow from and hang
    /// towards, in window coordinates.
    ///
    /// **Zero when zero is in the range, and the plot's own edge otherwise.**
    /// A bar is a magnitude from zero — that is what a bar chart is *for*, and a
    /// negative reading has to be able to hang below the line — so a range that
    /// straddles zero has its baseline at zero's own y and a negative sample's
    /// bar runs downward from it. A range entirely above zero has nothing to hang
    /// below and grows from the bottom; a range entirely below grows from the
    /// top.
    ///
    /// **This is deliberately not where an area chart's fill goes.** A fill runs
    /// to the plot's own bottom edge, because an area is *the region under a
    /// curve* and the bottom edge is that region's boundary; a fill that stopped
    /// at zero would leave the region below a negative reading uncovered, which
    /// is the opposite of what a filled area means. The two differ, and the module
    /// says so rather than leaving a reader to find it.
    ///
    /// A range with no room in it has no zero to speak of, and the plot's bottom
    /// is where a bar of one value in an unmeasurable range belongs.
    fn bar_baseline(&self, plot: Rect, range: Option<(f32, f32)>) -> f32 {
        let Some((low, high)) = range else {
            return plot.y + plot.height;
        };
        if !spanned(low, high) {
            return plot.y + plot.height;
        }
        if low <= 0.0 {
            self.y_of(plot, range, 0.0)
        } else {
            plot.y + plot.height
        }
    }

    /// Returns the drawn points of the series inside `plot`: each sample's place
    /// on the plot in window coordinates, with a `NaN` y for a sample that has
    /// no reading.
    ///
    /// The `NaN` is the marker rather than an `Option` because the two consumers
    /// want different things from a gap: a line and an area chart have to **break
    /// the run** there, and a bar chart has to skip the bar. One marker serves
    /// both, and `is_finite` is the question both ask.
    ///
    /// The two halves of the series are read as far as the **shorter** one goes,
    /// for the reason [`Series`] documents.
    fn points(&self, plot: Rect, range: Option<(f32, f32)>) -> Vec<(f32, f32)> {
        let series = self.shown.get();
        (0..series.len())
            .map(|index| {
                let x = plot.x + bounded(series.x[index], 0.0, 1.0) * plot.width;
                let value = series.values[index];
                let y = if value.is_finite() {
                    self.y_of(plot, range, value)
                } else {
                    f32::NAN
                };
                (x, y)
            })
            .collect()
    }

    /// Draws the grid: `GRID_DIVISIONS - 1` horizontal lines across the plot.
    ///
    /// **Horizontal only**, and [`Chart::set_grid_visible`] gives the reason: a
    /// vertical line per sample is a picket fence past a dozen samples, and the x
    /// positions are already named by the labels and by the series.
    ///
    /// A [`DrawCommand::Line`] is the right primitive for all of them, and for the
    /// two axes, for the reason the gauge gives for its tick marks: `line_quad`
    /// offsets a segment perpendicular to it, and a horizontal or a vertical line
    /// *is* the offset direction, so a `GRID_WIDTH`-pixel line is drawn
    /// `GRID_WIDTH` pixels wide with no error in it. A line has no joins to get
    /// wrong either, which is the whole of what
    /// [`Chart::paint`]'s series cannot say about itself.
    fn draw_grid(&self, painter: &mut Painter, plot: Rect) {
        if !self.show_grid {
            return;
        }
        let divisions = count_to_f32(GRID_DIVISIONS);
        for index in 1..GRID_DIVISIONS {
            let y = plot.y + plot.height * count_to_f32(index) / divisions;
            painter.line(
                (plot.x, y),
                (plot.x + plot.width, y),
                GRID_WIDTH,
                self.grid.get(),
            );
        }
    }

    /// Draws the two axes: the y axis up the plot's left edge and the x axis
    /// along its bottom one.
    ///
    /// Both are [`DrawCommand::Line`]s for the reason
    /// [`Chart::draw_grid`] gives. They are drawn **after** the data, which is
    /// what makes a bar that reaches the bottom of the plot read as standing *on*
    /// the axis rather than under it: the axis is `AXIS_WIDTH` wide about
    /// `plot.y + plot.height`, so half of it is over the bar and half is under
    /// it, and only one of those orders shows the line.
    fn draw_axes(&self, painter: &mut Painter, plot: Rect) {
        let bottom = plot.y + plot.height;
        let right = plot.x + plot.width;
        let color = self.axis.get();
        painter.line((plot.x, plot.y), (plot.x, bottom), AXIS_WIDTH, color);
        painter.line((plot.x, bottom), (right, bottom), AXIS_WIDTH, color);
    }

    /// Draws one rectangle per sample with a reading, from the bar chart's
    /// baseline to the reading's own place.
    ///
    /// The bar is centred on its sample's `x` and is
    /// `Chart::bar_width` wide. A reading with **no height** — at the baseline,
    /// or `NaN` — is not recorded at all rather than recorded as a rectangle of
    /// nothing, which is what the progress bar does for an empty fill and for the
    /// same reason.
    ///
    /// A reading below the baseline gives a rectangle whose top is *below* its
    /// bottom, and the `min` and the `abs` are what turn that back into a bar
    /// hanging downward: the height is always the distance between the two, and
    /// the top is always the smaller of the two.
    fn draw_bars(&self, painter: &mut Painter, plot: Rect, range: Option<(f32, f32)>) {
        let points = self.points(plot, range);
        if points.is_empty() {
            return;
        }
        let baseline = self.bar_baseline(plot, range);
        let width = self.bar_width(plot, points.len());
        let newest = one_fewer(points.len());
        let reveal = bounded(self.reveal.get(), 0.0, 1.0);
        for (index, (x, y)) in points.iter().enumerate() {
            if !y.is_finite() {
                continue;
            }
            // The rise applies to the newest bar alone: a bar chart's readings
            // moving is a bar's top moving, which is a transition on its own, and
            // scaling every bar by it would have the whole chart shrink and grow
            // around a value that only the last one moved.
            let height = (baseline - y).abs() * if index == newest { reveal } else { 1.0 };
            if height <= 0.0 {
                continue;
            }
            // **The centre is pulled in so the bar stays inside the plot.** A bar
            // is centred on its own sample wherever it can be, and an evenly
            // spaced series puts its first sample on the plot's left edge and its
            // last on the right, so a bar centred there would hang half its own
            // width over both. The end bars are therefore shifted inward until
            // their outer edges land exactly on the plot's edges; **every interior
            // bar is still centred on its own sample**, which is where the reading
            // is. The shift is `width / 2` — a third of a pitch at the default
            // fraction — and it is on the two end bars alone.
            let half_width = width / 2.0;
            let centre = bounded(*x, plot.x + half_width, plot.x + plot.width - half_width);
            painter.rect(
                Rect::new(centre - half_width, baseline.min(*y), width, height),
                self.series.get(),
            );
        }
    }

    /// Returns how wide a bar is, in pixels: `BAR_WIDTH_FRACTION` of the width a
    /// **slot** would have, at least `MIN_BAR_WIDTH` and never more than the plot.
    ///
    /// **A slot is `plot.width / count`, not the spacing between the samples**, and
    /// the difference is not a detail: the samples span the whole plot, so their
    /// spacing is `plot.width / (count − 1)`, and a bar centred on the first of
    /// them hangs half its own width over the plot's left edge. Both end bars are
    /// pulled in to stop that (see [`Chart::draw_bars`]), which moves them
    /// **towards each other** — and on two samples, at three fifths, the
    /// `(count − 1)` pitch of 600 px gives 360 px bars whose pulled-in centres are
    /// 240 px apart, so the two bars **overlap by 120 px**. With the slot pitch of
    /// 300 px the bars are 180 px wide and their centres are 420 px apart, which
    /// leaves a 240 px gap.
    ///
    /// The slot pitch is what makes the gaps provably positive for every count:
    /// with bars at `f·width/n` and centres at most `width − f·width/(2n)` apart
    /// from the ends, the smallest gap is `width·(n − f·(n−1)) / (n·(n−1))` =
    /// `width·(n·(1−f) + f) / (n·(n−1))`, which is positive for every `n ≥ 2` and
    /// every `f ≤ 1`. **That is the whole reason it is a slot.**
    ///
    /// A caller whose samples are unevenly spaced gets evenly sized bars at
    /// unevenly spaced positions, which is the ordinary reading of a bar chart:
    /// the position carries the reading and the width carries nothing.
    ///
    /// One sample has no slots to divide, so its slot is the whole plot: a chart of
    /// one reading is one bar at `BAR_WIDTH_FRACTION` of the width, which is the
    /// ordinary reading of one reading.
    fn bar_width(&self, plot: Rect, count: usize) -> f32 {
        let slot = if count == 0 {
            plot.width
        } else {
            plot.width / count_to_f32(count)
        };
        (slot * self.bar_fraction)
            .max(MIN_BAR_WIDTH)
            .min(plot.width.max(MIN_BAR_WIDTH))
    }

    /// Draws an area chart's body: one convex quad per segment of every run,
    /// dropping from the segment's two points to the plot's own bottom edge.
    ///
    /// **Not one polygon for the whole region**, which is concave and would fan
    /// into triangles over it and past it; see the module document. **And not
    /// translucent**, because neighbouring quads share their vertical edges and
    /// cover them twice; see [`Palette::fill`].
    ///
    /// The quads are convex **by construction** rather than by luck: every point
    /// on the line is at or above the plot's bottom edge, because
    /// [`Chart::y_of`] bounds every reading into the range first, so no segment
    /// can straddle the edge they drop to.
    fn draw_fill(&self, painter: &mut Painter, plot: Rect, range: Option<(f32, f32)>) {
        let points = self.points(plot, range);
        let bottom = plot.y + plot.height;
        let color = self.fill.get();
        each_run(&points, |start, end| {
            for index in start..end {
                let (x0, y0) = points[index];
                let (x1, y1) = points[index + 1];
                painter.polygon(&[(x0, y0), (x1, y1), (x1, bottom), (x0, bottom)], color);
            }
        });
    }

    /// Draws the line: a disc at every vertex whose mitre is past
    /// `MITRE_LIMIT`, and then one mitred convex quad per segment of every run.
    ///
    /// The discs come **first**, so a run's discs are all together and a reader of
    /// the recorded commands does not have to skip over segments to find them.
    ///
    /// Every quad is convex, which is
    /// [`DrawCommand::Polygon`]'s precondition; the module document has the proof
    /// and `every_segment_quad_is_convex_over_two_hundred_thousand_geometries`
    /// asserts it against a search as well.
    ///
    /// **A gap is filtered out of the neighbour question below**, and that filter
    /// is load-bearing rather than tidiness: the first draft asked [`join_at`]
    /// about the point on the far side of a `NaN`, whose direction is a `NaN`,
    /// whose normal is therefore `None` — and **a `None` join is read by the
    /// segments on both sides of it**, so the two real segments either side of
    /// every gap went with it. Filtering makes the vertex ask the question the
    /// *end of a run* asks, and [`join_at`]'s own answer to that is a flat cap of
    /// exactly `half`. [`each_run`] has the rule; this is the only place that acts
    /// on it, and `a_gap_ends_a_run_and_the_segments_beside_it_are_still_drawn`
    /// pins the result through [`Chart::paint`].
    fn draw_series(&self, painter: &mut Painter, plot: Rect, range: Option<(f32, f32)>) {
        let points = self.points(plot, range);
        let half = self.line_width / 2.0;
        if half <= 0.0 {
            return;
        }
        let color = self.series.get();
        // A gap is an absent neighbour, not one of no direction — see this
        // function's own document, and `each_run`.
        let joins: Vec<Option<Join>> = points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                join_at(
                    *point,
                    index
                        .checked_sub(1)
                        .map(|before| points[before])
                        .filter(|(_, y)| y.is_finite()),
                    points
                        .get(index + 1)
                        .copied()
                        .filter(|(_, y)| y.is_finite()),
                    half,
                )
            })
            .collect();
        // The discs come before the segments, so a reader of the recorded commands
        // finds them without skipping over the segments in between. Only a vertex
        // *inside* a run has a turn at all, and only a turn can exceed the limit:
        // the two ends of a run have one neighbour and therefore a ratio of
        // exactly one.
        for (index, point) in points.iter().enumerate() {
            if index > 0 && index + 1 < points.len() {
                if let Some(Join::Flat { .. }) = joins[index] {
                    painter.circle(*point, half, color);
                }
            }
        }
        each_run(&points, |start, end| {
            for index in start..end {
                let (Some(here), Some(there)) = (joins[index], joins[index + 1]) else {
                    continue;
                };
                // Each side of the segment reads its **own** offset from the join
                // at that end. Reading the same offset for both is what made a
                // quad non-convex past the mitre limit, because the mitre's two
                // offsets are symmetric about the vertex while the flat offsets
                // are not: one is the perpendicular of the run arriving and one
                // is the perpendicular of the run leaving, and for a sharp turn
                // they are on opposite sides of the segment between them.
                let (a, b) = (points[index], points[index + 1]);
                let start = here.outgoing();
                let end = there.incoming();
                painter.polygon(
                    &[
                        (a.0 + start.0, a.1 + start.1),
                        (b.0 + end.0, b.1 + end.1),
                        (b.0 - end.0, b.1 - end.1),
                        (a.0 - start.0, a.1 - start.1),
                    ],
                    color,
                );
            }
        });
    }

    /// Draws the labels: the y labels in the left gutter, then the x labels in the
    /// bottom one.
    ///
    /// **Placed, never measured**, because [`DrawCommand::Text`] carries no width
    /// and nothing outside the text pipeline knows how far a run reaches. So:
    ///
    /// - a **y** label is left-aligned in the gutter with its line box's top
    ///   [`LABEL_FONT_SIZE / 2`] above the mark it belongs to, which centres it on
    ///   the mark to within the difference between the line's height and its font
    ///   size;
    /// - an **x** label is left-aligned at **its own sample's `x`** and **overhangs
    ///   the plot's right edge by its own width**, which this widget cannot know.
    ///   The caller's answers are fewer labels and shorter ones.
    ///
    /// One label marks one sample: `x_labels[i]` labels sample `i` and a sample
    /// with no label at its index is drawn without one, so a caller labelling every
    /// third sample writes an empty string at the others' indices. Labels past the
    /// last sample are not drawn at all.
    fn draw_labels(&self, painter: &mut Painter, plot: Rect) {
        let color = self.label.get();
        let y_labels = self.y_labels.get();
        if !y_labels.is_empty() {
            // One label has no neighbour to spread against and is at the top,
            // which is where index zero of any number is. Dividing by `last`
            // without this would be a division by zero and a `NaN` label.
            let last = one_fewer(y_labels.len());
            for (index, label) in y_labels.iter().enumerate() {
                let y = if last == 0 {
                    plot.y
                } else {
                    plot.y + plot.height * count_to_f32(index) / count_to_f32(last)
                };
                painter.text(
                    plot.x - Y_LABEL_GUTTER,
                    y - LABEL_FONT_SIZE / 2.0,
                    label,
                    color,
                    LABEL_FONT_SIZE,
                    0.0,
                );
            }
        }
        let x_labels = self.x_labels.get();
        if !x_labels.is_empty() {
            let series = self.shown.get();
            let count = series.len();
            for (index, label) in x_labels.iter().enumerate() {
                if index >= count {
                    continue;
                }
                let x = plot.x + bounded(series.x[index], 0.0, 1.0) * plot.width;
                painter.text(
                    x,
                    plot.y + plot.height + X_LABEL_GAP,
                    label,
                    color,
                    LABEL_FONT_SIZE,
                    0.0,
                );
            }
        }
    }
}

/// Returns how a stroked run meets at one of its vertices.
///
/// A join is asked for **per vertex** rather than per segment, and each of the
/// two segments at that vertex asks it for **its own side's** offset. That is the
/// whole of what makes every segment quad convex; the module document has the
/// proof and the failure this avoids.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Join {
    /// One point `half` from the vertex, along the bisector of the two segments'
    /// normals: the outer corner of a mitre. Its length is `half/cos(φ/2)`, which
    /// is `half` on a straight run and unbounded as `φ` approaches a full
    /// reversal.
    Corner((f32, f32)),
    /// Each segment keeps its own perpendicular offset, `half` from the vertex,
    /// and the corner between them is covered by a disc of radius `half` at the
    /// vertex instead.
    ///
    /// Which is `MITRE_LIMIT`'s other end, and the disc is not an approximation
    /// of the mitre: the wedge the two flat offsets leave empty is the triangle
    /// `v, v + half·p, v + half·q`, whose two outer vertices are at distance
    /// `half` from `v` and whose third vertex is `v`, so the disc of radius `half`
    /// about `v` contains it and touches its boundary at exactly those two points
    /// — which are also where each segment's outer edge line touches the disc, so
    /// it reaches past neither. SVG's own rule for a miter limit.
    Flat {
        /// The offset for the run **arriving** at the vertex.
        incoming: (f32, f32),
        /// The offset for the run **leaving** it.
        outgoing: (f32, f32),
    },
}

impl Join {
    /// Returns the offset for the run arriving at the vertex.
    ///
    /// Its length is `half` for a [`Flat`](Join::Flat) join and
    /// `half/cos(φ/2)` for a [`Corner`](Join::Corner) one, and its dot with the
    /// arriving run's own unit normal is `half` either way — which is what
    /// [`Chart::draw_series`] relies on.
    fn incoming(self) -> (f32, f32) {
        match self {
            Join::Corner(corner) => corner,
            Join::Flat { incoming, .. } => incoming,
        }
    }

    /// Returns the offset for the run leaving the vertex, and the counterpart of
    /// [`incoming`](Join::incoming).
    fn outgoing(self) -> (f32, f32) {
        match self {
            Join::Corner(corner) => corner,
            Join::Flat { outgoing, .. } => outgoing,
        }
    }
}

/// Returns the join at `vertex` for a stroke `half` wide, between a run arriving
/// from `prev` and leaving toward `next`.
///
/// A missing `prev` or `next` is an **end** of the run rather than a corner: the
/// one direction that exists is used for both sides, which makes the mitre `half`
/// long — a flat cap, which is what the end of a stroke is. A vertex with neither
/// is not on a run at all and gives `None`.
///
/// **The other two `None`s, and what each costs.** [`normal_of`] answers `None`
/// when **either** side has no length, so one segment of no length takes its
/// neighbours down with it: a repeat between vertices `i` and `i + 1` leaves
/// `joins[i]` and `joins[i + 1]` undefined, and each is read by the segments on
/// **both** sides of it, so **three consecutive segments are lost**, `i - 1`, `i`
/// and `i + 1`, and drawing resumes at `i + 2`. That rule is stated once, in
/// [`Chart::paint`] — the paragraph beginning *"A repeated **point** is the
/// deterministic half of this"* — and
/// `a_repeated_x_draws_a_vertical_rule_and_a_repeated_point_draws_nothing` pins it.
/// **The paragraph is cited by its words rather than by a `file:line`**, because a
/// line number inside the file it is quoted from goes stale the next time this
/// module is edited, and a `file:line` that no longer names what it was evidence
/// for is worse than no citation at all.
///
/// The third `None` is a **full reversal**, where `1 + p·q` is zero and **no
/// segment is of no length** — which is the half of this doc's old claim that was
/// simply wrong. That branch is [`Chart::paint`]'s next paragraph, *"A full
/// reversal is the other half, and it is not a rule"*, where the outcome is decided
/// by which way `f32` rounds rather than by the turn, and where the oblique
/// branch is recorded as **a defect, not a degenerate case**.
///
/// [`Chart::draw_series`] skips a segment with a `None` end rather than reaching
/// for an unwrap.
///
/// The limit is `MITRE_LIMIT` and the arithmetic is the module document's.
fn join_at(
    vertex: (f32, f32),
    prev: Option<(f32, f32)>,
    next: Option<(f32, f32)>,
    half: f32,
) -> Option<Join> {
    let arriving = match (prev, next) {
        (Some(prev), _) => (vertex.0 - prev.0, vertex.1 - prev.1),
        (None, Some(next)) => (next.0 - vertex.0, next.1 - vertex.1),
        (None, None) => return None,
    };
    let leaving = match next {
        Some(next) => (next.0 - vertex.0, next.1 - vertex.1),
        None => arriving,
    };
    let before = normal_of(arriving)?;
    let after = normal_of(leaving)?;
    let dot = before.0 * after.0 + before.1 * after.1;
    let denominator = 1.0 + dot;
    if !positive(denominator) {
        return None;
    }
    let scale = half / denominator;
    let corner = (scale * (before.0 + after.0), scale * (before.1 + after.1));
    // The two sides of the vertex, each as the length and the direction of the
    // segment it belongs to. The arriving run *is* the left segment when there is
    // one, and is the leaving run when there is not — an end of a run, where the
    // corner is a flat cap of exactly `half` and there is no disc to draw.
    let left = prev.map(|prev| magnitude((vertex.0 - prev.0, vertex.1 - prev.1)));
    let right = next.map(|next| magnitude((next.0 - vertex.0, next.1 - vertex.1)));
    if magnitude(corner) <= MITRE_LIMIT * half
        && carries_a_corner(corner, arriving, left)
        && carries_a_corner(corner, leaving, right)
    {
        return Some(Join::Corner(corner));
    }
    Some(Join::Flat {
        incoming: (half * before.0, half * before.1),
        outgoing: (half * after.0, half * after.1),
    })
}

/// Returns whether a segment of length `length` along `direction` can carry the
/// mitre `corner` at its end without the two of them making its quad concave.
///
/// **This is the second of the two reasons a join falls back to being flat, and it
/// is the one the mitre limit cannot see.** A mitre corner is `half` from the
/// centre line *along the normal* and swings an unbounded distance *along* the
/// segment, and a segment quad `[a + A, b + B, b − B, a − A]` is convex exactly
/// when `|A·d̂ − B·d̂| < L` — write the offsets in a frame whose `y` axis is the
/// segment's normal and the four corners are `(t_A, half)`, `(L + t_B, half)`,
/// `(L − t_B, −half)`, `(−t_A, −half)`, whose four turns carry the factors
/// `−2·half·(L + t_B − t_A)` at the first and last and `−2·half·(L + t_A − t_B)`
/// at the two in between. They agree in sign — which is convexity — exactly when
/// **`|t_A − t_B| < L`**.
///
/// So a mitre whose swing reaches further along a segment than that segment is
/// long makes its quad concave, and the renderer fans a concave polygon into
/// overlapping and outside triangles. It is not a corner case: on a 600-pixel
/// plot a **201-sample** chart has a 3-pixel pitch, so at a 90-degree turn
/// `half·tan(45°) = half = 1.5` is half the segment's own length and the two
/// factors are already of opposite sign. `MITRE_LIMIT` does not catch it — that
/// mitre is `1.414·half`, comfortably inside the limit.
///
/// **The rule is `|t| < L/2` per side**, which is stricter than the `|t_A − t_B| <
/// L` it needs, and that is what lets it be decided **per vertex** and settled in
/// one pass: the triangle inequality gives `|t_A − t_B| ≤ |t_A| + |t_B| < L`, and
/// the segment's two ends are the two ends of one of the vertex's sides.
///
/// It costs nothing on a chart a person would read: `|t| ≤ |corner| ≤
/// MITRE_LIMIT·half = 4·half = 6` px, so no corner is affected while the pitch
/// stays above 12 px — **50 samples on a 600-pixel plot**, `600/(n−1) > 12`, so
/// `n = 50` at 12.245 px and `n = 51` at exactly 12. Below that the
/// segments really are shorter than the mitres and rounding those corners is the
/// right answer rather than a fallback.
///
/// `length` is the length of the segment this side of the vertex is, and `None`
/// for a side the run does not have — the end of a run, where there is no segment
/// to carry the corner and no disc either.
fn carries_a_corner(corner: (f32, f32), direction: (f32, f32), length: Option<f32>) -> bool {
    let Some(length) = length else {
        // The end of a run: there is no segment on this side to carry the corner,
        // so there is nothing to be concave about. The corner is a flat cap of
        // exactly `half` there in any case.
        return true;
    };
    if !positive(length) {
        return false;
    }
    let along = (corner.0 * direction.0 + corner.1 * direction.1) / length;
    along.abs() < length / 2.0
}

/// Returns the unit vector perpendicular to `direction`, or `None` for a
/// direction of no length.
///
/// The rotation is the same on every segment of a run, which is what makes the
/// offsets at a vertex comparable to each other: `Join::Corner`'s bisector is a
/// blend of these, and a run whose segments were rotated inconsistently would
/// have a mitre that pointed nowhere.
///
/// `None` for a `NaN` direction as well as an empty one, because `magnitude`
/// answers `false` for `NaN` and a length nobody can compute is not a length.
fn normal_of(direction: (f32, f32)) -> Option<(f32, f32)> {
    let length = magnitude(direction);
    if length > 0.0 {
        Some((-direction.1 / length, direction.0 / length))
    } else {
        None
    }
}

/// Returns the length of `vector`.
///
/// Spelled out rather than `vector.0.hypot(vector.1)`: the square root is the
/// same number either way and this is the four places the geometry asks, and it is
/// the predicate that answers `false` for a `NaN`, which
/// [`normal_of`](normal_of) and [`join_at`](join_at) both rely on.
fn magnitude(vector: (f32, f32)) -> f32 {
    (vector.0 * vector.0 + vector.1 * vector.1).sqrt()
}

/// Calls `f` with the **segment** index range `start..end` of each maximal run of
/// consecutive samples that have readings.
///
/// The range is a range of *segments*, not of samples: segment `i` is the one
/// from sample `i` to sample `i + 1`, so a run of two samples is the single
/// segment `0..1` and a run of one sample is **no segments at all** and calls
/// `f` not at all. That is the whole of what makes "a line chart cannot draw a
/// segment" true in one place rather than as an off-by-one at each consumer, and
/// it is why no consumer of this writes `index + 1` into a bounds it has not been
/// given.
///
/// **A gap breaks a run** — that is what a `NaN` means — and the sample on either
/// side of it starts and ends a run of its own.
///
/// **And a gap is an *absent* neighbour, not a neighbour of no direction.**
/// [`Chart::draw_series`] filters the non-finite points out of the neighbour
/// question it asks [`join_at`], so the vertex either side of a gap gets that
/// run's **end** join — a flat cap of exactly `half` — and every real segment
/// beside the gap is drawn and capped. The first draft of this change did not
/// filter, so a `NaN` neighbour produced a direction of `NaN`, `normal_of`
/// answered `None` for it, and a `None` join is read by the segments on **both**
/// sides of it: the two real segments adjacent to every gap were dropped, and
/// an area chart's fill — which walks this function and never consults a join —
/// was drawn across a gap its own outline was missing. The rule lives here
/// because this is the one place "what happens at a gap" is written down for
/// every consumer, and `each_run_reports_the_segments_of_every_run_of_a_gappy_series`
/// pins the ranges while
/// `a_gap_ends_a_run_and_the_segments_beside_it_are_still_drawn` pins what the
/// stroke does with them.
///
/// Every geometry that walks the series goes through here, so "what happens at a
/// gap" is written down once rather than once per consumer.
fn each_run(points: &[(f32, f32)], mut f: impl FnMut(usize, usize)) {
    let mut index = 0;
    while index < points.len() {
        if !points[index].1.is_finite() {
            index += 1;
            continue;
        }
        let start = index;
        while index < points.len() && points[index].1.is_finite() {
            index += 1;
        }
        let last = one_fewer(index);
        if last > start {
            f(start, last);
        }
    }
}

/// Returns `count - 1` for a count of at least one, and zero for none.
///
/// Named rather than written `count - 1` at each of the three places that divide
/// by it, because **one sample has no neighbours and two has one**, and a
/// `count - 1` of `0` is a division by zero rather than a pitch of a whole plot
/// or a plot of no height. Every caller has already refused an empty count.
fn one_fewer(count: usize) -> usize {
    count.saturating_sub(1)
}

/// Converts a count to the float the position arithmetic uses.
///
/// `f32` has no `From<usize>` in std — the `From` impls between integers stop at
/// the 16-bit widths and no float conversion is provided at all — so this is the
/// one place a `usize`-to-`f32` cast happens, for the reason
/// [`List`](crate::widgets::list)'s own `count_to_f32` gives and which
/// [`Keyboard`](crate::widgets::keyboard)'s repeats. The conversion is well
/// defined for every `usize`: the result rounds to the nearest `f32`, and a chart
/// with more samples than that rounding matters for is not a chart this arena can
/// hold. What this module uses it for is three even divisions — a sample's place
/// across the plot, a grid line's place down it and a label's place beside it —
/// and all three are exact for every count under two and a half million.
fn count_to_f32(count: usize) -> f32 {
    count as f32
}

/// Returns `low` and `high` in order, with `low` never above `high`.
///
/// `f32::min` and `f32::max` return the other operand when one of the two is
/// `NaN`, so a caller that has lost track of a bound gets a usable range rather
/// than a `NaN` one. Two `NaN` bounds leave the mapping `NaN`, which
/// [`Chart::y_of`] draws through the middle of the plot rather than taking the
/// frame down for.
fn ordered(low: f32, high: f32) -> (f32, f32) {
    (low.min(high), low.max(high))
}

/// Returns whether `low` and `high` describe a range with room in it.
///
/// **The question a chart asks before it divides**, and it is asked of the *range*
/// rather than of its length: `high − low` is `0.0` for a range of one value and
/// `NaN` for a range of two `NaN`s, and both are cases a chart has to draw
/// something for. [`Chart::y_of`] and [`Chart::bar_baseline`] answer them by
/// putting the series in the middle of the plot and the bars at its bottom
/// respectively, which is why this is a named predicate and not a `length > 0.0`
/// at each of them.
fn spanned(low: f32, high: f32) -> bool {
    high > low
}

/// Returns whether `value` is a number this widget can do geometry with: greater
/// than zero, and not `NaN`.
///
/// The same question [`spanned`](spanned) asks of a range, asked of a single
/// number, for the same reason: `Join`'s denominator `1 + p·q` is a positive
/// quantity for every pair of non-opposite directions and goes to zero — and then
/// to a division by it — only for a run that doubles back on itself.
fn positive(value: f32) -> bool {
    value > 0.0
}

/// Returns `value` between `low` and `high`, `NaN` included.
///
/// The two comparisons rather than [`f32::clamp`], for the reason the progress
/// bar's own `bounded` gives: `clamp` panics when its bounds are the wrong way
/// round, and a widget must not take a frame down to say that a caller passed the
/// wrong pair. `max` then `min` is the same answer for an ordered pair, it cannot
/// panic, and a `NaN` is passed over rather than propagated — `f32::max` returns
/// the *other* operand when one of the two is `NaN`, so a `NaN` value becomes
/// `low` and a `NaN` bound is ignored.
#[allow(clippy::manual_clamp)]
fn bounded(value: f32, low: f32, high: f32) -> f32 {
    value.max(low).min(high)
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback for
/// a token a caller has written the wrong variant into — and black rather than a
/// panic, because a mistyped theme token is not worth taking a frame down for. It
/// is the button's, the slider's, the gauge's and the keyboard's own helper,
/// repeated rather than imported: it is four lines, and a shared module for one
/// four-line helper is a module.
fn token_color(theme: &Theme, token: crate::theme::ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::animation::Easing;
    use crate::paint::PaintState;
    use crate::theme::ThemeToken;

    /// The fixture most of the geometry tests lay a chart out in: 600 by 300 at
    /// (700, 40).
    ///
    /// **It is deliberately not at the origin and deliberately not square**, and
    /// both of those are the slider suite's whole lesson rather than tidiness. An
    /// `x` and a `width` are different numbers, and a chart's geometry touches
    /// both: a gutter is taken off the *width* and added to the *x*, and reading
    /// either as the other puts the plot a whole gutter to the left or the right
    /// of where it belongs. A square rect is the degenerate case for a chart for
    /// the same reason it is for a gauge — a 600-by-300 plot and a 300-by-600 one
    /// have the same two numbers swapped, which is exactly the mistake a
    /// non-square fixture makes visible.
    const WIDE: Rect = Rect {
        x: 700.0,
        y: 40.0,
        width: 600.0,
        height: 300.0,
    };

    /// A second fixture, 200 by 200 at (32, 64), for the tests that want a plot
    /// whose two sides are the same number and must prove they are read
    /// separately.
    const SQUARE: Rect = Rect {
        x: 32.0,
        y: 64.0,
        width: 200.0,
        height: 200.0,
    };

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a
    /// value at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// Advances `chart`'s transitions by `millis` and returns whether anything
    /// moved.
    ///
    /// The chart's own `tick` is what a frame calls and it answers whether a
    /// repaint is needed, so a test that only wants time to pass goes through here
    /// rather than discarding a `#[must_use]` result: that answer is passed on,
    /// not dropped on the floor.
    fn tick(chart: &Chart, millis: u64) -> bool {
        chart.tick(ms(millis))
    }

    /// A themed chart of `chart_type` plotting `data` over `min..=max`, snapped so
    /// the drawn series is where the truth is.
    fn showing(
        chart_type: ChartType,
        data: &[f32],
        min: f32,
        max: f32,
    ) -> (Arena<WidgetNode>, Chart) {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, chart_type);
        chart.set_palette(Palette::from_theme(&Theme::dark()));
        chart.set_fixed_range(min, max);
        chart.data.set(data.to_vec());
        chart.snap_to_state();
        (nodes, chart)
    }

    /// A themed line chart over `min..=max` plotting `data`, snapped.
    fn line(data: &[f32], min: f32, max: f32) -> (Arena<WidgetNode>, Chart) {
        showing(ChartType::Line, data, min, max)
    }

    /// A themed bar chart over `min..=max` plotting `data`, snapped.
    fn bars(data: &[f32], min: f32, max: f32) -> (Arena<WidgetNode>, Chart) {
        showing(ChartType::Bar, data, min, max)
    }

    /// A themed area chart over `min..=max` plotting `data`, snapped, with a
    /// **fill colour of its own**.
    ///
    /// Out of the theme the fill and the series are the same colour — which is
    /// what an area chart wants to look like, and what
    /// `the_theme_gives_the_fill_and_the_series_the_same_colour` asserts — and it
    /// means a test cannot tell a fill quad from a stroke quad by colour. Giving
    /// them apart here keeps every geometry assertion about geometry rather than
    /// about the recorded order.
    fn area(data: &[f32], min: f32, max: f32) -> (Arena<WidgetNode>, Chart) {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Area);
        let mut palette = Palette::from_theme(&Theme::dark());
        palette.fill = FILL;
        chart.set_palette(palette);
        chart.set_fixed_range(min, max);
        chart.data.set(data.to_vec());
        chart.snap_to_state();
        (nodes, chart)
    }

    /// A series of `count` readings over `range` with a single peak at the middle,
    /// every other reading at `low`.
    ///
    /// **The pitch is what makes the turn.** On this fixture `WIDE`'s plot is 600
    /// wide, so `count` samples have a pitch of `600/(count−1)` and the peak's two
    /// sides rise the plot's own height — a slope of `300/pitch` each, and a turn
    /// of `2·atan(300/pitch)`. **21 samples give a 30-pixel pitch, a slope of 10
    /// and a turn of 167.98°**, which is past `MITRE_LIMIT`. Three samples give a
    /// slope of 1 and a turn of 90°, which is nowhere near it: the mitre limit is
    /// not reachable on this plot with two or three samples, and a test that
    /// assumed otherwise would be asserting a number the geometry does not have.
    fn spiking(count: usize, low: f32, high: f32) -> Vec<f32> {
        let middle = count / 2;
        (0..count)
            .map(|index| if index == middle { high } else { low })
            .collect()
    }

    /// The colour an area chart's body is drawn in when a test has to tell it from
    /// the stroke over it.
    const FILL: Color = Color {
        r: 12,
        g: 34,
        b: 56,
        a: 255,
    };

    /// The filled polygons a paint recorded, in order, with the colour of each.
    #[allow(clippy::type_complexity)]
    fn polygons(commands: &[DrawCommand]) -> Vec<(Vec<(f32, f32)>, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Polygon { points, color } => Some((points.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    /// The filled polygons a paint recorded **in `color`**, in order.
    fn filled_with(commands: &[DrawCommand], color: Color) -> Vec<Vec<(f32, f32)>> {
        polygons(commands)
            .into_iter()
            .filter(|(_, drawn)| *drawn == color)
            .map(|(points, _)| points)
            .collect()
    }

    /// The circles a paint recorded, as `(centre, radius)`.
    fn circles(commands: &[DrawCommand]) -> Vec<((f32, f32), f32)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Circle { center, radius, .. } => Some((*center, *radius)),
                _ => None,
            })
            .collect()
    }

    /// The lines a paint recorded, as `(start, end, width)`.
    type Line = ((f32, f32), (f32, f32), f32);

    /// The lines a paint recorded, in the order they were recorded.
    fn lines(commands: &[DrawCommand]) -> Vec<Line> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    start, end, width, ..
                } => Some((*start, *end, *width)),
                _ => None,
            })
            .collect()
    }

    /// The rectangles a paint recorded, in order.
    fn rects(commands: &[DrawCommand]) -> Vec<Rect> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// The text runs a paint recorded, as `(x, y, string, font_size)`.
    #[allow(clippy::type_complexity)]
    fn texts(commands: &[DrawCommand]) -> Vec<(f32, f32, String, f32)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    x,
                    y,
                    text,
                    font_size,
                    ..
                } => Some((*x, *y, text.clone(), *font_size)),
                _ => None,
            })
            .collect()
    }

    /// Asserts two values are within a hundredth of a pixel of one another.
    ///
    /// The products are computed in `f32`, so `300.0 * (1.0 / 3.0)` is not exactly
    /// `100.0`: a test that wrote the decimal out would be asserting the
    /// compiler's rounding rather than the widget. Everything the widget computes
    /// exactly is asserted with `assert_eq!` instead.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!(
            (got - want).abs() < 0.01,
            "{what}: {got} against {want}, within a hundredth of a pixel"
        );
    }

    /// Asserts `point` lies inside `rect`, to within `slack` on each edge.
    ///
    /// `slack` is a stroke's half width wherever a stroke is what is being
    /// measured, because a stroke's corners are offset from its endpoints and the
    /// widget documents that overhang rather than clipping it away.
    /// A short name for a series, so a failure says which one failed.
    fn series_label(data: &[f32]) -> String {
        match data {
            [] => "empty".to_string(),
            [one] => format!("one sample {one}"),
            [a, b] => format!("two samples {a} {b}"),
            _ => format!("{} samples", data.len()),
        }
    }

    /// Every point a command's **geometry** reaches, and nothing else: text is
    /// left out, because [`DrawCommand::Text`] carries no width and its box cannot
    /// be reconstructed (see [`Chart::x_labels`]).
    ///
    /// The offsets are computed the way the painters compute them — a segment's
    /// corners are `± n · width / 2` about its samples, and a disc is its radius
    /// about its vertex — so this measures the drawn geometry and not the samples.
    fn geometry(commands: &[DrawCommand]) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for command in commands {
            match command {
                DrawCommand::Polygon { points, .. } => out.extend(points.iter().copied()),
                DrawCommand::Rect { rect, .. } => out.extend([
                    (rect.x, rect.y),
                    (rect.x + rect.width, rect.y),
                    (rect.x + rect.width, rect.y + rect.height),
                    (rect.x, rect.y + rect.height),
                ]),
                DrawCommand::Line {
                    start, end, width, ..
                } => {
                    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                    let length = (dx * dx + dy * dy).sqrt();
                    if length <= f32::EPSILON {
                        continue;
                    }
                    // The painters' own perpendicular.
                    let (nx, ny) = (-dy / length * width / 2.0, dx / length * width / 2.0);
                    out.extend([
                        (start.0 + nx, start.1 + ny),
                        (end.0 + nx, end.1 + ny),
                        (end.0 - nx, end.1 - ny),
                        (start.0 - nx, start.1 - ny),
                    ]);
                }
                DrawCommand::Circle { center, radius, .. } => {
                    out.extend([-1.0_f32, 0.0, 1.0].iter().flat_map(|dx| {
                        [-1.0_f32, 0.0, 1.0]
                            .iter()
                            .map(move |dy| (center.0 + dx * radius, center.1 + dy * radius))
                    }))
                }
                _ => {}
            }
        }
        out
    }

    /// How far past `rect` `point` lies, per axis: `(left, right, top, bottom)`,
    /// each `0.0` when the point is on that side of `rect`.
    fn outside(rect: Rect, point: (f32, f32)) -> (f32, f32, f32, f32) {
        (
            (rect.x - point.0).max(0.0),
            (point.0 - (rect.x + rect.width)).max(0.0),
            (rect.y - point.1).max(0.0),
            (point.1 - (rect.y + rect.height)).max(0.0),
        )
    }

    /// The furthest past `rect` that any drawn point reaches, as
    /// `(left, right, top, bottom)`.
    fn reach_of(chart: &Chart, rect: Rect) -> (f32, f32, f32, f32) {
        reach_within(&chart.paint(rect), rect)
    }

    /// The same measurement, over a paint that has already been taken.
    ///
    /// Split out so the battery below can measure a chart's geometry **and** its
    /// labels off one paint rather than two — the two are the two bounds its
    /// allowance is the larger of, and reading them from two different paints
    /// would be comparing a chart with its labels against a chart without.
    fn reach_within(commands: &[DrawCommand], rect: Rect) -> (f32, f32, f32, f32) {
        geometry(commands)
            .iter()
            .map(|point| outside(rect, *point))
            .fold((0.0, 0.0, 0.0, 0.0), |worst, here| {
                (
                    worst.0.max(here.0),
                    worst.1.max(here.1),
                    worst.2.max(here.2),
                    worst.3.max(here.3),
                )
            })
    }

    fn assert_near_inside(rect: Rect, point: (f32, f32), slack: f32) {
        let pad = slack + 0.01;
        assert!(
            point.0 >= rect.x - pad
                && point.1 >= rect.y - pad
                && point.0 <= rect.x + rect.width + pad
                && point.1 <= rect.y + rect.height + pad,
            "{point:?} is inside {rect:?} to within {slack}"
        );
    }

    /// Asserts `points` describe a convex polygon the renderer's convex fan draws
    /// exactly: every turn round the edge goes the same way.
    ///
    /// [`DrawCommand::Polygon`] documents that the fan is exact for a convex
    /// polygon and produces overlapping and outside triangles for a concave one, so
    /// this is the precondition for the primitive at all — and it is a property of
    /// the *order the points were given in*, not of the shape they describe. A
    /// segment whose two offsets were read from the wrong join would still have
    /// four corners in two pairs; it would be a bow tie, and this is what says so.
    fn assert_convex(points: &[(f32, f32)], what: &str) {
        assert!(
            points.len() >= 3,
            "{what}: fewer than three points enclose no area"
        );
        let (mut left, mut right) = (0, 0);
        for index in 0..points.len() {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            let c = points[(index + 2) % points.len()];
            let cross = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
            if cross > 0.0 {
                left += 1;
            } else if cross < 0.0 {
                right += 1;
            }
        }
        assert!(
            left == 0 || right == 0,
            "{what}: {left} turns one way and {right} the other, so the fan overlaps itself"
        );
    }

    /// Returns the distance from `point` to the infinite line through `a` and `b`.
    fn distance_to_line(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = magnitude((dx, dy));
        if length <= 0.0 {
            return magnitude((point.0 - a.0, point.1 - a.1));
        }
        ((point.1 - a.1) * dx - (point.0 - a.0) * dy).abs() / length
    }

    /// Returns how wide a stroke is across, measured from the second of a quad's
    /// two long edges to the line through the first.
    ///
    /// A mitred segment quad is `[a + A, b + B, b − B, a − A]`, so corners 0 and 1
    /// are on the `+` edge and corner 2 is on the `−` edge, and the perpendicular
    /// distance between them is **twice the stroke's half width** however long the
    /// mitres are. That is the number the on-screen measurement of a join is
    /// compared against, and it is the number this returns.
    fn across(quad: &[(f32, f32)]) -> f32 {
        distance_to_line(quad[2], quad[0], quad[1])
    }

    /// Returns a polygon's x extent as `(left, right)`.
    ///
    /// The smallest and largest x of its points, which for a fill quad is the
    /// pair of samples it was cut from.
    fn x_extent(points: &[(f32, f32)]) -> (f32, f32) {
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for point in points {
            low = low.min(point.0);
            high = high.max(point.0);
        }
        (low, high)
    }

    /// Returns a polygon's y extent as `(top, bottom)`.
    fn y_extent(points: &[(f32, f32)]) -> (f32, f32) {
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for point in points {
            low = low.min(point.1);
            high = high.max(point.1);
        }
        (low, high)
    }

    /// Returns the absolute area of a quadrilateral by the shoelace formula.
    fn quad_area(points: &[(f32, f32)]) -> f32 {
        let mut total = 0.0;
        for index in 0..points.len() {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            total += a.0 * b.1 - b.0 * a.1;
        }
        (total / 2.0).abs()
    }

    /// Returns the midpoint of two points.
    ///
    /// A segment quad is `[a + A, b + B, b − B, a − A]`, so corners 0 and 3 are
    /// one sample's two offsets and corners 1 and 2 are the other's — and their
    /// midpoints are therefore **the samples themselves** whenever the cap is flat,
    /// and somewhere past the vertex whenever it is mitred. That is the whole of
    /// what "a flat cap" means as a number.
    fn midpoint(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
        ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
    }

    /// Returns `1 + p·q` at the turning vertex of
    /// `a_full_reversal_depends_on_which_way_f32_rounds`'s oblique fixture, laid
    /// out at a plot height of `height` — the mitre denominator, computed from the
    /// **drawn** points and through the same two [`normal_of`] calls [`join_at`]
    /// makes.
    ///
    /// It is `None` where a normal is missing, which the oblique fixture never is:
    /// its whole point is that both normals exist and their dot is a rounding step
    /// away from `-1`.
    fn reversal_denominator(height: f32) -> Option<f32> {
        let rect = Rect::new(700.0, 40.0, 600.0, height);
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_palette(Palette::from_theme(&Theme::dark()));
        chart.set_fixed_range(0.0, 10.0);
        chart.set_grid_visible(false);
        chart.shown.set(Series {
            x: vec![0.0, 0.5, 1.0, 0.5, 1.0],
            values: vec![1.0, 5.0, 9.0, 5.0, 1.0],
        });
        let points = chart.points(chart.plot_rect(rect), chart.y_range());
        let vertex = points[2];
        let before = normal_of((vertex.0 - points[1].0, vertex.1 - points[1].1))?;
        let after = normal_of((points[3].0 - vertex.0, points[3].1 - vertex.1))?;
        Some(1.0 + before.0 * after.0 + before.1 * after.1)
    }

    // ---- Construction, the type, and the plain fields ------------------------

    #[test]
    fn a_new_chart_is_in_the_arena_with_nothing_to_plot() {
        let mut nodes = Arena::new();
        let chart = Chart::new(&mut nodes, ChartType::Line);
        assert!(nodes.get(chart.handle()).is_some());
        assert!(chart.data.get().is_empty());
        assert!(
            chart.shown.get().is_empty(),
            "and it starts arrived, not empty"
        );
        assert!(chart.x_labels.get().is_empty());
        assert!(chart.y_labels.get().is_empty());
        assert_eq!(chart.chart_type(), ChartType::Line);
        assert_eq!(chart.fixed_range(), None, "it auto-scales to start with");
        assert!(chart.grid_visible());
        assert_eq!(chart.line_width(), LINE_WIDTH);
        assert_eq!(chart.bar_fraction(), BAR_WIDTH_FRACTION);
        assert_eq!(chart.size(), Size::new(DEFAULT_WIDTH, DEFAULT_HEIGHT));
    }

    #[test]
    fn the_type_is_only_writable_through_its_setter() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_chart_type(ChartType::Bar);
        assert_eq!(chart.chart_type(), ChartType::Bar);
        chart.set_chart_type(ChartType::Area);
        assert_eq!(chart.chart_type(), ChartType::Area);
    }

    #[test]
    fn switching_to_bars_puts_the_revealed_bar_where_bars_expect_it() {
        // The gauge's rule, for the same reason: the first frame with a needle on
        // it is that needle in the current palette rather than in whatever the
        // colour was before.
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.reveal.set(0.0);
        chart.set_chart_type(ChartType::Bar);
        assert_eq!(
            chart.reveal.get(),
            1.0,
            "so the first bar is at full height"
        );
    }

    #[test]
    fn switching_away_from_bars_leaves_the_reveal_alone() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Bar);
        chart.reveal.set(0.25);
        chart.set_chart_type(ChartType::Line);
        assert_eq!(
            chart.reveal.get(),
            0.25,
            "nothing outside a bar chart draws it, so it is left where it stood"
        );
    }

    #[test]
    fn turning_the_grid_on_puts_its_colour_where_lines_expect_it() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_palette(Palette::from_theme(&Theme::dark()));
        chart.set_grid_visible(false);
        chart.grid.set(Color::new(1, 2, 3, 255));
        chart.set_grid_visible(true);
        assert_eq!(
            chart.grid.get(),
            Palette::from_theme(&Theme::dark()).grid,
            "so the first grid is in the current palette"
        );
    }

    #[test]
    fn a_negative_width_is_no_width() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_line_width(-4.0);
        assert_eq!(
            chart.line_width(),
            0.0,
            "a negative stroke is a stroke of none"
        );
        chart.set_line_width(7.0);
        assert_eq!(chart.line_width(), 7.0);
    }

    #[test]
    fn a_bar_fraction_is_bounded_rather_than_refused() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Bar);
        chart.set_bar_fraction(-1.0);
        assert_eq!(chart.bar_fraction(), 0.0);
        chart.set_bar_fraction(4.0);
        assert_eq!(chart.bar_fraction(), 1.0, "and a bar fills its whole slot");
    }

    #[test]
    fn a_fixed_range_is_ordered_wherever_it_came_from() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_fixed_range(240.0, 0.0);
        assert_eq!(chart.fixed_range(), Some((0.0, 240.0)));
        chart.set_fixed_range(f32::NAN, 10.0);
        assert_eq!(
            chart.fixed_range(),
            Some((10.0, 10.0)),
            "and a lost bound gives the other one rather than a NaN range"
        );
    }

    #[test]
    fn clearing_the_fixed_range_goes_back_to_the_drawn_series() {
        let (_, chart) = line(&[3.0, 9.0], 0.0, 10.0);
        assert_eq!(chart.y_range(), Some((0.0, 10.0)));
    }

    #[test]
    fn clearing_the_fixed_range_really_goes_back_to_the_drawn_series() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_fixed_range(0.0, 1000.0);
        chart.data.set(vec![2.0, 8.0]);
        chart.snap_to_state();
        assert_eq!(chart.y_range(), Some((0.0, 1000.0)));
        chart.clear_fixed_range();
        assert_eq!(
            chart.y_range(),
            Some((2.0, 8.0)),
            "and the axis goes back to fitting the data"
        );
    }

    // ---- Colours ------------------------------------------------------------

    #[test]
    fn a_palette_is_the_theme_five_parts_told_apart() {
        let theme = Theme::dark();
        let palette = Palette::from_theme(&theme);
        let color = |token| theme.get(token).as_color().unwrap();
        assert_eq!(palette.series, color(ThemeToken::Primary));
        assert_eq!(palette.fill, color(ThemeToken::Primary));
        assert_eq!(palette.axis, color(ThemeToken::TextMuted));
        assert_eq!(palette.label, color(ThemeToken::TextMuted));
        assert_eq!(palette.grid, color(ThemeToken::Border));
    }

    #[test]
    fn the_themes_give_an_opaque_fill_and_it_is_their_own_primary() {
        // **Not** because the per-segment quads need it — they are a tiling and
        // do not seam, which the module document records as a measurement rather
        // than as the argument this test used to make — but because that is what
        // both themes' `Primary` is, and a caller lowering the alpha is making a
        // decision about what is behind the chart.
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            assert_eq!(palette.fill.a, 255, "the fill is opaque in every theme");
            assert_eq!(palette.series.a, 255, "and so is the stroke");
            assert_eq!(
                palette.fill,
                theme.get(ThemeToken::Primary).as_color().unwrap()
            );
        }
        assert_eq!(Palette::default().fill.a, 255);
    }

    /// The property behind "a translucent fill does not seam", asserted on the
    /// recorded commands: **the fill's quads are a tiling, not an overlay.**
    ///
    /// Consecutive quads share exactly one x — the sample's — and their interiors
    /// do not overlap, so no pixel is covered twice and a colour with alpha below
    /// 255 composites once per pixel however many quads there are. This is the
    /// whole of the module's claim, and it is a property of the geometry rather
    /// than of a colour, so it is testable without a GL context: a mutation that
    /// widened a quad by a fraction of a pixel, or that offset a segment's `x1`,
    /// is caught here.
    ///
    /// Run over a **flat** series, so the quads are all the same height and a gap
    /// or an overlap is a difference in x alone; and over a long one, so it is not
    /// an accident of five samples.
    #[test]
    fn the_area_fill_is_a_tiling_and_not_an_overlay() {
        for count in [2_usize, 3, 5, 17, 64, 300] {
            let (_, chart) = area(&vec![40.0; count], 0.0, 100.0);
            let plot = chart.plot_rect(WIDE);
            let quads = filled_with(&chart.paint(WIDE), chart.fill.get());
            assert_eq!(
                quads.len(),
                count - 1,
                "{count} samples are {count} - 1 segments"
            );
            assert_close(
                x_extent(&quads[0]).0,
                plot.x,
                &format!("{count}: the first quad starts at the plot's left edge"),
            );
            assert_close(
                x_extent(&quads[quads.len() - 1]).1,
                plot.x + plot.width,
                &format!("{count}: and the last ends at its right edge"),
            );
            for (index, pair) in quads.windows(2).enumerate() {
                let (left, right) = x_extent(&pair[0]);
                let (next_left, next_right) = x_extent(&pair[1]);
                assert_close(
                    right,
                    next_left,
                    &format!("{count}: quads {index} and {} abut at one x", index + 1),
                );
                assert!(
                    right > left,
                    "{count}: quad {index} has width, so it is not degenerate"
                );
                assert!(
                    next_right >= next_left,
                    "{count}: and so is the one after it"
                );
            }
        }
    }

    #[test]
    fn no_two_fill_quads_of_a_rising_series_overlap_either() {
        // The same property with a series that is **not** flat, so the quads are
        // trapezoids of four different heights and a shared edge is the only
        // thing two of them have in common.
        let data: Vec<f32> = (0..40)
            .map(|index| {
                if index % 3 == 0 {
                    90.0
                } else {
                    10.0 + (index % 7) as f32 * 10.0
                }
            })
            .collect();
        let (_, chart) = area(&data, 0.0, 100.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let quads = filled_with(&chart.paint(WIDE), chart.fill.get());
        assert_eq!(quads.len(), points.len() - 1);
        for (index, quad) in quads.iter().enumerate() {
            let (left, right) = x_extent(quad);
            assert_close(
                left,
                points[index].0,
                &format!("quad {index} starts at its sample"),
            );
            assert_close(
                right,
                points[index + 1].0,
                &format!("quad {index} ends at the next one"),
            );
            // The two points on the baseline are the quad's own, so the fill cannot
            // reach past the plot's bottom edge either.
            assert_close(
                quad[2].1,
                plot.y + plot.height,
                &format!("quad {index} on the bottom"),
            );
            assert_close(
                quad[3].1,
                plot.y + plot.height,
                &format!("quad {index} on the bottom"),
            );
        }
    }

    #[test]
    fn a_translucent_fill_is_recorded_exactly_as_the_caller_set_it() {
        // The widget does not clamp, premultiply or otherwise second-guess the
        // alpha on a fill. The per-segment quads it draws from are a tiling, so a
        // translucent fill does not seam — measured on screen, in the module
        // document — and the widget's job here is only to pass the colour through.
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Area);
        let mut palette = Palette::from_theme(&Theme::dark());
        palette.fill = Color::new(187, 134, 252, 140);
        chart.set_palette(palette);
        chart.set_fixed_range(0.0, 100.0);
        chart.data.set(vec![40.0, 40.0, 40.0, 40.0, 40.0]);
        chart.snap_to_state();

        let fill = chart.fill.get();
        assert_eq!(fill.a, 140, "the alpha the caller chose is the alpha drawn");
        let commands = chart.paint(WIDE);
        assert_eq!(
            filled_with(&commands, fill).len(),
            4,
            "one quad per segment"
        );
        for quad in filled_with(&commands, fill) {
            assert_eq!(quad.len(), 4, "each four points, so the fan stays exact");
            assert_close(
                quad[2].1,
                WIDE.y + WIDE.height,
                "and each on the plot's bottom",
            );
        }
    }

    #[test]
    fn a_theme_switch_animates_the_chart_through_its_properties() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_palette(Palette::from_theme(&Theme::dark()));
        chart.snap_to_state();
        let dark = Palette::from_theme(&Theme::dark());

        chart.set_palette(Palette::from_theme(&Theme::light()));
        chart.animate_to_state(Motion {
            duration: ms(100),
            easing: Easing::Linear,
        });
        assert_eq!(chart.series.get(), dark.series, "still the dark theme's");
        tick(&chart, 50);
        assert_ne!(chart.series.get(), dark.series, "and half way between");
        tick(&chart, 50);
        assert_eq!(
            chart.series.get(),
            Palette::from_theme(&Theme::light()).series,
            "landing on the light theme's"
        );
    }

    /// Every colour a chart draws, on every chart type.
    ///
    /// The five properties are `label`, `axis`, `grid`, `series` and, for an
    /// area chart, `fill`. The first four are drawn for **every** type —
    /// [`Chart::draw_labels`] reads `label` whatever the type is, and
    /// [`Chart::snap_to_state`] sets it whatever the type is — so a type that
    /// stopped animating one of them is a chart left on the old theme's colour
    /// with nothing saying so.
    #[test]
    fn every_colour_a_chart_draws_reaches_every_chart_type() {
        let dark = Palette::from_theme(&Theme::dark());
        let light = Palette::from_theme(&Theme::light());
        let mut checked = 0;
        for chart_type in [ChartType::Line, ChartType::Bar, ChartType::Area] {
            let mut nodes = Arena::new();
            let mut chart = Chart::new(&mut nodes, chart_type);
            chart.set_palette(dark);
            chart.data.set(vec![1.0, 5.0, 2.0]);
            chart.snap_to_state();
            assert_eq!(
                chart.label.get(),
                dark.label,
                "{chart_type:?} starts on the dark label"
            );
            assert_eq!(chart.axis.get(), dark.axis, "{chart_type:?} starts dark");
            assert_eq!(chart.grid.get(), dark.grid, "{chart_type:?} starts dark");
            assert_eq!(
                chart.series.get(),
                dark.series,
                "{chart_type:?} starts dark"
            );

            chart.set_palette(light);
            chart.animate_to_state(Motion {
                duration: ms(100),
                easing: Easing::Linear,
            });
            tick(&chart, 100);
            let what = format!("{chart_type:?}");
            assert_eq!(
                chart.label.get(),
                light.label,
                "{what} lands on the light label"
            );
            assert_eq!(
                chart.axis.get(),
                light.axis,
                "{what} lands on the light axis"
            );
            assert_eq!(
                chart.grid.get(),
                light.grid,
                "{what} lands on the light grid"
            );
            assert_eq!(
                chart.series.get(),
                light.series,
                "{what} lands on the light series"
            );
            if chart_type == ChartType::Area {
                assert_eq!(
                    chart.fill.get(),
                    light.fill,
                    "{what} lands on the light fill"
                );
            }
            checked += 1;
        }
        assert_eq!(checked, 3, "all three types, and no more");
    }

    /// The same question asked of the **command stream** rather than of the
    /// properties: a **bar** chart whose labels are still on the dark theme's
    /// `TextMuted` while every other colour has moved.
    ///
    /// `Chart::label` is `pub`, so this is the *second* question and not the only
    /// one — it was previously described here as the only possible one, which was
    /// false, and the property test 60 lines above is what reads it directly. Both
    /// are worth asking and they fail differently: the property says what the chart
    /// **holds**, this says what it **draws**, and a chart could satisfy the first
    /// while the painter used a different colour for the one case the reviewer found.
    /// Dark `TextMuted` is `(158,158,158)` and light is `(117,117,117)`, and a label
    /// drawn in the first is a grey axis label on a light window.
    #[test]
    fn a_bars_labels_arrive_at_the_new_theme_as_the_text_they_draw() {
        let dark = Palette::from_theme(&Theme::dark());
        let light = Palette::from_theme(&Theme::light());
        assert_ne!(
            dark.label, light.label,
            "and the two themes' labels really differ, or this proves nothing"
        );
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Bar);
        chart.set_palette(dark);
        chart.data.set(vec![1.0, 5.0, 2.0]);
        chart.snap_to_state();
        chart.x_labels.set(vec!["a".into(), "b".into(), "c".into()]);
        chart.y_labels.set(vec!["0".into(), "5".into()]);

        chart.set_palette(light);
        chart.animate_to_state(Motion {
            duration: ms(100),
            easing: Easing::Linear,
        });
        tick(&chart, 100);

        let rect = Rect::new(0.0, 0.0, 400.0, 240.0);
        let commands = chart.paint(rect);
        let drawn: Vec<&DrawCommand> = commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Text { .. }))
            .collect();
        assert!(
            drawn.len() >= 5,
            "three x labels and two y labels are on screen, so the fixture can fail: {}",
            drawn.len()
        );
        for command in drawn {
            let DrawCommand::Text { color, text, .. } = command else {
                unreachable!("filtered to Text just above")
            };
            assert_ne!(
                *color, dark.label,
                "'{text}' is still on the dark theme's colour, which is the bug"
            );
            assert_eq!(
                *color, light.label,
                "and every label is on the light theme's"
            );
        }
    }

    /// Three different `shown` series a degenerate-case table has to tell apart,
    /// measured through the public API and not through a helper.
    ///
    /// The table once had one row for all of them and said they drew nothing. Two
    /// of the three do not:
    ///
    /// - **same `x`, different readings** — a vertical segment, `line_width`
    ///   wide, spanning the two readings. Measured at `Rect(700, 40, 600, 300)`
    ///   over `0.0 ..= 10.0` as 3 px across, 120 px down and centred on x = 1000.
    /// - **same `x`, same reading** — the same point twice, so no direction and
    ///   no segment.
    /// - **a run that doubles back** — a 180 degree turn, where the mitre
    ///   denominator is zero. No segment on *either* side of the turn.
    #[test]
    fn a_repeated_x_draws_a_vertical_rule_and_a_repeated_point_draws_nothing() {
        let rect = Rect::new(700.0, 40.0, 600.0, 300.0);
        let shown = |x: Vec<f32>, values: Vec<f32>| {
            let mut nodes = Arena::new();
            let mut chart = Chart::new(&mut nodes, ChartType::Line);
            chart.set_palette(Palette::from_theme(&Theme::dark()));
            chart.set_fixed_range(0.0, 10.0);
            chart.set_grid_visible(false);
            chart.shown.set(Series { x, values });
            chart
        };

        // Same x, different readings: a real vertical rule.
        let vertical = shown(vec![0.5, 0.5], vec![1.0, 5.0]);
        let quads = polygons(&vertical.paint(rect));
        assert_eq!(quads.len(), 1, "one segment, not none");
        let (points, _) = &quads[0];
        let (left, right) = x_extent(points);
        let (top, bottom) = y_extent(points);
        assert_close(right - left, LINE_WIDTH, "it is as wide as the line itself");
        assert_close(
            bottom - top,
            120.0,
            "and it spans the two readings, 4 apart over a 300-pixel plot",
        );
        assert_close(
            (left + right) / 2.0,
            1000.0,
            "and it stands where the shared x put it, mid-plot",
        );

        // Same x, same reading: the same point twice, so no direction at all.
        let repeated = shown(vec![0.5, 0.5], vec![1.0, 1.0]);
        assert_eq!(
            polygons(&repeated.paint(rect)).len(),
            0,
            "nothing: two identical points have no direction to offset"
        );

        // A full reversal: the mitre denominator is zero at the middle sample,
        // and that one undefined join takes the segments on both sides with it.
        let doubled_back = shown(vec![0.5, 0.5, 0.5], vec![1.0, 5.0, 1.0]);
        assert_eq!(
            polygons(&doubled_back.paint(rect)).len(),
            0,
            "a reversal is not a corner, so neither segment beside it is drawn"
        );

        // And a repeated *point* at the end of a run still loses the segment that
        // led to it, because both shared that one join. This is the consequence
        // the table now names.
        let tail = shown(vec![0.0, 0.5, 0.5], vec![1.0, 5.0, 5.0]);
        assert_eq!(
            polygons(&tail.paint(rect)).len(),
            0,
            "the 1 -> 5 segment dies with the join its end shared with the repeat"
        );

        // **The fifth case, and the one that is a rule rather than an accident:**
        // the same repeat at the length of a real series. Each repeated point
        // costs the segments on **both** sides of it, so two repeats in five
        // samples leave nothing at all — not "a whole run disappearing" as an
        // aside, and not a partial draw either.
        let two_repeats = shown(vec![0.0, 0.5, 0.5, 1.0, 1.0], vec![1.0, 5.0, 5.0, 1.0, 1.0]);
        assert_eq!(
            polygons(&two_repeats.paint(rect)).len(),
            0,
            "0 of 4 segments: the repeat at 0.5 kills segments 0 and 1, the one \
             at 1.0 kills segments 2 and 3, and the pairs do not overlap"
        );
        // One repeat inside a longer run costs **three** of the four, not two: a
        // repeat between vertices `i` and `i+1` leaves `joins[i]` undefined, because
        // its *leaving* direction is zero, and `joins[i + 1]` undefined, because
        // its *arriving* direction is zero — and one join is read by the segments on
        // both sides of it, so segments `i - 1`, `i` and `i + 1` each lose an end.
        // Drawing resumes at `i + 2`, which is what makes this a break rather than
        // a truncation. The two samples must share **both** `x` and the reading, or
        // they are two different points at the same height and the segment between
        // them draws as a vertical rule — the case two rows up.
        let one_repeat = shown(
            vec![0.0, 0.25, 0.25, 0.75, 1.0],
            vec![1.0, 5.0, 5.0, 7.0, 9.0],
        );
        assert_eq!(
            polygons(&one_repeat.paint(rect)).len(),
            1,
            "1 of 4: the repeat between vertices 1 and 2 kills segments 0, 1 and 2, \
             and segment 3 is drawn, so the run resumes at the next vertex"
        );

        // The supported path never produces any of this, which is why it is a
        // documented edge rather than a bug: `x` comes from the sample index.
        let (_, derived) = showing(ChartType::Line, &[1.0, 5.0, 1.0], 0.0, 10.0);
        let xs = derived.style().shown.x;
        assert!(
            xs.windows(2).all(|pair| pair[1] > pair[0]),
            "and a derived series is strictly increasing: {xs:?}"
        );
    }

    /// A full reversal, and the finding that **which** of two outcomes happens is
    /// decided by `f32` rounding rather than by the turn.
    ///
    /// `join_at` returns `None` for a turn when `1 + p·q` is not positive. At a
    /// full reversal `p` and `q` are opposite, so `p·q` is `-1` exactly and
    /// `1 + p·q` is `0` — but in `f32` the two normals are products of divisions,
    /// and their dot product lands on either side of `-1` according to the
    /// coordinates. **Changing only the plot's height changes which branch runs**,
    /// which is the whole of the finding:
    ///
    /// | the fixture | plot height | `1 + p·q` | segments drawn of 4 |
    /// |---|---:|---|---:|
    /// | `x = [0.5, 0.5, 0.5]`, `values = [1, 5, 1]` — axis-aligned, so the normals are exactly `±(1, 0)` | any | exactly `0.0` | **0** |
    /// | `x = [0, 0.5, 1, 0.5, 1]`, `values = [1, 5, 9, 5, 1]` — oblique | 300 | exactly `0.0` | **2** |
    /// | the same | 240 | **`5.9604645e-8`**, which is exactly `2^-24` | **4, and one of them degenerate** |
    ///
    /// **The third row's denominator is one `f32` step, and the first version of
    /// this table published `1.2e-3` for it** — four orders of magnitude too big,
    /// and the one figure in it a reader would sit down and reproduce. Measured at
    /// this fixture, with `1 + p·q` computed the way `join_at` computes it:
    ///
    /// | plot height | `1 + p·q` | join | drawn of 4 |
    /// |---:|---:|---|---:|
    /// | 200 | `+5.9604645e-8` | `Some(Corner((0, 0)))` | 4 |
    /// | 240 | `+5.9604645e-8` | `Some(Corner((0, 0)))` | 4 |
    /// | 250 | `-5.9604645e-8` | `None` | 2 |
    /// | 260 | `0.0` | `None` | 2 |
    /// | 280 | `-5.9604645e-8` | `None` | 2 |
    /// | 300 | `0.0` | `None` | 2 |
    /// | 320 | `-1.1920929e-7` | `None` | 2 |
    /// | 360 | `0.0` | `None` | 2 |
    /// | 400 | `-5.9604645e-8` | `None` | 2 |
    ///
    /// **So the denominator is never anything but a rounding step** — `0`, `±2^-24`
    /// or `±2^-23` across the whole sweep, which is what a sum of two products of
    /// one rounding each can be. The row's *conclusion* is unaffected and is what
    /// the test below asserts; only the published magnitude was wrong.
    /// `a_full_reversal_depends_on_which_way_f32_rounds` **asserts every cell of
    /// that sweep**, exactly rather than to a tolerance, so this table is a
    /// quotation of the suite rather than of the author.
    ///
    /// **The third row is a defect, not a degenerate case**, and it is the
    /// integrator's to schedule rather than this round's to change. With the
    /// denominator small and positive, `scale = half / (1 + p·q)` is enormous, and
    /// the corner is that scale times `p + q` — which cancels to **exactly zero**,
    /// because the two normals are computed as exact negatives of each other. So the
    /// corner is `(0, 0)`, its length is `0`, and **`0 <= MITRE_LIMIT · half`
    /// accepts it as a *shorter* join**: `join_at` returns `Join::Corner((0, 0))` and
    /// the segment quad inherits a zero offset, so two of its four corners are the
    /// same point. **The mitre-limit test cannot tell a short corner from no
    /// corner at all**, and that is the bug in one sentence.
    ///
    /// This test pins it **on purpose**. It is a tripwire, not an endorsement: a fix
    /// that makes the third row return 2, or makes its quad honest, fails this test
    /// and the table above says what the right answer is. All three rows are
    /// asserted so that none can be changed silently.
    #[test]
    fn a_full_reversal_depends_on_which_way_f32_rounds() {
        let oblique = |height: f32| {
            let mut nodes = Arena::new();
            let mut chart = Chart::new(&mut nodes, ChartType::Line);
            chart.set_palette(Palette::from_theme(&Theme::dark()));
            chart.set_fixed_range(0.0, 10.0);
            chart.set_grid_visible(false);
            chart.shown.set(Series {
                x: vec![0.0, 0.5, 1.0, 0.5, 1.0],
                values: vec![1.0, 5.0, 9.0, 5.0, 1.0],
            });
            let rect = Rect::new(700.0, 40.0, 600.0, height);
            polygons(&chart.paint(rect))
        };

        // Row 1: axis-aligned, so the join is `None` at any height. This is the
        // one branch of the two that is a rule rather than an accident.
        for height in [240.0_f32, 300.0] {
            let mut nodes = Arena::new();
            let mut chart = Chart::new(&mut nodes, ChartType::Line);
            chart.set_palette(Palette::from_theme(&Theme::dark()));
            chart.set_fixed_range(0.0, 10.0);
            chart.set_grid_visible(false);
            chart.shown.set(Series {
                x: vec![0.5, 0.5, 0.5],
                values: vec![1.0, 5.0, 1.0],
            });
            assert_eq!(
                polygons(&chart.paint(Rect::new(700.0, 40.0, 600.0, height))).len(),
                0,
                "an axis-aligned reversal has an exactly-zero denominator at every \
                 height, so it is dropped: {height}"
            );
        }

        // Row 2: oblique at 300, the same coordinates one rounding step apart.
        let dropped = oblique(300.0);
        assert_eq!(
            dropped.len(),
            2,
            "at 300 the dot lands exactly on -1 and the two segments at the turn \
             are dropped"
        );
        assert!(
            dropped.iter().all(|(points, _)| {
                points.len() == 4 && points.windows(2).all(|pair| pair[0] != pair[1])
            }),
            "and what survives is four real corners each: {dropped:?}"
        );

        // Row 3: oblique at 240, and the defect.
        let kept = oblique(240.0);
        assert_eq!(
            kept.len(),
            4,
            "at 240 the denominator is small and positive, so nothing is dropped: \
             {kept:?}"
        );
        let degenerate: Vec<&Vec<(f32, f32)>> = kept
            .iter()
            .map(|(points, _)| points)
            .filter(|points| points.windows(2).any(|pair| pair[0] == pair[1]))
            .collect();
        assert_eq!(
            degenerate.len(),
            1,
            "exactly one quad inherits the zero offset, and it is the one at the \
             turn: {degenerate:?}"
        );
        assert_eq!(
            kept.len() - degenerate.len(),
            3,
            "and the other three are untouched: the defect is local to the turn"
        );

        // **The magnitude the table above publishes, asserted.** It is one `f32`
        // rounding step and nothing else, so it is compared exactly and not to a
        // tolerance — every value in the sweep is `0` or `±2^-24` or `±2^-23`.
        for (height, want) in [
            (200.0_f32, 5.9604645e-8_f32),
            (240.0, 5.9604645e-8),
            (250.0, -5.9604645e-8),
            (260.0, 0.0),
            (280.0, -5.9604645e-8),
            (300.0, 0.0),
            (320.0, -1.1920929e-7),
            (360.0, 0.0),
            (400.0, -5.9604645e-8),
        ] {
            assert_eq!(
                reversal_denominator(height),
                Some(want),
                "1 + p·q at plot height {height} is exactly as the table publishes it"
            );
        }
        // And that it really is a power of two, which is the reason the sweep has
        // three values and no fourth: `2^-24` is one step at 1.0, and the sum of two
        // products of one division each can be no coarser.
        assert_eq!(5.9604645e-8_f32, 1.0_f32 / 16_777_216.0, "which is 2^-24");
        assert_eq!(-1.1920929e-7_f32, -(2.0_f32 / 16_777_216.0), "and 2^-23");
    }

    #[test]
    fn a_snap_puts_a_themed_chart_on_its_theme_at_once() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Area);
        let themed = Palette::from_theme(&Theme::light());
        chart.set_palette(themed);
        chart.data.set(vec![1.0, 2.0]);
        chart.snap_to_state();
        assert_eq!(chart.series.get(), themed.series);
        assert_eq!(chart.fill.get(), themed.fill);
        assert_eq!(chart.axis.get(), themed.axis);
        assert_eq!(chart.grid.get(), themed.grid);
        assert_eq!(chart.label.get(), themed.label);
        assert_eq!(chart.shown.get().values, vec![1.0, 2.0]);
    }

    #[test]
    fn a_style_reports_the_series_the_truth_maps_to_and_five_colours() {
        let (_, chart) = showing(ChartType::Line, &[2.0, 4.0, 6.0], 0.0, 10.0);
        let style = chart.style();
        assert_eq!(style.shown.values, vec![2.0, 4.0, 6.0]);
        assert_eq!(style.shown.x, vec![0.0, 0.5, 1.0]);
        assert_eq!(style.series, chart.series.get());
        assert_eq!(style.fill, chart.fill.get());
        assert_eq!(style.axis, chart.axis.get());
        assert_eq!(style.grid, chart.grid.get());
        assert_eq!(style.label, chart.label.get());
    }

    // ---- The scale ----------------------------------------------------------

    #[test]
    fn an_auto_scale_is_the_extent_of_the_drawn_series() {
        let (_, mut chart) = line(&[-4.0, 0.0, 8.0], 0.0, 10.0);
        chart.clear_fixed_range();
        chart.snap_to_state();
        assert_eq!(chart.y_range(), Some((-4.0, 8.0)));
    }

    #[test]
    fn an_auto_scale_ignores_the_readings_that_are_not_there() {
        let (_, mut chart) = line(&[3.0, f32::NAN, 9.0, f32::INFINITY], 0.0, 10.0);
        chart.clear_fixed_range();
        chart.snap_to_state();
        assert_eq!(
            chart.y_range(),
            Some((3.0, 9.0)),
            "an infinite reading is not an upper bound either"
        );
    }

    #[test]
    fn an_all_gap_series_has_no_scale_at_all() {
        let (_, mut chart) = line(&[f32::NAN, f32::NAN], 0.0, 10.0);
        chart.clear_fixed_range();
        chart.snap_to_state();
        assert_eq!(chart.y_range(), None, "rather than one full of NaN");
    }

    #[test]
    fn the_scale_follows_the_drawn_series_and_not_the_truth() {
        // A scale read from `data` moves on the frame the caller wrote it, so
        // every reading on the chart shifts at once.
        let (_, mut chart) = line(&[0.0, 1.0], 0.0, 10.0);
        chart.clear_fixed_range();
        chart.snap_to_state();
        assert_eq!(chart.y_range(), Some((0.0, 1.0)));

        chart.data.set(vec![0.0, 1.0, 100.0]);
        chart.animate_to_state(motion());
        assert_eq!(
            chart.y_range(),
            Some((0.0, 1.0)),
            "aiming does not move the scale"
        );
        tick(&chart, 50);
        let halfway = chart.y_range().expect("a scale");
        assert!(
            halfway.1 > 1.0 && halfway.1 < 100.0,
            "the scale grows with the series: {halfway:?}"
        );
        tick(&chart, 50);
        assert_eq!(
            chart.y_range(),
            Some((0.0, 100.0)),
            "and arrives at the truth"
        );
    }

    // ---- The plot and its gutters -------------------------------------------

    #[test]
    fn the_plot_is_the_whole_rect_when_nothing_is_labelled() {
        let (_, chart) = line(&[1.0, 2.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        assert_eq!(plot.x, WIDE.x);
        assert_eq!(plot.y, WIDE.y);
        assert_eq!(plot.width, WIDE.width);
        assert_eq!(plot.height, WIDE.height);
    }

    #[test]
    fn a_gutter_comes_off_the_extent_and_not_off_the_origin() {
        // The slider suite's lesson: a rect's origin and a rect's extent are two
        // different numbers. `plot.x` is the rect's **x plus** the gutter and
        // `plot.width` is the rect's **width less** it.
        let (_, chart) = line(&[1.0, 2.0], 0.0, 10.0);
        chart.y_labels.set(vec!["0".to_string()]);
        let plot = chart.plot_rect(WIDE);
        assert_close(
            plot.x,
            WIDE.x + Y_LABEL_GUTTER,
            "the plot starts past the gutter",
        );
        assert_close(
            plot.width,
            WIDE.width - Y_LABEL_GUTTER,
            "and is narrower by it",
        );
        assert_close(plot.y, WIDE.y, "the y origin is untouched");
        assert_close(plot.height, WIDE.height, "and so is the height");
    }

    #[test]
    fn a_bottom_gutter_takes_the_labels_off_the_bottom_and_nothing_else() {
        let (_, chart) = line(&[1.0, 2.0], 0.0, 10.0);
        chart.x_labels.set(vec!["a".to_string(), "b".to_string()]);
        let plot = chart.plot_rect(WIDE);
        assert_eq!(plot.x, WIDE.x, "the left is not moved by an x label");
        assert_eq!(plot.width, WIDE.width);
        assert_close(
            plot.height,
            WIDE.height - X_LABEL_GUTTER,
            "the height loses it",
        );
        assert_eq!(plot.y, WIDE.y);
    }

    #[test]
    fn a_rect_narrower_than_its_gutter_leaves_a_plot_of_no_width() {
        let (_, chart) = line(&[1.0, 2.0], 0.0, 10.0);
        chart.y_labels.set(vec!["0".to_string()]);
        let narrow = Rect::new(10.0, 10.0, 20.0, 100.0);
        let plot = chart.plot_rect(narrow);
        assert_eq!(plot.width, 0.0, "and not a negative width");
        let commands = chart.paint(narrow);
        assert!(
            polygons(&commands).is_empty()
                && lines(&commands).is_empty()
                && rects(&commands).is_empty(),
            "a chart with no plot draws no geometry at all"
        );
        assert_eq!(
            texts(&commands).len(),
            1,
            "and only its labels, which have no plot to be inside"
        );
    }

    // ---- Where a reading lands ----------------------------------------------

    #[test]
    fn the_first_sample_is_at_the_plots_left_edge_and_the_last_at_its_right() {
        let (_, chart) = line(&[0.0, 5.0, 10.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        assert_eq!(points.len(), 3);
        assert_close(points[0].0, plot.x, "the first sample is on the left edge");
        assert_close(
            points[1].0,
            plot.x + plot.width / 2.0,
            "the middle is in the middle",
        );
        assert_close(points[2].0, plot.x + plot.width, "the last is on the right");
    }

    #[test]
    fn the_topmost_point_is_where_its_value_says_it_is() {
        // Not "the line got there": the y of a known reading against the plot's
        // own top, which is the number a reader off the screen would check.
        let (_, chart) = line(&[0.0, 7.5, 3.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let highest = points[1].1;
        assert!(
            highest < points[0].1 && highest < points[2].1,
            "the 7.5 is the topmost, because the y axis points down: {points:?}"
        );
        assert_close(
            highest,
            plot.y + (10.0 - 7.5) / 10.0 * plot.height,
            "a quarter of the way down from the top",
        );
        assert_close(points[0].1, plot.y + plot.height, "0.0 is on the bottom");
        assert_close(
            points[2].1,
            plot.y + 0.7 * plot.height,
            "and the 3.0 is seven tenths of the way down from the top",
        );
    }

    #[test]
    fn a_reading_outside_a_fixed_range_is_pinned_to_the_plots_edge() {
        let (_, chart) = line(&[-100.0, 5.0, 100.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        assert_close(
            points[0].1,
            plot.y + plot.height,
            "below the range is the bottom",
        );
        assert_close(points[2].1, plot.y, "above the range is the top");
    }

    #[test]
    fn a_range_of_one_value_draws_the_series_through_the_middle_of_the_plot() {
        // The divide-by-zero case: `high - low` is zero, and both edges of a plot
        // are edges rather than positions.
        let (_, mut chart) = line(&[5.0, 5.0, 5.0], 0.0, 10.0);
        chart.clear_fixed_range();
        chart.snap_to_state();
        assert_eq!(
            chart.y_range(),
            Some((5.0, 5.0)),
            "the range has no room in it"
        );
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        for (index, point) in points.iter().enumerate() {
            assert_close(
                point.1,
                plot.y + plot.height / 2.0,
                &format!("sample {index}"),
            );
        }
        assert!(points.iter().all(|point| point.1.is_finite()));
    }

    #[test]
    fn a_nan_reading_breaks_the_run_rather_than_dividing_by_it() {
        let (_, chart) = line(&[0.0, f32::NAN, 10.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        assert!(!points[1].1.is_finite(), "the gap is marked, not placed");
        assert!(points[0].1.is_finite() && points[2].1.is_finite());

        let mut runs = Vec::new();
        each_run(&points, |start, end| runs.push((start, end)));
        assert_eq!(
            runs,
            Vec::<(usize, usize)>::new(),
            "a run of one sample is a run with no segments in it, either side of the gap"
        );
    }

    /// A gap ends a run, and **the two real segments either side of it are still
    /// drawn** — which is what four documents in this module already promised and
    /// the first draft did not deliver.
    ///
    /// [`Chart::draw_series`] built its joins for *every* point, the `NaN` one
    /// included, so the vertex before a gap asked [`join_at`] about a direction of
    /// `NaN`; [`normal_of`] answers `None` for one, and **a `None` join is read by
    /// the segments on both sides of it** — so both real segments adjacent to every
    /// gap went with it. Measured through `paint` before the fix, against what each
    /// of those runs holds:
    ///
    /// | the data | before | after | the run's own segments |
    /// |---|---:|---:|---:|
    /// | `[1, 5, 9, NaN, 3, 7]` | 1 | 3 | 3 |
    /// | `[1, 5, NaN, 3]` | 0 | 1 | 1 |
    /// | `[NaN, 5, 9]` | 0 | 1 | 1 |
    /// | `[1, 5, ∞, 3, 7]` | 0 | 2 | 2 |
    /// | `[1, 5, 9, 3, 7]`, the control | 4 | 4 | 4 |
    ///
    /// **The fix is in the behaviour and not in those four documents**, which were
    /// right: a gap is now an **absent** neighbour rather than a neighbour of no
    /// direction, so the vertex either side of one asks the question the end of a
    /// run asks and [`join_at`]'s own answer to that is a flat cap of exactly
    /// `half` — a mitre of `half/cos(0)`, since both directions are the same one.
    ///
    /// Every assertion here goes through `paint`, because the claim is about what
    /// is recorded. **The caps are asserted as the samples themselves** — a run-end
    /// segment quad's two end midpoints are its own two samples, which is false of
    /// a mitred corner and is the whole of what "flat cap" means as a number.
    #[test]
    fn a_gap_ends_a_run_and_the_segments_beside_it_are_still_drawn() {
        let gapped = [1.0, 5.0, 9.0, f32::NAN, 3.0, 7.0];

        // The middle row of the table, with its control one line below it.
        let (_, chart) = line(&gapped, 0.0, 10.0);
        let commands = chart.paint(WIDE);
        let quads = polygons(&commands);
        assert_eq!(
            quads.len(),
            3,
            "1 -> 5 -> 9 is two segments and 3 -> 7 is one, and a gap in the middle \
             costs the segment that spanned it and nothing else"
        );
        let (_, whole) = line(&[1.0, 5.0, 9.0, 3.0, 7.0], 0.0, 10.0);
        assert_eq!(
            polygons(&whole.paint(WIDE)).len(),
            4,
            "and the same five samples with the gap filled in have four segments"
        );

        // Every quad is the requested width, is convex, and is capped **at its own
        // two samples** — including the two that touch the gap, which are the end of
        // segment 1 at `points[2]` and the start of segment 4 at `points[4]`.
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let mut segments = Vec::new();
        each_run(&points, |start, end| segments.extend(start..end));
        assert_eq!(
            segments,
            vec![0, 1, 4],
            "and the three drawn are the runs' own segments in order: only segment 2, \
             which ran from 9 into the gap, and segment 3, which ran out of it, are \
             not there"
        );
        for (drawn, index) in segments.iter().enumerate() {
            let (index, quad) = (*index, &quads[drawn].0);
            assert_eq!(quad.len(), 4, "segment {index} is four points");
            assert_convex(quad, &format!("segment {index} across the gap"));
            assert_close(across(quad), LINE_WIDTH, "and is the requested width");
            let (start, end) = (midpoint(quad[0], quad[3]), midpoint(quad[1], quad[2]));
            assert_close(
                start.0,
                points[index].0,
                "segment {index} is capped at its own sample",
            );
            assert_close(
                start.1,
                points[index].1,
                "segment {index} is capped at its own sample",
            );
            assert_close(
                end.0,
                points[index + 1].0,
                "segment {index} is capped at its next sample",
            );
            assert_close(
                end.1,
                points[index + 1].1,
                "segment {index} is capped at its next sample",
            );
        }
        assert!(
            circles(&commands).is_empty(),
            "and no disc is drawn for any of it: a run's end has a flat cap and a \
             flat cap needs no disc"
        );

        // The two mirror cases, each of which lost its **whole** run before.
        for (data, what) in [
            (&[f32::NAN, 5.0, 9.0][..], "a leading gap"),
            (&[1.0, 5.0, f32::NAN][..], "a trailing gap"),
        ] {
            let (_, edge) = line(data, 0.0, 10.0);
            assert_eq!(
                polygons(&edge.paint(WIDE)).len(),
                1,
                "{what}: its one real segment, and a cap at the sample beside the gap"
            );
        }

        // An infinity is the same gap — `points` marks it with the same marker — so
        // a reading that overflows a division cannot take its neighbours with it.
        for gap in [f32::INFINITY, f32::NEG_INFINITY] {
            let (_, overflow) = line(&[1.0, 5.0, gap, 3.0, 7.0], 0.0, 10.0);
            assert_eq!(
                polygons(&overflow.paint(WIDE)).len(),
                2,
                "two runs of two samples either side of {gap}, and both segments of each"
            );
        }

        // The area chart is where the defect was **visible**, because its fill walks
        // [`each_run`] and never consults a join: before the fix the same data
        // recorded three fill quads and **one** stroke quad, a body drawn across a
        // gap its own outline was missing. The outline now follows it.
        let (_, body) = area(&gapped, 0.0, 10.0);
        let body_commands = body.paint(WIDE);
        assert_eq!(
            filled_with(&body_commands, body.fill.get()).len(),
            3,
            "three fill quads, one per segment of each run"
        );
        assert_eq!(
            filled_with(&body_commands, body.series.get()).len(),
            3,
            "and three stroke quads over them, so the outline is not missing exactly \
             where the fill is not"
        );
        let (_, filled) = area(&[1.0, 5.0, 9.0, 3.0, 7.0], 0.0, 10.0);
        let filled_commands = filled.paint(WIDE);
        assert_eq!(
            (
                filled_with(&filled_commands, filled.fill.get()).len(),
                filled_with(&filled_commands, filled.series.get()).len()
            ),
            (4, 4),
            "the control is four and four"
        );
    }

    /// A repeated **point** and a non-finite sample are two different things, and
    /// this keeps them two different things.
    ///
    /// The gap ends a run and the segments beside it are drawn; a repeat — the same
    /// `x` **and** the same reading — leaves a direction of zero, which is a real
    /// neighbour of no length, and [`normal_of`] answers `None` for that as it does
    /// for a `NaN`. **This change did not touch that**, because it is a different
    /// case and `Chart::paint`'s table, the paragraph under it and
    /// `a_repeated_x_draws_a_vertical_rule_and_a_repeated_point_draws_nothing` all
    /// still say **three** segments are lost. Both fixtures are five samples with
    /// the same readings, and they differ in one number: `5.0` twice against a
    /// `NaN` in the middle.
    #[test]
    fn a_repeated_point_is_not_a_gap_and_still_costs_its_neighbours() {
        let hand = |x: Vec<f32>, values: Vec<f32>| {
            let mut nodes = Arena::new();
            let mut chart = Chart::new(&mut nodes, ChartType::Line);
            chart.set_palette(Palette::from_theme(&Theme::dark()));
            chart.set_fixed_range(0.0, 10.0);
            chart.set_grid_visible(false);
            chart.shown.set(Series { x, values });
            chart
        };

        let repeat = hand(
            vec![0.0, 0.25, 0.25, 0.75, 1.0],
            vec![1.0, 5.0, 5.0, 7.0, 9.0],
        );
        let repeated = repeat.paint(WIDE);
        assert_eq!(
            polygons(&repeated).len(),
            1,
            "1 of 4, unchanged: a repeat between vertices 1 and 2 leaves two joins \
             undefined and the segments on both sides of each lose an end"
        );
        assert!(
            circles(&repeated).is_empty(),
            "and no disc: there is no turn at a repeat, so there is nothing to round off"
        );

        let gap = hand(
            vec![0.0, 0.2, 0.4, 0.6, 1.0],
            vec![1.0, 5.0, f32::NAN, 7.0, 9.0],
        );
        assert_eq!(
            polygons(&gap.paint(WIDE)).len(),
            2,
            "5 samples with one of them a gap hold 2 segments — 1 -> 5 and 7 -> 9 — \
             and both are drawn: a gap loses nothing at all"
        );
    }

    #[test]
    fn a_series_whose_two_halves_differ_in_length_is_read_as_far_as_it_is_whole() {
        let mut nodes = Arena::new();
        let chart = Chart::new(&mut nodes, ChartType::Line);
        chart.shown.set(Series {
            x: vec![0.0, 0.5, 1.0],
            values: vec![1.0, 2.0],
        });
        assert_eq!(chart.shown.get().len(), 2);
        let plot = chart.plot_rect(WIDE);
        assert_eq!(
            chart.points(plot, chart.y_range()).len(),
            2,
            "and the third sample is not reached"
        );
    }

    #[test]
    fn a_sample_outside_the_plot_is_bounded_onto_it() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_fixed_range(0.0, 10.0);
        chart.shown.set(Series {
            x: vec![-3.0, 0.5, 7.0],
            values: vec![0.0, 5.0, 10.0],
        });
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        assert_close(points[0].0, plot.x, "a sample off the left is on the left");
        assert_close(points[2].0, plot.x + plot.width, "and off the right on it");
    }

    // ---- The line: thickness, mitres, and the limit --------------------------

    #[test]
    fn a_straight_segment_is_the_requested_thickness_across() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        let quad = filled_with(&chart.paint(WIDE), chart.series.get());
        assert_eq!(quad.len(), 1);
        assert_close(
            across(&quad[0]),
            LINE_WIDTH,
            "a 3-pixel line is 3 pixels across",
        );
    }

    #[test]
    fn every_segment_is_the_requested_thickness_across_at_both_of_its_ends() {
        // **The measurement the on-screen check is compared against.** A segment
        // quad is `[a + A, b + B, b − B, a − A]`, so the distance from corner 2 to
        // the line through corners 0 and 1 is twice the half width however long
        // the mitres are — which is the whole claim, checked on every segment of
        // a deliberately spiky series rather than on one hand-picked pair.
        let spiky = [
            0.0, 8.0, 1.0, 9.5, 2.0, 3.0, 6.0, 10.0, 7.0, 2.5, 8.0, 9.0, 9.0, 1.0, 10.0, 0.5,
        ];
        let (_, chart) = line(&spiky, 0.0, 10.0);
        let quads = filled_with(&chart.paint(WIDE), chart.series.get());
        assert_eq!(quads.len(), spiky.len() - 1, "one quad per segment");
        for (index, quad) in quads.iter().enumerate() {
            assert_eq!(quad.len(), 4, "segment {index} is four points");
            assert_close(across(quad), LINE_WIDTH, &format!("segment {index} across"));
        }
    }

    #[test]
    fn the_end_of_a_straight_run_is_a_flat_cap_of_the_requested_width() {
        let (_, chart) = line(&[0.0, 10.0, 5.0], 0.0, 10.0);
        let quads = filled_with(&chart.paint(WIDE), chart.series.get());
        // On a flat cap the two offsets are the same vector, so the cap itself is
        // `line_width` long rather than a mitre's `half / cos(φ/2)`.
        let first = &quads[0];
        let cap = magnitude((first[0].0 - first[3].0, first[0].1 - first[3].1));
        assert_close(cap, LINE_WIDTH, "the first cap");
        let last = quads.last().expect("two quads");
        let cap = magnitude((last[1].0 - last[2].0, last[1].1 - last[2].1));
        assert_close(cap, LINE_WIDTH, "and the last");
    }

    #[test]
    fn a_right_angle_turn_is_mitred_to_half_root_two() {
        // `half / cos(φ/2)` at `φ = 90°` is `half · √2`: 2.1213 px on a 3-pixel
        // line, against the 3.0 px a flat pair of caps manages. That factor is
        // what fills the corner, and it is measured here off the recorded quad
        // rather than asserted from the closed form.
        // A genuine right angle on this fixture needs the pitch and the rise to
        // be equal: three samples put the middle one 300 pixels along and a
        // 300-unit rise puts it 300 pixels up, over a 0..300 range on a 300-tall
        // plot. One unit is one pixel, so `[0, 300, 0]` is exactly 90°.
        let (_, chart) = line(&spiking(3, 0.0, 300.0), 0.0, 300.0);
        let quads = filled_with(&chart.paint(WIDE), chart.series.get());
        assert_eq!(quads.len(), 2);
        let plot = chart.plot_rect(WIDE);
        let vertex = chart.points(plot, chart.y_range())[1];
        // The mitre corner at the vertex is the second corner of the segment that
        // *ends* there — a segment's quad is `[a + A, b + B, b − B, a − A]`, so
        // corner 1 is `b` with the far end's offset. Measuring "the furthest
        // corner of either quad" would be measuring the length of a segment
        // instead, which is what the first draft of this test did.
        let corner = quads[0][1];
        let reach = magnitude((corner.0 - vertex.0, corner.1 - vertex.1));
        let half = LINE_WIDTH / 2.0;
        assert_close(
            reach,
            half * 2.0_f32.sqrt(),
            "the mitre corner at a right angle",
        );
        assert!(
            reach > half * 1.4 && reach < LINE_WIDTH,
            "so it is √2 of the half width — past a flat cap's and short of the stroke's own: {reach}"
        );
        assert_close(
            reach / half,
            2.0_f32.sqrt(),
            "which is the 1/cos(45°) the closed form says",
        );
    }

    #[test]
    fn a_gentle_turn_is_mitred_to_almost_exactly_half() {
        // The other end of the table: a 30° turn mitres to `half / cos(15°)` =
        // 1.5529 px, so the corner barely leaves the stroke. It is checked because
        // a mitre that were a *bevel* everywhere would also pass the width test
        // above, and only this one separates the two.
        // A 30-degree turn on this fixture: a 300-pixel pitch and a rise of
        // `300·tan(15°)` = 80.4 px, which is 5.359 units of a 0..10 range over a
        // 300-tall plot — so `[0, 10, 4.641]`, and the peak's angle is
        // `2·atan(80.4/300)` = 30°.
        // Three samples put the middle one at the *middle* of this plot, so the
        // pitch is 300 px and a 30-degree turn needs a rise of `300·tan(15°)` =
        // 80.4 px, which is 2.679 units of a 0..10 range.
        let (_, chart) = line(&[0.0, 10.0, 7.321], 0.0, 10.0);
        let quads = filled_with(&chart.paint(WIDE), chart.series.get());
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let vertex = points[1];
        let corner = quads[0][1];
        let reach = magnitude((corner.0 - vertex.0, corner.1 - vertex.1));
        let half = LINE_WIDTH / 2.0;

        // **The turn is read off the geometry the chart made**, not out of the data
        // it was given: three samples put the middle one at the *middle* of the
        // plot, so the pitch is half the plot and the turn is not the one the data
        // alone suggests. Deriving it is what makes this a check of the closed form
        // rather than a check of my arithmetic — the first draft of this test
        // asserted 30° against data that turns 73°, and failed for the right reason.
        let turn = {
            let before = (vertex.0 - points[0].0, vertex.1 - points[0].1);
            let after = (points[2].0 - vertex.0, points[2].1 - vertex.1);
            let angle = |v: (f32, f32)| v.1.atan2(v.0);
            (angle(after) - angle(before)).abs().to_degrees()
        };
        assert!(turn > 10.0, "so the fixture really does turn: {turn}");
        let want = half / (turn.to_radians() / 2.0).cos();
        assert_close(reach, want, &format!("the mitre at a {turn}-degree turn"));
        // **The separation from a flat cap.** A pair of flat caps reaches `half`
        // from the vertex whatever the turn is, so `reach > half` is the whole of
        // what a mitre buys; at a gentle turn the margin is small, which is the
        // honest number, and the module's two tables are the same statement at four
        // other angles and at the two ends of a run.
        assert!(
            reach > half,
            "and a mitre reaches further from the vertex than a flat cap does: {reach} against {half}"
        );
    }

    #[test]
    fn a_turn_past_the_limit_gets_a_disc_and_no_spike() {
        // 21 samples on a 600-pixel plot: a 30-pixel pitch and a 300-pixel rise,
        // so slopes of ±10 and a turn of 167.98°, whose mitre is 9.55 `half` —
        // 14.33 px on a 3-pixel line. Past `MITRE_LIMIT` the join falls back to
        // each segment's own offset plus a disc at the vertex.
        let data = spiking(21, 0.0, 300.0);
        let (_, chart) = line(&data, 0.0, 300.0);
        let commands = chart.paint(WIDE);
        let discs = circles(&commands);
        assert_eq!(
            discs.len(),
            1,
            "one disc, at the vertex whose turn is over the limit — and only there"
        );
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let vertex = points[10];
        assert_close(discs[0].0 .0, vertex.0, "the disc is at the vertex, in x");
        assert_close(discs[0].0 .1, vertex.1, "and in y");
        assert_close(
            discs[0].1,
            LINE_WIDTH / 2.0,
            "and its radius is the half width",
        );
        assert_eq!(
            disc_centre_of(&commands),
            Some(vertex),
            "and it is the peak"
        );

        // **Only the two quads that touch the vertex.** A quad's other corners
        // belong to its far end, and a corner is measured from *its* vertex — the
        // first draft of this measured every corner of every quad from the peak
        // and reported a segment's own length as a mitre.
        let quads = filled_with(&commands, chart.series.get());
        // Segment 9 *ends* at the vertex and segment 10 *starts* there, so the
        // corners at it are quads[9]'s 1 and 2 and quads[10]'s 0 and 3. The other
        // four belong to the segments' far ends, which on this fixture is 300
        // pixels away — measuring those from the peak reported a segment's own
        // length as a mitre, which is the mistake the first draft of this made.
        for corner in [&quads[9][1], &quads[9][2], &quads[10][0], &quads[10][3]] {
            let reach = magnitude((corner.0 - vertex.0, corner.1 - vertex.1));
            assert!(
                reach <= LINE_WIDTH,
                "no corner at the vertex is past the stroke's own width: {reach}"
            );
        }
    }

    /// Returns the centre of the one disc a paint recorded, or `None` if there is
    /// not exactly one.
    fn disc_centre_of(commands: &[DrawCommand]) -> Option<(f32, f32)> {
        let discs = circles(commands);
        (discs.len() == 1).then_some(discs[0].0)
    }

    #[test]
    fn a_turn_just_inside_the_limit_still_mitres() {
        // The limit bites at `2 · acos(1/4)` = 151.045°. A 30-pixel pitch reaches
        // a turn of `2·atan(rise/30)`, so a rise of 146 px gives 156.4° — past it —
        // and a rise of 116 px gives 151.0° — a hair inside it. This is the pair
        // either side of the number, which is the only way to say the rule bites
        // *there*.
        // `spiking` puts the peak 300 px up, so its **angle** is `2·atan(300/30)`
        // = 168.6° whatever the range says — the range only decides how many units
        // that is. So a peak from `spiking` can never be *inside* the limit on this
        // fixture, which is a fact worth its own assertion: the pair either side of
        // 151.045° has to be built from `shown`, with a rise of
        // `300·tan(75.5°)` = 116 px, which this plot can hold.
        let peak_turn = 2.0 * (300.0_f32 / 30.0).atan().to_degrees();
        assert!(
            peak_turn > 151.045,
            "a 300-pixel rise on a 30-pixel pitch turns {peak_turn}°, so `spiking` cannot reach the inside"
        );
        let (_, chart) = line(&spiking(21, 0.0, 116.0), 0.0, 116.0);
        assert_eq!(circles(&chart.paint(WIDE)).len(), 1, "and it gets a disc");

        let peaked = |rise: f32| Series {
            x: (0..21).map(|index| count_to_f32(index) / 20.0).collect(),
            values: (0..21)
                .map(|index| if index == 10 { rise } else { 0.0 })
                .collect(),
        };
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_fixed_range(0.0, 300.0);
        chart.shown.set(peaked(116.0));
        assert!(
            2.0 * (116.0_f32 / 30.0).atan().to_degrees() < 151.045,
            "116 px on a 30 pitch turns {}°, which is inside the limit",
            2.0 * (116.0_f32 / 30.0).atan().to_degrees()
        );
        assert!(
            circles(&chart.paint(WIDE)).is_empty(),
            "so the peak is a mitred corner and no disc"
        );
        chart.shown.set(peaked(122.0));
        assert_eq!(
            circles(&chart.paint(WIDE)).len(),
            1,
            "and 122 px — {}° — is past it",
            2.0 * (122.0_f32 / 30.0).atan().to_degrees()
        );
    }

    #[test]
    fn the_peak_turn_of_this_fixtures_spikes_is_where_the_pitch_puts_it() {
        // The number every other limit test rests on, measured rather than
        // remembered: **three** samples put the peak at the *middle* of a 600-px
        // plot, so its sides run 300 px along and 300 px up and the turn is
        // `2·atan(1)` = 90°. **Twenty-one** samples put it 30 px along and 300 px
        // up, so the turn is `2·atan(10)` = 168.58°. The first draft of this
        // claimed three samples gave a 90° turn for the reason above and then
        // asserted 168° for the same fixture, and the tests were written against
        // the wrong one.
        // `spiking` puts every reading at `low` and one at `high`, and `line()`
        // fixes the range to exactly `[low, high]`, so the peak's rise is the plot's
        // whole height however many samples there are.
        let turn_of = |count: usize| {
            let pitch = WIDE.width / count_to_f32(one_fewer(count));
            2.0 * (WIDE.height / pitch).atan().to_degrees()
        };
        assert_close(turn_of(3), 90.0, "three samples on this plot");
        assert_close(turn_of(21), 168.579_24, "and twenty-one");

        let (_, chart) = line(&spiking(3, 0.0, 300.0), 0.0, 300.0);
        assert!(
            circles(&chart.paint(WIDE)).is_empty(),
            "so three samples are nowhere near the limit and get no disc"
        );
    }

    #[test]
    fn the_ends_of_a_run_never_get_a_disc() {
        // A run's two ends have one neighbour each, so their mitre ratio is
        // exactly one and no limit can fire on them however spiky the run is.
        let (_, chart) = line(&[0.0, 300.0], 0.0, 300.0);
        assert!(
            circles(&chart.paint(WIDE)).is_empty(),
            "two samples are one segment and no vertex inside it"
        );
    }

    #[test]
    fn a_turn_over_the_limit_does_not_leave_a_notch() {
        // The fallback is a disc rather than a clamped mitre because a clamped
        // mitre would put the stroke *thinner* than the caller asked for: the
        // clamped corner's distance from the centre line is `limit · half ·
        // cos(φ/2)`, and at 168° that is `6.0 · 0.105` = 0.63 px each side where
        // 1.5 was asked for. The disc keeps it at exactly the half width.
        let (_, chart) = line(&spiking(21, 0.0, 300.0), 0.0, 300.0);
        let quads = filled_with(&chart.paint(WIDE), chart.series.get());
        assert_eq!(
            quads.len(),
            20,
            "one quad per segment of twenty-one samples"
        );
        for (index, quad) in quads.iter().enumerate() {
            assert_close(across(quad), LINE_WIDTH, &format!("segment {index} across"));
        }
    }

    #[test]
    fn a_line_chart_with_no_width_draws_no_series() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_fixed_range(0.0, 10.0);
        chart.data.set(vec![0.0, 5.0, 10.0]);
        chart.set_line_width(0.0);
        chart.snap_to_state();
        let commands = chart.paint(WIDE);
        assert!(polygons(&commands).is_empty(), "no series at all");
        assert_eq!(
            lines(&commands).len(),
            one_fewer(GRID_DIVISIONS) + 2,
            "and the frame is still there"
        );
    }

    /// The number the false `paint` doc missed, measured from the recorded
    /// geometry rather than from the samples: a peak **on the plot's top edge**
    /// whose turn is exactly at `MITRE_LIMIT`, so the join is still a corner
    /// and its outer point sits a whole `MITRE_LIMIT · line_width / 2` above
    /// `rect.y`.
    ///
    /// **6 px, not 1.5.** The turn is placed at the limit and the peak at the
    /// range's high, which puts it on the plot's top edge, which is `rect.y`:
    /// there is no top gutter to hide in. A cap is `line_width / 2`, so this
    /// assertion fails against the old claim's number by a factor of four.
    /// The pitch is chosen so the turn lands just inside the limit — `2·atan` of
    /// `(MITRE_LIMIT^2 - 1)^0.5 / MITRE_LIMIT` scaled by the pitch — which is where
    /// `|w|` is `MITRE_LIMIT · half` and therefore as long as a corner ever gets.
    /// A flat cap would put the furthest point at `half`, a quarter of this, so a
    /// test that passed at `half` would have passed with the bug it exists to catch.
    #[test]
    fn the_reach_of_a_peak_at_the_high_is_the_mitre_limit_times_the_half_width() {
        let rect = Rect::new(300.0, 240.0, 480.0, 240.0);
        let count = 21;
        let pitch = rect.width / (count as f32 - 1.0);
        // The turn that saturates `MITRE_LIMIT`: `2·acos(1 / MITRE_LIMIT)`.
        let turn = 2.0 * (1.0 / MITRE_LIMIT).acos();
        let rise = (turn / 2.0).tan() * pitch;
        let low = 100.0 - rise / rect.height * 100.0;
        let mut data = vec![low; count];
        data[count / 2] = 100.0;

        let (_, chart) = line(&data, 0.0, 100.0);
        let (left, right, top, bottom) = reach_of(&chart, rect);

        assert_close(top, MITRE_LIMIT * LINE_WIDTH / 2.0, "and it is a mitre");
        assert!(
            top > LINE_WIDTH / 2.0,
            "and it is four times a cap, not a cap"
        );
        assert!(
            top > 0.0,
            "and it is outside the node rect, which is the point"
        );

        // The corner is one point `stroke_reach()` from its vertex, so it is
        // bounded on every axis — sideways it is a quarter of the reach, because
        // the turn is past 90 degrees and the offset has swung that far round.
        assert!(
            left <= LINE_WIDTH / 2.0,
            "a corner does not overhang sideways past a cap"
        );
        assert!(
            right <= LINE_WIDTH / 2.0,
            "a corner does not overhang sideways past a cap"
        );
        assert!(left + right > 0.0, "and it does overhang sideways at all");
        // The peak is the only thing above the edge, and it is *four times* the
        // axes' own overhang — which is `MITRE_LIMIT`, and is the ratio the old
        // claim denied by naming `line_width / 2` as its one exception.
        assert_close(top, MITRE_LIMIT * LINE_WIDTH / 2.0, "the mitre at the peak");
        assert_close(
            left,
            AXIS_WIDTH / 2.0,
            "and what reaches the left is the y axis",
        );
        assert_close(bottom, AXIS_WIDTH / 2.0, "and the bottom is the x axis");
        assert_close(right, 0.0, "and nothing reaches the right");
        // Twice the line's own width, and six times the axes' overhang: both are
        // consequences of `MITRE_LIMIT` rather than of anything the caps can do,
        // and the old claim's `line_width / 2` is a third of the first.
        assert_close(top, 2.0 * LINE_WIDTH, "twice the line's own width");
        assert_close(top, 6.0 * AXIS_WIDTH / 2.0, "and six times the axes'");
        assert!(top > LINE_WIDTH / 2.0, "and four times a cap, not a cap");
        assert!(
            top > 0.0,
            "and it is outside the node rect, which is the point"
        );

        // The accessor is the number a caller needs, so it is the number this
        // pins — with the peak inside the rect by a hair short of it.
        assert_close(
            chart.stroke_reach(),
            MITRE_LIMIT * LINE_WIDTH / 2.0,
            "the reach",
        );
        assert!(
            reach_of(&chart, rect).2 <= chart.stroke_reach(),
            "the drawn reach is inside the drawn reach it reports"
        );

        // And it follows the line width, which is why it is not a constant.
        let mut wider = line(&data, 0.0, 100.0).1;
        wider.set_line_width(0.0);
        assert_close(wider.stroke_reach(), 0.0, "a zero width reaches nothing");
        wider.set_line_width(6.0);
        assert_close(wider.stroke_reach(), MITRE_LIMIT * 3.0, "and it doubled");
    }

    /// Nothing but the documented bounds leaves the node rect, across every chart
    /// type, a degenerate shape, a rect with no room at all, **two line widths and
    /// three label sets**.
    ///
    /// The per-type allowance is the point: the stroke is bounded by
    /// [`stroke_reach`](Chart::stroke_reach), and the axes by half their own
    /// width. A single allowance for all of them would pass with the axes at a
    /// stroke's reach, which is four times what they need.
    ///
    /// **The label column is what this battery was missing, and it is the reason
    /// the bound is two numbers.** Before it, every chart here was unlabelled and
    /// every chart here ran the default 3-pixel line, so the two bounds were
    /// **equal at 6.0 px** in every single case and an allowance of
    /// `stroke_reach` alone passed. [`set_line_width`](Chart::set_line_width) is
    /// public and [`LABEL_FONT_SIZE`] is not, so `width = 1.0` takes the stroke's
    /// reach to **2.0** and leaves the label's box at **6.0**, and the label becomes
    /// the larger bound. [`geometry`] drops text — [`DrawCommand::Text`] carries no
    /// width, so a label's *horizontal* reach is unknowable and `Chart::paint`'s
    /// table says so — but its **`y` is the top edge of the line box**, which is the
    /// one number a caller can check, and that is what the label assertion uses.
    #[test]
    fn nothing_but_the_stroke_reach_or_a_label_line_box_leaves_the_node_rect() {
        let rects = [
            Rect::new(300.0, 240.0, 480.0, 240.0),
            Rect::new(0.0, 0.0, 480.0, 240.0),
            Rect::new(0.0, 0.0, 8.0, 8.0),
            Rect::new(0.0, 0.0, 0.0, 0.0),
            Rect::new(-40.0, -40.0, 80.0, 80.0),
        ];
        let series = [
            vec![],
            vec![50.0],
            vec![0.0, 100.0],
            vec![100.0, 0.0, 100.0, 0.0, 100.0],
            spiking(9, 0.0, 100.0),
            spiking(33, -1e6, 1e6),
            vec![f32::NAN, 50.0, f32::INFINITY, f32::NEG_INFINITY],
        ];
        // The default and a thin line: the second is where the label's own 6 px
        // overtakes a reach of 2.0, which is the case the allowance has to be the
        // larger of rather than the stroke's.
        let widths = [LINE_WIDTH, 1.0];
        let label_sets: [(&[&str], &[&str]); 3] = [
            (&[], &[]),
            (&["100"], &["a", "b", "c"]),
            (&["100", "50", "0"], &[]),
        ];
        let mut checked = 0_u32;
        let mut labelled = 0_u32;
        let mut widest_label = 0.0_f32;
        for rect in rects {
            for chart_type in [ChartType::Line, ChartType::Bar, ChartType::Area] {
                for grid in [false, true] {
                    for width in widths {
                        for data in &series {
                            for (ys, xs) in label_sets {
                                let mut nodes = Arena::new();
                                let mut chart = Chart::new(&mut nodes, chart_type);
                                chart.set_palette(Palette::from_theme(&Theme::dark()));
                                chart.set_grid_visible(grid);
                                chart.set_line_width(width);
                                chart
                                    .y_labels
                                    .set(ys.iter().map(|l| (*l).to_string()).collect());
                                chart
                                    .x_labels
                                    .set(xs.iter().map(|l| (*l).to_string()).collect());
                                chart.data.set(data.clone());
                                chart.snap_to_state();

                                let is_bar = chart_type == ChartType::Bar;
                                // Two bounds, and the allowance is the larger: the
                                // axes (or the stroke) from the geometry, and the
                                // first y label's line box from the labels.
                                let drawn_allowance = if is_bar {
                                    AXIS_WIDTH / 2.0
                                } else {
                                    chart.stroke_reach()
                                };
                                let label_allowance = if ys.is_empty() {
                                    0.0
                                } else {
                                    LABEL_FONT_SIZE / 2.0
                                };
                                let allowance = drawn_allowance.max(label_allowance);
                                let what = format!(
                                    "{chart_type:?} grid {grid} width {width} \
                                     {} y {ys:?} x {xs:?} over {rect:?}",
                                    series_label(data)
                                );
                                let commands = chart.paint(rect);
                                let (left, right, top, bottom) = reach_within(&commands, rect);
                                for (reach, axis) in [
                                    (left, "left"),
                                    (right, "right"),
                                    (top, "top"),
                                    (bottom, "bottom"),
                                ] {
                                    assert!(
                                        reach <= allowance + 0.01,
                                        "{what}: {reach} past the {axis}, allowed {allowance}"
                                    );
                                }
                                // And the label's own box, measured off the same
                                // paint: `y` is the **top** of its line box, and that
                                // is the one edge a caller can check, since
                                // `DrawCommand::Text` carries no width.
                                for (_, y, text, _) in texts(&commands) {
                                    let past = (rect.y - y).max(0.0);
                                    assert!(
                                        past <= label_allowance + 0.01,
                                        "{what}: label {text:?} is {past} above the node, \
                                         allowed {label_allowance}"
                                    );
                                    widest_label = widest_label.max(past);
                                }
                                if !ys.is_empty() {
                                    labelled += 1;
                                }
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(checked >= 200, "the battery actually ran: {checked} charts");
        assert!(
            labelled > 200,
            "and a third of them carried a y label: {labelled}"
        );
        // The battery saw a real overhang from a label, so it is not passing
        // because every labelled chart happened to fit — and the overhang is
        // **exactly** `LABEL_FONT_SIZE / 2`, not merely under it, because
        // `plot.y` is `rect.y` whatever the labels do (the bottom gutter comes off
        // the *height*). That is the number `Chart::paint`'s headline publishes, so
        // it is compared and not merely bounded.
        assert_close(
            widest_label,
            LABEL_FONT_SIZE / 2.0,
            "the battery saw a label's line box leave the node by its own half font",
        );
    }

    /// The battery above, searched rather than listed: a random series, a random
    /// turn, and the reach it draws, over two hundred thousand charts.
    ///
    /// The listed cases pin the *shapes*; this pins the *bound*. `MITRE_LIMIT` is
    /// what makes the corner finite, so a change to it is exactly the kind of
    /// change this searches for — and a `paint` that grew the plot's top gutter
    /// would make every listed case pass while this one still caught it.
    #[test]
    fn the_reach_stays_inside_the_stroke_reach_over_two_hundred_thousand_charts() {
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((seed >> 33) as f32 / 2_147_483_648.0) - 1.0
        };
        let mut checked = 0_u32;
        let mut attempts = 0;
        let mut worst = 0.0_f32;
        while checked < 200_000 && attempts < 2_000_000 {
            attempts += 1;
            let count = 3 + (next().abs() * 6.0) as usize;
            let span = 200.0 + next().abs() * 400.0;
            // A reading at the exact high lands on the plot's top edge, which is
            // `rect.y`: that is where the overhang is measurable at all.
            let data: Vec<f32> = (0..count)
                .map(|_| {
                    if next() > 0.9 {
                        100.0
                    } else {
                        100.0 + next() * 100.0
                    }
                })
                .collect();
            let mut nodes = Arena::new();
            let mut chart = Chart::new(&mut nodes, ChartType::Line);
            chart.set_palette(Palette::from_theme(&Theme::dark()));
            chart.set_line_width(0.5 + next().abs() * 6.0);
            chart.data.set(data);
            chart.snap_to_state();

            let rect = Rect::new(next().abs() * 40.0, 0.0, span, span * 0.5);
            let (left, right, top, bottom) = reach_of(&chart, rect);
            let allowance = chart.stroke_reach();
            worst = worst.max(left).max(right).max(top).max(bottom);
            for (reach, axis) in [
                (left, "left"),
                (right, "right"),
                (top, "top"),
                (bottom, "bottom"),
            ] {
                assert!(
                    reach <= allowance + 0.01,
                    "random chart {attempts}: {reach} past the {axis}, \
                     allowed {allowance} (width {})",
                    chart.line_width()
                );
            }
            checked += 1;
        }
        assert!(
            checked >= 200_000,
            "the search actually ran: {checked} charts"
        );
        // And the search found a real overhang to measure, so it is not passing
        // because every random chart happened to fit.
        assert!(worst > 0.0, "the search saw an overhang: {worst}");
    }

    #[test]
    fn every_segment_quad_is_convex_over_two_hundred_thousand_geometries() {
        // The module's proof is that both offsets of a segment satisfy
        // `A · n = B · n = half`, which makes every turn of the quad carry the
        // same factor. This searches for the counterexample that proof rules out,
        // and the count is in the test's name so a reader knows what was searched.
        let mut seed: u64 = 0x5EED_C4A2_1D3B_77F1;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((seed >> 33) as f32 / 2_147_483_648.0) - 1.0
        };
        let mut checked = 0_u32;
        let mut attempts = 0;
        while checked < 200_000 && attempts < 2_000_000 {
            attempts += 1;
            let count = 3 + (next().abs() * 4.0) as usize;
            let points: Vec<(f32, f32)> = (0..count)
                .map(|_| (200.0 + next() * 200.0, 150.0 + next() * 150.0))
                .collect();
            let half = 0.5 + next().abs() * 6.0;
            let joins: Vec<Option<Join>> = points
                .iter()
                .enumerate()
                .map(|(index, point)| {
                    join_at(
                        *point,
                        index.checked_sub(1).map(|before| points[before]),
                        points.get(index + 1).copied(),
                        half,
                    )
                })
                .collect();
            for index in 0..count.saturating_sub(1) {
                let (Some(here), Some(there)) = (joins[index], joins[index + 1]) else {
                    continue;
                };
                let (a, b) = (points[index], points[index + 1]);
                let (start, end) = (here.outgoing(), there.incoming());
                let quad = [
                    (a.0 + start.0, a.1 + start.1),
                    (b.0 + end.0, b.1 + end.1),
                    (b.0 - end.0, b.1 - end.1),
                    (a.0 - start.0, a.1 - start.1),
                ];
                assert_convex(
                    &quad,
                    &format!("random geometry {attempts} segment {index}"),
                );
                assert_close(across(&quad), half * 2.0, "and it is the requested width");
                checked += 1;
            }
        }
        assert!(
            checked >= 200_000,
            "the search actually ran: {checked} quads"
        );
    }

    #[test]
    fn every_polygon_a_chart_records_is_convex() {
        // The search above checks the line's quads. This checks the ones a chart
        // *records*, on a battery of real shapes including the degenerate ones, so
        // that the invariant is asserted on the artifact rather than on a
        // reconstruction of it.
        let shapes: [&[f32]; 7] = [
            &[],
            &[5.0],
            &[5.0, 5.0],
            &[0.0, 10.0, 0.0],
            &[0.0, 10.0, 9.0, 10.0, 0.0, 10.0],
            &[0.0, 300.0, 1.0, 0.0],
            &[-10.0, -5.0, 0.0, 5.0, 10.0],
        ];
        for chart_type in [ChartType::Line, ChartType::Bar, ChartType::Area] {
            for shape in shapes {
                for rect in [WIDE, SQUARE] {
                    let (_, chart) = showing(chart_type, shape, -10.0, 310.0);
                    for (index, (points, _)) in polygons(&chart.paint(rect)).into_iter().enumerate()
                    {
                        assert_convex(
                            &points,
                            &format!("{chart_type:?} {shape:?} polygon {index}"),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_run_of_one_sample_is_no_segment_in_either_geometry() {
        // `each_run`'s range is a range of *segments*, so a run of one calls
        // nothing — which is the whole of "a line chart cannot draw a segment" and
        // "an area chart can draw nothing".
        let points = [(0.0, 1.0), (1.0, f32::NAN), (2.0, 3.0), (3.0, 4.0)];
        let mut runs = Vec::new();
        each_run(&points, |start, end| runs.push((start, end)));
        assert_eq!(runs, vec![(2, 3)], "only the two-sample run has a segment");
        assert!(
            points[2].1 < points[3].1,
            "and the gap in the middle of the first three is what splits them"
        );
    }

    #[test]
    fn each_run_reports_the_segments_of_every_run_of_a_gappy_series() {
        let points = [
            (0.0, 1.0),
            (1.0, 2.0),
            (2.0, f32::NAN),
            (3.0, 4.0),
            (4.0, 5.0),
            (5.0, 6.0),
            (6.0, f32::NAN),
            (7.0, 8.0),
        ];
        let mut runs = Vec::new();
        each_run(&points, |start, end| runs.push((start, end)));
        assert_eq!(
            runs,
            vec![(0, 1), (3, 5)],
            "the lone sample at the end is a run with no segments in it"
        );
    }

    // ---- The area fill ------------------------------------------------------

    #[test]
    fn the_area_fill_is_one_quad_per_segment_and_nothing_more() {
        let (_, chart) = area(&[0.0, 5.0, 10.0], 0.0, 10.0);
        let commands = chart.paint(WIDE);
        let fill = filled_with(&commands, chart.fill.get());
        let line = filled_with(&commands, chart.series.get());
        assert_eq!(fill.len(), 2, "two fill quads");
        assert_eq!(line.len(), 2, "and two stroke quads over them");
        for quad in &fill {
            assert_eq!(quad.len(), 4, "each is four points, so the fan is exact");
        }
    }

    #[test]
    fn every_fill_quad_runs_from_its_segments_points_down_to_the_plots_bottom() {
        let (_, chart) = area(&[1.0, 7.0, 3.0, 9.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let bottom = plot.y + plot.height;
        let fill = filled_with(&chart.paint(WIDE), chart.fill.get());
        assert_eq!(fill.len(), 3);
        for (index, quad) in fill.iter().enumerate() {
            assert_close(quad[0].0, points[index].0, "the quad starts at its sample");
            assert_close(quad[1].0, points[index + 1].0, "and ends at the next");
            assert_close(quad[0].1, points[index].1, "at the line's own height");
            assert_close(quad[1].1, points[index + 1].1, "and so does the next");
            assert_close(quad[2].1, bottom, "the third point is on the bottom edge");
            assert_close(quad[3].1, bottom, "and so is the fourth");
        }
    }

    #[test]
    fn the_fill_covers_the_region_under_the_line_and_does_not_spill_outside_it() {
        // Not a shape assertion: the **total area**, which is the only number
        // that can see a quad drawn too tall or too far. The quads are disjoint in
        // x, so the union's area is their sum, and the ideal is the trapezoidal
        // area under the polyline. Both are computed from the same points.
        let data = [0.0, 4.0, 9.0, 2.0, 7.0, 1.0];
        let (_, chart) = area(&data, 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let range = chart.y_range();
        let points = chart.points(plot, range);
        let bottom = plot.y + plot.height;

        let painted: f32 = filled_with(&chart.paint(WIDE), chart.fill.get())
            .iter()
            .map(|quad| quad_area(quad))
            .sum();
        let ideal: f32 = points
            .windows(2)
            .map(|pair| {
                let run = (pair[0].1 - bottom).abs() + (pair[1].1 - bottom).abs();
                run * (pair[1].0 - pair[0].0).abs() / 2.0
            })
            .sum();
        assert!(
            (painted - ideal).abs() < 1.0,
            "painted {painted} against ideal {ideal}, within a pixel squared"
        );
        // And nothing above the line, which is the other half of "does not spill".
        for quad in filled_with(&chart.paint(WIDE), chart.fill.get()) {
            assert!(quad[0].1 <= bottom && quad[1].1 <= bottom);
            assert!(quad.iter().all(|point| point.1 >= plot.y - 0.01));
        }
    }

    #[test]
    fn the_fill_runs_to_the_plots_bottom_and_not_to_the_zero_line() {
        // An area is *the region under a curve*, so the bottom edge is its
        // boundary. With a range straddling zero the two differ, and the widget
        // documents which one it uses.
        let (_, chart) = area(&[-8.0, 4.0, -2.0], -10.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let baseline = chart.bar_baseline(plot, chart.y_range());
        let fill = filled_with(&chart.paint(WIDE), chart.fill.get());
        let bottom = plot.y + plot.height;
        for quad in &fill {
            assert_close(quad[2].1, bottom, "every fill quad ends on the bottom edge");
        }
        assert!(
            (baseline - bottom).abs() > 1.0,
            "and that is not where the bars would have started: {baseline} against {bottom}"
        );
        // The quad from the −8 up to the +4 **crosses** the zero line, which is
        // the whole difference between a fill to the plot's bottom and a fill to
        // zero: a fill to zero would leave the part below it uncovered.
        assert!(
            fill[0][0].1 > baseline && fill[0][1].1 < baseline,
            "the first fill quad straddles the zero line: {} and {} against {baseline}",
            fill[0][0].1,
            fill[0][1].1
        );
        assert!(
            points.iter().any(|point| point.1 > baseline),
            "and a negative reading is drawn below it"
        );
    }

    #[test]
    fn an_area_chart_of_one_sample_draws_nothing() {
        let (_, chart) = area(&[5.0], 0.0, 10.0);
        let commands = chart.paint(WIDE);
        assert!(
            polygons(&commands).is_empty(),
            "a fill needs a segment, and so does a line"
        );
        assert_eq!(
            lines(&commands).len(),
            one_fewer(GRID_DIVISIONS) + 2,
            "but the frame is there"
        );
    }

    // ---- The bars -----------------------------------------------------------

    #[test]
    fn a_bar_is_inside_the_plot_and_the_pitch_wide() {
        let (_, chart) = bars(&[2.0, 6.0, 9.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 3);
        let slot = plot.width / 3.0;
        for (index, (bar, point)) in drawn.iter().zip(points.iter()).enumerate() {
            assert_close(
                bar.width,
                slot * BAR_WIDTH_FRACTION,
                &format!("bar {index} is BAR_WIDTH_FRACTION of its slot"),
            );
            // Every bar is where it *can* be: an evenly spaced series puts its first
            // sample on the plot's left edge and its last on the right, and a bar
            // centred there hangs half its own width over both, so the end bars are
            // pulled in until their outer edges land on the plot's.
            let want = bounded(
                point.0,
                plot.x + bar.width / 2.0,
                plot.x + plot.width - bar.width / 2.0,
            );
            assert_close(
                bar.x + bar.width / 2.0,
                want,
                &format!("bar {index} is inside the plot"),
            );
        }
        assert_close(
            drawn[1].x + drawn[1].width / 2.0,
            points[1].0,
            "and the interior one is still centred on its own sample",
        );
        assert_close(
            drawn[0].x,
            plot.x,
            "while the first bar's outer edge is on the plot's left edge",
        );
        assert_close(
            drawn[2].x + drawn[2].width,
            plot.x + plot.width,
            "and the last one's is on the right",
        );
    }

    #[test]
    fn a_bar_of_a_two_sample_chart_is_bounded_by_the_plot_and_not_by_the_pitch() {
        // Two samples have a pitch of the whole plot, so three fifths of it is 360
        // px on a 600-pixel plot — and a bar centred on the plot's left edge would
        // then put 180 px of itself outside the caller's rect.
        let (_, chart) = bars(&[5.0, 5.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 2);
        for bar in &drawn {
            assert!(bar.width <= plot.width, "no bar is wider than the plot");
            assert!(bar.x >= plot.x - 0.01, "and none is off its left edge");
            assert!(
                bar.x + bar.width <= plot.x + plot.width + 0.01,
                "or its right edge either"
            );
        }
        assert_close(
            drawn[0].width,
            plot.width / 2.0 * BAR_WIDTH_FRACTION,
            "at 180 px — three fifths of a half-plot slot, not of the 600 px pitch",
        );
        assert!(
            drawn[1].x > drawn[0].x + drawn[0].width,
            "and the two do not meet: {} against {}",
            drawn[1].x,
            drawn[0].x + drawn[0].width
        );
    }

    #[test]
    fn a_bars_height_is_the_distance_from_the_baseline_to_its_own_value() {
        let (_, chart) = bars(&[2.5, 5.0, 7.5], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let baseline = chart.bar_baseline(plot, chart.y_range());
        assert_close(
            baseline,
            plot.y + plot.height,
            "an all-positive range grows from the bottom",
        );
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 3);
        for (bar, point) in drawn.iter().zip(points.iter()) {
            assert_close(
                bar.y + bar.height,
                baseline,
                "a bar's bottom is the baseline",
            );
            assert_close(bar.y, point.1, "and its top is its value's own place");
            assert_close(
                bar.height,
                (baseline - point.1).abs(),
                "so the height is the distance",
            );
        }
        assert_close(
            drawn[0].height,
            plot.height * 0.25,
            "the 2.5 is a quarter of the plot",
        );
        assert_close(
            drawn[2].height,
            plot.height * 0.75,
            "and the 7.5 three quarters",
        );
    }

    #[test]
    fn a_bar_chart_of_one_sample_draws_one_bar_at_the_pitch_of_the_whole_plot() {
        let (_, chart) = bars(&[5.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 1, "one sample is one bar");
        assert_close(
            drawn[0].width,
            plot.width * BAR_WIDTH_FRACTION,
            "the pitch is the plot",
        );
        assert_close(
            drawn[0].x + drawn[0].width / 2.0,
            plot.x + plot.width / 2.0,
            "and it is centred on the plot",
        );
    }

    #[test]
    fn a_negative_reading_hangs_below_the_zero_baseline() {
        let (_, chart) = bars(&[-6.0, 4.0, 10.0], -10.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let baseline = chart.bar_baseline(plot, chart.y_range());
        assert!(
            (baseline - plot.y - plot.height / 2.0).abs() < 0.01,
            "a range straddling zero starts its bars at zero: {baseline}"
        );
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 3);
        assert_close(
            drawn[0].y,
            baseline,
            "the negative bar's top is the zero line, where every bar starts",
        );
        assert!(
            drawn[0].y + drawn[0].height > drawn[0].y,
            "and its bottom is below it, so it hangs downward: {}",
            drawn[0].y + drawn[0].height
        );
        assert_close(
            drawn[0].y + drawn[0].height,
            chart.points(plot, chart.y_range())[0].1,
            "which is where the −6 is drawn",
        );
        assert!(
            drawn[1].y < baseline && drawn[1].y + drawn[1].height == baseline,
            "while the positive one rises up to it"
        );
    }

    #[test]
    fn an_all_negative_range_grows_its_bars_from_the_top() {
        let (_, chart) = bars(&[-4.0, -10.0], -10.0, -2.0);
        let plot = chart.plot_rect(WIDE);
        let drawn = rects(&chart.paint(WIDE));
        for bar in &drawn {
            assert_close(bar.y, plot.y, "a range below zero starts at the top");
        }
        assert!(
            drawn[1].height > drawn[0].height,
            "and the deeper reading is the longer bar"
        );
    }

    #[test]
    fn a_reading_on_the_baseline_is_no_bar_at_all() {
        let (_, chart) = bars(&[0.0, 5.0], 0.0, 10.0);
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 1, "a rectangle of no height draws nothing");
    }

    #[test]
    fn a_bar_of_a_gap_is_not_recorded() {
        let (_, chart) = bars(&[5.0, f32::NAN, 9.0], 0.0, 10.0);
        assert_eq!(
            rects(&chart.paint(WIDE)).len(),
            2,
            "the middle one has no reading"
        );
    }

    #[test]
    fn a_chart_of_more_samples_than_the_plot_has_pixels_draws_overlapping_bars() {
        // Documented rather than prevented: a bar is at least `MIN_BAR_WIDTH`
        // whatever the pitch, so below a pixel of pitch they overlap.
        // Never zero: a reading on the baseline is no bar, which is its own
        // documented fact and would make this a test of two things at once.
        let data: Vec<f32> = (0..900).map(|index| 1.0 + (index % 7) as f32).collect();
        let (_, chart) = bars(&data, 0.0, 8.0);
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 900, "every reading with one is a bar");
        assert!(drawn.iter().all(|bar| bar.width >= MIN_BAR_WIDTH));
        let pitch = WIDE.width / 899.0;
        assert!(drawn[0].width > pitch, "and a bar is wider than the pitch");
    }

    #[test]
    fn a_zero_bar_fraction_still_draws_a_bar_of_the_minimum_width() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Bar);
        chart.set_bar_fraction(0.0);
        chart.set_fixed_range(0.0, 10.0);
        chart.data.set(vec![4.0, 10.0]);
        chart.snap_to_state();
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 2);
        assert!(drawn.iter().all(|bar| bar.width >= MIN_BAR_WIDTH));
    }

    // ---- The frame and the labels -------------------------------------------

    #[test]
    fn the_frame_is_the_grid_then_the_two_axes() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        let plot = chart.plot_rect(WIDE);
        let grid = lines(&chart.paint(WIDE));
        let interior = one_fewer(GRID_DIVISIONS);
        assert_eq!(
            grid.len(),
            interior + 2,
            "the grid's lines and the two axes"
        );
        for (index, line) in grid.iter().take(interior).enumerate() {
            assert_close(
                line.0 .1,
                line.1 .1,
                &format!("grid line {index} is horizontal"),
            );
            assert_close(line.0 .0, plot.x, "and starts at the plot's left edge");
            assert_close(line.1 .0, plot.x + plot.width, "and ends at its right");
            assert_close(line.2, GRID_WIDTH, "at the grid's width");
        }
        let (y_axis, x_axis) = (grid[interior], grid[interior + 1]);
        assert_close(y_axis.0 .0, plot.x, "the y axis is up the left edge");
        assert_close(
            y_axis.1 .1,
            plot.y + plot.height,
            "from the top to the bottom",
        );
        assert_close(
            x_axis.0 .1,
            plot.y + plot.height,
            "and the x axis along the bottom",
        );
        assert_close(
            x_axis.1 .0,
            plot.x + plot.width,
            "from the left to the right",
        );
        assert_close(y_axis.2, AXIS_WIDTH, "both at the axis width");
    }

    #[test]
    fn a_grid_off_leaves_only_the_two_axes() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.set_fixed_range(0.0, 10.0);
        chart.data.set(vec![0.0, 10.0]);
        chart.set_grid_visible(false);
        chart.snap_to_state();
        assert_eq!(lines(&chart.paint(WIDE)).len(), 2);
    }

    #[test]
    fn the_grid_is_horizontal_only() {
        let (_, chart) = line(&[0.0, 5.0, 10.0], 0.0, 10.0);
        for line in lines(&chart.paint(WIDE))
            .iter()
            .take(one_fewer(GRID_DIVISIONS))
        {
            assert_close(line.0 .1, line.1 .1, "no grid line runs vertically");
        }
    }

    #[test]
    fn y_labels_are_spread_from_the_plots_top_to_its_bottom() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        chart
            .y_labels
            .set(vec!["100".to_string(), "50".to_string(), "0".to_string()]);
        let plot = chart.plot_rect(WIDE);
        let labels = texts(&chart.paint(WIDE));
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0].2, "100", "the caller's own order, top first");
        assert_eq!(labels[2].2, "0");
        for (index, (x, y, _, size)) in labels.iter().enumerate() {
            assert_close(*x, plot.x - Y_LABEL_GUTTER, "in the left gutter");
            assert_close(*size, LABEL_FONT_SIZE, "at the label font size");
            let want = plot.y + plot.height * count_to_f32(index) / 2.0 - LABEL_FONT_SIZE / 2.0;
            assert_close(*y, want, &format!("label {index}"));
        }
        assert_close(
            labels[0].1 + LABEL_FONT_SIZE / 2.0,
            plot.y,
            "the first is at the top",
        );
        assert_close(
            labels[2].1 + LABEL_FONT_SIZE / 2.0,
            plot.y + plot.height,
            "and the last at the bottom",
        );
    }

    #[test]
    fn one_y_label_sits_at_the_plots_top_rather_than_dividing_by_zero() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        chart.y_labels.set(vec!["50".to_string()]);
        let plot = chart.plot_rect(WIDE);
        let labels = texts(&chart.paint(WIDE));
        assert_eq!(labels.len(), 1);
        assert_close(
            labels[0].1 + LABEL_FONT_SIZE / 2.0,
            plot.y,
            "the only label is at the top, where index 0 is",
        );
    }

    #[test]
    fn an_x_label_sits_under_its_own_sample_and_no_other() {
        let (_, chart) = line(&[0.0, 5.0, 10.0], 0.0, 10.0);
        chart.x_labels.set(vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ]);
        let plot = chart.plot_rect(WIDE);
        let points = chart.points(plot, chart.y_range());
        let labels = texts(&chart.paint(WIDE));
        assert_eq!(labels.len(), 3);
        for (index, (x, y, text, size)) in labels.iter().enumerate() {
            assert_close(
                *x,
                points[index].0,
                &format!("label {index} under its own sample"),
            );
            assert_eq!(text, &["first", "second", "third"][index]);
            assert_close(*size, LABEL_FONT_SIZE, "at the label font size");
            assert_close(*y, plot.y + plot.height + X_LABEL_GAP, "below the axis");
        }
    }

    #[test]
    fn fewer_x_labels_than_samples_leaves_the_rest_unlabelled() {
        let (_, chart) = line(&[0.0, 5.0, 10.0, 10.0], 0.0, 10.0);
        chart.x_labels.set(vec!["a".to_string(), "b".to_string()]);
        let labels = texts(&chart.paint(WIDE));
        assert_eq!(
            labels.len(),
            2,
            "and the samples past them are not shifted a label"
        );
        assert_eq!(labels[0].2, "a");
        assert_eq!(labels[1].2, "b");
    }

    #[test]
    fn more_x_labels_than_samples_draws_only_the_ones_with_a_sample_under_them() {
        let (_, chart) = line(&[0.0, 5.0], 0.0, 10.0);
        chart.x_labels.set(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ]);
        let labels = texts(&chart.paint(WIDE));
        assert_eq!(labels.len(), 2, "for there is nothing under the other two");
    }

    #[test]
    fn an_unlabelled_chart_draws_no_text_at_all() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        assert!(texts(&chart.paint(WIDE)).is_empty());
    }

    #[test]
    fn a_gutter_is_reserved_only_for_the_axis_that_has_labels() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        let bare = chart.plot_rect(WIDE);
        chart.x_labels.set(vec!["a".to_string(), "b".to_string()]);
        let with_x = chart.plot_rect(WIDE);
        assert_eq!(
            with_x.width, bare.width,
            "an x label does not move the left edge"
        );
        assert!(with_x.height < bare.height);
        chart.y_labels.set(vec!["0".to_string()]);
        let with_both = chart.plot_rect(WIDE);
        assert!(
            with_both.width < with_x.width,
            "and a y label narrows the plot"
        );
    }

    // ---- What reaches outside -----------------------------------------------

    #[test]
    fn a_series_spanning_its_whole_range_reaches_only_the_caps() {
        // The **cap** case, and the complement of
        // `nothing_but_the_stroke_reach_or_a_label_line_box_leaves_the_node_rect`. Every
        // shape here can
        // only ever reach the plot's **bottom** edge, and by nothing but a flat cap.
        //
        // **This test used to be named `nothing_reaches_outside_the_rect_but_the
        // _strokes_own_half_width` and to call a half width "the one documented
        // exception". That claim was retracted** 100 lines earlier in this file: a
        // mitred corner reaches `MITRE_LIMIT` times that far, four times as much at
        // the defaults. The assertion below is the claim this fixture can support —
        // with no corner anywhere in it, the reach is **one half width and not
        // `MITRE_LIMIT` of them**, which is what keeps this a second case rather
        // than a second copy of the first.
        //
        // Measured **1.4992676 px on WIDE and 1.4954529 on SQUARE** — not one
        // number for both, which an earlier version of this comment claimed. The
        // argmax on WIDE is the first cap of the ascending series, and it is
        // arithmetic rather than a capture: `WIDE` is 600 by 240, so y(-10) = 280 is
        // the plot's **bottom** edge and y(-5) = 276.25, giving a first segment of
        // (120, -3.75) whose unit normal is (0.031234, 0.999512) and whose cap is
        // 1.5x that — **(0.04685, 1.49927)**, so 1.4992676 is the vertical one.
        //
        // **Only the bottom edge is reached, and that is the whole reason.** The
        // range is `-10.0 ..= 310.0`, and `-10` is the one sample extreme in any
        // shape here that lands on an edge: on WIDE the highest sample is y(300) =
        // 47.5 against a top edge of 40, and on SQUARE it is 72.25 against 64. So
        // **nothing in this battery puts a sample on the top edge at all** — an
        // earlier version of this comment said "its ends land on the plot's top and
        // bottom edges", which is why its number was stated for the wrong reason.
        // With no corner anywhere in the fixture, the reach is one cap's half width
        // and the assertion below is about caps and nothing else.
        //
        // The labels are excluded, here and in the other battery, because they
        // overhang by their own width and `DrawCommand::Text` carries none.
        let shapes: [&[f32]; 7] = [
            &[],
            &[5.0],
            &[5.0, 5.0],
            &[0.0, 10.0, 0.0],
            &[0.0, 300.0, 1.0, 0.0, 300.0, 2.0],
            &[-10.0, -5.0, 0.0, 5.0, 10.0, 15.0],
            &[f32::NAN, 3.0, 4.0, f32::NAN, 6.0],
        ];
        let mut checked = 0;
        let mut widest = 0.0_f32;
        for chart_type in [ChartType::Line, ChartType::Bar, ChartType::Area] {
            for shape in shapes {
                for rect in [WIDE, SQUARE] {
                    let (_, chart) = showing(chart_type, shape, -10.0, 310.0);
                    let commands = chart.paint(rect);
                    for (points, _) in polygons(&commands) {
                        for point in &points {
                            assert_near_inside(rect, *point, LINE_WIDTH / 2.0);
                            let reach = outside(rect, *point);
                            widest = widest.max(reach.0).max(reach.1).max(reach.2).max(reach.3);
                            checked += 1;
                        }
                    }
                    for rect_drawn in rects(&commands) {
                        for corner in [
                            (rect_drawn.x, rect_drawn.y),
                            (rect_drawn.x + rect_drawn.width, rect_drawn.y),
                            (rect_drawn.x, rect_drawn.y + rect_drawn.height),
                            (
                                rect_drawn.x + rect_drawn.width,
                                rect_drawn.y + rect_drawn.height,
                            ),
                        ] {
                            assert_near_inside(rect, corner, 0.0);
                            checked += 1;
                        }
                    }
                    for (start, end, width) in lines(&commands) {
                        assert_near_inside(rect, start, width / 2.0);
                        assert_near_inside(rect, end, width / 2.0);
                        checked += 2;
                    }
                }
            }
        }
        assert!(
            checked > 500,
            "and it actually checked something: {checked}"
        );

        // The distinguishing claim, and it is deliberately a **ratio** rather
        // than a comparison against `stroke_reach()`: at this fixture's numbers a
        // cap reaches 1.4993 and a corner reaches 6.0, so "less than
        // `stroke_reach()`" is true at *both* bounds and proves nothing, while
        // "no more than one half width" is false the moment a corner appears.
        // Written the weak way first, this assertion caught nothing when
        // `stroke_reach()` was deliberately collapsed to `line_width / 2`.
        assert!(
            widest <= LINE_WIDTH / 2.0 * 1.01,
            "the reach is {widest}, which is one half width and not a corner's \
             MITRE_LIMIT of them"
        );
        assert!(
            widest > LINE_WIDTH / 2.0 * 0.99,
            "and it really is a full cap rather than less: {widest}"
        );
        // So the two batteries are disjoint in what they claim: this one is about
        // caps and the other is about corners, and the corner's own number is
        // pinned four times higher in
        // `the_reach_of_a_peak_at_the_high_is_the_mitre_limit_times_the_half_width`.
        let (_, chart) = showing(ChartType::Line, &[0.0, 1.0, 0.0], -10.0, 310.0);
        assert_close(
            chart.stroke_reach(),
            MITRE_LIMIT * LINE_WIDTH / 2.0,
            "a corner is still four times as far",
        );
    }

    #[test]
    fn a_chart_draws_nothing_into_a_rect_of_no_area() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        for degenerate in [
            Rect::new(700.0, 40.0, 0.0, 300.0),
            Rect::new(700.0, 40.0, 600.0, 0.0),
            Rect::new(700.0, 40.0, -5.0, 300.0),
        ] {
            assert!(
                chart.paint(degenerate).is_empty(),
                "{degenerate:?} has nowhere to draw"
            );
        }
    }

    // ---- Updating the data ---------------------------------------------------

    #[test]
    fn pushing_appends_to_the_truth_and_draws_nothing_by_itself() {
        let mut nodes = Arena::new();
        let chart = Chart::new(&mut nodes, ChartType::Line);
        chart.push(1.0);
        chart.push(2.0);
        assert_eq!(chart.data.get(), vec![1.0, 2.0]);
        assert!(
            chart.shown.get().is_empty(),
            "a push is the plain write: the drawn series waits for an aim"
        );
    }

    #[test]
    fn shifting_drops_the_oldest_and_appends_and_reports_what_it_dropped() {
        let mut nodes = Arena::new();
        let chart = Chart::new(&mut nodes, ChartType::Line);
        assert_eq!(
            chart.shift(9.0),
            None,
            "an empty series has no oldest sample"
        );
        assert_eq!(chart.data.get(), vec![9.0]);
        chart.push(8.0);
        assert_eq!(chart.shift(7.0), Some(9.0));
        assert_eq!(chart.data.get(), vec![8.0, 7.0]);
        assert_eq!(chart.data.get().len(), 2, "and the length does not change");
    }

    #[test]
    fn an_aim_writes_nothing_so_the_frame_it_happens_is_the_old_picture() {
        let (_, chart) = line(&[0.0, 10.0], 0.0, 10.0);
        let before = chart.shown.get();
        chart.data.set(vec![0.0, 10.0, 20.0]);
        chart.animate_to_state(motion());
        assert_eq!(chart.shown.get(), before, "aiming moves nothing");
        assert!(chart.is_animating());
    }

    #[test]
    fn an_append_holds_the_existing_samples_and_puts_the_new_one_at_the_far_edge() {
        // The requirement's "no jarring jumps", read off the drawn series: the
        // samples already on the plot do not re-space on the first frame, and the
        // new one arrives where the old run ended.
        let (_, chart) = line(&[1.0, 2.0], 0.0, 10.0);
        let before = chart.shown.get();
        chart.data.set(vec![1.0, 2.0, 3.0]);
        chart.animate_to_state(motion());
        assert!(tick(&chart, 1));

        let started = chart.shown.get();
        assert_eq!(started.len(), 3);
        assert_close(started.x[0], before.x[0], "the oldest has not moved");
        assert_close(
            started.x[2],
            1.0,
            "and the new one is where the old run ended",
        );
        assert!(
            started.x[1] > 0.5 && started.x[1] < before.x[1],
            "while the middle is between the two spacings: {} against {}",
            started.x[1],
            before.x[1]
        );
        assert!(
            started.values[2] > 2.0 && started.values[2] < 3.0,
            "and the new reading is on its way: {}",
            started.values[2]
        );
    }

    #[test]
    fn an_append_arrives_at_an_even_spacing_on_the_truth() {
        let (_, chart) = line(&[1.0, 2.0], 0.0, 10.0);
        chart.data.set(vec![1.0, 2.0, 3.0]);
        chart.animate_to_state(motion());
        tick(&chart, 100);
        assert_eq!(chart.shown.get().x, vec![0.0, 0.5, 1.0]);
        assert_eq!(chart.shown.get().values, vec![1.0, 2.0, 3.0]);
        assert!(!chart.is_animating());
    }

    #[test]
    fn a_shift_slides_the_series_one_slot_to_the_left() {
        // `[a,b,c,d]` becoming `[b,c,d,e]` is four samples at four fixed places
        // whose readings glide to their neighbour's — which is the scroll, and it
        // needs no geometry at all because nothing re-spaced.
        let (_, chart) = line(&[1.0, 2.0, 3.0, 4.0], 0.0, 10.0);
        let before = chart.shown.get();
        assert_eq!(chart.animate_shift(5.0, motion()), Some(1.0));
        assert_eq!(chart.shown.get(), before, "aiming writes nothing");
        tick(&chart, 50);
        let halfway = chart.shown.get();
        assert_eq!(halfway.x, before.x, "and the places have not moved at all");
        assert_eq!(halfway.values, vec![1.5, 2.5, 3.5, 4.5]);
        tick(&chart, 50);
        let arrived = chart.shown.get();
        assert_eq!(arrived.x, before.x);
        assert_eq!(
            arrived.values,
            vec![2.0, 3.0, 4.0, 5.0],
            "one slot to the left"
        );
    }

    #[test]
    fn the_glide_lands_on_the_truth_exactly_and_then_stops() {
        // Not `within an epsilon`: the last frame writes the target itself, so a
        // chart's own drawn series is bit-for-bit its data afterwards and the next
        // transition starts from a number nothing else holds.
        let (_, chart) = line(&[0.0], 0.0, 10.0);
        chart.data.set(vec![0.0, 7.0, 13.0]);
        chart.animate_to_state(motion());
        tick(&chart, 99);
        assert_ne!(chart.shown.get().values, vec![0.0, 7.0, 13.0], "not yet");
        tick(&chart, 1);
        assert_eq!(chart.shown.get().values, vec![0.0, 7.0, 13.0], "exactly");
        assert!(!chart.is_animating());
        assert!(
            !tick(&chart, 100),
            "and nothing is left running to report a repaint for"
        );
    }

    #[test]
    fn a_glide_that_would_change_nothing_is_not_started() {
        let (_, chart) = line(&[1.0, 2.0, 3.0], 0.0, 10.0);
        let before = chart.shown.get();
        chart.animate_to_state(motion());
        assert_eq!(chart.shown.get(), before, "the drawn series has not moved");
        // The colour transitions are running, so `tick` reports a write; what it
        // never writes is the series, and that is what this is about.
        tick(&chart, 50);
        assert_eq!(chart.shown.get(), before);
    }

    #[test]
    fn a_zero_length_motion_arrives_on_the_first_tick() {
        let (_, chart) = line(&[1.0, 2.0], 0.0, 10.0);
        chart.data.set(vec![1.0, 2.0, 3.0]);
        chart.animate_to_state(Motion {
            duration: Duration::ZERO,
            easing: Easing::Linear,
        });
        assert!(
            tick(&chart, 1),
            "and the tick that arrives reports that it moved"
        );
        assert_eq!(chart.shown.get().values, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn a_chart_with_nothing_to_plot_aimed_at_nothing_does_not_move() {
        // The **series** does not move. `is_animating` is true anyway, because
        // `animate_to_state` aims the colour transitions on every widget in this
        // repository and a colour already at its target is still a transition
        // that has not finished. The question here is the one a caller repainting
        // on the series' own callback is asking.
        let mut nodes = Arena::new();
        let chart = Chart::new(&mut nodes, ChartType::Line);
        chart.animate_to_state(motion());
        assert!(chart.shown.get().is_empty());
        // `tick` reports `true` here: the colour transitions are running. What it
        // never writes is the series, and `shown`'s own callback is what a caller
        // repainting on this would be listening to.
        tick(&chart, 50);
        assert!(
            chart.shown.get().is_empty(),
            "the drawn series has not moved, because there is nothing to move it"
        );
    }

    #[test]
    fn the_first_sample_of_an_otherwise_empty_chart_arrives_at_its_own_value() {
        // There is no earlier reading for it to have come from, so it starts at
        // its target rather than at a number nothing holds.
        let (_, chart) = line(&[], 0.0, 10.0);
        chart.data.set(vec![6.0]);
        chart.animate_to_state(motion());
        assert_eq!(
            chart.shown.get(),
            Series::default(),
            "aiming writes nothing"
        );
        tick(&chart, 1);
        let started = chart.shown.get();
        assert_eq!(started.len(), 1);
        assert_close(started.values[0], 6.0, "and the reading is already there");
    }

    // ---- The bars rising ----------------------------------------------------

    #[test]
    fn a_new_bar_rises_from_the_baseline() {
        let (_, chart) = bars(&[4.0], 0.0, 10.0);
        chart.snap_to_state();
        assert_eq!(rects(&chart.paint(WIDE)).len(), 1);
        assert_close(
            rects(&chart.paint(WIDE))[0].height,
            WIDE.height * 0.4,
            "the lone bar is at full height",
        );

        chart.animate_push(6.0, motion());
        tick(&chart, 50);
        let plot = chart.plot_rect(WIDE);
        let baseline = plot.y + plot.height;
        // The drawn series is re-read at every tick, because it is what the bars
        // are built from — the first draft of this captured it once and then
        // compared the second bar against the *pre-tick* reading, which moves.
        let ideal = |index: usize| {
            let points = chart.points(plot, chart.y_range());
            (baseline - points[index].1).abs()
        };
        let drawn = rects(&chart.paint(WIDE));
        assert_eq!(drawn.len(), 2, "the new sample is a bar even at no height");
        assert_close(drawn[1].height, ideal(1) * 0.5, "half way up");
        assert_close(drawn[0].height, ideal(0), "and the old one is unmoved");

        tick(&chart, 50);
        assert_close(
            rects(&chart.paint(WIDE))[1].height,
            ideal(1),
            "and the new bar arrives at its own value's height",
        );
    }

    #[test]
    fn a_bar_of_no_height_is_not_recorded_rather_than_recorded_as_nothing() {
        // At `reveal` of zero the newest bar has no height at all, so a chart
        // whose new sample has just arrived draws the old bars and not a
        // rectangle of no extent.
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Bar);
        chart.set_fixed_range(0.0, 10.0);
        chart.data.set(vec![4.0, 8.0]);
        chart.snap_to_state();
        assert_eq!(rects(&chart.paint(WIDE)).len(), 2);
        chart.reveal.set(0.0);
        assert_eq!(
            rects(&chart.paint(WIDE)).len(),
            1,
            "and the newest bar is not among them"
        );
    }

    #[test]
    fn a_bar_chart_does_not_blank_its_bars_when_only_the_readings_move() {
        // A `reveal` that eased from zero on *every* change would empty the whole
        // chart for the length of every transition. `from_reveal` is `1.0` unless
        // the series actually grew.
        let (_, chart) = bars(&[2.0, 8.0], 0.0, 10.0);
        chart.snap_to_state();
        chart.data.set(vec![4.0, 9.0]);
        chart.animate_to_state(motion());
        tick(&chart, 1);
        assert_close(
            chart.reveal.get(),
            1.0,
            "the bars are all there on the first frame",
        );
        assert_eq!(rects(&chart.paint(WIDE)).len(), 2);
    }

    #[test]
    fn a_line_chart_leaves_the_reveal_alone() {
        // Nothing outside a bar chart draws it, so nothing animates it: a
        // transition on a property nothing draws reports itself as running for
        // nothing.
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        chart.data.set(vec![1.0, 2.0]);
        chart.snap_to_state();
        chart.reveal.set(0.3);
        chart.data.set(vec![1.0, 2.0, 3.0]);
        chart.animate_to_state(motion());
        tick(&chart, 50);
        assert_eq!(chart.reveal.get(), 0.3, "untouched");
        chart.set_chart_type(ChartType::Bar);
        assert_eq!(chart.reveal.get(), 1.0, "until bars are asked for");
    }

    #[test]
    fn a_spring_cannot_make_a_bar_taller_than_its_own_value() {
        let (_, chart) = bars(&[4.0], 0.0, 10.0);
        chart.snap_to_state();
        let plot = chart.plot_rect(WIDE);
        let baseline = plot.y + plot.height;
        chart.animate_push(
            6.0,
            Motion {
                duration: ms(100),
                easing: Easing::Spring {
                    damping: 2.0,
                    stiffness: 200.0,
                },
            },
        );
        let mut worst: f32 = 0.0;
        for _ in 0..20 {
            tick(&chart, 10);
            // Re-read every tick: the drawn series moves, so the bar's own height
            // and its own reading move together and only the ratio between them is
            // a fact.
            let drawn = rects(&chart.paint(WIDE));
            let points = chart.points(plot, chart.y_range());
            let newest = drawn.len().saturating_sub(1);
            if let Some(bar) = drawn.get(newest) {
                let ideal = (baseline - points[newest].1).abs().max(0.01);
                worst = worst.max(bar.height / ideal);
            }
        }
        assert!(
            worst <= 1.001,
            "no bar is taller than its own reading: {worst} of it"
        );
        assert!(
            (0.0..=1.0).contains(&chart.reveal.get()),
            "and the reveal never leaves the range"
        );
    }

    // ---- Property plumbing --------------------------------------------------

    #[test]
    fn a_property_callback_reaches_the_caller_when_the_series_moves() {
        // The demo's own idiom, and the reason `tick` returns whether it wrote.
        let mut nodes = Arena::new();
        let chart = Chart::new(&mut nodes, ChartType::Line);
        let writes = Rc::new(Cell::new(0));
        let counting = Rc::clone(&writes);
        chart
            .shown
            .on_change(move |_| counting.set(counting.get() + 1));

        chart.data.set(vec![1.0, 2.0]);
        chart.snap_to_state();
        assert_eq!(writes.get(), 1, "the snap");

        chart.data.set(vec![1.0, 2.0, 3.0]);
        chart.animate_to_state(motion());
        tick(&chart, 50);
        assert_eq!(writes.get(), 2, "and one for the frame the series moved on");
        tick(&chart, 50);
        assert_eq!(writes.get(), 3, "and one for the frame it arrived");
        assert!(!tick(&chart, 50), "and none once it has stopped");
    }

    #[test]
    fn a_node_marked_by_a_callback_is_painted_from_the_drawn_series() {
        let mut nodes = Arena::new();
        let mut chart = Chart::new(&mut nodes, ChartType::Line);
        let handle = chart.handle();
        chart.set_fixed_range(0.0, 10.0);
        chart.data.set(vec![0.0, 10.0]);
        chart.snap_to_state();

        if let Some(node) = nodes.get_mut(handle) {
            *node.paint_mut() = PaintState::from_commands(chart.paint(WIDE));
        }
        let state = nodes.get(handle).expect("the node is in the arena").paint();
        assert!(state.is_dirty(), "and the renderer is told to draw it");
        assert_eq!(
            state.commands().len(),
            one_fewer(GRID_DIVISIONS) + 3,
            "the frame and one segment for two samples"
        );
    }

    // ---- The helpers, on their own ------------------------------------------

    #[test]
    fn a_normal_is_a_unit_vector_a_quarter_turn_from_its_direction() {
        let right = normal_of((3.0, 4.0)).expect("a direction");
        assert_close(magnitude(right), 1.0, "a normal is a unit vector");
        assert_close(
            right.1,
            right.0 * 0.0 + 3.0 / 5.0,
            "and it is a quarter turn away",
        );
        let dot = right.0 * 3.0 + right.1 * 4.0;
        assert!(dot.abs() < 1e-5, "and it is perpendicular: {dot}");
    }

    #[test]
    fn a_direction_of_no_length_has_no_normal() {
        assert_eq!(normal_of((0.0, 0.0)), None);
        assert_eq!(
            normal_of((f32::NAN, 1.0)),
            None,
            "nor has one nobody can measure"
        );
    }

    #[test]
    fn a_vertex_with_neither_neighbour_is_not_a_join() {
        assert_eq!(join_at((0.0, 0.0), None, None, 1.5), None);
    }

    #[test]
    fn a_run_that_doubles_back_exactly_has_no_join() {
        // The denominator `1 + p·q` is zero for a full reversal, and a stroke
        // across no direction has no width; the widget skips that segment rather
        // than reaching for an unwrap.
        let corner = join_at((10.0, 0.0), Some((0.0, 0.0)), Some((0.0, 0.0)), 1.5);
        assert_eq!(corner, None);
    }

    #[test]
    fn a_mitre_is_half_on_a_straight_run_and_sqrt_two_half_at_a_right_angle() {
        let half = 1.5;
        let straight = join_at((0.0, 0.0), None, Some((10.0, 0.0)), half).expect("a join");
        assert_eq!(
            straight,
            Join::Corner((0.0, half)),
            "a flat cap of the half width"
        );

        let turn = join_at((10.0, 0.0), Some((0.0, 0.0)), Some((10.0, -10.0)), half);
        assert!(
            matches!(turn, Some(Join::Corner(_))),
            "a right angle is inside the limit, so it is still a corner"
        );
        let corner = turn.expect("a join").incoming();
        assert_close(magnitude(corner), half * 2.0_f32.sqrt(), "half · √2");
    }

    #[test]
    fn a_turn_over_the_limit_gives_each_side_its_own_perpendicular() {
        let join = join_at((0.0, 0.0), Some((-1.0, -10.0)), Some((1.0, -10.0)), 1.5);
        let Join::Flat { incoming, outgoing } = join.expect("a join") else {
            panic!("a 168-degree turn is past the mitre limit, so it is flat");
        };
        assert_close(magnitude(incoming), 1.5, "the arriving side's offset");
        assert_close(magnitude(outgoing), 1.5, "and the leaving side's");
        assert!(
            magnitude((incoming.0 - outgoing.0, incoming.1 - outgoing.1,)) > 1.0,
            "and they are not the same vector, which is the whole of the fix"
        );
    }

    #[test]
    fn a_mitre_corner_sits_half_from_the_centre_line_on_both_sides_of_a_turn() {
        // The invariant the convexity proof rests on, asserted directly.
        let points = [(0.0, 0.0), (10.0, 4.0), (20.0, -6.0), (30.0, 0.0)];
        let half = 1.5;
        for index in 0..3 {
            let a = points[index];
            let b = points[index + 1];
            let normal = normal_of((b.0 - a.0, b.1 - a.1)).expect("a direction");
            let join = join_at(a, index.checked_sub(1).map(|i| points[i]), Some(b), half);
            let (Some(here), Some(there)) = (
                join,
                join_at(b, Some(a), points.get(index + 2).copied(), half),
            ) else {
                continue;
            };
            for offset in [here.outgoing(), there.incoming()] {
                let dot = offset.0 * normal.0 + offset.1 * normal.1;
                assert_close(
                    dot,
                    half,
                    &format!("segment {index}'s offset is `half` from its centre line"),
                );
            }
        }
    }

    #[test]
    fn bounded_passes_over_a_nan_rather_than_propagating_it() {
        assert_eq!(bounded(f32::NAN, 0.0, 1.0), 0.0);
        assert_eq!(bounded(5.0, 0.0, 1.0), 1.0);
        assert_eq!(bounded(-5.0, 0.0, 1.0), 0.0);
        assert_eq!(bounded(0.5, 0.0, 1.0), 0.5);
    }

    #[test]
    fn ordered_orders_a_pair_even_when_a_bound_is_a_nan() {
        assert_eq!(ordered(2.0, 1.0), (1.0, 2.0));
        assert_eq!(ordered(f32::NAN, 1.0), (1.0, 1.0));
        assert_eq!(ordered(1.0, f32::NAN), (1.0, 1.0));
    }

    #[test]
    fn spanned_is_the_question_the_widget_asks_before_it_divides() {
        assert!(spanned(0.0, 1.0));
        assert!(!spanned(1.0, 1.0), "a range of one value has no room in it");
        assert!(!spanned(1.0, 0.0));
        assert!(
            !spanned(f32::NAN, f32::NAN),
            "nor has one nobody can measure"
        );
        assert!(!spanned(f32::NAN, 1.0));
        assert!(!spanned(0.0, f32::NAN));
    }

    #[test]
    fn count_to_f32_is_exact_for_the_counts_this_module_divides_by() {
        assert_eq!(count_to_f32(0), 0.0);
        assert_eq!(count_to_f32(1), 1.0);
        assert_eq!(count_to_f32(5), 5.0);
        assert_eq!(count_to_f32(900), 900.0);
    }

    #[test]
    fn one_fewer_never_goes_below_zero() {
        assert_eq!(one_fewer(0), 0);
        assert_eq!(one_fewer(1), 0, "one sample has no neighbours");
        assert_eq!(one_fewer(4), 3);
    }

    #[test]
    fn magnitude_answers_false_for_a_nan_length() {
        assert_eq!(magnitude((3.0, 4.0)), 5.0);
        assert!(
            !positive(magnitude((f32::NAN, 0.0))),
            "so the predicate that reads it is false rather than a panic"
        );
    }
}
