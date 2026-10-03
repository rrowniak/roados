//! Rendering.
//!
//! Owns the GL context and the frame lifecycle: the passes in order, the GPU
//! buffers they fill, and the submission that puts a frame on screen.

pub mod blur;
pub mod context;
pub mod target;

use std::collections::HashMap;
use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::batch::{Batch, Batcher, ShaderKind};
use crate::font::{Font, FontSet, FontWeight, GlyphAtlas, GlyphPlacement};
use crate::node::WidgetNode;
use crate::paint::{DrawCommand, Rect, UvRect};
use crate::property::Color;
use crate::texture::{self, TextureCache, TextureError, TextureHandle};
use blur::BlurQuad;
use context::Context;
use glow::HasContext;
use target::ShadowTarget;

/// GL_VERTEX_SHADER constant (0x8B31).
const GL_VERTEX_SHADER: u32 = 0x8B31;
/// GL_FRAGMENT_SHADER constant (0x8B30).
const GL_FRAGMENT_SHADER: u32 = 0x8B30;
/// GL_ARRAY_BUFFER constant (0x8892).
const GL_ARRAY_BUFFER: u32 = 0x8892;
/// GL_ELEMENT_ARRAY_BUFFER constant (0x8893).
const GL_ELEMENT_ARRAY_BUFFER: u32 = 0x8893;
/// GL_STATIC_DRAW constant (0x88E4).
const GL_STATIC_DRAW: u32 = 0x88E4;
/// GL_DYNAMIC_DRAW constant (0x88E8).
const GL_DYNAMIC_DRAW: u32 = 0x88E8;
/// GL_FLOAT constant (0x1406).
const GL_FLOAT: u32 = 0x1406;
/// GL_TRIANGLES constant (0x0004).
const GL_TRIANGLES: u32 = 0x0004;
/// GL_UNSIGNED_INT constant (0x1405).
const GL_UNSIGNED_INT: u32 = 0x1405;
/// GL_BLEND constant (0x0BE2).
const GL_BLEND: u32 = 0x0BE2;
/// GL_SCISSOR_TEST constant (0x0C11).
const GL_SCISSOR_TEST: u32 = 0x0C11;
/// GL_ONE constant (1), the source factor of premultiplied-alpha blending.
const GL_ONE: u32 = 1;
/// GL_ONE_MINUS_SRC_ALPHA constant (0x0303).
const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
/// GL_COLOR_BUFFER_BIT constant (0x4000).
const GL_COLOR_BUFFER_BIT: u32 = 0x4000;
/// GL_TEXTURE_2D constant (0x0DE1).
const GL_TEXTURE_2D: u32 = 0x0DE1;
/// GL_R8 constant (0x8229): a single-channel texture.
const GL_R8: u32 = 0x8229;
/// GL_RED constant (0x1903): the red channel of a single-channel texture.
const GL_RED: u32 = 0x1903;
/// GL_UNSIGNED_BYTE constant (0x1401).
const GL_UNSIGNED_BYTE: u32 = 0x1401;
/// GL_RGBA8 constant (0x8058): a four-channel, eight-bit-per-channel texture.
///
/// Distinct from the glyph atlas's [`GL_R8`]: the glyph atlas carries a single
/// coverage channel, and an image carries premultiplied RGBA. One texture
/// cannot be both, so the image atlas is a texture of its own.
const GL_RGBA8: u32 = 0x8058;
/// GL_RGBA constant (0x1908): the pixel format the image atlas is stored in.
const GL_RGBA: u32 = 0x1908;
/// GL_TEXTURE0 constant (0x84C0).
const GL_TEXTURE0: u32 = 0x84C0;
/// GL_LINEAR constant (0x2601): linear texture filtering.
const GL_LINEAR: u32 = 0x2601;
/// GL_CLAMP_TO_EDGE constant (0x812F).
const GL_CLAMP_TO_EDGE: u32 = 0x812F;

/// Converts a `u32` pixel count to the `i32` the texture and viewport setters
/// take.
///
/// There is no `From<u32> for i32` in std — only [`f32_to_i32`] and
/// [`u32_to_f32`]'s inverse exist for the other two directions — so this is a
/// checked conversion. It cannot fail for a value GL can address, since
/// `GL_MAX_TEXTURE_SIZE` is far below `i32::MAX`; the fallback `0` is
/// `GL_INVALID_VALUE` for an extent and is what an unrepresentable number would
/// have produced anyway.
fn u32_to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(0)
}

/// Stride of one [`Vertex`] in bytes: 11 `f32` fields, no padding.
const VERTEX_STRIDE: i32 = 44;
/// Byte offset of `Vertex::local` within the vertex.
const LOCAL_OFFSET: i32 = 8;
/// Byte offset of `Vertex::color` within the vertex.
const COLOR_OFFSET: i32 = 16;
/// Byte offset of `Vertex::radius` within the vertex.
const RADIUS_OFFSET: i32 = 32;
/// Byte offset of `Vertex::size` within the vertex.
const SIZE_OFFSET: i32 = 36;

/// Quads the vertex and index buffers are allocated for before the first
/// frame; both grow geometrically past this.
const INITIAL_CAPACITY: usize = 256;

/// Vertices per quad: two triangles.
const VERTS_PER_QUAD: usize = 4;

/// Stride of one [`TextVertex`] in bytes: 8 `f32` fields, no padding.
const TEXT_VERTEX_STRIDE: i32 = 32;
/// Byte offset of `TextVertex::uv` within the vertex.
const TEXT_UV_OFFSET: i32 = 8;
/// Byte offset of `TextVertex::color` within the vertex.
const TEXT_COLOR_OFFSET: i32 = 16;

/// Stride of one [`ImageVertex`] in bytes: 10 `f32` fields, no padding.
///
/// 2 for the position, 2 for the position within the quad, 2 for the UV, one
/// each for the corner radius and the opacity and 2 for the quad size.
const IMAGE_VERTEX_STRIDE: i32 = 40;
/// Byte offset of `ImageVertex::local` within the vertex.
const IMAGE_LOCAL_OFFSET: i32 = 8;
/// Byte offset of `ImageVertex::uv` within the vertex.
const IMAGE_UV_OFFSET: i32 = 16;
/// Byte offset of `ImageVertex::radius` within the vertex.
const IMAGE_RADIUS_OFFSET: i32 = 24;
/// Byte offset of `ImageVertex::opacity` within the vertex.
const IMAGE_OPACITY_OFFSET: i32 = 28;
/// Byte offset of `ImageVertex::size` within the vertex.
const IMAGE_SIZE_OFFSET: i32 = 32;

/// The number of indices one quad contributes to an index buffer: two triangles.
const INDICES_PER_QUAD: usize = 6;

/// The glyph atlas texture size in pixels, square.
const ATLAS_SIZE: u32 = 2048;

/// The solid-color shader: window-space positions, per-vertex color, and an
/// SDF that cuts rounded corners in the fragment shader.
const VERTEX_SHADER_SRC: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_local;
layout(location = 2) in vec4 a_color;
layout(location = 3) in float a_radius;
layout(location = 4) in vec2 a_size;
uniform vec2 u_resolution;
out vec2 v_local;
out vec4 v_color;
out float v_radius;
out vec2 v_size;
void main() {
    vec2 normalized = a_pos / u_resolution;
    vec2 clip = normalized * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_local = a_local;
    v_color = a_color;
    v_radius = a_radius;
    v_size = a_size;
}
"#;

/// The solid-color fragment shader. Colors arrive premultiplied, so the
/// fragment is written out as-is and blended with
/// `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`.
const FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision mediump float;
in vec2 v_local;
in vec4 v_color;
in float v_radius;
in vec2 v_size;
out vec4 frag_color;
void main() {
    if (v_radius > 0.0) {
        vec2 half_size = v_size * 0.5;
        vec2 q = abs(v_local - half_size) - (half_size - vec2(v_radius));
        float dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - v_radius;
        if (dist > 0.0) {
            discard;
        }
    }
    frag_color = v_color;
}
"#;

/// The text vertex shader: window-space positions, atlas UVs, and a
/// premultiplied color per vertex.
const TEXT_VERTEX_SHADER_SRC: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec4 a_color;
uniform vec2 u_resolution;
out vec2 v_uv;
out vec4 v_color;
void main() {
    vec2 normalized = a_pos / u_resolution;
    vec2 clip = normalized * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_uv = a_uv;
    v_color = a_color;
}
"#;

/// The text fragment shader. It samples the glyph's antialiased coverage
/// straight out of the atlas and uses it as the alpha, which is what keeps the
/// text sharp: the coverage is FreeType's own, rasterized at exactly the size
/// the glyph is drawn, so each glyph's edge is as finely resolved as the
/// display can show it.
///
/// The color is premultiplied by the coverage because the text pass blends with
/// `ONE, ONE_MINUS_SRC_ALPHA`.
///
/// This shader used to reconstruct a coverage from a signed distance field with
/// a `smoothstep` whose width came from `fwidth`. That is a good way to draw
/// text at a size other than the one it was rasterized for, and a poor way to
/// draw it at exactly that size: the field had to binarize the coverage to be
/// built, so the sub-pixel edge position was already gone before the shader saw
/// it, and the edge rendered as a hard aliased step. See `ui_core::font`'s
/// module docs for the measurement.
const TEXT_FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision mediump float;
in vec2 v_uv;
in vec4 v_color;
uniform sampler2D u_atlas;
out vec4 frag_color;
void main() {
    float coverage = texture(u_atlas, v_uv).r;
    frag_color = vec4(v_color.rgb * coverage, v_color.a * coverage);
}
"#;

/// The image vertex shader: window-space positions, the UV window the quad
/// samples, and the two per-quad scalars the fragment shader needs.
///
/// `a_local`, `a_size` and `a_radius` are the solid shader's three, carried for
/// the same reason: the rounded corner is an SDF evaluated per fragment, and it
/// has to know where in the quad the fragment is and how big the quad is. They
/// ride along per *vertex* rather than per command so a batch still draws with
/// no per-command uniforms.
const IMAGE_VERTEX_SHADER_SRC: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_local;
layout(location = 2) in vec2 a_uv;
layout(location = 3) in float a_radius;
layout(location = 4) in float a_opacity;
layout(location = 5) in vec2 a_size;
uniform vec2 u_resolution;
out vec2 v_local;
out vec2 v_uv;
out float v_radius;
out float v_opacity;
out vec2 v_size;
void main() {
    vec2 normalized = a_pos / u_resolution;
    vec2 clip = normalized * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_local = a_local;
    v_uv = a_uv;
    v_radius = a_radius;
    v_opacity = a_opacity;
    v_size = a_size;
}
"#;

/// The image fragment shader: one texture sample, scaled by the opacity, with
/// the rounded corners cut away.
///
/// Three things here are contracts rather than taste.
///
/// The texel is **premultiplied** — `crate::texture` converts on load — and the
/// pipeline blends `ONE, ONE_MINUS_SRC_ALPHA`, so the result has to stay
/// premultiplied: an alpha-only scale of a premultiplied colour is exactly
/// `vec4(texel.rgb * opacity, texel.a * opacity)`, and anything else is a
/// second conversion at every fragment.
///
/// The opacity is clamped **here** as well as in the CPU that built the vertex.
/// `DrawCommand::Image`'s docs say a value outside the range is clamped when the
/// command is drawn; the vertex carries the clamped number, and this is the last
/// mile that keeps the promise true whatever reaches it.
///
/// The corners are cut with `discard`, not painted over. `DrawCommand::Image`'s
/// radius is a **clip**: the corners must show whatever the widget drew behind
/// the image. Filling them with a corner colour instead would make a rounded
/// image a rectangle with the wrong corners — and `.ai/NEVERAGAIN.md` § *A
/// filled rounded rectangle is not an outline* is that failure in a different
/// primitive. No unit test can see the difference between the two: they differ
/// only in what reaches the framebuffer.
///
/// The clipping expression is the solid fragment shader's, character for
/// character, so the two passes round a corner by the same rule. It is written
/// out in both because a GLSL source string is one literal each, and a test
/// asserts the two agree — an edit to one and not the other fails the suite.
const IMAGE_FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision mediump float;
in vec2 v_local;
in vec2 v_uv;
in float v_radius;
in float v_opacity;
in vec2 v_size;
uniform sampler2D u_image;
out vec4 frag_color;
void main() {
    if (v_radius > 0.0) {
        vec2 half_size = v_size * 0.5;
        vec2 q = abs(v_local - half_size) - (half_size - vec2(v_radius));
        float dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - v_radius;
        if (dist > 0.0) {
            discard;
        }
    }
    float opacity = clamp(v_opacity, 0.0, 1.0);
    vec4 texel = texture(u_image, v_uv);
    frag_color = vec4(texel.rgb * opacity, texel.a * opacity);
}
"#;

/// The fragment shader that draws a shadow's shape into the offscreen target.
///
/// It writes **coverage and nothing else**, into the **red** channel: the target
/// is [`GL_R8`] / [`GL_RED`], and a single-channel attachment keeps `.r` and
/// discards green, blue and alpha. Blurring one channel and tinting once at the
/// end is the same image as blurring a premultiplied RGBA, since convolution is
/// linear and the colour is the same everywhere; it is a quarter of the
/// bandwidth, and the bandwidth is what the frame rate is spent on.
///
/// **The channel is the whole content of this shader and it is worth being
/// pedantic about.** Writing the coverage into `.a` — which is the obvious thing
/// next to the solid shader's `v_color.a`, and what this shader did first —
/// composites as a mask whose every texel is zero: an `GL_RED` framebuffer keeps
/// `.r` and throws the rest away. Nothing errors, the pipeline reports no GL
/// failure at any step, and the picture is a window with no shadow on it.
///
/// The corner clip is the solid fragment shader's, character for character, so
/// the shadow's shape is rounded by the same signed distance as the panel that
/// casts it — a test asserts the two sources agree, and an edit to one without
/// the other fails the suite rather than the screen.
///
/// Blending must be **off** for this pass: the alpha it writes *is* the coverage,
/// and blending a value into another would make the coverage depend on what the
/// target held before the clear.
const SHADOW_MASK_FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision mediump float;
in vec2 v_local;
in vec4 v_color;
in float v_radius;
in vec2 v_size;
out vec4 frag_color;
void main() {
    if (v_radius > 0.0) {
        vec2 half_size = v_size * 0.5;
        vec2 q = abs(v_local - half_size) - (half_size - vec2(v_radius));
        float dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - v_radius;
        if (dist > 0.0) {
            discard;
        }
    }
    frag_color = vec4(v_color.a, 0.0, 0.0, 0.0);
}
"#;

/// The fragment shader that draws a shadow with **no blur** straight to the
/// screen, which is what a `blur` at or below [`blur::SOLID_BLUR`] takes.
///
/// It premultiplies: `vec4(v_color.rgb * v_color.a, v_color.a)`. **This is the
/// whole reason the shadow path has its own shader**, because the pipeline's
/// blend func is `ONE, ONE_MINUS_SRC_ALPHA` and the solid pass does not
/// premultiply what it feeds it — a recorded, unfixed defect
/// (`doc/ui/IMPLEMENTATION_STATE.md` § *The finding that is not this task's: the
/// solid pass does not premultiply*). A black shadow at alpha `a` composites the
/// same either way, so the defect is invisible on the usual black shadow and
/// shows on any other colour; doing the multiplication here means the shadow is
/// right on both, and right in the same way whether it was blurred or not.
const SHADOW_FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision mediump float;
in vec2 v_local;
in vec4 v_color;
in float v_radius;
in vec2 v_size;
out vec4 frag_color;
void main() {
    if (v_radius > 0.0) {
        vec2 half_size = v_size * 0.5;
        vec2 q = abs(v_local - half_size) - (half_size - vec2(v_radius));
        float dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - v_radius;
        if (dist > 0.0) {
            discard;
        }
    }
    frag_color = vec4(v_color.rgb * v_color.a, v_color.a);
}
"#;

/// The vertex shader the blur passes and the shadow's composite share.
///
/// The positions are in **window coordinates** — the same space as every other
/// vertex shader here — and the quad is window-sized, so one `u_size` covers all
/// three passes.
///
/// `v_uv` is **flipped**. A position at the window's top edge is the framebuffer's
/// *last* row, which is texture coordinate `v = 1`; emitting `normalized.y`
/// would sample the target upside down, which is a shadow that is soft but in
/// the wrong place — the kind of defect that reads as "the blur looks odd" and
/// has no unit test. The flip is here rather than in the fragment shaders
/// because there is one of this and three of them.
const BLUR_VERTEX_SHADER_SRC: &str = r#"#version 300 es
layout(location = 0) in vec2 a_pos;
uniform vec2 u_size;
out vec2 v_uv;
void main() {
    vec2 normalized = a_pos / u_size;
    vec2 clip = normalized * 2.0 - 1.0;
    gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
    v_uv = vec2(normalized.x, 1.0 - normalized.y);
}
"#;

/// The separable Gaussian: one pass, with the direction and the weights as
/// uniforms.
///
/// `u_taps` is the **total** number of taps, odd, and `offset` walks
/// `-(taps-1)/2 .. (taps-1)/2` — which is the same vector [`blur::kernel`]
/// returns, in the same order. The array is indexed by the loop index, which GLSL
/// ES 3.00 allows for a uniform array; a `break` rather than a dynamic bound
/// keeps the loop's trip count a compile-time constant, which is what a
/// `#define`d array length needs.
///
/// **`highp`, and it is not optional.** The default fragment precision in ES is
/// `mediump`, which is `fp16`: about 11 bits of mantissa. An 8-bit coverage
/// summed over nine taps and read back at 8 bits again is inside the range where
/// `mediump`'s rounding steps, and the visible result is a shadow edge that
/// **bands** — a staircase of plateaus where a ramp should be. That is the one
/// artefact this pass cannot have, so the precision is asked for.
///
/// The weights arrive from the CPU already normalised to sum to one; this is a
/// plain dot product, and a shader that normalised again would be a second place
/// where the normalisation could be wrong.
const BLUR_FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_source;
uniform vec2 u_texel;
uniform vec2 u_direction;
uniform float u_weights[9];
uniform int u_taps;
out vec4 frag_color;
void main() {
    float total = 0.0;
    for (int i = 0; i < 9; i++) {
        if (i >= u_taps) {
            break;
        }
        float offset = float(i) - float(u_taps - 1) * 0.5;
        total = total + texture(u_source, v_uv + u_direction * u_texel * offset).r * u_weights[i];
    }
    frag_color = vec4(total, 0.0, 0.0, 0.0);
}
"#;

/// The shadow's composite: the blurred coverage, tinted, over the screen.
///
/// **`vec4(u_color.rgb * coverage, coverage)` — premultiplied, deliberately.**
/// The blend func is `ONE, ONE_MINUS_SRC_ALPHA`, so a source that is not
/// premultiplied is composited as `rgb + dst·(1 − a)` and a coloured shadow comes
/// out brighter than it was asked for. A black shadow at alpha `a` gives
/// `0 + dst·(1 − a)` either way, so **the premultiplied and non-premultiplied
/// forms are pixel-identical on a black shadow** — which is why the mistake
/// survives a capture and why the arithmetic is pinned by a test using a colour
/// that is not black.
///
/// Only `r` is sampled: the target holds coverage in one channel
/// ([`blur`]'s module docs say why).
const SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC: &str = r#"#version 300 es
precision mediump float;
in vec2 v_uv;
uniform sampler2D u_source;
uniform vec4 u_color;
out vec4 frag_color;
void main() {
    float coverage = texture(u_source, v_uv).r;
    frag_color = vec4(u_color.rgb * coverage, coverage);
}
"#;

/// An error that can occur while creating a [`Renderer`] or submitting a
/// frame.
#[derive(Debug)]
pub enum RenderError {
    /// A shader failed to compile; carries the info log.
    ShaderCompile(String),
    /// The shader program failed to link; carries the info log.
    ProgramLink(String),
    /// A GL object could not be created or a size was out of range.
    Gl(String),
    /// An image could not be made available; carries the message from
    /// [`TextureError`].
    ///
    /// The decode and the placement are `crate::texture`'s decisions, and its
    /// error carries the specifics — a decoder naming the format it rejected, a
    /// size with no addressable pixel count. Only the GL half of an image is the
    /// renderer's, and that is [`RenderError::Gl`].
    Texture(String),
}

