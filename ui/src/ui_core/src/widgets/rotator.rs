//! The Rotator widget: two drag-driven angles, and nothing else.
//!
//! A rotator is a yaw about y and a pitch about x — two [`Property`] values a
//! drag writes, with the pitch clamped. There is no matrix here, no projection,
//! no mesh, no `GL_` call and no paint: the car is drawn by
//! [`DrawCommand::Mesh`](crate::paint::DrawCommand::Mesh) through the caller's
//! own `Mat4`, and a two-axis value with no surface is a state holder. That
//! split is what makes every line of this module unit-testable, which is the
//! only verification this host can offer for the interaction.
//!
//! [`Rotator::yaw`] is **unbounded and never wrapped**. Yaw is periodic and
//! wrapping it introduces a discontinuity at `±π` that an animated value would
//! cross and an equality assertion would see; nothing needs a bounded yaw,
//! because the model the demo orbits is closed in yaw, so there is no "wrong
//! side" for it to be on.
//!
//! [`Rotator::pitch`] is **clamped** to `-ROTOR_PITCH_LIMIT..=ROTOR_PITCH_LIMIT`.
//! The measured car is 1.50 m wide × 2.55 m long × 1.30 m tall, so its
//! projected silhouette is square at `atan(1.30 / (2.55 / 2)) ≈ 45.5°` — at or
//! above that angle the view reads as a plan view of the roof and the four
//! wheels are four indistinguishable discs. 30° is comfortably below 45.5° and
//! is a round number in degrees, and it is the angle at which the roof and both
//! flanks are visible at once, which is the whole reason a second axis exists.
//! The clamp is symmetric rather than `0.0..=30°`: a drag whose two directions
//! are not equivalent is a drag that feels broken, and the pipeline draws no
//! floor while the model's shells are closed, so the negative end shows
//! geometry rather than a hole. What would reverse either half is a measurement
//! that 30° hides a surface the demo needs to show.
//!
//! The clamp is mandatory rather than cosmetic. `Mat4::rotated_x` is finite at
//! every angle — including exactly `±π/2`, where the car's length projects to
//! nothing and the picture is a 1.50 m sliver — so an unclamped pitch produces
//! a silently wrong picture with no GL error and a green suite. The one write
//! path below is what stops that.
//!
//! The mapping is grab-and-turn: a horizontal drag turns the yaw, a vertical
//! drag tilts the pitch, a diagonal drag moves both, and there is no axis
//! locking. `yaw` **decreases** as `delta.x` increases — drag right and the
//! car's nose goes right, which from above is clockwise, which means the camera
//! orbits the other way — and `pitch` **increases** as `delta.y` increases,
//! because window space is y-down and a drag down raises the camera and shows
//! more roof. This is deliberately **not** [`Scroll`](crate::widgets::scroll)
//! 's "down is later": that convention was chosen because the wheel arm
//! disagreed with content-follows-the-finger and one control with two
//! directions is worse than either alone. The rotator has no second input, so
//! the tie that decided `Scroll` does not exist here, and grab-and-turn is the
//! only convention every 3D viewer uses.
//!
//! A crate widget rather than demo-local state, for four reasons.
//! [`Scroll`](crate::widgets::scroll) is the precedent for a drag-driven `Property`, and
//! [`Slider`](crate::widgets::slider::Slider) is the closer one —
//! `dragging: Property<bool>` written by the caller from a press, an `on_event`
//! whose `KeyDown` arm requires `focused`, a private helper mapping arrows and
//! the d-pad to a signed step — and reusing that shape means a reviewer has one
//! shape to check rather than a new one. A rotator with no 3D in it is the
//! reusable part, and it is most of the thing. A demo-local struct cannot close
//! a row of a table about `ui_core`: row `L4` is phrased about widgets, and a
//! struct `input::route` can never reach leaves the row true afterwards. And
//! the camera's placement is the demo's: `Mat4` has no `look_at`, so the demo
//! owns the eye, the projection and the matrices.
//!
//! There is no inertia, no fling, no momentum, no decay, no snap and no
//! release animation. The crate has none of those anywhere — no `inertia`,
//! `momentum` or `flick` identifier exists — so a rotator that kept moving
//! after the last event would be the one control with a velocity. A snap needs
//! a preset set the demo has no source for, and a release animation needs a
//! rest pose: the demo's resting angles are an arbitrary flattering choice for
//! the first frame, not a pose the product has. The car stops where the finger
//! stopped.
//!
//! The drag arm does not read `rect`: a rotation is a delta, not a position,
//! and the rect is taken only because the `on_event` shape every sibling uses
//! carries one.

