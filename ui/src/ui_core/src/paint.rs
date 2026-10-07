//! Paint.
//!
//! Owns the cached paint state of a node, and the recording of the draw
//! commands a frame is made of.

use crate::animation::Interpolate;
use crate::layout::Offset;
use crate::render::matrix::Mat4;
use crate::render::mesh::{MeshId, SubMeshRange};

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

    /// Returns the box both rectangles cover, edges included.
    ///
    /// **Total, and an empty box rather than a `None`.** Two boxes that do not
    /// overlap have an overlap of no area, and saying so with a zero-width or
    /// zero-height rect is what a scissor already understands: [`GL_SCISSOR`]
    /// with a zero extent draws nothing, which is the right answer for a command
    /// clipped out of its own viewport. An `Option` here would have to invent a
    /// second answer for "clipped away" beside `None`'s own, which is *no
    /// clip* — and a caller that read a vanished clip as an unbounded one would
    /// draw the very thing the clip was there to cut.
    ///
    /// **A second intersection in the tree, beside
    /// `layout::intersect`, and `TASK_UI_PRIM_45.md` requirement 1 owns the
    /// reconciliation.** That one is private and takes two `Option<layout::Rect>`s
    /// to answer "what does a node inherit from its ancestors", where an unset
    /// clip means *no ancestor clips it*; this one is total, over one
    /// `paint::Rect` — the command's own box against the batch's — where an
    /// absent clip means *the whole window*. The two differ in their empty answer
    /// as well as in their type, and this paragraph records which is which rather
    /// than merging them: a merge is requirement 1's decision to make, with both
    /// call sites in front of it, and not something a task that needed an
    /// intersection today should decide by copying a number across.
    ///
    /// `# Examples`
    ///
    /// ```
    /// use ui_core::paint::Rect;
    ///
    /// let outer = Rect::new(0.0, 0.0, 100.0, 50.0);
    /// let inner = Rect::new(80.0, 10.0, 40.0, 80.0);
    /// assert_eq!(outer.intersection(inner), Rect::new(80.0, 10.0, 20.0, 40.0));
    ///
    /// // Boxes that miss each other overlap by nothing, which is a box of no
    /// // area and not the whole window.
    /// let missed = Rect::new(200.0, 0.0, 10.0, 10.0);
    /// assert_eq!(outer.intersection(missed).width, 0.0);
    /// assert!(outer.intersection(missed).width < outer.width);
    /// ```
    ///
    /// [`GL_SCISSOR`]: crate::render::Renderer::set_scissor
    #[must_use]
    pub fn intersection(self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        Rect::new(x, y, (right - x).max(0.0), (bottom - y).max(0.0))
    }

    /// Returns the rectangle moved by `by`.
    #[must_use]
    pub fn translated(self, by: Offset) -> Rect {
        Rect::new(self.x + by.x, self.y + by.y, self.width, self.height)
    }

    /// Returns whether the rectangle covers no area, and so is a clip that
    /// draws nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// The window a text run's colour ramps out over, in window coordinates.
///
/// **The ramp is a per-corner vertex alpha and nothing else.** Every one of a
/// glyph quad's four corners is scaled by its *own* x against this window, and
/// the text shader interpolates `v_color` across the quad, so the ramp is smooth
/// inside a glyph and not stepped at glyph boundaries.
///
/// # Why per corner, and not the two alternatives
///
/// The operator chose this on 2026-10-06, and the reasoning is worth keeping
/// because the two rejected answers look cheaper:
///
/// - **Per glyph**, one alpha per glyph quad: the ramp quantises to glyph
///   boundaries, so a 48-pixel fade across four 12-pixel glyphs is four flat
///   steps — which is not a fade. It needs no renderer change either, and that
///   is the whole of its case.
/// - **A shader term**: smooth, but a uniform is per draw call, and one text
///   batch holds every run in the frame. A per-run ramp would then cost one draw
///   call per ramped run — the opposite of what batching is for — unless the
///   window became a per-vertex attribute, which is a stride change to
///   `TextVertex` and a new attribute in every text vertex layout.
///
/// So the window rides the command, the corners carry the scale, and
/// `TextVertex` stays 32 bytes with no shader change and no batching change.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{faded_color, ramp_factor, Color, FadeRamp};
///
/// let ramp = FadeRamp::new(240.0, 288.0);
/// assert_eq!(ramp_factor(ramp, 240.0), 1.0, "untouched at the start");
/// assert_eq!(ramp_factor(ramp, 264.0), 0.5, "halfway is half");
/// assert_eq!(ramp_factor(ramp, 288.0), 0.0, "gone at the end");
///
/// // Every channel moves: the colour is premultiplied, so a fade that scaled
/// // only the alpha would leave the text at full brightness over the background.
/// let colour = Color::new(200, 180, 160, 255);
/// let faded = faded_color(ramp, 264.0, colour);
/// assert_eq!((faded.r, faded.g, faded.b, faded.a), (100, 90, 80, 128));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeRamp {
    /// Where the ramp begins: at and to the left of this, the run is drawn at
    /// the colour it was recorded with.
    pub start_x: f32,
    /// Where the ramp ends: at and to the right of this, the run draws nothing.
    pub end_x: f32,
}

