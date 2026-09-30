//! Theming as a property source.
//!
//! Owns the theme, the tokens it exposes, and the data behind them.
//!
//! A theme is one [`Property`] per token, and a token is one named value a
//! widget can reach: a colour, a spacing, a font size, a duration. A widget
//! that wants the background colour binds a property to the theme's
//! background property, and the property graph carries every change to it — no
//! widget tree rebuild, and no widget knowing that a theme exists beyond the
//! one token it reads.
//!
//! Switching theme is [`Theme::switch_to`]: every token's property is animated
//! from the value it holds to the new theme's, so each dependent is recomputed
//! on every frame of the transition rather than once at its end. The clock that
//! drives those animations is the theme's own, ticked by whoever owns the
//! theme.
//!
//! # Sharing a theme
//!
//! A caller that binds to a theme and ticks it too shares one theme behind
//! [`std::rc::Rc`] and [`std::cell::RefCell`]. The binds capture the theme's
//! *properties* — the clones [`Theme::property`] returns — rather than the
//! shared handle, so that a tick's recompute never borrows the theme while the
//! caller holds it borrowed.
//!
//! # Examples
//!
//! A widget's background follows the theme it is bound to:
//!
//! ```
//! use ui_core::property::Property;
//! use ui_core::theme::{Theme, ThemeToken, PropertyValue};
//!
//! let theme = Theme::dark();
//! let background = Property::bind(move || theme.get(ThemeToken::Background));
//!
//! assert!(matches!(background.get(), PropertyValue::Color(_)));
//! ```
//!
//! Switching to the light theme animates every token over the theme's own
//! clock, and the bound property follows:
//!
//! ```
//! use std::time::Duration;
//! use ui_core::property::Property;
//! use ui_core::theme::{Theme, ThemeToken};
//!
//! let theme = Theme::dark();
//! let background = {
//!     let background_prop = theme.property(ThemeToken::Background);
//!     Property::bind(move || background_prop.get())
//! };
//!
//! theme.switch_to(Theme::light(), 100);
//! let dark = background.get();
//!
//! theme.tick(Duration::from_millis(50));
//! let middle = background.get();
//! assert_ne!(dark, middle, "half way through, the colour has moved");
//!
//! theme.tick(Duration::from_millis(50));
//! assert_eq!(background.get(), Theme::light().get(ThemeToken::Background));
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use crate::animation::{AnimationClock, Easing, Interpolate};
use crate::property::{Color, Property};

/// A theme token: one named value a theme exposes.
///
/// The tokens are the ones `doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Theme tokens*
/// lists, in the order that document writes them: colours, spacing, typography,
/// shape, motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeToken {
    /// The colour behind everything else.
    Background,
    /// The colour a raised surface — a card, a pad — is painted with.
    Surface,
    /// The colour of the primary action and its accents.
    Primary,
    /// The colour drawn on top of [`Primary`](ThemeToken::Primary).
    OnPrimary,
    /// The colour of body text.
    Text,
    /// The colour of text that is present but not emphasised.
    TextMuted,
    /// The colour of hairlines and outlines.
    Border,
    /// The colour of a failure.
    Error,
    /// The colour of a warning.
    Warning,
    /// The colour of a success.
    Success,
    /// The tightest gap between two related elements.
    SpacingXs,
    /// A small gap.
    SpacingSm,
    /// The default gap.
    SpacingMd,
    /// A large gap.
    SpacingLg,
    /// The loosest gap, between unrelated groups.
    SpacingXl,
    /// The font family every token that does not override it inherits.
    FontFamily,
    /// The smallest font size: footnotes, captions.
    FontSizeXs,
    /// A small font size: secondary text.
    FontSizeSm,
    /// The default font size: body text.
    FontSizeMd,
    /// A large font size: subheadings.
    FontSizeLg,
    /// The largest font size: headings.
    FontSizeXl,
    /// The weight of body text.
    FontWeightNormal,
    /// The weight of emphasised text.
    FontWeightBold,
    /// The tightest corner radius.
    BorderRadiusSm,
    /// The default corner radius.
    BorderRadiusMd,
    /// The loosest corner radius.
    BorderRadiusLg,
    /// The width of a hairline.
    BorderWidth,
    /// How long a fast transition takes: a press, a hover.
    DurationFast,
    /// How long a default transition takes.
    DurationNormal,
    /// How long a slow transition takes: a theme switch, a sheet.
    DurationSlow,
    /// The curve a default transition follows.
    EasingStandard,
    /// The curve a transition that is leaving the screen follows.
    EasingDecelerate,
    /// The curve a transition that is entering the screen follows.
    EasingAccelerate,
}

/// The number of tokens [`ThemeToken::all`] lists.
const TOKEN_COUNT: usize = 33;

