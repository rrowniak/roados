//! OpenGL ES 3.1 context management.
//!
//! Owns the SDL3 window, the GL context, and the GLES function pointers.
//! The context is created with [`Context::new`] and cleaned up on drop.

use glow::HasContext;
use sdl3::video::GLProfile;

/// GL_VERSION constant (0x1F02).
const GL_VERSION: u32 = 0x1F02;
/// GL_RENDERER constant (0x1F01).
const GL_RENDERER: u32 = 0x1F01;

/// Parses a GL version string like "OpenGL ES 3.2 Mesa 26.0.8-1ubuntu0.3"
/// and returns true if the version is at least `major.minor`.
fn is_gles_version_at_least(version: &str, major: u32, minor: u32) -> bool {
    let Some(rest) = version.strip_prefix("OpenGL ES ") else {
        return false;
    };
    let Some(dot_pos) = rest.find('.') else {
        return false;
    };
    let Ok(v_major) = rest[..dot_pos].parse::<u32>() else {
        return false;
    };
    let after_dot = &rest[dot_pos + 1..];
    let space_pos = after_dot
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_dot.len());
    let Ok(v_minor) = after_dot[..space_pos].parse::<u32>() else {
        return false;
    };
    v_major > major || (v_major == major && v_minor >= minor)
}

/// An error that can occur when creating a [`Context`].
#[derive(Debug)]
pub enum ContextError {
    /// SDL3 reported an error.
    Sdl(sdl3::Error),
    /// Window creation failed with a Rust-side validation error (bad size or
    /// title) that never reached SDL, so no `sdl3::Error` exists to store.
    WindowBuild(String),
    /// The GL context is not OpenGL ES 3.1 or higher.
    VersionMismatch(String),
    /// The GL renderer string is empty.
    EmptyRenderer,
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextError::Sdl(e) => write!(f, "SDL error: {e}"),
            ContextError::WindowBuild(msg) => write!(f, "window build error: {msg}"),
            ContextError::VersionMismatch(msg) => write!(f, "GL version mismatch: {msg}"),
            ContextError::EmptyRenderer => write!(f, "GL renderer string is empty"),
        }
    }
}

impl std::error::Error for ContextError {}

/// An owned OpenGL ES 3.1 rendering context.
///
/// Holds the SDL3 window, the GL context, and the GLES function pointers.
/// Dropping the context deletes the GL context, destroys the window, and
/// quits SDL.
///
/// Field order matters: `gl` is dropped first (function pointers become
/// invalid), then `gl_context` (deletes the GL context), then `window`
/// (destroys the window), then `sdl` (quits SDL).
pub struct Context {
    /// GLES function pointers. Kept alive so the pointers remain valid.
    gl: glow::Context,
    /// The SDL3 GL context. Kept alive so the context remains current.
    #[allow(dead_code)]
    gl_context: sdl3::video::GLContext,
    window: sdl3::video::Window,
    sdl: sdl3::Sdl,
}

