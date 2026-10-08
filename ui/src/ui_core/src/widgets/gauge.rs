//! The Gauge widget: a value shown as an arc swept around a dial.
//!
//! A gauge is a node, a truth, a drawn value, and a band of filled quads.
//! [`Gauge::value`] is the truth — a reading in the gauge's own units, a speed
//! or a temperature — and [`Gauge::shown`] is what is *drawn*, animated toward
//! the truth, and the distance between the two is the transition. That split is
//! [`slider`](crate::widgets::slider::Slider)'s and
//! [`progress`](crate::widgets::progress::Progress)'s, for the reason both give:
//! a caller writing `value` from a sensor wants the needle to *arrive* at the
//! reading rather than jump to it, and requirement 4's "fill animates when value
//! changes" is the drawn half of that pair.
//!
//! Three types, one widget. [`GaugeType::Arc`] is the partial circle a
//! speedometer is — the default, 270 degrees with a quarter left open at the
//! bottom. [`GaugeType::Circle`] is full-circle progress, and it is a **full
//! turn whatever the two angles say**: the sweep is 360 degrees and only
//! [`start_angle`](Gauge::start_angle) matters, as the place the seam falls.
//! [`GaugeType::Needle`] is the arc plus a pointer across it.
//!
//! # The arc is a band of quads, and both rejected alternatives are measured
//!
//! Requirement 5 asks for "a thick line (triangle strip or quad strip)". Neither
//! of those is reachable without a new vertex type, a new shader and a change to
//! the index buffer, so the band is drawn as **one convex four-point
//! [`DrawCommand::Polygon`] per segment**, with corners on the inner and outer
//! radii at the segment's two endpoint angles: `outer = R + thickness/2` and
//! `inner = R - thickness/2`. Four points is the same case three is — the
//! renderer's fan is exact for a convex polygon — and the winding is the same way
//! round on every segment, which is the fan's precondition and is asserted rather
//! than assumed.
//!
//! The two answers this module gave first were both wrong, and one of them was
//! written down as a fact, so the arithmetic behind all three is here.
//!
//! **A chain of overlapping circles — what this module drew first — is visibly
//! beaded.** Circles tangent on their **centre lines** have outer edges that
//! touch only where `R >> r`, and at this gauge's defaults `r/R` is 0.075, which
//! is nowhere near that. A capture of the running demo measured the outer edge at
//! **100.0 px at every circle centre and 94.4–95.8 px at every bisector** — a
//! **5.6 px scallop on a 14 px band** — and the geometry agrees: at the widest
//! step two tangent circles of radius 7 on a 93-pixel radius leave
//! `100 − 93·cos(step/2)` = **7.26 px** missing, and spread evenly over 32 steps
//! of 270° it is **5.77 px**, which is 41% of the thickness. The old module doc
//! said the opposite — *"no notch on the outside of the curve, because every
//! circle is round"* — and that was true of each circle and false of their
//! union: the notch is in the *gap* between two round things, not in either of
//! them. **What would reverse this choice** is a measurement that the scallop is
//! below a tenth of a pixel at a radius the operator actually ships, which at
//! these defaults is 45 times further away than the band is thick.
//!
//! **A [`DrawCommand::Path`] is worse, and for a different reason.** `render.rs`'s
//! `line_quad` offsets each segment **perpendicular** to it, so a segment of the
//! arc has its outer corner at `sqrt(R² + r²)` = 93.3 rather than at `R + r` =
//! 100: a band of the requested 14 pixels comes out **6.7 px thin everywhere**,
//! before any scalloping is asked about. The tick marks are `Path`-shaped and
//! do not suffer it, because a tick is *radial* and the offset is therefore
//! across the tick rather than along it — the one primitive in this widget where
//! a perpendicular offset costs nothing.
//!
//! **What is left is the chord, and it is second order in the step.** The
//! straight edge between two consecutive outer corners sags by
//! `(R + r)·(1 − cos(step/2))` below the arc it stands in for: **0.255 px** on the
//! outer edge and **0.219 px** inside at the defaults, against the chain's 5.77 px
//! and the `Path`'s 6.7 px of missing thickness. Being second order is what makes
//! a coarse tessellation safe rather than a tuning question: halving the segment
//! count quadruples the error, doubling it quarters the error, and
//! `the_sagitta_grows_with_the_square_of_the_step` is that law asserted.
//!
//! **The cost.** 33 quads for the 270-degree default, where the chain spent 33
//! circles on the track and as many again on a full fill — 66 quads and 132
//! triangles against 33 quads and 66. It is one batch either way: the batcher
//! groups by colour, texture and blend mode, not by primitive, so the cost is
//! vertex throughput and not draw calls. **What would reverse the whole choice**
//! is a real stroked-arc primitive — a join rule, a cap, and a vertex type that
//! carries a direction — which would draw the band in one pass with no chord in
//! it at all.
//!
//! The needle is a filled [`DrawCommand::Polygon`] — a triangle, and three points
//! is the case the renderer's convex fan is exact for. A
//! [`DrawCommand::Line`] was the alternative and the operator declined it on
//! 2026-10-01: a line is a bar with two flat ends, and a pointer with a blunt
//! end reads as a dash rather than as a needle. A small `Circle`
//! at the hub covers the triangle's base, the same trick the slider uses for a
//! thumb's border.
//!
//! # There is no anti-aliasing, and this widget does not have any
//! **(superseded 2026-10-02 — MSAA arrived; the title is kept so the section is
//! findable, and read with the paragraph under it)**
//!
//! **The central claim below is now false, and the section is kept because it
//! records what was measured and why.** The GL context now asks for **4x MSAA on
//! the default framebuffer**
//! (`render::context`'s `MULTISAMPLE_SAMPLES` and `MULTISAMPLE_BUFFERS`, set
//! before the window is built), the driver granted it (`GL_SAMPLE_BUFFERS` 1,
//! `GL_SAMPLES` 4, read back from a live context), and every geometric edge the
//! renderer draws is now resolved by the hardware rather than one whole pixel at
//! a time. **What is superseded** is the first paragraph's *"Neither mechanism
//! exists in this renderer today"* and its *"Requirement 5's 'anti-aliased edges
//! via SDF or MSAA' is not met"*; **what is still true** is the account of what
//! this module records and which of it reached the shader's branch, and of the
//! *other* two mechanisms the original list ended with, which are still not
//! here.
//!
//! **Requirement 5's "anti-aliased edges via SDF or MSAA" was not met when this
//! was written, and the paragraph below is kept as the record of that.** Neither
//! mechanism existed in this renderer then, and the three facts below are why.
//! The solid fragment shader
//! had exactly one antialiasing branch and it is a **hard** one — `if (dist >
//! 0.0) { discard; }`, a binary test with no `smoothstep` and no `fwidth`
//! (`render.rs`, `FRAGMENT_SHADER_SRC`) — and what it models is an axis-aligned
//! rounded rectangle, so [`DrawCommand::Circle`] reaches it only because
//! `command_quads` expands a circle to a rounded rect of twice its radius, and
//! even then the edge is still the pixel grid. **Nothing this widget records
//! reaches that branch at all any more**: the band and the needle are
//! [`DrawCommand::Polygon`]s and the tick marks are [`DrawCommand::Line`]s, and
//! `line_quad` and `polygon_quad` both hardcode `radius: 0.0`, so every one of
//! them takes the shader's plain-colour path and is a quad of hard pixels. The
//! band used to be circles, and so did reach the branch: trading the branch for
//! a band whose corners are on the radii cost the arc its own edges'
//! antialiasing — which, on a branch that is a hard `discard` on a rounded
//! rectangle, was worth very little to begin with.
//!
//! What that cost, on a gauge specifically, and what it now costs: the arc's
//! inner and outer boundaries were stair-stepped rather than smooth, which shows
//! most where the tangent is vertical — the left and right of the dial — and
//! least along the top, where the curve is shallow. The needle's two long edges
//! showed it too. **Measured on 2026-10-02, before and after**: the arc's outer
//! edge on the horizontal through the dial's centre read `18 18 18 18 18 187 187`
//! before — a step with nothing between it, the same shape a text stem edge
//! shows — and `18 18 18 18 145 187 187` after, the 145
//! being a pixel three of four samples covered. Over the 106 rows of the arc
//! that carry an outer edge, 103 were full coverage and **3** were intermediate
//! before; after, **88 of 108** are intermediate, at 1/4, 1/2 and 3/4 coverage.
//! Nothing about the *shape* changed: the band's 33 quads, its radii and its
//! chord sag are the same numbers, and the `Path` and the chain of circles are
//! still worse for the reasons measured above. **What 4x does not reach is an
//! edge that lands on a pixel boundary**, which resolves to full coverage on one
//! side and none on the other: the fill's radial cut at twelve o'clock sits on
//! `x = 764` and is still a hard step, in the capture, on purpose.
//!
//! The other two mechanisms the original list ended with — a shader `smoothstep`
//! and a per-primitive distance field — are still not here, and **what would
//! reverse, or replace, the antialiasing that arrived** is in the order of how
//! little it costs: a `smoothstep` over `fwidth(dist)` in the solid fragment
//! shader, which antialiases every rounded rectangle and every circle in the
//! library at once and is free, but reaches none of the band or the marks that
//! this widget actually draws; a distance field per primitive edge, a new shader
//! and a new vertex type, which reaches them and would supersede the
//! framebuffer's samples rather than add to them; and 8 samples on the same
//! default framebuffer, which this driver's `GL_MAX_SAMPLES` reports as
//! available (16) and which nobody has measured, because
//! `MULTISAMPLE_SAMPLES`'s own doc says what it would cost.
//!
//! # Colours, and what is not a theme token
//!
//! The colours follow the progress bar's and the slider's rule rather than adding
//! theme tokens: a [`Palette`] names the four a gauge draws with, the properties
//! hold them so a theme switch can be animated into them, and
//! [`snap_to_state`](Gauge::snap_to_state) puts a themed gauge on its theme at
//! once. The *sizing* — how thick the arc is, how big the dial asks to be, how
//! many ticks there are, how long the needle is — is named constants rather than
//! tokens, for the reason `progress.rs`'s own module documents at length: the
//! theme has no token for a gauge's parts, and adding one would change
//! [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables, the
//! token count and the transition every token takes part in during a switch, for
//! values a switch does not change. Each constant below says what would reverse
//! that.
//!
//! Requirement 4's "animation duration from theme tokens" is satisfied the way
//! every other widget satisfies it: the caller hands a [`Motion`], and
//! [`Motion::from_theme`] reads the theme's `DurationFast` and `EasingStandard`.
//! The widget picks no duration of its own.
//!
//! # There is no `on_event`, and nothing here can be touched
//!
//! Requirement 4 lists no gesture for a gauge, and a gauge is a display rather
//! than a control: there is no value a drag would set, no action a tap would
//! report, and a caller driving one writes [`value`](Gauge::value). **The tick
//! marks and the needle are decoration.** They are drawn in the gauge's own
//! colours and at its own inset, they are not a thumb, and **nothing in this
//! module responds to a pointer over them**: a finger that lands on the needle
//! goes to whatever is behind the gauge. A caller that wants a gauge to be
//! grabbable has to build that gesture itself, and should not assume that the
//! existence of a needle implies one — a drawn control with nothing behind it is
//! what happens when that assumption is made in the other direction.
//!
//! # Angles are degrees, clockwise, y down
//!
//! [`start_angle`](Gauge::start_angle) and [`end_angle`](Gauge::end_angle) are
//! degrees measured from the positive x axis and increasing **clockwise on the
//! screen**, because the screen's y axis points down. Zero is three o'clock, 90
//! is six o'clock, 180 is nine, 270 is twelve. The default pair, 135 and 45, is
//! the speedometer: the arc runs from seven-thirty through nine, twelve and
//! three to half-past-four, leaving the bottom quarter open.
//!
//! # Examples
//!
//! The truth moves at once and the drawn value follows it, which is the whole of
//! requirement 4's first half:
//!
//! ```
//! use std::time::Duration;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::theme::Theme;
//! use ui_core::widgets::button::Motion;
//! use ui_core::widgets::gauge::{Palette, Gauge};
//!
//! let mut nodes = Arena::new();
//! let mut gauge = Gauge::new(&mut nodes, 0.0, 240.0);
//! gauge.set_palette(Palette::from_theme(&Theme::dark()));
//! gauge.snap_to_state();
//!
//! gauge.value.set(120.0);
//! gauge.animate_to_state(Motion::from_theme(&Theme::dark()));
//! assert_eq!(gauge.shown.get(), 0.0, "the needle is where it was, not at 120");
//! for _ in 0..30 {
//!     gauge.tick(Duration::from_millis(10));
//! }
//! assert_eq!(gauge.shown.get(), 120.0, "and it arrives after the fast duration");
//! ```
//!
//! A needle gauge with tick marks, painted off the origin, and a caller's own
//! spring for the needle's motion:
//!
//! ```
//! use std::time::Duration;
//! use ui_core::animation::Easing;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::{DrawCommand, Rect};
//! use ui_core::widgets::button::Motion;
//! use ui_core::widgets::gauge::{Gauge, GaugeType};
//!
//! let mut nodes = Arena::new();
//! let mut gauge = Gauge::new(&mut nodes, 0.0, 240.0);
//! gauge.set_gauge_type(GaugeType::Needle);
//! gauge.value.set(60.0);
//! gauge.snap_to_state();
//!
//! // The spring is the caller's: the widget never picks a curve.
//! gauge.animate_to_state(Motion {
//!     duration: Duration::from_millis(300),
//!     easing: Easing::Spring { damping: 8.0, stiffness: 120.0 },
//! });
//!
//! let rect = Rect::new(700.0, 40.0, 220.0, 220.0);
//! let commands = gauge.paint(rect);
//! assert!(
//!     commands.iter().any(|command| matches!(command, DrawCommand::Polygon { .. })),
//!     "a needle gauge draws its pointer as a filled triangle"
//! );
//! ```

use std::cell::RefCell;
use std::time::Duration;

use crate::animation::AnimationClock;
use crate::arena::{Arena, Handle};
use crate::layout::Size;
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::button::Motion;

/// The side of the square a gauge asks for when its caller gives it no size of
/// its own.
///
/// A gauge has no content to measure, so this is the one number that decides how
/// big it is by default, and it is 200 rather than the slider's 240 because the
/// useful part of a gauge is its *diameter*: the arc's circumference, and so the
/// length of the fill at a given value, grows with this number rather than with
/// its side. It is a constant rather than a theme token for the reason the module
/// documents. What would reverse it is a caller that wants its dials in one
/// family with its bars, which is [`slider::DEFAULT_LENGTH`](crate::widgets::slider).
const DEFAULT_SIZE: f32 = 200.0;

/// How thick the arc is, in pixels: the band's full width, from its inner edge to
/// its outer one.
///
/// Fourteen on a 200-pixel dial is about a car's instrument. It used to be the
/// diameter of each circle in a chain, and so it decided the chain's length — a
/// thinner arc needed more circles to cover the same sweep, and
/// `a_thicker_arc_is_a_shorter_chain` said so. **It decides nothing about the
/// tessellation now**: a chord is a chord, and the band is cut into
/// [`MAX_DEGREES_PER_SEGMENT`]-wide pieces whatever the width of the thing being
/// cut. So the cost this constant used to carry has moved to that one, where it
/// can be read next to the number of pixels of error it buys. Halving the
/// thickness halves the sagitta; doubling it doubles it, and neither changes the
/// count. What would reverse it is a theme token for it, on the condition every
/// constant here names.
const ARC_THICKNESS: f32 = 14.0;

/// A full turn in degrees.
///
/// The one number the arc's sweep is measured in, and the value
/// [`GaugeType::Circle`] uses for its sweep whatever the two angles say. It is a
/// constant rather than a literal repeated at each use because "a turn" is
/// named once and the three places it appears must not drift apart.
const FULL_TURN: f32 = 360.0;

/// Where an arc starts, in degrees: seven-thirty on a clock face.
///
/// The bottom-left of the default dial, and the first of the two ends of the
/// 270-degree sweep that is a speedometer's shape. It is a constant because the
/// gauge's whole default look is one number — a 270-degree arc is only a
/// speedometer if it opens at the bottom — and a caller changing it changes the
/// face of the dial rather than one of its parts. What would reverse it is a
/// caller who wants every gauge on screen to open at the same place and be able
/// to say so without hard-coding 135.
const DEFAULT_START_ANGLE: f32 = 135.0;

/// Where an arc ends, in degrees: half-past-four, 270 degrees after the start.
///
/// Written as 45 rather than 405 because it is where the pointer sits on the
/// dial, not where the sweep stops counting: the sweep is
/// `end - start` taken **clockwise and modulo a turn**, so 405 and 45 describe
/// the same arc and 45 is the one a reader recognises.
const DEFAULT_END_ANGLE: f32 = 45.0;

/// How many tick marks a gauge draws unless a caller says otherwise.
///
/// Eleven, which is ten intervals with both ends drawn, because a dial whose
/// extreme values have no mark on them cannot be read at its own extremes — the
/// tick at `0.0` is the only thing that says which way the numbers run. What
/// would reverse it is a caller with a specific scale in mind, which is
/// [`Gauge::set_ticks`].
const DEFAULT_TICKS: u8 = 11;

/// How long a tick mark is, in pixels, measured inward from the track's inner
/// edge.
///
/// Eight on a fourteen-pixel arc: long enough to see at a glance, short enough
/// that eleven of them still read as marks rather than as a second ring. It is a
/// constant for the reason [`ARC_THICKNESS`] is. What would reverse it is the
/// same thing — a density setting in the theme, which would take the tick length
/// and [`ARC_THICKNESS`] together.
const TICK_LENGTH: f32 = 8.0;

/// How wide a tick mark is, in pixels.
///
/// Two, which is enough to be visible and thin enough that eleven of them do not
/// read as a dashed ring. It is a constant for the reason [`ARC_THICKNESS`] is.
const TICK_THICKNESS: f32 = 2.0;

/// How far the needle reaches, as a fraction of the dial's radius.
///
/// Four fifths, so the tip stops short of the track it sweeps across rather than
/// touching it. A needle that reached the track would sit on top of the fill for
/// the whole of its length and hide the value the arc is showing, which is the
/// one thing the arc is for. It is a constant for the reason [`ARC_THICKNESS`]
/// is; what would reverse it is a dial whose track is drawn at a much smaller
/// radius than its needle.
const NEEDLE_LENGTH_FRACTION: f32 = 0.8;

/// How wide the needle is at its base, as a fraction of the dial's radius.
///
/// An eighth, so the triangle is about a sixth as wide as it is long — a pointer
/// and not a wedge. It is a constant for the reason [`ARC_THICKNESS`] is.
const NEEDLE_BASE_FRACTION: f32 = 0.125;

/// The radius of the circle drawn at the needle's hub, in pixels.
///
/// Six, which is a little under half the arc's thickness and so reads as a pin
/// rather than as a second, smaller arc. It exists because
/// [`DrawCommand::Polygon`] draws a triangle with three visible edges and a
/// needle whose base is visible in the middle of a dial looks like a triangle
/// rather than a pointer: the hub covers the base, exactly as the slider's thumb
/// border covers a thumb's own edge. It is a constant for the reason
/// [`ARC_THICKNESS`] is.
const NEEDLE_HUB_RADIUS: f32 = 6.0;

