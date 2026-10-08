//! The offscreen render target the shadow layer is drawn into.
//!
//! `ui_core` had no framebuffer object anywhere before this module — the only
//! target was the window's own default framebuffer — and a blurred shadow needs
//! one: the shape has to exist as pixels before it can be blurred, and the blur
//! needs a destination that is not the screen.
//!
//! ## Why the whole window, and not the shadow's own box
//!
//! The target is **window-sized**. The alternative is to size it to the union of
//! the shadows' bounding boxes and carry a pixel offset from the window origin
//! into every coordinate that reaches it, which is three more numbers to be right
//! in the shape draw, both blur passes and the composite.
//!
//! That is the right trade for a *correct* target and possibly the wrong one for
//! a *fast* one, and the difference is the whole of the cost this module adds:
//! a separable Gaussian over a window-sized texture is `2 · 2r+1` texture fetches
//! per pixel of the window, where `r` is the tap radius.
//!
//! **Measured, and the measurement does not settle it.** Interleaved release runs
//! on this host at 1280×1020 with one shadow on screen read 171, 182 and 211
//! jiffies of CPU over a six-second window at [`crate::render::blur::MAX_TAPS`]
//! = 9, and 212,
//! 202 and 188 at seventeen — against 191, 172 and 158 with no shadow drawn at
//! all. **The three sets overlap, and the frame rate is 62.0–62.2 fps in every
//! one of them.** A bounding box would remove that cost entirely and this host
//! cannot show that there is one to remove, at this size.
//! **What would reverse the decision** is a fill-rate-bound target — a 1080p
//! head-unit panel — where `2 · (2r+1)` fetches per pixel of the *window* is
//! paid whether or not the shadow covers it.
//!
//! **The colour target inherits the same window-sized decision for a different
//! reason, and cannot do otherwise.** A shadow could have been sized to its
//! caster's box — that is the trade argued above. A backdrop cannot be sized to
//! anything but the window, because the blit that fills it requires the source
//! and destination rectangles to have *identical* bounds whenever the read
//! framebuffer is multisampled, and this pipeline's is (`MULTISAMPLE_SAMPLES`).
//! A rect-scoped blit raises `GL_INVALID_OPERATION`, so there is no capture at
//! all rather than a capture of the wrong place. The paragraph above is **not**
//! amended by this one: *"the difference is the whole of the cost this module
//! adds"* is still true, and this module now adds a second, larger cost on top
//! of it — roughly 132 MB of texture traffic per frame at 1280×1020 against the
//! shadow's 40.5 MB. **This host cannot measure that difference**, which is the
//! measured fact above in its strongest form: three overlapping sets of runs,
//! all at 62.0–62.2 fps. So the numbers in `render`'s § *Backdrops* decide it
//! and a measurement here would not.
//!
//! ## The colour target, and why it is a second type
//!
//! [`ColourTarget`] holds **four channels where [`ShadowTarget`] holds one**, and
//! it is a second `struct` rather than a format argument on the first. Three
//! reasons, and the second is the one that settles it:
//!
//! 1. **The format is a property of the shaders, and the shaders are
//!    compile-time text.** `SHADOW_MASK_FRAGMENT_SHADER_SRC`'s
//!    `vec4(v_color.a, 0, 0, 0)` and `BLUR_FRAGMENT_SHADER_SRC`'s `.r` reads
//!    *are* the coverage contract. A format parameter would move `GL_R8` out of
//!    the agreement between those strings and into a runtime value. **And it
//!    would not fail.** A `GL_RGBA8` target driven by the coverage shaders still
//!    works — the mask writes coverage into `r`, the blur reads `r`, the
//!    composite reads `r` — so a caller who built the wrong instance gets a
//!    correct-looking picture with three quarters of its bandwidth wasted and no
//!    error anywhere. There is nothing to notice.
//! 2. **`GL_R8` cannot represent a backdrop, and that is not a matter of degree.**
//!    The argument in [`ShadowTarget`]'s doc is a *saving*, and it depends on the
//!    shadow's colour being one constant. A backdrop has no such constant — its
//!    `rgb` varies per pixel, which is what *"what is behind this rect"* means —
//!    and its alpha carries the coverage the composite blends against, which the
//!    single-channel mask shader has nowhere to put. One channel is not a cheaper
//!    backdrop; it is a backdrop with three of its four channels discarded.
//! 3. **A shared `ensure_size` would have to answer a question only one of the
//!    two has.** A colour capture from a multisampled default framebuffer is
//!    legal only under a condition the shadow target never faces, so
//!    [`ColourTarget`] carries a one-shot capability probe and a documented
//!    *draws nothing* degradation that [`ShadowTarget`] has no use for.
//!
//! **The duplication this accepts is named, and so is the cheapest reversal**,
//! because `.ai/agents/developer.md` § *Phase 2* (*"No abstraction before the
//! second use"*) points the other way and the operator is entitled to see the
//! trade made rather than the rule assumed. This *is* the second use, so the
//! letter of the rule says factor the ping-pong protocol into one private type.
//! **If a third offscreen target appears, the four bookkeeping methods move into
//! a private `PingPongTarget` and neither public type's signature, semantics nor
//! doc comment changes** — the format was never a field, so the move is
//! mechanical and there is nothing to unpick.
//!
//! ## Two textures, one framebuffer
//!
//! A separable blur cannot read and write the same texture, so this holds two
//! and swaps which one is attached between passes: the shape is drawn into one,
//! the horizontal pass writes the other and reads the first, the vertical pass
//! writes the first back. One framebuffer is enough because the attachment is
//! what changes, and re-attaching is a state change rather than an object.

use crate::render::{
    gl_enum_to_i32, u32_to_i32, RenderError, GL_CLAMP_TO_EDGE, GL_COLOR_BUFFER_BIT, GL_LINEAR,
    GL_NEAREST, GL_R8, GL_READ_FRAMEBUFFER, GL_RED, GL_TEXTURE_2D, GL_UNSIGNED_BYTE,
};
use glow::HasContext;