use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::LayoutState;
use crate::node::{self, WidgetNode};
use crate::paint::Rect;
use crate::property::Property;

/// Radians per pixel of drag — `0.0025`, about 0.143°/px.
///
/// Across the demo's `WINDOW` width of 1280 px a full-width drag turns the car
/// `1280 × 0.0025 = 3.2 rad ≈ 183°`, so a comfortable quarter turn is about
/// **630 px**, a bit under half the panel — the reach a thumb covers on a
/// centre display — and **209 px** of vertical travel reaches the pitch limit,
/// which is why the clamp is reachable in well under a third of a half-height
/// drag and therefore not a corner case.
pub const ROTOR_SENSITIVITY: f32 = 0.0025;

/// The pitch clamp, symmetric about zero — 30°.
///
/// Carries the `atan(1.30 / 1.275) ≈ 45.5°` silhouette-square argument from
/// the module doc: 30° is comfortably below the angle at which the car reads
/// as a plan view of its own roof, and it is symmetric because a drag whose
/// two directions are not equivalent feels broken. What would reverse it is a
/// measurement that 30° hides a surface the demo needs to show.
pub const ROTOR_PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_6;

/// Radians per key press — `0.0873`, 5°.
///
/// [`Slider`](crate::widgets::slider::Slider)'s own step is `5.0` of
/// `0..=100`, so "five" is the number this crate already reaches for.
pub const ROTOR_KEY_STEP: f32 = 0.0873;

/// The yaw [`Rotator::new`] starts at: zero, facing the camera's rest pose.
///
/// Zero rather than the demo's resting angle, because the resting angle is the
/// demo's — which way the model's nose points is recorded nowhere in this
/// repository, so a crate widget cannot know which yaw flatters it. The demo
/// aims at its own rest pose once, through [`Rotator::rotate_by`], after
/// construction.
const YAW_REST: f32 = 0.0;

/// The pitch [`Rotator::new`] starts at: level.
///
/// Zero for the reason [`YAW_REST`] is: the rest pose is the caller's, and the
/// widget starts at the centre of its clamped range.
const PITCH_REST: f32 = 0.0;

/// Two angles — a yaw about y and a clamped pitch about x — and nothing else.
///
/// `Clone` is derived; `Debug` is written out below, because
/// `Property` is `Clone` and not `Debug` — which
/// is also why [`Slider`](crate::widgets::slider::Slider),
/// [`Toggle`](crate::widgets::toggle::Toggle) and
/// [`Scroll`](crate::widgets::scroll::Scroll) carry neither attribute.
#[derive(Clone)]
pub struct Rotator {
    /// Yaw in radians, **unbounded and never wrapped**.
    pub yaw: Property<f32>,
    /// Pitch in radians, **clamped** to `-ROTOR_PITCH_LIMIT..=ROTOR_PITCH_LIMIT`.
    pub pitch: Property<f32>,
    /// Whether a pointer is down on this rotator. **Written by the caller**
    /// from a press and a release, never by the widget.
    ///
    /// The gesture recogniser reports a tap on the *release*, so the flag has
    /// to come from `MouseButtonDown` / `FingerDown` and its counterparts —
    /// the same reason [`Slider`](crate::widgets::slider::Slider)'s doc gives
    /// for its own `dragging`.
    pub dragging: Property<bool>,
    /// Whether the rotator holds focus. Written by the caller from
    /// [`input::Focus`](crate::input::Focus), as `Slider::focused`'s doc says.
    pub focused: Property<bool>,
    sensitivity: f32,
    pitch_limit: f32,
    node: Handle,
}

impl std::fmt::Debug for Rotator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rotator")
            .field("yaw", &self.yaw.get())
            .field("pitch", &self.pitch.get())
            .field("dragging", &self.dragging.get())
            .field("focused", &self.focused.get())
            .finish()
    }
}

