//! The TabBar widget: a row of tabs that knows which one is selected.
//!
//! A tab bar owns its [`Button`]s rather than taking them. It is a [`Container`]
//! in [`LayoutMode::row`](crate::layout::LayoutMode::row), one button per label,
//! a selection and the single aim that carries every button's appearance. The
//! buttons are its children, so the layout pass places them the way it places
//! any row — this widget does not compute a tab's rect, it reads the one the
//! pass left on the node.
//!
//! **The selection is one write path and the aim is one other.** [`TabBar::select`]
//! writes the [`Button::selected`] flags and reports through
//! [`TabBar::on_select`]; it does not move any appearance. [`TabBar::sync`] is the
//! button's only aim, and it reaches exactly the buttons whose selected, hovered
//! or pressed flags have moved since the last aim. A caller therefore drives the
//! widget with a press, a hover, a focus or a click and gets the transition on
//! the next [`TabBar::sync`].
//!
//! **A button's click asks; it does not write.** The click handler a tab's button
//! carries records the tab in a private request, because a callback is `Fn` and
//! cannot write the bar. [`TabBar::sync`] drains that request through
//! [`TabBar::select`], so a finger, an activation key and a navigation key all
//! reach the selection by the same door.
//!
//! **Focus is not part of the aimed appearance.** The focus ring is drawn from
//! the palette at paint time rather than from an animated property, so
//! [`TabBar::focus`] writes the buttons' `focused` flags and nothing is aimed.
//! [`TabBar::on_event`] moves the selection, not the focus ring; a caller that
//! wants the two coupled writes [`TabBar::focus`] from its own
//! [`TabBar::on_select`].
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::widgets::tab_bar::TabBar;
//!
//! let mut nodes = Arena::new();
//! let bar = TabBar::new(&mut nodes, ["One", "Two"]);
//! let two = bar.tab_ids()[1];
//!
//! assert_eq!(bar.selected(), None, "nothing is selected to begin with");
//! assert!(bar.select(two), "and a selection is a change");
//! assert_eq!(bar.selected(), Some(two));
//! assert_eq!(bar.label(two), Some("Two"));
//! ```

use std::cell::Cell;
use std::time::Duration;

use crate::animation::Easing;
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::{Constraints, FlexConfig, LayoutMode, Offset, Padding, Size};
use crate::node::WidgetNode;
use crate::paint::{DrawCommand, Rect};
use crate::property::Property;
use crate::widgets::button::{Button, Motion, Palette};
use crate::widgets::container::Container;
use crate::widgets::Callback;

/// The gap between two tab buttons, in pixels.
///
/// A constant rather than a theme token for the reason the button's own
/// `MIN_TOUCH_TARGET` documents: the theme has no token for a widget's own
/// parts. The bar's [`Container`] carries it as its flex spacing, so the layout
/// pass places the gap and not this widget.
pub const TAB_SPACING: f32 = 8.0;

/// The bar's padding, on every side, in pixels.
///
/// The bar is as tall as [`TAB_HEIGHT`] plus this on the top and the bottom, so
/// the tabs sit inside a strip the same colour as the bar's background.
pub const TAB_BAR_PADDING: f32 = 10.0;

/// The height of one tab, in pixels.
///
/// At the 44-pixel touch floor, so every tab is a target a finger can hit.
/// [`TabBar::measure`] gives every button a tight box this tall, which is what
/// keeps a short label's button — whose content is shorter — from shrinking
/// below the floor.
pub const TAB_HEIGHT: f32 = 44.0;

/// The motion a tab bar is built with: 150 ms on the standard curve.
///
/// The widget's own default until a caller gives it a themed one with
/// [`TabBar::set_motion`]. It is the fast duration both themes hold, and the
/// curve both hold, so an unthemed bar moves exactly as a themed one does.
const DEFAULT_MOTION: Motion = Motion {
    duration: Duration::from_millis(150),
    easing: Easing::EaseInOut,
};

/// One tab's identity: the position of its button in the bar.
///
/// An index and not a label, because a label is what a caller reads and two tabs
/// may share one; and not a node handle, because a handle says nothing about the
/// order the tabs are in. It is opaque — [`TabId::index`] is the only way to its
/// number — so a caller cannot build an id the bar does not hold.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TabId(usize);

impl TabId {
    /// Returns the tab's position in the bar, from zero.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// One tab: its button, its label and the flags the last aim was for.
struct Tab {
    /// The widget, and the bar's child.
    button: Button,
    /// The text the tab was created with.
    ///
    /// Held beside the button because [`TabBar::label`] lends a `&str` and a
    /// [`Button`]'s label is a [`Property<String>`](crate::property::Property),
    /// whose `get` clones — there is no borrow to hand out.
    label: String,
    /// The selected, hovered and pressed flags the last aim was for, or `None`
    /// when a new palette has made the appearance stale and the next aim must
    /// re-reach every tab.
    ///
    /// **The focus flag is deliberately not one of the three**: the ring is
    /// painted straight from the palette, so a focus change needs no aim. A
    /// `Cell` because [`TabBar::sync`] borrows the bar immutably.
    aimed: Cell<Option<(bool, bool, bool)>>,
}

/// A row of tabs, one of them selected.
///
/// The bar holds its [`Container`] and the buttons attached to it, a selection,
/// the motion an appearance transition runs on, and the callback a selection
/// reports through. It does not hold a clock: every button owns its own, and
/// [`TabBar::tick`] advances them together.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::widgets::tab_bar::TabBar;
///
/// let mut nodes = Arena::new();
/// let bar = TabBar::new(&mut nodes, ["One", "Two", "Three"]);
/// assert_eq!(bar.tab_ids().len(), 3, "one tab per label");
/// ```
pub struct TabBar {
    /// The container the tabs are children of. A caller binds its `background`
    /// and `border_radius` to the theme the way it binds any container's.
    pub bar: Container,
    /// The callback a selection change reports through, with the new
    /// [`TabId`].
    ///
    /// It fires once per real change, from [`TabBar::select`] and nowhere else,
    /// so a caller that switches a screen here cannot be called twice for one
    /// tab. It is a field because a callback is set after construction, the way
    /// a [`Button`]'s own is.
    pub on_select: Callback<TabId>,
    /// A tab a click asked for, waiting for the next [`TabBar::sync`] to apply
    /// it. `Some` only between a button's click and that sync.
    requested: Property<Option<TabId>>,
    /// The tab that is selected, or `None` before the first selection.
    selected: Cell<Option<TabId>>,
    /// The motion an aim runs on. Written by [`TabBar::set_motion`].
    motion: Motion,
    /// The tabs, in the order their labels came in.
    tabs: Vec<Tab>,
}

impl std::fmt::Debug for TabBar {
    /// Prints the tab count and the selection, and not the buttons: a [`Button`]
    /// is not `Debug` — its properties are not — so a derived implementation
    /// could not exist, and a bar's identity is its tabs and its selection.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabBar")
            .field("tabs", &self.tabs.len())
            .field("selected", &self.selected.get())
            .finish()
    }
}

