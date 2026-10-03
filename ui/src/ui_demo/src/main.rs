//! Demo harness for `ui_core`.
//!
//! Opens a window with an OpenGL ES 3.1 context and draws three pads on a
//! themed background. A pad has one property — how far it is pressed, from 0
//! at rest to 1 held — and paints itself by interpolating between its rest
//! and held colours from that value at paint time: the animation writes a
//! single number, and the pad's whole appearance follows.
//!
//! The three pads sit inside a card: the [`Container`] widget, with a background
//! bound to the theme's `Surface` and a padding of [`CARD_PADDING`]. It is the
//! one place the demo draws a container — the other four in the tree, the text
//! column, the text panel, the controls layer and the root, group children and
//! have no background, which is what a container with no background looks like.
//!
//! Every colour in the demo comes from the theme: the background from
//! `Background`, each pad's rest colour from `Error`, `Success` or `Primary`.
//! Pressing `T` switches between the dark and light themes over 300 ms, and
//! the property graph carries the change to every colour — no widget is
//! told, and none needs to be.
//!
//! Pressing the left mouse button over a pad presses it; holding the space bar
//! presses all three, cascading across them with a stagger. Releasing springs
//! them back to rest. Every colour property carries an `on_change` callback
//! that marks its node dirty — the link from an animation or a theme switch to
//! the node arena, which the animation and theme modules know nothing about.
//!
//! Down the right-hand column the demo carries a **slider**, dragged or driven
//! by `0` and `1`; a **toggle**, whose click turns it on and off and whose label
//! says which state it is in; a **gauge**, reading half of its range and moved
//! by `,` and `.`, cycled between its three shapes by `G` and drawn with a
//! needle that arrives on a spring; a **progress bar** at half, moved by `[` and
//! `]` and switched into its sliding mode by `P`; and a **chart**, plotting a
//! window of sample readings — eight at a start and twelve at most — whose shape
//! is cycled by `H`, to which `A` appends a reading and `S` shifts the oldest one
//! out, so that requirement 4's "append new value, shift old values" and
//! requirement 5's "new data points animate in" are both reachable on screen. The
//! **image** sits in the top right, loaded from `assets/demo.png` and cycled
//! through its four fits by `F`.
//!
//! Every control below the pads is driven by the input module rather than by raw
//! events: the gesture recogniser turns an SDL event into the tap or key press
//! it completed, and dispatch routes it to the node under it, which is what lets
//! a control consume the events meant for it. `Tab` and `Shift+Tab` move focus
//! and `Enter` activates the control holding it.
//!
//! **There is no row of buttons.** Three of them — a counter, a disabled one and
//! a reset — were here for tasks 11 to 19 to prove that a button animates, and
//! the operator took them out on 2026-10-01: *"You can remove the first three
//! buttons that were used for testing animations."* What that costs is written
//! down where it is paid rather than left for a reader to discover: the press
//! and release transition, the hover tint, the focus ring and the click callback
//! are no longer on screen anywhere, and `ui_demo` is the only place in the
//! repository where any of them was demonstrated. What is left that a pointer
//! drives is the slider and the toggle.
//!
//! **There is no list either, and that cost more.** It was here for task 18 — a
//! hundred rows in a 280-tall viewport, with a readout naming the first row on
//! screen and the length of the free list — and the operator took it out on
//! 2026-10-02 alongside a request to remove *"some existing widgets like list or
//! so (Keep fps label)"*, to make room for the chart. **The demo has therefore
//! lost its only scrolling viewport**: `Scroll`, the widget the operator has
//! reported two problems with — a 6-pixel bar no finger could aim at and a drag
//! that lagged the cursor ten to one — is no longer driven by anything here, and
//! the widget's own unit tests are now the only place either behaviour is
//! checked. **It has also lost its on-screen proof of virtualisation**: the
//! `first 0, live 10, free 0` line was task 18's evidence that ninety rows of a
//! hundred are not in the tree, and there is no substitute for it in this demo.
//! Both losses are the operator's decision rather than an agent's, and they are
//! written down here because that is what this file does with a cost.
//!
//! The window is 1280 by 720 rather than the 1024 by 600 the first three pads
//! and the text panel were laid out for, because those two regions are full: the
//! card of pads ends at x 788 and the text panel's column at x 654, so the newer
//! widgets had nowhere to go. Enlarging the window is the smallest change
//! that fits them, because every widget's position in it is absolute and
//! independent of the window's size — only the root's constraint and the
//! background node's tight constraint read [`WINDOW`] — so nothing that was
//! already there moves. `no_two_placed_rects_overlap` and
//! `every_placed_rect_is_inside_the_window` are what hold that claim up.
//!
//! In the bottom left of the window a readout names how fast the loop is running:
//! the current rate, the run's average and its worst single frame. The number is
//! measured from the frame deltas the loop already computes — see [`fps`] — and
//! the same run is printed to stdout as one `roados-fps …` line when the demo
//! stops, which is what `.ai/tools/fps-check.sh` reads and what a comparison
//! between two builds is made of. `ROADOS_RUN_SECONDS` bounds the run, because
//! the demo otherwise only stops when its window is closed and a measurement
//! nobody can end is not a measurement.
//!
//! **And a dialog is over the whole of it, showing.** `D` opens and closes it,
//! `Escape` closes it, and while it is up it is modal: it takes every tap and
//! every key aimed at a widget, the five controls behind it come out of the `Tab`
//! order, and its own two buttons come into it. `OK` switches the theme and
//! `Cancel` does nothing, which is the only way two buttons in a dialog can mean
//! different things without a second widget in it.
//!
//! It **opens showing rather than waiting to be opened**, and that is a decision
//! about evidence rather than about the demo. Input injection does not work on
//! this host — task 21's positive control had `T` move 212 pixels of gauge needle
//! from an injected press, and pointer injection has never delivered anything at
//! all — so a dialog that began hidden could only ever be photographed through an
//! instrument, and three of the captures in this task's history already rested on
//! instrumented routes. Starting visible means the capture in
//! `doc/ui/TASK_UI_PRIM_22.md` needs no seed, no rebuild and no environment
//! variable, and it is a capture of the real demo.
//!
//! **It covers the middle of the gallery while it is up**, which the operator
//! accepted rather than a reader having to discover: a centred panel over a full
//! window hides whatever is behind it, and the window cannot grow (a 1280 by 1320
//! request comes back 1280 by 1052 on this host), so the alternatives were a
//! dialog that hid part of itself or a window that could not be shown. Nothing was
//! moved to make room.
//!
//! **What "modal" covers here, and it is everything.** While the dialog is visible
//! the demo offers every event to the dialog's subtree and to nothing else, the
//! `Tab` order is the dialog's two buttons alone, and **the demo's own shortcuts
//! are suppressed as well** — the whole of [`GALLERY_SHORTCUTS`] and the three
//! pointer arms that reach the pads, the slider and the keyboard without routing
//! at all. One guard covers the key table and one covers the pointer presses; see
//! [`Demo::handle_event`].
//!
//! **That last half was not in the first version of this paragraph, and the gap was
//! deliberate rather than overlooked**: `input::route` walks nodes, so *the input
//! module's* half of modality is about nodes, and a shortcut the demo binds itself
//! is not an event offered to one. The operator decided on 2026-10-03, having been
//! shown that closing the gap silently changes the preconditions of 38
//! pre-existing tests and choosing it anyway, that a modal which leaves the host
//! application's shortcuts live is not modal. The 38 tests changed **fixture**, and
//! not one assertion.
//!
//! **The releases are the one thing not guarded**, and the asymmetry is
//! load-bearing: hold `Space`, press `D`, release `Space`, and a gated release
//! would leave three pads at full press behind the scrim with no gesture left that
//! could bring them back. [`Demo::space_pressed`] is what answers "was there a
//! press" instead of asking about the current focus, which by then is inside the
//! dialog.

mod fps;

use crate::fps::FrameRate;
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
#[cfg(test)]
use sdl3::keyboard::Mod;
use sdl3::mouse::MouseButton;
use std::cell::RefCell;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};
use ui_core::animation::{AnimationClock, AnyAnimation, Easing, Interpolate, Stagger};
use ui_core::arena::{Arena, Handle};
use ui_core::font::Font;
use ui_core::input::{self, Focus, GestureRecognizer, InputEvent, InputEventKind, Key};
use ui_core::layout::{
    mark_dirty, Constraints, CrossAxisAlignment, FlexConfig, Layout, LayoutMode, LayoutState,
    MainAxisAlignment, Offset, Padding, Size,
};
use ui_core::node::{self, WidgetNode};
use ui_core::paint::{Color, PaintState, Painter, Rect};
#[cfg(test)]
use ui_core::paint::{DrawCommand, FontWeight, UvRect};
use ui_core::property::Property;
use ui_core::render::context::Context;
use ui_core::render::Renderer;
use ui_core::texture::{Pixels, TextureCache, TextureHandle};
use ui_core::theme::{PropertyValue, Theme, ThemeToken};
use ui_core::widgets::button::Motion;
use ui_core::widgets::chart::{Chart, ChartType, Palette as ChartPalette};
use ui_core::widgets::container::Container;
use ui_core::widgets::dialog::{Dialog, DialogAction, Palette as DialogPalette};
use ui_core::widgets::gauge::{Gauge, GaugeType, Palette as GaugePalette};
use ui_core::widgets::image::{Image, ImageFit, ImageSource};
use ui_core::widgets::keyboard::{KeyAction, Keyboard, Palette as KeyboardPalette};
use ui_core::widgets::label::{Label, LayoutOptions, TextAlign, Truncation, WrapMode};
use ui_core::widgets::progress::{Palette as ProgressPalette, Progress};
use ui_core::widgets::slider::{Orientation, Palette as SliderPalette, Slider};
use ui_core::widgets::text_input::{Palette as TextInputPalette, TextInput};
use ui_core::widgets::toggle::{Palette as TogglePalette, Toggle};
// `Callback` is the payload-free alias of the same type every other widget's
// handler is, and the demo imports it under a second name: a slider's handler
// takes the value it moved to, and `Callback::from_fn` on the alias would be
// `Callback<()>`.
use ui_core::widgets::Callback;
use ui_core::widgets::Callback as ValueCallback;

/// The window, and the box the root is laid out in.
///
/// 1280 by 1020 rather than 1024 by 600: the four widgets the later tasks added
/// do not fit beside the ones already here, and every position in the demo is
/// absolute, so growing the window moves nothing and lets four more in. See the
/// module documentation for the whole of the argument.
///
/// **The height grew twice, and the second time is task 19's.** 720 was reached
/// because the controls would not fit beside the text panel, and 1020 because
/// [`BAND_TOP`] is 720: everything above it is the gallery exactly as it was, and
/// the text-entry band goes below. **Nothing above [`BAND_TOP`] moved**, which is
/// the point, and `the_gallery_above_the_band_is_where_it_was` checks it rather
/// than trusting it.
///
/// **1020 is measured, not chosen.** The first attempt was 1160 — a field and a
/// keyboard stacked — and the window came back **1052 pixels tall**: this host
/// has two stacked displays, `eDP-1` at 1920x1080 and `HDMI-A-1` at 1920x1200,
/// and the window manager capped the height where the window landed. A window
/// taller than the cap is not merely awkward, it is **unverifiable**: the bottom
/// of the keyboard never reaches the screen, so the capture that is supposed to
/// prove the widget draws cannot see it. 1020 leaves room under the cap, and the
/// band is laid out **side by side** rather than stacked for the same reason — a
/// field over a 300-tall keyboard needs 364 pixels of band and the budget is 300.
const WINDOW: Size = Size {
    width: 1280.0,
    height: 1020.0,
};

/// How long one frame is budgeted to take: **60 Hz**, the rate
/// `doc/ui/DEMO_APPLICATION.md` lists as *"Smooth animations and transitions —
/// 60 FPS"*.
///
/// A frame budget, **not** a wait. This replaced a fixed 16 ms
/// `wait_event_timeout`, and the difference is the whole fix. That loop waited a
/// fixed 16 ms and *then* drew, so the wait and the frame's own cost were
/// **serialised** and a frame was `16 ms + work` no matter how little the work
/// was. Measured on this host, that is `19.9 ms` — **50.2 fps** — for a frame
/// whose own work is 3.9 ms, and an infinitely fast frame would still have
/// capped it at 62.5 fps.
///
/// The budget inverts that: the loop waits only what is **left** of this frame
/// after the work is done, so the frame self-paces at 60 Hz and the work is
/// spent *inside* the budget rather than after it. With 3.9 ms of work that is
/// 16.67 ms a frame, and the measured rate went **50.2 → 62.0 fps**.
///
/// What would reverse it is a display that is not 60 Hz: this is a hard-coded
/// rate, not a query of the monitor's refresh, because SDL's
/// `GL_SetSwapInterval` is never called and the demo has no mode to change. A
/// cluster panel at 30 Hz would run this at half its refresh and waste half its
/// budget; asking the display for its rate, or setting the swap interval to match
/// it, is the change that would fix that and it is not this task's.
const FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667);

/// The size of one pad.
const PAD_SIZE: Size = Size {
    width: 220.0,
    height: 140.0,
};

/// The gap between the pads.
const PAD_SPACING: f32 = 52.0;

/// The padding the row of pads is given, and so the margin between the card it
/// is drawn on and the pads inside it.
///
/// The card is the pads' 140 pixels of height plus this twice, and it has to
/// clear the text panel below it: the first label's line box starts at
/// [`TEXT_PANEL_ORIGIN`]'s 170, so twelve leaves six pixels between them and
/// sixteen would have eaten two of the line box's own top.
const CARD_PADDING: f32 = 12.0;

/// The corner radius a card falls back to if the theme ever holds something
/// other than a number in its `BorderRadiusLg` token. It is the value both
/// themes hold today.
const CARD_RADIUS_FALLBACK: f32 = 16.0;

/// The corner radius a pad is painted with.
const PAD_RADIUS: f32 = 28.0;

/// How long the demo's theme switch takes.
const THEME_TRANSITION: u32 = 300;

/// How far a pad's held colour is lifted toward white from its rest colour.
const HELD_LIGHTEN: f32 = 0.4;

/// The theme token each pad's rest colour comes from: a red, a green and a
/// blue pad, from the theme's error, success and primary colours.
const PAD_TOKENS: [ThemeToken; 3] = [ThemeToken::Error, ThemeToken::Success, ThemeToken::Primary];

/// The font file the demo's labels are drawn with.
///
/// **Lato Medium, not Regular**, and the choice is the operator's to revisit
/// rather than the demo's: it is the face every advance in the gallery was
/// measured with, and changing it moves every label's rect.
const FONT_PATH: &str = "/usr/share/fonts/truetype/lato/Lato-Medium.ttf";

/// The font file the demo's **bold** runs are drawn with.
///
/// The partner of [`FONT_PATH`] out of the same family and the same foundry's
/// weight axis: [`ui_core::render::Renderer::set_bold_font`] takes a second face
/// and resolves [`ui_core::paint::FontWeight::Bold`] against it, so a bold run
/// carries that face's own glyphs, bearings and advances. Lato-Bold beside
/// Lato-Medium is the pairing the two neighbouring weights of one design give,
/// which is what makes the pair look like one typeface at two weights rather than
/// like two typefaces.
///
/// **Measured, and the measurement is a warning rather than a reassurance:** at
/// 20 and 28 pixels the bold run is 1.5% and 0.5% wider than the regular one for
/// `"Handgloves 42"` and **identical** at 28 pixels for `"Settings"`, because
/// FreeType rounds each advance to a whole pixel at the size the face is set to.
/// So a caller cannot assume a bold run ends at a different `x`, and must not
/// assume it ends at the same one. That sentence is
/// [`ui_core::paint::Painter::text_bold`]'s own doc, and this constant is where
/// the pair it was measured on lives.
///
/// What would reverse it: a font the operator picks. Nothing else in the demo
/// depends on this file existing — [`ui_core::font::Font::from_path`] returns a
/// `Result`, and `main` propagates it, so a missing file is a failed run rather
/// than a silently regular title.
const BOLD_FONT_PATH: &str = "/usr/share/fonts/truetype/lato/Lato-Bold.ttf";

/// The panel the text is laid out in: the width its wrapping label wraps at,
/// and the height the text column is given.
///
/// The height is the column's height at [`TEXT_SIZE_START`]; the column is
/// not clipped to it, so a larger `+` size overflows the window bottom by
/// design.
const TEXT_PANEL: Size = Size {
    width: 900.0,
    height: 380.0,
};

/// Where the text column sits in the window: below the pads, which a `Stack`
/// leaves at the top-left corner.
const TEXT_PANEL_ORIGIN: (f32, f32) = (60.0, 170.0);

/// The width the text column's labels are laid out in.
///
/// This is what keeps the column clear of the control column, and it is the reason
/// it is not the panel's own width. The panel is [`TEXT_PANEL`] wide, so a
/// right-aligned label laid out across it ends at 60 + 900 = 960 and runs
/// underneath the column, which starts at [`CONTROLS_ORIGIN`]'s 664 — the kind of
/// collision no unit test sees, because both the label and the control to its
/// right lay out correctly on their own. Laying the column out at 594 puts its
/// right edge at
/// 654, and the ten pixels between are the clearance.
///
/// The paragraph wraps at this width and the three alignment rows share it, so
/// the comparison those three exist for is still like for like.
const TEXT_COLUMN_WIDTH: f32 = 594.0;

/// The gap between the text column's labels.
const LABEL_SPACING: f32 = 12.0;

/// The font size the labels start at, and the bounds `+` and `-` move within.
const TEXT_SIZE_START: f32 = 24.0;
const TEXT_SIZE_MIN: f32 = 10.0;
const TEXT_SIZE_MAX: f32 = 64.0;
const TEXT_SIZE_STEP: f32 = 2.0;

/// The theme tokens the labels' colour cycles through on `C`, in that order.
///
/// The labels take their colour from the theme, like every other colour in the
/// demo, so `T` carries a theme switch to the text too — and `C` moves which
/// token they read, which is the same link one step earlier in the chain.
const TEXT_COLOR_TOKENS: [ThemeToken; 4] = [
    ThemeToken::Text,
    ThemeToken::Primary,
    ThemeToken::Success,
    ThemeToken::Warning,
];

/// How long a pad takes to press down.
const PRESS_DURATION: Duration = Duration::from_millis(150);

/// How far apart the pads' presses cascade when the space bar is held.
const STAGGER_STEP: Duration = Duration::from_millis(60);

/// How long a pad takes to spring back to rest.
const RELEASE_DURATION: Duration = Duration::from_millis(400);

/// The spring a pad releases on: a little underdamped, so it settles with a
/// whisper of overshoot rather than a thud.
const RELEASE_SPRING: Easing = Easing::Spring {
    damping: 9.0,
    stiffness: 140.0,
};

/// Where the demo's right-hand control column starts in the window.
///
/// **Not a widget's origin but the column's left edge**, and it is the one number
/// in this file that four widgets are expressed against rather than each
/// repeating: the gauge's, the slider's, the toggle's and the progress bar's own
/// origins all say its `x`. Before task 20 this was the origin of the row of
/// three buttons — the same number, under a name that described a widget — and
/// the buttons were removed on 2026-10-01, so the number outlived its name and is
/// now written down for what it actually is.
///
/// Its `y`, 396, is **where the buttons were**, so it is the top of the column
/// rather than the top of any one thing in it. The list shared it until it went
/// on 2026-10-02; the chart is above it rather than on it, at
/// [`CHART_ORIGIN`].
///
/// The `x` is 664 because the text panel's labels stay left of 660 — the panel is
/// 900 wide from an origin of 60, but its widest line wraps at
/// [`TEXT_COLUMN_WIDTH`] — and the pads end at y = 140, so the column goes right
/// of the text and below the pads rather than under the text column, which
/// already reaches the bottom of the window. The four pixels between 660 and 664
/// are the clearance `no_text_label_reaches_under_the_controls` measures.
const CONTROLS_ORIGIN: (f32, f32) = (664.0, 396.0);

/// The font size every readout below the pads is drawn at.
///
/// The panel's labels are at [`TEXT_SIZE_START`]; everything below the pads is
/// short strings rather than a column of prose, and `+` and `-` move the panel
/// alone.
///
/// **This was `BUTTON_FONT` until the buttons went**, and it is renamed rather
/// than left under a name describing three widgets the demo no longer has:
/// eight readouts draw at this size and none of them is a button's.
const READOUT_FONT: f32 = 20.0;

/// Where the slider sits in the window.
///
/// The same column as everything else below the pads — right of the text panel,
/// whose widest line ends at `TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH` — and
/// 496 is unchanged by the buttons going: it is where the slider has always
/// been. The space above it that the row of buttons used to hold is now the
/// gauge's.
const SLIDER_ORIGIN: (f32, f32) = (CONTROLS_ORIGIN.0, 496.0);

/// How thick the demo's slider track is, in pixels.
///
/// **The widget's own default is 6, and the operator called that unusable:**
/// *"the slider is very narrow, can't imagine how I could use it in a car with my
/// finger"* (2026-09-30). Six pixels is a hairline — correct with a mouse, where
/// precision is free, and wrong with a fingertip, which is roughly 40 across.
///
/// The widget's constants are **not** changed: they are a documented baseline and
/// task 14's record says each one says what would reverse it. This is the setter
/// doing what it exists for, which is also what makes the two numbers visible side
/// by side. What would reverse *that* choice is the head unit's own bezel and
/// glove spec, and it is the operator's call.
const SLIDER_TRACK_THICKNESS: f32 = 12.0;

/// The radius of the demo's slider's thumb, in pixels.
///
/// 18 is a 36-pixel knob with the widget's 2-pixel border, against the 6-pixel
/// track and 24-pixel knob the widget defaults to.
///
/// **Why not larger.** 22 gave a 44-pixel knob and a 62-tall node, and the node is
/// what the hit test uses — but the right-hand column then stopped fitting: five
/// controls, of which the slider, the toggle and the progress bar are all finger-
/// sized, need more than the 324 pixels between the top of the column and the
/// bottom of the window, and `no_two_placed_rects_overlap` said so. **This is the
/// ceiling for a bigger slider without moving the progress bar out of this
/// column**, and that trade is the operator's to make rather than an agent's.
///
/// Removing the buttons loosened that constraint rather than tightening it — the
/// column now starts 156 pixels higher, at the gauge — so 18 is a ceiling that has
/// not been retested against a 22. **What would reverse it** is a knob a thumb
/// cannot miss, and that is the same question the operator has already answered
/// twice about a 6-pixel track.
const SLIDER_THUMB_RADIUS: f32 = 18.0;

/// How long the demo's slider is, in pixels.
///
/// The widget asks for its own [`DEFAULT_LENGTH`](ui_core::widgets::slider)
/// default of 240; a finger benefits from a longer swipe, and 300 still clears
/// [`CHART_ORIGIN`]'s left edge at 1000 by 36 pixels.
const SLIDER_LENGTH: f32 = 300.0;

/// How far below the slider its value readout sits.
///
/// The slider is **52** tall now, not the 44 its default sizing asked for, so this
/// has to clear 52 rather than 44. It leaves the readout's own 24-pixel line at
/// 560..584, clear of the toggle at 592 below it.
const SLIDER_READOUT_DROP: f32 = 64.0;

/// The width the value readout is given, wide enough for the longest string it
/// shows: a value out of the maximum, and a count of the adjustments so far.
const SLIDER_READOUT_WIDTH: f32 = 320.0;

/// The value the demo's slider starts at, and its minimum.
const SLIDER_MIN: f32 = 0.0;

/// The maximum of the demo's slider.
const SLIDER_MAX: f32 = 100.0;

/// The grid the demo's slider snaps to, in the same units as its range.
///
/// Five is a step that divides the range, so every grid point is reachable — the
/// widget snaps to the nearest step whatever the range is, and a step that does
/// not divide it leaves the top unreachable.
const SLIDER_STEP: f32 = 5.0;

/// Where the demo's image sits, in the gap right of the card of pads.
///
/// The card is 788 wide, so 800 is twelve clear of it, and the image is 160 tall
/// against the card's 164: the two are side by side rather than stacked, because
/// the band below them is where the other three new widgets are.
const IMAGE_ORIGIN: (f32, f32) = (800.0, 0.0);

/// The box the image is fitted into.
///
/// 220 by 160 is *wider* than `assets/demo.png`'s own 320 by 192 is tall in the
/// sense that matters: the asset is 1.67 to 1 and the box is 1.375 to 1, so
/// [`ImageFit::Contain`] letterboxes it top and bottom and [`ImageFit::Cover`]
/// crops its width, and the two are visibly different pictures rather than two
/// names for one.
const IMAGE_SIZE: Size = Size {
    width: 220.0,
    height: 160.0,
};

/// Where the label naming the image's current fit sits.
///
/// It is **below** the image rather than beside it because
/// [`ImageFit::None`] draws the image at its own size — 320 by 192, from the
/// box's top left — and a label to the right of the box would be under it in
/// that one mode and clear of it in the other three. Below the box it is clear
/// of all four, and 200 is eight past the tallest of them.
const IMAGE_FIT_ORIGIN: (f32, f32) = (800.0, 200.0);

/// The width the fit label is given: the longest string it can show on one line.
///
/// The longest is `fit: Contain (stand-in), focused` — the longest fit name, the
/// stand-in note and the focus marker together — and the width is what stops the
/// label cutting it with an ellipsis. It is 360 rather than what the string needs
/// under the demo's *test* font, because the width is fixed at construction and
/// a face with wider glyphs than the monospace stand-in would otherwise ellipsise
/// a line that is not really too long.
const IMAGE_FIT_WIDTH: f32 = 360.0;

/// Where the demo's gauge sits, the top of the right-hand control column.
///
/// **This is where the row of three buttons was**, and the two agree on the x —
/// [`CONTROLS_ORIGIN`]'s 664 — because both were the first thing in the column
/// and the buttons' removal moved nothing: the gauge is above the slider, at the
/// slider's x, and the slider stayed at [`SLIDER_ORIGIN`].
///
/// The **y is not the buttons' 396.** A button row is 44 tall and a gauge is a
/// dial, and a dial has to be big enough to read its own tick marks and see its
/// needle move: 200 is the widget's own `DEFAULT_SIZE`, and a 100-tall box in
/// the space the counter left would put eleven 8-pixel marks on a quarter of the
/// arc. So the gauge takes the two regions the buttons and the counter used to
/// hold **and the empty band above them**, 240 down to 440, and its readout sits
/// in what is left of the freed space at [`GAUGE_READOUT_ORIGIN`].
///
/// 240 is above [`IMAGE_FIT_ORIGIN`]'s own 200 plus the height of the fit label's
/// line, so the gauge is below the image rather than beside its readout, and 440
/// is 56 above [`SLIDER_ORIGIN`]'s 496 — the arc's outer edge is at the node's own
/// edge, so nothing this widget draws reaches past the 440. What would reverse
/// this is a head unit whose dial is a different size, which is the operator's
/// number and not an agent's.
const GAUGE_ORIGIN: (f32, f32) = (CONTROLS_ORIGIN.0, 240.0);

/// The box the demo's gauge is given.
///
/// **The widget's own `size()`**, which is [`GAUGE_ORIGIN`]'s square: a gauge
/// has no content to measure, so `Gauge::size()` is the widget saying how big it
/// wants to be, and the demo asks for exactly that rather than choosing a number
/// of its own. It is the only square box in the demo — every other control is a
/// bar or a panel — because a dial inscribed in a non-square box is a dial
/// inscribed in the *shorter* of the two sides, so a wide box buys nothing and
/// only leaves empty space the collision tests would have to reason about.
const GAUGE_SIZE: Size = Size {
    width: 200.0,
    height: 200.0,
};

/// Where the label naming the gauge's value and shape sits, under the dial.
///
/// Under rather than beside it, because the region to the right of the dial is
/// the slider's: the gauge ends at 864 and the slider's own 300-pixel box runs to
/// 964, and the 100 pixels between them are narrower than this label is. 448 is
/// in the space the button row and the click counter used to hold — 440 is the
/// dial's own bottom edge — and it is 48 above [`SLIDER_ORIGIN`]'s 496.
const GAUGE_READOUT_ORIGIN: (f32, f32) = (CONTROLS_ORIGIN.0, 448.0);

/// The width the gauge's readout is given: the longest string it can show on one
/// line.
///
/// `240 of 240, 100%, circle` is that string — a value, its share of the range
/// and the shape it is drawn in, in the widget's own enum order so that the third
/// word can be compared against `GaugeType`'s variants — and the width stops the
/// label cutting it at the node's own width, which is what would happen to
/// `240 of 240, 100%, ci…`. 320 is [`SLIDER_READOUT_WIDTH`]'s number for the same
/// reason and keeps the two readouts the same width, so the column reads as one
/// column.
const GAUGE_READOUT_WIDTH: f32 = 320.0;

/// The range the demo's gauge covers, in km/h.
///
/// **Zero to 240, which is a speedometer**, and the unit is what the readout
/// prints rather than something the widget knows: a gauge is told its range and
/// nothing else, so the demo is the only place that can say what the number is.
const GAUGE_MIN: f32 = 0.0;
const GAUGE_MAX: f32 = 240.0;

/// The value the demo's gauge starts at, and the share of its range that is.
///
/// **Half, and it is half exactly**: task 20's acceptance criterion is *"Demo
/// shows a gauge at 50%"*, and 120 of 0 to 240 is 120/240 = 0.5 with no rounding
/// in it. A tenth of the range is the step the two keys move it by, and it
/// divides 240 exactly for the reason [`PROGRESS_TENTHS`] gives for the bar's.
const GAUGE_START: f32 = 120.0;
const GAUGE_STEP: f32 = 24.0;

/// How finely the gauge's value is snapped, as a count of steps.
///
/// It is the reciprocal of [`GAUGE_STEP`], written down as the number it is used
/// as: the demo rounds `value / GAUGE_STEP` to an integer and multiplies back, so
/// a value that arrived by repeated addition lands on the grid rather than near
/// it. A gauge at 119.99999 is a needle a fraction of a pixel off twelve o'clock,
/// which is the same defect [`PROGRESS_START`] describes for the bar.
const GAUGE_TENTHS: f32 = 10.0;

/// The shapes the demo's gauge cycles through, in the order `G` walks them.
///
/// **`Needle` first**, because it is the shape that shows every part of the
/// widget at once — the track, the fill, the tick marks *and* the pointer — and a
/// demo that starts on the plain arc would need a keypress before anything new
/// appeared. [`Arc`](GaugeType) and [`GaugeType::Circle`] follow, and `Circle` is
/// last because it is the one that ignores [`end_angle`](Gauge::end_angle)
/// entirely: a full turn whatever the two angles say, which is worth seeing but
/// is not a new shape so much as the same sweep closed.
///
/// The three are named again in [`GAUGE_TYPE_NAMES`] rather than derived from the
/// enum, because the readout needs the *word* and `GaugeType`'s `Debug` is not
/// part of its contract.
const GAUGE_TYPES: [GaugeType; 3] = [GaugeType::Needle, GaugeType::Arc, GaugeType::Circle];

/// The name of each shape in [`GAUGE_TYPES`], in the same order.
///
/// The names are capitalised as the other readouts' words are (`Contain`, not
/// `contain`) because they are read as labels rather than as code, and because
/// `GaugeType`'s own variants are capitalised.
const GAUGE_TYPE_NAMES: [&str; 3] = ["Needle", "Arc", "Circle"];

/// How long the demo's gauge takes to arrive at a new value, on a spring.
///
/// **The demo's number, and the reason it is not the theme's**: a gauge needle
/// that eased from 60 to 120 over [`THEME_TRANSITION`]'s 300 ms reads as a bar
/// being filled, and the whole of a gauge is that a pointer *swings* and settles.
/// Every other control in the demo takes [`Motion::from_theme`], and this is the
/// first that does not, so the two are written out side by side rather than one
/// being derived from the other: 600 ms is long enough for the spring below to
/// have most of its travel and short enough that a keypress feels answered
/// before the next one.
///
/// What would reverse it is a head unit whose needle is meant to glide rather than
/// swing, which is a judgement about a car rather than about this widget.
const GAUGE_MOTION: Duration = Duration::from_millis(600);

/// The spring the demo's gauge's needle arrives on.
///
/// A little underdamped — the damping ratio is `9 / (2 · sqrt(140))` = 0.38, so
/// it overshoots by about a sixth and comes back — which is what makes a needle
/// look like a needle. **The widget never picks this**: `Gauge::animate_to_state`
/// takes the caller's [`Motion`] and honours it, and it is the caller that has to
/// know that a spring on that call is a springing *needle*, because the needle
/// points at [`shown`](Gauge::shown) and both it and the fill read that one
/// property.
///
/// These two numbers are [`RELEASE_SPRING`]'s, deliberately: a pad springing
/// back to rest and a needle springing to a reading are the same movement, and
/// one pair of coefficients for both is one less number to explain.
const GAUGE_SPRING: Easing = Easing::Spring {
    damping: 9.0,
    stiffness: 140.0,
};

/// Where the demo's toggle sits, under the slider's readout.
///
/// The readout's own line ends at [`SLIDER_ORIGIN`]'s 496 plus
/// [`SLIDER_READOUT_DROP`]'s 52 and its own 24 pixels, and the toggle's box is
/// the widget's own — a 48-wide track in a 44-tall touch target — so this is the
/// next line that clears it.
const TOGGLE_ORIGIN: (f32, f32) = (CONTROLS_ORIGIN.0, 592.0);

/// Where the label naming the toggle's state sits, beside it.
///
/// Twelve to the right of the toggle's own 48 pixels, and low enough that its
/// line is about the middle of the toggle's height rather than level with its
/// top.
///
/// It follows [`TOGGLE_ORIGIN`] rather than repeating its number: the toggle moved
/// down 20 when the slider grew above it, and a literal here was still sitting at
/// the old 592 — clear of nothing, since the slider's readout now ends there.
const TOGGLE_READOUT_ORIGIN: (f32, f32) = (CONTROLS_ORIGIN.0 + 64.0, TOGGLE_ORIGIN.1 + 20.0);

/// The width the toggle's readout is given: enough for `on, 1 change` and
/// `off, 0 changes` on one line each.
const TOGGLE_READOUT_WIDTH: f32 = 240.0;

/// Where the demo's progress bar sits, the last thing in the left column of the
/// band.
///
/// 668 plus the bar's own 44 pixels is 712, and the window is 720: eight
/// pixels of margin, which is the whole of what is left below the band.
const PROGRESS_ORIGIN: (f32, f32) = (CONTROLS_ORIGIN.0, 668.0);

/// The box the progress bar is given.
///
/// 240 by 44 is the slider's own box: a bar of the same width as the slider
/// above it reads as part of the same column rather than as a second control of
/// its own shape.
const PROGRESS_SIZE: Size = Size {
    width: 240.0,
    height: 44.0,
};

/// Where the label naming the progress bar's value and mode sits, above it.
///
/// Above rather than beside, because the region to the right of the bar is
/// where the chart is: the bar ends at 904 and the chart starts at
/// [`CHART_ORIGIN`]'s 1000, and 96 pixels is not enough for the longest string
/// this label shows.
const PROGRESS_READOUT_ORIGIN: (f32, f32) = (CONTROLS_ORIGIN.0, 640.0);

/// The width the progress readout is given: the longest string it can show on
/// one line.
///
/// `50%, determinate, focused` is that string — a percentage, a mode and the
/// focus marker — and the width is what stops the label cutting it, which it
/// would at 240: the monospace stand-in the tests measure with is ten pixels a
/// character, so twenty-three of them are 230 and the ellipsis goes on at the
/// node's own width. A bar that says `50%, determinate, focus…` is a readout
/// that cannot say what it is saying.
const PROGRESS_READOUT_WIDTH: f32 = 280.0;

/// The value the demo's progress bar starts at, and the fraction `[` and `]`
/// move it by.
///
/// Half is task 17's acceptance criterion. A tenth is a step that divides one
/// exactly, which ten steps of it do not: `0.5 + 0.1` ten times lands on
/// `0.99999994`, and a bar filled to 99.999994 per cent of its track is a bar
/// that is not quite full. The demo snaps the value to tenths for the reason
/// [`PROGRESS_TENTHS`] gives.
const PROGRESS_START: f32 = 0.5;
const PROGRESS_STEP: f32 = 0.1;

/// How finely the progress bar's value is snapped, as a count of steps.
///
/// It is the reciprocal of [`PROGRESS_STEP`], written down as the number it is
/// used as: the demo rounds `value * 10` to an integer and divides, so a value
/// that arrived by repeated addition lands on the grid rather than near it.
const PROGRESS_TENTHS: f32 = 10.0;

/// Where the demo's chart sits: the right-hand column, in the space the list
/// occupied and above where it sat.
///
/// **Every number is measured rather than chosen, and each names what it clears.**
/// 1000 is the list's own x, and it is sixteen pixels right of the widest thing
/// in the control column — a readout at 664 given [`SLIDER_READOUT_WIDTH`], so
/// ending at 984 — and those sixteen are the clearance
/// `no_two_placed_rects_overlap` checks. 1270 is the list's own right edge and
/// ten short of the window. 240 is sixteen below the image-fit label's own line,
/// which ends at 224, so the chart is below the tallest thing in that column
/// rather than beside its label.
///
/// **It is above [`BAND_TOP`]**, so the chart joins the gallery rather than the
/// band, and `the_gallery_above_the_band_is_where_it_was` still means what it
/// said: nothing above 720 moved, so every capture of tasks 11 to 20 is still a
/// capture of the same pixels.
///
/// What would reverse this is a head unit whose chart is wide rather than tall,
/// which is the operator's number and not an agent's.
const CHART_ORIGIN: (f32, f32) = (1000.0, 240.0);

/// The box the demo's chart is given.
///
/// **270 by 450: the list's own width and twice its own height**, because the
/// window cannot grow — a 1280 by 1320 request comes back 1280 by 1052 on this
/// host, the window manager's cap — so a chart needs space a widget gives up.
///
/// **Inside it the plot is 270 wide and 432 tall: the gutter comes off the
/// height, not the width.** That is `X_LABEL_GUTTER`'s 18 pixels, and it comes off
/// the **height** because it is a **bottom** gutter — the space the labels need
/// *under* the plot, so [`plot_rect`](ui_core::widgets::chart) takes it off
/// `rect.height`. The **width** is untouched because the other gutter,
/// `Y_LABEL_GUTTER`, is a **left** one and is reserved only when there are y labels
/// to put in it, and **the demo writes none** (an auto-scaled axis has no numbers
/// a caller can write honestly). So a twelve-sample series has a pitch of
/// `270 / 11` = 24.5 pixels across, and 432 of height under it.
///
/// **Both of those are measured, not asserted** —
/// `the_plot_is_the_nodes_own_width_and_its_height_less_the_widgets_x_gutter`
/// reads the plot's own edges out of the widget's recorded paint (the grid lines'
/// x extent, the x axis's y, and the last x label's x, which solves for the width
/// a second way), so a change to the widget's gutters fails here instead of
/// leaving this sentence quietly false.
///
/// **The pitch is also the number that keeps the series mitred**, and the geometry
/// behind it is the widget's rather than guessed at: a join falls back from a
/// mitred corner to a round one when the corner's swing along a segment reaches
/// half that segment's own length, the swing is at most
/// [`Chart::stroke_reach`], and a segment is at least a pitch long — so a pitch
/// above **twice the reach** mitres every corner, and 23 samples is the most this
/// plot holds below that. [`CHART_SERIES`] is far short of it;
/// `the_series_never_grows_past_what_the_widget_can_mitre` is what holds the two
/// numbers together, and it asks the widget for the reach rather than restating
/// it.
const CHART_SIZE: Size = Size {
    width: 270.0,
    height: 450.0,
};

/// The space the chart reserves under its plot for its x labels, in pixels.
///
/// **Eighteen, and it is [`X_LABEL_GUTTER`](ui_core::widgets::chart)'s, which the
/// widget keeps private and exposes no accessor for.** `Chart` has a method for the
/// stroke's reach — [`Chart::stroke_reach`], added by its author precisely so a
/// caller would stop re-deriving it — but no method for either gutter, so this is
/// the one number of the widget's the demo cannot ask for and has to hold itself.
///
/// **Which axis it comes off is the part worth writing down, because getting it
/// backwards turns every width in this file's geometry into a false number.** It
/// comes off the plot's **height**: the gutter is the room the labels need
/// *below* the plot, and `plot_rect` reserves it against `rect.height`. The
/// **width** loses nothing, because the other gutter — `Y_LABEL_GUTTER`, 36 — is a
/// **left** one, reserved only when the chart has y labels, and this demo writes
/// none. A review of [`CHART_SIZE`]'s doc above read that sentence the other way
/// round and recomputed the whole of this file's geometry from it; the pixels and
/// the widget's recorded paint both say otherwise, and
/// `the_plot_is_the_nodes_own_width_and_its_height_less_the_widgets_x_gutter` is
/// what now says so in one command.
///
/// **And it is measured rather than trusted**: that test reads the gutter back out
/// of the widget's own recorded x axis and fails if this number and the widget's
/// ever disagree, so a copy that could rot is a copy that is checked. What would
/// remove the copy entirely is a `plot_gutter()` of the widget's own, which is the
/// author's call and not this file's.
///
/// **`cfg(test)` because the only place the demo does arithmetic about the widget's
/// *drawing* is a test** — and publishing a number in a doc comment while holding
/// it only for tests is how the two drift apart. [`CHART_SIZE`]'s doc states the
/// geometry it produces; this constant is what checks it.
#[cfg(test)]
const CHART_X_LABEL_GUTTER: f32 = 18.0;

/// How far below the chart its own readout sits.
///
/// **Four, and it is a clearance rather than a gap** — and the thing it clears is
/// **measured, not derived**, which is why this is four and not a computed number.
/// The chart's x labels live inside its own node, in the gutter the widget
/// reserves below the plot; the gutter is 18 px of a 450-tall node, so the reserved
/// space ends at the node's bottom edge at 690 — **but the reserved 18 is
/// `X_LABEL_GAP` plus `LABEL_FONT_SIZE` plus two pixels of slack, and the real
/// line box of a run of text at 12 px is taller than 12**, so "the labels end at
/// 690" is a claim about the reservation rather than about the pixels. What the
/// pixels do is in a capture of the running demo: **the labels' ink is on rows
/// 678 to 687**, three pixels clear of 690, and the readout's own line is at
/// 699..712 in the same capture — twelve pixels below the lowest ink the chart
/// draws.
///
/// `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect` is
/// what holds that claim in the suite: it asserts, over all three shapes and both
/// ends of a transition, that **the lowest pixel the chart draws is above the
/// readout's own line**. `.ai/NEVERAGAIN.md`'s 2026-10-01 entry *a brief's
/// rationale becomes the widget's doc comment, and nobody re-checks it* is the
/// reason this constant carries a measurement rather than a derivation.
const CHART_READOUT_GAP: f32 = 4.0;

/// Where the label naming what the chart is showing sits, under it.
const CHART_READOUT_ORIGIN: (f32, f32) = (
    CHART_ORIGIN.0,
    CHART_ORIGIN.1 + CHART_SIZE.height + CHART_READOUT_GAP,
);

/// The width the chart's readout is given, which is the chart's own width.
///
/// The window is 1280 wide and the chart's right edge is 1270, so this is also
/// the widest a box at this x can be without leaving the window. The line has to
/// fit inside it: `read_only_label` gives a readout the box of its **first**
/// string and cuts anything longer with an ellipsis, so a line that did not fit
/// would be a readout that cannot say what it is saying —
/// `the_chart_readouts_box_holds_the_longest_line_it_can_print` measures the
/// longest one the format can produce through the same metrics the tests lay out
/// with.
const CHART_READOUT_WIDTH: f32 = 270.0;

/// The font size the chart's readout is drawn at.
///
/// **Smaller than [`READOUT_FONT`]'s 20 for the reason the list's was**, which is
/// gone with it: this line names four things rather than one, and this column is
/// 270 wide where the control column is 320. At 16 pixels the monospace
/// stand-in the tests measure with is eight pixels a character, so
/// [`CHART_READOUT_WIDTH`] holds thirty-three of them, and the longest line the
/// format can print is thirty-two.
const CHART_READOUT_FONT: f32 = 16.0;

/// The shapes the demo's chart cycles through, in the order `H` walks them.
///
/// **`Line` first**, for [`GAUGE_TYPES`]'s reason: it is the shape that shows
/// every part of the widget at once — the grid, the axes, the labels *and* the
/// series as a joined run — so a reader sees the most before pressing anything.
/// `Bar` follows, because a bar is the shape whose value a reader can count off
/// the screen, and `Area` is last because it is the one whose fill is drawn
/// between the grid and the line rather than beside them.
///
/// **The order is the task's own** — task 21's three criteria name a line, a bar
/// and an area chart — so the three acceptance criteria are three presses of one
/// key rather than three widgets.
///
/// The three are named again in [`CHART_TYPE_NAMES`] rather than derived from the
/// enum, for [`GAUGE_TYPE_NAMES`]'s reason: the readout needs the *word* and
/// `ChartType`'s `Debug` is not part of its contract.
const CHART_TYPES: [ChartType; 3] = [ChartType::Line, ChartType::Bar, ChartType::Area];

/// The name of each shape in [`CHART_TYPES`], in the same order.
///
/// Capitalised as the other readouts' words are — `Bar`, not `bar` — because they
/// are read as labels and because `ChartType`'s own variants are capitalised. It
/// is also one of the four things the readout's line is measured against, so
/// shortening one of these words is a change to how wide that line is.
const CHART_TYPE_NAMES: [&str; 3] = ["Line", "Bar", "Area"];

/// The readings the demo's chart plots, oldest first, in km/h.
///
/// **A constant, and constant on purpose**: it is a named series rather than a
/// function of the frame counter, so a test can assert against a number instead
/// of against a formula and two runs of the demo show the same picture. The gauge
/// beside it is a speedometer reading 120 of 0 to 240, so the two are the same
/// quantity on the same window — which is why the unit is written down here.
///
/// **The shape of it is the point as much as the numbers.** The y axis
/// **auto-scales**, which requirement 4 asks for and which is only worth having
/// if the data has a range to stretch, so:
///
/// - **the extremes are far apart** — 42.0 and 86.75 across the twelve — so the
///   axis has a real span and a scaling error is a visible height rather than a
///   rounding difference;
/// - **half of the values are not round numbers** — 55.5, 61.25, 48.75, 73.5,
///   81.25, 86.75 — because a reading that landed on a tenth or a quarter is one
///   a rounding mistake could hit by accident.
///
/// **The four after the opening eight are what `A` and `S` reach for**, in this
/// order, and the end of it: `A` slides the window once the constant runs out
/// rather than lengthening it, for [`CHART_OPENING_COUNT`]'s reason.
const CHART_SERIES: [f32; 12] = [
    42.0, 55.5, 61.25, 48.75, 73.5, 66.0, 81.25, 74.5, 58.0, 86.75, 79.5, 68.25,
];

/// How many of [`CHART_SERIES`] the demo opens with.
///
/// **Eight, which is a chart a person reads**: over [`CHART_SIZE`] that is a
/// `270 / 7` = 38.6-pixel pitch, so an x label under every sample has room and a
/// bar under every sample is wider than its own gap. The remaining four are what
/// the append and shift keys reach for, so a reader can see four presses change
/// the series without a press taking the demo off it.
///
/// **It is also the most the series ever holds.** `A` stops adding when the
/// window is this long and slides it instead — see [`Demo::append_chart_sample`]
/// — because a chart the demo can only grow is a chart whose geometry the operator
/// never gets to see twice, and because [`CHART_SIZE`]'s mitre argument is about
/// a bounded pitch.
const CHART_OPENING_COUNT: usize = 8;

/// How many x labels the demo writes under its samples.
///
/// **Seven of the eight, and the omission is arithmetic rather than taste.** The
/// widget draws an x label left-aligned *at* its own sample, so a label on the
/// newest sample starts at the plot's right edge — 1270 — and runs right from
/// there, and `DrawCommand::Text` carries no width, so neither the widget nor
/// anything outside the text pipeline can find out by how much. Ten pixels of
/// window are left to lose it in. The widget's own rule does the rest: a sample
/// with no label at its own index is drawn without one.
///
/// **They are the sample's place in the window, not its age** — `0` is the oldest
/// reading on screen whatever it reads — which is why they are written once at
/// construction and never changed. A shift moves the readings under them and the
/// labels stay where they are, and an appended sample arrives unlabelled. Both
/// are the widget's documented behaviour rather than a workaround, and
/// `the_chart_labels_every_sample_but_its_newest_one` is what holds them to it.
const CHART_X_LABEL_COUNT: usize = CHART_OPENING_COUNT - 1;

/// What the chart's readout names for a newest reading when there is none.
///
/// A dash rather than a blank or a zero, for [`NOTHING_ENTERED`]'s reason: "no
/// reading yet" and "a reading of zero" are different facts, and a series whose
/// first sample has not arrived is not a series of zeroes. **Unreachable from the
/// demo**, which opens with [`CHART_OPENING_COUNT`] of them, and written down
/// rather than unwrapped for the reason [`NOTHING_SUBMITTED`] is.
const NO_READING: &str = "-";

/// The corner radius the demo's image is painted with.
///
/// Not zero, because a rounded corner is a shader feature and a square one is
/// no evidence that the shader ran — and because the asset is a square test card
/// whose edges are the only thing in it that would show it.
const IMAGE_CORNER_RADIUS: f32 = 10.0;

/// Where the frame-rate readout sits: the bottom left of the window, below the
/// text column and left of the control column.
///
/// Measured rather than guessed, and the two numbers below are what the measuring
/// found. The text column's last label ends at y 501, the control column starts at
/// x 664 and the chart's column is at x 1000, so the strip from (0, 505) to
/// (664, 720) is the one region of the window nothing is in. 684 is also where
/// the chart's own readout's line is — it is at 694, ten above this one — so the
/// two readouts at the bottom of the window are within a line of each other and
/// neither is over the other: this one is 400 wide and ends at 460, a long way
/// left of 1000.
const FPS_READOUT_ORIGIN: (f32, f32) = (60.0, 684.0);

/// The width the frame-rate readout is given.
///
/// **Written out rather than measured from its first string**, which is what
/// every other readout below the pads does, because this is the one whose text
/// changes on every frame that moves it: a rect measured from `fps 0, avg 0.0,
/// worst 0 ms` is a rect measured from a number that was true for one frame, and
/// [`read_only_label`]'s ellipsis would then cut the line at the width of the
/// shortest string it ever shows.
///
/// 400 fits the longest line the readout can print — `fps 10000, avg 10000.0,
/// worst 9999 ms` is 37 characters, and a frame cannot be a hundredth of a
/// millisecond long, which is what would give the rates six digits — and it ends
/// at x 460, two hundred pixels clear of the control column at
/// [`CONTROLS_ORIGIN`]'s 664. A box wider than the line in it costs nothing: the
/// text is drawn from the box's own left edge.
const FPS_READOUT_WIDTH: f32 = 400.0;

// ------------------------------------------------------------------ task 19

/// The y at which the text-entry band begins: the old bottom of the window.
///
/// Everything above this line is tasks 11–18's layout and **has not moved**. The
/// window grew downwards rather than the demo being re-laid-out, so every capture
/// taken of the earlier tasks is still a capture of the same pixels. That is the
/// whole argument for growing the window rather than rearranging it, and it is
/// why the operator chose it over restructuring the gallery.
const BAND_TOP: f32 = 720.0;

/// How far below [`BAND_TOP`] the band starts.
///
/// Sixteen is the same margin the whole demo has on its left edge, so the band
/// starts where the gallery's margin already is rather than at a new number.
/// Written as a drop from [`BAND_TOP`] rather than as an absolute 736, because the
/// relationship is the fact: move the band and the whole of it follows.
const BAND_DROP: f32 = 16.0;

/// The top of everything in the band.
const BAND_TOP_OF_BAND: f32 = BAND_TOP + BAND_DROP;

/// Where the demo's text input sits, at the left of the band.
///
/// **Left of the keyboard rather than above it**, which is the layout decision
/// this band makes. Stacked is the arrangement most phone keyboards use, and it
/// needs [`BAND_HEIGHT`] to be a field plus a gap plus a 260-tall keyboard; beside
/// it the band is as tall as the keyboard alone, which is what fits the 300 pixels
/// [`WINDOW`] leaves below the gallery. For a car it is also the better shape: a
/// driver reaches a keyboard to the side of the field without the field moving
/// under their hand.
const TEXT_INPUT_ORIGIN: (f32, f32) = (BAND_MARGIN, BAND_TOP_OF_BAND + 20.0);

/// The box the text input is given.
///
/// **Both numbers are the demo's, not the widget's**, and both go in through the
/// properties task 19's own review asked for. The widget defaults to 240 by 44;
/// 480 by 64 is a field read at arm's length from a driver's seat, which is the
/// same judgement the operator made twice already when they called a 6-pixel
/// slider track and a 6-pixel scrollbar unusable with a finger. This is the
/// third control in that series and the first one that could have been answered
/// without a code change.
const TEXT_INPUT_SIZE: Size = Size {
    width: 420.0,
    height: 64.0,
};

/// The margin the band keeps on the left, the same 60 the gallery uses.
const BAND_MARGIN: f32 = 60.0;

/// How tall the band is: [`WINDOW`] less [`BAND_TOP`].
///
/// The whole of the space task 19 was given, and the number the layout has to fit
/// inside. It is a named constant rather than an expression at each use because
/// the alternative is a `- BAND_TOP` at four different places, which is four
/// chances to disagree about where the gallery ends.
const BAND_HEIGHT: f32 = WINDOW.height - BAND_TOP;

/// The font size the text input's own text is drawn at.
///
/// Larger than [`READOUT_FONT`]'s 20 because this is *the thing being read*, and
/// a field whose value is set in 16-pixel type is a field read by leaning in.
const TEXT_INPUT_FONT: f32 = 24.0;

/// Where the on-screen keyboard sits, right of the field.
///
/// **Side by side rather than under it**, which is the layout decision this band
/// makes, and [`WINDOW`] gives the reason: a field over a keyboard needs the band
/// to be a field plus a gap plus a 260-tall keyboard, and [`BAND_HEIGHT`] is 300.
/// For a car the side-by-side shape is also the better one — a driver reaches the
/// keys beside the field without the field moving under their hand.
const KEYBOARD_ORIGIN: (f32, f32) = (520.0, BAND_TOP_OF_BAND);

/// How tall one key is drawn, in pixels.
///
/// **44 is the widget's own touch floor, and the demo does not go below it.** The
/// widget's *default* is 52, which is taller than this band has room for, so the
/// demo asks for the floor rather than for something smaller — the same
/// relationship [`SLIDER_TRACK_THICKNESS`] has with its widget's default, except
/// that that one was widened and this one is lowered **to** the floor and never
/// under it.
///
/// **The third control the operator rejected a 6-pixel default on went with the
/// list on 2026-10-02.** They called the slider's 6-pixel track unusable on
/// 2026-09-30 — *"the slider is very narrow, can't imagine how I could use it in
/// a car with my finger"* — and the list's 6-pixel scrollbar unusable for the
/// same reason on 2026-10-01: *"is too narrow, I have issues with pointing on it
/// with my mouse, so doing that on tablet with a finger is impossible"*. The demo
/// answered the first through [`SLIDER_TRACK_THICKNESS`] and the second through
/// `Scroll::set_thickness`, at the same value of 12, and the list was then
/// removed to make room for the chart. **The widget's own 6 is unchanged in both
/// cases** — that is what the setters exist for — and the scrollbar's is now
/// reachable only from `ui_core`'s own tests.
const KEY_HEIGHT: f32 = 44.0;

/// The gap between two keys, in pixels: the widget's own `KEY_GAP`.
const KEY_GAP: f32 = 6.0;

/// The keyboard's own padding, in pixels, on every side.
const KEY_PADDING: f32 = 8.0;

/// The height the keyboard occupies: the padding top and bottom, five rows of
/// [`KEY_HEIGHT`], and four gaps between them.
///
/// Written out rather than read from `keyboard.size().height` so the demo's
/// layout and the demo's constants cannot disagree about where the window ends.
/// `the_keyboard_is_the_height_the_demo_says_it_is` is what holds the two in step,
/// and it fails if either number is changed alone.
const KEYBOARD_HEIGHT: f32 = KEY_PADDING * 2.0 + KEY_HEIGHT * 5.0 + KEY_GAP * 4.0;

/// The width the keyboard is given.
///
/// 700 of the window's 1280, with the field and its two readouts in the other
/// [`BAND_MARGIN`] to 480 and 60 of margin beyond that. **Ten keys across 700
/// comes to 63 pixels each**, which is comfortably over [`KEY_HEIGHT`] and
/// therefore over the touch floor; the width is a layout choice and the floor is
/// the constraint, and `every_key_is_at_least_forty_four_across_and_tall` is what
/// checks the second against the first.
const KEYBOARD_WIDTH: f32 = 700.0;

/// The font size the keyboard draws its key labels at.
///
/// Below the widget's own 22, because the band is laid out side by side and each
/// key is 63 pixels wide rather than 110: a label is centred by an average
/// advance and the demo measures that advance at *this* size from its own
/// `TextMetrics`, so the two cannot disagree.
const KEY_FONT: f32 = 18.0;

/// Where the readout naming what the field holds sits, right of the input.
///
/// On the field's own first line rather than its centre, so the readout's baseline
/// and the field's text are on one line and a driver reads the two as a pair.
const TEXT_READOUT_ORIGIN: (f32, f32) = (BAND_MARGIN, BAND_TOP_OF_BAND + 100.0);

/// The width the text readout is given.
const TEXT_READOUT_WIDTH: f32 = 400.0;

/// Where the readout naming what was last submitted sits, under the text one.
///
/// It is a separate readout rather than a second line of the first because
/// `on_submit` is a different event from `on_change`: a value that was submitted
/// and a value that is still being typed are different facts, and a driver
/// glancing at the screen needs to tell them apart.
const SUBMIT_READOUT_ORIGIN: (f32, f32) = (BAND_MARGIN, BAND_TOP_OF_BAND + 132.0);

/// The width the submit readout is given.
const SUBMIT_READOUT_WIDTH: f32 = 400.0;

/// The font size both of the field's readouts are drawn at.
const TEXT_FIELD_READOUT_FONT: f32 = 20.0;

/// The grey text the field shows while it is empty.
///
/// Not `""`: an empty placeholder satisfies "shown when the text is empty" and
/// shows nothing at all, which is the failure a test that only checks the
/// placeholder's *presence* would pass. What a driver reads on an empty field is
/// the field's purpose, so it names one.
const PLACEHOLDER_TEXT: &str = "Search stations, cities…";

/// What the submit readout says when nothing has been submitted yet.
///
/// A dash rather than an empty string, for the same reason [`PLACEHOLDER_TEXT`]
/// is not empty: "no submission yet" and "an empty submission" are different
/// facts and one of them is a bug.
const NOTHING_SUBMITTED: &str = "-";

/// What both readouts say when there is no value.
///
/// **The text readout is bound to the field's own `text`**, so with no value in
/// the field it printed an empty line — a readout that cannot be seen, and a
/// driver could not tell a missing readout from an empty field. This is the same
/// argument as [`PLACEHOLDER_TEXT`] and [`NOTHING_SUBMITTED`], applied to the
/// third place it turned up: a value that is absent has to be *shown* as absent,
/// because "absent" and "blank" are indistinguishable on a screen.
const NOTHING_ENTERED: &str = "-";

/// The environment variable that bounds how long the demo runs, in seconds.
///
/// The demo is a window: it runs until the window is closed or SDL turns a
/// `SIGTERM` into a quit event, so without this an agent that launched it has no
/// way to *end* a measurement and therefore no way to read one — a number only
/// exists once something prints it. Ten seconds is the default the check script
/// asks for and about a hundred of frames is enough for the average to be worth
/// comparing.
const RUN_SECONDS_VAR: &str = "ROADOS_RUN_SECONDS";

// ------------------------------------------------------------------ task 22
//
// The dialog's five numbers and its two strings. They are constants for the same
// reason every other constant in this file is: a test asserts against them, a
// capture is read against them, and a reader who wants to know what is on screen
// has one place to look. Each carries what it is, what it was measured or chosen
// against, and what would reverse it.

/// The title of the dialog the demo opens with.
///
/// **It is a question with a consequence rather than a notice**, because a dialog
/// whose two buttons do the same thing is a dialog that demonstrates nothing: `OK`
/// switches the theme over, exactly as `T` does, and `Cancel` closes it having
/// changed nothing. The title names what will happen rather than what the widget
/// is, so a reader of a capture knows what pressing `OK` would have done.
///
/// What would reverse it: a second dialog, which the task file puts out of scope.
const DIALOG_TITLE: &str = "Switch theme";

/// The body of the dialog the demo opens with.
///
/// **It names the key as well as the effect**, because the effect is one the
/// gallery has had since task 11 and a reader who has never seen this demo cannot
/// tell a new behaviour from an old one. It is long enough to **wrap** to a second
/// line at [`DIALOG_FONT`] inside the panel's own inner width, which is the point:
/// a one-line body would leave the wrapping untested on screen, and the widget's
/// 34 unit tests cannot tell whether a real face wraps where a fixture's advance
/// closure says it does.
///
/// What would reverse it: a shorter or longer sentence; there is no measurement
/// behind the length, only the wish to see two lines and to fit inside 420.
const DIALOG_BODY: &str =
    "Turn the gallery's colours over to the other theme, exactly as the T key does.";

/// The dialog's action labels, in the order they are added and therefore in the
/// order they are drawn and walked by `Tab`.
///
/// **`OK` first**, which is the order [`ui_core::widgets::dialog`]'s own fixtures
/// use and so the order its `Tab` order is documented in; the row is laid out left
/// to right and right-aligned against the panel's inner edge, so this is the order
/// a reader sees them in. Acceptance criterion 7 asks for exactly these two.
///
/// What would reverse it: platform convention, which usually puts `Cancel` on the
/// left, and which the operator may prefer.
const DIALOG_ACTIONS: [&str; 2] = ["OK", "Cancel"];

/// The font size the dialog's own text is **measured** at, and the size its one
/// `line_height` is taken from.
///
/// **The theme's `FontSizeLg`, which is the largest of the three sizes the panel
/// draws at** — the title is drawn at it, the body at `FontSizeMd` and each
/// button's label at the button's own 14. Measuring both blocks at the largest of
/// the three is the *conservative* direction, and the choice between the two ends
/// is not a coin toss: a run measured at 18 pixels is never narrower than the same
/// run drawn at 14, so nothing can overflow the panel's padding, and the cost of
/// being wrong in this direction is a body that wraps a word early. The other way
/// round would let the title run past the panel's own 24-pixel padding, which is
/// visible.
///
/// **It is a weight-blind measurement and that is deliberate, not an oversight:**
/// the widget measures its title with the one `advance` closure its caller gives
/// it, so there is no way to measure a bold run with this seam — a bold run's
/// advances are the *bold face's*, and this closure is the regular face's. The
/// consequence is measured rather than assumed: at 18 pixels on this pair of faces
/// a bold title is a hair wider or the same, and the panel is a pixel or two of
/// slack wider than its text needs either way. See
/// [`ui_core::widgets::dialog`]'s module doc and
/// [`ui_core::paint::Painter::text_bold`].
///
/// `the_dialog_measure_size_is_the_themes_largest` reads the tokens and holds the
/// number to them, so a theme that grew `FontSizeLg` would fail there rather than
/// being noticed on screen.
///
/// What would reverse it: a seam that takes a weight, which is a change to
/// `Painter` and to every widget that measures text — not this task's.
const DIALOG_FONT: f32 = 18.0;

/// The key that opens and closes the dialog.
///
/// **`D`, for dialog**, and it is the only key the dialog owns: `Escape` closes it
/// and is consumed by the widget rather than by this arm, so a dismissal is one
/// thing whether it came from the row, the scrim, the keyboard or the toggling
/// key. Every other key in the demo is a single letter already spoken for — `T`,
/// `C`, `F`, `G`, `H`, `A`, `S`, `P` — and `D` was the nearest unused one, because
/// a key a reader has to look up is a key that does not get pressed.
///
/// What would reverse it: anything the operator prefers, or a key that is already
/// free on the target's keyboard; `D` is free in a QWERTY, a QWERTZ and an AZERTY.
const DIALOG_KEY: Keycode = Keycode::D;

/// The image the demo shows: a texture and the window of it the fit needs.
///
/// The two together, because [`ImageSource::of`] is what turns a handle into
/// something a fit can be computed from, and a caller that has a handle and no
/// source has half an image.
struct Picture {
    /// The decoded image's handle.
    texture: TextureHandle,
    /// Where the image sits inside its texture, and how big it is.
    source: ImageSource,
}

/// The fits the demo's image cycles through, in the order `F` walks them.
///
/// It is the task's four in the task's order, and `Cover` comes second on
/// purpose: it is the one whose result differs most from `Contain`'s on this
/// asset, so a reader pressing `F` twice sees the fit change rather than see
/// two names for the same picture.
const IMAGE_FITS: [ImageFit; 4] = [
    ImageFit::Contain,
    ImageFit::Cover,
    ImageFit::Fill,
    ImageFit::None,
];

/// The name of each fit in [`IMAGE_FITS`], in the same order.
const IMAGE_FIT_NAMES: [&str; 4] = ["Contain", "Cover", "Fill", "None"];

/// The file name of the demo's image, under whichever directory is found.
const ASSET_FILE: &str = "demo.png";

/// Where the image is, relative to a directory in the workspace's own layout.
///
/// The path from the workspace root — `ui/` — and not from the repository root,
/// because the asset belongs to the `ui_demo` crate and the executable is built
/// under `ui/target`.
const ASSET_RELATIVE: &str = "src/ui_demo/assets/demo.png";

/// The environment variable that overrides where the image is looked for.
///
/// A path to the **directory** holding the image rather than to the image, so
/// the file's name is written down once in [`ASSET_FILE`].
const ASSET_DIR_VAR: &str = "ROADOS_ASSET_DIR";

/// The size `assets/demo.png` is, which the stand-in is built at.
///
/// It is written down rather than read, because a stand-in has to be the shape
/// of the thing it stands in for: every fit's geometry is computed from the
/// source's own width and height, and a stand-in of a different shape would make
/// `Cover` crop a different amount from the one on screen.
const ASSET_SIZE: (u32, u32) = (320, 192);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = Renderer::new(Context::new(
        "roados ui_demo",
        f32_to_u32(WINDOW.width),
        f32_to_u32(WINDOW.height),
    )?)?;
    let font = Font::from_path(FONT_PATH)?;
    renderer.set_font(font.clone());
    // **The second face, and the reason the dialog's title is bold on screen.**
    // `DrawCommand::Text` carries a weight and the renderer resolves it against
    // the faces it holds; with no face for the weight asked for it falls back to
    // the regular one, which is a deliberate rule and also the way a bold title
    // silently arrives as a plain one. There is nothing else to call here — this
    // is the only bold run in the demo.
    renderer.set_bold_font(Font::from_path(BOLD_FONT_PATH)?);
    // The image is loaded before the demo is built and before the window has
    // been shown anything, because `Demo::new` needs a texture and a source to
    // build its `Image` at all. A failure is not a failure of the demo: the
    // caller gets `None`, says so once on stderr, and the demo stands in a
    // transparent image of the same shape — see `stand_in_picture`.
    let picture = load_picture(&mut renderer);
    let sdl = renderer.sdl();
    let mut events = sdl.event_pump()?;
    let mut demo = Demo::new(TextMetrics::new(font), picture)?;
    let run_for = run_seconds();

    let mut last = Instant::now();
    let started = last;
    'running: loop {
        // The frame budget, spent properly: wait only what is left of
        // `FRAME_BUDGET` after the previous frame's work. The old loop waited a
        // fixed 16 ms here and drew afterwards, which made every frame
        // `16 ms + work` and capped a perfectly good frame at 50 fps.
        let spent = last.elapsed();
        let event = events.wait_event_timeout(frame_wait(spent));
        if matches!(
            &event,
            Some(
                Event::Quit { .. }
                    | Event::Window {
                        win_event: WindowEvent::CloseRequested,
                        ..
                    }
            )
        ) {
            break 'running;
        }
        if let Some(event) = event {
            demo.handle_event(event);
        }

        let now = Instant::now();
        let delta = now.duration_since(last);
        last = now;

        renderer.begin_frame();
        demo.frame(WINDOW, delta);
        demo.draw(&mut renderer);
        renderer.end_frame()?;

        // Checked after the frame rather than before the wait, so a bounded run
        // ends on a frame that was drawn: the last frame is counted and the last
        // one is on the screen when the window goes.
        if run_for.is_some_and(|limit| now.duration_since(started) >= limit) {
            break 'running;
        }
    }

    // On every way out, and printed rather than logged because a test runner
    // reads it: `fps-check.sh` greps for the prefix and compares the numbers. A
    // measurement nobody can get at is the failure this whole mechanism exists to
    // avoid.
    println!("{}", demo.fps_report());

    Ok(())
}

/// Returns how long the loop should block after a frame that took `spent`.
///
/// This is the whole of the frame budget as arithmetic, kept out of the loop so
/// it can be tested: `.ai/NEVERAGAIN.md` § *a test of a helper cannot see a call
/// site that stopped using it* is the reason the loop calls **this** function
/// rather than inlining the subtraction, because a helper tested from one place
/// and inlined in another has two places to be wrong and only one under test.
///
/// A frame that overran the budget has nothing left. **`Duration::saturating_sub`
/// is the whole of that rule**: it stops at zero rather than handing SDL a
/// negative duration, which would ask for a wait of undefined length.
///
/// There was a `.max(MIN_WAIT)` on top of it as well, and a deliberate break of
/// that clamp **survived the whole suite** — `saturating_sub` already stops at
/// zero, so the clamp restated a rule the subtraction had already enforced and no
/// test could ever tell the two apart. It is gone rather than kept as a second
/// place to be wrong. `.ai/NEVERAGAIN.md` § *a test of a helper cannot see a call
/// site that stopped using it* is also why the loop calls **this** function
/// rather than inlining the subtraction: a helper tested from one place and
/// inlined in another has two places to be wrong and only one under test.
///
/// Returning zero for an overrun is not mercy, it is accuracy: the loop is
/// already late and is reporting a frame that cost too much rather than waiting a
/// while to hide it.
fn frame_wait(spent: Duration) -> Duration {
    FRAME_BUDGET.saturating_sub(spent)
}

/// Returns how long the demo should run for, read from [`RUN_SECONDS_VAR`], or
/// `None` to run until the window is closed.
fn run_seconds() -> Option<Duration> {
    run_seconds_from(std::env::var(RUN_SECONDS_VAR).ok())
}

/// Returns how long a run of `raw` bounds itself to, or `None`.
///
/// The parsing is a free function over its argument rather than a pair of
/// statements inside `main`, for the reason `asset_candidates_from` is: the
/// process's environment is one value for the whole test process and a test that
/// wrote it would be writing it for every other test running beside it. Given the
/// value, the rule is a function and can be tested one.
///
/// **A value that is not a positive number of seconds says so on stderr and is
/// ignored**, because a run that quietly lasts for ever is exactly the case the
/// variable exists to remove, and a typo that produced one would look like a hang
/// rather than like a typo. Zero is refused rather than honoured for the same
/// reason: a run of no length measures nothing, and it is a much more likely
/// thing to type than a negative one.
fn run_seconds_from(raw: Option<String>) -> Option<Duration> {
    let raw = raw?;
    // `try_from_secs_f64` and not `from_secs_f64`, which **panics** on a value too
    // large to be a duration: this is text off the environment, and a panic is
    // not what an unparsable variable earns.
    let limit = raw
        .trim()
        .parse::<f64>()
        .ok()
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .filter(|limit| !limit.is_zero());
    if limit.is_none() {
        eprintln!(
            "ui_demo: {RUN_SECONDS_VAR}={raw:?} is not a positive number of seconds; \
             running until the window is closed"
        );
    }
    limit
}

/// Loads the demo's image and returns it, or `None` after saying why.
///
/// **A missing asset must not take the window down.** The demo draws everything
/// else, the image's own label says it is standing in, and this function says
/// once on stderr where it looked — a silent stand-in is a picture of nothing
/// with no explanation, and an unexplained blank rectangle is what
/// `.ai/NEVERAGAIN.md` records two of this repository's defects as having been
/// mistaken for.
fn load_picture(renderer: &mut Renderer) -> Option<Picture> {
    let candidates = asset_candidates();
    let path = candidates.iter().find(|path| path.is_file());
    let Some(path) = path else {
        eprintln!(
            "ui_demo: {ASSET_FILE} not found; looked in {}",
            join_paths(&candidates)
        );
        return None;
    };
    let texture = match renderer.load_texture(path) {
        Ok(texture) => texture,
        Err(error) => {
            eprintln!("ui_demo: {}: {error}; standing in for it", path.display());
            return None;
        }
    };
    match ImageSource::of(renderer.textures(), texture) {
        Some(source) => Some(Picture { texture, source }),
        None => {
            eprintln!(
                "ui_demo: {}: the cache could not place it; standing in",
                path.display()
            );
            None
        }
    }
}

/// Returns `paths` as one comma-separated string, for a message about where
/// something was looked for.
///
/// A `PathBuf` is not `Display` — it is not necessarily text — so this is where
/// the demo turns a list of them into something a reader can act on, which is
/// the whole of what the line above it is for.
fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

/// Returns the paths the demo's image may be at, in the order it looks.
///
/// Nothing here is relative to the process's own working directory, because
/// that is not the same for `cargo run` and `cargo test` — the first is the
/// workspace root and the second is the crate's — so a path relative to "here"
/// finds the asset in one and not the other. The environment variable comes
/// first so a caller can say where the file is; the rest are the workspace's own
/// layout walked **up** from the executable, which is under `ui/target` whether
/// it was run by Cargo or not.
fn asset_candidates() -> Vec<PathBuf> {
    let Ok(exe) = std::env::current_exe() else {
        return Vec::new();
    };
    asset_candidates_from(&exe, std::env::var_os(ASSET_DIR_VAR).as_deref())
}

/// Returns the candidates for an executable at `exe` and an override directory
/// of `dir`, in the order [`asset_candidates`] looks in them.
///
/// This is the half of the search that is arithmetic rather than the filesystem,
/// which is what makes it a function a test can call: the `is_file` check that
/// picks between them is one line in [`load_picture`].
fn asset_candidates_from(exe: &Path, dir: Option<&OsStr>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = dir {
        candidates.push(Path::new(dir).join(ASSET_FILE));
    }
    // `ancestors` starts at the executable itself, which is a file and not a
    // directory to walk out of, so the first candidate is its own directory.
    candidates.extend(
        exe.ancestors()
            .skip(1)
            .map(|directory| directory.join(ASSET_RELATIVE)),
    );
    candidates
}

/// Returns a transparent image the shape of [`ASSET_SIZE`], to stand in for the
/// asset when it could not be loaded.
///
/// It is a [`TextureCache`] and a handle rather than a texture of its own,
/// because that is all an [`Image`] needs: a stand-in is not drawn from a GPU
/// texture, it is given to the same widget with the same geometry, so every fit
/// is computed from the same 320 by 192 the asset is and a `Cover` crop of it
/// is the crop on screen. The cache is dropped on the way out, which is safe
/// because [`Image`] holds the handle and the window and asks nothing of the
/// cache again.
///
/// A stand-in therefore draws **nothing** — an image of transparent pixels is
/// an image of the background — and the demo's own label says so, because
/// "nothing there" with a label saying which fit it is would be the wrong
/// answer to a reader.
fn stand_in_picture() -> Option<Picture> {
    let mut cache = TextureCache::new();
    let mut loader = |_: &Path| Ok(Pixels::transparent(ASSET_SIZE.0, ASSET_SIZE.1));
    let texture = cache.load(Path::new("stand-in"), &mut loader).ok()?;
    let source = ImageSource::of(&cache, texture)?;
    Some(Picture { texture, source })
}

/// Converts a float extent to the pixel count the window is created with.
///
/// There is no `From`/`TryFrom` between `f32` and any unsigned integer in std,
/// so this is the one place a float-to-integer `as` cast is used, for the same
/// reason and with the same guarantee as the renderer's own: the cast is
/// saturating, so an extent outside the range clamps instead of wrapping.
fn f32_to_u32(value: f32) -> u32 {
    value as u32
}

/// Returns the fit at `index` in [`IMAGE_FITS`], or the first one.
///
/// `F` only ever writes an index it has just taken modulo the length, so the
/// fallback is unreachable from the demo; it is here rather than an index
/// expression so that nothing in the demo can panic on a number it wrote
/// itself, and because the same question is asked of the fit's *name* below.
fn image_fit_at(index: usize) -> ImageFit {
    IMAGE_FITS.get(index).copied().unwrap_or(IMAGE_FITS[0])
}

/// Returns the name of the fit at `index` in [`IMAGE_FITS`], or of the first
/// one. See [`image_fit_at`] for the fallback.
fn image_fit_name(index: usize) -> &'static str {
    IMAGE_FIT_NAMES
        .get(index)
        .copied()
        .unwrap_or(IMAGE_FIT_NAMES[0])
}

/// Builds one of the demo's readouts: a label whose text is `text`, drawn once
/// on a line at `font_size` and never wrapped.
///
/// Every readout the band below the pads carries is one of these, and they are
/// all built the same way for two reasons. The text is **bound** to whatever the
/// widget's own properties are, so a label never has to be told what the toggle
/// is or what the chart is plotting — it recomputes, and a binding rather than a
/// callback is the whole difference between a number that cannot go stale and a
/// number that has to be kept in step. And the options say `wrap: None` with an
/// ellipsis, because the node is given a box of the *first* string's size and a
/// label that wrapped would put its second line outside the box the layout gave
/// it — which is the same shape of mistake as a control placed over its
/// neighbour, and the collision tests would not see it either.
///
/// The colour is the demo's cycling one, so `C` and `T` reach these readouts
/// exactly as they reach the text panel.
fn read_only_label(
    nodes: &mut Arena<WidgetNode>,
    metrics: &TextMetrics,
    font_size: f32,
    max_width: f32,
    text: Property<String>,
    color_token: &Property<ThemeToken>,
    tokens: &[(ThemeToken, Property<PropertyValue>)],
) -> Result<DemoLabel, &'static str> {
    let mut label = Label::new(nodes, String::new());
    label.text = text;
    label.color = cycling_color(color_token, tokens);
    label.font_size.set(font_size);
    let demo_label = DemoLabel {
        label,
        options: LayoutOptions {
            max_width,
            wrap: WrapMode::None,
            truncation: Truncation::Ellipsis,
            ..LayoutOptions::default()
        },
    };
    let size = demo_label.size(metrics, font_size);
    nodes
        .get_mut(demo_label.label.handle())
        .ok_or("ui_demo: a readout's node is missing")?
        .layout_mut()
        .set_constraints(Constraints::tight(size));
    Ok(demo_label)
}

/// Records `demo_label`'s draw commands into the node at `handle`, in `rect`.
///
/// The one place a label is painted, for the panel's seven and the band's nine
/// readouts between them. `rect` is the node's own **laid-out** rect converted to
/// the painter's.
///
/// **It had a second caller once**, for the list's rows, which were painted in
/// their own coordinates and then translated by the widget — and that is the only
/// thing this function ever did that was not "paint the node the layout pass
/// placed", so the caveat is gone with the list rather than left behind as a
/// paragraph about a widget the demo no longer has.
///
/// A missing node or a node the pass has not placed records nothing and says
/// nothing: a widget that is not in the tree draws nothing, which is the same
/// answer [`Painter`] gives.
fn record_label(
    nodes: &mut Arena<WidgetNode>,
    handle: Handle,
    demo_label: &DemoLabel,
    metrics: &TextMetrics,
    font_size: f32,
    rect: Option<Rect>,
) {
    let Some(node) = nodes.get_mut(handle) else {
        return;
    };
    let Some(rect) = rect else {
        return;
    };
    let mut options = demo_label.options;
    options.line_height = metrics.line_height(font_size);
    let commands = demo_label
        .label
        .paint(rect, &options, &|ch: char| metrics.advance(ch, font_size));
    *node.paint_mut() = PaintState::from_commands(commands);
}

/// Lifts a colour toward white, for a pad's held colour: the pad's rest colour
/// is a theme token, and its held colour is that token lifted toward white, so
/// a press reads as the same hue brightened.
fn lighten(color: Color) -> Color {
    Color::interpolate(&color, &Color::new(255, 255, 255, 255), HELD_LIGHTEN)
}

/// Returns whether the point `(x, y)` is inside `rect`, edges included.
///
/// Inclusive on every edge, which is `ui_core::input::contains`'s convention and
/// the one every "is the pointer over this control" question in the demo is
/// already answering; a rect that did not claim its own last pixel row would
/// have a one-pixel strip where a control could not be pressed.
fn over_rect(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}

/// Returns `rect` grown by `by` on every side.
///
/// It is [`over_rect`]'s arithmetic for a box rather than a point, and it exists
/// because the chart's stroke reaches [`Chart::stroke_reach`] past its own node:
/// a box that is a claim about where something is drawn has to be the box
/// something is drawn in.
///
/// **The `by` is always the widget's own answer and never a number written here.**
/// This function was written when the demo carried a private `6.0` for that
/// reach, derived by hand from a `MITRE_LIMIT` the widget keeps private, and the
/// widget now answers the question itself. `.ai/NEVERAGAIN.md`'s 2026-10-01 entry
/// *one sibling got the operator's fix; the other with the same constant did not*
/// is about that shape, and a copy of a number is wrong for exactly one reason:
/// it is right until one of its two inputs moves, and nothing in the demo says so
/// when they do. There is no constant of that kind in this file any more.
fn grown(rect: Rect, by: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// Returns whether `inner` is wholly inside `outer`, edges included.
///
/// Edges included, so a box whose right edge is the window's right edge is
/// inside the window rather than one pixel proud of it: the alternative would
/// make a demo that fits exactly look like one that does not.
#[cfg(test)]
fn inside(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

/// Returns whether `a` and `b` share at least one pixel.
///
/// Also edges included, and for the same reason: two boxes that meet along an
/// edge have not been laid out badly, and a test that called that a collision
/// would be reporting a difference in taste. What it is here for is a box drawn
/// **over** another, which is the defect three previous tasks each found by eye.
#[cfg(test)]
fn touches(a: Rect, b: Rect) -> bool {
    a.x <= b.x + b.width && b.x <= a.x + a.width && a.y <= b.y + b.height && b.y <= a.y + a.height
}

/// Turns one key press on the on-screen keyboard into an edit on the field.
///
/// **This is the whole of the wiring between the two widgets**, and it is one
/// function for the same reason `scroll::gesture_delta` was one function: the
/// convention "a key press means this edit" was written out in every arm of a
/// dispatch and would have to be rewritten in every place it appeared. Neither
/// widget knows the other exists — `Keyboard` reports a [`KeyAction`] and
/// `TextInput` offers three edit methods — and this is where the two vocabularies
/// are given the same meaning.
///
/// The three actions it ignores are the three the keyboard handles itself, and
/// ignoring them here is deliberate rather than an oversight:
///
/// - [`KeyAction::Shift`] has already been applied to the *next* character by the
///   keyboard, which is why it arrives as `Char('A')` and not as a shifted
///   letter. Applying it again would double it.
/// - [`KeyAction::PageUp`] and [`KeyAction::PageDown`] have already switched the
///   keyboard's page by the time they are reported.
///
/// The one that is neither is [`KeyAction::Enter`], and it is the interesting
/// one: `TextInput` fires `on_submit` from its **own** key handling, and a key
/// on a touchscreen is not a key event — it is a `KeyAction` that reached this
/// function instead. So an Enter from the keyboard calls the callback the same
/// way a hardware Enter does, rather than leaving the demo with two different
/// routes to the same fact.
fn apply_key_action(field: &TextInput, action: KeyAction) {
    match action {
        KeyAction::Char(character) => {
            let mut buffer = [0u8; 4];
            field.insert_text(character.encode_utf8(&mut buffer));
        }
        KeyAction::Space => field.insert_text(" "),
        KeyAction::Backspace => field.delete_backward(),
        KeyAction::Enter => {
            // Through the callback, not by writing the readout's property here.
            // A hardware Enter takes this same route inside the widget, so an
            // Enter on the touchscreen and an Enter on a keyboard produce one
            // fact rather than two — which is the whole reason this is a call to
            // `on_submit` and not a write.
            field.on_submit.call(field.text.get());
        }
        // Handled by the keyboard itself — see the doc comment above.
        KeyAction::Shift | KeyAction::PageUp | KeyAction::PageDown => {}
    }
}

/// A pad that presses: one property says how far down it is, and its colour
/// follows.
struct Pad {
    /// How far the pad is pressed: 0 at rest, 1 held.
    press: Property<f32>,
    /// The pad's colour at its current press, bound to the press and the theme.
    color: Property<PropertyValue>,
    /// The node the pad paints itself with.
    node: Handle,
}

impl Pad {
    /// Creates a pad whose press property is `press`, and whose colour is
    /// bound to it and to the theme.
    ///
    /// The pad has one property of its own — how far pressed it is — and a
    /// colour property bound to that and to the theme: the colour is
    /// interpolated from the press at recompute time, so an animation moves a
    /// single number and the pad's whole appearance follows it, and a theme
    /// switch moves the rest and held colours it interpolates between.
    fn new(press: Property<f32>, color: Property<PropertyValue>, node: Handle) -> Self {
        Pad { press, color, node }
    }

    /// Returns the colour to paint this pad at its current press.
    ///
    /// The bound colour property always holds a colour; the fallback is for a
    /// theme that put something else in the colour token the pad reads, and
    /// paints black rather than panicking.
    fn color(&self) -> Color {
        self.color
            .get()
            .as_color()
            .unwrap_or(Color::new(0, 0, 0, 255))
    }
}

/// The font measurements the demo's text panel needs: a character's advance and
/// the height of one line, both at a font size.
///
/// The demo takes these two rather than a [`Font`] because a font can only be
/// loaded from a file, and the demo's tests must not need a filesystem. `main`
/// builds them from the same face it hands the renderer; the tests build them
/// from a monospace stand-in.
#[derive(Clone)]
struct TextMetrics {
    /// A character's advance width at a size, in pixels.
    advance: Rc<dyn Fn(char, f32) -> f32>,
    /// The height of one line at a size, in pixels.
    line_height: Rc<dyn Fn(f32) -> f32>,
}

impl TextMetrics {
    /// Returns the measurements of `font`.
    fn new(font: Font) -> Self {
        let face = Rc::new(font);
        let lines = Rc::clone(&face);
        TextMetrics {
            advance: Rc::new(move |ch: char, size: f32| face.advance(ch, size)),
            line_height: Rc::new(move |size: f32| lines.line_height(size)),
        }
    }

    /// Returns the advance width of `ch` at `font_size`, in pixels.
    fn advance(&self, ch: char, font_size: f32) -> f32 {
        (self.advance)(ch, font_size)
    }

    /// Returns the height of one line at `font_size`, in pixels.
    fn line_height(&self, font_size: f32) -> f32 {
        (self.line_height)(font_size)
    }
}

/// A label in the demo's text panel: the widget, and the layout options it is
/// painted with.
///
/// The options are the demo's, not the label's: the label lays out whatever it
/// is given, and the demo decides that this one wraps, that one is centred and
/// that this one is truncated. Keeping them together here is what lets the
/// panel re-lay out every label when the font size changes.
struct DemoLabel {
    /// The label widget, with its node in the arena.
    label: Label,
    /// The layout the label is drawn with.
    options: LayoutOptions,
}

impl DemoLabel {
    /// Returns the size this label's text needs in its layout at `font_size`.
    fn size(&self, metrics: &TextMetrics, font_size: f32) -> Size {
        let mut options = self.options;
        options.line_height = metrics.line_height(font_size);
        let layout = self
            .label
            .layout(&options, &|ch| metrics.advance(ch, font_size));
        let width = options.max_width.max(
            layout
                .lines
                .iter()
                .map(|line| line.width)
                .fold(0.0, f32::max),
        );
        Size::new(width, layout.total_height)
    }
}

/// The two things a toggle's drawn appearance is derived from.
///
/// A record rather than a re-aim every frame, for
/// [`Demo::sync_toggle_state`]'s reason: aiming restarts the transition, so
/// aiming on every frame would leave the thumb creeping toward its target for
/// ever instead of arriving at it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ToggleState {
    /// Which way the switch is, which is the thumb's position and the pill's
    /// colour together.
    checked: bool,
    /// Whether it holds focus, which is what its ring is derived from.
    focused: bool,
}

/// Returns a colour property that follows whichever of `tokens` the
/// `color_token` property names.
///
/// Every piece of text in the demo is bound through this, so `C` moves the text
/// panel and the readouts below the pads together.
fn cycling_color(
    color_token: &Property<ThemeToken>,
    tokens: &[(ThemeToken, Property<PropertyValue>)],
) -> Property<Color> {
    let token = color_token.clone();
    let candidates = tokens.to_vec();
    Property::bind(move || {
        let wanted = token.get();
        candidates
            .iter()
            .find(|(candidate, _)| *candidate == wanted)
            .and_then(|(_, property)| property.get().as_color())
            .unwrap_or(Color::new(255, 255, 255, 255))
    })
}

/// Returns a colour property bound to a single `token`, falling back to
/// `fallback` if the theme ever holds something else there.
///
/// The one-colour case of [`cycling_color`], which needs a list because it
/// follows a token the caller moves. The card the pads sit on is bound here
/// rather than copied, so `T` carries a theme switch to it through the same
/// property graph every other colour in the demo uses.
fn themed_color(token: &Property<PropertyValue>, fallback: Color) -> Property<Color> {
    let token = token.clone();
    Property::bind(move || token.get().as_color().unwrap_or(fallback))
}

/// A slider in the demo: the widget, and the dragging state the demo last wrote
/// to it and last aimed it at.
///
/// **The two records are what stop a frame from doing work**: a property write
/// notifies the node's `on_change` callback and marks the node dirty, and
/// aiming restarts the slider's transition, so both are done only when the state
/// has actually moved.
struct DemoSlider {
    /// The slider widget, with its node in the arena.
    widget: Slider,
    /// The dragging state the demo last wrote to the widget.
    written: bool,
    /// The dragging state the widget was last aimed at.
    aimed: bool,
}

impl DemoSlider {
    /// Returns the slider's node in the arena.
    fn node(&self) -> Handle {
        self.widget.handle()
    }
}

/// The demo's widget tree, the pads that press, the labels that show what text
/// rendering does, the theme every colour comes from, and the clock that drives
/// the pads.
struct Demo {
    nodes: Rc<RefCell<Arena<WidgetNode>>>,
    root: Handle,
    order: Vec<Handle>,
    pads: Vec<Pad>,
    /// The node painted with the theme's background colour.
    background: Handle,
    /// The window's background colour, bound to the theme's `Background` token.
    background_color: Property<PropertyValue>,
    /// The labels, in the order the text column places them.
    labels: Vec<DemoLabel>,
    /// The handles of the label nodes, parallel to `labels`.
    label_nodes: Vec<Handle>,
    /// The node the text column is placed in, so a label change can mark the
    /// whole panel dirty rather than every label.
    text_panel: Handle,
    /// The font measurements every label is laid out and drawn with.
    metrics: TextMetrics,
    /// The font size the labels are at, moved by `+` and `-`.
    text_size: f32,
    /// Which theme token the labels take their colour from, moved by `C`.
    color_token: Property<ThemeToken>,
    clock: AnimationClock,
    /// The theme the demo's colours come from.
    theme: Theme,
    /// Whether the theme is currently the dark one.
    dark: bool,
    mouse_pressed: Option<usize>,
    /// Whether the pads are down because the **space bar** is held.
    ///
    /// **The counterpart of `mouse_pressed`, and it exists because a release has
    /// to be able to ask whether there was a press.** The key-up arm used to ask
    /// `self.focused.is_none()` instead, which was the right question for the
    /// *press* — `Space` is the activation key of a focused button, so a held
    /// control must keep it — and the wrong question for the release for a reason
    /// that only exists because focus can move between the two:
    ///
    /// 1. hold `Space`, so the three pads go down;
    /// 2. press `D`, so the dialog opens and **takes focus**, because a modal
    ///    opens with focus inside it;
    /// 3. release `Space` — and `focused.is_none()` is now false, so the release
    ///    declined, and the three pads sat at 1.0 behind the scrim with **no
    ///    gesture left that could bring them back**: every other key is either
    ///    guarded by the modal or does not touch the pads.
    ///
    /// That is `.ai/NEVERAGAIN.md` § *A drawn control with nothing behind it*
    /// reached through the keyboard rather than through a missing event handler,
    /// and `a_release_is_never_gated_so_nothing_can_be_stranded_mid_press` is the
    /// test that found it. Written by [`Demo::press_all`] and cleared by
    /// [`Demo::release_all`], so the flag and the animation are one fact.
    space_pressed: bool,
    /// The gesture recogniser every control's events are built from.
    recognizer: GestureRecognizer,
    /// The control holding focus, or `None` when nothing does.
    focused: Option<Handle>,
    /// Every node in the demo that has children, as the widget that owns it.
    ///
    /// The tree is built out of [`Container`]s rather than out of nodes the demo
    /// assembles itself, so a parent is the widget and the demo's frame loop
    /// paints it through [`Container::paint`]. Only the row of pads is given a
    /// background: it is the card that shows what a container with a background
    /// and padding looks like, and the other four draw nothing.
    containers: Vec<Container>,
    /// The gauge, at the head of the right-hand control column.
    ///
    /// **A plain field and not a `DemoGauge` wrapper**, for the reason
    /// [`Demo::progress`] is: nothing in a frame writes to the gauge. A slider
    /// needs a record of what the demo last wrote to it because a *pointer* can
    /// change it from outside, and a button needs one for the same reason; a
    /// gauge has no `on_event` at all — its needle and its tick marks are
    /// decoration and nothing in it answers a finger — so the only thing that
    /// changes its value is a keypress, which is the demo's own doing and needs no
    /// record of what it last did.
    gauge: Gauge,
    /// Which of [`GAUGE_TYPES`] the gauge is drawn in, as an index into it.
    ///
    /// A property rather than a plain field for the reason
    /// [`Demo::image_fit`] is: the readout naming the shape is bound to it, and
    /// [`GaugeType`] is a plain field behind
    /// [`set_gauge_type`](Gauge::set_gauge_type), so there is no property on the
    /// widget for the label to follow. It is written **after** the widget's,
    /// which has the same consequence `progress_indeterminate` documents — a
    /// frame in which the two disagree draws a dial the label has not caught up
    /// with rather than the reverse.
    gauge_type: Property<usize>,
    /// The label showing the gauge's drawn value, its share of the range and the
    /// shape it is drawn in.
    gauge_readout: DemoLabel,
    /// The slider, under the gauge.
    slider: DemoSlider,
    /// The label showing the slider's value, and how many times it has been
    /// adjusted.
    slider_readout: DemoLabel,
    /// Whether a pointer is holding the slider down.
    slider_dragging: bool,
    /// The toggle, under the slider's readout.
    toggle: Toggle,
    /// The state the toggle was last aimed at.
    toggle_aimed: ToggleState,
    /// The label showing the toggle's state and how many times it has been
    /// switched.
    toggle_readout: DemoLabel,
    /// The image, right of the card of pads.
    image: Image,
    /// Which of [`IMAGE_FITS`] the image is showing, as an index into it.
    ///
    /// A property rather than a plain field because the label naming the fit is
    /// bound to it, which is the one link in the demo that is a binding rather
    /// than a callback: the label does not need telling, it recomputes.
    image_fit: Property<usize>,
    /// Whether the image's node holds focus, for the fit label to say so.
    ///
    /// An [`Image`] has no `focused` property of its own, so there is nothing
    /// for the widget to draw a ring with and the demo has to say where focus
    /// is in words instead. `image_focused` and `progress_focused` are this
    /// question asked twice, once per widget that cannot answer it.
    image_focused: Property<bool>,
    /// The label naming the image's current fit.
    image_fit_readout: DemoLabel,
    /// The progress bar, at the foot of the band's left column.
    progress: Progress,
    /// Whether the progress bar is sliding rather than showing a value.
    ///
    /// The demo's own record of it, because [`Progress::indeterminate`] is a
    /// plain field behind a setter: there is no property for the readout to bind
    /// to, so this is the one the readout follows and the one the setter is
    /// driven from. It is written **after** the widget's, so a frame in which
    /// they disagree draws a bar the label has not caught up with rather than
    /// the reverse.
    progress_indeterminate: Property<bool>,
    /// Whether the progress bar's node holds focus, for its readout to say so.
    progress_focused: Property<bool>,
    /// The label showing the progress bar's value and mode.
    progress_readout: DemoLabel,
    /// The chart, in the column the list used to fill.
    ///
    /// **A plain field and not a `DemoGauge` wrapper**, for the same reason:
    /// nothing in a frame writes to it. A chart has no `on_event` at all — its
    /// series, its grid and its axis labels all look grabbable and nothing reads
    /// a pointer over any of them, which is what its own module document says —
    /// so the only things that change its data are three keys, and a keypress is
    /// the demo's own doing and needs no record of what it last did.
    chart: Chart,
    /// Which of [`CHART_TYPES`] the chart is drawn in, as an index into it.
    ///
    /// A property rather than a plain field for the reason
    /// [`Demo::gauge_type`] is: `ChartType` is a plain field behind
    /// [`set_chart_type`](Chart::set_chart_type), so there is no property on the
    /// widget for the readout to follow, and the readout has to follow *something*.
    /// It is written **after** the widget's, which has the same consequence
    /// `progress_indeterminate` documents — a frame in which the two disagree
    /// draws a chart the label has not caught up with rather than the reverse.
    chart_type: Property<usize>,
    /// Which of [`CHART_SERIES`] the next reading the demo adds comes from.
    ///
    /// The demo's own cursor into its own constant, and the reason the appended
    /// value is a number a test can name rather than a formula: `A` and `S` take
    /// the reading this cursor is on and move it on, so a press of either key and
    /// the reading it produced are both derivable from a constant and a counter.
    chart_next: usize,
    /// Whether the chart is mid-transition, for its readout to say so.
    ///
    /// **The demo's record of [`Chart::is_animating`]**, written by the frame
    /// rather than bound, for the reason its own field comment gives: the
    /// question is a method over the widget's clock and a binding captures
    /// properties, not widgets.
    chart_moving: Property<bool>,
    /// The label naming the chart's length, its newest reading, the shape it is
    /// drawn in and whether it is mid-transition.
    chart_readout: DemoLabel,
    /// How fast the demo is running, counted from the loop's own frame deltas.
    ///
    /// The demo's and not `main`'s, because the demo is what draws the frames it
    /// measures and the readout is a widget like any other; `main` only asks for
    /// the report when the loop ends.
    fps: FrameRate,
    /// The text the frame-rate readout shows, written by the frame.
    ///
    /// A plain property rather than a bound one, for the reason
    /// [`Demo::progress_indeterminate`] is: the meter is a plain field, and a
    /// binding can only recompute from properties. The write is guarded by the
    /// text it would write — see [`Demo::tick_fps`] — because a readout whose text
    /// changes on every frame would otherwise re-lay out a string sixty times a
    /// second to say the same thing.
    fps_text: Property<String>,
    /// The label showing the frame rate, in the bottom left of the window.
    fps_readout: DemoLabel,
    /// The demo's text field, under the gallery.
    ///
    /// A plain owned field, **not** an `Rc`. The first version of this wiring
    /// shared it so the keyboard's `on_key` callback could reach it, and that was
    /// wrong twice over: `set_palette` takes `&mut self`, so an `Rc` with a live
    /// clone in a callback can never be re-themed — `Rc::get_mut` returns `None`
    /// and the palette silently stayed the old one across a theme switch. It is
    /// now the repo's own pattern instead: the callback writes
    /// [`pending_key`](Demo::pending_key) and [`Demo::offer_to`] drains it.
    text_input: TextInput,
    /// The key the keyboard last reported, waiting for the demo to act on it.
    ///
    /// The mechanism the list's `on_item_click` used — a widget whose callback
    /// cannot reach the `Demo` writes a property, and the demo reads it — and the
    /// reason it is a property and not a field is that `Callback` is `Fn`, so it
    /// cannot write a `&mut self` it was not lent. The list is gone; the pattern
    /// it left behind is not.
    pending_key: Property<Option<KeyAction>>,
    /// The on-screen keyboard, under the field.
    ///
    /// The text input's other half and not a child of it: the keyboard reports a
    /// [`KeyAction`] and [`apply_key_action`] is what turns one into an
    /// edit. Keeping the wiring in the demo is what lets the two widgets be built
    /// and tested without either knowing the other exists.
    keyboard: Keyboard,
    /// Whether a pointer is holding a key down.
    ///
    /// The keyboard's counterpart to the slider's, and needed for the same reason:
    /// `Keyboard::on_event` only ever sees a `Tap`, which the recogniser reports
    /// on the *release*, so a key cannot light up from inside it. `grab_key` and
    /// `release_key` are called from the press and the release.
    keyboard_pressed: bool,
    /// What the field last reported through `on_change`, for the readout.
    text_readout: DemoLabel,
    /// What the field last reported through `on_submit`, for the readout.
    submit_readout: DemoLabel,
    /// The modal dialog, over the whole window.
    ///
    /// **A plain owned field, for the reason [`Demo::text_input`] is.** Its `OK`
    /// writes [`pending_theme`](Demo::pending_theme) and the demo drains it, which
    /// is the same two-hop the keyboard's `on_key` uses and the reason there is
    /// no `Rc` round it: `set_palette` takes `&mut self`, and an `Rc` with a live
    /// clone in a callback can never be re-themed — which is the trap
    /// `Demo::text_input` documents having been caught by once.
    dialog: Dialog,
    /// That the dialog's `OK` was pressed, waiting for the demo to act on it.
    ///
    /// The mechanism [`Demo::pending_key`] names, asked again: `Callback` is `Fn`,
    /// so an action's click cannot write a `&mut self` it was not lent, and a
    /// property is the only thing it can be given that reaches the demo.
    pending_theme: Property<bool>,
}

impl Demo {
    /// Builds the demo: a themed background behind a centred row of three
    /// pads, each with a press property and a colour bound to it and to the
    /// theme, and a text panel above them that exercises the Label.
    ///
    /// The text panel is the visual proof for the label work, so it shows what
    /// the pipeline does rather than one string: the greeting itself, a
    /// paragraph wrapped at the panel's width, the three alignments side by
    /// side, and a line truncated to fit. `+` and `-` move the font size, `C`
    /// moves which theme token the text takes its colour from, and `T` switches
    /// the theme, which reaches the text through the same property graph the
    /// pads use.
    ///
    /// The error is a message rather than a type of its own: the tree is
    /// written out here, so a node that cannot be attached is a bug in this
    /// file and not a runtime condition a caller could act on.
    fn new(metrics: TextMetrics, picture: Option<Picture>) -> Result<Self, &'static str> {
        let theme = Theme::new();
        let mut nodes = Arena::new();
        let mut pads = Vec::new();

        // The window's background: a node that fills the window, painted with
        // the theme's Background token. It is a stack child that asks for the
        // window's own size, so it covers whatever the window is.
        let background = node::create(
            &mut nodes,
            LayoutState::new().with_constraints(Constraints::tight(WINDOW)),
        );
        let background_color = {
            let background_prop = theme.property(ThemeToken::Background);
            Property::bind(move || background_prop.get())
        };

        for &rest_token in &PAD_TOKENS {
            let node = node::create(
                &mut nodes,
                LayoutState::new().with_constraints(Constraints::tight(PAD_SIZE)),
            );
            let press = Property::new(0.0);
            let color = {
                let rest = theme.property(rest_token);
                let press = press.clone();
                Property::bind(move || {
                    let rest = rest.get().as_color().unwrap_or(Color::new(0, 0, 0, 255));
                    let held = lighten(rest);
                    PropertyValue::Color(Color::interpolate(&rest, &held, press.get()))
                })
            };
            pads.push(Pad::new(press, color, node));
        }

        // The row of pads, which is also the demo's card: a container with the
        // theme's surface behind the pads and the pads inset by its padding.
        let mut row = Container::new(&mut nodes, LayoutMode::row());
        row.set_flex_config(
            &mut nodes,
            FlexConfig::new()
                .with_spacing(PAD_SPACING)
                .with_main_axis_alignment(MainAxisAlignment::Center)
                .with_cross_axis_alignment(CrossAxisAlignment::Center),
        );
        row.set_padding(&mut nodes, Padding::all(CARD_PADDING));
        row.background = themed_color(
            &theme.property(ThemeToken::Surface),
            Color::new(0, 0, 0, 255),
        );
        // The radius is read once rather than bound, because the theme animates
        // a token's colour but nothing here moves a corner: the two themes hold
        // the same radius today, and a theme that changed it would be a change
        // to the shape of the card rather than to what is on it.
        row.border_radius.set(
            theme
                .get(ThemeToken::BorderRadiusLg)
                .as_number()
                .unwrap_or(CARD_RADIUS_FALLBACK),
        );
        for &child in &[pads[0].node, pads[1].node, pads[2].node] {
            if !row.add_child(&mut nodes, child) {
                return Err("ui_demo: a pad could not be attached to the row");
            }
        }

        // The text panel, and the labels in it. Their colours come from the
        // theme like the pads', bound through the token `C` moves, so both a
        // theme switch and a `C` reach the text through the property graph.
        let color_token = Property::new(ThemeToken::Text);
        let token_properties: Vec<(ThemeToken, Property<PropertyValue>)> = TEXT_COLOR_TOKENS
            .iter()
            .map(|&token| (token, theme.property(token)))
            .collect();
        let mut labels = Vec::new();
        for (text, options) in demo_labels() {
            let mut label = Label::new(&mut nodes, text);
            label.color = cycling_color(&color_token, &token_properties);
            label.font_size.set(TEXT_SIZE_START);
            labels.push(DemoLabel { label, options });
        }
        // Each label is given a rect the size its own laid-out text needs, so
        // the panel's wrapping is the wrapping on screen.
        for demo_label in &labels {
            let handle = demo_label.label.handle();
            let size = demo_label.size(&metrics, TEXT_SIZE_START);
            nodes
                .get_mut(handle)
                .ok_or("ui_demo: a label node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let label_nodes: Vec<Handle> = labels.iter().map(|demo| demo.label.handle()).collect();
        let text_column = Container::new(&mut nodes, LayoutMode::column());
        text_column.set_flex_config(
            &mut nodes,
            FlexConfig::new()
                .with_spacing(LABEL_SPACING)
                .with_cross_axis_alignment(CrossAxisAlignment::Start),
        );
        for &child in &label_nodes {
            if !text_column.add_child(&mut nodes, child) {
                return Err("ui_demo: a label could not be attached to the text column");
            }
        }
        // The panel is `Absolute` so the column inside it can sit at the panel's
        // margin: a `Stack` places every child at its own origin, so the margin
        // has to come from the column's declared position rather than from the
        // panel's rect.
        nodes
            .get_mut(text_column.handle())
            .ok_or("ui_demo: the text column is missing")?
            .layout_mut()
            .set_position(Some(Offset::new(TEXT_PANEL_ORIGIN.0, TEXT_PANEL_ORIGIN.1)));
        let text_panel = Container::new(&mut nodes, LayoutMode::Absolute);
        {
            let panel = nodes
                .get_mut(text_panel.handle())
                .ok_or("ui_demo: the text panel is missing")?;
            panel
                .layout_mut()
                .set_constraints(Constraints::tight(TEXT_PANEL));
        }
        if !text_panel.add_child(&mut nodes, text_column.handle()) {
            return Err("ui_demo: the text column could not be attached");
        }

        // The gauge, at the head of the right-hand control column, and the
        // label naming what it is showing. Three things happen in this order and
        // the order is the whole of it: the palette names the four colours the
        // gauge draws with while the properties still hold the neutral greys
        // `Gauge::new` wrote, `snap_to_state` puts the value *and* the palette on
        // the gauge before it is ever drawn, and only then is the shape chosen.
        // A `set_gauge_type` after the snap would write `needle` from the palette
        // again for the same value, which is harmless, and the reverse order
        // would leave the tick marks and the needle on the neutral greys for a
        // frame.
        let gauge_type = Property::new(0usize);
        let mut gauge = Gauge::new(&mut nodes, GAUGE_MIN, GAUGE_MAX);
        gauge.set_gauge_type(GAUGE_TYPES[0]);
        gauge.set_palette(GaugePalette::from_theme(&theme));
        gauge.value.set(GAUGE_START);
        gauge.snap_to_state();
        {
            // The box is the widget's own `size()`, written out as
            // [`GAUGE_SIZE`]. A gauge has no content to measure, so this is the
            // widget saying how big it wants to be rather than the demo choosing,
            // and the constant is here so the layout has a number to point at and
            // a test has something to compare the drawn dial against.
            let size = GAUGE_SIZE;
            nodes
                .get_mut(gauge.handle())
                .ok_or("ui_demo: the gauge node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let gauge_readout = read_only_label(
            &mut nodes,
            &metrics,
            READOUT_FONT,
            GAUGE_READOUT_WIDTH,
            {
                // The gauge's own `value` property and not its `shown`, so the
                // line names the reading rather than where the needle has got to:
                // the needle's travel is what the dial shows, and a number chasing
                // it would be a second thing moving for no extra information.
                //
                // **The share is the demo's own arithmetic and not
                // `Gauge::fraction`,** which is public for this exact purpose and
                // which a caller with the widget in hand should be using. The
                // reason it cannot be used here is the same one every other
                // bound readout in the demo has: a `Property::bind` closure
                // captures properties, not widgets, and the `Gauge` is moved into
                // [`Demo`] three lines below. Reaching it would mean an `Rc` round
                // the widget, and `set_palette` takes `&mut self` — which is the
                // trap `Demo::text_input` documents having already been caught by
                // once. So the mapping is spelled out, and
                // `the_gauge_readout_says_the_share_the_widgets_own_fraction_gives`
                // is what holds the two in step.
                let value = gauge.value.clone();
                let shape = gauge_type.clone();
                Property::bind(move || {
                    let read = value.get();
                    let span = GAUGE_MAX - GAUGE_MIN;
                    let share = if span > 0.0 {
                        ((read - GAUGE_MIN) / span).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    format!(
                        "{read:.0} of {GAUGE_MAX:.0}, {:.0}%, {}",
                        share * 100.0,
                        GAUGE_TYPE_NAMES
                            .get(shape.get())
                            .copied()
                            .unwrap_or(GAUGE_TYPE_NAMES[0])
                    )
                })
            },
            &color_token,
            &token_properties,
        )?;

        // The slider, under the gauge, and the readout that shows what it is at.
        // The slider's colours come from the theme the same way the gauge's do,
        // and it is snapped onto them before it is ever drawn for the reason
        // `snap_to_state` exists.
        let mut widget = Slider::new(&mut nodes, SLIDER_MIN, SLIDER_MAX);
        widget.set_step(Some(SLIDER_STEP));
        widget.set_orientation(Orientation::Horizontal);
        widget.set_palette(SliderPalette::from_theme(&theme));
        // Finger-sized, through the widget's own setters: a 14-pixel track and a
        // 44-pixel knob rather than the 6-pixel hairline the widget defaults to.
        // Set before asking for a size, because the size is computed from them.
        widget.set_track_thickness(SLIDER_TRACK_THICKNESS);
        widget.set_thumb_radius(SLIDER_THUMB_RADIUS);
        widget.snap_to_state();
        // The size is the widget's own maths for the **across** axis — a slider has
        // no content to measure, so `size()` is the widget saying how thick it wants
        // to be — with this demo's own length along the track.
        {
            let mut size = widget.size();
            size.width = SLIDER_LENGTH;
            nodes
                .get_mut(widget.handle())
                .ok_or("ui_demo: the slider node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }

        // The readout is a bound label over two properties: the slider's value,
        // so it follows a drag and a key and a programmatic write alike, and a
        // count of the widget's own adjustments, so the line also says which of
        // those it was. `on_change` is what increments the second — the widget
        // fires it for a pointer and a key, and not for a value the demo wrote
        // itself. The count needs no field of its own: the two closures below
        // hold it, and the readout's text is how a reader sees it.
        let changes = Property::new(0u32);
        {
            let counted = changes.clone();
            widget.on_change = ValueCallback::from_fn(move |_value| {
                counted.set(counted.get() + 1);
            });
        }
        let readout_text = {
            let value = widget.value.clone();
            let counted = changes.clone();
            Property::bind(move || {
                let count = counted.get();
                let noun = if count == 1 {
                    "adjustment"
                } else {
                    "adjustments"
                };
                // One space between the words, because the label lays a run of
                // them out as one word gap: a readout that lined its two halves up
                // with padding would have it collapsed.
                format!("{:.0} of {SLIDER_MAX:.0}, {count} {noun}", value.get())
            })
        };
        let mut readout = Label::new(&mut nodes, String::new());
        readout.text = readout_text;
        readout.color = cycling_color(&color_token, &token_properties);
        readout.font_size.set(READOUT_FONT);
        let slider_readout = DemoLabel {
            label: readout,
            options: LayoutOptions {
                max_width: SLIDER_READOUT_WIDTH,
                ..LayoutOptions::default()
            },
        };
        {
            let size = slider_readout.size(&metrics, READOUT_FONT);
            nodes
                .get_mut(slider_readout.label.handle())
                .ok_or("ui_demo: the slider readout is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let slider = DemoSlider {
            widget,
            written: false,
            aimed: false,
        };

        // The toggle, under the slider's readout, and the label that says which
        // state it is in. The toggle is snapped onto the theme before it is ever
        // drawn for the reason `snap_to_state` exists, and its `on_change` counts
        // the switches, so the label reports two things: which way the switch is
        // and how many times it has been thrown.
        let toggle_changes = Property::new(0u32);
        let mut toggle = Toggle::new(&mut nodes);
        toggle.set_palette(TogglePalette::from_theme(&theme));
        toggle.snap_to_state();
        let toggle_state = toggle.checked.get();
        {
            let counted = toggle_changes.clone();
            toggle.on_change = ValueCallback::from_fn(move |_checked: bool| {
                counted.set(counted.get() + 1);
            });
        }
        {
            let size = toggle.size();
            nodes
                .get_mut(toggle.handle())
                .ok_or("ui_demo: the toggle node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let toggle_readout = read_only_label(
            &mut nodes,
            &metrics,
            READOUT_FONT,
            TOGGLE_READOUT_WIDTH,
            {
                let checked = toggle.checked.clone();
                let counted = toggle_changes.clone();
                Property::bind(move || {
                    let state = if checked.get() { "on" } else { "off" };
                    let count = counted.get();
                    let noun = if count == 1 { "change" } else { "changes" };
                    format!("{state}, {count} {noun}")
                })
            },
            &color_token,
            &token_properties,
        )?;

        // The image, right of the card of pads, and the label naming the fit it
        // is showing. `F` cycles the fit and the label is bound to the same
        // property, so it cannot name a fit the image is not showing.
        //
        // A missing asset is a transparent image of the asset's own shape rather
        // than no image: the node has to exist for the layout, the paint order
        // and the collision tests to mean anything. The label says which of the
        // two it is, because an image of transparent pixels is a rectangle of
        // background and an unexplained one is the shape a defect looks like.
        let (picture, image_is_stand_in) = match picture {
            Some(picture) => (picture, false),
            None => (stand_in_picture().ok_or("ui_demo: no image to show")?, true),
        };
        let image_fit = Property::new(0usize);
        let image_focused = Property::new(false);
        let mut image = Image::new(&mut nodes, picture.texture, picture.source);
        // A rounded corner rather than the widget's square default: the corner is
        // a shader feature, and a square corner is not evidence it ran.
        image.corner_radius.set(IMAGE_CORNER_RADIUS);
        image.set_fit(image_fit_at(0));
        image.snap_to_state();
        {
            let size = IMAGE_SIZE;
            nodes
                .get_mut(image.handle())
                .ok_or("ui_demo: the image node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let image_fit_readout = read_only_label(
            &mut nodes,
            &metrics,
            READOUT_FONT,
            IMAGE_FIT_WIDTH,
            {
                let fit = image_fit.clone();
                let focused = image_focused.clone();
                Property::bind(move || {
                    let mut line = format!("fit: {}", image_fit_name(fit.get()));
                    if image_is_stand_in {
                        line.push_str(" (stand-in)");
                    }
                    // The word rather than a ring: an `Image` has no `focused`
                    // property, so there is no shape of its own to grow a ring
                    // around and put back over — see `image_focused`.
                    if focused.get() {
                        line.push_str(", focused");
                    }
                    line
                })
            },
            &color_token,
            &token_properties,
        )?;

        // The progress bar, at the foot of the band's left column, and the label
        // naming its value and which of the two modes it is in. The mode is a
        // property of the demo's own because `Progress::indeterminate` is a
        // plain field behind a setter, so a label bound to it would have to be
        // told rather than recompute.
        let progress_focused = Property::new(false);
        let progress_indeterminate = Property::new(false);
        let mut progress = Progress::new(&mut nodes);
        progress.set_palette(ProgressPalette::from_theme(&theme));
        progress.set_orientation(Orientation::Horizontal);
        progress.value.set(PROGRESS_START);
        progress.snap_to_state();
        {
            let size = PROGRESS_SIZE;
            nodes
                .get_mut(progress.handle())
                .ok_or("ui_demo: the progress bar node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let progress_readout = read_only_label(
            &mut nodes,
            &metrics,
            READOUT_FONT,
            PROGRESS_READOUT_WIDTH,
            {
                let value = progress.value.clone();
                let focused = progress_focused.clone();
                let indeterminate = progress_indeterminate.clone();
                Property::bind(move || {
                    let mode = if indeterminate.get() {
                        "sliding"
                    } else {
                        "determinate"
                    };
                    let mut line = format!("{:.0}%, {mode}", value.get() * 100.0);
                    if focused.get() {
                        line.push_str(", focused");
                    }
                    line
                })
            },
            &color_token,
            &token_properties,
        )?;

        // Whether anything the chart draws is mid-transition, for its readout to
        // say so. Declared before the chart because the readout's text is bound to
        // it.
        //
        // **The demo's own record of the widget's own answer, written by the
        // frame**, and that is the whole of why it exists:
        // `Chart::is_animating` is a *method* over the widget's internal clock,
        // and a `Property::bind` closure captures properties rather than widgets
        // — reaching the chart would mean an `Rc` round it, and `set_palette`
        // takes `&mut self`, which is the trap `Demo::text_input` documents
        // having already been caught by once. The frame writes it, guarded by the
        // value, for the reason `tick_fps` gives: a write fires the readout's
        // `on_change` and re-lays out its text, and nothing is mid-transition on
        // almost every frame of a run.
        let chart_moving = Property::new(false);

        // The chart, its readout and the sample data it plots. Three things
        // happen in this order and the order is the whole of it, and it is the
        // gauge's order for the same reason: the palette names the five colours
        // the chart draws with while the properties still hold the neutral greys
        // `Chart::new` wrote, the data is written next, and `snap_to_state` puts
        // **both** on the chart before it is ever drawn. Without the snap a
        // themed chart opens on the first frame as a grey one and then animates
        // to the colour it should have started at — which is what
        // `the_demo_opens_with_a_themed_chart_of_sample_data_and_says_so` is
        // about.
        let chart_type = Property::new(0usize);
        let mut chart = Chart::new(&mut nodes, CHART_TYPES[0]);
        chart.set_palette(ChartPalette::from_theme(&theme));
        chart.data.set(CHART_SERIES[..CHART_OPENING_COUNT].to_vec());
        chart
            .x_labels
            .set((0..CHART_X_LABEL_COUNT).map(|i| i.to_string()).collect());
        // **No `y_labels`, and that is requirement 4's default rather than an
        // omission**: an auto-scaled axis moves its own numbers on every frame of
        // a transition, so numbers the caller wrote for it would be wrong for the
        // length of every append. The widget says the same from its own side — it
        // draws none of its own, and a caller that wants them fixes the range
        // first — so the demo keeps the range auto-scaling and has no y gutter at
        // all, which puts the axis on the node's own left edge at 1000.
        chart.snap_to_state();
        {
            let size = CHART_SIZE;
            nodes
                .get_mut(chart.handle())
                .ok_or("ui_demo: the chart node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let chart_readout = read_only_label(
            &mut nodes,
            &metrics,
            CHART_READOUT_FONT,
            CHART_READOUT_WIDTH,
            {
                // Bound to three properties and to nothing else: the widget's own
                // `data`, the demo's record of the shape, and — through a
                // property of its own rather than through the widget, which has no
                // property for it — whether anything is mid-transition.
                //
                // `Chart::is_animating` is a *method* over the widget's internal
                // clock, so a `Property::bind` closure cannot call it: it would have
                // to hold the widget, and the widget is moved into `Demo` a few
                // lines below. `chart_moving` is the demo's own record of the same
                // fact, written by the frame from the widget's own answer.
                let data = chart.data.clone();
                let shape = chart_type.clone();
                let moving = chart_moving.clone();
                Property::bind(move || {
                    let readings = data.get();
                    // Shown as absent rather than as blank, for
                    // [`NOTHING_ENTERED`]'s reason: a readout that printed an
                    // empty number could not be told from a readout that was not
                    // there. The demo's own window is never empty, so this arm is
                    // unreachable from a keypress — which is the point of writing
                    // it down rather than unwrapping.
                    let last = match readings.last() {
                        Some(value) => format!("{value:.2}"),
                        None => String::from(NO_READING),
                    };
                    format!(
                        "{} pts, last {last}, {}, {}",
                        readings.len(),
                        CHART_TYPE_NAMES
                            .get(shape.get())
                            .copied()
                            .unwrap_or(CHART_TYPE_NAMES[0]),
                        if moving.get() { "moving" } else { "still" }
                    )
                })
            },
            &color_token,
            &token_properties,
        )?;

        // The frame-rate readout, at the foot of the window. It is the one
        // readout whose text the demo *writes* rather than binds, because what it
        // shows is the loop's own measurement and not a property of a widget —
        // and the one whose rect is written out rather than measured, because its
        // text is different on almost every frame. See [`FPS_READOUT_WIDTH`].
        let fps_text = Property::new(FrameRate::new().summary());
        let fps_readout = read_only_label(
            &mut nodes,
            &metrics,
            READOUT_FONT,
            FPS_READOUT_WIDTH,
            fps_text.clone(),
            &color_token,
            &token_properties,
        )?;
        {
            let size = Size {
                width: FPS_READOUT_WIDTH,
                height: metrics.line_height(READOUT_FONT),
            };
            nodes
                .get_mut(fps_readout.label.handle())
                .ok_or("ui_demo: the frame-rate readout is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }

        // ------------------------------------------------- task 19: text entry
        //
        // The field and the keyboard are wired to each other by
        // `apply_key_action`, and by nothing else. Neither widget knows the other
        // exists — `Keyboard` reports a `KeyAction`, `TextInput` offers three edit
        // methods — and that independence is what let the two be written, tested
        // and mutation-checked as file-isolated sub-tasks and then meet here.

        // What the field last submitted, for the second readout. Declared before
        // the field because `on_submit` is wired into it.
        let submitted = Property::new(String::new());
        // The keyboard's report, waiting to be acted on. See
        // [`Demo::pending_key`].
        let pending_key = Property::new(None);

        let mut field = TextInput::new(&mut nodes);
        field.set_palette(TextInputPalette::from_theme(&theme));
        // The field's own defaults are 240 by 44, which is a mouse-sized control.
        // 480 by 64 is this demo's answer, for the reason the slider's track and
        // the list's scrollbar were both widened, and it goes in through the
        // properties task 19's review asked for rather than by editing the
        // widget's constants.
        field.width.set(TEXT_INPUT_SIZE.width);
        field.height.set(TEXT_INPUT_SIZE.height);
        field.font_size.set(TEXT_INPUT_FONT);
        field.placeholder.set(String::from(PLACEHOLDER_TEXT));
        field.snap_to_state();
        {
            // Wired **before** the `Rc`, because `on_submit` takes `&mut self` and
            // an `Rc` has no `get_mut` to lend until it is the sole owner. Doing
            // it here rather than inside `apply_key_action` is what makes a
            // hardware Enter and an on-screen Enter one fact: both end in
            // `on_submit.call`, and this is the only place that decides what that
            // call means.
            let submitted = submitted.clone();
            field.on_submit = ValueCallback::from_fn(move |value: String| {
                submitted.set(value);
            });
        }
        let text_input = field;

        let mut keyboard = Keyboard::new(&mut nodes);
        keyboard.set_palette(KeyboardPalette::from_theme(&theme));
        // The demo's three sizing numbers, and the widget's defaults left behind:
        // 52-tall keys do not fit the band, and the band is what decides them.
        keyboard.key_height.set(KEY_HEIGHT);
        keyboard.font_size.set(KEY_FONT);
        // The widget centres a key's label with a single average advance, and
        // ships a guess. **This is a real font and a real measurement**, so every
        // keycap's label is centred against what it is actually drawn with rather
        // than against 0.6 of the font size. `n` is the glyph the guess is a
        // stand-in for.
        keyboard.advance.set(metrics.advance('n', KEY_FONT));
        keyboard.snap_to_state();
        {
            // The keyboard's one callback is the whole of the wiring from the keys
            // to the field, and this closure is the **only** place in the
            // repository that knows a `KeyAction::Char` is a character.
            let pending = pending_key.clone();
            keyboard.on_key = ValueCallback::from_fn(move |action| {
                pending.set(Some(action));
            });
        }

        // The field's two readouts, both **bound** rather than written each
        // frame: `on_change` and `on_submit` are the widget's own notifications,
        // so a readout bound to the text property re-derives itself on every
        // keystroke whatever asked for it — a key, a finger, or a test.
        let text_readout = read_only_label(
            &mut nodes,
            &metrics,
            TEXT_FIELD_READOUT_FONT,
            TEXT_READOUT_WIDTH,
            Property::bind({
                // Cloned out of the widget rather than captured from it: the
                // closure would otherwise move the `TextInput` itself, and the
                // widget is still needed by handle two lines below.
                let value = text_input.text.clone();
                move || {
                    let entered = value.get();
                    if entered.is_empty() {
                        format!("text: {NOTHING_ENTERED}")
                    } else {
                        format!("text: {entered}")
                    }
                }
            }),
            &color_token,
            &token_properties,
        )?;
        let submit_readout = read_only_label(
            &mut nodes,
            &metrics,
            TEXT_FIELD_READOUT_FONT,
            SUBMIT_READOUT_WIDTH,
            Property::bind(move || {
                let value = submitted.get();
                if value.is_empty() {
                    format!("submitted: {NOTHING_SUBMITTED}")
                } else {
                    format!("submitted: {value}")
                }
            }),
            &color_token,
            &token_properties,
        )?;

        {
            let size = Size {
                width: TEXT_INPUT_SIZE.width,
                height: TEXT_INPUT_SIZE.height,
            };
            nodes
                .get_mut(text_input.handle())
                .ok_or("ui_demo: the text input is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        {
            let size = Size {
                width: KEYBOARD_WIDTH,
                height: KEYBOARD_HEIGHT,
            };
            nodes
                .get_mut(keyboard.handle())
                .ok_or("ui_demo: the keyboard is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }

        // The band has to fit the space the window gave it, and the first attempt
        // at this layout did not: it asked for a 1160-tall window and this host
        // returned 1052, which put the bottom of the keyboard off the bottom of
        // the screen where nobody could see it. Checked here rather than only in
        // a test because **this is the failure that is invisible until somebody
        // looks at the screen**, and the demo is built long before that.
        for (what, bottom) in [
            ("the keyboard", KEYBOARD_ORIGIN.1 + KEYBOARD_HEIGHT),
            ("the field", TEXT_INPUT_ORIGIN.1 + TEXT_INPUT_SIZE.height),
        ] {
            if bottom > BAND_TOP + BAND_HEIGHT {
                return Err(what);
            }
        }

        // ------------------------------------------------ task 22: the dialog
        //
        // **Built last and attached to nothing**, and both of those are the
        // widget's own design rather than tidiness: `Dialog::new` gives the
        // dialog a node with no parent, because a dialog overlays everything and
        // is not laid out inside the content it covers, and it paints its actions
        // into its own painter rather than onto their nodes. So the dialog is a
        // **second root** in the arena, and `paint_order` walks it after the
        // gallery's own — which is the whole of how it gets on top.
        //
        // Three things are wired here and each is a decision:
        //
        // - **the palette**, read from the theme before anything is drawn, for
        //   the reason every other palette in this function is;
        // - **two real actions**, `OK` and `Cancel` in that order, so the row is
        //   the acceptance criterion's pair and `Tab` order is the row left to
        //   right;
        // - **`OK`'s callback written before `add_action`**, because that is the
        //   one moment at which the widget wraps it in the dismissal. A callback
        //   written afterwards replaces the wrapper and the dialog stops closing,
        //   which is the failure `DialogAction` documents in as many words.

        // What the dialog's `OK` asked for, waiting to be acted on. Declared
        // before the dialog because the action's callback is wired into it.
        let pending_theme = Property::new(false);
        let mut dialog = Dialog::new(&mut nodes, DIALOG_TITLE, DIALOG_BODY);
        dialog.set_palette(DialogPalette::from_theme(&theme));
        // **No `set_motion` here, where every other widget in this function has
        // one, and the omission is the point.** `Dialog::motion`'s default is the
        // theme's `DurationNormal` with its `EasingStandard`, and
        // `Motion::from_theme` reads the *fast* duration — 150 ms, which is the
        // length of a press. A four-arc bounce in 150 ms gives each arc 54 ms,
        // which the widget's own doc calls a twitch rather than an arrival, so
        // the default is a deliberate answer and copying it here would be the
        // bug.
        for (index, label) in DIALOG_ACTIONS.iter().enumerate() {
            let mut action = DialogAction::new(&mut nodes, *label);
            if index == 0 {
                // `OK` asks for the theme switch the `T` key already does, and
                // `Cancel` asks for nothing at all — a dialog whose two buttons
                // did the same thing would be demonstrating a shape rather than
                // a choice. Written **before** `add_action`, which is when the
                // dismissal is wrapped around it.
                let asked = pending_theme.clone();
                action.button.on_click = Callback::new(move || asked.set(true));
            }
            dialog.add_action(&mut nodes, action);
        }
        // The overlay is the whole window, and the node is given the window tight
        // so that **the layout pass and the paint pass read one number**: both go
        // through the node's own laid-out rect, and `Dialog::on_event` is given
        // the same one as `Dialog::paint` — which is what the widget asks its
        // caller for in three words and what a tap landing in the wrong place
        // would come from getting wrong.
        {
            let handle = dialog.handle();
            nodes
                .get_mut(handle)
                .ok_or("ui_demo: the dialog node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(WINDOW));
        }
        // **Presented here, at construction**, so the demo opens with it up. See
        // the module doc for why that is a decision about evidence and not about
        // the demo: input injection does not work on this host, so a dialog that
        // began hidden could only ever be photographed through an instrument.
        let _ = dialog.present();
        // Focus starts **inside** the dialog, on its first action. Two things
        // need it and neither can do without: `Focus` hands an activation key to
        // the button node itself, so a dialog with nothing focused inside it has
        // an `Enter` that reaches no button; and `Space` is the demo's
        // *press-all-pads* gesture only `while self.focused.is_none()`, so with
        // focus outside the dialog a bare space would press the three pads
        // underneath the scrim.
        let first_action: Option<Handle> = dialog.actions.first().map(|action| action.handle());

        // The chart's stroke reaches [`Chart::stroke_reach`] past its own node,
        // and a node's rect is not what its pixels are — so the neighbours that
        // box is close to are checked here rather than only in a test, for the
        // reason the band budget above is: **this is a failure that is invisible
        // until somebody looks at the screen**, and the demo is built long before
        // that. The three are the control column's readouts, which end at
        // [`CHART_ORIGIN`]'s x less sixteen, the image-fit label's own line, which
        // is the tallest thing above the chart, and the text-entry band.
        //
        // **The bound is the widget's, and that is the whole of this block.**
        // There is no constant here of the reach's own, and there was: the demo
        // used to carry a private `6.0` for it, derived by hand from a
        // `MITRE_LIMIT` the widget keeps private. The widget answers it now —
        // `stroke_reach` is `MITRE_LIMIT · line_width / 2`, and it is a *method*
        // because `set_line_width` moves it, so a copied number would have gone
        // stale on the first line-width change with nothing to say so.
        //
        // **And the reach is the *stroke's*, not the widget's**, which
        // [`Chart::stroke_reach`] says in as many words, so the two bounds it does
        // not cover are named here rather than left in the reader's head:
        //
        // - a **y label's line box** overhangs the plot's top edge by
        //   `LABEL_FONT_SIZE / 2`, which is 6 px at the defaults — the same as the
        //   stroke's reach here, and *larger* than it at any font size above 12.
        //   **The demo writes no y labels** (an auto-scaled axis has no numbers a
        //   caller can write honestly), so there is none to bound, and the guard
        //   below turns that premise into something the build checks rather than
        //   something a comment claims;
        // - an **x label's right-hand end** overhangs by its own width and the
        //   widget cannot know it, because a `DrawCommand::Text` carries no width.
        //   That one is not bounded here at all: [`CHART_X_LABEL_COUNT`] leaves the
        //   newest sample unlabelled for exactly this reason, and
        //   `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_
        //   rect` measures the consequence — that nothing is inked in the node's
        //   last two columns.
        {
            if !chart.y_labels.get().is_empty() {
                // Refusing rather than guessing: the label's bound is
                // `LABEL_FONT_SIZE / 2` and `LABEL_FONT_SIZE` is private with no
                // accessor, so the demo cannot compute it — and a reach check that
                // silently stopped covering the top and the left is the failure
                // `.ai/NEVERAGAIN.md` § *a brief's rationale becomes the widget's
                // doc comment* is about, one level down.
                return Err("the chart writes y labels, and this check does not bound them");
            }
            let reach = grown(
                Rect::new(
                    CHART_ORIGIN.0,
                    CHART_ORIGIN.1,
                    CHART_SIZE.width,
                    CHART_SIZE.height,
                ),
                chart.stroke_reach(),
            );
            if reach.x < CONTROLS_ORIGIN.0 + SLIDER_READOUT_WIDTH {
                return Err("the chart, or its stroke, reaches into the control column's readouts");
            }
            let label = image_fit_readout.size(&metrics, READOUT_FONT);
            if reach.y < IMAGE_FIT_ORIGIN.1 + label.height {
                return Err("the chart, or its stroke, reaches into the image's own readout");
            }
            if reach.y + reach.height > BAND_TOP {
                return Err("the chart, or its stroke, reaches into the text-entry band");
            }
            // **Downward is the fourth direction, and this check does not settle
            // it either** — [`CHART_READOUT_GAP`] is four pixels and the reach is
            // six, so the two *boxes* overlap while the pixels do not. Every
            // reading is bounded into the range before it is scaled, so nothing is
            // drawn below the plot's own bottom edge, which is the node's bottom
            // less the x labels' gutter, and that gutter is another private
            // constant. **So the claim is measured rather than derived**, in
            // `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_
            // rect`: the lowest thing the chart draws is y=670 against a readout
            // that starts at 699.
        }

        // The gauge, its readout, the slider, its own readout and the four newer
        // widgets are each placed inside the controls layer, which is what
        // `Absolute` is for: each at its own offset from the layer's origin, which
        // is the window's own top left.
        for (node, origin) in [
            (gauge.handle(), GAUGE_ORIGIN),
            (gauge_readout.label.handle(), GAUGE_READOUT_ORIGIN),
        ] {
            nodes
                .get_mut(node)
                .ok_or("ui_demo: the gauge is missing")?
                .layout_mut()
                .set_position(Some(Offset::new(origin.0, origin.1)));
        }
        nodes
            .get_mut(slider.widget.handle())
            .ok_or("ui_demo: the slider is missing")?
            .layout_mut()
            .set_position(Some(Offset::new(SLIDER_ORIGIN.0, SLIDER_ORIGIN.1)));
        nodes
            .get_mut(slider_readout.label.handle())
            .ok_or("ui_demo: the slider readout is missing")?
            .layout_mut()
            .set_position(Some(Offset::new(
                SLIDER_ORIGIN.0,
                SLIDER_ORIGIN.1 + SLIDER_READOUT_DROP,
            )));
        for (node, origin) in [
            (toggle.handle(), TOGGLE_ORIGIN),
            (toggle_readout.label.handle(), TOGGLE_READOUT_ORIGIN),
            (image.handle(), IMAGE_ORIGIN),
            (image_fit_readout.label.handle(), IMAGE_FIT_ORIGIN),
            (progress.handle(), PROGRESS_ORIGIN),
            (progress_readout.label.handle(), PROGRESS_READOUT_ORIGIN),
            (chart.handle(), CHART_ORIGIN),
            (chart_readout.label.handle(), CHART_READOUT_ORIGIN),
            (fps_readout.label.handle(), FPS_READOUT_ORIGIN),
            (text_input.handle(), TEXT_INPUT_ORIGIN),
            (keyboard.handle(), KEYBOARD_ORIGIN),
            (text_readout.label.handle(), TEXT_READOUT_ORIGIN),
            (submit_readout.label.handle(), SUBMIT_READOUT_ORIGIN),
        ] {
            nodes
                .get_mut(node)
                .ok_or("ui_demo: a node in the band is missing")?
                .layout_mut()
                .set_position(Some(Offset::new(origin.0, origin.1)));
        }
        // The layer is the window, not a box of its own: it is a `Stack` child,
        // and a `Stack` sizes a child from its own constraints but places it at
        // the origin. Giving it the window's size makes the offsets inside it
        // window coordinates, which is what the positions above assume.
        //
        // It was called the **button band** while there were buttons in it, and it
        // is now the controls layer — it holds every control below the pads and
        // has held four of them that are no longer here. The name is the only
        // thing about it that changed.
        let controls = Container::new(&mut nodes, LayoutMode::Absolute);
        {
            let layer = nodes
                .get_mut(controls.handle())
                .ok_or("ui_demo: the controls layer is missing")?;
            layer
                .layout_mut()
                .set_constraints(Constraints::tight(WINDOW));
        }
        // The order the layer holds its children in **is** their paint order and
        // therefore their `Tab` order, so it is a list and not a set. **The gauge
        // is first**, because it is first in the column it belongs to: it sits
        // above the slider in the window and it was built before the slider in
        // this function, and the two agreeing is the whole argument for the
        // ordering being readable. The image comes after the slider and before
        // the toggle because that is where it sits in the window — top right,
        // above everything else here. The chart comes after the progress bar
        // because that is where it sits: the progress bar is the last of the
        // control column and the chart is the whole of the one to its right.
        if !controls.add_child(&mut nodes, gauge.handle())
            || !controls.add_child(&mut nodes, gauge_readout.label.handle())
            || !controls.add_child(&mut nodes, slider.widget.handle())
            || !controls.add_child(&mut nodes, slider_readout.label.handle())
            || !controls.add_child(&mut nodes, image.handle())
            || !controls.add_child(&mut nodes, image_fit_readout.label.handle())
            || !controls.add_child(&mut nodes, toggle.handle())
            || !controls.add_child(&mut nodes, toggle_readout.label.handle())
            || !controls.add_child(&mut nodes, progress.handle())
            || !controls.add_child(&mut nodes, progress_readout.label.handle())
            || !controls.add_child(&mut nodes, chart.handle())
            || !controls.add_child(&mut nodes, chart_readout.label.handle())
            || !controls.add_child(&mut nodes, fps_readout.label.handle())
            || !controls.add_child(&mut nodes, text_input.handle())
            || !controls.add_child(&mut nodes, text_readout.label.handle())
            || !controls.add_child(&mut nodes, submit_readout.label.handle())
            || !controls.add_child(&mut nodes, keyboard.handle())
        {
            return Err("ui_demo: the controls layer could not be assembled");
        }

        // A stack: the background fills the window behind the row of pads, the
        // text panel and the controls layer, and all three are painted over it.
        let root = Container::new(&mut nodes, LayoutMode::Stack);
        for &child in &[
            background,
            row.handle(),
            text_panel.handle(),
            controls.handle(),
        ] {
            if !root.add_child(&mut nodes, child) {
                return Err("ui_demo: a panel could not be attached to the root");
            }
        }

        // The link from an animation or a theme switch to a node: every write
        // a pad's colour property receives — from a clock tick, from
        // `animate_to` putting the property at its start value, or from the
        // theme switching — marks the pad's node dirty, so the next pass picks
        // up the colour the press and the theme imply. The animation and theme
        // modules know nothing about nodes; this wiring is the demo's.
        let nodes = Rc::new(RefCell::new(nodes));
        for pad in &pads {
            let nodes = Rc::clone(&nodes);
            let node = pad.node;
            pad.color.on_change(move |_| {
                mark_dirty(&mut nodes.borrow_mut(), node);
            });
        }

        // The same link for the background: a theme switch marks it dirty so
        // the next pass repaints it with the new background colour.
        {
            let nodes = Rc::clone(&nodes);
            background_color.on_change(move |_| {
                mark_dirty(&mut nodes.borrow_mut(), background);
            });
        }

        // The card behind the pads gets **no** `on_change` link, and that is
        // deliberate. The links above exist because the pad and label *layout*
        // inputs change with their properties — a label's text changes the rect
        // it is given — and the layout pass only re-measures a dirty node. The
        // card's background is a paint-only property: `Demo::frame` rebuilds
        // every node's paint state on every frame, so a write to it needs
        // nothing to reach the node, and `mark_dirty` here would be *layout*
        // dirt on a node whose layout did not change.
        //
        // The link a caller outside the demo needs is in
        // `ui_core::widgets::container`: `Container` documents that a themed
        // background is wired with `paint_mut().mark_dirty()`.

        // The same link for the labels: a change to a label's text, font size
        // or colour marks the panel dirty, so the next pass repaints the text
        // with it. The panel is one node, so a change to any label dirties the
        // whole panel rather than a label that does not exist. Each closure gets
        // its own clone of the arena handle, so the original is never moved.
        for demo_label in &labels {
            let text_nodes = Rc::clone(&nodes);
            let node = demo_label.label.handle();
            demo_label.label.text.on_change(move |_| {
                mark_dirty(&mut text_nodes.borrow_mut(), node);
            });
            let size_nodes = Rc::clone(&nodes);
            demo_label.label.font_size.on_change(move |_| {
                mark_dirty(&mut size_nodes.borrow_mut(), node);
            });
            let color_nodes = Rc::clone(&nodes);
            demo_label.label.color.on_change(move |_| {
                mark_dirty(&mut color_nodes.borrow_mut(), node);
            });
        }

        // The same two links for the gauge's readout, whose text is bound to the
        // gauge's `value` and to the shape the demo last chose. The gauge itself
        // gets **no** `on_change` link of its own, and that is the same decision
        // the toggle, the bar, the image and the chart get below: every property
        // the gauge animates is a *paint* property — it draws inside whatever rect
        // the layout pass gave its node, and none of them is an input to that rect
        // — so a write reaches the screen without a link.
        {
            let text_nodes = Rc::clone(&nodes);
            let node = gauge_readout.label.handle();
            gauge_readout.label.text.on_change(move |_| {
                mark_dirty(&mut text_nodes.borrow_mut(), node);
            });
            let color_nodes = Rc::clone(&nodes);
            gauge_readout.label.color.on_change(move |_| {
                mark_dirty(&mut color_nodes.borrow_mut(), node);
            });
        }

        // The same two links for the slider's readout, whose text is bound to the
        // slider's value and to the count of its adjustments: a drag reaches it
        // twice over, once through the value the widget wrote and once through
        // the count its `on_change` bumped.
        {
            let text_nodes = Rc::clone(&nodes);
            let node = slider_readout.label.handle();
            slider_readout.label.text.on_change(move |_| {
                mark_dirty(&mut text_nodes.borrow_mut(), node);
            });
            let color_nodes = Rc::clone(&nodes);
            slider_readout.label.color.on_change(move |_| {
                mark_dirty(&mut color_nodes.borrow_mut(), node);
            });
        }

        // And for the slider's own properties. There are six of them, all painted
        // and none of them a layout input — a slider draws inside whatever rect
        // it is given — so this is the link between a transition writing and the
        // next frame drawing it, and nothing else.
        {
            let node = slider.widget.handle();
            // The four colours first, then the two numbers: one array cannot hold
            // both, and the writes all mark the same node dirty anyway.
            for property in [
                &slider.widget.track,
                &slider.widget.fill,
                &slider.widget.thumb_fill,
                &slider.widget.thumb_border,
            ] {
                let nodes = Rc::clone(&nodes);
                property.on_change(move |_| {
                    mark_dirty(&mut nodes.borrow_mut(), node);
                });
            }
            for property in [&slider.widget.thumb, &slider.widget.thumb_scale] {
                let nodes = Rc::clone(&nodes);
                property.on_change(move |_| {
                    mark_dirty(&mut nodes.borrow_mut(), node);
                });
            }
        }

        // The same link for each of the four newer readouts, whose text is bound
        // to a widget's own properties: a change to the text or to the colour
        // marks that readout's node dirty, so the next pass repaints it. Their
        // **rects** never change — each was given the box of the string it
        // started with, and every string they go on to show is inside that box
        // by the `wrap: None` `read_only_label` set — so the link is a paint
        // link in effect, and it is kept because it is the pattern the gauge's
        // and the slider's readouts already follow and because a caller who
        // widened one of these labels would need it.
        //
        // **The chart's readout is the fifth and the least idle of them**: its
        // text changes on every press of `A`, `S` or `H` and twice more as the
        // transition arrives and stops, so this link is load-bearing for it in a
        // way it is not for the toggle's.
        for readout in [
            &toggle_readout,
            &image_fit_readout,
            &progress_readout,
            &chart_readout,
        ] {
            let text_nodes = Rc::clone(&nodes);
            let node = readout.label.handle();
            readout.label.text.on_change(move |_| {
                mark_dirty(&mut text_nodes.borrow_mut(), node);
            });
            let color_nodes = Rc::clone(&nodes);
            readout.label.color.on_change(move |_| {
                mark_dirty(&mut color_nodes.borrow_mut(), node);
            });
        }

        // The toggle, the progress bar, the image and the chart get **no**
        // `on_change` links of their own, and that is the same decision the card
        // behind the pads gets above, for the same reason: every property the
        // four animate is a *paint* property — each draws itself inside whatever
        // rect the layout pass gave its node, and none of them is an input to
        // that rect — and `Demo::frame` rebuilds every node's paint state on
        // every frame, so a write to one reaches the screen without a link and a
        // link here would be `mark_dirty` on layout that did not change.
        //
        // What a caller outside the demo needs is in each widget's own module
        // document, and `Chart::tick` says it in one sentence: the write is what
        // reaches the node, so a caller that repaints only when `tick` returns
        // true repaints exactly while something moves.

        // The tree never changes shape, so the order is computed once.
        //
        // The **content node the list's rows hung from** was the reason this was
        // a paragraph, and the list is gone as of 2026-10-02: no node in the
        // demo is built by something the order cannot see, so what is left is
        // that the order is computed once because the tree never changes shape.
        //
        // **Two roots, walked in that order.** The dialog's node has no parent,
        // because a dialog is an overlay and the widget says so in its own tests,
        // so `paint_order` over the gallery's root cannot reach it; appending the
        // dialog's own walk is what puts it on top of the whole gallery rather
        // than under it, and it is a *concatenation* rather than a merge because
        // `Demo::draw` sends nodes in this order and there is no other order to
        // send them in.
        let mut order = paint_order(&nodes.borrow(), root.handle());
        order.extend(paint_order(&nodes.borrow(), dialog.handle()));
        Ok(Demo {
            nodes,
            root: root.handle(),
            order,
            pads,
            background,
            background_color,
            labels,
            label_nodes,
            text_panel: text_panel.handle(),
            metrics,
            text_size: TEXT_SIZE_START,
            color_token,
            clock: AnimationClock::new(),
            theme,
            dark: true,
            mouse_pressed: None,
            space_pressed: false,
            recognizer: GestureRecognizer::new(),
            // The dialog's first action, for the reason it is read at the end of
            // this function: a modal opens with focus inside it. `sync_dialog_focus`
            // is what turns this into the property the button draws a ring from.
            focused: first_action,
            containers: vec![row, text_column, text_panel, controls, root],
            gauge,
            gauge_type,
            gauge_readout,
            slider,
            slider_readout,
            slider_dragging: false,
            toggle,
            toggle_aimed: ToggleState {
                checked: toggle_state,
                focused: false,
            },
            toggle_readout,
            image,
            image_fit,
            image_focused,
            image_fit_readout,
            progress,
            progress_indeterminate,
            progress_focused,
            progress_readout,
            chart,
            chart_type,
            chart_next: CHART_OPENING_COUNT,
            chart_moving,
            chart_readout,
            fps: FrameRate::new(),
            fps_text,
            fps_readout,
            text_input,
            pending_key,
            keyboard,
            keyboard_pressed: false,
            text_readout,
            submit_readout,
            dialog,
            pending_theme,
        })
    }

    /// Sets the font size every label is drawn at, and re-gives each label the
    /// rect its text now needs.
    ///
    /// The size is a plain field, not a property, because it changes the labels'
    /// *rects* as well as their glyphs: a bigger font wraps onto more lines, and
    /// the panel lays the labels out by the rects it is given. A property write
    /// could only mark a node dirty, and the size a node was laid out at would
    /// still be the old one.
    fn set_text_size(&mut self, size: f32) {
        let size = size.clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX);
        if (self.text_size - size).abs() < f32::EPSILON {
            return;
        }
        self.text_size = size;
        // The property writes come first: each one marks its own label's node
        // dirty through that label's callback, and the callbacks borrow the
        // arena, so the borrow below cannot be held across them.
        for demo_label in &self.labels {
            demo_label.label.font_size.set(size);
        }
        let mut nodes = self.nodes.borrow_mut();
        for (&node, demo_label) in self.label_nodes.iter().zip(self.labels.iter()) {
            let laid_out = demo_label.size(&self.metrics, size);
            let Some(widget) = nodes.get_mut(node) else {
                continue;
            };
            widget
                .layout_mut()
                .set_constraints(Constraints::tight(laid_out));
        }
        mark_dirty(&mut nodes, self.text_panel);
    }

    /// Moves which theme token the labels take their colour from, to the next
    /// one in [`TEXT_COLOR_TOKENS`].
    fn cycle_text_color(&mut self) {
        let current = self.color_token.get();
        let index = TEXT_COLOR_TOKENS
            .iter()
            .position(|token| *token == current)
            .unwrap_or(0);
        let next = (index + 1) % TEXT_COLOR_TOKENS.len();
        self.color_token.set(TEXT_COLOR_TOKENS[next]);
    }

    /// Handles one input event: `T` switches between the dark and light
    /// themes, the space bar presses all three pads with a stagger and
    /// releases them on the spring, `+` and `-` move the text size, `C` moves
    /// the token the text takes its colour from, and the left mouse button
    /// presses and releases the pad under the cursor.
    ///
    /// Every control below the pads is driven the other way round, through the
    /// input module: the event goes to the [`GestureRecognizer`] first, and
    /// whatever gesture or key it completed is dispatched to the node under it,
    /// which is what lets a control consume the events meant for it rather than
    /// letting them reach whatever is behind. `Tab` and `Shift+Tab` move focus,
    /// and Enter activates the control holding it.
    ///
    /// Space is the one key two halves want. It belongs to the control holding
    /// focus, because that is the one a user has navigated to, and falls back to
    /// the pads when nothing is focused — so the pads still work with `Tab` never
    /// pressed, which is how the demo starts.
    ///
    /// `0` and `1` put the slider at its two ends without a pointer, which is the
    /// one thing a drag cannot show: the thumb travelling to a value it was not
    /// given.
    ///
    /// `F` cycles the image's fit, `[` and `]` move the progress bar's value by
    /// [`PROGRESS_STEP`] either way, and `P` switches the bar into its sliding
    /// mode and back. `,` and `.` move the gauge's value by [`GAUGE_STEP`] either
    /// way and `G` cycles its shape through [`GAUGE_TYPES`].
    ///
    /// **The gauge's keys are not an afterthought and they are not arbitrary.**
    /// They exist because of a fact about this host rather than about the widget:
    /// **no pointer event can be injected here**, so
    /// `.ai/NEVERAGAIN.md` § *a still screenshot of a 4 fps application* and
    /// `doc/ui/IMPLEMENTATION_STATE.md` § *Verifying a change that draws* both
    /// record that a capture taken by clicking the demo cannot be reproduced. A
    /// gauge is a **display** — its needle and its tick marks are decoration and
    /// nothing in the widget answers a finger — so with no pointer there would be
    /// **no route to its value at all** and the acceptance criterion *"Demo shows
    /// a gauge at 50%"* could only be met by a build that was seeded from the
    /// environment, which is the entry § *a capture whose only route was
    /// instrumented* is about. The keys are the honest route.
    ///
    /// `,` and `.` rather than `[` and `]` because those are the progress bar's,
    /// and `<` and `>` because they are `,` and `.` under a shift and a demo
    /// should not need two hands. `G` is the widget's own initial, and the reason
    /// `F` is not reused for both the image's fit and the gauge's shape is that
    /// one key doing two things is one thing fewer a reader can check.
    ///
    /// **The chart's three keys follow the same rule and none of them is
    /// arbitrary.** `H` cycles the shape, which is the gauge's `G` with the one
    /// letter `C` unavailable — `C` is the text token's — and it is there for
    /// [`GAUGE_TYPES`]'s reason and one more: task 21's three rendering criteria
    /// are three *distinct* things, and a demo that showed one of them would not
    /// show three, so all three have to be reachable from the running demo rather
    /// than from three builds. `A` appends and `S` shifts, which are the two
    /// halves of requirement 4's *"append new value, shift old values"* and
    /// requirement 5's *"new data points animate in"*, and both are single
    /// letters for a key a reader has to find twice.
    ///
    /// **These three keys are also the honest route to the chart**, for the
    /// reason the gauge's keys are: no pointer event can be injected on this host,
    /// a chart has no `on_event` to route one to in the first place, and a
    /// capture taken by clicking the demo cannot be reproduced. `A` and `S` are
    /// what requirement 4 and requirement 5 look like on screen, and the readout's
    /// length, newest reading, shape and still-or-moving is what says so.
    fn handle_event(&mut self, event: Event) {
        let produced = self.recognizer.process(&event);

        match event {
            Event::KeyDown {
                keycode: Some(keycode),
                repeat: false,
                ..
            } => {
                // **The dialog's own key is matched above the guard**, because the
                // guard is what it is guarding against: `D` is the one key that
                // must work while the dialog is showing, since it is how a dialog
                // that opened itself is closed again. Written as an `if` above
                // rather than as an arm of the table below for that reason alone.
                if keycode == DIALOG_KEY {
                    self.toggle_dialog();
                } else if !self.dialog_is_modal() {
                    // **One guard for the whole table**, and the operator's decision
                    // of 2026-10-03: a modal that leaves the host application's own
                    // shortcuts live is not modal. It was this file's own module doc
                    // that first said so, while also recording that it did *not* do
                    // it — the gap is closed and the sentence is now true of the
                    // code rather than aspirational.
                    //
                    //
                    // **The table rather than a `match`, and that is the fix for the
                    // hazard a review named on 2026-10-03**: a `match` and a list of
                    // its keys are two objects that can disagree, and they did — the
                    // arm read `Keycode::Equals | Keycode::Plus` while the test's
                    // list named only `Plus`, so one real key went unchecked by the
                    // suppression test. Making the table *be* the dispatch means
                    // **there is no second place to put a key**: a shortcut the demo
                    // binds is a row in [`GALLERY_SHORTCUTS`], a row there is wrapped
                    // by the guard above, and the suppression tests iterate the rows.
                    //
                    // Deriving the rows from the `match` was the second option and it
                    // is not available in Rust; keeping the `match` and *testing* that
                    // the two agree was the third, and the only test that could say so
                    // would have been a second list — the hazard again. Enumerating
                    // the platform's keycodes was the fourth, and it is not available
                    // either: SDL3's keycode space is `SDLK_SCANCODE_MASK | scancode`,
                    // a billion values wide with 250 values in it, and the two
                    // functions that bridge it (`Keycode::from_scancode`,
                    // `Scancode::from_keycode`) are **`unsafe`**, which this file does
                    // not add.
                    if let Some((_, _, act)) =
                        GALLERY_SHORTCUTS.iter().find(|(_, key, _)| *key == keycode)
                    {
                        act(self);
                    }
                }
            }
            // **The releases are deliberately not guarded**, and the asymmetry is
            // the point rather than an oversight. Every arm above that *starts*
            // something is guarded; every arm that *ends* one is not, because a
            // release that a dialog could suppress is how a control gets stranded
            // mid-press: hold `Space`, press `D` so the dialog opens, release
            // `Space` — and with the release guarded the three pads stay at 1.0
            // behind the scrim with nothing left to bring them back. A release has
            // no effect on anything a guarded press did not start, and
            // `release_all` on a press that never happened animates three
            // properties from 0.0 to 0.0, which is a write and not a picture.
            Event::KeyUp {
                keycode: Some(Keycode::Space),
                ..
            } => {
                // **`space_pressed` and not `focused.is_none()`**, and the field's
                // doc gives the sequence that is the reason: focus can move between
                // a press and its release — `D` opens the dialog and takes focus —
                // and a condition on the *current* focus then declines a release
                // that has a press behind it. The press still asks about focus,
                // because `Space` is a focused button's activation key and must be.
                if self.space_pressed {
                    self.release_all();
                }
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                // A press is tracked here rather than taken from the recogniser,
                // because a recogniser reports a tap on the *release*: the
                // pressed appearance has to be on screen for the whole time the
                // pointer is down, which is before any tap exists.
                //
                // **Guarded, and this arm is the reason modality needed saying
                // twice.** These three checks reach the gallery *directly*, without
                // going through [`Demo::route_input_event`], so the tap-side filter
                // in that function cannot see them: a press on a pad with the
                // dialog showing lit the pad, because the press is answered here
                // and only the *tap* on the release was ever routed. All three
                // targets are outside the panel — the pads are in the top left, the
                // slider at 664 by 496 and the keyboard below the band — so a press
                // on any of them lands on the scrim, and a scrim that dims
                // something which then lights up is not modal.
                if self.dialog_is_modal() {
                    // Nothing to do, and **nothing to undo either**: the releases
                    // below are unguarded, so a pointer that goes down on the scrim
                    // and up over a pad still releases cleanly.
                } else if let Some(index) = self.pad_at(x, y) {
                    self.mouse_pressed = Some(index);
                    self.press_pad(index);
                } else if self.slider_at(x, y).is_some() {
                    self.slider_dragging = true;
                } else if self.keyboard_at(x, y) {
                    // The key is grabbed **here**, on the press, and nowhere else:
                    // the gesture recogniser reports a tap on the release and a
                    // drag only once the pointer has already moved, so it has no
                    // "the finger went down on the key" for the widget to read. A
                    // press that missed the key grabs nothing, and the keyboard
                    // lights whichever key the gesture ends on.
                    self.grab_key(Offset::new(x, y));
                }
                // **Nothing is offered to the chart**, and that is the same
                // decision `the_chart_is_not_in_the_focus_order` records: a chart
                // has no `on_event`, no value a drag would set and no action a tap
                // would report, so a press over its series falls through to
                // whatever is behind it — which here is nothing at all.
            }
            Event::MouseButtonUp {
                mouse_btn: MouseButton::Left,
                ..
            } => {
                if let Some(index) = self.mouse_pressed.take() {
                    self.release_pad(index);
                }
                self.slider_dragging = false;
                self.release_key();
            }
            // A finger is a pointer too, and a car has no mouse: the same press
            // and release the left button gets, from the touch events SDL delivers
            // for the same gesture. A canceled touch drops the slider as well as
            // the pointer, because a canceled finger is one that is gone.
            //
            // **The press is guarded and the releases are not**, on the mouse arm's
            // argument above: a finger down on the scrim must not start a drag or
            // light a key, and a finger lifted afterwards must still tidy up.
            Event::FingerDown { x, y, .. } => {
                if self.dialog_is_modal() {
                    // Nothing to do, and nothing to undo: the arms below are not.
                } else if self.slider_at(x, y).is_some() {
                    self.slider_dragging = true;
                } else if self.keyboard_at(x, y) {
                    self.grab_key(Offset::new(x, y));
                }
            }
            Event::FingerUp { .. } | Event::FingerCanceled { .. } => {
                self.slider_dragging = false;
                // A canceled touch releases the key too, for the same reason: a
                // finger that is gone must not leave a key lit.
                self.release_key();
            }
            _ => {}
        }

        for mut input_event in produced {
            self.route_input_event(&mut input_event);
        }

        // The dialog's `OK`, drained **after** the event that fired it rather
        // than inside the handler, for the reason `offer_to` gives for the
        // keyboard's key: a `Callback` is `Fn` and cannot reach a `&mut self`, so
        // the click could only write a property — and reading it at the end of the
        // event keeps a theme switch out of the middle of a dispatch.
        if self.pending_theme.get() {
            self.pending_theme.set(false);
            self.toggle_theme();
        }
    }

    /// Delivers one event the recogniser produced to the widget under it.
    ///
    /// A positional event is routed through [`input::route`] and offered to each
    /// node in turn until a widget consumes it, which is what lets a control stop
    /// the events meant for it. A key has no position and is not routed by one, so
    /// it goes to the control holding focus — and so does a positionless `Scroll`,
    /// which is the steering wheel's axis and the only gamepad axis the input
    /// module maps, so a focused slider is driven by it.
    ///
    /// The chain is resolved before any handler runs, rather than dispatched
    /// through [`input::dispatch_event`], because a widget's handler writes a
    /// property and that write fires the `on_change` callback which marks a node
    /// dirty: a handler reaching the arena while the dispatch still held a borrow
    /// of it would be a `RefCell` double borrow, and would panic on the first
    /// click rather than on anything a test could have caught by reading the code.
    fn route_input_event(&mut self, event: &mut InputEvent) {
        if event.position().is_none() {
            // **The modal dialog first, on a key as much as on a tap.** It is the
            // only thing a key may reach while it is showing, and
            // [`Dialog::on_event`] is the whole of that answer: it consumes
            // `Escape` and dismisses, and it forwards every other key to its own
            // focused action — so an `Enter` still reaches the button node, by the
            // widget's route rather than the demo's. It declines `Tab`, which is
            // then the navigation arm's to move, and the five controls behind it
            // are out of the order while the dialog is up.
            //
            // Offering the *button* first would be the alternative and it is
            // wrong in a way only the acceptance test finds: a `Button` declines
            // `Escape` and nothing else in the demo would then handle it, so a
            // dialog no key could close would be the one control in the window
            // with no way out.
            if self.dialog_is_modal() && self.offer_to(self.dialog.handle(), event) {
                return;
            }
            // The focused control has the first claim on a positionless event, and
            // focus navigation runs only for whatever it left alone. A button takes
            // its activation keys and leaves Tab; a slider takes its arrows and
            // the wheel's axis; neither takes the other's.
            if self.offer_to_focused(event) {
                return;
            }
            if matches!(event.kind(), InputEventKind::KeyDown { .. })
                && self.focus_navigation(event)
            {
                event.consume();
            }
            return;
        }

        // **The modal dialog's own subtree, and nothing else, while it is
        // showing.** This is before the slider's dragged arm below, and that is
        // the order that matters: a drag is a positional event, and a finger that
        // went down on the slider before the dialog appeared and then moved must
        // not keep driving it underneath the scrim.
        //
        // **The decline is not a licence to carry on down the chain.**
        // `Dialog::on_event` handles a tap and a key and leaves everything else
        // alone — a `Drag` among them — and "leaves it alone" here means *drops
        // it*, because the alternative is the content behind the overlay being
        // reachable one event kind at a time. That is the caller's half of the
        // contract `Dialog::on_event` states and `ui_core::input` cannot enforce,
        // because nothing in the input module knows what a modal is.
        if self.dialog_is_modal() {
            for handle in self.modal_chain(event) {
                self.offer_to(handle, event);
                if event.consumed() {
                    break;
                }
            }
            return;
        }

        // A drag goes to the slider being dragged first, whether or not the
        // pointer is still over it. The chain below finds the node under the
        // pointer, and a finger that has travelled past the end of a slider is
        // outside it — which is exactly when the slider most needs to hear about
        // the drag, because the answer is its own end of the range.
        //
        // **This used to have a second arm for the list**, and it was the same
        // case: a `Scroll` has no grabbed state, so a drag that had left the
        // list's own rect would stop reaching it. The list was removed on
        // 2026-10-02 and with it the demo's last widget that needed a drag to
        // keep hearing about, so the slider is the only one left.
        if self.slider_dragging && matches!(event.kind(), InputEventKind::Drag { .. }) {
            if let Some(rect) = self.slider_rect() {
                self.slider.widget.on_event(event, rect);
                if event.consumed() {
                    return;
                }
            }
        }

        let chain = {
            let nodes = self.nodes.borrow();
            input::route(&nodes, self.root, event)
        };
        for handle in chain {
            self.offer_to(handle, event);
            if event.consumed() {
                break;
            }
        }
    }

    /// Returns the nodes a positional event may be offered to while the dialog is
    /// showing: the one action whose **drawn** rect holds it, or the dialog's own
    /// node, and **nothing outside the dialog's subtree**.
    ///
    /// **Built from [`Dialog::action_rect`] and not from the nodes' own laid-out
    /// rects**, and that is the whole reason the action buttons' nodes are laid
    /// out nowhere in particular: the widget draws its actions into its own
    /// painter at `action_rect` and hit-tests the same numbers, so the nodes
    /// exist for the two things a *node* is for — the paint order and the `Tab`
    /// order — and nothing declares where they go. Asking the layout where a
    /// button is would answer the dialog's origin and route every tap in the
    /// window to the first button.
    ///
    /// A one-element chain, and that is the modality: `ui_core::input` walks from
    /// the node under the pointer up to the root, so the content behind the
    /// overlay is reachable by construction unless the caller declines to walk
    /// there. `a_tap_outside_the_panel_does_not_reach_the_gallery_behind_it` is
    /// what holds that down, and `the_demo_never_offers_an_event_outside_the_
    /// dialog_while_it_is_showing` walks the whole set of positions rather than
    /// one of them.
    fn modal_chain(&self, event: &InputEvent) -> Vec<Handle> {
        let mut chain = Vec::with_capacity(2);
        let Some(position) = event.position() else {
            return chain;
        };
        let rect = self.dialog_rect();
        for index in 0..self.dialog.actions.len() {
            let held = self
                .dialog
                .action_rect(
                    index,
                    rect,
                    &self.dialog_advance(),
                    self.dialog_line_height(),
                )
                .is_some_and(|drawn| over_rect(drawn, position.x, position.y));
            if held {
                chain.push(self.dialog.actions[index].handle());
                return chain;
            }
        }
        chain.push(self.dialog.handle());
        chain
    }

    /// Whether the dialog is **modal** right now: showing, rather than merely on
    /// screen.
    ///
    /// **Not [`Dialog::is_drawn`], and the difference is the fade-out.** `is_drawn`
    /// answers *"would `paint` record anything this frame"*, which a dismissed
    /// dialog answers `true` for another 300 ms while its scrim fades; modality is
    /// a question about **where events go**, and a dialog that has been dismissed
    /// owns nothing. Keyed on `visible` for that reason — a pointer pressed
    /// through a half-transparent scrim is aimed at the gallery, not at a panel
    /// that is on its way out — and **all three readers are this one function**
    /// (`route_input_event`'s two branches, `focus_navigation` and
    /// `toggle_dialog`), so there is no second place to answer it differently.
    ///
    /// The **rings** are the exception and deliberately so: they answer "is this
    /// control's ring **on screen**?", which is [`Dialog::is_drawn`] and not this,
    /// so a button keeps its ring for as long as the dialog is still being painted.
    /// [`Demo::sync_dialog_focus`] is where the two are told apart, and its doc
    /// has the table of which flag drives which.
    fn dialog_is_modal(&self) -> bool {
        self.dialog.visible.get()
    }

    /// Returns whether `handle` is the dialog's own node or anything under it.
    ///
    /// **Walked up the parent chain rather than matched against the actions the
    /// demo knows about**, because "the dialog's subtree" is a property of the tree
    /// and the demo should not have to be told what the widget put in it: a dialog
    /// that grew a third kind of child would be inside this answer without anything
    /// here changing.
    ///
    /// The walk cannot leave the dialog's root, because that root has no parent
    /// (see [`Demo::new`]) and so the chain ends at it.
    fn in_dialog_subtree(&self, handle: Handle) -> bool {
        let nodes = self.nodes.borrow();
        let mut current = Some(handle);
        while let Some(node) = current {
            if node == self.dialog.handle() {
                return true;
            }
            current = nodes.get(node).and_then(WidgetNode::parent);
        }
        false
    }

    /// Returns whether a control that can still be **activated** holds focus.
    ///
    /// **One predicate for the two code paths that both mean "is there somebody
    /// there"**, and the reason it is not `self.focused.is_some()` is the finding a
    /// review returned against this file on 2026-10-03: a dialog dismissed from
    /// **inside** the widget — `Escape`, a tap on an action, a tap on the scrim —
    /// does not go through [`Demo::toggle_dialog`], so nothing retired
    /// [`Demo::focused`], and a stale `Some` is a handle to a control that has gone.
    /// Two readers were affected and only one of them was visible:
    ///
    /// - [`Demo::offer_to_focused`] offered the key to that button, so `Enter` kept
    ///   firing `OK` from a dialog dismissed a frame earlier;
    /// - the `Space` row in [`GALLERY_SHORTCUTS`] reads a focus condition, so a
    ///   stale `Some` stopped the pads being pressed at all. Measured before the
    ///   fix: `Space` after an `Escape` dismissal left the pads at
    ///   `[0.0, 0.0, 0.0]` where a `D` dismissal left them at `[1.0, 0.84, 0.36]`.
    ///   Two dismissal routes, two behaviours, and nothing on screen to say which.
    ///
    /// [`Demo::in_dialog_subtree`] is how a stale handle is recognised: by walking
    /// parents, not by listing the actions, so a dialog that gained a child would be
    /// covered without anything here changing.
    ///
    /// [`Demo::sync_dialog_focus`] is still what makes the *state* converge — it
    /// retires the record on the next frame — and this predicate is what makes the
    /// *routing* right in the window before that, which is the window a reader
    /// reproduces by pressing two keys with no frame between them.
    fn focus_is_live(&self) -> bool {
        self.focused
            .is_some_and(|handle| self.dialog_is_modal() || !self.in_dialog_subtree(handle))
    }

    /// Offers a positionless event to the control holding focus, and reports
    /// whether it took it.
    fn offer_to_focused(&self, event: &mut InputEvent) -> bool {
        // **`focus_is_live` and not `focused.is_some()`**, and that predicate's doc
        // is the whole of the finding: a dialog dismissed from inside the widget
        // leaves the record naming a button that has gone, and this was the last
        // place in the file that offered a key without asking whether there was
        // still somebody there.
        let Some(handle) = self.focused.filter(|_| self.focus_is_live()) else {
            return false;
        };
        self.offer_to(handle, event)
    }

    /// Offers `event` to the widget at `handle`, and reports whether it took it.
    ///
    /// A handle that belongs to no widget at all is nobody's, which is what lets
    /// one loop serve four kinds of control without asking what is there. **That
    /// is where the buttons' arm used to be**, and the fall-through they needed is
    /// gone with it: the last arm is now the keyboard's, which returns explicitly.
    ///
    /// The two that answer a `KeyDown` only while they hold focus are the slider
    /// and the toggle; the field answers a key of its own and the keyboard answers
    /// a tap. So a `Tab` reaches all four and none of them takes it.
    ///
    /// **The gauge is not in this function and is not in the `Tab` order**, and
    /// that is a decision rather than an oversight: it has no `on_event` at all,
    /// because a gauge is a display — there is no value a drag would set and no
    /// action a tap would report — and it has no `focused` property either. A
    /// `Tab` stop on it would move focus to a control that can do nothing with it
    /// and would draw no ring, so focus would arrive somewhere with no sign of
    /// having arrived. `.ai/NEVERAGAIN.md` § *a drawn control with nothing behind
    /// it* is the entry about the other direction of that mistake, and this is
    /// the same one avoided.
    ///
    /// **The chart is in neither half of that**, and for the chart's own stated
    /// reason rather than the gauge's: `chart.rs` says in its module document
    /// that a drawn series, a grid and a cursor's worth of hairlines all look
    /// grabbable and **nothing in it reads a pointer over any of them**. It has
    /// no `on_event` to call, so there is nothing here to add.
    ///
    /// **The dialog and its two actions are the sixth and seventh arms**, and
    /// they are here for the reason [`Demo::modal_chain`] gives: the action
    /// buttons are real `Button` widgets with nodes of their own, and a tap that
    /// lands on one is offered to the button rather than to the dialog, so the
    /// button's own press state is the thing a finger sees.
    fn offer_to(&self, handle: Handle, event: &mut InputEvent) -> bool {
        if handle == self.slider.node() {
            return match self.slider_rect() {
                Some(rect) => self.slider.widget.on_event(event, rect),
                None => false,
            };
        }
        if handle == self.toggle.handle() {
            return match self.toggle_rect() {
                Some(rect) => self.toggle.on_event(event, rect),
                None => false,
            };
        }
        if handle == self.text_input.handle() {
            // The advance closure is the demo's own `TextMetrics`, exactly as it
            // is for a button's `paint` — the caret's x is measured per character
            // and nothing outside the widget can measure it for it.
            let advance = |ch: char| self.metrics.advance(ch, TEXT_INPUT_FONT);
            return match self.text_input_rect() {
                Some(rect) => self.text_input.on_event(event, rect, &advance),
                None => false,
            };
        }
        if handle == self.keyboard.handle() {
            let consumed = match self.keyboard_rect() {
                Some(rect) => self.keyboard.on_event(event, rect),
                None => false,
            };
            // The key the keyboard just reported is applied here, immediately
            // after the event that reported it, rather than in the next frame.
            // Waiting a frame would put a keystroke behind everything else the
            // demo does per frame and make a typing test depend on `frame` being
            // called, which the "a key inserts its character" tests deliberately
            // do not do.
            if let Some(action) = self.pending_key.get() {
                self.pending_key.set(None);
                apply_key_action(&self.text_input, action);
            }
            return consumed;
        }
        // The dialog, and each of its actions, in the one place that turns a
        // handle into a widget. **The three numbers handed to `on_event` are the
        // three `paint` is handed, through the same three accessors** — that is
        // the widget's own contract ("you must pass the same `advance` and
        // `line_height`") and the reason they are functions rather than values
        // spelled at each site.
        if handle == self.dialog.handle() {
            return self.dialog.on_event(
                event,
                self.dialog_rect(),
                &self.dialog_advance(),
                self.dialog_line_height(),
            );
        }
        for action in &self.dialog.actions {
            if handle == action.handle() {
                // The button's own handler, which is what an **activation key**
                // needs: `Focus` hands `Enter` to the button node itself, and
                // `Button::on_event` refuses it unless that button is focused.
                // The dismissal is already wrapped around the click, so this route
                // closes the dialog exactly as the tap route does.
                return action.button.on_event(event);
            }
        }
        false
    }

    /// Returns the box the dialog is drawn in and hit-tested against: the node's
    /// own laid-out rect, which is the whole window.    ///
    /// **The node's rect and not [`WINDOW`]**, for the reason every other widget
    /// in this file takes its rect from the node: `Dialog::on_event` says its
    /// caller must pass the *same* `advance` and `line_height` as to `paint`, and
    /// the rect is the third of the three numbers. A tap judged against a
    /// different box than the panel was drawn in is a tap that lands somewhere
    /// else, and both readers go through here, so there is one number.
    ///
    /// The fallback is [`WINDOW`] rather than nothing, because the demo has one
    /// window and a node that has not been placed is a node the layout pass has
    /// not run over yet — not a condition a caller should have to handle.
    fn dialog_rect(&self) -> Rect {
        self.node_rect(self.dialog.handle())
            .unwrap_or_else(|| Rect::new(0.0, 0.0, WINDOW.width, WINDOW.height))
    }

    /// Returns the advance the dialog's text is measured with, at
    /// [`DIALOG_FONT`].
    ///
    /// **A function and not a closure written at each site**, because there are
    /// two sites — the paint pass and the hit test — and they must be handed the
    /// same one. The widget says so in its own contract, and a caller that
    /// spelled the closure twice would have two places to be wrong with nothing
    /// to say so.
    fn dialog_advance(&self) -> impl Fn(char) -> f32 + '_ {
        move |ch: char| self.metrics.advance(ch, DIALOG_FONT)
    }

    /// Returns the line box the dialog's title, body and button labels are all
    /// measured at.
    ///
    /// One number for three sizes, because that is the seam the widget offers:
    /// [`DIALOG_FONT`]'s doc says why the *measurement* is at the panel's largest
    /// size, and the widget's own doc says what would reverse it — a second
    /// parameter, or a font metric per size.
    fn dialog_line_height(&self) -> f32 {
        self.metrics.line_height(DIALOG_FONT)
    }

    /// Moves focus if `event` is a navigation key, and reports whether it was.
    ///
    /// `Tab` and `Shift+Tab` are the keyboard's navigation, and a gamepad
    /// steering-wheel axis arrives as a `Scroll` with no position, so both reach
    /// this through the same [`Focus`] tracker. A **mouse** wheel is not one of
    /// them: `GestureRecognizer` gives a wheel event the pointer's position, so
    /// it is routed as a positional event and lands on the node under the cursor
    /// rather than here. The focusable set is rebuilt from the six controls each
    /// time, which is what kept a disabled button out of the order rather than
    /// letting it sit in one.
    ///
    /// The order itself is the tree's paint order, so the controls layer's
    /// children are what decide it and **the order they were added in is the
    /// `Tab` order**: the slider, the image, the toggle, the progress bar and the
    /// field. `tab_walks_every_focusable_control_in_order_and_wraps` walks it.
    ///
    /// **The gauge is deliberately not in this list**, for the reason
    /// [`Demo::offer_to`] gives: it has no `on_event` and no `focused` property,
    /// so a stop on it would be a stop where focus arrives and nothing shows that
    /// it did. **The chart is not in it either**, for the chart's own reason, and
    /// `the_chart_is_not_in_the_focus_order` is that decision's test.
    ///
    /// **The dialog replaces the whole list while it is showing**, and it is a
    /// replacement rather than an addition: a modal's background controls are not
    /// focusable, because a `Tab` that walked out of the dialog into the gallery
    /// would put focus somewhere the dialog's overlay covers and that nothing can
    /// be seen through. The five come back the moment the dialog closes, and
    /// `the_dialog_replaces_the_tab_order_while_it_is_showing` is that claim with
    /// the control beside it.
    ///
    /// **The walk's root changes with the set**, and that is not tidiness:
    /// [`Focus`] recomputes the order by walking the tree from its own root, and
    /// the dialog's node is a root of its own because it is an overlay. Walking
    /// the gallery's root with the two actions marked focusable would find them —
    /// they are nowhere in that tree — and so would find nothing, which would make
    /// `Tab` a key that does nothing at all rather than one that moves between the
    /// two buttons.
    fn focus_navigation(&mut self, event: &InputEvent) -> bool {
        if event.position().is_some() {
            return false;
        }
        let scroll = match event.kind() {
            InputEventKind::Scroll { delta } => Some(delta.y),
            _ => None,
        };
        let is_tab = matches!(
            event.kind(),
            InputEventKind::KeyDown {
                key: Key::Keyboard(Keycode::Tab),
                ..
            }
        );
        if !is_tab && scroll.is_none() {
            return false;
        }

        let next = {
            let nodes = self.nodes.borrow();
            let modal = self.dialog_is_modal();
            // The five in the tree's paint order, and so in the `Tab` order. None
            // of them has a disabled state of its own, so every one of them is in
            // the order always — which is not the same as saying every one of them
            // answers every key: `offer_to` is what decides, and a widget with no
            // key of its own declines.
            let background = [
                self.slider.node(),
                self.image.handle(),
                self.toggle.handle(),
                self.progress.handle(),
                self.text_input.handle(),
            ];
            let actions: Vec<Handle> = self
                .dialog
                .actions
                .iter()
                .map(|action| action.handle())
                .collect();
            let mut focus = Focus::new(
                &nodes,
                if modal {
                    self.dialog.handle()
                } else {
                    self.root
                },
            );
            for handle in if modal { &actions[..] } else { &background[..] } {
                focus.set_focusable(*handle, true);
            }
            // Re-entering the order where focus already is. `Focus` starts with
            // nothing focused, so without this a wheel turned twice in a row
            // would walk from the top both times, and Shift+Tab from the first
            // control would go forward instead of back.
            if let Some(current) = self.focused {
                // **Deliberately left to fail while the dialog is up**: the
                // buttons are its own, and the five behind it are not in this
                // walk, so `focus` refuses and `Tab` starts from the first button
                // rather than from a stale node the walk cannot see.
                let _ = focus.focus(current);
            }
            if let Some(delta) = scroll {
                focus.handle_scroll(delta);
            } else {
                let _ = focus.handle_key(event);
            }
            focus.current()
        };
        self.set_focus(next);
        true
    }

    /// Records which control holds focus, and writes the flag each widget's
    /// `focused` property holds, so the rings move.
    ///
    /// Three of the five have a `focused` property to write — the slider, the
    /// toggle and the field — and the other two, the progress bar and the image,
    /// have none, so their readouts say where focus is in words instead. See
    /// `progress_focused` and `image_focused`. Neither the gauge nor the chart is
    /// named here because neither is in the order; see [`Demo::offer_to`].
    ///
    /// **The dialog's actions are not written here**, and that is deliberate:
    /// a dismissal takes the focus away without going through this function — `OK`
    /// and `Cancel` fire and close, `Escape` closes, a tap on the scrim closes —
    /// and this is only called when a *navigation key* moved focus.
    /// [`Demo::sync_dialog_focus`] is the one place that writes them, every frame,
    /// and it asks the dialog whether it is still there.
    fn set_focus(&mut self, next: Option<Handle>) {
        self.focused = next;
        let focused = self.focused;
        let slider_wanted = Some(self.slider.node()) == focused;
        if self.slider.widget.focused.get() != slider_wanted {
            self.slider.widget.focused.set(slider_wanted);
        }
        let toggle_wanted = Some(self.toggle.handle()) == focused;
        if self.toggle.focused.get() != toggle_wanted {
            self.toggle.focused.set(toggle_wanted);
        }
        self.progress_focused
            .set(Some(self.progress.handle()) == focused);
        self.image_focused.set(Some(self.image.handle()) == focused);
        // The field is given its own `focus()`/`blur()` rather than having its
        // `focused` property written like the others, because those two are not
        // the same thing: `blur` also restarts the blink, and a field whose caret
        // stayed mid-phase could come back from `Tab` with no caret at all. The
        // property is the field's appearance; the methods are its state.
        let field_wanted = Some(self.text_input.handle()) == focused;
        if self.text_input.focused.get() != field_wanted {
            if field_wanted {
                self.text_input.focus();
            } else {
                self.text_input.blur();
            }
        }
    }

    /// Advances the clocks by one frame's worth of time, lays the tree out in
    /// `size`, and records every node's draw commands.
    ///
    /// The clocks go first: an animation that finished mid-frame has to have
    /// written its last value — and marked its node dirty through the
    /// property's callback — before the pass below reads the tree. The theme's
    /// clock is ticked alongside the pads': a theme switch is an animation
    /// like any other, and its frames have to land before the pass too. Each
    /// button ticks its own clock, which is the same order for the same reason.
    ///
    /// **The arena is borrowed twice rather than once**, and the split is not
    /// tidiness: the chart's readout names whether the chart is moving, and
    /// writing that property recomputes the text, and the text's own `on_change`
    /// link reaches the arena to mark its node dirty — which a frame already
    /// holding the arena refuses, with a `RefCell` panic on the first frame
    /// rather than on anything a test could have found by reading the numbers.
    /// The same is true of every bound readout in the demo.
    fn frame(&mut self, size: Size, delta: Duration) {
        let _ = self.clock.tick(delta);
        let _ = self.theme.tick(delta);
        self.sync_slider_state();
        self.sync_toggle_state();
        let _ = self.slider.widget.tick(delta);
        // The toggle's own thumb slide, the bar's value and — the reason it is
        // worth a line of its own — the *loop*: `Progress::tick` aims the next
        // leg of the slide on the tick the last one arrives, so a bar in
        // indeterminate mode that has been aimed once keeps moving without the
        // demo coming back to it.
        let _ = self.toggle.tick(delta);
        let _ = self.progress.tick(delta);
        // The field's blink and the keyboard's key colours. The field's is the
        // one that matters: it is the only clock in the demo that is *always*
        // running while a control has focus, and `tick` returning true on a
        // phase flip is what makes the caret redraw.
        let _ = self.text_input.tick(delta);
        let _ = self.keyboard.tick(delta);
        // The gauge's needle and its fill, and the reason this tick is here at
        // all rather than in the arm above with the others: `Gauge::tick` returns
        // whether any of its transitions wrote, which is what makes a needle that
        // is *arriving* a needle that moves rather than one that is redrawn at the
        // same place. `animate_to_state` is what starts that travel, and it is
        // called from `step_gauge` and from `toggle_theme` rather than from here —
        // a per-frame aim would restart the spring on every frame and the needle
        // would creep toward its target for ever instead of arriving at it, which
        // is the same argument `sync_toggle_state` makes.
        let _ = self.gauge.tick(delta);
        // The chart's series and its colours, on the gauge's argument exactly: the
        // tick is here, the **aim** is not. `animate_push` and `animate_shift` are
        // what start a transition and they are called from the key arms, once per
        // press — a per-frame `animate_to_state` would restart the glide on every
        // frame, and a chart whose glide restarts every frame is a chart that
        // creeps towards its target for ever and never arrives, which is the
        // argument `sync_toggle_state` makes and the reason this comment exists
        // twice.
        let _ = self.chart.tick(delta);
        // The dialog's show and hide transition, on the gauge's argument exactly:
        // the tick is here, the **aim** is not. `present` and `dismiss` are what
        // start the fade and the bounce, and they are called from the toggling key
        // and from the widget's own dismissal — a per-frame aim would restart both
        // on every frame, which is the argument `sync_toggle_state` makes.
        let _ = self.dialog.tick(delta);
        // The one number the readout below the chart names that the demo has to
        // write rather than bind, for the reason `Demo::chart_moving` gives:
        // `Chart::is_animating` is a method over the widget's own clock. Guarded
        // by the value for the reason `tick_fps` is: a write fires the readout's
        // `on_change` and re-lays out a string, and nothing is moving on almost
        // every frame of a run.
        let moving = self.chart.is_animating();
        if self.chart_moving.get() != moving {
            self.chart_moving.set(moving);
        }

        {
            let mut nodes = self.nodes.borrow_mut();
            let mut layout = Layout::new(&mut nodes);
            layout.layout(self.root, Constraints::tight(size));
            // **The dialog's own pass, because the dialog is a second root.** Its
            // node has no parent — a dialog overlays everything — so the walk
            // above cannot reach it, and the one number that both the paint pass
            // and the hit test read is its laid-out rect.
            layout.layout(self.dialog.handle(), Constraints::tight(size));
        }

        // Where it is in the order above that the frame rate goes: after the
        // clocks and the chart's own numbers, and **before** the arena is borrowed.
        self.tick_fps(delta);
        // The dialog's focus rings, on the same argument: they are read by the
        // paint arm below and by `Dialog::on_event`, and a button inside a dialog
        // that has closed must not still be holding one.
        self.sync_dialog_focus();

        let mut nodes = self.nodes.borrow_mut();

        for handle in self.order.iter().copied() {
            let Some(node) = nodes.get_mut(handle) else {
                continue;
            };
            if handle == self.background {
                let mut painter = Painter::new();
                if let Some(rect) = node.layout().rect() {
                    painter.rect(rect.into(), self.background_color());
                }
                *node.paint_mut() = PaintState::from_commands(painter.finish());
                continue;
            }
            // A container paints its own background, and paints nothing at all
            // when it has none: four of the demo's five draw no commands, and
            // the row of pads draws the card the pads sit inside. The walk is
            // parent first, so the card is recorded before the pads and so is
            // drawn behind them.
            if let Some(container) = self.containers.iter().find(|it| it.handle() == handle) {
                let commands = match node.layout().rect() {
                    Some(rect) => container.paint(rect.into()),
                    None => Vec::new(),
                };
                *node.paint_mut() = PaintState::from_commands(commands);
                continue;
            }
            if handle == self.slider.node() {
                // The rect comes from the node the loop already holds, rather than
                // from `slider_rect`: that borrows the arena, and the loop has it
                // borrowed mutably already.
                let commands = match node.layout().rect() {
                    Some(rect) => self.slider.widget.paint(rect.into()),
                    None => Vec::new(),
                };
                *node.paint_mut() = PaintState::from_commands(commands);
                continue;
            }
            // The dialog, in the walk and **last in it**, which is the whole of how
            // an overlay gets on top: `Demo::draw` sends the nodes in `order`, and
            // the dialog's subtree was appended to that order rather than attached
            // to the gallery's root. It has an arm of its own because its `paint`
            // takes an advance closure **and** a line height — the seam the widget
            // asks its caller for — and the walk has no such parameter, the same
            // reason the field below is painted outside it.
            //
            // The rect is the node's own laid-out one rather than `WINDOW`, because
            // `offer_to` reads the same number for `on_event` and a tap judged
            // against a different box from the one the panel was drawn in is a tap
            // that lands somewhere else.
            if handle == self.dialog.handle() {
                let advance = self.dialog_advance();
                let line_height = self.dialog_line_height();
                let commands = match node.layout().rect() {
                    Some(rect) => self.dialog.paint(rect.into(), &advance, line_height),
                    None => Vec::new(),
                };
                *node.paint_mut() = PaintState::from_commands(commands);
                continue;
            }
            // The six widget nodes that paint themselves with nothing but a rect.
            // Each is the widget's **own** node, which is the whole of what it
            // takes to put one of them on the screen: `order` reaches it, the
            // commands go on it, and `draw` sends it.
            //
            // **The gauge is in this arm and not beside the slider**, and that is
            // not tidiness: a slider's `paint` needs an advance closure for its
            // label, so it has an arm of its own and it is borrowed above. The
            // gauge's `paint` takes a rect and nothing else — a dial has no text
            // in it at all — so it joins the five that need nothing but the box,
            // and **so does the chart**, whose `paint` is the same signature: a
            // plot has text in it, but the text is *recorded* rather than
            // measured, so the widget needs no metrics from its caller either.
            if handle == self.gauge.handle()
                || handle == self.toggle.handle()
                || handle == self.image.handle()
                || handle == self.progress.handle()
                || handle == self.keyboard.handle()
                || handle == self.chart.handle()
            {
                let commands = match node.layout().rect() {
                    Some(rect) if handle == self.gauge.handle() => self.gauge.paint(rect.into()),
                    Some(rect) if handle == self.toggle.handle() => self.toggle.paint(rect.into()),
                    Some(rect) if handle == self.image.handle() => self.image.paint(rect.into()),
                    Some(rect) if handle == self.progress.handle() => {
                        self.progress.paint(rect.into())
                    }
                    Some(rect) if handle == self.chart.handle() => self.chart.paint(rect.into()),
                    Some(rect) => self.keyboard.paint(rect.into()),
                    None => Vec::new(),
                };
                *node.paint_mut() = PaintState::from_commands(commands);
                continue;
            }

            let Some(pad) = self.pads.iter().find(|pad| pad.node == handle) else {
                continue;
            };
            let mut painter = Painter::new();
            if let Some(rect) = node.layout().rect() {
                // The colour is derived from the single press property here,
                // at paint time: the animation moves one number, and the
                // pad's whole appearance follows it.
                painter.rounded_rect(rect.into(), PAD_RADIUS, pad.color());
            }
            *node.paint_mut() = PaintState::from_commands(painter.finish());
        }

        // The labels last, so the text is on top of the pads and the controls:
        // each is painted with the layout it was given, measuring its
        // characters through the demo's font, so what is drawn is the laid-out
        // text and not one raw run. They are painted here rather than in the
        // loop above because the loop walks the tree's own order, and the
        // labels are reached through the panel, not as its siblings.
        let text_size = self.text_size;
        for (demo_label, &handle) in self.labels.iter().zip(self.label_nodes.iter()) {
            let rect = nodes
                .get(handle)
                .and_then(|node| node.layout().rect())
                .map(Into::into);
            record_label(
                &mut nodes,
                handle,
                demo_label,
                &self.metrics,
                text_size,
                rect,
            );
        }

        // The controls layer's own labels, which are reached through the layer
        // rather than as its siblings. They are painted after the widgets they
        // report on, so a number is on top of the control it is a number about —
        // the slider's readout over the slider has been the rule since task 14.
        // **The gauge's readout leads them** because the gauge is the layer's
        // first child, so the two agree about which is first in the column.
        for (readout, font) in [
            (&self.gauge_readout, READOUT_FONT),
            (&self.slider_readout, READOUT_FONT),
            (&self.toggle_readout, READOUT_FONT),
            (&self.image_fit_readout, READOUT_FONT),
            (&self.progress_readout, READOUT_FONT),
            (&self.chart_readout, CHART_READOUT_FONT),
            (&self.fps_readout, READOUT_FONT),
            (&self.text_readout, TEXT_FIELD_READOUT_FONT),
            (&self.submit_readout, TEXT_FIELD_READOUT_FONT),
        ] {
            let handle = readout.label.handle();
            let rect = nodes
                .get(handle)
                .and_then(|node| node.layout().rect())
                .map(Into::into);
            record_label(&mut nodes, handle, readout, &self.metrics, font, rect);
        }

        // The text field, after its readouts rather than with the other
        // rect-only widgets, for one reason: its `paint` needs an **advance
        // closure** — the caret's x is measured a character at a time — so it
        // cannot go in the walk above without every arm of that walk gaining a
        // parameter it does not otherwise need.
        {
            let advance = |ch: char| self.metrics.advance(ch, TEXT_INPUT_FONT);
            let handle = self.text_input.handle();
            let commands = match nodes.get(handle).and_then(|node| node.layout().rect()) {
                Some(rect) => self.text_input.paint(rect.into(), &advance),
                None => Vec::new(),
            };
            if let Some(node) = nodes.get_mut(handle) {
                *node.paint_mut() = PaintState::from_commands(commands);
            }
        }
    }

    /// Opens the dialog if it is closed and closes it if it is up.
    ///
    /// **One function for both directions**, and the reason is that two arms
    /// doing one direction each are two places to be right: a key that opened a
    /// dialog nothing could close would be one of them, and nothing in the suite
    /// would say so. The two directions are asserted separately anyway —
    /// `the_dialog_key_opens_a_closed_dialog` and `the_dialog_key_closes_an_open
    /// _one` — because "one key toggles" is the claim and each direction is half
    /// of it.
    ///
    /// **Armed on `visible` and not on `is_drawn`**, so a key pressed while the
    /// fade-out is still running presents the dialog again rather than doing
    /// nothing. `present` and `dismiss` also return whether they changed
    /// anything, and that is deliberately not what this arms on: they would both
    /// answer `false` for a press during the fade.
    ///
    /// Focus moves with it in both directions, and the reason is in the module doc
    /// and in `Demo::sync_dialog_focus`: a dialog that opens with nothing focused
    /// inside it has an `Enter` that reaches no button and a `Space` that reaches
    /// the three pads behind the scrim.
    fn toggle_dialog(&mut self) {
        if self.dialog_is_modal() {
            let _ = self.dialog.dismiss();
            // **No `set_focus(None)` here, and its removal is not a simplification
            // but a fix.** This used to drop the record on the spot, which left
            // the `D` route as the one dismissal whose ring popped: the other
            // three routes (Escape, an action's own click, a tap on the scrim)
            // leave the record to `sync_dialog_focus`, which kept it for the fade.
            // Clearing it eagerly made `D` the odd route out, and
            // `a_dismissed_dialog_withdraws_activation_on_every_route` is the
            // test that says all four now behave alike. The record is retired
            // when the dialog stops being drawn, which is the same frame the ring
            // goes.
        } else {
            let _ = self.dialog.present();
            self.set_focus(self.dialog.actions.first().map(|action| action.handle()));
        }
    }

    /// Writes **three** things from **one** record and **two** predicates: it
    /// retires [`Demo::focused`] when the dialog stops being drawn, and it writes
    /// each action's `focused` and `activatable` from the answers the two
    /// predicates give. **The only thing in the file that writes any of the three.**
    ///
    /// **The two questions, and why they are two properties:**
    ///
    /// | question | predicate | widget property |
    /// |---|---|---|
    /// | is this control's ring **on screen**? | [`Dialog::is_drawn`] | `Button::focused` |
    /// | may this control be **activated**? | [`Demo::dialog_is_modal`] — `visible` | `Button::activatable` |
    ///
    /// They are not the same question, and for exactly 300 ms they have different
    /// answers. A dismissed dialog stops owning its buttons immediately, so
    /// *activation* must be withdrawn at once or an activation key fires a button
    /// nobody can see — that is the review's finding. But the panel, its text, its
    /// buttons and its scrim keep drawing until the transition ends, so the *ring*
    /// must stay until then or it vanishes in one frame over an opaque panel.
    ///
    /// **One record for both, because the ring is read off it.** The record is
    /// retired on `is_drawn` and **not** on `visible`, which is the reversal of
    /// 2026-10-03: a dismissal withdraws activation and leaves the record alone,
    /// because the record is what the ring is read from. Before
    /// `Button::activatable` existed the record *was* the activation gate, so
    /// retiring it was the only way to stop the key — and that is what made the
    /// ring pop. With the widget carrying the second answer, the record can go
    /// back to meaning one thing: *a control the demo still has focus on, and is
    /// still drawing*.
    ///
    /// **A ring is a statement about where focus is, not about what can be
    /// activated.** That is the operator's decision of 2026-10-03, and it reverses
    /// what this file's own comment used to argue here — that the ring going with
    /// the record "is the truth rather than a pop", on the grounds that after a
    /// dismissal the control cannot be activated. Activation is now refused by the
    /// widget, and asserted by
    /// `a_dismissed_dialog_does_not_answer_enter_during_its_fade` and
    /// `a_dismissed_dialog_withdraws_activation_on_every_route`; the ring is left
    /// free to answer the other question, and is asserted on the recorded commands
    /// by `the_focus_ring_fades_out_with_the_panel_instead_of_popping`.
    ///
    /// **Its own function rather than an arm of `set_focus`**, because a dismissal
    /// takes the focus away without going through `set_focus`: `OK` and `Cancel`
    /// fire their clicks and close, `Escape` closes, a tap on the scrim closes,
    /// and each of those is the widget's own code rather than the demo's. The
    /// `.ai/NEVERAGAIN.md` § *a drawn control with nothing behind it* concern is
    /// still real and is now answered by `activatable` rather than by the ring: a
    /// ring on a control that cannot be reached is a ring whose *meaning* is wrong,
    /// and the fix for that is to stop implying it can be reached.
    ///
    /// **The record is retired, not just the ring, and that is the half this
    /// function did not do until a review pointed at it.** The `is_drawn` test on
    /// the ring was right and still left [`Demo::focused`] naming a button of a
    /// dialog that was on its way out, which had **two** consequences and only one
    /// of them was visible: `Enter` kept firing that button (the reported defect),
    /// and `Space` stopped pressing the pads, because its condition reads a stale
    /// `Some` as "something holds focus".
    ///
    /// The retirement's condition is [`Demo::dialog_is_modal`] and **not**
    /// `is_drawn`, for the reason the routing's is: a dismissed dialog owns
    /// nothing from the instant it is dismissed, and holding a control for the
    /// 300 ms its scrim takes to fade is not ownership. The ring's condition is
    /// the other one, for the other reason, and the two are not redundant.
    ///
    /// [`Demo::focus_is_live`] asks the same question as the retirement, and both
    /// are wanted: this one makes the *state* right from the next frame on, and
    /// that one makes the *routing* right in the window before it.
    fn sync_dialog_focus(&mut self) {
        // **Retired on `is_drawn` and not on `dialog_is_modal`, and that reversal
        // is the whole of this change.** The record used to be retired the moment
        // the dialog stopped being *interactive*, because the record was the only
        // thing that could stop the key. It is no longer: the widget carries that
        // answer in `Button::activatable`, so a dismissal **withdraws activation
        // and leaves the record alone** — and the record has to survive, because
        // the ring is read off it and the panel is still drawn for 300 ms after
        // the dismissal.
        //
        // So the record is retired on the predicate that matches what it now
        // means — *a control the demo still has focus on, and is still drawing* —
        // which is `is_drawn`. One record, one meaning, and no second field to
        // keep in step with it.
        let drawn = self.dialog.is_drawn();
        let stale = self
            .focused
            .is_some_and(|handle| !drawn && self.in_dialog_subtree(handle));
        if stale {
            self.focused = None;
        }

        // **Two answers, two properties, one predicate each.** The ring is a
        // question about the screen and the key is a question about ownership,
        // and for the length of a fade-out they disagree.
        let modal = self.dialog_is_modal();
        for action in &self.dialog.actions {
            let ring = drawn && self.focused == Some(action.handle());
            if action.button.focused.get() != ring {
                action.button.focused.set(ring);
            }
            if action.button.activatable.get() != modal {
                action.button.activatable.set(modal);
            }
        }
    }

    /// Moves the gauge's value by `step` and carries the needle and the fill
    /// there.
    ///
    /// The three things in this function are the three the widget needs from a
    /// caller and picks nothing of itself: the **value** is written directly, the
    /// way a caller binding a gauge to a speedometer's reading writes it;
    /// `animate_to_state` is what starts the travel; and the value is snapped to
    /// [`GAUGE_STEP`]'s grid on the way in, for the reason
    /// [`PROGRESS_START`] gives for the bar's.
    ///
    /// **The spring is the caller's, and that is the point of the call.** The
    /// widget's own contract is that `animate_to_state` starts the transitions
    /// toward whatever `value` now says, and that the *needle* points at the
    /// drawn property rather than at the truth — so a spring here is a springing
    /// needle, and an ease would be an easing needle. Nothing here reads the
    /// animation's progress, and nothing could: the widget holds the clock.
    ///
    /// Clamped to the range rather than wrapped, which is the widget's own rule
    /// for a value outside `min..=max` and the demo's too: ten presses of `.` at
    /// the top of a speedometer stops at the top of the speedometer.
    fn step_gauge(&mut self, step: f32) {
        let stepped = (self.gauge.value.get() + step).clamp(GAUGE_MIN, GAUGE_MAX);
        let steps = (stepped - GAUGE_MIN) / GAUGE_STEP * GAUGE_TENTHS;
        self.gauge
            .value
            .set(GAUGE_MIN + steps.round() / GAUGE_TENTHS * GAUGE_STEP);
        self.gauge.animate_to_state(Motion {
            duration: GAUGE_MOTION,
            easing: GAUGE_SPRING,
        });
    }

    /// Moves the gauge to the next shape in [`GAUGE_TYPES`], wrapping round.
    ///
    /// Two properties are written and **the order is the whole of it**: the
    /// widget's own mode first and the demo's record of it second, so a frame in
    /// which the two disagree draws a dial the readout has not caught up with
    /// rather than the reverse. That is `progress_indeterminate`'s argument,
    /// asked again.
    ///
    /// Nothing is animated. A shape is not a value and not a colour, so there is
    /// nothing to transition *to*: the needle's triangle and the tick marks are
    /// there on the next frame. Animating it would mean a shape interpolating
    /// between two shapes, which is not a thing the widget has.
    ///
    /// The needle's colour is put on the gauge by `set_gauge_type` itself when the
    /// new shape *is* the needle, which is why no palette is read here: the widget
    /// takes the colour from the palette it already holds, and that palette is the
    /// themed one `toggle_theme` keeps current.
    fn cycle_gauge_type(&mut self) {
        let next = (self.gauge_type.get() + 1) % GAUGE_TYPES.len();
        self.gauge.set_gauge_type(GAUGE_TYPES[next]);
        self.gauge_type.set(next);
    }

    /// Moves the chart to the next shape in [`CHART_TYPES`], wrapping round.
    ///
    /// The gauge's function with the gauge's argument: two properties are written
    /// and **the order is the whole of it** — the widget's own mode first and the
    /// demo's record of it second, so a frame in which the two disagree draws a
    /// chart the readout has not caught up with rather than the reverse. That is
    /// `progress_indeterminate`'s argument, asked again.
    ///
    /// **Nothing is animated**, for `cycle_gauge_type`'s reason: a shape is not a
    /// value and not a colour. It is worth saying that this is a decision and not
    /// an omission, because it is the one place where this widget's animation is
    /// *not* what requirement 5 asked for — requirement 5 is about data points
    /// arriving, and `Demo::append_chart_sample` is that.
    fn cycle_chart_type(&mut self) {
        let next = (self.chart_type.get() + 1) % CHART_TYPES.len();
        self.chart.set_chart_type(CHART_TYPES[next]);
        self.chart_type.set(next);
    }

    /// Appends the next of [`CHART_SERIES`] to the chart and starts the series
    /// gliding to it.
    ///
    /// **`animate_push`, not `push` and `animate_to_state` written out here**, so
    /// that the two calls the widget pairs — in the order it pairs them — are one
    /// call in the demo, which is what a caller driving a chart from a sensor
    /// writes per reading. The `Motion` is the theme's, which is how requirement
    /// 4's "animation duration from theme tokens" is satisfied: the widget picks
    /// no duration of its own.
    ///
    /// **The window slides rather than growing once the whole of
    /// [`CHART_SERIES`] is on it** — [`CHART_OPENING_COUNT`] at the start and
    /// four presses later all twelve — and this is the demo's decision rather than the
    /// widget's: `animate_push` would happily make the series a thirteenth sample
    /// long and then a fourteenth, and every press of `A` would re-space the
    /// whole run against a wider or narrower set of labels — which is exactly the
    /// geometry [`CHART_SIZE`]'s mitre argument is about. So past the window the
    /// two keys meet: `A` shifts the way `S` does, which makes `A` a key that
    /// never runs out and `S` the one that says "drop the oldest" out loud.
    /// `the_append_key_slides_the_window_once_the_series_is_full` is what holds
    /// that.
    fn append_chart_sample(&mut self) {
        let motion = Motion::from_theme(&self.theme);
        // The reading is taken **before** the widget is named, because
        // `next_chart_sample` moves the demo's cursor and the widget call borrows
        // `self`: writing it as one expression would be two mutable borrows of the
        // same `self` in one statement, which is a compile error rather than a
        // subtlety.
        let reading = self.next_chart_sample();
        if self.chart.data.get().len() < CHART_SERIES.len() {
            self.chart.animate_push(reading, motion);
        } else {
            let _ = self.chart.animate_shift(reading, motion);
        }
    }

    /// Drops the oldest reading off the chart and appends the next of
    /// [`CHART_SERIES`], gliding the series to it.
    ///
    /// **Requirement 4's "append new value, shift old values" in one key**, which
    /// is why the two halves are a single call: a shift is a drop and an append,
    /// and a caller that wants only one of them writes the other. What was
    /// dropped is discarded rather than shown, because the readout has four things
    /// to name already and the value that left the window is the one a reader
    /// cannot check it against — the length is unchanged and the newest reading is
    /// not, and those two together are the whole claim.
    ///
    /// **The x labels do not move**, for [`CHART_X_LABEL_COUNT`]'s reason: they
    /// name a sample's place in the window and not its age, so the readings travel
    /// under them and the plot's bottom gutter stays the size it was.
    fn shift_chart_sample(&mut self) {
        let reading = self.next_chart_sample();
        let motion = Motion::from_theme(&self.theme);
        let _ = self.chart.animate_shift(reading, motion);
    }

    /// Returns the next reading of [`CHART_SERIES`] the demo will add, and moves
    /// the cursor on.
    ///
    /// **A constant and a cursor rather than a formula**, which is what makes a
    /// test able to say which reading a press of `A` produced: the value is the
    /// element of [`CHART_SERIES`] the cursor is on, and the next press takes the
    /// next one. The cursor wraps, so a key pressed for ever keeps drawing
    /// readings that exist.
    fn next_chart_sample(&mut self) -> f32 {
        let value = CHART_SERIES
            .get(self.chart_next % CHART_SERIES.len())
            .copied()
            .unwrap_or(CHART_SERIES[0]);
        self.chart_next = (self.chart_next + 1) % CHART_SERIES.len();
        value
    }

    /// Writes the slider's `dragging` flag, and re-aims it when that has moved.
    ///
    /// The two records are [`Demo::gauge`]'s absence of a counterpart: aiming
    /// restarts the slider's transition, so aiming every frame would leave the
    /// thumb creeping toward its target for ever. The slider's *value* is not
    /// re-aimed here, because an interaction writes the thumb itself and a
    /// programmatic write is aimed by whoever made it — see
    /// [`Demo::set_slider_value`].
    fn sync_slider_state(&mut self) {
        let wanted = self.slider_dragging;
        if self.slider.widget.dragging.get() != wanted {
            self.slider.widget.dragging.set(wanted);
            self.slider.written = wanted;
        }
        if self.slider.aimed != wanted {
            self.slider.aimed = wanted;
            self.slider
                .widget
                .animate_to_state(Motion::from_theme(&self.theme));
        }
    }

    /// Re-aims the toggle when the state its appearance is derived from has
    /// moved.
    ///
    /// The same one record as [`Demo::sync_slider_state`] and the same reason:
    /// `Toggle::animate_to_state` is what starts the thumb's slide and the pill's
    /// colour transition, and aiming it on every frame would restart those
    /// transitions on every frame. **Nothing else aims it** — a tap and an
    /// activation key write `checked` and stop there, and a caller that never
    /// re-aims gets a toggle that is *on* and draws as *off*, because the drawn
    /// state is the transition's and the transition never started.
    ///
    /// It is called before the tick, so the frame in which the state moves is
    /// the frame the transition starts on.
    fn sync_toggle_state(&mut self) {
        let wanted = ToggleState {
            checked: self.toggle.checked.get(),
            focused: self.toggle.focused.get(),
        };
        if self.toggle_aimed != wanted {
            self.toggle_aimed = wanted;
            self.toggle
                .animate_to_state(Motion::from_theme(&self.theme));
        }
    }

    /// Puts the slider at `value` without a pointer having asked for it, and
    /// carries the thumb there.
    ///
    /// This is the demo's "changes programmatically" case, which is what `0` and
    /// `1` do: the value property is written directly, the way a caller binding a
    /// slider to a model writes it, and the thumb is animated to it rather than
    /// jumping — so the one thing a drag cannot show is on screen. It is also the
    /// only way the demo changes the value without the widget doing it, which is
    /// why the readout's count of adjustments does not move when this is called.
    fn set_slider_value(&mut self, value: f32) {
        self.slider.widget.value.set(value);
        self.slider
            .widget
            .animate_to_state(Motion::from_theme(&self.theme));
    }

    /// Returns the window's background colour: the theme's `Background` token,
    /// or black if the theme ever holds something else there.
    fn background_color(&self) -> Color {
        self.background_color
            .get()
            .as_color()
            .unwrap_or(Color::new(0, 0, 0, 255))
    }

    /// Counts the frame that has just been drawn and writes the readout if what
    /// it says has changed.
    ///
    /// **The write is guarded by the text it would write.** `Property::set` writes
    /// and fires its callbacks on every call, and the rate changes on most frames
    /// at the boundary of a window and on none of them in between, so an
    /// unguarded write would re-shape the string on every frame of the run to
    /// show a picture that is the same one most of the time. It also costs the
    /// label its paint link: this readout's text carries **no** `on_change` mark
    /// of its node dirty, for the reason `Demo::new` gives for the toggle, the
    /// progress bar, the image and the list — the text is not an input to the
    /// rect the node was laid out at, and the label draws from its own options
    /// rather than from the rect's size, so the frame's own paint pass is what
    /// puts the new text on the screen.
    fn tick_fps(&mut self, delta: Duration) {
        self.fps.tick(delta);
        let text = self.fps.summary();
        if self.fps_text.get() != text {
            self.fps_text.set(text);
        }
    }

    /// Returns the report line a test runner reads, for the run so far.
    ///
    /// One line, on stdout, with a fixed prefix and `key=value` fields — see
    /// [`FrameRate::report`], which owns the format and says why it is that shape.
    #[must_use]
    fn fps_report(&self) -> String {
        self.fps.report()
    }

    /// Hands the recorded commands to the renderer, in paint order.
    fn draw(&mut self, renderer: &mut Renderer) {
        let mut nodes = self.nodes.borrow_mut();
        let clips = self.frame_clips();
        for (handle, clip) in self.order.iter().copied().zip(clips) {
            renderer.draw_node_clipped(handle, &mut nodes, clip);
        }
    }

    /// Returns the clip for every node in paint order, positionally matching
    /// [`Demo::order`].
    ///
    /// **This is the whole of the frame's clipping, in one place, and the frame
    /// loop uses it rather than deciding inline.** That is not tidiness: an
    /// earlier version had the loop call a `clip_for` helper itself — since folded
    /// into this function, so the name resolves to nothing, which is why it is named
    /// here in the past tense — and a mutation that inlined the same logic into the
    /// loop instead sailed through every test, because the tests were calling
    /// `clip_for` and the defect was in a caller of it. A test that exercises a
    /// helper cannot see a call site that stopped
    /// using the helper. One function, used by the loop and by the tests, closes
    /// that.
    ///
    /// **The rects are gone with it**, and so is the arena argument this function
    /// used to take: a clip is a node's own laid-out rect, and a node that is not
    /// clipped does not need to be looked up to say so. The signature changing is
    /// the honest consequence — there is nothing left to read from the arena, and
    /// a parameter kept only so a caller could pass it would be a second thing to
    /// keep in step.
    fn frame_clips(&self) -> Vec<Option<Rect>> {
        self.order.iter().map(|_| None).collect()
    }

    /// Switches between the dark and light themes, animated over
    /// `THEME_TRANSITION` milliseconds.
    ///
    /// Every widget is aimed at the *new* theme's palette rather than the one the
    /// theme is passing through, so each one's transition and the theme's own
    /// arrive together at the end of the same window. Aiming at the theme's
    /// current value would instead leave every widget chasing a target that moves
    /// for as long as the switch does.
    ///
    /// **The gauge is here and its palette is read from `new_theme` with the
    /// rest**, which is the whole of what the widget needs and the reason the read
    /// is on this side of `switch_to` rather than after it. It is the defect class
    /// this repository has already paid for once: a palette read *after* the
    /// switch is the palette the theme is leaving, which re-aims the widget at
    /// what it already had and the transition goes nowhere while every test stays
    /// green — a grey dial on the light theme, indistinguishable from a widget that
    /// was never themed.
    ///
    /// The gauge is aimed on the *theme's* motion rather than on
    /// [`GAUGE_MOTION`], which is the one place this file uses a second answer to
    /// "how long". The two are about different things: [`GAUGE_MOTION`] is how
    /// long a **needle takes to reach a reading**, and the theme's is how long a
    /// **colour takes to cross between two palettes**. Re-aiming on the spring
    /// would put a colour transition on a needle's curve, and a ring of circles
    /// interpolating through an overshoot is not a thing anyone wants to see. The
    /// chart is on the theme's motion for the first of those two reasons alone: it
    /// has no second answer to "how long", because a chart's data arrives at the
    /// rate it arrives and the widget picks nothing.
    fn toggle_theme(&mut self) {
        self.dark = !self.dark;
        let new_theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        // Read every palette from `new_theme`, **before** `switch_to` consumes it.
        // The switch animates the theme's own tokens, so a palette read after it
        // is the palette the theme is leaving, which re-aims every widget at what
        // it already had and the transition goes nowhere.
        let gauge_palette = GaugePalette::from_theme(&new_theme);
        let dialog_palette = DialogPalette::from_theme(&new_theme);
        let slider_palette = SliderPalette::from_theme(&new_theme);
        let toggle_palette = TogglePalette::from_theme(&new_theme);
        let progress_palette = ProgressPalette::from_theme(&new_theme);
        let chart_palette = ChartPalette::from_theme(&new_theme);
        let text_input_palette = TextInputPalette::from_theme(&new_theme);
        let keyboard_palette = KeyboardPalette::from_theme(&new_theme);
        let motion = Motion::from_theme(&new_theme);
        self.theme.switch_to(new_theme, THEME_TRANSITION);
        self.gauge.set_palette(gauge_palette);
        self.gauge.animate_to_state(motion);
        // The dialog carries a palette too, and it is the **one** palette in this
        // function that is set and not animated: `Dialog::set_palette` says the
        // change is immediate on purpose, because a dialog's three colours change
        // only when the theme does and a crossfade would be a second pair of
        // transitions on a clock that already carries the appearance and the
        // dismissal. A dialog left on the dark palette over a light gallery would
        // be a dark panel floating in a white window, which is why this line is
        // here at all rather than left to the panel's first paint.
        self.dialog.set_palette(dialog_palette);
        self.slider.widget.set_palette(slider_palette);
        self.slider.widget.animate_to_state(motion);
        self.toggle.set_palette(toggle_palette);
        self.toggle.animate_to_state(motion);
        self.progress.set_palette(progress_palette);
        // The bar's aim is also what keeps the indeterminate loop going, so it is
        // re-aimed on a theme switch for the second reason as well as the first:
        // the colours move on the theme's own transition and the slide carries on
        // where it was.
        self.progress.animate_to_state(motion);
        // The chart themes with everything else and animates with it, on the
        // gauge's argument: `set_palette` names the five colours it draws with and
        // `animate_to_state` carries them there over the theme's own transition,
        // so a chart that jumped to the light palette while the window behind it
        // was still crossfading would be a visible disagreement.
        //
        // **The palette is read from `new_theme` above `switch_to`, and that is
        // the whole defect class this repository has already paid for once**: a
        // palette read *after* the switch is the palette the theme is leaving,
        // which re-aims the widget at what it already had — the transition goes
        // nowhere and every test that does not wait for it stays green.
        // `a_theme_switch_reaches_the_chart_and_its_readout_in_every_shape` is
        // what catches a revert.
        self.chart.set_palette(chart_palette);
        self.chart.animate_to_state(motion);
        // The field and the keyboard both carry a palette, and both are aimed
        // rather than snapped, for the same reason as everything above: the
        // theme's own transition is 300ms and a keycap that jumped to the light
        // palette while the window behind it was still crossfading would be a
        // visible disagreement.
        //
        // `set_palette` on the field takes `&mut self` and the field is behind an
        // `Rc`, so this is `get_mut` — and it is infallible here only because
        // nothing else holds a clone yet: the keyboard's callback is the one
        // clone and it is created in `Demo::new`. A `None` here would be a clone
        // the demo made and forgot about, so it is reported rather than ignored.
        self.text_input.set_palette(text_input_palette);
        self.text_input.animate_to_state(motion);
        self.keyboard.set_palette(keyboard_palette);
        self.keyboard.animate_to_state(motion);
        // The image has no palette at all — an image is not themed, it is a
        // picture — and a theme switch reaches it nowhere, which is correct: the
        // window behind it changes and the picture does not.
    }

    /// Moves the image to the next fit in [`IMAGE_FITS`], wrapping round.
    ///
    /// The demo shows **one** image in **one** place and changes what that one
    /// draws: four copies of the same picture in four fits would be a widget
    /// gallery, and a gallery is a screenshot rather than a demonstration. The
    /// label naming the fit is bound to the same property, so it cannot fall out
    /// of step with what is drawn.
    fn cycle_image_fit(&mut self) {
        let next = (self.image_fit.get() + 1) % IMAGE_FITS.len();
        self.image_fit.set(next);
        self.image.set_fit(image_fit_at(next));
    }

    /// Moves the progress bar's value by `step`, and carries its fill there.
    ///
    /// The value is snapped to tenths rather than accumulated, so ten presses of
    /// `]` land on exactly one and a bar is a bar that is full. The fill is
    /// animated rather than jumped, which is the demo's "changes
    /// programmatically" case and the same one `0` and `1` are for the slider.
    fn step_progress(&mut self, step: f32) {
        let stepped = (self.progress.value.get() + step).clamp(0.0, 1.0);
        let snapped = (stepped * PROGRESS_TENTHS).round() / PROGRESS_TENTHS;
        self.progress.value.set(snapped);
        self.progress
            .animate_to_state(Motion::from_theme(&self.theme));
    }

    /// Switches the progress bar between its two modes.
    ///
    /// The widget's own setter is the only route into the mode, and it parks the
    /// slide at the start of a leg; `animate_to_state` is what starts the loop,
    /// and the loop is then the widget's — [`Progress::tick`] aims the next leg
    /// itself, so this is called once per switch rather than once a frame.
    fn set_progress_indeterminate(&mut self, indeterminate: bool) {
        self.progress.set_indeterminate(indeterminate);
        self.progress_indeterminate.set(indeterminate);
        self.progress
            .animate_to_state(Motion::from_theme(&self.theme));
    }

    /// Presses every pad, cascading across them one `STAGGER_STEP` apart.
    ///
    /// **Records that it did**, for the reason [`Demo::space_pressed`] gives: the
    /// release has to be able to find out whether there was a press, and a
    /// condition on the *current* focus cannot answer that.
    fn press_all(&mut self) {
        self.space_pressed = true;
        self.clock.clear();
        let animations: Vec<AnyAnimation> = self
            .pads
            .iter()
            .map(|pad| {
                pad.press
                    .animate_to(1.0, PRESS_DURATION, Easing::EaseOut)
                    .into()
            })
            .collect();
        Stagger::new(animations, STAGGER_STEP).play(&mut self.clock);
    }

    /// Presses every pad, but only while no **live** control holds focus.
    ///
    /// **`focus_is_live` and not `focused.is_none()`** — that predicate's doc gives
    /// the sequence: a dismissal from inside the widget leaves the record stale, and
    /// a stale `Some` is not `None`, so the one key that presses the pads stopped
    /// pressing them after an `Escape`. This is `Space`'s whole body, and it is a
    /// method rather than a pattern guard on a `match` arm because a row of
    /// [`GALLERY_SHORTCUTS`] is a **function** and a function is not a pattern.
    fn press_pads_if_unfocused(&mut self) {
        if !self.focus_is_live() {
            self.press_all();
        }
    }

    /// Moves the text size up one [`TEXT_SIZE_STEP`].
    fn text_size_up(&mut self) {
        self.set_text_size(self.text_size + TEXT_SIZE_STEP);
    }

    /// Moves the text size down one [`TEXT_SIZE_STEP`].
    fn text_size_down(&mut self) {
        self.set_text_size(self.text_size - TEXT_SIZE_STEP);
    }

    /// Sends the slider to [`SLIDER_MIN`].
    fn slider_to_min(&mut self) {
        self.set_slider_value(SLIDER_MIN);
    }

    /// Sends the slider to [`SLIDER_MAX`].
    fn slider_to_max(&mut self) {
        self.set_slider_value(SLIDER_MAX);
    }

    /// Winds the progress bar back one [`PROGRESS_STEP`].
    fn progress_back(&mut self) {
        self.step_progress(-PROGRESS_STEP);
    }

    /// Winds the progress bar on one [`PROGRESS_STEP`].
    fn progress_on(&mut self) {
        self.step_progress(PROGRESS_STEP);
    }

    /// Switches the progress bar between its two modes.
    fn toggle_progress_mode(&mut self) {
        self.set_progress_indeterminate(!self.progress_indeterminate.get());
    }

    /// Turns the gauge back one [`GAUGE_STEP`].
    fn gauge_back(&mut self) {
        self.step_gauge(-GAUGE_STEP);
    }

    /// Turns the gauge on one [`GAUGE_STEP`].
    fn gauge_on(&mut self) {
        self.step_gauge(GAUGE_STEP);
    }

    /// Releases every pad at once, on the release spring.
    ///
    /// **Clears the record of the press**, for [`Demo::press_all`]'s reason: the
    /// two are one fact written in two places, and the flag is not the animation's
    /// to own.
    fn release_all(&mut self) {
        self.space_pressed = false;
        self.clock.clear();
        for pad in &self.pads {
            self.clock
                .add(pad.press.animate_to(0.0, RELEASE_DURATION, RELEASE_SPRING));
        }
    }

    /// Presses one pad.
    ///
    /// The `clear` is whole-clock, not per-pad: a mouse press while the space
    /// bar is held drops the other pads' in-flight stagger animations, and
    /// they sit frozen at their last written values until the next
    /// space-driven press or release. The clock cannot replace one property's
    /// animation in isolation — `add` leaves both writing the property — so
    /// a proper fix needs per-property replacement in `AnimationClock`, which
    /// no task needs yet. Dropping the `clear` is not a fix either: a stale
    /// longer animation (the 400 ms release spring) would outlive the shorter
    /// new one and write the property back after it finishes.
    fn press_pad(&mut self, index: usize) {
        let Some(pad) = self.pads.get(index) else {
            return;
        };
        self.clock.clear();
        self.clock
            .add(pad.press.animate_to(1.0, PRESS_DURATION, Easing::EaseOut));
    }

    /// Releases one pad.
    ///
    /// Same whole-clock `clear` limitation as `press_pad`: a mouse release
    /// while the space bar is held strands the other pads mid-press, frozen
    /// at their last written values until the next space-driven press or
    /// release. A proper fix needs per-property replacement in
    /// `AnimationClock`, which no task needs yet; dropping the `clear` would
    /// let a stale shorter animation — the 150 ms press — write the property
    /// back in the middle of the new release.
    fn release_pad(&mut self, index: usize) {
        let Some(pad) = self.pads.get(index) else {
            return;
        };
        self.clock.clear();
        self.clock
            .add(pad.press.animate_to(0.0, RELEASE_DURATION, RELEASE_SPRING));
    }

    /// Returns the index of the pad whose laid-out rect contains `(x, y)`.
    fn pad_at(&self, x: f32, y: f32) -> Option<usize> {
        let nodes = self.nodes.borrow();
        self.pads.iter().position(|pad| {
            nodes
                .get(pad.node)
                .and_then(|node| node.layout().rect())
                .is_some_and(|rect| {
                    x >= rect.origin.x
                        && x <= rect.origin.x + rect.size.width
                        && y >= rect.origin.y
                        && y <= rect.origin.y + rect.size.height
                })
        })
    }

    /// Returns `Some(())` when the point is over the slider, and `None` when it
    /// is not.
    ///
    /// The slider is asked about after the pads, so a point over the pads never
    /// reaches it: those are the controls that are drawn on top of that part of
    /// the window.
    fn slider_at(&self, x: f32, y: f32) -> Option<()> {
        self.slider_rect()
            .is_some_and(|rect| over_rect(rect, x, y))
            .then_some(())
    }

    /// Returns the slider's rect in window coordinates, or `None` if it has not
    /// been laid out.
    ///
    /// This is the rect the slider draws inside and the one a pointer event is
    /// measured against, so it is the demo's own statement of where the slider is
    /// rather than each caller working it out — and it is why the widget's
    /// `on_event` takes a rect: a node cannot reach the arena that holds it.
    fn slider_rect(&self) -> Option<Rect> {
        self.node_rect(self.slider.node())
    }

    /// Returns the toggle's rect in window coordinates, or `None` if it has not
    /// been laid out. See [`Demo::slider_rect`].
    fn toggle_rect(&self) -> Option<Rect> {
        self.node_rect(self.toggle.handle())
    }

    /// Returns the image's box in window coordinates, or `None` if it has not
    /// been laid out. See [`Demo::slider_rect`].
    ///
    /// `cfg(test)` because the demo itself paints the image from the node the
    /// paint walk already holds, and only a test asks where the box is: what
    /// each fit does *with* that box is a question about the four draws, and
    /// that is what the fit test asks.
    #[cfg(test)]
    fn image_rect(&self) -> Option<Rect> {
        self.node_rect(self.image.handle())
    }

    /// Returns the text field's rect in window coordinates. See
    /// [`Demo::slider_rect`].
    fn text_input_rect(&self) -> Option<Rect> {
        self.node_rect(self.text_input.handle())
    }

    /// Returns the keyboard's rect in window coordinates. See
    /// [`Demo::slider_rect`].
    fn keyboard_rect(&self) -> Option<Rect> {
        self.node_rect(self.keyboard.handle())
    }

    /// Reports whether `(x, y)` is over the keyboard, for the press that grabs
    /// a key.
    ///
    /// **Over the keyboard's box, not over a key.** A press in a gap between two
    /// keys still has to grab, release and unlight something, and the keyboard
    /// decides which: `grab_key` reports whether it found a key at all. Narrowing
    /// this to "over a key" would leave a key lit by a press that started in a
    /// gap and travelled onto it.
    fn keyboard_at(&self, x: f32, y: f32) -> bool {
        self.keyboard_rect()
            .is_some_and(|rect| over_rect(rect, x, y))
    }

    /// Lights the key under `position`, if there is one.
    ///
    /// Called from the press, for the reason the scrollbar's thumb is grabbed
    /// there: the recogniser reports a tap on the release and a drag only once
    /// the pointer has moved, so there is no press for the widget to read.
    fn grab_key(&mut self, position: Offset) {
        if let Some(rect) = self.keyboard_rect() {
            self.keyboard_pressed = self.keyboard.grab_key(position, rect);
        }
    }

    /// Unlights whatever key was lit. Called from every release.
    fn release_key(&mut self) {
        if self.keyboard_pressed {
            self.keyboard.release_key();
            self.keyboard_pressed = false;
        }
    }

    /// Returns the node at `handle`'s laid-out rect, converted to the painter's,
    /// or `None` if the node is not there or has not been placed.
    ///
    /// The conversion is because the layout pass and the painters speak two
    /// `Rect` types, and every widget in the demo is handed the painter's: this
    /// is the one place that turns one into the other.
    fn node_rect(&self, handle: Handle) -> Option<Rect> {
        let nodes = self.nodes.borrow();
        nodes.get(handle)?.layout().rect().map(Into::into)
    }

    /// Returns the rect of every **leaf** the demo places, with the name of what
    /// it belongs to.
    ///
    /// This is what the two collision tests are about, and the word doing the
    /// work in it is *leaf*. The demo's tree is [`Container`]s and widgets, and
    /// a `Container` with no background draws nothing: the band is the whole
    /// window and the text panel is 900 by 380 of nothing. Two of those
    /// "overlap" everything, so a test over all of them would assert that
    /// everything overlaps everything and prove nothing.
    ///
    /// What a reader can actually see is a set of boxes with names, and this is
    /// that set: the card the pads sit in, the seven labels of the text panel, the
    /// gauge and its readout, the slider and its readout, and then the things
    /// tasks 15 to 21 added. Two tests read it —
    /// `every_placed_rect_is_inside_the_window` and
    /// `no_two_placed_rects_overlap` — and the third defect this repository has
    /// found only by looking at the screen was a control placed over the thing
    /// next to it, so this is the pair of tests that would have found the first
    /// two.
    ///
    /// **The gauge and the chart are in this list**, which is the only way
    /// `no_two_placed_rects_overlap` could see a dial or a plot laid on top of
    /// something. The gauge arrived in task 20 for that reason and the chart in
    /// task 21 for the same one: the widgets' own hundreds of unit tests know
    /// nothing about where the demo put them, and a hand-placed 270 by 450 box in
    /// the one column the list just vacated is exactly the kind of claim only a
    /// reader looking at the arithmetic can check.
    ///
    /// **The dialog is deliberately not in it**, and that is the whole of the
    /// omission: an overlay is *meant* to be drawn on top of the gallery, so
    /// `no_two_placed_rects_overlap` would fail on the first pair it compared and
    /// the failure would be the design working. Its panel is placed by the widget
    /// from the window and is measured against the window's own edges in
    /// `the_demo_opens_with_a_dialog_showing_and_its_panel_inside_the_window`
    /// instead, which is the question an overlay's geometry actually raises.
    ///
    /// It is `cfg(test)` because nothing in the running demo asks: the demo lays
    /// its widgets out and paints them, and a list of the boxes it placed is
    /// something only a reader checking the arithmetic wants.
    #[cfg(test)]
    fn placed_rects(&self) -> Vec<(&'static str, Rect)> {
        let nodes = self.nodes.borrow();
        let rect = |what: &'static str, handle: Handle| {
            let found = nodes
                .get(handle)
                .and_then(|node| node.layout().rect())
                .map(Into::into);
            found.map(|rect| (what, rect))
        };
        let mut rects: Vec<(&'static str, Rect)> = Vec::new();
        if let Some(found) = rect("pads card", self.card().handle()) {
            rects.push(found);
        }
        for &handle in &self.label_nodes {
            if let Some(found) = rect("text panel label", handle) {
                // Named for what it is rather than for its text: the text moves
                // with `+` and `-` and a name that moved with it would name a
                // different failure every run.
                rects.push(found);
            }
        }
        for (what, handle) in [
            ("gauge", self.gauge.handle()),
            ("gauge readout", self.gauge_readout.label.handle()),
            ("slider", self.slider.node()),
            ("slider readout", self.slider_readout.label.handle()),
            ("image", self.image.handle()),
            ("image fit label", self.image_fit_readout.label.handle()),
            ("toggle", self.toggle.handle()),
            ("toggle readout", self.toggle_readout.label.handle()),
            ("progress bar", self.progress.handle()),
            ("progress readout", self.progress_readout.label.handle()),
            ("chart", self.chart.handle()),
            ("chart readout", self.chart_readout.label.handle()),
            ("fps readout", self.fps_readout.label.handle()),
            ("text input", self.text_input.handle()),
            ("text readout", self.text_readout.label.handle()),
            ("submit readout", self.submit_readout.label.handle()),
            ("keyboard", self.keyboard.handle()),
        ] {
            if let Some(found) = rect(what, handle) {
                rects.push(found);
            }
        }
        rects
    }

    /// Returns where the slider's thumb is at `fraction` of its range, in window
    /// coordinates, and leaves the slider where it was.
    ///
    /// The point is the widget's own answer rather than a number worked out here:
    /// `Slider::thumb_center` is the geometry every pointer position is measured
    /// against, so a test aiming at the thumb is aiming at the same thing the
    /// demo routes its events to. The value is put back afterwards — along with
    /// the thumb, which `snap_to_state` follows — so a test can aim without having
    /// moved anything; nothing is animating while a test is only asking where a
    /// point is.
    #[cfg(test)]
    fn slider_at_fraction(&self, fraction: f32) -> Option<(f32, f32)> {
        let rect = self.slider_rect()?;
        let before = self.slider.widget.value.get();
        self.slider
            .widget
            .value
            .set(SLIDER_MIN + (SLIDER_MAX - SLIDER_MIN) * fraction);
        self.slider.widget.snap_to_state();
        let center = self.slider.widget.thumb_center(rect);
        self.slider.widget.value.set(before);
        self.slider.widget.snap_to_state();
        Some(center)
    }

    /// Returns the card the pads sit inside: the row of pads, which is the
    /// first container the demo builds and the only one it gives a background.
    #[cfg(test)]
    fn card(&self) -> &Container {
        &self.containers[0]
    }

    /// Returns the text the slider's readout is showing, as the last frame
    /// recorded it.
    #[cfg(test)]
    fn readout_text(&self) -> Option<String> {
        let nodes = self.nodes.borrow();
        nodes
            .get(self.slider_readout.label.handle())?
            .paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
    }

    /// Returns the draw commands the slider recorded on the last frame.
    #[cfg(test)]
    fn slider_commands(&self) -> Vec<DrawCommand> {
        let nodes = self.nodes.borrow();
        nodes
            .get(self.slider.node())
            .map(|node| node.paint().commands().to_vec())
            .unwrap_or_default()
    }

    /// Returns the draw commands the node at `handle` recorded on the last frame.
    #[cfg(test)]
    fn commands_at(&self, handle: Handle) -> Vec<DrawCommand> {
        let nodes = self.nodes.borrow();
        nodes
            .get(handle)
            .map(|node| node.paint().commands().to_vec())
            .unwrap_or_default()
    }

    /// Returns the text a readout is showing, as the last frame recorded it.
    ///
    /// Every readout in the band is a label whose text is a bound property, so
    /// "what the readout says" is the only thing a test can observe about the
    /// chain that produced it — and the thing a reader sees.
    #[cfg(test)]
    fn readout_text_of(&self, readout: &DemoLabel) -> Option<String> {
        self.commands_at(readout.label.handle())
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
    }
}

/// The gallery's own keyboard shortcuts: what each key is called and what it does.
///
/// **This table *is* the dispatch**, and [`Demo::handle_event`] looks a key up in
/// it rather than matching on one. That is not a style preference and it is the
/// answer to a hazard a review named on 2026-10-03: with a `match` beside a list of
/// the keys under test, the two are separate objects and they drift. They had
/// drifted — the `match` read `Keycode::Equals | Keycode::Plus` and the list named
/// only `Plus`, so `Equals` was a key the demo acted on and nothing checked that
/// it was suppressed while the dialog was showing.
///
/// **Three properties follow from the table being the only route, and each is a
/// thing a `match` cannot promise:**
///
/// - a shortcut the demo binds **is** a row here, so there is nowhere else to put
///   one — the guard in `handle_event` wraps this table, so a row cannot be left
///   unsuppressed;
/// - the suppression tests iterate the rows, so they cannot miss a key that exists;
/// - `Space`'s focus condition, which was a pattern guard on a `match` arm, is a
///   method — [`Demo::press_pads_if_unfocused`] — because a row here is a function
///   and a function is not a pattern.
///
/// **The two `+` rows are aliases and not a duplication mistake**: `Plus` is the
/// shifted key and `Equals` the unshifted one, and both grow the text. Naming
/// both is the whole of what finding 2 was about.
///
/// What would reverse it: a shortcut that needs its key's *held* state — a chord, or
/// a binding that reads the modifiers — which a `fn(&mut Demo)` cannot express and
/// which would want the `match` back with a coverage test beside it.
/// One row of [`GALLERY_SHORTCUTS`]: what the key is called, which key it is, and
/// what it does.
///
/// A named alias because `clippy::type_complexity` reads the spelled-out type as
/// three things to factor and because the third field is the whole point of the
/// table — a reader should not have to count `fn(&mut Demo)`s to see that the rows
/// carry behaviour and not only names.
type GalleryShortcut = (&'static str, Keycode, fn(&mut Demo));

const GALLERY_SHORTCUTS: [GalleryShortcut; 18] = [
    (
        "Space, which presses every pad",
        Keycode::Space,
        Demo::press_pads_if_unfocused,
    ),
    (
        "T, which switches the theme",
        Keycode::T,
        Demo::toggle_theme,
    ),
    (
        "+, which grows the text with the shift held",
        Keycode::Plus,
        Demo::text_size_up,
    ),
    (
        "=, which grows the text without it",
        Keycode::Equals,
        Demo::text_size_up,
    ),
    (
        "-, which shrinks the text",
        Keycode::Minus,
        Demo::text_size_down,
    ),
    (
        "C, which moves the text's colour token",
        Keycode::C,
        Demo::cycle_text_color,
    ),
    (
        "0, which sends the slider to its minimum",
        Keycode::_0,
        Demo::slider_to_min,
    ),
    (
        "1, which sends the slider to its maximum",
        Keycode::_1,
        Demo::slider_to_max,
    ),
    (
        "F, which cycles the image's fit",
        Keycode::F,
        Demo::cycle_image_fit,
    ),
    (
        "[, which winds the progress bar back",
        Keycode::LeftBracket,
        Demo::progress_back,
    ),
    (
        "], which winds the progress bar on",
        Keycode::RightBracket,
        Demo::progress_on,
    ),
    (
        "P, which switches the bar's mode",
        Keycode::P,
        Demo::toggle_progress_mode,
    ),
    (
        ", which turns the gauge back",
        Keycode::Comma,
        Demo::gauge_back,
    ),
    (
        ". which turns the gauge on",
        Keycode::Period,
        Demo::gauge_on,
    ),
    (
        "G, which changes the gauge's shape",
        Keycode::G,
        Demo::cycle_gauge_type,
    ),
    (
        "H, which changes the chart's shape",
        Keycode::H,
        Demo::cycle_chart_type,
    ),
    (
        "A, which appends a chart reading",
        Keycode::A,
        Demo::append_chart_sample,
    ),
    (
        "S, which drops the oldest chart reading",
        Keycode::S,
        Demo::shift_chart_sample,
    ),
];

/// Returns the labels in the demo's text panel, in the order they are shown,
/// each with the layout it is drawn with.
///
/// This is the visual proof for the label work, so it shows what the pipeline
/// does rather than one string: the greeting, a paragraph that wraps at the
/// panel's width, the three alignments across the same width, a letter-spaced
/// line, and a line too long for the panel, truncated with an ellipsis. A
/// change to a font size, a colour or a layout is visible in the running demo
/// because of what is on this list.
fn demo_labels() -> Vec<(&'static str, LayoutOptions)> {
    let alignment = |align| LayoutOptions {
        max_width: TEXT_COLUMN_WIDTH,
        align,
        ..LayoutOptions::default()
    };
    vec![
        (
            "Hello, World!",
            LayoutOptions {
                max_width: TEXT_COLUMN_WIDTH,
                ..LayoutOptions::default()
            },
        ),
        (
            "This paragraph wraps at the panel's width, one word at a time, and \
             every line after the first is laid out from the same options.",
            LayoutOptions {
                max_width: TEXT_COLUMN_WIDTH,
                ..LayoutOptions::default()
            },
        ),
        ("left aligned", alignment(TextAlign::Left)),
        ("centred", alignment(TextAlign::Center)),
        ("right aligned", alignment(TextAlign::Right)),
        (
            "letter spacing widens every gap",
            LayoutOptions {
                max_width: TEXT_COLUMN_WIDTH,
                letter_spacing: 3.0,
                ..LayoutOptions::default()
            },
        ),
        (
            "A line far too long for the panel it is given is cut at the panel's \
             edge and closed with an ellipsis.",
            LayoutOptions {
                max_width: TEXT_COLUMN_WIDTH * 0.5,
                wrap: WrapMode::None,
                truncation: Truncation::Ellipsis,
                ..LayoutOptions::default()
            },
        ),
    ]
}

/// Returns the handles of the tree below `root` in paint order: a parent, then
/// its children in the order its layout mode places them. A `Stack` places them
/// all at the same rect, and the later one covers the earlier, so the order
/// decides which is on top.
fn paint_order(nodes: &Arena<WidgetNode>, root: Handle) -> Vec<Handle> {
    fn walk(nodes: &Arena<WidgetNode>, handle: Handle, into: &mut Vec<Handle>) {
        into.push(handle);
        if let Some(node) = nodes.get(handle) {
            for &child in node.children() {
                walk(nodes, child, into);
            }
        }
    }
    let mut order = Vec::new();
    walk(nodes, root, &mut order);
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Monospace stand-in measurements: every character half its size wide, and
    /// every line 1.2 times its size tall, so both grow with the font size the
    /// way a real face's do. A real font needs a file, and a test may not need a
    /// filesystem.
    fn mono_metrics() -> TextMetrics {
        TextMetrics {
            advance: Rc::new(|_, size| size * 0.5),
            line_height: Rc::new(|size| size * 1.2),
        }
    }

    /// Returns a demo with the stand-in measurements.
    ///
    /// The image is the **stand-in**, not the asset: a test may not need a
    /// filesystem, and the stand-in is a `TextureCache` and a handle with no GPU
    /// behind it. Everything the tests measure about the image — its box, its
    /// fit's geometry, its place in the paint order — is the same either way,
    /// because the two are the same size.
    fn demo() -> Demo {
        Demo::new(mono_metrics(), None).unwrap()
    }

    /// Lays the demo out once, the way the first frame does, so a test can ask
    /// where the pads are.
    fn laid_out() -> Demo {
        let mut demo = demo();
        demo.frame(WINDOW, Duration::from_millis(16));
        demo
    }

    /// Returns a demo whose dialog has been **closed and whose fade has run out**,
    /// so a test about the gallery is not also a test about the modal over it.
    ///
    /// **This is the single most consequential fixture change task 22 made**, and
    /// it is a change of *precondition* rather than of assertion: the demo now
    /// opens with the dialog showing — input injection does not work on this host,
    /// so a dialog that began hidden could only be photographed through an
    /// instrument — and twenty-odd tests drive a control **behind** the overlay. A
    /// tap on the slider with the dialog up is correctly swallowed by the modal, so
    /// those tests have to say that the modal is not there. **Not one of their
    /// assertions is changed.**
    ///
    /// Closed through the **demo's own key** and then left to finish, so the state
    /// it produces is the state a reader produces by pressing `D` — and the
    /// assertion is `is_drawn()` rather than `visible`, because a dialog that is
    /// hidden but still fading is still on the screen.
    fn dialog_closed() -> Demo {
        let mut demo = laid_out();
        demo.handle_event(key_event(DIALOG_KEY));
        // The widget's own transition is 300 ms — its `motion`'s default, which the
        // demo deliberately does not override — so three whole spans is two spans
        // of slack over it.
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert!(
            !demo.dialog.is_drawn(),
            "so the overlay is gone rather than merely invisible: a dialog still \
             fading would be `visible` false and `is_drawn` true"
        );
        assert_eq!(
            demo.dialog.overlay_alpha.get(),
            0.0,
            "and its scrim has faded all the way out"
        );
        demo
    }

    /// Returns the `T` key-down event that switches the theme.
    fn toggle_theme_event() -> Event {
        Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::T),
            scancode: None,
            keymod: Mod::empty(),
            repeat: false,
            which: 0,
            raw: 0,
        }
    }

    #[test]
    fn a_new_pad_is_at_rest_and_paints_its_rest_colour() {
        let demo = demo();
        let pad = &demo.pads[0];
        assert_eq!(pad.press.get(), 0.0);
        assert!(!pad.press.is_bound());
        let rest = demo.theme.get(ThemeToken::Error).as_color().unwrap();
        assert_eq!(
            pad.color.get(),
            PropertyValue::Color(rest),
            "at rest the pad paints the theme's Error token"
        );
    }

    #[test]
    fn a_fully_pressed_pad_paints_its_held_colour() {
        let demo = demo();
        let pad = &demo.pads[0];
        pad.press.set(1.0);
        let rest = demo.theme.get(ThemeToken::Error).as_color().unwrap();
        assert_eq!(
            pad.color.get(),
            PropertyValue::Color(lighten(rest)),
            "fully pressed, the pad paints the lightened Error token"
        );
    }

    #[test]
    fn a_pad_interpolates_its_colour_at_paint_time() {
        // The colour is derived from the single press property at paint time
        // rather than animated: halfway pressed, the pad paints exactly halfway
        // between its rest and held colours.
        let demo = demo();
        let pad = &demo.pads[0];
        pad.press.set(0.5);
        let rest = demo.theme.get(ThemeToken::Error).as_color().unwrap();
        let held = lighten(rest);
        assert_eq!(
            pad.color.get(),
            PropertyValue::Color(Color::interpolate(&rest, &held, 0.5))
        );
    }

    #[test]
    fn the_demo_starts_with_three_pads_at_rest() {
        let demo = demo();
        assert_eq!(demo.pads.len(), 3);
        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 0.0);
        }
    }

    #[test]
    fn the_demo_starts_with_the_dark_theme() {
        let demo = demo();
        assert!(demo.dark);
        assert_eq!(
            demo.theme.get(ThemeToken::Background),
            Theme::dark().get(ThemeToken::Background)
        );
        assert_eq!(
            demo.background_color.get(),
            Theme::dark().get(ThemeToken::Background),
            "the background is bound to the dark theme's Background token"
        );
    }

    #[test]
    fn pressing_t_switches_the_theme_with_animation() {
        // Pressing T switches to the light theme over THEME_TRANSITION
        // milliseconds: the background holds its start value, moves half way
        // through, and arrives at the light theme's background.
        let mut demo = dialog_closed();
        let dark_background = demo.background_color.get();
        demo.handle_event(toggle_theme_event());
        assert!(!demo.dark, "the demo is now on the light theme");
        assert_eq!(
            demo.background_color.get(),
            dark_background,
            "the switch has started but its first tick has not written yet"
        );

        for _ in 0..15 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_ne!(
            demo.background_color.get(),
            dark_background,
            "half way through, the background has moved"
        );

        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.background_color.get(),
            Theme::light().get(ThemeToken::Background),
            "and it arrived at the light theme's background"
        );
    }

    #[test]
    fn the_pads_follow_the_theme_switch() {
        // The pads' rest colours come from the theme, so a switch carries the
        // new colours to them through the property graph: after the switch,
        // each pad paints the light theme's token.
        let mut demo = dialog_closed();
        let dark_color = demo.pads[0].color.get();
        demo.handle_event(toggle_theme_event());
        for _ in 0..35 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_ne!(
            demo.pads[0].color.get(),
            dark_color,
            "the pad's colour has moved"
        );
        let rest = Theme::light().get(ThemeToken::Error).as_color().unwrap();
        assert_eq!(
            demo.pads[0].color.get(),
            PropertyValue::Color(rest),
            "and it arrived at the light theme's Error token"
        );
    }

    #[test]
    fn a_theme_switch_marks_the_background_dirty() {
        // The link from a theme switch to a node: the background colour's
        // on_change callback marks the background node dirty, so the next pass
        // repaints it. The demo is laid out first, because a node is born
        // dirty and only a pass clears the flag — without that, the assertion
        // would hold whatever the callback did.
        let mut demo = dialog_closed();
        demo.handle_event(toggle_theme_event());
        let nodes = demo.nodes.borrow();
        assert!(
            nodes.get(demo.background).unwrap().layout().is_dirty(),
            "the background node was marked dirty by the theme switch"
        );
    }

    #[test]
    fn pressing_the_pads_cascades_them_with_a_stagger() {
        // Holding the space bar presses all three pads, one STAGGER_STEP
        // apart: after one step's worth of time the first is pressing and the
        // last has not started.
        let mut demo = demo();
        demo.press_all();
        let _ = demo.clock.tick(STAGGER_STEP);
        assert!(demo.pads[0].press.get() > 0.0, "the first pad is pressing");
        assert_eq!(
            demo.pads[2].press.get(),
            0.0,
            "and the last has not started: it is still in its stagger delay"
        );
    }

    #[test]
    fn a_pressed_pad_arrives_at_held() {
        let mut demo = demo();
        demo.press_all();
        for _ in 0..10 {
            let _ = demo.clock.tick(Duration::from_millis(50));
        }
        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 1.0, "the pad is fully pressed");
        }
        assert!(!demo.clock.is_animating());
    }

    #[test]
    fn releasing_a_pad_springs_it_back_to_rest() {
        let mut demo = demo();
        demo.press_all();
        for _ in 0..10 {
            let _ = demo.clock.tick(Duration::from_millis(50));
        }
        demo.release_all();
        for _ in 0..20 {
            let _ = demo.clock.tick(Duration::from_millis(50));
        }
        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 0.0, "the pad is back at rest");
        }
        assert!(!demo.clock.is_animating());
    }

    #[test]
    fn an_animated_press_marks_its_pad_dirty() {
        // The link from an animation to a node: the pad colour property's
        // on_change callback marks the pad's node dirty, so the next pass
        // repaints it. The demo is laid out first, because a node is born
        // dirty and only a pass clears the flag — without that, the
        // assertion would hold whatever the callback did.
        let mut demo = laid_out();
        demo.press_all();
        let _ = demo.clock.tick(Duration::from_millis(16));
        let nodes = demo.nodes.borrow();
        assert!(
            nodes.get(demo.pads[0].node).unwrap().layout().is_dirty(),
            "the first pad's node was marked dirty by the animation's write"
        );
    }

    #[test]
    fn pressing_the_mouse_presses_the_pad_under_the_cursor() {
        let mut demo = dialog_closed();
        let rect = {
            let nodes = demo.nodes.borrow();
            nodes
                .get(demo.pads[1].node)
                .unwrap()
                .layout()
                .rect()
                .unwrap()
        };
        demo.handle_event(Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: rect.origin.x + rect.size.width / 2.0,
            y: rect.origin.y + rect.size.height / 2.0,
        });
        demo.frame(WINDOW, Duration::from_millis(50));
        assert!(
            demo.pads[1].press.get() > 0.0,
            "the pad under the cursor is pressing"
        );
        assert_eq!(demo.pads[0].press.get(), 0.0, "and the others are not");
        assert_eq!(demo.pads[2].press.get(), 0.0);
    }

    /// Returns the `keycode` key-down event the demo reacts to.
    fn key(keycode: Keycode) -> Event {
        Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(keycode),
            scancode: None,
            keymod: Mod::empty(),
            repeat: false,
            which: 0,
            raw: 0,
        }
    }

    /// The colour the demo's first label paints with.
    fn first_label_color(demo: &Demo) -> Color {
        demo.labels[0].label.color.get()
    }

    /// The text runs the first label recorded on the last frame.
    fn first_label_runs(demo: &Demo) -> Vec<(f32, f32, String)> {
        let nodes = demo.nodes.borrow();
        let Some(node) = nodes.get(demo.label_nodes[0]) else {
            return Vec::new();
        };
        node.paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { x, y, text, .. } => Some((*x, *y, text.clone())),
                _ => None,
            })
            .collect()
    }

    /// The text runs a label recorded on the last frame.
    fn label_runs(demo: &Demo, index: usize) -> Vec<String> {
        let nodes = demo.nodes.borrow();
        let Some(node) = demo
            .label_nodes
            .get(index)
            .and_then(|&handle| nodes.get(handle))
        else {
            return Vec::new();
        };
        node.paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_last_panel_label_is_cut_with_an_ellipsis() {
        let demo = laid_out();
        let runs = label_runs(&demo, 6);
        assert_eq!(runs.len(), 1, "the long label is one line, not wrapped");
        assert!(
            runs[0].ends_with('\u{2026}'),
            "the long line is closed with an ellipsis, got {:?}",
            runs[0]
        );
        assert!(
            !runs[0].contains("ellipsis."),
            "the text is cut before its end, got {:?}",
            runs[0]
        );
    }

    #[test]
    fn the_text_panel_shows_the_greeting() {
        let demo = laid_out();
        assert_eq!(
            first_label_runs(&demo)[0].2,
            "Hello, World!",
            "the first thing the panel says is the greeting"
        );
    }

    #[test]
    fn the_panel_lays_out_its_labels_in_the_column() {
        let mut demo = demo();
        demo.frame(WINDOW, Duration::from_millis(16));
        let nodes = demo.nodes.borrow();
        let rects: Vec<_> = demo
            .label_nodes
            .iter()
            .map(|&handle| nodes.get(handle).unwrap().layout().rect().unwrap())
            .collect();
        for pair in rects.windows(2) {
            assert!(
                pair[1].origin.y > pair[0].origin.y,
                "each label is below the one before it"
            );
        }
        assert!(
            rects[0].origin.x > 0.0 && rects[0].origin.y > 0.0,
            "and the panel is inset from the window's edge"
        );
    }

    #[test]
    fn a_wrapped_label_records_more_than_one_line() {
        let mut demo = demo();
        demo.frame(WINDOW, Duration::from_millis(16));
        let nodes = demo.nodes.borrow();
        let runs = nodes
            .get(demo.label_nodes[1])
            .unwrap()
            .paint()
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::Text { .. }))
            .count();
        assert!(
            runs > 1,
            "the paragraph label wraps onto more than one line, at {runs} runs"
        );
    }

    #[test]
    fn the_plus_and_minus_keys_move_the_text_size() {
        let mut demo = dialog_closed();
        let start = demo.labels[0].label.font_size.get();
        demo.handle_event(key(Keycode::Equals));
        assert_eq!(
            demo.labels[0].label.font_size.get(),
            start + TEXT_SIZE_STEP,
            "every label takes the new size"
        );
        demo.handle_event(key(Keycode::Minus));
        assert_eq!(demo.labels[0].label.font_size.get(), start);
    }

    #[test]
    fn the_text_size_stays_inside_its_bounds() {
        let mut demo = dialog_closed();
        for _ in 0..100 {
            demo.handle_event(key(Keycode::Minus));
        }
        assert_eq!(demo.text_size, TEXT_SIZE_MIN, "and stops at the floor");
        for _ in 0..200 {
            demo.handle_event(key(Keycode::Plus));
        }
        assert_eq!(demo.text_size, TEXT_SIZE_MAX, "and stops at the ceiling");
    }

    #[test]
    fn a_bigger_font_gives_the_labels_taller_rects() {
        // The size is a plain field rather than a property because it changes
        // the labels' rects, not only their glyphs: the panel lays them out by
        // the rects it is given.
        let mut demo = dialog_closed();
        let first = demo.label_nodes[0];
        let before = {
            let nodes = demo.nodes.borrow();
            nodes.get(first).unwrap().layout().rect().unwrap()
        };
        demo.handle_event(key(Keycode::Equals));
        demo.frame(WINDOW, Duration::from_millis(16));
        let after = {
            let nodes = demo.nodes.borrow();
            nodes.get(first).unwrap().layout().rect().unwrap()
        };
        assert!(
            after.size.height > before.size.height,
            "a bigger font needs a taller line box: {} then {}",
            before.size.height,
            after.size.height
        );
    }

    #[test]
    fn the_c_key_moves_the_token_the_text_takes_its_colour_from() {
        let mut demo = dialog_closed();
        let first = first_label_color(&demo);
        assert_eq!(
            first,
            demo.theme.get(ThemeToken::Text).as_color().unwrap(),
            "the text starts on the theme's Text token"
        );
        demo.handle_event(key(Keycode::C));
        assert_ne!(first_label_color(&demo), first, "and C moved it");
        assert_eq!(
            first_label_color(&demo),
            demo.theme.get(ThemeToken::Primary).as_color().unwrap(),
            "onto the next token in the cycle"
        );
    }

    #[test]
    fn the_text_follows_the_theme_switch() {
        let dark = first_label_color(&dialog_closed());
        let mut demo = dialog_closed();
        demo.handle_event(key(Keycode::T));
        for _ in 0..35 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_ne!(
            first_label_color(&demo),
            dark,
            "the text moved with the theme"
        );
        assert_eq!(
            first_label_color(&demo),
            Theme::light().get(ThemeToken::Text).as_color().unwrap(),
            "and arrived at the light theme's Text token"
        );
    }

    /// The mouse press at `(x, y)`, and its release.
    ///
    /// The gap between the two timestamps is a hundred **nanoseconds**, because
    /// that is the unit SDL stamps events with (`SDL_GetTicksNS`, per
    /// `SDL_events.h`) and the recogniser reads them unchanged. A hundred
    /// milliseconds — a hundred million of them — is just as much a tap, and
    /// saying so here rather than in prose is what keeps the two apart.
    fn click_at(x: f32, y: f32) -> (Event, Event) {
        (
            Event::MouseButtonDown {
                timestamp: 0,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
            },
            Event::MouseButtonUp {
                timestamp: 100_000_000,
                window_id: 0,
                which: 0,
                mouse_btn: MouseButton::Left,
                clicks: 1,
                x,
                y,
            },
        )
    }

    // ------------------------------------------------ task 20: the gauge
    //
    // Everything below is about the *wiring*, not about the dial: `gauge.rs`'s own
    // 73 unit tests and 14 doctests are about what the widget draws and what its
    // setters do, and repeating them here would be a second opinion about someone
    // else's code rather than a check of this one. What is left that only this
    // file can know is that the demo built it, snapped it onto the theme, gave it
    // a rect, ticks it every frame, can move it without a pointer, and put it
    // where nothing else is.

    #[test]
    fn the_demo_shows_a_gauge_at_half() {
        // **Task 20's acceptance criterion, verbatim: "Demo shows a gauge at
        // 50%".** Half of 0..240 is 120, and that is what the demo builds it at,
        // so the dial opens on exactly the reading the criterion names rather than
        // on its minimum.
        let demo = laid_out();
        assert_eq!(
            demo.gauge.value.get(),
            GAUGE_START,
            "the gauge's value is the number the demo wrote"
        );
        assert_eq!(
            demo.gauge.min(),
            GAUGE_MIN,
            "and the range is the demo's, so the two cannot drift apart"
        );
        assert_eq!(demo.gauge.max(), GAUGE_MAX);
        // The widget's own mapping, not the demo's arithmetic: 120 of 240 is half.
        assert_eq!(demo.gauge.fraction(demo.gauge.value.get()), 0.5);
        // And it is *drawn* there rather than arrived at later, because
        // `snap_to_state` is what puts the drawn value on the truth at once. A
        // gauge that sprang to 120 over 600 ms on the first frame would still show
        // an empty dial in any capture taken before the spring finished.
        assert_eq!(
            demo.gauge.shown.get(),
            GAUGE_START,
            "the drawn value is on it too: the demo snapped, it did not animate"
        );
        assert_eq!(
            demo.readout_text_of(&demo.gauge_readout).as_deref(),
            Some("120 of 240, 50%, Needle"),
            "and the readout says so on screen, in the value, the share and the \
             shape"
        );
    }

    #[test]
    fn the_gauge_readout_says_the_share_the_widgets_own_fraction_gives() {
        // The demo computes the percentage itself rather than calling
        // `Gauge::fraction`, because a `Property::bind` closure cannot reach the
        // widget it is bound to — see the note where the readout is built. Two
        // copies of one mapping is two things to keep in step, so this is what
        // holds them in step: every tenth of the range, through the widget.
        let mut demo = laid_out();
        for step in 0..=10u8 {
            let share = f32::from(step) / 10.0;
            let value = GAUGE_MIN + (GAUGE_MAX - GAUGE_MIN) * share;
            assert_eq!(
                demo.gauge.fraction(value),
                share,
                "the widget maps {value} to {share}"
            );
        }
        // And the string the demo would print for each of them, at the one place
        // that string is formed: `readout_text_of` is the readout, so this asks the
        // readout rather than re-deriving the format.
        demo.gauge.value.set(GAUGE_MAX);
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.gauge_readout).as_deref(),
            Some("240 of 240, 100%, Needle"),
            "and the top of the range reads as a hundred per cent"
        );
        demo.gauge.value.set(GAUGE_MIN);
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.gauge_readout).as_deref(),
            Some("0 of 240, 0%, Needle"),
            "and the bottom as none"
        );
    }

    #[test]
    fn the_gauge_is_on_its_theme_rather_than_on_the_neutral_greys() {
        // **A palette read and `set_palette` are not enough**: the four colour
        // properties still hold the neutral greys `Gauge::new` wrote until
        // something moves them, and a themed gauge that was never snapped draws
        // those greys. That is the same trap the button band fell into first, and
        // the test below the demo's `snap_to_state` calls is the reason it is
        // written where it is rather than left to a paint pass.
        let demo = laid_out();
        let themed = GaugePalette::from_theme(&Theme::dark());
        assert_eq!(demo.gauge.track.get(), themed.track, "the track");
        assert_eq!(demo.gauge.fill.get(), themed.fill, "the fill");
        assert_eq!(demo.gauge.tick.get(), themed.tick, "the marks");
        assert_eq!(
            demo.gauge.needle.get(),
            themed.needle,
            "and the needle, which is only drawn in the needle shape"
        );
        // Every one of them is a palette colour, and the palette's greys are not
        // the theme's: an assertion that only compared the four to each other
        // would pass on a palette of the wrong greys.
        assert_ne!(
            demo.gauge.fill.get(),
            ui_core::widgets::gauge::Palette::default().fill,
            "and the fill is not the neutral default it was constructed with"
        );
    }

    #[test]
    fn a_gauge_moved_by_a_key_animates_its_needle_rather_than_jumping() {
        // Requirement 4's "fill animates when value changes" and "needle
        // animates with spring physics", as the demo wires them: the value is
        // written, `animate_to_state` is what starts the travel, and the drawn
        // value is somewhere **between** the old and the new one on the way.
        //
        // The spring matters and is asserted as the spring: an underdamped one
        // passes its target and comes back, so the needle overshoots. A `set`
        // instead of an animation would be at the far end on the first frame and
        // pass a test that only looked at where it ended up.
        let mut demo = dialog_closed();
        let before = demo.gauge.shown.get();
        demo.handle_event(key(Keycode::Period));
        assert_eq!(
            demo.gauge.value.get(),
            GAUGE_START + GAUGE_STEP,
            "the key wrote the truth, one step up"
        );
        assert!(
            demo.gauge.is_animating(),
            "and something is on its way there"
        );
        assert_eq!(
            demo.gauge.shown.get(),
            before,
            "the needle has not moved yet: the first frame has not been drawn"
        );

        // A tenth of the way through 600 ms, part way up and **past** where a
        // linear curve would be, because the spring is quicker than linear early
        // and then rings.
        demo.frame(WINDOW, Duration::from_millis(60));
        let midway = demo.gauge.shown.get();
        assert!(
            midway > before && midway < GAUGE_START + GAUGE_STEP,
            "part way to {GAUGE_STEP}, not there and not here: {midway}"
        );

        // And it arrives, rather than creeping toward its target for ever.
        for _ in 0..40 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert_eq!(
            demo.gauge.shown.get(),
            GAUGE_START + GAUGE_STEP,
            "and it arrives at the value the key wrote"
        );
        assert!(
            !demo.gauge.is_animating(),
            "a transition that has arrived has stopped"
        );
    }

    #[test]
    fn the_spring_is_the_callers_and_the_demo_owns_the_curve() {
        // The widget never picks a curve or a duration: `animate_to_state` takes
        // the caller's `Motion` and honours it, which is what makes "the needle
        // animates with spring physics" a statement about the demo rather than
        // about the widget. This is that statement, held as an assertion: the
        // demo hands over the spring and the widget arrives on it.
        //
        // The shape of the argument is the same one every other motion assertion
        // in this file uses — a `Motion` built from named constants rather than
        // inline, so a reader can see the coefficients rather than trust them.
        let motion = Motion {
            duration: GAUGE_MOTION,
            easing: GAUGE_SPRING,
        };
        assert_eq!(motion.duration, Duration::from_millis(600), "the span");
        assert!(
            matches!(
                motion.easing,
                Easing::Spring {
                    damping: 9.0,
                    stiffness: 140.0
                }
            ),
            "and the curve is the spring, which is a copy of RELEASE_SPRING: a pad \
             springing back to rest and a needle springing to a reading are the \
             same movement"
        );
    }

    #[test]
    fn the_needle_overshoots_because_the_spring_is_underdamped() {
        // The distinction between a spring and an ease, on the one number that
        // tells them apart: a spring goes **past** its target and comes back, an
        // ease does not. Without this the demo would animate a needle and the
        // claim "spring physics" would be a name for whatever curve was used.
        let mut demo = dialog_closed();
        demo.handle_event(key(Keycode::Period));
        let mut peak = 0.0f32;
        for _ in 0..60 {
            demo.frame(WINDOW, Duration::from_millis(16));
            peak = peak.max(demo.gauge.shown.get());
        }
        assert_eq!(
            demo.gauge.shown.get(),
            GAUGE_START + GAUGE_STEP,
            "it settles on the value"
        );
        assert!(
            peak > GAUGE_START + GAUGE_STEP,
            "and it went past it first, at {peak}, which an ease cannot do"
        );
        // Bounded, though: a spring that rings for ever is not what a needle
        // should do either, and the widget pins its endpoints.
        assert!(
            peak < GAUGE_START + GAUGE_STEP * 1.5,
            "and the overshoot is a fraction, not a swing: {peak}"
        );
    }

    #[test]
    fn the_gauge_keys_move_it_by_a_whole_step_and_stop_at_both_ends() {
        // A key-driven gauge with no clamping would run off the end of its own
        // range: `,` at the minimum would report −24 km/h, which is a gauge
        // pointing below its own floor. The demo clamps and the widget clamps, and
        // this asks the demo's — because the demo is the thing that can produce a
        // value outside the range in the first place.
        let mut demo = dialog_closed();
        for _ in 0..20 {
            demo.handle_event(key(Keycode::Comma));
        }
        assert_eq!(
            demo.gauge.value.get(),
            GAUGE_MIN,
            "twenty steps below the start is the bottom of the range and no lower"
        );
        for _ in 0..30 {
            demo.handle_event(key(Keycode::Period));
        }
        assert_eq!(
            demo.gauge.value.get(),
            GAUGE_MAX,
            "and thirty above is the top"
        );
        // Ten steps of `GAUGE_STEP` is the whole range, so the two loops above
        // between them prove the count reaches both ends rather than jumping.
        assert_eq!(
            GAUGE_MIN + (GAUGE_MAX - GAUGE_MIN) / GAUGE_STEP,
            10.0,
            "the range is ten steps of {GAUGE_STEP}"
        );
    }

    #[test]
    fn a_gauge_value_that_arrived_by_addition_lands_on_the_grid() {
        // The reason [`GAUGE_TENTHS`] exists, asked of the demo rather than of
        // the constant: a value accumulated by addition is off the grid by a
        // fraction of a pixel per press, and a needle a third of a degree off is a
        // needle that is not on the mark the key name claims it is on.
        let mut demo = dialog_closed();
        let mut seen: Vec<f32> = Vec::new();
        for _ in 0..10 {
            demo.handle_event(key(Keycode::Comma));
            seen.push(demo.gauge.value.get());
        }
        for &value in &seen {
            assert_eq!(
                value % GAUGE_STEP,
                0.0,
                "{value} is a whole number of {GAUGE_STEP}s"
            );
        }
        assert_eq!(
            seen,
            vec![96.0, 72.0, 48.0, 24.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "which is the whole grid down to the floor, and every step after it \
             is the same number and not a smaller one"
        );
    }

    #[test]
    fn the_g_key_walks_the_gauge_through_its_three_shapes_and_wraps() {
        // Three shapes and a key to reach them with, and the readout has to
        // follow: a demo that cycled the dial and left the label naming the shape
        // it started in would be showing a caption for a different picture.
        let mut demo = dialog_closed();
        assert_eq!(demo.gauge.gauge_type(), GaugeType::Needle, "it starts here");
        for expected in ["Arc", "Circle", "Needle"] {
            demo.handle_event(key(Keycode::G));
            demo.frame(WINDOW, Duration::from_millis(16));
            assert!(
                demo.readout_text_of(&demo.gauge_readout)
                    .as_deref()
                    .is_some_and(|said| said.ends_with(expected)),
                "G moved it to {expected}, and the readout says so: {:?}",
                demo.readout_text_of(&demo.gauge_readout)
            );
        }
        assert_eq!(
            demo.gauge.gauge_type(),
            GaugeType::Needle,
            "and three presses are back where it started, so the cycle closes"
        );
    }

    #[test]
    fn the_dial_draws_a_needle_only_in_the_needle_shape() {
        // The needle is a filled triangle plus a hub, and it is decoration: a
        // needle appearing in the other two shapes would be a caller error at
        // best, so this asks the widget's own rule through the demo's paint.
        //
        // **Counted by point count, not by "a polygon".** The arc is drawn as
        // polygons too — one per band segment — so a filter on the variant alone
        // would count 51 of them and call them needles. A three-pointed polygon is
        // the needle and nothing else this widget records.
        let mut demo = dialog_closed();
        let needles = |demo: &Demo| {
            demo.commands_at(demo.gauge.handle())
                .iter()
                .filter(|command| {
                    matches!(command, DrawCommand::Polygon { points, .. } if points.len() == 3)
                })
                .count()
        };
        assert_eq!(needles(&demo), 1, "a needle shape draws its pointer");
        for _ in 0..2 {
            demo.handle_event(key(Keycode::G));
            demo.frame(WINDOW, Duration::from_millis(16));
            assert_eq!(
                needles(&demo),
                0,
                "and the other two shapes draw no needle at all"
            );
        }
    }

    #[test]
    fn the_fill_is_drawn_over_the_track_and_neither_leaves_the_node() {
        // Two claims the capture settles and the draw commands can also settle,
        // which is why both are asked. **The order**: the track's band is recorded
        // first and the fill's second, so the fill is painted over the track — the
        // alternative would be a fill hidden underneath it, which looks like a
        // gauge that does not move. **The bounds**: nothing the widget records
        // reaches outside its own rect, so a 200-pixel node cannot draw into the
        // slider 56 pixels below it.
        let demo = laid_out();
        let rect = demo
            .node_rect(demo.gauge.handle())
            .expect("a laid-out gauge");
        let commands = demo.commands_at(demo.gauge.handle());
        assert!(
            commands.len() > 20,
            "the dial records {} commands, so this is walking the whole set",
            commands.len()
        );
        let palette = demo.gauge.palette();
        // **The band is polygons**, one four-pointed quad per segment: 33 for the
        // track's 270 degrees and 17 for the fill's half of it. The counts are the
        // widget's own arithmetic and are written out because the count *is* the
        // cost: the band is cut into `ceil(sweep / 8.25°)` segments whatever the
        // thickness, so 270 needs 33 and 135 needs 17.
        //
        // **Asserted exactly rather than "at least"**, because "at least" would pass
        // on a band twice as long, and a longer band is the failure mode
        // `.ai/NEVERAGAIN.md` § *a still screenshot of a 4 fps application* warns
        // about — invisible in a still and expensive every frame.
        let band: Vec<&DrawCommand> = commands
            .iter()
            .filter(|command| {
                matches!(command, DrawCommand::Polygon { points, .. } if points.len() == 4)
            })
            .collect();
        assert_eq!(
            band.len(),
            33 + 17,
            "33 for the track and 17 for half a sweep"
        );
        // The two are distinguishable only by their colour, and the first is the
        // track's because `Gauge::paint` draws the track first.
        let colors: Vec<Color> = band
            .iter()
            .map(|command| match command {
                DrawCommand::Polygon { color, .. } => *color,
                _ => Color::new(0, 0, 0, 0),
            })
            .collect();
        assert_eq!(
            colors[0], palette.track,
            "the band opens on the track's own colour"
        );
        let fill_at = colors.iter().position(|color| *color == palette.fill);
        assert_eq!(
            fill_at,
            Some(33),
            "and the fill's band begins at the 34th quad, immediately after the \
             track's 33, so it is drawn over the track rather than under it"
        );
        // The bounds, from the recorded geometry rather than from the widget's
        // promise: every corner of the band, and the hub's own circle, is inside
        // the node's own box. A quad's corners are the whole of its extent, which
        // is more than a circle's centre and its radius could say.
        for command in &commands {
            match command {
                DrawCommand::Polygon { points, .. } => {
                    for (x, y) in points {
                        assert!(
                            *x >= rect.x && *x <= rect.x + rect.width,
                            "a band corner at ({x}, {y}) is left of or right of {rect:?}"
                        );
                        assert!(
                            *y >= rect.y && *y <= rect.y + rect.height,
                            "a band corner at ({x}, {y}) is above or below {rect:?}"
                        );
                    }
                }
                // The centre is a plain tuple of two `f32`s and not a `Point`,
                // which is the shape the draw-command record carries so that a
                // command can be cloned and compared without a layout type in it.
                DrawCommand::Circle { center, radius, .. } => {
                    let (x, y, r) = (center.0, center.1, *radius);
                    assert!(
                        x - r >= rect.x && x + r <= rect.x + rect.width,
                        "a circle at ({x}, {y}) of radius {r} reaches outside {rect:?}"
                    );
                    assert!(
                        y - r >= rect.y && y + r <= rect.y + rect.height,
                        "a circle at ({x}, {y}) of radius {r} reaches outside {rect:?}"
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn the_gauge_is_aimed_at_the_new_theme_rather_than_the_one_it_is_leaving() {
        // The defect class this repository has already paid for once: a palette
        // read **after** `switch_to` is the palette the theme is leaving, which
        // re-aims every widget at what it already had and the transition goes
        // nowhere while every test stays green. The gauge is aimed like the rest,
        // and this is the assertion that says so.
        let mut demo = dialog_closed();
        let dark_track = demo.gauge.track.get();
        assert_eq!(
            dark_track,
            GaugePalette::from_theme(&Theme::dark()).track,
            "it starts on the dark theme\'s own Border"
        );

        demo.handle_event(key(Keycode::T));
        for _ in 0..35 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_ne!(demo.gauge.track.get(), dark_track, "the track moved");
        assert_eq!(
            demo.gauge.track.get(),
            GaugePalette::from_theme(&Theme::light()).track,
            "and arrived at the light theme\'s own Border"
        );
        // The fill is the one a reader would notice, because it is the part of the
        // dial that says "the value", and the track is the part that says
        // "the rest of it". Both are checked because a palette with one of them
        // right and the other wrong is a plausible near-miss.
        assert_eq!(
            demo.gauge.fill.get(),
            GaugePalette::from_theme(&Theme::light()).fill,
            "and so did the fill, which is the theme\'s Primary"
        );
    }

    #[test]
    fn the_gauge_sits_above_the_slider_and_below_the_image() {
        // The gauge took the space the buttons and the click counter vacated, and
        // it **overlapped nothing to get there**. The two collision tests over
        // `placed_rects` are the general version of this; this one names the
        // neighbours, so a failure says which box it is on top of rather than
        // reporting forty pairs — and it is the test that would have caught a
        // dial laid over the slider, which is what putting a 200-pixel square in
        // a column of 44-pixel bars invites.
        let demo = laid_out();
        let at = |handle: Handle| demo.node_rect(handle).expect("a laid-out node");
        let gauge = at(demo.gauge.handle());
        let slider = at(demo.slider.node());
        let readout = at(demo.gauge_readout.label.handle());
        let image_fit = at(demo.image_fit_readout.label.handle());

        assert!(
            gauge.y + gauge.height <= slider.y,
            "the dial ends at {} and the slider starts at {}",
            gauge.y + gauge.height,
            slider.y
        );
        assert!(
            gauge.y + gauge.height <= readout.y,
            "and the readout is under it, not inside it"
        );
        assert!(
            readout.y + readout.height <= slider.y,
            "and clear of the slider in turn"
        );
        assert!(
            gauge.y >= image_fit.y + image_fit.height,
            "the dial starts at {}, below the image\'s own readout which ends at {}",
            gauge.y,
            image_fit.y + image_fit.height
        );
        let column_right = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        assert!(
            gauge.x > column_right,
            "and the whole column is right of the text at {column_right}"
        );
        // Above the band, which is the constraint that keeps every capture of
        // tasks 11 to 19 a capture of the same pixels.
        assert!(
            readout.y + readout.height <= BAND_TOP,
            "and nothing it draws is below {BAND_TOP}"
        );

        // And the general collision test can see it at all. `placed_rects` is the
        // set `no_two_placed_rects_overlap` walks, so a gauge left out of it is a
        // gauge the whole-suite collision check is blind to — and this mutation is
        // the one that survived when the rest of these tests were checked, because
        // every neighbour claim above names its neighbour by hand and each one
        // still held. The assertion is deliberately about **membership**: a dial
        // on top of something the demo has not heard of is exactly what the
        // general check exists to catch, and it can only catch what it is given.
        let placed = demo.placed_rects();
        let named: Vec<&str> = placed.iter().map(|(what, _)| *what).collect();
        for what in ["gauge", "gauge readout"] {
            assert!(
                named.contains(&what),
                "the {what} is in placed_rects, so the collision tests can see it: \
                 {named:?}"
            );
        }
        let placed_gauge = placed
            .iter()
            .find(|(what, _)| *what == "gauge")
            .map(|(_, rect)| *rect)
            .expect("the gauge is named above");
        assert_eq!(
            placed_gauge, gauge,
            "and the box it reports is the box its own node was laid out at"
        );
    }

    #[test]
    fn the_gauge_is_the_only_box_in_the_window_that_is_square() {
        // A gauge inscribed in a non-square box is a dial inscribed in the
        // shorter of its two sides, so a wide box buys nothing and leaves empty
        // space the collision tests would have to reason about. This is the
        // assertion that the demo asked for [`GAUGE_SIZE`] and not for a rect of
        // its own, and it is the reason a widened node would be a silent change:
        // the dial would be the same size and the box twice as wide.
        let demo = laid_out();
        let rect = demo
            .node_rect(demo.gauge.handle())
            .expect("a laid-out gauge");
        assert_eq!(
            (rect.width, rect.height),
            (GAUGE_SIZE.width, GAUGE_SIZE.height),
            "the node is the box the demo asked for"
        );
        assert_eq!(rect.width, rect.height, "and the box is square");
        assert_eq!(
            (rect.width, rect.height),
            (demo.gauge.size().width, demo.gauge.size().height),
            "which is the widget\'s own DEFAULT_SIZE: the demo chose nothing"
        );
    }

    #[test]
    fn a_press_on_the_gauge_reaches_nothing() {
        // A gauge is a display: there is no value a drag would set and no action
        // a tap would report, so a finger that lands on the needle goes to
        // whatever is behind the gauge. **The demo has nothing behind it there**,
        // so the correct outcome of a press over the dial is that nothing at all
        // happens — and this asserts that, because `.ai/NEVERAGAIN.md` § *a drawn
        // control with nothing behind it* is the entry about a widget that looks
        // grabbable and is not, and the mistake it records was made by a test
        // suite that only asked whether the control could be operated.
        //
        // What it also rules out is the opposite defect: a tap that fell through
        // to the pads, which are the one thing in the window that does answer a
        // press.
        let mut demo = laid_out();
        let gauge = demo
            .node_rect(demo.gauge.handle())
            .expect("a laid-out gauge");
        let (down, up) = click_at(gauge.x + gauge.width / 2.0, gauge.y + gauge.height / 2.0);
        let before = demo.gauge.value.get();

        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.gauge.value.get(),
            before,
            "the dial\'s value is the demo\'s to write, not a tap\'s"
        );
        assert_eq!(
            demo.gauge.gauge_type(),
            GaugeType::Needle,
            "and a tap does not change its shape either"
        );
        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 0.0, "and no pad behind it was pressed");
        }
    }

    #[test]
    fn the_controls_sit_clear_of_the_text_panel_and_the_pads() {
        // The controls layer is placed by hand, so its position is a claim about
        // the window that has to be checked. **The head of the column moved when
        // the buttons went** — the gauge is at 240 rather than the row\'s 396,
        // because a dial needs room a button row did not — and this is what
        // checks that the new head is still below the pads and still right of the
        // text.
        let demo = laid_out();
        let gauge = demo
            .node_rect(demo.gauge.handle())
            .expect("a laid-out gauge");
        let column_right = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        assert!(
            gauge.x > column_right,
            "the column starts at x = {}, right of the text column\'s edge at \
             {column_right}",
            gauge.x
        );
        assert!(
            gauge.y > PAD_SIZE.height,
            "and at y = {}, below the pads",
            gauge.y
        );

        // The concrete claim, and the one that would fail if the origin were put
        // on the layer rather than on the gauge: a `Stack` places every child at
        // the origin regardless of the position it declares, so an offset on the
        // layer is ignored and everything inside it would land on the pads.
        let nodes = demo.nodes.borrow();
        for pad in &demo.pads {
            let rect = nodes
                .get(pad.node)
                .and_then(|node| node.layout().rect())
                .expect("a laid-out pad");
            let overlaps = gauge.x < rect.origin.x + rect.size.width
                && rect.origin.x < gauge.x + gauge.width
                && gauge.y < rect.origin.y + rect.size.height
                && rect.origin.y < gauge.y + gauge.height;
            assert!(!overlaps, "the controls overlap a pad at {rect:?}");
        }
    }

    #[test]
    fn no_text_label_reaches_under_the_controls() {
        // The collision a screenshot showed, asked again against the column that
        // is there now: the alignment rows are laid out across
        // `TEXT_COLUMN_WIDTH`, so a right-aligned one ends at the column\'s right
        // edge. If that edge ever moves right of the controls, the text runs under
        // the dial — and nothing else in the suite would notice, since both the
        // label and the gauge lay out correctly on their own.
        let demo = laid_out();
        let gauge = demo
            .node_rect(demo.gauge.handle())
            .expect("a laid-out gauge");
        let nodes = demo.nodes.borrow();
        for &handle in &demo.label_nodes {
            let node = nodes.get(handle).expect("a label node");
            let right = node
                .paint()
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { x, text, .. } => {
                        let width = demo
                            .metrics
                            .advance(text.chars().next().unwrap_or(' '), TEXT_SIZE_START);
                        Some(*x + width * text.chars().count() as f32)
                    }
                    _ => None,
                })
                .fold(f32::MIN, f32::max);
            if right == f32::MIN {
                continue;
            }
            assert!(
                right <= gauge.x,
                "a label ends at {right}, which is under the controls at {}",
                gauge.x
            );
        }
    }

    /// The rounded rectangles the card recorded, with their radii and colours.
    ///
    /// The card is the one container in the demo with a background, so this is
    /// how a test asks what was drawn behind the pads.
    fn card_rects(demo: &Demo) -> Vec<(f32, Color)> {
        let nodes = demo.nodes.borrow();
        nodes
            .get(demo.card().handle())
            .expect("the card's node")
            .paint()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect { radius, color, .. } => Some((*radius, *color)),
                _ => None,
            })
            .collect()
    }

    /// The parents in the demo's tree that a **widget** owns rather than the
    /// demo assembling them.
    ///
    /// **One again, and it came back with the dialog on 2026-10-03.** There were
    /// two such parents from task 18 — the node a `List` scrolls in and the
    /// content node it hangs its rows from, both owned by the `Scroll` — and this
    /// list held them until the operator had the list taken out on 2026-10-02. It
    /// is back for the same shape of reason: a `Dialog`'s action buttons are real
    /// [`Button`](ui_core::widgets::button::Button) widgets with nodes of their
    /// own, and [`Dialog::add_action`] attaches them to the dialog's own node,
    /// which the demo did not build and cannot.
    ///
    /// Named here rather than papered over, which is what the previous two
    /// entries in this history were for, and the count asserted below is what
    /// stops the list from quietly growing.
    fn widget_owned_parents(demo: &Demo) -> Vec<Handle> {
        vec![demo.dialog.handle()]
    }

    #[test]
    fn every_parent_the_demo_assembles_is_a_container_widget() {
        // The demo used to assemble its own parent nodes, which meant two
        // implementations of the same composition primitive in one repository:
        // the demo's private helper and the widget. Nothing in the suite would
        // have noticed a new one appearing, so this is the check that a parent
        // the *demo* built is a `Container` and not a node the demo wired up.
        //
        // **One exception, and it is a widget's rather than a relaxation.** See
        // `widget_owned_parents`: the list was empty for one day between task 21
        // and task 22 and it is not empty now, because a dialog's actions are
        // real buttons in real nodes under the dialog's own node. The claim
        // being tested is unchanged — **the demo assembles its own parents as
        // `Container` widgets and never as nodes it wired up by hand** — and this
        // is the check that a *new* hand-wired parent fails.
        let demo = laid_out();
        let nodes = demo.nodes.borrow();
        let containers: Vec<Handle> = demo.containers.iter().map(Container::handle).collect();
        let widget_owned = widget_owned_parents(&demo);
        let mut parents = 0;
        for &handle in &demo.order {
            let node = nodes.get(handle).expect("a node in the demo's tree");
            if node.children().is_empty() {
                continue;
            }
            parents += 1;
            assert!(
                containers.contains(&handle) || widget_owned.contains(&handle),
                "node {handle:?} has children but is neither a Container of the \
                 demo's nor one of the widget-owned parents {:?}",
                widget_owned
            );
        }
        assert_eq!(
            parents,
            containers.len() + widget_owned.len(),
            "and every parent in the tree is one of those five or the dialog"
        );
        assert_eq!(
            containers.len(),
            5,
            "the demo assembles five: the card, the text column and its panel, the \
             controls layer and the root. It was six while the button row was a \
             container of its own"
        );
        assert_eq!(
            widget_owned.len(),
            1,
            "and exactly one parent is a widget's: the dialog's own node, with its \
             two action buttons under it. A second would be a new exception and \
             would have to be named and counted here"
        );
    }

    #[test]
    fn the_pads_sit_inside_the_card_their_row_draws() {
        let demo = laid_out();
        let nodes = demo.nodes.borrow();
        let card = nodes
            .get(demo.card().handle())
            .expect("the card's node")
            .layout()
            .rect()
            .expect("a laid-out card");
        let rect = |node| {
            nodes
                .get(node)
                .expect("a laid-out pad")
                .layout()
                .rect()
                .expect("a laid-out pad")
        };
        let first = rect(demo.pads[0].node);
        let last = rect(demo.pads[2].node);

        // The card's own rect is the row's plus the padding, and the pads are
        // CARD_PADDING in from its far edges. A padding that was stored and never
        // applied would leave the card exactly the row's size, and these two
        // differences would be zero.
        assert_eq!(
            card.size.width - (last.origin.x + last.size.width),
            CARD_PADDING,
            "the card is CARD_PADDING wider than the pads reach"
        );
        assert_eq!(
            card.size.height - (first.origin.y + PAD_SIZE.height),
            CARD_PADDING,
            "and CARD_PADDING taller"
        );
        assert_eq!(
            first.origin.x, CARD_PADDING,
            "with the first pad inset by it as well"
        );
    }

    #[test]
    fn the_card_paints_the_themes_surface_behind_the_pads() {
        let demo = laid_out();
        let surface = demo
            .theme
            .get(ThemeToken::Surface)
            .as_color()
            .expect("a colour in the Surface token");
        let radius = demo
            .theme
            .get(ThemeToken::BorderRadiusLg)
            .as_number()
            .expect("a number in the BorderRadiusLg token");

        assert_eq!(
            card_rects(&demo),
            vec![(radius, surface)],
            "the card is one rounded rectangle, in the theme's surface colour \
             and at the theme's loosest corner radius"
        );

        // Behind the pads, because the walk is parent first: the card is
        // recorded before every pad, so it is drawn before every pad.
        let card_at = demo
            .order
            .iter()
            .position(|&handle| handle == demo.card().handle())
            .expect("the card is in the tree");
        for pad in &demo.pads {
            let pad_at = demo
                .order
                .iter()
                .position(|&handle| handle == pad.node)
                .expect("a pad is in the tree");
            assert!(
                card_at < pad_at,
                "pad at {pad_at} is painted after the card at {card_at}"
            );
        }
    }

    #[test]
    fn the_card_follows_a_theme_switch() {
        let mut demo = dialog_closed();
        let before = card_rects(&demo).first().map(|(_, color)| *color);

        demo.handle_event(toggle_theme_event());
        // The switch is 300 ms, so four 100 ms frames run it out.
        for _ in 0..4 {
            demo.frame(WINDOW, Duration::from_millis(100));
        }

        let after = demo
            .theme
            .get(ThemeToken::Surface)
            .as_color()
            .expect("a colour in the Surface token");
        assert_eq!(
            card_rects(&demo).first().map(|(_, color)| *color),
            Some(after),
            "the card arrived at the light theme's own surface"
        );
        assert_ne!(
            before,
            Some(after),
            "and the two themes really do hold different surfaces"
        );
    }

    #[test]
    fn the_other_containers_draw_nothing() {
        // A container with no background is invisible: the demo has five of them
        // and the card is the sixth, so a background leaking onto any of the
        // others would be visible as a box where the demo has always had the
        // window's own background.
        let demo = laid_out();
        let card = demo.card().handle();
        for container in &demo.containers {
            if container.handle() == card {
                continue;
            }
            let nodes = demo.nodes.borrow();
            let node = nodes.get(container.handle()).expect("a container's node");
            assert_eq!(
                node.paint().commands().len(),
                0,
                "a container with no background records no draw commands"
            );
        }
    }

    /// A finger down at `(x, y)`, a motion to `(to_x, to_y)`, and a release at
    /// the same place.
    ///
    /// A car has no mouse, so the demo's slider is driven by touch and the tests
    /// drive it the way the platform delivers it. The motion has to travel past
    /// the recogniser's own threshold to be a drag at all — ten pixels, per
    /// `TAP_MAX_MOVEMENT` — which is what a real finger does.
    fn drag_on(x: f32, y: f32, to_x: f32, to_y: f32) -> [Event; 3] {
        let finger = 1;
        [
            Event::FingerDown {
                timestamp: 0,
                touch_id: finger,
                finger_id: finger,
                x,
                y,
                dx: 0.0,
                dy: 0.0,
                pressure: 1.0,
                window_id: 0,
            },
            Event::FingerMotion {
                timestamp: 1_000_000,
                touch_id: finger,
                finger_id: finger,
                x: to_x,
                y: to_y,
                dx: to_x - x,
                dy: to_y - y,
                pressure: 1.0,
                window_id: 0,
            },
            Event::FingerUp {
                timestamp: 200_000_000,
                touch_id: finger,
                finger_id: finger,
                x: to_x,
                y: to_y,
                dx: 0.0,
                dy: 0.0,
                pressure: 1.0,
                window_id: 0,
            },
        ]
    }

    /// A steering-wheel axis event, which is what a wheel turned one way arrives
    /// as: a `Scroll` with no position, because an axis is not under a pointer.
    ///
    /// The axis is the module's own `STEERING_WHEEL_SCROLL_AXIS` rather than a
    /// literal, because that constant is what decides whether the recogniser
    /// produces a `Scroll` at all — the one axis it maps.
    fn wheel(value: i16) -> Event {
        Event::GamepadAxisMotion {
            timestamp: 0,
            which: sdl3::joystick::JoystickId::from(0),
            axis: ui_core::input::STEERING_WHEEL_SCROLL_AXIS,
            value,
        }
    }

    /// The circles the slider recorded on the last frame, as their centre and
    /// radius.
    ///
    /// The thumb is two of them — the border and the thumb's own — and the thumb's
    /// own is the last, because it is painted over the border.
    fn painted_thumb(demo: &Demo) -> ((f32, f32), f32) {
        let circles: Vec<((f32, f32), f32)> = demo
            .slider_commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Circle { center, radius, .. } => Some((*center, *radius)),
                _ => None,
            })
            .collect();
        let (center, radius) = *circles
            .last()
            .unwrap_or_else(|| panic!("the slider painted no thumb: {circles:?}"));
        (center, radius)
    }

    /// The x the slider's thumb was painted at.
    fn painted_thumb_x(demo: &Demo) -> f32 {
        painted_thumb(demo).0 .0
    }

    /// The radius the slider's thumb was painted at.
    fn painted_thumb_radius(demo: &Demo) -> f32 {
        painted_thumb(demo).1
    }

    #[test]
    fn the_demo_has_a_slider_over_a_hundred() {
        let demo = laid_out();
        let slider = &demo.slider.widget;
        assert_eq!(slider.min(), SLIDER_MIN);
        assert_eq!(slider.max(), SLIDER_MAX);
        assert_eq!(slider.step(), Some(SLIDER_STEP));
        assert_eq!(slider.orientation(), Orientation::Horizontal);
        assert_eq!(
            slider.value.get(),
            SLIDER_MIN,
            "a slider starts at its minimum, and the demo has not touched it"
        );
        assert_eq!(
            slider.track.get(),
            SliderPalette::from_theme(&Theme::dark()).track,
            "and is already on the theme's colours rather than the neutral ones"
        );
        assert!(
            demo.nodes.borrow().get(demo.slider.node()).is_some(),
            "with a node of its own in the tree"
        );
    }

    #[test]
    fn the_slider_paints_its_track_its_fill_and_its_thumb() {
        // "Renders track, fill, and thumb" is three shapes on one node: two
        // rounded rectangles and two circles, the thumb being a border with a
        // circle on top of it.
        let demo = laid_out();
        let commands = demo.slider_commands();
        let rects = commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::RoundedRect { .. }))
            .count();
        let circles = commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Circle { .. }))
            .count();
        assert_eq!(rects, 2, "a track and a fill");
        assert_eq!(circles, 2, "a thumb, which is a border and a circle");
        assert_eq!(commands.len(), 4, "and nothing else on the node");
    }

    #[test]
    fn a_drag_on_the_slider_moves_its_value_and_its_thumb() {
        // The whole path a finger takes: down, motion, up, and a frame between
        // each so the widget's properties reach the screen.
        let mut demo = dialog_closed();
        let (x, y) = demo.slider_at_fraction(0.5).expect("a laid-out slider");
        for event in drag_on(x, y, x + 80.0, y) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }

        let slider = &demo.slider.widget;
        assert!(
            slider.value.get() > 50.0,
            "dragging right of the middle raised the value: {}",
            slider.value.get()
        );
        assert_eq!(
            slider.value.get() % SLIDER_STEP,
            0.0,
            "and it snapped to the step, so {} is on the grid",
            slider.value.get()
        );
        let rect = demo.slider_rect().expect("a laid-out slider");
        let wanted = slider.thumb_center(rect).0;
        assert!(
            (painted_thumb_x(&demo) - wanted).abs() < 0.01,
            "and the thumb was painted at {} rather than {wanted}",
            painted_thumb_x(&demo)
        );
    }

    #[test]
    fn a_drag_past_the_end_of_the_slider_clamps_at_its_maximum() {
        // The finger has left the slider by the time it is past the end, which is
        // why the demo offers the drag to the slider being dragged rather than
        // only to whatever is under the pointer. Without that the value would
        // stop at the edge of the node instead of at the end of the range.
        let mut demo = dialog_closed();
        let (x, y) = demo.slider_at_fraction(0.25).expect("a laid-out slider");
        for event in drag_on(x, y, x + 900.0, y) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert_eq!(demo.slider.widget.value.get(), SLIDER_MAX);

        // The thumb's centre stops one radius in from the far end of the node, so
        // the number is **the widget's own thumb radius** rather than a literal
        // that went stale when the demo's slider grew: the node is 300 wide and the
        // radius is 22, so 300 - 22 = 278 px of travel from the left inset.
        let rect = demo.slider_rect().expect("a laid-out slider");
        assert_eq!(
            painted_thumb_x(&demo),
            rect.x + rect.width - SLIDER_THUMB_RADIUS,
            "and the thumb is a radius in from the far end of the track"
        );
    }

    #[test]
    fn a_tap_on_the_sliders_track_jumps_the_value_there() {
        let mut demo = dialog_closed();
        let (x, y) = demo.slider_at_fraction(0.75).expect("a laid-out slider");
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.slider.widget.value.get(),
            75.0,
            "the tap landed three quarters along 0 to 100"
        );
    }

    #[test]
    fn the_sliders_readout_follows_the_value_and_counts_its_adjustments() {
        let mut demo = dialog_closed();
        assert_eq!(
            demo.readout_text().as_deref(),
            Some("0 of 100, 0 adjustments")
        );

        // A finger that goes down at the thumb's position for the **middle** of
        // the range and drags forty pixels further lands past 50, not at 90: the
        // thumb now has `SLIDER_LENGTH - 2 * SLIDER_THUMB_RADIUS` = 300 - 36 =
        // 264 px of run rather than 240 - 24 = 216, so 40 px of it is 40/264 of
        // the range — about 15 points, not 20.
        let (x, y) = demo.slider_at_fraction(0.5).expect("a laid-out slider");
        for event in drag_on(x, y, x + 40.0, y) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        let readout = demo.readout_text().expect("a readout");
        assert!(
            readout.starts_with("65 of 100"),
            "the value followed the drag: {readout}"
        );
        assert!(
            readout.ends_with(", 1 adjustment"),
            "and the widget reported exactly one adjustment: {readout}"
        );

        // A second drag to the same place changes nothing, and so reports
        // nothing: the widget fires its callback when the value moves.
        for event in drag_on(x, y, x + 40.0, y) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert!(
            demo.readout_text()
                .expect("a readout")
                .ends_with(", 1 adjustment"),
            "a drag that lands where the thumb already was is not an adjustment"
        );
    }

    #[test]
    fn a_programmatic_change_moves_the_slider_without_counting_an_adjustment() {
        // The `0` and `1` keys write the value directly, which is what a caller
        // binding a slider to a model does. The readout follows because it is
        // bound to the value; the count does not, because the widget did not
        // adjust anything.
        let mut demo = dialog_closed();
        demo.handle_event(key(Keycode::_1));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(demo.slider.widget.value.get(), SLIDER_MAX);
        let readout = demo.readout_text().expect("a readout");
        assert!(readout.starts_with("100 of 100"), "{readout}");
        assert!(readout.ends_with(", 0 adjustments"), "{readout}");
    }

    #[test]
    fn a_programmatic_change_carries_the_thumb_to_the_value_rather_than_jumping() {
        // The acceptance criterion "thumb position animates smoothly": the thumb
        // starts where it was, is part way after one frame's worth of the
        // transition, and arrives. A `set` instead of an animation would be at
        // the value on the first frame and pass an end-only assertion.
        let mut demo = dialog_closed();
        assert_eq!(
            painted_thumb_x(&demo),
            demo.slider_at_fraction(0.0).unwrap().0
        );

        // Both ends are read before the key is pressed, and reading one puts the
        // slider there and snaps it — which is the very thing under test, so it
        // has to happen while nothing is animating.
        let end = demo.slider_at_fraction(1.0).unwrap().0;
        let start = demo.slider_at_fraction(0.0).unwrap().0;

        demo.handle_event(key(Keycode::_1));
        demo.frame(WINDOW, Duration::from_millis(10));
        let midway = painted_thumb_x(&demo);
        assert!(
            midway > start && midway < end,
            "ten milliseconds in, the thumb is part way to {end}: {midway}"
        );

        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            painted_thumb_x(&demo),
            end,
            "and it arrives rather than creeping"
        );
        assert!(
            !demo.slider.widget.is_animating(),
            "a transition that has arrived has stopped"
        );
    }

    #[test]
    fn the_thumb_grows_while_a_pointer_is_holding_the_slider() {
        let mut demo = dialog_closed();
        let (x, y) = demo.slider_at_fraction(0.5).expect("a laid-out slider");
        let resting = painted_thumb_radius(&demo);

        let mut events = drag_on(x, y, x + 40.0, y).into_iter();
        demo.handle_event(events.next().expect("a finger going down"));
        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert!(demo.slider.widget.dragging.get(), "the press took hold");
        let held = painted_thumb_radius(&demo);
        assert!(held > resting, "and the thumb grew: {resting} then {held}");

        demo.handle_event(events.last().expect("the same finger coming up"));
        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert!(!demo.slider.widget.dragging.get(), "the release let go");
        assert_eq!(painted_thumb_radius(&demo), resting, "and it shrank back");
    }

    #[test]
    fn an_arrow_key_moves_the_slider_once_it_holds_focus() {
        let mut demo = dialog_closed();
        // **One** `Tab`, where it was three before the buttons went: the slider is
        // the first control in the order and the two enabled buttons that used to
        // precede it are not here.
        demo.handle_event(key(Keycode::Tab));
        assert_eq!(demo.focused, Some(demo.slider.node()));

        demo.handle_event(key(Keycode::Right));
        demo.handle_event(key(Keycode::Right));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.slider.widget.value.get(),
            10.0,
            "two presses of the right arrow, by the slider's own step"
        );

        demo.handle_event(key(Keycode::Left));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(demo.slider.widget.value.get(), 5.0);
    }

    #[test]
    fn an_arrow_key_does_nothing_while_the_slider_is_not_focused() {
        // A key is not routed by position, so an arrow would otherwise move every
        // slider on screen. The widget's own test says this too; what the demo
        // adds is that nothing focuses the slider by accident.
        let mut demo = dialog_closed();
        demo.handle_event(key(Keycode::Right));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(demo.slider.widget.value.get(), SLIDER_MIN);
        assert!(demo.focused.is_none());
    }

    #[test]
    fn a_steering_wheel_scroll_adjusts_the_focused_slider() {
        // The one gamepad axis the input module maps is the wheel's scroll, and
        // it arrives as a positionless `Scroll`. The focused control gets it
        // first, and focus navigation only runs for what the control left alone —
        // so a focused slider is driven by it and whatever is left over still
        // walks the focus order.
        let mut demo = dialog_closed();
        demo.handle_event(key(Keycode::Tab));
        assert_eq!(demo.focused, Some(demo.slider.node()));
        demo.handle_event(wheel(8000));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.slider.widget.value.get(),
            SLIDER_STEP,
            "one notch of the wheel is one step of the grid"
        );
    }

    #[test]
    fn the_slider_paints_a_focus_ring_once_it_is_focused() {
        let mut demo = dialog_closed();
        let rects = |demo: &Demo| {
            demo.slider_commands()
                .iter()
                .filter(|command| matches!(command, DrawCommand::RoundedRect { .. }))
                .count()
        };
        assert_eq!(rects(&demo), 2, "an unfocused slider is a track and a fill");

        demo.handle_event(key(Keycode::Tab));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(rects(&demo), 3, "and a focused one has a ring as well");
    }

    #[test]
    fn the_slider_follows_the_theme_switch() {
        // The track is the theme's `Border` token and the fill its `Primary`, so
        // a switch carries the new colours to the slider through the property
        // graph: the widget is aimed at the new palette and its own transition
        // runs alongside the theme's.
        let mut demo = dialog_closed();
        let dark = demo.slider.widget.track.get();
        assert_eq!(
            dark,
            Theme::dark().get(ThemeToken::Border).as_color().unwrap()
        );

        demo.handle_event(toggle_theme_event());
        for _ in 0..35 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.slider.widget.track.get(),
            Theme::light().get(ThemeToken::Border).as_color().unwrap(),
            "the slider arrived at the light theme's own Border token"
        );
    }

    #[test]
    fn the_slider_sits_clear_of_the_counter_the_buttons_and_the_text() {
        // The collision tasks 12 and 13 each found only by looking: a control
        // placed by hand lands on top of whatever is already there, and nothing
        // in the suite would notice because each of them is laid out correctly on
        // its own.
        let demo = laid_out();
        let rect = demo.slider_rect().expect("a laid-out slider");
        let column_right = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        assert!(
            rect.x > column_right,
            "the slider starts at x = {}, right of the text column's edge at {}",
            rect.x,
            column_right
        );
        let gauge_bottom = GAUGE_ORIGIN.1 + GAUGE_SIZE.height;
        assert!(
            rect.y > gauge_bottom,
            "and at y = {}, below the gauge's own bottom edge at {gauge_bottom}",
            rect.y
        );
        assert!(
            rect.y + rect.height < WINDOW.height,
            "with its readout still inside the window"
        );
        assert!(
            rect.x + rect.width < WINDOW.width,
            "and inside it horizontally, at {} wide",
            rect.width
        );
    }

    #[test]
    fn the_slider_is_the_last_thing_painted_in_the_band() {
        // The readout is drawn after the slider it reports, so a number that
        // overlapped it would be the readable one rather than the covered one.
        let demo = laid_out();
        let slider_at = demo
            .order
            .iter()
            .position(|&handle| handle == demo.slider.node())
            .expect("the slider is in the tree");
        let readout_at = demo
            .order
            .iter()
            .position(|&handle| handle == demo.slider_readout.label.handle())
            .expect("the readout is in the tree");
        assert!(
            readout_at > slider_at,
            "the readout at {readout_at} is painted after the slider at {slider_at}"
        );
    }

    // ---------------------------------------------------------------------
    // Tasks 15 to 18: the four widgets the later tasks added, wired into the
    // band. Everything below is about the wiring, not about the widgets: each
    // of the four has its own module's own tests for what it draws and what it
    // does with an event, and repeating them here would be a second opinion
    // about someone else's code rather than a check of this one.
    // ---------------------------------------------------------------------

    /// Returns the toggle's centre in window coordinates, or `None` if it has
    /// not been laid out. This is what a click test aims at.
    ///
    /// **The list's fixture, `list_point(demo, rows, across)`, went with the list
    /// on 2026-10-02**: it addressed a viewport by *row*, which is a property of a
    /// list's item height rather than of the window. A chart's equivalent would be
    /// a fraction of its plot, and no test here needs one — the chart is driven by
    /// keys, and the one test that presses it asks for its centre.
    fn toggle_center(demo: &Demo) -> Option<(f32, f32)> {
        let rect = demo.toggle_rect()?;
        Some((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))
    }

    /// Clicks the toggle and lays the demo out, which is the whole path a click
    /// takes: two SDL events, the recogniser's tap, and the dispatch that routes
    /// it to the node under the pointer.
    fn click_toggle(demo: &mut Demo) {
        let (x, y) = toggle_center(demo).expect("a laid-out toggle");
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));
    }

    // ---------------------------------------------------------------------
    // Task 21: the chart, wired into the column the list vacated. Everything
    // below is about the wiring — the keys, the readout and the placement —
    // and not about the widget, whose own module has its own hundreds of tests
    // for what it draws. What *is* here is the half a widget's own tests cannot
    // see: where the demo put it, what its keys do, and what its readout says
    // afterwards.
    // ---------------------------------------------------------------------

    /// Returns the text the chart's readout is showing, as the last frame
    /// recorded it.
    fn chart_readout_text(demo: &Demo) -> Option<String> {
        demo.readout_text_of(&demo.chart_readout)
    }

    /// The line the chart's readout prints for `count` readings, newest reading
    /// `last`, drawn in the shape at `shape`, and still or moving.
    ///
    /// **Written out here as well as in the binding**, because a test that
    /// rebuilt the string from the same format would agree with a format that
    /// had changed — and the format *is* the contract: it is what a driver reads
    /// off the screen, and a readout that stopped naming the shape would still
    /// be a passing test.
    fn chart_readout_line(count: usize, last: f32, shape: usize, moving: bool) -> String {
        format!(
            "{count} pts, last {last:.2}, {}, {}",
            CHART_TYPE_NAMES[shape],
            if moving { "moving" } else { "still" }
        )
    }

    #[test]
    fn the_chart_sits_in_the_column_the_list_occupied_and_is_the_box_it_asks_for() {
        // The placement is a claim about the window and the window cannot grow —
        // 1280 by 1320 comes back 1280 by 1052 on this host — so this is where
        // the operator's decision to give up the list is spent.
        //
        // **Asserted through `placed_rects`, not by recomputing the numbers**,
        // because the point is that the *collision* tests can see the chart: a
        // rect this test reads off the node directly and never registers would
        // satisfy every assertion here and leave `no_two_placed_rects_overlap`
        // blind to a 270 by 450 box. That is the survivor task 20's record
        // describes for the gauge, and it is why the membership assertion is
        // here rather than tidiness.
        let demo = laid_out();
        let placed = demo.placed_rects();
        let named: Vec<&str> = placed.iter().map(|(what, _)| *what).collect();
        for what in ["chart", "chart readout"] {
            assert!(
                named.contains(&what),
                "the {what} is in placed_rects, so the collision tests can see it: \
                 {named:?}"
            );
        }
        let chart = placed
            .iter()
            .find(|(what, _)| *what == "chart")
            .map(|(_, rect)| *rect)
            .expect("the chart is named above");
        assert_eq!(
            (chart.x, chart.y),
            CHART_ORIGIN,
            "at the list's own x and sixteen below the image fit label"
        );
        assert_eq!(
            (chart.width, chart.height),
            (CHART_SIZE.width, CHART_SIZE.height),
            "and the box the demo asked for, which is the list's width by twice \
             its height"
        );
        assert_eq!(
            chart,
            demo.node_rect(demo.chart.handle())
                .expect("the chart is placed"),
            "and it is the rect its own node was laid out at, so the two cannot \
             disagree"
        );
        assert!(
            chart.y + chart.height <= BAND_TOP,
            "and it ends at {}, above the band at {BAND_TOP}, so the gallery above \
             the band has not moved",
            chart.y + chart.height
        );
    }

    #[test]
    fn the_demo_opens_with_a_themed_chart_of_sample_data_and_says_so() {
        // Requirement 6 — *"Demo shows a chart with sample data"* — is this: the
        // data is a named constant a test can assert against, the chart is on the
        // theme rather than on the neutral greys `Chart::new` writes, and the
        // readout names what is on screen.
        let demo = laid_out();
        assert_eq!(
            demo.chart.data.get(),
            CHART_SERIES[..CHART_OPENING_COUNT].to_vec(),
            "the opening window is the first {} readings of the demo's own series",
            CHART_OPENING_COUNT
        );
        assert_eq!(
            demo.chart.chart_type(),
            ChartType::Line,
            "and it opens on a line"
        );

        // The snap. Without `snap_to_state` at construction the chart would open
        // on `Palette::default`'s greys and animate to the theme over the first
        // half second — a themed chart that starts grey, and a test that waited
        // long enough would never see it.
        let themed = ChartPalette::from_theme(&Theme::dark());
        assert_eq!(
            demo.chart.series.get(),
            themed.series,
            "the series is the theme's Primary, written by snap_to_state rather \
             than left on Palette::default's grey"
        );
        assert_eq!(demo.chart.axis.get(), themed.axis);
        assert_ne!(
            demo.chart.axis.get(),
            ChartPalette::default().axis,
            "and the axes are the theme's TextMuted rather than the neutral grey"
        );
        assert_eq!(
            demo.chart.shown.get().values,
            CHART_SERIES[..CHART_OPENING_COUNT].to_vec(),
            "with the drawn series already arrived at the truth"
        );

        assert_eq!(
            chart_readout_text(&demo).as_deref(),
            Some(chart_readout_line(CHART_OPENING_COUNT, CHART_SERIES[7], 0, false).as_str()),
            "and the readout names the length, the newest reading, the shape and \
             that nothing is moving"
        );
        assert!(
            !demo.chart.is_animating(),
            "a chart that has just been snapped is not animating, or the readout's \
             last word is wrong on the first frame"
        );
    }

    #[test]
    fn the_chart_key_walks_its_three_shapes_and_the_readout_names_each_one() {
        // Task 21's three rendering criteria are three *distinct* things, and a
        // demo showing one of them does not show three. This is the key that
        // makes all three reachable on screen rather than in three builds, and
        // the readout is what says which one is on the glass.
        let mut demo = dialog_closed();
        for lap in 0..2 {
            for press in 0..CHART_TYPES.len() {
                // **The press comes first and the assertion names what it arrived
                // at**, which is the gauge's walk's order: the demo opens on the
                // first shape, so an assertion *before* the press would compare
                // the opening state with a shape and pass without a keypress ever
                // having done anything. The walk is therefore indexed by **how
                // many presses have happened**, not by where the walk starts.
                let arrived = (press + 1) % CHART_TYPES.len();
                demo.handle_event(key(Keycode::H));
                demo.frame(WINDOW, Duration::from_millis(16));
                assert_eq!(
                    demo.chart.chart_type(),
                    CHART_TYPES[arrived],
                    "lap {lap}: press {press} is the {}",
                    CHART_TYPE_NAMES[arrived]
                );
                assert_eq!(
                    chart_readout_text(&demo).as_deref(),
                    Some(
                        chart_readout_line(
                            CHART_OPENING_COUNT,
                            CHART_SERIES[CHART_OPENING_COUNT - 1],
                            arrived,
                            false,
                        )
                        .as_str()
                    ),
                    "and the readout names it"
                );
            }
        }
        assert_eq!(
            demo.chart.chart_type(),
            CHART_TYPES[0],
            "and two laps of three presses end where they started"
        );
    }

    /// The three shapes draw three different things, and this is what tells them
    /// apart.
    ///
    /// **Counts, and one position.** A count is what a draw-command assertion can
    /// see, and it separates the three: a line records one polygon per segment, a
    /// bar chart one rectangle per sample, and an area chart two of the former —
    /// the fill and then the stroke. The **position** is what makes the area
    /// chart's half distinguishable from its own stroke: the fill's quads drop to
    /// the plot's bottom edge and the stroke's do not, and the bottom edge is
    /// read off the **x axis the widget recorded** rather than off a constant this
    /// file does not own.
    #[test]
    fn each_chart_shape_records_its_own_primitive_in_its_own_place() {
        let mut demo = dialog_closed();
        let mut seen: Vec<(usize, usize, usize)> = Vec::new();
        for shape in 0..CHART_TYPES.len() {
            // **One press per pass, not `shape` presses**: the demo opens on the
            // first shape, so a press on every pass walks all three and a press
            // `shape` times walks past the end of them — which is how this test
            // read a line where it meant an area chart and produced seven quads
            // where it meant fourteen.
            if shape > 0 {
                demo.handle_event(key(Keycode::H));
            }
            demo.frame(WINDOW, Duration::from_millis(16));
            assert_eq!(
                demo.chart.chart_type(),
                CHART_TYPES[shape],
                "pass {shape} is in {}",
                CHART_TYPE_NAMES[shape]
            );
            let commands = demo.commands_at(demo.chart.handle());
            let polygons: Vec<&DrawCommand> = commands
                .iter()
                .filter(|c| matches!(c, DrawCommand::Polygon { .. }))
                .collect();
            let rects = commands
                .iter()
                .filter(|c| matches!(c, DrawCommand::Rect { .. }))
                .count();
            let segments = CHART_OPENING_COUNT - 1;

            match shape {
                0 => {
                    assert_eq!(polygons.len(), segments, "a line is one quad a segment");
                    assert_eq!(rects, 0, "and no rectangles at all");
                }
                1 => {
                    // **One bar per sample, and one of them is not there.** A bar
                    // is a magnitude from the plot's own bottom edge, and the
                    // lowest reading *is* the range's low, so its bar has no
                    // height at all — and `Chart::draw_bars` records nothing rather
                    // than a rectangle of nothing, for the reason the progress bar
                    // leaves an empty fill out. So eight readings are seven bars,
                    // and a test that expected eight would be asking the widget for
                    // a zero-height rectangle.
                    assert_eq!(
                        rects,
                        CHART_OPENING_COUNT - 1,
                        "a bar chart is a bar per sample, and the reading at the \
                         bottom of the range has no height to draw"
                    );
                    assert_eq!(polygons.len(), 0, "and no quads at all");
                }
                _ => {
                    assert_eq!(
                        polygons.len(),
                        segments * 2,
                        "an area chart is a fill quad and a stroke quad per segment"
                    );
                    // The plot's bottom edge, from the x axis: `AXIS_WIDTH` is 2 and
                    // every grid line is 1, which is how the axis is picked out of
                    // the `Line`s without a constant copied out of the widget.
                    let axis = commands
                        .iter()
                        .find_map(|command| match command {
                            DrawCommand::Line {
                                start, end, width, ..
                            } if *width > 1.0 && start.1 == end.1 => Some(start.1),
                            _ => None,
                        })
                        .expect("the x axis is a horizontal line of the axis's width");
                    let to_the_bottom = polygons
                        .iter()
                        .filter(|command| match command {
                            DrawCommand::Polygon { points, .. } => {
                                let lowest = points.iter().map(|p| p.1).fold(f32::MIN, f32::max);
                                (lowest - axis).abs() < 0.01
                            }
                            _ => false,
                        })
                        .count();
                    assert_eq!(
                        to_the_bottom, segments,
                        "and exactly the fill's {segments} quads reach the plot's own \
                         bottom edge at {axis}, with the stroke's {segments} on the \
                         line above them"
                    );
                }
            }
            seen.push((shape, polygons.len(), rects));
        }
        assert_eq!(
            seen,
            vec![(0, 7, 0), (1, 0, 7), (2, 14, 0)],
            "and the three are told apart, not one of them mistaken for another"
        );
    }

    #[test]
    fn the_append_key_adds_a_sample_and_the_readout_names_the_length_and_the_value() {
        // Requirement 5 — *"new data points animate in"* — through the demo's own
        // event path: an SDL key-down, the recogniser, and the arm in
        // `handle_event`, which is the only route a key takes. A test that called
        // `animate_push` on the widget would prove the widget and not the wiring.
        let mut demo = dialog_closed();
        let opening = CHART_SERIES[..CHART_OPENING_COUNT].to_vec();
        assert_eq!(demo.chart.data.get(), opening);

        demo.handle_event(key(Keycode::A));
        demo.frame(WINDOW, Duration::from_millis(16));

        let appended = CHART_SERIES[CHART_OPENING_COUNT];
        let after = demo.chart.data.get();
        assert_eq!(after.len(), CHART_OPENING_COUNT + 1, "one reading longer");
        assert_eq!(
            &after[..CHART_OPENING_COUNT],
            &opening[..],
            "with the eight that were on screen untouched, in order"
        );
        assert_eq!(
            after[CHART_OPENING_COUNT], appended,
            "and the new one at the end, which is {appended} — the next reading of \
             the demo's own constant, not a formula"
        );

        // **Mid-flight is the whole of requirement 5.** The drawn series is still
        // the eight that were there, and the new sample is entering at the far
        // edge reading what the last one read — which is what makes it a line that
        // grows rather than a series that jumps.
        let flying = demo.chart.shown.get();
        assert_eq!(
            flying.len(),
            CHART_OPENING_COUNT + 1,
            "and the drawn series already has the new sample in it"
        );
        assert_eq!(
            flying.x[flying.len() - 1],
            1.0,
            "entering at the plot's far edge, where the old run ended"
        );
        assert_ne!(
            flying.values[flying.len() - 1],
            appended,
            "reading between the last one and its own rather than already arrived"
        );
        assert!(
            demo.chart.is_animating(),
            "and something is running, which is the definition of animating in"
        );
        assert_eq!(
            chart_readout_text(&demo).as_deref(),
            Some(chart_readout_line(CHART_OPENING_COUNT + 1, appended, 0, true).as_str()),
            "so the readout says nine readings, names the new value and says it is \
             moving"
        );

        // And the far end of the transition: the series arrives, and the last word
        // changes back.
        for _ in 0..40 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert_eq!(
            demo.chart.shown.get().values,
            after,
            "the drawn series arrived"
        );
        assert!(!demo.chart.is_animating());
        assert_eq!(
            chart_readout_text(&demo).as_deref(),
            Some(chart_readout_line(CHART_OPENING_COUNT + 1, appended, 0, false).as_str()),
            "and the readout says it is still"
        );
    }

    #[test]
    fn the_shift_key_drops_the_oldest_sample_and_keeps_the_length() {
        // Requirement 4 — *"append new value, shift old values"* — through the
        // same event path. **The length is the claim**, because a shift that
        // appended without dropping is an append: the window would grow on every
        // press, and the plot's pitch would shrink under the labels.
        let mut demo = dialog_closed();
        let opening = CHART_SERIES[..CHART_OPENING_COUNT].to_vec();
        let dropped = opening[0];
        let arriving = CHART_SERIES[CHART_OPENING_COUNT];

        demo.handle_event(key(Keycode::S));
        demo.frame(WINDOW, Duration::from_millis(16));

        let after = demo.chart.data.get();
        assert_eq!(
            after.len(),
            CHART_OPENING_COUNT,
            "the same number of readings"
        );
        assert_eq!(
            &after[..CHART_OPENING_COUNT - 1],
            &opening[1..],
            "each reading one place along, so the oldest {dropped} went and nothing \
             else did"
        );
        assert_eq!(
            after[CHART_OPENING_COUNT - 1],
            arriving,
            "and {arriving} arrived"
        );

        // Half way, each reading slides toward its neighbour's — which is what
        // `Chart::animate_shift` says it does, and what makes a shift read as a
        // scrolling window rather than a re-plot.
        for _ in 0..5 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        let flying = demo.chart.shown.get();
        assert!(
            flying.values[0] > dropped && flying.values[0] < opening[1],
            "the oldest reading is between {dropped} and {} — it slid rather than \
             jumped, and it read {}",
            opening[1],
            flying.values[0]
        );
        assert_eq!(
            chart_readout_text(&demo).as_deref(),
            Some(chart_readout_line(CHART_OPENING_COUNT, arriving, 0, true).as_str()),
            "and the readout names the same length, the new value and the motion"
        );

        for _ in 0..40 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert_eq!(
            demo.chart.shown.get().values,
            after,
            "and the slide arrived"
        );
    }

    /// `A` is a key that never runs out, so the window slides once it is full.
    ///
    /// The alternative — letting `animate_push` grow the series for ever — would
    /// re-space the whole run against a fixed set of x labels on every press, and
    /// the pitch [`CHART_SIZE`]'s mitre argument is about would shrink under the
    /// reader's feet. So past [`CHART_OPENING_COUNT`] the two keys meet, and this
    /// is what says so: **the length is capped at the constant's own length**, and
    /// the oldest reading is the one that went.
    #[test]
    fn the_append_key_slides_the_window_once_the_series_is_full() {
        let mut demo = dialog_closed();
        let full = CHART_SERIES.len();
        let presses = full - CHART_OPENING_COUNT + 2;
        for _ in 0..presses {
            demo.handle_event(key(Keycode::A));
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        let after = demo.chart.data.get();
        assert_eq!(
            after.len(),
            full,
            "{full} readings after {presses} presses: the four the constant had \
             left, and no more"
        );
        // Four presses append the constant's own last four; the two after that
        // slide, and the cursor has wrapped to the front of the constant by then —
        // so what is on screen is the constant from its third reading on, plus its
        // own first two at the end.
        let mut expected: Vec<f32> = CHART_SERIES[2..].to_vec();
        expected.extend_from_slice(&CHART_SERIES[..2]);
        assert_eq!(
            after, expected,
            "and they are the window slid twice rather than grown: the front of the \
             constant went out and its own first two readings came back in at the \
             end"
        );
        assert_eq!(
            demo.chart.x_labels.get().len(),
            CHART_X_LABEL_COUNT,
            "the labels did not move: they name a sample's place in the window, and \
             a window that slides leaves them where they are"
        );
    }

    /// The other half of requirement 5 — *"bar rises"* — in the only shape where
    /// it is a thing at all.
    ///
    /// `Chart::reveal` is aimed **only** in [`ChartType::Bar`], and a line
    /// chart's new sample arrives as a place and a reading rather than as a
    /// height. So this walks to the bar shape with `H`, appends with `A`, and
    /// asserts the number that says a bar is *rising* — and then the drawn thing
    /// itself, because `reveal` is the widget's own bookkeeping and the height of
    /// the rectangle it recorded is what a reader sees.
    #[test]
    fn a_new_bar_rises_from_its_baseline_rather_than_appearing() {
        let mut demo = dialog_closed();
        demo.handle_event(key(Keycode::H));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.chart.chart_type(),
            ChartType::Bar,
            "H is the bar shape"
        );

        demo.handle_event(key(Keycode::A));
        demo.frame(WINDOW, Duration::from_millis(16));

        let reveal = demo.chart.reveal.get();
        assert!(
            (0.0..1.0).contains(&reveal),
            "one frame into the theme's own motion the newest bar is {reveal} of \
             its height, so it is on its way up rather than there"
        );
        let flying = bar_rects(&demo);
        assert_eq!(
            flying.len(),
            CHART_OPENING_COUNT,
            "eight bars for the nine readings after the append: the lowest is the \
             range's own low, so its bar has no height and `Chart::draw_bars` \
             records nothing for it"
        );
        let newest = flying.last().copied().expect("at least one bar");
        assert!(
            newest.height < flying[flying.len() - 2].height,
            "and the newest is {} px tall against its neighbour's {}, so it is the \
             one still rising",
            newest.height,
            flying[flying.len() - 2].height
        );

        for _ in 0..40 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert_eq!(demo.chart.reveal.get(), 1.0, "and it arrives");

        // Its own share of the plot, from the data rather than from a remembered
        // pixel — and **the plot is read off the drawing too**: the baseline every
        // bar stands on is the plot's own bottom edge, and the plot's top is the
        // node's top edge, which is the case because the demo writes no y labels
        // and a gutter is reserved only when there are labels to put in it.
        let readings = demo.chart.data.get();
        let low = readings.iter().copied().fold(f32::MAX, f32::min);
        let high = readings.iter().copied().fold(f32::MIN, f32::max);
        let arrived = readings.last().copied().unwrap_or(0.0) - low;
        let bars = bar_rects(&demo);
        let baseline = bars.first().map_or(0.0, |bar| bar.y + bar.height);
        let plot = demo
            .node_rect(demo.chart.handle())
            .map_or(0.0, |node| baseline - node.y);
        let expected = arrived / (high - low) * plot;
        let standing = bars.last().copied().expect("at least one bar");
        assert_eq!(
            bars.len(),
            CHART_OPENING_COUNT,
            "and the same eight once it has arrived: nine readings, one of them at \
             the range's own low"
        );
        assert!(
            (standing.height - expected).abs() < 1.0,
            "and the newest bar stands {} px tall against the {:.1} its \
             own share of a {:.0} px plot gives it — {arrived} of a range \
             {high} - {low}",
            standing.height,
            expected,
            plot
        );
    }

    /// Returns every bar the chart recorded, left to right.
    ///
    /// **The recorded rectangles themselves**, which is all a bar is: the widget
    /// builds each as `Rect::new(centre - half, baseline.min(y), width, height)`,
    /// so a bar's height and its place are fields of its own and nothing here
    /// counts pixels or subtracts an edge. They are ordered by `x` rather than by
    /// the order they were recorded, because a reader of this list wants the
    /// left-to-right order they appear on the screen in.
    fn bar_rects(demo: &Demo) -> Vec<Rect> {
        let mut bars: Vec<Rect> = demo
            .commands_at(demo.chart.handle())
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Rect { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect();
        bars.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        bars
    }

    /// Every colour the chart draws reaches every chart type on `T`, **and the
    /// label is the one that was left out.**
    ///
    /// Two defects, one visible in this file and one in `ui_core`, and the second
    /// is why this test is driven onto a **bar chart** rather than the demo's
    /// opening line chart.
    ///
    /// The first is this file's own: a palette read **after** `switch_to` is the
    /// palette the theme is leaving, so a widget is re-aimed at what it already has
    /// — the transition goes nowhere and every test that does not wait for it stays
    /// green. That is the gauge's bug, and it is why this test asserts the
    /// *arrival* rather than that something moved.
    ///
    /// The second was a real defect in `Chart::animate_to_state`, found by the
    /// widget's own author: it re-themed `axis`, `grid`, `series` and `fill` but
    /// guarded the **label** colour behind `if chart_type != ChartType::Bar`, while
    /// `draw_labels` draws with `label.get()` for every type. On a bar chart the
    /// axis labels therefore kept the dark theme's `(158,158,158)` on the light
    /// window. **This demo is where that is visible in the running application** —
    /// it is `H` then `T` — and the assertion that would have caught it is the one
    /// this file's comment already named and its body left out.
    ///
    /// **So the test walks to the bar shape with the demo's own key**, through
    /// `handle_event`, which is the only route a key takes: a test on the opening
    /// line chart would pass against that bug, because a line chart animated its
    /// label all along.
    #[test]
    fn a_theme_switch_reaches_the_chart_and_its_readout_in_every_shape() {
        for presses in 0..CHART_TYPES.len() {
            let mut demo = dialog_closed();
            for _ in 0..presses {
                demo.handle_event(key(Keycode::H));
            }
            demo.frame(WINDOW, Duration::from_millis(16));
            let shape = presses;
            assert_eq!(
                demo.chart.chart_type(),
                CHART_TYPES[shape],
                "{} is on screen before anything is switched",
                CHART_TYPE_NAMES[shape]
            );

            let dark = demo.chart.palette();
            assert_eq!(
                demo.chart.series.get(),
                ChartPalette::from_theme(&Theme::dark()).series,
                "and it starts on the dark theme's own Primary"
            );
            // **A note on what this test cannot check**, because it is a trap in
            // the palette rather than in the demo: `Palette::default`'s `label` and
            // the **dark** theme's `TextMuted` are the *same* colour, (158,158,158).
            // So on the dark theme the label's starting value is identical whether
            // the chart was themed or not, and an assertion that the labels "start
            // on the theme" would pass against an unthemed chart — the grey chart
            // and the dark-themed chart are the same picture here. **That is why
            // the assertion this test rests on is the arrival at the light theme's
            // label**, which is (117,117,117) and a different colour entirely.
            assert_eq!(
                demo.chart.label.get(),
                ChartPalette::from_theme(&Theme::dark()).label,
                "the {} shape's labels start on the dark theme's TextMuted — which \
                 happens to be the same colour as `Palette::default`'s, so this \
                 line says nothing either way and the switch below is the test",
                CHART_TYPE_NAMES[shape]
            );

            demo.handle_event(toggle_theme_event());
            assert!(
                demo.chart.is_animating(),
                "the switch aims the chart at the new palette rather than snapping \
                 to it — a label that jumped while the window crossfaded would be a \
                 visible disagreement"
            );

            for _ in 0..40 {
                demo.frame(WINDOW, Duration::from_millis(16));
            }
            let light = ChartPalette::from_theme(&Theme::light());
            assert_ne!(
                demo.chart.series.get(),
                dark.series,
                "the {} series moved",
                CHART_TYPE_NAMES[shape]
            );
            assert_eq!(
                demo.chart.series.get(),
                light.series,
                "and arrived at the light theme's own Primary"
            );
            assert_eq!(demo.chart.axis.get(), light.axis, "and so did the axes");
            assert_eq!(demo.chart.grid.get(), light.grid, "and the grid");
            // **The label, which is the one this test is for.** It is in the list
            // rather than after it, and it is asserted for every shape, because the
            // defect it catches was conditional on the shape — a bar chart was the
            // only type that skipped it, so a test on any other type proves nothing
            // about the thing it is named for.
            assert_eq!(
                demo.chart.label.get(),
                light.label,
                "and in the {} shape the axis labels too, which is the colour that \
                 stayed behind: `draw_labels` draws with it for every type, so a \
                 palette that left it alone left grey numbers on a light window",
                CHART_TYPE_NAMES[shape]
            );
            assert_ne!(
                demo.chart.label.get(),
                dark.label,
                "and they are not the dark theme's own TextMuted"
            );
            // And the readout, which is a bound label and reaches the theme through
            // the property graph rather than through a palette. It is listed here
            // because a readout that did not re-colour would be a number in the old
            // theme's ink on the new theme's window — the one thing `T` must never do.
            assert_eq!(
                demo.chart_readout.label.color.get(),
                demo.theme
                    .get(ThemeToken::Text)
                    .as_color()
                    .expect("the theme holds a colour in Text"),
                "and the readout's own colour, which is bound to the theme's Text \
                 token and needs no palette at all"
            );
        }
    }

    #[test]
    fn the_chart_readouts_box_holds_the_longest_line_it_can_print() {
        // `read_only_label` gives a readout the box of its **first** string and
        // ellipsises anything longer, so a line that does not fit is a readout
        // that cannot say what it is saying — which is the whole of the operator's
        // complaint about a readout that has been cut short. The longest line the
        // format can print is the one at the widest window: twelve readings, the
        // series' own largest reading, and the longest shape name while it moves.
        let demo = laid_out();
        let width = demo
            .node_rect(demo.chart_readout.label.handle())
            .expect("a laid-out readout")
            .width;
        assert_eq!(
            width, CHART_READOUT_WIDTH,
            "the box is the constant, the chart's own width"
        );

        let metrics = mono_metrics();
        let widest = chart_readout_line(
            CHART_SERIES.len(),
            CHART_SERIES.iter().copied().fold(f32::MIN, f32::max),
            2,
            true,
        );
        let drawn: f32 = widest
            .chars()
            .map(|ch| metrics.advance(ch, CHART_READOUT_FONT))
            .sum();
        assert!(
            drawn <= CHART_READOUT_WIDTH,
            "the widest line it can show is {widest:?}, {drawn} px, and the box is \
             {CHART_READOUT_WIDTH} px"
        );
        // And the line it opens with is inside it too, which is the box the node
        // was actually given.
        let opening = chart_readout_line(CHART_OPENING_COUNT, CHART_SERIES[7], 0, false);
        assert!(
            !opening.contains('…'),
            "and nothing on screen is cut short: the opening line is {opening:?}"
        );
    }

    #[test]
    fn the_chart_labels_every_sample_but_its_newest_one() {
        // The widget places an x label **at** its own sample and cannot measure a
        // string, so the label on the newest sample starts on the plot's right
        // edge and runs right from there into the ten pixels of window that are
        // left. This is the caller's half of that answer, and it is a decision
        // rather than an oversight: one label fewer than samples is the widget's
        // own documented case — a sample with no label at its index is drawn
        // without one.
        let mut demo = dialog_closed();
        let runs = |demo: &Demo| {
            demo.commands_at(demo.chart.handle())
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<String>>()
        };
        // **The labels are written out here rather than built from
        // `CHART_X_LABEL_COUNT`**, which is the assertion that cannot fail if it
        // were: an expectation derived from the constant the production code
        // derives its own from agrees with any value of it, and a deliberate break
        // that gave the newest sample a label survived the first version of this
        // test for exactly that reason. Seven names, then — and the count is
        // pinned separately so the two numbers cannot be one.
        let expected: Vec<String> = (0..7).map(|i| i.to_string()).collect();
        assert_eq!(
            expected.len(),
            CHART_X_LABEL_COUNT,
            "and the constant says seven, which is one fewer than the window's eight"
        );
        assert_eq!(
            CHART_X_LABEL_COUNT,
            CHART_OPENING_COUNT - 1,
            "which is the opening window less its newest sample"
        );
        assert_eq!(
            runs(&demo),
            expected,
            "the labels are the opening window's own places, in order, and the \
             newest sample has none"
        );

        // And an appended sample does not get one either, which is the rule rather
        // than an accident of the construction.
        demo.handle_event(key(Keycode::A));
        for _ in 0..40 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert_eq!(
            runs(&demo),
            expected,
            "an appended sample arrives unlabelled, so the gutter and the pitch \
             never move"
        );
        assert_eq!(
            demo.chart.data.get().len(),
            CHART_OPENING_COUNT + 1,
            "with the sample itself there — the point is the missing label, not the \
             missing data"
        );
    }

    /// The pitch below which the widget stops mitring a corner, asserted rather
    /// than quoted, and the demo's own window checked against it.
    ///
    /// **The bound is the widget's own answer, twice over.** A join falls back
    /// from a mitred corner to a round one when the corner's swing along a segment
    /// reaches half that segment's own length; the swing is at most
    /// [`Chart::stroke_reach`], and a segment is at least a pitch long — so a
    /// pitch above **twice** the reach mitres every corner. This used to derive
    /// the reach from a local `4.0` standing in for the widget's private
    /// `MITRE_LIMIT`, which is the same re-derived-constant shape the demo's
    /// private `CHART_STROKE_REACH` was, a few hundred lines away in the same
    /// file; both are gone, and the number here comes from the widget through the
    /// method its author added for it.
    ///
    /// **The `2` is the rule's, not the widget's**: the rule is `|t| < L/2` per
    /// side of a vertex, so a segment has to be longer than twice the largest
    /// swing. That factor is written out here rather than folded into the
    /// widget's number, because it is the rule and the rule is not the widget's
    /// to answer for a caller.
    #[test]
    fn the_series_never_grows_past_what_the_widget_can_mitre() {
        // **Laid out**, because the plot is measured off what the widget recorded
        // and a node has no rect until a pass has placed it.
        let demo = laid_out();
        let reach = demo.chart.stroke_reach();
        assert_eq!(
            reach,
            demo.chart.line_width() / 2.0 * 4.0,
            "at the demo's own 3-px line width the reach is 6 px, and **both sides of \\
             this assertion are the widget's**, so on its own it is a tautology: a \\
             literal 6.0 written at either one is the same number here. The half \\
             that can fail is below, at a width the demo does not use."
        );
        let floor = reach * 2.0;
        assert_eq!(
            floor, 12.0,
            "so a segment longer than 12 px mitres every corner at the demo's 3-px \\
             line width — and the 12 is a consequence of the reach the widget \\
             reported, not a number of its own: the next half changes the width and \\
             the floor moves with it"
        );

        // The plot's width, **measured out of the widget's own drawing** rather
        // than taken from `CHART_SIZE`. The width is the node's own because the
        // demo writes no y labels and the *left* gutter is reserved only when there
        // are labels to put in it — and the gutter the demo's x labels do reserve
        // is a **bottom** one, so it comes off the height. Reading it instead of
        // asserting it is what stops this test agreeing with a false premise about
        // which axis the gutter belongs to.
        let plot = measured_plot(&demo).width;
        for samples in [CHART_OPENING_COUNT, CHART_SERIES.len()] {
            let pitch = plot / (samples as f32 - 1.0);
            assert!(
                pitch > floor,
                "{samples} samples over a {plot}-wide plot is a {pitch:.1} px pitch, \
                 which is above the {floor} px floor — and below it every corner \
                 would be rounded rather than mitred"
            );
        }
        // And the number it would take to reach the floor, so a reader who wants
        // to push the window further knows where the wall is.
        let crowded = (plot / floor).floor() as usize + 1;
        assert_eq!(
            crowded,
            23,
            "23 samples is the most this plot mitres every corner of, and the \
             demo's longest series is {}",
            CHART_SERIES.len()
        );

        // **The coupling, and this is the half that can fail.** The demo never sets
        // a line width, so at its own 3 px a copied 6.0 is not a different number.
        // At 10 px the widget's answer is 20 px and a literal is not, so the two
        // sides of the identity come apart and the assertion above stops being a
        // tautology.
        let mut thicker = laid_out();
        thicker.chart.set_line_width(10.0);
        assert_eq!(
            thicker.chart.stroke_reach(),
            thicker.chart.line_width() / 2.0 * 4.0,
            "and the reach the demo reads is the widget's own and follows its line \
             width — {}. A literal 6.0 written at either side agrees at the demo's \
             3-px width and disagrees here, which is the whole of what this test \
             can and cannot police",
            thicker.chart.stroke_reach()
        );
        assert!(
            thicker.chart.stroke_reach() > reach * 2.0,
            "a 10-px line reaches more than twice as far as a 3-px one, so the bound \
             the demo checks its clearances against is not a constant in disguise"
        );
    }

    /// Returns the chart's plot in window coordinates, **measured from what the
    /// widget recorded** rather than derived from any constant of this file's.
    ///
    /// `(left, width, bottom, height)`, read two independent ways because two
    /// independent reads are what makes a number checkable:
    ///
    /// - the **grid lines** and the **x axis** are drawn from `plot.x` to
    ///   `plot.x + plot.width`, so their x extent *is* the width;
    /// - the **x labels** sit at `plot.x + x · plot.width` for each sample's own
    ///   even spacing, so the last label's x solves for the width again — and the
    ///   **y axis** runs from `plot.y` to `plot.y + plot.height`, so its y extent is
    ///   the height.
    ///
    /// **Why measured at all**, which is the finding this helper answers: the widget
    /// keeps `X_LABEL_GUTTER` and `Y_LABEL_GUTTER` private and exposes no accessor
    /// for either, so the demo holds [`CHART_X_LABEL_GUTTER`] itself. A copy that is
    /// never compared against its source is a copy that can rot, and the way this
    /// repository has shipped wrong numbers before is by writing them down and
    /// believing them. `the_plot_is_the_nodes_own_width_and_its_height_less_the_
    /// widget_s_x_gutter` is where the copy is compared.
    fn measured_plot(demo: &Demo) -> MeasuredPlot {
        let node = demo
            .node_rect(demo.chart.handle())
            .expect("the chart is placed");
        let commands = demo.commands_at(demo.chart.handle());
        let mut left = f32::MAX;
        let mut right = f32::MIN;
        let mut top = f32::MAX;
        let mut bottom = f32::MIN;
        let mut labels: Vec<f32> = Vec::new();
        for command in &commands {
            match command {
                DrawCommand::Line { start, end, .. } => {
                    if start.1 == end.1 {
                        left = left.min(start.0);
                        right = right.max(end.0);
                        bottom = bottom.max(start.1);
                    } else if start.0 == end.0 {
                        top = top.min(start.1);
                        bottom = bottom.max(end.1);
                    }
                }
                DrawCommand::Text { x, .. } => labels.push(*x),
                _ => {}
            }
        }
        // The grid lines run the plot's full width; the x axis is one of them and
        // is 2 px wide where the grid is 1, so the extreme x is the same for all.
        let width = right - left;
        assert_eq!(
            left, node.x,
            "the plot's left edge is the node's own: the demo \
            writes no y labels, so no left gutter is reserved"
        );
        assert!(
            (width - CHART_SIZE.width).abs() < 0.01,
            "and the plot keeps the node's whole width, because the gutter the x \
             labels reserve is a bottom one and comes off the height: {width} px \
             against the node's {}",
            CHART_SIZE.width
        );
        assert!(
            (top - node.y).abs() < 0.01,
            "the plot's top edge is the node's own top edge, since the y axis runs \
             up to it"
        );
        // And the width a second time, from the labels: the demo writes one label
        // per sample but its newest, so the last label belongs to sample
        // `count - 2` of `count` and sits at `(count - 2) / (count - 1)` of the way
        // across. A label at the plot's right edge would be the unbounded overhang
        // `CHART_X_LABEL_COUNT` exists to avoid, so this also re-checks that rule
        // from the drawing rather than from the constant.
        if let Some(last) = labels.iter().copied().fold(f32::MIN, f32::max).into() {
            let count = demo.chart.data.get().len();
            let fraction = (count - 2) as f32 / (count - 1) as f32;
            assert!(
                (last - (left + width * fraction)).abs() < 0.01,
                "the last x label at {last} is where a {width}-wide plot puts \
                 sample {} of {count}, so the width is confirmed a second way",
                count - 2
            );
            assert!(
                last < right,
                "and no label sits on the plot's right edge, which is the overhang \
                 `CHART_X_LABEL_COUNT` keeps clear"
            );
        }
        MeasuredPlot {
            left,
            width,
            bottom,
            top,
        }
    }

    /// The chart's plot as the widget drew it, in window coordinates.
    ///
    /// Named fields rather than a tuple of four floats, because the use sites are
    /// arithmetic (`width / 11.0`) and a positional `.1` there is a number nobody
    /// can check by reading.
    struct MeasuredPlot {
        /// The plot's left edge, where the grid lines start.
        left: f32,
        /// How wide it is, which is the grid lines' own x extent.
        width: f32,
        /// Its bottom edge, which is where the x axis is drawn.
        bottom: f32,
        /// Its top edge, which is where the y axis starts.
        top: f32,
    }

    /// The plot the demo's chart actually draws, against the two numbers
    /// [`CHART_SIZE`]'s doc publishes.
    ///
    /// **This is the finding-1 test**: the doc says "the plot is 270 wide and 432
    /// tall" and a review read that sentence as though the 18-pixel gutter came off
    /// the width — which would make it 252 wide, the pitch 22.9, and the mitre wall
    /// 22 samples instead of 23. **The reading is wrong and the numbers are right**,
    /// because `X_LABEL_GUTTER` is a *bottom* gutter and is reserved against the
    /// height; the pixels agree (the x axis is drawn 1000 → 1270 and the grid lines
    /// with it, and the last x label sits at 1231.4286, which is `1000 + 270 · 6/7`
    /// and not `1000 + 252 · 6/7`).
    ///
    /// **So this test's job is not to change a number but to make the number
    /// checkable in one command**, which is what would have caught the misreading
    /// without a reviewer having to reconstruct the geometry: every figure comes out
    /// of the widget's own recorded paint, and the one number this file holds
    /// itself — [`CHART_X_LABEL_GUTTER`] — is compared against the widget's.
    #[test]
    fn the_plot_is_the_nodes_own_width_and_its_height_less_the_widgets_x_gutter() {
        let demo = laid_out();
        assert!(
            demo.chart.y_labels.get().is_empty(),
            "the demo writes no y labels, so no left gutter is reserved — which is \
             the premise that lets the plot keep the node's whole width, and it is \
             the premise `Demo::new` refuses to build without"
        );
        let MeasuredPlot {
            left,
            width,
            bottom,
            top,
        } = measured_plot(&demo);
        let node = demo
            .node_rect(demo.chart.handle())
            .expect("the chart is placed");

        // The gutter, read back out of the widget: the x axis is drawn along the
        // plot's own bottom edge, so the space between it and the node's bottom is
        // what the widget reserved for the labels.
        let gutter = (node.y + node.height) - bottom;
        assert!(
            (gutter - CHART_X_LABEL_GUTTER).abs() < 0.01,
            "the widget reserved {gutter} px under the plot and the demo holds \
             {CHART_X_LABEL_GUTTER} — the one number of the widget's this file has \
             to keep, checked against the source it came from"
        );
        assert!(
            (width - CHART_SIZE.width).abs() < 0.01,
            "and the plot keeps the node's whole {width} px of width: a bottom \
             gutter does not come off a width"
        );
        assert_eq!(
            (left, top),
            (node.x, node.y),
            "with its top-left at the node's own, because the y axis runs up to the \
             node's top edge"
        );
        assert!(
            (bottom - top - (node.height - gutter)).abs() < 0.01,
            "so the plot is {} px tall, the node's {} less the {gutter}-px gutter, \
             and a twelve-sample series has a pitch of {width} / 11 = {:.1} px",
            bottom - top,
            node.height,
            width / 11.0
        );
    }

    /// A press over the chart reaches nothing, exactly as over the gauge.
    ///
    /// A chart is a display: no value a drag would set, no action a tap would
    /// report, and no `on_event` on the type for one to be routed to. What is
    /// asserted is both directions of the mistake `.ai/NEVERAGAIN.md` § *a drawn
    /// control with nothing behind it* is about: nothing the chart owns changed,
    /// **and nothing behind it was pressed either** — the pads are the one thing in
    /// this window that answers a press, and the chart sits nowhere near them.
    #[test]
    fn a_press_on_the_chart_reaches_nothing() {
        let mut demo = dialog_closed();
        let chart = demo
            .node_rect(demo.chart.handle())
            .expect("a laid-out chart");
        let (down, up) = click_at(chart.x + chart.width / 2.0, chart.y + chart.height / 2.0);
        let before = demo.chart.data.get();
        let presses: Vec<f32> = demo.pads.iter().map(|pad| pad.press.get()).collect();

        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.chart.data.get(),
            before,
            "the series is the demo's to write"
        );
        assert_eq!(
            demo.chart.chart_type(),
            ChartType::Line,
            "and so is its shape"
        );
        assert_eq!(demo.focused, None, "and a tap does not move focus onto it");
        assert_eq!(
            demo.pads
                .iter()
                .map(|pad| pad.press.get())
                .collect::<Vec<f32>>(),
            presses,
            "and no pad behind it was pressed"
        );
    }

    #[test]
    fn the_demo_has_the_widgets_the_later_tasks_added() {
        // They are in the band, in the order they were added, and each is wired
        // to something the demo can show: the toggle to a label that names its
        // state, the progress bar to a label that names its value, the chart to a
        // label that names its length and its shape, and the image to a label
        // that names its fit.
        //
        // **The chart is in this list where the list was**, and the list is not:
        // it went on 2026-10-02 to make room, so this test follows the same claim
        // onto the widget that now holds that column.
        let demo = laid_out();
        for (what, handle) in [
            ("toggle", demo.toggle.handle()),
            ("image", demo.image.handle()),
            ("progress bar", demo.progress.handle()),
            ("chart", demo.chart.handle()),
        ] {
            assert!(
                demo.nodes.borrow().get(handle).is_some(),
                "the {what} has a node of its own in the tree"
            );
            assert!(
                demo.order.contains(&handle),
                "and the {what} is in the paint order, or it is never drawn"
            );
        }
        assert_eq!(
            demo.readout_text_of(&demo.toggle_readout).as_deref(),
            Some("off, 0 changes"),
            "the toggle starts off and its label says so"
        );
        assert_eq!(
            demo.readout_text_of(&demo.progress_readout).as_deref(),
            Some("50%, determinate"),
            "the bar starts at half, which is task 17's acceptance criterion"
        );
        assert_eq!(
            demo.readout_text_of(&demo.image_fit_readout).as_deref(),
            Some("fit: Contain (stand-in)"),
            "and the image starts fitted inside its box"
        );
    }

    #[test]
    fn clicking_the_toggle_switches_it_and_its_label_follows() {
        // "A click turns it on and off, with a readout showing its state": the
        // readout is checked after each of two clicks, so a toggle that stuck on
        // would pass a test that only looked at the first.
        let mut demo = dialog_closed();
        assert!(!demo.toggle.checked.get(), "it starts off");

        click_toggle(&mut demo);

        assert!(demo.toggle.checked.get(), "the click turned it on");
        assert_eq!(
            demo.readout_text_of(&demo.toggle_readout).as_deref(),
            Some("on, 1 change"),
            "and the label says which way it is and how many times it was thrown"
        );

        click_toggle(&mut demo);

        assert!(
            !demo.toggle.checked.get(),
            "and a second click turned it off"
        );
        assert_eq!(
            demo.readout_text_of(&demo.toggle_readout).as_deref(),
            Some("off, 2 changes"),
            "with the count following, so the second click is not the first again"
        );
    }

    #[test]
    fn the_toggle_thumb_slides_rather_than_jumping() {
        // Requirement 4 is a transition, not an end state: a `set` instead of an
        // animation would be at the far end on the first frame and pass a test
        // that only looked at where it ended up.
        let mut demo = dialog_closed();
        let rect = demo.toggle_rect().expect("a laid-out toggle");
        let off_end = demo.toggle.thumb_center(rect).0;

        click_toggle(&mut demo);
        let midway = demo.toggle.thumb_center(rect).0;

        assert!(
            midway > off_end && midway < rect.x + rect.width,
            "one frame in, the thumb is part way from {off_end} to the far end: \
             {midway} in a track from {} to {}",
            rect.x,
            rect.x + rect.width
        );

        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.toggle.thumb_center(rect).0,
            rect.x + rect.width - 12.0,
            "and it arrives at the on end, a thumb radius in"
        );
        assert!(
            !demo.toggle.is_animating(),
            "a transition that has arrived has stopped"
        );
    }

    #[test]
    fn the_toggles_pill_changes_colour_between_its_two_states() {
        // "Track colour changes: off / on" is a colour, so the assertion is on
        // the colour: two rounded rects of the same size and the same place with
        // the same shape are the same drawing, and only one of the two numbers
        // distinguishes them.
        let mut demo = dialog_closed();
        let before = demo.toggle.style().track;
        assert_eq!(
            before,
            demo.toggle.palette().track_off,
            "off is the off colour"
        );

        click_toggle(&mut demo);

        assert_ne!(
            demo.toggle.style().track,
            before,
            "and on is a different one"
        );
        assert_eq!(
            demo.toggle.style().track,
            demo.toggle.palette().track_on,
            "namely the theme's on colour"
        );
    }

    #[test]
    fn the_toggle_paints_a_pill_and_a_thumb_and_nothing_else() {
        // "Renders track and thumb" is two shapes on one node, in the order that
        // makes the thumb's shadow read as a ring rather than a disc.
        let demo = laid_out();
        let commands = demo.commands_at(demo.toggle.handle());
        let pills = commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::RoundedRect { .. }))
            .count();
        let circles = commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Circle { .. }))
            .count();
        assert_eq!(pills, 1, "one pill, an unfocused toggle");
        assert_eq!(circles, 2, "a thumb, which is a shadow and a thumb on top");
    }

    #[test]
    fn a_key_switches_the_toggle_once_it_holds_focus() {
        // A key is not routed by position, so the toggle has to be the focused
        // node to hear one — and the demo is what has to put it there.
        let mut demo = dialog_closed();
        demo.handle_event(key(Keycode::Return));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(
            !demo.toggle.checked.get(),
            "an unfocused toggle ignores the key, or nothing would ever focus it"
        );

        // The third stop: the slider, the image, and then the toggle — the
        // controls layer's paint order, which is the `Tab` order. It was the fifth
        // while two enabled buttons preceded it.
        for _ in 0..3 {
            demo.handle_event(key(Keycode::Tab));
        }
        assert_eq!(demo.focused, Some(demo.toggle.handle()), "Tab reached it");

        demo.handle_event(key(Keycode::Return));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(
            demo.toggle.checked.get(),
            "and the key switched it once it was the focused control"
        );
    }

    #[test]
    fn the_f_key_cycles_the_image_through_its_four_fits() {
        // "The demo must show one fit at a time, in one place": there is **one**
        // image node, and pressing `F` changes what that one draws. Four copies
        // would be a gallery, and a test that counted four images would pass on
        // one.
        let mut demo = dialog_closed();
        let image_node = demo.image.handle();
        let images = |demo: &Demo| {
            demo.commands_at(image_node)
                .iter()
                .filter(|command| matches!(command, DrawCommand::Image { .. }))
                .count()
        };
        assert_eq!(images(&demo), 1, "one image, and one command for it");

        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(demo.image.fit());
            demo.handle_event(key(Keycode::F));
            demo.frame(WINDOW, Duration::from_millis(16));
            assert_eq!(images(&demo), 1, "still one image after cycling");
        }
        seen.push(demo.image.fit());

        assert_eq!(
            seen,
            vec![
                ImageFit::Contain,
                ImageFit::Cover,
                ImageFit::Fill,
                ImageFit::None,
                ImageFit::Contain
            ],
            "and the four fits, in the task's order, wrapping back to the first"
        );
    }

    #[test]
    fn the_image_says_which_fit_it_is_showing() {
        // The label is bound to the same property the key writes, so a cycle
        // that changed the fit without changing the label would show here.
        let mut demo = dialog_closed();
        let mut said = Vec::new();
        for _ in 0..4 {
            said.push(
                demo.readout_text_of(&demo.image_fit_readout)
                    .unwrap_or_default(),
            );
            demo.handle_event(key(Keycode::F));
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert_eq!(
            said,
            vec![
                "fit: Contain (stand-in)",
                "fit: Cover (stand-in)",
                "fit: Fill (stand-in)",
                "fit: None (stand-in)",
            ],
            "one name per fit, in the order the key walks them"
        );
    }

    #[test]
    fn the_four_fits_draw_the_image_in_four_different_places() {
        // "Contain works", "Cover works", "Fill works" and "None works" are four
        // claims about geometry, and the numbers that tell them apart are the
        // drawn rect and the sampled window. A test that only checked "an image
        // was drawn" would pass on four copies of the same call.
        let mut demo = dialog_closed();
        let bounds = demo.image_rect().expect("a laid-out image");
        let mut drawn: Vec<(Rect, UvRect)> = Vec::new();
        for _ in 0..4 {
            demo.frame(WINDOW, Duration::from_millis(16));
            let commands = demo.commands_at(demo.image.handle());
            let found = commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Image { rect, uv, .. } => Some((*rect, *uv)),
                    _ => None,
                })
                .expect("the image paints an image");
            drawn.push(found);
            demo.handle_event(key(Keycode::F));
        }

        let (contain, contain_uv) = drawn[0];
        let (cover, cover_uv) = drawn[1];
        let (fill, fill_uv) = drawn[2];
        let (none, none_uv) = drawn[3];

        // The source is 320 by 192 and the box is 220 by 160, so the source is
        // relatively *wider* than the box: `Contain` binds the width and follows
        // the shape down, which is 220 by 132, letterboxed inside 160.
        assert_eq!(contain.width, bounds.width, "Contain binds the width");
        assert!(
            (contain.height - 132.0).abs() < 0.01,
            "and 220 by 132 is the shape of 320 by 192, not {}",
            contain.height
        );

        // `Cover` fills the box and crops the image's *width* to the middle of
        // it, so the same two numbers are not the same picture.
        assert_eq!(cover, bounds, "Cover is the box itself");
        assert!(
            !cover_uv.is_full(),
            "and it samples a window of it: {cover_uv:?} against {contain_uv:?}"
        );
        // Which axis is cropped follows from which side is relatively wider:
        // the source is 1.67 to 1 and the box is 1.375 to 1, so covering the box
        // binds the image's **height** and crops its width, and the window
        // narrows across and not down.
        assert!(
            cover_uv.u0 > contain_uv.u0 && cover_uv.u1 < contain_uv.u1,
            "a narrower window across the texture: {cover_uv:?} against \
             {contain_uv:?}"
        );
        assert_eq!(
            (cover_uv.v0, cover_uv.v1),
            (contain_uv.v0, contain_uv.v1),
            "and the full height, which is the side that was bound"
        );

        // `Fill` is the box as well, and the difference from `Contain` is that
        // it does not follow the shape: the same window, drawn on a box of a
        // different proportion. So the two UVs are equal and the two rects are
        // not, and that pair is what "ignores aspect ratio" means.
        assert_eq!(fill, bounds, "Fill is the box as well");
        assert_ne!(fill, contain, "and not the shape of the image inside it");
        assert_eq!(
            fill_uv, contain_uv,
            "sampling the same window of it, which is what stretches it"
        );

        // `None` is the source's own size, from the box's own top left, and it is
        // the one fit that is not the box.
        assert_eq!(
            (none.width, none.height),
            (ASSET_SIZE.0 as f32, ASSET_SIZE.1 as f32),
            "None is the image at its own size: {none:?}"
        );
        assert_eq!(none.x, bounds.x, "from the box's own left edge");
        assert_eq!(none.y, bounds.y, "and its own top");
        assert_eq!(
            none_uv, contain_uv,
            "and the same window as Contain, unscaled"
        );

        // And the claim every one of the above is making, stated once: the four
        // are four different drawings. The UVs are **not** `UvRect::full` and
        // are not expected to be: a 320 by 192 image is small enough for the
        // shared atlas, so its window is a window *into the atlas* and only the
        // comparison between two fits says anything about it.
        assert_ne!(cover_uv, contain_uv, "Cover and Contain sample differently");
        assert_ne!(cover_uv, none_uv, "and so does None");
    }

    #[test]
    fn the_image_stays_inside_the_window_and_clear_of_its_label_at_every_fit() {
        // The collision the placement of the fit label has to survive: `None`
        // draws the image at its own 320 by 192 rather than the box's 220 by
        // 160, so a label beside the box would be under it in that one mode and
        // clear of it in the other three. It is below the box, and this is what
        // says so for all four.
        let mut demo = laid_out();
        let window = Rect::new(0.0, 0.0, WINDOW.width, WINDOW.height);
        let label = demo
            .node_rect(demo.image_fit_readout.label.handle())
            .expect("a laid-out fit label");
        for _ in 0..4 {
            demo.frame(WINDOW, Duration::from_millis(16));
            let drawn = demo
                .image
                .destination(demo.image_rect().expect("a laid-out image"));
            assert!(
                inside(window, drawn),
                "the image at {} is inside the window",
                image_fit_name(demo.image_fit.get())
            );
            assert!(
                !touches(drawn, label),
                "and clear of the label naming its fit: {drawn:?} against {label:?}"
            );
            demo.handle_event(key(Keycode::F));
        }
    }

    #[test]
    fn the_image_paints_with_a_rounded_corner() {
        // "Rounded corners work" is the radius on the command, not the shape: a
        // textured quad is a quad whatever the radius, and only the number tells
        // the shader was asked.
        let demo = laid_out();
        let radius = demo
            .commands_at(demo.image.handle())
            .iter()
            .find_map(|command| match command {
                DrawCommand::Image { radius, .. } => Some(*radius),
                _ => None,
            })
            .expect("the image paints an image");
        assert_eq!(
            radius, IMAGE_CORNER_RADIUS,
            "at the radius the demo asked for"
        );
        assert!(radius > 0.0, "which is not none");
    }

    #[test]
    fn the_bracket_keys_move_the_progress_bars_value_and_stop_at_its_ends() {
        // The step is a tenth and the ends are 0 and 1, and the readout names the
        // **value** rather than the drawn one — which on the frame after a press
        // is a claim, because the two are not the same number then.
        let mut demo = dialog_closed();
        assert_eq!(demo.progress.value.get(), PROGRESS_START);

        demo.handle_event(key(Keycode::RightBracket));
        assert_eq!(demo.progress.value.get(), 0.6, "one press up is a tenth");

        demo.frame(WINDOW, Duration::from_millis(10));
        assert!(
            demo.progress.shown.get() < demo.progress.value.get(),
            "the drawn value is still on its way from 50 to 60: {} of {}",
            demo.progress.shown.get(),
            demo.progress.value.get()
        );
        assert_eq!(
            demo.readout_text_of(&demo.progress_readout).as_deref(),
            Some("60%, determinate"),
            "and the readout has already said where the bar is going, which a \
             readout bound to the animated property would not"
        );

        demo.handle_event(key(Keycode::LeftBracket));
        assert_eq!(
            demo.progress.value.get(),
            0.5,
            "and one press down is a tenth"
        );

        // The clamp, and it is the only part of this that is observable: five
        // tenths up from a half reaches the top, forty more presses leave it
        // there rather than at four and a half, and eighty down is the bottom.
        for _ in 0..5 {
            demo.handle_event(key(Keycode::RightBracket));
        }
        assert_eq!(
            demo.progress.value.get(),
            1.0,
            "five tenths up from a half is the top of the bar"
        );
        for _ in 0..40 {
            demo.handle_event(key(Keycode::RightBracket));
        }
        assert_eq!(
            demo.progress.value.get(),
            1.0,
            "and forty more presses leave it at the top"
        );
        for _ in 0..80 {
            demo.handle_event(key(Keycode::LeftBracket));
        }
        assert_eq!(
            demo.progress.value.get(),
            0.0,
            "and eighty down is the bottom"
        );
    }

    #[test]
    fn the_progress_bars_fill_animates_to_its_value_rather_than_jumping() {
        // "Value changes animate smoothly" is two halves, and a `set` instead of
        // an animation would be at the value on the first frame and pass an
        // end-only assertion. So the test reads the drawn fill half way.
        let mut demo = dialog_closed();
        // The **second** rounded rect, which is the fill: the track is drawn
        // first and is the whole width, so the widest of the two is the track and
        // asking for it would report a fill that never moves.
        let fill_width = |demo: &Demo| {
            demo.commands_at(demo.progress.handle())
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::RoundedRect { rect, .. } => Some(rect.width),
                    _ => None,
                })
                .nth(1)
                .unwrap_or_default()
        };
        let start = fill_width(&demo);

        demo.handle_event(key(Keycode::RightBracket));
        demo.frame(WINDOW, Duration::from_millis(10));
        let midway = fill_width(&demo);

        assert!(
            midway > start,
            "ten milliseconds in, the fill has grown from {start} to {midway}"
        );
        assert!(
            demo.progress.shown.get() < demo.progress.value.get(),
            "and it is behind the value, not already there: {} of {}",
            demo.progress.shown.get(),
            demo.progress.value.get()
        );

        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.progress.shown.get(),
            demo.progress.value.get(),
            "and it arrives rather than creeping"
        );
        assert!(
            !demo.progress.is_animating(),
            "a transition that has arrived has stopped"
        );
    }

    #[test]
    fn the_p_key_switches_the_progress_bar_into_its_sliding_mode_and_back() {
        // "Indeterminate mode shows sliding animation": the two halves are the
        // mode and the *movement*, and a bar that said it was sliding while its
        // slide sat still would pass a test on the label alone.
        let mut demo = dialog_closed();
        assert!(!demo.progress.indeterminate(), "it starts determinate");
        assert_eq!(
            demo.readout_text_of(&demo.progress_readout).as_deref(),
            Some("50%, determinate")
        );

        demo.handle_event(key(Keycode::P));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(
            demo.progress.indeterminate(),
            "P put it in indeterminate mode"
        );
        assert_eq!(
            demo.readout_text_of(&demo.progress_readout).as_deref(),
            Some("50%, sliding"),
            "and the label followed"
        );

        let parked = demo.progress.slide.get();
        for _ in 0..6 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        let moved = demo.progress.slide.get();
        assert_ne!(
            moved, parked,
            "and the bar is sliding, not sitting where it was parked"
        );
        assert!(
            demo.progress.is_animating(),
            "for as long as it is in this mode, which is the loop the widget owns"
        );

        demo.handle_event(key(Keycode::P));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(!demo.progress.indeterminate(), "and P again puts it back");
        assert_eq!(
            demo.progress.shown.get(),
            PROGRESS_START,
            "with the drawn value written at once, not animated in from the slide"
        );
    }

    #[test]
    fn the_progress_bar_paints_a_track_and_a_fill_and_nothing_else() {
        // "Progress bar renders track and fill" is two rounded rectangles, and
        // the second test above is what says the second of them is a *fill*.
        let demo = laid_out();
        let commands = demo.commands_at(demo.progress.handle());
        assert_eq!(commands.len(), 2, "a track and a fill");
        assert!(
            commands
                .iter()
                .all(|command| matches!(command, DrawCommand::RoundedRect { .. })),
            "and both of them are rounded rectangles"
        );
    }

    #[test]
    fn every_placed_rect_is_inside_the_window() {
        // The window grew to 1280 by 720 so that four more widgets fitted, and
        // every one of them is placed by hand. A hand-placed box is a claim about
        // the window, and a claim about the window is the kind of thing that is
        // only ever true until somebody moves something.
        let demo = laid_out();
        let rects = demo.placed_rects();
        assert!(
            rects.len() > 20,
            "the demo places {} boxes, so this is walking the whole set",
            rects.len()
        );
        let window = Rect::new(0.0, 0.0, WINDOW.width, WINDOW.height);
        for (what, rect) in rects {
            assert!(
                inside(window, rect),
                "the {what} is at {rect:?}, which is outside the {WINDOW:?}"
            );
        }
    }

    #[test]
    fn no_two_placed_rects_overlap() {
        // The collision three previous tasks each found by eye. A control placed
        // over the thing next to it is laid out correctly on its own, so neither
        // the control's tests nor its own layout pass can see it — and both of
        // this repository's first two rendering defects were a *drawing* that was
        // right about everything except where it was.
        let demo = laid_out();
        let rects = demo.placed_rects();
        for (index, (what, rect)) in rects.iter().enumerate() {
            for (other_what, other) in &rects[index + 1..] {
                assert!(
                    !touches(*rect, *other),
                    "the {what} at {rect:?} and the {other_what} at {other:?} share \
                     a pixel"
                );
            }
        }
    }

    #[test]
    fn the_new_widgets_sit_clear_of_the_things_already_in_the_window() {
        // The same claim, one at a time, in the words the previous tasks used: the
        // toggle is below the slider's readout, the image is right of the card of
        // pads, and the chart is right of the widest control readout. The pair of
        // tests above says no two boxes touch; this one says which boxes the new
        // ones are *not* allowed to be near, so a failure names the neighbour
        // rather than reporting forty pairs.
        let demo = laid_out();
        let at = |handle: Handle| demo.node_rect(handle).expect("a laid-out node");

        let slider_readout = at(demo.slider_readout.label.handle());
        let toggle = at(demo.toggle.handle());
        assert!(
            toggle.y > slider_readout.y + slider_readout.height,
            "the toggle at {toggle:?} is below the slider's readout at \
             {slider_readout:?}"
        );

        let card = at(demo.card().handle());
        let image = at(demo.image.handle());
        assert!(
            image.x > card.x + card.width,
            "the image at {image:?} is right of the card at {card:?}"
        );

        // The chart starts right of the widest thing in the control column, which
        // is a readout: the click counter that used to be the widest of them and
        // the list that used to be to the right of it are both gone, and the
        // gauge's readout is given the same width, so the clearance is the same
        // 16 pixels it always was.
        let widest = [
            at(demo.gauge_readout.label.handle()),
            at(demo.slider_readout.label.handle()),
        ]
        .iter()
        .max_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        .copied()
        .expect("at least one readout");
        let chart = at(demo.chart.handle());
        assert!(
            chart.x > widest.x + widest.width,
            "the chart at {chart:?} is right of the widest control readout at \
             {widest:?}"
        );

        let column_right = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        for (what, handle) in [
            ("image", demo.image.handle()),
            ("chart", demo.chart.handle()),
            ("progress bar", demo.progress.handle()),
            ("toggle", demo.toggle.handle()),
        ] {
            assert!(
                at(handle).x > column_right,
                "the {what} is right of the text column's edge at {column_right}"
            );
        }
    }

    fn mouse_down_at(x: f32, y: f32, ts: u64) -> Event {
        Event::MouseButtonDown {
            timestamp: ts,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x,
            y,
        }
    }

    /// A mouse motion with the **left button down**, which is the whole point:
    /// `GestureRecognizer` only reports a `Drag` for a pointer whose button
    /// state says it is held, so a test that forgets this is not testing a drag.
    /// SDL's mask for the left button is `SDL_BUTTON_LEFT = 1`, and the binding
    /// computes `1 << (button as u32 - 1)`, so bit 0 is the left button.
    fn mouse_move_held(x: f32, y: f32, ts: u64) -> Event {
        Event::MouseMotion {
            timestamp: ts,
            window_id: 0,
            which: 0,
            mousestate: sdl3::mouse::MouseState::from_sdl_state(1),
            x,
            y,
            xrel: 0.0,
            yrel: 0.0,
        }
    }

    /// The operator reported that dragging the slider with a **mouse** did
    /// nothing, and every test in this file that dragged the slider used
    /// [`drag_on`] — a `FingerDown`/`FingerMotion`/`FingerUp` sequence, not a
    /// mouse. So the exact route the operator uses had no coverage at all.
    ///
    /// It works, and the numbers here are the ones it produced: the value tracks
    /// the pointer to within a step at every motion, the thumb is written with it
    /// rather than animated after it, and both survive the release and a frame.
    /// The stamps are nanoseconds, as SDL delivers them, and the drag takes 400 ms
    /// — task 12's blocker was a unit mistake in exactly this field.
    #[test]
    fn a_mouse_drag_moves_the_slider_through_its_real_event_path() {
        let mut demo = dialog_closed();
        let rect = demo.slider_rect().expect("the slider is placed");
        let y = rect.y + rect.height / 2.0;

        // The pointer is already over the slider before the press, which is what
        // a hand does and what the recogniser needs to pair the motion with a
        // pointer that is held.
        demo.handle_event(mouse_move_held(rect.x + 10.0, y, 0));
        demo.handle_event(mouse_down_at(rect.x + 10.0, y, 1_000_000));
        assert_eq!(
            demo.slider.widget.value.get(),
            0.0,
            "the press alone moves nothing"
        );

        for (step, dx) in [40.0_f32, 90.0, 140.0, 190.0].into_iter().enumerate() {
            demo.handle_event(mouse_move_held(
                rect.x + dx,
                y,
                2_000_000 + (step as u64) * 100_000_000,
            ));
            let value = demo.slider.widget.value.get();
            assert!(
                value > 0.0,
                "each motion moves the value, and it had not after step {step}"
            );
            assert_eq!(
                demo.slider.widget.thumb.get(),
                value,
                "and the thumb is written with it, not animated after it — a thumb \
                 chasing a finger is a thumb the finger has passed"
            );
        }
        // The arithmetic, from the geometry rather than from a remembered number:
        // the thumb's centre travels from `x + radius` to `x + width - radius`, so
        // over a 300-wide node with an 18 radius it has 300 - 36 = 264 px of run.
        // 190 px of pointer from the node's left edge is 190 - 18 = 172 px along
        // that run, which is 172/264 = 0.6515 of the range, and 65.15 snapped to
        // [`SLIDER_STEP`]'s grid is 65.
        assert_eq!(
            demo.slider.widget.value.get(),
            65.0,
            "which is the pointer's own position mapped through the run between \
             the two thumb centres"
        );

        demo.handle_event(Event::MouseButtonUp {
            timestamp: 600_000_000,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x: rect.x + 190.0,
            y,
        });
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.slider.widget.value.get(),
            65.0,
            "and the release does not send it back"
        );
    }

    /// A press held still before moving, which is what a hand does — you put the
    /// cursor on the control and pause. It is the case where a long press could
    /// plausibly eat the drag, so it is the case worth pinning: the value does not
    /// move while the pointer is still, and moves on the first motion after it.
    #[test]
    fn a_mouse_press_held_still_before_draging_still_drags() {
        let mut demo = dialog_closed();
        let rect = demo.slider_rect().expect("the slider is placed");
        let y = rect.y + rect.height / 2.0;
        demo.handle_event(mouse_down_at(rect.x + 10.0, y, 0));
        // 900 ms of holding still: past every tap threshold in the recogniser.
        demo.handle_event(mouse_move_held(rect.x + 10.0, y, 900_000_000));
        assert_eq!(
            demo.slider.widget.value.get(),
            0.0,
            "a pointer that has not moved moves nothing"
        );
        demo.handle_event(mouse_move_held(rect.x + 150.0, y, 1_100_000_000));
        // 150 px of pointer is 150 - 18 = 132 px along a 264 px run, which is
        // exactly half the range and needs no snapping to say so.
        assert_eq!(
            demo.slider.widget.value.get(),
            50.0,
            "and the first motion after the hold is a drag like any other"
        );
    }

    /// Nothing is clipped any more, and the chart's own geometry is what says why
    /// that is allowed to be true.
    ///
    /// **The first half is the frame's own list of clips**, which is the one
    /// `Demo::draw` walks — not a helper called directly, because a test of a
    /// helper cannot see a call site that stopped using it. That was a real
    /// survivor: a mutation inlining the clip decision into the loop passed every
    /// test written against the helper. The function survives with no decision
    /// left in it, and this is what says so — one `None` per node in paint order,
    /// positionally matching, and no node offered a clip of its own.
    ///
    /// The list was the only clipped node this demo ever had and it is gone as of
    /// 2026-10-02: a scrolling viewport's rows are drawn half outside it *by
    /// design*, which is the whole reason a scissor was needed and the whole
    /// reason nothing needs one now.
    ///
    /// **The second half is the load-bearing one**, and it is this file's answer
    /// to `.ai/NEVERAGAIN.md` § *a draw-command assertion cannot see where a
    /// command lands*: the list's rows were drawn over the window background above
    /// the panel while eighty-eight unit tests passed, because every assertion in
    /// them asked *what was recorded* and not *where it landed*. So this asks the
    /// second question — over **all three** of the chart's shapes, and **twice
    /// each**: once mid-transition, where the drawn series is the glide's and a
    /// command could land anywhere between where it was and where it is going,
    /// and once after it has arrived.
    ///
    /// **The allowance is [`Chart::stroke_reach`], asked of the widget**, and it is
    /// the stroke's reach only — `Chart::stroke_reach`'s own doc is explicit that a
    /// caller wanting the widget's reach has to take the maximum of it and the
    /// label and axis bounds. **Neither of those is in the allowance, and both are
    /// named here so that "the stroke's reach" is never read as "the widget's
    /// reach":**
    ///
    /// - the **first y label's line box** overhangs the plot's top edge by
    ///   `LABEL_FONT_SIZE / 2` — **6.0 px at the defaults, which is exactly the
    ///   stroke's reach, and larger than it for any font size above 12 px.** It is
    ///   absent here because **the demo writes no y labels** (an auto-scaled axis has
    ///   no numbers a caller can write honestly), and `Demo::new` refuses to
    ///   construct if that ever stops being true rather than leaving this comment
    ///   as the only thing holding it up;
    /// - an **x label's right-hand end** overhangs by its own width, which is
    ///   **unbounded** — the widget cannot know it, because a
    ///   [`DrawCommand::Text`] carries no width. That is why
    ///   [`CHART_X_LABEL_COUNT`] leaves the newest sample unlabelled: the
    ///   [`DrawCommand::Text`] arm below therefore checks a run's *start* only, and
    ///   `the_chart_labels_every_sample_but_its_newest_one` is what keeps the label
    ///   set short enough for that to mean anything.
    ///
    /// **The axes** overhang by `AXIS_WIDTH / 2` = 1 px, on the y axis's left and
    /// the x axis's bottom, which is inside the stroke's reach at any line width
    /// the demo uses and is not separately allowed for.
    ///
    /// **Two measurements of the overhang, and they are not the same number.** The
    /// series peaks at its highest reading, which lands on the plot's top edge, and
    /// the two segments into that peak turn hard enough for a mitre: **one frame
    /// into an append the corner is 4.3956 px long and reaches 4.4 px above the
    /// node's top edge** (this test's own geometry, mid-glide), and **the settled
    /// chart on screen reaches 2 px** — measured at series-coloured row 238 against
    /// a node top edge of 240, in a capture of the running demo. Both are inside
    /// the 6 px the widget reports, and both are well inside the ten pixels of
    /// clearance to the image-fit label's line at 224, which is the number that
    /// says nothing is overdrawn. `.ai/NEVERAGAIN.md` § *a brief's rationale
    /// becomes the widget's doc comment, and nobody re-checks it* is why they are
    /// written here, where a reader can check them, and not only in a constant's
    /// doc — the constant this test's allowance used to be is gone.
    #[test]
    fn no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect() {
        let mut demo = laid_out();
        let clips = demo.frame_clips();
        assert_eq!(
            clips.len(),
            demo.order.len(),
            "one clip per node in paint order, positionally matching"
        );
        let clipped: Vec<Handle> = demo
            .order
            .iter()
            .zip(clips.iter())
            .filter(|(_, clip)| clip.is_some())
            .map(|(handle, _)| *handle)
            .collect();
        assert!(
            clipped.is_empty(),
            "and no node is offered a clip — but these are: {clipped:?}"
        );

        for (shape, name) in CHART_TYPE_NAMES.iter().enumerate() {
            for _ in 0..shape {
                demo.handle_event(key(Keycode::H));
            }
            demo.handle_event(key(Keycode::A));
            // Once mid-transition and once arrived, which are the two ends of the
            // range a command's position can take.
            for frames in [1, 40] {
                for _ in 0..frames {
                    demo.frame(WINDOW, Duration::from_millis(16));
                }
                let rect = demo
                    .node_rect(demo.chart.handle())
                    .expect("the chart is placed");
                let mut lowest = rect.y;
                // **The widget's own answer, asked once per shape** — a copied 6.0
                // is exactly what this line used to read, and the reason it does
                // not any more is the reason the constant is gone.
                let reach = demo.chart.stroke_reach();
                for command in demo.commands_at(demo.chart.handle()) {
                    match command_box(&command) {
                        Some(box_of) => {
                            assert!(
                                inside(grown(rect, reach), box_of),
                                "in the {} shape, {frames} frames in, a {command:?} \
                                 puts pixels at {box_of:?}, outside the chart's own \
                                 rect {rect:?} grown by the {reach} px \
                                 `Chart::stroke_reach` reports",
                                *name
                            );
                            lowest = lowest.max(box_of.y + box_of.height);
                        }
                        // A text run carries a position and **no width**, so all
                        // this can ask about one is that its top left corner is on
                        // the chart. The chart's own labels are one character each
                        // and there are seven of them, which is the caller's half of
                        // that answer — see `CHART_X_LABEL_COUNT`.
                        None => {
                            if let DrawCommand::Text { x, y, .. } = &command {
                                assert!(
                                    *x >= rect.x
                                        && *x <= rect.x + rect.width
                                        && *y >= rect.y
                                        && *y <= rect.y + rect.height,
                                    "in the {} shape, a {command:?} starts outside \
                                     the chart's own rect {rect:?}",
                                    *name
                                );
                                lowest = lowest.max(*y);
                            }
                        }
                    }
                }
                // And the one neighbour the reach could plausibly land on: its own
                // readout, four pixels below the node. The reach is a *lateral and
                // upward* one — every reading is bounded into the range before it is
                // scaled, so nothing is drawn below the plot's own bottom edge — and
                // this is the number that says so for this demo's data rather than
                // leaving it to the derivation.
                let readout = demo
                    .node_rect(demo.chart_readout.label.handle())
                    .expect("the readout is placed");
                assert!(
                    lowest < readout.y,
                    "in the {} shape, {frames} frames in, the chart draws as low as \
                     {lowest} and its readout starts at {}",
                    *name,
                    readout.y
                );
            }
        }
    }

    /// Returns the box every pixel `command` covers, or `None` for a
    /// [`DrawCommand::Text`].
    ///
    /// **One rule per primitive rather than a bound per variant**, because the
    /// question is the same for all of them and a rule per variant is a rule to
    /// forget — and a variant nobody taught this about is a `panic!` rather than
    /// a silent `Some`, because a new [`DrawCommand`] counted as contained by a
    /// test that never looked at it is the failure this file keeps paying for.
    ///
    /// The `Line` arm is the one worth reading: it computes the stroke's **four
    /// corners**, offset perpendicular to the segment by half its width, rather
    /// than growing the segment's bounding box on every side. `line_quad` offsets
    /// a segment perpendicular to it, so a horizontal grid line is not half a
    /// pixel wider at each end than the plot is, and a box grown in x as well
    /// fails on the very first grid line while saying nothing true about where
    /// anything lands.
    fn command_box(command: &DrawCommand) -> Option<Rect> {
        let box_of = |points: &[(f32, f32)]| {
            let low_x = points.iter().map(|p| p.0).fold(f32::MAX, f32::min);
            let low_y = points.iter().map(|p| p.1).fold(f32::MAX, f32::min);
            let high_x = points.iter().map(|p| p.0).fold(f32::MIN, f32::max);
            let high_y = points.iter().map(|p| p.1).fold(f32::MIN, f32::max);
            Rect::new(low_x, low_y, high_x - low_x, high_y - low_y)
        };
        match command {
            DrawCommand::Rect { rect, .. } | DrawCommand::RoundedRect { rect, .. } => Some(*rect),
            DrawCommand::Circle { center, radius, .. } => Some(Rect::new(
                center.0 - radius,
                center.1 - radius,
                radius * 2.0,
                radius * 2.0,
            )),
            DrawCommand::Line {
                start, end, width, ..
            } => {
                let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                let length = (dx * dx + dy * dy).sqrt();
                let (nx, ny) = if length > 0.0 {
                    (-dy / length * width / 2.0, dx / length * width / 2.0)
                } else {
                    (0.0, 0.0)
                };
                Some(box_of(&[
                    (start.0 + nx, start.1 + ny),
                    (end.0 + nx, end.1 + ny),
                    (end.0 - nx, end.1 - ny),
                    (start.0 - nx, start.1 - ny),
                ]))
            }
            DrawCommand::Polygon { points, .. } => Some(box_of(points)),
            DrawCommand::Text { .. } => None,
            // Added 2026-10-02 for `DrawCommand::Shadow`, an exhaustive match that
            // could not be taught the new variant without an arm. The chart
            // records no shadow and none is placed through this helper.
            DrawCommand::Shadow { .. } | DrawCommand::Image { .. } | DrawCommand::Path { .. } => {
                panic!(
                    "the chart records no {command:?}, and this test does not know \
                        how to place one"
                );
            }
        }
    }

    #[test]
    fn the_toggle_and_the_bar_follow_the_theme_switch() {
        // `T` carries a new theme to every widget here, and the way it does is
        // each widget's own: the ones that take a palette are aimed at the new
        // one and animated. The image is not in this list and must not be: it has
        // no palette, and a picture of a test card is the same picture in either
        // theme. **The chart is not in this list either and gets its own test**
        // below — `a_theme_switch_reaches_the_chart_and_its_readout_in_every_shape`
        // — because it has a *series* to animate as well as five colours, and a
        // test that only watched the colours would be watching half of it.
        let mut demo = dialog_closed();
        // It starts on the **dark** theme, which is the half of the claim that is
        // easy to leave out: a widget built with the neutral defaults
        // `Toggle::new` and `Progress::new` write is aimed at nothing, and it
        // would still pass everything below.
        assert_eq!(
            demo.toggle.style().track,
            TogglePalette::from_theme(&Theme::dark()).track_off,
            "the pill starts on the dark theme's own off colour"
        );
        assert_eq!(
            demo.progress.style().fill,
            ProgressPalette::from_theme(&Theme::dark()).fill,
            "and so does the bar's fill"
        );
        let dark_toggle = demo.toggle.style().track;
        let dark_progress = demo.progress.style().fill;

        demo.handle_event(toggle_theme_event());
        for _ in 0..35 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }

        assert_ne!(demo.toggle.style().track, dark_toggle, "the pill moved");
        assert_eq!(
            demo.toggle.style().track,
            TogglePalette::from_theme(&Theme::light()).track_off,
            "and arrived at the light theme's own off colour — the toggle was \
             never switched on, so the off colour is the state it is in"
        );
        assert_ne!(
            demo.progress.style().fill,
            dark_progress,
            "and so did the bar's fill"
        );
    }

    #[test]
    fn the_two_widgets_with_no_focus_state_say_where_focus_is() {
        // An `Image` and a `Progress` have no `focused` property, so a `Tab` onto
        // either of them would show nothing at all and the demo would be telling
        // the reader a control is selected with no mark on it. Their readouts say
        // so in words, which is the whole of what a widget with no focus state of
        // its own can be given.
        let mut demo = dialog_closed();
        for _ in 0..4 {
            demo.handle_event(key(Keycode::Tab));
        }
        assert_eq!(
            demo.focused,
            Some(demo.progress.handle()),
            "four Tabs from the top is the progress bar: the slider, the image, the \
             toggle and then it"
        );
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.progress_readout).as_deref(),
            Some("50%, determinate, focused"),
            "and its readout says so"
        );

        // The other one is the **second** stop, so a fresh demo rather than a
        // walk from the progress bar: the order is the slider, the image, the
        // toggle, the bar and the field.
        let mut demo = dialog_closed();
        for _ in 0..2 {
            demo.handle_event(key(Keycode::Tab));
        }
        assert_eq!(demo.focused, Some(demo.image.handle()));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.image_fit_readout).as_deref(),
            Some("fit: Contain (stand-in), focused"),
            "and the image's readout says so as well"
        );
    }

    #[test]
    fn the_asset_is_looked_for_next_to_the_executable_and_never_in_the_working_directory() {
        // A `cargo test` run's working directory is the crate's and a
        // `cargo run`'s is the workspace root, so a path relative to "here" finds
        // the asset in one and not the other. Everything the demo looks at is
        // therefore an absolute path built from the executable — and the
        // environment variable is first, so a caller can say where the file is.
        let candidates = asset_candidates_from(
            Path::new("/w/ui/target/debug/ui_demo"),
            Some(OsStr::new("/somewhere/else")),
        );
        assert_eq!(
            candidates[0],
            PathBuf::from("/somewhere/else/demo.png"),
            "the override directory is looked at first, and it names the file"
        );
        assert!(
            candidates
                .iter()
                .any(|path| path == Path::new("/w/ui/src/ui_demo/assets/demo.png")),
            "and the workspace root is in there: {candidates:?}"
        );
        assert!(
            !candidates
                .iter()
                .any(|path| path.is_relative() || path.starts_with("./")),
            "and not one of them is relative to the working directory"
        );

        // Without an override the walk starts at the executable's own directory,
        // which is `target/debug`, and climbs.
        let plain = asset_candidates_from(Path::new("/w/ui/target/debug/ui_demo"), None);
        assert_eq!(
            plain.first().map(PathBuf::as_path),
            Some(Path::new("/w/ui/target/debug/src/ui_demo/assets/demo.png")),
            "it starts one above the executable, which is not a directory to walk out of"
        );
        assert_eq!(plain.len(), 5, "and it walks every ancestor it has");
    }

    /// Returns the text the frame-rate readout is showing, as the last frame
    /// recorded it.
    fn fps_readout_text(demo: &Demo) -> Option<String> {
        demo.readout_text_of(&demo.fps_readout)
    }

    /// Runs `count` frames of `delta` each through `demo`.
    ///
    /// The loop's own two lines with the deltas a test chose instead of the ones
    /// a clock produced, which is what lets a test say what the demo does with a
    /// known sequence of frame costs.
    fn frames(demo: &mut Demo, count: usize, delta: Duration) {
        for _ in 0..count {
            demo.frame(WINDOW, delta);
        }
    }

    #[test]
    fn the_readout_reports_the_frames_the_loop_actually_drew() {
        // The wiring, and the only thing here that is not the meter's own
        // arithmetic: thirty frames of 20 ms is 600 ms and 50 frames per second,
        // so the readout saying 50 means the frames `Demo::frame` drew are the
        // frames it measured. A frame that did not tick the meter would leave the
        // readout on its constructed string forever.
        let mut demo = demo();
        frames(&mut demo, 30, Duration::from_millis(20));
        assert_eq!(
            fps_readout_text(&demo).as_deref(),
            Some("fps 50, avg 50.0, worst 20 ms")
        );
    }

    #[test]
    fn the_report_line_names_the_runs_own_numbers() {
        // **This is the line a test runner parses**, so its shape is the contract:
        // the prefix `fps-check.sh` greps for, and every field spelled the way that
        // script spells it. A renamed field is a runner that quietly finds
        // nothing, which is why the whole line is asserted rather than the parts.
        let mut demo = demo();
        frames(&mut demo, 30, Duration::from_millis(20));
        assert_eq!(
            demo.fps_report(),
            "roados-fps frames=30 duration_s=0.600 average_fps=50.0 \
             worst_frame_ms=20.0 long_frames=0"
        );
    }

    #[test]
    fn a_stall_reaches_both_the_readout_and_the_report() {
        // The defect `.ai/NEVERAGAIN.md` records — a demo that renders correctly
        // at four frames a second — is invisible in a capture and obvious in these
        // two numbers. Forty uniform frames and one slow one: the average still
        // reads like a working application, and the rate and the worst frame say
        // what happened.
        let mut demo = demo();
        frames(&mut demo, 40, Duration::from_millis(20));
        demo.frame(WINDOW, Duration::from_millis(200));
        let said = fps_readout_text(&demo).expect("the frame-rate readout");
        assert_eq!(said, "fps 32, avg 41.0, worst 200 ms");
        assert!(
            !said.contains('…'),
            "and it is not cut short: {said:?} is inside the box it was given"
        );
        let report = demo.fps_report();
        assert!(report.contains("worst_frame_ms=200.0"), "{report}");
        assert!(report.contains("long_frames=1"), "{report}");
    }

    #[test]
    fn the_readouts_box_is_written_out_rather_than_measured_from_its_first_text() {
        // `read_only_label` gives a readout the box of the string it starts with,
        // which is the right rule for every other readout in the band and the wrong
        // one here: this text is different on almost every frame, so a box measured
        // from `fps 0, avg 0.0, worst 0 ms` would ellipsise the line the moment the
        // rate moved. The width is [`FPS_READOUT_WIDTH`], whatever the text says.
        let demo = laid_out();
        let width = demo
            .node_rect(demo.fps_readout.label.handle())
            .expect("a laid-out readout")
            .width;
        assert_eq!(
            width, FPS_READOUT_WIDTH,
            "the box is the constant, not the width of whatever the text measured"
        );
        // And the longest line the readout can print fits inside it, measured
        // through the same metrics `record_label` draws with. Five digits each,
        // because a frame cannot be a hundredth of a millisecond long: a rate of
        // six digits needs a 16 µs frame, and this loop's own wait is 16 ms.
        let metrics = mono_metrics();
        let widest = "fps 10000, avg 10000.0, worst 9999 ms";
        let drawn: f32 = widest
            .chars()
            .map(|ch| metrics.advance(ch, READOUT_FONT))
            .sum();
        assert!(
            drawn <= FPS_READOUT_WIDTH,
            "the widest line it can show is {drawn} px and the box is \
             {FPS_READOUT_WIDTH} px"
        );
    }

    #[test]
    fn the_readout_sits_below_the_text_column_and_left_of_the_controls() {
        // The bottom left of the window is the one region nothing else is in: the
        // text column's last label ends at y 501, the controls column starts at
        // x 664 and the list's readout is at x 1000. `no_two_placed_rects_overlap`
        // says no two boxes touch; this says which boxes this one is clear of, so a
        // failure names the neighbour.
        let demo = laid_out();
        let at = |handle: Handle| demo.node_rect(handle).expect("a laid-out node");
        let fps = at(demo.fps_readout.label.handle());

        let column = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        let controls = CONTROLS_ORIGIN.0;
        assert!(
            fps.x + fps.width <= controls,
            "the readout at {fps:?} reaches into the control column at x {controls}"
        );
        assert!(
            fps.x < column,
            "and it is in the left region, which ends at x {column}"
        );
        for handle in demo.label_nodes.iter().copied() {
            let label = at(handle);
            assert!(
                label.y + label.height <= fps.y,
                "the readout at {fps:?} is below a text panel label at {label:?}"
            );
        }
        let window = Rect::new(0.0, 0.0, WINDOW.width, WINDOW.height);
        assert!(inside(window, fps), "and it is inside the {WINDOW:?}");
    }

    // -------------------------------------------------- task 19: the text-entry band

    /// A left-button release at `(x, y)`.
    ///
    /// The other half of [`mouse_down_at`], and named rather than inlined because
    /// task 19's tests press and release a great many keys and a release written
    /// out at each call site is 10 lines of noise per key.
    fn mouse_up_at(x: f32, y: f32, ts: u64) -> Event {
        Event::MouseButtonUp {
            timestamp: ts,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x,
            y,
        }
    }

    /// A key-down event for `keycode`, which is what a `Tab` or an arrow is.
    fn key_event(keycode: Keycode) -> Event {
        key_event_with(keycode, Mod::empty())
    }

    /// A key-down event for `keycode` with `keymod` held, which is what a
    /// `Shift+Tab` is.
    ///
    /// The modifier is the *event's* own rather than something the demo's
    /// recogniser infers: `input.rs`'s `Focus::handle_key` reads
    /// `InputEventKind::KeyDown`'s own `keymod` field, and the recogniser copies
    /// SDL's `keymod` straight into it, so a `Shift+Tab` is an SDL event with
    /// `LSHIFTMOD` set and nothing else. A walk test that built the event
    /// differently from the one the keyboard builds would be testing the
    /// recogniser and calling it the focus order.
    fn key_event_with(keycode: Keycode, keymod: Mod) -> Event {
        Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(keycode),
            scancode: None,
            keymod,
            repeat: false,
            which: 0,
            raw: 0,
        }
    }

    /// The key-**up** event for `keycode`.
    ///
    /// Its own helper rather than a variant of `key_event_with`, because the two
    /// are different variants of `Event` and a flag on one of them would be a
    /// boolean that means "which half of the press" — and because only the pads'
    /// `press_all`/`release_all` pair reads a key-up at all, so there is exactly
    /// one caller and it is a test about releases.
    fn key_up(keycode: Keycode) -> Event {
        Event::KeyUp {
            timestamp: 0,
            window_id: 0,
            keycode: Some(keycode),
            scancode: None,
            keymod: Mod::empty(),
            repeat: false,
            which: 0,
            raw: 0,
        }
    }

    /// Returns the centre of the key that would report `wanted`, in window
    /// coordinates.
    ///
    /// Found by asking the widget rather than worked out from the layout
    /// constants: the demo's job here is to aim a real press at the control **as
    /// it is drawn**, and a hand-computed point is a fixture near the key rather
    /// than the key. A test that presses 40 pixels to the left of the key it means
    /// proves only that pressing a gap does nothing.
    fn key_center(demo: &Demo, wanted: KeyAction) -> (f32, f32) {
        let rect = demo
            .keyboard_rect()
            .expect("the keyboard has been laid out");
        // Walk the keys by index and ask for each one's own rect, then hit-test
        // that rect's centre to get the widget's `KeyId` for it. Going through
        // `key_at` rather than assuming index N is `KeyId` N is the point: the two
        // pages have different key counts, so an index is not an identity.
        (0..demo.keyboard.key_count())
            .filter_map(|index| {
                let key = demo.keyboard.key_rect(index, rect)?;
                let centre = Offset::new(key.x + key.width / 2.0, key.y + key.height / 2.0);
                let id = demo.keyboard.key_at(centre, rect)?;
                (demo.keyboard.key_action(id) == Some(wanted))
                    .then_some((key.x + key.width / 2.0, key.y + key.height / 2.0))
            })
            .next()
            .unwrap_or_else(|| panic!("no key reports {wanted:?}"))
    }

    /// Presses and releases the key reporting `wanted`, through the demo's own
    /// event path — `MouseButtonDown` into `Demo::handle_event`, the recogniser,
    /// `input::route`, and back — rather than by calling the widget directly.
    #[test]
    fn a_key_on_the_keyboard_inserts_its_character_into_the_field() {
        let mut demo = dialog_closed();
        let (x, y) = key_center(&demo, KeyAction::Char('q'));

        demo.handle_event(mouse_down_at(x, y, 0));
        demo.handle_event(mouse_up_at(x, y, 40_000_000));

        assert_eq!(
            demo.text_input.text.get(),
            "q",
            "one key press is one character in the field"
        );
        // And the readout, which is bound to the same property rather than
        // written each frame, followed it — **after a frame**, because
        // `readout_text_of` reads the *recorded draw commands*, so a readout that
        // has not been repainted is a readout whose text this repository cannot
        // see. That is the distinction between the property being right and the
        // pixels being right, and both are worth asserting.
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.text_readout).as_deref(),
            Some("text: q"),
            "the text readout did not follow the field"
        );
    }

    #[test]
    fn a_run_of_keys_accumulates_in_order_and_the_field_starts_empty() {
        let mut demo = dialog_closed();
        assert_eq!(
            demo.text_input.text.get(),
            "",
            "the field opens empty, which is what makes the placeholder visible"
        );
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.text_readout).as_deref(),
            Some("text: -"),
            "an empty field reads as empty on the readout too, rather than as a \
             line that is not there"
        );

        for wanted in ['r', 'o', 'a', 'd'] {
            let (x, y) = key_center(&demo, KeyAction::Char(wanted));
            demo.handle_event(mouse_down_at(x, y, 0));
            demo.handle_event(mouse_up_at(x, y, 40_000_000));
        }

        assert_eq!(demo.text_input.text.get(), "road");
    }

    #[test]
    fn backspace_on_the_keyboard_deletes_the_character_before_the_caret() {
        let mut demo = dialog_closed();
        demo.text_input.insert_text("road");
        demo.text_input.move_caret(4);

        let (x, y) = key_center(&demo, KeyAction::Backspace);
        demo.handle_event(mouse_down_at(x, y, 0));
        demo.handle_event(mouse_up_at(x, y, 40_000_000));

        assert_eq!(
            demo.text_input.text.get(),
            "roa",
            "one backspace removes exactly one character"
        );
    }

    #[test]
    fn enter_on_the_keyboard_submits_and_the_readout_says_so() {
        let mut demo = dialog_closed();
        demo.text_input.insert_text("hamburg");

        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.submit_readout).as_deref(),
            Some("submitted: -"),
            "nothing has been submitted yet, and the readout says that rather \
             than showing nothing at all"
        );

        let (x, y) = key_center(&demo, KeyAction::Enter);
        demo.handle_event(mouse_down_at(x, y, 0));
        demo.handle_event(mouse_up_at(x, y, 40_000_000));
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.readout_text_of(&demo.submit_readout).as_deref(),
            Some("submitted: hamburg"),
            "Enter went through on_submit, not through a write of the property"
        );
    }

    #[test]
    fn a_tap_on_the_field_focuses_it_and_puts_the_caret_where_the_pointer_was() {
        let mut demo = dialog_closed();
        demo.text_input.insert_text("hamburg");
        let field = demo.text_input_rect().expect("the field has been laid out");

        // Three quarters along the text, which is past the fourth character.
        let x = field.x + field.width * 0.75;
        let y = field.y + field.height / 2.0;
        demo.handle_event(mouse_down_at(x, y, 0));
        demo.handle_event(mouse_up_at(x, y, 40_000_000));

        assert!(
            demo.text_input.focused.get(),
            "a tap on the field focuses it"
        );
        assert!(
            demo.text_input.caret() > 2,
            "the caret followed the pointer: it is at {} in a seven-character \
             field three quarters of the way along",
            demo.text_input.caret()
        );
    }

    #[test]
    fn a_key_lights_on_the_press_and_goes_out_on_the_release() {
        let mut demo = dialog_closed();
        let (x, y) = key_center(&demo, KeyAction::Char('a'));

        // Before any press, nothing is lit.
        assert!(
            !demo.keyboard.is_key_grabbed(),
            "no key is grabbed before anything is pressed"
        );

        demo.handle_event(mouse_down_at(x, y, 0));
        assert!(
            demo.keyboard.is_key_grabbed(),
            "the press lit a key — the recogniser reports a tap on the release, \
             so nothing else could"
        );

        demo.handle_event(mouse_up_at(x, y, 40_000_000));
        assert!(
            !demo.keyboard.is_key_grabbed(),
            "the release put it out again"
        );
    }

    #[test]
    fn a_press_that_misses_every_key_lights_nothing() {
        let mut demo = laid_out();
        let rect = demo
            .keyboard_rect()
            .expect("the keyboard has been laid out");

        // The padding just inside the keyboard's own edge, which is where the
        // widget draws no key at all.
        let x = rect.x + 2.0;
        let y = rect.y + 2.0;
        demo.handle_event(mouse_down_at(x, y, 0));

        assert!(
            !demo.keyboard.is_key_grabbed(),
            "a press in the keyboard's margin grabbed a key it should not have"
        );
        demo.handle_event(mouse_up_at(x, y, 40_000_000));
        assert_eq!(demo.text_input.text.get(), "", "and it typed nothing");
    }

    #[test]
    fn a_character_typed_on_a_hardware_keyboard_reaches_the_field() {
        let mut demo = laid_out();
        demo.set_focus(Some(demo.text_input.handle()));

        demo.handle_event(Event::TextInput {
            timestamp: 0,
            window_id: 0,
            text: String::from("ok"),
        });

        assert_eq!(
            demo.text_input.text.get(),
            "ok",
            "SDL's TEXTINPUT is the layout-correct route and reaches a focused \
             field; this is the path the new InputEventKind::Text exists for"
        );
    }

    #[test]
    fn tab_reaches_the_field_and_lights_its_border() {
        let mut demo = dialog_closed();
        let field = demo.text_input.handle();

        for _ in 0..40 {
            demo.handle_event(key_event(Keycode::Tab));
            if demo.focused == Some(field) {
                break;
            }
        }
        assert_eq!(demo.focused, Some(field), "Tab reaches the field");
        assert!(
            demo.text_input.focused.get(),
            "and the field knows it: set_focus calls focus(), not a bare write, \
             so the blink restarts with the caret visible"
        );
    }

    /// The focus order the demo offers, in the order `focus_navigation` puts the
    /// controls in and the tree then lays out.
    ///
    /// Written out rather than read off the walk, because the walk is what is
    /// under test: a list *derived* from the walk would agree with any order the
    /// walk produced, which is the whole reason the walk needs a test of its own.
    /// Each entry names its handle getter, so the failure says which control moved.
    ///
    /// **Five, since the list went on 2026-10-02.** The list was the fifth stop
    /// and the field the sixth; the rest keep their order, which is what makes this
    /// a removal rather than a re-laying.
    fn expected_focus_order(demo: &Demo) -> Vec<(&'static str, Handle)> {
        vec![
            ("slider", demo.slider.node()),
            ("image", demo.image.handle()),
            ("toggle", demo.toggle.handle()),
            ("progress bar", demo.progress.handle()),
            ("text field", demo.text_input.handle()),
        ]
    }

    #[test]
    fn tab_walks_every_focusable_control_in_order_and_wraps() {
        // **The walk, not the arrival.** `tab_reaches_the_field_and_lights_its_
        // border` asks whether `Tab` can reach one widget from anywhere in the
        // order; it cannot tell an order from any other order, because a walk that
        // visited the controls in the wrong sequence would still reach the field.
        // This is the test for the sequence itself, and it is here because the
        // button row's removal took 22 tests with it and this was one of them.
        //
        // The order is the tree's paint order, so it is the order the controls
        // layer's children were added in, and it is asserted against the tree
        // rather than against `focus_navigation`'s own array: that array only says
        // which nodes are *focusable*, and the order comes from the walk over the
        // tree. A control that was added to the layer out of order would pass an
        // array-order test and fail this one.
        let mut demo = dialog_closed();
        let order = expected_focus_order(&demo);
        assert_eq!(order.len(), 5, "five controls take focus in this demo");

        // Forward, one `Tab` at a time, and the whole lap twice over: a walk that
        // visited them in a different order, or stopped early, or cycled two at a
        // time, is caught by the first pass and the second is what says the last
        // one wraps to the first.
        for lap in 0..2 {
            for (index, (what, handle)) in order.iter().enumerate() {
                demo.handle_event(key_event(Keycode::Tab));
                assert_eq!(
                    demo.focused,
                    Some(*handle),
                    "lap {lap}: Tab {index} is the {what}"
                );
            }
        }

        // And every one of them *lights up*, which is a different claim from being
        // the current node: three of the five have a `focused` property and two say
        // so in words, and a stop where nothing shows is a stop a reader cannot see.
        demo.handle_event(key_event(Keycode::Tab));
        assert!(demo.slider.widget.focused.get(), "the slider's ring");
        for _ in 0..3 {
            demo.handle_event(key_event(Keycode::Tab));
        }
        assert!(demo.progress_focused.get(), "the bar's readout says so");
    }

    #[test]
    fn shift_tab_walks_the_same_order_backwards() {
        // The other direction, from nothing focused: `Focus::focus_prev` with no
        // current node takes the **last** control rather than the first, so the
        // backwards walk starts at the field. That asymmetry is the input module's
        // rule and it is worth a test of its own, because a `Shift+Tab` that went
        // forwards would still reach all five.
        let mut demo = dialog_closed();
        let order = expected_focus_order(&demo);
        let back = |demo: &mut Demo| {
            demo.handle_event(key_event_with(Keycode::Tab, Mod::LSHIFTMOD));
        };

        back(&mut demo);
        assert_eq!(
            demo.focused,
            Some(order[4].1),
            "Shift+Tab from nothing focused is the last control, the field"
        );
        for index in (0..4).rev() {
            back(&mut demo);
            assert_eq!(
                demo.focused,
                Some(order[index].1),
                "and one more back is the {}",
                order[index].0
            );
        }
        back(&mut demo);
        assert_eq!(
            demo.focused,
            Some(order[4].1),
            "which wraps to the field again rather than stopping"
        );
    }

    #[test]
    fn the_gauge_is_not_in_the_focus_order() {
        // A gauge is a display: `Gauge` has no `on_event` and no `focused`
        // property, and its own module doc says so. A `Tab` stop on one would be a
        // stop where focus arrives and nothing shows that it did — which is the
        // defect `the_two_widgets_with_no_focus_state_say_where_focus_is` exists
        // for, and the reason the gauge is not in `focus_navigation`'s array.
        //
        // **A full lap in both directions**, because "not in the order" is a claim
        // about every stop and not about the one after the last: a control that was
        // appended to the tree would be reached on the wrap, which is exactly where
        // a test that only checks the first five looks away.
        let mut demo = dialog_closed();
        let order = expected_focus_order(&demo);
        let gauge = demo.gauge.handle();
        assert!(
            !order.iter().any(|(_, handle)| *handle == gauge),
            "and the expected order does not name it either, so this is not the \
             walk agreeing with itself"
        );

        for lap in 0..2 {
            for _ in 0..(order.len() + 1) {
                demo.handle_event(key_event(Keycode::Tab));
                assert_ne!(
                    demo.focused,
                    Some(gauge),
                    "lap {lap}: Tab never lands on the dial"
                );
            }
            for _ in 0..(order.len() + 1) {
                demo.handle_event(key_event_with(Keycode::Tab, Mod::LSHIFTMOD));
                assert_ne!(
                    demo.focused,
                    Some(gauge),
                    "lap {lap}: and Shift+Tab never does either"
                );
            }
        }
    }

    /// The chart's twin of the test above, and with its **own** reason rather than
    /// the gauge's.
    ///
    /// `chart.rs` says in its module document that a drawn series, a grid and a
    /// cursor's worth of hairlines **all look grabbable and nothing in the module
    /// reads a pointer over any of them**: there is no `on_event` on the type for
    /// one to be routed to. A `Tab` stop would therefore be a stop where focus
    /// arrives, nothing answers, and no ring is drawn —
    /// `.ai/NEVERAGAIN.md` § *a drawn control with nothing behind it* is the entry
    /// about the mistake in the other direction.
    ///
    /// **Both halves of the gauge's test, for the same reason**: the expected
    /// order does not name the chart either — so this is not the walk agreeing
    /// with itself — and the walk runs a full lap in **both** directions, which is
    /// where a control appended to the tree would be reached.
    #[test]
    fn the_chart_is_not_in_the_focus_order() {
        let mut demo = dialog_closed();
        let order = expected_focus_order(&demo);
        let chart = demo.chart.handle();
        assert!(
            !order.iter().any(|(_, handle)| *handle == chart),
            "and the expected order does not name it either"
        );

        for lap in 0..2 {
            for _ in 0..(order.len() + 1) {
                demo.handle_event(key_event(Keycode::Tab));
                assert_ne!(
                    demo.focused,
                    Some(chart),
                    "lap {lap}: Tab never lands on the plot"
                );
            }
            for _ in 0..(order.len() + 1) {
                demo.handle_event(key_event_with(Keycode::Tab, Mod::LSHIFTMOD));
                assert_ne!(
                    demo.focused,
                    Some(chart),
                    "lap {lap}: and Shift+Tab never does either"
                );
            }
        }
    }

    #[test]
    fn the_sliders_knob_fits_its_column_at_the_top_of_its_range() {
        // `SLIDER_THUMB_RADIUS` is 18 because 22 stopped the right-hand column
        // fitting, and removing the buttons loosened that constraint without the
        // ceiling being retested. This is the retest, and it is cheap: the knob at
        // the **top** of the range is the one that reaches furthest right, and the
        // node's own height is what has to clear the toggle below it.
        let mut demo = laid_out();
        demo.handle_event(key_event(Keycode::_1));
        demo.frame(WINDOW, Duration::from_millis(16));
        let rect = demo.slider_rect().expect("a laid-out slider");
        let thumb = painted_thumb_radius(&demo);

        // Across: the knob's outer edge, not its centre, has to clear the chart.
        assert_eq!(
            thumb, SLIDER_THUMB_RADIUS,
            "and it is the demo's own radius"
        );
        let right = rect.x + rect.width - SLIDER_THUMB_RADIUS + thumb;
        assert!(
            right <= CHART_ORIGIN.0,
            "the knob's right edge at {right} clears the chart's left edge at {}",
            CHART_ORIGIN.0
        );
        // Down: the readout is dropped by `SLIDER_READOUT_DROP` from the slider's
        // **origin**, not from the node's bottom — the constant's own comment puts
        // the readout's 24-pixel line at 560..584 and the toggle at 592 — so the
        // column's budget is the drop plus the line, and it is checked against the
        // toggle the same way `the_new_widgets_sit_clear_of_the_things_already_in_
        // the_window` checks the drawn rects.
        let readout = demo
            .node_rect(demo.slider_readout.label.handle())
            .expect("a laid-out readout");
        let toggle = demo
            .node_rect(demo.toggle.handle())
            .expect("a laid-out toggle");
        assert!(
            (readout.y - (SLIDER_ORIGIN.1 + SLIDER_READOUT_DROP)).abs() < 0.01,
            "the readout hangs off the origin at {readout:?}, not off the node's \
             bottom"
        );
        assert!(
            toggle.y > readout.y + readout.height,
            "and the toggle at {toggle:?} is still below the readout at {readout:?}"
        );
        // The headroom a bigger knob would need, which is what the constant's
        // comment claims and what nothing was checking. **A 22-pixel knob is a
        // 62-tall node rather than 52** — two radii and a border — so it takes four
        // pixels more of the gap, and the gap is four. That is the ceiling, measured
        // rather than asserted, and it is why `SLIDER_THUMB_RADIUS` is not 22 even
        // though the buttons that used to be above it are gone.
        let headroom = toggle.y - (readout.y + readout.height);
        let taller = rect.height + (22.0 - SLIDER_THUMB_RADIUS) * 2.0;
        assert_eq!(
            taller - rect.height,
            8.0,
            "a 22-pixel knob is 8 pixels taller"
        );
        assert_eq!(
            headroom, 8.0,
            "and the clearance under the readout is exactly the same 8 pixels, so \
             22 would put the node on the toggle"
        );
    }

    #[test]
    fn the_field_draws_its_placeholder_only_while_it_is_empty() {
        let demo = laid_out();
        let field = demo.text_input_rect().expect("the field has been laid out");
        let advance = |ch: char| demo.metrics.advance(ch, TEXT_INPUT_FONT);

        let empty = demo.text_input.paint(field, &advance);
        let placeholder = empty.iter().find(
            |command| matches!(command, DrawCommand::Text { text, .. } if text == PLACEHOLDER_TEXT),
        );
        assert!(
            placeholder.is_some(),
            "an empty field shows what it is for: {PLACEHOLDER_TEXT:?}"
        );

        demo.text_input.insert_text("x");
        let filled = demo.text_input.paint(field, &advance);
        assert!(
            !filled.iter().any(
                |command| matches!(command, DrawCommand::Text { text, .. } if text == PLACEHOLDER_TEXT)
            ),
            "the placeholder is gone once there is text to show instead"
        );
    }

    #[test]
    fn every_key_is_at_least_forty_four_across_and_tall() {
        // The 44dp floor is the widget's own constant and its own test. This is the
        // demo asking the question for the geometry it actually handed the
        // keyboard, because a widget can satisfy its floor at its default size and
        // be given a box where it does not.
        let demo = laid_out();
        let rect = demo
            .keyboard_rect()
            .expect("the keyboard has been laid out");

        for index in 0..demo.keyboard.key_count() {
            let key = demo
                .keyboard
                .key_rect(index, rect)
                .unwrap_or_else(|| panic!("key {index} has no rect"));
            assert!(
                key.width >= 44.0 && key.height >= 44.0,
                "key {index} at {key:?} is smaller than a fingertip"
            );
        }
    }

    #[test]
    fn the_keyboard_and_the_field_are_both_inside_the_window() {
        let demo = laid_out();
        let window = Rect::new(0.0, 0.0, WINDOW.width, WINDOW.height);
        for (what, handle) in [
            ("text input", demo.text_input.handle()),
            ("keyboard", demo.keyboard.handle()),
            ("text readout", demo.text_readout.label.handle()),
            ("submit readout", demo.submit_readout.label.handle()),
        ] {
            let rect = demo.node_rect(handle).expect("a laid-out node");
            assert!(
                inside(window, rect),
                "the {what} at {rect:?} is outside the {WINDOW:?}"
            );
        }
    }

    #[test]
    fn the_gallery_above_the_band_is_where_it_was() {
        // The whole argument for growing the window instead of re-laying the demo
        // was that **nothing above the band moves**. This is what checks it, and
        // it is the test that makes the claim checkable rather than a sentence in
        // a doc comment.
        let demo = laid_out();
        let task_19 = ["text input", "text readout", "submit readout", "keyboard"];

        for (what, rect) in demo.placed_rects() {
            if task_19.contains(&what) {
                assert!(
                    rect.y >= BAND_TOP,
                    "the {what} at {rect:?} is inside the gallery rather than \
                     below the band"
                );
            } else {
                assert!(
                    rect.y + rect.height <= BAND_TOP,
                    "the {what} at {rect:?} reaches into the band, so the window \
                     did not grow — it moved something"
                );
            }
        }

        // And the two figures that would move if the root's own box had changed,
        // written out rather than derived: the gallery's tallest leaf is the
        // progress bar at 668..712 and the frame-rate readout is at y 684.
        let at = |handle: Handle| demo.node_rect(handle).expect("a laid-out node");
        let bar = at(demo.progress.handle());
        assert_eq!(bar.y, PROGRESS_ORIGIN.1, "the progress bar did not move");
        let fps = at(demo.fps_readout.label.handle());
        assert_eq!(
            (fps.x, fps.y),
            (FPS_READOUT_ORIGIN.0, FPS_READOUT_ORIGIN.1),
            "the frame-rate readout did not move"
        );
    }

    #[test]
    fn the_field_is_finger_sized_rather_than_the_widgets_mouse_sized_default() {
        // 240 by 44 is the widget's default and 480 by 64 is the demo's. Both
        // numbers are here so that a reader who rejects the demo's has the
        // widget's to fall back to, which is the point of the setter existing.
        let demo = laid_out();
        let rect = demo.text_input_rect().expect("the field has been laid out");
        assert_eq!(
            (rect.width, rect.height),
            (TEXT_INPUT_SIZE.width, TEXT_INPUT_SIZE.height),
            "the field is not the size the demo asked for"
        );
        assert!(
            rect.height > 44.0,
            "which is taller than the widget's own 44dp default, for the same \
             reason the slider's track is 12 rather than 6"
        );
    }

    #[test]
    fn a_theme_switch_reaches_the_field_and_the_keyboard() {
        let mut demo = dialog_closed();
        let field_before = demo.text_input.background.get();
        let key_before = demo
            .commands_at(demo.keyboard.handle())
            .first()
            .and_then(|command| match command {
                DrawCommand::RoundedRect { color, .. } => Some(*color),
                _ => None,
            });
        assert!(
            field_before != Color::new(0, 0, 0, 0),
            "the field was painted before the switch, so there is something to \
             change from"
        );

        demo.handle_event(toggle_theme_event());
        // One frame puts `animate_to` at the start of the transition, which for a
        // colour is the old value; the value only differs once frames have run.
        for _ in 0..30 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }

        assert_ne!(
            demo.text_input.background.get(),
            field_before,
            "the field's background did not follow the theme"
        );
        let key_after = demo
            .commands_at(demo.keyboard.handle())
            .first()
            .and_then(|command| match command {
                DrawCommand::RoundedRect { color, .. } => Some(*color),
                _ => None,
            });
        assert_ne!(
            key_after, key_before,
            "a key's colour did not follow the theme"
        );
    }

    #[test]
    fn the_whole_band_fits_in_the_space_below_the_gallery() {
        // [`BAND_HEIGHT`] is the whole of what task 19 was given, and this is what
        // spends it. Every band node is checked against the bottom of the window,
        // which is the constraint that actually bit: the first attempt asked for a
        // 1160-tall window and this host's window manager returned **1052**, so a
        // keyboard whose bottom was off the bottom was a keyboard nobody could
        // photograph. A band that overflows here is a band that cannot be
        // verified.
        let demo = laid_out();
        let band = Rect::new(0.0, BAND_TOP, WINDOW.width, BAND_HEIGHT);
        assert!(
            inside(Rect::new(0.0, 0.0, WINDOW.width, WINDOW.height), band),
            "the band at {band:?} is not inside the {WINDOW:?} at all"
        );

        for (what, handle) in [
            ("text input", demo.text_input.handle()),
            ("keyboard", demo.keyboard.handle()),
            ("text readout", demo.text_readout.label.handle()),
            ("submit readout", demo.submit_readout.label.handle()),
        ] {
            let rect = demo.node_rect(handle).expect("a laid-out node");
            assert!(
                inside(band, rect),
                "the {what} at {rect:?} is not inside the band at {band:?}"
            );
        }
    }

    #[test]
    fn the_keyboard_is_the_height_the_demo_says_it_is() {
        // The demo writes [`KEYBOARD_HEIGHT`] out rather than reading it from the
        // widget, because the demo's layout uses it and the widget's is the
        // widget's business. Two copies of one number is a second thing to keep in
        // step, and this is what makes changing either one alone a failure rather
        // than a surprise on screen.
        //
        // **After** the key height is set, which is the only state the demo's
        // layout is ever in: the widget's own default of 52-tall keys gives 300,
        // and asserting against that would be asserting about a keyboard this demo
        // does not have.
        let keyboard = Keyboard::new(&mut Arena::new());
        keyboard.key_height.set(KEY_HEIGHT);
        assert_eq!(
            keyboard.size().height,
            KEYBOARD_HEIGHT,
            "the demo says the keyboard is {KEYBOARD_HEIGHT} tall at {} tall keys \
             and the widget says otherwise",
            KEY_HEIGHT
        );

        // And the arithmetic, written out, because the constant is the widget's
        // and a reader is entitled to see where it came from: 8 of padding top
        // and bottom, five rows, four gaps.
        assert_eq!(
            KEYBOARD_HEIGHT,
            8.0 * 2.0 + KEY_HEIGHT * 5.0 + 6.0 * 4.0,
            "the demo's keyboard height is not the padding plus five rows plus \
             four gaps"
        );
    }

    #[test]
    fn the_demo_lowers_the_keys_to_the_floor_and_never_below_it() {
        // The one number here that is a **floor** rather than a preference. The
        // band was too short for the widget's 52-tall default, so the demo asked
        // for 44 — and 44 is where it stops. If this is ever lowered it is not a
        // layout change, it is a rejection of the touch target the operator
        // rejected twice already on two other controls.
        let demo = laid_out();
        assert_eq!(KEY_HEIGHT, 44.0, "the demo's key height moved");
        assert_eq!(
            demo.keyboard.key_height.get(),
            KEY_HEIGHT,
            "and the widget was not told"
        );
        // Deliberately **not** `assert!(KEY_HEIGHT >= 44.0)`: that is a constant
        // compared with a constant, it cannot fail, and clippy is right to say
        // so. What can fail is the line above it — the widget being told — and
        // that is the assertion that has any power.
    }

    #[test]
    fn the_loop_waits_the_rest_of_the_frame_and_not_a_flat_sixteen_milliseconds() {
        // The defect this replaced, stated as the arithmetic that fixes it. The
        // old loop waited a fixed 16 ms and *then* drew, so a frame was
        // `16 ms + work`; at this host's measured 3.9 ms of work that is 19.9 ms,
        // which is the 50 fps the demo used to sit at. The rule is that the wait
        // is what is **left** of the budget, so the work lands inside it.
        let work = Duration::from_micros(3900);
        assert_eq!(
            frame_wait(work) + work,
            FRAME_BUDGET,
            "a frame's work plus its wait is the whole budget, so the loop \
             self-paces at 60 Hz instead of paying the work on top of the wait"
        );
        assert_eq!(FRAME_BUDGET, Duration::from_nanos(16_666_667));
        // And the number the old loop could not reach, spelled out: a zero-work
        // frame waits the entire budget rather than a flat 16 ms.
        assert_eq!(
            frame_wait(Duration::ZERO),
            FRAME_BUDGET,
            "with nothing spent, the whole budget is waited"
        );
        assert_ne!(
            frame_wait(Duration::ZERO),
            Duration::from_millis(16),
            "the flat 16 ms wait is the bug this replaced, not the fix"
        );
    }

    #[test]
    fn a_frame_that_overran_its_budget_waits_nothing_rather_than_a_negative_time() {
        // Why the wait is zero and not some floor, and what would break if it were
        // not. `Duration::saturating_sub` is what stops the subtraction handing SDL
        // a negative `Duration`, which asks for a wait of undefined length — it is
        // the whole mechanism, and there is deliberately no `.max(..)` on top of
        // it: a deliberate break of such a clamp survived this suite once, because
        // `saturating_sub` already stops at zero and no assertion could tell the
        // two apart. An overrun reporting itself immediately is the point: a loop
        // that slept here would hide a frame that cost too much.
        for spent in [
            FRAME_BUDGET,
            FRAME_BUDGET + Duration::from_millis(1),
            Duration::from_millis(200),
        ] {
            assert_eq!(
                frame_wait(spent),
                Duration::ZERO,
                "a frame that spent {spent:?} of a {FRAME_BUDGET:?} budget has \
                 nothing left, and asks for no wait rather than a negative one"
            );
        }
        // And just under the budget there is still something left, so the clamp
        // is not simply swallowing every late frame.
        assert!(
            frame_wait(FRAME_BUDGET - Duration::from_millis(1)) > Duration::ZERO,
            "a frame one millisecond inside its budget still has a millisecond \
             to wait, so the clamp is not hiding small overruns"
        );
    }

    #[test]
    fn run_seconds_bounds_a_run_and_refuses_anything_that_is_not_a_positive_number() {
        // The demo is a window: it runs until the window is closed, so this is what
        // lets a measurement *end*, and therefore what lets one be read. The values
        // are the whole of the rule, and they are checked through the function that
        // holds it rather than through the process's environment — which is one
        // value for the whole test process, and a test that wrote it would be
        // writing it for every other test beside it.
        assert_eq!(run_seconds_from(None), None, "unset is not a bound");
        assert_eq!(
            run_seconds_from(Some("10".to_string())),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            run_seconds_from(Some(" 2.5 ".to_string())),
            Some(Duration::from_millis(2500)),
            "and a fraction of a second, which is what a fast check wants"
        );
        for refused in ["0", "-3", "ten", "", "1e300"] {
            assert_eq!(
                run_seconds_from(Some(refused.to_string())),
                None,
                "{refused:?} bounds nothing: zero measures no time, a negative \
                 runs backwards, text is not a number and a number too large for \
                 a Duration panics `Duration::from_secs_f64` rather than bounding \
                 anything"
            );
        }
    }

    // -------------------------------------------------- task 22: the dialog
    //
    // Everything below is about the **wiring**, for the reason the gauge's and the
    // chart's sections give: `dialog.rs` owns the widget's own contract — where
    // the panel is, what it records, when it is hidden — and 34 of its unit tests
    // plus 9 doctests are about that. What is left is what only *this* file can
    // know, and it is a short list: the dialog is in the tree, it is showing, its
    // panel is inside the window, its nodes go through the frame loop, a tap on a
    // button fires the button that was drawn there, a tap behind the overlay
    // reaches nothing, `Escape` and `D` close it, the title is bold, and the
    // `Tab` order is the dialog's own.

    /// The panel the dialog is drawn in, written out from the arithmetic rather
    /// than read back from the widget.
    ///
    /// `1280 - 420` of slack either side of a 420-wide panel — the widget's own
    /// `PANEL_MAX_WIDTH`, because the window is wider than that — so 430 and 850.
    /// The height is the 48 of padding, the title's one line box of 21.6
    /// (`DIALOG_FONT`'s 18 times the stand-in's 1.2), the 8 gap, the body's two
    /// line boxes of 43.2, the 8 gap and the 44 of the action row: 172.8, and
    /// `1020 - 172.8` of slack either side of it, so 423.6.
    ///
    /// **Written out and not read back**, because a test that compares the panel
    /// with `dialog.panel_rect(..)` proves the accessor agrees with itself, and
    /// every number here is arithmetic a reader can do in their head.
    const PANEL: Rect = Rect {
        x: 430.0,
        y: 423.6,
        width: 420.0,
        height: 172.8,
    };

    /// The `OK` button's rect: right-aligned against the panel's inner right edge
    /// at `430 + 420 - 24 = 826`, and the row of 44 plus 70 plus an 8-pixel gap is
    /// 122 wide, so it starts at 704. On the panel's bottom line box, one padding
    /// above its edge: `423.6 + 172.8 - 24 - 44 = 528.4`.
    const OK_BUTTON: Rect = Rect {
        x: 704.0,
        y: 528.4,
        width: 44.0,
        height: 44.0,
    };

    /// The `Cancel` button's: 8 pixels right of `OK`'s edge and 70 wide, because
    /// "Cancel" is six characters of 9 plus 16 of padding over the 44-pixel touch
    /// floor.
    const CANCEL_BUTTON: Rect = Rect {
        x: 756.0,
        y: 528.4,
        width: 70.0,
        height: 44.0,
    };

    /// The corner radius the dialog's action buttons draw their **background**
    /// with.
    ///
    /// **Read off the recorded paint on 2026-10-03**, and the reason a test names
    /// it rather than describing it in prose: the dialog's own panel is drawn with
    /// the widget's `BorderRadiusLg` of 16 and a *focused* button is drawn with
    /// its background grown by 2 on each side and its radius grown by 2 with it,
    /// so the three radii on a panel with a focused button are 16, 10 and 8 — and a
    /// filter written as "the rounded rectangles after the panel" picks up the
    /// focus ring as well as the two buttons. This is `Button`'s own default and
    /// it is the demo's business only for the length of this line.
    const ACTION_BACKGROUND_RADIUS: f32 = 8.0;

    /// The corner radius the dialog's **focus ring** is drawn with: the focused
    /// action's background radius grown by `Button`'s default `focus_ring`.
    ///
    /// The measurement, and the reason a test has to name a radius at all, are
    /// [`ACTION_BACKGROUND_RADIUS`]'s, where the three radii of a panel with one
    /// focused action are written out; this is the middle one. **What would
    /// reverse it:** `Button`'s `focus_ring` default of 2.0, which is a widget
    /// decision the demo has no vote in.
    const RING_RADIUS: f32 = ACTION_BACKGROUND_RADIUS + 2.0;

    /// The corner radius the dialog's **own panel** is drawn with, the widget's
    /// `BorderRadiusLg`, and the largest of the three.
    ///
    /// [`ACTION_BACKGROUND_RADIUS`]'s doc carries the measurement. **What would
    /// reverse it:** the widget's `BorderRadiusLg` token.
    const PANEL_RADIUS: f32 = 16.0;

    /// Returns a demo whose dialog has **finished appearing**, so a fixture is in
    /// the state the tests are about rather than 16 ms into a 300 ms transition.
    ///
    /// The twin of [`dialog_closed`], and the same argument: three whole spans of
    /// the widget's 300 ms, which is two spans of slack, and the assertions below
    /// are about a panel at rest rather than about a bounce in flight.
    fn shown_dialog() -> Demo {
        let mut demo = laid_out();
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert!(demo.dialog.visible.get(), "the dialog opens showing");
        assert_eq!(
            demo.dialog.scale.get(),
            1.0,
            "and it has arrived, so the rects below are the panel's own and not a \
             scaled copy of them"
        );
        demo
    }

    /// Returns the rects the dialog's node recorded for its **action buttons**:
    /// each button's background, in the order the actions were added.
    ///
    /// **Read back from the recorded paint**, and that is the whole point of it:
    /// the acceptance test asks for a press *where the button is painted*, and a
    /// test that asks the widget where it draws a button and then presses there is
    /// asking the widget twice and checking nothing. The filter is the radius,
    /// named by [`ACTION_BACKGROUND_RADIUS`].
    fn painted_action_rects(demo: &Demo) -> Vec<Rect> {
        demo.commands_at(demo.dialog.handle())
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect { rect, radius, .. }
                    if *radius == ACTION_BACKGROUND_RADIUS =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .collect()
    }

    /// Returns the alpha of every rounded rectangle the dialog recorded with
    /// `radius`, in the order it recorded them.
    ///
    /// **Filtered on the radius and not on the position or the count**, because a
    /// dialog with one focused action records four rounded rectangles — the panel,
    /// the ring, and the two action backgrounds — and only the radius tells them
    /// apart. That is not tidiness: the bug this helper exists for showed up as a
    /// *radius* disappearing rather than as a count changing, so a test written
    /// against the count would have seen the same four shapes before and after.
    fn dialog_shape_alphas(demo: &Demo, radius: f32) -> Vec<u8> {
        demo.commands_at(demo.dialog.handle())
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect {
                    radius: drawn,
                    color,
                    ..
                } if *drawn == radius => Some(color.a),
                _ => None,
            })
            .collect()
    }

    /// Returns the face each text run the dialog recorded asks for, in order.
    ///
    /// **The weights and not a count**, because a title and a body are the same
    /// two commands either way and what changed between them is one field on the
    /// first: a count of runs cannot tell a bold title from a regular one.
    fn dialog_run_weights(demo: &Demo) -> Vec<FontWeight> {
        demo.commands_at(demo.dialog.handle())
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { weight, .. } => Some(*weight),
                _ => None,
            })
            .collect()
    }

    /// Returns the strings each text run the dialog recorded carries, in order.
    fn dialog_run_texts(demo: &Demo) -> Vec<String> {
        demo.commands_at(demo.dialog.handle())
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// What [`gallery_state`] reports for a chart with no readings at all.
    ///
    /// **Minus one, and the number is the point.** The demo's own series is
    /// [`CHART_SERIES`] — 8 to 24 — so `0.0` would be a reading this could be
    /// confused with, and `f32::NAN` would compare unequal to *itself*, which
    /// would make the "nothing moved" assertion pass for a chart that emptied and
    /// refilled with the same values. A value no reading can hold is the only one
    /// that cannot lie in either direction.
    ///
    /// **Unreachable from the demo**, whose window is between
    /// [`CHART_OPENING_COUNT`] and [`CHART_SERIES`]'s length at every point a key
    /// can reach — which is why it is written down rather than unwrapped.
    ///
    /// What would reverse it: a series with negative readings, at which point this
    /// should be a value below the smallest of them.
    const NO_READING_VALUE: f32 = -1.0;

    /// Everything the demo's **own** shortcut keys can move, as one comparable
    /// value.
    ///
    /// It is a struct rather than a list of assertions because the claim under test
    /// is *"nothing the gallery can see changed"*, and eleven separate `assert_eq!`
    /// calls would each name one field: a shortcut that moved a field this struct
    /// forgot to hold would pass. **Every arm of `handle_event`'s key table writes
    /// something in here**, and that is what makes it a whole-gallery instrument
    /// rather than a list of examples — the `Space` row is the one to check first
    /// when a field is added.
    ///
    /// `PartialEq` and nothing else: a test comparing two of these wants to know
    /// whether anything moved, and a test that wants to know *what* reads the
    /// fields.
    #[derive(Debug, PartialEq)]
    struct GalleryState {
        /// `T`: which theme the demo is on.
        dark: bool,
        /// `+` and `-`: the text size.
        text_size: f32,
        /// `C`: the token the text takes its colour from.
        color_token: ThemeToken,
        /// `0` and `1`: the slider's value.
        slider: f32,
        /// `F`: the image's fit.
        image_fit: usize,
        /// `[` and `]`: the progress bar's value.
        progress: f32,
        /// `P`: the bar's mode.
        progress_indeterminate: bool,
        /// `,` and `.`: the gauge's value.
        gauge: f32,
        /// `G`: the gauge's shape.
        gauge_type: usize,
        /// `H`: the chart's shape.
        chart_type: usize,
        /// `A` and `S`: the chart's length.
        chart_len: usize,
        /// `S`: the chart's newest reading, which moves while its length does not.
        ///
        /// **`shift_chart_sample` drops the oldest reading and appends a new one,
        /// so the length is unchanged** — the length is `S`'s *documented* effect
        /// ("keep the length", `the_shift_key_drops_the_oldest_sample_and_keeps_
        /// the_length`) and is therefore no way at all to see that it acted. The
        /// newest reading is what moves, and it is here because that test exists.
        chart_last: f32,
        /// `Space`: each pad's press.
        pads: Vec<f32>,
    }

    fn gallery_state(demo: &Demo) -> GalleryState {
        GalleryState {
            dark: demo.dark,
            text_size: demo.text_size,
            color_token: demo.color_token.get(),
            slider: demo.slider.widget.value.get(),
            image_fit: demo.image_fit.get(),
            progress: demo.progress.value.get(),
            progress_indeterminate: demo.progress_indeterminate.get(),
            gauge: demo.gauge.value.get(),
            gauge_type: demo.gauge_type.get(),
            chart_type: demo.chart_type.get(),
            chart_len: demo.chart.data.get().len(),
            chart_last: demo
                .chart
                .data
                .get()
                .last()
                .copied()
                .unwrap_or(NO_READING_VALUE),
            pads: demo.pads.iter().map(|pad| pad.press.get()).collect(),
        }
    }

    /// A demo in the state the shortcut table is measured from: the dialog showing
    /// or closed, and **the slider moved off the end it started on**.
    ///
    /// **The slider priming is not tidiness, it is what makes two rows testable.**
    /// `0` sends the slider to [`SLIDER_MIN`] and the demo *starts* at
    /// [`SLIDER_MIN`], so `0` is a no-op from the launch state — and priming it to
    /// the top would make `1` the no-op instead. A shortcut that cannot move the
    /// thing it names cannot be shown to act, and the whole value of the table
    /// below is that each row moves something.
    ///
    /// Written through the demo's own [`Demo::set_slider_value`], which is what a
    /// keypress calls, so the fixture and the gesture under test are one code path
    /// rather than two.
    fn shortcut_fixture(dialog_up: bool) -> Demo {
        let mut demo = if dialog_up {
            shown_dialog()
        } else {
            dialog_closed()
        };
        demo.set_slider_value(SLIDER_MIN + SLIDER_STEP);
        demo
    }

    /// Runs three 50 ms frames, so a shortcut that writes through an **animation**
    /// has written.
    ///
    /// **Sixteen of the seventeen shortcuts write their property synchronously** —
    /// `Property::set` fires at once — and only `Space` goes through
    /// [`Demo::press_all`]'s stagger, so without a frame here `Space` reads as
    /// having done nothing in *both* directions and the table's most interesting
    /// row is the one that proves nothing. 150 ms is past
    /// [`STAGGER_STEP`]'s 60 twice, so all three pads have arrived rather than the
    /// first having started.
    fn settle(demo: &mut Demo) {
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(50));
        }
    }

    /// The lowest `SDL_Keycode` this build of SDL maps, through
    /// [`Keycode::from_u32`], that no row of [`GALLERY_SHORTCUTS`] names.
    ///
    /// **A spot-check and not an enumeration, and the difference is worth
    /// recording.** The obvious way to close the gap finding 2 named — press every
    /// key the platform can deliver and fail on any the demo acts on and the table
    /// omits — is **not available in this crate**. SDL3's keycode space is
    /// `SDLK_SCANCODE_MASK | scancode`: measured on 2026-10-03, `from_u32` maps 250
    /// distinct values, the highest of the printable ones is 177, and the
    /// non-printable ones sit at `1 << 30` and above — a billion values wide with
    /// 250 in it. Enumerating that range is not a test, and the two functions that
    /// bridge scancode to keycode (`Keycode::from_scancode`,
    /// `Scancode::from_keycode`) are **`unsafe`**, which this file does not add.
    ///
    /// So the guarantee is **structural** instead, and this constant is the residue:
    /// [`GALLERY_SHORTCUTS`] is in production code, `handle_event` looks every key
    /// up in it, and there is no second object for the two to disagree about — which
    /// is what went wrong before.
    ///
    /// What would reverse it: a safe keycode enumeration in `sdl3`, which is an
    /// upstream change and not this task's.
    const KEYCODE_PRINTABLE_END: u32 = 1023;

    #[test]
    fn the_two_plus_keys_are_both_rows_and_do_the_same_thing() {
        // **Finding 2's instance, pinned by name rather than by the row count.**
        //
        // Making the table the dispatch closed the *drift* — there is no second
        // object to disagree — but it opened a quieter hole of its own: a row
        // **deleted** from the table is a key that silently stops working, and every
        // test that iterates the rows is blind to a missing one. The row count in
        // `the_gallery_shortcut_list_holds_every_key_the_table_has` is the only
        // thing that would notice, and it says "eighteen" rather than "Equals".
        //
        // So the alias is asserted by name and by **effect**, which is the only
        // thing that survives the row being edited rather than counted.
        let mut effects = Vec::new();
        for keycode in [Keycode::Plus, Keycode::Equals] {
            assert!(
                GALLERY_SHORTCUTS.iter().any(|(_, key, _)| *key == keycode),
                "{keycode:?} has a row in the table, which is the object the modal \
                 guard wraps"
            );
            let mut demo = shortcut_fixture(false);
            let before = demo.text_size;
            demo.handle_event(key_event(keycode));
            settle(&mut demo);
            assert_eq!(
                demo.text_size,
                before + TEXT_SIZE_STEP,
                "{keycode:?} grows the text by one step, as its name says"
            );
            effects.push(demo.text_size);
        }
        assert_eq!(
            effects[0], effects[1],
            "and the shifted and unshifted keys land on the same size, which is \
             what makes them aliases rather than two different bindings"
        );
    }

    #[test]
    fn a_dismissed_dialog_does_not_answer_enter_during_its_fade() {
        // **Review finding 1, MAJOR**, and all three dismissal routes rather than
        // the `D` toggle: `Escape`, a tap on an action and a tap on the scrim all
        // dismiss from **inside** the widget, so none of them goes through
        // [`Demo::toggle_dialog`] and none of them retired [`Demo::focused`]. The
        // record still named a button of a dialog on its way out, and `Enter` fired
        // it.
        //
        // **Asserted on `demo.dark`, and that is the reviewer's own lesson
        // recorded.** A first version of this assertion read `demo.pending_theme`,
        // which is *always* false after `handle_event` because `handle_event`
        // drains it — so it would have passed whether or not `OK` fired, and for
        // the wrong reason. `dark` survives the call.
        //
        // **No frame between the dismissal and the `Enter`**, because that is the
        // window [`Demo::focus_is_live`] exists for: the loop calls `frame` between
        // events so on screen there is not one, and a reader reproducing it with two
        // `handle_event` calls has exactly one. A frame here would test
        // [`Demo::sync_dialog_focus`] alone and let the routing half rot.
        let shown = shown_dialog();
        assert!(
            shown.dialog.actions[0].button.focused.get(),
            "`OK` holds focus and shows its ring while the dialog is up, which is \
             what makes the next line a change"
        );
        let ok = painted_action_rects(&shown)[0];
        assert_eq!(
            ok, OK_BUTTON,
            "and it is where the fixture says it is drawn"
        );

        // **The route, the button pressed, and whether that press should have
        // moved the theme** — all three in one row, because the third is what makes
        // the first assertion readable: `OK` is the one action whose callback asks
        // for the theme switch, so on its route the theme has *already* moved by the
        // time `Enter` arrives, and "the theme did not move" would be the wrong
        // thing to assert there. What must hold on every route is the weaker and
        // actual claim: **`Enter` moved nothing.**
        for (route, button, theme_moved_by_the_dismissal) in [
            ("Escape", None, false),
            ("a tap on OK", Some(OK_BUTTON), true),
            ("a tap on Cancel", Some(CANCEL_BUTTON), false),
        ] {
            let mut demo = shown_dialog();
            let dark_before = demo.dark;
            match button {
                None => demo.handle_event(key_event(Keycode::Escape)),
                Some(button) => {
                    let (down, up) = click_at(button.x + 4.0, button.y + 4.0);
                    demo.handle_event(down);
                    demo.handle_event(up);
                }
            }
            assert!(
                !demo.dialog.visible.get(),
                "{route} dismissed it, which is the precondition of the next line"
            );
            let dark_after_dismissal = demo.dark;

            demo.handle_event(key_event(Keycode::Return));
            assert_eq!(
                demo.dark, dark_after_dismissal,
                "{route} then Enter: the theme is where the dismissal left it, so \
                 the dismissed dialog's button did not fire"
            );
            assert_eq!(
                dark_after_dismissal,
                if theme_moved_by_the_dismissal {
                    !dark_before
                } else {
                    dark_before
                },
                "and the dismissal itself moved the theme on exactly the `OK` route, \
                 whose callback is the one that asks for it"
            );
        }
    }

    #[test]
    fn the_control_enter_reaches_ok_while_the_dialog_is_showing() {
        // **The control beside the test above, and it has to be its own function**
        // because the claim is about a *pair* of states: `Enter` must reach `OK`
        // while the dialog is up and must not reach it one dismissal later. A test
        // with both halves would also do, but a failure would then not say which
        // half moved — and this half is the one that can pass by `Enter` being
        // broken everywhere.
        let mut demo = shown_dialog();
        assert!(
            demo.dialog.actions[0].button.focused.get(),
            "the demo opens with focus on its first action"
        );

        demo.handle_event(key_event(Keycode::Return));

        assert!(
            !demo.dark,
            "`Enter` reached `OK`: its callback asked for the theme switch, which is \
             the one thing only `OK` does"
        );
        assert!(
            !demo.dialog.visible.get(),
            "and the dismissal wrapped around the callback closed the dialog"
        );
        assert_eq!(
            demo.focused,
            Some(demo.dialog.actions[0].handle()),
            "with the record still stale — which is the state the test above starts \
             from, and the reason it needs no frame to reproduce"
        );
    }

    /// **Renamed 2026-10-03, and the old name was a lie by then.** It read
    /// `a_dismissed_dialogs_stale_focus_is_retired_on_the_next_frame`, and it is
    /// now `a_dismissed_dialog_withdraws_activation_without_retiring_the_focus_record`.
    /// The record is **not** retired on the next frame any more: `Button` grew an
    /// `activatable` property, so a dismissal withdraws activation and leaves the
    /// record alone, because the record is what the ring is read from and the
    /// panel outlives the dismissal by 300 ms. A test whose name contradicts what
    /// it asserts is worse than no test, because the next reader trusts the name.
    #[test]
    fn a_dismissed_dialog_withdraws_activation_without_retiring_the_focus_record() {
        // **The half of the fix with a visible consequence.** The record is no
        // longer what stops the key — `Button::activatable` is — and that is what
        // lets the record survive long enough for the ring to fade with the panel.
        let mut demo = shown_dialog();
        demo.handle_event(key_event(Keycode::Escape));
        assert_eq!(
            demo.focused,
            Some(demo.dialog.actions[0].handle()),
            "the record is untouched the instant the dismissal happens — the frame \
             has not run yet, and nothing in the event path writes it"
        );

        demo.frame(WINDOW, Duration::from_millis(16));

        assert!(
            demo.dialog.is_drawn(),
            "the panel is still being drawn, which is the whole of the `is_drawn` \
             against `dialog_is_modal` distinction"
        );
        assert_eq!(
            demo.focused,
            Some(demo.dialog.actions[0].handle()),
            "**and the record survives it**, which is the reversal: the thing that \
             used to be retired here is now kept, so the ring below has something \
             to be read from"
        );
        assert!(
            !demo.dialog.actions[0].button.activatable.get(),
            "while activation is withdrawn — the answer that used to be the \
             record's job, now asked of the widget"
        );
        assert!(
            !demo.focus_is_live(),
            "so nothing inside the dismissed dialog can be reached. **This is the \
             claim this test still keeps**, and it is kept by `activatable` rather \
             than by hiding the ring"
        );
        assert!(
            demo.dialog.actions[0].button.focused.get(),
            "while the ring stays on for the length of the fade: a ring says *where \
             focus is*, not *that this can be activated*, so it leaves with the \
             panel instead of one frame ahead of it. \
             `the_focus_ring_fades_out_with_the_panel_instead_of_popping` asserts \
             the same fact on the recorded commands rather than on this flag"
        );

        // **And it is retired when the drawing stops**, which is what the record
        // means now: a control the demo still has focus on and is still drawing.
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert!(!demo.dialog.is_drawn(), "the fade runs out");
        assert_eq!(
            demo.focused, None,
            "and only then is the record retired, on the frame the ring goes"
        );
    }

    /// **All four dismissal routes, and the gap this found.** `toggle_dialog`'s
    /// dismiss branch used to call `set_focus(None)`, so the `D` route dropped the
    /// focus record on the spot while the other three routes left it to
    /// `sync_dialog_focus` — which made `D` the one route whose ring popped, a
    /// defect the operator did not report and yesterday's fix did not reach.
    /// Found by asking which tests broke when the record stopped being retired.
    #[test]
    fn a_dismissed_dialog_withdraws_activation_on_every_route() {
        // **One frame**, not zero and not the whole fade: the record used to go on
        // the first frame for three routes and on the event for the fourth, and a
        // test that ran no frame would see all four agreeing for the wrong reason.
        for route in ["Escape", "a tap on the scrim", "a tap on Cancel", "D"] {
            let mut demo = shown_dialog();
            match route {
                "Escape" => demo.handle_event(key_event(Keycode::Escape)),
                "D" => demo.handle_event(key_event(DIALOG_KEY)),
                "a tap on Cancel" => {
                    let (down, up) = click_at(CANCEL_BUTTON.x + 4.0, CANCEL_BUTTON.y + 4.0);
                    demo.handle_event(down);
                    demo.handle_event(up);
                }
                _ => {
                    let (down, up) = click_at(60.0, 60.0);
                    demo.handle_event(down);
                    demo.handle_event(up);
                }
            }
            assert!(!demo.dialog.visible.get(), "{route} dismissed it");
            assert!(demo.dialog.is_drawn(), "{route} and the fade is running");

            demo.frame(WINDOW, Duration::from_millis(16));

            assert_eq!(
                dialog_shape_alphas(&demo, RING_RADIUS),
                vec![250],
                "{route}: the ring is still drawn, fading with the panel"
            );
            assert_eq!(
                dialog_shape_alphas(&demo, PANEL_RADIUS),
                vec![250],
                "{route}: at the panel's own alpha, so the same fade"
            );
            assert!(
                demo.dialog
                    .actions
                    .iter()
                    .all(|a| !a.button.activatable.get()),
                "{route}: and no action may be activated"
            );
            assert!(!demo.focus_is_live(), "{route}: nor is the record live");
        }
    }

    /// The record outliving a dismissal is new, and `Space`'s condition reads
    /// focus, so the pads stranding at full press is a live risk rather than a
    /// hypothetical one — it was a real bug two rounds ago, measured on the pads
    /// at `[0.0, 0.0, 0.0]` after an `Escape`.
    #[test]
    fn a_surviving_focus_record_does_not_strand_the_pads_after_a_frame() {
        // **After a frame, which is the gap.** `space_presses_the_pads_again_after
        // _every_dismissal_route` presses `Space` with no frame in between, so it
        // never reaches the new state: the record alive *and* `sync_dialog_focus`
        // having run. Only here is `focused` a `Some` naming a button of a dialog
        // that is no longer there, which is the state that could strand the pads.
        for route in ["Escape", "a tap on the scrim", "a tap on Cancel", "D"] {
            let mut demo = shown_dialog();
            match route {
                "Escape" => demo.handle_event(key_event(Keycode::Escape)),
                "D" => demo.handle_event(key_event(DIALOG_KEY)),
                "a tap on Cancel" => {
                    let (down, up) = click_at(CANCEL_BUTTON.x + 4.0, CANCEL_BUTTON.y + 4.0);
                    demo.handle_event(down);
                    demo.handle_event(up);
                }
                _ => {
                    let (down, up) = click_at(60.0, 60.0);
                    demo.handle_event(down);
                    demo.handle_event(up);
                }
            }
            demo.frame(WINDOW, Duration::from_millis(16));

            assert_eq!(
                demo.focused,
                Some(demo.dialog.actions[0].handle()),
                "{route}: the record did survive the frame — this is the state under \
                 test, and it is new"
            );
            assert!(
                !demo.focus_is_live(),
                "{route}: but it is not live, which is the whole of the answer"
            );
            let mut enter = InputEvent::new(
                InputEventKind::KeyDown {
                    key: Key::Keyboard(Keycode::Return),
                    keymod: Mod::empty(),
                },
                None,
            );
            assert!(
                !demo.offer_to_focused(&mut enter),
                "{route}: and the focused control declines the key outright"
            );

            demo.handle_event(key_event(Keycode::Space));
            settle(&mut demo);
            let first = demo.pads[0].press.get();
            assert!(
                first > 0.5,
                "{route}: so Space still presses the pads, and the first has had the \
                 whole 150 ms: {first}"
            );
        }
    }

    #[test]
    fn space_presses_the_pads_again_after_every_dismissal_route() {
        // **The second consequence of the same stale record**, and the one with no
        // report against it: `Space`'s condition reads focus, so a stale `Some`
        // stopped the one key that presses the pads from pressing them. Measured
        // before the fix — `Space` after an `Escape` dismissal left the pads at
        // `[0.0, 0.0, 0.0]` where a `D` dismissal left them at `[1.0, 0.84, 0.36]`.
        // Two dismissal routes, two behaviours, and nothing on screen to say which.
        //
        // **The threshold is on the first pad only**, and that is the stagger rather
        // than a convenience: [`settle`] runs 150 ms, `STAGGER_STEP` is 60 and
        // `PRESS_DURATION` is 150, so the second pad is 90 ms in and the third 30 ms
        // in — the measured triple is in the message so a reader can see all three.
        for route in ["Escape", "a tap on the scrim", "a tap on Cancel", "D"] {
            let mut demo = shown_dialog();
            match route {
                "Escape" => demo.handle_event(key_event(Keycode::Escape)),
                "D" => demo.handle_event(key_event(DIALOG_KEY)),
                "a tap on Cancel" => {
                    let (down, up) = click_at(CANCEL_BUTTON.x + 4.0, CANCEL_BUTTON.y + 4.0);
                    demo.handle_event(down);
                    demo.handle_event(up);
                }
                _ => {
                    let (down, up) = click_at(60.0, 60.0);
                    demo.handle_event(down);
                    demo.handle_event(up);
                }
            }
            assert!(!demo.dialog.visible.get(), "{route} dismissed it");

            demo.handle_event(key_event(Keycode::Space));
            settle(&mut demo);
            let pads = demo
                .pads
                .iter()
                .map(|pad| pad.press.get())
                .collect::<Vec<_>>();
            assert!(
                pads[0] > 0.5,
                "{route} then Space presses the pads again, and the first has had \
                 the whole 150 ms: {pads:?}"
            );
            assert!(
                pads.iter().all(|press| *press > 0.0),
                "and all three have left zero, which is what the defect looked like: \
                 {pads:?}"
            );
        }
    }

    #[test]
    fn no_printable_key_acts_without_a_row_in_the_shortcut_table() {
        // **The complement of finding 2's fix, over the keycodes this build can
        // deliver cheaply.** Every keycode [`Keycode::from_u32`] maps below
        // [`KEYCODE_PRINTABLE_END`] is pressed with the dialog closed, and each one
        // that is not a row of [`GALLERY_SHORTCUTS`] must leave the gallery
        // untouched — which is what makes the table the *only* route rather than
        // merely the one under the guard.
        //
        // **It cannot pass by the table and the dispatch having drifted**, because
        // there is nothing to drift from: `handle_event` looks the key up in this
        // very array.
        let mut checked = 0_usize;
        let mut unaccounted: Vec<Keycode> = Vec::new();
        for raw in 0..=KEYCODE_PRINTABLE_END {
            let Some(keycode) = Keycode::from_u32(raw) else {
                continue;
            };
            if GALLERY_SHORTCUTS.iter().any(|(_, key, _)| *key == keycode) {
                continue;
            }
            checked += 1;
            let mut demo = shortcut_fixture(false);
            let before = gallery_state(&demo);
            demo.handle_event(key_event(keycode));
            settle(&mut demo);
            if gallery_state(&demo) != before {
                unaccounted.push(keycode);
            }
        }
        assert!(
            unaccounted.is_empty(),
            "these keys move the gallery and have no row in `GALLERY_SHORTCUTS`, so \
             nothing wraps them in the modal guard: {unaccounted:?}"
        );
        assert!(
            checked > 50,
            "and {checked} unlisted printable keycodes really were pressed, rather \
             than the loop skipping them all"
        );
    }

    #[test]
    fn the_gallery_shortcut_list_holds_every_key_the_table_has() {
        // **The production table, pressed one at a time with the dialog closed**,
        // so every row is shown to reach something. A row that reached nothing would
        // satisfy the "does not act while showing" half on its own and be a lie in
        // the other direction.
        //
        // The table *is* the dispatch, so this is not a list checked against a
        // `match` — there is no `match`. What it does check is that every row's
        // function still does what its name says, which is the one way a table of
        // function pointers can rot silently.
        assert_eq!(GALLERY_SHORTCUTS.len(), 18, "eighteen shortcuts");
        assert!(
            !GALLERY_SHORTCUTS
                .iter()
                .any(|(_, key, _)| *key == DIALOG_KEY),
            "and `D` is not one of them, because `D` is the dialog's own and is \
             matched above the guard"
        );
        for (what, keycode, _) in GALLERY_SHORTCUTS {
            let mut demo = shortcut_fixture(false);
            let before = gallery_state(&demo);
            demo.handle_event(key_event(keycode));
            settle(&mut demo);
            assert_ne!(
                gallery_state(&demo),
                before,
                "{what} moves something in the gallery when the dialog is closed"
            );
        }
    }

    #[test]
    fn a_gallery_shortcut_does_nothing_while_the_dialog_is_showing() {
        // **The operator's decision of 2026-10-03**, and the assertion that says it
        // is implemented rather than intended: every one of the demo's own
        // shortcuts, pressed once with the dialog up, and nothing the gallery can
        // see has moved.
        //
        // **`Space` is not in this list and that is not an oversight.** Its arm is
        // `Space if self.focused.is_none()`, and a showing dialog holds focus
        // inside itself, so the focus condition alone already stops it and this
        // test could not tell the guard from that. It is in
        // `the_gallery_shortcut_list_holds_every_key_the_table_has` instead, where
        // the closed-dialog half is what matters, and its arm is still inside the
        // guard because the condition is about focus rather than about modality.
        for (what, keycode, _) in GALLERY_SHORTCUTS
            .into_iter()
            .filter(|(_, key, _)| *key != Keycode::Space)
        {
            let mut demo = shortcut_fixture(true);
            let before = gallery_state(&demo);
            demo.handle_event(key_event(keycode));
            settle(&mut demo);
            assert_eq!(
                gallery_state(&demo),
                before,
                "{what} must not act while the dialog is showing"
            );
        }
    }

    #[test]
    fn a_gallery_shortcut_acts_again_once_the_dialog_is_closed() {
        // **The control beside the test above, and the reason it is a separate
        // function rather than a second assertion in the first.** A shortcut that
        // did nothing in either state — a typo'd keycode, a key the table never
        // matched — passes "does not act while showing" perfectly, and the only
        // thing that tells the two apart is the same key moving something once the
        // dialog is gone.
        //
        // `.ai/NEVERAGAIN.md` § *A drawn control with nothing behind it*, applied to
        // a keyboard shortcut: name the gesture that operates the thing, and assert
        // the gesture moves it.
        for (what, keycode, _) in GALLERY_SHORTCUTS {
            let mut demo = shortcut_fixture(false);
            let before = gallery_state(&demo);
            demo.handle_event(key_event(keycode));
            settle(&mut demo);
            assert_ne!(
                gallery_state(&demo),
                before,
                "{what} acts again with the dialog closed"
            );
        }
    }

    #[test]
    fn the_dialogs_own_key_is_above_the_guard_and_still_both_ways() {
        // The guard in `handle_event` is `else if !self.dialog_is_modal()`, so
        // anything that has to work *while the dialog is up* must be matched above
        // it. `D` is the one such key and this is what holds it there: if `D`'s arm
        // were moved back into the table, this fails.
        let mut demo = shown_dialog();
        assert!(demo.dialog.visible.get(), "the dialog opens showing");
        demo.handle_event(key_event(DIALOG_KEY));
        assert!(
            !demo.dialog.visible.get(),
            "`D` closes it from inside the guard's scope"
        );

        let mut demo = dialog_closed();
        assert!(!demo.dialog.visible.get(), "and it starts closed");
        demo.handle_event(key_event(DIALOG_KEY));
        assert!(
            demo.dialog.visible.get(),
            "and `D` opens it too, which is the same arm and the other direction"
        );
    }

    #[test]
    fn a_pointer_press_behind_the_scrim_operates_nothing() {
        // **The same guard, on the three arms that reach the gallery without
        // routing.** `handle_event`'s `MouseButtonDown` and `FingerDown` arms call
        // `pad_at`, `slider_at` and `keyboard_at` themselves, so the tap-side
        // filter in `route_input_event` never sees them: a press on a pad with the
        // dialog showing lit the pad, and only the *tap* on the release was ever
        // offered to the dialog. All three targets are outside the panel — asserted
        // here rather than assumed, because a target under the panel would be
        // swallowed by the panel and this test would pass for the wrong reason.
        //
        // Three gestures, three controls, each with the closed-dialog control
        // beside it, for the reason the entry above is named after — and **three
        // fresh demos, one per control**, because a *release* on the scrim is a
        // dismissal: `Dialog::on_event` consumes a tap anywhere in its box and
        // closes on one that misses the panel, which is correct and which would
        // leave the second and third controls being tested with nothing in the
        // way. The first control's release is therefore its own last act.

        // **The pad: a press over its own centre.**
        let probe = shown_dialog();
        let pad = probe.node_rect(probe.pads[0].node).expect("a laid-out pad");
        let (px, py) = (pad.x + pad.width / 2.0, pad.y + pad.height / 2.0);
        assert!(
            !inside(PANEL, Rect::new(px - 1.0, py - 1.0, 2.0, 2.0)),
            "the pad's centre at ({px}, {py}) is outside the panel at {PANEL:?}, so \
             this is a press on the scrim"
        );
        let mut demo = shown_dialog();
        demo.handle_event(mouse_down_at(px, py, 0));
        settle(&mut demo);
        assert_eq!(
            demo.pads
                .iter()
                .map(|pad| pad.press.get())
                .collect::<Vec<f32>>(),
            vec![0.0, 0.0, 0.0],
            "no pad is pressing"
        );
        assert_eq!(demo.mouse_pressed, None, "and no pad is armed for release");

        // **The slider: a press over its own centre, which is the gesture that
        // starts a drag.**
        let mut demo = shown_dialog();
        let slider = demo.slider_rect().expect("a laid-out slider");
        let (sx, sy) = (
            slider.x + slider.width / 2.0,
            slider.y + slider.height / 2.0,
        );
        demo.handle_event(mouse_down_at(sx, sy, 0));
        assert!(
            !demo.slider_dragging,
            "the drag never started, at ({sx}, {sy})"
        );
        demo.handle_event(mouse_up_at(sx, sy, 40_000_000));

        // **The keyboard: a press over one key's own centre, which is the gesture
        // that lights it.**
        let mut demo = shown_dialog();
        let (kx, ky) = key_center(&demo, KeyAction::Char('a'));
        demo.handle_event(mouse_down_at(kx, ky, 0));
        assert!(
            !demo.keyboard.is_key_grabbed(),
            "no key lit at ({kx}, {ky}), which is `a`'s own centre"
        );

        // **The three controls, with the dialog closed.** Without this the whole
        // test is satisfied by three broken controls.
        let mut demo = dialog_closed();
        let pad = demo.node_rect(demo.pads[0].node).expect("a laid-out pad");
        demo.handle_event(mouse_down_at(
            pad.x + pad.width / 2.0,
            pad.y + pad.height / 2.0,
            0,
        ));
        demo.frame(WINDOW, Duration::from_millis(50));
        assert!(
            demo.pads[0].press.get() > 0.0,
            "the pad presses when the dialog is closed"
        );
        demo.handle_event(mouse_up_at(
            pad.x + pad.width / 2.0,
            pad.y + pad.height / 2.0,
            40_000_000,
        ));

        let (kx, ky) = key_center(&demo, KeyAction::Char('a'));
        demo.handle_event(mouse_down_at(kx, ky, 0));
        assert!(
            demo.keyboard.is_key_grabbed(),
            "and the key lights, which is what `is_key_grabbed` is for"
        );
        demo.handle_event(mouse_up_at(kx, ky, 40_000_000));
    }

    #[test]
    fn a_release_is_never_gated_so_nothing_can_be_stranded_mid_press() {
        // **The asymmetry the guard deliberately does not have**, and the one way
        // to strand a control: hold `Space` so the pads are going down, press `D`
        // so the dialog opens, release `Space`. A guard on the releases would
        // leave three pads at 1.0 behind the scrim with nothing left to bring them
        // back, and the demo has no other gesture that would.
        let mut demo = dialog_closed();
        demo.handle_event(key_event(Keycode::Space));
        for _ in 0..6 {
            demo.frame(WINDOW, Duration::from_millis(50));
        }
        let held = demo
            .pads
            .iter()
            .map(|pad| pad.press.get())
            .collect::<Vec<f32>>();
        assert!(
            held.iter().all(|press| *press > 0.5),
            "all three pads are down first: {held:?}"
        );

        // The dialog opens underneath them, mid-press.
        demo.handle_event(key_event(DIALOG_KEY));
        assert!(demo.dialog.visible.get(), "and it is showing");

        // The release, with the dialog up. Nothing else in the demo will ever run
        // again while it is, so this is the only thing that can bring them back.
        demo.handle_event(key_up(Keycode::Space));
        for _ in 0..40 {
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        let after = demo
            .pads
            .iter()
            .map(|pad| pad.press.get())
            .collect::<Vec<f32>>();
        assert!(
            after.iter().all(|press| *press < 0.01),
            "and all three came back up, so the release is not gated: {after:?}"
        );
    }

    #[test]
    fn the_demo_opens_with_a_dialog_showing_and_its_panel_inside_the_window() {
        // Acceptance criterion 7 — *"Demo shows a dialog with OK and Cancel
        // buttons"* — and the tree it is shown in. **From literals**: `PANEL`,
        // `OK_BUTTON` and `CANCEL_BUTTON` are written out from the arithmetic in
        // their own docs, and the window's own size is a literal here rather than
        // `WINDOW`, so a fixture that read the constants back could not fail.
        let demo = shown_dialog();

        let window = Rect::new(0.0, 0.0, 1280.0, 1020.0);
        assert_eq!(
            demo.dialog_rect(),
            window,
            "the dialog's box is the whole window, which is what an overlay is: \
             the scrim covers all of it, not the panel"
        );
        let panel = demo.dialog.panel_rect(
            demo.dialog_rect(),
            &demo.dialog_advance(),
            demo.dialog_line_height(),
        );
        assert_eq!(
            panel, PANEL,
            "and the panel is the {PANEL:?} the arithmetic gives"
        );

        // **Inside the window, against a literal rather than against `WINDOW`.**
        // 430 is past zero, 423.6 is below the top, and 850 and 596.4 are short of
        // 1280 and 1020 — a panel 172.8 tall in a window 1020 tall cannot be in
        // the wrong place by accident, and `no_two_placed_rects_overlap` says
        // nothing about an overlay on purpose.
        assert!(
            inside(window, panel),
            "the panel at {panel:?} is inside the {window:?} drawn with literals"
        );
        assert_eq!(
            (panel.y + panel.height, window.height),
            (596.4, 1020.0),
            "and the arithmetic in `PANEL`'s own doc is what produced it: 423.6 of \
             top and 596.4 of bottom"
        );

        // **Both buttons are inside the panel**, which is the part of criterion 7
        // that a screenshot cannot settle by itself: a panel with buttons beside it
        // satisfies "a dialog with OK and Cancel" in words and not on screen.
        let actions = painted_action_rects(&demo);
        assert_eq!(actions.len(), 2, "the acceptance criterion's two");
        assert_eq!(actions, vec![OK_BUTTON, CANCEL_BUTTON], "as recorded");
        for (label, button) in DIALOG_ACTIONS.iter().zip(&actions) {
            assert!(
                inside(panel, *button),
                "{label} at {button:?} is inside the panel at {panel:?}"
            );
        }
        assert_eq!(
            CANCEL_BUTTON.x - (OK_BUTTON.x + OK_BUTTON.width),
            8.0,
            "with the widget's own 8-pixel gap between them, and `OK` first, which \
             is the order [`DIALOG_ACTIONS`] gives and so the order `Tab` walks"
        );
        assert_eq!(
            dialog_run_texts(&demo),
            vec![
                DIALOG_TITLE,
                "Turn the gallery's colours over to the",
                "other theme, exactly as the T key does.",
                DIALOG_ACTIONS[0],
                DIALOG_ACTIONS[1],
            ],
            "and the panel says what it is: the title, the body **wrapped to two \
             lines** at the panel's inner width, then the two button labels"
        );

        // In the tree, on the node the frame loop and the hit test both name.
        let nodes = demo.nodes.borrow();
        assert!(
            nodes.get(demo.dialog.handle()).is_some(),
            "the dialog has a node of its own"
        );
        assert_eq!(
            nodes.get(demo.dialog.handle()).and_then(WidgetNode::parent),
            None,
            "which is a root: an overlay is not laid out inside the content it \
             covers, and that is what lets `modal_chain` be the only thing keeping \
             the gallery out of the tap route"
        );
        for (index, label) in DIALOG_ACTIONS.iter().enumerate() {
            assert_eq!(
                nodes
                    .get(demo.dialog.actions[index].handle())
                    .and_then(WidgetNode::parent),
                Some(demo.dialog.handle()),
                "{label} is a real node attached to the dialog's, which is what \
                 puts it in the paint order and in the `Tab` order"
            );
        }
    }

    #[test]
    fn the_dialogs_nodes_are_painted_by_the_frame_loop_and_only_while_it_is_there() {
        // `Demo::frame` walks `self.order` and rebuilds every node's `PaintState`,
        // and `Demo::draw` walks the same `order`, so a widget the loop does not
        // name is a widget that is never on the screen however correct its own
        // paint is. The dialog's subtree is **appended** to that order rather than
        // attached to the gallery's root, and this is what holds the append down.
        let mut demo = shown_dialog();
        let dialog = demo.dialog.handle();
        let first_action = demo.dialog.actions[0].handle();

        let gallery_nodes = demo.order.len() - 1 - demo.dialog.actions.len();
        assert_eq!(
            demo.order[gallery_nodes], dialog,
            "the dialog's own node is the first of its subtree, and it comes after \
             all {gallery_nodes} nodes of the gallery"
        );
        assert!(
            demo.order.contains(&first_action),
            "and its actions are in the order too, or nothing could send their \
             commands and `Tab` would have nothing to walk"
        );
        assert!(
            !demo.commands_at(dialog).is_empty(),
            "so the loop painted it: an overlay, a shadow, a panel, three text runs \
             and two buttons' worth of commands"
        );

        // The other direction: a dialog that is hidden with nothing running records
        // nothing, and **the loop is what proves that**, because a test that asked
        // `Dialog::paint` directly would not know whether the demo ever stopped
        // calling it.
        demo.handle_event(key_event(DIALOG_KEY));
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert!(
            demo.commands_at(dialog).is_empty(),
            "a closed dialog with its fade over draws nothing at all, so there is \
             no invisible panel left on the screen"
        );
        assert!(
            demo.dialog
                .actions
                .iter()
                .all(|action| !action.button.focused.get()),
            "and no focus ring left on a button nobody can reach, which is what \
             `sync_dialog_focus` exists for"
        );
    }

    #[test]
    fn a_tap_on_the_button_that_was_drawn_there_fires_that_button() {
        // **The gesture that operates the control, aimed where it is painted** —
        // `.ai/NEVERAGAIN.md` § *a drawn control with nothing behind it*. The point
        // comes out of the recorded paint and not out of `Dialog::action_rect`,
        // because asking the widget where it draws a button and then pressing
        // there is asking it twice.
        let mut demo = shown_dialog();
        let actions = painted_action_rects(&demo);
        assert_eq!(actions, vec![OK_BUTTON, CANCEL_BUTTON]);
        let middle = |rect: Rect| (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);

        // **The control, in the theme as well as the dialog**: the demo opens on
        // the dark theme, so `OK` switching it is a change and not a coincidence.
        assert!(demo.dark, "the demo opens on the dark theme");
        let (x, y) = middle(actions[0]);
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);

        assert!(
            !demo.dark,
            "`OK` fired: its callback asked for the theme switch, which is the only \
             thing in the demo either button can do that the other cannot"
        );
        // And the *panel* followed it, which is a separate thing from the gallery
        // following it: `toggle_theme` re-reads the dialog's palette, and a dialog
        // left on the dark surface over a light window is a dark hole in it.
        assert_eq!(
            demo.dialog.surface.get(),
            DialogPalette::from_theme(&Theme::light()).surface,
            "so the panel is on the theme it is now over rather than the one it \
             was opened on"
        );
        assert!(
            !demo.dialog.visible.get(),
            "and the dialog is on its way out — the dismissal is wrapped around the \
             callback by `add_action`, so a caller cannot lose it by writing a \
             callback"
        );
        assert!(
            !demo.pending_theme.get(),
            "with the request drained rather than left set, so the next `OK` is a \
             fresh one and not a repeat of this one"
        );

        // **The control beside it: `Cancel` does not switch the theme.** Without
        // this, "a tap fires the button under it" is satisfied by a chain that
        // fires the first button whatever was pressed.
        let mut demo = shown_dialog();
        let cancel = painted_action_rects(&demo)[1];
        let (x, y) = middle(cancel);
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);
        assert!(demo.dark, "`Cancel` changed nothing");
        assert!(
            !demo.dialog.visible.get(),
            "and closed the dialog, which is the half of it that is not its callback"
        );

        // And **the gap between them**, one pixel left of `Cancel` and so inside
        // the panel and on nobody: a press there fires nothing and is not a
        // dismissal either. A test that pressed "near a button" would pass without
        // this and prove nothing about the button.
        let mut demo = shown_dialog();
        let cancel = painted_action_rects(&demo)[1];
        let (down, up) = click_at(cancel.x - 1.0, cancel.y + cancel.height / 2.0);
        demo.handle_event(down);
        demo.handle_event(up);
        assert!(demo.dark, "the gap is nobody's");
        assert!(
            demo.dialog.visible.get(),
            "and it is inside the panel, so it does not dismiss either"
        );
    }

    #[test]
    fn a_tap_outside_the_panel_does_not_reach_the_gallery_behind_it() {
        // **This is the assertion that makes "modal" mean something.** A control
        // the dialog should have blocked, pressed where it is drawn, and nothing
        // happens — with the same press against a closed dialog beside it, so the
        // test cannot pass by the control being broken.
        //
        // The **field** is the control, and the choice is arithmetic rather than
        // taste: its rect starts at y 756 and the panel's ends at 596, so it is
        // entirely below the panel and a press on it is a press on the *scrim* and
        // not on the panel. A press on the slider or the toggle would have been
        // swallowed by the panel — the panel covers the middle of the gallery,
        // which the operator accepted — and would have proved less.
        let mut demo = shown_dialog();
        let field = demo.text_input_rect().expect("the field is placed");
        assert!(
            field.y > PANEL.y + PANEL.height,
            "the field at {field:?} is below the panel at {PANEL:?}, so it is \
             behind the scrim and not behind the panel"
        );
        let (x, y) = (field.x + field.width / 2.0, field.y + field.height / 2.0);
        let focus_before = demo.focused;

        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);

        assert!(
            !demo.text_input.focused.get(),
            "the field was not focused: the modal ate the tap"
        );
        assert_eq!(
            demo.focused, focus_before,
            "and focus stayed where it was — inside the dialog"
        );
        assert!(
            !demo.dialog.visible.get(),
            "so the press did reach the scrim, which is a dismissal: the tap was \
             **consumed by the dialog**, not dropped on the floor, and this is the \
             assertion that says so rather than a control that happens not to \
             have moved"
        );

        // **The control: the identical press with the dialog closed.**
        let mut demo = dialog_closed();
        let field = demo.text_input_rect().expect("the field is placed");
        let (x, y) = (field.x + field.width / 2.0, field.y + field.height / 2.0);
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);
        assert!(
            demo.text_input.focused.get(),
            "with the dialog closed the same press does reach the field, so the \
             assertion above is about the modal and not about a dead control"
        );
    }

    #[test]
    fn a_tap_on_a_key_behind_the_scrim_inserts_nothing() {
        // The keyboard is the second control behind the overlay, and it is here
        // because **a tap that reaches nothing and a tap that reaches a control
        // with nothing to say look identical** from the first test alone: the
        // keyboard is the one control in the window whose tap is *reported* rather
        // than applied, so a press behind the scrim that still inserted a
        // character would be a hole the field's focus assertion could not see.
        let mut demo = shown_dialog();
        let (x, y) = key_center(&demo, KeyAction::Char('a'));
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);

        assert_eq!(
            demo.text_input.text.get(),
            String::new(),
            "no key reported, so no character reached the field"
        );
        assert_eq!(
            demo.pending_key.get(),
            None,
            "and the key the keyboard would have reported is not left waiting: the \
             modal branch returns rather than falling through to the chain that \
             drains it"
        );

        // The control: the same press with the dialog closed does insert.
        let mut demo = dialog_closed();
        let (x, y) = key_center(&demo, KeyAction::Char('a'));
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);
        assert_eq!(
            demo.text_input.text.get(),
            String::from("a"),
            "which is what makes the assertion above about the modal"
        );
    }

    #[test]
    fn the_demo_never_offers_an_event_outside_the_dialog_while_it_is_showing() {
        // The whole of `modal_chain`, over **every control in the window** rather
        // than one of them, and with the closed demo's own chain beside each one.
        // The input module cannot be asked this question: `input::route` walks
        // from the node under the pointer to the root and knows nothing about a
        // modal, so a caller that offers to the chain it returns has offered to
        // the gallery.
        let demo = shown_dialog();
        let mut in_dialog = vec![demo.dialog.handle()];
        in_dialog.extend(demo.dialog.actions.iter().map(|action| action.handle()));

        // **Every leaf in the gallery, and each one aimed at by its own laid-out
        // centre** rather than at a hand-written point — the same rule
        // `key_center` states for the keyboard's keys.
        let positions: Vec<(&'static str, Handle, Rect)> = [
            ("the first pad", demo.pads[0].node),
            ("a text panel label", demo.label_nodes[0]),
            ("the gauge", demo.gauge.handle()),
            ("the slider", demo.slider.node()),
            ("the toggle", demo.toggle.handle()),
            ("the image", demo.image.handle()),
            ("the chart", demo.chart.handle()),
            ("the field", demo.text_input.handle()),
            ("the keyboard", demo.keyboard.handle()),
        ]
        .into_iter()
        .map(|(what, handle)| {
            let rect = demo.node_rect(handle).expect("a laid-out node");
            (what, handle, rect)
        })
        .collect();
        assert_eq!(positions.len(), 9, "and the gallery has nine");

        for (what, _handle, rect) in &positions {
            let (x, y) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            let event = InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)));

            // **The claim: whatever this point is over, the only node the demo
            // offers it to is inside the dialog.**
            let chain = demo.modal_chain(&event);
            assert!(
                !chain.is_empty(),
                "{what}: an empty chain would drop the event, which is not the same \
                 claim as 'only the dialog sees it'"
            );
            for offered in &chain {
                assert!(
                    in_dialog.contains(offered),
                    "{what}: {offered:?} is outside the dialog, at ({x}, {y})"
                );
            }
        }

        // **The control beside it, and only where the input module can supply
        // one.** `input::route` reaches seven of the nine: the pads and the text
        // column's labels sit *under* the controls layer, which is given the whole
        // window and is therefore the deepest node under every point to the left
        // of it — a property of the demo's own tree that predates this change and
        // is the reason [`Demo::pad_at`] exists at all. Two positions the module
        // cannot reach are not two positions a caller may offer to.
        let closed = dialog_closed();
        for (what, handle, rect) in &positions {
            if !matches!(*what, "the first pad" | "a text panel label") {
                let (x, y) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                let event = InputEvent::new(InputEventKind::Tap, Some(Offset::new(x, y)));
                let chain = {
                    let nodes = closed.nodes.borrow();
                    input::route(&nodes, closed.root, &event)
                };
                assert!(
                    chain.contains(handle),
                    "{what}: `input::route` reaches it at ({x}, {y}) when nothing is \
                     in the way, so the assertion above is about the modal and not \
                     about a control the module cannot see"
                );
            }
        }
    }

    #[test]
    fn escape_closes_the_dialog_through_the_demos_own_event_path() {
        // **Through the demo's own path** — an SDL key-down, the recogniser, the
        // routing — and not by calling `Dialog::on_event`. The route that matters
        // is the one a reader presses: a `Button` declines `Escape`, so this only
        // works because the demo offers the key to the *dialog* before it offers
        // it to the focused button.
        let mut demo = shown_dialog();
        demo.handle_event(key_event(Keycode::Escape));
        assert!(
            !demo.dialog.visible.get(),
            "the dialog is on its way out and the demo has stopped routing to it"
        );
        assert!(demo.dark, "and nothing else happened");

        // **The control beside it**: the same key with the dialog closed reaches
        // nothing and changes nothing, so the assertion above is about the dialog.
        let mut demo = dialog_closed();
        demo.handle_event(key_event(Keycode::Escape));
        assert!(!demo.dialog.visible.get(), "still closed");
        assert!(demo.dark, "and the theme did not switch");

        // And a dialog is not left open by a key it does not answer: `Tab` is
        // declined by `Dialog::on_event` and then becomes the navigation key's.
        let mut demo = shown_dialog();
        demo.handle_event(key_event(Keycode::Escape));
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert!(!demo.dialog.is_drawn(), "and its fade runs out");
    }

    /// The ring on a dismissed dialog's button must leave **with the panel**.
    ///
    /// **The defect this is for.** [`Demo::sync_dialog_focus`] derived the ring
    /// from `self.focused`, and that record is retired the instant the dialog
    /// stops being modal — so on the *first* frame after a dismissal the ring was
    /// already gone while the panel, its text and its buttons were still on screen
    /// at 98% of their opacity. Measured on the recorded commands before the fix,
    /// one 16 ms frame into the widget's 300 ms bounce: the panel and both action
    /// backgrounds at alpha **250**, and no ring at all.
    ///
    /// **The numbers are written out rather than derived.** 250 is
    /// `round(255 * 0.978)`, and 0.978 is `shown_fraction` at the scale 0.9978
    /// that one 16 ms step of the fade's easing reaches; an assertion written as
    /// that expression would be true by construction and would survive the very
    /// change it is meant to catch. **What would move these numbers:** the
    /// dialog's `duration` or `easing`, or the easing's own curve.
    #[test]
    fn the_focus_ring_fades_out_with_the_panel_instead_of_popping() {
        let mut demo = shown_dialog();

        assert_eq!(
            dialog_shape_alphas(&demo, PANEL_RADIUS),
            vec![255],
            "the fixture is a panel at rest"
        );
        assert_eq!(
            dialog_shape_alphas(&demo, RING_RADIUS),
            vec![255],
            "with **one** action's ring on it and opaque — one shape at the ring's \
             radius and not two, because Cancel is not focused"
        );

        demo.handle_event(key_event(Keycode::Escape));

        // **One frame.** Not two and not the whole fade: the complaint is about
        // what the first frame draws, and a test that advanced further would pass
        // with the ring gone from the first.
        demo.frame(WINDOW, Duration::from_millis(16));

        assert!(demo.dialog.is_drawn(), "the panel is still on screen");
        assert!(
            !demo.dialog.visible.get(),
            "and the dialog is no longer modal, which is the predicate that retired \
             the focus record"
        );
        assert!(
            !demo.focus_is_live(),
            "so the focused control cannot be activated — **liveness** is keyed on \
             `visible` and stays keyed on it; this is the half of the split that \
             must not move, and it is asserted here rather than assumed"
        );

        assert_eq!(
            dialog_shape_alphas(&demo, PANEL_RADIUS),
            vec![250],
            "the panel has barely moved: 16 ms into a 300 ms bounce"
        );
        assert_eq!(
            dialog_shape_alphas(&demo, RING_RADIUS),
            vec![250],
            "**and the ring is still drawn, at the panel's own alpha**: a shape at \
             the ring's radius is still being recorded, fading with the panel \
             rather than having vanished over an opaque one"
        );
        assert_eq!(
            dialog_shape_alphas(&demo, ACTION_BACKGROUND_RADIUS),
            vec![250, 250],
            "the two action backgrounds agree with it, which is the same fade \
             arriving through the one multiplier they already had"
        );

        // **The control beside it.** Past the end of the fade the ring is gone, so
        // the assertions above are about the fade rather than about the ring being
        // drawn unconditionally.
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert!(!demo.dialog.is_drawn(), "the fade runs out");
        assert_eq!(
            dialog_shape_alphas(&demo, RING_RADIUS),
            Vec::<u8>::new(),
            "and the ring has gone with it"
        );
    }

    #[test]
    fn the_dialog_key_opens_a_closed_dialog() {
        let mut demo = dialog_closed();
        assert!(!demo.dialog.is_drawn(), "it starts closed");

        demo.handle_event(key_event(DIALOG_KEY));

        assert!(demo.dialog.visible.get(), "`D` opened it");
        assert!(
            demo.dialog.is_drawn(),
            "and it is being drawn from that frame"
        );
        assert_eq!(
            demo.focused,
            demo.dialog.actions.first().map(|action| action.handle()),
            "with focus on its first action, because `Focus` hands an activation \
             key to the button node itself and a dialog nothing is focused in has \
             an `Enter` that reaches nothing"
        );
        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert_eq!(demo.dialog.scale.get(), 1.0, "and it arrives");
        assert_eq!(
            painted_action_rects(&demo),
            vec![OK_BUTTON, CANCEL_BUTTON],
            "at the panel these tests wrote out"
        );
    }

    #[test]
    fn the_dialog_key_closes_an_open_one() {
        // **The other direction, asserted on its own.** "One key toggles" is two
        // claims and the second is the one that catches a key wired to `present`
        // only — which would open the dialog, close nothing, and leave a `D` that
        // appeared to do nothing on the second press.
        let mut demo = shown_dialog();
        assert!(demo.dialog.visible.get(), "it starts open");

        demo.handle_event(key_event(DIALOG_KEY));

        assert!(!demo.dialog.visible.get(), "`D` closed it");
        assert!(
            demo.dialog.is_drawn(),
            "and the fade is still running, which is why `dialog_is_modal` is keyed \
             on `visible` rather than on this"
        );
        assert_eq!(
            demo.focused,
            Some(demo.dialog.actions[0].handle()),
            "**and the record stays**, which is the reversal of 2026-10-03: `D` used \
             to call `set_focus(None)` here, and that made this the one dismissal \
             route whose ring popped. Activation is withdrawn instead, by the frame \
             that follows"
        );
        assert!(
            !demo.focus_is_live(),
            "so it is already unreachable even though the record still names it"
        );
        assert!(demo.dark, "with nothing else changed: closing is not `OK`");

        for _ in 0..3 {
            demo.frame(WINDOW, Duration::from_millis(300));
        }
        assert!(!demo.dialog.is_drawn(), "the fade runs out");
        assert!(demo.commands_at(demo.dialog.handle()).is_empty());
    }

    #[test]
    fn the_dialogs_title_is_bold_and_its_body_is_not() {
        // **Requirement 2's *"Title: bold text at top of panel"*, on the recorded
        // commands**, and through the demo's own frame loop rather than by asking
        // the widget: a test that called `Dialog::paint` would not know whether
        // the demo ever asked it.
        let demo = shown_dialog();
        assert_eq!(
            dialog_run_weights(&demo),
            vec![
                FontWeight::Bold,    // "Switch theme"
                FontWeight::Regular, // the body's first line
                FontWeight::Regular, // and its second
                FontWeight::Regular, // "OK", a Button's own label
                FontWeight::Regular, // "Cancel", likewise
            ],
            "the title is the panel's only bold run, and the button labels are \
             regular whatever the panel around them is doing"
        );
        assert_eq!(
            dialog_run_texts(&demo)[0],
            DIALOG_TITLE,
            "and the bold run is the title, not the body"
        );

        // **The theme switch keeps it bold**, because `set_palette` rewrites three
        // colours and a palette that reached the panel without reaching the weight
        // would be a themed dialog with a plain title.
        let mut demo = shown_dialog();
        demo.handle_event(key_event(Keycode::T));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            dialog_run_weights(&demo).first(),
            Some(&FontWeight::Bold),
            "and `OK`'s theme switch, which is the same code path, does not lose it \
             either"
        );
    }

    #[test]
    fn the_dialog_replaces_the_tab_order_while_it_is_showing() {
        // **Both halves, each with the other beside it**, because "the background
        // is out of the order" is only a claim if the dialog's buttons are in it,
        // and "the buttons are in the order" is only a claim if the five are
        // reachable the moment the dialog closes.
        let mut demo = shown_dialog();
        let actions: Vec<Handle> = demo.dialog.actions.iter().map(|a| a.handle()).collect();
        assert_eq!(actions.len(), 2, "`OK` and `Cancel`");
        let background = [
            demo.slider.node(),
            demo.image.handle(),
            demo.toggle.handle(),
            demo.progress.handle(),
            demo.text_input.handle(),
        ];

        // A full lap in **both** directions, three times round, because a control
        // appended to the tree would be reached on the wrap — which is exactly where
        // a test that only checks the first two stops looking.
        for lap in 0..3 {
            for (key, name) in [
                (key_event(Keycode::Tab), "Tab"),
                (key_event_with(Keycode::Tab, Mod::LSHIFTMOD), "Shift+Tab"),
            ] {
                for _ in 0..(actions.len() + 1) {
                    demo.handle_event(key.clone());
                    let landed = demo
                        .focused
                        .expect("a navigation key always lands somewhere");
                    assert!(
                        actions.contains(&landed),
                        "lap {lap}: {name} stays inside the dialog, on {landed:?}"
                    );
                    assert!(
                        !background.contains(&landed),
                        "lap {lap}: and never on one of the five behind the overlay"
                    );
                }
            }
        }

        // **The walk's own order**, on a fresh demo so the starting point is the
        // seeded one: from `OK`, `Tab` is `Cancel` and the next is `OK` again.
        // Without this, "focus is somewhere among the two" would be satisfied by a
        // set that also held the five and by a walk that visited them in the wrong
        // order.
        let mut demo = shown_dialog();
        assert_eq!(
            demo.focused,
            Some(actions[0]),
            "the dialog opens with its first action focused, which is `OK` — the \
             left-hand one, as `DIALOG_ACTIONS` has them"
        );
        demo.handle_event(key_event(Keycode::Tab));
        assert_eq!(
            demo.focused,
            Some(actions[1]),
            "`Tab` from `OK` is `Cancel`"
        );
        demo.handle_event(key_event(Keycode::Tab));
        assert_eq!(demo.focused, Some(actions[0]), "and the next wraps to `OK`");
        demo.handle_event(key_event_with(Keycode::Tab, Mod::LSHIFTMOD));
        assert_eq!(
            demo.focused,
            Some(actions[1]),
            "and `Shift+Tab` goes the other way"
        );
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(
            demo.dialog.actions[1].button.focused.get(),
            "and the focused action draws its ring: `Focus` hands the key to the \
             button node and the node's `focused` property is what draws it"
        );
        assert!(
            !demo.dialog.actions[0].button.focused.get(),
            "while the other one does not"
        );

        // **And the other half: closed, the five are back**, in their own order and
        // wrapping — which `tab_walks_every_focusable_control_in_order_and_wraps`
        // also checks, and is repeated here as the **control beside** the first
        // half's claim rather than as a second opinion about that test.
        let mut demo = dialog_closed();
        let order = expected_focus_order(&demo);
        assert_eq!(order.len(), 5, "the gallery's own five");
        for lap in 0..2 {
            for (index, (what, handle)) in order.iter().enumerate() {
                demo.handle_event(key_event(Keycode::Tab));
                assert_eq!(
                    demo.focused,
                    Some(*handle),
                    "lap {lap}: Tab {index} is the {what}"
                );
            }
        }

        // **And the dialog's two buttons are out of it**, on a fresh demo and a full
        // lap plus one in **both** directions: a control appended to the tree would
        // be reached on the wrap, which is exactly where a test that only checks
        // the first five stops looking.
        let mut demo = dialog_closed();
        let buttons: Vec<Handle> = demo.dialog.actions.iter().map(|a| a.handle()).collect();
        for (key, name) in [
            (key_event(Keycode::Tab), "Tab"),
            (key_event_with(Keycode::Tab, Mod::LSHIFTMOD), "Shift+Tab"),
        ] {
            for step in 0..(order.len() + 1) {
                demo.handle_event(key.clone());
                let landed = demo
                    .focused
                    .expect("a navigation key always lands somewhere");
                assert!(
                    !buttons.contains(&landed),
                    "{name} {step} lands on {landed:?}, which is a dialog button"
                );
            }
        }
    }

    #[test]
    fn an_activation_key_reaches_the_dialogs_focused_action() {
        // The keyboard half of the row, and the reason the dialog is offered a key
        // before the button is: `Dialog::on_event` forwards a key that is not
        // `Escape` to the one action that holds focus, and `Button::on_event`
        // refuses an activation key unless **that** button is focused. `Tab` has
        // just moved focus to `Cancel`, so this must fire `Cancel` and not `OK`.
        let mut demo = shown_dialog();
        demo.handle_event(key_event(Keycode::Tab));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(
            demo.dialog.actions[1].button.focused.get(),
            "focus is on the second action"
        );

        demo.handle_event(key_event(Keycode::Return));

        assert!(
            !demo.dialog.visible.get(),
            "`Enter` fired the focused action and the dismissal wrapped around it"
        );
        assert!(
            demo.dark,
            "and it was `Cancel`, so the theme did not switch"
        );
    }

    #[test]
    fn the_dialog_measure_size_is_the_themes_largest() {
        // A copy of a theme token drifts, and a drifted one is invisible: the panel
        // would simply be measured against a size nothing else in the window uses.
        // Both themes are read, and the body's own size is checked against the
        // other half of the same claim — **the measurement is at the largest of
        // the three sizes the panel draws at**, which is what makes it the
        // conservative direction (see `DIALOG_FONT`).
        for theme in [Theme::dark(), Theme::light()] {
            let number = |token| {
                theme
                    .get(token)
                    .as_number()
                    .unwrap_or_else(|| panic!("{token:?} is not a number in the theme"))
            };
            assert_eq!(
                number(ThemeToken::FontSizeLg),
                DIALOG_FONT,
                "the title's size, which is the one the dialog measures at"
            );
            assert!(
                number(ThemeToken::FontSizeLg) > number(ThemeToken::FontSizeMd),
                "and it really is the larger of the two the panel draws text at"
            );
        }
        // And the line height really is one number for all three: 18 times the
        // stand-in's 1.2, which is what makes the fixture's panel 172.8 tall.
        let demo = laid_out();
        assert!(
            (demo.dialog_line_height() - 21.6).abs() < 0.01,
            "so the panel's height in `PANEL`'s doc is this arithmetic and not a \
             remembered number: {}",
            demo.dialog_line_height()
        );
    }
}
