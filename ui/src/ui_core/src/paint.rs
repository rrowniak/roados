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

/// A handle to a texture the renderer has bound.
///
/// The cache that issues these, and the two places an image can end up — the
/// shared atlas or a texture of its own — are [`crate::texture`]'s business.
/// What a draw command needs is only the handle, so a widget can record an
/// image without borrowing the cache for the frame.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextureId(u32);

impl TextureId {
    /// Creates a texture handle from a raw texture index.
    #[must_use]
    pub fn new(id: u32) -> Self {
        TextureId(id)
    }

    /// Returns the raw texture index.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// The part of a texture a quad samples, in normalised texture coordinates.
///
/// `u` runs across the texture and `v` down it, both `0.0..=1.0`, with the
/// origin at the texture's top left: the same origin, the same four names and
/// the same order as [`Placement`](crate::texture::Placement), so a widget
/// holding a placement hands its four fields straight over.
///
/// A named struct rather than four loose `f32`s because a quad is two sets of
/// four numbers — `rect` and `uv` — and the shader reads both. A transposed
/// pair of coordinates is a vertically mirrored image, and with a bare tuple
/// the mix-up is invisible at the point it happens.
///
/// # Examples
///
/// ```
/// use ui_core::paint::UvRect;
///
/// let whole = UvRect::full();
/// assert_eq!((whole.u0, whole.v0, whole.u1, whole.v1), (0.0, 0.0, 1.0, 1.0));
/// assert!(whole.is_full());
///
/// // A window into a texture that holds more than one image.
/// let left_half = UvRect { u0: 0.0, v0: 0.0, u1: 0.5, v1: 1.0 };
/// assert!(!left_half.is_full(), "half a texture is not the texture");
/// assert_eq!(left_half.u1, 0.5, "and it is still the window it was given");
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvRect {
    /// Left edge, `0.0..=1.0` across the texture.
    pub u0: f32,
    /// Top edge, `0.0..=1.0` down the texture.
    pub v0: f32,
    /// Right edge, `0.0..=1.0` across the texture.
    pub u1: f32,
    /// Bottom edge, `0.0..=1.0` down the texture.
    pub v1: f32,
}

impl UvRect {
    /// Returns the whole texture: what a caller records when the texture holds
    /// one image and the whole of it is on screen.
    #[must_use]
    pub const fn full() -> Self {
        UvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        }
    }