impl TabBar {
    /// Creates a bar with one button per label, and returns it.
    ///
    /// The bar is a row with [`TAB_SPACING`] between its tabs and
    /// [`TAB_BAR_PADDING`] on every side, and nothing is selected. Each button's
    /// click asks the bar for its own tab and carries no other state; the bar
    /// applies that request on the next [`TabBar::sync`].
    ///
    /// The tabs have no size until [`TabBar::measure`] is called: that is what
    /// reads the font and the padding a caller has set, and a width measured
    /// before they are set is a width for the defaults.
    #[must_use]
    pub fn new(
        nodes: &mut Arena<WidgetNode>,
        labels: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let bar = Container::new(nodes, LayoutMode::row());
        bar.set_padding(nodes, Padding::all(TAB_BAR_PADDING));
        bar.set_flex_config(nodes, FlexConfig::new().with_spacing(TAB_SPACING));

        let requested: Property<Option<TabId>> = Property::new(None);
        let mut tabs = Vec::new();
        for (index, label) in labels.into_iter().enumerate() {
            let label: String = label.into();
            let mut button = Button::new(nodes, label.clone());
            let asked = requested.clone();
            let id = TabId(index);
            button.on_click = Callback::new(move || asked.set(Some(id)));
            // A button just created has no parent, so this cannot fail; a
            // refusal would leave the button out of the bar rather than take a
            // frame down.
            let _ = bar.add_child(nodes, button.handle());
            tabs.push(Tab {
                button,
                label,
                aimed: Cell::new(Some((false, false, false))),
            });
        }

        TabBar {
            bar,
            on_select: Callback::none(),
            requested,
            selected: Cell::new(None),
            motion: DEFAULT_MOTION,
            tabs,
        }
    }

