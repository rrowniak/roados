//! The Dialog widget: a modal overlay with a title, a body and a row of actions.
//!
//! A dialog is **three things on one node**: a translucent overlay covering
//! whatever box it is given, a panel centred in it, and real
//! [`Button`](crate::widgets::button) widgets along the bottom of that
//! panel. The buttons are not shapes this module's [`paint`](Dialog::paint)
//! draws — each one is its own node in the arena, which is what lets a caller
//! register it with [`Focus`](crate::input::Focus) and a key activate it. A
//! dialog whose "OK" is a rounded rectangle in a `Vec<DrawCommand>` is a dialog
//! whose "OK" cannot be pressed, and `.ai/NEVERAGAIN.md` § *A drawn control with
//! nothing behind it* is the entry for that.
//!
//! # The paint order, and the shadow that makes it work
//!
//! The commands go out in one order — **overlay, shadow, panel, title, body,
//! actions** — and the middle three are not interchangeable.
//! [`Batcher::submit_order`](crate::batch::Batcher::submit_order) splits a
//! frame at every [`DrawCommand::Shadow`] and groups *each segment* opaque
//! first, because an opaque primitive drawn over a translucent one can only
//! work inside one segment. A panel and an overlay recorded adjacently are in
//! the **same** segment: the overlay is translucent, the panel is opaque, so the
//! grouping draws the panel first and the overlay lands on top of it and dims
//! it — requirement 5's *"overlay rendered first … panel on top"* inverted, with
//! both commands recorded correctly and in the right order, which is why no
//! draw-command assertion could see it. The [`Shadow`](DrawCommand::Shadow)
//! between them is the boundary that fixes it: the overlay is segment one, the
//! panel is segment two, and each segment is grouped on its own.
//!
//! # Modality is the caller's, and that is a decision
//!
//! [`input::route`](crate::input::route) walks from the node under the pointer
//! up to a root, and nothing in the input module knows what a modal is. So the
//! dialog states the contract on [`on_event`](Dialog::on_event) and the caller
//! enforces it: **while `visible` is true, offer every event to
//! `Dialog::on_event` and to nothing outside the dialog's subtree.** The
//! dialog's own node must also be laid out to the whole box, because
//! [`input::hit_test`](crate::input::hit_test) is what decides which node a tap
//! is under.
//!
//! The buttons' **keyboard** route does not go through
//! [`on_event`](Dialog::on_event) at all: the caller writes `focused` from
//! [`Focus`](crate::input::Focus), and either the dialog forwards an activation
//! key to the focused action or the caller routes it to the button node
//! directly. Both dismiss, which is why [`Dialog::add_action`] installs the
//! dismissal *into* each button's click callback rather than running it from
//! the dialog's tap handler — one place the dismissal lives, and a caller cannot
//! route an activation down a path that skips it.
//!
//! # One number, one owner
//!
//! The panel, the inner box, the text layouts and the action rects are all
//! computed in `Geometry`, which [`paint`](Dialog::paint),
//! [`action_rect`](Dialog::action_rect) and the hit test each build. The scale
//! animation moves the drawn rect, and the hit test reads the moved rect, so a
//! tap lands where the button is drawn on every frame of the animation and not
//! only at rest.
//!
//! # The title is bold, and the panel's size cannot tell
//!
//! Requirement 2's title is drawn with [`Painter::text_bold`], so it is a real
//! second face rather than the regular glyphs drawn twice — see
//! [`DrawCommand::Text`]'s `weight` and [`FontWeight`]. **It changes nothing
//! about the panel's geometry**, and that is a property of where the weight
//! lives rather than a lucky coincidence: the module's `Geometry` measures the
//! title with **the caller's own `advance` closure**, the same one it measures
//! the body and the button labels with, and the weight travels only on the
//! recorded command. There is one measurement in this module and the weight is
//! not one of its inputs.
//!
//! So `panel_rect` and `action_rect` return the same numbers for a bold title as
//! they would for a regular one. **A caller must not take that as a promise that
//! a bold run ends at the same `x`**: it is this widget's measurement that is
//! weight-blind, and
//! [`Painter::text_bold`] measured through a **real** pair of faces at 28 pixels
//! is 1 px wider over thirteen characters and *identical* for `"Settings"`,
//! because FreeType rounds each advance to a whole pixel at the size the face is
//! set to. `the_bold_title_leaves_the_panels_geometry_where_it_was` is the test,
//! and it carries a control: a title one word longer *does* move the panel, so
//! the fixture is not merely insensitive.
//!
//! # What this widget does not do
//!
//! - **A dialog with text input, with list selection, with custom content, or
//!   stacked on another.** All four are the task file's out of scope.
//!
//! # Examples
//!
//! ```
//! use std::cell::Cell;
//! use std::rc::Rc;
//! use std::time::Duration;
//! use ui_core::arena::Arena;
//! use ui_core::input::{InputEvent, InputEventKind};
//! use ui_core::layout::Offset;
//! use ui_core::node::WidgetNode;
//! use ui_core::paint::Rect;
//! use ui_core::theme::Theme;
//! use ui_core::widgets::button::Motion;
//! use ui_core::widgets::dialog::{Dialog, DialogAction};
//! use ui_core::widgets::Callback;
//!
//! let mut nodes = Arena::new();
//! let mut dialog = Dialog::new(&mut nodes, "Confirm", "Discard the draft?");
//! dialog.set_motion(Motion::from_theme(&Theme::dark()));
//!
//! // The callback is written before the action is handed over, because that is
//! // when `add_action` wraps it in the dismissal.
//! let confirmed = Rc::new(Cell::new(false));
//! let mut ok = DialogAction::new(&mut nodes, "OK");
//! let said = Rc::clone(&confirmed);
//! ok.button.on_click = Callback::new(move || said.set(true));
//! dialog.add_action(&mut nodes, ok);
//!
//! dialog.present();
//! for _ in 0..30 {
//!     dialog.tick(Duration::from_millis(10));
//! }
//!
//! let screen = Rect::new(40.0, 30.0, 1000.0, 700.0);
//! let commands = dialog.paint(screen, &|_: char| 7.0, 20.0);
//! assert!(matches!(commands[0], ui_core::paint::DrawCommand::Rect { .. }));
//!
//! // A tap on the scrim dismisses, and it reaches nothing behind the dialog.
//! let mut tap = InputEvent::new(InputEventKind::Tap, Some(Offset::new(60.0, 60.0)));
//! assert!(dialog.on_event(&mut tap, screen, &|_: char| 7.0, 20.0));
//! assert!(!dialog.visible.get());
//! ```

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::animation::{AnimationClock, Easing, Interpolate};
use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::{Offset, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, FontWeight, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::{Theme, ThemeToken};
use crate::widgets::button::{Button, Motion};
use crate::widgets::label::{
    layout_text, LayoutOptions, TextAlign, TextLayout, Truncation, WrapMode,
};
use crate::widgets::Callback;

/// The gap between the panel's edge and its text, and between its bottom and the
/// action row: [`SpacingLg`](ThemeToken::SpacingLg), which both themes hold at
/// 24 pixels.
///
/// A copy rather than a read, for the reason
/// [`button`](crate::widgets::button)'s own three copies give: the widget has no
/// theme to read from, and a caller themes a dialog by binding its properties —
/// which is the mechanism the property graph already carries a theme switch on.
/// The test `the_theme_token_copies_are_the_tokens_values` is what stops it from
/// drifting away from the token it names.
///
/// What would reverse it: a panel padding a theme switch moves, which is a
/// `SpacingLg`-bound property rather than a constant.
const PANEL_PADDING: f32 = 24.0;

/// The gap between the title and the body, and between two action buttons:
/// [`SpacingSm`](ThemeToken::SpacingSm), 8 pixels in both themes.
///
/// One constant for both rather than two names for one number, and neither is
/// [`PANEL_PADDING`]: a button row separates its buttons by less than a panel
/// separates its content, and a dialog whose actions are as far apart as its
/// title is from its body reads as two panels.
const THEME_SPACING_SM: f32 = 8.0;

/// The panel's corner radius: [`BorderRadiusLg`](ThemeToken::BorderRadiusLg),
/// 16 pixels in both themes.
///
/// The loosest of the theme's three radii because a panel is the largest
/// rounded shape in this library, and a radius scaled to a panel's size looks
/// like a mistake at that size. The **same** number is the shadow's radius, so
/// the shadow's silhouette and the panel's are one shape rather than two that
/// happen to be near each other.
const THEME_RADIUS_LG: f32 = 16.0;

/// The title's font size: [`FontSizeLg`](ThemeToken::FontSizeLg), 18 pixels in
/// both themes.
///
/// One step above the body's [`FontSizeMd`](ThemeToken::FontSizeMd), which is
/// one of the two things that distinguish the title — the other is the face, and
/// the title is drawn in the bold one. See the module doc for why the weight
/// moves nothing about the panel's geometry.
const TITLE_FONT_SIZE: f32 = 18.0;

/// The body's font size: [`FontSizeMd`](ThemeToken::FontSizeMd), 14 pixels in
/// both themes.
const BODY_FONT_SIZE: f32 = 14.0;

/// The width the panel is measured at when the box it is given is wider than
/// this, in pixels.
///
/// **Chosen, not measured.** 420 is roughly two thirds of a head unit's
/// landscape width at the sizes this repository renders text at, which puts a
/// body paragraph at about 50 characters a line and a title on one line. It is
/// a constant rather than a token because the theme has no token for a panel's
/// width, and adding one would change [`ThemeToken::all`], both theme tables and
/// the transition every token takes part in during a switch, for a value a
/// switch does not change.
///
/// What would reverse it: a capture of a real head unit at a real viewing
/// distance, which is the measurement this number is standing in for.
const PANEL_MAX_WIDTH: f32 = 420.0;

/// The fraction of the box's height the panel may take before the body is cut
/// short.
///
/// Four fifths, so a dialog with a long body keeps its action row on screen.
/// Without a ceiling the panel grows downwards until its own buttons leave the
/// window, and a dialog whose buttons are off screen is a dialog that cannot be
/// dismissed by pressing one — the same defect as a control drawn where nothing
/// can hit it, reached through the sizing instead of through a missing gesture.
///
/// It is a fraction rather than a pixel count so the panel adapts to the box it
/// is given, which is the whole of what the caller's `rect` means.
const PANEL_MAX_HEIGHT_FRACTION: f32 = 0.8;

/// The shadow's Gaussian standard deviation, in pixels.
///
/// [`Painter::shadow`](crate::paint::Painter::shadow) takes a standard deviation,
/// so this asks [`blur::kernel`] for `ceil(2σ)` taps either side — **which 8 does
/// not get.** [`blur::MAX_TAPS`] is nine in total, so every σ from 2 upwards is
/// capped at **four taps either side**, and what actually differs between 2 and 8
/// is the kernel's *weights* rather than its width: at σ 8 the outermost tap is
/// 10.3% of the centre's against 2.8% at σ 2, which makes 8 the flattest — and so
/// the softest — edge the blur can produce here.
///
/// **Measured on 2026-10-03 from a capture of the running demo, in pixels**,
/// because the number above was originally documented as *"a 16-tap kernel either
/// side, holding 99.7% of the distribution's mass"*, which is what `ceil(2σ)`
/// gives and **not** what this repository's blur does. What is on the screen:
/// the darkest value reaches the scrimmed background within **11 pixels below the
/// panel**, in 2 to 3 pixels beside it, and **not at all above it** — the last
/// because the shadow is offset 8 pixels down, so its own top edge is inside the
/// panel. A four-tap kernel around the shadow's own edge at `596.4 + 8 = 604.4`
/// predicts a ramp over 600 to 608, which is what the capture reads.
///
/// The ramp is **monotone**: 4, 4, 4, 4, 5, 5, 6, 6, 7, 8, 8, then the background
/// again. So it is a blur and not a second edge.
///
/// **It is also a performance number**, and a sharper one than the σ it names: the
/// blur pass costs two full-window passes and a target bind every frame, and the
/// widening from 4 to 8 buys nothing at all — the kernel is already at the cap.
/// What would reverse it: a wider [`blur::MAX_TAPS`], at which σ 8 would become
/// the sixteen-tap kernel this doc used to claim, or a capture on a display where
/// a 4-pixel shadow edge reads as a hairline rather than as depth.
const SHADOW_BLUR: f32 = 8.0;

/// How far the shadow falls below the panel, in pixels.
///
/// A centred shadow under a panel reads as a halo around it rather than as a
/// light source above it; 8 is about the panel's own corner radius, which puts
/// the shadow's shoulder just outside the panel's corner.
const SHADOW_OFFSET_Y: f32 = 8.0;

/// The most opaque the panel's shadow gets, from `0.0` to `1.0`.
///
/// **Half, not nearly all, and half is generous.** The shadow sits on top of an
/// overlay already at [`MAX_OVERLAY_ALPHA`], so its job is to lift the panel off
/// the dimmed content and not to black the window out. A shadow at full opacity
/// over a half-black overlay is not depth, it is a second dimmer.
///
/// **Chosen, not measured.** Nothing in this repository has captured a shadow
/// beside a scrim, so this is a judgement about how much of the window a
/// centred panel may hide. The one thing that is settled is that it is *not*
/// 1.0, for the reason above.
///
/// The shadow is **always black and has no theme token behind it**, because
/// neither theme's [`Background`](ThemeToken::Background) would do: the dark
/// theme's is `18 18 18` and the light theme's is `255 255 255`, so a shadow
/// taken from the token would be black on one theme and white on the other, and
/// a white drop shadow is not a shadow. What a light theme changes about a
/// shadow is how far it reaches, not which way it darkens.
const SHADOW_ALPHA: f32 = 0.5;

/// The scale the panel is drawn at before it has appeared, from `0.0` to `1.0`.
///
/// The task's 0.9, and it is read rather than written: the panel's opacity is
/// derived from it by [`shown_fraction`], so one property carries the whole of
/// the appear animation and there is no second number to keep in step with it.
const MIN_SCALE: f32 = 0.9;

/// The most opaque the overlay gets, from `0.0` to `1.0`.
///
/// The task's 0.5, and the reason it is half rather than nearly all is the same
/// one [`SHADOW_ALPHA`] gives: the content behind the dialog stays legible
/// through the overlay, which is the whole of what a modal scrim is for. It says
/// *there is something here you must answer first*; it does not say *nothing
/// behind this is reachable*.
const MAX_OVERLAY_ALPHA: f32 = 0.5;

/// The colours a dialog's panel and text are drawn in.
///
/// The same shape as [`button::Palette`](crate::widgets::button::Palette) and
/// [`keyboard::Palette`](crate::widgets::keyboard::Palette): a value rather than
/// a read from a theme inside the widget, because the property graph is what
/// carries a theme switch to a widget holding a property. The dialog's own
/// properties hold the three colours so a switch can be written into them.
///
/// The shadow is **not** in here, for the reason the module's `SHADOW_ALPHA`
/// gives: it is black in both themes and on no token.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The panel's fill: [`Surface`](ThemeToken::Surface), the token both
    /// themes use for a raised card.
    pub surface: Color,
    /// The title's colour: [`Text`](ThemeToken::Text), which both themes define
    /// as legible on their own surface.
    pub title: Color,
    /// The body's colour: [`TextMuted`](ThemeToken::TextMuted), so the body is
    /// present under the title rather than competing with it.
    pub body: Color,
}