/// Every token, in the order the theme tables below are written.
static ALL_TOKENS: [ThemeToken; TOKEN_COUNT] = [
    ThemeToken::Background,
    ThemeToken::Surface,
    ThemeToken::Primary,
    ThemeToken::OnPrimary,
    ThemeToken::Text,
    ThemeToken::TextMuted,
    ThemeToken::Border,
    ThemeToken::Error,
    ThemeToken::Warning,
    ThemeToken::Success,
    ThemeToken::SpacingXs,
    ThemeToken::SpacingSm,
    ThemeToken::SpacingMd,
    ThemeToken::SpacingLg,
    ThemeToken::SpacingXl,
    ThemeToken::FontFamily,
    ThemeToken::FontSizeXs,
    ThemeToken::FontSizeSm,
    ThemeToken::FontSizeMd,
    ThemeToken::FontSizeLg,
    ThemeToken::FontSizeXl,
    ThemeToken::FontWeightNormal,
    ThemeToken::FontWeightBold,
    ThemeToken::BorderRadiusSm,
    ThemeToken::BorderRadiusMd,
    ThemeToken::BorderRadiusLg,
    ThemeToken::BorderWidth,
    ThemeToken::DurationFast,
    ThemeToken::DurationNormal,
    ThemeToken::DurationSlow,
    ThemeToken::EasingStandard,
    ThemeToken::EasingDecelerate,
    ThemeToken::EasingAccelerate,
];

impl ThemeToken {
    /// Returns every token, in a fixed order.
    ///
    /// The order is the one the theme tables are written in, and the one
    /// [`Theme::switch_to`] animates in. It is a fixed order rather than a
    /// hash iteration so a transition's frames are reproducible.
    #[must_use]
    pub fn all() -> &'static [ThemeToken] {
        &ALL_TOKENS
    }
}

/// The value a theme token holds.
///
/// Tokens are of several kinds — colours, numbers, a font family, durations,
/// easing curves — and this is the one type that can hold any of them. A theme
/// stores one of these per token, so [`Theme::get`] and [`Theme::set`] have a
/// single signature for all 33 tokens; what a token's variant must be is fixed
/// by the token, and the theme tables below maintain it.
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    /// A colour, for the colour tokens.
    Color(Color),
    /// A number: a spacing, a font size, a radius, a border width, a weight.
    Number(f32),
    /// A string: the font family.
    Text(String),
    /// A duration: the motion tokens.
    Duration(Duration),
    /// An easing curve: the easing tokens.
    Easing(Easing),
}

impl PropertyValue {
    /// Returns the colour in this value, or `None` if it holds another kind.
    #[must_use]
    pub fn as_color(&self) -> Option<Color> {
        match self {
            PropertyValue::Color(color) => Some(*color),
            _ => None,
        }
    }

    /// Returns the number in this value, or `None` if it holds another kind.
    #[must_use]
    pub fn as_number(&self) -> Option<f32> {
        match self {
            PropertyValue::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// Returns the string in this value, or `None` if it holds another kind.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            PropertyValue::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Returns the duration in this value, or `None` if it holds another kind.
    #[must_use]
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            PropertyValue::Duration(duration) => Some(*duration),
            _ => None,
        }
    }

    /// Returns the easing in this value, or `None` if it holds another kind.
    #[must_use]
    pub fn as_easing(&self) -> Option<Easing> {
        match self {
            PropertyValue::Easing(easing) => Some(*easing),
            _ => None,
        }
    }
}

impl Default for PropertyValue {
    /// Returns a transparent black — the value a token holds before anything
    /// has been set, and the initial value of a property bound to a token.
    ///
    /// The default is a formality: [`Property::bind`] evaluates its closure
    /// immediately and overwrites this, and every token of a constructed theme
    /// holds a real value. It is a colour because a colour is what a token
    /// most often holds, and transparent black is the one that paints nothing.
    fn default() -> Self {
        PropertyValue::Color(Color::new(0, 0, 0, 0))
    }
}

impl Interpolate for PropertyValue {
    /// Interpolates between two values `t` of the way from `from` to `to`.
    ///
    /// Colours, numbers and durations lerp: the colour tokens crossfade, the
    /// spacing and size tokens grow and shrink, the motion tokens slow down
    /// and speed up. Text and easing have no meaningful halfway — there is no
    /// font family between two — so they hold `from` until `t` reaches `1` and
    /// then snap to `to`. The consequence is that a theme switch crossfades
    /// every colour and size over the transition while the font family and the
    /// easing curves change at its end, which is the right behaviour for both:
    /// a font that crossfaded would be unreadable, and an easing curve is not
    /// a value that moves.
    ///
    /// A pair whose two sides are different kinds — which a constructed theme
    /// never produces — switches discretely too, holding `from` and jumping to
    /// `to` at the end.
    fn interpolate(from: &Self, to: &Self, t: f32) -> Self {
        match (from, to) {
            (PropertyValue::Color(from), PropertyValue::Color(to)) => {
                PropertyValue::Color(Color::interpolate(from, to, t))
            }
            (PropertyValue::Number(from), PropertyValue::Number(to)) => {
                PropertyValue::Number(f32::interpolate(from, to, t))
            }
            (PropertyValue::Duration(from), PropertyValue::Duration(to)) => {
                PropertyValue::Duration(interpolate_duration(from, to, t))
            }
            _ => {
                if t >= 1.0 {
                    to.clone()
                } else {
                    from.clone()
                }
            }
        }
    }
}

