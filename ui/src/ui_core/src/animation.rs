//! Animation.
//!
//! Owns the animation types, the easing functions they apply, and the clock
//! that drives them.
//!
//! An animation is a value moving from one place to another over a duration. It
//! is created through the property it writes to — [`Property::animate_to`] and
//! its siblings in [`crate::property`] — handed to an [`AnimationClock`], and
//! advanced by [`AnimationClock::tick`], which the frame loop calls once per
//! frame with the time that frame took. Nothing here rebuilds the widget tree:
//! the tick writes a property value, and the property's own dependents and
//! callbacks are what carry the change to whatever reads it.
//!
//! Time is never read from a clock this module owns. Every entry point that
//! needs to know "when" takes the elapsed time as a [`Duration`] the caller
//! supplies, so a frame loop that measures `Instant::now()` drives it, and a
//! test drives it with whatever `Duration` it means. That is the whole reason
//! [`Animation::value_at`] and [`Animation::progress_at`] are separate from
//! [`AnimationClock::tick`]: an animation can be sampled at any moment without
//! one.
//!
//! # Examples
//!
//! A number moving from where it is to somewhere else, driven by hand:
//!
//! ```
//! use std::time::Duration;
//! use ui_core::animation::Easing;
//! use ui_core::property::Property;
//!
//! let opacity = Property::new(0.0_f32);
//! let animation = opacity.animate_to(1.0, Duration::from_millis(400), Easing::EaseOut);
//!
//! assert_eq!(animation.value_at(Duration::from_millis(200)), 0.75);
//! ```
//!
//! The same animation under a clock, which writes the property as it goes:
//!
//! ```
//! use std::time::Duration;
//! use ui_core::animation::{AnimationClock, Easing};
//! use ui_core::property::Property;
//!
//! let opacity = Property::new(0.0_f32);
//! let mut clock = AnimationClock::new();
//! clock.add(opacity.animate_to(1.0, Duration::from_millis(400), Easing::Linear));
//!
//! clock.tick(Duration::from_millis(100));
//! assert_eq!(opacity.get(), 0.25);
//! assert!(clock.tick(Duration::from_millis(300)));
//! assert_eq!(opacity.get(), 1.0);
//! assert!(!clock.is_animating());
//! ```

use std::time::Duration;

use crate::property::{Color, Property, PropertyHandle, Transform};

/// A type whose values can be interpolated towards each other.
///
/// This is the bound [`Property::animate_to`] adds and [`Property::new`] does
/// not: a property of a type that cannot be interpolated is still a perfectly
/// ordinary property, it just cannot be animated.
///
/// # Examples
///
/// ```
/// use ui_core::animation::Interpolate;
///
/// assert_eq!(f32::interpolate(&0.0, &10.0, 0.25), 2.5);
/// ```
pub trait Interpolate: Clone + 'static {
    /// Returns the value `t` of the way from `from` to `to`.
    ///
    /// `t` is not clamped here: below zero extrapolates past `from` and above
    /// one past `to`, which is what an overshooting spring asks for. An
    /// implementation whose values cannot leave a range clamps for itself —
    /// [`Color`] has to, and does.
    fn interpolate(from: &Self, to: &Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    fn interpolate(from: &Self, to: &Self, t: f32) -> Self {
        from + (to - from) * t
    }
}

impl Interpolate for Color {
    /// Interpolates the four components and rounds each to the nearest of the
    /// 256 a channel holds.
    ///
    /// The components are already premultiplied by the alpha, which is what
    /// makes a component-wise interpolation *the* premultiplied interpolation:
    /// interpolating straight alpha separately and premultiplying afterwards
    /// would let a fading-out colour bleed the colour underneath it into the
    /// faded one, which is the reason the whole pipeline is premultiplied.
    ///
    /// An extrapolating easing can ask for a component outside `0..=255`, and a
    /// channel is clamped into range rather than wrapped.
    fn interpolate(from: &Self, to: &Self, t: f32) -> Self {
        Color {
            r: channel(from.r, to.r, t),
            g: channel(from.g, to.g, t),
            b: channel(from.b, to.b, t),
            a: channel(from.a, to.a, t),
        }
    }
}

impl Interpolate for Transform {
    /// Interpolates translation, scale and rotation field by field.
    ///
    /// Rotation is interpolated as the number of radians it is, not along the
    /// short way round: the two angles are values, and taking the difference is
    /// the caller's decision, not this function's.
    fn interpolate(from: &Self, to: &Self, t: f32) -> Self {
        Transform {
            tx: f32::interpolate(&from.tx, &to.tx, t),
            ty: f32::interpolate(&from.ty, &to.ty, t),
            sx: f32::interpolate(&from.sx, &to.sx, t),
            sy: f32::interpolate(&from.sy, &to.sy, t),
            rotation: f32::interpolate(&from.rotation, &to.rotation, t),
        }
    }
}

/// Interpolates one colour channel between two channel values.
fn channel(from: u8, to: u8, t: f32) -> u8 {
    let value = f32::from(from) + (f32::from(to) - f32::from(from)) * t;
    // There is no `From`/`TryFrom` between `f32` and any integer type in std,
    // so this is the one float-to-integer `as` cast in the module, for the same
    // reason and with the same guarantee as the renderer's own: the cast is
    // saturating (Rust 1.45 and later). The clamp is therefore belt and
    // braces, and states the intent — an extrapolating easing must not wrap a
    // channel round to the other end.
    value.round().clamp(0.0, 255.0) as u8
}

/// The curve an animation follows between its two endpoints.
///
/// Every variant maps normalised time to progress: zero is where the animation
/// starts and one is where it arrives. The variants differ only in the shape
/// between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    /// The same distance covered in every equal span of time.
    Linear,
    /// Slow at first and fast at the end: `t²`.
    EaseIn,
    /// Fast at first and slow at the end: `1 - (1 - t)²`.
    EaseOut,
    /// Slow, then fast, then slow again, symmetric about the midpoint:
    /// `2t²` below it and `1 - 2(1 - t)²` above.
    EaseInOut,
    /// A damped harmonic oscillator, simulated rather than solved.
    ///
    /// A spring is not a curve in closed form; it is a system, and
    /// [`apply`](Easing::apply) is handed a `t` and nothing else. So this
    /// variant answers by *running* the system: a fixed number of fixed steps
    /// of a semi-implicit integrator, from rest at zero towards one, over a
    /// span scaled to the coefficients. The answer depends on `t` alone, which
    /// is what both callers need — the same `t` gives the same value on every
    /// frame at whatever rate the loop runs, and a test can ask for the value
    /// at a moment instead of sleeping until it arrives.
    ///
    /// `damping` and `stiffness` are the coefficients of a unit mass:
    ///
    /// ```text
    /// x'' = -stiffness * (x - 1) - damping * x'
    /// ```
    ///
    /// where `x` is the progress and `x'` its rate. The damping ratio
    /// `damping / (2 * sqrt(stiffness))` is what decides the shape: below one
    /// the spring passes its target and rings, one is the fastest approach
    /// without overshoot, and above one it creeps in without overshooting at
    /// all. `stiffness` sets how fast, being the square of the natural
    /// frequency.
    ///
    /// Both coefficients are sanitized before the system runs, because they
    /// are public fields and a caller can put anything in them: a non-finite
    /// damping falls back to 20 and a negative one to zero — an undamped spring
    /// is a legitimate curve and is left alone — a non-finite stiffness to 100,
    /// and a stiffness outside `1.0..=10_000.0` to the end of that range. The
    /// endpoint is exact whatever they were: an animation that finished short of
    /// the value it promised would never arrive, so [`apply`](Easing::apply)
    /// pins `1.0` and a spring whose coefficients do not settle within the span
    /// reaches it in the last frame instead of not at all.
    Spring {
        /// How strongly the spring is damped. Zero rings forever.
        damping: f32,
        /// The square of the spring's natural frequency. Higher is faster.
        stiffness: f32,
    },
    /// Falling and rebounding: the value touches its target three times on the
    /// way and rests on it at the end.
    ///
    /// Closed form, not a simulation — four parabolic arcs rather than a
    /// spring settling. A bounce that rings like a spring is not a bounce.
    Bounce,
}

