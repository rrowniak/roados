//! Demo harness for `ui_core`.
//!
//! Opens a window with an OpenGL ES 3.1 context and draws three pads. A pad
//! has one property — how far it is pressed, from 0 at rest to 1 held — and
//! paints itself by interpolating between its rest and held colours from that
//! value at paint time: the animation writes a single number, and the pad's
//! whole appearance follows.
//!
//! Pressing the left mouse button over a pad presses it; holding the space bar
//! presses all three, cascading across them with a stagger. Releasing springs
//! them back to rest. Every press property carries an `on_change` callback that
//! marks its pad's node dirty — the link from an animation to the node arena,
//! which the animation module itself knows nothing about.

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
use sdl3::mouse::MouseButton;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use ui_core::animation::{AnimationClock, AnyAnimation, Easing, Interpolate, Stagger};
use ui_core::arena::{Arena, Handle};
use ui_core::layout::{
    mark_dirty, Constraints, CrossAxisAlignment, FlexConfig, Layout, LayoutMode, LayoutState,
    MainAxisAlignment, Size,
};
use ui_core::node::{self, WidgetNode};
use ui_core::paint::{Color, PaintState, Painter};
use ui_core::property::Property;
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

/// The size of one pad.
const PAD_SIZE: Size = Size {
    width: 220.0,
    height: 140.0,
};

/// The gap between the pads.
const PAD_SPACING: f32 = 52.0;

/// The corner radius a pad is painted with.
const PAD_RADIUS: f32 = 28.0;

/// The pads' rest and held colours: a dark tone at rest, a bright one held.
const PAD_COLORS: [(Color, Color); 3] = [
    (
        Color {
            r: 64,
            g: 32,
            b: 32,
            a: 255,
        },
        Color {
            r: 224,
            g: 64,
            b: 64,
            a: 255,
        },
    ),
    (
        Color {
            r: 32,
            g: 64,
            b: 48,
            a: 255,
        },
        Color {
            r: 64,
            g: 192,
            b: 96,
            a: 255,
        },
    ),
    (
        Color {
            r: 32,
            g: 48,
            b: 96,
            a: 255,
        },
        Color {
            r: 64,
            g: 112,
            b: 224,
            a: 255,
        },
    ),
];

/// How long a pad takes to press down.
const PRESS_DURATION: Duration = Duration::from_millis(150);

/// How far apart the pads' presses cascade when the space bar is held.
const STAGGER_STEP: Duration = Duration::from_millis(60);

/// How long a pad takes to spring back to rest.
const RELEASE_DURATION: Duration = Duration::from_millis(400);