/// Interpolates between two durations, in seconds.
///
/// The seconds are `f64`, not `f32`: a duration is nanoseconds, and an `f32`
/// cannot hold a millisecond count's worth of them — interpolating 150 ms and
/// 300 ms in `f32` seconds lands on 225.000009 ms, which is a duration the
/// animation then arrives at a frame late.
///
/// `Interpolate` does not clamp `t`, and an extrapolating easing can ask for a
/// value outside the endpoints — which for a duration means negative, or past
/// the maximum a `Duration` can hold. `Duration::from_secs_f64` panics on all
/// three, so the result is clamped into the range a duration can hold rather
/// than trusted.
fn interpolate_duration(from: &Duration, to: &Duration, t: f32) -> Duration {
    let from_secs = from.as_secs_f64();
    let to_secs = to.as_secs_f64();
    let seconds = from_secs + (to_secs - from_secs) * f64::from(t);
    if seconds.is_nan() {
        return *from;
    }
    Duration::from_secs_f64(seconds.clamp(0.0, Duration::MAX.as_secs_f64()))
}

/// A theme: one [`Property`] per token, and the clock that drives transitions.
///
/// The properties are the theme. A widget binds to a token's property and the
/// property graph carries every change to it; [`Theme::set`] writes one token
/// and every dependent is recomputed. The clock is what makes
/// [`Theme::switch_to`] animated: it is the theme's own, because switching is
/// something the theme does to itself, and whoever owns the theme ticks it
/// once a frame alongside any other clock they drive.
///
/// A theme is not [`Clone`]: the clock inside it is a running thing, and two
/// handles to one theme are shared with [`std::rc::Rc`] and
/// [`std::cell::RefCell`] the way the module documentation describes.
///
/// The clock is behind a [`RefCell`] because [`Theme::switch_to`] takes `&self`
/// — switching is something the theme does to itself, and the caller does not
/// hold the clock to be given `&mut`.
pub struct Theme {
    tokens: HashMap<ThemeToken, Property<PropertyValue>>,
    clock: RefCell<AnimationClock>,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::new()
    }
}

impl Theme {
    /// Creates the default theme: [`Theme::dark`].
    #[must_use]
    pub fn new() -> Self {
        Theme::dark()
    }

    /// Creates the dark theme.
    ///
    /// The colours are a dark surface palette — near-black backgrounds, a
    /// violet primary, light text — and the non-colour tokens are the shared
    /// defaults: the spacing, typography, shape and motion values every theme
    /// defines, dark and light alike.
    #[must_use]
    pub fn dark() -> Self {
        Theme::from_table(&[
            (
                ThemeToken::Background,
                PropertyValue::Color(Color::new(18, 18, 18, 255)),
            ),
            (
                ThemeToken::Surface,
                PropertyValue::Color(Color::new(30, 30, 30, 255)),
            ),
            (
                ThemeToken::Primary,
                PropertyValue::Color(Color::new(187, 134, 252, 255)),
            ),
            (
                ThemeToken::OnPrimary,
                PropertyValue::Color(Color::new(0, 0, 0, 255)),
            ),
            (
                ThemeToken::Text,
                PropertyValue::Color(Color::new(255, 255, 255, 255)),
            ),
            (
                ThemeToken::TextMuted,
                PropertyValue::Color(Color::new(158, 158, 158, 255)),
            ),
            (
                ThemeToken::Border,
                PropertyValue::Color(Color::new(51, 51, 51, 255)),
            ),
            (
                ThemeToken::Error,
                PropertyValue::Color(Color::new(207, 102, 121, 255)),
            ),
            (
                ThemeToken::Warning,
                PropertyValue::Color(Color::new(255, 183, 77, 255)),
            ),
            (
                ThemeToken::Success,
                PropertyValue::Color(Color::new(102, 187, 106, 255)),
            ),
            (ThemeToken::SpacingXs, PropertyValue::Number(4.0)),
            (ThemeToken::SpacingSm, PropertyValue::Number(8.0)),
            (ThemeToken::SpacingMd, PropertyValue::Number(16.0)),
            (ThemeToken::SpacingLg, PropertyValue::Number(24.0)),
            (ThemeToken::SpacingXl, PropertyValue::Number(32.0)),
            (
                ThemeToken::FontFamily,
                PropertyValue::Text("Inter".to_string()),
            ),
            (ThemeToken::FontSizeXs, PropertyValue::Number(10.0)),
            (ThemeToken::FontSizeSm, PropertyValue::Number(12.0)),
            (ThemeToken::FontSizeMd, PropertyValue::Number(14.0)),
            (ThemeToken::FontSizeLg, PropertyValue::Number(18.0)),
            (ThemeToken::FontSizeXl, PropertyValue::Number(24.0)),
            (ThemeToken::FontWeightNormal, PropertyValue::Number(400.0)),
            (ThemeToken::FontWeightBold, PropertyValue::Number(700.0)),
            (ThemeToken::BorderRadiusSm, PropertyValue::Number(4.0)),
            (ThemeToken::BorderRadiusMd, PropertyValue::Number(8.0)),
            (ThemeToken::BorderRadiusLg, PropertyValue::Number(16.0)),
            (ThemeToken::BorderWidth, PropertyValue::Number(1.0)),
            (
                ThemeToken::DurationFast,
                PropertyValue::Duration(Duration::from_millis(150)),
            ),
            (
                ThemeToken::DurationNormal,
                PropertyValue::Duration(Duration::from_millis(300)),
            ),
            (
                ThemeToken::DurationSlow,
                PropertyValue::Duration(Duration::from_millis(500)),
            ),
            (
                ThemeToken::EasingStandard,
                PropertyValue::Easing(Easing::EaseInOut),
            ),
            (
                ThemeToken::EasingDecelerate,
                PropertyValue::Easing(Easing::EaseOut),
            ),
            (
                ThemeToken::EasingAccelerate,
                PropertyValue::Easing(Easing::EaseIn),
            ),
        ])
    }

