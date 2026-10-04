//! The Toast widget: a notification that appears, says something, and leaves.
//!
//! There are **two types and not one**, and the split is the reading of the task
//! file rather than an addition to it.
//!
//! [`Toast`] is one notification: its message, how long it stays, its severity,
//! its own node, and the two numbers its arrival and its departure animate.
//! [`Toasts`] is the **host** that owns a `Vec<Toast>`, lays them out in a
//! stack, ticks them, drops them once they have gone, and is where
//! [`show`](Toasts::show) lives.
//!
//! # Why a host, when the task file says `Toast::show`
//!
//! The task file asks for `Toast::show(message, duration) -> Handle`, and
//! returning a bare handle would leave the caller with no properties to set and
//! **nowhere for placement to live**: requirement 2's *"Position: bottom of
//! screen (or top — configurable)"* and requirement 5's *"rendered as overlay
//! (on top of all other content)"* are decisions about **where** a toast goes,
//! and `ui_core` has no overlay layer, no window manager and no place that
//! knows the size of the screen a toast is drawn on. `Dialog::new` was re-read
//! the same way — see [`Dialog::new`](crate::widgets::dialog::Dialog::new),
//! which is a widget with a `handle()` rather than a constructor returning one.
//!
//! So `show` is a method on the host, it returns **the toast's handle** exactly
//! as the task file says, and the host is the thing a caller keeps. A caller
//! that wants a severity or a different message reaches the toast through
//! [`Toasts::toast`].
//!
//! # The paint order is the reverse of the visual order, and it is load-bearing
//!
//! The commands go out as **shadow, then the text, then the surface, then the
//! disc** — which is not the order they are drawn in, and getting it wrong puts
//! a toast's own text under its own background. Three facts about this pipeline
//! decide it, and all three are the reason a draw-command assertion cannot be
//! trusted here (`.ai/NEVERAGAIN.md` § *A draw-command assertion cannot see
//! where a command lands*):
//!
//! 1. **A translucent command cannot share a segment with an opaque one.**
//!    [`Batcher::submit_order`](crate::batch::Batcher::submit_order) groups
//!    each segment's opaque batches ahead of its translucent ones
//!    (`render.rs:1994`), so a surface and a fully opaque text run recorded
//!    together are submitted text-first and the surface lands on top of the
//!    text. Every colour this widget records is therefore premultiplied by
//!    `SURFACE_OPACITY` as well as by the toast's own opacity, which keeps
//!    all of them in the translucent group — and **that is why the constant is
//!    load-bearing**: at `1.0` the text's alpha would reach 255, the text would
//!    enter the opaque group, and the ordering below would stop working. It is
//!    a private constant for that reason, and
//!    `every_colour_a_toast_records_is_in_the_translucent_group` is the test
//!    that says so.
//! 2. **The translucent group is submitted reversed** (`batch.rs:286`), so
//!    within one segment the *last* translucent command recorded is the *first*
//!    drawn. The text is therefore recorded **before** the surface, and the
//!    surface **before** the disc: the surface and the disc share a batch (same
//!    shader, same blend mode, same clip) and so keep their own recording order
//!    inside it, while the text's batch is submitted **behind both of them** —
//!    last of all, which is what puts a message on the card rather than under it.
//!    Reading "behind" as "ahead" gives point 1's own defect: the surface drawn
//!    over the text.
//! 3. **A shadow is composited after everything its own segment recorded**
//!    (`render.rs:2011`), so it must be recorded **first** or it lands on top
//!    of the toast's own surface. [`Dialog`](crate::widgets::dialog::Dialog) has
//!    the opposite-looking order (`overlay → shadow → panel`) for the same
//!    reason: its opaque panel covers the shadow, and a toast's translucent
//!    surface would not.
//!
//! `the_shadow_lands_behind_the_surface_and_the_text_lands_on_it_in_submission`
//! runs the recorded commands through the real batcher and reads the order they
//! are submitted in. **A count of the four commands is satisfied by any of
//! twenty-four orders**, which is how an ordering requirement goes untested.
//!
//! # The icon is a disc, and it is not a glyph
//!
//! Requirement 2's icon is one [`Painter::circle`] in
//! [`Error`](ThemeToken::Error), [`Warning`](ThemeToken::Warning),
//! [`Success`](ThemeToken::Success) or [`Primary`](ThemeToken::Primary) for
//! error, warning, success and info. **A missing glyph is silently invisible**
//! — `ui_core::font`'s `FontSet` is closed at two faces and the demo's Lato
//! face was measured on this host as having none of ⚠ ℓ ✗ ▲ — so a glyph icon
//! would be an icon that is not drawn when the font changes, with nothing
//! failing. There is **no `Info` token**: adding one would change
//! [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables and the
//! transition every token takes part in during a switch, for a colour a switch
//! does not change. `Primary` is the house accent and is what "info" means
//! here. The icon is **optional**: no severity means no disc, and the text then
//! starts at the panel's padding.
//!
//! # The colours are premultiplied here, by hand, for a recorded reason
//!
//! `at_alpha` multiplies all four channels by the same factor, which is what
//! the solid pass expects and does not do: `render.rs:179` writes `frag_color`
//! verbatim and blends `GL_ONE, GL_ONE_MINUS_SRC_ALPHA`, so a `RoundedRect`
//! handed a non-premultiplied colour composites as `rgb + dst·(1 − a)` instead
//! of `rgb·a + dst·(1 − a)`. That defect is recorded, cross-cutting and
//! **unfixed** (`doc/ui/IMPLEMENTATION_STATE.md` § *The finding that is not
//! this task's: the solid pass does not premultiply*), and fixing it is not this
//! task. The precedent is the dialog's own `faded`, which interpolates toward
//! transparent black because *that is* the premultiplied fade.
//!
//! **It is a workaround with an expiry date, not a colour model**: the record
//! names the fix as one place (`quad_color`), and when the solid pass
//! premultiplies, this module's `at_alpha` — and the dialog's `faded` — become
//! a double multiply and must go.
//!
//! # A toast does not block input, structurally
//!
//! [`input::hit_test`](crate::input::hit_test) walks **down** from a root the
//! *caller* supplies, and a caller supplies the gallery's root. The host's node
//! is a **root of its own** ([`node::create`] gives it no parent, and nothing
//! here attaches it to anything), so nothing in the gallery's tree can reach a
//! toast or its host, and a tap that lands on a toast's drawn rect finds the
//! control **under** it exactly as if the toast were not there. There is no
//! flag and no hit test to keep in step: the property is a fact about the tree.
//! `a_toast_is_unreachable_from_the_gallerys_own_walk` is the test, and it is
//! the one acceptance criterion no draw-command assertion can see.
//!
//! **Each toast's node is a child of the host's**, which is what lets one
//! `paint_order` walk over the whole stack — the caller gains **one** node in
//! its paint order rather than one per toast. It does not weaken the property
//! above, because the walk that matters starts at the gallery's root and the
//! host is not in that tree.
//!
//! # `duration` is how long it is *there*
//!
//! The countdown starts when the toast is shown and the fade-out begins when it
//! reaches zero, so a three-second toast is on screen for about 3.15 seconds:
//! [`DurationFast`](ThemeToken::DurationFast) of arrival, three seconds of it,
//! and [`DurationFast`](ThemeToken::DurationFast) of leaving. The countdown is a
//! [`Property<Duration>`] the frame's delta accumulates into
//! ([`TextInput::tick`](crate::widgets::text_input::TextInput::tick)'s model),
//! **never a clock this widget reads** — `AGENTS.md` forbids a wall-clock test
//! and a toast that read one could not be tested at all.
//!
//! # What this widget does not do
//!
//! - **A toast with an action button, with a progress bar, or a queue.** All
//!   three are the task file's out of scope. The consequence worth stating is
//!   that **there is no cap on how many toasts may be live**, so *n* toasts
//!   cost *n* blurred shadows — see `SHADOW_BLUR` — and *n* is a number the
//!   caller chooses by how often it calls [`show`](Toasts::show).
//!
//! # Examples
//!
//! ```
//! use std::time::Duration;
//! use ui_core::arena::Arena;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::Rect;
//! use ui_core::widgets::toast::{Severity, Toasts};
//!
//! let mut nodes = Arena::new();
//! let mut toasts = Toasts::new(&mut nodes);
//! let screen = Rect::new(40.0, 30.0, 1000.0, 700.0);
//!
//! let handle = toasts.show(&mut nodes, "Settings saved", Duration::from_secs(3));
//! toasts.toast(0).expect("one toast").severity.set(Some(Severity::Success));
//!
//! // Nothing is drawn on the frame it is raised, and it is drawn in full once
//! // the arrival has run: the two numbers it animates are 1.0 and 0.
//! assert!(toasts.paint_toast(0, screen, &|_: char| 7.0, 20.0).is_empty());
//! for _ in 0..3 {
//!     let _ = toasts.tick(&mut nodes, Duration::from_millis(100));
//! }
//! let commands = toasts.paint_toast(0, screen, &|_: char| 7.0, 20.0);
//! assert!(!commands.is_empty(), "and the toast is drawn");
//!
//! // Three seconds later it is on its way out, and once the leaving has run it
//! // is gone from the host rather than left at zero opacity for ever.
//! for _ in 0..32 {
//!     let _ = toasts.tick(&mut nodes, Duration::from_millis(100));
//! }
//! assert_eq!(toasts.len(), 0);
//! assert_eq!(toasts.index_of(handle), None, "and its node with it");
//! ```

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::animation::{AnimationClock, Easing};
use crate::arena::{Arena, Handle};
use crate::layout::{Constraints, LayoutState, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::{Theme, ThemeToken};
use crate::widgets::button::Motion;
use crate::widgets::label::{
    layout_text, LayoutOptions, Line, TextAlign, TextLayout, Truncation, WrapMode,
};

/// The gap between the stack and the edge of the box it is laid out in:
/// [`SpacingLg`](ThemeToken::SpacingLg), which both themes hold at 24 pixels.
///
/// A copy rather than a read, for the reason
/// [`dialog`](crate::widgets::dialog)'s own three copies give: the widget has
/// no theme to read from, and a caller themes a host by binding its properties.
/// **It is the dialog's [`PANEL_PADDING`](crate::widgets::dialog::PANEL_PADDING)
/// as a number**, so a toast and a panel sit the same distance from the edge,
/// which is what makes a toast read as part of this interface rather than as a
/// notice pasted over it. The test
/// `the_theme_token_copies_are_the_tokens_values` holds both to their tokens.
const TOAST_MARGIN: f32 = 24.0;

/// The padding inside a toast: [`SpacingMd`](ThemeToken::SpacingMd), 16 pixels
/// in both themes.
///
/// **One step below the dialog's [`SpacingLg`](ThemeToken::SpacingLg)**, because
/// the padding scales with what it pads and a toast is a third of a dialog's
/// height: 24 pixels either side of a one-line message is a third of the
/// message, and the disc beside it would be swimming.
const TOAST_PADDING: f32 = 16.0;

/// The gap between the disc and the text, and between two toasts in the stack:
/// [`SpacingSm`](ThemeToken::SpacingSm), 8 pixels in both themes.
///
/// One constant for both, and neither is `TOAST_PADDING`: the gap *inside* a
/// toast is between two marks on one surface and the gap *between* two surfaces
/// is the whole of what separates them, and a stack whose toasts touch reads as
/// one tall card rather than as three notifications.
const TOAST_GAP: f32 = 8.0;

/// The corner radius: [`BorderRadiusMd`](ThemeToken::BorderRadiusMd), 8 pixels
/// in both themes.
///
/// **The default radius and not the dialog's
/// [`BorderRadiusLg`](ThemeToken::BorderRadiusLg)**, and the difference is
/// deliberate: a toast is a small surface, and a radius scaled to a small
/// surface reads as a pill. The **same** number is the shadow's radius, so the
/// shadow's silhouette and the toast's are one shape.
const TOAST_RADIUS: f32 = 8.0;

/// The message's font size: [`FontSizeMd`](ThemeToken::FontSizeMd), 14 pixels
/// in both themes.
///
/// The theme's body size, and a notification is body text: nothing in it is a
/// heading, and the dialog's [`FontSizeLg`](ThemeToken::FontSizeLg) title would
/// make a one-line message the loudest thing in a window that is mostly
/// gallery.
const MESSAGE_FONT: f32 = 14.0;

/// The width a toast is measured at when the box it is given is wider than
/// this, in pixels.
///
/// **Chosen, not measured.** 360 is a little under the gallery's 900-pixel text
/// panel and a little over the demo's own 400-pixel readout row, which puts a
/// notification at about 55 characters at the theme's body size and keeps it
/// from reading as a paragraph. It is a constant rather than a token because the
/// theme has no token for a notification's width, and adding one would change
/// [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables and the
/// transition every token takes part in during a switch, for a value a switch
/// does not change — the argument [`PANEL_MAX_WIDTH`](crate::widgets::dialog::PANEL_MAX_WIDTH)
/// makes in as many words.
///
/// What would reverse it: a capture of a real head unit at a real viewing
/// distance, which is the measurement this number stands in for.
const TOAST_MAX_WIDTH: f32 = 360.0;

/// How many lines of message a toast may show before it is cut short.
///
/// **Three, and the cap is what keeps a notification a notification.** Without
/// it a caller that hands `show` a paragraph gets a card the height of the
/// paragraph, which is a dialog with no way to dismiss it early. The dialog's
/// own body is capped the same way, by a pixel height rather than a line count;
/// this is a line count because the toast's caller supplies the line height and
/// the panel's does not.
///
/// What would reverse it: a token for it, which is the same argument as every
/// other un-tokened number here.
const TOAST_MAX_LINES: usize = 3;

/// The radius of the severity disc, in pixels.
///
/// **Chosen, not measured.** 8 is the theme's own
/// [`BorderRadiusMd`](ThemeToken::BorderRadiusMd) and therefore the same order as
/// the 14-pixel text the disc sits beside, which is the only requirement worth
/// making of an icon this size: it must not read as a letter and it must not
/// read as a dot. What would reverse it: a capture with the disc beside its text
/// at the real font size, which is what this number is standing in for.
const ICON_RADIUS: f32 = 8.0;

/// How present the toast is at rest, from `0.0` to `1.0`.
///
/// Requirement 2's *"surface color with slight transparency"*: 0.94 leaves six
/// percent of the page behind a toast, which is enough to tell the toast from
/// the window and not enough to read the text underneath it.
///
/// **This constant is load-bearing for the paint order, and that is not a
/// detail.** `at_alpha` multiplies every colour this widget records by it, so
/// the text, the disc and the surface all stay below `alpha 255` and therefore
/// all stay in the same (translucent) batch group — which is the only reason the
/// recording order in [`Toast::paint`] submits the way it does. **At `1.0` the
/// text would be opaque, would be submitted before the surface, and would be
/// drawn under it.** It is a private constant rather than a property for exactly
/// that reason: a caller who could set it to `1.0` would break the widget
/// silently, and there is nothing to give them.
///
/// What it costs, measured rather than argued: on the dark theme the message
/// composites to 242 rather than 255 (it is pulled six percent toward the
/// surface it is on), and on the light theme to 14 rather than 0. Both are
/// inside what a person can see, and both go away entirely when the solid pass
/// premultiplies — see `at_alpha`.
const SURFACE_OPACITY: f32 = 0.94;

/// How far below its place in the stack a toast is drawn while it is arriving
/// and leaving, in pixels.
///
/// **The task's 20, and it is half the toast's own height** (52 pixels with one
/// line at the theme's body size and the padding either side), which is what
/// makes the arrival read as *coming up from the edge* rather than as fading in
/// place. Anything above half a toast and the toast has visibly travelled
/// across the page; anything below a fifth and the slide is a fifth of a pixel
/// of movement at 60 frames a second, which is a flicker.
///
/// The direction is **down for both directions of the animation**: an arriving
/// toast comes up out of the bottom edge and a leaving one sinks back into it,
/// because the stack is anchored to that edge and a toast that slid away from
/// it would be sliding *into* the stack above it.
const SLIDE_OFFSET: f32 = 20.0;

/// The shadow's Gaussian standard deviation, in pixels.
///
/// [`Painter::shadow`](crate::paint::Painter::shadow) takes a standard
/// deviation, so this asks [`blur::kernel`](crate::render::blur::kernel) for
/// taps either side — **which 8 does not get.**
/// [`MAX_TAPS`](crate::render::blur::MAX_TAPS) is nine in total, so every σ
/// from 2 upwards is capped at four taps either side and what differs between 2
/// and 8 is the kernel's *weights*, not its width. It is **the dialog's number
/// and for the dialog's reasons**, which are recorded at
/// `SHADOW_BLUR`(crate::widgets::dialog::SHADOW_BLUR) with a measurement from
/// a capture of the running demo on 2026-10-03: the panel's shadow reached 11
/// pixels below the panel and 3 beside it.
///
/// **It has not been measured for a toast.** A toast is half a dialog's height
/// and 140 pixels narrower, so the same σ is a larger fraction of it; what
/// would reverse it is a capture of a toast's own edge, in pixels, which is
/// what the task's frame-rate note below is the other half of.
///
/// **And it is a performance number**: the blur costs two full-window passes and
/// a target bind **per shadow per frame**, so *n* live toasts cost *n* of them.
/// There is no cap on *n* — see the module doc's *Out of scope* — and this is
/// the constant that makes that matter.
const SHADOW_BLUR: f32 = 8.0;

/// How far the shadow falls below the toast, in pixels.
///
/// The dialog's number for the dialog's reason — *"a centred shadow under a
/// panel reads as a halo around it rather than as a light source above it"* —
/// at two thirds of its value, because a toast is half as tall: 8 pixels of
/// offset on a 52-pixel card puts the shadow's shoulder below its own bottom
/// edge, which is where a drop shadow has to start to read as one.
const SHADOW_OFFSET_Y: f32 = 8.0;

/// The most opaque the shadow gets, from `0.0` to `1.0`.
///
/// **Half, and the same half the dialog uses.** A toast sits directly on the
/// page with no scrim under it, so the shadow's only job is to lift the toast
/// off the content — and at full opacity over a dark background it is not
/// depth, it is a black card one step larger than the toast.
///
/// **Chosen, not measured for a toast.** What is settled is that it is not
/// `1.0`, and that the shadow is black in both themes and has no token behind
/// it: the dark theme's [`Background`](ThemeToken::Background) is `18 18 18` and
/// the light theme's is `255 255 255`, so a shadow taken from the token would be
/// black on one theme and white on the other.
const SHADOW_ALPHA: f32 = 0.5;

/// The colours a toast's surface, its message and its disc are drawn in.
///
/// The same shape as [`dialog::Palette`](crate::widgets::dialog::Palette) and
/// [`button::Palette`](crate::widgets::button::Palette): a value rather than a
/// read from a theme inside the widget, because the property graph is what
/// carries a theme switch to a widget holding a property. Each toast holds its
/// own properties, so [`Toasts::set_palette`] reaches the ones already on
/// screen as well as the ones raised after it.
///
/// The shadow is **not** in here, for the reason `SHADOW_ALPHA` gives: it is
/// black in both themes and on no token.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The surface behind the message: [`Surface`](ThemeToken::Surface), the
    /// token both themes use for a raised card.
    pub surface: Color,
    /// The message's colour: [`Text`](ThemeToken::Text), which both themes
    /// define as legible on their own surface.
    pub text: Color,
    /// The disc for [`Severity::Error`]: [`Error`](ThemeToken::Error).
    pub error: Color,
    /// The disc for [`Severity::Warning`]: [`Warning`](ThemeToken::Warning).
    pub warning: Color,
    /// The disc for [`Severity::Success`]: [`Success`](ThemeToken::Success).
    pub success: Color,
    /// The disc for [`Severity::Info`]:
    /// [`Primary`](ThemeToken::Primary) — see the module doc for why there is no
    /// `Info` token.
    pub info: Color,
}

