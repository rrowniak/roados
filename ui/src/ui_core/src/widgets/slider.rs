//! The Slider widget: a continuous value, a track, and every way of moving it.
//!
//! A slider is a node, the properties the task gives it, and four properties it
//! animates. [`Slider::value`] is the truth; [`Slider::thumb`] is what is *drawn*,
//! and the distance between the two is the transition. That split is what makes
//! requirement 4's "animates smoothly when the value changes programmatically"
//! and requirement 3's "touch-drag updates the value" the same widget: an
//! interaction writes both at once, so the thumb is under the finger, and a
//! programmatic write moves `value` alone, leaving [`Slider::animate_to_state`]
//! to carry the thumb there.
//!
//! The colours follow the button's rule rather than adding theme tokens: a
//! [`Palette`] names the four colours and one ring colour, the properties hold
//! them so a theme switch can be animated into them, and
//! [`Slider::snap_to_state`] puts a themed slider on its theme at once. The
//! *sizing* — how thick the track is, how big the thumb is — is named constants
//! rather than tokens, for the reason the button's own `MIN_TOUCH_TARGET`
//! documents: the theme has no token for a slider's parts, and adding one would
//! change [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables,
//! the token count and the animation every token takes part in during a switch,
//! for values a switch does not change. Each constant below says what would
//! reverse that.
//!
//! Two things are the caller's, not the slider's: which node is dirty, and which
//! node is focused. The first is a property callback per the demo's own idiom —
//! `on_change` on the animated properties, marking the node — and the second is
//! [`input::Focus`](crate::input::Focus), which knows the focus order and not
//! what a left arrow means.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::theme::Theme;
//! use ui_core::widgets::button::Motion;
//! use ui_core::widgets::slider::Slider;
//!
//! let mut nodes = Arena::new();
//! let mut slider = Slider::new(&mut nodes, 0.0, 100.0);
//! slider.set_palette(ui_core::widgets::slider::Palette::from_theme(&Theme::dark()));
//!
//! // A programmatic change moves the value at once and carries the thumb after
//! // it, so the two agree only when the transition has arrived.
//! slider.value.set(80.0);
//! slider.animate_to_state(Motion::from_theme(&Theme::dark()));
//! for _ in 0..15 {
//!     slider.tick(Duration::from_millis(10));
//! }
//! assert_eq!(slider.thumb.get(), 80.0, "the thumb has arrived");
//! ```

use std::cell::RefCell;
use std::time::Duration;

use crate::animation::AnimationClock;
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::{Offset, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::button::Motion;
use crate::widgets::Callback;

/// The length a slider asks for when its caller gives it no size of its own.
///
/// A slider has no content to measure, so this is the one number that decides
/// how long it is by default. It is a constant rather than a theme token for the
/// reason the other constants here are.
const DEFAULT_LENGTH: f32 = 240.0;

/// How thick the track and the fill are, in pixels.
///
/// The track is a rounded rectangle, so its radius is half of this — which is
/// what makes it a line with rounded ends rather than a bar.
const TRACK_THICKNESS: f32 = 6.0;

/// The radius of the thumb, in pixels, at rest.
///
/// Twelve is twice the track's thickness, so the thumb is a knob that stands
/// proud of its track in both orientations.
const THUMB_RADIUS: f32 = 12.0;

/// The width of the ring drawn around the thumb, in pixels.
///
/// The thumb is "a circle, primary colour with a border", and a filled circle
/// with a filled circle on top of it is how that reads: the border is the larger
/// of the two. Zero draws the thumb with no ring.
const THUMB_BORDER: f32 = 2.0;

/// The scale the thumb is drawn at while a pointer is dragging it.
///
/// Requirement 4 asks for a thumb that "scales up slightly"; 1.15 is slightly,
/// and the word is worth a number here for the reason
/// [`PRESS_SHADOW_ALPHA`](crate::widgets::button::PRESS_SHADOW_ALPHA) gives — an
/// effect with an adjective in its spec needs the number the adjective fixes, so
/// a test can assert it.
const THUMB_DRAGGED_SCALE: f32 = 1.15;

/// The focus ring's width, in pixels, unless a caller changes it.
///
/// The ring is drawn **around the track** and then covered by it, so only its
/// border shows: a [`DrawCommand::RoundedRect`] fills its rect, and a ring around
/// the slider's whole rect would be a card rather than an outline — a slider has
/// no background of its own to draw over the middle of one the way a button draws
/// its background over its own ring. [`Slider::paint`] says so where it draws it.
const FOCUS_RING: f32 = 2.0;

/// The smallest a slider may be across its short axis, in pixels.
///
/// 44dp is the platform touch target a finger can hit, and it is the same floor
/// [`button::MIN_TOUCH_TARGET`](crate::widgets::button::MIN_TOUCH_TARGET) puts
/// under a button: the thumb is 28 pixels across and a slider is held by its
/// thumb, so the *node* has to be big enough to be found. It is a constant rather
/// than a theme token for the reason that one documents; it is repeated here
/// rather than imported because a `pub const` in another widget's module is not a
/// shared place to keep one, and the two will diverge when the theme grows a
/// density setting.
const MIN_TOUCH_TARGET: f32 = 44.0;

/// How much of the range one arrow press moves the value by, when the slider
/// has no step.
///
/// A slider with a step moves by its step; one without has no grid to move on,
/// and a fraction of the range is the only figure that means the same thing on a
/// 0–100 volume and a 0–1 one.
const KEY_STEP_FRACTION: f32 = 0.05;

/// The axis a slider's track runs along.
///
/// It is not decoration: it decides where the thumb starts (a vertical slider's
/// minimum is at the *bottom*, because the screen's y axis points down and up
/// should mean more), which arrow keys adjust the value, which component of a
/// [`Scroll`](InputEventKind::Scroll) does, and the shape of the track, the fill
/// and the thumb drawn inside `rect`.
///
/// # Examples
///
/// ```
/// use ui_core::paint::Rect;
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::widgets::slider::{Orientation, Slider};
///
/// let mut nodes = Arena::new();
/// let mut slider = Slider::new(&mut nodes, 0.0, 10.0);
/// let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
///
/// // Horizontal: the thumb's x follows the value.
/// assert_eq!(slider.thumb_center(rect).0, 12.0, "at the minimum");
///
/// slider.set_orientation(Orientation::Vertical);
/// // Vertical: the value follows the thumb's y, and a bigger value is higher.
/// assert_eq!(slider.thumb_center(rect).1, 28.0, "at the minimum, at the bottom");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// The track runs from left to right.
    #[default]
    Horizontal,
    /// The track runs from bottom to top.
    Vertical,
}

/// The colours a slider draws with.
///
/// Four colours and a ring: the track, the fill, the thumb, the ring around the
/// thumb, and the focus indicator. They are not tokens of their own — the theme
/// has none per part, and adding one per part would put five more tokens in
/// every theme table and in every theme switch — so a slider is themed with the
/// theme's own four and [`Palette::from_theme`] says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The track: the whole range, unfilled.
    pub track: Color,
    /// The fill: the part of the range the value has passed.
    pub fill: Color,
    /// The thumb's own circle.
    pub thumb_fill: Color,
    /// The ring around the thumb.
    pub thumb_border: Color,
    /// The focus indicator, drawn around the track and covered by it.
    pub ring: Color,
}

impl Default for Palette {
    /// Returns a neutral grey slider: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    fn default() -> Self {
        Palette {
            track: Color::new(64, 64, 64, 255),
            fill: Color::new(160, 160, 160, 255),
            thumb_fill: Color::new(220, 220, 220, 255),
            thumb_border: Color::new(32, 32, 32, 255),
            ring: Color::new(255, 255, 255, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The track is [`Border`](crate::theme::ThemeToken::Border), which is the
    /// theme's hairline colour and the only one of its nine that is *muted* by
    /// definition — a slider's track is the part of it that is not the value.
    /// The fill and the thumb are [`Primary`](crate::theme::ThemeToken::Primary),
    /// because a filled track is the value and the thumb is the value's handle.
    /// The ring around the thumb is
    /// [`OnPrimary`](crate::theme::ThemeToken::OnPrimary), which is by
    /// definition legible on `Primary` in both themes.
    ///
    /// The focus ring is the one exception: it is drawn on whatever the slider is
    /// laid over, which is the window's background and not the track — and
    /// `OnPrimary` is black in the dark theme, which *is* the dark theme's
    /// background. So the ring is [`Text`](crate::theme::ThemeToken::Text), the
    /// colour this repository uses for anything that has to be legible on the
    /// background itself.
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            track: token_color(theme, crate::theme::ThemeToken::Border),
            fill: token_color(theme, crate::theme::ThemeToken::Primary),
            thumb_fill: token_color(theme, crate::theme::ThemeToken::Primary),
            thumb_border: token_color(theme, crate::theme::ThemeToken::OnPrimary),
            ring: token_color(theme, crate::theme::ThemeToken::Text),
        }
    }
}

/// The appearance the slider's value and its dragging flag imply.
///
/// Every field is a target, not a value in flight:
/// [`Slider::animate_to_state`] animates the slider's properties toward this and
/// [`Slider::paint`] draws whatever the properties have reached, which is a
/// [`Style`] part way through on a frame where something is moving.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The value the thumb is heading for: `value`, clamped and stepped.
    pub thumb: f32,
    /// The scale the thumb is drawn at.
    pub thumb_scale: f32,
    /// The colour of the track.
    pub track: Color,
    /// The colour of the fill.
    pub fill: Color,
    /// The colour of the thumb.
    pub thumb_fill: Color,
    /// The colour of the ring around the thumb.
    pub thumb_border: Color,
    /// The focus ring's thickness, or zero for no ring.
    pub ring_width: f32,
}

