//! Demo harness for `ui_core`.
//!
//! Opens a window with an OpenGL ES 3.1 context and draws colored rectangles
//! through the rendering pipeline: draw commands are recorded into per-node
//! paint states, batched by material, and submitted to the GPU each frame.
//! The widget tree arrives with the layout system (task 07); until then the
//! demo records its rectangles directly into the arena.

use sdl3::event::{Event, WindowEvent};
use std::time::Duration;
use ui_core::arena::Arena;
use ui_core::paint::{Color, PaintState, Painter, Rect};
use ui_core::render::context::Context;
use ui_core::render::Renderer;

/// How long the loop blocks waiting for the next event. Nothing moves on
/// screen, so this paces the loop rather than budgeting a frame.
const EVENT_WAIT: Duration = Duration::from_millis(16);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = Renderer::new(Context::new("roados ui_demo", 1024, 600)?)?;
    let sdl = renderer.sdl();
    let mut events = sdl.event_pump()?;

    let mut nodes = Arena::new();
    let main = nodes.insert(PaintState::new());

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

        let mut painter = Painter::new();
        painter.rect(
            Rect::new(48.0, 48.0, 220.0, 140.0),
            Color::new(230, 60, 60, 255),
        );
        painter.rounded_rect(
            Rect::new(320.0, 48.0, 220.0, 140.0),
            28.0,
            Color::new(60, 200, 90, 255),
        );
        painter.rect(
            Rect::new(592.0, 48.0, 220.0, 140.0),
            Color::new(70, 110, 230, 255),
        );
        painter.circle((160.0, 330.0), 70.0, Color::new(240, 200, 60, 255));
        painter.rect(
            Rect::new(320.0, 100.0, 220.0, 140.0),
            Color::new(120, 60, 110, 128),
        );

        if let Some(state) = nodes.get_mut(main) {
            *state = PaintState::from_commands(painter.finish());
        }

        renderer.begin_frame();
        renderer.draw_node(main, &mut nodes);
        renderer.end_frame()?;
    }

    Ok(())
}
