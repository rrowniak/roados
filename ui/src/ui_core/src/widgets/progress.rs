//! The Progress widget: a value shown as how much of a track is filled.
//!
//! A progress bar is a node, a truth, a drawn value, and two filled rounded
//! rectangles. [`Progress::value`] is the truth — a fraction of the track, in
//! `0.0..=1.0` — and [`Progress::shown`] is what is *drawn*, animated toward the
//! truth, and the distance between the two is the transition. That split is
//! [`slider`](crate::widgets::slider::Slider)'s, for the same reason it is the
//! button's: a caller writing `value` from a download's progress wants the fill
//! to *arrive* there rather than jump, and requirement 4's "fill width animates
//! when value changes" is the drawn half of the pair.
//!
//! Two modes, one widget. Determinate, the fill's extent is the value times the
//! track's length. Indeterminate, the value is not yet known, and a bar of
//! `INDETERMINATE_BAR_FRACTION` of the track slides from one end to the other
//! and back, forever, on the motion the caller last handed to
//! [`animate_to_state`](Progress::animate_to_state). The loop is the widget's:
//! [`tick`](Progress::tick) aims the next leg when one finishes, so a caller
//! starts it once and never touches it again.
//!
//! **The `Orientation` is the slider's.** Requirement 1 names a type
//! `Orientation`, and the field is `orientation: Orientation` — which is
//! [`slider::Orientation`](crate::widgets::slider::Orientation), imported rather
//! than declared a second time. It is the call
//! [`button::Motion`](crate::widgets::button::Motion) makes, and for its reason:
//! one line of import beats a second public type that has to be kept in step
//! with the first, and these two axes mean exactly the same thing in both
//! widgets. What would move it: a third widget whose orientation is not "a bar
//! along a line" — a grid's is a plane — at which point the enum is no longer
//! about a track and belongs in a module of its own, and this import becomes
//! `use crate::widgets::Orientation;`. The enum is load-bearing, not decoration:
//! it decides the shape of the track, which end the fill grows from, which end
//! the sliding bar starts at, and which way it travels.
//!
//! **`indeterminate` is a plain field behind a setter**, though requirement 1
//! lists it among the properties, and the reason is that it is a *mode* and not
//! an appearance: nothing animates *which* mode a bar is in. A progress bar does
//! not cross-fade from a determinate fill to a sliding one; it is one or the
//! other, and which one is a fact about the operation rather than a step in a
//! transition. A `Property<bool>` here would be a property whose only writes are
//! a caller's, read through `get()`, and — worse than useless — free to change
//! on its own in the middle of a frame, leaving the mode disagreeing with the
//! property that is actually drawn ([`shown`](Progress::shown) in one mode,
//! [`slide`](Progress::slide) in the other). The setter is how a caller writes
//! it, and it puts the two in step in the same breath: switching on parks the
//! sliding bar at the start of a leg, switching off writes the drawn value at
//! once from the truth, so the first frame after a switch is right rather than a
//! bar animating away from a mode that has ended.
//!
//! The colours follow the button's and the slider's rule rather than adding
//! theme tokens: a [`Palette`] names the two colours, the properties hold them so
//! a theme switch can be animated into them, and
//! [`snap_to_state`](Progress::snap_to_state) puts a themed bar on its theme at
//! once. The *sizing* — how thick the track is, how long the sliding bar is, how
//! big the node asks to be — is named constants rather than tokens, for the
//! reason the slider's own constants document: the theme has no token for a
//! progress bar's parts, and adding one would change
//! [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables, the
//! token count and the transition every token takes part in during a switch, for
//! values a switch does not change. Requirement 5's "respects theme tokens for
//! sizing and colour" is therefore split honestly: the *colour* and the *timing*
//! come from the theme's tokens, and the shapes are named and documented here.
//! What would reverse it is a theme that grows a `TrackThickness` token — at
//! which point each constant below becomes the fallback its default is read
//! from, and this paragraph goes.
//!
//! Two things are the caller's, not the bar's. Which node is dirty is a property
//! callback per the demo's own idiom, which is what the second example below
//! shows. And there is **no `on_event`**: requirement 4 lists no gesture for a
//! progress bar, and a bar that cannot be dragged has no meaning to report from
//! a tap. A caller driving one from a gesture writes [`value`](Progress::value),
//! and the same callback every property has carries the write to the node.
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
//! use ui_core::widgets::progress::{Palette, Progress};
//!
//! let mut nodes = Arena::new();
//! let mut progress = Progress::new(&mut nodes);
//! progress.set_palette(Palette::from_theme(&Theme::dark()));
//! progress.snap_to_state();
//!
//! progress.value.set(0.5);
//! progress.animate_to_state(Motion::from_theme(&Theme::dark()));
//! assert_eq!(progress.shown.get(), 0.0, "the fill is where it was, not at 0.5");
//! for _ in 0..30 {
//!     progress.tick(Duration::from_millis(10));
//! }
//! assert_eq!(progress.shown.get(), 0.5, "and it arrives after the fast duration");
//! ```
//!
//! The caller keeps the node clean, through the property's own callback:
//!
//! ```
//! use std::cell::Cell;
//! use std::rc::Rc;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::widgets::progress::Progress;
//!
//! let mut nodes = Arena::new();
//! let progress = Progress::new(&mut nodes);
//! let marks = Rc::new(Cell::new(0));
//! let counting = Rc::clone(&marks);
//! progress.shown.on_change(move |_| counting.set(counting.get() + 1));
//!
//! progress.snap_to_state();
//! assert_eq!(marks.get(), 1, "the snap wrote the drawn value, and the node knows");
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
use crate::widgets::slider::Orientation;

/// The length a progress bar asks for when its caller gives it no size of its
/// own.
///
/// A bar has no content to measure, so this is the one number that decides how
/// long it is by default, and it is the same number
/// [`slider::DEFAULT_LENGTH`](crate::widgets::slider) uses for its own default so
/// a caller laying both out gets one family of widths. It is a constant rather
/// than a theme token for the reason the module documents.
const DEFAULT_LENGTH: f32 = 240.0;

/// How thick the track and the fill are, in pixels.
///
/// The track is a rounded rectangle, so its radius is half of this — which is
/// what makes it a line with rounded ends rather than a bar — and the fill
/// carries the *same* radius, which is what keeps it inside the track at both
/// ends rather than poking out of it at a value of 1.0. Six is the slider's
/// track thickness for the same reason it is [`DEFAULT_LENGTH`]'s length: two
/// bars in one window should be one family. What would reverse it is a
/// `TrackThickness` token in the theme.
const TRACK_THICKNESS: f32 = 6.0;

/// How much of the track the sliding bar covers in indeterminate mode, from
/// `0.0` to `1.0`.
///
/// A quarter: enough that the bar reads as *moving* rather than as a hairline
/// drifting, small enough that its motion is still obviously a progress bar
/// rather than a sweep of something else. It is a fraction and not a pixel count
/// because the bar's length has to follow the track's — a fixed length would be
/// most or all of a narrow track. The bar is never shorter than one thickness
/// however small the fraction is, so it stays a bar rather than a square; see
/// [`Progress::bar_length`].
const INDETERMINATE_BAR_FRACTION: f32 = 0.25;

/// How many of a theme's fast durations one traverse of the track takes.
///
/// [`Motion::from_theme`] reads [`DurationFast`](crate::theme::ThemeToken::DurationFast),
/// which is 150 ms — the length of a *press*, and about right for a state change
/// between two nearby places. A bar crossing its whole track in 150 ms and back
/// in 150 ms is a 3.3 Hz shimmer rather than progress, so a leg takes two of
/// them: 300 ms across, 600 ms for the cycle. It stays a multiple of the token
/// rather than a number of its own, so a theme that lengthens `DurationFast`
/// lengthens the loop with it. What would reverse it: a theme token for the
/// indeterminate period, which is what the platform's own specifications call
/// this and what would let a theme slow the loop without changing
/// `DurationFast`.
const INDETERMINATE_LEG_MULTIPLIER: u32 = 2;

/// Which side of the track's middle the sliding bar is on, and therefore where
/// its next leg goes.
///
/// A leg runs to `1.0` and the next one runs back to `0.0`, and which of the two
/// is next is the only piece of state the loop needs — and it is not state at
/// all, because a bar at the middle of its travel has not arrived anywhere, so
/// deriving the direction from where it stands is both enough and
/// self-correcting. A bar written by hand to exactly the middle goes back, which
/// is as good an answer as going forward.
const SLIDE_MIDPOINT: f32 = 0.5;

/// The smallest a progress bar may be across its short axis, in pixels.
///
/// 44dp is the platform touch target, and it is the same floor
/// [`button::MIN_TOUCH_TARGET`](crate::widgets::button::MIN_TOUCH_TARGET) puts
/// under a button and [`slider::MIN_TOUCH_TARGET`](crate::widgets::slider) puts
/// under a slider. A bar is not grabbed by a finger, so the honest reason is
/// narrower than a button's: a six-pixel node is a six-pixel hit test, so
/// anything lying within a finger's width of the bar stays reachable through it.
/// It is repeated here rather than imported for the reason the slider repeats
/// it — a `pub const` in another widget's module is not a shared place to keep
/// one.
const MIN_TOUCH_TARGET: f32 = 44.0;

/// The colours a progress bar draws with.
///
/// Two: the track and the fill, because a progress bar has two parts and no
/// third thing to colour. They are not tokens of their own — the theme has none
/// per part, and adding one per part would put two more tokens in every theme
/// table and in every theme switch — so a bar is themed with the theme's own
/// and [`Palette::from_theme`] says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The track: the whole range, unfilled.
    pub track: Color,
    /// The fill: the part of the range the value has passed, and the sliding
    /// bar in indeterminate mode.
    pub fill: Color,
}

