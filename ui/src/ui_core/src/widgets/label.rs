//! The Label widget: text layout and the widget node.
//!
//! This module owns the parts of text rendering that need no font: wrapping,
//! alignment, line height, letter spacing and truncation, plus the [`Label`]
//! widget node and its properties. The width of a line is measured through an
//! advance-width callback, so the same layout code serves any font and is
//! testable without one.
//!
//! [`crate::font`] supplies the advances and the glyphs: FreeType rasterises
//! each glyph and the SDF atlas packs them, and [`Label::paint`] turns a laid
//! out label into one draw command per line for the text shader.
//!
//! Two parts of the task's pipeline are not here. Shaping — ligatures,
//! complex scripts, bidirectional text — needs HarfBuzz, whose safe Rust
//! binding exposes no shaping API, and the operator declined the `unsafe` it
//! would take (see `doc/ui/IMPLEMENTATION_STATE.md`). Dynamic atlas growth is not
//! built either: an atlas full of glyphs larger than the one being packed evicts
//! a shelf and, failing that, drops the glyph.
//!
//! **The font fallback chain is in [`crate::font`] and this module reaches it twice
//!**: once through `font_family`, which the recorded command carries so the
//! renderer can walk the chain per character, and once through the `advance`
//! callback the caller supplies, which has to ask the same chain or the words will
//! not land under their glyphs.
//!
//! A character no font in the chain covers is drawn as the replacement box, and it
//! is measured at the width that box is drawn at, from one function. **"Never a
//! silent hole" is a statement about the chain and not about every state a label
//! can be in**: a label whose family names a set with **no font at all** is drawn
//! as nothing, because there is no face to draw it with — that is
//! [`FontSet::primary`](crate::font::FontSet::primary) answering `None`, and the
//! renderer skipping the run. An empty *family* is not that state: it resolves to
//! the default family (see [`FontSet`](crate::font::FontSet)'s docs on
//! `drawable`), which is a fix the review of task 30 required after this sentence
//! claimed otherwise.

use crate::arena::{Arena, Handle};
use crate::font::{FamilyId, FontWeight};
use crate::layout::LayoutState;
use crate::node::{self, WidgetNode};
use crate::property::{Color, Property};

/// How a line of text is aligned within its container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    /// The line starts at the container's left edge.
    Left,
    /// The line is centred in the container.
    Center,
    /// The line ends at the container's right edge.
    Right,
    /// The line's words are spread to fill the container's width. The last
    /// line of a paragraph is never justified.
    Justify,
}

/// How text is wrapped onto multiple lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapMode {
    /// No wrapping: each paragraph is one line, however long.
    None,
    /// Break at word boundaries. A single word longer than the line
    /// overflows it rather than being broken.
    Word,
    /// Break at any character once the line is full.
    Character,
}

/// How text that does not fit its container is truncated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truncation {
    /// No truncation: text overflows the container.
    None,
    /// Cut at the container's edge, and every glyph past it is cut away by a
    /// scissor.
    ///
    /// **Two cuts, and both are needed.** [`layout_text`] drops whole
    /// characters past `max_width`, which is a cut of *text*: no glyph starts
    /// where the box ends. A glyph whose ink overhangs its own advance — which is
    /// a fact about the font's bearings and not about the layout — is left
    /// standing past the edge, and [`Label::paint`] records the box on each
    /// command so the GPU cuts it. The command's `clip` is the box;
    /// `Batcher::add_clipped` intersects it with whatever clip the command's own
    /// caller recorded under, and `Renderer::apply_clip` sets the scissor
    /// between draw calls.
    Clip,
    /// Cut at the container's edge and append an ellipsis (`…`).
    Ellipsis,
    /// Cut at the container's edge, then ramped to nothing over the last
    /// [`FADE_WIDTH_EM`] of the drawn run.
    ///
    /// **The layout cut is [`Truncation::Clip`]'s**, because a run of text has
    /// to stop somewhere and dropping whole characters is the only cut the
    /// advance widths can measure. **The ramp is applied at draw time, per
    /// corner**: each glyph quad's four corners are scaled by their own x
    /// against the window [`Label::paint`] records, so the fade is smooth inside
    /// a glyph rather than stepped at glyph boundaries, and `TextVertex` stays 32
    /// bytes with no shader change. [`FadeRamp`](crate::paint::FadeRamp) is where
    /// that choice and its two rejected alternatives are argued.
    ///
    /// **Every truncated line ramps, at its own cut edge** — the operator decided
    /// this on 2026-10-06, and it is not the last line only: a paragraph cut
    /// across three lines has three cut edges and three windows.
    ///
    /// **The window is at the drawn run's far end, not at the container's edge.**
    /// Under [`TextAlign::Right`] or [`TextAlign::Center`] the text ends
    /// somewhere other than `max_width`, and it is the text's own end that is the
    /// cut.
    Fade,
}

/// The inputs [`layout_text`] needs beyond the text itself.
///
/// `max_width` is the only field with no useful default: it is the container's
/// inner width, and wrapping and alignment are both measured against it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutOptions {
    /// The width a line may not exceed, in pixels. Set this to the
    /// container's inner width.
    pub max_width: f32,
    /// The height the laid-out text may not exceed, in pixels. `None` means
    /// no vertical limit.
    pub max_height: Option<f32>,
    /// The height of one line, in pixels. Must be positive for `max_height`
    /// to have an effect.
    pub line_height: f32,
    /// Extra pixels between two consecutive characters, in pixels.
    pub letter_spacing: f32,
    /// How each line is aligned.
    pub align: TextAlign,
    /// How text wraps onto multiple lines.
    pub wrap: WrapMode,
    /// How text that does not fit is truncated.
    pub truncation: Truncation,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            max_width: f32::INFINITY,
            max_height: None,
            line_height: 16.0,
            letter_spacing: 0.0,
            align: TextAlign::Left,
            wrap: WrapMode::Word,
            truncation: Truncation::Ellipsis,
        }
    }
}

/// One laid-out line.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// The line's text after wrapping and truncation.
    pub text: String,
    /// The line's width in pixels, including letter spacing but excluding
    /// the extra space justification adds between words.
    pub width: f32,
    /// The line's left offset within the container, from alignment.
    pub x_offset: f32,
    /// Extra pixels between two words on this line, beyond letter spacing.
    /// Zero unless the line is justified.
    pub word_gap: f32,
    /// Whether **this line** had text cut from it.
    ///
    /// **Per line, and not [`TextLayout::truncated`], which is the whole layout.**
    /// That flag folds every line's answer together with an `||`, so it cannot
    /// say *which* line was cut, and a fade needs to know: the operator's policy
    /// is that **every** truncated line ramps at its own cut edge, so a layout
    /// whose first of three lines was cut and whose other two fit produces two
    /// un-ramped runs and one ramped one — a question only this field can answer.
    ///
    /// **It is `true` for a line cut by the vertical limit as well as by its own
    /// width**, which is `truncate_line`'s own answer: a line the height cut
    /// off is truncated for the purposes of every caller of this flag, and a fade
    /// at the end of the last visible line is the one place a vertical cut wants
    /// one. The layout's `truncated` answers the same question the same way, so
    /// this field is the per-line half of that one answer rather than a second
    /// rule about it.
    pub truncated: bool,
}

/// The result of laying a text out.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLayout {
    /// The laid-out lines, top to bottom.
    pub lines: Vec<Line>,
    /// The total height of the laid-out text: the line count times the line
    /// height.
    pub total_height: f32,
    /// Whether any text was cut to fit the container.
    ///
    /// **An `||` over every [`Line::truncated`], plus the vertical cut**, so it
    /// answers "did anything at all go missing" and not "which line lost
    /// something". A caller that draws has the second question and reads the
    /// first.
    pub truncated: bool,
}

