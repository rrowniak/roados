//! First-class panel objects for the demo page.
//!
//! A [`Panel`] is a region of a screen: it owns a rectangle **in its parent's
//! coordinates** and a subtree of widgets. Every widget inside a panel is laid
//! out relative to the panel's own origin, so the whole object can be
//! **resized** and **repositioned** in one call and its subtree follows — a
//! widget never names a window coordinate.
//!
//! The demo page is four of them: the **map background**, the **top status
//! bar**, the **car-status pane** and the **bottom dock**. The object model is
//! what lets a later task move or resize a panel (drag the pane wider, collapse
//! the status bar, swap the dock to the top) without reaching into any child.

use ui_core::arena::{Arena, Handle};
use ui_core::layout::{mark_dirty, Constraints, LayoutMode, Offset, Size};
use ui_core::node::{self, WidgetNode};
use ui_core::paint::Rect;
use ui_core::widgets::container::Container;

/// A rectangle-owning region of a screen with widgets relative to its origin.
pub struct Panel {
    /// The node the panel is drawn on: a `Container` for the chrome, the map
    /// `Image` for the background.
    node: Handle,
    /// The panel's rectangle, **relative to its parent's origin**.
    rect: Rect,
}

impl Panel {
    /// Creates a panel node in `mode` and gives it `rect`, relative to its
    /// parent.
    ///
    /// The returned [`Container`] is the caller's to configure — flex, padding,
    /// background, radius — before it is dropped; the panel holds only the node
    /// and the rect, so a region that is not a container (the map image) can be
    /// a panel too.
    #[must_use]
    pub fn container(
        nodes: &mut Arena<WidgetNode>,
        rect: Rect,
        mode: LayoutMode,
    ) -> (Self, Container) {
        let container = Container::new(nodes, mode);
        let mut panel = Panel {
            node: container.handle(),
            rect,
        };
        panel.set_rect(nodes, rect);
        (panel, container)
    }

    /// Wraps an existing node — a widget the demo built itself — as a panel.
    #[must_use]
    pub fn wrapping(node: Handle, rect: Rect) -> Self {
        Panel { node, rect }
    }

    /// Returns the panel's node.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the panel's rectangle, relative to its parent.
    #[must_use]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Returns whether this panel is the one at `handle`.
    #[must_use]
    pub fn is(&self, handle: Handle) -> bool {
        self.node == handle
    }

    /// Moves and resizes the panel to `rect`, **in its parent's coordinates**.
    ///
    /// This is the one operation a resize or a reposition is: the subtree is
    /// re-laid out against the new box by the next layout pass, which this marks
    /// dirty.
    pub fn set_rect(&mut self, nodes: &mut Arena<WidgetNode>, rect: Rect) {
        self.rect = rect;
        if let Some(node) = nodes.get_mut(self.node) {
            node.layout_mut()
                .set_constraints(Constraints::tight(Size::new(rect.width, rect.height)));
            node.layout_mut()
                .set_position(Some(Offset::new(rect.x, rect.y)));
            mark_dirty(nodes, self.node);
        }
    }

    /// Attaches `child` to the panel.
    pub fn add_child(&self, nodes: &mut Arena<WidgetNode>, child: Handle) -> bool {
        node::attach(nodes, self.node, child)
    }

    /// Places `child` at `local`, a rectangle **relative to the panel's own
    /// origin**.
    ///
    /// Only a panel that arranges its children in [`LayoutMode::Absolute`] reads
    /// this position, and the position it reads is relative to the panel — which
    /// is the whole of *"a widget positioned relative to its parent object"*.
    #[allow(dead_code)]
    pub fn place_child(&self, nodes: &mut Arena<WidgetNode>, child: Handle, local: Rect) {
        if let Some(node) = nodes.get_mut(child) {
            node.layout_mut()
                .set_constraints(Constraints::tight(Size::new(local.width, local.height)));
            node.layout_mut()
                .set_position(Some(Offset::new(local.x, local.y)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::layout::{Layout, LayoutState};

    /// The child's placed origin, or a panic that says why it is missing.
    fn origin(nodes: &Arena<WidgetNode>, handle: Handle) -> (f32, f32) {
        let rect = nodes
            .get(handle)
            .expect("the node is live")
            .layout()
            .rect()
            .expect("the node is placed");
        (rect.origin.x, rect.origin.y)
    }

    #[test]
    fn a_panel_carries_its_rect_and_the_node_it_wraps() {
        let mut nodes: Arena<WidgetNode> = Arena::new();
        let (panel, container) = Panel::container(
            &mut nodes,
            Rect::new(10.0, 20.0, 300.0, 50.0),
            LayoutMode::row(),
        );
        assert_eq!(panel.handle(), container.handle());
        assert_eq!(panel.rect(), Rect::new(10.0, 20.0, 300.0, 50.0));
        assert!(panel.is(container.handle()));
    }

    #[test]
    fn moving_and_resizing_a_panel_keeps_its_widget_at_the_same_offset_inside_it() {
        let mut nodes: Arena<WidgetNode> = Arena::new();
        // A root, so the panel has a parent to be positioned against.
        let root = Container::new(&mut nodes, LayoutMode::Absolute);
        let (mut panel, _container) = Panel::container(
            &mut nodes,
            Rect::new(0.0, 0.0, 200.0, 100.0),
            LayoutMode::Absolute,
        );
        let child = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::tight(Size::new(40.0, 20.0))),
        );
        // A widget 30 across and 10 down **inside** the panel.
        panel.place_child(&mut nodes, child, Rect::new(30.0, 10.0, 40.0, 20.0));
        assert!(panel.add_child(&mut nodes, child));
        assert!(root.add_child(&mut nodes, panel.handle()));

        let layout = |nodes: &mut Arena<WidgetNode>| {
            Layout::new(nodes).layout(root.handle(), Constraints::tight(Size::new(1000.0, 1000.0)));
        };
        layout(&mut nodes);
        assert_eq!(
            origin(&nodes, child),
            (30.0, 10.0),
            "the widget's local offset"
        );

        // Move and grow the panel: the widget keeps its local offset.
        panel.set_rect(&mut nodes, Rect::new(50.0, 70.0, 260.0, 140.0));
        layout(&mut nodes);
        assert_eq!(
            origin(&nodes, child),
            (50.0 + 30.0, 70.0 + 10.0),
            "the widget followed its panel and kept the offset inside it"
        );
    }
}
