//! Demo harness for `ui_core`.
//!
//! Opens a window and waits. The GL context, the widgets and the drawing arrive
//! in later tasks; what is here is the smallest thing that proves the vendored
//! SDL build produces a window on this host.

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
use std::time::Duration;

/// How long the loop blocks waiting for the next event. Nothing is drawn yet, so
/// this paces the loop rather than budgeting a frame.
const EVENT_WAIT: Duration = Duration::from_millis(16);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = sdl3::init()?;
    let video = sdl.video()?;

    // Held for the lifetime of `main`; dropping it destroys the window, and
    // dropping `sdl` calls `SDL_Quit`.
    let _window = video
        .window("roados ui_demo", 1024, 600)
        .position_centered()
        .build()?;

    let mut events = sdl.event_pump()?;

    'running: loop {
        // `wait_event` panics on an SDL error; the timeout form returns `None`
        // instead, which for a loop with nothing to do is the same thing as a
        // quiet interval.
        let Some(event) = events.wait_event_timeout(EVENT_WAIT) else {
            continue;
        };

        match event {
            Event::Quit { .. }
            | Event::Window {
                win_event: WindowEvent::CloseRequested,
                ..
            }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => break 'running,
            _ => {}
        }
    }

    Ok(())
}
