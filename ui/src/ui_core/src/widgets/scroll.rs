//! The Scroll widget: a window onto content taller than the box it is in.
//!
//! A scroll is a **viewport** and an **offset**, and almost everything else about
//! it is one of two decisions. The first is the direction of the offset, and it
//! is fixed here once so that no caller has to guess: [`Scroll::scroll_offset`]
//! is the distance the content is shifted **up** by. Zero is the top of the
//! content, a positive offset reveals what is below it, and the content is drawn
//! at `rect.y - scroll_offset`. That makes the offset read the way it is named
//! and the way requirement 3's `[0, max_scroll]` reads: zero is the top, the
//! maximum is the end.
//!
//! A finger is the other direction, and it is the one that has to be pinned by a
//! test rather than by a comment. Dragging **down** moves the content **down**,
//! which reveals what was above it, which is the offset going *back towards zero*
//! — so a drag subtracts. `a_drag_down_moves_the_content_down_and_reveals_what
//! _was_above` is the test that says so, and it says it in the only way that can
//! be checked: by the offset, by the content rect, and by the content node's own
//! position in the tree, all at once.
//!
//! The second decision is clipping, and it is a decision this widget is not free
//! to make. [`DrawCommand`] has **no scissor state of its own** —
//! [`Renderer::set_scissor`](crate::render::Renderer::set_scissor) applies to the
//! whole frame, and `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the
//! spec, and why* records per-node clipping as a deferral. So a scroll cannot
//! clip at the GL level, and nothing here pretends to. What it can do is the
//! half that does not need a scissor:
//!
//! - [`Scroll::clip_rect`] hands back the rect a renderer would scissors to, and
//!   that rect is **already computed by the layout pass**: it is the content
//!   node's own [`LayoutState::clip`](crate::layout::LayoutState::clip), which
//!   the pass fills in as the intersection of every ancestor's box with the
//!   scroll's own box. Nothing here recomputes it, because a second copy of that
//!   intersection arithmetic is a second thing to keep in step with
//!   `layout::intersect`, and a stale copy would clip to the wrong place.
//! - [`clip_commands`] drops the commands that are entirely outside that rect.
//!   That is requirement 5's "items outside the viewport are not drawn", and it
//!   is the half that is a real saving: an item the scroll knows is off screen
//!   costs nothing at all.
//! - [`visible_rect`] says which band of the content is on screen at a given
//!   offset, and it is what a caller uses to decide *not to build* an off-screen
//!   item's commands. That is what carries the requirement; the other two make it
//!   cheap.
//!
//! [`clip_commands`] **keeps a command that straddles the edge whole** rather
//! than trimming it, and that is a deliberate choice with a reason. A
//! [`DrawCommand::RoundedRect`] *fills* its rect, so a rounded rectangle trimmed
//! to part of itself is not a clipped rounded rectangle — it is a smaller one,
//! with its own corner radii, which is a different shape. The same is true of a
//! [`DrawCommand::Circle`], whose centre and radius cannot express the part of a
//! disc that is left, and of a [`DrawCommand::Text`] run, whose width is not in
//! the command at all. Real clipping needs the scissor; the honest thing for a
//! function that has none is to drop what is wholly gone and leave what is
//! partly there for the scissor to cut.
//!
//! # Where the arena is needed
//!
//! A node cannot reach the arena that holds it, because the arena owns the node.
//! Every method here that has to touch the tree therefore takes the arena, the
//! way [`Slider::on_event`](crate::widgets::slider::Slider::on_event) takes a
//! `rect` and [`Container::add_child`](crate::widgets::container::Container::add_child)
//! takes `&mut Arena`:
//!
//! - [`Scroll::content_size`] and [`Scroll::clip_rect`] read the tree, and take
//!   `&Arena<WidgetNode>`.
//! - [`Scroll::sync_content`] reads the content's laid-out height into the
//!   widget, and takes `&Arena<WidgetNode>`.
//! - [`Scroll::apply_offset`] moves the content node, and takes
//!   `&mut Arena<WidgetNode>`.
//! - [`Scroll::new`] takes it to create the node and to attach the content.
//!
//! [`Scroll::on_event`] deliberately does **not** take it, exactly like the
//! slider's: an input handler is called from [`input::dispatch_event`](crate::input::dispatch_event), and
//! re-entering the arena from there is the double borrow
//! [`input::route`](crate::input::route) documents. So a frame is two steps —
//! `on_event` moves the offset, `apply_offset` moves the node — and the second
//! step is the one that needs the tree.
//!
//! # What is deliberately not here
//!
//! - **Momentum.** Requirement 3 marks smooth scrolling with momentum optional
//!   and this task does not ask for it, so a released finger stops the content
//!   exactly where it is.
//! - **The analogue stick.** The input module maps exactly one gamepad axis —
//!   [`STEERING_WHEEL_SCROLL_AXIS`](crate::input::STEERING_WHEEL_SCROLL_AXIS),
//!   [`Axis::RightX`](sdl3::gamepad::Axis::RightX) — into a positionless
//!   [`Scroll`](InputEventKind::Scroll), and this widget handles that event, so a
//!   steering wheel's scroll axis already reaches a focused scroll. Mapping
//!   `Axis::LeftY` is a change to `input.rs`, a module task 10 shipped, and it is
//!   not made here.
//! - **Horizontal scrolling**, grid content, pull-to-refresh and sticky headers.
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::input::{InputEvent, InputEventKind};
//! use ui_core::layout::{Constraints, LayoutState, Offset, Size};
//! use ui_core::node::{self, WidgetNode};
//! use ui_core::paint::Rect;
//! use ui_core::widgets::scroll::Scroll;
//!
//! let mut nodes = Arena::new();
//! let content = node::create(
//!     &mut nodes,
//!     LayoutState::new().with_constraints(Constraints::tight(Size::new(200.0, 900.0))),
//! );
//! let mut scroll = Scroll::new(&mut nodes, content);
//! scroll.set_content_height(900.0);
//!
//! let rect = Rect::new(0.0, 0.0, 200.0, 300.0);
//! // 900 of content in a 300 viewport is 600 to scroll through.
//! assert_eq!(ui_core::widgets::scroll::max_scroll(rect.height, scroll.content_height()), 600.0);
//!
//! // A wheel notch scrolls towards the end, and is consumed.
//! let mut wheel = InputEvent::new(
//!     InputEventKind::Scroll { delta: Offset::new(0.0, 1.0) },
//!     Some(Offset::new(100.0, 150.0)),
//! );
//! assert!(scroll.on_event(&mut wheel, rect));
//! assert!(wheel.consumed());
//!
//! // The content is drawn that many pixels *above* the viewport's top edge.
//! scroll.scroll_offset.set(60.0);
//! assert_eq!(scroll.content_rect(rect).y, -60.0);
//! ```

use std::cell::{Cell, RefCell};
use std::time::Duration;

use crate::animation::AnimationClock;
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::{self as layout_module};
use crate::layout::{Constraints, LayoutMode, LayoutState, Offset, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::button::Motion;

/// The width a scroll asks for when its caller gives it no width of its own.
///
/// A scroll is a container, so the size that matters is the caller's — the box
/// it is laid out in. This is only for a caller with nothing else to go on, and
/// [`Scroll::new`] gives the node this size so that the layout pass does not
/// measure a viewport to fit its own content, which would defeat the viewport
/// entirely. It is a constant rather than a theme token for the reason
/// `WHEEL_STEP` and the sizing constants in the other widgets are: the theme
/// has no token for a scrollbar's parts.
const DEFAULT_VIEWPORT_WIDTH: f32 = 240.0;

/// The height a scroll asks for when its caller gives it none of its own. See
/// `DEFAULT_VIEWPORT_WIDTH`.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 320.0;

/// How thick the scrollbar's groove and its thumb are, in pixels, unless a
/// caller changes it.
///
/// The groove is a rounded rectangle, so its radius is half of this — which is
/// what makes it a line rather than a bar.
///
/// **Six is a baseline, and the operator rejected it for the same reason they
/// rejected the slider's 6-pixel track** (2026-09-30): *"too narrow, I have
/// issues with pointing on it with my mouse"*. Six pixels is a hairline —
/// correct with a mouse, where precision is free, and wrong with a fingertip,
/// which is roughly 40 across.
///
/// The constant is **not** raised, for the reason the slider's sizing constants
/// give: it is a documented baseline that [`Scroll::set_thickness`] exists to
/// change, and the demo's number is the operator's call rather than the
/// library's. What would reverse *that* is the head unit's own bezel and glove
/// spec. The slider made the same split at the same time and with the same
/// reasoning — see `ui_demo`'s `SLIDER_TRACK_THICKNESS`.
const SCROLLBAR_THICKNESS: f32 = 6.0;

/// How far the scrollbar sits from the right edge of the viewport, in pixels.
///
/// It is half the focus ring's width, so the ring drawn around the thumb stays
/// inside the node the viewport owns instead of crossing its edge.
const SCROLLBAR_MARGIN: f32 = 2.0;

/// The shortest the scrollbar's thumb may be, in pixels.
///
/// The thumb's length is the visible fraction of the content, so a content
/// thousands of pixels long gives a thumb a fraction of a pixel — a speck that
/// is there and cannot be seen or aimed at. This is the floor that stops that,
/// and the run the thumb travels over is shortened by it, so the thumb can still
/// reach the very end of the groove.
const SCROLLBAR_MIN_THUMB: f32 = 16.0;

/// The focus ring's width, in pixels, unless a caller changes it.
///
/// The ring is drawn around the **thumb** and then covered by it, so only its
/// border shows: a [`DrawCommand::RoundedRect`] fills its rect, and a filled
/// rectangle with nothing drawn over its middle is a card rather than an
/// outline. [`Scroll::paint`] says so where it draws it.
const FOCUS_RING: f32 = 2.0;

/// How far one wheel notch scrolls the content, in pixels.
///
/// The wheel's own magnitude is not used, and the reason is the slider's: the
/// input module puts a wheel's notch and a gamepad axis's count in the same
/// field, and a `Scroll` off a steering wheel's axis carries 32000 for a full
/// deflection. The only thing both agree on is which way the control was pushed,
/// so that is all that is read, and one notch is this many pixels either way.
const WHEEL_STEP: f32 = 48.0;

/// How much of the viewport one arrow press scrolls, as a fraction of it.
///
/// A fraction rather than a constant for the reason the slider's
/// `KEY_STEP_FRACTION` is one: a step of 20 pixels is a fifth of a 100-pixel
/// control and a fortieth of a 400-pixel one, and only the fraction means the
/// same thing in both. A viewport with no height has nowhere to scroll, and its
/// key press moves nothing — it is still consumed, because the key was aimed at
/// this scroll.
const KEY_STEP_FRACTION: f32 = 0.1;

/// The colours a scrollbar draws with.
///
/// Three colours: the groove the thumb runs in, the thumb itself, and the focus
/// indicator. They are not tokens of their own — the theme has none per part,
/// and adding one per part would put three more tokens in every theme table and
/// in every theme switch — so a scrollbar is themed with the theme's own three
/// and [`Palette::from_theme`] says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The groove: the whole run the thumb travels over, unfilled.
    pub track: Color,
    /// The thumb: the part of the run the current offset has reached.
    pub thumb: Color,
    /// The focus indicator, drawn around the thumb and covered by it.
    pub ring: Color,
}

impl Default for Palette {
    /// Returns a neutral grey scrollbar: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    fn default() -> Self {
        Palette {
            track: Color::new(64, 64, 64, 255),
            thumb: Color::new(160, 160, 160, 255),
            ring: Color::new(255, 255, 255, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The groove is [`Border`](crate::theme::ThemeToken::Border), the theme's
    /// hairline colour and the only one of its nine that is *muted* by
    /// definition — a scrollbar's groove is the part of it that is not the
    /// position. The thumb is [`TextMuted`](crate::theme::ThemeToken::TextMuted),
    /// which is by definition "present but not emphasised", and that is what a
    /// scrollbar is: it is telling the user where they are, not offering them a
    /// control. A `Primary` thumb would be a button.
    ///
    /// The ring is [`Text`](crate::theme::ThemeToken::Text), for the reason the
    /// slider's is: the ring is drawn on whatever the scroll is laid over, which
    /// is the window's background and not the groove, and `Text` is the colour
    /// this repository uses for anything that has to be legible on the
    /// background itself.
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            track: token_color(theme, crate::theme::ThemeToken::Border),
            thumb: token_color(theme, crate::theme::ThemeToken::TextMuted),
            ring: token_color(theme, crate::theme::ThemeToken::Text),
        }
    }
}

/// The appearance the scroll's focus implies.
///
/// Every field is a target, not a value in flight:
/// [`Scroll::animate_to_state`] animates the scroll's properties toward this and
/// [`Scroll::paint`] draws whatever the properties have reached, which is a
/// [`Style`] part way through on a frame where a theme is switching.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The colour of the groove.
    pub track: Color,
    /// The colour of the thumb.
    pub thumb: Color,
    /// The focus ring's thickness, or zero for no ring.
    pub ring_width: f32,
}
/// A scrollable viewport: a window onto content, shifted by an offset.
///
/// The widget holds the two things the task gives it — [`content`] and
/// [`scroll_offset`] — and the three properties it animates, [`track`], [`thumb`]
/// and [`focus_ring`]. [`scroll_offset`] is the truth, and it is **never
/// animated**: an interaction is immediate, and a viewport that eases towards a
/// finger is a viewport the finger has already passed. What
/// [`animate_to_state`](Scroll::animate_to_state) moves is the *colours*, and it
/// moves them because a theme switch is animated and a theme switch is a caller
/// announcing a new [`Palette`](Scroll::set_palette) and then aiming the scroll
/// at it.
///
/// The one number that is not a property is
/// [`content_height`](Scroll::content_height), the extent of the content the
/// offset is clamped against. It
/// is a plain field behind a setter because it is a *measurement* rather than an
/// appearance, and nothing about it animates. It starts at zero, and a scroll
/// that has not been told how tall its content is has no geometry to clamp
/// against: it draws no scrollbar and it does not scroll. That is the honest
/// answer rather than a failure mode to paper over, and
/// [`sync_content`](Scroll::sync_content) is the call that normally supplies the
/// number after a layout pass.
///
/// The node is the caller's to size through
/// [`layout_mut`](crate::node::WidgetNode::layout_mut) — [`size`](Scroll::size)
/// is only what [`Scroll::new`] hands it so that the layout pass does not
/// measure a viewport to fit its own content. A caller that gives the node a box
/// of its own replaces that, and the widget paints inside whatever rect it is
/// given.
///
/// [`content`]: Scroll::content
/// [`track`]: Scroll::track
/// [`thumb`]: Scroll::thumb
/// [`focus_ring`]: Scroll::focus_ring
/// [`scroll_offset`]: Scroll::scroll_offset
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::{InputEvent, InputEventKind};
/// use ui_core::layout::{LayoutState, Offset};
/// use ui_core::node::{self, WidgetNode};
/// use ui_core::paint::Rect;
/// use ui_core::widgets::scroll::Scroll;
///
/// let mut nodes = Arena::new();
/// let content = node::create(&mut nodes, LayoutState::new());
/// let mut scroll = Scroll::new(&mut nodes, content);
/// scroll.set_content_height(900.0);
///
/// let rect = Rect::new(0.0, 0.0, 200.0, 300.0);
/// // **Down is later** — see [`gesture_delta`]. A drag *up* the screen from the
/// // top reveals what is above, which is nothing: the offset is already at zero
/// // and cannot go below it. A drag *down* is the one that moves it.
/// let mut up = InputEvent::new(
///     InputEventKind::Drag { delta: Offset::new(0.0, -100.0) },
///     Some(Offset::new(100.0, 50.0)),
/// );
/// assert!(scroll.on_event(&mut up, rect));
/// assert_eq!(scroll.scroll_offset.get(), 0.0, "clamped at the top");
///
/// let mut down = InputEvent::new(
///     InputEventKind::Drag { delta: Offset::new(0.0, 40.0) },
///     Some(Offset::new(100.0, 150.0)),
/// );
/// assert!(scroll.on_event(&mut down, rect));
/// assert_eq!(
///     scroll.scroll_offset.get(),
///     40.0,
///     "and a drag down the screen scrolls towards the end, by exactly how far \
///      the finger went"
/// );
/// ```
pub struct Scroll {
    /// The node this scroll shows.
    ///
    /// It is attached to the scroll's own node by [`Scroll::new`], which is what
    /// makes the layout pass place it and the paint pass walk it. Its
    /// [`LayoutState::position`](crate::layout::LayoutState::position) is what
    /// [`apply_offset`](Scroll::apply_offset) writes, and its laid-out rect is
    /// what [`content_size`](Scroll::content_size) reads.
    pub content: Handle,
    /// How far the content is shifted **up**, in pixels.
    ///
    /// Zero is the top of the content and the maximum is [`max_scroll`] of the
    /// viewport. Every write the widget makes is clamped; a caller that writes the
    /// property directly is writing the truth with no clamp, which the widget
    /// survives — [`paint`](Scroll::paint) pins a thumb that is off the end
    /// rather than drawing it outside the node.
    pub scroll_offset: Property<f32>,
    /// The colour of the scrollbar's groove.
    pub track: Property<Color>,
    /// The colour of the scrollbar's thumb.
    pub thumb: Property<Color>,
    /// The focus ring's thickness, in pixels. Zero draws no ring even when the
    /// scroll is focused.
    pub focus_ring: Property<f32>,
    /// Whether the scroll holds focus. Written by the caller, from
    /// [`input::Focus`](crate::input::Focus), and it is what a key press acts on.
    pub focused: Property<bool>,
    content_height: f32,
    thickness: f32,
    /// Where inside the thumb a pointer grabbed it, in pixels from the thumb's
    /// top, or `None` when nothing is grabbed.
    ///
    /// It is the whole of the thumb-drag fix, and its being a *Cell* is why
    /// [`on_event`](Scroll::on_event) can act on a drag: that method takes
    /// `&self`, because it is called from
    /// [`input::dispatch_event`](crate::input::dispatch_event) and cannot reach
    /// the arena — so the state a drag has to write cannot be a plain field.
    grabbed: Cell<Option<f32>>,
    palette: Palette,
    clock: RefCell<AnimationClock>,
    node: Handle,
}

