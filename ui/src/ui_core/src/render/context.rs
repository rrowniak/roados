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
/// The multisample buffer count this pipeline asks for.
///
/// **It is a consequence of [`MULTISAMPLE_SAMPLES`], not a second decision**,
/// and it is named rather than written as a `1` at the call site because
/// `SDL_GL_MULTISAMPLEBUFFERS` **defaults to 0**: asking for samples without
/// asking for a buffer to hold them is a request a driver is entitled to honour
/// by ignoring, and the result is a context that reports
/// `GL_SAMPLE_BUFFERS = 0` and looks, on screen, exactly as it did before.
///
/// **One buffer and not two** because there is nothing to gain from a second
/// buffer here: the window is opaque and single-composited, and the driver's
/// own resolve would be handing back the same image a later pass would have to
/// composite again. **What would reverse it** is a caller that wanted to read
/// the multisample buffer's contents itself rather than the resolved image —
/// a deferred pass, or a `glBlitFramebuffer` to another target — which is an
/// FBO-shaped decision and belongs with the FBO, not with a window attribute.
///
/// **Measured 2026-10-02, and it is not inert:** with this attribute at 1 and
/// the sample count left at SDL's own default of 0, this driver reports
/// `GL_SAMPLE_BUFFERS = 1` and **`GL_SAMPLES = 2`** — a buffer with samples in
/// it is a request the driver fills in the best it can, and "best" here is 2x.
/// So the two attributes are not redundant, and neither is the free one: the
/// buffer count is what gets *any* antialiasing out of a driver that will not
/// grant 4, and [`MULTISAMPLE_SAMPLES`] is what asks for 4 rather than 2.
const MULTISAMPLE_BUFFERS: u8 = 1;

/// The samples per pixel this pipeline asks for on the **default** framebuffer.
///
/// This is the `SDL_GL_MULTISAMPLESAMPLES` attribute, and it is set on the
/// window before the GL context is created rather than on a framebuffer object,
/// because there is no FBO anywhere in this renderer and the operator chose the
/// default framebuffer on 2026-10-02.
///
/// **What it does.** It makes the hardware resolve every geometric edge from
/// four coverage samples instead of deciding it one whole pixel at a time.
/// Before it, the only antialiasing in the pipeline was
/// `if (dist > 0.0) { discard; }` in `render.rs`'s solid fragment shader — a
/// **hard** branch, and one that models an axis-aligned rounded rectangle, so it
/// reaches [`DrawCommand::Circle`](crate::paint::DrawCommand::Circle) only
/// because a circle is expanded to a rounded rect of twice its radius. The
/// gauge's band is a `Polygon`, a `Path` is `line_quad`s, and both hardcode
/// `radius: 0.0`, so they took the shader's plain-colour path and were quads of
/// hard pixels. A capture before this change measured the gauge arc's outer
/// edge as `18 18 18 18 18 187 187` — a step with nothing between it.
///
/// **It is a property of the whole pipeline, not of one widget.** It is a
/// window attribute, so it changes every edge on screen at once and it cannot
/// be turned on for a dial, for a key or for a chart without paying for all
/// three. That is the point: the thing it fixes is not any one widget's arc, it
/// is that *every* widget's geometry was being stair-stepped, and the list of
/// beneficiaries — the gauge, the slider, the progress bar, the toggle's pill,
/// the keyboard, and whatever draws next — is decided by the pipeline and not by
/// the caller. A widget that wanted sharper edges than 4x would have to ask for
/// its own framebuffer, and nothing in this renderer has one.
///
/// **What it costs.** Four samples of fragment work and a resolve blit per frame,
/// and **nothing measurable on this host**: an interleaved A/B over
/// `.ai/tools/fps-check.sh 12`, alternating the attribute's absence with its
/// presence in the same session, read **61.9 and 61.0 fps without** against
/// **61.8 and 61.7 fps with** — the same order as the spread between the two
/// builds, so the rate is not evidence either way and a frame-rate claim here
/// would be a rounding of noise. The cost is not free in principle and this host
/// does not show it: the pipeline's own submission was already measured at
/// ~3.9 ms of a 16 ms budget, and a fill-rate-bound target — a 1080p head unit
/// panel rather than this machine's Intel HD 530 — is where four samples per
/// pixel of the *whole* window would be paid for.
///
/// **Why four.** It is the count this pipeline's driver reports as available
/// (`GL_MAX_SAMPLES` read back as **16** from a live context on this host) and
/// the count it granted (`GL_SAMPLE_BUFFERS` **1**, `GL_SAMPLES` **4**, both
/// read back from a live context rather than assumed from the request), and it
/// is the step at which the eye stops seeing the stair-step before it stops
/// seeing blur. **What would reverse it**, in the order of how little it costs:
/// a measurement that 2x is indistinguishable from 4x at the pixel scale the
/// operator ships, which would halve the sample cost — and which a screenshot
/// cannot answer on this host, because 2x and 4x both smooth a stair-step and
/// only the pixel counts tell them apart; a `smoothstep` over `fwidth(dist)` in
/// the solid shader, which is free but reaches only rounded rectangles and
/// circles; and a driver that honours the request with a *lower* count, which is
/// not a reversal but a silent cap. **That cap is not hypothetical on this
/// driver:** asked for 0 samples with one buffer it answered 2 (see
/// [`MULTISAMPLE_BUFFERS`]), so `GL_SAMPLES` read back from a live context is
/// the only way to know what was granted, and there is no GL context in the test
/// harness — a driver that quietly granted fewer samples than were asked for
/// would be indistinguishable there from one that granted all four.
///
/// **What 4x does *not* fix.** It antialiases only edges that fall strictly
/// inside a pixel. An edge landing on a pixel boundary — the demo's progress
/// fill meeting its track at x = 784, the fill's cut at twelve o'clock at
/// x = 764 — resolves to full coverage on one side and none on the other, and
/// reads exactly as hard as it did before. That is the same reason an edge drawn
/// at an integer offset looks aliased in every renderer, and it is why a
/// capture of this change shows some edges smoothed and others untouched.
/// `pub(crate)` as of task 41, and for one caller: the backdrop capture probe's
/// error message interpolates this so it names **this host's** sample count
/// rather than a guess. The ES 3.1 rule the probe exists for turns on
/// `GL_SAMPLE_BUFFERS` for the read framebuffer, so a reader handed
/// `RenderError::Gl` needs the number that made the rule bite.
pub(crate) const MULTISAMPLE_SAMPLES: u8 = 4;

