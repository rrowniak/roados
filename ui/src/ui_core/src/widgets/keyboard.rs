//! The Keyboard widget: an on-screen keyboard for a head unit that has no
//! hardware one.
//!
//! A keyboard is a **grid of keys and a report of the one that was pressed**.
//! The grid is laid out here, the arithmetic is here, and the hit test reads the
//! same arithmetic the paint pass draws from — one number, one owner, because a
//! key that is drawn in one place and found in another is a key nobody can hit.
//! What is *not* here is the text: the widget reports a [`KeyAction`] through
//! [`on_key`](Keyboard::on_key) and has no idea what a text field is, so that
//! the same keyboard drives a search box, an address entry and a PIN pad without
//! either of them knowing about the other.
//!
//! # The layout, and why it is not a phone's
//!
//! Five rows, top to bottom: the **numbers row**, three rows of letters, and a
//! **bottom row** of the three things a user does most. The order is chosen for
//! where a thumb is, not for how it reads. A driver's hand rests at the bottom of
//! the bezel, so the bottom band is the only band a fingertip reaches without the
//! arm moving, and that band carries the space bar, the enter key and the
//! backspace. The numbers row is at the *top*, where reach is worst, because it
//! is the one row every page shares: a digit is one reach away on every layer
//! rather than on one. The three letter rows are in between — frequent, but never
//! so frequent that they are worth a reach the thumb cannot make.
//!
//! Within a row, **each key divides the row**: a row's keys share the space
//! between the padding and the gaps in proportion to their weights. So the
//! nine-key home row's keys are wider than the ten-key rows' — a short row gives
//! every one of its keys more room, which is the cheapest large-target win
//! available and needs no constant changed. The wide keys are weights and not
//! extra columns: `Space` is five units where a letter is one.
//!
//! # The touch floor
//!
//! Every key is at least `MIN_TOUCH_TARGET` in **both** dimensions, floored in
//! [`Keyboard::key_rect`] — which is the one place a key's geometry is computed,
//! so the floor cannot be honoured by the drawing and missed by the hit test. A
//! rect too narrow for the floor produces keys that **overflow** rather than keys
//! that shrink, because a 20-pixel key is a target no finger can be asked to hit
//! and an overflowing one is at least visible and pressable at its edge. A caller
//! that does not want that case gives the keyboard the box
//! [`Keyboard::size`] asks for, which is wide enough for the floor.
//!
//! # Pages
//!
//! Two pages, [`Page::Letters`] and [`Page::Symbols`], swapped by one key that
//! sits at the bottom left and **says which page it switches to** — `?123` on the
//! letters page and `ABC` on the symbols one, so the control never has to be
//! guessed at. The mechanism is a key like any other rather than a gesture or a
//! long press, because a car keyboard has a *large* target for every action and a
//! gesture is none of them. Both pages keep the numbers row and the bottom row
//! and differ only in the three rows between, so a switch never moves the
//! controls a user has just learned the position of.
//!
//! # Press state
//!
//! A [`Tap`](InputEventKind::Tap) arrives on the **release**, so it cannot light
//! the key that is under the finger — by then the finger is gone. The press is
//! therefore a call from the caller, exactly as a
//! [`Button`](crate::widgets::button::Button)'s `pressed` and a
//! [`Slider`](crate::widgets::slider::Slider)'s `dragging` are:
//! [`grab_key`](Keyboard::grab_key) on the pointer down and
//! [`release_key`](Keyboard::release_key) on the pointer up, in the same place
//! the demo calls [`Scroll::grab_thumb`](crate::widgets::scroll::Scroll::grab_thumb).
//! The gesture recogniser has no press to give, and this is the arrangement three
//! other widgets in this library already use rather than a fourth one.
//!
//! # Colours
//!
//! A [`Palette`] names six colours and the properties hold them so a theme switch
//! can be animated into them, exactly as [`Toggle`](crate::widgets::toggle::Toggle)
//! does. No token is added: the theme has no token for a keyboard, and adding six
//! would change [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme
//! tables, the token count and the transition every token takes part in during a
//! switch, for values a switch does not change. [`Palette::from_theme`] says
//! which of the six existing tokens each field is.
//!
//! # What would reverse the decisions above
//!
//! - A **density setting** in the theme would take `MIN_TOUCH_TARGET`,
//!   `KEY_HEIGHT` and `KEY_GAP` off the constants and onto tokens, and the
//!   per-key floors would read from a token rather than a number.
//! - A **head unit with a bezel and glove spec** would replace `MIN_TOUCH_TARGET`
//!   and `DEFAULT_WIDTH` outright; that is the device's number, not the
//!   library's.
//! - A **language with its own layout** would replace `LETTERS` and `SYMBOLS`
//!   and their weights; the arithmetic that consumes them does not change, which
//!   is why they are tables and not code.
//! - A **hardware keyboard** in the head unit would make the page key and the
//!   shift key redundant, and the widget would then be reached by
//!   [`InputEventKind::Text`] — which is the *text* widget's business and not this
//!   one's.
//!
//! # Examples
//!
//! ```
//! use std::cell::RefCell;
//! use std::rc::Rc;
//! use ui_core::arena::Arena;
//! use ui_core::input::{InputEvent, InputEventKind};
//! use ui_core::layout::Offset;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::Rect;
//! use ui_core::theme::Theme;
//! use ui_core::widgets::keyboard::{KeyAction, Keyboard, Palette};
//! use ui_core::widgets::Callback;
//!
//! let mut nodes = Arena::new();
//! let mut keyboard = Keyboard::new(&mut nodes);
//! keyboard.set_palette(Palette::from_theme(&Theme::dark()));
//! keyboard.snap_to_state();
//!
//! let seen = Rc::new(RefCell::new(Vec::new()));
//! let reported = Rc::clone(&seen);
//! keyboard.on_key = Callback::from_fn(move |action| reported.borrow_mut().push(action));
//!
//! // The box a caller would give it, and where the first key lands in it.
//! let rect = Rect::new(0.0, 0.0, keyboard.size().width, keyboard.size().height);
//! let first = keyboard.key_rect(0, rect).expect("the letters page has a first key");
//! assert_eq!(first, Rect::new(8.0, 8.0, 65.0, 52.0), "the `1` key");
//!
//! // A tap on that key is that key.
//! let mut tap = InputEvent::new(
//!     InputEventKind::Tap,
//!     Some(Offset::new(first.x + 1.0, first.y + 1.0)),
//! );
//! assert!(keyboard.on_event(&mut tap, rect));
//! assert_eq!(seen.borrow().as_slice(), &[KeyAction::Char('1')]);
//! assert!(tap.consumed(), "so it does not reach the panel behind");
//!
//! // The gap between two keys belongs to nobody.
//! let mut between = InputEvent::new(InputEventKind::Tap, Some(Offset::new(75.0, 20.0)));
//! assert!(!keyboard.on_event(&mut between, rect));
//! assert!(!between.consumed());
//! ```

use std::cell::{Cell, RefCell};
use std::time::Duration;

use crate::animation::AnimationClock;
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind};
use crate::layout::{Offset, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::button::Motion;
use crate::widgets::Callback;

/// The smallest a key may be, in either dimension, in pixels.
///
/// 44dp is the platform touch target a finger can hit, and it is the same floor
/// [`Button`](crate::widgets::button::Button) puts under a button. It is a
/// constant rather than a theme token for the reason that button's copy of it
/// documents: the theme has no token for it, and adding one would change
/// [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables, the
/// token count and the transition every token takes part in during a theme switch,
/// for a value a switch does not change.
///
/// It is repeated rather than imported for the reason the slider's and the
/// toggle's copies of it are: a `pub const` in another widget's module is not a
/// shared place to keep one, and the three will diverge the day the theme grows a
/// density setting. What would reverse it is that density setting — see the
/// module's *What would reverse the decisions above*.
const MIN_TOUCH_TARGET: f32 = 44.0;

/// How tall a key is drawn and hit-tested, in pixels, unless a caller changes
/// [`key_height`](Keyboard::key_height).
///
/// `MIN_TOUCH_TARGET` plus a finger's worth of margin: the floor is what a
/// finger can reach, and this is what it can *confirm*, because a target exactly
/// at the floor has no edge to feel its way out of. It is a constant for the
/// reason `MIN_TOUCH_TARGET` is, and the floor is applied to it in
/// `key_in_row` so that one expression governs both dimensions — which matters
/// because a caller *can* ask for a shorter key, and the floor is what stops that
/// ask from producing an unhittable one.
const KEY_HEIGHT: f32 = 52.0;

/// The gap between two keys, in pixels, and between a row and the one above it.
///
/// Six is the [`ThemeToken::SpacingSm`](crate::theme::ThemeToken::SpacingSm) the
/// button's own padding defaults to, so a keyboard and a button in one window are
/// one family. It is also what makes a tap *between* two keys land on neither:
/// the gap is real dead space and [`Keyboard::key_at`] says so, which is what
/// lets a tap that hit nothing reach the panel behind.
const KEY_GAP: f32 = 6.0;

/// The gap between the keyboard's own edge and its outermost keys, in pixels.
///
/// The same [`ThemeToken::SpacingXs`](crate::theme::ThemeToken::SpacingXs) the
/// button's vertical padding defaults to. It is in the same unit as `KEY_GAP`
/// and therefore in the same arithmetic, which is what makes the rows tile
/// exactly to the right-hand edge rather than stopping short of it.
const KEY_PADDING: f32 = 8.0;

/// The corner radius of a key and of the panel, in pixels.
///
/// The theme's own [`BorderRadiusMd`](crate::theme::ThemeToken::BorderRadiusMd),
/// which is what every other control in this library draws its corners at. It is
/// a constant for the reason every other sizing constant here is, and the same
/// theme change that moves the token moves it.
const KEY_RADIUS: f32 = 8.0;

/// The focus ring's width, in pixels, unless a caller changes it.
///
/// The ring is drawn **around the panel** and then covered by it, so only its
/// border shows; [`Keyboard::paint`] says so where it draws it, and the test that
/// covers it asserts the grown shape *and* the panel recorded over it.
const FOCUS_RING: f32 = 2.0;

/// The width a keyboard asks for when its caller gives it none of its own.
///
/// 720 is a centre console's share of a landscape head unit, and it is the width
/// at which the ten-key rows come out at 65 pixels each — comfortably above
/// `MIN_TOUCH_TARGET` with room to spare. A narrower rect is still laid out and
/// still hit-tested correctly; it would only put the floor under pressure. See
/// the module's *What would reverse the decisions above*.
const DEFAULT_WIDTH: f32 = 720.0;

/// How tall a key's label is drawn, in pixels.
///
/// The theme's [`FontSizeXl`](crate::theme::ThemeToken::FontSizeXl) is 24, which
/// is a heading; 22 is a keycap. It is a constant for the reason every other
/// sizing constant here is, and it is a **property**
/// ([`font_size`](Keyboard::font_size)) rather than a bare constant because it is
/// the one number here a caller has a real reason to change.
const KEY_FONT_SIZE: f32 = 22.0;

/// The width of one character of a label, as a fraction of the font size.
///
/// [`Keyboard::paint`] takes no font, so it cannot measure a label, and a label
/// centred by a guess is centred by a guess. 0.6 is about the width of a capital
/// in a UI sans at a given size. A caller with a font replaces the guess outright
/// through [`advance`](Keyboard::advance); this is only what a caller that has
/// none gets.
const LABEL_ADVANCE_RATIO: f32 = 0.6;

/// How many rows the keyboard has, on every page.
///
/// A fixed count is what lets both page tables share the row-index arithmetic
/// below and what makes [`Keyboard::size`] one expression. Two pages of the same
/// height is also the *point* of the page design: the bottom row does not move.
const ROWS: usize = 5;

/// What one key does when it is pressed.
///
/// This is the whole of the widget's output. It is an enum rather than a `String`
/// or an index because a keyboard has a *fixed, known* set of keys: a newtype
/// over the index of the key that was pressed would make the caller look the
/// meaning up again, and a string would make every caller re-parse it. The
/// characters are lower or upper case **as the key was drawn**, so a caller
/// inserting [`Char`](KeyAction::Char) inserts exactly the glyph the user pressed
/// and never has to track [`Shift`](KeyAction::Shift) itself.
///
/// A page change is reported as [`PageUp`](KeyAction::PageUp) and
/// [`PageDown`](KeyAction::PageDown) rather than as a `Page(Page)` variant,
/// because a caller that cares about the page reads [`Keyboard::page`] and one that
/// does not ignores both — whereas a variant carrying a page would invite a caller
/// to hold a second notion of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyAction {
    /// A character, in the case the key was drawn in.
    Char(char),
    /// Delete the character before the cursor.
    Backspace,
    /// Confirm the entry.
    Enter,
    /// Insert a space.
    Space,
    /// The shift key. It toggles the keyboard's own sticky shift, so the *next*
    /// character is reported upper case, and it is reported as well for a caller
    /// that wants to know a shift was tapped — for auto-capitalisation, say.
    Shift,
    /// Move to the symbols page.
    PageUp,
    /// Move back to the letters page.
    PageDown,
}

/// Which of the two pages the keyboard is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    /// The numbers row, three rows of letters, and the bottom row.
    Letters,
    /// The numbers row, two rows of punctuation, a row of brackets, and the
    /// bottom row.
    Symbols,
}

impl Default for Page {
    /// Returns [`Page::Letters`]: a keyboard opens on letters, which is what it
    /// is for and what a caller typing a word wants first.
    fn default() -> Self {
        Page::Letters
    }
}

impl Page {
    /// Returns the page this one switches to.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Page::Letters => Page::Symbols,
            Page::Symbols => Page::Letters,
        }
    }

    /// Returns the label the page-switch key carries **while this page is
    /// showing**: the name of the page it goes to.
    ///
    /// It is the page's *destination*, not its own name, and that is deliberate:
    /// a control that says where it takes you needs no mental model of layers,
    /// which is the whole point of a two-page keyboard.
    #[must_use]
    pub fn switch_label(self) -> &'static str {
        match self {
            Page::Letters => "?123",
            Page::Symbols => "ABC",
        }
    }
}

/// Returns the five rows of `page`, top to bottom.
///
/// The two page tables are `LETTERS` and `SYMBOLS`, and this is the one place
/// a page is turned into rows: a caller and a test that both want "what is on
/// this page" read the same answer, rather than each keeping its own `match`.
fn rows_for(page: Page) -> &'static [&'static [KeyCap]; ROWS] {
    match page {
        Page::Letters => &LETTERS,
        Page::Symbols => &SYMBOLS,
    }
}

/// The identifier of one key on the current page.
///
/// A newtype over the key's index rather than a bare `usize`, because the index
/// is only meaningful **on the page it was taken from** and a value that can go
/// stale deserves a name: a grab made on the letters page and read after a switch
/// to the symbols page is not the key the finger is on.
/// [`Keyboard::set_page`] clears the grab for that reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyId(usize);

impl KeyId {
    /// Returns the key's index in the current page's key list.
    ///
    /// Page-local: it is an index into whichever page the keyboard is showing,
    /// and it means nothing on the other one.
    #[must_use]
    pub fn get(self) -> usize {
        self.0
    }
}

/// The colours a keyboard draws with.
///
/// Six colours, all of them existing theme tokens: the panel the keys sit on, a
/// key at rest, a key being pressed *or* being a control in its own right, and
/// the two label colours that go with those two key colours. They are not tokens
/// of their own — the theme has none per part, and adding six would put six more
/// tokens in every theme table and in every theme switch — so a keyboard is
/// themed with the theme's own six and [`Palette::from_theme`] says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The keyboard's own backing, behind the keys.
    pub panel: Color,
    /// A key at rest.
    pub key: Color,
    /// A key being pressed, and the page-switch key at rest.
    pub key_active: Color,
    /// The label on a key at rest.
    pub label: Color,
    /// The label on an active key.
    pub label_active: Color,
    /// The focus ring, drawn around the keyboard's own panel and covered by it.
    pub ring: Color,
}