impl Scroll {
    /// Creates a scroll showing `content` in the arena, and returns it.
    ///
    /// The content is **attached to the scroll's own node here**, so a caller
    /// does not have to: the layout pass only places a node that is in the tree,
    /// and the paint pass only walks one. A caller who has already attached it to
    /// this scroll gets the second attach refused, which
    /// [`is_attached`](Scroll::is_attached) reports.
    ///
    /// The scroll's node is laid out in [`LayoutMode::Absolute`], because that is
    /// the one mode in which a child sits where it says rather than where its
    /// parent's flow puts it — and a viewport's child's position is the whole of
    /// what scrolling is. The mode also says the node holds exactly one child: a
    /// second one would be placed at its own declared position or at the origin,
    /// and neither is a thing a caller means by adding it.
    ///
    /// The node is given [`Constraints::tight`] of [`size`](Scroll::size) so
    /// that the layout pass does not measure the viewport to fit its own content
    /// — a viewport sized to its content is a viewport that does not scroll. A
    /// caller that gives the node a box of its own replaces that.
    ///
    /// The offset starts at zero, the content height at zero — see
    /// [`content_height`](Scroll::content_height) — and the colours at the
    /// neutral defaults until a caller gives it a
    /// [`Palette`](Scroll::set_palette) and calls
    /// [`snap_to_state`](Scroll::snap_to_state).
    ///
    /// The task file's `Scroll::new(content) -> Handle` is read the way task 12's
    /// [`Button::new`](crate::widgets::button::Button::new) and task 14's
    /// [`Slider::new`](crate::widgets::slider::Slider::new) were read: the handle
    /// is [`handle`](Scroll::handle)'s, and returning it alone would leave a
    /// caller with no properties to set and no way to give the scroll its content
    /// height, which is the one number it cannot guess.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, content: Handle) -> Self {
        let palette = Palette::default();
        let node = node::create(
            nodes,
            LayoutState::new()
                .with_mode(LayoutMode::Absolute)
                .with_constraints(Constraints::tight(Size::new(
                    DEFAULT_VIEWPORT_WIDTH,
                    DEFAULT_VIEWPORT_HEIGHT,
                ))),
        );
        // The attach's answer is reported by `is_attached` rather than here:
        // `new` has no way to hand a `bool` back without changing the shape
        // `Button::new` and `Slider::new` settled.
        let _ = node::attach(nodes, node, content);
        Scroll {
            content,
            scroll_offset: Property::new(0.0),
            track: Property::new(palette.track),
            thumb: Property::new(palette.thumb),
            focus_ring: Property::new(FOCUS_RING),
            focused: Property::new(false),
            content_height: 0.0,
            thickness: SCROLLBAR_THICKNESS,
            // A grab is one number and the widget is shared, so it is a `Cell`:
            // `on_event` takes `&self` because an input handler cannot re-enter
            // the arena, and a grabbed thumb is exactly the state such a handler
            // has to write. It is `Option<f32>` — "the pointer landed this far
            // down the thumb" — and `None` is no grab.
            grabbed: Cell::new(None),
            palette,
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the scroll's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns whether the scroll's own node holds the content as a child.
    ///
    /// [`Scroll::new`] attaches it, and this is how a caller finds out that the
    /// attach was refused — because the content already had a parent, or because
    /// the handle does not resolve. A scroll whose content is not attached holds
    /// a node that no layout pass will ever place and no paint pass will ever
    /// walk, so everything else about it is inert.
    #[must_use]
    pub fn is_attached(&self, nodes: &Arena<WidgetNode>) -> bool {
        nodes
            .get(self.node)
            .is_some_and(|node| node.children().contains(&self.content))
    }

    /// Returns how tall the content is, in pixels, as the widget last learned.
    ///
    /// Zero until a caller gives it a number — see the note on
    /// [`content_height`](Scroll::content_height) on the struct itself.
    #[must_use]
    pub fn content_height(&self) -> f32 {
        self.content_height
    }

    /// Sets how tall the content is, in pixels, and returns what it set.
    ///
    /// A negative height is floored at zero: an extent cannot be negative, and a
    /// negative one would make [`max_scroll`] larger than the content it
    /// describes. The setter does **not** re-clamp the offset, because it cannot:
    /// clamping needs the viewport's height and the setter is not given a rect. A
    /// caller that has just changed the size of the content calls
    /// [`scroll_by`](Scroll::scroll_by) with a delta of zero, which is exactly
    /// this widget's re-clamp.
    ///
    /// The normal way to supply the number is
    /// [`sync_content`](Scroll::sync_content), which reads it out of the arena
    /// after a layout pass rather than having the caller measure it twice.
    pub fn set_content_height(&mut self, height: f32) -> f32 {
        self.content_height = height.max(0.0);
        self.content_height
    }

    /// Returns the content's laid-out size, or `None` if it has not been laid
    /// out or the arena no longer holds it.
    ///
    /// This is where the arena is needed: a handle addresses the arena rather
    /// than a node, so the node cannot answer this about itself. A caller that
    /// wants the number every frame asks the widget to keep it — see
    /// [`sync_content`](Scroll::sync_content).
    #[must_use]
    pub fn content_size(&self, nodes: &Arena<WidgetNode>) -> Option<Size> {
        nodes
            .get(self.content)
            .map(WidgetNode::layout)
            .and_then(LayoutState::rect)
            .map(|rect| rect.size)
    }

    /// Reads the content's laid-out height out of `nodes` into the widget, and
    /// reports whether it changed.
    ///
    /// This is the call a frame makes after its layout pass: the content's height
    /// is a measurement, and the widget cannot take it without the arena. It
    /// leaves the height alone when the content has not been laid out or the
    /// arena no longer holds it — there is no number to read, and zeroing the
    /// height would silently turn a scroll that worked into one that does not.
    ///
    /// A caller that has just changed the size of the content also calls
    /// [`scroll_by`](Scroll::scroll_by) with a delta of zero, to re-clamp an
    /// offset that may now be past the end.
    pub fn sync_content(&mut self, nodes: &Arena<WidgetNode>) -> bool {
        let Some(size) = self.content_size(nodes) else {
            return false;
        };
        let height = size.height.max(0.0);
        if (self.content_height - height).abs() <= f32::EPSILON {
            return false;
        }
        self.content_height = height;
        true
    }

    /// Moves the content node so that the tree agrees with
    /// [`scroll_offset`](Scroll::scroll_offset), and reports whether it moved.
    ///
    /// This is the second of the frame's two steps and the one that needs the
    /// arena: `on_event` writes the offset because an input handler cannot
    /// re-enter the arena, and this writes the position the layout pass reads.
    /// The node is placed in [`LayoutMode::Absolute`] for exactly this — its
    /// position *is* the offset, negated, and a child of a flow-mode node has
    /// nowhere to put it.
    ///
    /// Nothing animates the offset, so the two cannot disagree: this writes the
    /// property's own value every time, and a caller that wrote the property
    /// itself gets the node moved by it.
    ///
    /// The content and its ancestors are marked dirty, because a node that moves
    /// has to take the chain above it with it and only the arena can walk it. A
    /// handle the arena no longer holds changes nothing and reports `false` —
    /// there is no node left to move.
    pub fn apply_offset(&self, nodes: &mut Arena<WidgetNode>) -> bool {
        let target = Offset::new(0.0, -self.scroll_offset.get());
        let Some(node) = nodes.get(self.content) else {
            return false;
        };
        // A child that declares no position sits at the parent's origin, so an
        // unset position and `Offset::ZERO` are the same place and an offset of
        // zero has nothing to do.
        if node.layout().position().unwrap_or(Offset::ZERO) == target {
            return false;
        }
        if let Some(node) = nodes.get_mut(self.content) {
            node.layout_mut().set_position(Some(target));
        }
        layout_module::mark_dirty(nodes, self.content);
        true
    }

    /// Returns the rect a renderer would scissors the content to, or `None` if
    /// the arena no longer holds the content.
    ///
    /// This is the layout pass's own answer, read off the content node: the
    /// intersection of every ancestor's box with the scroll's own. It is not
    /// recomputed here — a second copy of that arithmetic would be a second
    /// thing to keep in step with the pass, and the wrong answer here is a
    /// scissor in the wrong place, which is a wrong picture rather than a slow
    /// frame.
    ///
    /// `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the spec, and why*
    /// records that no per-node scissor is applied yet:
    /// [`Renderer::set_scissor`](crate::render::Renderer::set_scissor) is a
    /// frame-wide state and a [`DrawCommand`] carries none. **Applying this rect
    /// is that task's job**, at the point in the render pass where the content
    /// node's commands are submitted; until then the viewport's own box is the
    /// only thing keeping a scrolled list inside it.
    #[must_use]
    pub fn clip_rect(&self, nodes: &Arena<WidgetNode>) -> Option<Rect> {
        nodes
            .get(self.content)
            .map(WidgetNode::layout)
            .and_then(LayoutState::clip)
            .map(Rect::from)
    }
    /// Returns how thick the scrollbar draws, in pixels.
    ///
    /// It is the widget's own six-pixel baseline until a caller changes it with
    /// [`set_thickness`](Scroll::set_thickness), which is the same split
    /// [`Slider::set_track_thickness`](crate::widgets::slider::Slider::set_track_thickness)
    /// makes for the slider's track.
    #[must_use]
    pub fn thickness(&self) -> f32 {
        self.thickness
    }

    /// Sets how thick the scrollbar's groove and its thumb draw, in pixels, and
    /// returns what it set.
    ///
    /// A negative thickness is floored at zero, for the reason every other
    /// extent here is: a thickness is a length, and a negative one would put the
    /// groove's own left edge to the right of its right edge.
    ///
    /// **This changes the drawn bar and the bar's hit test together**, because
    /// they are the same number. A scrollbar a finger cannot land on is not a
    /// narrower scrollbar, it is an unusable one, and the two widths are what
    /// [`grab_thumb`](Scroll::grab_thumb) is measured against — so widening the
    /// bar here widens the grabbable part by the same amount rather than leaving
    /// a visible target that is still only as big as the old hairline.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::scroll::Scroll;
    ///
    /// let mut nodes = Arena::new();
    /// let content = ui_core::node::create(&mut nodes, ui_core::layout::LayoutState::new());
    /// let mut scroll = Scroll::new(&mut nodes, content);
    /// assert_eq!(scroll.thickness(), 6.0, "the widget's own baseline");
    /// assert_eq!(scroll.set_thickness(12.0), 12.0);
    /// assert_eq!(scroll.set_thickness(-4.0), 0.0, "and a length cannot be negative");
    /// ```
    pub fn set_thickness(&mut self, thickness: f32) -> f32 {
        self.thickness = thickness.max(0.0);
        self.thickness
    }

    /// Returns the colours the scrollbar draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the scrollbar draws with, and leaves the current ones
    /// where they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](Scroll::animate_to_state) or
    /// [`snap_to_state`](Scroll::snap_to_state): a theme switch is animated, and
    /// a theme switch is the caller announcing a new palette and then moving the
    /// scrollbar toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the scrollbar chasing a palette that is
    /// still moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the size a scroll asks for: `DEFAULT_VIEWPORT_WIDTH` by
    /// `DEFAULT_VIEWPORT_HEIGHT`.
    ///
    /// A scroll is a container and the box it lives in is its caller's decision,
    /// so this is only what [`Scroll::new`] gives the node to keep the layout
    /// pass from measuring a viewport to fit its own content. A caller that lays
    /// the node out itself writes whatever it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](Scroll::paint) draws inside whatever it is given.
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT)
    }

    /// Returns the box the content occupies inside `rect`, once the offset has
    /// been applied.
    ///
    /// The content is the viewport's width — a vertical scroll does not move it
    /// sideways — its own [`content_height`](Scroll::content_height) tall, and
    /// shifted **up** by the offset, so its top edge is `rect.y - scroll_offset`.
    /// It is the same arithmetic [`apply_offset`](Scroll::apply_offset) hands to
    /// the layout pass, and a test can therefore compare the two.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node::{self, WidgetNode};
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::scroll::Scroll;
    ///
    /// let mut nodes = Arena::new();
    /// let content = node::create(&mut nodes, LayoutState::new());
    /// let mut scroll = Scroll::new(&mut nodes, content);
    /// scroll.set_content_height(900.0);
    ///
    /// // A viewport that is not at the origin: the offset shifts the content
    /// // from *its* top edge, and that edge is 120, not 0.
    /// let rect = Rect::new(664.0, 120.0, 200.0, 300.0);
    /// scroll.scroll_offset.set(60.0);
    /// assert_eq!(scroll.content_rect(rect).y, 60.0);
    /// assert_eq!(scroll.content_rect(rect).height, 900.0);
    /// ```
    #[must_use]
    pub fn content_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            rect.x,
            rect.y - self.scroll_offset.get(),
            rect.width,
            self.content_height,
        )
    }

    /// Moves the offset by `delta`, clamps it to `0.0..=max_scroll`, writes it,
    /// and returns what it settled on.
    ///
    /// A delta of zero is this widget's **re-clamp**: it writes the offset back
    /// inside the range the current viewport and content allow, and returns it.
    /// That is what a caller does after the content has changed size, because the
    /// offset it was holding may now be past the end.
    ///
    /// A delta that would leave the range is clamped rather than refused: a drag
    /// past the top or the bottom of the content is an ordinary thing for a
    /// finger to do, and the content stops there.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node::{self, WidgetNode};
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::scroll::Scroll;
    ///
    /// let mut nodes = Arena::new();
    /// let content = node::create(&mut nodes, LayoutState::new());
    /// let mut scroll = Scroll::new(&mut nodes, content);
    /// scroll.set_content_height(900.0);
    /// let rect = Rect::new(0.0, 0.0, 200.0, 300.0);
    ///
    /// assert_eq!(scroll.scroll_by(rect, 400.0), 400.0);
    /// assert_eq!(
    ///     scroll.scroll_by(rect, 400.0),
    ///     600.0,
    ///     "900 of content in a 300 viewport stops at 600"
    /// );
    /// ```
    #[must_use]
    pub fn scroll_by(&self, rect: Rect, delta: f32) -> f32 {
        let settled = clamp_scroll(
            self.scroll_offset.get() + delta,
            rect.height,
            self.content_height,
        );
        self.scroll_offset.set(settled);
        settled
    }

    /// Returns the appearance the scroll's focus implies.
    ///
    /// The ring is applied independently of the colours, the way a button's and
    /// a slider's are: a scroll that is both mid-theme-switch and focused keeps
    /// both.
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            track: self.palette.track,
            thumb: self.palette.thumb,
            ring_width: if self.focused.get() {
                self.focus_ring.get()
            } else {
                0.0
            },
        }
    }

    /// Applies the appearance the scroll's focus implies at once, with no
    /// transition.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a scrollbar that has just been given a
    /// [`Palette`](Scroll::set_palette) and has never animated — whose colour
    /// properties still hold the neutral defaults [`Scroll::new`] wrote, so
    /// without this a themed scrollbar starts out grey — and a caller that has
    /// written a property itself and wants the scroll to be that state now.
    ///
    /// Any transition already running is cleared first, so it cannot write over
    /// what this just set when it arrives.
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.track.set(style.track);
        self.thumb.set(style.thumb);
    }

    /// Starts the transitions that carry the scroll's colours from wherever they
    /// are toward the appearance [`style`](Scroll::style) implies, on `motion`.
    ///
    /// The scroll's own clock is cleared first, so the transitions this replaces
    /// stop where they are rather than writing over the new ones when they arrive
    /// — the reason the button and the slider each own a clock.
    ///
    /// The target is a snapshot, not a continuous one: a caller re-aims when its
    /// own state moves, exactly as the demo re-aims a button's press when the
    /// theme moves under it.
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.track
                .animate_to(style.track, motion.duration, motion.easing),
        );
        clock.add(
            self.thumb
                .animate_to(style.thumb, motion.duration, motion.easing),
        );
    }

    /// Advances the scroll's transitions by `delta`, and returns whether any of
    /// them wrote.
    ///
    /// It is the scroll's frame integration: call it once a frame, before the
    /// paint pass, with the time that frame took. The write is what reaches the
    /// node — a property callback registered by the caller marks the node dirty —
    /// so a caller that repaints only when this is true repaints exactly while a
    /// theme is moving.
    ///
    /// Scrolling does **not** go through here. The offset is not animated, so an
    /// interaction has nothing to tick: see [`Scroll::on_event`].
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Returns whether any of the scroll's transitions is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Returns the scrollbar's groove inside `rect`.
    ///
    /// It is inset from the right edge by `SCROLLBAR_MARGIN` and runs the full
    /// height of the viewport, because the thumb travels all of it.
    fn track_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            rect.x + rect.width - SCROLLBAR_MARGIN - self.thickness,
            rect.y,
            self.thickness,
            rect.height.max(0.0),
        )
    }

    /// Returns the groove's half-thickness, which is every radius a scrollbar
    /// shape is drawn with.
    ///
    /// It is the groove's own radius rather than a separate word for the thumb's,
    /// because the two are the same line: the groove and the thumb are both
    /// rounded rectangles of [`thickness`](Scroll::thickness), and a thumb drawn
    /// with any other radius would not be a rounded rectangle of the bar's width.
    fn radius(&self) -> f32 {
        self.thickness / 2.0
    }

    /// Returns the scrollbar's own rect inside `rect`: the strip a press on the
    /// scrollbar lands in, or `None` when this scroll has nothing to scroll and
    /// draws no scrollbar at all.
    ///
    /// It is the **groove's** rect rather than the thumb's, because it answers
    /// "was the press on the scrollbar", and a press below the thumb is still on
    /// the scrollbar — it just missed. It is `None` on the same condition the thumb
    /// is: a scroll with nothing to scroll draws no scrollbar, and a scrollbar
    /// that is not drawn is not there to be pressed.
    ///
    /// This is what a caller that owns its own gestures measures against:
    /// [`grab_thumb`](Scroll::grab_thumb) is the one that acts on it, and
    /// `List::item_at` is the one that keeps a tap on it off a row.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::scroll::Scroll;
    ///
    /// let mut nodes = Arena::new();
    /// let content = node::create(&mut nodes, LayoutState::new());
    /// let mut scroll = Scroll::new(&mut nodes, content);
    /// let rect = Rect::new(664.0, 120.0, 200.0, 300.0);
    ///
    /// // A scroll that has not been told how tall its content is draws no
    /// // scrollbar, so there is nothing to press.
    /// assert_eq!(scroll.scrollbar_rect(rect), None);
    ///
    /// scroll.set_content_height(900.0);
    /// assert_eq!(
    ///     scroll.scrollbar_rect(rect),
    ///     Some(Rect::new(856.0, 120.0, 6.0, 300.0)),
    ///     "the last 6 pixels of the viewport's right edge, inset by the margin"
    /// );
    /// ```
    #[must_use]
    pub fn scrollbar_rect(&self, rect: Rect) -> Option<Rect> {
        self.thumb_rect(rect).map(|_| self.track_rect(rect))
    }

    /// Records that a pointer at `position` has grabbed the scrollbar's thumb,
    /// and reports whether it did.
    ///
    /// This is the **press** half of dragging the thumb, and it is a call rather
    /// than an event because the gesture recogniser has no press to give: it
    /// reports a [`Tap`](InputEventKind::Tap) on the *release*, and a
    /// [`Drag`](InputEventKind::Drag) once the pointer has already moved. A
    /// widget cannot recover "the finger went down here" from either. That is
    /// why `dragging` on a [`Slider`](crate::widgets::slider::Slider) is written
    /// by its caller from a press, and it is the same reason twice over here.
    ///
    /// The grab is recorded as **where inside the thumb the pointer landed**, and
    /// not as "the pointer is over the thumb", because that is the whole of the
    /// lag the operator reported:
    ///
    /// > *"when I click it and drag - it doesn't follow my mouse cursor exactly,
    /// > it's like something was keeping it from moving faster"*
    /// > (2026-10-01)
    ///
    /// A drag that scrolls the offset by its own delta moves the **thumb** by
    /// `delta * run / max_scroll`, and `run` is shorter than the viewport by the
    /// thumb's own length while `max_scroll` is longer than the viewport by
    /// everything that does not fit. On the demo's list that is 252 against
    /// 2 520 — the thumb moves **a tenth** of the distance the cursor did, and
    /// no amount of tuning the drag would fix it, because the drag is the wrong
    /// mapping. Tracking the pointer's *position* against the groove is 1:1 by
    /// construction, and recording where in the thumb it landed is what keeps
    /// the thumb from jumping its width sideways under the finger on the first
    /// frame of the drag.
    ///
    /// A press that is not on the thumb — anywhere in the content, or on the
    /// part of the groove below the thumb — grabs nothing and returns `false`.
    /// The caller keeps its own press state, so a drag that started on a row
    /// still scrolls the content the way [`on_event`](Scroll::on_event) always
    /// has; a press that missed the thumb is not a scrollbar gesture.
    ///
    /// The grab holds until [`release_thumb`](Scroll::release_thumb), and the
    /// widget cannot release it itself: a release is as invisible to it as the
    /// press was.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::{LayoutState, Offset};
    /// use ui_core::node;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::scroll::Scroll;
    ///
    /// let mut nodes = Arena::new();
    /// let content = node::create(&mut nodes, LayoutState::new());
    /// let mut scroll = Scroll::new(&mut nodes, content);
    /// scroll.set_content_height(900.0);
    /// let rect = Rect::new(0.0, 0.0, 200.0, 300.0);
    ///
    /// // The thumb is the top sixth of a 300-tall groove over 900 of content.
    /// let on_thumb = Offset::new(197.0, 30.0);
    /// assert!(scroll.grab_thumb(on_thumb, rect));
    /// assert!(scroll.is_thumb_grabbed());
    ///
    /// // The content is not the scrollbar, so a press there grabs nothing.
    /// let on_content = Offset::new(100.0, 30.0);
    /// assert!(!scroll.grab_thumb(on_content, rect));
    /// ```
    pub fn grab_thumb(&self, position: Offset, rect: Rect) -> bool {
        let Some(thumb) = self.thumb_rect(rect) else {
            self.grabbed.set(None);
            return false;
        };
        if !contains(thumb, position) {
            self.grabbed.set(None);
            return false;
        }
        // The offset **inside** the thumb, not the pointer's position: this is
        // what makes the thumb keep still under the finger rather than snap so
        // its top edge is at the pointer.
        self.grabbed.set(Some(position.y - thumb.y));
        true
    }

    /// Gives up a grab recorded by [`grab_thumb`](Scroll::grab_thumb), and
    /// reports whether there was one.
    ///
    /// A caller calls this from its pointer release for the same reason it calls
    /// [`grab_thumb`](Scroll::grab_thumb) from its press: the gesture recogniser
    /// has no release event of its own for a drag, so a grab left behind would
    /// be the next gesture's, and a drag of the content would move the thumb.
    pub fn release_thumb(&self) -> bool {
        self.grabbed.replace(None).is_some()
    }

    /// Returns whether the scrollbar's thumb is currently grabbed.
    #[must_use]
    pub fn is_thumb_grabbed(&self) -> bool {
        self.grabbed.get().is_some()
    }

    /// Returns the offset a pointer at `position` over a grabbed thumb is asking
    /// for, or `None` when nothing is grabbed or the thumb has nowhere to travel.
    ///
    /// The thumb's top is the pointer's position less the recorded grab offset,
    /// and the offset is that position's share of the run the thumb travels —
    /// which is [`thumb_rect`](Scroll::thumb_rect)'s own arithmetic read
    /// backwards, so the two cannot drift apart: the thumb drawn at this offset
    /// is under the pointer by construction.
    ///
    /// A drag past either end of the groove gives that end's offset rather than
    /// a position off the side of the viewport, which is what
    /// [`clamp_scroll`] does for every other write.
    fn offset_under(&self, position: Offset, rect: Rect) -> Option<f32> {
        let grab = self.grabbed.get()?;
        let max = max_scroll(rect.height, self.content_height);
        let run = self.thumb_run(rect);
        // Nothing to scroll, or a thumb that fills its groove: there is no
        // position-to-offset mapping to invert, and dividing by a zero run would
        // answer `NaN` rather than an end of the document.
        if max <= 0.0 || run <= 0.0 {
            return None;
        }
        let thumb_top = position.y - grab;
        Some(bounded((thumb_top - rect.y) / run) * max)
    }

    /// Returns how far the scrollbar's thumb travels along the viewport, in
    /// pixels.
    ///
    /// It is the viewport less the thumb's own length, and it is
    /// [`thumb_rect`](Scroll::thumb_rect)'s number rather than a second copy of
    /// the subtraction — a second copy is a second thing to keep in step with the
    /// thumb's length, and the two disagreeing is a thumb that jumps.
    fn thumb_run(&self, rect: Rect) -> f32 {
        let thumb_height = self.thumb_length(rect);
        (rect.height - thumb_height).max(0.0)
    }

    /// Returns how long the scrollbar's thumb is, in pixels.
    ///
    /// It is the visible fraction of the content, floored at
    /// [`SCROLLBAR_MIN_THUMB`] and capped at the viewport, which are the two
    /// degenerate cases: a speck nobody can aim at, and a thumb taller than the
    /// groove it runs in.
    fn thumb_length(&self, rect: Rect) -> f32 {
        (rect.height * rect.height / self.content_height)
            .max(SCROLLBAR_MIN_THUMB)
            .min(rect.height)
            .max(0.0)
    }

    /// Returns the scrollbar's thumb inside `rect`, or `None` when there is
    /// nothing to scroll.
    ///
    /// A scroll with nothing to scroll draws no scrollbar at all, which is the
    /// same decision a container makes about a transparent background: a
    /// scrollbar on content that fits is telling the user about a scroll that
    /// does not exist.
    ///
    /// The thumb's length is the visible fraction of the content,
    /// `viewport / content`, times the viewport's own height; the rest of the
    /// groove is the run it travels over, and the offset's share of that run is
    /// how far down it sits. The fraction is bounded rather than trusted,
    /// because a caller that wrote [`scroll_offset`](Scroll::scroll_offset)
    /// directly is writing it with no clamp and a thumb must not leave its groove
    /// because of it.
    fn thumb_rect(&self, rect: Rect) -> Option<Rect> {
        let max = max_scroll(rect.height, self.content_height);
        if max <= 0.0 {
            return None;
        }
        let track = self.track_rect(rect);
        let thumb_height = self.thumb_length(rect);
        let run = self.thumb_run(rect);
        let fraction = bounded(self.scroll_offset.get() / max);
        Some(Rect::new(
            track.x,
            rect.y + run * fraction,
            track.width,
            thumb_height,
        ))
    }
    /// Returns how far this scroll's content can be scrolled in `rect`, in
    /// pixels.
    ///
    /// It is [`max_scroll`] of the viewport's height and the widget's own
    /// [`content_height`](Scroll::content_height): the number every bound on the
    /// offset is made from, and the number a caller needs to lay out a scrollbar
    /// of its own or to answer "is there anywhere to go".
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node::{self, WidgetNode};
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::scroll::Scroll;
    ///
    /// let mut nodes = Arena::new();
    /// let content = node::create(&mut nodes, LayoutState::new());
    /// let mut scroll = Scroll::new(&mut nodes, content);
    /// scroll.set_content_height(900.0);
    ///
    /// assert_eq!(scroll.max_scroll_for(Rect::new(0.0, 0.0, 200.0, 300.0)), 600.0);
    /// assert_eq!(scroll.max_scroll_for(Rect::new(0.0, 0.0, 200.0, 900.0)), 0.0);
    /// ```
    #[must_use]
    pub fn max_scroll_for(&self, rect: Rect) -> f32 {
        max_scroll(rect.height, self.content_height)
    }

    /// Handles `event` as this scroll would inside `rect`, and reports whether it
    /// consumed it.
    ///
    /// **A [`Drag`](InputEventKind::Drag) on a grabbed thumb moves the thumb
    /// rather than the content.** That is the one branch here that reads the
    /// event's *position*: a drag on the content is a scroll, and scrolls by its
    /// own delta, but a thumb travelling a run shorter than the viewport would
    /// then move a fraction of the distance the pointer did — which is the lag
    /// [`grab_thumb`](Scroll::grab_thumb) records the operator reporting. The
    /// grabbed branch consumes the drag whether or not it moved the offset, so a
    /// drag past the end of the document is still this scroll's.
    ///
    /// Any other [`Drag`](InputEventKind::Drag) moves the offset by the drag's own
    /// vertical delta, **not** negated: a finger travelling down moves the content
    /// down, which reveals what was above it, which is the offset going back
    /// towards zero. That is the sign convention this whole module is built on,
    /// and [`Scroll::content_rect`] is where it becomes visible.
    ///
    /// A [`Scroll`](InputEventKind::Scroll) — a wheel notch, and the same event a
    /// steering wheel's
    /// [`STEERING_WHEEL_SCROLL_AXIS`](crate::input::STEERING_WHEEL_SCROLL_AXIS)
    /// arrives as — moves it by `WHEEL_STEP`, up or down. Only the *sign* of
    /// the event's `y` is read, for the reason `WHEEL_STEP` gives: a notch and
    /// a gamepad axis share the field and agree on nothing else. `x` is the
    /// wheel and `y` the vertical one, so a horizontal wheel is not this widget's
    /// and is left for a horizontal scroller.
    ///
    /// An arrow key — or the gamepad's d-pad, which the input module already
    /// maps into [`Key`] — scrolls by `KEY_STEP_FRACTION` of the viewport, and
    /// is consumed **only while the scroll holds focus**. A key press is not
    /// routed by position, so this is called on the focused node by the caller,
    /// and every arrow would otherwise move every scroll on screen. Left and
    /// right are not keys a vertical scroll has: they carry on up the tree.
    ///
    /// Every write goes through [`clamp_scroll`], so an offset that would leave
    /// `0.0..=max_scroll` is pinned at the end it is past. A drag is consumed
    /// whether or not it moved anything, because the event was aimed at this
    /// scroll; a wheel notch and a key press are consumed on the same terms.
    /// A drag that reports no vertical component and is not on a grabbed thumb
    /// is the exception: there was nothing for this scroll to do with it, and
    /// consuming it would steal a horizontal scroll from whatever is above.
    ///
    /// **The offset is written here and the content node is not moved**, because
    /// an input handler is called from [`input::dispatch_event`](crate::input::dispatch_event) and reaching the
    /// arena from there is the double borrow
    /// [`input::route`](crate::input::route) documents. The frame's second step
    /// is [`apply_offset`](Scroll::apply_offset), which needs the arena.
    ///
    /// **There is no momentum.** Requirement 3 marks smooth scrolling with
    /// momentum optional and this task does not ask for it, so a released finger
    /// leaves the content exactly where the drag left it: nothing here has a
    /// velocity, and nothing keeps moving after the last event.
    ///
    /// Every other event is left alone and not consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind, Key};
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node::{self, WidgetNode};
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::scroll::Scroll;
    ///
    /// let mut nodes = Arena::new();
    /// let content = node::create(&mut nodes, LayoutState::new());
    /// let mut scroll = Scroll::new(&mut nodes, content);
    /// scroll.set_content_height(900.0);
    /// let rect = Rect::new(0.0, 0.0, 200.0, 300.0);
    ///
    /// // Arrows only reach a scroll that holds focus.
    /// let mut down = InputEvent::new(
    ///     InputEventKind::KeyDown {
    ///         key: Key::Keyboard(sdl3::keyboard::Keycode::Down),
    ///         keymod: sdl3::keyboard::Mod::empty(),
    ///     },
    ///     None,
    /// );
    /// assert!(!scroll.on_event(&mut down, rect), "an unfocused scroll ignores it");
    /// assert!(!down.consumed(), "and lets it travel on");
    ///
    /// scroll.focused.set(true);
    /// assert!(scroll.on_event(&mut down, rect));
    /// assert_eq!(scroll.scroll_offset.get(), 30.0, "a tenth of a 300 viewport");
    /// assert!(down.consumed());
    /// ```
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        match event.kind() {
            InputEventKind::Drag { delta } => {
                // A grabbed thumb wins over everything else, and it reads the
                // pointer's **position** rather than the drag's delta: the thumb
                // travels a shorter run than the content does, so a delta moves
                // it a fraction of the distance the cursor went. See
                // [`grab_thumb`](Scroll::grab_thumb) for the arithmetic and the
                // operator's report of what it looks like.
                if self.is_thumb_grabbed() {
                    // A drag with no position cannot be placed against the
                    // groove at all, and the delta path is the only mapping left,
                    // so it is the one that runs. The recogniser always gives a
                    // drag a position; this is a caller that did not.
                    if let Some(position) = event.position() {
                        if let Some(offset) = self.offset_under(position, rect) {
                            event.consume();
                            let _ = self.scroll_by(rect, offset - self.scroll_offset.get());
                            return true;
                        }
                    }
                }
                // A drag with no vertical component is not a vertical scroll, and
                // leaving it unconsumed lets a horizontal scroller above take it.
                if delta.y == 0.0 {
                    return false;
                }
                event.consume();
                // The direction is [`gesture_delta`]'s, and the settled offset is
                // in the property; the caller reads it there.
                let _ = self.scroll_by(rect, gesture_delta(delta));
                true
            }
            InputEventKind::Scroll { delta } => {
                if delta.y == 0.0 {
                    return false;
                }
                event.consume();
                // The direction is [`wheel_delta`]'s, which is
                // [`gesture_delta`]'s with the magnitude dropped — so a wheel and
                // a finger move this control the same way, and a steering wheel's
                // 32000 is one notch rather than a jump to the end.
                let _ = self.scroll_by(rect, wheel_delta(delta));
                true
            }
            InputEventKind::KeyDown { key, .. } => {
                if !self.focused.get() {
                    return false;
                }
                let Some(direction) = scroll_key(&key) else {
                    return false;
                };
                event.consume();
                let _ = self.scroll_by(rect, direction * rect.height * KEY_STEP_FRACTION);
                true
            }
            _ => false,
        }
    }

    /// Returns the draw commands that paint the scroll's scrollbar within `rect`.
    ///
    /// The commands are, in order: the groove, a rounded rectangle in the
    /// groove's own colour; the focus ring, if the scroll is focused and the
    /// ring is not zero wide, which is the **thumb's rect grown by the ring** and
    /// not the viewport's; and the thumb, in the thumb's colour.
    ///
    /// The order is the point of the middle one. A [`DrawCommand::RoundedRect`]
    /// *fills* its rect, so a ring drawn around the whole viewport and left
    /// uncovered is a card — 200 by 300 of it — with a scrollbar lying on top,
    /// which is the defect `.ai/NEVERAGAIN.md` records against the slider's focus
    /// ring. The ring here goes around the **thumb** and the thumb is recorded
    /// after it, so only its border shows. The ring is also what keeps a scroll
    /// identifiable at all: a scrollbar has no background of its own to draw over
    /// a ring with, exactly as a slider has none.
    ///
    /// A scroll with nothing to scroll records **nothing at all**, not a full
    /// length thumb: see `Scroll::thumb_rect`.
    ///
    /// The content is not painted here. It is a node of its own, the paint pass
    /// walks it after this node, and clipping it is the renderer's job — see
    /// [`Scroll::clip_rect`].
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let Some(thumb) = self.thumb_rect(rect) else {
            return Vec::new();
        };
        let mut painter = Painter::new();
        let track = self.track_rect(rect);
        painter.rounded_rect(track, self.radius(), self.track.get());
        let ring = self.focus_ring.get();
        if ring > 0.0 && self.focused.get() {
            painter.rounded_rect(grow(thumb, ring), self.radius() + ring, self.palette.ring);
        }
        painter.rounded_rect(thumb, self.radius(), self.thumb.get());
        painter.finish()
    }
}
/// Returns how far the content can be scrolled: `content_height` past
/// `viewport_height`, floored at zero.
///
/// This is the `max_scroll` of requirement 3, and it is the one number both this
/// widget and the list derive every bound from, so it is public and pure rather
/// than a private helper with a copy in the next module.
///
/// A content shorter than — or exactly as tall as — the viewport scrolls
/// nowhere, which is zero rather than a negative run: a negative maximum would
/// make every clamp in the module an interval with its ends the wrong way round.
/// A `NaN` content height gives zero as well, because [`f32::max`] returns its
/// non-`NaN` operand; a caller that has lost track of a measurement gets a
/// scroll that does not scroll rather than one whose every position is `NaN`.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::scroll::max_scroll;
///
/// assert_eq!(max_scroll(300.0, 900.0), 600.0, "600 past the viewport");
/// assert_eq!(max_scroll(300.0, 300.0), 0.0, "content that fits");
/// assert_eq!(max_scroll(300.0, 100.0), 0.0, "and content that does not");
/// assert_eq!(max_scroll(300.0, f32::NAN), 0.0, "and a caller that lost it");
/// ```
/// Returns the largest offset a viewport of `viewport_height` pixels can
/// scroll over content of `content_height` pixels: the content that will not
/// fit.
///
/// It is the whole of the clamp, and [`clamp_scroll`] is the offset clamped to
/// it.
#[must_use]
pub fn max_scroll(viewport_height: f32, content_height: f32) -> f32 {
    (content_height - viewport_height).max(0.0)
}

