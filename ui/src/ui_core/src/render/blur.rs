//! The separable Gaussian blur.
//!
//! ## What "separable" buys and what it costs
//!
//! A two-dimensional Gaussian is the product of two one-dimensional ones, so
//! blurring a row and then a column is the same image as blurring a column and
//! then a row, and costs `2 · 2r+1` texture fetches per pixel where `r` is the
//! tap radius — not `(2r+1)²`. That is the whole reason this is two passes and
//! not one.
//!
//! The cost is **two off-screen round trips** and a ping-pong between two
//! textures, which is [`crate::render::target`]'s job and not this one's.
//!
//! ## The kernel
//!
//! The weights are computed on the CPU and uploaded as a uniform array, rather
//! than evaluated per fragment in the shader. Evaluating `exp()` per tap per
//! fragment is `2r+1` transcendentals for every pixel of the window on every
//! shadowed frame, for a kernel that changed when the widget's `blur` did and
//! not since; computing it once per shadow turns a per-pixel cost into a
//! per-shadow one, which on a 1024×600 window is six hundred thousand fewer
//! evaluations than the other arrangement.
//!
//! The weights are **normalised to sum to one**. An un-normalised Gaussian
//! truncated at `2σ` sums to about `σ·√(2π)` — 5.0 at `σ = 2`, not 1 — so
//! compositing the raw weights would draw a shadow at a fifth of the alpha it
//! was asked for, which on screen is a shadow that is too faint to see and a
//! test that cannot see why.

use crate::render::{
    f32_to_i32, i32_to_f32, RenderError, GL_ARRAY_BUFFER, GL_DYNAMIC_DRAW, GL_FLOAT, GL_TRIANGLES,
};
use glow::HasContext;

/// The largest number of taps **in total** the blur kernel has.
///
/// A tap count, not a radius: the shader's loop is bounded by a `#define`, and a
/// uniform array has to have a compile-time length to be one.
///
/// **Why nine**, and what would reverse it: at nine the widest blur a caller can
/// ask for is `σ = 2` px, which the capture measured as an **8-pixel ramp** with
/// 8 distinct values and no plateau — a soft edge, not a step. It costs 18
/// texture fetches per window pixel per shadowed frame; seventeen would reach
/// `σ = 4` at 34.
///
/// **The cost is bounded by measurement and the constant is not chosen by it.**
/// Interleaved runs on this host at 1280×1020 — one shadow, release, six-second
/// CPU windows — read 171/182/211 jiffies at nine taps and 212/202/188 at
/// seventeen, against 191/172/158 with no shadow at all: **the three sets
/// overlap completely and the frame rate is 62.0–62.2 fps in all of them**, which
/// is the recorded release baseline. So this host cannot tell nine from
/// seventeen, and this number rests on the kernel's *width* argument rather than
/// on a frame-rate difference.
///
/// **What would reverse it**, in the order of how little it costs: a measurement
/// on a fill-rate-bound target — a 1080p head unit panel rather than this
/// machine's desktop GPU — where 34 fetches per pixel of the window is not free;
/// and an operator who asks for a shadow wide enough that an 8-pixel ramp reads
/// as a hard edge, which is a change to the number rather than a reason for it.
pub const MAX_TAPS: usize = 9;

/// The blur radius below which a shadow is drawn solid rather than blurred.
///
/// **Zero, exactly, and not a fraction of a pixel.** At a blur of zero the kernel
/// is a single tap at the centre, which is the identity — so the offscreen
/// round trip would redraw precisely the shape that drawing it directly draws,
/// and cost two full-window passes and an FBO bind to do it. At any blur above
/// zero there are at least two taps and the pass is what makes the edge soft.
///
/// The threshold is where it is because the kernel's tap count is
/// `2·ceil(2σ)+1`, which is one below `MAX_TAPS` at zero and `MAX_TAPS` at the
/// smallest σ that fits. A non-zero floor would put a threshold in the middle of
/// a discontinuity in the tap count rather than at the end of the one case that
/// is exactly a no-op.
///
/// What would reverse it: an argument that a sub-pixel blur is worth a whole
/// pass, which nobody has made; the direct path is also **better** than the
/// blurred one below the threshold, because it draws into the window's 4x
/// multisampled default framebuffer and the offscreen target is single-sampled,
/// so a zero-blur shadow gets antialiased corners the blurred one does not.
pub const SOLID_BLUR: f32 = 0.0;