impl Default for Palette {
    /// Returns a neutral grey keyboard: legible without a theme, and a visible
    /// starting point for a caller that will bind the theme's own colours.
    fn default() -> Self {
        Palette {
            panel: Color::new(20, 20, 20, 255),
            key: Color::new(64, 64, 64, 255),
            key_active: Color::new(150, 150, 150, 255),
            label: Color::new(240, 240, 240, 255),
            label_active: Color::new(20, 20, 20, 255),
            ring: Color::new(255, 255, 255, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// - The panel is [`Surface`](crate::theme::ThemeToken::Surface): a keyboard
    ///   is a surface laid over whatever is behind it.
    /// - A key at rest is [`TextMuted`](crate::theme::ThemeToken::TextMuted), and
    ///   not `Surface` again: two adjacent rounded rectangles in the same colour
    ///   have no edge between them, and the gap between keys is what tells a
    ///   driver where one key stops and the next begins.
    /// - An active key is [`Primary`](crate::theme::ThemeToken::Primary), which is
    ///   the colour every other control in this library gives to the thing being
    ///   acted on.
    /// - The two label colours are [`Text`](crate::theme::ThemeToken::Text) on a
    ///   key at rest and [`OnPrimary`](crate::theme::ThemeToken::OnPrimary) on an
    ///   active key, which by definition is the colour drawn on `Primary`.
    /// - The ring is [`Border`](crate::theme::ThemeToken::Border): the ring sits
    ///   *outside* the panel and has to be legible on the page behind it, which is
    ///   not what a label has to be legible on.
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            panel: token_color(theme, crate::theme::ThemeToken::Surface),
            key: token_color(theme, crate::theme::ThemeToken::TextMuted),
            key_active: token_color(theme, crate::theme::ThemeToken::Primary),
            label: token_color(theme, crate::theme::ThemeToken::Text),
            label_active: token_color(theme, crate::theme::ThemeToken::OnPrimary),
            ring: token_color(theme, crate::theme::ThemeToken::Border),
        }
    }
}

/// The appearance the keyboard's palette implies.
///
/// Every field is a target, not a value in flight:
/// [`animate_to_state`](Keyboard::animate_to_state) animates the keyboard's
/// properties toward this and [`paint`](Keyboard::paint) draws whatever the
/// properties have reached, which is a [`Style`] part way through on a frame
/// where something is moving.
///
/// There is no separate field per state, because the two states a key has — at
/// rest and pressed — differ only in which of these five colours they use, and
/// `key_style` is the one line that says which.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// The colour behind the keys.
    pub panel: Color,
    /// The fill of a key that is neither pressed nor a control in its own right.
    pub key: Color,
    /// The fill of a key that is.
    pub key_active: Color,
    /// The colour of a label on a resting key.
    pub label: Color,
    /// The colour of a label on an active key.
    pub label_active: Color,
}

/// An on-screen keyboard: a grid of keys, and a report of the one that was
/// pressed.
///
/// The widget holds the five colour properties a theme switch animates —
/// [`panel`](Keyboard::panel), [`key`](Keyboard::key),
/// [`key_active`](Keyboard::key_active), [`label`](Keyboard::label) and
/// [`label_active`](Keyboard::label_active) — and the one callback the task's
/// behaviour needs, [`on_key`](Keyboard::on_key). The **page** and the **press**
/// are not colours, and neither is animated: a page switch re-lays the whole grid
/// rather than moving anything between two places, and a press is written by the
/// caller's own pointer events (see the module docs on *Press state*).
///
/// The node is the caller's to keep clean and the node's box is the caller's to
/// give: [`size`](Keyboard::size) is the number that goes into a
/// [`Constraints::tight`](crate::layout::Constraints::tight) for a caller with
/// nothing else to go on, and [`paint`](Keyboard::paint) lays the keys out inside
/// whatever rect it is handed.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::node::WidgetNode;
/// use ui_core::widgets::keyboard::{Keyboard, Page};
///
/// let mut nodes = Arena::new();
/// let keyboard = Keyboard::new(&mut nodes);
/// assert!(nodes.get(keyboard.handle()).is_some());
/// assert!(!keyboard.on_key.is_set(), "no handler until one is given");
/// assert_eq!(keyboard.page(), Page::Letters);
/// ```
pub struct Keyboard {
    /// The colour behind the keys, animated between palettes.
    pub panel: Property<Color>,
    /// The fill of a key at rest, animated between palettes.
    pub key: Property<Color>,
    /// The fill of a key that is pressed or that is a control in its own right,
    /// animated between palettes.
    pub key_active: Property<Color>,
    /// The colour of a label at rest, animated alongside the key.
    pub label: Property<Color>,
    /// The colour of a label on an active key, animated with the key it is on.
    pub label_active: Property<Color>,
    /// How tall a key is drawn and hit-tested, in pixels.
    ///
    /// It is a property rather than a constant because it is the one sizing number
    /// here a caller has a real reason to change — a denser head unit — and
    /// because a value nobody can change is a value no test can exercise: with the
    /// constant alone, the height floor is unreachable and the assertion about it
    /// would pass whatever the code did. It is floored at `MIN_TOUCH_TARGET` in
    /// `key_in_row` whatever the caller writes, so a caller that asks for a
    /// 20-pixel key gets 44 and a keyboard that runs past its own box rather than
    /// one full of targets nobody can hit.
    pub key_height: Property<f32>,
    /// The font size a key's label is drawn at, in pixels.
    pub font_size: Property<f32>,
    /// The width of one character of a label, in pixels.
    ///
    /// [`paint`](Keyboard::paint) takes no font and so cannot measure a label in
    /// order to centre it, and this is the seam for the caller that has one: fill
    /// it from [`Font::advance`](crate::font::Font::advance) and the labels are
    /// centred exactly. The default is `KEY_FONT_SIZE` times
    /// `LABEL_ADVANCE_RATIO`, which is what a caller with no font gets.
    pub advance: Property<f32>,
    /// The focus ring's thickness, in pixels. Zero draws no ring even when the
    /// keyboard is focused.
    pub focus_ring: Property<f32>,
    /// Whether the keyboard holds focus. Written by the caller, from
    /// [`input::Focus`](crate::input::Focus).
    pub focused: Property<bool>,
    /// The callback a pressed key fires, with the [`KeyAction`] the key means.
    ///
    /// It is the shared [`Callback<T>`](crate::widgets::Callback), which is what a
    /// [`slider`](crate::widgets::slider::Slider)'s change notification and a
    /// [`list`](crate::widgets::list::List)'s row click made general. A keyboard
    /// whose caller has given it nothing to report to is an ordinary widget.
    pub on_key: Callback<KeyAction>,
    /// Which page is showing. Written by [`set_page`](Keyboard::set_page) rather
    /// than by the caller, because the page owns state that has to be reset with
    /// it — see that method.
    page: Property<Page>,
    /// Whether the shift key has been tapped and not yet spent.
    shifted: Cell<bool>,
    /// The key a press landed on, held until the release.
    grabbed: Cell<Option<KeyId>>,
    palette: Palette,
    /// The clock the palette's crossfade runs on. It is a clock of the keyboard's
    /// own for the reason the button's is: `AnimationClock::clear` is
    /// whole-clock, so a clock shared with another widget would strand their
    /// transitions.
    clock: RefCell<AnimationClock>,
    node: Handle,
}

/// One key as a page's table describes it: what it inserts, what it prints, and
/// how much of its row it takes.
///
/// The two text fields are separate rather than one `&'static str` because a
/// character key's label *is* its character and deriving that string would have
/// to allocate — or, in a `const`, would have to use `char::encode_utf8`, which
/// borrows a buffer the table cannot own. So a character key holds the character
/// and a named key holds the word, and there is no case where both are needed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct KeyCap {
    /// The character a character key inserts, or `None` for a named key.
    character: Option<char>,
    /// The word printed on a named key, empty for a character key.
    name: &'static str,
    /// The key's share of its row, in units where a letter is one. A letter and
    /// a space bar are both keys; this is the only thing that makes the second one
    /// wide.
    weight: f32,
    action: CapAction,
}

/// What a [`KeyCap`] does, apart from printing itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CapAction {
    /// Inserts its own character.
    Char,
    /// The page-switch key.
    Page,
    /// The fixed keys, which are distinct keys and not characters: the whole
    /// point of requirement 4's "backspace, enter, and space keys" is that they
    /// are not three more letters.
    Backspace,
    Enter,
    Space,
    Shift,
}

impl KeyCap {
    /// Returns a character key of one unit's width.
    const fn letter(character: char) -> Self {
        KeyCap {
            character: Some(character),
            name: "",
            weight: 1.0,
            action: CapAction::Char,
        }
    }

    /// Returns a named key of `weight` units' width.
    const fn named(name: &'static str, weight: f32, action: CapAction) -> Self {
        KeyCap {
            character: None,
            name,
            weight,
            action,
        }
    }
}

/// The letters page: the numbers row, three rows of letters, the bottom row.
///
/// The weights are the whole of the layout's *shape*. Row three is the nine-key
/// home row and says nothing: it divides its own width and comes out wider per key
/// than the ten-key rows, which is the large-target win the module documents.
///
/// Every glyph is ASCII on purpose: the label is a run of text handed to the glyph
/// atlas, and a glyph the font does not have draws as nothing at all — which is
/// how a pictographic backspace would come out an *empty key* rather than a key
/// with a mark on it. A word is also what a driver reads fastest.
const LETTERS: [&[KeyCap]; ROWS] = [
    &[
        KeyCap::letter('1'),
        KeyCap::letter('2'),
        KeyCap::letter('3'),
        KeyCap::letter('4'),
        KeyCap::letter('5'),
        KeyCap::letter('6'),
        KeyCap::letter('7'),
        KeyCap::letter('8'),
        KeyCap::letter('9'),
        KeyCap::letter('0'),
    ],
    &[
        KeyCap::letter('q'),
        KeyCap::letter('w'),
        KeyCap::letter('e'),
        KeyCap::letter('r'),
        KeyCap::letter('t'),
        KeyCap::letter('y'),
        KeyCap::letter('u'),
        KeyCap::letter('i'),
        KeyCap::letter('o'),
        KeyCap::letter('p'),
    ],
    &[
        KeyCap::letter('a'),
        KeyCap::letter('s'),
        KeyCap::letter('d'),
        KeyCap::letter('f'),
        KeyCap::letter('g'),
        KeyCap::letter('h'),
        KeyCap::letter('j'),
        KeyCap::letter('k'),
        KeyCap::letter('l'),
    ],
    &[
        KeyCap::named("Shift", 1.5, CapAction::Shift),
        KeyCap::letter('z'),
        KeyCap::letter('x'),
        KeyCap::letter('c'),
        KeyCap::letter('v'),
        KeyCap::letter('b'),
        KeyCap::letter('n'),
        KeyCap::letter('m'),
        KeyCap::named("Bksp", 1.5, CapAction::Backspace),
    ],
    &[
        KeyCap::named("?123", 1.5, CapAction::Page),
        KeyCap::named("Space", 5.0, CapAction::Space),
        KeyCap::named("Enter", 2.0, CapAction::Enter),
    ],
];

/// The symbols page: the numbers row, two rows of punctuation, a row of brackets
/// and the bottom row.
///
/// The punctuation is the ASCII set a URL or an address needs and a car meets
/// often — `@`, `+`, `-`, `.`, `,`, `/`, `:`, `_`, `=` — with a row of brackets
/// and pipes for the rest of it. The shift key is here as on the letters page
/// because it is what the user has learned to reach for, and because dropping it
/// would change that row's weights.
const SYMBOLS: [&[KeyCap]; ROWS] = [
    &[
        KeyCap::letter('1'),
        KeyCap::letter('2'),
        KeyCap::letter('3'),
        KeyCap::letter('4'),
        KeyCap::letter('5'),
        KeyCap::letter('6'),
        KeyCap::letter('7'),
        KeyCap::letter('8'),
        KeyCap::letter('9'),
        KeyCap::letter('0'),
    ],
    &[
        KeyCap::letter('!'),
        KeyCap::letter('"'),
        KeyCap::letter('#'),
        KeyCap::letter('$'),
        KeyCap::letter('%'),
        KeyCap::letter('&'),
        KeyCap::letter('\''),
        KeyCap::letter('('),
        KeyCap::letter(')'),
        KeyCap::letter('*'),
    ],
    &[
        KeyCap::letter('@'),
        KeyCap::letter('+'),
        KeyCap::letter('-'),
        KeyCap::letter('.'),
        KeyCap::letter(','),
        KeyCap::letter('/'),
        KeyCap::letter(':'),
        KeyCap::letter('_'),
        KeyCap::letter('='),
        KeyCap::letter('?'),
    ],
    &[
        KeyCap::named("Shift", 1.5, CapAction::Shift),
        KeyCap::letter('<'),
        KeyCap::letter('>'),
        KeyCap::letter('['),
        KeyCap::letter(']'),
        KeyCap::letter('{'),
        KeyCap::letter('}'),
        KeyCap::letter('\\'),
        KeyCap::letter('|'),
        KeyCap::named("Bksp", 1.5, CapAction::Backspace),
    ],
    &[
        KeyCap::named("ABC", 1.5, CapAction::Page),
        KeyCap::named("Space", 5.0, CapAction::Space),
        KeyCap::named("Enter", 2.0, CapAction::Enter),
    ],
];