/// A slider: a control that picks a continuous value out of a range.
///
/// The widget holds the properties the task gives it — [`value`], [`thumb`],
/// [`thumb_scale`], [`track`], [`fill`], [`thumb_fill`], [`thumb_border`],
/// [`dragging`], [`focused`] and [`on_change`] — and the four plain numbers that
/// define the range and the shape, each behind a setter: [`min`] and [`max`]
/// through [`Slider::set_range`], [`step`] through [`Slider::set_step`],
/// [`orientation`] through [`Slider::set_orientation`], and the track's thickness
/// and the thumb's radius and border through their own setters.
///
/// The range and the shape are plain fields rather than properties because they
/// are the mapping rather than the appearance: nothing animates a slider's
/// minimum, and a property the caller writes would need the setters anyway to
/// re-clamp the value against it.
///
/// The node is the caller's to keep clean, and its size is the caller's to give
/// through [`layout_mut`](crate::node::WidgetNode::layout_mut) — [`Slider::size`]
/// is a suggestion for a caller who has nothing else to go on. A slider draws
/// inside whatever rect it is given: a track along its long axis, a fill from the
/// track's start to the thumb, and the thumb between them.
///
/// [`value`]: Slider::value
/// [`thumb`]: Slider::thumb
/// [`thumb_scale`]: Slider::thumb_scale
/// [`track`]: Slider::track
/// [`fill`]: Slider::fill
/// [`thumb_fill`]: Slider::thumb_fill
/// [`thumb_border`]: Slider::thumb_border
/// [`dragging`]: Slider::dragging
/// [`focused`]: Slider::focused
/// [`on_change`]: Slider::on_change
/// [`min`]: Slider::min
/// [`max`]: Slider::max
/// [`step`]: Slider::step
/// [`orientation`]: Slider::orientation
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::{InputEvent, InputEventKind};
/// use ui_core::layout::Offset;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::slider::Slider;
///
/// let mut nodes = Arena::new();
/// let mut slider = Slider::new(&mut nodes, 0.0, 10.0);
/// assert_eq!(slider.value.get(), 0.0, "a slider starts at its minimum");
///
/// // A tap on the track jumps the value to where the tap was, and is consumed.
/// let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
/// let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(50.0, 20.0)));
/// assert!(slider.on_event(&mut tap, rect));
/// assert_eq!(slider.value.get(), 5.0, "half way along 0 to 10");
/// ```
pub struct Slider {
    /// The value the slider is at. The caller may write it — that is the
    /// "changes programmatically" case — and the widget writes it from every
    /// interaction.
    pub value: Property<f32>,
    /// The value the thumb is *drawn* at, animated toward [`value`](Slider::value).
    ///
    /// An interaction writes it at once, so the thumb is under the finger; a
    /// programmatic write leaves it behind, and
    /// [`animate_to_state`](Slider::animate_to_state) carries it over. Writing
    /// it by hand makes the thumb and the value disagree.
    pub thumb: Property<f32>,
    /// The scale the thumb is drawn at, animated while a pointer drags it.
    pub thumb_scale: Property<f32>,
    /// The colour of the track: the whole range, unfilled.
    pub track: Property<Color>,
    /// The colour of the fill: the part of the range the value has passed.
    pub fill: Property<Color>,
    /// The colour of the thumb.
    pub thumb_fill: Property<Color>,
    /// The colour of the ring drawn around the thumb.
    pub thumb_border: Property<Color>,
    /// The focus ring's thickness, in pixels. Zero draws no ring even when the
    /// slider is focused.
    pub focus_ring: Property<f32>,
    /// Whether a pointer is down over the slider, which is what makes the thumb
    /// grow. Written by the caller from a press and a release, the way a button's
    /// `pressed` is: the gesture recogniser reports a tap on the *release*, so
    /// the pressed appearance has to be on screen for the whole time the pointer
    /// is down, which is before any tap exists.
    pub dragging: Property<bool>,
    /// Whether the slider holds focus. Written by the caller, from
    /// [`input::Focus`](crate::input::Focus), and it is what a key press acts on.
    pub focused: Property<bool>,
    /// The callback every interaction fires, with the value it moved to.
    ///
    /// It fires when the value *changed*: dragging a slider that is already at
    /// its maximum fires nothing, so a caller driving something from this is not
    /// told about every frame of a finger resting on the end of the track. A
    /// caller that writes [`value`](Slider::value) itself is itself, and reads
    /// the property; the callback is the widget reporting its own doing.
    pub on_change: Callback<f32>,
    min: f32,
    max: f32,
    step: Option<f32>,
    orientation: Orientation,
    track_thickness: f32,
    thumb_radius: f32,
    thumb_border_width: f32,
    palette: Palette,
    clock: RefCell<AnimationClock>,
    node: Handle,
}

impl Slider {
    /// Creates a slider over `min..=max` in the arena, and returns it.
    ///
    /// The range is ordered rather than taken as given, so a caller that passes
    /// them the wrong way round gets a slider rather than a division by a
    /// negative span. The value starts at the minimum and the thumb with it, and
    /// the colours are the neutral defaults until a caller gives it a
    /// [`Palette`](Slider::set_palette) and calls
    /// [`snap_to_state`](Slider::snap_to_state).
    ///
    /// The task file's `Slider::new(min, max) -> Handle` is read as this: the
    /// handle is [`Slider::handle`]'s, and returning it alone would leave a
    /// caller with no properties to set and no callback to register. Task 12's
    /// [`Button::new`](crate::widgets::button::Button::new) settled the same
    /// reading.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, min: f32, max: f32) -> Self {
        let palette = Palette::default();
        let (min, max) = ordered(min, max);
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Slider {
            value: Property::new(min),
            thumb: Property::new(min),
            thumb_scale: Property::new(1.0),
            track: Property::new(palette.track),
            fill: Property::new(palette.fill),
            thumb_fill: Property::new(palette.thumb_fill),
            thumb_border: Property::new(palette.thumb_border),
            focus_ring: Property::new(FOCUS_RING),
            dragging: Property::new(false),
            focused: Property::new(false),
            on_change: Callback::none(),
            min,
            max,
            step: None,
            orientation: Orientation::default(),
            track_thickness: TRACK_THICKNESS,
            thumb_radius: THUMB_RADIUS,
            thumb_border_width: THUMB_BORDER,
            palette,
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the slider's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the slider draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the slider draws with, and leaves the current ones where
    /// they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](Slider::animate_to_state) or
    /// [`snap_to_state`](Slider::snap_to_state): a theme switch is animated, and
    /// a theme switch is the caller announcing a new palette and then moving the
    /// slider toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the slider chasing a palette that is still
    /// moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the slider's minimum.
    #[must_use]
    pub fn min(&self) -> f32 {
        self.min
    }

    /// Returns the slider's maximum.
    #[must_use]
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Sets the range the slider covers, and pulls the value and the thumb back
    /// inside it.
    ///
    /// The two are ordered, so `set_range(10.0, 0.0)` is a slider from 0 to 10
    /// and not one with a negative span. The value is re-clamped and re-snapped
    /// and the thumb follows it at once rather than travelling: a range that has
    /// just changed has no transition to run, and a thumb animating into a track
    /// that has not been drawn yet is a frame of nonsense.
    pub fn set_range(&mut self, min: f32, max: f32) {
        let (min, max) = ordered(min, max);
        self.min = min;
        self.max = max;
        let value = self.normalize(self.value.get());
        self.clock.borrow_mut().clear();
        self.value.set(value);
        self.thumb.set(value);
    }

    /// Returns the slider's step, or `None` if it has none.
    #[must_use]
    pub fn step(&self) -> Option<f32> {
        self.step
    }

    /// Sets the grid the value snaps to, or removes it with `None`.
    ///
    /// A step of zero or less is no step at all: the value would be snapped to
    /// itself, or to a point on the wrong side of where it started, and a
    /// division by it would be meaningless. So it is stored as [`None`], which is
    /// what a caller who meant nothing has.
    ///
    /// The step does not have to divide the range. A slider from 0 to 1 with a
    /// step of 0.3 snaps to 0, 0.3, 0.6 and 0.9 and cannot reach 1: the snap is to
    /// the nearest step, and the nearest step to 1 *is* 0.9. That is what
    /// "snaps to the nearest step" says, and a caller who needs every step to be
    /// reachable gives a step that divides the range.
    pub fn set_step(&mut self, step: Option<f32>) {
        self.step = step.filter(|step| *step > 0.0);
        let value = self.normalize(self.value.get());
        self.thumb.set(value);
    }

    /// Returns the axis the slider's track runs along.
    #[must_use]
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Sets the axis the slider's track runs along.
    ///
    /// The value does not move, so the thumb does not animate: its position is
    /// derived from the drawn value at paint time, and a new orientation is a
    /// new shape for the same value rather than a change of value.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
    }

    /// Sets how thick the track and the fill are, in pixels.
    pub fn set_track_thickness(&mut self, thickness: f32) {
        self.track_thickness = thickness.max(0.0);
    }

    /// Sets the thumb's radius, in pixels, at rest.
    ///
    /// This is also how far the thumb's centre is kept inside the slider's rect:
    /// a thumb whose centre could reach the very edge would hang half outside it,
    /// and a thumb with a radius bigger than half the slider would have its ends
    /// the wrong way round.
    pub fn set_thumb_radius(&mut self, radius: f32) {
        self.thumb_radius = radius.max(0.0);
    }

    /// Sets the width of the ring drawn around the thumb, in pixels. Zero draws
    /// the thumb with no ring.
    pub fn set_thumb_border(&mut self, width: f32) {
        self.thumb_border_width = width.max(0.0);
    }

    /// Returns the size a slider asks for: `DEFAULT_LENGTH` along its long
    /// axis, and enough across the short one to be grabbed.
    ///
    /// A slider has no content to measure, so this is only for a caller that has
    /// nothing else to go on; a caller that lays the slider out itself gives the
    /// node whatever rect it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](Slider::paint) draws inside whatever it is given.
    #[must_use]
    pub fn size(&self) -> Size {
        let across = (self.track_thickness + (self.thumb_radius + self.thumb_border_width) * 2.0)
            .max(MIN_TOUCH_TARGET);
        match self.orientation {
            Orientation::Horizontal => Size::new(DEFAULT_LENGTH, across),
            Orientation::Vertical => Size::new(across, DEFAULT_LENGTH),
        }
    }

