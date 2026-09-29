//! Layout.
//!
//! Owns the layout modes a node may use, and the pass that computes the rects
//! they imply for the dirty subtrees.
//!
//! Layout is constraint-based: a parent hands each child a [`Constraints`] box
//! and the child takes a size inside it — the size it asks for, clamped into
//! the box. Asking and placing are the same code (`resolve_box` decides the box
//! a node lays *its* children out in, and the parent measures the child with
//! the same call), so a parent and a child never disagree about a size.
//!
//! Rects are absolute in the coordinate space the root's constraints describe;
//! the root sits at the origin. The paint pass can therefore use a computed
//! rect without a second traversal. [`paint::Rect`] is a separate type for the
//! same rectangle on purpose — see the conversion impl below.
//!
//! # Examples
//!
//! ```
//! use ui_core::arena::Arena;
//! use ui_core::layout::{Constraints, FlexConfig, Layout, LayoutMode, LayoutState, Size};
//! use ui_core::node::{self, WidgetNode};
//!
//! let mut nodes = Arena::new();
//! let row = node::create(
//!     &mut nodes,
//!     LayoutState::new()
//!         .with_mode(LayoutMode::row())
//!         .with_flex_config(FlexConfig::new().with_spacing(10.0)),
//! );
//! let left = node::create(
//!     &mut nodes,
//!     LayoutState::new().with_constraints(Constraints::tight(Size::new(100.0, 40.0))),
//! );
//! let right = node::create(
//!     &mut nodes,
//!     LayoutState::new().with_constraints(Constraints::tight(Size::new(200.0, 40.0))),
//! );
//! assert!(node::attach(&mut nodes, row, left));
//! assert!(node::attach(&mut nodes, row, right));
//!
//! Layout::new(&mut nodes).layout(row, Constraints::tight(Size::new(400.0, 40.0)));
//!
//! let left_rect = nodes.get(left).unwrap().layout().rect().unwrap();
//! let right_rect = nodes.get(right).unwrap().layout().rect().unwrap();
//! assert_eq!(left_rect.size, Size::new(100.0, 40.0));
//! assert_eq!(right_rect.origin.x, 110.0);
//! ```

use crate::arena::{Arena, Handle};
use crate::node::WidgetNode;
use crate::paint;

/// A width and a height, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

impl Size {
    /// A size of zero on both axes.
    pub const ZERO: Size = Size {
        width: 0.0,
        height: 0.0,
    };

    /// Creates a size.
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        Size { width, height }
    }

    /// Returns the extent of this size along `direction`'s main axis.
    fn main(self, direction: FlexDirection) -> f32 {
        match direction {
            FlexDirection::Row => self.width,
            FlexDirection::Column => self.height,
        }
    }

    /// Returns the extent of this size along `direction`'s cross axis.
    fn cross(self, direction: FlexDirection) -> f32 {
        match direction {
            FlexDirection::Row => self.height,
            FlexDirection::Column => self.width,
        }
    }

    /// Builds a size from a main-axis and a cross-axis extent.
    fn from_main_cross(direction: FlexDirection, main: f32, cross: f32) -> Self {
        match direction {
            FlexDirection::Row => Size::new(main, cross),
            FlexDirection::Column => Size::new(cross, main),
        }
    }
}

/// A point, in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Offset {
    /// Distance from the left edge of the coordinate space.
    pub x: f32,
    /// Distance from the top edge of the coordinate space.
    pub y: f32,
}

impl Offset {
    /// The origin of a coordinate space.
    pub const ZERO: Offset = Offset { x: 0.0, y: 0.0 };

    /// Creates an offset.
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        Offset { x, y }
    }
}

/// An axis-aligned rectangle: a position and an extent.
///
/// This is the layout pass's rectangle, in the coordinate space the root's
/// constraints describe. A draw command's rectangle is [`paint::Rect`]; the two
/// are kept apart so a paint-time transform can move one without touching the
/// layout cache, and the conversion between them is explicit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// The top-left corner.
    pub origin: Offset,
    /// The extent from `origin`.
    pub size: Size,
}

impl Rect {
    /// A zero-size rectangle at the origin.
    pub const ZERO: Rect = Rect {
        origin: Offset::ZERO,
        size: Size::ZERO,
    };

    /// Creates a rectangle from its origin and its size.
    #[must_use]
    pub fn new(origin: Offset, size: Size) -> Self {
        Rect { origin, size }
    }

    /// Creates a rectangle from its four edges.
    #[must_use]
    pub fn from_parts(x: f32, y: f32, width: f32, height: f32) -> Self {
        Rect {
            origin: Offset::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// Returns the bottom-right corner.
    #[must_use]
    pub fn far_corner(self) -> Offset {
        Offset::new(
            self.origin.x + self.size.width,
            self.origin.y + self.size.height,
        )
    }
}

/// Converts a layout rectangle into the rectangle a draw command carries.
///
/// The conversion is identity on the four coordinates, and it is spelled out as
/// a `From` impl rather than a helper so the one place where layout geometry
/// becomes draw-command geometry is visible in the API.
///
/// # Examples
///
/// ```
/// use ui_core::layout::{Offset, Rect, Size};
/// use ui_core::paint;
///
/// let rect = Rect::new(Offset::new(4.0, 8.0), Size::new(20.0, 10.0));
/// let draw_rect = paint::Rect::from(rect);
/// assert_eq!(
///     (draw_rect.x, draw_rect.y, draw_rect.width, draw_rect.height),
///     (4.0, 8.0, 20.0, 10.0),
/// );
/// ```
impl From<Rect> for paint::Rect {
    fn from(rect: Rect) -> Self {
        paint::Rect::new(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        )
    }
}

/// The axis a flex container lays its children out along.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlexDirection {
    /// Children run left to right; the main axis is horizontal.
    Row,
    /// Children run top to bottom; the main axis is vertical.
    Column,
}

/// How a flex container distributes the space left over once its children are
/// sized.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MainAxisAlignment {
    /// Pack the children against the start of the main axis.
    #[default]
    Start,
    /// Pack the children against the end of the main axis.
    End,
    /// Centre the children on the main axis.
    Center,
    /// Put the leftover space between the children, none at the ends.
    SpaceBetween,
    /// Put half of a gap at each end and a full one between the children.
    SpaceAround,
    /// Put an equal gap at the ends and between the children.
    SpaceEvenly,
}

/// How a flex container places a child across the cross axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CrossAxisAlignment {
    /// Align the child's near edge with the container's.
    #[default]
    Start,
    /// Align the child's far edge with the container's.
    End,
    /// Centre the child across the container.
    Center,
    /// Give the child the container's whole cross extent.
    Stretch,
}

/// How a flex container arranges its children, beyond the direction.
///
/// These are properties of the relationship between the children rather than of
/// any one of them, so they belong to the node that holds them: a flex factor
/// and a declared size travel with the child, alignment and spacing with the
/// parent.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlexConfig {
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    spacing: f32,
}

impl FlexConfig {
    /// Creates a config that packs children at the start with no spacing.
    #[must_use]
    pub fn new() -> Self {
        FlexConfig {
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Start,
            spacing: 0.0,
        }
    }

    /// Sets the main-axis alignment.
    #[must_use]
    pub fn with_main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = alignment;
        self
    }

    /// Sets the cross-axis alignment.
    #[must_use]
    pub fn with_cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }

    /// Sets the gap between children on the main axis.
    ///
    /// A negative spacing is clamped to zero: overlapping children are what
    /// [`LayoutMode::Stack`] is for.
    #[must_use]
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    /// Returns the main-axis alignment.
    #[must_use]
    pub fn main_axis_alignment(self) -> MainAxisAlignment {
        self.main_axis_alignment
    }

    /// Returns the cross-axis alignment.
    #[must_use]
    pub fn cross_axis_alignment(self) -> CrossAxisAlignment {
        self.cross_axis_alignment
    }

    /// Returns the gap between children on the main axis.
    #[must_use]
    pub fn spacing(self) -> f32 {
        self.spacing
    }
}

/// The box a parent allows a child.
///
/// `min_width`/`max_width` and `min_height`/`max_height` are inclusive bounds in
/// pixels; `f32::INFINITY` on a maximum means the axis is unbounded. A minimum
/// above its maximum is reachable — the fields are public, and intersecting two
/// boxes can produce one — and is resolved by [`Constraints::constrain`], not
/// by panicking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Smallest allowed width.
    pub min_width: f32,
    /// Largest allowed width, or `f32::INFINITY`.
    pub max_width: f32,
    /// Smallest allowed height.
    pub min_height: f32,
    /// Largest allowed height, or `f32::INFINITY`.
    pub max_height: f32,
}

impl Constraints {
    /// The least restrictive box: no minimum, no maximum.
    pub const UNBOUNDED: Constraints = Constraints {
        min_width: 0.0,
        max_width: f32::INFINITY,
        min_height: 0.0,
        max_height: f32::INFINITY,
    };

