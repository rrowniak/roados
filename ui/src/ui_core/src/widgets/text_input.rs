//! The TextInput widget: single-line text entry, with a caret and a selection.
//!
//! A text input is a node, the three properties the task gives it — `text`,
//! `placeholder` and `focused` — two callbacks, and the properties the widget
//! animates. The caret is the interesting part, and it is two numbers: `caret`,
//! a character index, is the truth, and `caret_position`, a *fractional*
//! character index, is what is drawn. The distance between them is
//! the transition, exactly as a [`Slider`]'s `value` and `thumb` are, and for
//! the same reason: an arrow key moves the truth and
//! [`animate_to_state`](TextInput::animate_to_state) carries the drawn position
//! over, while a keystroke writes both at once — a caret that lags behind the
//! character being typed is a caret in the wrong place.
//!
//! # The caret's x position, and why this module accumulates it
//!
//! [`label`](crate::widgets::label)'s `layout_text` owns the text layout, and it
//! is the only public
//! measurement of a string there is. It returns *lines* — their text, their
//! width, their alignment offset — and not one x offset per character. The
//! helpers that would give one, `measure` and `fit`, are private to
//! [`label`](crate::widgets::label), and a text input is the second widget that
//! needs per-character measurement (the [`Button`] is the first) and the first
//! that needs it *mid-string*. So this module walks the characters itself,
//! accumulating `advance(ch) + letter_spacing` per character through the same
//! `&dyn Fn(char) -> f32` seam the layout uses, and every caret position, every
//! selection edge and every clip boundary is a point on that walk. Exposing
//! `label::measure` instead would be the right call for the repository, but
//! `label.rs` is not this task's file, so the walk is local and its one
//! requirement is that it agree with `label`'s: both add the letter spacing
//! after *every* character including the last, so a caret at the end of a line
//! sits one letter spacing past the final glyph.
//!
//! No `f32` in the walk is ever converted to an integer, and no integer to an
//! `f32`. There is no `From` or `TryFrom` between them in std, the walk crosses
//! between a character index and a position on several edges, and the one
//! documented `as` cast in `animation.rs` is not a precedent worth a second in
//! a widget. So both crossings are themselves walks: `position_of` counts
//! characters up to an index, and `offset_at_position` counts them up to a
//! fractional position, stopping inside the character the position falls in.
//!
//! # Clipping, and why the widget has to do it itself
//!
//! Requirement 6 asks for the text to be clipped to the input's bounds, and
//! nothing above this widget can do it. A [`DrawCommand::Text`] carries an `x`,
//! a `y`, a `String`, a colour, a size and a tracking, and **no width**:
//! `scroll::command_bounds` returns `None` for one, because bounding a run
//! honestly would mean inventing a width it does not carry, and
//! `scroll::clip_commands` therefore *keeps* every text run whatever the
//! viewport is — "a text run on screen is never dropped for want of a
//! measurement this layer has no way to take". A run that overflows is drawn
//! whole, and the only thing that can stop it is the scissor.
//!
//! So the widget does what a single-line field does and owns a horizontal scroll
//! offset: `scroll_x` is how far the content is scrolled, `caret_x` is where
//! the caret is drawn, and [`paint`](TextInput::paint) emits only the
//! characters that **fit entirely** inside the field's inner box — the same
//! walk, the same numbers, one character at a time. "Fits entirely" is the rule
//! and it is stricter than it looks: a character that overhangs either edge is
//! not emitted, so no glyph is ever drawn over the border or into the padding,
//! and the cost is a gap of up to one character's advance at the right edge
//! rather than an overhang. A gap is the right trade, because the overhang
//! alternative needs the GPU to hide: a caller that wants the full width drawn
//! scissors the node with `Renderer::draw_node_clipped`, and this widget does
//! not depend on that — it clips correctly whether or not anything scissors it.
//!
//! What the scroll buys is the caret. `ensure_caret_visible` moves the offset so
//! the caret is inside the field, and it runs in `paint` as well as in
//! [`on_event`](TextInput::on_event), so a caret moved through
//! [`move_caret`](TextInput::move_caret) — which has no rect to measure against
//! — is brought into view on the next frame rather than never.
//!
//! # What is not here
//!
//! The on-screen keyboard of requirement 4 is another widget. The operations it
//! needs from this one are public, and are exactly the ones a key-cap press maps
//! onto: [`insert_text`](TextInput::insert_text),
//! [`delete_backward`](TextInput::delete_backward) and
//! [`move_caret`](TextInput::move_caret).
//!
//! [`Slider`]: crate::widgets::slider::Slider
//! [`Button`]: crate::widgets::button::Button

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::time::Duration;

use crate::animation::{AnimationClock, Interpolate};
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::{LayoutState, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::button::Motion;
use crate::widgets::Callback;
use sdl3::gamepad::Button as GamepadButton;
use sdl3::keyboard::Keycode;

/// The width a text input asks for when its caller gives it no size of its own.
///
/// A field has no content to measure — it is as wide as whatever the value in it
/// happens to be, which is not a size — so this is the one number that decides
/// how long it is by default. It is a constant rather than a theme token for the
/// reason the slider's own `DEFAULT_LENGTH` is.
///
/// It is a **default**, not a fixed number: [`TextInput::width`] starts here and
/// a caller writes its own. That is the defect `.ai/NEVERAGAIN.md` records as
/// *one sibling got the operator's fix* — the operator widened a 6-pixel slider
/// track and a 6-pixel scrollbar on two consecutive days, and a sizing constant
/// with no door cannot be answered the same way twice.
const DEFAULT_WIDTH: f32 = 240.0;

/// The height a text input asks for when its caller gives it no size of its own.
///
/// 44dp is the platform touch target a finger can hit, and it is the same floor
/// [`button::MIN_TOUCH_TARGET`](crate::widgets::button::MIN_TOUCH_TARGET) puts
/// under a button: a field is found by tapping it. It is repeated here rather
/// than imported for the reason the slider's own `MIN_TOUCH_TARGET` is.
///
/// It is a **default**, not a fixed number, for the same reason
/// [`DEFAULT_WIDTH`] is: [`TextInput::height`] starts here and a caller writes
/// its own. A head unit read across a cabin wants more than 44, and the widget
/// cannot know that.
const MIN_TOUCH_TARGET: f32 = 44.0;

/// The width of the border drawn around the field, in pixels, unless a caller
/// changes it.
///
/// The border is a filled rounded rectangle at the field's own rect, with the
/// surface drawn over it inset by this much, so only the border is left:
/// [`DrawCommand::RoundedRect`] fills its rect, and a border drawn any other way
/// is a second background rather than an outline — the defect `.ai/NEVERAGAIN.md`
/// records as *a filled rounded rectangle is not an outline*.
///
/// The theme's own `BorderWidth` is a 1-pixel hairline. A field is read at a
/// glance from across a cabin, so its border is twice that, and a caller that
/// wants the hairline writes the theme's own number into the property.
const DEFAULT_BORDER_WIDTH: f32 = 2.0;

/// The radius of the field's corners, in pixels, unless a caller changes it.
///
/// Eight is the theme's own `BorderRadiusMd`, repeated here rather than read
/// from the theme because a [`Palette`] carries colours and not shape — the
/// reason the button's own `THEME_RADIUS_MD` gives.
const DEFAULT_CORNER_RADIUS: f32 = 8.0;

/// The gap between the field's border and its text, in pixels, unless a caller
/// changes it.
///
/// The same number feeds [`TextInput::inner_rect`], which is both the text's
/// drawing box and the box a pointer's x is measured against, so hit testing
/// and drawing cannot disagree about where the text starts.
const DEFAULT_PADDING: f32 = 8.0;

/// The width of the caret, in pixels, unless a caller changes it.
///
/// A property and not a bare constant for the reason the operator's own
/// rejection of a 6-pixel slider track is recorded for: a sizing number the
/// operator may want a different one of has to be reachable by a setter, or the
/// next request for it is a rebuild.
const DEFAULT_CARET_WIDTH: f32 = 2.0;

/// The font size the text is drawn at, in pixels, unless a caller changes it.
///
/// The theme's `FontSizeMd` is 14 and this is 16: a field's text is the value
/// being read rather than a caption beside it, and the demo's own readouts are
/// the same size. A caller that wants the theme's number writes it in.
const DEFAULT_FONT_SIZE: f32 = 16.0;

/// The height of the text's line box, in pixels, unless a caller changes it.
///
/// The line box is what the text's `y` is the top of and what the text is
/// centred in, so it is the one number that decides where the text sits
/// vertically inside the field.
const DEFAULT_LINE_HEIGHT: f32 = 20.0;

/// The extra pixels between two characters, in pixels, unless a caller changes
/// it.
///
/// Zero, which is what a caret's x is measured with unless a caller says
/// otherwise. It is a property because it is a *text* setting the same pipeline
/// [`label::LayoutOptions`](crate::widgets::label::LayoutOptions) takes, and a
/// field with tracking on has a caret that has to count it.
const DEFAULT_LETTER_SPACING: f32 = 0.0;

/// How long the caret's visible phase lasts.
///
/// Half a second, which is requirement 5's number. It is its own constant rather
/// than half of [`BLINK_PERIOD`] because "visible for 500ms, hidden for 500ms"
/// is two numbers, and one number the other is derived from is one that can be
/// edited into disagreeing with the sentence it came from.
const BLINK_VISIBLE: Duration = Duration::from_millis(500);

/// How long the caret's hidden phase lasts.
const BLINK_HIDDEN: Duration = Duration::from_millis(500);

/// One whole blink: the visible phase and the hidden phase together.
///
/// Not a constant, because a `Duration` sum is not one, and not a third number
/// either: the period is computed from the two phases wherever it is needed, so
/// it cannot be edited into disagreeing with them.
fn blink_period() -> Duration {
    BLINK_VISIBLE + BLINK_HIDDEN
}

/// How far the selection's fill is from the field's own surface toward the
/// theme's primary, as a fraction of the way between them.
///
/// The fill is not a translucent `Primary` over the surface, and the reason is
/// arithmetic rather than taste. These colours are premultiplied, so a
/// translucent primary over the surface is a *composite*, and a composite
/// cannot be a token: a command carries one colour and the renderer applies the
/// one alpha it has. The fill is therefore the surface moved this much of the
/// way toward the primary — the same shape the button's own `shade` has — which
/// needs no knowledge of either colour and is a colour a caller can read off the
/// widget and assert on.
///
/// 0.35 is what both themes need and neither can do without. In the dark theme
/// `Primary` is a violet on a near-black surface and in the light one a deep
/// violet on a near-white; at 0.35 the fill differs from the surface by 55 to 86
/// in one channel, which is a highlight rather than a tint, and it leaves `Text`
/// legible over it in both. A smaller number is invisible in the light theme and
/// a larger one stops being a background for the text.
const SELECTION_BLEND: f32 = 0.35;

/// The colours a text input draws with.
///
/// Six colours, and no token of their own: the theme has none for a caret, a
/// selection, a placeholder or a text field's parts, and adding one per part
/// would put six more tokens in `ThemeToken::all`, in both theme tables, in the
/// token count and in the animation every token takes part in during a switch —
/// for values a switch does not change. That cost is what `button.rs` documents;
/// each field below says which token it is instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The field's own surface: [`Surface`](crate::theme::ThemeToken::Surface),
    /// the colour a raised thing is painted with.
    pub surface: Color,
    /// The text: [`Text`](crate::theme::ThemeToken::Text).
    pub text: Color,
    /// The placeholder: [`TextMuted`](crate::theme::ThemeToken::TextMuted),
    /// which the theme calls "text that is present but not emphasised" — a
    /// placeholder is exactly that.
    pub muted: Color,
    /// The border while the field is *not* focused:
    /// [`Border`](crate::theme::ThemeToken::Border), the theme's hairline.
    pub border: Color,
    /// The border while the field is focused, and the token the selection's fill
    /// is derived from: [`Primary`](crate::theme::ThemeToken::Primary).
    pub focus: Color,
    /// The caret. `Text`, not `OnPrimary`: the caret is drawn on the field's own
    /// surface and not on the primary colour, and the one token this repository
    /// uses for something that has to be legible on a surface is `Text`.
    pub caret: Color,
}

impl Default for Palette {
    /// Returns a neutral grey field: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    fn default() -> Self {
        Palette {
            surface: Color::new(48, 48, 48, 255),
            text: Color::new(240, 240, 240, 255),
            muted: Color::new(144, 144, 144, 255),
            border: Color::new(80, 80, 80, 255),
            focus: Color::new(200, 200, 200, 255),
            caret: Color::new(240, 240, 240, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The theme's own [`Background`](crate::theme::ThemeToken::Background) is
    /// deliberately not among them. It is the other candidate for the field's
    /// surface, and a field painted in it is invisible: in both themes it is the
    /// colour the window already is, so the field would be a border drawn round
    /// nothing. `Surface` exists for a raised thing, which is what a field is.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::theme::{Theme, ThemeToken};
    /// use ui_core::widgets::text_input::Palette;
    ///
    /// let dark = Palette::from_theme(&Theme::dark());
    /// let text = match Theme::dark().get(ThemeToken::Text) {
    ///     ui_core::theme::PropertyValue::Color(color) => color,
    ///     _ => unreachable!(),
    /// };
    /// assert_eq!(dark.text, text);
    /// assert_ne!(dark.text, Palette::from_theme(&Theme::light()).text);
    /// ```
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            surface: token_color(theme, crate::theme::ThemeToken::Surface),
            text: token_color(theme, crate::theme::ThemeToken::Text),
            muted: token_color(theme, crate::theme::ThemeToken::TextMuted),
            border: token_color(theme, crate::theme::ThemeToken::Border),
            focus: token_color(theme, crate::theme::ThemeToken::Primary),
            caret: token_color(theme, crate::theme::ThemeToken::Text),
        }
    }
}

/// The appearance the field's state implies.
///
/// Every field is a target, not a value in flight:
/// [`animate_to_state`](TextInput::animate_to_state) animates the widget's
/// properties toward this and [`paint`](TextInput::paint) draws whatever the
/// properties have reached, which is a `Style` part way through on a frame where
/// something is moving.
///
/// The placeholder's colour is not among them. It is a token rather than an
/// animated quantity — nothing about focus or a theme switch changes whether a
/// placeholder is muted — so it is read from the [`Palette`] at paint time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The field's own surface.
    pub surface: Color,
    /// The text's colour.
    pub text: Color,
    /// The border's colour: the focus colour while focused, the border token
    /// otherwise. It is applied independently of everything else, the way a
    /// [`Button`](crate::widgets::button::Button)'s ring is, so a focused field
    /// keeps every other part of its appearance.
    pub border: Color,
    /// The caret's colour.
    pub caret: Color,
    /// The selection's fill: the surface moved `SELECTION_BLEND` of the way
    /// toward [`Palette::focus`].
    pub selection: Color,
}

impl Style {
    /// Returns the selection's fill for a `focus` token over a `surface`.
    ///
    /// The whole of [`SELECTION_BLEND`], as a colour rather than an alpha, for
    /// the reason that constant gives.
    fn selection_fill(focus: Color, surface: Color) -> Color {
        Color::interpolate(&surface, &focus, SELECTION_BLEND)
    }
}