impl Default for Palette {
    /// Returns a dialog that is legible with no theme at all.
    ///
    /// A dark grey panel with a near-white title and a lighter grey body: the
    /// same shape as the button's own neutral default, and a visible starting point
    /// for a caller that will bind the theme's own colours.
    fn default() -> Self {
        Palette {
            surface: Color::new(48, 48, 48, 255),
            title: Color::new(245, 245, 245, 255),
            body: Color::new(200, 200, 200, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes: its [`Surface`](ThemeToken::Surface)
    /// for the panel, its [`Text`](ThemeToken::Text) for the title and its
    /// [`TextMuted`](ThemeToken::TextMuted) for the body.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::theme::Theme;
    /// use ui_core::widgets::dialog::Palette;
    ///
    /// let dark = Palette::from_theme(&Theme::dark());
    /// let light = Palette::from_theme(&Theme::light());
    /// assert_ne!(dark, light, "and two themes really are two palettes");
    /// ```
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            surface: token_color(theme, ThemeToken::Surface),
            title: token_color(theme, ThemeToken::Text),
            body: token_color(theme, ThemeToken::TextMuted),
        }
    }
}

/// One button in a dialog's action row.
///
/// The task file names this type, and it is a [`Button`] rather than a label and
/// a closure because the action row is the part of a dialog a finger has to hit:
/// the button is its own node, it has a touch floor, it has a press state, and
/// it can be registered with [`Focus`](crate::input::Focus).
///
/// **Set `on_click` before handing the action to
/// [`Dialog::add_action`].** That is when the dismissal is installed around the
/// callback, so a callback written afterwards replaces it and the dialog stays
/// up. There is no other way to lose the dismissal, and
/// `an_action_activated_through_its_own_node_still_dismisses` is the test for
/// the route that would otherwise skip it.
///
/// # Examples
///
/// ```
/// use ui_core::arena::Arena;
/// use ui_core::input::{InputEvent, InputEventKind};
/// use ui_core::layout::Offset;
/// use ui_core::node::WidgetNode;
/// use ui_core::paint::Rect;
/// use ui_core::widgets::dialog::{Dialog, DialogAction};
/// use ui_core::widgets::Callback;
///
/// let mut nodes = Arena::new();
/// let mut dialog = Dialog::new(&mut nodes, "Confirm", "Discard the draft?");
/// let cancel = DialogAction::new(&mut nodes, "Cancel");
/// dialog.add_action(&mut nodes, cancel);
/// dialog.present();
///
/// // A tap on the button is that button, and it closes the dialog.
/// let screen = Rect::new(40.0, 30.0, 1000.0, 700.0);
/// let cancel = dialog.action_rect(0, screen, &|_: char| 7.0, 20.0).expect("one action");
/// let mut tap = InputEvent::new(
///     InputEventKind::Tap,
///     Some(Offset::new(cancel.x + 2.0, cancel.y + 2.0)),
/// );
/// assert!(dialog.on_event(&mut tap, screen, &|_: char| 7.0, 20.0));
/// assert!(tap.consumed());
/// assert!(!dialog.visible.get());
/// ```
pub struct DialogAction {
    /// The action's button: the colours, the press state and the node are all
    /// reached through this.
    ///
    /// Public because every caller of a dialog reaches for it —
    /// [`Palette::from_theme`](crate::widgets::button::Palette::from_theme) for
    /// its colours, `hovered` and `pressed` from a pointer, and its node for
    /// [`Focus`](crate::input::Focus).
    pub button: Button,
    /// What closes the dialog, so the click callback this action is wired with
    /// can close it wherever the click came from.
    transition: Transition,
}

impl DialogAction {
    /// Creates an action whose button says `text`, in the arena, and returns it.
    ///
    /// The action is not yet part of any dialog and holds nothing that can close
    /// one; [`Dialog::add_action`] fills that in.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, text: impl Into<String>) -> Self {
        DialogAction {
            button: Button::new(nodes, text),
            transition: Transition::detached(),
        }
    }

    /// Returns the action's node in the arena.
    ///
    /// The handle a caller registers with
    /// [`Focus::set_focusable`](crate::input::Focus::set_focusable) and matches
    /// against [`Focus::current`](crate::input::Focus::current).
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.button.handle()
    }

    /// Returns the draw commands that paint the action's button within `rect`,
    /// with every colour scaled by `multiplier` on top of the button's own
    /// opacity.
    ///
    /// `rect` is the one [`Dialog::action_rect`] returns rather than one the
    /// caller composes: the two are the same number by construction, which is
    /// what stops a button being drawn somewhere it cannot be tapped.
    ///
    /// **`multiplier` is a multiplier and not a replacement** — the product is what
    /// reaches the screen, so a disabled action inside a fading dialog comes out at
    /// the product of the two rather than at whichever is smaller. See
    /// [`Button::paint_faded`](crate::widgets::button::Button::paint_faded), whose
    /// doc says the same and explains why.
    ///
    /// [`Dialog::paint`] passes its own transition progress here, which is the whole
    /// of what this parameter is for: the panel, the title, the body and the shadow
    /// all fade with it, and before it existed the two action buttons did not, so
    /// they sat at full strength for the whole of a 300 ms fade and then went in a
    /// single frame.
    #[must_use]
    pub fn paint(
        &self,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
        multiplier: f32,
    ) -> Vec<DrawCommand> {
        self.button
            .paint_faded(rect, advance, line_height, multiplier)
    }
}

/// A dialog: the overlay, the panel and the actions, and the transition between
/// showing and not showing.
///
/// The properties that hold a theme token are bound by the caller rather than
/// read from a theme here, for the reason the rest of the library reads tokens
/// through properties: the property graph carries a theme switch to a widget
/// holding a property, and a widget that reached into a theme would have to be
/// told about the switch instead. The defaults are neutral literals, the way
/// [`Button`]'s are.
///
/// [`Dialog::new`] gives a dialog that is **not** showing: `visible` starts
/// `false`, the overlay's alpha starts at zero and the panel's scale starts at
/// 0.9, which is the state a dismissal leaves behind. Nothing is drawn
/// until [`present`](Dialog::present) is called.
pub struct Dialog {
    /// The panel's title, drawn **bold** at the theme's `FontSizeLg`.
    pub title: Property<String>,
    /// The panel's body, drawn below the title at the theme's `FontSizeMd` and
    /// wrapped to the panel's inner width.
    pub body: Property<String>,
    /// Whether the dialog is showing.
    ///
    /// A caller that repaints only when something asks it to has to mark this
    /// node dirty, because the dialog is what decides when it repaints:
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::dialog::Dialog;
    ///
    /// let nodes = Rc::new(RefCell::new(Arena::new()));
    /// let dialog = Dialog::new(&mut nodes.borrow_mut(), "Title", "Body");
    /// let handle = dialog.handle();
    /// let arena = Rc::clone(&nodes);
    /// dialog.visible.on_change(move |_| {
    ///     if let Some(node) = arena.borrow_mut().get_mut(handle) {
    ///         node.paint_mut().mark_dirty();
    ///     }
    /// });
    ///
    /// assert!(!dialog.visible.get());
    /// dialog.present();
    /// assert!(dialog.visible.get());
    /// ```
    pub visible: Property<bool>,
    /// The dialog's action buttons, in the order they are laid out in.
    ///
    /// A plain field and not a property: these are widgets the dialog owns, with
    /// nodes of their own in the arena, and a [`Property`] holds one `T` behind
    /// a shared cell — it cannot hold a `Vec<Button>`. It is the same argument
    /// [`Container`](crate::widgets::container::Container) makes about layout
    /// inputs living on the node rather than on the widget.
    pub actions: Vec<DialogAction>,
    /// Whether a tap outside the panel dismisses the dialog.
    ///
    /// **On** by default, because a dialog opened by mistake is otherwise stuck
    /// until something inside it is pressed, and requirement 3 lists this as
    /// configurable rather than as the only behaviour. A dialog asking a question
    /// that must be answered turns it off, so the scrim does not offer an answer
    /// that is not one of its buttons.
    pub dismiss_on_outside_tap: Property<bool>,
    /// The panel's fill.
    pub surface: Property<Color>,
    /// The title's colour.
    pub title_color: Property<Color>,
    /// The body's colour.
    pub body_color: Property<Color>,
    /// The panel's corner radius, and the shadow's.
    pub border_radius: Property<f32>,
    /// The panel's scale, `1.0` when it is fully shown and the task's 0.9 when it
    /// is not.
    ///
    /// Written by the transition, and read by both [`paint`](Dialog::paint) and
    /// [`action_rect`](Dialog::action_rect). It is public because a caller that
    /// wants no animation at all can write it — with the overlay's alpha — and
    /// the panel's opacity follows from it.
    pub scale: Property<f32>,
    /// The overlay's opacity, `0.0` to the task's 0.5.
    pub overlay_alpha: Property<f32>,
    palette: Palette,
    /// The show and hide transition, and the shared handles an action's click
    /// uses to run it.
    ///
    /// The three properties above are the ones this holds: a [`Property`] clone
    /// shares the cell rather than copying the value, so a caller writing
    /// `dialog.visible.set(false)` and an action calling `Transition::hide` are
    /// writing one value and not two.
    transition: Transition,
    node: Handle,
}

impl Dialog {
    /// Creates a dialog with `title` and `body` in the arena, and returns it.
    ///
    /// It starts hidden: [`present`](Dialog::present) is what makes it appear,
    /// and until then [`paint`](Dialog::paint) records nothing at all.
    ///
    /// The task file's `Dialog::new(title, body) -> Handle` is read as this: the
    /// handle is [`Dialog::handle`]'s, and returning it alone would leave the
    /// caller with no properties to set and no way to add the actions that are
    /// the whole of what a dialog is for.
    /// [`Container::new`](crate::widgets::container::Container::new) and
    /// [`Button::new`](crate::widgets::button::Button::new) settled the reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::dialog::Dialog;
    ///
    /// let mut nodes = Arena::new();
    /// let dialog = Dialog::new(&mut nodes, "Confirm", "Discard the draft?");
    /// assert_eq!(dialog.title.get(), "Confirm");
    /// assert!(!dialog.visible.get());
    /// assert!(
    ///     dialog.paint(Rect::new(40.0, 30.0, 1000.0, 700.0), &|_: char| 7.0, 20.0).is_empty(),
    ///     "and a hidden dialog draws nothing at all"
    /// );
    /// ```
    #[must_use]
    pub fn new(
        nodes: &mut Arena<WidgetNode>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        let palette = Palette::default();
        let transition = Transition::new();
        let node = node::create(nodes, crate::layout::LayoutState::new());
        Dialog {
            title: Property::new(title.into()),
            body: Property::new(body.into()),
            // The three the transition writes, and not three more: a `Property`
            // clone shares the cell, so these and the ones inside `transition`
            // are one value each. Two `Property::new` calls here would give the
            // caller three properties nothing ever writes.
            visible: transition.visible.clone(),
            scale: transition.scale.clone(),
            overlay_alpha: transition.overlay_alpha.clone(),
            actions: Vec::new(),
            dismiss_on_outside_tap: Property::new(true),
            surface: Property::new(palette.surface),
            title_color: Property::new(palette.title),
            body_color: Property::new(palette.body),
            border_radius: Property::new(THEME_RADIUS_LG),
            palette,
            transition,
            node,
        }
    }

    /// Returns the dialog's node in the arena.
    ///
    /// The node is the whole of the dialog's overlay: lay it out to the box the
    /// overlay covers, because [`input::hit_test`](crate::input::hit_test) is
    /// what decides which node a tap is under, and the widget draws and
    /// hit-tests in the `rect` it is handed rather than reading this node's own.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Adds `action` to the dialog's action row, in the order it was added.
    ///
    /// The action's node is attached to the dialog's, so
    /// [`input::route`](crate::input::route) reaches it from inside the panel,
    /// and the callback it carries is wrapped so that **activating the action
    /// closes the dialog** — from this widget's own [`on_event`](Dialog::on_event),
    /// from a key the caller routes to the button node directly, or from anything
    /// else that runs the callback. Requirement 3's *"fire callbacks and dismiss
    /// dialog"* is one order: the caller's callback runs **first**, so a handler
    /// that reads the dialog still sees it showing.
    ///
    /// **It cannot fail, and there is no `bool` saying so.** [`node::attach`]
    /// refuses a child that already has a parent, the parent's own node, and a
    /// link that would close a cycle — and an action carries a node
    /// [`DialogAction::new`] has just created in `nodes`, so none of the three is
    /// reachable from here. The refusal is [`node::attach`]'s, and it is this
    /// widget's caller who would meet it by handing over a [`Handle`] rather than
    /// an action, the way [`Container::add_child`](crate::widgets::container::Container::add_child)
    /// does.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::dialog::{Dialog, DialogAction};
    ///
    /// let mut nodes = Arena::new();
    /// let mut dialog = Dialog::new(&mut nodes, "Confirm", "Discard the draft?");
    /// for label in ["Cancel", "OK"] {
    ///     let action = DialogAction::new(&mut nodes, label);
    ///     dialog.add_action(&mut nodes, action);
    /// }
    /// assert_eq!(dialog.actions.len(), 2);
    /// assert_eq!(dialog.actions[0].button.label.get(), "Cancel");
    /// assert_eq!(dialog.actions[1].button.label.get(), "OK");
    /// ```
    pub fn add_action(&mut self, nodes: &mut Arena<WidgetNode>, mut action: DialogAction) {
        // `node::attach` is `#[must_use]`, and its answer is deliberately
        // discarded rather than asserted on: an action carries a node this
        // call's caller has just created in this caller's arena, so there is no
        // refusal to report, nothing to return and nothing to panic about.
        let _ = node::attach(nodes, self.node, action.button.handle());
        let closes = self.transition.clone();
        let reported = action.button.on_click.clone();
        action.transition = closes.clone();
        action.button.on_click = Callback::new(move || {
            reported.call(());
            let _ = closes.hide(closes.motion());
        });
        self.actions.push(action);
    }