impl Default for Palette {
    /// Returns a neutral grey bar: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    ///
    /// The two greys are the slider's, so a bar and a slider look like one
    /// family before either is themed.
    fn default() -> Self {
        Palette {
            track: Color::new(64, 64, 64, 255),
            fill: Color::new(160, 160, 160, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The track is [`Border`](crate::theme::ThemeToken::Border), which is the
    /// theme's hairline colour and the only one of its nine that is *muted* by
    /// definition — a bar's track is the part of it that is not the value. The
    /// fill is [`Primary`](crate::theme::ThemeToken::Primary), because a filled
    /// track is the value, in both modes: in indeterminate mode the sliding bar
    /// is the fill and carries the same colour.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::theme::{Theme, ThemeToken};
    /// use ui_core::widgets::progress::Palette;
    ///
    /// let theme = Theme::dark();
    /// let palette = Palette::from_theme(&theme);
    /// let color = |token| theme.get(token).as_color().unwrap();
    /// assert_eq!(palette.track, color(ThemeToken::Border));
    /// assert_eq!(palette.fill, color(ThemeToken::Primary));
    /// ```
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            track: token_color(theme, crate::theme::ThemeToken::Border),
            fill: token_color(theme, crate::theme::ThemeToken::Primary),
        }
    }
}

/// The appearance the progress bar's value and its palette imply.
///
/// Every field is a target, not a value in flight:
/// [`Progress::animate_to_state`] animates the bar's properties toward this and
/// [`Progress::paint`] draws whatever the properties have reached, which is a
/// [`Style`] part way through on a frame where something is moving.
///
/// The sliding bar's position is deliberately **not** here. It is not implied by
/// the value or by the palette — it is a loop, and where its next leg ends
/// depends on where the last one left it, which
/// [`Progress::animate_to_state`] reads rather than this. A [`Style`] that
/// claimed to hold it would be describing a phase as though it were a state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The value the fill is drawn at: [`value`](Progress::value), clamped to
    /// `0.0..=1.0`.
    pub shown: f32,
    /// The colour of the track.
    pub track: Color,
    /// The colour of the fill.
    pub fill: Color,
}

/// A progress bar: a value shown as how much of a track is filled.
///
/// The widget holds the properties the task gives it — [`value`] and [`slide`] —
/// the property that is drawn rather than held ([`shown`]), and the two colour
/// properties a theme switch moves ([`track`], [`fill`]) — and the three plain
/// fields that define the shape: [`orientation`] through
/// [`Progress::set_orientation`], [`indeterminate`] through
/// [`Progress::set_indeterminate`], and the sizing through
/// [`Progress::set_track_thickness`] and
/// [`Progress::set_indeterminate_fraction`].
///
/// The shape is plain fields rather than properties because they are the mapping
/// rather than the appearance: nothing animates a bar's axis, its mode or its
/// thickness, and a property the caller could write directly would let the mode
/// and the drawn value disagree, which is the one thing the setters exist to
/// prevent.
///
/// The node is the caller's to keep clean, and its size is the caller's to give
/// through [`layout_mut`](crate::node::WidgetNode::layout_mut) —
/// [`Progress::size`] is a suggestion for a caller who has nothing else to go
/// on. A bar draws inside whatever rect it is given: a track along its long axis,
/// and a fill drawn from the track's start to wherever the drawn value has got
/// to.
///
/// [`value`]: Progress::value
/// [`shown`]: Progress::shown
/// [`track`]: Progress::track
/// [`fill`]: Progress::fill
/// [`slide`]: Progress::slide
/// [`orientation`]: Progress::orientation
/// [`indeterminate`]: Progress::indeterminate
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::progress::{Palette, Progress};
///
/// let mut nodes = Arena::new();
/// let mut progress = Progress::new(&mut nodes);
/// progress.set_palette(Palette::from_theme(&ui_core::theme::Theme::dark()));
/// progress.snap_to_state();
///
/// // The caller owns the value; the bar draws it and does not otherwise touch it.
/// progress.value.set(1.0);
/// progress.snap_to_state();
/// let rect = Rect::new(0.0, 0.0, 200.0, 24.0);
/// assert_eq!(progress.paint(rect).len(), 2, "a track and a full fill");
/// ```
pub struct Progress {
    /// The truth: how much of the track is filled, from `0.0` to `1.0`.
    ///
    /// The caller may write it — a download's progress, a task's — and the
    /// widget never writes it: nothing a bar receives is a *new value*, only a
    /// new mode or a new colour. A value outside `0.0..=1.0` is not an error
    /// and is not wrapped; see [`Progress::style`] and [`Progress::paint`].
    pub value: Property<f32>,
    /// The value the fill is *drawn* at, animated toward [`value`](Progress::value).
    ///
    /// Requirement 4's "fill width animates when value changes" is this
    /// property: a caller writes [`value`](Progress::value) and
    /// [`animate_to_state`](Progress::animate_to_state) carries this after it.
    /// Writing this one directly overrides the animation until the next aim.
    pub shown: Property<f32>,
    /// Where the sliding bar is along its travel in indeterminate mode, from
    /// `0.0` at the start of the track to `1.0` at its far end.
    ///
    /// It is animated by the loop rather than by a caller's aim, and it is not
    /// drawn at all in determinate mode. Its *drawn* position is this value
    /// clamped to `0.0..=1.0`, so an overshooting curve cannot push the bar off
    /// the end of its track; see `Progress::fill_rect`.
    pub slide: Property<f32>,
    /// The colour of the track: the whole range, unfilled.
    pub track: Property<Color>,
    /// The colour of the fill: the part of the range the value has passed, and
    /// the sliding bar in indeterminate mode.
    pub fill: Property<Color>,
    orientation: Orientation,
    indeterminate: bool,
    track_thickness: f32,
    bar_fraction: f32,
    palette: Palette,
    /// The motion the indeterminate loop runs on, as handed to the last
    /// [`animate_to_state`](Progress::animate_to_state).
    ///
    /// The widget has to remember it because the loop is the widget's: a leg
    /// that finishes in [`tick`](Progress::tick) is re-aimed from here, with no
    /// caller in between. `None` means no loop has been started, so a bar put
    /// into indeterminate mode and never animated does not move — which is the
    /// same rule the rest of the library follows, that nothing moves until a
    /// caller aims it.
    loop_motion: RefCell<Option<Motion>>,
    clock: RefCell<AnimationClock>,
    node: Handle,
}