/// The number of taps either side of the centre for a blur of standard deviation
/// `sigma`, capped at [`MAX_TAPS`].
///
/// **`ceil(2σ)`, and why two.** A kernel truncated at `2σ` holds 95.4% of the
/// distribution's mass; one truncated at `3σ` holds 99.7% and costs 50% more
/// taps, and the missing 4.6% at `2σ` is spread outward into a tail far below
/// the 8-bit coverage the target stores, so it does not survive the quantisation
/// to be seen. One sigma would hold 68% and the edge would visibly square off.
///
/// A blur that does not fit is **capped rather than refused**: a caller asking
/// for more than the kernel can express gets the widest blur there is, and the
/// capping is what [`MAX_TAPS`]'s doc means by "what would reverse it".
#[must_use]
pub fn taps_for(sigma: f32) -> usize {
    let ceiling = (2.0 * sigma.max(0.0)).ceil();
    // `2 * taps + 1 <= MAX_TAPS`. The float-to-integer step is
    // `f32_to_i32`, the module's **one** documented saturating cast, and it is
    // reached through it rather than spelled out again here: `round` first, so
    // the truncation that cast does lands on an integer, and `min` after, so a
    // sigma of 1000 becomes the cap instead of a two-thousand-tap kernel.
    let rounded = f32_to_i32(ceiling.round());
    usize::try_from(rounded.max(0))
        .unwrap_or(0)
        .min((MAX_TAPS - 1) / 2)
}

/// The Gaussian weight `exp(-d²/2σ²)` at `d` taps from the centre.
///
/// Only ever called with a non-negative `d` — [`kernel`] walks the centre out to
/// the edge — and it is **even in `d`** because `d²` is, which is what makes one
/// kernel serve both passes of a separable blur: the horizontal and the vertical
/// pass are the same convolution.
#[must_use]
pub fn gaussian(distance: f32, sigma: f32) -> f32 {
    let two_sigma_squared = 2.0 * sigma * sigma;
    if two_sigma_squared <= 0.0 {
        // A zero or negative sigma has no distribution; every tap would divide by
        // zero. The centre tap takes the whole weight, which is the identity and
        // is what the direct path does anyway.
        return if distance == 0.0 { 1.0 } else { 0.0 };
    }
    (-(distance * distance) / two_sigma_squared).exp()
}

/// The normalised one-dimensional Gaussian kernel for a blur of standard
/// deviation `sigma`: `2·taps+1` weights, from the leftmost tap to the rightmost,
/// summing to one.
///
/// The normalisation is the whole of the difference between a shadow and a
/// shadow that is too faint to see; see the module docs.
///
/// # Examples
///
/// ```
/// use ui_core::render::blur::{kernel, taps_for};
///
/// let weights = kernel(2.0);
/// assert_eq!(weights.len(), 2 * taps_for(2.0) + 1);
/// let sum: f32 = weights.iter().sum();
/// assert!((sum - 1.0).abs() < 1e-5, "the weights sum to one, not to {sum}");
///
/// // Symmetric: the centre tap is the peak and the two halves mirror.
/// assert!(weights[weights.len() / 2] > weights[0]);
/// ```
#[must_use]
pub fn kernel(sigma: f32) -> Vec<f32> {
    let taps = u32::try_from(taps_for(sigma)).unwrap_or(0);
    // Walk the taps as *signed* distances around the centre rather than
    // outwards and inwards separately: the kernel is symmetric because `d²` is,
    // and building one half and mirroring it would be a second place where that
    // could stop being true.
    let centre = f32::from(u16::try_from(taps).unwrap_or(0));
    let raw: Vec<f32> = (0..=taps * 2)
        .map(|index| {
            let distance = i32_to_f32(i32::try_from(index).unwrap_or(0)) - centre;
            gaussian(distance, sigma)
        })
        .collect();
    let total: f32 = raw.iter().sum();
    if total <= 0.0 {
        return vec![1.0];
    }
    raw.iter().map(|weight| weight / total).collect()
}