/// The spring a pad releases on: a little underdamped, so it settles with a
/// whisper of overshoot rather than a thud.
const RELEASE_SPRING: Easing = Easing::Spring {
    damping: 9.0,
    stiffness: 140.0,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = Renderer::new(Context::new(
        "roados ui_demo",
        f32_to_u32(WINDOW.width),
        f32_to_u32(WINDOW.height),
    )?)?;
    let sdl = renderer.sdl();
    let mut events = sdl.event_pump()?;
    let mut demo = Demo::new()?;

    let mut last = Instant::now();
    'running: loop {
        let event = events.wait_event_timeout(EVENT_WAIT);
        if matches!(
            &event,
            Some(
                Event::Quit { .. }
                    | Event::Window {
                        win_event: WindowEvent::CloseRequested,
                        ..
                    }
            )
        ) {
            break 'running;
        }
        if let Some(event) = event {
            demo.handle_event(event);
        }

        let now = Instant::now();
        let delta = now.duration_since(last);
        last = now;

        renderer.begin_frame();
        demo.frame(WINDOW, delta);
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

/// A pad that presses: one property says how far down it is, and its colour
/// follows.
struct Pad {
    /// How far the pad is pressed: 0 at rest, 1 held.
    press: Property<f32>,
    /// The colour the pad paints at while it is at rest.
    rest: Color,
    /// The colour the pad paints at while it is held.
    held: Color,
    /// The node the pad paints itself with.
    node: Handle,
}

impl Pad {
    /// Creates a pad whose press property is `press`, that paints at `rest`
    /// while the press is zero and at `held` while it is one.
    ///
    /// The pad has one property — how far pressed it is — and no colour
    /// property of its own: the colour is interpolated from the press at
    /// paint time, so an animation moves a single number and the pad's whole
    /// appearance follows it.
    fn new(press: Property<f32>, rest: Color, held: Color, node: Handle) -> Self {
        Pad {
            press,
            rest,
            held,
            node,
        }
    }

    /// Returns the colour to paint this pad at `press`: `rest` at zero, `held`
    /// at one, and the premultiplied interpolation between them at every value
    /// in between.
    fn color_at(&self, press: f32) -> Color {
        Color::interpolate(&self.rest, &self.held, press)
    }

    /// Returns the colour to paint this pad at its current press.
    fn color(&self) -> Color {
        self.color_at(self.press.get())
    }
}

/// The demo's widget tree, the pads that press, and the clock that drives
/// them.
struct Demo {
    nodes: Rc<RefCell<Arena<WidgetNode>>>,
    root: Handle,
    order: Vec<Handle>,
    pads: Vec<Pad>,
    clock: AnimationClock,
    mouse_pressed: Option<usize>,
}

impl Demo {
    /// Builds the demo: a centred row of three pads, each with a press
    /// property that starts the pad's node dirty whenever it is written.
    ///
    /// The error is a message rather than a type of its own: the tree is
    /// written out here, so a node that cannot be attached is a bug in this
    /// file and not a runtime condition a caller could act on.
    fn new() -> Result<Self, &'static str> {
        let mut nodes = Arena::new();
        let mut pads = Vec::new();

        for &(rest, held) in &PAD_COLORS {
            let node = node::create(
                &mut nodes,
                LayoutState::new().with_constraints(Constraints::tight(PAD_SIZE)),
            );
            pads.push(Pad::new(Property::new(0.0), rest, held, node));
        }

        let root = container(
            &mut nodes,
            LayoutMode::row(),
            FlexConfig::new()
                .with_spacing(PAD_SPACING)
                .with_main_axis_alignment(MainAxisAlignment::Center)
                .with_cross_axis_alignment(CrossAxisAlignment::Center),
            &[pads[0].node, pads[1].node, pads[2].node],
        )?;

        // The link from an animation to a node: every write a pad's press
        // property receives — from a clock tick, or from `animate_to` putting
        // the property at its start value — marks the pad's node dirty, so the
        // next pass picks up the colour the press implies. The animation
        // module knows nothing about nodes; this wiring is the demo's.
        let nodes = Rc::new(RefCell::new(nodes));
        for pad in &pads {
            let nodes = Rc::clone(&nodes);
            let node = pad.node;
            pad.press.on_change(move |_| {
                mark_dirty(&mut nodes.borrow_mut(), node);
            });
        }

        // The tree never changes shape, so the order is computed once.
        let order = paint_order(&nodes.borrow(), root);
        Ok(Demo {
            nodes,
            root,
            order,
            pads,
            clock: AnimationClock::new(),
            mouse_pressed: None,
        })
    }

    /// Handles one input event: the space bar presses all three pads with a
    /// stagger and releases them on the spring, and the left mouse button
    /// presses and releases the pad under the cursor.
    fn handle_event(&mut self, event: Event) {
        match event {
            Event::KeyDown {
                keycode: Some(Keycode::Space),
                repeat: false,
                ..
            } => self.press_all(),
            Event::KeyUp {
                keycode: Some(Keycode::Space),
                ..
            } => self.release_all(),
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if let Some(index) = self.pad_at(x, y) {
                    self.mouse_pressed = Some(index);
                    self.press_pad(index);
                }
            }
            Event::MouseButtonUp {
                mouse_btn: MouseButton::Left,
                ..
            } => {
                if let Some(index) = self.mouse_pressed.take() {
                    self.release_pad(index);
                }
            }
            _ => {}
        }
    }

    /// Advances the clock by one frame's worth of time, lays the tree out in
    /// `size`, and records every pad's draw commands.
    ///
    /// The clock goes first: an animation that finished mid-frame has to have
    /// written its last value — and marked its pad dirty through the
    /// property's callback — before the pass below reads the tree.
    fn frame(&mut self, size: Size, delta: Duration) {
        let _ = self.clock.tick(delta);

        let mut nodes = self.nodes.borrow_mut();
        Layout::new(&mut nodes).layout(self.root, Constraints::tight(size));

        for handle in self.order.iter().copied() {
            let Some(pad) = self.pads.iter().find(|pad| pad.node == handle) else {
                continue;
            };
            let Some(node) = nodes.get_mut(handle) else {
                continue;
            };
            let mut painter = Painter::new();
            if let Some(rect) = node.layout().rect() {
                // The colour is derived from the single press property here,
                // at paint time: the animation moves one number, and the
                // pad's whole appearance follows it.
                painter.rounded_rect(rect.into(), PAD_RADIUS, pad.color());
            }
            *node.paint_mut() = PaintState::from_commands(painter.finish());
        }
    }

    /// Hands the recorded commands to the renderer, in paint order.
    fn draw(&mut self, renderer: &mut Renderer) {
        let mut nodes = self.nodes.borrow_mut();
        for handle in self.order.iter().copied() {
            renderer.draw_node(handle, &mut nodes);
        }
    }

    /// Presses every pad, cascading across them one `STAGGER_STEP` apart.
    fn press_all(&mut self) {
        self.clock.clear();
        let animations: Vec<AnyAnimation> = self
            .pads
            .iter()
            .map(|pad| {
                pad.press
                    .animate_to(1.0, PRESS_DURATION, Easing::EaseOut)
                    .into()
            })
            .collect();
        Stagger::new(animations, STAGGER_STEP).play(&mut self.clock);
    }

    /// Releases every pad at once, on the release spring.
    fn release_all(&mut self) {
        self.clock.clear();
        for pad in &self.pads {
            self.clock
                .add(pad.press.animate_to(0.0, RELEASE_DURATION, RELEASE_SPRING));
        }
    }

    /// Presses one pad.
    ///
    /// The `clear` is whole-clock, not per-pad: a mouse press while the space
    /// bar is held drops the other pads' in-flight stagger animations, and
    /// they sit frozen at their last written values until the next
    /// space-driven press or release. The clock cannot replace one property's
    /// animation in isolation — `add` leaves both writing the property — so
    /// a proper fix needs per-property replacement in `AnimationClock`, which
    /// no task needs yet. Dropping the `clear` is not a fix either: a stale
    /// longer animation (the 400 ms release spring) would outlive the shorter
    /// new one and write the property back after it finishes.
    fn press_pad(&mut self, index: usize) {
        let Some(pad) = self.pads.get(index) else {
            return;
        };
        self.clock.clear();
        self.clock
            .add(pad.press.animate_to(1.0, PRESS_DURATION, Easing::EaseOut));
    }

    /// Releases one pad.
    ///
    /// Same whole-clock `clear` limitation as `press_pad`: a mouse release
    /// while the space bar is held strands the other pads mid-press, frozen
    /// at their last written values until the next space-driven press or
    /// release. A proper fix needs per-property replacement in
    /// `AnimationClock`, which no task needs yet; dropping the `clear` would
    /// let a stale shorter animation — the 150 ms press — write the property
    /// back in the middle of the new release.
    fn release_pad(&mut self, index: usize) {
        let Some(pad) = self.pads.get(index) else {
            return;
        };
        self.clock.clear();
        self.clock
            .add(pad.press.animate_to(0.0, RELEASE_DURATION, RELEASE_SPRING));
    }

    /// Returns the index of the pad whose laid-out rect contains `(x, y)`.
    fn pad_at(&self, x: f32, y: f32) -> Option<usize> {
        let nodes = self.nodes.borrow();
        self.pads.iter().position(|pad| {
            nodes
                .get(pad.node)
                .and_then(|node| node.layout().rect())
                .is_some_and(|rect| {
                    x >= rect.origin.x
                        && x <= rect.origin.x + rect.size.width
                        && y >= rect.origin.y
                        && y <= rect.origin.y + rect.size.height
                })
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Lays the demo out once, the way the first frame does, so a test can ask
    /// where the pads are.
    fn laid_out() -> Demo {
        let mut demo = Demo::new().unwrap();
        demo.frame(WINDOW, Duration::from_millis(16));
        demo
    }

    #[test]
    fn a_new_pad_is_at_rest_and_paints_its_rest_colour() {
        let demo = Demo::new().unwrap();
        let pad = &demo.pads[0];
        assert_eq!(pad.press.get(), 0.0);
        assert!(!pad.press.is_bound());
        assert_eq!(pad.color(), pad.rest);
        assert_eq!(pad.color_at(0.0), pad.rest);
    }

    #[test]
    fn a_fully_pressed_pad_paints_its_held_colour() {
        let demo = Demo::new().unwrap();
        let pad = &demo.pads[0];
        assert_eq!(pad.color_at(1.0), pad.held);
    }

    #[test]
    fn a_pad_interpolates_its_colour_at_paint_time() {
        // The colour is derived from the single press property at paint time
        // rather than animated: halfway pressed, the pad paints exactly halfway
        // between its rest and held colours.
        let demo = Demo::new().unwrap();
        let pad = &demo.pads[0];
        assert_eq!(pad.color_at(0.5), Color::new(144, 48, 48, 255));
    }

    #[test]
    fn the_demo_starts_with_three_pads_at_rest() {
        let demo = Demo::new().unwrap();
        assert_eq!(demo.pads.len(), 3);
        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 0.0);
        }
    }

    #[test]
    fn pressing_the_pads_cascades_them_with_a_stagger() {
        // Holding the space bar presses all three pads, one STAGGER_STEP
        // apart: after one step's worth of time the first is pressing and the
        // last has not started.
        let mut demo = Demo::new().unwrap();
        demo.press_all();
        let _ = demo.clock.tick(STAGGER_STEP);
        assert!(demo.pads[0].press.get() > 0.0, "the first pad is pressing");
        assert_eq!(
            demo.pads[2].press.get(),
            0.0,
            "and the last has not started: it is still in its stagger delay"
        );
    }

    #[test]
    fn a_pressed_pad_arrives_at_held() {
        let mut demo = Demo::new().unwrap();
        demo.press_all();
        for _ in 0..10 {
            let _ = demo.clock.tick(Duration::from_millis(50));
        }
        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 1.0, "the pad is fully pressed");
        }
        assert!(!demo.clock.is_animating());
    }

    #[test]
    fn releasing_a_pad_springs_it_back_to_rest() {
        let mut demo = Demo::new().unwrap();
        demo.press_all();
        for _ in 0..10 {
            let _ = demo.clock.tick(Duration::from_millis(50));
        }
        demo.release_all();
        for _ in 0..20 {
            let _ = demo.clock.tick(Duration::from_millis(50));
        }
        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 0.0, "the pad is back at rest");
        }
        assert!(!demo.clock.is_animating());
    }

    #[test]
    fn an_animated_press_marks_its_pad_dirty() {
        // The link from an animation to a node: the press property's
        // on_change callback marks the pad's node dirty, so the next pass
        // repaints it. The demo is laid out first, because a node is born
        // dirty and only a pass clears the flag — without that, the
        // assertion would hold whatever the callback did.
        let mut demo = laid_out();
        demo.press_all();
        let _ = demo.clock.tick(Duration::from_millis(16));
        let nodes = demo.nodes.borrow();
        assert!(
            nodes.get(demo.pads[0].node).unwrap().layout().is_dirty(),
            "the first pad's node was marked dirty by the animation's write"
        );
    }

    #[test]
    fn pressing_the_mouse_presses_the_pad_under_the_cursor() {
        let mut demo = laid_out();
        let rect = {
            let nodes = demo.nodes.borrow();
            nodes
                .get(demo.pads[1].node)
                .unwrap()
                .layout()
                .rect()
                .unwrap()
        };
        demo.handle_event(Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: rect.origin.x + rect.size.width / 2.0,
            y: rect.origin.y + rect.size.height / 2.0,
        });
        demo.frame(WINDOW, Duration::from_millis(50));
        assert!(
            demo.pads[1].press.get() > 0.0,
            "the pad under the cursor is pressing"
        );
        assert_eq!(demo.pads[0].press.get(), 0.0, "and the others are not");
        assert_eq!(demo.pads[2].press.get(), 0.0);
    }
}