impl Keyboard {
    /// Creates a keyboard in the arena, on the letters page, and returns it.
    ///
    /// The colours are the neutral defaults until a caller gives it a
    /// [`Palette`](Keyboard::set_palette) and calls
    /// [`snap_to_state`](Keyboard::snap_to_state) or
    /// [`animate_to_state`](Keyboard::animate_to_state); the geometry is the
    /// constants above.
    ///
    /// The task file's requirement 4 asks for the keyboard as part of a text
    /// input, and it is built on its own because it reports a [`KeyAction`]: the
    /// same widget serves every field, and no text widget has to know its
    /// geometry. Task 12's [`Button::new`](crate::widgets::button::Button::new)
    /// and task 14's [`Slider::new`](crate::widgets::slider::Slider::new) settled
    /// the same reading of a constructor that hands back a handle alone.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>) -> Self {
        let palette = Palette::default();
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Keyboard {
            panel: Property::new(palette.panel),
            key: Property::new(palette.key),
            key_active: Property::new(palette.key_active),
            label: Property::new(palette.label),
            label_active: Property::new(palette.label_active),
            key_height: Property::new(KEY_HEIGHT),
            font_size: Property::new(KEY_FONT_SIZE),
            advance: Property::new(KEY_FONT_SIZE * LABEL_ADVANCE_RATIO),
            focus_ring: Property::new(FOCUS_RING),
            focused: Property::new(false),
            on_key: Callback::none(),
            page: Property::new(Page::default()),
            shifted: Cell::new(false),
            grabbed: Cell::new(None),
            palette,
            clock: RefCell::new(AnimationClock::new()),
            node,
        }
    }

    /// Returns the keyboard's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the colours the keyboard draws with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the keyboard draws with, and leaves the current ones where
    /// they are.
    ///
    /// The appearance moves when the caller says so, by calling
    /// [`animate_to_state`](Keyboard::animate_to_state) or
    /// [`snap_to_state`](Keyboard::snap_to_state): a theme switch is animated, and
    /// a theme switch is the caller announcing a new palette and then moving the
    /// keyboard toward it. Moving the colours here would make a theme switch
    /// instantaneous and would leave the keyboard chasing a palette that is still
    /// moving.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Returns the page the keyboard is showing.
    #[must_use]
    pub fn page(&self) -> Page {
        self.page.get()
    }

    /// Shows `page`, and reports whether the keyboard moved to a different one.
    ///
    /// It is a call and not a plain public field for the reason the widget's press
    /// state is one: **a page switch has state with it**. A key's index means a
    /// different key on the other page, so a grab made before the switch would
    /// light an unrelated key after it; and a shift armed for a letter means
    /// nothing on the punctuation page. Both are cleared here rather than left for
    /// every reader of [`key_at`](Keyboard::key_at) to remember.
    ///
    /// Showing the page that is already showing changes nothing and reports
    /// `false`, which is what lets a caller mark its node dirty exactly when the
    /// keyboard's appearance has really moved.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::keyboard::{Keyboard, Page};
    ///
    /// let mut nodes = Arena::new();
    /// let keyboard = Keyboard::new(&mut nodes);
    /// assert!(keyboard.set_page(Page::Symbols), "a real switch");
    /// assert!(!keyboard.set_page(Page::Symbols), "and not a second one");
    /// assert_eq!(keyboard.page(), Page::Symbols);
    /// ```
    pub fn set_page(&self, page: Page) -> bool {
        if self.page.get() == page {
            return false;
        }
        self.page.set(page);
        self.shifted.set(false);
        self.grabbed.set(None);
        true
    }

    /// Returns whether the shift key has been tapped and not yet spent.
    #[must_use]
    pub fn is_shifted(&self) -> bool {
        self.shifted.get()
    }

    /// Returns the size a keyboard asks for: `DEFAULT_WIDTH` wide, and five rows
    /// of [`key_height`](Keyboard::key_height) with the gaps and the padding
    /// between them.
    ///
    /// A keyboard has no content to measure — its content is a fixed table of keys
    /// — so this is the number a caller with nothing else to go on puts into its
    /// node's constraints. A caller that lays the keyboard out itself gives the
    /// node whatever rect it wants through
    /// [`layout_mut`](crate::node::WidgetNode::layout_mut), and
    /// [`paint`](Keyboard::paint) lays the keys out inside whatever it is handed,
    /// flooring every key at `MIN_TOUCH_TARGET`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::keyboard::Keyboard;
    ///
    /// let mut nodes = Arena::new();
    /// let keyboard = Keyboard::new(&mut nodes);
    /// let size = keyboard.size();
    ///
    /// // 2 * 8 of padding, five rows of 52 and four gaps of 6.
    /// assert_eq!(size.height, 300.0);
    /// assert!(size.width > size.height, "a keyboard is wider than it is tall");
    /// ```
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(
            DEFAULT_WIDTH,
            KEY_PADDING * 2.0 + grid_height(self.key_height.get()),
        )
    }

    /// Returns how many keys the current page has.
    ///
    /// The two pages do not have the same count — the home row has nine keys and
    /// the punctuation row has ten — which is exactly why a key has to be
    /// identified by a [`KeyId`] rather than by its label.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.rows().iter().map(|row| row.len()).sum()
    }

    /// Returns the rect of key `index` inside `rect`, or `None` when there is no
    /// such key on the current page.
    ///
    /// This is the **only** place a key's geometry is computed. [`paint`](Keyboard::paint)
    /// draws it and [`key_at`](Keyboard::key_at) finds against it, so a change to
    /// the floor or to the gaps cannot reach one and miss the other — which is the
    /// failure the scrollbar's thickness recorded when it had two consumers and
    /// one number.
    ///
    /// The keys tile from `KEY_PADDING` down and across:
    ///
    /// - A **row** runs from the padding to the width less the padding, with
    ///   `keys - 1` gaps of `KEY_GAP` out of it, and is `KEY_HEIGHT` tall.
    /// - A **key** takes its share of what is left in proportion to its
    ///   the key's weight, so a ten-key row's letters are equal and a space bar
    ///   is five of them.
    /// - Both dimensions are floored at `MIN_TOUCH_TARGET`, and the floor
    ///   **overflows** rather than shrinks: a rect too narrow for the floor gives
    ///   keys that run past the right-hand edge and are still pressable, where a
    ///   shrink would give 20-pixel targets. A rect the caller took from
    ///   [`size`](Keyboard::size) never reaches that case.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::keyboard::Keyboard;
    ///
    /// let mut nodes = Arena::new();
    /// let keyboard = Keyboard::new(&mut nodes);
    /// let rect = Rect::new(0.0, 0.0, 720.0, 300.0);
    ///
    /// // The first key is one padding in from the top left.
    /// let first = keyboard.key_rect(0, rect).expect("the letters page has a first key");
    /// assert_eq!(first, Rect::new(8.0, 8.0, 65.0, 52.0));
    /// // And there is no key past the end of the page.
    /// assert_eq!(keyboard.key_rect(keyboard.key_count(), rect), None);
    /// ```
    #[must_use]
    pub fn key_rect(&self, index: usize, rect: Rect) -> Option<Rect> {
        let mut remaining = index;
        for (row_index, row) in self.rows().iter().enumerate() {
            if remaining < row.len() {
                return Some(self.key_in_row(remaining, row_index, row, rect));
            }
            remaining -= row.len();
        }
        None
    }

    /// Returns the rect of one key of one row inside `rect`.
    ///
    /// The row's unit width is what is left of the row after its gaps, divided by
    /// the row's total weight, and a key is that unit times its own weight. The
    /// unit is floored at zero, because a rect narrower than its own gaps would
    /// otherwise hand the keys a negative width to size themselves from.
    ///
    /// The left edges are then accumulated from the keys' **drawn** widths rather
    /// than from their shares of the unit. It is the same number twice over while
    /// no key is floored, and the only difference when one is: a row too narrow for
    /// the floor would otherwise lay its keys *on top of each other* — the third
    /// key would start inside the first — and a key drawn under another is
    /// unpressable, which is the floor defeating its own purpose. Accumulating
    /// means a floored row **overflows** instead, and every key on it is still a
    /// target of its own.
    fn key_in_row(&self, index: usize, row_index: usize, row: &[KeyCap], rect: Rect) -> Rect {
        let height = self.key_height.get();
        let gaps = KEY_GAP * count_to_f32(row.len().saturating_sub(1));
        let total_weight: f32 = row.iter().map(|cap| cap.weight).sum();
        let available = (rect.width - KEY_PADDING * 2.0 - gaps).max(0.0);
        let unit = if total_weight > 0.0 {
            available / total_weight
        } else {
            0.0
        };
        let before: f32 = row[..index]
            .iter()
            .map(|cap| (unit * cap.weight).max(MIN_TOUCH_TARGET) + KEY_GAP)
            .sum();
        Rect::new(
            rect.x + KEY_PADDING + before,
            rect.y + KEY_PADDING + count_to_f32(row_index) * (height + KEY_GAP),
            (unit * row[index].weight).max(MIN_TOUCH_TARGET),
            height.max(MIN_TOUCH_TARGET),
        )
    }

    /// Returns the key under `position` inside `rect`, or `None` when there is
    /// none — a point outside the keyboard, or in one of the gaps between keys.
    ///
    /// The gaps are the point of saying `None`. A keyboard whose keys tiled edge
    /// to edge would have no gap to miss, so there would be nothing to decide
    /// here; the gap is what makes a tap that hit no key a tap that hit *nothing*,
    /// and [`on_event`](Keyboard::on_event) leaves it unconsumed for the panel
    /// behind. A key's edges belong to the key, which is
    /// [`input::contains`](crate::input)'s convention rather than a choice here.
    ///
    /// Keys are tested in reading order and the first that contains the point
    /// wins, so where a row overflows onto the one below it, the row above wins —
    /// the one the finger arrived from.
    #[must_use]
    pub fn key_at(&self, position: Offset, rect: Rect) -> Option<KeyId> {
        (0..self.key_count())
            .filter_map(|index| self.key_rect(index, rect).map(|key| (index, key)))
            .find(|(_, key)| covers(*key, position))
            .map(|(index, _)| KeyId(index))
    }

    /// Returns the label drawn on `id`, in the case it is drawn in, or `None`
    /// when there is no such key.
    ///
    /// It is the string [`paint`](Keyboard::paint) records, which is what makes it
    /// worth having: a caller drawing a hint beside the keyboard, or a test asking
    /// what the bottom-left key says, both want the label rather than the action
    /// behind it.
    #[must_use]
    pub fn key_label(&self, id: KeyId) -> Option<String> {
        let cap = self.cap(id.get())?;
        Some(match cap.action {
            CapAction::Page => self.page.get().switch_label().to_string(),
            CapAction::Char => {
                let character = cap.character?;
                if self.shifted.get() {
                    character.to_ascii_uppercase().to_string()
                } else {
                    character.to_string()
                }
            }
            _ => cap.name.to_string(),
        })
    }

    /// Returns the action `id` means, or `None` when there is no such key.
    ///
    /// It is the **pure** answer and it does not spend a shift: a caller that wants
    /// to know what a key would do without the keyboard having done it asks here,
    /// and only the press path in [`on_event`](Keyboard::on_event) spends the
    /// shift, so a key's meaning is the same whether it was pressed or only looked
    /// at.
    ///
    /// A [`Char`](KeyAction::Char) comes back in the case the key is *drawn* in,
    /// which is what makes the shift the keyboard's business and not the caller's:
    /// the caller inserts the character it is given and cannot get the case wrong.
    #[must_use]
    pub fn key_action(&self, id: KeyId) -> Option<KeyAction> {
        let cap = self.cap(id.get())?;
        Some(match cap.action {
            CapAction::Char => {
                let character = cap.character?;
                if self.shifted.get() {
                    KeyAction::Char(character.to_ascii_uppercase())
                } else {
                    KeyAction::Char(character)
                }
            }
            CapAction::Backspace => KeyAction::Backspace,
            CapAction::Enter => KeyAction::Enter,
            CapAction::Space => KeyAction::Space,
            CapAction::Shift => KeyAction::Shift,
            CapAction::Page => match self.page.get() {
                Page::Letters => KeyAction::PageUp,
                Page::Symbols => KeyAction::PageDown,
            },
        })
    }

    /// Returns the key at `index` on the current page, or `None`.
    fn cap(&self, index: usize) -> Option<&'static KeyCap> {
        let mut remaining = index;
        for row in self.rows() {
            if remaining < row.len() {
                return row.get(remaining);
            }
            remaining -= row.len();
        }
        None
    }

    /// Returns the five rows of the current page, top to bottom.
    fn rows(&self) -> &'static [&'static [KeyCap]; ROWS] {
        rows_for(self.page.get())
    }

    /// Returns the two colours one key is **drawn** in.
    ///
    /// It reads the animated properties rather than [`style`](Keyboard::style), so
    /// a theme switch reaches a pressed key exactly as it reaches a resting one:
    /// a lit key left on the previous palette would be a colour nothing else on
    /// screen uses.
    ///
    /// An **active** key is one being pressed, and — the second user of the same
    /// two colours — the page-switch key at rest. Painting the switch in the active
    /// colours is what makes the layer discoverable: a control the user has to
    /// already know about is a control half the users never find, and there is
    /// nothing else on a keyboard to carry that state.
    fn key_style(&self, active: bool) -> (Color, Color) {
        if active {
            (self.key_active.get(), self.label_active.get())
        } else {
            (self.key.get(), self.label.get())
        }
    }

    /// Returns the appearance the keyboard's palette implies.
    ///
    /// It is the **target**, read from the palette rather than from the animated
    /// properties, which is the split the whole module is built on: the properties
    /// are what is *drawn* and they chase this. The pressed and switch keys are
    /// per-key and come from `key_style`, so this is the
    /// resting appearance and nothing else.
    ///
    /// It is here because [`snap_to_state`](Keyboard::snap_to_state) and
    /// [`animate_to_state`](Keyboard::animate_to_state) both read it, and a caller
    /// that wants to see where a transition is going reads it too.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::keyboard::{Keyboard, Palette};
    ///
    /// let mut nodes = Arena::new();
    /// let mut keyboard = Keyboard::new(&mut nodes);
    /// let themed = Palette::from_theme(&Theme::dark());
    /// keyboard.set_palette(themed);
    ///
    /// // The target is the new palette, whatever the properties still hold.
    /// assert_eq!(keyboard.style().panel, themed.panel);
    /// assert_ne!(keyboard.style().panel, keyboard.panel.get());
    /// ```
    #[must_use]
    pub fn style(&self) -> Style {
        Style {
            panel: self.palette.panel,
            key: self.palette.key,
            key_active: self.palette.key_active,
            label: self.palette.label,
            label_active: self.palette.label_active,
        }
    }

    /// Applies the appearance the keyboard's palette implies at once, with no
    /// transition.
    ///
    /// This is what a caller wants in the two places a transition is the wrong
    /// answer: a keyboard that has just been given a
    /// [`Palette`](Keyboard::set_palette) and has never animated — whose colour
    /// properties still hold the neutral defaults [`Keyboard::new`] wrote, so
    /// without this a themed keyboard starts out grey — and a caller that has
    /// written a state of its own and wants the keyboard to be that state now.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::keyboard::{Keyboard, Palette};
    ///
    /// let mut nodes = Arena::new();
    /// let mut keyboard = Keyboard::new(&mut nodes);
    /// let themed = Palette::from_theme(&Theme::dark());
    /// keyboard.set_palette(themed);
    ///
    /// // Aiming alone would leave the keyboard grey until the transition ran.
    /// keyboard.snap_to_state();
    /// assert_eq!(keyboard.panel.get(), themed.panel);
    /// assert!(!keyboard.is_animating(), "and a snap is not a transition");
    /// ```
    pub fn snap_to_state(&self) {
        let style = self.style();
        self.clock.borrow_mut().clear();
        self.panel.set(style.panel);
        self.key.set(style.key);
        self.key_active.set(style.key_active);
        self.label.set(style.label);
        self.label_active.set(style.label_active);
    }

    /// Starts the transitions that carry the keyboard's colours from wherever they
    /// are toward the palette's, on `motion`.
    ///
    /// The slide and the colour crossfade both run on the motion the caller hands
    /// over — the theme's `DurationFast` and `EasingStandard` — so a keyboard's
    /// transition is the theme's transition and not a number written here.
    ///
    /// The keyboard's own clock is cleared first, so a transition this replaces
    /// stops where it is rather than writing over the new one when it arrives, and
    /// so a caller that aims on every frame does not leave one animation per aim
    /// on a clock that keeps them all.
    ///
    /// The target is a snapshot, not a continuous one: a caller re-aims when its
    /// palette moves, exactly as the demo re-aims a button's press when the theme
    /// moves under it. Re-aiming without a change restarts each colour from
    /// wherever it is, so a caller that aims on every frame sees a transition that
    /// *approaches* its target rather than one that runs to its end —
    /// [`Slider::animate_to_state`](crate::widgets::slider::Slider::animate_to_state)
    /// has the same property, for the same reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::keyboard::{Keyboard, Palette};
    ///
    /// let mut nodes = Arena::new();
    /// let mut keyboard = Keyboard::new(&mut nodes);
    /// keyboard.set_palette(Palette::from_theme(&Theme::dark()));
    /// keyboard.animate_to_state(Motion {
    ///     duration: Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    /// keyboard.tick(Duration::from_millis(50));
    /// assert_ne!(keyboard.panel.get(), keyboard.palette().panel, "half way");
    /// ```
    pub fn animate_to_state(&self, motion: Motion) {
        let style = self.style();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.panel
                .animate_to(style.panel, motion.duration, motion.easing),
        );
        clock.add(
            self.key
                .animate_to(style.key, motion.duration, motion.easing),
        );
        clock.add(
            self.key_active
                .animate_to(style.key_active, motion.duration, motion.easing),
        );
        clock.add(
            self.label
                .animate_to(style.label, motion.duration, motion.easing),
        );
        clock.add(
            self.label_active
                .animate_to(style.label_active, motion.duration, motion.easing),
        );
    }

    /// Advances the keyboard's transitions by `delta`, and returns whether any of
    /// them wrote.
    ///
    /// It is the keyboard's frame integration: call it once a frame, before the
    /// paint pass, with the time that frame took. The delta is always **passed
    /// in**; the widget never reads a wall clock, which is what makes its
    /// transitions testable without a display and a time.
    ///
    /// The write is what reaches the node — a property callback registered by the
    /// caller marks it dirty — so a caller that repaints only when this is true
    /// repaints exactly while something moves.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Returns whether any of the keyboard's transitions is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Records that a pointer at `position` has pressed the key under it, and
    /// reports whether it hit one.
    ///
    /// This is the **press** half of a key press, and it is a call rather than an
    /// event because the gesture recogniser has no press to give: it reports a
    /// [`Tap`](InputEventKind::Tap) on the *release*, and a
    /// [`Drag`](InputEventKind::Drag) only once the pointer has already moved. A
    /// widget cannot recover "the finger went down on this key" from either, which
    /// is why it is written by the caller here for the same reason a
    /// [`Button`](crate::widgets::button::Button)'s `pressed` and a
    /// [`Slider`](crate::widgets::slider::Slider)'s `dragging` are — the demo
    /// writes all three in the same handler.
    ///
    /// The press is held until [`release_key`](Keyboard::release_key), and the
    /// widget cannot release it itself: a release is as invisible to it as the
    /// press was.
    ///
    /// A press in a gap, or outside the keyboard, grabs nothing and reports
    /// `false` — and clears a grab that was there, because the new press is the
    /// whole of the pointer's state now.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::layout::Offset;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::keyboard::Keyboard;
    ///
    /// let mut nodes = Arena::new();
    /// let keyboard = Keyboard::new(&mut nodes);
    /// let rect = Rect::new(0.0, 0.0, 720.0, 300.0);
    ///
    /// // The `1` key is 8 across and 65 wide.
    /// assert!(keyboard.grab_key(Offset::new(20.0, 20.0), rect));
    /// assert!(keyboard.is_key_grabbed());
    /// assert_eq!(
    ///     keyboard.grabbed_key().map(|id| id.get()),
    ///     Some(0),
    ///     "and it is the `1` key"
    /// );
    ///
    /// // The gap to the right of it is not a key.
    /// assert!(!keyboard.grab_key(Offset::new(75.0, 20.0), rect));
    /// assert!(!keyboard.is_key_grabbed(), "and a miss grabs nothing");
    /// ```
    pub fn grab_key(&self, position: Offset, rect: Rect) -> bool {
        let Some(id) = self.key_at(position, rect) else {
            self.grabbed.set(None);
            return false;
        };
        self.grabbed.set(Some(id));
        true
    }

    /// Gives up the press recorded by [`grab_key`](Keyboard::grab_key), and
    /// reports whether there was one.
    ///
    /// A caller calls this from its pointer release for the same reason it calls
    /// [`grab_key`](Keyboard::grab_key) from its press: the recogniser has no
    /// release for the gesture that produced no press, so a grab left behind would
    /// be the next gesture's, and a key would stay lit after the finger left.
    pub fn release_key(&self) -> bool {
        self.grabbed.replace(None).is_some()
    }

    /// Returns whether a key is currently pressed.
    #[must_use]
    pub fn is_key_grabbed(&self) -> bool {
        self.grabbed.get().is_some()
    }

    /// Returns the key a press landed on, or `None` when nothing is pressed.
    ///
    /// It is page-local and [`set_page`](Keyboard::set_page) clears it, so it
    /// always names the key the finger is on.
    #[must_use]
    pub fn grabbed_key(&self) -> Option<KeyId> {
        self.grabbed.get()
    }

    /// Handles `event` as this keyboard would inside `rect`, and reports whether it
    /// consumed it.
    ///
    /// A [`Tap`](InputEventKind::Tap) on a key reports that key's [`KeyAction`]
    /// through [`on_key`](Keyboard::on_key) and is consumed. A tap that hit no key
    /// — in a gap between keys, or outside the keyboard — is **neither** consumed
    /// nor reported: it was aimed at nothing of this widget's, and letting it
    /// through to the panel behind is what "consumed" means. It is the
    /// [`List`](crate::widgets::list::List)'s rule for the tap between two rows,
    /// for the same reason: the gap exists.
    ///
    /// A tap does **not** release the press: the release is the caller's, from its
    /// own pointer-up, and by the time a tap arrives the finger has already gone.
    /// It does spend a shift, because a shift is a *sticky modifier of the
    /// keyboard* and a second letter typed without another tap on the shift must
    /// not come out upper case.
    ///
    /// Every other event is left alone and not consumed, so it carries on up the
    /// tree. That includes [`InputEventKind::Text`], which is the text widget's
    /// business and not this one's: a hardware keyboard's characters belong to the
    /// field that holds focus, and a keyboard that consumed them would put them
    /// nowhere.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind};
    /// use ui_core::layout::Offset;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::keyboard::{KeyAction, Keyboard};
    /// use ui_core::widgets::Callback;
    ///
    /// let mut nodes = Arena::new();
    /// let mut keyboard = Keyboard::new(&mut nodes);
    /// let rect = Rect::new(0.0, 0.0, 720.0, 300.0);
    /// let seen = Rc::new(RefCell::new(Vec::new()));
    /// let reported = Rc::clone(&seen);
    /// keyboard.on_key = Callback::from_fn(move |action| reported.borrow_mut().push(action));
    ///
    /// // The bottom-left key says where it goes, and reports that it went there.
    /// let at_switch = Offset::new(60.0, 266.0);
    /// let mut tap = InputEvent::new(InputEventKind::Tap, Some(at_switch));
    /// assert!(keyboard.on_event(&mut tap, rect));
    /// assert_eq!(seen.borrow().as_slice(), &[KeyAction::PageUp]);
    ///
    /// // And a typed text event belongs to somebody else.
    /// let mut typed = InputEvent::new(InputEventKind::Text { text: "a".to_string() }, None);
    /// assert!(!keyboard.on_event(&mut typed, rect));
    /// assert!(!typed.consumed());
    /// ```
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        match event.kind() {
            InputEventKind::Tap => {
                let Some(position) = event.position() else {
                    return false;
                };
                let Some(id) = self.key_at(position, rect) else {
                    return false;
                };
                let Some(action) = self.key_action(id) else {
                    return false;
                };
                event.consume();
                match action {
                    // A shift is spent by the character it was armed for and by
                    // nothing else: `A` then `a` is what a user means after one tap
                    // on it, and a shift that locked would have to be tapped twice
                    // to get back down.
                    KeyAction::Char(_) => self.shifted.set(false),
                    KeyAction::Shift => {
                        let armed = !self.shifted.get();
                        self.shifted.set(armed);
                    }
                    _ => {}
                }
                self.on_key.call(action);
                true
            }
            _ => false,
        }
    }

    /// Returns the draw commands that paint the keyboard inside `rect`.
    ///
    /// The commands are, in order: the focus ring, if the keyboard is focused and
    /// the ring is not zero wide; the panel; then, per key in reading order, the
    /// key's rounded rectangle and its label.
    ///
    /// The panel is drawn **over** the focus ring, and that is the reason the ring
    /// reads as an outline rather than as a card: a [`DrawCommand::RoundedRect`]
    /// *fills* its rect, so a ring drawn around the keyboard's whole rect and left
    /// on top is a filled panel with a border around it rather than a focus
    /// indicator. The ring goes down first, the panel over it, and only the ring's
    /// own border is left — the trick [`Button`](crate::widgets::button::Button)
    /// uses to draw an outline at all.
    ///
    /// Every key's rect is [`key_rect`](Keyboard::key_rect)'s, which is the same
    /// call [`key_at`](Keyboard::key_at) makes, so what is drawn and what is found
    /// cannot come apart.
    ///
    /// A label is centred by [`advance`](Keyboard::advance) rather than by a
    /// measurement, because `paint` takes no font; see that property for what a
    /// caller with a font should put in it.
    #[must_use]
    pub fn paint(&self, rect: Rect) -> Vec<DrawCommand> {
        let mut painter = Painter::new();
        let ring = self.focus_ring.get();
        if ring > 0.0 && self.focused.get() {
            painter.rounded_rect(grow(rect, ring), KEY_RADIUS + ring, self.palette.ring);
        }
        painter.rounded_rect(rect, KEY_RADIUS, self.panel.get());

        let font_size = self.font_size.get();
        let advance = self.advance.get();
        for index in 0..self.key_count() {
            let Some(key) = self.key_rect(index, rect) else {
                continue;
            };
            let id = KeyId(index);
            let active = self.grabbed.get() == Some(id)
                || self.cap(index).map(|cap| cap.action) == Some(CapAction::Page);
            let (fill, ink) = self.key_style(active);
            painter.rounded_rect(key, KEY_RADIUS, fill);
            let Some(label) = self.key_label(id) else {
                continue;
            };
            let text_width = advance * count_to_f32(label.chars().count());
            painter.text(
                key.x + (key.width - text_width) / 2.0,
                key.y + (key.height - font_size) / 2.0,
                &label,
                ink,
                font_size,
                0.0,
            );
        }
        painter.finish()
    }
}

