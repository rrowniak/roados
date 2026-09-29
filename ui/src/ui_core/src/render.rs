//! Rendering.
//!
//! Owns the GL context and the frame lifecycle: the passes in order, the GPU
//! buffers they fill, and the submission that puts a frame on screen.

pub mod context;

use crate::arena::{Arena, Handle};
use crate::batch::{Batch, Batcher, ShaderKind};
use crate::node::WidgetNode;
use crate::paint::{DrawCommand, Rect};
use crate::property::Color;
use context::Context;
use glow::HasContext;

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
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::ShaderCompile(log) => write!(f, "shader compile error: {log}"),
            RenderError::ProgramLink(log) => write!(f, "program link error: {log}"),
            RenderError::Gl(msg) => write!(f, "GL error: {msg}"),
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

/// Expands a draw command into quads.
///
/// Text and image commands expand to nothing: their shaders arrive with the
/// Label (task 11) and Image (task 16) widgets.
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
        DrawCommand::Text { .. } | DrawCommand::Image { .. } => Vec::new(),
    }
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
        // SAFETY: The GL context is current on this thread.
        let (vao, vbo, ibo) = unsafe {
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
        renderer.ensure_index_capacity(INITIAL_CAPACITY)?;
        renderer.ensure_vertex_capacity(INITIAL_CAPACITY)?;
        Ok(renderer)
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
        let Some(state) = nodes.get_mut(handle).map(|node| node.paint_mut()) else {
            return;
        };
        if !state.is_dirty() {
            return;
        }
        let commands = state.take_commands();
        for command in commands {
            self.batcher.add(command);
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
    /// Opaque batches are drawn first with blending disabled, then
    /// transparent batches back-to-front with premultiplied-alpha blending.
    ///
    /// # Errors
    ///
    /// Returns an error if a buffer grows past what GL can address.
    pub fn end_frame(&mut self) -> Result<(), RenderError> {
        let batched = self.batcher.finish();
        // SAFETY: The GL context is current on this thread.
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
        for batch in &batched.opaque {
            self.draw_solid_batch(batch)?;
        }
        // SAFETY: The GL context is current on this thread.
        unsafe {
            let gl = self.context.gl();
            gl.enable(GL_BLEND);
            gl.blend_func(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
        }
        for batch in &batched.transparent {
            self.draw_solid_batch(batch)?;
        }
        self.context.swap();
        Ok(())
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
        if vertices.is_empty() {
            return Ok(());
        }
        let quads = vertices.len() / 4;
        self.ensure_index_capacity(quads)?;
        self.ensure_vertex_capacity(quads)?;
        // SAFETY: `Vertex` is `repr(C)` with 11 `f32` fields and no padding,
        // so the slice is a valid `VERTEX_STRIDE`-strided vertex array. The
        // slice borrows `vertices`, which outlives the upload.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr().cast::<u8>(),
                vertices.len() * std::mem::size_of::<Vertex>(),
            )
        };
        let count = i32::try_from(quads * 6)
            .map_err(|_| RenderError::Gl("index count exceeds the i32 range".to_string()))?;
        let gl = self.context.gl();
        // SAFETY: The GL context is current on this thread; `self.vao` is a
        // valid vertex array carrying the attribute pointers and the element
        // array binding, and `self.vbo` is a valid buffer bound for the
        // upload.
        unsafe {
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(GL_ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_sub_data_u8_slice(GL_ARRAY_BUFFER, 0, bytes);
            gl.draw_elements(GL_TRIANGLES, count, GL_UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);
        }
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
        let mut indices = Vec::with_capacity(capacity * 6);
        for quad in 0..capacity {
            let base = u32::try_from(quad)
                .map_err(|_| RenderError::Gl("quad count exceeds the u32 index range".to_string()))?
                .checked_mul(4)
                .ok_or_else(|| {
                    RenderError::Gl("quad count exceeds the u32 index range".to_string())
                })?;
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 3);
        }
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

    /// Grows the vertex buffer to hold `quads` quads' worth of vertices.
    fn ensure_vertex_capacity(&mut self, quads: usize) -> Result<(), RenderError> {
        if quads <= self.vertex_capacity {
            return Ok(());
        }
        let mut capacity = self.vertex_capacity.max(INITIAL_CAPACITY);
        while capacity < quads {
            capacity = capacity.saturating_mul(2);
        }
        let size = i32::try_from(capacity * std::mem::size_of::<Vertex>())
            .map_err(|_| RenderError::Gl("vertex buffer size exceeds the i32 range".to_string()))?;
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
    use crate::paint::TextureId;

    #[test]
    fn vertex_layout_matches_offsets() {
        assert_eq!(std::mem::size_of::<Vertex>(), VERTEX_STRIDE as usize);
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
    fn text_and_image_commands_expand_to_no_quads() {
        let text = command_quads(&DrawCommand::Text {
            x: 0.0,
            y: 0.0,
            text: "a".to_string(),
            color: Color::new(0, 0, 0, 255),
        });
        assert!(text.is_empty());

        let image = command_quads(&DrawCommand::Image {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            texture: TextureId::new(1),
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
}