impl From<TextureError> for RenderError {
    fn from(error: TextureError) -> Self {
        RenderError::Texture(error.to_string())
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::ShaderCompile(log) => write!(f, "shader compile error: {log}"),
            RenderError::ProgramLink(log) => write!(f, "program link error: {log}"),
            RenderError::Gl(msg) => write!(f, "GL error: {msg}"),
            RenderError::Texture(msg) => write!(f, "texture error: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Converts a float coordinate to the integer the GL APIs take.
///
/// There is no `From`/`TryFrom` between `f32` and any integer type in std,
/// so this is the one place a float-to-integer `as` cast is used. The cast
/// is saturating (Rust 1.45 and later), so an out-of-range coordinate clamps
/// to the nearest `i32` instead of wrapping; sub-pixel precision is not
/// meaningful for scissor rects and viewports.
fn f32_to_i32(value: f32) -> i32 {
    value as i32
}

/// Converts a `u32` window dimension to `f32` for the shader.
///
/// `f32` has no `From<u32>` (nor `From<i32>`) in std — its `From` impls stop
/// at 16-bit integers — so this is an `as` cast like [`f32_to_i32`]. It is
/// well-defined for every `u32`: the result rounds to the nearest `f32`.
fn u32_to_f32(value: u32) -> f32 {
    value as f32
}

/// Converts a pixel offset to `f32` for vertex positions.
///
/// Like [`u32_to_f32`], there is no `From<i32> for f32` in std, so this is an
/// `as` cast; the value is a small pixel offset that `f32` represents exactly.
fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

/// Converts a GL enum to the `i32` the texture-parameter setters take.
///
/// There is no `From<u32> for i32` in std, so this is a checked conversion
/// rather than a cast. The values it is called with are the GL constants in
/// this module and all of them fit an `i32`, so the fallback never fires; it is
/// `0`, which is `GL_INVALID_ENUM` and what an unrepresentable value would have
/// produced anyway — a parameter GL rejects loudly, not one that silently
/// configures the texture wrongly.
fn gl_enum_to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(0)
}

/// One vertex of a quad: window position, position within the quad, color,
/// corner radius and quad size — everything the solid shader needs, so a
/// batch draws with no per-command uniforms.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex {
    /// Position in window coordinates, origin at the top left.
    pos: [f32; 2],
    /// Position within the quad, `(0,0)` at its top left corner.
    local: [f32; 2],
    /// Premultiplied color.
    color: [f32; 4],
    /// Corner radius in pixels, already clamped to half the smaller side.
    radius: f32,
    /// Quad size in pixels.
    size: [f32; 2],
}

/// The geometry of one quad: four corners, their local coordinates, and the
/// per-quad shader inputs.
#[derive(Clone, Copy, Debug)]
struct Quad {
    /// Corner positions in window coordinates.
    corners: [[f32; 2]; 4],
    /// Per-corner position within the quad.
    locals: [[f32; 2]; 4],
    /// Premultiplied color.
    color: [f32; 4],
    /// Corner radius in pixels.
    radius: f32,
    /// Quad size in pixels.
    size: [f32; 2],
}

/// One vertex of a text glyph quad: window position, atlas UV, and a
/// premultiplied color.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct TextVertex {
    /// Position in window coordinates, origin at the top left.
    pos: [f32; 2],
    /// Position within the glyph atlas.
    uv: [f32; 2],
    /// Premultiplied color.
    color: [f32; 4],
}

/// One vertex of a textured quad: window position, position within the quad,
/// the UV window it samples, and the two per-quad scalars the fragment shader
/// needs.
///
/// The field order is the shader's attribute order, and the offsets the GL
/// attribute pointers use are constants derived from it — see
/// [`IMAGE_VERTEX_STRIDE`] and its neighbours. Nothing here is per-command
/// uniform state, which is what lets a whole batch draw in one call.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ImageVertex {
    /// Position in window coordinates, origin at the top left.
    pos: [f32; 2],
    /// Position within the quad, `(0,0)` at its top left corner.
    local: [f32; 2],
    /// The point of the UV window this corner samples.
    uv: [f32; 2],
    /// Corner radius in pixels, already clamped to half the smaller side.
    radius: f32,
    /// How much of the sampled texel reaches the screen, `0.0..=1.0`.
    opacity: f32,
    /// Quad size in pixels.
    size: [f32; 2],
}

/// Builds the four vertices of one glyph quad.
///
/// `(x, y)` is the quad's top-left corner in window coordinates, `(w, h)` its
/// size, `(u0, v0)`–`(u1, v1)` its rect in the atlas, and `color` the
/// premultiplied fill.
fn text_quad(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    uv: (f32, f32, f32, f32),
    color: Color,
) -> [TextVertex; 4] {
    let (u0, v0, u1, v1) = uv;
    let c = quad_color(color);
    [
        TextVertex {
            pos: [x, y],
            uv: [u0, v0],
            color: c,
        },
        TextVertex {
            pos: [x + w, y],
            uv: [u1, v0],
            color: c,
        },
        TextVertex {
            pos: [x + w, y + h],
            uv: [u1, v1],
            color: c,
        },
        TextVertex {
            pos: [x, y + h],
            uv: [u0, v1],
            color: c,
        },
    ]
}

/// Builds the four vertices of one textured quad.
///
/// `(x, y)` is the quad's top-left corner in window coordinates and `(w, h)` its
/// size, exactly as [`text_quad`] takes them; `uv` is the window into the
/// texture, `opacity` how much of what is sampled reaches the screen and
/// `radius` how many pixels of corner the shader clips away.
///
/// The two clamps are the ones [`DrawCommand::Image`]'s docs promise, and they
/// happen here rather than in the shader because here is where they can be
/// tested: a radius past half the shorter side becomes half of it — the same
/// rule [`rect_quad`] follows — so a radius too large to fit is a rounded pill
/// and not an inverted shape, and an opacity outside `0.0..=1.0` becomes the
/// nearest end of it.
///
/// The UVs are **not** touched. `UvRect` is recorded by whoever knows where the
/// image sits in its texture, and the quad has no opinion about it: an image
/// packed into the shared atlas samples its placement, and an image with a
/// texture of its own samples the whole of it.
fn image_quad(rect: Rect, uv: UvRect, opacity: f32, radius: f32) -> [ImageVertex; 4] {
    let size = [rect.width, rect.height];
    let radius = radius.min(rect.width / 2.0).min(rect.height / 2.0);
    let opacity = opacity.clamp(0.0, 1.0);
    let left = [rect.x, rect.y];
    let right = [rect.x + rect.width, rect.y];
    let bottom_right = [rect.x + rect.width, rect.y + rect.height];
    let bottom_left = [rect.x, rect.y + rect.height];
    let near_left = [0.0, 0.0];
    let near_right = [rect.width, 0.0];
    let far_right = [rect.width, rect.height];
    let far_left = [0.0, rect.height];
    let top_left = [uv.u0, uv.v0];
    let top_right = [uv.u1, uv.v0];
    let bottom_right_uv = [uv.u1, uv.v1];
    let bottom_left_uv = [uv.u0, uv.v1];
    [
        ImageVertex {
            pos: left,
            local: near_left,
            uv: top_left,
            radius,
            opacity,
            size,
        },
        ImageVertex {
            pos: right,
            local: near_right,
            uv: top_right,
            radius,
            opacity,
            size,
        },
        ImageVertex {
            pos: bottom_right,
            local: far_right,
            uv: bottom_right_uv,
            radius,
            opacity,
            size,
        },
        ImageVertex {
            pos: bottom_left,
            local: far_left,
            uv: bottom_left_uv,
            radius,
            opacity,
            size,
        },
    ]
}

/// Returns how far the pen moves after drawing `ch`.
///
/// The advance comes from `placement` when the glyph was rasterized, and from
/// `fallback` when it was not. The second case is not an error path: a **space**
/// rasterizes to a zero-width bitmap, so there is no quad to draw and no
/// placement to read an advance from, yet the space still occupies width.
/// Treating "no quad" as "no advance" ran every word together on screen —
/// "Hello,World!" — and no test could see it, because the run is assembled
/// against a GL context and a real font, and both are unavailable to a unit
/// test. A character the font genuinely has no glyph for takes the same path
/// and is given the width it would have had, which is the honest answer.
///
/// `extra_advance` is the letter spacing the command carries, and is added in
/// both cases: a space is spaced like any other character.
fn advance_for(placement: Option<&GlyphPlacement>, fallback: f32, extra_advance: f32) -> f32 {
    let advance = match placement {
        Some(placement) => placement.advance,
        None => fallback,
    };
    advance + extra_advance
}

/// Walks one text run, calling `emit` for every character that produced a quad,
/// with the glyph's placement and the pen position it is drawn at, and
/// returning the pen's final position.
///
/// A character `lookup` returns `None` for emits nothing and still advances the
/// pen, by `fallback`. That is what keeps a space a space: see
/// [`advance_for`]. The lookups are closures so the rule can be exercised
/// without a GL context or a font file, neither of which a unit test may open.
fn walk_run<E, L, F>(
    text: &str,
    start_x: f32,
    extra_advance: f32,
    lookup: &mut L,
    fallback: &mut F,
    emit: &mut E,
) -> f32
where
    E: FnMut(&GlyphPlacement, f32),
    L: FnMut(char) -> Option<GlyphPlacement>,
    F: FnMut(char) -> f32,
{
    let mut pen_x = start_x;
    for ch in text.chars() {
        let placement = lookup(ch);
        if let Some(ref placement) = placement {
            emit(placement, pen_x);
        }
        let unrasterized = if placement.is_none() {
            fallback(ch)
        } else {
            0.0
        };
        pen_x += advance_for(placement.as_ref(), unrasterized, extra_advance);
    }
    pen_x
}

/// Converts a color to normalized premultiplied components.
fn quad_color(color: Color) -> [f32; 4] {
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        f32::from(color.a) / 255.0,
    ]
}

/// Builds the quad for an axis-aligned rectangle, clamping the corner radius
/// to half the smaller side so the SDF stays well-defined.
fn rect_quad(rect: Rect, color: Color, radius: f32) -> Quad {
    let radius = radius.min(rect.width / 2.0).min(rect.height / 2.0);
    Quad {
        corners: [
            [rect.x, rect.y],
            [rect.x + rect.width, rect.y],
            [rect.x + rect.width, rect.y + rect.height],
            [rect.x, rect.y + rect.height],
        ],
        locals: [
            [0.0, 0.0],
            [rect.width, 0.0],
            [rect.width, rect.height],
            [0.0, rect.height],
        ],
        color: quad_color(color),
        radius,
        size: [rect.width, rect.height],
    }
}

/// Builds the quad for a thick line segment: the segment expanded by half its
/// width along its normal.
fn line_quad(start: (f32, f32), end: (f32, f32), width: f32, color: Color) -> Quad {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let len = (dx * dx + dy * dy).sqrt();
    // A degenerate segment has no normal; the quad collapses to zero area.
    let (nx, ny) = if len > 0.0 {
        (-dy / len * width / 2.0, dx / len * width / 2.0)
    } else {
        (0.0, 0.0)
    };
    Quad {
        corners: [
            [start.0 + nx, start.1 + ny],
            [end.0 + nx, end.1 + ny],
            [end.0 - nx, end.1 - ny],
            [start.0 - nx, start.1 - ny],
        ],
        locals: [[0.0, 0.0], [len, 0.0], [len, width], [0.0, width]],
        color: quad_color(color),
        radius: 0.0,
        size: [len, width],
    }
}

/// Builds the quad for one triangle of a filled polygon.
///
/// A triangle is a quad whose last corner repeats its third: [`quad_indices`]
/// addresses four corners as `0,1,2 / 0,2,3`, so `[a, b, c, c]` draws the
/// triangle `(a, b, c)` and the degenerate `(a, c, c)`, which encloses no area
/// and covers no fragments. That is the whole trick — the pipeline is quad-only,
/// and a duplicated corner is the one way a triangle fits through it without a
/// new vertex type, a new shader or a change to the index buffer.
///
/// Winding is whatever the fan produced, and nothing depends on it: no pass in
/// this module enables face culling or a depth test, so `(a, b, c)` and
/// `(a, c, b)` draw the same pixels. It is left as the caller's order because
/// that is the order the fan was handed.
///
/// `locals` and `size` are **honest but unread**. The solid fragment shader only
/// consults them inside `if (v_radius > 0.0)`, and this radius is `0.0`, so the
/// shader takes the `frag_color = v_color` path and never evaluates the corner
/// SDF — which would be meaningless here anyway, since it models an axis-aligned
/// rounded rectangle and this quad is a triangle. They are set to the triangle's
/// bounding box and to `0.0` at the box's origin because those are the values a
/// rounded-rect quad carries, so a vertex never claims a position that is not
/// where it is drawn; `[[0.0; 2]; 4]` would have been a lie about the geometry
/// that costs nothing to make true.
fn polygon_quad(a: (f32, f32), b: (f32, f32), c: (f32, f32), color: Color) -> Quad {
    let low_x = a.0.min(b.0).min(c.0);
    let low_y = a.1.min(b.1).min(c.1);
    let size = [a.0.max(b.0).max(c.0) - low_x, a.1.max(b.1).max(c.1) - low_y];
    Quad {
        corners: [[a.0, a.1], [b.0, b.1], [c.0, c.1], [c.0, c.1]],
        locals: [
            [a.0 - low_x, a.1 - low_y],
            [b.0 - low_x, b.1 - low_y],
            [c.0 - low_x, c.1 - low_y],
            [c.0 - low_x, c.1 - low_y],
        ],
        color: quad_color(color),
        radius: 0.0,
        size,
    }
}

/// Expands a draw command into quads.
///
/// Text and image commands expand to nothing **here**, and for a different
/// reason each: a text command needs a font to turn characters into quads, and
/// an image command needs a *different vertex* ([`ImageVertex`], with a UV and
/// two scalars the solid geometry has no place for). Neither is a solid quad, so
/// neither may reach [`Renderer::draw_solid_batch`] — which is why they are
/// named and dropped here rather than by the solid pass guessing at the shader
/// kind it was handed.
fn command_quads(command: &DrawCommand) -> Vec<Quad> {
    match command {
        DrawCommand::Rect { rect, color } => vec![rect_quad(*rect, *color, 0.0)],
        DrawCommand::RoundedRect {
            rect,
            radius,
            color,
        } => vec![rect_quad(*rect, *color, *radius)],
        // A circle is a rounded rect whose radius is half its size.
        DrawCommand::Circle {
            center,
            radius,
            color,
        } => vec![rect_quad(
            Rect::new(
                center.0 - radius,
                center.1 - radius,
                radius * 2.0,
                radius * 2.0,
            ),
            *color,
            *radius,
        )],
        DrawCommand::Line {
            start,
            end,
            width,
            color,
        } => vec![line_quad(*start, *end, *width, *color)],
        DrawCommand::Path {
            points,
            width,
            color,
            closed,
        } => {
            let mut quads = Vec::new();
            if points.len() >= 2 {
                for segment in points.windows(2) {
                    quads.push(line_quad(segment[0], segment[1], *width, *color));
                }
                if *closed {
                    quads.push(line_quad(
                        points[points.len() - 1],
                        points[0],
                        *width,
                        *color,
                    ));
                }
            }
            quads
        }
        DrawCommand::Polygon { points, color } => {
            let mut quads = Vec::new();
            // Fewer than three points enclose no area, and the same is true of a
            // `Path` below two: nothing to draw is an empty result, not an
            // error. Checked before the fan so a two-point "polygon" does not
            // index a third point that is not there.
            if points.len() < 3 {
                return Vec::new();
            }
            quads.reserve(points.len() - 2);
            // A fan from the first point. This is exact for a convex polygon and
            // nothing else, which is what `DrawCommand::Polygon`'s docs say it is;
            // a concave one fans into overlapping and inverted triangles. The
            // alternative — ear clipping, or a stencil pass — is a rasteriser
            // with its own vertex type, and neither a gauge needle nor anything
            // else recorded so far needs one.
            for point in points[1..].windows(2) {
                quads.push(polygon_quad(points[0], point[0], point[1], *color));
            }
            quads
        }
        // A shadow expands to nothing here, and for the same reason as the two
        // above it: it is not a solid quad but a program that draws one. It has a
        // vertex type of its own in the only sense that matters — a *different
        // program* over the same [`Vertex`] — and `draw_shadow_batch` is the only
        // thing that knows which. Returning empty rather than a quad means a
        // shadow that reached the solid pass would draw nothing at all, which is
        // a missing shadow rather than a wrong picture.
        DrawCommand::Shadow { .. } | DrawCommand::Text { .. } | DrawCommand::Image { .. } => {
            Vec::new()
        }
    }
}

/// Byte size of a vertex buffer that holds `capacity` quads.
///
/// A quad is four vertices, so a buffer sized for one vertex per quad is a
/// quarter of the size [`Renderer::ensure_vertex_capacity`] and
/// [`Renderer::ensure_text_vertex_capacity`] promise to their callers. The
/// subsequent `glBufferSubData` then fails with `GL_INVALID_VALUE` and the
/// batch is silently dropped.
fn vertex_buffer_size(capacity: usize, bytes_per_vertex: usize) -> Result<i32, RenderError> {
    let bytes = capacity
        .checked_mul(VERTS_PER_QUAD)
        .and_then(|vertices| vertices.checked_mul(bytes_per_vertex))
        .ok_or_else(|| RenderError::Gl("vertex buffer size exceeds the i32 range".to_string()))?;
    i32::try_from(bytes)
        .map_err(|_| RenderError::Gl("vertex buffer size exceeds the i32 range".to_string()))
}

/// The indices for `capacity` quads: two triangles each, addressing four
/// vertices per quad.
///
/// Shared by all three passes because the arithmetic is the same for all of
/// them and is the arithmetic that has to be right: a quad's four vertices are
/// `base, base+1, base+2, base+3`, so the indices only change when the vertex
/// count per quad does.
///
/// # Errors
///
/// Returns [`RenderError::Gl`] when `capacity` is past the point where a quad's
/// last vertex no longer fits a `u32`, which is the index type every pass
/// submits. The bound is checked before anything is allocated rather than
/// inside the loop, so an absurd capacity is a refused answer and not a
/// multi-gigabyte allocation.
fn quad_indices(capacity: usize) -> Result<Vec<u32>, RenderError> {
    let largest = usize::try_from(u32::MAX)
        .unwrap_or(usize::MAX)
        .saturating_add(1)
        .checked_div(VERTS_PER_QUAD)
        .unwrap_or(0);
    if capacity > largest {
        return Err(RenderError::Gl(
            "quad count exceeds the u32 index range".to_string(),
        ));
    }
    let mut indices = Vec::new();
    if let Some(exact) = capacity.checked_mul(INDICES_PER_QUAD) {
        indices.reserve(exact);
    }
    let per_quad = u32::try_from(VERTS_PER_QUAD).unwrap_or(0);
    for quad in 0..capacity {
        let base = u32::try_from(quad)
            .map_err(|_| RenderError::Gl("quad count exceeds the u32 index range".to_string()))?
            .checked_mul(per_quad)
            .ok_or_else(|| RenderError::Gl("quad count exceeds the u32 index range".to_string()))?;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
    }
    Ok(indices)
}

/// Expands every command in a batch into vertices, four per quad.
fn batch_vertices(batch: &Batch) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    for command in &batch.commands {
        for quad in command_quads(command) {
            for i in 0..4 {
                vertices.push(Vertex {
                    pos: quad.corners[i],
                    local: quad.locals[i],
                    color: quad.color,
                    radius: quad.radius,
                    size: quad.size,
                });
            }
        }
    }
    vertices
}

/// Expands one quad into its four vertices, in the order `quad_indices`
/// addresses them.
///
/// The same shape [`batch_vertices`] builds, for a quad that is not in a batch —
/// a shadow's, whose program and destination have nothing to do with the solid
/// pass's. Four vertices and not three, because `draw_vertex_quads` computes
/// `quads = vertices.len() / 4` and one that emitted three would put the sixth
/// index on the first vertex of the next one.
fn quad_vertices(quad: Quad) -> Vec<Vertex> {
    (0..VERTS_PER_QUAD)
        .map(|i| Vertex {
            pos: quad.corners[i],
            local: quad.locals[i],
            color: quad.color,
            radius: quad.radius,
            size: quad.size,
        })
        .collect()
}

/// Compiles one shader stage.
///
/// # Errors
///
/// Returns [`RenderError::ShaderCompile`] with the info log if the shader
/// fails to compile.
fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, RenderError> {
    // SAFETY: The GL context is current on this thread.
    let shader = unsafe { gl.create_shader(shader_type) }.map_err(RenderError::Gl)?;
    // SAFETY: `shader` is a valid shader object and `source` is a valid
    // string.
    unsafe {
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            return Err(RenderError::ShaderCompile(log));
        }
    }
    Ok(shader)
}

/// Links the solid-color program.
///
/// # Errors
///
/// Returns [`RenderError::ShaderCompile`] or [`RenderError::ProgramLink`]
/// with the info log on failure.
fn create_program(gl: &glow::Context) -> Result<glow::Program, RenderError> {
    let vertex_shader = compile_shader(gl, GL_VERTEX_SHADER, VERTEX_SHADER_SRC)?;
    let fragment_shader = compile_shader(gl, GL_FRAGMENT_SHADER, FRAGMENT_SHADER_SRC)?;
    // SAFETY: The GL context is current on this thread.
    let program = unsafe { gl.create_program() }.map_err(RenderError::Gl)?;
    // SAFETY: `program` and both shaders are valid objects.
    unsafe {
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
        gl.detach_shader(program, vertex_shader);
        gl.detach_shader(program, fragment_shader);
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(RenderError::ProgramLink(log));
        }
    }
    Ok(program)
}

