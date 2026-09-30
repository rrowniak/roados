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
//! would take (see `doc/ui/IMPLEMENTATION_STATE.md`). The font fallback chain
//! and dynamic atlas growth are not built either; a character the one loaded
//! face has no glyph for is skipped.

use crate::arena::{Arena, Handle};
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
    /// Cut at the container's edge.
    Clip,
    /// Cut at the container's edge and append an ellipsis (`…`).
    Ellipsis,
    /// Cut at the container's edge. Layout-wise identical to
    /// [`Truncation::Clip`]: the fade is a rendering effect, and the renderer
    /// that will apply it does not exist yet.
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
    /// The font family name, resolved against the font fallback chain by the
    /// rendering pipeline.
    pub font_family: Property<String>,
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
            font_family: Property::new("sans-serif".to_string()),
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
    pub fn paint(
        &self,
        rect: crate::paint::Rect,
        options: &LayoutOptions,
        advance: &dyn Fn(char) -> f32,
    ) -> Vec<crate::paint::DrawCommand> {
        let layout = self.layout(options, advance);
        let font_size = self.font_size.get();
        let color = self.color.get();
        let space = measure(" ", options.letter_spacing, advance);
        let mut painter = crate::paint::Painter::new();
        let mut top = rect.y;
        for line in &layout.lines {
            let left = rect.x + line.x_offset;
            if line.word_gap > 0.0 {
                let mut x = left;
                for (index, word) in line.text.split_whitespace().enumerate() {
                    if index > 0 {
                        x += space + line.word_gap;
                    }
                    painter.text(x, top, word, color, font_size, options.letter_spacing);
                    x += measure(word, options.letter_spacing, advance);
                }
            } else {
                painter.text(
                    left,
                    top,
                    &line.text,
                    color,
                    font_size,
                    options.letter_spacing,
                );
            }
            top += options.line_height;
        }
        painter.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(label.font_family.get(), "sans-serif");
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
}

#[cfg(test)]
mod paint_tests {
    use super::*;
    use crate::paint::{DrawCommand, Rect};

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
