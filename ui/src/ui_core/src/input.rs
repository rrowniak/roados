//! Input.
//!
//! Owns the unified input event, the hit test that routes it, and the gestures
//! recognised from a stream of raw events.
//!
//! The pipeline an event travels:
//!
//! 1. An SDL3 event enters [`GestureRecognizer::process`]. Pointer events
//!    (finger and mouse button) are tracked per pointer until a gesture
//!    completes; keyboard, wheel and gamepad events map straight to an
//!    [`InputEvent`].
//! 2. [`hit_test`] finds the deepest node under an event's position.
//! 3. [`dispatch_event`] delivers the event to that node and bubbles it up the
//!    tree until a handler consumes it.
//!
//! [`Focus`] owns the keyboard and steering-wheel navigation between focusable
//! nodes, which is the half of input that is not routed by position.
//!
//! The recogniser is event-driven and reads the timestamps SDL puts on events
//! rather than the wall clock, so a gesture is decided by the events that
//! describe it and a test can drive it with chosen times. The one consequence
//! is that a long press fires on the first event at or after its threshold —
//! which, for a held pointer, is always the release.
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::input::{GestureRecognizer, InputEventKind, SwipeDirection};
//! use ui_core::layout::{Constraints, Layout, LayoutState, Size};
//! use ui_core::node::{self, WidgetNode};
//! use sdl3::event::Event;
//!
//! // A swipe is recognised from a finger that travels far enough.
//! let mut recognizer = GestureRecognizer::new();
//! let down = Event::FingerDown {
//!     timestamp: 0,
//!     touch_id: 1,
//!     finger_id: 1,
//!     x: 100.0,
//!     y: 100.0,
//!     dx: 0.0,
//!     dy: 0.0,
//!     pressure: 1.0,
//!     window_id: 0,
//! };
//! assert!(recognizer.process(&down).is_empty());
//!
//! let up = Event::FingerUp {
//!     timestamp: 100,
//!     touch_id: 1,
//!     finger_id: 1,
//!     x: 200.0,
//!     y: 100.0,
//!     dx: 0.0,
//!     dy: 0.0,
//!     pressure: 1.0,
//!     window_id: 0,
//! };
//! let events = recognizer.process(&up);
//! assert_eq!(events.len(), 1);
//! assert_eq!(
//!     events[0].kind(),
//!     InputEventKind::Swipe {
//!         direction: SwipeDirection::Right,
//!     }
//! );
//! ```

use std::time::Duration;

use crate::arena::{Arena, Handle};
use crate::layout::Offset;
use crate::node::WidgetNode;
use sdl3::event::Event;
use sdl3::gamepad::Axis;
use sdl3::keyboard::{Keycode, Mod};
use sdl3::mouse::MouseButton;

/// The longest a press may last and still be a tap.
///
/// These two are [`Duration`]s rather than bare counts on purpose. SDL stamps
/// every event with `SDL_GetTicksNS()` — nanoseconds, per `SDL_events.h` — and
/// these thresholds used to be `u64` milliseconds compared against that value
/// directly, which made the tap window 300 *nanoseconds* and fired a long press
/// at 500 of them. Every press was therefore a long press, and
/// `release_pointer` returns as soon as one has fired, so **no `Tap` was ever
/// produced and no widget that acts on a tap could fire at all**.
///
/// A `Duration` makes that mistake unrepresentable: `held <= TAP_MAX_DURATION`
/// does not compile if the two sides are in different units, where
/// `held_ms <= 300` compiles happily and is wrong by a factor of a million.
const TAP_MAX_DURATION: Duration = Duration::from_millis(300);
/// How far a pointer may travel from its press point and still be a tap.
const TAP_MAX_MOVEMENT: f32 = 10.0;
/// How long a press must be held to become a long press.
const LONG_PRESS_MIN_DURATION: Duration = Duration::from_millis(500);
/// How far a pointer must travel for the gesture to be a swipe.
const SWIPE_MIN_DISTANCE: f32 = 50.0;

/// The gamepad axis a steering wheel's scroll wheel reports on.
///
/// SDL does not fix which axis a given wheel's scroll occupies — it depends on
/// the controller mapping — so this is the convention the input module
/// assumes. A wheel that reports its scroll on another axis is remapped by
/// feeding [`Focus::handle_scroll`] the value from the axis it does use.
pub const STEERING_WHEEL_SCROLL_AXIS: Axis = Axis::RightX;

/// The direction a swipe travelled in.
///
/// The screen's y axis points down, so a swipe with a positive vertical
/// component is [`SwipeDirection::Down`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwipeDirection {
    /// The pointer travelled left.
    Left,
    /// The pointer travelled right.
    Right,
    /// The pointer travelled up.
    Up,
    /// The pointer travelled down.
    Down,
}

/// A key from the keyboard or a button from a gamepad.
///
/// One variant for each source because a gamepad button is not a keycode and
/// the two have to travel in the same event: focus navigation is driven by
/// both, and an event that could not say which source a press came from could
/// not tell a Tab from a steering-wheel button.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    /// A keyboard key.
    Keyboard(Keycode),
    /// A gamepad button.
    Gamepad(sdl3::gamepad::Button),
}

/// What happened in an [`InputEvent`].
///
/// The kind carries the gesture's own payload — a swipe's direction, a pinch's
/// scale, a drag's delta — while the position and the consumed flag live on
/// the event itself, because every kind has them.
///
/// **`Text` is why this type is not `Copy`.** Every other kind's payload is a
/// number or a `Copy` type, so handing one out was free. A typed run is a
/// [`String`], and a `Copy` enum cannot hold one: the alternatives were a
/// `char`, which cannot carry the multi-character run SDL delivers for an IME
/// composition commit, or a fixed buffer, which would truncate. `kind()`
/// clones instead, which costs one allocation on the rare event that carries
/// text and nothing at all on every other.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEventKind {
    /// A press released quickly without moving.
    Tap,
    /// A press held still past the long-press threshold.
    LongPress,
    /// A press that travelled far enough in one direction.
    Swipe {
        /// The dominant direction the pointer travelled.
        direction: SwipeDirection,
    },
    /// Two pointers moving apart or together.
    ///
    /// `scale` is the current distance between the pointers relative to the
    /// distance when the second one went down: `1.0` is unchanged, above it is
    /// a spread, below it a pinch closed.
    Pinch {
        /// The distance between the pointers relative to the pinch's start.
        scale: f32,
    },
    /// A pointer held down and moving.
    Drag {
        /// The movement since the previous event, in pixels.
        delta: Offset,
    },
    /// A key or gamepad button was pressed.
    KeyDown {
        /// The key or button that was pressed.
        key: Key,
        /// The modifier keys held at the moment of the press.
        keymod: Mod,
    },
    /// A key or gamepad button was released.
    KeyUp {
        /// The key or button that was released.
        key: Key,
        /// The modifier keys held at the moment of the release.
        keymod: Mod,
    },
    /// A scroll wheel or steering wheel turned.
    Scroll {
        /// The scroll amount; `x` is the wheel, `y` a vertical one.
        delta: Offset,
    },
    /// Text was typed.
    ///
    /// This is SDL's `EVENT_TEXT_INPUT`, which is the **layout-correct** text:
    /// SDL has already applied Shift, the keyboard layout and any dead-key or
    /// IME composition, and this is the string a word processor would receive.
    /// It is deliberately not derived from [`KeyDown`](Self::KeyDown) — a
    /// keycode names a physical key and says nothing about which character that
    /// key produces on the layout in use, so a widget that built characters from
    /// keycodes would re-implement, wrongly, what this event already delivers.
    ///
    /// One event carries the whole run SDL sent, which is a single character for
    /// an ordinary keypress and a whole word for a composition commit. A widget
    /// that inserts text should insert the string as given.
    ///
    /// Like the key events, this has no position: a character does not happen
    /// anywhere in particular. That is what puts it on the focused control's
    /// path rather than the positional route.
    Text {
        /// The text that was typed.
        text: String,
    },
}

/// One unified input event.
///
/// The event carries the [`InputEventKind`] that says what happened, the
/// position it happened at — `None` for events that have no position, such as
/// key presses — and the `consumed` flag that a handler sets to stop the event
/// bubbling further up the tree.
pub struct InputEvent {
    kind: InputEventKind,
    position: Option<Offset>,
    consumed: bool,
}

impl InputEvent {
    /// Creates an event of `kind` at `position`.
    #[must_use]
    pub fn new(kind: InputEventKind, position: Option<Offset>) -> Self {
        InputEvent {
            kind,
            position,
            consumed: false,
        }
    }

    /// Returns what happened.
    ///
    /// Clones, because [`InputEventKind::Text`] carries a `String` and the type
    /// is therefore not `Copy`. Every other kind's payload is a number, so the
    /// clone is a copy of a few bytes.
    #[must_use]
    pub fn kind(&self) -> InputEventKind {
        self.kind.clone()
    }

    /// Returns the position the event happened at, or `None` if it has none.
    #[must_use]
    pub fn position(&self) -> Option<Offset> {
        self.position
    }

    /// Returns `true` once a handler has consumed the event.
    #[must_use]
    pub fn consumed(&self) -> bool {
        self.consumed
    }