impl Rotator {
    /// Creates a rotator in the arena, and returns it.
    ///
    /// The yaw starts at `YAW_REST`, the pitch at `PITCH_REST`, `dragging`
    /// and `focused` at false, `sensitivity` at [`ROTOR_SENSITIVITY`] and the
    /// pitch limit at [`ROTOR_PITCH_LIMIT`]. `new` takes no tuning arguments:
    /// a widget whose limits are arguments is a widget whose limits two callers
    /// disagree about, and this project has one rotator.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>) -> Self {
        let node = node::create(nodes, LayoutState::new());
        Rotator {
            yaw: Property::new(YAW_REST),
            pitch: Property::new(PITCH_REST),
            dragging: Property::new(false),
            focused: Property::new(false),
            sensitivity: ROTOR_SENSITIVITY,
            pitch_limit: ROTOR_PITCH_LIMIT,
            node,
        }
    }

    /// Returns the rotator's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns where the yaw ends: nowhere, so infinity.
    ///
    /// A caller placing a needle next to the rotator needs to know where the
    /// ends are and there is no other way to ask; the yaw has no ends, so this
    /// answers infinity rather than inventing one.
    #[must_use]
    pub fn yaw_limit(&self) -> f32 {
        f32::INFINITY
    }

    /// Returns where the pitch ends: [`ROTOR_PITCH_LIMIT`].
    ///
    /// The same question as [`Rotator::yaw_limit`], with an answer rather than
    /// an absence: a needle beside the rotator stops here.
    #[must_use]
    pub fn pitch_limit(&self) -> f32 {
        self.pitch_limit
    }

    /// Moves both angles by the two deltas, clamping the pitch, and reports
    /// whether either value moved.
    ///
    /// **This is the single write path**: a drag calls it, a key press calls
    /// it, and a caller's own integration — such as the demo's ambient turn —
    /// calls it. That is what makes "one value, two producers" a structural
    /// fact rather than a sentence: there is no second path to either
    /// property, so the producers cannot bypass the clamp and cannot be
    /// reordered by accident. [`Scroll::scroll_by`](crate::widgets::scroll::Scroll::scroll_by)
    /// is the precedent and its shape is the model: it clamps, writes the
    /// settled value and returns the new offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::rotator::{Rotator, ROTOR_PITCH_LIMIT};
    ///
    /// let mut nodes = Arena::new();
    /// let rotator = Rotator::new(&mut nodes);
    /// assert!(rotator.rotate_by(0.5, 0.0));
    /// assert_eq!(rotator.yaw.get(), 0.5);
    /// let _ = rotator.rotate_by(0.0, 100.0);
    /// assert_eq!(rotator.pitch.get(), ROTOR_PITCH_LIMIT);
    /// ```
    #[must_use]
    pub fn rotate_by(&self, delta_yaw: f32, delta_pitch: f32) -> bool {
        let yaw_before = self.yaw.get();
        let pitch_before = self.pitch.get();
        let pitch = clamp_pitch(pitch_before + delta_pitch, self.pitch_limit);
        self.yaw.set(yaw_before + delta_yaw);
        self.pitch.set(pitch);
        self.yaw.get() != yaw_before || self.pitch.get() != pitch_before
    }

    /// Handles `event` as this rotator would inside `rect`, and reports
    /// whether it consumed it.
    ///
    /// A [`Drag`](InputEventKind::Drag) needs no focus — a drag is positional
    /// and [`Scroll`](crate::widgets::scroll::Scroll) takes one from an
    /// unfocused scroller — and `rect` is **not read by this arm**: a rotation
    /// is a delta, not a position. A drag with no component at all is
    /// declined, `Scroll`'s rule and for `Scroll`'s reason: it changes nothing,
    /// and a widget that swallows one is a widget something behind it can no
    /// longer have. Otherwise the event is consumed and both axes move — a
    /// diagonal drag moves both, which is the deliberate opposite of `Scroll`'s
    /// horizontal refusal: refusing an axis here is refusing the feature.
    ///
    /// An arrow key, or the gamepad's d-pad, nudges both axes by
    /// [`ROTOR_KEY_STEP`] — and is consumed **only while the rotator holds
    /// focus**, because a key press is not routed by position and every arrow
    /// would otherwise move every rotator on screen. A key the rotator does
    /// not use is left alone and not consumed, so it carries on up the tree.
    ///
    /// Seven variants reach `_ => false` and are declined for the reasons
    /// beside them: `Tap` carries no delta for a rotation to read; `LongPress`
    /// and `Swipe` belong to the discrete threshold-based decisions row `L4`'s
    /// Blocks column names — dock edit mode, card paging, alert dismissal, the
    /// drive-mode strip — and a `Swipe` carries a direction and no magnitude,
    /// so mapping it onto a rotation means inventing one; `Pinch` is a zoom
    /// and a zoom is a projection change (`Mat4`'s `fov_y`, or `near`/`far`),
    /// not a rotation; `KeyUp` ends nothing the widget started, because the
    /// held state is the caller's; `Scroll` is declined because there is no
    /// documented wheel gesture for the visualisation; and `Text` types
    /// nothing a camera answers to.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind};
    /// use ui_core::layout::Offset;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::rotator::{Rotator, ROTOR_SENSITIVITY};
    ///
    /// let mut nodes = Arena::new();
    /// let rotator = Rotator::new(&mut nodes);
    /// let rect = Rect::new(60.0, 240.0, 560.0, 400.0);
    /// let mut drag = InputEvent::new(
    ///     InputEventKind::Drag {
    ///         delta: Offset::new(60.0, 0.0),
    ///     },
    ///     Some(Offset::new(100.0, 300.0)),
    /// );
    /// assert!(rotator.on_event(&mut drag, rect));
    /// assert_eq!(rotator.yaw.get(), -60.0 * ROTOR_SENSITIVITY);
    /// ```
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        // `rect` is deliberately unread: see the module doc. It is named
        // rather than `_rect` to keep the `on_event` shape every sibling
        // uses, so the reason has to be written down instead of read off it.
        let _ = rect;
        match event.kind() {
            InputEventKind::Drag { delta } => {
                if delta.x == 0.0 && delta.y == 0.0 {
                    return false;
                }
                event.consume();
                let _ = self.rotate_by(-self.sensitivity * delta.x, self.sensitivity * delta.y);
                true
            }
            InputEventKind::KeyDown { key, .. } => {
                if !self.focused.get() {
                    return false;
                }
                let Some((delta_yaw, delta_pitch)) = rotation_key(&key) else {
                    return false;
                };
                event.consume();
                let _ = self.rotate_by(delta_yaw, delta_pitch);
                true
            }
            _ => false,
        }
    }
}