    /// Creates the light theme.
    ///
    /// Same structure as [`Theme::dark`], with a light surface palette —
    /// white backgrounds, a deep violet primary, dark text. The non-colour
    /// tokens are the dark theme's: spacing, typography, shape and motion do
    /// not change between dark and light, only the colours do.
    #[must_use]
    pub fn light() -> Self {
        Theme::from_table(&[
            (
                ThemeToken::Background,
                PropertyValue::Color(Color::new(255, 255, 255, 255)),
            ),
            (
                ThemeToken::Surface,
                PropertyValue::Color(Color::new(245, 245, 245, 255)),
            ),
            (
                ThemeToken::Primary,
                PropertyValue::Color(Color::new(98, 0, 238, 255)),
            ),
            (
                ThemeToken::OnPrimary,
                PropertyValue::Color(Color::new(255, 255, 255, 255)),
            ),
            (
                ThemeToken::Text,
                PropertyValue::Color(Color::new(0, 0, 0, 255)),
            ),
            (
                ThemeToken::TextMuted,
                PropertyValue::Color(Color::new(117, 117, 117, 255)),
            ),
            (
                ThemeToken::Border,
                PropertyValue::Color(Color::new(224, 224, 224, 255)),
            ),
            (
                ThemeToken::Error,
                PropertyValue::Color(Color::new(176, 0, 32, 255)),
            ),
            (
                ThemeToken::Warning,
                PropertyValue::Color(Color::new(245, 124, 0, 255)),
            ),
            (
                ThemeToken::Success,
                PropertyValue::Color(Color::new(56, 142, 60, 255)),
            ),
            (ThemeToken::SpacingXs, PropertyValue::Number(4.0)),
            (ThemeToken::SpacingSm, PropertyValue::Number(8.0)),
            (ThemeToken::SpacingMd, PropertyValue::Number(16.0)),
            (ThemeToken::SpacingLg, PropertyValue::Number(24.0)),
            (ThemeToken::SpacingXl, PropertyValue::Number(32.0)),
            (
                ThemeToken::FontFamily,
                PropertyValue::Text("Inter".to_string()),
            ),
            (ThemeToken::FontSizeXs, PropertyValue::Number(10.0)),
            (ThemeToken::FontSizeSm, PropertyValue::Number(12.0)),
            (ThemeToken::FontSizeMd, PropertyValue::Number(14.0)),
            (ThemeToken::FontSizeLg, PropertyValue::Number(18.0)),
            (ThemeToken::FontSizeXl, PropertyValue::Number(24.0)),
            (ThemeToken::FontWeightNormal, PropertyValue::Number(400.0)),
            (ThemeToken::FontWeightBold, PropertyValue::Number(700.0)),
            (ThemeToken::BorderRadiusSm, PropertyValue::Number(4.0)),
            (ThemeToken::BorderRadiusMd, PropertyValue::Number(8.0)),
            (ThemeToken::BorderRadiusLg, PropertyValue::Number(16.0)),
            (ThemeToken::BorderWidth, PropertyValue::Number(1.0)),
            (
                ThemeToken::DurationFast,
                PropertyValue::Duration(Duration::from_millis(150)),
            ),
            (
                ThemeToken::DurationNormal,
                PropertyValue::Duration(Duration::from_millis(300)),
            ),
            (
                ThemeToken::DurationSlow,
                PropertyValue::Duration(Duration::from_millis(500)),
            ),
            (
                ThemeToken::EasingStandard,
                PropertyValue::Easing(Easing::EaseInOut),
            ),
            (
                ThemeToken::EasingDecelerate,
                PropertyValue::Easing(Easing::EaseOut),
            ),
            (
                ThemeToken::EasingAccelerate,
                PropertyValue::Easing(Easing::EaseIn),
            ),
        ])
    }

    /// Returns the value `token` holds.
    ///
    /// The value's variant is fixed by the token: a colour token holds
    /// [`PropertyValue::Color`], a spacing token holds [`PropertyValue::Number`],
    /// and so on. The theme tables maintain that; a caller that wants to write
    /// it is [`Theme::set`].
    #[must_use]
    pub fn get(&self, token: ThemeToken) -> PropertyValue {
        self.tokens
            .get(&token)
            .map_or_else(PropertyValue::default, |property| property.get())
    }