/// A text input: a single-line field for entering and editing text.
///
/// The widget holds the properties the task gives it — [`text`](TextInput::text),
/// [`placeholder`](TextInput::placeholder),
/// [`focused`](TextInput::focused), [`on_change`](TextInput::on_change) and
/// [`on_submit`](TextInput::on_submit) — and the ones it animates:
/// [`caret_position`](TextInput::caret_position),
/// [`background`](TextInput::background), [`foreground`](TextInput::foreground),
/// [`border`](TextInput::border), [`caret_color`](TextInput::caret_color),
/// [`selection`](TextInput::selection) and [`scroll_x`](TextInput::scroll_x).
/// The sizing is properties too, so an operator who rejects a number has a
/// setter to change it with. The two numbers that are not properties are the
/// blink's two phases, which are a timing rather than an appearance, and the
/// selection's blend, which is a decision rather than a choice.
///
/// The node is the caller's to keep clean, and its size is the caller's to give
/// through [`layout_mut`](crate::node::WidgetNode::layout_mut);
/// [`TextInput::size`] is a suggestion for a caller who has nothing else to go
/// on. A field draws inside whatever rect it is given.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::{InputEvent, InputEventKind};
/// use ui_core::layout::Offset;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::text_input::TextInput;
///
/// let mut nodes = Arena::new();
/// let input = TextInput::new(&mut nodes);
/// assert_eq!(input.text.get(), "");
/// assert!(!input.focused.get());
///
/// // A tap focuses the field and puts the caret where the finger was.
/// let rect = Rect::new(0.0, 0.0, 100.0, 44.0);
/// let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(5.0, 20.0)));
/// assert!(input.on_event(&mut tap, rect, &|_: char| 10.0));
/// assert!(input.focused.get());
///
/// // Typed text arrives as `Text`, which is layout-correct.
/// let mut typed = InputEvent::new(
///     InputEventKind::Text { text: "ab".to_string() },
///     None,
/// );
/// assert!(input.on_event(&mut typed, rect, &|_: char| 10.0));
/// assert_eq!(input.text.get(), "ab");
/// assert_eq!(input.caret(), 2, "with the caret after what was inserted");
/// ```
pub struct TextInput {
    /// The text in the field. The caller may write it — that is the "set
    /// programmatically" case — and the widget writes it from every edit.
    ///
    /// Writing it moves neither the caret nor [`on_change`](TextInput::on_change):
    /// a caller that sets the value is itself, and reads the property back. The
    /// callback is the widget reporting its own doing, the same rule the
    /// [`Slider`](crate::widgets::slider::Slider) documents.
    pub text: Property<String>,
    /// The text drawn, in [`Palette::muted`], while [`text`](TextInput::text) is
    /// empty. It goes through the same pipeline and is clipped to the same
    /// bounds as the real text.
    pub placeholder: Property<String>,
    /// Whether the field holds focus, and therefore whether it takes keys, draws
    /// a caret and blinks.
    ///
    /// Written by the caller from [`input::Focus`](crate::input::Focus), or by a
    /// tap, which [`on_event`](TextInput::on_event) handles itself. Prefer
    /// [`focus`](TextInput::focus), which also restarts the blink: a field that
    /// gained focus with the caret in its hidden phase would show nothing for up
    /// to half a second.
    pub focused: Property<bool>,
    /// The callback every edit fires, with the text as it is after the edit.
    ///
    /// It fires when the text *moved*, and only for an interaction: a backspace
    /// at the start, or a delete at the end, reports nothing, and neither does
    /// an arrow key or a caret move, which change where the insertion point is
    /// rather than what is there.
    pub on_change: Callback<String>,
    /// The callback Enter fires, with the text as it is at that moment.
    ///
    /// It fires while the field holds focus and is consumed there, so a Return
    /// that submits one field does not also submit whatever is behind it.
    pub on_submit: Callback<String>,

    /// The character index the caret is *drawn* at, animated toward
    /// [`caret`](TextInput::caret).
    ///
    /// A fractional index, because that is what a caret in motion is: between
    /// character 3 and character 4 it is at 3.5, and
    /// [`caret_x`](TextInput::caret_x) reads it as such. An interaction writes
    /// it at once with the index, so the caret is under the finger and after the
    /// character just typed; a programmatic move leaves it behind, and
    /// [`animate_to_state`](TextInput::animate_to_state) carries it over.
    pub caret_position: Property<f32>,
    /// The field's own surface, animated.
    pub background: Property<Color>,
    /// The text's colour, animated.
    pub foreground: Property<Color>,
    /// The border's colour, animated. It is [`Palette::focus`] while focused
    /// and [`Palette::border`] otherwise; this is the property that follows.
    pub border: Property<Color>,
    /// The caret's colour, animated.
    pub caret_color: Property<Color>,
    /// The selection's fill, animated. It is *not* a token: the field blends its
    /// own surface toward [`Palette::focus`] by `SELECTION_BLEND`, and
    /// [`style`](TextInput::style) says what the blend is.
    pub selection: Property<Color>,
    /// How far the text is scrolled horizontally, in pixels, in the field's own
    /// content coordinates.
    ///
    /// Written by the widget from `ensure_caret_visible` and read by nothing
    /// else, so it is a property rather than a private `Cell` for the repo's own
    /// reason: a write fires the caller's `on_change`, which is how a node is
    /// marked dirty.
    pub scroll_x: Property<f32>,
    /// The font size the text is drawn at, in pixels.
    pub font_size: Property<f32>,
    /// The height of the text's line box, in pixels. The text is centred in the
    /// field's inner box against it.
    pub line_height: Property<f32>,
    /// Extra pixels between two characters, in pixels. The caret's x counts it,
    /// because [`label`](crate::widgets::label)'s measurement does.
    pub letter_spacing: Property<f32>,
    /// The gap between the border and the text, in pixels. It is both the text's
    /// drawing inset and the box a pointer's x is measured against.
    pub padding: Property<f32>,
    /// The radius of the field's corners, in pixels.
    pub border_radius: Property<f32>,
    /// The width of the border, in pixels. The surface is drawn inset by it, so
    /// this is also how much of the field's own rect the border covers.
    ///
    /// Not hit tested, and nothing in this module needs it to be: a tap anywhere
    /// in the field focuses it, which is the same contract a
    /// [`Slider`](crate::widgets::slider::Slider) has with its track.
    pub border_width: Property<f32>,
    /// The width of the caret, in pixels.
    pub caret_width: Property<f32>,
    /// The width [`size`](TextInput::size) reports, in pixels.
    ///
    /// A property rather than a bare constant because the operator has already
    /// rejected a sizing number on one control and had it widened the next day on
    /// a sibling with the same number, and the defect that cost both rounds is a
    /// sizing constant with no setter. What would reverse it is a theme token for
    /// a field's width, which does not exist and would change `ThemeToken::all`,
    /// both theme tables and every theme switch for a value a switch does not
    /// change.
    pub width: Property<f32>,
    /// The height [`size`](TextInput::size) reports, in pixels.
    ///
    /// Starts at the 44dp touch floor. **It is not clamped to it**: a caller that
    /// writes a smaller number gets a smaller field, because the floor is this
    /// repository's default for a demo and the head unit's bezel is a different
    /// number that only the operator can supply.
    pub height: Property<f32>,

    /// The character index the caret is at: the truth every edit is applied at.
    caret: Cell<usize>,
    /// Where a selection started, or `None` when there is none. A selection runs
    /// from here to [`caret`](TextInput::caret), in whichever order they fall.
    anchor: Cell<Option<usize>>,
    /// How far through the blink this field is, counted from the start of the
    /// visible phase. Driven by [`tick`](TextInput::tick) and never by a clock
    /// this widget reads, so a test decides the phase by choosing its deltas.
    blink: Cell<Duration>,
    clock: RefCell<AnimationClock>,
    palette: Palette,
    node: Handle,
}