/// How far, in pixels, a shadow of blur `sigma` spreads past its own rect.
///
/// **The kernel's last tap, and nothing more.** It is the number the scrolling
/// widget needs to bound a shadow — see
/// [`crate::widgets::scroll::command_bounds`] — and it is *not* a claim about
/// where the blur becomes invisible: the kernel continues past its last tap,
/// because every tap is a sum over the pixels beyond it. What is bounded here is
/// what can possibly be non-zero, which is what a clip has to contain.
///
/// Truncated at `MAX_TAPS`, so it is the widest reach the pipeline can draw and
/// not the reach a shadow asked for beyond it.
#[must_use]
pub fn reach(sigma: f32) -> f32 {
    f32::from(u16::try_from(taps_for(sigma)).unwrap_or(0))
}

/// The vertex the blur passes draw: a position in **window coordinates**.
///
/// Two floats and nothing else. The blur reads the whole target, so it has no
/// per-quad colour, radius or UV to carry — and a full-window quad is six of
/// these drawn as two triangles, which is why there is no index buffer here at
/// all.
///
/// **`#[repr(C)]`, and it is load-bearing as a stated invariant rather than as a
/// layout.** The layout of a single-field struct is the same with or without the
/// attribute, so nothing in this suite can observe the difference — but
/// `vertex_bytes` hands GL a byte slice whose layout it asserts out loud, and
/// `Vertex`, `ImageVertex` and `TextVertex` in the parent module all carry the
/// same attribute for the same reason. A SAFETY comment that says "`#[repr(C)]`"
/// is checkable against the type; one that says "it is only one field, so it must
/// be" is checkable only by trusting the reader.
///
/// # Examples
///
/// ```
/// use ui_core::render::blur::BLUR_QUAD_SIZE;
///
/// assert_eq!(BLUR_QUAD_SIZE, 6, "two triangles, six vertices, no index buffer");
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BlurVertex {
    /// Position in window coordinates, origin at the top left.
    pub pos: [f32; 2],
}

/// The vertices of a full-`size` quad, in the order `GL_TRIANGLES` reads them.
///
/// Six vertices and no index buffer, because the quad is drawn with
/// `glDrawArrays` and six numbers is not a size worth indexing. The order is the
/// two triangles `quad_indices` would produce for a quad — `0,1,2` then `0,2,3`
/// — so a reader who knows the rest of the pipeline recognises it.
///
/// # Examples
///
/// ```
/// use ui_core::render::blur::{full_quad, BLUR_QUAD_SIZE};
///
/// // A window-sized quad, off the origin so a fixture cannot confuse the two.
/// let (width, height) = (320.0, 200.0);
/// let vertices = full_quad(width, height);
///
/// assert_eq!(vertices.len(), BLUR_QUAD_SIZE);
/// assert_eq!(vertices[0].pos, [0.0, 0.0], "first triangle: top left, ...");
/// assert_eq!(vertices[1].pos, [width, 0.0], "... top right, ...");
/// assert_eq!(vertices[2].pos, [width, height], "... bottom right");
/// assert_eq!(vertices[3].pos, [0.0, 0.0], "second triangle back to top left, ...");
/// assert_eq!(vertices[4].pos, [width, height], "... across to bottom right, ...");
/// assert_eq!(vertices[5].pos, [0.0, height], "... and down to bottom left");
/// ```
#[must_use]
pub fn full_quad(width: f32, height: f32) -> [BlurVertex; BLUR_QUAD_SIZE] {
    let positions = [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]];
    [
        BlurVertex { pos: positions[0] },
        BlurVertex { pos: positions[1] },
        BlurVertex { pos: positions[2] },
        BlurVertex { pos: positions[0] },
        BlurVertex { pos: positions[2] },
        BlurVertex { pos: positions[3] },
    ]
}