    /// Returns how far along the track `value` sits: `0.0` at the minimum and
    /// `1.0` at the maximum.
    ///
    /// The value is clamped first, so a caller asking where a value outside the
    /// range sits is told about the end of the track rather than about a
    /// position beyond it. A range whose two ends are the same number has one
    /// position and no way along the track to reach it — the same kind of
    /// degenerate case as `MIN_TOUCH_TARGET` — and the thumb sits at the start.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::slider::Slider;
    ///
    /// let mut nodes = Arena::new();
    /// let slider = Slider::new(&mut nodes, 0.0, 200.0);
    /// assert_eq!(slider.fraction(0.0), 0.0);
    /// assert_eq!(slider.fraction(50.0), 0.25);
    /// assert_eq!(slider.fraction(1000.0), 1.0, "past the top is the top");
    /// ```
    #[must_use]
    pub fn fraction(&self, value: f32) -> f32 {
        let span = self.max - self.min;
        // Spelled out rather than written `!(span > 0.0)`: a range whose ends are
        // both `NaN` gives a `NaN` span, and the question here is whether the
        // slider has any distance along its track to divide by, which `NaN` does
        // not answer.
        if span <= 0.0 || span.is_nan() {
            return 0.0;
        }
        bounded((value - self.min) / span)
    }

    /// Returns the value a pointer at `position` over `rect` is asking for.
    ///
    /// This is the inverse of [`thumb_center`](Slider::thumb_center), and it is
    /// public because it is the slider's contract with a pointer: the demo reads
    /// the value out of a drag through it, and a caller drawing a scale of its own
    /// needs the same mapping rather than its own.
    ///
    /// The position is the component along the slider's own axis; the other one
    /// is not read, because the widget's hit test has already decided the pointer
    /// is over it. A position beyond either end gives that end's value.
    #[must_use]
    pub fn value_at(&self, position: Offset, rect: Rect) -> f32 {
        let (from, to) = self.travel(rect);
        let span = to - from;
        let along = match self.orientation {
            Orientation::Horizontal => position.x,
            Orientation::Vertical => position.y,
        };
        let fraction = if span.abs() > f32::EPSILON {
            bounded((along - from) / span)
        } else {
            0.0
        };
        self.normalize(self.min + (self.max - self.min) * fraction)
    }

    /// Returns the point the thumb's centre is drawn at inside `rect`.
    ///
    /// It follows the *drawn* value, not [`value`](Slider::value), so a caller
    /// asking mid-transition is told where the thumb is rather than where it is
    /// going — which is the only version of the question a test about animation
    /// can ask.
    #[must_use]
    pub fn thumb_center(&self, rect: Rect) -> (f32, f32) {
        let (from, to) = self.travel(rect);
        let along = from + (to - from) * self.fraction(self.thumb.get());
        match self.orientation {
            Orientation::Horizontal => (along, rect.y + rect.height / 2.0),
            Orientation::Vertical => (rect.x + rect.width / 2.0, along),
        }
    }

    /// Returns the appearance the slider's value and its dragging flag imply.
    ///
    /// The ring is applied independently of everything else, the way a button's
    /// is: a slider that is both focused and dragged keeps both.
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            thumb: self.normalize(self.value.get()),
            thumb_scale: if self.dragging.get() {
                THUMB_DRAGGED_SCALE
            } else {
                1.0
            },
            track: self.palette.track,
            fill: self.palette.fill,
            thumb_fill: self.palette.thumb_fill,
            thumb_border: self.palette.thumb_border,
            ring_width: if self.focused.get() {
                self.focus_ring.get()
            } else {
                0.0
            },
        }
    }

    /// Applies the appearance the slider's value and dragging flag imply at
    /// once, with no transition.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a slider that has just been given a
    /// [`Palette`](Slider::set_palette) and has never animated — whose colour
    /// properties still hold the neutral defaults [`Slider::new`] wrote, so
    /// without this a themed slider starts out grey — and a caller that has
    /// written a property itself and wants the slider to be that state now.
    ///
    /// Any transition already running is cleared first, so it cannot write over
    /// what this just set when it arrives.
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.thumb.set(style.thumb);
        self.thumb_scale.set(style.thumb_scale);
        self.track.set(style.track);
        self.fill.set(style.fill);
        self.thumb_fill.set(style.thumb_fill);
        self.thumb_border.set(style.thumb_border);
    }

    /// Starts the transitions that carry the slider from wherever it is toward
    /// the appearance [`style`](Slider::style) implies, on `motion`.
    ///
    /// The slider's own clock is cleared first, so the transitions this replaces
    /// stop where they are rather than writing over the new ones when they
    /// arrive — the reason the button owns a clock, for the same reason.
    ///
    /// The target is a snapshot, not a continuous one: a caller re-aims when its
    /// own state moves, exactly as the demo re-aims a button's press when the
    /// theme moves under it.
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.thumb
                .animate_to(style.thumb, motion.duration, motion.easing),
        );
        clock.add(
            self.thumb_scale
                .animate_to(style.thumb_scale, motion.duration, motion.easing),
        );
        clock.add(
            self.track
                .animate_to(style.track, motion.duration, motion.easing),
        );
        clock.add(
            self.fill
                .animate_to(style.fill, motion.duration, motion.easing),
        );
        clock.add(
            self.thumb_fill
                .animate_to(style.thumb_fill, motion.duration, motion.easing),
        );
        clock.add(
            self.thumb_border
                .animate_to(style.thumb_border, motion.duration, motion.easing),
        );
    }

    /// Advances the slider's transitions by `delta`, and returns whether any of
    /// them wrote.
    ///
    /// It is the slider's frame integration: call it once a frame, before the
    /// paint pass, with the time that frame took. The write is what reaches the
    /// node — a property callback registered by the caller marks the node dirty
    /// — so a caller that repaints only when this is true repaints exactly while
    /// something moves.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Returns whether any of the slider's transitions is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Handles `event` as this slider would inside `rect`, and reports whether it
    /// consumed it.
    ///
    /// A [`Tap`](InputEventKind::Tap) and a [`Drag`](InputEventKind::Drag) both
    /// set the value to wherever the pointer is along the track and are
    /// consumed — which is requirement 3's "touch-drag on thumb updates value"
    /// and "touch-drag on track jumps to that position" both being true, because
    /// there is one mapping from a position to a value and a thumb is only a
    /// position the pointer happens to start at. A tap is consumed whether or not
    /// the value moved, so a tap on a slider that is already at that value does
    /// not carry on to whatever is behind it.
    ///
    /// A [`Scroll`](InputEventKind::Scroll) is one notch in whichever direction
    /// its component along the slider's axis points, and is consumed when that
    /// component is not zero. The magnitude is not used: the input module puts a
    /// wheel's notch and a gamepad axis's count in the same field, and the only
    /// thing both agree on is which way the control is being pushed. That is also
    /// the event a steering wheel or an analogue stick arrives as, so the one
    /// axis the input module maps today already reaches a focused slider.
    ///
    /// An arrow key, or the gamepad's d-pad, adjusts the value by one step — the
    /// slider's [`step`](Slider::step) if it has one, a fifth of its range if not
    /// — and is consumed **only while the slider holds focus**. A key press is not
    /// routed by position, so this is called on the focused node by the caller,
    /// and every arrow would otherwise move every slider on screen. Left and
    /// right are a horizontal slider's; up and down are a vertical one's. A key the
    /// slider does not use is left alone and not consumed, so it carries on up
    /// the tree.
    ///
    /// Every other event is left alone and not consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind, Key};
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::slider::Slider;
    ///
    /// let mut nodes = Arena::new();
    /// let mut slider = Slider::new(&mut nodes, 0.0, 10.0);
    /// let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
    ///
    /// // Arrows only reach a slider that holds focus.
    /// let mut right = InputEvent::new(
    ///     InputEventKind::KeyDown {
    ///         key: Key::Keyboard(sdl3::keyboard::Keycode::Right),
    ///         keymod: sdl3::keyboard::Mod::empty(),
    ///     },
    ///     None,
    /// );
    /// assert!(!slider.on_event(&mut right, rect), "an unfocused slider ignores it");
    /// assert!(!right.consumed(), "and lets it travel on");
    ///
    /// slider.focused.set(true);
    /// assert!(slider.on_event(&mut right, rect));
    /// assert_eq!(slider.value.get(), 0.5, "a fifth of 0 to 10");
    /// assert!(right.consumed());
    /// ```
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        match event.kind() {
            InputEventKind::Tap | InputEventKind::Drag { .. } => {
                event.consume();
                if let Some(position) = event.position() {
                    self.set_value(self.value_at(position, rect));
                }
                true
            }
            InputEventKind::Scroll { delta } => {
                let amount = match self.orientation {
                    Orientation::Horizontal => delta.x,
                    Orientation::Vertical => delta.y,
                };
                if amount == 0.0 {
                    return false;
                }
                event.consume();
                self.step_by(if amount > 0.0 { 1.0 } else { -1.0 });
                true
            }
            InputEventKind::KeyDown { key, .. } => {
                if !self.focused.get() {
                    return false;
                }
                let Some(direction) = adjustment(&key, self.orientation) else {
                    return false;
                };
                event.consume();
                self.step_by(direction);
                true
            }
            _ => false,
        }
    }

    /// Returns the draw commands that paint the slider within `rect`.
    ///
    /// The commands are, in order: the focus ring, if the slider is focused and
    /// the ring is not zero wide; the track, a rounded rectangle in the track's
    /// own colour; the fill, from the track's start to the thumb's centre in the
    /// fill's colour; and the thumb, which is two circles — the border, at the
    /// thumb's radius plus its border width, and the thumb's own on top of it.
    ///
    /// The order is the point of all of it. The thumb's fill is above its border,
    /// so the border is a ring rather than a disc; the thumb is above the fill, so
    /// the fill can end at the thumb's centre and be covered by it; and the track
    /// is above the focus ring, so the ring reads as an outline. That last one is
    /// not a detail: a [`DrawCommand::RoundedRect`] *fills* its rect, so a ring
    /// drawn around the slider's whole rect is a white card with a track on it,
    /// and the demo's capture showed exactly that — a focus indicator that read
    /// as a panel. The ring is drawn around the **track** and then covered by it,
    /// which is the trick [`Button`](crate::widgets::button::Button) uses to draw
    /// an outline at all: put the bigger shape down, put the real one over it,
    /// and only the border is left.
    ///
    /// Every size here comes from the drawn value, so a slider mid-transition
    /// paints its thumb where the transition has got to and its fill up to it.
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let track = self.track_rect(rect);
        let radius = self.track_thickness / 2.0;
        let ring = self.focus_ring.get();
        if ring > 0.0 && self.focused.get() {
            painter.rounded_rect(grow(track, ring), radius + ring, self.palette.ring);
        }

        painter.rounded_rect(track, radius, self.track.get());
        let (center_x, center_y) = self.thumb_center(rect);
        let fill = match self.orientation {
            Orientation::Horizontal => Rect::new(
                track.x,
                track.y,
                (center_x - track.x).max(0.0),
                track.height,
            ),
            Orientation::Vertical => Rect::new(
                track.x,
                center_y,
                track.width,
                (track.y + track.height - center_y).max(0.0),
            ),
        };
        painter.rounded_rect(fill, self.track_thickness / 2.0, self.fill.get());

        let radius = (self.thumb_radius * self.thumb_scale.get()).max(0.0);
        let center = (center_x, center_y);
        if self.thumb_border_width > 0.0 {
            painter.circle(
                center,
                radius + self.thumb_border_width,
                self.thumb_border.get(),
            );
        }
        painter.circle(center, radius, self.thumb_fill.get());
        painter.finish()
    }

    /// Returns the first and last points the thumb's centre may occupy along the
    /// slider's axis inside `rect`.
    ///
    /// The two are a radius in from each end, so the thumb stays inside the rect
    /// at both extremes. For a vertical slider the first is the *bottom*: the
    /// screen's y axis points down, so a bigger value is higher up.
    ///
    /// A slider too short to hold its thumb — with less than twice the radius to
    /// move through — has nowhere for the thumb to travel, and its two ends would
    /// be the wrong way round, which would turn a value into a position off the
    /// far side of the rect. Both ends are pinned to the centre of the slider's
    /// own extent in that case, so the thumb sits still rather than escaping.
    fn travel(&self, rect: Rect) -> (f32, f32) {
        let radius = self.thumb_radius.max(0.0);
        let (extent, origin, middle) = match self.orientation {
            Orientation::Horizontal => (rect.width, rect.x, rect.x + rect.width / 2.0),
            Orientation::Vertical => (rect.y + rect.height, rect.y, rect.y + rect.height / 2.0),
        };
        // `extent` is a length, so the travel is that less a radius at each end.
        // It used to subtract the rect's *origin* as well, which every test here
        // missed because every rect in this module starts at (0, 0): a slider laid
        // out anywhere else in the window reported `run` negative, pinned its two
        // ends to its own centre and put every pointer at the minimum. The demo
        // found it, because the demo's slider is at (664, 496).
        let run = extent - radius * 2.0;
        if run <= 0.0 {
            return (middle, middle);
        }
        match self.orientation {
            Orientation::Horizontal => (origin + radius, origin + radius + run),
            Orientation::Vertical => (origin + radius + run, origin + radius),
        }
    }

    /// Returns the track's rounded rectangle inside `rect`.
    ///
    /// The track is inset by half its own thickness rather than by the thumb's
    /// radius: the thumb's centre travels a radius in from each end, so a track
    /// run to the rect's own edges has the thumb centred on its ends, which is
    /// how a slider reads. Insetting the track by the radius instead would leave
    /// the track shorter than the travel and a thumb overhanging both ends of it.
    fn track_rect(&self, rect: Rect) -> Rect {
        let thickness = self.track_thickness;
        match self.orientation {
            Orientation::Horizontal => Rect::new(
                rect.x + thickness / 2.0,
                rect.y + (rect.height - thickness) / 2.0,
                (rect.width - thickness).max(0.0),
                thickness,
            ),
            Orientation::Vertical => Rect::new(
                rect.x + (rect.width - thickness) / 2.0,
                rect.y + thickness / 2.0,
                thickness,
                (rect.height - thickness).max(0.0),
            ),
        }
    }

    /// Returns `value` clamped to the range and snapped to the step.
    fn normalize(&self, value: f32) -> f32 {
        let clamped = bounded_to(value, self.min, self.max);
        match self.step {
            Some(step) => {
                let steps = ((clamped - self.min) / step).round();
                bounded_to(self.min + steps * step, self.min, self.max)
            }
            None => clamped,
        }
    }

    /// Moves the value by `direction` steps, where `direction` is `1.0` or
    /// `-1.0`.
    ///
    /// A slider with no step moves by [`KEY_STEP_FRACTION`] of its range, which
    /// is the only figure that means the same thing on a 0–100 volume and a 0–1
    /// one. The value is normalized on the way in, so a step that lands off the
    /// grid snaps onto it and one that leaves the range is clamped back into it.
    fn step_by(&self, direction: f32) {
        let step = match self.step {
            Some(step) => step,
            None => (self.max - self.min) * KEY_STEP_FRACTION,
        };
        self.set_value(self.value.get() + direction * step);
    }

    /// Writes `value`, clamped and stepped, and the thumb with it.
    ///
    /// The thumb is written at once and the clock cleared: an interaction is
    /// immediate, and a thumb animating toward a finger is a thumb the finger has
    /// already passed. The value is only written, and the callback only fired,
    /// when the number actually moved — a finger resting past the end of the track
    /// moves nothing and reports nothing.
    fn set_value(&self, value: f32) {
        let value = self.normalize(value);
        let moved = (self.value.get() - value).abs() > f32::EPSILON;
        self.clock.borrow_mut().clear();
        self.thumb.set(value);
        if moved {
            self.value.set(value);
            self.on_change.call(value);
        }
    }
}