/// Lays `text` out into lines that fit `options`, measuring each character's
/// advance with `advance`.
///
/// Explicit newlines split the text into paragraphs, each wrapped on its own.
/// The `advance` callback is the seam the rendering pipeline plugs into: it
/// supplies the width of one character in the label's font, and everything
/// else — wrapping, alignment, truncation — follows from it.
///
/// # Examples
///
/// ```
/// use ui_core::widgets::label::{layout_text, LayoutOptions, WrapMode};
///
/// // A monospace advance of 5 pixels per character.
/// let options = LayoutOptions {
///     max_width: 30.0,
///     wrap: WrapMode::Word,
///     ..LayoutOptions::default()
/// };
/// let layout = layout_text("hello world", &options, &|_: char| 5.0);
///
/// let texts: Vec<&str> = layout.lines.iter().map(|l| l.text.as_str()).collect();
/// assert_eq!(texts, vec!["hello", "world"]);
/// ```
pub fn layout_text(
    text: &str,
    options: &LayoutOptions,
    advance: &dyn Fn(char) -> f32,
) -> TextLayout {
    let paragraphs: Vec<&str> = text.split('\n').collect();

    // Wrap each paragraph, tracking which lines close their paragraph: the
    // last line of a paragraph is never justified.
    let mut wrapped: Vec<(String, bool)> = Vec::new();
    for (p_idx, paragraph) in paragraphs.iter().enumerate() {
        let mut lines = wrap_paragraph(paragraph, options, advance);
        let is_last_paragraph = p_idx + 1 == paragraphs.len();
        let count = lines.len();
        for (l_idx, line) in lines.drain(..).enumerate() {
            let is_last = is_last_paragraph && l_idx + 1 == count;
            wrapped.push((line, is_last));
        }
    }

    // Vertical truncation: keep the lines that fit in `max_height`. The
    // accumulation avoids a float-to-integer conversion, which std gives no
    // `TryFrom` for.
    let mut truncated = false;
    let mut kept: Vec<(String, bool)> = Vec::new();
    let vertical_limit = options.max_height.filter(|_| options.line_height > 0.0);
    if let Some(max_height) = vertical_limit {
        let mut height = 0.0;
        for (line, is_last) in wrapped {
            if height + options.line_height > max_height {
                truncated = true;
                break;
            }
            height += options.line_height;
            kept.push((line, is_last));
        }
    } else {
        kept = wrapped;
    }

    // A line cut by the vertical limit is a visual last line even when it was
    // not its paragraph's last: it is left-aligned and, in ellipsis mode,
    // marked with an ellipsis.
    if truncated {
        if let Some(last) = kept.last_mut() {
            last.1 = true;
        }
    }

    let kept_len = kept.len();
    let mut lines = Vec::new();
    let mut total_height = 0.0;
    for (idx, (text, is_last)) in kept.into_iter().enumerate() {
        let force_ellipsis =
            truncated && idx + 1 == kept_len && options.truncation == Truncation::Ellipsis;
        let (text, line_truncated) = truncate_line(&text, options, advance, force_ellipsis);
        truncated = truncated || line_truncated;
        let width = measure(&text, options.letter_spacing, advance);
        let word_count = text.split_whitespace().count();
        let (x_offset, word_gap) = align_line(width, options, is_last, word_count);
        lines.push(Line {
            text,
            width,
            x_offset,
            word_gap,
            truncated: line_truncated,
        });
        total_height += options.line_height;
    }

    TextLayout {
        lines,
        total_height,
        truncated,
    }
}

/// Wraps one paragraph (text without newlines) into lines.
fn wrap_paragraph(
    paragraph: &str,
    options: &LayoutOptions,
    advance: &dyn Fn(char) -> f32,
) -> Vec<String> {
    match options.wrap {
        WrapMode::None => vec![paragraph.to_string()],
        WrapMode::Word => wrap_word(paragraph, options, advance),
        WrapMode::Character => wrap_character(paragraph, options, advance),
    }
}

/// Wraps at word boundaries, packing words onto a line until the next would
/// overflow it. A word longer than the line gets its own line and overflows
/// it, rather than being broken.
fn wrap_word(
    paragraph: &str,
    options: &LayoutOptions,
    advance: &dyn Fn(char) -> f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0.0;
    for word in paragraph.split_whitespace() {
        let word_width = measure(word, options.letter_spacing, advance);
        if current.is_empty() {
            current.push_str(word);
            current_width = word_width;
            continue;
        }
        let space_width = measure(" ", options.letter_spacing, advance);
        let candidate = current_width + space_width + word_width;
        if candidate <= options.max_width {
            current.push(' ');
            current.push_str(word);
            current_width = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
            current_width = word_width;
        }
    }
    lines.push(current);
    lines
}

