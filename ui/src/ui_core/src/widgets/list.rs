//! The List widget: a long column of equal rows, of which only the rows on
//! screen exist.
//!
//! A list is two things that happen to live in one widget: a **mapping** from an
//! index to a place on screen, and a **free list** of arena nodes that get
//! attached to the list while their row is on screen and detached when it is
//! not. Everything here is one of those two, and the free list is the whole of
//! what "virtualised" means in this repository: a hundred rows cost three nodes,
//! not a hundred.
//!
//! # What an item is, and why it is a factory
//!
//! The task file gives `item_count` and `item_height` and says nothing about a
//! row's *content*, so the decision is this module's. There were two workable
//! designs and this is the second of them.
//!
//! The first was a **row template**: the caller gives one node, and the list
//! clones its recorded draw commands per item and translates them into place.
//! Nothing is ever allocated per item, so the free list is empty by construction
//! — and that is exactly why it does not work. Requirement 2 says "only visible
//! items are **allocated in the arena**", and a template allocates *no* items in
//! the arena, so the requirement is vacuous under it. It also cannot answer a
//! tap: hit testing walks the tree ([`input::hit_test`](crate::input::hit_test)), and a template's rows
//! are not in the tree.
//!
//! So an item is a **row factory**: the list holds an [`ItemFactory`], calls it
//! for a row that arrives with nothing on the free list, and the row is a real
//! node in the arena with its own layout and its own paint. A row scrolling out
//! of view is detached and its handle goes back on the free list; a row
//! scrolling in is popped off that list and re-attached, and the factory is not
//! asked again. [`List::sync`] is the once-a-frame reconciliation of the three
//! sets — left, stayed, arrived — and it takes the arena because **a node cannot
//! reach the arena that holds it**, the reason
//! [`Container::add_child`](crate::widgets::container::Container::add_child) and
//! [`Scroll::apply_offset`](crate::widgets::scroll::Scroll::apply_offset) both
//! take one.
//!
//! ## A row is drawn in its own coordinates
//!
//! The commands on a row node are read as **row-local**: the origin is the row's
//! top-left corner, not the window's. [`List::paint`] translates them by
//! [`item_rect`](List::item_rect) and the row lands in its slot wherever that
//! slot is. This is what lets a row be *recycled* — a row that baked in a window
//! position would be in the wrong place the moment it was attached for a
//! different index — and it is why a row's factory cannot know its index. The
//! factory is an `Fn` over the arena and nothing else, and a caller that
//! highlights a selected row writes that row's commands into the row node's
//! paint state, which it can do because [`List::visible_items`] hands the
//! handles out.
//!
//! The consequence to be careful about: [`List::paint`] **is** a row's route to
//! the screen. A caller that also calls
//! [`Renderer::draw_node`](crate::render::Renderer::draw_node) on a row handle
//! draws the row twice. The integration is one line — the list's own node is
//! where [`List::paint`]'s commands go, and the row handles are skipped in the
//! caller's draw order.
//!
//! # Which rows are visible
//!
//! [`visible_range`] is the function that decides, and its rule is that a row is
//! visible when its band and the viewport's band share a **positive-length**
//! run: a row that is half on screen counts, and a row that only *touches* the
//! band's edge does not, because it has no pixel there. With 100-tall rows in a
//! 300-tall viewport that gives rows 0, 1 and 2 at offset zero — three rows for
//! three slots — and rows 1, 2 and 3 at offset 100, where row 0's last pixel row
//! is 99 and the band's first is 100.
//!
//! That is deliberately *not*
//! [`clip_commands`]'s rule, which bounds
//! with inclusive edges so that a command one pixel inside the clip is never
//! dropped. The two are answering different questions. `clip_commands` is a
//! filter over commands that were **already recorded**, and dropping a row whose
//! last pixel row is on screen loses that row; keeping one a hair outside costs
//! nothing because it is already paid for. [`visible_range`] decides what to
//! **build**, and a node for a row with no pixel on screen costs an arena slot,
//! an attach, a layout pass and a command list to produce nothing. The two
//! cannot lose a row between them: every row [`visible_range`] returns shares a
//! positive run with the band, and `clip_commands` keeps a straddler whole, so no
//! row that shares one loses a pixel row; and every row `visible_range` leaves
//! out has no pixel in the band to draw.
//!
//! # A tap between two rows
//!
//! **Half a row is still that row.** The rows tile: row `i` occupies
//! `[i·h, (i+1)·h)` and row `i + 1` starts exactly where it ends, so inside the
//! content there is no gap for a tap to fall into, and [`index_at`] is a
//! *partition* of the content rather than a search with a margin. That includes
//! the row that is only half on screen, which is why the hit test works on the
//! **content** coordinate and not on the visible band: a tap in the twelve pixels
//! of a row's bottom half that are still on screen is that row.
//!
//! Where a tap is genuinely nobody's, it is **not consumed**. That is the ragged
//! tail below the content when the content is shorter than the viewport, and any
//! point outside the list's own rect. The event carries on up the tree, because a
//! list that swallowed a tap which hit no row would make whatever is behind it
//! unclickable — and because "did it hit a row" is then a pure function of the
//! geometry, which is what lets [`input::hit_test`](crate::input::hit_test) and
//! [`on_event`](List::on_event) agree about where a tap lands. This is the
//! opposite of a drag or a wheel notch, which are consumed even when they move
//! nothing: a scroll gesture aimed at this list *is* this list's, whereas a tap
//! is about what is under it.
//!
//! # Scrolling is `Scroll`'s
//!
//! [`on_event`](List::on_event) hands every event that is not a tap straight to
//! an embedded [`Scroll`], and the offset the list scrolls is that scroll's
//! [`Property`](crate::property::Property). There is deliberately **one** offset and not two: a
//! `scroll_offset` on the list as well as one on the scroll would be two numbers
//! that have to be written in step, and a list that scrolled one while it placed
//! rows by the other would show a gap where there is none. So the list's offset
//! is reached through [`List::scroll`], and the clamp, the wheel step, the sign
//! of a drag, the arrow-key step and the scrollbar all come from that one widget
//! rather than from a second copy of any of them.
//!
//! # Clipping
//!
//! [`List::clip_rect`] is the rect a renderer would scissors to, derived from
//! [`visible_rect`] rather than from a
//! second piece of intersection arithmetic, and [`List::paint`] drops the
//! commands that are wholly outside it. That is requirement 5's "items outside
//! the viewport are not drawn", and it is the half that needs no scissor.
//! **Real per-node clipping is not here and no scissor is set.** A
//! [`DrawCommand`] carries no scissor state of its own and
//! [`Renderer::set_scissor`](crate::render::Renderer::set_scissor) applies to the
//! whole frame; `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the spec, and
//! why* records that as a deferral.
//!
//! # What is deliberately not here
//!
//! - **Variable item heights, a grid, pull-to-refresh and sticky headers.** All
//!   four are the task file's *Out of Scope*. The arithmetic in
//!   [`visible_range`] is a single `index * item_height`, which is the whole of
//!   why a variable height is a different function and not a parameter.
//! - **Momentum.** [`Scroll::on_event`] has none and this does not add any.
//! - **A row's appearance.** [`List::new`] installs no factory, so a list whose
//!   caller has not given one **draws no rows** — it still draws its scrollbar,
//!   because that is [`Scroll`]'s and a list with a hundred rows in a
//!   three-hundred-pixel box does scroll. Inventing a row's colour, radius and
//!   text here would be a decision about the demo's look made in the widget, and
//!   `doc/ui/PRIMITIVES_ARCHITECTURE.md` has no theme token for a row.
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::input::{InputEvent, InputEventKind};
//! use ui_core::layout::{LayoutState, Offset};
//! use ui_core::node::{self, WidgetNode};
//! use ui_core::paint::Rect;
//! use ui_core::widgets::list::{ItemFactory, List};
//!
//! let mut nodes = Arena::new();
//! let mut list = List::new(&mut nodes, 100, 40.0);
//! list.set_item_factory(
//!     &mut nodes,
//!     ItemFactory::new(|nodes| node::create(nodes, LayoutState::new())),
//! );
//!
//! let rect = Rect::new(0.0, 0.0, 240.0, 320.0);
//! // 100 rows of 40 is 4 000 of content in a 320 viewport.
//! assert_eq!(list.max_scroll_for(rect), 3680.0);
//!
//! // Eight rows of 40 fit in 320, so eight are allocated and no more.
//! assert!(list.sync(&mut nodes, rect));
//! assert_eq!(list.visible_items().len(), 8);
//!
//! // A wheel notch scrolls towards the end, and the rows follow. The notch is
//! // **negative**: SDL reports the wheel rolling towards the user that way, and
//! // under the scrollbar convention this list adopted that is the direction that
//! // advances down the document.
//! let mut wheel = InputEvent::new(
//!     InputEventKind::Scroll { delta: Offset::new(0.0, -1.0) },
//!     Some(Offset::new(120.0, 160.0)),
//! );
//! assert!(list.on_event(&mut wheel, rect));
//! assert_eq!(list.scroll().scroll_offset.get(), 48.0);
//! assert!(list.sync(&mut nodes, rect), "and one row left, one row arrived");
//! ```

use std::ops::Range;
use std::rc::Rc;

use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind};
use crate::layout::{Constraints, LayoutMode, LayoutState, Offset, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Rect};
use crate::widgets::scroll::{self, Scroll};
use crate::widgets::scroll::{clip_commands, visible_rect};
use crate::widgets::Callback;

/// The width a list asks for when its caller gives it no width of its own.
///
/// It is the same default viewport the scroll asks for, and [`List::new`] gives
/// the node that size so the layout pass does not measure a viewport to fit its
/// own content. It is a constant of the scroll's and not a theme token for the
/// reason `WHEEL_STEP` and the sizing constants in the other widgets are: the
/// theme has no token for a viewport's size.
const DEFAULT_VIEWPORT_WIDTH: f32 = 240.0;

/// The height a list asks for when its caller gives it none of its own. See
/// `DEFAULT_VIEWPORT_WIDTH`.
const DEFAULT_VIEWPORT_HEIGHT: f32 = 320.0;

/// What an [`ItemFactory`] holds: a closure that creates one row node in the
/// arena and hands back its handle.
///
/// It is a type alias rather than a spelled-out trait object so that
/// [`ItemFactory`]'s own definition reads as a newtype over a name — and so that
/// the field's type is not a nest of generics for `clippy::type_complexity` to
/// complain about, or for the next reader to parse.
type RowBuilder = dyn Fn(&mut Arena<WidgetNode>) -> Handle;

/// The factory that builds a list's rows.
///
/// It is a newtype over an `Option<Rc<dyn Fn(&mut Arena<WidgetNode>) -> Handle>>`
/// for the reason [`Callback`] is: this is a field a caller writes, and a newtype
/// is where its documentation and its two constructors live. An unset factory is
/// not an error and not a panic — it is what [`List::new`] installs, and a list
/// with no factory builds no rows and therefore draws none.
///
/// The closure takes the arena because it has to create a node in it, and
/// **returns the handle** rather than the node, because a handle is what
/// addresses the arena. It is given no index and no geometry: a row is built once
/// and then recycled for whatever row comes next, so a factory that baked either
/// in would be wrong the moment the row was reused. See
/// [`translate_commands`] for how a row's recorded commands are placed.
pub struct ItemFactory(Option<Rc<RowBuilder>>);

impl ItemFactory {
    /// Returns a factory that builds nothing, which is what a list holds until a
    /// caller gives it one.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::list::ItemFactory;
    ///
    /// let mut nodes: Arena<WidgetNode> = Arena::new();
    /// let factory = ItemFactory::none();
    /// assert!(!factory.is_set());
    /// assert!(factory.build(&mut nodes).is_none());
    /// assert_eq!(nodes.len(), 0, "and nothing was allocated for it");
    /// ```
    #[must_use]
    pub fn none() -> Self {
        ItemFactory(None)
    }

    /// Returns a factory that runs `build` for each row that arrives with
    /// nothing on the free list.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node::{self, WidgetNode};
    /// use ui_core::widgets::list::ItemFactory;
    ///
    /// let mut nodes = Arena::new();
    /// let factory = ItemFactory::new(|nodes| node::create(nodes, LayoutState::new()));
    /// assert!(factory.is_set());
    /// let row = factory.build(&mut nodes).expect("a factory with a closure builds");
    /// assert_eq!(nodes.len(), 1);
    /// assert!(nodes.is_valid(row));
    /// ```
    #[must_use]
    pub fn new<F>(build: F) -> Self
    where
        F: Fn(&mut Arena<WidgetNode>) -> Handle + 'static,
    {
        ItemFactory(Some(Rc::new(build)))
    }

    /// Returns whether a factory is set.
    #[must_use]
    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// Builds a row in `nodes` and returns its handle, or `None` if no factory is
    /// set.
    ///
    /// A caller asking a factory-less list for a row gets `None` and not a
    /// panic, because a list with no factory is a list that draws nothing and
    /// that is an ordinary thing to have built.
    pub fn build(&self, nodes: &mut Arena<WidgetNode>) -> Option<Handle> {
        self.0.as_ref().map(|build| build(nodes))
    }
}

/// Cloning a factory shares the closure rather than copying it, and needs no
/// bound: what is cloned is the `Rc`.
impl Clone for ItemFactory {
    fn clone(&self) -> Self {
        ItemFactory(self.0.clone())
    }
}

impl Default for ItemFactory {
    fn default() -> Self {
        ItemFactory::none()
    }
}

/// A virtualised list: a fixed set of equal rows, of which only the visible ones
/// are in the tree.
///
/// The widget holds the two things the task gives it that are *measurements* —
/// [`item_count`](List::item_count) and [`item_height`](List::item_height) — and
/// the two that are not: [`on_item_click`](List::on_item_click) and the
/// [`scroll_offset`](List::scroll) its embedded [`Scroll`] owns. The count and
/// the height are plain fields behind setters rather than properties, for
/// [`Slider`](crate::widgets::slider::Slider)'s reason: they are the **mapping**
/// from an index to a place, not an appearance, nothing animates a row's height,
/// and a property the caller wrote would need the setters anyway to release the
/// rows the change made impossible.
///
/// The two setters take the arena, the way
/// [`Container::set_mode`](crate::widgets::container::Container::set_mode) does,
/// because they have to keep the list consistent with itself: a list that has
/// just lost its last item has a node to give back, and **a node cannot reach
/// the arena that holds it**.
///
/// The free list, the live rows and the factory are private, and
/// [`visible_items`](List::visible_items) and
/// [`free_len`](List::free_len) are the read-only windows onto them, so that a
/// caller can watch the recycling without being able to break it.
///
/// [`item_count`]: List::item_count
/// [`item_height`]: List::item_height
/// [`on_item_click`]: List::on_item_click
/// [`scroll_offset`]: List::scroll
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::list::List;
///
/// let mut nodes: Arena<WidgetNode> = Arena::new();
/// let list = List::new(&mut nodes, 2, 100.0);
/// assert_eq!(list.item_count(), 2);
/// assert_eq!(list.item_height(), 100.0);
/// assert!(!list.on_item_click.is_set(), "no handler until one is given");
/// assert!(
///     list.paint(&nodes, Rect::new(0.0, 0.0, 240.0, 320.0)).is_empty(),
///     "no factory, so no row, and two rows in a 320 viewport do not scroll"
/// );
/// ```
pub struct List {
    /// The callback a tap on a row fires, with the **index** of the row.
    ///
    /// It is the shared [`Callback<usize>`](crate::widgets::Callback), which is
    /// what a [`slider`](crate::widgets::slider::Slider)'s change notification
    /// made general. The index is the row's own, not its place on screen: a
    /// caller that wants a row's rect asks
    /// [`item_rect`](List::item_rect).
    ///
    /// A tap that hits no row fires nothing and is not consumed — see the module
    /// docs on a tap between two rows.
    pub on_item_click: Callback<usize>,
    item_count: usize,
    item_height: f32,
    factory: ItemFactory,
    /// The rows currently in the tree, by index, ascending.
    live: Vec<(usize, Handle)>,
    /// The rows detached and waiting to be used again.
    free: Vec<Handle>,
    /// The width the live rows' constraints were last written with, or `None`
    /// before [`List::sync`] has run once.
    row_width: Option<f32>,
    scroll: Scroll,
}

