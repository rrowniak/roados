//! The Container widget: the node that groups children and lays them out.
//!
//! A container is the composition primitive everything else is built on. It has
//! no appearance of its own: it arranges its children — a row, a column, a
//! stack, or children that position themselves — and, when it is given a
//! background, draws a rounded rectangle behind them.
//!
//! The container is a thin wrapper over a node, and the *node* is where the
//! layout inputs live. [`Container::set_mode`], [`Container::set_padding`] and
//! [`Container::set_flex_config`] write the node's [`LayoutState`], which is
//! what the layout pass reads; the size of the container is set the way every
//! other node is sized, through [`WidgetNode::layout_mut`]. A copy of the mode
//! or the padding held on the widget as well would be a value nothing reads, and
//! a padding only the container held would leave every other node — every row,
//! every panel, every stacked child — placing its children at the unpadded
//! origin.
//!
//! Every method that needs the arena takes it, the way [`node::attach`],
//! [`node::detach`] and [`Button::new`](crate::widgets::button::Button::new)
//! do: a node cannot reach the arena that holds it, because the arena owns the
//! node. The task file's
//! `Container::add_child(&self, child: Handle) -> Handle` is read as that shape,
//! with the handle coming from [`Container::handle`].
//!
//! The background is a plain [`Property`] rather than an
//! `Option<Property<Color>>`, and the task file's optionality is read as a
//! transparent default. A colour with a zero alpha is how this repository says
//! "not drawn", and it is the only shape a *themed* background can take: a
//! colour that follows the theme is a [`Property::bind`], and a bind cannot
//! produce an `Option`. [`Container::paint`] records nothing at all while the
//! background is transparent, so a container with no background costs no draw
//! command rather than a transparent one.
//!
//! The background is behind the children without the container knowing they
//! exist: a node's draw commands are recorded in the order the tree is walked,
//! and a parent is walked before its children.
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::layout::{Constraints, Layout, LayoutMode, LayoutState, Rect, Size};
//! use ui_core::node::{self, WidgetNode};
//! use ui_core::widgets::container::Container;
//!
//! let mut nodes = Arena::new();
//! let row = Container::new(&mut nodes, LayoutMode::row());
//! let first = node::create(
//!     &mut nodes,
//!     LayoutState::new().with_constraints(Constraints::tight(Size::new(40.0, 20.0))),
//! );
//! let second = node::create(
//!     &mut nodes,
//!     LayoutState::new().with_constraints(Constraints::tight(Size::new(60.0, 20.0))),
//! );
//! assert!(row.add_child(&mut nodes, first));
//! assert!(row.add_child(&mut nodes, second));
//!
//! Layout::new(&mut nodes).layout(row.handle(), Constraints::tight(Size::new(200.0, 20.0)));
//!
//! let rect = |handle| nodes.get(handle).unwrap().layout().rect().unwrap();
//! assert_eq!(rect(first), Rect::from_parts(0.0, 0.0, 40.0, 20.0));
//! assert_eq!(rect(second), Rect::from_parts(40.0, 0.0, 60.0, 20.0));
//! ```

use crate::arena::{Arena, Handle};
use crate::layout::{FlexConfig, LayoutMode, LayoutState, Padding};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};

/// A container: a node that arranges the children attached to it.
///
/// The widget owns its node and the two properties that are *painted* — the
/// background and its corner radius. Everything the layout pass reads stays on
/// the node's [`LayoutState`], and the setters below are the doors to it, so
/// that the widget and the pass can never disagree about how this container
/// arranges its children.
///
/// The properties are bound by the caller rather than read from a theme here,
/// for the reason the rest of the library reads tokens through properties: the
/// property graph carries a theme switch to a widget holding one. The defaults
/// are a fully transparent background and no corner radius, which is a
/// container that draws nothing — the task's "invisible unless a background is
/// set".
///
/// A setter whose handle the arena no longer holds changes nothing. The only way
/// to reach that state is to have removed the node, and there is then no node
/// left to lay out or paint.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::layout::LayoutMode;
/// use ui_core::node::WidgetNode;
/// use ui_core::property::Color;
/// use ui_core::widgets::container::Container;
///
/// let mut nodes = Arena::new();
/// let mut card = Container::new(&mut nodes, LayoutMode::column());
/// assert!(
///     card.paint(ui_core::paint::Rect::new(0.0, 0.0, 10.0, 10.0)).is_empty(),
///     "a container with no background draws nothing"
/// );
///
/// card.background.set(Color::new(20, 20, 20, 255));
/// card.border_radius.set(6.0);
/// assert_eq!(card.paint(ui_core::paint::Rect::new(0.0, 0.0, 10.0, 10.0)).len(), 1);
/// ```
pub struct Container {
    /// The colour drawn behind the children. A zero alpha draws nothing.
    pub background: Property<Color>,
    /// The corner radius of the background, in pixels.
    pub border_radius: Property<f32>,
    node: Handle,
}