impl TextInput {
    /// Creates a text input in the arena, and returns it.
    ///
    /// The field starts empty, unfocused, with the caret at 0 and the neutral
    /// [`Palette`] colours until a caller gives it a palette of its own and calls
    /// [`snap_to_state`](TextInput::snap_to_state).
    ///
    /// The task file's `TextInput::new() -> Handle` is read as this: the handle
    /// is [`handle`](TextInput::handle)'s, and returning it alone would leave a
    /// caller with no properties to set and no callback to register — a field
    /// with no text and no way to be given any. `Toggle::new` and
    /// `Button::new` settled the same reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// assert!(nodes.get(input.handle()).is_some(), "and it made its own node");
    /// ```
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>) -> Self {
        let palette = Palette::default();
        let node = node::create(nodes, LayoutState::new());
        TextInput {
            text: Property::new(String::new()),
            placeholder: Property::new(String::new()),
            focused: Property::new(false),
            on_change: Callback::none(),
            on_submit: Callback::none(),
            caret_position: Property::new(0.0),
            background: Property::new(palette.surface),
            foreground: Property::new(palette.text),
            border: Property::new(palette.border),
            caret_color: Property::new(palette.caret),
            selection: Property::new(Style::selection_fill(palette.focus, palette.surface)),
            scroll_x: Property::new(0.0),
            font_size: Property::new(DEFAULT_FONT_SIZE),
            line_height: Property::new(DEFAULT_LINE_HEIGHT),
            letter_spacing: Property::new(DEFAULT_LETTER_SPACING),
            padding: Property::new(DEFAULT_PADDING),
            border_radius: Property::new(DEFAULT_CORNER_RADIUS),
            border_width: Property::new(DEFAULT_BORDER_WIDTH),
            caret_width: Property::new(DEFAULT_CARET_WIDTH),
            width: Property::new(DEFAULT_WIDTH),
            height: Property::new(MIN_TOUCH_TARGET),
            caret: Cell::new(0),
            anchor: Cell::new(None),
            blink: Cell::new(Duration::ZERO),
            clock: RefCell::new(AnimationClock::new()),
            palette,
            node,
        }
    }

    /// Returns the field's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the field draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the field draws with, and leaves the current ones where
    /// they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](TextInput::animate_to_state) or
    /// [`snap_to_state`](TextInput::snap_to_state): a theme switch is animated,
    /// and a theme switch is the caller announcing a new palette and then moving
    /// the field toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the field chasing a palette that is still
    /// moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the size a field asks for: [`width`](TextInput::width) by
    /// [`height`](TextInput::height), which start at the module's two defaults.
    ///
    /// A field's width is a decision about the value it holds rather than a
    /// measurement of it, so this is only for a caller that has nothing else to
    /// go on. A caller that lays the field out itself gives the node whatever rect
    /// it wants, and [`paint`](TextInput::paint) draws inside whatever it is
    /// given.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// let size = input.size();
    /// assert_eq!((size.width, size.height), (240.0, 44.0));
    ///
    /// // And a caller that wants a different size writes one, rather than being
    /// // unable to answer a request to change it.
    /// input.width.set(420.0);
    /// input.height.set(64.0);
    /// let size = input.size();
    /// assert_eq!((size.width, size.height), (420.0, 64.0));
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(self.width.get(), self.height.get())
    }

    /// Returns the character index the caret is at, counting characters and not
    /// bytes.
    ///
    /// It is always inside the text: every operation that moves it clamps, so
    /// this is `0..=text.chars().count()` and a byte offset is never returned.
    /// A caller reading a byte offset has to ask for one itself, because the
    /// widget has no reason to know where the text's bytes are.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("äöü");
    /// assert_eq!(input.caret(), 3, "three characters, six bytes");
    /// ```
    #[must_use]
    pub fn caret(&self) -> usize {
        self.caret.get()
    }

    /// Returns the selected range of character indices, or `None` when nothing is
    /// selected.
    ///
    /// The range runs from the anchor to the caret whichever order they fall in,
    /// so dragging leftwards gives a range rather than a reversed one. An anchor
    /// that has caught up with the caret is no selection, and neither is a field
    /// that has never been dragged: an empty selection draws nothing and is
    /// `None` here, so a caller does not have to test for an empty range of its
    /// own.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("hello");
    /// assert_eq!(input.selection(), None);
    ///
    /// input.set_selection(1, 4);
    /// assert_eq!(input.selection(), Some(1..4));
    ///
    /// // And leftwards: the range is the same, the anchor is the other end.
    /// input.set_selection(4, 1);
    /// assert_eq!(input.selection(), Some(1..4));
    /// ```
    #[must_use]
    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor.get()?;
        let caret = self.caret.get();
        let start = anchor.min(caret);
        let end = anchor.max(caret);
        (start < end).then_some(start..end)
    }

    /// Returns the width the field's text occupies, in pixels, with no
    /// truncation: the whole string's advances plus the letter spacing after
    /// each character.
    ///
    /// This is the number a horizontal scroll is measured against — the widest
    /// the content can be — and it is *not* the width of a line on screen, which
    /// is the field's inner width. The two are equal exactly when nothing needs
    /// scrolling.
    #[must_use]
    pub fn content_width(&self, advance: &dyn Fn(char) -> f32) -> f32 {
        let text = self.text.get();
        offset_of(
            &text,
            text.chars().count(),
            self.letter_spacing.get(),
            advance,
        )
    }

    /// Returns the x the caret is drawn at, inside `rect`.
    ///
    /// It follows the *drawn* caret position rather than
    /// [`caret`](TextInput::caret), so a caller asking mid-transition is told
    /// where the caret is and not where it is going — the only version of the
    /// question a test about the animation can ask. The offset is in window
    /// coordinates, because that is what a [`DrawCommand::Text`]'s `x` is, and
    /// the width of the caret is not included: this is the caret's leading edge.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("abc");
    ///
    /// // The field starts at (120, 64) with a 2-pixel border and 8 of padding,
    /// // so its text starts at 130, and a 10-pixel advance puts the caret after
    /// // two characters at 150.
    /// let rect = Rect::new(120.0, 64.0, 200.0, 44.0);
    /// input.move_caret(2);
    /// input.snap_to_state();
    /// assert_eq!(input.caret_x(rect, &|_: char| 10.0), 150.0);
    /// ```
    #[must_use]
    pub fn caret_x(&self, rect: Rect, advance: &dyn Fn(char) -> f32) -> f32 {
        let text = self.text.get();
        self.caret_x_in(&text, rect, advance)
    }

    /// Gives the field focus and restarts the blink at its visible phase.
    ///
    /// The blink restart is the reason this exists rather than a write to
    /// [`focused`](TextInput::focused): a field that took focus half way through
    /// its hidden phase would show no caret for the rest of that phase, and a
    /// caller that wrote the property directly would get exactly that.
    ///
    /// Gaining focus does not move the caret, and does not clear a selection: a
    /// field can be focused with a word already selected, which is what a
    /// long-press select is.
    pub fn focus(&self) {
        self.focused.set(true);
        self.restart_blink();
    }

    /// Removes the field's focus, and stops the blink with it.
    ///
    /// The caret index is kept, so a field that is focused again comes back with
    /// its insertion point where it was.
    pub fn blur(&self) {
        self.focused.set(false);
    }

    /// Moves the caret to character `index`, and collapses any selection onto it.
    ///
    /// The index counts characters, is clamped to the text's length, and is the
    /// *truth* the caret is at — [`caret_position`](TextInput::caret_position),
    /// the position the caret is **drawn** at, is left where it is. That split is
    /// the [`Slider`](crate::widgets::slider::Slider)'s, between `value` and
    /// `thumb`, and it is where requirement 5's "the cursor position animates
    /// when it moved" lives: a caller follows this with
    /// [`animate_to_state`](TextInput::animate_to_state) and the caret slides to
    /// where it went, or with [`snap_to_state`](TextInput::snap_to_state) and it
    /// arrives at once. Any transition already running is cleared, so it cannot
    /// write over the new one when it arrives.
    ///
    /// This is the *programmatic* move, and it defers for the reason an
    /// interaction does not: a caller that changed the value from code wants the
    /// caret to travel there, while a caller that pressed a key wants the caret
    /// to be under the key. A key press therefore does not come through here —
    /// it comes through [`on_event`](TextInput::on_event), which settles the
    /// drawn caret itself. An index past the end of the text is the end of it,
    /// and one that is no index at all cannot be expressed, since it is a
    /// `usize`.
    ///
    /// The blink is restarted. A caret that arrives somewhere the user did not
    /// put it should be visible when it gets there rather than landing in the
    /// hidden phase. The scroll is *not* moved: this has no rect to measure
    /// against, and [`paint`](TextInput::paint) brings the caret into view on
    /// the next frame instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("hello");
    ///
    /// input.move_caret(2);
    /// assert_eq!(input.caret(), 2);
    /// input.move_caret(99);
    /// assert_eq!(input.caret(), 5, "past the end is the end");
    /// ```
    pub fn move_caret(&self, index: usize) {
        let count = self.text.get().chars().count();
        self.caret.set(index.min(count));
        self.anchor.set(None);
        self.restart_blink();
        self.clock.borrow_mut().clear();
    }

    /// Selects the characters in `anchor..caret`, or the range between the two in
    /// whichever order they are given.
    ///
    /// Both indices are clamped to the text's length, as every other index here
    /// is, and neither is moved to a character boundary by the caller: they are
    /// character counts, so there is no boundary to miss. An anchor that equals
    /// the caret is no selection.
    ///
    /// The drawn caret is put on the new index at once, because a selection is
    /// drawn from *both* ends and a caret left travelling would show one end of
    /// the selection and not the other. This is what a drag does, through
    /// [`on_event`](TextInput::on_event); it is public because it is the other
    /// thing a selection can come from — a double tap, a long press, or a caller
    /// driving a selection of its own.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("hello");
    ///
    /// input.set_selection(0, 2);
    /// assert_eq!(input.selection(), Some(0..2));
    ///
    /// // Typing over a selection replaces it.
    /// input.insert_text("HE");
    /// assert_eq!(input.text.get(), "HEllo");
    /// assert_eq!(input.caret(), 2, "and the caret is after what went in");
    /// assert_eq!(input.selection(), None, "and the selection is gone");
    /// ```
    pub fn set_selection(&self, anchor: usize, caret: usize) {
        let count = self.text.get().chars().count();
        self.caret.set(caret.min(count));
        self.anchor.set(Some(anchor.min(count)));
        self.restart_blink();
        self.clock.borrow_mut().clear();
        self.settle_caret();
    }

    /// Inserts `text` at the caret, replacing the selection if there is one, and
    /// leaves the caret after what went in.
    ///
    /// This is the route both [`on_event`](TextInput::on_event)'s
    /// [`InputEventKind::Text`] and an on-screen keyboard take. It is public
    /// because requirement 4's keyboard is a separate widget whose key caps are
    /// not key presses: a cap labelled `ü` has to insert `ü` whether or not a
    /// keyboard with that key exists, which is the whole reason
    /// [`InputEventKind::Text`] exists and the whole reason this is not derived
    /// from a keycode.
    ///
    /// It does not check [`focused`](TextInput::focused). The keyboard is a
    /// caller that has already been told which field it is for, and a public
    /// editing method that refused to edit would be a second rule for it to
    /// learn; the focus gate belongs on the event route, where it is.
    ///
    /// The blink restarts, so a typed character is always followed by a visible
    /// caret: one that happens to land in the hidden phase would look like the
    /// keystroke was dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("world");
    /// input.move_caret(0);
    /// input.snap_to_state();
    /// input.insert_text("hello ");
    /// assert_eq!(input.text.get(), "hello world");
    ///
    /// // A whole IME run arrives as one string, and goes in as one.
    /// input.move_caret(11);
    /// input.snap_to_state();
    /// input.insert_text("日本語");
    /// assert_eq!(input.text.get(), "hello world日本語");
    /// assert_eq!(input.caret(), 14);
    /// ```
    pub fn insert_text(&self, text: &str) {
        let (from, to) = self.selected_or_caret();
        self.replace(from, to, text);
    }

    /// Deletes the character before the caret, or the selection if there is one.
    ///
    /// The public half of requirement 3's "Backspace deletes character before
    /// cursor" and the other half of what an on-screen keyboard's backspace cap
    /// needs. At the start of the text there is nothing before the caret, nothing
    /// is written, and [`on_change`](TextInput::on_change) does not fire: a
    /// backspace that changes nothing is not a change.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("naïve");
    ///
    /// input.delete_backward();
    /// assert_eq!(input.text.get(), "naïv", "a whole character, not a byte");
    /// input.delete_backward();
    /// assert_eq!(input.text.get(), "naï", "including a two-byte one");
    /// ```
    pub fn delete_backward(&self) {
        match self.selection() {
            Some(selection) => self.replace(selection.start, selection.end, ""),
            None => {
                let caret = self.caret.get();
                if caret > 0 {
                    self.replace(caret - 1, caret, "");
                }
            }
        }
    }

    /// Deletes the character after the caret, or the selection if there is one.
    ///
    /// The forward counterpart of [`delete_backward`](TextInput::delete_backward),
    /// and the other half of what an on-screen keyboard's delete cap needs. At the
    /// end of the text there is nothing after the caret, and nothing is written.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("abcd");
    /// input.move_caret(0);
    ///
    /// input.delete_forward();
    /// assert_eq!(input.text.get(), "bcd", "and the caret did not move");
    /// assert_eq!(input.caret(), 0);
    /// ```
    pub fn delete_forward(&self) {
        match self.selection() {
            Some(selection) => self.replace(selection.start, selection.end, ""),
            None => {
                let caret = self.caret.get();
                let count = self.text.get().chars().count();
                if caret < count {
                    self.replace(caret, caret + 1, "");
                }
            }
        }
    }

    /// Returns the appearance the field's state implies.
    ///
    /// The border is the focus colour while the field holds focus and the border
    /// token otherwise, applied independently of everything else, and the
    /// selection's fill is the surface blended toward the focus token. Everything
    /// else is the palette unchanged.
    #[must_use]
    pub fn style(&self) -> Style {
        let focused = self.focused.get();
        let border = if focused {
            self.palette.focus
        } else {
            self.palette.border
        };
        Style {
            surface: self.palette.surface,
            text: self.palette.text,
            border,
            caret: self.palette.caret,
            selection: Style::selection_fill(self.palette.focus, self.palette.surface),
        }
    }

    /// Applies the appearance the field's state implies at once, with no
    /// transition, and puts the drawn caret on the caret.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a field that has just been given a
    /// [`Palette`](TextInput::set_palette) and has never animated — whose colour
    /// properties still hold the neutral defaults [`TextInput::new`] wrote, so
    /// without this a themed field starts out grey — and a caller that has
    /// written a property itself and wants the field to be that state now.
    ///
    /// Any transition already running is cleared first, so it cannot write over
    /// what this just set when it arrives. The blink is not touched: it is a
    /// phase, not a state, and snapping it would make a field blink from wherever
    /// it happened to be.
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.caret_position.set(position_of(
            self.caret.get(),
            self.text.get().chars().count(),
        ));
        self.background.set(style.surface);
        self.foreground.set(style.text);
        self.border.set(style.border);
        self.caret_color.set(style.caret);
        self.selection.set(style.selection);
    }

    /// Starts the transitions that carry the field from wherever it is toward the
    /// appearance [`style`](TextInput::style) implies, on `motion`.
    ///
    /// The drawn caret is part of what this aims, and it is the reason the field
    /// is split into a truth and a drawing: an arrow key moves [`caret`](TextInput::caret)
    /// and the caller re-aims, and the caret slides to where it went. The clock
    /// is cleared first, so a transition this replaces stops where it is rather
    /// than writing over the new one when it arrives.
    ///
    /// The target is a snapshot, not a continuous one: a caller re-aims when its
    /// own state moves, exactly as the demo re-aims a button's press when the
    /// theme moves under it.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// input.focus();
    /// input.insert_text("hello");
    /// let rect = Rect::new(120.0, 64.0, 200.0, 44.0);
    ///
    /// // The drawn caret is put at the start, and then told to go to the end
    /// // without going there: the truth moves, the drawing waits for the
    /// // transition.
    /// input.move_caret(0);
    /// input.snap_to_state();
    /// assert_eq!(input.caret_x(rect, &|_: char| 10.0), 130.0);
    ///
    /// input.move_caret(5);
    /// input.animate_to_state(Motion {
    ///     duration: Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    /// assert_eq!(input.caret_x(rect, &|_: char| 10.0), 130.0, "still where it was");
    /// input.tick(Duration::from_millis(50));
    /// assert_eq!(input.caret_x(rect, &|_: char| 10.0), 155.0, "half way there");
    /// input.tick(Duration::from_millis(50));
    /// assert_eq!(input.caret_x(rect, &|_: char| 10.0), 180.0, "and arrived");
    /// ```
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let count = self.text.get().chars().count();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(self.caret_position.animate_to(
            position_of(self.caret.get(), count),
            motion.duration,
            motion.easing,
        ));
        clock.add(
            self.background
                .animate_to(style.surface, motion.duration, motion.easing),
        );
        clock.add(
            self.foreground
                .animate_to(style.text, motion.duration, motion.easing),
        );
        clock.add(
            self.border
                .animate_to(style.border, motion.duration, motion.easing),
        );
        clock.add(
            self.caret_color
                .animate_to(style.caret, motion.duration, motion.easing),
        );
        clock.add(
            self.selection
                .animate_to(style.selection, motion.duration, motion.easing),
        );
    }

    /// Advances the field's transitions and its blink by `delta`, and returns
    /// whether anything that is drawn has changed.
    ///
    /// It is the field's frame integration: call it once a frame, before the
    /// paint pass, with the time that frame took. It is the `delta` that drives
    /// the blink, and never a clock this widget reads — `AGENTS.md` forbids a
    /// wall-clock test, and a blink that read one could not be tested at all.
    ///
    /// It is true on every frame of a transition and on every frame the blink's
    /// phase *flips* on, which is twice a second rather than sixty times: a
    /// caller that repaints only when this is true repaints exactly while
    /// something a viewer can see has moved.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        let mut changed = self.clock.borrow_mut().tick(delta);
        let was_visible = self.blink_visible();
        self.blink.set(self.blink.get().saturating_add(delta));
        if self.focused.get() && was_visible != self.blink_visible() {
            changed = true;
        }
        changed
    }

    /// Returns whether the field's transitions are still running.
    ///
    /// The blink is not one of them, and that is not an oversight: it is a phase
    /// that runs for as long as the field holds focus, so asking whether it is
    /// animating is asking whether the field is focused.
    /// [`tick`](TextInput::tick) reports the flips.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Returns whether the caret is drawn on this frame: the field holds focus
    /// and the blink is in its visible phase.
    ///
    /// A field that is not focused draws no caret at all, so the answer is
    /// `false` for one however far through its blink it is — the two questions
    /// are "should there be a caret" and "is this one showing", and only the
    /// second is a phase.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// assert!(!input.caret_visible(), "an unfocused field shows no caret");
    ///
    /// input.focus();
    /// assert!(input.caret_visible(), "and focusing starts the visible phase");
    /// input.tick(Duration::from_millis(500));
    /// assert!(!input.caret_visible(), "500ms in, the caret is hidden");
    /// input.tick(Duration::from_millis(500));
    /// assert!(input.caret_visible(), "and 500ms after that, it is back");
    /// ```
    #[must_use]
    pub fn caret_visible(&self) -> bool {
        self.focused.get() && self.blink_visible()
    }

    /// Handles `event` as this field would inside `rect`, and reports whether it
    /// consumed it.
    ///
    /// `advance` measures one character in the field's font, the same seam
    /// [`label::layout_text`](crate::widgets::label::layout_text) takes, and the
    /// same one [`paint`](TextInput::paint) takes. The two have to agree: the
    /// tap below places the caret from the pointer's x through the same
    /// `inner_rect` the text is drawn into, so a caret that lands where the
    /// finger is cannot drift from where the text is.
    ///
    /// `rect` is needed because a node cannot reach the arena that holds it: the
    /// field has no way to ask where it was laid out, so the caller that knows
    /// passes it. That is the same contract the
    /// [`Slider`](crate::widgets::slider::Slider) has.
    ///
    /// A [`Tap`](InputEventKind::Tap) focuses the field and puts the caret at the
    /// character boundary nearest the pointer's x, and is consumed whether or
    /// not that moved anything — a tap on a field already focused with the caret
    /// already there is still a tap on the field, and letting it through would
    /// deliver it to whatever is behind. A tap's y is not read: the caller's hit
    /// test has already decided the pointer is over this field, and a single-line
    /// field has one line to be over.
    ///
    /// A [`Drag`](InputEventKind::Drag) extends the selection from the anchor,
    /// with no anchor set by a drag alone, so a drag in a field that was not
    /// selected moves the caret rather than selecting nothing. It is also
    /// consumed, because it is the second half of the tap that started it.
    ///
    /// A [`Text`](InputEventKind::Text) inserts the run it carries, replacing the
    /// selection if there is one. Every key this field uses — Backspace, Delete,
    /// Left, Right, Home, End, Return, and the d-pad's left and right — acts
    /// **only while the field holds focus**. A key press is not routed by
    /// position: it goes to `root` and is offered to whatever the caller calls
    /// [`on_event`](TextInput::on_event) on, and every arrow would otherwise move
    /// every field on screen. The d-pad is in the same match as the arrow keys
    /// for the reason the slider's own `adjustment` gives: the input module
    /// already maps a gamepad's buttons into [`Key`], and the d-pad is the four
    /// directions a controller has.
    ///
    /// **A caret that moves from a key or a tap is drawn there at once**, and not
    /// animated: the finger or the key owns where it is, and a caret travelling
    /// toward it is one the user has already passed. The animation of
    /// requirement 5 is on the other path — [`move_caret`](TextInput::move_caret)
    /// followed by [`animate_to_state`](TextInput::animate_to_state) — which is
    /// the one a caller takes when it moved the caret from code rather than from
    /// a gesture.
    ///
    /// A key the field does not use is left alone and not consumed, so it carries
    /// on up the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind, Key};
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::text_input::TextInput;
    ///
    /// let mut nodes = Arena::new();
    /// let input = TextInput::new(&mut nodes);
    /// let rect = Rect::new(0.0, 0.0, 100.0, 44.0);
    ///
    /// // Keys only reach a focused field.
    /// let mut backspace = InputEvent::new(
    ///     InputEventKind::KeyDown {
    ///         key: Key::Keyboard(sdl3::keyboard::Keycode::Backspace),
    ///         keymod: sdl3::keyboard::Mod::empty(),
    ///     },
    ///     None,
    /// );
    /// assert!(!input.on_event(&mut backspace, rect, &|_: char| 10.0));
    /// assert!(!backspace.consumed(), "and lets it travel on");
    ///
    /// input.focus();
    /// assert!(input.on_event(&mut backspace, rect, &|_: char| 10.0));
    /// assert!(backspace.consumed());
    /// ```
    pub fn on_event(
        &self,
        event: &mut InputEvent,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
    ) -> bool {
        match event.kind() {
            InputEventKind::Tap => {
                event.consume();
                self.focus();
                if let Some(position) = event.position() {
                    self.caret_at(position.x, rect, advance);
                }
                true
            }
            InputEventKind::Drag { .. } => {
                event.consume();
                if let Some(position) = event.position() {
                    // A drag with no anchor yet is a caret move, not a selection
                    // of nothing: the anchor is wherever the caret was when the
                    // drag started, so the first drag extends from there and a
                    // second one extends from the first.
                    let anchor = self.anchor.get().unwrap_or_else(|| self.caret.get());
                    self.set_selection(anchor, self.index_under(position.x, rect, advance));
                }
                true
            }
            InputEventKind::Text { text } => {
                if !self.focused.get() {
                    return false;
                }
                event.consume();
                self.insert_text(&text);
                true
            }
            InputEventKind::KeyDown { key, .. } => self.on_key(&key, rect, advance, event),
            _ => false,
        }
    }

    /// Returns the draw commands that paint the field within `rect`.
    ///
    /// The commands are, in order: the border, a rounded rectangle in the
    /// border's colour at the field's own rect; the surface, the same shape inset
    /// by the border's width in the background's colour; the selection, if there
    /// is one, as a rounded rectangle in the selection's colour; exactly one
    /// text run, which is the text or the placeholder; and the caret, as a
    /// rectangle `DEFAULT_CARET_WIDTH` wide, if the field is focused and the
    /// blink is in its visible phase.
    ///
    /// **The order is the point of all of it.** The surface is recorded *after*
    /// the border, so the border reads as an outline: a
    /// [`DrawCommand::RoundedRect`] fills its rect, so a border drawn on its own
    /// is a second background, and a border that the surface does not cover is
    /// the defect `.ai/NEVERAGAIN.md` records as *a filled rounded rectangle is
    /// not an outline*. The text is recorded after the selection, so the
    /// selection is a highlight behind the text rather than a box over it. The
    /// caret is last, because a caret is over everything.
    ///
    /// There is exactly one text run whatever the text's length, because the run
    /// carries no width and therefore cannot be trimmed: what is not visible is
    /// not recorded. The run holds only the characters that fit entirely inside
    /// the inner box, and its `x` is the first of them, not the inner box's left
    /// edge — so a scrolled field is not drawn with a run that starts off the
    /// left of the text. See the module doc for what that costs.
    ///
    /// The scroll is reconciled here as well as in
    /// [`on_event`](TextInput::on_event), so a caret moved by
    /// [`move_caret`](TextInput::move_caret) — which has no rect to measure
    /// against — is in view on the next frame. A paint therefore writes
    /// [`scroll_x`](TextInput::scroll_x), and through it the caller's `on_change`,
    /// only when the scroll actually moves.
    #[must_use]
    pub fn paint(&self, rect: Rect, advance: &dyn Fn(char) -> f32) -> Vec<DrawCommand> {
        let text = self.text.get();
        self.ensure_caret_visible(&text, rect, advance);

        let spacing = self.letter_spacing.get();
        let border_width = self.border_width.get().max(0.0);
        let radius = self.border_radius.get().max(0.0);
        let surface = grow(rect, -border_width);
        let inner = self.inner_rect(rect);

        let mut painter = Painter::new();
        painter.rounded_rect(rect, radius, self.border.get());
        painter.rounded_rect(
            surface,
            (radius - border_width).max(0.0),
            self.background.get(),
        );

        if let Some(selection) = self.selection() {
            let from = offset_of(&text, selection.start, spacing, advance);
            let to = offset_of(&text, selection.end, spacing, advance);
            let scroll = self.scroll_x.get();
            let left = inner.x + (from - scroll).max(0.0);
            let right = inner.x + to - scroll;
            let width = (right - left).max(0.0);
            if width > 0.0 {
                painter.rounded_rect(
                    Rect::new(left, inner.y, width, inner.height),
                    selection_radius(width, inner.height),
                    self.selection.get(),
                );
            }
        }

        let empty = text.is_empty();
        let (run, colour) = if empty {
            (self.placeholder.get(), self.palette.muted)
        } else {
            (text.clone(), self.foreground.get())
        };
        let scroll = self.scroll_x.get();
        let (line, first) = visible_run(&run, scroll, scroll + inner.width, spacing, advance);
        let line_height = self.line_height.get();
        painter.text(
            inner.x + first - scroll,
            inner.y + (inner.height - line_height) / 2.0,
            &line,
            colour,
            self.font_size.get(),
            spacing,
        );

        if self.caret_visible() {
            let width = self.caret_width.get().max(0.0);
            painter.rect(
                Rect::new(
                    self.caret_x_in(&text, rect, advance),
                    inner.y,
                    width,
                    inner.height,
                ),
                self.caret_color.get(),
            );
        }

        painter.finish()
    }

    /// Handles the key press `key` while the field holds focus, and reports
    /// whether it consumed the event.
    fn on_key(
        &self,
        key: &Key,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        event: &mut InputEvent,
    ) -> bool {
        if !self.focused.get() {
            return false;
        }
        let caret = self.caret.get();
        match key {
            Key::Keyboard(Keycode::Backspace) => {
                event.consume();
                self.delete_backward();
                true
            }
            Key::Keyboard(Keycode::Delete) => {
                event.consume();
                self.delete_forward();
                true
            }
            Key::Keyboard(Keycode::Left) | Key::Gamepad(GamepadButton::DPadLeft) => {
                event.consume();
                self.step_caret(caret.saturating_sub(1), rect, advance);
                true
            }
            Key::Keyboard(Keycode::Right) | Key::Gamepad(GamepadButton::DPadRight) => {
                event.consume();
                self.step_caret(caret + 1, rect, advance);
                true
            }
            Key::Keyboard(Keycode::Home) => {
                event.consume();
                self.step_caret(0, rect, advance);
                true
            }
            Key::Keyboard(Keycode::End) => {
                event.consume();
                self.step_caret(self.text.get().chars().count(), rect, advance);
                true
            }
            Key::Keyboard(Keycode::Return) | Key::Keyboard(Keycode::KpEnter) => {
                event.consume();
                self.on_submit.call(self.text.get());
                true
            }
            _ => false,
        }
    }

    /// Returns the box the text is drawn in and a pointer's x is measured
    /// against: the field's own rect, inset by the border and then by the
    /// padding.
    ///
    /// Both of its uses read the same numbers, which is the point of it being
    /// one function: the text is drawn into it and the tap places the caret from
    /// a position inside it, so a caret cannot land somewhere the text is not.
    fn inner_rect(&self, rect: Rect) -> Rect {
        let inset = self.border_width.get().max(0.0) + self.padding.get().max(0.0);
        grow(rect, -inset)
    }

    /// Returns the x the caret is drawn at, from `text` rather than from the
    /// property, for the paths that have already read the text.
    fn caret_x_in(&self, text: &str, rect: Rect, advance: &dyn Fn(char) -> f32) -> f32 {
        let inner = self.inner_rect(rect);
        let offset = offset_at_position(
            text,
            self.caret_position.get(),
            self.letter_spacing.get(),
            advance,
        );
        inner.x + offset - self.scroll_x.get()
    }

    /// Returns the character index a pointer at `x` is asking for, measured
    /// against [`inner_rect`](TextInput::inner_rect) and the current scroll.
    ///
    /// The one place a pointer's x becomes a character index, so the tap that
    /// places the caret and the drag that extends the selection cannot disagree
    /// about what a position means — and both are the same walk the paint uses,
    /// against the same box the text is drawn in.
    fn index_under(&self, x: f32, rect: Rect, advance: &dyn Fn(char) -> f32) -> usize {
        index_beside(
            &self.text.get(),
            x - self.inner_rect(rect).x + self.scroll_x.get(),
            self.letter_spacing.get(),
            advance,
        )
    }

    /// Puts the caret at the character boundary nearest a pointer at `x`, and
    /// brings it into view.
    ///
    /// The y is not read: the caller's hit test has already decided the pointer
    /// is over the field, and a single-line field has one line to be over.
    fn caret_at(&self, x: f32, rect: Rect, advance: &dyn Fn(char) -> f32) {
        self.step_caret(self.index_under(x, rect, advance), rect, advance);
    }

    /// Returns the range an insertion replaces: the selection if there is one,
    /// and the caret's own empty range if there is not.
    fn selected_or_caret(&self) -> (usize, usize) {
        match self.selection() {
            Some(selection) => (selection.start, selection.end),
            None => (self.caret.get(), self.caret.get()),
        }
    }

    /// Replaces the characters in `from..to` with `text`, leaves the caret after
    /// what went in, and fires [`on_change`](TextInput::on_change) if the field's
    /// text moved.
    ///
    /// Every edit goes through here, which is what makes "on_change fires only
    /// when the text moved" a property of one function rather than of each
    /// caller. The two indices are character counts, clamped and ordered against
    /// each other first, so every slice below is between two character
    /// boundaries — there is no `&text[..i]` anywhere in this module that could
    /// land inside a character.
    fn replace(&self, from: usize, to: usize, text: &str) {
        let current = self.text.get();
        let count = current.chars().count();
        let from = from.min(count);
        let to = to.min(count).max(from);

        let mut next = String::with_capacity(current.len() + text.len());
        next.push_str(&current[..byte_index(&current, from)]);
        next.push_str(text);
        next.push_str(&current[byte_index(&current, to)..]);

        let caret = from + text.chars().count();
        self.caret.set(caret);
        self.anchor.set(None);
        self.restart_blink();
        self.clock.borrow_mut().clear();

        if next != current {
            self.text.set(next.clone());
            self.on_change.call(next);
        }
        // After the callback, so a handler that reads the caret sees the text it
        // was given and not the one before it.
        self.settle_caret();
    }

    /// Puts the drawn caret on the truth at once, with no transition.
    ///
    /// This is the half of the split that an *interaction* uses and a
    /// programmatic move does not: a finger or a key owns where the caret is,
    /// and a caret that travels toward it is a caret the user has already
    /// passed — the argument the [`Slider`](crate::widgets::slider::Slider)
    /// makes for writing its thumb at once and only animating it when the
    /// *value* was changed from code.
    fn settle_caret(&self) {
        self.caret_position.set(position_of(
            self.caret.get(),
            self.text.get().chars().count(),
        ));
    }

    /// Moves the caret to character `index`, puts the drawn caret there at once,
    /// and brings it into view — the whole of an interaction, for the key route
    /// and the tap route alike.
    fn step_caret(&self, index: usize, rect: Rect, advance: &dyn Fn(char) -> f32) {
        self.move_caret(index);
        self.settle_caret();
        self.reconcile_scroll(rect, advance);
    }

    /// Moves the scroll so the caret is inside the field, and reports whether it
    /// moved.
    ///
    /// The caret is inside when its content position lies within
    /// `scroll..=scroll + width`, and the scroll moves by the *least* amount that
    /// puts it there: a caret that has run off the right edge brings the edge to
    /// it, and one the field has been scrolled past brings the scroll back to it.
    /// Anything else leaves the scroll alone, so typing a character that is
    /// already on screen does not move the text out from under the reader.
    ///
    /// The result is also clamped to `0..=content - width`, which is what stops a
    /// field whose text is shorter than itself from scrolling into empty space
    /// and drawing a gap between its text and its own padding.
    fn ensure_caret_visible(&self, text: &str, rect: Rect, advance: &dyn Fn(char) -> f32) -> bool {
        let inner = self.inner_rect(rect);
        let spacing = self.letter_spacing.get();
        // The caret's own width comes out of the room there is, so that the *far*
        // edge of the caret and not the near one is what the field keeps inside
        // it: a caret half out of the field is a caret that cannot be seen.
        let width = (inner.width - self.caret_width.get().max(0.0)).max(0.0);
        let caret = offset_at_position(text, self.caret_position.get(), spacing, advance);
        let content = offset_of(text, text.chars().count(), spacing, advance);
        let scroll = self.scroll_x.get();
        let wanted = if caret > scroll + width {
            caret - width
        } else if caret < scroll {
            caret
        } else {
            scroll
        };
        let next = wanted.clamp(0.0, (content - width).max(0.0));
        let moved = (next - scroll).abs() > f32::EPSILON;
        if moved {
            self.scroll_x.set(next);
        }
        moved
    }

    /// Reconciles the scroll against the rect and font this widget was given,
    /// for the paths that have both.
    fn reconcile_scroll(&self, rect: Rect, advance: &dyn Fn(char) -> f32) {
        let text = self.text.get();
        self.ensure_caret_visible(&text, rect, advance);
    }

    /// Puts the blink back at the start of its visible phase.
    ///
    /// Every interaction that moves the caret goes through it, for the reason
    /// [`insert_text`](TextInput::insert_text) gives: a keystroke whose caret
    /// happened to land in the hidden phase looks like a keystroke that was
    /// dropped.
    fn restart_blink(&self) {
        self.blink.set(Duration::ZERO);
    }

    /// Returns whether the blink is in its visible phase, regardless of focus.
    ///
    /// The remainder is taken in nanoseconds, which is the unit
    /// [`Duration::as_nanos`] gives for both sides and the only one in which the
    /// arithmetic is exact: `Duration` has no `Rem`, and a modulo over a smaller
    /// unit would round a phase boundary to whatever that unit could hold. A
    /// blink whose period is a whole number of milliseconds is not a blink whose
    /// period is *nearly* a whole number of milliseconds.
    fn blink_visible(&self) -> bool {
        self.blink.get().as_nanos() % blink_period().as_nanos() < BLINK_VISIBLE.as_nanos()
    }
}