impl List {
    /// Creates a list of `item_count` rows `item_height` tall in the arena, and
    /// returns it.
    ///
    /// It builds **no rows**: the rows are built by [`sync`](List::sync), on the
    /// first frame, for the rows that are on screen. A constructor that allocated
    /// a hundred nodes for a hundred items would be the widget this one exists
    /// not to be.
    ///
    /// The node is the embedded [`Scroll`]'s, so the list's box is a viewport and
    /// its offset is that scroll's. It is given a tight default box for the
    /// reason a scroll's is: a viewport sized to fit its own content is a
    /// viewport that does not scroll. A caller that gives the node a box of its
    /// own replaces that, through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`size`](List::size) is the number that goes in it.
    ///
    /// `item_height` is floored at zero, exactly as
    /// [`Scroll::set_content_height`](crate::widgets::scroll::Scroll::set_content_height)
    /// floors a height: an extent cannot be negative. A zero height is a list
    /// with no geometry, which scrolls nowhere, allocates nothing and draws
    /// nothing — an honest answer rather than a failure mode to paper over.
    ///
    /// The task file's `List::new(item_count, item_height) -> Handle` is read the
    /// way task 12's [`Button::new`](crate::widgets::button::Button::new) and
    /// task 14's [`Slider::new`](crate::widgets::slider::Slider::new) were read:
    /// the handle is [`handle`](List::handle)'s, and returning it alone would
    /// leave a caller with no properties to set, no callback to register and no
    /// factory.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, item_count: usize, item_height: f32) -> Self {
        // The content node holds the rows and is `Absolute`, because a row's
        // position is the whole of what a list says about where it goes, and a
        // flow mode would place it where the flow put it instead.
        let content = node::create(nodes, LayoutState::new().with_mode(LayoutMode::Absolute));
        let scroll = Scroll::new(nodes, content);
        let mut list = List {
            on_item_click: Callback::none(),
            item_count,
            item_height: item_height.max(0.0),
            factory: ItemFactory::none(),
            live: Vec::new(),
            free: Vec::new(),
            row_width: None,
            scroll,
        };
        // The scroll's content height is this list's content height, and the
        // setters are the only things that can change either, so they are the
        // only things that have to write it. `on_event` reads the scroll's.
        list.scroll.set_content_height(list.content_height());
        list
    }

    /// Returns the list's node in the arena, which is its [`Scroll`]'s node.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.scroll.handle()
    }

    /// Returns the scroll this list scrolls with.
    ///
    /// It is where [`scroll_offset`](Scroll::scroll_offset),
    /// [`focused`](Scroll::focused) and the scrollbar's
    /// [`Palette`](crate::widgets::scroll::Palette) live, and it is exposed
    /// rather than copied onto the list because **one offset is the only honest
    /// number here**: a second `Property<f32>` on the list would be a second truth
    /// to write in step with the first, and a list that scrolled one while it
    /// placed rows by the other would show a gap where there is none.
    #[must_use]
    pub fn scroll(&self) -> &Scroll {
        &self.scroll
    }

    /// Sets the colours the list's scrollbar draws with, and leaves the current
    /// ones where they are.
    ///
    /// It is here rather than left to `scroll().set_palette(..)` because
    /// [`scroll`](List::scroll) hands out a **shared** reference and
    /// [`Scroll::set_palette`] needs a mutable one — a list has exactly one
    /// scrollbar, and a caller should not have to reach inside a `Scroll` it
    /// cannot get at. Everything else about the scrollbar's appearance is
    /// already reachable, because [`Scroll::snap_to_state`] and
    /// [`Scroll::animate_to_state`] take `&self`: a caller announces the new
    /// palette with this and then moves the scrollbar toward it through
    /// [`scroll`](List::scroll), which is the arrangement every other widget in
    /// this repository uses.
    ///
    /// Without this door a caller writes the two colour properties directly. That
    /// themes the scrollbar but takes the animation out of a theme switch,
    /// because there is no palette for the transition to aim at — which is a
    /// small thing to see and an annoying one to explain.
    pub fn set_palette(&mut self, palette: crate::widgets::scroll::Palette) {
        self.scroll.set_palette(palette);
    }

    /// Sets how thick the list's scrollbar draws, in pixels, and returns what it
    /// set.
    ///
    /// It is here rather than left to `scroll().set_thickness(..)` for exactly
    /// the reason [`set_palette`](List::set_palette) gives:
    /// [`scroll`](List::scroll) hands out a **shared** reference and the setter
    /// needs a mutable one. A list has exactly one scrollbar, and a caller
    /// should not have to reach inside a `Scroll` it cannot get at.
    ///
    /// It is a measurement rather than a theme token, so this is a
    /// construction-time call and not part of a theme switch: a theme that
    /// changed it would be a theme with a token for a scrollbar's thickness,
    /// which is the addition
    /// [`Scroll::set_thickness`](crate::widgets::scroll::Scroll::set_thickness)'s
    /// own doc declines to make.
    pub fn set_scrollbar_thickness(&mut self, thickness: f32) -> f32 {
        self.scroll.set_thickness(thickness)
    }

    /// Returns the node the rows are attached to.
    ///
    /// It is [`Scroll::content`], and a caller attaching a child of its own to it
    /// is taking a place in the list's own set: the row it adds will not be in
    /// [`visible_items`](List::visible_items), will not be recycled, and will be
    /// laid out by the pass at whatever position it declares.
    #[must_use]
    pub fn content(&self) -> Handle {
        self.scroll.content
    }

    /// Returns how many rows the list has.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Sets how many rows the list has, and returns what it set.
    ///
    /// A list that shrinks below the number of rows it has allocated **gives the
    /// extras back**: every live row whose index the new count has dropped is
    /// detached and its handle goes on the free list, so a list that goes from a
    /// hundred rows to three holds three live rows and a free list, not a hundred
    /// nodes attached to nothing.
    ///
    /// The arena is taken because the release is a change to the tree, and a node
    /// cannot reach the arena that holds it. Nothing is *built* here: the rows the
    /// new count needs are built by the next [`sync`](List::sync), which is the
    /// call that knows the viewport.
    ///
    /// The scroll's content height follows the new count at once, so a drag or a
    /// wheel notch on the frame before the next `sync` is already clamped against
    /// the new geometry.
    pub fn set_item_count(&mut self, nodes: &mut Arena<WidgetNode>, count: usize) -> usize {
        self.item_count = count;
        self.scroll.set_content_height(self.content_height());
        let leaving: Vec<Handle> = self
            .live
            .iter()
            .filter(|(index, _)| *index >= count)
            .map(|(_, handle)| *handle)
            .collect();
        self.live.retain(|(index, _)| *index < count);
        for handle in leaving {
            let _ = self.recycle(nodes, handle);
        }
        count
    }

    /// Returns how tall one row is, in pixels.
    #[must_use]
    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    /// Sets how tall one row is, in pixels, with every side floored at zero, and
    /// returns what it set.
    ///
    /// A negative height is floored at zero for the reason
    /// [`Scroll::set_content_height`](crate::widgets::scroll::Scroll::set_content_height)
    /// gives: an extent cannot be negative, and a negative one would put every
    /// row after the first above the top of the screen.
    ///
    /// The live rows are re-sized at once rather than on the next
    /// [`sync`](List::sync), because a row that keeps the height it had is a row
    /// of the wrong shape until something notices — and the arena is needed to
    /// write it, because a node cannot reach the arena that holds it.
    pub fn set_item_height(&mut self, nodes: &mut Arena<WidgetNode>, height: f32) -> f32 {
        self.item_height = height.max(0.0);
        self.scroll.set_content_height(self.content_height());
        if let Some(width) = self.row_width {
            self.apply_row_sizes(nodes, width);
        }
        self.item_height
    }

    /// Returns the factory that builds this list's rows.
    #[must_use]
    pub fn item_factory(&self) -> &ItemFactory {
        &self.factory
    }

    /// Sets the factory that builds this list's rows, and gives back every row
    /// the **old** one built.
    ///
    /// Recycling a row across factories would draw the old factory's row in the
    /// new one's place, so a change of factory removes every node the list
    /// created — live and free — from the arena rather than detaching it. Their
    /// handles stop resolving, because [`Arena::remove`] bumps their generation,
    /// and the next [`sync`](List::sync) builds fresh rows.
    ///
    /// The arena is taken for the same reason as every other door in this widget:
    /// removing a node from the arena is the only way to give it back.
    pub fn set_item_factory(&mut self, nodes: &mut Arena<WidgetNode>, factory: ItemFactory) {
        let _ = self.release_all(nodes);
        self.factory = factory;
    }

    /// Detaches and removes every row this list has allocated, and returns how
    /// many that was.
    ///
    /// Both the live rows and the free list go, because a caller asking for the
    /// list's nodes back is asking for all of them: a free row is a node the
    /// arena is still holding, and holding a row nobody will ever use is the one
    /// thing virtualisation is supposed to prevent.
    ///
    /// A subsequent [`sync`](List::sync) builds the rows the viewport needs again,
    /// so this is for a caller tearing a list down rather than for one hiding it.
    #[must_use]
    pub fn release_all(&mut self, nodes: &mut Arena<WidgetNode>) -> usize {
        let mut handles: Vec<Handle> = self.live.iter().map(|(_, handle)| *handle).collect();
        handles.append(&mut self.free);
        for &handle in &handles {
            let _ = node::detach(nodes, self.scroll.content, handle);
            let _ = nodes.remove(handle);
        }
        self.live.clear();
        self.free.clear();
        handles.len()
    }

    /// Returns the rows currently in the tree, as `(index, handle)` pairs in
    /// ascending index order.
    ///
    /// It is the window onto the virtualisation: a caller can see which rows
    /// exist, and can write a selected row's commands into its paint state,
    /// because a row is reused for whatever row comes next and so cannot be told
    /// its own index by its factory.
    #[must_use]
    pub fn visible_items(&self) -> &[(usize, Handle)] {
        &self.live
    }

    /// Returns how many detached rows are on the free list, waiting to be
    /// attached again.
    ///
    /// A list that has been scrolled has rows on this list; a list that has never
    /// been scrolled has none, because its rows are still in the tree.
    #[must_use]
    pub fn free_len(&self) -> usize {
        self.free.len()
    }

    /// Returns the size a list asks for: `DEFAULT_VIEWPORT_WIDTH` by
    /// `DEFAULT_VIEWPORT_HEIGHT`, which is the same default viewport
    /// [`Scroll::new`] gives its node.
    ///
    /// A list is a container and the box it lives in is its caller's decision,
    /// so this is only what [`List::new`] gives the node to keep the layout pass
    /// from measuring a viewport to fit its own content. A caller that lays the
    /// node out itself writes whatever it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](List::paint) draws inside whatever it is given.
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT)
    }

    /// Returns how tall the whole list is, in pixels: `item_count` rows of
    /// `item_height`.
    #[must_use]
    pub fn content_height(&self) -> f32 {
        content_height(self.item_count, self.item_height)
    }

    /// Returns how far this list can be scrolled in `rect`, in pixels.
    ///
    /// It is [`max_scroll`](crate::widgets::scroll::max_scroll) of the viewport's
    /// height and this list's [`content_height`](List::content_height), so it is
    /// the same number the offset is clamped against — computed by `Scroll`, not
    /// by a second rule in this module.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::list::List;
    ///
    /// let mut nodes: Arena<WidgetNode> = Arena::new();
    /// let list = List::new(&mut nodes, 100, 40.0);
    /// // 100 rows of 40 is 4 000; a 320 viewport leaves 3 680 of scroll.
    /// assert_eq!(list.max_scroll_for(Rect::new(0.0, 0.0, 240.0, 320.0)), 3680.0);
    /// assert_eq!(list.content_height(), 4000.0);
    /// ```
    #[must_use]
    pub fn max_scroll_for(&self, rect: Rect) -> f32 {
        scroll::max_scroll(rect.height, self.content_height())
    }

    /// Returns the offset the list is **drawn** at: the scroll's
    /// [`scroll_offset`](Scroll::scroll_offset) clamped to the range the current
    /// count, height and viewport allow.
    ///
    /// It is [`clamp_scroll`](crate::widgets::scroll::clamp_scroll) and not a
    /// local comparison, because a list and a scroll have to agree about where
    /// the end is: an offset one of them lets out of range is a row the other
    /// draws a viewport's height away from where it belongs.
    ///
    /// Every write the widget makes goes through the scroll, which clamps, so in
    /// normal use this returns the property's own value. It clamps anyway because
    /// a caller may write
    /// [`Property::set`](crate::property::Property::set) directly, and every
    /// number this widget draws a row from is read through here.
    #[must_use]
    pub fn offset(&self, rect: Rect) -> f32 {
        scroll::clamp_scroll(
            self.scroll.scroll_offset.get(),
            rect.height,
            self.content_height(),
        )
    }

    /// Returns the indices of the rows on screen in `rect`, ascending.
    ///
    /// It is [`visible_range`] of this list's own four numbers, and the half of
    /// requirement 2 that carries no cost at all: a caller can ask what is on
    /// screen without touching the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::list::List;
    ///
    /// let mut nodes: Arena<WidgetNode> = Arena::new();
    /// let list = List::new(&mut nodes, 100, 40.0);
    /// let rect = Rect::new(0.0, 0.0, 240.0, 320.0);
    /// // 320 of viewport over 40-tall rows is eight rows.
    /// assert_eq!(list.visible_range(rect), 0..8);
    ///
    /// // 100 down, which is two and a half rows, and the half matters: row 2
    /// // shows its bottom 20 pixels and row 10 shows its top 20, so there are
    /// // nine rows rather than eight.
    /// list.scroll().scroll_offset.set(100.0);
    /// assert_eq!(list.visible_range(rect), 2..11);
    /// ```
    #[must_use]
    pub fn visible_range(&self, rect: Rect) -> Range<usize> {
        visible_range(
            self.item_count,
            self.item_height,
            self.offset(rect),
            rect.height,
        )
    }

    /// Returns the rect row `index` occupies inside `rect`, once the offset has
    /// been applied.
    ///
    /// It is `rect`'s own x and width — a vertical list does not move its rows
    /// sideways — `rect.y` plus `index * item_height` less the
    /// [`offset`](List::offset), and one row tall. It is the same arithmetic
    /// [`sync`](List::sync) hands the layout pass as each row's position, and the
    /// same arithmetic [`paint`](List::paint) translates a row's commands by, so
    /// the three cannot drift apart;
    /// `the_layout_pass_places_a_row_exactly_where_item_rect_says` is the test
    /// that says so against the pass itself.
    ///
    /// An `index` the list does not have still gets a rect: this is where *that*
    /// row would go, which is what a caller measuring a scroll position wants.
    /// Use [`item_at`](List::item_at) for the other direction.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::list::List;
    ///
    /// let mut nodes: Arena<WidgetNode> = Arena::new();
    /// let list = List::new(&mut nodes, 100, 40.0);
    /// // (664, 120) is where the demo's other widgets are, and a rect's origin is
    /// // a different number from its extent.
    /// let rect = Rect::new(664.0, 120.0, 200.0, 300.0);
    /// list.scroll().scroll_offset.set(60.0);
    ///
    /// // Row 2 is 2 * 40 down the content, and 60 of that is above the top edge.
    /// assert_eq!(list.item_rect(2, rect), Rect::new(664.0, 140.0, 200.0, 40.0));
    /// ```
    #[must_use]
    pub fn item_rect(&self, index: usize, rect: Rect) -> Rect {
        let top = rect.y + count_to_f32(index) * self.item_height - self.offset(rect);
        Rect::new(rect.x, top, rect.width, self.item_height)
    }

    /// Returns the index of the row under `position` inside `rect`, or `None` if
    /// there is no row there.
    ///
    /// `None` is the ragged tail below the content when the content is shorter
    /// than the viewport, the strip below the last visible row when the content is
    /// longer, **the scrollbar's own strip**, and any point outside the list's
    /// rect. See the module docs on a tap between two rows: a row that is half on
    /// screen is that row, and a tap that hits no row is nobody's and must not be
    /// consumed.
    ///
    /// The scrollbar is excluded because it is a control drawn over the rows, and
    /// a tap aimed at a scrollbar is not a tap aimed at whatever row is behind it.
    /// It is asked of the embedded [`Scroll`] rather than recomputed here, so the
    /// strip this rejects is the strip that is drawn — a second copy of the
    /// scrollbar's geometry in the list would be a second thing to keep in step
    /// with the scrollbar's own thickness and margin, and it would be wrong the
    /// moment a caller widened the bar. The press that *drags* it is
    /// [`Scroll::grab_thumb`](crate::widgets::scroll::Scroll::grab_thumb)'s, for
    /// the same reason: one geometry, one owner.
    ///
    /// A tap is only ever matched against the **band that is on screen**, which is
    /// [`clip_rect`](List::clip_rect) measured down the y axis, and not against the
    /// whole content. That is what stops a tap on the viewport's bottom edge from
    /// naming the row *below* the last one on screen: the rows tile end to end, so
    /// that row's first pixel is exactly at the edge. The rect test itself is
    /// inclusive on every edge, which is
    /// [`input::contains`](crate::input)'s convention and not a choice here — a
    /// tap on the last pixel row of the list is a tap on the list.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::Offset;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::list::List;
    ///
    /// let mut nodes: Arena<WidgetNode> = Arena::new();
    /// let list = List::new(&mut nodes, 2, 100.0);
    /// let rect = Rect::new(0.0, 0.0, 200.0, 300.0);
    ///
    /// // Two rows in a 300-tall viewport: the third slot is empty.
    /// assert_eq!(list.item_at(Offset::new(100.0, 99.0), rect), Some(0));
    /// assert_eq!(list.item_at(Offset::new(100.0, 100.0), rect), Some(1));
    /// assert_eq!(list.item_at(Offset::new(100.0, 250.0), rect), None);
    /// // And the viewport's bottom edge is the far side of the last row on
    /// // screen, not the first pixel of the one after it.
    /// let long = List::new(&mut nodes, 100, 100.0);
    /// assert_eq!(long.item_at(Offset::new(100.0, 299.0), rect), Some(2));
    /// assert_eq!(long.item_at(Offset::new(100.0, 300.0), rect), None);
    /// ```
    #[must_use]
    pub fn item_at(&self, position: Offset, rect: Rect) -> Option<usize> {
        if !covers(rect, position) {
            return None;
        }
        // The scrollbar is a control of its own, drawn over the right edge of
        // the rows, so a tap on it is nobody's row. Without this the strip named
        // whichever row happened to be behind it — a scrollbar a tenth of the
        // width of a 270-pixel list is six pixels of row that a press aimed at a
        // scrollbar silently activated. It is the **groove** and not the thumb
        // that is tested, because the whole strip is the scrollbar and a press
        // below the thumb has still missed a row and hit the bar.
        if self
            .scroll
            .scrollbar_rect(rect)
            .is_some_and(|bar| covers(bar, position))
        {
            return None;
        }
        // The band's bottom is the only bound `covers` does not already give: a
        // viewport taller than its content, or a list scrolled to its end, has
        // rows that begin inside the rect and are not on screen.
        let band = self.clip_rect(rect);
        if position.y >= band.y + band.height {
            return None;
        }
        index_at(
            position.y - rect.y + self.offset(rect),
            self.item_count,
            self.item_height,
        )
    }

    /// Returns the rect a renderer would scissors this list to.
    ///
    /// It is the viewport, shortened to the extent of the content that is left
    /// under the offset — which is [`visible_rect`]'s
    /// own answer about the band, in the window's coordinates rather than the
    /// content's. It is **not** applied: a [`DrawCommand`] carries no scissor
    /// state of its own and
    /// [`Renderer::set_scissor`](crate::render::Renderer::set_scissor) applies to
    /// the whole frame, which `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations
    /// from the spec, and why* records as a deferral. What [`paint`](List::paint)
    /// does with it is the half that needs no scissor, which is dropping the
    /// commands that are wholly outside it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::list::List;
    ///
    /// let mut nodes: Arena<WidgetNode> = Arena::new();
    /// let list = List::new(&mut nodes, 100, 40.0);
    /// let rect = Rect::new(664.0, 120.0, 200.0, 300.0);
    ///
    /// assert_eq!(list.clip_rect(rect), rect, "at the top, the list is its own clip");
    /// list.scroll().scroll_offset.set(60.0);
    /// assert_eq!(
    ///     list.clip_rect(rect),
    ///     rect,
    ///     "and scrolled, because the viewport has not moved — only the content has"
    /// );
    ///
    /// // A list shorter than its viewport is clipped to the content it has: two
    /// // rows of 100 is 200, and the 100-pixel tail below them is not the list's.
    /// let mut quiet: Arena<WidgetNode> = Arena::new();
    /// let short = List::new(&mut quiet, 2, 100.0);
    /// assert_eq!(short.clip_rect(rect), Rect::new(664.0, 120.0, 200.0, 200.0));
    /// ```
    #[must_use]
    pub fn clip_rect(&self, rect: Rect) -> Rect {
        let band = visible_rect(
            Rect::new(0.0, 0.0, rect.width, rect.height),
            self.content_height(),
            self.offset(rect),
        );
        // `band.height` is the viewport shortened to the content that is left
        // under the offset. Its `y` is the band's top in the *content's*
        // coordinates and is not this rect's origin: the content is drawn at
        // `rect.y - offset`, so a band moved by its own offset would be a band in
        // the wrong place. A rect's origin and a rect's extent are different
        // numbers, and this is one of the two places in this widget that has to
        // keep them apart.
        Rect::new(rect.x, rect.y, band.width, band.height)
    }

    /// Reconciles the tree with the offset, and reports whether anything moved.
    ///
    /// This is the list's once-a-frame step, and it is the half that needs the
    /// arena: a node cannot reach the arena that holds it, so a method that has
    /// to attach a row, detach a row or move the content node takes
    /// `&mut Arena<WidgetNode>` — the way
    /// [`Scroll::apply_offset`](crate::widgets::scroll::Scroll::apply_offset) and
    /// [`Container::add_child`](crate::widgets::container::Container::add_child)
    /// do. [`on_event`](List::on_event) does not take one, because an input
    /// handler is called from
    /// [`input::dispatch_event`](crate::input::dispatch_event) and re-entering the
    /// arena from there is the double borrow
    /// [`input::route`](crate::input::route) documents.
    ///
    /// It does four things, in this order:
    ///
    /// 1. **Re-clamps the offset** with the scroll's own re-clamp — a delta of
    ///    zero — so that a count, a height or a hand-written property has brought
    ///    it out of range, and then moves the content node by
    ///    [`Scroll::apply_offset`].
    /// 2. **Re-sizes the live rows** if the list's width has changed since the
    ///    last call, so that a window resize does not leave every row the width it
    ///    had.
    /// 3. **Releases** the rows whose index is not in
    ///    [`visible_range`](List::visible_range), detaching each and putting its
    ///    handle on the free list.
    /// 4. **Allocates** the rows that are visible and have none: from the free
    ///    list if there is one, and from the [`factory`](List::item_factory) only
    ///    when the free list is empty. A row that is reused is not rebuilt, and a
    ///    row that stayed is not moved.
    ///
    /// The answer is whether the tree or the offset changed, which is when a
    /// caller marks the list's node dirty and repaints.
    ///
    /// A frame is [`on_event`](List::on_event) for the offset and this for the
    /// tree, and this is the second of the two. A caller that never calls it has
    /// a list that scrolls and allocates nothing: the offset moves, the rows do
    /// not follow.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::LayoutState;
    /// use ui_core::node::{self, WidgetNode};
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::list::{ItemFactory, List};
    ///
    /// let mut nodes = Arena::new();
    /// let mut list = List::new(&mut nodes, 100, 40.0);
    /// list.set_item_factory(
    ///     &mut nodes,
    ///     ItemFactory::new(|nodes| node::create(nodes, LayoutState::new())),
    /// );
    /// let rect = Rect::new(0.0, 0.0, 240.0, 320.0);
    ///
    /// assert!(list.sync(&mut nodes, rect), "eight rows joined the tree");
    /// assert_eq!(list.visible_items().len(), 8);
    /// assert!(!list.sync(&mut nodes, rect), "and a second call changes nothing");
    /// ```
    pub fn sync(&mut self, nodes: &mut Arena<WidgetNode>, rect: Rect) -> bool {
        // A delta of zero is `Scroll`'s own re-clamp, and reading the property
        // around it is how the answer is known without a second comparison.
        let before = self.scroll.scroll_offset.get();
        let after = self.scroll.scroll_by(rect, 0.0);
        let mut changed = self.scroll.apply_offset(nodes) || after != before;

        if self.row_width != Some(rect.width) {
            self.row_width = Some(rect.width);
            self.apply_row_sizes(nodes, rect.width);
            // Only a change the caller can see: a list with no rows has no row to
            // re-size, and recording the width it last saw is bookkeeping rather
            // than a repaint. A list with rows always reports a change here
            // anyway, because the rows were just attached.
            changed = !self.live.is_empty() || changed;
        }

        let wanted = self.visible_range(rect);
        let mut live: Vec<(usize, Handle)> = Vec::with_capacity(wanted.len());
        let mut leaving: Vec<Handle> = Vec::new();
        for entry in self.live.drain(..) {
            if wanted.contains(&entry.0) {
                live.push(entry);
            } else {
                leaving.push(entry.1);
            }
        }
        for handle in leaving {
            changed = self.recycle(nodes, handle) || changed;
        }

        for index in wanted {
            if live.iter().any(|(held, _)| *held == index) {
                continue;
            }
            let Some(handle) = self.take_row(nodes) else {
                // No factory and an empty free list: there is no row to build,
                // and a list that draws nothing is not a failure.
                break;
            };
            self.place_row(nodes, handle, index, rect.width);
            if !node::attach(nodes, self.scroll.content, handle) {
                // A factory that handed back a node with a parent has given the
                // list something it cannot use, and putting that on the free list
                // would hand the same problem to the next row.
                let _ = nodes.remove(handle);
                changed = true;
                break;
            }
            live.push((index, handle));
            changed = true;
        }
        live.sort_by_key(|(index, _)| *index);
        self.live = live;
        changed
    }

    /// Returns the draw commands that paint the list within `rect`.
    ///
    /// The commands are the visible rows' own, each translated by that row's
    /// [`item_rect`](List::item_rect) and then filtered by
    /// [`clip_commands`] against
    /// [`clip_rect`](List::clip_rect); and after them the [`Scroll`]'s scrollbar.
    ///
    /// **Only the rows the list is holding are drawn**, and there are only as
    /// many of those as fit on screen. That is not only a saving on draw calls:
    /// an off-screen row is not in the tree, so the caller's
    /// [`input::hit_test`](crate::input::hit_test)(crate::input::hit_test) walk and its node walk shrink
    /// with the list too, and a hundred rows cost no hit test.
    ///
    /// **The commands on a row node are row-local** — the origin is the row's own
    /// top-left corner — and are translated here, because a row is recycled and a
    /// row that baked in a window position would be in the wrong place the moment
    /// it was attached for a different index. See the module docs.
    ///
    /// The scrollbar is recorded **after** the rows, so a row's own background
    /// cannot cover it: a [`DrawCommand::RoundedRect`] fills its rect, and a
    /// thumb drawn under an opaque row is a scrollbar nobody can see. That is the
    /// rule that a filled rounded rectangle is not an outline, one level up.
    ///
    /// A row whose node the arena no longer holds is skipped rather than
    /// reported: the only way to reach that state is for a caller to have removed
    /// it, and there is no row left to draw.
    ///
    /// This is a read. It does not write the offset, allocate a row, or take the
    /// arena mutably, so a caller that paints on a frame where it has not synced
    /// sees the rows it had.
    #[must_use]
    pub fn paint(&self, nodes: &Arena<WidgetNode>, rect: Rect) -> Vec<DrawCommand> {
        let mut commands = Vec::new();
        for (index, handle) in &self.live {
            let Some(node) = nodes.get(*handle) else {
                continue;
            };
            let origin = self.item_rect(*index, rect);
            commands.extend(translate_commands(
                node.paint().commands(),
                Offset::new(origin.x, origin.y),
            ));
        }
        let mut painted = clip_commands(&commands, self.clip_rect(rect));
        painted.extend(self.scroll.paint(rect));
        painted
    }

    /// Handles `event` as this list would inside `rect`, and reports whether it
    /// consumed it.
    ///
    /// A [`Tap`](InputEventKind::Tap) is the list's own: it is
    /// [`item_at`](List::item_at) that decides, and the row it names is reported
    /// through [`on_item_click`](List::on_item_click). A tap that hits no row is
    /// **not** consumed and fires nothing — see the module docs on a tap between
    /// two rows for why that is the opposite of what a drag does.
    ///
    /// Every other event is [`Scroll`]'s, and it is handed straight over: the
    /// wheel step, the sign of a drag, the arrow-key step and the clamp are all
    /// that widget's, and a list with its own copies of them would be a second
    /// rule for each. The offset it writes is the list's offset, because there is
    /// only one.
    ///
    /// **A frame is two steps**: this moves the offset, and
    /// [`sync`](List::sync) moves the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind};
    /// use ui_core::layout::Offset;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::list::List;
    ///
    /// let mut nodes: Arena<WidgetNode> = Arena::new();
    /// let mut list = List::new(&mut nodes, 100, 40.0);
    /// let rect = Rect::new(0.0, 0.0, 200.0, 300.0);
    ///
    /// let taps = Rc::new(RefCell::new(Vec::new()));
    /// let seen = Rc::clone(&taps);
    /// list.on_item_click = ui_core::widgets::Callback::from_fn(move |index| {
    ///     seen.borrow_mut().push(index)
    /// });
    ///
    /// // A tap 100 down is the third row of 40: rows 0 and 1 are above it.
    /// let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(100.0, 100.0)));
    /// assert!(list.on_event(&mut tap, rect));
    /// assert_eq!(taps.borrow().as_slice(), &[2]);
    /// ```
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        match event.kind() {
            InputEventKind::Tap => {
                let Some(position) = event.position() else {
                    return false;
                };
                let Some(index) = self.item_at(position, rect) else {
                    return false;
                };
                event.consume();
                self.on_item_click.call(index);
                true
            }
            _ => self.scroll.on_event(event, rect),
        }
    }

    /// Detaches `handle` from the content node and puts it on the free list.
    ///
    /// A handle the arena no longer holds is dropped rather than recycled: a free
    /// row is a node somebody can attach again, and a stale handle would make
    /// the next [`sync`](List::sync) attach nothing and believe it had.
    fn recycle(&mut self, nodes: &mut Arena<WidgetNode>, handle: Handle) -> bool {
        let detached = node::detach(nodes, self.scroll.content, handle);
        if nodes.is_valid(handle) {
            self.free.push(handle);
        }
        detached
    }

    /// Returns a row to build into: off the free list if there is one, and out of
    /// the factory if there is not.
    fn take_row(&mut self, nodes: &mut Arena<WidgetNode>) -> Option<Handle> {
        while let Some(handle) = self.free.pop() {
            if nodes.is_valid(handle) {
                return Some(handle);
            }
        }
        self.factory.build(nodes)
    }

    /// Gives a row the position and the size its index implies.
    ///
    /// The position is in the **content's** coordinates — `index * item_height`
    /// and nothing else — because the content node is what carries the offset,
    /// through [`Scroll::apply_offset`]. A row's own node therefore does not move
    /// when the list scrolls: the node above it does, and every row below it with
    /// it.
    fn place_row(&self, nodes: &mut Arena<WidgetNode>, handle: Handle, index: usize, width: f32) {
        let Some(node) = nodes.get_mut(handle) else {
            return;
        };
        node.layout_mut().set_position(Some(Offset::new(
            0.0,
            count_to_f32(index) * self.item_height,
        )));
        node.layout_mut()
            .set_constraints(Constraints::tight(Size::new(width, self.item_height)));
    }

    /// Writes the current row height and `width` onto every live row.
    fn apply_row_sizes(&mut self, nodes: &mut Arena<WidgetNode>, width: f32) {
        for (_, handle) in &self.live {
            if let Some(node) = nodes.get_mut(*handle) {
                node.layout_mut()
                    .set_constraints(Constraints::tight(Size::new(width, self.item_height)));
            }
        }
    }
}