impl FadeRamp {
    /// A ramp that leaves the colour alone at and before `start_x` and has
    /// reached nothing at and after `end_x`.
    #[must_use]
    pub const fn new(start_x: f32, end_x: f32) -> Self {
        FadeRamp { start_x, end_x }
    }

    /// The ramp a truncation fade runs over: the last `width` pixels of a drawn
    /// run that ends at `run_end` and starts at `left`.
    ///
    /// **`left` is a floor on where the ramp begins, and that is what keeps a
    /// short line from being dimmed at its start.** A run narrower than the ramp
    /// would otherwise begin part-way down it and be drawn at a uniform partial
    /// opacity — a line that looks like a wrong colour rather than a fade. With
    /// the floor, the whole run ramps, which is the same claim as a longer one:
    /// this text is cut here.
    #[must_use]
    pub fn to_run_end(left: f32, run_end: f32, width: f32) -> Self {
        FadeRamp::new((run_end - width).max(left), run_end)
    }

    /// Returns the window moved by `by`.
    ///
    /// **Both edges, and both are positions.** A ramp is a place on the screen
    /// rather than a property of the text, which is what
    /// [`list::translate_commands`](crate::widgets::list::translate_commands)
    /// needs it to be — see the `Text` row of that function's own table.
    #[must_use]
    pub fn translated(self, by: Offset) -> Self {
        FadeRamp::new(self.start_x + by.x, self.end_x + by.x)
    }
}

/// Returns how much of a text run's own colour reaches the screen at `at_x`.
///
/// **Free, and over nothing but numbers**, for the reason
/// `crate::render`'s `walk_run` is: `AGENTS.md` forbids a unit test to open a GL
/// context, so a rule that can only be exercised through the renderer is a rule
/// with a capture as its only evidence.
///
/// Monotonic non-increasing by construction, which is the whole of the
/// requirement: `1.0` at and before `start_x`, `0.0` at and after `end_x`, and a
/// straight line between. A window of no width reaches both ends at the same `x`
/// and therefore reads as `1.0` there rather than dividing by zero — the run is
/// drawn whole, which is the answer for a ramp with nothing to ramp over.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{ramp_factor, FadeRamp};
///
/// let ramp = FadeRamp::new(100.0, 200.0);
/// assert_eq!(ramp_factor(ramp, 0.0), 1.0);
/// assert_eq!(ramp_factor(ramp, 100.0), 1.0);
/// assert_eq!(ramp_factor(ramp, 150.0), 0.5);
/// assert_eq!(ramp_factor(ramp, 200.0), 0.0);
/// assert_eq!(ramp_factor(ramp, 9999.0), 0.0);
/// ```
#[must_use]
pub fn ramp_factor(ramp: FadeRamp, at_x: f32) -> f32 {
    if at_x <= ramp.start_x {
        1.0
    } else if at_x >= ramp.end_x {
        0.0
    } else {
        1.0 - (at_x - ramp.start_x) / (ramp.end_x - ramp.start_x)
    }
}