    /// Returns whether this is the whole texture.
    ///
    /// An exact comparison against [`UvRect::full`] rather than a tolerance:
    /// the question is whether the caller *asked* for the whole texture, and a
    /// sub-rectangle that rounds to within a float of it is still a
    /// sub-rectangle. Being wrong in this direction costs a blended draw call;
    /// being wrong the other way costs a quad submitted with blending off,
    /// which is a wrong picture rather than a slow one.
    #[must_use]
    pub fn is_full(&self) -> bool {
        *self == UvRect::full()
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
    /// The quad is drawn with the image shader: `uv` says which part of the
    /// texture it samples, `opacity` how much of what it samples reaches the
    /// screen, and `radius` how many pixels of corner the shader clips away.
    /// Rendering needs the texture atlas, which arrives with the Image widget
    /// (task 16); the command is recorded and batched but not yet submitted to
    /// the GPU.
    Image {
        /// The rectangle to fill, in window coordinates.
        rect: Rect,
        /// The texture to sample.
        texture: TextureId,
        /// Which part of `texture` the rect samples.
        ///
        /// [`UvRect::full`] when the texture holds this image and no other; a
        /// window into it when the image shares the atlas with other images,
        /// which is every image small enough to be worth packing.
        uv: UvRect,
        /// How much of the image to draw, `0.0..=1.0`.
        ///
        /// A scalar rather than a [`Color`] because the colours are already in
        /// the texture: this scales the alpha the image brought with it, so
        /// `0.5` is *the image, half as present* and not *the image, tinted*.
        /// A [`Color`] would have to be folded into the sampled texel at every
        /// fragment to say the same thing.
        ///
        /// A value outside the range is clamped when the command is drawn, not
        /// when it is recorded — below `0.0` draws nothing, above `1.0` draws
        /// the image in full. The recorder stores what it was handed, so a
        /// caller whose opacity is a computed quantity can see the value it
        /// computed instead of a value this layer invented.
        opacity: f32,
        /// Corner radius in pixels, `0.0` for square corners.
        ///
        /// This is a **clip**, not a fill: the shader discards the fragments
        /// outside the rounded rect, so the corners show whatever the widget
        /// drew behind the image rather than being painted with a corner
        /// colour. A radius past half the shorter side is treated as half of it,
        /// as it is for [`DrawCommand::RoundedRect`], so a radius too large to
        /// fit is a rounded pill rather than an inverted shape.
        radius: f32,
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
    /// A filled polygon through `points`, in order around its edge.
    ///
    /// The shape [`Path`](DrawCommand::Path) cannot draw: a path *strokes* an
    /// outline of its own width, so a closed three-point path is a hollow
    /// triangle. The Gauge's needle is a filled triangle, and the operator
    /// decided on 2026-10-01 to add this primitive rather than settle for the
    /// outline.
    ///
    /// **Convex only.** The renderer fans the points from the first of them into
    /// `n - 2` triangles, which is exact for a convex polygon and is not a
    /// polygon rasteriser: given a **concave** polygon the fan also produces
    /// triangles that overlap the shape and triangles outside it, so the result
    /// is a wrong picture rather than a rough one. There is no ear-clipping pass
    /// and no stencil pass here, and a caller that cannot promise convexity must
    /// decompose the shape into convex pieces itself before recording it. That is
    /// the cost of reusing the quad pipeline, and it is what the Gauge needle —
    /// a triangle — is inside.
    ///
    /// Fewer than three points enclose no area and draw nothing, exactly as a
    /// [`Path`](DrawCommand::Path) of fewer than two points draws nothing. That
    /// is an ordinary recorded value rather than an error: the recorder stores
    /// what it was handed, and a caller whose point list came out empty has
    /// nothing to draw rather than a failure to report.
    ///
    /// The edges are as hard as the rasterizer makes them. The only
    /// antialiasing this pipeline has is the solid fragment shader's corner SDF,
    /// and a polygon is emitted with a radius of `0.0`, which takes the shader's
    /// plain-colour path instead — so the boundary is the pixel grid, the same as
    /// a [`Rect`](DrawCommand::Rect)'s. Anti-aliased polygon edges would need a
    /// distance field over the polygon's own edges, which is a different
    /// primitive with a different vertex type.
    Polygon {
        /// The vertices of the polygon, in order around its edge.
        points: Vec<(f32, f32)>,
        /// Fill color, premultiplied alpha.
        color: Color,
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

    /// Records a textured rectangle: `rect` in window coordinates, sampling
    /// `texture` through `uv`, drawn at `opacity`, with `radius` pixels of
    /// rounded corners clipped away.
    ///
    /// The one way to record an image, so the two things a textured quad has to
    /// say — which part of the texture, and how much of it — cannot be left to
    /// a default that silently draws the wrong picture.
    pub fn image(&mut self, rect: Rect, texture: TextureId, uv: UvRect, opacity: f32, radius: f32) {
        self.commands.push(DrawCommand::Image {
            rect,
            texture,
            uv,
            opacity,
            radius,
        });
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

    /// Records a filled polygon through `points`, in order around its edge.
    ///
    /// The counterpart to [`Painter::path`]: where a path strokes an outline of
    /// its `width`, this fills the shape its points describe, so a needle is a
    /// triangle here and a hollow one only if a caller asks for a path.
    ///
    /// `points` must describe a **convex** polygon, and fewer than three of them
    /// draw nothing — [`DrawCommand::Polygon`] says what each of those costs, and
    /// this method only records; it neither measures the points nor rejects them,
    /// because the recorder's contract is to store what it was handed.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::paint::{Color, Painter};
    ///
    /// // The three points of a needle pointing up, off the origin because a
    /// // geometry fixture at (0, 0) cannot see a coordinate being misread.
    /// let mut painter = Painter::new();
    /// painter.polygon(
    ///     &[(100.0, 20.0), (96.0, 70.0), (104.0, 70.0)],
    ///     Color::new(255, 255, 255, 255),
    /// );
    /// let commands = painter.finish();
    /// assert_eq!(commands.len(), 1);
    /// ```
    pub fn polygon(&mut self, points: &[(f32, f32)], color: Color) {
        self.commands.push(DrawCommand::Polygon {
            points: points.to_vec(),
            color,
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
        painter.image(
            Rect::new(4.0, 4.0, 8.0, 8.0),
            TextureId::new(7),
            UvRect::full(),
            1.0,
            2.0,
        );
        painter.line((0.0, 0.0), (10.0, 10.0), 2.0, Color::new(1, 2, 3, 255));
        painter.circle((5.0, 5.0), 3.0, Color::new(4, 5, 6, 255));
        painter.path(
            &[(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)],
            1.5,
            Color::new(7, 8, 9, 255),
            true,
        );
        painter.polygon(
            &[(0.0, 0.0), (1.0, 2.0), (2.0, 0.0)],
            Color::new(10, 11, 12, 255),
        );

        let commands = painter.finish();
        assert_eq!(commands.len(), 8);
        assert!(matches!(&commands[0], DrawCommand::Rect { .. }));
        assert!(matches!(&commands[1], DrawCommand::RoundedRect { .. }));
        assert!(matches!(&commands[2], DrawCommand::Text { .. }));
        assert!(matches!(&commands[3], DrawCommand::Image { .. }));
        assert!(matches!(&commands[4], DrawCommand::Line { .. }));
        assert!(matches!(&commands[5], DrawCommand::Circle { .. }));
        assert!(matches!(&commands[6], DrawCommand::Path { .. }));
        assert!(matches!(&commands[7], DrawCommand::Polygon { .. }));
    }

    #[test]
    fn a_polygon_records_its_points_and_its_color() {
        // Destructure rather than match with `..`: `command_quads` and
        // `command_bounds` both name these fields, so a rename here is a rename
        // in the renderer and in the clipper.
        let points = [(100.0, 20.0), (96.0, 70.0), (104.0, 70.0)];
        let mut painter = Painter::new();
        painter.polygon(&points, Color::new(3, 4, 5, 255));

        let commands = painter.finish();
        assert_eq!(commands.len(), 1);
        let DrawCommand::Polygon {
            points: recorded,
            color,
        } = &commands[0]
        else {
            panic!("the painter recorded something that is not a polygon");
        };
        assert_eq!(*recorded, points, "the points reach the command unmoved");
        assert_eq!(*color, Color::new(3, 4, 5, 255));
    }

    #[test]
    fn a_polygon_of_no_points_is_recorded_rather_than_refused() {
        // An empty point list draws nothing, and that is an ordinary value here:
        // the recorder stores what it was handed and reports nothing, because a
        // caller whose point list came out empty has nothing to draw, not a
        // failure to announce. Refusing it would mean an error type for an
        // absence, in a layer that has none.
        let mut painter = Painter::new();
        painter.polygon(&[], Color::new(0, 0, 0, 255));
        painter.polygon(&[(10.0, 10.0)], Color::new(0, 0, 0, 255));

        let commands = painter.finish();
        assert_eq!(commands.len(), 2, "two recorded, neither rejected");
        let DrawCommand::Polygon { points, .. } = &commands[0] else {
            panic!("the painter recorded something that is not a polygon");
        };
        assert!(points.is_empty());
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

    #[test]
    fn the_whole_texture_runs_from_zero_to_one() {
        let whole = UvRect::full();
        assert_eq!(whole.u0, 0.0, "the left edge of the texture is 0");
        assert_eq!(
            whole.v0, 0.0,
            "and so is the top: 0 is the top, not the bottom"
        );
        assert_eq!(whole.u1, 1.0, "the right edge is the whole width");
        assert_eq!(whole.v1, 1.0, "and the bottom edge the whole height");
        assert!(whole.is_full());
    }

    #[test]
    fn a_window_into_a_texture_is_not_the_whole_texture() {
        // An image packed into the atlas records its placement rather than the
        // whole texture, and the batcher tells the two apart by exactly this
        // question. The recording is in the path because that is where a widget
        // puts a placement.
        let window = UvRect {
            u0: 0.25,
            v0: 0.5,
            u1: 0.75,
            v1: 1.0,
        };
        let mut painter = Painter::new();
        painter.image(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            TextureId::new(1),
            window,
            1.0,
            0.0,
        );

        let commands = painter.finish();
        let DrawCommand::Image { uv, .. } = &commands[0] else {
            panic!("the painter recorded something that is not an image");
        };
        assert_eq!(*uv, window, "the window reaches the command unmoved");
        assert!(
            !uv.is_full(),
            "and a window into a texture is not the texture"
        );
    }

    #[test]
    fn is_full_reads_all_four_edges_exactly() {
        // One float below the whole texture is not the whole texture: the
        // question is what the caller asked for, and rounding would turn a
        // caller asking for a window into one asking for everything.
        let almost = UvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0 - f32::EPSILON,
            v1: 1.0,
        };
        assert!(!almost.is_full(), "a hair short of the whole is a window");

        // Each edge on its own: a window that is otherwise the whole texture is
        // still a window. An `is_full` reading three of the four edges lets one
        // of these through, and it would then be drawn in the opaque pass.
        let windows = [
            UvRect {
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 0.5,
            },
            UvRect {
                u0: 0.0,
                v0: 0.5,
                u1: 1.0,
                v1: 1.0,
            },
            UvRect {
                u0: 0.0,
                v0: 0.0,
                u1: 0.5,
                v1: 1.0,
            },
            UvRect {
                u0: 0.5,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            },
        ];
        for window in windows {
            assert!(!window.is_full(), "{window:?} is a window, not a texture");
        }
    }

    #[test]
    fn a_painter_records_the_uv_the_opacity_and_the_radius_it_was_given() {
        let rect = Rect::new(20.0, 30.0, 40.0, 50.0);
        let uv = UvRect {
            u0: 0.1,
            v0: 0.2,
            u1: 0.3,
            v1: 0.4,
        };
        let mut painter = Painter::new();
        painter.image(rect, TextureId::new(9), uv, 0.25, 6.0);

        let commands = painter.finish();
        assert_eq!(commands.len(), 1);
        // Destructure rather than match on the variant with `..`: the renderer's
        // quad builder names these fields, so a rename here is a rename there.
        let DrawCommand::Image {
            rect: recorded_rect,
            texture,
            uv: recorded_uv,
            opacity,
            radius,
        } = &commands[0]
        else {
            panic!("the painter recorded something that is not an image");
        };
        assert_eq!(*recorded_rect, rect);
        assert_eq!(*texture, TextureId::new(9));
        assert_eq!(*recorded_uv, uv);
        assert_eq!(*opacity, 0.25);
        assert_eq!(*radius, 6.0);
    }

    #[test]
    fn an_opacity_outside_the_range_is_recorded_as_it_was_given() {
        // The clamp belongs to the draw, not to the recorder: a caller that
        // computed the value keeps the number it computed, and the batcher sees
        // the same number the renderer will clamp.
        let mut painter = Painter::new();
        painter.image(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            TextureId::new(1),
            UvRect::full(),
            1.5,
            0.0,
        );
        painter.image(
            Rect::new(0.0, 0.0, 1.0, 1.0),
            TextureId::new(1),
            UvRect::full(),
            -0.5,
            0.0,
        );

        let commands = painter.finish();
        assert_eq!(commands.len(), 2);
        let DrawCommand::Image { opacity: over, .. } = &commands[0] else {
            panic!("not an image");
        };
        let DrawCommand::Image { opacity: under, .. } = &commands[1] else {
            panic!("not an image");
        };
        assert_eq!(*over, 1.5, "above the range survives recording");
        assert_eq!(*under, -0.5, "and so does below it");
    }
}
