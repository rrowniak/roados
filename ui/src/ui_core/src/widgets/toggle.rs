//! The Toggle widget: a binary state, a pill, and a thumb that slides between
//! its two ends.
//!
//! A toggle is a node, the properties the task gives it, and four it animates.
//! [`Toggle::checked`] is the truth; [`Toggle::position`] is where the thumb is
//! *drawn*, and the distance between the two is the transition. That split is
//! what makes requirement 4's "thumb slides from left to right" and a
//! programmatic `checked.set(true)` the same widget: an interaction writes the
//! state alone, and [`Toggle::animate_to_state`] carries the thumb, the track's
//! colour and the bounce across — so a toggle set in code slides exactly as one
//! a finger turned does.
//!
//! The colours follow the button's and the slider's rule rather than adding
//! theme tokens: a [`Palette`] names the four colours a toggle needs — the off
//! track, the on track, the thumb and the focus ring — the properties hold them
//! so a theme switch can be animated into them, and [`Toggle::snap_to_state`]
//! puts a themed toggle on its theme at once. The *sizing* — how long the pill
//! is, how thick it is, how big the thumb is — is named constants rather than
//! tokens, for the reason the button's own `MIN_TOUCH_TARGET` documents: the
//! theme has no token for a toggle's parts, and adding one would change
//! [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables, the
//! token count and the animation every token takes part in during a switch, for
//! values a switch does not change. Each constant below says what would reverse
//! that.
//!
//! Two of the toggle's effects are *how much* rather than *which colour*, and
//! are constants with a cap for the reason the button's own
//! `PRESS_SHADOW_ALPHA` documents for its press overlay: the thumb's shadow is
//! black under a ceiling, and its bounce is a scale under a ceiling. An effect
//! with an adjective in its spec — "a slight scale bounce" — needs the number
//! the adjective fixes, written out, so a test can assert it.
//!
//! Two things are the caller's, not the toggle's: which node is dirty, and which
//! node is focused. The first is a property callback per the demo's own idiom —
//! `on_change` on the animated properties, marking the node — and the second is
//! [`input::Focus`](crate::input::Focus), which knows the focus order and not
//! what Enter means. A disabled toggle refuses its interactions and is drawn
//! inert; *stepping over it* in the focus order is `Focus::set_focusable`, which
//! the caller calls, because the widget cannot reach the arena that holds it.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use ui_core::arena::Arena;
//! use ui_core::input::{InputEvent, InputEventKind};
//! use ui_core::layout::Offset;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::Rect;
//! use ui_core::theme::Theme;
//! use ui_core::widgets::button::Motion;
//! use ui_core::widgets::toggle::{Palette, Toggle};
//!
//! let mut nodes = Arena::new();
//! let mut toggle = Toggle::new(&mut nodes);
//! toggle.set_palette(Palette::from_theme(&Theme::dark()));
//! toggle.snap_to_state();
//!
//! // A tap flips the state at once, and the thumb follows it across the track.
//! let rect = Rect::new(0.0, 0.0, 48.0, 44.0);
//! let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(24.0, 22.0)));
//! assert!(toggle.on_event(&mut tap, rect));
//! assert!(toggle.checked.get());
//! assert!(tap.consumed(), "so it does not travel on to the panel behind it");
//!
//! toggle.animate_to_state(Motion::from_theme(&Theme::dark()));
//! for _ in 0..15 {
//!     toggle.tick(Duration::from_millis(10));
//! }
//! assert_eq!(
//!     toggle.position.get(),
//!     1.0,
//!     "and the thumb has arrived at the other end of the pill"
//! );
//! ```

use std::cell::{Cell, RefCell};
use std::time::Duration;

use crate::animation::{AnimationClock, Interpolate};
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::{Offset, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::button::Motion;
use crate::widgets::Callback;

/// How wide the pill is, in pixels.
///
/// Two thumb diameters with nothing spare: the thumb's centre travels a radius
/// in from each end, so at the left end the thumb's left edge is flush with the
/// pill's and at the right end its right edge is, and there is no room to grow.
///
/// It is a constant rather than a theme token for the reason the module
/// documentation gives. Revisit when a second control needs the same length and
/// the two should move together, or when the operator wants a toggle's size
/// themeable — either is a theme change, not a toggle change.
const TRACK_WIDTH: f32 = 48.0;

/// How thick the pill is, in pixels.
///
/// Half of the pill's width, which is what makes it a switch rather than a
/// lozenge, and 2 pixels more than the thumb is across, so the thumb sits inside
/// the pill with a pixel of track above and below it.
///
/// It is a constant for the reason `TRACK_WIDTH` is. Revisit on the same
/// condition.
const TRACK_HEIGHT: f32 = 28.0;

/// The radius of the thumb, in pixels, at rest.
///
/// Half the pill's height less two pixels, so the thumb is a circle that fits
/// inside the pill at either end: a thumb as big as the pill would be flush with
/// it rather than in it, and there would be no track to see on either side of
/// it at all.
///
/// It is a constant for the reason `TRACK_WIDTH` is. Revisit on the same
/// condition.
const THUMB_RADIUS: f32 = 12.0;

/// How far the thumb's shadow reaches past the thumb, in pixels.
///
/// The shadow is the larger of the two circles the thumb is drawn as, the same
/// trick [`Slider`](crate::widgets::slider::Slider) uses for a thumb's border,
/// and two pixels is what reads as the edge of a shadow rather than as a ring
/// drawn on purpose.
///
/// It is a constant for the reason `TRACK_WIDTH` is. Revisit on the same
/// condition.
const THUMB_SHADOW_INSET: f32 = 2.0;

/// The most opaque the thumb's shadow ever gets.
///
/// The task asks for a thumb "with shadow" and says nothing about how dark it
/// is, so the number needs a reason: 0.3 of black is a shadow, where half and
/// above reads as an outline and a tenth is invisible on either of the two track
/// colours. It is the same kind of ceiling as the button's
/// `PRESS_SHADOW_ALPHA`, and for the same reason — a shape assertion cannot tell
/// a shadow from a black disc, so the alpha is asserted instead.
const THUMB_SHADOW_OPACITY: f32 = 0.3;

/// The scale the thumb is drawn at for the first half of a toggle's bounce.
///
/// The task asks for a "slight scale bounce on toggle", and *slight* is the word
/// that has to become a number: a quarter bigger is slight and half again is
/// not, and the ceiling between the two is a named number in the tests, where
/// the assertion that uses it lives. The bounce is the thumb arriving at this
/// scale on the frame the state changes and easing back to its own size over the
/// same duration the slide takes, so it is an out-and-back pulse and not an
/// oscillation — the theme's standard curve is monotonic, and a two-step
/// animation on one property does not work here because a delayed animation
/// writes its own start value on every frame of its delay, which would pin the
/// thumb at this scale for the whole first leg.
const THUMB_BOUNCE_SCALE: f32 = 1.25;

/// The focus ring's width, in pixels, unless a caller changes it.
///
/// The ring is drawn **around the pill** and then covered by it, so only its
/// border shows: a [`DrawCommand::RoundedRect`] fills its rect, and a ring
/// around the toggle's whole node would be a card rather than an outline.
/// [`Toggle::paint`] says so where it draws it, and the test that covers it
/// asserts the grown shape *and* the track recorded over it.
const FOCUS_RING: f32 = 2.0;

/// The smallest a toggle may be, in either dimension, in pixels.
///
/// 44dp is the platform touch target a finger can hit, and it is the same floor
/// [`button::MIN_TOUCH_TARGET`](crate::widgets::button::Button) puts under a
/// button. A toggle's *pill* is 28 pixels tall, which no finger can be asked to
/// hit, so the floor is on the node and the pill is centred inside it: the pill
/// is what is drawn and the node is what is found.
///
/// It is a constant for the reason `TRACK_WIDTH` is, and it is repeated rather
/// than imported for the reason the slider's own copy repeats the button's: a
/// `pub const` in another widget's module is not a shared place to keep one, and
/// the two will diverge when the theme grows a density setting.
const MIN_TOUCH_TARGET: f32 = 44.0;

/// The opacity a disabled toggle's own colours are drawn at.
///
/// The button's own number, and for the same reason: a disabled control is one
/// that cannot be pressed, and the way to say so without inventing a fourth
/// colour is to draw the two it has as not fully there. It is applied in
/// [`Toggle::style`], so the change is animated by the ordinary colour
/// properties and needs no opacity property of its own.
const DISABLED_OPACITY: f32 = 0.45;

/// The colours a toggle draws with.
///
/// Four colours: the pill while it is off, the pill while it is on, the thumb,
/// and the focus ring. They are not tokens of their own — the theme has none
/// per part, and adding one per part would put four more tokens in every theme
/// table and in every theme switch — so a toggle is themed with the theme's own
/// four and [`Palette::from_theme`] says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The pill while the toggle is off: the muted colour, and the state a
    /// toggle rests in.
    pub track_off: Color,
    /// The pill while the toggle is on: the primary colour, because a toggle
    /// that is on is a primary action that has been taken.
    pub track_on: Color,
    /// The thumb, which sits on whichever of the two track colours is drawn
    /// under it.
    pub thumb: Color,
    /// The focus indicator, drawn around the pill and covered by it.
    pub ring: Color,
}

impl Default for Palette {
    /// Returns a neutral grey toggle: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    fn default() -> Self {
        Palette {
            track_off: Color::new(80, 80, 80, 255),
            track_on: Color::new(150, 150, 150, 255),
            thumb: Color::new(235, 235, 235, 255),
            ring: Color::new(255, 255, 255, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The two track colours are [`TextMuted`](crate::theme::ThemeToken::TextMuted)
    /// and [`Primary`](crate::theme::ThemeToken::Primary): the task asks for an
    /// *off, muted* track and an *on, primary* one, and `TextMuted` is the one
    /// token the theme describes as muted. The slider's track is
    /// [`Border`](crate::theme::ThemeToken::Border) instead, and that is not a
    /// disagreement: a slider's track is a hairline the value runs along, and a
    /// toggle's pill is a filled surface that is either one colour or the other.
    ///
    /// The thumb is [`OnPrimary`](crate::theme::ThemeToken::OnPrimary) — the
    /// task's own second choice, "white or on-primary" — because it is by
    /// definition the colour drawn on `Primary`, which is the track it sits on
    /// for half of every toggle's life, and it stays legible on `TextMuted` in
    /// both themes as well.
    ///
    /// The focus ring is [`Text`](crate::theme::ThemeToken::Text), for the
    /// reason the slider's is: the ring is drawn around the pill, so it is a
    /// border *inside* the toggle's own box rather than something under it, and
    /// `Text` is the colour this repository uses for anything that has to be
    /// legible against a surface that may be either of the two.
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            track_off: token_color(theme, crate::theme::ThemeToken::TextMuted),
            track_on: token_color(theme, crate::theme::ThemeToken::Primary),
            thumb: token_color(theme, crate::theme::ThemeToken::OnPrimary),
            ring: token_color(theme, crate::theme::ThemeToken::Text),
        }
    }
}

