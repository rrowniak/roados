//! Demo harness for `ui_core`.
//!
//! Opens a window with an OpenGL ES 3.1 context and waits. The widgets and
//! the drawing arrive in later tasks; what is here is the smallest thing that
//! proves the vendored SDL build produces a window with a working GL context
//! on this host.

use sdl3::event::{Event, WindowEvent};
use std::time::Duration;
use ui_core::render::context::Context;

/// How long the loop blocks waiting for the next event. Nothing is drawn yet,
/// so this paces the loop rather than budgeting a frame.
const EVENT_WAIT: Duration = Duration::from_millis(16);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new("roados ui_demo", 1024, 600)?;
    let sdl = context.sdl();
    let mut events = sdl.event_pump()?;

    'running: loop {
        // `wait_event_timeout` returns `None` on both timeout and SDL error.
        // Treating `None` as "continue" is the current task 02 behavior; the
        // error case is a known limitation to address when the loop starts
        // drawing in a later task.
        let Some(event) = events.wait_event_timeout(EVENT_WAIT) else {
            continue;
        };

        match event {
            Event::Quit { .. }
            | Event::Window {
                win_event: WindowEvent::CloseRequested,
                ..
            } => break 'running,
            _ => {}
        }

        context.swap();
    }

    Ok(())
}