impl Context {
    /// Creates a new OpenGL ES 3.1 rendering context.
    ///
    /// Initializes SDL3 with video, events, and gamepad subsystems; creates
    /// a window with an OpenGL ES 3.1 context; loads GLES function pointers
    /// via `SDL_GL_GetProcAddress`; and verifies the context is active.
    ///
    /// # Errors
    ///
    /// Returns an error if SDL initialization fails, window creation fails,
    /// GL context creation fails, or the GLES version is not 3.1 or higher.
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, ContextError> {
        // SAFETY: SDL_Init is idempotent and thread-safe. The flags are the
        // standard SDL3 init flags for video, events, and gamepad.
        let init_flags = sdl3::sys::init::SDL_INIT_VIDEO
            | sdl3::sys::init::SDL_INIT_EVENTS
            | sdl3::sys::init::SDL_INIT_GAMEPAD;
        let result = unsafe { sdl3::sys::init::SDL_Init(init_flags) };
        if !result {
            return Err(ContextError::Sdl(sdl3::get_error()));
        }

        let sdl = sdl3::init().map_err(ContextError::Sdl)?;
        let video = sdl.video().map_err(ContextError::Sdl)?;

        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(GLProfile::GLES);
        gl_attr.set_context_version(3, 1);
        gl_attr.set_double_buffer(true);
        gl_attr.set_depth_size(0);

        let window = video
            .window(title, width, height)
            .position_centered()
            .opengl()
            .resizable()
            .build()
            .map_err(|e| match e {
                sdl3::video::WindowBuildError::SdlError(err) => ContextError::Sdl(err),
                other => ContextError::WindowBuild(other.to_string()),
            })?;

        let gl_context = window.gl_create_context().map_err(ContextError::Sdl)?;

        window
            .gl_make_current(&gl_context)
            .map_err(ContextError::Sdl)?;

        // SAFETY: The GL context is current on this thread. The loader
        // function calls SDL_GL_GetProcAddress, which is valid as long as
        // the GL context is current.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                video
                    .gl_get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const std::os::raw::c_void)
            })
        };

        // SAFETY: The GL context is current on this thread.
        let version = unsafe { gl.get_parameter_string(GL_VERSION) };
        if !is_gles_version_at_least(&version, 3, 1) {
            return Err(ContextError::VersionMismatch(version));
        }

        // SAFETY: The GL context is current on this thread.
        let renderer = unsafe { gl.get_parameter_string(GL_RENDERER) };
        if renderer.is_empty() {
            return Err(ContextError::EmptyRenderer);
        }

        Ok(Context {
            gl,
            gl_context,
            window,
            sdl,
        })
    }

    /// Swaps the front and back buffers.
    pub fn swap(&self) {
        self.window.gl_swap_window();
    }

    /// Returns a reference to the GLES function pointers.
    #[must_use]
    pub fn gl(&self) -> &glow::Context {
        &self.gl
    }

    /// Returns the window size in points.
    #[must_use]
    pub fn window_size(&self) -> (u32, u32) {
        self.window.size()
    }

    /// Returns a reference to the SDL3 context, for obtaining the event pump.
    #[must_use]
    pub fn sdl(&self) -> &sdl3::Sdl {
        &self.sdl
    }
}

#[cfg(test)]
mod tests {
    use super::is_gles_version_at_least;

    #[test]
    fn gles_version_parsing() {
        assert!(is_gles_version_at_least(
            "OpenGL ES 3.1 Mesa 26.0.8-1ubuntu0.3",
            3,
            1
        ));
        assert!(is_gles_version_at_least(
            "OpenGL ES 3.2 Mesa 26.0.8-1ubuntu0.3",
            3,
            1
        ));
        assert!(is_gles_version_at_least(
            "OpenGL ES 4.0 Mesa 26.0.8-1ubuntu0.3",
            3,
            1
        ));
        assert!(!is_gles_version_at_least(
            "OpenGL ES 3.0 Mesa 26.0.8-1ubuntu0.3",
            3,
            1
        ));
        assert!(!is_gles_version_at_least(
            "OpenGL ES 2.0 Mesa 26.0.8-1ubuntu0.3",
            3,
            1
        ));
        assert!(!is_gles_version_at_least(
            "OpenGL ES 3.1.1 Mesa 26.0.8-1ubuntu0.3",
            3,
            2
        ));
        assert!(!is_gles_version_at_least("not a version string", 3, 1));
        assert!(!is_gles_version_at_least("OpenGL ES", 3, 1));
        assert!(!is_gles_version_at_least("OpenGL ES 3", 3, 1));
        assert!(is_gles_version_at_least("OpenGL ES 3.1", 3, 1));
        assert!(is_gles_version_at_least("OpenGL ES 3.2", 3, 1));
        assert!(!is_gles_version_at_least("OpenGL ES 3.0", 3, 1));
    }
}