/// The depth buffer size this pipeline asks for on the **default** framebuffer.
///
/// This is the `SDL_GL_DEPTH_SIZE` attribute, and it is set on the
/// window before the GL context is created rather than on a framebuffer object,
/// because there is no FBO anywhere in this renderer and the operator chose the
/// default framebuffer on 2026-10-02.
///
/// **What it does.** It makes the default framebuffer include a depth buffer of
/// at least this many bits. The depth buffer belongs to the mesh pass
/// (tasks 35–37), and the 2D passes neither read it nor write it — see
/// `render.rs`'s `## Depth` section for the policy. The 2D vertex shaders write
/// `gl_Position.z = 0.0`, which is window depth **0.5**; a global depth test
/// with clear `1.0` and `GL_LESS` would pass the first layer and discard the
/// rest (`0.5 < 0.5` is false), deleting the UI. So the frame's resting state
/// is `GL_DEPTH_TEST` disabled and `GL_DEPTH_WRITEMASK` false, and the mesh
/// pass brackets its draws with both enabled.
///
/// **Why 24.** `SDL_GL_DEPTH_SIZE` is a **minimum** request, so the driver may
/// grant more and the only way to know is to read `GL_DEPTH_BITS` back from a
/// live context — exactly the discipline `MULTISAMPLE_SAMPLES` already states
/// for `GL_SAMPLES`.
///
/// - **16** is SDL's own default and the minimum GLES 3.1 requires of a depth
///   buffer. It is rejected: over a car-sized scene with a near plane a few
///   centimetres out, 16 bits quantises `z` coarsely enough that two coplanar
///   surfaces — a door skin against a wing, which is what a car body is made of
///   — z-fight.
/// - **32** is not reachable as `GL_DEPTH_COMPONENT32` in the ES 3.1 header
///   this build compiles against (it is a desktop-GL/extension sized format
///   there) and is not needed: with a near/far ratio task 36 will choose in the
///   range of hundreds, 24 bits leaves far more precision than the raster can
///   show. It also costs the most bandwidth of the three.
/// - **24** is `GL_DEPTH_COMPONENT24`, the sized format ES 3.1 defines for
///   exactly this, and the smallest count that removes the coplanar z-fighting
///   this sequence is for. **What would reverse it** is a measurement that 16 is
///   indistinguishable at the panel resolution the operator ships — the same
///   "screenshot cannot answer it" limit `MULTISAMPLE_SAMPLES` already records,
///   because z-fighting is a *depth* artefact and a capture photographs whatever
///   won.
///
/// **The MSAA interaction is the real cost, and it is not the bit count.** Depth
/// on a multisample default framebuffer is stored **per sample**, so the depth
/// allocation is `samples × bytes`. At this demo's window (1280 × 1020) with 4
/// samples: **16-bit is 10.4 MB, 24-bit is 15.7 MB, 32-bit is 20.9 MB**,
/// against 5.2 MB of colour today. And there is a second interaction with the
/// same shape as the one `MULTISAMPLE_BUFFERS`'s doc records: **a driver asked
/// for four samples *and* a depth buffer may grant fewer samples**, which is why
/// `render.rs` requirement 11 reports both numbers and requirement 12 makes a
/// dropped sample count a finding rather than a note.
///
/// **Measured 2026-10-06.** On this host's driver (Mesa 26.0.8, Intel HD 530):
/// `GL_DEPTH_BITS` granted **24**, `GL_SAMPLES` granted **4** — both read back
/// from a live context after `Context::new`. The request was honoured.
const DEPTH_BITS: u8 = 24;

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
        gl_attr.set_depth_size(DEPTH_BITS);
        // Both attributes, before the window: `MULTISAMPLE_BUFFERS` defaults to
        // 0, so a request for samples with no buffer to put them in is one a
        // driver may honour by ignoring. See the two constants for why four.
        gl_attr.set_multisample_buffers(MULTISAMPLE_BUFFERS);
        gl_attr.set_multisample_samples(MULTISAMPLE_SAMPLES);

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
    use super::{is_gles_version_at_least, DEPTH_BITS, MULTISAMPLE_BUFFERS, MULTISAMPLE_SAMPLES};

    /// The multisample request this pipeline makes, held as a number.
    ///
    /// **This is a pin on the request, not on what the driver did with it, and
    /// the difference is the whole limit of the test harness here.** There is no
    /// GL context in `cargo test`, so nothing in this module can read
    /// `GL_SAMPLES` back, and the acceptance for
    /// [`MULTISAMPLE_SAMPLES`](super::MULTISAMPLE_SAMPLES) is a capture and a
    /// frame rate rather than a unit test.
    ///
    /// What it *can* catch is the request becoming a no-op, which is the failure
    /// mode that is invisible on screen: a sample count of 0 or 1 asks for no
    /// antialiasing and looks, in a capture, exactly like a pipeline that never
    /// asked. `MULTISAMPLE_BUFFERS` is here for the same reason — **0 is SDL's
    /// own default**, so a constant that fell to 0 would leave
    /// `set_multisample_samples` called with a real count into a context with no
    /// multisample buffer in it.
    ///
    /// What it cannot catch is the attribute call being deleted, because the
    /// call takes `&GLAttr` and constructing one needs an initialised video
    /// subsystem, which needs a display. A test seam for it would be an
    /// abstraction built for its second use, which does not exist.
    #[test]
    fn the_multisample_request_is_four_samples_in_one_buffer() {
        assert_eq!(
            MULTISAMPLE_BUFFERS, 1,
            "SDL_GL_MULTISAMPLEBUFFERS defaults to 0, and a driver may honour a \
             sample count by ignoring it when there is no buffer to hold it"
        );
        assert_eq!(
            MULTISAMPLE_SAMPLES, 4,
            "the sample count is a decision, not a default: 0 and 1 are no-ops and \
             8 is a count this pipeline's driver did not grant"
        );
        assert_eq!(
            MULTISAMPLE_SAMPLES.count_ones(),
            1,
            "a multisample count is a power of two; a driver rounds anything else down \
             and the request stops meaning what it says"
        );
    }

    /// The depth buffer request this pipeline makes, held as a number.
    ///
    /// **This is a pin on the request, not on what the driver did with it, and
    /// the difference is the whole limit of the test harness here.** There is no
    /// GL context in `cargo test`, so nothing in this module can read
    /// `GL_DEPTH_BITS` back, and the acceptance for
    /// [`DEPTH_BITS`](super::DEPTH_BITS) is a capture and a frame rate rather
    /// than a unit test.
    ///
    /// What it *can* catch is the request becoming a no-op, which is the failure
    /// mode that is invisible on screen: a depth size of 0 asks for no depth
    /// buffer and looks, in a capture, exactly like a pipeline that never asked.
    /// `DEPTH_BITS` is here for the same reason — **0 is SDL's own default
    /// before this change**, so a constant that fell to 0 would leave
    /// `set_depth_size` called with a real count into a context with no depth
    /// buffer in it.
    ///
    /// What it cannot catch is the attribute call being deleted, because the
    /// call takes `&GLAttr` and constructing one needs an initialised video
    /// subsystem, which needs a display. A test seam for it would be an
    /// abstraction built for its second use, which does not exist.
    #[test]
    fn the_depth_request_is_twenty_four_bits() {
        assert_eq!(
            DEPTH_BITS, 24,
            "SDL_GL_DEPTH_SIZE defaults to 16, and 16 bits is not enough for a \
             car-sized scene: coplanar surfaces z-fight"
        );
        assert!(
            matches!(DEPTH_BITS, 16 | 24 | 32),
            "the depth size is a decision, not a default: 0 is a no-op and other \
             values are not sized formats in ES 3.1"
        );
    }

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