impl Easing {
    /// Returns the progress `t` of the way along the curve.
    ///
    /// `t` is normalised time. Values outside `0..=1` are clamped and a `NaN`
    /// counts as zero, so a caller that has lost track of where it is cannot
    /// get a nonsense value out of here.
    ///
    /// Every variant fixes both ends, `apply(0.0) == 0.0` and `apply(1.0) ==
    /// 1.0`, with one apparent exception that is the point of the variant: an
    /// underdamped [`Spring`](Easing::Spring) goes past `1.0` on its way and
    /// comes back.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::animation::Easing;
    ///
    /// assert_eq!(Easing::Linear.apply(0.25), 0.25);
    /// assert_eq!(Easing::EaseIn.apply(0.5), 0.25);
    /// assert_eq!(Easing::EaseOut.apply(0.5), 0.75);
    /// assert_eq!(Easing::EaseInOut.apply(0.5), 0.5);
    ///
    /// // Beyond the ends of the animation, the curve is held.
    /// assert_eq!(Easing::EaseIn.apply(-1.0), 0.0);
    /// assert_eq!(Easing::Linear.apply(2.0), 1.0);
    /// ```
    #[must_use]
    pub fn apply(&self, t: f32) -> f32 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - 2.0 * (1.0 - t) * (1.0 - t)
                }
            }
            Easing::Spring { damping, stiffness } => spring_progress(*damping, *stiffness, t),
            Easing::Bounce => bounce(t),
        }
    }
}

/// One value moving from `from` to `to` over `duration`.
///
/// An animation here is a *description*: which property it writes, where from,
/// where to, how long, and along which curve. It becomes a running thing when
/// [`AnimationClock::add`] takes it, and it is advanced by
/// [`AnimationClock::tick`].
///
/// Every question about where the value stands at a moment —
/// [`progress_at`](Animation::progress_at), [`value_at`](Animation::value_at),
/// [`is_finished_at`](Animation::is_finished_at) — answers from the time it is
/// given, without a clock. That is what makes the whole system testable
/// without a clock at all.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ui_core::animation::{Animation, Easing};
/// use ui_core::property::Property;
///
/// let width = Property::new(100.0_f32);
/// let animation =
///     Animation::new(&width, 0.0, 200.0, Duration::from_millis(500), Easing::Linear);
///
/// assert_eq!(animation.progress_at(Duration::from_millis(250)), 0.5);
/// assert_eq!(animation.value_at(Duration::from_millis(250)), 100.0);
/// assert!(animation.is_finished_at(Duration::from_millis(500)));
/// ```
pub struct Animation<T: Interpolate> {
    property: PropertyHandle<T>,
    from: T,
    to: T,
    start_time: Duration,
    duration: Duration,
    delay: Duration,
    easing: Easing,
}

impl<T: Interpolate> Animation<T> {
    /// Creates an animation of `property` from `from` to `to`.
    ///
    /// The property is put at `from` before this returns. An animation started
    /// between two frames has to have taken effect before the next paint, or
    /// the frame in between would be drawn from the value the animation is
    /// replacing.
    ///
    /// A `duration` of zero completes at the first tick rather than dividing by
    /// zero on the way there.
    pub fn new(property: &Property<T>, from: T, to: T, duration: Duration, easing: Easing) -> Self {
        property.set(from.clone());
        Animation {
            property: property.handle(),
            from,
            to,
            start_time: Duration::ZERO,
            duration,
            delay: Duration::ZERO,
            easing,
        }
    }

    /// Returns this animation with `delay` before it starts.
    ///
    /// The property holds `from` for the whole delay, which is what a stagger
    /// between children is built out of.
    #[must_use]
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Returns this animation starting at `start_time` instead of zero.
    ///
    /// [`AnimationClock::add`] stamps the clock's own elapsed time on everything
    /// it takes, so this matters to an animation sampled by hand, or added to a
    /// clock that has been running and is not being told to reset its own
    /// schedule.
    #[must_use]
    pub fn with_start_time(mut self, start_time: Duration) -> Self {
        self.start_time = start_time;
        self
    }

    /// Returns the value this animation starts from.
    #[must_use]
    pub fn from(&self) -> T {
        self.from.clone()
    }

    /// Returns the value this animation arrives at.
    #[must_use]
    pub fn to(&self) -> T {
        self.to.clone()
    }

    /// Returns how long the animation takes once it has started, without its
    /// delay.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns how long this animation waits before it starts.
    #[must_use]
    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Returns when this animation started, on the clock that drives it.
    ///
    /// Zero until a clock stamps it.
    #[must_use]
    pub fn start_time(&self) -> Duration {
        self.start_time
    }

    /// Returns the curve this animation follows.
    #[must_use]
    pub fn easing(&self) -> Easing {
        self.easing
    }

    /// Returns how long this animation takes from the moment it is added to a
    /// clock: its delay plus its duration.
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        self.delay + self.duration
    }

    /// Returns how far along the animation is at `now`, from `0.0` to `1.0`,
    /// before the easing curve is applied.
    ///
    /// This is linear time, not eased progress: it is `1.0` for the whole of
    /// the animation's tail regardless of the curve, and `0.0` for the whole of
    /// its delay. A `now` before the animation started is before its delay too,
    /// and reads as zero.
    #[must_use]
    pub fn progress_at(&self, now: Duration) -> f32 {
        let Some(elapsed) = now.checked_sub(self.start_time.saturating_add(self.delay)) else {
            return 0.0;
        };
        if self.duration.is_zero() {
            // A zero-length animation is over rather than infinitely long, so
            // the division below is not reached.
            return 1.0;
        }
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Returns the value this animation holds at `now`.
    ///
    /// This does not write the property: it answers the question. Writing is
    /// [`AnimationClock::tick`]'s job, or the caller's.
    #[must_use]
    pub fn value_at(&self, now: Duration) -> T {
        T::interpolate(
            &self.from,
            &self.to,
            self.easing.apply(self.progress_at(now)),
        )
    }

    /// Returns whether the animation has run its course by `now`.
    #[must_use]
    pub fn is_finished_at(&self, now: Duration) -> bool {
        self.progress_at(now) >= 1.0
    }

    /// Returns whether the animation still has somewhere to go at `now`.
    #[must_use]
    pub fn is_running_at(&self, now: Duration) -> bool {
        !self.is_finished_at(now)
    }

    /// Writes the value this animation holds at `now` into its property, and
    /// returns whether the property was still there to be written.
    ///
    /// A property whose last [`Property`] handle has been dropped is gone, and
    /// an animation writing into nothing stops rather than holding the clock
    /// open for the rest of its duration.
    fn write_at(&self, now: Duration) -> bool {
        match self.property.upgrade() {
            Some(property) => {
                property.set(self.value_at(now));
                true
            }
            None => false,
        }
    }
}

/// An animation of a single number: a position, a size, an opacity.
///
/// The three documented animation types are this one with a value type chosen,
/// so they are aliases rather than three types to keep in step. What a value
/// needs to be animatable is [`Interpolate`], and a caller that adds a type
/// gets an animation of it without this crate writing anything.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ui_core::animation::{Easing, NumberAnimation};
/// use ui_core::property::Property;
///
/// let width = Property::new(0.0_f32);
/// let grow: NumberAnimation = width.animate_to(100.0, Duration::from_millis(100), Easing::Linear);
/// assert_eq!(grow.value_at(Duration::from_millis(50)), 50.0);
/// ```
pub type NumberAnimation = Animation<f32>;