impl Container {
    /// Creates a container that arranges its children in `mode`, in the arena,
    /// and returns it.
    ///
    /// The mode is written onto the node's [`LayoutState`] here rather than held
    /// on the widget, so the pass reads the same value the widget set. The node
    /// starts dirty, so the next layout pass places it and its children.
    ///
    /// The task file's `Container::new(mode) -> Handle` is read as this: the
    /// handle is [`Container::handle`]'s, and returning it alone would leave a
    /// caller with no properties to set and no way to add the children that are
    /// the whole of what a container is for. Task 12's
    /// [`Button::new`](crate::widgets::button::Button::new) settled the same
    /// reading.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, mode: LayoutMode) -> Self {
        let node = node::create(nodes, LayoutState::new().with_mode(mode));
        Container {
            background: Property::new(Color::new(0, 0, 0, 0)),
            border_radius: Property::new(0.0),
            node,
        }
    }

    /// Returns the container's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Sets how this container arranges its children, and marks its node dirty.
    ///
    /// The change is not visible until the next layout pass, which the dirty
    /// flag asks for: the children were placed by the old mode and only a pass
    /// over this node moves them.
    pub fn set_mode(&self, nodes: &mut Arena<WidgetNode>, mode: LayoutMode) {
        if let Some(node) = nodes.get_mut(self.node) {
            node.layout_mut().set_mode(mode);
        }
    }

    /// Sets the gap between this container's bounds and its children, and marks
    /// its node dirty.
    ///
    /// The padding is a layout input, so it lands on the node's
    /// [`LayoutState`] and the pass applies it — the same padding any other
    /// node with children could declare, and the reason a container is not
    /// special to the pass.
    pub fn set_padding(&self, nodes: &mut Arena<WidgetNode>, padding: Padding) {
        if let Some(node) = nodes.get_mut(self.node) {
            node.layout_mut().set_padding(padding);
        }
    }

    /// Sets how a flex mode arranges this container's children — their spacing
    /// and their alignment — and marks its node dirty.
    pub fn set_flex_config(&self, nodes: &mut Arena<WidgetNode>, config: FlexConfig) {
        if let Some(node) = nodes.get_mut(self.node) {
            node.layout_mut().set_flex_config(config);
        }
    }

    /// Attaches `child` to this container, and reports whether it did.
    ///
    /// A child that already has a parent, a handle the arena no longer holds,
    /// the container's own handle, and a link that would close a cycle are all
    /// refused — this is [`node::attach`] under a name that says which end is
    /// the parent.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::{LayoutMode, LayoutState};
    /// use ui_core::node::{self, WidgetNode};
    /// use ui_core::widgets::container::Container;
    ///
    /// let mut nodes = Arena::new();
    /// let outer = Container::new(&mut nodes, LayoutMode::Stack);
    /// let inner = Container::new(&mut nodes, LayoutMode::Stack);
    /// assert!(outer.add_child(&mut nodes, inner.handle()));
    ///
    /// assert!(
    ///     !outer.add_child(&mut nodes, inner.handle()),
    ///     "a node has one parent, and adding it twice is not a move"
    /// );
    /// ```
    #[must_use]
    pub fn add_child(&self, nodes: &mut Arena<WidgetNode>, child: Handle) -> bool {
        node::attach(nodes, self.node, child)
    }

    /// Detaches `child` from this container, and reports whether it was
    /// attached to it.
    ///
    /// The link is this container's alone: a child some other node holds, or
    /// one already detached, leaves this container's children in order.
    #[must_use]
    pub fn remove_child(&self, nodes: &mut Arena<WidgetNode>, child: Handle) -> bool {
        node::detach(nodes, self.node, child)
    }

    /// Returns the draw commands that paint the container's background within
    /// `rect`, or nothing at all while the background is fully transparent.
    ///
    /// One rounded rectangle, or no command: a container with no background
    /// must not put a transparent one in the batch, which is the task's
    /// "invisible unless a background is set" and is also cheaper than drawing
    /// it. A background that is *nearly* transparent is drawn — a caller fading
    /// one out gets the fade.
    ///
    /// The rect is the container's own, from
    /// [`LayoutState::rect`](crate::layout::LayoutState::rect), and the
    /// background is drawn behind the children because the paint pass walks a
    /// parent before its children.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::LayoutMode;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::property::Color;
    /// use ui_core::widgets::container::Container;
    ///
    /// let mut nodes = Arena::new();
    /// let card = Container::new(&mut nodes, LayoutMode::Stack);
    /// card.background.set(Color::new(30, 30, 30, 255));
    /// card.border_radius.set(12.0);
    ///
    /// let commands = card.paint(Rect::new(4.0, 8.0, 100.0, 40.0));
    /// assert_eq!(commands.len(), 1);
    /// ```
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let background = self.background.get();
        if background.a == 0 {
            return Vec::new();
        }
        let mut painter = Painter::new();
        painter.rounded_rect(rect, self.border_radius.get(), background);
        painter.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationClock, Easing};
    use crate::layout::{Constraints, Layout, Offset, Rect as LayoutRect, Size};
    use crate::theme::{Theme, ThemeToken};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    /// The colour a container's background is given: neither black nor white, so
    /// a test cannot pass by having painted the wrong thing.
    const CARD: Color = Color {
        r: 40,
        g: 90,
        b: 140,
        a: 255,
    };

    /// Creates a child of a fixed size, the child every test here is built from.
    fn child(nodes: &mut Arena<WidgetNode>, width: f32, height: f32) -> Handle {
        node::create(
            nodes,
            LayoutState::new().with_constraints(Constraints::tight(Size::new(width, height))),
        )
    }

    /// Creates a container in `mode` holding `children`, and returns it.
    fn container(
        nodes: &mut Arena<WidgetNode>,
        mode: LayoutMode,
        children: &[Handle],
    ) -> Container {
        let container = Container::new(nodes, mode);
        for &handle in children {
            assert!(
                container.add_child(nodes, handle),
                "a child with no parent is attached to a container that has none"
            );
        }
        container
    }

    /// Lays `root` out in a tight `size` box.
    fn layout_in(nodes: &mut Arena<WidgetNode>, root: Handle, size: Size) {
        Layout::new(nodes).layout(root, Constraints::tight(size));
    }

    /// Returns the rect `handle` was laid out to.
    ///
    /// Every test here asks where a child went rather than reaching for the
    /// node's own fields, so a failure names the child.
    fn rect_of(nodes: &Arena<WidgetNode>, handle: Handle) -> LayoutRect {
        let node = nodes
            .get(handle)
            .unwrap_or_else(|| panic!("no node at {handle:?} in the arena"));
        node.layout()
            .rect()
            .unwrap_or_else(|| panic!("{handle:?} was never laid out"))
    }

    /// Returns the rounded rectangles a paint recorded, with their radii and
    /// colours.
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

    /// Returns a background property bound to `theme`'s `Surface` token, which
    /// is what a caller themes a container's card with.
    fn surface(theme: &Theme) -> Property<Color> {
        let token = Rc::new(theme.property(ThemeToken::Surface));
        Property::bind(move || token.get().as_color().unwrap_or(Color::new(0, 0, 0, 255)))
    }

    /// A colour the `Surface` token holds in `theme`, for the tests that say
    /// where the background ends up.
    fn surface_of(theme: &Theme) -> Color {
        theme
            .get(ThemeToken::Surface)
            .as_color()
            .unwrap_or(Color::new(0, 0, 0, 255))
    }

    #[test]
    fn a_container_is_a_node_with_the_mode_it_was_given() {
        let mut nodes = Arena::new();
        let row = Container::new(&mut nodes, LayoutMode::row());

        let node = nodes.get(row.handle()).expect("a node of its own");
        assert_eq!(node.layout().mode(), LayoutMode::row());
        assert!(node.children().is_empty(), "and no children yet");
        assert!(
            node.layout().is_dirty(),
            "unplaced, so the pass has work to do"
        );
        assert_eq!(row.background.get(), Color::new(0, 0, 0, 0));
        assert_eq!(row.border_radius.get(), 0.0, "and no corner radius");
        assert!(
            row.paint(Rect::new(0.0, 0.0, 10.0, 10.0)).is_empty(),
            "so it draws nothing until it is given a background"
        );
    }

    #[test]
    fn a_row_container_places_its_children_side_by_side() {
        let mut nodes = Arena::new();
        let first = child(&mut nodes, 40.0, 20.0);
        let second = child(&mut nodes, 60.0, 20.0);
        let row = container(&mut nodes, LayoutMode::row(), &[first, second]);

        layout_in(&mut nodes, row.handle(), Size::new(200.0, 20.0));

        assert_eq!(
            rect_of(&nodes, first),
            LayoutRect::from_parts(0.0, 0.0, 40.0, 20.0)
        );
        assert_eq!(
            rect_of(&nodes, second),
            LayoutRect::from_parts(40.0, 0.0, 60.0, 20.0),
            "the second starts where the first ends"
        );
    }

    #[test]
    fn a_column_container_places_its_children_one_below_another() {
        let mut nodes = Arena::new();
        let first = child(&mut nodes, 40.0, 20.0);
        let second = child(&mut nodes, 40.0, 30.0);
        let column = container(&mut nodes, LayoutMode::column(), &[first, second]);

        layout_in(&mut nodes, column.handle(), Size::new(100.0, 200.0));

        assert_eq!(
            rect_of(&nodes, first),
            LayoutRect::from_parts(0.0, 0.0, 40.0, 20.0)
        );
        assert_eq!(
            rect_of(&nodes, second),
            LayoutRect::from_parts(0.0, 20.0, 40.0, 30.0),
            "the second starts where the first ends"
        );
    }

    #[test]
    fn a_stack_container_places_every_child_at_the_same_origin() {
        let mut nodes = Arena::new();
        let under = child(&mut nodes, 100.0, 40.0);
        let over = child(&mut nodes, 20.0, 20.0);
        let stack = container(&mut nodes, LayoutMode::Stack, &[under, over]);

        layout_in(&mut nodes, stack.handle(), Size::new(200.0, 100.0));

        assert_eq!(rect_of(&nodes, under).origin, Offset::ZERO);
        assert_eq!(
            rect_of(&nodes, over).origin,
            Offset::ZERO,
            "and both are at the origin, so the later child covers the earlier"
        );
    }

    #[test]
    fn a_padded_container_insets_its_children() {
        let mut nodes = Arena::new();
        let first = child(&mut nodes, 40.0, 20.0);
        let second = child(&mut nodes, 40.0, 20.0);
        let row = container(&mut nodes, LayoutMode::row(), &[first, second]);
        row.set_padding(&mut nodes, Padding::all(18.0));

        layout_in(&mut nodes, row.handle(), Size::new(300.0, 200.0));

        // The children are *inside* the padding, not merely the container: 18
        // from the left edge and 18 from the top, and the second child 40 — its
        // own width — further along. These are literals rather than the padding
        // read back, so a padding that was stored and never applied fails here.
        assert_eq!(
            rect_of(&nodes, first),
            LayoutRect::from_parts(18.0, 18.0, 40.0, 20.0)
        );
        assert_eq!(
            rect_of(&nodes, second),
            LayoutRect::from_parts(58.0, 18.0, 40.0, 20.0)
        );
        assert_eq!(
            rect_of(&nodes, row.handle()).size,
            Size::new(300.0, 200.0),
            "and the container is still the box it was given"
        );
    }

    #[test]
    fn a_padded_container_measures_its_children_in_the_padded_box() {
        let mut nodes = Arena::new();
        let flexible = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let row = container(&mut nodes, LayoutMode::row(), &[flexible]);
        row.set_padding(&mut nodes, Padding::all(18.0));

        layout_in(&mut nodes, row.handle(), Size::new(300.0, 100.0));

        // A flexible child takes what is left of the box the padding leaves, not
        // of the container's own: 300 less 18 on each side is 264 along the row,
        // and a flexible child fills the cross axis of the same box, which is
        // 100 less 36. The offsets in the other padded tests cannot tell this
        // apart, because a child of a fixed size lands in the same place either
        // way — what the padding changes for it is the box it is measured in.
        assert_eq!(rect_of(&nodes, flexible).size, Size::new(264.0, 64.0));
    }

    #[test]
    fn padding_set_after_a_settled_pass_still_moves_the_children() {
        // Every other padding test sets the padding before the first layout, so
        // the node was dirty anyway and the dirty-marking was never exercised.
        // This is the case a caller actually hits: a card whose padding changes
        // at runtime, on a node the pass has already laid out and settled.
        let mut nodes = Arena::new();
        let flexible = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let card = container(&mut nodes, LayoutMode::row(), &[flexible]);

        layout_in(&mut nodes, card.handle(), Size::new(300.0, 100.0));
        let before = rect_of(&nodes, flexible);
        assert_eq!(
            before.size,
            Size::new(300.0, 100.0),
            "unpadded, the flexible child takes the whole box"
        );
        assert!(
            !nodes.get(card.handle()).unwrap().layout().is_dirty(),
            "and the pass has settled, so the node is clean before the change"
        );

        card.set_padding(&mut nodes, Padding::all(18.0));
        layout_in(&mut nodes, card.handle(), Size::new(300.0, 100.0));
        let after = rect_of(&nodes, flexible);

        assert_eq!(
            after.size,
            Size::new(264.0, 64.0),
            "the child is re-measured in the padded box"
        );
        assert_eq!(after.origin.x, 18.0, "and moved by the padding");
        assert_eq!(after.origin.y, 18.0);
    }

    #[test]
    fn a_container_with_a_background_draws_one_rect() {
        let mut nodes = Arena::new();
        let only = child(&mut nodes, 40.0, 20.0);
        let card = container(&mut nodes, LayoutMode::Stack, &[only]);
        card.background.set(CARD);

        let rect = Rect::new(7.0, 9.0, 120.0, 60.0);
        assert_eq!(
            rounded(&card.paint(rect)),
            vec![(rect, 0.0, CARD)],
            "one rounded rectangle, in the container's own colour, over the rect \
             the pass gave it"
        );
    }

    #[test]
    fn a_container_without_a_background_draws_nothing() {
        let mut nodes = Arena::new();
        let card = Container::new(&mut nodes, LayoutMode::Stack);
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);

        assert!(
            card.paint(rect).is_empty(),
            "a transparent background is not drawn at all"
        );

        // A background that fades out stops being drawn, which is what makes
        // "no background" one value rather than a flag beside it.
        card.background.set(Color::new(40, 90, 140, 0));
        assert!(card.paint(rect).is_empty());
        card.background.set(Color::new(40, 90, 140, 1));
        assert_eq!(
            card.paint(rect).len(),
            1,
            "and one that is even faintly opaque is drawn"
        );
    }

    #[test]
    fn the_background_is_drawn_with_the_border_radius() {
        let mut nodes = Arena::new();
        let card = Container::new(&mut nodes, LayoutMode::Stack);
        card.background.set(CARD);
        card.border_radius.set(12.0);

        let rect = Rect::new(0.0, 0.0, 120.0, 60.0);
        assert_eq!(
            rounded(&card.paint(rect)),
            vec![(rect, 12.0, CARD)],
            "the corner radius the caller gave it, not the default"
        );
    }

    #[test]
    fn a_container_can_contain_another_container() {
        let mut nodes = Arena::new();
        let leaf = child(&mut nodes, 30.0, 10.0);
        let inner = container(&mut nodes, LayoutMode::row(), &[leaf]);
        let outer = container(&mut nodes, LayoutMode::column(), &[inner.handle()]);
        outer.set_padding(&mut nodes, Padding::all(18.0));

        layout_in(&mut nodes, outer.handle(), Size::new(200.0, 100.0));

        // The outer's padding moves the inner, and the inner's own rect is what
        // the leaf is placed against: composition, not two separate trees.
        assert_eq!(
            rect_of(&nodes, inner.handle()).origin,
            Offset::new(18.0, 18.0)
        );
        assert_eq!(
            rect_of(&nodes, leaf),
            LayoutRect::from_parts(18.0, 18.0, 30.0, 10.0),
            "and the leaf is at the inner container's own origin"
        );
    }

    #[test]
    fn a_padded_container_is_measured_as_its_content_plus_its_padding() {
        let mut nodes = Arena::new();
        let leaf = child(&mut nodes, 30.0, 10.0);
        let inner = container(&mut nodes, LayoutMode::row(), &[leaf]);
        inner.set_padding(&mut nodes, Padding::all(18.0));
        let outer = container(&mut nodes, LayoutMode::column(), &[inner.handle()]);

        // A loose box on the outer one, so both are measured from their content
        // rather than told how big to be.
        Layout::new(&mut nodes).layout(outer.handle(), Constraints::loose(Size::new(200.0, 100.0)));

        assert_eq!(
            rect_of(&nodes, inner.handle()).size,
            Size::new(66.0, 46.0),
            "30 wide and 10 tall, plus 18 on each side"
        );
        assert_eq!(
            rect_of(&nodes, outer.handle()).size,
            Size::new(66.0, 46.0),
            "and the outer container wraps it exactly, having no padding itself"
        );
        assert_eq!(
            rect_of(&nodes, leaf).origin,
            Offset::new(18.0, 18.0),
            "so the leaf is inset by the inner padding and by nothing else"
        );
    }

    #[test]
    fn set_mode_moves_the_children_to_the_new_mode() {
        let mut nodes = Arena::new();
        let first = child(&mut nodes, 40.0, 20.0);
        let second = child(&mut nodes, 40.0, 20.0);
        let card = container(&mut nodes, LayoutMode::Stack, &[first, second]);
        layout_in(&mut nodes, card.handle(), Size::new(200.0, 100.0));
        assert_eq!(rect_of(&nodes, second).origin, Offset::ZERO);

        card.set_mode(&mut nodes, LayoutMode::column());
        assert!(
            nodes.get(card.handle()).unwrap().layout().is_dirty(),
            "and the change is one the pass has to make"
        );

        layout_in(&mut nodes, card.handle(), Size::new(200.0, 100.0));
        assert_eq!(
            rect_of(&nodes, second),
            LayoutRect::from_parts(0.0, 20.0, 40.0, 20.0),
            "the same two children, now stacked"
        );
    }

    #[test]
    fn set_flex_config_spaces_the_children() {
        let mut nodes = Arena::new();
        let first = child(&mut nodes, 40.0, 20.0);
        let second = child(&mut nodes, 40.0, 20.0);
        let row = container(&mut nodes, LayoutMode::row(), &[first, second]);
        row.set_flex_config(&mut nodes, FlexConfig::new().with_spacing(10.0));

        layout_in(&mut nodes, row.handle(), Size::new(200.0, 20.0));

        assert_eq!(
            rect_of(&nodes, second).origin.x,
            50.0,
            "40 wide, then 10 of gap"
        );
    }

    #[test]
    fn a_background_bound_to_the_theme_follows_a_theme_switch() {
        // The task's "background colour animates with theme changes" is a
        // property-graph statement rather than a widget one: the container holds
        // a colour property and the graph is what carries the switch to it. A
        // `Property::bind` cannot produce an `Option`, which is the other half of
        // why the background is not one.
        let nodes = Rc::new(RefCell::new(Arena::new()));
        let theme = Theme::new();
        let mut card = Container::new(&mut nodes.borrow_mut(), LayoutMode::Stack);
        card.background = surface(&theme);
        let node = card.handle();

        // The demo's own link: a changed colour marks the node dirty, which is
        // what gets the new colour onto the screen.
        let arena = Rc::clone(&nodes);
        card.background.on_change(move |_| {
            if let Some(node) = arena.borrow_mut().get_mut(node) {
                node.paint_mut().mark_dirty();
            }
        });

        assert_eq!(card.background.get(), surface_of(&Theme::dark()));

        theme.switch_to(Theme::light(), 100);
        for _ in 0..20 {
            theme.tick(Duration::from_millis(10));
        }

        assert_eq!(
            card.background.get(),
            surface_of(&Theme::light()),
            "the background arrived at the light theme's own surface"
        );
        assert_ne!(
            surface_of(&Theme::dark()),
            surface_of(&Theme::light()),
            "and the two themes really do hold different surfaces"
        );
        assert!(
            nodes.borrow().get(node).unwrap().paint().is_dirty(),
            "and the change reached the node, which is the half a theme switch \
             needs from a widget"
        );
    }

    #[test]
    fn a_background_can_be_animated_like_any_other_property() {
        let mut nodes = Arena::new();
        let card = Container::new(&mut nodes, LayoutMode::Stack);
        let mut clock = AnimationClock::new();
        clock.add(
            card.background
                .animate_to(CARD, Duration::from_millis(100), Easing::Linear),
        );
        assert_eq!(
            card.background.get().a,
            0,
            "the fade starts where the transparent background was"
        );

        let _ = clock.tick(Duration::from_millis(50));
        let half = card.background.get();
        assert!(
            half.a > 0 && half.a < 255,
            "half way through, the alpha is in between, at {}",
            half.a
        );
        assert_eq!(
            card.paint(Rect::new(0.0, 0.0, 10.0, 10.0)).len(),
            1,
            "so a card fading in is drawn from the first frame of the fade"
        );

        let _ = clock.tick(Duration::from_millis(50));
        assert_eq!(card.background.get(), CARD);
    }

    #[test]
    fn add_child_refuses_what_attach_refuses() {
        let mut nodes = Arena::new();
        let taken = child(&mut nodes, 10.0, 10.0);
        let other = child(&mut nodes, 10.0, 10.0);
        let first = container(&mut nodes, LayoutMode::Stack, &[taken]);
        let second = container(&mut nodes, LayoutMode::Stack, &[]);

        assert!(
            !first.add_child(&mut nodes, taken),
            "a child has one parent"
        );
        assert!(first.add_child(&mut nodes, other));
        assert!(
            !first.add_child(&mut nodes, other),
            "and adding the same child twice is a duplicate, not a move"
        );
        assert!(
            second.add_child(&mut nodes, first.handle()),
            "a container may hold another container, which is what composition is"
        );
        assert!(
            !first.add_child(&mut nodes, second.handle()),
            "though the two may not hold each other"
        );
        assert_eq!(
            nodes.get(first.handle()).unwrap().children(),
            &[taken, other],
            "so the first container kept the two children it took"
        );
        assert_eq!(
            nodes.get(second.handle()).unwrap().children(),
            &[first.handle()]
        );
    }

    #[test]
    fn remove_child_detaches_and_leaves_the_rest_in_order() {
        let mut nodes = Arena::new();
        let first = child(&mut nodes, 10.0, 10.0);
        let middle = child(&mut nodes, 20.0, 10.0);
        let last = child(&mut nodes, 30.0, 10.0);
        let row = container(&mut nodes, LayoutMode::row(), &[first, middle, last]);

        assert!(row.remove_child(&mut nodes, middle));
        assert_eq!(nodes.get(row.handle()).unwrap().children(), &[first, last]);
        assert_eq!(nodes.get(middle).unwrap().parent(), None);
        assert!(
            !row.remove_child(&mut nodes, middle),
            "and a second removal changes nothing"
        );

        // The gap the removed child left is closed, which is what the survivors
        // moving is for.
        layout_in(&mut nodes, row.handle(), Size::new(200.0, 20.0));
        assert_eq!(rect_of(&nodes, last).origin.x, 10.0);
    }
}