    /// Creates a constraint set from its four bounds.
    #[must_use]
    pub fn new(min_width: f32, max_width: f32, min_height: f32, max_height: f32) -> Self {
        Constraints {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// A box of exactly `size`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::layout::{Constraints, Size};
    ///
    /// let box_ = Constraints::tight(Size::new(320.0, 200.0));
    /// assert!(box_.is_tight());
    /// assert_eq!(box_.constrain(Size::new(10.0, 10.0)), Size::new(320.0, 200.0));
    /// ```
    #[must_use]
    pub fn tight(size: Size) -> Self {
        Constraints {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
        }
    }

    /// A box of at most `size`, with no minimum.
    #[must_use]
    pub fn loose(size: Size) -> Self {
        Constraints {
            min_width: 0.0,
            max_width: size.width,
            min_height: 0.0,
            max_height: size.height,
        }
    }

    /// Returns `true` if both axes admit exactly one size.
    #[must_use]
    pub fn is_tight(&self) -> bool {
        self.min_width == self.max_width && self.min_height == self.max_height
    }

    /// Returns the largest size this box allows.
    ///
    /// An unbounded axis reports `f32::INFINITY`.
    #[must_use]
    pub fn biggest(&self) -> Size {
        Size::new(self.max_width, self.max_height)
    }

    /// Clamps `size` into this box.
    ///
    /// When a minimum exceeds its maximum, the minimum wins: it is the bound a
    /// caller can still grow to, and honouring a lower maximum would silently
    /// push a child below the size it declared.
    #[must_use]
    pub fn constrain(&self, size: Size) -> Size {
        Size::new(
            clamp_axis(size.width, self.min_width, self.max_width),
            clamp_axis(size.height, self.min_height, self.max_height),
        )
    }

    /// Returns the intersection of this box and `other`.
    ///
    /// The result can have a minimum above its maximum; [`Constraints::constrain`]
    /// resolves that when a size is clamped into it.
    #[must_use]
    pub fn tighten(self, other: Constraints) -> Self {
        Constraints {
            min_width: self.min_width.max(other.min_width),
            max_width: self.max_width.min(other.max_width),
            min_height: self.min_height.max(other.min_height),
            max_height: self.max_height.min(other.max_height),
        }
    }

    /// Returns this box with both minima dropped to zero.
    #[must_use]
    pub fn loosen(self) -> Self {
        Constraints {
            min_width: 0.0,
            max_width: self.max_width,
            min_height: 0.0,
            max_height: self.max_height,
        }
    }

    /// Returns the (minimum, maximum) extent of `direction`'s main axis.
    fn main(self, direction: FlexDirection) -> (f32, f32) {
        match direction {
            FlexDirection::Row => (self.min_width, self.max_width),
            FlexDirection::Column => (self.min_height, self.max_height),
        }
    }

    /// Returns the (minimum, maximum) extent of `direction`'s cross axis.
    fn cross(self, direction: FlexDirection) -> (f32, f32) {
        match direction {
            FlexDirection::Row => (self.min_height, self.max_height),
            FlexDirection::Column => (self.min_width, self.max_width),
        }
    }

    /// Replaces the main-axis bounds, leaving the cross axis alone.
    fn with_main(self, direction: FlexDirection, min: f32, max: f32) -> Self {
        match direction {
            FlexDirection::Row => Constraints {
                min_width: min,
                max_width: max,
                ..self
            },
            FlexDirection::Column => Constraints {
                min_height: min,
                max_height: max,
                ..self
            },
        }
    }

    /// Replaces the cross-axis bounds, leaving the main axis alone.
    fn with_cross(self, direction: FlexDirection, min: f32, max: f32) -> Self {
        match direction {
            FlexDirection::Row => Constraints {
                min_height: min,
                max_height: max,
                ..self
            },
            FlexDirection::Column => Constraints {
                min_width: min,
                max_width: max,
                ..self
            },
        }
    }
}

/// A node with no declared constraints accepts any size the parent offers.
impl Default for Constraints {
    fn default() -> Self {
        Constraints::UNBOUNDED
    }
}

/// How a node places its children.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutMode {
    /// No automatic layout: each child keeps the size it asks for and sits at
    /// the position it declares. A child that declares no position sits at the
    /// parent's origin.
    Absolute,
    /// Children run along `direction` on one line, sharing the space that is
    /// left after the rigid ones are sized.
    ///
    /// `wrap` is accepted for the mode the architecture describes but is not
    /// honoured yet: children stay on a single line and an overflowing child is
    /// clipped rather than wrapped. Wrapping arrives with the list and scroll
    /// widgets.
    Flex {
        /// The axis the children run along.
        direction: FlexDirection,
        /// Whether children should wrap onto further lines. Not yet honoured.
        wrap: bool,
    },
    /// A grid of `columns` columns. The mode is part of the layout vocabulary
    /// but its algorithm is out of scope for this task; a grid node lays out no
    /// children and reports no rects.
    Grid {
        /// Number of columns the grid would have.
        columns: usize,
    },
    /// Children overlap, all placed at the parent's origin.
    #[default]
    Stack,
}

impl LayoutMode {
    /// A flex container laying its children out left to right on one line.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::layout::LayoutMode;
    ///
    /// assert_eq!(LayoutMode::column(), LayoutMode::Flex {
    ///     direction: ui_core::layout::FlexDirection::Column,
    ///     wrap: false,
    /// });
    /// ```
    #[must_use]
    pub fn row() -> Self {
        LayoutMode::Flex {
            direction: FlexDirection::Row,
            wrap: false,
        }
    }

    /// A flex container laying its children out top to bottom on one line.
    #[must_use]
    pub fn column() -> Self {
        LayoutMode::Flex {
            direction: FlexDirection::Column,
            wrap: false,
        }
    }
}

/// One node's layout inputs, and the rect the last pass computed for it.
///
/// The inputs are the node's declared [`Constraints`], its flex factor, its
/// position for an [`LayoutMode::Absolute`] parent, and its own [`LayoutMode`]
/// with the [`FlexConfig`] to arrange its children by. The cache is the rect
/// the pass computed, the clip rectangle it sits inside, and whether it is
/// still up to date.
///
/// A fresh state is dirty and has no rect: the pass lays out every node it
/// reaches for the first time. Changing an input through a `set_` method marks
/// the node itself dirty; the change only reaches the ancestors when
/// [`mark_dirty`] walks the tree, because the node cannot see the arena that
/// holds its parent.
pub struct LayoutState {
    mode: LayoutMode,
    flex_config: FlexConfig,
    constraints: Constraints,
    flex: f32,
    position: Option<Offset>,
    rect: Option<Rect>,
    clip: Option<Rect>,
    dirty: bool,
    placed_under: Option<Constraints>,
}

impl Default for LayoutState {
    fn default() -> Self {
        LayoutState {
            mode: LayoutMode::default(),
            flex_config: FlexConfig::default(),
            constraints: Constraints::default(),
            flex: 0.0,
            position: None,
            rect: None,
            clip: None,
            dirty: true,
            placed_under: None,
        }
    }
}

impl LayoutState {
    /// Creates a dirty state with no cached rect and no sizing preferences.
    #[must_use]
    pub fn new() -> Self {
        LayoutState::default()
    }

    /// Sets how this node places its children.
    #[must_use]
    pub fn with_mode(mut self, mode: LayoutMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets how a flex mode arranges this node's children.
    #[must_use]
    pub fn with_flex_config(mut self, flex_config: FlexConfig) -> Self {
        self.flex_config = flex_config;
        self
    }

    /// Sets the constraints this node declares.
    ///
    /// The pass intersects them with the ones the parent imposes, so they are
    /// both the size the node asks for and the bound it will not be squeezed
    /// below.
    #[must_use]
    pub fn with_constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Sets this node's share of the free space on its parent's main axis.
    ///
    /// A negative factor is clamped to zero. A factor of zero — the default —
    /// means the node is rigid: the parent gives it the size it asks for.
    #[must_use]
    pub fn with_flex(mut self, flex: f32) -> Self {
        self.flex = flex.max(0.0);
        self
    }

    /// Sets where an `Absolute` parent places this node, relative to the
    /// parent's origin. Every other mode ignores it.
    #[must_use]
    pub fn with_position(mut self, position: Offset) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets how this node places its children and marks it dirty.
    pub fn set_mode(&mut self, mode: LayoutMode) {
        self.mode = mode;
        self.mark_dirty();
    }

    /// Sets the flex arrangement and marks this node dirty.
    pub fn set_flex_config(&mut self, flex_config: FlexConfig) {
        self.flex_config = flex_config;
        self.mark_dirty();
    }

    /// Sets the declared constraints and marks this node dirty.
    pub fn set_constraints(&mut self, constraints: Constraints) {
        self.constraints = constraints;
        self.mark_dirty();
    }

    /// Sets the flex factor, clamped to zero, and marks this node dirty.
    pub fn set_flex(&mut self, flex: f32) {
        self.flex = flex.max(0.0);
        self.mark_dirty();
    }

    /// Sets the position an `Absolute` parent uses and marks this node dirty.
    pub fn set_position(&mut self, position: Option<Offset>) {
        self.position = position;
        self.mark_dirty();
    }

    /// Returns the mode this node places its children with.
    #[must_use]
    pub fn mode(&self) -> LayoutMode {
        self.mode
    }

    /// Returns the flex arrangement this node's children get.
    #[must_use]
    pub fn flex_config(&self) -> &FlexConfig {
        &self.flex_config
    }

    /// Returns the constraints this node declares.
    #[must_use]
    pub fn constraints(&self) -> Constraints {
        self.constraints
    }

    /// Returns this node's flex factor.
    #[must_use]
    pub fn flex(&self) -> f32 {
        self.flex
    }

    /// Returns the position an `Absolute` parent places this node at, if it
    /// declares one.
    #[must_use]
    pub fn position(&self) -> Option<Offset> {
        self.position
    }

    /// Returns the rect the last pass computed, or `None` before the first one.
    #[must_use]
    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    /// Returns the rectangle this node's paint must be clipped to, or `None`
    /// if no ancestor clips it.
    #[must_use]
    pub fn clip(&self) -> Option<Rect> {
        self.clip
    }

    /// Returns `true` if this node has to be laid out again.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns the constraints the cached rect was computed under, or `None`
    /// before the first pass.
    #[must_use]
    pub fn placed_under(&self) -> Option<Constraints> {
        self.placed_under
    }

    /// Marks this node dirty without touching its ancestors.
    ///
    /// Use [`mark_dirty`] after changing an input: a node that moves has to
    /// take its ancestors with it, and only the arena can walk them.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Records the result of a pass over this node and clears the dirty flag.
    fn place(&mut self, rect: Rect, clip: Option<Rect>, placed_under: Constraints) {
        self.rect = Some(rect);
        self.clip = clip;
        self.placed_under = Some(placed_under);
        self.dirty = false;
    }
}

/// Returns whether anything under `handle` needs a pass.
///
/// A node that has never been placed is dirty itself, so a clean node may still
/// sit above one that was only reached through [`mark_dirty`] and not from the
/// root. Skipping such a node would leave the dirty descendant with the rect it
/// had in its old box, so the walk is what keeps a deep change visible without
/// marking the whole ancestor chain.
///
/// The walk stops at the first dirty node it finds, and only recurses into
/// children that are themselves clean, so a fully clean subtree costs one
/// recursion level per clean node and never reaches a dirty one twice.
fn subtree_is_dirty(nodes: &Arena<WidgetNode>, handle: Handle) -> bool {
    let Some(node) = nodes.get(handle) else {
        // A stale handle is not laid out, and not laid out means not clean.
        return true;
    };
    if node.layout().is_dirty() {
        return true;
    }
    node.children()
        .iter()
        .any(|&child| subtree_is_dirty(nodes, child))
}

/// The layout pass over one arena of nodes.
///
/// The pass borrows the arena because a node cannot reach the arena that holds
/// it, and the pass both reads a node's inputs and writes its cache. It holds no
/// state of its own and is cheap to create.
pub struct Layout<'a> {
    nodes: &'a mut Arena<WidgetNode>,
    /// How many nodes the current pass laid out, for the tests that pin the
    /// skip. It counts work, not calls: a skipped node returns before this is
    /// touched, which is the whole point of counting here.
    #[cfg(test)]
    laid_out: usize,
}

impl<'a> Layout<'a> {
    /// Creates a pass over `nodes`.
    #[must_use]
    pub fn new(nodes: &'a mut Arena<WidgetNode>) -> Self {
        Layout {
            nodes,
            #[cfg(test)]
            laid_out: 0,
        }
    }