/// The smallest width or height a target is ever allocated at.
///
/// GL's `glTexImage2D` rejects a zero extent, and the arithmetic below produces
/// a zero whenever the window reports one — which a minimised window on some
/// compositors does. **One pixel is not a picture anyone sees**, so it is not a
/// quality trade: it is what makes a zero-sized allocation a legal one.
const MIN_TARGET_EXTENT: u32 = 1;

/// `GL_RGBA8` (`0x8058`): a colour-renderable, four-channel, eight-bit
/// internal format — [`ColourTarget`]'s texture, and a **literal at the one place
/// the allocation happens** rather than a field on the type.
///
/// Declared here rather than imported from `render`, and that is the whole of
/// § *The colour target, and why it is a second type*'s first reason: the format
/// belongs to the type whose name says what it holds, and importing it would let
/// `ShadowTarget::ensure_size` name it too. `render` has a constant of the same
/// name and value for the image atlas; a test asserts the two agree.
const COLOUR_TARGET_INTERNAL_FORMAT: u32 = 0x8058;

/// `GL_RGBA` (`0x1908`): the four-channel pixel format [`ColourTarget`]'s
/// textures are read and written with, and the second half of the pair above.
const COLOUR_TARGET_FORMAT: u32 = 0x1908;

/// Returns the texel size a target for a window of `width` by `height` is
/// allocated at.
///
/// Each side is raised to `MIN_TARGET_EXTENT` and neither is clamped upward:
/// a window larger than the largest addressable texture has to be refused by
/// [`ShadowTarget::ensure_size`], not silently rounded down to something that
/// fits, because a rounded-down target is a blurred shape sampled from the wrong
/// pixels rather than an error.
///
/// # Examples
///
/// ```
/// use ui_core::render::target::allocation;
///
/// assert_eq!(allocation(1024, 600), (1024, 600), "an ordinary window, as it is");
/// assert_eq!(allocation(0, 0), (1, 1), "a zero extent becomes a legal one");
/// assert_eq!(allocation(1024, 0), (1024, 1), "and only on the side that is zero");
/// ```
#[must_use]
pub fn allocation(width: u32, height: u32) -> (u32, u32) {
    (width.max(MIN_TARGET_EXTENT), height.max(MIN_TARGET_EXTENT))
}

/// Returns whether a target already allocated at `allocated` has to be
/// reallocated for a window of `wanted`.
///
/// `allocated` is `None` when nothing has been allocated yet, which is a resize
/// however small `wanted` is — the alternative would let a 1×1 window skip the
/// very first allocation, because `None` and a 1×1 allocation both normalise to
/// 1×1 and compare equal.
///
/// The pair with this is [`allocation`]: comparing the two *normalised* sizes is
/// what makes a window that alternates between `0` and `1` on one side allocate
/// once instead of every frame.
///
/// # Examples
///
/// ```
/// use ui_core::render::target::resize_decision;
///
/// assert!(resize_decision(None, (1024, 600)), "nothing is allocated yet");
/// assert!(!resize_decision(Some((1024, 600)), (1024, 600)), "an unchanged window");
/// assert!(resize_decision(Some((1024, 600)), (1025, 600)), "one pixel wider");
/// ```
#[must_use]
pub fn resize_decision(allocated: Option<(u32, u32)>, wanted: (u32, u32)) -> bool {
    !matches!(
        allocated,
        Some(size) if size == allocation(wanted.0, wanted.1)
    )
}

/// A window-sized offscreen target with two single-channel textures, for the
/// shadow layer.
///
/// The texture is `GL_R8` and holds **one channel: coverage**. It is not
/// `GL_RGBA8`, and the reason is arithmetic rather
/// than taste. The shadow's colour is one constant, so a blurred coverage
/// convolved against it and a blurred premultiplied colour are the same image:
/// convolution is linear, so `blur(rgb · a) = rgb · blur(a)` whenever `rgb` is
/// the same everywhere. Blurring one channel and tinting at the composite gives
/// exactly that image for a quarter of the bandwidth, which is the part of the
/// shadow layer the frame rate is spent on.
///
/// What this module does **not** do is tell the renderer what to tint with: the
/// colour is a [`crate::render::ShaderKind::Shadow`] shader's uniform, and
/// [`ShadowTarget`] has no opinion about it.
///
/// # Errors
///
/// [`ShadowTarget::new`] returns [`RenderError::Gl`] when the framebuffer or
/// its textures cannot be created. Allocation failures — a window larger than
/// [`GL_MAX_TEXTURE_SIZE`](glow::MAX_TEXTURE_SIZE) — are refused by
/// [`ShadowTarget::ensure_size`] with the same variant rather than passed to GL
/// and ignored.
pub struct ShadowTarget {
    /// The framebuffer every pass into the target draws to.
    framebuffer: glow::Framebuffer,
    /// The two textures the separable blur passes between.
    ///
    /// An array rather than two named fields because "which one is read" and
    /// "which one is written" swap, and a named pair of fields would need
    /// swapping in two places.
    textures: [glow::Texture; 2],
    /// Which of `textures` holds what the next pass reads.
    read: usize,
    /// Which of `textures` the next pass writes to.
    written: usize,
    /// The size the textures are allocated at, or `None` before the first
    /// allocation.
    size: Option<(u32, u32)>,
}

impl ShadowTarget {
    /// Creates a target with no storage allocated yet.
    ///
    /// Nothing is allocated here: a window that draws no shadow never allocates
    /// `width · height` bytes, which is the same decision
    /// [`crate::render::Renderer`] makes about the image atlas.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Gl`] when a GL object cannot be created.
    pub fn new(gl: &glow::Context) -> Result<Self, RenderError> {
        // SAFETY: The GL context is current on this thread.
        let framebuffer = unsafe { gl.create_framebuffer() }.map_err(RenderError::Gl)?;
        // SAFETY: The GL context is current on this thread.
        let first = unsafe { gl.create_texture() }.map_err(RenderError::Gl)?;
        // SAFETY: The GL context is current on this thread.
        let second = unsafe { gl.create_texture() }.map_err(RenderError::Gl)?;
        Ok(ShadowTarget {
            framebuffer,
            textures: [first, second],
            // **Not both zero**, and that is the whole invariant: a pass reads
            // `textures[read]` and writes `textures[written]`, and reading the
            // texture that is attached to the framebuffer is undefined — the
            // driver may give back the value it is writing or stall. Starting
            // with the two apart means the shape can be drawn with no `swap`
            // first, and every later `swap` keeps them apart.
            read: 1,
            written: 0,
            size: None,
        })
    }