/// The appearance the toggle's state and its flags imply.
///
/// Every field is a target, not a value in flight:
/// [`Toggle::animate_to_state`] animates the toggle's properties toward this and
/// [`Toggle::paint`] draws whatever the properties have reached, which is a
/// [`Style`] part way through on a frame where something is moving.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// Where the thumb's centre sits along the pill: `0.0` at the off end and
    /// `1.0` at the on end.
    pub position: f32,
    /// The scale the thumb is drawn at, which is `1.0` except while a bounce is
    /// running.
    pub thumb_scale: f32,
    /// The colour of the pill.
    pub track: Color,
    /// The colour of the thumb.
    pub thumb: Color,
    /// The focus ring's thickness, or zero for no ring.
    pub ring_width: f32,
}

/// A toggle: a control that is either on or off.
///
/// The widget holds the properties the task gives it — [`checked`] and
/// [`on_change`] — the two state properties a caller writes, [`focused`] and
/// [`disabled`], and the four it animates: [`position`], [`track`], [`thumb`]
/// and [`thumb_scale`]. It also holds [`focus_ring`], which is the width of the
/// indicator a caller may want to turn off, for the same reason the slider
/// holds one.
///
/// The node is the caller's to keep clean, and its size is the caller's to give
/// through [`layout_mut`](crate::node::WidgetNode::layout_mut) —
/// [`Toggle::size`] is a suggestion for a caller who has nothing else to go on.
/// A toggle draws inside whatever rect it is given: a pill centred in it, and a
/// thumb travelling between the pill's two ends.
///
/// [`checked`]: Toggle::checked
/// [`on_change`]: Toggle::on_change
/// [`focused`]: Toggle::focused
/// [`disabled`]: Toggle::disabled
/// [`position`]: Toggle::position
/// [`track`]: Toggle::track
/// [`thumb`]: Toggle::thumb
/// [`thumb_scale`]: Toggle::thumb_scale
/// [`focus_ring`]: Toggle::focus_ring
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::{InputEvent, InputEventKind};
/// use ui_core::layout::Offset;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::toggle::Toggle;
///
/// let mut nodes = Arena::new();
/// let mut toggle = Toggle::new(&mut nodes);
/// assert!(!toggle.checked.get(), "a toggle starts off");
///
/// // A tap anywhere in the toggle's own rect flips it, and is consumed.
/// let rect = Rect::new(0.0, 0.0, 48.0, 44.0);
/// let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(4.0, 4.0)));
/// assert!(toggle.on_event(&mut tap, rect));
/// assert!(toggle.checked.get());
/// ```
pub struct Toggle {
    /// Whether the toggle is on. The truth, and the one the next interaction
    /// acts on: an interaction writes it at once and
    /// [`animate_to_state`](Toggle::animate_to_state) carries the appearance
    /// after it. A caller may write it itself — that is the "set in code" case —
    /// and the thumb slides across on the same transition a tap would have used.
    pub checked: Property<bool>,
    /// Where the thumb is *drawn*, in `0.0..=1.0` of the pill's travel: `0.0`
    /// at the off end and `1.0` at the on end.
    ///
    /// It is animated rather than derived from [`checked`](Toggle::checked), so
    /// a caller that wants the thumb somewhere else can write it: the widget
    /// clamps it to the two ends, and a value outside them reads as the end it
    /// is past.
    pub position: Property<f32>,
    /// The scale the thumb is drawn at, animated by a toggle's bounce and put
    /// back to `1.0` by [`snap_to_state`](Toggle::snap_to_state).
    pub thumb_scale: Property<f32>,
    /// The colour of the pill: the muted colour while off and the primary one
    /// while on, interpolated between on the way.
    pub track: Property<Color>,
    /// The colour of the thumb.
    pub thumb: Property<Color>,
    /// The focus ring's thickness, in pixels. Zero draws no ring even when the
    /// toggle is focused.
    pub focus_ring: Property<f32>,
    /// Whether the toggle holds focus. Written by the caller, from
    /// [`input::Focus`](crate::input::Focus), and it is what a key press acts on.
    pub focused: Property<bool>,
    /// Whether the toggle refuses interaction and is drawn inert. A disabled
    /// toggle still swallows a tap aimed at it, for the reason a disabled button
    /// does, and it is skipped by the focus order because the caller leaves it
    /// out of [`Focus::set_focusable`](crate::input::Focus::set_focusable).
    pub disabled: Property<bool>,
    /// The callback every interaction fires, with the state it moved to.
    ///
    /// It fires when the state *changed*, which for a toggle is every time it
    /// acts: a tap on a toggle that is already on switches it off and says so.
    /// A caller that writes [`checked`](Toggle::checked) itself is itself, and
    /// reads the property; the callback is the widget reporting its own doing.
    pub on_change: Callback<bool>,
    palette: Palette,
    /// The clock the slide and the colour crossfade run on. It is a clock of the
    /// toggle's own for the reason the button's is: `AnimationClock::clear` is
    /// whole-clock, so a clock shared with another widget would strand their
    /// transitions.
    clock: RefCell<AnimationClock>,
    /// The clock the bounce runs on, apart from the clock above, so that each of
    /// the toggle's animated properties has exactly one writer. `clear` is
    /// whole-clock, so one clock driving both would have a re-aim drop the bounce
    /// along with the slide — and two animations writing one property leave the
    /// last one ticked in charge of it.
    bounce_clock: RefCell<AnimationClock>,
    /// The state the last aim was for. The bounce is played on a *change* of
    /// state rather than on every call, so a caller that aims every frame gets
    /// one bounce per toggle rather than a permanently grown thumb.
    aimed: Cell<bool>,
    node: Handle,
}

