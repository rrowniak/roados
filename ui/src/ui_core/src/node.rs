//! The node that every widget is.
//!
//! Owns the node structure and the widget kinds the arena stores.
//!
//! A [`WidgetNode`] is the arena's element. It holds the children a
//! [`LayoutMode`](crate::layout::LayoutMode) arranges, the parent that the
//! dirty walk climbs, a [`LayoutState`] carrying the node's layout inputs and
//! the rect the last pass computed for it, and a [`PaintState`] carrying the
//! draw commands the paint pass recorded for it.
//!
//! The shape of the tree is edited through [`create`], [`attach`] and
//! [`detach`], which take the arena. A node cannot reach the arena that holds
//! it, so a mutable getter for the child list would be the only way to break
//! the parent links that [`layout::mark_dirty`](crate::layout::mark_dirty) and
//! the pass's own recursion both walk. The state a node owns is still reachable
//! directly, through [`WidgetNode::layout_mut`] and
//! [`WidgetNode::paint_mut`].
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::layout::{Constraints, LayoutMode, LayoutState, Size};
//! use ui_core::node::{self, WidgetNode};
//!
//! let mut nodes = Arena::new();
//! let row = node::create(&mut nodes, LayoutState::new().with_mode(LayoutMode::row()));
//! let child = node::create(
//!     &mut nodes,
//!     LayoutState::new().with_constraints(Constraints::tight(Size::new(30.0, 10.0))),
//! );
//!
//! assert!(node::attach(&mut nodes, row, child));
//! assert_eq!(nodes.get(row).unwrap().children(), &[child]);
//! assert_eq!(nodes.get(child).unwrap().parent(), Some(row));
//! ```

use crate::arena::{Arena, Handle};
use crate::layout::{mark_dirty, LayoutState};
use crate::paint::PaintState;

/// One node of the widget tree.
///
/// The children are in the order their parent arranges them, which is the
/// order paint records them in and so the order they appear on screen. A
/// [`LayoutMode::Stack`](crate::layout::LayoutMode::Stack) places them all at
/// the same rect, and the later one covers the earlier.
pub struct WidgetNode {
    children: Vec<Handle>,
    parent: Option<Handle>,
    layout: LayoutState,
    paint: PaintState,
}

impl WidgetNode {
    /// Returns the children in the order they are placed in.
    #[must_use]
    pub fn children(&self) -> &[Handle] {
        &self.children
    }

    /// Returns the node this one is attached to, or `None` for a root.
    #[must_use]
    pub fn parent(&self) -> Option<Handle> {
        self.parent
    }

    /// Returns the layout inputs and the cached rect.
    #[must_use]
    pub fn layout(&self) -> &LayoutState {
        &self.layout
    }

    /// Returns the layout inputs and the cached rect for modification.
    ///
    /// Changing an input through a `set_` method marks this node dirty, which
    /// only reaches the rest of the tree through
    /// [`layout::mark_dirty`](crate::layout::mark_dirty).
    pub fn layout_mut(&mut self) -> &mut LayoutState {
        &mut self.layout
    }

    /// Returns the recorded draw commands.
    #[must_use]
    pub fn paint(&self) -> &PaintState {
        &self.paint
    }

    /// Returns the recorded draw commands for modification.
    pub fn paint_mut(&mut self) -> &mut PaintState {
        &mut self.paint
    }
}

/// Adds a node with `layout` as its inputs and no parent, and returns its
/// handle.
///
/// The node starts dirty, so the next pass lays it out.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::layout::LayoutState;
/// use ui_core::node::{self, WidgetNode};
///
/// let mut nodes = Arena::new();
/// let handle = node::create(&mut nodes, LayoutState::new());
///
/// assert_eq!(nodes.get(handle).unwrap().parent(), None);
/// assert!(nodes.get(handle).unwrap().layout().is_dirty());
/// ```
#[must_use]
pub fn create(nodes: &mut Arena<WidgetNode>, layout: LayoutState) -> Handle {
    nodes.insert(WidgetNode {
        children: Vec::new(),
        parent: None,
        layout,
        paint: PaintState::new(),
    })
}