    /// Returns how many nodes the last pass laid out.
    ///
    /// A pass over an unchanged tree lays out none of them, and a pass over a
    /// tree with one changed leaf lays out the path down to it and nothing
    /// else. Tests assert on this, because "the rects came out the same" cannot
    /// tell a skipped node from a recomputed one.
    ///
    /// The count is per pass: [`Layout::layout`] clears it, so it never has to
    /// add up across a reused `Layout`. A cumulative count would also be
    /// correct here, but it would mean something different to a reader than the
    /// name says.
    #[cfg(test)]
    fn laid_out(&self) -> usize {
        self.laid_out
    }

    /// Lays `root` out inside `constraints`, and every dirty node below it.
    ///
    /// The root is placed at the origin of the coordinate space `constraints`
    /// describes. A clean node whose incoming constraints are the ones its
    /// cached rect was computed under is left alone, and so is its whole
    /// subtree.
    pub fn layout(&mut self, root: Handle, constraints: Constraints) {
        #[cfg(test)]
        {
            self.laid_out = 0;
        }
        self.visit(root, constraints, Offset::ZERO, None);
    }

    /// Lays one node out and recurses into its children.
    fn visit(&mut self, handle: Handle, incoming: Constraints, origin: Offset, clip: Option<Rect>) {
        let Some(node) = self.nodes.get(handle) else {
            return;
        };
        // A node that is clean, was placed under the same box, and is clipped by
        // the same rectangle keeps its cache, and the pass stops here instead of
        // walking the subtree. A node that any of that has moved out from under
        // is marked dirty by whoever moved it.
        //
        // The subtree is the fourth thing that has to hold. A descendant that
        // was marked on its own — not climbed to from here — would otherwise be
        // left with the rect of the box it used to sit in, and the whole chain
        // above it would read as up to date. So the walk looks down for one
        // dirty node before deciding the cache is good enough to keep.
        if !node.layout().is_dirty()
            && node.layout().placed_under() == Some(incoming)
            && node.layout().clip() == clip
            && !subtree_is_dirty(self.nodes, handle)
        {
            return;
        }
        #[cfg(test)]
        {
            self.laid_out += 1;
        }
        // The children are copied out because the node cannot stay borrowed
        // while its children are laid out through the arena.
        let children = node.children().to_vec();
        let mode = node.layout().mode();
        let config = *node.layout().flex_config();
        // The box this node fills, and the one its children are placed in.
        let box_constraints = resolve_box(self.nodes, node, incoming);
        let rect = Rect::new(origin, box_constraints.biggest());
        let placements = arrange(self.nodes, &children, box_constraints, mode, &config);
        // Children are clipped to their ancestors' boxes; the intersection is
        // the same for all of them, so it is computed once.
        let children_clip = intersect(clip, Some(rect));

        if let Some(node) = self.nodes.get_mut(handle) {
            node.layout_mut().place(rect, clip, incoming);
        }

        for placement in placements {
            // `arrange` answers in the box's own coordinates, and the cache
            // holds absolute ones, so the node's origin is added here rather
            // than inside `arrange` — which is what lets `layout_constraints`
            // answer for a box it was not given an origin for.
            let at = Rect::new(
                Offset::new(
                    origin.x + placement.rect.origin.x,
                    origin.y + placement.rect.origin.y,
                ),
                placement.rect.size,
            );
            // Rects are absolute, so a child placed anywhere new takes its whole
            // subtree with it — and a child whose *sibling* grew is moved just
            // as surely as one whose parent did, which is why this compares
            // placements rather than watching a single node's origin.
            if self.moved(placement.handle, at) {
                if let Some(child) = self.nodes.get_mut(placement.handle) {
                    child.layout_mut().mark_dirty();
                }
            }
            self.visit(
                placement.handle,
                placement.constraints,
                at.origin,
                children_clip,
            );
        }
    }