impl Progress {
    /// Creates a progress bar in the arena, and returns it.
    ///
    /// It starts at zero, determinate, horizontal, and paints the neutral
    /// defaults of [`Palette::default`] until a caller gives it a
    /// [`Palette`](Progress::set_palette) and calls
    /// [`snap_to_state`](Progress::snap_to_state) or
    /// [`animate_to_state`](Progress::animate_to_state).
    ///
    /// The task file's `Progress::new() -> Handle` is read as this: the handle is
    /// [`Progress::handle`]'s, and returning it alone would leave a caller with
    /// no property to write and no way to draw the bar. Task 12's
    /// [`Button::new`](crate::widgets::button::Button::new) and task 16's
    /// [`Slider::new`](crate::widgets::slider::Slider::new) settled the same
    /// reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let progress = Progress::new(&mut nodes);
    /// assert!(nodes.get(progress.handle()).is_some(), "its node is in the arena");
    /// assert_eq!(progress.value.get(), 0.0, "and it starts empty");
    /// ```
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>) -> Self {
        let palette = Palette::default();
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Progress {
            value: Property::new(0.0),
            shown: Property::new(0.0),
            slide: Property::new(0.0),
            track: Property::new(palette.track),
            fill: Property::new(palette.fill),
            orientation: Orientation::default(),
            indeterminate: false,
            track_thickness: TRACK_THICKNESS,
            bar_fraction: INDETERMINATE_BAR_FRACTION,
            palette,
            loop_motion: RefCell::new(None),
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the progress bar's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the progress bar draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the progress bar draws with, and leaves the current ones
    /// where they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](Progress::animate_to_state) or
    /// [`snap_to_state`](Progress::snap_to_state): a theme switch is animated,
    /// and a theme switch is the caller announcing a new palette and then moving
    /// the bar toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the bar chasing a palette that is still
    /// moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the axis the progress bar's track runs along.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::slider::Orientation;
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// assert_eq!(progress.orientation(), Orientation::Horizontal);
    /// progress.set_orientation(Orientation::Vertical);
    /// assert_eq!(progress.orientation(), Orientation::Vertical);
    /// ```
    #[must_use]
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Sets the axis the progress bar's track runs along.
    ///
    /// The value does not move, and the sliding bar does not restart. A new
    /// orientation is a new shape for the same value rather than a change of
    /// value, and the geometry is derived at paint time, so it applies from the
    /// next frame. A vertical bar's fill grows from the *bottom* of its rect,
    /// because the screen's y axis points down and up should mean progress, and
    /// its sliding bar travels from the bottom to the top.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
    }

    /// Returns whether the bar is showing an unknown progress.
    ///
    /// A determinate bar draws [`value`](Progress::value) as the extent of its
    /// fill; an indeterminate one ignores it and slides a bar across the track
    /// instead.
    #[must_use]
    pub fn indeterminate(&self) -> bool {
        self.indeterminate
    }

    /// Sets which of the two modes the bar is in, and puts the drawn value where
    /// the new mode expects to find it.
    ///
    /// Switching **on** parks the sliding bar at `0.0` — the start of a leg, and
    /// the beginning of the first traverse — and clears every transition
    /// running, so the bar starts sliding from a known place rather than from
    /// wherever an abandoned determinate transition had left
    /// [`shown`](Progress::shown). It does
    /// not itself start the loop: that is
    /// [`animate_to_state`](Progress::animate_to_state), and a bar in
    /// indeterminate mode that has never been aimed draws its track alone.
    ///
    /// Switching **off** writes [`shown`](Progress::shown) at once from
    /// [`value`](Progress::value) and clears every transition running, so the
    /// first frame drawn in determinate mode is the truth rather than a fill
    /// animating in from wherever the sliding bar happened to be. It does not
    /// animate: a mode change is a fact, not a step, and there is nothing to
    /// animate between "a bar sliding because the value is unknown" and "a bar
    /// showing the value".
    ///
    /// Either way the mode is a plain field and this setter is the only way to
    /// write it, which is what keeps the mode and the drawn value from
    /// disagreeing. The module documents why a `Property<bool>` here would be
    /// worse than nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// progress.value.set(0.5);
    /// progress.set_indeterminate(true);
    /// assert!(progress.indeterminate());
    /// assert_eq!(progress.slide.get(), 0.0, "parked at the start of a leg");
    ///
    /// progress.set_indeterminate(false);
    /// assert_eq!(progress.shown.get(), 0.5, "and back to the truth at once");
    /// ```
    pub fn set_indeterminate(&mut self, indeterminate: bool) {
        self.indeterminate = indeterminate;
        self.clock.borrow_mut().clear();
        if indeterminate {
            self.slide.set(0.0);
        } else {
            self.shown.set(self.clamped_value());
        }
    }

    /// Sets how thick the track and the fill are, in pixels.
    ///
    /// A negative thickness is no thickness: the bar's across extent is the
    /// thickness and a negative one would draw outside its own track, which is
    /// the one thing this widget promises never to do.
    pub fn set_track_thickness(&mut self, thickness: f32) {
        self.track_thickness = thickness.max(0.0);
    }

    /// Sets how much of the track the sliding bar covers, from `0.0` to `1.0`.
    ///
    /// A fraction outside that range is clamped rather than rejected, for the
    /// reason [`Progress::set_track_thickness`] gives for a negative thickness:
    /// a caller error must not put a bar outside its own track. A fraction of
    /// zero is not a bar of no length — `Progress::bar_length` holds it to one
    /// thickness — and one of `1.0` is a bar that fills the whole track and
    /// therefore does not move.
    pub fn set_indeterminate_fraction(&mut self, fraction: f32) {
        self.bar_fraction = bounded(fraction, 0.0, 1.0);
    }

    /// Returns the size a progress bar asks for: `DEFAULT_LENGTH` along its
    /// long axis, and enough across the short one to be found by a finger.
    ///
    /// A bar has no content to measure, so this is only for a caller that has
    /// nothing else to go on; a caller that lays the bar out itself gives the
    /// node whatever rect it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](Progress::paint) draws inside whatever it is given.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::slider::Orientation;
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// let flat = progress.size();
    /// assert!(flat.width > flat.height, "a bar is long and thin: {flat:?}");
    ///
    /// progress.set_orientation(Orientation::Vertical);
    /// let upright = progress.size();
    /// assert!(upright.height > upright.width, "and long down: {upright:?}");
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        let across = self.track_thickness.max(MIN_TOUCH_TARGET);
        match self.orientation {
            Orientation::Horizontal => Size::new(DEFAULT_LENGTH, across),
            Orientation::Vertical => Size::new(across, DEFAULT_LENGTH),
        }
    }

    /// Returns the appearance the progress bar's value and its palette imply.
    ///
    /// The value is clamped here rather than at paint time alone, so a caller
    /// reading the target — which is what a test and a caller drawing a scale of
    /// its own both do — is told about the end of the track and not about a
    /// position past it. A value of `NaN` reads as `0.0`: a caller that has lost
    /// track of its arithmetic gets an empty bar rather than a rect whose every
    /// coordinate is `NaN`.
    ///
    /// The sliding bar's position is not in here, for the reason [`Style`]
    /// documents.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// progress.value.set(0.5);
    /// assert_eq!(progress.style().shown, 0.5);
    ///
    /// progress.value.set(2.0);
    /// assert_eq!(progress.style().shown, 1.0, "past the top is the top");
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            shown: self.clamped_value(),
            track: self.palette.track,
            fill: self.palette.fill,
        }
    }

    /// Applies the appearance the value and the palette imply at once, with no
    /// transition, and ends every transition already running.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a bar that has just been given a [`Palette`](Progress::set_palette)
    /// and has never animated — whose colour properties still hold the neutral
    /// defaults [`Progress::new`] wrote, so without this a themed bar starts out
    /// grey — and a caller that has written [`value`](Progress::value) itself and
    /// wants the bar to be that state now.
    ///
    /// The sliding bar is left exactly where it stands. A snap is about the
    /// properties that have a target, and the sliding bar's next leg ends
    /// somewhere neither the value nor the palette says anything about; its
    /// position is whatever the abandoned transition had got to, and the loop's
    /// next [`tick`](Progress::tick) — or the caller's next
    /// [`animate_to_state`](Progress::animate_to_state) — takes it from there.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::progress::{Palette, Progress};
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// let themed = Palette::from_theme(&Theme::dark());
    /// progress.set_palette(themed);
    /// progress.snap_to_state();
    /// assert_eq!(progress.track.get(), themed.track);
    /// ```
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.shown.set(style.shown);
        self.track.set(style.track);
        self.fill.set(style.fill);
    }

    /// Starts the transitions that carry the progress bar from wherever it is
    /// toward the appearance [`style`](Progress::style) implies, on `motion`, and
    /// in indeterminate mode starts — or restarts — the sliding loop on the same
    /// motion.
    ///
    /// This is the call a caller makes after writing [`value`](Progress::value),
    /// and it is what requirement 4's "fill width animates when value changes" is
    /// made of. It is also the call that starts the loop, once: after it,
    /// [`tick`](Progress::tick) aims each leg in turn, so a caller never has to
    /// come back.
    ///
    /// The bar's own clock is cleared first, so the transitions this replaces
    /// stop where they are rather than writing over the new ones when they
    /// arrive — the reason the button owns a clock, for the same reason. The
    /// slide's leg is `INDETERMINATE_LEG_MULTIPLIER` of `motion.duration`
    /// rather than `motion.duration` itself, for the reason that constant gives;
    /// the colours and the drawn value are on `motion` as they are everywhere
    /// else.
    ///
    /// In determinate mode [`slide`](Progress::slide) is not touched at all, and
    /// in indeterminate mode [`shown`](Progress::shown) is not: no bar is
    /// sliding in the one mode, and no fill is drawn in the other, and a
    /// transition on a property nothing draws is a transition that reports itself
    /// as running for nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// let motion = Motion { duration: Duration::from_millis(100), easing: Easing::Linear };
    ///
    /// progress.value.set(0.5);
    /// progress.animate_to_state(motion);
    /// assert_eq!(progress.shown.get(), 0.0, "the fill starts where it was");
    /// progress.tick(Duration::from_millis(50));
    /// assert_eq!(progress.shown.get(), 0.25, "and is half way at half the time");
    ///
    /// // Indeterminate: one leg is two of the motion's durations.
    /// progress.set_indeterminate(true);
    /// progress.animate_to_state(motion);
    /// progress.tick(Duration::from_millis(100));
    /// assert_eq!(progress.slide.get(), 0.5);
    /// ```
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        *self.loop_motion.borrow_mut() = Some(motion);
        if self.indeterminate {
            // Nothing draws `shown` in this mode, and a transition on a property
            // nothing draws is a transition that reports itself as running for
            // nothing — the same rule [`slide`](Progress::slide) is left alone
            // by. The drawn value is written at once when the mode is switched
            // off, so it is never stale by the time it is drawn.
            self.aim_slide(&mut clock, motion);
        } else {
            clock.add(
                self.shown
                    .animate_to(style.shown, motion.duration, motion.easing),
            );
        }
        clock.add(
            self.track
                .animate_to(style.track, motion.duration, motion.easing),
        );
        clock.add(
            self.fill
                .animate_to(style.fill, motion.duration, motion.easing),
        );
    }

    /// Advances the progress bar's transitions by `delta`, and returns whether
    /// any of them wrote.
    ///
    /// It is the bar's frame integration: call it once a frame, before the paint
    /// pass, with the time that frame took. The write is what reaches the node —
    /// a property callback registered by the caller marks the node dirty — so a
    /// caller that repaints only when this is true repaints exactly while
    /// something moves.
    ///
    /// In indeterminate mode it also **keeps the loop going**: the tick on which
    /// a leg arrives aims the next one, so a caller that called
    /// [`animate_to_state`](Progress::animate_to_state) once is running a loop
    /// forever. It returns `true` on that tick as well, because the bar is
    /// certainly about to move and the caller would otherwise skip the frame
    /// before it does.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// progress.value.set(1.0);
    /// progress.animate_to_state(Motion {
    ///     duration: Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    ///
    /// assert!(progress.tick(Duration::from_millis(50)), "the fill is moving");
    /// assert!(progress.is_animating());
    /// assert!(progress.tick(Duration::from_millis(50)), "and still is");
    /// assert_eq!(progress.shown.get(), 1.0, "arrived");
    /// assert!(!progress.is_animating(), "so nothing is left running");
    /// ```
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        let mut clock = self.clock.borrow_mut();
        let wrote = clock.tick(delta);
        if !self.indeterminate || clock.is_animating() {
            return wrote;
        }
        // A leg has arrived and nothing else is running. Aim the next one, which
        // is what makes the loop the widget's rather than the caller's: the
        // caller started it once and the bar has been moving ever since.
        let Some(motion) = *self.loop_motion.borrow() else {
            return wrote;
        };
        self.aim_slide(&mut clock, motion);
        true
    }

    /// Returns whether any of the progress bar's transitions is still running.
    ///
    /// In indeterminate mode it is true for as long as the loop is running, which
    /// is for as long as the caller keeps the bar aimed — a bar in indeterminate
    /// mode is never at rest.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Returns the draw commands that paint the progress bar within `rect`.
    ///
    /// The commands are two, in order: the track, a rounded rectangle in the
    /// track's colour, and the fill on top of it — the same fill colour, the same
    /// radius, the same two ends, at whichever of the two modes is in force.
    ///
    /// **The fill never reaches outside the track**, at any value and at any
    /// position of the sliding bar, and that is a property of the geometry rather
    /// than a hope. Three things carry it. The fill's rect is built *from* the
    /// track's, so it starts at the track's own origin and is measured along the
    /// track's own extent — the two numbers that the slider's own
    /// `travel` confused, an origin and an extent, are the two this file never
    /// has to subtract. The fill carries the **track's own radius**, half the
    /// thickness, at both ends: a fill with square ends at a value of 1.0 would
    /// have its four corners standing outside the track's four rounded corners,
    /// and a bar whose fill pokes out of its track at every pixel of "full" is not
    /// a filled track. And both the value and the sliding position are clamped
    /// before the rect is built, so an overshooting curve — a
    /// [`Spring`](crate::animation::Easing::Spring), which passes its target on
    /// the way and comes back — moves the bar past the end of its travel and
    /// still draws it inside the track.
    ///
    /// There is nothing here that could read as a filled shape instead of the
    /// outline it was meant to be, and that is worth saying: a
    /// [`DrawCommand::RoundedRect`] *fills* its rect, so a bar that drew its
    /// track as a grown shape with the fill drawn over its middle would draw a
    /// card. This widget has no border, no ring and no background of its own, so
    /// it draws exactly two bars and no outlines at all — the one case where a
    /// filled rounded rectangle is precisely what was meant.
    ///
    /// A fill with no extent is not recorded at all: a bar at a value of `0.0`
    /// draws its track and nothing else, rather than a zero-width rounded
    /// rectangle that is a quad of nothing.
    ///
    /// Every size here comes from the *drawn* value, so a bar mid-transition
    /// paints its fill as wide as the transition has got and not as wide as the
    /// value it is travelling toward.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{DrawCommand, Rect};
    /// use ui_core::widgets::progress::Progress;
    ///
    /// let mut nodes = Arena::new();
    /// let mut progress = Progress::new(&mut nodes);
    /// let rect = Rect::new(32.0, 64.0, 200.0, 24.0);
    ///
    /// // Nothing to fill: the track alone.
    /// assert_eq!(progress.paint(rect).len(), 1);
    ///
    /// progress.value.set(0.5);
    /// progress.snap_to_state();
    /// let commands = progress.paint(rect);
    /// let fill = commands
    ///     .iter()
    ///     .filter_map(|command| match command {
    ///         DrawCommand::RoundedRect { rect, color, .. } if *color == progress.fill.get() => {
    ///                             Some(*rect)
    ///                         }
    ///                         _ => None,
    ///                     })
    ///     .next()
    ///     .unwrap();
    /// assert_eq!(fill.width, 100.0, "half of a 200-pixel track, at the track's x");
    /// assert_eq!(fill.x, 32.0);
    /// ```
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let track = self.track_rect(rect);
        let radius = self.track_thickness / 2.0;
        painter.rounded_rect(track, radius, self.track.get());
        if let Some(fill) = self.fill_rect(rect) {
            painter.rounded_rect(fill, radius, self.fill.get());
        }
        painter.finish()
    }

    /// Returns the value clamped to `0.0..=1.0`: what
    /// [`style`](Progress::style) reports and what a determinate fill is drawn
    /// at.
    ///
    /// Clamping is the target of [`Progress::value`], not the target of
    /// [`shown`](Progress::shown): a caller reading `style` is asking where the
    /// fill is heading and is told about the end of the track rather than about a
    /// position past it, while [`drawn_fraction`](Progress::drawn_fraction)
    /// clamps the property as well, for the caller who wrote it by hand.
    fn clamped_value(&self) -> f32 {
        bounded(self.value.get(), 0.0, 1.0)
    }

    /// Returns the fraction of the track the fill is *drawn* at, from the drawn
    /// property rather than from the truth.
    ///
    /// It is what a frame mid-transition paints, which is the only version of
    /// the question a test about animation can ask. The clamp is here as well as
    /// in [`clamped_value`](Progress::clamped_value) because [`shown`] is a public
    /// property and a caller may write it directly: a drawn value of 1.4 is a
    /// full bar, not a bar 40% wider than its track.
    ///
    /// [`shown`]: Progress::shown
    fn drawn_fraction(&self) -> f32 {
        bounded(self.shown.get(), 0.0, 1.0)
    }

    /// Returns the track's rounded rectangle inside `rect`.
    ///
    /// The track is the whole of the node's extent along its own axis and is
    /// centred across the other one, at the bar's thickness. It is not inset
    /// along its axis the way the slider's track is: a slider insets because its
    /// thumb needs a radius of clearance at each end, and a progress bar has no
    /// thumb, so a value of 1.0 is the whole of the track rather than all but a
    /// radius.
    ///
    /// The extent is `.max(0.0)`: a rect narrower than the bar's thickness is a
    /// caller error, and a track of no length is a bar that draws nothing rather
    /// than one that draws inside out.
    fn track_rect(&self, rect: Rect) -> Rect {
        let thickness = self.track_thickness;
        match self.orientation {
            Orientation::Horizontal => Rect::new(
                rect.x,
                rect.y + (rect.height - thickness) / 2.0,
                rect.width.max(0.0),
                thickness,
            ),
            Orientation::Vertical => Rect::new(
                rect.x + (rect.width - thickness) / 2.0,
                rect.y,
                thickness,
                rect.height.max(0.0),
            ),
        }
    }

    /// Returns the fill's rounded rectangle inside `rect`, or `None` if it has no
    /// extent to draw.
    ///
    /// This is the one function where the orientation is load-bearing three times
    /// over: which axis the extent is measured along, which end the fill grows
    /// from, and — in indeterminate mode — which way the sliding bar travels. A
    /// vertical bar fills from the bottom up and its sliding bar rises, because
    /// the screen's y axis points down and progress should go up.
    ///
    /// Both branches build their rect from the track's own origin and the track's
    /// own extent, and both clamp before they multiply, so the result is inside
    /// the track by construction: `x + width` of the horizontal fill is at most
    /// `x + width` of the track, and `y + length - bar - along` of the vertical
    /// sliding bar is between the track's `y` and its bottom.
    fn fill_rect(&self, rect: Rect) -> Option<Rect> {
        let track = self.track_rect(rect);
        let length = self.track_length(track).max(0.0);
        let thickness = self.track_thickness;
        let fill = if self.indeterminate {
            let bar = self.bar_length(track);
            let travel = (length - bar).max(0.0);
            let along = bounded(self.slide.get(), 0.0, 1.0) * travel;
            match self.orientation {
                Orientation::Horizontal => Rect::new(track.x + along, track.y, bar, thickness),
                Orientation::Vertical => {
                    Rect::new(track.x, track.y + length - bar - along, thickness, bar)
                }
            }
        } else {
            let fraction = self.drawn_fraction();
            match self.orientation {
                Orientation::Horizontal => {
                    Rect::new(track.x, track.y, length * fraction, thickness)
                }
                Orientation::Vertical => Rect::new(
                    track.x,
                    track.y + length * (1.0 - fraction),
                    thickness,
                    length * fraction,
                ),
            }
        };
        (fill.width > 0.0 && fill.height > 0.0).then_some(fill)
    }

    /// Returns how far along the track the bar runs, in pixels.
    fn track_length(&self, track: Rect) -> f32 {
        match self.orientation {
            Orientation::Horizontal => track.width,
            Orientation::Vertical => track.height,
        }
    }

    /// Returns how long the sliding bar is, in pixels: the track's length times
    /// [`Progress::set_indeterminate_fraction`], between one thickness and the
    /// whole of it.
    ///
    /// The two ends of that range are both necessary. A sliding bar shorter than
    /// the thickness stops being a bar: its own radius would be clamped to half
    /// its own length, it would draw as a square, and on a narrow track a square
    /// is a dot that appears not to move at all. A bar longer than the track
    /// leaves a negative travel and a rectangle whose far end is before its near
    /// one — so the lower bound and the upper bound are each the other one's
    /// error.
    fn bar_length(&self, track: Rect) -> f32 {
        let length = self.track_length(track).max(0.0);
        (length * self.bar_fraction)
            .max(self.track_thickness)
            .min(length)
    }

    /// Returns where the sliding bar's next leg ends: `1.0` for the far end of
    /// the track, `0.0` for the near one.
    ///
    /// Which way a leg goes is decided by which half of the travel the bar is
    /// standing in, so the loop needs no state of its own and cannot get a phase
    /// wrong: a bar halfway is not at either end, and goes back.
    fn leg_target(&self) -> f32 {
        if self.slide.get() < SLIDE_MIDPOINT {
            1.0
        } else {
            0.0
        }
    }

    /// Aims the sliding bar's next leg at [`leg_target`](Progress::leg_target),
    /// over [`INDETERMINATE_LEG_MULTIPLIER`] of `motion`'s duration.
    fn aim_slide(&self, clock: &mut AnimationClock, motion: Motion) {
        let leg = motion.duration * INDETERMINATE_LEG_MULTIPLIER;
        clock.add(self.slide.animate_to(self.leg_target(), leg, motion.easing));
    }
}