    /// Returns the bar's own node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.bar.handle()
    }

    /// Returns every tab's id, in the order the labels came in.
    #[must_use]
    pub fn tab_ids(&self) -> Vec<TabId> {
        (0..self.tabs.len()).map(TabId).collect()
    }

    /// Returns the id of the tab whose button is `handle`, or `None` when the
    /// handle is not a tab of this bar.
    ///
    /// This is the door from a routed handle back to the bar: a caller that has
    /// an event aimed at a node asks here whether the node is one of its tabs.
    #[must_use]
    pub fn tab_id_of(&self, handle: Handle) -> Option<TabId> {
        self.tabs
            .iter()
            .position(|tab| tab.button.handle() == handle)
            .map(TabId)
    }

    /// Returns the button at `id`, or `None` when there is no such tab.
    ///
    /// **A tab's label is fixed at construction.** The button is lent for driving
    /// — hover, press, focus — but a caller must not reassign its `label`: the
    /// bar keeps its own copy, which [`TabBar::label`] and [`TabBar::select_name`]
    /// read, and a reassignment would leave them naming text the tab no longer
    /// shows.
    #[must_use]
    pub fn tab(&self, id: TabId) -> Option<&Button> {
        self.tabs.get(id.index()).map(|tab| &tab.button)
    }

    /// Returns the label at `id`, or `None` when there is no such tab.
    #[must_use]
    pub fn label(&self, id: TabId) -> Option<&str> {
        self.tabs.get(id.index()).map(|tab| tab.label.as_str())
    }

    /// Selects the tab at `id`, and reports whether the selection moved.
    ///
    /// **The one write path.** It writes [`Button::selected`] on the outgoing
    /// and the incoming tab, records the new selection and reports it through
    /// [`TabBar::on_select`] once. Selecting the tab that is already selected
    /// changes nothing and reports `false`, and an id past the last tab does the
    /// same. **It does not aim**: the appearance moves on the next
    /// [`TabBar::sync`], which is what lets a select and a palette change in the
    /// same frame reach the screen as one transition.
    pub fn select(&self, id: TabId) -> bool {
        if self.tabs.get(id.index()).is_none() {
            return false;
        }
        if self.selected.get() == Some(id) {
            return false;
        }
        if let Some(previous) = self.selected.get() {
            if let Some(tab) = self.tabs.get(previous.index()) {
                tab.button.selected.set(false);
            }
        }
        if let Some(tab) = self.tabs.get(id.index()) {
            tab.button.selected.set(true);
        }
        self.selected.set(Some(id));
        self.on_select.call(id);
        true
    }

    /// Selects the tab labelled `name`, and reports whether the selection moved.
    ///
    /// The first tab whose label matches wins, and a name no tab carries changes
    /// nothing and reports `false` — the same rule [`TabBar::select`] gives an
    /// id past the end.
    pub fn select_name(&self, name: &str) -> bool {
        for index in 0..self.tabs.len() {
            if self.tabs[index].label == name {
                return self.select(TabId(index));
            }
        }
        false
    }

    /// Returns the selected tab, or `None` before the first selection.
    #[must_use]
    pub fn selected(&self) -> Option<TabId> {
        self.selected.get()
    }

    /// Sets the palette every tab is drawn in while it is **not** selected, and
    /// leaves the appearance where it is.
    ///
    /// The aim is the next [`TabBar::sync`]'s: this marks every tab stale, so
    /// the sync after it re-reaches all of them.
    pub fn set_palette(&mut self, palette: Palette) {
        for tab in &mut self.tabs {
            tab.button.set_palette(palette);
            tab.aimed.set(None);
        }
    }

    /// Sets the palette every tab is drawn in while it **is** selected, and
    /// leaves the appearance where it is.
    ///
    /// A selected tab's fill is the selected palette's and its ring is the
    /// selected palette's too — see [`Button::selected`] — so this is what a
    /// caller themes a bar's active tab with.
    pub fn set_selected_palette(&mut self, palette: Palette) {
        for tab in &mut self.tabs {
            tab.button.set_selected_palette(palette);
            tab.aimed.set(None);
        }
    }

    /// Sets the motion an aim runs on.
    ///
    /// A transition already in flight keeps the motion it started with; this
    /// reaches the next aim.
    pub fn set_motion(&mut self, motion: Motion) {
        self.motion = motion;
    }

    /// Puts every tab's appearance straight at its current state, with no
    /// transition, and records that nothing needs aiming.
    ///
    /// This is what a caller wants after it has set a palette, for the reason
    /// [`Button::snap_to_state`] gives: the buttons still hold the neutral
    /// colours [`Button::new`] wrote, and a themed bar would otherwise be grey
    /// until something moved it.
    pub fn snap_to_state(&self) {
        for tab in &self.tabs {
            tab.button.snap_to_state();
            tab.aimed.set(Some((
                tab.button.selected.get(),
                tab.button.hovered.get(),
                tab.button.pressed.get(),
            )));
        }
    }

    /// Returns the size the bar asks for: the sum of its tabs' content widths,
    /// the gaps between them and its own padding, by `TAB_BAR_PADDING` on the top
    /// and the bottom plus [`TAB_HEIGHT`].
    ///
    /// A bar with no tabs is a box of its padding in both directions — a strip
    /// with nothing in it is the padding and no more. `advance` and
    /// `line_height` are the measurements [`Button::content_size`] takes.
    #[must_use]
    pub fn size(&self, advance: &dyn Fn(char) -> f32, line_height: f32) -> Size {
        if self.tabs.is_empty() {
            return Size::new(TAB_BAR_PADDING * 2.0, TAB_BAR_PADDING * 2.0);
        }
        let mut width = 0.0;
        for tab in &self.tabs {
            width += tab.button.content_size(advance, line_height).width;
        }
        width +=
            TAB_SPACING * count_to_f32(self.tabs.len().saturating_sub(1)) + TAB_BAR_PADDING * 2.0;
        Size::new(width, TAB_BAR_PADDING * 2.0 + TAB_HEIGHT)
    }

    /// Gives every tab's node a tight box as wide as its label needs and
    /// [`TAB_HEIGHT`] tall, and reports whether any box moved.
    ///
    /// The width is [`Button::content_size`]'s and the height is
    /// [`TAB_HEIGHT`] outright, so a short label's tab keeps the touch floor
    /// rather than shrinking to its text. The caller passes the same `advance`
    /// and `line_height` the paint pass will, or a tab is sized for a
    /// measurement it is not drawn with.
    ///
    /// A box that has not moved is left alone, so a caller that measures every
    /// frame after a font change pays for one pass and not every one.
    pub fn measure(
        &mut self,
        nodes: &mut Arena<WidgetNode>,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> bool {
        let mut changed = false;
        for tab in &self.tabs {
            let content = tab.button.content_size(advance, line_height);
            let wanted = Constraints::tight(Size::new(content.width, TAB_HEIGHT));
            if let Some(node) = nodes.get_mut(tab.button.handle()) {
                if node.layout().constraints() != wanted {
                    node.layout_mut().set_constraints(wanted);
                    changed = true;
                }
            }
        }
        changed
    }

    /// Returns the bar's own placed rect, from the layout pass's cache, or
    /// `None` while the bar has never been laid out.
    #[must_use]
    pub fn bar_rect(&self, nodes: &Arena<WidgetNode>) -> Option<Rect> {
        node_rect(nodes, self.bar.handle())
    }

    /// Returns the placed rect of tab `id`, or `None` when there is no such tab
    /// or the pass has not placed it.
    #[must_use]
    pub fn tab_rect(&self, nodes: &Arena<WidgetNode>, id: TabId) -> Option<Rect> {
        self.tab(id)
            .and_then(|button| node_rect(nodes, button.handle()))
    }

    /// Returns the tab under `position`, or `None` when the point is not over a
    /// placed tab.
    ///
    /// It reads the same rects [`TabBar::tab_rect`] returns and the pass placed,
    /// so "which tab is under this point" has one answer rather than the bar's
    /// own arithmetic beside the pass's. The edges belong to the tab, and a
    /// point in the gap between two tabs is over neither — the same rule the
    /// keyboard's keys give their own gaps.
    #[must_use]
    pub fn tab_at(&self, nodes: &Arena<WidgetNode>, position: Offset) -> Option<TabId> {
        (0..self.tabs.len()).find_map(|index| {
            let id = TabId(index);
            self.tab_rect(nodes, id)
                .filter(|rect| covers(*rect, position))
                .map(|_| id)
        })
    }

    /// Writes the buttons' focus flags from the tab the caller says holds focus,
    /// and aims nothing.
    ///
    /// The ring is drawn from the palette at paint time, so a focus change needs
    /// no transition; this only moves the flag. The bar does not remember the
    /// focus — the caller owns that record and hands it back on every call,
    /// which is what lets the focus and the selection come apart.
    pub fn focus(&self, focused: Option<TabId>) {
        for (index, tab) in self.tabs.iter().enumerate() {
            let wanted = focused == Some(TabId(index));
            if tab.button.focused.get() != wanted {
                tab.button.focused.set(wanted);
            }
        }
    }

    /// Writes each button's hovered flag from whether it is the tab under the
    /// pointer, and reports whether any flag moved.
    ///
    /// The move is not aimed here; the next [`TabBar::sync`] carries it. A
    /// pointer already inside a tab leaves every flag as it was, so a stream of
    /// motion events aims nothing.
    pub fn hover(&self, over: Option<TabId>) -> bool {
        let mut changed = false;
        for (index, tab) in self.tabs.iter().enumerate() {
            let wanted = over == Some(TabId(index));
            if tab.button.hovered.get() != wanted {
                tab.button.hovered.set(wanted);
                changed = true;
            }
        }
        changed
    }

    /// Holds the tab at `id` down, and reports whether it moved.
    ///
    /// A finger on a tab has no press the widget can read — the gesture
    /// recogniser reports a tap on the release — so the caller says which tab
    /// went down. Only that tab's flag is set; the next [`TabBar::sync`] aims
    /// it.
    pub fn press(&self, id: TabId) -> bool {
        let Some(tab) = self.tabs.get(id.index()) else {
            return false;
        };
        if tab.button.pressed.get() {
            return false;
        }
        tab.button.pressed.set(true);
        true
    }

    /// Releases whichever tab is held down, and reports whether there was one.
    ///
    /// **One function and not an index**, because a release has to find out
    /// *whether* there was a press rather than which tab the pointer is over: a
    /// pointer that travelled off the tab between the press and the release would
    /// otherwise leave it shrunk with nothing left to bring it back.
    pub fn release(&self) -> bool {
        let mut changed = false;
        for tab in &self.tabs {
            if tab.button.pressed.get() {
                tab.button.pressed.set(false);
                changed = true;
            }
        }
        changed
    }

    /// Drains a clicked tab's request through [`TabBar::select`], then aims every
    /// tab whose selected, hovered or pressed flags have moved since the last
    /// aim, and reports whether anything was aimed.
    ///
    /// **The one aim.** Every appearance transition a tab bar runs starts here,
    /// which is what keeps a caller from restarting a transition on every frame:
    /// a sync that changes nothing aims nothing. A tab a button's click asked
    /// for is applied first, through the same [`TabBar::select`] a direct caller
    /// uses, so the two paths cannot disagree.
    ///
    /// A palette change marks every tab stale, so the sync after one re-reaches
    /// all of them; a selection change reaches exactly the two tabs it moved
    /// between.
    pub fn sync(&self) -> bool {
        if let Some(id) = self.requested.get() {
            self.requested.set(None);
            let _ = self.select(id);
        }
        let mut aimed = false;
        for tab in &self.tabs {
            let current = (
                tab.button.selected.get(),
                tab.button.hovered.get(),
                tab.button.pressed.get(),
            );
            if tab.aimed.get() != Some(current) {
                tab.button.animate_to_state(self.motion);
                tab.aimed.set(Some(current));
                aimed = true;
            }
        }
        aimed
    }

    /// Advances every tab's transitions by `delta`, and reports whether any of
    /// them wrote.
    ///
    /// It is the bar's frame integration: call it once a frame, before the paint
    /// pass. Each button's own clock is what moves, and the write marks the node
    /// dirty through the callback the caller registered, so a frame that repaints
    /// only on this repaints exactly while something moves.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        let mut moved = false;
        for tab in &self.tabs {
            if tab.button.tick(delta) {
                moved = true;
            }
        }
        moved
    }

    /// Returns the draw commands that paint the bar within its placed rect: the
    /// bar's own background, then every tab in the order they came in.
    ///
    /// The commands come from [`Container::paint`] and [`Button::paint`] over the
    /// rects the layout pass cached, so nothing is drawn at a size the pass did
    /// not place. A tab with no cached rect is painted as nothing rather than as
    /// a default box, which is what an unlaid-out bar draws. `advance` and
    /// `line_height` are the measurements a tab's label is laid out with.
    #[must_use]
    pub fn paint(
        &self,
        nodes: &Arena<WidgetNode>,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        if let Some(rect) = self.bar_rect(nodes) {
            commands.extend(self.bar.paint(rect));
        }
        for tab in &self.tabs {
            let Some(rect) = tab_rect_of(nodes, tab) else {
                continue;
            };
            commands.extend(tab.button.paint(rect, advance, line_height));
        }
        commands
    }

    /// Handles `event` as this bar would, and reports whether it consumed it.
    ///
    /// Only `Left`, `Right`, `Home`, `End` and the two horizontal d-pad buttons
    /// are consumed, and only while `focused` names a tab: `Left` and `Right`
    /// step one tab and **wrap** at both ends, `Home` and `End` jump to the ends.
    /// The move is one [`TabBar::select`], so it reports through
    /// [`TabBar::on_select`] the same way a click does.
    ///
    /// A key with no tab focused is left alone and not consumed — a bar that
    /// swallowed an arrow would take it from whatever control does hold focus.
    /// Every other event is left alone too, including the activation keys: those
    /// reach the focused tab's own [`Button`], whose click asks the bar for it.
    pub fn on_event(&self, event: &mut InputEvent, focused: Option<TabId>) -> bool {
        let InputEventKind::KeyDown { key, .. } = event.kind() else {
            return false;
        };
        let Some(current) = focused else {
            return false;
        };
        let Some(next) = navigation(&key, current, self.tabs.len()) else {
            return false;
        };
        event.consume();
        let _ = self.select(next);
        true
    }
}