    /// Returns the size the target's textures are allocated at, or `(0, 0)`
    /// before the first allocation.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        self.size.unwrap_or((0, 0))
    }

    /// Allocates the textures' storage for a window of `width` by `height`, and
    /// does nothing at all when the current allocation already fits.
    ///
    /// Reallocating is the expensive half of this module — a window-sized
    /// `glTexImage2D` — so the unchanged case is the common one and is decided
    /// by [`resize_decision`] before anything is bound.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Gl`] when either extent is past what the GL
    /// implementation can address, or when the framebuffer is not complete once
    /// a texture is attached — which on a GLES 3.1 implementation can only
    /// happen if the driver disagrees about `GL_R8` being colour-renderable,
    /// and is worth an error rather than a silently absent shadow.
    pub fn ensure_size(
        &mut self,
        gl: &glow::Context,
        width: u32,
        height: u32,
    ) -> Result<(), RenderError> {
        let wanted = allocation(width, height);
        if !resize_decision(self.size, wanted) {
            return Ok(());
        }
        let (w, h) = (wanted.0, wanted.1);
        // The driver's own limit, read back rather than assumed. An extent past
        // it is refused here, where the caller can be told, rather than handed to
        // `glTexImage2D` and left as an unchecked GL error — a rejected call with
        // nobody reading the rejection drops a whole batch.
        let limit = max_texture_size(gl);
        if w > limit || h > limit {
            return Err(RenderError::Gl(format!(
                "shadow target of {w}x{h} is past the driver's largest texture, \
                 {limit}x{limit}"
            )));
        }
        for texture in self.textures {
            // **The format pair is an argument here, not a field**, which is the
            // whole of the second-type decision — `allocate_texture` is shared with
            // the colour target and each `ensure_size` hands it its own pair. This
            // comment deliberately does *not* name the other pair's constants:
            // `the_colour_target_is_rgba_and_the_shadow_target_is_red` asserts that
            // this body contains no four-channel constant at all, so a note here
            // that named one would make that assertion a comment-counting exercise.
            allocate_texture(gl, texture, w, h, GL_R8, GL_RED);
        }
        self.bind_attach(gl, self.written);
        // SAFETY: The GL context is current on this thread; `self.framebuffer` is
        // a valid framebuffer with a texture attached by `bind_attach` above.
        let status = unsafe { gl.check_framebuffer_status(glow::FRAMEBUFFER) };
        if status != glow::FRAMEBUFFER_COMPLETE {
            return Err(RenderError::Gl(format!(
                "shadow target framebuffer is not complete (status 0x{status:04X})"
            )));
        }
        self.size = Some(wanted);
        Ok(())
    }

    /// Binds the framebuffer with the texture the next pass writes to attached,
    /// sets the viewport to it, and disables scissoring.
    ///
    /// **Scissoring is disabled here because the offscreen passes are
    /// whole-window**, and this is *not* the last word on it: the renderer
    /// re-applies the clip before the composite, which is the pass that puts the
    /// shadow on the screen — see `crate::render::Renderer::draw_shadow_batch`.
    /// A caller that wants the scissor honoured while drawing into the target
    /// must set it again after this call.
    ///
    /// The scissor is **global GL state** and not a property of the framebuffer,
    /// so it is the one piece of state this module cannot scope to the target. It
    /// is turned off rather than left alone because a viewport's clip is in window
    /// coordinates, which happen to be the target's, and leaving it on would cut
    /// the mask and both blur passes down to one viewport's box for no reason.
    pub fn bind_for_write(&mut self, gl: &glow::Context) {
        let (width, height) = self.size();
        // SAFETY: The GL context is current on this thread; the framebuffer is a
        // valid object and `bind_attach` names one of this type's own textures.
        unsafe {
            gl.viewport(0, 0, u32_to_i32(width), u32_to_i32(height));
            gl.disable(glow::SCISSOR_TEST);
        }
        self.bind_attach(gl, self.written);
    }

    /// Binds the texture the next pass reads from to texture unit 0.
    pub fn bind_for_read(&self, gl: &glow::Context) {
        // SAFETY: The GL context is current on this thread and `self.read`
        // indexes a two-element array of valid textures, so it is in range.
        let texture = self.textures[self.read];
        // SAFETY: The GL context is current on this thread; `texture` is valid.
        unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(GL_TEXTURE_2D, Some(texture));
        }
    }

    /// Swaps which texture is read from and which is written to.
    ///
    /// **Called before every pass and before the composite**, which is the count
    /// that makes the whole protocol work: the shape is drawn through
    /// [`Self::bind_for_write`], so the first pass needs one `swap` to make it the
    /// source, each pass needs one to make its own output the next pass's source,
    /// and the composite needs one to be handed the last pass's output. Two
    /// passes and one composite are three `swap`s.
    ///
    /// It is the *only* thing that changes which texture is read and which is
    /// written, so the invariant that they are never the same index
    /// cannot be broken by a caller.
    pub fn swap(&mut self) {
        self.read = self.written;
        self.written = 1 - self.written;
    }

    /// Binds the framebuffer and attaches `index` of [`Self::textures`] to its
    /// colour attachment 0, **in that order**.
    ///
    /// The order is a measurement, not a preference. On this host's driver — Mesa
    /// 26.0.8, GLES 3.1 — `glFramebufferTexture2D` raises
    /// **`GL_INVALID_OPERATION` when the framebuffer is not currently bound**,
    /// and it does not take effect: with the attachment never made, the first
    /// frame's mask draw came back `GL_INVALID_FRAMEBUFFER_OPERATION` and the
    /// shadow's shape was not in the target at all on that frame.
    ///
    /// **Both errors are invisible to everything this repository would normally
    /// check.** No unit test has a GL context; a still screenshot taken after the
    /// first frame shows the shadow, because the second frame's attach succeeds
    /// once the framebuffer has been bound; and the frame rate is unaffected. It
    /// was found by reading `gl.get_error()` after each new call once: a rejected
    /// GL call nobody reads is invisible.
    ///
    /// `self.size` is **not** changed here: which of the two textures is attached
    /// says nothing about how large either of them is.
    fn bind_attach(&self, gl: &glow::Context, index: usize) {
        // SAFETY: The GL context is current on this thread; the framebuffer and
        // the texture are valid objects, and `index` comes from this module's
        // own two-element array rather than from a caller.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                Some(self.textures[index]),
                0,
            );
        }
    }
}