/// The widest step a band segment may span, in degrees.
///
/// **The one number the tessellation is built from, and it is a number of degrees
/// rather than of pixels because that is what the error is made of.** A segment's
/// outer edge is a chord, and a chord falls short of the arc it stands in for by
/// `(R + thickness/2)·(1 − cos(step/2))` — a sagitta, second order in the step,
/// and a *pixel* error that scales with the size of the dial. So the constant
/// answers a question about angles, and what it costs is read back in pixels at
/// whatever radius the caller gave.
///
/// Eight and a quarter degrees is 33 segments for the 270-degree default, where
/// the sagitta is **0.255 px on the outer edge and 0.219 px inside** — about a
/// fifth of a pixel, a fortieth of the band it stands in, and 29x less than the
/// 7.26 px of scallop the chain of circles it replaced had. It is also a little
/// *finer* than the chain's own widest step, `2·asin(7/93)` = 8.633°, so at the
/// defaults the band is drawn with the same 33 pieces and is an order of
/// magnitude smoother than what those 33 pieces used to be.
///
/// What a different number costs, in both directions, from the same closed form:
/// **10°** is 27 segments and 0.38 px, **5°** is 54 and 0.079 px, and **20°** is
/// 14 segments and 1.5 px — visible as a faceted ring on a 100-pixel dial, which
/// is the first number here a reader should be unwilling to accept. The cost is
/// one quad per segment, so a full turn goes from 33 to 44 quads as this constant
/// goes from 8.25 to 6, and each of those fans to two triangles.
const MAX_DEGREES_PER_SEGMENT: f32 = 8.25;

/// The fewest segments any arc is drawn as.
///
/// A full turn wants [`MAX_DEGREES_PER_SEGMENT`]'s 44 on its own, so this is not
/// what keeps a circle looking like one; it is the other end of the range, and it
/// exists so that a sweep too short to divide is a band rather than a division
/// with no segments in it. A one-degree arc is eight slivers of an eighth of a
/// degree each, whose sagitta is six hundredths of a thousandth of a pixel — it
/// costs eight quads where one would do and is the right way round, because the
/// alternative is asking the caller to know that a hairline arc is a special
/// case.
///
/// The floor can only ever make the tessellation **finer**, never coarser, and
/// that is the reason 8 is safe rather than arbitrary: it binds for a sweep
/// below `8 × 8.25` = 66°, and a sweep below that is a step below 8.25° however
/// it is divided, so the band's error is bounded by [`MAX_DEGREES_PER_SEGMENT`]
/// either way.
const MIN_ARC_SEGMENTS: u16 = 8;

/// The shape a gauge draws itself in.
///
/// A mode and not an appearance, and behind a setter rather than a
/// `Property<GaugeType>` for the reason `Progress::indeterminate` gives: nothing
/// animates *which* shape a gauge is in, so a property here would be one whose
/// only writes are a caller's and which could change on its own in the middle of
/// a frame, leaving the mode disagreeing with the properties actually drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GaugeType {
    /// A partial circle: the sweep from [`start_angle`](Gauge::start_angle) to
    /// [`end_angle`](Gauge::end_angle), clockwise. The default.
    #[default]
    Arc,
    /// A full turn, whatever the two angles say: full-circle progress. Only
    /// [`start_angle`](Gauge::start_angle) is read, as the place the seam falls.
    Circle,
    /// The arc, plus a pointer at the drawn value across it.
    Needle,
}

/// The colours a gauge draws with.
///
/// Four: the track, the fill, the tick marks and the needle. They are not tokens
/// of their own — the theme has none per part, and adding one per part would put
/// four more tokens in every theme table and in every theme switch — so a gauge
/// is themed with the theme's own four and [`Palette::from_theme`] says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The track: the whole range, unfilled.
    pub track: Color,
    /// The fill: the part of the range the value has passed.
    pub fill: Color,
    /// The tick marks, when there are any.
    pub tick: Color,
    /// The needle, in [`GaugeType::Needle`].
    pub needle: Color,
}

impl Default for Palette {
    /// Returns a neutral grey dial: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    ///
    /// The two greys are the progress bar's, so a gauge and a bar look like one
    /// family before either is themed.
    fn default() -> Self {
        Palette {
            track: Color::new(64, 64, 64, 255),
            fill: Color::new(160, 160, 160, 255),
            tick: Color::new(110, 110, 110, 255),
            needle: Color::new(235, 235, 235, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The track is [`Border`](crate::theme::ThemeToken::Border), for the reason
    /// the progress bar's track is: it is the theme's hairline colour and the
    /// only one of its nine that is muted by definition, and a track is the part
    /// of the gauge that is not the value. The fill is
    /// [`Primary`](crate::theme::ThemeToken::Primary), because a filled arc *is*
    /// the value.
    ///
    /// The tick marks are [`TextMuted`](crate::theme::ThemeToken::TextMuted),
    /// which is what that token is for — present but not emphasised — and which
    /// also keeps them off the fill's own colour, so a mark at the value's own
    /// angle stays visible against the fill rather than vanishing into it.
    ///
    /// The needle is [`Text`](crate::theme::ThemeToken::Text), and not
    /// `Primary`: the needle is drawn *over* the fill, and `Text` is the colour
    /// this repository uses for anything that has to be legible against a surface
    /// that may be either the muted track or the primary fill.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::theme::{Theme, ThemeToken};
    /// use ui_core::widgets::gauge::Palette;
    ///
    /// let theme = Theme::dark();
    /// let palette = Palette::from_theme(&theme);
    /// let color = |token| theme.get(token).as_color().unwrap();
    /// assert_eq!(palette.track, color(ThemeToken::Border));
    /// assert_eq!(palette.fill, color(ThemeToken::Primary));
    /// assert_eq!(palette.tick, color(ThemeToken::TextMuted));
    /// assert_eq!(palette.needle, color(ThemeToken::Text));
    /// ```
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            track: token_color(theme, crate::theme::ThemeToken::Border),
            fill: token_color(theme, crate::theme::ThemeToken::Primary),
            tick: token_color(theme, crate::theme::ThemeToken::TextMuted),
            needle: token_color(theme, crate::theme::ThemeToken::Text),
        }
    }
}

/// The appearance the gauge's value and its palette imply.
///
/// Every field is a target, not a value in flight:
/// [`animate_to_state`](Gauge::animate_to_state) animates the gauge's properties
/// toward this and [`paint`](Gauge::paint) draws whatever the properties have
/// reached, which is a [`Style`] part way through on a frame where something is
/// moving.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The value the gauge is *drawn* at: [`value`](Gauge::value), clamped to
    /// `min..=max`. In the gauge's own units, like the property.
    pub shown: f32,
    /// The colour of the track.
    pub track: Color,
    /// The colour of the fill.
    pub fill: Color,
    /// The colour of the tick marks.
    pub tick: Color,
    /// The colour of the needle.
    pub needle: Color,
}

/// A gauge: a value shown as an arc swept around a dial.
///
/// The widget holds the property the task gives it — [`value`] — the property
/// that is drawn rather than held ([`shown`]), and the four colour properties a
/// theme switch moves ([`track`], [`fill`], [`tick`], [`needle`]) — and the six
/// plain fields that define the shape: [`min`] and [`max`] through
/// [`set_range`](Gauge::set_range), [`start_angle`] and [`end_angle`] through
/// [`set_angles`](Gauge::set_angles), [`gauge_type`] through
/// [`set_gauge_type`](Gauge::set_gauge_type), and the sizing through
/// [`set_thickness`](Gauge::set_thickness) and [`set_ticks`](Gauge::set_ticks).
///
/// The shape is plain fields rather than properties for the reason
/// [`GaugeType`] documents: nothing animates a gauge's axis, its range or its
/// mode, and a property the caller could write directly would let the mode and
/// the drawn value disagree, which is what the setters exist to prevent.
///
/// The node is the caller's to keep clean, and its size is the caller's to give
/// through [`layout_mut`](crate::node::WidgetNode::layout_mut) —
/// [`Gauge::size`] is a suggestion for a caller who has nothing else to go on.
/// A gauge draws inside whatever rect it is given: a dial centred in it, an arc
/// swept from [`start_angle`](Gauge::start_angle), a fill drawn over the same
/// arc as far as the drawn value has reached.
///
/// [`value`]: Gauge::value
/// [`shown`]: Gauge::shown
/// [`track`]: Gauge::track
/// [`fill`]: Gauge::fill
/// [`tick`]: Gauge::tick
/// [`needle`]: Gauge::needle
/// [`min`]: Gauge::min
/// [`max`]: Gauge::max
/// [`start_angle`]: Gauge::start_angle
/// [`end_angle`]: Gauge::end_angle
/// [`gauge_type`]: Gauge::gauge_type
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::gauge::Gauge;
///
/// let mut nodes = Arena::new();
/// let mut gauge = Gauge::new(&mut nodes, 0.0, 100.0);
/// gauge.value.set(50.0);
/// gauge.snap_to_state();
///
/// // The caller owns the value; the gauge draws it and does not otherwise touch it.
/// let rect = Rect::new(32.0, 64.0, 200.0, 200.0);
/// assert!(
///     gauge.paint(rect).len() > 1,
///     "a track, tick marks, and a fill of the drawn value's share of the arc"
/// );
/// ```
pub struct Gauge {
    /// The truth: what the gauge is reading, in its own units.
    ///
    /// The caller may write it — a speed, a temperature — and the widget never
    /// writes it: nothing a gauge receives is a *new value*, only a new mode, a
    /// new shape or a new colour. A value outside `min..=max` is not an error
    /// and is not wrapped; see [`Gauge::fraction`] and [`Gauge::paint`].
    pub value: Property<f32>,
    /// The value the gauge is *drawn* at, animated toward [`value`](Gauge::value).
    ///
    /// Requirement 4's "fill animates when value changes" is this property, and
    /// it is what the needle points at rather than what the truth says: a needle
    /// that jumped to the new value a frame before the arc arrived would be a
    /// needle and an arc disagreeing about the same reading. Writing this one
    /// directly overrides the animation until the next aim.
    pub shown: Property<f32>,
    /// The colour of the track: the whole range, unfilled.
    pub track: Property<Color>,
    /// The colour of the fill: the part of the range the value has passed.
    pub fill: Property<Color>,
    /// The colour of the tick marks, when there are any.
    pub tick: Property<Color>,
    /// The colour of the needle, in [`GaugeType::Needle`].
    pub needle: Property<Color>,
    min: f32,
    max: f32,
    start_angle: f32,
    end_angle: f32,
    gauge_type: GaugeType,
    thickness: f32,
    ticks: u8,
    palette: Palette,
    clock: RefCell<AnimationClock>,
    node: Handle,
}