/// Returns `color` as `ramp_factor(ramp, at_x)` of itself.
///
/// **All four channels, and that is not a detail.** A [`Color`] is premultiplied
/// by construction — every colour in this pipeline is — so scaling only the
/// alpha produces the classic non-premultiplied fade: a run that keeps its full
/// brightness and lets the background through it, which reads as text drawn in
/// the wrong colour rather than as text fading out. Interpolating toward
/// transparent black *is* the premultiplied fade, which is why this is the one
/// line [`button::with_opacity`](crate::widgets::button) writes and this is a
/// call to the same arithmetic rather than a fourth copy of it.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{faded_color, Color, FadeRamp};
///
/// let ramp = FadeRamp::new(0.0, 10.0);
/// assert_eq!(faded_color(ramp, 0.0, Color::new(9, 9, 9, 9)), Color::new(9, 9, 9, 9));
/// assert_eq!(faded_color(ramp, 10.0, Color::new(9, 9, 9, 9)), Color::new(0, 0, 0, 0));
/// ```
#[must_use]
pub fn faded_color(ramp: FadeRamp, at_x: f32, color: Color) -> Color {
    Color::interpolate(
        &color,
        &Color::new(0, 0, 0, 0),
        1.0 - ramp_factor(ramp, at_x),
    )
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
        /// Where this run's colour ramps out, in window coordinates, or `None`
        /// for a run drawn at one colour throughout.
        ///
        /// **A window and not a colour per glyph**, because the ramp is applied
        /// per *corner* at draw time and the rasteriser interpolates between
        /// them — see [`FadeRamp`], which is where the choice and the two
        /// rejected alternatives are recorded.
        ///
        /// The command carries the window rather than the alphas because the
        /// alphas are four evaluations of it, one per corner of every glyph quad,
        /// and a run of thirty glyphs would then carry a hundred and twenty
        /// numbers where two say the same thing. It also means the fade is
        /// arithmetic a unit test can reach: `AGENTS.md` forbids a test to open a
        /// GL context, so a rule that only the renderer could evaluate would have
        /// a screenshot as its only evidence.
        ///
        /// **In window coordinates, and so it moves with the run** — see
        /// [`list::translate_commands`](crate::widgets::list::translate_commands),
        /// which is the one function that moves a recorded command.
        fade: Option<FadeRamp>,
        /// The box this run is clipped to, in window coordinates, or `None` for
        /// the whole window.
        ///
        /// **The cut is made by the scissor, not by this field** — see
        /// [`Batch::clip`](crate::batch::Batch::clip): the batcher intersects it
        /// with the clip its own caller asked for, splits the batch when the
        /// effective clip changes, and `Renderer::apply_clip` sets
        /// `glScissor` between draw calls. A `DrawCommand` with no scissor of
        /// its own is what deferred per-node clipping for as long as it did.
        ///
        /// **A glyph that overhangs the box is cut by the GPU**, which is the one
        /// thing here that no assertion on a recorded command can see: the
        /// command records the box, and whether a quad crosses its right edge is
        /// a question about the font's own bearings. The layout's cut is a cut of
        /// whole characters ([`label::fit`](crate::widgets::label::layout_text)),
        /// so the two are different cuts of different things and this is the one
        /// that catches a glyph's last pixel column. **Measured**, on Lato Medium
        /// at 24 px: `A`, `f` and `v` overhang their advance by exactly 1.0 px and
        /// nothing else in the demo's sentence does, so a cut that lands on one of
        /// them is a glyph the scissor removes and the layout could not have.
        ///
        /// **A clipped text run still travels whole through a scroll's CPU-side
        /// clipping, and that is unchanged.** [`command_bounds`](crate::widgets::scroll::command_bounds)
        /// answers `None` for a `Text` command — it carries no width, which is the
        /// reason it answers `None` for *every* `Text` command — so
        /// [`clip_commands`](crate::widgets::scroll::clip_commands) keeps it
        /// whatever the viewport is. This box does not change that, and it is not
        /// the place it could: `clip_commands` has to decide from a *bounds*, and
        /// a bounds for a run of text is a width this command does not carry and
        /// did not before. The consequence to state rather than fix is that a
        /// clipped run inside a scrolling viewport is clipped by the **scissor**
        /// and not by the command list, so it is drawn and cut by the GPU
        /// instead of being dropped on the CPU — which is the whole of the
        /// `A draw-command assertion cannot see where a command *lands*` entry in
        /// `.ai/NEVERAGAIN.md`, unchanged by this field.
        ///
        /// **In window coordinates, and so it moves with the run.**
        clip: Option<Rect>,
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
    /// A triangle mesh: one named sub-mesh of one uploaded mesh, through one
    /// transform, in one tint.
    ///
    /// **The transform is on the command and not on the mesh**, because the five
    /// sub-meshes of one car share one buffer and differ only in the transform
    /// they are drawn through — which is why task 35 made them index ranges into
    /// one interleaved pair rather than five buffer pairs.
    ///
    /// **Where it lands in the frame is the caller's recording order, and there is
    /// no flag that changes it.** A command recorded before this one is submitted
    /// first and is under it; a command recorded after it is submitted later and is
    /// over it, whatever this mesh's depth says — the 2D passes neither test nor
    /// write depth, so the depth buffer cannot express "the map is behind the car"
    /// and no comparison function on it ever will. Record the map first and the
    /// chrome second. A mesh recorded *before* the map is under the map: that is
    /// correct for submission order, not a defect.
    Mesh {
        /// The mesh to draw from, as returned by `Renderer::upload_mesh`.
        mesh: MeshId,
        /// Which part of that mesh: one sub-mesh's `(first_index, index_count)`
        /// range, with **absolute** indices into the shared index buffer.
        range: SubMeshRange,
        /// Model-view-projection, from the renderer's transform path.
        mvp: Mat4,
        /// Material tint, premultiplied, like every other colour on this enum.
        tint: Color,
        /// How much of the mesh reaches the screen, `0.0..=1.0`, clamped when
        /// drawn.
        ///
        /// **A scalar and not part of `tint`,** because it decides the blend mode
        /// and the blend mode must not depend on how the caller chose to express
        /// the alpha: a tint of alpha 0 draws nothing and is transparent, while
        /// `opacity: 1.0` says the draw is opaque whatever the tint. This is the
        /// same split `DrawCommand::Image`'s `opacity` makes against the texture.
        opacity: f32,
        /// The material's colormap, sampled by the mesh pass.
        ///
        /// **On the command rather than on the mesh**, because `BatchKey`'s
        /// `texture` is what the pass reads the sampler from — the same field
        /// `draw_image_batch` reads — and because one model may be drawn with a
        /// different colormap without re-uploading its geometry.
        texture: TextureId,
    },
}