/// Returns how tall `item_count` rows of `item_height` are, in pixels.
///
/// It is the one number the list's offset is clamped against, and it is public
/// because a caller needs it to reason about a scroll position and there is
/// nothing else in the crate that computes it. A count of zero, a height of zero
/// and a height that is not a positive finite number all give zero, which is a
/// list with no content: nothing to scroll through, nothing to show, and a
/// content height that is not a lie.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::list::content_height;
///
/// assert_eq!(content_height(100, 40.0), 4000.0);
/// assert_eq!(content_height(0, 40.0), 0.0, "no rows, no content");
/// assert_eq!(content_height(100, 0.0), 0.0, "and no height is no content");
/// assert_eq!(content_height(100, f32::NAN), 0.0);
/// ```
#[must_use]
pub fn content_height(item_count: usize, item_height: f32) -> f32 {
    // `is_positive` alone would let an infinite height through and turn the
    // content into an infinity that every arithmetic downstream then has to
    // survive. A height that is not a positive *finite* number is a list whose
    // rows have no place.
    if !is_positive(item_height) || !item_height.is_finite() {
        return 0.0;
    }
    count_to_f32(item_count) * item_height
}

/// Returns how far a list of `item_count` rows of `item_height` can be scrolled
/// in a viewport `viewport_height` tall, in pixels.
///
/// It is [`max_scroll`](crate::widgets::scroll::max_scroll) of the two, and it is
/// here so a caller can ask the question about three numbers without having to
/// build a list first. The clamp itself is not repeated: it is the scroll's.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::list::max_scroll_for;
///
/// assert_eq!(max_scroll_for(100, 40.0, 320.0), 3680.0);
/// assert_eq!(max_scroll_for(3, 100.0, 300.0), 0.0, "three rows fit exactly");
/// assert_eq!(max_scroll_for(2, 100.0, 300.0), 0.0, "and two leave a gap");
/// ```
#[must_use]
pub fn max_scroll_for(item_count: usize, item_height: f32, viewport_height: f32) -> f32 {
    scroll::max_scroll(viewport_height, content_height(item_count, item_height))
}

/// Returns the indices of the rows on screen, ascending: the rows whose band
/// overlaps the viewport's band by more than nothing.
///
/// The rule, in one line: row `i` is visible when `i * h < bottom` **and**
/// `(i + 1) * h > top`, where `h` is `item_height` and `[top, bottom)` is the
/// band [`visible_rect`] returns for
/// `scroll_offset` in a `viewport_height`-tall viewport. A row that is half on
/// screen is on screen; a row that only touches an edge has no pixel there and
/// is not.
///
/// The closed form of that predicate is `floor(top / h) ..= ceil(bottom / h) - 1`,
/// and this function computes the closed form and then steps to the nearest
/// index the predicate actually accepts, in both directions. That is not belt and
/// braces: `ceil` of a value that is a hair above a whole number is a hair above
/// that number, so a division that *should* give 3 can give 3.0000002 and push
/// the answer one row out. The predicate is the definition and the closed form is
/// the fast path, so the correction cannot change a case the closed form already
/// got right.
///
/// The predicate is evaluated in `f32`, and at a boundary that is the only thing
/// it can be: ten rows of 0.1 in a 0.3 viewport at the very end puts row 6's
/// bottom and the band's top within a hundred-millionth of a pixel of one another,
/// and which side of that the arithmetic lands on is what decides the row. It is
/// kept, and a hundred-millionth of a pixel of a row nobody can see is a cost of
/// one node.
/// `a_boundary_a_hundred_millionth_of_a_pixel_apart_is_decided_by_f32` is the test
/// that says so.
///
/// **Everything with no geometry gives an empty range**: no rows, a zero,
/// negative or `NaN` item height, a zero or `NaN` viewport. There is no row to
/// place in any of them, and a range with a guess in it would be a list placing a
/// row somewhere nobody asked for.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::list::visible_range;
///
/// // 300 of viewport over 100-tall rows: three slots, three rows.
/// assert_eq!(visible_range(100, 100.0, 0.0, 300.0), 0..3);
/// // One row down, and row 0's last pixel row is 99 where the band's is 100.
/// assert_eq!(visible_range(100, 100.0, 100.0, 300.0), 1..4);
/// // Half a row at each end: four rows, and the two ends are half on screen.
/// assert_eq!(visible_range(100, 100.0, 250.0, 300.0), 2..6);
/// // The end of 10 000 of content in a 300 viewport is 9 700.
/// assert_eq!(visible_range(100, 100.0, 9_700.0, 300.0), 97..100);
/// // An offset past the end is pinned, not obeyed.
/// assert_eq!(visible_range(100, 100.0, 20_000.0, 300.0), 97..100);
/// // A row taller than the viewport: one row, and two where they straddle.
/// assert_eq!(visible_range(100, 500.0, 0.0, 300.0), 0..1);
/// assert_eq!(visible_range(100, 500.0, 250.0, 300.0), 0..2);
/// // And no geometry at all is no rows.
/// assert_eq!(visible_range(0, 100.0, 0.0, 300.0), 0..0);
/// assert_eq!(visible_range(100, 0.0, 0.0, 300.0), 0..0);
/// assert_eq!(visible_range(100, 100.0, 0.0, 0.0), 0..0);
/// ```
#[must_use]
pub fn visible_range(
    item_count: usize,
    item_height: f32,
    scroll_offset: f32,
    viewport_height: f32,
) -> Range<usize> {
    // A row's position is `index * item_height`, so a height that is not a
    // positive finite number leaves nothing to place and nothing to divide by.
    if item_count == 0 || !is_positive(item_height) || !item_height.is_finite() {
        return 0..0;
    }
    if !is_positive(viewport_height) {
        return 0..0;
    }
    let content = content_height(item_count, item_height);
    if !is_positive(content) {
        return 0..0;
    }
    // `visible_rect` answers in the content's own coordinates and clamps the
    // offset on the way, which is requirement 3's clamp and not a second one.
    // The width is carried because the band carries one; it is not read.
    let band = visible_rect(
        Rect::new(0.0, 0.0, 0.0, viewport_height),
        content,
        scroll_offset,
    );
    let (top, bottom) = (band.y, band.y + band.height);
    if !is_positive(bottom - top) {
        return 0..0;
    }

    let mut first = floor_to_usize(top / item_height);
    let mut last = ceil_to_usize(bottom / item_height).saturating_sub(1);
    if last >= item_count {
        last = item_count - 1;
    }
    // `overlaps` is monotone in the index — its first half stops being true and
    // its second half starts — so the run is contiguous and each of these walks
    // stops after at most one step in exact arithmetic.
    while first > 0 && overlaps(first - 1, item_height, top, bottom) {
        first -= 1;
    }
    while first < item_count && !overlaps(first, item_height, top, bottom) {
        first += 1;
    }
    while last < item_count - 1 && overlaps(last + 1, item_height, top, bottom) {
        last += 1;
    }
    while last > 0 && !overlaps(last, item_height, top, bottom) {
        last -= 1;
    }
    if last < first {
        return 0..0;
    }
    first..last + 1
}