    /// Sets the colours the panel and its text are drawn in.
    ///
    /// The change is immediate rather than animated, and the asymmetry with
    /// [`Button::set_palette`](crate::widgets::button::Button::set_palette) is the
    /// point: a button animates its colours because its *states* change
    /// constantly, and a caller animates toward a new palette after announcing
    /// it. A dialog's three colours change only when the theme does, so a
    /// crossfade would be a second pair of transitions on a clock that already
    /// carries the show and the hide.
    ///
    /// What would reverse it: a theme switch that crossfades the panel over
    /// [`DurationNormal`](ThemeToken::DurationNormal), which is two more
    /// animations on the same clock and nothing more.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
        self.surface.set(palette.surface);
        self.title_color.set(palette.title);
        self.body_color.set(palette.body);
    }

    /// Returns the colours the panel and its text are drawn in.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets how long the show and hide transitions take and the curve they
    /// follow.
    ///
    /// The panel's scale ignores `motion`'s easing: requirement 4 names a bounce
    /// for the panel and the theme has no token for one. See `default_motion`.
    pub fn set_motion(&mut self, motion: Motion) {
        self.transition.set_motion(motion);
    }

    /// Returns how the show and hide transitions are currently timed.
    ///
    /// The default is `default_motion()`, which is the theme's
    /// [`DurationNormal`](ThemeToken::DurationNormal) and
    /// [`EasingStandard`](ThemeToken::EasingStandard) — **not**
    /// [`Motion::from_theme`], which reads the *fast* duration and would give a
    /// four-arc bounce half the time it needs. The test
    /// `the_default_motion_is_the_themes` reads the tokens and is what stops the
    /// two from drifting.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::animation::Easing;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::widgets::button::Motion;
    /// use ui_core::widgets::dialog::Dialog;
    ///
    /// let mut nodes = Arena::new();
    /// let mut dialog = Dialog::new(&mut nodes, "Title", "Body");
    /// assert_eq!(dialog.motion().duration, std::time::Duration::from_millis(300));
    ///
    /// dialog.set_motion(Motion {
    ///     duration: std::time::Duration::from_millis(100),
    ///     easing: Easing::Linear,
    /// });
    /// assert_eq!(dialog.motion().duration, std::time::Duration::from_millis(100));
    /// assert_eq!(dialog.motion().easing, Easing::Linear);
    /// ```
    #[must_use]
    pub fn motion(&self) -> Motion {
        self.transition.motion()
    }

    /// Shows the dialog, and reports whether it was not showing already.
    ///
    /// The overlay fades from nothing to its 0.5 and the panel scales from 0.9
    /// to its own size on a bounce, both over
    /// [`motion`](Dialog::motion). The transition starts from wherever the two
    /// properties stand, so a dialog dismissed half way out comes back from half
    /// way rather than snapping.
    #[must_use]
    pub fn present(&self) -> bool {
        let motion = self.transition.motion();
        self.transition.show(motion)
    }

    /// Dismisses the dialog, and reports whether it was showing.
    ///
    /// `visible` goes false immediately, so the caller stops routing to the
    /// dialog at once, while the overlay fades out and the panel scales back to
    /// 0.9 over [`motion`](Dialog::motion). This is what every
    /// dismissal runs: an action button's click, an
    /// [`Escape`](Dialog::on_event) key, and a tap outside the panel.
    #[must_use]
    pub fn dismiss(&self) -> bool {
        let motion = self.transition.motion();
        self.transition.hide(motion)
    }

    /// Advances the show and hide transition by `delta`, and reports whether it
    /// wrote anything.
    ///
    /// It is the dialog's frame integration, the same contract
    /// [`Button::tick`](crate::widgets::button::Button::tick) has: call it once a
    /// frame, before the paint pass. A caller that repaints only when this is
    /// true repaints exactly while something moves.
    #[must_use]
    pub fn tick(&self, delta: Duration) -> bool {
        self.transition.tick(delta)
    }

    /// Returns whether the show or hide transition is still running.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.transition.is_animating()
    }

    /// Returns whether `paint` would record anything this frame.
    ///
    /// It is [`visible`](Dialog::visible) **or** a running transition, which is
    /// what lets a dismissed dialog fade out while no longer being the thing the
    /// caller routes input to. A dialog that is hidden with nothing running
    /// records nothing — not a transparent overlay and not a transparent panel.
    #[must_use]
    pub fn is_drawn(&self) -> bool {
        self.visible.get() || self.is_animating()
    }

    /// Returns the rect the panel is drawn in, inside the box `rect`.
    ///
    /// The panel is measured from its own content — its title, its body and the
    /// buttons in its action row — capped at 420 pixels wide and at four fifths
    /// of `rect`'s height, and then **centred** in `rect`.
    /// It is measured rather than given a fixed size because the two things that
    /// decide a dialog's height are a wrapped paragraph and a row of buttons,
    /// and neither has a size this widget could know without measuring them.
    ///
    /// The rect returned is the one as **drawn**: the panel's scale is already
    /// applied about its centre, so this is the rect
    /// [`action_rect`](Dialog::action_rect) places its actions inside and the
    /// rect a scrim tap is judged against. One number, three readers.
    ///
    /// `advance` and `line_height` are the measurements
    /// [`Button::content_size`](crate::widgets::button::Button::content_size)
    /// takes.
    #[must_use]
    pub fn panel_rect(&self, rect: Rect, advance: &dyn Fn(char) -> f32, line_height: f32) -> Rect {
        Geometry::new(self, rect, advance, line_height).drawn
    }

    /// Returns the rect action `index` is drawn in and hit-tested by, or `None`
    /// when there is no such action.
    ///
    /// This is the rect a caller should ask for rather than composing one: it is
    /// built from the same `Geometry` the drawing is, at the same scale, so a tap
    /// lands where the button is painted on every frame of the animation.
    #[must_use]
    pub fn action_rect(
        &self,
        index: usize,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Option<Rect> {
        let geometry = Geometry::new(self, rect, advance, line_height);
        geometry
            .actions
            .get(index)
            .map(|action| geometry.drawn_rect(*action))
    }

    /// Handles `event` as the dialog would, and reports whether it consumed it.
    ///
    /// **The modality contract, which the caller enforces.** While
    /// [`visible`](Dialog::visible) is true:
    ///
    /// - Offer every event to this method, and to nothing outside the dialog's
    ///   subtree. A tap that missed the dialog has to find no node on the way
    ///   out, or the content behind it is not blocked and the dialog is a
    ///   picture rather than a modal.
    /// - Lay the dialog's own node out to the whole box, because
    ///   [`input::hit_test`](crate::input::hit_test) decides which node a tap is
    ///   under from that rect.
    /// - Register each action's node with
    ///   [`Focus::set_focusable`](crate::input::Focus::set_focusable) and write
    ///   its `focused` property from [`Focus::current`](crate::input::Focus):
    ///   that is the `Tab` order, and it is the action row left to right.
    ///
    /// A [`Tap`](InputEventKind::Tap) is **always** consumed while the dialog is
    /// showing, wherever it lands: on an action, on the panel, or on the scrim.
    /// On an action it fires that action's click — which dismisses, through the
    /// wrapper [`Dialog::add_action`] installed — unless the button is disabled.
    /// On the panel it does nothing else. On the scrim it dismisses if
    /// [`dismiss_on_outside_tap`](Dialog::dismiss_on_outside_tap) is set. A tap
    /// with no position is left alone.
    ///
    /// An [`Escape`](InputEventKind::KeyDown) key dismisses the dialog and is
    /// consumed, so it does not reach whatever is behind the overlay. Every
    /// other key is offered to the **focused** action, and a key that action does
    /// not consume is left alone and not consumed, so it carries on up the tree.
    ///
    /// Every other event is left alone, and a dialog that is not showing handles
    /// nothing at all.
    ///
    /// # Examples
    ///
    /// ```
    /// use ui_core::arena::Arena;
    /// use ui_core::input::{InputEvent, InputEventKind, Key};
    /// use ui_core::layout::Offset;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::Rect;
    /// use ui_core::widgets::dialog::Dialog;
    ///
    /// let mut nodes = Arena::new();
    /// let dialog = Dialog::new(&mut nodes, "Confirm", "Discard the draft?");
    /// let screen = Rect::new(40.0, 30.0, 1000.0, 700.0);
    ///
    /// // A hidden dialog handles nothing, so a caller that has not presented it
    /// // yet has nothing swallowed.
    /// let mut escape = InputEvent::new(
    ///     InputEventKind::KeyDown {
    ///         key: Key::Keyboard(sdl3::keyboard::Keycode::Escape),
    ///         keymod: sdl3::keyboard::Mod::empty(),
    ///     },
    ///     None,
    /// );
    /// assert!(!dialog.on_event(&mut escape, screen, &|_: char| 7.0, 20.0));
    /// assert!(!escape.consumed());
    ///
    /// dialog.present();
    /// assert!(dialog.on_event(&mut escape, screen, &|_: char| 7.0, 20.0));
    /// assert!(escape.consumed());
    /// assert!(!dialog.visible.get(), "and the dialog is on its way out");
    /// ```
    pub fn on_event(
        &self,
        event: &mut InputEvent,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> bool {
        if !self.visible.get() {
            return false;
        }
        match event.kind() {
            InputEventKind::Tap => {
                let Some(position) = event.position() else {
                    return false;
                };
                // Consumed before anything else is decided: a modal takes every
                // tap that lands in its box, whether or not it does anything with
                // it. A tap that fell through to the panel behind is not a modal
                // any more, it is a picture.
                event.consume();
                // One measurement for all three questions: which action, is the
                // panel under it, and is this the scrim. Asking twice is two
                // chances to measure the same thing differently.
                let geometry = Geometry::new(self, rect, advance, line_height);
                if let Some(index) = geometry.action_at(position) {
                    // **`may_activate` and not `disabled` alone.** This arm
                    // activates the button itself rather than offering it the tap,
                    // so it is a second activation path and it has to ask the same
                    // question the button's own `on_event` asks. It used to read
                    // `disabled` only, which was complete when `disabled` was the
                    // only way to refuse; `Button::activatable` made it incomplete,
                    // and a withdrawn action would have stayed live to a finger
                    // while refusing the keyboard.
                    if self.actions[index].button.may_activate() {
                        let _ = self.actions[index].button.activate();
                    }
                    return true;
                }
                if contains(geometry.drawn, position) {
                    return true;
                }
                if self.dismiss_on_outside_tap.get() {
                    let _ = self.dismiss();
                }
                true
            }
            InputEventKind::KeyDown {
                key: Key::Keyboard(sdl3::keyboard::Keycode::Escape),
                ..
            } => {
                event.consume();
                let _ = self.dismiss();
                true
            }
            InputEventKind::KeyDown { .. } => {
                // A key press is not routed by position, so it reaches the one
                // action that holds focus and nothing else. An action that
                // declines it leaves it unconsumed for the tree behind.
                //
                // **`focused` and not `activatable`, and that is the separation
                // working rather than an omission.** This arm answers *which
                // action owns the keyboard right now*, which is one question with
                // one answer: the loop returns on the first action holding focus,
                // so consulting `activatable` here could only route the key to a
                // **different** action than the focused one — which is not a
                // refusal, it is a misroute. Whether the focused action then *acts*
                // is the button's answer to give, and
                // [`Button::on_event`](crate::widgets::button::Button::on_event)
                // gives it by consulting `activatable` and consuming the key.
                //
                // The consequence worth stating: a withdrawn action still claims
                // the key rather than declining it, so the tree behind the dialog
                // does not see it. That is the same wall a disabled button is,
                // and it is what makes the demo's own `dialog_is_modal` guard
                // redundant rather than load-bearing.
                for action in &self.actions {
                    if action.button.focused.get() {
                        return action.button.on_event(event);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Returns the draw commands that paint the dialog inside `rect`, or nothing
    /// at all while it is hidden and no transition is running.
    ///
    /// The commands are, in order: the **overlay**, over the whole of `rect`; the
    /// **shadow**, offset below the panel and blurred; the **panel**; the
    /// **title** in the renderer's bold face, one command per line; the **body**
    /// in its regular one, one per line; and then each action's own commands, in
    /// the order the actions were added.
    ///
    /// The weight is on each recorded command and on nothing else: a renderer
    /// given no bold face draws the title with its regular one, which
    /// [`Painter::text_bold`] says is deliberate. See the module doc for why the
    /// weight is not an input to the panel's size.
    ///
    /// **The shadow is between the overlay and the panel, and that is the whole
    /// of its job.** [`Batcher::submit_order`](crate::batch::Batcher::submit_order)
    /// groups each segment's opaque commands ahead of its translucent ones, so an
    /// opaque panel recorded next to a translucent overlay is drawn *first* and
    /// the overlay lands on top of it. The shadow is the segment boundary that
    /// keeps them apart. See the module doc.
    ///
    /// The panel's scale is applied to the rect that is drawn and to the
    /// positions inside it, and not to the font sizes: a scale that resized the
    /// type would ask the glyph atlas for a new rasterisation of every character
    /// on every frame of the transition, and the panel's own size already carries
    /// the motion.
    ///
    /// `advance` and `line_height` are the measurements
    /// [`Button::content_size`](crate::widgets::button::Button::content_size)
    /// takes. The line height is **one** number for the title and the body: this
    /// widget has two font sizes and one `line_height`, and the caller's is the
    /// one both use. What would reverse it: a second parameter, or a font metric
    /// per size.
    ///
    /// # Examples
    ///
    /// The title is the panel's only bold run, and the way to see that is the
    /// weight on the command rather than a count of commands:
    ///
    /// ```
    /// use std::time::Duration;
    /// use ui_core::arena::Arena;
    /// use ui_core::node::WidgetNode;
    /// use ui_core::paint::{DrawCommand, FontWeight, Rect};
    /// use ui_core::widgets::dialog::{Dialog, DialogAction};
    ///
    /// let mut nodes = Arena::new();
    /// let mut dialog = Dialog::new(&mut nodes, "Settings", "The gallery's colours.");
    /// let ok = DialogAction::new(&mut nodes, "OK");
    /// dialog.add_action(&mut nodes, ok);
    /// dialog.present();
    /// for _ in 0..30 {
    ///     let _ = dialog.tick(Duration::from_millis(10));
    /// }
    ///
    /// let commands = dialog.paint(Rect::new(0.0, 0.0, 1280.0, 1020.0), &|_: char| 7.0, 20.0);
    /// let weights: Vec<FontWeight> = commands
    ///     .iter()
    ///     .filter_map(|command| match command {
    ///         DrawCommand::Text { weight, .. } => Some(*weight),
    ///         _ => None,
    ///     })
    ///     .collect();
    /// assert_eq!(
    ///     weights,
    ///     vec![FontWeight::Bold, FontWeight::Regular, FontWeight::Regular],
    ///     "the title is bold, the body is not, and the button's own label is \
    ///      this widget's business only in that it is on the panel"
    /// );
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
        let geometry = Geometry::new(self, rect, advance, line_height);
        let radius = self.border_radius.get();
        let alpha = self.overlay_alpha.get().clamp(0.0, 1.0);
        let shown = shown_fraction(self.scale.get());

        let mut painter = Painter::new();
        // The overlay, the scrim, over everything the caller already drew: the
        // one command here that is drawn at all during the first frames of an
        // appearance, and skipped while the alpha is still nothing.
        if alpha > 0.0 {
            painter.rect(rect, Color::new(0, 0, 0, level(alpha * 255.0)));
        }
        if shown > 0.0 {
            painter.shadow(
                geometry.drawn,
                radius,
                Color::new(0, 0, 0, level(SHADOW_ALPHA * shown * 255.0)),
                SHADOW_BLUR,
                (0.0, SHADOW_OFFSET_Y),
            );
        }
        painter.rounded_rect(geometry.drawn, radius, faded(self.surface.get(), shown));

        if geometry.title_present {
            let color = faded(self.title_color.get(), shown);
            self.paint_lines(
                &mut painter,
                &geometry,
                &geometry.title,
                geometry.title_top,
                Run {
                    color,
                    font_size: TITLE_FONT_SIZE,
                    // Requirement 2's "bold text at top of panel", and the only
                    // place in this module that asks for a face: the body's own
                    // call below passes the regular one.
                    weight: FontWeight::Bold,
                },
            );
        }
        if geometry.body_present {
            let color = faded(self.body_color.get(), shown);
            self.paint_lines(
                &mut painter,
                &geometry,
                &geometry.body,
                geometry.body_top,
                Run {
                    color,
                    font_size: BODY_FONT_SIZE,
                    weight: FontWeight::Regular,
                },
            );
        }
        // **The actions fade with `shown`, and that is the whole of this call's
        // fourth argument.** Everything else this method draws — the panel, the
        // title, the body and the shadow — is faded by `shown` a few lines above,
        // and the two buttons were not: `Button::paint` takes its opacity from the
        // button's own `opacity` property, which is animated for the *disabled*
        // state and which nothing in this module writes. So on the first frame of an
        // appearance the buttons were drawn at full strength over an invisible
        // panel, they stayed at full strength for the whole 300 ms of a dismissal,
        // and then they were gone in one frame — which is what an operator reported
        // as *"the buttons blink for a moment"* on the way in and on the way out.
        //
        // `.ai/NEVERAGAIN.md` § *a strength clamped to 0..=1, used directly as an
        // effect's size* is the class: every test in this module asserted **which**
        // commands were recorded, and these were recorded correctly on every frame.
        // The defect was in the alpha **on** a command that was present.
        for (index, action) in self.actions.iter().enumerate() {
            if let Some(rect) = geometry
                .actions
                .get(index)
                .map(|drawn| geometry.drawn_rect(*drawn))
            {
                painter.extend(action.paint(rect, advance, line_height, shown));
            }
        }
        painter.finish()
    }

    /// Records one block of text, top to bottom, starting `top` pixels below the
    /// panel's inner top edge, each line placed where it is drawn.
    ///
    /// **The weight is on the [`Run`], not on the position or the size**, so the
    /// two calls above are the whole of the title's boldness: this function
    /// measures nothing and the weight reaches the screen only on the command it
    /// records, which is what makes the module doc's claim about the panel's
    /// geometry true by construction rather than by a measurement agreeing.
    fn paint_lines(
        &self,
        painter: &mut Painter,
        geometry: &Geometry,
        layout: &TextLayout,
        top: f32,
        run: Run,
    ) {
        let mut y = geometry.map_y(geometry.inner.y + top);
        for line in &layout.lines {
            let x = geometry.map_x(geometry.inner.x + line.x_offset);
            // One branch and two calls rather than a weight carried into the
            // command by hand: `Painter` keeps the two faces apart on purpose,
            // and a third route into a text command is a third place for the two
            // of them to disagree about anything else.
            match run.weight {
                FontWeight::Regular => {
                    painter.text(x, y, &line.text, run.color, run.font_size, 0.0)
                }
                FontWeight::Bold => {
                    painter.text_bold(x, y, &line.text, run.color, run.font_size, 0.0);
                }
            }
            y += geometry.line_height;
        }
    }
}

/// One block of the panel's text, and the three things that make a recorded run
/// what it is.
///
/// A struct rather than three more parameters because [`Dialog::paint_lines`]
/// would then take eight of them and `clippy::too_many_arguments` would either
/// fire or want an `allow` — and because a reader asking *which field of a
/// recorded title run is the bold one* can be answered by pointing at this.
///
/// It is a **paint** description and not a layout one, deliberately: nothing in
/// [`Geometry`] reads it, which is the module doc's whole claim that a bold title
/// moves nothing about the panel's size.
#[derive(Clone, Copy)]
struct Run {
    /// The colour, already faded by the panel's own opacity.
    color: Color,
    /// The size the run is rasterized at.
    font_size: f32,
    /// The face it is drawn with.
    weight: FontWeight,
}

/// How a dialog's show and hide transition runs: the two properties it writes,
/// the clock they run on and the motion they run at.
///
/// It is a type of its own rather than two fields of the dialog because an
/// action's click has to be able to run the dismissal **from wherever the click
/// came from** — a tap this widget dispatched, or a key a caller routed to the
/// button's own node. Those are two callers and one behaviour, and the
/// alternative was a caller that had to remember to dismiss, which is how a
/// drawn control ends up with nothing behind it.
///
/// Cloning it shares the clock, the motion and the properties: every field is
/// behind an `Rc` or a [`Property`], so there is one transition per dialog
/// however many actions it has.
#[derive(Clone)]
struct Transition {
    visible: Property<bool>,
    overlay_alpha: Property<f32>,
    scale: Property<f32>,
    clock: Rc<RefCell<AnimationClock>>,
    motion: Rc<Cell<Motion>>,
}

impl Transition {
    /// Returns a transition over properties of its own, holding the hidden state.
    fn new() -> Self {
        Transition {
            visible: Property::new(false),
            overlay_alpha: Property::new(0.0),
            scale: Property::new(MIN_SCALE),
            clock: Rc::new(RefCell::new(AnimationClock::new())),
            motion: Rc::new(Cell::new(default_motion())),
        }
    }

    /// Returns a transition that can show and hide nothing.
    ///
    /// What an action holds between [`DialogAction::new`] and
    /// [`Dialog::add_action`]. `add_action` refuses an action whose button
    /// already has a parent, so an action left detached is never wired to
    /// anything and its closure is never run.
    fn detached() -> Self {
        Transition::new()
    }

    /// Sets the motion the show and hide transitions run at.
    fn set_motion(&self, motion: Motion) {
        self.motion.set(motion);
    }

    /// Returns the motion the show and hide transitions run at.
    fn motion(&self) -> Motion {
        self.motion.get()
    }

    /// Shows the dialog, and reports whether it was not showing already.
    fn show(&self, motion: Motion) -> bool {
        if self.visible.get() {
            return false;
        }
        self.visible.set(true);
        self.start(1.0, MAX_OVERLAY_ALPHA, motion);
        true
    }

    /// Dismisses the dialog, and reports whether it was showing.
    fn hide(&self, motion: Motion) -> bool {
        if !self.visible.get() {
            return false;
        }
        self.visible.set(false);
        self.start(MIN_SCALE, 0.0, motion);
        true
    }

    /// Starts both transitions toward `to_scale` and `to_alpha` over `motion`.
    ///
    /// The clock is cleared first, so a dismissal started half way through an
    /// appearance stops where it is rather than writing over the new transition
    /// when the old one arrives — which is what a clock shared with another
    /// widget could not do, and why this owns one.
    ///
    /// The panel's scale follows [`Easing::Bounce`] whatever `motion`'s easing
    /// is, because requirement 4 names a bounce for the panel and the theme has
    /// no token for one.
    fn start(&self, to_scale: f32, to_alpha: f32, motion: Motion) {
        let mut clock = self.clock.borrow_mut();
        clock.clear();
        clock.add(
            self.overlay_alpha
                .animate_to(to_alpha, motion.duration, motion.easing),
        );
        clock.add(
            self.scale
                .animate_to(to_scale, motion.duration, Easing::Bounce),
        );
    }

    /// Advances the transition by `delta`, and reports whether it wrote.
    fn tick(&self, delta: Duration) -> bool {
        self.clock.borrow_mut().tick(delta)
    }

    /// Returns whether either transition is still running.
    fn is_animating(&self) -> bool {
        self.clock.borrow().is_animating()
    }
}

/// One frame's worth of the dialog's geometry, computed once and read by the
/// drawing, by the public rect accessors and by the hit test.
///
/// It exists because there are three consumers and they must not be three
/// computations: a panel drawn at one size and hit-tested at another is a panel
/// whose scrim tap lands in the wrong place, and `.ai/NEVERAGAIN.md` § *A drawn
/// control with nothing behind it* asks for exactly that consistency.
struct Geometry {
    /// The panel at rest: centred in the box, sized from its own content.
    rest: Rect,
    /// The panel as it is drawn — `rest` scaled about its own centre — and the
    /// rect the scrim hit test is judged against.
    drawn: Rect,
    /// The scale `drawn` was made at.
    scale: f32,
    /// The box inside `rest` that the text and the action row live in.
    inner: Rect,
    /// The title's lines, measured at `inner`'s width.
    title: TextLayout,
    /// The body's lines, measured at `inner`'s width and capped to the height
    /// the panel's maximum leaves.
    body: TextLayout,
    /// How far below `inner`'s top edge each block of text starts.
    title_top: f32,
    body_top: f32,
    /// Whether each block of text has anything in it.
    ///
    /// An empty string still lays out one empty line, so a caller who leaves the
    /// title blank would get a draw command for it — a command that costs a
    /// vertex buffer slot and draws nothing. This is the flag that says a block
    /// is absent rather than empty, and it is the same decision the height
    /// arithmetic above made.
    title_present: bool,
    body_present: bool,
    /// Each action's rect **at rest**, left to right along the panel's bottom.
    actions: Vec<Rect>,
    /// The caller's line height, which both blocks of text use.
    line_height: f32,
}

impl Geometry {
    /// Measures the dialog's panel, its text and its action row inside `rect`.
    fn new(dialog: &Dialog, rect: Rect, advance: &dyn Fn(char) -> f32, line_height: f32) -> Self {
        let line_height = line_height.max(0.0);
        let action_sizes: Vec<Size> = dialog
            .actions
            .iter()
            .map(|action| action.button.size(advance, line_height))
            .collect();
        let gaps = count_to_f32(action_sizes.len().saturating_sub(1));
        let row_width: f32 =
            action_sizes.iter().map(|size| size.width).sum::<f32>() + gaps * THEME_SPACING_SM;
        let row_height = action_sizes
            .iter()
            .map(|size| size.height)
            .fold(0.0, f32::max);

        // The inner width is the panel's width less its padding, and **never less
        // than the action row**. The row is measured first for exactly that
        // reason: a panel narrower than its own buttons would draw them outside
        // itself, which is the same defect as a button hit-tested somewhere it is
        // not drawn. Both the text and the row are then placed in this one width,
        // so the width a line wraps to and the box it is drawn in cannot disagree.
        let inner_width = (rect.width.min(PANEL_MAX_WIDTH) - PANEL_PADDING * 2.0)
            .max(row_width)
            .max(0.0);
        let max_height = (rect.height * PANEL_MAX_HEIGHT_FRACTION).max(0.0);

        let title_text = dialog.title.get();
        let body_text = dialog.body.get();
        let title_present = !title_text.is_empty();
        let body_present = !body_text.is_empty();
        let row_present = !action_sizes.is_empty();

        let title = text_layout(&title_text, inner_width, None, line_height, advance);
        let title_height = if title_present {
            title.total_height
        } else {
            0.0
        };

        // The height the body may take: the panel's own maximum less the padding,
        // the title and the action row, and less one gap for each of the two
        // places the body sits beside another block.
        let mut gaps_around_body = 0_usize;
        if title_present && body_present {
            gaps_around_body += 1;
        }
        if body_present && row_present {
            gaps_around_body += 1;
        }
        let body_max = (max_height
            - PANEL_PADDING * 2.0
            - title_height
            - row_height
            - count_to_f32(gaps_around_body) * THEME_SPACING_SM)
            .max(0.0);
        let body = text_layout(
            &body_text,
            inner_width,
            Some(body_max),
            line_height,
            advance,
        );
        let body_height = if body_present { body.total_height } else { 0.0 };

        let blocks =
            usize::from(title_present) + usize::from(body_present) + usize::from(row_present);
        let content_height = title_height
            + body_height
            + row_height
            + count_to_f32(blocks.saturating_sub(1)) * THEME_SPACING_SM;
        // Floored at the padding rather than at zero, so the inner box is never
        // a negative height however small the caller's box is.
        let panel_height = (PANEL_PADDING * 2.0 + content_height)
            .min(max_height)
            .max(PANEL_PADDING * 2.0);
        let panel_width = inner_width + PANEL_PADDING * 2.0;

        let rest = Rect::new(
            rect.x + (rect.width - panel_width) / 2.0,
            rect.y + (rect.height - panel_height) / 2.0,
            panel_width,
            panel_height,
        );
        let inner = Rect::new(
            rest.x + PANEL_PADDING,
            rest.y + PANEL_PADDING,
            inner_width,
            panel_height - PANEL_PADDING * 2.0,
        );

        // Right-aligned, which is where a dialog's actions sit: the eye has
        // already read the title above them, and the row is short.
        let mut x = inner.x + (inner_width - row_width).max(0.0);
        let top = rest.y + rest.height - PANEL_PADDING - row_height;
        let actions: Vec<Rect> = action_sizes
            .iter()
            .map(|size| {
                let drawn = Rect::new(x, top, size.width, size.height);
                x += size.width + THEME_SPACING_SM;
                drawn
            })
            .collect();

        let title_top = 0.0;
        let body_top = title_height
            + if title_present && body_present {
                THEME_SPACING_SM
            } else {
                0.0
            };

        let scale = dialog.scale.get();
        Geometry {
            rest,
            drawn: scaled(rest, scale),
            scale,
            inner,
            title,
            body,
            title_top,
            body_top,
            title_present,
            body_present,
            actions,
            line_height,
        }
    }

    /// Returns `x` as it is drawn, given where it was at rest.
    fn map_x(&self, x: f32) -> f32 {
        self.drawn.x + (x - self.rest.x) * self.scale
    }

    /// Returns `y` as it is drawn, given where it was at rest.
    fn map_y(&self, y: f32) -> f32 {
        self.drawn.y + (y - self.rest.y) * self.scale
    }

    /// Returns the index of the action whose drawn rect holds `position`, or
    /// `None` when no action does.
    ///
    /// The actions are tested against the same numbers
    /// [`Dialog::action_rect`] hands out and the same ones they are drawn with,
    /// in order, so a tap is one action's and not a neighbour's. It lives here
    /// rather than on the dialog so that the tap, the accessor and the drawing
    /// cannot each measure it for themselves.
    fn action_at(&self, position: Offset) -> Option<usize> {
        self.actions
            .iter()
            .position(|action| contains(self.drawn_rect(*action), position))
    }

    /// Returns `rect` as it is drawn, given where it was at rest.
    fn drawn_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            self.map_x(rect.x),
            self.map_y(rect.y),
            rect.width * self.scale,
            rect.height * self.scale,
        )
    }
}

/// The motion a dialog's show and hide transitions run on when its caller has
/// said nothing.
///
/// The duration is [`DurationNormal`](ThemeToken::DurationNormal) and the easing
/// is [`EasingStandard`](ThemeToken::EasingStandard). **Deliberately not
/// [`Motion::from_theme`]**, which reads [`DurationFast`](ThemeToken::DurationFast)
/// — 150 ms, the length of a press. A four-arc bounce in 150 ms gives each arc
/// 54 ms, which reads as a twitch rather than as an arrival.
///
/// A dialog is a *default* transition rather than a slow one:
/// [`DurationSlow`](ThemeToken::DurationSlow) is documented for *a sheet*, and a
/// sheet travels across the screen while a centred dialog does not move at all,
/// so 500 ms of bounce in the middle of a still screen reads as a stall.
///
/// The panel's **scale** does not use this easing. Requirement 4 asks for a
/// bounce, [`Easing::Bounce`] is a theme-independent constant rather than a
/// token, and the overlay's fade uses this one.
///
/// What would reverse it: a token for "the duration a modal takes to appear",
/// which is what a platform's own specifications call this and which would let
/// a theme slow the appearance without changing `DurationNormal` for everything
/// else.
fn default_motion() -> Motion {
    Motion {
        duration: Duration::from_millis(300),
        easing: Easing::EaseInOut,
    }
}

/// Lays one block of the panel's text out in `max_width`, with no more than
/// `max_height` of lines when that is given.
///
/// Word wrapping, so a paragraph breaks at spaces, and an ellipsis on whatever
/// does not fit — vertically as well as horizontally, because the height cap is
/// what keeps a long body's action row on screen.
fn text_layout(
    text: &str,
    max_width: f32,
    max_height: Option<f32>,
    line_height: f32,
    advance: &dyn Fn(char) -> f32,
) -> TextLayout {
    layout_text(
        text,
        &LayoutOptions {
            max_width,
            max_height,
            line_height,
            letter_spacing: 0.0,
            align: TextAlign::Left,
            wrap: WrapMode::Word,
            truncation: Truncation::Ellipsis,
        },
        advance,
    )
}

/// Returns how far along its own scale the panel has come, from `0.0` to `1.0`.
///
/// **Derived from the scale rather than held as a property of its own**, for the
/// same reason [`Button::style`](crate::widgets::button::Button) derives a press
/// from the scale: the scale is what the transition animates, and a second
/// property would be a second number to keep in step with it. It is what fades
/// the panel and its shadow. The overlay has an opacity of its own because the
/// overlay's is the task's 0.0 to 0.5 and the panel's is not a number anybody
/// named.
///
/// A scale above its resting value — which an overshooting easing would ask for —
/// is no more than fully shown.
fn shown_fraction(scale: f32) -> f32 {
    ((scale - MIN_SCALE) / (1.0 - MIN_SCALE)).clamp(0.0, 1.0)
}

/// Returns `rect` scaled about its own centre.
fn scaled(rect: Rect, scale: f32) -> Rect {
    let width = rect.width * scale;
    let height = rect.height * scale;
    Rect::new(
        rect.x + (rect.width - width) / 2.0,
        rect.y + (rect.height - height) / 2.0,
        width,
        height,
    )
}

/// Returns whether `point` lies inside `rect`, edges included.
fn contains(rect: Rect, point: Offset) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

/// Returns `color` drawn at `opacity`.
///
/// Interpolating toward transparent black *is* the premultiplied fade: every
/// channel arrives at zero with the alpha, so a colour never bleeds what is
/// under it. `opacity` is clamped, because a caller that has lost track of the
/// scale cannot ask for a colour outside the segment.
///
/// This is [`button`](crate::widgets::button)'s own `with_opacity`, repeated
/// rather than imported for the reason its `count_to_f32` gives: a shared home
/// would be a new function on [`Color`], and `property.rs` is not this widget's
/// to change.
fn faded(color: Color, opacity: f32) -> Color {
    Color::interpolate(
        &color,
        &Color::new(0, 0, 0, 0),
        (1.0 - opacity).clamp(0.0, 1.0),
    )
}

/// Returns `value` as a channel value, rounded and clamped to `0..=255`.
///
/// There is no `From`/`TryFrom` between `f32` and any integer type in std, so
/// this is the one float-to-integer `as` cast in the module, for the same reason
/// and with the same guarantee as [`animation`](crate::animation)'s and
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
/// one place the cast happens, for the reason
/// [`keyboard`](crate::widgets::keyboard)'s own `count_to_f32` gives. The
/// conversion is well defined for every `usize`: the result rounds to the
/// nearest `f32`, and a dialog with more action buttons than that rounding
/// matters for is not a dialog this arena can draw.
fn count_to_f32(count: usize) -> f32 {
    count as f32
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// [`keyboard`](crate::widgets::keyboard)'s own `token_color`, repeated for the
/// same reason `faded` above is: a shared home would be a change to `theme.rs`.
fn token_color(theme: &Theme, token: ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;
    use crate::layout::Rect as LayoutRect;

    /// A character's advance, in pixels. A whole number, so every width in these
    /// fixtures is one too and every expected rect can be written out rather
    /// than derived from the constants under test.
    const ADVANCE: f32 = 7.0;

    /// The line box every line in these tests is drawn in.
    const LINE_HEIGHT: f32 = 20.0;

    /// The title every fixture is built with: 7 characters, 49 pixels wide.
    const TITLE: &str = "Confirm";

    /// The body every fixture is built with: 18 characters, 126 pixels wide,
    /// which is inside the panel's 372-pixel inner width and so does not wrap.
    const BODY: &str = "Discard the draft?";

    /// The box the dialogs below are laid out in.
    ///
    /// **Not at the origin**, deliberately, for the reason `.ai/NEVERAGAIN.md` §
    /// *A rect's origin and a rect's extent are different numbers* gives: a
    /// fixture at `(0, 0)` cannot see a coordinate being read as a size.
    const SCREEN: Rect = Rect {
        x: 40.0,
        y: 30.0,
        width: 1000.0,
        height: 700.0,
    };

    /// The panel's rect in [`SCREEN`] for the fixture below, written out.
    ///
    /// `1000 - 420` of slack either side of a 420-wide panel and `700 - 148` of
    /// it either side of a 148-tall one: `40 + 290` and `30 + 276`. The height is
    /// the 48 of padding, the title's 20, the 8 gap, the body's 20, the 8 gap and
    /// the 44 of the action row.
    const PANEL: Rect = Rect {
        x: 330.0,
        y: 306.0,
        width: 420.0,
        height: 148.0,
    };

    /// The first action's rect in [`SCREEN`], right-aligned against the panel's
    /// inner right edge at `330 + 420 - 24 = 726`: 44 plus 58 plus an 8-pixel gap
    /// is 110, so the row starts at 616.
    const OK: Rect = Rect {
        x: 616.0,
        y: 386.0,
        width: 44.0,
        height: 44.0,
    };

    /// The second action's rect: 8 past the first, and 58 wide because "Cancel"
    /// is 6 characters of 7 plus 16 of padding, over the 44-pixel touch floor.
    const CANCEL: Rect = Rect {
        x: 668.0,
        y: 386.0,
        width: 58.0,
        height: 44.0,
    };

    /// The whole of the default show transition, so a fixture can arrive at the
    /// shown state with one tick.
    const TRANSITION: Duration = Duration::from_millis(300);

    /// How far two derived numbers may differ in these tests.
    ///
    /// Every expectation here is arithmetic in `f32` — a closed form from
    /// [`Easing::apply`], a sum of sizes — and `f32` has 24 bits of mantissa, so
    /// the last bit is not a promise anybody can make. [`blur`](crate::render::blur)
    /// states the same reason for the same constant.
    const EPSILON: f32 = 1e-5;

    /// The advance measurement every fixture passes.
    fn advance(_: char) -> f32 {
        ADVANCE
    }

    /// Creates a dialog with no actions and no transition running.
    fn dialog(nodes: &mut Arena<WidgetNode>) -> Dialog {
        Dialog::new(nodes, TITLE, BODY)
    }

    /// Creates a dialog with `OK` and `Cancel`, presented and finished, so a
    /// fixture is in the state the tests are about rather than the state a fresh
    /// widget is in.
    fn shown(nodes: &mut Arena<WidgetNode>) -> Dialog {
        let mut dialog = dialog(nodes);
        for label in ["OK", "Cancel"] {
            let action = DialogAction::new(nodes, label);
            dialog.add_action(nodes, action);
        }
        present(&dialog);
        dialog
    }

    /// Presents `dialog` and runs its show transition to the end.
    fn present(dialog: &Dialog) {
        assert!(dialog.present(), "and it was not showing already");
        finish(dialog);
    }

    /// Runs `dialog`'s transition by three whole spans, which is one span of
    /// slack over [`TRANSITION`].
    fn finish(dialog: &Dialog) {
        for _ in 0..3 {
            let _ = dialog.tick(TRANSITION);
        }
    }

    /// Adds an action to `dialog` whose callback counts into `hits`, and returns
    /// the counter.
    ///
    /// The callback is written **before** the action is handed over, because that
    /// is when [`Dialog::add_action`] wraps it in the dismissal.
    fn counting_action(
        dialog: &mut Dialog,
        nodes: &mut Arena<WidgetNode>,
        label: &str,
    ) -> Rc<Cell<i32>> {
        let hits = Rc::new(Cell::new(0));
        let counted = Rc::clone(&hits);
        let mut action = DialogAction::new(nodes, label);
        action.button.on_click = Callback::new(move || counted.set(counted.get() + 1));
        dialog.add_action(nodes, action);
        hits
    }

    /// Returns the index of the first command that is `what`, or panics naming
    /// the commands that were recorded.
    ///
    /// The **index** and not a count: a count is satisfied by the same three
    /// commands in any of six orders, which is how an ordering requirement goes
    /// untested.
    fn index_of(commands: &[DrawCommand], what: &str) -> usize {
        commands
            .iter()
            .position(|command| match what {
                "overlay" => matches!(command, DrawCommand::Rect { .. }),
                "shadow" => matches!(command, DrawCommand::Shadow { .. }),
                "panel" => matches!(command, DrawCommand::RoundedRect { .. }),
                other => panic!("{other} is not a kind of command this helper knows"),
            })
            .unwrap_or_else(|| panic!("no {what} among {commands:?}"))
    }

    /// Returns the panel's rounded rectangle: the **first** one.
    ///
    /// Not the only one — each action's button is a rounded rectangle as well,
    /// which is why the ordering test below says where they land rather than
    /// this counting them.
    fn panel_of(commands: &[DrawCommand]) -> (Rect, f32, Color) {
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
            .unwrap_or_else(|| panic!("no panel among {commands:?}"))
    }

    /// Returns the rounded rectangles after the first, which are the actions'.
    fn rounded_after_panel(commands: &[DrawCommand]) -> Vec<(Rect, f32, Color)> {
        let mut seen_panel = false;
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color,
                } => {
                    if seen_panel {
                        Some((*rect, *radius, *color))
                    } else {
                        seen_panel = true;
                        None
                    }
                }
                _ => None,
            })
            .collect()
    }

    /// Returns the shadow's rect, colour, blur and offset, or panics.
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

    /// Returns the overlay's rect and colour, or panics.
    fn overlay_of(commands: &[DrawCommand]) -> (Rect, Color) {
        commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Rect { rect, color } => Some((*rect, *color)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no overlay among {commands:?}"))
    }

    /// Returns the texts the dialog recorded, in order, with their colours and
    /// the top of each line's box.
    fn text_of(commands: &[DrawCommand]) -> Vec<(String, Color, f32)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, y, .. } => Some((text.clone(), *color, *y)),
                _ => None,
            })
            .collect()
    }

    /// Returns the face each recorded text run asks for, in order.
    ///
    /// The whole of what a bold title **is** on this side of the pipeline: a
    /// [`FontWeight`] on the command, resolved by the renderer against the faces
    /// it holds. A count of commands could not tell a bold title from a regular
    /// one, which is why this is the weights and not the number.
    fn weights_of(commands: &[DrawCommand]) -> Vec<FontWeight> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { weight, .. } => Some(*weight),
                _ => None,
            })
            .collect()
    }

    /// Returns one text run's every field but its weight, or panics.
    ///
    /// The five numbers and the string, so a test can compare a recorded title
    /// against a run it builds itself and say **which** fields moved — which is
    /// the claim [`Painter::text_bold`] makes and the one a title needs: same
    /// text, same place, same colour, same size, and one field different.
    #[allow(clippy::type_complexity)]
    fn run_of(commands: &[DrawCommand], index: usize) -> (f32, f32, String, Color, f32, f32) {
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
            .nth(index)
            .unwrap_or_else(|| panic!("no text run {index} among {commands:?}"))
    }

    /// Returns the commands a `Painter` records for the same run, as
    /// [`run_of`] reads them.
    ///
    /// Built through the **regular** painter on purpose: the comparison a test
    /// wants is between what the dialog recorded and what the same run in the
    /// regular face would be, so that the weight is the only field left over.
    fn regular_run(
        x: f32,
        y: f32,
        text: &str,
        color: Color,
        font_size: f32,
    ) -> (f32, f32, String, Color, f32, f32) {
        let mut painter = Painter::new();
        painter.text(x, y, text, color, font_size, 0.0);
        let commands = painter.finish();
        assert_eq!(weights_of(&commands), vec![FontWeight::Regular]);
        run_of(&commands, 0)
    }

    /// A tap at `point`.
    fn tap_at(point: Offset) -> InputEvent {
        InputEvent::new(InputEventKind::Tap, Some(point))
    }

    /// A key press of `keycode`, with no position.
    fn key_down(keycode: sdl3::keyboard::Keycode) -> InputEvent {
        InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(keycode),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        )
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

    #[test]
    fn a_dialog_holds_the_properties_the_task_gives_it() {
        let mut nodes = Arena::new();
        let dialog = Dialog::new(&mut nodes, "Title", "Body");

        assert_eq!(dialog.title.get(), "Title");
        assert_eq!(dialog.body.get(), "Body");
        assert!(!dialog.visible.get(), "and it starts hidden");
        assert!(dialog.actions.is_empty(), "with no actions yet");

        assert!(nodes.get(dialog.handle()).is_some(), "in a node of its own");
        assert_eq!(
            nodes.get(dialog.handle()).unwrap().parent(),
            None,
            "which is a root of its own: a dialog overlays everything, so it \
             is not laid out inside the content it covers"
        );
        assert!(
            nodes.get(dialog.handle()).unwrap().layout().is_dirty(),
            "and the pass has to place it, because it is the whole overlay"
        );
    }

    #[test]
    fn an_action_is_a_button_node_attached_to_the_dialog() {
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        let action = DialogAction::new(&mut nodes, "OK");

        assert_eq!(action.button.label.get(), "OK");
        assert!(nodes.get(action.handle()).is_some(), "its own node");
        assert_eq!(nodes.get(action.handle()).unwrap().parent(), None);

        dialog.add_action(&mut nodes, action);
        assert_eq!(dialog.actions.len(), 1);
        assert_eq!(
            nodes.get(dialog.actions[0].handle()).unwrap().parent(),
            Some(dialog.handle()),
            "and attached to the dialog, so a tap inside the panel reaches it"
        );
    }

    #[test]
    fn the_panel_is_the_fixture_it_was_told_to_be() {
        // The rest of this module's fixtures are written from these three
        // literals, so they are pinned here in one place and nowhere else.
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);

        assert_eq!(dialog.panel_rect(SCREEN, &advance, LINE_HEIGHT), PANEL);
        assert_eq!(
            dialog.action_rect(0, SCREEN, &advance, LINE_HEIGHT),
            Some(OK)
        );
        assert_eq!(
            dialog.action_rect(1, SCREEN, &advance, LINE_HEIGHT),
            Some(CANCEL)
        );
        assert_eq!(dialog.action_rect(2, SCREEN, &advance, LINE_HEIGHT), None);
    }

    #[test]
    fn paint_records_the_overlay_the_shadow_and_the_panel_in_that_order() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        let commands = dialog.paint(SCREEN, &advance, LINE_HEIGHT);

        let overlay = index_of(&commands, "overlay");
        let shadow = index_of(&commands, "shadow");
        let panel = index_of(&commands, "panel");

        assert!(
            overlay < shadow,
            "the shadow is composited between the overlay and the panel, so it \
             is recorded after the overlay it falls on and before the panel that \
             casts it; got {overlay} then {shadow}"
        );
        assert!(
            shadow < panel,
            "and the panel after it, or the panel is not on top; got {shadow} \
             then {panel}"
        );

        // And the panel is drawn before the text and before the buttons that
        // sit on it, which the three commands above say nothing about.
        let first_text = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { .. }))
            .unwrap_or_else(|| panic!("no text among {commands:?}"));
        let buttons = rounded_after_panel(&commands);
        assert_eq!(buttons.len(), 2, "the fixture's two buttons");
        assert!(
            panel < first_text,
            "the panel at {panel} then the text at {first_text}"
        );
        assert_eq!(
            buttons[0].0, OK,
            "the first of them is where the fixture says the OK is drawn"
        );
        assert_eq!(buttons[1].0, CANCEL, "and the next the Cancel");
    }

    #[test]
    fn the_shadow_is_the_panels_own_shape_blurred_and_offset_below_it() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        dialog.border_radius.set(20.0);
        let commands = dialog.paint(SCREEN, &advance, LINE_HEIGHT);
        let (rect, radius, color, blur, offset) = shadow_of(&commands);
        let (panel, panel_radius, _) = panel_of(&commands);

        assert_eq!(rect, panel, "the same rect the panel is drawn in");
        assert_eq!(radius, panel_radius, "and the same radius, from one number");
        assert_eq!(blur, 8.0, "the Gaussian's standard deviation");
        assert_eq!(offset, (0.0, 8.0), "falling below the panel");
        assert_eq!(
            color,
            Color::new(0, 0, 0, 128),
            "and black at SHADOW_ALPHA of 255"
        );

        // And the panel's radius is one property with two readers: a change to
        // it that missed the shadow would leave a shadow that is not the panel's
        // silhouette.
        assert_eq!(panel_radius, 20.0, "so both moved together");
    }

    #[test]
    fn the_overlay_is_black_at_half_alpha_over_the_whole_box() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        let commands = dialog.paint(SCREEN, &advance, LINE_HEIGHT);
        let (rect, color) = overlay_of(&commands);

        assert_eq!(rect, SCREEN, "over the whole of the box it was given");
        assert_eq!(
            color,
            Color::new(0, 0, 0, level(MAX_OVERLAY_ALPHA * 255.0)),
            "and black at the task's 0.5"
        );
        assert_eq!(color.a, 128, "which is 128 of 255");
    }

    #[test]
    fn a_hidden_dialog_records_nothing() {
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        for label in ["OK", "Cancel"] {
            let action = DialogAction::new(&mut nodes, label);
            dialog.add_action(&mut nodes, action);
        }

        assert!(
            dialog.paint(SCREEN, &advance, LINE_HEIGHT).is_empty(),
            "not a transparent overlay and not a transparent panel: nothing"
        );

        // And a dialog that was presented and dismissed again stops recording
        // once its transition has run out, rather than leaving a transparent
        // panel on screen for ever.
        present(&dialog);
        assert!(!dialog.paint(SCREEN, &advance, LINE_HEIGHT).is_empty());
        let _ = dialog.dismiss();
        finish(&dialog);
        assert!(
            dialog.paint(SCREEN, &advance, LINE_HEIGHT).is_empty(),
            "and once the fade out has finished there is nothing left to draw"
        );
    }

    #[test]
    fn the_panel_is_centred_in_the_rect_it_is_given() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        let (rect, _, _) = panel_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT));

        assert_eq!(rect, PANEL, "the whole panel, from literals");
        assert_eq!(
            dialog.panel_rect(SCREEN, &advance, LINE_HEIGHT),
            PANEL,
            "and the accessor agrees with the drawing"
        );

        // Off-centre on purpose, and asymmetrically, so a panel centred in one
        // axis only still fails.
        let narrow = Rect::new(11.0, 5.0, 613.0, 411.0);
        let drawn = dialog.panel_rect(narrow, &advance, LINE_HEIGHT);
        assert_eq!(drawn.width, PANEL.width, "the same panel, in a smaller box");
        assert_eq!(
            drawn.x,
            narrow.x + (narrow.width - drawn.width) / 2.0,
            "the slack is split evenly across"
        );
        assert_eq!(
            drawn.y,
            narrow.y + (narrow.height - drawn.height) / 2.0,
            "and down"
        );
    }

    #[test]
    fn the_title_and_the_body_are_recorded_in_the_panels_colours_the_title_first() {
        let mut nodes = Arena::new();
        let mut dialog = shown(&mut nodes);
        dialog.set_palette(Palette {
            surface: Color::new(11, 22, 33, 255),
            title: Color::new(1, 2, 3, 255),
            body: Color::new(4, 5, 6, 255),
        });

        let commands = dialog.paint(SCREEN, &advance, LINE_HEIGHT);
        let (rect, radius, surface) = panel_of(&commands);
        assert_eq!(rect, PANEL);
        assert_eq!(radius, 16.0, "BorderRadiusLg");
        assert_eq!(
            surface,
            Color::new(11, 22, 33, 255),
            "in the palette's surface"
        );

        let text = text_of(&commands);
        assert_eq!(
            text.iter()
                .map(|(line, ..)| line.as_str())
                .collect::<Vec<_>>(),
            vec!["Confirm", "Discard the draft?", "OK", "Cancel"],
            "the title, then the body, then the action row's own labels"
        );
        assert_eq!(text[0].1, Color::new(1, 2, 3, 255), "the title's colour");
        assert_eq!(text[1].1, Color::new(4, 5, 6, 255), "and the body's");
        assert!(
            text[0].2 < text[1].2,
            "and the title above it: {} then {}",
            text[0].2,
            text[1].2
        );
        // 330 + 24 of padding is the inner left edge and 306 + 24 the inner top
        // edge, so the title's line box starts there and the body's one gap
        // lower.
        assert_eq!(text[0].2, 330.0, "the title on the panel's first line box");
        assert_eq!(text[1].2, 358.0, "and the body one gap below it");

        let sizes: Vec<f32> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { font_size, .. } => Some(*font_size),
                _ => None,
            })
            .collect();
        assert_eq!(
            sizes,
            vec![18.0, 14.0, 14.0, 14.0],
            "the title at the theme's FontSizeLg and the body at its FontSizeMd; \\
             the action labels are each button's own default, not this widget's"
        );
    }

    /// Requirement 2's *"Title: bold text at top of panel"*, on the recorded
    /// commands.
    ///
    /// **The weights and not the count of runs**, because the title and the body
    /// were already the same two commands and the same two strings before the
    /// weight existed: what changed is one field on the first of them, and a
    /// count is blind to that by construction. The two action labels are in the
    /// list beside the body's because they are runs too — a caller reading the
    /// command list has to be able to see which runs are this widget's bold and
    /// which are a button's own label, and a button's label is regular whatever
    /// the panel around it is doing.
    #[test]
    fn the_title_is_recorded_bold_and_the_body_in_the_regular_face() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);

        let weights = weights_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT));
        assert_eq!(
            weights,
            vec![
                FontWeight::Bold,    // the title
                FontWeight::Regular, // the body
                FontWeight::Regular, // "OK", the action button's own label
                FontWeight::Regular, // "Cancel", likewise
            ],
            "so the title is the only run in the panel that asks for the bold face"
        );
        assert_eq!(
            weights.len(),
            text_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT)).len(),
            "and every run the dialog records is accounted for, so the four above \
             are all of them"
        );

        // And the title's weight travels with the *line*, so a title that wraps
        // is bold on both lines rather than on the first only.
        dialog.title.set(String::from(
            "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk",
        ));
        let wrapped = weights_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT));
        assert_eq!(
            wrapped[..2],
            [FontWeight::Bold, FontWeight::Bold],
            "the eleven four-character words are 378 pixels against the panel's \
             372-pixel inner width, so the title is two lines"
        );
        assert!(
            wrapped[2..]
                .iter()
                .all(|weight| *weight == FontWeight::Regular),
            "and the body below it is not: {wrapped:?}"
        );
    }

    /// The bold title is a regular run with **one field changed**, and not a
    /// second block of text, a second measurement or a second position.
    ///
    /// Each recorded run is compared against the same run built through the
    /// **regular** painter, field by field. That is the claim
    /// [`Painter::text_bold`] makes, asked of this widget's own output: if a
    /// title were drawn at a different size, in a different colour, at a
    /// different y, or with a tracking value of its own, this fails — and each
    /// of those *would* move the panel, because `Geometry` reads the title's
    /// layout and its top offset.
    #[test]
    fn the_title_is_a_regular_run_with_one_field_changed() {
        let mut nodes = Arena::new();
        let mut dialog = shown(&mut nodes);
        dialog.set_palette(Palette {
            surface: Color::new(11, 22, 33, 255),
            title: Color::new(1, 2, 3, 255),
            body: Color::new(4, 5, 6, 255),
        });
        let commands = dialog.paint(SCREEN, &advance, LINE_HEIGHT);

        // Run 0 is the title, run 1 the body, and the two action labels are each
        // a `Button`'s own paint rather than anything this module records.
        for (index, what) in [(0, "the title"), (1, "the body")] {
            let (x, y, text, color, font_size, extra) = run_of(&commands, index);
            assert_eq!(
                (x, y, text.clone(), color, font_size, extra),
                regular_run(x, y, &text, color, font_size),
                "{what} carries its own position, text, colour, size and tracking \
                 unchanged, and only the weight differs"
            );
            // The colours really are the palette's, so the comparison above is
            // between two runs that would look different if anything but the
            // face had moved.
            assert_eq!(
                color,
                if index == 0 {
                    Color::new(1, 2, 3, 255)
                } else {
                    Color::new(4, 5, 6, 255)
                },
                "{what} is in the palette's own colour"
            );
        }
        assert_eq!(
            (run_of(&commands, 0).1, run_of(&commands, 1).1),
            (330.0, 358.0),
            "and the title's line box is still the panel's first one and the \
             body's one gap below it: the bold face moved neither"
        );
    }

    /// **A bold title does not move the panel, and this is why that is a
    /// statement about the code rather than a hope.**
    ///
    /// [`Geometry`] measures the title with the caller's `advance` closure — the
    /// same one it measures the body and the button labels with — so the weight,
    /// which travels only on the recorded command, is not an input to the panel's
    /// size. The first assertion is the claim. The other two are its **control**:
    /// a fixture that did not move would be satisfied by a measurement that never
    /// read the title at all, and this asserts that a title one word longer moves
    /// the panel by exactly one line box and recentres it.
    ///
    /// What a caller must **not** read into it is that a bold run ends at the
    /// same `x`: that is this widget's measurement being weight-blind, where the
    /// glyphs on the screen are not.
    #[test]
    fn the_bold_title_leaves_the_panels_geometry_where_it_was() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);

        // The claim: the fixture's literals, from a title drawn bold.
        assert_eq!(
            dialog.panel_rect(SCREEN, &advance, LINE_HEIGHT),
            PANEL,
            "the panel is the same 420 by 148 box the module's fixtures were \
             written against, with a bold title"
        );
        assert_eq!(
            dialog.action_rect(0, SCREEN, &advance, LINE_HEIGHT),
            Some(OK)
        );
        assert_eq!(
            dialog.action_rect(1, SCREEN, &advance, LINE_HEIGHT),
            Some(CANCEL),
            "and the action row is in the same place, so a tap lands where the \
             button is painted"
        );

        // Control one: **ten** four-character words is 343 pixels against the
        // panel's 372-pixel inner width, so this title is one line and a much
        // wider one than "Confirm" — and the panel does not move, because its
        // width is capped at `PANEL_MAX_WIDTH` and its height is its lines.
        let one_line = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj";
        dialog.title.set(String::from(one_line));
        assert_eq!(
            dialog
                .paint(SCREEN, &advance, LINE_HEIGHT)
                .iter()
                .filter(|command| matches!(command, DrawCommand::Text { .. }))
                .count(),
            4,
            "the wider title is still one line: it, the body and the two labels"
        );
        assert_eq!(
            dialog.panel_rect(SCREEN, &advance, LINE_HEIGHT),
            PANEL,
            "so the panel is unmoved, which is the point of measuring at all"
        );

        // Control two: **eleven** of those words is 378 pixels, which does not
        // fit, so the title wraps and the panel grows by exactly one line box —
        // and is centred again, so it moves up by half of what it grew.
        dialog.title.set(String::from(&format!("{one_line} kkkk")));
        let taller = dialog.panel_rect(SCREEN, &advance, LINE_HEIGHT);
        assert_eq!(
            taller.height,
            PANEL.height + LINE_HEIGHT,
            "the panel is one line box taller: {taller:?}"
        );
        assert_eq!(
            taller.y,
            SCREEN.y + (SCREEN.height - taller.height) / 2.0,
            "and centred in the box again, which is what makes the growth visible \
             as a move rather than as a stretch"
        );
        assert!(
            weights_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT))[..2]
                == [FontWeight::Bold, FontWeight::Bold],
            "and the title is bold on both of its lines while the panel measures \
             both of them at the caller's regular advance"
        );
    }

    #[test]
    fn the_body_wraps_to_the_panels_inner_width() {
        let dialog_inner = PANEL.width - PANEL_PADDING * 2.0;
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        assert_eq!(dialog_inner, 372.0, "the width this fixture wraps to");

        // The control: ten four-character words with a space between are
        // `10 * 28 + 9 * 7` = 343 pixels, which fits in 372 on one line.
        let fits = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj";
        dialog.body.set(String::from(fits));
        assert_eq!(
            text_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT))[1].0,
            fits,
            "one line while it fits"
        );

        // Four more words: `14 * 28 + 13 * 7` = 483, which does not. Word wrap
        // packs words until the next would overflow, and the eleventh word takes
        // the line to 343 while one more would be 378 — so the first line holds
        // exactly ten words and the rest go below it.
        dialog
            .body
            .set(String::from(&format!("{fits} kkkk llll mmmm nnnn")));
        let text = text_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT));
        let body: Vec<&String> = text[1..3].iter().map(|(line, ..)| line).collect();
        assert_eq!(
            body.len(),
            2,
            "a body wider than the panel's inner width is wrapped, not drawn \
             off it: {} lines among {:?}",
            body.len(),
            text.iter().map(|(line, ..)| line).collect::<Vec<_>>()
        );
        assert_eq!(body[0], fits, "the ten words that fit, and no more");
        assert_eq!(body[1], "kkkk llll mmmm nnnn", "and the four that did not");
        for line in &body {
            assert!(
                line.chars().count() as f32 * ADVANCE <= dialog_inner,
                "{line:?} is {} pixels wide and the inner box is {dialog_inner}",
                line.chars().count() as f32 * ADVANCE
            );
        }
    }

    #[test]
    fn a_body_too_tall_for_the_panel_is_cut_short_rather_than_pushing_the_actions_off_screen() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        // Forty lines of body: 800 pixels against a 700-pixel box, so an
        // uncapped panel would be 928 tall and its action row 228 pixels below
        // the bottom of the window.
        dialog.body.set("a line\n".repeat(40));
        let panel = dialog.panel_rect(SCREEN, &advance, LINE_HEIGHT);

        assert!(
            panel.y + panel.height <= SCREEN.y + SCREEN.height,
            "the panel stays inside the box it was given: {} tall at y {}",
            panel.height,
            panel.y
        );
        let action = dialog
            .action_rect(0, SCREEN, &advance, LINE_HEIGHT)
            .expect("OK");
        assert!(
            action.y + action.height <= SCREEN.y + SCREEN.height,
            "and so does the action row, which is what has to be reachable"
        );

        let lines = text_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT)).len() - 1;
        assert!(
            lines < 40,
            "and the body was cut rather than laid out whole: {lines}"
        );
        assert!(lines > 0, "with some of it still drawn");
    }

    #[test]
    fn a_tap_on_a_button_activates_that_button_and_only_that_button() {
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        let ok_hits = counting_action(&mut dialog, &mut nodes, "OK");
        let cancel_hits = counting_action(&mut dialog, &mut nodes, "Cancel");
        present(&dialog);

        // **Where the button is drawn**, read back from the same geometry the
        // drawing uses, one pixel inside its corner.
        let first = dialog
            .action_rect(0, SCREEN, &advance, LINE_HEIGHT)
            .expect("OK");
        assert_eq!(first, OK, "and it is where the fixture says");
        let mut tap = tap_at(Offset::new(first.x + 1.0, first.y + 1.0));
        assert!(dialog.on_event(&mut tap, SCREEN, &advance, LINE_HEIGHT));
        assert!(tap.consumed(), "and it does not reach the content behind");
        assert_eq!(ok_hits.get(), 1, "the button it landed on fired");
        assert_eq!(cancel_hits.get(), 0, "and the other one did not");
        assert!(!dialog.visible.get(), "and the action dismissed the dialog");
    }

    #[test]
    fn a_tap_on_one_button_does_not_reach_its_neighbour() {
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        let ok_hits = counting_action(&mut dialog, &mut nodes, "OK");
        let cancel_hits = counting_action(&mut dialog, &mut nodes, "Cancel");
        present(&dialog);

        // One pixel left of "Cancel", which is the 8-pixel gap between the two
        // and belongs to nobody — the same miss a "tap on a button" test gets
        // wrong by aiming near one rather than at it.
        let mut between = tap_at(Offset::new(CANCEL.x - 1.0, CANCEL.y + 10.0));
        assert!(dialog.on_event(&mut between, SCREEN, &advance, LINE_HEIGHT));
        assert_eq!(ok_hits.get(), 0, "no button fired");
        assert_eq!(cancel_hits.get(), 0);
        assert!(
            dialog.visible.get(),
            "and the gap is inside the panel, so it is not a dismissal either"
        );

        // The other side of the same boundary, and then the button itself.
        let mut on_cancel = tap_at(Offset::new(CANCEL.x, CANCEL.y + 10.0));
        assert!(dialog.on_event(&mut on_cancel, SCREEN, &advance, LINE_HEIGHT));
        assert_eq!(cancel_hits.get(), 1, "the edge is the button's");
        assert_eq!(ok_hits.get(), 0, "and still not its neighbour's");
    }

    #[test]
    fn a_tap_outside_the_panel_dismisses() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);

        // Inside the box, 10 pixels in, and 290 pixels left of the panel's edge.
        let mut tap = tap_at(Offset::new(SCREEN.x + 10.0, SCREEN.y + 10.0));
        assert!(dialog.on_event(&mut tap, SCREEN, &advance, LINE_HEIGHT));
        assert!(
            tap.consumed(),
            "a tap on a modal is never left for the panel"
        );
        assert!(!dialog.visible.get(), "and the scrim closes it");
    }

    #[test]
    fn a_tap_outside_the_panel_does_nothing_when_dismiss_on_outside_tap_is_off() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        dialog.dismiss_on_outside_tap.set(false);

        let mut tap = tap_at(Offset::new(SCREEN.x + 10.0, SCREEN.y + 10.0));
        assert!(
            dialog.on_event(&mut tap, SCREEN, &advance, LINE_HEIGHT),
            "still consumed: the dialog is still modal"
        );
        assert!(
            dialog.visible.get(),
            "but a dialog that must be answered does not offer an answer that \
             is not one of its buttons"
        );

        // The other direction: the flag on, the same tap, the same result as
        // `a_tap_outside_the_panel_dismisses`.
        dialog.dismiss_on_outside_tap.set(true);
        let mut again = tap_at(Offset::new(SCREEN.x + 10.0, SCREEN.y + 10.0));
        assert!(dialog.on_event(&mut again, SCREEN, &advance, LINE_HEIGHT));
        assert!(!dialog.visible.get());
    }

    #[test]
    fn a_tap_on_the_panel_itself_is_consumed_and_does_not_dismiss() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);

        // The panel's own top-left corner, one pixel inside: the scrim's
        // complement, and the case a "tap outside" test written as "anywhere
        // that is not a button" gets wrong.
        let mut tap = tap_at(Offset::new(PANEL.x + 1.0, PANEL.y + 1.0));
        assert!(dialog.on_event(&mut tap, SCREEN, &advance, LINE_HEIGHT));
        assert!(tap.consumed(), "it does not reach what is behind");
        assert!(dialog.visible.get(), "but it is not a dismissal either");
    }

    #[test]
    fn escape_dismisses_the_dialog_and_is_consumed() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);

        let mut escape = key_down(sdl3::keyboard::Keycode::Escape);
        assert!(dialog.on_event(&mut escape, SCREEN, &advance, LINE_HEIGHT));
        assert!(escape.consumed(), "so it does not reach the panel behind");
        assert!(!dialog.visible.get(), "and the dialog is on its way out");

        // A second Escape has nothing left to dismiss.
        let mut again = key_down(sdl3::keyboard::Keycode::Escape);
        assert!(!dialog.on_event(&mut again, SCREEN, &advance, LINE_HEIGHT));
        assert!(!again.consumed());
    }

    #[test]
    fn a_key_that_is_not_escape_is_left_alone_and_not_consumed() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);

        for keycode in [
            sdl3::keyboard::Keycode::A,
            sdl3::keyboard::Keycode::Left,
            sdl3::keyboard::Keycode::Space,
            sdl3::keyboard::Keycode::Tab,
        ] {
            let mut key = key_down(keycode);
            assert!(
                !dialog.on_event(&mut key, SCREEN, &advance, LINE_HEIGHT),
                "{keycode:?} is not the dialog's to answer with nothing focused"
            );
            assert!(!key.consumed(), "so it carries on up the tree");
        }
        assert!(dialog.visible.get(), "and the dialog is still up");
    }

    #[test]
    fn an_activation_key_reaches_the_focused_action_and_only_it() {
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        let ok_hits = counting_action(&mut dialog, &mut nodes, "OK");
        let cancel_hits = counting_action(&mut dialog, &mut nodes, "Cancel");
        present(&dialog);

        let mut enter = key_down(sdl3::keyboard::Keycode::Return);
        assert!(
            !dialog.on_event(&mut enter, SCREEN, &advance, LINE_HEIGHT),
            "no action holds focus yet, so no action answers"
        );
        assert_eq!(ok_hits.get(), 0);

        dialog.actions[1].button.focused.set(true);
        let mut again = key_down(sdl3::keyboard::Keycode::Return);
        assert!(dialog.on_event(&mut again, SCREEN, &advance, LINE_HEIGHT));
        assert_eq!(cancel_hits.get(), 1, "the focused one fired");
        assert_eq!(ok_hits.get(), 0, "and the other one did not");
        assert!(!dialog.visible.get(), "and the action dismissed the dialog");
    }

    #[test]
    fn an_action_activated_through_its_own_node_still_dismisses() {
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        let hits = counting_action(&mut dialog, &mut nodes, "OK");
        present(&dialog);

        // The caller routed the activation key to the **button's** node rather
        // than to the dialog, which is what `Focus` gives it. The dismissal has
        // to survive that route, or an OK that closes nothing is a drawn control
        // with nothing behind it.
        let button = &dialog.actions[0].button;
        button.focused.set(true);
        let mut enter = key_down(sdl3::keyboard::Keycode::Return);
        assert!(button.on_event(&mut enter), "the button handled it");
        assert_eq!(hits.get(), 1, "and its callback ran");
        assert!(
            !dialog.visible.get(),
            "and the dialog closed, which is only true because the dismissal \
             lives in the callback rather than in the dialog's tap handler"
        );
    }

    #[test]
    fn a_tap_on_a_disabled_button_is_consumed_and_does_not_fire_or_dismiss() {
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        let hits = counting_action(&mut dialog, &mut nodes, "OK");
        present(&dialog);
        dialog.actions[0].button.disabled.set(true);

        let mut tap = tap_at(Offset::new(OK.x + 1.0, OK.y + 1.0));
        assert!(dialog.on_event(&mut tap, SCREEN, &advance, LINE_HEIGHT));
        assert!(tap.consumed(), "the tap was aimed at it, so it swallows it");
        assert_eq!(hits.get(), 0, "an inert control does not fire");
        assert!(dialog.visible.get(), "and does not close the dialog");
    }

    #[test]
    fn a_hidden_dialog_handles_nothing() {
        let mut nodes = Arena::new();
        let dialog = dialog(&mut nodes);

        let mut tap = tap_at(Offset::new(SCREEN.x + 10.0, SCREEN.y + 10.0));
        assert!(!dialog.on_event(&mut tap, SCREEN, &advance, LINE_HEIGHT));
        assert!(
            !tap.consumed(),
            "so a caller that has not presented it yet has nothing swallowed"
        );
        assert!(!dialog.is_drawn(), "and it draws nothing");
    }

    #[test]
    fn the_scale_runs_from_nine_tenths_to_one_and_the_overlay_from_nothing_to_a_half() {
        let mut nodes = Arena::new();
        let dialog = dialog(&mut nodes);
        assert_eq!(dialog.scale.get(), 0.9, "the hidden state it starts from");
        assert_eq!(dialog.overlay_alpha.get(), 0.0);
        assert!(dialog.present());

        // **Derived from the curve's closed form, not remembered.** The overlay
        // follows `EaseInOut` and the panel `Bounce`, both over the theme's
        // 300 ms, at 150 ms each is the midpoint `t = 0.5`:
        //
        // - overlay: `0 + 0.5 * EaseInOut.apply(0.5)`, and the curve's upper
        //   branch is `1 - 2(1 - 0.5)^2` = `1 - 0.5` = `0.5`, so `0.25`.
        // - scale: `0.9 + 0.1 * Bounce.apply(0.5)`, and `bounce(0.5)` is the
        //   second arc, `7.5625 * (0.5 - 1.5/2.75)^2 + 0.75`. The shift is
        //   exactly `-1/22`, so the arc is `7.5625/484 + 0.75 = 0.015625 + 0.75`
        //   = `0.765625` = `49/64`, and the scale is `0.9765625`.
        assert!(
            dialog.tick(Duration::from_millis(150)),
            "the first tick of a transition writes"
        );
        close("the scale half way", dialog.scale.get(), 0.976_562_5);
        close("the overlay half way", dialog.overlay_alpha.get(), 0.25);

        finish(&dialog);
        close("the scale arrived", dialog.scale.get(), 1.0);
        close(
            "the overlay arrived",
            dialog.overlay_alpha.get(),
            MAX_OVERLAY_ALPHA,
        );
        assert!(!dialog.is_animating(), "and the transition has stopped");
    }

    #[test]
    fn the_scale_and_the_overlay_return_to_the_hidden_values_on_dismiss() {
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        close("shown", dialog.scale.get(), 1.0);

        assert!(dialog.dismiss());
        assert!(
            !dialog.visible.get(),
            "the flag goes at once, so the caller stops routing to it"
        );

        // Half way out, the same closed form run from the other end. The bounce
        // is the same curve over the same span, so the panel is halfway between
        // its two sizes: a bounce touches its target and comes back, it does not
        // pass it, so nothing overshoots past `MIN_SCALE`.
        let _ = dialog.tick(Duration::from_millis(150));
        close("half out", dialog.scale.get(), 1.0 - 0.1 * 0.765_625);
        close("half out overlay", dialog.overlay_alpha.get(), 0.25);

        finish(&dialog);
        close("the scale returned", dialog.scale.get(), MIN_SCALE);
        close("the overlay returned", dialog.overlay_alpha.get(), 0.0);
        assert!(!dialog.is_animating());
    }

    #[test]
    fn the_easing_the_panel_uses_is_the_bounce_requirement_4_names() {
        // The theme has no token for a bounce, so the panel's scale cannot be a
        // themed value. This pins the one it does use rather than reading it out
        // of a run: `Bounce.apply(0.25)` is the first arc, `7.5625 * 0.0625` =
        // `0.47265625`, so a quarter of the way through a *linear* 100 ms the
        // panel is nearly half way up its scale, while the overlay is a quarter
        // of the way up its alpha.
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        dialog.set_motion(Motion {
            duration: Duration::from_millis(100),
            easing: Easing::Linear,
        });
        assert!(dialog.present());
        let _ = dialog.tick(Duration::from_millis(25));
        close(
            "the panel's curve is Bounce",
            dialog.scale.get(),
            0.9 + 0.1 * 0.472_656_25,
        );
        close(
            "the overlay's is the motion's",
            dialog.overlay_alpha.get(),
            MAX_OVERLAY_ALPHA * 0.25,
        );
    }

    #[test]
    fn the_default_motion_is_the_themes() {
        // The default is a copy of `DurationNormal` and `EasingStandard`, and a
        // copy drifts. This reads the tokens, so a theme that lengthens either
        // one fails here rather than being noticed on screen. Both themes are
        // read: they agree today, and a divergence is the thing worth catching.
        for theme in [Theme::dark(), Theme::light()] {
            assert_eq!(
                default_motion().duration,
                match theme.get(ThemeToken::DurationNormal) {
                    crate::theme::PropertyValue::Duration(duration) => duration,
                    other => panic!("DurationNormal is a {other:?}"),
                },
                "the default duration is the theme's DurationNormal"
            );
            assert_eq!(
                default_motion().easing,
                match theme.get(ThemeToken::EasingStandard) {
                    crate::theme::PropertyValue::Easing(easing) => easing,
                    other => panic!("EasingStandard is a {other:?}"),
                },
                "and the default easing is the theme's EasingStandard"
            );
        }
        assert_eq!(
            default_motion().duration,
            TRANSITION,
            "which is also the span a fixture ticks through"
        );
        assert_ne!(
            default_motion().duration,
            Motion::from_theme(&Theme::dark()).duration,
            "and deliberately not the button's DurationFast, which is half of it"
        );
    }

    #[test]
    fn the_theme_token_copies_are_the_tokens_values() {
        // Five constants in this module are copies of theme tokens. See
        // `the_default_motion_is_the_themes` for why a copy drifts.
        for theme in [Theme::dark(), Theme::light()] {
            let number = |token| {
                theme
                    .get(token)
                    .as_number()
                    .unwrap_or_else(|| panic!("{token:?} is not a number in the theme"))
            };
            assert_eq!(number(ThemeToken::SpacingLg), PANEL_PADDING);
            assert_eq!(number(ThemeToken::SpacingSm), THEME_SPACING_SM);
            assert_eq!(number(ThemeToken::BorderRadiusLg), THEME_RADIUS_LG);
            assert_eq!(number(ThemeToken::FontSizeLg), TITLE_FONT_SIZE);
            assert_eq!(number(ThemeToken::FontSizeMd), BODY_FONT_SIZE);
        }

        assert_eq!(
            level(SHADOW_ALPHA * 255.0),
            128,
            "and the shadow's channel is its fraction of 255, rounded"
        );
        assert_eq!(MIN_SCALE, 0.9, "the task's own 0.9");
        assert_eq!(MAX_OVERLAY_ALPHA, 0.5, "and its 0.5");
        assert_eq!(PANEL_MAX_HEIGHT_FRACTION, 0.8, "and four fifths of the box");
    }

    #[test]
    fn the_palette_from_a_theme_names_the_tokens_it_claims() {
        for theme in [Theme::dark(), Theme::light()] {
            let palette = Palette::from_theme(&theme);
            assert_eq!(
                palette.surface,
                token_color(&theme, ThemeToken::Surface),
                "the panel is the theme's raised surface"
            );
            assert_eq!(
                palette.title,
                token_color(&theme, ThemeToken::Text),
                "the title is the theme's own text"
            );
            assert_eq!(
                palette.body,
                token_color(&theme, ThemeToken::TextMuted),
                "and the body the muted one, so it reads under the title"
            );
            assert_ne!(
                palette.title, palette.body,
                "which really are two different colours"
            );
            assert_ne!(
                palette.surface,
                Palette::default().surface,
                "and a themed dialog is not the neutral default's"
            );
        }
    }

    #[test]
    fn a_bare_dialog_with_no_title_no_body_and_no_actions_still_draws_its_panel() {
        // The degenerate case the guards above mention: nothing to measure, so
        // the panel is its own padding and the title, the body and the row are
        // all absent rather than empty.
        let mut nodes = Arena::new();
        let dialog = Dialog::new(&mut nodes, "", "");
        present(&dialog);

        let commands = dialog.paint(SCREEN, &advance, LINE_HEIGHT);
        let (rect, _, _) = panel_of(&commands);
        assert_eq!(rect.height, 48.0, "two paddings and nothing between them");
        assert!(text_of(&commands).is_empty(), "and no text");
        assert_eq!(
            index_of(&commands, "panel"),
            index_of(&commands, "shadow") + 1,
            "the shadow is still recorded before it"
        );
    }

    #[test]
    fn a_panel_narrower_than_its_own_buttons_grows_to_hold_them() {
        // The inner width is floored at the action row, so a box narrower than
        // its own buttons widens the panel rather than drawing them outside it.
        let mut nodes = Arena::new();
        let mut dialog = dialog(&mut nodes);
        counting_action(&mut dialog, &mut nodes, "Cancel");
        present(&dialog);

        let narrow = Rect::new(40.0, 30.0, 200.0, 300.0);
        let panel = dialog.panel_rect(narrow, &advance, LINE_HEIGHT);
        let action = dialog
            .action_rect(0, narrow, &advance, LINE_HEIGHT)
            .expect("Cancel");
        assert!(
            panel.width > 200.0 - PANEL_PADDING * 2.0,
            "the panel is wider than the box allows, which is the point: {}",
            panel.width
        );
        assert!(
            action.x >= panel.x && action.x + action.width <= panel.x + panel.width,
            "and the button is inside it: {action:?} in {panel:?}"
        );
    }

    /// Returns the colours action `index` recorded: its background's alpha and its
    /// label's, plus the rect the background was drawn in so the caller can say
    /// which button it is looking at.
    ///
    /// **Found by the rect rather than by position in the command list**, because a
    /// position is a claim about ordering and the ring is a rounded rectangle too:
    /// a focused action records a ring, then a background, then a label, and the
    /// second rounded rectangle is the background only while nothing else has been
    /// added. `action_rect` is the number the widget drew into and the number the
    /// test wants to look at, so matching on it cannot pick up the wrong primitive.
    fn action_colors(
        dialog: &Dialog,
        index: usize,
        commands: &[DrawCommand],
    ) -> (Rect, Color, Color) {
        let rect = dialog
            .action_rect(index, SCREEN, &advance, LINE_HEIGHT)
            .unwrap_or_else(|| panic!("action {index}"));
        let background = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::RoundedRect {
                    rect: drawn, color, ..
                } if *drawn == rect => Some(*color),
                _ => None,
            })
            .unwrap_or_else(|| panic!("action {index}'s background among {commands:?}"));
        let label = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, color, .. }
                    if text == &dialog.actions[index].button.label.get() =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("action {index}'s label among {commands:?}"));
        (rect, background, label)
    }

    /// Returns a dialog at a `scale` whose [`shown_fraction`] is `shown`, with the
    /// transition otherwise finished.
    ///
    /// **The scale is written rather than reached through a tick**, because the
    /// whole claim is about one number on one frame: a fixture that arrived at half
    /// opacity by ticking a clock would be a claim about the easing as well as about
    /// the fade, and a change to either would move it. `shown_fraction(0.95)` is
    /// `(0.95 - 0.9) / 0.1`, which is exactly `0.5`.
    fn dialog_at_scale(nodes: &mut Arena<WidgetNode>, scale: f32) -> Dialog {
        let dialog = shown(nodes);
        dialog.scale.set(scale);
        assert!(
            (shown_fraction(scale) - 0.5).abs() < EPSILON || (scale - 1.0).abs() < EPSILON,
            "the fixture is written for half and for full only, and {scale} is \
             neither: shown_fraction is {}",
            shown_fraction(scale)
        );
        dialog
    }

    /// **The defect, on the numbers.** At half of the panel's opacity, every colour
    /// the dialog draws must be at half its own — the panel, the title, the body,
    /// **and each action's background and each action's label**.
    ///
    /// The control is the same test read at `shown == 1.0`, where every one of those
    /// alphas is 255: without it this would pass on a dialog that faded everything
    /// to nothing at all times.
    ///
    /// `.ai/NEVERAGAIN.md` § *a strength clamped to 0..=1, used directly as an
    /// effect's size* is why this asserts alpha and not the presence of a command:
    /// the buttons' commands were recorded correctly on every frame of the broken
    /// build, and every other test in this module passed while they sat at 255 over
    /// an invisible panel.
    #[test]
    fn every_colour_the_dialog_draws_fades_together() {
        let mut nodes = Arena::new();
        let full = dialog_at_scale(&mut nodes, 1.0);
        let full_commands = full.paint(SCREEN, &advance, LINE_HEIGHT);
        let (_, _, full_panel) = panel_of(&full_commands);

        // **The control: opaque at rest, every one of them.**
        assert_eq!(full_panel.a, 255, "the panel at shown 1.0");
        let full_actions: Vec<(Color, Color)> = (0..full.actions.len())
            .map(|index| {
                let (_, background, label) = action_colors(&full, index, &full_commands);
                (background, label)
            })
            .collect();
        assert_eq!(full_actions.len(), 2, "the fixture's two actions");
        for (index, (background, label)) in full_actions.iter().enumerate() {
            assert_eq!(
                (background.a, label.a),
                (255, 255),
                "action {index}'s background and label at shown 1.0"
            );
        }

        // **And the claim: half of every one of them at half.**
        let mut nodes = Arena::new();
        let half = dialog_at_scale(&mut nodes, 0.95);
        let half_commands = half.paint(SCREEN, &advance, LINE_HEIGHT);
        let (_, _, half_panel) = panel_of(&half_commands);
        assert_eq!(half_panel.a, 128, "the panel is at half, as it always was");
        let text = text_of(&half_commands);
        assert_eq!(
            (text[0].1.a, text[1].1.a),
            (128, 128),
            "and so are the title and the body"
        );

        for (index, at_full) in full_actions.iter().enumerate() {
            let (_, background, label) = action_colors(&half, index, &half_commands);
            assert_eq!(
                (background.a, label.a),
                (128, 128),
                "action {index} — {:?} — must fade with the panel it sits on, not \
                 sit at full strength over a half-transparent one",
                half.actions[index].button.label.get()
            );
            // **Derived through [`level`], the module's own rounding, rather than
            // by halving 255.** `255 * 0.5` is 127.5 and `level` rounds it to 128;
            // an integer `255 / 2` is 127, so the assertion written the obvious way
            // is off by one and fails on a build that is correct. This is the same
            // arithmetic `faded` performs, so the two cannot drift.
            assert_eq!(
                (background.a, label.a),
                (level(255.0 * 0.5), level(255.0 * 0.5)),
                "which is `level` of half the alpha the same action recorded at \
                 shown 1.0 — {}",
                at_full.0.a
            );
        }
    }

    #[test]
    fn the_shadows_blur_is_the_softest_edge_the_kernel_cap_allows() {
        // This constant's own doc used to claim that 8 buys a sixteen-tap kernel
        // either side. It does not: `blur::MAX_TAPS` is nine in total, so every
        // sigma from 2 upwards is capped at four taps either side, and **the width
        // of the kernel is the same for 2 as for 8**. What the number then chooses
        // is the weights, and this is what holds both halves of that down — the
        // capture in the doc is the third, and a reader who wants to know whether
        // the pixels agree should take their own.
        assert_eq!(
            crate::render::blur::taps_for(SHADOW_BLUR),
            4,
            "σ 8 asks for sixteen taps either side and gets four, because the \
             kernel is nine taps in total"
        );
        assert_eq!(
            2 * crate::render::blur::taps_for(SHADOW_BLUR) + 1,
            crate::render::blur::MAX_TAPS,
            "so the shadow is asking for the widest blur there is"
        );
        assert_eq!(
            crate::render::blur::taps_for(2.0),
            crate::render::blur::taps_for(SHADOW_BLUR),
            "and the same four taps a σ of 2 would get: the cap, not this number, \
             is what sets the width"
        );

        // **The weights are what 8 chooses**, and "flatter" is the whole of it: the
        // outermost tap's share of the centre's is what a soft edge is made of.
        let wide = crate::render::blur::kernel(SHADOW_BLUR);
        let tight = crate::render::blur::kernel(2.0);
        assert_eq!(wide.len(), tight.len(), "the same width, as above");
        assert!(
            wide[0] > tight[0],
            "and σ 8's outermost tap is the heavier of the two: {:.4} against \
             {:.4}, so it is the softer edge",
            wide[0],
            tight[0]
        );
        assert!(
            wide[0] > 0.1,
            "and heavy enough to matter — a tenth of the centre's weight, which is \
             what a four-pixel ramp is made of"
        );
    }

    #[test]
    fn shown_fraction_is_the_panel_s_opacity_and_nothing_more() {
        // The panel and its shadow are faded from the scale rather than from a
        // second property, so this is the whole of that mapping. A scale above
        // the resting value — which an overshooting easing would produce — is no
        // more than fully shown.
        assert_eq!(shown_fraction(MIN_SCALE), 0.0);
        close("half way up", shown_fraction(0.95), 0.5);
        assert_eq!(shown_fraction(1.0), 1.0);
        assert_eq!(shown_fraction(1.2), 1.0, "and an overshoot is clamped");
    }

    #[test]
    fn the_fading_of_a_block_is_the_scale_and_nothing_else() {
        // One block of the panel's text, faded to nothing while the dialog is
        // leaving, so the panel's opacity is not a colour the caller has to keep
        // in step.
        let mut nodes = Arena::new();
        let dialog = dialog(&mut nodes);
        present(&dialog);
        let shown = text_of(&dialog.paint(SCREEN, &advance, LINE_HEIGHT));
        assert_eq!(shown[0].1, dialog.title_color.get());

        let _ = dialog.dismiss();
        finish(&dialog);
        assert!(
            dialog.paint(SCREEN, &advance, LINE_HEIGHT).is_empty(),
            "which cannot be seen at the end, so this is the half way frame"
        );
        assert!(dialog.present());
        let _ = dialog.tick(Duration::from_millis(150));
        let fading = dialog.paint(SCREEN, &advance, LINE_HEIGHT);
        let text = text_of(&fading);
        assert_ne!(
            text[0].1, shown[0].1,
            "the title is drawn in a lighter colour"
        );
        assert!(
            text[0].1.r < shown[0].1.r,
            "by fading toward transparent black: {} then {}",
            text[0].1.r,
            shown[0].1.r
        );
    }

    #[test]
    fn hit_testing_and_drawing_read_the_same_numbers() {
        // Every control drawn here is hit-tested through the same `Geometry`,
        // and this walks a range of boxes and dialog sizes so a geometry change
        // that moves one reader without the other shows up as a mismatch rather
        // than as a green suite.
        for (screen_width, screen_height, buttons) in [
            (1000.0, 700.0, 2_usize),
            (640.0, 480.0, 2),
            (1920.0, 1080.0, 3),
            (520.0, 900.0, 1),
            (300.0, 220.0, 2),
        ] {
            let mut nodes = Arena::new();
            let mut dialog = dialog(&mut nodes);
            let hits: Vec<Rc<Cell<i32>>> = (0..buttons)
                .map(|index| {
                    counting_action(
                        &mut dialog,
                        &mut nodes,
                        if index == 0 { "OK" } else { "Cancel now" },
                    )
                })
                .collect();
            present(&dialog);

            // Off the origin every time: a box at `(0, 0)` cannot see a
            // coordinate read as a size.
            let screen = Rect::new(17.0, 23.0, screen_width, screen_height);
            let commands = dialog.paint(screen, &advance, LINE_HEIGHT);
            let (panel, _, _) = panel_of(&commands);
            assert_eq!(
                dialog.panel_rect(screen, &advance, LINE_HEIGHT),
                panel,
                "the accessor and the drawing agree at {screen:?}"
            );

            for index in 0..buttons {
                let action = dialog
                    .action_rect(index, screen, &advance, LINE_HEIGHT)
                    .unwrap_or_else(|| panic!("action {index} of {buttons}"));
                assert!(
                    panel.x <= action.x && panel.y <= action.y,
                    "action {index} is inside the panel at {screen:?}"
                );
                assert!(
                    action.x + action.width <= panel.x + panel.width
                        && action.y + action.height <= panel.y + panel.height,
                    "and inside it on the far side too: {action:?} in {panel:?}"
                );

                // Pressed one pixel inside it and released with nothing firing
                // one pixel to its left: the same numbers on both sides.
                let before: Vec<i32> = hits.iter().map(|hit| hit.get()).collect();
                let mut outside = tap_at(Offset::new(action.x - 1.0, action.y + 1.0));
                assert!(dialog.on_event(&mut outside, screen, &advance, LINE_HEIGHT));
                assert!(
                    dialog.visible.get(),
                    "the gap dismissed nothing at {screen:?}"
                );
                let after: Vec<i32> = hits.iter().map(|hit| hit.get()).collect();
                assert_eq!(after, before, "and fired nothing either");

                let mut inside = tap_at(Offset::new(action.x + 1.0, action.y + 1.0));
                assert!(dialog.on_event(&mut inside, screen, &advance, LINE_HEIGHT));
                assert!(
                    inside.consumed() && !dialog.visible.get(),
                    "and one pixel further in it dismissed at {screen:?}"
                );
                assert_eq!(
                    hits[index].get(),
                    before[index] + 1,
                    "and it was action {index} of {buttons} that fired"
                );

                present(&dialog);
            }
        }
    }

    #[test]
    fn the_dialog_lays_its_actions_out_from_the_end_of_the_inner_box() {
        // The row is right-aligned, and the arithmetic behind that is the panel's
        // own numbers rather than a constant: the inner right edge less the
        // widths and the gaps between them.
        let mut nodes = Arena::new();
        let dialog = shown(&mut nodes);
        assert_eq!(
            dialog.panel_rect(SCREEN, &advance, LINE_HEIGHT),
            PANEL,
            "the panel these are read out of"
        );
        assert_eq!(
            dialog.action_rect(0, SCREEN, &advance, LINE_HEIGHT),
            Some(OK),
            "and the row it holds"
        );
        let inner_right = PANEL.x + PANEL.width - PANEL_PADDING;
        let row = OK.width + THEME_SPACING_SM + CANCEL.width;
        assert_eq!(
            OK.x,
            inner_right - row,
            "the row's right edge is the inner one"
        );
        assert_eq!(CANCEL.x + CANCEL.width, inner_right);

        // And the row sits on the panel's bottom line box, one padding above it.
        assert_eq!(OK.y + OK.height, PANEL.y + PANEL.height - PANEL_PADDING);
        assert_eq!(
            LayoutRect::from_parts(OK.x, OK.y, OK.width, OK.height)
                .size
                .width,
            OK.width,
            "and `layout::Rect` reads the same numbers, if a caller wants them there"
        );
    }
}