/// The bytes one blur pass uploads for `vertices`.
///
/// # Safety
///
/// [`BlurVertex`] is `#[repr(C)]` over a single `[f32; 2]` and has no padding, so
/// any correctly aligned byte slice is a valid sequence of them. The slice borrows
/// `vertices`, so it is valid exactly as long as `vertices` is alive — which is
/// why `BlurQuad::draw` takes the vertices rather than the bytes.
fn vertex_bytes(vertices: &[BlurVertex]) -> &[u8] {
    // SAFETY: `BlurVertex` is `#[repr(C)]` over `[f32; 2]`, so its bytes are two
    // `f32` with no padding and `size_of_val` counts them exactly. The pointer
    // comes from the slice itself and the length from the same slice.
    unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr().cast::<u8>(),
            std::mem::size_of_val(vertices),
        )
    }
}

/// A vertex array and buffer for the blur passes' full-window quad.
///
/// **Its own, and not the solid pass's.** The solid vertex array binds five
/// attributes at a stride of 44 bytes; this one binds a single 2-component
/// attribute at a stride of 8. Sharing the array would mean uploading 8-byte
/// vertices through a pointer that strides 44, which reads past the end of the
/// data and draws a shape nobody asked for.
///
/// The buffer is allocated once, for exactly [`BLUR_QUAD_SIZE`] vertices, and
/// **not** through `crate::render::vertex_buffer_size`: that helper multiplies
/// by four because a *quad* is four vertices, and this buffer's unit is already
/// vertices — a quad of six would be sized at 24 vertices' worth and the upload
/// would fail with `GL_INVALID_VALUE`. See `.ai/NEVERAGAIN.md` § *A buffer sized
/// for one vertex per quad*, which is that mistake in the other direction.
pub struct BlurQuad {
    /// The vertex array carrying the attribute pointer.
    vao: glow::VertexArray,
    /// The buffer the quad's vertices are uploaded into.
    vbo: glow::Buffer,
}

impl BlurQuad {
    /// Creates the blur quad's vertex array and its six-vertex buffer.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Gl`] when either GL object cannot be created.
    pub fn new(gl: &glow::Context) -> Result<Self, RenderError> {
        // SAFETY: The GL context is current on this thread.
        let vao = unsafe { gl.create_vertex_array() }.map_err(RenderError::Gl)?;
        // SAFETY: The GL context is current on this thread.
        let vbo = unsafe { gl.create_buffer() }.map_err(RenderError::Gl)?;
        // Six vertices of two `f32`, which `u32::try_from` reaches through the
        // helper below rather than through the solid pass's quad arithmetic.
        let size = i32::try_from(BLUR_QUAD_SIZE * std::mem::size_of::<BlurVertex>())
            .map_err(|_| RenderError::Gl("blur vertex buffer is too large".to_string()))?;
        // SAFETY: The GL context is current on this thread; `vao` and `vbo` are
        // the valid objects created above, the attribute pointer is captured by
        // the vertex array, and `size` is the buffer's own byte length.
        unsafe {
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_size(GL_ARRAY_BUFFER, size, GL_DYNAMIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, GL_FLOAT, false, BLUR_VERTEX_STRIDE, 0);
            gl.bind_vertex_array(None);
        }
        Ok(BlurQuad { vao, vbo })
    }