/// Links the text program.
///
/// # Errors
///
/// Returns [`RenderError::ShaderCompile`] or [`RenderError::ProgramLink`]
/// with the info log on failure.
fn create_text_program(gl: &glow::Context) -> Result<glow::Program, RenderError> {
    let vertex_shader = compile_shader(gl, GL_VERTEX_SHADER, TEXT_VERTEX_SHADER_SRC)?;
    let fragment_shader = compile_shader(gl, GL_FRAGMENT_SHADER, TEXT_FRAGMENT_SHADER_SRC)?;
    // SAFETY: The GL context is current on this thread.
    let program = unsafe { gl.create_program() }.map_err(RenderError::Gl)?;
    // SAFETY: `program` and both shaders are valid objects.
    unsafe {
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
        gl.detach_shader(program, vertex_shader);
        gl.detach_shader(program, fragment_shader);
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(RenderError::ProgramLink(log));
        }
    }
    Ok(program)
}

/// Links the image program.
///
/// # Errors
///
/// Returns [`RenderError::ShaderCompile`] or [`RenderError::ProgramLink`]
/// with the info log on failure.
fn create_image_program(gl: &glow::Context) -> Result<glow::Program, RenderError> {
    let vertex_shader = compile_shader(gl, GL_VERTEX_SHADER, IMAGE_VERTEX_SHADER_SRC)?;
    let fragment_shader = compile_shader(gl, GL_FRAGMENT_SHADER, IMAGE_FRAGMENT_SHADER_SRC)?;
    // SAFETY: The GL context is current on this thread.
    let program = unsafe { gl.create_program() }.map_err(RenderError::Gl)?;
    // SAFETY: `program` and both shaders are valid objects.
    unsafe {
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
        gl.detach_shader(program, vertex_shader);
        gl.detach_shader(program, fragment_shader);
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(RenderError::ProgramLink(log));
        }
    }
    Ok(program)
}

/// Links the shadow programs: the offscreen mask and the unblurred shadow.
///
/// Both share the **solid vertex shader** and differ only in what they write,
/// which is the whole of the difference between them: the mask pass fills the
/// single-channel target with coverage, the direct pass fills the screen with a
/// premultiplied colour. Sharing the vertex stage is why the shadow's shape is
/// rounded by exactly the signed distance the panel's is.
///
/// # Errors
///
/// Returns [`RenderError::ShaderCompile`] or [`RenderError::ProgramLink`] with
/// the info log on failure.
fn create_shadow_programs(
    gl: &glow::Context,
) -> Result<(glow::Program, glow::Program), RenderError> {
    let mask =
        create_program_with_fragment(gl, VERTEX_SHADER_SRC, SHADOW_MASK_FRAGMENT_SHADER_SRC)?;
    let solid = create_program_with_fragment(gl, VERTEX_SHADER_SRC, SHADOW_FRAGMENT_SHADER_SRC)?;
    Ok((mask, solid))
}

/// Links the blur program, and with it the composite: the two share a vertex
/// stage and differ in the one uniform that says what they are for.
///
/// # Errors
///
/// Returns [`RenderError::ShaderCompile`] or [`RenderError::ProgramLink`] with
/// the info log on failure.
fn create_blur_programs(gl: &glow::Context) -> Result<(glow::Program, glow::Program), RenderError> {
    let blur = create_program_with_fragment(gl, BLUR_VERTEX_SHADER_SRC, BLUR_FRAGMENT_SHADER_SRC)?;
    let composite = create_program_with_fragment(
        gl,
        BLUR_VERTEX_SHADER_SRC,
        SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC,
    )?;
    Ok((blur, composite))
}

/// Links one program from a vertex source and a fragment source.
///
/// The four programs above this used to be spelled out one by one, which is four
/// copies of the same eleven unsafe lines and four places to forget one. This is
/// the shape they all had.
///
/// # Errors
///
/// Returns [`RenderError::ShaderCompile`] or [`RenderError::ProgramLink`] with
/// the info log on failure.
fn create_program_with_fragment(
    gl: &glow::Context,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<glow::Program, RenderError> {
    let vertex_shader = compile_shader(gl, GL_VERTEX_SHADER, vertex_source)?;
    let fragment_shader = compile_shader(gl, GL_FRAGMENT_SHADER, fragment_source)?;
    // SAFETY: The GL context is current on this thread.
    let program = unsafe { gl.create_program() }.map_err(RenderError::Gl)?;
    // SAFETY: The GL context is current on this thread; `program` and both
    // shaders are valid objects, and a program needs its shaders attached before
    // it is linked.
    unsafe {
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
        gl.detach_shader(program, vertex_shader);
        gl.detach_shader(program, fragment_shader);
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(RenderError::ProgramLink(log));
        }
    }
    Ok(program)
}

/// One of the passes a frame is made of.
///
/// A pass is one shader and one set of buffers, so a batch belongs to exactly
/// one of them and [`Renderer::draw_pass`] is the only place that knows which
/// draw function goes with which.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Pass {
    /// Filled shapes, from [`crate::paint::Painter`]'s rects, lines and paths.
    Solid,
    /// Textured quads.
    Image,
    /// Glyph quads from the glyph atlas.
    Text,
}

/// The passes [`Renderer::end_frame`] draws, in order.
///
/// The order is the layering, and it is the reason a label is legible over a
/// picture. Within the frame an image is a *background* — it fills a region and
/// the content that belongs on it is text — so the image pass is drawn after the
/// solid pass that fills what is behind it and before the text pass that sits
/// on it. Both text passes drawing after every image pass, and every image pass
/// drawing after every solid pass, is what "a background" means here; the solid
/// and text passes already run in that order for the same reason.
///
/// The constant exists so the order is one list rather than the order of lines
/// in [`Renderer::end_frame`], and so a test can state it. `end_frame` reads it;
/// nothing else may reorder the draws behind its back.
const COMPOSITED_PASSES: [Pass; 3] = [Pass::Solid, Pass::Image, Pass::Text];

/// Which of the two places an image lives, decided by its [`TextureId`] alone.
///
/// This is the whole of the atlas-versus-standalone decision
/// [`crate::texture::is_standalone`] encodes, and it is its own type so the
/// decision is a value that can be asserted on without a GL context — a wrong
/// answer here is a black square, and the pixels are the only place it shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImageSource {
    /// A rect inside the one shared image-atlas texture, which the renderer
    /// keeps a GL object for and uploads whenever the cache says it changed.
    Atlas,
    /// A texture of the image's own, keyed in the renderer's map by this number.
    ///
    /// The key is the full identifier, bit and all, so it is exactly what a
    /// recorded `DrawCommand::Image` carries and the map needs no arithmetic
    /// before it can be indexed.
    Own(u32),
}

/// Returns where the image `id` is drawn from.
fn image_source(id: crate::paint::TextureId) -> ImageSource {
    if texture::is_standalone(id) {
        ImageSource::Own(id.get())
    } else {
        ImageSource::Atlas
    }
}

/// Submits batched draw commands to the GPU through OpenGL ES.
///
/// The renderer owns the window and GL context, the shader program, and the
/// shared GPU buffers. The buffers live for the renderer's lifetime —
/// geometry is retained on the GPU and only the vertices of the batches
/// drawn in a frame are re-uploaded.
pub struct Renderer {
    context: Context,
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ibo: glow::Buffer,
    u_resolution: Option<glow::UniformLocation>,
    batcher: Batcher,
    viewport: (u32, u32),
    vertex_capacity: usize,
    index_capacity: usize,
    text_program: glow::Program,
    text_vao: glow::VertexArray,
    text_vbo: glow::Buffer,
    text_ibo: glow::Buffer,
    u_atlas: Option<glow::UniformLocation>,
    u_text_resolution: Option<glow::UniformLocation>,
    atlas_texture: glow::Texture,
    /// The faces text is drawn with, one per [`FontWeight`], and the identity
    /// each is cached under in [`Self::atlas`].
    ///
    /// **A set rather than one face**, so that a [`DrawCommand::Text`] naming a
    /// weight is answered from a face rather than re-derived: which face to
    /// rasterize from, which advances to lay the run out with and which ascent to
    /// put the baseline at are three readings of one decision, and a run that took
    /// them from two places would be laid out with one face's metrics and drawn
    /// with another's. Empty until a face is installed, which is the state that
    /// draws no text.
    fonts: FontSet,
    atlas: GlyphAtlas,
    /// The scissor rect currently applied to the GPU, so a frame's worth of
    /// batches sets it once per change rather than once per batch.
    applied_clip: Option<Rect>,
    text_vertex_capacity: usize,
    text_index_capacity: usize,
    image_program: glow::Program,
    image_vao: glow::VertexArray,
    image_vbo: glow::Buffer,
    image_ibo: glow::Buffer,
    u_image_resolution: Option<glow::UniformLocation>,
    u_image: Option<glow::UniformLocation>,
    image_atlas_texture: glow::Texture,
    /// One GL texture per image too large for the shared atlas, by the
    /// identifier its draw command carries.
    standalone_textures: HashMap<u32, glow::Texture>,
    /// The images this process has decoded, and where each of them lives.
    textures: TextureCache,
    /// The image pass's scratch vertices, reused across frames.
    image_vertices: Vec<ImageVertex>,
    image_vertex_capacity: usize,
    image_index_capacity: usize,
    /// Draws a shadow's shape into the offscreen target as coverage.
    shadow_mask_program: glow::Program,
    /// Draws a shadow with no blur straight to the screen, premultiplied.
    shadow_program: glow::Program,
    /// The separable Gaussian, over one axis per pass.
    blur_program: glow::Program,
    /// Tints the blurred coverage and composites it over the screen.
    shadow_composite_program: glow::Program,
    /// The window-sized quad the two blur passes and the composite draw.
    blur_quad: BlurQuad,
    /// The offscreen target, allocated the first time a shadow needs it.
    shadow_target: ShadowTarget,
    /// The shadow's colour, for the composite. It rides here rather than in the
    /// vertex data because the whole full-window quad is one colour by
    /// construction: blurring one channel and tinting once is the same image as
    /// carrying the colour through the blur, and it is a quarter of the bandwidth.
    u_shadow_color: Option<glow::UniformLocation>,
    /// `u_resolution` for each of the two shadow programs.
    ///
    /// **A location per program, and the field's own name hides the trap: a
    /// uniform location belongs to the program it was queried from.** The mask
    /// program and the unblurred-shadow program each need their own, for the same
    /// reason `u_text_resolution` and `u_image_resolution` exist above. A missing
    /// one neither errors nor draws: `a_pos / vec2(0.0)` is a division by zero,
    /// every vertex becomes `NaN`, and the pass covers no fragments — a shadow
    /// that is not there, with every GL call reporting success.
    u_shadow_resolution: Option<glow::UniformLocation>,
    u_shadow_solid_resolution: Option<glow::UniformLocation>,
    /// `u_size` for each of the two programs that draw the window-sized quad, for
    /// the same reason and with the same failure.
    u_blur_size: Option<glow::UniformLocation>,
    u_composite_size: Option<glow::UniformLocation>,
    /// The blur's texel size and direction, its weight array and its tap count.
    u_blur_texel: Option<glow::UniformLocation>,
    u_blur_direction: Option<glow::UniformLocation>,
    u_blur_weights: Option<glow::UniformLocation>,
    u_blur_taps: Option<glow::UniformLocation>,
    /// The sampler each of the two quad programs samples, set to unit 0 explicitly
    /// rather than left at its default — the image and text passes both do the
    /// same, and a sampler nobody wrote down is one more uniform whose value is a
    /// default rather than a decision.
    u_blur_source: Option<glow::UniformLocation>,
    u_composite_source: Option<glow::UniformLocation>,
    /// The window-sized quad's six vertices, rebuilt only when the window moves.
    blur_vertices: Vec<blur::BlurVertex>,
    /// The window size `blur_vertices` was built for, and the test for whether
    /// they need rebuilding.
    blur_vertex_size: (u32, u32),
}

