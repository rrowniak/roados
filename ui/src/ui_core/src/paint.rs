//! Paint.
//!
//! Owns the cached paint state of a node, and the recording of the draw
//! commands a frame is made of.

pub use crate::property::Color;

/// An axis-aligned rectangle in window coordinates, origin at the top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// X offset from the left edge of the window.
    pub x: f32,
    /// Y offset from the top edge of the window.
    pub y: f32,
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

impl Rect {
    /// Creates a new rectangle.
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

/// A handle to a texture in the atlas.
///
/// Texture atlas population is out of scope for this task; the handle exists
/// so image draw commands can be recorded and batched.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextureId(u32);

impl TextureId {
    /// Creates a texture handle from a raw atlas index.
    #[must_use]
    pub fn new(id: u32) -> Self {
        TextureId(id)
    }

    /// Returns the raw atlas index.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// A single draw command recorded during the paint pass.
///
/// Colors are premultiplied alpha, matching the rest of the pipeline.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    /// A filled rectangle.
    Rect {
        /// The rectangle to fill.
        rect: Rect,
        /// Fill color, premultiplied alpha.
        color: Color,
    },
    /// A filled rectangle with rounded corners.
    RoundedRect {
        /// The rectangle to fill.
        rect: Rect,
        /// Corner radius in pixels, clamped to half the smaller side.
        radius: f32,
        /// Fill color, premultiplied alpha.
        color: Color,
    },
    /// A text run.
    ///
    /// The run is drawn with the text shader, from the glyph atlas the
    /// renderer keeps. It is one *line*: a multi-line run is recorded as one
    /// command per line, each positioned on its own line box.
    Text {
        /// X offset of the line's left edge.
        x: f32,
        /// Y offset of the *top* of the line's box, not the baseline. The font
        /// decides where the baseline falls inside it, so a caller that has
        /// measured a line does not have to know the ascent.
        y: f32,
        /// The text to draw.
        text: String,
        /// Fill color, premultiplied alpha.
        color: Color,
        /// The font size in pixels, used to rasterize the glyphs.
        font_size: f32,
        /// Pixels added after every glyph, including the last: the layout's
        /// letter spacing. A run that is justified is recorded word by word
        /// instead, so its extra gap is carried by the word positions.
        extra_advance: f32,
    },
    /// A textured rectangle.
    ///
    /// Rendering needs the texture atlas, which arrives with the Image widget
    /// (task 16); the command is recorded and batched but not yet submitted to
    /// the GPU.
    Image {
        /// The rectangle to fill.
        rect: Rect,
        /// The texture to sample.
        texture: TextureId,
    },
    /// A thick line segment.
    Line {
        /// Segment start point.
        start: (f32, f32),
        /// Segment end point.
        end: (f32, f32),
        /// Stroke width in pixels.
        width: f32,
        /// Stroke color, premultiplied alpha.
        color: Color,
    },
    /// A filled circle.
    Circle {
        /// Circle center.
        center: (f32, f32),
        /// Circle radius in pixels.
        radius: f32,
        /// Fill color, premultiplied alpha.
        color: Color,
    },
    /// A thick polyline through `points`.
    Path {
        /// The vertices the polyline visits, in order.
        points: Vec<(f32, f32)>,
        /// Stroke width in pixels.
        width: f32,
        /// Stroke color, premultiplied alpha.
        color: Color,
        /// Whether a segment joins the last point back to the first.
        closed: bool,
    },
}

/// The cached paint state of one widget node.
///
/// The paint pass records draw commands here; the renderer drains them when
/// the node is dirty. `dirty` is set by whatever marks the node for repaint
/// and cleared by the renderer once it has recorded the commands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintState {
    commands: Vec<DrawCommand>,
    dirty: bool,
}

impl PaintState {
    /// Creates an empty paint state that is not dirty.
    #[must_use]
    pub fn new() -> Self {
        PaintState {
            commands: Vec::new(),
            dirty: false,
        }
    }

    /// Creates a paint state holding `commands`, marked dirty so the renderer
    /// records them on the next frame.
    #[must_use]
    pub fn from_commands(commands: Vec<DrawCommand>) -> Self {
        PaintState {
            commands,
            dirty: true,
        }
    }

    /// Marks the state dirty so the renderer records its commands.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns `true` if the state is dirty.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns the recorded commands without consuming them.
    #[must_use]
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Takes the recorded commands and clears the dirty flag.
    pub fn take_commands(&mut self) -> Vec<DrawCommand> {
        self.dirty = false;
        std::mem::take(&mut self.commands)
    }
}

/// Records draw commands for a widget subtree during the paint pass.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{Color, Painter, Rect};
///
/// let mut painter = Painter::new();
/// painter.rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::new(255, 0, 0, 255));
/// let commands = painter.finish();
/// assert_eq!(commands.len(), 1);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Painter {
    commands: Vec<DrawCommand>,
}