/// Returns the character index `index` as a fractional position on a walk of
/// `count` characters.
///
/// A caret is a *position*, and a position between two characters is a number
/// between two integers — so the drawn caret is a `f32` and the truth is a
/// `usize`, and this is the crossing between them. It counts rather than casts,
/// because std has no `From` or `TryFrom` from `usize` to `f32` and a
/// saturating `as` would be the second documented cast in the repository for a
/// conversion this does in four lines.
///
/// Adding `1.0` is exact for every count below `2^24`, which is 16 million
/// characters — a text field that long is a file, not a field. `index` is
/// clamped, so this never returns more than `count`, and the two are equal for
/// every whole index.
fn position_of(index: usize, count: usize) -> f32 {
    let mut position = 0.0_f32;
    for _ in 0..index.min(count) {
        position += 1.0;
    }
    position
}

/// Returns the x offset, from the start of `text`, of the caret at the fractional
/// character `position`.
///
/// This is the crossing the other way, and the crossing the other way is *not*
/// a cast either: the position may fall inside a character, and the answer is
/// then part of that character's advance. So the walk stops inside the character
/// it lands in rather than at the next boundary — which is what makes the caret
/// slide continuously between two characters during a transition instead of
/// jumping from one to the next at the halfway point.
///
/// A position that is not inside the text — past its end, negative, or `NaN` —
/// gives the offset of the text's end or of its start, never a number outside
/// it. `NaN` reads as the *start*, and is asked about separately rather than
/// through one comparison: every comparison against `NaN` is false, so a `NaN`
/// position would otherwise walk the whole string and land the caret at the
/// *end*, and a caret at the start of a field is a thing a caller can see and
/// fix while a caret at the far end of a field of news looks like content.
fn offset_at_position(
    text: &str,
    position: f32,
    letter_spacing: f32,
    advance: &dyn Fn(char) -> f32,
) -> f32 {
    if position <= 0.0 || position.is_nan() {
        return 0.0;
    }
    let mut offset = 0.0;
    let mut passed = 0.0_f32;
    for ch in text.chars() {
        if passed >= position {
            break;
        }
        let width = advance(ch) + letter_spacing;
        if passed + 1.0 > position {
            return offset + width * (position - passed);
        }
        offset += width;
        passed += 1.0;
    }
    offset
}

/// Returns the x offset, from the start of `text`, of the character at the whole
/// number `index`.
///
/// The end of a string is its own offset, and it is one letter spacing past the
/// last glyph: [`label`](crate::widgets::label)'s `measure` adds the spacing
/// after every character including the last, so this agrees with it by
/// construction. `index` past the end is the end, and it is clamped rather than
/// trusted because it comes from a pointer.
fn offset_of(text: &str, index: usize, letter_spacing: f32, advance: &dyn Fn(char) -> f32) -> f32 {
    offset_at_position(
        text,
        position_of(index, text.chars().count()),
        letter_spacing,
        advance,
    )
}

/// Returns the character index whose boundary is nearest a pointer at `x`, in the
/// same coordinates the text is laid out in.
///
/// Nearest, rather than "the first character that ends past `x`": a pointer in
/// the middle of a character is asking for the side of it the pointer is on, and
/// a pointer exactly on a boundary is asking for that boundary, so the test is
/// against the character's midpoint and a half is up.
fn index_beside(text: &str, x: f32, letter_spacing: f32, advance: &dyn Fn(char) -> f32) -> usize {
    let mut index = 0_usize;
    let mut offset = 0.0_f32;
    for ch in text.chars() {
        let width = advance(ch) + letter_spacing;
        if offset + width / 2.0 > x {
            break;
        }
        offset += width;
        index += 1;
    }
    index
}

/// Returns the byte offset of the `index`-th character, or the text's own length
/// when `index` is at or past its end.
///
/// This is the only place in the module that turns a character index into
/// something a slice can be cut at, and it is why none of the slices can land
/// inside a character. `char_indices` reports an offset per character and
/// reports none for a position past the end, so `map_or` is what makes "one
/// past the last character" mean the end of the string rather than nothing.
fn byte_index(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(offset, _)| offset)
}

/// Returns the characters of `text` that fit entirely inside `from..to`, in the
/// content coordinates a horizontal scroll works in, and the offset of the first
/// of them from the start of the string.
///
/// This is the clip, and it is a walk rather than a measurement because a
/// [`DrawCommand::Text`] carries no width: a run that is recorded is drawn whole,
/// so what must not be drawn must not be recorded. A character is in the run only
/// if it neither starts before `from` nor ends after `to`, so nothing the field
/// draws is ever drawn over its own border. The cost is stated in the module doc:
/// a gap of up to one advance at the right edge, rather than a glyph over the
/// border.
fn visible_run(
    text: &str,
    from: f32,
    to: f32,
    letter_spacing: f32,
    advance: &dyn Fn(char) -> f32,
) -> (String, f32) {
    let mut run = String::new();
    let mut offset = 0.0_f32;
    let mut first = from;
    for ch in text.chars() {
        let width = advance(ch) + letter_spacing;
        if offset >= to {
            break;
        }
        if offset >= from && offset + width <= to {
            if run.is_empty() {
                first = offset;
            }
            run.push(ch);
        }
        offset += width;
    }
    (run, first)
}

/// Returns the radius of the selection's rounded rectangle.
///
/// A selection is a band of the field's own height, and a pill whose ends are as
/// round as they can be reads as a highlight over the text while a small radius
/// reads as a box drawn round a word. It is capped at half the shorter side,
/// which is what [`DrawCommand::RoundedRect`] does with a radius too large to
/// fit, and stated here so the caller sees the same number the command will.
fn selection_radius(width: f32, height: f32) -> f32 {
    (height / 2.0).min(width / 2.0).max(0.0)
}

