//! Batching.
//!
//! Owns the grouping of recorded draw commands into GPU draw calls, by texture
//! atlas, blend mode and shader.

use crate::paint::{Color, DrawCommand, Rect, TextureId};

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
    /// The rect these commands are clipped to, or `None` for the whole window.
    ///
    /// **It is not part of [`BatchKey`].** A clip is not a property of the
    /// material — every batch inside a scrolling viewport shares one — so keying
    /// on it would split a single list into one batch per command and defeat the
    /// batching. It rides along on the batch instead, and the renderer sets the
    /// scissor when the clip *changes* between batches, which is once per
    /// viewport rather than once per command.
    pub clip: Option<Rect>,
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
        self.add_clipped(command, None);
    }

    /// Records a command clipped to `clip`, in window coordinates.
    ///
    /// Commands under different clips never share a batch, because the clip is
    /// GPU state the renderer has to set between draw calls: one batch is one
    /// draw call, and a draw call has one scissor.
    pub fn add_clipped(&mut self, command: DrawCommand, clip: Option<Rect>) {
        let key = command.batch_key();
        if let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.key == key && batch.clip == clip)
        {
            batch.commands.push(command);
        } else {
            self.batches.push(Batch {
                key,
                commands: vec![command],
                clip,
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
            | DrawCommand::Path { color, .. }
            | DrawCommand::Polygon { color, .. } => BatchKey {
                texture: None,
                blend_mode: BlendMode::from_color(*color),
                shader: ShaderKind::Solid,
            },
            DrawCommand::Text { color, .. } => BatchKey {
                texture: None,
                blend_mode: BlendMode::from_color(*color),
                shader: ShaderKind::Text,
            },
            // An image draws in the opaque pass only when it asks for no blending of its
            // own: the whole texture at full opacity. Anything else blends — a
            // reduced opacity, or a window into a texture, which is what every
            // image small enough to be worth packing into the atlas is, since
            // its source is its placement rather than the whole texture.
            //
            // Deliberately conservative. Such an image is very often opaque
            // too, but the command does not say so, and a command that *might*
            // show what is behind it must not be submitted with blending off.
            // The cost when the guess is wrong is one blended draw call; the
            // cost the other way is a wrong picture.
            DrawCommand::Image {
                texture,
                uv,
                opacity,
                ..
            } => BatchKey {
                texture: Some(*texture),
                // Exactly `1.0`, not "at least": an opacity that claims to be
                // more than the image has is clamped at draw time, so it draws
                // the same as `1.0` and is batched as blending all the same.
                blend_mode: if uv.is_full() && *opacity == 1.0 {
                    BlendMode::Opaque
                } else {
                    BlendMode::Transparent
                },
                shader: ShaderKind::Image,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{Rect, UvRect};

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

    /// An image of `texture` that asks for no blending of its own: the whole
    /// texture at full opacity.
    fn image(texture: u32) -> DrawCommand {
        DrawCommand::Image {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            texture: TextureId::new(texture),
            uv: UvRect::full(),
            opacity: 1.0,
            radius: 0.0,
        }
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
            font_size: 16.0,
            extra_advance: 0.0,
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
            font_size: 16.0,
            extra_advance: 0.0,
        });

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 2);
        assert_eq!(batched.opaque[0].key.shader, ShaderKind::Solid);
        assert_eq!(batched.opaque[1].key.shader, ShaderKind::Text);
    }

    #[test]
    fn images_batch_by_texture() {
        let mut batcher = Batcher::new();
        batcher.add(image(1));
        batcher.add(image(1));
        batcher.add(image(2));

        let batched = batcher.finish();
        // A whole texture at full opacity asks for no blending of its own, so
        // the two textures' batches land in the opaque group — in recording
        // order, so the two of texture 1 came first.
        assert_eq!(batched.opaque.len(), 2);
        assert_eq!(batched.opaque[0].key.texture, Some(TextureId::new(1)));
        assert_eq!(batched.opaque[0].commands.len(), 2);
        assert_eq!(batched.opaque[1].commands.len(), 1);
        assert!(batched.transparent.is_empty());
    }

    #[test]
    fn a_whole_texture_at_full_opacity_blends_as_opaque() {
        let mut batcher = Batcher::new();
        batcher.add(image(7));

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(batched.opaque[0].key.blend_mode, BlendMode::Opaque);
        assert_eq!(batched.opaque[0].key.texture, Some(TextureId::new(7)));
        assert_eq!(batched.opaque[0].key.shader, ShaderKind::Image);
        assert!(
            batched.transparent.is_empty(),
            "and it is not also in the group that blends"
        );
    }

    #[test]
    fn an_opacity_a_hair_below_one_blends() {
        // The boundary, on the side that matters: `1.0` is the only opacity that
        // means "draw no blending of my own", and the number just below it is a
        // different number.
        let mut almost = image(1);
        if let DrawCommand::Image { opacity, .. } = &mut almost {
            *opacity = 1.0 - f32::EPSILON;
        }

        let mut batcher = Batcher::new();
        batcher.add(almost);

        let batched = batcher.finish();
        assert!(batched.opaque.is_empty(), "not opaque, not even nearly");
        assert_eq!(batched.transparent.len(), 1);
        assert_eq!(
            batched.transparent[0].key.blend_mode,
            BlendMode::Transparent
        );
    }

    #[test]
    fn a_window_into_a_texture_blends_even_at_full_opacity() {
        // What every atlas-resident image records: its source is its placement
        // in the shared atlas, not the whole texture.
        let windowed = DrawCommand::Image {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            texture: TextureId::new(1),
            uv: UvRect {
                u0: 0.1,
                v0: 0.1,
                u1: 0.2,
                v1: 0.2,
            },
            opacity: 1.0,
            radius: 0.0,
        };

        let mut batcher = Batcher::new();
        batcher.add(windowed);

        let batched = batcher.finish();
        assert!(batched.opaque.is_empty());
        assert_eq!(batched.transparent.len(), 1);
    }

    #[test]
    fn an_opacity_claiming_more_than_the_image_has_blends() {
        // It draws as `1.0` after the draw-time clamp, so either blend mode
        // would be correct on screen; blending is the one that cannot be wrong
        // about what the command actually said.
        let mut over = image(1);
        if let DrawCommand::Image { opacity, .. } = &mut over {
            *opacity = 1.5;
        }

        let mut batcher = Batcher::new();
        batcher.add(over);

        let batched = batcher.finish();
        assert!(batched.opaque.is_empty());
        assert_eq!(batched.transparent.len(), 1);
    }

    #[test]
    fn the_source_rectangle_is_not_part_of_a_batch_key() {
        // `uv` rides in the vertex data, not in the key: two icons off the same
        // atlas texture that differ only in where they sit in it are one draw
        // call. A key that grew a `uv` would turn a screen of icons into a
        // screen of draw calls.
        let mut first = image(1);
        if let DrawCommand::Image { uv, .. } = &mut first {
            *uv = UvRect {
                u0: 0.0,
                v0: 0.0,
                u1: 0.1,
                v1: 0.1,
            };
        }
        let mut second = image(1);
        if let DrawCommand::Image { uv, .. } = &mut second {
            *uv = UvRect {
                u0: 0.9,
                v0: 0.9,
                u1: 1.0,
                v1: 1.0,
            };
        }

        let mut batcher = Batcher::new();
        batcher.add(first);
        batcher.add(second);

        let batched = batcher.finish();
        assert_eq!(batched.transparent.len(), 1, "one texture, one key");
        assert_eq!(batched.transparent[0].commands.len(), 2);
    }

    #[test]
    fn the_corner_radius_does_not_split_a_batch() {
        // A rounded image and a square one off the same texture are the same
        // shader with the same uniform range, so they are one draw call; the
        // radius goes in the vertex data like the source rectangle does.
        let mut rounded = image(1);
        if let DrawCommand::Image { radius, .. } = &mut rounded {
            *radius = 8.0;
        }

        let mut batcher = Batcher::new();
        batcher.add(rounded);
        batcher.add(image(1));

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(batched.opaque[0].commands.len(), 2);
    }

    #[test]
    fn commands_under_different_clips_do_not_share_a_batch() {
        // The clip rides on the batch rather than in the key, but two batches
        // with the same key and different clips still cannot merge: a draw call
        // has one scissor, so merging them would draw one viewport's rows with
        // the other viewport's rect applied.
        let mut batcher = Batcher::new();
        let a = Some(Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = Some(Rect::new(0.0, 100.0, 10.0, 10.0));
        batcher.add_clipped(rect(opaque()), a);
        batcher.add_clipped(rect(opaque()), a);
        batcher.add_clipped(rect(opaque()), b);
        batcher.add(rect(opaque()));

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 3, "two clips and an unclipped batch");
        assert_eq!(
            batched.opaque[0].commands.len(),
            2,
            "the shared clip merges"
        );
        assert_eq!(batched.opaque[0].clip, a);
        assert_eq!(batched.opaque[1].commands.len(), 1);
        assert_eq!(batched.opaque[1].clip, b);
        assert_eq!(batched.opaque[2].clip, None, "unclipped is its own batch");
    }

    #[test]
    fn adding_a_command_without_a_clip_is_adding_it_unclipped() {
        // `add` and `add_clipped(.., None)` are the same call, so a caller that
        // has no clip cannot accidentally get one.
        let mut batcher = Batcher::new();
        batcher.add(rect(opaque()));
        batcher.add_clipped(rect(opaque()), None);
        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(batched.opaque[0].commands.len(), 2);
        assert_eq!(batched.opaque[0].clip, None);
    }

    #[test]
    fn a_clipped_batch_keeps_its_commands_in_recording_order() {
        // The clip must not reorder anything: the pass composites back to front
        // and a batch that reordered under a scissor would composite wrong.
        let mut batcher = Batcher::new();
        let clip = Some(Rect::new(0.0, 0.0, 4.0, 4.0));
        for index in [0.0_f32, 1.0, 2.0] {
            batcher.add_clipped(
                DrawCommand::Rect {
                    rect: Rect::new(index, 0.0, 1.0, 1.0),
                    color: opaque(),
                },
                clip,
            );
        }
        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        let xs: Vec<f32> = batched.opaque[0]
            .commands
            .iter()
            .map(|c| match c {
                DrawCommand::Rect { rect, .. } => rect.x,
                _ => f32::NAN,
            })
            .collect();
        assert_eq!(xs, vec![0.0, 1.0, 2.0]);
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

    /// A filled polygon of `points`, in `color`.
    fn polygon(color: Color) -> DrawCommand {
        DrawCommand::Polygon {
            points: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            color,
        }
    }

    #[test]
    fn a_polygon_batches_with_the_other_filled_shapes() {
        // It is a solid colour with no texture, so it belongs in the same draw
        // call as a rect — one batch for the panel's background, its border and
        // the needle on top of it, rather than a batch per primitive.
        let mut batcher = Batcher::new();
        batcher.add(polygon(opaque()));
        batcher.add(rect(opaque()));

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1, "one draw call, not two");
        assert_eq!(batched.opaque[0].key.shader, ShaderKind::Solid);
        assert_eq!(batched.opaque[0].key.texture, None);
        assert_eq!(batched.opaque[0].commands.len(), 2);
    }

    #[test]
    fn a_translucent_polygon_blends_rather_than_covering() {
        // The needle at partial opacity, over a filled track: submitted to the
        // opaque pass it would hide the track instead of showing through it, and
        // the command's own alpha is the only thing that says so.
        let mut batcher = Batcher::new();
        batcher.add(polygon(transparent()));

        let batched = batcher.finish();
        assert!(batched.opaque.is_empty(), "not the opaque pass");
        assert_eq!(batched.transparent.len(), 1);
        assert_eq!(
            batched.transparent[0].key.blend_mode,
            BlendMode::Transparent,
            "a translucent polygon is not Opaque"
        );
        assert_eq!(batched.transparent[0].key.shader, ShaderKind::Solid);
    }

    #[test]
    fn a_polygon_of_fewer_than_three_points_still_batches_by_its_color() {
        // It draws nothing, but that is not the batcher's business: it groups
        // what it was given by key, and refusing or re-routing a command here
        // would be a second decision about emptiness in a layer that has none.
        let mut batcher = Batcher::new();
        batcher.add(DrawCommand::Polygon {
            points: Vec::new(),
            color: opaque(),
        });

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(batched.opaque[0].key.shader, ShaderKind::Solid);
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