/// Returns the change a gesture of `delta` makes to the scroll offset, in pixels.
///
/// **This is the direction rule, in one place, and it is the only copy.** A
/// [`Drag`](InputEventKind::Drag)'s delta and a [`Scroll`](InputEventKind::Scroll)'s
/// delta are *different quantities that happen to share a field* — where the
/// pointer went, versus which way the document is being asked to move — so both
/// arms of [`Scroll::on_event`] call this rather than each carrying its own sign.
/// An earlier version wrote the sign out in both arms and restated it in
/// **eleven tests**, so changing the convention meant finding eleven places; a
/// mutation here is now caught by one test that names the convention.
///
/// **The convention, decided by the operator on 2026-09-30: down is later.** A
/// gesture whose `delta.y` is positive moves *down* the document — so a finger
/// travelling down the screen advances the list, and so does a wheel rolled
/// towards the user, which SDL reports as a negative `y` and which the wheel's
/// own arm therefore negates. The two agree, and that is the point: one control,
/// one direction, whichever way you drive it.
///
/// It was the other way round when this was written. Content-follows-the-finger
/// is the touch convention and is perfectly defensible; it was dropped because
/// the wheel had already been decided the other way, and a wheel and a finger
/// that disagree on one control is worse than either convention alone. The cost
/// is that a drag now feels like it is *pushing* the content rather than pulling
/// it, which is the trade this function's doc records.
///
/// # Examples
///
/// ```
/// use ui_core::layout::Offset;
/// use ui_core::widgets::scroll::gesture_delta;
///
/// // Down the screen advances down the document.
/// assert!(gesture_delta(Offset::new(0.0, 40.0)) > 0.0);
/// // And up the screen goes back up it.
/// assert!(gesture_delta(Offset::new(0.0, -40.0)) < 0.0);
/// // Horizontal movement is not a vertical scroll, and reads as no movement.
/// assert_eq!(gesture_delta(Offset::new(40.0, 0.0)), 0.0);
/// ```
#[must_use]
pub fn gesture_delta(delta: Offset) -> f32 {
    delta.y
}

