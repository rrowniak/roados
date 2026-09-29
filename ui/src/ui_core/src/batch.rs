//! Batching.
//!
//! Owns the grouping of recorded draw commands into GPU draw calls, by texture
//! atlas, blend mode and shader.

use crate::paint::{Color, DrawCommand, TextureId};

/// How a batch blends with what is already on screen.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BlendMode {
    /// No blending; the source overwrites the destination.
    Opaque,
    /// Premultiplied-alpha blending over the destination.
    Transparent,
}

impl BlendMode {
    /// Returns the blend mode for a command drawn with `color`.
    ///
    /// A fully opaque color blends as [`BlendMode::Opaque`]; anything else
    /// blends as [`BlendMode::Transparent`].
    #[must_use]
    pub fn from_color(color: Color) -> Self {
        if color.a == u8::MAX {
            BlendMode::Opaque
        } else {
            BlendMode::Transparent
        }
    }
}

/// The shader a batch is drawn with.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ShaderKind {
    /// Solid color with SDF-based corner rounding.
    Solid,
    /// Text rendering. The font atlas arrives with the Label widget (task 11).
    Text,
    /// Texture sampling. The texture atlas arrives with the Image widget
    /// (task 16).
    Image,
}

/// The key that groups draw commands into batches.
///
/// Commands with the same texture, blend mode and shader are submitted as one
/// GPU draw call.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BatchKey {
    /// The texture the batch samples, or `None` for untextured batches.
    pub texture: Option<TextureId>,
    /// How the batch blends with the destination.
    pub blend_mode: BlendMode,
    /// The shader the batch is drawn with.
    pub shader: ShaderKind,
}

/// A group of draw commands that share a [`BatchKey`] and are submitted as
/// one GPU draw call.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    /// The key this batch was grouped by.
    pub key: BatchKey,
    /// The commands to draw, in recording order.
    pub commands: Vec<DrawCommand>,
}

/// The result of batching one frame's worth of draw commands.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchedCommands {
    /// Batches drawn front-to-back with blending disabled.
    pub opaque: Vec<Batch>,
    /// Batches drawn back-to-front with premultiplied-alpha blending.
    pub transparent: Vec<Batch>,
}

/// Groups draw commands into batches by [`BatchKey`].
///
/// Batches keep recording order, so the opaque group is front-to-back; the
/// transparent group is reversed by [`Batcher::finish`] so it composites
/// back-to-front.
#[derive(Clone, Debug, Default)]
pub struct Batcher {
    batches: Vec<Batch>,
}

impl Batcher {
    /// Creates an empty batcher.
    #[must_use]
    pub fn new() -> Self {
        Batcher {
            batches: Vec::new(),
        }
    }

    /// Records a command, appending it to the batch with the matching key or
    /// starting a new batch when none exists yet.
    pub fn add(&mut self, command: DrawCommand) {
        let key = command.batch_key();
        if let Some(batch) = self.batches.iter_mut().find(|batch| batch.key == key) {
            batch.commands.push(command);
        } else {
            self.batches.push(Batch {
                key,
                commands: vec![command],
            });
        }
    }

    /// Clears all batches, leaving the batcher usable for the next frame.
    pub fn reset(&mut self) {
        self.batches.clear();
    }

    /// Returns `true` if no commands have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }

    /// Drains the batches into an opaque group in recording order and a
    /// transparent group reversed to back-to-front.
    pub fn finish(&mut self) -> BatchedCommands {
        let mut opaque = Vec::new();
        let mut transparent = Vec::new();
        for batch in self.batches.drain(..) {
            match batch.key.blend_mode {
                BlendMode::Opaque => opaque.push(batch),
                BlendMode::Transparent => transparent.push(batch),
            }
        }
        transparent.reverse();
        BatchedCommands {
            opaque,
            transparent,
        }
    }
}