impl Toggle {
    /// Creates a toggle in the arena, off, and returns it.
    ///
    /// The colours are the neutral defaults until a caller gives it a
    /// [`Palette`](Toggle::set_palette) and calls
    /// [`snap_to_state`](Toggle::snap_to_state) or
    /// [`animate_to_state`](Toggle::animate_to_state); the sizing is the
    /// constants above.
    ///
    /// The task file's `Toggle::new() -> Handle` is read as this: the handle is
    /// [`Toggle::handle`]'s, and returning it alone would leave a caller with no
    /// properties to set and no callback to register, which is the whole of what
    /// the toggle is for. Task 12's [`Button::new`](crate::widgets::button::Button::new)
    /// and task 14's [`Slider::new`](crate::widgets::slider::Slider::new) settled
    /// the same reading.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>) -> Self {
        let palette = Palette::default();
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Toggle {
            checked: Property::new(false),
            position: Property::new(0.0),
            thumb_scale: Property::new(1.0),
            track: Property::new(palette.track_off),
            thumb: Property::new(palette.thumb),
            focus_ring: Property::new(FOCUS_RING),
            focused: Property::new(false),
            disabled: Property::new(false),
            on_change: Callback::none(),
            palette,
            clock: RefCell::new(AnimationClock::new()),
            bounce_clock: RefCell::new(AnimationClock::new()),
            aimed: Cell::new(false),
            node,
        }
    }

    /// Returns the toggle's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the toggle draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the toggle draws with, and leaves the current ones where
    /// they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](Toggle::animate_to_state) or
    /// [`snap_to_state`](Toggle::snap_to_state): a theme switch is animated, and
    /// a theme switch is the caller announcing a new palette and then moving the
    /// toggle toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the toggle chasing a palette that is still
    /// moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the size a toggle asks for: its pill, floored at the touch target
    /// in both directions.
    ///
    /// A toggle has no content to measure, so this is only for a caller that has
    /// nothing else to go on; a caller that lays the toggle out itself gives the
    /// node whatever rect it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](Toggle::paint) centres the pill inside whatever it is given.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::toggle::Toggle;
    ///
    /// let mut nodes = Arena::new();
    /// let toggle = Toggle::new(&mut nodes);
    /// let size = toggle.size();
    ///
    /// assert_eq!(size.width, 48.0, "as wide as the pill");
    /// assert_eq!(size.height, 44.0, "and as tall as a finger can reach");
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(
            TRACK_WIDTH.max(MIN_TOUCH_TARGET),
            TRACK_HEIGHT.max(MIN_TOUCH_TARGET),
        )
    }

    /// Returns the appearance the toggle's state and flags imply.
    ///
    /// The disabled state is applied last and to both of the colours, so a
    /// disabled toggle is dim wherever it sits, and it drops the focus ring: a
    /// control that cannot be focused should not claim to be. Its
    /// [`position`](Style::position) is untouched, so a disabled toggle still
    /// shows which of the two states it is in.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::toggle::{Palette, Toggle};
    ///
    /// let mut nodes = Arena::new();
    /// let mut toggle = Toggle::new(&mut nodes);
    /// toggle.set_palette(Palette::from_theme(&Theme::dark()));
    /// let off = toggle.style();
    ///
    /// toggle.checked.set(true);
    /// let on = toggle.style();
    /// assert_eq!(on.position, 1.0, "the thumb is heading for the far end");
    /// assert_eq!(on.track, toggle.palette().track_on);
    /// assert_ne!(on.track, off.track, "and the pill is a different colour");
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        let checked = self.checked.get();
        let mut track = if checked {
            self.palette.track_on
        } else {
            self.palette.track_off
        };
        let mut thumb = self.palette.thumb;
        if self.disabled.get() {
            track = with_opacity(track, DISABLED_OPACITY);
            thumb = with_opacity(thumb, DISABLED_OPACITY);
        }
        Style {
            position: if checked { 1.0 } else { 0.0 },
            thumb_scale: 1.0,
            track,
            thumb,
            // Applied after the focus rather than instead of it, so a caller
            // that leaves `focused` set on a toggle it has just disabled gets
            // the button's behaviour: a control that cannot be focused does not
            // claim to be.
            ring_width: if self.focused.get() && !self.disabled.get() {
                self.focus_ring.get()
            } else {
                0.0
            },
        }
    }

    /// Applies the appearance the toggle's state and flags imply at once, with
    /// no transition.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a toggle that has just been given a
    /// [`Palette`](Toggle::set_palette) and has never animated — whose colour
    /// properties still hold the neutral defaults [`Toggle::new`] wrote, so
    /// without this a themed toggle starts out grey — and a caller that has
    /// written a state property itself and wants the toggle to be that state now.
    ///
    /// It cancels the bounce as well as the slide, so a snap is not a
    /// transition in either direction: the thumb is left at its own size.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::toggle::{Palette, Toggle};
    ///
    /// let mut nodes = Arena::new();
    /// let mut toggle = Toggle::new(&mut nodes);
    /// let themed = Palette::from_theme(&Theme::dark());
    /// toggle.set_palette(themed);
    ///
    /// // Aiming alone would leave the toggle grey until the transition ran.
    /// toggle.snap_to_state();
    /// assert_eq!(toggle.track.get(), themed.track_off);
    ///
    /// // And a snap is not a transition, so nothing is left running.
    /// assert!(!toggle.is_animating());
    /// ```
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.bounce_clock.borrow_mut().clear();
        self.position.set(style.position);
        self.thumb_scale.set(style.thumb_scale);
        self.track.set(style.track);
        self.thumb.set(style.thumb);
        self.aimed.set(self.checked.get());
    }

    /// Starts the transitions that carry the toggle from wherever it is toward
    /// the appearance [`style`](Toggle::style) implies, on `motion`, and plays
    /// the bounce if the state has moved since the last aim.
    ///
    /// The slide, the colour crossfade and the bounce all run on the motion the
    /// caller hands over — the theme's `DurationFast` and `EasingStandard` — so
    /// a toggle's transition is the theme's transition and not a number written
    /// here.
    ///
    /// The toggle's own clocks are cleared first, so the transitions this
    /// replaces stop where they are rather than writing over the new ones when
    /// they arrive, and so a caller that aims on every frame does not leave one
    /// animation per aim behind on a clock that keeps them all.
    ///
    /// What the clear is *not* doing is deciding which of two transitions wins: a
    /// clock writes its animations in the order it holds them, so the newest
    /// would be written last and would win either way. A deliberate break of
    /// this `clear` leaves every test in the module green, which is why this
    /// paragraph says "keeps the clock from accumulating" and not "is what makes
    /// the replacement correct" — the latter is a claim about
    /// [`AnimationClock`]'s write order that belongs to `animation.rs`, and it is
    /// the slider's own doc that asserts the stronger version without evidence.
    ///
    /// The target is a snapshot, not a continuous one: a caller re-aims when its
    /// own state moves, exactly as the demo re-aims a button's press when the
    /// theme moves under it. Re-aiming without a state change plays no second
    /// bounce, which is what lets a caller aim on every frame; it does restart
    /// the slide from wherever the thumb is, so a caller that aims on every
    /// frame sees a transition that *approaches* its target rather than one that
    /// runs to its end — [`Slider::animate_to_state`](crate::widgets::slider::Slider::animate_to_state)
    /// has the same property, for the same reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::toggle::Toggle;
    ///
    /// let mut nodes = Arena::new();
    /// let toggle = Toggle::new(&mut nodes);
    /// toggle.checked.set(true);
    /// toggle.animate_to_state(Motion {
    ///     duration: Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    ///
    /// assert_eq!(toggle.position.get(), 0.0, "it starts where it was");
    /// assert_eq!(toggle.thumb_scale.get(), 1.25, "and the thumb has bounced");
    /// toggle.tick(Duration::from_millis(50));
    /// assert_eq!(toggle.position.get(), 0.5, "half way across");
    /// ```
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        if self.checked.get() != self.aimed.get() {
            self.aimed.set(self.checked.get());
            self.bounce(motion);
        }
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.position
                .animate_to(style.position, motion.duration, motion.easing),
        );
        clock.add(
            self.track
                .animate_to(style.track, motion.duration, motion.easing),
        );
        clock.add(
            self.thumb
                .animate_to(style.thumb, motion.duration, motion.easing),
        );
    }

    /// Advances the toggle's transitions by `delta`, and returns whether any of
    /// them wrote.
    ///
    /// It is the toggle's frame integration: call it once a frame, before the
    /// paint pass, with the time that frame took. The write is what reaches the
    /// node — a property callback registered by the caller marks the node dirty
    /// — so a caller that repaints only when this is true repaints exactly while
    /// something moves.
    ///
    /// Both of the toggle's clocks are advanced, and both are advanced even when
    /// the first one wrote: a bounce can be running on its own, and a toggle
    /// whose slide has arrived and whose thumb is still settling is still
    /// moving.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        let sliding = self.clock.borrow_mut().tick(delta);
        let bouncing = self.bounce_clock.borrow_mut().tick(delta);
        sliding || bouncing
    }

    /// Returns whether any of the toggle's transitions is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating() || self.bounce_clock.borrow().is_animating()
    }

    /// Returns the point the thumb's centre is drawn at inside `rect`.
    ///
    /// It follows the *drawn* position, not [`checked`](Toggle::checked), so a
    /// caller asking mid-transition is told where the thumb is rather than where
    /// it is going — which is the only version of the question a test about
    /// animation can ask.
    ///
    /// The two ends are a radius in from the pill's own ends, so the thumb stays
    /// inside the pill at both extremes; a pill too narrow to hold its thumb has
    /// nowhere for it to travel, and both ends are then the pill's own centre.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::toggle::Toggle;
    ///
    /// let mut nodes = Arena::new();
    /// let toggle = Toggle::new(&mut nodes);
    /// let rect = Rect::new(0.0, 0.0, 48.0, 44.0);
    ///
    /// assert_eq!(toggle.thumb_center(rect), (12.0, 22.0), "at the off end");
    ///
    /// toggle.position.set(1.0);
    /// assert_eq!(
    ///     toggle.thumb_center(rect),
    ///     (36.0, 22.0),
    ///     "and at the on end, a radius in"
    /// );
    /// ```
    #[must_use]
    pub fn thumb_center(&self, rect: Rect) -> (f32, f32) {
        let track = self.track_rect(rect);
        let (from, to) = self.travel(track);
        let along = from + (to - from) * bounded(self.position.get());
        (along, track.y + track.height / 2.0)
    }

    /// Handles `event` as this toggle would inside `rect`, and reports whether
    /// it consumed it.
    ///
    /// A [`Tap`](InputEventKind::Tap) switches the state and is consumed, which
    /// is requirement 3's "tap/click toggles the state" and "toggle consumes tap
    /// events" both being true. A tap over the toggle's own rect is the only one
    /// it acts on: a tap outside it belongs to whatever is behind the toggle, and
    /// letting it through is what "consumes tap events" is for. The rect here is
    /// the one the toggle paints in, so "over the toggle" means the same thing
    /// in both.
    ///
    /// An activation key — Enter, the keypad's Enter, Space, or the gamepad's
    /// south button — switches the state and is consumed, but only while the
    /// toggle holds focus. A key press is not routed by position, so this is
    /// called on the focused node by the caller, and an unfocused toggle must
    /// not answer one.
    ///
    /// A disabled toggle consumes both and switches nothing. The event was aimed
    /// at it, and letting a tap on an inert control reach the panel behind it
    /// would be the one behaviour a disabled control must not have; this is the
    /// button's rule, for the same reason.
    ///
    /// Every other event is left alone and not consumed, so it carries on up the
    /// tree. That includes a [`Drag`](InputEventKind::Drag) across the toggle and
    /// a [`LongPress`](InputEventKind::LongPress) on it: a toggle has no
    /// continuous value for a drag to change, and the task names no long press.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind, Key};
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::toggle::Toggle;
    ///
    /// let mut nodes = Arena::new();
    /// let toggle = Toggle::new(&mut nodes);
    /// let rect = Rect::new(0.0, 0.0, 48.0, 44.0);
    ///
    /// // Enter only reaches a focused toggle.
    /// let mut enter = InputEvent::new(
    ///     InputEventKind::KeyDown {
    ///         key: Key::Keyboard(sdl3::keyboard::Keycode::Return),
    ///         keymod: sdl3::keyboard::Mod::empty(),
    ///     },
    ///     None,
    /// );
    /// assert!(!toggle.on_event(&mut enter, rect), "an unfocused toggle ignores it");
    /// assert!(!enter.consumed(), "and lets it travel on");
    ///
    /// toggle.focused.set(true);
    /// assert!(toggle.on_event(&mut enter, rect));
    /// assert!(toggle.checked.get());
    /// ```
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        match event.kind() {
            InputEventKind::Tap => {
                if !over(rect, event.position()) {
                    return false;
                }
                event.consume();
                if !self.disabled.get() {
                    self.set_checked(!self.checked.get());
                }
                true
            }
            InputEventKind::KeyDown { key, .. } if self.focused.get() && is_toggle_key(&key) => {
                event.consume();
                if !self.disabled.get() {
                    self.set_checked(!self.checked.get());
                }
                true
            }
            _ => false,
        }
    }

    /// Returns the draw commands that paint the toggle within `rect`.
    ///
    /// The commands are, in order: the focus ring, if the toggle is focused and
    /// the ring is not zero wide; the pill, a rounded rectangle whose radius is
    /// half its own height, which is what makes it a pill; the thumb's shadow,
    /// a circle; and the thumb, a circle on top of it.
    ///
    /// The order is the point of all of it, and two of the four positions are
    /// load bearing. The thumb is above its shadow, so the shadow is a ring and
    /// not a disc. The pill is above the focus ring, so the ring reads as an
    /// outline: a [`DrawCommand::RoundedRect`] *fills* its rect, and a ring drawn
    /// around the toggle's whole node would be a white card with a pill lying on
    /// it. The ring is drawn around the **pill** and then covered by it, which is
    /// the trick [`Button`](crate::widgets::button::Button) uses to draw an
    /// outline at all: put the bigger shape down, put the real one over it, and
    /// only the border is left. That is the third defect in this repository
    /// found only by looking at the screen, and the test for it asserts the pair
    /// — the grown shape *and* the shape covering it recorded after it.
    ///
    /// Every size here comes from the drawn position, so a toggle mid-transition
    /// paints its thumb where the transition has got to and its pill in the
    /// colour the transition has reached.
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let track = self.track_rect(rect);
        let radius = track.height / 2.0;
        let ring = self.focus_ring.get();
        if ring > 0.0 && self.focused.get() {
            painter.rounded_rect(grow(track, ring), radius + ring, self.palette.ring);
        }
        painter.rounded_rect(track, radius, self.track.get());

        let center = self.thumb_center(rect);
        let thumb_radius = (THUMB_RADIUS * self.thumb_scale.get()).max(0.0);
        painter.circle(
            center,
            thumb_radius + THUMB_SHADOW_INSET,
            with_opacity(Color::new(0, 0, 0, 255), THUMB_SHADOW_OPACITY),
        );
        painter.circle(center, thumb_radius, self.thumb.get());
        painter.finish()
    }

    /// Returns the pill's rounded rectangle inside `rect`.
    ///
    /// The pill is the constants above and it is *centred* in `rect`, not
    /// stretched to fill it: a toggle whose node is 200 pixels wide is a 48-pixel
    /// pill in a 200-pixel row, and a pill stretched to the node's width would
    /// have a 24-pixel thumb at either end of a long empty run. It is capped at
    /// the node's own size in each direction, so a node too small for the pill
    /// gets a pill that fits rather than one hanging over both its edges.
    ///
    /// The two caps are two comparisons rather than [`f32::clamp`], for the
    /// reason [`bounded`] gives: `clamp` panics when its bounds are the wrong way
    /// round, and a rect a caller has got the wrong way up must not take a frame
    /// down to say so.
    #[allow(clippy::manual_clamp)]
    fn track_rect(&self, rect: Rect) -> Rect {
        let width = rect.width.max(0.0).min(TRACK_WIDTH);
        let height = rect.height.max(0.0).min(TRACK_HEIGHT);
        Rect::new(
            rect.x + (rect.width - width) / 2.0,
            rect.y + (rect.height - height) / 2.0,
            width,
            height,
        )
    }

    /// Returns the first and last points the thumb's centre may occupy along the
    /// pill `track` runs through.
    ///
    /// The two are a radius in from each end, so the thumb stays inside the pill
    /// at both extremes and is flush with it at each one.
    ///
    /// A pill too short to hold its thumb — with less than twice the radius to
    /// move through — has nowhere for the thumb to travel, and its two ends would
    /// be the wrong way round, which would turn a position into a place off the
    /// far side of the pill. Both ends are then the pill's own centre, so the
    /// thumb sits still rather than escaping.
    fn travel(&self, track: Rect) -> (f32, f32) {
        // `track.width` is a length, so the travel is that less a radius at each
        // end. It used to subtract the pill's own origin as well, which every
        // test in the slider's module missed because every rect there started at
        // the origin: a control laid out anywhere else in the window reported a
        // negative run and pinned its two ends to its own centre. The rule that
        // came out of it is in `.ai/NEVERAGAIN.md`, and the fixture that follows
        // it is `a_toggle_away_from_the_origin_puts_its_whole_geometry_elsewhere`.
        let run = track.width - THUMB_RADIUS * 2.0;
        if run <= 0.0 {
            let middle = track.x + track.width / 2.0;
            return (middle, middle);
        }
        (track.x + THUMB_RADIUS, track.x + THUMB_RADIUS + run)
    }

    /// Writes the state and reports it, and nothing else.
    ///
    /// The appearance is the caller's to move: this writes the state at once and
    /// leaves [`position`](Toggle::position) where the last transition left it,
    /// so the slide is a transition rather than a jump. A caller that wants the
    /// toggle to *be* the new state at once asks for
    /// [`snap_to_state`](Toggle::snap_to_state).
    fn set_checked(&self, checked: bool) {
        self.checked.set(checked);
        self.on_change.call(checked);
    }

    /// Puts the thumb at [`THUMB_BOUNCE_SCALE`] and eases it back to its own
    /// size, on `motion`.
    ///
    /// The bounce clock is cleared first, so the clock holds one bounce rather
    /// than one per toggle, and so the thumb pops from the cap every time
    /// instead of from wherever the last bounce had got to. As in
    /// [`animate_to_state`](Toggle::animate_to_state), the clear is not what
    /// decides the value: the clock writes in the order it holds its animations,
    /// so the newest is written last either way.
    fn bounce(&self, motion: Motion) {
        self.bounce_clock.borrow_mut().clear();
        self.bounce_clock
            .borrow_mut()
            .add(self.thumb_scale.animate_from_to(
                THUMB_BOUNCE_SCALE,
                1.0,
                motion.duration,
                motion.easing,
            ));
    }
}