/// Returns `min` and `max` in order, with `min` never above `max`.
///
/// `f32::min` and `f32::max` return the other operand when one of the two is
/// `NaN`, so a caller that has lost track of a bound gets a usable range rather
/// than a `NaN` one. Two `NaN` bounds cannot be made sense of, and they leave the
/// slider's mapping `NaN`, which draws nothing — a caller error that shows as an
/// empty slider rather than as a frame that takes the process down.
fn ordered(min: f32, max: f32) -> (f32, f32) {
    (min.min(max), min.max(max))
}

/// Returns `value` in `0.0..=1.0`.
///
/// The two comparisons rather than [`f32::clamp`], for the reason
/// [`bounded_to`] gives.
#[allow(clippy::manual_clamp)]
fn bounded(value: f32) -> f32 {
    value.max(0.0).min(1.0)
}

/// Returns `value` between `low` and `high`.
///
/// This is [`bounded`] with a range of its own, and it is written as two
/// comparisons rather than [`f32::clamp`] for the reason it is written that way:
/// `clamp` panics when its bounds are the wrong way round, and a slider whose
/// range a caller has got the wrong way up must not take a frame down to say so.
/// `max` then `min` is the same answer for an ordered pair and cannot panic, and
/// a `NaN` bound is passed over rather than propagated.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::slider::Slider;
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
///
/// let mut nodes = Arena::new();
/// let slider = Slider::new(&mut nodes, 0.0, 10.0);
/// // A value outside the range, and a reversed one, both give an answer.
/// assert_eq!(slider.value_at(
///     ui_core::layout::Offset::new(-100.0, 0.0),
///     ui_core::paint::Rect::new(0.0, 0.0, 100.0, 40.0),
/// ), 0.0);
/// ```
#[allow(clippy::manual_clamp)]
fn bounded_to(value: f32, low: f32, high: f32) -> f32 {
    value.max(low).min(high)
}