    /// Marks the event consumed, so [`dispatch_event`] stops bubbling it.
    pub fn consume(&mut self) {
        self.consumed = true;
    }
}

/// Returns the distance between two points.
fn distance(a: Offset, b: Offset) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// Returns `true` if `point` lies inside `rect`, edges included.
fn contains(rect: crate::layout::Rect, point: Offset) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.height
}

/// Returns the deepest node under `position`, or `None` if none covers it.
///
/// The walk is top-down from `root`: a node is a candidate when it is visible
/// and its laid-out rect contains the point, and its children are then tried
/// in reverse order, because the last child is the one a
/// [`LayoutMode::Stack`](crate::layout::LayoutMode::Stack) paints on top and
/// the one a touch should reach first. An invisible node is skipped with its
/// whole subtree.
///
/// A node the pass never placed has no rect and is skipped the same way.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::hit_test;
/// use ui_core::layout::{Constraints, Layout, LayoutState, Size};
/// use ui_core::node::{self, WidgetNode};
///
/// let mut nodes = Arena::new();
/// let root = node::create(
///     &mut nodes,
///     LayoutState::new().with_constraints(Constraints::tight(Size::new(100.0, 100.0))),
/// );
/// let child = node::create(
///     &mut nodes,
///     LayoutState::new().with_constraints(Constraints::tight(Size::new(20.0, 20.0))),
/// );
/// assert!(node::attach(&mut nodes, root, child));
/// Layout::new(&mut nodes).layout(root, Constraints::tight(Size::new(100.0, 100.0)));
///
/// let hit = hit_test(&nodes, root, ui_core::layout::Offset::new(5.0, 5.0));
/// assert_eq!(hit, Some(child), "the deepest node under the point wins");
/// ```
#[must_use]
pub fn hit_test(nodes: &Arena<WidgetNode>, root: Handle, position: Offset) -> Option<Handle> {
    hit_test_from(nodes, root, position)
}

/// Returns the deepest visible node under `position` in `handle`'s subtree.
fn hit_test_from(nodes: &Arena<WidgetNode>, handle: Handle, position: Offset) -> Option<Handle> {
    let node = nodes.get(handle)?;
    if !node.layout().visible() {
        return None;
    }
    let rect = node.layout().rect()?;
    if !contains(rect, position) {
        return None;
    }
    for &child in node.children().iter().rev() {
        if let Some(hit) = hit_test_from(nodes, child, position) {
            return Some(hit);
        }
    }
    Some(handle)
}

/// Delivers `event` to the node under its position, then bubbles it up the tree
/// until a handler consumes it.
///
/// The event is routed by [`hit_test`]: `handler` is called on the deepest
/// node under `event`'s position, and if the handler does not consume the
/// event, on that node's parent, and so on up to `root`. An event with no
/// position — a key press, for instance — is not routed by position and goes
/// to `root` directly. An event whose position is over no node at all is
/// dropped: there is no hit node, and so no parent chain to bubble along.
///
/// `handler` is called with the node's handle and the event, and consumes the
/// event by calling [`InputEvent::consume`].
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::{dispatch_event, InputEvent, InputEventKind};
/// use ui_core::layout::{Constraints, Layout, LayoutState, Offset, Size};
/// use ui_core::node::{self, WidgetNode};
///
/// let mut nodes = Arena::new();
/// let root = node::create(
///     &mut nodes,
///     LayoutState::new().with_constraints(Constraints::tight(Size::new(100.0, 100.0))),
/// );
/// let child = node::create(
///     &mut nodes,
///     LayoutState::new().with_constraints(Constraints::tight(Size::new(20.0, 20.0))),
/// );
/// assert!(node::attach(&mut nodes, root, child));
/// Layout::new(&mut nodes).layout(root, Constraints::tight(Size::new(100.0, 100.0)));
///
/// let mut delivered = Vec::new();
/// let mut event = InputEvent::new(
///     InputEventKind::Tap,
///     Some(Offset::new(5.0, 5.0)),
/// );
/// dispatch_event(&nodes, root, &mut event, &mut |handle, event| {
///     delivered.push(handle);
///     if handle == child {
///         event.consume();
///     }
/// });
///
/// assert_eq!(delivered, vec![child], "the child consumed the tap");
/// assert!(event.consumed());
/// ```
pub fn dispatch_event(
    nodes: &Arena<WidgetNode>,
    root: Handle,
    event: &mut InputEvent,
    handler: &mut dyn FnMut(Handle, &mut InputEvent),
) {
    for handle in route(nodes, root, event) {
        handler(handle, event);
        if event.consumed() {
            break;
        }
    }
}

/// Returns the nodes `event` would be offered to by [`dispatch_event`], in the
/// order it offers them: the deepest node under the event's position, then each
/// ancestor, ending at `root`.
///
/// This is the same routing as [`dispatch_event`] with the borrow released
/// before the caller acts on it, and it exists because a handler that reaches
/// the arena re-enters it. The demo's widgets are reached through property
/// callbacks — an `on_change` that marks a node dirty — and a property write
/// from inside a handler while `dispatch_event` still holds a `Ref` on the arena
/// is a `RefCell` double borrow, which panics rather than misbehaves. A caller
/// whose handlers cannot reach the arena should use `dispatch_event`; one whose
/// handlers can should route, drop the borrow, and then handle.
///
/// An event whose position is over no node yields an empty chain: there is no
/// hit node, and so nothing to bubble along. An event with no position is not
/// routed by one and yields `root`.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::{route, InputEvent, InputEventKind};
/// use ui_core::layout::{Constraints, Layout, LayoutState, Offset, Size};
/// use ui_core::node::{self, WidgetNode};
///
/// let mut nodes = Arena::new();
/// let root = node::create(
///     &mut nodes,
///     LayoutState::new().with_constraints(Constraints::tight(Size::new(100.0, 100.0))),
/// );
/// let child = node::create(
///     &mut nodes,
///     LayoutState::new().with_constraints(Constraints::tight(Size::new(20.0, 20.0))),
/// );
/// assert!(node::attach(&mut nodes, root, child));
/// Layout::new(&mut nodes).layout(root, Constraints::tight(Size::new(100.0, 100.0)));
///
/// let event = InputEvent::new(InputEventKind::Tap, Some(Offset::new(5.0, 5.0)));
/// let chain = route(&nodes, root, &event);
///
/// assert_eq!(chain, vec![child, root], "the child, then the parent");
/// ```
#[must_use]
pub fn route(nodes: &Arena<WidgetNode>, root: Handle, event: &InputEvent) -> Vec<Handle> {
    let mut chain = Vec::new();
    let mut current = match event.position() {
        Some(position) => hit_test(nodes, root, position),
        None => Some(root),
    };
    while let Some(handle) = current {
        chain.push(handle);
        current = nodes.get(handle).and_then(WidgetNode::parent);
    }
    chain
}

/// Tracks the focused node in a tree and moves it on navigation input.
///
/// Focus order is the tree's paint order — a parent, then its children in the
/// order its layout mode places them — restricted to the nodes marked
/// focusable with [`Focus::set_focusable`]. [`Focus::focus_next`] and
/// [`Focus::focus_prev`] move along that order and wrap at the ends, so Tab
/// from the last focusable node returns to the first.
///
/// The focusable set is membership only: the order is recomputed from the tree
/// each time focus moves, so a node takes its natural place wherever it sits
/// and a node removed from the tree simply drops out of the order.
pub struct Focus<'a> {
    arena: &'a Arena<WidgetNode>,
    root: Handle,
    current: Option<Handle>,
    focusable: Vec<Handle>,
}

impl<'a> Focus<'a> {
    /// Creates a focus tracker over `root` with nothing focusable and nothing
    /// focused.
    #[must_use]
    pub fn new(arena: &'a Arena<WidgetNode>, root: Handle) -> Self {
        Focus {
            arena,
            root,
            current: None,
            focusable: Vec::new(),
        }
    }

    /// Adds or removes `handle` from the set of focusable nodes.
    ///
    /// Removing the focused node leaves nothing focused.
    pub fn set_focusable(&mut self, handle: Handle, focusable: bool) {
        if focusable {
            if !self.focusable.contains(&handle) {
                self.focusable.push(handle);
            }
        } else {
            self.focusable.retain(|&held| held != handle);
            if self.current == Some(handle) {
                self.current = None;
            }
        }
    }

    /// Returns the focused node, or `None` when nothing is focused.
    #[must_use]
    pub fn current(&self) -> Option<Handle> {
        self.current
    }

    /// Moves focus to `handle`, and reports whether it did.
    ///
    /// Only a node the caller has marked focusable with
    /// [`Focus::set_focusable`] can be focused, so this cannot leave focus on
    /// something the caller never offered; it returns `false` and leaves focus
    /// where it was for anything else. That is the one way focus reaches a node
    /// without a navigation key, and it exists for a caller that owns focus
    /// itself — a gamepad and keyboard scheme, a pointer, or a tracker that does
    /// not outlive the borrow it is built from and is rebuilt around the node it
    /// last had.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::input::Focus;
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node::{self, WidgetNode};
    ///
    /// let mut nodes = Arena::new();
    /// let button = node::create(&mut nodes, LayoutState::new());
    /// let panel = node::create(&mut nodes, LayoutState::new());
    /// assert!(node::attach(&mut nodes, panel, button));
    ///
    /// let mut focus = Focus::new(&nodes, panel);
    /// assert!(!focus.focus(button), "a node nobody offered cannot be focused");
    ///
    /// focus.set_focusable(button, true);
    /// assert!(focus.focus(button));
    /// assert_eq!(focus.current(), Some(button));
    /// ```
    #[must_use]
    pub fn focus(&mut self, handle: Handle) -> bool {
        if !self.focusable.contains(&handle) {
            return false;
        }
        self.current = Some(handle);
        true
    }