/// Attaches `child` to `parent`, and returns whether it did.
///
/// The child goes last, which is where a flex mode places it. A handle that no
/// longer resolves, a child that already has a parent or is already in
/// `parent`'s list, `parent` itself, and a link that would close a cycle are
/// all refused: the function reports `false` and changes nothing.
///
/// A node the arena no longer holds cannot be checked, and a parent that
/// still lists one is not repaired here. The pass tolerates a stale child
/// handle — it lays the rest of the row out around a rect of no size — so
/// removing a node with [`Arena::remove`] is safe, though detaching it first
/// keeps the parent's own children list honest.
///
/// The link is a layout change, so the parent and everything above it are
/// marked dirty: a laid-out parent would otherwise skip the pass and the new
/// child would never be placed.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::layout::LayoutState;
/// use ui_core::node::{self, WidgetNode};
///
/// let mut nodes = Arena::new();
/// let parent = node::create(&mut nodes, LayoutState::new());
/// let child = node::create(&mut nodes, LayoutState::new());
///
/// assert!(node::attach(&mut nodes, parent, child));
/// assert!(
///     !node::attach(&mut nodes, parent, child),
///     "a second link is a duplicate, not a move"
/// );
/// ```
#[must_use]
pub fn attach(nodes: &mut Arena<WidgetNode>, parent: Handle, child: Handle) -> bool {
    if parent == child || !can_attach(nodes, parent, child) {
        return false;
    }
    if let Some(parent_node) = nodes.get_mut(parent) {
        parent_node.children.push(child);
    }
    if let Some(child_node) = nodes.get_mut(child) {
        child_node.parent = Some(parent);
    }
    mark_dirty(nodes, parent);
    true
}

/// Returns `true` if `parent` may take `child`.
fn can_attach(nodes: &Arena<WidgetNode>, parent: Handle, child: Handle) -> bool {
    let (Some(parent_node), Some(child_node)) = (nodes.get(parent), nodes.get(child)) else {
        return false;
    };
    child_node.parent.is_none()
        && !parent_node.children().contains(&child)
        && !descends_from(nodes, child, parent)
}

/// Returns `true` if `ancestor` is `node` or one of its ancestors.
fn descends_from(nodes: &Arena<WidgetNode>, ancestor: Handle, node: Handle) -> bool {
    let mut current = Some(node);
    while let Some(handle) = current {
        if handle == ancestor {
            return true;
        }
        current = nodes.get(handle).and_then(WidgetNode::parent);
    }
    false
}