/// Everything [`Painter::text_run`] records, in one named struct.
///
/// **The command's own fields, gathered.** [`Painter::text_in`] takes seven of
/// them positionally and `text_in_weight` eight, and `too_many_arguments` is on
/// with `-D warnings`, so the two extra things a truncation needs — where the run
/// ramps out and what box it is clipped to — cannot be two more parameters on
/// either. They *could* have been two parameters on a third sibling and the lint
/// would still be right to complain, which is why the whole run arrives as one
/// value instead.
///
/// The alternative the existing code argues for — "a struct to hold them would be
/// a public API change to every call site in the tree for a type whose only use
/// is to be destructured again on the other side of the function" — is answered
/// here by **not touching those call sites**: `text_in` and `text_in_weight` keep
/// their signatures and build a `TextRun` with `fade` and `clip` of their own.
///
/// A widget that has nothing to say about the ramp or the clip — every one of
/// them but a truncating [`Label`](crate::widgets::label::Label) — records
/// through [`Painter::text_in`] and never names this type.
///
/// # Examples
///
/// ```
/// use ui_core::paint::{Color, DrawCommand, FadeRamp, Painter, Rect, TextRun};
///
/// let mut painter = Painter::new();
/// painter.text_run(TextRun {
///     x: 60.0,
///     y: 170.0,
///     text: "a line cut here",
///     color: Color::new(220, 220, 220, 255),
///     font_size: 24.0,
///     extra_advance: 0.0,
///     fade: Some(FadeRamp::new(120.0, 168.0)),
///     clip: Some(Rect::new(60.0, 170.0, 108.0, 29.0)),
///     ..TextRun::default()
/// });
///
/// let commands = painter.finish();
/// assert_eq!(commands.len(), 1);
/// let DrawCommand::Text { fade, clip, .. } = &commands[0] else {
///     panic!("a text run records a text command");
/// };
/// assert_eq!(fade, &Some(FadeRamp::new(120.0, 168.0)));
/// assert_eq!(clip, &Some(Rect::new(60.0, 170.0, 108.0, 29.0)));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextRun<'a> {
    /// X offset of the line's left edge.
    pub x: f32,
    /// Y offset of the *top* of the line's box, not the baseline.
    pub y: f32,
    /// The text to draw.
    pub text: &'a str,
    /// Fill color, premultiplied alpha, before any ramp.
    pub color: Color,
    /// The font size in pixels, used to rasterize the glyphs.
    pub font_size: f32,
    /// Pixels added after every glyph, including the last.
    pub extra_advance: f32,
    /// Which family the run is drawn in. The default family.
    pub family: FamilyId,
    /// Which face the run is drawn with. The regular one.
    pub weight: FontWeight,
    /// Where the run's colour ramps out, or `None` for one flat colour.
    pub fade: Option<FadeRamp>,
    /// The box the run is clipped to, or `None` for the whole window.
    pub clip: Option<Rect>,
}