    /// Sets the value `token` holds, and recomputes every dependent.
    ///
    /// The value's variant must match the token's kind, the way
    /// [`Theme::get`] says it: a colour token takes a [`PropertyValue::Color`].
    /// `set` itself stores what it is given — the theme cannot tell a colour
    /// from a number once both are the same type — so a mismatched value is a
    /// programming error the way a mismatched argument to any function is.
    pub fn set(&self, token: ThemeToken, value: PropertyValue) {
        if let Some(property) = self.tokens.get(&token) {
            property.set(value);
        }
    }

    /// Returns the property behind `token`.
    ///
    /// This is the property itself rather than its value, for a caller that
    /// wants to bind to it directly — a widget holding the theme's background
    /// property is bound to the theme's background, and follows it through
    /// every change and every transition. A caller that shares the theme behind
    /// `Rc` and `RefCell` captures these clones in its binds rather than the
    /// shared handle, so that a tick's recompute never borrows the theme while
    /// the caller holds it borrowed.
    #[must_use]
    pub fn property(&self, token: ThemeToken) -> Property<PropertyValue> {
        self.tokens
            .get(&token)
            .cloned()
            .unwrap_or_else(|| Property::new(PropertyValue::default()))
    }

    /// Animates every token from the value it holds to `new_theme`'s.
    ///
    /// Each token's property is animated with [`Property::animate_to`] over
    /// `duration_ms`, along the theme's own
    /// [`EasingStandard`](ThemeToken::EasingStandard) curve, so every dependent
    /// is recomputed on every frame of the transition: a widget bound to the
    /// background colour crossfades from the old one to the new, and a widget
    /// bound to a spacing grows or shrinks.
    ///
    /// A transition already in flight is replaced. The clock is cleared first,
    /// so the old animations stop where they are, and the new ones start from
    /// the values those reached — a token half way between two themes switches
    /// to the third from there, rather than jumping back to where it started.
    ///
    /// A `duration_ms` of zero completes on the first tick rather than dividing
    /// by zero on the way there, which is what an instant switch wants.
    pub fn switch_to(&self, new_theme: Theme, duration_ms: u32) {
        let duration = Duration::from_millis(u64::from(duration_ms));
        let easing = match self.get(ThemeToken::EasingStandard) {
            PropertyValue::Easing(easing) => easing,
            _ => Easing::EaseInOut,
        };
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        for token in ThemeToken::all() {
            let to = new_theme.get(*token);
            let animation = self.property(*token).animate_to(to, duration, easing);
            clock.add(animation);
        }
    }