/// Detaches `child` from `parent`, and returns whether it was attached.
///
/// Only the link `parent` — `child` is removed; a child that some other node
/// holds, or one that is already detached, leaves `parent` alone.
///
/// The surviving children move, so the parent and everything above it are
/// marked dirty. `child` is marked too: it is a root now, and the rect it
/// cached belonged to the tree it just left, so nothing should read it as
/// current.
#[must_use]
pub fn detach(nodes: &mut Arena<WidgetNode>, parent: Handle, child: Handle) -> bool {
    let attached = nodes
        .get(child)
        .and_then(WidgetNode::parent)
        .is_some_and(|holder| holder == parent);
    if !attached {
        return false;
    }
    if let Some(parent_node) = nodes.get_mut(parent) {
        parent_node.children.retain(|&handle| handle != child);
    }
    if let Some(child_node) = nodes.get_mut(child) {
        child_node.parent = None;
    }
    mark_dirty(nodes, parent);
    mark_dirty(nodes, child);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Constraints, FlexConfig, Layout, LayoutMode, Rect, Size};

    /// Creates a node with no layout inputs beyond the default.
    fn bare(nodes: &mut Arena<WidgetNode>) -> Handle {
        create(nodes, LayoutState::new())
    }

    /// Creates a fixed-size leaf.
    fn leaf(nodes: &mut Arena<WidgetNode>, size: Size) -> Handle {
        create(
            nodes,
            LayoutState::new().with_constraints(Constraints::tight(size)),
        )
    }

    /// Creates a row holding `children`, and returns its handle.
    fn row(nodes: &mut Arena<WidgetNode>, config: FlexConfig, children: &[Handle]) -> Handle {
        let handle = create(
            nodes,
            LayoutState::new()
                .with_mode(LayoutMode::row())
                .with_flex_config(config),
        );
        for &child in children {
            assert!(attach(nodes, handle, child));
        }
        handle
    }

    /// Creates a column holding `children`, and returns its handle.
    fn column(nodes: &mut Arena<WidgetNode>, config: FlexConfig, children: &[Handle]) -> Handle {
        let handle = create(
            nodes,
            LayoutState::new()
                .with_mode(LayoutMode::column())
                .with_flex_config(config),
        );
        for &child in children {
            assert!(attach(nodes, handle, child));
        }
        handle
    }

    /// Lays `root` out in `size` and returns the rect of `handle`.
    fn rect_of(nodes: &mut Arena<WidgetNode>, root: Handle, size: Size, handle: Handle) -> Rect {
        Layout::new(nodes).layout(root, Constraints::tight(size));
        nodes.get(handle).unwrap().layout().rect().unwrap()
    }

    #[test]
    fn a_new_node_is_a_dirty_root() {
        let mut nodes = Arena::new();
        let handle = bare(&mut nodes);

        let node = nodes.get(handle).unwrap();
        assert!(node.children().is_empty());
        assert_eq!(node.parent(), None);
        assert!(
            node.layout().is_dirty(),
            "an unplaced node has to be laid out"
        );
        assert!(node.paint().commands().is_empty());
        assert!(!node.paint().is_dirty());
    }

    #[test]
    fn attach_links_both_ends_and_appends() {
        let mut nodes = Arena::new();
        let parent = bare(&mut nodes);
        let first = bare(&mut nodes);
        let second = bare(&mut nodes);

        assert!(attach(&mut nodes, parent, first));
        assert!(attach(&mut nodes, parent, second));

        assert_eq!(nodes.get(parent).unwrap().children(), &[first, second]);
        assert_eq!(nodes.get(first).unwrap().parent(), Some(parent));
        assert_eq!(nodes.get(second).unwrap().parent(), Some(parent));
    }

    #[test]
    fn attach_refuses_a_stale_handle_on_either_end() {
        let mut nodes = Arena::new();
        let parent = bare(&mut nodes);
        let child = bare(&mut nodes);
        let dead = bare(&mut nodes);
        assert!(nodes.remove(dead).is_some());

        assert!(!attach(&mut nodes, dead, child), "no stale parent");
        assert!(!attach(&mut nodes, parent, dead), "no stale child");
        assert!(nodes.get(child).unwrap().children().is_empty());
        assert_eq!(nodes.get(child).unwrap().parent(), None);
    }

    #[test]
    fn attach_refuses_a_child_that_already_has_a_parent() {
        let mut nodes = Arena::new();
        let first = bare(&mut nodes);
        let second = bare(&mut nodes);
        let child = bare(&mut nodes);

        assert!(attach(&mut nodes, first, child));
        assert!(!attach(&mut nodes, second, child), "a child has one parent");

        assert_eq!(nodes.get(first).unwrap().children(), &[child]);
        assert!(nodes.get(second).unwrap().children().is_empty());
        assert_eq!(nodes.get(child).unwrap().parent(), Some(first));
    }

    #[test]
    fn attach_refuses_a_duplicate_link() {
        let mut nodes = Arena::new();
        let parent = bare(&mut nodes);
        let child = bare(&mut nodes);

        assert!(attach(&mut nodes, parent, child));
        assert!(!attach(&mut nodes, parent, child));

        assert_eq!(nodes.get(parent).unwrap().children(), &[child]);
    }

    #[test]
    fn attach_refuses_a_node_onto_itself() {
        let mut nodes = Arena::new();
        let handle = bare(&mut nodes);

        assert!(!attach(&mut nodes, handle, handle));
        assert!(nodes.get(handle).unwrap().children().is_empty());
        assert_eq!(nodes.get(handle).unwrap().parent(), None);
    }

    #[test]
    fn attach_refuses_a_link_that_would_close_a_cycle() {
        let mut nodes = Arena::new();
        let outer = bare(&mut nodes);
        let inner = bare(&mut nodes);
        let leaf = bare(&mut nodes);
        assert!(attach(&mut nodes, outer, inner));
        assert!(attach(&mut nodes, inner, leaf));

        assert!(!attach(&mut nodes, leaf, outer), "outer is above leaf");
        assert!(!attach(&mut nodes, leaf, inner), "inner is above leaf");
        assert!(!attach(&mut nodes, inner, outer), "a direct cycle");

        assert_eq!(nodes.get(outer).unwrap().children(), &[inner]);
        assert_eq!(nodes.get(inner).unwrap().children(), &[leaf]);
        assert_eq!(nodes.get(leaf).unwrap().parent(), Some(inner));
    }

    #[test]
    fn detach_unlinks_both_ends() {
        let mut nodes = Arena::new();
        let parent = bare(&mut nodes);
        let child = bare(&mut nodes);
        assert!(attach(&mut nodes, parent, child));

        assert!(detach(&mut nodes, parent, child));

        assert!(nodes.get(parent).unwrap().children().is_empty());
        assert_eq!(nodes.get(child).unwrap().parent(), None);
    }

    #[test]
    fn detach_leaves_the_other_children_in_order() {
        let mut nodes = Arena::new();
        let parent = bare(&mut nodes);
        let first = bare(&mut nodes);
        let second = bare(&mut nodes);
        let third = bare(&mut nodes);
        assert!(attach(&mut nodes, parent, first));
        assert!(attach(&mut nodes, parent, second));
        assert!(attach(&mut nodes, parent, third));

        assert!(detach(&mut nodes, parent, second));

        assert_eq!(nodes.get(parent).unwrap().children(), &[first, third]);
    }

    #[test]
    fn detach_refuses_a_child_of_another_parent() {
        let mut nodes = Arena::new();
        let parent = bare(&mut nodes);
        let other = bare(&mut nodes);
        let child = bare(&mut nodes);
        assert!(attach(&mut nodes, other, child));

        assert!(!detach(&mut nodes, parent, child));
        assert_eq!(nodes.get(other).unwrap().children(), &[child]);
        assert_eq!(nodes.get(child).unwrap().parent(), Some(other));
    }

    #[test]
    fn detach_refuses_an_unattached_or_stale_handle() {
        let mut nodes = Arena::new();
        let parent = bare(&mut nodes);
        let child = bare(&mut nodes);
        let dead = bare(&mut nodes);
        assert!(nodes.remove(dead).is_some());

        assert!(!detach(&mut nodes, parent, child), "never attached");
        assert!(!detach(&mut nodes, dead, child), "stale parent");
    }

    #[test]
    fn a_detached_node_can_be_attached_again() {
        let mut nodes = Arena::new();
        let first = bare(&mut nodes);
        let second = bare(&mut nodes);
        let child = bare(&mut nodes);
        assert!(attach(&mut nodes, first, child));
        assert!(detach(&mut nodes, first, child));

        assert!(attach(&mut nodes, second, child));
        assert_eq!(nodes.get(first).unwrap().children(), &[] as &[Handle]);
        assert_eq!(nodes.get(second).unwrap().children(), &[child]);
    }

    #[test]
    fn layout_inputs_and_paint_state_are_reachable() {
        let mut nodes = Arena::new();
        let handle = create(&mut nodes, LayoutState::new().with_mode(LayoutMode::row()));

        assert_eq!(
            nodes.get(handle).unwrap().layout().mode(),
            LayoutMode::row()
        );

        nodes
            .get_mut(handle)
            .unwrap()
            .layout_mut()
            .set_constraints(Constraints::tight(Size::new(10.0, 10.0)));
        assert_eq!(
            nodes.get(handle).unwrap().layout().constraints(),
            Constraints::tight(Size::new(10.0, 10.0))
        );
        assert!(
            nodes.get(handle).unwrap().layout().is_dirty(),
            "changing an input marks the node"
        );

        nodes.get_mut(handle).unwrap().paint_mut().mark_dirty();
        assert!(nodes.get(handle).unwrap().paint().is_dirty());
    }

    #[test]
    fn attaching_after_a_pass_places_the_new_child() {
        let mut nodes = Arena::new();
        let first = leaf(&mut nodes, Size::new(10.0, 10.0));
        let root = row(&mut nodes, FlexConfig::new(), &[first]);
        let size = Size::new(100.0, 20.0);
        assert_eq!(
            rect_of(&mut nodes, root, size, first),
            Rect::from_parts(0.0, 0.0, 10.0, 10.0)
        );

        // The pass already ran, so the root is clean. The new child still has to
        // be placed, which only happens if the attach reached the root.
        let second = leaf(&mut nodes, Size::new(60.0, 20.0));
        assert!(attach(&mut nodes, root, second));

        assert_eq!(
            rect_of(&mut nodes, root, size, second),
            Rect::from_parts(10.0, 0.0, 60.0, 20.0)
        );
        assert_eq!(
            rect_of(&mut nodes, root, size, first),
            Rect::from_parts(0.0, 0.0, 10.0, 10.0),
            "the sibling before the new one does not move"
        );
    }

    #[test]
    fn reattaching_a_clean_node_places_it_again() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, Size::new(30.0, 20.0));
        let b = leaf(&mut nodes, Size::new(10.0, 20.0));
        let root = row(&mut nodes, FlexConfig::new(), &[a, b]);
        let size = Size::new(100.0, 20.0);
        assert_eq!(
            rect_of(&mut nodes, root, size, a),
            Rect::from_parts(0.0, 0.0, 30.0, 20.0)
        );

        // Take `a` out and lay it out on its own, which is legal public API and
        // leaves it clean: it holds the rect it was given as a root, and the
        // layout walk will not look for dirt in a subtree it is told is clean.
        assert!(detach(&mut nodes, root, a));
        Layout::new(&mut nodes).layout(a, Constraints::tight(size));
        assert!(!nodes.get(a).unwrap().layout().is_dirty());

        // The detach dirtied the row, so settle it again. This is the step that
        // gives the test its teeth: with the row still dirty at the moment of
        // the reattach, the pass would re-lay-out everything anyway and the
        // missing invalidation in `attach` would never show.
        Layout::new(&mut nodes).layout(root, Constraints::tight(size));
        assert!(
            !nodes.get(root).unwrap().layout().is_dirty(),
            "the row is clean before the node comes back, so `attach` is the \
             only thing that can dirty it again"
        );

        // Putting it back has to invalidate the parent all the same, or `a`
        // keeps the rect it was given as a root — 100 pixels wide at the
        // origin — and the two children share an origin.
        assert!(attach(&mut nodes, root, a));

        // `attach` appends, so `b` is now first and the row is 10 + 30 wide with
        // 60 pixels to spare.
        assert_eq!(
            rect_of(&mut nodes, root, size, b),
            Rect::from_parts(0.0, 0.0, 10.0, 20.0)
        );
        assert_eq!(
            rect_of(&mut nodes, root, size, a),
            Rect::from_parts(10.0, 0.0, 30.0, 20.0),
            "the node that came back is placed in the row again"
        );
    }

    #[test]
    fn detaching_after_a_pass_closes_the_gap() {
        let mut nodes = Arena::new();
        let first = leaf(&mut nodes, Size::new(10.0, 10.0));
        let middle = leaf(&mut nodes, Size::new(10.0, 10.0));
        let last = leaf(&mut nodes, Size::new(10.0, 10.0));
        let root = row(
            &mut nodes,
            FlexConfig::new().with_spacing(10.0),
            &[first, middle, last],
        );
        let size = Size::new(100.0, 20.0);
        assert_eq!(
            rect_of(&mut nodes, root, size, last),
            Rect::from_parts(40.0, 0.0, 10.0, 10.0)
        );

        assert!(detach(&mut nodes, root, middle));

        assert_eq!(
            rect_of(&mut nodes, root, size, last),
            Rect::from_parts(20.0, 0.0, 10.0, 10.0),
            "the child behind the removed one moves up into its place"
        );
        assert_eq!(
            rect_of(&mut nodes, root, size, first),
            Rect::from_parts(0.0, 0.0, 10.0, 10.0)
        );
    }

    #[test]
    fn a_detached_node_no_longer_claims_a_valid_rect() {
        let mut nodes = Arena::new();
        let only = leaf(&mut nodes, Size::new(10.0, 10.0));
        let root = row(&mut nodes, FlexConfig::new(), &[only]);
        let size = Size::new(100.0, 20.0);
        rect_of(&mut nodes, root, size, only);

        assert!(detach(&mut nodes, root, only));

        // It is a root now, and no pass reaches it, so its old rect has to be
        // marked stale rather than left looking current.
        assert!(
            nodes.get(only).unwrap().layout().is_dirty(),
            "a detached node's cached rect belongs to the tree it left"
        );
    }

    #[test]
    fn editing_a_subtree_below_a_clean_root_reaches_the_root() {
        let mut nodes = Arena::new();
        let first = leaf(&mut nodes, Size::new(10.0, 10.0));
        let inner = row(&mut nodes, FlexConfig::new(), &[first]);
        let root = column(&mut nodes, FlexConfig::new(), &[inner]);
        let size = Size::new(100.0, 40.0);
        assert_eq!(
            rect_of(&mut nodes, root, size, inner),
            Rect::from_parts(0.0, 0.0, 10.0, 10.0)
        );

        // Neither the root nor the inner row is dirty here; the invalidation has
        // to climb all the way, or the new leaf is never placed.
        let second = leaf(&mut nodes, Size::new(60.0, 20.0));
        assert!(attach(&mut nodes, inner, second));

        assert_eq!(
            rect_of(&mut nodes, root, size, second),
            Rect::from_parts(10.0, 0.0, 60.0, 20.0)
        );
        assert_eq!(
            rect_of(&mut nodes, root, size, inner),
            Rect::from_parts(0.0, 0.0, 70.0, 20.0),
            "the container grows around the child that joined it"
        );
    }

    #[test]
    fn editing_a_deep_subtree_re_measures_the_ancestors() {
        let mut nodes = Arena::new();
        let deep = leaf(&mut nodes, Size::new(10.0, 10.0));
        let middle = row(&mut nodes, FlexConfig::new(), &[deep]);
        let inner = row(&mut nodes, FlexConfig::new(), &[middle]);
        let root = column(&mut nodes, FlexConfig::new(), &[inner]);
        let size = Size::new(100.0, 40.0);
        assert_eq!(
            rect_of(&mut nodes, root, size, deep),
            Rect::from_parts(0.0, 0.0, 10.0, 10.0)
        );

        assert!(detach(&mut nodes, middle, deep));

        // The root's own box is the window and does not change, so nothing about
        // it can reveal that anything below it did: the dirty flags are the only
        // thing carrying the edit down, and the two rows it emptied shrink.
        assert_eq!(
            rect_of(&mut nodes, root, size, root),
            Rect::from_parts(0.0, 0.0, 100.0, 40.0)
        );
        assert_eq!(rect_of(&mut nodes, root, size, middle), Rect::ZERO);
        assert_eq!(rect_of(&mut nodes, root, size, inner), Rect::ZERO);
        assert!(
            nodes.get(deep).unwrap().layout().is_dirty(),
            "the detached node is not part of the tree any more"
        );
    }
}