impl Painter {
    /// Creates a new painter with no commands recorded.
    #[must_use]
    pub fn new() -> Self {
        Painter {
            commands: Vec::new(),
        }
    }

    /// Records a filled rectangle.
    pub fn rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(DrawCommand::Rect { rect, color });
    }

    /// Records a filled rectangle with rounded corners.
    pub fn rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.commands.push(DrawCommand::RoundedRect {
            rect,
            radius,
            color,
        });
    }

    /// Records a text run at `font_size` pixels, on a line whose top edge is at
    /// `y`, with `extra_advance` pixels of tracking after each glyph.
    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        color: Color,
        font_size: f32,
        extra_advance: f32,
    ) {
        self.commands.push(DrawCommand::Text {
            x,
            y,
            text: text.to_string(),
            color,
            font_size,
            extra_advance,
        });
    }

    /// Records a textured rectangle.
    pub fn image(&mut self, rect: Rect, texture: TextureId) {
        self.commands.push(DrawCommand::Image { rect, texture });
    }

    /// Records a thick line segment.
    pub fn line(&mut self, start: (f32, f32), end: (f32, f32), width: f32, color: Color) {
        self.commands.push(DrawCommand::Line {
            start,
            end,
            width,
            color,
        });
    }

    /// Records a filled circle.
    pub fn circle(&mut self, center: (f32, f32), radius: f32, color: Color) {
        self.commands.push(DrawCommand::Circle {
            center,
            radius,
            color,
        });
    }

    /// Records a thick polyline through `points`.
    pub fn path(&mut self, points: &[(f32, f32)], width: f32, color: Color, closed: bool) {
        self.commands.push(DrawCommand::Path {
            points: points.to_vec(),
            width,
            color,
            closed,
        });
    }

    /// Returns the recorded commands, leaving the painter empty.
    #[must_use]
    pub fn finish(self) -> Vec<DrawCommand> {
        self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn painter_records_each_variant() {
        let mut painter = Painter::new();
        painter.rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::new(255, 0, 0, 255));
        painter.rounded_rect(
            Rect::new(1.0, 1.0, 10.0, 10.0),
            4.0,
            Color::new(0, 255, 0, 255),
        );
        painter.text(2.0, 3.0, "hi", Color::new(0, 0, 255, 255), 16.0, 0.0);
        painter.image(Rect::new(4.0, 4.0, 8.0, 8.0), TextureId::new(7));
        painter.line((0.0, 0.0), (10.0, 10.0), 2.0, Color::new(1, 2, 3, 255));
        painter.circle((5.0, 5.0), 3.0, Color::new(4, 5, 6, 255));
        painter.path(
            &[(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)],
            1.5,
            Color::new(7, 8, 9, 255),
            true,
        );

        let commands = painter.finish();
        assert_eq!(commands.len(), 7);
        assert!(matches!(&commands[0], DrawCommand::Rect { .. }));
        assert!(matches!(&commands[1], DrawCommand::RoundedRect { .. }));
        assert!(matches!(&commands[2], DrawCommand::Text { .. }));
        assert!(matches!(&commands[3], DrawCommand::Image { .. }));
        assert!(matches!(&commands[4], DrawCommand::Line { .. }));
        assert!(matches!(&commands[5], DrawCommand::Circle { .. }));
        assert!(matches!(&commands[6], DrawCommand::Path { .. }));
    }

    #[test]
    fn painter_finish_leaves_no_commands() {
        let mut painter = Painter::new();
        painter.rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::new(0, 0, 0, 255));
        let commands = painter.finish();
        assert_eq!(commands.len(), 1);
        // The painter was consumed; a fresh one starts empty.
        let painter = Painter::new();
        assert!(painter.finish().is_empty());
    }

    #[test]
    fn paint_state_dirty_lifecycle() {
        let mut state = PaintState::new();
        assert!(!state.is_dirty());
        assert!(state.commands().is_empty());

        state.mark_dirty();
        assert!(state.is_dirty());

        let commands = state.take_commands();
        assert!(commands.is_empty());
        assert!(!state.is_dirty());

        state.mark_dirty();
        assert!(state.is_dirty());
    }

    #[test]
    fn paint_state_from_commands_is_dirty() {
        let mut painter = Painter::new();
        painter.rect(Rect::new(0.0, 0.0, 5.0, 5.0), Color::new(9, 9, 9, 255));
        let mut state = PaintState::from_commands(painter.finish());

        assert!(state.is_dirty());
        assert_eq!(state.commands().len(), 1);

        let taken = state.take_commands();
        assert_eq!(taken.len(), 1);
        assert!(!state.is_dirty());
        assert!(state.commands().is_empty());
    }

    #[test]
    fn texture_id_roundtrip() {
        let id = TextureId::new(42);
        assert_eq!(id.get(), 42);
    }
}