impl Default for Palette {
    /// Returns a toast that is legible with no theme at all.
    ///
    /// The same neutral shape as the dialog's and the button's own defaults: a
    /// dark grey surface, a near-white message, and four discs a reader can tell
    /// apart in the dark, which is what makes them a severity rather than a
    /// decoration.
    fn default() -> Self {
        Palette {
            surface: Color::new(48, 48, 48, 255),
            text: Color::new(245, 245, 245, 255),
            error: Color::new(207, 102, 121, 255),
            warning: Color::new(255, 183, 77, 255),
            success: Color::new(102, 187, 106, 255),
            info: Color::new(187, 134, 252, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes: its [`Surface`](ThemeToken::Surface)
    /// for the toast, its [`Text`](ThemeToken::Text) for the message and its
    /// three severity tokens with [`Primary`](ThemeToken::Primary) for info.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::toast::Palette;
    ///
    /// let dark = Palette::from_theme(&Theme::dark());
    /// let light = Palette::from_theme(&Theme::light());
    /// assert_ne!(dark, light, "and two themes really are two palettes");
    /// ```
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            surface: token_color(theme, ThemeToken::Surface),
            text: token_color(theme, ThemeToken::Text),
            error: token_color(theme, ThemeToken::Error),
            warning: token_color(theme, ThemeToken::Warning),
            success: token_color(theme, ThemeToken::Success),
            info: token_color(theme, ThemeToken::Primary),
        }
    }
}

/// How urgent a toast says it is, and so which colour its disc is drawn in.
///
/// **Four cases and three tokens**, because the theme has no
/// [`Info`](ThemeToken) token: `Error`, `Warning` and `Success` are tokens of
/// their own and info is [`Primary`](ThemeToken::Primary), the house accent.
/// Adding a token for it would change
/// [`ThemeToken::all`](crate::theme::ThemeToken::all), both theme tables and the
/// transition every token takes part in during a switch, for a value a switch
/// does not change.
///
/// A toast with **no** severity at all draws no disc, and its message starts at
/// the panel's padding rather than beside one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Something failed: the [`Error`](ThemeToken::Error) disc.
    Error,
    /// Something needs attention but did not fail: the
    /// [`Warning`](ThemeToken::Warning) disc.
    Warning,
    /// Something worked: the [`Success`](ThemeToken::Success) disc.
    Success,
    /// Something is worth saying and is not one of the three above: the
    /// [`Primary`](ThemeToken::Primary) disc.
    Info,
}

/// Which edge of the box the stack of toasts grows from.
///
/// **Two cases and no theme token**, for the reason [`Severity`] gives: there is
/// no token for where a notification goes, and adding one would change
/// [`ThemeToken::all`](crate::theme::ThemeToken::all) and every theme switch
/// with it. [`Toasts::set_anchor`] is the whole of the configuration.
///
/// The name says the edge the stack is **anchored to**, and the stack grows
/// *away* from it: [`Bottom`](Anchor::Bottom) is the task's default and puts the
/// newest toast nearest the bottom edge, [`Top`](Anchor::Top) puts the newest
/// furthest from the top edge, and both have the newest **last in the stack**
/// because a stack reads from its oldest end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// The stack hangs from the bottom edge of the box, growing upward.
    Bottom,
    /// The stack hangs from the top edge of the box, growing downward.
    Top,
}

/// One notification: its message, its severity, its own node, and the two
/// numbers its arrival and its departure animate.
///
/// **A toast is never constructed by a caller.** [`Toasts::show`] is the
/// constructor, and it is the only one, because a toast nobody can reach is a
/// toast that is never ticked — so it never counts down, never fades out and is
/// never removed. [`Toasts::toast`] hands out the live one to write its
/// properties; [`handle`](Toast::handle) is its node.
///
/// The properties that hold a theme token are bound by the caller rather than
/// read from a theme here, for the reason the rest of the library reads tokens
/// through properties. The defaults are neutral literals, the way
/// [`Dialog`](crate::widgets::dialog::Dialog)'s are.
pub struct Toast {
    /// The message, laid out in `TOAST_MAX_LINES` lines at the theme's body
    /// size.
    ///
    /// An empty message draws a surface and no text, and a toast whose message is
    /// wider than `TOAST_MAX_WIDTH` is wrapped and then cut with an ellipsis —
    /// which is the dialog's body treatment for the same reason: a notification
    /// is one thing to read, not a paragraph to scroll.
    pub message: Property<String>,
    /// How long the toast stays, from the moment it is shown to the moment it
    /// starts to leave.
    ///
    /// **Not** how long it is on screen: the two transitions run on top of it, so
    /// a three-second toast is up for about 3.15 seconds. See the module doc.
    pub duration: Duration,
    /// Whether the toast is showing.
    ///
    /// It goes `false` the instant the countdown runs out and the fade-out
    /// starts, so a caller that routes input through this reads "is it asking
    /// for anything" rather than "is any of it still on the screen" — which is
    /// [`is_drawn`](Toast::is_drawn)'s question and the same distinction the
    /// dialog's `visible` and `is_drawn` make.
    pub visible: Property<bool>,
    /// The severity, and with it whether a disc is drawn at all.
    ///
    /// `None` — the default — is a toast with no icon, and requirement 2 calls
    /// the icon optional.
    pub severity: Property<Option<Severity>>,
    /// The surface behind the message.
    pub surface: Property<Color>,
    /// The message's colour.
    pub text: Property<Color>,
    /// The disc drawn for [`Severity::Error`].
    pub error: Property<Color>,
    /// The disc drawn for [`Severity::Warning`].
    pub warning: Property<Color>,
    /// The disc drawn for [`Severity::Success`].
    pub success: Property<Color>,
    /// The disc drawn for [`Severity::Info`].
    pub info: Property<Color>,
    /// The corner radius, and the shadow's.
    pub border_radius: Property<f32>,
    /// The toast's own opacity, `0.0` while it is leaving and `1.0` while it is
    /// there.
    ///
    /// Written by the arrival and the departure, and **not** the number the
    /// drawing multiplies into a colour on its own: see `at_alpha` and
    /// `SURFACE_OPACITY`, which is why this is `1.0` at rest and the surface
    /// is not quite opaque.
    pub opacity: Property<f32>,
    /// How far below its place in the stack the toast is drawn, in pixels.
    ///
    /// `SLIDE_OFFSET` while it is arriving and leaving it and `0.0` while it
    /// is there, and **added to the rect the host laid out** rather than
    /// replacing it — so a toast slides within its own place in the stack and
    /// never moves its neighbours.
    pub slide: Property<f32>,
    palette: Palette,
    clock: Rc<RefCell<AnimationClock>>,
    motion: Rc<Cell<Motion>>,
    /// How far through its own duration this toast is, accumulated from the
    /// frames' deltas.
    elapsed: Cell<Duration>,
    node: Handle,
}

impl Toast {
    /// Creates a toast that says `message` for `duration`, in `nodes`.
    ///
    /// Private because [`Toasts::show`] is the only way to raise one: the host
    /// owns the vector, and a toast nobody holds is a toast nobody ticks.
    ///
    /// The toast is created **mid-flight**: `visible` is already true, the
    /// opacity is at `0.0` and the slide at `SLIDE_OFFSET`, and both are
    /// animating toward their resting values over [`motion`](Toast::motion). A
    /// frame drawn between this and the first [`tick`](Toast::tick) therefore
    /// shows the toast where its arrival starts rather than at rest, which is
    /// what [`Property::animate_to`](crate::property::Property::animate_to)
    /// guarantees by writing the start value itself.
    fn new(
        nodes: &mut Arena<WidgetNode>,
        message: impl Into<String>,
        duration: Duration,
        palette: Palette,
        motion: Motion,
    ) -> Self {
        let toast = Toast {
            message: Property::new(message.into()),
            duration,
            // `false` and not `true`: `start` below is what turns it on and
            // starts the two animations, and it declines a toast that is already
            // showing — so a toast created `true` would arrive already finished.
            visible: Property::new(false),
            severity: Property::new(None),
            surface: Property::new(palette.surface),
            text: Property::new(palette.text),
            error: Property::new(palette.error),
            warning: Property::new(palette.warning),
            success: Property::new(palette.success),
            info: Property::new(palette.info),
            border_radius: Property::new(TOAST_RADIUS),
            // The two values the arrival starts from, written here rather than by
            // the transition: the clock is ticked by the host, which may not tick
            // it before the first paint.
            opacity: Property::new(0.0),
            slide: Property::new(SLIDE_OFFSET),
            palette,
            clock: Rc::new(RefCell::new(AnimationClock::new())),
            motion: Rc::new(Cell::new(motion)),
            elapsed: Cell::new(Duration::ZERO),
            // **Tight to nothing, deliberately.** The toast's rect is the host's
            // computation and this node's own rect is nowhere near it — the same
            // property the dialog's action buttons have, and for the same reason:
            // the nodes exist for the paint order and nothing declares where they
            // go. An empty rect is the honest version of that, where the default
            // unbounded one would place the node across the whole parent and make
            // it look like a hit target the toast does not have.
            node: node::create(
                nodes,
                LayoutState::new().with_constraints(Constraints::tight(Size::ZERO)),
            ),
        };
        toast.start(true);
        toast
    }