impl Gauge {
    /// Creates a gauge over `min..=max` in the arena, and returns it.
    ///
    /// The range is ordered rather than taken as given, so a caller that passes
    /// the two the wrong way round gets a gauge rather than a division by a
    /// negative span. The value starts at the minimum and the drawn value with
    /// it, the gauge is an [`GaugeType::Arc`] opening at
    /// `DEFAULT_START_ANGLE` with `DEFAULT_TICKS` marks on it, and the
    /// colours are the neutral defaults until a caller gives it a
    /// [`Palette`](Gauge::set_palette) and calls
    /// [`snap_to_state`](Gauge::snap_to_state) or
    /// [`animate_to_state`](Gauge::animate_to_state).
    ///
    /// The task file's `Gauge::new(min, max) -> Handle` is read as this: the
    /// handle is [`Gauge::handle`]'s, and returning it alone would leave a caller
    /// with no property to write and no way to draw the gauge. Task 12's
    /// [`Button::new`](crate::widgets::button::Button::new), task 16's
    /// [`Slider::new`](crate::widgets::slider::Slider::new) and task 18's
    /// [`Progress::new`](crate::widgets::progress::Progress::new) settled the same
    /// reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// assert!(nodes.get(gauge.handle()).is_some(), "its node is in the arena");
    /// assert_eq!(gauge.value.get(), 0.0, "and it starts at the minimum");
    /// assert_eq!((gauge.min(), gauge.max()), (0.0, 240.0));
    /// ```
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, min: f32, max: f32) -> Self {
        let palette = Palette::default();
        let (min, max) = ordered(min, max);
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Gauge {
            value: Property::new(min),
            shown: Property::new(min),
            track: Property::new(palette.track),
            fill: Property::new(palette.fill),
            tick: Property::new(palette.tick),
            needle: Property::new(palette.needle),
            min,
            max,
            start_angle: DEFAULT_START_ANGLE,
            end_angle: DEFAULT_END_ANGLE,
            gauge_type: GaugeType::default(),
            thickness: ARC_THICKNESS,
            ticks: DEFAULT_TICKS,
            palette,
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the gauge's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the gauge draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the gauge draws with, and leaves the current ones where
    /// they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](Gauge::animate_to_state) or
    /// [`snap_to_state`](Gauge::snap_to_state): a theme switch is animated, and a
    /// theme switch is the caller announcing a new palette and then moving the
    /// gauge toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the gauge chasing a palette that is still
    /// moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the gauge's minimum, in its own units.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// assert_eq!(gauge.min(), 0.0);
    /// assert_eq!(gauge.max(), 240.0);
    /// ```
    #[must_use]
    pub fn min(&self) -> f32 {
        self.min
    }

    /// Returns the gauge's maximum, in its own units.
    #[must_use]
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Sets the range the gauge covers, and pulls the value and the drawn value
    /// back inside it.
    ///
    /// The two are ordered, so `set_range(240.0, 0.0)` is a gauge from 0 to 240
    /// and not one with a negative span. The value is re-clamped and the drawn
    /// value follows it at once rather than travelling: a range that has just
    /// changed has no transition to run, and a needle travelling into a track
    /// that has not been drawn yet is a frame of nonsense.
    pub fn set_range(&mut self, min: f32, max: f32) {
        let (min, max) = ordered(min, max);
        self.min = min;
        self.max = max;
        let value = self.clamped_value();
        self.clock.borrow_mut().clear();
        self.value.set(value);
        self.shown.set(value);
    }

    /// Returns where the gauge's arc starts, in degrees clockwise from three
    /// o'clock.
    #[must_use]
    pub fn start_angle(&self) -> f32 {
        self.start_angle
    }

    /// Returns where the gauge's arc ends, in degrees clockwise from three
    /// o'clock.
    #[must_use]
    pub fn end_angle(&self) -> f32 {
        self.end_angle
    }

    /// Sets where the gauge's arc starts and ends, in degrees clockwise from
    /// three o'clock.
    ///
    /// The sweep is the difference taken **modulo a turn**, so the pair 135 and 45
    /// and the pair 135 and 405 are the same 270-degree arc, and a pair whose
    /// difference is a whole number of turns — the same angle twice, or 135 and
    /// 135 — is an arc of no extent at all. A gauge in that state draws nothing
    /// rather than a full turn or a single mark, and [`GaugeType::Circle`] is how
    /// a caller asks for the latter.
    ///
    /// Nothing animates: the angles are the mapping from a value to a place, not
    /// the value and not its colour, so a new pair takes effect on the next frame
    /// without disturbing a transition already running.
    pub fn set_angles(&mut self, start_angle: f32, end_angle: f32) {
        self.start_angle = start_angle;
        self.end_angle = end_angle;
    }

    /// Returns the shape the gauge draws itself in.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::gauge::{Gauge, GaugeType};
    ///
    /// let mut nodes = Arena::new();
    /// let mut gauge = Gauge::new(&mut nodes, 0.0, 100.0);
    /// assert_eq!(gauge.gauge_type(), GaugeType::Arc);
    /// gauge.set_gauge_type(GaugeType::Needle);
    /// assert_eq!(gauge.gauge_type(), GaugeType::Needle);
    /// ```
    #[must_use]
    pub fn gauge_type(&self) -> GaugeType {
        self.gauge_type
    }

    /// Sets the shape the gauge draws itself in, and puts the needle's colour
    /// where a needle about to appear expects to find it.
    ///
    /// Switching **to** [`GaugeType::Needle`] writes
    /// [`needle`](Gauge::needle) at once, so the first frame drawn with a
    /// pointer on it is that pointer in the current palette rather than in
    /// whatever the colour was before the caller had one. Switching **away**
    /// leaves it alone, because a needle is not drawn and its colour is not
    /// something the frame can see.
    ///
    /// The mode is a plain field and this setter is the only way to write it,
    /// which is what keeps the mode and the drawn value from disagreeing.
    /// [`GaugeType`] documents why it is not a `Property<GaugeType>`.
    pub fn set_gauge_type(&mut self, gauge_type: GaugeType) {
        self.gauge_type = gauge_type;
        if gauge_type == GaugeType::Needle {
            self.needle.set(self.palette.needle);
        }
    }

    /// Returns how thick the arc is, in pixels.
    #[must_use]
    pub fn thickness(&self) -> f32 {
        self.thickness
    }

    /// Sets how thick the arc is, in pixels.
    ///
    /// A negative thickness is no thickness: a negative one puts the band's inner
    /// edge outside its own outer edge, which is a quad of nothing drawn inside
    /// out. Zero is also drawn as nothing at all — the ticks and the needle, which
    /// have sizes of their own, are still drawn.
    ///
    /// The thickness is the band's only width. It does **not** set how many quads
    /// the band is cut into: that is `MAX_DEGREES_PER_SEGMENT`, the one number the
    /// tessellation is built from, and it is the only thing that is — so a caller
    /// halving the thickness halves the pixels of chord error and changes nothing
    /// else. What would bring the two together again is a caller that wants a
    /// constant error as a *share* of the band rather than a constant number of
    /// pixels.
    pub fn set_thickness(&mut self, thickness: f32) {
        self.thickness = thickness.max(0.0);
    }

    /// Returns how many tick marks the gauge draws.
    ///
    /// Zero means none, which is the way to ask for a plain progress ring.
    #[must_use]
    pub fn ticks(&self) -> u8 {
        self.ticks
    }

    /// Sets how many tick marks the gauge draws, and puts the tick colour where
    /// marks about to appear expect to find it.
    ///
    /// The count is a `u8`, so the largest number of marks a caller can ask for is
    /// 255 and no clamping is needed: a dial with 255 marks on it is a caller
    /// error, not a number this setter has to guess what was meant. Zero is the
    /// other end and is meaningful — it is how a caller asks for a plain progress
    /// ring.
    ///
    /// The marks are one [`DrawCommand::Line`] each, so this is the number that
    /// decides how many quads a gauge costs alongside its arc.
    ///
    /// Turning marks **on** writes [`tick`](Gauge::tick) at once, for the reason
    /// [`set_gauge_type`](Gauge::set_gauge_type) writes the needle's: the first
    /// frame with marks on it is that frame in the current palette, rather than in
    /// whatever the colour was before the caller had one. Turning them **off**
    /// leaves the colour alone, because nothing draws it.
    pub fn set_ticks(&mut self, ticks: u8) {
        self.ticks = ticks;
        if ticks > 0 {
            self.tick.set(self.palette.tick);
        }
    }

    /// Returns the size a gauge asks for: `DEFAULT_SIZE`, 200, square.
    ///
    /// A gauge has no content to measure, so this is only for a caller that has
    /// nothing else to go on; a caller that lays the dial out itself gives the
    /// node whatever rect it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](Gauge::paint) draws inside whatever it is given. The dial is
    /// inscribed in the rect, so the *shorter* of the two sides is what a caller
    /// with a wide box to fill is really giving the gauge.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// let size = gauge.size();
    /// assert_eq!((size.width, size.height), (200.0, 200.0), "a dial is square");
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(DEFAULT_SIZE, DEFAULT_SIZE)
    }

    /// Returns the share of the range `value` is, from `0.0` to `1.0`.
    ///
    /// It is public because it is the gauge's contract with anything drawn beside
    /// it: a caller drawing a value label, or a scale of its own, needs this
    /// mapping rather than its own. The drawn value is the same map applied to
    /// [`shown`](Gauge::shown), which is the only version of the question a test
    /// about animation can ask.
    ///
    /// A value outside the range is clamped rather than wrapped, so a caller
    /// reporting 260 on a 0-to-240 dial gets a full gauge and one reporting −20
    /// gets an empty one. A value of `NaN` reads as `0.0`: a caller that has lost
    /// track of its arithmetic gets an empty gauge rather than an arc of `NaN`
    /// centres. A range whose two ends are the same number has one position and
    /// no way along the arc to reach it, and answers `0.0` for everything.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// assert_eq!(gauge.fraction(0.0), 0.0);
    /// assert_eq!(gauge.fraction(60.0), 0.25);
    /// assert_eq!(gauge.fraction(120.0), 0.5);
    /// assert_eq!(gauge.fraction(300.0), 1.0, "past the top is the top");
    /// assert_eq!(gauge.fraction(-20.0), 0.0, "and below the bottom is the bottom");
    /// ```
    #[must_use]
    pub fn fraction(&self, value: f32) -> f32 {
        let span = self.max - self.min;
        // Spelled out rather than written `!(span > 0.0)`: a range whose ends are
        // both `NaN` gives a `NaN` span, and the question here is whether the
        // gauge has any distance along its arc to divide by, which `NaN` does not
        // answer.
        if span <= 0.0 || span.is_nan() {
            return 0.0;
        }
        bounded((value - self.min) / span, 0.0, 1.0)
    }

    /// Returns the appearance the gauge's value and its palette imply.
    ///
    /// The value is clamped here rather than at paint time alone, so a caller
    /// reading the target — which is what a test and a caller drawing a label
    /// both do — is told about the end of the range and not about a reading past
    /// it. The angles and the mode are **not** in here: they are the mapping from
    /// a value to a place, and no transition runs toward either of them.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// gauge.value.set(120.0);
    /// assert_eq!(gauge.style().shown, 120.0);
    ///
    /// gauge.value.set(500.0);
    /// assert_eq!(gauge.style().shown, 240.0, "past the top is the top");
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            shown: self.clamped_value(),
            track: self.palette.track,
            fill: self.palette.fill,
            tick: self.palette.tick,
            needle: self.palette.needle,
        }
    }

    /// Applies the appearance the value and the palette imply at once, with no
    /// transition, and ends every transition already running.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a gauge that has just been given a [`Palette`](Gauge::set_palette)
    /// and has never animated — whose colour properties still hold the neutral
    /// defaults [`Gauge::new`] wrote, so without this a themed gauge starts out
    /// grey — and a caller that has written [`value`](Gauge::value) itself and
    /// wants the gauge to be that state now.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::gauge::{Palette, Gauge};
    ///
    /// let mut nodes = Arena::new();
    /// let mut gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// let themed = Palette::from_theme(&Theme::dark());
    /// gauge.set_palette(themed);
    /// gauge.snap_to_state();
    /// assert_eq!(gauge.track.get(), themed.track);
    /// assert_eq!(gauge.needle.get(), themed.needle);
    /// ```
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.shown.set(style.shown);
        self.track.set(style.track);
        self.fill.set(style.fill);
        self.tick.set(style.tick);
        self.needle.set(style.needle);
    }

    /// Starts the transitions that carry the gauge from wherever it is toward the
    /// appearance [`style`](Gauge::style) implies, on `motion`.
    ///
    /// This is the call a caller makes after writing [`value`](Gauge::value), and
    /// it is what requirement 4's "fill animates when value changes" and its
    /// "needle animates with spring physics" are both made of: the needle points
    /// at [`shown`](Gauge::shown), so a spring on this call is a springing needle
    /// — and a spring on the fill at the same time, since both read the same
    /// property. **The widget picks no curve of its own.** A caller who wants the
    /// needle to settle and the arc to ease passes an
    /// [`Easing::Spring`](crate::animation::Easing::Spring); a caller who wants a
    /// plain glide passes `Easing::EaseOut`; and the duration comes from the
    /// theme through [`Motion::from_theme`], which is how requirement 4's
    /// "animation duration from theme tokens" is satisfied.
    ///
    /// The gauge's own clock is cleared first, so the transitions this replaces
    /// stop where they are rather than writing over the new ones when they arrive.
    ///
    /// A property nothing draws in the current mode is not animated at all: the
    /// tick marks are left alone when there are none, and the needle's colour is
    /// left alone outside [`GaugeType::Needle`]. A transition on a property
    /// nothing draws is a transition that reports itself as running for nothing,
    /// and a caller repainting only while `is_animating` would then repaint for a
    /// frame in which nothing moves.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let gauge = Gauge::new(&mut nodes, 0.0, 100.0);
    /// let motion = Motion { duration: Duration::from_millis(100), easing: Easing::Linear };
    ///
    /// gauge.value.set(50.0);
    /// gauge.animate_to_state(motion);
    /// assert_eq!(gauge.shown.get(), 0.0, "the needle starts where it was");
    /// gauge.tick(Duration::from_millis(50));
    /// assert_eq!(gauge.shown.get(), 25.0, "and is half way at half the time");
    /// ```
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.shown
                .animate_to(style.shown, motion.duration, motion.easing),
        );
        clock.add(
            self.track
                .animate_to(style.track, motion.duration, motion.easing),
        );
        clock.add(
            self.fill
                .animate_to(style.fill, motion.duration, motion.easing),
        );
        if self.ticks > 0 {
            clock.add(
                self.tick
                    .animate_to(style.tick, motion.duration, motion.easing),
            );
        }
        if self.gauge_type == GaugeType::Needle {
            clock.add(
                self.needle
                    .animate_to(style.needle, motion.duration, motion.easing),
            );
        }
    }

    /// Advances the gauge's transitions by `delta`, and returns whether any of
    /// them wrote.
    ///
    /// It is the gauge's frame integration: call it once a frame, before the paint
    /// pass, with the time that frame took. The write is what reaches the node — a
    /// property callback registered by the caller marks the node dirty — so a
    /// caller that repaints only when this is true repaints exactly while
    /// something moves.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// gauge.value.set(240.0);
    /// gauge.animate_to_state(Motion {
    ///     duration: Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    ///
    /// assert!(gauge.tick(Duration::from_millis(50)), "the needle is moving");
    /// assert!(gauge.is_animating());
    /// assert!(gauge.tick(Duration::from_millis(50)), "and still is");
    /// assert_eq!(gauge.shown.get(), 240.0, "arrived");
    /// assert!(!gauge.is_animating(), "so nothing is left running");
    /// ```
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Returns whether any of the gauge's transitions is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Returns the draw commands that paint the gauge within `rect`.
    ///
    /// The commands are, in order: the **track**, the whole sweep as a band of
    /// filled quads whose corners sit on the inner and outer radii; the **tick
    /// marks**, when there are any; the **fill**, the same band over the part of
    /// the sweep the drawn value has reached; and in [`GaugeType::Needle`] the
    /// **needle** as a filled triangle with a circle at its hub.
    ///
    /// **The fill never reaches outside the track**, at any value and at any point
    /// of any curve, and that is a property of the geometry rather than a hope.
    /// Both bands are cut by the same function from the *same* arc radius and the
    /// *same* thickness, so every fill corner is at the same distance from the
    /// dial's centre as the track's corner at the same angle — `R + thickness/2`
    /// for the outer pair and `R - thickness/2` for the inner — which puts the
    /// fill inside the track's own annulus by construction. The fill's angular
    /// extent is the track's own extent times a fraction clamped to `0.0..=1.0`,
    /// so an overshooting curve cannot carry it past the end of the arc either.
    ///
    /// The dial is inscribed in `rect`, and the band's outer edge is exactly at
    /// `rect`'s own shorter side, so **nothing this method records reaches outside
    /// the rect either** — including the needle, whose tip is four fifths of the
    /// radius out.
    ///
    /// A sweep that is not positive — the two angles the same, or a `NaN` among
    /// the angles or the thickness — draws nothing at all rather than a mark or a
    /// full turn. And a fill with no extent is not recorded at all: a gauge at its
    /// minimum draws its track and its ticks and nothing else.
    ///
    /// **Every size here comes from the *drawn* value**, so a gauge mid-transition
    /// paints its fill as far as the transition has got and its needle where the
    /// transition has reached, not as far as the value it is travelling toward.
    ///
    /// **These edges are antialiased, by the framebuffer rather than by this
    /// widget** (superseded 2026-10-02 — the module document's *There is no
    /// anti-aliasing* section is superseded in place, and this said
    /// *"None of these edges is antialiased … and nothing in the pipeline is
    /// multisampled"*, which was true of the context before
    /// `render::context`'s `MULTISAMPLE_SAMPLES` and is false now). **What is
    /// still true** is the account of what this method records: the band and the
    /// needle are [`DrawCommand::Polygon`]s and the tick marks are
    /// [`DrawCommand::Line`]s, `line_quad` and `polygon_quad` both hardcode
    /// `radius: 0.0`, and **none of them reaches the solid shader's own
    /// antialiasing branch** — a hard `discard` on an axis-aligned rounded
    /// rectangle. The default framebuffer is now 4x multisampled, so every
    /// geometric edge below is resolved from four coverage samples by the
    /// hardware, and the module document has the before-and-after pixel counts.
    /// **What 4x does not do** is make the pipeline resolution-independent, and
    /// it costs measurable frame time on a fill-rate-bound target; both are in
    /// `MULTISAMPLE_SAMPLES`'s own doc.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{DrawCommand, Rect};
    /// use ui_core::widgets::gauge::Gauge;
    ///
    /// let mut nodes = Arena::new();
    /// let mut gauge = Gauge::new(&mut nodes, 0.0, 240.0);
    /// let rect = Rect::new(32.0, 64.0, 200.0, 200.0);
    ///
    /// // Nothing to fill: the track's band, then one line per tick mark, and no
    /// // second band drawn over it.
    /// let empty = gauge.paint(rect);
    /// assert_eq!(quads(&empty), 33, "the 270-degree track's own band");
    /// assert_eq!(lines(&empty), 11, "and eleven tick marks");
    ///
    /// gauge.value.set(240.0);
    /// gauge.snap_to_state();
    /// assert_eq!(
    ///     quads(&gauge.paint(rect)),
    ///     66,
    ///     "a full value adds a second band of the same length"
    /// );
    ///
    /// // A band segment is a four-pointed polygon; a needle is a three-pointed one.
    /// fn quads(commands: &[DrawCommand]) -> usize {
    ///     commands
    ///         .iter()
    ///         .filter(|c| matches!(c, DrawCommand::Polygon { points, .. } if points.len() == 4))
    ///         .count()
    /// }
    /// fn lines(commands: &[DrawCommand]) -> usize {
    ///     commands.iter().filter(|c| matches!(c, DrawCommand::Line { .. })).count()
    /// }
    /// ```
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let radius = self.arc_radius(rect);
        let sweep = self.sweep();
        // A sweep that is not positive, or a radius with nowhere to put it, is a
        // caller error handled as an ordinary value: there is no arc, so there is
        // nothing to draw, and neither the ticks nor the needle have a place to
        // sit.
        if !positive(radius) || !positive(sweep) {
            return painter.finish();
        }
        let centre = dial_centre(rect);
        self.draw_arc(&mut painter, centre, radius, sweep, self.track.get());
        self.draw_ticks(&mut painter, centre, radius, sweep);
        let fill = sweep * self.drawn_fraction();
        self.draw_arc(&mut painter, centre, radius, fill, self.fill.get());
        if self.gauge_type == GaugeType::Needle {
            self.draw_needle(&mut painter, centre, radius);
        }
        painter.finish()
    }

    /// Returns the value clamped to `min..=max`: what [`style`](Gauge::style)
    /// reports and what a transition is aimed at.
    ///
    /// Clamping is the target of [`value`](Gauge::value), not the target of
    /// [`shown`](Gauge::shown): a caller reading `style` is asking where the
    /// needle is heading and is told about the end of the range rather than about
    /// a reading past it. The drawn property needs no clamp of its own, because
    /// [`drawn_fraction`](Gauge::drawn_fraction) asks for a *share* and
    /// [`fraction`](Gauge::fraction) bounds one.
    fn clamped_value(&self) -> f32 {
        bounded(self.value.get(), self.min, self.max)
    }

    /// Returns the share of the range the gauge is *drawn* at, from the drawn
    /// property rather than from the truth.
    ///
    /// It is what a frame mid-transition paints, which is the only version of the
    /// question a test about animation can ask. There is no clamp here, and
    /// [`shown`](Gauge::shown) being a public property a caller may write
    /// directly is exactly why there does not need to be one:
    /// [`fraction`](Gauge::fraction) already returns a share in `0.0..=1.0`, so a
    /// drawn value of 500 on a 0-to-240 gauge is a full arc and one of −100 is an
    /// empty one whatever order the range is applied in.
    ///
    /// That is *not* the same arrangement as the progress bar's, which has no
    /// `fraction` of its own and does need its own clamp here, and a mutation
    /// removing this one's callers would have found it — a clamp that survives
    /// being deleted is a clamp that was not doing anything.
    fn drawn_fraction(&self) -> f32 {
        self.fraction(self.shown.get())
    }

    /// Returns how far the arc sweeps, in degrees clockwise.
    ///
    /// [`GaugeType::Circle`] is a full turn whatever the angles say, so this is
    /// [`FULL_TURN`] in that mode and the difference of the two angles in the
    /// others. The difference is taken **modulo a turn**, which is what makes 135
    /// and 45 a 270-degree arc and lets a caller write the end of an arc as the
    /// angle the pointer ends at rather than as an unwrapped 405.
    fn sweep(&self) -> f32 {
        if self.gauge_type == GaugeType::Circle {
            return FULL_TURN;
        }
        (self.end_angle - self.start_angle).rem_euclid(FULL_TURN)
    }

    /// Returns the radius of the arc's centre line inside `rect`.
    ///
    /// The dial is inscribed in the rect's **shorter** side, and half the
    /// thickness is taken off that, so the band's outer edge — a circle of
    /// `thickness/2` about the centre line, which is what the corners in
    /// [`draw_arc`](Gauge::draw_arc) are placed at — lands exactly on the rect's
    /// own edge. The two numbers that a rect does not have in common, its width
    /// and its height, are the two that matter: a non-square box inscribes its
    /// dial in the smaller of them, and neither is ever read as the other.
    ///
    /// The result is `.max(0.0)`: a rect narrower than the arc is thick is a caller
    /// error, and a dial of no radius draws nothing rather than one that draws
    /// inside out.
    fn arc_radius(&self, rect: Rect) -> f32 {
        (rect.width.min(rect.height) / 2.0 - self.thickness / 2.0).max(0.0)
    }

    /// Returns the number of quads an arc of `sweep` degrees at `radius` is drawn
    /// as, or none at all.
    ///
    /// The count is what a test asserts against and what a caller measuring cost
    /// would measure, so it is a named function rather than a number inside
    /// [`draw_arc`](Gauge::draw_arc): the band's length is the price of drawing an
    /// arc out of quads instead of out of one primitive, and a claim about it
    /// should be checkable without counting quads out of a paint.
    ///
    /// **`radius` is refused but not used**, and that is the retune the chain of
    /// circles forced. A chain's step was whatever two tangent circles of its
    /// thickness needed at that radius — `2·asin(thickness / 2 / radius)` — so its
    /// length fell out of the dial's size and its stroke, and a hairline arc on a
    /// large dial asked for thousands of them and had to be capped. A band's step
    /// is set by [`MAX_DEGREES_PER_SEGMENT`] alone: the sagitta is second order in
    /// the step and scales with the radius, so the angle is the only thing worth
    /// choosing and the radius is only worth refusing. **What would bring the
    /// radius back** is a demand for a constant error in *fractions of a pixel per
    /// hundred of radius* rather than a constant error in pixels, which would make
    /// a big dial and a small one look equally faceted.
    ///
    /// A sweep that is not positive, a radius of nothing, or a thickness of nothing
    /// is zero segments — and all three are answered before anything is divided, so
    /// a caller who has set the two angles equal gets nothing drawn rather than the
    /// floor's worth of quads with a step of nothing between them.
    fn arc_segment_count(&self, radius: f32, sweep: f32) -> u16 {
        if !positive(radius) || !positive(sweep) || !positive(self.thickness) {
            return 0;
        }
        segments_u16(sweep / MAX_DEGREES_PER_SEGMENT).max(MIN_ARC_SEGMENTS)
    }

    /// Draws an arc of `sweep` degrees at `radius` about `centre`, as a band of
    /// filled quads whose corners sit on the inner and outer radii.
    ///
    /// One quad per [`arc_segment_count`](Gauge::arc_segment_count) segment,
    /// evenly spread from [`start_angle`](Gauge::start_angle) so that the last
    /// segment's far edge lands **on** the end of the sweep rather than short of
    /// it. The four corners are `inner_i, outer_i, outer_{i+1}, inner_{i+1}`: the
    /// two at the segment's start angle, the two at its end, and the inner pair
    /// nearer the dial's centre.
    ///
    /// **The corners are the whole reason for this shape.** Placed on the radii
    /// rather than offset from a centre line, the band's drawn thickness is the
    /// requested one with no error in it at all — where `DrawCommand::Path` would
    /// put a segment's outer corner at `sqrt(R² + r²)` and come out 6.7 px thin
    /// everywhere. The only error left is the chord along the outer edge, which
    /// sags by `(R + r)·(1 − cos(step/2))` — 0.255 px at the defaults, and second
    /// order in the step.
    ///
    /// **The order is the same on every segment, and it has to be.** The renderer
    /// fans a polygon from its first point, and the fan is exact for a convex one
    /// whatever its winding, so a consistent order is what makes every quad convex
    /// in the first place: `inner → outer → outer' → inner'` turns the same way at
    /// every angle, and a band's ends are flat caps rather than the round ones the
    /// chain had.
    ///
    /// The inner radius is floored at zero. A rect narrower than the arc is thick
    /// leaves a centre line inside the band, and a negative inner radius would put
    /// the inner corners on the *far* side of the dial and make the quad concave,
    /// which the fan would draw as overlapping triangles. At zero the two inner
    /// corners are the dial's own centre and the quad is a sector.
    fn draw_arc(
        &self,
        painter: &mut Painter,
        centre: (f32, f32),
        radius: f32,
        sweep: f32,
        color: Color,
    ) {
        let segments = self.arc_segment_count(radius, sweep);
        if segments == 0 {
            return;
        }
        let step = sweep / f32::from(segments);
        let half = self.thickness / 2.0;
        let outer = radius + half;
        let inner = (radius - half).max(0.0);
        for index in 0..segments {
            let from = self.start_angle + step * f32::from(index);
            let to = from + step;
            painter.polygon(
                &[
                    point_at(centre, inner, from),
                    point_at(centre, outer, from),
                    point_at(centre, outer, to),
                    point_at(centre, inner, to),
                ],
                color,
            );
        }
    }

    /// Draws the tick marks: one [`DrawCommand::Line`] each, from the track's inner
    /// edge inward by [`TICK_LENGTH`].
    ///
    /// The marks are spread over the **whole** sweep rather than over the fill, at
    /// `i / (ticks - 1)` so that the first and the last marks sit exactly on the
    /// arc's two ends — a dial whose extremes have no mark on them cannot be read
    /// at its own extremes. One mark is drawn at the start angle rather than
    /// divided by a zero.
    ///
    /// **The band's inner edge is the same number it always was** —
    /// `radius - thickness/2`, the same expression the band's own inner corners
    /// are placed at — so a mark still starts exactly where the track stops. That
    /// is the whole of what the change to the arc's shape did to the marks, and
    /// they need no adjustment for it: `outer` here is the band's *inner* edge and
    /// is still `radius - thickness/2`, because the band's radii did not move when
    /// the circles went away.
    ///
    /// A mark is the one primitive here that pays nothing for being a stroke.
    /// `line_quad` offsets a segment **perpendicular** to it, and a tick is radial,
    /// so the offset is across the tick and its length is its own: a two-pixel-wide
    /// mark from 86 to 78 on the defaults is drawn as a two-pixel-wide mark from 86
    /// to 78. The band is not so lucky, which is why it is a polygon.
    fn draw_ticks(&self, painter: &mut Painter, centre: (f32, f32), radius: f32, sweep: f32) {
        if self.ticks == 0 {
            return;
        }
        let outer = radius - self.thickness / 2.0;
        let inner = (outer - TICK_LENGTH).max(0.0);
        let span = f32::from(self.ticks - 1).max(1.0);
        for index in 0..self.ticks {
            let along = f32::from(index) / span;
            let angle = self.start_angle + sweep * along;
            painter.line(
                point_at(centre, outer, angle),
                point_at(centre, inner, angle),
                TICK_THICKNESS,
                self.tick.get(),
            );
        }
    }

    /// Draws the needle: a filled triangle from the hub out to the drawn value's
    /// angle, and a circle over its base.
    ///
    /// Three points, which is the case the renderer's convex fan is exact for, and
    /// the tip at [`NEEDLE_LENGTH_FRACTION`] of the radius so it stops short of
    /// the track rather than lying along it. The hub covers the base, which is
    /// what makes the shape read as a pointer rather than as a triangle.
    ///
    /// The angle is the **drawn** value's, not the truth's: a needle that reached
    /// for the truth a frame before the arc arrived would be a needle and an arc
    /// disagreeing about the same reading.
    fn draw_needle(&self, painter: &mut Painter, centre: (f32, f32), radius: f32) {
        let angle = self.start_angle + self.sweep() * self.drawn_fraction();
        let tip = point_at(centre, radius * NEEDLE_LENGTH_FRACTION, angle);
        let base = radius * NEEDLE_BASE_FRACTION;
        let points = [
            tip,
            point_at(centre, base, angle + 90.0),
            point_at(centre, base, angle - 90.0),
        ];
        painter.polygon(&points, self.needle.get());
        painter.circle(centre, NEEDLE_HUB_RADIUS, self.needle.get());
    }
}

