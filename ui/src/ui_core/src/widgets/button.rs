//! The Button widget: visual states, the transitions between them, and the
//! click.
//!
//! A button is a node, the five properties the task gives it, four state
//! properties the caller writes, and four properties the widget animates. The
//! state properties are booleans rather than one state enum because the states
//! overlap: a button can be focused *and* hovered, and an enum could only hold
//! one of the two. [`ButtonState`] names the five states the task distinguishes,
//! [`Button::state`] resolves them to the one a caller that asks for the primary
//! state wants, and [`Button::style`] resolves all four flags to the appearance
//! that is actually drawn.
//!
//! The appearance a state implies is a [`Style`]: colours, a scale, an opacity
//! and a focus ring's width. [`Button::style`] is the pure resolution of that,
//! and [`Button::animate_to_state`] is the same resolution handed to a clock, so
//! a caller can see where a transition is going and the widget can start one
//! without the caller repeating the resolution.
//!
//! The button owns the clock its transitions run on, the way [`Theme`] owns the
//! one its token transitions run on. An owned clock is what makes a state change
//! *replace* the transition before it: [`AnimationClock::clear`] is whole-clock,
//! and a clock shared with other widgets would strand theirs. The caller ticks
//! it with [`Button::tick`] and learns from the return value whether anything
//! moved, which is the same contract [`Theme::tick`] has.
//!
//! Two things are the caller's, not the button's: which node is dirty, and which
//! node is focused. The first is a property callback per the demo's own idiom —
//! `on_change` on the animated properties, marking the node — and the second is
//! [`input::Focus`](crate::input::Focus), which knows the focus order and not
//! what Enter means.
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::widgets::button::{Button, Motion, Palette};
//! use ui_core::theme::Theme;
//!
//! let mut nodes = Arena::new();
//! let mut button = Button::new(&mut nodes, "Press me");
//! button.set_palette(Palette::from_theme(&Theme::dark()));
//!
//! // A press darkens the background and shrinks the button to 0.95 of its size,
//! // both over the theme's fast duration.
//! button.pressed.set(true);
//! button.animate_to_state(Motion::from_theme(&Theme::dark()));
//! for _ in 0..15 {
//!     button.tick(Duration::from_millis(10));
//! }
//! assert_eq!(button.scale.get(), 0.95, "the press transition has arrived");
//! ```

use std::cell::RefCell;
use std::time::Duration;

use crate::animation::{AnimationClock, Easing, Interpolate};
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::Size;
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::label::{layout_text, LayoutOptions, TextAlign, Truncation, WrapMode};

/// The smallest a button may be, in either dimension, in pixels.
///
/// 44dp is the platform touch target a finger can hit, and it is a constant here
/// rather than a theme token because the theme has no token for it: adding one
/// would change [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme
/// tables, the token count, and the animation every token takes part in during a
/// theme switch — all for a value a theme switch does not change. Revisit when
/// a second control needs the same floor, or when the operator wants it
/// themeable; either is a theme change, not a button change.
const MIN_TOUCH_TARGET: f32 = 44.0;

/// The default padding and corner radius, from the theme's own values.
///
/// The task names `SpacingSm` across, `SpacingXs` down and the default radius,
/// and a caller that never themes a button should still get the geometry it
/// asked for. These are the numbers both themes hold today, copied rather than
/// read, because the widget has no theme to read them from — a caller themes a
/// button by binding these three properties to
/// [`Theme::property`](crate::theme::Theme::property), which is the mechanism
/// the property graph already carries a theme switch on.
///
/// Reading them per frame instead would be the alternative, and it is not
/// taken: a widget that reached into a theme on every layout pass would need to
/// be told about a switch rather than following one. The test
/// `the_default_padding_and_radius_are_the_theme_tokens_values` is what stops
/// these three from drifting away from the tokens they name.
const THEME_SPACING_SM: f32 = 8.0;
const THEME_SPACING_XS: f32 = 4.0;
const THEME_RADIUS_MD: f32 = 8.0;

/// How far a hovered button's background moves toward white.
const HOVER_LIGHTEN: f32 = 0.12;

/// How far a pressed button's background moves toward black.
const PRESS_DARKEN: f32 = 0.18;

/// The scale a pressed button is drawn at.
const PRESSED_SCALE: f32 = 0.95;

/// How far a disabled button's colours move toward the even grey.
const DISABLED_DESATURATE: f32 = 0.8;

/// The opacity a disabled button is drawn at.
const DISABLED_OPACITY: f32 = 0.45;

/// How far inside the background the pressed overlay is drawn.
///
/// A shadow that reached the background's edge would read as a border rather
/// than as depth, so the overlay is inset and the corners follow.
const PRESS_SHADOW_INSET: f32 = 2.0;

/// The most opaque the pressed overlay ever gets.
///
/// The task asks for a *slight* inner shadow, and the background is already
/// darkened by [`PRESS_DARKEN`] through [`Button::style`], so this only has to
/// add depth on top of that. It used to interpolate to opaque black, which
/// turned a pressed button into a solid black rectangle with the label
/// invisible on it — the label is drawn after the overlay, and in the dark
/// theme it is the theme's `OnPrimary`, which is black.
///
/// A capture is what found that, not a test: 46 of them passed throughout,
/// because asserting on the recorded draw commands cannot tell a slight shadow
/// from a black one. Both are "a rounded rect of near-black at some alpha".
const PRESS_SHADOW_ALPHA: f32 = 0.28;

/// A button's click handler: an action callback with nothing to carry.
///
/// This used to be the button's own callback type, and its own doc said it
/// would move somewhere a second widget could reach it. That second widget is
/// [`slider::Callback`](crate::widgets::Callback): a slider reports the value it
/// moved to, so its handler takes one, and a type that cannot carry a payload
/// could not be it. The type itself is now
/// [`widgets::Callback`](crate::widgets::Callback), parameterised, and this alias
/// is the button's spelling of the `()` case — so `button::Callback` still names
/// what it named, and a caller writes `Callback::new(move || …)` exactly as
/// before.
///
/// One thing did change, and it is the cost of the payload: the shared type's
/// [`call`](crate::widgets::Callback::call) takes the payload, so calling this
/// one directly is `call(())`. That is inside this module; a caller goes through
/// [`Button::activate`], which is the button's contract with the world.
pub type Callback = crate::widgets::Callback<()>;

/// The colours a button's states are derived from.
///
/// The states are not colours of their own — the theme has no token per state,
/// and adding one per state would put six more tokens in every theme table and
/// in every theme switch. So a button is themed with one pair of colours and
/// one ring colour, and [`Button::style`] derives the hovered, pressed and
/// disabled appearances from them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The background at rest: the resting, hovered and pressed backgrounds are
    /// this colour moved toward white or black.
    pub background: Color,
    /// The label's colour.
    pub foreground: Color,
    /// The focus ring's colour. It is a separate field from the label's because
    /// a ring has to be legible on the background *and* on what the background
    /// is drawn over, which the label colour is not asked to be.
    pub ring: Color,
}

impl Default for Palette {
    /// Returns a neutral grey button: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    fn default() -> Self {
        Palette {
            background: Color::new(64, 64, 64, 255),
            foreground: Color::new(240, 240, 240, 255),
            ring: Color::new(255, 255, 255, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The background is [`Primary`](crate::theme::ThemeToken::Primary) — a
    /// button is the primary action — and the label and the ring are
    /// [`OnPrimary`](crate::theme::ThemeToken::OnPrimary), which is by
    /// definition the colour drawn on the primary surface and is therefore
    /// legible against it in both themes: black on the dark theme's violet,
    /// white on the light theme's deeper one.
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            background: token_color(theme, crate::theme::ThemeToken::Primary),
            foreground: token_color(theme, crate::theme::ThemeToken::OnPrimary),
            ring: token_color(theme, crate::theme::ThemeToken::OnPrimary),
        }
    }
}

/// The motion a button's transitions run on: how long each takes, and the curve
/// they follow.
///
/// A value rather than two arguments because both come from the theme together —
/// [`DurationFast`](crate::theme::ThemeToken::DurationFast) and
/// [`EasingStandard`](crate::theme::ThemeToken::EasingStandard) — and a caller
/// that chose one of them by hand would be asserting something the theme says
/// otherwise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    /// How long one transition takes.
    pub duration: Duration,
    /// The curve it follows.
    pub easing: Easing,
}

impl Motion {
    /// Returns the motion a theme describes: its fast duration and its standard
    /// curve.
    ///
    /// The fallbacks are for a theme that holds something else in either token,
    /// and they are the values both themes hold anyway, so a mismatched token
    /// moves nothing rather than producing a transition of no length or of a
    /// curve that is not a curve.
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        let duration = match theme.get(crate::theme::ThemeToken::DurationFast) {
            crate::theme::PropertyValue::Duration(duration) => duration,
            _ => Duration::from_millis(150),
        };
        let easing = match theme.get(crate::theme::ThemeToken::EasingStandard) {
            crate::theme::PropertyValue::Easing(easing) => easing,
            _ => Easing::EaseInOut,
        };
        Motion { duration, easing }
    }
}

/// The visual states a button distinguishes.
///
/// This is the *primary* state: the states overlap, and a button that is
/// focused and hovered is [`Hovered`](ButtonState::Hovered) here, because the
/// hover is what its background shows and the focus is only its ring. The
/// appearance of the combination is [`Button::style`], which is what is
/// actually drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    /// Nothing is happening: the resting appearance.
    Default,
    /// A pointer is over the button.
    Hovered,
    /// The button is held down.
    Pressed,
    /// The button refuses interaction and is drawn inert.
    Disabled,
    /// The button holds keyboard or gamepad focus.
    Focused,
}

/// The appearance one set of state flags implies.
///
/// Every field is a target, not a value in flight: [`Button::animate_to_state`]
/// animates the button's properties toward this, and [`Button::paint`] draws
/// whatever the properties have reached, which is a [`Style`] part way through
/// on a frame where something is moving.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The background the button is heading for.
    pub background: Color,
    /// The colour of the label and the ring.
    pub foreground: Color,
    /// The scale the button is drawn at.
    pub scale: f32,
    /// The opacity the button is drawn at, before its own colours.
    pub opacity: f32,
    /// The focus ring's thickness, or zero for no ring.
    pub ring_width: f32,
}