/// Returns `value` in `0.0..=1.0`.
///
/// The two comparisons rather than [`f32::clamp`], and for the reason the
/// slider's own copy of this function gives: `clamp` panics when its bounds are
/// the wrong way round, and a position a caller has got adrift in must not take
/// a frame down to say so. `max` then `min` is the same answer for an ordered
/// pair and cannot panic, and a `NaN` is passed over rather than propagated —
/// which is what the test on a `NaN` position is for.
#[allow(clippy::manual_clamp)]
fn bounded(value: f32) -> f32 {
    value.max(0.0).min(1.0)
}

/// Returns whether a tap at `position` is over `rect`.
///
/// `rect` is the rect the toggle paints in, so this is the same question
/// "which node is under the pointer" asks — decided by the widget rather than by
/// [`input::hit_test`](crate::input::hit_test), because a caller that hands
/// [`Toggle::on_event`] an event has already decided this and a caller that has
/// not should not be trusted with a toggle that switches from across the window.
///
/// Two cases answer "yes" whatever the position: a rect with no extent, because a
/// caller that has not laid the toggle out has no rect to be outside of and a
/// toggle with no rect is a toggle with nowhere to *not* be; and a position
/// there is none, because there is nothing to test it against.
fn over(rect: Rect, position: Option<Offset>) -> bool {
    let Some(position) = position else {
        return true;
    };
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return true;
    }
    position.x >= rect.x
        && position.x <= rect.x + rect.width
        && position.y >= rect.y
        && position.y <= rect.y + rect.height
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