/// Returns whether `value` is a number this widget can do geometry with: greater
/// than zero, and not `NaN`.
///
/// It is named rather than written `value > 0.0` at each of the four places that
/// ask, because the question is not "is it above zero" but "is it *usable*", and
/// the answer to the second is `false` for a `NaN` in a way the first — with a
/// `!` in front of it — says much less clearly. A radius of `NaN` and a sweep of
/// `NaN` are both degenerate caller values, and both draw nothing.
fn positive(value: f32) -> bool {
    value > 0.0
}

/// Returns `min` and `max` in order, with `min` never above `max`.
///
/// `f32::min` and `f32::max` return the other operand when one of the two is
/// `NaN`, so a caller that has lost track of a bound gets a usable range rather
/// than a `NaN` one. Two `NaN` bounds cannot be made sense of, and they leave the
/// mapping `NaN`, which draws nothing — a caller error that shows as an empty
/// dial rather than as a frame that takes the process down.
fn ordered(min: f32, max: f32) -> (f32, f32) {
    (min.min(max), min.max(max))
}

/// Returns `value` rounded up, as a segment count.
///
/// **The one `as` cast in this module, and std is the reason**: there is no
/// `From`/`TryFrom` between `f32` and any integer type — the `From` impls between
/// integers stop at the 16-bit widths and no float conversion is provided at all
/// — so there is no `TryInto` to reach for. `list.rs`'s `ceil_to_usize` is the
/// same argument for the same shape of question.
///
/// The cast is saturating rather than wrapping since Rust 1.45, so a value past
/// the top of the range gives `u16::MAX` rather than zero: a very long band
/// rather than a missing one. **Nothing can reach that case** — a sweep is at most
/// [`FULL_TURN`] and the divisor is 8.25, so the largest count this can be asked
/// for is 44 — and `arc_segment_count` refuses a sweep that is not positive before
/// it gets here, so the `NaN` that would saturate to zero never arrives either.
fn segments_u16(value: f32) -> u16 {
    value.ceil() as u16
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

/// Returns the centre of the dial inside `rect`: the rect's own middle, from its
/// own origin and its own two extents.
///
/// It is a named function rather than arithmetic in three places because the
/// origin and the extent are different numbers and every one of them has to be
/// read: a circle in a box whose centre was computed from its left edge instead
/// of from its width is a circle in the wrong place, and no assertion on a
/// fixture at the origin could see it.
fn dial_centre(rect: Rect) -> (f32, f32) {
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

/// Returns the point at `degrees` clockwise from three o'clock, `radius` from
/// `centre`.
///
/// The conversion is `f32::to_radians` rather than a multiplication by a constant,
/// so the unit is named where the conversion happens. The sign of `y` follows the
/// screen: degrees increase clockwise because the y axis points down, and a
/// caller reading these four numbers back can tell which way the dial runs.
fn point_at(centre: (f32, f32), radius: f32, degrees: f32) -> (f32, f32) {
    let radians = degrees.to_radians();
    (
        centre.0 + radius * radians.cos(),
        centre.1 + radius * radians.sin(),
    )
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback for
/// a token a caller has written the wrong variant into — and black rather than a
/// panic, because a mistyped theme token is not worth taking a frame down for. It
/// is the button's and the slider's own helper, repeated rather than imported: it
/// is four lines, and a shared module for one four-line helper is a module.
fn token_color(theme: &Theme, token: crate::theme::ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Easing;
    use crate::theme::ThemeToken;

    /// The square most of the geometry tests lay a gauge out in: 200 by 200 at
    /// (32, 64), so the dial's centre is at (132, 164).
    ///
    /// It is deliberately **not** at the origin, and every other fixture in this
    /// module is off it too. A rect's origin and a rect's extent are different
    /// numbers, and a suite whose every fixture sits at `(0, 0)` cannot see the
    /// one being used as the other — the slider's tests passed all fifty-three of
    /// them while every slider drawn anywhere else reported a negative travel. A
    /// gauge is radial, so the mistake would be a dial drawn off the middle of its
    /// own box rather than a fill one pixel out, and there is no way to see that
    /// except from a fixture that is not at the corner.
    const RECT: Rect = Rect {
        x: 32.0,
        y: 64.0,
        width: 200.0,
        height: 200.0,
    };

    /// A gauge in a box that is **not** square: 320 by 180 at (700, 40), whose
    /// centre is (860, 130).
    ///
    /// This is the fixture that matters more here than anywhere else in the
    /// repository. A circle in a square box is the degenerate case: its centre is
    /// on the diagonal, so a centre computed from `x` and one computed from
    /// `y + height` can agree by accident, and an arc's radius read from the
    /// width and one read from the height are the same number. A non-square box
    /// makes every one of those four reads distinguishable, which is why this
    /// fixture is a gauge test suite's first requirement rather than its last.
    const WIDE: Rect = Rect {
        x: 700.0,
        y: 40.0,
        width: 320.0,
        height: 180.0,
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

    /// Advances `gauge`'s transitions by `millis` and returns whether anything
    /// moved.
    ///
    /// The gauge's own `tick` is what a frame calls and it answers whether a
    /// repaint is needed, so a test that only wants time to pass goes through here
    /// rather than discarding a `#[must_use]` result: that answer is passed on, not
    /// dropped on the floor.
    fn tick(gauge: &Gauge, millis: u64) -> bool {
        gauge.tick(ms(millis))
    }

    /// A gauge in the dark theme's palette, with its colours already arrived at
    /// that palette.
    fn gauge() -> (Arena<WidgetNode>, Gauge) {
        let mut nodes = Arena::new();
        let mut gauge = Gauge::new(&mut nodes, 0.0, 240.0);
        gauge.set_palette(Palette::from_theme(&Theme::dark()));
        gauge.snap_to_state();
        (nodes, gauge)
    }

    /// A gauge over `min..=max` showing `value` of it, snapped so the drawn value
    /// is where the truth is.
    fn showing(min: f32, max: f32, value: f32) -> (Arena<WidgetNode>, Gauge) {
        let mut nodes = Arena::new();
        let mut gauge = Gauge::new(&mut nodes, min, max);
        gauge.set_palette(Palette::from_theme(&Theme::dark()));
        gauge.value.set(value);
        gauge.snap_to_state();
        (nodes, gauge)
    }

    /// A themed gauge at half of a 0-to-240 range, snapped.
    fn half() -> (Arena<WidgetNode>, Gauge) {
        showing(0.0, 240.0, 120.0)
    }

    /// A short name for each recorded command, in the order they were recorded.
    ///
    /// It cannot see a size or a position — two bands of different lengths are the
    /// same list of names — so every test about geometry reads the coordinates too.
    /// The polygon name carries its point count, because **the count is the
    /// difference between the two kinds of polygon this widget records**: a band
    /// segment is four points and the needle is three.
    fn shapes(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::Circle { .. } => "circle",
                DrawCommand::Line { .. } => "line",
                DrawCommand::Polygon { points, .. } => match points.len() {
                    3 => "needle",
                    4 => "band",
                    _ => "poly",
                },
                _ => "other",
            })
            .collect()
    }

    /// The circles a paint recorded, as `(centre, radius)`.
    ///
    /// A gauge records exactly one: the needle's hub, and only in
    /// [`GaugeType::Needle`]. Every other part of the dial is a polygon or a line.
    fn circles(commands: &[DrawCommand]) -> Vec<((f32, f32), f32)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Circle { center, radius, .. } => Some((*center, *radius)),
                _ => None,
            })
            .collect()
    }

    /// A recorded line, as its two ends and its width.
    type Line = ((f32, f32), (f32, f32), f32);

    /// The lines a paint recorded.
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

    /// The band's four points, for each segment of each band a paint recorded in
    /// `color`, in the order they were recorded.
    ///
    /// This is the raw geometry — the numbers the defect lived in. Every other
    /// view of the band in this module is derived from these four points, because a
    /// draw-command assertion that counts a band cannot see where any of its
    /// corners landed, and a band drawn with its corners off the radii is a band of
    /// the wrong thickness with the right number of commands in it.
    fn band(commands: &[DrawCommand], color: Color) -> Vec<Vec<(f32, f32)>> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Polygon { points, color: c } if *c == color && points.len() == 4 => {
                    Some(points.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The band's own extent, as the outer corner at the arc's **start** followed by
    /// the outer corner at the **end** of every segment.
    ///
    /// So the first point is where the band begins and the last is where it
    /// finishes, and [`span`] over the pair is the sweep in degrees whatever the
    /// tessellation. It is a list and not a pair of points because [`gaps`] and
    /// the evenness assertions want every corner between the two as well.
    ///
    /// **The ends, not the centres.** The chain this replaced had one circle per
    /// step and each of them sat on a step boundary, so its first and last centres
    /// *were* the arc's two ends; a segment's outer corner is at its own end, and a
    /// list of those starts one step late and stops one step early.
    fn reach(commands: &[DrawCommand], color: Color) -> Vec<(f32, f32)> {
        let quads = band(commands, color);
        let mut points = Vec::with_capacity(quads.len() + 1);
        if let Some(first) = quads.first() {
            points.push(first[1]);
        }
        points.extend(quads.iter().map(|quad| quad[2]));
        points
    }

    /// The needle's three points and colour, or `None` if no needle was recorded.
    ///
    /// Matched on the point count rather than taken as the first polygon, because
    /// the band's segments are polygons too and come first: a "first polygon" would
    /// have found a four-pointed band segment and read its second point as a needle
    /// tip.
    #[allow(clippy::type_complexity)]
    fn needle(commands: &[DrawCommand]) -> Option<(Vec<(f32, f32)>, Color)> {
        commands.iter().find_map(|command| match command {
            DrawCommand::Polygon { points, color } if points.len() == 3 => {
                Some((points.clone(), *color))
            }
            _ => None,
        })
    }

    /// The colour of each recorded polygon, in order.
    fn polygon_colors(commands: &[DrawCommand]) -> Vec<Color> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Polygon { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    }

    /// The colour of each recorded line, in order.
    fn line_colors(commands: &[DrawCommand]) -> Vec<Color> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Line { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    }

    /// The centre of the dial in `rect`.
    fn centre(rect: Rect) -> (f32, f32) {
        (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    /// The distance from the dial's centre in `rect` to `point`.
    fn from_centre(rect: Rect, point: (f32, f32)) -> f32 {
        let (cx, cy) = centre(rect);
        ((point.0 - cx).powi(2) + (point.1 - cy).powi(2)).sqrt()
    }

    /// The angle, in degrees clockwise from three o'clock, of `point` as seen
    /// from the dial's centre in `rect`.
    fn angle_of(rect: Rect, point: (f32, f32)) -> f32 {
        let (cx, cy) = centre(rect);
        (point.1 - cy)
            .atan2(point.0 - cx)
            .to_degrees()
            .rem_euclid(FULL_TURN)
    }

    /// Asserts two values are within a millionth of one another.
    ///
    /// The products are computed in `f32`, so `240.0 * 0.3` is not exactly 72.0: a
    /// test that wrote the decimal out would be asserting the compiler's rounding
    /// rather than the widget. Everything the widget computes exactly is asserted
    /// with `assert_eq!` instead.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-3, "{what}: {got} against {want}");
    }

    /// Asserts `point` lies inside `rect`, to within a hundredth of a pixel on
    /// each edge.
    fn assert_inside(rect: Rect, point: (f32, f32), what: &str) {
        let slack = 0.01;
        assert!(
            point.0 >= rect.x - slack
                && point.1 >= rect.y - slack
                && point.0 <= rect.x + rect.width + slack
                && point.1 <= rect.y + rect.height + slack,
            "{what}: {point:?} is inside {rect:?}"
        );
    }

    /// Asserts `got` and `want` agree to within `share` of `want`, relatively.
    ///
    /// [`assert_close`]'s thousandth of a pixel is the wrong instrument for the
    /// sagitta: that number is 0.25 px and is arrived at by cancelling two
    /// distances of about a hundred, so it is read to three or four significant
    /// figures rather than to a fixed absolute place.
    fn assert_near(got: f32, want: f32, share: f32, what: &str) {
        assert!(
            (got - want).abs() <= want.abs() * share,
            "{what}: {got} against {want}, within {share} of it"
        );
    }

    /// Asserts `points` describe a convex polygon the renderer's convex fan draws
    /// exactly: every turn round the edge goes the same way.
    ///
    /// [`DrawCommand::Polygon`] documents that the fan is exact for a convex
    /// polygon and produces overlapping and outside triangles for a concave one, so
    /// this is the precondition for the primitive at all — and it is a property of
    /// the *order the points were given in*, not of the shape they describe. A
    /// band segment whose inner corners were placed at a negative radius would still
    /// have four corners at two radii; it would be a bow tie, and this is what says
    /// so.
    fn assert_convex(points: &[(f32, f32)], what: &str) {
        assert!(
            points.len() >= 3,
            "{what}: fewer than three points enclose no area"
        );
        let mut left = 0;
        let mut right = 0;
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
            "{what}: {} turns one way and {right} the other, so the fan overlaps \
             itself",
            left
        );
    }

    /// Which way round `points` turns at its first corner that turns at all: `1`
    /// for one way, `-1` for the other.
    ///
    /// It is a number rather than an assertion because **the winding is a property
    /// of the whole band**, not of one segment: whether any single quad is convex is
    /// [`assert_convex`]'s question, and whether the 33 of them agree about which
    /// way round is convex is this one's. A band whose segments disagreed would
    /// still be a correct picture, and it would be a correct picture by accident.
    fn winding(points: &[(f32, f32)]) -> i32 {
        (0..points.len())
            .map(|index| {
                let a = points[index];
                let b = points[(index + 1) % points.len()];
                let c = points[(index + 2) % points.len()];
                (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
            })
            .find(|cross| *cross != 0.0)
            .map_or(0, |cross| if cross > 0.0 { 1 } else { -1 })
    }

    /// How far a segment's outer edge falls short of the arc it stands in for, in
    /// pixels: the **sagitta**, measured off the two recorded corners.
    ///
    /// The distance from the dial's centre to the straight line between the
    /// segment's two outer corners is `outer · cos(step/2)`, and the arc's own
    /// radius is `outer`, so the difference is the sagitta — read here as
    /// `outer − distance`, and so as a positive number that a band of exactly
    /// right thickness and no chord at all would answer as zero.
    fn sagitta(rect: Rect, quad: &[(f32, f32)]) -> f32 {
        let (cx, cy) = centre(rect);
        let a = (quad[1].0 - cx, quad[1].1 - cy);
        let b = (quad[2].0 - cx, quad[2].1 - cy);
        let outer = (a.0 * a.0 + a.1 * a.1).sqrt();
        // The chord's midpoint, which is its own closest point to the centre
        // because the two corners are the same distance from it.
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        outer - (mid.0 * mid.0 + mid.1 * mid.1).sqrt()
    }

    /// The same measurement on the band's **inner** edge, which is the other
    /// chord and the one a thicker band moves.
    ///
    /// It is positive in the same way: the inner edge is a chord of the inner
    /// circle, and the band's material reaches no further in than the chord does,
    /// so the drawn band is up to this much *narrower* than the one asked for at
    /// the middle of each segment. The sign is the same as the outer edge's for the
    /// same reason — a chord is inside its arc on both sides.
    fn inner_sagitta(rect: Rect, quad: &[(f32, f32)]) -> f32 {
        let (cx, cy) = centre(rect);
        let a = (quad[0].0 - cx, quad[0].1 - cy);
        let b = (quad[3].0 - cx, quad[3].1 - cy);
        let inner = (a.0 * a.0 + a.1 * a.1).sqrt();
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        inner - (mid.0 * mid.0 + mid.1 * mid.1).sqrt()
    }

    /// The sagitta the closed form gives for a band of `outer` radius cut into
    /// `step`-degree segments: `outer · (1 − cos(step/2))`.
    ///
    /// Derived from the definition rather than remembered, because the number a
    /// test asserts against must not be the number the code produced —
    /// expectations remembered instead of derived — and a sagitta is exactly the
    /// sort of thing an author writes down from a calculator once and then stops
    /// believing.
    fn sagitta_of(outer: f32, step: f32) -> f32 {
        outer * (1.0 - (step / 2.0).to_radians().cos())
    }

    /// The angle swept clockwise from `from` to `to`, in degrees.
    ///
    /// Wrapped, because a gauge's default arc crosses the top of the dial: the
    /// angle after three o'clock is a *small* number and the one before it is a
    /// *large* one, so the plain difference of two consecutive points on a band is
    /// negative for every one of them on the right of the dial. The module's own
    /// angles are read the same way by [`angle_of`].
    fn clockwise(from: f32, to: f32) -> f32 {
        (to - from).rem_euclid(FULL_TURN)
    }

    /// The gaps between consecutive points, as angles in degrees.
    ///
    /// Measured off the recorded corners rather than off the widget's own step,
    /// because "the band is cut evenly" is a claim about the picture and this is the
    /// question that asks it.
    fn gaps(rect: Rect, points: &[(f32, f32)]) -> Vec<f32> {
        points
            .windows(2)
            .map(|pair| clockwise(angle_of(rect, pair[0]), angle_of(rect, pair[1])))
            .collect()
    }

    /// The angle swept from the first of `points` to the last, in degrees.
    fn span(rect: Rect, points: &[(f32, f32)]) -> f32 {
        if points.len() < 2 {
            return 0.0;
        }
        clockwise(
            angle_of(rect, points[0]),
            angle_of(rect, points[points.len() - 1]),
        )
    }

    /// The arc radius a gauge computes for `rect`, exposed to the tests because
    /// every containment and every band's own radii are stated in terms of it.
    fn radius_of(rect: Rect) -> f32 {
        (rect.width.min(rect.height) / 2.0 - ARC_THICKNESS / 2.0).max(0.0)
    }

    /// How many quads a 270-degree arc is cut into at the defaults, from
    /// [`MAX_DEGREES_PER_SEGMENT`]: `ceil(270 / 8.25)` = `ceil(32.727)` = 33, of
    /// 8.1818° each.
    ///
    /// Asserted as a number rather than derived inside each test that wants it,
    /// because it is the *cost* and a cost is a claim: "at least 30 quads" would
    /// pass on a band twice as long, and a longer band is a slower frame that no
    /// capture and no draw-command count would notice.
    const RECT_TRACK_SEGMENTS: usize = 33;

    #[test]
    fn a_gauge_holds_the_properties_the_task_gives_it() {
        let (nodes, gauge) = gauge();
        assert!(
            nodes.get(gauge.handle()).is_some(),
            "with its node in the arena"
        );
        assert_eq!(gauge.value.get(), 0.0, "the value starts at the minimum");
        assert_eq!(gauge.shown.get(), 0.0, "and so does what is drawn");
        assert_eq!(gauge.min(), 0.0);
        assert_eq!(gauge.max(), 240.0);
        assert_eq!(gauge.gauge_type(), GaugeType::Arc, "and it is an arc");
        assert_eq!(
            (gauge.start_angle(), gauge.end_angle()),
            (135.0, 45.0),
            "opening at seven-thirty and closing at half-past-four"
        );
    }

    #[test]
    fn the_default_geometry_is_the_numbers_this_module_documents() {
        // These are constants, not derived numbers, and the band's two radii, the
        // needle's reach, the segment count and the module's own cost figure all
        // hang off them.
        let (_nodes, gauge) = gauge();
        assert_eq!(gauge.thickness(), ARC_THICKNESS);
        assert_eq!(ARC_THICKNESS, 14.0);
        assert_eq!(DEFAULT_SIZE, 200.0);
        assert_eq!(FULL_TURN, 360.0);
        assert_eq!(gauge.ticks(), DEFAULT_TICKS);
        assert_eq!(DEFAULT_TICKS, 11);
        assert_eq!(TICK_LENGTH, 8.0);
        assert_eq!(TICK_THICKNESS, 2.0);
        assert_eq!(NEEDLE_LENGTH_FRACTION, 0.8);
        assert_eq!(NEEDLE_BASE_FRACTION, 0.125);
        assert_eq!(NEEDLE_HUB_RADIUS, 6.0);
        assert_eq!(
            MAX_DEGREES_PER_SEGMENT, 8.25,
            "the widest step, and the only number the tessellation is built from"
        );
        assert_eq!(
            MIN_ARC_SEGMENTS, 8,
            "a floor for a sweep too short to divide"
        );
    }

    #[test]
    fn a_gauge_asks_for_a_square() {
        // A gauge has no content to measure, so this is the one number that
        // decides how big it is by default — and it is square because a dial is.
        let (_nodes, gauge) = gauge();
        let size = gauge.size();
        assert_eq!((size.width, size.height), (DEFAULT_SIZE, DEFAULT_SIZE));
        assert_eq!(size.width, size.height);
    }

    #[test]
    fn the_palette_is_the_themes_border_primary_muted_and_text() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            let color = |token| theme.get(token).as_color().unwrap();
            assert_eq!(
                palette.track,
                color(ThemeToken::Border),
                "the track is the part that is not the value"
            );
            assert_eq!(palette.fill, color(ThemeToken::Primary));
            assert_eq!(
                palette.tick,
                color(ThemeToken::TextMuted),
                "a mark is present but not emphasised"
            );
            assert_eq!(
                palette.needle,
                color(ThemeToken::Text),
                "the needle is legible on the track and on the fill"
            );
        }
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        // A theme switch has to reach the dials, or the theme would only change
        // half the window.
        assert_ne!(
            Palette::from_theme(&Theme::dark()),
            Palette::from_theme(&Theme::light())
        );
    }

    #[test]
    fn a_theme_that_holds_the_wrong_kind_of_value_still_answers() {
        // A token written with the wrong variant is a caller error, and it must
        // not take the frame down.
        let theme = Theme::dark();
        theme.set(
            ThemeToken::Primary,
            crate::theme::PropertyValue::Text("x".to_string()),
        );
        assert_eq!(
            Palette::from_theme(&theme).fill,
            Color::new(0, 0, 0, 255),
            "black rather than a panic"
        );
    }

    #[test]
    fn a_gauge_that_has_never_been_themed_paints_the_neutral_defaults() {
        // A caller who never themes a gauge still gets something visible, which is
        // the default palette's job.
        let mut nodes = Arena::new();
        let gauge = Gauge::new(&mut nodes, 0.0, 240.0);
        gauge.value.set(240.0);
        gauge.snap_to_state();
        assert_eq!(gauge.track.get(), Palette::default().track);
        let commands = gauge.paint(RECT);
        assert_eq!(
            polygon_colors(&commands).first().copied(),
            Some(Palette::default().track),
            "and the track's first segment is in the neutral default"
        );
    }

    #[test]
    fn setting_the_palette_leaves_the_gauge_where_it_is() {
        // A theme switch is animated by the caller setting the palette and
        // animating toward it; a palette that moved the gauge on its own would make
        // every theme switch instantaneous.
        let (_nodes, mut gauge) = gauge();
        let before = gauge.fill.get();
        gauge.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(gauge.fill.get(), before);
        assert_eq!(gauge.palette(), Palette::from_theme(&Theme::light()));
    }

    #[test]
    fn snapping_puts_a_themed_gauge_where_its_theme_says_at_once() {
        // The failure this guards: a gauge given a palette but never aimed still
        // paints the neutral defaults `Gauge::new` wrote, so a themed gauge starts
        // out grey and only becomes the theme's colour once something has moved it.
        // Built from `new` rather than through the `gauge` helper, which snaps.
        let mut nodes = Arena::new();
        let mut gauge = Gauge::new(&mut nodes, 0.0, 240.0);
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(themed.track, gauge.track.get(), "so this can fail");

        gauge.set_palette(themed);
        gauge.snap_to_state();

        assert_eq!(gauge.track.get(), themed.track);
        assert_eq!(gauge.fill.get(), themed.fill);
        assert_eq!(gauge.tick.get(), themed.tick);
        assert_eq!(gauge.needle.get(), themed.needle);
        assert!(!gauge.is_animating(), "a snap is not a transition");
    }

    #[test]
    fn snapping_a_mid_transition_gauge_back_out_ends_the_transition() {
        // Without the `clear`, a transition already running writes over what the
        // snap just set when it arrives, and the needle drifts off again.
        let (_nodes, gauge) = gauge();
        gauge.value.set(240.0);
        gauge.animate_to_state(motion());
        assert!(tick(&gauge, 10), "the needle is animating");

        gauge.value.set(0.0);
        gauge.snap_to_state();
        assert_eq!(gauge.shown.get(), 0.0, "the snap put it at the new value");

        assert!(!tick(&gauge, 500), "and nothing arrives afterwards");
        assert_eq!(gauge.shown.get(), 0.0);
    }

    #[test]
    fn a_value_written_without_an_aim_does_not_move_the_drawn_value() {
        // The truth and the drawn value are two properties on purpose, and this is
        // the half of the contract where they disagree: writing `value` is not an
        // instruction to draw it *now*.
        let (_nodes, gauge) = gauge();
        gauge.value.set(120.0);
        assert_eq!(gauge.style().shown, 120.0, "the target is the new value");
        assert_eq!(gauge.shown.get(), 0.0, "but the needle has not moved");
        assert_eq!(
            gauge.paint(RECT).len(),
            44,
            "and the fill is still empty, so the paint is the track and its ticks"
        );
    }

    #[test]
    fn a_value_below_the_minimum_clamps_to_the_minimum_rather_than_wrapping() {
        // `bounded` is two comparisons, so a caller reporting a negative temperature
        // gets an empty dial. Wrapping would have put it in the middle of the arc.
        let (_nodes, gauge) = gauge();
        for (written, drawn) in [(-40.0, 0.0), (0.0, 0.0), (240.0, 240.0), (600.0, 240.0)] {
            gauge.value.set(written);
            assert_eq!(
                gauge.style().shown,
                drawn,
                "{written} is clamped to {drawn}"
            );
            assert_eq!(gauge.fraction(written), drawn / 240.0);
        }
    }

    #[test]
    fn a_value_above_the_maximum_ends_the_arc_rather_than_over_filling_it() {
        // The promise, at the end that matters: a full value's fill must be the
        // track's own band and not one segment more.
        let (_nodes, gauge) = showing(0.0, 240.0, 999.0);
        let commands = gauge.paint(RECT);
        assert_eq!(
            band(&commands, gauge.fill.get()).len(),
            RECT_TRACK_SEGMENTS,
            "a reading past the top fills exactly what a reading at the top fills"
        );
    }

    #[test]
    fn a_value_of_nan_is_an_empty_dial_rather_than_a_broken_one() {
        // A caller that has lost track of its arithmetic hands over `NaN`, and
        // `f32::max` returns the other operand, so the clamp turns it into the low
        // end rather than propagating it into every coordinate of every circle.
        let (_nodes, gauge) = showing(0.0, 240.0, f32::NAN);
        assert_eq!(gauge.style().shown, 0.0);
        let commands = gauge.paint(RECT);
        assert!(
            reach(&commands, gauge.fill.get()).is_empty(),
            "no fill at all"
        );
        assert_eq!(commands.len(), 44, "a track and its eleven marks");
        for (center, radius) in circles(&commands) {
            assert!(
                center.0.is_finite() && center.1.is_finite(),
                "no circle has a NaN centre: {center:?}"
            );
            assert_eq!(radius, ARC_THICKNESS / 2.0);
        }
    }

    #[test]
    fn a_drawn_value_written_past_the_range_ends_at_the_range_end() {
        // `shown` is public, so the clamp inside `fraction` is the only thing
        // between a caller's own arithmetic and an arc longer than its own track.
        let (_nodes, gauge) = gauge();
        gauge.shown.set(480.0);
        let commands = gauge.paint(RECT);
        assert_eq!(
            band(&commands, gauge.fill.get()).len(),
            RECT_TRACK_SEGMENTS,
            "the drawn value is clamped, not wrapped"
        );
        gauge.shown.set(-240.0);
        assert!(
            reach(&gauge.paint(RECT), gauge.fill.get()).is_empty(),
            "and below the bottom there is no fill at all"
        );
    }

    #[test]
    fn the_two_ends_of_a_range_are_ordered_rather_than_taken_as_given() {
        // A caller that passes them the wrong way round gets a gauge rather than a
        // division by a negative span.
        let mut nodes = Arena::new();
        let gauge = Gauge::new(&mut nodes, 240.0, 0.0);
        assert!(
            nodes.get(gauge.handle()).is_some(),
            "with its node in the arena"
        );
        assert_eq!(gauge.min(), 0.0);
        assert_eq!(gauge.max(), 240.0);
        assert_eq!(
            gauge.value.get(),
            0.0,
            "and it starts at the ordered minimum"
        );
    }

    #[test]
    fn a_range_whose_ends_are_the_same_number_has_one_position() {
        // The degenerate case the slider answers the same way: a range of one value
        // has no distance along the arc to divide by, so everything maps to the
        // start rather than to a `NaN` centre.
        let (_nodes, gauge) = showing(50.0, 50.0, 50.0);
        assert_eq!(gauge.fraction(50.0), 0.0);
        assert!(reach(&gauge.paint(RECT), gauge.fill.get()).is_empty());
        assert_eq!(gauge.style().shown, 50.0, "and the value is still reported");
    }

    #[test]
    fn a_range_of_two_nan_bounds_draws_an_empty_dial_rather_than_panicking() {
        let mut nodes = Arena::new();
        let gauge = Gauge::new(&mut nodes, f32::NAN, f32::NAN);
        gauge.snap_to_state();
        let commands = gauge.paint(RECT);
        assert!(
            commands.iter().all(|c| match c {
                DrawCommand::Circle { center, .. } => center.0.is_finite() && center.1.is_finite(),
                DrawCommand::Line { start, end, .. } => {
                    start.0.is_finite() && start.1.is_finite() && end.0.is_finite()
                }
                _ => true,
            }),
            "nothing with a NaN coordinate reaches the recorder"
        );
        assert_eq!(gauge.fraction(1.0), 0.0);
    }

    #[test]
    fn setting_a_range_pulls_the_value_back_inside_it_at_once() {
        // A range that has just changed has no transition to run, and a needle
        // travelling into a track that has not been drawn yet is a frame of
        // nonsense.
        let (_nodes, mut gauge) = half();
        gauge.animate_to_state(motion());
        tick(&gauge, 50);
        assert!(gauge.shown.get() > 0.0, "it was travelling");

        gauge.set_range(0.0, 100.0);
        assert_eq!(
            gauge.value.get(),
            100.0,
            "120 on a 0-to-100 dial is its top"
        );
        assert_eq!(gauge.shown.get(), 100.0, "and the needle is there now");
        assert!(!gauge.is_animating(), "not travelling there");
    }

    #[test]
    fn setting_a_range_keeps_a_value_that_is_still_inside_it() {
        let (_nodes, mut gauge) = half();
        gauge.set_range(0.0, 480.0);
        assert_eq!(gauge.value.get(), 120.0, "still where it was");
        assert_eq!(gauge.shown.get(), 120.0);
    }

    #[test]
    fn the_arc_is_a_band_of_quads_whose_corners_are_on_the_two_radii() {
        // **The test that kills the chain of circles.** It is here because no
        // assertion of the old kind could exist: a chain of circles has no corner
        // points, so there was nothing to ask "is this on the outer radius" about,
        // and the only questions available — how many commands, are the circles
        // tangent, is the centre on the arc's radius — were all true of a band
        // that was 5.6 px beaded.
        //
        // So it reads the four points of every recorded quad back and asks where
        // they are. A band whose corners are on `R ± thickness/2` has the
        // thickness that was asked for with no error in it; a band built any other
        // way has a number of the right commands and a picture of the wrong width.
        let (_nodes, gauge) = half();
        let commands = gauge.paint(RECT);
        let quads = band(&commands, gauge.track.get());
        assert_eq!(
            quads.len(),
            RECT_TRACK_SEGMENTS,
            "33 quads, one per segment"
        );

        let radius = radius_of(RECT);
        let half = ARC_THICKNESS / 2.0;
        assert_eq!(radius, 93.0, "100 across less half the thickness");
        let outer = radius + half;
        let inner = radius - half;
        assert_eq!((outer, inner), (100.0, 86.0), "the band's two radii");

        // `assert_close` and `assert_near` take their label as a `&str` rather than
        // as format arguments, so each segment's own label is formatted once here.
        for (index, quad) in quads.iter().enumerate() {
            assert_eq!(quad.len(), 4, "segment {index} is a quad");
            assert_close(
                from_centre(RECT, quad[0]),
                inner,
                &format!("segment {index}: the first corner is on the inner radius"),
            );
            assert_close(
                from_centre(RECT, quad[1]),
                outer,
                &format!("segment {index}: the second is on the outer"),
            );
            assert_close(
                from_centre(RECT, quad[2]),
                outer,
                &format!("segment {index}: and so is the third"),
            );
            assert_close(
                from_centre(RECT, quad[3]),
                inner,
                &format!("segment {index}: and the fourth is back on the inner"),
            );
            // The two corners at each end are at the *same angle*, which is what
            // makes the band `thickness` wide rather than merely four points near
            // two radii: a quad whose inner corner was at a different angle would
            // be a wedge, and a caller asking for a 14-pixel band would get a
            // thirteen-pixel band with one end flared.
            assert_close(
                angle_of(RECT, quad[0]),
                angle_of(RECT, quad[1]),
                &format!("segment {index}: the two corners at its start share an angle"),
            );
            assert_close(
                angle_of(RECT, quad[2]),
                angle_of(RECT, quad[3]),
                &format!("segment {index}: and so do the two at its end"),
            );
            assert_convex(quad, &format!("segment {index}"));
        }
    }

    #[test]
    fn every_segment_is_wound_the_same_way_round() {
        // The renderer's fan is a fan from the polygon's first point, and it is
        // exact for a convex polygon. **Convexity here is a property of the order
        // the four corners were given in**, not of the two circles they sit on, so
        // "each quad is convex" and "all the quads agree about which way round is
        // convex" are two different questions — and only the second one says the
        // band is one shape rather than 33 shapes that happen to fit together.
        let (_nodes, gauge) = half();
        let quads = band(&gauge.paint(RECT), gauge.track.get());
        assert_eq!(quads.len(), RECT_TRACK_SEGMENTS);

        let mut windings = quads.iter().map(|quad| winding(quad));
        let first = windings.next().expect("a band of 33 quads has a first one");
        assert_ne!(
            first, 0,
            "a segment's four corners do not all lie on one line"
        );
        for (index, this) in windings.enumerate() {
            assert_eq!(
                this,
                first,
                "segment {} is wound the other way round from the first, and the \
                 fan needs them to agree",
                index + 1
            );
        }
    }

    #[test]
    fn the_sagitta_of_the_band_is_within_the_bound_the_step_allows() {
        // The one error the band has, bounded by a number **derived from the
        // constants rather than remembered**: a segment's outer edge is a chord of
        // the outer circle, so it falls short of the arc by `outer·(1 − cos(step/2))`,
        // and the widest step allowed is `MAX_DEGREES_PER_SEGMENT`.
        //
        // Asserted on the recorded corners, so the number being checked is the one
        // that will be drawn. The chain this replaced was 5.77 px of scallop on the
        // same band; a `Path` would have been 6.7 px thin before any chord at all.
        let (_nodes, gauge) = half();
        let radius = radius_of(RECT);
        let outer = radius + ARC_THICKNESS / 2.0;
        let segments = gauge.arc_segment_count(radius, 270.0);
        let quads = band(&gauge.paint(RECT), gauge.track.get());
        let step = 270.0 / f32::from(segments);
        let bound = sagitta_of(outer, MAX_DEGREES_PER_SEGMENT);
        let expected = sagitta_of(outer, step);
        assert_near(step, MAX_DEGREES_PER_SEGMENT, 0.01, "the step actually cut");

        for (index, quad) in quads.iter().enumerate() {
            let measured = sagitta(RECT, quad);
            assert!(
                measured <= bound,
                "segment {index} sags {measured} px, over the {bound} px the \
                 widest allowed step gives"
            );
            assert_near(
                measured,
                expected,
                0.01,
                &format!("segment {index}'s own sagitta against the closed form"),
            );
        }
        // Positive, because a band with no chord at all would be a band of arcs and
        // this primitive cannot draw one. A zero here would mean the measurement
        // had stopped measuring anything.
        assert!(
            sagitta(RECT, &quads[0]) > 0.0,
            "the outer edge is a straight line and does fall short of the arc"
        );
    }

    #[test]
    fn the_sagitta_grows_with_the_square_of_the_step() {
        // **The step² signature, and the reason a coarse tessellation is safe.**
        // The error of a chord is second order in the angle it subtends, so an
        // eightfold coarser band is not eight times wrong — it is sixty-four times
        // wrong, and a band eight times finer is a sixty-fourth of the error. A
        // first-order error would grow with the step itself, and this is what tells
        // the two apart: 8 is the linear prediction and 64 is the quadratic one.
        //
        // Both arcs are the same dial and the same thickness, both steps are
        // measured off the recorded corners rather than taken from the widget, and
        // both sagittas are large enough to be read out of an `f32` — a one-degree
        // arc's 0.00006 px is 0.01 of an ulp at a radius of 100, and a law asserted
        // on a number the arithmetic cannot hold is a law about nothing.
        let sagitta_of_sweep = |sweep: f32| {
            let (_nodes, mut gauge) = half();
            gauge.set_angles(135.0, 135.0 + sweep);
            let quads = band(&gauge.paint(RECT), gauge.track.get());
            let step = {
                let corners: Vec<(f32, f32)> = quads.iter().map(|quad| quad[1]).collect();
                gaps(RECT, &corners).into_iter().fold(0.0_f32, f32::max)
            };
            let measured = quads
                .iter()
                .map(|quad| sagitta(RECT, quad))
                .fold(0.0, f32::max);
            (step, measured)
        };
        let (coarse_step, coarse) = sagitta_of_sweep(64.0);
        let (fine_step, fine) = sagitta_of_sweep(8.0);
        assert_close(
            coarse_step,
            8.0,
            "64 degrees over the floor's eight segments",
        );
        assert_close(fine_step, 1.0, "and 8 degrees over the same eight");
        // The closed form for each, and the measurement against it: this is what
        // says the measurement is of the chord and not of the arithmetic's noise.
        let outer = radius_of(RECT) + ARC_THICKNESS / 2.0;
        assert_near(
            coarse,
            sagitta_of(outer, coarse_step),
            0.01,
            "the coarse one",
        );
        assert_near(fine, sagitta_of(outer, fine_step), 0.01, "the fine one");

        let ratio = coarse / fine;
        let linear = coarse_step / fine_step;
        let quadratic = linear * linear;
        assert!(
            (ratio - quadratic).abs() < quadratic * 0.02,
            "the eightfold coarser band sags {ratio} times as much, and the square \
             of the step says {quadratic} where a first-order error would say \
             {linear}"
        );
        assert!(
            (ratio - linear).abs() > linear,
            "and it is nowhere near the linear prediction, or this proves nothing"
        );
        // The fine one is a four-hundredth of a pixel, which is the other half of
        // the claim: the floor does not make a short arc a coarse one.
        assert!(
            fine < 0.01,
            "an eight-degree arc is {fine} px of error, not a visible one"
        );
    }

    #[test]
    fn the_segment_count_is_derived_from_the_sweep_and_the_named_maximum() {
        // The retune the chain of circles forced. A chain's length came out of the
        // dial's size and its stroke, so it needed a ceiling; a band's comes out of
        // the sweep alone, and the two bounds on it are named constants.
        let (_nodes, gauge) = half();
        let radius = radius_of(RECT);
        let count = |gauge: &Gauge, sweep: f32| gauge.arc_segment_count(radius, sweep);

        assert_eq!(
            usize::from(count(&gauge, 270.0)),
            RECT_TRACK_SEGMENTS,
            "ceil(270 / 8.25) = 33"
        );
        assert_eq!(
            usize::from(count(&gauge, FULL_TURN)),
            44,
            "a full turn is 44, which is what keeps a circle from looking like one"
        );
        // A sweep that is not a whole number of steps rounds **up**, so no segment
        // is ever wider than the maximum: 32.7 rounds to 33, and 33.1 to 34.
        for sweep in [1.0, 8.25, 8.3, 33.0, 100.0, 269.0] {
            let segments = count(&gauge, sweep);
            let step = sweep / f32::from(segments);
            assert!(
                step <= MAX_DEGREES_PER_SEGMENT,
                "{sweep} degrees over {segments} segments is a {step} step, over \
                 the maximum"
            );
            assert!(
                segments >= MIN_ARC_SEGMENTS,
                "{sweep} degrees is at least the floor"
            );
        }
        // And the count does not follow the thickness, which is the other half of
        // what the chain used to decide: a hairline arc is no longer a different
        // number of quads from a thick one.
        let (_nodes, mut thin) = half();
        thin.set_thickness(0.5);
        assert_eq!(
            count(&thin, 270.0),
            count(&gauge, 270.0),
            "a hairline arc is cut into the same number of pieces"
        );
    }

    #[test]
    fn a_degenerate_sweep_or_thickness_asks_for_no_segments_at_all() {
        // The guard, asked of the function that does the arithmetic rather than of
        // `paint`, which refuses the same two values one layer up — a caller
        // reaching this directly gets nothing rather than a division by a step of
        // nothing. The floor is why the guard has to be *before* the floor: eight
        // quads over a sweep of nothing is a band drawn eight times on top of
        // itself at one angle.
        let mut nodes = Arena::new();
        let mut degenerate = Gauge::new(&mut nodes, 0.0, 240.0);
        let radius = radius_of(RECT);
        for (what, thickness, radius_here, sweep) in [
            ("no sweep", ARC_THICKNESS, radius, 0.0),
            ("no thickness", 0.0, radius, 270.0),
            ("both", 0.0, radius, 0.0),
            ("no radius", ARC_THICKNESS, 0.0, 270.0),
            ("a NaN sweep", ARC_THICKNESS, radius, f32::NAN),
        ] {
            degenerate.set_thickness(thickness);
            assert_eq!(
                degenerate.arc_segment_count(radius_here, sweep),
                0,
                "{what} draws nothing rather than the floor's worth of quads"
            );
        }
        // The same two, through the widget, as an empty recorder or a band that is
        // not there: nothing panics and nothing is drawn.
        let (_nodes, mut zero_sweep) = half();
        zero_sweep.set_angles(135.0, 135.0);
        assert!(zero_sweep.paint(RECT).is_empty(), "a sweep of nothing");
        let (_nodes, mut no_thickness) = half();
        no_thickness.set_thickness(0.0);
        assert!(
            band(&no_thickness.paint(RECT), no_thickness.track.get()).is_empty(),
            "a thickness of nothing"
        );
    }

    #[test]
    fn a_one_degree_arc_is_a_band_rather_than_nothing() {
        // The other end of the floor: a sweep too short to divide is still an arc,
        // and the guard above is about a sweep of *zero*. The count is the floor's
        // eight, the sagitta is six hundredths of a thousandth of a pixel, and
        // nothing about the call panics on the way there.
        let (_nodes, mut gauge) = half();
        gauge.set_angles(135.0, 136.0);
        let commands = gauge.paint(RECT);
        let quads = band(&commands, gauge.track.get());
        assert_eq!(
            quads.len(),
            usize::from(MIN_ARC_SEGMENTS),
            "eight quads for one degree, and not zero"
        );
        for quad in &quads {
            assert_convex(quad, "a one-degree segment");
            assert_close(from_centre(RECT, quad[1]), 100.0, "on the outer radius");
        }
        assert_close(
            span(RECT, &reach(&commands, gauge.track.get())),
            1.0,
            "and it reaches the one degree it was asked for",
        );
    }

    #[test]
    fn a_hairline_arc_is_a_solid_band_rather_than_a_beaded_one() {
        // What the old `MAX_ARC_CIRCLES` ceiling cost, and the reason the constant
        // is gone: a chain of circles could not be covered by tangent circles
        // within a cap, so a hairline arc on a big dial was drawn with its circles
        // further apart than the arc was thick and read as a row of beads. The
        // band's error is a sagitta, so a thinner band is *more* accurate and costs
        // the same.
        let (_nodes, mut gauge) = half();
        gauge.set_thickness(0.5);
        // A thinner band leaves the arc's *outer* edge on the rect's own edge and
        // moves only the inner one, so the radii are 100 and 99.5 — the whole of
        // the band is half a pixel wide and the whole of the error is a quarter of
        // a pixel of chord.
        let radius = RECT.width.min(RECT.height) / 2.0 - 0.25;
        let quads = band(&gauge.paint(RECT), gauge.track.get());
        assert_eq!(quads.len(), RECT_TRACK_SEGMENTS, "the same 33 quads");
        for quad in &quads {
            assert_close(
                from_centre(RECT, quad[1]),
                radius + 0.25,
                "the outer corner is on this gauge's own outer radius",
            );
            assert_close(
                from_centre(RECT, quad[0]),
                radius - 0.25,
                "and the inner one on its inner radius",
            );
        }
        assert!(
            sagitta(RECT, &quads[0]) < 0.3,
            "and the chord is a quarter of a pixel, whatever the band's width: {}",
            sagitta(RECT, &quads[0])
        );
    }

    #[test]
    fn the_fill_is_the_track_s_own_band_over_the_value_s_share_of_the_sweep() {
        // Proportionality, asserted on the picture: the fill's last corner's angle
        // is the fraction of the sweep, so half a dial reaches 270°.
        let sweep = |gauge: &Gauge| (gauge.end_angle() - gauge.start_angle()).rem_euclid(FULL_TURN);
        let filled = |gauge: &Gauge| {
            reach(&gauge.paint(RECT), gauge.fill.get())
                .last()
                .copied()
                .map_or(0.0, |last| angle_of(RECT, last))
        };
        let (_nodes, at_rest) = gauge();
        assert_close(sweep(&at_rest), 270.0, "the default sweep");

        // The fill's reach is measured as a span from the arc's own start angle,
        // not as a difference of two angles: a fill that ends at half past four is
        // at 45 degrees, which is *90 less* than the 135 it started at, and the
        // wrapped difference is the 270 the test wants.
        let reached = |gauge: &Gauge| clockwise(135.0, filled(gauge));

        let (_nodes, quarter) = showing(0.0, 240.0, 60.0);
        assert_close(
            reached(&quarter),
            270.0 * 0.25,
            "a quarter of the range sweeps a quarter of the arc",
        );
        let (_nodes, half) = half();
        assert_close(reached(&half), 135.0, "and half reaches twelve o'clock");
        let (_nodes, full) = showing(0.0, 240.0, 240.0);
        assert_close(
            reached(&full),
            270.0,
            "and a full value ends where the track does",
        );
    }

    #[test]
    fn the_fill_is_proportional_to_the_value_across_the_whole_range() {
        // The acceptance criterion, stated as a sweep over every tenth rather than
        // as three points: the fill's reach grows evenly with the value, which is
        // what "proportional" means and what a `Path` with a fixed segment count
        // would quietly stop doing.
        for step in 1u8..=10 {
            let value = f32::from(step) * 24.0;
            let (_nodes, gauge) = showing(0.0, 240.0, value);
            let fill = reach(&gauge.paint(RECT), gauge.fill.get());
            let reached = span(RECT, &fill);
            assert_close(
                reached,
                270.0 * f32::from(step) / 10.0,
                "at {value} the fill has swept that much of the arc",
            );
        }
    }

    #[test]
    fn the_fill_never_reaches_outside_the_track_at_any_value() {
        // The promise, over every value a caller might write and every point of any
        // curve. **Every corner of the fill is on one of the track's own two
        // radii** — the band is cut by the same function from the same arc radius
        // and the same thickness — so the fill occupies the track's own annulus and
        // cannot leave it. It is the same argument the chain made about its
        // circles, and it is now checkable: a chain had no corners to ask about.
        let (_nodes, gauge) = gauge();
        let radius = radius_of(RECT);
        let outer = radius + ARC_THICKNESS / 2.0;
        let inner = radius - ARC_THICKNESS / 2.0;
        for step in 1u8..=25 {
            let value = f32::from(step) * 9.6;
            gauge.value.set(value);
            gauge.snap_to_state();
            let commands = gauge.paint(RECT);
            for (index, quad) in band(&commands, gauge.fill.get()).iter().enumerate() {
                // `assert_close` takes the label as a `&str` rather than as format
                // arguments, so the index is formatted into it here.
                let inner_label = format!("at {value} fill segment {index}'s inner corner");
                let outer_label = format!("at {value} fill segment {index}'s outer corner");
                for corner in [quad[0], quad[3]] {
                    assert_close(from_centre(RECT, corner), inner, &inner_label);
                }
                for corner in [quad[1], quad[2]] {
                    assert_close(from_centre(RECT, corner), outer, &outer_label);
                }
                assert!(
                    from_centre(RECT, quad[0]) >= inner - 1e-3
                        && from_centre(RECT, quad[1]) <= outer + 1e-3,
                    "at {value} the fill stays inside the track's own band"
                );
            }
            // And the fill's angular reach is the track's own, never more: an
            // overshooting curve cannot carry it past the end of the arc.
            let fill = reach(&commands, gauge.fill.get());
            let track = reach(&commands, gauge.track.get());
            assert!(
                span(RECT, &fill) <= span(RECT, &track) + 1e-3,
                "at {value} the fill's sweep is no longer than the track's"
            );
        }
    }

    #[test]
    fn a_full_gauge_draws_two_bands_that_land_on_the_same_corners() {
        let (_nodes, gauge) = showing(0.0, 240.0, 240.0);
        let commands = gauge.paint(RECT);
        let track = band(&commands, gauge.track.get());
        let fill = band(&commands, gauge.fill.get());
        assert_eq!(track.len(), RECT_TRACK_SEGMENTS);
        assert_eq!(
            fill.len(),
            RECT_TRACK_SEGMENTS,
            "a full fill is the whole arc"
        );
        for (index, (track_quad, fill_quad)) in track.iter().zip(fill.iter()).enumerate() {
            assert_eq!(
                track_quad, fill_quad,
                "and segment {index} of the two lands on the same four corners"
            );
        }
    }

    #[test]
    fn the_track_is_drawn_before_the_fill_so_the_fill_is_visible() {
        let (_nodes, gauge) = half();
        let commands = gauge.paint(RECT);
        assert_eq!(
            shapes(&commands).first().copied(),
            Some("band"),
            "the track's own band is first"
        );
        assert_eq!(
            polygon_colors(&commands).first().copied(),
            Some(gauge.track.get())
        );
        assert_eq!(
            polygon_colors(&commands)[RECT_TRACK_SEGMENTS],
            gauge.fill.get(),
            "and the fill's band begins immediately after the track's, over it"
        );
    }

    #[test]
    fn an_arc_gauge_leaves_a_ninety_degree_gap_at_the_bottom() {
        // The default's shape, asserted off the recorded corners: the band runs
        // from seven-thirty to half-past-four and the quarter it does not cover is
        // the quarter at the bottom. This is the thing a 360 default would break
        // and nothing else in the suite would notice.
        let (_nodes, gauge) = half();
        let track = reach(&gauge.paint(RECT), gauge.track.get());
        assert_close(
            angle_of(RECT, track[0]),
            135.0,
            "the first segment ends where the arc starts, at seven-thirty",
        );
        assert_close(
            angle_of(RECT, track[track.len() - 1]),
            45.0,
            "and the last ends at half-past-four",
        );
        let covered = span(RECT, &track);
        assert_close(covered, 270.0, "so 270 degrees of the 360 are covered");
        assert_close(FULL_TURN - covered, 90.0, "and the gap is the other 90");
    }

    #[test]
    fn a_circle_gauge_leaves_no_gap_at_all() {
        let (_nodes, mut gauge) = half();
        gauge.set_gauge_type(GaugeType::Circle);
        let track = reach(&gauge.paint(RECT), gauge.track.get());
        assert_close(span(RECT, &track), FULL_TURN, "a full turn of a band");
        assert_close(FULL_TURN - span(RECT, &track), 0.0, "so no gap at all");
    }

    #[test]
    fn a_circle_gauge_sweeps_a_full_turn_whatever_the_two_angles_say() {
        let (mut nodes, mut gauge) = half();
        gauge.set_gauge_type(GaugeType::Circle);
        gauge.set_angles(10.0, 20.0);
        assert_eq!(gauge.sweep(), FULL_TURN, "the mode overrides the pair");

        let mut arc = Gauge::new(&mut nodes, 0.0, 240.0);
        arc.set_angles(10.0, 20.0);
        assert_eq!(
            arc.sweep(),
            10.0,
            "while an arc really does sweep only the difference"
        );
    }

    #[test]
    fn the_three_gauge_types_draw_three_different_sets_of_shapes() {
        let kinds = |gauge_type| {
            let mut nodes = Arena::new();
            let mut gauge = Gauge::new(&mut nodes, 0.0, 240.0);
            gauge.set_gauge_type(gauge_type);
            gauge.value.set(120.0);
            gauge.snap_to_state();
            let commands = gauge.paint(RECT);
            (
                band(&commands, gauge.track.get()).len(),
                circles(&commands).len(),
                lines(&commands).len(),
                needle(&commands).is_some(),
            )
        };
        let (arc_quads, arc_circles, arc_lines, arc_poly) = kinds(GaugeType::Arc);
        let (circle_quads, circle_circles, circle_lines, circle_poly) = kinds(GaugeType::Circle);
        let (needle_quads, needle_circles, needle_lines, needle_poly) = kinds(GaugeType::Needle);

        assert!(!arc_poly, "an arc draws no needle");
        assert!(!circle_poly, "and neither does a full circle");
        assert!(needle_poly, "only the needle does");
        assert_eq!(
            (arc_lines, circle_lines, needle_lines),
            (11, 11, 11),
            "all three carry their tick marks"
        );
        assert_eq!(
            (arc_circles, circle_circles),
            (0, 0),
            "and none of them draws a circle: the band is polygons and the hub is \
             the needle's alone"
        );
        assert_eq!(needle_circles, 1, "the hub, and nothing else");
        assert_eq!(
            (arc_quads, circle_quads, needle_quads),
            (RECT_TRACK_SEGMENTS, 44, RECT_TRACK_SEGMENTS),
            "33 segments for a 270-degree track and 44 for a full turn, and the \
             needle's mode changes neither"
        );
    }

    #[test]
    fn the_needle_is_a_filled_triangle_and_a_hub() {
        // Requirement 5's "triangle or line", and the operator's choice of the
        // triangle on 2026-10-01: a line is a bar with two flat ends, and a
        // pointer with a blunt end reads as a dash. The hub is what covers the
        // base so the shape reads as a pointer rather than as a triangle.
        let (_nodes, mut gauge) = half();
        gauge.set_gauge_type(GaugeType::Needle);
        let commands = gauge.paint(RECT);
        let (points, color) = needle(&commands).expect("a needle draws a polygon");
        assert_eq!(points.len(), 3, "three points is the convex fan's own case");
        assert_convex(&points, "a needle");
        assert_eq!(color, gauge.needle.get());

        let circles = circles(&commands);
        let hub = circles.last().copied().expect("a hub circle");
        assert_eq!(hub.0, centre(RECT), "the hub is at the dial's own centre");
        assert_eq!(hub.1, NEEDLE_HUB_RADIUS);
    }

    #[test]
    fn the_needle_points_at_the_drawn_value_s_own_angle() {
        let (_nodes, mut gauge) = half();
        gauge.set_gauge_type(GaugeType::Needle);
        gauge.value.set(60.0);
        gauge.snap_to_state();
        let commands = gauge.paint(RECT);
        let (points, _) = needle(&commands).expect("a needle");
        let radius = radius_of(RECT);

        assert_close(
            from_centre(RECT, points[0]),
            radius * NEEDLE_LENGTH_FRACTION,
            "the tip reaches four fifths of the radius",
        );
        assert_close(
            angle_of(RECT, points[0]),
            135.0 + 270.0 * 0.25,
            "and it sits at a quarter of the sweep",
        );
        for base in &points[1..] {
            assert_close(
                from_centre(RECT, *base),
                radius * NEEDLE_BASE_FRACTION,
                "the base is a narrow collar about the hub",
            );
        }
    }

    #[test]
    fn the_needle_follows_the_drawn_value_rather_than_the_truth() {
        // Requirement 4's animation, asserted on the picture rather than on the
        // property: a needle that reached for the truth a frame before the arc
        // arrived would be a needle and an arc disagreeing about the same reading.
        // Built from `gauge()` rather than `half()`, because this test is about
        // the needle *starting* at the arc's own start: a fixture already showing
        // half would start at twelve o'clock and the three expectations below would
        // all be off by a quarter of the sweep.
        let (_nodes, mut gauge) = gauge();
        gauge.set_gauge_type(GaugeType::Needle);
        gauge.value.set(240.0);
        gauge.animate_to_state(motion());

        let tip_angle = |gauge: &Gauge| {
            let tip = needle(&gauge.paint(RECT)).expect("a needle").0[0];
            angle_of(RECT, tip)
        };
        let at_start = tip_angle(&gauge);
        tick(&gauge, 50);
        let halfway = tip_angle(&gauge);
        tick(&gauge, 50);
        let arrived = tip_angle(&gauge);

        assert_close(at_start, 135.0, "still at the start angle");
        assert_close(halfway, 270.0, "half way there at half the time");
        assert_close(
            arrived,
            45.0,
            "and on the end when it arrives, which reads as 45",
        );
        assert!(
            halfway > at_start && halfway < 405.0,
            "growing, not jumping"
        );
    }

    #[test]
    fn a_needle_that_overshoots_does_not_reach_past_the_end_of_its_arc() {
        // A spring passes its target on the way and comes back, so the drawn value
        // really does leave the range mid-flight. The drawn fraction is clamped, so
        // the needle stops at the end of the arc for the frames of the overshoot.
        let (_nodes, mut gauge) = half();
        gauge.set_gauge_type(GaugeType::Needle);
        gauge.value.set(240.0);
        gauge.animate_to_state(Motion {
            duration: ms(100),
            easing: Easing::Spring {
                damping: 2.0,
                stiffness: 100.0,
            },
        });

        let mut furthest = 0.0_f32;
        for _ in 0..20 {
            tick(&gauge, 10);
            furthest = furthest.max(gauge.shown.get());
            let (points, _) = needle(&gauge.paint(RECT)).expect("a needle");
            assert!(
                angle_of(RECT, points[0]) <= 45.0 + 1e-3,
                "the needle stayed at or before the end: {}",
                angle_of(RECT, points[0])
            );
        }
        assert!(
            furthest > 240.0,
            "and the curve really did overshoot, or this proves nothing: {furthest}"
        );
    }

    #[test]
    fn the_needle_s_tip_stays_inside_the_node_at_every_value() {
        // The gauge's other containment promise: the dial is inscribed in the rect
        // and the needle stops short of the track, so nothing the needle draws can
        // leave the node it was given.
        let (_nodes, mut gauge) = gauge();
        gauge.set_gauge_type(GaugeType::Needle);
        for step in 0u8..=20 {
            gauge.shown.set(f32::from(step) * 12.0);
            let commands = gauge.paint(WIDE);
            let (points, _) = needle(&commands).expect("a needle");
            for point in &points {
                assert_inside(WIDE, *point, "a needle point");
            }
            for (center, radius) in circles(&commands) {
                assert_inside(WIDE, center, "a circle's centre");
                assert!(
                    from_centre(WIDE, center) + radius <= WIDE.height / 2.0 + 0.01,
                    "and its outer edge is inside the shorter side"
                );
            }
            for (start, end, _) in lines(&commands) {
                assert_inside(WIDE, start, "a tick's outer end");
                assert_inside(WIDE, end, "and its inner end");
            }
        }
    }

    #[test]
    fn the_needle_is_not_drawn_in_the_other_two_types() {
        for gauge_type in [GaugeType::Arc, GaugeType::Circle] {
            let (_nodes, mut gauge) = half();
            gauge.set_gauge_type(gauge_type);
            let commands = gauge.paint(RECT);
            assert!(
                needle(&commands).is_none(),
                "{gauge_type:?} draws no needle"
            );
        }
    }

    #[test]
    fn the_tick_marks_are_the_count_that_was_asked_for() {
        for count in [0u8, 1, 5, 11, 40] {
            let (_nodes, mut gauge) = half();
            gauge.set_ticks(count);
            let commands = gauge.paint(RECT);
            assert_eq!(lines(&commands).len(), usize::from(count), "{count} marks");
            assert_eq!(
                line_colors(&commands).first().copied(),
                if count > 0 {
                    Some(gauge.tick.get())
                } else {
                    None
                },
                "in the tick colour, where there are any"
            );
        }
    }

    #[test]
    fn the_tick_marks_sit_inside_the_track_s_own_band() {
        // "Inside the track" read as the arc's inner edge: a mark runs inward from
        // where the band stops, so it is never drawn over the track's own colour
        // and never reaches past the dial's outside.
        let (_nodes, gauge) = half();
        let radius = radius_of(RECT);
        let outer = radius - ARC_THICKNESS / 2.0;
        let inner = outer - TICK_LENGTH;
        assert_close(outer, 86.0, "the band's inner edge on a 93-pixel radius");
        assert_close(inner, 78.0, "and eight pixels inward of it");

        for (start, end, width) in lines(&gauge.paint(RECT)) {
            assert_close(from_centre(RECT, start), outer, "a mark's outer end");
            assert_close(from_centre(RECT, end), inner, "and its inner end");
            assert_eq!(width, TICK_THICKNESS);
        }
    }

    #[test]
    fn the_tick_marks_sit_at_the_arc_s_two_ends_and_evenly_between() {
        // Eleven marks on ten intervals, the first and the last on the arc's own
        // ends: a dial whose extremes have no mark on them cannot be read at its
        // own extremes.
        let (_nodes, gauge) = half();
        let marks = lines(&gauge.paint(RECT));
        assert_eq!(marks.len(), 11);
        assert_close(
            angle_of(RECT, marks[0].0),
            135.0,
            "the first mark is at the arc's start",
        );
        assert_close(angle_of(RECT, marks[10].0), 45.0, "and the last at its end");
        let angles: Vec<f32> = marks
            .iter()
            .map(|(start, _, _)| angle_of(RECT, *start))
            .collect();
        for index in 1..angles.len() {
            assert_close(
                clockwise(angles[index - 1], angles[index]),
                270.0 / 10.0,
                "ten even intervals of 27 degrees, measured clockwise so the mark \
                 at twelve o'clock is not a negative one",
            );
        }
    }

    #[test]
    fn turning_the_ticks_off_leaves_the_arc_alone() {
        // Ticks are decoration. A gauge with none is the same gauge, and the
        // difference is exactly the lines.
        let (_nodes, mut gauge) = half();
        let with = gauge.paint(RECT);
        gauge.set_ticks(0);
        let without = gauge.paint(RECT);
        assert_eq!(with.len() - without.len(), 11, "eleven marks fewer");
        // Counted as the two bands of one colour each, so the needle's triangle
        // cannot stand in for one of them and make this pass.
        assert_eq!(
            band(&with, gauge.track.get()).len() + band(&with, gauge.fill.get()).len(),
            band(&without, gauge.track.get()).len() + band(&without, gauge.fill.get()).len(),
            "and the same two bands, segment for segment"
        );
        assert!(lines(&without).is_empty());
        assert_eq!(gauge.ticks(), 0);
    }

    #[test]
    fn turning_the_ticks_on_writes_the_tick_colour_at_once() {
        // The same rule the indeterminate bar follows for its sliding position: a
        // mark about to appear is drawn in the current palette on its first frame,
        // not in whatever the colour was before the caller had one.
        let (_nodes, mut gauge) = gauge();
        // The gauge arrives through the `gauge` helper, which snapped the dark
        // theme's palette into the properties — so the colour at rest is the *dark*
        // theme's tick, and neither the neutral default nor the light theme's.
        let dark_tick = Palette::from_theme(&Theme::dark()).tick;
        assert_ne!(dark_tick, Palette::default().tick, "or this could not fail");
        gauge.set_ticks(0);
        gauge.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(
            gauge.tick.get(),
            dark_tick,
            "nothing drew the ticks, so nothing updated them"
        );

        gauge.set_ticks(7);
        assert_eq!(
            gauge.tick.get(),
            Palette::from_theme(&Theme::light()).tick,
            "and the first frame with marks on it is that palette's"
        );
        assert_eq!(
            line_colors(&gauge.paint(RECT))[0],
            Palette::from_theme(&Theme::light()).tick
        );
    }

    #[test]
    fn turning_the_needle_on_writes_the_needle_colour_at_once() {
        let (_nodes, mut gauge) = gauge();
        gauge.set_palette(Palette::from_theme(&Theme::light()));
        gauge.set_gauge_type(GaugeType::Needle);
        assert_eq!(
            gauge.needle.get(),
            Palette::from_theme(&Theme::light()).needle,
            "the needle's first frame is in the current palette"
        );
    }

    #[test]
    fn a_negative_thickness_is_no_thickness() {
        // A negative circle radius is a rect of negative width: a quad of nothing
        // drawn inside out, which is worse than drawing nothing.
        let (_nodes, mut gauge) = half();
        gauge.set_thickness(-20.0);
        assert_eq!(
            gauge.thickness(),
            0.0,
            "clamped rather than drawn inside out"
        );
        let commands = gauge.paint(RECT);
        assert!(
            reach(&commands, gauge.track.get()).is_empty(),
            "no band at all"
        );
        assert!(reach(&commands, gauge.fill.get()).is_empty());
    }

    #[test]
    fn a_thickness_of_zero_leaves_the_ticks_and_the_needle_and_no_band() {
        // The three parts have three separate sizes, and only one of them is the
        // band: a caller who wants a hairline scale with no arc gets it.
        let (_nodes, mut gauge) = half();
        gauge.set_thickness(0.0);
        gauge.set_gauge_type(GaugeType::Needle);
        let commands = gauge.paint(RECT);
        assert!(
            reach(&commands, gauge.track.get()).is_empty(),
            "no track band"
        );
        assert!(
            reach(&commands, gauge.fill.get()).is_empty(),
            "no fill band"
        );
        assert_eq!(
            lines(&commands).len(),
            11,
            "the marks still have their own width"
        );
        assert!(needle(&commands).is_some(), "and so does the needle");
    }

    #[test]
    fn a_sweep_of_nothing_draws_nothing_at_all() {
        // The same angle twice is an arc of no extent, and a gauge whose only
        // geometry is that arc has nothing to draw — not a mark, and not a full
        // turn, which is what `GaugeType::Circle` is for.
        let (_nodes, mut gauge) = half();
        gauge.set_angles(135.0, 135.0);
        assert_eq!(gauge.sweep(), 0.0);
        assert_eq!(gauge.sweep(), 0.0, "and the same whatever the pair's order");
        assert!(gauge.paint(RECT).is_empty(), "an empty recorder");
    }

    #[test]
    fn a_sweep_written_the_long_way_round_is_the_same_arc() {
        // The sweep is taken modulo a turn, which is what lets a caller write the
        // end of an arc as the angle the pointer ends at rather than as 405.
        let (_nodes, mut gauge) = half();
        gauge.set_angles(135.0, 405.0);
        assert_close(gauge.sweep(), 270.0, "135 to 405");
        let unwrapped = reach(&gauge.paint(RECT), gauge.track.get());
        gauge.set_angles(135.0, 45.0);
        assert_eq!(
            reach(&gauge.paint(RECT), gauge.track.get()),
            unwrapped,
            "and the same band as 135 to 45"
        );
    }

    #[test]
    fn a_gauge_in_a_box_that_is_not_square_inscribes_its_dial_in_the_shorter_side() {
        // The fixture that matters more here than in any other module in the
        // repository: a circle in a square box is the degenerate case, because its
        // centre is on the diagonal and both sides are the same number, so a centre
        // read from `y + height` agrees by accident with one read from `y + width`
        // and a radius read from the width agrees with one read from the height.
        let (_nodes, gauge) = half();
        assert_eq!(
            centre(WIDE),
            (860.0, 130.0),
            "the dial is at the box's own middle, from its origin and its extents"
        );
        assert_eq!(
            radius_of(WIDE),
            83.0,
            "180 across less half the thickness, not 320"
        );

        let commands = gauge.paint(WIDE);
        let track = band(&commands, gauge.track.get());
        assert!(
            !track.is_empty(),
            "and it drew something, at the smaller of the two sides"
        );
        // The band is on 83 ± 7, so 90 and 76: the **outer** corner is the one a
        // radius read from the wrong side of the box would move, and asserting the
        // centre line's own radius here would be the old test's question.
        for quad in &track {
            assert_close(
                from_centre(WIDE, quad[1]),
                90.0,
                "on the 90-pixel outer radius, which is 180 across less the arc's \
                 own half-thickness",
            );
            assert_close(from_centre(WIDE, quad[0]), 76.0, "and 76 inside it");
        }
    }

    #[test]
    fn a_gauge_drawn_in_a_wide_box_leaves_the_box_s_own_extents_alone() {
        // The same fixture, the other direction: the dial must not stretch to fill
        // the box, and it must not be placed by the box's left edge.
        let (_nodes, gauge) = half();
        let corners: Vec<(f32, f32)> = band(&gauge.paint(WIDE), gauge.track.get())
            .iter()
            .flatten()
            .copied()
            .collect();
        let (cx, cy) = centre(WIDE);
        // The band's **outer** edge is 180 across the shorter side, so 90 from the
        // centre — a box 320 wide could hold 160 and the dial must not try.
        assert!(
            corners.iter().all(|c| (c.0 - cx).abs() <= 90.0 + 0.01),
            "nothing is left of the dial's own centre"
        );
        assert!(corners.iter().all(|c| (c.1 - cy).abs() <= 90.0 + 0.01));
        // And the band reaches the box's own top edge without going past it. **A
        // polygon's extreme point is not always one of its corners**: at 8.18° a
        // step, 135° + 16 steps is 265.9° and the next is 274.1°, so no corner is at
        // twelve o'clock and the topmost corner is one sagitta below the top of the
        // box while the topmost *edge* is on it. Asking for the corner to be at the
        // edge is asking for a tessellation whose corners happen to land there, which
        // is a claim about the angles and not about the band.
        let step = 270.0 / f32::from(gauge.arc_segment_count(radius_of(WIDE), 270.0));
        let bound = sagitta_of(90.0, step);
        let highest = corners.iter().map(|c| c.1).fold(f32::INFINITY, f32::min);
        let lowest = corners
            .iter()
            .map(|c| c.1)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            highest >= WIDE.y - 0.01,
            "no corner is above the box's own top edge at {}: {highest}",
            WIDE.y
        );
        assert!(
            highest <= WIDE.y + bound + 0.01,
            "and the topmost corner is within the {bound} px of chord of it: {highest}"
        );
        // The bottom of the box is the quarter the arc leaves open, so the band
        // stops a long way short of it: 26 pixels of clear box below the ends.
        assert!(
            lowest < WIDE.y + WIDE.height - 20.0,
            "and the bottom of the box is untouched by the 90-degree gap: {lowest}"
        );
    }

    #[test]
    fn a_gauge_away_from_the_origin_measures_its_own_extent_and_not_its_position() {
        // Two gauges of the same size at different places draw the same shapes at
        // two places, and the length of the band does not follow the position. A
        // version of this widget that read `rect.x` as a radius would report a
        // negative radius at 700 and draw nothing.
        let (_nodes, gauge) = half();
        let track = gauge.track.get();
        let here = band(&gauge.paint(RECT), track);
        let elsewhere = band(
            &gauge.paint(Rect::new(
                RECT.x + 500.0,
                RECT.y + 300.0,
                RECT.width,
                RECT.height,
            )),
            track,
        );
        assert_eq!(here.len(), elsewhere.len(), "the same number of quads");
        let (here_first, there_first) = (
            here.first().expect("a band").as_slice(),
            elsewhere.first().expect("a band").as_slice(),
        );
        for (index, (a, b)) in here_first.iter().zip(there_first).enumerate() {
            // `assert_close` and not `assert_eq`: the two corners are each a sum of a
            // centre and a radius times a cosine, and the cosine of the same angle at
            // (632, 364) and (1132, 664) is not bit for bit the same number.
            let across = format!("corner {index} of the first segment across the box");
            assert_close(b.0 - a.0, 500.0, &across);
            assert_close(b.1 - a.1, 300.0, "and down it");
        }
    }

    #[test]
    fn a_gauge_with_no_extent_draws_nothing() {
        // A zero-size rect has no radius, and a radius of zero is answered before
        // the step is asked, because the step would divide by it.
        let (_nodes, gauge) = half();
        assert!(gauge.paint(Rect::new(50.0, 50.0, 0.0, 0.0)).is_empty());
        assert!(gauge.paint(Rect::new(50.0, 50.0, 0.0, 200.0)).is_empty());
        assert!(gauge.paint(Rect::new(50.0, 50.0, 200.0, 0.0)).is_empty());
    }

    #[test]
    fn a_box_narrower_than_the_arc_is_thick_draws_nothing_rather_than_inside_out() {
        let (_nodes, gauge) = half();
        let narrow = Rect::new(700.0, 40.0, 10.0, 200.0);
        assert_eq!(radius_of(narrow), 0.0, "10 wide less 14 is negative");
        assert!(gauge.paint(narrow).is_empty());
    }

    #[test]
    fn a_box_narrower_than_twice_the_thickness_draws_a_sector_rather_than_a_bow_tie() {
        // The one degenerate case the floor on the inner radius is there for, and it
        // is **not** the empty one above: a box 24 wide leaves a 5-pixel centre line
        // and a 14-pixel band, so `radius - thickness/2` is −2 and the inner corners
        // would be placed on the *far* side of the dial's centre.
        //
        // The resulting quad is `(−2 at from, 12 at from, 12 at to, −2 at to)`: four
        // corners on two circles, and edges that cross. It is concave, and the
        // renderer's fan from the first point would draw two triangles that overlap
        // the shape and two that fall outside it. Clamping the inner radius at zero
        // puts both inner corners on the dial's own centre, and the quad is a sector
        // — still degenerate at that size, but drawn as what it is.
        let (_nodes, gauge) = half();
        let narrow = Rect::new(700.0, 40.0, 24.0, 200.0);
        let radius = radius_of(narrow);
        assert_eq!(radius, 5.0, "24 across less half the thickness is 5");
        assert!(
            radius - ARC_THICKNESS / 2.0 < 0.0,
            "so the inner radius is negative before it is floored"
        );

        let quads = band(&gauge.paint(narrow), gauge.track.get());
        assert_eq!(quads.len(), RECT_TRACK_SEGMENTS, "and it still draws");
        let centre = centre(narrow);
        for (index, quad) in quads.iter().enumerate() {
            assert_convex(quad, &format!("narrow-box segment {index}"));
            for corner in [quad[0], quad[3]] {
                assert_eq!(
                    corner, centre,
                    "segment {index}: the inner corners are the dial's own centre"
                );
            }
            assert_close(
                from_centre(narrow, quad[1]),
                12.0,
                "and the outer ones are on the outer radius",
            );
        }
    }

    #[test]
    fn a_value_change_animates_the_drawn_value_toward_the_truth() {
        // The contract is that the needle *moves* rather than jumping, so this
        // checks the middle of the transition and not only its end: a `set` instead
        // of an animation would pass an end-only assertion.
        let (_nodes, gauge) = gauge();
        gauge.value.set(240.0);
        gauge.animate_to_state(motion());

        assert_eq!(gauge.shown.get(), 0.0, "it starts where it was");
        assert!(tick(&gauge, 50));
        let half = gauge.shown.get();
        assert!(half > 0.0 && half < 240.0, "half way through, it has moved");
        assert_eq!(half, 120.0, "to exactly half, on a linear curve");

        assert!(tick(&gauge, 50));
        assert_eq!(gauge.shown.get(), 240.0, "and it arrives");
        assert!(!gauge.is_animating());
    }

    #[test]
    fn a_second_aim_replaces_the_first_transition_rather_than_racing_it() {
        // The reason `animate_to_state` clears the gauge's own clock, asserted as
        // its effect rather than as its presence. Without the `clear` the first
        // aim's animation stays in the clock and keeps writing every frame, so
        // once the *shorter* second aim has arrived the abandoned one drags the
        // drawn value back toward a reading the caller has already replaced.
        //
        // The first aim is the **longer** of the two, and that is the whole
        // arrangement: with two aims of the same length the abandoned animation
        // arrives on the same frame as its replacement and its write is the one
        // the replacement overwrites, so a missing `clear` would pass every other
        // test in this module — the progress bar's own test of this rule has that
        // shape, and only the clock's own `clear` distinguishes the two cases.
        let (_nodes, gauge) = gauge();
        gauge.value.set(240.0);
        gauge.animate_to_state(Motion {
            duration: ms(400),
            easing: Easing::Linear,
        });
        tick(&gauge, 50);
        // An eighth of 400 ms, so an eighth of the way from 0 to 240.
        assert_eq!(gauge.shown.get(), 30.0, "an eighth along the first aim");

        gauge.value.set(48.0);
        gauge.animate_to_state(Motion {
            duration: ms(100),
            easing: Easing::Linear,
        });
        tick(&gauge, 50);
        assert_eq!(
            gauge.shown.get(),
            39.0,
            "half way along the second: 30 + (48 - 30) / 2"
        );

        tick(&gauge, 50);
        assert_eq!(gauge.shown.get(), 48.0, "and the second aim arrives");
        assert!(
            !gauge.is_animating(),
            "with the abandoned transition gone rather than still dragging the \
             needle back toward 240"
        );
        assert!(!tick(&gauge, 50), "so nothing writes on the frame after");
        assert_eq!(gauge.shown.get(), 48.0, "and it stays where it arrived");
    }

    #[test]
    fn a_second_theme_switch_does_not_drag_the_colours_back() {
        // The other half of the same claim, on the colours rather than the value,
        // because the two are animated on one clock and the racing would show on
        // whichever happened to be checked.
        let (_nodes, mut gauge) = gauge();
        gauge.set_palette(Palette::from_theme(&Theme::light()));
        gauge.animate_to_state(Motion {
            duration: ms(400),
            easing: Easing::Linear,
        });
        tick(&gauge, 50);
        let light_track = Palette::from_theme(&Theme::light()).track;
        assert_ne!(
            gauge.track.get(),
            light_track,
            "on its way out of the dark theme"
        );

        gauge.set_palette(Palette::from_theme(&Theme::dark()));
        gauge.animate_to_state(Motion {
            duration: ms(100),
            easing: Easing::Linear,
        });
        tick(&gauge, 50);
        tick(&gauge, 50);
        assert_eq!(
            gauge.track.get(),
            Palette::from_theme(&Theme::dark()).track,
            "the second switch arrives"
        );
        assert!(!gauge.is_animating(), "and the abandoned one is gone");
        assert_eq!(light_track, Palette::from_theme(&Theme::light()).track);
    }

    #[test]
    fn the_fill_follows_the_animating_value_rather_than_the_value() {
        // Requirement 4's first half, asserted on the picture: the fill's reach is
        // the measure of what is drawn, and it must be the drawn value's share and
        // not the truth's.
        let (_nodes, gauge) = gauge();
        gauge.value.set(240.0);
        gauge.animate_to_state(motion());

        let reach = |gauge: &Gauge| span(RECT, &reach(&gauge.paint(RECT), gauge.fill.get()));
        let at_start = reach(&gauge);
        tick(&gauge, 50);
        let halfway = reach(&gauge);
        tick(&gauge, 50);
        let arrived = reach(&gauge);

        assert_eq!(at_start, 0.0, "nothing filled before the first tick");
        assert_close(halfway, 135.0, "half the arc at half the time");
        assert_close(arrived, 270.0, "and the whole of it when it arrives");
        assert_close(
            arrived - halfway,
            halfway - at_start,
            "growing evenly, because the curve is linear",
        );
    }

    #[test]
    fn a_value_that_goes_backwards_animates_backwards() {
        // The other half of "animates when the value changes": not only forwards.
        let (_nodes, gauge) = showing(0.0, 240.0, 192.0);
        gauge.value.set(48.0);
        gauge.animate_to_state(motion());
        tick(&gauge, 50);
        assert_eq!(gauge.shown.get(), 120.0, "from 192 towards 48");
        tick(&gauge, 50);
        // `192.0 + (48.0 - 192.0) * 1.0` is `47.999998` in `f32`, which is the
        // interpolation arriving at its target and not a value short of it: the
        // midpoint above is exact and this end is a product, so it is asserted to
        // the third decimal rather than to the compiler's rounding.
        assert_close(gauge.shown.get(), 48.0, "and arrives there");
    }

    #[test]
    fn a_second_change_replaces_the_first_rather_than_racing_it() {
        // A value written twice in quick succession used to be a race: both
        // transitions wrote the drawn value and the older one arriving last left
        // it short of the truth. The gauge's own clock is what rules that out.
        let (_nodes, gauge) = gauge();
        gauge.value.set(240.0);
        gauge.animate_to_state(motion());
        tick(&gauge, 50);
        gauge.value.set(60.0);
        gauge.animate_to_state(motion());
        for _ in 0..4 {
            tick(&gauge, 50);
        }
        assert_eq!(gauge.shown.get(), 60.0, "it ends at the newest value");
        assert!(!gauge.is_animating(), "and nothing is left running");
    }

    #[test]
    fn an_arc_gauge_leaves_the_needle_colour_alone() {
        // Nothing draws the needle outside `GaugeType::Needle`, and a transition on
        // a property nothing draws is a transition that reports itself as running
        // for nothing.
        let (_nodes, mut gauge) = gauge();
        let before = gauge.needle.get();
        gauge.set_palette(Palette::from_theme(&Theme::light()));
        gauge.animate_to_state(motion());
        assert!(tick(&gauge, 50), "the drawn value is moving");
        assert_eq!(gauge.needle.get(), before, "and the needle's colour is not");
    }

    #[test]
    fn a_needle_gauge_animates_its_needle_colour() {
        // The other half of the test above, so the rule cannot be satisfied by
        // leaving the colour alone everywhere.
        let (_nodes, mut gauge) = gauge();
        gauge.set_gauge_type(GaugeType::Needle);
        let before = gauge.needle.get();
        gauge.set_palette(Palette::from_theme(&Theme::light()));
        gauge.animate_to_state(motion());
        let light = Palette::from_theme(&Theme::light()).needle;
        tick(&gauge, 50);
        let halfway = gauge.needle.get();
        assert_ne!(halfway, before, "the colour has left the old theme");
        assert_ne!(halfway, light, "and has not arrived at the new one");
        tick(&gauge, 50);
        assert_eq!(gauge.needle.get(), light, "and it arrives");
    }

    #[test]
    fn a_gauge_with_no_ticks_leaves_the_tick_colour_alone() {
        let (_nodes, mut gauge) = gauge();
        gauge.set_ticks(0);
        let before = gauge.tick.get();
        gauge.set_palette(Palette::from_theme(&Theme::light()));
        gauge.animate_to_state(motion());
        tick(&gauge, 50);
        assert_eq!(gauge.tick.get(), before, "nothing draws the ticks");
        // The other colours *are* on their way: half way through the motion, each
        // is at the midpoint of its own old and new value and at neither end.
        let light = Palette::from_theme(&Theme::light());
        for (moving, target) in [
            (gauge.fill.get(), light.fill),
            (gauge.track.get(), light.track),
        ] {
            assert_ne!(moving, target, "on its way, not arrived");
            assert_ne!(
                moving,
                Palette::from_theme(&Theme::dark()).track,
                "and not at rest"
            );
        }
        tick(&gauge, 50);
        assert_eq!(gauge.fill.get(), light.fill, "and they arrive");
    }

    #[test]
    fn the_palette_switch_is_animated_over_the_motions_own_duration() {
        // The colours are properties for the same reason the drawn value is: a
        // theme switch is a transition, not a jump.
        let (_nodes, mut gauge) = gauge();
        let light = Palette::from_theme(&Theme::light());
        let before = gauge.fill.get();
        gauge.set_palette(light);
        gauge.animate_to_state(motion());

        tick(&gauge, 50);
        let halfway = gauge.fill.get();
        assert_ne!(halfway, before, "the colour has left the old theme");
        assert_ne!(halfway, light.fill, "and has not arrived at the new one");
        tick(&gauge, 50);
        assert_eq!(
            gauge.fill.get(),
            light.fill,
            "arriving exactly at half of each"
        );
        assert_eq!(gauge.track.get(), light.track);
    }

    #[test]
    fn the_motion_the_caller_hands_over_is_the_curve_the_gauge_follows() {
        // Requirement 4's "needle animates with spring physics" is satisfied by
        // honouring the caller's curve, not by the widget picking one — so this
        // asks for a spring, gets a spring, and a linear motion gets a linear
        // draw. A version of this widget that hardcoded a curve would fail both
        // halves.
        let (_nodes, mut spring) = half();
        spring.set_gauge_type(GaugeType::Needle);
        spring.value.set(240.0);
        spring.animate_to_state(Motion {
            duration: ms(100),
            easing: Easing::Spring {
                damping: 6.0,
                stiffness: 180.0,
            },
        });

        let tip = |gauge: &Gauge| needle(&gauge.paint(RECT)).expect("a needle").0[0];
        tick(&spring, 50);
        let sprung = spring.shown.get();
        assert_ne!(
            sprung, 120.0,
            "a spring is not half way at half the time, or it is not a spring"
        );
        let _ = tip(&spring);

        let (_nodes, linear) = gauge();
        linear.value.set(240.0);
        linear.animate_to_state(motion());
        tick(&linear, 50);
        assert_eq!(linear.shown.get(), 120.0, "a linear motion still is");
    }

    #[test]
    fn is_animating_is_true_during_a_transition_and_false_after_it() {
        let (_nodes, gauge) = gauge();
        assert!(!gauge.is_animating(), "a fresh gauge is at rest");
        gauge.value.set(240.0);
        gauge.animate_to_state(motion());
        assert!(gauge.is_animating());
        assert!(tick(&gauge, 50));
        assert!(gauge.is_animating(), "mid transition");
        assert!(tick(&gauge, 50));
        assert!(!gauge.is_animating(), "and false once it has arrived");
        assert!(!tick(&gauge, 50), "a tick after that writes nothing");
    }

    #[test]
    fn a_gauge_that_has_never_been_aimed_does_not_move() {
        // Nothing moves until a caller aims it, which is the same rule the rest of
        // the library follows. Ten seconds of ticks changes nothing.
        let (_nodes, gauge) = gauge();
        gauge.value.set(240.0);
        for _ in 0..10 {
            assert!(!tick(&gauge, 1000));
        }
        assert_eq!(gauge.shown.get(), 0.0);
        assert!(!gauge.is_animating());
    }

    #[test]
    fn setting_the_angles_does_not_disturb_a_transition_already_running() {
        // The angles are the mapping from a value to a place, not the value and not
        // its colour, so nothing animates toward them.
        let (_nodes, mut gauge) = half();
        gauge.value.set(240.0);
        gauge.animate_to_state(motion());
        tick(&gauge, 50);
        let halfway = gauge.shown.get();
        gauge.set_angles(90.0, 45.0);
        assert_eq!(gauge.shown.get(), halfway, "the transition is untouched");
        assert!(gauge.is_animating());
    }

    #[test]
    fn setting_the_thickness_does_not_change_the_value() {
        // A new shape for the same value, not a change of value.
        let (_nodes, mut gauge) = half();
        gauge.animate_to_state(motion());
        tick(&gauge, 50);
        let halfway = gauge.shown.get();
        gauge.set_thickness(28.0);
        assert_eq!(gauge.shown.get(), halfway);
        assert_eq!(gauge.style().shown, 120.0, "and the value with it");
    }

    #[test]
    fn a_thicker_arc_is_the_same_number_of_quads_of_twice_the_thickness() {
        // The retune, stated as the thing it changed: a chain's step was whatever
        // two tangent circles of its thickness needed, so twice the thickness was
        // fewer circles for the same arc, and this test used to say so. **A band's
        // step is not the thickness's business** — a chord is a chord — so twice the
        // thickness is the same 33 quads and a band twice as wide.
        //
        // And the width comes off the **inner** edge, because the outer edge is
        // pinned to the rect's own side in both cases: 100 either way, with the
        // inner edge at 86 and then at 72. So a thicker band is not a smoother one
        // and a thinner one is not a rougher one, and the inner edge's own chord
        // error *falls* as the band grows because that edge moves to a smaller
        // radius. Which is the whole of what `ARC_THICKNESS` buys and does not buy.
        let (_nodes, mut gauge) = half();
        let thin = band(&gauge.paint(RECT), gauge.track.get());
        let thin_outer_sagitta = sagitta(RECT, &thin[0]);
        let thin_inner_sagitta = inner_sagitta(RECT, &thin[0]);
        gauge.set_thickness(28.0);
        let radius = (RECT.width.min(RECT.height) / 2.0 - 28.0 / 2.0).max(0.0);
        let thick = band(&gauge.paint(RECT), gauge.track.get());

        assert_eq!(
            thick.len(),
            thin.len(),
            "the same number of quads, because the step is a number of degrees"
        );
        for quad in &thick {
            assert_close(
                from_centre(RECT, quad[1]),
                100.0,
                "the outer corner is still on the rect's own side",
            );
            assert_close(
                from_centre(RECT, quad[0]),
                radius - 14.0,
                "and the inner one is 72 rather than 86",
            );
            assert_close(from_centre(RECT, quad[0]), 72.0, "which is the number");
        }
        assert_near(
            sagitta(RECT, &thick[0]),
            thin_outer_sagitta,
            0.01,
            "the outer edge's own error is unchanged",
        );
        assert!(
            inner_sagitta(RECT, &thick[0]) < thin_inner_sagitta * 0.95,
            "and the inner edge's is smaller, because that edge is on a smaller \
             radius: {} against {thin_inner_sagitta}",
            inner_sagitta(RECT, &thick[0])
        );
    }

    #[test]
    fn the_only_way_a_gauge_changes_is_a_write_to_its_value() {
        // Requirement 4 lists no gesture for a gauge, so this widget has no
        // `on_event` at all: there is nothing for a tap on it to report. The
        // needle and the tick marks are decoration — a finger over either goes to
        // whatever is behind the gauge — and what a caller drives instead is the
        // value property, whose own callback is the link to the node.
        let (_nodes, gauge) = gauge();
        assert_eq!(
            gauge.paint(RECT).len(),
            44,
            "a track, its marks, and no fill"
        );
        gauge.value.set(120.0);
        gauge.snap_to_state();
        assert_eq!(
            gauge.paint(RECT).len(),
            44 + 17,
            "and a fill once there is one"
        );
    }

    #[test]
    fn the_caller_can_find_out_what_share_of_the_dial_a_value_is() {
        // The public mapping the demo needs to place a label, and the reason it is
        // public: a caller drawing a value readout beside a gauge needs this and
        // not its own arithmetic over the range.
        let (_nodes, gauge) = gauge();
        assert_eq!(gauge.fraction(0.0), 0.0);
        assert_eq!(gauge.fraction(60.0), 0.25);
        assert_eq!(gauge.fraction(120.0), 0.5);
        assert_eq!(gauge.fraction(180.0), 0.75);
        assert_eq!(gauge.fraction(240.0), 1.0);
        for step in 0..=10u8 {
            let value = f32::from(step) * 24.0;
            assert_close(
                gauge.fraction(value),
                f32::from(step) / 10.0,
                "{value} is that share of a 240 range",
            );
        }
    }

    #[test]
    fn the_fraction_of_a_value_on_a_range_with_a_negative_floor_still_works() {
        // A temperature gauge is not a 0-to-100 gauge, and the mapping is a
        // fraction of the range rather than of the absolute value.
        let (_nodes, gauge) = showing(-40.0, 60.0, -40.0);
        assert_eq!(gauge.fraction(-40.0), 0.0);
        assert_eq!(gauge.fraction(60.0), 1.0);
        assert_eq!(gauge.fraction(10.0), 0.5);
        assert_eq!(gauge.fraction(-100.0), 0.0, "clamped below the floor");
        assert_eq!(gauge.fraction(200.0), 1.0, "and above the ceiling");
    }

    #[test]
    fn the_four_palette_colours_are_what_each_part_of_the_gauge_paints() {
        // The pair the colours are asserted as: every part of the picture is in
        // the palette, and each in the palette's own colour, so a caller that
        // re-themes a dial re-themes all of it.
        let (_nodes, mut gauge) = gauge();
        gauge.set_gauge_type(GaugeType::Needle);
        gauge.value.set(120.0);
        gauge.snap_to_state();
        let commands = gauge.paint(RECT);
        let palette = gauge.palette();
        assert_eq!(polygon_colors(&commands)[0], palette.track);
        assert_eq!(
            polygon_colors(&commands)[RECT_TRACK_SEGMENTS],
            palette.fill,
            "the fill's band begins right after the track's"
        );
        // The needle and its hub are the one colour on two primitives, so the
        // triangle is asked for its colour and the hub for being there: both are
        // drawn from `needle.get()`, and the hub's own radius and place are the
        // needle test's business.
        assert_eq!(needle(&commands).unwrap().1, palette.needle);
        assert_eq!(
            circles(&commands).len(),
            1,
            "and the hub is the only circle a gauge draws at all"
        );
        assert!(line_colors(&commands).contains(&palette.tick), "the marks");
    }
}