/// A button: a control that does something when it is clicked.
///
/// The widget holds the properties the task gives it — [`label`], [`background`],
/// [`foreground`], [`border_radius`] and [`on_click`] — the four state
/// properties a caller writes, and the four it animates: background, label
/// colour, scale and opacity. The node is the caller's to keep clean: a property
/// change marks it dirty through the `on_change` callbacks the caller
/// registers, which is the same link the demo wires its pads and labels
/// through.
///
/// The properties that hold a theme token are bound by the caller rather than
/// read from a theme here, for the reason the rest of the library reads tokens
/// through properties: the property graph carries a theme switch to a widget
/// that is holding one, and a widget that reached into a theme would need to be
/// told about the switch instead. The defaults are neutral literals, the way a
/// [`Label`](crate::widgets::label::Label)'s are, and they carry the values the
/// task names — `SpacingSm` across, `SpacingXs` down, `BorderRadiusMd` for the
/// corners — so a caller that never themes a button still gets the geometry the
/// task asked for.
///
/// [`label`]: Button::label
/// [`background`]: Button::background
/// [`foreground`]: Button::foreground
/// [`border_radius`]: Button::border_radius
/// [`on_click`]: Button::on_click
///
/// # Examples
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
/// use ui_core::arena::Arena;
/// use ui_core::input::{InputEvent, InputEventKind};
/// use ui_core::layout::Offset;
/// use ui_core::node::WidgetNode;
/// use ui_core::widgets::button::{Button, ButtonState, Callback};
///
/// let mut nodes = Arena::new();
/// let mut button = Button::new(&mut nodes, "Press me");
/// let clicks = Rc::new(Cell::new(0));
/// let counted = Rc::clone(&clicks);
/// button.on_click = Callback::new(move || counted.set(counted.get() + 1));
///
/// assert_eq!(button.state(), ButtonState::Default);
/// assert!(nodes.get(button.handle()).is_some());
///
/// // A tap is consumed whether or not the button acts on it.
/// let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(4.0, 4.0)));
/// assert!(button.on_event(&mut tap));
/// assert_eq!(clicks.get(), 1);
/// assert!(tap.consumed(), "and it does not travel on to the parent");
/// ```
pub struct Button {
    /// The text drawn in the button.
    pub label: Property<String>,
    /// The button's background, animated between the colours its states imply.
    pub background: Property<Color>,
    /// The label's colour, animated alongside the background.
    pub foreground: Property<Color>,
    /// The corner radius of the background, in pixels.
    pub border_radius: Property<f32>,
    /// The callback a tap or an activation key fires.
    pub on_click: Callback,
    /// The focus ring's thickness, in pixels. Zero draws no ring even when the
    /// button is focused.
    pub focus_ring: Property<f32>,
    /// The gap between the label and the button's left and right edges.
    pub padding_h: Property<f32>,
    /// The gap between the label and the button's top and bottom edges.
    pub padding_v: Property<f32>,
    /// The label's font size, in pixels.
    pub font_size: Property<f32>,
    /// Whether a pointer is over the button. Written by the caller, from a hit
    /// test or from the pointer's own state.
    pub hovered: Property<bool>,
    /// Whether the button is held down.
    pub pressed: Property<bool>,
    /// Whether the button refuses interaction and is drawn inert.
    pub disabled: Property<bool>,
    /// Whether the button holds focus. Written by the caller, from
    /// [`input::Focus`](crate::input::Focus).
    pub focused: Property<bool>,
    /// Whether the button may be **activated** — fired by a tap or by an
    /// activation key. Written by the caller. **Defaults to `true`.**
    ///
    /// **A second flag rather than a mode of [`Button::focused`], because the two
    /// answers come apart.** `focused` means "this control has keyboard focus" —
    /// the meaning it has in every other widget in this tree — and it is what
    /// draws the ring and answers the keyboard. Those are the same answer right up
    /// until a control that is *leaving* needs to say otherwise: a dialog's action
    /// button is still on screen for the whole 300 ms of a dismissal, so its ring
    /// belongs in the picture, and it is already unreachable from the first frame,
    /// so an activation key must not fire it. One bit cannot answer both, and
    /// before this property existed a caller had to carry the distinction in
    /// guards of its own around the widget.
    ///
    /// **`true` by default, and that is the load-bearing choice.** A caller that
    /// writes only `focused` — which is every caller in this repository, and the
    /// only production writer of a button's `focused` anywhere in it is
    /// `ui_demo`'s `Demo::sync_dialog_focus` — must get exactly the behaviour it
    /// got before this field existed: ring and activation both. A `false` default
    /// looks the more conservative choice and is a trap, because it would strip
    /// the ring and the key handling off every button a caller builds without
    /// saying so, and **absence of a blocker is permission** for a control that
    /// holds focus.
    /// `a_button_written_only_with_focused_still_activates_and_still_draws_its_ring`
    /// is the test that holds the default in place.
    ///
    /// **An adjective like the rest, and not a near miss for
    /// [`Button::focus_ring`]**, which is the ring's *width* in pixels and shares
    /// no question with this one.
    pub activatable: Property<bool>,
    /// The scale the button is drawn at, animated on press.
    pub scale: Property<f32>,
    /// The opacity the button is drawn at, animated for the disabled state.
    pub opacity: Property<f32>,
    palette: Palette,
    clock: RefCell<AnimationClock>,
    node: Handle,
}

impl Button {
    /// Creates a button showing `text` in the arena, and returns it.
    ///
    /// The button's node starts dirty, so the next layout pass places it, and
    /// its transitions are unthemed defaults until a caller gives it a
    /// [`Palette`](Button::set_palette).
    ///
    /// The task file's `Button::new(label) -> Handle` is read as this: the handle
    /// is [`Button::handle`]'s, and returning it alone would leave the caller
    /// with no properties to set and no callback to register, which is the whole
    /// of what the button is for.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, text: impl Into<String>) -> Self {
        let palette = Palette::default();
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Button {
            label: Property::new(text.into()),
            background: Property::new(palette.background),
            foreground: Property::new(palette.foreground),
            border_radius: Property::new(THEME_RADIUS_MD),
            on_click: Callback::none(),
            focus_ring: Property::new(2.0),
            padding_h: Property::new(THEME_SPACING_SM),
            padding_v: Property::new(THEME_SPACING_XS),
            font_size: Property::new(14.0),
            hovered: Property::new(false),
            pressed: Property::new(false),
            disabled: Property::new(false),
            focused: Property::new(false),
            // `true`, and the field's doc is the argument: every existing caller
            // writes only `focused` and must keep both behaviours.
            activatable: Property::new(true),
            scale: Property::new(1.0),
            opacity: Property::new(1.0),
            palette,
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the button's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the button's states are derived from.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the button's states are derived from, and leaves the
    /// button's current appearance where it is.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`Button::animate_to_state`]: a theme switch is animated, and a theme
    /// switch is the caller announcing a new palette and then animating toward
    /// it. Moving it here would make a theme switch instantaneous and would
    /// leave the button chasing a palette that is still moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the button's primary state, in the order one overrides another:
    /// disabled, then pressed, then hovered, then focused, then at rest.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::{Button, ButtonState};
    ///
    /// let mut nodes = Arena::new();
    /// let button = Button::new(&mut nodes, "Press me");
    /// assert_eq!(button.state(), ButtonState::Default);
    ///
    /// button.hovered.set(true);
    /// button.focused.set(true);
    /// assert_eq!(
    ///     button.state(),
    ///     ButtonState::Hovered,
    ///     "a hovered button is hovered whatever else it is"
    /// );
    /// ```
    #[must_use]
    pub fn state(&self) -> ButtonState {
        if self.disabled.get() {
            ButtonState::Disabled
        } else if self.pressed.get() {
            ButtonState::Pressed
        } else if self.hovered.get() {
            ButtonState::Hovered
        } else if self.focused.get() {
            ButtonState::Focused
        } else {
            ButtonState::Default
        }
    }

    /// Returns the appearance the button's current state flags imply.
    ///
    /// The states are applied in the order they override one another, and the
    /// focus ring is applied independently of all of them: a button that is both
    /// focused and pressed has the pressed background *and* the ring, and a
    /// disabled button has neither — it is inert, and an inert control that
    /// cannot be focused should not claim to be.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Button;
    ///
    /// let mut nodes = Arena::new();
    /// let button = Button::new(&mut nodes, "Press me");
    /// let rest = button.style();
    ///
    /// button.pressed.set(true);
    /// let pressed = button.style();
    /// assert_eq!(pressed.scale, 0.95, "the press shrinks it to 0.95");
    /// assert_ne!(pressed.background, rest.background, "and a darker background");
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        let mut style = Style {
            background: self.palette.background,
            foreground: self.palette.foreground,
            scale: 1.0,
            opacity: 1.0,
            ring_width: 0.0,
        };
        if self.focused.get() {
            style.ring_width = self.focus_ring.get();
        }
        if self.hovered.get() {
            style.background = shade(style.background, HOVER_LIGHTEN);
        }
        if self.pressed.get() {
            style.background = shade(style.background, -PRESS_DARKEN);
            style.scale = PRESSED_SCALE;
        }
        if self.disabled.get() {
            style.background = desaturate(style.background, DISABLED_DESATURATE);
            style.foreground = desaturate(style.foreground, DISABLED_DESATURATE);
            style.opacity = DISABLED_OPACITY;
            style.scale = 1.0;
            style.ring_width = 0.0;
        }
        style
    }