    /// Moves focus to the next focusable node, wrapping from the last to the
    /// first. With nothing focused, focuses the first.
    pub fn focus_next(&mut self) {
        self.step(true);
    }

    /// Moves focus to the previous focusable node, wrapping from the first to
    /// the last. With nothing focused, focuses the last.
    pub fn focus_prev(&mut self) {
        self.step(false);
    }

    /// Handles a key event for focus navigation: Tab moves focus forward,
    /// Shift+Tab backward.
    ///
    /// Returns `true` when the key was a navigation key, so a caller can tell
    /// a key it should ignore from one focus has taken.
    #[must_use]
    pub fn handle_key(&mut self, event: &InputEvent) -> bool {
        let InputEventKind::KeyDown { key, keymod } = event.kind() else {
            return false;
        };
        let Key::Keyboard(Keycode::Tab) = key else {
            return false;
        };
        if keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
            self.focus_prev();
        } else {
            self.focus_next();
        }
        true
    }

    /// Maps a steering-wheel scroll to focus navigation.
    ///
    /// A scroll past zero in either direction moves focus that way; a scroll
    /// of exactly zero leaves focus alone.
    pub fn handle_scroll(&mut self, delta: f32) {
        if delta > 0.0 {
            self.focus_next();
        } else if delta < 0.0 {
            self.focus_prev();
        }
    }

    /// Moves focus one step through the focusable nodes in paint order.
    ///
    /// With nothing focused, a forward step takes the first node and a
    /// backward step the last, so a steering wheel turned either way from a
    /// unfocused tree still lands somewhere.
    fn step(&mut self, forward: bool) {
        let order = self.focus_order();
        let len = order.len();
        if len == 0 {
            return;
        }
        let next = match self
            .current
            .and_then(|current| order.iter().position(|&held| held == current))
        {
            Some(index) => {
                let next_index = if forward {
                    (index + 1) % len
                } else {
                    (index + len - 1) % len
                };
                order[next_index]
            }
            None => {
                if forward {
                    order[0]
                } else {
                    order[len - 1]
                }
            }
        };
        self.current = Some(next);
    }

    /// Returns the focusable nodes in the tree's paint order.
    fn focus_order(&self) -> Vec<Handle> {
        let mut order = Vec::new();
        self.collect_focusable(self.root, &mut order);
        order
    }

    /// Appends the focusable nodes in `handle`'s subtree to `order`, in paint
    /// order.
    fn collect_focusable(&self, handle: Handle, order: &mut Vec<Handle>) {
        let Some(node) = self.arena.get(handle) else {
            return;
        };
        if self.focusable.contains(&handle) {
            order.push(handle);
        }
        for &child in node.children() {
            self.collect_focusable(child, order);
        }
    }
}

/// Recognises gestures from a stream of SDL3 events.
///
/// Pointer events — finger and mouse button — are tracked per pointer: a press
/// starts a potential tap, long press, drag or swipe, and the gesture is
/// decided as the pointer moves and when it is released. Two fingers down
/// together become a pinch. Keyboard, wheel and gamepad events carry no
/// gesture and map straight to an [`InputEvent`].
///
/// The recogniser holds no timer of its own: it reads the timestamp SDL puts
/// on each event, so the long-press threshold is checked against the events
/// that arrive while a pointer is held.
pub struct GestureRecognizer {
    pointers: Vec<PointerState>,
    /// The distance between the two fingers when the pinch armed, the
    /// baseline every scale is relative to, or `None` when fewer than two
    /// fingers are down.
    pinch_distance: Option<f32>,
    /// The last position the mouse was seen at, for hover.
    mouse_position: Option<Offset>,
}

/// One pointer the recogniser is tracking.
struct PointerState {
    id: PointerId,
    /// Where the pointer went down.
    start: Offset,
    /// Where the pointer was last seen.
    last: Offset,
    /// The timestamp of the down event.
    down_at: u64,
    /// Whether the pointer has travelled past the tap threshold.
    moved: bool,
    /// Whether the long press has already fired for this press.
    long_press_fired: bool,
}

/// Identifies a pointer: a finger, or one mouse button.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PointerId {
    /// A touch finger, by its SDL finger id.
    Finger(u64),
    /// A mouse button.
    Mouse(MouseButton),
}

impl GestureRecognizer {
    /// Creates a recogniser tracking no pointers.
    #[must_use]
    pub fn new() -> Self {
        GestureRecognizer {
            pointers: Vec::new(),
            pinch_distance: None,
            mouse_position: None,
        }
    }

    /// Returns the last position the mouse was seen at, for hover.
    #[must_use]
    pub fn mouse_position(&self) -> Option<Offset> {
        self.mouse_position
    }

    /// Feeds one SDL3 event to the recogniser and returns the input events it
    /// produced.
    ///
    /// Most events produce none: a gesture is decided by the events that
    /// complete it. A pointer release produces the tap or swipe it finalises, a
    /// held pointer past the threshold produces its long press, and a wheel or
    /// key event produces the event it maps to.
    #[must_use]
    pub fn process(&mut self, event: &Event) -> Vec<InputEvent> {
        let mut out = Vec::new();
        let now = handled_timestamp(event);
        self.check_long_press(now, &mut out);
        match event {
            Event::FingerDown {
                finger_id, x, y, ..
            } => self.press_pointer(PointerId::Finger(*finger_id), Offset::new(*x, *y), now),
            Event::FingerMotion {
                finger_id, x, y, ..
            } => self.update_pointer(PointerId::Finger(*finger_id), Offset::new(*x, *y), &mut out),
            Event::FingerUp {
                finger_id, x, y, ..
            } => self.release_pointer(
                PointerId::Finger(*finger_id),
                Offset::new(*x, *y),
                now,
                &mut out,
            ),
            // A canceled touch finalises nothing: the pointer is dropped
            // without evaluating a gesture, exactly as a release that
            // produced no travel and no duration would.
            Event::FingerCanceled { finger_id, .. } => {
                self.remove_pointer(PointerId::Finger(*finger_id));
            }
            Event::MouseButtonDown {
                mouse_btn, x, y, ..
            } => self.press_pointer(PointerId::Mouse(*mouse_btn), Offset::new(*x, *y), now),
            Event::MouseButtonUp {
                mouse_btn, x, y, ..
            } => self.release_pointer(
                PointerId::Mouse(*mouse_btn),
                Offset::new(*x, *y),
                now,
                &mut out,
            ),
            Event::MouseMotion {
                x, y, mousestate, ..
            } => {
                self.mouse_position = Some(Offset::new(*x, *y));
                let pressed: Vec<PointerId> = self
                    .pointers
                    .iter()
                    .filter(|pointer| {
                        matches!(pointer.id, PointerId::Mouse(button) if mousestate.is_mouse_button_pressed(button))
                    })
                    .map(|pointer| pointer.id)
                    .collect();
                for id in pressed {
                    self.update_pointer(id, Offset::new(*x, *y), &mut out);
                }
            }
            Event::MouseWheel {
                x,
                y,
                mouse_x,
                mouse_y,
                ..
            } => out.push(InputEvent::new(
                InputEventKind::Scroll {
                    delta: Offset::new(*x, *y),
                },
                Some(Offset::new(*mouse_x, *mouse_y)),
            )),
            Event::KeyDown {
                keycode: Some(keycode),
                keymod,
                ..
            } => out.push(InputEvent::new(
                InputEventKind::KeyDown {
                    key: Key::Keyboard(*keycode),
                    keymod: *keymod,
                },
                None,
            )),
            Event::KeyUp {
                keycode: Some(keycode),
                keymod,
                ..
            } => out.push(InputEvent::new(
                InputEventKind::KeyUp {
                    key: Key::Keyboard(*keycode),
                    keymod: *keymod,
                },
                None,
            )),
            Event::TextInput { text, .. } => {
                // An empty run is not text. SDL can deliver one — clearing an IME
                // composition reports the text that is left, which is nothing —
                // and a `Text` carrying "" would be an event every consumer has
                // to learn to ignore.
                if !text.is_empty() {
                    out.push(InputEvent::new(
                        InputEventKind::Text { text: text.clone() },
                        None,
                    ));
                }
            }
            Event::GamepadButtonDown { button, .. } => out.push(InputEvent::new(
                InputEventKind::KeyDown {
                    key: Key::Gamepad(*button),
                    keymod: Mod::empty(),
                },
                None,
            )),
            Event::GamepadButtonUp { button, .. } => out.push(InputEvent::new(
                InputEventKind::KeyUp {
                    key: Key::Gamepad(*button),
                    keymod: Mod::empty(),
                },
                None,
            )),
            Event::GamepadAxisMotion { axis, value, .. } if *axis == STEERING_WHEEL_SCROLL_AXIS => {
                out.push(InputEvent::new(
                    InputEventKind::Scroll {
                        delta: Offset::new(f32::from(*value), 0.0),
                    },
                    None,
                ));
            }
            _ => {}
        }
        out
    }