/// An animation of a colour, in premultiplied alpha space.
///
/// See [`NumberAnimation`] for why this is an alias. Colour channels are
/// clamped rather than wrapped, so a colour that is animated by an overshooting
/// curve still arrives where it was aimed.
pub type ColorAnimation = Animation<Color>;

/// An animation of a transform: translation, scale and rotation together.
pub type TransformAnimation = Animation<Transform>;

/// What a clock needs from an animation, with the value type out of the way.
///
/// This is the seam that lets one clock drive a number and a colour at once:
/// without it, a composition type would be generic over `T` and could only
/// ever hold animations of one kind of value.
trait Driven {
    fn start_at(&mut self, start: Duration);
    fn delay(&self) -> Duration;
    fn set_delay(&mut self, delay: Duration);
    fn total_duration(&self) -> Duration;
    fn write_at(&self, now: Duration) -> bool;
    fn is_running_at(&self, now: Duration) -> bool;
}

impl<T: Interpolate> Driven for Animation<T> {
    fn start_at(&mut self, start: Duration) {
        self.start_time = start;
    }

    fn delay(&self) -> Duration {
        self.delay
    }

    fn set_delay(&mut self, delay: Duration) {
        self.delay = delay;
    }

    fn total_duration(&self) -> Duration {
        Animation::total_duration(self)
    }

    fn write_at(&self, now: Duration) -> bool {
        Animation::write_at(self, now)
    }

    fn is_running_at(&self, now: Duration) -> bool {
        Animation::is_running_at(self, now)
    }
}

/// An animation whose value type has been erased.
///
/// [`Sequence`], [`Parallel`] and [`Stagger`] hold these rather than
/// [`Animation`], so one composition can hold a number animation and a colour
/// animation side by side — which is what "these two happen at the same time"
/// usually means, and what a theme transition over tokens of several kinds is.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ui_core::animation::{AnyAnimation, Easing};
/// use ui_core::property::{Color, Property};
///
/// let tint = Property::new(Color::new(0, 0, 0, 255));
/// let lift = Property::new(0.0_f32);
///
/// let both: Vec<AnyAnimation> = vec![
///     tint.animate_to(Color::new(255, 255, 255, 255), Duration::from_millis(200), Easing::EaseOut).into(),
///     lift.animate_to(4.0, Duration::from_millis(200), Easing::EaseOut).into(),
/// ];
/// assert_eq!(both[0].total_duration(), Duration::from_millis(200));
/// ```
pub struct AnyAnimation {
    inner: Box<dyn Driven>,
}

impl<T: Interpolate> From<Animation<T>> for AnyAnimation {
    fn from(animation: Animation<T>) -> Self {
        AnyAnimation {
            inner: Box::new(animation),
        }
    }
}

impl AnyAnimation {
    /// Returns this animation with `delay` before it starts, replacing any
    /// delay it already had.
    ///
    /// The compositions below add their own offset to whatever delay a step
    /// carried; this is the plain builder, for a caller holding a single
    /// animation.
    #[must_use]
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.inner.set_delay(delay);
        self
    }

    /// Returns how long this animation takes from the moment it is added to a
    /// clock, delay included.
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        self.inner.total_duration()
    }

    fn delay(&self) -> Duration {
        self.inner.delay()
    }

    fn start_at(&mut self, start: Duration) {
        self.inner.start_at(start);
    }

    fn write_at(&self, now: Duration) -> bool {
        self.inner.write_at(now)
    }

    fn is_running_at(&self, now: Duration) -> bool {
        self.inner.is_running_at(now)
    }
}

/// The clock that drives animations.
///
/// The clock holds the animations running against it and one elapsed time,
/// advanced by whatever the caller hands [`tick`](AnimationClock::tick). It
/// reads no clock of its own: the frame loop measures how long the last frame
/// took and passes it in, and a test passes in whatever it means.
///
/// One clock per frame loop is the intended shape. It is not a global, and it
/// is not shared: nothing in this module reaches outside the value it was
/// given.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ui_core::animation::{AnimationClock, Easing};
/// use ui_core::property::Property;
///
/// let width = Property::new(0.0_f32);
/// let mut clock = AnimationClock::new();
///
/// assert!(!clock.is_animating());
/// clock.add(width.animate_to(1.0, Duration::from_millis(100), Easing::Linear));
/// assert!(clock.is_animating());
///
/// clock.tick(Duration::from_millis(100));
/// assert_eq!(width.get(), 1.0);
/// assert!(!clock.is_animating());
/// ```
#[derive(Default)]
pub struct AnimationClock {
    elapsed: Duration,
    animations: Vec<AnyAnimation>,
}

impl AnimationClock {
    /// Creates a clock with no animations and no elapsed time.
    #[must_use]
    pub fn new() -> Self {
        AnimationClock {
            elapsed: Duration::ZERO,
            animations: Vec::new(),
        }
    }

    /// Returns how much time this clock has been advanced by.
    ///
    /// Animations measure their progress against this, which is why an
    /// animation's start time is the clock's elapsed time and not a wall clock.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns whether any animation is still running.
    ///
    /// True from the moment something is added until the tick that completes
    /// it.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        !self.animations.is_empty()
    }

    /// Starts `animation` against this clock, stamping it with the clock's
    /// elapsed time.
    ///
    /// The start time is the clock's, so an animation added between two ticks
    /// begins at that tick's time rather than at some absolute point the clock
    /// has never reached.
    ///
    /// A clock does not know that two animations are writing the same property,
    /// because the two properties cannot be compared through the erased value
    /// type. Starting a second animation over one already running therefore
    /// leaves both writing it, and the last to be ticked wins; a caller that
    /// means to replace an animation clears the clock with
    /// [`clear`](AnimationClock::clear) first.
    pub fn add(&mut self, animation: impl Into<AnyAnimation>) {
        let mut animation = animation.into();
        animation.start_at(self.elapsed);
        self.animations.push(animation);
    }

    /// Drops every animation, running or not.
    ///
    /// This is what replaces one animation with another: the animations stop
    /// where they are, and their properties hold those values.
    pub fn clear(&mut self) {
        self.animations.clear();
    }

    /// Advances the clock by `delta` and writes every running animation's value
    /// into its property, and returns whether any of them wrote.
    ///
    /// This is the frame integration: call it once per frame, before layout,
    /// with the time that frame took. It is true on every frame from the first
    /// an animation ticks through the frame it completes on, and false whenever
    /// nothing is running — so a caller that repaints only when it is true
    /// repaints exactly while something moves.
    ///
    /// Writing a property notifies its dependents and its callbacks, and that
    /// notification is the whole path from "an animation moved" to "a widget
    /// needs redrawing". Nothing here knows about widgets; the demo wires an
    /// `on_change` callback to a node handle and marks that node dirty.
    ///
    /// An animation is dropped once it has run its course, or as soon as its
    /// property is gone, so the clock does not grow for the life of the loop.
    #[must_use]
    pub fn tick(&mut self, delta: Duration) -> bool {
        let now = self.elapsed.saturating_add(delta);
        self.elapsed = now;
        let mut wrote = false;
        self.animations.retain(|animation| {
            let alive = animation.write_at(now);
            wrote = wrote || alive;
            alive && animation.is_running_at(now)
        });
        wrote
    }
}