    /// Applies the appearance the button's current state implies immediately,
    /// with no transition.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer. The first is a button that has just been given a
    /// [`Palette`](Button::set_palette) and has never animated: its properties
    /// still hold the colours [`Button::new`] wrote, which are the neutral
    /// default palette's, so without this a themed button starts out grey and
    /// only becomes the theme's colour once something has moved it. The second
    /// is a caller that has written a state property itself and wants the
    /// button to be that state now rather than to arrive at it.
    ///
    /// Any transition already running is cleared first, so it cannot write over
    /// what this just set when it arrives.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::button::{Button, Palette};
    ///
    /// let mut nodes = Arena::new();
    /// let mut button = Button::new(&mut nodes, "Press me");
    /// let themed = Palette::from_theme(&Theme::dark());
    /// button.set_palette(themed);
    ///
    /// // Aiming alone would leave the button grey until the transition ran.
    /// button.snap_to_state();
    /// assert_eq!(button.background.get(), themed.background);
    ///
    /// // And it does not animate, so nothing is left running.
    /// assert!(!button.is_animating());
    /// assert!(!button.tick(Duration::from_millis(16)));
    /// ```
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.background.set(style.background);
        self.foreground.set(style.foreground);
        self.scale.set(style.scale);
        self.opacity.set(style.opacity);
    }

    /// Starts the transitions that carry the button from wherever it is to the
    /// appearance its state flags imply, on `motion`.
    ///
    /// The button's own clock is cleared first, so the transitions this replaces
    /// stop where they are rather than writing over the new ones when they
    /// arrive — which is what a [`AnimationClock`] shared with other widgets
    /// could not do.
    ///
    /// The target is [`Button::style`] at the moment of the call, not a
    /// continuous target: a theme switch is animated by the caller announcing the
    /// new palette and calling this again, the same way the demo re-aims a pad's
    /// press when the theme moves under it.
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.background
                .animate_to(style.background, motion.duration, motion.easing),
        );
        clock.add(
            self.foreground
                .animate_to(style.foreground, motion.duration, motion.easing),
        );
        clock.add(
            self.scale
                .animate_to(style.scale, motion.duration, motion.easing),
        );
        clock.add(
            self.opacity
                .animate_to(style.opacity, motion.duration, motion.easing),
        );
    }

    /// Advances the button's transitions by `delta`, and returns whether any of
    /// them wrote.
    ///
    /// It is the button's frame integration: call it once a frame, before the
    /// paint pass, with the time that frame took. The write is what reaches the
    /// node — a property callback registered by the caller marks the node dirty
    /// — so a caller that repaints only when this is true repaints exactly while
    /// something moves.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Returns whether any of the button's transitions is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Runs the button's click callback, and reports whether it did.
    ///
    /// A click is the button's whole contract with its caller, and the button
    /// does not consult its own state: a caller that activates a disabled button
    /// has a bug, and this is where it shows rather than being papered over. What
    /// *is* checked is whether there is anything to run, so a caller can tell a
    /// button that acted from one with nothing to do.
    #[must_use]
    pub fn activate(&self) -> bool {
        if !self.on_click.is_set() {
            return false;
        }
        self.on_click.call(());
        true
    }

    /// Handles `event` as this button would, and reports whether it consumed it.
    ///
    /// A [`Tap`](InputEventKind::Tap) is always consumed, and fires the click
    /// unless the button is [disabled](Button::disabled) or
    /// [not activatable](Button::activatable). A button in either state still
    /// swallows the tap: the event was aimed at it, and letting it through would
    /// make a tap on an inert control reach the panel behind it. **Consumed
    /// rather than declined, and the same as `disabled`'s rule on purpose** — a
    /// control that is on screen and refusing to act is still a wall, and a
    /// caller whose dialog is fading wants the key to stop here rather than reach
    /// whatever is behind it.
    ///
    /// An activation key — Enter, the keypad's Enter, Space, or the gamepad's
    /// south button — fires the click and is consumed, but only while the button
    /// holds focus. A key press is not routed by position, so this is called on
    /// the focused node by the caller, and an unfocused button must not answer
    /// one.
    ///
    /// Every other event is left alone and not consumed, so it carries on up the
    /// tree.
    ///
    /// Activation fires on every `KeyDown` and [`InputEventKind::KeyDown`] has no
    /// repeat flag, so a held Enter activates once per repeat the platform sends.
    /// Adding one is a change to the input module's event, not to the button.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind, Key};
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::{Button, Callback};
    ///
    /// let mut nodes = Arena::new();
    /// let mut button = Button::new(&mut nodes, "Press me");
    /// let clicks = Rc::new(Cell::new(0));
    /// let counted = Rc::clone(&clicks);
    /// button.on_click = Callback::new(move || counted.set(counted.get() + 1));
    ///
    /// // Enter only reaches a focused button.
    /// let mut enter = InputEvent::new(
    ///     InputEventKind::KeyDown {
    ///         key: Key::Keyboard(sdl3::keyboard::Keycode::Return),
    ///         keymod: sdl3::keyboard::Mod::empty(),
    ///     },
    ///     None,
    /// );
    /// assert!(!button.on_event(&mut enter), "an unfocused button ignores Enter");
    /// assert!(!enter.consumed(), "and lets it travel on");
    ///
    /// button.focused.set(true);
    /// assert!(button.on_event(&mut enter));
    /// assert_eq!(clicks.get(), 1);
    /// ```
    pub fn on_event(&self, event: &mut InputEvent) -> bool {
        match event.kind() {
            InputEventKind::Tap => {
                event.consume();
                if self.may_activate() {
                    let _ = self.activate();
                }
                true
            }
            InputEventKind::KeyDown { key, .. }
                if self.focused.get() && is_activation_key(&key) =>
            {
                event.consume();
                if self.may_activate() {
                    let _ = self.activate();
                }
                true
            }
            _ => false,
        }
    }

    /// Returns whether a click would actually fire: the button is neither
    /// [`disabled`](Button::disabled) nor [withdrawn](Button::activatable).
    ///
    /// **Public because a caller can activate a button without going through
    /// [`Button::on_event`]**, and [`Dialog`](crate::widgets::dialog::Dialog)'s
    /// own tap arm does exactly that: it measures which action a tap landed on
    /// and activates it, because a positional hit test is its business rather
    /// than the button's. That arm used to ask `disabled` alone, so it would have
    /// kept firing a withdrawn button while the button's own key arm refused it —
    /// one property, two answers. Exposing the rule is what makes the two agree by
    /// construction rather than by both being edited.
    ///
    /// **The one definition of "does not act"**, for [`disabled`](Button::disabled)
    /// and [`activatable`](Button::activatable) together: a disabled button and a
    /// withdrawn one differ in *why* and in what a caller has to remember, not in
    /// what the button does when it is asked.
    #[must_use]
    pub fn may_activate(&self) -> bool {
        !self.disabled.get() && self.activatable.get()
    }

    /// Returns the size the button's label and padding need, with no minimum
    /// applied.
    ///
    /// `advance` supplies a character's width in the button's font and
    /// `line_height` the height of one line, the same seam
    /// [`label::LayoutOptions`](crate::widgets::label::LayoutOptions) takes, so
    /// this is testable without a font.
    ///
    /// A button's label is one line: it is neither wrapped nor truncated, so a
    /// label too long for the button overflows it rather than being cut. A
    /// newline is honoured, because it is a character in the text and not
    /// something the button decides to honour or not.
    ///
    /// A negative `line_height` is treated as no line at all, since a negative
    /// height would make the button smaller than its own padding.
    #[must_use]
    pub fn content_size(&self, advance: &dyn Fn(char) -> f32, line_height: f32) -> Size {
        let options = LayoutOptions {
            max_width: f32::INFINITY,
            line_height: line_height.max(0.0),
            wrap: WrapMode::None,
            truncation: Truncation::None,
            ..LayoutOptions::default()
        };
        let layout = layout_text(&self.label.get(), &options, advance);
        let text_width = layout
            .lines
            .iter()
            .map(|line| line.width)
            .fold(0.0, f32::max);
        Size::new(
            text_width + self.padding_h.get() * 2.0,
            layout.total_height + self.padding_v.get() * 2.0,
        )
    }

    /// Returns the size the button's node should be given: its content's size,
    /// floored at 44×44 pixels in both dimensions.
    ///
    /// The floor is what the task's minimum touch target means in practice: a
    /// button labelled `Ok` is a 24-pixel target without it, and a finger cannot
    /// be asked to hit one. It is a floor on the *node*, so a caller that gives
    /// the button a larger rect keeps it — the floor never shrinks anything.
    ///
    /// `advance` and `line_height` are the measurements
    /// [`Button::content_size`] takes.
    #[must_use]
    pub fn size(&self, advance: &dyn Fn(char) -> f32, line_height: f32) -> Size {
        let content = self.content_size(advance, line_height);
        Size::new(
            content.width.max(MIN_TOUCH_TARGET),
            content.height.max(MIN_TOUCH_TARGET),
        )
    }

    /// Returns the draw commands that paint the button within `rect`, at the
    /// button's own [`opacity`](Button::opacity).
    ///
    /// **This is [`Button::paint_faded`] with a multiplier of one**, and it is
    /// written as that call rather than as a second copy of the body so the two
    /// cannot drift: a caller fading a button and a caller not fading one are then
    /// the same code path, differing by one number.
    ///
    /// The commands are, in order: the focus ring, if the button is focused and
    /// the ring is not zero wide; the background, at the button's current scale;
    /// the pressed overlay, while the button is below its resting scale; and the
    /// label, centred in the box the padding leaves.
    ///
    /// The scale is applied here, to the rect that is drawn, and not to the
    /// node's own rect: a press that reflowed the tree would move everything
    /// around it, and the whole point of a 0.95 scale is that nothing else
    /// notices. The overlay's alpha follows the same scale, so it fades in and
    /// out with the press instead of appearing and vanishing with the flag.
    ///
    /// `advance` and `line_height` are the measurements
    /// [`Button::content_size`] takes.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::button::Button;
    ///
    /// let mut nodes = Arena::new();
    /// let button = Button::new(&mut nodes, "Press me");
    /// let commands = button.paint(Rect::new(0.0, 0.0, 100.0, 44.0), &|_: char| 5.0, 14.0);
    ///
    /// // The background, then the label. No ring, and no overlay, at rest.
    /// assert_eq!(commands.len(), 2);
    /// ```
    #[must_use]
    pub fn paint(
        &self,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Vec<DrawCommand> {
        self.paint_faded(rect, advance, line_height, 1.0)
    }

    /// Returns the draw commands that paint the button within `rect`, with every
    /// colour scaled by `multiplier` on top of the button's own
    /// [`opacity`](Button::opacity).
    ///
    /// **`multiplier` is a multiplier and not a replacement**, and that is the one
    /// thing a caller has to be sure of: what reaches the screen is
    /// `self.opacity * multiplier`, **not** whichever of the two is smaller. For
    /// an enabled button — `opacity` at 1.0 — the two readings are identical, and
    /// they part company the moment the button is not enabled: a disabled button
    /// at `opacity` 0.5 painted with a multiplier of 0.5 comes out at **0.25**,
    /// where a replacement would have given 0.5. A caller that means "draw this
    /// button at half strength" wants the product; a caller that means "ignore the
    /// button's own opacity" wants [`Button::paint`].
    ///
    /// **It exists because [`Button::opacity`] is documented as the *disabled*
    /// state's opacity, and a button inside something that is itself fading is a
    /// second, unrelated reason for it to be drawn translucent.**
    /// [`Dialog`](crate::widgets::dialog::Dialog) is the case in point: it fades
    /// its own panel, title and body with its transition's progress, and before
    /// this method existed it handed its action buttons to
    /// [`Button::paint`] — so the buttons stayed at full strength for the whole
    /// 300 ms of the fade and then vanished in one frame. That is what an operator
    /// reported as *"the buttons blink for a moment"*.
    ///
    /// `multiplier` is clamped with the product into `0.0..=1.0`, so a negative
    /// value draws nothing rather than wrapping. A value **above** one brightens
    /// toward opaque instead of overflowing, which is a misuse rather than a
    /// feature: the one caller in this repository passes a transition's progress,
    /// which is `0.0..=1.0` by construction.
    ///
    /// **Every colour the button draws goes through it** — the focus ring, the
    /// background, the pressed overlay and the label — because each is computed
    /// from the one `opacity` local below rather than from `self.opacity` where it
    /// is used. `a_focus_ring_fades_with_the_multiplier_and_not_only_the_label`
    /// is the test for the ring specifically, since a ring that stayed solid while
    /// the button faded would be the same defect one primitive in.
    ///
    /// `advance` and `line_height` are the measurements
    /// [`Button::content_size`] takes, and the commands are the ones
    /// [`Button::paint`] lists.
    ///
    /// # Examples
    ///
    /// The multiplier is visible on the numbers, which is what a caller fading a
    /// button actually needs to check:
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{DrawCommand, Rect};
    /// use ui_core::widgets::button::Button;
    ///
    /// let mut nodes = Arena::new();
    /// let button = Button::new(&mut nodes, "OK");
    /// let rect = Rect::new(0.0, 0.0, 44.0, 44.0);
    ///
    /// let alpha_of = |commands: &[DrawCommand]| commands.iter().find_map(|command| match command {
    ///     DrawCommand::RoundedRect { color, .. } => Some(color.a),
    ///     _ => None,
    /// }).expect("a background");
    ///
    /// let full = button.paint_faded(rect, &|_: char| 5.0, 14.0, 1.0);
    /// let half = button.paint_faded(rect, &|_: char| 5.0, 14.0, 0.5);
    /// assert_eq!(alpha_of(&full), 255, "an unfaded button is opaque");
    /// assert_eq!(alpha_of(&half), 128, "and half of one is half of the alpha");
    /// ```
    #[must_use]
    pub fn paint_faded(
        &self,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
        multiplier: f32,
    ) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let scale = self.scale.get();
        // **The product, not `self.opacity` alone**, and the doc above is the whole
        // of why: the button's own opacity is the disabled state's and the
        // multiplier is whatever the button is drawn inside.
        let opacity = (self.opacity.get() * multiplier).clamp(0.0, 1.0);
        let background = scaled(rect, scale);
        let radius = self.border_radius.get();
        let ring = self.focus_ring.get();

        if ring > 0.0 && self.focused.get() {
            painter.rounded_rect(
                grow(background, ring),
                radius + ring,
                with_opacity(self.palette.ring, opacity),
            );
        }
        painter.rounded_rect(
            background,
            radius,
            with_opacity(self.background.get(), opacity),
        );

        let press = press_amount(scale);
        if press > 0.0 {
            painter.rounded_rect(
                inset(background, PRESS_SHADOW_INSET),
                (radius - PRESS_SHADOW_INSET).max(0.0),
                // The overlay is the one colour that never went through `opacity`
                // before this method existed, so it is the one that would have
                // popped: a press held while its dialog faded would leave a
                // full-strength black rectangle over a half-transparent panel.
                with_opacity(
                    Color::new(0, 0, 0, 255),
                    press * PRESS_SHADOW_ALPHA * opacity,
                ),
            );
        }

        let inner = content_box(background, self.padding_h.get(), self.padding_v.get());
        let options = LayoutOptions {
            max_width: inner.width,
            line_height: line_height.max(0.0),
            align: TextAlign::Center,
            wrap: WrapMode::None,
            truncation: Truncation::None,
            ..LayoutOptions::default()
        };
        let layout = layout_text(&self.label.get(), &options, advance);
        let color = with_opacity(self.foreground.get(), opacity);
        let mut top = inner.y + (inner.height - layout.total_height) / 2.0;
        for line in &layout.lines {
            painter.text(
                inner.x + line.x_offset,
                top,
                &line.text,
                color,
                self.font_size.get(),
                0.0,
            );
            top += options.line_height;
        }

        painter.finish()
    }
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback
/// for a token a caller has written the wrong variant into — and black rather
/// than a panic, because a mistyped theme token is not worth taking a frame
/// down for.
fn token_color(theme: &Theme, token: crate::theme::ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

/// Returns `color` moved `t` of the way toward white, or toward black where `t`
/// is negative.
///
/// `t` is clamped to `0.0..=1.0`, so no caller can ask for a colour outside the
/// segment between `color` and the extreme it names.
fn shade(color: Color, t: f32) -> Color {
    let extreme = if t < 0.0 {
        Color::new(0, 0, 0, color.a)
    } else {
        Color::new(255, 255, 255, color.a)
    };
    Color::interpolate(&color, &extreme, t.abs().clamp(0.0, 1.0))
}

/// Returns `color` moved `t` of the way toward an even grey of the same
/// brightness: the "grayed out" a disabled control is drawn in, which takes the
/// colour out without moving it much in or out.
///
/// The grey is the colour's own brightness under the usual luminance weights,
/// and not an average of its channels: averaging a violet gives a colour that is
/// still violet, and a disabled button that is still violet is not gray.
fn desaturate(color: Color, t: f32) -> Color {
    let brightness =
        0.299 * f32::from(color.r) + 0.587 * f32::from(color.g) + 0.114 * f32::from(color.b);
    let level = level(brightness);
    Color::interpolate(
        &color,
        &Color::new(level, level, level, color.a),
        t.clamp(0.0, 1.0),
    )
}

/// Returns `value` as a channel value, rounded and clamped to 0..=255.
///
/// There is no `From`/`TryFrom` between `f32` and any integer type in std, so
/// this is the one float-to-integer `as` cast in the module, for the same reason
/// and with the same guarantee as [`animation`](crate::animation)'s own: the
/// cast is saturating (Rust 1.45 and later), so the clamp is belt and braces and
/// states the intent — a brightness that has come adrift cannot wrap a channel
/// round to the other end of the range.
fn level(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// Returns `color` drawn at `opacity`.
///
/// Interpolating toward transparent black *is* the premultiplied fade: every
/// channel arrives at zero with the alpha, so the colour never bleeds what is
/// under it. `opacity` is clamped, because a caller that has lost track of it
/// cannot ask for a colour outside the segment.
fn with_opacity(color: Color, opacity: f32) -> Color {
    Color::interpolate(
        &color,
        &Color::new(0, 0, 0, 0),
        (1.0 - opacity).clamp(0.0, 1.0),
    )
}

/// Returns `rect` scaled about its own centre.
fn scaled(rect: Rect, scale: f32) -> Rect {
    let width = rect.width * scale;
    let height = rect.height * scale;
    Rect::new(
        rect.x + (rect.width - width) / 2.0,
        rect.y + (rect.height - height) / 2.0,
        width,
        height,
    )
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

/// Returns `rect` inset by `by` on every side.
fn inset(rect: Rect, by: f32) -> Rect {
    grow(rect, -by)
}

/// Returns the box inside `rect` that `padding_h` and `padding_v` leave for the
/// label.
///
/// The two paddings are inset on their own axes rather than the larger of the
/// two on both: a button with 8 pixels of horizontal and 4 of vertical padding is
/// a shape, and insetting it by 8 in every direction would draw a label in a box
/// that is not there. The box is floored at zero, because a rect smaller than
/// its own padding would otherwise hand the text a negative width to lay out in.
fn content_box(rect: Rect, padding_h: f32, padding_v: f32) -> Rect {
    Rect::new(
        rect.x + padding_h,
        rect.y + padding_v,
        (rect.width - padding_h * 2.0).max(0.0),
        (rect.height - padding_v * 2.0).max(0.0),
    )
}

/// Returns how far the button is pressed, from the scale it is drawn at: zero
/// at rest and one at [`PRESSED_SCALE`].
///
/// It is derived from the scale rather than held as a property of its own,
/// because the scale is what the task animates and a second property would be a
/// second thing to keep in step with it. A scale above its resting value — which
/// an overshooting easing asks for — reads as no press at all.
fn press_amount(scale: f32) -> f32 {
    ((1.0 - scale) / (1.0 - PRESSED_SCALE)).clamp(0.0, 1.0)
}

/// Returns whether `key` activates the focused control.
///
/// The gamepad's south button is the one an A-capable controller puts its
/// confirm face button on: SDL names the four face buttons by position, and the
/// per-controller label that says which of them is `A` is a live query on a
/// connected gamepad that the input module does not carry.
fn is_activation_key(key: &Key) -> bool {
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
    use crate::layout::{Constraints, Layout, Offset};
    use crate::node;
    use crate::paint::DrawCommand;
    use crate::theme::ThemeToken;
    use std::cell::Cell;
    use std::rc::Rc;

    /// A monospace advance of 5 pixels per character.
    fn mono(_: char) -> f32 {
        5.0
    }

    /// A line 20 pixels tall, so a button's height is padding and this.
    const LINE: f32 = 20.0;

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Advances `button`'s transitions by `millis` and reports whether anything
    /// moved.
    ///
    /// The button's own `tick` is what a frame calls, and it answers whether a
    /// repaint is needed; a test that only wants time to pass goes through here
    /// so that the answer is passed on rather than dropped on the floor.
    fn tick(button: &Button, millis: u64) -> bool {
        button.tick(ms(millis))
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a
    /// value at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// A button in the dark theme's palette, with its appearance already
    /// arrived at that palette.
    ///
    /// Setting a palette does not move the button by itself — a theme switch is
    /// animated — so a test that wants a button *in* a palette asks for the
    /// animation and ticks it out. Otherwise the colours would be the ones
    /// `Button::new` starts with and the palette's would only be targets.
    fn button() -> (Arena<WidgetNode>, Button) {
        let mut nodes = Arena::new();
        let mut button = Button::new(&mut nodes, "Ok");
        button.set_palette(Palette::from_theme(&Theme::dark()));
        button.animate_to_state(motion());
        tick(&button, 100);
        (nodes, button)
    }

    /// A button with the flags for `state` set and nothing else.
    fn in_state(state: ButtonState) -> (Arena<WidgetNode>, Button) {
        let (nodes, button) = button();
        match state {
            ButtonState::Default => {}
            ButtonState::Hovered => button.hovered.set(true),
            ButtonState::Pressed => button.pressed.set(true),
            ButtonState::Disabled => button.disabled.set(true),
            ButtonState::Focused => button.focused.set(true),
        }
        (nodes, button)
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

    /// The text runs a paint recorded, as their x, y and text.
    fn runs(commands: &[DrawCommand]) -> Vec<(f32, f32, String)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { x, y, text, .. } => Some((*x, *y, text.clone())),
                _ => None,
            })
            .collect()
    }

    /// The alpha of the first rounded rectangle in `commands` and of the label.
    ///
    /// **The first rounded rectangle is the focus ring when the button is focused
    /// and the background otherwise**, which is exactly the pair this module's
    /// multiplier tests need to read: a ring that stayed solid while the button
    /// faded would be the same defect one primitive in, and only a helper that
    /// reports both can say whether it does.
    fn alphas(commands: &[DrawCommand]) -> (u8, u8) {
        let shape = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::RoundedRect { color, .. } => Some(color.a),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no rounded rectangle among {commands:?}"));
        let label = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { color, .. } => Some(color.a),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text among {commands:?}"));
        (shape, label)
    }

    /// The alpha `with_opacity` gives a fully opaque colour at `opacity`.
    ///
    /// **The module's own rounding**, through the same `channel` the pipeline uses,
    /// because `255 * 0.5` is 127.5 and the two ways of writing "half of 255" —
    /// `127` from integer division and `128` from the renderer — differ by one, so
    /// an expected value written by hand is a claim about the rounding rather than
    /// about the multiplier.
    fn alpha_at(opacity: f32) -> u8 {
        with_opacity(Color::new(255, 255, 255, 255), opacity).a
    }

    /// A short name for each recorded command, in the order they were recorded.
    ///
    /// The filtering helpers above cannot see order, and order is half of what
    /// a pressed button is: the label has to be drawn *after* the overlay, or the
    /// overlay's shadow falls across the text.
    fn shapes(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::RoundedRect { .. } => "rect",
                DrawCommand::Text { .. } => "text",
                _ => "other",
            })
            .collect()
    }

    /// A tap at the origin, for the tests that only care that there is one.
    fn tap() -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(Offset::new(1.0, 1.0)))
    }

    /// A tap at `(x, y)`.
    fn tap_at(x: f32, y: f32) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)))
    }

    /// Hangs `button` on a new panel of `panel_size` and lays it out, and returns
    /// the panel's handle.
    ///
    /// The panel is a stack, so the button sits at its origin, and the button's
    /// own node is given the rect its touch target asks for. A tap therefore has
    /// a real hit test to pass or fail, and a node behind the button to reach if
    /// it does not.
    fn on_a_panel(button: &Button, nodes: &mut Arena<WidgetNode>, panel_size: Size) -> Handle {
        let panel = node::create(nodes, crate::layout::LayoutState::new());
        assert!(
            node::attach(nodes, panel, button.handle()),
            "the button is hung on the panel"
        );
        let size = button.size(&mono, LINE);
        if let Some(node) = nodes.get_mut(button.handle()) {
            node.layout_mut().set_constraints(Constraints::tight(size));
        }
        Layout::new(nodes).layout(panel, Constraints::tight(panel_size));
        panel
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

    /// A counter a click moves, so a test can see how many times it ran.
    fn counter(button: &mut Button) -> Rc<Cell<u32>> {
        let clicks = Rc::new(Cell::new(0));
        let counted = Rc::clone(&clicks);
        button.on_click = Callback::new(move || counted.set(counted.get() + 1));
        clicks
    }

    #[test]
    fn the_default_padding_and_radius_are_the_theme_tokens_values() {
        // The three constants in this module are copies of theme tokens, and a
        // copy is exactly the thing that drifts. This reads the tokens, so
        // changing `SpacingSm` in the theme without changing `THEME_SPACING_SM`
        // fails here rather than being noticed by eye. Both themes are checked:
        // they hold the same values today, and a divergence is the thing worth
        // catching.
        for theme in [Theme::dark(), Theme::light()] {
            let number = |token| {
                theme
                    .get(token)
                    .as_number()
                    .unwrap_or_else(|| panic!("{token:?} is not a number in the theme"))
            };
            assert_eq!(
                number(crate::theme::ThemeToken::SpacingSm),
                THEME_SPACING_SM,
                "SpacingSm is the default horizontal padding"
            );
            assert_eq!(
                number(crate::theme::ThemeToken::SpacingXs),
                THEME_SPACING_XS,
                "SpacingXs is the default vertical padding"
            );
            assert_eq!(
                number(crate::theme::ThemeToken::BorderRadiusMd),
                THEME_RADIUS_MD,
                "BorderRadiusMd is the default corner radius"
            );
        }
    }

    #[test]
    fn a_button_holds_the_properties_the_task_gives_it() {
        let (nodes, button) = button();
        assert_eq!(button.label.get(), "Ok");
        assert_eq!(
            button.background.get(),
            Palette::from_theme(&Theme::dark()).background
        );
        assert_eq!(
            button.foreground.get(),
            Palette::from_theme(&Theme::dark()).foreground
        );
        assert_eq!(
            button.border_radius.get(),
            8.0,
            "the theme's BorderRadiusMd"
        );
        assert_eq!(button.padding_h.get(), 8.0, "SpacingSm across");
        assert_eq!(button.padding_v.get(), 4.0, "SpacingXs down");
        assert!(
            !button.on_click.is_set(),
            "and no callback until one is given"
        );
        assert!(
            nodes.get(button.handle()).is_some(),
            "and a node of its own"
        );
    }

    #[test]
    fn a_button_starts_at_rest_animated() {
        let (_nodes, button) = button();
        assert_eq!(button.state(), ButtonState::Default);
        assert_eq!(button.scale.get(), 1.0);
        assert_eq!(button.opacity.get(), 1.0);
        assert!(!button.is_animating());
    }

    #[test]
    fn the_primary_state_is_disabled_then_pressed_then_hovered_then_focused() {
        let (_nodes, button) = button();
        assert_eq!(button.state(), ButtonState::Default);
        button.focused.set(true);
        assert_eq!(button.state(), ButtonState::Focused);
        button.hovered.set(true);
        assert_eq!(button.state(), ButtonState::Hovered);
        button.pressed.set(true);
        assert_eq!(button.state(), ButtonState::Pressed);
        button.disabled.set(true);
        assert_eq!(
            button.state(),
            ButtonState::Disabled,
            "disabled is the state that overrides every other"
        );
    }

    #[test]
    fn the_five_states_look_different_from_one_another() {
        // The acceptance criterion is that the states are *distinct*, so this
        // compares every pair rather than each against the resting one: a hover
        // that moved nothing would still differ from a press.
        let states = [
            ButtonState::Default,
            ButtonState::Hovered,
            ButtonState::Pressed,
            ButtonState::Disabled,
            ButtonState::Focused,
        ];
        let styles: Vec<Style> = states
            .iter()
            .map(|&state| in_state(state).1.style())
            .collect();
        for (index, first) in styles.iter().enumerate() {
            for (other, second) in styles.iter().enumerate().skip(index + 1) {
                assert_ne!(
                    first, second,
                    "{:?} and {:?} paint the same thing",
                    states[index], states[other]
                );
            }
        }
    }

    #[test]
    fn a_hovered_button_is_lighter_and_otherwise_unchanged() {
        let (_nodes, button) = button();
        let rest = button.style();
        button.hovered.set(true);
        let hovered = button.style();
        // Every channel moves toward white, so none of them may get darker — but
        // a channel already at 255 has nowhere to go, and a test that demanded it
        // grow would be demanding a highlight the colour cannot take.
        assert!(hovered.background.r >= rest.background.r);
        assert!(hovered.background.g >= rest.background.g);
        assert!(hovered.background.b >= rest.background.b);
        assert!(
            hovered.background != rest.background,
            "and at least one of them moved: {:?} against {:?}",
            hovered.background,
            rest.background
        );
        assert_eq!(hovered.scale, rest.scale, "and the hover does not move it");
        assert_eq!(hovered.ring_width, 0.0);
    }

    #[test]
    fn a_pressed_button_is_darker_and_smaller() {
        let (_nodes, button) = button();
        let rest = button.style();
        button.pressed.set(true);
        let pressed = button.style();
        assert!(pressed.background.r < rest.background.r);
        assert_eq!(pressed.scale, PRESSED_SCALE, "and at 0.95 of its size");
    }

    #[test]
    fn a_disabled_button_is_greyed_faded_and_carries_no_ring() {
        let (_nodes, button) = button();
        let rest = button.style();
        button.focused.set(true);
        button.pressed.set(true);
        button.disabled.set(true);
        let disabled = button.style();
        assert_eq!(
            disabled.ring_width, 0.0,
            "an inert control does not claim focus"
        );
        assert_eq!(disabled.scale, 1.0, "and does not stay shrunk");
        assert_eq!(disabled.opacity, DISABLED_OPACITY, "it fades out");
        assert_ne!(
            disabled.background, rest.background,
            "and it is not the rest colour"
        );
        // Grey means every channel inside the same narrow band, which is what
        // separates a disabled button from a merely darker one.
        let spread = disabled
            .background
            .r
            .max(disabled.background.g)
            .max(disabled.background.b)
            - disabled
                .background
                .r
                .min(disabled.background.g)
                .min(disabled.background.b);
        assert!(
            spread < 40,
            "a disabled button is grey, and its channels are {spread} apart"
        );
    }

    #[test]
    fn a_focused_button_wears_a_ring_and_keeps_its_rest_colour() {
        let (_nodes, button) = button();
        let rest = button.style();
        button.focused.set(true);
        let focused = button.style();
        assert_eq!(focused.ring_width, 2.0, "the ring is as thick as asked");
        assert_eq!(
            focused.background, rest.background,
            "the ring is the focus indicator, not a different fill"
        );
    }

    #[test]
    fn a_focused_and_hovered_button_keeps_both() {
        // The states overlap, which is why they are four booleans and not one
        // enum: the ring and the lighter fill are independent of one another.
        let (_nodes, button) = button();
        let rest = button.style();
        button.focused.set(true);
        button.hovered.set(true);
        let both = button.style();
        assert_eq!(both.ring_width, 2.0);
        assert_ne!(both.background, rest.background);
    }

    #[test]
    fn a_state_change_starts_a_transition_that_arrives() {
        // The contract is that the appearance *moves* rather than jumping, so
        // this checks the middle of the transition and not only its end: a
        // `set` instead of an animation would pass an end-only assertion.
        let (_nodes, button) = button();
        let rest = button.style();
        button.hovered.set(true);
        button.animate_to_state(motion());
        assert_eq!(
            button.background.get(),
            rest.background,
            "the property starts where it was, so a frame drawn now is right"
        );
        tick(&button, 50);
        let half = button.background.get();
        assert_ne!(half, rest.background, "half way through, it has moved");
        assert_ne!(half, button.style().background, "and has not arrived");
        tick(&button, 50);
        assert_eq!(button.background.get(), button.style().background);
        assert!(!button.is_animating());
    }

    #[test]
    fn a_transition_animates_the_scale_and_the_opacity_too() {
        let (_nodes, button) = button();
        button.pressed.set(true);
        button.animate_to_state(motion());
        tick(&button, 50);
        // 0.95 from 1.0, half way along a linear curve over 100 ms.
        assert_eq!(button.scale.get(), 0.975);
        tick(&button, 50);
        assert_eq!(button.scale.get(), PRESSED_SCALE);

        button.disabled.set(true);
        button.animate_to_state(motion());
        tick(&button, 100);
        assert_eq!(button.opacity.get(), DISABLED_OPACITY);
    }

    #[test]
    fn a_transition_takes_the_themes_duration_and_curve() {
        // The task's "duration from theme DurationFast, theme easing curves" is
        // one method reading two tokens, so this is where it is checked.
        let theme = Theme::dark();
        let motion = Motion::from_theme(&theme);
        assert_eq!(
            motion.duration,
            theme.get(ThemeToken::DurationFast).as_duration().unwrap()
        );
        assert_eq!(
            motion.easing,
            theme.get(ThemeToken::EasingStandard).as_easing().unwrap()
        );

        let (_nodes, button) = button();
        button.pressed.set(true);
        button.animate_to_state(motion);
        // Half of 150 ms on EaseInOut is halfway, so the scale is halfway.
        tick(&button, 75);
        assert_eq!(button.scale.get(), 1.0 - (1.0 - PRESSED_SCALE) / 2.0);
    }

    #[test]
    fn a_transition_uses_the_palette_the_button_was_given() {
        let mut nodes = Arena::new();
        let mut button = Button::new(&mut nodes, "Ok");
        button.pressed.set(true);
        button.set_palette(Palette::from_theme(&Theme::light()));
        button.animate_to_state(motion());
        tick(&button, 100);
        assert_eq!(
            button.background.get(),
            button.style().background,
            "and the target came from the palette the caller set"
        );
        assert_eq!(
            button.background.get(),
            shade(
                Palette::from_theme(&Theme::light()).background,
                -PRESS_DARKEN
            )
        );
    }

    #[test]
    fn a_second_transition_replaces_the_first_rather_than_racing_it() {
        // A press released before its transition finished used to be a race:
        // both transitions wrote the background, and the older one arriving last
        // left the button pressed. The button's own clock is what rules that out.
        let (_nodes, button) = button();
        button.pressed.set(true);
        button.animate_to_state(motion());
        tick(&button, 50);
        button.pressed.set(false);
        button.animate_to_state(motion());
        for _ in 0..4 {
            tick(&button, 50);
        }
        assert_eq!(
            button.background.get(),
            button.style().background,
            "the released button ends at the resting colour"
        );
        assert_eq!(button.scale.get(), 1.0);
        assert!(!button.is_animating(), "and nothing is left running");
    }

    #[test]
    fn setting_the_palette_leaves_the_button_where_it_is() {
        // A theme switch is animated by the caller setting the palette and
        // animating toward it; a palette that moved the button on its own would
        // make every theme switch instantaneous.
        let (_nodes, mut button) = button();
        let before = button.background.get();
        button.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(button.background.get(), before);
    }

    #[test]
    fn snapping_puts_a_themed_button_where_its_theme_says_at_once() {
        // The failure this guards: a button given a palette but never aimed
        // still paints the neutral defaults `Button::new` wrote, so a themed
        // button starts out grey and only becomes the theme's colour once
        // something has moved it. Built from `new` rather than through the
        // `button` helper, which aims and ticks.
        let mut nodes = Arena::new();
        let mut button = Button::new(&mut nodes, "Ok");
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(
            themed.background,
            button.background.get(),
            "the theme's Primary is not the neutral default, so this can fail"
        );

        button.set_palette(themed);
        button.snap_to_state();

        assert_eq!(button.background.get(), themed.background);
        assert_eq!(button.foreground.get(), themed.foreground);
        assert_eq!(button.scale.get(), 1.0);
        assert_eq!(button.opacity.get(), 1.0);
        assert!(
            !button.is_animating(),
            "a snap is not a transition, so nothing is left running"
        );
    }

    #[test]
    fn snapping_takes_the_button_straight_to_its_current_state() {
        // The other reason to snap: a caller that has written a state property
        // and wants the button to be that state now.
        let (_nodes, button) = button();
        button.pressed.set(true);
        button.snap_to_state();

        assert_eq!(button.scale.get(), PRESSED_SCALE);
        assert_eq!(button.background.get(), button.style().background);
    }

    #[test]
    fn snapping_a_mid_press_button_back_out_ends_the_transition() {
        // Without the `clear`, a transition already running writes over what the
        // snap just set when it arrives, and the button drifts off again.
        let (_nodes, button) = button();
        button.pressed.set(true);
        button.animate_to_state(Motion {
            duration: Duration::from_millis(150),
            easing: Easing::EaseInOut,
        });
        assert!(
            button.tick(Duration::from_millis(10)),
            "the press is animating"
        );

        button.pressed.set(false);
        button.snap_to_state();
        assert_eq!(button.scale.get(), 1.0, "the snap put it at rest");

        assert!(
            !button.tick(Duration::from_millis(500)),
            "and nothing arrives afterwards to move it off again"
        );
        assert_eq!(button.scale.get(), 1.0);
    }

    #[test]
    fn a_button_paints_its_background_and_its_label() {
        let (_nodes, button) = button();
        let rect = Rect::new(10.0, 20.0, 100.0, 44.0);
        let commands = button.paint(rect, &mono, LINE);

        let filled = rounded(&commands);
        assert_eq!(filled.len(), 1, "one background, and no ring or overlay");
        assert_eq!(filled[0].0, rect, "at the rect it was given");
        assert_eq!(filled[0].1, 8.0, "with the radius it was given");
        assert_eq!(filled[0].2, button.background.get());

        assert_eq!(
            runs(&commands),
            vec![(55.0, 32.0, "Ok".to_string())],
            "the label, centred in the 100×44 button"
        );
    }

    #[test]
    fn a_button_centres_its_label_in_the_box_its_padding_leaves() {
        // The padding is 8 either side and 4 above and below, so the text box is
        // 84 wide inside a 100-wide button, starting 8 in: "Ok" is 10 wide, so it
        // sits (84 - 10) / 2 = 37 further along, at 8 + 10 + 37 = 55. Centring
        // in the padded box lands on the button's own centre, 10 + (100 - 10) / 2,
        // which is the point of padding on both sides equally.
        let (_nodes, button) = button();
        let commands = button.paint(Rect::new(10.0, 20.0, 100.0, 44.0), &mono, LINE);
        assert_eq!(runs(&commands), vec![(55.0, 32.0, "Ok".to_string())]);
    }

    #[test]
    fn the_label_stays_centred_whatever_the_padding() {
        // The padding is symmetric, so it moves the box the label is centred in
        // and not the label's own position: the centre of the text is the centre
        // of the button, for any padding at all. "Ok" is 10 wide, so the text
        // starts 5 left of the button's centre.
        let (_nodes, button) = button();
        let rect = Rect::new(20.0, 30.0, 120.0, 44.0);
        for padding in [0.0, 8.0, 20.0, 40.0] {
            button.padding_h.set(padding);
            let painted = runs(&button.paint(rect, &mono, LINE));
            assert_eq!(
                painted,
                vec![(75.0, 42.0, "Ok".to_string())],
                "{padding} pixels of padding either side"
            );
        }
    }

    #[test]
    fn a_label_wider_than_the_button_starts_at_the_padded_edge() {
        // A label too long for the button is neither wrapped nor cut — the button
        // says so, and the text runs past the edges instead. What it does is start
        // where the padding leaves it, rather than being centred on a box narrower
        // than the text.
        let mut nodes = Arena::new();
        let button = Button::new(&mut nodes, "0123456789");
        let painted = runs(&button.paint(Rect::new(0.0, 0.0, 60.0, 44.0), &mono, LINE));
        assert_eq!(painted, vec![(8.0, 12.0, "0123456789".to_string())]);
    }

    #[test]
    fn a_focused_button_paints_a_ring_outside_its_background() {
        let (_nodes, button) = button();
        button.focus_ring.set(2.0);
        button.focused.set(true);
        let commands = button.paint(Rect::new(0.0, 0.0, 100.0, 44.0), &mono, LINE);

        let filled = rounded(&commands);
        assert_eq!(
            filled.len(),
            2,
            "the ring, then the background on top of it"
        );
        assert_eq!(
            filled[0].0,
            Rect::new(-2.0, -2.0, 104.0, 48.0),
            "the ring is the background grown by its own width, so it shows"
        );
        assert_eq!(filled[0].2, button.palette().ring);
        assert_eq!(filled[1].0, Rect::new(0.0, 0.0, 100.0, 44.0));
    }

    #[test]
    fn a_zero_width_ring_is_not_painted() {
        // A theme that sets the ring's width to zero turns the indicator off;
        // painting a zero-width ring would be a command that draws nothing.
        let (_nodes, button) = button();
        button.focus_ring.set(0.0);
        button.focused.set(true);
        let commands = button.paint(Rect::new(0.0, 0.0, 100.0, 44.0), &mono, LINE);
        assert_eq!(rounded(&commands).len(), 1);
    }

    #[test]
    fn a_pressed_button_paints_smaller_about_its_centre() {
        // The press scales what is drawn and not the node, so nothing around the
        // button moves: the same centre, five per cent less of each dimension.
        let (_nodes, button) = button();
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        button.scale.set(PRESSED_SCALE);
        let filled = rounded(&button.paint(rect, &mono, LINE));
        let (drawn, _, _) = filled[0];
        assert_eq!(drawn.width, 95.0);
        assert_eq!(drawn.height, 38.0);
        assert_eq!(drawn.x, 2.5, "and it is still centred on the same point");
        assert_eq!(drawn.y, 1.0);
    }

    #[test]
    fn the_pressed_overlay_follows_the_scale_and_vanishes_at_rest() {
        let (_nodes, button) = button();
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        assert_eq!(
            rounded(&button.paint(rect, &mono, LINE)).len(),
            1,
            "no overlay at rest"
        );

        // Half way down a press transition the scale is 0.975, so the press is
        // (1 - 0.975) / (1 - 0.95) = 0.5 of the way in, and the overlay is half
        // of `PRESS_SHADOW_ALPHA` of opaque black: 0.5 * 0.28 * 255 = 35.7,
        // which the channel rounding puts at 36.
        button.scale.set(0.975);
        let filled = rounded(&button.paint(rect, &mono, LINE));
        assert_eq!(filled.len(), 2, "the overlay is painted while pressing");
        let (_, _, color) = filled[1];
        assert_eq!(color.a, 36, "at half strength, half of the shadow's alpha");
        assert_eq!(color.r, 0, "and it is black");

        // An overshooting easing takes the scale above rest on the way back; that
        // is no press at all, not a shadow in the wrong direction.
        button.scale.set(1.02);
        assert_eq!(rounded(&button.paint(rect, &mono, LINE)).len(), 1);
    }

    #[test]
    fn a_pressed_button_paints_its_label_after_the_shadow_that_covers_it() {
        // The other half of the defect the alpha fixes, and the half no existing
        // assertion could see: `rounded` and `runs` both filter, so every test
        // in this module was blind to the order the commands are recorded in.
        // Moving the label's text loop above the overlay — so the shadow falls
        // across the text instead of under it — left the whole suite green.
        //
        // A painter draws in order and the renderer submits in order, so the
        // label must come last: it is the thing the shadow must not touch.
        let (_nodes, button) = button();
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        button.scale.set(PRESSED_SCALE);
        let commands = button.paint(rect, &mono, LINE);

        assert_eq!(
            shapes(&commands),
            vec!["rect", "rect", "text"],
            "the background, the shadow over it, and the label on top of both"
        );
        // Which text it is, and in what colour. The position is deliberately not
        // asserted: where the label lands is another test's job, and a pressed
        // button's label is at different coordinates from a resting one's, which
        // is what made the first version of this assertion wrong.
        let Some(DrawCommand::Text { text, color, .. }) = commands.last() else {
            panic!("the last command painted is not the label");
        };
        assert_eq!(text, "Ok");
        assert_eq!(*color, button.foreground.get());
    }

    #[test]
    fn a_resting_button_paints_its_background_and_then_its_label() {
        // The same order without the shadow, so the two tests together pin the
        // sequence rather than only the pressed case.
        let (_nodes, button) = button();
        let commands = button.paint(Rect::new(0.0, 0.0, 100.0, 40.0), &mono, LINE);

        assert_eq!(shapes(&commands), vec!["rect", "text"]);
    }

    #[test]
    fn a_focused_button_paints_its_ring_first_and_its_label_last() {
        // The ring is the one command that is *meant* to sit under the label, so
        // the sequence for a focused button is the same shape with a longer
        // prefix.
        let (_nodes, button) = button();
        button.focused.set(true);
        let commands = button.paint(Rect::new(0.0, 0.0, 100.0, 40.0), &mono, LINE);

        assert_eq!(
            shapes(&commands),
            vec!["rect", "rect", "text"],
            "the ring, the background over it, the label on top of both"
        );
    }

    #[test]
    fn a_fully_pressed_button_is_shadowed_not_blacked_out() {
        // The defect a capture found and no draw-command assertion could: the
        // overlay used to interpolate to opaque black, so a pressed button was a
        // solid black rectangle with the label invisible on it — the label is
        // drawn after the overlay, and in the dark theme it is `OnPrimary`,
        // which is black. "A rounded rect of near-black at some alpha" is what a
        // test sees either way, so the assertion has to be on the number.
        let (_nodes, button) = button();
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        button.scale.set(PRESSED_SCALE);
        let filled = rounded(&button.paint(rect, &mono, LINE));

        assert_eq!(filled.len(), 2, "the background and its shadow");
        let background = filled[0].2;
        let shadow = filled[1].2;
        // Not `shadow.a <= PRESS_SHADOW_ALPHA * 255`: the alpha is *computed*
        // from that constant, so such an assertion is true by construction and
        // survives setting the constant to 1.0 — the 2026-09-29 entry in
        // `.ai/NEVERAGAIN.md` exactly. The number is written out instead, so
        // that a change to either the constant or the path that produces the
        // alpha has to move this.
        assert!(
            shadow.a < 128,
            "a fully pressed button is shadowed, not blacked out: the overlay's \
             alpha is {} and may not reach opaque",
            shadow.a
        );
        // The number that matters is not the alpha but what is left of the
        // background underneath: a quarter of its light survives the shadow.
        let blended = Color::interpolate(&background, &shadow, 0.35);
        assert!(
            blended.r > background.r / 2,
            "the background is still itself under the shadow: {background:?} then \
             {blended:?}"
        );
    }

    #[test]
    fn a_pressed_overlay_is_inset_so_it_reads_as_depth() {
        let (_nodes, button) = button();
        button.scale.set(PRESSED_SCALE);
        let filled = rounded(&button.paint(Rect::new(0.0, 0.0, 100.0, 40.0), &mono, LINE));
        let (shadow, radius, _) = filled[1];
        assert_eq!(shadow.width, 95.0 - PRESS_SHADOW_INSET * 2.0);
        assert_eq!(shadow.height, 38.0 - PRESS_SHADOW_INSET * 2.0);
        assert_eq!(
            radius,
            8.0 - PRESS_SHADOW_INSET,
            "and its corners follow the background's, inwards"
        );
    }

    #[test]
    fn a_disabled_button_paints_its_colours_faded() {
        let (_nodes, button) = button();
        button.disabled.set(true);
        button.animate_to_state(motion());
        tick(&button, 100);
        let filled = rounded(&button.paint(Rect::new(0.0, 0.0, 100.0, 44.0), &mono, LINE));
        assert_eq!(filled.len(), 1, "and no ring, no overlay");
        assert_eq!(
            filled[0].2.a, 115,
            "the background is drawn at {} of its alpha",
            filled[0].2.a
        );

        let text = runs(&button.paint(Rect::new(0.0, 0.0, 100.0, 44.0), &mono, LINE));
        assert!(!text.is_empty(), "the label is still drawn, faded too");
        let faded = button
            .paint(Rect::new(0.0, 0.0, 100.0, 44.0), &mono, LINE)
            .into_iter()
            .find_map(|command| match command {
                DrawCommand::Text { color, .. } => Some(color),
                _ => None,
            })
            .unwrap();
        assert!(faded.a < 255, "at {} rather than opaque", faded.a);
    }

    #[test]
    fn an_empty_label_still_paints_its_background() {
        let mut nodes = Arena::new();
        let button = Button::new(&mut nodes, "");
        let commands = button.paint(Rect::new(0.0, 0.0, 100.0, 44.0), &mono, LINE);
        assert_eq!(rounded(&commands).len(), 1);
        // An empty line is laid out and painted like any other: a zero-width line
        // centred in the 84-pixel box starts 42 in, and the box starts 8 in.
        assert_eq!(runs(&commands), vec![(50.0, 12.0, String::new())]);
    }

    #[test]
    fn a_button_sizes_to_its_label_and_its_padding() {
        // "Ok" is 10 wide and one 20-pixel line tall: 10 + 8 + 8 = 26 across,
        // 20 + 4 + 4 = 28 down.
        let (_nodes, button) = button();
        assert_eq!(button.content_size(&mono, LINE), Size::new(26.0, 28.0));
    }

    #[test]
    fn a_button_never_smaller_than_the_minimum_touch_target() {
        // A 26-pixel button is not a target a finger can be asked to hit, so the
        // node is floored even though the content is smaller than the floor.
        let (_nodes, button) = button();
        assert_eq!(button.size(&mono, LINE), Size::new(44.0, 44.0));
    }

    #[test]
    fn a_button_larger_than_the_minimum_keeps_its_content_size() {
        // The floor is a floor and not a size: a long label still gets the width
        // it needs, and only the axis under the floor is lifted.
        let mut nodes = Arena::new();
        let button = Button::new(&mut nodes, "A considerably longer label");
        let content = button.content_size(&mono, LINE);
        let size = button.size(&mono, LINE);
        assert_eq!(size.width, content.width, "the label's width is kept");
        assert!(size.width > MIN_TOUCH_TARGET);
        assert_eq!(
            size.height, MIN_TOUCH_TARGET,
            "while the height, one short line plus padding, is lifted to the floor"
        );
    }

    #[test]
    fn the_minimum_floor_is_44_in_both_dimensions() {
        // 44 is the number the task gives, and it is a constant rather than a
        // derived figure, so it is worth one test that would notice a change.
        let mut nodes = Arena::new();
        let button = Button::new(&mut nodes, "");
        assert_eq!(MIN_TOUCH_TARGET, 44.0);
        assert_eq!(button.size(&mono, 0.0), Size::new(44.0, 44.0));
    }

    #[test]
    fn a_negative_line_height_does_not_shrink_the_button() {
        // A negative line height would make the content smaller than the padding
        // that is added to it, which is a caller that lost track of a font.
        let (_nodes, button) = button();
        assert_eq!(
            button.content_size(&mono, -10.0),
            Size::new(26.0, 8.0),
            "the line is treated as no line, and the padding is all that is left"
        );
    }

    #[test]
    fn a_tap_fires_the_click_and_is_consumed() {
        // The button is a child of a panel that also records what it was sent, so
        // "consumed" is checked against a real node behind it: a button that did
        // not consume would have let the tap on up to the panel.
        let (mut nodes, mut button) = button();
        let clicks = counter(&mut button);
        let panel = on_a_panel(&button, &mut nodes, Size::new(200.0, 200.0));

        let mut event = tap();
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == button.handle() {
                button.on_event(event);
            }
        });
        assert_eq!(clicks.get(), 1, "the tap fired the click");
        assert!(event.consumed(), "and was consumed");
        assert_eq!(
            reached,
            vec![button.handle()],
            "so the panel behind it never saw the tap"
        );
    }

    #[test]
    fn a_tap_outside_the_button_reaches_the_panel_instead() {
        // The other half of the previous test: a tap that misses the button's own
        // 44×44 rect is not its to consume, and the button never fires.
        let (mut nodes, mut button) = button();
        let clicks = counter(&mut button);
        let panel = on_a_panel(&button, &mut nodes, Size::new(200.0, 200.0));

        let mut event = tap_at(150.0, 150.0);
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == button.handle() {
                button.on_event(event);
            }
        });
        assert_eq!(reached, vec![panel], "the panel took it");
        assert_eq!(clicks.get(), 0, "and the button never fired");
        assert!(!event.consumed());
    }

    #[test]
    fn a_disabled_button_swallows_a_tap_without_firing() {
        // A tap aimed at an inert control must not carry on to the panel behind
        // it, so the event is consumed even though nothing happens.
        let (_nodes, mut button) = button();
        let clicks = counter(&mut button);
        button.disabled.set(true);
        let mut event = tap();
        assert!(button.on_event(&mut event));
        assert_eq!(clicks.get(), 0, "a disabled button does not fire");
        assert!(event.consumed(), "but it does swallow the tap");
    }

    #[test]
    fn a_button_with_no_callback_reports_that_it_did_nothing() {
        let (_nodes, button) = button();
        assert!(!button.activate(), "there is nothing to run");
        let mut event = tap();
        assert!(button.on_event(&mut event), "and the tap is still consumed");
    }

    #[test]
    fn an_activation_key_fires_a_focused_button() {
        for key in [
            Key::Keyboard(sdl3::keyboard::Keycode::Return),
            Key::Keyboard(sdl3::keyboard::Keycode::KpEnter),
            Key::Keyboard(sdl3::keyboard::Keycode::Space),
            Key::Gamepad(sdl3::gamepad::Button::South),
        ] {
            let (_nodes, mut button) = button();
            let clicks = counter(&mut button);
            button.focused.set(true);
            let mut event = key_down(key);
            assert!(button.on_event(&mut event), "{key:?} activates the button");
            assert_eq!(clicks.get(), 1, "{key:?} fired the click once");
            assert!(event.consumed(), "{key:?} was consumed");
        }
    }

    #[test]
    fn the_ring_follows_focused_and_the_activation_follows_activatable() {
        // **The two-flag matrix, all four quadrants, each with its controls.**
        // `focused` and `activatable` are independent, so the claim is not "the
        // property works" but "the ring and the key answer to different bits" —
        // and the quadrant that proves it is `focused` and not `activatable`,
        // which is a dialog's action button for the whole 300 ms of a dismissal.
        // A test of the other three quadrants could not tell one bit wearing two
        // hats from two bits.
        //
        // The ring is read off the recorded commands and the activation off a
        // counter, so neither half is asserted through the flag that drives it.
        for (focused, activatable) in [(true, true), (true, false), (false, true), (false, false)] {
            let case = format!("focused {focused}, activatable {activatable}");
            let (_nodes, mut button) = button();
            let clicks = counter(&mut button);
            button.focused.set(focused);
            button.activatable.set(activatable);

            // **The ring**, and its alpha.
            let shapes = rounded(&button.paint(Rect::new(0.0, 0.0, 44.0, 44.0), &mono, LINE));
            if focused {
                assert_eq!(shapes.len(), 2, "{case}: the ring and the background");
                assert_eq!(
                    shapes[0].1,
                    THEME_RADIUS_MD + 2.0,
                    "{case}: the ring is the wider of the two"
                );
                assert_eq!(shapes[0].2.a, 255, "{case}: and it is opaque");
            } else {
                assert_eq!(
                    shapes.len(),
                    1,
                    "{case}: no ring at all, only the background — whatever \
                     `activatable` says"
                );
            }

            // **Activation by an activation key**, which is the route a dismissed
            // dialog has to refuse.
            let mut event = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
            let consumed = button.on_event(&mut event);
            let after_key = clicks.get();
            assert_eq!(
                consumed, focused,
                "{case}: the key is consumed by a focused button"
            );
            assert_eq!(
                after_key,
                u32::from(focused && activatable),
                "{case}: and fires the click only when it may also be activated"
            );

            // **Activation by a tap**, which is the other arm and the reason the
            // property gates it too: an unfocused button is still tappable, so a
            // `focused`-only gate would leave a withdrawn button live to a finger.
            let mut tap = tap();
            button.on_event(&mut tap);
            assert_eq!(
                clicks.get() - after_key,
                u32::from(activatable),
                "{case}: a tap fires it exactly when it may be activated"
            );
        }
    }

    #[test]
    fn a_button_written_only_with_focused_still_activates_and_still_draws_its_ring() {
        // **The backwards-compatibility guard, written before the property it
        // guards exists.** `activatable` is a new public field, and the one way
        // to add one without breaking every caller is a default of `true` — so
        // this test is the whole of the argument, and it is deliberately written
        // against a button whose *only* written state is `focused`, which is what
        // every caller in this tree does today.
        //
        // If it fails, the default is wrong. A default of `false` would pass this
        // file's other activation tests (they set `focused` and nothing else, but
        // they would be asserting the new behaviour) and silently strip the ring
        // and the activation from every button a caller builds, because absence of
        // a blocker is permission.
        let (_nodes, mut button) = button();
        let clicks = counter(&mut button);
        button.focused.set(true);

        // **Activation**, by a counter rather than by a flag.
        let mut event = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(
            button.on_event(&mut event),
            "a fresh button answers Enter, as it did before the property existed"
        );
        assert_eq!(
            clicks.get(),
            1,
            "and fires its click exactly once: absence of a blocker is permission"
        );

        // **The ring**, on the recorded commands. It is the first rounded
        // rectangle and it is the wider one: `focus_ring`'s default of 2.0 grown
        // onto `THEME_RADIUS_MD`, so 10.0 where the background is 8.0.
        let commands = button.paint(Rect::new(0.0, 0.0, 44.0, 44.0), &mono, LINE);
        let shapes = rounded(&commands);
        assert_eq!(shapes.len(), 2, "the ring and the background");
        assert_eq!(
            shapes[0].1,
            THEME_RADIUS_MD + 2.0,
            "the ring is the outer of the two, its radius grown by the focus ring's \
             thickness"
        );
        assert_eq!(shapes[0].2.a, 255, "and it is opaque");
    }

    #[test]
    fn an_activation_key_does_nothing_to_an_unfocused_button() {
        // A key press is not routed by position, so it reaches the button only
        // because the caller sent it to the focused node. Every button would
        // otherwise fire on every Enter.
        let (_nodes, mut button) = button();
        let clicks = counter(&mut button);
        let mut event = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(!button.on_event(&mut event));
        assert_eq!(clicks.get(), 0);
        assert!(!event.consumed(), "and the key carries on to the tree");
    }

    #[test]
    fn a_key_that_is_not_an_activation_key_is_left_alone() {
        // Tab moves focus and is the focus tracker's business, not the
        // button's; the button must not swallow it.
        let (_nodes, mut button) = button();
        let clicks = counter(&mut button);
        button.focused.set(true);
        for key in [
            Key::Keyboard(sdl3::keyboard::Keycode::Tab),
            Key::Keyboard(sdl3::keyboard::Keycode::A),
            Key::Gamepad(sdl3::gamepad::Button::East),
        ] {
            let mut event = key_down(key);
            assert!(!button.on_event(&mut event), "{key:?} is not activation");
            assert!(!event.consumed());
        }
        assert_eq!(clicks.get(), 0);
    }

    #[test]
    fn a_key_release_does_not_activate() {
        // Activation is on the press, so a held key does not fire again when it
        // comes up.
        let (_nodes, mut button) = button();
        let clicks = counter(&mut button);
        button.focused.set(true);
        let mut event = InputEvent::new(
            InputEventKind::KeyUp {
                key: Key::Keyboard(sdl3::keyboard::Keycode::Return),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        );
        assert!(!button.on_event(&mut event));
        assert_eq!(clicks.get(), 0);
    }

    #[test]
    fn a_disabled_focused_button_ignores_an_activation_key() {
        let (_nodes, mut button) = button();
        let clicks = counter(&mut button);
        button.focused.set(true);
        button.disabled.set(true);
        let mut event = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(button.on_event(&mut event), "the key is still consumed");
        assert_eq!(clicks.get(), 0, "but an inert button does not fire");
    }

    #[test]
    fn a_callback_with_nothing_in_it_does_nothing_without_failing() {
        // The two states a callback can be in, and the one that matters is that
        // the empty one is a no-op rather than a panic.
        let unset = Callback::none();
        assert!(!unset.is_set());
        unset.call(());
        let set = Callback::new(|| {});
        assert!(set.is_set());
        set.call(());
    }

    #[test]
    fn a_palette_is_the_themes_primary_and_on_primary() {
        let palette = Palette::from_theme(&Theme::dark());
        assert_eq!(
            palette.background,
            Theme::dark().get(ThemeToken::Primary).as_color().unwrap()
        );
        assert_eq!(
            palette.foreground,
            Theme::dark().get(ThemeToken::OnPrimary).as_color().unwrap()
        );
        assert_eq!(palette.ring, palette.foreground);
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        // A theme switch has to reach the buttons, or the theme would only
        // change half the window.
        assert_ne!(
            Palette::from_theme(&Theme::dark()),
            Palette::from_theme(&Theme::light())
        );
    }

    #[test]
    fn a_theme_that_holds_the_wrong_kind_of_value_still_answers() {
        // A token written with the wrong variant is a caller error, and it must
        // not take the frame down: the fallbacks are the values both themes hold
        // anyway.
        let theme = Theme::dark();
        theme.set(
            ThemeToken::Primary,
            crate::theme::PropertyValue::Text("x".to_string()),
        );
        theme.set(
            ThemeToken::DurationFast,
            crate::theme::PropertyValue::Number(1.0),
        );
        assert_eq!(
            Palette::from_theme(&theme).background,
            Color::new(0, 0, 0, 255)
        );
        assert_eq!(Motion::from_theme(&theme).duration, ms(150));
    }

    /// **`multiplier` is a multiplier and not a replacement.** A button whose own
    /// [`opacity`](Button::opacity) is below one, painted through
    /// [`Button::paint_faded`], must come out at the **product** — and the two
    /// readings are identical whenever the button is enabled, so this is the only
    /// test that can tell them apart.
    ///
    /// The button's own opacity is set directly rather than by
    /// [`Button::snap_to_state`] on a `disabled` flag, so the two factors are
    /// chosen here and the arithmetic is the claim: 0.4 by 0.5 is 0.2, and
    /// `alpha_at(0.2)` is a number neither `alpha_at(0.4)` nor `alpha_at(0.5)` is.
    #[test]
    fn the_pressed_overlay_fades_with_the_multiplier_too() {
        // The doc on `paint_faded` claims every colour goes through the opacity,
        // and the pressed overlay is the one that did not: its alpha came straight
        // from `press`. A test that only reads the background and the label cannot
        // see that, because it never looks at the second rounded rect.
        let (_nodes, button) = button();
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        button.scale.set(PRESSED_SCALE);

        let overlay_alpha = |multiplier: f32| {
            let commands = button.paint_faded(rect, &mono, LINE, multiplier);
            let overlays: Vec<u8> = rounded(&commands)
                .into_iter()
                .map(|(_, _, color)| color.a)
                .collect();
            assert_eq!(
                overlays.len(),
                2,
                "the background and the overlay, at multiplier {multiplier}"
            );
            overlays[1]
        };

        let full = overlay_alpha(1.0);
        let half = overlay_alpha(0.5);
        assert!(
            half < full,
            "the overlay fades with what it is drawn inside: {full} at full \
             strength, {half} at half"
        );
        // Not `half == full / 2`: both numbers are *computed* from that product,
        // so such an assertion is true by construction and survives dropping the
        // `* opacity` term entirely -- the 2026-09-29 entry in
        // `.ai/NEVERAGAIN.md`. The numbers are written out instead, so that a
        // change to the constant or to the path producing the alpha has to move
        // one of them.
        assert_eq!(
            full, 71,
            "a fully pressed button at full strength: 0.28 * 255 = 71.4"
        );
        assert_eq!(
            half, 36,
            "and at half strength the multiplier halves the product: \
             0.28 * 0.5 * 255 = 35.7, which the channel rounding puts at 36 -- \
             the same number a half-*pressed* button gets at full strength, \
             because the press and the multiplier enter as one product"
        );
    }

    #[test]
    fn the_multiplier_scales_the_buttons_own_opacity_rather_than_replacing_it() {
        let mut nodes = Arena::new();
        let button = Button::new(&mut nodes, "OK");
        button.opacity.set(0.4);
        let rect = Rect::new(0.0, 0.0, 44.0, 44.0);

        let product = alphas(&button.paint_faded(rect, &mono, LINE, 0.5));
        let own_alone = button.paint(rect, &mono, LINE);

        assert_eq!(
            product,
            (alpha_at(0.2), alpha_at(0.2)),
            "0.4 by 0.5 is 0.2, and both the background and the label are there"
        );
        assert_eq!(
            product.0,
            alpha_at(0.2),
            "which is neither factor alone: the button's own 0.4 would give {} and \
             the multiplier's 0.5 would give {}",
            alpha_at(0.4),
            alpha_at(0.5)
        );
        // **Neither factor alone**, on the numbers. The three cases are three
        // different alphas, which is what makes the product assertion a measurement
        // rather than a tautology: a *replacement* implementation would answer
        // `alpha_at(0.5)` here and a *pass-through* one `alpha_at(0.4)`, and only
        // the product answers `alpha_at(0.2)`.
        //
        // **Both the button's own and the multiplier are read through the same two
        // factors rather than through two widgets**, because `multiplier_alone` is
        // the very call `product` was measured from — an earlier version of this
        // line compared the two and would have compared a value with itself.
        assert_eq!(
            (alphas(&own_alone).0, product.0),
            (alpha_at(0.4), alpha_at(0.2)),
            "the button's own opacity alone would be the first of those"
        );
        assert_ne!(
            product.0,
            alpha_at(0.5),
            "and the multiplier alone is not it"
        );
        assert_ne!(
            product.0,
            alpha_at(0.4),
            "nor is the button's own — which is what a replacement would give"
        );

        // `paint` is `paint_faded` with a multiplier of one, so an unfaded button
        // through the new path is the same commands — asserted rather than assumed,
        // because "it delegates" is a claim about the body and this is about the
        // bytes it produces.
        assert_eq!(
            button.paint_faded(rect, &mono, LINE, 1.0),
            own_alone,
            "a multiplier of one is `paint`, command for command"
        );
    }

    /// **The focus ring fades with the multiplier, verified and not assumed.**
    ///
    /// [`Button::paint`] computes the ring's colour from the same `opacity` local as
    /// every other colour, so it should follow for free — and "should follow for
    /// free" is exactly the kind of claim `.ai/NEVERAGAIN.md` says nobody re-checks.
    /// A ring left solid on a button that is fading is the same defect the dialog
    /// had, one primitive in and with no other test to catch it.
    #[test]
    fn a_focus_ring_fades_with_the_multiplier_and_not_only_the_label() {
        let mut nodes = Arena::new();
        let button = Button::new(&mut nodes, "OK");
        button.focused.set(true);
        button.focus_ring.set(2.0);
        let rect = Rect::new(0.0, 0.0, 44.0, 44.0);

        // With the button focused the first rounded rectangle **is** the ring, so
        // `alphas` is reading the ring rather than the background. Asserted, because
        // a helper that silently read the background would make this test pass for
        // the wrong reason.
        let painted = button.paint_faded(rect, &mono, LINE, 0.5);
        assert_eq!(
            rounded(&painted).len(),
            2,
            "a focused button draws a ring and then its background"
        );
        assert!(
            rounded(&painted)[0].1 > rounded(&painted)[1].1,
            "and the ring is the wider of the two, so it is the one read first"
        );

        assert_eq!(
            alphas(&painted),
            (alpha_at(0.5), alpha_at(0.5)),
            "the ring and the label are both at half"
        );
        assert_eq!(
            alphas(&button.paint(rect, &mono, LINE)),
            (alpha_at(1.0), alpha_at(1.0)),
            "and both at full strength with a multiplier of one, which is the \
             control: a ring that ignored the multiplier would pass the half \
             assertion and fail this one"
        );
    }
}