/// Maps an arrow or d-pad key to a `(delta_yaw, delta_pitch)` nudge, or `None`
/// for a key the rotator does not use.
///
/// In [`Slider::adjustment`](crate::widgets::slider::Slider)'s shape. The sign
/// convention is one sentence: **an arrow is the drag in that direction** —
/// Right is a rightward drag so the yaw decreases, Down is a downward drag so
/// the pitch increases — and the key-path tests pin both halves of it.
fn rotation_key(key: &Key) -> Option<(f32, f32)> {
    use sdl3::gamepad::Button as Pad;
    use sdl3::keyboard::Keycode;
    match key {
        Key::Keyboard(Keycode::Left) | Key::Gamepad(Pad::DPadLeft) => Some((ROTOR_KEY_STEP, 0.0)),
        Key::Keyboard(Keycode::Right) | Key::Gamepad(Pad::DPadRight) => {
            Some((-ROTOR_KEY_STEP, 0.0))
        }
        Key::Keyboard(Keycode::Up) | Key::Gamepad(Pad::DPadUp) => Some((0.0, -ROTOR_KEY_STEP)),
        Key::Keyboard(Keycode::Down) | Key::Gamepad(Pad::DPadDown) => Some((0.0, ROTOR_KEY_STEP)),
        _ => None,
    }
}