/// Animations that run one after another.
///
/// A step starts when the one before it has finished, so the composition's
/// length is the sum of its steps'. The delay a step already carried is kept
/// and the steps before it are added on top, so a step that was going to wait
/// still waits — for longer, because it starts later.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ui_core::animation::{AnimationClock, Easing, Sequence, Stagger};
/// use ui_core::property::Property;
///
/// let left = Property::new(0.0_f32);
/// let right = Property::new(0.0_f32);
/// let mut clock = AnimationClock::new();
///
/// let move_there = Sequence::new(vec![
///     left.animate_to(1.0, Duration::from_millis(100), Easing::Linear).into(),
///     right.animate_to(1.0, Duration::from_millis(100), Easing::Linear).into(),
/// ]);
/// assert_eq!(move_there.total_duration(), Duration::from_millis(200));
/// move_there.play(&mut clock);
///
/// clock.tick(Duration::from_millis(100));
/// assert_eq!((left.get(), right.get()), (1.0, 0.0));
/// clock.tick(Duration::from_millis(100));
/// assert_eq!((left.get(), right.get()), (1.0, 1.0));
/// # let _ = Stagger::new(vec![], Duration::ZERO);
/// ```
#[derive(Default)]
pub struct Sequence {
    steps: Vec<AnyAnimation>,
}

impl Sequence {
    /// Creates a sequence that runs `steps` in order.
    #[must_use]
    pub fn new(steps: Vec<AnyAnimation>) -> Self {
        Sequence { steps }
    }

    /// Returns this sequence with `step` appended.
    #[must_use]
    pub fn then(mut self, step: impl Into<AnyAnimation>) -> Self {
        self.steps.push(step.into());
        self
    }

    /// Returns how long the whole sequence takes from the moment it is played.
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        let mut offset = Duration::ZERO;
        let mut total = Duration::ZERO;
        for step in &self.steps {
            let step_total = step.total_duration();
            total = total.max(offset + step_total);
            offset += step_total;
        }
        total
    }

    /// Adds every step to `clock`, each delayed until the ones before it are
    /// done.
    pub fn play(self, clock: &mut AnimationClock) {
        let mut offset = Duration::ZERO;
        for step in self.steps {
            let delay = offset + step.delay();
            offset += step.total_duration();
            clock.add(step.with_delay(delay));
        }
    }
}

/// Animations that run at the same time.
///
/// Every step starts when the composition does, apart from any delay it
/// already carried — which is what a [`Stagger`] is, and why a parallel is the
/// shorter way to write one.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ui_core::animation::{AnimationClock, Easing, Parallel};
/// use ui_core::property::Property;
///
/// let x = Property::new(0.0_f32);
/// let y = Property::new(0.0_f32);
/// let mut clock = AnimationClock::new();
///
/// Parallel::new(vec![
///     x.animate_to(10.0, Duration::from_millis(100), Easing::Linear).into(),
///     y.animate_to(20.0, Duration::from_millis(200), Easing::Linear).into(),
/// ])
/// .play(&mut clock);
///
/// clock.tick(Duration::from_millis(100));
/// assert_eq!((x.get(), y.get()), (10.0, 10.0));
/// assert_eq!(Parallel::new(vec![
///     x.animate_to(0.0, Duration::from_millis(300), Easing::Linear).into(),
/// ]).total_duration(), Duration::from_millis(300));
/// ```
#[derive(Default)]
pub struct Parallel {
    animations: Vec<AnyAnimation>,
}

impl Parallel {
    /// Creates a parallel that runs `animations` together.
    #[must_use]
    pub fn new(animations: Vec<AnyAnimation>) -> Self {
        Parallel { animations }
    }

    /// Returns this parallel with `animation` added to it.
    #[must_use]
    pub fn and(mut self, animation: impl Into<AnyAnimation>) -> Self {
        self.animations.push(animation.into());
        self
    }

    /// Returns how long the whole parallel takes: as long as its longest
    /// member, delay included.
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        self.animations
            .iter()
            .map(AnyAnimation::total_duration)
            .max()
            .unwrap_or_default()
    }

    /// Adds every animation to `clock`, each starting now.
    pub fn play(self, clock: &mut AnimationClock) {
        for animation in self.animations {
            clock.add(animation);
        }
    }
}

/// Animations spread across children, a fixed distance apart.
///
/// Step *n* starts `n * step` after the composition does, so a row of children
/// comes in one after the other rather than all at once. The children are the
/// caller's: this is an offset over a list of animations, and it has no notion
/// of a node, an arena, or a child order beyond the one the list is in.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use ui_core::animation::{AnimationClock, Easing, Stagger};
/// use ui_core::property::Property;
///
/// let first = Property::new(0.0_f32);
/// let second = Property::new(0.0_f32);
/// let mut clock = AnimationClock::new();
///
/// let arrive = Stagger::new(
///     vec![
///         first.animate_to(1.0, Duration::from_millis(100), Easing::Linear).into(),
///         second.animate_to(1.0, Duration::from_millis(100), Easing::Linear).into(),
///     ],
///     Duration::from_millis(100),
/// );
/// assert_eq!(arrive.total_duration(), Duration::from_millis(200));
/// arrive.play(&mut clock);
///
/// clock.tick(Duration::from_millis(150));
/// assert_eq!((first.get(), second.get()), (1.0, 0.5));
/// ```
#[derive(Default)]
pub struct Stagger {
    steps: Vec<AnyAnimation>,
    step: Duration,
}

impl Stagger {
    /// Creates a stagger whose `steps` start one `step` apart.
    #[must_use]
    pub fn new(steps: Vec<AnyAnimation>, step: Duration) -> Self {
        Stagger { steps, step }
    }

    /// Returns the distance between one step's start and the next.
    #[must_use]
    pub fn step(&self) -> Duration {
        self.step
    }

    /// Returns how long the whole stagger takes: the last step's place in the
    /// queue plus its own length.
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        let mut offset = Duration::ZERO;
        let mut total = Duration::ZERO;
        for step in &self.steps {
            total = total.max(offset + step.total_duration());
            offset += self.step;
        }
        total
    }

    /// Adds every step to `clock`, each delayed by its place in the queue.
    pub fn play(self, clock: &mut AnimationClock) {
        let mut offset = Duration::ZERO;
        for step in self.steps {
            clock.add(step.with_delay(offset));
            offset += self.step;
        }
    }
}

/// The number of integration steps a spring is simulated over.
///
/// A `u16` because [`f32::from`] converts one and not a `u32`, and a step count
/// a `u16` cannot hold is not one this integrator needs.
const SPRING_STEPS: u16 = 240;

/// How many decay constants — or how many oscillation periods — the span of a
/// spring simulation is long enough to cover.
const SPRING_SETTLE_TIME: f32 = 6.0;

/// The bounds the settling rate is held inside before the span is derived from
/// it.
const SPRING_MIN_RATE: f32 = 1.0;
const SPRING_MAX_RATE: f32 = 1_000.0;

/// The stiffness a spring is held inside. Below the floor a spring would take
/// the whole animation to move at all; above the ceiling the fixed step below
/// is no longer small enough beside the natural frequency for the integrator to
/// stay stable, and the curve would diverge rather than settle.
const SPRING_MIN_STIFFNESS: f32 = 1.0;
const SPRING_MAX_STIFFNESS: f32 = 10_000.0;

/// Substituted for coefficients that are not finite. The damping value is the
/// critical damping of [`SPRING_DEFAULT_STIFFNESS`] — `2 * sqrt(100)` — so the
/// default spring arrives without overshooting.
const SPRING_DEFAULT_DAMPING: f32 = 20.0;
const SPRING_DEFAULT_STIFFNESS: f32 = 100.0;

/// Two pi, which is the conversion between a frequency and a period.
const TAU: f32 = std::f32::consts::TAU;