/// Returns the change one wheel notch of `delta` makes to the offset.
///
/// The magnitude of a [`Scroll`](InputEventKind::Scroll) is **not** used: the
/// input module puts a wheel's notch of 1 and a steering wheel's axis of 32000
/// in the same field, and reading it would teleport the content to the end on
/// every frame of a full deflection. Only the direction is taken, from
/// [`gesture_delta`].
#[must_use]
pub fn wheel_delta(delta: Offset) -> f32 {
    // **A wheel is the one gesture that is negated first.** SDL's positive `y` is
    // "away from the user", which under the convention above is *up* the document,
    // so it must be turned around before the rule is applied. A drag needs no
    // negation: a finger's positive `y` really is travelling down the screen.
    //
    // That one negation is the whole reason the two arms do not share a sign, and
    // it is the line a reader should check first if the two ever disagree.
    if delta.y < 0.0 {
        WHEEL_STEP
    } else {
        -WHEEL_STEP
    }
}

/// Returns `offset` clamped to `0.0..=max_scroll(viewport_height, content_height)`.
///
/// This is the clamp of requirement 3, in one place, because a scroll and a list
/// have to agree about it: an offset that one of them lets out of range is an
/// item the other draws a viewport's height away from where it belongs.
///
/// Two comparisons rather than [`f32::clamp`], for the reason the slider's own
/// clamp helper gives: `clamp` panics when its bounds are the wrong way round,
/// and a widget whose caller has the wrong way up must not take a frame down to
/// say so. `max` then `min` is the same answer for an ordered pair and cannot
/// panic, and a `NaN` in either is passed over rather than propagated.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::scroll::clamp_scroll;
///
/// // 900 of content in a 300 viewport: 0 to 600 and nowhere else.
/// assert_eq!(clamp_scroll(200.0, 300.0, 900.0), 200.0);
/// assert_eq!(clamp_scroll(900.0, 300.0, 900.0), 600.0, "past the end");
/// assert_eq!(clamp_scroll(-40.0, 300.0, 900.0), 0.0, "above the top");
/// assert_eq!(clamp_scroll(100.0, 300.0, 200.0), 0.0, "nothing to scroll");
/// assert_eq!(clamp_scroll(f32::NAN, 300.0, 900.0), 0.0, "and a NaN is the top");
/// ```
#[must_use]
#[allow(clippy::manual_clamp)]
pub fn clamp_scroll(offset: f32, viewport_height: f32, content_height: f32) -> f32 {
    offset
        .max(0.0)
        .min(max_scroll(viewport_height, content_height))
}

/// Returns the band of the content that `viewport` shows at `scroll_offset`, in
/// the content's own coordinates.
///
/// The returned rect is `viewport`'s own width at its own `x` — a vertical scroll
/// does not move the content sideways — from `scroll_offset` down for as far as
/// the content and the viewport both reach. It is clamped first, so an offset
/// outside `0.0..=max_scroll` gives the band that offset would really show.
///
/// This is the function that decides what is on screen, and it is what a caller
/// asks before deciding whether to build an item's draw commands at all. Nothing
/// here measures the content's children: an item at `y` with height `h` is
/// visible when its band and this one intersect, and the caller knows the item's
/// band because the caller placed it.
///
/// # Examples
///
/// ```
/// use ui_core::paint::Rect;
/// use ui_core::widgets::scroll::visible_rect;
///
/// let viewport = Rect::new(664.0, 120.0, 200.0, 300.0);
/// let at_top = visible_rect(viewport, 900.0, 0.0);
/// assert_eq!(at_top, Rect::new(664.0, 0.0, 200.0, 300.0));
///
/// let halfway = visible_rect(viewport, 900.0, 450.0);
/// assert_eq!(halfway, Rect::new(664.0, 450.0, 200.0, 300.0));
///
/// // An offset past the end gives the last full viewport, not a negative band.
/// assert_eq!(
///     visible_rect(viewport, 900.0, 5000.0),
///     Rect::new(664.0, 600.0, 200.0, 300.0),
/// );
/// ```
#[must_use]
pub fn visible_rect(viewport: Rect, content_height: f32, scroll_offset: f32) -> Rect {
    let top = clamp_scroll(scroll_offset, viewport.height, content_height);
    let height = viewport
        .height
        .min((content_height - top).max(0.0))
        .max(0.0);
    Rect::new(viewport.x, top, viewport.width, height)
}

/// Returns the smallest rectangle containing `command`, or `None` when the
/// command's extent cannot be bounded from the command itself.
///
/// Only [`Text`](DrawCommand::Text) and an empty
/// [`Path`](DrawCommand::Path) are `None`. A text run carries the `x` and `y` of
/// its line's top-left and its font size, but **not its width** — that needs the
/// font's advance for every character in the run — so bounding one honestly would
/// mean inventing a width. A caller that wants text clipped measures the run and
/// clips it itself.
///
/// Everything else is bounded from what it draws: a circle by its centre and
/// radius, a line by its two ends and half its width, a path by the extent of
/// its points.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{Color, DrawCommand, Rect};
/// use ui_core::widgets::scroll::command_bounds;
///
/// let filled = DrawCommand::Rect { rect: Rect::new(4.0, 8.0, 10.0, 6.0), color: Color::new(0, 0, 0, 255) };
/// assert_eq!(command_bounds(&filled), Some(Rect::new(4.0, 8.0, 10.0, 6.0)));
///
/// let ring = DrawCommand::Circle { center: (100.0, 50.0), radius: 12.0, color: Color::new(0, 0, 0, 255) };
/// assert_eq!(command_bounds(&ring), Some(Rect::new(88.0, 38.0, 24.0, 24.0)));
/// ```
#[must_use]
pub fn command_bounds(command: &DrawCommand) -> Option<Rect> {
    match command {
        DrawCommand::Rect { rect, .. }
        | DrawCommand::RoundedRect { rect, .. }
        | DrawCommand::Image { rect, .. } => Some(*rect),
        DrawCommand::Line {
            start, end, width, ..
        } => {
            let half = (*width).max(0.0) / 2.0;
            Some(Rect::new(
                start.0.min(end.0) - half,
                start.1.min(end.1) - half,
                (start.0.max(end.0) - start.0.min(end.0)) + half * 2.0,
                (start.1.max(end.1) - start.1.min(end.1)) + half * 2.0,
            ))
        }
        DrawCommand::Circle { center, radius, .. } => {
            let radius = (*radius).max(0.0);
            Some(Rect::new(
                center.0 - radius,
                center.1 - radius,
                radius * 2.0,
                radius * 2.0,
            ))
        }
        DrawCommand::Path { points, width, .. } => {
            let half = (*width).max(0.0) / 2.0;
            let mut low = (f32::INFINITY, f32::INFINITY);
            let mut high = (f32::NEG_INFINITY, f32::NEG_INFINITY);
            for point in points {
                low.0 = low.0.min(point.0);
                low.1 = low.1.min(point.1);
                high.0 = high.0.max(point.0);
                high.1 = high.1.max(point.1);
            }
            // An empty path visits nothing and draws nothing; there is no place
            // to say it is, and inventing one would keep or drop it arbitrarily.
            if points.is_empty() {
                return None;
            }
            Some(Rect::new(
                low.0 - half,
                low.1 - half,
                (high.0 - low.0) + half * 2.0,
                (high.1 - low.1) + half * 2.0,
            ))
        }
        DrawCommand::Text { .. } => None,
    }
}

/// Returns the commands that are worth drawing inside `clip`, in the order they
/// were recorded.
///
/// Three outcomes, and the middle one is the deliberate choice:
///
/// - a command entirely inside `clip` passes through **unchanged**;
/// - a command entirely outside it is **dropped**, which is requirement 5's
///   "items outside the viewport are not drawn" and the only part of clipping
///   that costs the caller nothing;
/// - a command that **straddles** an edge is kept **whole**, because trimming
///   it is not clipping it. A [`DrawCommand::RoundedRect`] fills its rect, so a
///   rounded rectangle cut to part of itself is a smaller rounded rectangle with
///   its own corners — a different shape, not a clipped one. A
///   [`DrawCommand::Circle`] has no way to say "all but the bottom of this
///   disc" in a centre and a radius, and a [`DrawCommand::Text`] run does not
///   carry its width at all. Real clipping needs the scissor, and
///   [`Scroll::clip_rect`] is where the rect for it comes from; until that
///   scissor is applied, this function is the part of the job that can be done
///   without one, and doing the other half wrongly would look worse than not
///   doing it.
///
/// A command [`command_bounds`] cannot bound is kept, so a text run on screen is
/// never dropped for want of a measurement this layer has no way to take.
///
/// A command touching an edge counts as inside: the bounds are inclusive, so an
/// item whose last pixel row is the clip's last row is drawn.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{Color, DrawCommand, Rect};
/// use ui_core::widgets::scroll::clip_commands;
///
/// let inside = DrawCommand::Rect { rect: Rect::new(10.0, 10.0, 10.0, 10.0), color: Color::new(1, 1, 1, 255) };
/// let outside = DrawCommand::Rect { rect: Rect::new(400.0, 400.0, 10.0, 10.0), color: Color::new(2, 2, 2, 255) };
/// let straddling = DrawCommand::Rect { rect: Rect::new(0.0, 0.0, 60.0, 20.0), color: Color::new(3, 3, 3, 255) };
///
/// let kept = clip_commands(&[inside.clone(), outside, straddling.clone()], Rect::new(0.0, 0.0, 200.0, 200.0));
/// assert_eq!(kept, vec![inside, straddling], "and the straddler keeps its own size");
/// ```
#[must_use]
pub fn clip_commands(commands: &[DrawCommand], clip: Rect) -> Vec<DrawCommand> {
    commands
        .iter()
        .filter(|command| command_bounds(command).is_none_or(|bounds| intersects(bounds, clip)))
        .cloned()
        .collect()
}

/// Returns `value` in `0.0..=1.0`.
///
/// The two comparisons rather than [`f32::clamp`], for the reason
/// [`clamp_scroll`] gives.
#[allow(clippy::manual_clamp)]
fn bounded(value: f32) -> f32 {
    value.max(0.0).min(1.0)
}

/// Returns whether `a` and `b` share at least a point, edges included.
fn intersects(a: Rect, b: Rect) -> bool {
    a.x <= b.x + b.width && b.x <= a.x + a.width && a.y <= b.y + b.height && b.y <= a.y + a.height
}

/// Returns whether `rect` contains `point`, its edges included.
///
/// This is the module's own copy of the test rather than
/// [`crate::input::contains`], for the reason the button's and the slider's
/// helpers give: it is three comparisons, and a widget reaching into the input
/// module for them would make the input module's convention a dependency of a
/// widget that never routes an event. Inclusive edges are deliberate and are the
/// same convention [`List::item_at`](crate::widgets::list::List::item_at) follows:
/// a press on the thumb's last pixel row is a press on the thumb.
fn contains(rect: Rect, point: Offset) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
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

