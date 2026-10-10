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
    /// A blurred drop shadow, drawn offscreen and composited on its own.
    ///
    /// **A shader kind and not a blend mode**, because the shadow is the only
    /// command in this module that cannot be drawn by one of the other three
    /// passes: it needs an offscreen target, a blur and a tint, none of which a
    /// quad carries. Giving it a kind of its own also means it can never merge
    /// into a solid batch — see [`BatchKey::is_singleton`].
    Shadow,
    /// A triangle mesh, drawn through the mesh program with the depth test on.
    ///
    /// **A shader kind and not a batch**, because a mesh command can never
    /// merge into another batch: a mesh's per-command state is a transform and
    /// an index range, and neither can ride in a vertex buffer the way a
    /// rounded rectangle's radius and size do, so two mesh commands are two
    /// draw calls however their keys compare — see
    /// [`BatchKey::is_singleton`].
    Mesh,
    /// A captured scene, blurred and composited where it was recorded.
    ///
    /// **What the kind is:** the scene already on screen, copied into an
    /// offscreen target, optionally blurred, and composited back over its own rect.
    ///
    /// **Why it can never merge:** a backdrop needs an offscreen target and a
    /// per-command rect, **neither of which can ride in a vertex buffer** — the way
    /// a rounded rectangle's radius and size do. So two backdrops are two draw
    /// calls however their keys compare, and the key is what stops the batcher
    /// from trying — see [`BatchKey::is_singleton`], which is where the frame is
    /// split at one.
    Backdrop,
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

impl BatchKey {
    /// Returns whether a batch under this key must hold exactly one command.
    ///
    /// **Three such keys are the shadow's, the mesh's and the backdrop's**, and
    /// the reason is about *where* a command draws rather than what it draws.
    /// [`Batcher::submit_order`] splits a frame into [`Segment`]s at every one of
    /// them, so a command that shared a batch with anything else — with the
    /// boundary before it, or with a solid rect — would take that neighbour across
    /// the boundary with it, and the composite would land one command too early or
    /// too late.
    ///
    /// Three keys are singletons and they are singletons for the same reason, which
    /// is that `submit_order` splits the frame at each of them: a command recorded
    /// after a mesh, a shadow or a backdrop must not merge into a batch recorded
    /// before it, or it lands on the wrong side of it. **They differ only in the
    /// second reason** — a shadow needs an offscreen target, a mesh needs a
    /// per-command transform, a backdrop needs an offscreen target and a
    /// per-command rect — and in none of them does merging buy anything: each is
    /// one draw call already.
    ///
    /// Merging is otherwise strictly good: it is the difference between one draw
    /// call and two. A singleton batch pays a second draw call every frame, which
    /// is why this is a property of the key rather than a flag every caller has
    /// to remember to set.
    #[must_use]
    pub fn is_singleton(&self) -> bool {
        matches!(
            self.shader,
            ShaderKind::Shadow | ShaderKind::Mesh | ShaderKind::Backdrop
        )
    }
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

/// One run of batches between two shadows, and the shadow that ends it.
///
/// **This is a frame's submission order, and the order is the whole point of
/// the type.** A group is not enough: grouping every opaque batch before every
/// translucent one is correct for translucent-over-opaque, which is all the
/// application stacked until a modal dialog — an overlay at alpha 128 and a
/// panel at the theme's opaque surface — needed the other order. Both are
/// recorded in the right order and both commands are right; only the submission
/// was wrong, and no assertion on a recorded command can see that.
///
/// A frame with no shadow has exactly one segment, and that segment's two groups
/// are the two [`BatchedCommands`] groups — the same batches, in the same order,
/// by the same code. [`Batcher::submit_order`] and [`Batcher::finish`] share
/// `Batcher::groups_of` so that equality is structural rather than a
/// coincidence two methods have to keep.
///
/// # Examples
///
/// ```
/// use ui_core::batch::Batcher;
/// use ui_core::paint::{Color, DrawCommand, Painter, Rect};
///
/// // A modal dialog: the overlay, then the shadow, then the panel. Both of the
/// // overlay and the panel would have landed in one opaque-first frame without
/// // the segmentation, and the overlay would have ended up on top of the panel.
/// let mut painter = Painter::new();
/// painter.rect(Rect::new(0.0, 0.0, 800.0, 600.0), Color::new(0, 0, 0, 128));
/// painter.shadow(
///     Rect::new(240.0, 160.0, 320.0, 200.0),
///     12.0,
///     Color::new(0, 0, 0, 128),
///     6.0,
///     (0.0, 8.0),
/// );
/// painter.rounded_rect(
///     Rect::new(240.0, 160.0, 320.0, 200.0),
///     12.0,
///     Color::new(245, 245, 245, 255),
/// );
///
/// let mut batcher = Batcher::new();
/// for command in painter.finish() {
///     batcher.add(command);
/// }
///
/// let segments = batcher.submit_order();
/// assert_eq!(segments.len(), 2, "the shadow is one boundary among three commands");
/// assert_eq!(segments[0].transparent.len(), 1, "the overlay, before the shadow");
/// assert!(segments[0].opaque.is_empty());
/// assert!(segments[0].shadow.is_some());
/// assert_eq!(segments[1].opaque.len(), 1, "the panel, after it");
/// assert!(segments[1].shadow.is_none());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    /// Batches drawn front-to-back with blending disabled.
    pub opaque: Vec<Batch>,
    /// Batches drawn back-to-front with premultiplied-alpha blending.
    pub transparent: Vec<Batch>,
    /// The shadow to composite once this segment is on screen, as a batch
    /// holding that one command and the clip it was recorded under.
    ///
    /// A [`Batch`] and not a [`DrawCommand`] so that the shadow's clip rides
    /// along the way [`Batch::clip`] says every other batch's does: the
    /// composite is a draw call, and a draw call has one scissor.
    ///
    /// The boundary the segment ends with — see `mesh`.
    pub shadow: Option<Batch>,
    /// The mesh to draw once this segment is on screen, as a batch holding
    /// that one command and the clip it was recorded under.
    ///
    /// A [`Batch`] and not a [`DrawCommand`] for the same reason as `shadow`:
    /// the draw is a draw call, and a draw call has one scissor.
    ///
    /// The boundary the segment ends with — at most one of `shadow`, `mesh` and
    /// `backdrop` is `Some`, because a seal consumes exactly one singleton
    /// command. Not a sum type: the invariant needs a test rather than a
    /// type, and a sum would rename a `pub` field twenty-three tests name.
    pub mesh: Option<Batch>,
    /// The backdrop to composite once this segment is on screen, as a batch
    /// holding that one command and the clip it was recorded under.
    ///
    /// A [`Batch`] and not a [`DrawCommand`] for the same reason as `shadow`: the
    /// composite is a draw call, and a draw call has one scissor.
    ///
    /// The boundary the segment ends with — **at most one of `shadow`, `mesh` and
    /// `backdrop` is `Some`, because a seal consumes exactly one singleton
    /// command.** Which also means **a backdrop, a shadow and a mesh are never
    /// adjacent**, so their order relative to each other is not observable; the
    /// one case where it *is* is a shadow recorded immediately after a backdrop,
    /// which lands on it, and that is the caller's ordering again.
    pub backdrop: Option<Batch>,
}