/// Returns the placed rect of `tab`'s button, as the rectangle a draw command
/// carries, or `None` while the pass has not placed it.
fn tab_rect_of(nodes: &Arena<WidgetNode>, tab: &Tab) -> Option<Rect> {
    node_rect(nodes, tab.button.handle())
}

/// Returns `handle`'s placed rect as the rectangle a draw command carries, or
/// `None` while the pass has not placed it.
fn node_rect(nodes: &Arena<WidgetNode>, handle: Handle) -> Option<Rect> {
    nodes
        .get(handle)
        .and_then(|node| node.layout().rect())
        .map(Rect::from)
}

/// Returns the tab a navigation key moves to, or `None` for any other key.
///
/// `Left` and `Right` wrap at both ends, so a bar is a cycle rather than a row
/// with a wall at each end. `count` is the number of tabs; an empty bar answers
/// `None` for every key.
fn navigation(key: &Key, current: TabId, count: usize) -> Option<TabId> {
    if count == 0 {
        return None;
    }
    let index = current.index();
    let next = match key {
        Key::Keyboard(sdl3::keyboard::Keycode::Left)
        | Key::Gamepad(sdl3::gamepad::Button::DPadLeft) => (index + count - 1) % count,
        Key::Keyboard(sdl3::keyboard::Keycode::Right)
        | Key::Gamepad(sdl3::gamepad::Button::DPadRight) => (index + 1) % count,
        Key::Keyboard(sdl3::keyboard::Keycode::Home) => 0,
        Key::Keyboard(sdl3::keyboard::Keycode::End) => count - 1,
        _ => return None,
    };
    Some(TabId(next))
}