/// Returns the progress a spring with these coefficients has reached at `t`.
fn spring_progress(damping: f32, stiffness: f32, t: f32) -> f32 {
    // The endpoints are pinned rather than simulated. The one at zero is what
    // the integrator starts from anyway; the one at one is pinned because an
    // animation that finished short of the value it promised would never
    // arrive, so a spring whose coefficients do not settle inside the span
    // reaches its target in the last frame rather than not at all.
    if t >= 1.0 {
        return 1.0;
    }
    if t <= 0.0 {
        return 0.0;
    }
    let damping = if damping.is_finite() {
        // Negative damping is a spring that gains energy, which is a
        // divergence rather than a curve; zero is not, and is left alone.
        damping.max(0.0)
    } else {
        SPRING_DEFAULT_DAMPING
    };
    let stiffness = if stiffness.is_finite() {
        stiffness.clamp(SPRING_MIN_STIFFNESS, SPRING_MAX_STIFFNESS)
    } else {
        SPRING_DEFAULT_STIFFNESS
    };
    let rate = settle_rate(damping, stiffness).clamp(SPRING_MIN_RATE, SPRING_MAX_RATE);
    // The span grows with the slow coefficient instead of the step having to
    // shrink with it, which is what keeps `step * frequency` around four
    // thousandths for every parameter pair above: the integrator is stable
    // everywhere the simulation is allowed to run.
    let step = SPRING_SETTLE_TIME / (rate * f32::from(SPRING_STEPS));
    let steps = t * f32::from(SPRING_STEPS);

    let mut position = 0.0_f32;
    let mut velocity = 0.0_f32;
    for index in 1..=SPRING_STEPS {
        if f32::from(index) > steps {
            break;
        }
        let acceleration = -stiffness * (position - 1.0) - damping * velocity;
        velocity += acceleration * step;
        position += velocity * step;
    }
    position
}

/// Returns the rate at which a spring with these coefficients settles, in
/// inverse seconds.
///
/// The span of a simulation is this rate's inverse times [`SPRING_SETTLE_TIME`].
fn settle_rate(damping: f32, stiffness: f32) -> f32 {
    let discriminant = damping * damping - 4.0 * stiffness;
    if discriminant < 0.0 {
        // Underdamped: the amplitude decays with the envelope, and the
        // oscillation is given room to happen at all — a barely damped spring
        // simulated through a fraction of one period would look like it stalls.
        let frequency = (-discriminant).sqrt() / 2.0;
        (damping / 2.0).max(frequency / TAU)
    } else {
        // Critically damped, or over it: the slower root governs the tail, and
        // the quotient form avoids cancelling `damping - sqrt(discriminant)`
        // when the damping is large and the two are nearly equal.
        2.0 * stiffness / (damping + discriminant.sqrt())
    }
}