    /// Starts tracking a pointer that went down at `position` at `now`.
    fn press_pointer(&mut self, id: PointerId, position: Offset, now: u64) {
        if self.pointers.iter().any(|pointer| pointer.id == id) {
            return;
        }
        self.pointers.push(PointerState {
            id,
            start: position,
            last: position,
            down_at: now,
            moved: false,
            long_press_fired: false,
        });
        self.update_pinch_state();
    }

    /// Records a pointer's new position and emits the gesture it implies.
    fn update_pointer(&mut self, id: PointerId, position: Offset, out: &mut Vec<InputEvent>) {
        let Some(index) = self.pointers.iter().position(|pointer| pointer.id == id) else {
            return;
        };
        let previous = self.pointers[index].last;
        self.pointers[index].last = position;
        if distance(self.pointers[index].start, position) > TAP_MAX_MOVEMENT {
            self.pointers[index].moved = true;
        }
        // A pinch whose fingers coincided when the second went down has no
        // baseline to scale against; the first motion that separates them
        // arms it, and that separation becomes the baseline.
        if self.pinch_distance == Some(0.0) && self.pointers.len() == 2 {
            self.pinch_distance = Some(distance(self.pointers[0].last, self.pointers[1].last));
        }
        match self.pinch_distance {
            Some(initial) if initial > 0.0 => {
                let current = distance(self.pointers[0].last, self.pointers[1].last);
                out.push(InputEvent::new(
                    InputEventKind::Pinch {
                        scale: current / initial,
                    },
                    Some(position),
                ));
            }
            _ if self.pointers[index].moved => out.push(InputEvent::new(
                InputEventKind::Drag {
                    delta: Offset::new(position.x - previous.x, position.y - previous.y),
                },
                Some(position),
            )),
            _ => {}
        }
    }

    /// Finalises a pointer that was released at `position` at `now`.
    fn release_pointer(
        &mut self,
        id: PointerId,
        position: Offset,
        now: u64,
        out: &mut Vec<InputEvent>,
    ) {
        let Some((pointer, was_pinch)) = self.remove_pointer(id) else {
            return;
        };
        if was_pinch || pointer.long_press_fired {
            return;
        }
        let held = held_for(pointer.down_at, now);
        let travel = distance(pointer.start, position);
        if travel >= SWIPE_MIN_DISTANCE {
            out.push(InputEvent::new(
                InputEventKind::Swipe {
                    direction: swipe_direction(pointer.start, position),
                },
                Some(position),
            ));
        } else if travel <= TAP_MAX_MOVEMENT && held <= TAP_MAX_DURATION {
            out.push(InputEvent::new(InputEventKind::Tap, Some(position)));
        }
    }

    /// Fires the long press for every held pointer that has been still long
    /// enough at `now`.
    fn check_long_press(&mut self, now: u64, out: &mut Vec<InputEvent>) {
        // A long press is a one-finger gesture: with two pointers down the
        // hold is a pinch, and neither finger is a press.
        if self.pointers.len() > 1 {
            return;
        }
        for pointer in &mut self.pointers {
            if !pointer.long_press_fired
                && !pointer.moved
                && held_for(pointer.down_at, now) >= LONG_PRESS_MIN_DURATION
            {
                pointer.long_press_fired = true;
                out.push(InputEvent::new(
                    InputEventKind::LongPress,
                    Some(pointer.last),
                ));
            }
        }
    }

    /// Stops tracking the pointer `id` and returns it, with whether its removal
    /// ended a two-pointer pinch, or `None` if it was not being tracked.
    fn remove_pointer(&mut self, id: PointerId) -> Option<(PointerState, bool)> {
        let index = self.pointers.iter().position(|pointer| pointer.id == id)?;
        let was_pinch = self.pointers.len() == 2;
        let pointer = self.pointers.remove(index);
        self.update_pinch_state();
        Some((pointer, was_pinch))
    }

    /// Recomputes the pinch distance from the fingers currently down.
    ///
    /// A pinch needs exactly two fingers; a mouse button is not a finger, so a
    /// finger and a button down together drag rather than pinch.
    fn update_pinch_state(&mut self) {
        self.pinch_distance = match self.pointers.as_slice() {
            [a, b] if a.id.is_finger() && b.id.is_finger() => Some(distance(a.last, b.last)),
            _ => None,
        };
    }
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerId {
    /// Returns `true` if this id is a touch finger.
    fn is_finger(&self) -> bool {
        matches!(self, PointerId::Finger(_))
    }
}

/// Returns the dominant direction from `start` to `end`.
///
/// The axis with the larger travel wins; a tie goes to the horizontal, and a
/// zero travel on the losing axis still picks the winning side.
fn swipe_direction(start: Offset, end: Offset) -> SwipeDirection {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if dx.abs() >= dy.abs() {
        if dx > 0.0 {
            SwipeDirection::Right
        } else {
            SwipeDirection::Left
        }
    } else if dy > 0.0 {
        SwipeDirection::Down
    } else {
        SwipeDirection::Up
    }
}

/// Returns the timestamp of an event the recogniser handles, or `0` for one
/// it does not.
///
/// Every event variant carries its own `timestamp` field and there is no
/// common accessor, so this is the one place they are read together. Only the
/// handled events advance the long-press clock: an unhandled event between two
/// pointer events is rare, and the long press fires by the release at the
/// latest. A `0` timestamp is therefore a safe stand-in — it is older than
/// any real press, so it fires nothing.
/// Returns how long a pointer pressed at `down_at` has been held at `now`.
///
/// `down_at` and `now` are SDL's own nanosecond stamps, so the difference is
/// nanoseconds and this converts it to a [`Duration`] once, at the boundary.
/// `saturating_sub` is kept so a clock that steps backwards — which a
/// synthesised event can produce — reads as no time held rather than as a
/// wrapped enormous one.
fn held_for(down_at: u64, now: u64) -> Duration {
    Duration::from_nanos(now.saturating_sub(down_at))
}

fn handled_timestamp(event: &Event) -> u64 {
    match event {
        Event::FingerDown { timestamp, .. }
        | Event::FingerMotion { timestamp, .. }
        | Event::FingerUp { timestamp, .. }
        | Event::MouseButtonDown { timestamp, .. }
        | Event::MouseButtonUp { timestamp, .. }
        | Event::MouseMotion { timestamp, .. }
        | Event::MouseWheel { timestamp, .. }
        | Event::KeyDown { timestamp, .. }
        | Event::KeyUp { timestamp, .. }
        | Event::GamepadButtonDown { timestamp, .. }
        | Event::GamepadButtonUp { timestamp, .. }
        | Event::GamepadAxisMotion { timestamp, .. } => *timestamp,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Constraints, Layout, LayoutMode, LayoutState, Size};
    use crate::node;
    use sdl3::gamepad::Button;

    /// Creates a fixed-size leaf.
    fn leaf(nodes: &mut Arena<WidgetNode>, size: Size) -> Handle {
        node::create(
            nodes,
            LayoutState::new().with_constraints(Constraints::tight(size)),
        )
    }

    /// Creates a stack holding `children`, and returns its handle.
    fn stack(nodes: &mut Arena<WidgetNode>, children: &[Handle]) -> Handle {
        let handle = node::create(nodes, LayoutState::new().with_mode(LayoutMode::Stack));
        for &child in children {
            assert!(node::attach(nodes, handle, child));
        }
        handle
    }

    /// Lays `root` out in a tight box.
    fn laid_out(nodes: &mut Arena<WidgetNode>, root: Handle, size: Size) {
        Layout::new(nodes).layout(root, Constraints::tight(size));
    }

    /// Returns a finger-down event for `finger_id` at `(x, y)`.
    fn finger_down(finger_id: u64, x: f32, y: f32) -> Event {
        Event::FingerDown {
            timestamp: 0,
            touch_id: finger_id,
            finger_id,
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        }
    }

    /// A nanosecond timestamp `duration` after the zero the event helpers stamp
    /// their events with.
    ///
    /// SDL timestamps events with `SDL_GetTicksNS()`, so a test that wants a
    /// press to last 200 milliseconds has to say 200 *million* nanoseconds. The
    /// first version of this suite said `200`, and every test in it passed
    /// against a recogniser that read milliseconds where SDL writes nanoseconds:
    /// a 200-nanosecond press is under a 300-nanosecond window, and a
    /// 300-nanosecond one is over it, so the bug was invisible here and fatal in
    /// the product.
    fn after(duration: Duration) -> u64 {
        u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
    }

    /// Returns a finger-motion event for `finger_id` at `(x, y)`.
    fn finger_motion(finger_id: u64, x: f32, y: f32) -> Event {
        Event::FingerMotion {
            timestamp: 0,
            touch_id: finger_id,
            finger_id,
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        }
    }

    /// Returns a finger-up event for `finger_id` at `(x, y)`.
    fn finger_up(finger_id: u64, x: f32, y: f32) -> Event {
        Event::FingerUp {
            timestamp: 0,
            touch_id: finger_id,
            finger_id,
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        }
    }

    /// Returns a finger-canceled event for `finger_id` at `(x, y)`.
    fn finger_canceled(finger_id: u64, x: f32, y: f32) -> Event {
        Event::FingerCanceled {
            timestamp: 0,
            touch_id: finger_id,
            finger_id,
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        }
    }

    /// Returns a key-down event for `keycode`.
    fn key_down(keycode: Keycode) -> Event {
        Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(keycode),
            scancode: None,
            keymod: Mod::empty(),
            repeat: false,
            which: 0,
            raw: 0,
        }
    }