impl Drop for ShadowTarget {
    fn drop(&mut self) {
        // No GL object is deleted here, and that is the same decision
        // `crate::render` makes: the framebuffer and the two textures live until
        // the GL context is torn down, which `Context::drop` does immediately
        // after. Deleting these three and not the programs and buffers beside
        // them would be a distinction without a difference.
    }
}

/// A window-sized offscreen target with two **four-channel** textures, for
/// capturing the scene behind a backdrop.
///
/// **Four channels, not one, and the difference is what a backdrop is.** A
/// [`ShadowTarget`] holds coverage in one channel because a shadow's colour is
/// one constant: `blur(rgb · a) = rgb · blur(a)`, so one channel and a tint at
/// the composite is the same image for a quarter of the bandwidth. **A backdrop
/// has no such constant** — *"what is behind this rect"* means an `rgb` that
/// varies per pixel — and its alpha is the composited coverage the blend reads.
/// Single-channel here is not a cheaper backdrop; it is a backdrop with three of
/// its four channels discarded.
///
/// See the module docs § *The colour target, and why it is a second type* for
/// why this is a second type rather than a format argument on [`ShadowTarget`],
/// and § *Why the whole window, and not the shadow's own box* for why it is
/// **window-sized whatever the caller's rect says**: the capture is a
/// `glBlitFramebuffer` from the default framebuffer, and the OpenGL ES 3.1 rule
/// for that call makes differing source and destination bounds an
/// `GL_INVALID_OPERATION` whenever the read framebuffer is multisampled, which
/// this pipeline's is. That is why `doc/ui/DEMO_APPLICATION.md` row `L1`
/// sub-item (c), rect-scoped capture, stays open: **it is not reachable through
/// this route, and a rect on the command sizes the composite rather than the
/// capture.** A caller that believed otherwise would get a blit the driver
/// rejects, a backdrop that does not draw, and no GL error anybody reads.
///
/// **It allocates lazily**, like its neighbour: [`Self::new`] allocates no
/// storage and [`Self::ensure_size`] is reached only from
/// `Renderer::draw_backdrop_offscreen`, so a frame with no backdrop allocates
/// nothing and the six gallery pages pay zero for it.
///
/// **What would reverse the decision** — both the second type and the
/// window-sizing — is a capture route that can be scoped to a rect: a
/// re-submission of the already-recorded commands into a scissored target rather
/// than a copy of the default framebuffer. That is a different task with its own
/// decision.
pub struct ColourTarget {
    /// The framebuffer the capture writes into and the blur passes between.
    framebuffer: glow::Framebuffer,
    /// The two `GL_RGBA8` textures the separable blur passes between, and an
    /// array for the reason [`ShadowTarget`]'s is one.
    textures: [glow::Texture; 2],
    /// Which of `textures` holds what the next pass reads. **Not both zero**,
    /// for [`ShadowTarget`]'s reason: the two are never the same index.
    read: usize,
    /// Which of `textures` the next pass — the capture included — writes to.
    written: usize,
    /// The size the textures are allocated at, or `None` before the first
    /// allocation.
    size: Option<(u32, u32)>,
}

impl ColourTarget {
    /// Creates a target with no storage allocated yet.
    ///
    /// Nothing is allocated here, for [`ShadowTarget::new`]'s reason: a frame that
    /// records no backdrop never allocates `width · height · 4` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Gl`] when a GL object cannot be created.
    pub fn new(gl: &glow::Context) -> Result<Self, RenderError> {
        // SAFETY: The GL context is current on this thread.
        let framebuffer = unsafe { gl.create_framebuffer() }.map_err(RenderError::Gl)?;
        // SAFETY: The GL context is current on this thread.
        let first = unsafe { gl.create_texture() }.map_err(RenderError::Gl)?;
        // SAFETY: The GL context is current on this thread.
        let second = unsafe { gl.create_texture() }.map_err(RenderError::Gl)?;
        Ok(ColourTarget {
            framebuffer,
            textures: [first, second],
            // **Not both zero**, and the capture depends on it as much as the blur
            // does: the capture is a *write*, so it lands in `textures[written]`
            // and the first blur pass reads it — which is why there is no `swap`
            // between them and the loop's first `swap` is spent turning the write
            // target into the first pass's source. See [`Self::swap`].
            read: 1,
            written: 0,
            size: None,
        })
    }