    /// Advances the transition clock by `delta`, and returns whether any token
    /// is still moving.
    ///
    /// This is the frame integration for a theme switch: call it once per
    /// frame, before layout, with the time that frame took. It is the theme's
    /// own clock, separate from any other — a caller that drives animations of
    /// its own ticks those separately, the way the demo ticks its pads' clock
    /// and this one.
    pub fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Builds a theme from a table of token values.
    ///
    /// The table must hold every token exactly once; a token missing from it
    /// would have no property, and [`Theme::get`] would answer it with the
    /// default. Both theme constructors write their tables out in full.
    fn from_table(table: &[(ThemeToken, PropertyValue)]) -> Theme {
        let mut tokens = HashMap::with_capacity(table.len());
        for (token, value) in table {
            tokens.insert(*token, Property::new(value.clone()));
        }
        Theme {
            tokens,
            clock: RefCell::new(AnimationClock::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A frame's worth of time, so tests read in milliseconds.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Returns the colour a theme holds for `token`, asserting it is a colour.
    fn color_of(theme: &Theme, token: ThemeToken) -> Color {
        match theme.get(token) {
            PropertyValue::Color(color) => color,
            other => panic!("{token:?} holds {other:?}, expected a colour"),
        }
    }

    /// Returns whether `token` is one of the colour tokens.
    fn is_color_token(token: ThemeToken) -> bool {
        matches!(
            token,
            ThemeToken::Background
                | ThemeToken::Surface
                | ThemeToken::Primary
                | ThemeToken::OnPrimary
                | ThemeToken::Text
                | ThemeToken::TextMuted
                | ThemeToken::Border
                | ThemeToken::Error
                | ThemeToken::Warning
                | ThemeToken::Success
        )
    }

    #[test]
    fn a_new_theme_is_the_dark_theme() {
        let dark = Theme::dark();
        let new = Theme::new();
        for token in ThemeToken::all() {
            assert_eq!(
                dark.get(*token),
                new.get(*token),
                "{token:?} is the same in the default theme and the dark one"
            );
        }
    }

    #[test]
    fn token_access_returns_the_value_each_token_holds() {
        let theme = Theme::dark();
        assert_eq!(
            color_of(&theme, ThemeToken::Background),
            Color::new(18, 18, 18, 255)
        );
        assert_eq!(
            theme.get(ThemeToken::SpacingMd),
            PropertyValue::Number(16.0)
        );
        assert_eq!(
            theme.get(ThemeToken::FontFamily),
            PropertyValue::Text("Inter".to_string())
        );
        assert_eq!(
            theme.get(ThemeToken::DurationNormal),
            PropertyValue::Duration(Duration::from_millis(300))
        );
        assert_eq!(
            theme.get(ThemeToken::EasingStandard),
            PropertyValue::Easing(Easing::EaseInOut)
        );
    }

    #[test]
    fn dark_and_light_define_every_token() {
        // Both themes hold a value for all 33 tokens, and they are two
        // different themes: every colour token differs between them. A light
        // theme that kept the dark background would be a bug this catches.
        let dark = Theme::dark();
        let light = Theme::light();
        assert_eq!(ThemeToken::all().len(), 33);
        for token in ThemeToken::all() {
            if is_color_token(*token) {
                assert_ne!(
                    dark.get(*token),
                    light.get(*token),
                    "{token:?} differs between the two themes"
                );
            }
        }
    }

    #[test]
    fn the_non_colour_tokens_are_shared_between_the_themes() {
        // Spacing, typography, shape and motion do not change between dark and
        // light — only colours do. A light theme that restated them differently
        // would be a bug this catches.
        let dark = Theme::dark();
        let light = Theme::light();
        for token in [
            ThemeToken::SpacingMd,
            ThemeToken::FontFamily,
            ThemeToken::FontSizeMd,
            ThemeToken::FontWeightBold,
            ThemeToken::BorderRadiusMd,
            ThemeToken::BorderWidth,
            ThemeToken::DurationFast,
            ThemeToken::DurationNormal,
            ThemeToken::DurationSlow,
            ThemeToken::EasingStandard,
            ThemeToken::EasingDecelerate,
            ThemeToken::EasingAccelerate,
        ] {
            assert_eq!(
                dark.get(token),
                light.get(token),
                "{token:?} is shared between the themes"
            );
        }
    }

    #[test]
    fn set_updates_a_token_and_recomputes_its_dependents() {
        // The theme as a property source: a widget bound to a token sees the
        // write, the way it would see a switch.
        let theme = Theme::dark();
        let background_prop = theme.property(ThemeToken::Background);
        let background = Property::bind(move || background_prop.get());
        assert_eq!(background.get(), theme.get(ThemeToken::Background));

        theme.set(
            ThemeToken::Background,
            PropertyValue::Color(Color::new(1, 2, 3, 255)),
        );
        assert_eq!(
            background.get(),
            PropertyValue::Color(Color::new(1, 2, 3, 255)),
            "the bound property was recomputed"
        );
    }

    #[test]
    fn a_bound_property_follows_a_theme_switch() {
        // The whole path from "the theme switched" to "the widget's bound
        // property holds the new value", through the animation and the clock.
        let theme = Theme::dark();
        let background_prop = theme.property(ThemeToken::Background);
        let background = Property::bind(move || background_prop.get());
        let dark_background = background.get();

        theme.switch_to(Theme::light(), 100);
        theme.tick(ms(100));

        assert_eq!(
            background.get(),
            Theme::light().get(ThemeToken::Background),
            "the bound property arrived at the light theme's background"
        );
        assert_ne!(background.get(), dark_background);
    }

    #[test]
    fn a_theme_switch_updates_every_token() {
        let theme = Theme::dark();
        theme.switch_to(Theme::light(), 100);
        theme.tick(ms(100));

        let light = Theme::light();
        for token in ThemeToken::all() {
            assert_eq!(
                theme.get(*token),
                light.get(*token),
                "{token:?} arrived at the light theme's value"
            );
        }
    }

    #[test]
    fn an_animated_transition_produces_intermediate_values() {
        // Half way through a 200 ms switch, every colour token is between the
        // two themes' values — not at either end. This is the crossfade.
        let theme = Theme::dark();
        theme.switch_to(Theme::light(), 200);
        theme.tick(ms(100));

        let dark = Theme::dark();
        let light = Theme::light();
        for token in [
            ThemeToken::Background,
            ThemeToken::Surface,
            ThemeToken::Primary,
            ThemeToken::Text,
        ] {
            let value = theme.get(token);
            assert_ne!(
                value,
                dark.get(token),
                "{token:?} has moved off the dark theme's value"
            );
            assert_ne!(
                value,
                light.get(token),
                "{token:?} has not arrived at the light theme's value yet"
            );
            // A colour half way between two colours is the interpolation of
            // them, which is what makes it a crossfade and not a journey
            // through some third colour.
            let expected = PropertyValue::Color(Color::interpolate(
                &color_of(&dark, token),
                &color_of(&light, token),
                0.5,
            ));
            assert_eq!(value, expected, "{token:?} is half way between the themes");
        }
    }

    #[test]
    fn a_transition_runs_along_the_themes_standard_easing() {
        // The switch reads its curve from the theme's EasingStandard token, so
        // the transition follows the theme's own idea of standard motion.
        let theme = Theme::dark();
        theme.switch_to(Theme::light(), 100);
        theme.tick(ms(50));

        // EaseInOut at half the time is half the distance.
        let expected = PropertyValue::Color(Color::interpolate(
            &Color::new(18, 18, 18, 255),
            &Color::new(255, 255, 255, 255),
            Easing::EaseInOut.apply(0.5),
        ));
        assert_eq!(theme.get(ThemeToken::Background), expected);
    }

    #[test]
    fn a_second_switch_starts_from_where_the_first_one_reached() {
        // A transition in flight is replaced, not stacked: the new animations
        // start from the values the old ones had reached, so a token half way
        // between two themes switches to the third from there.
        let theme = Theme::dark();
        theme.switch_to(Theme::light(), 200);
        theme.tick(ms(100));
        let middle = theme.get(ThemeToken::Background);

        theme.switch_to(Theme::dark(), 200);
        // The new transition's first act is to put every property at its start
        // value, which is where the old transition reached.
        assert_eq!(
            theme.get(ThemeToken::Background),
            middle,
            "the new transition starts from the value the old one reached"
        );

        theme.tick(ms(100));
        let expected = PropertyValue::Color(Color::interpolate(
            &match middle {
                PropertyValue::Color(color) => color,
                _ => panic!("the background token holds a colour"),
            },
            &Color::new(18, 18, 18, 255),
            Easing::EaseInOut.apply(0.5),
        ));
        assert_eq!(
            theme.get(ThemeToken::Background),
            expected,
            "half way through the second switch, the colour is half way back"
        );
    }

    #[test]
    fn a_zero_length_switch_completes_on_the_first_tick() {
        let theme = Theme::dark();
        theme.switch_to(Theme::light(), 0);
        assert_eq!(
            theme.get(ThemeToken::Background),
            Theme::dark().get(ThemeToken::Background),
            "the switch has started but not ticked: the property holds its start value"
        );
        theme.tick(ms(1));
        assert_eq!(
            theme.get(ThemeToken::Background),
            Theme::light().get(ThemeToken::Background),
            "a zero-length switch completes on the first tick"
        );
    }

    #[test]
    fn text_and_easing_tokens_switch_at_the_end_of_the_transition() {
        // There is no font family between two, so the text and easing tokens
        // hold their old value for the whole transition and snap to the new one
        // at its end. The dark and light themes share both tokens, so the test
        // sets them to something else first — otherwise the switch would have
        // nothing to hold and nothing to snap to.
        let theme = Theme::dark();
        theme.set(
            ThemeToken::FontFamily,
            PropertyValue::Text("Roboto".to_string()),
        );
        theme.set(
            ThemeToken::EasingStandard,
            PropertyValue::Easing(Easing::Linear),
        );

        theme.switch_to(Theme::light(), 100);
        theme.tick(ms(50));
        assert_eq!(
            theme.get(ThemeToken::FontFamily),
            PropertyValue::Text("Roboto".to_string()),
            "half way through, the font family has not moved"
        );
        assert_eq!(
            theme.get(ThemeToken::EasingStandard),
            PropertyValue::Easing(Easing::Linear),
            "and neither has the easing curve"
        );

        theme.tick(ms(50));
        assert_eq!(
            theme.get(ThemeToken::FontFamily),
            Theme::light().get(ThemeToken::FontFamily),
            "at the end, the font family has snapped to the new one"
        );
        assert_eq!(
            theme.get(ThemeToken::EasingStandard),
            Theme::light().get(ThemeToken::EasingStandard),
            "and so has the easing curve"
        );
    }

    #[test]
    fn text_and_easing_values_hold_their_start_until_the_end() {
        // The discrete switch at the `Interpolate` level: there is no font
        // family between two, so the value holds where it started for the
        // whole transition and jumps to where it promised at the end.
        let roboto = PropertyValue::Text("Roboto".to_string());
        let inter = PropertyValue::Text("Inter".to_string());
        assert_eq!(PropertyValue::interpolate(&roboto, &inter, 0.0), roboto);
        assert_eq!(
            PropertyValue::interpolate(&roboto, &inter, 0.5),
            roboto,
            "half way through, it holds the value it started from"
        );
        assert_eq!(
            PropertyValue::interpolate(&roboto, &inter, 1.0),
            inter,
            "at the end it jumps to the value it promised"
        );

        let linear = PropertyValue::Easing(Easing::Linear);
        let bounce = PropertyValue::Easing(Easing::Bounce);
        assert_eq!(PropertyValue::interpolate(&linear, &bounce, 0.5), linear);
        assert_eq!(PropertyValue::interpolate(&linear, &bounce, 1.0), bounce);
    }

    #[test]
    fn duration_tokens_interpolate_between_the_two_themes() {
        // The motion tokens lerp like the numbers they are: a duration half way
        // between 150 ms and 300 ms is 225 ms.
        let from = PropertyValue::Duration(Duration::from_millis(150));
        let to = PropertyValue::Duration(Duration::from_millis(300));
        assert_eq!(
            PropertyValue::interpolate(&from, &to, 0.5),
            PropertyValue::Duration(Duration::from_millis(225)),
            "a duration half way is the mean of the two"
        );
        assert_eq!(
            PropertyValue::interpolate(&from, &to, 0.0),
            from,
            "at the start it is the value it started from"
        );
        assert_eq!(
            PropertyValue::interpolate(&from, &to, 1.0),
            to,
            "at the end it is the value it promised"
        );
    }

    #[test]
    fn a_duration_survives_an_extrapolating_interpolation() {
        // `Interpolate` does not clamp `t`, and an extrapolating easing can ask
        // for a duration past either end. `Duration::from_secs_f32` panics on
        // a negative, NaN or overflowing value, so the interpolation clamps
        // rather than panics.
        let from = PropertyValue::Duration(Duration::from_millis(100));
        let to = PropertyValue::Duration(Duration::from_millis(200));

        let below = PropertyValue::interpolate(&from, &to, -1.0);
        assert_eq!(
            below,
            PropertyValue::Duration(Duration::ZERO),
            "below zero it clamps to zero rather than panicking"
        );
        let above = PropertyValue::interpolate(&from, &to, 2.0);
        assert_eq!(
            above,
            PropertyValue::Duration(Duration::from_millis(300)),
            "past the end it extrapolates, which is what an overshooting easing wants"
        );
    }

    #[test]
    fn colour_and_number_values_interpolate_field_by_field() {
        let black = PropertyValue::Color(Color::new(0, 0, 0, 255));
        let white = PropertyValue::Color(Color::new(255, 255, 255, 255));
        assert_eq!(
            PropertyValue::interpolate(&black, &white, 0.5),
            PropertyValue::Color(Color::new(128, 128, 128, 255)),
            "a colour half way is the interpolation of its channels"
        );

        let zero = PropertyValue::Number(0.0);
        let ten = PropertyValue::Number(10.0);
        assert_eq!(
            PropertyValue::interpolate(&zero, &ten, 0.25),
            PropertyValue::Number(2.5),
            "a number a quarter of the way is a quarter of the distance"
        );
    }

    #[test]
    fn a_mismatched_pair_switches_discretely() {
        // A constructed theme never produces a pair of different kinds, but
        // `Interpolate` is total: it holds `from` and jumps to `to` at the end.
        let colour = PropertyValue::Color(Color::new(1, 2, 3, 255));
        let number = PropertyValue::Number(7.0);
        assert_eq!(
            PropertyValue::interpolate(&colour, &number, 0.5),
            colour,
            "half way through, it holds the value it started from"
        );
        assert_eq!(
            PropertyValue::interpolate(&colour, &number, 1.0),
            number,
            "at the end it jumps to the value it promised"
        );
    }

    #[test]
    fn the_accessors_return_the_value_of_their_own_kind() {
        let colour = PropertyValue::Color(Color::new(1, 2, 3, 255));
        assert_eq!(colour.as_color(), Some(Color::new(1, 2, 3, 255)));
        assert_eq!(colour.as_number(), None);
        assert_eq!(colour.as_text(), None);
        assert_eq!(colour.as_duration(), None);
        assert_eq!(colour.as_easing(), None);

        let number = PropertyValue::Number(7.0);
        assert_eq!(number.as_number(), Some(7.0));
        assert_eq!(number.as_color(), None);

        let text = PropertyValue::Text("Inter".to_string());
        assert_eq!(text.as_text(), Some("Inter"));
        assert_eq!(text.as_color(), None);

        let duration = PropertyValue::Duration(Duration::from_millis(5));
        assert_eq!(duration.as_duration(), Some(Duration::from_millis(5)));
        assert_eq!(duration.as_color(), None);

        let easing = PropertyValue::Easing(Easing::Linear);
        assert_eq!(easing.as_easing(), Some(Easing::Linear));
        assert_eq!(easing.as_color(), None);
    }

    #[test]
    fn a_theme_shared_with_rc_and_refcell_follows_its_switch() {
        // A caller that binds to the theme and ticks it too shares one theme
        // behind Rc and RefCell. The binds capture the theme's properties — the
        // clones `Theme::property` returns — rather than the shared handle, so
        // that a tick's recompute never borrows the theme while the caller
        // holds it borrowed.
        let theme = Rc::new(RefCell::new(Theme::dark()));
        let background = {
            let background_prop = theme.borrow().property(ThemeToken::Background);
            Property::bind(move || background_prop.get())
        };
        let dark = background.get();

        theme.borrow_mut().switch_to(Theme::light(), 100);
        theme.borrow_mut().tick(ms(50));
        assert_ne!(
            background.get(),
            dark,
            "half way through, the colour has moved"
        );

        theme.borrow_mut().tick(ms(50));
        assert_eq!(
            background.get(),
            Theme::light().get(ThemeToken::Background),
            "and the bound property arrived"
        );
    }

    #[test]
    fn a_switch_notifies_a_dependent_on_every_frame_it_moves() {
        // The path from "an animation wrote a token" to "whatever reads it is
        // told": a dependent of an animated token is recomputed on every frame
        // of the transition, not only at its end.
        let theme = Theme::dark();
        let background_prop = theme.property(ThemeToken::Background);
        let background = Property::bind(move || background_prop.get());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_log = Rc::clone(&seen);
        background.on_change(move |value| seen_log.borrow_mut().push(value.clone()));

        theme.switch_to(Theme::light(), 40);
        for _ in 0..4 {
            theme.tick(ms(10));
        }

        let seen = seen.borrow();
        assert_eq!(
            seen.len(),
            5,
            "the animation's start value plus four frames, each recomputed"
        );
        assert_eq!(
            seen.last(),
            Some(&Theme::light().get(ThemeToken::Background)),
            "and the last one is the value it arrived at"
        );
    }
}
