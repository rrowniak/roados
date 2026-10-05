//! Paint.
//!
//! Owns the cached paint state of a node, and the recording of the draw
//! commands a frame is made of.

pub use crate::font::{FamilyId, FontWeight};
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
    /// A blurred drop shadow of a rounded rectangle.
    ///
    /// The shape is drawn offscreen, blurred, and composited where it was
    /// recorded — [`crate::render::blur`] owns the kernel and
    /// [`crate::render::target`] the target. It is a command of its own rather
    /// than a flag on a [`RoundedRect`](DrawCommand::RoundedRect) because of
    /// **when** it draws: it is composited at the boundary between the batches
    /// recorded before it and the batches recorded after it
    /// ([`crate::batch::Segment`]), which is what lets an opaque panel sit on
    /// top of a translucent overlay with a shadow between them.
    ///
    /// `blur` is the Gaussian's **standard deviation in pixels**, and the kernel
    /// runs `min(ceil(2 * blur), (MAX_TAPS - 1) / 2)` taps either side of each
    /// sample, which holds 95.4% of the distribution's mass while the cap holds.
    /// A `blur` at or below [`SOLID_BLUR`](crate::render::blur::SOLID_BLUR)
    /// draws the shape directly, and touches no offscreen target at all.
    ///
    /// **The cap is not a detail.** With [`MAX_TAPS`](crate::render::blur::MAX_TAPS)
    /// at 9 it is four taps either side, so every `blur` above **2.0** produces
    /// the same nine-tap kernel and differs only in the weights its sigma gives
    /// it — a `blur` of 6.0 is four taps either side, not twelve. A blur asking
    /// for more is **capped rather than refused**, which is what
    /// [`taps_for`](crate::render::blur::taps_for) computes.
    ///
    /// `offset` moves the shadow relative to the rect it is the shadow **of**.
    /// It is a separate number rather than a pre-offset `rect` because the two
    /// are two decisions: a caller that knows where the panel goes does not have
    /// to know how far below it the shadow falls, and a theme that moves the
    /// shadow does not move the panel.
    ///
    /// **The shadow does not carry its own alpha into the blend.** The offscreen
    /// target holds one channel — the blurred coverage — and the composite
    /// shader premultiplies `color` by it before the pipeline's premultiplied
    /// blend func reads it. Every other primitive here relies on the blend func
    /// alone, which `doc/ui/IMPLEMENTATION_STATE.md` records as a defect in
    /// the solid pass; the shadow path does not inherit it, and the difference
    /// is deliberate.
    Shadow {
        /// The rectangle the shadow is the shadow of, before `offset`.
        rect: Rect,
        /// Corner radius in pixels, clamped to half the smaller side exactly as
        /// [`RoundedRect`](DrawCommand::RoundedRect)'s is.
        radius: f32,
        /// The shadow's colour as given. The composite multiplies it by the
        /// blurred coverage and by nothing else, so at full coverage this is
        /// the colour the shadow reaches and not one to be pre-multiplied.
        color: Color,
        /// The blur's standard deviation in pixels. Zero or less draws the
        /// shape directly, with no blur.
        blur: f32,
        /// The shadow's offset from `rect`, `(x, y)` in pixels. A light source
        /// above and to the left puts both components positive.
        offset: (f32, f32),
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
        /// Which family the run is drawn in.
        ///
        /// **The family is on the command and nowhere else**, for the same reason
        /// the weight is: a widget that wants a heading in one family should say
        /// so in one word rather than grow a parameter that every other call site
        /// would have to be given. A family is a *chain* of fonts — see
        /// [`FontId`](crate::font::FontId)'s family — and the renderer walks it per
        /// character: the face for this run's weight first, then the fallbacks,
        /// and the first font with a glyph for a character draws that character.
        ///
        /// A handle rather than a name because **the name is resolved once, where
        /// the family is defined** (`FontSet::family`), and this is a command
        /// recorded every frame: a `String` here would allocate per text command
        /// per frame to carry a word that never changes. The operator chose this
        /// over resolving the family in the `Painter` for the same reason the
        /// weight rode in the command rather than in a `Font` parameter.
        ///
        /// `FamilyId::default()` is the set's default family, so a run recorded by
        /// [`Painter::text`] — which every run that has not been given a family is
        /// — is drawn in the family every program gets without asking for one. An
        /// id the renderer's set does not hold resolves to its default family
        /// rather than to nothing, because a run that is not drawn is a hole in the
        /// layout.
        family: FamilyId,
        /// Which face the run is drawn with.
        ///
        /// **The weight is on the command and nowhere else**, so that a widget
        /// that wants a bold title says so in one word rather than growing a
        /// `Font` parameter that every other call site — every label, every
        /// button, every key of the on-screen keyboard — would then have to be
        /// given. The renderer resolves it against the chain the family names, and
        /// everything the run is drawn *with* comes from one face: its rasterized
        /// glyphs, their bearings and their advances. There is no synthetic
        /// weight, so a bold run carries a bold face's own ink at a bold face's
        /// own width.
        ///
        /// A chain with no face for the weight asked for falls back to the
        /// family's regular one, and a family that names no face of its own falls
        /// back to its first font, so this is a request and not a promise — see
        /// `ui_core::font`'s [`resolve_slot`](crate::font::resolve_slot) and
        /// [`Family`](crate::font::FontSet::pick), which is where that rule lives.
        weight: FontWeight,
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
    /// The edges are antialiased **by the framebuffer, not by this primitive**
    /// (superseded 2026-10-02, by `render::context`'s `MULTISAMPLE_SAMPLES`).
    /// This doc said the boundary was "the pixel grid, the same as a
    /// [`Rect`](DrawCommand::Rect)'s", which was true when the context asked for no
    /// multisample attribute at all; the default framebuffer is now **4x**
    /// multisampled, so every geometric edge recorded here — a polygon's boundary,
    /// a rect's corner, a line's side — is resolved by the hardware from four
    /// coverage samples. **What is still true** is the shape of the record: a
    /// polygon carries no radius and reaches none of the solid shader's own
    /// antialiasing branch, which is a hard `discard` on an *axis-aligned rounded
    /// rectangle*. **What 4x does not do is make the pipeline
    /// resolution-independent** — an edge landing on a pixel boundary resolves to
    /// full coverage on one side and none on the other and still reads hard, and
    /// two quads that share an edge may gain a hairline seam. Both are in
    /// `MULTISAMPLE_SAMPLES`'s own doc, with the measurement and the cost.
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
    ///
    /// Drawn in the renderer's regular face and its **default family** —
    /// [`Painter::text_bold`] is this method with one word changed, and
    /// [`Painter::text_in`] is this method with a family in front of it, and
    /// every argument means the same thing in all three.
    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        color: Color,
        font_size: f32,
        extra_advance: f32,
    ) {
        self.text_in_weight(
            FamilyId::default(),
            x,
            y,
            text,
            color,
            font_size,
            extra_advance,
            FontWeight::Regular,
        );
    }

    /// Records a text run in `family`: every argument as [`Painter::text`], one
    /// family different, in the renderer's regular weight.
    ///
    /// A family is a chain of fonts tried in order for each character, so this is
    /// the call a widget makes when its `font_family` property is not the default
    /// — a label whose property can change, say. [`Painter::text`] is this method
    /// with [`FamilyId::default`], which is why every other call site in the tree
    /// — every button, every toast, every key of the on-screen keyboard — is
    /// unaffected by families at all: a widget that never asks for one draws in
    /// the default family, exactly as it drew before there were any.
    ///
    /// The family is a handle, so the name a caller has was resolved once through
    /// `FontSet::family` rather than per command: a `String` on the command would
    /// allocate once per line per frame. `FamilyId::default()` here is the set's
    /// default family, and a handle the renderer's set does not hold resolves to
    /// its default family rather than to nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::font::FontSet;
    /// use ui_core::paint::{Color, DrawCommand, Painter};
    ///
    /// // A family is a handle the caller gets from the set that defines it, and
    /// // the handle is what the command carries — the name is resolved once.
    /// let mut fonts = FontSet::new();
    /// let heading = fonts.define_family("heading");
    ///
    /// // The same string in two families: two commands, two family handles, and
    /// // the renderer draws each in its own chain.
    /// let mut painter = Painter::new();
    /// painter.text_in(heading, 0.0, 0.0, "Settings", Color::new(255, 255, 255, 255), 20.0, 0.0);
    /// painter.text(0.0, 24.0, "Settings", Color::new(180, 180, 180, 255), 16.0, 0.0);
    ///
    /// let families: Vec<_> = painter
    ///     .finish()
    ///     .iter()
    ///     .filter_map(|command| match command {
    ///         DrawCommand::Text { family, .. } => Some(*family),
    ///         _ => None,
    ///     })
    ///     .collect();
    /// assert_eq!(families, vec![heading, fonts.default_family()]);
    /// assert_ne!(heading, fonts.default_family(), "and it is not the default");
    /// ```
    // The same eight arguments [`Painter::text`] takes, plus the family, for the
    // same reason the recorder below allows them: they are the text command's own
    // fields, and a struct to hold them would be a public API change to every
    // call site in the tree for a type whose only use is to be destructured again
    // on the other side of the function.
    #[allow(clippy::too_many_arguments)]
    pub fn text_in(
        &mut self,
        family: FamilyId,
        x: f32,
        y: f32,
        text: &str,
        color: Color,
        font_size: f32,
        extra_advance: f32,
    ) {
        self.text_in_weight(
            family,
            x,
            y,
            text,
            color,
            font_size,
            extra_advance,
            FontWeight::Regular,
        );
    }

    /// Records a text run in the renderer's **bold** face: every argument as
    /// [`Painter::text`], one weight different.
    ///
    /// A bold run is a *real* second face rather than the regular glyph drawn
    /// twice, so it carries the second face's own coverage, bearings and
    /// advances — and it is laid out with the **second face's** advance widths,
    /// which are its own and not the regular face's reused. How much wider that
    /// is in practice depends on the pair of faces, and for the pair this
    /// repository's demo loads (Lato Medium against Lato Bold) it is very little:
    /// measured through [`crate::font::Font::measure`] at 20 and 28 pixels, the
    /// bold run is 1.5% and 0.5% wider for `"Handgloves 42"` and **identical** at
    /// 28 pixels for `"Settings"`, because FreeType rounds each advance to a
    /// whole pixel at the size the face is set to. **So a caller cannot assume a
    /// bold run ends at a different `x` — and must not assume it ends at the same
    /// one either.** Measure the width in the weight being drawn; the
    /// `doc/ui/IMPLEMENTATION_STATE.md` font-size notes are where this
    /// repository measures text.
    ///
    /// On a renderer that was given no bold face this draws the run with the
    /// regular one, because a heading that is not bold is readable and a heading
    /// that is not *drawn* is a hole — see [`Painter::text`]'s note on
    /// `DrawCommand::Text`'s `weight`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::paint::{Color, DrawCommand, FontWeight, Painter};
    ///
    /// // A heading and its body: same string, same size, one word apart.
    /// let mut painter = Painter::new();
    /// painter.text_bold(120.0, 48.0, "Settings", Color::new(255, 255, 255, 255), 20.0, 0.0);
    /// painter.text(120.0, 80.0, "Settings", Color::new(180, 180, 180, 255), 20.0, 0.0);
    ///
    /// let commands = painter.finish();
    /// let weights: Vec<FontWeight> = commands
    ///     .iter()
    ///     .filter_map(|command| match command {
    ///         DrawCommand::Text { weight, .. } => Some(*weight),
    ///         _ => None,
    ///     })
    ///     .collect();
    /// assert_eq!(weights, vec![FontWeight::Bold, FontWeight::Regular]);
    /// ```
    pub fn text_bold(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        color: Color,
        font_size: f32,
        extra_advance: f32,
    ) {
        self.text_in_weight(
            FamilyId::default(),
            x,
            y,
            text,
            color,
            font_size,
            extra_advance,
            FontWeight::Bold,
        );
    }

    /// The one place a text command is recorded, so neither the weight nor the
    /// family can be a field some of the painters forget.
    ///
    /// Nine arguments is the command's eight fields plus the weight, and grouping
    /// them into a struct would be a public API change to every call site in the
    /// tree for nothing a caller could see — so the lint is answered here rather
    /// than by inventing a type whose only purpose is to be destructured again
    /// inside the function.
    #[allow(clippy::too_many_arguments)]
    fn text_in_weight(
        &mut self,
        family: FamilyId,
        x: f32,
        y: f32,
        text: &str,
        color: Color,
        font_size: f32,
        extra_advance: f32,
        weight: FontWeight,
    ) {
        self.commands.push(DrawCommand::Text {
            x,
            y,
            text: text.to_string(),
            color,
            font_size,
            extra_advance,
            family,
            weight,
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

    /// Records a blurred drop shadow of the rounded rectangle `rect`, offset by
    /// `offset` pixels and blurred with a standard deviation of `blur` pixels.
    ///
    /// Record it **before** the thing casting the shadow and after whatever the
    /// shadow falls on: the renderer composites it between the two, so a panel
    /// drawn after its shadow covers it and an overlay drawn before it does not.
    ///
    /// `blur` is a standard deviation rather than a reach because a reach has no
    /// single number: a kernel truncated at three sigma and one truncated at two
    /// put their last tap at different fractions of their peak, so two shadows
    /// with the same "blur radius" and different sharpness are not expressible.
    /// At or below [`SOLID_BLUR`](crate::render::blur::SOLID_BLUR) the shape is
    /// drawn with no blur at all and no offscreen target is touched.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::paint::{Color, DrawCommand, Painter, Rect};
    ///
    /// let panel = Rect::new(300.0, 180.0, 420.0, 260.0);
    ///
    /// // A card: the shadow first, then the panel that casts it.
    /// let mut painter = Painter::new();
    /// painter.shadow(panel, 12.0, Color::new(0, 0, 0, 128), 6.0, (0.0, 8.0));
    /// painter.rounded_rect(panel, 12.0, Color::new(245, 245, 245, 255));
    ///
    /// let commands = painter.finish();
    /// assert_eq!(commands.len(), 2);
    /// assert!(matches!(commands[0], DrawCommand::Shadow { .. }));
    ///
    /// // The blur is a standard deviation, and the kernel runs up to 2σ either
    /// // side — **capped** at `MAX_TAPS`, which with nine taps is four. So a
    /// // 6-pixel sigma is four taps either side and nine in all, not the twelve
    /// // and twenty-five an uncapped kernel would be, and not twenty-five
    /// // anything can draw. Asserted against the real function so this comment
    /// // and the pipeline cannot drift apart again.
    /// let DrawCommand::Shadow { blur, offset, .. } = commands[0] else {
    ///     panic!("the shadow is recorded first");
    /// };
    /// assert_eq!(blur, 6.0);
    /// assert_eq!(offset, (0.0, 8.0));
    /// assert_eq!(ui_core::render::blur::taps_for(blur), 4);
    /// assert_eq!(
    ///     2 * ui_core::render::blur::taps_for(blur) + 1,
    ///     ui_core::render::blur::MAX_TAPS,
    /// );
    /// ```
    pub fn shadow(&mut self, rect: Rect, radius: f32, color: Color, blur: f32, offset: (f32, f32)) {
        self.commands.push(DrawCommand::Shadow {
            rect,
            radius,
            color,
            blur,
            offset,
        });
    }

    /// Appends every command in `commands`, in order, to what this painter has
    /// already recorded.
    ///
    /// This is how a composite widget stitches a child widget's own output into
    /// its own paint — the [`Dialog`](crate::widgets::dialog::Dialog) draws its
    /// overlay, its shadow, its panel and its two blocks of text, then appends
    /// each [`Button`](crate::widgets::button::Button) in its action row, and the
    /// row has to land *after* the panel for the dialog's command order to mean
    /// anything. `extend(commands.finish())` is the same thing and allocates; this
    /// takes the `Vec` it would throw away.
    pub fn extend(&mut self, commands: Vec<DrawCommand>) {
        self.commands.extend(commands);
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
        painter.shadow(
            Rect::new(40.0, 40.0, 30.0, 20.0),
            4.0,
            Color::new(0, 0, 0, 128),
            5.0,
            (0.0, 6.0),
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
        assert_eq!(commands.len(), 9);
        assert!(matches!(&commands[0], DrawCommand::Rect { .. }));
        assert!(matches!(&commands[1], DrawCommand::RoundedRect { .. }));
        assert!(matches!(&commands[2], DrawCommand::Shadow { .. }));
        assert!(matches!(&commands[3], DrawCommand::Text { .. }));
        assert!(matches!(&commands[4], DrawCommand::Image { .. }));
        assert!(matches!(&commands[5], DrawCommand::Line { .. }));
        assert!(matches!(&commands[6], DrawCommand::Circle { .. }));
        assert!(matches!(&commands[7], DrawCommand::Path { .. }));
        assert!(matches!(&commands[8], DrawCommand::Polygon { .. }));
    }

    #[test]
    fn a_shadow_records_its_rect_its_radius_its_blur_and_its_offset() {
        // Destructure rather than match with `..`, for the reason the polygon
        // and image tests give: `command_quads` names every one of these fields,
        // so a rename here is a rename in the renderer.
        //
        // The fixture is off the origin on purpose. `.ai/NEVERAGAIN.md` § *A
        // rect's origin and a rect's extent are different numbers* — an origin
        // of `(0, 0)` cannot see an origin read as an extent, and the offset is
        // exactly the number that would be lost.
        let rect = Rect::new(240.0, 160.0, 420.0, 260.0);
        let mut painter = Painter::new();
        painter.shadow(rect, 12.0, Color::new(0, 0, 0, 128), 6.0, (3.0, 9.0));

        let commands = painter.finish();
        assert_eq!(commands.len(), 1);
        let DrawCommand::Shadow {
            rect: recorded_rect,
            radius,
            color,
            blur,
            offset,
        } = &commands[0]
        else {
            panic!("the painter recorded something that is not a shadow");
        };
        assert_eq!(*recorded_rect, rect);
        assert_eq!(*radius, 12.0);
        assert_eq!(*color, Color::new(0, 0, 0, 128));
        assert_eq!(*blur, 6.0);
        assert_eq!(
            *offset,
            (3.0, 9.0),
            "an offset is a pair of non-zero numbers, so this fixture can see one \
             read as the other, or as the rect's own origin"
        );
    }

    #[test]
    fn a_shadow_at_zero_blur_is_recorded_rather_than_dropped() {
        // The zero-blur shadow is a real shape — a hard-edged rounded rect drawn
        // straight to the screen — and the recorder's contract is to store what
        // it was handed. Whether the renderer blurs it is the renderer's
        // decision; a recorder that dropped it would make "no blur" and "no
        // shadow" the same value, and they are different pictures.
        let mut painter = Painter::new();
        painter.shadow(
            Rect::new(80.0, 60.0, 100.0, 40.0),
            4.0,
            Color::new(10, 20, 30, 255),
            0.0,
            (0.0, 0.0),
        );

        let commands = painter.finish();
        assert_eq!(commands.len(), 1, "a shadow with no blur is still a shadow");
        let DrawCommand::Shadow { blur, .. } = &commands[0] else {
            panic!("the painter recorded something that is not a shadow");
        };
        assert_eq!(*blur, 0.0);
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
    #[test]
    fn a_text_run_records_the_regular_weight_and_text_bold_the_bold_one() {
        // The one word that separates the two painters, and the whole of what a
        // widget has to say to get a bold title.
        //
        // Off the origin, as every geometry fixture here is: `.ai/NEVERAGAIN.md` §
        // *A rect's origin and a rect's extent are different numbers*, and the
        // positions below are part of what the two commands are compared on.
        let mut painter = Painter::new();
        painter.text(
            120.0,
            48.0,
            "Settings",
            Color::new(255, 255, 255, 255),
            20.0,
            0.0,
        );
        painter.text_bold(
            120.0,
            80.0,
            "Settings",
            Color::new(180, 180, 180, 255),
            20.0,
            0.0,
        );

        let commands = painter.finish();
        assert_eq!(commands.len(), 2);
        let DrawCommand::Text {
            x,
            y,
            text,
            font_size,
            extra_advance,
            weight,
            ..
        } = &commands[0]
        else {
            panic!("a regular run recorded as something that is not text");
        };
        assert_eq!(*x, 120.0);
        assert_eq!(*y, 48.0);
        assert_eq!(text, "Settings");
        assert_eq!(*font_size, 20.0);
        assert_eq!(*extra_advance, 0.0);
        assert_eq!(
            *weight,
            FontWeight::Regular,
            "`text` names the regular face, so a caller that has never heard of \
             weight records exactly the run it recorded before there was a second \
             face"
        );

        let DrawCommand::Text { weight, .. } = &commands[1] else {
            panic!("a bold run recorded as something that is not text");
        };
        assert_eq!(*weight, FontWeight::Bold, "and `text_bold` the bold one");
    }

    #[test]
    fn text_bold_differs_from_text_in_the_weight_and_in_nothing_else() {
        // The claim [`Painter::text_bold`]'s doc makes — that a bold title costs
        // the caller one word — is a claim about the *record*, and a record is
        // exactly what a test can compare. Field by field rather than with
        // `assert_eq!` on the two commands, because the weight is the one field
        // that is *meant* to differ and comparing the records whole would fail on
        // it rather than on everything else. If a second field ever rides along
        // with the weight, this is what catches it.
        let color = Color::new(255, 255, 255, 255);
        let mut painter = Painter::new();
        painter.text(200.0, 120.0, "Handgloves 42", color, 18.0, 0.5);
        painter.text_bold(200.0, 120.0, "Handgloves 42", color, 18.0, 0.5);

        let commands = painter.finish();
        assert_eq!(commands.len(), 2, "two runs of the same string");
        let DrawCommand::Text {
            x: regular_x,
            y: regular_y,
            text: regular_text,
            color: regular_color,
            font_size: regular_size,
            extra_advance: regular_tracking,
            family: regular_family,
            weight: regular,
        } = &commands[0]
        else {
            panic!("not text");
        };
        let DrawCommand::Text {
            x: bold_x,
            y: bold_y,
            text: bold_text,
            color: bold_color,
            font_size: bold_size,
            extra_advance: bold_tracking,
            family: bold_family,
            weight: bold,
        } = &commands[1]
        else {
            panic!("not text");
        };
        assert_eq!(
            regular_family, bold_family,
            "both runs name the default family, and only the weight differs"
        );
        assert_eq!(
            (
                regular_x,
                regular_y,
                regular_text,
                regular_color,
                regular_size,
                regular_tracking
            ),
            (
                bold_x,
                bold_y,
                bold_text,
                bold_color,
                bold_size,
                bold_tracking
            ),
            "same position, same string, same colour, same size, same tracking"
        );
        assert_eq!(*regular, FontWeight::Regular);
        assert_eq!(
            *bold,
            FontWeight::Bold,
            "and the two are not the same request"
        );
    }
}