    /// Returns the toast's node in the arena.
    ///
    /// **The node is under [`Toasts`]'s node and nowhere else**, which is what
    /// puts every toast in one paint-order walk and is also why the gallery's own
    /// walk cannot reach any of them — see the module doc.
    ///
    /// The handle is what [`Toasts::show`] returns, and what a caller matches
    /// against to find out *which* toast a node belongs to.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Sets the colours this toast is drawn in, and leaves its current
    /// appearance where it is.
    ///
    /// Immediate rather than animated, on the dialog's argument: a toast's
    /// colours change only when the theme does, and a theme switch is a 300 ms
    /// crossfade of the page underneath a toast that is itself mid-flight — two
    /// animations on one clock is one too many.
    ///
    /// What would reverse it: a caller that switches a palette per toast, which
    /// the task file does not ask for.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
        self.surface.set(palette.surface);
        self.text.set(palette.text);
        self.error.set(palette.error);
        self.warning.set(palette.warning);
        self.success.set(palette.success);
        self.info.set(palette.info);
    }

    /// Returns the colours this toast is drawn in.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets how long the arrival and the departure take and the curve they
    /// follow.
    ///
    /// **`&self` and not `&mut self`**, because the motion is behind a
    /// [`Cell`] on an `Rc`: the host writes one for every toast
    /// it owns without holding a mutable borrow of any of them.
    pub fn set_motion(&self, motion: Motion) {
        self.motion.set(motion);
    }

    /// Returns how long the arrival and the departure are currently timed.
    ///
    /// The default is `default_motion()`, which is the theme's
    /// [`DurationFast`](ThemeToken::DurationFast) with its
    /// [`EasingStandard`](ThemeToken::EasingStandard) — **which is
    /// [`Motion::from_theme`]'s own answer**, because requirement 4 asks for
    /// every one of the four animations over exactly that duration, and
    /// [`DurationFast`](crate::theme::ThemeToken::DurationFast) is 150 ms in both
    /// themes. The dialog deliberately does
    /// *not* use `from_theme`, for a bounce that wants twice the time.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::toast::Toasts;
    ///
    /// let mut nodes = Arena::new();
    /// let mut toasts = Toasts::new(&mut nodes);
    /// toasts.set_motion(Motion::from_theme(&Theme::dark()));
    /// assert_eq!(
    ///     toasts.motion().duration,
    ///     std::time::Duration::from_millis(150),
    ///     "which is the theme's own fast duration"
    /// );
    /// ```
    #[must_use]
    pub fn motion(&self) -> Motion {
        self.motion.get()
    }

    /// Brings the toast on screen, and reports whether it was not showing
    /// already.
    ///
    /// Both transitions start from wherever their properties stand — which is
    /// [`Property::animate_to`](crate::property::Property::animate_to)'s own
    /// behaviour, it reads the property before it writes the first frame — so **a
    /// toast brought back half way out comes from half way rather than snapping**.
    /// `a_toast_brought_back_half_way_out_comes_from_half_way` is the test, with
    /// the control beside it: the same toast arriving from nothing would be at
    /// 0.5 and this one is at 0.625.
    ///
    /// This is the dialog's
    /// [`present`](crate::widgets::dialog::Dialog::present) behaviour, and the
    /// reason both live on one clock each.
    #[must_use]
    pub fn present(&self) -> bool {
        self.start(true)
    }

    /// Sends the toast on its way, and reports whether it was showing.
    ///
    /// **The countdown is not rewound**, so a toast that comes back has the time
    /// it had left rather than the whole of its duration again. That is what
    /// makes [`dismiss`](Toast::dismiss) usable as the countdown's own answer:
    /// the host calls it when the countdown ends, and it is the same code path a
    /// caller would use to dismiss early.
    #[must_use]
    pub fn dismiss(&self) -> bool {
        self.start(false)
    }

    /// Advances the toast's animations and its countdown by `delta`, and reports
    /// whether anything that is drawn has changed.
    ///
    /// It is the toast's frame integration, the same contract every other
    /// widget's [`tick`](crate::widgets::button::Button::tick) has: call it once
    /// a frame, before the paint pass. It is true on every frame of either
    /// transition and on the **one** frame the countdown reaches zero, so a
    /// caller that repaints only when this is true repaints exactly while
    /// something a viewer can see has moved.
    ///
    /// **The countdown is an accumulator and never a clock**: the delta this is
    /// given is the whole of what makes time pass, so
    /// [`TextInput::tick`](crate::widgets::text_input::TextInput::tick)'s
    /// argument applies here and `Instant::now()` would make this untestable.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        let mut changed = self.clock.borrow_mut().tick(delta);
        if self.visible.get() {
            let remaining = self.remaining();
            self.elapsed.set(self.elapsed.get().saturating_add(delta));
            // The phase **flip** and not "the countdown is over": a toast that
            // stays visible for three seconds has nine hundred of those frames
            // where the remaining time changed and nothing was drawn differently.
            if remaining > Duration::ZERO && self.remaining() == Duration::ZERO {
                let _ = self.dismiss();
                changed = true;
            }
        }
        changed
    }

    /// Returns whether the arrival or the departure is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }

    /// Returns whether `paint` would record anything this frame.
    ///
    /// It is [`visible`](Toast::visible) **or** a running transition, which is
    /// what lets a dismissed toast fade out while it is no longer the thing the
    /// caller would route anything to. A toast that has left records nothing —
    /// not a transparent surface and not a transparent shadow.
    #[must_use]
    pub fn is_drawn(&self) -> bool {
        self.visible.get() || self.is_animating()
    }

    /// Returns how far through its own [`duration`](Toast::duration) this toast
    /// is.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed.get()
    }

    /// Returns how much longer this toast has to stay, or zero once it has
    /// started to leave.
    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.duration.saturating_sub(self.elapsed.get())
    }

    /// Returns the rect the toast is drawn in, when it is given one.
    ///
    /// It is the rect as drawn: the slide is applied, so the number here is the
    /// number the pixels land in on this frame rather than the toast's place in
    /// the stack. [`Toasts::toast_rect`] returns the place.
    ///
    /// `advance` and `line_height` are the measurements
    /// [`Button::content_size`](crate::widgets::button::Button::content_size)
    /// takes, and they are the same two the host laid the stack out with.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::toast::Toasts;
    ///
    /// let mut nodes = Arena::new();
    /// let mut toasts = Toasts::new(&mut nodes);
    /// toasts.show(&mut nodes, "Settings saved", Duration::from_secs(3));
    /// for _ in 0..3 {
    ///     let _ = toasts.tick(&mut nodes, Duration::from_millis(100));
    /// }
    /// let commands = toasts.toast(0).expect("one toast").paint(
    ///     ui_core::paint::Rect::new(360.0, 654.0, 360.0, 52.0),
    ///     &|_: char| 7.0,
    ///     20.0,
    /// );
    /// // Shadow, the message, the surface: three commands, in the order the
    /// // module doc says they are recorded and not the order they are drawn.
    /// assert!(matches!(
    ///     commands.first(),
    ///     Some(ui_core::paint::DrawCommand::Shadow { .. })
    /// ));
    /// ```
    #[must_use]
    pub fn paint(
        &self,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Vec<DrawCommand> {
        if !self.is_drawn() {
            return Vec::new();
        }
        // **The one number every colour below is multiplied by**, and
        // `SURFACE_OPACITY`'s own doc says why it is on the content and not
        // only on the surface.
        let presence = self.opacity.get().clamp(0.0, 1.0) * SURFACE_OPACITY;
        if presence <= 0.0 {
            return Vec::new();
        }
        let line_height = line_height.max(0.0);
        let measured = measure(
            rect.width,
            &self.message.get(),
            self.severity.get().is_some(),
            line_height,
            advance,
        );
        let radius = self.border_radius.get();
        // The slide moves the whole toast — surface, content and shadow — and
        // nothing else, because the host laid the stack out where this toast
        // belongs and the slide is the toast's own arrival.
        let drawn = Rect::new(rect.x, rect.y + self.slide.get(), rect.width, rect.height);

        let mut painter = Painter::new();
        // **First, and the reason is that a shadow is composited after
        // everything its own segment recorded** (`render.rs:2011`). Recorded
        // second it would land on top of the surface below, and a translucent
        // surface does not cover it the way the dialog's opaque panel does.
        painter.shadow(
            drawn,
            radius,
            at_alpha(shadow_colour(), presence * SHADOW_ALPHA),
            SHADOW_BLUR,
            (0.0, SHADOW_OFFSET_Y),
        );
        // **The text before the surface**, because the translucent group is
        // submitted reversed: a batch recorded after another is drawn before it.
        // Two one-line messages do not overlap, so the text runs' own order
        // among themselves is not a question.
        let mut y = drawn.y + TOAST_PADDING;
        if measured.present {
            let colour = at_alpha(self.text.get(), presence);
            let x = drawn.x + TOAST_PADDING + measured.inset;
            for line in &measured.lines {
                painter.text(x + line.x_offset, y, &line.text, colour, MESSAGE_FONT, 0.0);
                y += line_height;
            }
        }
        painter.rounded_rect(drawn, radius, at_alpha(self.surface.get(), presence));
        // **The disc last**, because it shares a batch with the surface — same
        // shader, same blend mode, same clip — and a merged batch keeps its own
        // recording order inside it. Recorded before the surface it would be in
        // that batch's first position, and the reversed group would draw the
        // pair as disc-then-surface with the disc under the toast it belongs to.
        if let Some(severity) = self.severity.get() {
            let centre = (
                drawn.x + TOAST_PADDING + ICON_RADIUS,
                drawn.y + TOAST_PADDING + measured.text_height / 2.0,
            );
            painter.circle(
                centre,
                ICON_RADIUS,
                at_alpha(self.severity_colour(severity), presence),
            );
        }
        painter.finish()
    }

    /// Returns the colour this toast draws [`severity`]'s disc in.
    fn severity_colour(&self, severity: Severity) -> Color {
        match severity {
            Severity::Error => self.error.get(),
            Severity::Warning => self.warning.get(),
            Severity::Success => self.success.get(),
            Severity::Info => self.info.get(),
        }
    }

    /// Starts the arrival or the departure toward `showing`, and reports whether
    /// that changed anything.
    ///
    /// **The clock is cleared first, and that is the contract
    /// [`AnimationClock::add`](crate::animation::AnimationClock::add) states**:
    /// "starting a second animation over one already running therefore leaves
    /// both writing it, and the last to be ticked wins; a caller that means to
    /// replace an animation clears the clock with
    /// [`clear`](crate::animation::AnimationClock::clear) first." This widget
    /// means to replace, so it clears. **Each toast owns a clock rather than
    /// sharing the host's** for the same reason the dialog's `Transition` does,
    /// and `clear` is whole-clock: a shared one would strand the host's other
    /// toasts' arrivals.
    ///
    /// **What `clear()` is worth here is worth stating precisely, because it is
    /// not what holds the interrupted transition correct.** With the line removed,
    /// the four animations on the clock still produce the *same numbers*, because
    /// `add` appends and a tick writes the vector in order — so the arriving pair
    /// is written after the interrupted pair and wins every frame. The line is
    /// therefore **defensive against `AnimationClock`'s insertion order rather
    /// than load-bearing today**, and no assertion on a value can kill its
    /// removal; the two interruption tests below hold the *behaviour*
    /// ([`present`](Toast::present), and this method's caller
    /// [`dismiss`](Toast::dismiss)), which is what a caller can see.
    fn start(&self, showing: bool) -> bool {
        if self.visible.get() == showing {
            return false;
        }
        self.visible.set(showing);
        let motion = self.motion.get();
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        let (opacity, slide) = if showing {
            (1.0, 0.0)
        } else {
            (0.0, SLIDE_OFFSET)
        };
        clock.add(
            self.opacity
                .animate_to(opacity, motion.duration, motion.easing),
        );
        clock.add(self.slide.animate_to(slide, motion.duration, motion.easing));
        true
    }
}

/// A stack of notifications: the host that owns them, places them, ticks them
/// and drops them.
///
/// It is **a root of its own node**, which is the whole of how a toast stays out
/// of the way — see the module doc. Its node is laid out to the box the stack
/// is placed in, and **each toast's node is its child**, so a caller reaches the
/// whole stack with one walk of its paint order.
///
/// Nothing here needs the caller's input router, and that is the point: a toast
/// **consumes no event at all**, by construction rather than by a handler that
/// declines them. There is no `on_event` to route to and no flag to set.
pub struct Toasts {
    /// The live toasts, oldest first, and the newest last.
    toasts: Vec<Toast>,
    /// Which edge the stack hangs from.
    anchor: Anchor,
    /// The colours a toast raised from now on is created with.
    palette: Palette,
    /// The motion a toast raised from now on arrives and leaves with.
    motion: Motion,
    node: Handle,
}