/// Returns the index of the row `content_y` pixels down the content falls in, or
/// `None` if it falls in no row.
///
/// `content_y` is in the **content's** own coordinates, which is what makes this
/// a *partition* of the content rather than a search with a margin: the rows tile
/// `[0, content_height)` end to end, so every point strictly inside is in exactly
/// one row and no point in the content is in two or in none. The two ends are
/// `None` — a point above the first row and a point below the last are not rows,
/// and neither is a point past the content's end however near it is.
///
/// There is no "between two rows" case inside the content, and that is the answer
/// to the question rather than an accident: row `i` ends where row `i + 1`
/// begins, so half a row is still that row, including a row that is only half on
/// screen. The gap a caller can hit is the ragged tail of a viewport taller than
/// the content, and this returns `None` for it.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::list::index_at;
///
/// // 100-tall rows: row 0 is [0, 100) and row 1 is [100, 200).
/// assert_eq!(index_at(0.0, 100, 100.0), Some(0));
/// assert_eq!(index_at(99.999, 100, 100.0), Some(0), "the last pixel of row 0");
/// assert_eq!(index_at(100.0, 100, 100.0), Some(1), "and the first of row 1");
/// assert_eq!(index_at(250.0, 100, 100.0), Some(2));
///
/// assert_eq!(index_at(-1.0, 100, 100.0), None, "above the first row");
/// assert_eq!(index_at(10_000.0, 100, 100.0), None, "past the last one");
/// assert_eq!(index_at(50.0, 0, 100.0), None, "and a list with no rows");
/// assert_eq!(index_at(50.0, 100, 0.0), None, "of no height");
/// ```
#[must_use]
pub fn index_at(content_y: f32, item_count: usize, item_height: f32) -> Option<usize> {
    if item_count == 0 || !is_positive(item_height) || !item_height.is_finite() {
        return None;
    }
    // The rows tile `[0, content_height)` end to end, so the whole of "is this
    // point in a row" is these two bounds and nothing else. The first refuses the
    // top — a `NaN` included, because `NaN >= 0.0` is false, which is why it is
    // written as a comparison and not as a negation — and `0.0` passes it, because
    // `0.0` is the first pixel of the first row and belongs to it. The second
    // refuses the bottom: a `content_y` past the last row divides to an index the
    // list does not have.
    //
    // An earlier version also compared `content_y` against `content_height`, and a
    // mutation of *that* line survived the whole suite, because the index bound
    // already says the same thing. Two bounds for one rule is one of them
    // untested, so the arithmetic bound is the one that stays.
    if !at_or_above_zero(content_y) {
        return None;
    }
    let slot = floor_to_usize(content_y / item_height);
    (slot < item_count).then_some(slot)
}

/// Returns `commands` moved by `by`, in the same order.
///
/// Every variant is handled and there is no catch-all arm, which is the point: a
/// [`DrawCommand`] this function does not know about is a **compile error**, not a
/// command quietly left where it was. A row's commands are row-local and this is
/// what puts them in the window, so a variant that was skipped would draw a row's
/// whole label in the corner of the list.
///
/// What moves and what does not, per variant:
///
/// | Variant | moves | does not move |
/// | --- | --- | --- |
/// | [`Rect`](DrawCommand::Rect), [`RoundedRect`](DrawCommand::RoundedRect) | the rect's `x` and `y` | its width and height, and its radius |
/// | [`Text`](DrawCommand::Text) | the run's `x` and `y`, its ramp's `start_x` and `end_x`, and its clip rect | the text, its colour, its font size and its tracking |
/// | [`Image`](DrawCommand::Image) | the quad's rect | `uv`, `opacity`, `radius` and the texture — a `uv` is in the **texture's** coordinates and has no window position to move |
/// | [`Line`](DrawCommand::Line) | both ends | its width |
/// | [`Circle`](DrawCommand::Circle) | the centre | its radius |
/// | [`Path`](DrawCommand::Path) | every point | its width and whether it is closed |
///
/// A size, a radius, a width and a `uv` are not positions, and moving one would
/// be the "a rect's origin and a rect's extent are different numbers" mistake
/// under different names.
///
/// An empty path stays an empty path. It draws nothing, and translating it would
/// change nothing about it — but **dropping** it would be this function deciding
/// that a command a caller recorded is not worth keeping, which is
/// [`clip_commands`]'s decision and not
/// this one's.
///
/// # Examples
///
/// ```
/// use ui_core::layout::Offset;
/// use ui_core::paint::{Color, DrawCommand, Rect};
/// use ui_core::widgets::list::translate_commands;
///
/// let rect = DrawCommand::Rect {
///     rect: Rect::new(0.0, 0.0, 10.0, 20.0),
///     color: Color::new(1, 2, 3, 255),
/// };
/// let moved = translate_commands(std::slice::from_ref(&rect), Offset::new(664.0, 140.0));
/// assert_eq!(
///     moved,
///     vec![DrawCommand::Rect {
///         rect: Rect::new(664.0, 140.0, 10.0, 20.0),
///         color: Color::new(1, 2, 3, 255),
///     }],
/// );
///
/// // A move of nothing is the identity, on every variant.
/// let all = [
///     rect,
///     DrawCommand::Circle { center: (5.0, 6.0), radius: 3.0, color: Color::new(0, 0, 0, 255) },
/// ];
/// assert_eq!(translate_commands(&all, Offset::ZERO), all.to_vec());
/// ```
#[must_use]
pub fn translate_commands(commands: &[DrawCommand], by: Offset) -> Vec<DrawCommand> {
    commands
        .iter()
        .map(|command| match command {
            DrawCommand::Rect { rect, color } => DrawCommand::Rect {
                rect: moved_rect(*rect, by),
                color: *color,
            },
            DrawCommand::RoundedRect {
                rect,
                radius,
                color,
            } => DrawCommand::RoundedRect {
                rect: moved_rect(*rect, by),
                radius: *radius,
                color: *color,
            },
            DrawCommand::Text {
                x,
                y,
                text,
                color,
                font_size,
                extra_advance,
                family,
                weight,
                fade,
                clip,
            } => DrawCommand::Text {
                x: x + by.x,
                y: y + by.y,
                text: text.clone(),
                color: *color,
                font_size: *font_size,
                extra_advance: *extra_advance,
                // The family travels with the command: it names the chain the run
                // is looked for in, which is a property of the text rather than of
                // where the text is on screen.
                family: *family,
                weight: *weight,
                // **The ramp moves and the text does not**, which is the same
                // split as the family's and for the same reason read the other
                // way round: `start_x` and `end_x` are positions on the screen,
                // and a ramp left behind while its run moves fades a different
                // part of the screen than the one it was recorded for — a run
                // moved 600 px to the right with its window at the old place
                // would draw at a factor of zero.
                fade: fade.map(|ramp| ramp.translated(by)),
                // **The clip moves for the same reason and because it is the one
                // field whose being left behind is a wrong picture rather than a
                // missing one**: a scissor is a box in window coordinates, so a
                // clip that did not move would cut the wrong box and the run
                // would disappear or spill.
                clip: clip.map(|rect| rect.translated(by)),
            },
            DrawCommand::Image {
                rect,
                texture,
                uv,
                opacity,
                radius,
            } => DrawCommand::Image {
                rect: moved_rect(*rect, by),
                texture: *texture,
                // `uv` is a window into the *texture*, in normalised coordinates
                // of it, and has no place in the window at all. Moving it would
                // be sampling a different part of the image.
                uv: *uv,
                opacity: *opacity,
                radius: *radius,
            },
            DrawCommand::Line {
                start,
                end,
                width,
                color,
            } => DrawCommand::Line {
                start: (start.0 + by.x, start.1 + by.y),
                end: (end.0 + by.x, end.1 + by.y),
                width: *width,
                color: *color,
            },
            DrawCommand::Circle {
                center,
                radius,
                color,
            } => DrawCommand::Circle {
                center: (center.0 + by.x, center.1 + by.y),
                radius: *radius,
                color: *color,
            },
            // Every point, because a path's position is all of its points: moving
            // the first alone would be a different polyline through the same
            // number of vertices.
            DrawCommand::Path {
                points,
                width,
                color,
                closed,
            } => DrawCommand::Path {
                points: points
                    .iter()
                    .map(|point| (point.0 + by.x, point.1 + by.y))
                    .collect(),
                width: *width,
                color: *color,
                closed: *closed,
            },
            // Every point, for the reason the path arm above gives: a polygon's
            // position is all of its points, and moving the first alone would be a
            // different shape rather than a moved one.
            DrawCommand::Polygon { points, color } => DrawCommand::Polygon {
                points: points
                    .iter()
                    .map(|point| (point.0 + by.x, point.1 + by.y))
                    .collect(),
                color: *color,
            },
            // The rect moves; the offset does not. An offset is a displacement
            // between the shadow and the thing casting it, so adding the move to
            // it as well would move the shadow twice and leave the pair
            // together — which is what a translation is for.
            //
            // **Added 2026-10-02 for `DrawCommand::Shadow`,** which is an
            // exhaustive match and could not be taught the new variant without
            // this arm. See the sub-task A handoff for the wider report.
            DrawCommand::Shadow {
                rect,
                radius,
                color,
                blur,
                offset,
            } => DrawCommand::Shadow {
                rect: moved_rect(*rect, by),
                radius: *radius,
                color: *color,
                blur: *blur,
                offset: *offset,
            },
            // The mesh is passed through unchanged, and that is a limit rather
            // than a preservation: the command's position is its `mvp`, which
            // maps to clip space rather than window pixels, so a window-space
            // move cannot be composed into it here — `translate_commands` is
            // given no viewport and no projection, and a pixel offset applied
            // to a clip-space matrix is a different picture. A mesh recorded
            // inside translated content therefore keeps its recorded transform
            // while its siblings move; the batch clip still cuts it to the
            // viewport at draw time. No widget records a mesh inside
            // translated content today, so the arm never fires — it exists
            // because the match is exhaustive.
            DrawCommand::Mesh { .. } => command.clone(),
        })
        .collect()
}

/// Returns whether `value` is greater than zero.
///
/// Spelled `> 0.0` and not `>= 0.0`, and written as a helper rather than as
/// `!(value > 0.0)` at each of its call sites, because the `!` form is the one
/// that refuses a `NaN` and `clippy::neg_cmp_op_on_partial_ord` asks for a
/// `partial_cmp` that says the same thing in twice the words. Every caller here
/// is asking "is there a length here at all", and a `NaN` has none: a caller that
/// has lost track of a measurement gets a list with no geometry rather than one
/// whose every position is `NaN`.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn is_positive(value: f32) -> bool {
    value > 0.0
}

/// Returns whether `value` is zero or greater, refusing a `NaN`.
///
/// The question this asks is "is this point inside the content", and its answer
/// for the top edge is yes: `0.0` is the first pixel of the first row. It is
/// [`is_positive`] with the strictness taken off, and it is a separate function
/// because a *length* and a *position* are different questions — a rect's origin
/// and a rect's extent are different numbers, one level down.
fn at_or_above_zero(value: f32) -> bool {
    value >= 0.0
}

/// Returns `rect` moved by `by`, with its extent untouched.
fn moved_rect(rect: Rect, by: Offset) -> Rect {
    Rect::new(rect.x + by.x, rect.y + by.y, rect.width, rect.height)
}

/// Returns whether row `index` shares more than nothing with the band.
fn overlaps(index: usize, item_height: f32, top: f32, bottom: f32) -> bool {
    let upper = count_to_f32(index) * item_height;
    upper < bottom && upper + item_height > top
}

/// Returns whether `point` is inside `rect`, edges included.
///
/// The input module has a `contains` of its own and it is private there; this is
/// two comparisons, repeated rather than made public for one caller, the way
/// `token_color` is repeated in the slider and the scroll.
fn covers(rect: Rect, point: Offset) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

/// Converts a row count to the float the position arithmetic uses.
///
/// `f32` has no `From<usize>` in std, so this is the one place a `usize`-to-`f32`
/// cast happens, and the reasoning is the layout module's `count_to_f32` — which
/// is private there, and a shared module for one four-line helper is a module. The
/// conversion is well defined for every `usize`: the result rounds to the nearest
/// `f32`, and a list with more rows than that rounding matters for is not a list
/// this arena can hold.
fn count_to_f32(count: usize) -> f32 {
    count as f32
}

/// Returns `value` rounded down, as a row index.
///
/// The counterpart of [`count_to_f32`], and an `as` cast for the same reason from
/// the other side: **std has no `TryFrom<f32>` for any integer type** — the
/// `From` impls between integers stop at the 16-bit widths and no float
/// conversion is provided at all — so there is no `TryInto` to reach for. The
/// cast is saturating rather than wrapping since Rust 1.45, so a value beyond the
/// range gives `usize::MAX` instead of zero, and `NaN` gives zero; every caller
/// here has already refused a negative and a non-finite value, and
/// `a_row_index_beyond_the_range_saturates_rather_than_wrapping` says what the
/// cast does when one gets through anyway.
fn floor_to_usize(value: f32) -> usize {
    value.floor() as usize
}