/// The height of the grid alone: five rows `height` tall and the gaps between
/// them, with no padding. The half of [`Keyboard::size`] that is not the width.
///
/// It takes the height rather than reading the constant, because a caller may
/// have changed it and a `size` that disagreed with the keys it sizes is a node
/// the layout pass places one row short of the bottom.
fn grid_height(height: f32) -> f32 {
    count_to_f32(ROWS) * height + count_to_f32(ROWS.saturating_sub(1)) * KEY_GAP
}

/// Converts a row or key count to the float the layout arithmetic uses.
///
/// `f32` has no `From<usize>` in std — the `From` impls between integers stop at
/// the 16-bit widths and no float conversion is provided at all — so this is the
/// one place the cast happens, for the reason
/// [`List`](crate::widgets::list::List)'s own `count_to_f32` gives and which is
/// the layout module's private one. The conversion is well defined for every
/// `usize`: the result rounds to the nearest `f32`, and a keyboard with more keys
/// than that rounding matters for is not a keyboard this arena can draw.
fn count_to_f32(count: usize) -> f32 {
    count as f32
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback for
/// a token a caller has written the wrong variant into — and black rather than a
/// panic, because a mistyped theme token is not worth taking a frame down for. It
/// is the slider's own helper, repeated rather than imported, for the reason
/// [`count_to_f32`] gives.
fn token_color(theme: &Theme, token: crate::theme::ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

/// Returns `rect` grown by `by` on every side. A negative `by` insets it.
fn grow(rect: Rect, by: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// Returns whether `point` is inside `rect`, edges included.
///
/// The input module has a `contains` of its own and it is private there; this is
/// two comparisons, repeated rather than made public for one caller, the way
/// `token_color` is repeated in the slider, the toggle and the scroll.
fn covers(rect: Rect, point: Offset) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Easing;
    use crate::layout::{Constraints, Layout, LayoutState};
    use crate::theme::{PropertyValue, ThemeToken};
    use std::rc::Rc;

    /// The box most of the geometry tests lay a keyboard out in: its own
    /// [`Keyboard::size`], so the grid fills it and the arithmetic is whole
    /// numbers.
    const RECT: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 720.0,
        height: 300.0,
    };

    /// The same keyboard somewhere else in a window: 720 by 300 at (280, 380).
    /// Every other geometry fixture here is at the origin, so this is the one that
    /// catches an origin being read as an extent — the defect the slider's module
    /// shipped past 53 green tests, and the reason this fixture exists.
    const OFFSET_RECT: Rect = Rect {
        x: 280.0,
        y: 380.0,
        width: 720.0,
        height: 300.0,
    };

    /// The ten digits, in the order the numbers row shows them.
    const DIGITS: &str = "1234567890";

    /// A frame's worth of time.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// The motion a test animates on: a fixed 100 ms on a linear curve, so a value
    /// at a given tick is the one the closed form gives and not a curve's.
    fn motion() -> Motion {
        Motion {
            duration: ms(100),
            easing: Easing::Linear,
        }
    }

    /// A keyboard in the dark theme's palette, with its appearance already arrived
    /// at that palette.
    ///
    /// Setting a palette does not move the keyboard by itself — a theme switch is
    /// animated — so a test that wants a keyboard *in* a palette asks for the snap.
    fn keyboard() -> (Arena<WidgetNode>, Keyboard) {
        let mut nodes = Arena::new();
        let mut keyboard = Keyboard::new(&mut nodes);
        keyboard.set_palette(Palette::from_theme(&Theme::dark()));
        keyboard.snap_to_state();
        (nodes, keyboard)
    }

    /// A tap at `(x, y)`.
    fn tap_at(x: f32, y: f32) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)))
    }

    /// A counter of the actions the callback was given, in order.
    fn actions(keyboard: &mut Keyboard) -> Rc<RefCell<Vec<KeyAction>>> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let recorded = Rc::clone(&seen);
        keyboard.on_key = Callback::from_fn(move |action| recorded.borrow_mut().push(action));
        seen
    }

    /// The rounded rectangles a paint recorded, with their radii and colours.
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

    /// The text runs a paint recorded, with their position and their string.
    fn labels(commands: &[DrawCommand]) -> Vec<(f32, f32, String, Color)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    x, y, text, color, ..
                } => Some((*x, *y, text.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    /// A short name for each recorded command, in the order they were recorded.
    ///
    /// It cannot see sizes, so it cannot tell one key from another; the tests that
    /// care about *which* key assert on rects instead, and the two that care about
    /// order assert this alongside them.
    fn shapes(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::RoundedRect { .. } => "key",
                DrawCommand::Text { .. } => "label",
                _ => "other",
            })
            .collect()
    }

    /// Asserts two values are within a thousandth of a pixel of one another.
    ///
    /// A key's width out of a row of ten is a fraction of a pixel, because the row
    /// arithmetic is computed in `f32`: a test that wrote the decimal out to the
    /// last digit would be asserting the compiler's rounding rather than the
    /// widget. Everything the layout computes exactly — the ten-key rows' 65
    /// pixels, the five row offsets — is asserted with `assert_eq!`.
    fn assert_close(got: f32, want: f32, what: &str) {
        assert!((got - want).abs() < 1e-3, "{what}: {got} against {want}");
    }

    /// Returns the rounded rectangle a paint recorded **at** `where`, with its
    /// radius and colour.
    ///
    /// Matched on the rect rather than on the order, because "the key that is drawn
    /// there" is the question a caller and a reviewer both ask; an assertion on a
    /// command's index would say "the eleventh command", which is the same answer
    /// with the geometry thrown away.
    fn painted_in(commands: &[DrawCommand], where_: Rect) -> Option<(Rect, f32, Color)> {
        rounded(commands)
            .into_iter()
            .find(|(rect, _, _)| *rect == where_)
    }

    /// Hangs `keyboard` on a panel and lays it out, and returns the panel's handle
    /// and the rect its node was placed at.
    ///
    /// The panel is a stack, so the keyboard sits at its origin, and the keyboard's
    /// own node is given the box [`Keyboard::size`] asks for. A tap therefore has a
    /// real hit test to pass or fail, and a node behind the keyboard to reach if it
    /// does not.
    fn on_a_panel(
        keyboard: &Keyboard,
        nodes: &mut Arena<WidgetNode>,
        panel_size: Size,
    ) -> (Handle, Rect) {
        let panel = node::create(nodes, LayoutState::new());
        assert!(
            node::attach(nodes, panel, keyboard.handle()),
            "the keyboard is hung on the panel"
        );
        let size = keyboard.size();
        if let Some(node) = nodes.get_mut(keyboard.handle()) {
            node.layout_mut().set_constraints(Constraints::tight(size));
        }
        Layout::new(nodes).layout(panel, Constraints::tight(panel_size));
        let placed = nodes
            .get(keyboard.handle())
            .and_then(|node| node.layout().rect())
            .map(|rect| {
                Rect::new(
                    rect.origin.x,
                    rect.origin.y,
                    rect.size.width,
                    rect.size.height,
                )
            })
            .unwrap_or(Rect::new(0.0, 0.0, size.width, size.height));
        (panel, placed)
    }

    /// Returns the centre of `rect`, which is inside it for any rect with a
    /// positive extent.
    fn centre(rect: Rect) -> Offset {
        Offset::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    /// Returns the index of the first key of `row` on `page`.
    ///
    /// The rows are not all the same length — the home row has nine keys and the
    /// punctuation rows ten — so a row's first index is a sum, and this is that
    /// sum written once.
    fn row_start(page: Page, row: usize) -> usize {
        rows_for(page)[..row].iter().map(|keys| keys.len()).sum()
    }

    /// Returns the key of `action` on the keyboard, with no gesture involved: the
    /// index of the first key that means it.
    fn key_meaning(keyboard: &Keyboard, action: KeyAction) -> KeyId {
        (0..keyboard.key_count())
            .filter_map(|index| keyboard.key_action(KeyId(index)))
            .position(|candidate| candidate == action)
            .map(KeyId)
            .unwrap_or(KeyId(usize::MAX))
    }

    /// Presses the key drawn at `rect` — the gesture, not a fixture near it.
    ///
    /// Every control this module draws is operated through here, so a test that
    /// says a key is pressable is pressing where it is *drawn*, which is the rule
    /// the scrollbar's dead thumb was found by.
    fn press(keyboard: &Keyboard, rect: Rect, drawn: Rect) -> bool {
        keyboard.grab_key(centre(drawn), rect)
    }

    // -------------------------------------------------- the widget's own shape

    #[test]
    fn a_keyboard_has_a_node_of_its_own_in_the_arena() {
        let (nodes, keyboard) = keyboard();
        assert!(nodes.get(keyboard.handle()).is_some());
    }

    #[test]
    fn a_keyboard_starts_on_letters_with_no_handler_and_nothing_pressed() {
        let (_nodes, keyboard) = keyboard();
        assert_eq!(
            keyboard.page(),
            Page::Letters,
            "a keyboard opens on letters"
        );
        assert!(
            !keyboard.on_key.is_set(),
            "and no handler until one is given"
        );
        assert!(!keyboard.is_key_grabbed(), "with nothing pressed");
        assert!(!keyboard.is_shifted(), "and no shift armed");
        assert!(!keyboard.is_animating(), "and nothing left running");
    }

    #[test]
    fn a_keyboard_that_has_never_been_aimed_paints_the_neutral_defaults() {
        // The other half of the themed-keyboard defect: a caller who never themes a
        // keyboard still gets something visible, which is the default palette's
        // job.
        let mut nodes = Arena::new();
        let keyboard = Keyboard::new(&mut nodes);
        assert_eq!(keyboard.panel.get(), Palette::default().panel);
        assert_eq!(keyboard.key.get(), Palette::default().key);
        assert_eq!(
            rounded(&keyboard.paint(RECT))
                .first()
                .map(|(_, _, color)| *color),
            Some(Palette::default().panel)
        );
    }

    #[test]
    fn the_default_geometry_is_the_numbers_this_module_documents() {
        // These are constants, not derived numbers, and the layout, the touch floor
        // and the focus ring all hang off them.
        assert_eq!(MIN_TOUCH_TARGET, 44.0, "the floor a finger can hit");
        assert_eq!(KEY_HEIGHT, 52.0, "the floor plus a finger of margin");
        assert_eq!(KEY_GAP, 6.0);
        assert_eq!(KEY_PADDING, 8.0);
        assert_eq!(KEY_RADIUS, 8.0, "the theme's own BorderRadiusMd");
        assert_eq!(FOCUS_RING, 2.0);
        assert_eq!(DEFAULT_WIDTH, 720.0);
        assert_eq!(KEY_FONT_SIZE, 22.0);
        assert_eq!(LABEL_ADVANCE_RATIO, 0.6);
        assert_eq!(ROWS, 5, "a numbers row, three of letters and a bottom row");
    }

    // -------------------------------------------------- the layout arithmetic

    #[test]
    fn a_keyboard_asks_for_a_size_that_holds_five_rows_over_the_floor() {
        let (_nodes, keyboard) = keyboard();
        let size = keyboard.size();
        assert_eq!(size.width, DEFAULT_WIDTH);
        // 2 * 8 of padding, five rows of 52 and four gaps of 6.
        assert_eq!(size.height, 300.0);
        assert_close(
            grid_height(keyboard.key_height.get()),
            284.0,
            "the grid alone, without padding",
        );
        assert!(
            size.height > count_to_f32(ROWS) * MIN_TOUCH_TARGET,
            "and it is five rows over the floor: {size:?}"
        );
    }

    #[test]
    fn the_first_key_is_the_one_and_one_padding_in() {
        let (_nodes, keyboard) = keyboard();
        let first = keyboard.key_rect(0, RECT);
        assert_eq!(
            first,
            Some(Rect::new(8.0, 8.0, 65.0, 52.0)),
            "the `1` key, 8 in from the top left"
        );
    }

    #[test]
    fn the_rows_tile_down_the_keyboard_with_one_gap_between_them() {
        let (_nodes, keyboard) = keyboard();
        // 52 tall with a 6 gap is 58 per row, from 8 down.
        for (row, top) in [8.0_f32, 66.0, 124.0, 182.0, 240.0].into_iter().enumerate() {
            let key = keyboard
                .key_rect(row_start(Page::Letters, row), RECT)
                .unwrap_or(RECT);
            assert_eq!(key.y, top, "row {row} starts at {top}");
            assert_eq!(key.height, KEY_HEIGHT, "and is a key tall");
        }
    }

    #[test]
    fn the_last_row_ends_one_padding_above_the_keyboards_own_bottom_edge() {
        let (_nodes, keyboard) = keyboard();
        let key = keyboard
            .key_rect(row_start(Page::Letters, 4), RECT)
            .unwrap_or(RECT);
        assert_eq!(key.y + key.height, 292.0, "the bottom row's lower edge");
        assert_eq!(
            RECT.height - (key.y + key.height),
            KEY_PADDING,
            "which is one padding inside the box"
        );
    }

    #[test]
    fn a_ten_key_row_divides_its_width_into_ten_equal_keys() {
        // (720 - 16 of padding - 9 gaps of 6) / 10 = 65 exactly.
        let (_nodes, keyboard) = keyboard();
        for index in 0..10 {
            assert_eq!(
                keyboard.key_rect(index, RECT).map(|key| key.width),
                Some(65.0),
                "key {index} is 65 wide"
            );
        }
        assert_eq!(
            keyboard.key_rect(9, RECT).map(|key| key.x + key.width),
            Some(712.0),
            "and the tenth ends one padding in from the right"
        );
    }

    #[test]
    fn two_keys_in_a_row_are_one_gap_apart() {
        let (_nodes, keyboard) = keyboard();
        let first = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        let second = keyboard.key_rect(1, RECT).unwrap_or(RECT);
        assert_eq!(second.x - (first.x + first.width), KEY_GAP);
    }

    #[test]
    fn no_two_keys_in_the_keyboard_touch() {
        let (_nodes, keyboard) = keyboard();
        let keys: Vec<Rect> = (0..keyboard.key_count())
            .filter_map(|index| keyboard.key_rect(index, RECT))
            .collect();
        for (index, first) in keys.iter().enumerate() {
            for second in keys.iter().skip(index + 1) {
                let apart = first.x + first.width <= second.x
                    || second.x + second.width <= first.x
                    || first.y + first.height <= second.y
                    || second.y + second.height <= first.y;
                assert!(
                    apart,
                    "the keys at {index} and {:?} overlap: {first:?} against {second:?}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn the_nine_key_home_row_gives_every_one_of_its_keys_more_room() {
        // The short row divides the same space among fewer keys:
        // (720 - 16 - 8 gaps of 6) / 9 = 72.888...
        let (_nodes, keyboard) = keyboard();
        let ten_wide = keyboard
            .key_rect(0, RECT)
            .map(|key| key.width)
            .unwrap_or(0.0);
        let home = row_start(Page::Letters, 2);
        let nine_wide = keyboard
            .key_rect(home, RECT)
            .map(|key| key.width)
            .unwrap_or(0.0);
        assert_close(nine_wide, 72.888, "the home row's keys are wider");
        assert!(nine_wide > ten_wide, "{nine_wide} against {ten_wide}");
    }

    #[test]
    fn a_wide_key_gets_its_share_of_the_weight_and_its_neighbour_does_not() {
        // The bottom row is 1.5 + 5 + 2 = 8.5 units over (720 - 16 - 2 gaps of 6),
        // so a unit is 81.41 and the space bar is five of them.
        let (_nodes, keyboard) = keyboard();
        let base = row_start(Page::Letters, 4);
        let switch = keyboard.key_rect(base, RECT).unwrap_or(RECT);
        let space = keyboard.key_rect(base + 1, RECT).unwrap_or(RECT);
        let enter = keyboard.key_rect(base + 2, RECT).unwrap_or(RECT);
        assert_close(space.width, 407.0588, "the space bar is five units");
        assert_close(enter.width, 162.8235, "enter is two");
        assert_close(switch.width, 122.1176, "and the switch is one and a half");
        assert_close(
            space.width + enter.width + switch.width,
            692.0,
            "and the three of them account for the whole row",
        );
        assert!(
            space.width > enter.width && enter.width > switch.width,
            "and the widths are in the order the weights are"
        );
    }

    #[test]
    fn a_keyboards_own_width_is_shared_out_exactly() {
        // The ten-key rows account for every pixel of the row: 10 keys of 65 and
        // nine gaps of 6 is 704, which is the width less the padding on both sides.
        let ten = count_to_f32(10) * 65.0 + count_to_f32(9) * KEY_GAP;
        assert_eq!(ten, 704.0);
        assert_eq!(ten, RECT.width - KEY_PADDING * 2.0);
    }

    #[test]
    fn there_is_no_key_past_the_end_of_the_page() {
        let (_nodes, keyboard) = keyboard();
        assert_eq!(keyboard.key_count(), 41, "10 + 10 + 9 + 9 + 3");
        assert_eq!(keyboard.key_rect(keyboard.key_count(), RECT), None);
        assert_eq!(
            keyboard.key_rect(keyboard.key_count() + 100, RECT),
            None,
            "and no key either for an index well past it"
        );
    }

    #[test]
    fn a_keyboard_away_from_the_origin_puts_its_whole_grid_elsewhere() {
        // Every other geometry fixture here is at (0, 0), which is a blind spot: a
        // rect's origin and a rect's extent are different numbers, and only a
        // keyboard somewhere else on the window tells them apart. This is the
        // fixture the slider's module was missing, and the defect it would have
        // caught was in `travel`.
        let (_nodes, keyboard) = keyboard();
        assert_eq!(OFFSET_RECT.x, 280.0, "the fixture really is off the origin");

        assert_eq!(
            keyboard.key_rect(0, OFFSET_RECT),
            Some(Rect::new(288.0, 388.0, 65.0, 52.0)),
            "the first key is 8 in from the keyboard's own edge, not the window's"
        );
        assert_eq!(
            keyboard
                .key_rect(9, OFFSET_RECT)
                .map(|key| key.x + key.width),
            Some(992.0),
            "and the tenth ends at 712 past the keyboard's own left edge"
        );
        assert_eq!(
            keyboard
                .key_rect(row_start(Page::Letters, 4), OFFSET_RECT)
                .map(|key| key.y),
            Some(620.0),
            "the bottom row is 240 down the keyboard, wherever the keyboard is"
        );
    }

    // -------------------------------------------------- the touch floor

    #[test]
    fn every_key_of_the_default_size_is_well_over_the_touch_floor() {
        let (_nodes, keyboard) = keyboard();
        for index in 0..keyboard.key_count() {
            let key = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            assert!(
                key.width >= MIN_TOUCH_TARGET && key.height >= MIN_TOUCH_TARGET,
                "key {index} is {key:?}, under the {MIN_TOUCH_TARGET} floor"
            );
        }
    }

    #[test]
    fn the_narrowest_key_in_the_keyboard_is_a_ten_key_rows_letter_and_is_far_over_the_floor() {
        // The narrowest key is a letter on a ten-key row at 65, and the smallest
        // gap-weighted one is a letter on the bottom row's neighbours. Both are
        // named here so a change to the floor cannot quietly become "still fine".
        let (_nodes, keyboard) = keyboard();
        let narrowest = (0..keyboard.key_count())
            .filter_map(|index| keyboard.key_rect(index, RECT))
            .map(|key| key.width)
            .fold(f32::INFINITY, f32::min);
        assert_eq!(narrowest, 65.0, "a ten-key row's letter");
        assert!(
            narrowest > MIN_TOUCH_TARGET,
            "{narrowest} is above the floor"
        );
    }

    #[test]
    fn a_rect_too_narrow_for_the_floor_overflows_rather_than_shrinking_a_key() {
        // A 200-wide keyboard cannot fit ten keys of 44 in it. The keys keep the
        // floor and run past the right-hand edge, because a 20-pixel key is a
        // target no finger can be asked to hit.
        let (_nodes, keyboard) = keyboard();
        let narrow = Rect::new(0.0, 0.0, 200.0, 300.0);
        for index in 0..10 {
            let key = keyboard.key_rect(index, narrow).unwrap_or(narrow);
            assert_eq!(
                key.width, MIN_TOUCH_TARGET,
                "key {index} is the floor, not 15 pixels"
            );
        }
        let last = keyboard.key_rect(9, narrow).unwrap_or(narrow);
        assert!(
            last.x + last.width > narrow.width,
            "and the row overflows the rect rather than fitting inside it"
        );
    }

    #[test]
    fn a_short_node_does_not_make_a_short_key() {
        // The floor is on the key, not on the rect: a caller that laid the keyboard
        // out in twenty pixels of height gets keys that overflow the box, not keys
        // a finger cannot hit.
        let (_nodes, keyboard) = keyboard();
        let short = Rect::new(0.0, 0.0, 720.0, 20.0);
        let key = keyboard.key_rect(0, short).unwrap_or(short);
        assert_eq!(key.height, KEY_HEIGHT, "which is over the floor anyway");
        assert!(key.height >= MIN_TOUCH_TARGET);
    }

    #[test]
    fn a_key_shorter_than_the_floor_is_floored_at_it() {
        // The height floor is only reachable through `key_height`, which is why it
        // is a property and not a constant: with the constant alone this assertion
        // would hold for every value the code could take, and would have passed a
        // mutation that deleted the floor.
        let (_nodes, keyboard) = keyboard();
        keyboard.key_height.set(10.0);
        for index in 0..keyboard.key_count() {
            let key = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            assert_eq!(
                key.height, MIN_TOUCH_TARGET,
                "key {index} is the floor, not the ten pixels that was asked for"
            );
        }
        assert!(
            press(&keyboard, RECT, keyboard.key_rect(0, RECT).unwrap_or(RECT)),
            "and it is still pressable at that size"
        );
    }

    #[test]
    fn a_key_taller_than_the_floor_is_the_height_that_was_asked_for() {
        // The other side of the same floor: it is a floor and not a fixed height,
        // so a denser head unit that asks for 60 gets 60.
        let (_nodes, keyboard) = keyboard();
        keyboard.key_height.set(60.0);
        let key = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        assert_eq!(key.height, 60.0);
        assert_eq!(
            keyboard.size().height,
            KEY_PADDING * 2.0 + grid_height(60.0),
            "and the keyboard asks for a box its own keys fit in"
        );
        assert_eq!(
            keyboard
                .key_rect(row_start(Page::Letters, 4), RECT)
                .map(|k| k.y),
            Some(KEY_PADDING + 4.0 * (60.0 + KEY_GAP)),
            "with the rows below it pushed down to match"
        );
    }

    #[test]
    fn an_overflowing_row_leaves_its_keys_still_findable() {
        // The floor's other half: a key that overflows is only useful if it can be
        // *pressed*, so the hit test reads the same rect the paint pass drew.
        let (_nodes, keyboard) = keyboard();
        let narrow = Rect::new(0.0, 0.0, 200.0, 300.0);
        for index in 0..10 {
            let drawn = keyboard.key_rect(index, narrow).unwrap_or(narrow);
            assert_eq!(
                keyboard.key_at(centre(drawn), narrow),
                Some(KeyId(index)),
                "key {index} is drawn at {drawn:?} and found there"
            );
        }
    }

    // -------------------------------------------------- drawing and hit testing read one number

    #[test]
    fn hit_testing_and_drawing_read_the_same_number() {
        // The scrollbar's rule: one geometry, one owner. Every key's painted rect
        // is `key_rect`'s, and pressing the middle of that painted rect finds that
        // same key — asserted both ways, from the pure function and from the
        // commands, because a change that reached one and missed the other is the
        // defect this is here for.
        let (_nodes, keyboard) = keyboard();
        let commands = keyboard.paint(RECT);
        let drawn: Vec<Rect> = rounded(&commands)
            .into_iter()
            .map(|(rect, _, _)| rect)
            .collect();
        // One of those is the panel and the rest are the keys, in reading order.
        assert_eq!(drawn.len(), keyboard.key_count() + 1);
        for index in 0..keyboard.key_count() {
            let expected = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            assert_eq!(
                drawn[index + 1],
                expected,
                "key {index} is painted where key_rect says it is"
            );
            assert_eq!(
                keyboard.key_at(centre(drawn[index + 1]), RECT),
                Some(KeyId(index)),
                "and the press that finds it names key {index}"
            );
        }
    }

    #[test]
    fn a_key_away_from_the_origin_is_painted_where_it_is_found() {
        let (_nodes, keyboard) = keyboard();
        let drawn = rounded(&keyboard.paint(OFFSET_RECT));
        let first = drawn.get(1).map(|(rect, _, _)| *rect);
        assert_eq!(first, Some(Rect::new(288.0, 388.0, 65.0, 52.0)));
        assert_eq!(
            keyboard.key_at(Offset::new(300.0, 400.0), OFFSET_RECT),
            Some(KeyId(0)),
            "a press inside it is that key, and the window is not at the origin"
        );
    }

    #[test]
    fn a_tap_where_every_key_is_drawn_reports_that_key() {
        // Every drawn key, and every one of them named by its own rect: no fixture
        // anywhere near a control.
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let drawn: Vec<Rect> = rounded(&keyboard.paint(RECT))
            .into_iter()
            .skip(1)
            .map(|(rect, _, _)| rect)
            .collect();
        assert_eq!(drawn.len(), keyboard.key_count());
        for (index, rect) in drawn.into_iter().enumerate() {
            // What the key means is read at the moment it is pressed, not before
            // the loop and not after it: the shift key is among the keys being
            // pressed, so the letter after it is upper case, and an expectation
            // computed once would be wrong for the second half of the shift row.
            let expected = keyboard.key_action(KeyId(index));
            let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(rect)));
            assert!(
                keyboard.on_event(&mut tap, RECT),
                "key {index} took the tap"
            );
            assert!(tap.consumed(), "key {index} consumed it");
            assert_eq!(
                *seen.borrow().last().unwrap_or(&KeyAction::Space),
                expected.unwrap_or(KeyAction::Space),
                "and key {index} reported what it means"
            );
        }
        assert_eq!(
            seen.borrow().len(),
            keyboard.key_count(),
            "one action a key"
        );
    }

    #[test]
    fn a_press_where_every_key_is_drawn_grabs_that_key() {
        let (_nodes, keyboard) = keyboard();
        for index in 0..keyboard.key_count() {
            let drawn = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            assert!(press(&keyboard, RECT, drawn), "key {index} is pressable");
            assert_eq!(
                keyboard.grabbed_key(),
                Some(KeyId(index)),
                "and the press landed on key {index}"
            );
            assert!(keyboard.release_key());
        }
        assert!(!keyboard.is_key_grabbed());
    }

    // -------------------------------------------------- hit testing

    #[test]
    fn a_point_in_the_gap_between_two_keys_is_no_keys() {
        // The gap is the whole reason `key_at` can answer `None`: it is the dead
        // space that lets a tap that missed everything reach the panel behind.
        let (_nodes, keyboard) = keyboard();
        let first = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        let gap_centre = first.x + first.width + KEY_GAP / 2.0;
        assert_eq!(
            keyboard.key_at(Offset::new(gap_centre, first.y + 1.0), RECT),
            None
        );
    }

    #[test]
    fn a_point_in_the_gap_between_two_rows_is_no_keys() {
        let (_nodes, keyboard) = keyboard();
        let first = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        let between = first.y + first.height + KEY_GAP / 2.0;
        assert_eq!(
            keyboard.key_at(Offset::new(first.x + 1.0, between), RECT),
            None,
            "the horizontal gap is real as well as the vertical one"
        );
    }

    #[test]
    fn a_point_outside_the_keyboard_is_no_keys() {
        let (_nodes, keyboard) = keyboard();
        for point in [
            Offset::new(-1.0, 20.0),
            Offset::new(20.0, -1.0),
            Offset::new(800.0, 20.0),
            Offset::new(20.0, 400.0),
        ] {
            assert_eq!(keyboard.key_at(point, RECT), None, "{point:?} is outside");
        }
    }

    #[test]
    fn a_keys_edges_belong_to_the_key() {
        let (_nodes, keyboard) = keyboard();
        let first = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        for point in [
            Offset::new(first.x, first.y),
            Offset::new(first.x + first.width, first.y + first.height),
        ] {
            assert_eq!(keyboard.key_at(point, RECT), Some(KeyId(0)), "{point:?}");
        }
    }

    #[test]
    fn a_tap_in_a_gap_is_left_for_the_panel_behind() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let first = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        let mut tap = tap_at(first.x + first.width + 1.0, first.y + 1.0);
        assert!(!keyboard.on_event(&mut tap, RECT), "nothing consumed it");
        assert!(!tap.consumed(), "so it travels on");
        assert!(seen.borrow().is_empty(), "and nothing was reported");
    }

    #[test]
    fn a_tap_outside_the_keyboard_is_left_for_the_panel_behind() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let mut tap = tap_at(1500.0, 700.0);
        assert!(!keyboard.on_event(&mut tap, RECT));
        assert!(!tap.consumed());
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn a_tap_with_no_position_names_no_key() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let mut tap = InputEvent::new(InputEventKind::Tap, None);
        assert!(!keyboard.on_event(&mut tap, RECT));
        assert!(!tap.consumed());
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn a_tap_outside_the_keyboard_reaches_the_panel_and_not_the_keyboard() {
        let (mut nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let (panel, placed) = on_a_panel(&keyboard, &mut nodes, Size::new(900.0, 700.0));

        let mut event = tap_at(860.0, 660.0);
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == keyboard.handle() {
                keyboard.on_event(event, placed);
            }
        });
        assert_eq!(reached, vec![panel], "the panel took it");
        assert!(
            seen.borrow().is_empty(),
            "and the keyboard reported nothing"
        );
    }

    #[test]
    fn a_tap_reached_through_the_dispatcher_presses_the_key_under_it() {
        let (mut nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let (panel, placed) = on_a_panel(&keyboard, &mut nodes, Size::new(900.0, 700.0));
        let drawn = keyboard.key_rect(10, placed).unwrap_or(placed);

        let mut event = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        let mut reached = Vec::new();
        crate::input::dispatch_event(&nodes, panel, &mut event, &mut |handle, event| {
            reached.push(handle);
            if handle == keyboard.handle() {
                keyboard.on_event(event, placed);
            }
        });
        assert_eq!(reached, vec![keyboard.handle()], "only the keyboard saw it");
        assert_eq!(
            seen.borrow().as_slice(),
            &[KeyAction::Char('q')],
            "and it reported the `q` that is drawn there"
        );
    }

    // -------------------------------------------------- the press half

    #[test]
    fn a_press_lights_the_key_under_the_pointer_and_nothing_else() {
        let (_nodes, keyboard) = keyboard();
        let target = keyboard.key_rect(23, RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, target), "the press hit a key");
        assert_eq!(keyboard.grabbed_key(), Some(KeyId(23)));

        let lit = painted_in(&keyboard.paint(RECT), target);
        assert_eq!(
            lit.map(|(_, _, color)| color),
            Some(keyboard.key_active.get()),
            "the pressed key is drawn in the active colour"
        );

        let other = keyboard.key_rect(24, RECT).unwrap_or(RECT);
        let unlit = painted_in(&keyboard.paint(RECT), other);
        assert_eq!(
            unlit.map(|(_, _, color)| color),
            Some(keyboard.key.get()),
            "and its neighbour is not"
        );
    }

    #[test]
    fn a_release_unlights_the_key() {
        let (_nodes, keyboard) = keyboard();
        let target = keyboard.key_rect(5, RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, target));
        assert!(keyboard.is_key_grabbed());

        assert!(keyboard.release_key(), "there was a press to give up");
        assert!(!keyboard.is_key_grabbed());
        assert_eq!(keyboard.grabbed_key(), None);
        assert_eq!(
            painted_in(&keyboard.paint(RECT), target).map(|(_, _, color)| color),
            Some(keyboard.key.get()),
            "and the key is back to its resting colour"
        );
    }

    #[test]
    fn a_release_with_no_press_gives_up_nothing() {
        let (_nodes, keyboard) = keyboard();
        assert!(!keyboard.release_key());
        assert!(!keyboard.is_key_grabbed());
    }

    #[test]
    fn a_press_that_misses_every_key_grabs_nothing() {
        let (_nodes, keyboard) = keyboard();
        let first = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, first), "a key first");
        assert!(!keyboard.grab_key(Offset::new(first.x - 2.0, first.y + 1.0), RECT));
        assert!(
            !keyboard.is_key_grabbed(),
            "and the miss took the earlier press with it, because it is the pointer's state"
        );
    }

    #[test]
    fn a_tap_does_not_release_the_press() {
        // The release is the caller's, from its own pointer-up. By the time the
        // recogniser's tap arrives the finger has already gone, so a tap that
        // released the grab would unlight the key before the user saw it lit.
        let (_nodes, keyboard) = keyboard();
        let drawn = keyboard.key_rect(3, RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, drawn));
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT));
        assert_eq!(
            keyboard.grabbed_key(),
            Some(KeyId(3)),
            "the key is still the caller's to release"
        );
    }

    #[test]
    fn the_pressed_key_lights_before_the_tap_that_reports_it() {
        // The order the recogniser imposes, asserted end to end: press, the key is
        // lit, then the tap arrives and reports it, and only then does the caller
        // release. A keyboard that only lit on the tap would never show a pressed
        // key at all.
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let drawn = keyboard.key_rect(11, RECT).unwrap_or(RECT);

        assert!(press(&keyboard, RECT, drawn), "the press");
        assert!(seen.borrow().is_empty(), "reports nothing yet");
        assert_eq!(
            painted_in(&keyboard.paint(RECT), drawn).map(|(_, _, c)| c),
            Some(keyboard.key_active.get()),
            "but the key is lit while the finger is on it"
        );

        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT), "then the tap");
        assert_eq!(seen.borrow().as_slice(), &[KeyAction::Char('w')]);
    }

    // -------------------------------------------------- what a key reports

    #[test]
    fn a_letter_key_reports_that_letter() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let drawn = keyboard.key_rect(12, RECT).unwrap_or(RECT);
        assert_eq!(keyboard.key_label(KeyId(12)), Some("e".to_string()));
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT));
        assert_eq!(seen.borrow().as_slice(), &[KeyAction::Char('e')]);
    }

    #[test]
    fn the_digits_report_the_digits_and_not_a_numerical_key() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        for (index, digit) in DIGITS.chars().enumerate() {
            let drawn = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
            assert!(keyboard.on_event(&mut tap, RECT), "digit {digit}");
        }
        assert_eq!(
            seen.borrow().as_slice(),
            &[
                KeyAction::Char('1'),
                KeyAction::Char('2'),
                KeyAction::Char('3'),
                KeyAction::Char('4'),
                KeyAction::Char('5'),
                KeyAction::Char('6'),
                KeyAction::Char('7'),
                KeyAction::Char('8'),
                KeyAction::Char('9'),
                KeyAction::Char('0'),
            ]
        );
    }

    #[test]
    fn backspace_enter_and_space_are_their_own_keys_and_not_characters() {
        // Requirement 4's three named keys. Each is pressed *where it is drawn*, and
        // each reports an action that is not a character — a caller cannot tell
        // `Bksp` from the letters by looking at a string.
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        for action in [KeyAction::Backspace, KeyAction::Enter, KeyAction::Space] {
            let id = key_meaning(&keyboard, action);
            assert_ne!(id.get(), usize::MAX, "{action:?} is on the page");
            let drawn = keyboard.key_rect(id.get(), RECT).unwrap_or(RECT);
            assert!(press(&keyboard, RECT, drawn), "{action:?} is pressable");
            assert_eq!(keyboard.grabbed_key(), Some(id));
            keyboard.release_key();

            let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
            assert!(keyboard.on_event(&mut tap, RECT), "{action:?} took the tap");
        }
        assert_eq!(
            seen.borrow().as_slice(),
            &[KeyAction::Backspace, KeyAction::Enter, KeyAction::Space]
        );
        assert!(
            !seen
                .borrow()
                .iter()
                .any(|action| matches!(action, KeyAction::Char(_))),
            "and none of them arrived as a character"
        );
    }

    #[test]
    fn backspace_enter_and_space_print_the_names_a_driver_reads() {
        let (_nodes, keyboard) = keyboard();
        for (action, want) in [
            (KeyAction::Backspace, "Bksp"),
            (KeyAction::Enter, "Enter"),
            (KeyAction::Space, "Space"),
        ] {
            let id = key_meaning(&keyboard, action);
            assert_eq!(
                keyboard.key_label(id),
                Some(want.to_string()),
                "{action:?} is printed as {want}"
            );
        }
    }

    #[test]
    fn the_shift_key_reports_shift_and_arms_the_next_character() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let shift = key_meaning(&keyboard, KeyAction::Shift);
        let drawn = keyboard.key_rect(shift.get(), RECT).unwrap_or(RECT);

        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT), "the shift is pressable");
        assert_eq!(seen.borrow().as_slice(), &[KeyAction::Shift]);
        assert!(keyboard.is_shifted(), "and it armed the next character");
    }

    #[test]
    fn a_shifted_character_is_reported_and_drawn_upper_case() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        keyboard.shifted.set(true);
        let drawn = keyboard.key_rect(11, RECT).unwrap_or(RECT);
        assert_eq!(keyboard.key_label(KeyId(11)), Some("W".to_string()));

        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT));
        assert_eq!(seen.borrow().as_slice(), &[KeyAction::Char('W')]);
    }

    #[test]
    fn a_shift_is_spent_by_the_character_it_was_armed_for() {
        // `A` then `a` is what a user means after one tap on the shift; a sticky
        // shift that never disarms turns a whole word into capitals.
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        keyboard.shifted.set(true);
        let first = keyboard.key_rect(11, RECT).unwrap_or(RECT);
        let second = keyboard.key_rect(12, RECT).unwrap_or(RECT);
        for drawn in [first, second] {
            let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
            assert!(keyboard.on_event(&mut tap, RECT));
        }
        assert_eq!(
            seen.borrow().as_slice(),
            &[KeyAction::Char('W'), KeyAction::Char('e')],
            "the shift paid for one letter and no more"
        );
        assert!(!keyboard.is_shifted());
    }

    #[test]
    fn a_shift_is_not_spent_by_a_key_that_is_not_a_character() {
        let (_nodes, keyboard) = keyboard();
        keyboard.shifted.set(true);
        let enter = key_meaning(&keyboard, KeyAction::Enter);
        let drawn = keyboard.key_rect(enter.get(), RECT).unwrap_or(RECT);
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT));
        assert!(
            keyboard.is_shifted(),
            "an enter is not a letter, so the shift is still armed"
        );
        assert!(
            keyboard.grabbed_key().is_none(),
            "and a tap does not grab: the press is the caller's"
        );
    }

    #[test]
    fn a_shift_twice_arms_the_upper_case_and_there_and_back() {
        let (_nodes, keyboard) = keyboard();
        let shift = key_meaning(&keyboard, KeyAction::Shift);
        let drawn = keyboard.key_rect(shift.get(), RECT).unwrap_or(RECT);
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        keyboard.on_event(&mut tap, RECT);
        assert!(keyboard.is_shifted());
        let mut again = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        keyboard.on_event(&mut again, RECT);
        assert!(
            !keyboard.is_shifted(),
            "a second tap disarms it rather than locking it"
        );
    }

    #[test]
    fn one_tap_fires_one_action() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let drawn = keyboard.key_rect(10, RECT).unwrap_or(RECT);
        for _ in 0..3 {
            let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
            keyboard.on_event(&mut tap, RECT);
        }
        assert_eq!(seen.borrow().as_slice(), &[KeyAction::Char('q'); 3]);
    }

    #[test]
    fn a_keyboard_with_no_handler_still_consumes_the_tap_it_answers() {
        // A caller that has given it nothing to report to is an ordinary caller.
        let (_nodes, keyboard) = keyboard();
        assert!(!keyboard.on_key.is_set());
        let drawn = keyboard.key_rect(1, RECT).unwrap_or(RECT);
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT));
        assert!(tap.consumed());
    }

    #[test]
    fn a_drag_and_a_long_press_are_left_for_somebody_else() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let drawn = keyboard.key_rect(12, RECT).unwrap_or(RECT);
        let mut drag = InputEvent::new(
            InputEventKind::Drag {
                delta: Offset::new(4.0, 0.0),
            },
            Some(centre(drawn)),
        );
        assert!(!keyboard.on_event(&mut drag, RECT));
        assert!(!drag.consumed());

        let mut long = InputEvent::new(InputEventKind::LongPress, Some(centre(drawn)));
        assert!(!keyboard.on_event(&mut long, RECT));
        assert!(!long.consumed());
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn a_hardware_typings_text_event_belongs_to_the_text_widget() {
        // `InputEventKind::Text` is the layout-correct string a hardware keyboard
        // produced. It is not this widget's, and consuming it would put the
        // character nowhere at all.
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let mut typed = InputEvent::new(
            InputEventKind::Text {
                text: "road".to_string(),
            },
            None,
        );
        assert!(!keyboard.on_event(&mut typed, RECT));
        assert!(!typed.consumed());
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn a_key_press_is_left_for_the_focused_text_widget() {
        let (_nodes, keyboard) = keyboard();
        let mut press = InputEvent::new(
            InputEventKind::KeyDown {
                key: crate::input::Key::Keyboard(sdl3::keyboard::Keycode::A),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        );
        assert!(!keyboard.on_event(&mut press, RECT));
        assert!(!press.consumed());
    }

    // -------------------------------------------------- the pages

    #[test]
    fn both_pages_have_five_rows() {
        for page in [Page::Letters, Page::Symbols] {
            assert_eq!(rows_for(page).len(), ROWS, "{page:?} is five rows tall");
        }
    }

    #[test]
    fn the_two_pages_have_different_numbers_of_keys() {
        // The home row has nine keys and the punctuation rows ten, so an index is
        // page-local — which is what `KeyId` is for.
        let (_nodes, keyboard) = keyboard();
        assert_eq!(keyboard.key_count(), 41, "10 + 10 + 9 + 9 + 3");
        keyboard.set_page(Page::Symbols);
        assert_eq!(keyboard.key_count(), 43, "and 10 + 10 + 10 + 10 + 3");
    }

    #[test]
    fn the_numbers_row_is_in_the_same_place_on_both_pages() {
        let (_nodes, keyboard) = keyboard();
        let letters: Vec<Rect> = (0..10)
            .filter_map(|index| keyboard.key_rect(index, RECT))
            .collect();
        keyboard.set_page(Page::Symbols);
        let symbols: Vec<Rect> = (0..10)
            .filter_map(|index| keyboard.key_rect(index, RECT))
            .collect();
        assert_eq!(letters, symbols, "the numbers row does not move");
        assert!(letters.len() == 10, "and it is ten keys wide");
    }

    #[test]
    fn the_digits_report_the_digits_on_the_symbols_page_too() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        assert!(keyboard.set_page(Page::Symbols));
        for index in 0..10 {
            let drawn = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
            assert!(keyboard.on_event(&mut tap, RECT), "digit {index}");
        }
        let reported = seen.borrow();
        assert_eq!(reported.len(), 10);
        for (index, action) in reported.iter().enumerate() {
            let digit = DIGITS.chars().nth(index);
            assert_eq!(*action, KeyAction::Char(digit.unwrap_or(' ')));
        }
    }

    #[test]
    fn the_bottom_row_keeps_its_shape_on_both_pages() {
        let (_nodes, keyboard) = keyboard();
        let base = row_start(Page::Letters, 4);
        let letters: Vec<Rect> = (base..)
            .take(3)
            .filter_map(|index| keyboard.key_rect(index, RECT))
            .collect();
        keyboard.set_page(Page::Symbols);
        let base = row_start(Page::Symbols, 4);
        let symbols: Vec<Rect> = (base..)
            .take(3)
            .filter_map(|index| keyboard.key_rect(index, RECT))
            .collect();
        assert_eq!(letters, symbols, "the space bar does not move either");
    }

    #[test]
    fn the_thumb_row_holds_the_three_keys_a_driver_presses_most() {
        // The layout's *reason*: the bottom two rows are where a hand reaches, and
        // the bottom row carries space, enter and the page switch. Asserted from the
        // tables rather than from the prose, so a re-weighted table fails here.
        let letters = rows_for(Page::Letters);
        let bottom: Vec<CapAction> = letters[4].iter().map(|cap| cap.action).collect();
        assert_eq!(
            bottom,
            vec![CapAction::Page, CapAction::Space, CapAction::Enter],
            "the bottom row is the switch, the space bar and enter"
        );
        let backspace_row = letters[3]
            .iter()
            .find(|cap| cap.action == CapAction::Backspace)
            .copied();
        assert!(
            backspace_row.is_some(),
            "and the backspace is the row above it, so all four are in the bottom band"
        );
        assert_eq!(
            letters[3].len(),
            9,
            "so the backspace shares its row with seven letters"
        );
    }

    #[test]
    fn the_page_key_says_which_page_it_switches_to() {
        let (_nodes, keyboard) = keyboard();
        let id = key_meaning(&keyboard, KeyAction::PageUp);
        assert_eq!(keyboard.key_label(id), Some("?123".to_string()));
        assert_eq!(Page::Symbols.switch_label(), "ABC", "and the other way");
    }

    #[test]
    fn tapping_the_page_key_switches_the_page_and_reports_the_move() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        let id = key_meaning(&keyboard, KeyAction::PageUp);
        let drawn = keyboard.key_rect(id.get(), RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, drawn), "the switch is pressable");

        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        assert!(keyboard.on_event(&mut tap, RECT), "and it took the tap");
        assert_eq!(seen.borrow().as_slice(), &[KeyAction::PageUp]);
    }

    #[test]
    fn the_key_the_page_key_points_at_moves_to_the_page_the_key_says() {
        // The switch is reported as an action and the caller drives the page; this
        // is the wiring a caller does with it, asserted here because the two have
        // to agree or the key says "ABC" and takes you to the letters.
        let (_nodes, keyboard) = keyboard();
        let id = key_meaning(&keyboard, KeyAction::PageUp);
        let drawn = keyboard.key_rect(id.get(), RECT).unwrap_or(RECT);
        let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
        keyboard.on_event(&mut tap, RECT);

        assert!(keyboard.set_page(Page::Symbols), "the caller switches");
        let id = key_meaning(&keyboard, KeyAction::PageDown);
        assert_eq!(
            keyboard.key_label(id),
            Some("ABC".to_string()),
            "and the switch now names the way back"
        );
    }

    #[test]
    fn switching_pages_moves_the_page_key_and_leaves_the_digits_where_they_were() {
        let (_nodes, keyboard) = keyboard();
        let digit = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        assert!(key_meaning(&keyboard, KeyAction::PageUp).get() > 0);
        assert!(keyboard.set_page(Page::Symbols));
        let after = key_meaning(&keyboard, KeyAction::PageDown);
        assert_eq!(
            after.get(),
            row_start(Page::Symbols, 4),
            "the switch is the first key of the bottom row on this page too"
        );
        assert_eq!(
            keyboard
                .key_rect(after.get(), RECT)
                .map(|key| (key.x, key.y)),
            Some((RECT.x + KEY_PADDING, RECT.y + KEY_PADDING + 232.0)),
            "so it sits at the bottom left, whichever page is showing"
        );
        assert_eq!(
            keyboard.key_rect(0, RECT),
            Some(digit),
            "and the numbers row keeps its place"
        );
    }

    #[test]
    fn the_symbols_page_reports_punctuation_and_no_letters() {
        let (_nodes, mut keyboard) = keyboard();
        let seen = actions(&mut keyboard);
        keyboard.set_page(Page::Symbols);
        let base = row_start(Page::Symbols, 1);
        for (index, want) in ['!', '"', '#'].into_iter().enumerate() {
            let drawn = keyboard.key_rect(base + index, RECT).unwrap_or(RECT);
            let mut tap = InputEvent::new(InputEventKind::Tap, Some(centre(drawn)));
            assert!(keyboard.on_event(&mut tap, RECT), "{want} is pressable");
        }
        assert_eq!(
            seen.borrow().as_slice(),
            &[
                KeyAction::Char('!'),
                KeyAction::Char('"'),
                KeyAction::Char('#')
            ]
        );
    }

    #[test]
    fn switching_pages_clears_a_grab_because_an_index_means_a_different_key() {
        let (_nodes, keyboard) = keyboard();
        let drawn = keyboard.key_rect(0, RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, drawn));
        assert!(keyboard.is_key_grabbed());

        assert!(keyboard.set_page(Page::Symbols));
        assert!(
            !keyboard.is_key_grabbed(),
            "a grab made on the letters page names the `1` key on both, but a grab on \
             row three would name a bracket"
        );
    }

    #[test]
    fn a_grab_made_on_the_home_row_would_be_a_different_key_after_a_switch() {
        // Why the previous test is not paranoid: the home row's first key is `a` on
        // the letters page and `@` on the symbols page, at the same index.
        let (_nodes, keyboard) = keyboard();
        let home = row_start(Page::Letters, 2);
        let drawn = keyboard.key_rect(home, RECT).unwrap_or(RECT);
        assert_eq!(keyboard.key_action(KeyId(home)), Some(KeyAction::Char('a')));
        assert!(press(&keyboard, RECT, drawn));
        let grabbed = keyboard.grabbed_key();

        keyboard.set_page(Page::Symbols);
        assert_eq!(
            keyboard.key_action(grabbed.unwrap_or(KeyId(0))),
            Some(KeyAction::Char('@'))
        );
        assert_eq!(
            keyboard.grabbed_key(),
            None,
            "so the grab was dropped, not reused"
        );
    }

    #[test]
    fn switching_pages_clears_an_armed_shift() {
        // A shift arms the *next character*, and on the punctuation page the next
        // character is `@`, which has no case.
        let (_nodes, keyboard) = keyboard();
        keyboard.shifted.set(true);
        assert!(keyboard.set_page(Page::Symbols));
        assert!(!keyboard.is_shifted());
        let home = row_start(Page::Symbols, 2);
        assert_eq!(keyboard.key_action(KeyId(home)), Some(KeyAction::Char('@')));
    }

    #[test]
    fn showing_the_page_that_is_already_showing_is_not_a_change() {
        let (_nodes, keyboard) = keyboard();
        assert!(!keyboard.set_page(Page::Letters));
        assert!(keyboard.set_page(Page::Symbols));
        assert!(
            !keyboard.set_page(Page::Symbols),
            "the second time is not one"
        );
        assert!(keyboard.set_page(Page::Letters), "and toggling back is");
    }

    #[test]
    fn the_page_never_leaves_the_two_it_has() {
        // A caller holding a `Page` can only reach one of two: there is no third
        // state to fall into, and no arithmetic that could produce one.
        let mut page = Page::Letters;
        let mut seen = vec![page];
        for _ in 0..5 {
            page = page.toggled();
            seen.push(page);
        }
        assert_eq!(
            seen,
            vec![
                Page::Letters,
                Page::Symbols,
                Page::Letters,
                Page::Symbols,
                Page::Letters,
                Page::Symbols
            ]
        );
    }

    // -------------------------------------------------- painting

    #[test]
    fn a_keyboard_paints_its_panel_and_then_one_shape_and_one_label_per_key() {
        let (_nodes, keyboard) = keyboard();
        let commands = keyboard.paint(RECT);
        assert_eq!(
            commands.len(),
            keyboard.key_count() * 2 + 1,
            "a panel, and a shape and a label per key"
        );
        assert_eq!(
            shapes(&commands).first(),
            Some(&"key"),
            "and the panel is the first thing drawn"
        );
    }

    #[test]
    fn a_keyboard_paints_nothing_that_is_not_a_panel_a_key_or_a_label() {
        // A glyph, an image or a line would mean this widget had grown a feature
        // nobody asked for; and the *absence* of a label is what "no draw-command
        // assertion could see" looks like, so the presence of one per key is
        // asserted above and the absence of anything else here.
        let (_nodes, keyboard) = keyboard();
        for command in keyboard.paint(RECT) {
            assert!(
                matches!(
                    command,
                    DrawCommand::RoundedRect { .. } | DrawCommand::Text { .. }
                ),
                "a keyboard draws keys and their labels and nothing else: {command:?}"
            );
        }
    }

    #[test]
    fn each_keys_label_is_drawn_inside_that_key() {
        // Where a command *lands*, not only that it was recorded: a label drawn a
        // row above its own key is a recorded command and a wrong picture.
        let (_nodes, keyboard) = keyboard();
        let painted = labels(&keyboard.paint(RECT));
        assert_eq!(painted.len(), keyboard.key_count());
        for (index, (x, y, text, _)) in painted.into_iter().enumerate() {
            let key = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            assert!(
                x >= key.x && x <= key.x + key.width,
                "the label {text:?} at {x} is inside the width of key {index}"
            );
            assert!(
                y >= key.y && y <= key.y + key.height,
                "and its line box at {y} is inside the height of key {index}"
            );
        }
    }

    #[test]
    fn each_keys_label_is_centred_in_that_key() {
        let (_nodes, keyboard) = keyboard();
        let painted = labels(&keyboard.paint(RECT));
        for (index, (x, _, text, _)) in painted.into_iter().enumerate() {
            let key = keyboard.key_rect(index, RECT).unwrap_or(RECT);
            let width = keyboard.advance.get() * count_to_f32(text.chars().count());
            assert_close(
                x,
                key.x + (key.width - width) / 2.0,
                &format!("the label {text:?} on key {index}"),
            );
        }
    }

    #[test]
    fn a_keys_label_is_the_character_it_inserts() {
        let (_nodes, keyboard) = keyboard();
        let painted = labels(&keyboard.paint(RECT));
        let drawn: Vec<String> = painted.into_iter().map(|(_, _, text, _)| text).collect();
        assert_eq!(
            &drawn[..10],
            &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]
        );
        assert_eq!(
            &drawn[10..20],
            &["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"]
        );
        assert_eq!(
            &drawn[20..29],
            &["a", "s", "d", "f", "g", "h", "j", "k", "l"]
        );
        assert_eq!(
            &drawn[29], "Shift",
            "and the bottom-left key is the page switch"
        );
    }

    #[test]
    fn the_panel_is_drawn_in_the_theme_surface_colour() {
        let (_nodes, keyboard) = keyboard();
        let painted = rounded(&keyboard.paint(RECT));
        assert_eq!(painted[0].0, RECT, "the panel is the keyboard's own rect");
        assert_eq!(painted[0].2, keyboard.palette().panel);
        assert_eq!(
            painted[0].2,
            Palette::from_theme(&Theme::dark()).panel,
            "which is the theme's Surface"
        );
    }

    #[test]
    fn a_key_is_a_rounded_rectangle_at_its_own_rect() {
        let (_nodes, keyboard) = keyboard();
        let painted = rounded(&keyboard.paint(RECT));
        let switch = key_meaning(&keyboard, KeyAction::PageUp).get();
        for index in 0..keyboard.key_count() {
            let drawn = painted[index + 1];
            assert_eq!(drawn.0, keyboard.key_rect(index, RECT).unwrap_or(RECT));
            assert_eq!(drawn.1, KEY_RADIUS, "with the shared corner radius");
            let want = if index == switch {
                keyboard.key_active.get()
            } else {
                keyboard.key.get()
            };
            assert_eq!(drawn.2, want, "key {index} is in the colour of its state");
        }
    }

    #[test]
    fn the_page_switch_key_is_drawn_in_the_active_colours_at_rest() {
        // The layer has to be discoverable: a control nobody can find is a page
        // nobody reaches, and there is nothing else on a keyboard carrying the
        // state.
        let (_nodes, keyboard) = keyboard();
        let id = key_meaning(&keyboard, KeyAction::PageUp);
        let drawn = painted_in(
            &keyboard.paint(RECT),
            keyboard.key_rect(id.get(), RECT).unwrap(),
        );
        assert_eq!(drawn.map(|(_, _, c)| c), Some(keyboard.key_active.get()));
        let ink = labels(&keyboard.paint(RECT))
            .into_iter()
            .filter(|(_, _, text, _)| text == "?123")
            .map(|(_, _, _, color)| color)
            .next();
        assert_eq!(
            ink,
            Some(keyboard.label_active.get()),
            "and so is its label"
        );
    }

    #[test]
    fn a_focused_keyboard_draws_a_ring_around_its_panel_and_the_panel_covers_it() {
        // A `RoundedRect` *fills* its rect, so a ring drawn around the keyboard and
        // left on top is a filled panel with a border — not a focus indicator. The
        // pair is the assertion: the grown shape, and the panel recorded after it.
        let (_nodes, keyboard) = keyboard();
        assert_eq!(
            rounded(&keyboard.paint(RECT)).len(),
            keyboard.key_count() + 1,
            "no ring while unfocused"
        );

        keyboard.focused.set(true);
        let painted = rounded(&keyboard.paint(RECT));
        assert_eq!(painted.len(), keyboard.key_count() + 2, "a ring as well");
        assert_eq!(
            painted[0].0,
            Rect::new(-2.0, -2.0, 724.0, 304.0),
            "the ring is the panel grown by its own width, so its border shows"
        );
        assert_eq!(painted[0].2, keyboard.palette().ring);
        assert_eq!(
            painted[1].0, RECT,
            "and the panel covers the middle of it, which is what leaves the ring"
        );
        assert_eq!(
            shapes(&keyboard.paint(RECT)).first(),
            Some(&"key"),
            "recorded ring first, then the panel over it"
        );
    }

    #[test]
    fn a_zero_width_focus_ring_is_not_painted() {
        let (_nodes, keyboard) = keyboard();
        keyboard.focused.set(true);
        keyboard.focus_ring.set(0.0);
        assert_eq!(
            rounded(&keyboard.paint(RECT)).len(),
            keyboard.key_count() + 1
        );
    }

    // -------------------------------------------------- the theme

    #[test]
    fn the_palette_is_the_six_tokens_this_module_names() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            let color = |token| {
                theme
                    .get(token)
                    .as_color()
                    .unwrap_or(Color::new(0, 0, 0, 0))
            };
            assert_eq!(palette.panel, color(ThemeToken::Surface));
            assert_eq!(palette.key, color(ThemeToken::TextMuted));
            assert_eq!(palette.key_active, color(ThemeToken::Primary));
            assert_eq!(palette.label, color(ThemeToken::Text));
            assert_eq!(palette.label_active, color(ThemeToken::OnPrimary));
            assert_eq!(palette.ring, color(ThemeToken::Border));
        }
    }

    #[test]
    fn the_two_themes_give_two_different_palettes() {
        // A theme switch has to reach the keyboard, or the theme would only change
        // half the window. `Theme` is not `Debug` — it holds a clock — so the
        // themes are named rather than printed.
        assert_ne!(
            Palette::from_theme(&Theme::dark()),
            Palette::from_theme(&Theme::light())
        );
    }

    #[test]
    fn the_panel_and_a_resting_key_are_different_colours_in_both_themes() {
        // `Surface` twice would leave the gap between two keys as the only thing
        // telling them apart, which on a dark head unit is nothing at all.
        for (name, theme) in [("dark", Theme::dark()), ("light", Theme::light())] {
            let palette = Palette::from_theme(&theme);
            assert_ne!(palette.panel, palette.key, "in the {name} theme");
            assert_ne!(
                palette.key, palette.key_active,
                "and the active key in {name}"
            );
        }
    }

    #[test]
    fn a_theme_that_holds_the_wrong_kind_of_value_still_answers() {
        // A token written with the wrong variant is a caller error, and it must not
        // take the frame down.
        let theme = Theme::dark();
        theme.set(ThemeToken::Primary, PropertyValue::Text("x".to_string()));
        assert_eq!(
            Palette::from_theme(&theme).key_active,
            Color::new(0, 0, 0, 255),
            "black rather than a panic"
        );
    }

    #[test]
    fn snapping_puts_a_themed_keyboard_where_its_theme_says_at_once() {
        // The failure this guards: a keyboard given a palette but never aimed still
        // paints the neutral defaults `Keyboard::new` wrote, so a themed keyboard
        // starts out grey. Built from `new` rather than through the `keyboard`
        // helper, which snaps.
        let mut nodes = Arena::new();
        let mut keyboard = Keyboard::new(&mut nodes);
        let themed = Palette::from_theme(&Theme::dark());
        assert_ne!(themed.panel, keyboard.panel.get(), "so this can fail");

        keyboard.set_palette(themed);
        keyboard.snap_to_state();

        assert_eq!(keyboard.panel.get(), themed.panel);
        assert_eq!(keyboard.key.get(), themed.key);
        assert_eq!(keyboard.key_active.get(), themed.key_active);
        assert_eq!(keyboard.label.get(), themed.label);
        assert_eq!(keyboard.label_active.get(), themed.label_active);
        assert_eq!(
            painted_in(&keyboard.paint(RECT), keyboard.key_rect(0, RECT).unwrap())
                .map(|(_, _, color)| color),
            Some(themed.key),
            "and the key is painted in it"
        );
        assert!(!keyboard.is_animating(), "a snap is not a transition");
    }

    #[test]
    fn setting_the_palette_leaves_the_keyboard_where_it_is() {
        // A theme switch is animated by the caller setting the palette and
        // animating toward it; a palette that moved the keyboard on its own would
        // make every theme switch instantaneous.
        let (_nodes, mut keyboard) = keyboard();
        let before = keyboard.panel.get();
        keyboard.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(keyboard.panel.get(), before);
    }

    #[test]
    fn a_theme_switch_crossfades_every_colour_a_key_uses() {
        // The contract is that the colours *move* rather than jumping, so this
        // checks the middle of the transition and not only its end.
        let (_nodes, mut keyboard) = keyboard();
        let from = Palette::from_theme(&Theme::dark());
        keyboard.snap_to_state();
        let to = Palette::from_theme(&Theme::light());
        assert_ne!(from.panel, to.panel, "so this can fail");
        keyboard.set_palette(to);
        keyboard.animate_to_state(motion());

        assert_eq!(keyboard.panel.get(), from.panel, "it starts where it was");
        assert!(keyboard.tick(ms(50)), "and something moved");
        let half = keyboard.panel.get();
        assert_ne!(half, from.panel, "it is not still the old panel");
        assert_ne!(half, to.panel, "nor already the new one");
        assert!(keyboard.tick(ms(50)));
        assert_eq!(keyboard.panel.get(), to.panel, "and it arrives");
        assert!(!keyboard.is_animating());
    }

    #[test]
    fn a_theme_switch_reaches_the_key_that_is_pressed() {
        // The two that are not the panel: a theme switch that left a pressed key on
        // the old palette would show a lit key in a colour nothing else uses.
        let (_nodes, mut keyboard) = keyboard();
        let from = Palette::from_theme(&Theme::dark());
        let to = Palette::from_theme(&Theme::light());
        let drawn = keyboard.key_rect(7, RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, drawn));
        keyboard.set_palette(to);
        keyboard.animate_to_state(motion());
        let _ = keyboard.tick(ms(100));

        assert_eq!(keyboard.key_active.get(), to.key_active);
        assert_eq!(
            painted_in(&keyboard.paint(RECT), drawn).map(|(_, _, c)| c),
            Some(to.key_active),
            "and the pressed key is painted in the new one"
        );
        assert_ne!(to.key_active, from.key_active, "which really did change");
    }

    #[test]
    fn a_resting_key_keeps_the_resting_colour_while_a_neighbour_is_pressed() {
        let (_nodes, keyboard) = keyboard();
        let target = keyboard.key_rect(20, RECT).unwrap_or(RECT);
        let other = keyboard.key_rect(21, RECT).unwrap_or(RECT);
        assert!(press(&keyboard, RECT, target));
        assert_eq!(
            painted_in(&keyboard.paint(RECT), target).map(|(_, _, c)| c),
            Some(keyboard.key_active.get())
        );
        assert_eq!(
            painted_in(&keyboard.paint(RECT), other).map(|(_, _, c)| c),
            Some(keyboard.key.get()),
            "and only the pressed one is active"
        );
    }

    #[test]
    fn snapping_a_mid_crossfade_keyboard_back_out_ends_the_transition() {
        // Without the `clear`, a transition already running writes over what the
        // snap just set when it arrives, and the keyboard drifts off again.
        let (_nodes, mut keyboard) = keyboard();
        keyboard.set_palette(Palette::from_theme(&Theme::light()));
        keyboard.animate_to_state(motion());
        assert!(keyboard.tick(ms(10)), "the crossfade is running");

        keyboard.set_palette(Palette::from_theme(&Theme::dark()));
        keyboard.snap_to_state();
        let snapped = keyboard.panel.get();
        assert!(!keyboard.tick(ms(500)), "and nothing arrives afterwards");
        assert_eq!(keyboard.panel.get(), snapped);
    }

    #[test]
    fn two_keyboards_keep_separate_clocks() {
        // An `AnimationClock` is cleared whole, which is why a widget owns one: a
        // shared clock would strand the other keyboard's crossfade.
        let (mut nodes, mut one) = keyboard();
        let mut other = Keyboard::new(&mut nodes);
        other.set_palette(Palette::from_theme(&Theme::dark()));
        other.snap_to_state();
        let target = Color::new(1, 2, 3, 255);
        let mut other_palette = other.palette();
        other_palette.panel = target;
        other.set_palette(other_palette);
        other.animate_to_state(motion());
        let other_before = other.panel.get();

        one.set_palette(Palette::from_theme(&Theme::light()));
        one.animate_to_state(motion());
        for _ in 0..2 {
            let _ = one.tick(ms(100));
        }
        assert_eq!(
            other.panel.get(),
            other_before,
            "the other one did not move off what its own snap left it at"
        );
        assert_ne!(other.panel.get(), target, "which is not where it was aimed");
        assert_eq!(
            other.panel.get(),
            Palette::from_theme(&Theme::dark()).panel,
            "it is still the colour its own snap left it at"
        );
    }

    #[test]
    fn a_keyboard_aimed_on_every_frame_settles_and_approaches_its_target() {
        // Five frames of aim-and-tick, and the arithmetic is the point: a re-aim
        // restarts each colour from wherever it is, so each tick covers a fifth of
        // what is left. On a channel that has to cross 80 the sequence is 16, then
        // 16 + 64/5 = 28.8, then 28.8 + 51.2/5 = 39.04, ...
        let (_nodes, mut keyboard) = keyboard();
        let from = Palette::from_theme(&Theme::dark());
        keyboard.snap_to_state();
        let to = Palette::from_theme(&Theme::light());
        keyboard.set_palette(to);
        for _ in 0..5 {
            keyboard.animate_to_state(motion());
            let _ = keyboard.tick(ms(20));
        }
        let start = f32::from(from.panel.r);
        let target = f32::from(to.panel.r);
        let got = f32::from(keyboard.panel.get().r);
        let mut want = start;
        for _ in 0..5 {
            want += (target - want) / 5.0;
        }
        assert!(
            (got - want).abs() < 2.0,
            "the fifth of the gap left: {got} against {want}"
        );
        assert!(keyboard.is_animating(), "so the crossfade is still running");
    }

    // -------------------------------------------------- the widget's own helpers

    #[test]
    fn covers_includes_the_edges_of_the_rect() {
        let rect = OFFSET_RECT;
        assert!(
            covers(rect, Offset::new(280.0, 380.0)),
            "the top-left corner"
        );
        assert!(covers(rect, Offset::new(1000.0, 680.0)), "the far corner");
        assert!(covers(rect, Offset::new(640.0, 530.0)), "the middle");
        assert!(!covers(rect, Offset::new(279.9, 530.0)), "just left of it");
        assert!(
            !covers(rect, Offset::new(1000.1, 530.0)),
            "just right of it"
        );
        assert!(!covers(rect, Offset::new(640.0, 379.9)), "just above it");
    }

    #[test]
    fn a_keyboard_laid_out_through_the_layout_pass_gets_the_box_it_asked_for() {
        // The demo's route: `Constraints::tight(keyboard.size())`, so this is what
        // `size` is for, and the laid-out rect is the one the arithmetic above was
        // asserted against.
        let (mut nodes, keyboard) = keyboard();
        let size = keyboard.size();
        let (_panel, placed) = on_a_panel(&keyboard, &mut nodes, Size::new(900.0, 700.0));
        assert_eq!(placed, Rect::new(0.0, 0.0, size.width, size.height));
        assert_eq!(
            keyboard.key_rect(0, placed),
            Some(Rect::new(8.0, 8.0, 65.0, 52.0)),
            "and the first key lands where the pure arithmetic says it does"
        );
    }
}