    /// Returns `true` if `handle` is not already placed at `rect`.
    fn moved(&self, handle: Handle, rect: Rect) -> bool {
        self.nodes
            .get(handle)
            .map(WidgetNode::layout)
            .and_then(LayoutState::rect)
            .is_none_or(|old| old != rect)
    }
}

/// Marks `handle` and every ancestor dirty.
///
/// A change to a node moves everything above it, so the walk goes up the parent
/// links. The pass marks a node's *descendants* in turn, when their rects move
/// with it. A stale handle, or one whose ancestors are stale, stops the walk
/// there.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::layout::{mark_dirty, Constraints, Layout, LayoutState, Size};
/// use ui_core::node::{self, WidgetNode};
///
/// let mut nodes = Arena::new();
/// let child = node::create(&mut nodes, LayoutState::new());
/// let parent = node::create(&mut nodes, LayoutState::new());
/// assert!(node::attach(&mut nodes, parent, child));
/// Layout::new(&mut nodes).layout(parent, Constraints::tight(Size::new(100.0, 100.0)));
///
/// // A change to the child is not visible to the parent until it is marked.
/// nodes.get_mut(child).unwrap().layout_mut().set_flex(1.0);
/// assert!(!nodes.get(parent).unwrap().layout().is_dirty());
/// mark_dirty(&mut nodes, child);
/// assert!(nodes.get(parent).unwrap().layout().is_dirty());
/// ```
pub fn mark_dirty(nodes: &mut Arena<WidgetNode>, handle: Handle) {
    let mut current = Some(handle);
    while let Some(handle) = current {
        let Some(node) = nodes.get_mut(handle) else {
            return;
        };
        node.layout_mut().mark_dirty();
        current = node.parent();
    }
}

/// Lays `children` out inside `constraints` and returns one rect per child, in
/// the order they were given.
///
/// `mode` is the parent's own [`LayoutState::mode`] and `config` its
/// [`LayoutState::flex_config`]; both are arguments because a handle addresses
/// the arena rather than a node, so this call cannot look them up itself. The
/// rects are local to the constrained box: the parent adds its own origin.
///
/// A handle that no longer resolves gets a zero-size rect at the origin, so the
/// result stays index-aligned with `children`.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::layout::{layout_constraints, Constraints, FlexConfig, LayoutMode, LayoutState, Size};
/// use ui_core::node::{self, WidgetNode};
///
/// let mut nodes = Arena::new();
/// let child = node::create(
///     &mut nodes,
///     LayoutState::new().with_constraints(Constraints::tight(Size::new(30.0, 10.0))),
/// );
/// let rects = layout_constraints(
///     &nodes,
///     &[child],
///     Constraints::tight(Size::new(100.0, 50.0)),
///     LayoutMode::row(),
///     &FlexConfig::new(),
/// );
/// assert_eq!(rects[0].size, Size::new(30.0, 10.0));
/// ```
pub fn layout_constraints(
    nodes: &Arena<WidgetNode>,
    children: &[Handle],
    constraints: Constraints,
    mode: LayoutMode,
    config: &FlexConfig,
) -> Vec<Rect> {
    arrange(nodes, children, constraints, mode, config)
        .into_iter()
        .map(|placement| placement.rect)
        .collect()
}

/// One child's resolved placement: the rect it occupies inside the parent's
/// box, and the constraints it was placed under.
///
/// The constraints travel with the placement because the pass hands them
/// straight down to the child, so the child reaches the same size its parent
/// measured for it instead of measuring a second time against a different box.
struct Placement {
    handle: Handle,
    rect: Rect,
    constraints: Constraints,
}

/// Lays `children` out inside `constraints` and returns a placement per child.
fn arrange(
    nodes: &Arena<WidgetNode>,
    children: &[Handle],
    constraints: Constraints,
    mode: LayoutMode,
    config: &FlexConfig,
) -> Vec<Placement> {
    match mode {
        LayoutMode::Flex { direction, .. } => {
            arrange_flex(nodes, children, constraints, direction, config)
        }
        LayoutMode::Stack => arrange_stack(nodes, children, constraints),
        LayoutMode::Absolute => arrange_absolute(nodes, children, constraints),
        LayoutMode::Grid { .. } => Vec::new(),
    }
}

/// A flex child, in main/cross terms.
struct FlexItem {
    handle: Handle,
    flex: f32,
    declared: Constraints,
    desired: Size,
    main: f32,
    cross: f32,
    /// Whether the cross extent is the whole of a bounded box, so the child has
    /// to be handed a tight cross constraint rather than the box to measure in.
    tight_cross: bool,
}

/// Places `children` along `direction`, sharing the leftover space between the
/// flexible ones.
fn arrange_flex(
    nodes: &Arena<WidgetNode>,
    children: &[Handle],
    constraints: Constraints,
    direction: FlexDirection,
    config: &FlexConfig,
) -> Vec<Placement> {
    if children.is_empty() {
        return Vec::new();
    }
    let spacing = config.spacing();
    let (_, max_main) = constraints.main(direction);
    let (_, max_cross) = constraints.cross(direction);

    // 1. Every child's own inputs, and the size it asks for inside the box.
    let mut items = Vec::with_capacity(children.len());
    for &handle in children {
        // A handle that no longer resolves is sized as nothing and takes part in
        // no sharing, so the result stays index-aligned with `children`.
        let Some(node) = nodes.get(handle) else {
            items.push(FlexItem {
                handle,
                flex: 0.0,
                declared: Constraints::UNBOUNDED,
                desired: Size::ZERO,
                main: 0.0,
                cross: 0.0,
                tight_cross: false,
            });
            continue;
        };
        let declared = node.layout().constraints();
        let desired = sized(nodes, handle, constraints.loosen());
        items.push(FlexItem {
            handle,
            flex: node.layout().flex(),
            declared,
            desired,
            main: 0.0,
            cross: 0.0,
            tight_cross: false,
        });
    }

    // 2. The main axis. A rigid child keeps the size it asked for; a flexible
    // one starts at nothing and takes a share of what is left.
    let mut rigid_total = 0.0;
    for item in &mut items {
        if item.flex > 0.0 {
            item.main = 0.0;
        } else {
            item.main = item.desired.main(direction);
            rigid_total += item.main;
        }
    }
    let gaps = spacing * count_to_f32(children.len().saturating_sub(1));
    // An unbounded main axis has no free space to distribute: the content is as
    // long as it needs to be, and a flexible child in it has no intrinsic size.
    let available_main = if max_main.is_finite() {
        max_main
    } else {
        rigid_total + gaps
    };
    distribute(
        &mut items,
        (available_main - rigid_total - gaps).max(0.0),
        direction,
    );

    // 3. The cross axis. A child that fills — one the alignment stretches, or
    // a flexible child, which has no intrinsic extent to pack against — takes
    // the whole of a bounded box, clamped by what it declared; every other child
    // keeps the size it asked for.
    for item in &mut items {
        let (min_declared, max_declared) = item.declared.cross(direction);
        item.tight_cross = max_cross.is_finite()
            && (item.flex > 0.0 || config.cross_axis_alignment() == CrossAxisAlignment::Stretch);
        item.cross = if item.tight_cross {
            clamp_axis(max_cross, min_declared, max_declared)
        } else {
            item.desired.cross(direction)
        };
    }

    // 4. Positions: the alignment turns the leftover into a leading offset and
    // a gap between children.
    let used: f32 = items.iter().map(|item| item.main).sum::<f32>() + gaps;
    let extra = available_main - used;
    let (start, gap) =
        alignment_spacing(config.main_axis_alignment(), extra, children.len(), spacing);
    let mut cursor = start;
    let mut placements = Vec::with_capacity(children.len());
    for item in &items {
        let cross_offset = cross_offset(config.cross_axis_alignment(), max_cross, item.cross);
        let (x, y) = direction_cross_to_xy(direction, cursor, cross_offset);
        let size = Size::from_main_cross(direction, item.main, item.cross);
        // A flexible child is given exactly its share; a rigid one is free to
        // take any size up to the available main extent, which is the box the
        // size it asked for was clamped into.
        let main_bounds = if item.flex > 0.0 {
            (item.main, item.main)
        } else {
            (0.0, available_main)
        };
        // A child that fills the cross axis is given that extent rather than a
        // box to measure in, or it would measure its way back down to the size
        // it already has.
        let cross_bounds = if item.tight_cross {
            (item.cross, item.cross)
        } else {
            (0.0, max_cross)
        };
        placements.push(Placement {
            handle: item.handle,
            rect: Rect::new(Offset::new(x, y), size),
            constraints: constraints
                .with_main(direction, main_bounds.0, main_bounds.1)
                .with_cross(direction, cross_bounds.0, cross_bounds.1),
        });
        cursor += item.main + gap;
    }
    placements
}

/// Places every child at the parent's origin, at the size it asks for.
fn arrange_stack(
    nodes: &Arena<WidgetNode>,
    children: &[Handle],
    constraints: Constraints,
) -> Vec<Placement> {
    let loose = constraints.loosen();
    children
        .iter()
        .map(|&handle| Placement {
            handle,
            rect: Rect::new(Offset::ZERO, sized(nodes, handle, loose)),
            constraints: loose,
        })
        .collect()
}

/// Places every child at the position it declares, at the size it asks for.
///
/// A child that declares no position sits at the parent's origin.
fn arrange_absolute(
    nodes: &Arena<WidgetNode>,
    children: &[Handle],
    constraints: Constraints,
) -> Vec<Placement> {
    let loose = constraints.loosen();
    children
        .iter()
        .map(|&handle| {
            let origin = nodes
                .get(handle)
                .and_then(|node| node.layout().position())
                .unwrap_or(Offset::ZERO);
            Placement {
                handle,
                rect: Rect::new(origin, sized(nodes, handle, loose)),
                constraints: loose,
            }
        })
        .collect()
}

/// Shares `budget` pixels of the main axis between the flexible items, in
/// proportion to their flex factors and within each item's own bounds.
///
/// An item that hits one of its bounds is fixed there and its share of what is
/// left goes back into the pool for the others, so a child that reached its
/// maximum does not leave the row short, and one whose minimum took more than
/// its share does not leave the row long. Every round either fixes at least one
/// more item or spends the budget, so the loop runs at most once per item.
///
/// A budget no item's bounds fit into is left unallocated rather than forced
/// on somebody: the sum of the minima is then the row's real main extent, and
/// the row is longer than the box it was given, which is the overflow clip is
/// for.
fn distribute(items: &mut [FlexItem], budget: f32, direction: FlexDirection) {
    let mut fixed = vec![false; items.len()];
    let mut pinned = 0.0f32;
    loop {
        let total: f32 = items
            .iter()
            .enumerate()
            .filter(|(index, item)| !fixed[*index] && item.flex > 0.0)
            .map(|(_, item)| item.flex)
            .sum();
        if total <= 0.0 {
            return;
        }
        let free = (budget - pinned).max(0.0);
        let mut newly_fixed = Vec::new();
        for (index, item) in items.iter_mut().enumerate() {
            if fixed[index] || item.flex <= 0.0 {
                continue;
            }
            // The fraction is taken first. `free * item.flex / total` left-associates,
            // so two factors near the `f32` ceiling overflow to `inf` before the
            // division brings them back down, and the child was given an infinite
            // width. The ratio is at most 1.0 by construction — `item.flex <= total`
            // — so multiplying by it cannot overflow.
            let share = free * (item.flex / total);
            let (min_main, max_main) = item.declared.main(direction);
            let taken = clamp_axis(share, min_main, max_main);
            item.main = taken;
            if taken != share {
                newly_fixed.push(index);
            }
        }
        if newly_fixed.is_empty() {
            return;
        }
        for index in newly_fixed {
            fixed[index] = true;
            pinned += items[index].main;
        }
    }
}

/// Returns the leading offset and the per-child gap that `alignment` turns
/// `extra` pixels of leftover main-axis space into.
fn alignment_spacing(
    alignment: MainAxisAlignment,
    extra: f32,
    count: usize,
    spacing: f32,
) -> (f32, f32) {
    let count = count_to_f32(count);
    match alignment {
        MainAxisAlignment::Start => (0.0, spacing),
        MainAxisAlignment::End => (extra, spacing),
        MainAxisAlignment::Center => (extra / 2.0, spacing),
        MainAxisAlignment::SpaceBetween if count > 1.0 => (0.0, spacing + extra / (count - 1.0)),
        MainAxisAlignment::SpaceBetween => (0.0, spacing),
        MainAxisAlignment::SpaceAround if count > 0.0 => {
            (extra / count / 2.0, spacing + extra / count)
        }
        MainAxisAlignment::SpaceAround => (0.0, spacing),
        MainAxisAlignment::SpaceEvenly if count > 0.0 => {
            (extra / (count + 1.0), spacing + extra / (count + 1.0))
        }
        MainAxisAlignment::SpaceEvenly => (0.0, spacing),
    }
}

/// Returns the cross-axis offset of a child `cross` pixels wide in a box
/// `box_cross` wide.
fn cross_offset(alignment: CrossAxisAlignment, box_cross: f32, cross: f32) -> f32 {
    if !box_cross.is_finite() {
        // An unbounded cross axis has nothing to align against.
        return 0.0;
    }
    match alignment {
        CrossAxisAlignment::Start | CrossAxisAlignment::Stretch => 0.0,
        CrossAxisAlignment::End => box_cross - cross,
        CrossAxisAlignment::Center => (box_cross - cross) / 2.0,
    }
}

/// Maps a main-axis extent and a cross-axis extent to `(x, y)`.
fn direction_cross_to_xy(direction: FlexDirection, main: f32, cross: f32) -> (f32, f32) {
    match direction {
        FlexDirection::Row => (main, cross),
        FlexDirection::Column => (cross, main),
    }
}

/// The bounding box of a set of placements, measured from the box's origin.
///
/// Children can sit at a negative offset — a centred row that overflows, or an
/// absolute child placed off the top-left — so the extent is the farthest
/// corner reached, floored at zero.
fn bounding_box(placements: &[Placement]) -> Size {
    let mut right = 0.0f32;
    let mut bottom = 0.0f32;
    for placement in placements {
        let corner = placement.rect.far_corner();
        right = right.max(corner.x);
        bottom = bottom.max(corner.y);
    }
    Size::new(right.max(0.0), bottom.max(0.0))
}

/// The size a node's children need, measured with no box at all.
///
/// The content is measured unbounded so that a node's size stays a property of
/// its children rather than of how it arranges them: a centred child in a loose
/// box must not make its parent wider. An unbounded main axis also means a
/// flexible child has no intrinsic size there, which is the answer the rest of
/// the pass gives an unbounded main axis.
fn content_size(nodes: &Arena<WidgetNode>, node: &WidgetNode) -> Size {
    bounding_box(&arrange(
        nodes,
        node.children(),
        Constraints::UNBOUNDED,
        node.layout().mode(),
        node.layout().flex_config(),
    ))
}

/// The box a node lays its children out in: the tightest box its own declared
/// constraints and the constraints it was given agree on, and — when that box
/// still leaves a choice — the content its children need, clamped into it.
///
/// The measure step and the place step are both this call, which is what keeps
/// a parent and a child from disagreeing about a size.
fn resolve_box(nodes: &Arena<WidgetNode>, node: &WidgetNode, incoming: Constraints) -> Constraints {
    let effective = incoming.tighten(node.layout().constraints());
    if effective.is_tight() {
        return effective;
    }
    Constraints::tight(effective.constrain(content_size(nodes, node)))
}

/// The size `handle` takes within `incoming`, or zero for a stale handle.
fn sized(nodes: &Arena<WidgetNode>, handle: Handle, incoming: Constraints) -> Size {
    match nodes.get(handle) {
        Some(node) => resolve_box(nodes, node, incoming).biggest(),
        None => Size::ZERO,
    }
}

/// Intersects two optional clip rectangles.
///
/// An unset clip means no ancestor clips the node, so the other one — which may
/// also be unset — passes through.
fn intersect(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Rect::new(
            Offset::new(a.origin.x.max(b.origin.x), a.origin.y.max(b.origin.y)),
            Size::new(
                (a.far_corner().x.min(b.far_corner().x) - a.origin.x.max(b.origin.x)).max(0.0),
                (a.far_corner().y.min(b.far_corner().y) - a.origin.y.max(b.origin.y)).max(0.0),
            ),
        )),
        (Some(rect), None) | (None, Some(rect)) => Some(rect),
        (None, None) => None,
    }
}