/// Returns whether `point` is inside `rect`, edges included.
///
/// The input module has a private point-in-rect test of its own; this is two
/// comparisons, repeated rather than made public for one caller, the way the
/// keyboard's own `covers` is.
fn covers(rect: Rect, point: Offset) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

/// Returns `count` as an `f32`.
///
/// There is no `From<usize>` for `f32` in std, so this goes through `u16` — the
/// same route [`Segmented`](crate::widgets::segmented::Segmented) takes — rather
/// than an `as` cast. A bar with more than 65,535 tabs is not a bar.
fn count_to_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Layout, LayoutState};
    use crate::node;
    use crate::property::Color;
    use std::rc::Rc;

    /// A monospace advance of 5 pixels per character.
    fn mono(_: char) -> f32 {
        5.0
    }

    /// A line 20 pixels tall.
    const LINE: f32 = 20.0;

    /// The box the fixture lays the bar out in: the gallery's own bar, 520 by 64
    /// at (664, 120).
    const BAR: Rect = Rect {
        x: 664.0,
        y: 120.0,
        width: 520.0,
        height: 64.0,
    };

    /// A four-tab bar, measured and laid out at [`BAR`], with a default
    /// appearance already snapped.
    ///
    /// The bar is placed through an `Absolute` root so its rect is the one
    /// [`BAR`] names rather than the origin: a pass lays its root out at the
    /// origin, and only an `Absolute` parent reads a declared position.
    fn fixture() -> (Arena<WidgetNode>, TabBar, Vec<Handle>) {
        let mut nodes = Arena::new();
        let mut bar = TabBar::new(&mut nodes, ["One", "Two", "Three", "Four"]);
        let handles: Vec<Handle> = bar.tabs.iter().map(|tab| tab.button.handle()).collect();
        assert!(bar.measure(&mut nodes, &mono, LINE), "a fresh bar measures");

        let root = node::create(
            &mut nodes,
            LayoutState::new().with_mode(LayoutMode::Absolute),
        );
        assert!(node::attach(&mut nodes, root, bar.handle()));
        if let Some(node) = nodes.get_mut(bar.handle()) {
            node.layout_mut()
                .set_position(Some(Offset::new(BAR.x, BAR.y)));
            node.layout_mut()
                .set_constraints(Constraints::tight(Size::new(BAR.width, BAR.height)));
        }
        Layout::new(&mut nodes).layout(root, Constraints::tight(Size::new(1184.0, 720.0)));
        bar.snap_to_state();
        (nodes, bar, handles)
    }

    /// The placed rect of `id`, failing the test if the pass never placed it.
    fn rect_of(bar: &TabBar, nodes: &Arena<WidgetNode>, id: TabId) -> Rect {
        let rect = bar.tab_rect(nodes, id);
        assert!(rect.is_some(), "tab {id:?} was never placed");
        rect.unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0))
    }

    /// Runs `bar`'s transitions to rest.
    fn settle(bar: &TabBar) {
        for _ in 0..64 {
            let _ = bar.tick(Duration::from_millis(10));
        }
    }

    /// Counts a write to any property it is given, so a test can see an aim.
    fn count_moves<T: Clone + 'static>(property: &Property<T>, counter: &Rc<Cell<u32>>) {
        let counter = Rc::clone(counter);
        property.on_change(move |_| counter.set(counter.get() + 1));
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

    /// The texts a paint recorded, in order.
    fn texts(commands: &[DrawCommand]) -> Vec<String> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_tab_button_is_a_child_of_the_bar_in_the_order_the_labels_came_in() {
        let (nodes, bar, handles) = fixture();
        let children = nodes
            .get(bar.handle())
            .map(|node| node.children().to_vec())
            .unwrap_or_default();
        assert_eq!(
            children, handles,
            "the bar's children are its tabs, in order"
        );
        assert_eq!(
            bar.tab_ids(),
            vec![TabId(0), TabId(1), TabId(2), TabId(3)],
            "and an id per child"
        );
    }

    #[test]
    fn every_label_the_bar_was_given_names_exactly_one_tab() {
        let labels = ["One", "Two", "Three", "Four"];
        let (_nodes, bar, _) = fixture();
        for (index, label) in labels.iter().enumerate() {
            let named = (0..bar.tabs.len())
                .filter(|position| bar.label(TabId(*position)) == Some(*label))
                .count();
            assert_eq!(named, 1, "{label} names one tab");
            assert_eq!(bar.label(TabId(index)), Some(*label));
        }
        assert_eq!(bar.selected(), None);
    }

    #[test]
    fn a_button_that_is_not_a_tab_of_this_bar_is_never_aimed_and_never_painted_by_it() {
        let (mut nodes, bar, _) = fixture();
        let foreign = Button::new(&mut nodes, "Elsewhere");
        let rect = Rect::new(0.0, 0.0, 40.0, 40.0);
        let before = foreign.paint(rect, &mono, LINE);

        assert!(bar.select(TabId(0)));
        let _ = bar.sync();

        assert!(
            !foreign.is_animating(),
            "a button the bar does not hold is never aimed"
        );
        assert_eq!(foreign.paint(rect, &mono, LINE), before);
        assert!(
            !texts(&bar.paint(&nodes, &mono, LINE))
                .iter()
                .any(|text| text == "Elsewhere"),
            "and it is never painted by the bar"
        );
    }

    #[test]
    fn the_bar_places_its_tabs_left_to_right_with_the_gap_and_the_padding_it_declared() {
        let (nodes, bar, _) = fixture();
        let first = rect_of(&bar, &nodes, TabId(0));
        assert_eq!(
            first.x,
            BAR.x + TAB_BAR_PADDING,
            "one padding in from the left"
        );
        assert_eq!(first.y, BAR.y + TAB_BAR_PADDING, "and one from the top");

        let mut previous = first;
        for index in 1..bar.tabs.len() {
            let rect = rect_of(&bar, &nodes, TabId(index));
            assert_eq!(
                rect.x,
                previous.x + previous.width + TAB_SPACING,
                "tab {index} starts one gap past the one before"
            );
            assert_eq!(rect.y, first.y, "and every tab is on the same line");
            previous = rect;
        }
    }

    #[test]
    fn the_bar_is_exactly_as_tall_as_its_padding_and_its_tabs() {
        let (nodes, bar, _) = fixture();
        let bar_rect = bar
            .bar_rect(&nodes)
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(bar_rect.height, TAB_BAR_PADDING * 2.0 + TAB_HEIGHT);
        assert_eq!(bar_rect.height, 64.0, "ten, forty-four, ten");
        for index in 0..bar.tabs.len() {
            assert_eq!(rect_of(&bar, &nodes, TabId(index)).height, TAB_HEIGHT);
        }
    }

    #[test]
    fn every_tab_button_is_at_least_the_touch_target_floor_tall() {
        for constant in [TAB_SPACING, TAB_BAR_PADDING, TAB_HEIGHT] {
            assert!(
                constant.is_finite() && constant > 0.0,
                "a length is a length"
            );
        }
        let floor = 44.0_f32;
        assert!(TAB_HEIGHT >= floor, "the height is at the touch floor");
        let (nodes, bar, _) = fixture();
        for index in 0..bar.tabs.len() {
            let rect = rect_of(&bar, &nodes, TabId(index));
            assert!(
                rect.height >= 44.0,
                "tab {index} is {} tall, under the floor",
                rect.height
            );
        }
    }

    #[test]
    fn the_measured_size_is_the_sum_of_the_labels_the_gaps_and_the_padding() {
        // "One"/"Two" are 15 + 16 = 31 wide, "Three" 25 + 16 = 41 and "Four"
        // 20 + 16 = 36: 139 of labels, three 8-pixel gaps and 20 of padding.
        let (_nodes, bar, _) = fixture();
        assert_eq!(bar.size(&mono, LINE), Size::new(183.0, 64.0));
    }

    #[test]
    fn a_bar_with_no_tabs_is_a_box_of_its_padding_and_says_so() {
        let mut nodes = Arena::new();
        let bar = TabBar::new(&mut nodes, Vec::<String>::new());
        assert!(bar.tab_ids().is_empty());
        assert_eq!(bar.selected(), None);
        assert_eq!(
            bar.size(&mono, LINE),
            Size::new(TAB_BAR_PADDING * 2.0, TAB_BAR_PADDING * 2.0),
            "padding on both axes and no more"
        );
        assert!(bar.paint(&nodes, &mono, LINE).is_empty());
    }

    #[test]
    fn tab_at_tab_rect_and_hit_test_read_the_same_box() {
        let (nodes, bar, _) = fixture();
        for index in 0..bar.tabs.len() {
            let id = TabId(index);
            let rect = rect_of(&bar, &nodes, id);
            let centre = Offset::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            assert_eq!(
                bar.tab_at(&nodes, centre),
                Some(id),
                "the centre is the tab"
            );
        }
        assert_eq!(
            bar.tab_at(&nodes, Offset::new(BAR.x - 1.0, BAR.y + 1.0)),
            None,
            "left of the bar is not a tab"
        );
        assert_eq!(
            bar.tab_at(&nodes, Offset::new(BAR.x + BAR.width + 1.0, BAR.y)),
            None,
            "nor is right of it"
        );
        let first = rect_of(&bar, &nodes, TabId(0));
        let gap = Offset::new(
            first.x + first.width + TAB_SPACING / 2.0,
            first.y + first.height / 2.0,
        );
        assert_eq!(bar.tab_at(&nodes, gap), None, "nor is the gap between tabs");
    }

    #[test]
    fn the_bar_paints_its_background_and_then_its_tabs_in_the_bar_order() {
        let (nodes, bar, _) = fixture();
        bar.bar.background.set(Color::new(30, 30, 30, 255));
        let commands = bar.paint(&nodes, &mono, LINE);

        let bar_rect = bar
            .bar_rect(&nodes)
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(
            rounded(&commands)[0].0,
            bar_rect,
            "the bar's own background is the first command"
        );
        assert_eq!(
            texts(&commands),
            vec!["One", "Two", "Three", "Four"],
            "and the labels follow in the order the tabs came in"
        );
    }

    #[test]
    fn a_tab_with_no_cached_rect_is_painted_as_nothing_and_not_as_a_default_box() {
        let mut nodes = Arena::new();
        let bar = TabBar::new(&mut nodes, ["One", "Two"]);
        assert_eq!(
            bar.tab_rect(&nodes, TabId(0)),
            None,
            "nothing is placed yet"
        );
        assert!(
            bar.paint(&nodes, &mono, LINE).is_empty(),
            "and a bar that has never been laid out draws nothing at all"
        );
    }

    #[test]
    fn selecting_a_tab_moves_the_selection_and_reports_it_once() {
        let (_nodes, mut bar, _) = fixture();
        let reports = Rc::new(Cell::new(0));
        let counted = Rc::clone(&reports);
        bar.on_select = Callback::from_fn(move |_id: TabId| counted.set(counted.get() + 1));

        assert_eq!(bar.selected(), None);
        assert!(bar.select(TabId(1)));
        assert_eq!(bar.selected(), Some(TabId(1)));
        assert!(bar.tabs[1].button.selected.get(), "the flag is written");
        assert_eq!(reports.get(), 1, "and reported once");

        assert!(!bar.select(TabId(1)), "selecting it again is not a change");
        assert_eq!(reports.get(), 1);
    }

    #[test]
    fn selecting_the_current_tab_is_a_total_no_op_including_the_aim() {
        let (_nodes, bar, _) = fixture();
        assert!(bar.select(TabId(0)));
        assert!(bar.sync());
        settle(&bar);

        assert!(!bar.select(TabId(0)), "nothing moved");
        assert!(!bar.sync(), "so nothing is aimed");
        for tab in &bar.tabs {
            assert_eq!(
                tab.aimed.get(),
                Some((
                    tab.button.selected.get(),
                    tab.button.hovered.get(),
                    tab.button.pressed.get()
                )),
                "and the aim record still matches the flags"
            );
        }
    }

    #[test]
    fn selecting_past_the_last_tab_changes_nothing_and_says_so() {
        let (_nodes, bar, _) = fixture();
        assert!(!bar.select(TabId(4)), "there are four tabs, 0 to 3");
        assert_eq!(bar.selected(), None);
        assert!(!bar.sync());
    }

    #[test]
    fn select_name_resolves_a_label_and_refuses_one_that_is_not_a_tab() {
        let (_nodes, bar, _) = fixture();
        assert!(bar.select_name("Three"));
        assert_eq!(bar.selected(), Some(TabId(2)));
        assert!(!bar.select_name("Five"), "no tab is called Five");
        assert_eq!(bar.selected(), Some(TabId(2)), "and the selection held");
        assert!(!bar.select_name(""), "nor is one called nothing");
    }

    #[test]
    fn the_selected_tab_is_aimed_on_a_press_and_on_a_release_that_changed_no_selection() {
        let (_nodes, bar, _) = fixture();
        assert!(bar.select(TabId(2)));
        assert!(bar.sync());

        assert!(bar.press(TabId(2)));
        assert!(bar.sync(), "the press is aimed");
        assert_eq!(bar.selected(), Some(TabId(2)), "and changed no selection");

        assert!(bar.release());
        assert!(bar.sync(), "and the release is aimed");
        assert_eq!(bar.selected(), Some(TabId(2)));
        assert!(!bar.release(), "a second release has nothing to do");
    }

    #[test]
    fn a_click_through_the_buttons_own_event_selects_the_tab_and_reports_it_on_the_next_sync() {
        let (_nodes, mut bar, _) = fixture();
        let reports = Rc::new(Cell::new(0));
        let counted = Rc::clone(&reports);
        bar.on_select = Callback::from_fn(move |_id: TabId| counted.set(counted.get() + 1));

        let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(0.0, 0.0)));
        assert!(
            bar.tabs[2].button.on_event(&mut tap),
            "the button takes the tap"
        );
        assert_eq!(bar.selected(), None, "the click only asked");
        assert_eq!(reports.get(), 0, "and reported nothing yet");

        assert!(bar.sync(), "the next sync applies the request");
        assert_eq!(bar.selected(), Some(TabId(2)));
        assert_eq!(reports.get(), 1, "and reports the selection once");
    }

    #[test]
    fn an_activation_key_on_the_focused_tab_selects_it_through_the_same_path() {
        let (_nodes, bar, _) = fixture();
        bar.focus(Some(TabId(1)));
        let mut key = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));

        assert!(
            bar.tabs[1].button.on_event(&mut key),
            "the focused tab takes Enter"
        );
        assert_eq!(bar.selected(), None, "the key only asked");
        assert!(bar.sync());
        assert_eq!(bar.selected(), Some(TabId(1)));
    }

    #[test]
    fn an_unfocused_tab_declines_an_activation_key_and_lets_it_travel_on() {
        let (_nodes, bar, _) = fixture();
        let mut key = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Return));
        assert!(!bar.tabs[0].button.on_event(&mut key));
        assert!(!key.consumed(), "it travels on to whatever does hold focus");
        assert!(!bar.sync());
        assert_eq!(bar.selected(), None);
    }

    #[test]
    fn left_and_right_and_home_and_end_walk_the_selection_and_wrap_at_both_ends() {
        use sdl3::gamepad::Button as Pad;
        use sdl3::keyboard::Keycode;

        fn step(bar: &TabBar, key: Key, at: TabId) -> Option<TabId> {
            let mut event = key_down(key);
            assert!(bar.on_event(&mut event, Some(at)), "{key:?} is navigation");
            assert!(event.consumed());
            bar.selected()
        }

        let (_nodes, bar, _) = fixture();
        assert!(bar.select(TabId(0)));
        let mut at = TabId(0);
        for (key, expected) in [
            (Key::Keyboard(Keycode::Right), 1),
            (Key::Keyboard(Keycode::Right), 2),
            (Key::Keyboard(Keycode::Right), 3),
            (Key::Keyboard(Keycode::Right), 0),
            (Key::Keyboard(Keycode::Left), 3),
            (Key::Keyboard(Keycode::Home), 0),
            (Key::Keyboard(Keycode::End), 3),
            (Key::Gamepad(Pad::DPadLeft), 2),
            (Key::Gamepad(Pad::DPadRight), 3),
        ] {
            at = step(&bar, key, at).unwrap_or(at);
            assert_eq!(at, TabId(expected), "after {key:?}");
        }
    }

    #[test]
    fn an_arithmetic_key_with_no_tab_focused_is_left_for_the_focused_control() {
        let (_nodes, bar, _) = fixture();
        let mut event = key_down(Key::Keyboard(sdl3::keyboard::Keycode::Right));
        assert!(!bar.on_event(&mut event, None), "no tab holds focus");
        assert!(!event.consumed(), "so the key is not the bar's");
        assert_eq!(bar.selected(), None);
    }

    #[test]
    fn the_bar_consumes_nothing_but_the_four_navigation_keys() {
        use sdl3::gamepad::Button as Pad;
        use sdl3::keyboard::Keycode;

        let (_nodes, bar, _) = fixture();
        for key in [
            Key::Keyboard(Keycode::Left),
            Key::Keyboard(Keycode::Right),
            Key::Keyboard(Keycode::Home),
            Key::Keyboard(Keycode::End),
            Key::Gamepad(Pad::DPadLeft),
            Key::Gamepad(Pad::DPadRight),
        ] {
            let mut event = key_down(key);
            assert!(
                bar.on_event(&mut event, Some(TabId(0))),
                "{key:?} is navigation"
            );
            assert!(event.consumed());
        }
        for key in [
            Key::Keyboard(Keycode::Return),
            Key::Keyboard(Keycode::Space),
            Key::Keyboard(Keycode::A),
            Key::Keyboard(Keycode::Tab),
            Key::Gamepad(Pad::DPadUp),
            Key::Gamepad(Pad::DPadDown),
            Key::Gamepad(Pad::South),
        ] {
            let mut event = key_down(key);
            assert!(
                !bar.on_event(&mut event, Some(TabId(0))),
                "{key:?} is not the bar's"
            );
            assert!(!event.consumed());
        }
    }

    #[test]
    fn a_selection_change_aims_exactly_two_tabs_and_touches_no_other_flag() {
        let (_nodes, bar, _) = fixture();
        assert!(bar.select(TabId(0)));
        assert!(bar.sync());
        settle(&bar);

        assert!(bar.select(TabId(2)));
        assert!(bar.sync());
        let animating: Vec<bool> = bar
            .tabs
            .iter()
            .map(|tab| tab.button.is_animating())
            .collect();
        assert_eq!(
            animating,
            vec![true, false, true, false],
            "only the tab left and the tab arrived at are aimed"
        );
        assert_eq!(
            bar.tabs[1].aimed.get(),
            Some((false, false, false)),
            "an untouched tab keeps the aim it had"
        );
        assert_eq!(bar.tabs[3].aimed.get(), Some((false, false, false)));
    }

    #[test]
    fn an_aim_is_not_repeated_on_a_second_sync_that_changes_nothing() {
        let (_nodes, bar, _) = fixture();
        let writes = Rc::new(Cell::new(0u32));
        count_moves(&bar.tabs[1].button.background, &writes);
        count_moves(&bar.tabs[1].button.foreground, &writes);
        count_moves(&bar.tabs[1].button.scale, &writes);
        count_moves(&bar.tabs[1].button.opacity, &writes);

        assert!(bar.select(TabId(1)));
        assert!(bar.sync(), "the selection is aimed");
        assert_eq!(
            writes.get(),
            4,
            "one aim writes the button's four animated properties"
        );

        assert!(!bar.sync(), "a second sync that changes nothing");
        assert_eq!(writes.get(), 4, "moves no counter again");
    }

    #[test]
    fn the_focus_record_is_not_part_of_the_aimed_triple() {
        let (_nodes, bar, _) = fixture();
        assert!(!bar.sync(), "a fresh bar has nothing to aim");

        bar.focus(Some(TabId(2)));
        assert!(bar.tabs[2].button.focused.get(), "the flag moved");
        assert!(!bar.sync(), "but focus is not one of the aimed three");

        assert_eq!(
            bar.tabs[2].aimed.get(),
            Some((false, false, false)),
            "the triple is selected, hovered and pressed"
        );
        bar.focus(Some(TabId(0)));
        assert!(!bar.sync());
        bar.focus(None);
        assert!(!bar.sync(), "and taking focus away aims nothing either");
    }

    #[test]
    fn the_bar_follows_a_new_palette_and_a_new_selected_palette_on_the_next_sync() {
        let (_nodes, mut bar, _) = fixture();
        assert!(bar.select(TabId(0)));
        assert!(bar.sync());
        settle(&bar);

        let plain = Palette {
            background: Color::new(10, 20, 30, 255),
            foreground: Color::new(11, 21, 31, 255),
            ring: Color::new(12, 22, 32, 255),
        };
        let chosen = Palette {
            background: Color::new(200, 100, 50, 255),
            foreground: Color::new(201, 101, 51, 255),
            ring: Color::new(202, 102, 52, 255),
        };
        bar.set_palette(plain);
        bar.set_selected_palette(chosen);

        assert!(bar.sync(), "a palette change is aimed on the next sync");
        settle(&bar);
        assert_eq!(
            bar.tabs[0].button.background.get(),
            chosen.background,
            "the selected tab takes the selected palette"
        );
        assert_eq!(bar.tabs[0].button.foreground.get(), chosen.foreground);
        assert_eq!(
            bar.tabs[1].button.background.get(),
            plain.background,
            "and an unselected one the plain palette"
        );
    }
}