/// Groups draw commands into batches by [`BatchKey`].
///
/// Batches keep recording order, so the opaque group is front-to-back; the
/// transparent group is reversed by [`Batcher::finish`] so it composites
/// back-to-front. The two are per *segment*: see [`Batcher::submit_order`] for
/// why a frame is more than one of them and what the segmentation is for.
#[derive(Clone, Debug, Default)]
pub struct Batcher {
    /// The batches of the segment being recorded. **Only this list is searched
    /// for a batch to merge into.**
    open: Vec<Batch>,
    /// The segments already sealed, in recorded order.
    sealed: Vec<Vec<Batch>>,
}

impl Batcher {
    /// Creates an empty batcher.
    #[must_use]
    pub fn new() -> Self {
        Batcher {
            open: Vec::new(),
            sealed: Vec::new(),
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
    ///
    /// **`clip` is intersected with the clip the command carries**, and the
    /// *effective* clip is what the batches are compared on — see
    /// `DrawCommand::clip`. A command clipped to its own box and recorded
    /// inside a scrolling viewport is clipped to **both**, because a node's own
    /// box is not the window: taking either one alone would either clip the run
    /// to the viewport and let it spill out of its own box, or clip it to its own
    /// box and let it spill out of the viewport.
    ///
    /// **A command whose key [`BatchKey::is_singleton`] seals the open segment
    /// and starts the next**, so it is alone in its batch *and* nothing recorded
    /// after it can merge into a batch recorded before it. That second half is
    /// not a detail of the shadow; it is what makes the segmentation mean
    /// anything. Two opaque rects — the window's background and a dialog's
    /// panel — share a key and a clip, so without the seal they are one batch,
    /// one batch has one position in the recorded stream, and both would be
    /// submitted **before** the translucent overlay the panel is supposed to sit
    /// on top of. The panel would be dimmed, which is the defect
    /// [`Segment`] exists to fix, arrived at by a different road.
    pub fn add_clipped(&mut self, command: DrawCommand, clip: Option<Rect>) {
        let clip = match (clip, command.clip()) {
            (Some(outer), Some(inner)) => Some(outer.intersection(inner)),
            (only @ Some(_), None) | (None, only @ Some(_)) => only,
            (None, None) => None,
        };
        let key = command.batch_key();
        if key.is_singleton() {
            let mut finished = std::mem::take(&mut self.open);
            finished.push(Batch {
                key,
                commands: vec![command],
                clip,
            });
            self.sealed.push(finished);
            return;
        }
        if let Some(batch) = self
            .open
            .iter_mut()
            .find(|batch| batch.key == key && batch.clip == clip)
        {
            batch.commands.push(command);
        } else {
            self.open.push(Batch {
                key,
                commands: vec![command],
                clip,
            });
        }
    }

    /// Clears every batch, leaving the batcher usable for the next frame.
    pub fn reset(&mut self) {
        self.open.clear();
        self.sealed.clear();
    }

    /// Returns `true` if no commands have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.open.is_empty() && self.sealed.iter().all(|segment| segment.is_empty())
    }

    /// Splits `batches` into an opaque group in recording order and a
    /// transparent group reversed to back-to-front.
    ///
    /// The one place that grouping happens. [`Batcher::finish`] and
    /// [`Batcher::submit_order`] both go through it, so a frame with no shadow
    /// and the first segment of a frame with one are the same batches in the same
    /// order **by construction** — which matters, because this grouping touches
    /// the submission order of every frame in the application and a second copy
    /// of the rule would be free to drift from it.
    fn groups_of(batches: impl IntoIterator<Item = Batch>) -> BatchedCommands {
        let mut opaque = Vec::new();
        let mut transparent = Vec::new();
        for batch in batches {
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

    /// Drains the batches into an opaque group in recording order and a
    /// transparent group reversed to back-to-front.
    ///
    /// **This is the whole frame's grouping and nothing knows about order across
    /// a shadow.** A caller that has a frame with a shadow in it wants
    /// [`Batcher::submit_order`] instead: this method has nowhere to put the
    /// boundary, and putting the panel of a modal dialog in front of its overlay
    /// is the defect [`Segment`] exists to fix. It stays for the callers that ask
    /// about one frame's materials rather than its order.
    pub fn finish(&mut self) -> BatchedCommands {
        let mut batches = Vec::new();
        for segment in self.sealed.drain(..) {
            batches.extend(segment);
        }
        batches.append(&mut self.open);
        Self::groups_of(batches)
    }

    /// Drains the batches into the order they are submitted in: one [`Segment`]
    /// per run of commands between two shadows, in recorded order.
    ///
    /// A frame with no shadow yields **one** segment, and that segment is
    /// [`Batcher::finish`]'s two groups exactly — see [`Segment`] and
    /// `Batcher::groups_of`. A frame with `n` shadows yields `n + 1` segments,
    /// each of which may be empty of one group or of both: a shadow recorded
    /// first is a segment with nothing but the shadow in it, and nothing in the
    /// frame says that is not what was meant.
    ///
    /// The split is at the shadow and not around it, so the shadow is
    /// [`Segment::shadow`] of the segment it **ends**, and everything recorded
    /// after it is in the next one. That is what puts a panel's shadow behind the
    /// panel and in front of the overlay the panel sits on.
    pub fn submit_order(&mut self) -> Vec<Segment> {
        let mut segments: Vec<Segment> = Vec::new();
        for sealed in self.sealed.drain(..) {
            let mut batches = sealed;
            // Sealing puts the singleton last and nothing merges into it, so the
            // last batch is the boundary. The `match` rather than an `expect`
            // because a batch that turned out not to be one has to go back into
            // the segment it came from, not vanish.
            let popped = match batches.pop() {
                Some(batch) if batch.key.is_singleton() => Some(batch),
                other => {
                    if let Some(batch) = other {
                        batches.push(batch);
                    }
                    None
                }
            };
            // A seal consumes exactly one singleton command, so the boundary is
            // whichever kind it was — and a mesh, a shadow and a backdrop never
            // share a segment, which is the invariant the suite asserts rather
            // than the type.
            let (shadow, mesh, backdrop) = match popped {
                Some(batch) if batch.key.shader == ShaderKind::Shadow => (Some(batch), None, None),
                Some(batch) if batch.key.shader == ShaderKind::Backdrop => {
                    (None, None, Some(batch))
                }
                Some(batch) => (None, Some(batch), None),
                None => (None, None, None),
            };
            let BatchedCommands {
                opaque,
                transparent,
            } = Self::groups_of(batches);
            segments.push(Segment {
                opaque,
                transparent,
                shadow,
                mesh,
                backdrop,
            });
        }
        // The trailing run after the last boundary is a segment with no boundary
        // of its own. A frame that ended on a shadow still has one, and it is empty
        // — which keeps "the frame is a list of segments" true whatever the last
        // command was, instead of leaving the caller to ask whether the list ends
        // in one.
        let BatchedCommands {
            opaque,
            transparent,
        } = Self::groups_of(self.open.drain(..));
        segments.push(Segment {
            opaque,
            transparent,
            shadow: None,
            mesh: None,
            backdrop: None,
        });
        segments
    }
}

impl DrawCommand {
    /// Returns the clip this command carries itself, in window coordinates.
    ///
    /// **`None` for every variant but [`Text`](DrawCommand::Text)**, which is the
    /// only one with a box of its own — a label that truncates with
    /// [`Truncation::Clip`](crate::widgets::label::Truncation::Clip) records the
    /// box it is cut at, and no other primitive in the enum has a reason to
    /// carry one.
    ///
    /// **Read here rather than in [`Batcher::add_clipped`], so the intersection
    /// has one shape to get right.** A new variant that clips would be a missing
    /// arm in one match rather than a rule spread over a `match` in the batcher
    /// and a `match` in the renderer.
    pub(crate) fn clip(&self) -> Option<Rect> {
        match self {
            DrawCommand::Text { clip, .. } => *clip,
            _ => None,
        }
    }

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
            // **A run with a fade ramp is batched as `from_color` says and not as
            // `Transparent`, and that is read from the frame's submission order
            // rather than guessed at.** `BlendMode::from_color` sees the
            // command's colour, and a faded corner is a vertex the renderer
            // scales long after the key was taken — so a ramped run of opaque
            // text lands in the opaque group with `BlendMode::Opaque` written on
            // it. It composites correctly anyway: `Renderer::end_frame` disables
            // blending for the *solid* pass only and then enables it **once**,
            // before `COMPOSITED_PASSES`, and that loop draws `segment.opaque` as
            // well as `segment.transparent` with the blend still on and never
            // disabled again. `text_faded_text_is_still_drawn_with_blending_on`
            // pins that in `render.rs` by asserting over `end_frame`'s own text,
            // with a control — a caption here would be the one part of this
            // decision nothing checks.
            //
            // The alternative is one extra batch per ramped run, in exchange for
            // an assurance about a state change that is not made.
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
            // The blend mode is [`BlendMode::from_color`] of the shadow's own
            // colour and nothing decides anything with it: the composite turns
            // blending on explicitly, the way it does for every translucent
            // batch, and a shadow the caller made opaque blends exactly as
            // little as it is asked to. Reporting it honestly is what lets the
            // tests read a shadow's alpha off the key.
            DrawCommand::Shadow { color, .. } => BatchKey {
                texture: None,
                blend_mode: BlendMode::from_color(*color),
                shader: ShaderKind::Shadow,
            },
            // **The blend mode is [`BlendMode::Transparent`] unconditionally, and
            // it declines [`BlendMode::from_color`]'s rule on purpose.** A
            // backdrop at `tint.a == 255` is still a *blend*, because the
            // capture's alpha is the framebuffer's accumulated coverage and a
            // translucent primitive recorded before the backdrop left that below
            // 1.0 somewhere — an `Opaque` batch would overwrite it with `src`
            // rather than `src + dst·(1 − a)`, and put a hard edge exactly where a
            // blur has none. `from_color` answers *"does this command's own colour
            // ask for blending?"*, which is the right question for every other
            // variant here and the wrong one for this: a backdrop's blend mode does
            // not come from its tint at all, which is also why the command carries
            // no separate `opacity` scalar the way `Image` and `Mesh` do.
            DrawCommand::Backdrop { .. } => BatchKey {
                texture: None,
                blend_mode: BlendMode::Transparent,
                shader: ShaderKind::Backdrop,
            },
            // A mesh draws in the opaque pass only when it asks for no blending
            // of its own: exactly `1.0`, not "at least", for the same reason
            // `DrawCommand::Image`'s arm says — an opacity past one is clamped
            // at draw time, so it draws the same as `1.0` and is batched as
            // blending all the same. The colormap rides `texture` the way the
            // image pass's does: it is what the pass reads the sampler from.
            DrawCommand::Mesh {
                texture, opacity, ..
            } => BatchKey {
                texture: Some(*texture),
                blend_mode: if *opacity == 1.0 {
                    BlendMode::Opaque
                } else {
                    BlendMode::Transparent
                },
                shader: ShaderKind::Mesh,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{FamilyId, FontWeight, Rect, UvRect};
    use crate::render::matrix::Mat4;
    use crate::render::mesh::{MeshId, SubMeshRange};

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
            family: FamilyId::default(),
            weight: FontWeight::Regular,
            fade: None,
            clip: None,
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
            family: FamilyId::default(),
            weight: FontWeight::Regular,
            fade: None,
            clip: None,
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

    /// A text run of `color` clipped to `clip`, recorded through the painter so
    /// the test builds the command the way production does.
    fn clipped_text(color: Color, clip: Option<Rect>) -> DrawCommand {
        let mut painter = crate::paint::Painter::new();
        painter.text_run(crate::paint::TextRun {
            x: 4.0,
            y: 8.0,
            text: "a run",
            color,
            font_size: 16.0,
            clip,
            ..crate::paint::TextRun::default()
        });
        let commands = painter.finish();
        match commands.into_iter().next() {
            Some(command) => command,
            None => panic!("a text run records one command"),
        }
    }

    #[test]
    fn two_text_runs_whose_own_clips_differ_do_not_share_a_batch() {
        // Requirement 4's "per-node clip must not leak", at the layer where it is
        // decided: a scissor is a draw call's state, so two runs in one batch
        // would be drawn under whichever rect the batch happened to carry.
        let a = Some(Rect::new(0.0, 0.0, 100.0, 20.0));
        let b = Some(Rect::new(0.0, 400.0, 100.0, 20.0));
        let mut batcher = Batcher::new();
        batcher.add_clipped(clipped_text(opaque(), a), None);
        batcher.add_clipped(clipped_text(opaque(), b), None);

        let batched = batcher.finish();
        assert_eq!(
            batched.opaque.len(),
            2,
            "**the batch split on the effective clip**, which is the split the \
             renderer needs: one batch is one draw call and a draw call has one \
             scissor"
        );
        assert_eq!(batched.opaque[0].clip, a);
        assert_eq!(batched.opaque[1].clip, b);
        assert_eq!(
            batched.opaque[0].commands.len(),
            1,
            "and neither batch holds the other's run"
        );
    }

    #[test]
    fn a_commands_own_clip_is_intersected_with_the_batchs_clip_and_not_replacing_it() {
        // The two clips are two *different* claims — a node's own box and the
        // viewport it is inside — and the run has to obey both. Taking either one
        // alone is a wrong picture in one direction: the node's box alone lets
        // the run spill out of the viewport, and the viewport alone lets it spill
        // out of its own box.
        let viewport = Some(Rect::new(0.0, 0.0, 200.0, 200.0));
        let own = Some(Rect::new(0.0, 10.0, 120.0, 20.0));
        let mut batcher = Batcher::new();
        batcher.add_clipped(clipped_text(opaque(), own), viewport);

        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1);
        assert_eq!(
            batched.opaque[0].clip,
            Some(Rect::new(0.0, 10.0, 120.0, 20.0)),
            "**the node's box is inside the viewport, so the overlap is the node's \
             box** — and not `Some(viewport)`, which is what replacing rather than \
             intersecting would leave"
        );

        // And the other way round, where the viewport is the smaller claim: the
        // overlap is then the viewport, and a run recorded inside a node that
        // hangs off the bottom of the window is cut at the window.
        let tall = Some(Rect::new(0.0, 180.0, 400.0, 400.0));
        let mut batcher = Batcher::new();
        batcher.add_clipped(clipped_text(opaque(), tall), viewport);
        let batched = batcher.finish();
        assert_eq!(
            batched.opaque[0].clip,
            Some(Rect::new(0.0, 180.0, 200.0, 20.0)),
            "the window is inside the node here, and the overlap is the window's \
             20 pixels of it"
        );
    }

    #[test]
    fn two_runs_whose_effective_clip_is_the_same_share_a_batch_and_two_that_differ_do_not() {
        // The merge predicate compares the **effective** clip, so two runs that
        // arrive by different routes to the same box are one draw call and two
        // runs that arrive at different boxes are two. A predicate that compared
        // the caller's clip instead would split this first pair in two, which is
        // one draw call a frame for nothing.
        let viewport = Some(Rect::new(0.0, 0.0, 200.0, 200.0));
        let own = Some(Rect::new(0.0, 10.0, 120.0, 20.0));
        let mut batcher = Batcher::new();
        // Three routes to `own`: it alone, it under a viewport that contains it,
        // and it under a caller that claims the very same box.
        batcher.add_clipped(clipped_text(opaque(), own), viewport);
        batcher.add_clipped(clipped_text(opaque(), own), own);
        batcher.add_clipped(
            clipped_text(opaque(), Some(Rect::new(0.0, 10.0, 120.0, 20.0))),
            None,
        );
        // And one route to the viewport, which is a different box.
        batcher.add_clipped(clipped_text(opaque(), None), viewport);

        let batched = batcher.finish();
        assert_eq!(
            batched.opaque.len(),
            2,
            "**two effective clips, two batches** — a predicate that compared the \
             caller's clip instead of the effective one would make four"
        );
        assert_eq!(
            batched.opaque[0].commands.len(),
            3,
            "the three that are all `own`"
        );
        assert_eq!(batched.opaque[0].clip, own);
        assert_eq!(batched.opaque[1].commands.len(), 1);
        assert_eq!(batched.opaque[1].clip, viewport);
    }

    #[test]
    fn two_unclipped_text_runs_still_share_one_batch() {
        // The no-regression half of the same seam. Two runs with no clip
        // anywhere are the same batch they were before any of this existed: the
        // merge predicate's new term is `None == None`.
        let mut batcher = Batcher::new();
        batcher.add(clipped_text(opaque(), None));
        batcher.add(clipped_text(opaque(), None));
        let batched = batcher.finish();
        assert_eq!(batched.opaque.len(), 1, "one draw call, as before");
        assert_eq!(batched.opaque[0].commands.len(), 2);
        assert_eq!(batched.opaque[0].clip, None);
    }

    #[test]
    fn a_clip_that_cuts_a_run_out_of_its_own_box_leaves_a_batch_that_draws_nothing() {
        // The one case where the intersection is empty, and it is the reason
        // `Rect::intersection` answers a box rather than a `None`: an unset clip
        // means *the whole window*, so a `None` here would un-clip the run and
        // draw the very thing the clip was there to cut. A box of no width is
        // what a scissor reads as "nothing".
        let mut batcher = Batcher::new();
        batcher.add_clipped(
            clipped_text(opaque(), Some(Rect::new(0.0, 0.0, 10.0, 10.0))),
            Some(Rect::new(500.0, 500.0, 10.0, 10.0)),
        );
        let batched = batcher.finish();
        let clip = batched.opaque[0]
            .clip
            .expect("the effective clip is a box, not a None");
        assert!(
            clip.is_empty(),
            "an overlap of no area, at the nearer origin"
        );
        assert_eq!(clip, Rect::new(500.0, 500.0, 0.0, 0.0));
    }

    #[test]
    fn a_clip_is_not_part_of_the_batch_key_and_a_ramp_is_not_either() {
        // The reason [`BatchKey`] holds three fields and not four (or five), and
        // the reason a ramp rides the command rather than the key. Keying on the
        // clip would split one scrolling list into one batch per row — the exact
        // defect `Batch::clip`'s own doc names — and keying on the *presence of a
        // ramp* would do the same for every truncated line on the screen.
        let mut with_ramp = Batcher::new();
        with_ramp.add_clipped(
            clipped_text(opaque(), Some(Rect::new(0.0, 0.0, 200.0, 20.0))),
            None,
        );
        with_ramp.add_clipped(clipped_text(opaque(), None), None);
        let ramped = with_ramp.finish();
        assert_eq!(
            ramped.opaque.len(),
            2,
            "two clips, two batches — the clip splits"
        );
        assert_eq!(
            ramped.opaque[0].key, ramped.opaque[1].key,
            "**while the keys are equal**, which is what says the clip is not in \\
             them: `Batch::clip` is a field on the batch, and one text batch still \\
             covers every run in the frame that shares a clip"
        );
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

    // The tests below cover the submission order. They exist because the
    // ordering they pin is **invisible to every other assertion in this
    // module**: a draw-command assertion asks what was recorded, and both
    // commands of a modal dialog are recorded, with the right colours, in the
    // right order. Only the order they are *submitted* in was wrong — the
    // overlay, at alpha 128, landing on top of the panel at the theme's opaque
    // surface — and that is what these tests are about.

    /// A shadow of a rounded rect, off the origin so a fixture cannot mistake
    /// an offset for the rect's own.
    fn shadow() -> DrawCommand {
        DrawCommand::Shadow {
            rect: Rect::new(240.0, 160.0, 320.0, 200.0),
            radius: 12.0,
            color: Color::new(0, 0, 0, 128),
            blur: 6.0,
            offset: (0.0, 8.0),
        }
    }

    /// A rounded rect of `color`, at a rect whose `index` keeps two of them from
    /// sharing one in a test that cares about how many there are.
    fn rounded_at(index: u32, color: Color) -> DrawCommand {
        DrawCommand::RoundedRect {
            // `f32` has `From<u16>` but no `From<u32>`, and these indices are
            // single digits: a fixture that needed a wide one would be testing
            // the conversion rather than the batching.
            rect: Rect::new(
                f32::from(u16::try_from(index).unwrap_or(0)) * 40.0,
                80.0,
                32.0,
                24.0,
            ),
            radius: 4.0,
            color,
        }
    }

    /// The shaders a segment's two groups hold, in submission order: the opaque
    /// group first, then the transparent one as it composites.
    fn shaders_of(segment: &Segment) -> Vec<ShaderKind> {
        segment
            .opaque
            .iter()
            .chain(segment.transparent.iter())
            .map(|batch| batch.key.shader)
            .collect()
    }

    /// A mesh command at `opacity`, through the identity matrix in opaque
    /// white, sampling texture 7.
    fn mesh(opacity: f32) -> DrawCommand {
        DrawCommand::Mesh {
            mesh: MeshId::new(0),
            range: SubMeshRange {
                first_index: 0,
                index_count: 3,
            },
            mvp: Mat4::identity(),
            tint: Color::new(255, 255, 255, 255),
            opacity,
            texture: TextureId::new(7),
        }
    }

    #[test]
    fn a_frame_with_no_shadow_is_one_segment_and_the_groups_it_has_always_had() {
        // The control for everything below: the same three commands with no
        // shadow between two of them are **one** segment, and its groups are the
        // ones `finish` has always returned. Without this case a segmentation
        // that split at nothing and lost a batch would look correct.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, transparent()));
        batcher.add(rounded_at(1, opaque()));
        batcher.add(rounded_at(2, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 1, "no shadow, so no boundary");
        assert!(segments[0].shadow.is_none(), "and nothing to composite");

        // And the exact groups, measured against `finish`'s own answer on the
        // same commands — which is the claim that a no-shadow frame submits
        // exactly what it submitted before this type existed.
        let mut control = Batcher::new();
        control.add(rounded_at(0, transparent()));
        control.add(rounded_at(1, opaque()));
        control.add(rounded_at(2, opaque()));
        let expected = control.finish();

        assert_eq!(segments[0].opaque, expected.opaque);
        assert_eq!(segments[0].transparent, expected.transparent);
        assert_eq!(
            segments[0].opaque.len(),
            1,
            "one batch, as before: the two opaque rects share a key and a clip"
        );
        assert_eq!(
            segments[0].opaque[0].commands.len(),
            2,
            "and both commands are in it, as they were"
        );
        assert_eq!(segments[0].transparent.len(), 1);
    }

    #[test]
    fn an_empty_frame_is_one_empty_segment() {
        // The frame with nothing in it has to have an answer: a renderer that
        // walked segments would otherwise have to ask whether the list was empty
        // and treat "no commands" and "no segments" as different cases.
        let mut batcher = Batcher::new();
        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].opaque.is_empty());
        assert!(segments[0].transparent.is_empty());
        assert!(segments[0].shadow.is_none());
    }

    #[test]
    fn a_shadow_is_a_boundary_and_the_commands_around_it_keep_their_recorded_order() {
        // The dialog: an overlay, a shadow, a panel. Recorded in that order and
        // submitted in that order — which is the requirement, and which the
        // pre-segmentation grouping could not express, because it drew every
        // opaque batch before every translucent one.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, transparent()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(
            segments.len(),
            2,
            "one boundary among three commands, and a trailing segment"
        );
        assert_eq!(
            shaders_of(&segments[0]),
            vec![ShaderKind::Solid],
            "the overlay, on its own before the shadow"
        );
        assert!(
            segments[0].opaque.is_empty(),
            "an overlay is translucent, so the opaque group before a shadow is \
             empty and the frame's stacking is carried by the segmentation"
        );
        assert_eq!(
            shaders_of(&segments[1]),
            vec![ShaderKind::Solid],
            "the panel after it"
        );
        assert!(
            segments[1].shadow.is_none(),
            "the last segment has no shadow"
        );
    }

    #[test]
    fn an_opaque_command_after_a_shadow_lands_in_a_later_segment_than_a_translucent_one_before_it()
    {
        // The property itself, stated as an index comparison rather than as two
        // counts, because the counts pass in three different wrong orderings:
        // two segments with the shadow's group on either side, and one segment
        // holding both. This is the assertion that does not.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, transparent()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));

        let segments = batcher.submit_order();
        let index_of = |segment: usize, group: fn(&Segment) -> &Vec<Batch>| -> usize {
            segments
                .iter()
                .position(|candidate| !group(candidate).is_empty())
                .map(|found| found + segment)
                .unwrap_or(segments.len() + segment)
        };
        let translucent = index_of(0, |segment| &segment.transparent);
        let opaque = index_of(1, |segment| &segment.opaque);
        assert!(
            translucent < opaque,
            "the overlay is submitted at segment {translucent} and the panel at \
             {opaque}: the opaque panel has to be drawn **after** the \
             translucent overlay, or the overlay dims it"
        );
    }

    #[test]
    fn a_shadow_alone_is_a_segment_that_holds_nothing_but_the_shadow() {
        // Recorded first, before anything is drawn under it. The frame is
        // [shadow, overlay, panel], so the shadow's own segment is empty of both
        // groups and the list still starts with it — a shadow is where it was
        // put, and nothing here says otherwise.
        let mut batcher = Batcher::new();
        batcher.add(shadow());
        batcher.add(rounded_at(0, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 2);
        assert!(segments[0].opaque.is_empty(), "nothing was drawn before it");
        assert!(segments[0].transparent.is_empty());
        assert!(
            segments[0].shadow.is_some(),
            "and the shadow is still first, with an empty segment before it"
        );
        assert_eq!(segments[1].opaque.len(), 1, "the panel follows it");
    }

    #[test]
    fn a_frame_ending_on_a_shadow_still_ends_in_a_segment() {
        // The trailing segment is empty rather than absent, so a caller walking
        // segments does not have to ask whether the list ends in one. The control
        // is the same three commands with one more after the shadow.
        let mut batcher = Batcher::new();
        batcher.add(shadow());

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 2, "the shadow and then the trailing one");
        assert!(segments[1].opaque.is_empty());
        assert!(segments[1].transparent.is_empty());
        assert!(segments[1].shadow.is_none());

        let mut with_a_command_after = Batcher::new();
        with_a_command_after.add(shadow());
        with_a_command_after.add(rounded_at(0, opaque()));
        assert_eq!(
            with_a_command_after.submit_order().len(),
            2,
            "the same two segments: the trailing one now holds the command"
        );
    }

    #[test]
    fn n_shadows_are_n_plus_one_segments() {
        // The count, with the control beside it: the same commands with the
        // shadows left out are one segment, so the count cannot be right by
        // having found the right batches.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(2, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 3, "two shadows, three segments");
        assert!(segments[0].shadow.is_some(), "the first boundary");
        assert!(segments[1].shadow.is_some(), "the second");
        assert!(segments[2].shadow.is_none(), "and the trailing one");
        for segment in &segments {
            assert_eq!(
                segment.opaque.len(),
                1,
                "each segment holds the one opaque command recorded next to it"
            );
        }

        let mut control = Batcher::new();
        control.add(rounded_at(0, opaque()));
        control.add(rounded_at(1, opaque()));
        control.add(rounded_at(2, opaque()));
        assert_eq!(
            control.submit_order().len(),
            1,
            "the same three commands with no shadow are one segment"
        );
    }

    #[test]
    fn two_shadows_of_the_same_material_are_still_two_boundaries() {
        // The merge this has to refuse: both shadows are the same colour and the
        // same kind, so the batching rule that merges everything else merges
        // them into one batch of two commands — and one batch has one position in
        // the recorded stream, so the boundary between the two shadows would be
        // gone and the first one's composite would land after the second.
        let mut batcher = Batcher::new();
        batcher.add(shadow());
        batcher.add(shadow());

        let segments = batcher.submit_order();
        assert_eq!(
            segments.len(),
            3,
            "two shadows are two boundaries however alike they are"
        );
        for segment in &segments[..2] {
            let batch = segment.shadow.as_ref().expect("a shadow of its own");
            assert_eq!(
                batch.commands.len(),
                1,
                "and each shadow's batch holds one command, not two"
            );
        }
    }

    #[test]
    fn a_shadow_never_merges_into_a_solid_batch() {
        // The other half of `is_singleton`, and the one that would be a wrong
        // picture rather than a wrong count: a shadow's commands are handed to
        // the shadow pass, so a batch holding a rect and a shadow would submit
        // the rect to neither pass it is drawn by.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));

        let segments = batcher.submit_order();
        let shadow_batch = segments[0].shadow.as_ref().expect("the shadow");
        assert_eq!(shadow_batch.key.shader, ShaderKind::Shadow);
        assert!(
            shadow_batch
                .commands
                .iter()
                .all(|command| { matches!(command, DrawCommand::Shadow { .. }) }),
            "and the shadow batch holds nothing but the shadow"
        );
        assert_eq!(
            segments[1].opaque.len(),
            1,
            "the two rects are in two segments"
        );
    }

    #[test]
    fn a_shadows_batch_key_names_its_shader_and_reports_its_own_colour() {
        // The key is what routes the command to the shadow pass, and what
        // `submit_order` splits on. A shadow carrying a texture would be a
        // batch that cannot be drawn by the pass that owns it.
        let translucent = shadow().batch_key();
        assert_eq!(translucent.shader, ShaderKind::Shadow);
        assert_eq!(translucent.texture, None, "a shadow samples no texture");
        assert_eq!(translucent.blend_mode, BlendMode::Transparent);

        // The boundary, on both sides of it: an opaque shadow is still its own
        // batch, because "singleton" is about the shader and not about blending.
        let mut opaque_shadow = shadow();
        if let DrawCommand::Shadow { color, .. } = &mut opaque_shadow {
            *color = Color::new(0, 0, 0, 255);
        }
        let key = opaque_shadow.batch_key();
        assert_eq!(
            key.blend_mode,
            BlendMode::Opaque,
            "its own alpha still says so"
        );
        assert!(key.is_singleton(), "and it is still its own batch");

        // The control: no other key is a singleton, or every batch in the
        // application would be a draw call of its own.
        assert!(!rect(opaque()).batch_key().is_singleton());
        assert!(!image(1).batch_key().is_singleton());
    }

    #[test]
    fn a_mesh_is_a_singleton_and_no_2d_command_is() {
        // The mesh seals the segment for the same reason the shadow does: a
        // command recorded after it must not merge into a batch recorded before
        // it, or it lands on the wrong side of the car.
        assert!(mesh(1.0).batch_key().is_singleton());
        assert!(mesh(0.5).batch_key().is_singleton());

        // The controls: no 2D command seals anything, and the shadow still
        // seals for its own reason.
        assert!(!rect(opaque()).batch_key().is_singleton());
        assert!(!rounded_at(0, opaque()).batch_key().is_singleton());
        assert!(!image(1).batch_key().is_singleton());
        assert!(shadow().batch_key().is_singleton());
    }

    #[test]
    fn a_mesh_key_carries_its_colormap_and_opacity_decides_the_blend() {
        // Exactly `1.0`, not "at least" — the precedent is `Image`'s
        // `an_opacity_a_hair_below_one_blends`, and the reason is the same: an
        // opacity past one is clamped at draw time, so it draws as `1.0` and
        // is batched as blending all the same.
        let opaque = mesh(1.0).batch_key();
        assert_eq!(opaque.shader, ShaderKind::Mesh);
        assert_eq!(opaque.texture, Some(TextureId::new(7)));
        assert_eq!(opaque.blend_mode, BlendMode::Opaque);

        let hair_below = mesh(0.999).batch_key();
        assert_eq!(hair_below.shader, ShaderKind::Mesh);
        assert_eq!(hair_below.texture, Some(TextureId::new(7)));
        assert_eq!(hair_below.blend_mode, BlendMode::Transparent);

        let above = mesh(1.5).batch_key();
        assert_eq!(
            above.blend_mode,
            BlendMode::Transparent,
            "an opacity past one draws as one and blends"
        );
    }

    #[test]
    fn a_frame_with_no_mesh_and_no_shadow_has_no_boundary() {
        // The trailing-run literal sets all three boundary slots: a frame with
        // neither kind of boundary is one segment whose three slots are `None`.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, opaque()));
        batcher.add(rounded_at(1, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].shadow.is_none());
        assert!(segments[0].mesh.is_none());
    }

    #[test]
    fn alternating_shadows_and_meshes_yield_one_boundary_per_segment_in_order() {
        // Three boundaries — shadow, mesh, shadow — seal three segments in
        // recorded order, and the frame after the last one is the fourth.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));
        batcher.add(mesh(1.0));
        batcher.add(rounded_at(2, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(3, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 4);
        assert!(segments[0].shadow.is_some());
        assert!(segments[0].mesh.is_none());
        assert!(segments[1].shadow.is_none());
        assert!(segments[1].mesh.is_some());
        assert!(segments[2].shadow.is_some());
        assert!(segments[2].mesh.is_none());
        assert!(segments[3].shadow.is_none());
        assert!(segments[3].mesh.is_none());
        assert_eq!(
            segments[0].opaque[0].commands.len(),
            1,
            "the rect recorded before the shadow"
        );
        assert_eq!(
            segments[3].opaque[0].commands.len(),
            1,
            "and the one recorded after the last boundary is in a batch of its own"
        );
    }

    #[test]
    fn no_segment_holds_a_shadow_and_a_mesh() {
        // A seal consumes exactly one singleton command, so at most one of the
        // three boundary slots is `Some` — the invariant a three-`Option` type
        // cannot say for itself. A mesh and a shadow can never share a
        // segment, so their draw order relative to each other is unobservable.
        let mut batcher = Batcher::new();
        batcher.add(shadow());
        batcher.add(mesh(1.0));
        batcher.add(mesh(0.5));
        batcher.add(shadow());
        batcher.add(rounded_at(0, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 5);
        assert!(
            segments.iter().any(|segment| segment.mesh.is_some()),
            "a frame holding meshes has a mesh boundary — without this, a \
             classification that files every boundary as a shadow passes the \
             invariant below vacuously"
        );
        for (index, segment) in segments.iter().enumerate() {
            assert!(
                segment.shadow.is_none() || segment.mesh.is_none(),
                "segment {index} holds two boundaries"
            );
        }
    }

    #[test]
    fn every_variant_has_a_key_and_every_kind_routes() {
        // The compiler is the stronger check: one `match` over all eleven
        // `DrawCommand` variants and one over all six `ShaderKind`s, so a
        // variant added without a key — or a kind added without a route — is a
        // non-exhaustive match, which is a compile error rather than a silent
        // pass.
        let commands = [
            rect(opaque()),
            rounded_at(0, opaque()),
            shadow(),
            image(1),
            DrawCommand::Text {
                x: 0.0,
                y: 0.0,
                text: "a".to_string(),
                color: opaque(),
                font_size: 16.0,
                extra_advance: 0.0,
                family: FamilyId::default(),
                weight: FontWeight::Regular,
                fade: None,
                clip: None,
            },
            DrawCommand::Line {
                start: (0.0, 0.0),
                end: (1.0, 1.0),
                width: 1.0,
                color: opaque(),
            },
            DrawCommand::Circle {
                center: (0.0, 0.0),
                radius: 1.0,
                color: opaque(),
            },
            DrawCommand::Path {
                points: Vec::new(),
                width: 1.0,
                color: opaque(),
                closed: false,
            },
            DrawCommand::Polygon {
                points: Vec::new(),
                color: opaque(),
            },
            mesh(1.0),
        ];
        assert_eq!(commands.len(), 10, "one command per DrawCommand variant");
        for command in &commands {
            let _ = command.batch_key();
        }
        for kind in [
            ShaderKind::Solid,
            ShaderKind::Text,
            ShaderKind::Image,
            ShaderKind::Shadow,
            ShaderKind::Mesh,
        ] {
            let _ = BatchKey {
                texture: None,
                blend_mode: BlendMode::Opaque,
                shader: kind,
            }
            .is_singleton();
        }
    }

    #[test]
    fn a_shadow_keeps_the_clip_it_was_recorded_under() {
        // The composite is a draw call, and a draw call has one scissor, so a
        // shadow recorded inside a scrolling viewport has to carry that viewport
        // with it. A shadow batch with the clip dropped would composite over the
        // whole window.
        let clip = Some(Rect::new(40.0, 60.0, 300.0, 200.0));
        let mut batcher = Batcher::new();
        batcher.add_clipped(rounded_at(0, opaque()), clip);
        batcher.add_clipped(shadow(), clip);
        batcher.add_clipped(rounded_at(1, opaque()), clip);

        let segments = batcher.submit_order();
        let shadow_batch = segments[0].shadow.as_ref().expect("the shadow");
        assert_eq!(
            shadow_batch.clip, clip,
            "the shadow's own clip, not the one before or after it"
        );
        assert_eq!(
            segments[1].opaque[0].clip, clip,
            "and so does the batch after"
        );
    }

    #[test]
    fn nothing_recorded_after_a_shadow_shares_a_batch_with_something_before_it() {
        // **The defect this segmentation does not fix on its own, and the reason
        // the shadow seals the open segment rather than only being its own batch.**
        //
        // Both rects here are solid, opaque, untextured and unclipped — the
        // window's background and a dialog's panel are exactly those two things.
        // With merging by key alone they are **one batch**, a batch holds one
        // position in the recorded stream, and both of them would then be
        // submitted in the segment *before* the shadow: the panel before the
        // translucent overlay it is supposed to sit on top of, which is the
        // dimmed-panel defect arrived at by a different road.
        //
        // The control beside the count is the same pair of rects with no shadow
        // between them, which do merge — one batch of two commands — because
        // merging them is right when there is no boundary between them.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0].opaque[0].commands.len(),
            1,
            "only the rect recorded before the shadow"
        );
        assert_eq!(
            segments[1].opaque[0].commands.len(),
            1,
            "and the one recorded after it is in a batch of its own"
        );

        let mut control = Batcher::new();
        control.add(rounded_at(0, opaque()));
        control.add(rounded_at(1, opaque()));
        let segments = control.submit_order();
        assert_eq!(
            segments[0].opaque[0].commands.len(),
            2,
            "with no shadow between them the two rects merge, as they always did"
        );
    }

    #[test]
    fn a_seal_does_not_stop_commands_on_one_side_of_it_from_merging() {
        // The other side of the same rule, and the one that would be a
        // performance regression if sealing were a blunt instrument: the batching
        // task bought a screen of widgets drawn in a handful of draw calls, and a
        // shadow in one panel must not turn every widget after it into a batch of
        // its own.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));
        batcher.add(rounded_at(2, opaque()));
        batcher.add(rounded_at(3, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(
            segments[1].opaque.len(),
            1,
            "three opaque rects after the shadow are one draw call"
        );
        assert_eq!(segments[1].opaque[0].commands.len(), 3);
        assert_eq!(segments[0].opaque.len(), 1, "and one before it");
        assert_eq!(segments[0].opaque[0].commands.len(), 1);
    }

    /// A backdrop request, through the crate's own public recording path.
    fn backdrop_of() -> DrawCommand {
        let mut painter = crate::paint::Painter::new();
        painter.backdrop(
            Rect::new(240.0, 160.0, 320.0, 200.0),
            crate::paint::BackdropMode::Blur(2.0),
            Color::new(236, 239, 244, 170),
            12.0,
        );
        let commands = painter.finish();
        match commands.into_iter().next() {
            Some(command) => command,
            None => panic!("a backdrop request records one command"),
        }
    }

    #[test]
    fn a_public_backdrop_request_reaches_the_renderer_as_a_segment_boundary() {
        // **The label on this assertion is exact and the handoff must repeat it: it
        // proves the request reaches the renderer's *submission structure*, which is
        // as far as a display-free test can reach.** The GL half — whether the blit
        // is legal, whether the composite draws — is not claimed here, and no test in
        // this suite can claim it: it is the one-shot capture probe's job.
        let clip = Some(Rect::new(0.0, 64.0, 1280.0, 640.0));
        let mut batcher = Batcher::new();
        batcher.add_clipped(rounded_at(0, opaque()), clip);
        batcher.add_clipped(backdrop_of(), clip);
        batcher.add_clipped(rounded_at(1, opaque()), clip);

        let segments = batcher.submit_order();
        assert_eq!(
            segments.len(),
            2,
            "one boundary among three commands, and a trailing segment"
        );
        let boundary = segments[0]
            .backdrop
            .as_ref()
            .expect("the backdrop reached the segment as its boundary");
        assert_eq!(boundary.key.shader, ShaderKind::Backdrop);
        assert!(boundary.key.texture.is_none(), "and it samples no texture");
        assert_eq!(
            boundary.key.blend_mode,
            BlendMode::Transparent,
            "**unconditionally** — the next test is why"
        );
        assert_eq!(
            boundary.clip, clip,
            "**the clip it was recorded under**, because a composite is a draw call \
             and a draw call has one scissor — `Segment::shadow`'s recorded reason"
        );
        assert_eq!(boundary.commands.len(), 1, "and it holds one command");

        // **The property the singleton exists for**: a command recorded after the
        // backdrop lands in a later segment, so it is over the backdrop rather than
        // merged into a batch recorded before it.
        assert_eq!(
            segments[1].opaque.len(),
            1,
            "the command recorded after the backdrop is in a later segment"
        );
        assert_eq!(segments[1].opaque[0].commands.len(), 1);
        assert!(segments[0].shadow.is_none(), "and it is not also a shadow");
        assert!(segments[0].mesh.is_none(), "nor a mesh");
    }

    #[test]
    fn a_backdrop_batches_transparent_even_when_its_tint_is_opaque() {
        // **The control is `BlendMode::from_color` on the same tint**, which says
        // `Opaque` and is wrong here. The capture's alpha is the framebuffer's
        // accumulated coverage, and a translucent primitive recorded before the
        // backdrop left that below 1.0 somewhere; an `Opaque` batch overwrites it
        // with `src` and puts a hard edge exactly where a blur has none.
        let as_captured = DrawCommand::Backdrop {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            mode: crate::paint::BackdropMode::Sharp,
            tint: Color::new(255, 255, 255, 255),
            radius: 0.0,
        };
        assert_eq!(
            BlendMode::from_color(Color::new(255, 255, 255, 255)),
            BlendMode::Opaque,
            "the rule this arm declines would answer `Opaque` for this tint"
        );
        assert_eq!(
            as_captured.batch_key().blend_mode,
            BlendMode::Transparent,
            "**and the answer is `Transparent` anyway** — as captured is a blend, \
             not a cover"
        );
        assert_eq!(as_captured.batch_key().shader, ShaderKind::Backdrop);
        assert!(as_captured.batch_key().is_singleton());

        // The translucent case, so the arm is not passing on one value by accident.
        let frost = DrawCommand::Backdrop {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            mode: crate::paint::BackdropMode::Blur(2.0),
            tint: Color::new(236, 239, 244, 170),
            radius: 12.0,
        };
        assert_eq!(frost.batch_key().blend_mode, BlendMode::Transparent);
    }

    #[test]
    fn no_segment_has_two_boundaries_filled() {
        // **Task 37's invariant, over three slots now.** A seal consumes exactly one
        // singleton command, so at most one of `shadow`, `mesh` and `backdrop` is
        // `Some` in any segment — the invariant a three-`Option` type cannot say for
        // itself.
        let mut batcher = Batcher::new();
        // A frame that alternates all three boundary kinds, so the control below
        // cannot pass by every boundary being classified as one of them.
        batcher.add(rounded_at(0, opaque()));
        batcher.add(shadow());
        batcher.add(rounded_at(1, opaque()));
        batcher.add(backdrop_of());
        batcher.add(rounded_at(2, opaque()));
        batcher.add(mesh(1.0));
        batcher.add(rounded_at(3, opaque()));
        batcher.add(backdrop_of());
        batcher.add(rounded_at(4, opaque()));

        let segments = batcher.submit_order();
        let filled = |segment: &Segment| {
            usize::from(segment.shadow.is_some())
                + usize::from(segment.mesh.is_some())
                + usize::from(segment.backdrop.is_some())
        };
        let boundaries = segments.iter().map(filled).sum::<usize>();
        assert_eq!(boundaries, 4, "four boundaries across five segments");
        assert_eq!(segments.len(), 5, "and the trailing one after the last");
        // **The vacuity control**, without which a classifier that filed every
        // boundary as a shadow would pass the invariant above: all three slots are
        // reached.
        assert!(
            segments.iter().any(|segment| segment.backdrop.is_some()),
            "the fixture records backdrops — without this line a classification \
             that never fills this slot passes the invariant vacuously"
        );
        assert!(segments.iter().any(|segment| segment.mesh.is_some()));
        assert!(segments.iter().any(|segment| segment.shadow.is_some()));
        for (index, segment) in segments.iter().enumerate() {
            assert!(
                filled(segment) <= 1,
                "segment {index} holds {} boundaries",
                filled(segment)
            );
        }
    }

    #[test]
    fn a_backdrop_key_is_a_singleton_and_its_neighbours_still_are_not() {
        // The third singleton, with **both pre-existing controls kept present and
        // unweakened** — a `matches!` arm that named the backdrop *instead of* adding
        // it would make every command a boundary, and these two lines are what catch
        // that.
        assert!(backdrop_of().batch_key().is_singleton());
        assert!(shadow().batch_key().is_singleton());
        assert!(mesh(1.0).batch_key().is_singleton());

        assert!(!rect(opaque()).batch_key().is_singleton());
        assert!(!rounded_at(0, opaque()).batch_key().is_singleton());
        assert!(!image(1).batch_key().is_singleton());
    }

    #[test]
    fn every_variant_has_a_key_and_every_kind_routes_including_the_backdrop() {
        // **The compiler is the stronger check, and this is where it earns it**: one
        // `match` over all eleven `DrawCommand` variants and one over all six
        // `ShaderKind`s, so a variant added without a key — or a kind added without a
        // route — is a non-exhaustive match, which is a compile error rather than a
        // silent pass. **The count is asserted**, because a fixture that grew a
        // duplicate would otherwise make the array longer than the enum and the
        // assertion vacuous.
        let commands = [
            rect(opaque()),
            rounded_at(0, opaque()),
            shadow(),
            image(1),
            backdrop_of(),
            DrawCommand::Text {
                x: 0.0,
                y: 0.0,
                text: "a".to_string(),
                color: opaque(),
                font_size: 16.0,
                extra_advance: 0.0,
                family: FamilyId::default(),
                weight: FontWeight::Regular,
                fade: None,
                clip: None,
            },
            DrawCommand::Line {
                start: (0.0, 0.0),
                end: (1.0, 1.0),
                width: 1.0,
                color: opaque(),
            },
            DrawCommand::Circle {
                center: (0.0, 0.0),
                radius: 1.0,
                color: opaque(),
            },
            DrawCommand::Path {
                points: Vec::new(),
                width: 1.0,
                color: opaque(),
                closed: false,
            },
            DrawCommand::Polygon {
                points: Vec::new(),
                color: opaque(),
            },
            mesh(1.0),
        ];
        assert_eq!(commands.len(), 11, "one command per DrawCommand variant");
        for command in &commands {
            let _ = command.batch_key();
        }
        for kind in [
            ShaderKind::Solid,
            ShaderKind::Text,
            ShaderKind::Image,
            ShaderKind::Shadow,
            ShaderKind::Mesh,
            ShaderKind::Backdrop,
        ] {
            let _ = BatchKey {
                texture: None,
                blend_mode: BlendMode::Opaque,
                shader: kind,
            }
            .is_singleton();
        }
    }

    #[test]
    fn a_frame_with_no_mesh_shadow_or_backdrop_has_no_boundary() {
        // **The trailing-run literal sets all three slots.** A frame with no boundary
        // of any kind is one segment whose three slots are `None` — and a literal
        // that set only two would leave the third as whatever the previous frame put
        // there, which is the lifecycle leak task 24.1 found once already.
        let mut batcher = Batcher::new();
        batcher.add(rounded_at(0, opaque()));
        batcher.add(rounded_at(1, opaque()));

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].shadow.is_none());
        assert!(segments[0].mesh.is_none());
        assert!(segments[0].backdrop.is_none());
    }

    #[test]
    fn two_backdrops_of_the_same_material_are_still_two_boundaries() {
        // The merge this has to refuse, and it is the one the shadow's own test
        // established: two backdrops with the same rect, tint and mode share a key,
        // so the rule that merges everything else merges them into one batch of two —
        // and one batch has one position in the recorded stream, so the boundary
        // between them would be gone and the first composite would land after the
        // second.
        let mut batcher = Batcher::new();
        batcher.add(backdrop_of());
        batcher.add(backdrop_of());

        let segments = batcher.submit_order();
        assert_eq!(segments.len(), 3, "two backdrops are two boundaries");
        for segment in &segments[..2] {
            let boundary = segment.backdrop.as_ref().expect("a backdrop of its own");
            assert_eq!(
                boundary.commands.len(),
                1,
                "and each backdrop's batch holds one command, not two"
            );
        }
    }

    #[test]
    fn a_backdrop_keeps_the_clip_it_was_recorded_under() {
        // **The composite is a draw call and a draw call has one scissor**, so a
        // backdrop recorded inside a scrolling viewport carries that viewport with
        // it. The *effective* clip is what rides: the batcher intersects the caller's
        // clip with the command's own, and a backdrop carries none of its own, so the
        // caller's is the whole answer.
        let viewport = Some(Rect::new(0.0, 64.0, 1280.0, 600.0));
        let mut batcher = Batcher::new();
        batcher.add_clipped(rounded_at(0, opaque()), viewport);
        batcher.add_clipped(backdrop_of(), viewport);
        batcher.add_clipped(rounded_at(1, opaque()), viewport);

        let segments = batcher.submit_order();
        assert_eq!(
            segments[0].backdrop.as_ref().expect("the backdrop").clip,
            viewport,
            "the viewport, not the window and not the command's own rect"
        );
        assert_eq!(
            segments[1].opaque[0].clip, viewport,
            "and so does the batch after it"
        );
    }
}