/// Clamps `value` into `[min, max]`.
///
/// `f32::clamp` panics when `min > max`, and a constraint set whose minimum
/// exceeds its maximum is reachable. The minimum wins there: it is the bound a
/// caller can still act on, whereas honouring a lower maximum would silently
/// push a child below the size it declared. `max`/`min` also pass the non-NaN
/// operand through, so a NaN in a constraint cannot turn into a panic.
fn clamp_axis(value: f32, min: f32, max: f32) -> f32 {
    if min > max {
        return min;
    }
    value.max(min).min(max)
}

/// Converts a count of children to the float the layout arithmetic uses.
///
/// `f32` has no `From<usize>` in std, so this is the one place a `usize`-to-f32
/// cast happens. It is well defined for every `usize` — the result rounds to
/// the nearest `f32` — and a child count where that rounding mattered is not a
/// tree this arena can hold.
fn count_to_f32(count: usize) -> f32 {
    count as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node;

    /// Creates a fixed-size leaf, the child every layout test is built from.
    fn leaf(nodes: &mut Arena<WidgetNode>, width: f32, height: f32) -> Handle {
        node::create(
            nodes,
            LayoutState::new().with_constraints(Constraints::tight(Size::new(width, height))),
        )
    }

    /// Creates a container in `mode` with `config` and attaches `children`.
    fn container(
        nodes: &mut Arena<WidgetNode>,
        mode: LayoutMode,
        config: FlexConfig,
        children: &[Handle],
    ) -> Handle {
        let handle = node::create(
            nodes,
            LayoutState::new().with_mode(mode).with_flex_config(config),
        );
        for &child in children {
            assert!(node::attach(nodes, handle, child));
        }
        handle
    }

    /// Lays `root` out in a tight box and returns its rect.
    fn layout_in(nodes: &mut Arena<WidgetNode>, root: Handle, size: Size) -> Rect {
        Layout::new(nodes).layout(root, Constraints::tight(size));
        nodes.get(root).unwrap().layout().rect().unwrap()
    }

    /// Lays `root` out and returns how many nodes the pass laid out.
    ///
    /// The pass borrows the arena, so the count has to be taken before the
    /// caller can read the nodes it wrote.
    fn laid_out_by(nodes: &mut Arena<WidgetNode>, root: Handle, constraints: Constraints) -> usize {
        let mut pass = Layout::new(nodes);
        pass.layout(root, constraints);
        pass.laid_out()
    }

    #[test]
    fn constraints_clamp_into_the_box() {
        let box_ = Constraints::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(box_.constrain(Size::new(0.0, 0.0)), Size::new(10.0, 30.0));
        assert_eq!(box_.constrain(Size::new(15.0, 35.0)), Size::new(15.0, 35.0));
        assert_eq!(
            box_.constrain(Size::new(100.0, 100.0)),
            Size::new(20.0, 40.0)
        );
        assert!(!box_.is_tight());
        assert!(Constraints::tight(Size::new(5.0, 5.0)).is_tight());
        assert_eq!(
            Constraints::default(),
            Constraints::UNBOUNDED,
            "an unconstrained node must accept any size"
        );
    }

    #[test]
    fn a_minimum_above_its_maximum_wins() {
        // Reachable through the public fields and through `tighten`; the
        // minimum is the bound the caller can still act on.
        let broken = Constraints::new(50.0, 20.0, 0.0, 100.0);
        assert_eq!(broken.constrain(Size::new(35.0, 0.0)), Size::new(50.0, 0.0));
        let tightened = Constraints::tight(Size::new(80.0, 80.0)).tighten(Constraints::new(
            100.0,
            f32::INFINITY,
            0.0,
            f32::INFINITY,
        ));
        assert_eq!(tightened.min_width, 100.0);
        assert_eq!(tightened.max_width, 80.0);
    }

    #[test]
    fn layout_rect_converts_to_a_paint_rect() {
        let rect = Rect::from_parts(1.0, 2.0, 3.0, 4.0);
        let converted = paint::Rect::from(rect);
        assert_eq!(
            (converted.x, converted.y, converted.width, converted.height),
            (1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(rect.far_corner(), Offset::new(4.0, 6.0));
    }

    #[test]
    fn row_places_fixed_size_children_with_spacing() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, 100.0, 40.0);
        let b = leaf(&mut nodes, 50.0, 40.0);
        let c = leaf(&mut nodes, 30.0, 40.0);
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new().with_spacing(10.0),
            &[a, b, c],
        );

        layout_in(&mut nodes, row, Size::new(400.0, 40.0));

        let rects: Vec<Rect> = [a, b, c]
            .iter()
            .map(|&handle| nodes.get(handle).unwrap().layout().rect().unwrap())
            .collect();
        assert_eq!(rects[0], Rect::from_parts(0.0, 0.0, 100.0, 40.0));
        assert_eq!(rects[1], Rect::from_parts(110.0, 0.0, 50.0, 40.0));
        assert_eq!(rects[2], Rect::from_parts(170.0, 0.0, 30.0, 40.0));
    }

    #[test]
    fn column_places_fixed_size_children() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, 80.0, 20.0);
        let b = leaf(&mut nodes, 80.0, 30.0);
        let column = container(
            &mut nodes,
            LayoutMode::column(),
            FlexConfig::new().with_spacing(4.0),
            &[a, b],
        );

        layout_in(&mut nodes, column, Size::new(80.0, 100.0));

        assert_eq!(
            nodes.get(a).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 80.0, 20.0)
        );
        assert_eq!(
            nodes.get(b).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 24.0, 80.0, 30.0)
        );
    }

    #[test]
    fn a_column_shares_its_height_between_flex_children() {
        let mut nodes = Arena::new();
        let first = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let second = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let third = node::create(&mut nodes, LayoutState::new().with_flex(2.0));
        let column = container(
            &mut nodes,
            LayoutMode::column(),
            FlexConfig::new(),
            &[first, second, third],
        );

        layout_in(&mut nodes, column, Size::new(100.0, 80.0));

        // The main axis is the height, so the shares stack rather than run, and
        // the cross axis is the full width.
        assert_eq!(
            nodes.get(first).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 100.0, 20.0)
        );
        assert_eq!(
            nodes.get(second).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 20.0, 100.0, 20.0)
        );
        assert_eq!(
            nodes.get(third).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 40.0, 100.0, 40.0)
        );
    }

    #[test]
    fn flex_factors_distribute_the_free_space() {
        let mut nodes = Arena::new();
        let one = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let three = node::create(&mut nodes, LayoutState::new().with_flex(3.0));
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new(),
            &[one, three],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(one).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 25.0, 40.0)
        );
        assert_eq!(
            nodes.get(three).unwrap().layout().rect().unwrap(),
            Rect::from_parts(25.0, 0.0, 75.0, 40.0)
        );
    }

    #[test]
    fn extreme_flex_factors_still_split_the_free_space() {
        let mut nodes = Arena::new();
        // Two factors that are each finite, and whose sum is finite, but whose
        // product with the free space is not: this is the only combination that
        // makes the left-associative form overflow.
        let huge = f32::MAX / 2.0;
        let left = node::create(&mut nodes, LayoutState::new().with_flex(huge));
        let right = node::create(&mut nodes, LayoutState::new().with_flex(huge));
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new(),
            &[left, right],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(left).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 50.0, 40.0),
            "an overflowing intermediate must not reach the child"
        );
        assert_eq!(
            nodes.get(right).unwrap().layout().rect().unwrap(),
            Rect::from_parts(50.0, 0.0, 50.0, 40.0)
        );
    }

    #[test]
    fn a_flex_share_is_clamped_by_the_childs_maximum() {
        let mut nodes = Arena::new();
        let capped = node::create(
            &mut nodes,
            LayoutState::new()
                .with_flex(1.0)
                .with_constraints(Constraints::loose(Size::new(30.0, 40.0))),
        );
        let other = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new(),
            &[capped, other],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(capped).unwrap().layout().rect().unwrap().size,
            Size::new(30.0, 40.0)
        );
        // The share the capped child could not take goes back to the other.
        assert_eq!(
            nodes.get(other).unwrap().layout().rect().unwrap().size,
            Size::new(70.0, 40.0)
        );
    }

    #[test]
    fn a_flex_share_is_raised_to_the_childs_minimum() {
        let mut nodes = Arena::new();
        let floored = node::create(
            &mut nodes,
            LayoutState::new()
                .with_flex(1.0)
                .with_constraints(Constraints::new(60.0, f32::INFINITY, 0.0, 40.0)),
        );
        let other = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new(),
            &[floored, other],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(floored).unwrap().layout().rect().unwrap().size,
            Size::new(60.0, 40.0)
        );
        assert_eq!(
            nodes.get(other).unwrap().layout().rect().unwrap().size,
            Size::new(40.0, 40.0)
        );
    }

    #[test]
    fn a_flex_factor_of_zero_is_rigid() {
        let mut nodes = Arena::new();
        let rigid = leaf(&mut nodes, 40.0, 10.0);
        let flexible = node::create(&mut nodes, LayoutState::new().with_flex(2.0));
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new(),
            &[rigid, flexible],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 20.0));

        assert_eq!(
            nodes.get(rigid).unwrap().layout().rect().unwrap().size,
            Size::new(40.0, 10.0)
        );
        assert_eq!(
            nodes.get(flexible).unwrap().layout().rect().unwrap().size,
            Size::new(60.0, 20.0)
        );
    }

    #[test]
    fn stack_places_every_child_at_the_origin() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, 60.0, 40.0);
        let b = leaf(&mut nodes, 30.0, 70.0);
        let stack = container(&mut nodes, LayoutMode::Stack, FlexConfig::new(), &[a, b]);

        layout_in(&mut nodes, stack, Size::new(200.0, 200.0));

        assert_eq!(
            nodes.get(a).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 60.0, 40.0)
        );
        assert_eq!(
            nodes.get(b).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 30.0, 70.0)
        );
    }

    #[test]
    fn absolute_places_a_child_where_it_asked() {
        let mut nodes = Arena::new();
        let placed = node::create(
            &mut nodes,
            LayoutState::new()
                .with_constraints(Constraints::tight(Size::new(20.0, 20.0)))
                .with_position(Offset::new(35.0, 15.0)),
        );
        let absolute = container(
            &mut nodes,
            LayoutMode::Absolute,
            FlexConfig::new(),
            &[placed],
        );

        layout_in(&mut nodes, absolute, Size::new(100.0, 100.0));

        assert_eq!(
            nodes.get(placed).unwrap().layout().rect().unwrap(),
            Rect::from_parts(35.0, 15.0, 20.0, 20.0)
        );
    }

    #[test]
    fn absolute_places_an_unpositioned_child_at_the_origin() {
        let mut nodes = Arena::new();
        let unplaced = leaf(&mut nodes, 20.0, 20.0);
        let absolute = container(
            &mut nodes,
            LayoutMode::Absolute,
            FlexConfig::new(),
            &[unplaced],
        );

        layout_in(&mut nodes, absolute, Size::new(100.0, 100.0));

        assert_eq!(
            nodes.get(unplaced).unwrap().layout().rect().unwrap().origin,
            Offset::ZERO
        );
    }

    #[test]
    fn no_children_produce_no_rects() {
        let nodes = Arena::new();
        let rects = layout_constraints(
            &nodes,
            &[],
            Constraints::tight(Size::new(10.0, 10.0)),
            LayoutMode::row(),
            &FlexConfig::new(),
        );
        assert!(rects.is_empty());
    }

    #[test]
    fn one_child_gets_no_spacing_gap() {
        let mut nodes = Arena::new();
        let only = leaf(&mut nodes, 10.0, 10.0);
        let rects = layout_constraints(
            &nodes,
            &[only],
            Constraints::tight(Size::new(100.0, 100.0)),
            LayoutMode::row(),
            &FlexConfig::new().with_spacing(50.0),
        );
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].origin, Offset::ZERO, "a gap needs two children");
    }

    #[test]
    fn spacing_is_clamped_to_zero() {
        let config = FlexConfig::new().with_spacing(-10.0);
        assert_eq!(config.spacing(), 0.0);
    }

    #[test]
    fn a_box_with_no_space_pushes_no_child_backwards() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, 10.0, 10.0);
        let b = leaf(&mut nodes, 10.0, 10.0);
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[a, b]);

        layout_in(&mut nodes, row, Size::new(0.0, 0.0));

        assert_eq!(
            nodes.get(a).unwrap().layout().rect().unwrap().origin,
            Offset::ZERO
        );
        assert_eq!(
            nodes.get(b).unwrap().layout().rect().unwrap().origin,
            Offset::new(10.0, 0.0),
            "a child keeps its place in the row even with nowhere to put it"
        );
        for handle in [a, b] {
            let state = nodes.get(handle).unwrap().layout();
            assert!(
                state.rect().unwrap().origin.x >= 0.0,
                "{:?} went left",
                state.rect()
            );
            assert_eq!(state.clip(), Some(Rect::ZERO));
        }
    }

    #[test]
    fn alignment_shifts_the_leftover_space() {
        // 100 pixels wide, two 20-pixel children, so 60 pixels are left over.
        // Both offsets are pinned: the second one is what catches a distribution
        // that moves the first child correctly and forgets the gap behind it,
        // which is the part `SpaceBetween` and `SpaceAround` are named for.
        for (alignment, first_expected, second_expected) in [
            (MainAxisAlignment::Start, 0.0, 20.0),
            (MainAxisAlignment::End, 60.0, 80.0),
            (MainAxisAlignment::Center, 30.0, 50.0),
            (MainAxisAlignment::SpaceBetween, 0.0, 80.0),
            (MainAxisAlignment::SpaceAround, 15.0, 65.0),
            (MainAxisAlignment::SpaceEvenly, 20.0, 60.0),
        ] {
            let mut nodes = Arena::new();
            let first = leaf(&mut nodes, 20.0, 10.0);
            let second = leaf(&mut nodes, 20.0, 10.0);
            let row = container(
                &mut nodes,
                LayoutMode::row(),
                FlexConfig::new().with_main_axis_alignment(alignment),
                &[first, second],
            );
            layout_in(&mut nodes, row, Size::new(100.0, 10.0));

            assert_eq!(
                nodes.get(first).unwrap().layout().rect().unwrap().origin.x,
                first_expected,
                "{alignment:?} placed the first child wrongly"
            );
            assert_eq!(
                nodes.get(second).unwrap().layout().rect().unwrap().origin.x,
                second_expected,
                "{alignment:?} placed the second child wrongly"
            );
        }
    }

    #[test]
    fn cross_alignment_offsets_a_child_of_a_different_size() {
        // `Start` and `End` are already covered; the two that are not are
        // `Center`, which has to halve the difference, and `Stretch`, which has
        // to leave the offset at zero and let the box do the work.
        for (alignment, expected) in [
            (CrossAxisAlignment::Center, 10.0),
            (CrossAxisAlignment::End, 20.0),
            (CrossAxisAlignment::Start, 0.0),
        ] {
            let mut nodes = Arena::new();
            let short = leaf(&mut nodes, 20.0, 10.0);
            let tall = leaf(&mut nodes, 20.0, 20.0);
            let row = container(
                &mut nodes,
                LayoutMode::row(),
                FlexConfig::new().with_cross_axis_alignment(alignment),
                &[short, tall],
            );
            layout_in(&mut nodes, row, Size::new(100.0, 30.0));

            // The 10-pixel child is the only one with room to move: the tall one
            // is flush either way, and asserting on it too would hide a `Center`
            // that returns the offset unchanged.
            assert_eq!(
                nodes.get(short).unwrap().layout().rect().unwrap().origin.y,
                expected,
                "{alignment:?} placed the shorter child wrongly"
            );
        }
    }

    #[test]
    fn a_flexible_child_can_itself_have_children() {
        let mut nodes = Arena::new();
        let inner = leaf(&mut nodes, 10.0, 10.0);
        // The flexible node is a container, not a leaf, so its size comes from
        // the share it wins and its child is placed inside that share.
        let flexible = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[inner]);
        nodes.get_mut(flexible).unwrap().layout_mut().set_flex(1.0);
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new(),
            &[flexible],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(flexible).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 100.0, 40.0),
            "the container takes the whole share, and fills the cross axis"
        );
        assert_eq!(
            nodes.get(inner).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 10.0, 10.0),
            "the rigid child keeps its own size inside the box its parent won"
        );
    }

    #[test]
    fn alignment_with_no_free_space_is_the_start_offset() {
        for alignment in [
            MainAxisAlignment::Start,
            MainAxisAlignment::End,
            MainAxisAlignment::Center,
            MainAxisAlignment::SpaceBetween,
            MainAxisAlignment::SpaceAround,
            MainAxisAlignment::SpaceEvenly,
        ] {
            let (start, _) = alignment_spacing(alignment, 0.0, 2, 0.0);
            assert_eq!(start, 0.0, "{alignment:?} moved a full row");
        }
    }

    #[test]
    fn alignment_with_a_single_child_does_not_divide_by_zero() {
        for alignment in [
            MainAxisAlignment::SpaceBetween,
            MainAxisAlignment::SpaceAround,
            MainAxisAlignment::SpaceEvenly,
        ] {
            let (start, gap) = alignment_spacing(alignment, 30.0, 1, 5.0);
            assert!(start.is_finite());
            assert!(gap.is_finite(), "{alignment:?} produced a non-finite gap");
        }
    }

    #[test]
    fn cross_axis_end_aligns_the_far_edge() {
        let mut nodes = Arena::new();
        let tall = leaf(&mut nodes, 20.0, 40.0);
        let short = leaf(&mut nodes, 20.0, 10.0);
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new().with_cross_axis_alignment(CrossAxisAlignment::End),
            &[tall, short],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(tall).unwrap().layout().rect().unwrap().origin.y,
            0.0
        );
        assert_eq!(
            nodes.get(short).unwrap().layout().rect().unwrap().origin.y,
            30.0
        );
    }

    #[test]
    fn cross_axis_stretch_fills_the_box() {
        let mut nodes = Arena::new();
        // A fixed width and no opinion on the height, which is what stretching
        // acts on.
        let child = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::new(20.0, 20.0, 0.0, f32::INFINITY)),
        );
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new().with_cross_axis_alignment(CrossAxisAlignment::Stretch),
            &[child],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(child).unwrap().layout().rect().unwrap().size,
            Size::new(20.0, 40.0)
        );
    }

    #[test]
    fn cross_axis_stretch_does_not_stretch_a_fixed_child() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, 20.0, 10.0);
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new().with_cross_axis_alignment(CrossAxisAlignment::Stretch),
            &[child],
        );

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(child).unwrap().layout().rect().unwrap().size,
            Size::new(20.0, 10.0),
            "a child that declared both extents keeps them"
        );
    }

    #[test]
    fn a_flexible_child_fills_the_cross_axis_by_default() {
        let mut nodes = Arena::new();
        // A flexible child has no intrinsic extent to pack against, so it takes
        // what the box has.
        let child = node::create(&mut nodes, LayoutState::new().with_flex(1.0));
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[child]);

        layout_in(&mut nodes, row, Size::new(100.0, 40.0));

        assert_eq!(
            nodes.get(child).unwrap().layout().rect().unwrap().size,
            Size::new(100.0, 40.0)
        );
    }

    #[test]
    fn an_overflowing_child_keeps_its_size_and_computes_a_clip_rect() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, 80.0, 20.0);
        let b = leaf(&mut nodes, 80.0, 20.0);
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[a, b]);

        layout_in(&mut nodes, row, Size::new(100.0, 20.0));

        let b_rect = nodes.get(b).unwrap().layout().rect().unwrap();
        assert_eq!(b_rect, Rect::from_parts(80.0, 0.0, 80.0, 20.0));
        assert_eq!(
            nodes.get(b).unwrap().layout().clip(),
            Some(Rect::from_parts(0.0, 0.0, 100.0, 20.0)),
            "the pass computes the rect a renderer would clip to; it does not \
             clip anything itself"
        );
    }

    #[test]
    fn clip_is_the_intersection_of_the_ancestor_boxes() {
        let mut nodes = Arena::new();
        let inner = leaf(&mut nodes, 10.0, 10.0);
        let stack = container(&mut nodes, LayoutMode::Stack, FlexConfig::new(), &[inner]);
        let root = container(&mut nodes, LayoutMode::Stack, FlexConfig::new(), &[stack]);

        layout_in(&mut nodes, root, Size::new(100.0, 100.0));

        assert_eq!(nodes.get(root).unwrap().layout().clip(), None);
        assert_eq!(
            nodes.get(stack).unwrap().layout().clip(),
            Some(Rect::from_parts(0.0, 0.0, 100.0, 100.0))
        );
        assert_eq!(
            nodes.get(inner).unwrap().layout().clip(),
            Some(Rect::from_parts(0.0, 0.0, 10.0, 10.0)),
            "the inner node is clipped by the stack's own box"
        );
    }

    #[test]
    fn an_unbounded_main_axis_shrinks_to_the_content() {
        // Every alignment, because the one that was covered — `Start` — is the
        // one that cannot tell a box that fell back to the content size from a
        // box that placed the content at a negative offset. The unbounded branch
        // sets `available_main` from the content and then runs the distribution,
        // so a non-`Start` alignment is what exercises that path.
        for alignment in [
            MainAxisAlignment::Start,
            MainAxisAlignment::End,
            MainAxisAlignment::Center,
            MainAxisAlignment::SpaceBetween,
            MainAxisAlignment::SpaceAround,
            MainAxisAlignment::SpaceEvenly,
        ] {
            let mut nodes = Arena::new();
            let a = leaf(&mut nodes, 30.0, 10.0);
            let b = leaf(&mut nodes, 20.0, 10.0);
            let column = container(
                &mut nodes,
                LayoutMode::column(),
                FlexConfig::new()
                    .with_spacing(5.0)
                    .with_main_axis_alignment(alignment),
                &[a, b],
            );

            Layout::new(&mut nodes).layout(column, Constraints::UNBOUNDED);

            // As wide as the widest child, as tall as both plus the gap, with
            // nothing left over for an alignment to divide.
            assert_eq!(
                nodes.get(column).unwrap().layout().rect().unwrap().size,
                Size::new(30.0, 25.0),
                "{alignment:?} sized an unbounded column wrongly"
            );
            assert_eq!(
                nodes.get(a).unwrap().layout().rect().unwrap().origin.y,
                0.0,
                "{alignment:?} moved the first child in an unbounded column"
            );
            assert_eq!(
                nodes.get(b).unwrap().layout().rect().unwrap().origin.y,
                15.0,
                "{alignment:?} misplaced the second child in an unbounded column"
            );
        }
    }

    #[test]
    fn a_clip_is_the_overlap_of_two_rects_that_start_apart() {
        let mut nodes = Arena::new();
        // The parent's box starts at the origin, and the child is placed
        // 12 pixels into it, so the two rects the intersection is taken from
        // have different origins and the result is not simply the smaller one.
        let leaf = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::tight(Size::new(20.0, 20.0))),
        );
        let inner = container(&mut nodes, LayoutMode::Absolute, FlexConfig::new(), &[leaf]);
        nodes
            .get_mut(inner)
            .unwrap()
            .layout_mut()
            .set_position(Some(Offset::new(12.0, 12.0)));
        let parent = container(
            &mut nodes,
            LayoutMode::Absolute,
            FlexConfig::new(),
            &[inner],
        );
        nodes
            .get_mut(parent)
            .unwrap()
            .layout_mut()
            .set_position(Some(Offset::new(2.0, 2.0)));
        let root = container(
            &mut nodes,
            LayoutMode::Absolute,
            FlexConfig::new(),
            &[parent],
        );

        layout_in(&mut nodes, root, Size::new(100.0, 100.0));

        // `inner` sits at (14, 14) and is 20 wide, so it runs to 34, and
        // `parent` is 36 wide from (2, 2) to (38, 38). The overlap is 20 pixels
        // on each axis, and it starts at the child's own origin.
        assert_eq!(
            nodes.get(leaf).unwrap().layout().clip(),
            Some(Rect::from_parts(14.0, 14.0, 20.0, 20.0)),
            "the clip is the overlap of the two boxes, not the whole of either"
        );
    }

    #[test]
    fn grid_has_no_algorithm_yet() {
        // Requirement 1 names the mode; its algorithm is out of scope for this
        // task, and this test records the placeholder rather than hiding it.
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, 10.0, 10.0);
        let rects = layout_constraints(
            &nodes,
            &[child],
            Constraints::tight(Size::new(100.0, 100.0)),
            LayoutMode::Grid { columns: 2 },
            &FlexConfig::new(),
        );
        assert!(rects.is_empty());
    }

    #[test]
    fn nested_row_inside_a_column_is_positioned() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, 100.0, 40.0);
        let b = leaf(&mut nodes, 50.0, 40.0);
        let row = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new().with_spacing(10.0),
            &[a, b],
        );
        let below = leaf(&mut nodes, 200.0, 30.0);
        let column = container(
            &mut nodes,
            LayoutMode::column(),
            FlexConfig::new().with_spacing(20.0),
            &[row, below],
        );

        layout_in(&mut nodes, column, Size::new(300.0, 200.0));

        assert_eq!(
            nodes.get(row).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 160.0, 40.0)
        );
        assert_eq!(
            nodes.get(a).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 100.0, 40.0)
        );
        assert_eq!(
            nodes.get(b).unwrap().layout().rect().unwrap(),
            Rect::from_parts(110.0, 0.0, 50.0, 40.0),
            "the inner row's child is placed in the inner row's coordinates"
        );
        assert_eq!(
            nodes.get(below).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 60.0, 200.0, 30.0)
        );
    }

    #[test]
    fn a_child_is_clamped_by_its_declared_minimum() {
        let mut nodes = Arena::new();
        let child = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::new(
                50.0,
                f32::INFINITY,
                0.0,
                f32::INFINITY,
            )),
        );
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[child]);

        // The parent offers only 20 pixels; the child's declared minimum wins.
        layout_in(&mut nodes, row, Size::new(20.0, 20.0));

        assert_eq!(
            nodes
                .get(child)
                .unwrap()
                .layout()
                .rect()
                .unwrap()
                .size
                .width,
            50.0
        );
    }

    #[test]
    fn a_stale_child_handle_keeps_the_result_index_aligned() {
        let mut nodes = Arena::new();
        let live = leaf(&mut nodes, 10.0, 10.0);
        let dead = leaf(&mut nodes, 10.0, 10.0);
        assert!(nodes.remove(dead).is_some());

        let rects = layout_constraints(
            &nodes,
            &[dead, live],
            Constraints::tight(Size::new(100.0, 100.0)),
            LayoutMode::row(),
            &FlexConfig::new(),
        );
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], Rect::ZERO);
        assert_eq!(rects[1], Rect::from_parts(0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn a_default_state_is_dirty_and_unplaced() {
        let state = LayoutState::new();
        assert!(state.is_dirty());
        assert_eq!(state.rect(), None);
        assert_eq!(state.clip(), None);
        assert_eq!(state.placed_under(), None);
        assert_eq!(state.flex(), 0.0);
        assert_eq!(state.position(), None);
        assert_eq!(state.mode(), LayoutMode::Stack);
        assert_eq!(state.constraints(), Constraints::UNBOUNDED);
    }

    #[test]
    fn meaning_a_change_to_a_node_marks_it_dirty() {
        let mut state = LayoutState::new();
        state.mark_dirty();
        assert!(state.is_dirty());
    }

    #[test]
    fn a_dirty_leaf_is_reached_through_a_deep_chain_of_clean_nodes() {
        let mut nodes = Arena::new();
        // The shape the layout walk is expensive on, and the one its cost is
        // recorded against: a deep tree where one node at the bottom changes.
        // The chain is 200 links, because a two-level fixture cannot tell a
        // working walk from one that only looks one level down.
        let leaf = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::tight(Size::new(10.0, 10.0))),
        );
        let mut root = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[leaf]);
        for _ in 1..200 {
            root = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[root]);
        }
        let size = Size::new(100.0, 100.0);
        layout_in(&mut nodes, root, size);
        assert_eq!(
            nodes.get(leaf).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 10.0, 10.0),
            "the leaf starts at the origin, sized as it asked to be"
        );

        // Marked through the node's own setter, so no ancestor is dirtied: the
        // 200 links above it stay clean and the walk is the only thing that
        // carries the change to the bottom.
        nodes
            .get_mut(leaf)
            .unwrap()
            .layout_mut()
            .set_constraints(Constraints::tight(Size::new(30.0, 10.0)));
        assert!(
            !nodes.get(root).unwrap().layout().is_dirty(),
            "the root is clean and stays clean"
        );

        layout_in(&mut nodes, root, size);

        assert_eq!(
            nodes.get(leaf).unwrap().layout().rect().unwrap(),
            Rect::from_parts(0.0, 0.0, 30.0, 10.0),
            "the change at the bottom of a deep chain reaches the pass"
        );
    }

    #[test]
    fn a_clean_tree_keeps_its_rects_and_still_reaches_a_dirty_descendant() {
        let mut nodes = Arena::new();
        // Bounded but not fixed, so a flex factor has room to change it — a
        // tight child could not grow whatever the pass was told.
        let child = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::new(0.0, 50.0, 0.0, 50.0)),
        );
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[child]);
        let constraints = Constraints::tight(Size::new(100.0, 100.0));

        assert_eq!(
            laid_out_by(&mut nodes, row, constraints),
            2,
            "the first pass lays out both nodes"
        );
        let first = nodes.get(child).unwrap().layout().rect().unwrap();
        assert_eq!(first.size, Size::ZERO, "a child with no content is empty");

        // Requirement 3: only dirty subtrees are laid out. The rects being
        // unchanged cannot show this on its own — a full re-layout produces the
        // same rects — so the pass counts the work instead. Zero is the whole
        // claim, and it is the claim a removed skip would break.
        assert_eq!(
            laid_out_by(&mut nodes, row, constraints),
            0,
            "a pass over a clean tree must not lay out any node"
        );
        assert_eq!(nodes.get(child).unwrap().layout().rect().unwrap(), first);

        // A leaf with nothing inside it cannot be resized by a flex factor, so
        // give it one child and then a factor: the factor has content to work
        // with, and the box is bounded so there is room to grow into.
        let inner = node::create(&mut nodes, LayoutState::new());
        assert!(node::attach(&mut nodes, child, inner));
        nodes.get_mut(child).unwrap().layout_mut().set_flex(1.0);
        mark_dirty(&mut nodes, child);
        assert_eq!(
            laid_out_by(&mut nodes, row, constraints),
            3,
            "the marked path is laid out again, and so is the new child in it"
        );
        assert_eq!(
            nodes.get(child).unwrap().layout().rect().unwrap().size,
            Size::new(50.0, 50.0),
            "the flex factor has room to grow the child"
        );

        // A clean ancestor does not hide a dirty descendant. Here only the
        // middle row was ever marked: the root above it is clean and stays
        // clean, and the pass still has to reach the change. The count is what
        // makes this a test of the walk rather than of the rects.
        assert!(
            !nodes.get(row).unwrap().layout().is_dirty(),
            "the root is clean, and the walk is what finds the change"
        );
        nodes.get_mut(child).unwrap().layout_mut().set_flex(0.0);
        assert!(nodes.get(child).unwrap().layout().is_dirty());
        // Three, not two: the box `inner` sits in narrows from 50x50 to 0x0
        // with its parent, so its incoming constraints and its clip both change
        // and it is laid out again — even though its own rect is 0x0 before
        // and after, byte for byte. A rect that did not move is not what the
        // pass goes by.
        assert_eq!(
            laid_out_by(&mut nodes, row, constraints),
            3,
            "a clean root is walked through to the dirty node below it"
        );
        assert_eq!(
            nodes.get(child).unwrap().layout().rect().unwrap().size,
            Size::ZERO,
            "a clean ancestor does not swallow a dirty descendant"
        );
    }

    #[test]
    fn a_resized_sibling_moves_a_clean_subtree() {
        let mut nodes = Arena::new();
        let inner = leaf(&mut nodes, 10.0, 10.0);
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[inner]);
        let sibling = leaf(&mut nodes, 30.0, 30.0);
        let column = container(
            &mut nodes,
            LayoutMode::column(),
            FlexConfig::new().with_spacing(20.0),
            // The sibling comes first, so it is what pushes the row down.
            &[sibling, row],
        );

        layout_in(&mut nodes, column, Size::new(100.0, 100.0));
        assert_eq!(
            nodes.get(row).unwrap().layout().rect().unwrap().origin,
            Offset::new(0.0, 50.0)
        );

        // Nothing about the row or the leaf inside it changed, but the row is
        // placed 30 pixels lower, so its child has to be placed there too.
        nodes
            .get_mut(sibling)
            .unwrap()
            .layout_mut()
            .set_constraints(Constraints::tight(Size::new(30.0, 60.0)));
        mark_dirty(&mut nodes, sibling);
        layout_in(&mut nodes, column, Size::new(100.0, 100.0));

        assert_eq!(
            nodes.get(row).unwrap().layout().rect().unwrap().origin,
            Offset::new(0.0, 80.0)
        );
        assert_eq!(
            nodes.get(inner).unwrap().layout().rect().unwrap().origin,
            Offset::new(0.0, 80.0),
            "a moved node takes its children with it"
        );
    }

    #[test]
    fn mark_dirty_reaches_the_ancestors() {
        let mut nodes = Arena::new();
        let grandchild = leaf(&mut nodes, 10.0, 10.0);
        let child = container(
            &mut nodes,
            LayoutMode::Stack,
            FlexConfig::new(),
            &[grandchild],
        );
        let root = container(&mut nodes, LayoutMode::Stack, FlexConfig::new(), &[child]);
        layout_in(&mut nodes, root, Size::new(100.0, 100.0));
        assert!(!nodes.get(root).unwrap().layout().is_dirty());
        assert!(!nodes.get(child).unwrap().layout().is_dirty());

        mark_dirty(&mut nodes, grandchild);

        assert!(nodes.get(grandchild).unwrap().layout().is_dirty());
        assert!(nodes.get(child).unwrap().layout().is_dirty());
        assert!(nodes.get(root).unwrap().layout().is_dirty());
    }

    #[test]
    fn a_stale_handle_is_ignored_by_the_pass() {
        let mut nodes = Arena::new();
        let child = leaf(&mut nodes, 10.0, 10.0);
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[child]);
        assert!(nodes.remove(row).is_some());

        // The pass returns quietly rather than panicking on the stale root.
        Layout::new(&mut nodes).layout(row, Constraints::tight(Size::new(10.0, 10.0)));
        assert!(nodes.get(child).unwrap().layout().rect().is_none());
    }

    #[test]
    fn a_removed_child_is_skipped_but_others_are_placed() {
        let mut nodes = Arena::new();
        let a = leaf(&mut nodes, 10.0, 10.0);
        let b = leaf(&mut nodes, 10.0, 10.0);
        let row = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &[a, b]);
        assert!(node::detach(&mut nodes, row, a));
        assert!(nodes.remove(a).is_some());

        layout_in(&mut nodes, row, Size::new(100.0, 100.0));

        assert!(nodes.get(b).unwrap().layout().rect().is_some());
    }

    /// Measures what the layout walk in `visit`'s guard costs, on the three
    /// shapes quoted in `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the
    /// spec, and why*.
    ///
    /// This is a measurement harness, **not a test**: it reads the wall clock,
    /// so it is `#[ignore]`d and never runs as part of the suite. Run it with
    ///
    /// ```text
    /// cargo test -p ui_core --release --all-features --lib \
    ///     layout_walk_cost -- --ignored --nocapture
    /// ```
    ///
    /// and compare the output against the entry, which is where the numbers
    /// are meant to be reproducible from. The shapes are described there
    /// because the ratios depend entirely on them: a chain whose links declare
    /// constraints is linear apart from the walk, and a chain whose links
    /// declare none is already O(n·d) through `resolve_box`, so the walk is
    /// nearly free there. Quoting one without the other is what makes a cost
    /// figure unreproducible.
    #[test]
    #[ignore]
    fn layout_walk_cost() {
        use std::time::Instant;

        /// Collects `handle` and every node below it.
        fn below(nodes: &Arena<WidgetNode>, handle: Handle) -> Vec<Handle> {
            let mut all = vec![handle];
            if let Some(node) = nodes.get(handle) {
                for &child in node.children() {
                    all.extend(below(nodes, child));
                }
            }
            all
        }

        /// Best of `runs` passes over `root`, in nanoseconds.
        fn best_of(
            nodes: &mut Arena<WidgetNode>,
            root: Handle,
            size: Size,
            runs: usize,
            touch: impl Fn(&mut Arena<WidgetNode>),
        ) -> f64 {
            let mut best = f64::MAX;
            for _ in 0..runs {
                touch(nodes);
                let started = Instant::now();
                Layout::new(nodes).layout(root, Constraints::tight(size));
                best = best.min(started.elapsed().as_secs_f64() * 1e9);
            }
            best
        }

        let size = Size::new(1024.0, 768.0);
        let report = |label: &str, ns: f64| println!("COST {label}: {ns:.0} ns");
        let ratio = |label: &str, value: f64| println!("COST {label}: {value:.1}x");

        // Shape 1: one row of 2040 fixed-size leaves under a root, 2041 nodes,
        // all clean. This is the frame where nothing changed.
        let mut nodes = Arena::new();
        let leaves: Vec<Handle> = (0..2040).map(|_| leaf(&mut nodes, 4.0, 4.0)).collect();
        let flat = container(&mut nodes, LayoutMode::row(), FlexConfig::new(), &leaves);
        layout_in(&mut nodes, flat, size);
        report(
            "flat-2041 clean pass",
            best_of(&mut nodes, flat, size, 1000, |_| {}),
        );

        // Shapes 2 and 3: a chain `depth` links deep with one dirty leaf under a
        // clean root, against a cold pass over the same tree.
        for (label, declares) in [("declaring", true), ("bare", false)] {
            for depth in [200usize, 1000] {
                let mut nodes = Arena::new();
                let make = |nodes: &mut Arena<WidgetNode>| {
                    let leaf = node::create(
                        nodes,
                        LayoutState::new()
                            .with_constraints(Constraints::tight(Size::new(10.0, 10.0))),
                    );
                    let mut root = container(nodes, LayoutMode::row(), FlexConfig::new(), &[leaf]);
                    for _ in 1..depth {
                        let state = if declares {
                            LayoutState::new()
                                .with_mode(LayoutMode::row())
                                .with_flex_config(FlexConfig::new())
                                .with_constraints(Constraints::tight(Size::new(10.0, 10.0)))
                        } else {
                            LayoutState::new()
                                .with_mode(LayoutMode::row())
                                .with_flex_config(FlexConfig::new())
                        };
                        let link = node::create(nodes, state);
                        assert!(node::attach(nodes, link, root));
                        root = link;
                    }
                    (root, leaf)
                };
                let (root, leaf) = make(&mut nodes);
                layout_in(&mut nodes, root, size);

                let all = below(&nodes, root);
                let dirty_leaf = |nodes: &mut Arena<WidgetNode>| {
                    nodes.get_mut(leaf).unwrap().layout_mut().set_flex(1.0);
                };
                let one = best_of(&mut nodes, root, size, 20, dirty_leaf);
                let cold = best_of(&mut nodes, root, size, 20, |nodes| {
                    for &handle in &all {
                        nodes.get_mut(handle).unwrap().layout_mut().mark_dirty();
                    }
                });
                report(&format!("depth-{depth} {label} one dirty leaf"), one);
                report(&format!("depth-{depth} {label} cold pass"), cold);
                ratio(&format!("depth-{depth} {label} ratio"), one / cold);
            }
        }
    }
}