impl Toasts {
    /// Creates an empty stack in `nodes`, and returns it.
    ///
    /// The stack draws nothing until [`show`](Toasts::show) is called. Its node
    /// has no parent — see the module doc — and its children are the toasts, so a
    /// caller that walks it reaches every one of them.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::toast::Toasts;
    ///
    /// let mut nodes = Arena::new();
    /// let toasts = Toasts::new(&mut nodes);
    /// assert_eq!(toasts.len(), 0);
    /// assert_eq!(
    ///     nodes.get(toasts.handle()).expect("its own node").parent(),
    ///     None,
    ///     "and the node is a root of its own, which is what keeps a toast \
    ///      out of the gallery's input walk"
    /// );
    /// assert!(
    ///     toasts.paint_toast(0, Rect::new(0.0, 0.0, 800.0, 600.0), &|_: char| 7.0, 20.0)
    ///         .is_empty(),
    ///     "so there is nothing to draw"
    /// );
    /// ```
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>) -> Self {
        Toasts {
            toasts: Vec::new(),
            anchor: Anchor::Bottom,
            palette: Palette::default(),
            motion: default_motion(),
            // No parent: this is the reason a toast cannot block the gallery's
            // input, and `node::create` is what makes it so.
            node: node::create(nodes, LayoutState::new()),
        }
    }

    /// Returns the stack's own node in the arena.
    ///
    /// **Lay it out to the box the stack is placed in** — the window, for a
    /// notification — because that box is what every toast's rect is measured
    /// against. The handle is also the root a caller's paint order walks to reach
    /// every toast at once.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Raises a toast saying `message` for `duration`, and returns its node.
    ///
    /// This is the task file's `Toast::show(message, duration) -> Handle`, on the
    /// host rather than on the toast — see the module doc for why. **The handle
    /// is the toast's own**, so a caller can tell two of them apart and match it
    /// against [`index_of`](Toasts::index_of).
    ///
    /// The toast is created **already arriving**: it is
    /// [`visible`](Toast::visible) on the frame this is called, its opacity is at
    /// `0.0` and its slide at `SLIDE_OFFSET`, so a frame painted before the
    /// first [`tick`](Toasts::tick) shows nothing at all rather than a finished
    /// toast. To write its message, its severity or its duration, reach it
    /// through [`toast`](Toasts::toast) — the properties take `&self`, so a
    /// raised toast can be given a severity without another mutable borrow.
    ///
    /// **`nodes` is taken because a toast is a node of its own**, the same
    /// argument [`Dialog::add_action`](crate::widgets::dialog::Dialog::add_action)
    /// makes.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::toast::{Severity, Toasts};
    ///
    /// let mut nodes = Arena::new();
    /// let mut toasts = Toasts::new(&mut nodes);
    /// let first = toasts.show(&mut nodes, "Settings saved", Duration::from_secs(3));
    /// toasts.show(&mut nodes, "Connection lost", Duration::from_secs(5));
    ///
    /// assert_eq!(toasts.len(), 2, "both are live");
    /// assert_eq!(toasts.index_of(first), Some(0), "and the first is still the first");
    /// toasts
    ///     .toast(1)
    ///     .expect("two toasts")
    ///     .severity
    ///     .set(Some(Severity::Error));
    /// assert_eq!(
    ///     toasts.toast(1).expect("two toasts").severity.get(),
    ///     Some(Severity::Error)
    /// );
    /// ```
    pub fn show(
        &mut self,
        nodes: &mut Arena<WidgetNode>,
        message: impl Into<String>,
        duration: Duration,
    ) -> Handle {
        let toast = Toast::new(nodes, message, duration, self.palette, self.motion);
        let handle = toast.handle();
        // **The one node this widget takes from `node::attach`, and its answer is
        // discarded rather than asserted on** for the dialog's reason: the child
        // was created in this call, in this caller's arena, so it has no parent,
        // is not this node and cannot close a cycle. `attach` is `#[must_use]`
        // because a caller handing over a `Handle` *can* be refused.
        let _ = node::attach(nodes, self.node, handle);
        self.toasts.push(toast);
        handle
    }

    /// Returns how many toasts are live, including any that are on their way
    /// out.
    #[must_use]
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// Returns whether no toast is live.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Returns the toast at `index`, oldest first, or `None` when there is no
    /// such toast.
    ///
    /// It is the way to write a toast's properties after it is raised: the
    /// properties take `&self`, so
    /// `toasts.toast(0)?.severity.set(…)` needs nothing more than a shared
    /// borrow of the host.
    #[must_use]
    pub fn toast(&self, index: usize) -> Option<&Toast> {
        self.toasts.get(index)
    }

    /// Returns the index of the toast whose node is `handle`, or `None` when no
    /// toast has it.
    ///
    /// It is what a paint pass walks its nodes through: the stack's own node
    /// answers `None` here, and so does every node that is not a toast's.
    #[must_use]
    pub fn index_of(&self, handle: Handle) -> Option<usize> {
        self.toasts.iter().position(|toast| toast.node == handle)
    }

    /// Returns the node of the toast at `index`, or `None` when there is no such
    /// toast.
    #[must_use]
    pub fn toast_handle(&self, index: usize) -> Option<Handle> {
        self.toast(index).map(Toast::handle)
    }

    /// Returns which edge the stack hangs from.
    ///
    /// [`Bottom`](Anchor::Bottom) until a caller says otherwise, which is the
    /// task file's *"bottom of screen (or top — configurable)"*: a
    /// notification belongs at the edge of the screen the controls are not on,
    /// and the demo's are at the bottom.
    #[must_use]
    pub fn anchor(&self) -> Anchor {
        self.anchor
    }

    /// Sets which edge the stack hangs from, and reports whether it moved.
    ///
    /// Immediate, and the stack re-places itself on the next paint: a toast is
    /// placed by its host rather than by its caller, so there is no rect to give
    /// back and nothing to animate — a stack that slid from the bottom of the
    /// screen to the top of it would be two toasts' worth of animation for
    /// something that happens at start-up.
    pub fn set_anchor(&mut self, anchor: Anchor) -> bool {
        if self.anchor == anchor {
            return false;
        }
        self.anchor = anchor;
        true
    }

    /// Sets the colours toasts raised from now on are created with, and writes
    /// them into the ones already on screen.
    ///
    /// **Both halves are needed and the second is the reason this is here**: a
    /// theme switch happens while toasts are up — they are the one thing that is
    /// on screen while the theme moves — and a toast left on the dark palette
    /// over a light gallery is a dark card floating in a white window.
    ///
    /// Immediate rather than animated, on [`Toast::set_palette`]'s argument.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
        for toast in &mut self.toasts {
            toast.set_palette(palette);
        }
    }

    /// Returns the colours toasts raised from now on are created with.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets how long the arrival and the departure take and the curve they
    /// follow, for the toasts already on screen as well as for the ones to come.
    pub fn set_motion(&mut self, motion: Motion) {
        self.motion = motion;
        for toast in &self.toasts {
            toast.set_motion(motion);
        }
    }

    /// Returns how long the arrival and the departure are currently timed.
    #[must_use]
    pub fn motion(&self) -> Motion {
        self.motion
    }

    /// Returns whether any toast would record anything this frame.
    #[must_use]
    pub fn is_drawn(&self) -> bool {
        self.toasts.iter().any(Toast::is_drawn)
    }

    /// Advances every toast by `delta`, **returns the nodes of the ones that have
    /// gone to `nodes`**, and reports whether anything that is drawn has changed.
    ///
    /// It is the stack's frame integration, and it is **the only place a toast is
    /// ever removed** — from the host's vector *and* from the arena, in one call.
    /// The arena is taken for the reason
    /// [`List::release_all`](crate::widgets::list::List::release_all) takes it,
    /// and that doc is the argument in full: **removing a node from the arena is
    /// the only way to give it back**. Without it, a caller that raises a toast
    /// every few seconds accumulates one `WidgetNode` per toast ever raised, for
    /// the life of the process — the arena is a slot vector with a generation per
    /// slot, and **`List` is the only other widget in this library that gives one
    /// back.** It does so in three places, and all three are `Arena::remove`:
    /// [`release_all`](crate::widgets::list::List::release_all)
    /// (`list.rs:604`), the [`set_item_factory`](crate::widgets::list::List::set_item_factory)
    /// that calls it (`list.rs:589`), and the `take_row` path that drops a node
    /// whose `node::attach` failed (`list.rs:1031`).
    ///
    /// A removed node's handle **stops resolving** rather than aliasing whatever
    /// takes its slot, because [`Arena::remove`] bumps the slot's generation. A
    /// caller still holding a stale handle therefore skips it and cannot be
    /// pointed at somebody else's node.
    ///
    /// **What this does not reclaim is the caller's own bookkeeping**: a paint
    /// order built by appending to a `Vec` still holds one `Handle` per toast
    /// raised until that caller prunes it, which is a growing `Vec` of four-byte
    /// copies rather than of nodes — smaller, and still unbounded. `ui_demo`'s
    /// frame loop prunes, and it is a line.
    ///
    /// **The `&mut self` is a deviation from the widgets' usual `&self` tick, and
    /// it is not this repository's only one**:
    /// [`AnimationClock::tick`](crate::animation::AnimationClock::tick) takes
    /// `&mut self` because it drains its own vector, which is the same shape of
    /// reason. What is specific to this method is that a host dropping its own
    /// toasts has to mutate the `Vec` they live in, and every way of doing that
    /// behind `&self` costs more than the deviation: a `RefCell<Vec<Toast>>`
    /// would force every accessor to hand out a clone, because
    /// `toast(&self, i) -> Option<&Toast>` is unreachable through one. What
    /// callers depend on — call it once a frame, before the paint pass; `true`
    /// means something a viewer can see moved — is unchanged, and
    /// [`Toast::tick`] beside it is the `&self` one.
    #[must_use]
    pub fn tick(&mut self, nodes: &mut Arena<WidgetNode>, delta: Duration) -> bool {
        let mut changed = false;
        // **Back to front**, so that a toast dropped by the pass below cannot
        // shift the index of one this loop has not reached yet. Nothing is
        // removed here: the host does it once, after every toast has been given
        // its delta, so that `is_drawn` is asked of each of them at the same
        // moment.
        for index in (0..self.toasts.len()).rev() {
            if self.toasts[index].tick(delta) {
                changed = true;
            }
        }
        let mut gone: Vec<Handle> = Vec::new();
        self.toasts.retain(|toast| {
            if toast.is_drawn() {
                return true;
            }
            gone.push(toast.node);
            false
        });
        // **Detach before removing**, in that order, for
        // `List::release_all`'s reason: a node left in the host's children is a
        // child pointing at nothing, and `node::detach` is what takes it out.
        for handle in &gone {
            let _ = node::detach(nodes, self.node, *handle);
            let _ = nodes.remove(*handle);
        }
        changed || !gone.is_empty()
    }

    /// Returns the rect the toast at `index` is **placed** in inside `screen`, or
    /// `None` when there is no such toast.
    ///
    /// **The place and not the drawn rect**: the slide is not in it, because the
    /// place is what lays the stack out and what a test asserts on. Use
    /// [`Toast::paint`]'s own rect for the frame's pixels, or read them off the
    /// recorded commands.
    ///
    /// `screen` is the box the stack's own node is laid out to, and `advance` and
    /// `line_height` are the measurements
    /// [`Button::content_size`](crate::widgets::button::Button::content_size)
    /// takes. **They must be the ones the paint pass gives**
    /// [`paint_toast`](Toasts::paint_toast), or the stack is laid out in one set
    /// of numbers and drawn in another.
    #[must_use]
    pub fn toast_rect(
        &self,
        index: usize,
        screen: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Option<Rect> {
        self.placement(screen, advance, line_height)
            .into_iter()
            .nth(index)
    }

    /// Returns the draw commands that paint the toast at `index` inside
    /// `screen`, or nothing when there is no such toast.
    ///
    /// **One toast per call**, so a caller's paint pass can put each toast's
    /// commands on that toast's own node — which is what makes the walk over the
    /// stack one walk. The rect comes from the same
    /// `placement` as [`toast_rect`](Toasts::toast_rect), so
    /// the two are one number by construction rather than by agreement.
    ///
    /// `screen`, `advance` and `line_height` are
    /// [`toast_rect`](Toasts::toast_rect)'s three parameters, and the whole of
    /// [`Toast::paint`]'s contract is behind them.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::toast::{Severity, Toasts};
    ///
    /// let mut nodes = Arena::new();
    /// let mut toasts = Toasts::new(&mut nodes);
    /// toasts.show(&mut nodes, "Settings saved", Duration::from_secs(3));
    /// toasts.toast(0).expect("one toast").severity.set(Some(Severity::Info));
    /// for _ in 0..3 {
    ///     let _ = toasts.tick(&mut nodes, Duration::from_millis(100));
    /// }
    ///
    /// let screen = Rect::new(40.0, 30.0, 1000.0, 700.0);
    /// let commands = toasts.paint_toast(0, screen, &|_: char| 7.0, 20.0);
    /// assert_eq!(commands.len(), 4, "shadow, message, surface and the disc");
    /// ```
    #[must_use]
    pub fn paint_toast(
        &self,
        index: usize,
        screen: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Vec<DrawCommand> {
        let Some(toast) = self.toasts.get(index) else {
            return Vec::new();
        };
        let placement = self.placement(screen, advance, line_height);
        let Some(rect) = placement.get(index).copied() else {
            return Vec::new();
        };
        toast.paint(rect, advance, line_height)
    }

    /// Places every live toast inside `screen`, oldest first.
    ///
    /// **One computation, several readers**, for the dialog's reason: the stack
    /// is laid out from every toast's height, so two computations of it are two
    /// opportunities for two toasts to overlap.
    ///
    /// The width is **one number for the whole stack**: a stack of toasts whose
    /// cards were each as wide as their own message would be a staircase rather
    /// than a column, and the widest message would decide the width of all of
    /// them.
    fn placement(
        &self,
        screen: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Vec<Rect> {
        let line_height = line_height.max(0.0);
        // `clamp` and not two bounds in the order they were written: a box
        // narrower than the margin either side would ask for a negative width,
        // and `0.0` is the lower bound.
        let width = (screen.width - TOAST_MARGIN * 2.0).clamp(0.0, TOAST_MAX_WIDTH);
        // Centred horizontally: the eye has already read whatever raised the
        // toast, and a notification in the corner of a screen reads as a
        // different thing — a message from the system rather than from this
        // interface.
        let x = screen.x + (screen.width - width) / 2.0;
        let heights: Vec<f32> = self
            .toasts
            .iter()
            .map(|toast| {
                measure(
                    width,
                    &toast.message.get(),
                    toast.severity.get().is_some(),
                    line_height,
                    advance,
                )
                .height
            })
            .collect();
        let mut rects = vec![Rect::new(x, screen.y, width, 0.0); heights.len()];
        match self.anchor {
            // The stack hangs from the bottom edge and grows upward, so the
            // **newest** toast — the last of them — is the one nearest that edge,
            // and the loop walks the list backwards to place it first.
            Anchor::Bottom => {
                let mut edge = screen.y + screen.height - TOAST_MARGIN;
                for (index, height) in heights.iter().enumerate().rev() {
                    edge -= *height;
                    rects[index] = Rect::new(x, edge, width, *height);
                    edge -= TOAST_GAP;
                }
            }
            // The mirror: from the top edge downward, oldest first, so the newest
            // is still the last of the stack and still nearest the far end.
            Anchor::Top => {
                let mut edge = screen.y + TOAST_MARGIN;
                for (index, height) in heights.iter().enumerate() {
                    rects[index] = Rect::new(x, edge, width, *height);
                    edge += height + TOAST_GAP;
                }
            }
        }
        rects
    }
}

/// One toast's measurements inside a `width`-wide card, and the two numbers the
/// painting needs from it.
///
/// A struct and not six values out of one call, and it is **private** because
/// nothing outlives the call: the host measures every toast once to place the
/// stack and each toast measures itself once to paint, and both get the same
/// numbers because both go through here.
struct Measured {
    /// How tall the card is: the padding and the lines.
    height: f32,
    /// How far right of the padding's edge the text starts: the disc's diameter
    /// and the gap, or zero when there is no disc.
    inset: f32,
    /// How tall the text block is, which is what the disc is centred on.
    text_height: f32,
    /// Whether the message has anything in it.
    ///
    /// An empty string lays out one empty line, so a toast raised with an empty
    /// message would otherwise draw a text command for it: a command that costs a
    /// vertex buffer slot and draws nothing.
    present: bool,
    /// The lines, top to bottom.
    lines: Vec<Line>,
}

/// Measures the toast of `message` inside a card `width` pixels wide, with or
/// without a disc beside it.
///
/// Word wrapping with an ellipsis on whatever does not fit, and a cap of
/// `TOAST_MAX_LINES` lines: a notification is one thing to read.
fn measure(
    width: f32,
    message: &str,
    icon: bool,
    line_height: f32,
    advance: &dyn Fn(char) -> f32,
) -> Measured {
    let inset = if icon {
        ICON_RADIUS * 2.0 + TOAST_GAP
    } else {
        0.0
    };
    let inner = (width - TOAST_PADDING * 2.0 - inset).max(0.0);
    let present = !message.is_empty();
    let layout: TextLayout = layout_text(
        message,
        &LayoutOptions {
            max_width: inner,
            max_height: Some(line_height * count_to_f32(TOAST_MAX_LINES)),
            line_height,
            letter_spacing: 0.0,
            align: TextAlign::Left,
            wrap: WrapMode::Word,
            truncation: Truncation::Ellipsis,
        },
        advance,
    );
    let text_height = if present { layout.total_height } else { 0.0 };
    Measured {
        height: (TOAST_PADDING * 2.0 + text_height).max(0.0),
        inset,
        text_height,
        present,
        lines: layout.lines,
    }
}

/// Returns the shadow's own colour, before its alpha: black, because a white
/// drop shadow is not a shadow.
///
/// **A function and not a constant**, because [`Color::new`] is not a `const fn`
/// and a shadow's colour is asked for once per toast per frame rather than once
/// per program.
///
/// It is a literal rather than a token, and the argument is the dialog's
/// `SHADOW_ALPHA`: the dark theme's [`Background`](ThemeToken::Background) is
/// `18 18 18` and the light theme's is `255 255 255`, so a shadow taken from the
/// token would be black on one theme and white on the other.
fn shadow_colour() -> Color {
    Color::new(0, 0, 0, 255)
}

/// The motion a toast's arrival and departure run on when its caller has said
/// nothing.
///
/// The theme's [`DurationFast`](ThemeToken::DurationFast) with its
/// [`EasingStandard`](ThemeToken::EasingStandard) — which is exactly
/// [`Motion::from_theme`]'s answer, and requirement 4 asks for every one of the
/// four animations over that duration.
///
/// **Copied rather than read**, because the widget has no theme to read it from
/// and a caller themes a stack by handing it [`Motion::from_theme`]. The
/// fallbacks are the values both themes hold anyway, so a caller that never
/// themes a stack gets 150 ms and a curve that is a curve.
/// `the_default_motion_is_the_themes` reads the tokens and holds the two to
/// them.
fn default_motion() -> Motion {
    Motion {
        duration: Duration::from_millis(150),
        easing: Easing::EaseInOut,
    }
}

/// Returns `color` at `alpha` of its own presence, with every channel
/// multiplied by it.
///
/// **This is the premultiplied fade, done by hand**, and it is a workaround for
/// a recorded defect rather than a colour model: the solid pass blends as
/// `GL_ONE, GL_ONE_MINUS_SRC_ALPHA` (`render.rs:2001`) and writes `frag_color`
/// verbatim (`render.rs:179`), so a colour handed to it unpremultiplied
/// composites as `rgb + dst·(1 − a)` — which brightens, and a surface at alpha
/// 240 lands nowhere near 94% over a lighter destination. The defect is
/// recorded, cross-cutting and unfixed
/// (`doc/ui/IMPLEMENTATION_STATE.md` § *The finding that is not this task's: the
/// solid pass does not premultiply*).
///
/// Multiplying all four channels is what makes it correct rather than only
/// plausible: a colour at 50% has an alpha of 50% *and* half its own channels,
/// which is the definition. The dialog's `faded` arrives at the same place by
/// interpolating toward transparent black.
///
/// **When the solid pass is fixed, this goes.** The record names the fix as one
/// place — `quad_color`, which is `render.rs:801` and is cited here so a
/// maintainer following this lands on the function rather than on its caller —
/// and after it this function (and the dialog's `faded`) would multiply by the
/// alpha a second time. What is written here instead is
/// the fix's own shape: drop the call, hand the theme's colours over, and let the
/// shader do it.
///
/// `alpha` is clamped, because a caller that has lost track of the opacity
/// cannot ask for a colour outside the segment.
fn at_alpha(color: Color, alpha: f32) -> Color {
    let alpha = alpha.clamp(0.0, 1.0);
    Color {
        r: level(f32::from(color.r) * alpha),
        g: level(f32::from(color.g) * alpha),
        b: level(f32::from(color.b) * alpha),
        a: level(f32::from(color.a) * alpha),
    }
}

/// Returns `value` as a channel value, rounded and clamped to `0..=255`.
///
/// There is no `From`/`TryFrom` between `f32` and any integer type in std, so
/// this is the one float-to-integer `as` cast in the module, for the same reason
/// and with the same guarantee as the dialog's `level` and
/// [`button`](crate::widgets::button)'s: the cast is saturating (Rust 1.45 and
/// later), so the clamp states the intent — an opacity that has come adrift
/// cannot wrap a channel round to the other end of the range.
fn level(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// Converts a count to the float the geometry arithmetic uses.
///
/// `f32` has no `From<usize>` in std — the `From` impls between integers stop at
/// the 16-bit widths and no float conversion is provided at all — so this is the
/// one place the cast happens, for the reason the dialog's `count_to_f32` and
/// [`keyboard`](crate::widgets::keyboard)'s give. The conversion is well defined
/// for every `usize`: the result rounds to the nearest `f32`.
fn count_to_f32(count: usize) -> f32 {
    count as f32
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// [`dialog`](crate::widgets::dialog)'s own `token_color`, repeated for the same
/// reason `at_alpha` is repeated there: a shared home would be a change to
/// `theme.rs`, which is not this widget's to make.
fn token_color(theme: &Theme, token: ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::Batcher;
    use crate::input::{hit_test, route, InputEvent, InputEventKind};
    use crate::layout::{Constraints, Layout, Offset, Size};
    use crate::render::blur;

    /// A character's advance, in pixels. A whole number, so every width in these
    /// fixtures is one too and every expected rect can be written out rather
    /// than derived from the constants under test.
    const ADVANCE: f32 = 7.0;

    /// The line box every line in these tests is drawn in.
    const LINE_HEIGHT: f32 = 20.0;

    /// The message every fixture is built with: 14 characters, 98 pixels wide,
    /// which is inside the card's 304-pixel inner width and so does not wrap.
    const MESSAGE: &str = "Settings saved";

    /// A second message, so a two-toast stack is two messages rather than one
    /// twice. Thirteen characters, 91 pixels.
    const OTHER: &str = "Connection lost";

    /// The box the stacks below are laid out in.
    ///
    /// **Not at the origin**, deliberately, for the reason `.ai/NEVERAGAIN.md` §
    /// *A rect's origin and a rect's extent are different numbers* gives: a
    /// fixture at `(0, 0)` cannot see a coordinate being read as a size, and
    /// this module computes a card's `x` from the box's **width** and its `y`
    /// from the box's **bottom edge**.
    const SCREEN: Rect = Rect {
        x: 40.0,
        y: 30.0,
        width: 1000.0,
        height: 700.0,
    };

    /// How long a fixture's toasts stay.
    const DURATION: Duration = Duration::from_millis(3000);

    /// The whole of the arrival, which is the theme's `DurationFast`.
    const ARRIVAL: Duration = Duration::from_millis(150);

    /// Where a one-line toast with a disc is placed in [`SCREEN`], written out.
    ///
    /// `1000 - 48` of margin either side leaves 952, capped at [`TOAST_MAX_WIDTH`]'s
    /// 360, and the slack either side of that is `(1000 - 360) / 2 = 320`, so the
    /// card starts at `40 + 320 = 360`. Its height is 32 of padding and one
    /// 20-pixel line box, and the bottom margin puts its top edge at
    /// `30 + 700 - 24 - 52 = 654`.
    const CARD: Rect = Rect {
        x: 360.0,
        y: 654.0,
        width: 360.0,
        height: 52.0,
    };

    /// Where the second toast of a two-toast stack is placed: the first card's
    /// top edge less its height and the 8-pixel gap between two cards.
    const CARD_ABOVE: Rect = Rect {
        x: 360.0,
        y: 594.0,
        width: 360.0,
        height: 52.0,
    };

    /// The card's inner left edge **with** a disc beside the message: the card's
    /// own 16-pixel padding, the disc's 16-pixel diameter and the 8-pixel gap.
    const TEXT_X: f32 = 400.0;

    /// The card's inner left edge with **no** disc beside it.
    const TEXT_X_BARE: f32 = 376.0;

    /// The first line box's top edge in [`CARD`].
    const TEXT_Y: f32 = 670.0;

    /// The disc's centre in [`CARD`]: at the padding plus its own radius across,
    /// and at the padding plus half the text block's height down.
    const DISC: (f32, f32) = (384.0, 680.0);

    /// How many whole arrival spans a fixture toast lives for, used by the test
    /// that watches its node go back to the arena.
    ///
    /// **One arrival plus the duration plus one departure**, in spans of
    /// [`ARRIVAL`]: 1 + `3000 / 150` + 1 = 22. The extra span each side is slack,
    /// because the frame a transition completes on is the frame it is dropped on.
    const TOAST_LIFE_FRAMES: usize = 22;

    /// How far two derived floats may differ in these tests.
    ///
    /// Every expectation here is arithmetic in `f32` — a closed form from
    /// [`Easing::apply`], a sum of sizes — and `f32` has 24 bits of mantissa, so
    /// the last bit is not a promise anybody can make. The dialog states the same
    /// reason for the same constant.
    const EPSILON: f32 = 1e-5;

    /// The advance measurement every fixture passes.
    fn advance(_: char) -> f32 {
        ADVANCE
    }

    /// Returns a stack with one toast, still arriving.
    fn stack(nodes: &mut Arena<WidgetNode>) -> Toasts {
        let mut toasts = Toasts::new(nodes);
        toasts.show(nodes, MESSAGE, DURATION);
        toasts
    }

    /// Returns a stack with one toast that has finished arriving.
    ///
    /// Three whole spans and one span of slack, the same fixture arithmetic
    /// [`Dialog`]'s `finish` uses.
    fn settled(nodes: &mut Arena<WidgetNode>) -> Toasts {
        let mut toasts = stack(nodes);
        finish(nodes, &mut toasts);
        toasts
    }

    /// Runs the stack's arrivals by three whole spans.
    ///
    /// **The arena is a parameter** because [`Toasts::tick`] takes one: it gives
    /// back the nodes of the toasts that have gone, and a fixture that ticks has
    /// to have somewhere to give them back to.
    fn finish(nodes: &mut Arena<WidgetNode>, toasts: &mut Toasts) {
        for _ in 0..3 {
            let _ = toasts.tick(nodes, ARRIVAL);
        }
    }

    /// Writes the toast's own two animated properties where a test wants them,
    /// rather than ticking a clock to get there.
    ///
    /// **The alternative is a fixture that has to run the right number of frames
    /// to reach a half-faded state**, and a test whose expectation is a whole
    /// number of frames is a test that breaks when the motion changes. The
    /// properties are public and the animation writes them, which is what the
    /// dialog's own half-faded fixtures do.
    fn posed(toasts: &Toasts, opacity: f32, slide: f32) {
        let toast = toast_of(toasts, 0);
        toast.opacity.set(opacity);
        toast.slide.set(slide);
    }

    /// Returns the toast at `index` of `toasts`.
    ///
    /// Re-fetched after every `tick`, which is the cost of
    /// [`Toasts::tick`]'s `&mut self`: a borrow of a toast cannot outlive a
    /// mutation of the vector holding it, and the tests below are written the way
    /// a caller has to write them.
    fn toast_of(toasts: &Toasts, index: usize) -> &Toast {
        toasts
            .toast(index)
            .unwrap_or_else(|| panic!("no toast {index} among {} live", toasts.len()))
    }

    /// Returns the first toast's opacity.
    fn opacity(toasts: &Toasts) -> f32 {
        toast_of(toasts, 0).opacity.get()
    }

    /// Returns the first toast's slide.
    fn slide(toasts: &Toasts) -> f32 {
        toast_of(toasts, 0).slide.get()
    }

    /// Returns the first toast's remaining time.
    fn remaining(toasts: &Toasts) -> Duration {
        toast_of(toasts, 0).remaining()
    }

    /// Returns the first toast's elapsed time.
    fn elapsed(toasts: &Toasts) -> Duration {
        toast_of(toasts, 0).elapsed.get()
    }

    /// Returns whether the first toast is showing.
    fn visible(toasts: &Toasts) -> bool {
        toast_of(toasts, 0).visible.get()
    }

    /// Returns the first toast's disc colour, in the dark theme.
    fn severity_colour(toasts: &Toasts) -> Color {
        let toast = toast_of(toasts, 0);
        let Some(severity) = toast.severity.get() else {
            return Color::new(0, 0, 0, 0);
        };
        match severity {
            Severity::Error => toast.error.get(),
            Severity::Warning => toast.warning.get(),
            Severity::Success => toast.success.get(),
            Severity::Info => toast.info.get(),
        }
    }

    /// Returns the first toast's corner radius.
    fn radius(toasts: &Toasts) -> f32 {
        toast_of(toasts, 0).border_radius.get()
    }

    /// Returns the name of each recorded command, in the order it was recorded.
    fn recorded(commands: &[DrawCommand]) -> Vec<&'static str> {
        commands.iter().map(kind_of).collect()
    }

    /// Returns the name of each recorded command, **in the order the renderer
    /// submits it**.
    ///
    /// This is the traversal [`Renderer::end_frame`](crate::render::Renderer)
    /// performs, in the order it performs it: each segment's opaque batches, then
    /// its translucent ones, then the shadow the segment ends with —
    /// `render.rs:1994` to `:2016`. Running the recorded commands through the
    /// **real** [`Batcher`] is what makes this a measurement of where a command
    /// lands rather than a restatement of the order it was written in, and
    /// `.ai/NEVERAGAIN.md` § *A draw-command assertion cannot see where a command
    /// lands* is the entry for what it replaces.
    fn submitted(commands: &[DrawCommand]) -> Vec<&'static str> {
        let mut batcher = Batcher::new();
        for command in commands {
            batcher.add(command.clone());
        }
        let mut order = Vec::new();
        for segment in batcher.submit_order() {
            for batch in &segment.opaque {
                order.extend(batch.commands.iter().map(kind_of));
            }
            for batch in &segment.transparent {
                order.extend(batch.commands.iter().map(kind_of));
            }
            if let Some(batch) = &segment.shadow {
                order.extend(batch.commands.iter().map(kind_of));
            }
        }
        order
    }

    /// Returns the name of `command` for [`recorded`] and [`submitted`].
    fn kind_of(command: &DrawCommand) -> &'static str {
        match command {
            DrawCommand::Shadow { .. } => "shadow",
            DrawCommand::RoundedRect { .. } => "surface",
            DrawCommand::Circle { .. } => "disc",
            DrawCommand::Text { .. } => "text",
            other => panic!("{other:?} is not a command this widget records"),
        }
    }

    /// Returns the surface's rect, radius and colour, or panics.
    fn surface_of(commands: &[DrawCommand]) -> (Rect, f32, Color) {
        commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color,
                } => Some((*rect, *radius, *color)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no surface among {commands:?}"))
    }

    /// Returns the shadow's rect, radius, colour, blur and offset, or panics.
    fn shadow_of(commands: &[DrawCommand]) -> (Rect, f32, Color, f32, (f32, f32)) {
        commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Shadow {
                    rect,
                    radius,
                    color,
                    blur,
                    offset,
                } => Some((*rect, *radius, *color, *blur, *offset)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no shadow among {commands:?}"))
    }

    /// Returns the disc's centre, radius and colour, or panics.
    fn disc_of(commands: &[DrawCommand]) -> ((f32, f32), f32, Color) {
        commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Circle {
                    center,
                    radius,
                    color,
                } => Some((*center, *radius, *color)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no disc among {commands:?}"))
    }

    /// Returns every text run's text, position, colour and size, in order.
    fn texts_of(commands: &[DrawCommand]) -> Vec<(String, f32, f32, Color, f32)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    x,
                    y,
                    text,
                    color,
                    font_size,
                    ..
                } => Some((text.clone(), *x, *y, *color, *font_size)),
                _ => None,
            })
            .collect()
    }

    /// Returns the colour of every command that carries one, in recorded order.
    fn colours_of(commands: &[DrawCommand]) -> Vec<Color> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect { color, .. }
                | DrawCommand::Circle { color, .. }
                | DrawCommand::Text { color, .. }
                | DrawCommand::Shadow { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    }

    /// Returns a tap at `point`.
    fn tap_at(point: Offset) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(point))
    }

    /// Returns a token's colour, or panics: a colour token is a colour.
    fn token_colour(theme: &Theme, token: ThemeToken) -> Color {
        theme
            .get(token)
            .as_color()
            .unwrap_or_else(|| panic!("{token:?} is a colour in both themes"))
    }

    /// Returns a token's number, or panics: a number token is a number.
    fn token_number(theme: &Theme, token: ThemeToken) -> f32 {
        theme
            .get(token)
            .as_number()
            .unwrap_or_else(|| panic!("{token:?} is a number in both themes"))
    }

    /// Asserts two derived floats are within [`EPSILON`] of each other.
    #[track_caller]
    fn close(what: &str, got: f32, want: f32) {
        assert!(
            (got - want).abs() < EPSILON,
            "{what}: got {got}, want {want} (they differ by {})",
            (got - want).abs()
        );
    }

    /// Requirement 1's three properties, on a toast the host raised.
    #[test]
    fn a_toast_holds_the_properties_the_task_gives_it() {
        let mut nodes = Arena::new();
        let toasts = stack(&mut nodes);
        let toast = toast_of(&toasts, 0);

        assert_eq!(toast.message.get(), MESSAGE);
        assert_eq!(toast.duration, DURATION);
        assert!(
            toast.visible.get(),
            "and it starts showing: `show` raised it, so the countdown is the \
             thing that ends it"
        );
        assert!(
            nodes.get(toast.handle()).is_some(),
            "in a node of its own, so the host's walk reaches it"
        );
        assert_eq!(
            nodes.get(toast.handle()).unwrap().parent(),
            Some(toasts.handle()),
            "hung from the host's node, which is what makes one `paint_order` \
             walk cover the whole stack"
        );
    }

    /// **The second-root fact, and the whole of requirement 3's *"does not block
    /// input"*** — the host's node has no parent, so the walk that matters cannot
    /// reach a toast or its host.
    ///
    /// The assertion is built the way the demo builds `modal_chain`:
    /// [`hit_test`] is rooted at a handle **the caller supplies**, so a second root
    /// is unreachable by construction and the property to hold down is the shape
    /// of the tree plus the answer from an unrelated root. There is no flag to
    /// read and no handler to decline an event, which is what makes it a fact
    /// rather than a setting somebody can turn off — and it is the one acceptance
    /// criterion no assertion on recorded commands could see.
    #[test]
    fn a_toast_is_unreachable_from_the_gallerys_own_walk() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        let card = toasts
            .toast_rect(0, SCREEN, &advance, LINE_HEIGHT)
            .expect("one toast");
        let on_it = Offset::new(card.x + 4.0, card.y + 4.0);

        assert_eq!(
            nodes.get(toasts.handle()).unwrap().parent(),
            None,
            "the host's node is a root of its own: a notification overlays the \
             window, so it is not laid out inside the content it covers"
        );

        // The gallery's root: a node of its own over the same screen the toast is
        // drawn in, with nothing of the toast's under it.
        let window = Size::new(SCREEN.width, SCREEN.height);
        let gallery = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::tight(window)),
        );
        Layout::new(&mut nodes).layout(gallery, Constraints::tight(window));

        assert_eq!(
            hit_test(&nodes, gallery, on_it),
            Some(gallery),
            "a point inside the toast's drawn rect is the gallery's own node: the \
             toast is painted there and is nowhere in the tree under it"
        );
        let chain = route(&nodes, gallery, &tap_at(on_it));
        assert!(
            !chain.contains(&toasts.handle()) && !chain.contains(&toast_of(&toasts, 0).handle()),
            "and nothing routed reaches either of them: {chain:?}"
        );
        assert!(
            chain.contains(&gallery),
            "while the walk still finds the node under the toast, which is what \
             *not blocking input* means: {chain:?}"
        );
    }

    #[test]
    fn show_returns_the_handles_node_and_adds_exactly_one_toast() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        assert!(toasts.is_empty());

        let first = toasts.show(&mut nodes, MESSAGE, DURATION);
        assert_eq!(toasts.len(), 1);
        assert_eq!(
            toasts.toast_handle(0),
            Some(first),
            "the toast's own node, which is what the task file's `show` returns"
        );
        assert_eq!(toasts.index_of(first), Some(0));
        assert_eq!(
            toasts.index_of(toasts.handle()),
            None,
            "and the host's own node is not one of the toasts"
        );

        let second = toasts.show(&mut nodes, OTHER, DURATION);
        assert_eq!(toasts.len(), 2, "a second toast does not replace the first");
        assert_eq!(toasts.index_of(second), Some(1), "and it is the newer one");
        assert_ne!(first, second, "with a node of its own each");
        assert_eq!(message_at(&toasts, 1), OTHER, "carrying its own message");
        assert_eq!(
            toasts.toast_handle(2),
            None,
            "and no third in either accessor"
        );
    }

    /// Requirement 4's arrival, at the frame it is raised, half way and arrived.
    ///
    /// **Half of `DurationFast` is 75 ms of a 150 ms span**, and
    /// [`Easing::EaseInOut`] at `t = 0.5` is `1 − 2(1 − 0.5)² = 0.5` — the one
    /// place the standard curve has its midpoint exactly in the middle. The slide
    /// is the same fraction of the way from [`SLIDE_OFFSET`] to its place, so half
    /// way is ten pixels of twenty.
    #[test]
    fn a_raised_toast_is_visible_and_arriving_and_is_half_way_at_seventy_five_ms() {
        let mut nodes = Arena::new();
        let mut toasts = stack(&mut nodes);

        close(
            "nothing of it on the frame it is raised",
            opacity(&toasts),
            0.0,
        );
        close(
            "and it is a whole slide below its place",
            slide(&toasts),
            20.0,
        );
        assert!(
            toast_of(&toasts, 0).is_animating(),
            "which the clock says, so a caller repaints on the first frame"
        );

        let _ = toasts.tick(&mut nodes, Duration::from_millis(75));
        close("the opacity half way", opacity(&toasts), 0.5);
        close("and the slide half way back", slide(&toasts), 10.0);
        assert!(visible(&toasts));

        finish(&mut nodes, &mut toasts);
        close("and the arrival arrives", opacity(&toasts), 1.0);
        close("with the slide at its place", slide(&toasts), 0.0);
        assert!(
            !toast_of(&toasts, 0).is_animating(),
            "and then nothing is running, so a caller that repaints only on \
             `tick` stops"
        );
    }

    /// The motion requirement 4's four animations run at, read from the tokens.
    #[test]
    fn the_motion_is_the_themes_fast_duration_and_its_standard_curve() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);

        assert_eq!(
            toasts.motion(),
            Motion::from_theme(&Theme::dark()),
            "the default is exactly what `Motion::from_theme` reads, so a caller \
             that themes the stack says nothing and gets the same answer"
        );

        toasts.set_motion(Motion {
            duration: Duration::from_millis(40),
            easing: Easing::Linear,
        });
        assert_eq!(toasts.motion().duration, Duration::from_millis(40));
        toasts.show(&mut nodes, MESSAGE, DURATION);
        assert_eq!(
            toast_of(&toasts, 0).motion(),
            toasts.motion(),
            "and a toast raised after the change arrives over it too"
        );
    }

    /// Requirement 3's *"auto-dismisses after duration"*, and the two numbers it
    /// leaves on.
    ///
    /// **The countdown starts when the toast is raised, so the arrival counts
    /// toward it**: a settled fixture has three whole arrival spans on its clock
    /// and every expectation below is that 450 ms off three seconds rather than
    /// off nothing. A countdown that began when the arrival finished would make
    /// the time on screen depend on how long the arrival took, which is a number
    /// the theme decides rather than the widget.
    #[test]
    fn a_toast_dismisses_itself_once_its_duration_has_run_out() {
        let mut nodes = Arena::new();
        let mut toasts = settled(&mut nodes);
        let arrival_time = ARRIVAL * 3;
        assert_eq!(
            elapsed(&toasts),
            arrival_time,
            "the arrival counts toward the countdown"
        );
        assert_eq!(
            remaining(&toasts),
            DURATION - arrival_time,
            "so a settled toast has the rest of its duration and not all of it"
        );

        // Three deltas of a second rather than one of the remainder: an
        // accumulator and a clock both satisfy a single tick.
        let _ = toasts.tick(&mut nodes, Duration::from_millis(1000));
        assert_eq!(
            remaining(&toasts),
            DURATION - arrival_time - Duration::from_millis(1000)
        );
        assert!(visible(&toasts), "still there after a second of the rest");
        let _ = toasts.tick(&mut nodes, Duration::from_millis(1000));
        assert_eq!(
            remaining(&toasts),
            DURATION - arrival_time - Duration::from_millis(2000)
        );
        assert!(visible(&toasts), "and after two");

        let _ = toasts.tick(&mut nodes, Duration::from_millis(1000));
        assert_eq!(remaining(&toasts), Duration::ZERO);
        assert!(
            !visible(&toasts),
            "and it starts to leave the moment the countdown ends"
        );
        assert!(
            toast_of(&toasts, 0).is_drawn(),
            "while it is still on the screen, which is the difference between \
             `visible` and `is_drawn` and the same one the dialog draws"
        );
        close(
            "and the departure starts from the opacity it was at",
            opacity(&toasts),
            1.0,
        );
        assert!(
            !toasts
                .paint_toast(0, SCREEN, &advance, LINE_HEIGHT)
                .is_empty(),
            "so it is still on the screen, fading"
        );
    }

    /// The same countdown in many small deltas: an accumulator and not a clock.
    ///
    /// **299 frames of 10 ms against a three-second toast**, so the last of them
    /// is a hundredth of a second short — which is the boundary a countdown that
    /// compared against a clock rather than accumulating would get wrong, and the
    /// reason this fixture ticks three hundred times instead of three.
    #[test]
    fn the_countdown_is_accumulated_from_the_deltas_it_is_given() {
        let mut nodes = Arena::new();
        let mut toasts = stack(&mut nodes);
        assert_eq!(
            elapsed(&toasts),
            Duration::ZERO,
            "a toast starts with none of its duration behind it"
        );

        for _ in 0..299 {
            let _ = toasts.tick(&mut nodes, Duration::from_millis(10));
        }
        assert_eq!(elapsed(&toasts), Duration::from_millis(2990), "299 of ten");
        assert_eq!(remaining(&toasts), Duration::from_millis(10));
        assert!(
            visible(&toasts),
            "a hundredth of a second early is still there"
        );

        let _ = toasts.tick(&mut nodes, Duration::from_millis(10));
        assert!(
            !visible(&toasts),
            "and the three hundredth frame is the one"
        );
        assert_eq!(elapsed(&toasts), DURATION, "at exactly its own duration");
    }

    /// [`Toasts::tick`]'s contract: true on every frame something is drawn
    /// differently, and false on the frames in between.
    ///
    /// The countdown is the reason this is a test: a toast that stays up for three
    /// seconds has a hundred and eighty frames where its remaining time changed
    /// and **nothing was drawn differently**, and a tick that answered `true` for
    /// those would have a caller repainting a stationary toast sixty times a
    /// second. The frames are counted here rather than asserted one by one so
    /// the numbers are the closed form — `ARRIVAL / 10 ms` frames either side.
    #[test]
    fn tick_is_true_while_something_moves_and_false_while_it_does_not() {
        let mut nodes = Arena::new();
        let mut toasts = stack(&mut nodes);

        let mut moving = 0;
        while toasts.tick(&mut nodes, Duration::from_millis(10)) {
            moving += 1;
        }
        assert_eq!(
            moving * 10,
            ARRIVAL.as_millis(),
            "the arrival reports a frame for each of its {ARRIVAL:?} of ten \
             milliseconds, and not one more"
        );
        assert!(
            !toasts.tick(&mut nodes, Duration::from_millis(10)),
            "and a frame later, with the toast standing still and the countdown \
             two hundred and ninety frames from ending, nothing has moved"
        );

        assert!(
            toasts.tick(&mut nodes, DURATION),
            "the frame the countdown ends on is a frame something moved"
        );
        assert!(!visible(&toasts), "and it left on it");
        let mut leaving = 0;
        while toasts.tick(&mut nodes, Duration::from_millis(10)) {
            leaving += 1;
        }
        assert_eq!(
            leaving * 10,
            ARRIVAL.as_millis(),
            "and the departure reports its own {ARRIVAL:?} of frames"
        );
        assert!(toasts.is_empty(), "after which the host drops the toast");
        assert!(
            !toasts.tick(&mut nodes, Duration::from_millis(10)),
            "and a stack with no toast in it reports nothing"
        );
    }

    /// **A toast interrupted in its arrival**, which is the case
    /// [`Toast::dismiss`]'s doc claims a behaviour for and which nothing held
    /// down until now.
    ///
    /// **The numbers are the closed form.** Half of the 150 ms arrival is 75 ms,
    /// and `EaseInOut` at `t = 0.5` is 0.5, so the toast is at opacity 0.5 and a
    /// slide of 10. The departure then animates **from those numbers** toward 0
    /// and [`SLIDE_OFFSET`], so at its own midpoint it is at
    /// `0.5 + (0 − 0.5) × 0.5 = 0.25` and `10 + (20 − 10) × 0.5 = 15`.
    ///
    /// **The control is the same two numbers without the interruption**: a toast
    /// dismissed from rest would be at 0.5 and 10 at its midpoint, so 0.25 and 15
    /// are the interruption and not the curve.
    ///
    /// **`clock.clear()` is defensive here, and this test does not depend on it.**
    /// With the line removed the clock holds four animations instead of two, and
    /// `AnimationClock::add` appends while `tick` writes in vector order, so the
    /// arriving pair is written *after* the interrupted pair and every number here
    /// is unchanged. The line is the contract
    /// [`AnimationClock::clear`](crate::animation::AnimationClock::clear) states
    /// and this widget honours it; what a caller can see is the behaviour, and
    /// this is what holds it.
    #[test]
    fn a_toast_sent_away_half_way_through_its_arrival_starts_from_where_it_was() {
        let mut nodes = Arena::new();
        let mut toasts = stack(&mut nodes);

        let _ = toasts.tick(&mut nodes, Duration::from_millis(75));
        close("the arrival's midpoint", opacity(&toasts), 0.5);
        close("and its slide", slide(&toasts), 10.0);

        assert!(
            toast_of(&toasts, 0).dismiss(),
            "and it is sent away from there"
        );
        let _ = toasts.tick(&mut nodes, Duration::from_millis(75));
        close(
            "the departure's own midpoint, interpolated from 0.5 rather than 1.0",
            opacity(&toasts),
            0.25,
        );
        close(
            "and its slide, from 10 rather than 20",
            slide(&toasts),
            15.0,
        );

        // The control, written as the interrupted value the same midpoint would
        // have produced without it.
        assert_ne!(
            (opacity(&toasts), slide(&toasts)),
            (0.5, 10.0),
            "a toast dismissed from rest would be at 0.5 and 10 here, so these \
             numbers are the interruption and not the curve"
        );
    }

    /// **A toast brought back half way out**, which is
    /// [`Toast::present`]'s doc's claim and the direction the last test does not
    /// cover.
    ///
    /// The same closed form, run forwards then backwards: 0.25 and 15 after the
    /// interruption above, an arrival animating **from those numbers**, and at its
    /// own 75 ms `0.25 + (1 − 0.25) × 0.5 = 0.625` and `15 + (0 − 15) × 0.5 = 7.5`.
    /// The control is the same two numbers for an arrival that started from
    /// nothing: 0.5 and 10.
    #[test]
    fn a_toast_brought_back_half_way_out_comes_from_half_way() {
        let mut nodes = Arena::new();
        let mut toasts = stack(&mut nodes);
        let _ = toasts.tick(&mut nodes, Duration::from_millis(75));
        let _ = toast_of(&toasts, 0).dismiss();
        let _ = toasts.tick(&mut nodes, Duration::from_millis(75));
        close("half way out", opacity(&toasts), 0.25);
        close("and half way down", slide(&toasts), 15.0);

        assert!(toast_of(&toasts, 0).present(), "and it is brought back");
        let _ = toasts.tick(&mut nodes, Duration::from_millis(75));
        close(
            "its arrival's midpoint, interpolated from 0.25 rather than 0.0",
            opacity(&toasts),
            0.625,
        );
        close("and its slide, from 15 rather than 20", slide(&toasts), 7.5);
        assert!(
            visible(&toasts),
            "and it is showing again, which is the other half of the report"
        );
        assert_eq!(
            (
                toast_of(&toasts, 0).is_animating(),
                toast_of(&toasts, 0).is_drawn()
            ),
            (true, true),
            "with its arrival running"
        );
    }

    /// **The node goes back to the arena**, which is what
    /// [`Toasts::tick`]'s arena argument is for and what its doc used to claim
    /// for a toast's shadows and nodes both, wrongly.
    ///
    /// The count is the arena's own [`Arena::len`], so this is a count through
    /// the public API with a control beside it: one before the toast is raised,
    /// two while it is up, and one again once it has gone.
    #[test]
    fn a_finished_toast_gives_its_node_back_to_the_arena() {
        let mut nodes = Arena::new();
        assert_eq!(nodes.len(), 0, "nothing to begin with");

        let mut toasts = Toasts::new(&mut nodes);
        let host = toasts.handle();
        assert_eq!(nodes.len(), 1, "the host's own node is in there");

        let handle = toasts.show(&mut nodes, MESSAGE, DURATION);
        assert_eq!(nodes.len(), 2, "and the toast's beside it");
        assert_eq!(
            nodes.get(host).map(|node| node.children().to_vec()),
            Some(vec![handle]),
            "under the host, which is what one paint-order walk covers"
        );

        // The whole of its life, through the real tick: the countdown, the
        // departure, and the frame the departure completes on.
        for _ in 0..(1 + TOAST_LIFE_FRAMES) {
            let _ = toasts.tick(&mut nodes, ARRIVAL);
        }
        assert!(toasts.is_empty(), "the toast is gone from the host");
        assert_eq!(
            nodes.len(),
            1,
            "and the arena holds the host's node alone again"
        );
        assert!(
            nodes.get(handle).is_none(),
            "the toast's own handle **stops resolving**, because `Arena::remove` \
             bumps the slot's generation — so a caller still holding it skips it \
             rather than being pointed at whatever takes the slot"
        );
        assert_eq!(
            nodes.get(host).map(|node| node.children().len()),
            Some(0),
            "and the host's child list is empty, because it is detached before \
             it is removed rather than left pointing at a slot that is free"
        );
    }

    /// The host's half of the lifecycle: a toast that has finished leaving is
    /// dropped, and one that has not is kept.
    #[test]
    fn the_host_drops_a_toast_once_its_departure_has_run() {
        let mut nodes = Arena::new();
        let mut toasts = settled(&mut nodes);

        // Sent away by hand rather than by the countdown, so the removal is
        // demonstrably not the countdown's doing.
        assert!(toast_of(&toasts, 0).dismiss());
        assert!(
            !toast_of(&toasts, 0).dismiss(),
            "and dismissing a toast on its way out changes nothing"
        );
        assert_eq!(toasts.len(), 1, "a toast on its way out is still live");
        let _ = toasts.tick(&mut nodes, ARRIVAL / 2);
        assert_eq!(
            toasts.len(),
            1,
            "and it is dropped only once the departure has run: a toast removed \
             on the first frame of its fade would pop off the screen"
        );
        let _ = toasts.tick(&mut nodes, ARRIVAL);
        assert_eq!(toasts.len(), 0, "which is now");
        assert!(toasts
            .paint_toast(0, SCREEN, &advance, LINE_HEIGHT)
            .is_empty());
    }

    /// **The command order, measured where the commands land.**
    ///
    /// This is the requirement no assertion on the recorded list can see, and it
    /// is written against the real [`Batcher`] for that reason: the translucent
    /// group is submitted reversed and a shadow is composited after everything
    /// its own segment recorded, so a list that reads `shadow, text, surface,
    /// disc` reaches the screen as something else entirely.
    #[test]
    fn the_shadow_lands_behind_the_surface_and_the_text_lands_on_it_in_submission() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        toast_of(&toasts, 0).severity.set(Some(Severity::Info));
        let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);

        assert_eq!(
            recorded(&commands),
            vec!["shadow", "text", "surface", "disc"],
            "the recorded order: the shadow alone in its own segment, then the \
             text the reversed group needs recorded before the surface, and the \
             disc after the surface it shares a batch with"
        );
        assert_eq!(
            submitted(&commands),
            vec!["shadow", "surface", "disc", "text"],
            "**and the submitted order, which is the one that reaches the \
             screen**: the shadow behind the card, the card, the disc on it, and \
             the message on top of both"
        );

        // The control beside it: a toast with no disc still submits its surface
        // before its message, so the assertion above is about the order and not
        // about there being four commands.
        toast_of(&toasts, 0).severity.set(None);
        let bare = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        assert_eq!(
            submitted(&bare),
            vec!["shadow", "surface", "text"],
            "with three commands and the same order"
        );
    }

    /// **Every colour a toast records is translucent**, and that is what keeps
    /// them in one batch group.
    ///
    /// [`SURFACE_OPACITY`]'s own doc says the recording order above stops working
    /// the moment one of them reaches `alpha 255`, so this is the assertion that
    /// holds the two together. It is also the only check on a change to
    /// `SURFACE_OPACITY`, which nothing else would notice.
    #[test]
    fn every_colour_a_toast_records_is_in_the_translucent_group() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        toast_of(&toasts, 0).severity.set(Some(Severity::Error));

        for opacity in [1.0, 0.5, 0.01] {
            posed(&toasts, opacity, 0.0);
            let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
            assert_eq!(
                colours_of(&commands).len(),
                4,
                "opacity {opacity}: the shadow, the message, the surface and the \
                 disc"
            );
            for colour in colours_of(&commands) {
                assert!(
                    colour.a < 255,
                    "opacity {opacity}: {colour:?} is opaque, so it is submitted \
                     ahead of the translucent card it is drawn on"
                );
            }
        }
    }

    /// Requirement 2's *"surface color with slight transparency"*, and the
    /// premultiplication that makes it mean what it says.
    ///
    /// The numbers are derived by hand rather than computed by the code: the dark
    /// theme's `Surface` is `30 30 30`, six percent transparent means 94% of each
    /// channel (`30 × 0.94 = 28.2`, rounded to 28) and an alpha of `255 × 0.94 =
    /// 239.7`, rounded to **240**. **A surface handed to this pipeline
    /// unpremultiplied would composite as `28 + dst × 0.06` and brighten**, which
    /// is the recorded defect this premultiplies around.
    #[test]
    fn the_surface_is_the_theme_surface_at_slight_transparency_and_premultiplied() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        toasts.set_palette(Palette::from_theme(&Theme::dark()));
        toasts.show(&mut nodes, MESSAGE, DURATION);
        finish(&mut nodes, &mut toasts);

        let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        let (rect, radius, surface) = surface_of(&commands);
        assert_eq!(rect, CARD, "in the card's own place in the box");
        assert_eq!(radius, 8.0, "BorderRadiusMd");
        assert_eq!(
            surface,
            Color::new(28, 28, 28, 240),
            "the dark theme's Surface at 94%, **premultiplied**: 30 × 0.94 is 28 \
             and 255 × 0.94 is 240, where an unpremultiplied 240 would brighten \
             over anything lighter than 28"
        );
        assert_eq!(surface.a, 240, "which is 94% of 255");
        assert_eq!(
            surface.r,
            level(30.0 * SURFACE_OPACITY),
            "and the channel is that fraction of the token rather than the token"
        );
    }

    /// Requirement 2's optional icon: no severity means no disc, and the three
    /// tokens plus the accent are the four that are drawn.
    ///
    /// **The four colours are the dark theme's own tokens multiplied by 0.94**,
    /// each written out: `207 × 0.94 = 194.6` is 195, `102 × 0.94 = 95.9` is 96,
    /// `121 × 0.94 = 113.7` is 114, and so on. A palette that read the wrong token
    /// would be a disc in a colour nobody asked for with every other assertion in
    /// this module still green.
    #[test]
    fn each_severity_records_its_disc_in_its_own_theme_colour_and_none_records_none() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        toasts.set_palette(Palette::from_theme(&Theme::dark()));
        toasts.show(&mut nodes, MESSAGE, DURATION);
        finish(&mut nodes, &mut toasts);

        // The control: with no severity at all there is no disc, which is what
        // makes the four assertions below about a colour rather than about a shape
        // that is always there.
        let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        assert_eq!(
            recorded(&commands),
            vec!["shadow", "text", "surface"],
            "an undecorated toast records three commands and no disc"
        );
        assert_eq!(
            texts_of(&commands)[0].1,
            TEXT_X_BARE,
            "and its message starts at the card's own padding: 360 + 16"
        );

        for (severity, token, colour) in [
            (
                Severity::Error,
                ThemeToken::Error,
                Color::new(195, 96, 114, 240),
            ),
            (
                Severity::Warning,
                ThemeToken::Warning,
                Color::new(240, 172, 72, 240),
            ),
            (
                Severity::Success,
                ThemeToken::Success,
                Color::new(96, 176, 100, 240),
            ),
            (
                Severity::Info,
                ThemeToken::Primary,
                Color::new(176, 126, 237, 240),
            ),
        ] {
            toast_of(&toasts, 0).severity.set(Some(severity));
            let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
            let (centre, radius, drawn) = disc_of(&commands);
            assert_eq!(
                (centre, radius),
                (DISC, 8.0),
                "{severity:?}: at the card's left and beside the middle of the \
                 message, at 16 pixels across and the size of the module's radius"
            );
            assert_eq!(
                drawn, colour,
                "{severity:?}: the dark theme's {token:?} at 94%, premultiplied"
            );
            assert_eq!(
                severity_colour(&toasts),
                token_colour(&Theme::dark(), token),
                "{severity:?}: and the property the disc is drawn from **is** that \
                 token, which is what stops the drift test below being the only \
                 thing that knows"
            );
            assert_eq!(
                texts_of(&commands)[0].1,
                TEXT_X,
                "{severity:?}: and the message moves clear of the disc by its \
                 diameter and the gap"
            );
        }
    }

    /// The two properties that must move together: the toast's own opacity and
    /// the alpha every one of its colours carries.
    ///
    /// **The assertion is on the alpha and not on the presence of a command**,
    /// because a card at 94% and a card at 100% record the same four commands in
    /// the same places. `.ai/NEVERAGAIN.md` § *A strength clamped to 0..=1, used
    /// directly as an effect's size* is the class: forty-six tests once passed
    /// over a button whose press overlay was a fully opaque black box.
    #[test]
    fn the_message_the_disc_and_the_surface_fade_with_the_toast() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        toast_of(&toasts, 0).severity.set(Some(Severity::Error));

        // Half the toast's own opacity is 47% of presence, so every channel is
        // `× 0.47`: `255 × 0.47 = 119.85` rounds to **120**, and the shadow's half
        // of that is `255 × 0.235 = 59.925`, rounds to **60**.
        posed(&toasts, 0.5, 0.0);
        let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        assert_eq!(
            texts_of(&commands)[0].3.a,
            120,
            "the message at half strength"
        );
        assert_eq!(disc_of(&commands).2.a, 120, "and the disc with it");
        assert_eq!(surface_of(&commands).2.a, 120, "and the card under it");
        assert_eq!(
            shadow_of(&commands).2.a,
            60,
            "and the shadow at half of that"
        );

        // And the other end of the segment: nothing at all rather than something
        // invisible.
        posed(&toasts, 0.0, 0.0);
        assert!(
            toasts
                .paint_toast(0, SCREEN, &advance, LINE_HEIGHT)
                .is_empty(),
            "a toast at no opacity records nothing, not four transparent commands"
        );
    }

    /// Requirement 4's slide, as pixels rather than as a property.
    ///
    /// The toast is posed at its resting opacity with the slide at
    /// [`SLIDE_OFFSET`], and every command's rect is compared against the same
    /// frame's at zero: the whole card moves — surface, message, disc and shadow
    /// together — by twenty pixels down.
    #[test]
    fn the_slide_moves_the_whole_toast_down_by_twenty_pixels() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        toast_of(&toasts, 0).severity.set(Some(Severity::Warning));

        posed(&toasts, 1.0, 0.0);
        let at_rest = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        posed(&toasts, 1.0, 20.0);
        let slid = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);

        assert_eq!(
            recorded(&at_rest),
            recorded(&slid),
            "the same four commands either way: the slide moves them rather than \
             adding or dropping one"
        );
        assert_eq!(
            surface_of(&slid).0,
            Rect::new(CARD.x, CARD.y + 20.0, CARD.width, CARD.height),
            "the card is in its own place twenty pixels lower, and nowhere else"
        );
        close(
            "and the surface's top edge moved by exactly that",
            surface_of(&slid).0.y - surface_of(&at_rest).0.y,
            20.0,
        );
        let rest_text = texts_of(&at_rest)[0].clone();
        let slid_text = texts_of(&slid)[0].clone();
        close(
            "and the message's line box with it",
            slid_text.2 - rest_text.2,
            20.0,
        );
        assert_eq!(
            (slid_text.1, slid_text.3, slid_text.4),
            (rest_text.1, rest_text.3, rest_text.4),
            "and it keeps its x, its colour and its size: only y moves"
        );
        assert_eq!(
            disc_of(&slid).0 .1 - disc_of(&at_rest).0 .1,
            20.0,
            "and the disc goes with it, so it never slides out from under the \
             message"
        );
        assert_eq!(
            shadow_of(&slid).0.y - shadow_of(&at_rest).0.y,
            20.0,
            "and so does the shadow, which is the toast's own rather than a thing \
             the screen draws behind it"
        );
    }

    /// The shadow's own shape: the card's rect and radius, blurred and offset,
    /// and black at half of the card's own presence.
    #[test]
    fn the_shadow_is_the_cards_own_shape_blurred_and_offset_below_it() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        toast_of(&toasts, 0).border_radius.set(20.0);
        assert_eq!(radius(&toasts), 20.0);
        let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        let (rect, shadow_radius, colour, blur_sigma, offset) = shadow_of(&commands);
        let (card, card_radius, _) = surface_of(&commands);

        assert_eq!(rect, card, "the same rect the card is drawn in");
        assert_eq!(
            shadow_radius, card_radius,
            "and the same radius, from one property"
        );
        assert_eq!(card_radius, 20.0, "so both moved together");
        assert_eq!(blur_sigma, 8.0, "the Gaussian's standard deviation");
        assert_eq!(offset, (0.0, 8.0), "falling below the card");
        assert_eq!(
            colour,
            Color::new(0, 0, 0, 120),
            "and black at half of the card's 94%: 255 × 0.47 is 120"
        );
        assert_eq!(
            blur::taps_for(blur_sigma),
            4,
            "which the cap turns into four taps either side however wide the \
             sigma is asked for"
        );
    }

    /// Requirement 2's message, on the recorded commands: the theme's body size,
    /// one run per line, on the card's first line box.
    #[test]
    fn the_message_is_recorded_at_the_theme_body_size_on_the_first_line_box() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        let texts = texts_of(&commands);

        assert_eq!(texts.len(), 1, "one line for a message that fits");
        assert_eq!(texts[0].0, MESSAGE, "carrying the message");
        assert_eq!(
            (texts[0].1, texts[0].2),
            (TEXT_X_BARE, TEXT_Y),
            "at the card's padding across and one line box down: 360 + 16 and \
             654 + 16"
        );
        assert_eq!(
            texts[0].4, 14.0,
            "at the theme's FontSizeMd, which is what a notification is: a \
             message, not a heading"
        );
    }

    /// An empty message: a card and no text command.
    #[test]
    fn an_empty_message_draws_a_surface_and_no_text() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        toasts.show(&mut nodes, "", DURATION);
        finish(&mut nodes, &mut toasts);

        let commands = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        assert_eq!(
            recorded(&commands),
            vec!["shadow", "surface"],
            "no run for a run with nothing in it: a text command that draws \
             nothing still costs a vertex buffer slot"
        );
        assert_eq!(
            toasts.toast_rect(0, SCREEN, &advance, LINE_HEIGHT),
            Some(Rect::new(CARD.x, 674.0, CARD.width, 32.0)),
            "and the card is its 32 of padding and nothing else — 706 − 32 — so \
             an empty toast is not a card the height of a line"
        );
    }

    /// A message too wide for the card: wrapped, and then cut at
    /// [`TOAST_MAX_LINES`].
    ///
    /// **The inner width is 304 with a disc beside it**, and a four-character word
    /// is 28 pixels with a 7-pixel space between, so a line holds eight such words
    /// (`8 × 28 + 7 × 7 = 273`) and not nine (`9 × 28 + 8 × 7 = 308`).
    #[test]
    fn a_message_wider_than_the_card_wraps_and_is_cut_at_three_lines() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        // **With a disc beside it**, which is what makes the inner width 304
        // rather than 328 and the line hold eight words rather than nine.
        toast_of(&toasts, 0).severity.set(Some(Severity::Info));
        // Four characters each, so a word is 28 pixels wide and a space is 7.
        let words: Vec<String> = (0..30).map(|index| format!("w{index:03}")).collect();

        toast_of(&toasts, 0).message.set(words[..10].join(" "));
        let two_lines = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        assert_eq!(
            texts_of(&two_lines)
                .iter()
                .map(|(text, ..)| text.as_str())
                .collect::<Vec<_>>(),
            vec!["w000 w001 w002 w003 w004 w005 w006 w007", "w008 w009"],
            "eight words to a line, because the ninth would need 308 pixels of \
             the card's 304"
        );
        assert_eq!(
            toasts.toast_rect(0, SCREEN, &advance, LINE_HEIGHT),
            Some(Rect::new(CARD.x, 634.0, CARD.width, 72.0)),
            "and the card is two lines tall: 32 of padding and 40 of line boxes, \
             so 706 − 72 puts its top edge at 634"
        );
        let lines = texts_of(&two_lines);
        assert_eq!(
            lines[0].2, 650.0,
            "the first line box is one padding below that, at 634 + 16"
        );
        assert_eq!(
            lines[1].2,
            lines[0].2 + LINE_HEIGHT,
            "and the second one line box lower: 670"
        );

        // Thirty words is four lines of eight, and the cap keeps three.
        toast_of(&toasts, 0).message.set(words.join(" "));
        let three_lines = toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT);
        assert_eq!(
            texts_of(&three_lines).len(),
            3,
            "four lines of eight words do not fit three line boxes, and a \
             notification is one thing to read"
        );
        assert_eq!(
            toasts.toast_rect(0, SCREEN, &advance, LINE_HEIGHT),
            Some(Rect::new(CARD.x, 614.0, CARD.width, 92.0)),
            "so the card is 32 of padding and three 20-pixel line boxes: 92, and \
             706 − 92 puts its top edge at 614"
        );
    }

    /// Requirement 3's *"Multiple toasts stack vertically (newest at bottom)"*,
    /// from literals rather than from the constants that produced them.
    #[test]
    fn the_toasts_stack_vertically_with_the_newest_lowest() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        for message in [MESSAGE, OTHER] {
            toasts.show(&mut nodes, message, DURATION);
        }
        assert_eq!(toasts.len(), 2);
        finish(&mut nodes, &mut toasts);

        let older = toasts
            .toast_rect(0, SCREEN, &advance, LINE_HEIGHT)
            .expect("two toasts");
        let newer = toasts
            .toast_rect(1, SCREEN, &advance, LINE_HEIGHT)
            .expect("two toasts");
        assert_eq!(
            older, CARD_ABOVE,
            "the older toast is the one further from the bottom edge: 654 − 52 − 8"
        );
        assert_eq!(newer, CARD, "and the newest is the one nearest it");
        assert!(
            older.y + older.height < newer.y,
            "and the two do not overlap: a stack whose cards touch is one card"
        );
        assert!(
            newer.y + newer.height <= SCREEN.y + SCREEN.height - 24.0,
            "and the newest keeps the margin from the bottom edge: 706 − 52"
        );
        assert_eq!(
            toasts.toast_rect(2, SCREEN, &advance, LINE_HEIGHT),
            None,
            "and there is no third toast to place"
        );
    }

    /// Requirement 2's *"or top — configurable"*, as a placement rather than as
    /// a setter's return value.
    #[test]
    fn a_top_anchored_stack_grows_downward_from_the_top_edge() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        assert_eq!(
            toasts.anchor(),
            Anchor::Bottom,
            "which is the task's default"
        );
        for message in [MESSAGE, OTHER] {
            toasts.show(&mut nodes, message, DURATION);
        }
        finish(&mut nodes, &mut toasts);

        assert!(toasts.set_anchor(Anchor::Top), "which moved");
        assert!(
            !toasts.set_anchor(Anchor::Top),
            "and setting it again moved nothing, which is what `set_anchor` reports"
        );
        assert_eq!(
            toasts.toast_rect(0, SCREEN, &advance, LINE_HEIGHT),
            Some(Rect::new(CARD.x, 54.0, CARD.width, CARD.height)),
            "the older toast is nearest the top edge, one margin from it: 30 + 24"
        );
        assert_eq!(
            toasts.toast_rect(1, SCREEN, &advance, LINE_HEIGHT),
            Some(Rect::new(CARD.x, 114.0, CARD.width, CARD.height)),
            "and the newest is below it by 52 of card and the 8-pixel gap, so a \
             top-anchored stack still reads from its oldest end"
        );
    }

    /// The stack's width is one number for every toast, and it is the box's.
    #[test]
    fn the_stack_is_one_width_for_every_toast_and_never_wider_than_its_maximum() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        let long = "a".repeat(200);
        toasts.show(&mut nodes, OTHER, DURATION);
        toasts.show(&mut nodes, &long, DURATION);
        finish(&mut nodes, &mut toasts);

        let short = toasts
            .toast_rect(0, SCREEN, &advance, LINE_HEIGHT)
            .expect("two toasts");
        let wide = toasts
            .toast_rect(1, SCREEN, &advance, LINE_HEIGHT)
            .expect("two toasts");
        assert_eq!(
            short.width, 360.0,
            "TOAST_MAX_WIDTH, not the message's own width"
        );
        assert_eq!(
            short.width, wide.width,
            "and the same width as a message three times its length: a stack of \
             cards sized to their own messages is a staircase"
        );
        assert_eq!(
            (short.x, wide.x),
            (360.0, 360.0),
            "both centred on the same axis"
        );

        // And a box narrower than the maximum is the box's width less the two
        // margins: the other half of the rule. **Its own stack of one**, because
        // the vertical answer depends on how many cards are above it and this is
        // about the width.
        let narrow = Rect::new(0.0, 0.0, 300.0, 700.0);
        assert_eq!(
            stack(&mut nodes).toast_rect(0, narrow, &advance, LINE_HEIGHT),
            Some(Rect::new(24.0, 624.0, 252.0, CARD.height)),
            "252 of 300 less the two 24-pixel margins, with (300 − 252) / 2 = 24 \
             of slack beside it and 0 + 700 − 24 − 52 down"
        );
    }

    /// The palette reaches the toasts already on screen, which is the half a
    /// theme switch needs.
    #[test]
    fn set_palette_reaches_the_toasts_already_on_screen_and_the_ones_to_come() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        toasts.set_palette(Palette::from_theme(&Theme::dark()));
        toasts.show(&mut nodes, MESSAGE, DURATION);
        finish(&mut nodes, &mut toasts);
        assert_eq!(
            surface_of(&toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT)).2,
            Color::new(28, 28, 28, 240),
            "the dark theme's Surface at 94%"
        );

        toasts.set_palette(Palette::from_theme(&Theme::light()));
        assert_eq!(
            toasts.palette(),
            Palette::from_theme(&Theme::light()),
            "the stack holds the new palette for the toasts to come"
        );
        assert_eq!(
            surface_of(&toasts.paint_toast(0, SCREEN, &advance, LINE_HEIGHT)).2,
            Color::new(230, 230, 230, 240),
            "and the toast already on screen is on the light palette at once: \
             245 × 0.94 is 230, and a toast left on the dark one over a light \
             gallery is a dark card in a white window"
        );
        assert_eq!(
            toast_of(&toasts, 0).palette(),
            Palette::from_theme(&Theme::light()),
            "and the toast's own copy moved with it"
        );

        toasts.show(&mut nodes, OTHER, DURATION);
        finish(&mut nodes, &mut toasts);
        assert_eq!(
            surface_of(&toasts.paint_toast(1, SCREEN, &advance, LINE_HEIGHT)).2,
            Color::new(230, 230, 230, 240),
            "and a toast raised after the switch is created with it"
        );
    }

    /// Every palette names the tokens it claims, read from a real theme.
    ///
    /// The same drift test as the dialog's, and for the same reason: a palette
    /// that read the wrong token is a widget in the wrong colour with every other
    /// test here still green, because they all compare against the palette rather
    /// than against the theme.
    #[test]
    fn the_palette_from_a_theme_names_the_tokens_it_claims() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            for (what, got, token) in [
                ("surface", palette.surface, ThemeToken::Surface),
                ("text", palette.text, ThemeToken::Text),
                ("error", palette.error, ThemeToken::Error),
                ("warning", palette.warning, ThemeToken::Warning),
                ("success", palette.success, ThemeToken::Success),
                ("info", palette.info, ThemeToken::Primary),
            ] {
                assert_eq!(
                    got,
                    token_colour(&theme, token),
                    "{what} is the theme's {token:?}"
                );
            }
        }
        assert_ne!(
            Palette::from_theme(&Theme::dark()),
            Palette::from_theme(&Theme::light()),
            "and two themes really are two palettes"
        );
    }

    /// The theme token copies are the tokens' values, in both themes.
    #[test]
    fn the_theme_token_copies_are_the_tokens_values() {
        for theme in [Theme::dark(), Theme::light()] {
            assert_eq!(
                token_number(&theme, ThemeToken::SpacingLg),
                TOAST_MARGIN,
                "TOAST_MARGIN is SpacingLg"
            );
            assert_eq!(
                token_number(&theme, ThemeToken::SpacingMd),
                TOAST_PADDING,
                "TOAST_PADDING is SpacingMd"
            );
            assert_eq!(
                token_number(&theme, ThemeToken::SpacingSm),
                TOAST_GAP,
                "TOAST_GAP is SpacingSm"
            );
            assert_eq!(
                token_number(&theme, ThemeToken::BorderRadiusMd),
                TOAST_RADIUS,
                "TOAST_RADIUS is BorderRadiusMd"
            );
            assert_eq!(
                token_number(&theme, ThemeToken::FontSizeMd),
                MESSAGE_FONT,
                "MESSAGE_FONT is FontSizeMd"
            );
        }
    }

    /// The default motion is the theme's own fast duration and standard curve,
    /// read from the tokens rather than from [`Motion::from_theme`].
    #[test]
    fn the_default_motion_is_the_themes() {
        for theme in [Theme::dark(), Theme::light()] {
            let (duration, easing) = match (
                theme.get(ThemeToken::DurationFast),
                theme.get(ThemeToken::EasingStandard),
            ) {
                (
                    crate::theme::PropertyValue::Duration(duration),
                    crate::theme::PropertyValue::Easing(easing),
                ) => (duration, easing),
                (duration, easing) => {
                    panic!("{duration:?} and {easing:?} are not the motion tokens")
                }
            };
            assert_eq!(
                duration,
                Duration::from_millis(150),
                "which is the number requirement 4's four animations are over"
            );
            assert_eq!(
                default_motion(),
                Motion { duration, easing },
                "and the default is that token and that curve"
            );
            assert_eq!(default_motion(), Motion::from_theme(&theme));
        }
    }

    /// *n* toasts are *n* shadows, and each toast's shadow is submitted before
    /// that same toast's card.
    ///
    /// The cost is in the handoff rather than only here: the blur costs two
    /// full-window passes and a target bind per shadow per frame, so a stack of
    /// three costs three of each. The order is the other half of it, and it is
    /// what makes the stack read as depth — each card's shadow falls onto the card
    /// below it, so that card is submitted before this one's shadow and this one's
    /// card after it.
    #[test]
    fn every_live_toast_records_one_shadow_and_the_whole_stack_submits_in_depth() {
        let mut nodes = Arena::new();
        let mut toasts = Toasts::new(&mut nodes);
        for _ in 0..3 {
            toasts.show(&mut nodes, MESSAGE, DURATION);
        }
        finish(&mut nodes, &mut toasts);

        for index in 0..3 {
            let commands = toasts.paint_toast(index, SCREEN, &advance, LINE_HEIGHT);
            assert_eq!(
                recorded(&commands),
                vec!["shadow", "text", "surface"],
                "toast {index} carries exactly one shadow of its own"
            );
        }

        let mut whole = Vec::new();
        for index in 0..3 {
            whole.extend(toasts.paint_toast(index, SCREEN, &advance, LINE_HEIGHT));
        }
        assert_eq!(
            submitted(&whole),
            vec![
                "shadow", "surface", "text", "shadow", "surface", "text", "shadow", "surface",
                "text",
            ],
            "**each toast's shadow is submitted after the card above it and before \
             its own**, so the stack reads as depth rather than as three cards \
             shading each other"
        );
    }

    /// A box smaller than the stack's own margin does not produce a negative
    /// width and does not take the frame down.
    ///
    /// A ten-pixel box is not a screen anything is drawn on, and what the toast
    /// does there is worth pinning because the arithmetic has two ways to come
    /// apart: a width that goes negative, which is a coordinate read as an extent,
    /// and a card placed somewhere surprising, which for a bottom-anchored stack
    /// is above the box rather than inside it. The second is left as it is,
    /// because capping the stack is the *Out of scope* entry the module doc names.
    #[test]
    fn a_box_narrower_than_the_margin_still_places_a_card_inside_itself() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);
        let tiny = Rect::new(100.0, 100.0, 10.0, 10.0);
        let rect = toasts
            .toast_rect(0, tiny, &advance, LINE_HEIGHT)
            .expect("one toast");

        assert_eq!(
            rect.width, 0.0,
            "a card of no width rather than one of `10 - 48` pixels"
        );
        assert_eq!(rect.x, 105.0, "and it is inside the box it was given");
        assert_eq!(
            rect.height, 72.0,
            "and it is two lines tall rather than one: with no inner width every \
             word takes a line of its own, so \"Settings saved\" is two 20-pixel \
             line boxes on 32 of padding"
        );
        assert_eq!(
            recorded(&toasts.paint_toast(0, tiny, &advance, LINE_HEIGHT)),
            vec!["shadow", "text", "text", "surface"],
            "so the shadow, a run per line and the surface are all still \
             recorded: a card with no width draws nothing on the screen, and the \
             commands are what the batcher and these tests read"
        );
    }

    /// The card's `x` comes from the box's **width** and its `y` from the box's
    /// **bottom edge** — two numbers that are not the same number, and the reason
    /// this module's fixture is not at the origin.
    #[test]
    fn the_cards_x_comes_from_the_boxs_width_and_its_y_from_its_bottom_edge() {
        let mut nodes = Arena::new();
        let toasts = settled(&mut nodes);

        // The demo's own window: 1280 wide and 1020 tall, with an origin of
        // `(0, 0)` and three numbers that are none of each other.
        let window = Rect::new(0.0, 0.0, 1280.0, 1020.0);
        let rect = toasts
            .toast_rect(0, window, &advance, LINE_HEIGHT)
            .expect("one toast");
        assert_eq!(rect.x, 460.0, "(1280 − 360) / 2 from the box's left edge");
        assert_eq!(
            rect.y, 944.0,
            "and 1020 − 24 − 52 from the box's top edge, so it hangs off the bottom"
        );

        // And moved: the same box 90 to the right and 11 down moves the card by
        // the same 90 and 11.
        let moved = Rect::new(90.0, 11.0, 1280.0, 1020.0);
        let moved_rect = toasts
            .toast_rect(0, moved, &advance, LINE_HEIGHT)
            .expect("one toast");
        assert_eq!(
            moved_rect.x,
            rect.x + 90.0,
            "the card follows the box across"
        );
        assert_eq!(moved_rect.y, rect.y + 11.0, "and down it");
    }

    /// Returns the message of the toast at `index`.
    fn message_at(toasts: &Toasts, index: usize) -> String {
        toast_of(toasts, index).message.get()
    }
}