/// Returns `color` drawn at `opacity`.
///
/// Interpolating toward transparent black *is* the premultiplied fade: every
/// channel arrives at zero with the alpha, so the colour never bleeds what is
/// under it. `opacity` is clamped, because a caller that has lost track of it
/// cannot ask for a colour outside the segment. It is the button's own helper,
/// repeated rather than imported: it is eight lines, and a shared module for one
/// eight-line helper is a module.
fn with_opacity(color: Color, opacity: f32) -> Color {
    Color::interpolate(
        &color,
        &Color::new(0, 0, 0, 0),
        (1.0 - opacity).clamp(0.0, 1.0),
    )
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback
/// for a token a caller has written the wrong variant into — and black rather
/// than a panic, because a mistyped theme token is not worth taking a frame down
/// for. It is the slider's own helper, repeated rather than imported, for the
/// reason [`with_opacity`] gives.
fn token_color(theme: &Theme, token: crate::theme::ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

/// Returns whether `key` switches the focused toggle.
///
/// The gamepad's south button is the one an A-capable controller puts its
/// confirm face button on: SDL names the four face buttons by position
/// (`North`, `East`, `South`, `West`) and has no `A` — the `A` is a
/// [`ButtonLabel`](sdl3::gamepad::ButtonLabel), a live query about the
/// controller that is connected, which an event cannot carry. So "the gamepad's A
/// button" is [`South`](sdl3::gamepad::Button::South) here, which is the button
/// the button widget's own activation keys name.
///
/// The keys are the button's, because a toggle and a button are the same
/// control with two positions: both switch on Enter and on Space, and a caller
/// should not have to learn which of them took which.
fn is_toggle_key(key: &Key) -> bool {
    matches!(
        key,
        Key::Keyboard(sdl3::keyboard::Keycode::Return | sdl3::keyboard::Keycode::KpEnter)
            | Key::Keyboard(sdl3::keyboard::Keycode::Space)
            | Key::Gamepad(sdl3::gamepad::Button::South)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Easing;
    use crate::layout::{Constraints, Layout, LayoutState};
    use crate::theme::ThemeToken;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The rect most of the geometry tests lay a toggle out in: its own size, so
    /// the pill fills it and the two ends of the travel are 12 and 36.
    const RECT: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 48.0,
        height: 44.0,
    };

    /// The same toggle somewhere else in a window: 48 by 44 at (664, 496). Every
    /// other fixture here starts at the origin, so this is the one that catches
    /// an origin being read as an extent — the defect the slider's module shipped
    /// past 53 green tests, and the reason this fixture exists.
    const OFFSET_RECT: Rect = Rect {
        x: 664.0,
        y: 496.0,
        width: 48.0,
        height: 44.0,
    };

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// The scale above which a bounce stops being the "slight" one the task asks
    /// for: a third again, where a quarter is a pop and a half is a gaggle.
    ///
    /// It lives here rather than beside [`THUMB_BOUNCE_SCALE`] on purpose. A cap
    /// the production code clamps to would make the test that uses it pass with
    /// any scale at all — the clamp would do the work the assertion exists to
    /// do — and this file's job is to fail when the effect grows.
    const SLIGHT_CEILING: f32 = 1.3;

    /// Advances `toggle`'s transitions by `millis` and reports whether anything
    /// moved.
    ///
    /// The toggle's own `tick` is what a frame calls, and it answers whether a
    /// repaint is needed; a test that only wants time to pass goes through here
    /// so that the answer is passed on rather than dropped on the floor.
    fn tick(toggle: &Toggle, millis: u64) -> bool {
        toggle.tick(ms(millis))
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a
    /// value at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// A toggle in the dark theme's palette, with its appearance already arrived
    /// at that palette.
    ///
    /// Setting a palette does not move the toggle by itself — a theme switch is
    /// animated — so a test that wants a toggle *in* a palette asks for the snap.
    /// Otherwise the colours would be the ones `Toggle::new` starts with and the
    /// palette's would only be targets.
    fn toggle() -> (Arena<WidgetNode>, Toggle) {
        let mut nodes = Arena::new();
        let mut toggle = Toggle::new(&mut nodes);
        toggle.set_palette(Palette::from_theme(&Theme::dark()));
        toggle.snap_to_state();
        (nodes, toggle)
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
    /// The filtering helpers above cannot see order, and the order is most of
    /// what a toggle is: the pill has to be drawn over the focus ring and the
    /// thumb over its shadow. **This helper sees shapes and not sizes**, so it
    /// cannot tell the thumb's shadow from the thumb itself — two `Circle`s are
    /// one word to it, and swapping the two leaves its answer unchanged. The
    /// half of the order that is *within* a pair of the same shape is asserted by
    /// radius instead, in the two tests named for it.
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
    /// The travel and the slide are computed in `f32`, so a midpoint is a
    /// fraction of a pixel: a test that wrote the decimal out would be asserting
    /// the compiler's rounding rather than the widget. Everything a toggle
    /// computes exactly — a whole number of the two ends, a half of a colour
    /// channel that is an even number apart — is asserted with `assert_eq!`.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-5, "{what}: {got} against {want}");
    }

    /// A tap at `(x, y)`.
    fn tap_at(x: f32, y: f32) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)))
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

    /// A key release of `key`, which carries no position.
    fn key_up(key: Key) -> InputEvent {
        InputEvent::new(
            InputEventKind::KeyUp {
                key,
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        )
    }

    /// A counter of the states the change callback was given, in order.
    fn changes(toggle: &mut Toggle) -> Rc<RefCell<Vec<bool>>> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorded = Rc::clone(&seen);
        toggle.on_change = Callback::from_fn(move |checked| recorded.borrow_mut().push(checked));
        seen
    }

    /// Hangs `toggle` on a panel and lays it out, and returns the panel's handle
    /// and the rect its node was placed at.
    ///
    /// The panel is a stack, so the toggle sits at its origin, and the toggle's
    /// own node is given the rect its touch target asks for. A tap therefore has
    /// a real hit test to pass or fail, and a node behind the toggle to reach if
    /// it does.
    fn on_a_panel(
        toggle: &Toggle,
        nodes: &mut Arena<WidgetNode>,
        panel_size: Size,
    ) -> (Handle, Rect) {
        let panel = node::create(nodes, LayoutState::new());
        assert!(
            node::attach(nodes, panel, toggle.handle()),
            "the toggle is hung on the panel"
        );
        let size = toggle.size();
        if let Some(node) = nodes.get_mut(toggle.handle()) {
            node.layout_mut().set_constraints(Constraints::tight(size));
        }
        Layout::new(nodes).layout(panel, Constraints::tight(panel_size));
        let placed = nodes
            .get(toggle.handle())
            .and_then(|node| node.layout().rect())
            .map(|rect| {
                Rect::new(
                    rect.origin.x,
                    rect.origin.y,
                    rect.size.width,
                    rect.size.height,
                )
            })
            .unwrap_or(Rect::new(0.0, 0.0, size.width, size.height));
        (panel, placed)
    }

    #[test]
    fn a_toggle_holds_the_properties_the_task_gives_it() {
        let (_nodes, toggle) = toggle();
        assert!(!toggle.checked.get(), "a toggle starts off");
        assert_eq!(toggle.position.get(), 0.0, "with the thumb at the off end");
        assert_eq!(toggle.thumb_scale.get(), 1.0, "and at its own size");
        assert_eq!(
            toggle.track.get(),
            Palette::from_theme(&Theme::dark()).track_off,
            "the pill in the muted colour"
        );
        assert_eq!(
            toggle.thumb.get(),
            Palette::from_theme(&Theme::dark()).thumb
        );
        assert_eq!(toggle.focus_ring.get(), FOCUS_RING);
        assert!(!toggle.focused.get());
        assert!(!toggle.disabled.get());
        assert!(
            !toggle.on_change.is_set(),
            "and no callback until one is given"
        );
    }

    #[test]
    fn a_toggle_has_a_node_of_its_own_in_the_arena() {
        let (nodes, toggle) = toggle();
        assert!(nodes.get(toggle.handle()).is_some());
    }

    #[test]
    fn a_new_toggle_starts_animated_at_the_off_end() {
        // Built from `new` rather than through the `toggle` helper, which snaps.
        let mut nodes = Arena::new();
        let toggle = Toggle::new(&mut nodes);
        assert_eq!(toggle.style().position, 0.0);
        assert_eq!(toggle.thumb_center(RECT), (12.0, 22.0));
        assert!(!toggle.is_animating(), "and nothing is left running");
    }

    #[test]
    fn the_default_geometry_is_the_numbers_this_module_documents() {
        // These are constants, not derived numbers, and the task's "pill shape",
        // "circle that slides" and "slight scale bounce" all hang off them.
        assert_eq!(TRACK_WIDTH, 48.0, "two thumb diameters and nothing spare");
        assert_eq!(TRACK_HEIGHT, 28.0, "half as wide again, so it is a pill");
        assert_eq!(THUMB_RADIUS, 12.0, "and a thumb that fits inside it");
        assert_eq!(THUMB_SHADOW_INSET, 2.0);
        assert_eq!(FOCUS_RING, 2.0);
        assert_eq!(MIN_TOUCH_TARGET, 44.0, "the floor a finger can hit");
    }

    #[test]
    fn the_bounce_is_capped_at_slight() {
        // The task says "slight scale bounce", and *slight* is the word that has
        // to become a number before anything can be tested about it. This is the
        // assertion the button's press overlay is given for the same reason, and
        // it is against the radius that is *painted* rather than against the
        // constant: a constant compared with a constant is a build-time check,
        // and what a caller sees is a circle.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        let painted = circles(&toggle.paint(RECT))
            .last()
            .map(|(_, radius, _)| *radius)
            .unwrap_or_default();
        assert!(
            painted <= THUMB_RADIUS * SLIGHT_CEILING,
            "the thumb never grows past a third again: {painted}"
        );
        assert!(painted > THUMB_RADIUS, "and it did grow rather than shrink");
    }

    #[test]
    fn the_track_is_a_pill_centred_in_the_toggles_rect() {
        let (_nodes, toggle) = toggle();
        let pill = rounded(&toggle.paint(RECT));
        assert_eq!(pill.len(), 1, "one pill, and nothing else is a rect");
        assert_eq!(
            pill[0].0,
            Rect::new(0.0, 8.0, 48.0, 28.0),
            "48 by 28, centred in a 48 by 44 node"
        );
        assert_eq!(
            pill[0].1, 14.0,
            "with half its own height as radius, which is what makes it a pill"
        );
    }

    #[test]
    fn the_pill_is_the_constants_size_in_a_node_bigger_than_it() {
        // The fixture above is *exactly* the pill's width, which is a blind spot
        // this mutation testing found: with a 48-wide node, "the pill is 48 wide
        // and centred" and "the pill is stretched to the node" draw the same
        // rect, and deleting the cap passed every assertion in the module. A
        // node bigger than the pill is what tells them apart — the pill is
        // centred in 200 by 120, so its left edge is 76 and its top 46.
        let (_nodes, toggle) = toggle();
        let roomy = Rect::new(0.0, 0.0, 200.0, 120.0);
        assert_eq!(
            rounded(&toggle.paint(roomy))[0].0,
            Rect::new(76.0, 46.0, 48.0, 28.0),
            "48 by 28 in the middle of a 200 by 120 node, not 200 by 120"
        );
        assert_eq!(
            toggle.thumb_center(roomy),
            (88.0, 60.0),
            "and the travel is the pill's, not the node's"
        );
    }

    #[test]
    fn the_thumb_travels_a_radius_in_from_each_end_of_the_pill() {
        let (_nodes, toggle) = toggle();
        assert_eq!(
            toggle.thumb_center(RECT),
            (12.0, 22.0),
            "at the off end, a radius in, on the pill's own middle line"
        );

        toggle.position.set(1.0);
        assert_eq!(
            toggle.thumb_center(RECT),
            (36.0, 22.0),
            "and at the on end, a radius in from the other one"
        );
    }

    #[test]
    fn a_position_written_by_hand_moves_the_thumb_to_that_part_of_the_pill() {
        // The travel is 48 less two radii, so half of it is 12: 12 in from the
        // left end is 24.
        let (_nodes, toggle) = toggle();
        toggle.position.set(0.5);
        assert_eq!(toggle.thumb_center(RECT), (24.0, 22.0));
    }

    #[test]
    fn a_position_outside_zero_to_one_is_clamped_to_the_end_it_is_past() {
        // The clamp is inside `thumb_center` rather than inside every writer,
        // which is why nothing else in this module could see its absence: a
        // caller that writes the property is the only way to reach it.
        let (_nodes, toggle) = toggle();
        toggle.position.set(5.0);
        assert_eq!(toggle.thumb_center(RECT).0, 36.0, "past the top is the top");
        toggle.position.set(-4.0);
        assert_eq!(toggle.thumb_center(RECT).0, 12.0, "and below it is the end");
        toggle.position.set(f32::NAN);
        assert_eq!(
            toggle.thumb_center(RECT).0,
            12.0,
            "a position that has come adrift is no position at all"
        );
    }

    #[test]
    fn a_toggle_away_from_the_origin_puts_its_whole_geometry_elsewhere() {
        // Every other geometry fixture here is at (0, 0), which is a blind spot:
        // a rect's origin and a rect's extent are different numbers, and only a
        // toggle somewhere else on the window tells them apart. This is the
        // fixture the slider's module was missing, and the defect it would have
        // caught was in `travel`.
        let (_nodes, toggle) = toggle();
        let rect = OFFSET_RECT;
        assert_eq!(rect.x, 664.0, "the fixture really is away from the origin");

        assert_eq!(
            rounded(&toggle.paint(rect))[0].0,
            Rect::new(664.0, 504.0, 48.0, 28.0),
            "the pill is 8 down from the node's own top edge, not 8 from the window's"
        );
        assert_eq!(
            toggle.thumb_center(rect),
            (676.0, 518.0),
            "the thumb is a radius in from the pill's left end, wherever that is"
        );

        toggle.position.set(1.0);
        assert_eq!(
            toggle.thumb_center(rect).0,
            700.0,
            "and a radius in from its right one, which is not 36"
        );
    }

    #[test]
    fn a_pill_too_narrow_to_hold_its_thumb_holds_the_thumb_still() {
        // Both ends would otherwise be the wrong way round, and a position would
        // turn into a place off the far side of the pill.
        let (_nodes, toggle) = toggle();
        let narrow = Rect::new(0.0, 0.0, 20.0, 44.0);
        assert_eq!(
            rounded(&toggle.paint(narrow))[0].0,
            Rect::new(0.0, 8.0, 20.0, 28.0),
            "the pill is capped at the node's own width"
        );
        assert_eq!(
            toggle.thumb_center(narrow),
            (10.0, 22.0),
            "the pill's centre"
        );

        toggle.position.set(1.0);
        assert_eq!(
            toggle.thumb_center(narrow),
            (10.0, 22.0),
            "and it stays there at every position"
        );
    }

    #[test]
    fn a_tap_switches_the_state_and_is_consumed() {
        let (_nodes, toggle) = toggle();
        let mut tap = tap_at(24.0, 22.0);
        assert!(toggle.on_event(&mut tap, RECT));
        assert!(toggle.checked.get(), "so the toggle is on");
        assert!(tap.consumed(), "and the tap does not travel on");
    }

    #[test]
    fn a_tap_switches_it_back_again() {
        let (_nodes, toggle) = toggle();
        for want in [true, false, true] {
            let mut tap = tap_at(4.0, 4.0);
            assert!(toggle.on_event(&mut tap, RECT));
            assert_eq!(toggle.checked.get(), want, "each tap flips it");
        }
    }

    #[test]
    fn a_tap_is_answered_anywhere_in_the_toggles_own_rect() {
        // A toggle is a switch: the whole of it is the target, which is why the
        // position is not read. The corners are the case that says so — the
        // edges are inside the test, because the rect holds its edges.
        let (_nodes, toggle) = toggle();
        for (index, (x, y)) in [(0.0, 0.0), (48.0, 44.0), (0.0, 44.0), (24.0, 22.0)]
            .into_iter()
            .enumerate()
        {
            let mut tap = tap_at(x, y);
            assert!(toggle.on_event(&mut tap, RECT), "at {x},{y}");
            assert!(tap.consumed());
            assert_eq!(
                toggle.checked.get(),
                index % 2 == 0,
                "and each of the four taps flipped it"
            );
        }
        assert!(!toggle.checked.get(), "four flips is two pairs");
    }

    #[test]
    fn a_tap_outside_the_toggles_own_rect_is_not_the_toggles_to_consume() {
        // The other half of "toggle consumes tap events": a tap that misses the
        // toggle's rect belongs to whatever is behind it.
        let (_nodes, toggle) = toggle();
        let mut tap = tap_at(300.0, 22.0);
        assert!(!toggle.on_event(&mut tap, RECT));
        assert!(!tap.consumed(), "so it carries on to the panel");
        assert!(!toggle.checked.get(), "and the toggle never switched");
    }

    #[test]
    fn a_toggle_with_no_rect_of_its_own_answers_any_tap() {
        // A caller that has not laid the toggle out has no rect to be outside
        // of, and a toggle that refuses every tap is a dead widget with no
        // symptom. This is the case that keeps `over` from being a footgun.
        let (_nodes, toggle) = toggle();
        let mut tap = tap_at(940.0, 500.0);
        assert!(toggle.on_event(&mut tap, Rect::new(0.0, 0.0, 0.0, 0.0)));
        assert!(toggle.checked.get());
    }

    #[test]
    fn a_tap_with_no_position_is_the_toggles_too() {
        // There is nothing to test a position against, so it is over.
        let (_nodes, toggle) = toggle();
        let mut tap = InputEvent::new(InputEventKind::Tap, None);
        assert!(toggle.on_event(&mut tap, RECT));
        assert!(toggle.checked.get());
    }

    #[test]
    fn a_toggle_reached_through_the_dispatcher_switches_and_the_panel_does_not_see_it() {
        let (mut nodes, mut toggle) = toggle();
        let seen = changes(&mut toggle);
        let (panel, placed) = on_a_panel(&toggle, &mut nodes, Size::new(400.0, 200.0));

        let mut event = tap_at(24.0, 22.0);
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == toggle.handle() {
                toggle.on_event(event, placed);
            }
        });
        assert_eq!(reached, vec![toggle.handle()], "only the toggle saw it");
        assert_eq!(
            seen.borrow().as_slice(),
            &[true],
            "and it reported the new state"
        );
    }

    #[test]
    fn a_tap_outside_the_toggle_reaches_the_panel_behind_it() {
        let (mut nodes, mut toggle) = toggle();
        let seen = changes(&mut toggle);
        let (panel, placed) = on_a_panel(&toggle, &mut nodes, Size::new(400.0, 200.0));

        let mut event = tap_at(300.0, 150.0);
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == toggle.handle() {
                toggle.on_event(event, placed);
            }
        });
        assert_eq!(reached, vec![panel], "the panel took it");
        assert!(!toggle.checked.get(), "and the toggle never switched");
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn a_tap_switches_the_state_and_leaves_the_slide_to_the_caller() {
        // The tap is the state change and nothing more: the thumb is *drawn* from
        // `position`, and a tap does not write it. A tap that moved the thumb
        // itself would leave a caller that never aims with a toggle whose state
        // and whose thumb disagree for ever.
        let (_nodes, toggle) = toggle();
        let mut tap = tap_at(24.0, 22.0);
        assert!(toggle.on_event(&mut tap, RECT));
        assert!(toggle.checked.get(), "the state is on at once");
        assert_eq!(
            toggle.position.get(),
            0.0,
            "and the thumb has not moved, because the slide is the caller's"
        );

        toggle.animate_to_state(motion());
        tick(&toggle, 50);
        assert_eq!(
            toggle.position.get(),
            0.5,
            "which is what carries it, exactly as for a switch set in code"
        );
    }

    #[test]
    fn the_change_callback_reports_the_state_it_moved_to() {
        // The callback carries the state, which is the whole reason the callback
        // moved: a caller writing a setting needs to know whether it is on or off.
        let (_nodes, mut toggle) = toggle();
        let seen = changes(&mut toggle);
        for _ in 0..3 {
            let mut tap = tap_at(24.0, 22.0);
            toggle.on_event(&mut tap, RECT);
        }
        assert_eq!(seen.borrow().as_slice(), &[true, false, true]);
        assert_eq!(*seen.borrow().last().unwrap(), toggle.checked.get());
    }

    #[test]
    fn a_toggle_with_no_change_callback_still_switches() {
        // A caller that has given it nothing to report to is an ordinary caller.
        let (_nodes, toggle) = toggle();
        assert!(!toggle.on_change.is_set());
        let mut tap = tap_at(24.0, 22.0);
        assert!(toggle.on_event(&mut tap, RECT));
        assert!(toggle.checked.get());
    }

    #[test]
    fn enter_switches_a_focused_toggle() {
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        let mut enter = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(toggle.on_event(&mut enter, RECT));
        assert!(toggle.checked.get());
        assert!(enter.consumed());
    }

    #[test]
    fn the_keypads_enter_and_space_switch_a_focused_toggle() {
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        for code in [
            sdl3::keyboard::Keycode::KpEnter,
            sdl3::keyboard::Keycode::Space,
        ] {
            let mut event = key_down(Key::Keyboard(code));
            assert!(
                toggle.on_event(&mut event, RECT),
                "{code:?} is an activation key"
            );
            assert!(event.consumed());
        }
    }

    #[test]
    fn the_gamepads_south_button_switches_a_focused_toggle() {
        // The task says "gamepad: A button". SDL's `Button` has no `A` — the
        // face buttons are named by position and `A` is a `ButtonLabel`, a live
        // query about the controller that is connected, which an event cannot
        // carry — so the button an A-labelled controller puts its confirm face
        // on is `South`, which is what the button widget's keys name too.
        use sdl3::gamepad::Button as Pad;
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        let mut press = key_down(Key::Gamepad(Pad::South));
        assert!(toggle.on_event(&mut press, RECT), "the south face button");
        assert!(toggle.checked.get());
        assert!(press.consumed());
    }

    #[test]
    fn the_d_pad_and_the_other_face_buttons_are_not_a_toggles_keys() {
        use sdl3::gamepad::Button as Pad;
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        for pad in [
            Pad::DPadRight,
            Pad::DPadLeft,
            Pad::East,
            Pad::West,
            Pad::North,
        ] {
            let mut event = key_down(Key::Gamepad(pad));
            assert!(!toggle.on_event(&mut event, RECT), "{pad:?} is not one");
            assert!(!event.consumed(), "{pad:?} carries on to the tree");
        }
        assert!(!toggle.checked.get(), "so nothing switched");
    }

    #[test]
    fn a_key_does_nothing_to_an_unfocused_toggle() {
        // A key press is not routed by position, so it reaches the toggle only
        // because the caller sent it to the focused node. Every toggle on screen
        // would otherwise switch on every Enter.
        let (_nodes, toggle) = toggle();
        let mut enter = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(!toggle.on_event(&mut enter, RECT));
        assert!(!enter.consumed(), "and the key carries on to the tree");
        assert!(!toggle.checked.get());
    }

    #[test]
    fn a_key_the_toggle_does_not_use_is_left_alone() {
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        for key in [
            Key::Keyboard(sdl3::keyboard::Keycode::Tab),
            Key::Keyboard(sdl3::keyboard::Keycode::Escape),
            Key::Keyboard(sdl3::keyboard::Keycode::Left),
        ] {
            let mut event = key_down(key);
            assert!(!toggle.on_event(&mut event, RECT), "{key:?} is not its key");
            assert!(!event.consumed(), "{key:?} carries on to the tree");
        }
        assert!(!toggle.checked.get());
    }

    #[test]
    fn a_key_release_switches_nothing() {
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        let mut release = key_up(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(!toggle.on_event(&mut release, RECT));
        assert!(!release.consumed());
        assert!(!toggle.checked.get(), "only the press toggles");
    }

    #[test]
    fn a_drag_and_a_long_press_across_a_toggle_are_left_for_somebody_else() {
        // A toggle has no continuous value for a drag to change, and the task
        // names no long press, so neither is consumed.
        let (_nodes, toggle) = toggle();
        let mut drag = InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(20.0, 0.0),
            },
            Some(Offset::new(24.0, 22.0)),
        );
        assert!(!toggle.on_event(&mut drag, RECT));
        assert!(!drag.consumed());

        let mut long = InputEvent::new(InputEventKind::LongPress, Some(Offset::new(24.0, 22.0)));
        assert!(!toggle.on_event(&mut long, RECT));
        assert!(!long.consumed());
        assert!(!toggle.checked.get());
    }

    #[test]
    fn a_disabled_toggle_refuses_a_tap_but_still_swallows_it() {
        // The event was aimed at it, and letting a tap on an inert control reach
        // the panel behind it is the one behaviour a disabled control must not
        // have. The button's rule, for the same reason.
        let (_nodes, mut toggle) = toggle();
        let seen = changes(&mut toggle);
        toggle.disabled.set(true);
        let mut tap = tap_at(24.0, 22.0);
        assert!(toggle.on_event(&mut tap, RECT), "the tap is still consumed");
        assert!(tap.consumed());
        assert!(!toggle.checked.get(), "but nothing switched");
        assert!(seen.borrow().is_empty(), "and nothing was reported");
    }

    #[test]
    fn a_disabled_toggle_refuses_a_key_while_focused() {
        let (_nodes, toggle) = toggle();
        toggle.disabled.set(true);
        toggle.focused.set(true);
        let mut enter = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(
            toggle.on_event(&mut enter, RECT),
            "the key is still consumed"
        );
        assert!(!toggle.checked.get(), "but nothing switched");
    }

    #[test]
    fn a_disabled_toggle_still_shows_which_state_it_is_in() {
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.disabled.set(true);
        // Focused as well, on purpose: with only `disabled` set, "the ring is
        // dropped" and "there was no ring to drop" are the same answer, and a
        // deliberate break of the disabled arm of the ring left every test in
        // this module green until this line was here.
        toggle.focused.set(true);
        let style = toggle.style();
        assert_eq!(style.position, 1.0, "the thumb still belongs at the on end");
        assert_eq!(
            style.ring_width, 0.0,
            "and a control that cannot be focused does not claim to be, whatever \
             the caller left in `focused`"
        );
    }

    #[test]
    fn a_disabled_toggle_is_drawn_dimmer() {
        // The button's own opacity, applied in `style` so it is animated by the
        // colour properties and needs no opacity property of its own. 0.45 of
        // black over the primary colour: 255 - 255 * (1 - 0.45) = 114.75, which
        // rounds to 115.
        let (_nodes, toggle) = toggle();
        let enabled = toggle.style().track;
        toggle.disabled.set(true);
        let dimmed = toggle.style().track;
        assert_ne!(dimmed, enabled, "a disabled toggle is not the same colour");
        assert_eq!(dimmed.a, 115, "and it is 0.45 of the way to transparent");
        assert!(dimmed.a < enabled.a, "which is the whole of 'drawn inert'");
    }

    #[test]
    fn a_programmatic_switch_slides_the_thumb_across_the_pill() {
        // The contract is that the thumb *moves* rather than jumping, so this
        // checks the middle of the transition and not only its end: a `set`
        // instead of an animation would pass an end-only assertion.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());

        assert_eq!(
            toggle.position.get(),
            0.0,
            "the thumb starts where it was, so a frame drawn now is right"
        );
        assert!(toggle.tick(ms(50)));
        assert_eq!(
            toggle.position.get(),
            0.5,
            "half way across, on a linear curve"
        );

        assert!(toggle.tick(ms(50)));
        assert_eq!(toggle.position.get(), 1.0, "and it arrives");
        assert!(!toggle.is_animating());
    }

    #[test]
    fn the_checked_state_and_the_drawn_position_disagree_while_it_slides() {
        // The split the whole module is built on: `checked` is the truth and
        // `position` is what is drawn, and the two agree only when the
        // transition has arrived.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        tick(&toggle, 50);
        assert!(toggle.checked.get(), "the state is already on");
        assert_eq!(
            toggle.position.get(),
            0.5,
            "and the thumb is still crossing"
        );
    }

    #[test]
    fn the_track_colour_is_half_way_between_off_and_on_half_way_through() {
        // The dark theme's off track is TextMuted (158, 158, 158) and its on
        // track is Primary (187, 134, 252), so the midpoint of each channel is
        // (172.5, 146, 205) and a channel is rounded to the nearest of the 256
        // it holds: 173, 146, 205.
        let (_nodes, toggle) = toggle();
        let off = toggle.style().track;
        let on = {
            toggle.checked.set(true);
            toggle.style().track
        };
        assert_eq!(
            (off.r, off.g, off.b),
            (158, 158, 158),
            "the theme's TextMuted is the off track"
        );
        assert_eq!(
            (on.r, on.g, on.b),
            (187, 134, 252),
            "and Primary is the on one"
        );

        toggle.snap_to_state();
        toggle.checked.set(false);
        toggle.snap_to_state();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        tick(&toggle, 50);

        let half = toggle.track.get();
        assert_ne!(half, off, "it is not still the off colour");
        assert_ne!(half, on, "nor already the on one");
        assert_eq!(
            (half.r, half.g, half.b),
            (173, 146, 205),
            "but the midpoint"
        );
        assert_eq!(half.a, 255, "and it is still opaque");
    }

    #[test]
    fn the_drawn_thumb_follows_the_transition_rather_than_the_state() {
        // Requirement 4's second half: the thumb has to move with the
        // transition, not jump to the end the transition is travelling towards.
        let (_nodes, toggle) = toggle();
        let at = |toggle: &Toggle| {
            circles(&toggle.paint(RECT))
                .last()
                .map(|(center, _, _)| center.0)
                .unwrap_or_default()
        };
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        let start = at(&toggle);
        tick(&toggle, 25);
        let a_quarter = at(&toggle);
        tick(&toggle, 25);
        let half = at(&toggle);
        tick(&toggle, 50);
        let arrived = at(&toggle);

        assert_eq!(start, 12.0, "at the off end to begin with");
        assert!(
            start < a_quarter && a_quarter < half,
            "and it moves as it goes"
        );
        assert_eq!(half, 24.0, "to the middle of the pill at half way");
        assert_eq!(arrived, 36.0, "and it arrives at the on end");
        assert_eq!(half - start, arrived - half, "linearly");
    }

    #[test]
    fn a_second_switch_replaces_the_first_rather_than_racing_it() {
        // A state written twice in quick succession used to be a race: both
        // transitions wrote the position and the older one arriving last left it
        // short of the state. The toggle's own clock is what rules that out.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        tick(&toggle, 50);
        toggle.checked.set(false);
        toggle.animate_to_state(motion());
        for _ in 0..4 {
            tick(&toggle, 50);
        }
        assert_eq!(
            toggle.position.get(),
            0.0,
            "the thumb ends at the newest state"
        );
        assert!(!toggle.is_animating(), "and nothing is left running");
    }

    #[test]
    fn the_bounce_grows_the_thumb_and_eases_it_back_to_its_own_size() {
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        assert_eq!(
            toggle.thumb_scale.get(),
            THUMB_BOUNCE_SCALE,
            "the frame the state changes is the frame the thumb pops"
        );
        assert!(toggle.tick(ms(50)));
        assert_close(
            toggle.thumb_scale.get(),
            1.125,
            "half way back, which is 1.25 less a quarter of the way",
        );
        assert!(toggle.tick(ms(50)));
        assert_eq!(toggle.thumb_scale.get(), 1.0, "and it settles");
        assert!(!toggle.is_animating());
    }

    #[test]
    fn the_bounce_never_leaves_the_range_it_is_capped_to() {
        // The cap is the assertion the "slight" in the task needs, and it is
        // sampled across the whole bounce rather than at its ends, because a
        // curve that overshoots in the middle is exactly the thing a cap is for.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        for _ in 0..20 {
            let scale = toggle.thumb_scale.get();
            assert!(
                (1.0..=THUMB_BOUNCE_SCALE).contains(&scale),
                "the thumb is never smaller than itself or bigger than the cap: {scale}"
            );
            tick(&toggle, 5);
        }
    }

    #[test]
    fn the_drawn_thumb_is_the_constant_times_the_scale() {
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        let drawn = circles(&toggle.paint(RECT))
            .last()
            .map(|(_, radius, _)| *radius)
            .unwrap_or_default();
        assert_close(drawn, 15.0, "12 of thumb and a quarter again: 1.25 * 12");
        assert!(
            drawn > THUMB_RADIUS,
            "which is a thumb that grew rather than one that shrank"
        );
    }

    #[test]
    fn a_second_switch_replaces_a_running_bounce_rather_than_adding_to_it() {
        // The two clocks are separate, and this is where it shows: the scale is
        // restarted at the cap, because `animate_from_to` puts the property at
        // its `from` before the first tick, while the position is aimed from
        // wherever it already is and so does not move. A toggle switched twice
        // in quick succession therefore pops once, not twice.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        tick(&toggle, 50);
        assert_close(
            toggle.thumb_scale.get(),
            1.125,
            "half way back from the cap",
        );
        assert_close(toggle.position.get(), 0.5, "and half way across the pill");

        toggle.checked.set(false);
        toggle.animate_to_state(motion());
        assert_eq!(
            toggle.thumb_scale.get(),
            THUMB_BOUNCE_SCALE,
            "the bounce is back at the cap, not easing on from 1.125"
        );
        assert_eq!(
            toggle.position.get(),
            0.5,
            "and the slide is aimed from where it was rather than restarted"
        );
    }

    #[test]
    fn aiming_again_without_a_state_change_does_not_replay_the_bounce() {
        // A caller that aims on every frame must get one bounce per toggle, not
        // a thumb that is permanently a quarter too big.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        for _ in 0..4 {
            tick(&toggle, 50);
        }
        assert_eq!(toggle.thumb_scale.get(), 1.0, "the bounce has run out");

        toggle.animate_to_state(motion());
        assert_eq!(
            toggle.thumb_scale.get(),
            1.0,
            "aiming again does not pop the thumb a second time"
        );
    }

    #[test]
    fn a_toggle_aimed_on_every_frame_settles_its_bounce_and_approaches_its_end() {
        // Five frames of aim-and-tick with one state change between them, and the
        // arithmetic is the point. A re-aim restarts the transition from wherever
        // the property is, and each tick covers a fifth of what is left, so the
        // position closes a fifth of the remaining gap every frame:
        // 0.2, then 0.2 + 0.8/5 = 0.36, then 0.36 + 0.64/5 = 0.488, then 0.5904,
        // then 0.67232. So a caller that aims on every frame gets a transition
        // that *approaches* its target rather than one that runs to its end —
        // the slider's own semantics — and the test states the number instead of
        // guessing at an arrival.
        //
        // What has to be exact is the bounce: a thumb that stayed a quarter too
        // big for as long as the caller kept aiming would be the defect.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        for _ in 0..5 {
            toggle.animate_to_state(motion());
            tick(&toggle, 20);
        }
        assert_close(toggle.position.get(), 0.67232, "the fifth of the gap left");
        assert!(toggle.is_animating(), "so the slide is still running");
        assert_eq!(
            toggle.thumb_scale.get(),
            1.0,
            "and the bounce has settled: it is the slide, not the thumb, that is moving"
        );
    }

    #[test]
    fn a_toggle_toggled_twice_before_the_slide_finishes_returns_to_where_it_started() {
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        tick(&toggle, 50);
        toggle.checked.set(false);
        toggle.animate_to_state(motion());
        for _ in 0..3 {
            tick(&toggle, 50);
        }
        assert_eq!(toggle.position.get(), 0.0);
        assert_eq!(
            toggle.thumb_center(RECT),
            (12.0, 22.0),
            "and the thumb is back"
        );
    }

    #[test]
    fn two_toggles_keep_separate_clocks() {
        // An `AnimationClock` is cleared whole, which is why a widget owns one:
        // a shared clock would strand the other toggle's slide.
        let (mut nodes, one) = toggle();
        let mut other = Toggle::new(&mut nodes);
        other.set_palette(Palette::from_theme(&Theme::dark()));
        other.snap_to_state();
        one.checked.set(true);
        one.animate_to_state(motion());
        tick(&other, 100);
        assert_eq!(other.position.get(), 0.0, "the other one did not move");
        assert!(!other.is_animating(), "and is not animating");
    }

    #[test]
    fn snapping_puts_a_themed_toggle_where_its_theme_says_at_once() {
        // The failure this guards: a toggle given a palette but never aimed still
        // paints the neutral defaults `Toggle::new` wrote, so a themed toggle
        // starts out grey and only becomes the theme's colour once something has
        // moved it. Built from `new` rather than through the `toggle` helper,
        // which snaps.
        let mut nodes = Arena::new();
        let mut toggle = Toggle::new(&mut nodes);
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(themed.track_off, toggle.track.get(), "so this can fail");

        toggle.set_palette(themed);
        toggle.snap_to_state();

        assert_eq!(toggle.track.get(), themed.track_off);
        assert_eq!(toggle.thumb.get(), themed.thumb);
        assert_eq!(toggle.position.get(), 0.0);
        assert_eq!(toggle.thumb_scale.get(), 1.0);
        assert!(!toggle.is_animating(), "a snap is not a transition");
    }

    #[test]
    fn snapping_a_mid_slide_toggle_back_out_ends_the_slide_and_the_bounce() {
        // Without the `clear`, a transition already running writes over what the
        // snap just set when it arrives, and the toggle drifts off again.
        let (_nodes, toggle) = toggle();
        toggle.checked.set(true);
        toggle.animate_to_state(motion());
        assert!(toggle.tick(ms(10)), "the slide and the bounce are running");

        toggle.checked.set(false);
        toggle.snap_to_state();
        assert_eq!(
            toggle.position.get(),
            0.0,
            "the snap put the thumb at the off end"
        );

        assert!(!toggle.tick(ms(500)), "and nothing arrives afterwards");
        assert_eq!(toggle.position.get(), 0.0);
        assert_eq!(
            toggle.thumb_scale.get(),
            1.0,
            "and the thumb is its own size"
        );
    }

    #[test]
    fn setting_the_palette_leaves_the_toggle_where_it_is() {
        // A theme switch is animated by the caller setting the palette and
        // animating toward it; a palette that moved the toggle on its own would
        // make every theme switch instantaneous.
        let (_nodes, mut toggle) = toggle();
        let before = toggle.track.get();
        toggle.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(toggle.track.get(), before);
    }

    #[test]
    fn a_toggle_paints_its_pill_and_its_thumb() {
        let (_nodes, toggle) = toggle();
        let commands = toggle.paint(RECT);

        let filled = rounded(&commands);
        assert_eq!(filled.len(), 1, "the pill and nothing else is a rect");
        assert_eq!(filled[0].2, toggle.track.get(), "in the colour it holds");

        let drawn = circles(&commands);
        assert_eq!(drawn.len(), 2, "a thumb, which is a shadow and a circle");
        assert_eq!(drawn[0].0, (12.0, 22.0), "at the off end");
        assert_eq!(drawn[0].1, 14.0, "the shadow is the radius plus its inset");
        assert_eq!(
            drawn[1].0,
            (12.0, 22.0),
            "and the thumb's own circle is on top"
        );
        assert_eq!(drawn[1].1, THUMB_RADIUS, "at the radius the constant names");
        assert_eq!(drawn[1].2, toggle.thumb.get());
        assert_eq!(commands.len(), 3, "and there is nothing else to draw");
    }

    #[test]
    fn a_toggle_paints_its_thumb_over_the_pill_and_its_shadow_under_itself() {
        // Both halves of the order need different evidence. `shapes` sees a
        // rounded rect before two circles, which is the thumb over the pill; it
        // cannot tell two circles apart, so the half that is *within* the thumb
        // is the radius, the shadow being the larger of the two.
        let (_nodes, toggle) = toggle();
        let commands = toggle.paint(RECT);
        assert_eq!(
            shapes(&commands),
            vec!["rect", "circle", "circle"],
            "the pill, then the thumb's two circles"
        );

        let drawn = circles(&commands);
        assert_eq!(drawn.len(), 2);
        assert!(
            drawn[0].1 > drawn[1].1,
            "the first of the two is the larger — {} against {} — which is the \
             shadow, and it is recorded first so that it shows as a ring",
            drawn[0].1,
            drawn[1].1
        );
    }

    #[test]
    fn the_thumb_shadow_is_slight() {
        // The same assertion the button's press overlay is given: a shape
        // assertion cannot tell a shadow from a black disc, so the alpha is the
        // number that has to be asserted. 0.3 of black is 255 - 255 * 0.7 = 76.5,
        // which rounds to 77.
        let (_nodes, toggle) = toggle();
        let shadow = circles(&toggle.paint(RECT))[0].2;
        assert_eq!(
            (shadow.r, shadow.g, shadow.b),
            (0, 0, 0),
            "a shadow is black"
        );
        assert_eq!(shadow.a, 77, "at 0.3 of the way to opaque");
        assert!(
            shadow.a < 128,
            "and that is a shadow rather than a border: {}",
            shadow.a
        );
    }

    #[test]
    fn a_toggle_paints_the_pill_in_the_colour_its_state_implies() {
        let (_nodes, toggle) = toggle();
        let at = |toggle: &Toggle| rounded(&toggle.paint(RECT))[0].2;
        let off = at(&toggle);

        toggle.checked.set(true);
        toggle.snap_to_state();
        let on = at(&toggle);

        assert_ne!(on, off, "which is acceptance criterion four");
        assert_eq!(off, toggle.palette().track_off);
        assert_eq!(on, toggle.palette().track_on);
    }

    #[test]
    fn a_focused_toggle_paints_a_ring_around_its_pill_and_the_pill_covers_it() {
        // The ring is around the *pill*, not around the whole node, and it is
        // recorded before the pill that covers it. A `RoundedRect` fills its
        // rect, so a ring around the node would be a card with a pill drawn on
        // it — which is what the slider's demo capture showed, and what every
        // draw-command assertion here would have called correct: the old rect
        // was a real rect of the right colour, in the right place, at the wrong
        // size for an outline. Asserting the grown shape alone is satisfied by a
        // filled rectangle, so the pair is what this test is for.
        let (_nodes, toggle) = toggle();
        assert_eq!(
            rounded(&toggle.paint(RECT)).len(),
            1,
            "no ring while unfocused"
        );

        toggle.focused.set(true);
        let filled = rounded(&toggle.paint(RECT));
        assert_eq!(filled.len(), 2, "a focused one has a ring as well");
        assert_eq!(
            filled[0].0,
            Rect::new(-2.0, 6.0, 52.0, 32.0),
            "the ring is the pill grown by its own width, so its border shows"
        );
        assert_eq!(filled[0].1, 14.0 + FOCUS_RING, "with the pill's own radius");
        assert_eq!(filled[0].2, toggle.palette().ring);
        assert_eq!(
            filled[1].0,
            Rect::new(0.0, 8.0, 48.0, 28.0),
            "and the pill covers the middle of it, which is what leaves the ring"
        );
        assert_eq!(
            shapes(&toggle.paint(RECT)),
            vec!["rect", "rect", "circle", "circle"],
            "recorded ring first, then the pill over it"
        );
    }

    #[test]
    fn a_zero_width_focus_ring_is_not_painted() {
        // A theme or a caller that sets the ring's width to zero turns the
        // indicator off; painting a zero-width ring would be a command that draws
        // nothing.
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        toggle.focus_ring.set(0.0);
        assert_eq!(rounded(&toggle.paint(RECT)).len(), 1);
    }

    #[test]
    fn a_focused_and_checked_toggle_keeps_both_the_ring_and_the_on_colour() {
        // The states overlap, which is why they are two booleans and not one
        // enum: the ring and the checked appearance are independent of one
        // another.
        let (_nodes, toggle) = toggle();
        toggle.focused.set(true);
        toggle.checked.set(true);
        toggle.snap_to_state();
        let filled = rounded(&toggle.paint(RECT));
        assert_eq!(filled.len(), 2, "the ring and the pill");
        assert_eq!(filled[1].2, toggle.palette().track_on, "and the pill is on");
    }

    #[test]
    fn a_toggle_asks_for_a_rect_a_finger_can_reach() {
        // The pill is 28 pixels tall and no finger can be asked to hit one, so
        // the floor is on the node and the pill is centred inside it.
        let (_nodes, toggle) = toggle();
        let size = toggle.size();
        assert_eq!(size.width, TRACK_WIDTH);
        assert_eq!(size.height, MIN_TOUCH_TARGET);
        assert!(
            size.width >= MIN_TOUCH_TARGET && size.height >= MIN_TOUCH_TARGET,
            "both of its axes are at least the touch target: {size:?}"
        );
    }

    #[test]
    fn the_palette_is_the_themes_muted_primary_and_on_primary() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            let color = |token| theme.get(token).as_color().unwrap();
            assert_eq!(palette.track_off, color(ThemeToken::TextMuted));
            assert_eq!(palette.track_on, color(ThemeToken::Primary));
            assert_eq!(palette.thumb, color(ThemeToken::OnPrimary));
            assert_eq!(palette.ring, color(ThemeToken::Text));
        }
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        // A theme switch has to reach the toggles, or the theme would only change
        // half the window.
        assert_ne!(
            Palette::from_theme(&Theme::dark()),
            Palette::from_theme(&Theme::light())
        );
    }

    #[test]
    fn the_off_and_on_tracks_are_different_colours_in_both_themes() {
        // "Track color changes between off and on" is a claim about both themes,
        // not about the dark one. `Theme` is not `Debug` — it holds a clock — so
        // the themes are named rather than printed.
        for (name, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
            let palette = Palette::from_theme(&theme);
            assert_ne!(palette.track_off, palette.track_on, "in the {name} theme");
        }
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
            Palette::from_theme(&theme).track_on,
            Color::new(0, 0, 0, 255),
            "black rather than a panic"
        );
    }

    #[test]
    fn a_toggle_that_has_never_been_aimed_paints_the_neutral_defaults() {
        // The other half of the themed-toggle defect: a caller who never themes a
        // toggle still gets something visible, which is the default palette's
        // job.
        let mut nodes = Arena::new();
        let toggle = Toggle::new(&mut nodes);
        assert_eq!(toggle.track.get(), Palette::default().track_off);
        assert_eq!(
            rounded(&toggle.paint(RECT))
                .first()
                .map(|(_, _, color)| *color),
            Some(Palette::default().track_off)
        );
    }

    #[test]
    fn a_toggle_paints_nothing_that_is_not_its_pill_and_its_thumb() {
        // A toggle with a label is a different widget: the task lists it out of
        // scope, and this is what "out of scope" looks like in the commands.
        let (_nodes, toggle) = toggle();
        for command in toggle.paint(RECT) {
            assert!(
                matches!(
                    command,
                    DrawCommand::RoundedRect { .. } | DrawCommand::Circle { .. }
                ),
                "a toggle draws two shapes and no text: {command:?}"
            );
        }
    }
}