/// Returns the bounce curve at `t`: four parabolic arcs, each leaving the
/// height it arrived at.
fn bounce(t: f32) -> f32 {
    const STRETCH: f32 = 7.5625;
    const FOLD: f32 = 2.75;
    if t < 1.0 / FOLD {
        STRETCH * t * t
    } else if t < 2.0 / FOLD {
        let shifted = t - 1.5 / FOLD;
        STRETCH * shifted * shifted + 0.75
    } else if t < 2.5 / FOLD {
        let shifted = t - 2.25 / FOLD;
        STRETCH * shifted * shifted + 0.9375
    } else {
        let shifted = t - 2.625 / FOLD;
        STRETCH * shifted * shifted + 0.984375
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::Property;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A frame's worth of time, so tests read in milliseconds.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Samples `easing` at `samples + 1` points across the unit interval.
    ///
    /// The curves are sampled rather than evaluated at hand-picked `t`s because
    /// what the tests below are about — does it ring, does it settle, is it
    /// monotonic — is a property of the whole shape.
    fn curve(easing: Easing, samples: u16) -> Vec<f32> {
        let denominator = f32::from(samples);
        (0..=samples)
            .map(|index| easing.apply(f32::from(index) / denominator))
            .collect()
    }

    /// Samples a spring across the unit interval.
    fn spring_curve(damping: f32, stiffness: f32, samples: u16) -> Vec<f32> {
        curve(Easing::Spring { damping, stiffness }, samples)
    }

    /// Returns how many times the curve crosses `level` on its way up.
    fn crossings(curve: &[f32], level: f32) -> usize {
        curve
            .windows(2)
            .filter(|pair| (pair[0] < level) != (pair[1] < level))
            .count()
    }

    #[test]
    fn linear_easing_is_the_identity_on_the_unit_interval() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(0.25), 0.25);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
        assert_eq!(Easing::Linear.apply(0.75), 0.75);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
    }

    #[test]
    fn linear_easing_is_held_outside_the_unit_interval() {
        // An animation that overshot its own duration by a millisecond must not
        // interpolate past the value it promised to arrive at.
        assert_eq!(Easing::Linear.apply(-0.5), 0.0);
        assert_eq!(Easing::Linear.apply(1.5), 1.0);
        assert_eq!(
            Easing::EaseIn.apply(-0.5),
            0.0,
            "the clamp is on the input, not on each variant"
        );
    }

    #[test]
    fn ease_in_is_slow_first_and_fast_last() {
        assert_eq!(Easing::EaseIn.apply(0.25), 0.0625);
        assert_eq!(Easing::EaseIn.apply(0.5), 0.25);
        assert_eq!(Easing::EaseIn.apply(0.75), 0.5625);
        assert!(
            Easing::EaseIn.apply(0.5) < Easing::Linear.apply(0.5),
            "half the time has covered less than half the distance"
        );
    }

    #[test]
    fn ease_out_is_fast_first_and_slow_last() {
        assert_eq!(Easing::EaseOut.apply(0.25), 0.4375);
        assert_eq!(Easing::EaseOut.apply(0.5), 0.75);
        assert_eq!(Easing::EaseOut.apply(0.75), 0.9375);
        assert!(
            Easing::EaseOut.apply(0.5) > Easing::Linear.apply(0.5),
            "half the time has covered more than half the distance"
        );
    }

    #[test]
    fn ease_in_out_is_symmetric_about_the_midpoint() {
        assert_eq!(Easing::EaseInOut.apply(0.5), 0.5);
        assert_eq!(
            Easing::EaseInOut.apply(0.25),
            1.0 - Easing::EaseInOut.apply(0.75),
            "the curve either side of the midpoint mirrors"
        );
        let curve: Vec<f32> = (0..=10u16)
            .map(|step| Easing::EaseInOut.apply(f32::from(step) / 10.0))
            .collect();
        assert!(
            curve.windows(2).all(|pair| pair[1] >= pair[0]),
            "and it never comes back down"
        );
    }

    #[test]
    fn every_easing_fixes_both_ends() {
        let variants = [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::Spring {
                damping: 6.0,
                stiffness: 180.0,
            },
            Easing::Bounce,
        ];
        for easing in variants {
            assert_eq!(easing.apply(0.0), 0.0, "{easing:?} does not start at 0");
            assert_eq!(easing.apply(1.0), 1.0, "{easing:?} does not end at 1");
        }
    }

    #[test]
    fn a_non_finite_progress_is_treated_as_the_start() {
        // A caller that has lost track of where it is must not get a nonsense
        // value out of the curve.
        assert_eq!(Easing::Linear.apply(f32::NAN), 0.0);
        assert_eq!(Easing::EaseInOut.apply(f32::NAN), 0.0);
        assert_eq!(Easing::Bounce.apply(f32::NAN), 0.0);
    }

    #[test]
    fn an_underdamped_spring_overshoots_its_target_and_comes_back() {
        // At damping 4 and stiffness 100 the damping ratio is 0.2, so the peak
        // overshoot of a step response is 1 + exp(-0.2 * pi / 0.98) = 1.53, and
        // the spring swings about its target rather than about the start: it
        // comes back to about 0.72, not to less than 0.
        let curve = spring_curve(4.0, 100.0, 400);
        let peak = curve
            .iter()
            .enumerate()
            .max_by(|left, right| {
                left.1
                    .partial_cmp(right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap_or(0);

        assert!(
            curve[peak] > 1.4,
            "it rings well past its target, peaked at {}",
            curve[peak]
        );
        assert!(
            curve[peak..].iter().any(|value| *value < 1.0),
            "and comes back down from that peak"
        );
        assert!(
            curve[peak..].iter().all(|value| *value > 0.0),
            "swinging about the target, not about the start, it never goes \
             below where it began"
        );
        assert!(
            crossings(&curve, 1.0) >= 2,
            "it crosses its target more than once, which is what ringing is: {}",
            crossings(&curve, 1.0)
        );
    }

    #[test]
    fn damping_a_spring_more_stops_it_ringing() {
        let ringing = spring_curve(2.0, 100.0, 400);
        let critical = spring_curve(20.0, 100.0, 400);

        assert!(
            critical.iter().all(|value| *value <= 1.0),
            "critical damping is the fastest approach that does not overshoot"
        );
        assert!(
            critical.windows(2).all(|pair| pair[1] >= pair[0]),
            "and it only ever rises"
        );
        assert!(
            crossings(&critical, 0.5) == 1,
            "it crosses the middle of its travel once, where an underdamped \
             spring crosses it more than once"
        );
        assert!(
            crossings(&critical, 0.5) < crossings(&ringing, 0.5),
            "got {} and {}",
            crossings(&critical, 0.5),
            crossings(&ringing, 0.5)
        );
    }

    #[test]
    fn a_spring_reaches_its_target_within_the_span() {
        // The span follows the settle rate, so a settled spring is at its
        // target by the end whatever its coefficients: the critically damped
        // case at 1 - 7e^-6, and the over-damped one, whose span is set by its
        // slow root rather than by its fast one.
        let critical = spring_curve(20.0, 100.0, 1000);
        assert!(
            critical[999] > 0.98,
            "a critically damped spring has arrived by the end, got {}",
            critical[999]
        );

        let over_damped = spring_curve(60.0, 100.0, 1000);
        assert!(
            over_damped[999] > 0.98,
            "and so has an over-damped one, whose span is four times the \
             critical one but still lands: got {}",
            over_damped[999]
        );
        assert!(
            over_damped.iter().all(|value| *value <= 1.0),
            "with no overshoot to speak of, being past critical damping"
        );
    }

    #[test]
    fn a_spring_answers_the_same_every_time() {
        // The frame loop may ask for the same `t` on two different frames; a
        // curve that integrated from wherever it happened to be would give two
        // answers.
        let easing = Easing::Spring {
            damping: 7.0,
            stiffness: 150.0,
        };
        for step in 0..20u16 {
            let t = f32::from(step) / 20.0;
            assert_eq!(easing.apply(t), easing.apply(t), "{t} is not stable");
        }
    }

    #[test]
    fn a_springs_coefficients_are_sanitized_rather_than_trusted() {
        // The fields are public, so `Easing::Spring` can hold anything at all.
        // A divergent curve would be worse than an arbitrary one.
        for easing in [
            Easing::Spring {
                damping: f32::NAN,
                stiffness: 100.0,
            },
            Easing::Spring {
                damping: -5.0,
                stiffness: 100.0,
            },
            Easing::Spring {
                damping: 10.0,
                stiffness: f32::INFINITY,
            },
            Easing::Spring {
                damping: 10.0,
                stiffness: -1.0,
            },
            Easing::Spring {
                damping: 10.0,
                stiffness: f32::NAN,
            },
            Easing::Spring {
                damping: f32::NAN,
                stiffness: f32::NAN,
            },
            Easing::Spring {
                damping: 10.0,
                stiffness: 1.0e9,
            },
        ] {
            let sampled = curve(easing, 64);
            assert!(
                sampled.iter().all(|value| value.is_finite()),
                "{easing:?} produced a non-finite progress"
            );
            assert_eq!(sampled[0], 0.0, "{easing:?} does not start at 0");
            assert_eq!(sampled[64], 1.0, "{easing:?} does not end at 1");
        }
    }

    #[test]
    fn a_spring_with_unusable_coefficients_falls_back_to_the_documented_value() {
        // Negative damping is a spring that gains energy, which is a divergence
        // rather than a curve; zero is not, and is left alone. Both must give a
        // finite answer, and the first must be the second.
        assert_eq!(
            spring_curve(-5.0, 100.0, 64),
            spring_curve(0.0, 100.0, 64),
            "a negative damping is an undamped one"
        );
        assert_eq!(
            spring_curve(f32::NAN, 100.0, 64),
            spring_curve(SPRING_DEFAULT_DAMPING, 100.0, 64),
            "a non-finite damping is the documented default"
        );
        assert_eq!(
            spring_curve(20.0, f32::NAN, 64),
            spring_curve(20.0, SPRING_DEFAULT_STIFFNESS, 64),
            "and so is a non-finite stiffness"
        );
    }

    #[test]
    fn bounce_touches_its_target_and_dips_away_from_it() {
        // The first arc peaks at 7.5625 * (1/2.75)^2 = 1, so the curve reaches
        // its target — without ever going past it — at 1/2.75 of its span.
        assert_eq!(
            Easing::Bounce.apply(1.0 / 2.75),
            1.0,
            "the first arc lands exactly on the target"
        );
        assert!(
            Easing::Bounce.apply(0.5) < 1.0,
            "and the curve leaves it again: {}",
            Easing::Bounce.apply(0.5)
        );

        let sampled = curve(Easing::Bounce, 256);
        assert!(
            sampled.iter().all(|value| (0.0..=1.0).contains(value)),
            "a bounce never leaves the interval it started in"
        );
        assert!(
            sampled[1..sampled.len() - 1]
                .iter()
                .any(|value| *value > 0.99),
            "it gets all the way to the target before the end, rather than \
             merely getting close"
        );
        assert!(
            sampled.windows(2).any(|pair| pair[1] < pair[0]),
            "and comes back: it is not a curve that only rises"
        );
        assert!(
            sampled[sampled.len() / 2] < 0.9,
            "so half way through it is well back off the target it touched: {}",
            sampled[sampled.len() / 2]
        );
    }

    #[test]
    fn number_animation_interpolates_between_its_endpoints() {
        let opacity = Property::new(0.0_f32);
        let animation = opacity.animate_to(1.0, ms(400), Easing::Linear);

        assert_eq!(animation.progress_at(ms(0)), 0.0);
        assert_eq!(animation.progress_at(ms(100)), 0.25);
        assert_eq!(animation.progress_at(ms(200)), 0.5);
        assert_eq!(animation.value_at(ms(200)), 0.5);
        assert_eq!(
            animation.value_at(ms(400)),
            1.0,
            "the endpoint is the value it promised"
        );
        assert!(animation.is_finished_at(ms(400)));
        assert!(animation.is_running_at(ms(399)));
    }

    #[test]
    fn an_animation_interpolates_downwards_as_well_as_up() {
        let height = Property::new(100.0_f32);
        let animation = height.animate_to(0.0, ms(100), Easing::Linear);

        assert_eq!(animation.value_at(ms(0)), 100.0);
        assert_eq!(animation.value_at(ms(25)), 75.0);
        assert_eq!(animation.value_at(ms(100)), 0.0);
    }

    #[test]
    fn an_animation_holds_its_start_value_through_its_delay() {
        let opacity = Property::new(0.0_f32);
        let animation = opacity
            .animate_to(1.0, ms(100), Easing::Linear)
            .with_delay(ms(200));

        assert_eq!(animation.progress_at(ms(100)), 0.0, "still in the delay");
        assert_eq!(
            animation.progress_at(ms(250)),
            0.5,
            "a quarter of the way in"
        );
        assert_eq!(animation.total_duration(), ms(300));
    }

    #[test]
    fn a_zero_length_animation_is_over_rather_than_infinite() {
        let opacity = Property::new(0.0_f32);
        let animation = opacity.animate_to(1.0, Duration::ZERO, Easing::Linear);

        assert_eq!(
            animation.progress_at(ms(0)),
            1.0,
            "a zero duration completes instead of dividing by zero"
        );
        assert!(animation.is_finished_at(Duration::ZERO));
    }

    #[test]
    fn a_time_before_the_start_of_an_animation_reads_as_the_start() {
        let opacity = Property::new(0.0_f32);
        let animation = opacity
            .animate_to(1.0, ms(100), Easing::Linear)
            .with_start_time(ms(100));

        assert_eq!(animation.progress_at(ms(50)), 0.0);
        assert_eq!(animation.progress_at(ms(150)), 0.5);
        assert_eq!(animation.start_time(), ms(100));
    }

    #[test]
    fn animate_to_takes_the_value_the_property_holds_now() {
        let offset = Property::new(17.0_f32);
        let animation = offset.animate_to(0.0, ms(100), Easing::Linear);

        assert_eq!(
            animation.from(),
            17.0,
            "from wherever it was, not from zero"
        );
        assert_eq!(animation.to(), 0.0);
        assert_eq!(animation.duration(), ms(100));
        assert_eq!(animation.easing(), Easing::Linear);
    }

    #[test]
    fn creating_an_animation_puts_the_property_at_its_start_value() {
        // A frame drawn between the call and the first tick must already show
        // the animation's start, not the value it replaced.
        let offset = Property::new(100.0_f32);
        let _animation = offset.animate_from_to(0.0, 50.0, ms(100), Easing::Linear);

        assert_eq!(offset.get(), 0.0);
    }

    #[test]
    fn color_animation_interpolates_in_premultiplied_space() {
        let black = Property::new(Color::new(0, 0, 0, 255));
        let white = Property::new(Color::new(255, 255, 255, 255));
        let black_to_white = black.animate_to(white.get(), ms(100), Easing::Linear);

        assert_eq!(black_to_white.value_at(ms(50)).r, 128);
        assert_eq!(
            black_to_white.value_at(ms(50)).a,
            255,
            "alpha is a channel too"
        );

        // A colour fading out over an opaque one interpolates towards the
        // premultiplied result rather than the straight one, which is what stops
        // a fading colour bleeding what is behind it.
        let opaque_red = Color::new(255, 0, 0, 255);
        let clear = Color::new(0, 0, 0, 0);
        let fade = Property::new(opaque_red);
        let fade = fade.animate_to(clear, ms(100), Easing::Linear);
        assert_eq!(
            fade.value_at(ms(50)),
            Color::new(128, 0, 0, 128),
            "red and alpha fall together, so the colour never outruns its alpha"
        );
    }

    #[test]
    fn a_color_component_is_clamped_rather_than_wrapped() {
        // `Interpolate` is public and its `t` is not clamped — that is the
        // whole point of it, because an overshooting spring passes one above 1.
        // This is a colour asked for a value well outside its endpoints, which
        // is what such a spring does to one at its peak.
        let from = Color::new(10, 250, 128, 200);
        let to = Color::new(250, 10, 128, 200);

        assert_eq!(
            Color::interpolate(&from, &to, 0.5),
            Color::new(130, 130, 128, 200),
            "inside the endpoints it simply interpolates"
        );
        assert_eq!(
            Color::interpolate(&from, &to, 3.0),
            Color::new(255, 0, 128, 200),
            "past them the channels that left the range are clamped to it, \
             while a channel that stayed is untouched"
        );
        assert_eq!(
            Color::interpolate(&from, &to, -1.0),
            Color::new(0, 255, 128, 200),
            "and the same on the way out the other side"
        );
    }

    #[test]
    fn a_spring_drives_a_colour_through_the_channels_between_its_endpoints() {
        // The same extrapolation through the whole animation machinery rather
        // than by calling `interpolate` directly: an overshooting spring asks
        // for values outside a channel's range at its peak, and the colour
        // still arrives where it was aimed.
        let black = Property::new(Color::new(0, 0, 0, 255));
        let white = Property::new(Color::new(255, 255, 255, 255));
        let animation = black.animate_to(
            white.get(),
            ms(100),
            Easing::Spring {
                damping: 2.0,
                stiffness: 200.0,
            },
        );

        let mut interpolated = false;
        for millis in 0..=100u64 {
            let value = animation.value_at(Duration::from_millis(millis));
            interpolated |= value.r != 0 && value.r != 255;
        }
        assert!(
            interpolated,
            "the spring really did pass through intermediate channels"
        );
        assert_eq!(
            animation.value_at(ms(100)),
            white.get(),
            "and the overshoot does not leave the colour somewhere else"
        );
    }

    #[test]
    fn transform_animation_interpolates_every_field() {
        let identity = Property::new(Transform::identity());
        let moved = Transform {
            tx: 10.0,
            ty: 20.0,
            sx: 2.0,
            sy: 3.0,
            rotation: 1.0,
        };
        let animation = identity.animate_to(moved.clone(), ms(100), Easing::Linear);

        assert_eq!(
            animation.value_at(ms(50)),
            Transform {
                tx: 5.0,
                ty: 10.0,
                sx: 1.5,
                sy: 2.0,
                rotation: 0.5,
            }
        );
        assert_eq!(animation.value_at(ms(100)), moved);
    }

    #[test]
    fn a_clock_writes_every_animation_it_holds() {
        let width = Property::new(0.0_f32);
        let height = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        clock.add(width.animate_to(10.0, ms(100), Easing::Linear));
        clock.add(height.animate_to(20.0, ms(200), Easing::Linear));

        assert!(clock.is_animating());
        assert_eq!(clock.elapsed(), Duration::ZERO);

        assert!(clock.tick(ms(100)));
        assert_eq!(
            (width.get(), height.get()),
            (10.0, 10.0),
            "the shorter animation finished, the other is halfway"
        );

        assert!(clock.tick(ms(100)));
        assert_eq!((width.get(), height.get()), (10.0, 20.0));
        assert!(
            !clock.is_animating(),
            "both have run their course and been dropped"
        );
    }

    #[test]
    fn an_idle_clock_reports_that_it_wrote_nothing() {
        let mut clock = AnimationClock::new();

        assert!(
            !clock.tick(ms(16)),
            "a frame with nothing running must not claim to have painted"
        );
        assert_eq!(clock.elapsed(), ms(16));
    }

    #[test]
    fn a_completed_animation_is_dropped_and_stops_writing() {
        let width = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        clock.add(width.animate_to(10.0, ms(100), Easing::Linear));

        assert!(clock.tick(ms(100)));
        assert_eq!(width.get(), 10.0);
        width.set(0.0);

        assert!(
            !clock.tick(ms(16)),
            "the animation is gone, so nothing wrote the property back"
        );
        assert_eq!(width.get(), 0.0);
    }

    #[test]
    fn clearing_a_clock_leaves_its_properties_where_they_were() {
        let width = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        clock.add(width.animate_to(10.0, ms(100), Easing::Linear));
        let _ = clock.tick(ms(50));
        assert_eq!(width.get(), 5.0);

        clock.clear();

        assert!(!clock.is_animating());
        assert_eq!(
            width.get(),
            5.0,
            "the value it reached is the value it keeps"
        );
        let _ = clock.tick(ms(50));
        assert_eq!(width.get(), 5.0, "and nothing writes it again");
    }

    #[test]
    fn an_animation_stops_when_its_property_is_gone() {
        let width = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        let animation = width.animate_to(10.0, ms(100), Easing::Linear);
        clock.add(animation);
        let _ = clock.tick(ms(10));
        assert!(clock.is_animating());

        // The animation holds a weak handle, so dropping the last `Property`
        // drops the property it was writing.
        drop(width);

        assert!(
            !clock.tick(ms(10)),
            "there is nothing left to write, and the animation is not held open"
        );
        assert!(!clock.is_animating());
    }

    #[test]
    fn an_animated_property_notifies_on_every_frame_it_moves() {
        // This is the path from "an animation wrote a property" to "whatever
        // reads it needs redrawing". A widget cannot be reached from here — the
        // property graph is type-erased and knows nothing about nodes — so the
        // notification is the whole mechanism, and it has to fire every frame
        // rather than on the first one.
        let offset = Property::new(0.0_f32);
        let frames = Rc::new(RefCell::new(Vec::new()));
        let recorder = Rc::clone(&frames);
        offset.on_change(move |value| recorder.borrow_mut().push(*value));

        let mut clock = AnimationClock::new();
        clock.add(offset.animate_to(4.0, ms(40), Easing::Linear));
        for _ in 0..4 {
            let _ = clock.tick(ms(10));
        }

        let seen = frames.borrow().clone();
        assert_eq!(
            seen,
            // The leading 0.0 is `Animation::new` putting the property at its
            // start value before the clock ever ran.
            vec![0.0, 1.0, 2.0, 3.0, 4.0],
            "a widget bound to this property would be dirtied on every frame"
        );
    }

    #[test]
    fn an_animated_property_recomputes_its_dependents_on_every_frame() {
        // The other half of the same path: a widget reads an animated property
        // through a binding rather than through a callback.
        let opacity = Property::new(0.0_f32);
        let source = opacity.clone();
        let doubled = Property::bind(move || source.get() * 2.0);
        let mut clock = AnimationClock::new();
        clock.add(opacity.animate_to(1.0, ms(20), Easing::Linear));

        let _ = clock.tick(ms(10));
        assert_eq!(doubled.get(), 1.0);
        let _ = clock.tick(ms(10));
        assert_eq!(
            doubled.get(),
            2.0,
            "the bound value followed the animation on the second frame too"
        );
    }

    #[test]
    fn a_sequence_runs_its_steps_one_after_another() {
        let first = Property::new(0.0_f32);
        let second = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        let sequence = Sequence::new(vec![
            first.animate_to(1.0, ms(100), Easing::Linear).into(),
            second.animate_to(1.0, ms(100), Easing::Linear).into(),
        ]);
        assert_eq!(sequence.total_duration(), ms(200));
        sequence.play(&mut clock);

        let _ = clock.tick(ms(100));
        assert_eq!(
            (first.get(), second.get()),
            (1.0, 0.0),
            "the second step is only now starting"
        );
        let _ = clock.tick(ms(100));
        assert_eq!((first.get(), second.get()), (1.0, 1.0));
        assert!(!clock.is_animating());
    }

    #[test]
    fn a_sequence_keeps_the_delay_each_step_already_carried() {
        let first = Property::new(0.0_f32);
        let second = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        Sequence::new(vec![first.animate_to(1.0, ms(100), Easing::Linear).into()])
            .then(
                second
                    .animate_to(1.0, ms(100), Easing::Linear)
                    .with_delay(ms(50)),
            )
            .play(&mut clock);

        let _ = clock.tick(ms(100));
        assert_eq!(
            second.get(),
            0.0,
            "the second step has not started: it is still in the delay it was given"
        );
        let _ = clock.tick(ms(50));
        assert_eq!(
            second.get(),
            0.0,
            "and at the moment the sequence reaches it, that delay has just \
             elapsed"
        );
        let _ = clock.tick(ms(50));
        assert_eq!(second.get(), 0.5, "and then half way through its own run");
        let _ = clock.tick(ms(50));
        assert_eq!(second.get(), 1.0, "which ends 250ms after the start");

        assert_eq!(
            Sequence::new(vec![
                first.animate_to(1.0, ms(100), Easing::Linear).into(),
                second
                    .animate_to(1.0, ms(100), Easing::Linear)
                    .with_delay(ms(50))
                    .into(),
            ])
            .total_duration(),
            ms(250),
            "the total carries the last step's own delay"
        );
    }

    #[test]
    fn a_parallel_runs_its_animations_together() {
        let width = Property::new(0.0_f32);
        let colour = Property::new(Color::new(0, 0, 0, 255));
        let mut clock = AnimationClock::new();
        let parallel = Parallel::new(vec![
            width.animate_to(10.0, ms(100), Easing::Linear).into(),
            colour
                .animate_to(Color::new(255, 255, 255, 255), ms(100), Easing::Linear)
                .into(),
        ]);
        assert_eq!(parallel.total_duration(), ms(100));
        parallel.play(&mut clock);

        let _ = clock.tick(ms(50));
        assert_eq!(
            (width.get(), colour.get().r),
            (5.0, 128),
            "both are half way at the same time, one a number and one a colour"
        );
        let _ = clock.tick(ms(50));
        assert_eq!((width.get(), colour.get().r), (10.0, 255));
    }

    #[test]
    fn a_parallel_is_as_long_as_its_longest_member() {
        let parallel = Parallel::new(vec![
            Property::new(0.0_f32)
                .animate_to(1.0, ms(100), Easing::Linear)
                .into(),
            Property::new(0.0_f32)
                .animate_to(1.0, ms(300), Easing::Linear)
                .with_delay(ms(50))
                .into(),
        ]);
        assert_eq!(parallel.total_duration(), ms(350));
        assert_eq!(Parallel::default().total_duration(), Duration::ZERO);
    }

    #[test]
    fn a_stagger_spaces_its_steps_by_its_step() {
        let first = Property::new(0.0_f32);
        let second = Property::new(0.0_f32);
        let third = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        let stagger = Stagger::new(
            vec![
                first.animate_to(1.0, ms(100), Easing::Linear).into(),
                second.animate_to(1.0, ms(100), Easing::Linear).into(),
                third.animate_to(1.0, ms(100), Easing::Linear).into(),
            ],
            ms(50),
        );
        assert_eq!(stagger.step(), ms(50));
        assert_eq!(stagger.total_duration(), ms(200));
        stagger.play(&mut clock);

        let _ = clock.tick(ms(50));
        assert_eq!(
            (first.get(), second.get(), third.get()),
            (0.5, 0.0, 0.0),
            "the first is half way through its own 100ms and the others have \
             not started"
        );
        let _ = clock.tick(ms(50));
        assert_eq!(
            (first.get(), second.get(), third.get()),
            (1.0, 0.5, 0.0),
            "the first has arrived, the second is half way, the third has only \
             just started"
        );
        let _ = clock.tick(ms(50));
        assert_eq!(
            (first.get(), second.get(), third.get()),
            (1.0, 1.0, 0.5),
            "and the third is on its own at last"
        );
        let _ = clock.tick(ms(50));
        assert_eq!((first.get(), second.get(), third.get()), (1.0, 1.0, 1.0));
        assert!(
            !clock.is_animating(),
            "the last one landing is the end of the whole thing"
        );
    }

    #[test]
    fn a_stagger_measures_its_offset_from_the_moment_it_is_played() {
        let first = Property::new(0.0_f32);
        let second = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        // The clock has been running already, so the animations start at its
        // elapsed time rather than at zero.
        let _ = clock.tick(ms(500));
        Stagger::new(
            vec![
                first.animate_to(1.0, ms(100), Easing::Linear).into(),
                second.animate_to(1.0, ms(100), Easing::Linear).into(),
            ],
            ms(100),
        )
        .play(&mut clock);

        let _ = clock.tick(ms(50));
        assert_eq!(
            (first.get(), second.get()),
            (0.5, 0.0),
            "the stagger counts from when it was played"
        );
    }

    #[test]
    fn two_animations_on_one_property_write_in_the_order_they_were_added() {
        // A clock cannot tell that two animations are writing the same property:
        // the properties cannot be compared through the erased value type. The
        // behaviour is the documented one — the last tick wins.
        let width = Property::new(0.0_f32);
        let mut clock = AnimationClock::new();
        clock.add(width.animate_to(10.0, ms(100), Easing::Linear));
        clock.add(width.animate_to(100.0, ms(100), Easing::Linear));

        let _ = clock.tick(ms(100));
        assert_eq!(
            width.get(),
            100.0,
            "the animation added last has the last word"
        );
    }
}