/// Returns `value` rounded up, as a row index. See [`floor_to_usize`].
fn ceil_to_usize(value: f32) -> usize {
    value.ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontSet;
    use crate::layout::{Layout, LayoutState as State};
    use crate::paint::{Color, FadeRamp, FontWeight, PaintState, Painter, TextureId, UvRect};
    use std::cell::{Cell, RefCell};

    /// The list every virtualisation test scrolls: 100 rows 100 tall in a
    /// 300-tall viewport. So 10 000 of content, 9 700 of scroll, and whole
    /// numbers at every step of the arithmetic.
    const ROWS: usize = 100;
    /// The height of one row, which is also a whole divisor of the viewport.
    const ROW: f32 = 100.0;
    /// The list's box at the origin of the window. Most of this module's
    /// geometry fixtures are here, which is the blind spot the slider fell into;
    /// `OFFSET_VIEWPORT` is the one that is not.
    const VIEWPORT: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 300.0,
    };
    /// The same list somewhere else in a window: 200 by 300 at (664, 120), which
    /// is where the demo puts its other widgets. A rect's origin and a rect's
    /// extent are different numbers, and only a list that is not at the window's
    /// top tells them apart.
    const OFFSET_VIEWPORT: Rect = Rect {
        x: 664.0,
        y: 120.0,
        width: 200.0,
        height: 300.0,
    };
    /// `ROWS` rows of `ROW`.
    const CONTENT: f32 = 10_000.0;
    /// `CONTENT` less the viewport, which is where the list stops.
    const MAX_OFFSET: f32 = 9_700.0;
    /// The extent of the rectangle a test row's own paint covers, in the row's
    /// own coordinates — a row is a template, recorded once and translated per
    /// row, exactly as `translate_commands` says.
    const ROW_WIDTH: f32 = 200.0;
    const ROW_HEIGHT: f32 = 100.0;
    /// The colour a test row's rectangle is in, so that a row cannot be
    /// confused with anything else the list records.
    const ROW_COLOR: Color = Color {
        r: 40,
        g: 80,
        b: 120,
        a: 255,
    };
    /// The window a panel-based test lays its list out in.
    const WINDOW: Size = Size {
        width: 1024.0,
        height: 600.0,
    };

    /// Writes `commands` into `handle`'s paint state.
    ///
    /// `PaintState` has no other public way to be given commands, so this is the
    /// demo's own idiom — `*node.paint_mut() = PaintState::from_commands(…)` —
    /// wrapped in a helper every factory here goes through.
    fn paint_row(nodes: &mut Arena<WidgetNode>, handle: Handle, commands: Vec<DrawCommand>) {
        if let Some(node) = nodes.get_mut(handle) {
            *node.paint_mut() = PaintState::from_commands(commands);
        }
    }

    /// Returns a factory that records one filled rectangle over the whole row, in
    /// the row's own coordinates, and counts every row it was asked for.
    ///
    /// The count is the observable that tells "the free list was used" from "the
    /// factory was used", which is the whole of requirement 2's second and third
    /// clauses.
    fn counting_factory(calls: &Rc<Cell<usize>>) -> ItemFactory {
        let counted = Rc::clone(calls);
        ItemFactory::new(move |nodes| {
            counted.set(counted.get() + 1);
            let row = node::create(nodes, State::new());
            let mut painter = Painter::new();
            painter.rect(Rect::new(0.0, 0.0, ROW_WIDTH, ROW_HEIGHT), ROW_COLOR);
            paint_row(nodes, row, painter.finish());
            row
        })
    }

    /// Returns a factory that records `commands` in the row's own coordinates,
    /// whatever they are, so a test can put a row's drawing where it likes.
    fn factory_recording(commands: Vec<DrawCommand>) -> ItemFactory {
        ItemFactory::new(move |nodes| {
            let row = node::create(nodes, State::new());
            paint_row(nodes, row, commands.clone());
            row
        })
    }

    /// A list of `count` rows `height` tall with a counting factory installed,
    /// together with the arena and the count of rows the factory has built.
    fn list(count: usize, height: f32) -> (Arena<WidgetNode>, List, Rc<Cell<usize>>) {
        let mut nodes = Arena::new();
        let mut list = List::new(&mut nodes, count, height);
        let calls = Rc::new(Cell::new(0_usize));
        list.set_item_factory(&mut nodes, counting_factory(&calls));
        (nodes, list, calls)
    }

    /// Hangs `list` on a panel at (664, 120) in a 1024 by 600 window, lays it
    /// out, and returns the window's root and the list's own rect.
    ///
    /// The holder between the root and the list is what puts the list off the
    /// origin: a child of an `Absolute` parent sits where it declares, and the
    /// list does not declare a position of its own — the caller does. The list's
    /// node is given the viewport's size explicitly, because a viewport sized to
    /// its own content is a viewport that does not scroll.
    fn on_a_panel(list: &List, nodes: &mut Arena<WidgetNode>, viewport: Size) -> (Handle, Rect) {
        let root = node::create(
            nodes,
            State::new()
                .with_mode(LayoutMode::Absolute)
                .with_constraints(Constraints::tight(WINDOW)),
        );
        let placed_at = node::create(
            nodes,
            State::new()
                .with_mode(LayoutMode::Absolute)
                .with_position(Offset::new(OFFSET_VIEWPORT.x, OFFSET_VIEWPORT.y))
                .with_constraints(Constraints::tight(viewport)),
        );
        assert!(node::attach(nodes, root, placed_at));
        assert!(node::attach(nodes, placed_at, list.handle()));
        if let Some(node) = nodes.get_mut(list.handle()) {
            node.layout_mut()
                .set_constraints(Constraints::tight(viewport));
        }
        Layout::new(nodes).layout(root, Constraints::tight(WINDOW));
        let rect = nodes
            .get(list.handle())
            .map(WidgetNode::layout)
            .and_then(State::rect)
            .map(Rect::from)
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        (root, rect)
    }

    /// Runs the layout pass over the window again, which is the step a frame
    /// takes after `List::sync` has moved nodes.
    fn relayout(nodes: &mut Arena<WidgetNode>, root: Handle) {
        Layout::new(nodes).layout(root, Constraints::tight(WINDOW));
    }

    /// Returns the indices of the rows currently in the tree.
    fn indices(list: &List) -> Vec<usize> {
        list.visible_items()
            .iter()
            .map(|(index, _)| *index)
            .collect()
    }

    /// Returns the rect of the row at `index`, as the layout pass computed it.
    fn placed(nodes: &Arena<WidgetNode>, list: &List, index: usize) -> Rect {
        let handle = list
            .visible_items()
            .iter()
            .find(|(held, _)| *held == index)
            .map(|(_, handle)| *handle)
            .expect("the row is in the tree");
        nodes
            .get(handle)
            .map(WidgetNode::layout)
            .and_then(State::rect)
            .map(Rect::from)
            .expect("the pass laid it out")
    }

    /// Returns the filled rectangles a paint recorded, in the order they were
    /// recorded.
    ///
    /// A test row paints a [`DrawCommand::Rect`] and a scrollbar paints
    /// [`DrawCommand::RoundedRect`]s, so this is the rows and only the rows.
    fn rows_painted(commands: &[DrawCommand]) -> Vec<Rect> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    /// Returns the number of scrollbar commands a paint recorded.
    fn scrollbar_commands(commands: &[DrawCommand]) -> usize {
        commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::RoundedRect { .. }))
            .count()
    }

    /// Returns the first index in `commands` that is a scrollbar command, or
    /// `None` if there is none.
    fn first_scrollbar(commands: &[DrawCommand]) -> Option<usize> {
        commands
            .iter()
            .position(|command| matches!(command, DrawCommand::RoundedRect { .. }))
    }

    /// A tap at `(x, y)`.
    fn tap_at(x: f32, y: f32) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)))
    }

    /// A drag of `delta` ending at `(x, y)`.
    fn drag_to(x: f32, y: f32, delta: Offset) -> InputEvent {
        InputEvent::new(InputEventKind::Drag { delta }, Some(Offset::new(x, y)))
    }

    /// A wheel notch of `(x, y)`, which is also what a gamepad axis arrives as.
    fn wheel(x: f32, y: f32) -> InputEvent {
        InputEvent::new(
            InputEventKind::Scroll {
                delta: Offset::new(x, y),
            },
            Some(Offset::new(100.0, 150.0)),
        )
    }

    /// A key press of `key`, which carries no position.
    fn key_down(key: crate::input::Key) -> InputEvent {
        InputEvent::new(
            InputEventKind::KeyDown {
                key,
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        )
    }

    /// Returns the indices the rows a tap on `position` would have reported.
    fn taps_on(list: &mut List, position: Offset, rect: Rect) -> Vec<usize> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorded = Rc::clone(&seen);
        list.on_item_click = Callback::from_fn(move |index| recorded.borrow_mut().push(index));
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(position));
        let _ = list.on_event(&mut tap, rect);
        let reported = seen.borrow().clone();
        reported
    }

    /// Returns the band, in the content's own coordinates, that
    /// `visible_range` saw for these four numbers.
    fn band_of(item_count: usize, item_height: f32, offset: f32, viewport: f32) -> Rect {
        let band = visible_rect(
            Rect::new(0.0, 0.0, 0.0, viewport),
            content_height(item_count, item_height),
            offset,
        );
        Rect::new(0.0, band.y, 0.0, band.height)
    }

    // ------------------------------------------------------- the visible range

    #[test]
    fn a_three_slot_viewport_shows_three_rows_at_the_top() {
        // The fixture, written out: 100 rows of 100 in a 300 viewport is 10 000
        // of content and 9 700 of scroll.
        assert_eq!(content_height(ROWS, ROW), CONTENT);
        assert_eq!(max_scroll_for(ROWS, ROW, VIEWPORT.height), MAX_OFFSET);
        assert_eq!(MAX_OFFSET, 9_700.0, "and 10 000 less 300 is 9 700");

        assert_eq!(visible_range(ROWS, ROW, 0.0, VIEWPORT.height), 0..3);
    }

    #[test]
    fn one_row_down_replaces_the_row_that_left_and_not_the_one_that_arrived() {
        // At offset 100 the band is [100, 400). Row 0 is [0, 100): its last pixel
        // row is 99 and the band's first is 100, so it shares nothing with the
        // band and is not on screen. Row 3 is [300, 400) and its bottom row is
        // the band's bottom row, so it is.
        let range = visible_range(ROWS, ROW, 100.0, VIEWPORT.height);
        assert_eq!(range, 1..4);
        assert!(!range.contains(&0), "the row that scrolled off the top");
        assert!(range.contains(&3), "and the row that came in at the bottom");
    }

    #[test]
    fn a_row_only_touching_the_edge_of_the_band_is_not_in_the_range() {
        // The decision stated on its own, because it is the one the arithmetic
        // could have gone the other way on: the band's top is 100 and row 0's
        // bottom is 100, and "touching" is not "on screen".
        let band = band_of(ROWS, ROW, 100.0, VIEWPORT.height);
        assert_eq!(band, Rect::new(0.0, 100.0, 0.0, 300.0));
        assert!(!overlaps(0, ROW, band.y, band.y + band.height));
        assert!(overlaps(1, ROW, band.y, band.y + band.height));
    }

    #[test]
    fn a_row_half_on_screen_at_each_end_is_on_screen() {
        // Offset 250 puts the band at [250, 550), so row 2 ([200, 300)) shows its
        // bottom half and row 5 ([500, 600)) shows its top half. Four rows for
        // three slots, and the two outer ones are the reason.
        let range = visible_range(ROWS, ROW, 250.0, VIEWPORT.height);
        assert_eq!(range, 2..6);
        assert_eq!(range.len(), 4);

        let band = band_of(ROWS, ROW, 250.0, VIEWPORT.height);
        for index in [2_usize, 5] {
            let upper = count_to_f32(index) * ROW;
            let shown = (upper + ROW).min(band.y + band.height) - upper.max(band.y);
            assert_eq!(shown, 50.0, "row {index} shows half of itself");
        }
    }

    #[test]
    fn the_end_of_the_list_is_its_last_three_rows() {
        // 10 000 of content less a 300 viewport is 9 700, and the band there is
        // [9700, 10000): rows 97, 98 and 99. There is no row 100.
        let range = visible_range(ROWS, ROW, MAX_OFFSET, VIEWPORT.height);
        assert_eq!(range, 97..100);
        assert!(!range.contains(&100), "a count of 100 has no row 100");
    }

    #[test]
    fn an_offset_past_the_end_is_pinned_and_not_obeyed() {
        // The clamp is `clamp_scroll`'s and not a second rule here, so the
        // numbers are the scroll's: 20 000 and a billion both land on 9 700.
        assert_eq!(visible_range(ROWS, ROW, 20_000.0, VIEWPORT.height), 97..100);
        assert_eq!(visible_range(ROWS, ROW, 1.0e9, VIEWPORT.height), 97..100);
    }

    #[test]
    fn an_offset_above_the_top_is_pinned_too() {
        assert_eq!(visible_range(ROWS, ROW, -40.0, VIEWPORT.height), 0..3);
        assert_eq!(visible_range(ROWS, ROW, f32::NAN, VIEWPORT.height), 0..3);
    }

    #[test]
    fn a_row_taller_than_the_viewport_is_one_row_and_two_where_they_straddle() {
        // 500-tall rows in a 300 viewport: at the top only row 0 reaches into the
        // band. At 250 the band is [250, 550) and row 1 ([500, 1000)) shows its
        // top 50, so there are two. At 500 the band is [500, 800) and row 0's
        // bottom is exactly the band's top, so it is gone.
        assert_eq!(visible_range(ROWS, 500.0, 0.0, 300.0), 0..1);
        assert_eq!(visible_range(ROWS, 500.0, 250.0, 300.0), 0..2);
        assert_eq!(visible_range(ROWS, 500.0, 500.0, 300.0), 1..2);
    }

    #[test]
    fn a_list_with_no_rows_has_none_to_show() {
        for offset in [0.0, 100.0, 9_700.0, f32::NAN] {
            assert_eq!(
                visible_range(0, ROW, offset, VIEWPORT.height),
                0..0,
                "no rows at offset {offset}"
            );
        }
    }

    #[test]
    fn a_row_height_that_is_not_a_positive_finite_number_shows_no_rows() {
        // `NaN` and an infinity are the two a caller can produce by dividing, and
        // both are refused: a row whose place cannot be computed is not a row.
        for height in [0.0, -100.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                visible_range(ROWS, height, 0.0, VIEWPORT.height),
                0..0,
                "a height of {height} is no geometry"
            );
            assert_eq!(content_height(ROWS, height), 0.0);
        }
    }

    #[test]
    fn a_viewport_of_no_height_shows_no_rows() {
        for height in [0.0, -10.0, f32::NAN] {
            assert_eq!(
                visible_range(ROWS, ROW, 0.0, height),
                0..0,
                "a viewport of {height} shows nothing"
            );
        }
    }

    #[test]
    fn the_range_never_names_more_rows_than_there_are_and_never_names_none_while_there_is_content()
    {
        for count in [1_usize, 2, 3, 5, 17, 100] {
            for height in [1.0_f32, 7.5, 40.0, 100.0, 333.0] {
                for viewport in [1.0_f32, 13.0, 300.0, 1000.0] {
                    let max = max_scroll_for(count, height, viewport);
                    for offset in [-100.0, 0.0, 0.5, 3.0, max / 2.0, max, max + 1.0] {
                        let range = visible_range(count, height, offset, viewport);
                        assert!(
                            range.end <= count,
                            "{count} rows of {height} in {viewport} at {offset}: \
                             {} names a row past the end",
                            range.len()
                        );
                        assert!(
                            range.start < range.end,
                            "a positive viewport over positive content always has a \
                             row in it: {count} of {height} in {viewport} at {offset}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_row_the_range_names_is_the_row_a_point_there_belongs_to() {
        // The oracle is `index_at`, a different function: a row is visible when
        // there is a point of the band that `index_at` puts in that row, which is
        // the definition of "on screen" and not a restatement of the arithmetic
        // under test.
        let awkward: &[(f32, f32)] = &[
            (0.1, 0.3),
            (0.7, 2.1),
            (0.3, 1.0),
            (1.0 / 3.0, 1.0),
            (2.5, 7.5),
            (16.0, 100.0),
            (100.0, 300.0),
        ];
        for &(item_height, viewport) in awkward {
            for count in [3_usize, 10, 100] {
                let max = max_scroll_for(count, item_height, viewport);
                for offset in [0.0, max / 3.0, max / 2.0, max] {
                    let band = band_of(count, item_height, offset, viewport);
                    // The rule itself, evaluated one row at a time, as the
                    // definition rather than as a second copy of the closed form:
                    // `index * h < bottom && (index + 1) * h > top`.
                    let brute: Vec<usize> = (0..count)
                        .filter(|&index| {
                            let upper = count_to_f32(index) * item_height;
                            upper < band.y + band.height && upper + item_height > band.y
                        })
                        .collect();
                    let got = visible_range(count, item_height, offset, viewport);
                    assert_eq!(
                        got.collect::<Vec<_>>(),
                        brute,
                        "{count} rows of {item_height} in a {viewport} viewport at {offset}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_endpoints_of_the_range_are_the_only_rows_that_overlap_the_band() {
        // The invariant the closed form and the correction step to: the row
        // before the first does not overlap, the first does, the last does, and
        // the row after the last does not. A division that rounds a hair past a
        // whole number breaks exactly this.
        for item_height in [0.1_f32, 0.3, 0.7, 1.0, 2.5, 16.0, 100.0] {
            for viewport in [0.3_f32, 1.0, 2.1, 7.5, 100.0, 300.0] {
                for count in [1_usize, 4, 9, 40, 100] {
                    let max = max_scroll_for(count, item_height, viewport);
                    for offset in [0.0, max / 4.0, max / 2.0, max * 0.99, max] {
                        let range = visible_range(count, item_height, offset, viewport);
                        let band = band_of(count, item_height, offset, viewport);
                        let (top, bottom) = (band.y, band.y + band.height);
                        assert!(
                            overlaps(range.start, item_height, top, bottom),
                            "the first row overlaps: {item_height} in {viewport} at {offset}"
                        );
                        if range.start > 0 {
                            assert!(
                                !overlaps(range.start - 1, item_height, top, bottom),
                                "and the row before it does not: {item_height} in \
                                 {viewport} at {offset}"
                            );
                        }
                        let last = range.end - 1;
                        assert!(
                            overlaps(last, item_height, top, bottom),
                            "the last row overlaps: {item_height} in {viewport} at {offset}"
                        );
                        if last + 1 < count {
                            assert!(
                                !overlaps(last + 1, item_height, top, bottom),
                                "and the row after it does not: {item_height} in \
                                 {viewport} at {offset}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_boundary_a_hundred_millionth_of_a_pixel_apart_is_decided_by_f32() {
        // The one place this module's arithmetic is not exact, pinned rather than
        // hidden. Ten rows of 0.1 — where 0.1 is the nearest `f32` to a tenth, not
        // a tenth — in a 0.3 viewport: the content is 1.0, the maximum offset is
        // 0.7, and at that offset row 6's bottom and the band's top are within a
        // hundred-millionth of a pixel of one another. In exact reals they touch,
        // which would make the row off screen; in `f32` the row's bottom lands
        // above the band's top, so it is kept. Keeping it costs one node and
        // draws a hundred-millionth of a pixel of a row, and no epsilon is applied
        // to hide the decision, because an epsilon here would be a second rule
        // deciding which rows a caller can see.
        let item_height = 0.1_f32;
        let viewport = 0.3_f32;
        let count = 10_usize;
        let max = max_scroll_for(count, item_height, viewport);
        assert_eq!(
            content_height(count, item_height),
            1.0,
            "10 * 0.1 rounds to 1.0"
        );
        assert!((max - 0.7).abs() < 1e-6, "and 1.0 less 0.3 is 0.7");

        let band = band_of(count, item_height, max, viewport);
        let row_six_top = count_to_f32(6) * item_height;
        let row_six_bottom = row_six_top + item_height;
        assert!(
            row_six_bottom - band.y > 0.0,
            "row 6's bottom is {} past the band's top at {} — a hundred-millionth \\
             of a pixel, and it is positive",
            row_six_bottom - band.y,
            band.y
        );
        assert!(row_six_bottom - band.y < 1e-6, "which really is that small");
        assert_eq!(
            visible_range(count, item_height, max, viewport),
            6..10,
            "so the row is kept, and the range is what the f32 arithmetic says"
        );
    }

    #[test]
    fn a_row_index_beyond_the_range_saturates_rather_than_wrapping() {
        // The float-to-integer conversion in this module is an `as` cast
        // because std has no `TryFrom<f32>` for any integer, so what it does
        // *past* the range is part of its contract and not an accident. Since
        // Rust 1.45 it saturates: a huge value is `usize::MAX`, and a `NaN` is
        // zero.
        assert_eq!(
            floor_to_usize(1.0e30),
            usize::MAX,
            "saturates, not wraps to 0"
        );
        assert_eq!(ceil_to_usize(1.0e30), usize::MAX);
        assert_eq!(floor_to_usize(f32::NAN), 0);
        assert_eq!(ceil_to_usize(f32::NAN), 0);
        assert_eq!(floor_to_usize(2.7), 2);
        assert_eq!(ceil_to_usize(2.1), 3);
    }

    // ------------------------------------------------------------- index_at

    #[test]
    fn the_first_and_the_last_pixel_of_a_row_are_that_row() {
        assert_eq!(index_at(0.0, ROWS, ROW), Some(0), "the first pixel");
        assert_eq!(index_at(99.999, ROWS, ROW), Some(0), "the last of row 0");
        assert_eq!(
            index_at(100.0, ROWS, ROW),
            Some(1),
            "and the first of row 1"
        );
        assert_eq!(index_at(10_000.0 - 0.001, ROWS, ROW), Some(99));
    }

    #[test]
    fn a_point_past_the_content_or_above_it_is_no_rows_row() {
        // 10 000 is the end of the content and is not in it, however near it is:
        // the rows tile [0, 10 000) and row 100 would start there.
        assert_eq!(index_at(10_000.0, ROWS, ROW), None);
        assert_eq!(index_at(1.0e9, ROWS, ROW), None);
        assert_eq!(index_at(-0.001, ROWS, ROW), None);
        assert_eq!(index_at(f32::NAN, ROWS, ROW), None, "a NaN is not a place");
    }

    #[test]
    fn a_viewport_taller_than_its_content_leaves_a_ragged_tail() {
        // Two rows of 100 in a 300 viewport: 200 of content, no scroll, and the
        // third slot has nothing in it. This is the only gap there is.
        let range = visible_range(2, ROW, 0.0, VIEWPORT.height);
        assert_eq!(range, 0..2);
        assert_eq!(index_at(250.0, 2, ROW), None, "below the last row");
        assert_eq!(index_at(199.999, 2, ROW), Some(1), "and just inside it");
    }

    #[test]
    fn the_rows_partition_the_content_with_no_gap_and_no_overlap() {
        // Ten pixels at a time across 10 000 of content: every point strictly
        // inside belongs to exactly one row, and every row is reachable. A
        // `floor` of the wrong sign, or a `<` where a `<=` belongs, leaves a gap
        // or an overlap here.
        let mut reached: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        let mut previous = 0;
        for step in 0..1_000_u16 {
            let y = f32::from(step) * 10.0 + 0.5;
            let index = index_at(y, ROWS, ROW).expect("a point inside the content");
            assert!(
                index >= previous,
                "row {index} at {y} is above row {previous}: the rows are in \
                 order down the content, so a `floor` of the wrong sign shows here"
            );
            previous = index;
            let _ = reached.insert(index);
        }
        assert_eq!(
            reached.len(),
            ROWS,
            "every one of the 100 rows was reached, so no row is a gap"
        );
    }

    #[test]
    fn a_row_height_or_a_count_of_no_rows_refuses_every_position() {
        for height in [0.0, -10.0, f32::NAN, f32::INFINITY] {
            assert_eq!(index_at(50.0, ROWS, height), None, "height {height}");
        }
        assert_eq!(index_at(50.0, 0, ROW), None, "no rows at all");
    }

    // ------------------------------------------------- the content's own numbers

    #[test]
    fn the_content_height_is_the_count_times_the_height() {
        assert_eq!(content_height(100, 40.0), 4000.0);
        assert_eq!(content_height(1, 0.5), 0.5);
        assert_eq!(content_height(0, 40.0), 0.0);
        assert_eq!(content_height(100, 0.0), 0.0);
    }

    #[test]
    fn the_maximum_scroll_is_the_scroll_modules_arithmetic_and_not_a_second_rule() {
        // The whole point of asking `scroll::max_scroll` rather than subtracting
        // here: a caller comparing the two numbers can see there is one rule.
        for (count, height, viewport) in [
            (100_usize, 40.0_f32, 320.0_f32),
            (2, 100.0, 300.0),
            (7, 13.0, 91.0),
            (0, 10.0, 10.0),
        ] {
            assert_eq!(
                max_scroll_for(count, height, viewport),
                scroll::max_scroll(viewport, content_height(count, height)),
                "{count} rows of {height} in a {viewport} viewport"
            );
        }
        assert_eq!(max_scroll_for(100, 40.0, 320.0), 3680.0);
        assert_eq!(
            max_scroll_for(2, 100.0, 300.0),
            0.0,
            "and content that fits"
        );
    }

    // ---------------------------------------------------- translating a row

    #[test]
    fn a_rect_moves_and_keeps_its_extent() {
        let command = DrawCommand::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 20.0),
            color: Color::new(1, 2, 3, 255),
        };
        let moved = translate_commands(std::slice::from_ref(&command), Offset::new(664.0, 140.0));
        assert_eq!(
            moved,
            vec![DrawCommand::Rect {
                rect: Rect::new(664.0, 140.0, 10.0, 20.0),
                color: Color::new(1, 2, 3, 255),
            }],
            "the origin moves and the extent does not — they are different numbers"
        );
    }

    #[test]
    fn a_rounded_rect_moves_and_keeps_its_radius() {
        let command = DrawCommand::RoundedRect {
            rect: Rect::new(3.0, 4.0, 30.0, 40.0),
            radius: 5.0,
            color: Color::new(4, 5, 6, 255),
        };
        assert_eq!(
            translate_commands(std::slice::from_ref(&command), Offset::new(10.0, 20.0)),
            vec![DrawCommand::RoundedRect {
                rect: Rect::new(13.0, 24.0, 30.0, 40.0),
                radius: 5.0,
                color: Color::new(4, 5, 6, 255),
            }],
            "a radius is an extent, not a position"
        );
    }

    #[test]
    fn a_text_run_moves_and_keeps_its_text_colour_size_tracking_family_and_weight() {
        // A family that is not the default one, so the assertion below is about
        // this handle and not about two defaults comparing equal.
        let mut fonts = FontSet::new();
        let heading = fonts.define_family("heading");
        let command = DrawCommand::Text {
            x: 7.0,
            y: 8.0,
            text: "row".to_string(),
            color: Color::new(7, 8, 9, 255),
            font_size: 16.0,
            extra_advance: 1.5,
            family: heading,
            weight: FontWeight::Bold,
            fade: Some(FadeRamp::new(107.0, 123.0)),
            clip: Some(Rect::new(7.0, 8.0, 120.0, 20.0)),
        };
        let moved = translate_commands(std::slice::from_ref(&command), Offset::new(664.0, 120.0));
        let DrawCommand::Text {
            x,
            y,
            text,
            color,
            font_size,
            extra_advance,
            family,
            weight,
            fade,
            clip,
        } = &moved[0]
        else {
            panic!("a text run translated into something that is not a text run");
        };
        assert_eq!((*x, *y), (671.0, 128.0), "the run's own position moves");
        assert_eq!(text, "row", "and the text is still the text");
        assert_eq!(*color, Color::new(7, 8, 9, 255));
        assert_eq!(*font_size, 16.0, "a font size is not a position");
        assert_eq!(*extra_advance, 1.5, "and neither is the tracking");
        assert_eq!(
            *weight,
            FontWeight::Bold,
            "a translated bold run is still a bold run: the face it is drawn \
             with rides in the command, so a translation that dropped it would \
             quietly move a heading into the regular weight"
        );
        assert_eq!(
            *family, heading,
            "and it keeps its family too, for the same reason: a translated run \
             that fell back to the default family would be drawn from a different \
             chain, at different widths, with nothing to say so"
        );
        assert_eq!(
            fade,
            &Some(FadeRamp::new(771.0, 787.0)),
            "**and the ramp moves with it.** Both of its edges are positions on \
             the screen, so a ramp left at 107..123 while its run went to 671 \
             fades a window the run is nowhere near — every corner of every glyph \
             would be read at a factor of zero and the row would draw as nothing"
        );
        assert_eq!(
            clip,
            &Some(Rect::new(671.0, 128.0, 120.0, 20.0)),
            "**and the clip moves with it**, and this one is a wrong picture rather \
             than a missing one: a scissor is a box in window coordinates, so a \
             clip left behind cuts the wrong box and the row either disappears or \
             spills outside its own viewport"
        );
    }

    #[test]
    fn an_image_moves_its_quad_and_keeps_its_uv() {
        // The one that would be silently wrong: a `uv` is a window into the
        // *texture*, in normalised coordinates of it, and it has no place in the
        // window at all.
        let uv = UvRect {
            u0: 0.1,
            v0: 0.2,
            u1: 0.3,
            v1: 0.4,
        };
        let command = DrawCommand::Image {
            rect: Rect::new(9.0, 10.0, 50.0, 60.0),
            texture: TextureId::new(11),
            uv,
            opacity: 0.25,
            radius: 6.0,
        };
        let moved = translate_commands(std::slice::from_ref(&command), Offset::new(664.0, 120.0));
        let DrawCommand::Image {
            rect,
            texture,
            uv: moved_uv,
            opacity,
            radius,
        } = &moved[0]
        else {
            panic!("an image translated into something that is not an image");
        };
        assert_eq!(*rect, Rect::new(673.0, 130.0, 50.0, 60.0), "the quad moves");
        assert_eq!(*moved_uv, uv, "and the window into the texture does not");
        assert_eq!(*texture, TextureId::new(11));
        assert_eq!(*opacity, 0.25);
        assert_eq!(*radius, 6.0);
    }

    #[test]
    fn a_line_moves_both_ends_and_keeps_its_width() {
        let command = DrawCommand::Line {
            start: (11.0, 12.0),
            end: (13.0, 14.0),
            width: 2.0,
            color: Color::new(10, 11, 12, 255),
        };
        let DrawCommand::Line {
            start, end, width, ..
        } = &translate_commands(std::slice::from_ref(&command), Offset::new(5.0, 7.0))[0]
        else {
            panic!("a line translated into something that is not a line");
        };
        assert_eq!(*start, (16.0, 19.0), "the first end");
        assert_eq!(*end, (18.0, 21.0), "the second end");
        assert_eq!(*width, 2.0, "and the width is not a position");
    }

    #[test]
    fn a_circle_moves_its_centre_and_keeps_its_radius() {
        let command = DrawCommand::Circle {
            center: (15.0, 16.0),
            radius: 17.0,
            color: Color::new(13, 14, 15, 255),
        };
        let DrawCommand::Circle { center, radius, .. } =
            &translate_commands(std::slice::from_ref(&command), Offset::new(1.0, 2.0))[0]
        else {
            panic!("a circle translated into something that is not a circle");
        };
        assert_eq!(*center, (16.0, 18.0));
        assert_eq!(*radius, 17.0);
    }

    #[test]
    fn a_path_moves_every_point_and_keeps_its_width_and_closure() {
        let points = vec![(18.0, 19.0), (20.0, 21.0), (22.0, 23.0)];
        let command = DrawCommand::Path {
            points: points.clone(),
            width: 1.5,
            color: Color::new(16, 17, 18, 255),
            closed: true,
        };
        let DrawCommand::Path {
            points: moved,
            width,
            closed,
            ..
        } = &translate_commands(std::slice::from_ref(&command), Offset::new(664.0, 120.0))[0]
        else {
            panic!("a path translated into something that is not a path");
        };
        assert_eq!(
            *moved,
            vec![(682.0, 139.0), (684.0, 141.0), (686.0, 143.0)],
            "every point, because a path's position is all of its points"
        );
        assert_eq!(*width, 1.5);
        assert!(*closed);
        assert_eq!(moved.len(), points.len(), "and no vertex is dropped");
    }

    #[test]
    fn a_polygon_moves_every_point_and_keeps_its_color() {
        // The path test above, for the filled primitive. A translation that moved
        // only a polygon's first point would leave a *different shape* on screen
        // rather than a moved one, and no draw-command assertion that only checks
        // the variant would see it — so this one reads the points back.
        let command = DrawCommand::Polygon {
            points: vec![(18.0, 19.0), (20.0, 21.0), (22.0, 23.0)],
            color: Color::new(19, 20, 21, 255),
        };
        let DrawCommand::Polygon { points, color } =
            &translate_commands(std::slice::from_ref(&command), Offset::new(664.0, 120.0))[0]
        else {
            panic!("a polygon translated into something that is not a polygon");
        };
        assert_eq!(
            *points,
            vec![(682.0, 139.0), (684.0, 141.0), (686.0, 143.0)],
            "every point, because a polygon's position is all of its points"
        );
        assert_eq!(*color, Color::new(19, 20, 21, 255));
    }

    #[test]
    fn every_variant_survives_a_translation() {
        let mut painter = Painter::new();
        painter.rect(Rect::new(1.0, 2.0, 10.0, 20.0), Color::new(1, 2, 3, 255));
        painter.rounded_rect(
            Rect::new(3.0, 4.0, 30.0, 40.0),
            5.0,
            Color::new(4, 5, 6, 255),
        );
        painter.text(7.0, 8.0, "row", Color::new(7, 8, 9, 255), 16.0, 1.5);
        painter.image(
            Rect::new(9.0, 10.0, 50.0, 60.0),
            TextureId::new(11),
            UvRect::full(),
            0.25,
            6.0,
        );
        painter.line((11.0, 12.0), (13.0, 14.0), 2.0, Color::new(10, 11, 12, 255));
        painter.circle((15.0, 16.0), 17.0, Color::new(13, 14, 15, 255));
        painter.path(
            &[(18.0, 19.0), (20.0, 21.0)],
            1.5,
            Color::new(16, 17, 18, 255),
            false,
        );
        painter.polygon(
            &[(25.0, 26.0), (27.0, 28.0), (29.0, 26.0)],
            Color::new(19, 20, 21, 255),
        );
        let every = painter.finish();
        assert_eq!(every.len(), 8, "one of each variant the enum has");

        let moved = translate_commands(&every, Offset::new(664.0, 120.0));
        assert_eq!(moved.len(), 8, "and one of each out: none was dropped");
        for (before, after) in every.iter().zip(moved.iter()) {
            assert_eq!(
                std::mem::discriminant(before),
                std::mem::discriminant(after),
                "a translation changed the variant, which it must never do"
            );
        }
    }

    #[test]
    fn a_translation_keeps_the_order_of_the_commands() {
        // Paint order is what decides who covers whom, and a list's rows overlap
        // each other's band, so a translation that sorted them would change the
        // picture.
        let mut painter = Painter::new();
        for shade in 0..6_u8 {
            painter.rect(
                Rect::new(f32::from(shade), 0.0, 10.0, 10.0),
                Color::new(shade, shade, shade, 255),
            );
        }
        let recorded = painter.finish();
        let moved = translate_commands(&recorded, Offset::new(100.0, 100.0));
        let want = [100.0_f32, 101.0, 102.0, 103.0, 104.0, 105.0];
        let got: Vec<f32> = rows_painted(&moved).iter().map(|rect| rect.x).collect();
        assert_eq!(
            got, want,
            "every rect moved by the same 100, so the order is the one it was \
             recorded in: a translation that sorted them would change the picture"
        );
    }

    #[test]
    fn an_empty_path_is_kept_rather_than_dropped() {
        // It draws nothing and translating it changes nothing, but *dropping* it
        // would be this function deciding that a command a caller recorded is not
        // worth keeping, which is `clip_commands`' decision and not this one's.
        let empty = DrawCommand::Path {
            points: Vec::new(),
            width: 2.0,
            color: Color::new(0, 0, 0, 255),
            closed: false,
        };
        let moved = translate_commands(std::slice::from_ref(&empty), Offset::new(9.0, 9.0));
        assert_eq!(moved, vec![empty.clone()], "one in, one out");
        assert_eq!(moved.len(), 1, "and it is still an empty path");
        assert_eq!(scroll::command_bounds(&moved[0]), None);
    }

    #[test]
    fn a_translation_of_nothing_is_the_identity() {
        let mut painter = Painter::new();
        painter.rect(Rect::new(1.0, 2.0, 3.0, 4.0), Color::new(1, 1, 1, 255));
        painter.text(5.0, 6.0, "x", Color::new(2, 2, 2, 255), 12.0, 0.0);
        painter.circle((7.0, 8.0), 9.0, Color::new(3, 3, 3, 255));
        let recorded = painter.finish();
        assert_eq!(
            translate_commands(&recorded, Offset::ZERO),
            recorded,
            "moving by nothing changes nothing, on any variant"
        );
    }

    #[test]
    fn an_empty_list_of_commands_translates_to_an_empty_list() {
        assert!(translate_commands(&[], Offset::new(10.0, 10.0)).is_empty());
    }

    #[test]
    fn a_row_away_from_the_origin_lands_where_the_row_goes() {
        // The fixture every other translation test does not have: the whole point
        // of translating by a row's position is that the position is not zero.
        let row = vec![DrawCommand::Rect {
            rect: Rect::new(0.0, 0.0, ROW_WIDTH, ROW_HEIGHT),
            color: ROW_COLOR,
        }];
        let moved = translate_commands(&row, Offset::new(OFFSET_VIEWPORT.x, 320.0));
        assert_eq!(
            rows_painted(&moved),
            vec![Rect::new(664.0, 320.0, 200.0, 100.0)],
            "the row's own (0, 0) is the row's top-left corner and nothing else"
        );
    }

    // ------------------------------------------------------- what a list is

    #[test]
    fn a_list_holds_the_things_the_task_gives_it() {
        let (_nodes, subject, _calls) = list(ROWS, ROW);
        assert_eq!(subject.item_count(), ROWS);
        assert_eq!(subject.item_height(), ROW);
        assert!(
            !subject.on_item_click.is_set(),
            "no handler until one is given"
        );
        assert_eq!(
            subject.scroll().scroll_offset.get(),
            0.0,
            "and one offset, reached through the scroll"
        );
        assert_eq!(subject.content_height(), CONTENT);
    }

    #[test]
    fn a_new_list_allocates_no_rows() {
        // A constructor that built a hundred nodes for a hundred items would be
        // the widget this one exists not to be.
        let mut nodes = Arena::new();
        let list = List::new(&mut nodes, ROWS, ROW);
        assert_eq!(
            nodes.len(),
            2,
            "the viewport node and the content node, and nothing else"
        );
        assert!(list.visible_items().is_empty());
        assert_eq!(list.free_len(), 0);
    }

    #[test]
    fn a_list_is_its_scrolls_node_and_the_rows_hang_off_the_content() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert_eq!(list.handle(), list.scroll().handle());
        assert!(
            list.scroll().is_attached(&nodes),
            "the scroll's content is attached, so the pass places it"
        );
        assert_eq!(
            nodes.get(list.content()).unwrap().layout().mode(),
            LayoutMode::Absolute,
            "a row's declared position is only read in this mode"
        );
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(nodes.get(list.content()).unwrap().children().len(), 3);
    }

    #[test]
    fn the_scrolls_content_height_is_the_lists_own() {
        // `on_event` reads the scroll's content height for its clamp, so the two
        // numbers must never drift apart — and the setters are the only things
        // that can move either of them.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        let agree = |list: &List| list.scroll().content_height() == list.content_height();
        assert!(agree(&list), "from the constructor");
        list.set_item_count(&mut nodes, 7);
        assert!(agree(&list), "and after a new count");
        list.set_item_height(&mut nodes, 33.0);
        assert!(agree(&list), "and after a new height");
        assert_eq!(list.scroll().content_height(), 231.0);
    }

    #[test]
    fn a_row_height_is_floored_at_zero() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert_eq!(list.set_item_height(&mut nodes, -50.0), 0.0);
        assert_eq!(list.item_height(), 0.0, "an extent cannot be negative");
        assert_eq!(list.content_height(), 0.0);
        assert!(!list.sync(&mut nodes, VIEWPORT), "and there is no geometry");
        assert!(list.paint(&nodes, VIEWPORT).is_empty());
        assert_eq!(List::new(&mut nodes, ROWS, -1.0).item_height(), 0.0);
    }

    #[test]
    fn a_list_with_no_factory_builds_no_rows_and_draws_nothing() {
        // The default is no factory, and a list with no factory is a list that
        // draws no rows. Inventing a row's colour here would be a decision about
        // the demo's look made in the widget.
        let mut nodes = Arena::new();
        let mut subject = List::new(&mut nodes, ROWS, ROW);
        assert!(!subject.item_factory().is_set());
        assert!(
            !subject.sync(&mut nodes, VIEWPORT),
            "there is nothing to build and nothing to change"
        );
        assert!(subject.visible_items().is_empty());
        assert!(rows_painted(&subject.paint(&nodes, VIEWPORT)).is_empty());
        assert_eq!(
            scrollbar_commands(&subject.paint(&nodes, VIEWPORT)),
            2,
            "and what it does draw is the scroll's scrollbar: 100 rows in 300 \
             pixels really do scroll"
        );

        // And a list with no factory that does not scroll draws nothing at all.
        let mut quiet_nodes = Arena::new();
        let mut quiet_list = List::new(&mut quiet_nodes, 2, ROW);
        assert!(
            !quiet_list.sync(&mut quiet_nodes, VIEWPORT),
            "no rows to build, no offset to move, nothing changed"
        );
        assert!(quiet_list.paint(&quiet_nodes, VIEWPORT).is_empty());
    }

    #[test]
    fn an_unset_factory_builds_nothing_and_says_so() {
        let mut nodes = Arena::new();
        let factory = ItemFactory::none();
        assert!(!factory.is_set());
        assert_eq!(factory.build(&mut nodes), None);
        assert_eq!(nodes.len(), 0);
        assert!(!ItemFactory::default().is_set());
    }

    // ------------------------------------------------- the setters' own work

    #[test]
    fn a_list_that_shrinks_gives_the_extra_rows_back() {
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(nodes.len(), 5, "two for the list and three rows");
        assert_eq!(list.visible_items().len(), 3);

        assert_eq!(list.set_item_count(&mut nodes, 2), 2);
        assert_eq!(
            indices(&list),
            vec![0, 1],
            "row 2's index is past the count"
        );
        assert_eq!(list.free_len(), 1, "and its node is on the free list");
        assert_eq!(
            nodes.len(),
            5,
            "which is still in the arena: a released row is recycled, not thrown \
             away, and `release_all` is what gives it back"
        );
        assert_eq!(calls.get(), 3, "and no row was built to make room");
    }

    #[test]
    fn a_list_that_grows_allocates_nothing_by_itself() {
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let before = nodes.len();
        assert_eq!(list.set_item_count(&mut nodes, 10_000), 10_000);
        assert_eq!(list.item_count(), 10_000);
        assert_eq!(
            nodes.len(),
            before,
            "growing is `sync`'s work, not the setter's"
        );
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn a_list_that_empties_gives_every_row_back() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(list.set_item_count(&mut nodes, 0), 0);
        assert!(list.visible_items().is_empty());
        assert_eq!(list.free_len(), 3, "every row went on the free list");
        assert_eq!(
            nodes.len(),
            5,
            "and is still in the arena, because a released row is still a row"
        );
        assert_eq!(list.release_all(&mut nodes), 3);
        assert_eq!(nodes.len(), 2, "until the caller asks for the nodes back");
    }

    #[test]
    fn a_new_row_height_re_sizes_the_live_rows() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let (_, first) = list.visible_items()[0];
        assert_eq!(
            nodes.get(first).unwrap().layout().constraints(),
            Constraints::tight(Size::new(VIEWPORT.width, ROW))
        );

        assert_eq!(list.set_item_height(&mut nodes, 55.0), 55.0);
        for (_, handle) in list.visible_items() {
            assert_eq!(
                nodes.get(*handle).unwrap().layout().constraints(),
                Constraints::tight(Size::new(VIEWPORT.width, 55.0)),
                "every live row took the new height at once, not on the next sync"
            );
        }
    }

    #[test]
    fn a_new_factory_gives_every_old_row_back() {
        // Recycling a row across factories would draw the old factory's row in
        // the new one's place, so the change of factory has to give the old nodes
        // to the arena rather than detach them.
        let (mut nodes, mut list, old_calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let stale = list.visible_items()[0].1;
        assert_eq!(nodes.len(), 5);

        let new_calls = Rc::new(Cell::new(0_usize));
        list.set_item_factory(&mut nodes, counting_factory(&new_calls));
        assert_eq!(nodes.len(), 2, "every old row is out of the arena");
        assert!(!nodes.is_valid(stale), "and its handle no longer resolves");
        assert_eq!(old_calls.get(), 3, "the old factory built nothing new");

        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(new_calls.get(), 3, "and the new one built the new rows");
        assert_eq!(nodes.len(), 5);
    }

    // ----------------------------------------------------- the virtualisation

    #[test]
    fn only_the_visible_rows_are_in_the_tree() {
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(indices(&list), vec![0, 1, 2], "three slots, three rows");
        assert_eq!(calls.get(), 3, "and three rows were built, not a hundred");
        assert_eq!(nodes.get(list.content()).unwrap().children().len(), 3);

        list.scroll().scroll_offset.set(1_000.0);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(
            indices(&list),
            vec![10, 11, 12],
            "the rows that are on screen at offset 1 000"
        );
        assert_eq!(
            calls.get(),
            3,
            "and the factory was not asked again: the three that left are the \
             three that arrived"
        );
    }

    #[test]
    fn a_row_that_scrolled_out_comes_back_as_the_same_handle_and_the_arena_did_not_grow() {
        // Requirement 2's second and third clauses, in one test: the arena's
        // length is the observable, because a handle that came back is only
        // evidence if nothing was allocated to produce it.
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let first = list.visible_items()[0].1;
        let after_first_sync = nodes.len();
        assert_eq!(after_first_sync, 5, "the list's two nodes and three rows");

        list.scroll().scroll_offset.set(1_000.0);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(
            list.free_len(),
            0,
            "the free list was emptied into the three rows that arrived in the \
             same pass, which is the whole of the recycling"
        );
        assert_eq!(nodes.len(), after_first_sync, "and the arena did not grow");

        list.scroll().scroll_offset.set(0.0);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(
            list.visible_items()[0].1,
            first,
            "the row that scrolled away and back is the same node"
        );
        assert_eq!(
            nodes.len(),
            after_first_sync,
            "and the arena did not grow by one node to get it"
        );
        assert_eq!(
            calls.get(),
            3,
            "three rows in total, for two screens and a hundred rows of list"
        );
    }

    #[test]
    fn a_row_is_taken_from_the_free_list_before_the_factory_is_asked() {
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(calls.get(), 3);

        // A scroll of exactly one row: row 0 leaves and one new row arrives, so
        // the free list has one and the factory must not be asked.
        list.scroll().scroll_offset.set(ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(indices(&list), vec![1, 2, 3]);
        assert_eq!(calls.get(), 3, "the released row was reused, not rebuilt");
    }

    #[test]
    fn the_factory_is_asked_only_when_the_free_list_is_empty() {
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(calls.get(), 3);
        for step in 1..=5 {
            list.scroll()
                .scroll_offset
                .set(ROW * f32::from(u8::try_from(step).unwrap_or(1)));
            assert!(list.sync(&mut nodes, VIEWPORT));
        }
        assert_eq!(
            indices(&list),
            vec![5, 6, 7],
            "five rows down, three on screen"
        );
        assert_eq!(
            calls.get(),
            3,
            "five scrolls and a hundred rows, and the factory was asked once per \
             row that is on screen"
        );
    }

    #[test]
    fn a_taller_viewport_reuses_the_rows_a_shorter_one_released() {
        // The only way the free list is ever non-empty at the end of a `sync`: a
        // screen that needs *fewer* rows than the last one did. A scroll of a
        // fixed viewport needs the same number, so it empties the free list into
        // the arrivals in the same pass — which is why the recycling test has to
        // change the viewport to see it.
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(calls.get(), 3);
        let after_three = nodes.len();

        let tall = Rect::new(0.0, 0.0, 200.0, 600.0);
        assert!(list.sync(&mut nodes, tall), "three more rows are needed");
        assert_eq!(
            indices(&list),
            vec![0, 1, 2, 3, 4, 5],
            "600 over 100 is six"
        );
        assert_eq!(calls.get(), 6);

        assert!(list.sync(&mut nodes, VIEWPORT), "three of them are not");
        assert_eq!(indices(&list), vec![0, 1, 2]);
        assert_eq!(list.free_len(), 3, "so three rows are on the free list");
        assert_eq!(calls.get(), 6, "and none of them was rebuilt");
        assert_eq!(
            nodes.len(),
            after_three + 3,
            "the arena holds the three live rows and the three waiting ones"
        );

        assert!(list.sync(&mut nodes, tall), "and the six are wanted again");
        assert_eq!(list.free_len(), 0, "the free list paid for the difference");
        assert_eq!(calls.get(), 6, "and the factory was never asked again");
        assert_eq!(nodes.len(), after_three + 3, "and the arena did not grow");
    }

    #[test]
    fn a_row_that_stayed_is_not_moved() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let (_, middle) = list.visible_items()[1];
        list.scroll().scroll_offset.set(50.0);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(
            nodes.get(middle).unwrap().layout().position(),
            Some(Offset::new(0.0, ROW)),
            "row 1's position is its index in the content, and the content moved"
        );
    }

    #[test]
    fn a_rows_position_is_its_index_in_the_contents_coordinates() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        for (index, handle) in list.visible_items().to_vec() {
            assert_eq!(
                nodes.get(handle).unwrap().layout().position(),
                Some(Offset::new(0.0, count_to_f32(index) * ROW)),
                "row {index} is placed at its index, and the offset is not in it"
            );
        }
    }

    #[test]
    fn the_layout_pass_places_a_row_exactly_where_item_rect_says() {
        // The widget's own arithmetic, checked against the pass rather than
        // against itself. The list is at (664, 120), so an origin read as an
        // extent cannot pass.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        let (root, rect) = on_a_panel(&list, &mut nodes, Size::new(200.0, 300.0));
        assert_eq!(rect, OFFSET_VIEWPORT, "the list really is off the origin");

        assert!(list.sync(&mut nodes, rect));
        relayout(&mut nodes, root);
        for index in 0..3 {
            assert_eq!(
                placed(&nodes, &list, index),
                list.item_rect(index, rect),
                "row {index} is where `item_rect` says it goes"
            );
            assert_eq!(
                placed(&nodes, &list, index).x,
                664.0,
                "and not at the origin"
            );
        }

        list.scroll().scroll_offset.set(60.0);
        assert!(list.sync(&mut nodes, rect));
        relayout(&mut nodes, root);
        for index in 0..4 {
            assert_eq!(
                placed(&nodes, &list, index),
                list.item_rect(index, rect),
                "row {index} is where `item_rect` says it goes, 60 scrolled"
            );
        }
        assert_eq!(
            placed(&nodes, &list, 0).y,
            60.0,
            "60 above the viewport's top edge, which is at 120"
        );
    }

    #[test]
    fn a_resize_writes_the_new_width_onto_every_live_row() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        let (root, _) = on_a_panel(&list, &mut nodes, Size::new(200.0, 300.0));
        assert!(list.sync(&mut nodes, OFFSET_VIEWPORT));
        relayout(&mut nodes, root);
        assert_eq!(placed(&nodes, &list, 0).width, 200.0);

        let wider = Rect::new(OFFSET_VIEWPORT.x, OFFSET_VIEWPORT.y, 320.0, 300.0);
        if let Some(node) = nodes.get_mut(list.handle()) {
            node.layout_mut()
                .set_constraints(Constraints::tight(Size::new(320.0, 300.0)));
        }
        assert!(list.sync(&mut nodes, wider), "the width changed");
        relayout(&mut nodes, root);
        for index in 0..3 {
            assert_eq!(
                placed(&nodes, &list, index).width,
                320.0,
                "row {index} took the new width on the frame the width changed"
            );
        }
    }

    #[test]
    fn a_row_the_arena_no_longer_holds_is_dropped_rather_than_reused() {
        // A free row is a node somebody can attach again, so a stale handle on
        // the free list would make the next `sync` attach nothing and believe it
        // had. The only way to reach that state is a caller that removed a node
        // of its own, which is what this does.
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let tall = Rect::new(0.0, 0.0, 200.0, 600.0);
        assert!(list.sync(&mut nodes, tall));
        let six: Vec<Handle> = list.visible_items().iter().map(|(_, h)| *h).collect();
        assert_eq!(six.len(), 6);

        assert!(
            list.sync(&mut nodes, VIEWPORT),
            "three of them are released"
        );
        assert_eq!(list.free_len(), 3);
        let freed: Vec<Handle> = six
            .iter()
            .copied()
            .filter(|handle| !list.visible_items().iter().any(|(_, held)| held == handle))
            .collect();
        assert_eq!(
            freed.len(),
            3,
            "which are the three the list is not holding"
        );

        let doomed = freed[0];
        assert!(
            nodes.remove(doomed).is_some(),
            "a caller removed a free row"
        );
        let before = calls.get();

        assert!(list.sync(&mut nodes, tall), "the six are wanted again");
        assert_eq!(indices(&list), vec![0, 1, 2, 3, 4, 5]);
        assert!(
            !list
                .visible_items()
                .iter()
                .any(|(_, handle)| *handle == doomed),
            "the handle the arena dropped was not attached to anything"
        );
        assert!(
            list.visible_items()
                .iter()
                .all(|(_, handle)| nodes.is_valid(*handle)),
            "and every row on screen is a node the arena holds"
        );
        assert_eq!(
            calls.get(),
            before + 1,
            "the free list had two usable rows and the factory was asked once for \
             the third"
        );
    }

    #[test]
    fn a_factory_that_hands_back_a_taken_node_does_not_break_the_list() {
        let mut nodes = Arena::new();
        let mut list = List::new(&mut nodes, 10, ROW);
        let taken = node::create(&mut nodes, State::new());
        let other = node::create(&mut nodes, State::new());
        assert!(
            node::attach(&mut nodes, other, taken),
            "it already has a parent"
        );
        let before = nodes.len();

        list.set_item_factory(&mut nodes, ItemFactory::new(move |_nodes| taken));
        assert!(
            list.sync(&mut nodes, VIEWPORT),
            "the tree changed: a row was refused"
        );
        assert!(list.visible_items().is_empty(), "and no row took its place");
        assert!(
            !nodes.is_valid(taken),
            "the node the list cannot use is gone"
        );
        assert_eq!(nodes.len(), before - 1, "and the arena does not hold it");
    }

    #[test]
    fn syncing_twice_changes_nothing_the_second_time() {
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert!(
            !list.sync(&mut nodes, VIEWPORT),
            "the offset, the width and the set of rows are all the same"
        );
        assert_eq!(calls.get(), 3);
        assert_eq!(nodes.len(), 5);
    }

    #[test]
    fn a_hand_written_offset_out_of_range_is_clamped_by_the_sync() {
        // A caller that wrote the property itself wrote it with no clamp, and
        // `sync` is the re-clamp — the same delta of zero the scroll documents.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        list.scroll().scroll_offset.set(1.0e9);
        assert_eq!(list.offset(VIEWPORT), MAX_OFFSET, "clamped when read");

        assert!(list.sync(&mut nodes, VIEWPORT), "the property itself moved");
        assert_eq!(list.scroll().scroll_offset.get(), MAX_OFFSET);
        assert_eq!(indices(&list), vec![97, 98, 99], "and the rows followed");
    }

    #[test]
    fn releasing_every_row_gives_the_arena_its_nodes_back() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        list.scroll().scroll_offset.set(1_000.0);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let handles: Vec<Handle> = list.visible_items().iter().map(|(_, h)| *h).collect();
        assert_eq!(nodes.len(), 5, "three rows, whichever of them is where");

        assert_eq!(list.release_all(&mut nodes), 3, "the three live rows");
        assert!(list.visible_items().is_empty());
        assert_eq!(list.free_len(), 0);
        assert_eq!(nodes.len(), 2, "only the list's own two nodes are left");
        assert!(
            handles.iter().all(|handle| !nodes.is_valid(*handle)),
            "and every handle stopped resolving"
        );
    }

    #[test]
    fn a_released_list_rebuilds_what_it_needs_on_the_next_sync() {
        let (mut nodes, mut list, calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(list.release_all(&mut nodes), 3);
        assert_eq!(calls.get(), 3);

        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(indices(&list), vec![0, 1, 2]);
        assert_eq!(nodes.len(), 5, "three fresh rows");
        assert_eq!(calls.get(), 6, "and the factory built them again");
    }

    // --------------------------------------------------------- painting

    #[test]
    fn only_the_visible_rows_are_drawn() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let painted = list.paint(&nodes, VIEWPORT);
        assert_eq!(
            rows_painted(&painted),
            vec![
                Rect::new(0.0, 0.0, ROW_WIDTH, ROW_HEIGHT),
                Rect::new(0.0, 100.0, ROW_WIDTH, ROW_HEIGHT),
                Rect::new(0.0, 200.0, ROW_WIDTH, ROW_HEIGHT),
            ],
            "one row per slot, and no fourth row"
        );
    }

    #[test]
    fn a_rows_commands_are_drawn_where_the_row_goes() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, OFFSET_VIEWPORT));
        let painted = rows_painted(&list.paint(&nodes, OFFSET_VIEWPORT));
        assert_eq!(
            painted,
            vec![
                Rect::new(664.0, 120.0, ROW_WIDTH, ROW_HEIGHT),
                Rect::new(664.0, 220.0, ROW_WIDTH, ROW_HEIGHT),
                Rect::new(664.0, 320.0, ROW_WIDTH, ROW_HEIGHT),
            ],
            "the rows are at the list's x and at the list's y, not at the origin"
        );

        list.scroll().scroll_offset.set(60.0);
        assert!(list.sync(&mut nodes, OFFSET_VIEWPORT));
        let scrolled = rows_painted(&list.paint(&nodes, OFFSET_VIEWPORT));
        assert_eq!(
            scrolled.first().map(|rect| rect.y),
            Some(60.0),
            "60 above the viewport's top edge, which is at 120"
        );
        assert_eq!(
            scrolled.len(),
            4,
            "and four rows, because 60 is not a whole row"
        );
    }

    #[test]
    fn a_command_whole_outside_the_band_is_dropped_and_a_straddler_is_kept() {
        // Requirement 5, in the shape the list uses `clip_commands`: a row's own
        // drawing decides where it can be, and trimming a straddler is not
        // clipping it.
        let inside = Rect::new(0.0, 0.0, 10.0, 10.0);
        let gone = Rect::new(0.0, 5_000.0, 10.0, 10.0);
        let straddler = Rect::new(0.0, -50.0, 10.0, 500.0);
        let shade = |rect: Rect, tone: u8| DrawCommand::Rect {
            rect,
            color: Color::new(tone, tone, tone, 255),
        };
        let mut nodes = Arena::new();
        let mut list = List::new(&mut nodes, 2, ROW);
        list.set_item_factory(
            &mut nodes,
            factory_recording(vec![shade(inside, 1), shade(gone, 2), shade(straddler, 3)]),
        );
        assert!(list.sync(&mut nodes, VIEWPORT));

        let painted = rows_painted(&list.paint(&nodes, VIEWPORT));
        assert_eq!(painted.len(), 4, "two rows of two surviving commands each");
        assert!(
            painted.contains(&inside),
            "the command inside the band is drawn, at the row's own origin"
        );
        assert!(
            painted.contains(&straddler),
            "and the one that straddles the edge is kept whole rather than trimmed"
        );
        assert!(
            !painted.contains(&gone),
            "and the one 5 000 pixels below the band is not drawn at all"
        );
        assert!(
            painted.contains(&Rect::new(0.0, 100.0, 10.0, 10.0)),
            "row 1's own command is at row 1's origin, so each row drew its own"
        );
    }

    #[test]
    fn the_scrollbar_is_drawn_after_the_rows() {
        // A thumb drawn under an opaque row is a scrollbar nobody can see — a
        // filled rounded rectangle is not an outline, one level up.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let painted = list.paint(&nodes, VIEWPORT);
        assert_eq!(
            rows_painted(&painted).len(),
            3,
            "there is something to scroll"
        );
        let bar = first_scrollbar(&painted).expect("100 rows in 300 pixels scroll");
        assert!(
            bar >= 3,
            "the scrollbar is recorded at {bar}, after all three rows"
        );
    }

    #[test]
    fn a_list_with_nothing_to_scroll_draws_no_scrollbar() {
        // Two rows of 100 in a 300 viewport: a scrollbar would lie about a scroll
        // that does not exist.
        let (mut nodes, mut list, _calls) = list(2, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let painted = list.paint(&nodes, VIEWPORT);
        assert_eq!(rows_painted(&painted).len(), 2);
        assert_eq!(scrollbar_commands(&painted), 0);
        assert_eq!(first_scrollbar(&painted), None);
    }

    #[test]
    fn painting_does_not_move_the_offset_or_the_tree() {
        // Painting is a read. A caller that paints on a frame where it has not
        // synced must not have the sync undone by the draw.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        list.scroll().scroll_offset.set(250.0);
        let first = list.paint(&nodes, VIEWPORT);
        let second = list.paint(&nodes, VIEWPORT);
        assert_eq!(first, second, "two paints of the same state agree");
        assert_eq!(
            list.scroll().scroll_offset.get(),
            250.0,
            "and the offset is 250"
        );
        assert_eq!(list.visible_items().len(), 3);
    }

    #[test]
    fn a_row_removed_from_the_arena_is_skipped_rather_than_reported() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let (_, middle) = list.visible_items()[1];
        assert!(nodes.remove(middle).is_some());

        let painted = list.paint(&nodes, VIEWPORT);
        assert_eq!(
            rows_painted(&painted).len(),
            2,
            "the two rows that are there"
        );
        assert_eq!(
            rows_painted(&painted)
                .iter()
                .map(|rect| rect.y)
                .collect::<Vec<_>>(),
            vec![0.0, 200.0],
            "and they are rows 0 and 2, with the hole where row 1 was"
        );
    }

    // ------------------------------------------------------------- the clip

    #[test]
    fn the_clip_rect_is_the_viewport_until_the_content_is_short() {
        let (_nodes, tall, _calls) = list(ROWS, ROW);
        assert_eq!(tall.clip_rect(OFFSET_VIEWPORT), OFFSET_VIEWPORT);

        let (_short_nodes, short, _short_calls) = list(2, ROW);
        assert_eq!(
            short.clip_rect(OFFSET_VIEWPORT),
            Rect::new(664.0, 120.0, 200.0, 200.0),
            "two rows of 100 is 200 of content, and the tail is not the list's"
        );
    }

    #[test]
    fn the_clip_rect_never_runs_past_the_content() {
        for (count, height, want) in [
            (1_usize, 100.0_f32, 100.0_f32),
            (2, 100.0, 200.0),
            (3, 100.0, 300.0),
            (7, 13.0, 91.0),
        ] {
            let (_nodes, list, _calls) = list(count, height);
            let clip = list.clip_rect(OFFSET_VIEWPORT);
            assert_eq!(
                clip.height,
                want.min(OFFSET_VIEWPORT.height),
                "{count} rows of {height} clip to the content, not past it"
            );
            assert_eq!(clip.x, OFFSET_VIEWPORT.x);
            assert_eq!(
                clip.y, OFFSET_VIEWPORT.y,
                "and the band is the viewport's top"
            );
        }
    }

    #[test]
    fn the_clip_rect_follows_a_list_off_the_window_origin() {
        let (_nodes, list, _calls) = list(ROWS, ROW);
        assert_eq!(OFFSET_VIEWPORT.x, 664.0, "the fixture is off the origin");
        let clip = list.clip_rect(OFFSET_VIEWPORT);
        assert_eq!(
            clip, OFFSET_VIEWPORT,
            "the band's top is the viewport's top, not the content's offset"
        );
        assert_ne!(clip.x, 0.0);
        assert_ne!(clip.y, 0.0);
    }

    // ---------------------------------------------------------- the interaction

    #[test]
    fn a_tap_on_a_row_reports_that_rows_index() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        for (y, want) in [(0.0, 0_usize), (50.0, 0), (99.0, 0), (100.0, 1), (250.0, 2)] {
            let seen = taps_on(&mut list, Offset::new(100.0, y), VIEWPORT);
            assert_eq!(seen, vec![want], "a tap {y} down is row {want}");
        }
    }

    #[test]
    fn half_a_row_is_still_that_row() {
        // The module's answer to "a tap between two rows", on both sides of a
        // row's middle: the rows tile, so there is no gap between them to fall
        // into, and the first pixel of a row is that row.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        for y in [200.0_f32, 240.0, 299.0] {
            assert_eq!(
                taps_on(&mut list, Offset::new(100.0, y), VIEWPORT),
                vec![2],
                "the lower half of row 2 is row 2, at {y}"
            );
        }
        assert_eq!(
            taps_on(&mut list, Offset::new(100.0, 300.0), VIEWPORT),
            Vec::<usize>::new(),
            "and the viewport's bottom edge is nobody's: it is past the last row \
             on screen, and row 3 begins exactly there"
        );
        assert_eq!(
            taps_on(&mut list, Offset::new(100.0, 299.0), VIEWPORT),
            vec![2],
            "the last pixel row of the viewport is the last pixel of row 2"
        );
    }

    #[test]
    fn a_tap_on_a_half_visible_row_is_that_row() {
        // At offset 250 the band is [250, 550), so row 2 shows its bottom 50
        // pixels: window rows 0 to 49. A tap on the last of them is row 2, and
        // the first pixel below is row 3 — the row boundary in the window is
        // exactly the content boundary.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        list.scroll().scroll_offset.set(250.0);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(indices(&list), vec![2, 3, 4, 5]);

        assert_eq!(
            taps_on(&mut list, Offset::new(100.0, 0.0), VIEWPORT),
            vec![2]
        );
        assert_eq!(
            taps_on(&mut list, Offset::new(100.0, 49.0), VIEWPORT),
            vec![2]
        );
        assert_eq!(
            taps_on(&mut list, Offset::new(100.0, 50.0), VIEWPORT),
            vec![3]
        );
    }

    #[test]
    fn a_tap_below_the_content_is_nobodys_and_is_not_consumed() {
        // The gap there is, and it is the only one: the ragged tail of a viewport
        // taller than the content. The tap is not consumed, so whatever is behind
        // the list still sees it.
        let (mut nodes, mut list, _calls) = list(2, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let mut tap = tap_at(100.0, 250.0);
        assert!(!list.on_event(&mut tap, VIEWPORT));
        assert!(!tap.consumed(), "and it carries on up the tree");
        assert!(taps_on(&mut list, Offset::new(100.0, 250.0), VIEWPORT).is_empty());
    }

    #[test]
    fn a_tap_outside_the_lists_own_rect_is_nobodys() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        for (x, y) in [
            (100.0, -1.0),
            (100.0, 300.0),
            (-1.0, 150.0),
            (201.0, 150.0),
            (1_000.0, 1_000.0),
        ] {
            let mut tap = tap_at(x, y);
            assert!(
                !list.on_event(&mut tap, VIEWPORT),
                "a tap at ({x}, {y}) is outside the list"
            );
            assert!(!tap.consumed());
        }
        // The edges themselves are inside, which is a hit test's convention: a tap
        // on the last pixel row of the list is a tap on the list.
        assert_eq!(
            taps_on(&mut list, Offset::new(200.0, 0.0), VIEWPORT),
            vec![0]
        );
    }

    /// A tap on the scrollbar's strip is the scrollbar's, not a row's.
    ///
    /// It names no row **and fires nothing**, and it is not consumed: the strip is
    /// a control drawn over the right edge of the rows, so a press aimed at a
    /// scrollbar that named row 0 was activating something the user did not mean.
    #[test]
    fn a_tap_on_the_scrollbar_is_nobodys_row() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let bar = list
            .scroll()
            .scrollbar_rect(VIEWPORT)
            .expect("a hundred rows in a 300 viewport scrolls, so a bar is drawn");

        // Every row's worth of height, sampled across the whole strip: the strip
        // is not "the thumb" and it is not "the top", it is the scrollbar.
        for y in [0.0, 99.0, 150.0, 299.0] {
            for x in [bar.x, bar.x + bar.width / 2.0, bar.x + bar.width] {
                let at = Offset::new(x, y);
                assert_eq!(
                    list.item_at(at, VIEWPORT),
                    None,
                    "({x}, {y}) is inside the scrollbar, so it is no row's"
                );
                assert!(
                    taps_on(&mut list, at, VIEWPORT).is_empty(),
                    "and firing nothing is the half a row index cannot show"
                );
            }
        }
    }

    /// The column beside the scrollbar is still a row's, which is what stops the
    /// exclusion above from eating a strip of the list's own content.
    #[test]
    fn a_tap_beside_the_scrollbar_is_still_that_row() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let bar = list
            .scroll()
            .scrollbar_rect(VIEWPORT)
            .expect("a bar is drawn");

        // Half a pixel clear of the bar's left edge, at a height that is row 1.
        let just_left = Offset::new(bar.x - 0.5, 150.0);
        assert_eq!(
            list.item_at(just_left, VIEWPORT),
            Some(1),
            "150 down is the second row, and half a pixel left of the scrollbar \
             is still inside the list"
        );
        assert_eq!(taps_on(&mut list, just_left, VIEWPORT), vec![1]);
    }

    /// A list that does not scroll draws no scrollbar, so it excludes no strip:
    /// every pixel of its viewport is still a row's, to the last column.
    ///
    /// This is the direction the previous test does not cover. Without it, a
    /// list whose content fits would silently lose its right-hand edge to a
    /// scrollbar it never drew.
    #[test]
    fn a_list_that_does_not_scroll_excludes_no_strip() {
        let (mut nodes, mut list, _calls) = list(2, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(
            list.scroll().scrollbar_rect(VIEWPORT),
            None,
            "two rows in a 300 viewport leaves a ragged tail and no bar"
        );
        for x in [100.0, 198.0, 199.0, 200.0] {
            assert_eq!(
                list.item_at(Offset::new(x, 150.0), VIEWPORT),
                Some(1),
                "x={x} is a row, because there is no scrollbar to claim it"
            );
        }
    }

    /// The strip the list refuses is the strip the scrollbar draws — including
    /// after a caller widens it.
    ///
    /// A second copy of the scrollbar's geometry in this module would be a
    /// second thing to keep in step with the scrollbar's own thickness, and it
    /// would be wrong the moment a caller widened the bar: the operator widened
    /// the demo's to 12 for exactly the reason the 6 was unusable, and a stale
    /// copy here would still be refusing 6 of those 12 pixels.
    #[test]
    fn the_excluded_strip_is_the_widened_scrollbar_rather_than_the_old_hairline() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        list.set_scrollbar_thickness(12.0);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let bar = list
            .scroll()
            .scrollbar_rect(VIEWPORT)
            .expect("a bar is drawn");
        assert_eq!(bar.width, 12.0);

        // 200 wide, inset by the margin, so the bar starts at 186 — which is
        // inside the 6-pixel bar the default would have drawn and inside the 16
        // columns that the default's geometry would have called content.
        assert!(
            list.item_at(Offset::new(bar.x, 150.0), VIEWPORT).is_none(),
            "x={} is the widened bar's own left edge",
            bar.x
        );
        assert_eq!(
            list.item_at(Offset::new(bar.x - 0.5, 150.0), VIEWPORT),
            Some(1),
            "and half a pixel before it is row 1"
        );
    }

    #[test]
    fn a_tap_with_no_position_is_left_alone() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let mut blind = InputEvent::new(InputEventKind::Tap, None);
        assert!(!list.on_event(&mut blind, VIEWPORT));
        assert!(!blind.consumed());
    }

    /// **The sign reversed, 2026-09-30** — the convention is now *down is later*,
    /// see `scroll::gesture_delta`. This used to say a drag down reveals what was
    /// above. It is kept rather than dropped because it is also the only test that
    /// says a **clamped** drag is still *consumed*: a finger pushed past the top
    /// belongs to this list and stops there, rather than falling through to
    /// whatever is behind it.
    #[test]
    fn a_drag_scrolls_the_list() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let mut up = drag_to(100.0, 50.0, Offset::new(0.0, -120.0));
        assert!(list.on_event(&mut up, VIEWPORT));
        assert_eq!(
            list.scroll().scroll_offset.get(),
            0.0,
            "a drag up the screen at the top cannot go back any further"
        );
        assert!(up.consumed(), "but the drag was still this list's");

        let mut down = drag_to(100.0, 150.0, Offset::new(0.0, 120.0));
        assert!(list.on_event(&mut down, VIEWPORT));
        assert_eq!(
            list.scroll().scroll_offset.get(),
            120.0,
            "and down is later"
        );
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(
            indices(&list),
            vec![1, 2, 3, 4],
            "four rows, because 120 is not a whole row: the band is [120, 420) and \
             row 4 shows its bottom 20 pixels"
        );
    }
    /// **The wheel's sign changed on 2026-09-30, by the operator's decision, and
    /// these two tests were written against the other one.** The list now uses the
    /// **scrollbar convention**: SDL reports the wheel rolling *towards* the user
    /// as a negative `y`, and that is the direction that advances *down* the
    /// document. So the notch that moves the offset **up** is `dy = -1.0` now and
    /// was `+1.0` before — the same fact about the hardware, the opposite question
    /// about what the reader expects.
    ///
    /// The tests are rewritten rather than flipped, because a test that says "and
    /// the other way goes back" is the part worth keeping: it is what says the two
    /// notches are inverses of each other rather than both going the same way.
    #[test]
    fn a_wheel_notch_scrolls_the_list_one_notch() {
        // The step is `scroll::WHEEL_STEP`'s and not a number here: 48 pixels,
        // whichever way the wheel was turned.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        // The wheel rolled *towards* the user: SDL's negative y, and down the list.
        let mut down = wheel(0.0, -1.0);
        assert!(list.on_event(&mut down, VIEWPORT));
        assert!(down.consumed());
        assert_eq!(list.scroll().scroll_offset.get(), 48.0);
        assert_eq!(list.max_scroll_for(VIEWPORT), MAX_OFFSET);

        // And the other way is exactly its inverse.
        let mut up = wheel(0.0, 1.0);
        assert!(list.on_event(&mut up, VIEWPORT));
        assert_eq!(list.scroll().scroll_offset.get(), 0.0);
    }

    #[test]
    fn a_wheel_notch_is_clamped_at_both_ends() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        for _ in 0..300 {
            let mut notch = wheel(0.0, -1.0);
            assert!(list.on_event(&mut notch, VIEWPORT));
        }
        assert_eq!(list.scroll().scroll_offset.get(), MAX_OFFSET);
        assert!(list.sync(&mut nodes, VIEWPORT));
        assert_eq!(indices(&list), vec![97, 98, 99]);

        // And back the other way, to the top, which is where a list that cannot
        // scroll past its own start would differ from one that can.
        for _ in 0..300 {
            let mut notch = wheel(0.0, 1.0);
            assert!(list.on_event(&mut notch, VIEWPORT));
        }
        assert_eq!(list.scroll().scroll_offset.get(), 0.0);
    }

    /// **Rewritten 2026-09-30.** This used to assert that a wheel notch and a drag
    /// move the list *opposite* ways, which was true for one day and was the state
    /// the operator reported as "the direction has to be reversed". Both were
    /// inverted and they agree now; what is left to pin is that a list — which
    /// delegates its scrolling wholesale to
    /// [`Scroll`](crate::widgets::scroll::Scroll) — inherits the agreement rather
    /// than keeping a copy of the old rule of its own.
    ///
    /// The number that matters is the sum: a notch down the document and then 40
    /// pixels of finger, in the same direction, add up. A list with a sign of its
    /// own anywhere would give 8 here.
    #[test]
    fn a_list_takes_its_scroll_direction_from_the_scroll_underneath_it() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let mut notch = wheel(0.0, -1.0);
        assert!(list.on_event(&mut notch, VIEWPORT));
        assert_eq!(
            list.scroll().scroll_offset.get(),
            48.0,
            "a wheel notch goes down the document"
        );

        let mut finger = drag_to(100.0, 190.0, Offset::new(0.0, 40.0));
        assert!(list.on_event(&mut finger, VIEWPORT));
        assert_eq!(
            list.scroll().scroll_offset.get(),
            88.0,
            "and a finger travelling down the screen adds to it, in the same \
             direction — 48 + 40, not 48 - 40"
        );
    }

    #[test]
    fn an_arrow_key_scrolls_a_focused_list_and_nothing_else() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let mut down = key_down(crate::input::Key::Keyboard(sdl3::keyboard::Keycode::Down));
        assert!(
            !list.on_event(&mut down, VIEWPORT),
            "a key is not routed by position, and this list is not focused"
        );
        assert!(!down.consumed());
        assert_eq!(list.scroll().scroll_offset.get(), 0.0);

        list.scroll().focused.set(true);
        assert!(list.on_event(&mut down, VIEWPORT));
        assert_eq!(
            list.scroll().scroll_offset.get(),
            30.0,
            "a tenth of a 300 viewport, which is `Scroll`'s fraction"
        );
        assert!(down.consumed());
    }

    #[test]
    fn a_horizontal_wheel_is_left_for_a_horizontal_scroll() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let mut across = wheel(1.0, 0.0);
        assert!(!list.on_event(&mut across, VIEWPORT));
        assert!(!across.consumed());
        assert_eq!(list.scroll().scroll_offset.get(), 0.0);
    }

    #[test]
    fn a_list_with_no_callback_still_reports_its_taps() {
        // A caller that has given the widget nothing to report to is an ordinary
        // caller, and this is the shape every other widget in the crate has.
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        let mut tap = tap_at(100.0, 250.0);
        assert!(list.on_event(&mut tap, VIEWPORT), "the tap was still its");
        assert_eq!(list.scroll().scroll_offset.get(), 0.0, "and nothing moved");
    }

    #[test]
    fn a_long_press_and_a_swipe_are_neither_a_tap_nor_a_scroll() {
        let (mut nodes, mut list, _calls) = list(ROWS, ROW);
        assert!(list.sync(&mut nodes, VIEWPORT));
        for kind in [
            InputEventKind::LongPress,
            InputEventKind::Swipe {
                direction: crate::input::SwipeDirection::Up,
            },
            InputEventKind::Pinch { scale: 2.0 },
            InputEventKind::KeyUp {
                key: crate::input::Key::Keyboard(sdl3::keyboard::Keycode::Down),
                keymod: sdl3::keyboard::Mod::empty(),
            },
        ] {
            // `kind` is not `Copy` any more — `InputEventKind::Text` carries a
            // `String` — so the message is rendered before the move, not after.
            let label = format!("{kind:?}");
            let mut event = InputEvent::new(kind, Some(Offset::new(100.0, 250.0)));
            assert!(
                !list.on_event(&mut event, VIEWPORT),
                "{label} is not this list's"
            );
            assert!(!event.consumed());
        }
        assert_eq!(list.scroll().scroll_offset.get(), 0.0);
    }

    // -------------------------------------------------- the widget's own helpers

    #[test]
    fn covers_includes_the_edges_of_the_rect() {
        let rect = OFFSET_VIEWPORT;
        assert!(
            covers(rect, Offset::new(664.0, 120.0)),
            "the top-left corner"
        );
        assert!(covers(rect, Offset::new(864.0, 420.0)), "the far corner");
        assert!(covers(rect, Offset::new(764.0, 270.0)), "the middle");
        assert!(!covers(rect, Offset::new(663.9, 270.0)), "just left of it");
        assert!(!covers(rect, Offset::new(864.1, 270.0)), "just right of it");
        assert!(!covers(rect, Offset::new(764.0, 119.9)), "just above it");
    }

    #[test]
    fn a_factory_can_be_cloned_and_shares_its_closure() {
        let calls = Rc::new(Cell::new(0_usize));
        let factory = counting_factory(&calls);
        let mut nodes = Arena::new();
        let copy = factory.clone();
        let _ = factory.build(&mut nodes);
        let _ = copy.build(&mut nodes);
        assert_eq!(calls.get(), 2, "both ran the same closure");
        assert_eq!(nodes.len(), 2);
    }
}
