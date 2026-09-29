//! Demo harness for `ui_core`.
//!
//! Opens a window with an OpenGL ES 3.1 context and draws a widget tree: the
//! layout pass gives every node a rect, and each node paints itself from the
//! rect it was given. What a node looks like is the demo's business rather than
//! the library's, so the visuals live in a map beside the tree instead of on
//! the node.

use sdl3::event::{Event, WindowEvent};
use std::collections::HashMap;
use std::time::Duration;
use ui_core::arena::{Arena, Handle};
use ui_core::layout::{
    Constraints, CrossAxisAlignment, FlexConfig, Layout, LayoutMode, LayoutState,
    MainAxisAlignment, Size,
};
use ui_core::node::{self, WidgetNode};
use ui_core::paint::{self, Color, PaintState, Painter};
use ui_core::render::context::Context;
use ui_core::render::Renderer;

/// The window, and the box the root is laid out in.
const WINDOW: Size = Size {
    width: 1024.0,
    height: 600.0,
};

/// How long the loop blocks waiting for the next event. Nothing moves on
/// screen, so this paces the loop rather than budgeting a frame.
const EVENT_WAIT: Duration = Duration::from_millis(16);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = Renderer::new(Context::new(
        "roados ui_demo",
        f32_to_u32(WINDOW.width),
        f32_to_u32(WINDOW.height),
    )?)?;
    let sdl = renderer.sdl();
    let mut events = sdl.event_pump()?;
    let mut demo = Demo::new()?;

    'running: loop {
        let event = events.wait_event_timeout(EVENT_WAIT);
        if let Some(
            Event::Quit { .. }
            | Event::Window {
                win_event: WindowEvent::CloseRequested,
                ..
            },
        ) = event
        {
            break 'running;
        }

        renderer.begin_frame();
        demo.frame(WINDOW);
        demo.draw(&mut renderer);
        renderer.end_frame()?;
    }

    Ok(())
}

/// Converts a float extent to the pixel count the window is created with.
///
/// There is no `From`/`TryFrom` between `f32` and any unsigned integer in std,
/// so this is the one place a float-to-integer `as` cast is used, for the same
/// reason and with the same guarantee as the renderer's own: the cast is
/// saturating, so an extent outside the range clamps instead of wrapping.
fn f32_to_u32(value: f32) -> u32 {
    value as u32
}

/// What a node paints itself as, once layout has given it a rect.
#[derive(Clone, Copy)]
enum Visual {
    /// A rectangle in the given color.
    Solid(Color),
    /// A rectangle with corners rounded by the given radius.
    Rounded(f32, Color),
    /// A circle inscribed in the node's rect.
    Circle(Color),
}

impl Visual {
    /// Records this visual into `painter`, filling `rect`.
    fn paint(&self, painter: &mut Painter, rect: paint::Rect) {
        match self {
            Visual::Solid(color) => painter.rect(rect, *color),
            Visual::Rounded(radius, color) => painter.rounded_rect(rect, *radius, *color),
            Visual::Circle(color) => painter.circle(
                (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
                rect.width.min(rect.height) / 2.0,
                *color,
            ),
        }
    }
}

/// The demo's widget tree, what each of its nodes paints, and the order to
/// paint them in.
struct Demo {
    nodes: Arena<WidgetNode>,
    visuals: HashMap<Handle, Visual>,
    root: Handle,
    order: Vec<Handle>,
}

impl Demo {
    /// Builds the tree the demo draws: a centred column of two rows. The upper
    /// one holds three fixed-size shapes; the lower one a circle and a stack of
    /// two rectangles that overlap.
    ///
    /// The error is a message rather than a type of its own: the tree is written
    /// out here, so a node that cannot be attached is a bug in this file and not
    /// a runtime condition a caller could act on.
    fn new() -> Result<Self, &'static str> {
        let mut nodes = Arena::new();
        let mut visuals = HashMap::new();

        let red = leaf(
            &mut nodes,
            &mut visuals,
            Size::new(220.0, 140.0),
            Visual::Solid(Color::new(230, 60, 60, 255)),
        );
        let green = leaf(
            &mut nodes,
            &mut visuals,
            Size::new(220.0, 140.0),
            Visual::Rounded(28.0, Color::new(60, 200, 90, 255)),
        );
        let blue = leaf(
            &mut nodes,
            &mut visuals,
            Size::new(220.0, 140.0),
            Visual::Solid(Color::new(70, 110, 230, 255)),
        );
        let top = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new()
                .with_spacing(52.0)
                .with_main_axis_alignment(MainAxisAlignment::Center),
            &[red, green, blue],
        )?;