/// Wraps at any character once the line is full, breaking words mid-glyph.
fn wrap_character(
    paragraph: &str,
    options: &LayoutOptions,
    advance: &dyn Fn(char) -> f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0.0;
    for ch in paragraph.chars() {
        let ch_width = advance(ch) + options.letter_spacing;
        if !current.is_empty() && current_width + ch_width > options.max_width {
            lines.push(std::mem::take(&mut current));
            current_width = 0.0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    lines.push(current);
    lines
}

/// Measures a string's width: the sum of its characters' advances plus the
/// letter spacing after each one.
fn measure(text: &str, letter_spacing: f32, advance: &dyn Fn(char) -> f32) -> f32 {
    let mut width = 0.0;
    for ch in text.chars() {
        width += advance(ch) + letter_spacing;
    }
    width
}

/// Returns the longest prefix of `text` whose width does not exceed `budget`.
fn fit(text: &str, budget: f32, letter_spacing: f32, advance: &dyn Fn(char) -> f32) -> String {
    let mut fitted = String::new();
    let mut width = 0.0;
    for ch in text.chars() {
        let ch_width = advance(ch) + letter_spacing;
        if width + ch_width > budget {
            break;
        }
        fitted.push(ch);
        width += ch_width;
    }
    fitted
}

/// Cuts a line to the container's width, returning the cut text and whether
/// anything was removed.
///
/// `force_ellipsis` marks a line that was cut by the vertical limit rather
/// than its own width: it gets an ellipsis even when it would have fit.
fn truncate_line(
    line: &str,
    options: &LayoutOptions,
    advance: &dyn Fn(char) -> f32,
    force_ellipsis: bool,
) -> (String, bool) {
    let width = measure(line, options.letter_spacing, advance);
    if width <= options.max_width && !force_ellipsis {
        return (line.to_string(), false);
    }
    match options.truncation {
        Truncation::None => (line.to_string(), false),
        Truncation::Clip | Truncation::Fade => (
            fit(line, options.max_width, options.letter_spacing, advance),
            true,
        ),
        Truncation::Ellipsis => {
            let ellipsis = "…";
            let ellipsis_width = measure(ellipsis, options.letter_spacing, advance);
            let budget = (options.max_width - ellipsis_width).max(0.0);
            let mut text = fit(line, budget, options.letter_spacing, advance);
            text.push_str(ellipsis);
            (text, true)
        }
    }
}

/// Returns a line's left offset and the extra space justification adds
/// between its words.
fn align_line(
    line_width: f32,
    options: &LayoutOptions,
    is_last: bool,
    word_count: usize,
) -> (f32, f32) {
    match options.align {
        TextAlign::Left => (0.0, 0.0),
        TextAlign::Center => {
            let offset = (options.max_width - line_width) / 2.0;
            (offset.max(0.0), 0.0)
        }
        TextAlign::Right => {
            let offset = options.max_width - line_width;
            (offset.max(0.0), 0.0)
        }
        TextAlign::Justify => {
            if is_last || word_count < 2 {
                (0.0, 0.0)
            } else {
                // There is no `From`/`TryFrom` between `usize` and `f32` in
                // std, and a word count is a small non-negative integer that
                // `f32` represents exactly, so this is the same justified `as`
                // cast the demo's `f32_to_u32` makes for the opposite
                // direction.
                let gaps = (word_count - 1) as f32;
                let extra = (options.max_width - line_width) / gaps;
                (0.0, extra.max(0.0))
            }
        }
    }
}

/// A label: a widget that displays text.
///
/// The label holds the four properties the task gives it and a node in the
/// arena. [`Label::layout`] lays the text out and [`Label::paint`] records it
/// as draw commands; the font is the caller's, passed in as the advance
/// callback, so the label itself loads no font and needs none to be tested.
///
/// The link from a property change to the arena is the caller's, the same
/// way the demo wires its pads: a property's `on_change` callback marks the
/// node dirty.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::widgets::label::{Label, LayoutOptions};
///
/// let mut nodes = Arena::new();
/// let label = Label::new(&mut nodes, "Hello");
/// assert_eq!(label.text.get(), "Hello");
/// assert!(nodes.get(label.handle()).is_some());
///
/// let layout = label.layout(&LayoutOptions::default(), &|_: char| 5.0);
/// assert_eq!(layout.lines.len(), 1);
/// assert_eq!(layout.lines[0].text, "Hello");
/// ```
pub struct Label {
    /// The text to display.
    pub text: Property<String>,
    /// The font size in pixels.
    pub font_size: Property<f32>,
    /// The text colour.
    pub color: Property<Color>,
    /// The font family the label's text is drawn in, as a handle into the
    /// [`FontSet`](crate::font::FontSet) the caller laid it out with.
    ///
    /// **A handle rather than a name**, and the reason is where the name is
    /// resolved. A family is a chain of fonts tried in order per character, so
    /// naming it in a property would mean resolving it once per line per frame —
    /// which is a lookup in the set, from a `String` the widget would then have to
    /// clone into every command it records. [`FontSet::family`](crate::font::FontSet::family)
    /// turns the name into a handle once, when the family is defined, and the
    /// handle is what rides here and on `DrawCommand::Text`.
    ///
    /// The default is [`FamilyId::default`], which is the set's default family, so
    /// a label that has never been told about families draws in it — which is what
    /// a caller with no families at all wants and what keeps every existing caller
    /// of `Label::new` drawing exactly what it drew before.
    pub font_family: Property<FamilyId>,
    node: Handle,
}

impl Label {
    /// Creates a label showing `text` in the arena, and returns it.
    ///
    /// The label's node starts dirty, so the next layout pass places it.
    pub fn new(nodes: &mut Arena<WidgetNode>, text: impl Into<String>) -> Self {
        let node = node::create(nodes, LayoutState::new());
        Label {
            text: Property::new(text.into()),
            font_size: Property::new(16.0),
            color: Property::new(Color::new(0, 0, 0, 255)),
            font_family: Property::new(FamilyId::default()),
            node,
        }
    }

    /// Returns the label's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the label's size for its current text and font size: the text's
    /// width and one line of height.
    ///
    /// The caller turns this into the node's constraints, so the layout pass
    /// places the label at the size its text needs.
    pub fn size(&self, font: &crate::font::Font) -> crate::layout::Size {
        let size = self.font_size.get();
        let width = font.measure(&self.text.get(), size);
        crate::layout::Size::new(width, size)
    }

    /// Lays the label's current text out with `options`, measuring each
    /// character's advance with `advance`.
    ///
    /// This is the seam the rendering pipeline plugs into: it produces the
    /// lines, their offsets and their truncation, and the renderer turns them
    /// into draw commands.
    pub fn layout(&self, options: &LayoutOptions, advance: &dyn Fn(char) -> f32) -> TextLayout {
        layout_text(&self.text.get(), options, advance)
    }

    /// Returns the draw commands that paint the label within `rect`, measuring
    /// each character's advance with `advance`.
    ///
    /// One command per laid-out line, each on its own line box: the line's
    /// vertical offset is `rect.y` plus its index times `options.line_height`,
    /// and its horizontal offset is the one [`Label::layout`] computed, so
    /// wrapping and alignment are what is actually drawn. A justified line is
    /// recorded word by word, because its extra gap belongs between words and
    /// not after every glyph.
    ///
    /// **The layout is run here rather than handed in**, and that is why the
    /// fade is computed here too: the ramp's far end is the drawn run's end,
    /// which is a number this method produces and no earlier layer has. A caller
    /// that laid the label out itself cannot hand the answer over without this
    /// method taking a second way of getting the same lines.
    ///
    /// **The `advance` callback and the recorded family must be the same
    /// chain**, and the caller is the only thing that can see both: this method
    /// knows the family (from the property) and is handed the advances (from
    /// `advance`), and a run laid out with one chain's advances and drawn in
    /// another's is a line whose words do not land under their glyphs. It says so
    /// rather than checking, because a check would mean a second copy of the
    /// chain's rule — see [`FontSet::advance`](crate::font::FontSet::advance),
    /// which is the one function a caller asks for both.
    pub fn paint(
        &self,
        rect: crate::paint::Rect,
        options: &LayoutOptions,
        advance: &dyn Fn(char) -> f32,
    ) -> Vec<crate::paint::DrawCommand> {
        let layout = self.layout(options, advance);
        let font_size = self.font_size.get();
        let color = self.color.get();
        let family = self.font_family.get();
        let space = measure(" ", options.letter_spacing, advance);
        let clip = clip_for(rect, options, layout.total_height);
        let fading = options.truncation == Truncation::Fade;
        let mut painter = crate::paint::Painter::new();
        let mut top = rect.y;
        for line in &layout.lines {
            let left = rect.x + line.x_offset;
            if line.word_gap > 0.0 {
                // **The words are placed first and recorded second, and that is
                // the only reason this branch has two loops.** Every command of a
                // justified line carries one ramp, and the ramp's far end is where
                // the last word ends — a number no word knows until the last one
                // has been measured. Measuring them into a buffer first is what
                // keeps the window and the word positions derived from the *same*
                // pass; the alternative, a second expression for where the run
                // ends, is the one place the two could disagree and the tests
                // would be reading the wrong one.
                let mut words: Vec<(f32, &str)> = Vec::new();
                let mut x = left;
                for (index, word) in line.text.split_whitespace().enumerate() {
                    if index > 0 {
                        x += space + line.word_gap;
                    }
                    words.push((x, word));
                    x += measure(word, options.letter_spacing, advance);
                }
                let fade = ramp_for(line, fading, left, x, font_size);
                for (word_x, word) in words {
                    painter.text_run(crate::paint::TextRun {
                        family,
                        x: word_x,
                        y: top,
                        text: word,
                        color,
                        font_size,
                        extra_advance: options.letter_spacing,
                        // **Named, not taken from `Default::default()`.** The
                        // `..` fills `weight` in, and a `Default` that changed from
                        // `Regular` would silently draw every label in the
                        // application bold — which a `Label` property says nothing
                        // about, because it has no weight property.
                        weight: FontWeight::Regular,
                        fade,
                        clip,
                    });
                }
            } else {
                // `left + line.width` is where the pen ends, and not an estimate
                // of it: `measure` is the sum this loop and `render::walk_run`
                // both add, one `advance(ch) + extra_advance` per character
                // including the last. So the window's far end is the run's own
                // end under every alignment, which is what requirement 2 asks
                // for — with `Right` and `Center` this is well short of
                // `rect.x + max_width`, and the overflow is at *this* end.
                let fade = ramp_for(line, fading, left, left + line.width, font_size);
                painter.text_run(crate::paint::TextRun {
                    family,
                    x: left,
                    y: top,
                    text: &line.text,
                    color,
                    font_size,
                    extra_advance: options.letter_spacing,
                    // Named rather than defaulted — see the note in the branch above.
                    weight: FontWeight::Regular,
                    fade,
                    clip,
                });
            }
            top += options.line_height;
        }
        painter.finish()
    }
}

/// The truncation fade's width, in ems of the run's own font size.
///
/// **Two, and it is pinned by a test** because nothing can prove a chosen
/// constant is the right one — the number is a decision, not a measurement, and
/// the one thing a reader can check is that it has not moved.
///
/// The reasoning it rests on is about the text rather than about a number in
/// pixels: a ramp narrower than the text's own glyph advance quantises into
/// steps, because it only has room to move one glyph's alpha and a reader sees a
/// band rather than a fade; and a ramp wide enough to swallow a whole short line
/// stops reading as a fade at all and starts reading as a wrong opacity. **Two
/// ems is between those**, and it scales with the text because an em does — a
/// constant in pixels would be a legible fade at 10 px and an illegible one at
/// 64, and the demo's `+`/`-` moves the size by 2 px at a time.
///
/// Change it with a recorded reason, the way this paragraph is the reason for
/// the number above.
pub const FADE_WIDTH_EM: f32 = 2.0;

/// The ramp a laid-out line fades out over, or `None` for a run drawn flat.
///
/// **Two conditions, and they are not one.** `truncated` is the line's own
/// [`Line::truncated`]: a line that fit has no cut edge to fade at, and fading
/// one anyway would dim a paragraph for no reason. `fading` is the label's mode:
/// the other three modes do not fade, and a ramp recorded under them would be
/// applied by the renderer whether or not anything asked for it.
///
/// **`run_end` is where the drawn run ends, not where the container ends.** Every
/// branch of [`align_line`] floors its offset with `.max(0.0)`, so an overlong
/// line under `Right` or `Center` is drawn from `rect.x` — left-aligned in
/// practice — and its run ends at `rect.x + line.width`, which is *shorter* than
/// `rect.x + max_width` by the width the cut removed. That arithmetic is the
/// reason the window is the run's and not the container's.
fn ramp_for(
    line: &Line,
    fading: bool,
    left: f32,
    run_end: f32,
    font_size: f32,
) -> Option<crate::paint::FadeRamp> {
    if !fading || !line.truncated {
        return None;
    }
    Some(crate::paint::FadeRamp::to_run_end(
        left,
        run_end,
        FADE_WIDTH_EM * font_size,
    ))
}

/// The box a truncating label's commands are clipped to, or `None`.
///
/// **`None` for every mode but [`Truncation::Clip`]**, which is the only one
/// whose name is a claim about a paint-time cut. An ellipsis's `…` and a fade's
/// ramp are both about the text itself, and recording a scissor for them would
/// be a state change per run for nothing.
///
/// **Narrowed to `max_width`, and the `.min` is load-bearing in both
/// directions.** `LayoutOptions::default` has `max_width` of `f32::INFINITY`, and
/// a label that is *not* truncated by width still gets a clip: the `.min` is what
/// makes that the label's own box rather than an unbounded one. And a caller
/// whose `rect` is narrower than its own `max_width` — a node given a tight
/// constraint by the layout pass — is clipped to the node, because the node's box
/// is the smaller claim and the label does not get to draw outside it.
///
/// **`total_height` widens the box downwards, and that is a correction rather
/// than a fudge.** [`Label::size`] answers **a font size** for the height and a
/// line box is a *line height* — the demo's `metrics.line_height(24.0)` — so a
/// node given the label's own size is **shorter than the text it holds**, and a
/// descender is what lives in the difference. Clipping to `rect.height` alone
/// would cut the tail off a `g`, `p` or `y` on a `Truncation::Clip` label at
/// every size, and `Truncation::Clip` claims to cut overflow and nothing else.
///
/// **Measured at 64 px, not asserted about the demo.** At the demo's 24 px the
/// node's box happens to cover this sentence's ink exactly — 23 rows of ink in a
/// 24-tall box — so the difference is invisible on screen and the claim above is
/// arithmetic over [`Label::size`] and `options.line_height`.
/// `a_clipping_labels_box_covers_the_text_it_draws` holds it down where the gap
/// is 14 pixels rather than 5.
fn clip_for(
    rect: crate::paint::Rect,
    options: &LayoutOptions,
    total_height: f32,
) -> Option<crate::paint::Rect> {
    if options.truncation != Truncation::Clip {
        return None;
    }
    Some(crate::paint::Rect::new(
        rect.x,
        rect.y,
        options.max_width.min(rect.width),
        rect.height.max(total_height),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FontSet;

    /// A monospace advance of 5 pixels per character, spaces included.
    fn mono(_: char) -> f32 {
        5.0
    }

    /// Options with `max_width` set and everything else default.
    fn options(max_width: f32) -> LayoutOptions {
        LayoutOptions {
            max_width,
            ..LayoutOptions::default()
        }
    }

    #[test]
    fn no_wrap_keeps_a_paragraph_on_one_line() {
        let layout = layout_text("hello world", &options(100.0), &mono);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, "hello world");
        assert!(!layout.truncated);
    }

    #[test]
    fn word_wrap_breaks_at_word_boundaries() {
        // "hello" is 25, the space is 5, "world" is 25: "hello world" is 55,
        // which does not fit in 30, so the words split.
        let layout = layout_text("hello world foo", &options(30.0), &mono);
        let texts: Vec<&str> = layout.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["hello", "world", "foo"]);
        assert!(!layout.truncated);
    }

    #[test]
    fn word_wrap_packs_what_fits() {
        // "hello world" is exactly 55, which fits.
        let layout = layout_text("hello world foo", &options(55.0), &mono);
        let texts: Vec<&str> = layout.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["hello world", "foo"]);
    }

    #[test]
    fn word_wrap_leaves_an_overlong_word_on_its_own_line() {
        // Truncation is off: the point is that the word is not broken, not
        // that it is cut to the container.
        let mut opts = options(30.0);
        opts.truncation = Truncation::None;
        let layout = layout_text("hi supercalifragilistic", &opts, &mono);
        let texts: Vec<&str> = layout.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["hi", "supercalifragilistic"]);
        assert!(
            layout.lines[1].width > 30.0,
            "an overlong word overflows rather than being broken"
        );
    }

    #[test]
    fn character_wrap_breaks_mid_word() {
        // Each character is 5 pixels: "he" is 10, "ll" is 10, "o" is 5.
        let mut opts = options(12.0);
        opts.wrap = WrapMode::Character;
        let layout = layout_text("hello", &opts, &mono);
        let texts: Vec<&str> = layout.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["he", "ll", "o"]);
    }

    #[test]
    fn explicit_newlines_start_new_paragraphs() {
        let layout = layout_text("a\nb", &options(100.0), &mono);
        let texts: Vec<&str> = layout.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["a", "b"]);
    }

    #[test]
    fn an_empty_text_is_one_blank_line() {
        let layout = layout_text("", &options(100.0), &mono);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, "");
        assert_eq!(layout.total_height, 16.0);
    }

    #[test]
    fn alignment_left_starts_at_the_container_edge() {
        let layout = layout_text("hello", &options(30.0), &mono);
        assert_eq!(layout.lines[0].x_offset, 0.0);
    }

    #[test]
    fn alignment_center_centres_the_line() {
        let mut opts = options(30.0);
        opts.align = TextAlign::Center;
        let layout = layout_text("hello", &opts, &mono);
        assert_eq!(layout.lines[0].x_offset, 2.5);
    }

    #[test]
    fn alignment_right_ends_at_the_container_edge() {
        let mut opts = options(30.0);
        opts.align = TextAlign::Right;
        let layout = layout_text("hello", &opts, &mono);
        assert_eq!(layout.lines[0].x_offset, 5.0);
    }

    #[test]
    fn alignment_justify_spreads_words_and_leaves_the_last_line_left() {
        let mut opts = options(60.0);
        opts.align = TextAlign::Justify;
        let layout = layout_text("hello world\nbye", &opts, &mono);
        // "hello world" is 55 wide in a 60-wide container: 5 extra pixels
        // over 1 gap.
        assert_eq!(layout.lines[0].word_gap, 5.0);
        assert_eq!(layout.lines[0].x_offset, 0.0);
        // The last line of the paragraph is not justified.
        assert_eq!(layout.lines[1].word_gap, 0.0);
        assert_eq!(layout.lines[1].x_offset, 0.0);
    }

    #[test]
    fn truncation_clip_cuts_to_the_container_width() {
        let mut opts = options(30.0);
        opts.truncation = Truncation::Clip;
        opts.wrap = WrapMode::None;
        let layout = layout_text("hello world", &opts, &mono);
        assert_eq!(layout.lines[0].text, "hello ");
        assert!(layout.truncated);
    }

    #[test]
    fn truncation_ellipsis_appends_an_ellipsis_that_fits() {
        let mut opts = options(30.0);
        opts.truncation = Truncation::Ellipsis;
        opts.wrap = WrapMode::None;
        let layout = layout_text("hello world", &opts, &mono);
        assert_eq!(layout.lines[0].text, "hello…");
        assert!(layout.lines[0].width <= 30.0);
        assert!(layout.truncated);
    }

    #[test]
    fn truncation_none_leaves_an_overlong_line_whole() {
        let mut opts = options(30.0);
        opts.truncation = Truncation::None;
        opts.wrap = WrapMode::None;
        let layout = layout_text("hello world", &opts, &mono);
        assert_eq!(layout.lines[0].text, "hello world");
        assert!(!layout.truncated);
    }

    #[test]
    fn vertical_truncation_keeps_the_lines_that_fit() {
        // Truncation is off: the point is how many lines fit, not how the
        // last one is cut.
        let mut opts = options(100.0);
        opts.max_height = Some(25.0);
        opts.line_height = 10.0;
        opts.truncation = Truncation::None;
        let layout = layout_text("a\nb\nc\nd", &opts, &mono);
        let texts: Vec<&str> = layout.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["a", "b"]);
        assert_eq!(layout.total_height, 20.0);
        assert!(layout.truncated);
    }

    #[test]
    fn vertical_truncation_ellipsis_marks_the_last_kept_line() {
        let mut opts = options(100.0);
        opts.max_height = Some(15.0);
        opts.line_height = 10.0;
        opts.truncation = Truncation::Ellipsis;
        let layout = layout_text("hello\nworld", &opts, &mono);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, "hello…");
        assert!(layout.truncated);
    }

    #[test]
    fn letter_spacing_widens_every_line() {
        let mut opts = options(100.0);
        opts.letter_spacing = 2.0;
        let layout = layout_text("ab", &opts, &mono);
        // Two characters at 5 pixels plus 2 pixels of spacing each.
        assert_eq!(layout.lines[0].width, 14.0);
    }

    #[test]
    fn total_height_is_line_count_times_line_height() {
        let mut opts = options(100.0);
        opts.line_height = 12.0;
        let layout = layout_text("a\nb\nc", &opts, &mono);
        assert_eq!(layout.total_height, 36.0);
    }

    #[test]
    fn a_label_holds_its_properties_and_node() {
        let mut nodes = Arena::new();
        let label = Label::new(&mut nodes, "Hello");
        assert_eq!(label.text.get(), "Hello");
        assert_eq!(label.font_size.get(), 16.0);
        assert_eq!(label.color.get(), Color::new(0, 0, 0, 255));
        assert_eq!(
            label.font_family.get(),
            FamilyId::default(),
            "and it draws in the font set's default family, which is what a \
             caller that has never heard of families wants"
        );
        assert!(nodes.get(label.handle()).is_some());
    }

    #[test]
    fn a_label_lays_its_text_out_through_the_advance_callback() {
        let mut nodes = Arena::new();
        let label = Label::new(&mut nodes, "Hello");
        let layout = label.layout(&options(100.0), &mono);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, "Hello");
    }

    #[test]
    fn a_label_follows_its_text_property() {
        let mut nodes = Arena::new();
        let label = Label::new(&mut nodes, "Hello");
        label.text.set("Goodbye".to_string());
        let layout = label.layout(&options(100.0), &mono);
        assert_eq!(layout.lines[0].text, "Goodbye");
    }

    /// The two families a label can be painted in, as the handles `Label::paint`
    /// puts on its commands. **A `FontSet` is enough to make them** — no font file
    /// and no installed font — which is what lets this test be about the property
    /// reaching the command without a filesystem.
    fn two_families() -> (FontSet, FamilyId) {
        let mut fonts = FontSet::new();
        let other = fonts.define_family("other");
        (fonts, other)
    }

    #[test]
    fn the_family_the_label_carries_is_the_one_it_paints_its_runs_in() {
        // The whole of requirement 7's property half: a write to `font_family`
        // changes the family on every command the label records, which is how it
        // reaches the renderer at all.
        let (fonts, other) = two_families();
        let mut nodes = Arena::new();
        let label = Label::new(&mut nodes, "Hi");
        let families = |label: &Label| -> Vec<FamilyId> {
            label
                .paint(
                    crate::paint::Rect::new(0.0, 0.0, 100.0, 20.0),
                    &options(100.0),
                    &mono,
                )
                .iter()
                .filter_map(|command| match command {
                    crate::paint::DrawCommand::Text { family, .. } => Some(*family),
                    _ => None,
                })
                .collect()
        };

        assert_eq!(
            families(&label),
            vec![FamilyId::default()],
            "one line, in the default family, before anything is written"
        );
        label.font_family.set(other);
        assert_eq!(
            families(&label),
            vec![other],
            "and in the other family after the property is written — which is \
             also that the default family and a defined one are different handles, \
             since the fixture has defined it"
        );
        assert_ne!(other, FamilyId::default());
        assert_eq!(
            fonts.family("other"),
            other,
            "and the handle the property now holds is the one the set would hand \
             a caller that resolved the name"
        );
    }

    #[test]
    fn a_family_handle_reaches_the_command_unchanged() {
        // **What the widget owns, and only that:** the handle in the property is
        // the handle on the command. Whether the renderer can *draw* with it is
        // `FontSet`'s half and is not visible here — the handle this uses is an
        // ordinary one from a set that defined it, so this is not a test about
        // unknown handles at all.
        //
        // **The review of task 30 renamed this**, and both halves of the old name
        // were wrong. It was `a_family_the_set_never_defined_still_paints_rather_
        // than_vanishing`, while the fixture defined **two** families and handed
        // over the one registered as `"two"` — which the set knew perfectly well,
        // so no handle from an undefined family was ever produced — and "rather
        // than vanishing" claimed the opposite of what the code did, since a family
        // with nothing in it **did** vanish until the same review made `FontSet`
        // resolve an empty family to the default one. A test named for a case its
        // fixture cannot produce is the failure this repository records twice, and
        // this file makes the same point two hundred lines above.
        let mut fonts = FontSet::new();
        fonts.define_family("one");
        fonts.define_family("two");
        let defined = fonts.family("two");
        let mut nodes = Arena::new();
        let label = Label::new(&mut nodes, "Hi");
        label.font_family.set(defined);
        let commands = label.paint(
            crate::paint::Rect::new(0.0, 0.0, 100.0, 20.0),
            &options(100.0),
            &mono,
        );
        assert_eq!(
            commands.len(),
            1,
            "the run is recorded, and it carries the handle"
        );
        let crate::paint::DrawCommand::Text { family, .. } = &commands[0] else {
            panic!("a label's paint is text commands");
        };
        assert_eq!(*family, defined, "unchanged and unexamined");
    }
}