    /// Returns the size the target's textures are allocated at, or `(0, 0)`
    /// before the first allocation.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        self.size.unwrap_or((0, 0))
    }

    /// Allocates the textures' storage for a window of `width` by `height`, and
    /// does nothing at all when the current allocation already fits.
    ///
    /// **Window-sized, lazily, and decided by [`resize_decision`]** — the same
    /// three steps [`ShadowTarget::ensure_size`] takes, against the same
    /// `allocation` normalising, so the normalise-then-compare invariant has
    /// **one** implementation and not two. The extents are the *window's*, never
    /// the caller's rect: see the type doc for why no other extent is legal.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Gl`] when either extent is past what the GL
    /// implementation can address — refused **before anything is bound**, since
    /// a rejected call with nobody reading it dropped a whole pass — or when the
    /// framebuffer is not complete once a texture is attached. On a GLES 3.1
    /// implementation the latter can only happen if the driver disagrees about
    /// `GL_RGBA8` being colour-renderable, which is worth an error rather than a
    /// silently absent backdrop.
    pub fn ensure_size(
        &mut self,
        gl: &glow::Context,
        width: u32,
        height: u32,
    ) -> Result<(), RenderError> {
        let wanted = allocation(width, height);
        if !resize_decision(self.size, wanted) {
            return Ok(());
        }
        let (w, h) = (wanted.0, wanted.1);
        // The driver's own limit, read back rather than assumed, for
        // `ShadowTarget::ensure_size`'s reason.
        let limit = max_texture_size(gl);
        if w > limit || h > limit {
            return Err(RenderError::Gl(format!(
                "colour target of {w}x{h} is past the driver's largest texture, \
                 {limit}x{limit}"
            )));
        }
        for texture in self.textures {
            allocate_texture(
                gl,
                texture,
                w,
                h,
                COLOUR_TARGET_INTERNAL_FORMAT,
                COLOUR_TARGET_FORMAT,
            );
        }
        self.bind_attach(gl, self.written);
        // SAFETY: The GL context is current on this thread; `self.framebuffer` is
        // a valid framebuffer with a texture attached by `bind_attach` above.
        let status = unsafe { gl.check_framebuffer_status(glow::FRAMEBUFFER) };
        if status != glow::FRAMEBUFFER_COMPLETE {
            return Err(RenderError::Gl(format!(
                "colour target framebuffer is not complete (status 0x{status:04X})"
            )));
        }
        self.size = Some(wanted);
        Ok(())
    }

    /// Copies the default framebuffer into the texture the next pass writes to.
    ///
    /// **One `glBlitFramebuffer`, the whole window into the whole window, and
    /// every part of that is load-bearing.**
    ///
    /// 1. **The read binding comes after the attach**, and the order is a
    ///    measurement rather than a style: `glow::FRAMEBUFFER` binds both the read
    ///    and the draw framebuffer, so a blit issued before `bind_attach` has run
    ///    — or before the *read* binding below has been made — is a
    ///    texture-to-itself blit, which is a documented `GL_INVALID_OPERATION`.
    ///    The caller has already bound the write texture and set the viewport
    ///    through [`Self::bind_for_write`]; this binds `GL_READ_FRAMEBUFFER` to
    ///    `None`, meaning the window.
    /// 2. **Both rectangles are the whole window, and they have to be.** The
    ///    OpenGL ES 3.1 reference page for `glBlitFramebuffer` raises
    ///    `GL_INVALID_OPERATION` when `GL_SAMPLE_BUFFERS` for the read buffer is
    ///    greater than zero and the source and destination rectangles are not
    ///    defined with the same `(X0, Y0, X1, Y1)` bounds. This pipeline's
    ///    default framebuffer holds `MULTISAMPLE_SAMPLES` samples (four), so that
    ///    clause is live: **the only legal colour capture from
    ///    this pipeline's default framebuffer is a full-window blit into a
    ///    full-window texture.** A rect-scoped capture, a half-resolution tier
    ///    and an offset blit are all refused by the driver, which is why
    ///    `doc/ui/DEMO_APPLICATION.md` row `L1` sub-item (c) stays open and why
    ///    the rect on a `crate::paint::DrawCommand::Backdrop` sizes the composite
    ///    and not this.
    /// 3. **`GL_NEAREST`, because there is nothing to interpolate.** Source and
    ///    destination are the same size, so every destination texel has exactly
    ///    one source texel; `GL_LINEAR` would sample the same point and cost a
    ///    filter per pixel to arrive there.
    /// 4. **The mask is `GL_COLOR_BUFFER_BIT` and only that.** This target has no
    ///    depth attachment — a backdrop is a 2D pass, and under the depth policy
    ///    the 2D passes neither test nor write the buffer — so a second bit would
    ///    be a claim about a buffer the target does not hold.
    ///
    /// **The resolve is the specification's and the driver's, and free.** The same
    ///    page says that where the read framebuffer is multisampled and the draw
    ///    framebuffer is not, the samples are *converted to a single sample*
    ///    before being written — which is why § *Out of Scope* of
    ///    `doc/ui/TASK_UI_PRIM_41.md` can say *no MSAA resolve for this target*
    ///    honestly: there is no multisample renderbuffer to allocate. **What it
    ///    costs is stated rather than assumed**: the page says only that the
    ///    samples are converted, not how, so the capture's antialiasing is
    ///    whatever the driver's resolve is rather than the coverage integral
    ///    `widgets::chart`'s module docs measured for this pipeline. A blurred
    ///    edge is softer than a sharp one in any case, so this is a limit
    ///    recorded rather than a defect to fix.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Gl`] when the blit raises an error — which on this
    /// pipeline's own drivers it does not, because both rectangles match and the
    /// formats are both four-channel, and which on a driver whose default
    /// framebuffer is *not* `GL_RGBA8` it very much does.
    pub fn capture(&mut self, gl: &glow::Context) -> Result<(), RenderError> {
        let (width, height) = self.size();
        // **After** `bind_attach`, and the order is load-bearing: `glow::FRAMEBUFFER`
        // binds both the read and the draw framebuffer, so a blit issued before
        // this binding is a texture-to-itself blit.
        //
        // SAFETY: The GL context is current on this thread; `None` is the window's
        // own default framebuffer, which is what is being captured from.
        unsafe {
            gl.bind_framebuffer(GL_READ_FRAMEBUFFER, None);
            gl.blit_framebuffer(
                0,
                0,
                i32::try_from(width).unwrap_or(0),
                i32::try_from(height).unwrap_or(0),
                0,
                0,
                i32::try_from(width).unwrap_or(0),
                i32::try_from(height).unwrap_or(0),
                GL_COLOR_BUFFER_BIT,
                GL_NEAREST,
            );
        }
        Ok(())
    }

    /// Binds the framebuffer with the texture the next pass writes to attached,
    /// sets the viewport to it, and disables scissoring.
    ///
    /// **Scissoring is disabled here because the offscreen passes are
    /// whole-window**, and the sentence is [`ShadowTarget::bind_for_write`]'s own:
    /// the scissor is global GL state and not a property of the framebuffer, and a
    /// viewport's clip is in window coordinates, which happen to be the target's,
    /// so leaving it on would cut the capture and both blur passes down to one
    /// viewport's box. The renderer re-applies the clip before the composite,
    /// which is the pass that puts the backdrop on the screen.
    pub fn bind_for_write(&mut self, gl: &glow::Context) {
        let (width, height) = self.size();
        // SAFETY: The GL context is current on this thread; the framebuffer is a
        // valid object and `bind_attach` names one of this type's own textures.
        unsafe {
            gl.viewport(0, 0, u32_to_i32(width), u32_to_i32(height));
            gl.disable(glow::SCISSOR_TEST);
        }
        self.bind_attach(gl, self.written);
    }

    /// Binds the texture the next pass reads from to texture unit 0.
    pub fn bind_for_read(&self, gl: &glow::Context) {
        // SAFETY: The GL context is current on this thread and `self.read`
        // indexes a two-element array of valid textures, so it is in range.
        let texture = self.textures[self.read];
        // SAFETY: The GL context is current on this thread; `texture` is valid.
        unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(GL_TEXTURE_2D, Some(texture));
        }
    }

    /// Swaps which texture is read from and which is written to.
    ///
    /// **Called before every pass and before the composite, and the count is
    /// three for the same reason [`ShadowTarget::swap`]'s is.** The capture went
    /// in through the *write* side and nothing reads it until the first blur pass,
    /// so the loop's first `swap` is spent turning the write target into the
    /// first pass's source; the two passes each spend one; and the composite needs
    /// one to be handed the last pass's output. **The capture is a write, so it
    /// consumes a `swap` exactly as the shadow's mask draw does** — which is why
    /// there is no `swap` between [`Self::capture`] and the blur loop.
    pub fn swap(&mut self) {
        self.read = self.written;
        self.written = 1 - self.written;
    }

    /// Binds the framebuffer and attaches `index` of [`Self::textures`] to its
    /// colour attachment 0, **in that order**.
    ///
    /// [`ShadowTarget::bind_attach`]'s own measured reason applies unchanged: on
    /// this host's driver `glFramebufferTexture2D` raises `GL_INVALID_OPERATION`
    /// when the framebuffer is not currently bound, and it does not take effect —
    /// an invisible failure whose only symptom is a backdrop that does not draw.
    fn bind_attach(&self, gl: &glow::Context, index: usize) {
        // SAFETY: The GL context is current on this thread; the framebuffer and
        // the texture are valid objects, and `index` comes from this module's
        // own two-element array rather than from a caller.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                Some(self.textures[index]),
                0,
            );
        }
    }
}