impl DrawCommand {
    /// Returns the batch key for this command.
    pub(crate) fn batch_key(&self) -> BatchKey {
        match self {
            DrawCommand::Rect { color, .. }
            | DrawCommand::RoundedRect { color, .. }
            | DrawCommand::Line { color, .. }
            | DrawCommand::Circle { color, .. }
            | DrawCommand::Path { color, .. } => BatchKey {
                texture: None,
                blend_mode: BlendMode::from_color(*color),
                shader: ShaderKind::Solid,
            },
            DrawCommand::Text { color, .. } => BatchKey {
                texture: None,
                blend_mode: BlendMode::from_color(*color),
                shader: ShaderKind::Text,
            },
            // An image carries no opacity of its own, so it blends as
            // transparent until the atlas lands and can say otherwise.
            DrawCommand::Image { texture, .. } => BatchKey {
                texture: Some(*texture),
                blend_mode: BlendMode::Transparent,
                shader: ShaderKind::Image,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Rect;

    fn rect(color: Color) -> DrawCommand {
        DrawCommand::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            color,
        }
    }

    fn opaque() -> Color {
        Color::new(1, 2, 3, 255)
    }

    fn transparent() -> Color {
        Color::new(1, 2, 3, 128)
    }

    #[test]
    fn same_material_batches_together() {
        let mut batcher = Batcher::new();
        batcher.add(rect(opaque()));
        batcher.add(rect(opaque()));
        batcher.add(rect(opaque()));

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(batched.opaque[0].commands.len(), 3);
        assert!(batched.transparent.is_empty());
    }

    #[test]
    fn opaque_and_transparent_are_separated() {
        let mut batcher = Batcher::new();
        batcher.add(rect(opaque()));
        batcher.add(rect(transparent()));

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(batched.transparent.len(), 1);
        assert_eq!(batched.opaque[0].key.blend_mode, BlendMode::Opaque);
        assert_eq!(
            batched.transparent[0].key.blend_mode,
            BlendMode::Transparent
        );
    }

    #[test]
    fn transparent_batches_are_reversed_back_to_front() {
        let mut batcher = Batcher::new();
        batcher.add(rect(transparent()));
        batcher.add(DrawCommand::Text {
            x: 0.0,
            y: 0.0,
            text: "a".to_string(),
            color: transparent(),
        });
        batcher.add(rect(opaque()));

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(batched.transparent.len(), 2);
        // Recording order was [rect, text]; the transparent group is reversed.
        assert_eq!(batched.transparent[0].key.shader, ShaderKind::Text);
        assert_eq!(batched.transparent[1].key.shader, ShaderKind::Solid);
    }

    #[test]
    fn different_shaders_do_not_merge() {
        let mut batcher = Batcher::new();
        batcher.add(rect(opaque()));
        batcher.add(DrawCommand::Text {
            x: 0.0,
            y: 0.0,
            text: "a".to_string(),
            color: opaque(),
        });

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 2);
        assert_eq!(batched.opaque[0].key.shader, ShaderKind::Solid);
        assert_eq!(batched.opaque[1].key.shader, ShaderKind::Text);
    }

    #[test]
    fn images_batch_by_texture() {
        let mut batcher = Batcher::new();
        batcher.add(DrawCommand::Image {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            texture: TextureId::new(1),
        });
        batcher.add(DrawCommand::Image {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            texture: TextureId::new(1),
        });
        batcher.add(DrawCommand::Image {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            texture: TextureId::new(2),
        });

        let batched = batcher.finish();
        assert_eq!(batched.transparent.len(), 2);
        assert_eq!(batched.transparent[0].commands.len(), 1);
        assert_eq!(batched.transparent[1].commands.len(), 2);
    }

    #[test]
    fn reset_clears_batches() {
        let mut batcher = Batcher::new();
        batcher.add(rect(opaque()));
        assert!(!batcher.is_empty());

        batcher.reset();
        assert!(batcher.is_empty());

        let batched = batcher.finish();
        assert!(batched.opaque.is_empty());
        assert!(batched.transparent.is_empty());
    }

    #[test]
    fn blend_mode_from_color_boundary() {
        assert_eq!(
            BlendMode::from_color(Color::new(0, 0, 0, 255)),
            BlendMode::Opaque
        );
        assert_eq!(
            BlendMode::from_color(Color::new(0, 0, 0, 254)),
            BlendMode::Transparent
        );
        assert_eq!(
            BlendMode::from_color(Color::new(0, 0, 0, 0)),
            BlendMode::Transparent
        );
    }
}