/// Returns `rect` grown by `by` on every side. A negative `by` insets it.
///
/// The button's own helper, repeated rather than imported: it is four lines, and
/// a shared module for one four-line helper is a module.
fn grow(rect: Rect, by: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// The slider's own helper, repeated for the reason it gives: a theme's tables
/// keep each token to its own kind, so this is a fallback for a mistyped one —
/// and black rather than a panic, because a mistyped theme token is not worth
/// taking a frame down for.
fn token_color(theme: &Theme, token: crate::theme::ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Easing;
    use crate::input::{hit_test, route};
    use crate::layout::Offset;
    use crate::layout::{Constraints, Layout, LayoutState};
    use crate::theme::ThemeToken;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A monospace advance of 10 pixels per character, spaces included.
    fn mono(_: char) -> f32 {
        10.0
    }

    /// The rect most geometry tests lay a field out in: 200 by 44 at
    /// `(120, 64)`, which is nowhere near the origin.
    ///
    /// Every number in this module is derived from it: the border is 2 wide, the
    /// padding 8, so the field's surface is `(122, 66, 196, 40)` and the box its
    /// text is drawn in is `(130, 74, 180, 24)`. The text's line box is 20 high,
    /// so a run is recorded with its top edge at `74 + (24 - 20) / 2 = 76`. With
    /// [`mono`] and no letter spacing, the character at index `i` is drawn at
    /// `130 + 10 * i` and the whole string is `10 * len` wide.
    ///
    /// It is deliberately **not** at `(0, 0)`. A fixture at the origin cannot
    /// see an origin being read as an extent, and that exact bug shipped in this
    /// repository once already: every slider test in the module laid its slider
    /// at `(0, 0)` and all 53 of them passed while the widget reported a negative
    /// run everywhere but there.
    const FIELD: Rect = Rect {
        x: 120.0,
        y: 64.0,
        width: 200.0,
        height: 44.0,
    };

    /// The same field at the origin, for the one test that is about a field in
    /// the corner of the window.
    const FIELD_AT_ORIGIN: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 200.0,
        height: 44.0,
    };

    /// The left edge of [`FIELD`]'s text.
    const TEXT_X: f32 = 130.0;
    /// The top edge of a run recorded in [`FIELD`].
    const TEXT_Y: f32 = 76.0;
    /// The right edge of [`FIELD`]'s text box.
    const TEXT_RIGHT: f32 = 310.0;

    /// A string 26 characters long, which at [`mono`] is 260 pixels against
    /// [`FIELD`]'s 180 — so it does not fit, and every scroll test needs it.
    const LONG: &str = "abcdefghijklmnopqrstuvwxyz";

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Advances `input` by `millis`, for the tests that only want time to pass
    /// and have no answer to hear.
    ///
    /// The field's own `tick` is what a frame calls, and it answers whether a
    /// repaint is needed; a test that does not care goes through here so that the
    /// answer is dropped on purpose rather than by accident.
    fn tick(input: &TextInput, millis: u64) {
        let _ = input.tick(ms(millis));
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a
    /// value at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// A field in the dark theme's palette, with its colours already arrived at
    /// that palette.
    ///
    /// Setting a palette does not move the field by itself — a theme switch is
    /// animated — so a test that wants a field *in* a palette asks for the
    /// animation and ticks it out, or snaps. Otherwise the colours would be the
    /// ones `TextInput::new` starts with and the palette's would only be targets.
    fn field() -> TextInput {
        let mut nodes = Arena::new();
        let mut input = TextInput::new(&mut nodes);
        input.set_palette(Palette::from_theme(&Theme::dark()));
        input.snap_to_state();
        input
    }

    /// A focused field holding `text`.
    fn typed(text: &str) -> TextInput {
        let input = field();
        input.focus();
        input.insert_text(text);
        input
    }

    /// A key press of `key`, which carries no position.
    fn key_down(keycode: Keycode) -> InputEvent {
        InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(keycode),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        )
    }

    /// A gamepad button press of `button`, which is the d-pad's other name for
    /// the arrow keys.
    fn pad_down(button: GamepadButton) -> InputEvent {
        InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Gamepad(button),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        )
    }

    /// A tap at `(x, y)`.
    fn tap_at(x: f32, y: f32) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)))
    }

    /// A drag ending at `(x, y)`.
    fn drag_to(x: f32, y: f32) -> InputEvent {
        InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(1.0, 0.0),
            },
            Some(Offset::new(x, y)),
        )
    }

    /// A run of typed text, which is what a keystroke and an IME commit both
    /// arrive as.
    fn typed_text(text: &str) -> InputEvent {
        InputEvent::new(
            InputEventKind::Text {
                text: text.to_string(),
            },
            None,
        )
    }

    /// The x, y, text, colour, size and tracking of every text run recorded.
    #[allow(clippy::type_complexity)]
    fn runs(commands: &[DrawCommand]) -> Vec<(f32, f32, String, Color, f32, f32)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    x,
                    y,
                    text,
                    color,
                    font_size,
                    extra_advance,
                    ..
                } => Some((*x, *y, text.clone(), *color, *font_size, *extra_advance)),
                _ => None,
            })
            .collect()
    }

    /// The rounded rectangles recorded, with their radii and colours.
    fn rounded(commands: &[DrawCommand]) -> Vec<(Rect, f32, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color,
                } => Some((*rect, *radius, *color)),
                _ => None,
            })
            .collect()
    }

    /// The plain rectangles recorded, with their colours: the caret, and nothing
    /// else in this widget.
    fn rectangles(commands: &[DrawCommand]) -> Vec<(Rect, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Rect { rect, color } => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// A short name for each recorded command, in the order they were recorded.
    ///
    /// It cannot see sizes, so it cannot tell the field's own rect from its
    /// surface — two `RoundedRect`s are one word to it. The *order* is the half
    /// that matters here, and the sizes are asserted separately against
    /// [`rounded`].
    fn shapes(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::RoundedRect { .. } => "rounded",
                DrawCommand::Rect { .. } => "rect",
                DrawCommand::Text { .. } => "text",
                _ => "other",
            })
            .collect()
    }

    /// The index in `commands` of the first rounded rectangle of `size` at
    /// `rect`, if there is one.
    fn index_of_shape(commands: &[DrawCommand], rect: Rect) -> Option<usize> {
        commands.iter().position(|command| match command {
            DrawCommand::RoundedRect { rect: found, .. } => *found == rect,
            _ => false,
        })
    }

    /// Asserts two values are within a millionth of one another.
    ///
    /// The blend of two colours and the scroll are computed in `f32`, so a test
    /// that wrote the decimal out would be asserting the compiler's rounding
    /// rather than the widget. Everything this module computes exactly — a
    /// character offset, a sum of whole 10-pixel advances — is asserted with
    /// `assert_eq!` instead.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-5, "{what}: {got} against {want}");
    }

    /// A counter of the values the change callback was given, in order.
    fn changes(input: &mut TextInput) -> Rc<RefCell<Vec<String>>> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorded = Rc::clone(&seen);
        input.on_change = Callback::from_fn(move |value| recorded.borrow_mut().push(value));
        seen
    }

    /// A counter of the values the submit callback was given, in order.
    fn submits(input: &mut TextInput) -> Rc<RefCell<Vec<String>>> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorded = Rc::clone(&seen);
        input.on_submit = Callback::from_fn(move |value| recorded.borrow_mut().push(value));
        seen
    }

    /// Hangs `input` on a panel, lays the panel out, and returns the panel's
    /// handle.
    ///
    /// The panel is a stack, so the field sits at its own origin, and the field's
    /// node is given the rect the geometry is asserted against. A tap therefore
    /// has a real hit test to pass or fail, and a node behind the field to reach
    /// if it does not.
    fn on_a_panel(input: &TextInput, nodes: &mut Arena<WidgetNode>, panel_size: Size) -> Handle {
        let panel = node::create(nodes, LayoutState::new());
        assert!(
            node::attach(nodes, panel, input.handle()),
            "the field is hung on the panel"
        );
        if let Some(node) = nodes.get_mut(input.handle()) {
            node.layout_mut()
                .set_constraints(Constraints::tight(input.size()));
        }
        Layout::new(nodes).layout(panel, Constraints::tight(panel_size));
        panel
    }

    #[test]
    fn a_text_input_holds_the_properties_the_task_gives_it() {
        let input = field();
        assert_eq!(input.text.get(), "", "and it starts empty");
        assert_eq!(input.placeholder.get(), "");
        assert!(!input.focused.get(), "unfocused");
        assert!(
            !input.on_change.is_set(),
            "no change callback until one is given"
        );
        assert!(!input.on_submit.is_set(), "and no submit callback either");
        assert_eq!(input.caret(), 0, "with the caret at the start");
        assert_eq!(input.selection(), None, "and nothing selected");
    }

    #[test]
    fn a_text_input_creates_its_own_node_in_the_arena() {
        // The task file's signature returns a `Handle`; the widget returns itself
        // and creates the node here, which is the reading `Button::new` and
        // `Toggle::new` settled. What has to hold either way is that a field has
        // a node, because a node is where it is laid out and painted.
        let mut nodes = Arena::new();
        let input = TextInput::new(&mut nodes);
        let handle = input.handle();
        assert!(nodes.get(handle).is_some(), "and it is in the arena");
        assert_eq!(
            input.handle(),
            handle,
            "and the handle it reports is the same one every time"
        );
    }

    #[test]
    fn the_default_geometry_is_the_numbers_this_module_documents() {
        let input = field();
        assert_eq!(input.size(), Size::new(240.0, 44.0));
        assert_eq!(input.font_size.get(), 16.0);
        assert_eq!(input.line_height.get(), 20.0);
        assert_eq!(input.letter_spacing.get(), 0.0);
        assert_eq!(input.padding.get(), 8.0);
        assert_eq!(input.border_radius.get(), 8.0);
        assert_eq!(input.border_width.get(), 2.0);
        assert_eq!(input.caret_width.get(), 2.0);
        assert_eq!(DEFAULT_WIDTH, 240.0);
        assert_eq!(MIN_TOUCH_TARGET, 44.0);
        assert_eq!(DEFAULT_BORDER_WIDTH, 2.0);
        assert_eq!(DEFAULT_CARET_WIDTH, 2.0, "a thin rectangle, not a bar");
    }

    #[test]
    fn the_size_is_settable_and_size_reads_what_was_written() {
        let input = field();

        // The point of this test is that a *caller* can answer a request to change
        // the field's size. The operator widened a 6-pixel slider track on one day
        // and a 6-pixel scrollbar the next, and both fixes went through a setter;
        // a sizing constant with no door is the defect that cost the second round,
        // and `.ai/NEVERAGAIN.md` records it under *one sibling got the operator's
        // fix*. Reverting `size` to the two constants fails here.
        input.width.set(420.0);
        input.height.set(64.0);
        assert_eq!(input.size(), Size::new(420.0, 64.0));

        // And back down again, including below the 44dp default: the floor is this
        // repository's default, not a rule imposed on a caller whose bezel has a
        // different number.
        input.width.set(180.0);
        input.height.set(30.0);
        assert_eq!(input.size(), Size::new(180.0, 30.0));
    }

    #[test]
    fn a_height_below_the_touch_floor_is_honoured_rather_than_clamped() {
        let input = field();
        input.height.set(20.0);

        // Asserted deliberately, because the tempting alternative — a `.max(44.0)`
        // in `size` — would make a setter that appears to work and does not. If the
        // operator ever wants a real floor here, this is the test that has to
        // change with it.
        assert_eq!(input.size().height, 20.0);
    }

    #[test]
    fn the_palette_is_the_six_tokens_it_names() {
        let dark = Theme::dark();
        let palette = Palette::from_theme(&dark);
        assert_eq!(palette.surface, token_color(&dark, ThemeToken::Surface));
        assert_eq!(palette.text, token_color(&dark, ThemeToken::Text));
        assert_eq!(palette.muted, token_color(&dark, ThemeToken::TextMuted));
        assert_eq!(palette.border, token_color(&dark, ThemeToken::Border));
        assert_eq!(palette.focus, token_color(&dark, ThemeToken::Primary));
        assert_eq!(palette.caret, token_color(&dark, ThemeToken::Text));
        assert_ne!(
            palette.surface,
            token_color(&dark, ThemeToken::Background),
            "and the window's own background is deliberately not the field's \
             surface: a field painted in it would be a border round nothing"
        );
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        let dark = Palette::from_theme(&Theme::dark());
        let light = Palette::from_theme(&Theme::light());
        assert_ne!(dark.surface, light.surface);
        assert_ne!(dark.text, light.text);
        assert_ne!(dark.muted, light.muted);
        assert_ne!(dark.border, light.border);
        assert_ne!(dark.focus, light.focus);
        assert_ne!(dark.caret, light.caret);
    }

    #[test]
    fn snapping_puts_a_themed_field_where_its_theme_says_at_once() {
        // The failure this guards: a field given a palette but never aimed still
        // paints the neutral defaults `TextInput::new` wrote, so a themed field
        // starts out grey. Built from `new` rather than through the `field`
        // helper, which snaps.
        let mut nodes = Arena::new();
        let mut input = TextInput::new(&mut nodes);
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(themed.surface, input.background.get(), "so this can fail");

        input.set_palette(themed);
        input.snap_to_state();

        let style = input.style();
        assert_eq!(input.background.get(), themed.surface);
        assert_eq!(input.foreground.get(), themed.text);
        assert_eq!(input.caret_color.get(), themed.caret);
        assert_eq!(input.border.get(), themed.border, "unfocused, the hairline");
        assert_eq!(input.selection.get(), style.selection);
    }

    #[test]
    fn typing_inserts_at_the_caret() {
        let input = typed("abcd");
        input.set_selection(2, 2);
        let mut event = typed_text("XY");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert!(event.consumed());
        assert_eq!(
            input.text.get(),
            "abXYcd",
            "index 2 of \"abcd\" is after the b, and not the end"
        );
        assert_eq!(input.caret(), 4, "with the caret after what went in");
    }

    #[test]
    fn typing_inserts_at_the_start_of_the_text() {
        let input = typed("bcd");
        input.set_selection(0, 0);
        input.snap_to_state();
        let mut event = typed_text("a");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "abcd");
        assert_eq!(input.caret(), 1);
    }

    #[test]
    fn typing_inserts_at_the_end_of_the_text() {
        let input = typed("abc");
        let mut event = typed_text("d");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "abcd");
        assert_eq!(input.caret(), 4, "at the end, which is where it was");
    }

    #[test]
    fn a_whole_run_goes_in_as_one_keystroke_would_not() {
        // SDL delivers a composition commit as one event carrying a whole word,
        // and a widget that took it a character at a time would fire on_change
        // once per character and animate the caret once per character.
        let input = typed("");
        let mut event = typed_text("日本語");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "日本語");
        assert_eq!(input.caret(), 3, "three characters, nine bytes");
    }

    #[test]
    fn a_run_of_nothing_inserts_nothing_and_reports_nothing() {
        let mut input = typed("ab");
        let seen = changes(&mut input);
        let mut event = typed_text("");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "ab");
        assert!(seen.borrow().is_empty(), "and it is not a change");
    }

    #[test]
    fn typing_in_an_unfocused_field_is_left_alone_and_not_consumed() {
        let input = field();
        let mut event = typed_text("a");
        assert!(
            !input.on_event(&mut event, FIELD, &mono),
            "an unfocused field ignores the run"
        );
        assert!(!event.consumed(), "and lets it travel on");
        assert_eq!(input.text.get(), "", "so nothing was inserted");
    }

    #[test]
    fn an_edit_replaces_the_selection() {
        let input = typed("hello");
        input.set_selection(1, 4);
        let mut event = typed_text("XY");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "hXYo", "\"ell\" is gone");
        assert_eq!(input.caret(), 3, "with the caret after the replacement");
        assert_eq!(input.selection(), None, "and nothing left selected");
    }

    #[test]
    fn an_edit_replaces_a_selection_made_backwards() {
        let input = typed("hello");
        input.set_selection(4, 1);
        let mut event = typed_text("XY");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(
            input.text.get(),
            "hXYo",
            "a backwards drag is the same range"
        );
        assert_eq!(input.caret(), 3);
    }

    #[test]
    fn backspace_deletes_the_character_before_the_caret() {
        let input = typed("abc");
        let mut event = key_down(Keycode::Backspace);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert!(event.consumed());
        assert_eq!(input.text.get(), "ab");
        assert_eq!(input.caret(), 2, "and the caret came with it");
    }

    #[test]
    fn backspace_at_the_start_of_the_text_deletes_nothing() {
        let input = typed("abc");
        input.set_selection(0, 0);
        input.snap_to_state();
        let mut event = key_down(Keycode::Backspace);
        assert!(
            input.on_event(&mut event, FIELD, &mono),
            "and is still consumed"
        );
        assert_eq!(input.text.get(), "abc", "there is nothing before the caret");
        assert_eq!(input.caret(), 0);
    }

    #[test]
    fn backspace_with_a_selection_deletes_the_whole_of_it() {
        let input = typed("hello");
        input.set_selection(1, 4);
        let mut event = key_down(Keycode::Backspace);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "ho");
        assert_eq!(input.caret(), 1, "at the start of what was deleted");
    }

    #[test]
    fn delete_removes_the_character_after_the_caret() {
        let input = typed("abc");
        input.set_selection(0, 0);
        input.snap_to_state();
        let mut event = key_down(Keycode::Delete);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert!(event.consumed());
        assert_eq!(input.text.get(), "bc");
        assert_eq!(input.caret(), 0, "and the caret did not move");
    }

    #[test]
    fn delete_at_the_end_of_the_text_deletes_nothing() {
        let input = typed("abc");
        let mut event = key_down(Keycode::Delete);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "abc", "there is nothing after the caret");
        assert_eq!(input.caret(), 3);
    }

    #[test]
    fn delete_with_a_selection_deletes_the_whole_of_it() {
        let input = typed("hello");
        input.set_selection(1, 4);
        let mut event = key_down(Keycode::Delete);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(input.text.get(), "ho");
        assert_eq!(input.caret(), 1);
    }

    #[test]
    fn backspace_and_delete_on_an_unfocused_field_are_left_alone() {
        let input = field();
        input.insert_text("abc");
        assert!(!input.focused.get(), "and it really is unfocused");
        let mut backspace = key_down(Keycode::Backspace);
        assert!(!input.on_event(&mut backspace, FIELD, &mono));
        assert!(!backspace.consumed());
        let mut delete = key_down(Keycode::Delete);
        assert!(!input.on_event(&mut delete, FIELD, &mono));
        assert!(!delete.consumed());
        assert_eq!(input.text.get(), "abc", "so neither key did anything");
    }

    #[test]
    fn an_arrow_moves_the_caret_one_character() {
        let input = typed("abcd");
        input.set_selection(2, 2);
        let mut right = key_down(Keycode::Right);
        assert!(input.on_event(&mut right, FIELD, &mono));
        assert_eq!(input.caret(), 3);
        let mut left = key_down(Keycode::Left);
        assert!(input.on_event(&mut left, FIELD, &mono));
        assert_eq!(input.caret(), 2);
    }

    #[test]
    fn an_arrow_at_the_start_of_the_text_stays_there() {
        let input = typed("abcd");
        input.set_selection(0, 0);
        let mut left = key_down(Keycode::Left);
        assert!(
            input.on_event(&mut left, FIELD, &mono),
            "and is still consumed, so it does not reach the field behind"
        );
        assert!(left.consumed());
        assert_eq!(input.caret(), 0, "clamped at the start");
    }

    #[test]
    fn an_arrow_at_the_end_of_the_text_stays_there() {
        let input = typed("abcd");
        let mut right = key_down(Keycode::Right);
        assert!(input.on_event(&mut right, FIELD, &mono));
        assert_eq!(input.caret(), 4, "at the end");
        assert!(input.on_event(&mut right, FIELD, &mono));
        assert_eq!(input.caret(), 4, "and clamped there, not past it");
    }

    #[test]
    fn home_and_end_go_to_the_two_ends() {
        let input = typed("abcdef");
        input.set_selection(2, 2);
        let mut end = key_down(Keycode::End);
        assert!(input.on_event(&mut end, FIELD, &mono));
        assert_eq!(input.caret(), 6, "End is the length of the text");
        let mut home = key_down(Keycode::Home);
        assert!(input.on_event(&mut home, FIELD, &mono));
        assert_eq!(input.caret(), 0);
    }

    #[test]
    fn the_d_pad_moves_the_caret_like_the_arrows() {
        let input = typed("abcd");
        input.set_selection(2, 2);
        let mut right = pad_down(GamepadButton::DPadRight);
        assert!(input.on_event(&mut right, FIELD, &mono));
        assert_eq!(input.caret(), 3, "the d-pad right is the right arrow");
        let mut left = pad_down(GamepadButton::DPadLeft);
        assert!(input.on_event(&mut left, FIELD, &mono));
        assert_eq!(input.caret(), 2, "and the d-pad left is the left arrow");
        assert!(right.consumed() && left.consumed(), "both are consumed");
    }

    #[test]
    fn a_caret_moved_by_a_key_is_drawn_where_it_lands() {
        // The interaction half of the split: a key owns where the caret is, so
        // the drawn one is there at once. A caret travelling toward the key is a
        // caret the user has already passed, and a held arrow key would leave it
        // permanently behind.
        let input = typed("abcd");
        input.set_selection(1, 1);
        let mut right = key_down(Keycode::Right);
        input.on_event(&mut right, FIELD, &mono);
        assert_eq!(input.caret(), 2, "the truth moved");
        assert_eq!(
            input.caret_position.get(),
            2.0,
            "and so did the drawing, with no tick in between"
        );
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X + 20.0);
    }

    #[test]
    fn a_caret_moved_from_code_travels_to_the_new_index() {
        // The other half: a move from code is animated, which is requirement 5's
        // "the cursor position animates when it moved".
        let input = typed("hello");
        input.set_selection(0, 0);
        input.snap_to_state();
        assert_eq!(
            input.caret_x(FIELD, &mono),
            TEXT_X,
            "the drawn caret is here"
        );

        input.move_caret(5);
        input.animate_to_state(motion());
        assert_eq!(
            input.caret_x(FIELD, &mono),
            TEXT_X,
            "and stays here until the transition runs"
        );
        assert!(input.tick(ms(50)));
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X + 25.0, "half way");
        assert!(input.tick(ms(50)));
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X + 50.0, "and arrived");
        assert!(!input.is_animating());
    }

    #[test]
    fn a_second_move_replaces_the_first_rather_than_racing_it() {
        // A caret moved twice in quick succession used to be a race: both
        // transitions wrote the drawn position and the older one arriving last
        // left it short of where it should be. The field's own clock is what
        // rules that out.
        let input = typed("abcdefghij");
        input.set_selection(0, 0);
        input.snap_to_state();
        input.move_caret(10);
        input.animate_to_state(motion());
        assert!(input.tick(ms(50)));
        input.move_caret(2);
        input.animate_to_state(motion());
        for _ in 0..4 {
            tick(&input, 50);
        }
        assert_eq!(
            input.caret_position.get(),
            2.0,
            "the drawn caret ends at the newest index"
        );
        assert!(!input.is_animating(), "and nothing is left running");
    }

    #[test]
    fn moving_the_caret_past_the_end_of_the_text_is_the_end() {
        let input = typed("abc");
        input.move_caret(99);
        input.snap_to_state();
        assert_eq!(input.caret(), 3);
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X + 30.0);
    }

    #[test]
    fn the_caret_x_is_the_sum_of_the_advances_before_it() {
        // Every number derived by hand from the closed form — 10 pixels a
        // character — rather than computed from the index, so the assertion is
        // about the widget and not about the arithmetic that produced it.
        let input = typed("hello");
        for (index, offset) in [
            (0_usize, 0.0_f32),
            (1, 10.0),
            (2, 20.0),
            (3, 30.0),
            (4, 40.0),
            (5, 50.0),
        ] {
            input.set_selection(index, index);
            assert_eq!(
                input.caret_x(FIELD, &mono),
                TEXT_X + offset,
                "at character {index}"
            );
        }
    }

    #[test]
    fn the_caret_x_is_measured_from_the_fields_inner_left_edge() {
        // The one number that says where the text starts is `inner_rect`, and it
        // has to be the border and the padding, in that order: 120 + 2 + 8.
        let input = typed("hello");
        input.set_selection(0, 0);
        assert_eq!(input.inner_rect(FIELD), Rect::new(130.0, 74.0, 180.0, 24.0));
        assert_eq!(input.caret_x(FIELD, &mono), 130.0);
    }

    #[test]
    fn the_caret_x_counts_the_letter_spacing() {
        // `label`'s measurement adds the spacing after *every* character, the last
        // one included, so a caret after three characters of a 10-pixel advance
        // with 2 pixels of tracking is at 36 and not 34.
        let input = typed("abc");
        input.letter_spacing.set(2.0);
        input.set_selection(3, 3);
        assert_eq!(input.content_width(&mono), 36.0, "and the text is 36 wide");
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X + 36.0);
    }

    #[test]
    fn the_caret_x_of_an_empty_field_is_the_inner_left_edge() {
        let input = typed("");
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X);
        assert_eq!(input.content_width(&mono), 0.0);
    }

    #[test]
    fn a_field_away_from_the_origin_draws_at_its_own_position() {
        // Every rect in this module is away from the origin for the reason the
        // module doc gives. This is the one that is also at it, so the two ends
        // of the fixture are both covered and a field in the corner of the
        // window is not a special case the suite has never run.
        let input = typed("abc");
        assert_eq!(FIELD.x, 120.0, "the main fixture really is away from it");
        let painted = runs(&input.paint(FIELD_AT_ORIGIN, &mono));
        assert_eq!(painted[0].0, 10.0, "10 = 0 + a 2 border + 8 of padding");
        assert_eq!(painted[0].1, 12.0, "and 12 = 0 + 2 + 8 + (24 - 20) / 2");
    }

    #[test]
    fn a_caret_position_that_is_not_a_number_puts_the_caret_at_the_start() {
        let input = typed("hello");
        input.set_selection(0, 0);
        input.snap_to_state();
        input.caret_position.set(f32::NAN);
        assert_eq!(
            input.caret_x(FIELD, &mono),
            TEXT_X,
            "every comparison against NaN is false, so a walk that did not ask \
             would have landed the caret at the far end of the text"
        );
    }

    #[test]
    fn a_caret_position_past_the_end_of_the_text_is_the_end_of_it() {
        let input = typed("hello");
        input.set_selection(0, 0);
        input.snap_to_state();
        input.caret_position.set(99.0);
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X + 50.0);
    }

    #[test]
    fn backspace_deletes_a_multi_byte_character_whole() {
        // The failure this guards: a byte offset into "naïve" after two
        // characters is 4, and `&text[..4]` is the middle of the ï, which is not
        // a boundary and panics rather than slicing.
        let input = typed("naïve");
        assert_eq!(input.caret(), 5, "five characters, six bytes");
        input.delete_backward();
        assert_eq!(input.text.get(), "naïv", "a whole character, not a byte");
        assert_eq!(input.caret(), 4);
        input.delete_backward();
        assert_eq!(input.text.get(), "naï", "including a two-byte one");
        assert_eq!(input.caret(), 3);
    }

    #[test]
    fn a_multi_byte_character_is_inserted_as_one_character() {
        let input = typed("");
        input.insert_text("äöü");
        assert_eq!(input.text.get(), "äöü");
        assert_eq!(input.caret(), 3, "one character each, six bytes in all");
        input.insert_text("x");
        assert_eq!(input.text.get(), "äöüx");
        assert_eq!(input.caret(), 4);
    }

    #[test]
    fn an_arrow_over_a_multi_byte_character_steps_one_character() {
        let input = typed("äöü");
        let mut left = key_down(Keycode::Left);
        input.on_event(&mut left, FIELD, &mono);
        assert_eq!(input.caret(), 2, "not 5, which would be a byte offset");
        input.on_event(&mut left, FIELD, &mono);
        assert_eq!(input.caret(), 1);
        input.on_event(&mut left, FIELD, &mono);
        assert_eq!(input.caret(), 0);
        assert!(input.on_event(&mut left, FIELD, &mono), "and clamps");
        assert_eq!(input.caret(), 0);
    }

    #[test]
    fn every_slicing_boundary_is_a_character_boundary() {
        // The property the whole multi-byte handling rests on, asserted over
        // every index of a string with two-byte and three-byte characters: the
        // byte offset of each character is a boundary, and the end is one too.
        let text = "aä日";
        assert_eq!(text.chars().count(), 3);
        let boundaries: Vec<bool> = text
            .char_indices()
            .map(|(offset, _)| text.is_char_boundary(offset))
            .collect();
        assert_eq!(boundaries, vec![true, true, true]);
        assert!(text.is_char_boundary(byte_index(text, 0)));
        assert!(text.is_char_boundary(byte_index(text, 1)));
        assert!(text.is_char_boundary(byte_index(text, 2)));
        assert_eq!(
            byte_index(text, 3),
            text.len(),
            "one past the last is the end"
        );
    }

    #[test]
    fn a_tap_focuses_the_field() {
        let input = field();
        assert!(!input.focused.get());
        let mut tap = tap_at(TEXT_X + 5.0, FIELD.y + FIELD.height / 2.0);
        assert!(input.on_event(&mut tap, FIELD, &mono));
        assert!(input.focused.get(), "and it stays focused afterwards");
    }

    #[test]
    fn a_tap_positions_the_caret_from_the_pointers_x() {
        // 22 pixels into the text is 2.2 characters in, so the nearest boundary is
        // the one after the second character.
        let input = typed("hello");
        let mut tap = tap_at(TEXT_X + 22.0, FIELD.y + 20.0);
        assert!(input.on_event(&mut tap, FIELD, &mono));
        assert_eq!(input.caret(), 2);
    }

    #[test]
    fn a_tap_past_the_end_of_the_text_puts_the_caret_at_the_end() {
        let input = typed("hi");
        let mut tap = tap_at(TEXT_X + 400.0, FIELD.y + 20.0);
        assert!(input.on_event(&mut tap, FIELD, &mono));
        assert_eq!(input.caret(), 2, "there is nothing past the last character");
    }

    #[test]
    fn a_tap_before_the_text_puts_the_caret_at_the_start() {
        let input = typed("hi");
        let mut tap = tap_at(TEXT_X - 30.0, FIELD.y + 20.0);
        assert!(input.on_event(&mut tap, FIELD, &mono));
        assert_eq!(input.caret(), 0, "not a negative index");
    }

    #[test]
    fn a_tap_on_a_boundary_goes_to_that_boundary() {
        let input = typed("hello");
        // Exactly on the line between the second and third characters: a pointer
        // there is asking for the boundary, and the field gives it rather than
        // rounding to the nearest character.
        let mut tap = tap_at(TEXT_X + 25.0, FIELD.y + 20.0);
        assert!(input.on_event(&mut tap, FIELD, &mono));
        assert_eq!(input.caret(), 3);
    }

    #[test]
    fn a_tap_collapses_a_selection() {
        let input = typed("hello");
        input.set_selection(1, 4);
        assert_eq!(input.selection(), Some(1..4));
        let mut tap = tap_at(TEXT_X + 25.0, FIELD.y + 20.0);
        assert!(input.on_event(&mut tap, FIELD, &mono));
        assert_eq!(input.selection(), None, "a tap is not a selection");
        assert_eq!(input.caret(), 3);
        assert_eq!(input.text.get(), "hello", "and it typed nothing");
    }

    #[test]
    fn a_tap_is_consumed_even_when_it_only_focuses() {
        let input = field();
        let mut tap = tap_at(TEXT_X, FIELD.y + 20.0);
        assert!(input.on_event(&mut tap, FIELD, &mono));
        assert!(tap.consumed(), "or it would reach whatever is behind");
    }

    #[test]
    fn a_drag_extends_the_selection() {
        // The gesture that operates the selection rectangle the paint records.
        // A test that only drew the rectangle would not know whether anything
        // could move it.
        let input = typed("hello");
        let mut tap = tap_at(TEXT_X + 2.0, FIELD.y + 20.0);
        input.on_event(&mut tap, FIELD, &mono);
        assert_eq!(input.caret(), 0, "2 pixels in is before the first midpoint");
        let mut drag = drag_to(TEXT_X + 32.0, FIELD.y + 20.0);
        assert!(input.on_event(&mut drag, FIELD, &mono));
        assert_eq!(
            input.selection(),
            Some(0..3),
            "from the caret to the pointer"
        );
        let mut back = drag_to(TEXT_X + 15.0, FIELD.y + 20.0);
        assert!(input.on_event(&mut back, FIELD, &mono));
        assert_eq!(input.selection(), Some(0..2), "and back again");
    }

    #[test]
    fn a_tap_over_a_field_reaches_the_field_and_not_the_panel_behind_it() {
        let mut nodes = Arena::new();
        let input = TextInput::new(&mut nodes);
        let panel = on_a_panel(&input, &mut nodes, Size::new(300.0, 100.0));
        let over_field = Offset::new(10.0, 10.0);
        let over_panel = Offset::new(280.0, 90.0);

        assert_eq!(hit_test(&nodes, panel, over_field), Some(input.handle()));
        assert_eq!(hit_test(&nodes, panel, over_panel), Some(panel));

        let tap = tap_at(over_field.x, over_field.y);
        let chain = route(&nodes, panel, &tap);
        assert_eq!(
            chain,
            vec![input.handle(), panel],
            "the field is offered first"
        );
    }

    #[test]
    fn the_tap_and_the_paint_measure_the_text_from_the_same_box() {
        // One number feeds both: `inner_rect` is the box the run is drawn into and
        // the box a pointer's x is measured against. Changing the padding must
        // move both by the same amount, or a caret drifts from the text it
        // belongs to.
        let input = typed("hello");
        let before = runs(&input.paint(FIELD, &mono))[0].0;
        let mut tap = tap_at(TEXT_X + 32.0, FIELD.y + 20.0);
        input.on_event(&mut tap, FIELD, &mono);
        assert_eq!(input.caret(), 3, "32 pixels in is the third boundary");

        input.padding.set(20.0);
        let after = runs(&input.paint(FIELD, &mono))[0].0;
        assert_eq!(
            after,
            before + 12.0,
            "the run moved 12 pixels, which is the 12 more padding"
        );
        let mut tap = tap_at(after + 32.0, FIELD.y + 20.0);
        input.on_event(&mut tap, FIELD, &mono);
        assert_eq!(
            input.caret(),
            3,
            "and the same pointer is still the same boundary, because the box \
             both readings use moved with it"
        );
    }

    #[test]
    fn a_change_callback_fires_with_the_text_after_the_edit() {
        let mut input = typed("ab");
        let seen = changes(&mut input);
        input.insert_text("c");
        assert_eq!(*seen.borrow(), vec!["abc".to_string()]);
        input.delete_backward();
        assert_eq!(
            *seen.borrow(),
            vec!["abc".to_string(), "ab".to_string()],
            "and again for the second edit, in order"
        );
    }

    #[test]
    fn a_backspace_that_changes_nothing_does_not_fire_the_change_callback() {
        let mut input = typed("ab");
        input.set_selection(0, 0);
        input.snap_to_state();
        let seen = changes(&mut input);
        let mut event = key_down(Keycode::Backspace);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert!(
            seen.borrow().is_empty(),
            "a backspace at the start is not a change, and a caller driving \
             something from this is not told about it"
        );
    }

    #[test]
    fn a_delete_that_changes_nothing_does_not_fire_the_change_callback() {
        let mut input = typed("ab");
        let seen = changes(&mut input);
        let mut event = key_down(Keycode::Delete);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert!(seen.borrow().is_empty(), "there is nothing after the caret");
    }

    #[test]
    fn an_edit_that_writes_the_same_text_reports_nothing() {
        // The replacement of a selection by the characters it already held: the
        // text is the same, so nothing moved and nothing is reported. This is the
        // case `Property::on_change` cannot express, because it fires on every
        // write including a write of the same value.
        let mut input = typed("hello");
        input.set_selection(1, 3);
        let seen = changes(&mut input);
        input.insert_text("el");
        assert_eq!(input.text.get(), "hello", "the same text");
        assert!(seen.borrow().is_empty(), "and the same report: none");
    }

    #[test]
    fn a_caret_move_does_not_fire_the_change_callback() {
        let mut input = typed("abc");
        let seen = changes(&mut input);
        for keycode in [Keycode::Right, Keycode::Left, Keycode::End, Keycode::Home] {
            let mut event = key_down(keycode);
            assert!(input.on_event(&mut event, FIELD, &mono));
        }
        input.move_caret(1);
        assert!(
            seen.borrow().is_empty(),
            "an arrow moves where the insertion point is, not what is there"
        );
    }

    #[test]
    fn a_caret_written_by_the_caller_does_not_fire_the_change_callback() {
        let mut input = typed("abc");
        let seen = changes(&mut input);
        input.text.set("changed by the caller".to_string());
        assert!(
            seen.borrow().is_empty(),
            "the caller is itself, and reads it back"
        );
        assert_eq!(input.text.get(), "changed by the caller");
    }

    #[test]
    fn enter_submits_the_text() {
        let mut input = typed("42");
        let seen = submits(&mut input);
        let mut event = key_down(Keycode::Return);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert!(event.consumed(), "so it does not reach the field behind");
        assert_eq!(*seen.borrow(), vec!["42".to_string()]);
    }

    #[test]
    fn the_keypads_enter_submits_too() {
        let mut input = typed("42");
        let seen = submits(&mut input);
        let mut event = key_down(Keycode::KpEnter);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(*seen.borrow(), vec!["42".to_string()]);
    }

    #[test]
    fn enter_submits_the_text_as_it_is_at_that_moment() {
        let mut input = typed("a");
        let seen = submits(&mut input);
        input.insert_text("bc");
        let mut event = key_down(Keycode::Return);
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert_eq!(
            *seen.borrow(),
            vec!["abc".to_string()],
            "and not the text as it was when the callback was registered"
        );
    }

    #[test]
    fn enter_in_an_unfocused_field_submits_nothing() {
        let mut input = field();
        input.insert_text("42");
        let seen = submits(&mut input);
        let mut event = key_down(Keycode::Return);
        assert!(!input.on_event(&mut event, FIELD, &mono));
        assert!(!event.consumed(), "and the Return travels on");
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn typing_does_not_fire_the_submit_callback() {
        let mut input = typed("");
        let seen = submits(&mut input);
        let mut event = typed_text("a");
        assert!(input.on_event(&mut event, FIELD, &mono));
        assert!(
            seen.borrow().is_empty(),
            "a run of text is not a submission"
        );
    }

    #[test]
    fn a_key_the_field_does_not_use_is_left_alone() {
        let input = typed("ab");
        let mut event = key_down(Keycode::Escape);
        assert!(!input.on_event(&mut event, FIELD, &mono));
        assert!(
            !event.consumed(),
            "so a caller's own Escape handler sees it"
        );
        let mut up = key_down(Keycode::Up);
        assert!(
            !input.on_event(&mut up, FIELD, &mono),
            "and an up arrow too"
        );
        assert!(!up.consumed());
    }

    #[test]
    fn a_focused_field_starts_with_a_visible_caret() {
        let input = field();
        assert!(!input.caret_visible(), "an unfocused field shows no caret");
        input.focus();
        assert!(
            input.caret_visible(),
            "and focusing starts the visible phase"
        );
    }

    #[test]
    fn the_blink_hides_the_caret_after_500ms() {
        let input = field();
        input.focus();
        assert!(!input.tick(ms(499)), "nothing changed half a second in");
        assert!(input.caret_visible(), "and the caret is still there");
        assert!(input.tick(ms(1)), "the 500th millisecond is the flip");
        assert!(!input.caret_visible());
    }

    #[test]
    fn the_blink_brings_the_caret_back_after_another_500ms() {
        let input = field();
        input.focus();
        tick(&input, 500);
        assert!(!input.caret_visible());
        assert!(!input.tick(ms(499)), "499ms into the hidden phase");
        assert!(!input.caret_visible(), "and the caret is still hidden");
        assert!(input.tick(ms(1)));
        assert!(input.caret_visible(), "and 500ms of it, the caret is back");
    }

    #[test]
    fn the_blink_reports_a_change_only_when_the_phase_flips() {
        let input = field();
        input.focus();
        assert!(!input.tick(ms(100)), "100ms in, the phase has not flipped");
        assert!(!input.tick(ms(100)));
        assert!(!input.tick(ms(100)));
        assert!(!input.tick(ms(100)));
        assert!(input.caret_visible(), "still visible after 400ms");
        assert!(input.tick(ms(100)), "and the 500th millisecond is the flip");
    }

    #[test]
    fn an_unfocused_field_reports_no_blink_change() {
        let input = field();
        assert!(!input.tick(ms(500)), "there is no caret to show or hide");
        assert!(!input.tick(ms(500)));
        assert!(!input.caret_visible());
    }

    #[test]
    fn focusing_restarts_the_blink_in_its_visible_phase() {
        let input = field();
        input.focus();
        tick(&input, 500);
        assert!(
            !input.caret_visible(),
            "half way through, the caret is hidden"
        );
        input.blur();
        input.focus();
        assert!(
            input.caret_visible(),
            "and a field that is focused again shows a caret at once, rather \
             than nothing for the rest of that phase"
        );
    }

    #[test]
    fn typing_restarts_the_blink_in_its_visible_phase() {
        let input = typed("ab");
        tick(&input, 500);
        assert!(!input.caret_visible());
        input.insert_text("c");
        assert!(
            input.caret_visible(),
            "a keystroke whose caret landed in the hidden phase looks like a \
             keystroke that was dropped"
        );
    }

    #[test]
    fn the_blink_is_500_visible_and_500_hidden() {
        assert_eq!(BLINK_VISIBLE, Duration::from_millis(500));
        assert_eq!(BLINK_HIDDEN, Duration::from_millis(500));
        assert_eq!(blink_period(), Duration::from_millis(1000));
    }

    #[test]
    fn a_hidden_caret_is_not_drawn_and_a_visible_one_is() {
        let input = typed("ab");
        assert!(input.caret_visible());
        assert_eq!(
            rectangles(&input.paint(FIELD, &mono)).len(),
            1,
            "the visible caret is a rectangle"
        );
        tick(&input, 500);
        assert!(
            rectangles(&input.paint(FIELD, &mono)).is_empty(),
            "and the hidden one is nothing at all"
        );
    }

    #[test]
    fn an_unfocused_field_draws_no_caret_at_all() {
        let input = typed("ab");
        input.blur();
        assert!(rectangles(&input.paint(FIELD, &mono)).is_empty());
        assert!(
            shapes(&input.paint(FIELD, &mono)) == vec!["rounded", "rounded", "text"],
            "which is the border, the surface and the run and nothing else"
        );
    }

    #[test]
    fn two_fields_keep_separate_blinks_and_separate_carets() {
        let first = typed("aa");
        let second = typed("bbbb");
        tick(&first, 500);
        assert!(!first.caret_visible(), "the first is in its hidden phase");
        assert!(
            second.caret_visible(),
            "and the second, which was never ticked, is not: the blink is per \
             field and not a clock the two share"
        );
        assert_eq!(first.caret(), 2);
        assert_eq!(second.caret(), 4);
    }

    #[test]
    fn the_placeholder_is_drawn_while_the_field_is_empty() {
        let input = typed("");
        input.placeholder.set("Search".to_string());
        let painted = runs(&input.paint(FIELD, &mono));
        assert_eq!(painted.len(), 1, "one run, as there always is");
        assert_eq!(painted[0].2, "Search");
        assert_eq!(painted[0].0, TEXT_X, "at the text's own left edge");
        assert_eq!(painted[0].1, TEXT_Y);
    }

    #[test]
    fn the_placeholder_is_not_drawn_over_real_text() {
        let input = typed("q");
        input.placeholder.set("Search".to_string());
        let painted = runs(&input.paint(FIELD, &mono));
        assert_eq!(painted.len(), 1, "still one run");
        assert_eq!(painted[0].2, "q", "and it is the text, not the placeholder");
    }

    #[test]
    fn the_placeholder_is_drawn_in_the_muted_colour_and_the_text_in_its_own() {
        let dark = Theme::dark();
        let input = typed("");
        input.placeholder.set("Search".to_string());
        let placeholder = runs(&input.paint(FIELD, &mono))[0].3;
        assert_eq!(placeholder, token_color(&dark, ThemeToken::TextMuted));
        assert_ne!(
            placeholder,
            input.foreground.get(),
            "and not the text's colour"
        );

        input.insert_text("q");
        let text = runs(&input.paint(FIELD, &mono))[0].3;
        assert_eq!(text, token_color(&dark, ThemeToken::Text));
    }

    #[test]
    fn an_empty_field_with_no_placeholder_still_draws_one_run() {
        // `Label::paint` records a line even when the line is empty, and the
        // renderer's text path is what a caller has to be able to rely on. So the
        // field records one run whatever it holds.
        let input = typed("");
        let painted = runs(&input.paint(FIELD, &mono));
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].2, "");
    }

    #[test]
    fn the_run_carries_the_fields_font_size_and_tracking() {
        let input = typed("ab");
        input.font_size.set(20.0);
        input.letter_spacing.set(3.0);
        let painted = runs(&input.paint(FIELD, &mono));
        assert_eq!(painted[0].4, 20.0, "the size it was given");
        assert_eq!(
            painted[0].5, 3.0,
            "and the tracking, which the caret counts too"
        );
    }

    #[test]
    fn the_run_is_centred_in_the_fields_inner_box() {
        // The inner box is 24 high and the line box 20, so the run's top edge is
        // 2 pixels down: 74 + (24 - 20) / 2.
        let input = typed("ab");
        assert_eq!(runs(&input.paint(FIELD, &mono))[0].1, 76.0);
        input.line_height.set(24.0);
        assert_eq!(
            runs(&input.paint(FIELD, &mono))[0].1,
            74.0,
            "and a line box as tall as the box is drawn flush with its top"
        );
    }

    #[test]
    fn a_field_paints_a_border_a_surface_and_one_run() {
        let input = typed("hello");
        let painted = input.paint(FIELD, &mono);
        assert_eq!(
            shapes(&painted),
            vec!["rounded", "rounded", "text", "rect"],
            "the border, the surface, the run and the caret, in that order"
        );
    }

    #[test]
    fn the_surface_is_drawn_over_the_border_so_the_border_reads_as_an_outline() {
        // The defect `.ai/NEVERAGAIN.md` records as *a filled rounded rectangle is
        // not an outline*: a `RoundedRect` fills its rect, so a border recorded on
        // its own is a second background and the field is a card with a line round
        // it. Asserting the pair — the grown shape *and* the covering shape
        // recorded after it — is the only way a recorded-command assertion can see
        // that, because the border alone is satisfied by a filled rectangle of
        // exactly the right colour and place.
        let input = typed("hello");
        let painted = input.paint(FIELD, &mono);
        let border = index_of_shape(&painted, FIELD).expect("the border is the field's own rect");
        let surface = Rect::new(122.0, 66.0, 196.0, 40.0);
        let over_surface = index_of_shape(&painted, surface)
            .expect("the surface is the field inset by the border");
        assert!(
            border < over_surface,
            "the surface is recorded after the border, so only the border is left"
        );
        assert_eq!(
            surface,
            grow(FIELD, -DEFAULT_BORDER_WIDTH),
            "and it is inset by the same number the property holds"
        );
    }

    #[test]
    fn the_border_is_the_focus_colour_when_focused_and_the_hairline_when_not() {
        let dark = Theme::dark();
        let input = typed("hello");
        let border_of = |input: &TextInput| rounded(&input.paint(FIELD, &mono))[0].2;

        assert_eq!(border_of(&input), token_color(&dark, ThemeToken::Border));
        input.focus();
        input.snap_to_state();
        assert_eq!(
            border_of(&input),
            token_color(&dark, ThemeToken::Primary),
            "and the primary, which is what \"highlighted\" means here"
        );
        assert_eq!(
            input.style().border,
            border_of(&input),
            "and the style agrees"
        );
    }

    #[test]
    fn the_caret_is_a_thin_rectangle_at_the_insertion_point() {
        let input = typed("hello");
        input.set_selection(2, 2);
        let painted = rectangles(&input.paint(FIELD, &mono));
        assert_eq!(painted.len(), 1, "one rectangle, and it is the caret");
        assert_eq!(
            painted[0].0,
            Rect::new(TEXT_X + 20.0, 74.0, 2.0, 24.0),
            "2 wide at the third boundary, the height of the inner box, at its top"
        );
        assert_eq!(
            painted[0].1,
            input.caret_color.get(),
            "in the caret's colour"
        );
        assert_eq!(
            painted[0].0.width, DEFAULT_CARET_WIDTH,
            "which is the property"
        );
    }

    #[test]
    fn the_caret_is_drawn_over_the_text() {
        let input = typed("hello");
        input.set_selection(2, 2);
        let painted = input.paint(FIELD, &mono);
        let caret = painted
            .iter()
            .position(|command| matches!(command, DrawCommand::Rect { .. }))
            .expect("the caret is recorded");
        let run = painted
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { .. }))
            .expect("the run is recorded");
        assert!(
            run < caret,
            "a caret is over everything, including its own text"
        );
    }

    #[test]
    fn a_selection_is_a_rounded_rectangle_with_the_text_over_it() {
        let input = typed("hello");
        input.set_selection(1, 4);
        let painted = input.paint(FIELD, &mono);
        let shapes = rounded(&painted);
        assert_eq!(shapes.len(), 3, "the border, the surface and the selection");
        let selection = shapes[2];
        assert_eq!(
            selection.0,
            Rect::new(TEXT_X + 10.0, 74.0, 30.0, 24.0),
            "from the first selected character's edge to the last one's"
        );
        assert_eq!(
            selection.1, 12.0,
            "a pill, which is half the 24-pixel inner box"
        );
        assert_eq!(selection.2, input.selection.get(), "in the fill's colour");

        let selection_at = index_of_shape(&painted, selection.0).expect("it is recorded");
        let run_at = painted
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { .. }))
            .expect("the run is recorded");
        assert!(
            run_at > selection_at,
            "and the run is recorded after it, so the selection is a highlight \
             behind the text rather than a box over it"
        );
    }

    #[test]
    fn an_empty_selection_draws_no_rectangle() {
        let input = typed("hello");
        input.set_selection(2, 2);
        assert_eq!(
            input.selection(),
            None,
            "an anchor on the caret is not a selection"
        );
        assert_eq!(
            rounded(&input.paint(FIELD, &mono)).len(),
            2,
            "just the field"
        );
    }

    #[test]
    fn a_selection_at_the_start_of_the_text_does_not_drag_the_field_left() {
        // A selection whose left edge is scrolled off must not push the
        // rectangle to a negative width or off the inner box: the drawn selection
        // starts at the inner box's own left edge and is the width that is left.
        let input = typed(LONG);
        input.set_selection(0, 26);
        let _ = input.paint(FIELD, &mono);
        let shapes = rounded(&input.paint(FIELD, &mono));
        let selection = shapes[2].0;
        assert!(
            selection.x >= TEXT_X,
            "the selection is inside the inner box: {selection:?}"
        );
        assert!(selection.width > 0.0, "and it is not empty");
    }

    #[test]
    fn the_selection_fill_is_the_surface_moved_toward_the_focus_token() {
        let dark = Theme::dark();
        let input = typed("hello");
        input.set_selection(0, 2);
        let expected = Color::interpolate(
            &token_color(&dark, ThemeToken::Surface),
            &token_color(&dark, ThemeToken::Primary),
            SELECTION_BLEND,
        );
        assert_eq!(rounded(&input.paint(FIELD, &mono))[2].2, expected);
        assert_eq!(SELECTION_BLEND, 0.35, "which is the number the doc fixes");
    }

    #[test]
    fn the_selection_fill_is_a_highlight_in_both_themes() {
        for theme in [Theme::dark(), Theme::light()] {
            let mut input = typed("hello");
            input.set_palette(Palette::from_theme(&theme));
            input.snap_to_state();
            input.set_selection(0, 2);
            let fill = rounded(&input.paint(FIELD, &mono))[2].2;
            let surface = input.background.get();
            let difference = (i32::from(fill.r) - i32::from(surface.r))
                .abs()
                .max((i32::from(fill.g) - i32::from(surface.g)).abs())
                .max((i32::from(fill.b) - i32::from(surface.b)).abs());
            assert!(
                difference >= 48,
                "a fill {difference} away from the surface is a highlight and \
                 not a tint: {fill:?} on {surface:?}"
            );
        }
    }

    #[test]
    fn the_selection_fill_leaves_the_text_legible_in_both_themes() {
        for theme in [Theme::dark(), Theme::light()] {
            let mut input = typed("hello");
            input.set_palette(Palette::from_theme(&theme));
            input.snap_to_state();
            input.set_selection(0, 2);
            let fill = rounded(&input.paint(FIELD, &mono))[2].2;
            let text = input.foreground.get();
            let difference = (i32::from(fill.r) - i32::from(text.r))
                .abs()
                .max((i32::from(fill.g) - i32::from(text.g)).abs())
                .max((i32::from(fill.b) - i32::from(text.b)).abs());
            assert!(
                difference >= 32,
                "the text has to be another colour from what it is drawn on: \
                 {text:?} on {fill:?}"
            );
        }
    }

    #[test]
    fn a_field_narrower_than_its_text_scrolls_to_keep_the_caret_visible() {
        // The closed form: 26 characters of 10 pixels is 260, the inner box is
        // 180 and the caret is 2 wide, so there are 178 pixels of room and 260
        // of content, and the furthest the field can scroll is 260 - 178 = 82.
        let input = typed(LONG);
        assert_eq!(input.content_width(&mono), 260.0);
        let _ = input.paint(FIELD, &mono);
        assert_close(input.scroll_x.get(), 82.0, "the scroll at the end");
        assert_close(
            input.caret_x(FIELD, &mono),
            TEXT_X + 178.0,
            "so the caret's left edge is the 178th pixel",
        );
        assert_eq!(
            input.caret_x(FIELD, &mono) + input.caret_width.get(),
            TEXT_RIGHT,
            "and its right edge is the inner box's own right edge"
        );
    }

    #[test]
    fn the_scroll_is_zero_while_the_text_fits() {
        let input = typed("hello");
        input.set_selection(0, 0);
        let _ = input.paint(FIELD, &mono);
        assert_eq!(
            input.scroll_x.get(),
            0.0,
            "50 pixels in 180: nothing to scroll"
        );
    }

    #[test]
    fn moving_the_caret_back_to_the_start_scrolls_the_text_back() {
        let input = typed(LONG);
        let _ = input.paint(FIELD, &mono);
        assert!(
            input.scroll_x.get() > 0.0,
            "the field has scrolled to the end"
        );
        input.move_caret(0);
        input.snap_to_state();
        let _ = input.paint(FIELD, &mono);
        assert_eq!(
            input.scroll_x.get(),
            0.0,
            "and a caret at the start scrolls it back"
        );
        assert_eq!(input.caret_x(FIELD, &mono), TEXT_X);
    }

    #[test]
    fn the_scroll_never_exceeds_the_content() {
        // An empty field in a wide box has nothing to scroll, and a scroll that
        // went past the content would draw the text off the left of the field
        // with nothing to fill the gap.
        let input = typed("");
        let _ = input.paint(FIELD, &mono);
        assert_eq!(input.scroll_x.get(), 0.0);

        let input = typed(LONG);
        input.padding.set(0.0);
        input.border_width.set(0.0);
        let _ = input.paint(FIELD, &mono);
        assert_eq!(
            input.scroll_x.get(),
            62.0,
            "with the whole 200 pixels of the field for text and the caret's 2 \
             pixels out of them, 260 - 198 = 62"
        );
    }

    #[test]
    fn a_caret_moved_without_a_rect_is_brought_into_view_at_the_next_paint() {
        // Every public mutator has no rect to measure against, so none of them can
        // scroll: the scroll is the paint's to reconcile, and this is the test
        // that says so.
        let input = typed(LONG);
        let _ = input.paint(FIELD, &mono);
        assert!(
            input.scroll_x.get() > 0.0,
            "the field has scrolled to the end"
        );
        let scrolled = input.scroll_x.get();
        input.set_selection(2, 2);
        assert_eq!(
            input.scroll_x.get(),
            scrolled,
            "the selection cannot know how wide the field is, so the text is \
             still scrolled to the end with the caret nowhere near it"
        );
        let _ = input.paint(FIELD, &mono);
        // The caret is 20 pixels into a 260-pixel string and the scroll had run
        // past it, so the least move that brings it back into view is a scroll of
        // exactly its own position.
        assert_eq!(input.scroll_x.get(), 20.0, "which is where the caret is");
        assert_eq!(
            input.caret_x(FIELD, &mono),
            TEXT_X,
            "so the caret is at the left edge of the text box, and in view"
        );
    }

    #[test]
    fn only_the_characters_that_fit_inside_the_field_are_drawn() {
        // The clip. A `DrawCommand::Text` carries no width, so what is not
        // recorded is what is not drawn, and the rule is that a character is
        // recorded only if it fits between the scroll and the inner box's right
        // edge. 82 pixels of scroll against a 10-pixel advance drops characters 0
        // through 7 — the one at offset 80 straddles the left edge — and leaves
        // "j" through "z", seventeen of them, ending 252 pixels in.
        let input = typed(LONG);
        let _ = input.paint(FIELD, &mono);
        let painted = runs(&input.paint(FIELD, &mono));
        assert_eq!(painted.len(), 1, "one run, as there always is");
        assert_eq!(
            painted[0].2, "jklmnopqrstuvwxyz",
            "and it is only what fits: 90..=260 inside 82..=262"
        );
        assert_eq!(painted[0].2.chars().count(), 17);
    }

    #[test]
    fn the_run_that_is_drawn_starts_inside_the_field() {
        let input = typed(LONG);
        let _ = input.paint(FIELD, &mono);
        let painted = &runs(&input.paint(FIELD, &mono))[0];
        assert_close(
            painted.0,
            138.0,
            "the first drawn character is at content 90, so 130 + 90 - 82",
        );
        assert!(painted.0 >= TEXT_X, "and never before the inner box");
        // Seventeen characters of 10 pixels.
        let width = 170.0;
        assert!(
            painted.0 + width <= TEXT_RIGHT,
            "and the run's end is inside it: {}",
            painted.0 + width,
        );
    }

    #[test]
    fn a_field_painted_at_the_origin_clips_the_same_way() {
        // The clip is measured from the field's own inner box, so a field in the
        // corner of the window clips to that box and not to the window's.
        let input = typed(LONG);
        let _ = input.paint(FIELD_AT_ORIGIN, &mono);
        let painted = runs(&input.paint(FIELD_AT_ORIGIN, &mono));
        assert_eq!(painted[0].2, "jklmnopqrstuvwxyz", "the same seventeen");
        assert_close(
            painted[0].0,
            18.0,
            "at 10 + 90 - 82, which is the same 8 pixels further right than the \
             one at 120, because the inner box is 8 further right too",
        );
    }

    #[test]
    fn the_placeholder_is_clipped_to_the_field_as_well() {
        let input = typed("");
        input.placeholder.set(LONG.to_string());
        let _ = input.paint(FIELD, &mono);
        let painted = runs(&input.paint(FIELD, &mono));
        assert_eq!(
            painted[0].2, "abcdefghijklmnopqr",
            "18 characters is what fits in 180 pixels, and the 19th would end \
             one pixel past the inner box, so it is not recorded — a \
             placeholder is cut, not drawn over whatever is beside it"
        );
        assert_eq!(
            painted[0].0, TEXT_X,
            "and it starts at the text's own edge: an empty field has its caret \
             at the left, so the field has not scrolled to the placeholder's \
             far end and there is nothing to scroll to"
        );
    }

    #[test]
    fn a_text_field_longer_than_its_content_does_not_scroll() {
        // A scroll that went past the content would leave a gap between the text
        // and the left edge with nothing in it, so the field clamps to
        // `content - width` and no further.
        let input = typed("hi");
        for _ in 0..3 {
            let _ = input.paint(FIELD, &mono);
        }
        assert_eq!(input.scroll_x.get(), 0.0);
        assert_eq!(runs(&input.paint(FIELD, &mono))[0].2, "hi");
    }

    #[test]
    fn byte_index_finds_a_boundary_or_the_end() {
        assert_eq!(byte_index("abc", 0), 0);
        assert_eq!(byte_index("abc", 1), 1);
        assert_eq!(byte_index("abc", 3), 3, "the end of the string");
        assert_eq!(byte_index("abc", 4), 3, "and past it, still the end");
        assert_eq!(byte_index("", 0), 0);
        // The two-byte and three-byte cases that make a byte offset an index and
        // a character index not one.
        assert_eq!(byte_index("ä", 1), 2, "one character, two bytes");
        assert_eq!(byte_index("日", 1), 3, "one character, three bytes");
        assert_eq!(byte_index("aä", 1), 1);
        assert_eq!(byte_index("aä", 2), 3);
    }

    #[test]
    fn position_of_counts_characters_into_a_fractional_index() {
        assert_eq!(position_of(0, 5), 0.0);
        assert_eq!(position_of(1, 5), 1.0);
        assert_eq!(position_of(5, 5), 5.0);
        assert_eq!(position_of(9, 5), 5.0, "clamped to the count");
        assert_eq!(position_of(0, 0), 0.0);
    }

    #[test]
    fn offset_at_position_stops_inside_the_character_it_lands_in() {
        // The crossing the animation depends on: a position of 1.5 is halfway
        // through the second character, and the answer is 15 rather than either
        // 10 or 20. A walk that snapped to the next boundary would make the caret
        // jump 10 pixels at the halfway point of every character.
        assert_eq!(offset_at_position("abcd", 0.0, 0.0, &mono), 0.0);
        assert_eq!(offset_at_position("abcd", 1.0, 0.0, &mono), 10.0);
        assert_eq!(offset_at_position("abcd", 1.5, 0.0, &mono), 15.0);
        assert_eq!(offset_at_position("abcd", 2.25, 0.0, &mono), 22.5);
        assert_eq!(offset_at_position("abcd", 4.0, 0.0, &mono), 40.0);
    }

    #[test]
    fn offset_at_position_past_the_end_is_the_end_and_before_the_start_is_the_start() {
        assert_eq!(offset_at_position("abc", 99.0, 0.0, &mono), 30.0);
        assert_eq!(offset_at_position("abc", -5.0, 0.0, &mono), 0.0);
        assert_eq!(offset_at_position("", 3.0, 0.0, &mono), 0.0);
    }

    #[test]
    fn offset_at_position_counts_the_letter_spacing() {
        assert_eq!(offset_at_position("abc", 1.0, 2.0, &mono), 12.0);
        assert_eq!(offset_at_position("abc", 3.0, 2.0, &mono), 36.0);
    }

    #[test]
    fn index_beside_picks_the_nearest_boundary() {
        // Midpoints are at 5, 15, 25: inside the first half of a character is that
        // character's own boundary, outside it is the next one.
        assert_eq!(index_beside("abcd", -10.0, 0.0, &mono), 0);
        assert_eq!(index_beside("abcd", 0.0, 0.0, &mono), 0);
        assert_eq!(index_beside("abcd", 4.9, 0.0, &mono), 0, "inside the first");
        assert_eq!(index_beside("abcd", 5.0, 0.0, &mono), 1, "and half is up");
        assert_eq!(index_beside("abcd", 14.9, 0.0, &mono), 1);
        assert_eq!(index_beside("abcd", 15.0, 0.0, &mono), 2);
        assert_eq!(index_beside("abcd", 15.1, 0.0, &mono), 2);
        assert_eq!(
            index_beside("abcd", 999.0, 0.0, &mono),
            4,
            "and past it is the end"
        );
    }

    #[test]
    fn index_beside_walks_characters_and_not_bytes() {
        // With two-byte characters the boundaries are at 0, 10, 30 in *content*
        // pixels, because a character's advance is what decides a midpoint and not
        // how many bytes it takes.
        assert_eq!(index_beside("ää", 0.0, 0.0, &mono), 0);
        assert_eq!(index_beside("ää", 4.9, 0.0, &mono), 0);
        assert_eq!(index_beside("ää", 5.1, 0.0, &mono), 1);
        assert_eq!(index_beside("ää", 15.1, 0.0, &mono), 2);
        assert_eq!(index_beside("ää", 999.0, 0.0, &mono), 2);
    }

    #[test]
    fn visible_run_takes_only_what_fits_between_the_two_edges() {
        // The whole alphabet at 10 pixels against a window of 82..=262: the
        // character at 80 straddles the left edge and the one at 260 would end at
        // 270, past the right edge, so neither is in the run.
        let (run, first) = visible_run("abcdefghijklmnopqrstuvwxyz", 82.0, 262.0, 0.0, &mono);
        assert_eq!(run, "jklmnopqrstuvwxyz");
        assert_eq!(first, 90.0, "the offset of the first of them");
    }

    #[test]
    fn a_visible_run_that_is_empty_reports_the_windows_left_edge() {
        // A run that is empty has no first character, so the x it is drawn at has
        // to be the left edge of the window — which is where the text would have
        // been had anything fit.
        let (run, first) = visible_run("abc", 100.0, 110.0, 0.0, &mono);
        assert_eq!(run, "");
        assert_eq!(first, 100.0);
    }

    #[test]
    fn a_window_wider_than_the_text_takes_all_of_it() {
        let (run, first) = visible_run("abc", 0.0, 1000.0, 0.0, &mono);
        assert_eq!(run, "abc");
        assert_eq!(first, 0.0);
    }

    #[test]
    fn the_selection_radius_is_half_the_inner_box_and_no_more() {
        // A pill rather than a box round a word, and never wider than it is long.
        assert_eq!(
            selection_radius(30.0, 24.0),
            12.0,
            "half of the 24-pixel box"
        );
        assert_eq!(
            selection_radius(4.0, 24.0),
            2.0,
            "half of the 4-pixel width"
        );
        assert_eq!(selection_radius(0.0, 24.0), 0.0, "and nothing to round");
    }

    #[test]
    fn grow_insets_a_rect_for_a_negative_amount() {
        let grown = grow(FIELD, 2.0);
        assert_eq!(grown, Rect::new(118.0, 62.0, 204.0, 48.0));
        let inset = grow(FIELD, -2.0);
        assert_eq!(inset, Rect::new(122.0, 66.0, 196.0, 40.0));
    }
}