impl<'a> TextRun<'a> {
    /// Returns the run's window positions moved by `by`, everything else
    /// unchanged.
    ///
    /// **The ramp and the clip move and the text does not**, which is the same
    /// split [`list::translate_commands`](crate::widgets::list::translate_commands)
    /// records for every field of every variant: a window is a position and the
    /// text is not, so a ramp left behind fades the wrong part of the screen.
    #[must_use]
    pub fn translated(self, by: Offset) -> TextRun<'a> {
        TextRun {
            x: self.x + by.x,
            y: self.y + by.y,
            fade: self.fade.map(|ramp| ramp.translated(by)),
            clip: self.clip.map(|rect| rect.translated(by)),
            ..self
        }
    }
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
    ///
    /// **The argument has grown since that was written and the answer has not.**
    /// The command carries ten fields now, nine of them positional here, and the
    /// two the ramp and the clip need cannot be two more. So the struct the
    /// paragraph above argues against exists as [`TextRun`] — for the callers
    /// that have something to say about the ramp or the clip, which is the
    /// truncating label and nothing else. **This method is that type's front door
    /// for every caller that does not**, and it keeps the eight-argument
    /// signature it had rather than forcing the change on the six widgets that
    /// call it.
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
        self.text_run(TextRun {
            x,
            y,
            text,
            color,
            font_size,
            extra_advance,
            family,
            weight,
            fade: None,
            clip: None,
        });
    }

    /// Records a text run with a ramp and a clip box, in one named value.
    ///
    /// [`TextRun`]'s own docs are where the two extra fields are argued for; this
    /// is the recorder, and it is the **only** place a `DrawCommand::Text` is
    /// built, which is what stops the ramp and the clip from being a field some
    /// of the painters forget — the defect
    /// `Painter::text_in_weight` exists to prevent for the family and the
    /// weight, one level up.
    pub fn text_run(&mut self, run: TextRun<'_>) {
        self.commands.push(DrawCommand::Text {
            x: run.x,
            y: run.y,
            text: run.text.to_string(),
            color: run.color,
            font_size: run.font_size,
            extra_advance: run.extra_advance,
            family: run.family,
            weight: run.weight,
            fade: run.fade,
            clip: run.clip,
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

    /// Records one sub-mesh of an uploaded mesh, drawn through `mvp` in `tint`.
    ///
    /// The recorder's contract is the other nine methods': store what was
    /// handed over. The transform rides the command because the sub-meshes of
    /// one model share one buffer and differ only in the matrix they are drawn
    /// through — see [`DrawCommand::Mesh`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::paint::{Color, Painter, TextureId};
    /// use ui_core::render::matrix::Mat4;
    /// use ui_core::render::mesh::{MeshId, SubMeshRange};
    ///
    /// let mut painter = Painter::new();
    /// painter.mesh(
    ///     MeshId::new(0),
    ///     SubMeshRange {
    ///         first_index: 0,
    ///         index_count: 3,
    ///     },
    ///     Mat4::identity(),
    ///     Color::new(255, 255, 255, 255),
    ///     1.0,
    ///     TextureId::new(7),
    /// );
    /// let commands = painter.finish();
    /// assert_eq!(commands.len(), 1);
    /// ```
    pub fn mesh(
        &mut self,
        mesh: MeshId,
        range: SubMeshRange,
        mvp: Mat4,
        tint: Color,
        opacity: f32,
        texture: TextureId,
    ) {
        self.commands.push(DrawCommand::Mesh {
            mesh,
            range,
            mvp,
            tint,
            opacity,
            texture,
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
            fade: regular_fade,
            clip: regular_clip,
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
            fade: bold_fade,
            clip: bold_clip,
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
        // The two fields that came with the ramp and the clip, in the same
        // spirit as the paragraph above: a bold run must not pick up a fade or a
        // scissor because it was drawn through a different method.
        assert_eq!(regular_fade, bold_fade, "and neither run carries a ramp");
        assert_eq!(regular_clip, bold_clip, "and neither is clipped");
    }

    // ---------------------------------------------------------------- fade ramp

    /// The ramp every test below reads, unless it is about a different one:
    /// 48 pixels wide, so a factor is a sixteenth per pixel and a half is 24.
    fn ramp() -> FadeRamp {
        FadeRamp::new(240.0, 288.0)
    }

    #[test]
    fn the_ramp_is_whole_before_its_window_and_gone_at_and_after_its_end() {
        let ramp = ramp();
        assert_eq!(ramp_factor(ramp, 0.0), 1.0, "far to the left of the window");
        assert_eq!(ramp_factor(ramp, 239.0), 1.0, "and one pixel before it");
        assert_eq!(
            ramp_factor(ramp, 240.0),
            1.0,
            "**at** the start, not a hair into it: the start edge is the last x \\
             that is untouched, so a glyph whose left edge is exactly here is \\
             drawn whole"
        );
        assert_eq!(ramp_factor(ramp, 288.0), 0.0, "**at** the end is nothing");
        assert_eq!(ramp_factor(ramp, 289.0), 0.0, "and past it stays nothing");
        assert_eq!(ramp_factor(ramp, 9999.0), 0.0);
    }

    #[test]
    fn the_ramp_falls_monotonically_across_its_window() {
        // Acceptance criterion 1's "the ramp is monotonic", as arithmetic: every
        // sample is at or below the one before it, and the ends are the two
        // extremes the requirement names. A ramp built backwards is rising
        // rather than falling, and this is what sees it.
        let ramp = ramp();
        let mut previous = f32::INFINITY;
        let mut samples = 0;
        let mut x = 200.0;
        while x <= 320.0 {
            let factor = ramp_factor(ramp, x);
            assert!(
                factor <= previous,
                "the ramp rose: {factor} at x {x} after {previous}"
            );
            previous = factor;
            samples += 1;
            x += 0.25;
        }
        assert_eq!(
            samples, 481,
            "the whole neighbourhood was sampled, not three points"
        );
        assert_eq!(
            ramp_factor(ramp, 264.0),
            0.5,
            "and the midpoint is the midpoint, which is what makes the fall linear \\
             rather than merely non-increasing"
        );
        assert_eq!(
            ramp_factor(ramp, 252.0),
            0.75,
            "and a quarter in is three quarters"
        );
        assert_eq!(
            ramp_factor(ramp, 276.0),
            0.25,
            "and three quarters in is a quarter"
        );
    }

    #[test]
    fn the_fade_scales_all_four_channels_of_a_premultiplied_colour() {
        // Requirement 5, and the whole of it. A `Color` is premultiplied by
        // construction, so a fade that moved only the alpha would leave `r`, `g`
        // and `b` at full brightness while the background showed through them —
        // text at the wrong colour rather than text fading out. The assertion is
        // on all four channels *independently*, because the failure this guards
        // is three of them holding still.
        let ramp = ramp();
        let color = Color::new(200, 180, 160, 255);
        let half = faded_color(ramp, 264.0, color);
        assert_eq!(
            half,
            Color::new(100, 90, 80, 128),
            "every channel is halved"
        );
        assert_ne!(
            half.a, color.a,
            "**the alpha moved**, which is the half that is easy to see"
        );
        assert_ne!(
            half.r, color.r,
            "and the red moved, which is the half that is not"
        );
        assert_ne!(half.g, color.g);
        assert_ne!(half.b, color.b);
        // And the channels kept their ratio to each other, which is what
        // "premultiplied" means for a fade: nothing about the hue changed.
        assert_eq!(
            (half.r, half.g, half.b),
            (color.r / 2, color.g / 2, color.b / 2),
            "**and the ratio between the colour channels is untouched**, so this \\
             is a fade rather than a tint"
        );
    }

    #[test]
    fn a_fade_at_full_factor_is_the_colour_it_was_given_and_at_zero_transparent() {
        let ramp = ramp();
        let color = Color::new(7, 11, 13, 255);
        assert_eq!(
            faded_color(ramp, 0.0, color),
            color,
            "left of the window the run is the colour it was recorded with"
        );
        assert_eq!(
            faded_color(ramp, 240.0, color),
            color,
            "and so is a corner exactly on the start edge"
        );
        assert_eq!(
            faded_color(ramp, 288.0, color),
            Color::new(0, 0, 0, 0),
            "**transparent black and not black at zero alpha**: the channels are \\
             premultiplied, so an opaque-looking black here is a colour at \\
             opacity zero, which composites as black"
        );
    }

    #[test]
    fn a_fade_keeps_the_alpha_it_was_given_and_multiplies_it() {
        // The inherited-opacity half of requirement 5, as arithmetic: a colour
        // that arrived at alpha 128 and a colour that arrived at 255 fade by the
        // *same* factor, and the dimmer one stays proportionally dimmer. So a
        // ramp composes with whatever opacity the run already had rather than
        // replacing it.
        let ramp = ramp();
        let opaque = faded_color(ramp, 252.0, Color::new(200, 200, 200, 255));
        let dimmed = faded_color(ramp, 252.0, Color::new(200, 200, 200, 128));
        assert_eq!(opaque.a, 191, "a quarter of the way out of 255");
        assert_eq!(dimmed.a, 96, "and a quarter of the way out of 128");
        assert!(
            dimmed.a < opaque.a,
            "**the run that was already dimmer stays dimmer**, which is what \\
             composes with an inherited opacity instead of overwriting it"
        );
    }

    #[test]
    fn a_window_of_no_width_fades_nothing_and_does_not_divide_by_zero() {
        // The degenerate window a caller can build by hand, and the one
        // `FadeRamp::to_run_end` cannot produce. A zero-width window reaches both
        // of its ends at the same x, so it is read as whole there — one branch
        // wins and the division never happens.
        let degenerate = FadeRamp::new(100.0, 100.0);
        assert_eq!(ramp_factor(degenerate, 99.0), 1.0);
        assert_eq!(
            ramp_factor(degenerate, 100.0),
            1.0,
            "the shared end reads as whole"
        );
        assert_eq!(ramp_factor(degenerate, 101.0), 0.0);
        // And a window whose ends are the wrong way round, which is what a
        // caller gets from a negative width. Non-increasing still holds.
        let reversed = FadeRamp::new(200.0, 100.0);
        assert_eq!(ramp_factor(reversed, 50.0), 1.0);
        assert_eq!(
            ramp_factor(reversed, 150.0),
            1.0,
            "between the ends, still whole"
        );
        assert_eq!(ramp_factor(reversed, 200.0), 1.0);
        assert_eq!(ramp_factor(reversed, 300.0), 0.0);
    }

    #[test]
    fn a_ramps_start_never_precedes_the_run_it_belongs_to() {
        // The floor `FadeRamp::to_run_end` puts on `start_x`, and the reason it is
        // there: a run narrower than the ramp would otherwise begin part-way
        // down it and be drawn at a uniform partial opacity, which reads as a
        // wrong colour rather than as a fade.
        let narrow = FadeRamp::to_run_end(100.0, 130.0, 48.0);
        assert_eq!(
            narrow,
            FadeRamp::new(100.0, 130.0),
            "**the window starts at the run's left edge**, so the whole run ramps"
        );
        assert_eq!(
            ramp_factor(narrow, 100.0),
            1.0,
            "and the run's first glyph is whole"
        );
        assert_eq!(ramp_factor(narrow, 130.0), 0.0, "and its last is nothing");
        // A run wider than the ramp is unaffected by the floor.
        let wide = FadeRamp::to_run_end(100.0, 400.0, 48.0);
        assert_eq!(wide, FadeRamp::new(352.0, 400.0));
        assert_eq!(ramp_factor(wide, 100.0), 1.0);
        assert_eq!(ramp_factor(wide, 352.0), 1.0);
        assert_eq!(ramp_factor(wide, 400.0), 0.0);
    }

    #[test]
    fn a_text_run_records_its_ramp_and_its_clip_and_its_window_moves_them() {
        let ramp = FadeRamp::new(100.0, 148.0);
        let clip = Rect::new(60.0, 170.0, 90.0, 29.0);
        let mut painter = Painter::new();
        painter.text_run(TextRun {
            x: 60.0,
            y: 170.0,
            text: "cut here",
            color: Color::new(1, 2, 3, 255),
            font_size: 24.0,
            extra_advance: 0.0,
            fade: Some(ramp),
            clip: Some(clip),
            ..TextRun::default()
        });
        let commands = painter.finish();
        assert_eq!(commands.len(), 1);
        let DrawCommand::Text {
            x,
            y,
            text,
            fade: recorded_ramp,
            clip: recorded_clip,
            ..
        } = &commands[0]
        else {
            panic!("a text run records a text command");
        };
        assert_eq!(*x, 60.0);
        assert_eq!(*y, 170.0);
        assert_eq!(text, "cut here");
        assert_eq!(*recorded_ramp, Some(ramp), "**the ramp is on the command**");
        assert_eq!(*recorded_clip, Some(clip), "**and so is the box**");

        // `TextRun::translated` is the other half of `list::translate_commands`:
        // both ends of the window and every edge of the box are positions.
        let by = Offset::new(400.0, 30.0);
        let moved = TextRun {
            x: 60.0,
            y: 170.0,
            text: "cut here",
            color: Color::new(1, 2, 3, 255),
            font_size: 24.0,
            fade: Some(ramp),
            clip: Some(clip),
            ..TextRun::default()
        }
        .translated(by);
        assert_eq!((moved.x, moved.y), (460.0, 200.0));
        assert_eq!(
            moved.fade,
            Some(FadeRamp::new(500.0, 548.0)),
            "both edges move"
        );
        assert_eq!(
            moved.clip,
            Some(Rect::new(460.0, 200.0, 90.0, 29.0)),
            "and so does the box"
        );
        assert_eq!(moved.text, "cut here", "and the text is untouched");
        assert_eq!(moved.font_size, 24.0, "and so is the size");
    }

    #[test]
    fn a_plain_text_recorder_records_no_ramp_and_no_clip() {
        // The other side of the same seam: the eight-argument path has nothing to
        // say about either, so it says `None` for both, and every caller that was
        // not a truncating label is unaffected.
        let mut painter = Painter::new();
        painter.text(1.0, 2.0, "hi", Color::new(9, 9, 9, 255), 16.0, 0.0);
        painter.text_in(
            FamilyId::default(),
            1.0,
            2.0,
            "hi",
            Color::new(9, 9, 9, 255),
            16.0,
            0.0,
        );
        painter.text_bold(1.0, 2.0, "hi", Color::new(9, 9, 9, 255), 16.0, 0.0);
        for command in painter.finish() {
            let DrawCommand::Text { fade, clip, .. } = command else {
                panic!("three text recorders make text commands");
            };
            assert_eq!(fade, None, "no ramp without a ramp");
            assert_eq!(clip, None, "and no clip without a box");
        }
    }

    #[test]
    fn two_rectangles_intersect_to_the_box_both_of_them_cover() {
        let outer = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(
            outer.intersection(Rect::new(80.0, 10.0, 40.0, 80.0)),
            Rect::new(80.0, 10.0, 20.0, 40.0),
            "the overlap is where they agree, not either of them"
        );
        assert_eq!(
            outer.intersection(outer),
            outer,
            "and a rect is its own overlap"
        );
        assert_eq!(
            outer.intersection(Rect::new(10.0, 5.0, 20.0, 10.0)),
            Rect::new(10.0, 5.0, 20.0, 10.0),
            "**the intersection is the smaller of the two**, so a rect wholly \
             inside another is not shrunk by it"
        );
        assert_eq!(
            outer.intersection(Rect::new(-40.0, -40.0, 60.0, 140.0)),
            Rect::new(0.0, 0.0, 20.0, 50.0),
            "and one that hangs off every edge is cut on all four"
        );
        // **Not `None` for a miss.** An unset clip means *the whole window*, so a
        // `None` here would be read as "unbounded" and the clip would cut
        // nothing — the opposite of what the caller asked for.
        let missed = outer.intersection(Rect::new(200.0, 0.0, 10.0, 10.0));
        assert_eq!(
            missed,
            Rect::new(200.0, 0.0, 0.0, 10.0),
            "an overlap of no width"
        );
        assert!(missed.is_empty(), "**and it says so**");
        assert!(!outer.is_empty(), "while a real box does not");
        assert!(
            Rect::new(0.0, 0.0, 100.0, 0.0).is_empty(),
            "a zero height is empty too"
        );
        assert!(
            Rect::new(0.0, 0.0, -1.0, 10.0).is_empty(),
            "and so is a negative width"
        );
    }
}