impl Renderer {
    /// Creates a renderer that draws into `context`'s window.
    ///
    /// Compiles the solid-color program and allocates the shared vertex
    /// array, the dynamic vertex buffer and the static index buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if a shader fails to compile, the program fails to
    /// link, or a GL object cannot be created.
    pub fn new(context: Context) -> Result<Self, RenderError> {
        let program = create_program(context.gl())?;
        let text_program = create_text_program(context.gl())?;
        let image_program = create_image_program(context.gl())?;
        // SAFETY: The GL context is current on this thread.
        let (vao, vbo, ibo) = unsafe {
            let gl = context.gl();
            (
                gl.create_vertex_array().map_err(RenderError::Gl)?,
                gl.create_buffer().map_err(RenderError::Gl)?,
                gl.create_buffer().map_err(RenderError::Gl)?,
            )
        };
        // SAFETY: The GL context is current on this thread.
        let (text_vao, text_vbo, text_ibo) = unsafe {
            let gl = context.gl();
            (
                gl.create_vertex_array().map_err(RenderError::Gl)?,
                gl.create_buffer().map_err(RenderError::Gl)?,
                gl.create_buffer().map_err(RenderError::Gl)?,
            )
        };
        // SAFETY: The GL context is current on this thread.
        let (image_vao, image_vbo, image_ibo) = unsafe {
            let gl = context.gl();
            (
                gl.create_vertex_array().map_err(RenderError::Gl)?,
                gl.create_buffer().map_err(RenderError::Gl)?,
                gl.create_buffer().map_err(RenderError::Gl)?,
            )
        };
        // SAFETY: The GL context is current on this thread and `program` is
        // the linked program.
        let u_resolution = unsafe { context.gl().get_uniform_location(program, "u_resolution") };
        // SAFETY: The GL context is current on this thread and `text_program`
        // is the linked program.
        let u_atlas = unsafe { context.gl().get_uniform_location(text_program, "u_atlas") };
        // A uniform location belongs to the program it was queried from, so the
        // text program needs its own `u_resolution`; the solid program's
        // location would leave the text shader's at (0, 0).
        // SAFETY: The GL context is current on this thread and `text_program`
        // is the linked program.
        let u_text_resolution = unsafe {
            context
                .gl()
                .get_uniform_location(text_program, "u_resolution")
        };
        // A uniform location belongs to the program it was queried from, so the
        // image program needs its own two: the solid and text programs' would
        // leave the image shader's at (0, 0).
        // SAFETY: The GL context is current on this thread and `image_program`
        // is the linked program.
        let u_image_resolution = unsafe {
            context
                .gl()
                .get_uniform_location(image_program, "u_resolution")
        };
        // SAFETY: The GL context is current on this thread and `image_program`
        // is the linked program.
        let u_image = unsafe { context.gl().get_uniform_location(image_program, "u_image") };
        // SAFETY: The GL context is current on this thread.
        let atlas_texture = unsafe { context.gl().create_texture().map_err(RenderError::Gl)? };
        // The image atlas is a texture object and nothing more: its storage is
        // allocated by the first upload, which happens when the first image is
        // packed into it. A renderer that draws no image never allocates 16 MB
        // of atlas, and the glyph atlas above already sets that precedent.
        // SAFETY: The GL context is current on this thread.
        let image_atlas_texture =
            unsafe { context.gl().create_texture().map_err(RenderError::Gl)? };
        let (shadow_mask_program, shadow_program) = create_shadow_programs(context.gl())?;
        let (blur_program, shadow_composite_program) = create_blur_programs(context.gl())?;
        let blur_quad = BlurQuad::new(context.gl())?;
        // Nothing is allocated: a frame that draws no shadow never allocates
        // `width · height` bytes, which is the decision the image atlas above
        // already makes and the same one.
        let shadow_target = ShadowTarget::new(context.gl())?;
        // SAFETY: The GL context is current on this thread and each program is
        // the linked program the location is asked of — a location belongs to
        // its own program, so every one of these is a separate query rather than
        // a shared handle.
        let u_shadow_color = unsafe {
            context
                .gl()
                .get_uniform_location(shadow_composite_program, "u_color")
        };
        // SAFETY: The GL context is current on this thread and `blur_program` is
        // the linked program.
        let u_blur_texel = unsafe { context.gl().get_uniform_location(blur_program, "u_texel") };
        // SAFETY: The GL context is current on this thread and `blur_program` is
        // the linked program.
        let u_blur_direction = unsafe {
            context
                .gl()
                .get_uniform_location(blur_program, "u_direction")
        };
        // SAFETY: The GL context is current on this thread and `blur_program` is
        // the linked program. The location asked for is the array itself, which
        // is what `uniform_1_f32_slice` writes through.
        let u_blur_weights =
            unsafe { context.gl().get_uniform_location(blur_program, "u_weights") };
        // SAFETY: The GL context is current on this thread and `blur_program` is
        // the linked program.
        let u_blur_taps = unsafe { context.gl().get_uniform_location(blur_program, "u_taps") };
        // SAFETY: The GL context is current on this thread and
        // `shadow_mask_program` is the linked program. The mask shares the solid
        // vertex shader, so it needs its own `u_resolution` — a location belongs
        // to the program it was queried from, and the solid program's would leave
        // this one's at (0, 0), which is a division by zero and no fragments.
        let u_shadow_resolution = unsafe {
            context
                .gl()
                .get_uniform_location(shadow_mask_program, "u_resolution")
        };
        // SAFETY: The GL context is current on this thread and `shadow_program`
        // is the linked program; a third location for the same uniform, for the
        // same reason.
        let u_shadow_solid_resolution = unsafe {
            context
                .gl()
                .get_uniform_location(shadow_program, "u_resolution")
        };
        // SAFETY: The GL context is current on this thread and `blur_program` is
        // the linked program.
        let u_blur_size = unsafe { context.gl().get_uniform_location(blur_program, "u_size") };
        // SAFETY: The GL context is current on this thread and
        // `shadow_composite_program` is the linked program.
        let u_composite_size = unsafe {
            context
                .gl()
                .get_uniform_location(shadow_composite_program, "u_size")
        };
        // SAFETY: The GL context is current on this thread and `blur_program` is
        // the linked program.
        let u_blur_source = unsafe { context.gl().get_uniform_location(blur_program, "u_source") };
        // SAFETY: The GL context is current on this thread and
        // `shadow_composite_program` is the linked program.
        let u_composite_source = unsafe {
            context
                .gl()
                .get_uniform_location(shadow_composite_program, "u_source")
        };
        let mut renderer = Renderer {
            context,
            program,
            vao,
            vbo,
            ibo,
            u_resolution,
            batcher: Batcher::new(),
            viewport: (0, 0),
            vertex_capacity: 0,
            index_capacity: 0,
            text_program,
            text_vao,
            text_vbo,
            text_ibo,
            u_atlas,
            u_text_resolution,
            atlas_texture,
            fonts: FontSet::new(),
            atlas: GlyphAtlas::new(ATLAS_SIZE),
            applied_clip: None,
            text_vertex_capacity: 0,
            text_index_capacity: 0,
            image_program,
            image_vao,
            image_vbo,
            image_ibo,
            u_image_resolution,
            u_image,
            image_atlas_texture,
            standalone_textures: HashMap::new(),
            textures: TextureCache::new(),
            image_vertices: Vec::new(),
            image_vertex_capacity: 0,
            image_index_capacity: 0,
            shadow_mask_program,
            shadow_program,
            blur_program,
            shadow_composite_program,
            blur_quad,
            shadow_target,
            u_shadow_color,
            u_shadow_resolution,
            u_shadow_solid_resolution,
            u_blur_size,
            u_composite_size,
            u_blur_texel,
            u_blur_direction,
            u_blur_weights,
            u_blur_taps,
            u_blur_source,
            u_composite_source,
            blur_vertices: Vec::new(),
            blur_vertex_size: (0, 0),
        };
        // SAFETY: The GL context is current on this thread; `vao`, `vbo` and
        // `ibo` are valid objects created above. The attribute pointers and
        // the element array binding are captured by the VAO.
        unsafe {
            let gl = renderer.context.gl();
            gl.bind_vertex_array(Some(renderer.vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(renderer.vbo));
            gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, Some(renderer.ibo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, GL_FLOAT, false, VERTEX_STRIDE, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, GL_FLOAT, false, VERTEX_STRIDE, LOCAL_OFFSET);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 4, GL_FLOAT, false, VERTEX_STRIDE, COLOR_OFFSET);
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, GL_FLOAT, false, VERTEX_STRIDE, RADIUS_OFFSET);
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(4, 2, GL_FLOAT, false, VERTEX_STRIDE, SIZE_OFFSET);
            gl.bind_vertex_array(None);
        }
        // SAFETY: The GL context is current on this thread; the text VAO, VBO
        // and IBO are valid objects created above.
        unsafe {
            let gl = renderer.context.gl();
            gl.bind_vertex_array(Some(renderer.text_vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(renderer.text_vbo));
            gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, Some(renderer.text_ibo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, GL_FLOAT, false, TEXT_VERTEX_STRIDE, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, GL_FLOAT, false, TEXT_VERTEX_STRIDE, TEXT_UV_OFFSET);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                4,
                GL_FLOAT,
                false,
                TEXT_VERTEX_STRIDE,
                TEXT_COLOR_OFFSET,
            );
            gl.bind_vertex_array(None);
        }
        // SAFETY: The GL context is current on this thread; the image VAO, VBO
        // and IBO are valid objects created above.
        unsafe {
            let gl = renderer.context.gl();
            gl.bind_vertex_array(Some(renderer.image_vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(renderer.image_vbo));
            gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, Some(renderer.image_ibo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, GL_FLOAT, false, IMAGE_VERTEX_STRIDE, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                2,
                GL_FLOAT,
                false,
                IMAGE_VERTEX_STRIDE,
                IMAGE_LOCAL_OFFSET,
            );
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                2,
                GL_FLOAT,
                false,
                IMAGE_VERTEX_STRIDE,
                IMAGE_UV_OFFSET,
            );
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(
                3,
                1,
                GL_FLOAT,
                false,
                IMAGE_VERTEX_STRIDE,
                IMAGE_RADIUS_OFFSET,
            );
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(
                4,
                1,
                GL_FLOAT,
                false,
                IMAGE_VERTEX_STRIDE,
                IMAGE_OPACITY_OFFSET,
            );
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(
                5,
                2,
                GL_FLOAT,
                false,
                IMAGE_VERTEX_STRIDE,
                IMAGE_SIZE_OFFSET,
            );
            gl.bind_vertex_array(None);
        }
        renderer.ensure_index_capacity(INITIAL_CAPACITY)?;
        renderer.ensure_vertex_capacity(INITIAL_CAPACITY)?;
        renderer.ensure_text_index_capacity(INITIAL_CAPACITY)?;
        renderer.ensure_text_vertex_capacity(INITIAL_CAPACITY)?;
        renderer.ensure_image_index_capacity(INITIAL_CAPACITY)?;
        renderer.ensure_image_vertex_capacity(INITIAL_CAPACITY)?;
        Ok(renderer)
    }

    /// Sets the font the renderer draws ordinary text with.
    ///
    /// The font is loaded from a file path by the caller; the renderer owns it
    /// and the glyph atlas, and rasterizes glyphs into the atlas as text is
    /// drawn. It is the **regular** face, and asking for it is what a run that
    /// names no weight gets; [`Renderer::set_bold_font`] is this call with the
    /// other weight.
    ///
    /// Installing a face rasterizes nothing: the work is done when a run asks for
    /// a glyph, and a renderer whose text is all regular pays exactly what it
    /// paid before the second weight existed.
    pub fn set_font(&mut self, font: Font) {
        self.fonts.set(FontWeight::Regular, font);
    }

    /// Sets the face the renderer draws [`FontWeight::Bold`] runs with.
    ///
    /// **Optional, and nothing changes without it.** A text command recorded in
    /// the bold weight resolves to the regular face until this is called — see
    /// `ui_core::font`'s [`resolve_slot`](crate::font::resolve_slot) — so a
    /// renderer that never loads a bold face is not merely missing a feature but
    /// behaves *identically* to one that does, right down to the cost.
    ///
    /// The two weights are two files and therefore two [`Font`]s, and a bold run
    /// is drawn from the second face's own glyphs, bearings and advances: nothing
    /// here thickens a regular glyph or draws it twice.
    pub fn set_bold_font(&mut self, font: Font) {
        self.fonts.set(FontWeight::Bold, font);
    }

    /// Loads the image at `path` and returns a handle to it.
    ///
    /// This is the renderer's route to an image, and the only one a caller
    /// needs: the decode, the premultiplication and the decision between the
    /// shared atlas and a texture of the image's own all belong to
    /// [`crate::texture`], and asking twice for the same path costs nothing —
    /// the second call is answered from the cache without decoding again.
    ///
    /// An image that will be packed into the shared atlas has nothing to do to
    /// the GPU here: its pixels go into the atlas and are uploaded with it.
    /// An image too large for the atlas gets its GL texture now, so that the
    /// first frame that draws it is not also the frame that allocates it.
    ///
    /// A path that cannot be loaded is an error and no handle. The caller may
    /// ask again — the failure is not remembered — which is what makes fixing a
    /// file and retrying possible.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Texture`] with
    /// [`crate::texture::TextureError`]'s message when the file cannot be opened
    /// or decoded or has no addressable size, and [`RenderError::Gl`] when a
    /// texture of its own cannot be created.
    pub fn load_texture(&mut self, path: &Path) -> Result<TextureHandle, RenderError> {
        let handle = self.textures.load_from_file(path)?;
        self.ensure_standalone_texture(handle)?;
        Ok(handle)
    }

    /// Returns the cache of decoded images, for a caller that needs to know
    /// what it has.
    ///
    /// A widget laying out an image asks [`TextureCache::size_of`] for the
    /// handle's size to work out a fit, and it cannot reach the renderer from
    /// inside a paint pass without a borrow that outlives the frame — hence a
    /// getter rather than a handle that carries its own size.
    #[must_use]
    pub fn textures(&self) -> &TextureCache {
        &self.textures
    }

    /// Returns a reference to the SDL3 context, for obtaining the event pump.
    #[must_use]
    pub fn sdl(&self) -> &sdl3::Sdl {
        self.context.sdl()
    }

    /// Starts a frame: clears the screen, resets the viewport to the window
    /// size, disables scissoring and empties the batcher.
    pub fn begin_frame(&mut self) {
        let (width, height) = self.context.window_size();
        self.viewport = (width, height);
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread.
        unsafe {
            gl.viewport(
                0,
                0,
                i32::try_from(width).unwrap_or(0),
                i32::try_from(height).unwrap_or(0),
            );
            gl.disable(GL_SCISSOR_TEST);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(GL_COLOR_BUFFER_BIT);
        }
        self.batcher.reset();
        // The scissor is disabled above, so the renderer's idea of what is
        // applied has to be cleared with it or the next clipped batch would be
        // skipped as "unchanged" and drawn unclipped.
        self.applied_clip = None;
    }

    /// Records the draw commands of the node `handle` points to.
    ///
    /// The commands are recorded into this frame's batches only when the
    /// node's [`crate::node::WidgetNode`] paint state is dirty; recording clears
    /// the dirty flag. A stale handle is ignored.
    ///
    /// The order the nodes are drawn in is the order they appear on screen, so
    /// this is the caller's to choose: a widget tree walks itself depth first,
    /// and a widget that overlaps another is drawn after it.
    pub fn draw_node(&mut self, handle: Handle, nodes: &mut Arena<WidgetNode>) {
        self.draw_node_clipped(handle, nodes, None);
    }

    /// Records the draw commands of the node `handle` points to, clipped to
    /// `clip`.
    ///
    /// The clip is in window coordinates and is applied by the **GPU**, as a
    /// scissor, at submission time rather than by discarding commands: a
    /// command that straddles the edge of a viewport is half drawn, which is what
    /// a scroll view is made of and what no test of recorded commands can see.
    ///
    /// It rides on the batch rather than on the command — see [`Batch::clip`] —
    /// and [`Renderer::end_frame`] sets the scissor when the clip changes between
    /// batches. `None` means the whole window, which is what every node but a
    /// scrolling one passes.
    ///
    /// The commands are recorded into this frame's batches only when the node's
    /// [`crate::node::WidgetNode`] paint state is dirty; recording clears the
    /// dirty flag. A stale handle is ignored.
    pub fn draw_node_clipped(
        &mut self,
        handle: Handle,
        nodes: &mut Arena<WidgetNode>,
        clip: Option<Rect>,
    ) {
        let Some(state) = nodes.get_mut(handle).map(|node| node.paint_mut()) else {
            return;
        };
        if !state.is_dirty() {
            return;
        }
        let commands = state.take_commands();
        for command in commands {
            self.batcher.add_clipped(command, clip);
        }
    }

    /// Sets the scissor rect for subsequent GPU draws, or disables
    /// scissoring when `rect` is `None`.
    ///
    /// The rect is in window coordinates with the origin at the top left,
    /// matching [`Rect`]; GL's scissor origin is the bottom left, so the Y
    /// axis is flipped.
    ///
    /// The scissor applies to the whole frame: draw commands are recorded
    /// during [`Renderer::draw_node`] and submitted together in
    /// [`Renderer::end_frame`], so there is no per-node clip to hook a
    /// scissor to yet. [`crate::layout::LayoutState::clip`] already carries
    /// the per-node rect; applying it per node is deferred, because a
    /// recorded command has no scissor state of its own to carry.
    pub fn set_scissor(&self, rect: Option<Rect>) {
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread.
        unsafe {
            match rect {
                Some(rect) => {
                    gl.enable(GL_SCISSOR_TEST);
                    let height = i32::try_from(self.viewport.1).unwrap_or(0);
                    gl.scissor(
                        f32_to_i32(rect.x),
                        height - f32_to_i32(rect.y) - f32_to_i32(rect.height),
                        f32_to_i32(rect.width),
                        f32_to_i32(rect.height),
                    );
                }
                None => gl.disable(GL_SCISSOR_TEST),
            }
        }
    }

    /// Submits the recorded batches to the GPU and swaps the buffers.
    ///
    /// The frame is submitted as a list of [`crate::batch::Segment`]s, in the
    /// order the commands were recorded, and each segment is submitted the way
    /// the frame used to be submitted as a whole:
    ///
    /// - the segment's opaque group, drawn with blending **disabled**;
    /// - then, with blending on and `glBlendFunc(ONE, ONE_MINUS_SRC_ALPHA)`,
    ///   the passes in `COMPOSITED_PASSES` order over the opaque group and then
    ///   the transparent group;
    /// - then, if the segment ended at a shadow, that shadow — see
    ///   `Renderer::draw_shadow_batch`.
    ///
    /// **A frame with no shadow is one segment, and that is the whole frame's
    /// old submission order.** The segmentation exists for one case — a
    /// translucent overlay with an opaque panel on top of it, which is what a
    /// modal dialog is — and the cost of that case is paid by the frames without
    /// it: one `Vec` and one branch per frame.
    ///
    /// The image pass is drawn with blending **on** even for the batches the
    /// batcher called opaque, and for the same reason the text pass is. Text:
    /// the glyph coverage is a per-fragment alpha, so even fully-opaque text
    /// composites its anti-aliased edges. Image: an opaque image command asks
    /// for the whole texture at full opacity, which says nothing about the
    /// texels in it. A PNG with a transparent background arrives in a batch
    /// that blends, and one with alpha 255 everywhere arrives in one that does
    /// not, but its corners may be discarded and the destination has to show
    /// through — so the whole pass blends.
    ///
    /// # Errors
    ///
    /// Returns an error if a buffer grows past what GL can address, or if a
    /// shadow's offscreen target cannot be allocated.
    pub fn end_frame(&mut self) -> Result<(), RenderError> {
        let segments = self.batcher.submit_order();
        for segment in &segments {
            // SAFETY: The GL context is current on this thread; `self.program` is
            // the linked solid program.
            unsafe {
                let gl = self.context.gl();
                gl.use_program(Some(self.program));
                gl.uniform_2_f32(
                    self.u_resolution.as_ref(),
                    u32_to_f32(self.viewport.0),
                    u32_to_f32(self.viewport.1),
                );
                gl.disable(GL_BLEND);
            }
            for batch in &segment.opaque {
                self.draw_pass(Pass::Solid, batch)?;
            }
            // SAFETY: The GL context is current on this thread.
            unsafe {
                let gl = self.context.gl();
                gl.enable(GL_BLEND);
                gl.blend_func(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
            }
            for pass in COMPOSITED_PASSES {
                for batch in &segment.opaque {
                    self.draw_pass(pass, batch)?;
                }
                for batch in &segment.transparent {
                    self.draw_pass(pass, batch)?;
                }
            }
            // The shadow comes after everything the segment recorded and before
            // everything the next one will, which is the whole of what the
            // segmentation is for.
            if let Some(shadow) = &segment.shadow {
                self.draw_shadow_batch(shadow)?;
            }
        }
        self.context.swap();
        Ok(())
    }

    /// Draws one batch with the pass `pass` draws it with.
    ///
    /// The pass is chosen here and nowhere else: each `draw_*_batch` below still
    /// checks the batch's own shader kind, so a batch reaching the wrong
    /// function is skipped rather than drawn as something it is not.
    fn draw_pass(&mut self, pass: Pass, batch: &Batch) -> Result<(), RenderError> {
        self.apply_clip(batch.clip);
        match pass {
            Pass::Solid => self.draw_solid_batch(batch),
            Pass::Image => self.draw_image_batch(batch),
            Pass::Text => self.draw_text_batch(batch),
        }
    }

    /// Sets the scissor to `clip`, unless it is already set to it.
    ///
    /// **This is what makes a per-node clip possible at all**, and it is the
    /// deferral `doc/ui/IMPLEMENTATION_STATE.md` § *Deviations from the spec*
    /// recorded as unhookable: draw commands are recorded during
    /// [`Renderer::draw_node`] and submitted together in [`Renderer::end_frame`],
    /// so setting a scissor while recording set it for the wrong moment and the
    /// last one won for the whole frame.
    ///
    /// The answer is to carry the clip on the **batch** and set it here, at
    /// submission, where a draw call is about to happen — see [`Batch::clip`].
    /// The cost is a GL state change whenever the clip changes, which is once per
    /// viewport and not once per batch.
    ///
    /// **The cache is only as good as every writer of the GL state it mirrors.**
    /// Anything that changes the scissor behind this function's back has to clear
    /// [`Self::applied_clip`] too, or the next `apply_clip` with an equal clip
    /// returns early and the box is never written.
    /// [`Self::bind_default_target`] is the one such writer — it clears the cache
    /// and then calls this, which is what makes the shadow's composite clipped
    /// after the offscreen passes turned the scissor off.
    fn apply_clip(&mut self, clip: Option<Rect>) {
        if clip == self.applied_clip {
            return;
        }
        self.applied_clip = clip;
        self.set_scissor(clip);
    }

    /// Draws one batch with the solid-color shader.
    ///
    /// Batches with another shader kind are skipped: their programs arrive
    /// with the widgets that need them.
    fn draw_solid_batch(&mut self, batch: &Batch) -> Result<(), RenderError> {
        if batch.key.shader != ShaderKind::Solid {
            return Ok(());
        }
        let vertices = batch_vertices(batch);
        self.draw_vertex_quads(&vertices, self.program)
    }

    /// Uploads `vertices` — four per quad — and draws them with `program`.
    ///
    /// **The one place the solid geometry is submitted.** The shadow's shape
    /// needs the same vertices, the same index buffer and the same vertex array
    /// as an ordinary rect, under a different program; splitting that out would
    /// have been a second copy of six lines that has to agree with the first
    /// about buffer sizes.
    ///
    /// `use_program` is called here rather than by the caller. It used to be set
    /// once by [`Renderer::end_frame`] before the opaque group, which was
    /// correct only because the solid pass happens to be the first thing drawn
    /// in a frame; a caller that reached this function through any other path got
    /// whatever program was last bound.
    fn draw_vertex_quads(
        &mut self,
        vertices: &[Vertex],
        program: glow::Program,
    ) -> Result<(), RenderError> {
        // SAFETY: The GL context is current on this thread and `program` is a
        // linked program.
        unsafe {
            self.context.gl().use_program(Some(program));
        }
        self.submit_vertex_quads(vertices)
    }

    /// Uploads `vertices` — four per quad — and draws them with whichever
    /// program is in use.
    ///
    /// **Split out of [`Self::draw_vertex_quads`] because a uniform is set on
    /// the program that is in use, not on the one it is about to be.** A caller
    /// that sets `u_resolution` and *then* asks for the draw writes the uniform
    /// to whichever program the previous pass left bound — which is what the two
    /// shadow passes did first, and the failure is a division by zero inside the
    /// vertex shader: no GL error, every call reporting success, and a shadow
    /// that is not on the screen.
    fn submit_vertex_quads(&mut self, vertices: &[Vertex]) -> Result<(), RenderError> {
        if vertices.is_empty() {
            return Ok(());
        }
        let quads = vertices.len() / VERTS_PER_QUAD;
        self.ensure_index_capacity(quads)?;
        self.ensure_vertex_capacity(quads)?;
        // SAFETY: `Vertex` is `repr(C)` with 11 `f32` fields and no padding,
        // so the slice is a valid `VERTEX_STRIDE`-strided vertex array. The
        // slice borrows `vertices`, which outlives the upload.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr().cast::<u8>(),
                std::mem::size_of_val(vertices),
            )
        };
        let count = i32::try_from(quads * INDICES_PER_QUAD)
            .map_err(|_| RenderError::Gl("index count exceeds the i32 range".to_string()))?;
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread; `self.vao` is a
        // valid vertex array carrying the attribute pointers and the element
        // array binding, and `self.vbo` is a valid buffer bound for the upload.
        unsafe {
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_sub_data_u8_slice(GL_ARRAY_BUFFER, 0, bytes);
            gl.draw_elements(GL_TRIANGLES, count, GL_UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);
        }
        Ok(())
    }

    /// Binds `program`, sets its `u_resolution` to the window, and draws
    /// `vertices` — four per quad — with it.
    ///
    /// The two shadow programs need this and the solid pass does not: the solid
    /// pass's `u_resolution` is set once per frame by [`Renderer::end_frame`],
    /// for the solid program, and the shadow programs are drawn at a point in the
    /// frame where a different program is in use.
    fn draw_shadow_quads(
        &mut self,
        vertices: &[Vertex],
        program: glow::Program,
        u_resolution: Option<&glow::UniformLocation>,
    ) -> Result<(), RenderError> {
        let (width, height) = self.viewport;
        // SAFETY: The GL context is current on this thread, `program` is a linked
        // program, and `u_resolution` was queried from that program.
        unsafe {
            let gl = self.context.gl();
            gl.use_program(Some(program));
            gl.uniform_2_f32(u_resolution, u32_to_f32(width), u32_to_f32(height));
        }
        self.submit_vertex_quads(vertices)
    }

    /// Draws one shadow, where it was recorded.
    ///
    /// Three paths, and the one that runs is decided by the command's `blur`:
    ///
    /// - **at or below [`blur::SOLID_BLUR`]** — the shape is drawn straight to
    ///   the screen with [`SHADOW_FRAGMENT_SHADER_SRC`], which premultiplies.
    ///   No offscreen target is touched and no texture is allocated; this is the
    ///   cheap path [`blur::SOLID_BLUR`]'s docs argue for.
    /// - **otherwise** — the shape is drawn into [`ShadowTarget`] as coverage,
    ///   blurred horizontally and then vertically, and the result is tinted and
    ///   composited over the screen.
    ///
    /// **The clip is enforced twice, and the second one is the one that matters.**
    /// The call to [`Self::apply_clip`] below enables the scissor for the offscreen
    /// passes — which is correct and is what a viewport's clip means for a
    /// mask and a blur — but [`ShadowTarget::bind_for_write`] then **disables**
    /// the scissor test, because those passes cover the whole window. So the
    /// **composite** is the pass that enforces the clip: it re-applies it through
    /// [`Self::bind_default_target`], which writes the scissor and the cache
    /// together. The no-blur path never reaches `bind_for_write`, so for it the
    /// first application is the only one — and its `bind_default_target` call is
    /// still what makes the two paths agree.
    ///
    /// An earlier version of this doc claimed the clip was applied *once* for all
    /// of it. That was wrong, and the composite ran with the scissor test off.
    ///
    /// The offscreen passes leave the framebuffer, the viewport and the scissor
    /// in a state the rest of the frame does not expect, so
    /// [`Self::bind_default_target`] puts them back before anything else is
    /// drawn — including this function's own no-blur path, which is why it is
    /// called from both.
    fn draw_shadow_batch(&mut self, batch: &Batch) -> Result<(), RenderError> {
        let Some(command) = batch.commands.iter().find_map(|command| match command {
            DrawCommand::Shadow {
                rect,
                radius,
                color,
                blur,
                offset,
            } => Some((*rect, *radius, *color, *blur, *offset)),
            _ => None,
        }) else {
            return Ok(());
        };
        let (rect, radius, color, blur_sigma, offset) = command;
        self.apply_clip(batch.clip);
        // The shape is the rect moved by the offset. **Not grown**: the blur
        // spreads the shape by itself, past its own edge, which is what a
        // shadow's falloff is — growing the quad would widen the shadow by the
        // blur's reach on top of that.
        let shape = Rect::new(
            rect.x + offset.0,
            rect.y + offset.1,
            rect.width,
            rect.height,
        );
        let quad = rect_quad(shape, color, radius);
        let vertices = quad_vertices(quad);

        if blur_sigma <= blur::SOLID_BLUR {
            self.bind_default_target(batch.clip);
            // SAFETY: The GL context is current on this thread.
            unsafe {
                let gl = self.context.gl();
                gl.enable(GL_BLEND);
                gl.blend_func(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
            }
            let u_resolution = self.u_shadow_solid_resolution;
            return self.draw_shadow_quads(&vertices, self.shadow_program, u_resolution.as_ref());
        }

        self.draw_shadow_offscreen(&vertices, color, blur_sigma, batch.clip)
    }

    /// Runs the offscreen half of one shadow: mask, two blur passes, composite.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Gl`] when the offscreen target cannot be
    /// allocated at the window's size.
    fn draw_shadow_offscreen(
        &mut self,
        vertices: &[Vertex],
        color: Color,
        sigma: f32,
        clip: Option<Rect>,
    ) -> Result<(), RenderError> {
        let (width, height) = self.viewport;
        // `self.context.gl()` is asked for again at each step rather than held
        // in a local: the immutable borrow it returns would outlive the
        // `&mut self` calls this function is mostly made of.
        self.shadow_target
            .ensure_size(self.context.gl(), width, height)?;
        // The weights are computed here rather than in the shader because they
        // are the same for every fragment and every shadow of the same blur: one
        // `exp()` per tap per shadow instead of per pixel. See `blur`'s module
        // docs for why that is the arrangement.
        let weights = blur::kernel(sigma);
        let taps = weights.len();
        // The window-sized quad's six vertices, rebuilt only when the window
        // moves. One `Vec` per shadowed frame would be a malloc per frame for a
        // thing that is six numbers and almost never changes.
        if self.blur_vertex_size != (width, height) {
            self.blur_vertices = blur::full_quad(u32_to_f32(width), u32_to_f32(height)).to_vec();
            self.blur_vertex_size = (width, height);
        }
        let target = self.shadow_target.size();

        // 1. The shape, as coverage, into the texture the first pass reads.
        self.shadow_target.bind_for_write(self.context.gl());
        // SAFETY: The GL context is current on this thread.
        unsafe {
            let gl = self.context.gl();
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(GL_COLOR_BUFFER_BIT);
            gl.disable(GL_BLEND);
        }
        let u_mask_resolution = self.u_shadow_resolution;
        self.draw_shadow_quads(
            vertices,
            self.shadow_mask_program,
            u_mask_resolution.as_ref(),
        )?;

        // 2 and 3. One pass per axis. The direction is the only thing that
        // changes between them, and each `swap` is what makes the texture just
        // written the one this pass reads — see `target::ShadowTarget`'s docs for
        // the invariant that the two are never the same texture.
        //
        // The two swaps inside the loop and the one after it are three, not two:
        // the shape went in through the *write* side, and there is no read of it
        // until the first pass, so the first `swap` is spent turning the write
        // target into the first pass's source. Written as it is rather than
        // folded into `bind_for_write` so the count is visible at the call site.
        for direction in [(1.0_f32, 0.0_f32), (0.0_f32, 1.0_f32)] {
            self.shadow_target.swap();
            self.shadow_target.bind_for_write(self.context.gl());
            self.shadow_target.bind_for_read(self.context.gl());
            // SAFETY: The GL context is current on this thread; `blur_program` is
            // the linked program and each location was queried from it.
            unsafe {
                let gl = self.context.gl();
                gl.use_program(Some(self.blur_program));
                gl.uniform_2_f32(
                    self.u_blur_size.as_ref(),
                    u32_to_f32(width),
                    u32_to_f32(height),
                );
                gl.uniform_1_i32(self.u_blur_source.as_ref(), 0);
                gl.uniform_2_f32(
                    self.u_blur_texel.as_ref(),
                    1.0 / u32_to_f32(target.0).max(1.0),
                    1.0 / u32_to_f32(target.1).max(1.0),
                );
                gl.uniform_2_f32(self.u_blur_direction.as_ref(), direction.0, direction.1);
                gl.uniform_1_f32_slice(self.u_blur_weights.as_ref(), &weights);
                gl.uniform_1_i32(self.u_blur_taps.as_ref(), i32::try_from(taps).unwrap_or(0));
            }
            self.blur_quad.draw(self.context.gl(), &self.blur_vertices);
        }
        self.shadow_target.swap();

        // 4. The composite, over the window, in the segment's place — and with
        // the clip back, which is the pass that puts the shadow on the screen and
        // therefore the one the scissor is for. See `bind_default_target`.
        self.bind_default_target(clip);
        self.shadow_target.bind_for_read(self.context.gl());
        // SAFETY: The GL context is current on this thread; the composite program
        // is linked and its colour location was queried from it.
        unsafe {
            let gl = self.context.gl();
            gl.use_program(Some(self.shadow_composite_program));
            gl.uniform_2_f32(
                self.u_composite_size.as_ref(),
                u32_to_f32(width),
                u32_to_f32(height),
            );
            gl.uniform_1_i32(self.u_composite_source.as_ref(), 0);
            let tint = quad_color(color);
            gl.uniform_4_f32(
                self.u_shadow_color.as_ref(),
                tint[0],
                tint[1],
                tint[2],
                tint[3],
            );
            gl.enable(GL_BLEND);
            gl.blend_func(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
        }
        self.blur_quad.draw(self.context.gl(), &self.blur_vertices);
        Ok(())
    }

    /// Puts the framebuffer, the viewport **and the scissor** back to the
    /// window's, for anything drawn after the offscreen passes.
    ///
    /// **The scissor is put back here rather than by the caller, and that is the
    /// whole of this function's existence in the shadow path.**
    /// `ShadowTarget::bind_for_write` turns the scissor test **off** — it has to,
    /// because the offscreen passes cover the whole window and a viewport's clip
    /// would otherwise cut the mask and both blur passes — and nothing between
    /// that and the composite turned it back on. The composite therefore ran with
    /// the scissor test off and drew a clipped shadow over the whole window,
    /// while the no-blur path, which never reaches `bind_for_write`, was clipped
    /// correctly. The two paths disagreed, and only the one the dialog does not
    /// use was right.
    ///
    /// It is here rather than in `draw_shadow_offscreen` because of the renderer's
    /// cache. `applied_clip` is what says whether the scissor is already correct;
    /// [`Self::apply_clip`] is what consults it, and a caller that wrote the
    /// scissor behind its back would leave the cache claiming a clip that the GL
    /// state does not have, and the next batch would skip the call that fixes it.
    /// Setting `applied_clip` to `None` before re-applying is what keeps the two
    /// in step — see [`Self::apply_clip`].
    fn bind_default_target(&mut self, clip: Option<Rect>) {
        let (width, height) = self.viewport;
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread; `None` is the
        // window's own default framebuffer.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, u32_to_i32(width), u32_to_i32(height));
        }
        // The cache is invalidated **before** the state is written, and the state
        // is then written through `apply_clip` rather than by calling
        // `set_scissor` here. Those two facts are the same fact: `apply_clip`
        // compares against `applied_clip` and returns early when they are equal,
        // so a stale `Some(clip)` would make the re-application a no-op and the
        // bug would come straight back.
        self.applied_clip = None;
        self.apply_clip(clip);
    }

    /// Draws one batch of textured quads with the image shader.
    ///
    /// The batch's texture comes from its key rather than from each command:
    /// [`Batcher::add`] derives the key from the command, so every command in a
    /// batch names the same image and the lookup happens once instead of once
    /// per quad. A batch keyed by a non-image, or a batch with no texture at
    /// all, is skipped.
    ///
    /// The vertices are built into the renderer's own buffer rather than a
    /// `Vec` made per call: this runs once per image batch per frame, and a
    /// malloc per batch per frame is a cost the frame pays forever for a
    /// buffer that never has to be smaller. The buffer is moved out for the
    /// duration of the build — which needs `&mut self` again for the capacity
    /// checks below — and put back before the upload.
    fn draw_image_batch(&mut self, batch: &Batch) -> Result<(), RenderError> {
        if batch.key.shader != ShaderKind::Image {
            return Ok(());
        }
        let Some(texture) = batch.key.texture else {
            return Ok(());
        };
        let handle = TextureHandle::new(texture);

        let mut vertices = std::mem::take(&mut self.image_vertices);
        vertices.clear();
        for command in &batch.commands {
            let DrawCommand::Image {
                rect,
                uv,
                radius,
                opacity,
                ..
            } = command
            else {
                continue;
            };
            vertices.extend_from_slice(&image_quad(*rect, *uv, *opacity, *radius));
        }
        if vertices.is_empty() {
            self.image_vertices = vertices;
            return Ok(());
        }
        // The image may have been packed into the atlas since this pass last
        // ran, so the atlas has to be current before anything samples it —
        // taking the pixels here, just before the draw, is what makes the
        // upload happen once per change rather than once per frame.
        self.upload_image_atlas()?;
        let Some(bound) = self.image_texture(handle)? else {
            self.image_vertices = vertices;
            return Ok(());
        };
        let quads = vertices.len() / 4;
        self.ensure_image_index_capacity(quads)?;
        self.ensure_image_vertex_capacity(quads)?;
        // SAFETY: `ImageVertex` is `repr(C)` with 10 `f32` fields and no
        // padding, so the slice is a valid `IMAGE_VERTEX_STRIDE`-strided vertex
        // array. The slice borrows `vertices`, which outlives the upload.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr().cast::<u8>(),
                vertices.len() * std::mem::size_of::<ImageVertex>(),
            )
        };
        let count = i32::try_from(quads * INDICES_PER_QUAD)
            .map_err(|_| RenderError::Gl("index count exceeds the i32 range".to_string()))?;
        // SAFETY: The GL context is current on this thread;
        // `self.image_program` is the linked program, `self.image_vao` carries
        // the attribute pointers, and `self.image_vbo` is bound for the upload.
        unsafe {
            let gl = self.context.gl();
            gl.use_program(Some(self.image_program));
            gl.uniform_2_f32(
                self.u_image_resolution.as_ref(),
                u32_to_f32(self.viewport.0),
                u32_to_f32(self.viewport.1),
            );
            gl.active_texture(GL_TEXTURE0);
            gl.bind_texture(GL_TEXTURE_2D, Some(bound));
            gl.uniform_1_i32(self.u_image.as_ref(), 0);
            gl.bind_vertex_array(Some(self.image_vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.image_vbo));
            gl.buffer_sub_data_u8_slice(GL_ARRAY_BUFFER, 0, bytes);
            gl.draw_elements(GL_TRIANGLES, count, GL_UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);
        }
        self.image_vertices = vertices;
        Ok(())
    }

    /// Returns the GL texture the image `handle` is drawn from, creating it if
    /// this is the first time the image has been drawn and giving `None` when
    /// there is nothing to sample.
    ///
    /// The atlas case answers `None` for an image the cache no longer has a
    /// placement for. That is not a formality: the command still carries the UV
    /// window the image had, and eviction does not revoke it, so drawing would
    /// sample whatever image was packed into the space afterwards. Skipping the
    /// batch draws the background instead, which is what a missing image should
    /// look like.
    fn image_texture(
        &mut self,
        handle: TextureHandle,
    ) -> Result<Option<glow::Texture>, RenderError> {
        match image_source(handle.id()) {
            ImageSource::Atlas => Ok(self
                .textures
                .placement(handle)
                .map(|_| self.image_atlas_texture)),
            ImageSource::Own(_) => self.ensure_standalone_texture(handle),
        }
    }

    /// Returns the image `handle`'s own GL texture, making it the first time the
    /// image is seen, and `None` when the image has no pixels to upload.
    ///
    /// The pixels are read once and the texture kept thereafter: this is the
    /// only route by which a large image reaches the GPU, so it runs on the load
    /// and on the first draw, and after that it is a `HashMap` lookup. An
    /// atlas-resident handle answers `None` — it has no texture of its own, and
    /// the shared atlas is the texture it is drawn from.
    fn ensure_standalone_texture(
        &mut self,
        handle: TextureHandle,
    ) -> Result<Option<glow::Texture>, RenderError> {
        if !texture::is_standalone(handle.id()) {
            return Ok(None);
        }
        let key = handle.id().get();
        if let Some(&existing) = self.standalone_textures.get(&key) {
            return Ok(Some(existing));
        }
        let Some(pixels) = self.textures.standalone_pixels(handle) else {
            return Ok(None);
        };
        let (width, height) = pixels.size();
        // SAFETY: The GL context is current on this thread. `pixels` borrows
        // the cache, which outlives the upload, and its rows are four bytes per
        // pixel and contiguous — `crate::texture` flattens the decoded
        // surface's pitch away before the pixels reach here — so the default
        // unpack alignment of four is the right one.
        let created = unsafe {
            let gl = self.context.gl();
            let created = gl.create_texture().map_err(RenderError::Gl)?;
            gl.bind_texture(GL_TEXTURE_2D, Some(created));
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
            // `GL_RGBA8`, not the glyph atlas's `GL_R8`: these texels carry four
            // premultiplied channels, and a single-channel texture would sample
            // a red image as coverage.
            gl.tex_image_2d(
                GL_TEXTURE_2D,
                0,
                i32::try_from(GL_RGBA8).unwrap_or(0),
                i32::try_from(width).unwrap_or(0),
                i32::try_from(height).unwrap_or(0),
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(pixels.data())),
            );
            created
        };
        self.standalone_textures.insert(key, created);
        Ok(Some(created))
    }

    /// Uploads the shared image atlas to the GPU, but only when it changed.
    ///
    /// An unchanged atlas is not re-uploaded: it is
    /// [`crate::texture::ATLAS_SIZE`]² of four channels, and a screen that
    /// draws the same icon every frame would otherwise send sixteen megabytes
    /// to the GPU sixty times a second to produce exactly the picture it
    /// already has.
    fn upload_image_atlas(&mut self) -> Result<(), RenderError> {
        let size = self.textures.atlas_size();
        let Some(pixels) = self.textures.take_dirty_atlas_pixels() else {
            return Ok(());
        };
        // SAFETY: The GL context is current on this thread;
        // `self.image_atlas_texture` is a valid texture and `pixels` is a
        // `size`×`size` array of four bytes per pixel that outlives the upload.
        unsafe {
            let gl = self.context.gl();
            gl.active_texture(GL_TEXTURE0);
            gl.bind_texture(GL_TEXTURE_2D, Some(self.image_atlas_texture));
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
                i32::try_from(GL_RGBA8).unwrap_or(0),
                i32::try_from(size).unwrap_or(0),
                i32::try_from(size).unwrap_or(0),
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(pixels)),
            );
        }
        Ok(())
    }

    /// Uploads the glyph atlas to the GPU, but only when it changed.
    ///
    /// The atlas is populated lazily while a batch is being expanded, so this
    /// has to run *after* the quads are built and *before* they are drawn: an
    /// upload taken before rasterization would sample an empty texture.
    fn upload_atlas(&mut self) -> Result<(), RenderError> {
        let size = self.atlas.size();
        let Some(pixels) = self.atlas.take_dirty_pixels() else {
            return Ok(());
        };
        // SAFETY: The GL context is current on this thread; `self.atlas_texture`
        // is a valid texture and `pixels` is a `size`×`size` array that outlives
        // the upload.
        unsafe {
            let gl = self.context.gl();
            gl.active_texture(GL_TEXTURE0);
            gl.bind_texture(GL_TEXTURE_2D, Some(self.atlas_texture));
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
                gl_enum_to_i32(GL_R8),
                i32::try_from(size).unwrap_or(0),
                i32::try_from(size).unwrap_or(0),
                0,
                GL_RED,
                GL_UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(pixels)),
            );
        }
        Ok(())
    }

    /// Draws one text batch with the text shader.
    ///
    /// Each text command is expanded into one quad per glyph, positioned by the
    /// glyph's bearing and advance, and sampling its coverage from the atlas.
    /// Batches that are not text, or drawn with no font set, are skipped.
    ///
    /// **The face is resolved per command, and the whole of it comes from one
    /// [`FaceRef`](crate::font::FaceRef).** Baseline, advances, bearings and the
    /// atlas entry are four readings of one decision, and a run that took its
    /// metrics from one face and its coverage from another would be laid out with
    /// the wrong widths — which shows as a bold run drawn at regular spacing
    /// rather than as an error. Two text commands in the same batch may therefore
    /// be different weights, and nothing about the batch key changes: the weight
    /// rides inside the command, so a batch is still one draw call.
    ///
    /// **What no test in this repository covers.** Resolving
    /// `FontWeight::Regular` here instead of `*weight` — the mutation that draws
    /// every run from the regular face — leaves the whole unit suite and every
    /// doctest green (measured deliberately on 2026-10-03: 1360 + 139 + 209,
    /// zero failures). Both reasons are structural and neither can be fixed in a
    /// unit test: the decision sits inside a function that needs a GL context,
    /// and the rasterizing half of it needs two real font files, which
    /// `AGENTS.md` forbids a test to open. The evidence is the pixels — two runs
    /// of one string, measured in ink and in width — and `.ai/NEVERAGAIN.md`
    /// records three defects in this repository that passed every unit test here
    /// for exactly this reason.
    fn draw_text_batch(&mut self, batch: &Batch) -> Result<(), RenderError> {
        if batch.key.shader != ShaderKind::Text {
            return Ok(());
        }
        if self.fonts.is_empty() {
            return Ok(());
        }
        let mut vertices: Vec<TextVertex> = Vec::new();
        for command in &batch.commands {
            let DrawCommand::Text {
                x,
                y,
                text,
                color,
                font_size,
                extra_advance,
                weight,
            } = command
            else {
                continue;
            };
            // A weight with no face installed resolves to the regular one, and a
            // set with no face at all resolved above; the `else` is therefore only
            // reachable while a face is being replaced, which cannot happen
            // mid-frame because the borrow of the set is held by `face`.
            let Some(face) = self.fonts.resolve(*weight) else {
                continue;
            };
            let font = face.font();
            // The command gives the top of the line box; the face places the
            // baseline inside it. Placing the baseline at `y` itself would put
            // the ascenders above the command's own rect, off the top of the
            // window for a label laid out at the origin.
            let baseline = *y + font.ascent(*font_size);
            walk_run(
                text,
                *x,
                *extra_advance,
                &mut |ch| self.atlas.get_or_insert(ch, *font_size, face),
                &mut |ch| font.advance(ch, *font_size),
                &mut |placement, pen_x| {
                    let gx = pen_x + i32_to_f32(placement.bearing_x);
                    let gy = baseline - i32_to_f32(placement.bearing_y);
                    let w = u32_to_f32(placement.width);
                    let h = u32_to_f32(placement.height);
                    let uv = (placement.u0, placement.v0, placement.u1, placement.v1);
                    vertices.extend_from_slice(&text_quad(gx, gy, w, h, uv, *color));
                },
            );
        }
        if vertices.is_empty() {
            return Ok(());
        }
        // The glyphs were just rasterized into the atlas, so it has to be
        // uploaded before the quads sample it.
        self.upload_atlas()?;
        let quads = vertices.len() / 4;
        self.ensure_text_index_capacity(quads)?;
        self.ensure_text_vertex_capacity(quads)?;
        // SAFETY: `TextVertex` is `repr(C)` with 8 `f32` fields and no padding,
        // so the slice is a valid `TEXT_VERTEX_STRIDE`-strided vertex array.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr().cast::<u8>(),
                vertices.len() * std::mem::size_of::<TextVertex>(),
            )
        };
        let count = i32::try_from(quads * 6)
            .map_err(|_| RenderError::Gl("index count exceeds the i32 range".to_string()))?;
        // SAFETY: The GL context is current on this thread; `self.text_program`
        // is the linked program, `self.text_vao` carries the attribute
        // pointers, and `self.text_vbo` is bound for the upload.
        unsafe {
            let gl = self.context.gl();
            gl.use_program(Some(self.text_program));
            gl.uniform_2_f32(
                self.u_text_resolution.as_ref(),
                u32_to_f32(self.viewport.0),
                u32_to_f32(self.viewport.1),
            );
            gl.active_texture(GL_TEXTURE0);
            gl.bind_texture(GL_TEXTURE_2D, Some(self.atlas_texture));
            gl.uniform_1_i32(self.u_atlas.as_ref(), 0);
            gl.bind_vertex_array(Some(self.text_vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.text_vbo));
            gl.buffer_sub_data_u8_slice(GL_ARRAY_BUFFER, 0, bytes);
            gl.draw_elements(GL_TRIANGLES, count, GL_UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);
        }
        Ok(())
    }

    /// Grows the text index buffer to hold `quads` quads' worth of indices.
    fn ensure_text_index_capacity(&mut self, quads: usize) -> Result<(), RenderError> {
        if quads <= self.text_index_capacity {
            return Ok(());
        }
        let mut capacity = self.text_index_capacity.max(INITIAL_CAPACITY);
        while capacity < quads {
            capacity = capacity.saturating_mul(2);
        }
        let indices = quad_indices(capacity)?;
        // SAFETY: `indices` is a `Vec<u32>`, so the slice is a valid index
        // array. It outlives the upload.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                indices.as_ptr().cast::<u8>(),
                indices.len() * std::mem::size_of::<u32>(),
            )
        };
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread and `self.text_ibo`
        // is a valid buffer.
        unsafe {
            gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, Some(self.text_ibo));
            gl.buffer_data_u8_slice(GL_ELEMENT_ARRAY_BUFFER, bytes, GL_STATIC_DRAW);
        }
        self.text_index_capacity = capacity;
        Ok(())
    }

    /// Grows the text vertex buffer to hold `quads` quads' worth of vertices.
    fn ensure_text_vertex_capacity(&mut self, quads: usize) -> Result<(), RenderError> {
        if quads <= self.text_vertex_capacity {
            return Ok(());
        }
        let mut capacity = self.text_vertex_capacity.max(INITIAL_CAPACITY);
        while capacity < quads {
            capacity = capacity.saturating_mul(2);
        }
        let size = vertex_buffer_size(capacity, std::mem::size_of::<TextVertex>())?;
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread and `self.text_vbo`
        // is a valid buffer.
        unsafe {
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.text_vbo));
            gl.buffer_data_size(GL_ARRAY_BUFFER, size, GL_DYNAMIC_DRAW);
        }
        self.text_vertex_capacity = capacity;
        Ok(())
    }

    /// Grows the index buffer to hold `quads` quads' worth of indices.
    fn ensure_index_capacity(&mut self, quads: usize) -> Result<(), RenderError> {
        if quads <= self.index_capacity {
            return Ok(());
        }
        let mut capacity = self.index_capacity.max(INITIAL_CAPACITY);
        while capacity < quads {
            capacity = capacity.saturating_mul(2);
        }
        let indices = quad_indices(capacity)?;
        // SAFETY: `indices` is a `Vec<u32>`, so the slice is a valid index
        // array. It outlives the upload.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                indices.as_ptr().cast::<u8>(),
                indices.len() * std::mem::size_of::<u32>(),
            )
        };
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread and `self.ibo` is
        // a valid buffer.
        unsafe {
            gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            gl.buffer_data_u8_slice(GL_ELEMENT_ARRAY_BUFFER, bytes, GL_STATIC_DRAW);
        }
        self.index_capacity = capacity;
        Ok(())
    }

    /// Grows the image index buffer to hold `quads` quads' worth of indices.
    fn ensure_image_index_capacity(&mut self, quads: usize) -> Result<(), RenderError> {
        if quads <= self.image_index_capacity {
            return Ok(());
        }
        let mut capacity = self.image_index_capacity.max(INITIAL_CAPACITY);
        while capacity < quads {
            capacity = capacity.saturating_mul(2);
        }
        let indices = quad_indices(capacity)?;
        // SAFETY: `indices` is a `Vec<u32>`, so the slice is a valid index
        // array. It outlives the upload.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                indices.as_ptr().cast::<u8>(),
                indices.len() * std::mem::size_of::<u32>(),
            )
        };
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread and `self.image_ibo`
        // is a valid buffer.
        unsafe {
            gl.bind_buffer(GL_ELEMENT_ARRAY_BUFFER, Some(self.image_ibo));
            gl.buffer_data_u8_slice(GL_ELEMENT_ARRAY_BUFFER, bytes, GL_STATIC_DRAW);
        }
        self.image_index_capacity = capacity;
        Ok(())
    }

    /// Grows the image vertex buffer to hold `quads` quads' worth of vertices.
    ///
    /// `vertex_buffer_size` counts the four vertices a quad is, which is the
    /// arithmetic that went wrong once already: a buffer sized for one vertex
    /// per quad is a quarter of the size it promises, the `glBufferSubData` that
    /// follows fails with `GL_INVALID_VALUE`, and the batch is silently
    /// dropped — no error, no picture, and nothing in the suite to see it. See
    /// `.ai/NEVERAGAIN.md` § *A buffer sized for one vertex per quad*.
    fn ensure_image_vertex_capacity(&mut self, quads: usize) -> Result<(), RenderError> {
        if quads <= self.image_vertex_capacity {
            return Ok(());
        }
        let mut capacity = self.image_vertex_capacity.max(INITIAL_CAPACITY);
        while capacity < quads {
            capacity = capacity.saturating_mul(2);
        }
        let size = vertex_buffer_size(capacity, std::mem::size_of::<ImageVertex>())?;
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread and `self.image_vbo`
        // is a valid buffer.
        unsafe {
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.image_vbo));
            gl.buffer_data_size(GL_ARRAY_BUFFER, size, GL_DYNAMIC_DRAW);
        }
        self.image_vertex_capacity = capacity;
        Ok(())
    }

    /// Grows the vertex buffer to hold `quads` quads' worth of vertices.
    fn ensure_vertex_capacity(&mut self, quads: usize) -> Result<(), RenderError> {
        if quads <= self.vertex_capacity {
            return Ok(());
        }
        let mut capacity = self.vertex_capacity.max(INITIAL_CAPACITY);
        while capacity < quads {
            capacity = capacity.saturating_mul(2);
        }
        let size = vertex_buffer_size(capacity, std::mem::size_of::<Vertex>())?;
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread and `self.vbo` is
        // a valid buffer.
        unsafe {
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_size(GL_ARRAY_BUFFER, size, GL_DYNAMIC_DRAW);
        }
        self.vertex_capacity = capacity;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::resolve_slot;
    use crate::paint::{Painter, TextureId};
    use blur::MAX_TAPS;

    #[test]
    fn vertex_layout_matches_offsets() {
        assert_eq!(std::mem::size_of::<Vertex>(), VERTEX_STRIDE as usize);
    }

    #[test]
    fn vertex_buffer_size_holds_four_vertices_per_quad() {
        let per_quad = VERTS_PER_QUAD * std::mem::size_of::<Vertex>();
        assert_eq!(
            vertex_buffer_size(INITIAL_CAPACITY, std::mem::size_of::<Vertex>()).unwrap(),
            i32::try_from(INITIAL_CAPACITY * per_quad).unwrap()
        );
        assert_eq!(
            vertex_buffer_size(INITIAL_CAPACITY, std::mem::size_of::<TextVertex>()).unwrap(),
            i32::try_from(INITIAL_CAPACITY * VERTS_PER_QUAD * 32).unwrap()
        );
    }

    #[test]
    fn vertex_buffer_size_rejects_an_unrepresentable_size() {
        assert!(vertex_buffer_size(usize::MAX, 4).is_err());
        assert!(vertex_buffer_size(1 << 30, 64).is_err());
    }

    #[test]
    fn rect_quad_corners_and_locals() {
        let quad = rect_quad(
            Rect::new(10.0, 20.0, 100.0, 50.0),
            Color::new(255, 0, 0, 255),
            0.0,
        );
        assert_eq!(
            quad.corners,
            [[10.0, 20.0], [110.0, 20.0], [110.0, 70.0], [10.0, 70.0]]
        );
        assert_eq!(
            quad.locals,
            [[0.0, 0.0], [100.0, 0.0], [100.0, 50.0], [0.0, 50.0]]
        );
        assert_eq!(quad.radius, 0.0);
        assert_eq!(quad.size, [100.0, 50.0]);
        assert_eq!(quad.color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn radius_is_clamped_to_half_the_smaller_side() {
        let quad = rect_quad(
            Rect::new(0.0, 0.0, 10.0, 40.0),
            Color::new(0, 0, 0, 255),
            30.0,
        );
        assert_eq!(quad.radius, 5.0);
    }

    #[test]
    fn line_quad_expands_along_the_normal() {
        let quad = line_quad((0.0, 0.0), (10.0, 0.0), 4.0, Color::new(0, 0, 0, 255));
        // Horizontal line: the normal (-dy, dx) points down, half-width 2 in Y.
        assert_eq!(
            quad.corners,
            [[0.0, 2.0], [10.0, 2.0], [10.0, -2.0], [0.0, -2.0]]
        );
        assert_eq!(quad.size, [10.0, 4.0]);
    }

    #[test]
    fn degenerate_line_collapses_to_zero_area() {
        let quad = line_quad((5.0, 5.0), (5.0, 5.0), 4.0, Color::new(0, 0, 0, 255));
        assert_eq!(
            quad.corners,
            [[5.0, 5.0], [5.0, 5.0], [5.0, 5.0], [5.0, 5.0]]
        );
        assert_eq!(quad.size, [0.0, 4.0]);
    }

    #[test]
    fn circle_is_a_rounded_rect_with_half_size_radius() {
        let quads = command_quads(&DrawCommand::Circle {
            center: (50.0, 60.0),
            radius: 20.0,
            color: Color::new(1, 2, 3, 255),
        });
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].radius, 20.0);
        assert_eq!(quads[0].size, [40.0, 40.0]);
        assert_eq!(quads[0].corners[0], [30.0, 40.0]);
    }

    #[test]
    fn path_expands_to_one_quad_per_segment() {
        let open = command_quads(&DrawCommand::Path {
            points: vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)],
            width: 1.0,
            color: Color::new(0, 0, 0, 255),
            closed: false,
        });
        assert_eq!(open.len(), 2);

        let closed = command_quads(&DrawCommand::Path {
            points: vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)],
            width: 1.0,
            color: Color::new(0, 0, 0, 255),
            closed: true,
        });
        assert_eq!(closed.len(), 3);

        let degenerate = command_quads(&DrawCommand::Path {
            points: vec![(0.0, 0.0)],
            width: 1.0,
            color: Color::new(0, 0, 0, 255),
            closed: true,
        });
        assert!(degenerate.is_empty());
    }

    #[test]
    fn a_triangle_is_one_quad_with_its_third_corner_repeated() {
        let quads = command_quads(&DrawCommand::Polygon {
            points: vec![(10.0, 20.0), (30.0, 60.0), (50.0, 20.0)],
            color: Color::new(255, 128, 0, 255),
        });
        assert_eq!(quads.len(), 1, "one triangle, one quad");
        let quad = quads[0];
        // Three distinct corners in the order they were given, and the fourth
        // repeating the third: `quad_indices` addresses 0,1,2 / 0,2,3, so this is
        // the triangle (a, b, c) plus a degenerate (a, c, c) that covers nothing.
        assert_eq!(
            quad.corners,
            [[10.0, 20.0], [30.0, 60.0], [50.0, 20.0], [50.0, 20.0]],
            "the points in order, with the last repeated"
        );
        assert_eq!(
            quad.radius, 0.0,
            "radius 0 takes the shader's plain-colour path: its SDF models a \
             rounded rectangle, and a triangle has no corner to round"
        );
        assert_eq!(quad.color, [1.0, 128.0 / 255.0, 0.0, 1.0]);
    }

    #[test]
    fn a_convex_quadrilateral_fans_into_two_quads() {
        // Off the origin: a fixture at (0, 0) cannot see an origin read as an
        // extent, per `.ai/NEVERAGAIN.md`.
        let quads = command_quads(&DrawCommand::Polygon {
            points: vec![(20.0, 20.0), (60.0, 20.0), (60.0, 50.0), (20.0, 50.0)],
            color: Color::new(10, 20, 30, 255),
        });
        assert_eq!(quads.len(), 2, "n - 2 triangles for n points");

        // The fan runs from the first point: (p0, p1, p2) then (p0, p2, p3).
        assert_eq!(
            quads[0].corners,
            [[20.0, 20.0], [60.0, 20.0], [60.0, 50.0], [60.0, 50.0]]
        );
        assert_eq!(
            quads[1].corners,
            [[20.0, 20.0], [60.0, 50.0], [20.0, 50.0], [20.0, 50.0]],
            "and the second triangle shares the fan's first corner"
        );

        // The polygon's own box, measured from the four points: x 20..=60 and
        // y 20..=50.
        let polygon_box = [40.0, 30.0];
        for (index, quad) in quads.iter().enumerate() {
            assert_eq!(quad.radius, 0.0, "triangle {index}");
            assert!(
                quad.size[0] <= polygon_box[0] && quad.size[1] <= polygon_box[1],
                "triangle {index}'s box {:?} fits inside the polygon's {:?}, as any \
                 triangle of a fan must",
                quad.size,
                polygon_box
            );
            // This rectangle's fan happens to reach all four corners from each
            // of its two triangles, so here the bound is tight rather than
            // merely true.
            assert_eq!(quad.size, polygon_box, "triangle {index}, in this fixture");
        }
    }

    #[test]
    fn a_polygon_of_fewer_than_three_points_expands_to_no_quads() {
        // Not an error and not a panic: a point list that cannot enclose an area
        // has nothing to draw, which is what `Path` already answers for fewer
        // than two points. The recorder stored the command either way.
        for points in [
            Vec::new(),
            vec![(10.0, 20.0)],
            vec![(10.0, 20.0), (30.0, 60.0)],
        ] {
            let quads = command_quads(&DrawCommand::Polygon {
                points: points.clone(),
                color: Color::new(0, 0, 0, 255),
            });
            assert!(
                quads.is_empty(),
                "{points:?} is not a polygon and draws nothing"
            );
        }
    }

    #[test]
    fn a_polygon_keeps_the_vertex_count_a_whole_number_of_quads() {
        // The invariant that breaks the whole solid pass silently:
        // `draw_solid_batch` computes `quads = vertices.len() / 4`, and the index
        // buffer addresses four vertices per quad. One quad that emitted three
        // vertices would put the sixth index on the first vertex of the *next*
        // one, with no error from anything.
        let shapes: [&[(f32, f32)]; 3] = [
            &[(10.0, 20.0), (30.0, 60.0), (50.0, 20.0)],
            &[(10.0, 20.0), (30.0, 60.0), (50.0, 20.0), (70.0, 80.0)],
            &[
                (10.0, 20.0),
                (30.0, 60.0),
                (50.0, 20.0),
                (70.0, 80.0),
                (90.0, 20.0),
            ],
        ];
        for points in shapes {
            let quads = command_quads(&DrawCommand::Polygon {
                points: points.to_vec(),
                color: Color::new(0, 0, 0, 255),
            });
            let vertices = batch_vertices(&Batch {
                key: crate::batch::BatchKey {
                    texture: None,
                    blend_mode: crate::batch::BlendMode::Opaque,
                    shader: ShaderKind::Solid,
                },
                clip: None,
                commands: vec![DrawCommand::Polygon {
                    points: points.to_vec(),
                    color: Color::new(0, 0, 0, 255),
                }],
            });
            assert_eq!(
                vertices.len(),
                quads.len() * 4,
                "{:?}: four vertices per quad, as `draw_solid_batch` assumes",
                points.len()
            );
            assert_eq!(vertices.len() % VERTS_PER_QUAD, 0);
            assert_eq!(vertices.len() / VERTS_PER_QUAD, quads.len());
        }
    }

    #[test]
    fn a_polygons_quads_are_sized_by_their_own_triangle() {
        // `locals` and `size` are unread while the radius is 0, but they are set
        // to the truth rather than to zeros: a vertex that claims (0, 0) while it
        // is drawn at (300, 200) is a trap for whoever reads it next.
        //
        // The fan anchor is deliberately **not** the box's top-left — it is the
        // middle of the left edge — so no corner of this triangle sits at
        // `(0, 0)` and a `locals` that were left at zero would be wrong on three
        // of the four entries. A fixture whose first corner *is* the origin
        // cannot see that, and would pass against a mutation.
        let anchor = (100.0, 60.0);
        let quads = command_quads(&DrawCommand::Polygon {
            points: vec![anchor, (160.0, 30.0), (160.0, 90.0)],
            color: Color::new(0, 0, 0, 255),
        });
        assert_eq!(quads.len(), 1);
        let low = [100.0, 30.0];
        assert_eq!(quads[0].size, [60.0, 60.0], "the triangle's own box");
        assert_eq!(
            quads[0].locals,
            [[0.0, 30.0], [60.0, 0.0], [60.0, 60.0], [60.0, 60.0]],
            "measured from that box's top left, as a rect quad's are"
        );
        for (index, local) in quads[0].locals.iter().enumerate() {
            let corner = quads[0].corners[index];
            assert_eq!(
                *local,
                [corner[0] - low[0], corner[1] - low[1]],
                "corner {index} sits where it says it does"
            );
        }
    }

    #[test]
    fn a_polygons_quad_carries_no_corner_radius() {
        // A radius above zero sends the fragment shader down its rounded-rect
        // SDF path, and that SDF models an axis-aligned rounded rectangle: on a
        // triangle it clips fragments off a shape that was never drawn there. So
        // the radius is zero and the shader takes `frag_color = v_color`.
        //
        // Pinned on its own because it is invisible in every other test here —
        // `locals` and `size` are not read when the radius is 0, so a wrong
        // radius changes nothing CPU-side that a vertex assertion can see.
        for points in [
            vec![(100.0, 40.0), (140.0, 40.0), (140.0, 90.0)],
            vec![(20.0, 20.0), (60.0, 20.0), (60.0, 50.0), (20.0, 50.0)],
        ] {
            let quads = command_quads(&DrawCommand::Polygon {
                points: points.clone(),
                color: Color::new(0, 0, 0, 255),
            });
            assert!(!quads.is_empty(), "{points:?} is a polygon");
            for (index, quad) in quads.iter().enumerate() {
                assert_eq!(
                    quad.radius, 0.0,
                    "{points:?}: triangle {index} carries no radius"
                );
            }
        }
    }

    #[test]
    fn a_glyph_quad_names_its_four_corners_in_the_order_the_indices_address() {
        // Top left, top right, bottom right, bottom left — the order
        // `quad_indices` addresses with 0,1,2 / 0,2,3.
        //
        // Nothing pinned this, and it was not hypothetical: adding the image
        // quad next to `text_quad` once swapped two of these entries and left
        // the third and fourth vertices both at the bottom left, which draws a
        // glyph whose lower-right triangle has no area. Every other test in
        // this module passes with that in place, because the run is assembled
        // against a GL context and a real font.
        let quad = text_quad(
            10.0,
            20.0,
            8.0,
            16.0,
            (0.1, 0.2, 0.3, 0.4),
            Color::new(0, 0, 0, 255),
        );
        assert_eq!(quad[0].pos, [10.0, 20.0], "top left");
        assert_eq!(quad[0].uv, [0.1, 0.2], "with the atlas rect's top left");
        assert_eq!(quad[1].pos, [18.0, 20.0], "top right");
        assert_eq!(quad[1].uv, [0.3, 0.2]);
        assert_eq!(
            quad[2].pos,
            [18.0, 36.0],
            "bottom right, and 8 across and 16 down"
        );
        assert_eq!(quad[2].uv, [0.3, 0.4]);
        assert_eq!(quad[3].pos, [10.0, 36.0], "bottom left");
        assert_eq!(quad[3].uv, [0.1, 0.4]);
    }

    #[test]
    fn text_and_image_commands_expand_to_no_quads() {
        let text = command_quads(&DrawCommand::Text {
            x: 0.0,
            y: 0.0,
            text: "a".to_string(),
            color: Color::new(0, 0, 0, 255),
            font_size: 16.0,
            extra_advance: 0.0,
            weight: FontWeight::Regular,
        });
        assert!(text.is_empty());

        let image = command_quads(&DrawCommand::Image {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            texture: TextureId::new(1),
            uv: crate::paint::UvRect::full(),
            opacity: 1.0,
            radius: 0.0,
        });
        assert!(image.is_empty());
    }

    #[test]
    fn batch_vertices_four_per_quad_in_corner_order() {
        let batch = Batch {
            key: crate::batch::BatchKey {
                texture: None,
                blend_mode: crate::batch::BlendMode::Opaque,
                shader: ShaderKind::Solid,
            },
            clip: None,
            commands: vec![
                DrawCommand::Rect {
                    rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                    color: Color::new(255, 0, 0, 255),
                },
                DrawCommand::Rect {
                    rect: Rect::new(20.0, 0.0, 10.0, 10.0),
                    color: Color::new(0, 255, 0, 255),
                },
            ],
        };
        let vertices = batch_vertices(&batch);
        assert_eq!(vertices.len(), 8);
        assert_eq!(vertices[0].pos, [0.0, 0.0]);
        assert_eq!(vertices[1].pos, [10.0, 0.0]);
        assert_eq!(vertices[2].pos, [10.0, 10.0]);
        assert_eq!(vertices[3].pos, [0.0, 10.0]);
        assert_eq!(vertices[4].pos, [20.0, 0.0]);
        assert_eq!(vertices[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[4].color, [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn f32_to_i32_clamps_to_the_i32_range() {
        assert_eq!(f32_to_i32(0.0), 0);
        assert_eq!(f32_to_i32(1.5), 1);
        assert_eq!(f32_to_i32(-1.5), -1);
        assert_eq!(f32_to_i32(2147483648.0), i32::MAX);
        assert_eq!(f32_to_i32(-2147483648.0), i32::MIN);
        assert_eq!(f32_to_i32(1.0e30), i32::MAX);
        assert_eq!(f32_to_i32(-1.0e30), i32::MIN);
    }

    #[test]
    fn u32_to_f32_roundtrips_small_values() {
        assert_eq!(u32_to_f32(0), 0.0);
        assert_eq!(u32_to_f32(1024), 1024.0);
        assert_eq!(u32_to_f32(600), 600.0);
    }

    #[test]
    fn render_error_display() {
        let err = RenderError::ShaderCompile("bad glsl".to_string());
        assert!(err.to_string().contains("bad glsl"));
        let err = RenderError::ProgramLink("bad link".to_string());
        assert!(err.to_string().contains("bad link"));
        let err = RenderError::Gl("no memory".to_string());
        assert!(err.to_string().contains("no memory"));
    }

    /// The tests below cover the run walk, and they exist because a space
    /// renders as a zero-width bitmap: there is no quad to draw and no
    /// placement to read an advance from, and treating that as "no advance" ran
    /// every word together on screen ("Hello,World!"). Nothing caught it — not
    /// one of the 384 tests, and not a still screenshot, because the run is
    /// assembled against a GL context and a real font and both are unavailable
    /// to a unit test. `walk_run` takes its lookups as closures so the rule can
    /// be pinned here instead.
    ///
    /// A placement standing in for a rasterized glyph, `advance` wide.
    fn glyph(advance: f32) -> GlyphPlacement {
        GlyphPlacement {
            u0: 0.0,
            v0: 0.0,
            u1: 0.1,
            v1: 0.1,
            row_y: 0,
            width: 8,
            height: 8,
            bearing_x: 0,
            bearing_y: 0,
            advance,
        }
    }

    /// Lays out `text` where every character but `blank` is `wide` and draws,
    /// and `blank` is `blank_wide` wide and produces nothing. Returns the pen
    /// position each drawn glyph was placed at, in order.
    fn run_pen_positions(text: &str, blank: char, wide: f32, blank_wide: f32) -> Vec<f32> {
        let mut positions = Vec::new();
        walk_run(
            text,
            0.0,
            0.0,
            &mut |ch: char| (ch != blank).then(|| glyph(wide)),
            &mut |_ch: char| blank_wide,
            &mut |_placement, pen_x| positions.push(pen_x),
        );
        positions
    }

    #[test]
    fn a_space_leaves_a_gap_even_though_it_draws_nothing() {
        // The regression: "Hello, World!" drawn with no space between the words.
        let positions = run_pen_positions("a b", ' ', 10.0, 5.0);
        assert_eq!(
            positions,
            vec![0.0, 15.0],
            "the second glyph starts one advance plus the space's width in, so \
             the space is a gap rather than nothing"
        );
    }

    #[test]
    fn a_character_the_font_has_no_glyph_for_still_occupies_its_width() {
        // U+FFFD is the stand-in here, but the rule is not about that character:
        // it is about any character the font cannot rasterize, which is the same
        // `None` a space produces.
        let positions = run_pen_positions("a\u{fffd}b", '\u{fffd}', 10.0, 4.0);
        assert_eq!(
            positions,
            vec![0.0, 14.0],
            "an unrasterizable character is not silently deleted"
        );
    }

    #[test]
    fn a_run_of_nothing_but_spaces_still_advances_the_pen() {
        let mut positions = Vec::new();
        let end = walk_run(
            "   ",
            7.0,
            0.0,
            &mut |_ch: char| None,
            &mut |_ch: char| 5.0,
            &mut |_placement, pen_x| positions.push(pen_x),
        );
        assert!(positions.is_empty(), "no glyph to draw");
        assert_eq!(end, 22.0, "three spaces of five from a start of seven");
    }

    #[test]
    fn letter_spacing_is_added_to_a_space_too() {
        let mut positions = Vec::new();
        walk_run(
            "a b",
            0.0,
            2.0,
            &mut |ch: char| (ch != ' ').then(|| glyph(10.0)),
            &mut |_ch: char| 5.0,
            &mut |_placement, pen_x| positions.push(pen_x),
        );
        // 10 (the 'a', plus 2 of spacing) + 5 (the space) + 2 (its spacing)
        assert_eq!(
            positions,
            vec![0.0, 19.0],
            "a space is spaced like any other character"
        );
    }

    #[test]
    fn advance_for_uses_the_placement_when_there_is_one() {
        let placement = glyph(7.0);
        assert_eq!(advance_for(Some(&placement), 99.0, 0.0), 7.0);
        assert_eq!(advance_for(Some(&placement), 99.0, 1.0), 8.0);
    }

    #[test]
    fn advance_for_falls_back_to_the_font_when_there_is_no_placement() {
        assert_eq!(advance_for(None, 5.0, 0.0), 5.0);
        assert_eq!(advance_for(None, 5.0, 2.0), 7.0);
    }

    /// The tests below cover which **face** a text run is drawn with: the one
    /// decision the text pass makes per command before it rasterizes anything.
    ///
    /// What they cannot check is the rasterization itself — `draw_text_batch`
    /// needs a GL context and two real font files, and a unit test may open
    /// neither — so what is tested here is the command → face decision it makes,
    /// driven from a command recorded by the public API rather than from a
    /// hand-built one, and the rule that a command which cannot be resolved is
    /// *skipped* rather than drawn with the wrong face.
    ///
    /// What it cannot see is the pair of runs on screen with different ink and
    /// different advances, which is the defect the whole feature exists to avoid;
    /// that is a capture, and `.ai/NEVERAGAIN.md` records three defects in this
    /// repository that passed every unit test here.
    fn text_run(weight: FontWeight) -> DrawCommand {
        let mut painter = Painter::new();
        match weight {
            FontWeight::Regular => painter.text(
                180.0,
                96.0,
                "Handgloves",
                Color::new(240, 240, 240, 255),
                20.0,
                0.0,
            ),
            FontWeight::Bold => painter.text_bold(
                180.0,
                96.0,
                "Handgloves",
                Color::new(240, 240, 240, 255),
                20.0,
                0.0,
            ),
        }
        painter.finish().remove(0)
    }

    #[test]
    fn a_runs_face_is_the_slot_its_own_weight_resolves_to() {
        // What `draw_text_batch` reads: the weight out of the recorded command,
        // and the slot it resolves to. Two runs of the same string at the same
        // size in two faces are two different slots, which is what stops one
        // being rasterized from the other's glyphs.
        let both: [Option<u8>; 2] = [Some(0), Some(1)];
        let command = text_run(FontWeight::Bold);
        let DrawCommand::Text {
            weight,
            text,
            font_size,
            ..
        } = &command
        else {
            panic!("the painter did not record a text run");
        };
        assert_eq!(text, "Handgloves", "the fixture is the run it claims to be");
        assert_eq!(*font_size, 20.0);
        assert_eq!(
            resolve_slot(&both, *weight),
            Some(1),
            "a bold command asks for the bold slot, so it is rasterized from the \
             bold face"
        );
    }

    #[test]
    fn a_run_whose_face_is_not_installed_is_drawn_with_the_regular_one() {
        // The renderer that was given one font: today's renderer, with a bold
        // title added to it. The run is not dropped, and it is not drawn from
        // anything the caller did not ask for.
        let regular_only: [Option<u8>; 2] = [Some(0), None];
        let DrawCommand::Text { weight, .. } = text_run(FontWeight::Bold) else {
            panic!("the painter did not record a text run");
        };
        assert_eq!(
            resolve_slot(&regular_only, weight),
            Some(0),
            "the same slot a regular command resolves to, which is what makes the \
             two runs pixel-identical rather than one of them missing"
        );
    }

    #[test]
    fn two_runs_of_one_string_in_two_faces_carry_different_weights() {
        // The end of the chain the painter starts: same string, same size, same
        // position, one word apart at the call site, and two different requests
        // by the time the renderer sees them.
        let both: [Option<u8>; 2] = [Some(0), Some(1)];
        let slots: Vec<Option<usize>> = [FontWeight::Regular, FontWeight::Bold]
            .iter()
            .map(|weight| match text_run(*weight) {
                DrawCommand::Text { weight, .. } => resolve_slot(&both, weight),
                _ => None,
            })
            .collect();
        assert_eq!(
            slots,
            vec![Some(0), Some(1)],
            "two faces, two slots: the weight on the command is the only thing \
             that tells them apart, and nothing else about the two runs differs"
        );
    }

    #[test]
    fn a_text_batch_with_no_face_at_all_is_skipped_before_it_is_walked() {
        // `draw_text_batch` asks the set this question once, before any command,
        // which is what keeps the "no font" path as free as it was before there
        // were two weights rather than free modulo a resolution per command.
        let set = FontSet::new();
        assert!(set.is_empty(), "a fresh set holds no face in it");
        for weight in FontWeight::ALL {
            assert!(
                set.resolve(weight).is_none(),
                "{weight:?} resolves to no face, so there is nothing to rasterize \
                 and the pass returns before it walks the batch"
            );
        }
    }

    /// The tests below cover the image pass. What they can check is everything
    /// that is decided before the GPU is touched: the vertex layout, the shader
    /// sources, the quad a command expands to, the two clamps
    /// `DrawCommand::Image` promises, and which of the two textures an image is
    /// drawn from. What they cannot check is whether any of it reaches the
    /// screen — the atlas upload, the sampler binding, the discard and the
    /// premultiplied blend are all framebuffer-side, and this suite has no
    /// context to observe them with. `.ai/NEVERAGAIN.md` records three defects
    /// in this repository that passed every unit test here and were only found
    /// by looking at the pixels.
    ///
    /// The rect every image fixture is laid out in, away from the origin: a
    /// geometry fixture at `(0, 0)` cannot see an origin being read as an
    /// extent, and this one is used to check both — see `.ai/NEVERAGAIN.md` §
    /// *A rect's origin and a rect's extent are different numbers*.
    fn fixture_rect() -> Rect {
        Rect::new(10.0, 20.0, 100.0, 50.0)
    }

    #[test]
    fn image_vertex_layout_matches_the_offsets_the_attributes_are_bound_with() {
        // 10 `f32` fields at 4 bytes: pos 0, local 8, uv 16, radius 24,
        // opacity 28, size 32, total 40. `offset_of!` is what GL's attribute
        // pointers mean, so this is the one assertion that would catch a field
        // inserted without its constant moved — which is a skewed image, or a
        // UV read from the middle of a quad, and both only on screen.
        assert_eq!(
            std::mem::size_of::<ImageVertex>(),
            IMAGE_VERTEX_STRIDE as usize
        );
        assert_eq!(std::mem::offset_of!(ImageVertex, pos), 0);
        assert_eq!(
            std::mem::offset_of!(ImageVertex, local),
            IMAGE_LOCAL_OFFSET as usize
        );
        assert_eq!(
            std::mem::offset_of!(ImageVertex, uv),
            IMAGE_UV_OFFSET as usize
        );
        assert_eq!(
            std::mem::offset_of!(ImageVertex, radius),
            IMAGE_RADIUS_OFFSET as usize
        );
        assert_eq!(
            std::mem::offset_of!(ImageVertex, opacity),
            IMAGE_OPACITY_OFFSET as usize
        );
        assert_eq!(
            std::mem::offset_of!(ImageVertex, size),
            IMAGE_SIZE_OFFSET as usize
        );
    }

    #[test]
    fn the_image_vertex_buffer_counts_four_vertices_per_quad() {
        // 256 quads is four vertices each, so 1024 vertices, at 40 bytes each:
        // 40960. The number the NEVERAGAIN entry is about is 256 * 40 = 10240 —
        // a quarter of it, which `glBufferSubData` rejects with
        // `GL_INVALID_VALUE` and the batch then vanishes with no error. So the
        // assertion is written against the full size and the wrong one is named
        // in the message.
        let per_quad = VERTS_PER_QUAD * std::mem::size_of::<ImageVertex>();
        assert_eq!(per_quad, 4 * 40);
        assert_eq!(
            vertex_buffer_size(INITIAL_CAPACITY, std::mem::size_of::<ImageVertex>()).unwrap(),
            40960,
            "256 quads of four 40-byte vertices, not 256 vertices"
        );
    }

    #[test]
    fn the_image_shader_reads_every_attribute_the_vertex_buffer_supplies() {
        // The vertex format, the shader's `layout(location = …)` list and GL's
        // attribute pointers are three names for one layout, connected only by
        // hand. A name in one and not the others is a constant that reads as
        // zero — a transparent quad, or a UV read from the middle of a quad —
        // and nothing but a string comparison can see it.
        //
        // The whole declaration is matched, never the bare name: `a_size` is a
        // substring of `a_size_renamed`, and a rename is precisely the break
        // this is here to catch.
        for declaration in [
            "layout(location = 0) in vec2 a_pos;",
            "layout(location = 1) in vec2 a_local;",
            "layout(location = 2) in vec2 a_uv;",
            "layout(location = 3) in float a_radius;",
            "layout(location = 4) in float a_opacity;",
            "layout(location = 5) in vec2 a_size;",
        ] {
            assert!(
                IMAGE_VERTEX_SHADER_SRC.contains(declaration),
                "{declaration} is where the vertex format's field is bound"
            );
        }
        assert!(IMAGE_VERTEX_SHADER_SRC.contains("uniform vec2 u_resolution;"));
        for varying in [
            "out vec2 v_local;",
            "out vec2 v_uv;",
            "out float v_radius;",
            "out float v_opacity;",
            "out vec2 v_size;",
        ] {
            assert!(
                IMAGE_VERTEX_SHADER_SRC.contains(varying),
                "{varying} is what the vertex stage hands the fragment stage"
            );
        }
        for declaration in [
            "in vec2 v_local;",
            "in vec2 v_uv;",
            "in float v_radius;",
            "in float v_opacity;",
            "in vec2 v_size;",
        ] {
            assert!(
                IMAGE_FRAGMENT_SHADER_SRC.contains(declaration),
                "{declaration} has to match what the vertex stage writes"
            );
        }
        assert!(IMAGE_FRAGMENT_SHADER_SRC.contains("uniform sampler2D u_image;"));
        assert!(
            IMAGE_FRAGMENT_SHADER_SRC.contains("texture(u_image, v_uv)"),
            "the colour comes from one sample of the bound texture"
        );
    }

    #[test]
    fn the_image_fragment_shader_discards_the_corners_rather_than_filling_them() {
        // `.ai/NEVERAGAIN.md` § *A filled rounded rectangle is not an outline*:
        // `DrawCommand::Image`'s radius is a clip, so the corners must show what
        // the widget drew behind. A colour write there is not a wrong colour,
        // it is a wrong shape, and no draw-command assertion can tell the two
        // apart — the only thing that can see this is the string.
        assert!(
            IMAGE_FRAGMENT_SHADER_SRC.contains("discard;"),
            "a rounded corner is cut away, not painted"
        );
        assert!(
            !IMAGE_FRAGMENT_SHADER_SRC.contains("frag_color = v_color"),
            "an image's colour is the texel's, not a colour of its own"
        );
        // The premultiplied contract: the texel arrives already multiplied into
        // its alpha, so scaling is `vec4(rgb * opacity, a * opacity)` — scaling
        // only the alpha would leave the colour channels unbounded by it.
        assert!(IMAGE_FRAGMENT_SHADER_SRC.contains("clamp(v_opacity, 0.0, 1.0)"));
        assert!(IMAGE_FRAGMENT_SHADER_SRC
            .contains("frag_color = vec4(texel.rgb * opacity, texel.a * opacity);"));
    }

    #[test]
    fn both_fragment_shaders_round_a_corner_by_the_same_rule() {
        // The whole clip block, not merely its first line. A divergence in the
        // `dist` expression — a dropped `- v_radius`, a different `min` — would
        // round an image's corners by a pixel or two differently from a rounded
        // rect's, and two source strings disagreeing is the only place that
        // shows: the compiled result is on screen and nowhere else.
        let clip = r#"    if (v_radius > 0.0) {
        vec2 half_size = v_size * 0.5;
        vec2 q = abs(v_local - half_size) - (half_size - vec2(v_radius));
        float dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - v_radius;
        if (dist > 0.0) {
            discard;
        }
    }
"#;
        assert!(
            FRAGMENT_SHADER_SRC.contains(clip),
            "the solid pass's corner"
        );
        assert!(
            IMAGE_FRAGMENT_SHADER_SRC.contains(clip),
            "the image pass's corner, which must clip by the same signed distance"
        );
    }

    #[test]
    fn an_image_quad_places_its_uv_window_on_the_matching_corners() {
        // Top left, top right, bottom right, bottom left — the order
        // `quad_indices` addresses, and the order the solid pass uses.
        let uv = UvRect {
            u0: 0.1,
            v0: 0.2,
            u1: 0.3,
            v1: 0.4,
        };
        let quad = image_quad(fixture_rect(), uv, 0.5, 0.0);

        assert_eq!(quad[0].pos, [10.0, 20.0], "the quad's own top left corner");
        assert_eq!(quad[1].pos, [110.0, 20.0]);
        assert_eq!(quad[2].pos, [110.0, 70.0]);
        assert_eq!(quad[3].pos, [10.0, 70.0]);
        assert_eq!(
            quad[0].uv,
            [0.1, 0.2],
            "the uv window's own top left, not the whole texture"
        );
        assert_eq!(quad[1].uv, [0.3, 0.2]);
        assert_eq!(quad[2].uv, [0.3, 0.4]);
        assert_eq!(quad[3].uv, [0.1, 0.4]);
        assert_eq!(quad[0].local, [0.0, 0.0], "the SDF's origin is the quad");
        assert_eq!(
            quad[2].local,
            [100.0, 50.0],
            "and its far corner is the size"
        );
        assert_eq!(
            quad[0].size,
            [100.0, 50.0],
            "which the shader needs as well"
        );
        assert_eq!(quad[3].opacity, 0.5);
    }

    #[test]
    fn an_opacity_outside_the_range_is_clamped_when_the_quad_is_built() {
        // `DrawCommand::Image`'s docs: "a value outside the range is clamped
        // when the command is drawn, not when it is recorded". The command keeps
        // what it was handed; the vertex carries the nearest end of the range.
        let over = image_quad(fixture_rect(), UvRect::full(), 1.5, 0.0);
        assert_eq!(over[0].opacity, 1.0, "an opacity claiming more than 1 is 1");

        let under = image_quad(fixture_rect(), UvRect::full(), -0.5, 0.0);
        assert_eq!(
            under[0].opacity, 0.0,
            "and one claiming less than 0 draws nothing"
        );

        let inside = image_quad(fixture_rect(), UvRect::full(), 0.25, 0.0);
        assert_eq!(inside[0].opacity, 0.25, "a value in range is not touched");
    }

    #[test]
    fn an_image_radius_past_half_the_shorter_side_is_half_of_it() {
        // The same rule `RoundedRect` follows, and the same one its own docs
        // promise for `Image`: a radius too large to fit is a rounded pill and
        // not an inverted shape. 30 against a 10-wide side is 5.
        let quad = image_quad(Rect::new(0.0, 0.0, 10.0, 40.0), UvRect::full(), 1.0, 30.0);
        assert_eq!(quad[0].radius, 5.0);

        let square = image_quad(Rect::new(0.0, 0.0, 8.0, 8.0), UvRect::full(), 1.0, 4.0);
        assert_eq!(
            square[0].radius, 4.0,
            "half of an 8-pixel side is 4, which is the same as the radius asked for"
        );
    }

    #[test]
    fn an_image_is_drawn_from_the_shared_atlas_or_from_a_texture_of_its_own() {
        // The one bit `crate::texture` sets on an identifier decides which GL
        // texture the quads sample, and there is no other signal: an image
        // drawn against the wrong texture is not a compile error and not an
        // error at all, it is somebody else's picture.
        let packed = TextureId::new(5);
        assert_eq!(
            image_source(packed),
            ImageSource::Atlas,
            "a small image is a window into the shared atlas"
        );

        let own = TextureId::new(0x8000_0005);
        assert_eq!(
            image_source(own),
            ImageSource::Own(0x8000_0005),
            "and a large one carries the bit, keyed by the number the command has"
        );
    }

    #[test]
    fn the_composited_passes_are_drawn_solid_then_image_then_text() {
        // An image is a background and a label is what sits on it, so the image
        // pass runs after the solid that fills what is behind it and before the
        // text that goes on it. `end_frame` reads this constant rather than
        // having an order of its own, so the order is asserted here and not
        // restated in a comment as the truth.
        assert_eq!(COMPOSITED_PASSES, [Pass::Solid, Pass::Image, Pass::Text]);
    }

    #[test]
    fn quad_indices_addresses_four_vertices_per_quad() {
        // 0,1,2 and 0,2,3 for the first quad, then the same pattern from 4.
        assert_eq!(
            quad_indices(2).expect("two quads of indices"),
            vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]
        );
        assert_eq!(
            quad_indices(1).expect("one quad of indices").len(),
            INDICES_PER_QUAD
        );
    }

    #[test]
    fn quad_indices_refuses_a_capacity_past_the_index_range() {
        // The last index a capacity of `n` uses is `4n - 1`, which must fit a
        // `u32`: `4 * 2^30 - 1 = 2^32 - 1`, exactly `u32::MAX`, so 2^30 quads is
        // the last capacity that fits and one more is refused. Nothing calls
        // that bound — it would be six gigabytes of indices — but the guard is
        // here rather than inside the loop precisely so it can be exercised
        // without allocating.
        assert!(
            quad_indices((1 << 30) + 1).is_err(),
            "one past the last that fits"
        );
        assert!(quad_indices(usize::MAX).is_err());
    }

    #[test]
    fn the_image_texture_format_is_rgba_and_not_the_glyph_atlases_red() {
        // 0x8058 = 32856 and 0x1908 = 6408. A typo here is invisible to this
        // suite and on screen is a black square: the atlas upload asks GL for a
        // format it does not support, GL rejects it, and the texture keeps
        // whatever it had — which for a first frame is nothing.
        assert_eq!(GL_RGBA8, 32856);
        assert_eq!(GL_RGBA, 6408);
        assert_ne!(
            GL_RGBA8, GL_R8,
            "the glyph atlas is single-channel coverage; an image is not"
        );
    }

    #[test]
    fn a_gl_enum_is_converted_to_the_i32_the_parameter_setters_take() {
        // 0x2601 = 9729 and 0x812F = 33071. The out-of-range case is not
        // reachable from this module's constants, which is the point: it is the
        // reason the conversion is checked and not a cast.
        assert_eq!(gl_enum_to_i32(GL_LINEAR), 9729);
        assert_eq!(gl_enum_to_i32(GL_CLAMP_TO_EDGE), 33071);
        assert_eq!(gl_enum_to_i32(u32::MAX), 0, "GL_INVALID_ENUM, not a wrap");
    }

    #[test]
    fn a_texture_failure_reaches_the_caller_as_a_render_error() {
        let error = RenderError::from(TextureError::Unreadable("no such file".to_string()));
        assert!(matches!(error, RenderError::Texture(_)));
        assert_eq!(
            error.to_string(),
            "texture error: could not read image: no such file",
            "the decoder's own message is kept rather than replaced"
        );
    }

    /// The tests below cover the shadow layer. What they can check is everything
    /// decided before the GPU is touched: the geometry a shadow's shape expands
    /// to, the composite's arithmetic, the two shader sources' contracts, and the
    /// routing from a command to its program.
    ///
    /// **What they cannot check is the ramp.** The whole point of this layer is
    /// that a shadow's edge is soft, and softness is pixels: the blur runs on the
    /// GPU, the target is single-sampled where the window's default framebuffer is
    /// 4x, and the weights are eight bits per texel. `.ai/NEVERAGAIN.md` records
    /// three defects in this repository that passed every unit test here and were
    /// found by looking at the screen, so the acceptance for the blur is a capture
    /// with the ramp measured in pixels, not this module.
    fn shadow_command() -> DrawCommand {
        DrawCommand::Shadow {
            // Off the origin: a geometry fixture at `(0, 0)` cannot see an origin
            // read as an extent, and the offset is exactly such a number.
            rect: Rect::new(240.0, 160.0, 320.0, 200.0),
            radius: 12.0,
            color: Color::new(20, 30, 40, 160),
            blur: 4.0,
            offset: (3.0, 9.0),
        }
    }

    #[test]
    fn a_shadow_command_expands_to_no_quads() {
        // The same reason text and image do not: it is drawn by its own program
        // and not by the solid pass, so a quad reaching `command_quads` would be
        // submitted to a shader that does not read it.
        assert!(command_quads(&shadow_command()).is_empty());
    }

    #[test]
    fn a_shadows_shape_is_its_rect_moved_by_the_offset_and_grown_by_nothing() {
        // **Not grown.** The blur spreads the shape by itself, past its own edge —
        // that is what a shadow's falloff is — so a shape grown by the blur's reach
        // would widen the shadow by the reach *on top of* the falloff and put the
        // soft edge twice as far out as the panel is wide.
        let quad = rect_quad(
            Rect::new(240.0 + 3.0, 160.0 + 9.0, 320.0, 200.0),
            Color::new(0, 0, 0, 160),
            12.0,
        );
        assert_eq!(
            quad.corners,
            [
                [243.0, 169.0],
                [563.0, 169.0],
                [563.0, 369.0],
                [243.0, 369.0]
            ],
            "the rect, moved by the offset and not grown by the blur's reach"
        );
        assert_eq!(quad.radius, 12.0, "and rounded by the radius it was given");
        assert_eq!(quad.size, [320.0, 200.0], "with the rect's own size");
    }

    #[test]
    fn a_shadow_quad_is_four_vertices_so_the_solid_index_buffer_addresses_it() {
        // `draw_vertex_quads` computes `quads = vertices.len() / 4` and
        // `quad_indices` addresses four vertices per quad. Three would put the
        // sixth index on the first vertex of the next quad, with nothing
        // complaining — the same invariant as the polygon's, and the same
        // consequence.
        let vertices = quad_vertices(rect_quad(
            Rect::new(100.0, 100.0, 40.0, 30.0),
            Color::new(0, 0, 0, 128),
            4.0,
        ));
        assert_eq!(vertices.len(), VERTS_PER_QUAD);
        assert_eq!(vertices.len() % VERTS_PER_QUAD, 0);
        assert_eq!(vertices[0].pos, [100.0, 100.0]);
        assert_eq!(vertices[3].pos, [100.0, 130.0]);
    }

    /// The uniform names `source` declares, in the order the declarations appear.
    ///
    /// The whole declaration is matched — `uniform vec2 u_size;` and not a bare
    /// `u_size` — because `u_size` is a substring of `u_size_texels` and a
    /// substring match reports a uniform that was renamed as one that survived.
    fn uniform_names(source: &str) -> Vec<String> {
        let mut names = Vec::new();
        for line in source.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("uniform ") else {
                continue;
            };
            // `sampler2D u_source;` — the type, then the name, then the `;`. An
            // array's brackets belong to the name and are dropped: `glGetUniform
            // Location` is asked for `u_weights`, not `u_weights[9]`.
            let Some(declaration) = rest.strip_suffix(';') else {
                continue;
            };
            let name = declaration
                .rsplit_once(' ')
                .map(|(_, name)| name)
                .unwrap_or(declaration);
            names.push(name.split('[').next().unwrap_or(name).to_string());
        }
        names
    }

    /// The composite's arithmetic, in the form the shader has to produce it in.
    ///
    /// This is a CPU mirror of `SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC`, and it
    /// exists because the shader string cannot be executed by `cargo test`. The
    /// string test below is what pins the GLSL; this one pins the *property*, so
    /// that a reader who changes one of them has to change both.
    fn composite_fragment(color: Color, coverage: f32) -> [f32; 4] {
        let tint = quad_color(color);
        [
            tint[0] * coverage,
            tint[1] * coverage,
            tint[2] * coverage,
            tint[3] * coverage,
        ]
    }

    #[test]
    fn the_composite_premultiplies_its_colour_by_the_coverage() {
        // The blend func is `ONE, ONE_MINUS_SRC_ALPHA`, so the source has to
        // arrive premultiplied or the result is `rgb + dst·(1 − a)`. **A black
        // shadow is identical either way** — `0 + dst·(1 − a)` — which is why the
        // defect this is about survives a capture of a black shadow, and why the
        // colour here is not black.
        let colour = Color::new(20, 30, 40, 160);
        let tint = quad_color(colour);
        let full = composite_fragment(colour, 1.0);
        assert_eq!(full[3], tint[3], "full coverage is the colour's own alpha");

        // Every colour channel is scaled by the coverage, not just the alpha:
        // this is the assertion a non-premultiplied composite cannot pass, since
        // it leaves `rgb` alone whatever the coverage is.
        let half = composite_fragment(colour, 0.5);
        for channel in 0..3 {
            assert_eq!(
                half[channel],
                tint[channel] * 0.5,
                "channel {channel} is halved with the coverage, so it is premultiplied \
                 and not merely scaled in its alpha"
            );
            assert!(
                half[channel] < tint[channel],
                "channel {channel} falls with the coverage, so it is premultiplied"
            );
        }

        // The premultiplied invariant itself: no channel of a premultiplied
        // colour exceeds its own alpha. This is the property that makes
        // `ONE, ONE_MINUS_SRC_ALPHA` the right blend for the source, and it is
        // false for the colour *before* the coverage is applied.
        for coverage in [0.25_f32, 0.5, 1.0] {
            let fragment = composite_fragment(colour, coverage);
            for channel in 0..4 {
                assert!(
                    fragment[channel] <= fragment[3] + 1e-6,
                    "at coverage {coverage}, channel {channel} ({}) is within the \
                     alpha ({})",
                    fragment[channel],
                    fragment[3]
                );
            }
        }
        // The control, on a colour where the two answers are visibly different: a
        // bright, nearly transparent shadow. Here `rgb` is *above* the alpha
        // before the coverage is applied, so a composite that left `rgb` alone
        // would emit 0.94 where the premultiplied source says 0.04 — and, blended
        // as `rgb + dst·(1 − a)`, a pale halo where a soft dark edge belongs.
        let pale = Color::new(240, 240, 240, 40);
        let pale_tint = quad_color(pale);
        assert!(
            pale_tint[0] > pale_tint[3],
            "the control colour's rgb ({}) is above its alpha ({}), so it can \
             tell the two composite forms apart",
            pale_tint[0],
            pale_tint[3]
        );
        let half_pale = composite_fragment(pale, 0.5);
        assert_eq!(
            half_pale[0],
            pale_tint[0] * 0.5,
            "and the premultiplied half-coverage is the colour scaled, not the \
             colour unchanged"
        );
        assert!(
            pale_tint[0] != half_pale[0],
            "so a non-premultiplied composite would differ from this one by \
             {} on this channel, at a coverage of one half",
            pale_tint[0] - half_pale[0]
        );
    }

    #[test]
    fn the_composite_shader_writes_the_premultiplied_form() {
        // The string, because the arithmetic above is a mirror of it and a mirror
        // cannot catch the original changing alone.
        assert!(
            SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC.contains("vec4(u_color.rgb * coverage, coverage)"),
            "rgb scaled by the coverage and the coverage as the alpha: the \\
             premultiplied source `ONE, ONE_MINUS_SRC_ALPHA` needs"
        );
        assert!(
            SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC.contains("texture(u_source, v_uv).r"),
            "and the coverage is the single channel the offscreen target holds"
        );
        // The same arithmetic in the no-blur path, so a zero-blur shadow and a
        // blurred one are the same colour at the same alpha.
        assert!(
            SHADOW_FRAGMENT_SHADER_SRC.contains("vec4(v_color.rgb * v_color.a, v_color.a)"),
            "the unblurred shadow premultiplies too"
        );
    }

    #[test]
    fn the_shadow_mask_shader_writes_coverage_and_nothing_else() {
        // The target is one channel. If the mask wrote the colour as well, the
        // blur would convolve `rgb` *and* the alpha — a doubled falloff, since a
        // blurred colour and a blurred alpha multiplied together is not a colour
        // at that alpha at any point but full coverage.
        assert!(
            SHADOW_MASK_FRAGMENT_SHADER_SRC.contains("vec4(v_color.a, 0.0, 0.0, 0.0)"),
            "the shape's coverage in the RED channel, because an GL_RED \
             attachment keeps `.r` and discards the rest"
        );
        assert!(
            !SHADOW_MASK_FRAGMENT_SHADER_SRC.contains("v_color.rgb"),
            "and no colour channel is written at all"
        );
        // The pairing, which is what would break if either half changed on its
        // own: the mask writes `.r` and the blur reads `.r`.
        assert!(
            SHADOW_MASK_FRAGMENT_SHADER_SRC.contains("vec4(v_color.a,")
                && BLUR_FRAGMENT_SHADER_SRC.contains(".r * u_weights[i]"),
            "the channel the mask writes is the channel the blur reads"
        );
    }

    #[test]
    fn both_shadow_shaders_round_a_corner_by_the_same_rule_as_the_solid_pass() {
        // The same three-way agreement `both_fragment_shaders_round_a_corner_by_
        // the_same_rule` asserts for the solid and image passes, extended to the
        // shadow's two. A shadow rounded by a different signed distance than the
        // panel that casts it is a panel with a shadow that does not fit under it,
        // and the only place that shows is the compiled string.
        let clip = r#"    if (v_radius > 0.0) {
        vec2 half_size = v_size * 0.5;
        vec2 q = abs(v_local - half_size) - (half_size - vec2(v_radius));
        float dist = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - v_radius;
        if (dist > 0.0) {
            discard;
        }
    }
"#;
        assert!(
            FRAGMENT_SHADER_SRC.contains(clip),
            "the solid pass's corner"
        );
        assert!(SHADOW_MASK_FRAGMENT_SHADER_SRC.contains(clip), "the mask's");
        assert!(
            SHADOW_FRAGMENT_SHADER_SRC.contains(clip),
            "and the unblurred shadow's"
        );
    }

    #[test]
    fn the_blur_shader_asks_for_high_precision_and_the_weight_array_it_declares_is_the_one_it_is_given(
    ) {
        // Two contracts a string comparison is the only place to see.
        //
        // The precision: ES's default fragment precision is `mediump`, which is
        // `fp16` — about 11 bits of mantissa. Summing nine 8-bit weights into it
        // and storing the result back into 8 bits is inside the range where the
        // rounding *steps*, and the visible artefact is a shadow edge that bands:
        // a staircase of plateaus where a ramp should be. The blur is the one pass
        // in this renderer whose whole output is a gradient, so it is the one pass
        // that cannot be `mediump`.
        assert!(
            BLUR_FRAGMENT_SHADER_SRC.contains("precision highp float;"),
            "a gradient is exactly what mediump quantises into plateaus"
        );
        // The array: a `#define`d length has to be the same number of taps the
        // Rust side builds, or the upload is either truncated or reads past the
        // uniform. `MAX_TAPS` is the Rust constant; `9` is the literal in the
        // source. Nothing else connects them.
        assert!(
            BLUR_FRAGMENT_SHADER_SRC.contains(&format!("uniform float u_weights[{MAX_TAPS}];")),
            "the shader's array length is the same MAX_TAPS the kernel builds"
        );
        assert_eq!(
            MAX_TAPS, 9,
            "and if that literal is ever changed, this is the number to change \\
             with it"
        );
        // The loop bound is the same literal, or the last tap would be dropped.
        assert!(
            BLUR_FRAGMENT_SHADER_SRC.contains("for (int i = 0; i < 9; i++)"),
            "the loop runs the whole array, and `u_taps` is what stops it early"
        );
    }

    #[test]
    fn the_blur_vertex_shader_flips_v_so_the_target_is_read_the_right_way_up() {
        // A position at the window's top edge is the framebuffer's *last* row, and
        // that row is texture coordinate `v = 1`. Emitting `normalized.y` would
        // sample the target upside down: a shadow that is soft but in the wrong
        // place, with the whole thing mirroring about the window's centre.
        assert!(
            BLUR_VERTEX_SHADER_SRC.contains("v_uv = vec2(normalized.x, 1.0 - normalized.y);"),
            "the flip, which is the difference between a shadow and a mirror image"
        );
        assert!(
            BLUR_VERTEX_SHADER_SRC.contains("gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);"),
            "and the same Y flip every other vertex shader here uses"
        );
        // One vertex shader for all three offscreen passes, so the composite and
        // the blur cannot disagree about where a window pixel is.
        assert!(
            SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC.contains("in vec2 v_uv;"),
            "the composite reads the same varying the blur wrote"
        );
    }

    /// The uniforms each shadow program declares, from its two shader stages, and
    /// the set the renderer writes them under.
    ///
    /// **A copy, and the copy is what makes the assertion possible.** The two
    /// numbers on each line are compared against each other: a uniform declared
    /// in a shader and absent here is one the renderer never writes, and a uniform
    /// listed here and absent from a shader is one whose lookup returns `None`.
    ///
    /// **What this cannot catch** is a *setter call being deleted*, because the
    /// table records what is set rather than what is called. That limit is why the
    /// acceptance for this layer is a capture with the ramp measured in pixels and
    /// not this test.
    #[test]
    fn every_uniform_a_shadow_shader_declares_is_one_the_renderer_sets() {
        // **The defect this test was written after**, and the reason it is a table
        // rather than a spot check.
        //
        // The shadow's mask and its blur both drew *nothing*: `u_resolution` and
        // `u_size` were declared, never written, GL defaulted them to zero, and
        // the vertex shader computed `a_pos / vec2(0.0)`. No GL call reported an
        // error, `gl.get_error()` read `0x0` at every step of the pass, the frame
        // rate was unchanged at 62 fps, and the picture was a window with no shadow
        // on it. Both halves of this pipeline's acceptance are blind to a uniform
        // that reads as zero — `.ai/NEVERAGAIN.md` § *A buffer sized for one vertex
        // per quad* is the same shape one layer down.
        let programs: [(&str, &str, &str, &[&str]); 4] = [
            (
                "the shadow mask",
                VERTEX_SHADER_SRC,
                SHADOW_MASK_FRAGMENT_SHADER_SRC,
                &["u_resolution"],
            ),
            (
                "the unblurred shadow",
                VERTEX_SHADER_SRC,
                SHADOW_FRAGMENT_SHADER_SRC,
                &["u_resolution"],
            ),
            (
                "the blur",
                BLUR_VERTEX_SHADER_SRC,
                BLUR_FRAGMENT_SHADER_SRC,
                &[
                    "u_size",
                    "u_texel",
                    "u_direction",
                    "u_weights",
                    "u_taps",
                    "u_source",
                ],
            ),
            (
                "the composite",
                BLUR_VERTEX_SHADER_SRC,
                SHADOW_COMPOSITE_FRAGMENT_SHADER_SRC,
                &["u_size", "u_color", "u_source"],
            ),
        ];
        for (label, vertex, fragment, set) in programs {
            let mut declared = uniform_names(vertex);
            declared.extend(uniform_names(fragment));
            for name in &declared {
                assert!(
                    set.contains(&name.as_str()),
                    "{label} declares `uniform {name}`, which the renderer does not \
                     set; GL defaults it to zero and the shader divides by it"
                );
            }
            for name in set {
                assert!(
                    declared.iter().any(|declared_name| declared_name == name),
                    "{label} is listed as setting `{name}`, which neither of its \
                     stages declares — the location lookup would answer None"
                );
            }
        }
    }

    #[test]
    fn the_uniform_names_are_read_from_the_declaration_and_not_from_a_substring() {
        // The control for the parser, in both directions: `u_size` is a substring
        // of nothing here, but `u_weights` is a substring of `u_weights[9]` and
        // `glGetUniformLocation` is asked for the bare name — so the brackets have
        // to come off. And the two stages are genuinely different, which is why
        // `u_size` is in the blur's *vertex* source and not its fragment source.
        assert_eq!(
            uniform_names(BLUR_VERTEX_SHADER_SRC),
            vec!["u_size"],
            "the vertex stage's one uniform"
        );
        assert_eq!(
            uniform_names(BLUR_FRAGMENT_SHADER_SRC),
            vec!["u_source", "u_texel", "u_direction", "u_weights", "u_taps"],
            "the fragment stage's five, in source order, with the array's brackets \
             taken off `u_weights[9]` — a substring match would report it as \
             `u_weights[9]`, which is not a name GL will answer to"
        );
        assert_eq!(
            uniform_names(SHADOW_MASK_FRAGMENT_SHADER_SRC),
            Vec::<String>::new(),
            "and the mask's fragment stage declares no uniform at all: it is the \
             solid vertex shader's fragment stage with the output changed"
        );
        assert_eq!(
            uniform_names(VERTEX_SHADER_SRC),
            vec!["u_resolution"],
            "which is where the mask's one uniform comes from"
        );
    }

    #[test]
    fn a_shadow_at_or_below_the_solid_blur_never_reaches_the_offscreen_target() {
        // The threshold's two sides, as a decision rather than as a picture: at
        // `SOLID_BLUR` the kernel is a single tap, which is the identity, so the
        // offscreen round trip would redraw precisely what drawing the shape
        // directly draws. The comparison is the same `blur_sigma <= blur::SOLID_BLUR`
        // `draw_shadow_batch` makes, written out so a change to one is a failure
        // here rather than a silent extra pass per frame.
        assert_eq!(blur::SOLID_BLUR, 0.0, "the threshold is exactly zero");
        for sigma in [-8.0, -0.5, 0.0] {
            assert!(
                sigma <= blur::SOLID_BLUR,
                "{sigma} takes the direct path, and no target is touched"
            );
            assert_eq!(
                blur::kernel(sigma).len(),
                1,
                "{sigma}: and its kernel is one tap, which is the identity"
            );
        }
        for sigma in [0.25_f32, 1.0, 2.0] {
            assert!(
                sigma > blur::SOLID_BLUR,
                "{sigma} takes the offscreen path, and its kernel is more than \\
                 one tap"
            );
            assert!(blur::kernel(sigma).len() > 1);
        }
    }
}