/// Returns `value` between `low` and `high`, `NaN` included.
///
/// The two comparisons rather than [`f32::clamp`], for the reason the slider's
/// own `bounded_to` gives: `clamp` panics when its bounds are the wrong way
/// round, and a widget must not take a frame down to say that a caller passed
/// the wrong pair. `max` then `min` is the same answer for an ordered pair, it
/// cannot panic, and a `NaN` is passed over rather than propagated —
/// `f32::max` returns the *other* operand when one of the two is `NaN`, so a
/// `NaN` value becomes `low` and a `NaN` bound is ignored. Two `NaN` bounds
/// cannot be made sense of and leave the answer `NaN`, which draws nothing.
#[allow(clippy::manual_clamp)]
fn bounded(value: f32, low: f32, high: f32) -> f32 {
    value.max(low).min(high)
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback
/// for a token a caller has written the wrong variant into — and black rather
/// than a panic, because a mistyped theme token is not worth taking a frame down
/// for. It is the button's and the slider's own helper, repeated rather than
/// imported: it is four lines, and a shared module for one four-line helper is a
/// module.
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

    /// The rect every horizontal geometry test lays a bar out in: 200 by 24 at
    /// (32, 64), which is somewhere in the middle of a window rather than at its
    /// corner.
    ///
    /// It is deliberately **not** at the origin, and every other fixture in this
    /// module is off it too. A rect's origin and a rect's extent are different
    /// numbers, and a suite whose every fixture is at `(0, 0)` cannot see the one
    /// being used as the other: the slider's tests passed all fifty-three of them
    /// while every slider drawn anywhere else in a window reported its own origin
    /// as a negative travel.
    const RECT: Rect = Rect {
        x: 32.0,
        y: 64.0,
        width: 200.0,
        height: 24.0,
    };

    /// A vertical bar: 24 by 200 at (700, 40), as far from the origin and from
    /// [`RECT`] as a fixture can be while still being legible in a failure.
    const TALL: Rect = Rect {
        x: 700.0,
        y: 40.0,
        width: 24.0,
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

    /// Advances `bar`'s transitions by `millis` and returns whether anything
    /// moved.
    ///
    /// The bar's own `tick` is what a frame calls and it answers whether a
    /// repaint is needed, so a test that only wants time to pass goes through
    /// here rather than discarding a `#[must_use]` result: that answer is passed
    /// on, not dropped on the floor. It is the same helper the slider's tests
    /// have, for the same reason.
    fn tick(bar: &Progress, millis: u64) -> bool {
        bar.tick(ms(millis))
    }

    /// A bar in the dark theme's palette, with its colours already arrived at
    /// that palette.
    ///
    /// Setting a palette does not move the bar by itself — a theme switch is
    /// animated — so a test that wants a bar *in* a palette asks for the snap.
    fn progress() -> (Arena<WidgetNode>, Progress) {
        let mut nodes = Arena::new();
        let mut bar = Progress::new(&mut nodes);
        bar.set_palette(Palette::from_theme(&Theme::dark()));
        bar.snap_to_state();
        (nodes, bar)
    }

    /// A bar at `value`, snapped so the drawn value is where the truth is.
    fn at(value: f32) -> (Arena<WidgetNode>, Progress) {
        let (nodes, bar) = progress();
        bar.value.set(value);
        bar.snap_to_state();
        (nodes, bar)
    }

    /// An indeterminate bar aimed on [`motion`], with its first leg under way.
    fn sliding() -> (Arena<WidgetNode>, Progress) {
        let (nodes, mut bar) = progress();
        bar.set_indeterminate(true);
        bar.animate_to_state(motion());
        (nodes, bar)
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

    /// A short name for each recorded command, in the order they were recorded.
    fn shapes(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::RoundedRect { .. } => "rect",
                _ => "other",
            })
            .collect()
    }

    /// Returns whether `inner` lies inside `outer`, to within a millionth on each
    /// edge.
    ///
    /// This is the promise [`Progress::paint`] makes — the fill never reaches
    /// outside the track — written as the question a paint can answer. Nothing
    /// about colours or order is in it: a rectangle of the wrong colour in the
    /// right place inside the track is what this is *for*, and the pair of
    /// commands is asserted separately.
    fn inside(outer: Rect, inner: Rect) -> bool {
        const TOLERANCE: f32 = 1e-5;
        inner.x >= outer.x - TOLERANCE
            && inner.y >= outer.y - TOLERANCE
            && inner.x + inner.width <= outer.x + outer.width + TOLERANCE
            && inner.y + inner.height <= outer.y + outer.height + TOLERANCE
    }

    /// Asserts two values are within a millionth of one another.
    ///
    /// The products are computed in `f32`, so `200.0 * 0.3` is not exactly 60.0:
    /// a test that wrote the decimal out would be asserting the compiler's
    /// rounding rather than the widget. Everything the widget computes exactly —
    /// a fraction of a tenth or a half of a length — is asserted with
    /// `assert_eq!` instead.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-5, "{what}: {got} against {want}");
    }

    /// Returns the track and the fill a paint recorded, or an empty pair if the
    /// paint recorded fewer than two rounded rectangles.
    fn bars(bar: &Progress, rect: Rect) -> ((Rect, f32), Option<(Rect, f32)>) {
        let drawn = rounded(&bar.paint(rect));
        let track = drawn.first().map_or_else(
            || (Rect::new(0.0, 0.0, 0.0, 0.0), 0.0),
            |(rect, radius, _)| (*rect, *radius),
        );
        let fill = drawn.get(1).map(|(rect, radius, _)| (*rect, *radius));
        (track, fill)
    }

    /// Returns the width of the fill a paint recorded, or zero when it recorded
    /// none — which is what a bar at a value of `0.0` records, and what the
    /// tests about a fill growing from nothing start from.
    fn fill_width(bar: &Progress, rect: Rect) -> f32 {
        bars(bar, rect).1.map_or(0.0, |(rect, _)| rect.width)
    }

    #[test]
    fn a_progress_bar_holds_the_properties_the_task_gives_it() {
        let (nodes, bar) = progress();
        assert!(
            nodes.get(bar.handle()).is_some(),
            "with its node in the arena"
        );
        assert_eq!(bar.value.get(), 0.0, "the value starts at nothing done");
        assert_eq!(bar.shown.get(), 0.0, "and so does what is drawn");
        assert_eq!(bar.slide.get(), 0.0, "with the sliding bar at its start");
        assert_eq!(bar.orientation(), Orientation::Horizontal);
        assert!(!bar.indeterminate(), "and determinate until told otherwise");
    }

    #[test]
    fn the_default_geometry_is_the_values_this_module_documents() {
        // These are constants, not derived numbers, and the task's "rounded
        // rectangle", the fill's containment and the loop's pace all hang off
        // them.
        let (_nodes, bar) = progress();
        assert_eq!(bar.track_thickness, TRACK_THICKNESS);
        assert_eq!(bar.bar_fraction, INDETERMINATE_BAR_FRACTION);
        assert_eq!(TRACK_THICKNESS, 6.0);
        assert_eq!(
            INDETERMINATE_BAR_FRACTION, 0.25,
            "and the sliding bar is a quarter of the track"
        );
        assert_eq!(INDETERMINATE_LEG_MULTIPLIER, 2, "a leg is two durations");
        assert_eq!(SLIDE_MIDPOINT, 0.5);
    }

    #[test]
    fn a_progress_bar_asks_for_a_long_line_across_a_touch_target() {
        let (_nodes, bar) = progress();
        let size = bar.size();
        assert_eq!(size.width, DEFAULT_LENGTH);
        assert_eq!(size.height, MIN_TOUCH_TARGET, "the floor a finger can find");
        assert_eq!(MIN_TOUCH_TARGET, 44.0);

        let mut bar = bar;
        bar.set_track_thickness(60.0);
        assert_eq!(
            bar.size().height,
            60.0,
            "and a thicker bar is taller across"
        );
    }

    #[test]
    fn a_vertical_progress_bar_asks_for_its_length_downwards() {
        let (_nodes, mut bar) = progress();
        bar.set_orientation(Orientation::Vertical);
        let size = bar.size();
        assert!(size.height > size.width, "a bar is long down: {size:?}");
        assert_eq!(size.height, DEFAULT_LENGTH);
        assert_eq!(size.width, MIN_TOUCH_TARGET);
    }

    #[test]
    fn the_palette_is_the_themes_border_and_primary() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            let color = |token| theme.get(token).as_color().unwrap();
            assert_eq!(
                palette.track,
                color(ThemeToken::Border),
                "the track is the part that is not the value"
            );
            assert_eq!(palette.fill, color(ThemeToken::Primary));
        }
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        // A theme switch has to reach the bars, or the theme would only change
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
    fn a_progress_bar_that_has_never_been_themed_paints_the_neutral_defaults() {
        // A caller who never themes a bar still gets something visible, which is
        // the default palette's job.
        let mut nodes = Arena::new();
        let bar = Progress::new(&mut nodes);
        bar.value.set(1.0);
        bar.snap_to_state();
        assert_eq!(bar.track.get(), Palette::default().track);
        assert_eq!(
            rounded(&bar.paint(RECT))
                .first()
                .map(|(_, _, color)| *color),
            Some(Palette::default().track)
        );
    }

    #[test]
    fn setting_the_palette_leaves_the_bar_where_it_is() {
        // A theme switch is animated by the caller setting the palette and
        // animating toward it; a palette that moved the bar on its own would
        // make every theme switch instantaneous.
        let (_nodes, mut bar) = progress();
        let before = bar.fill.get();
        bar.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(bar.fill.get(), before);
    }

    #[test]
    fn snapping_puts_a_themed_bar_where_its_theme_says_at_once() {
        // The failure this guards: a bar given a palette but never aimed still
        // paints the neutral defaults `Progress::new` wrote, so a themed bar
        // starts out grey and only becomes the theme's colour once something has
        // moved it. Built from `new` rather than through the `progress` helper,
        // which snaps.
        let mut nodes = Arena::new();
        let mut bar = Progress::new(&mut nodes);
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(themed.track, bar.track.get(), "so this can fail");

        bar.set_palette(themed);
        bar.snap_to_state();

        assert_eq!(bar.track.get(), themed.track);
        assert_eq!(bar.fill.get(), themed.fill);
        assert!(!bar.is_animating(), "a snap is not a transition");
    }

    #[test]
    fn snapping_a_mid_transition_bar_back_out_ends_the_transition() {
        // Without the `clear`, a transition already running writes over what the
        // snap just set when it arrives, and the bar drifts off again.
        let (_nodes, bar) = progress();
        bar.value.set(1.0);
        bar.animate_to_state(motion());
        assert!(bar.tick(ms(10)), "the fill is animating");

        bar.value.set(0.0);
        bar.snap_to_state();
        assert_eq!(bar.shown.get(), 0.0, "the snap put it at the new value");

        assert!(!bar.tick(ms(500)), "and nothing arrives afterwards");
        assert_eq!(bar.shown.get(), 0.0);
    }

    #[test]
    fn a_value_written_without_an_aim_does_not_move_the_drawn_value() {
        // The truth and the drawn value are two properties on purpose, and this
        // is the half of the contract where they disagree: writing `value` is
        // not an instruction to draw it *now*.
        let (_nodes, bar) = progress();
        bar.value.set(0.5);
        assert_eq!(bar.style().shown, 0.5, "the target is the new value");
        assert_eq!(bar.shown.get(), 0.0, "but the fill has not moved");
    }

    #[test]
    fn a_value_outside_the_unit_range_is_clamped_rather_than_wrapped() {
        // `bounded` is two comparisons, so a caller that reports 120% gets a full
        // bar and one that reports -5% gets an empty one. Wrapping would have
        // put both in the middle of the track.
        let (_nodes, bar) = progress();
        for (written, drawn) in [(-1.0, 0.0), (0.0, 0.0), (1.0, 1.0), (7.0, 1.0)] {
            bar.value.set(written);
            assert_eq!(bar.style().shown, drawn, "{written} is clamped to {drawn}");
        }
    }

    #[test]
    fn a_value_of_nan_is_an_empty_bar_rather_than_a_broken_one() {
        // A caller that has lost track of its arithmetic hands over `NaN`, and
        // `f32::max` returns the other operand, so the clamp turns it into the
        // low end rather than propagating it into every coordinate of a rect.
        let (_nodes, bar) = progress();
        bar.value.set(f32::NAN);
        assert_eq!(bar.style().shown, 0.0);
        assert_eq!(
            bar.paint(RECT).len(),
            1,
            "a track and nothing drawn over it"
        );
    }

    #[test]
    fn a_drawn_value_written_past_the_track_ends_at_the_track_end() {
        // `shown` is public, so the clamp in `drawn_fraction` is the only thing
        // between a caller's own arithmetic and a fill wider than its track.
        let (_nodes, bar) = progress();
        bar.shown.set(1.4);
        let (track, fill) = bars(&bar, RECT);
        assert_close(
            fill.unwrap().0.width,
            track.0.width,
            "no wider than the track",
        );
        assert!(inside(track.0, fill.unwrap().0));

        bar.shown.set(-0.4);
        assert_eq!(bar.paint(RECT).len(), 1, "and no fill at all below zero");
    }

    #[test]
    fn a_progress_bar_paints_its_track_at_the_whole_of_its_long_axis() {
        let (_nodes, bar) = at(0.0);
        let (track, _) = bars(&bar, RECT);
        assert_eq!(
            track.0,
            Rect::new(32.0, 73.0, 200.0, 6.0),
            "200 long, six thick, centred across a 24-tall rect"
        );
        assert_eq!(track.1, 3.0, "with half the thickness as its radius");
        assert_eq!(rounded(&bar.paint(RECT))[0].2, bar.track.get());
    }

    #[test]
    fn a_progress_bar_paints_its_track_and_its_fill_in_that_order() {
        let (_nodes, bar) = at(0.5);
        let commands = bar.paint(RECT);
        assert_eq!(
            shapes(&commands),
            vec!["rect", "rect"],
            "the fill is over the track, or it would be hidden"
        );
        let drawn = rounded(&commands);
        assert_eq!(drawn[0].2, bar.track.get(), "the track's colour");
        assert_eq!(drawn[1].2, bar.fill.get(), "and the fill's own");
        assert_eq!(drawn[1].0, Rect::new(32.0, 73.0, 100.0, 6.0));
    }

    #[test]
    fn the_fills_width_is_the_value_times_the_tracks_length() {
        // 200 long, so each fifth is 40 and the numbers are exact.
        let width_at = |value: f32| fill_width(&at(value).1, RECT);
        assert_close(width_at(0.0), 0.0, "nothing done");
        assert_close(width_at(0.25), 50.0, "a quarter");
        assert_eq!(width_at(0.5), 100.0, "half");
        assert_close(width_at(0.75), 150.0, "three quarters");
        assert_eq!(width_at(1.0), 200.0, "all of it");
        assert_close(
            width_at(0.5),
            width_at(0.25) * 2.0,
            "and the width is linear in the value",
        );
    }

    #[test]
    fn a_full_bar_covers_its_track_exactly() {
        // At 1.0 the fill's rect *is* the track's rect. That is the whole of why
        // the fill carries the track's own radius: a fill of the right rect with
        // square corners would stand outside the track's four rounded corners at
        // every pixel of "full".
        let (_nodes, bar) = at(1.0);
        let (track, fill) = bars(&bar, RECT);
        assert_eq!(fill.unwrap().0, track.0, "the same rect");
        assert_eq!(fill.unwrap().1, track.1, "and the same radius");
    }

    #[test]
    fn the_fill_carries_the_tracks_own_radius_at_both_ends() {
        // One number for the radius at every value, and the value 1.0 in
        // particular: a radius of zero is the defect above, and a radius larger
        // than the track's would round the fill's corners away from the track's.
        let (_nodes, bar) = at(0.0);
        for value in [0.05, 0.25, 0.5, 0.9, 1.0] {
            bar.value.set(value);
            bar.snap_to_state();
            let (track, fill) = bars(&bar, RECT);
            let (track_rect, track_radius) = track;
            let (fill_rect, fill_radius) = fill.unwrap();
            assert_eq!(
                fill_radius, track_radius,
                "at {value} the fill's radius is the track's"
            );
            assert_eq!(fill_radius, TRACK_THICKNESS / 2.0);
            assert!(inside(track_rect, fill_rect), "at {value} it stays inside");
        }
    }

    #[test]
    fn the_fill_never_reaches_outside_the_track_at_any_value() {
        // The promise, stated once over every value a caller might write. The
        // sweep starts at 0.04 rather than at nothing, because a bar at `0.0`
        // records no fill to contain — which is the other half of the same rule
        // and is asserted by `a_bar_with_nothing_to_show_paints_only_its_track`.
        // A fill that escaped at any one of these would be a bar drawn over its
        // own neighbours.
        let (_nodes, bar) = progress();
        for step in 1u8..=25 {
            let value = f32::from(step) * 0.04;
            bar.value.set(value);
            bar.snap_to_state();
            let (track, fill) = bars(&bar, RECT);
            let (track_rect, _) = track;
            let fill_rect = fill.unwrap().0;
            assert!(
                inside(track_rect, fill_rect),
                "at {value} the fill {fill_rect:?} is inside {track_rect:?}"
            );
        }
    }

    #[test]
    fn a_bar_with_nothing_to_show_paints_only_its_track() {
        // A zero-width rounded rectangle is a quad of nothing, and requirement 2
        // asks for a fill whose width is the value's share of the track: at zero
        // that share is nothing to record.
        let (_nodes, bar) = at(0.0);
        let commands = bar.paint(RECT);
        assert_eq!(commands.len(), 1);
        assert_eq!(shapes(&commands), vec!["rect"]);
        assert_eq!(rounded(&commands)[0].2, bar.track.get());
    }

    #[test]
    fn a_bar_away_from_the_origin_measures_its_own_extent_and_not_its_position() {
        // Every rect in this module is away from the origin, which is the point
        // of them; this is the test that names the defect. Two bars of the same
        // size at different places draw fills of the same length, and the fill
        // starts at the track's own x — a version of this widget that treated
        // `x` as an extent would report a negative travel here and pin the fill
        // to one end of the track.
        let rect = RECT;
        assert_eq!(rect.x, 32.0, "the fixture really is off the origin");

        let (_nodes, bar) = at(0.5);
        let fill = bars(&bar, rect).1.unwrap().0;
        assert_eq!(fill.width, 100.0, "half of the 200-pixel length");
        assert_eq!(fill.x, 32.0, "starting at the track's own left edge");

        // And the same size at another place gives the same length.
        let elsewhere = Rect::new(rect.x + 500.0, rect.y + 300.0, rect.width, rect.height);
        let fill = bars(&bar, elsewhere).1.unwrap().0;
        assert_eq!(fill.width, 100.0, "the length does not follow the position");
        assert_eq!(fill.x, 532.0, "but the position does");
    }

    #[test]
    fn a_vertical_bar_fills_from_the_bottom_up() {
        // The screen's y axis points down, so a vertical bar's minimum is at the
        // bottom of its rect. This is the test that the orientation is real and
        // not a shape swap: the extent is the height, the anchor is the bottom,
        // and the two are not what the horizontal branch would have done.
        let (_nodes, mut bar) = at(0.5);
        bar.set_orientation(Orientation::Vertical);
        let (track, fill) = bars(&bar, TALL);
        assert_eq!(track.0, Rect::new(709.0, 40.0, 6.0, 200.0));
        assert_eq!(
            fill.unwrap().0,
            Rect::new(709.0, 140.0, 6.0, 100.0),
            "the top half of the track, anchored at its bottom"
        );
    }

    #[test]
    fn the_two_orientations_are_not_the_same_picture() {
        // One test that fails if the vertical handling is swapped for the
        // horizontal, on all four things the two differ in: which axis carries
        // the extent, which one carries the thickness, which end the fill grows
        // from, and the shape of the track itself. A transposition of the arms
        // breaks at least the second and the third, and the fourth breaks in
        // both directions. The square rect is what makes the comparison fair: the
        // two tracks are transposes of each other, so nothing about the fixture
        // decides which is which.
        let square = Rect::new(100.0, 100.0, 200.0, 200.0);
        let (_nodes, horizontal) = at(0.25);
        let (_nodes, mut vertical) = at(0.25);
        vertical.set_orientation(Orientation::Vertical);

        let (across, fill) = bars(&horizontal, square);
        assert_eq!(
            fill.unwrap().0.height,
            across.1 * 2.0,
            "a horizontal fill keeps the thickness"
        );
        assert_close(
            fill.unwrap().0.width,
            50.0,
            "and takes a quarter of the length",
        );
        assert_eq!(
            fill.unwrap().0.x,
            square.x,
            "growing from the left, the track's own origin"
        );

        let (down, fill) = bars(&vertical, square);
        assert_eq!(
            fill.unwrap().0.width,
            down.1 * 2.0,
            "a vertical fill keeps the thickness"
        );
        assert_close(
            fill.unwrap().0.height,
            50.0,
            "and takes a quarter of the length",
        );
        assert_eq!(
            fill.unwrap().0.y + fill.unwrap().0.height,
            square.y + square.height,
            "growing from the bottom, not from the top: 100 + 150 + 50"
        );
        assert_ne!(
            across.0, down.0,
            "and the two tracks are transposes of one another"
        );
    }

    #[test]
    fn a_vertical_bar_asks_for_and_paints_its_own_axis() {
        let (_nodes, mut bar) = at(0.5);
        bar.set_orientation(Orientation::Vertical);
        assert!(bar.size().height > bar.size().width, "long down");
        let (track, fill) = bars(&bar, TALL);
        assert_eq!(track.0.height, 200.0, "the whole of the long axis");
        assert_eq!(fill.unwrap().0.height, 100.0, "and half of it filled");
    }

    #[test]
    fn setting_the_orientation_does_not_change_the_value() {
        // A new shape for the same value, not a change of value: the geometry is
        // derived at paint time, so nothing animates.
        let (_nodes, mut bar) = at(0.75);
        bar.animate_to_state(motion());
        tick(&bar, 50);
        let halfway = bar.shown.get();
        bar.set_orientation(Orientation::Vertical);
        assert_eq!(bar.shown.get(), halfway, "the transition is untouched");
        assert_eq!(bar.style().shown, 0.75, "and the value with it");
    }

    #[test]
    fn a_track_thicker_than_its_node_is_still_what_the_fill_lives_inside() {
        // A 2-pixel-tall node with a 6-pixel track is a track that overhangs its
        // own node, which is the caller's doing: the thickness is the bar's, not
        // the node's. What must not change is where the fill is measured — the
        // track's own origin and the track's own length — so the fill is half of
        // 100 and inside the track.
        let (_nodes, bar) = at(0.5);
        let narrow = Rect::new(20.0, 30.0, 100.0, 2.0);
        let (track, fill) = bars(&bar, narrow);
        assert_eq!(track.0.height, TRACK_THICKNESS, "the bar's own thickness");
        let fill = fill.unwrap().0;
        assert_eq!(fill.width, 50.0, "half of the track's 100 pixels");
        assert!(inside(track.0, fill), "and inside the track");
    }

    #[test]
    fn a_track_thickness_of_zero_leaves_a_bar_that_draws_nothing_over_its_track() {
        // The caller asked for no thickness, and gets a track of no height and a
        // fill of no height rather than a negative one.
        let (_nodes, mut bar) = at(0.5);
        bar.set_track_thickness(0.0);
        let (track, fill) = bars(&bar, RECT);
        assert_eq!(track.0.height, 0.0);
        assert_eq!(
            fill, None,
            "a fill as thick as the track is nothing to draw"
        );
    }

    #[test]
    fn a_negative_track_thickness_is_no_thickness() {
        // A negative thickness would put the fill outside its own track, which is
        // the one thing this widget promises never to do.
        let (_nodes, mut bar) = at(0.5);
        bar.set_track_thickness(-20.0);
        let (track, fill) = bars(&bar, RECT);
        assert_eq!(track.0.height, 0.0, "clamped rather than drawn inside out");
        assert_eq!(fill, None);
    }

    #[test]
    fn a_programmatic_change_animates_the_drawn_value_toward_the_truth() {
        // The contract is that the fill *moves* rather than jumping, so this
        // checks the middle of the transition and not only its end: a `set`
        // instead of an animation would pass an end-only assertion.
        let (_nodes, bar) = progress();
        bar.value.set(1.0);
        bar.animate_to_state(motion());

        assert_eq!(
            bar.shown.get(),
            0.0,
            "the fill starts where it was, so a frame drawn now is right"
        );
        assert!(bar.tick(ms(50)));
        let half = bar.shown.get();
        assert!(half > 0.0 && half < 1.0, "half way through, it has moved");
        assert_eq!(half, 0.5, "to exactly half, on a linear curve");

        assert!(bar.tick(ms(50)));
        assert_eq!(bar.shown.get(), 1.0, "and it arrives");
        assert!(!bar.is_animating());
    }

    #[test]
    fn the_fill_follows_the_animating_value_rather_than_the_value() {
        // Requirement 4's first half, asserted on the picture rather than on the
        // property: the fill's width is the measure of what is drawn, and it must
        // be the drawn value's share and not the truth's.
        let (_nodes, bar) = progress();
        bar.value.set(1.0);
        bar.animate_to_state(motion());

        let width = |bar: &Progress| fill_width(bar, RECT);
        let at_start = width(&bar);
        assert_eq!(at_start, 0.0, "nothing filled before the first tick");
        tick(&bar, 50);
        let halfway = width(&bar);
        tick(&bar, 50);
        let arrived = width(&bar);

        assert_eq!(halfway, 100.0, "half of a 200-pixel track at half time");
        assert_eq!(arrived, 200.0, "and all of it when it arrives");
        assert_eq!(
            arrived - halfway,
            halfway - at_start,
            "growing evenly, because the curve is linear"
        );
    }

    #[test]
    fn a_value_that_goes_backwards_animates_backwards() {
        // The other half of "animates when the value changes": not only forwards.
        let (_nodes, bar) = at(0.8);
        bar.value.set(0.2);
        bar.animate_to_state(motion());
        tick(&bar, 50);
        assert_eq!(bar.shown.get(), 0.5, "from 0.8 towards 0.2");
        tick(&bar, 50);
        // `0.8 + (0.2 - 0.8) * 1.0` is `0.19999999` in `f32`, which is the
        // interpolation arriving at its target and not a value short of it: the
        // midpoint above is exact and this end is a product, so it is asserted
        // to the sixth decimal place rather than to the compiler's rounding.
        assert_close(bar.shown.get(), 0.2, "and arrives there");
    }

    #[test]
    fn a_second_change_replaces_the_first_rather_than_racing_it() {
        // A value written twice in quick succession used to be a race: both
        // transitions wrote the drawn value and the older one arriving last left
        // it short of the truth. The bar's own clock is what rules that out.
        let (_nodes, bar) = progress();
        bar.value.set(1.0);
        bar.animate_to_state(motion());
        tick(&bar, 50);
        bar.value.set(0.25);
        bar.animate_to_state(motion());
        for _ in 0..4 {
            tick(&bar, 50);
        }
        assert_eq!(bar.shown.get(), 0.25, "the fill ends at the newest value");
        assert!(!bar.is_animating(), "and nothing is left running");
    }

    #[test]
    fn a_determinate_bar_leaves_the_sliding_bar_alone() {
        // Nothing draws `slide` in this mode, so a transition on it is a
        // transition that reports itself running for nothing.
        let (_nodes, bar) = progress();
        bar.animate_to_state(motion());
        assert_eq!(bar.slide.get(), 0.0);
        tick(&bar, 50);
        assert_eq!(bar.slide.get(), 0.0, "it does not move on its own");
    }

    #[test]
    fn an_indeterminate_bar_leaves_the_determinate_fill_alone() {
        // The mirror of the test above, and the reason for it: `shown` is what
        // the mode does not draw, so animating it here would be a transition
        // that reports itself running while the only thing moving is the bar.
        let (_nodes, mut bar) = progress();
        bar.value.set(0.5);
        bar.set_indeterminate(true);
        bar.animate_to_state(motion());
        tick(&bar, 100);
        assert_eq!(bar.shown.get(), 0.0, "the determinate fill has not moved");
        assert_eq!(bar.slide.get(), 0.5, "while the sliding one has");
    }

    #[test]
    fn the_palette_switch_is_animated_over_the_motions_own_duration() {
        // The colours are properties for the same reason the drawn value is: a
        // theme switch is a transition, not a jump.
        let (_nodes, mut bar) = progress();
        let light = Palette::from_theme(&Theme::light());
        let before = bar.fill.get();
        bar.set_palette(light);
        bar.animate_to_state(motion());

        tick(&bar, 50);
        let halfway = bar.fill.get();
        assert_ne!(halfway, before, "the colour has left the old theme");
        assert_ne!(halfway, light.fill, "and has not arrived at the new one");
        tick(&bar, 50);
        assert_eq!(
            bar.fill.get(),
            light.fill,
            "arriving exactly at half of each"
        );
        assert_eq!(bar.track.get(), light.track);
    }

    #[test]
    fn an_indeterminate_bar_slides_from_the_near_end_to_the_far_end() {
        // One leg is two of the motion's durations, so 100 ms of a 100 ms motion
        // is half a traverse.
        let (_nodes, bar) = sliding();
        assert_eq!(bar.slide.get(), 0.0, "it starts at the near end");
        tick(&bar, 100);
        assert_eq!(bar.slide.get(), 0.5, "half a traverse");
        tick(&bar, 100);
        assert_eq!(bar.slide.get(), 1.0, "and arrives at the far end");
    }

    #[test]
    fn a_slide_leg_takes_two_of_the_motions_durations() {
        // The multiplier, asserted through the clock rather than read off the
        // constant: with a 100 ms motion a leg arrives at 200 ms and not at 100.
        let (_nodes, bar) = sliding();
        tick(&bar, 199);
        assert!(
            bar.slide.get() < 1.0,
            "a millisecond short of two durations is short of the end: {}",
            bar.slide.get()
        );
        tick(&bar, 1);
        assert_eq!(
            bar.slide.get(),
            1.0,
            "and one millisecond later it is there"
        );
    }

    #[test]
    fn the_loop_runs_back_again_without_the_caller_asking() {
        // The widget keeps the loop going: `tick` aims each next leg, so a
        // caller that called `animate_to_state` once is running it forever.
        let (_nodes, bar) = sliding();
        for _ in 0..2 {
            tick(&bar, 100);
        }
        assert_eq!(bar.slide.get(), 1.0, "arrived at the far end");
        tick(&bar, 100);
        assert_eq!(bar.slide.get(), 0.5, "and already on its way back");
        tick(&bar, 100);
        assert_eq!(bar.slide.get(), 0.0, "back at the near end");
        assert!(
            bar.is_animating(),
            "and still running, which is what a loop is"
        );
    }

    #[test]
    fn an_aimed_indeterminate_bar_is_never_at_rest() {
        // A bar in indeterminate mode has something to show on every frame, so
        // `is_animating` is true at rest too — including on the frame a leg
        // arrives, where the next one has already been aimed.
        let (_nodes, bar) = sliding();
        assert!(bar.is_animating());
        tick(&bar, 50);
        assert!(bar.is_animating(), "mid leg");
        assert!(bar.tick(ms(150)), "the arriving frame reports a write");
        assert!(bar.is_animating(), "and the next leg is under way");
    }

    #[test]
    fn an_indeterminate_bar_that_has_never_been_aimed_does_not_slide() {
        // Nothing moves until a caller aims it, which is the same rule the rest
        // of the library follows. Ten seconds of ticks changes nothing and leaves
        // nothing running.
        let (_nodes, mut bar) = progress();
        bar.set_indeterminate(true);
        for _ in 0..10 {
            assert!(!bar.tick(ms(1000)));
        }
        assert_eq!(bar.slide.get(), 0.0);
        assert!(!bar.is_animating());
    }

    #[test]
    fn switching_into_indeterminate_parks_the_bar_at_the_start_of_a_leg() {
        // The bar may have been abandoned half a leg along; the first leg after a
        // mode switch starts at the near end rather than from wherever that was.
        let (_nodes, mut bar) = sliding();
        tick(&bar, 100);
        assert_eq!(bar.slide.get(), 0.5, "it was mid leg");

        bar.set_indeterminate(false);
        bar.set_indeterminate(true);
        assert_eq!(bar.slide.get(), 0.0, "parked at the start of a leg");
        assert!(
            !bar.is_animating(),
            "and the abandoned leg is not still running"
        );
    }

    #[test]
    fn switching_out_of_indeterminate_writes_the_drawn_value_at_once() {
        // A mode change is a fact, not a step: the first determinate frame is the
        // truth, not a fill animating in from wherever the sliding bar was.
        let (_nodes, mut bar) = progress();
        bar.value.set(0.75);
        bar.set_indeterminate(true);
        bar.animate_to_state(motion());
        tick(&bar, 100);
        assert_eq!(bar.shown.get(), 0.0, "the determinate fill was not drawn");

        bar.set_indeterminate(false);
        assert_eq!(
            bar.shown.get(),
            0.75,
            "and is the truth the moment the mode is over"
        );
        assert!(!bar.is_animating());
    }

    #[test]
    fn an_indeterminate_bar_ignores_the_value() {
        // The value says nothing about how far the operation has got, so it must
        // not move the sliding bar: at 1.0 the bar is still a quarter of the
        // track rather than the whole of it.
        let (_nodes, bar) = sliding();
        bar.value.set(1.0);
        tick(&bar, 100);
        let (track, fill) = bars(&bar, RECT);
        assert_close(
            fill.unwrap().0.width,
            50.0,
            "a quarter of a 200-pixel track, whatever the value says",
        );
        assert!(inside(track.0, fill.unwrap().0));
    }

    #[test]
    fn the_sliding_bar_keeps_its_own_length_while_it_slides() {
        let (_nodes, bar) = sliding();
        let widths = [
            bars(&bar, RECT).1.unwrap().0.width,
            {
                tick(&bar, 100);
                bars(&bar, RECT).1.unwrap().0.width
            },
            {
                tick(&bar, 100);
                bars(&bar, RECT).1.unwrap().0.width
            },
        ];
        assert_eq!(widths, [50.0, 50.0, 50.0], "a quarter of 200 throughout");
    }

    #[test]
    fn the_sliding_bar_travels_the_length_the_track_leaves_for_it() {
        // 200 of track, 50 of bar, so 150 of travel — the bar's left edge goes
        // from the track's own left edge to 150 along, and never further.
        let (_nodes, bar) = sliding();
        let left = |bar: &Progress| bars(bar, RECT).1.unwrap().0.x;
        assert_eq!(left(&bar), 32.0, "at the near end");
        tick(&bar, 100);
        assert_close(left(&bar), 107.0, "half way: 32 + 75");
        tick(&bar, 100);
        assert_eq!(left(&bar), 182.0, "at the far end, its right edge at 232");
    }

    #[test]
    fn a_vertical_indeterminate_bar_slides_upwards() {
        // The direction is load-bearing: a vertical bar's sliding bar starts at
        // the *bottom* and travels up, because the screen's y axis points down
        // and progress should go up. Transposing the arms of the orientation
        // match would put it at the top and send it down, and this fails.
        let (_nodes, mut bar) = sliding();
        bar.set_orientation(Orientation::Vertical);
        let (track, fill) = bars(&bar, TALL);
        let slide_rect = fill.unwrap().0;
        assert_eq!(slide_rect.height, 50.0, "the same quarter of the length");
        assert_eq!(
            slide_rect.y + slide_rect.height,
            track.0.y + track.0.height,
            "starting hard against the bottom of the track"
        );

        tick(&bar, 200);
        let arrived = bars(&bar, TALL).1.unwrap().0;
        assert_eq!(arrived.y, track.0.y, "and arriving at the top of it");
        assert_eq!(arrived.height, 50.0, "with its own length intact");
    }

    #[test]
    fn the_sliding_bar_never_leaves_the_track_at_any_position() {
        // The promise, over every position the property can be written to
        // including the ones no curve produces: below zero, above one, and
        // `NaN`.
        let (_nodes, mut bar) = progress();
        bar.set_indeterminate(true);
        for position in [-1.0, -0.1, 0.0, 0.25, 0.5, 0.75, 1.0, 1.1, 4.0, f32::NAN] {
            bar.slide.set(position);
            let (track, fill) = bars(&bar, RECT);
            let fill_rect = fill.unwrap().0;
            assert!(
                inside(track.0, fill_rect),
                "at {position} the bar {fill_rect:?} is inside {:?}",
                track.0
            );
        }
    }

    #[test]
    fn an_overshooting_curve_cannot_push_the_sliding_bar_off_its_track() {
        // A spring passes its target on the way and comes back, so the property
        // really does leave `0.0..=1.0` mid-leg. The drawn position is the value
        // clamped, so the bar reaches the end of its travel and stops there for
        // the frames of the overshoot.
        let (_nodes, mut bar) = progress();
        bar.set_indeterminate(true);
        bar.animate_to_state(Motion {
            duration: ms(100),
            easing: Easing::Spring {
                damping: 2.0,
                stiffness: 100.0,
            },
        });

        let mut furthest = 0.0_f32;
        let (track, _) = bars(&bar, RECT);
        for _ in 0..20 {
            tick(&bar, 10);
            furthest = furthest.max(bar.slide.get());
            let fill = bars(&bar, RECT).1.unwrap().0;
            assert!(
                inside(track.0, fill),
                "the bar stayed inside at slide {}",
                bar.slide.get()
            );
        }
        assert!(
            furthest > 1.0,
            "and the curve really did overshoot, or this proves nothing: {furthest}"
        );
        assert_eq!(
            bars(&bar, RECT).1.unwrap().0.x + 50.0,
            track.0.x + track.0.width,
            "so the bar sat at the very end of the track and no further"
        );
    }

    #[test]
    fn a_sliding_bar_shorter_than_the_track_is_still_a_bar() {
        // A fraction small enough to give a bar under one thickness on a narrow
        // track: the bar is held to one thickness rather than drawn as a square
        // that cannot appear to move.
        let (_nodes, mut bar) = progress();
        bar.set_indeterminate(true);
        bar.set_indeterminate_fraction(0.01);
        let narrow = Rect::new(10.0, 10.0, 8.0, 24.0);
        let (track, fill) = bars(&bar, narrow);
        assert_eq!(
            fill.unwrap().0.width,
            TRACK_THICKNESS,
            "one thickness of a track barely longer than that"
        );
        assert!(inside(track.0, fill.unwrap().0));
    }

    #[test]
    fn an_indeterminate_fraction_outside_the_unit_range_is_clamped() {
        // A caller error must not put a bar outside its own track, so both ends
        // are clamped rather than trusted — the same rule the value follows.
        let (_nodes, mut bar) = sliding();
        let length = |bar: &Progress| bars(bar, RECT).1.unwrap().0.width;

        bar.set_indeterminate_fraction(-1.0);
        assert_close(length(&bar), TRACK_THICKNESS, "a floor of one thickness");
        bar.set_indeterminate_fraction(f32::NAN);
        assert_close(length(&bar), TRACK_THICKNESS, "and no fraction at all");
        bar.set_indeterminate_fraction(9.0);
        assert_eq!(length(&bar), 200.0, "no more than the whole track");
    }

    #[test]
    fn a_sliding_bar_that_fills_its_whole_track_cannot_leave_it() {
        // The extreme of the fraction: travel of zero, so the bar is the track and
        // there is nothing for it to do.
        let (_nodes, mut bar) = progress();
        bar.set_indeterminate(true);
        bar.set_indeterminate_fraction(1.0);
        bar.animate_to_state(motion());
        let (track, fill) = bars(&bar, RECT);
        assert_eq!(fill.unwrap().0, track.0, "it is the track");
        for _ in 0..4 {
            tick(&bar, 100);
            assert!(inside(track.0, bars(&bar, RECT).1.unwrap().0));
        }
    }

    #[test]
    fn a_sliding_bar_on_an_empty_track_draws_nothing() {
        // A track with no length has no bar in it, and a bar's length is not
        // allowed to be one thickness when there is nowhere for a thickness to
        // go.
        let (_nodes, bar) = sliding();
        let (track, fill) = bars(&bar, Rect::new(50.0, 50.0, 0.0, 24.0));
        assert_eq!(track.0.width, 0.0);
        assert_eq!(fill, None);
    }

    #[test]
    fn the_only_way_a_bar_changes_is_a_write_to_its_value() {
        // Requirement 4 lists no gesture for a bar, so this widget has no
        // `on_event` at all: there is nothing for a tap on it to report. What a
        // caller drives instead is the value property, and the property's own
        // callback is the link to the node.
        let (_nodes, bar) = progress();
        assert_eq!(
            bar.paint(RECT).len(),
            1,
            "the only way it changes is a write"
        );
        bar.value.set(0.5);
        bar.snap_to_state();
        assert_eq!(bar.paint(RECT).len(), 2);
    }
}