/// Returns the direction a scrolling key moves the offset in, if it is one.
///
/// A positive result is towards the end of the content and a negative one towards
/// its top, so `Up` is `-1.0`: up means "earlier", and earlier is a smaller
/// offset.
///
/// The d-pad is in the same match as the arrow keys because the input module
/// already maps a gamepad's buttons into [`Key`] and a scroll's keys are
/// whichever four the caller pressed. The d-pad is the half of "gamepad scrolls"
/// that needs no change to the input module; the analogue stick is the other
/// half and is not here, for the reason [`Scroll`]'s module docs give.
///
/// Left and right are deliberately absent. This widget scrolls vertically, and a
/// key it does not use is left alone so it carries on up the tree.
fn scroll_key(key: &Key) -> Option<f32> {
    use sdl3::gamepad::Button as Pad;
    match key {
        Key::Keyboard(sdl3::keyboard::Keycode::Up) | Key::Gamepad(Pad::DPadUp) => Some(-1.0),
        Key::Keyboard(sdl3::keyboard::Keycode::Down) | Key::Gamepad(Pad::DPadDown) => Some(1.0),
        _ => None,
    }
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback
/// for a token a caller has written the wrong variant into — and black rather
/// than a panic, because a mistyped theme token is not worth taking a frame down
/// for. It is the button's own helper, repeated rather than imported: it is five
/// lines, and a shared module for one five-line helper is a module.
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
    use crate::layout::{Layout, LayoutState as State};
    use crate::theme::ThemeToken;
    use std::cell::Cell;
    use std::rc::Rc;

    /// The viewport every geometry test scrolls: 200 by 300, which is wider than
    /// tall so that a top and a bottom are different numbers.
    const VIEWPORT: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 300.0,
    };

    /// The same viewport somewhere else in a window: 200 by 300 at (664, 120),
    /// which is where the demo's window has room for one. Every other fixture
    /// here starts at the origin, so this is the one that catches an origin being
    /// read as an extent.
    const OFFSET_VIEWPORT: Rect = Rect {
        x: 664.0,
        y: 120.0,
        width: 200.0,
        height: 300.0,
    };

    /// The content height every scrolling test uses: 900 in a 300 viewport, so
    /// 600 can be scrolled and the arithmetic divides.
    const CONTENT: f32 = 900.0;

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a
    /// colour at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// A scroll showing a content of `content_height`, with its colours already
    /// arrived at the dark theme's palette.
    ///
    /// Setting a palette does not move the scrollbar by itself — a theme switch
    /// is animated — so a test that wants a scrollbar *in* a palette asks for the
    /// animation and ticks it out. Otherwise the colours would be the ones
    /// `Scroll::new` starts with and the palette's would only be targets.
    fn scroll(content_height: f32) -> (Arena<WidgetNode>, Scroll) {
        let mut nodes = Arena::new();
        let content = node::create(&mut nodes, State::new());
        let mut scroll = Scroll::new(&mut nodes, content);
        scroll.set_palette(Palette::from_theme(&Theme::dark()));
        scroll.snap_to_state();
        scroll.set_content_height(content_height);
        (nodes, scroll)
    }

    /// A drag of `delta` ending at `(x, y)`, which is what a finger moving
    /// produces.
    fn drag(delta: Offset) -> InputEvent {
        InputEvent::new(
            InputEventKind::Drag { delta },
            Some(Offset::new(
                VIEWPORT.x + VIEWPORT.width / 2.0,
                VIEWPORT.y + VIEWPORT.height / 2.0,
            )),
        )
    }

    /// A wheel notch of `(x, y)`, which is also what a gamepad axis arrives as.
    fn wheel(x: f32, y: f32) -> InputEvent {
        InputEvent::new(
            InputEventKind::Scroll {
                delta: Offset::new(x, y),
            },
            Some(Offset::new(
                VIEWPORT.x + VIEWPORT.width / 2.0,
                VIEWPORT.y + VIEWPORT.height / 2.0,
            )),
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

    /// A filled rectangle in a colour of its own, for the clipping tests.
    fn filled_rect(x: f32, y: f32, width: f32, height: f32, shade: u8) -> DrawCommand {
        DrawCommand::Rect {
            rect: Rect::new(x, y, width, height),
            color: Color::new(shade, shade, shade, 255),
        }
    }

    /// A scroll hung on a laid-out panel at the origin, with its viewport and its
    /// content both sized, and returns the panel and the viewport's rect.
    ///
    /// The scroll's node is given the viewport's size explicitly rather than
    /// leaving `Scroll::new`'s default, because a viewport sized to its own
    /// content is a viewport that does not scroll — and the test that checks the
    /// clamping would then pass for the wrong reason.
    fn on_a_panel(
        scroll: &Scroll,
        nodes: &mut Arena<WidgetNode>,
        content_size: Size,
        viewport: Size,
    ) -> (Handle, Rect) {
        let panel = node::create(nodes, State::new());
        assert!(
            node::attach(nodes, panel, scroll.handle()),
            "the scroll is hung on the panel"
        );
        if let Some(node) = nodes.get_mut(scroll.handle()) {
            node.layout_mut()
                .set_constraints(Constraints::tight(viewport));
        }
        if let Some(node) = nodes.get_mut(scroll.content) {
            node.layout_mut()
                .set_constraints(Constraints::tight(content_size));
        }
        let panel_size = viewport;
        Layout::new(nodes).layout(panel, Constraints::tight(panel_size));
        let rect = nodes
            .get(scroll.handle())
            .map(WidgetNode::layout)
            .and_then(State::rect)
            .map(Rect::from)
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        (panel, rect)
    }

    /// Asserts two values are within a millionth of one another.
    ///
    /// The thumb's length is `viewport² / content`, so a number that divides
    /// exactly is asserted with `assert_eq!` and everything else is asserted
    /// with this: `300 * 300 / 700` is not `128.57143`, and a test that wrote
    /// the decimal out would be asserting the compiler's rounding.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-4, "{what}: {got} against {want}");
    }

    // ---------------------------------------------------------------- geometry

    #[test]
    fn a_scroll_holds_the_properties_the_task_gives_it() {
        let (_nodes, scroll) = scroll(CONTENT);
        assert_eq!(scroll.content_height(), CONTENT);
        assert_eq!(
            scroll.scroll_offset.get(),
            0.0,
            "a scroll starts at the top"
        );
        assert!(!scroll.focused.get());
        assert_eq!(scroll.focus_ring.get(), FOCUS_RING);
        assert_eq!(
            scroll.track.get(),
            Palette::from_theme(&Theme::dark()).track,
            "already in its palette — the helper snaps, and the neutral defaults are \
             what an unsnapped scrollbar paints"
        );
        assert_eq!(scroll.size(), Size::new(240.0, 320.0));
    }

    #[test]
    fn a_scroll_attaches_its_content_to_its_own_node() {
        let (nodes, scroll) = scroll(CONTENT);
        assert!(
            scroll.is_attached(&nodes),
            "the paint pass only walks a node that is in the tree"
        );
        assert_eq!(
            nodes.get(scroll.content).unwrap().parent(),
            Some(scroll.handle())
        );
    }

    #[test]
    fn a_scroll_whose_content_is_already_attached_reports_that_it_is_not_attached() {
        // The constructor attaches, so a caller who attached first gets the
        // second attach refused — and the only way to see that is to ask.
        let mut nodes = Arena::new();
        let elsewhere = node::create(&mut nodes, State::new());
        let content = node::create(&mut nodes, State::new());
        assert!(
            node::attach(&mut nodes, elsewhere, content),
            "the content belongs to something else already"
        );

        let scroll = Scroll::new(&mut nodes, content);
        assert!(
            !scroll.is_attached(&nodes),
            "a node has one parent, and the second attach was refused"
        );
    }

    #[test]
    fn a_new_scroll_gives_its_node_a_viewport_so_the_pass_does_not_measure_one() {
        let (nodes, scroll) = scroll(CONTENT);
        let node = nodes.get(scroll.handle()).expect("a node of its own");
        assert_eq!(
            node.layout().constraints(),
            Constraints::tight(scroll.size()),
            "otherwise the pass sizes the viewport to fit its own content"
        );
        assert_eq!(
            node.layout().mode(),
            LayoutMode::Absolute,
            "the content's position is the offset, and only this mode reads one"
        );
    }

    #[test]
    fn a_scroll_that_has_never_been_told_its_contents_height_cannot_scroll() {
        let mut nodes = Arena::new();
        let content = node::create(&mut nodes, State::new());
        let scroll = Scroll::new(&mut nodes, content);
        assert_eq!(scroll.content_height(), 0.0, "nothing has measured it");

        let mut wheel = wheel(0.0, 1.0);
        assert!(
            scroll.on_event(&mut wheel, VIEWPORT),
            "the wheel was still its"
        );
        assert_eq!(
            scroll.scroll_offset.get(),
            0.0,
            "and there is no geometry to scroll through, so nothing moved"
        );
        assert!(
            scroll.paint(VIEWPORT).is_empty(),
            "and no scrollbar, because a scrollbar would lie about a scroll"
        );
    }

    #[test]
    fn max_scroll_is_the_content_past_the_viewport() {
        assert_eq!(max_scroll(300.0, 900.0), 600.0);
        assert_eq!(max_scroll(300.0, 400.0), 100.0);
        assert_eq!(max_scroll(300.0, 300.0), 0.0, "content that exactly fits");
        assert_eq!(max_scroll(300.0, 120.0), 0.0, "and content that does not");
    }

    #[test]
    fn max_scroll_and_clamp_scroll_survive_a_measurement_the_caller_has_lost() {
        // `f32::max` returns its non-NaN operand, so a NaN content gives a
        // scroll that does not scroll rather than one whose every position is
        // NaN — which is what a plain subtraction followed by a clamp would give.
        assert_eq!(max_scroll(300.0, f32::NAN), 0.0);
        assert_eq!(clamp_scroll(100.0, 300.0, f32::NAN), 0.0);
        assert_eq!(clamp_scroll(f32::NAN, 300.0, 900.0), 0.0);
    }

    #[test]
    fn clamp_scroll_holds_an_offset_inside_the_scrollable_range() {
        assert_eq!(clamp_scroll(0.0, 300.0, 900.0), 0.0);
        assert_eq!(clamp_scroll(599.0, 300.0, 900.0), 599.0);
        assert_eq!(
            clamp_scroll(600.0, 300.0, 900.0),
            600.0,
            "the end is reachable"
        );
        assert_eq!(clamp_scroll(601.0, 300.0, 900.0), 600.0, "and no further");
        assert_eq!(clamp_scroll(-1.0, 300.0, 900.0), 0.0);
        assert_eq!(clamp_scroll(50.0, 300.0, 200.0), 0.0, "nothing to scroll");
    }

    #[test]
    fn visible_rect_is_the_band_of_content_the_viewport_shows() {
        let at_top = visible_rect(VIEWPORT, CONTENT, 0.0);
        assert_eq!(at_top, Rect::new(0.0, 0.0, 200.0, 300.0));

        let halfway = visible_rect(VIEWPORT, CONTENT, 300.0);
        assert_eq!(halfway, Rect::new(0.0, 300.0, 200.0, 300.0));

        let at_end = visible_rect(VIEWPORT, CONTENT, 600.0);
        assert_eq!(
            at_end,
            Rect::new(0.0, 600.0, 200.0, 300.0),
            "the last viewport of the content, and 600 + 300 is the content's end"
        );
    }

    #[test]
    fn visible_rect_stops_at_the_end_of_the_content() {
        // A content barely taller than the viewport: the band cannot be a full
        // viewport, because there is not that much content left under the offset.
        let band = visible_rect(VIEWPORT, 320.0, 20.0);
        assert_eq!(band, Rect::new(0.0, 20.0, 200.0, 300.0));
        assert!(
            band.y + band.height <= 320.0,
            "and it does not run off the end of the content"
        );

        let empty = visible_rect(VIEWPORT, 0.0, 0.0);
        assert_eq!(empty.height, 0.0, "a content with no height shows nothing");
    }

    #[test]
    fn visible_rect_follows_a_viewport_that_is_not_at_the_origin() {
        // A rect's origin and a rect's extent are different numbers, and only a
        // viewport somewhere else on the window tells them apart.
        assert_eq!(
            OFFSET_VIEWPORT.x, 664.0,
            "the fixture really is off the origin"
        );
        let band = visible_rect(OFFSET_VIEWPORT, CONTENT, 450.0);
        assert_eq!(band, Rect::new(664.0, 450.0, 200.0, 300.0));
    }

    // ------------------------------------------------------------------ events

    /// **Renamed and its sign reversed, 2026-09-30** — the convention is now *down
    /// is later*, see [`gesture_delta`]. It used to say the content moved *down*
    /// with the finger, which was the touch convention and is now gone.
    ///
    /// What it still checks, and why it was worth keeping through the reversal: the
    /// offset, the content's top edge in window coordinates, and the content node
    /// in the tree all move **together**. A sign applied to one and not the others
    /// passes a test that only looks at the offset, and this one looks at all
    /// three.
    #[test]
    fn a_drag_moves_the_offset_the_content_rect_and_the_node_together() {
        let (mut nodes, scroll) = scroll(CONTENT);
        scroll.scroll_offset.set(300.0);

        let mut down = drag(Offset::new(0.0, 40.0));
        assert!(scroll.on_event(&mut down, VIEWPORT), "a drag is consumed");
        assert_eq!(
            scroll.scroll_offset.get(),
            340.0,
            "a drag down the screen raises the offset, because down is later"
        );
        assert_eq!(
            scroll.content_rect(VIEWPORT).y,
            -340.0,
            "and the content's top edge is that much above the viewport's"
        );

        assert!(scroll.apply_offset(&mut nodes), "the tree moves with it");
        assert_eq!(
            nodes.get(scroll.content).unwrap().layout().position(),
            Some(Offset::new(0.0, -340.0)),
            "and the content node sits 340 above its viewport"
        );
    }

    /// **The convention itself, as one fact about numbers this test chooses.**
    ///
    /// Everything else in this module derives its expectations from the
    /// arithmetic — a drag moves the offset by its own distance, a wheel by one
    /// notch — which is *direction-agnostic*. This is the one test that says
    /// which way is which, so that the other eleven do not each have to restate
    /// it. It exists because they used to, and inverting the direction meant
    /// finding all eleven.
    #[test]
    fn down_is_later_and_a_wheel_agrees_with_a_finger() {
        let (_nodes, scroll) = scroll(CONTENT);
        assert_eq!(scroll.scroll_offset.get(), 0.0, "and it starts at the top");

        // A finger travelling **down** the screen, by 40 pixels.
        let mut finger = InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(0.0, 40.0),
            },
            Some(Offset::new(100.0, 150.0)),
        );
        assert!(scroll.on_event(&mut finger, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            40.0,
            "down the screen is down the document, by exactly how far the finger \
             went"
        );

        // A wheel rolled **towards** the user, which SDL reports as negative.
        let mut notch = wheel(0.0, -1.0);
        assert!(scroll.on_event(&mut notch, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            40.0 + WHEEL_STEP,
            "and that notch goes the same way — one step, not the wheel's 1"
        );

        // The same gesture the other way, from both.
        let mut finger_up = InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(0.0, -40.0),
            },
            Some(Offset::new(100.0, 150.0)),
        );
        assert!(scroll.on_event(&mut finger_up, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            WHEEL_STEP,
            "back up by its distance"
        );
        let mut notch_away = wheel(0.0, 1.0);
        assert!(scroll.on_event(&mut notch_away, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            0.0,
            "and the wheel brings it home"
        );
    }

    /// **Which drag does nothing at the top changed, 2026-09-30.** Under *down is
    /// later* it is a drag **upwards** that cannot move a list already at its
    /// start, because the offset has nowhere to go below zero. This used to be
    /// `a_drag_down_at_the_top_reveals_nothing_and_moves_nothing` and said the
    /// opposite.
    ///
    /// It is kept rather than dropped because it is the same fact the operator
    /// reported twice from the other end: **at the top of a list, one direction
    /// cannot work, and which one is a decision rather than a law.** The decision
    /// has since been made so that the direction a reader tries first is the one
    /// that works, and this test is what says the other one is clamped.
    #[test]
    fn a_drag_upwards_at_the_top_reveals_nothing_and_moves_nothing() {
        let (_nodes, scroll) = scroll(CONTENT);
        assert_eq!(scroll.scroll_offset.get(), 0.0, "and it starts at the top");

        let mut up = drag(Offset::new(0.0, -200.0));
        assert!(
            scroll.on_event(&mut up, VIEWPORT),
            "and it is still consumed: a clamped scroll is not an unhandled one"
        );
        assert_eq!(scroll.scroll_offset.get(), 0.0, "but nothing moved");

        let mut down = drag(Offset::new(0.0, 40.0));
        assert!(scroll.on_event(&mut down, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            40.0,
            "and the other way is the one that works"
        );
    }
    /// **Renamed and its sign reversed, 2026-09-30**, when the operator decided
    /// *down is later* — see [`gesture_delta`]. This used to be
    /// `a_drag_up_scrolls_towards_the_end`, because the content used to follow
    /// the finger. A test whose *name* states the convention has to be rewritten
    /// rather than flipped, or the name and the body drift apart and the name is
    /// what a reader trusts.
    #[test]
    fn a_drag_down_scrolls_towards_the_end() {
        let (_nodes, scroll) = scroll(CONTENT);
        let mut down = drag(Offset::new(0.0, 120.0));
        assert!(scroll.on_event(&mut down, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            120.0,
            "by exactly how far it went"
        );
    }

    #[test]
    fn a_drag_is_clamped_at_both_ends() {
        let (_nodes, scroll) = scroll(CONTENT);
        for _ in 0..10 {
            let mut up = drag(Offset::new(0.0, 200.0));
            assert!(
                scroll.on_event(&mut up, VIEWPORT),
                "and it is still consumed"
            );
        }
        assert_eq!(
            scroll.scroll_offset.get(),
            600.0,
            "900 - 300, and no further"
        );

        for _ in 0..10 {
            let mut down = drag(Offset::new(0.0, -200.0));
            assert!(scroll.on_event(&mut down, VIEWPORT));
        }
        assert_eq!(
            scroll.scroll_offset.get(),
            0.0,
            "and back to the top, no further"
        );
    }

    /// A drag of `delta` that **ends** at `position`.
    ///
    /// It is what [`drag`] is for the other half of these tests: a dragged thumb
    /// is placed by where the pointer *is*, so the tests that pin it need to say
    /// where it is rather than only how far it moved.
    fn drag_to(delta: Offset, position: Offset) -> InputEvent {
        InputEvent::new(InputEventKind::Drag { delta }, Some(position))
    }

    #[test]
    fn a_drag_that_is_entirely_horizontal_is_left_for_a_horizontal_scroller() {
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.scroll_offset.set(100.0);
        let mut sideways = drag(Offset::new(50.0, 0.0));
        assert!(
            !scroll.on_event(&mut sideways, VIEWPORT),
            "this widget scrolls vertically and the event has no vertical part"
        );
        assert!(!sideways.consumed(), "so it carries on to the tree");
        assert_eq!(scroll.scroll_offset.get(), 100.0, "and nothing moved");
    }

    // ------------------------------------------------------- dragging the thumb

    /// The numbers every thumb-drag test is arithmetic over, written out once.
    ///
    /// A 300-tall viewport over 900 of content: the thumb is a third of the
    /// groove, so it is 100 long and travels the remaining **200**, while the
    /// offset has **600** to cover. The ratio between those two is the defect —
    /// a drag that moved the offset by its own delta moved the thumb a **third**
    /// of the distance the cursor went — so every test below names the ratio
    /// rather than a number chosen to pass.
    const THUMB: f32 = 100.0;
    const RUN: f32 = 200.0;
    const MAX: f32 = 600.0;

    /// Returns the centre of the scrollbar's thumb at the current offset.
    fn thumb_centre(scroll: &Scroll, rect: Rect) -> f32 {
        let thumb = rounded(&scroll.paint(rect))[1].0;
        thumb.y + thumb.height / 2.0
    }

    /// Returns a press that lands `from_thumb_top` pixels down the thumb, which
    /// is the only thing a press can say that matters: *where in the thumb* the
    /// pointer went down.
    fn grab_thumb(scroll: &Scroll, from_thumb_top: f32, rect: Rect) -> Offset {
        let thumb = rounded(&scroll.paint(rect))[1].0;
        let position = Offset::new(thumb.x + thumb.width / 2.0, thumb.y + from_thumb_top);
        assert!(
            scroll.grab_thumb(position, rect),
            "the press at {position:?} is on the thumb, so it grabs"
        );
        position
    }

    #[test]
    fn a_drag_on_a_grabbed_thumb_moves_the_thumb_and_not_the_cursor_delta() {
        let (_nodes, scroll) = scroll(CONTENT);
        let pressed = grab_thumb(&scroll, THUMB / 2.0, VIEWPORT);
        let before = thumb_centre(&scroll, VIEWPORT);

        // A drag of 40 down, ending 40 below where it was — the whole point is
        // that the position, not the delta, is what places the thumb.
        let mut moved = drag_to(
            Offset::new(0.0, 40.0),
            Offset::new(pressed.x, pressed.y + 40.0),
        );
        assert!(scroll.on_event(&mut moved, VIEWPORT));
        assert!(moved.consumed());

        let after = thumb_centre(&scroll, VIEWPORT);
        assert!(
            (after - before - 40.0).abs() < 0.01,
            "the thumb followed the cursor exactly: it moved {} and the cursor 40",
            after - before
        );
        // And the offset is *not* the delta, which is what the old code wrote.
        // 40 along a run of 200 is a fifth, and a fifth of 600 is 120: the
        // delta path would have answered 40 and moved the thumb 13.3.
        assert_eq!(
            scroll.scroll_offset.get(),
            120.0,
            "a fifth of the way along the thumb's run, and not the drag's own 40"
        );
    }

    #[test]
    fn the_thumb_stays_exactly_where_it_was_grabbed_under_the_pointer() {
        let (_nodes, scroll) = scroll(CONTENT);
        // Grabbed a quarter of the way down a 100-tall thumb, not its middle: a
        // test that grabs the middle cannot tell a grab that was recorded from
        // one that was not, because the middle is the one offset that is
        // symmetric.
        let pressed = grab_thumb(&scroll, THUMB / 4.0, VIEWPORT);

        for step in 1..=4 {
            let y = pressed.y + step as f32 * 30.0;
            let mut moved = drag_to(Offset::new(0.0, 30.0), Offset::new(pressed.x, y));
            assert!(scroll.on_event(&mut moved, VIEWPORT));
            let thumb = rounded(&scroll.paint(VIEWPORT))[1].0;
            assert!(
                (thumb.y + THUMB / 4.0 - y).abs() < 0.01,
                "after {step} the pointer is 25px into the thumb, and the pointer \
                 is at {y} while the thumb's own quarter is at {}",
                thumb.y + THUMB / 4.0
            );
        }
    }

    #[test]
    fn a_drag_on_a_grabbed_thumb_travels_the_whole_document_without_a_jump() {
        let (_nodes, scroll) = scroll(CONTENT);
        let pressed = grab_thumb(&scroll, THUMB / 2.0, VIEWPORT);

        // The end of the groove is RUN below the thumb's start, so the pointer
        // there is the end of the document — and it is the *position*, so the
        // answer has to be the maximum rather than a pixel short of it.
        let mut to_end = drag_to(
            Offset::new(0.0, RUN),
            Offset::new(pressed.x, pressed.y + RUN),
        );
        assert!(scroll.on_event(&mut to_end, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            MAX,
            "the thumb at the bottom of its groove is the end of the document"
        );

        // And back to the top, which is the same arithmetic with a negative.
        let mut to_top = drag_to(Offset::new(0.0, -RUN), Offset::new(pressed.x, pressed.y));
        assert!(scroll.on_event(&mut to_top, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0);
    }

    #[test]
    fn a_drag_past_the_end_of_the_groove_stops_at_the_end_of_the_document() {
        let (_nodes, scroll) = scroll(CONTENT);
        let pressed = grab_thumb(&scroll, THUMB / 2.0, VIEWPORT);

        // A finger that has run off the bottom of the scrollbar and keeps going:
        // the offset is pinned, and the event is still consumed, because it was
        // aimed at this scroll.
        let mut past = drag_to(
            Offset::new(0.0, 10_000.0),
            Offset::new(pressed.x, VIEWPORT.y + VIEWPORT.height + 10_000.0),
        );
        assert!(scroll.on_event(&mut past, VIEWPORT), "and consumed");
        assert_eq!(scroll.scroll_offset.get(), MAX, "pinned at the end");

        let mut above = drag_to(
            Offset::new(0.0, -10_000.0),
            Offset::new(pressed.x, VIEWPORT.y - 10_000.0),
        );
        assert!(scroll.on_event(&mut above, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0, "and at the top");
    }

    #[test]
    fn a_drag_on_the_content_still_scrolls_by_its_own_delta() {
        let (_nodes, scroll) = scroll(CONTENT);
        // No press, so no grab: this is the behaviour that has to survive the
        // thumb, and the one that made the thumb look slow.
        let mut moved = drag_to(
            Offset::new(0.0, 40.0),
            Offset::new(VIEWPORT.x + 100.0, VIEWPORT.y + 150.0),
        );
        assert!(scroll.on_event(&mut moved, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            40.0,
            "a drag over a row is a scroll, and it is the content that follows the \
             finger"
        );
    }

    #[test]
    fn a_release_gives_the_thumb_up_and_the_content_takes_the_next_drag_back() {
        let (_nodes, scroll) = scroll(CONTENT);
        let pressed = grab_thumb(&scroll, THUMB / 2.0, VIEWPORT);
        assert!(scroll.is_thumb_grabbed());

        assert!(scroll.release_thumb(), "and there was one to give up");
        assert!(!scroll.is_thumb_grabbed());
        assert!(
            !scroll.release_thumb(),
            "and a second release has nothing to do"
        );

        let mut moved = drag_to(
            Offset::new(0.0, 40.0),
            Offset::new(pressed.x, pressed.y + 40.0),
        );
        assert!(scroll.on_event(&mut moved, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            40.0,
            "the content follows the finger again, so a grab that outlived its \
             finger would be a scrollbar the user cannot get rid of"
        );
    }

    #[test]
    fn a_press_that_missed_the_thumb_grabs_nothing() {
        let (_nodes, scroll) = scroll(CONTENT);

        // The content, which is where every press on a list lands.
        assert!(
            !scroll.grab_thumb(Offset::new(VIEWPORT.x + 100.0, VIEWPORT.y + 10.0), VIEWPORT),
            "a press on a row is not a scrollbar gesture"
        );
        assert!(!scroll.is_thumb_grabbed());

        // The groove *below* the thumb: still the scrollbar, but the press missed
        // the thing that moves, and grabbing it would make the thumb jump down to
        // the pointer on the first frame of the drag.
        let groove = rounded(&scroll.paint(VIEWPORT))[0].0;
        let missed = Offset::new(
            groove.x + groove.width / 2.0,
            VIEWPORT.y + VIEWPORT.height - 10.0,
        );
        assert!(
            !scroll.grab_thumb(missed, VIEWPORT),
            "a press on the empty groove is not a grab"
        );

        // And the outside of the viewport entirely, which is the demo's own
        // `list_at` gate rather than the widget's.
        assert!(!scroll.grab_thumb(Offset::new(-10.0, -10.0), VIEWPORT));
    }

    #[test]
    fn a_press_on_the_thumb_of_a_scroll_with_nothing_to_scroll_grabs_nothing() {
        let (_nodes, scroll) = scroll(100.0);
        // A 100-tall content in a 300 viewport draws no scrollbar at all, so
        // there is no thumb to grab — and the arithmetic below would divide by a
        // run of nothing if it let one.
        assert_eq!(scroll.scrollbar_rect(VIEWPORT), None);
        assert!(!scroll.grab_thumb(Offset::new(VIEWPORT.x + 197.0, 10.0), VIEWPORT));
        assert!(!scroll.is_thumb_grabbed());
    }

    #[test]
    fn a_thumb_too_long_to_travel_grabs_nothing_rather_than_dividing_by_no_run() {
        // A viewport a few pixels shorter than its content: the thumb is floored
        // at SCROLLBAR_MIN_THUMB, which is longer than the whole groove, so the
        // run is zero. `grab_thumb` accepts the press, and the drag has to answer
        // with something other than `NaN`.
        let mut nodes = Arena::new();
        let content = node::create(&mut nodes, State::new());
        let mut scroll = Scroll::new(&mut nodes, content);
        scroll.set_content_height(306.0);
        let rect = Rect::new(0.0, 0.0, 200.0, 6.0);
        assert!(
            scroll.grab_thumb(Offset::new(197.0, 3.0), rect),
            "it is pressed"
        );

        let mut moved = drag_to(Offset::new(0.0, 20.0), Offset::new(197.0, 20.0));
        // No run means no mapping, so the drag falls through to the content's own
        // scroll rather than writing `NaN` into the offset.
        assert!(scroll.on_event(&mut moved, rect));
        assert!(
            scroll.scroll_offset.get().is_finite(),
            "the offset is {} and not a NaN from a division by a zero run",
            scroll.scroll_offset.get()
        );
    }

    #[test]
    fn a_drag_on_a_grabbed_thumb_with_no_position_falls_back_to_the_content_scroll() {
        let (_nodes, scroll) = scroll(CONTENT);
        let _pressed = grab_thumb(&scroll, THUMB / 2.0, VIEWPORT);
        // The recogniser always gives a drag a position; this is the caller that
        // did not, and the only mapping left is the content's own.
        let mut blind = InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(0.0, 40.0),
            },
            None,
        );
        assert!(scroll.on_event(&mut blind, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 40.0);
    }

    #[test]
    fn a_grabbed_thumb_is_reachable_at_the_whole_width_the_scrollbar_draws() {
        let (_nodes, mut scroll) = scroll(CONTENT);
        scroll.set_thickness(12.0);
        let groove = scroll
            .scrollbar_rect(VIEWPORT)
            .expect("a scrollbar is drawn");
        assert_eq!(groove.width, 12.0);

        // Every column of the drawn bar is pressable, and one column to its left
        // is not — the width is the *drawn* width, not the old hairline, because
        // a finger aims at what it can see.
        for step in 0..12 {
            let x = groove.x + step as f32;
            let thumb = rounded(&scroll.paint(VIEWPORT))[1].0;
            let on_bar = Offset::new(x, thumb.y);
            assert!(
                scroll.grab_thumb(on_bar, VIEWPORT),
                "column {step} at x={x} is inside the bar that is drawn"
            );
            assert!(scroll.release_thumb());
        }
        assert!(
            !scroll.grab_thumb(
                Offset::new(groove.x - 0.5, thumb_top(&scroll, VIEWPORT)),
                VIEWPORT
            ),
            "and half a pixel left of it is the content, not the scrollbar"
        );
    }

    /// The thumb's top at the current offset.
    fn thumb_top(scroll: &Scroll, rect: Rect) -> f32 {
        rounded(&scroll.paint(rect))[1].0.y
    }

    #[test]
    fn widening_the_scrollbar_widens_the_groove_the_thumb_and_their_radii() {
        let (_nodes, mut scroll) = scroll(CONTENT);
        assert_eq!(
            scroll.thickness(),
            SCROLLBAR_THICKNESS,
            "the widget's baseline"
        );

        scroll.set_thickness(12.0);
        let painted = rounded(&scroll.paint(VIEWPORT));
        assert_eq!(painted.len(), 2);
        assert_eq!(
            painted[0].0,
            Rect::new(
                VIEWPORT.x + VIEWPORT.width - SCROLLBAR_MARGIN - 12.0,
                VIEWPORT.y,
                12.0,
                VIEWPORT.height,
            ),
            "the groove is 12 wide and still inset from the right edge"
        );
        assert_eq!(painted[0].1, 6.0, "with half its thickness as radius");
        assert_eq!(painted[1].0.width, 12.0, "and the thumb is 12 wide too");
        assert_eq!(painted[1].1, 6.0, "with the same radius");
    }

    #[test]
    fn a_thickness_is_a_length_and_cannot_be_negative() {
        let (_nodes, mut scroll) = scroll(CONTENT);
        assert_eq!(scroll.set_thickness(-6.0), 0.0);
        assert_eq!(scroll.thickness(), 0.0);
        // A zero-width bar is degenerate but not inverted: the groove's left edge
        // is its right edge, not past it.
        let groove = scroll.scrollbar_rect(VIEWPORT).expect("still drawn");
        assert_eq!(
            groove.x + groove.width,
            VIEWPORT.x + VIEWPORT.width - SCROLLBAR_MARGIN
        );
    }

    /// **The wheel's sign changed on 2026-09-30, by the operator's decision.** A
    /// wheel now uses the **scrollbar convention**: SDL reports the wheel rolling
    /// *towards* the user as a negative `y`, and that is the notch that advances
    /// *down* the document. Before this, the positive notch was the one that
    /// moved the offset up — the touch convention, where the content follows the
    /// gesture. This test was written against that and is rewritten rather than
    /// flipped, because "and the other way goes back" is the half that matters.
    #[test]
    fn a_wheel_down_scrolls_towards_the_end_one_notch() {
        let (_nodes, scroll) = scroll(CONTENT);
        // `dy = -1.0` is SDL's "rolled towards the user", and it goes down.
        let mut down = wheel(0.0, -1.0);
        assert!(scroll.on_event(&mut down, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), WHEEL_STEP);
        assert!(down.consumed());

        let mut up = wheel(0.0, 1.0);
        assert!(scroll.on_event(&mut up, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0, "and one back to the top");
    }

    /// **Rewritten 2026-09-30.** This test used to assert that a wheel notch and a
    /// drag travel *opposite* ways, which was true for one day: the wheel was
    /// inverted to the scrollbar convention and the drag still followed the
    /// finger, so one control answered two directions. The operator then had the
    /// drag inverted too, and the two agree — which is the thing worth pinning.
    ///
    /// The sharp form of that is **the same magnitude in the same field**: a 48
    /// pixel wheel step and a 48 pixel drag now move the offset identically, and
    /// `gesture_delta`'s doctest is where the rule itself lives.
    #[test]
    fn a_wheel_notch_and_a_drag_of_the_same_size_move_a_scroll_alike() {
        // Down the document, by the wheel and then by the finger, each from a
        // fresh scroll so neither starts where the other finished.
        let (_wheel_nodes, by_wheel) = scroll(CONTENT);
        let mut notch = wheel(0.0, -1.0);
        assert!(by_wheel.on_event(&mut notch, VIEWPORT));

        let (_finger_nodes, by_finger) = scroll(CONTENT);
        let mut finger = InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(0.0, WHEEL_STEP),
            },
            Some(Offset::new(100.0, 150.0)),
        );
        assert!(by_finger.on_event(&mut finger, VIEWPORT));

        assert_eq!(
            by_wheel.scroll_offset.get(),
            by_finger.scroll_offset.get(),
            "one notch and a drag of the same size land in the same place, because \
             one control answers one direction whichever way you drive it"
        );
        assert_eq!(
            by_wheel.scroll_offset.get(),
            WHEEL_STEP,
            "which is one notch, not the wheel's own 1"
        );
    }
    #[test]
    fn a_wheels_size_is_not_used_only_its_direction() {
        // A wheel notch is 1 and a steering wheel's axis is 32000, and both
        // arrive in the same field. Reading the magnitude would teleport the
        // content to the end on every frame of a full deflection.
        let (_nodes, scroll) = scroll(CONTENT);
        // Negative, like a real notch under the convention above — the point of
        // this test is the magnitude, not the direction.
        let mut axle = wheel(0.0, -32000.0);
        assert!(scroll.on_event(&mut axle, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            WHEEL_STEP,
            "one notch, whatever the control said"
        );
        assert_eq!(WHEEL_STEP, 48.0, "and the step is the number it documents");
    }

    #[test]
    fn a_horizontal_wheel_is_not_a_vertical_scroll() {
        let (_nodes, scroll) = scroll(CONTENT);
        let mut across = wheel(1.0, 0.0);
        assert!(!scroll.on_event(&mut across, VIEWPORT));
        assert!(!across.consumed());

        let mut neither = wheel(0.0, 0.0);
        assert!(
            !scroll.on_event(&mut neither, VIEWPORT),
            "a scroll of exactly zero is nobody's"
        );
        assert_eq!(scroll.scroll_offset.get(), 0.0);
    }

    #[test]
    fn an_arrow_key_scrolls_a_focused_scroll() {
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.focused.set(true);

        let mut down = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Down));
        assert!(scroll.on_event(&mut down, VIEWPORT));
        assert_eq!(
            scroll.scroll_offset.get(),
            30.0,
            "a tenth of a 300 viewport, whatever the viewport"
        );
        assert!(down.consumed());

        let mut up = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Up));
        assert!(scroll.on_event(&mut up, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0, "and back to the top");
    }

    #[test]
    fn an_arrow_key_does_nothing_to_an_unfocused_scroll() {
        // A key press is not routed by position, so it reaches the scroll only
        // because the caller sent it to the focused node. Every scroll would
        // otherwise move on every arrow press.
        let (_nodes, scroll) = scroll(CONTENT);
        let mut down = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Down));
        assert!(!scroll.on_event(&mut down, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0);
        assert!(!down.consumed(), "and the key carries on up the tree");
    }

    #[test]
    fn a_key_step_is_a_fraction_of_the_viewport_not_a_number_of_pixels() {
        let (_nodes, short) = scroll(CONTENT);
        short.focused.set(true);
        let mut in_a_300_viewport = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Down));
        assert!(short.on_event(&mut in_a_300_viewport, VIEWPORT));
        assert_eq!(short.scroll_offset.get(), 30.0);

        let (_nodes, tall) = scroll(CONTENT);
        tall.focused.set(true);
        let mut in_a_600_viewport = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Down));
        assert!(tall.on_event(&mut in_a_600_viewport, Rect::new(0.0, 0.0, 200.0, 600.0)));
        assert_eq!(
            tall.scroll_offset.get(),
            60.0,
            "twice the viewport, twice the step — which is what a fraction means"
        );
    }

    #[test]
    fn left_and_right_are_not_keys_a_vertical_scroll_has() {
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.focused.set(true);
        for keycode in [
            sdl3::keyboard::Keycode::Left,
            sdl3::keyboard::Keycode::Right,
        ] {
            let mut event = key_down(Key::Keyboard(keycode));
            assert!(
                !scroll.on_event(&mut event, VIEWPORT),
                "{keycode:?} is not a key a vertical scroll has"
            );
            assert!(!event.consumed());
        }
        assert_eq!(scroll.scroll_offset.get(), 0.0, "so nothing moved");
    }

    #[test]
    fn the_gamepad_d_pad_scrolls() {
        use sdl3::gamepad::Button as Pad;
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.focused.set(true);

        let mut down = key_down(Key::Gamepad(Pad::DPadDown));
        assert!(scroll.on_event(&mut down, VIEWPORT), "the d-pad's down");
        assert_eq!(scroll.scroll_offset.get(), 30.0);

        let mut up = key_down(Key::Gamepad(Pad::DPadUp));
        assert!(scroll.on_event(&mut up, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0);

        let mut across = key_down(Key::Gamepad(Pad::DPadLeft));
        assert!(
            !scroll.on_event(&mut across, VIEWPORT),
            "and the horizontal half of the d-pad is not a vertical scroll"
        );
    }

    #[test]
    fn a_key_the_scroll_does_not_use_is_left_alone() {
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.focused.set(true);
        for key in [
            Key::Keyboard(sdl3::keyboard::Keycode::Tab),
            Key::Keyboard(sdl3::keyboard::Keycode::Return),
            Key::Gamepad(sdl3::gamepad::Button::South),
        ] {
            let mut event = key_down(key);
            assert!(
                !scroll.on_event(&mut event, VIEWPORT),
                "{key:?} is not its key"
            );
            assert!(!event.consumed(), "{key:?} carries on to the tree");
        }
    }

    #[test]
    fn nothing_moves_after_the_finger_lifts() {
        // Requirement 3 marks momentum optional and this task does not ask for
        // it, so a drag that ends leaves the content exactly where it was. If
        // momentum were added, this test is the one that has to change and the
        // one that says so.
        let (_nodes, scroll) = scroll(CONTENT);
        let mut last = drag(Offset::new(0.0, 100.0));
        assert!(scroll.on_event(&mut last, VIEWPORT));
        let where_it_stopped = scroll.scroll_offset.get();
        assert_eq!(where_it_stopped, 100.0);

        let mut released = InputEvent::new(
            InputEventKind::KeyUp {
                key: Key::Keyboard(sdl3::keyboard::Keycode::Escape),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        );
        assert!(
            !scroll.on_event(&mut released, VIEWPORT),
            "a release is not a scroll"
        );

        for _ in 0..60 {
            let _ = scroll.tick(ms(16));
        }
        assert_eq!(
            scroll.scroll_offset.get(),
            where_it_stopped,
            "a thousand milliseconds later, the content has not drifted"
        );
    }

    #[test]
    fn a_scroll_by_of_zero_re_clamps_after_the_content_shrank() {
        let (mut nodes, mut scroll) = scroll(CONTENT);
        scroll.scroll_offset.set(600.0);

        // The caller lays out a shorter content and tells the widget.
        if let Some(node) = nodes.get_mut(scroll.content) {
            node.layout_mut()
                .set_constraints(Constraints::tight(Size::new(200.0, 400.0)));
        }
        let panel = on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(200.0, 400.0),
            Size::new(200.0, 300.0),
        )
        .0;
        Layout::new(&mut nodes).layout(panel, Constraints::tight(Size::new(200.0, 300.0)));
        assert!(
            scroll.sync_content(&nodes),
            "the height it was told changed"
        );

        assert_eq!(
            scroll.scroll_by(VIEWPORT, 0.0),
            100.0,
            "400 - 300 is all there is"
        );
        assert_eq!(scroll.scroll_offset.get(), 100.0);
    }

    #[test]
    fn a_tap_is_not_a_scroll() {
        let (_nodes, scroll) = scroll(CONTENT);
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(100.0, 150.0)));
        assert!(
            !scroll.on_event(&mut tap, VIEWPORT),
            "a tap belongs to whatever is under it, and it is not this scroll's"
        );
        assert_eq!(scroll.scroll_offset.get(), 0.0);
    }

    #[test]
    fn a_key_up_is_not_a_key_down() {
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.focused.set(true);
        let mut up = InputEvent::new(
            InputEventKind::KeyUp {
                key: Key::Keyboard(sdl3::keyboard::Keycode::Down),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        );
        assert!(!scroll.on_event(&mut up, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0);
    }
    // ----------------------------------------------------------------- painting

    #[test]
    fn a_scroll_paints_a_groove_a_thumb_and_nothing_else() {
        let (_nodes, scroll) = scroll(CONTENT);
        let painted = rounded(&scroll.paint(VIEWPORT));

        assert_eq!(painted.len(), 2, "a groove and a thumb");
        assert_eq!(
            painted[0].0,
            Rect::new(
                VIEWPORT.x + VIEWPORT.width - SCROLLBAR_MARGIN - SCROLLBAR_THICKNESS,
                VIEWPORT.y,
                SCROLLBAR_THICKNESS,
                VIEWPORT.height,
            ),
            "the groove is inset from the right edge and runs the whole height"
        );
        assert_eq!(
            painted[0].1,
            SCROLLBAR_THICKNESS / 2.0,
            "with half its thickness as radius"
        );
        assert_eq!(painted[0].2, scroll.track.get());
        assert_eq!(painted[1].2, scroll.thumb.get());
    }

    #[test]
    fn the_thumbs_length_is_the_visible_fraction_of_the_content() {
        // 300 of 900 on screen is a third, and a third of a 300 viewport is 100.
        let (_nodes, thumbed) = scroll(CONTENT);
        let thumb = rounded(&thumbed.paint(VIEWPORT))[1].0;
        assert_eq!(thumb.height, 100.0);
        assert_eq!(thumb.width, SCROLLBAR_THICKNESS);
        assert_eq!(thumb.y, VIEWPORT.y, "and at the top, which is the offset");

        // A viewport that does not divide into its content gives a thumb that is
        // not a whole number either: 250 of 900 is a bit over a quarter of the
        // content, and 250 * 250 / 900 is 69.44 rather than a figure a test should
        // be written against by hand.
        let (_other, odd) = scroll(CONTENT);
        let half_height = Rect::new(0.0, 0.0, 200.0, 250.0);
        assert_close(
            rounded(&odd.paint(half_height))[1].0.height,
            69.444_44,
            "250 * 250 / 900",
        );
    }

    #[test]
    fn the_thumb_travels_the_groove_between_its_two_ends() {
        let (_nodes, scroll) = scroll(CONTENT);
        let top_of = |offset: f32| {
            scroll.scroll_offset.set(offset);
            rounded(&scroll.paint(VIEWPORT))[1].0.y
        };

        assert_eq!(top_of(0.0), 0.0, "the top of the groove");
        assert_eq!(
            top_of(300.0),
            100.0,
            "half way through 600 of scroll is half way down a 200 run"
        );
        assert_eq!(
            top_of(600.0),
            200.0,
            "and the end: 200 + the thumb's own 100 is the groove's 300"
        );
        assert_eq!(
            top_of(5000.0),
            200.0,
            "a hand-written offset is pinned, not obeyed"
        );
    }

    #[test]
    fn a_short_thumb_is_never_shorter_than_the_minimum() {
        // 300 of 20000 on screen is 4.5 pixels, which is a speck nobody can see.
        let (_nodes, scroll) = scroll(20000.0);
        let thumb = rounded(&scroll.paint(VIEWPORT))[1].0;
        assert_eq!(
            thumb.height, SCROLLBAR_MIN_THUMB,
            "the floor, not 300 * 300 / 20000"
        );
        assert_eq!(
            SCROLLBAR_MIN_THUMB, 16.0,
            "and the floor is the number it documents"
        );

        scroll.scroll_offset.set(19700.0);
        let at_end = rounded(&scroll.paint(VIEWPORT))[1].0;
        assert_eq!(
            at_end.y + at_end.height,
            VIEWPORT.y + VIEWPORT.height,
            "the thumb still reaches the very end of the groove after being floored"
        );
    }

    #[test]
    fn a_scrollbar_on_a_viewport_off_the_origin_is_inside_that_viewport() {
        // Every other fixture lays the viewport at (0, 0), where an origin read
        // as an extent subtracts nothing.
        assert_eq!(OFFSET_VIEWPORT.x, 664.0);
        let (_nodes, scroll) = scroll(CONTENT);
        let painted = rounded(&scroll.paint(OFFSET_VIEWPORT));

        assert_eq!(
            painted[0].0.x,
            OFFSET_VIEWPORT.x + OFFSET_VIEWPORT.width - SCROLLBAR_MARGIN - SCROLLBAR_THICKNESS,
            "the groove is inside the viewport, not at 192 of the window"
        );
        assert_eq!(
            painted[0].0.y, OFFSET_VIEWPORT.y,
            "and starts at its top, not at 0"
        );
        scroll.scroll_offset.set(300.0);
        assert_eq!(rounded(&scroll.paint(OFFSET_VIEWPORT))[1].0.y, 220.0);
    }

    #[test]
    fn a_scroll_with_nothing_to_scroll_paints_nothing() {
        for height in [0.0, 120.0, 300.0] {
            let (_nodes, scroll) = scroll(height);
            assert!(
                scroll.paint(VIEWPORT).is_empty(),
                "{height} of content in a 300 viewport does not scroll"
            );
        }
    }

    #[test]
    fn a_focused_scroll_draws_a_ring_around_its_thumb_and_the_thumb_covers_it() {
        let (_nodes, scroll) = scroll(CONTENT);
        assert_eq!(
            rounded(&scroll.paint(VIEWPORT)).len(),
            2,
            "no ring while unfocused"
        );

        scroll.focused.set(true);
        let painted = rounded(&scroll.paint(VIEWPORT));
        assert_eq!(painted.len(), 3, "a focused one has a ring as well");

        let (thumb, ring) = (painted[2].0, painted[1].0);
        assert_eq!(
            painted[1].2,
            scroll.palette().ring,
            "in the ring's own colour"
        );
        assert_eq!(
            ring,
            Rect::new(
                thumb.x - FOCUS_RING,
                thumb.y - FOCUS_RING,
                thumb.width + FOCUS_RING * 2.0,
                thumb.height + FOCUS_RING * 2.0,
            ),
            "the ring is the thumb's own rect grown, and NOT the viewport's — a filled \
             rectangle that size with nothing over its middle is a card, which is what \
             .ai/NEVERAGAIN.md records against the slider's focus ring"
        );
        assert!(
            ring.x + ring.width <= OFFSET_VIEWPORT.x + OFFSET_VIEWPORT.width,
            "and a ring around the thumb stays inside the viewport"
        );
        assert_eq!(
            painted[2].2,
            scroll.thumb.get(),
            "the thumb is recorded after the ring, so the ring reads as a border"
        );
    }

    #[test]
    fn a_zero_width_focus_ring_is_not_painted() {
        // A caller that turns the indicator off gets the two commands and not a
        // third that draws nothing.
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.focused.set(true);
        scroll.focus_ring.set(0.0);
        assert_eq!(rounded(&scroll.paint(VIEWPORT)).len(), 2);
        assert_eq!(scroll.style().ring_width, 0.0);
    }

    #[test]
    fn an_unfocused_scroll_has_no_ring_even_with_a_ring_width() {
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.focus_ring.set(4.0);
        assert_eq!(
            scroll.style().ring_width,
            0.0,
            "the ring is applied independently of the width the caller set"
        );
        assert_eq!(rounded(&scroll.paint(VIEWPORT)).len(), 2);

        scroll.focused.set(true);
        assert_eq!(
            scroll.style().ring_width,
            4.0,
            "and it appears when focused"
        );
    }

    #[test]
    fn a_scroll_that_has_never_been_aimed_paints_the_neutral_defaults() {
        let mut nodes = Arena::new();
        let content = node::create(&mut nodes, State::new());
        let mut scroll = Scroll::new(&mut nodes, content);
        scroll.set_content_height(CONTENT);
        assert_eq!(scroll.track.get(), Palette::default().track);
        assert_eq!(
            rounded(&scroll.paint(VIEWPORT))
                .first()
                .map(|(_, _, color)| *color),
            Some(Palette::default().track)
        );
    }

    #[test]
    fn snapping_puts_a_themed_scrollbar_where_its_theme_says_at_once() {
        // Built from `new` rather than through the `scroll` helper, which snaps.
        let mut nodes = Arena::new();
        let content = node::create(&mut nodes, State::new());
        let mut scroll = Scroll::new(&mut nodes, content);
        scroll.set_content_height(CONTENT);
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(themed.track, scroll.track.get(), "so this can fail");

        scroll.set_palette(themed);
        scroll.snap_to_state();

        assert_eq!(scroll.track.get(), themed.track);
        assert_eq!(scroll.thumb.get(), themed.thumb);
        assert!(!scroll.is_animating(), "a snap is not a transition");
    }

    #[test]
    fn setting_the_palette_leaves_the_scrollbar_where_it_is() {
        let (_nodes, mut scroll) = scroll(CONTENT);
        let before = scroll.thumb.get();
        scroll.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(
            scroll.thumb.get(),
            before,
            "a theme switch is animated by the caller, not instantaneous"
        );
    }

    #[test]
    fn a_theme_switch_is_animated_rather_than_snapped() {
        let (_nodes, mut scroll) = scroll(CONTENT);
        let light = Palette::from_theme(&Theme::light());
        scroll.set_palette(light);
        scroll.animate_to_state(motion());
        assert!(scroll.is_animating());

        assert!(scroll.tick(ms(50)), "half way through, something wrote");
        // Half of 50 ms on a 100 ms linear curve, so `from + (to - from) * 0.5`:
        // the dark theme's Border is 51 and the light one's is 224, and each channel
        // is rounded to the nearest of the 256 a channel holds, so 137.5 arrives as
        // 138. That is the number, not a number in the right neighbourhood.
        assert_eq!(
            scroll.track.get(),
            Color::new(138, 138, 138, 255),
            "halfway between the two themes' grooves"
        );
        assert_ne!(scroll.track.get(), light.track, "and not the end already");

        assert!(scroll.tick(ms(50)), "and it arrives");
        assert_eq!(scroll.track.get(), light.track);
        assert_eq!(scroll.thumb.get(), light.thumb);
        assert!(!scroll.is_animating());
    }

    #[test]
    fn a_second_change_replaces_the_first_rather_than_racing_it() {
        let (_nodes, mut scroll) = scroll(CONTENT);
        scroll.set_palette(Palette::from_theme(&Theme::light()));
        scroll.animate_to_state(motion());
        assert!(scroll.tick(ms(50)));

        let dark = Palette::from_theme(&Theme::dark());
        scroll.set_palette(dark);
        scroll.animate_to_state(motion());
        for _ in 0..4 {
            let _ = scroll.tick(ms(50));
        }
        assert_eq!(
            scroll.track.get(),
            dark.track,
            "the scrollbar ends on the newest palette"
        );
        assert!(!scroll.is_animating(), "and nothing is left running");
    }

    #[test]
    fn snapping_a_mid_transition_scroll_back_out_ends_the_transition() {
        let (_nodes, mut scroll) = scroll(CONTENT);
        scroll.set_palette(Palette::from_theme(&Theme::light()));
        scroll.animate_to_state(motion());
        assert!(scroll.tick(ms(10)), "a colour is animating");

        let dark = Palette::from_theme(&Theme::dark());
        scroll.set_palette(dark);
        scroll.snap_to_state();
        assert_eq!(scroll.track.get(), dark.track);

        assert!(!scroll.tick(ms(500)), "and nothing arrives afterwards");
        assert_eq!(scroll.track.get(), dark.track);
    }

    #[test]
    fn scrolling_leaves_a_theme_switch_running() {
        // The slider clears its clock on every interaction, because its thumb is
        // animated towards a value the interaction overrode. Nothing here
        // animates the offset, so a drag has nothing to override and a theme
        // switch must survive it.
        let (_nodes, mut scroll) = scroll(CONTENT);
        scroll.set_palette(Palette::from_theme(&Theme::light()));
        scroll.animate_to_state(motion());

        let mut up = drag(Offset::new(0.0, 100.0));
        assert!(scroll.on_event(&mut up, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 100.0);
        assert!(
            scroll.is_animating(),
            "and the colours are still on their way to the new palette"
        );
    }

    #[test]
    fn the_palette_is_the_themes_border_text_muted_and_text() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            let color = |token| theme.get(token).as_color().unwrap();
            assert_eq!(palette.track, color(ThemeToken::Border));
            assert_eq!(
                palette.thumb,
                color(ThemeToken::TextMuted),
                "a scrollbar tells the user where they are; it is not a control"
            );
            assert_eq!(
                palette.ring,
                color(ThemeToken::Text),
                "the ring is on the background, not on the groove"
            );
        }
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        assert_ne!(
            Palette::from_theme(&Theme::dark()),
            Palette::from_theme(&Theme::light())
        );
    }

    #[test]
    fn the_groove_and_the_thumb_are_two_different_colours() {
        // Every painting assertion above compares each shape against its own
        // property. If the two properties held the same colour, a swap between
        // them would leave all of them green.
        let palette = Palette::from_theme(&Theme::dark());
        assert_ne!(palette.track, palette.thumb);
        assert_ne!(Palette::default().track, Palette::default().thumb);
    }

    #[test]
    fn a_theme_that_holds_the_wrong_kind_of_value_still_answers() {
        let theme = Theme::dark();
        theme.set(
            ThemeToken::Border,
            crate::theme::PropertyValue::Text("x".to_string()),
        );
        assert_eq!(
            Palette::from_theme(&theme).track,
            Color::new(0, 0, 0, 255),
            "black rather than a panic"
        );
    }

    // ---------------------------------------------------------------- clipping

    #[test]
    fn a_command_fully_inside_the_clip_passes_through_unchanged() {
        let command = filled_rect(10.0, 10.0, 10.0, 10.0, 200);
        let clip = Rect::new(0.0, 0.0, 200.0, 200.0);
        assert_eq!(
            clip_commands(std::slice::from_ref(&command), clip),
            vec![command]
        );
    }

    #[test]
    fn a_command_fully_outside_the_clip_is_dropped() {
        let kept = filled_rect(10.0, 10.0, 10.0, 10.0, 200);
        let dropped = filled_rect(400.0, 400.0, 10.0, 10.0, 100);
        let clip = Rect::new(0.0, 0.0, 200.0, 200.0);
        assert_eq!(
            clip_commands(&[kept.clone(), dropped], clip),
            vec![kept],
            "and a dropped command costs the caller nothing at all"
        );
    }

    #[test]
    fn a_command_straddling_the_edge_is_kept_whole() {
        // The documented choice: trimming is not clipping. A rect cut to part of
        // itself is a *smaller* rect with its own corners, which is a different
        // shape rather than a clipped one, and the scissor is what cuts it.
        let straddler = filled_rect(-30.0, 10.0, 60.0, 20.0, 90);
        let clip = Rect::new(0.0, 0.0, 200.0, 200.0);
        let kept = clip_commands(std::slice::from_ref(&straddler), clip);
        assert_eq!(
            kept,
            vec![straddler.clone()],
            "the command itself, untrimmed"
        );
        assert_eq!(kept[0], straddler, "so its own size survives the clip");
    }

    #[test]
    fn clipping_keeps_the_commands_in_the_order_they_were_recorded() {
        let first = filled_rect(10.0, 10.0, 10.0, 10.0, 1);
        let gone = filled_rect(900.0, 900.0, 10.0, 10.0, 2);
        let middle = filled_rect(20.0, 20.0, 10.0, 10.0, 3);
        let last = filled_rect(30.0, 30.0, 10.0, 10.0, 4);
        let clip = Rect::new(0.0, 0.0, 200.0, 200.0);
        assert_eq!(
            clip_commands(&[first.clone(), gone, middle.clone(), last.clone()], clip),
            vec![first, middle, last],
            "paint order is what decides who covers whom, and clipping must not change it"
        );
    }

    #[test]
    fn a_command_touching_the_edge_counts_as_inside() {
        // The bounds are inclusive, so an item whose last row is the clip's last
        // row is drawn — the alternative drops a visible pixel row.
        let flush = filled_rect(190.0, 190.0, 10.0, 10.0, 70);
        let clip = Rect::new(0.0, 0.0, 200.0, 200.0);
        assert_eq!(
            clip_commands(std::slice::from_ref(&flush), clip),
            vec![flush]
        );
    }

    #[test]
    fn a_text_run_is_kept_because_a_draw_command_does_not_carry_its_width() {
        let text = DrawCommand::Text {
            x: 0.0,
            y: 5000.0,
            text: "far below the viewport".to_string(),
            color: Color::new(255, 255, 255, 255),
            font_size: 16.0,
            extra_advance: 0.0,
        };
        assert_eq!(
            command_bounds(&text),
            None,
            "there is no width in the command to bound it with"
        );
        assert_eq!(
            clip_commands(
                std::slice::from_ref(&text),
                Rect::new(0.0, 0.0, 200.0, 200.0)
            ),
            vec![text],
            "and a label nobody measured is dropped, which is a blank list"
        );
    }

    #[test]
    fn command_bounds_covers_every_shape_that_carries_one() {
        assert_eq!(
            command_bounds(&filled_rect(4.0, 8.0, 10.0, 6.0, 0)),
            Some(Rect::new(4.0, 8.0, 10.0, 6.0))
        );
        assert_eq!(
            command_bounds(&DrawCommand::RoundedRect {
                rect: Rect::new(1.0, 2.0, 3.0, 4.0),
                radius: 1.0,
                color: Color::new(0, 0, 0, 255),
            }),
            Some(Rect::new(1.0, 2.0, 3.0, 4.0))
        );
        assert_eq!(
            command_bounds(&DrawCommand::Image {
                rect: Rect::new(5.0, 6.0, 7.0, 8.0),
                texture: crate::paint::TextureId::new(1),
                uv: crate::paint::UvRect::full(),
                opacity: 1.0,
                radius: 0.0,
            }),
            Some(Rect::new(5.0, 6.0, 7.0, 8.0))
        );
        assert_eq!(
            command_bounds(&DrawCommand::Circle {
                center: (100.0, 50.0),
                radius: 12.0,
                color: Color::new(0, 0, 0, 255),
            }),
            Some(Rect::new(88.0, 38.0, 24.0, 24.0))
        );
        assert_eq!(
            command_bounds(&DrawCommand::Line {
                start: (10.0, 10.0),
                end: (30.0, 40.0),
                width: 4.0,
                color: Color::new(0, 0, 0, 255),
            }),
            Some(Rect::new(8.0, 8.0, 24.0, 34.0)),
            "two ends and half a width at each side"
        );
        assert_eq!(
            command_bounds(&DrawCommand::Path {
                points: vec![(10.0, 30.0), (50.0, 10.0), (30.0, 60.0)],
                width: 2.0,
                color: Color::new(0, 0, 0, 255),
                closed: false,
            }),
            Some(Rect::new(9.0, 9.0, 42.0, 52.0)),
            "the extent of the points"
        );
        assert_eq!(
            command_bounds(&DrawCommand::Path {
                points: Vec::new(),
                width: 2.0,
                color: Color::new(0, 0, 0, 255),
                closed: false,
            }),
            None,
            "an empty path visits nothing and there is no place to say it is"
        );
    }

    #[test]
    fn an_item_whose_band_meets_the_visible_one_is_kept_and_one_that_misses_is_dropped() {
        // Requirement 5 end to end, in the shape a caller actually uses it: the
        // visible band comes from `visible_rect`, each item's band from where the
        // caller put it.
        let clip = Rect::new(0.0, 0.0, 200.0, 300.0);
        let commands: Vec<DrawCommand> = (0u8..8)
            .map(|index| filled_rect(0.0, f32::from(index) * 100.0, 200.0, 100.0, 40 + index))
            .collect();
        let band = visible_rect(clip, CONTENT, 250.0);

        let kept = clip_commands(&commands, band);
        assert_eq!(
            kept.len(),
            4,
            "a band over [250, 550] of 100-tall items starting at 0: the items at 200, \
             300, 400 and 500 all meet it — 200 does, because its bottom row at 300 is \
             past the band's top at 250 — and 600 does not"
        );
        assert_eq!(
            kept.len(),
            commands
                .iter()
                .filter(|command| {
                    let bounds = command_bounds(command).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
                    bounds.y < band.y + band.height && band.y < bounds.y + bounds.height
                })
                .count(),
            "and the count agrees with the closed form rather than with the number above"
        );
    }

    #[test]
    fn a_clip_rect_is_the_rect_the_layout_pass_kept_for_the_content() {
        // The scissor rect is not computed here: it is the layout pass's own
        // answer, and reading it back is what keeps the two from diverging.
        let (mut nodes, scroll) = scroll(CONTENT);
        let (_, viewport) = on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(200.0, 900.0),
            Size::new(200.0, 300.0),
        );

        assert_eq!(viewport, Rect::new(0.0, 0.0, 200.0, 300.0));
        assert_eq!(
            scroll.clip_rect(&nodes),
            Some(viewport),
            "the content is clipped to its viewport, which is the rect a renderer \
             would scissors to"
        );
        assert!(
            scroll
                .clip_rect(&nodes)
                .is_some_and(|rect| rect.height < CONTENT),
            "and that rect is shorter than the content it is clipping"
        );
    }
    // ------------------------------------------------- the arena, and the two steps

    #[test]
    fn applying_the_offset_moves_the_content_node_and_marks_the_tree() {
        let (mut nodes, scroll) = scroll(CONTENT);
        let (panel, viewport) = on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(200.0, 900.0),
            Size::new(200.0, 300.0),
        );
        assert_eq!(
            nodes.get(scroll.content).unwrap().layout().rect(),
            Some(layout_module::Rect::from_parts(0.0, 0.0, 200.0, 900.0)),
            "the content starts at its viewport's top, and it is taller than the viewport"
        );

        scroll.scroll_offset.set(60.0);
        assert!(scroll.apply_offset(&mut nodes), "the node moved");
        Layout::new(&mut nodes).layout(panel, Constraints::tight(Size::new(200.0, 300.0)));

        let placed = nodes.get(scroll.content).unwrap().layout().rect();
        let expected = scroll.content_rect(viewport);
        assert_eq!(
            placed,
            Some(layout_module::Rect::from_parts(
                expected.x,
                expected.y,
                expected.width,
                expected.height,
            )),
            "and the pass put it exactly where `content_rect` says it goes"
        );
        assert_eq!(
            placed.map(|rect| rect.origin.y),
            Some(-60.0),
            "60 above the viewport"
        );
    }

    #[test]
    fn applying_the_offset_twice_moves_nothing_the_second_time() {
        let (mut nodes, scroll) = scroll(CONTENT);
        let (panel, _) = on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(200.0, 900.0),
            Size::new(200.0, 300.0),
        );
        scroll.scroll_offset.set(60.0);
        assert!(scroll.apply_offset(&mut nodes));
        Layout::new(&mut nodes).layout(panel, Constraints::tight(Size::new(200.0, 300.0)));
        assert!(
            !nodes.get(scroll.content).unwrap().layout().is_dirty(),
            "the first move dirtied the content and a pass has since cleared it"
        );

        assert!(
            !scroll.apply_offset(&mut nodes),
            "the offset has not changed, so the node does not move and nothing is \
             marked dirty for a pass that would find the same rects"
        );
        assert!(
            !nodes.get(scroll.content).unwrap().layout().is_dirty(),
            "and the content is still clean, so a pass over it is skipped"
        );
    }

    #[test]
    fn applying_a_zero_offset_leaves_an_unplaced_content_alone() {
        let (mut nodes, scroll) = scroll(CONTENT);
        on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(200.0, 900.0),
            Size::new(200.0, 300.0),
        );
        assert_eq!(
            nodes.get(scroll.content).unwrap().layout().position(),
            None,
            "an `Absolute` child with no position sits at its parent's origin"
        );
        assert!(
            !scroll.apply_offset(&mut nodes),
            "and an unset position is the same place as the origin, so offset zero \
             has nothing to do"
        );
    }

    #[test]
    fn a_removed_content_leaves_the_offset_written_and_the_move_unmade() {
        let (mut nodes, scroll) = scroll(CONTENT);
        scroll.scroll_offset.set(120.0);
        assert!(
            nodes.remove(scroll.content).is_some(),
            "the content is gone from the arena"
        );
        assert!(
            !scroll.apply_offset(&mut nodes),
            "there is no node left to move"
        );
        assert_eq!(scroll.content_size(&nodes), None, "and nothing to measure");
        assert_eq!(scroll.clip_rect(&nodes), None, "and nothing to clip to");
        assert_eq!(
            scroll.scroll_offset.get(),
            120.0,
            "while the offset itself is a number the widget owns, not the arena"
        );
    }

    #[test]
    fn syncing_the_content_reads_its_laid_out_height() {
        let (mut nodes, mut scroll) = scroll(0.0);
        let (panel, _) = on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(200.0, 900.0),
            Size::new(200.0, 300.0),
        );
        assert!(scroll.sync_content(&nodes), "measured for the first time");
        assert_eq!(scroll.content_height(), 900.0);
        assert!(
            !scroll.sync_content(&nodes),
            "and not again while it is the same"
        );

        // And it follows the content when the content changes.
        if let Some(node) = nodes.get_mut(scroll.content) {
            node.layout_mut()
                .set_constraints(Constraints::tight(Size::new(200.0, 500.0)));
        }
        Layout::new(&mut nodes).layout(panel, Constraints::tight(Size::new(200.0, 300.0)));
        assert!(scroll.sync_content(&nodes), "the content shrank");
        assert_eq!(scroll.content_height(), 500.0);
    }

    #[test]
    fn syncing_a_content_that_was_never_laid_out_changes_nothing() {
        let (nodes, mut scroll) = scroll(CONTENT);
        assert!(
            !scroll.sync_content(&nodes),
            "there is no rect to read, and zeroing the height would silently turn a \
             working scroll into one that does not scroll"
        );
        assert_eq!(scroll.content_height(), CONTENT);
    }

    #[test]
    fn a_scroll_off_the_origin_shifts_its_content_by_the_offset_and_not_by_the_origin() {
        // Every other geometry test lays the scroll at (0, 0), which is a blind
        // spot: a rect's origin and a rect's extent are different numbers. This
        // one is hung on a box at (664, 120) and the content's *absolute* rect is
        // compared, so a viewport that read its own origin as a length fails here.
        let mut nodes = Arena::new();
        let root = node::create(
            &mut nodes,
            State::new()
                .with_mode(LayoutMode::Absolute)
                .with_constraints(Constraints::tight(Size::new(1024.0, 600.0))),
        );
        let placed_at = node::create(
            &mut nodes,
            State::new()
                .with_mode(LayoutMode::Absolute)
                .with_position(Offset::new(664.0, 120.0))
                .with_constraints(Constraints::tight(Size::new(200.0, 300.0))),
        );
        assert!(node::attach(&mut nodes, root, placed_at));

        let content = node::create(
            &mut nodes,
            State::new().with_constraints(Constraints::tight(Size::new(200.0, 900.0))),
        );
        let mut scroll = Scroll::new(&mut nodes, content);
        if let Some(node) = nodes.get_mut(scroll.handle()) {
            node.layout_mut()
                .set_constraints(Constraints::tight(Size::new(200.0, 300.0)));
        }
        assert!(node::attach(&mut nodes, placed_at, scroll.handle()));
        Layout::new(&mut nodes).layout(root, Constraints::tight(Size::new(1024.0, 600.0)));
        assert!(scroll.sync_content(&nodes));

        scroll.scroll_offset.set(60.0);
        assert!(scroll.apply_offset(&mut nodes));
        Layout::new(&mut nodes).layout(root, Constraints::tight(Size::new(1024.0, 600.0)));

        let viewport = nodes
            .get(scroll.handle())
            .unwrap()
            .layout()
            .rect()
            .map(Rect::from)
            .unwrap();
        assert_eq!(
            viewport, OFFSET_VIEWPORT,
            "the scroll really is off the origin"
        );
        assert_eq!(
            nodes.get(scroll.content).unwrap().layout().rect(),
            Some(layout_module::Rect::from_parts(664.0, 60.0, 200.0, 900.0)),
            "664 across and 120 - 60 down, which is the origin *and* the offset"
        );
    }

    #[test]
    fn a_drag_over_the_content_reaches_the_scroll_and_not_the_panel_behind_it() {
        // The content is a node of its own and is the deepest node under the
        // pointer, so the event reaches the scroll by bubbling and the panel only
        // sees it if the scroll does not take it.
        let (mut nodes, scroll) = scroll(CONTENT);
        let (panel, _) = on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(200.0, 900.0),
            Size::new(200.0, 300.0),
        );

        let mut event = drag(Offset::new(0.0, 80.0));
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == scroll.handle() {
                scroll.on_event(event, VIEWPORT);
            }
        });

        assert_eq!(
            reached,
            vec![scroll.content, scroll.handle()],
            "the content, then the scroll — and the panel never, because the scroll \
             consumed it"
        );
        assert!(event.consumed());
        assert_eq!(scroll.scroll_offset.get(), 80.0);
    }

    #[test]
    fn a_drag_beside_a_narrower_content_reaches_the_scroll_directly() {
        // The other half of the route. A scrolled content's rect reaches from above
        // the viewport down past its bottom, so every finger over the viewport lands
        // on the *content* and reaches the scroll by bubbling — which is the case the
        // test beside this one covers. A content narrower than its viewport is the
        // case where the scroll is itself the deepest node under the pointer, and it
        // has to scroll there too.
        let (mut nodes, scroll) = scroll(CONTENT);
        let (panel, _) = on_a_panel(
            &scroll,
            &mut nodes,
            Size::new(80.0, 900.0),
            Size::new(200.0, 300.0),
        );
        Layout::new(&mut nodes).layout(panel, Constraints::tight(Size::new(200.0, 300.0)));

        let mut event = InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(0.0, 40.0),
            },
            Some(Offset::new(150.0, 10.0)),
        );
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == scroll.handle() {
                scroll.on_event(event, VIEWPORT);
            }
        });

        assert_eq!(
            reached,
            vec![scroll.handle()],
            "the scroll is the node under a point its content does not cover"
        );
        assert!(event.consumed());
        assert_eq!(
            scroll.scroll_offset.get(),
            40.0,
            "and it scrolled there: down is later, so a downward drag moves the offset up"
        );
    }

    #[test]
    fn a_scroll_with_a_callback_still_scrolls_without_one() {
        // A caller that has given the widget nothing to report to is an ordinary
        // caller: this is the shape every other widget in the crate has.
        let (nodes, scroll) = scroll(CONTENT);
        let before = scroll.scroll_offset.get();
        let mut up = drag(Offset::new(0.0, 10.0));
        assert!(scroll.on_event(&mut up, VIEWPORT));
        assert_ne!(scroll.scroll_offset.get(), before);
        assert!(scroll.is_attached(&nodes));
    }

    #[test]
    fn a_negative_content_height_is_floored_at_zero() {
        let (nodes, mut scroll) = scroll(CONTENT);
        assert_eq!(scroll.set_content_height(-100.0), 0.0);
        assert_eq!(
            scroll.content_height(),
            0.0,
            "an extent cannot be negative, and one would make max_scroll larger than \
             the content it describes"
        );
        let mut up = drag(Offset::new(0.0, 10.0));
        assert!(scroll.on_event(&mut up, VIEWPORT));
        assert_eq!(scroll.scroll_offset.get(), 0.0);
        assert!(nodes.get(scroll.handle()).is_some());
    }

    #[test]
    fn a_viewport_with_no_height_has_nothing_to_scroll() {
        let (_nodes, scroll) = scroll(CONTENT);
        let collapsed = Rect::new(0.0, 0.0, 200.0, 0.0);
        let mut down = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Down));
        scroll.focused.set(true);
        assert!(
            scroll.on_event(&mut down, collapsed),
            "the key is still consumed: it was aimed at this scroll"
        );
        assert_eq!(
            scroll.scroll_offset.get(),
            0.0,
            "but a tenth of no height is nothing, and 900 of content has all of it \
             to scroll even so"
        );
        assert!(
            scroll.paint(collapsed).iter().any(
                |command| matches!(command, DrawCommand::RoundedRect { rect, .. } if rect.height == 0.0)
            ),
            "the groove collapses with it rather than reaching out of the node"
        );
    }

    #[test]
    fn the_offset_survives_a_paint_it_does_not_apply() {
        // Painting is a read. A caller that paints the scrollbar on a frame where
        // the offset has moved must not have the move undone by the draw.
        let (_nodes, scroll) = scroll(CONTENT);
        scroll.scroll_offset.set(250.0);
        let before = rounded(&scroll.paint(VIEWPORT));
        let after = rounded(&scroll.paint(VIEWPORT));
        assert_eq!(before, after, "two paints of the same state agree");
        assert_eq!(
            scroll.scroll_offset.get(),
            250.0,
            "and the offset is still 250"
        );
    }

    #[test]
    fn a_themed_scrollbars_thumb_is_a_different_colour_from_its_groove_on_screen() {
        // The two properties are separate for a reason a screenshot can see: a
        // scrollbar whose thumb is its groove's colour reads as an empty groove.
        let (_nodes, scroll) = scroll(CONTENT);
        let painted = rounded(&scroll.paint(VIEWPORT));
        assert_ne!(
            painted[0].2, painted[1].2,
            "the groove and the thumb are painted in different colours"
        );
        assert_eq!(painted[0].2, scroll.track.get());
        assert_eq!(painted[1].2, scroll.thumb.get());
    }

    #[test]
    fn every_public_construction_of_a_scroll_produces_a_scrollable_one() {
        // The four shapes a caller can build: the helper, a bare `new`, a themed
        // `new`, and one whose content was measured from the arena.
        let (_nodes, from_helper) = scroll(CONTENT);
        assert_eq!(from_helper.max_scroll_for(VIEWPORT), 600.0);

        let mut nodes = Arena::new();
        let bare_content = node::create(&mut nodes, State::new());
        let mut bare = Scroll::new(&mut nodes, bare_content);
        bare.set_content_height(CONTENT);
        assert_eq!(bare.max_scroll_for(VIEWPORT), 600.0);

        let mut themed_nodes = Arena::new();
        let themed_content = node::create(&mut themed_nodes, State::new());
        let mut themed = Scroll::new(&mut themed_nodes, themed_content);
        themed.set_palette(Palette::from_theme(&Theme::light()));
        themed.snap_to_state();
        themed.set_content_height(CONTENT);
        assert_eq!(themed.max_scroll_for(VIEWPORT), 600.0);

        let counted = Rc::new(Cell::new(0_u32));
        let seen = Rc::clone(&counted);
        themed
            .scroll_offset
            .on_change(move |_value: &f32| seen.set(seen.get() + 1));
        let mut wheel_event = wheel(0.0, 1.0);
        themed.on_event(&mut wheel_event, VIEWPORT);
        assert_eq!(
            counted.get(),
            1,
            "and the property a caller marks dirty with is written exactly once"
        );
    }
}