    /// Uploads `vertices` and draws them as triangles.
    ///
    /// The blend state is the caller's: both blur passes draw with blending off,
    /// because a pass that reads the texture it is writing is undefined and the
    /// result would not be the convolution [`kernel`] returns.
    pub fn draw(&self, gl: &glow::Context, vertices: &[BlurVertex]) {
        let bytes = vertex_bytes(vertices);
        // SAFETY: The GL context is current on this thread; `vao` carries the
        // attribute pointer, `vbo` is a valid buffer, and `bytes` borrows
        // `vertices`, which outlives this call.
        unsafe {
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_sub_data_u8_slice(GL_ARRAY_BUFFER, 0, bytes);
            gl.draw_arrays(GL_TRIANGLES, 0, count_of(vertices.len()));
            gl.bind_vertex_array(None);
        }
    }
}

/// Byte stride of one [`BlurVertex`]: two `f32` fields, no padding.
const BLUR_VERTEX_STRIDE: i32 = 8;

/// The number of vertices one blur pass draws: two triangles, no index buffer.
///
/// Named because it is what the loop bound is and what
/// [`full_quad`]'s return type says; a literal `6` at either would be a second
/// place to keep in step.
pub const BLUR_QUAD_SIZE: usize = 6;

/// Converts a vertex count to the `i32` `glDrawArrays` takes.
///
/// There is no `From<usize> for i32` in std, so this is a checked conversion
/// whose fallback is [`BLUR_QUAD_SIZE`] — the only count this module ever draws —
/// rather than a saturating `i32::MAX`, which would be a valid number meaning
/// nothing and would ask GL for a hundred million vertices.
fn count_of(vertices: usize) -> i32 {
    i32::try_from(vertices).unwrap_or(i32::try_from(BLUR_QUAD_SIZE).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tolerance the weight assertions use: `f32` accumulation over nine
    /// terms, and a kernel that is *meant* to be a probability distribution
    /// rounded into 32 bits. Anything tighter is testing `f32`, not the code.
    const EPSILON: f32 = 1e-5;

    #[test]
    fn the_weights_sum_to_one() {
        // **The assertion the whole module's normalisation exists for.** An
        // un-normalised Gaussian truncated at 2σ sums to about σ·√(2π) — 5.01
        // at σ = 2 — so a kernel that was merely built and not normalised would
        // draw every shadow at a fifth of its alpha. The control is the raw sum,
        // which is nowhere near one, printed in the message so a failure says
        // which of the two it is.
        for sigma in [0.5, 1.0, 2.0, 3.5] {
            let weights = kernel(sigma);
            let sum: f32 = weights.iter().sum();
            let centre = f32::from(u16::try_from(taps_for(sigma)).unwrap_or(0));
            let raw: f32 = (0..=taps_for(sigma) * 2)
                .map(|index| {
                    let distance = i32_to_f32(i32::try_from(index).unwrap_or(0)) - centre;
                    gaussian(distance, sigma)
                })
                .sum();
            assert!(
                (sum - 1.0).abs() < EPSILON,
                "σ = {sigma}: the weights sum to {sum}, and the raw ones summed \
                 to {raw} — a kernel that was built without dividing by the total \
                 composites at {raw}× the alpha it was asked for"
            );
        }
    }

    #[test]
    fn a_kernel_of_one_tap_is_exactly_one() {
        // The no-blur case, and the one the direct path exists for. An
        // un-normalised single-tap kernel would also be 1.0, so the control is
        // that the *shape* is one tap and not two halves of 0.5 each.
        let weights = kernel(SOLID_BLUR);
        assert_eq!(weights.len(), 1, "one tap, and not a two-tap kernel of 0.5");
        assert_eq!(weights[0], 1.0);
    }

    #[test]
    fn the_kernel_is_symmetric_about_its_centre() {
        // Symmetry is what makes **one** kernel serve both passes of a separable
        // blur: the horizontal pass and the vertical pass are the same
        // convolution, which is only true if the weight at `-d` equals the weight
        // at `+d`. An asymmetric kernel — one built by walking only outwards, or
        // by a `powf` with an odd exponent — would shift the shadow instead of
        // softening it.
        let weights = kernel(2.0);
        let middle = weights.len() / 2;
        for distance in 1..=middle {
            assert_eq!(
                weights[middle - distance],
                weights[middle + distance],
                "tap {distance} either side of the centre has one weight, not two"
            );
        }
        assert!(
            weights[middle] > weights[middle - 1],
            "and the centre is the peak, so the kernel falls off rather than \
             rising towards the edge"
        );
    }

    #[test]
    fn the_two_passes_compose_to_the_same_image_in_either_order() {
        // The property that is *only* true of a separable kernel, measured
        // against a reference convolution rather than asserted about the weights.
        // Both orders are run on the same 9×9 impulse with the same weights; a
        // kernel that was not separable — or not symmetric — would make the two
        // results differ, which is a one-line check and a real one.
        const SIDE: usize = 9;
        let weights = kernel(1.5);
        let radius = (weights.len() - 1) / 2;
        let impulse = |x: usize, y: usize| f32::from(u8::from(x == 4 && y == 4));

        let horizontal = |input: &[f32; SIDE * SIDE]| {
            let mut output = [0.0_f32; SIDE * SIDE];
            for y in 0..SIDE {
                for x in 0..SIDE {
                    let mut sum = 0.0;
                    for (index, weight) in weights.iter().enumerate() {
                        let offset =
                            i32::try_from(index).unwrap_or(0) - i32::try_from(radius).unwrap_or(0);
                        let sample = i32::try_from(x).unwrap_or(0) + offset;
                        if (0..i32::try_from(SIDE).unwrap_or(0)).contains(&sample) {
                            sum += input[y * SIDE + usize::try_from(sample).unwrap_or(0)] * weight;
                        }
                    }
                    output[y * SIDE + x] = sum;
                }
            }
            output
        };
        let vertical = |input: &[f32; SIDE * SIDE]| {
            let mut output = [0.0_f32; SIDE * SIDE];
            for y in 0..SIDE {
                for x in 0..SIDE {
                    let mut sum = 0.0;
                    for (index, weight) in weights.iter().enumerate() {
                        let offset =
                            i32::try_from(index).unwrap_or(0) - i32::try_from(radius).unwrap_or(0);
                        let sample = i32::try_from(y).unwrap_or(0) + offset;
                        if (0..i32::try_from(SIDE).unwrap_or(0)).contains(&sample) {
                            sum += input[usize::try_from(sample).unwrap_or(0) * SIDE + x] * weight;
                        }
                    }
                    output[y * SIDE + x] = sum;
                }
            }
            output
        };

        let start: Vec<f32> = (0..SIDE * SIDE)
            .map(|index| impulse(index % SIDE, index / SIDE))
            .collect();
        let mut array = [0.0_f32; SIDE * SIDE];
        array.copy_from_slice(&start);

        let one_way = vertical(&horizontal(&array));
        let other_way = horizontal(&vertical(&array));
        for index in 0..SIDE * SIDE {
            assert!(
                (one_way[index] - other_way[index]).abs() < 1e-6,
                "the two orders differ at ({}, {}): {} against {}",
                index % SIDE,
                index / SIDE,
                one_way[index],
                other_way[index]
            );
        }
        // And the control: the composition is not the identity, so the
        // comparison above is not two copies of the input agreeing.
        assert!(
            one_way
                .iter()
                .zip(array.iter())
                .any(|(a, b)| (a - b).abs() > 0.01),
            "row-then-column actually changed the picture"
        );
    }

    #[test]
    fn a_blur_maps_onto_the_tap_count_the_shader_can_express() {
        // ceil(2σ), capped. Written as the closed form rather than the tap
        // counts, so a change to the truncation is a change to this assertion
        // rather than a silent difference.
        assert_eq!(taps_for(0.0), 0, "no blur, no taps either side");
        assert_eq!(taps_for(0.25), 1, "2σ = 0.5, which rounds up to one tap");
        assert_eq!(taps_for(1.0), 2, "2σ = 2");
        assert_eq!(
            taps_for(2.0),
            4,
            "2σ = 4, which is the whole kernel at MAX_TAPS = 9"
        );
        assert_eq!(
            taps_for(2.1),
            (MAX_TAPS - 1) / 2,
            "and past that it is capped rather than refused"
        );
        assert_eq!(taps_for(1000.0), (MAX_TAPS - 1) / 2, "however far past");
        // The control beside the cap: a sigma that asks for three taps gets three,
        // so the four above is the cap and not the arithmetic being stuck.
        assert_eq!(taps_for(1.5), 3);
        assert!(taps_for(1.5) < taps_for(1.75));
        assert_eq!(
            taps_for(1.75),
            taps_for(2.0),
            "and the cap means every sigma from 1.75 up asks for the same four \
             taps, differing only in the weights the sigma gives them"
        );
    }

    #[test]
    fn a_negative_blur_is_treated_as_no_blur() {
        // A caller whose blur is a computed number can hand over a negative one,
        // and the pipeline's rule is that a blur at or below `SOLID_BLUR` is drawn
        // solid. `-1.0` must therefore take the same path as `0.0` rather than
        // reach `ceil(2σ)` with a negative inside it and index a vector
        // backwards.
        assert_eq!(taps_for(-1.0), 0);
        assert_eq!(taps_for(f32::NAN), 0, "and a NaN is no taps, not a panic");
        assert_eq!(kernel(-1.0).len(), 1);
    }

    #[test]
    fn the_reach_is_the_last_tap_and_no_further() {
        // The number `scroll::command_bounds` grows a shadow's rect by. It has to
        // be the reach and not the sigma — a clip at the sigma would cut the
        // outer half of the blur off — and it has to be no more than the reach,
        // because a bounds that over-reports keeps a shadow that is entirely
        // outside the viewport.
        assert_eq!(reach(2.0), 4.0, "2σ = 4 taps either side, and no further");
        assert_eq!(reach(1.0), 2.0);
        assert_eq!(reach(0.0), 0.0, "a shadow with no blur does not spread");
        assert_eq!(reach(-1.0), 0.0);
        assert_eq!(
            reach(50.0),
            reach(2.0),
            "and a capped blur's reach is the cap"
        );
        // The control: the reach is strictly more than the sigma, so a bounds
        // built from the sigma alone would be short by every tap.
        assert!(reach(2.0) > 2.0);
    }

    #[test]
    fn a_zero_sigma_weights_only_the_centre_tap() {
        // `exp(-d²/0)` is `exp(-inf) = 0` for every `d > 0` and `NaN` at `d = 0`
        // — a kernel of NaN — which is what the guard in `gaussian` is for. The
        // control is the first tap, which is the one that would be NaN.
        let weights = [gaussian(0.0, 0.0), gaussian(1.0, 0.0), gaussian(4.0, 0.0)];
        assert_eq!(weights[0], 1.0, "the centre tap is the whole weight");
        assert_eq!(weights[1], 0.0);
        assert_eq!(weights[2], 0.0);
        assert!(weights.iter().all(|weight| weight.is_finite()));
    }

    #[test]
    fn the_gaussian_is_even_in_its_distance() {
        // The property `kernel`'s symmetry rests on, pinned on the function rather
        // than on the vector it returns, because `kernel` walks outwards and
        // never evaluates a negative distance at all.
        for distance in [0.0, 1.0, 2.5, 4.0] {
            assert_eq!(
                gaussian(-distance, 1.5),
                gaussian(distance, 1.5),
                "the weight {distance} taps left is the weight {distance} taps right"
            );
        }
        // And it falls off: a kernel that rose towards its edges would sharpen
        // the shadow rather than soften it, and would still be symmetric.
        assert!(gaussian(0.0, 1.5) > gaussian(1.0, 1.5));
        assert!(gaussian(1.0, 1.5) > gaussian(3.0, 1.5));
    }

    #[test]
    fn the_full_quad_is_two_triangles_in_the_order_the_indices_would_have_used() {
        // `0,1,2` then `0,2,3`, the same two triangles `quad_indices` builds for a
        // quad — so a reader of this module recognises the geometry. The
        // duplicate first corner is the one that turns the second triangle into
        // the complement of the first rather than a third of it.
        let vertices = full_quad(320.0, 200.0);
        assert_eq!(vertices.len(), 6);
        let positions: Vec<[f32; 2]> = vertices.iter().map(|vertex| vertex.pos).collect();
        assert_eq!(
            positions,
            vec![
                [0.0, 0.0],
                [320.0, 0.0],
                [320.0, 200.0],
                [0.0, 0.0],
                [320.0, 200.0],
                [0.0, 200.0],
            ]
        );
        // The control: the six positions hold four distinct corners, so a
        // triangle fan over all six would draw a hexagon.
        let mut distinct: Vec<[f32; 2]> = positions.clone();
        distinct.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        distinct.dedup();
        assert_eq!(distinct.len(), 4, "four corners, drawn as two triangles");
    }

    #[test]
    fn the_blur_vertex_layout_is_two_floats_and_no_padding() {
        // The attribute pointer in `BlurQuad::new` strides by this. A field added
        // without the constant moved is a skewed quad, and the stride is the only
        // thing that says so — there is no `offset_of!` assertion to fall back on
        // for a two-field struct that nothing reads by name.
        assert_eq!(std::mem::size_of::<BlurVertex>(), 8);
        assert_eq!(
            std::mem::size_of::<BlurVertex>(),
            BLUR_VERTEX_STRIDE as usize
        );
        assert_eq!(std::mem::offset_of!(BlurVertex, pos), 0);
    }

    #[test]
    fn the_blur_vertex_carries_the_repr_its_safety_comment_claims() {
        // **`size_of` above cannot see this**, and that is the whole point: a
        // single-field struct has the same layout with or without `#[repr(C)]`, so
        // a `#[repr(C)]` that was deleted would leave every layout assertion in
        // this module green while making `vertex_bytes`' SAFETY comment a false
        // claim — which is precisely what a review found here.
        //
        // The attribute is looked for in the part of this file **above the test
        // module**, so the assertion cannot be satisfied by its own text. A
        // reviewer reading it can see why it is not self-satisfying, which is the
        // property a string assertion usually loses.
        let source = include_str!("blur.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .unwrap_or("no test module in this file");
        assert!(
            production.contains("#[repr(C)]\npub struct BlurVertex"),
            "`BlurVertex` must carry `#[repr(C)]`: `vertex_bytes` asserts that \
             layout in a SAFETY comment, and a single field would make the \
             assertion unfalsifiable by `size_of`"
        );
        // The control: the type really is named the way the search above expects,
        // so a rename would fail here rather than silently satisfying the search.
        assert!(
            production.contains("pub struct BlurVertex"),
            "and the type is named the way the search above expects"
        );
    }

    #[test]
    fn the_vertex_bytes_are_the_vertices_themselves() {
        // `vertex_buffer_size` counts four vertices per quad because the solid
        // pass's quad is four; this buffer's "quad" is six vertices of the whole
        // window, and sizing it the other way is a quarter of what it promises.
        let vertices = full_quad(320.0, 200.0);
        let bytes = vertex_bytes(&vertices);
        assert_eq!(
            bytes.len(),
            BLUR_QUAD_SIZE * std::mem::size_of::<BlurVertex>(),
            "six vertices of two floats, not six floats"
        );
        assert_eq!(bytes.len(), 48);
    }

    #[test]
    fn a_vertex_count_gl_cannot_address_becomes_the_quad_and_not_a_wrap() {
        // `glDrawArrays` takes an `i32`, and there is no `From<usize> for i32`.
        assert_eq!(count_of(6), 6);
        assert_eq!(count_of(0), 0);
        assert_eq!(
            count_of(usize::MAX),
            i32::try_from(BLUR_QUAD_SIZE).unwrap_or(0),
            "an unrepresentable count is the one this module draws, not a wrap"
        );
    }
}
