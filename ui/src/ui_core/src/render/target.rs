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
//! ## Two textures, one framebuffer
//!
//! A separable blur cannot read and write the same texture, so this holds two
//! and swaps which one is attached between passes: the shape is drawn into one,
//! the horizontal pass writes the other and reads the first, the vertical pass
//! writes the first back. One framebuffer is enough because the attachment is
//! what changes, and re-attaching is a state change rather than an object.

use crate::render::{
    gl_enum_to_i32, u32_to_i32, RenderError, GL_CLAMP_TO_EDGE, GL_LINEAR, GL_R8, GL_RED,
    GL_TEXTURE_2D, GL_UNSIGNED_BYTE,
};
use glow::HasContext;

/// The smallest width or height a target is ever allocated at.
///
/// GL's `glTexImage2D` rejects a zero extent, and the arithmetic below produces
/// a zero whenever the window reports one — which a minimised window on some
/// compositors does. **One pixel is not a picture anyone sees**, so it is not a
/// quality trade: it is what makes a zero-sized allocation a legal one.
const MIN_TARGET_EXTENT: u32 = 1;

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
        // `glTexImage2D` and left as an unchecked GL error — which is the shape
        // of failure `.ai/NEVERAGAIN.md` § *A buffer sized for one vertex per
        // quad* records, where a rejected call with nobody reading the rejection
        // dropped a whole batch.
        let limit = max_texture_size(gl);
        if w > limit || h > limit {
            return Err(RenderError::Gl(format!(
                "shadow target of {w}x{h} is past the driver's largest texture, \
                 {limit}x{limit}"
            )));
        }
        for texture in self.textures {
            // SAFETY: The GL context is current on this thread. `tex_image_2d`
            // takes no pixel data — `Slice(None)` allocates storage and leaves
            // its contents undefined, which is what a target that is cleared
            // before every use wants — `texture` is a valid texture object, and
            // `w`/`h` are within the driver's limit as checked above.
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
                    i32::try_from(GL_R8).unwrap_or(0),
                    u32_to_i32(w),
                    u32_to_i32(h),
                    0,
                    GL_RED,
                    GL_UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
            }
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
    /// was found by reading `gl.get_error()` after each new call once, which is
    /// the rule `.ai/NEVERAGAIN.md` § *A buffer sized for one vertex per quad*
    /// records for a rejected GL call nobody read.
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

/// Returns the largest texture width or height this implementation supports.
///
/// [`ShadowTarget::ensure_size`] refuses anything larger than this rather than
/// letting `glTexImage2D` fail, because a GL error nothing reads is a whole
/// pass that quietly draws nothing.
fn max_texture_size(gl: &glow::Context) -> u32 {
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
}