        let circle = leaf(
            &mut nodes,
            &mut visuals,
            Size::new(140.0, 140.0),
            Visual::Circle(Color::new(240, 200, 60, 255)),
        );
        let panel = leaf(
            &mut nodes,
            &mut visuals,
            Size::new(320.0, 120.0),
            Visual::Solid(Color::new(50, 60, 80, 255)),
        );
        let highlight = leaf(
            &mut nodes,
            &mut visuals,
            Size::new(320.0, 120.0),
            Visual::Solid(Color::new(120, 60, 110, 128)),
        );
        // A stack places both rectangles at the same rect, so the translucent
        // one lands on top of the opaque one.
        let stack = container(
            &mut nodes,
            LayoutMode::Stack,
            FlexConfig::new(),
            &[panel, highlight],
        )?;
        let bottom = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new()
                .with_spacing(24.0)
                .with_main_axis_alignment(MainAxisAlignment::Center)
                .with_cross_axis_alignment(CrossAxisAlignment::End),
            &[circle, stack],
        )?;

        let root = container(
            &mut nodes,
            LayoutMode::column(),
            FlexConfig::new()
                .with_spacing(40.0)
                .with_main_axis_alignment(MainAxisAlignment::Center)
                .with_cross_axis_alignment(CrossAxisAlignment::Center),
            &[top, bottom],
        )?;

        // The tree never changes shape, so the order is computed once.
        let order = paint_order(&nodes, root);
        Ok(Demo {
            nodes,
            visuals,
            root,
            order,
        })
    }

    /// Lays the tree out in `size` and records every node's draw commands.
    ///
    /// The commands are recorded every frame rather than on a dirty flag: the
    /// renderer takes a node's commands once, and a frame with nothing to draw
    /// would clear the window to black.
    fn frame(&mut self, size: Size) {
        let Demo {
            nodes,
            visuals,
            root,
            order,
            ..
        } = self;
        Layout::new(nodes).layout(*root, Constraints::tight(size));

        for handle in order.iter().copied() {
            let Some(visual) = visuals.get(&handle) else {
                continue;
            };
            let Some(node) = nodes.get_mut(handle) else {
                continue;
            };
            let mut painter = Painter::new();
            if let Some(rect) = node.layout().rect() {
                // The layout rect is absolute, and the draw command's is the
                // same rectangle in the terms the renderer batches.
                visual.paint(&mut painter, rect.into());
            }
            *node.paint_mut() = PaintState::from_commands(painter.finish());
        }
    }

    /// Hands the recorded commands to the renderer, in paint order.
    fn draw(&mut self, renderer: &mut Renderer) {
        let Demo { nodes, order, .. } = self;
        for handle in order.iter().copied() {
            renderer.draw_node(handle, nodes);
        }
    }
}

/// Returns the handles of the tree below `root` in paint order: a parent, then
/// its children in the order its layout mode places them. A `Stack` places them
/// all at the same rect, and the later one covers the earlier, so the order
/// decides which is on top.
fn paint_order(nodes: &Arena<WidgetNode>, root: Handle) -> Vec<Handle> {
    fn walk(nodes: &Arena<WidgetNode>, handle: Handle, into: &mut Vec<Handle>) {
        into.push(handle);
        if let Some(node) = nodes.get(handle) {
            for &child in node.children() {
                walk(nodes, child, into);
            }
        }
    }
    let mut order = Vec::new();
    walk(nodes, root, &mut order);
    order
}

/// Adds a leaf of `size` that paints as `visual`.
fn leaf(
    nodes: &mut Arena<WidgetNode>,
    visuals: &mut HashMap<Handle, Visual>,
    size: Size,
    visual: Visual,
) -> Handle {
    let handle = node::create(
        nodes,
        LayoutState::new().with_constraints(Constraints::tight(size)),
    );
    visuals.insert(handle, visual);
    handle
}

/// Adds a container in `mode` that places `children` with `config`, and returns
/// its handle.
fn container(
    nodes: &mut Arena<WidgetNode>,
    mode: LayoutMode,
    config: FlexConfig,
    children: &[Handle],
) -> Result<Handle, &'static str> {
    let handle = node::create(
        nodes,
        LayoutState::new().with_mode(mode).with_flex_config(config),
    );
    for &child in children {
        if !node::attach(nodes, handle, child) {
            return Err("ui_demo: a demo node could not be attached");
        }
    }
    Ok(handle)
}