#[cfg(test)]
mod paint_tests {
    use super::*;
    use crate::paint::{DrawCommand, FadeRamp, Rect};

    /// The 5-pixel monospace advance this module's fixture is built on, so a
    /// character count is a pixel count and the numbers below are readable.
    ///
    /// **Its own copy rather than the one `tests` holds.** A private helper in a
    /// `#[cfg(test)]` module is not reachable from a sibling, and the two are
    /// fixtures rather than a rule: this module asserts its own numbers against
    /// this one and `tests` asserts its own against that one.
    fn mono(_: char) -> f32 {
        5.0
    }

    /// The x, y, text and tracking of every command a label records.
    fn runs(label: &Label, rect: Rect, options: &LayoutOptions) -> Vec<(f32, f32, String, f32)> {
        label
            .paint(rect, options, &|_: char| 5.0)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    x,
                    y,
                    text,
                    extra_advance,
                    ..
                } => Some((*x, *y, text.clone(), *extra_advance)),
                _ => None,
            })
            .collect()
    }

    /// A label in an arena, for painting.
    fn label(text: &str) -> (Arena<WidgetNode>, Label) {
        let mut nodes = Arena::new();
        let label = Label::new(&mut nodes, text);
        (nodes, label)
    }

    #[test]
    fn a_label_paints_one_command_per_line() {
        let (_nodes, label) = label("hello world");
        let mut options = LayoutOptions {
            max_width: 30.0,
            line_height: 20.0,
            ..LayoutOptions::default()
        };
        options.letter_spacing = 1.0;
        let rect = Rect::new(100.0, 50.0, 30.0, 40.0);
        let painted = runs(&label, rect, &options);
        assert_eq!(
            painted,
            vec![
                (100.0, 50.0, "hello".to_string(), 1.0),
                (100.0, 70.0, "world".to_string(), 1.0),
            ],
            "two wrapped lines, each on its own line box, each with the tracking"
        );
    }

    #[test]
    fn an_aligned_label_paints_at_the_offset_it_was_laid_out_at() {
        let (_nodes, label) = label("abcd");
        for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
            let options = LayoutOptions {
                max_width: 100.0,
                align,
                ..LayoutOptions::default()
            };
            let painted = runs(&label, Rect::new(0.0, 0.0, 100.0, 20.0), &options);
            assert_eq!(painted.len(), 1);
            let layout = label.layout(&options, &|_: char| 5.0);
            assert_eq!(painted[0].0, layout.lines[0].x_offset);
            assert_eq!(painted[0].1, 0.0, "the line sits at the rect's top edge");
        }
    }

    #[test]
    fn a_justified_label_paints_word_by_word_with_the_gap_between_them() {
        // The first line is justified because it is not its paragraph's last;
        // the second is not, because it is.
        let (_nodes, label) = label("aa bb cc\ndd ee ff");
        let options = LayoutOptions {
            max_width: 100.0,
            align: TextAlign::Justify,
            wrap: WrapMode::None,
            ..LayoutOptions::default()
        };
        let painted = runs(&label, Rect::new(0.0, 0.0, 100.0, 40.0), &options);
        let words: Vec<&str> = painted
            .iter()
            .map(|(_, _, text, _)| text.as_str())
            .collect();
        assert_eq!(
            words,
            vec!["aa", "bb", "cc", "dd ee ff"],
            "a justified line is word by word, and the last line of a paragraph \
             is left alone"
        );
        // Three 10-pixel words and two 5-pixel spaces take 40 of the 100
        // pixels, so each of the two gaps is stretched by 30.
        assert_eq!(painted[0].0, 0.0);
        assert_eq!(painted[1].0, 45.0);
        assert_eq!(painted[2].0, 90.0);
        assert_eq!(painted[3].2, "dd ee ff");
    }

    /// The commands a label recorded, as `(x, ramp, clip)` — the two fields this
    /// task added and nothing else, so a test about either is not also a test
    /// about the text.
    fn windows(
        label: &Label,
        rect: Rect,
        options: &LayoutOptions,
    ) -> Vec<(f32, Option<FadeRamp>, Option<Rect>)> {
        label
            .paint(rect, options, &mono)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { x, fade, clip, .. } => Some((*x, *fade, *clip)),
                _ => None,
            })
            .collect()
    }

    /// A label showing `text` at 24 pixels, which is the size the demo draws its
    /// text at and the size every width below is a number for.
    fn at_size(text: &str, size: f32) -> (Arena<WidgetNode>, Label) {
        let (nodes, label) = label(text);
        label.font_size.set(size);
        (nodes, label)
    }

    /// A single line long enough to be cut at 100, in the mode asked for.
    fn truncating(mode: Truncation) -> LayoutOptions {
        LayoutOptions {
            max_width: 100.0,
            line_height: 20.0,
            wrap: WrapMode::None,
            truncation: mode,
            ..LayoutOptions::default()
        }
    }

    /// 30 characters of `mono` is 150, so a 100-pixel box keeps 20 of them and
    /// the run ends 100 pixels after its left edge.
    const CUT_TEXT: &str = "abcdefghijklmnopqrstuvwxyz";

    #[test]
    fn a_label_records_its_runs_in_the_regular_face_whatever_the_truncation() {
        // `Label::paint` names `FontWeight::Regular` rather than letting
        // `TextRun::default()` fill it in, because a `Default` that changed would
        // draw every label in the application bold and nothing in `Label`'s API says
        // a label has a weight. This asserts every mode, because `paint` records
        // through two different literals and the clip only narrows the third.
        for mode in [
            Truncation::None,
            Truncation::Ellipsis,
            Truncation::Fade,
            Truncation::Clip,
        ] {
            let (_nodes, label) = at_size(CUT_TEXT, 24.0);
            let painted = label.paint(
                Rect::new(60.0, 170.0, 150.0, 29.0),
                &truncating(mode),
                &mono,
            );
            assert!(!painted.is_empty(), "{mode:?} records something");
            for command in &painted {
                let DrawCommand::Text { weight, .. } = command else {
                    panic!("a label records text commands");
                };
                assert_eq!(
                    *weight,
                    FontWeight::Regular,
                    "{mode:?} records a run in the regular face"
                );
            }
        }
    }

    #[test]
    fn a_fading_line_carries_a_ramp_that_ends_at_the_drawn_run_not_the_container() {
        // Requirement 1 and requirement 2 together, left-aligned, where the two
        // ends happen to agree and the assertion therefore says nothing about
        // which one the code used. The four-alignment test below is the one that
        // tells them apart.
        let (_nodes, label) = at_size(CUT_TEXT, 24.0);
        let options = truncating(Truncation::Fade);
        let rect = Rect::new(60.0, 170.0, 150.0, 29.0);
        let painted = windows(&label, rect, &options);
        assert_eq!(painted.len(), 1, "one line, one command");
        let (x, ramp, clip) = painted[0];
        assert_eq!(x, 60.0, "and it starts at the rect's left edge");
        let ramp = ramp.expect("a truncated fading line carries a ramp");
        assert_eq!(
            ramp.end_x, 160.0,
            "**the window ends at the run's end**, 60 + 100"
        );
        assert_eq!(
            ramp.start_x, 112.0,
            "**and begins two ems of 24 px before it** — `FADE_WIDTH_EM` pinned"
        );
        assert_eq!(clip, None, "a fading label is not clipped: only `Clip` is");
    }

    #[test]
    fn the_ramp_sits_at_the_drawn_runs_own_edge_under_every_alignment() {
        // Requirement 2, and the acceptance criterion that names it. `mono` is 5
        // pixels per character, so the run is exactly 100 wide and `fit` keeps 20
        // characters — and **`align_line` floors every offset with `.max(0.0)`**, so
        // an overlong line is drawn from `rect.x` under `Right` and `Center` too:
        // `max_width - line_width` is 0, not negative, because the *cut* line is
        // measured after the cut. So all four alignments put the run's end at
        // `rect.x + 100` and the window moves with the run, not with the
        // container. `rect.width` is 150 here, so a window that used the
        // container's edge would end at 210 and this assertion would fail.
        let (_nodes, label) = at_size(CUT_TEXT, 24.0);
        let rect = Rect::new(60.0, 170.0, 150.0, 29.0);
        for align in [
            TextAlign::Left,
            TextAlign::Center,
            TextAlign::Right,
            TextAlign::Justify,
        ] {
            let options = LayoutOptions {
                align,
                ..truncating(Truncation::Fade)
            };
            let painted = windows(&label, rect, &options);
            assert_eq!(painted.len(), 1, "{align:?} records one run");
            let ramp = painted[0]
                .1
                .unwrap_or_else(|| panic!("{align:?} carries a ramp, or this proves nothing"));
            assert_eq!(
                ramp.end_x, 160.0,
                "{align:?}: the window ends at the run's end, not at rect.x + width"
            );
            assert_ne!(
                ramp.end_x,
                rect.x + rect.width,
                "**and explicitly not at the container's edge**, which is the \
                 other number requirement 2 names"
            );
            assert_eq!(ramp.start_x, 112.0, "{align:?}: two ems of 24 px before it");
        }
    }

    #[test]
    fn the_ramps_far_end_is_the_runs_end_and_not_the_budget_the_layout_was_given() {
        // **The fixture that separates `left + line.width` from
        // `left + max_width`,** and without it the alignment test above cannot
        // tell which of the two the code computed: with a whole-pixel advance and
        // a whole-pixel budget, `fit` keeps exactly the budget and the two
        // numbers are the same one.
        //
        // 6.5 pixels a character against a 100-pixel budget: fifteen characters
        // are 97.5 and sixteen are 104, so the cut is 2.5 pixels short of the
        // budget. **That 2.5 is the whole of this test** — a window built from
        // `max_width` would put the ramp's far end inside the last glyph.
        let adv = 6.5;
        let advance = |_: char| adv;
        let (_nodes, label) = at_size(CUT_TEXT, 24.0);
        let options = truncating(Truncation::Fade);
        let layout = label.layout(&options, &advance);
        assert_eq!(layout.lines[0].text.len(), 15, "**fifteen characters fit**");
        assert!(
            (layout.lines[0].width - 97.5).abs() < 0.001,
            "and they are 97.5 wide, not 100: {}",
            layout.lines[0].width
        );

        let painted = label.paint(Rect::new(60.0, 170.0, 150.0, 29.0), &options, &advance);
        let ramp = match &painted[0] {
            DrawCommand::Text { fade, .. } => *fade,
            _ => panic!("a label records text commands"),
        };
        let ramp = ramp.expect("a truncated fading line carries a ramp");
        assert_eq!(
            ramp.end_x, 157.5,
            "**the window ends at the run's end**, 60 + 97.5"
        );
        assert_eq!(
            ramp.end_x,
            60.0 + options.max_width - 2.5,
            "and 2.5 pixels short of the budget, which is what `fit` left over"
        );
        assert_ne!(
            ramp.end_x,
            60.0 + options.max_width,
            "**not at the budget** — and this is the assertion that fails if the \
             window is ever built from `max_width` instead of from the drawn run"
        );
        assert_ne!(
            ramp.end_x,
            60.0 + 150.0,
            "**nor at the container's edge**, which is the other number requirement \
             2 names"
        );
    }

    #[test]
    fn the_ramp_is_pinned_to_two_ems_of_the_font_size() {
        // Requirement 1's "a documented width", as a number nothing can move.
        // **A design constant cannot be proved right by a test**, so the test
        // proves it has not changed, and the paragraph on `FADE_WIDTH_EM` is the
        // reason it is what it is.
        assert_eq!(
            FADE_WIDTH_EM, 2.0,
            "two ems: narrower than a glyph advance and it steps, wider than a \
             short line and it stops reading as a fade"
        );
        // The run here is `CUT_TEXT` cut to 100 pixels, and the sizes below are
        // the ones where two ems still fits inside it — so the window's width is
        // the ems and nothing else is in the way.
        for size in [8.0, 16.0, 24.0, 32.0] {
            let (_nodes, label) = at_size(CUT_TEXT, size);
            let painted = windows(
                &label,
                Rect::new(0.0, 0.0, 100.0, 29.0),
                &truncating(Truncation::Fade),
            );
            let ramp = painted[0]
                .1
                .expect("a truncated fading line carries a ramp");
            let width = ramp.end_x - ramp.start_x;
            let expected = FADE_WIDTH_EM * size;
            assert!(
                (width - expected).abs() < 0.001,
                "at {size} px the window is {width} wide and two ems is {expected}"
            );
        }
        // And the one case where it does not fit: 64 px puts two ems at 128
        // against a 100-pixel run, so the window is the whole run rather than
        // 128 pixels of it — which is `FadeRamp::to_run_end`'s floor and is
        // asserted here because a mutation that dropped the floor would widen
        // this window off the left end of the text.
        let (_nodes, label) = at_size(CUT_TEXT, 64.0);
        let painted = windows(
            &label,
            Rect::new(0.0, 0.0, 100.0, 29.0),
            &truncating(Truncation::Fade),
        );
        let ramp = painted[0]
            .1
            .expect("a truncated fading line carries a ramp");
        assert_eq!(
            ramp,
            FadeRamp::new(0.0, 100.0),
            "**at 64 px two ems is wider than the run, so the window is the run**"
        );
    }

    #[test]
    fn a_line_that_was_not_truncated_carries_no_ramp() {
        // The mutation "record a fade for every line" is a wrong picture — a
        // whole paragraph dimmed at its right-hand end for no reason — and the
        // only thing that sees it is the per-line flag.
        let (_nodes, label) = label("abcdef");
        // 6 characters at 5 pixels is 30, inside the 100-pixel box.
        let options = truncating(Truncation::Fade);
        let painted = windows(&label, Rect::new(0.0, 0.0, 100.0, 29.0), &options);
        assert_eq!(painted.len(), 1);
        assert_eq!(
            painted[0].1, None,
            "**a line that fits has no cut edge to fade at**"
        );
    }

    #[test]
    fn only_the_truncated_line_of_a_wrapped_fade_carries_a_ramp() {
        // The operator's policy: **every** truncated line ramps, at its own cut
        // edge — not only the last. A paragraph whose middle line is the one that
        // got cut is the shape that distinguishes "the last line" from "the cut
        // lines", so this is the assertion that holds the policy down rather than
        // the flag.
        // A word longer than the box, which is the one way a *wrapped* line gets
        // truncated: word wrapping leaves an overlong word on its own line
        // rather than breaking it, and `truncate_line` then cuts it.
        let long = "x".repeat(40);
        let (_nodes, label) = label(&format!("hi {long} bye"));
        let options = LayoutOptions {
            max_width: 60.0,
            line_height: 20.0,
            truncation: Truncation::Fade,
            ..LayoutOptions::default()
        };
        let layout = label.layout(&options, &mono);
        let truncated: Vec<bool> = layout.lines.iter().map(|l| l.truncated).collect();
        assert_eq!(
            truncated,
            vec![false, true, false],
            "**the middle line is the \
            one that was cut**, and the other two fit: 10, 60 of 60 and 15 pixels"
        );
        assert!(layout.truncated, "and the layout-wide flag still says so");

        let painted = windows(&label, Rect::new(0.0, 0.0, 100.0, 200.0), &options);
        assert_eq!(painted.len(), layout.lines.len(), "one command per line");
        for (index, (x, ramp, _)) in painted.iter().enumerate() {
            let expected = layout.lines[index].truncated;
            if expected {
                let ramp = ramp.expect("a truncated line ramps");
                assert_eq!(
                    ramp.end_x,
                    *x + layout.lines[index].width,
                    "**line {index} ramps at its own cut edge**: the drawn run's \\
                     end, which is where the characters this line lost would have \\
                     gone"
                );
            } else {
                assert_eq!(
                    *ramp, None,
                    "**line {index} was not cut and must not fade**"
                );
            }
        }
    }

    #[test]
    fn a_justified_fading_line_ramps_at_the_end_of_its_last_word() {
        // The branch with one command per word, where the window cannot be known
        // until the last word has been measured. `mono` is 5 per character, so
        // "aa bb cc" is 45 wide and the two 5-pixel gaps take 10 of the 100,
        // leaving 45 for the stretch: 22.5 each, so the words are at 0, 52.5 and
        // 105 and the run ends at 135.
        let (_nodes, short) = label("aa bb cc");
        let options = LayoutOptions {
            max_width: 57.0,
            line_height: 20.0,
            wrap: WrapMode::None,
            truncation: Truncation::Fade,
            align: TextAlign::Justify,
            ..LayoutOptions::default()
        };
        // 45 < 57, so this fits: a sanity check on the fixture, because the
        // second half of it only means something if the first really was cut.
        let fitted = short.layout(&options, &mono);
        assert!(
            !fitted.lines[0].truncated,
            "the short fixture fits: 45 of 57"
        );

        // So make it too long to fit and justify what is left. `fit` keeps whole
        // characters, so 57 pixels is 11 of them: "aa bb cc dd", which is four
        // words and **no trailing space** — and that matters, because a line cut
        // after a space measures the same as its budget and is justified by
        // nothing at all. Four words, three gaps, 55 pixels used and 2 to
        // stretch: `word_gap` is 2/3.
        //
        // **Two paragraphs, and that is not decoration.** `align_line`
        // justifies nothing that is its paragraph's last line, so a single
        // paragraph under `WrapMode::None` is one last line and gets
        // `word_gap = 0` — the second line here is what makes the first one
        // justifyable. It also has to be *short*, or it would be the one cut.
        let (_nodes, justified) = label("aa bb cc dd ee ff\nshort");
        let layout = justified.layout(&options, &mono);
        assert_eq!(
            layout.lines[0].text, "aa bb cc dd",
            "**the fixture is cut at 57**"
        );
        assert!(layout.lines[0].truncated, "and the line says so");
        assert!(
            !layout.lines[1].truncated,
            "while the second line is not cut"
        );
        assert!(
            layout.lines[0].word_gap > 0.0,
            "and justified, or there is no word branch to read: {}",
            layout.lines[0].word_gap
        );
        let painted = justified.paint(Rect::new(0.0, 0.0, 57.0, 20.0), &options, &mono);
        // **Only the first line's commands.** The fixture has two lines and the
        // second is not justified, so reading "the last command of the paint"
        // would read the wrong line's last word — which is the trap this test
        // exists to walk into, so it is named rather than hidden.
        let words: Vec<(f32, String, Option<FadeRamp>)> = painted
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    x, y, text, fade, ..
                } if *y == 0.0 => Some((*x, text.clone(), *fade)),
                _ => None,
            })
            .collect();
        assert!(
            words.len() > 1,
            "a justified line is one command per word, and this one is not \
             justified otherwise: {} command(s)",
            words.len()
        );
        let last = words.last().expect("at least one word");
        let last_end = last.0 + measure(&last.1, options.letter_spacing, &mono);
        for (x, word, ramp) in &words {
            let ramp = ramp.expect("every word of a truncated justified line carries the ramp");
            assert_eq!(
                ramp.end_x, last_end,
                "**every word carries the same window**, and it ends at the last \
                 word's end ({last_end}), not at the container's 57"
            );
            let word_end = x + measure(word, options.letter_spacing, &mono);
            assert!(
                word_end <= last_end,
                "and no word ends past it: {word:?} ends at {word_end}"
            );
        }
    }

    #[test]
    fn a_clipping_label_records_its_box_and_the_other_modes_record_none() {
        // Requirement 3, on the recorded command. The box is **narrowed to
        // `max_width`**, which is the box the text is cut at and — where the
        // layout pass gave the node a rect of the text's natural size, which is
        // what the demo does — not the node's own width.
        let (_nodes, label) = label(CUT_TEXT);
        let rect = Rect::new(60.0, 170.0, 150.0, 29.0);
        let painted = windows(&label, rect, &truncating(Truncation::Clip));
        assert_eq!(painted.len(), 1);
        assert_eq!(
            painted[0].2,
            Some(Rect::new(60.0, 170.0, 100.0, 29.0)),
            "**the box is the cut at 100, not the 150-wide node rect**"
        );
        assert_eq!(painted[0].1, None, "and a clipped line does not fade");

        for mode in [Truncation::None, Truncation::Ellipsis, Truncation::Fade] {
            let painted = windows(&label, rect, &truncating(mode));
            assert_eq!(painted[0].2, None, "{mode:?} records no clip");
        }
    }

    #[test]
    fn a_clipping_label_is_clipped_to_its_own_box_when_the_box_is_the_smaller_claim() {
        // The `.min` the other way round, and the reason it is there: a node given
        // a tight constraint narrower than the text's `max_width` is clipped to
        // the *node*, because the node's box is the smaller claim.
        let (_nodes, narrow_node) = label(CUT_TEXT);
        let painted = windows(
            &narrow_node,
            Rect::new(60.0, 170.0, 40.0, 29.0),
            &truncating(Truncation::Clip),
        );
        assert_eq!(painted[0].2, Some(Rect::new(60.0, 170.0, 40.0, 29.0)));

        // And `LayoutOptions::default`'s `max_width` of infinity, which is the
        // case a caller that never set it is in: the clip is the node's box, not
        // an unbounded one.
        let (_nodes, unbounded) = label(CUT_TEXT);
        let painted = windows(
            &unbounded,
            Rect::new(60.0, 170.0, 150.0, 29.0),
            &LayoutOptions {
                truncation: Truncation::Clip,
                wrap: WrapMode::None,
                line_height: 20.0,
                ..LayoutOptions::default()
            },
        );
        assert_eq!(
            painted[0].2,
            Some(Rect::new(60.0, 170.0, 150.0, 29.0)),
            "**an unset `max_width` does not make the clip unbounded**"
        );
    }

    #[test]
    fn a_clipping_labels_box_covers_the_text_it_draws() {
        // The vertical half of `clip_for`, at a size where the gap is visible in
        // the arithmetic. `Label::size` answers a **font size** for the height,
        // and a line box is a **line height**: 64 against 78, so a clip taken at
        // the node's own height would cut 14 pixels off the bottom of the text —
        // and a descender is exactly what lives there. `Truncation::Clip` claims
        // to cut overflow, so a clip that cuts a `p` in half is not that.
        let (_nodes, label) = at_size(CUT_TEXT, 64.0);
        let options = LayoutOptions {
            max_width: 100.0,
            line_height: 78.0,
            wrap: WrapMode::None,
            truncation: Truncation::Clip,
            ..LayoutOptions::default()
        };
        let painted = windows(&label, Rect::new(60.0, 170.0, 100.0, 64.0), &options);
        let clip = painted[0].2.expect("a clipping label records its box");
        assert_eq!(
            clip,
            Rect::new(60.0, 170.0, 100.0, 78.0),
            "**the box is 78 tall, not the node's 64**: it reaches the bottom of \
             the one line box the layout drew"
        );
        // And a node *taller* than its text keeps its own height, because then
        // the node's box is the larger claim and nothing is cut either way.
        let painted = windows(&label, Rect::new(60.0, 170.0, 100.0, 200.0), &options);
        assert_eq!(
            painted[0].2,
            Some(Rect::new(60.0, 170.0, 100.0, 200.0)),
            "a node taller than its text is clipped to the node"
        );
        // And a node taller than its text with two lines is the *text*, which is
        // 156, so it is still the node.
        let multi = LayoutOptions {
            max_width: 40.0,
            ..options
        };
        let painted = windows(&label, Rect::new(60.0, 170.0, 40.0, 200.0), &multi);
        assert_eq!(
            painted[0].2,
            Some(Rect::new(60.0, 170.0, 40.0, 200.0)),
            "two line boxes of 78 are 156, inside the node's 200"
        );
    }

    #[test]
    fn the_cut_leaves_the_last_glyph_inside_the_box_and_the_clip_is_what_would_cut_it() {
        // Requirement 3's acceptance criterion, as far as a font-free test can
        // reach it, and the honest statement of what is left for the GPU.
        //
        // `fit` keeps whole characters whose accumulated advance fits the budget,
        // so **the layout's cut never starts a glyph past the box** — that is the
        // half of the criterion a unit test can hold, and it is what makes the
        // other half a question about bearings rather than about this code. What
        // the GPU adds is the glyph's *ink* width, which is `bearing_x + width`
        // and is not in the command: `A`, `f` and `v` in Lato Medium overhang
        // their advance by exactly 1.0 px at 24 px, measured, and a cut that
        // happened to land on one of them is a glyph the scissor removes and the
        // layout could not have. So the gate is the box, and the box is asserted
        // here: a point inside the box is inside the clip and a point past it is
        // not.
        let (_nodes, label) = label(CUT_TEXT);
        let rect = Rect::new(60.0, 170.0, 150.0, 29.0);
        let painted = windows(&label, rect, &truncating(Truncation::Clip));
        let (x, _, clip) = painted[0];
        let clip = clip.expect("a clipping label records its box");
        let layout = label.layout(&truncating(Truncation::Clip), &mono);
        let last = &layout.lines[0];
        assert!(
            x + last.width <= clip.x + clip.width,
            "**every glyph the layout kept starts inside the box**: the run ends \
             at {} and the box at {}",
            x + last.width,
            clip.x + clip.width
        );
        let inside = clip.x + clip.width - 0.5;
        let outside = clip.x + clip.width + 0.5;
        assert!(
            inside >= clip.x && inside <= clip.x + clip.width,
            "**a glyph inside the box is not cut**: {inside} is inside \
             {}..={}",
            clip.x,
            clip.x + clip.width
        );
        assert!(
            !(outside >= clip.x && outside <= clip.x + clip.width),
            "**and a glyph past the edge is**: {outside} is outside \
             {}..={}",
            clip.x,
            clip.x + clip.width
        );
    }

    #[test]
    fn a_label_paints_nothing_without_text() {
        let (_nodes, label) = label("");
        let painted = runs(
            &label,
            Rect::new(0.0, 0.0, 100.0, 20.0),
            &LayoutOptions::default(),
        );
        assert_eq!(painted.len(), 1, "an empty label is still one line");
        assert_eq!(painted[0].2, "");
    }
}