    /// Returns a text-input event carrying `text`.
    fn text_input(text: &str) -> Event {
        Event::TextInput {
            timestamp: 0,
            window_id: 0,
            text: text.to_string(),
        }
    }

    /// Returns a gamepad button-down event for `button`.
    fn gamepad_down(button: Button) -> Event {
        Event::GamepadButtonDown {
            timestamp: 0,
            which: sdl3::joystick::JoystickId::from(0),
            button,
        }
    }

    /// Returns a gamepad axis-motion event for `axis` at `value`.
    fn gamepad_axis(axis: Axis, value: i16) -> Event {
        Event::GamepadAxisMotion {
            timestamp: 0,
            which: sdl3::joystick::JoystickId::from(0),
            axis,
            value,
        }
    }

    /// Returns a mouse-button-down event for `button` at `(x, y)`.
    fn mouse_down(button: MouseButton, x: f32, y: f32) -> Event {
        Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: button,
            clicks: 1,
            x,
            y,
        }
    }

    /// Returns a mouse-button-up event for `button` at `(x, y)`.
    fn mouse_up(button: MouseButton, x: f32, y: f32) -> Event {
        Event::MouseButtonUp {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: button,
            clicks: 1,
            x,
            y,
        }
    }

    /// Returns a mouse-motion event at `(x, y)` with `button` held.
    fn mouse_motion(button: MouseButton, x: f32, y: f32) -> Event {
        Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: mouse_state(button),
            x,
            y,
            xrel: 0.0,
            yrel: 0.0,
        }
    }

    /// Returns a mouse state with only `button` held.
    fn mouse_state(button: MouseButton) -> sdl3::mouse::MouseState {
        // SDL numbers the buttons from 1 and masks them with `1 << (n - 1)`,
        // which is the layout `is_mouse_button_pressed` tests. The cast reads
        // the enum's discriminant, which is not a numeric conversion.
        let button = u32::from(button as u8);
        sdl3::mouse::MouseState::from_sdl_state(1 << button.saturating_sub(1))
    }

    /// Returns a mouse-wheel event scrolling `(x, y)` at `(mouse_x, mouse_y)`.
    fn mouse_wheel(x: f32, y: f32, mouse_x: f32, mouse_y: f32) -> Event {
        Event::MouseWheel {
            timestamp: 0,
            window_id: 0,
            which: 0,
            x,
            y,
            direction: sdl3::mouse::MouseWheelDirection::Normal,
            mouse_x,
            mouse_y,
            integer_x: 0,
            integer_y: 0,
        }
    }

    /// Builds a root with three stacked children and lays it out, returning the
    /// handles and the size the tree was laid out in.
    fn three_stacked() -> (Arena<WidgetNode>, Handle, [Handle; 3], Size) {
        let mut nodes = Arena::new();
        let first = leaf(&mut nodes, Size::new(20.0, 20.0));
        let second = leaf(&mut nodes, Size::new(20.0, 20.0));
        let third = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[first, second, third]);
        let size = Size::new(100.0, 100.0);
        laid_out(&mut nodes, root, size);
        (nodes, root, [first, second, third], size)
    }

    #[test]
    fn a_point_in_a_child_hits_the_child() {
        // A row places the children side by side, so a point in one is not in
        // the other and the deepest node under it is that child alone.
        let mut nodes = Arena::new();
        let first = leaf(&mut nodes, Size::new(20.0, 20.0));
        let second = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = node::create(&mut nodes, LayoutState::new().with_mode(LayoutMode::row()));
        assert!(node::attach(&mut nodes, root, first));
        assert!(node::attach(&mut nodes, root, second));
        laid_out(&mut nodes, root, Size::new(100.0, 100.0));

        let in_first = Offset::new(5.0, 5.0);
        assert_eq!(hit_test(&nodes, root, in_first), Some(first));
        let in_second = Offset::new(25.0, 5.0);
        assert_eq!(hit_test(&nodes, root, in_second), Some(second));
    }

    #[test]
    fn a_point_over_overlapping_children_hits_the_last() {
        // A stack paints the last child over the others, so a touch over the
        // overlap reaches the last one.
        let (nodes, root, [first, second, third], _) = three_stacked();
        let point = Offset::new(5.0, 5.0);

        assert_eq!(hit_test(&nodes, root, point), Some(third));
        assert_ne!(
            hit_test(&nodes, root, point),
            Some(first),
            "the first child is underneath, not on top"
        );
        assert_ne!(hit_test(&nodes, root, point), Some(second));
    }

    #[test]
    fn a_point_outside_every_node_hits_nothing() {
        let (nodes, root, .., size) = three_stacked();
        let point = Offset::new(size.width + 10.0, size.height + 10.0);

        assert_eq!(hit_test(&nodes, root, point), None);
    }

    #[test]
    fn a_point_in_the_root_but_no_child_hits_the_root() {
        let (nodes, root, ..) = three_stacked();
        // The children cover the top-left 20x20; a point outside that but
        // inside the root hits the root itself.
        let point = Offset::new(50.0, 50.0);

        assert_eq!(hit_test(&nodes, root, point), Some(root));
    }

    #[test]
    fn an_invisible_node_and_its_subtree_are_skipped() {
        let mut nodes = Arena::new();
        let lower = leaf(&mut nodes, Size::new(20.0, 20.0));
        let upper = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[lower, upper]);
        laid_out(&mut nodes, root, Size::new(100.0, 100.0));
        let point = Offset::new(5.0, 5.0);

        // Both visible: the point hits the upper child, the one on top.
        assert_eq!(hit_test(&nodes, root, point), Some(upper));

        // Hide the upper child: the point falls through to the lower one.
        nodes
            .get_mut(upper)
            .unwrap()
            .layout_mut()
            .set_visible(false);
        assert_eq!(hit_test(&nodes, root, point), Some(lower));

        // Hide the lower child too: the point hits the root.
        nodes
            .get_mut(lower)
            .unwrap()
            .layout_mut()
            .set_visible(false);
        assert_eq!(hit_test(&nodes, root, point), Some(root));

        // Hide the root: the whole tree is skipped.
        nodes.get_mut(root).unwrap().layout_mut().set_visible(false);
        assert_eq!(hit_test(&nodes, root, point), None);
    }

    #[test]
    fn a_node_with_no_rect_is_skipped() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[child]);
        // No layout pass: the child has no rect.
        assert_eq!(hit_test(&nodes, root, Offset::new(5.0, 5.0)), None);
    }

    #[test]
    fn a_tap_is_a_quick_press_that_does_not_move() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let up = Event::FingerUp {
            timestamp: after(Duration::from_millis(200)),
            touch_id: 1,
            finger_id: 1,
            x: 105.0,
            y: 100.0,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        };
        let events = recognizer.process(&up);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), InputEventKind::Tap);
        assert_eq!(events[0].position(), Some(Offset::new(105.0, 100.0)));
    }

    #[test]
    fn a_press_that_too_long_is_not_a_tap() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let up = Event::FingerUp {
            timestamp: after(TAP_MAX_DURATION + Duration::from_nanos(1)),
            touch_id: 1,
            finger_id: 1,
            x: 100.0,
            y: 100.0,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        };
        let events = recognizer.process(&up);

        assert!(
            events.is_empty(),
            "a press past the tap window is not a tap"
        );
    }

    #[test]
    fn a_press_of_a_realistic_length_is_a_tap_and_a_held_one_is_a_long_press() {
        // The test that pins the unit SDL timestamps events in. A press held for
        // a tenth of a second is a tap and nothing else, and one held for six
        // tenths is a long press — read as milliseconds against a nanosecond
        // stamp, *both* were over the 300 threshold and both fired a long press,
        // so the recogniser produced no `Tap` at all and no widget acting on one
        // could ever fire. A test that used 200 for "quick" could not see that,
        // because 200 nanoseconds is under a 300-nanosecond window.
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let quick = Event::FingerUp {
            timestamp: after(Duration::from_millis(100)),
            touch_id: 1,
            finger_id: 1,
            x: 100.0,
            y: 100.0,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        };
        let events = recognizer.process(&quick);
        assert_eq!(events.len(), 1, "a tenth of a second is an ordinary click");
        assert_eq!(events[0].kind(), InputEventKind::Tap);

        // And the same press held past the long-press threshold, which is the
        // case the tap window has to stay clear of.
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());
        let mut held = Event::FingerMotion {
            timestamp: after(Duration::from_millis(100)),
            touch_id: 1,
            finger_id: 1,
            x: 100.0,
            y: 100.0,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        };
        assert!(recognizer.process(&held).is_empty());
        still_timestamp(&mut held, after(Duration::from_millis(600)));
        let events = recognizer.process(&held);

        assert_eq!(
            events.len(),
            1,
            "six tenths of a second has passed the long-press threshold"
        );
        assert_eq!(events[0].kind(), InputEventKind::LongPress);
    }

    #[test]
    fn a_press_that_moves_too_far_is_not_a_tap() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let up = Event::FingerUp {
            timestamp: 100,
            touch_id: 1,
            finger_id: 1,
            x: 100.0 + TAP_MAX_MOVEMENT + 1.0,
            y: 100.0,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
            window_id: 0,
        };
        let events = recognizer.process(&up);

        assert!(
            events.is_empty(),
            "a press that travels past the tap threshold is not a tap"
        );
    }

    #[test]
    fn a_long_press_fires_once_the_pointer_has_been_still() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        // A motion at the same position carries a later timestamp, which is
        // what the recogniser reads the threshold against.
        let mut still = finger_motion(1, 100.0, 100.0);
        still_timestamp(
            &mut still,
            after(LONG_PRESS_MIN_DURATION + Duration::from_millis(100)),
        );
        let events = recognizer.process(&still);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), InputEventKind::LongPress);
        assert_eq!(events[0].position(), Some(Offset::new(100.0, 100.0)));

        // The release after a long press finalises nothing.
        let mut up = finger_up(1, 100.0, 100.0);
        still_timestamp(
            &mut up,
            after(LONG_PRESS_MIN_DURATION + Duration::from_millis(200)),
        );
        let events = recognizer.process(&up);
        assert!(
            events.is_empty(),
            "the long press already fired; the release adds nothing"
        );
    }

    #[test]
    fn a_long_press_does_not_fire_once_the_pointer_has_moved() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        // Move well before the long-press threshold: the pointer is a drag now.
        let mut moving = finger_motion(1, 150.0, 100.0);
        still_timestamp(&mut moving, 100);
        let events = recognizer.process(&moving);
        assert_eq!(events.len(), 1, "the motion is a drag");
        assert_eq!(
            events[0].kind(),
            InputEventKind::Drag {
                delta: Offset::new(50.0, 0.0)
            }
        );

        // Well past the threshold, still no long press — the pointer moved.
        let mut later = finger_motion(1, 160.0, 100.0);
        still_timestamp(
            &mut later,
            after(LONG_PRESS_MIN_DURATION + Duration::from_millis(100)),
        );
        let events = recognizer.process(&later);
        assert!(
            events
                .iter()
                .all(|event| event.kind() != InputEventKind::LongPress),
            "a pointer that has moved is a drag, not a long press"
        );
    }

    #[test]
    fn a_swipe_is_recognised_in_the_direction_it_travelled() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let mut up = finger_up(1, 100.0 + SWIPE_MIN_DISTANCE + 10.0, 100.0);
        still_timestamp(&mut up, 100);
        let events = recognizer.process(&up);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].kind(),
            InputEventKind::Swipe {
                direction: SwipeDirection::Right,
            }
        );
    }

    #[test]
    fn a_swipe_down_beats_a_smaller_horizontal_travel() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let mut up = finger_up(1, 110.0, 200.0);
        still_timestamp(&mut up, 100);
        let events = recognizer.process(&up);

        assert_eq!(
            events[0].kind(),
            InputEventKind::Swipe {
                direction: SwipeDirection::Down,
            }
        );
    }

    #[test]
    fn a_drag_reports_the_movement_since_the_previous_event() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        // The first motion crosses the tap threshold and starts the drag.
        let mut first = finger_motion(1, 120.0, 100.0);
        still_timestamp(&mut first, 10);
        let events = recognizer.process(&first);
        assert_eq!(events.len(), 1, "the crossing motion starts the drag");
        assert_eq!(
            events[0].kind(),
            InputEventKind::Drag {
                delta: Offset::new(20.0, 0.0),
            }
        );

        let mut second = finger_motion(1, 130.0, 110.0);
        still_timestamp(&mut second, 20);
        let events = recognizer.process(&second);
        assert_eq!(
            events[0].kind(),
            InputEventKind::Drag {
                delta: Offset::new(10.0, 10.0),
            },
            "each drag reports the movement since the previous event"
        );
    }

    #[test]
    fn a_small_movement_before_the_drag_threshold_emits_nothing() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let mut small = finger_motion(1, 105.0, 100.0);
        still_timestamp(&mut small, 10);
        let events = recognizer.process(&small);

        assert!(
            events.is_empty(),
            "a motion within the tap threshold is still a potential tap"
        );
    }

    #[test]
    fn a_pinch_reports_the_scale_between_two_fingers() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());
        assert!(recognizer.process(&finger_down(2, 200.0, 100.0)).is_empty());

        // The fingers start 100 pixels apart; moving the first to 80 makes it
        // 120, so the scale is 120 / 100.
        let mut spread = finger_motion(1, 80.0, 100.0);
        still_timestamp(&mut spread, 10);
        let events = recognizer.process(&spread);
        assert_eq!(events.len(), 1);
        match events[0].kind() {
            InputEventKind::Pinch { scale } => {
                assert!((scale - 1.2).abs() < 1e-6, "the scale is 120 / 100");
            }
            other => panic!("expected a pinch, got {other:?}"),
        }

        // Moving the second to 140 closes the gap to 60, so the scale is
        // 60 / 100.
        let mut close = finger_motion(2, 140.0, 100.0);
        still_timestamp(&mut close, 20);
        let events = recognizer.process(&close);
        match events[0].kind() {
            InputEventKind::Pinch { scale } => {
                assert!((scale - 0.6).abs() < 1e-6, "the scale is 60 / 100");
            }
            other => panic!("expected a pinch, got {other:?}"),
        }
    }

    #[test]
    fn a_stationary_two_finger_hold_fires_no_long_press() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());
        assert!(recognizer.process(&finger_down(2, 200.0, 100.0)).is_empty());

        // A still motion well past the threshold: with two fingers down the
        // hold is a pinch, not a long press.
        let mut still = finger_motion(1, 100.0, 100.0);
        still_timestamp(
            &mut still,
            after(LONG_PRESS_MIN_DURATION + Duration::from_millis(100)),
        );
        let events = recognizer.process(&still);
        assert!(
            events
                .iter()
                .all(|event| event.kind() != InputEventKind::LongPress),
            "two fingers held still are not a long press"
        );
    }

    #[test]
    fn a_canceled_finger_leaves_no_stuck_pointer() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());

        let events = recognizer.process(&finger_canceled(1, 100.0, 100.0));
        assert!(events.is_empty(), "a canceled touch produces no gesture");

        // Well past the long-press threshold, a still motion of the same
        // finger id: a pointer the cancel left behind would fire a spurious
        // long press here.
        let mut still = finger_motion(1, 100.0, 100.0);
        still_timestamp(
            &mut still,
            after(LONG_PRESS_MIN_DURATION + Duration::from_millis(100)),
        );
        let events = recognizer.process(&still);
        assert!(
            events
                .iter()
                .all(|event| event.kind() != InputEventKind::LongPress),
            "a canceled finger must not fire a long press"
        );
    }

    #[test]
    fn a_pinch_that_starts_with_the_fingers_together_arms_when_they_separate() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());
        assert!(recognizer.process(&finger_down(2, 100.0, 100.0)).is_empty());

        // The fingers separate: the pinch arms with that separation as its
        // baseline, so the scale is 1.0.
        let mut spread = finger_motion(1, 150.0, 100.0);
        still_timestamp(&mut spread, 10);
        let events = recognizer.process(&spread);
        assert_eq!(
            events.len(),
            1,
            "the separating motion is a pinch, not a drag"
        );
        match events[0].kind() {
            InputEventKind::Pinch { scale } => {
                assert!((scale - 1.0).abs() < 1e-6, "the scale starts at 1.0");
            }
            other => panic!("expected a pinch, got {other:?}"),
        }

        // Moving further apart scales from that baseline: 150 / 50 = 3.0.
        let mut more = finger_motion(1, 250.0, 100.0);
        still_timestamp(&mut more, 20);
        let events = recognizer.process(&more);
        assert_eq!(events.len(), 1);
        match events[0].kind() {
            InputEventKind::Pinch { scale } => {
                assert!((scale - 3.0).abs() < 1e-6, "the scale is 150 / 50");
            }
            other => panic!("expected a pinch, got {other:?}"),
        }
    }

    #[test]
    fn a_pinch_ends_when_one_finger_lifts() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer.process(&finger_down(1, 100.0, 100.0)).is_empty());
        assert!(recognizer.process(&finger_down(2, 200.0, 100.0)).is_empty());

        let mut lift = finger_up(1, 120.0, 100.0);
        still_timestamp(&mut lift, 10);
        let events = recognizer.process(&lift);
        assert!(
            events.is_empty(),
            "lifting one finger of a pinch ends it, not a swipe"
        );

        // The remaining finger lifts to a tap of its own.
        let mut last = finger_up(2, 200.0, 100.0);
        still_timestamp(&mut last, 20);
        let events = recognizer.process(&last);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), InputEventKind::Tap);
    }

    #[test]
    fn a_mouse_button_press_is_tracked_like_a_finger() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer
            .process(&mouse_down(MouseButton::Left, 100.0, 100.0))
            .is_empty());

        let mut up = mouse_up(MouseButton::Left, 102.0, 100.0);
        still_timestamp(&mut up, 100);
        let events = recognizer.process(&up);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), InputEventKind::Tap);
    }

    #[test]
    fn a_mouse_drag_is_reported_while_the_button_is_held() {
        let mut recognizer = GestureRecognizer::new();
        assert!(recognizer
            .process(&mouse_down(MouseButton::Left, 100.0, 100.0))
            .is_empty());

        let mut drag = mouse_motion(MouseButton::Left, 130.0, 100.0);
        still_timestamp(&mut drag, 10);
        let events = recognizer.process(&drag);
        assert_eq!(events.len(), 1, "the button is held, so the motion drags");
        assert_eq!(
            events[0].kind(),
            InputEventKind::Drag {
                delta: Offset::new(30.0, 0.0),
            }
        );

        // 30 pixels is past the tap threshold but short of a swipe, so the
        // release finalises nothing.
        let mut up = mouse_up(MouseButton::Left, 130.0, 100.0);
        still_timestamp(&mut up, 20);
        let events = recognizer.process(&up);
        assert!(events.is_empty());
    }

    #[test]
    fn a_mouse_motion_with_no_button_held_tracks_hover_only() {
        let mut recognizer = GestureRecognizer::new();
        let motion = Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: sdl3::mouse::MouseState::from_sdl_state(0),
            x: 50.0,
            y: 60.0,
            xrel: 0.0,
            yrel: 0.0,
        };
        let events = recognizer.process(&motion);

        assert!(events.is_empty(), "hover produces no event");
        assert_eq!(
            recognizer.mouse_position(),
            Some(Offset::new(50.0, 60.0)),
            "but the position is remembered"
        );
    }

    #[test]
    fn a_mouse_wheel_scrolls() {
        let mut recognizer = GestureRecognizer::new();
        let events = recognizer.process(&mouse_wheel(0.0, 1.0, 50.0, 60.0));

        assert_eq!(events.len(), 1);
        match events[0].kind() {
            InputEventKind::Scroll { delta } => {
                assert_eq!(delta, Offset::new(0.0, 1.0));
            }
            other => panic!("expected a scroll, got {other:?}"),
        }
        assert_eq!(events[0].position(), Some(Offset::new(50.0, 60.0)));
    }

    #[test]
    fn a_key_press_becomes_a_key_event() {
        let mut recognizer = GestureRecognizer::new();
        let events = recognizer.process(&key_down(Keycode::Tab));

        assert_eq!(events.len(), 1);
        match events[0].kind() {
            InputEventKind::KeyDown { key, keymod } => {
                assert_eq!(key, Key::Keyboard(Keycode::Tab));
                assert_eq!(keymod, Mod::empty());
            }
            other => panic!("expected a key down, got {other:?}"),
        }
        assert_eq!(events[0].position(), None);
    }

    #[test]
    fn a_gamepad_button_becomes_a_key_event() {
        let mut recognizer = GestureRecognizer::new();
        let events = recognizer.process(&gamepad_down(Button::South));

        assert_eq!(events.len(), 1);
        match events[0].kind() {
            InputEventKind::KeyDown { key, .. } => {
                assert_eq!(key, Key::Gamepad(Button::South));
            }
            other => panic!("expected a key down, got {other:?}"),
        }
    }

    #[test]
    fn the_steering_wheel_axis_scrolls() {
        let mut recognizer = GestureRecognizer::new();
        let events = recognizer.process(&gamepad_axis(STEERING_WHEEL_SCROLL_AXIS, 100));

        assert_eq!(events.len(), 1);
        match events[0].kind() {
            InputEventKind::Scroll { delta } => {
                assert_eq!(delta, Offset::new(100.0, 0.0));
            }
            other => panic!("expected a scroll, got {other:?}"),
        }
    }

    #[test]
    fn another_gamepad_axis_produces_nothing() {
        let mut recognizer = GestureRecognizer::new();
        let events = recognizer.process(&gamepad_axis(Axis::LeftX, 100));

        assert!(events.is_empty());
    }

    #[test]
    fn an_unhandled_event_produces_nothing() {
        let mut recognizer = GestureRecognizer::new();
        let events = recognizer.process(&Event::Quit { timestamp: 0 });

        assert!(events.is_empty());
    }

    #[test]
    fn tab_moves_focus_to_the_next_focusable_node() {
        let (nodes, root, [first, second, third], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        for handle in [first, second, third] {
            focus.set_focusable(handle, true);
        }

        let tab = InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(Keycode::Tab),
                keymod: Mod::empty(),
            },
            None,
        );
        assert!(focus.handle_key(&tab));
        assert_eq!(focus.current(), Some(first));

        assert!(focus.handle_key(&tab));
        assert_eq!(focus.current(), Some(second));

        assert!(focus.handle_key(&tab));
        assert_eq!(focus.current(), Some(third));
    }

    #[test]
    fn tab_wraps_from_the_last_node_back_to_the_first() {
        let (nodes, root, [first, second, third], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        for handle in [first, second, third] {
            focus.set_focusable(handle, true);
        }

        focus.focus_next();
        focus.focus_next();
        focus.focus_next();
        assert_eq!(focus.current(), Some(third));
        focus.focus_next();
        assert_eq!(focus.current(), Some(first), "focus wraps forward");
    }

    #[test]
    fn shift_tab_moves_focus_backwards() {
        let (nodes, root, [first, second, third], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        for handle in [first, second, third] {
            focus.set_focusable(handle, true);
        }

        let shift_tab = || {
            InputEvent::new(
                InputEventKind::KeyDown {
                    key: Key::Keyboard(Keycode::Tab),
                    keymod: Mod::LSHIFTMOD,
                },
                None,
            )
        };
        focus.focus_next();
        focus.focus_next();
        assert_eq!(focus.current(), Some(second));
        assert!(focus.handle_key(&shift_tab()));
        assert_eq!(focus.current(), Some(first));
    }

    #[test]
    fn shift_tab_from_the_first_node_wraps_to_the_last() {
        let (nodes, root, [first, second, third], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        for handle in [first, second, third] {
            focus.set_focusable(handle, true);
        }

        let shift_tab = InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(Keycode::Tab),
                keymod: Mod::RSHIFTMOD,
            },
            None,
        );
        focus.focus_next();
        assert_eq!(focus.current(), Some(first));
        assert!(focus.handle_key(&shift_tab));
        assert_eq!(focus.current(), Some(third), "focus wraps backward");
    }

    #[test]
    fn a_key_that_is_not_tab_is_not_handled() {
        let (nodes, root, [first, ..], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        focus.set_focusable(first, true);

        let mut event = InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(Keycode::Space),
                keymod: Mod::empty(),
            },
            None,
        );
        assert!(!focus.handle_key(&event));
        assert_eq!(focus.current(), None, "focus did not move");

        event = InputEvent::new(
            InputEventKind::KeyUp {
                key: Key::Keyboard(Keycode::Tab),
                keymod: Mod::empty(),
            },
            None,
        );
        assert!(!focus.handle_key(&event), "a key release is not navigation");
    }

    #[test]
    fn a_gamepad_tab_is_not_focus_navigation() {
        let (nodes, root, [first, ..], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        focus.set_focusable(first, true);

        let event = InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Gamepad(Button::South),
                keymod: Mod::empty(),
            },
            None,
        );
        assert!(
            !focus.handle_key(&event),
            "only the keyboard Tab navigates focus"
        );
    }

    #[test]
    fn focus_moves_through_the_tree_in_paint_order() {
        let mut nodes = Arena::new();
        let inner_first = leaf(&mut nodes, Size::new(10.0, 10.0));
        let inner_second = leaf(&mut nodes, Size::new(10.0, 10.0));
        let inner = stack(&mut nodes, &[inner_first, inner_second]);
        let outer = leaf(&mut nodes, Size::new(10.0, 10.0));
        let root = stack(&mut nodes, &[inner, outer]);
        laid_out(&mut nodes, root, Size::new(100.0, 100.0));

        let mut focus = Focus::new(&nodes, root);
        for handle in [outer, inner_first, inner, inner_second] {
            focus.set_focusable(handle, true);
        }

        // Paint order is the parent before its children, so the order is
        // inner, inner_first, inner_second, outer — the registration order
        // above must not leak into the focus order.
        focus.focus_next();
        assert_eq!(focus.current(), Some(inner));
        focus.focus_next();
        assert_eq!(focus.current(), Some(inner_first));
        focus.focus_next();
        assert_eq!(focus.current(), Some(inner_second));
        focus.focus_next();
        assert_eq!(focus.current(), Some(outer));
    }

    #[test]
    fn a_tree_with_nothing_focusable_holds_no_focus() {
        let (nodes, root, ..) = three_stacked();
        let mut focus = Focus::new(&nodes, root);

        focus.focus_next();
        assert_eq!(focus.current(), None);
        focus.focus_prev();
        assert_eq!(focus.current(), None);
    }

    #[test]
    fn defocusing_the_focused_node_leaves_nothing_focused() {
        let (nodes, root, [first, second, ..], ..) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        focus.set_focusable(first, true);
        focus.set_focusable(second, true);
        focus.focus_next();
        assert_eq!(focus.current(), Some(first));

        focus.set_focusable(first, false);
        assert_eq!(focus.current(), None);
    }

    #[test]
    fn focusing_a_node_moves_focus_to_it_without_a_navigation_key() {
        let (nodes, root, [first, second, third], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        for handle in [first, second, third] {
            focus.set_focusable(handle, true);
        }

        assert!(focus.focus(third));
        assert_eq!(focus.current(), Some(third));
        assert!(focus.focus(first), "and straight on to another");
        assert_eq!(focus.current(), Some(first));
        // Navigation carries on from wherever focus was put.
        focus.focus_next();
        assert_eq!(focus.current(), Some(second));
    }

    #[test]
    fn focusing_a_node_nobody_offered_leaves_focus_alone() {
        let (nodes, root, [first, second, ..], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        focus.set_focusable(first, true);
        focus.set_focusable(second, true);
        focus.focus_next();
        assert_eq!(focus.current(), Some(first));

        assert!(!focus.focus(root), "the panel is not focusable");
        assert_eq!(focus.current(), Some(first), "so focus did not move");
    }

    #[test]
    fn a_steering_wheel_scroll_moves_focus() {
        let (nodes, root, [first, second, third], _) = three_stacked();
        let mut focus = Focus::new(&nodes, root);
        for handle in [first, second, third] {
            focus.set_focusable(handle, true);
        }

        focus.handle_scroll(1.0);
        assert_eq!(focus.current(), Some(first), "scroll forward moves forward");
        focus.handle_scroll(-1.0);
        assert_eq!(
            focus.current(),
            Some(third),
            "scroll backward from the first wraps to the last"
        );
        focus.handle_scroll(0.0);
        assert_eq!(focus.current(), Some(third), "a zero scroll leaves focus");
    }

    #[test]
    fn an_event_bubbles_to_the_parent_until_it_is_consumed() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[child]);
        let size = Size::new(100.0, 100.0);
        laid_out(&mut nodes, root, size);

        let mut delivered = Vec::new();
        let mut event = InputEvent::new(InputEventKind::Tap, Some(Offset::new(5.0, 5.0)));
        dispatch_event(&nodes, root, &mut event, &mut |handle, event| {
            delivered.push(handle);
            if handle == root {
                event.consume();
            }
        });

        assert_eq!(
            delivered,
            vec![child, root],
            "the child did not consume, so the event bubbled to the root"
        );
        assert!(event.consumed());
    }

    #[test]
    fn a_consumed_event_does_not_reach_the_parent() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[child]);
        let size = Size::new(100.0, 100.0);
        laid_out(&mut nodes, root, size);

        let mut delivered = Vec::new();
        let mut event = InputEvent::new(InputEventKind::Tap, Some(Offset::new(5.0, 5.0)));
        dispatch_event(&nodes, root, &mut event, &mut |handle, event| {
            delivered.push(handle);
            event.consume();
        });

        assert_eq!(delivered, vec![child], "the child consumed the event");
        assert!(event.consumed());
    }

    #[test]
    fn an_event_over_no_node_is_dropped() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[child]);
        let size = Size::new(100.0, 100.0);
        laid_out(&mut nodes, root, size);

        let mut delivered = Vec::new();
        let mut event = InputEvent::new(
            InputEventKind::Tap,
            Some(Offset::new(size.width + 10.0, size.height + 10.0)),
        );
        dispatch_event(&nodes, root, &mut event, &mut |handle, event| {
            delivered.push(handle);
            event.consume();
        });

        assert!(
            delivered.is_empty(),
            "a point over no node has no handler to receive it"
        );
        assert!(!event.consumed());
    }

    #[test]
    fn an_event_with_no_position_goes_to_the_root() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[child]);
        let size = Size::new(100.0, 100.0);
        laid_out(&mut nodes, root, size);

        let mut delivered = Vec::new();
        let mut event = InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(Keycode::Tab),
                keymod: Mod::empty(),
            },
            None,
        );
        dispatch_event(&nodes, root, &mut event, &mut |handle, event| {
            delivered.push(handle);
            event.consume();
        });

        assert_eq!(
            delivered,
            vec![root],
            "a key event is not routed by position"
        );
    }

    #[test]
    fn a_tap_is_dispatched_to_the_widget_under_the_point() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, Size::new(20.0, 20.0));
        let root = stack(&mut nodes, &[child]);
        let size = Size::new(100.0, 100.0);
        laid_out(&mut nodes, root, size);

        let mut tapped = Vec::new();
        let mut event = InputEvent::new(InputEventKind::Tap, Some(Offset::new(5.0, 5.0)));
        dispatch_event(&nodes, root, &mut event, &mut |handle, event| {
            if event.kind() == InputEventKind::Tap {
                tapped.push(handle);
            }
            event.consume();
        });

        assert_eq!(tapped, vec![child]);
    }

    // -------------------------------------------------------------- typed text

    #[test]
    fn typed_text_becomes_one_event_carrying_the_whole_run() {
        let mut recognizer = GestureRecognizer::new();

        // A single character for an ordinary keypress...
        let events = recognizer.process(&text_input("a"));
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].kind(),
            InputEventKind::Text {
                text: "a".to_string()
            }
        );

        // ...and a whole word for an IME composition commit, which is the case a
        // per-character event would have had to invent a boundary for. Written as
        // two runs on purpose: SDL delivers them as two events, and this says the
        // recogniser does not merge them and does not split them either.
        let events = recognizer.process(&text_input("ni hao"));
        assert_eq!(
            events[0].kind(),
            InputEventKind::Text {
                text: "ni hao".to_string()
            }
        );
    }

    #[test]
    fn typed_text_carries_no_position_because_a_character_happens_nowhere() {
        let mut recognizer = GestureRecognizer::new();
        let events = recognizer.process(&text_input("x"));

        assert_eq!(events[0].position(), None);
        // Which is what puts it on the focused control's path: `route` sends a
        // positionless event to the root, not to whatever is under a point.
        assert!(events[0].position().is_none());
    }

    #[test]
    fn an_empty_run_is_not_an_event() {
        let mut recognizer = GestureRecognizer::new();

        // Clearing an IME composition reports what is left, which is nothing. An
        // event carrying "" would be one every consumer has to learn to ignore,
        // and "ignore it" is exactly what a widget with a text buffer would fail
        // to do.
        assert!(recognizer.process(&text_input("")).is_empty());
    }

    #[test]
    fn typed_text_and_a_key_down_are_two_events_from_two_sdl_events() {
        let mut recognizer = GestureRecognizer::new();

        // The distinction the variant exists for: SDL sends BOTH a KEYDOWN and a
        // TEXTINPUT for one physical keypress. A widget must be able to read the
        // character from one and the navigation keys from the other, so neither
        // may swallow the other.
        let key = recognizer.process(&key_down(Keycode::A));
        let text = recognizer.process(&text_input("a"));
        assert_eq!(key.len(), 1);
        assert!(matches!(key[0].kind(), InputEventKind::KeyDown { .. }));
        assert!(matches!(text[0].kind(), InputEventKind::Text { .. }));
    }

    #[test]
    fn typed_text_does_not_disturb_a_pointer_gesture_in_flight() {
        let mut recognizer = GestureRecognizer::new();

        let mut down = finger_down(1, 10.0, 10.0);
        still_timestamp(&mut down, 0);
        assert!(recognizer.process(&down).is_empty());

        // A character arriving mid-drag must not look like a pointer event, nor
        // reset the drag's start point: the drag that follows is still measured
        // from where the finger went down.
        assert_eq!(recognizer.process(&text_input("q")).len(), 1);

        let mut motion = finger_motion(1, 60.0, 10.0);
        still_timestamp(&mut motion, 1_000_000);
        let dragged: Vec<InputEventKind> = recognizer
            .process(&motion)
            .iter()
            .map(InputEvent::kind)
            .collect();
        assert!(
            dragged
                .iter()
                .any(|kind| matches!(kind, InputEventKind::Drag { .. })),
            "the drag after an intervening character was lost: {dragged:?}"
        );
    }

    /// Sets the timestamp of a finger event to `now`, the way SDL stamps an
    /// event that arrived then.
    fn still_timestamp(event: &mut Event, now: u64) {
        match event {
            Event::FingerDown { timestamp, .. }
            | Event::FingerMotion { timestamp, .. }
            | Event::FingerUp { timestamp, .. }
            | Event::MouseButtonDown { timestamp, .. }
            | Event::MouseButtonUp { timestamp, .. }
            | Event::MouseMotion { timestamp, .. }
            | Event::MouseWheel { timestamp, .. }
            | Event::KeyDown { timestamp, .. }
            | Event::KeyUp { timestamp, .. }
            | Event::GamepadButtonDown { timestamp, .. }
            | Event::GamepadButtonUp { timestamp, .. }
            | Event::GamepadAxisMotion { timestamp, .. } => *timestamp = now,
            _ => {}
        }
    }
}