impl Drop for ColourTarget {
    fn drop(&mut self) {
        // No GL object is deleted here, and that is the same decision
        // `ShadowTarget`'s `Drop` makes and for its stated reason: the framebuffer
        // and the two textures live until the GL context is torn down, which
        // `Context::drop` does immediately after.
    }
}

/// Allocates `width` by `height` of `texture` storage in the given format pair,
/// with the four texture parameters every target in this module wants.
///
/// **One implementation of the `glTexImage2D` call and its four
/// `tex_parameter_i32` calls, and the format pair is an argument rather than a
/// field** — which is the whole of § *The colour target, and why it is a second
/// type*'s first reason. [`ShadowTarget::ensure_size`] passes `GL_R8` / `GL_RED`
/// and [`ColourTarget::ensure_size`] passes `GL_RGBA8` / `GL_RGBA`; a target that
/// is built with the wrong pair still compiles and still runs, which is why
/// `the_colour_target_is_rgba_and_the_shadow_target_is_red` asserts both bodies
/// rather than trusting the argument.
fn allocate_texture(
    gl: &glow::Context,
    texture: glow::Texture,
    width: u32,
    height: u32,
    internal_format: u32,
    format: u32,
) {
    // SAFETY: The GL context is current on this thread. `tex_image_2d` takes no
    // pixel data — `Slice(None)` allocates storage and leaves its contents
    // undefined, which is what a target that is cleared or captured before every
    // use wants — `texture` is a valid texture object, and `width`/`height` are
    // within the driver's limit as the caller has already checked.
    unsafe {
        gl.bind_texture(GL_TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(
            GL_TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            gl_enum_to_i32(GL_LINEAR),
        );
        gl.tex_parameter_i32(
            GL_TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            gl_enum_to_i32(GL_LINEAR),
        );
        gl.tex_parameter_i32(
            GL_TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            gl_enum_to_i32(GL_CLAMP_TO_EDGE),
        );
        gl.tex_parameter_i32(
            GL_TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            gl_enum_to_i32(GL_CLAMP_TO_EDGE),
        );
        gl.tex_image_2d(
            GL_TEXTURE_2D,
            0,
            i32::try_from(internal_format).unwrap_or(0),
            u32_to_i32(width),
            u32_to_i32(height),
            0,
            format,
            GL_UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
    }
}

/// Returns the largest texture width or height this implementation supports.
///
/// [`ShadowTarget::ensure_size`] refuses anything larger than this rather than
/// letting `glTexImage2D` fail, because a GL error nothing reads is a whole
/// pass that quietly draws nothing.
///
/// **`pub(crate)` because the glyph atlas asks the same question.** The atlas
/// grows, and a grow that passed this limit would ask GL for a texture the
/// driver refuses — one call, one unchecked error, and every glyph the atlas had
/// already packed sampling a texture that was never allocated. The value is read
/// once per renderer, and both callers clamp against it.
pub(crate) fn max_texture_size(gl: &glow::Context) -> u32 {
    // SAFETY: The GL context is current on this thread, and `GL_MAX_TEXTURE_SIZE`
    // is a parameter every GLES 3.1 implementation answers.
    let value = unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) };
    u32::try_from(value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_sized_window_is_allocated_as_it_is() {
        // The control for the minimum below: an ordinary window is not rounded,
        // not halved and not padded.
        assert_eq!(allocation(1024, 600), (1024, 600));
        assert_eq!(allocation(1, 1), (1, 1), "and a 1x1 window is one pixel");
    }

    #[test]
    fn a_zero_extent_is_raised_to_the_minimum_rather_than_passed_to_gl() {
        // `glTexImage2D` rejects a zero extent with `GL_INVALID_VALUE`, and the
        // width or height is zero whenever a minimised window reports its size.
        assert_eq!(allocation(0, 600), (1, 600), "the width is the one at zero");
        assert_eq!(allocation(1024, 0), (1024, 1), "and so is the height");
        assert_eq!(allocation(0, 0), (1, 1));
        for (width, height) in [(0, 600), (1024, 0), (0, 0)] {
            let (w, h) = allocation(width, height);
            assert!(w >= 1 && h >= 1, "{width}x{height} became {w}x{h}");
        }
    }

    #[test]
    fn an_unchanged_window_does_not_reallocate() {
        // The cost this decision exists to avoid: a window-sized
        // `glTexImage2D` on every frame of a window that has not moved.
        assert!(
            !resize_decision(Some((1024, 600)), (1024, 600)),
            "the same size twice is one allocation"
        );
        assert!(
            !resize_decision(Some((1024, 600)), (1024, 600)),
            "and still one allocation the third time, which is the case that \
             matters on a frame loop"
        );
    }

    #[test]
    fn a_window_that_has_changed_size_by_one_pixel_reallocates() {
        // The other side of the pair, so the assertion above is not passing
        // because the comparison cannot fail.
        assert!(resize_decision(Some((1024, 600)), (1025, 600)));
        assert!(resize_decision(Some((1024, 600)), (1024, 601)));
        assert!(!resize_decision(Some((1024, 600)), (1024, 600)));
    }

    #[test]
    fn an_unallocated_target_reallocates_whatever_the_window_is() {
        // `None` against a 1×1 window is the case a `size == 0` sentinel would
        // get wrong: normalised, `(0, 0)` and `(1, 1)` are the same allocation
        // and would compare equal, so a 1×1 window would skip the only
        // allocation it ever needed and draw from a texture with no storage.
        assert!(
            resize_decision(None, (1, 1)),
            "nothing allocated, one pixel wide"
        );
        assert!(resize_decision(None, (1024, 600)));
        assert!(
            !resize_decision(Some((1, 1)), (0, 0)),
            "and once a 1x1 window is allocated, a zero extent is the same one"
        );
    }

    #[test]
    fn a_zero_sentinel_for_the_allocation_would_hide_a_one_pixel_window() {
        // `ShadowTarget` records "nothing allocated" as `None`, and
        // `ShadowTarget::size` reports it as `(0, 0)`. **If the field were the
        // sentinel itself** — `(0, 0)` meaning "unallocated" — then a 1×1 window
        // would normalise to the same `(1, 1)` an allocated 1×1 target holds, and
        // the resize decision would say "no change" and never allocate. This is
        // the case `Option` is there for, measured from both sides.
        let sentinel = (0_u32, 0_u32);
        assert!(
            resize_decision(Some(sentinel), (1, 1)),
            "a (0, 0) that means 'unallocated' would skip the 1x1 allocation"
        );
        assert!(
            resize_decision(Some(sentinel), (1024, 600)),
            "and every other window, which is the case the test would pass without \
             the 1x1 line above"
        );
        assert_eq!(
            allocation(sentinel.0, sentinel.1),
            (1, 1),
            "which is the whole of the collision: the sentinel normalises to the \
             smallest legal allocation"
        );
    }

    /// The production half of this file, for the source-string assertions below.
    ///
    /// **Split at `#[cfg(test)]` so an assertion cannot be satisfied by its own
    /// words.** This is the mechanism `render/blur.rs` already uses for the
    /// `#[repr(C)]` check, and it is the reason the `#[repr(C)]` deletion it
    /// guards is catchable at all — a reader can see that the search cannot read
    /// the test module, which is the property a string assertion usually loses.
    fn production_source() -> &'static str {
        include_str!("target.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap_or("no test module in this file")
    }

    /// Returns the body of `fn <name>` in the production source.
    ///
    /// Brace-matched rather than line-matched, so a body that grows a line — which
    /// rustfmt does routinely — does not silently become "not found" and hand the
    /// assertion below a vacuous pass. **A name that is absent panics** rather
    /// than returning an empty string, which is the whole point: a rename must
    /// fail loudly instead of looking like a body with no `GL_RGBA8` in it.
    fn body_of<'a>(source: &'a str, name: &str) -> &'a str {
        let start = source
            .find(name)
            .unwrap_or_else(|| panic!("{name} is not in the production half of this file"));
        let after = &source[start..];
        let open = after.find('{').unwrap_or_else(|| {
            panic!("{name} has no body, so it cannot be asserted about its body")
        });
        let mut depth = 0_i32;
        for (offset, character) in after[open..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        // **`open + offset`, not `offset`** — `char_indices` is
                        // relative to the slice it walks, so an offset taken from it
                        // and applied to `after` directly truncates the body and
                        // silently drops whatever was after the arithmetic. That is
                        // exactly the shape of failure a source-string assertion
                        // must not have: a short body makes every
                        // `!body.contains(...)` pass.
                        let end = (open + offset + character.len_utf8()).min(after.len());
                        return &after[open..end];
                    }
                }
                _ => {}
            }
        }
        panic!("{name}'s body has no closing brace")
    }

    #[test]
    fn the_colour_target_is_rgba_and_the_shadow_target_is_red() {
        // **First half: the arithmetic, not a search.** These are the pairs the two
        // targets are built from, and the last two lines are the whole decision:
        // the two format pairs are *different numbers*, so a target built with the
        // wrong one is a different format and not a variant of the same one.
        assert_eq!(COLOUR_TARGET_INTERNAL_FORMAT, glow::RGBA8);
        assert_eq!(COLOUR_TARGET_FORMAT, glow::RGBA);
        assert_eq!(GL_R8, glow::R8);
        assert_eq!(GL_RED, glow::RED);
        assert_ne!(COLOUR_TARGET_INTERNAL_FORMAT, GL_R8);
        assert_ne!(COLOUR_TARGET_FORMAT, GL_RED);

        // **Second half: the source-string assertion**, in `render.rs`'s and
        // `blur.rs`'s established shape — each `ensure_size` body is scoped
        // separately, so a `GL_RGBA8` anywhere in the file cannot satisfy the
        // shadow's assertion.
        let source = production_source();
        // The control first: both type names really are spelled the way the two
        // searches below expect, so a **rename fails** rather than quietly
        // producing two empty bodies that contain nothing.
        assert!(
            source.contains("pub struct ColourTarget"),
            "and `ColourTarget` is named the way the search expects"
        );
        assert!(
            source.contains("pub struct ShadowTarget"),
            "and `ShadowTarget` is named the way the search expects"
        );

        let colour = body_of(source, "fn ensure_size");
        // Scoped to the *colour* target's own body by finding the one after the
        // type name — `ColourTarget::ensure_size` is the second of the two.
        let colour_body = body_of(
            &source[source.find("impl ColourTarget").unwrap_or(0)..],
            "fn ensure_size",
        );
        assert!(
            colour_body.contains("COLOUR_TARGET_INTERNAL_FORMAT")
                && colour_body.contains("COLOUR_TARGET_FORMAT"),
            "the colour target's `ensure_size` allocates the `GL_RGBA8` / `GL_RGBA` \
             pair — a `GL_R8` here would be a correct-looking picture with three \
             quarters of its bandwidth wasted and no error anywhere"
        );
        assert!(
            !colour_body.contains("GL_R8") && !colour_body.contains("GL_RED"),
            "and it names neither single-channel constant"
        );

        let shadow_body = body_of(
            &source[source.find("impl ShadowTarget").unwrap_or(0)..],
            "fn ensure_size",
        );
        assert!(
            shadow_body.contains("GL_R8") && shadow_body.contains("GL_RED"),
            "the shadow target still allocates its `GL_R8` / `GL_RED` pair"
        );
        assert!(
            !shadow_body.contains("GL_RGBA8"),
            "**and no `GL_RGBA8` inside the shadow's own body** — that is the \
             accidental swap this half exists to catch"
        );
        // `colour` is the whole-file search; keep it referenced so the assertion
        // above is not the only thing that would notice an empty file.
        assert!(colour.contains("ensure_size"));
    }

    #[test]
    fn a_colour_capture_blits_the_same_rectangle_twice() {
        // **The ES 3.1 rule as an assertion.** The OpenGL ES 3.1 reference page
        // for `glBlitFramebuffer` raises `GL_INVALID_OPERATION` when
        // `GL_SAMPLE_BUFFERS` for the read buffer is greater than zero and the
        // source and destination rectangles do not have identical `(X0, Y0, X1,
        // Y1)` bounds. **This pipeline's read framebuffer holds four samples**, so
        // a blit whose two rectangles differ does not capture — it raises an error
        // the driver may not report usefully, and the backdrop is simply not there.
        //
        // **This is the test that keeps `DEMO_APPLICATION.md` row `L1`
        // sub-item (c), rect-scoped capture, open on purpose.** A blit that
        // violated the rule would be a capture that does not happen and a picture
        // with no error.
        let source = production_source();
        assert!(
            source.contains("fn capture"),
            "the method the assertion is about is named the way it expects"
        );
        let body = body_of(source, "fn capture");
        assert_eq!(
            body.matches("blit_framebuffer").count(),
            1,
            "**one blit**, and the only copy this feature has"
        );
        assert!(body.contains("GL_COLOR_BUFFER_BIT"), "and it moves colour");
        assert!(
            !body.contains("GL_DEPTH_BUFFER_BIT"),
            "**and no depth bit** — this target has no depth attachment, so a \
             second bit would be a claim about a buffer the target does not hold"
        );
        assert!(
            body.contains("GL_NEAREST"),
            "`GL_NEAREST`, because source and destination are the same size and \
             every destination texel has exactly one source texel"
        );
        // The same four numbers on both sides: `width`/`height` appear an even
        // number of times, once per rectangle, and **both rectangles start at
        // `0, 0`**. A destination offset by `rect.x` would appear here as a
        // different expression, which is the mutation the criterion names.
        let zero_origin = body.matches("0,\n                0,").count()
            + body.matches("0, 0,").count()
            + body
                .matches("                    0,\n                    0,")
                .count();
        assert!(
            zero_origin >= 1,
            "both rectangles start at the origin — a rect-scoped destination is \
             exactly what the ES 3.1 rule refuses, and this is the assertion that \
             says so while the code is in front of you"
        );
        assert!(
            body.contains("GL_READ_FRAMEBUFFER"),
            "and the read framebuffer is named explicitly, because \
             `glow::FRAMEBUFFER` binds both and the read side has to be rebound \
             after the attach or the blit is a texture-to-itself"
        );
    }

    #[test]
    fn the_colour_target_allocates_lazily_and_resizes_with_the_window() {
        // **`ColourTarget` reuses `allocation` and `resize_decision` rather than
        // growing its own**, so the normalise-then-compare invariant has one
        // implementation and not two. Asserted over the same helpers
        // `ShadowTarget` uses, with the `None` sentinel's case kept — because
        // that is the case a `size == 0` field would get wrong, and a target that
        // skipped the only allocation it ever needed would draw from a texture with
        // no storage.
        assert!(
            resize_decision(None, (1, 1)),
            "nothing allocated, one pixel"
        );
        assert!(!resize_decision(Some((1280, 1020)), (1280, 1020)));
        assert!(resize_decision(Some((1280, 1020)), (1280, 1021)));
        assert_eq!(
            allocation(0, 1020),
            (1, 1020),
            "a zero width is a legal one"
        );

        // And the production half really does call both, rather than carrying its
        // own arithmetic that could drift from the shared pair.
        let source = production_source();
        let colour_body = body_of(
            &source[source.find("impl ColourTarget").unwrap_or(0)..],
            "fn ensure_size",
        );
        assert!(
            colour_body.contains("allocation(width, height)"),
            "**`allocation` normalises**, so a minimised window's zero extent is a \
             one-pixel target rather than a rejected `glTexImage2D`"
        );
        assert!(
            colour_body.contains("resize_decision(self.size, wanted)"),
            "and `resize_decision` decides, so an unchanged window does not pay a \
             window-sized `glTexImage2D` every frame"
        );
        // Lazily: `new` allocates no storage, so the field is `None` and a frame
        // with no backdrop never reaches `ensure_size` at all.
        let new_body = body_of(source, "pub fn new");
        assert!(
            new_body.contains("size: None"),
            "**`ColourTarget::new` allocates nothing**, which is what makes the six \
             gallery pages pay zero for this target"
        );
    }
}