/// Returns `pitch` clamped to `-limit..=limit`, or `0.0` for a non-finite
/// input.
///
/// In [`Scroll::bounded`](crate::widgets::scroll::Scroll)'s shape and beside
/// the module's other helpers rather than imported from `scroll.rs`: that
/// clamp is a scroll-specific two-value clamp for a fraction, and a shared
/// module for one caller is what `developer.md` refuses. A non-finite pitch
/// is `NaN`, and a `NaN` in a matrix makes the geometry it belongs to
/// disappear with no GL error — the arithmetic task 36 requires of
/// `perspective` and `orthographic`, where a non-finite angle is `inf` and
/// not a panic. A non-finite limit answers the same way, because `f32::clamp`
/// on a `NaN` bound is a panic and a widget must not take a frame down.
fn clamp_pitch(pitch: f32, limit: f32) -> f32 {
    if !pitch.is_finite() || !limit.is_finite() {
        return 0.0;
    }
    pitch.clamp(-limit, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Offset;
    use std::f32::consts::{FRAC_PI_4, TAU};

    /// The box every test offers the rotator: deliberately off the origin, so
    /// a test that read the origin as an extent would fail here rather than
    /// pass at `(0, 0)` — and the drag arm never reads it at all.
    const RECT: Rect = Rect {
        x: 60.0,
        y: 240.0,
        width: 560.0,
        height: 400.0,
    };

    /// A rotator with its arena held alive beside it.
    fn fresh_rotator() -> (Arena<WidgetNode>, Rotator) {
        let mut nodes = Arena::new();
        let rotator = Rotator::new(&mut nodes);
        (nodes, rotator)
    }

    /// A drag of `delta` at the box's centre.
    fn drag(delta: Offset) -> InputEvent {
        InputEvent::new(
            InputEventKind::Drag { delta },
            Some(Offset::new(
                RECT.x + RECT.width / 2.0,
                RECT.y + RECT.height / 2.0,
            )),
        )
    }

    /// A key-down of `key`, with no position: keys are not positional.
    fn key_down(key: Key) -> InputEvent {
        InputEvent::new(
            InputEventKind::KeyDown {
                key,
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        )
    }

    #[test]
    fn a_horizontal_drag_turns_the_yaw_and_leaves_the_pitch_alone() {
        let (_nodes, rotator) = fresh_rotator();
        let pitch_before = rotator.pitch.get();
        let mut event = drag(Offset::new(60.0, 0.0));
        assert!(rotator.on_event(&mut event, RECT));
        assert!(event.consumed());
        assert_eq!(
            rotator.yaw.get(),
            -60.0 * ROTOR_SENSITIVITY,
            "drag right turns the yaw down by the drag times the sensitivity"
        );
        assert_eq!(
            rotator.pitch.get(),
            pitch_before,
            "and the pitch is bit-identical to its value before the call"
        );
    }

    #[test]
    fn a_vertical_drag_tilts_the_pitch_and_leaves_the_yaw_alone() {
        let (_nodes, rotator) = fresh_rotator();
        let yaw_before = rotator.yaw.get();
        let mut event = drag(Offset::new(0.0, 40.0));
        assert!(rotator.on_event(&mut event, RECT));
        assert!(event.consumed());
        assert_eq!(
            rotator.pitch.get(),
            40.0 * ROTOR_SENSITIVITY,
            "drag down raises the pitch by the drag times the sensitivity"
        );
        assert_eq!(
            rotator.yaw.get(),
            yaw_before,
            "and the yaw is bit-identical to its value before the call"
        );
    }

    #[test]
    fn a_diagonal_drag_moves_both_axes() {
        // The contrast with `Scroll`'s horizontal refusal, and the name says
        // what it is for: a two-axis control that refused an axis would be
        // refusing the feature.
        let (_nodes, rotator) = fresh_rotator();
        let mut event = drag(Offset::new(60.0, 40.0));
        assert!(rotator.on_event(&mut event, RECT));
        assert!(event.consumed());
        assert_eq!(rotator.yaw.get(), -60.0 * ROTOR_SENSITIVITY);
        assert_eq!(rotator.pitch.get(), 40.0 * ROTOR_SENSITIVITY);
    }

    #[test]
    fn the_pitch_cannot_exceed_its_clamp_under_an_extreme_drag() {
        // Two halves: a single enormous drag, and a thousand small ones that
        // would walk a leaking clamp past its end one pixel at a time.
        let (_nodes, rotator) = fresh_rotator();
        let mut event = drag(Offset::new(0.0, 100_000.0));
        assert!(rotator.on_event(&mut event, RECT));
        assert!(
            rotator.pitch.get() <= ROTOR_PITCH_LIMIT && rotator.pitch.get() >= -ROTOR_PITCH_LIMIT,
            "a single 100 000 px drag stops at the clamp, at {}",
            rotator.pitch.get()
        );
        let settled = rotator.pitch.get();
        let mut further = drag(Offset::new(0.0, 1_000.0));
        assert!(rotator.on_event(&mut further, RECT));
        assert_eq!(
            rotator.pitch.get(),
            settled,
            "and a further drag in the same direction leaves it unchanged"
        );

        let (_nodes, slow) = fresh_rotator();
        for _ in 0..1_000 {
            let mut step = drag(Offset::new(0.0, 1.0));
            assert!(slow.on_event(&mut step, RECT));
        }
        assert!(
            slow.pitch.get() <= ROTOR_PITCH_LIMIT && slow.pitch.get() >= -ROTOR_PITCH_LIMIT,
            "and so do a thousand 1 px drags, at {}",
            slow.pitch.get()
        );
        let settled = slow.pitch.get();
        let mut further = drag(Offset::new(0.0, 1.0));
        assert!(slow.on_event(&mut further, RECT));
        assert_eq!(
            slow.pitch.get(),
            settled,
            "with no slow leak past the end either"
        );
    }

    #[test]
    fn the_pitch_is_clamped_at_both_ends() {
        // The negative end, in the same two-half shape: the clamp is
        // symmetric, so a test of one end alone would pass for a clamp that
        // forgot the sign.
        let (_nodes, rotator) = fresh_rotator();
        let mut event = drag(Offset::new(0.0, -100_000.0));
        assert!(rotator.on_event(&mut event, RECT));
        assert!(
            rotator.pitch.get() <= ROTOR_PITCH_LIMIT && rotator.pitch.get() >= -ROTOR_PITCH_LIMIT,
            "a single −100 000 px drag stops at the clamp, at {}",
            rotator.pitch.get()
        );
        let settled = rotator.pitch.get();
        let mut further = drag(Offset::new(0.0, -1_000.0));
        assert!(rotator.on_event(&mut further, RECT));
        assert_eq!(
            rotator.pitch.get(),
            settled,
            "and a further drag in the same direction leaves it unchanged"
        );

        let (_nodes, slow) = fresh_rotator();
        for _ in 0..1_000 {
            let mut step = drag(Offset::new(0.0, -1.0));
            assert!(slow.on_event(&mut step, RECT));
        }
        assert!(
            slow.pitch.get() <= ROTOR_PITCH_LIMIT && slow.pitch.get() >= -ROTOR_PITCH_LIMIT,
            "and so do a thousand −1 px drags, at {}",
            slow.pitch.get()
        );
        let settled = slow.pitch.get();
        let mut further = drag(Offset::new(0.0, -1.0));
        assert!(slow.on_event(&mut further, RECT));
        assert_eq!(
            slow.pitch.get(),
            settled,
            "with no slow leak past the negative end either"
        );
    }

    #[test]
    fn clamp_pitch_returns_zero_for_a_non_finite_pitch() {
        // Both branches: the finite one clamps, and the non-finite one
        // answers zero rather than letting a `NaN` into a matrix — where it
        // would make the geometry disappear with no GL error.
        assert_eq!(
            clamp_pitch(100.0, ROTOR_PITCH_LIMIT),
            ROTOR_PITCH_LIMIT,
            "a finite pitch past the end stops at it"
        );
        assert_eq!(
            clamp_pitch(-100.0, ROTOR_PITCH_LIMIT),
            -ROTOR_PITCH_LIMIT,
            "on the negative end too"
        );
        assert_eq!(
            clamp_pitch(0.1, ROTOR_PITCH_LIMIT),
            0.1,
            "and a finite pitch inside the range passes through"
        );
        assert_eq!(
            clamp_pitch(f32::NAN, ROTOR_PITCH_LIMIT),
            0.0,
            "a `NaN` pitch is zero rather than a hole in the picture"
        );
        assert_eq!(
            clamp_pitch(f32::INFINITY, ROTOR_PITCH_LIMIT),
            0.0,
            "and so is an infinite one"
        );
        assert_eq!(
            clamp_pitch(0.1, f32::NAN),
            0.0,
            "a non-finite limit answers the same way rather than panicking \
             inside `f32::clamp`"
        );
    }

    #[test]
    fn a_drag_with_no_component_is_not_consumed() {
        let (_nodes, rotator) = fresh_rotator();
        let yaw_before = rotator.yaw.get();
        let pitch_before = rotator.pitch.get();
        let mut event = drag(Offset::new(0.0, 0.0));
        assert!(
            !rotator.on_event(&mut event, RECT),
            "a zero-delta drag changes nothing and is declined"
        );
        assert!(
            !event.consumed(),
            "so whatever is behind the rotator can still have it"
        );
        assert_eq!(rotator.yaw.get(), yaw_before);
        assert_eq!(rotator.pitch.get(), pitch_before);
    }

    #[test]
    fn the_yaw_is_not_wrapped() {
        // A drag past a full turn must not snap back: yaw is periodic and a
        // wrap would put a discontinuity at `±π` that an animated value would
        // cross. 3 000 px at the sensitivity is 7.5 rad, past one full turn.
        let (_nodes, rotator) = fresh_rotator();
        let mut event = drag(Offset::new(3_000.0, 0.0));
        assert!(rotator.on_event(&mut event, RECT));
        assert_eq!(rotator.yaw.get(), -3_000.0 * ROTOR_SENSITIVITY);
        assert!(
            rotator.yaw.get() < -TAU,
            "the yaw is still past −2π at {}, not wrapped back near zero",
            rotator.yaw.get()
        );
    }

    #[test]
    fn the_widget_consumes_nothing_but_a_drag() {
        use sdl3::gamepad::Button as Pad;
        use sdl3::keyboard::Keycode;
        // All nine `InputEventKind` variants, each offered twice — unfocused
        // and focused — so the table says which of the eighteen cases consume.
        // Only a drag, and a focused key the rotator uses, may take one.
        let drag = InputEventKind::Drag {
            delta: Offset::new(60.0, 0.0),
        };
        let focused_key = InputEventKind::KeyDown {
            key: Key::Keyboard(Keycode::Right),
            keymod: sdl3::keyboard::Mod::empty(),
        };
        let cases: Vec<(&str, InputEventKind, bool, bool)> = vec![
            ("Tap", InputEventKind::Tap, false, false),
            ("LongPress", InputEventKind::LongPress, false, false),
            (
                "Swipe",
                InputEventKind::Swipe {
                    direction: crate::input::SwipeDirection::Left,
                },
                false,
                false,
            ),
            ("Pinch", InputEventKind::Pinch { scale: 1.5 }, false, false),
            ("Drag", drag, true, true),
            ("KeyDown", focused_key, false, true),
            (
                "KeyUp",
                InputEventKind::KeyUp {
                    key: Key::Keyboard(Keycode::Right),
                    keymod: sdl3::keyboard::Mod::empty(),
                },
                false,
                false,
            ),
            (
                "Scroll",
                InputEventKind::Scroll {
                    delta: Offset::new(0.0, 1.0),
                },
                false,
                false,
            ),
            (
                "Text",
                InputEventKind::Text {
                    text: String::from("a"),
                },
                false,
                false,
            ),
        ];
        for (name, kind, unfocused, focused) in &cases {
            for (is_focused, wanted) in [(false, *unfocused), (true, *focused)] {
                let (_nodes, rotator) = fresh_rotator();
                rotator.focused.set(is_focused);
                let position = Some(Offset::new(
                    RECT.x + RECT.width / 2.0,
                    RECT.y + RECT.height / 2.0,
                ));
                let mut event = InputEvent::new(kind.clone(), position);
                assert_eq!(
                    rotator.on_event(&mut event, RECT),
                    wanted,
                    "{name} with focused={is_focused}"
                );
                assert_eq!(
                    event.consumed(),
                    wanted,
                    "{name} with focused={is_focused} leaves the flag to match"
                );
            }
        }
        // `Swipe` and `LongPress` by name: the two row `L4` is waiting on, so
        // a later task that consumes either must break this test deliberately
        // rather than drift past a claim in a document.
        for kind in [
            InputEventKind::Swipe {
                direction: crate::input::SwipeDirection::Up,
            },
            InputEventKind::LongPress,
        ] {
            let (_nodes, rotator) = fresh_rotator();
            rotator.focused.set(true);
            let mut event = InputEvent::new(kind, None);
            assert!(
                !rotator.on_event(&mut event, RECT),
                "a focused rotator still declines it"
            );
            assert!(!event.consumed());
        }
        // The d-pad by name too: the second half of the `KeyDown` the table
        // counts as consumed.
        let (_nodes, rotator) = fresh_rotator();
        rotator.focused.set(true);
        let mut pad = InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Gamepad(Pad::DPadUp),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        );
        assert!(rotator.on_event(&mut pad, RECT));
        assert!(pad.consumed());
    }

    #[test]
    fn the_arrow_keys_nudge_both_axes_and_stop_at_the_same_clamp() {
        use sdl3::gamepad::Button as Pad;
        use sdl3::keyboard::Keycode;
        let (_nodes, rotator) = fresh_rotator();
        rotator.focused.set(true);
        let mut right = key_down(Key::Keyboard(Keycode::Right));
        assert!(rotator.on_event(&mut right, RECT));
        assert!(right.consumed());
        assert_eq!(
            rotator.yaw.get(),
            -ROTOR_KEY_STEP,
            "Right is a rightward drag: the yaw decreases by one step"
        );
        let mut left = key_down(Key::Gamepad(Pad::DPadLeft));
        assert!(rotator.on_event(&mut left, RECT));
        assert_eq!(rotator.yaw.get(), 0.0, "and the d-pad's Left walks it back");
        let mut down = key_down(Key::Keyboard(Keycode::Down));
        assert!(rotator.on_event(&mut down, RECT));
        assert_eq!(
            rotator.pitch.get(),
            ROTOR_KEY_STEP,
            "Down is a downward drag: the pitch increases by one step"
        );
        let mut up = key_down(Key::Gamepad(Pad::DPadUp));
        assert!(rotator.on_event(&mut up, RECT));
        assert_eq!(rotator.pitch.get(), 0.0, "and Up walks it back");
        // Thirty presses at 5° each is 150° of asking against a 30° clamp, so
        // the key path stopping at the same end proves it clamps identically.
        for _ in 0..30 {
            let mut press = key_down(Key::Keyboard(Keycode::Down));
            assert!(rotator.on_event(&mut press, RECT));
        }
        assert_eq!(
            rotator.pitch.get(),
            ROTOR_PITCH_LIMIT,
            "thirty Downs stop at the same clamp the drag stops at"
        );
    }

    #[test]
    fn an_unfocused_rotator_declines_a_key_and_lets_it_travel_on() {
        use sdl3::keyboard::Keycode;
        // In `Slider::on_event`'s own doc-example shape: decline first, then
        // the flag, then the value.
        let (_nodes, rotator) = fresh_rotator();
        let mut right = key_down(Key::Keyboard(Keycode::Right));
        assert!(
            !rotator.on_event(&mut right, RECT),
            "an unfocused rotator ignores it"
        );
        assert!(!right.consumed(), "and lets it travel on");
        assert_eq!(rotator.yaw.get(), 0.0);
        assert_eq!(rotator.pitch.get(), 0.0);
        // And a key the rotator has no mapping for is declined even focused,
        // so it carries on up the tree.
        rotator.focused.set(true);
        let mut tab = key_down(Key::Keyboard(Keycode::Tab));
        assert!(!rotator.on_event(&mut tab, RECT));
        assert!(!tab.consumed());
    }

    #[test]
    fn the_three_constants_are_positive_finite_and_under_a_quarter_turn() {
        // The silhouette-square argument as an assertion: a limit at or above
        // `atan(1.30 / 1.275) ≈ 45.5°` defeats the reason the clamp exists, so
        // widening it past a quarter turn fails here rather than passing
        // quietly. Read through a constructed widget rather than off the
        // constants: that pins the constructor's wiring as well as the
        // numbers, and a bare `assert!` on two constants is a constant
        // assertion clippy refuses.
        let (_nodes, rotator) = fresh_rotator();
        assert_eq!(ROTOR_SENSITIVITY, 0.0025);
        assert_eq!(ROTOR_PITCH_LIMIT, std::f32::consts::FRAC_PI_6);
        assert_eq!(ROTOR_KEY_STEP, 0.0873);
        assert_eq!(rotator.pitch_limit(), ROTOR_PITCH_LIMIT);
        for (name, value) in [
            ("ROTOR_SENSITIVITY", ROTOR_SENSITIVITY),
            ("ROTOR_PITCH_LIMIT", ROTOR_PITCH_LIMIT),
            ("ROTOR_KEY_STEP", ROTOR_KEY_STEP),
        ] {
            assert!(
                value.is_finite() && value > 0.0,
                "{name} is finite and positive"
            );
        }
        assert!(
            rotator.pitch_limit() < FRAC_PI_4,
            "the clamp sits below the quarter turn the silhouette argument forbids"
        );
    }

    #[test]
    fn the_drag_arm_needs_no_focus() {
        // `Scroll`'s one-axis behaviour named as the precedent: a drag is
        // positional, so an unfocused rotator answers one.
        let (_nodes, rotator) = fresh_rotator();
        assert!(!rotator.focused.get());
        let mut event = drag(Offset::new(60.0, 0.0));
        assert!(rotator.on_event(&mut event, RECT));
        assert!(event.consumed());
        assert_eq!(rotator.yaw.get(), -60.0 * ROTOR_SENSITIVITY);
    }
}