/// Returns `rect` grown by `by` on every side. A negative `by` insets it.
fn grow(rect: Rect, by: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// Returns the direction an adjustment key moves the value in, if it is one.
///
/// The d-pad is in the same match as the arrow keys because the input module
/// already maps a gamepad's buttons into [`Key`] and a slider's keys are
/// whichever four the caller pressed: the d-pad is the four directions a
/// controller has, and it is the half of "gamepad adjusts the value" that needs
/// no change to the input module.
///
/// An analogue stick is the other half and is not here: the module maps exactly
/// one gamepad axis into an event — the steering wheel's scroll, on
/// [`STEERING_WHEEL_SCROLL_AXIS`](crate::input::STEERING_WHEEL_SCROLL_AXIS) —
/// and it arrives as a [`Scroll`](InputEventKind::Scroll), which this slider
/// handles. Mapping a stick's axes is a change to `input.rs`, and is recorded as
/// a decision rather than made here.
fn adjustment(key: &Key, orientation: Orientation) -> Option<f32> {
    use sdl3::gamepad::Button as Pad;
    match (orientation, key) {
        (Orientation::Horizontal, Key::Keyboard(sdl3::keyboard::Keycode::Right))
        | (Orientation::Horizontal, Key::Gamepad(Pad::DPadRight))
        | (Orientation::Vertical, Key::Keyboard(sdl3::keyboard::Keycode::Up))
        | (Orientation::Vertical, Key::Gamepad(Pad::DPadUp)) => Some(1.0),
        (Orientation::Horizontal, Key::Keyboard(sdl3::keyboard::Keycode::Left))
        | (Orientation::Horizontal, Key::Gamepad(Pad::DPadLeft))
        | (Orientation::Vertical, Key::Keyboard(sdl3::keyboard::Keycode::Down))
        | (Orientation::Vertical, Key::Gamepad(Pad::DPadDown)) => Some(-1.0),
        _ => None,
    }
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback
/// for a token a caller has written the wrong variant into — and black rather
/// than a panic, because a mistyped theme token is not worth taking a frame down
/// for. It is the button's own helper, repeated rather than imported: it is four
/// lines, and a shared module for one four-line helper is a module.
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
    use crate::layout::{Constraints, Layout, LayoutState};
    use crate::theme::ThemeToken;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// The rect every geometry test lays a slider out in: 100 by 40, which is
    /// wide enough for a 12-pixel thumb at each end and thin enough that the
    /// track is unmistakably a line down the middle.
    const RECT: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 40.0,
    };

    /// The same slider somewhere else in a window: 240 by 44 at (664, 496),
    /// which is where the demo puts it. Every other fixture here starts at the
    /// origin, so this is the one that catches an origin being read as an extent.
    const OFFSET_RECT: Rect = Rect {
        x: 664.0,
        y: 496.0,
        width: 240.0,
        height: 44.0,
    };

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Advances `slider`'s transitions by `millis` and reports whether anything
    /// moved.
    ///
    /// The slider's own `tick` is what a frame calls, and it answers whether a
    /// repaint is needed; a test that only wants time to pass goes through here
    /// so that the answer is passed on rather than dropped on the floor.
    fn tick(slider: &Slider, millis: u64) -> bool {
        slider.tick(ms(millis))
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a
    /// value at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// A slider in the dark theme's palette, with its colours already arrived at
    /// that palette.
    ///
    /// Setting a palette does not move the slider by itself — a theme switch is
    /// animated — so a test that wants a slider *in* a palette asks for the
    /// animation and ticks it out. Otherwise the colours would be the ones
    /// `Slider::new` starts with and the palette's would only be targets.
    fn slider(min: f32, max: f32) -> (Arena<WidgetNode>, Slider) {
        let mut nodes = Arena::new();
        let mut slider = Slider::new(&mut nodes, min, max);
        slider.set_palette(Palette::from_theme(&Theme::dark()));
        slider.snap_to_state();
        (nodes, slider)
    }

    /// The rounded rectangles a paint recorded, with their radii and colours.
    fn rounded(commands: &[DrawCommand]) -> Vec<(Rect, f32, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color,
                } => Some((*rect, *radius, *color)),
                _ => None,
            })
            .collect()
    }

    /// The circles a paint recorded, as their centre, radius and colour.
    fn circles(commands: &[DrawCommand]) -> Vec<((f32, f32), f32, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Circle {
                    center,
                    radius,
                    color,
                } => Some((*center, *radius, *color)),
                _ => None,
            })
            .collect()
    }

    /// A short name for each recorded command, in the order they were recorded.
    ///
    /// The filtering helpers above cannot see order, and the order is half of what
    /// a slider is: the thumb has to be drawn over the fill. **This helper sees
    /// shapes and not sizes**, so it cannot tell a thumb's border from the thumb's
    /// own circle — two `Circle`s are one word to it, and swapping the two leaves
    /// its answer unchanged. The half of the order that is *within* the thumb is
    /// asserted by radius instead, in
    /// `a_slider_paints_its_thumb_over_its_fill_and_its_border_under_itself`.
    fn shapes(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::RoundedRect { .. } => "rect",
                DrawCommand::Circle { .. } => "circle",
                _ => "other",
            })
            .collect()
    }

    /// Asserts two values are within a millionth of one another.
    ///
    /// The step grid is computed in `f32`, so `3 * 0.3` is `0.89999997` and
    /// `0.3 * 3 * 12` is `13.799999`: a test that wrote the decimal out would be
    /// asserting the compiler's rounding rather than the widget. Everything a
    /// slider computes exactly — a whole number of steps on a step that divides
    /// its range — is asserted with `assert_eq!` instead.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-5, "{what}: {got} against {want}");
    }

    /// A tap at `(x, y)`.
    fn tap_at(x: f32, y: f32) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)))
    }

    /// A drag of `delta` ending at `(x, y)`.
    fn drag_to(x: f32, y: f32, delta: Offset) -> InputEvent {
        InputEvent::new(InputEventKind::Drag { delta }, Some(Offset::new(x, y)))
    }

    /// A scroll of `(x, y)`, which is what a wheel notch and a gamepad axis both
    /// arrive as.
    fn scroll(x: f32, y: f32) -> InputEvent {
        InputEvent::new(
            InputEventKind::Scroll {
                delta: Offset::new(x, y),
            },
            None,
        )
    }

    /// A key press of `key`, which carries no position.
    fn key_down(key: Key) -> InputEvent {
        InputEvent::new(
            InputEventKind::KeyDown {
                key,
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        )
    }

    /// A counter of the values the change callback was given, in order.
    fn changes(slider: &mut Slider) -> Rc<RefCell<Vec<f32>>> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorded = Rc::clone(&seen);
        slider.on_change = Callback::from_fn(move |value| recorded.borrow_mut().push(value));
        seen
    }

    /// Hangs `slider` on a panel and lays it out, and returns the panel's handle.
    ///
    /// The panel is a stack, so the slider sits at its origin, and the slider's
    /// own node is given the rect the geometry is asserted against. A tap
    /// therefore has a real hit test to pass or fail, and a node behind the slider
    /// to reach if it does not.
    fn on_a_panel(slider: &Slider, nodes: &mut Arena<WidgetNode>, panel_size: Size) -> Handle {
        let panel = node::create(nodes, LayoutState::new());
        assert!(
            node::attach(nodes, panel, slider.handle()),
            "the slider is hung on the panel"
        );
        let size = slider.size();
        if let Some(node) = nodes.get_mut(slider.handle()) {
            node.layout_mut().set_constraints(Constraints::tight(size));
        }
        Layout::new(nodes).layout(panel, Constraints::tight(panel_size));
        panel
    }

    #[test]
    fn a_slider_holds_the_properties_the_task_gives_it() {
        let (_nodes, slider) = slider(0.0, 100.0);
        assert_eq!(slider.min(), 0.0);
        assert_eq!(slider.max(), 100.0);
        assert_eq!(slider.value.get(), 0.0, "and it starts at its minimum");
        assert_eq!(slider.thumb.get(), 0.0, "with the thumb on it");
        assert_eq!(slider.step(), None, "no step until one is asked for");
        assert_eq!(slider.orientation(), Orientation::Horizontal);
        assert!(
            !slider.on_change.is_set(),
            "and no callback until one is given"
        );
        assert!(!slider.dragging.get());
        assert!(!slider.focused.get());
    }

    #[test]
    fn a_slider_orders_its_range_rather_than_dividing_by_a_negative_span() {
        // A caller that passes the two the wrong way round gets a slider, not a
        // thumb that escapes off the far end of the track.
        let (_nodes, slider) = slider(100.0, 0.0);
        assert_eq!(slider.min(), 0.0);
        assert_eq!(slider.max(), 100.0);
        assert_eq!(slider.value.get(), 0.0, "at the minimum it was given");
        assert_eq!(
            slider.value_at(Offset::new(1000.0, 0.0), RECT),
            100.0,
            "and the whole track is above it"
        );

        let mut slider = slider;
        slider.set_range(10.0, -10.0);
        assert_eq!((slider.min(), slider.max()), (-10.0, 10.0));
    }

    #[test]
    fn a_new_range_pulls_the_value_back_inside_it() {
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.value.set(8.0);
        slider.thumb.set(8.0);
        slider.set_range(0.0, 5.0);

        assert_eq!(slider.value.get(), 5.0, "the value is pulled back");
        assert_eq!(
            slider.thumb.get(),
            5.0,
            "and the thumb comes with it at once, rather than animating into a \
             track that has not been drawn yet"
        );
        assert!(
            !slider.is_animating(),
            "so nothing is left running to move it afterwards"
        );
    }

    #[test]
    fn the_default_geometry_is_the_values_this_module_documents() {
        // These are constants, not derived numbers, and the task's "rounded
        // rectangle" and "thumb scales up slightly" both hang off them.
        let (_nodes, slider) = slider(0.0, 1.0);
        assert_eq!(slider.track_thickness, TRACK_THICKNESS);
        assert_eq!(slider.thumb_radius, THUMB_RADIUS);
        assert_eq!(slider.thumb_border_width, THUMB_BORDER);
        assert_eq!(slider.focus_ring.get(), FOCUS_RING);
        assert_eq!(TRACK_THICKNESS, 6.0);
        assert_eq!(THUMB_RADIUS, 12.0);
        assert_eq!(THUMB_BORDER, 2.0);
        assert_eq!(
            THUMB_DRAGGED_SCALE, 1.15,
            "and the drag scale is 'slightly'"
        );
    }

    #[test]
    fn the_thumb_travels_a_radius_inside_each_end_of_the_track() {
        let (_nodes, slider) = slider(0.0, 10.0);
        assert_eq!(slider.thumb_center(RECT).0, 12.0, "at the minimum");

        slider.value.set(10.0);
        slider.thumb.set(10.0);
        assert_eq!(
            slider.thumb_center(RECT).0,
            88.0,
            "and at the maximum, a radius in from the far end"
        );

        slider.value.set(5.0);
        slider.thumb.set(5.0);
        assert_eq!(slider.thumb_center(RECT), (50.0, 20.0), "half way along");
    }

    #[test]
    fn a_slider_away_from_the_origin_maps_positions_to_the_same_values() {
        // Every other geometry test lays its slider out at (0, 0), which is a
        // blind spot: a rect's origin and a rect's extent are different numbers,
        // and only a slider somewhere else on the window tells them apart. This
        // one is where the demo found that difference.
        let (_nodes, slider) = slider(0.0, 10.0);
        let at = |slider: &Slider, x: f32| slider.value_at(Offset::new(x, 538.0), OFFSET_RECT);
        let rect = OFFSET_RECT;
        assert_eq!(rect.x, 664.0, "the fixture really is away from the origin");

        assert_eq!(at(&slider, rect.x + 12.0), 0.0, "at the left end");
        assert_eq!(at(&slider, rect.x + rect.width / 2.0), 5.0, "in the middle");
        assert_eq!(
            at(&slider, rect.x + rect.width - 12.0),
            10.0,
            "and at the right end, a radius in"
        );
        assert_eq!(
            slider.thumb_center(rect).0,
            rect.x + 12.0,
            "with the thumb at the minimum, inside the rect rather than at 12"
        );
    }

    #[test]
    fn a_vertical_slider_is_larger_at_the_top_than_at_the_bottom() {
        // The screen's y axis points down, so a vertical slider's minimum is at
        // the bottom of its rect. This is the test that orientation is real: it
        // changes which end of the rect the value is measured from.
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.set_orientation(Orientation::Vertical);
        assert_eq!(
            slider.thumb_center(RECT).1,
            28.0,
            "the minimum is at the bottom, a radius in"
        );

        slider.value.set(10.0);
        slider.thumb.set(10.0);
        assert_eq!(
            slider.thumb_center(RECT).1,
            12.0,
            "and the maximum at the top"
        );
        assert_eq!(slider.thumb_center(RECT).0, 50.0, "centred across");
    }

    #[test]
    fn a_vertical_slider_asks_for_its_length_downwards() {
        let (_nodes, mut slider) = slider(0.0, 1.0);
        slider.set_orientation(Orientation::Vertical);
        let size = slider.size();
        assert!(
            size.height > size.width,
            "a vertical slider is long down: {size:?}"
        );
    }

    #[test]
    fn the_fraction_of_a_value_outside_the_range_is_the_end_it_is_past() {
        // The clamp inside `fraction` is what the doctest on it also asserts, and
        // it is asserted here as well because `fraction` is public: a caller
        // asking where a value *it* computed would put the thumb on the track
        // without it. Every path the widget itself takes clamps first, which is
        // why no other test in this module could see its absence.
        let (_nodes, slider) = slider(0.0, 10.0);
        assert_eq!(slider.fraction(-5.0), 0.0);
        assert_eq!(slider.fraction(5.0), 0.5);
        assert_eq!(slider.fraction(15.0), 1.0);
    }

    #[test]
    fn a_value_is_clamped_to_the_sliders_range() {
        let (_nodes, slider) = slider(0.0, 10.0);
        assert_eq!(slider.value_at(Offset::new(-50.0, 20.0), RECT), 0.0);
        assert_eq!(slider.value_at(Offset::new(500.0, 20.0), RECT), 10.0);
        assert_eq!(slider.value_at(Offset::new(50.0, 20.0), RECT), 5.0);
    }

    #[test]
    fn a_range_whose_ends_are_the_same_number_puts_the_thumb_still() {
        let (_nodes, slider) = slider(3.0, 3.0);
        assert_eq!(slider.value.get(), 3.0);
        assert_eq!(
            slider.fraction(3.0),
            0.0,
            "there is one position and no way along the track to it"
        );
        assert_eq!(slider.thumb_center(RECT), (12.0, 20.0));
        assert_eq!(
            slider.value_at(Offset::new(99.0, 20.0), RECT),
            3.0,
            "and no pointer can move it"
        );
    }

    #[test]
    fn a_step_snaps_the_value_to_the_nearest_one_on_the_grid() {
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.set_step(Some(2.5));

        assert_eq!(slider.value_at(Offset::new(50.0, 20.0), RECT), 5.0);
        // 47 is 2.47 steps up, and two steps is nearer than three.
        slider.value.set(4.7);
        slider.set_value(4.7);
        assert_eq!(slider.value.get(), 5.0, "up to the nearest step");
        assert_eq!(slider.value_at(Offset::new(45.0, 20.0), RECT), 5.0);
    }

    #[test]
    fn a_step_is_measured_from_the_minimum_not_from_zero() {
        // A slider from 1 to 2 with a step of 0.5 has its grid at 1, 1.5 and 2 —
        // not at 0.5, 1 and 1.5, which is what anchoring on zero would give.
        let (_nodes, mut slider) = slider(1.0, 2.0);
        slider.set_step(Some(0.5));
        slider.set_value(1.4);
        assert_eq!(slider.value.get(), 1.5);
        slider.set_value(1.6);
        assert_eq!(slider.value.get(), 1.5);
        slider.set_value(1.75);
        assert_eq!(slider.value.get(), 2.0);
    }

    #[test]
    fn a_step_of_zero_or_less_is_no_step() {
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.set_step(Some(2.0));
        assert_eq!(slider.step(), Some(2.0));

        for useless in [Some(0.0), Some(-1.0), Some(f32::NAN)] {
            slider.set_step(useless);
            assert_eq!(slider.step(), None, "{useless:?} is not a step");
        }
    }

    #[test]
    fn a_step_that_does_not_divide_the_range_leaves_the_top_unreachable() {
        // "Snaps to the nearest step" says nothing about the ends being on the
        // grid, and 0.3 does not divide 1: the snap to the nearest step to 1 is
        // 0.9. What matters is that the answer is inside the range either way.
        let (_nodes, mut slider) = slider(0.0, 1.0);
        slider.set_step(Some(0.3));
        assert_close(
            slider.value_at(Offset::new(500.0, 20.0), RECT),
            0.9,
            "the nearest step to the top of the range",
        );
        assert_close(
            slider.value_at(Offset::new(50.0, 20.0), RECT),
            0.6,
            "and the nearest step to half of it",
        );
    }

    #[test]
    fn a_programmatic_change_animates_the_thumb_toward_the_value() {
        // The contract is that the thumb *moves* rather than jumping, so this
        // checks the middle of the transition and not only its end: a `set`
        // instead of an animation would pass an end-only assertion.
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.value.set(10.0);
        slider.animate_to_state(motion());

        assert_eq!(
            slider.thumb.get(),
            0.0,
            "the thumb starts where it was, so a frame drawn now is right"
        );
        assert!(slider.tick(ms(50)));
        let half = slider.thumb.get();
        assert!(half > 0.0 && half < 10.0, "half way through, it has moved");
        assert_eq!(half, 5.0, "to exactly half, on a linear curve");

        assert!(slider.tick(ms(50)));
        assert_eq!(slider.thumb.get(), 10.0, "and it arrives");
        assert!(!slider.is_animating());
    }

    #[test]
    fn the_fill_follows_the_animating_thumb_rather_than_the_value() {
        // Requirement 4's second half: the fill has to move with the thumb, and
        // not jump to the value the thumb is still travelling towards. The fill's
        // end is the thumb's centre, so its width is the measure of both.
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.value.set(10.0);
        slider.animate_to_state(motion());

        let fill_width = |slider: &Slider| {
            rounded(&slider.paint(RECT))
                .get(1)
                .map(|(rect, _, _)| rect.width)
                .unwrap_or_default()
        };
        let at_start = fill_width(&slider);
        tick(&slider, 50);
        let halfway = fill_width(&slider);
        tick(&slider, 50);
        let arrived = fill_width(&slider);

        assert!(
            at_start < halfway,
            "the fill grows as the thumb moves: {at_start}"
        );
        assert!(halfway < arrived, "all the way: {halfway} then {arrived}");
        assert_eq!(arrived - halfway, halfway - at_start, "linearly");
    }

    #[test]
    fn a_second_change_replaces_the_first_rather_than_racing_it() {
        // A value written twice in quick succession used to be a race: both
        // transitions wrote the thumb and the older one arriving last left it
        // short of the value. The slider's own clock is what rules that out.
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.value.set(10.0);
        slider.animate_to_state(motion());
        tick(&slider, 50);
        slider.value.set(2.0);
        slider.animate_to_state(motion());
        for _ in 0..4 {
            tick(&slider, 50);
        }
        assert_eq!(
            slider.thumb.get(),
            2.0,
            "the thumb ends at the newest value"
        );
        assert!(!slider.is_animating(), "and nothing is left running");
    }

    #[test]
    fn the_dragged_thumb_scales_up_and_back_down() {
        let (_nodes, slider) = slider(0.0, 10.0);
        assert_eq!(
            slider.style().thumb_scale,
            1.0,
            "at rest it is its own size"
        );

        slider.dragging.set(true);
        let dragged = slider.style();
        assert_eq!(dragged.thumb_scale, THUMB_DRAGGED_SCALE);
        slider.animate_to_state(motion());
        tick(&slider, 100);
        assert_eq!(slider.thumb_scale.get(), THUMB_DRAGGED_SCALE);

        slider.dragging.set(false);
        slider.animate_to_state(motion());
        tick(&slider, 100);
        assert_eq!(
            slider.thumb_scale.get(),
            1.0,
            "and back when it is released"
        );
    }

    #[test]
    fn the_thumb_is_drawn_at_the_dragged_scale() {
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.value.set(10.0);
        slider.thumb.set(10.0);
        assert_eq!(
            circles(&slider.paint(RECT))
                .last()
                .map(|(_, radius, _)| *radius),
            Some(12.0),
            "at rest it is the radius it was given"
        );

        slider.dragging.set(true);
        slider.snap_to_state();
        let dragged = circles(&slider.paint(RECT))
            .last()
            .map(|(_, radius, _)| *radius)
            .unwrap_or_default();
        assert_close(dragged, 13.8, "and 1.15 of it while it is dragged");
        assert!(
            dragged > 12.0,
            "which is a thumb that grew rather than one that shrank"
        );
    }

    #[test]
    fn snapping_puts_a_themed_slider_where_its_theme_says_at_once() {
        // The failure this guards: a slider given a palette but never aimed still
        // paints the neutral defaults `Slider::new` wrote, so a themed slider
        // starts out grey and only becomes the theme's colour once something has
        // moved it. Built from `new` rather than through the `slider` helper,
        // which snaps.
        let mut nodes = Arena::new();
        let mut slider = Slider::new(&mut nodes, 0.0, 10.0);
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(themed.track, slider.track.get(), "so this can fail");

        slider.set_palette(themed);
        slider.snap_to_state();

        assert_eq!(slider.track.get(), themed.track);
        assert_eq!(slider.fill.get(), themed.fill);
        assert_eq!(slider.thumb_fill.get(), themed.thumb_fill);
        assert_eq!(slider.thumb_border.get(), themed.thumb_border);
        assert_eq!(slider.thumb_scale.get(), 1.0);
        assert!(!slider.is_animating(), "a snap is not a transition");
    }

    #[test]
    fn snapping_a_mid_transition_slider_back_out_ends_the_transition() {
        // Without the `clear`, a transition already running writes over what the
        // snap just set when it arrives, and the slider drifts off again.
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.value.set(10.0);
        slider.animate_to_state(motion());
        assert!(slider.tick(ms(10)), "the value is animating");

        slider.value.set(0.0);
        slider.snap_to_state();
        assert_eq!(slider.thumb.get(), 0.0, "the snap put it at the new value");

        assert!(!slider.tick(ms(500)), "and nothing arrives afterwards");
        assert_eq!(slider.thumb.get(), 0.0);
    }

    #[test]
    fn setting_the_palette_leaves_the_slider_where_it_is() {
        // A theme switch is animated by the caller setting the palette and
        // animating toward it; a palette that moved the slider on its own would
        // make every theme switch instantaneous.
        let (_nodes, mut slider) = slider(0.0, 10.0);
        let before = slider.fill.get();
        slider.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(slider.fill.get(), before);
    }

    #[test]
    fn a_slider_paints_its_track_its_fill_and_its_thumb() {
        let (_nodes, slider) = slider(0.0, 10.0);
        let commands = slider.paint(RECT);

        let filled = rounded(&commands);
        assert_eq!(filled.len(), 2, "a track and a fill");
        assert_eq!(filled[0].0, Rect::new(3.0, 17.0, 94.0, 6.0), "the track");
        assert_eq!(
            filled[0].1, 3.0,
            "with half the track's thickness as radius"
        );
        assert_eq!(filled[0].2, slider.track.get());
        assert_eq!(
            filled[1].0,
            Rect::new(3.0, 17.0, 9.0, 6.0),
            "the fill, up to the thumb"
        );
        assert_eq!(filled[1].2, slider.fill.get());

        let drawn = circles(&commands);
        assert_eq!(drawn.len(), 2, "a thumb, which is a border and a circle");
        assert_eq!(drawn[0].0, (12.0, 20.0), "at the minimum");
        assert_eq!(drawn[0].1, 14.0, "the border is the radius plus its width");
        assert_eq!(drawn[1].1, 12.0, "and the thumb's own circle is on top");
        assert_eq!(drawn[1].2, slider.thumb_fill.get());
        assert_eq!(drawn[0].2, slider.thumb_border.get());
    }

    #[test]
    fn a_slider_paints_its_thumb_over_its_fill_and_its_border_under_itself() {
        // Both halves of the order need different evidence, and this test needed
        // both. `shapes` sees a rounded rect before a circle, which is the thumb
        // over the fill; it cannot tell two circles apart, so the half that is
        // *within* the thumb — the border under the thumb's own circle — is the
        // radius, the border being the larger of the two.
        //
        // The first version of this test asserted only the shape sequence and its
        // comment claimed the swap left every assertion green. The reviewer
        // swapped the two circles and this test stayed green: `shapes` maps both to
        // "circle", and the two assertions that *do* discriminate compare radii.
        // The name had to earn the claim, so the radii are asserted here too.
        let (_nodes, slider) = slider(0.0, 10.0);
        let commands = slider.paint(RECT);
        assert_eq!(
            shapes(&commands),
            vec!["rect", "rect", "circle", "circle"],
            "the track, the fill, then the thumb's two circles"
        );

        let drawn = circles(&commands);
        assert_eq!(drawn.len(), 2);
        assert!(
            drawn[0].1 > drawn[1].1,
            "the first of the two is the larger — {} against {} — which is the ring, \
             and it is recorded first so that it shows as a ring",
            drawn[0].1,
            drawn[1].1
        );
        assert_eq!(
            drawn[0].1, 14.0,
            "a thumb's border is 12 of thumb and 2 of ring"
        );
        assert_eq!(drawn[1].1, 12.0, "and the thumb's own circle is the radius");
    }

    #[test]
    fn the_fill_runs_from_the_tracks_start_to_the_thumb() {
        let fill_at = |value: f32| {
            let mut nodes = Arena::new();
            let slider = Slider::new(&mut nodes, 0.0, 10.0);
            slider.value.set(value);
            slider.snap_to_state();
            rounded(&slider.paint(RECT))[1].0.width
        };

        assert_eq!(
            fill_at(0.0),
            9.0,
            "at the minimum, up to the thumb's centre"
        );
        assert_eq!(fill_at(5.0), 47.0);
        assert_eq!(fill_at(10.0), 85.0, "and at the maximum");
        assert_eq!(
            fill_at(10.0) - fill_at(5.0),
            38.0,
            "half the track is 38 of fill"
        );
    }

    #[test]
    fn a_zero_width_thumb_border_paints_the_thumb_on_its_own() {
        // A caller that turns the border off gets one circle rather than two, and
        // the thumb is still the top one.
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.set_thumb_border(0.0);
        let drawn = circles(&slider.paint(RECT));
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].1, THUMB_RADIUS);
    }

    #[test]
    fn a_focused_slider_paints_a_ring_around_its_track() {
        // The ring is around the *track*, not around the whole node, and it is
        // recorded before the track that covers it. A `RoundedRect` fills its
        // rect, so a ring around the node would be a card with a track drawn on
        // it — which is what the demo's capture showed before this was fixed, and
        // what every draw-command assertion here called correct: the old rect was
        // a real rect of the right colour, drawn in the right place, at the wrong
        // size for an outline.
        let (_nodes, slider) = slider(0.0, 10.0);
        assert_eq!(
            rounded(&slider.paint(RECT)).len(),
            2,
            "no ring while unfocused"
        );

        slider.focused.set(true);
        let filled = rounded(&slider.paint(RECT));
        assert_eq!(filled.len(), 3, "a focused one has a ring as well");
        assert_eq!(
            filled[0].0,
            Rect::new(1.0, 15.0, 98.0, 10.0),
            "the ring is the track grown by its own width, so its border shows"
        );
        assert_eq!(filled[0].1, 3.0 + FOCUS_RING, "with the track's own radius");
        assert_eq!(filled[0].2, slider.palette().ring);
        assert_eq!(
            filled[1].0,
            Rect::new(3.0, 17.0, 94.0, 6.0),
            "and the track covers the middle of it, which is what leaves the ring"
        );
        assert_eq!(
            shapes(&slider.paint(RECT)),
            vec!["rect", "rect", "rect", "circle", "circle"],
            "recorded ring first, then the track over it"
        );
    }

    #[test]
    fn a_zero_width_focus_ring_is_not_painted() {
        // A theme or a caller that sets the ring's width to zero turns the
        // indicator off; painting a zero-width ring would be a command that draws
        // nothing.
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.focused.set(true);
        slider.focus_ring.set(0.0);
        assert_eq!(rounded(&slider.paint(RECT)).len(), 2);
    }

    #[test]
    fn a_focused_and_dragged_slider_keeps_both() {
        // The states overlap, which is why they are two booleans and not one
        // enum: the ring and the grown thumb are independent of one another.
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.focused.set(true);
        slider.dragging.set(true);
        let style = slider.style();
        assert_eq!(style.ring_width, FOCUS_RING);
        assert_eq!(style.thumb_scale, THUMB_DRAGGED_SCALE);
        assert_eq!(rounded(&slider.paint(RECT)).len(), 3);
    }

    #[test]
    fn a_tap_on_the_track_jumps_the_value_to_that_position() {
        let (_nodes, slider) = slider(0.0, 10.0);
        let mut tap = tap_at(50.0, 20.0);
        assert!(slider.on_event(&mut tap, RECT));
        assert_eq!(slider.value.get(), 5.0, "half way along 0 to 10");
        assert_eq!(slider.thumb.get(), 5.0, "and the thumb is there at once");
        assert!(tap.consumed(), "the tap is consumed");
    }

    #[test]
    fn a_tap_that_changes_nothing_still_consumes_the_event() {
        // The event was aimed at the slider, so letting it through would reach
        // whatever is behind it — and a tap past the end of the track is the case
        // that changes nothing.
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.value.set(10.0);
        slider.thumb.set(10.0);
        let seen = changes(&mut slider);
        let mut tap = tap_at(500.0, 20.0);
        assert!(slider.on_event(&mut tap, RECT));
        assert!(tap.consumed());
        assert_eq!(slider.value.get(), 10.0, "clamped at the maximum");
        assert!(
            seen.borrow().is_empty(),
            "and nothing changed, so nothing is reported"
        );
    }

    #[test]
    fn a_drag_follows_the_pointer_and_reports_every_value_it_moves_to() {
        let (_nodes, mut slider) = slider(0.0, 100.0);
        let seen = changes(&mut slider);

        for x in [20.0, 40.0, 60.0] {
            let mut drag = drag_to(x, 20.0, Offset::new(20.0, 0.0));
            assert!(slider.on_event(&mut drag, RECT));
        }
        assert!(
            seen.borrow().len() >= 2,
            "each step reported: {:?}",
            seen.borrow()
        );
        assert_eq!(*seen.borrow().last().unwrap(), slider.value.get());
        assert_eq!(slider.thumb.get(), slider.value.get(), "the thumb follows");
    }

    #[test]
    fn a_drag_past_the_end_of_the_track_clamps_and_then_stops_reporting() {
        // A finger resting past the end of the track moves nothing, and a caller
        // driving a speaker from this is not told about every frame of it.
        let (_nodes, mut slider) = slider(0.0, 10.0);
        let seen = changes(&mut slider);
        for x in [500.0, 600.0, 700.0] {
            let mut drag = drag_to(x, 20.0, Offset::new(100.0, 0.0));
            assert!(slider.on_event(&mut drag, RECT), "and it is still consumed");
        }
        assert_eq!(slider.value.get(), 10.0, "clamped at the maximum");
        assert_eq!(
            seen.borrow().as_slice(),
            &[10.0],
            "which was reported once, the time it arrived there"
        );
    }

    #[test]
    fn a_drag_on_a_vertical_slider_reads_the_vertical_axis() {
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.set_orientation(Orientation::Vertical);
        let mut drag = drag_to(99.0, 20.0, Offset::new(0.0, -10.0));
        assert!(slider.on_event(&mut drag, RECT));
        assert_eq!(
            slider.value.get(),
            5.0,
            "the pointer's y decides it, and its x is not read"
        );
    }

    #[test]
    fn an_arrow_key_adjusts_the_value_of_a_focused_slider() {
        use sdl3::keyboard::Keycode;
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.focused.set(true);

        let mut right = key_down(Key::Keyboard(Keycode::Right));
        assert!(slider.on_event(&mut right, RECT));
        assert_eq!(slider.value.get(), 0.5, "a fifth of the range");
        assert!(right.consumed());

        let mut left = key_down(Key::Keyboard(Keycode::Left));
        assert!(slider.on_event(&mut left, RECT));
        assert_eq!(slider.value.get(), 0.0, "and back, where the range starts");
    }

    #[test]
    fn an_arrow_key_does_nothing_to_an_unfocused_slider() {
        // A key press is not routed by position, so it reaches the slider only
        // because the caller sent it to the focused node. Every slider would
        // otherwise move on every arrow press.
        use sdl3::keyboard::Keycode;
        let (_nodes, slider) = slider(0.0, 10.0);
        let mut right = key_down(Key::Keyboard(Keycode::Right));
        assert!(!slider.on_event(&mut right, RECT));
        assert_eq!(slider.value.get(), 0.0);
        assert!(!right.consumed(), "and the key carries on to the tree");
    }

    #[test]
    fn a_vertical_slider_takes_up_and_down_and_ignores_left_and_right() {
        use sdl3::keyboard::Keycode;
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.set_orientation(Orientation::Vertical);
        slider.focused.set(true);

        let mut up = key_down(Key::Keyboard(Keycode::Up));
        assert!(
            slider.on_event(&mut up, RECT),
            "up increases a vertical slider"
        );
        assert_eq!(slider.value.get(), 0.5);

        let mut left = key_down(Key::Keyboard(Keycode::Left));
        assert!(
            !slider.on_event(&mut left, RECT),
            "and left is not a key a vertical slider has"
        );
        assert!(!left.consumed());
        assert_eq!(slider.value.get(), 0.5, "so nothing moved");
    }

    #[test]
    fn a_slider_with_a_step_uses_it_for_its_keys() {
        use sdl3::keyboard::Keycode;
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.set_step(Some(2.0));
        slider.focused.set(true);

        let mut right = key_down(Key::Keyboard(Keycode::Right));
        assert!(slider.on_event(&mut right, RECT));
        assert_eq!(
            slider.value.get(),
            2.0,
            "the step, not a fifth of the range"
        );
    }

    #[test]
    fn a_key_at_the_end_of_the_range_does_not_report_a_change() {
        use sdl3::keyboard::Keycode;
        let (_nodes, mut slider) = slider(0.0, 10.0);
        slider.focused.set(true);
        slider.value.set(10.0);
        let seen = changes(&mut slider);

        let mut right = key_down(Key::Keyboard(Keycode::Right));
        assert!(
            slider.on_event(&mut right, RECT),
            "the key is still consumed"
        );
        assert_eq!(
            slider.value.get(),
            10.0,
            "and the value cannot leave the range"
        );
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn the_gamepad_d_pad_adjusts_the_value() {
        use sdl3::gamepad::Button as Pad;
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.focused.set(true);

        let mut right = key_down(Key::Gamepad(Pad::DPadRight));
        assert!(slider.on_event(&mut right, RECT), "the d-pad's right");
        assert_eq!(slider.value.get(), 0.5);

        let mut down = key_down(Key::Gamepad(Pad::DPadDown));
        assert!(
            !slider.on_event(&mut down, RECT),
            "and a horizontal slider has no use for its up and down"
        );
    }

    #[test]
    fn a_scroll_along_the_sliders_axis_adjusts_the_value() {
        // This is the event a steering wheel or an analogue stick arrives as: the
        // input module maps one gamepad axis into a positionless `Scroll`, and a
        // slider is what it is for. The magnitude is a wheel's notch or an axis's
        // count and is not used; the direction is all both agree on.
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.focused.set(true);

        let mut out = scroll(1.0, 0.0);
        assert!(slider.on_event(&mut out, RECT));
        assert_eq!(slider.value.get(), 0.5, "one notch up, whatever its size");
        assert!(out.consumed());

        let mut back = scroll(-32000.0, 0.0);
        assert!(slider.on_event(&mut back, RECT));
        assert_eq!(slider.value.get(), 0.0, "and one back");
    }

    #[test]
    fn a_scroll_across_the_sliders_axis_is_left_for_somebody_else() {
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.focused.set(true);
        let mut vertical = scroll(0.0, 1.0);
        assert!(
            !slider.on_event(&mut vertical, RECT),
            "a horizontal slider reads the wheel's horizontal component only"
        );
        assert!(!vertical.consumed());
        assert_eq!(slider.value.get(), 0.0);

        let mut still = scroll(0.0, 0.0);
        assert!(
            !slider.on_event(&mut still, RECT),
            "and a scroll of exactly zero moves nothing and is nobody's"
        );
    }

    #[test]
    fn a_key_the_slider_does_not_use_is_left_alone() {
        use sdl3::keyboard::Keycode;
        let (_nodes, slider) = slider(0.0, 10.0);
        slider.focused.set(true);
        for key in [
            Key::Keyboard(Keycode::Tab),
            Key::Keyboard(Keycode::Return),
            Key::Gamepad(sdl3::gamepad::Button::South),
        ] {
            let mut event = key_down(key);
            assert!(!slider.on_event(&mut event, RECT), "{key:?} is not its key");
            assert!(!event.consumed(), "{key:?} carries on to the tree");
        }
    }

    #[test]
    fn a_tap_outside_the_slider_reaches_the_panel_behind_it() {
        // The other half of "a tap is consumed": a tap that misses the slider's
        // own rect is not its to consume, and the slider never fires.
        let (mut nodes, mut slider) = slider(0.0, 10.0);
        let seen = changes(&mut slider);
        let panel = on_a_panel(&slider, &mut nodes, Size::new(400.0, 200.0));

        let mut event = tap_at(300.0, 150.0);
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == slider.handle() {
                slider.on_event(event, RECT);
            }
        });
        assert_eq!(reached, vec![panel], "the panel took it");
        assert_eq!(slider.value.get(), 0.0, "and the slider never fired");
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn a_tap_on_the_slider_fires_the_change_and_does_not_reach_the_panel() {
        let (mut nodes, mut slider) = slider(0.0, 10.0);
        let seen = changes(&mut slider);
        let panel = on_a_panel(&slider, &mut nodes, Size::new(400.0, 200.0));

        let mut event = tap_at(120.0, 22.0);
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == slider.handle() {
                slider.on_event(event, RECT);
            }
        });
        assert_eq!(reached, vec![slider.handle()], "only the slider saw it");
        assert_eq!(seen.borrow().len(), 1, "and it reported the new value");
        assert_eq!(seen.borrow()[0], slider.value.get());
    }

    #[test]
    fn the_change_callback_hands_over_the_value_it_moved_to() {
        // The callback carries the value, which is the whole reason the callback
        // moved: a caller driving a speaker from it needs the number and not a
        // "something changed".
        let (_nodes, mut slider) = slider(0.0, 1.0);
        let last = Rc::new(Cell::new(-1.0_f32));
        let seen = Rc::clone(&last);
        slider.on_change = Callback::from_fn(move |value| seen.set(value));

        let mut tap = tap_at(50.0, 20.0);
        slider.on_event(&mut tap, RECT);
        assert_eq!(last.get(), 0.5);
    }

    #[test]
    fn a_slider_with_no_change_callback_still_moves() {
        // A caller that has given it nothing to report to is an ordinary caller.
        let (_nodes, slider) = slider(0.0, 10.0);
        assert!(!slider.on_change.is_set());
        let mut tap = tap_at(50.0, 20.0);
        assert!(slider.on_event(&mut tap, RECT));
        assert_eq!(slider.value.get(), 5.0);
    }

    #[test]
    fn the_palette_is_the_themes_border_primary_and_on_primary() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            let color = |token| theme.get(token).as_color().unwrap();
            assert_eq!(palette.track, color(ThemeToken::Border));
            assert_eq!(palette.fill, color(ThemeToken::Primary));
            assert_eq!(palette.thumb_fill, color(ThemeToken::Primary));
            assert_eq!(palette.thumb_border, color(ThemeToken::OnPrimary));
            assert_eq!(
                palette.ring,
                color(ThemeToken::Text),
                "the focus ring is on the background, not on the track"
            );
        }
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        // A theme switch has to reach the sliders, or the theme would only change
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
    fn a_slider_asks_for_a_rect_tall_enough_to_grab() {
        // The thumb is 28 pixels across and a slider is held by its thumb, so the
        // node has to be at least the platform touch target across.
        let (_nodes, slider) = slider(0.0, 1.0);
        let size = slider.size();
        assert_eq!(size.height, MIN_TOUCH_TARGET);
        assert_eq!(size.width, DEFAULT_LENGTH);
        assert_eq!(MIN_TOUCH_TARGET, 44.0, "the floor a finger can hit");

        let mut nodes = Arena::new();
        let mut big = Slider::new(&mut nodes, 0.0, 1.0);
        big.set_thumb_radius(40.0);
        assert!(
            big.size().height > MIN_TOUCH_TARGET,
            "a larger thumb makes it taller still"
        );
    }

    #[test]
    fn a_slider_smaller_than_its_thumb_holds_the_thumb_still() {
        // Both ends would otherwise be the wrong way round, and the value would
        // turn into a position off the far side of the slider.
        let (_nodes, slider) = slider(0.0, 10.0);
        let narrow = Rect::new(0.0, 0.0, 10.0, 40.0);
        assert_eq!(slider.thumb_center(narrow), (5.0, 20.0));
        slider.value.set(10.0);
        slider.thumb.set(10.0);
        assert_eq!(
            slider.thumb_center(narrow),
            (5.0, 20.0),
            "and it stays there at every value"
        );
    }

    #[test]
    fn a_slider_that_has_never_been_aimed_paints_the_neutral_defaults() {
        // The other half of the themed-slider defect: a caller who never themes
        // a slider still gets something visible, which is the default palette's
        // job.
        let mut nodes = Arena::new();
        let slider = Slider::new(&mut nodes, 0.0, 10.0);
        assert_eq!(slider.track.get(), Palette::default().track);
        assert_eq!(
            rounded(&slider.paint(RECT))
                .first()
                .map(|(_, _, color)| *color),
            Some(Palette::default().track)
        );
    }
}
