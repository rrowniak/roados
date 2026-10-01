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
//! one place the demo draws a container — the other five in the tree, the text
//! column, the text panel, the button row, the button band and the root, group
//! children and have no background, which is what a container with no background
//! looks like.
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
//! A band of buttons sits to the right of the text panel: click one and the
//! click counter under the row goes up, or reset it with the third. The middle
//! button is disabled — it swallows a tap, fires nothing, and focus steps over
//! it. The row is driven by the input module rather than by raw events: the
//! gesture recogniser turns an SDL event into the tap or key press it completed,
//! and dispatch routes it to the node under it, which is what makes a button
//! consume the events meant for it. `Tab` and `Shift+Tab` move focus and `Enter`
//! activates the button holding it.
//!
//! Under the slider the band carries the four widgets the primitive tasks added
//! after the buttons: a **toggle**, whose click turns it on and off and whose
//! label says which state it is in; a **progress bar** at half, moved by `[` and
//! `]` and switched into its sliding mode by `P`; a **list of a hundred rows**,
//! which scrolls under a finger, the wheel and the arrow keys, and whose readout
//! names the first row on screen and the length of the free list so that the
//! virtualisation is visible rather than merely asserted; and an **image** in
//! the top right, loaded from `assets/demo.png` and cycled through its four fits
//! by `F`.
//!
//! The window is 1280 by 720 rather than the 1024 by 600 the first three pads
//! and the text panel were laid out for, because those two regions are full: the
//! card of pads ends at x 788 and the text panel's column at x 654, so the four
//! new widgets had nowhere to go. Enlarging the window is the smallest change
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
use ui_core::paint::{DrawCommand, UvRect};
use ui_core::property::Property;
use ui_core::render::context::Context;
use ui_core::render::Renderer;
use ui_core::texture::{Pixels, TextureCache, TextureHandle};
use ui_core::theme::{PropertyValue, Theme, ThemeToken};
use ui_core::widgets::button::{Button, Callback, Motion, Palette};
use ui_core::widgets::container::Container;
use ui_core::widgets::image::{Image, ImageFit, ImageSource};
use ui_core::widgets::keyboard::{KeyAction, Keyboard, Palette as KeyboardPalette};
use ui_core::widgets::label::{Label, LayoutOptions, TextAlign, Truncation, WrapMode};
use ui_core::widgets::list::{ItemFactory, List};
use ui_core::widgets::progress::{Palette as ProgressPalette, Progress};
use ui_core::widgets::scroll::Palette as ScrollPalette;
use ui_core::widgets::slider::{Orientation, Palette as SliderPalette, Slider};
use ui_core::widgets::text_input::{Palette as TextInputPalette, TextInput};
use ui_core::widgets::toggle::{Palette as TogglePalette, Toggle};
// The button band's `Callback` is the payload-free alias of this same type, so
// the demo imports it under a second name: a slider's handler takes the value it
// moved to, and `Callback::from_fn` on the alias would be `Callback<()>`.
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

/// How long the loop blocks waiting for the next event. Nothing moves on
/// screen, so this paces the loop rather than budgeting a frame.
const EVENT_WAIT: Duration = Duration::from_millis(16);

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
const FONT_PATH: &str = "/usr/share/fonts/truetype/lato/Lato-Medium.ttf";

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
/// This is what keeps the column clear of the button band, and it is the reason
/// it is not the panel's own width. The panel is [`TEXT_PANEL`] wide, so a
/// right-aligned label laid out across it ends at 60 + 900 = 960 and runs
/// underneath the band, which starts at [`BUTTON_ORIGIN`]'s 664 — the kind of
/// collision no unit test sees, because both labels and both buttons lay out
/// correctly on their own. Laying the column out at 594 puts its right edge at
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

/// Where the button band sits in the window.
///
/// This is the row's position inside the band's `Absolute` box, and a
/// `Stack` places every one of its children at the origin regardless of the
/// position they declare — so the offset belongs here, on the row, and not on
/// the band itself, which a `Stack` would ignore. Putting it on the band is a
/// mistake that looks right: the band lands on top of the pads, which are a
/// `Stack` child too and so are laid out from the origin, not centred. (An
/// earlier version of this comment claimed they were centred and ran from
/// x = 130 to x = 894; they are not, and the claim came from reading the row's
/// `MainAxisAlignment::Center` as though it had anything to centre inside.)
///
/// The text panel's labels stay left of 660 — the panel is 900 wide from an
/// origin of 60, but its widest line wraps at 594 — and the pads end at y = 140,
/// so the band goes right of the text and below the pads rather than under the
/// text column, which already reaches the bottom of the window.
const BUTTON_ORIGIN: (f32, f32) = (664.0, 396.0);

/// The gap between the buttons in the row.
const BUTTON_SPACING: f32 = 16.0;

/// The font size the buttons, their click counter and the slider's readout are
/// drawn at.
///
/// The panel's labels are at [`TEXT_SIZE_START`]; everything below the pads is
/// short strings rather than a column of prose, and `+` and `-` move the panel
/// alone.
const BUTTON_FONT: f32 = 20.0;

/// How far below the row the click counter sits.
///
/// A row of buttons is 44 tall, so this clears it with room for the counter's
/// own line, and the counter is placed rather than stacked: an `Absolute` box
/// gives each of its children the position it declares, and the row's is
/// already taken by the buttons.
const COUNTER_DROP: f32 = 60.0;

/// The width the click counter is given, which is wide enough for the text it
/// ever shows: it counts up, and a label laid out narrower than its text would
/// wrap it onto a second line.
const COUNTER_WIDTH: f32 = 320.0;

/// Where the slider sits in the window, below the click counter.
///
/// The same column as the button band and the counter above it — right of the
/// text panel, whose widest line ends at `TEXT_PANEL_ORIGIN.0 +
/// TEXT_COLUMN_WIDTH` — and below the counter, whose line ends at
/// [`BUTTON_ORIGIN`]'s 396 plus [`COUNTER_DROP`]'s 60 and its own 24 pixels.
const SLIDER_ORIGIN: (f32, f32) = (664.0, 496.0);

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
/// what the hit test uses — but the right-hand column then stopped fitting: six
/// controls, of which the slider, the toggle and the progress bar are all finger-
/// sized, need more than the 324 pixels between the button row and the bottom of
/// the window, and `no_two_placed_rects_overlap` said so. **This is the ceiling for
/// a bigger slider without moving the progress bar out of this column**, and that
/// trade is the operator's to make rather than an agent's.
const SLIDER_THUMB_RADIUS: f32 = 18.0;

/// How long the demo's slider is, in pixels.
///
/// The widget asks for its own [`DEFAULT_LENGTH`](ui_core::widgets::slider)
/// default of 240; a finger benefits from a longer swipe, and 300 still clears the
/// list's left edge at 1000 by 36 pixels.
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

/// Where the demo's toggle sits, under the slider's readout.
///
/// The readout's own line ends at [`SLIDER_ORIGIN`]'s 496 plus
/// [`SLIDER_READOUT_DROP`]'s 52 and its own 24 pixels, and the toggle's box is
/// the widget's own — a 48-wide track in a 44-tall touch target — so this is the
/// next line that clears it.
const TOGGLE_ORIGIN: (f32, f32) = (664.0, 592.0);

/// Where the label naming the toggle's state sits, beside it.
///
/// Twelve to the right of the toggle's own 48 pixels, and low enough that its
/// line is about the middle of the toggle's height rather than level with its
/// top.
///
/// It follows [`TOGGLE_ORIGIN`] rather than repeating its number: the toggle moved
/// down 20 when the slider grew above it, and a literal here was still sitting at
/// the old 592 — clear of nothing, since the slider's readout now ends there.
const TOGGLE_READOUT_ORIGIN: (f32, f32) = (728.0, TOGGLE_ORIGIN.1 + 20.0);

/// The width the toggle's readout is given: enough for `on, 1 change` and
/// `off, 0 changes` on one line each.
const TOGGLE_READOUT_WIDTH: f32 = 240.0;

/// Where the demo's progress bar sits, the last thing in the left column of the
/// band.
///
/// 668 plus the bar's own 44 pixels is 712, and the window is 720: eight
/// pixels of margin, which is the whole of what is left below the band.
const PROGRESS_ORIGIN: (f32, f32) = (664.0, 668.0);

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
/// where the list is: the bar ends at 904 and the list starts at
/// [`LIST_ORIGIN`]'s 1000, and 96 pixels is not enough for the longest string
/// this label shows.
const PROGRESS_READOUT_ORIGIN: (f32, f32) = (664.0, 640.0);

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

/// Where the demo's list sits, filling the right of the window below the text
/// panel's band.
///
/// The list is a hundred rows tall and the point of it is the **viewport**: a
/// 280-tall box over [`LIST_ITEM_HEIGHT`] rows is ten of them, so ninety of the
/// hundred are not in the tree at all and the readout says so. It starts at 1000
/// because the click counter and the slider's readout are both given
/// [`COUNTER_WIDTH`] and [`SLIDER_READOUT_WIDTH`] and both end at 984, and 16
/// pixels of clearance is the gap `no_two_placed_rects_overlap` checks for.
const LIST_ORIGIN: (f32, f32) = (1000.0, 396.0);

/// The box the list is given: a viewport, not a content height.
///
/// 270 by 280 is ten rows of [`LIST_ITEM_HEIGHT`] exactly, so the list does not
/// open with a row half off the bottom of it.
const LIST_SIZE: Size = Size {
    width: 270.0,
    height: 280.0,
};

/// How many rows the list has.
///
/// A hundred, which is task 18's acceptance criterion and enough that the
/// scrollbar's thumb is a tenth of its groove — visible at a glance, where
/// twenty rows would fill a fifth of it and read as nearly full.
const LIST_ITEM_COUNT: usize = 100;

/// How tall one row is, and therefore how far apart two rows' indices are.
const LIST_ITEM_HEIGHT: f32 = 28.0;

/// How thick the list's scrollbar draws, in pixels.
///
/// **The widget's own default is 6, and the operator called that unusable for the
/// scrollbar for the same reason they called the slider's 6-pixel track
/// unusable** (2026-09-01 after 2026-09-30): *"is too narrow, I have issues with
/// pointing on it with my mouse, so doing that on tablet with a finger is
/// impossible"*. Six pixels is a hairline — correct with a mouse, where precision
/// is free, and wrong with a fingertip, which is roughly 40 across.
///
/// The widget's constant is **not** changed, for the reason its own doc gives:
/// it is a documented baseline, and `Scroll::set_thickness` is what exists to
/// change it. This is that setter doing its job, which is also what makes the
/// two numbers visible side by side. It is the same split, at the same value, as
/// [`SLIDER_TRACK_THICKNESS`] — one operator, one judgement about a finger, two
/// widgets that were both 6.
const SCROLLBAR_THICKNESS: f32 = 12.0;

/// Where the label naming what the list is showing sits, under it.
const LIST_READOUT_ORIGIN: (f32, f32) = (1000.0, 684.0);

/// The width the list's readout is given, which is the list's own width: the
/// line names four numbers and the longest of them is `first 99, live 10, free
/// 0, tap 99`.
const LIST_READOUT_WIDTH: f32 = 270.0;

/// The font size the list's readout and its rows are drawn at.
///
/// Smaller than [`BUTTON_FONT`] for the readout because it is four numbers
/// rather than one, and the list is narrower than the column above it; the rows
/// are at the same size because a row is one short word and 20 pixels of it in a
/// 28-pixel row leaves four either side.
const LIST_FONT: f32 = 16.0;

/// The gap between a row's edge and its text, on the left and the right.
///
/// It is also the row's vertical inset, because the label is painted inside a
/// box inset by the same number all round and a 28-tall row has 20 left over at
/// 16 pixels of type.
const ROW_PADDING: f32 = 8.0;

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
/// found. The text column's last label ends at y 501, the button band's column
/// starts at x 664 and the list's readout is at x 1000, so the strip from
/// (0, 505) to (664, 720) is the one region of the window nothing is in. 684 is
/// the list's readout's own y, so the two lines of numbers at the bottom of the
/// window are on one baseline.
const FPS_READOUT_ORIGIN: (f32, f32) = (60.0, 684.0);

/// The width the frame-rate readout is given.
///
/// **Written out rather than measured from its first string**, which is what
/// every other readout in the band does, because this is the one whose text
/// changes on every frame that moves it: a rect measured from `fps 0, avg 0.0,
/// worst 0 ms` is a rect measured from a number that was true for one frame, and
/// [`read_only_label`]'s ellipsis would then cut the line at the width of the
/// shortest string it ever shows.
///
/// 400 fits the longest line the readout can print — `fps 10000, avg 10000.0,
/// worst 9999 ms` is 37 characters, and a frame cannot be a hundredth of a
/// millisecond long, which is what would give the rates six digits — and it ends
/// at x 460, two hundred pixels clear of the button column at
/// [`BUTTON_ORIGIN`]'s 664. A box wider than the line in it costs nothing: the
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
/// Larger than [`BUTTON_FONT`]'s 20 because this is *the thing being read*, and
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
/// relationship [`SCROLLBAR_THICKNESS`] and [`SLIDER_TRACK_THICKNESS`] have with
/// their widgets' defaults, except that those two were widened and this one is
/// lowered **to** the floor and never under it.
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

/// What a button in the demo's band does when it is clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonAction {
    /// Add one to the click counter.
    Count,
    /// Put the click counter back to zero.
    Reset,
}

/// The buttons in the demo's band: their label, whether they are disabled, and
/// what they do.
///
/// The middle one is disabled on purpose — a control that refuses interaction is
/// half of what the widget is, and it is what shows a disabled button refusing a
/// tap and being stepped over by focus.
const BUTTONS: [(&str, bool, ButtonAction); 3] = [
    ("Press me", false, ButtonAction::Count),
    ("Disabled", true, ButtonAction::Count),
    ("Reset", false, ButtonAction::Reset),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = Renderer::new(Context::new(
        "roados ui_demo",
        f32_to_u32(WINDOW.width),
        f32_to_u32(WINDOW.height),
    )?)?;
    let font = Font::from_path(FONT_PATH)?;
    renderer.set_font(font.clone());
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
        let event = events.wait_event_timeout(EVENT_WAIT);
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
/// is or what the list is showing — it recomputes, and a binding rather than a
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
/// The one place a label is painted, for the panel's seven and the band's six
/// and the list's rows between them. `rect` is the node's own **laid-out** rect
/// converted to the painter's, except for a row — a row's commands are read by
/// [`List::paint`] and translated by it, so a row is painted in its own
/// coordinates and `rect` is the row's local box rather than anything the layout
/// pass placed.
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
/// function for the same reason `scroll::gesture_delta` is one function: the
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

/// Returns the text the demo's list gives row `index`.
///
/// A row is built once and recycled for whatever row comes next, so this is
/// written on the row rather than built into it — see [`Demo::paint_rows`].
fn row_text(index: usize) -> String {
    format!("item {index}")
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

/// The four state properties a button is written in, kept together so the demo
/// can tell whether anything about a button has changed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonFlags {
    /// A pointer is over the button.
    hovered: bool,
    /// The button is held down.
    pressed: bool,
    /// The button refuses interaction.
    disabled: bool,
    /// The button holds focus.
    focused: bool,
}

/// The two things a toggle's drawn appearance is derived from.
///
/// A record rather than a re-aim every frame, for
/// [`Demo::sync_button_state`]'s reason: aiming restarts the transition, so
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

/// A button in the demo's band: the widget, and the state the demo last wrote to
/// it and last aimed it at.
///
/// The two records are what stop a frame from doing work: a property write
/// notifies the node's `on_change` callback and marks the node dirty, and
/// aiming a button restarts its transition, so both are done only when the state
/// or the palette has actually moved.
struct DemoButton {
    /// The button widget, with its node in the arena.
    widget: Button,
    /// The state the demo last wrote to the widget's properties.
    written: ButtonFlags,
    /// The state the widget was last aimed at.
    aimed: ButtonFlags,
}

impl DemoButton {
    /// Returns the button's node in the arena.
    fn node(&self) -> Handle {
        self.widget.handle()
    }

    /// Returns the state the widget is in.
    fn state(&self) -> ButtonFlags {
        ButtonFlags {
            hovered: self.widget.hovered.get(),
            pressed: self.widget.pressed.get(),
            disabled: self.widget.disabled.get(),
            focused: self.widget.focused.get(),
        }
    }

    /// Returns whether the button can hold focus.
    ///
    /// A disabled button cannot: focus belongs to what the user can act on, and
    /// a control that swallows a tap and fires nothing has nothing to be
    /// activated by a key.
    fn is_focusable(&self) -> bool {
        !self.widget.disabled.get()
    }
}

/// Returns a colour property that follows whichever of `tokens` the
/// `color_token` property names.
///
/// Every piece of text in the demo is bound through this, so `C` moves the text
/// panel and the click counter together.
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

/// Returns the click handler for a button in the demo's band, writing to
/// `clicks`.
///
/// The handler holds a clone of the property rather than the demo: a property is
/// a handle to shared state, so the closure reaches the counter the demo shows
/// without the demo being captured, and the button outliving the demo cannot
/// leave a dangling borrow behind it.
fn button_callback(clicks: &Property<u32>, action: ButtonAction) -> Callback {
    let clicks = clicks.clone();
    Callback::new(move || match action {
        ButtonAction::Count => clicks.set(clicks.get() + 1),
        ButtonAction::Reset => clicks.set(0),
    })
}

/// A row of the demo's list: the node the factory built and the label in it.
///
/// The list's factory is given **no index**, because a row is built once and
/// then recycled for whatever row comes next, so a factory that baked an index
/// into its row would put the wrong text on the wrong line. The index is
/// therefore written by the frame, from [`List::visible_items`], onto whichever
/// row currently holds it.
struct DemoRow {
    /// The row's own node, which is the label's: the list gives it the position
    /// and the size and the demo gives it the text.
    node: Handle,
    /// The label inside it, painted in the row's **own** coordinates.
    label: DemoLabel,
}

/// A slider in the demo: the widget, and the dragging state the demo last wrote
/// to it and last aimed it at.
///
/// The two records are the [`DemoButton`] records again, and for the same
/// reason: a property write notifies the node's `on_change` callback and marks
/// the node dirty, and aiming restarts the slider's transition, so both are done
/// only when the state has actually moved.
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
    /// The buttons in the band, with the state the demo last wrote to each.
    buttons: Vec<DemoButton>,
    /// The label showing the click counter, under the row of buttons.
    counter: DemoLabel,
    /// The gesture recogniser the button row's events are built from.
    recognizer: GestureRecognizer,
    /// The button holding focus, or `None` when nothing does.
    focused: Option<Handle>,
    /// The index of the button a pointer is holding down, if any.
    pressed: Option<usize>,
    /// Every node in the demo that has children, as the widget that owns it.
    ///
    /// The tree is built out of [`Container`]s rather than out of nodes the demo
    /// assembles itself, so a parent is the widget and the demo's frame loop
    /// paints it through [`Container::paint`]. Only the row of pads is given a
    /// background: it is the card that shows what a container with a background
    /// and padding looks like, and the other five draw nothing.
    containers: Vec<Container>,
    /// The slider, under the click counter.
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
    /// The list, filling the right of the window.
    list: List,
    /// The rows the list's factory has built, in the order it built them.
    ///
    /// A handle and a label, because a row is built by a closure the demo hands
    /// to the list and the closure cannot reach the [`Demo`] it is being built
    /// for. It is a `RefCell` for the same reason the arena is: a node cannot
    /// reach the tree that holds it, and a factory takes `&mut Arena` rather
    /// than `&mut Demo`.
    rows: Rc<RefCell<Vec<DemoRow>>>,
    /// The first row the list has on screen, for the readout.
    first_visible: Property<usize>,
    /// How many rows the list has in the tree, for the readout.
    live_count: Property<usize>,
    /// How many rows are on the list's free list, for the readout.
    free_count: Property<usize>,
    /// The label naming what the list is showing.
    list_readout: DemoLabel,
    /// Whether a pointer is holding the list down.
    ///
    /// A [`Scroll`](ui_core::widgets::scroll::Scroll) has no grabbed state, so a drag that leaves the list stops
    /// being offered to it — which is the case that matters most, because a
    /// finger that has travelled past the end of a list wants the end of the
    /// list. This is `slider_dragging`'s fix applied to the other widget.
    list_dragging: bool,
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
    /// The mechanism [`List`]'s `on_item_click` uses — a widget whose callback
    /// cannot reach the `Demo` writes a property, and the demo reads it — and the
    /// reason it is a property and not a field is that `Callback` is `Fn`, so it
    /// cannot write a `&mut self` it was not lent.
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
    /// The keyboard's counterpart to [`Demo::list_dragging`], and needed for the
    /// same reason: `Keyboard::on_event` only ever sees a `Tap`, which the
    /// recogniser reports on the *release*, so a key cannot light up from inside
    /// it. `grab_key` and `release_key` are called from the press and the release.
    keyboard_pressed: bool,
    /// What the field last reported through `on_change`, for the readout.
    text_readout: DemoLabel,
    /// What the field last reported through `on_submit`, for the readout.
    submit_readout: DemoLabel,
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

        // The button band, to the right of the text panel: a row of three
        // buttons over a click counter. The band is `Absolute` so the row and
        // the counter each sit at their own position inside it.
        let clicks = Property::new(0u32);
        let counter_text = {
            let clicks = clicks.clone();
            Property::bind(move || {
                // "1 click" rather than "1 clicks": this string is on screen in
                // the demo, and the first count is the one everyone sees.
                let count = clicks.get();
                if count == 1 {
                    "1 click".to_string()
                } else {
                    format!("{count} clicks")
                }
            })
        };
        let mut counter = Label::new(&mut nodes, String::new());
        counter.text = counter_text;
        counter.color = cycling_color(&color_token, &token_properties);
        counter.font_size.set(BUTTON_FONT);
        let counter = DemoLabel {
            label: counter,
            options: LayoutOptions {
                max_width: COUNTER_WIDTH,
                ..LayoutOptions::default()
            },
        };

        let button_line_height = metrics.line_height(BUTTON_FONT);
        let mut buttons = Vec::new();
        for &(text, disabled, action) in &BUTTONS {
            let mut widget = Button::new(&mut nodes, text);
            widget.font_size.set(BUTTON_FONT);
            widget.disabled.set(disabled);
            widget.set_palette(Palette::from_theme(&theme));
            // The palette names the colours the states are derived from, and
            // the properties still hold the neutral defaults `Button::new` wrote,
            // so the button is snapped onto its theme before it is ever drawn.
            widget.snap_to_state();
            widget.on_click = button_callback(&clicks, action);
            // Every button is given the rect its own label and padding need,
            // floored at the minimum touch target, so the band lays out from the
            // widgets' sizes rather than from a size written out here.
            let size = widget.size(
                &|ch: char| metrics.advance(ch, BUTTON_FONT),
                button_line_height,
            );
            nodes
                .get_mut(widget.handle())
                .ok_or("ui_demo: a button node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
            buttons.push(DemoButton {
                widget,
                written: ButtonFlags {
                    hovered: false,
                    pressed: false,
                    disabled,
                    focused: false,
                },
                aimed: ButtonFlags {
                    hovered: false,
                    pressed: false,
                    disabled,
                    focused: false,
                },
            });
        }
        let button_nodes: Vec<Handle> = buttons.iter().map(DemoButton::node).collect();
        let button_row = Container::new(&mut nodes, LayoutMode::row());
        button_row.set_flex_config(
            &mut nodes,
            FlexConfig::new()
                .with_spacing(BUTTON_SPACING)
                .with_cross_axis_alignment(CrossAxisAlignment::Center),
        );
        for &child in &button_nodes {
            if !button_row.add_child(&mut nodes, child) {
                return Err("ui_demo: a button could not be attached to the band");
            }
        }
        // The slider, under the counter, and the readout that shows what it is
        // at. The slider's colours come from the theme the same way the band's
        // do, and it is snapped onto them before it is ever drawn for the reason
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
        readout.font_size.set(BUTTON_FONT);
        let slider_readout = DemoLabel {
            label: readout,
            options: LayoutOptions {
                max_width: SLIDER_READOUT_WIDTH,
                ..LayoutOptions::default()
            },
        };
        {
            let size = slider_readout.size(&metrics, BUTTON_FONT);
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
            BUTTON_FONT,
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
            BUTTON_FONT,
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
            BUTTON_FONT,
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

        // The list, its rows and the label naming what it is showing. The rows
        // come from a factory rather than from a hundred nodes written out here,
        // which is the whole of the virtualisation: the factory is called for the
        // rows on screen and for nothing else, and again only when the free list
        // is empty and a row is genuinely needed.
        let rows: Rc<RefCell<Vec<DemoRow>>> = Rc::new(RefCell::new(Vec::new()));
        let first_visible = Property::new(0usize);
        let live_count = Property::new(0usize);
        let free_count = Property::new(0usize);
        let tapped = Property::new(None);
        let mut list = List::new(&mut nodes, LIST_ITEM_COUNT, LIST_ITEM_HEIGHT);
        {
            let factory_rows = Rc::clone(&rows);
            let row_color = cycling_color(&color_token, &token_properties);
            list.set_item_factory(
                &mut nodes,
                ItemFactory::new(move |nodes: &mut Arena<WidgetNode>| {
                    let mut label = Label::new(nodes, String::new());
                    label.font_size.set(LIST_FONT);
                    label.color = row_color.clone();
                    let node = label.handle();
                    factory_rows.borrow_mut().push(DemoRow {
                        node,
                        label: DemoLabel {
                            label,
                            options: LayoutOptions {
                                max_width: LIST_SIZE.width - ROW_PADDING * 2.0,
                                wrap: WrapMode::None,
                                truncation: Truncation::Ellipsis,
                                ..LayoutOptions::default()
                            },
                        },
                    });
                    node
                }),
            );
            let tapped = tapped.clone();
            list.on_item_click =
                ValueCallback::from_fn(move |index: usize| tapped.set(Some(index)));
            // `List::new` has already told the embedded scroll how tall the
            // content is — the count and the height are the only things that can
            // change either, so they are the only things that write it — and a
            // scroll with no content height clamps every offset to zero, which is
            // a list that does not scroll and draws no scrollbar. `set_palette`
            // is the door for the scrollbar's colours, and `snap_to_state`
            // through `scroll()` is what puts them there at once rather than
            // leaving the scrollbar on the neutral defaults `Scroll::new` wrote.
            let palette = ScrollPalette::from_theme(&theme);
            list.set_palette(palette);
            // The scrollbar is 12 wide here and not the widget's 6, for the same
            // reason the slider's track is: a finger cannot aim at a hairline.
            list.set_scrollbar_thickness(SCROLLBAR_THICKNESS);
            list.scroll().snap_to_state();
        }
        {
            let size = LIST_SIZE;
            nodes
                .get_mut(list.handle())
                .ok_or("ui_demo: the list node is missing")?
                .layout_mut()
                .set_constraints(Constraints::tight(size));
        }
        let list_readout = read_only_label(
            &mut nodes,
            &metrics,
            LIST_FONT,
            LIST_READOUT_WIDTH,
            {
                let first = first_visible.clone();
                let live = live_count.clone();
                let free = free_count.clone();
                let tapped = tapped.clone();
                Property::bind(move || {
                    let tap = match tapped.get() {
                        Some(index) => index.to_string(),
                        None => "-".to_string(),
                    };
                    format!(
                        "first {}, live {}, free {}, tap {tap}",
                        first.get(),
                        live.get(),
                        free.get()
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
            BUTTON_FONT,
            FPS_READOUT_WIDTH,
            fps_text.clone(),
            &color_token,
            &token_properties,
        )?;
        {
            let size = Size {
                width: FPS_READOUT_WIDTH,
                height: metrics.line_height(BUTTON_FONT),
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

        // The row, the counter, the slider, its readout and the four newer
        // widgets are each placed inside the band, which is what `Absolute` is
        // for: each at its own offset from the band's origin, which is the
        // window's own top left.
        nodes
            .get_mut(button_row.handle())
            .ok_or("ui_demo: the button row is missing")?
            .layout_mut()
            .set_position(Some(Offset::new(BUTTON_ORIGIN.0, BUTTON_ORIGIN.1)));
        nodes
            .get_mut(counter.label.handle())
            .ok_or("ui_demo: the click counter is missing")?
            .layout_mut()
            .set_position(Some(Offset::new(
                BUTTON_ORIGIN.0,
                BUTTON_ORIGIN.1 + COUNTER_DROP,
            )));
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
            (list.handle(), LIST_ORIGIN),
            (list_readout.label.handle(), LIST_READOUT_ORIGIN),
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
        // The band is the window, not a box of its own: it is a `Stack` child,
        // and a `Stack` sizes a child from its own constraints but places it at
        // the origin. Giving it the window's size makes the offsets inside it
        // window coordinates, which is what the positions above assume.
        let button_area = Container::new(&mut nodes, LayoutMode::Absolute);
        {
            let band = nodes
                .get_mut(button_area.handle())
                .ok_or("ui_demo: the button band is missing")?;
            band.layout_mut()
                .set_constraints(Constraints::tight(WINDOW));
        }
        // The order the band holds its children in **is** their paint order and
        // therefore their `Tab` order, so it is a list and not a set: the four
        // children of tasks 12 and 14 first, in the order those tasks put them
        // in, and then the four of tasks 15 to 18 in the order this file
        // introduces them. The image comes after the slider and before the
        // toggle because that is where it sits in the window — top right, above
        // the band — and putting it first would move the first `Tab` off the
        // first button, which two existing tests are about.
        if !button_area.add_child(&mut nodes, button_row.handle())
            || !button_area.add_child(&mut nodes, counter.label.handle())
            || !button_area.add_child(&mut nodes, slider.widget.handle())
            || !button_area.add_child(&mut nodes, slider_readout.label.handle())
            || !button_area.add_child(&mut nodes, image.handle())
            || !button_area.add_child(&mut nodes, image_fit_readout.label.handle())
            || !button_area.add_child(&mut nodes, toggle.handle())
            || !button_area.add_child(&mut nodes, toggle_readout.label.handle())
            || !button_area.add_child(&mut nodes, progress.handle())
            || !button_area.add_child(&mut nodes, progress_readout.label.handle())
            || !button_area.add_child(&mut nodes, list.handle())
            || !button_area.add_child(&mut nodes, list_readout.label.handle())
            || !button_area.add_child(&mut nodes, fps_readout.label.handle())
            || !button_area.add_child(&mut nodes, text_input.handle())
            || !button_area.add_child(&mut nodes, text_readout.label.handle())
            || !button_area.add_child(&mut nodes, submit_readout.label.handle())
            || !button_area.add_child(&mut nodes, keyboard.handle())
        {
            return Err("ui_demo: the button band could not be assembled");
        }

        // A stack: the background fills the window behind the row of pads, the
        // text panel and the button band, and all three are painted over it.
        let root = Container::new(&mut nodes, LayoutMode::Stack);
        for &child in &[
            background,
            row.handle(),
            text_panel.handle(),
            button_area.handle(),
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

        // The same link for the buttons: every property a button's transition
        // writes — its background, label colour, scale and opacity — marks its
        // own node dirty, so the next pass repaints it. The button is a widget
        // that owns a clock rather than a property the demo animates, so this is
        // the only thing standing between a button's transition and the screen.
        for button in &buttons {
            let node = button.widget.handle();
            // The two colours first, then the two numbers: one array cannot hold
            // both, and the writes all mark the same node dirty anyway.
            let background = &button.widget.background;
            let foreground = &button.widget.foreground;
            for property in [background, foreground] {
                let nodes = Rc::clone(&nodes);
                property.on_change(move |_| {
                    mark_dirty(&mut nodes.borrow_mut(), node);
                });
            }
            let scale = &button.widget.scale;
            let opacity = &button.widget.opacity;
            for property in [scale, opacity] {
                let nodes = Rc::clone(&nodes);
                property.on_change(move |_| {
                    mark_dirty(&mut nodes.borrow_mut(), node);
                });
            }
        }

        // And for the click counter: the count is a property, the label's text
        // is bound to it, and a write to either marks the counter's node dirty.
        {
            let text_nodes = Rc::clone(&nodes);
            let node = counter.label.handle();
            counter.label.text.on_change(move |_| {
                mark_dirty(&mut text_nodes.borrow_mut(), node);
            });
            let color_nodes = Rc::clone(&nodes);
            counter.label.color.on_change(move |_| {
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

        // The same link for each of the four new readouts, whose text is bound
        // to a widget's own properties: a change to the text or to the colour
        // marks that readout's node dirty, so the next pass repaints it. Their
        // **rects** never change — each was given the box of the string it
        // started with, and every string they go on to show is inside that box
        // by the `wrap: None` `read_only_label` set — so the link is a paint
        // link in effect, and it is kept because it is the pattern the counter
        // and the slider's readout already follow and because a caller who
        // widened one of these labels would need it.
        for readout in [
            &toggle_readout,
            &image_fit_readout,
            &progress_readout,
            &list_readout,
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

        // The toggle, the progress bar, the image and the list get **no**
        // `on_change` links of their own, and that is the same decision the card
        // behind the pads gets above, for the same reason: every property the
        // four animate is a *paint* property — each draws itself inside whatever
        // rect the layout pass gave its node, and none of them is an input to
        // that rect — and `Demo::frame` rebuilds every node's paint state on
        // every frame, so a write to one reaches the screen without a link and a
        // link here would be `mark_dirty` on layout that did not change.
        //
        // The two that do have a *layout* input get one. A readout's text is
        // one, and the four above are linked for it. A row's text is not: the
        // demo writes it and paints the row itself, in the same frame, and the
        // row's rect is the list's arithmetic rather than anything a text change
        // could move — so a link would fire a callback that re-entered an arena
        // the frame already held, which is a `RefCell` panic on the first scroll
        // rather than a wrong picture.

        // The tree never changes shape, so the order is computed once.
        //
        // The list's **rows** are not in it, and cannot be: they do not exist
        // yet, because `List::sync` builds them on the first frame and
        // recycles them after that. That is what keeps a row from being drawn
        // twice — once by this walk, at whatever rect the layout pass gave it,
        // and once by `List::paint`, which reads the same node's commands and
        // translates them into place. The content node they hang from *is* in
        // the order, because `Scroll::new` attached it before this ran.
        let order = paint_order(&nodes.borrow(), root.handle());
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
            buttons,
            counter,
            recognizer: GestureRecognizer::new(),
            focused: None,
            pressed: None,
            containers: vec![row, text_column, text_panel, button_row, button_area, root],
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
            list,
            rows,
            first_visible,
            live_count,
            free_count,
            list_readout,
            list_dragging: false,
            fps: FrameRate::new(),
            fps_text,
            fps_readout,
            text_input,
            pending_key,
            keyboard,
            keyboard_pressed: false,
            text_readout,
            submit_readout,
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
    /// The button band is driven the other way round, through the input module:
    /// the event goes to the [`GestureRecognizer`]
    /// first, and whatever gesture or key it completed is dispatched to the node
    /// under it, which is what lets a button consume the events meant for it
    /// rather than letting them reach whatever is behind. `Tab` and `Shift+Tab`
    /// move focus, and Enter activates the button holding it.
    ///
    /// Space is the one key both halves want. It belongs to the button holding
    /// focus, because that is the control a user has navigated to, and falls
    /// back to the pads when nothing is focused — so the pads still work with
    /// `Tab` never pressed, which is how the demo starts.
    ///
    /// `0` and `1` put the slider at its two ends without a pointer, which is the
    /// one thing a drag cannot show: the thumb travelling to a value it was not
    /// given.
    ///
    /// `F` cycles the image's fit, `[` and `]` move the progress bar's value by
    /// [`PROGRESS_STEP`] either way, and `P` switches the bar into its sliding
    /// mode and back. `0` and `1` and `[` and `]` are the same idea four times
    /// over — a control a pointer cannot reach, moved by a key — which is the one
    /// thing the band was missing before these four widgets were in it.
    fn handle_event(&mut self, event: Event) {
        let produced = self.recognizer.process(&event);

        match event {
            Event::KeyDown {
                keycode: Some(Keycode::T),
                repeat: false,
                ..
            } => self.toggle_theme(),
            Event::KeyDown {
                keycode: Some(keycode),
                repeat: false,
                ..
            } => match keycode {
                Keycode::Space if self.focused.is_none() => self.press_all(),
                Keycode::Equals | Keycode::Plus => {
                    self.set_text_size(self.text_size + TEXT_SIZE_STEP);
                }
                Keycode::Minus => self.set_text_size(self.text_size - TEXT_SIZE_STEP),
                Keycode::C => self.cycle_text_color(),
                Keycode::_0 => self.set_slider_value(SLIDER_MIN),
                Keycode::_1 => self.set_slider_value(SLIDER_MAX),
                Keycode::F => self.cycle_image_fit(),
                Keycode::LeftBracket => self.step_progress(-PROGRESS_STEP),
                Keycode::RightBracket => self.step_progress(PROGRESS_STEP),
                Keycode::P => self.set_progress_indeterminate(!self.progress_indeterminate.get()),
                _ => {}
            },
            Event::KeyUp {
                keycode: Some(Keycode::Space),
                ..
            } => {
                if self.focused.is_none() {
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
                if let Some(index) = self.pad_at(x, y) {
                    self.mouse_pressed = Some(index);
                    self.press_pad(index);
                } else if let Some(index) = self.button_at(x, y) {
                    self.pressed = Some(index);
                } else if self.slider_at(x, y).is_some() {
                    self.slider_dragging = true;
                } else if self.list_at(x, y).is_some() {
                    // The scrollbar's thumb is grabbed **here**, on the press,
                    // and nowhere else: the gesture recogniser reports a tap on
                    // the release and a drag only once the pointer has already
                    // moved, so it has no "the finger went down on the thumb"
                    // for the widget to read. A press that missed the thumb grabs
                    // nothing, and the list keeps scrolling under the finger the
                    // way it always has.
                    if let Some(rect) = self.list_rect() {
                        self.list.scroll().grab_thumb(Offset::new(x, y), rect);
                    }
                    self.list_dragging = true;
                } else if self.keyboard_at(x, y) {
                    // Same reason as the scrollbar's thumb above: the recogniser
                    // has no press to give, so the key has to be lit from here.
                    self.grab_key(Offset::new(x, y));
                }
            }
            Event::MouseButtonUp {
                mouse_btn: MouseButton::Left,
                ..
            } => {
                if let Some(index) = self.mouse_pressed.take() {
                    self.release_pad(index);
                }
                self.pressed = None;
                self.slider_dragging = false;
                self.list_dragging = false;
                self.list.scroll().release_thumb();
                self.release_key();
            }
            // A finger is a pointer too, and a car has no mouse: the same press
            // and release the left button gets, from the touch events SDL delivers
            // for the same gesture. A canceled touch drops the slider and the list
            // as well as the pointer, because a canceled finger is one that is
            // gone.
            Event::FingerDown { x, y, .. } => {
                if self.slider_at(x, y).is_some() {
                    self.slider_dragging = true;
                } else if self.list_at(x, y).is_some() {
                    if let Some(rect) = self.list_rect() {
                        self.list.scroll().grab_thumb(Offset::new(x, y), rect);
                    }
                    self.list_dragging = true;
                } else if self.keyboard_at(x, y) {
                    self.grab_key(Offset::new(x, y));
                }
            }
            Event::FingerUp { .. } | Event::FingerCanceled { .. } => {
                self.slider_dragging = false;
                self.list_dragging = false;
                // A grab that outlived its finger would be the next gesture's,
                // and a drag of the content would move the thumb instead.
                self.list.scroll().release_thumb();
                // A canceled touch releases the key too, for the same reason: a
                // finger that is gone must not leave a key lit.
                self.release_key();
            }
            _ => {}
        }

        for mut input_event in produced {
            self.route_input_event(&mut input_event);
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

        // A drag goes to the slider being dragged first, whether or not the
        // pointer is still over it. The chain below finds the node under the
        // pointer, and a finger that has travelled past the end of a slider is
        // outside it — which is exactly when the slider most needs to hear about
        // the drag, because the answer is its own end of the range.
        //
        // The list is the same case and for the same reason: a `Scroll` has no
        // grabbed state, so a drag that has left the list's own rect would stop
        // reaching it, and a finger halfway down a hundred rows is most of the
        // way outside a viewport ten rows tall.
        if self.slider_dragging && matches!(event.kind(), InputEventKind::Drag { .. }) {
            if let Some(rect) = self.slider_rect() {
                self.slider.widget.on_event(event, rect);
                if event.consumed() {
                    return;
                }
            }
        }
        if self.list_dragging && matches!(event.kind(), InputEventKind::Drag { .. }) {
            if let Some(rect) = self.list_rect() {
                self.list.on_event(event, rect);
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

    /// Offers a positionless event to the control holding focus, and reports
    /// whether it took it.
    fn offer_to_focused(&self, event: &mut InputEvent) -> bool {
        let Some(handle) = self.focused else {
            return false;
        };
        self.offer_to(handle, event)
    }

    /// Offers `event` to the widget at `handle`, and reports whether it took it.
    ///
    /// A handle that belongs to no widget at all is nobody's, which is what lets
    /// one loop serve six kinds of control without asking what is there.
    ///
    /// The two that answer a `KeyDown` only while they hold focus are the slider
    /// and the toggle; the list answers through its own
    /// [`Scroll`](ui_core::widgets::scroll::Scroll), which answers the same way.
    /// So a `Tab` reaches all of them and none of them takes it.
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
        if handle == self.list.handle() {
            return match self.list_rect() {
                Some(rect) => self.list.on_event(event, rect),
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
        self.buttons
            .iter()
            .find(|button| button.node() == handle)
            .is_some_and(|button| button.widget.on_event(event))
    }

    /// Moves focus if `event` is a navigation key, and reports whether it was.
    ///
    /// `Tab` and `Shift+Tab` are the keyboard's navigation, and a gamepad
    /// steering-wheel axis arrives as a `Scroll` with no position, so both reach
    /// this through the same [`Focus`] tracker. A **mouse** wheel is not one of
    /// them: `GestureRecognizer` gives a wheel event the pointer's position, so
    /// it is routed as a positional event and lands on the node under the cursor
    /// rather than here. The focusable set is rebuilt from the buttons and the
    /// five other controls each time, which is what makes a disabled button fall
    /// out of the order rather than sit in it.
    ///
    /// The order itself is the tree's paint order, so the band's children are
    /// what decide it and **the order they were added in is the `Tab` order**:
    /// the two enabled buttons, the slider, the image, the toggle, the progress
    /// bar and the list. `tab_steps_over_the_disabled_button` walks it.
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
            let mut focus = Focus::new(&nodes, self.root);
            for button in &self.buttons {
                focus.set_focusable(button.node(), button.is_focusable());
            }
            // The five that follow the band in the tree's paint order, and so in
            // the `Tab` order. None of them has a disabled state of its own, so
            // every one of them is in the order always — which is not the same as
            // saying every one of them answers every key: `offer_to` is what
            // decides, and a widget with no key of its own declines.
            //
            // The **list's** node is the one registered, not the content node its
            // rows hang from: a row is not a control, and registering the content
            // node would put a `Tab` stop on the inside of a viewport.
            for handle in [
                self.slider.node(),
                self.image.handle(),
                self.toggle.handle(),
                self.progress.handle(),
                self.list.handle(),
                self.text_input.handle(),
            ] {
                focus.set_focusable(handle, true);
            }
            // Re-entering the order where focus already is. `Focus` starts with
            // nothing focused, so without this a wheel turned twice in a row
            // would walk from the top both times, and Shift+Tab from the first
            // button would go forward instead of back.
            if let Some(current) = self.focused {
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
    /// Three of the five that follow the band have a `focused` property to
    /// write — the slider, the toggle and the list's embedded scroll — and the
    /// other two, the progress bar and the image, have none, so their readouts
    /// say where focus is in words instead. See `progress_focused` and
    /// `image_focused`.
    fn set_focus(&mut self, next: Option<Handle>) {
        self.focused = next;
        let focused = self.focused;
        for button in &mut self.buttons {
            let wanted = Some(button.node()) == focused;
            if button.widget.focused.get() != wanted {
                button.widget.focused.set(wanted);
                button.written.focused = wanted;
            }
        }
        let slider_wanted = Some(self.slider.node()) == focused;
        if self.slider.widget.focused.get() != slider_wanted {
            self.slider.widget.focused.set(slider_wanted);
        }
        let toggle_wanted = Some(self.toggle.handle()) == focused;
        if self.toggle.focused.get() != toggle_wanted {
            self.toggle.focused.set(toggle_wanted);
        }
        let list_wanted = Some(self.list.handle()) == focused;
        if self.list.scroll().focused.get() != list_wanted {
            self.list.scroll().focused.set(list_wanted);
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
    /// The **list's** frame comes after the layout pass and not with the rest,
    /// because it is the one step here that needs the rect the pass has just
    /// given it: `List::sync` is what attaches a row, and a row's own size is a
    /// fraction of the viewport's. A frame that skipped it would scroll and
    /// allocate nothing, which is what `List::sync`'s own documentation calls a
    /// list that has never been called.
    ///
    /// The arena is therefore borrowed **twice** rather than once, and the split
    /// is not tidiness. The three numbers the list's readout names are properties
    /// its text is bound to, so writing one recomputes that text, and the text's
    /// own `on_change` link reaches the arena to mark its node dirty — which a
    /// frame already holding the arena refuses, with a `RefCell` panic on the
    /// first frame rather than on anything a test could have found by reading the
    /// numbers. The same is true of every bound readout in the demo, and the
    /// reason the counter's is written from a button's callback rather than from
    /// here.
    fn frame(&mut self, size: Size, delta: Duration) {
        let _ = self.clock.tick(delta);
        let _ = self.theme.tick(delta);
        self.track_hover();
        self.sync_button_state();
        self.sync_slider_state();
        self.sync_toggle_state();
        for button in &self.buttons {
            let _ = button.widget.tick(delta);
        }
        let _ = self.slider.widget.tick(delta);
        // The toggle's own thumb slide, the bar's value and — the reason it is
        // worth a line of its own — the *loop*: `Progress::tick` aims the next
        // leg of the slide on the tick the last one arrives, so a bar in
        // indeterminate mode that has been aimed once keeps moving without the
        // demo coming back to it.
        let _ = self.toggle.tick(delta);
        let _ = self.progress.tick(delta);
        let _ = self.list.scroll().tick(delta);
        // The field's blink and the keyboard's key colours. The field's is the
        // one that matters: it is the only clock in the demo that is *always*
        // running while a control has focus, and `tick` returning true on a
        // phase flip is what makes the caret redraw.
        let _ = self.text_input.tick(delta);
        let _ = self.keyboard.tick(delta);

        let list_rect: Option<Rect> = {
            let mut nodes = self.nodes.borrow_mut();
            Layout::new(&mut nodes).layout(self.root, Constraints::tight(size));
            // The list's viewport, read off its own node. It is an owned value
            // because this borrow ends with the block and the row painting at the
            // end of the frame wants it too.
            let rect: Option<Rect> = nodes
                .get(self.list.handle())
                .and_then(|node| node.layout().rect())
                .map(Into::into);
            if let Some(rect) = rect {
                self.list.sync(&mut nodes, rect);
            }
            rect
        };

        // The three numbers the list's readout names, read after the sync and
        // not before: they are what the sync decided, and a readout written
        // before it would name last frame's rows.
        self.first_visible.set(
            self.list
                .visible_items()
                .first()
                .map_or(0, |(index, _)| *index),
        );
        self.live_count.set(self.list.visible_items().len());
        self.free_count.set(self.list.free_len());

        // Where it is in the order above that the frame rate goes: after the
        // clocks and the list's numbers, and **before** the arena is borrowed.
        self.tick_fps(delta);

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
            // when it has none: five of the demo's six draw no commands, and
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
            if let Some(button) = self.buttons.iter().find(|button| button.node() == handle) {
                let commands = match node.layout().rect() {
                    Some(rect) => button.widget.paint(
                        rect.into(),
                        &|ch: char| self.metrics.advance(ch, BUTTON_FONT),
                        self.metrics.line_height(BUTTON_FONT),
                    ),
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
            // The three widget nodes that paint themselves with nothing but a
            // rect. Each is the widget's **own** node, which is the whole of what
            // it takes to put one of them on the screen: `order` reaches it, the
            // commands go on it, and `draw` sends it.
            if handle == self.toggle.handle()
                || handle == self.image.handle()
                || handle == self.progress.handle()
                || handle == self.keyboard.handle()
            {
                let commands = match node.layout().rect() {
                    Some(rect) if handle == self.toggle.handle() => self.toggle.paint(rect.into()),
                    Some(rect) if handle == self.image.handle() => self.image.paint(rect.into()),
                    Some(rect) if handle == self.progress.handle() => {
                        self.progress.paint(rect.into())
                    }
                    Some(rect) => self.keyboard.paint(rect.into()),
                    None => Vec::new(),
                };
                *node.paint_mut() = PaintState::from_commands(commands);
                continue;
            }

            // The list's node is **not** painted here, and neither is the content
            // node its rows hang from. The rows are painted further down, in
            // their own coordinates, and `List::paint` reads those and translates
            // them; painting either node from this walk would draw every row a
            // second time, at the rect the layout pass gave it rather than at the
            // one its index implies. Both fall through to the pad arm and out of
            // it, which is where a node that is nobody's ends up.
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

        // The labels last, so the text is on top of the pads and the buttons:
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

        // The band's own labels, which are reached through the band rather than
        // as its siblings. They are painted after the widgets they report on, so
        // a number is on top of the control it is a number about — the slider's
        // readout over the slider has been the rule since task 14.
        for (readout, font) in [
            (&self.counter, BUTTON_FONT),
            (&self.slider_readout, BUTTON_FONT),
            (&self.toggle_readout, BUTTON_FONT),
            (&self.image_fit_readout, BUTTON_FONT),
            (&self.progress_readout, BUTTON_FONT),
            (&self.list_readout, LIST_FONT),
            (&self.fps_readout, BUTTON_FONT),
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

        // The list's own frame, last of all and outside the borrow above: its
        // rows have to be painted first, and painting them writes a row's text,
        // and a text write is a property write. The rows carry no `on_change`
        // link — see the note in `Demo::new` — so nothing here reaches back into
        // the arena, and one borrow is enough.
        if let Some(rect) = list_rect {
            self.paint_rows(&mut nodes, rect);
            let commands = self.list.paint(&nodes, rect);
            if let Some(node) = nodes.get_mut(self.list.handle()) {
                *node.paint_mut() = PaintState::from_commands(commands);
            }
        }
    }

    /// Writes each live row's text and paints it, in the row's **own**
    /// coordinates.
    ///
    /// A row's recorded commands are row-local — the origin is the row's own top
    /// left — because [`List::paint`] reads them and translates them by
    /// [`List::item_rect`]. A row is recycled, so a row that baked in a window
    /// position would be in the wrong place the moment it was attached for a
    /// different index, and the demo writes the index on instead.
    ///
    /// The background is the theme's `Surface`, so a row reads as a row and the
    /// list reads as a panel rather than as loose text on the window. It is drawn
    /// first, in the row's own box, so the text is on top of it.
    fn paint_rows(&self, nodes: &mut Arena<WidgetNode>, list_rect: Rect) {
        let background = self.row_background();
        let mut rows = self.rows.borrow_mut();
        for &(index, handle) in self.list.visible_items() {
            let Some(row) = rows.iter_mut().find(|row| row.node == handle) else {
                continue;
            };
            row.label.label.text.set(row_text(index));
            let area = Rect::new(0.0, 0.0, list_rect.width, LIST_ITEM_HEIGHT);
            let mut painter = Painter::new();
            painter.rect(area, background);
            let mut commands = painter.finish();
            let mut options = row.label.options;
            options.line_height = self.metrics.line_height(LIST_FONT);
            let text = Rect::new(
                ROW_PADDING,
                ROW_PADDING,
                area.width - ROW_PADDING * 2.0,
                area.height - ROW_PADDING * 2.0,
            );
            commands.extend(row.label.label.paint(text, &options, &|ch: char| {
                self.metrics.advance(ch, LIST_FONT)
            }));
            if let Some(node) = nodes.get_mut(handle) {
                *node.paint_mut() = PaintState::from_commands(commands);
            }
        }
    }

    /// Returns the colour a list row is painted behind its text: the theme's
    /// `Surface`, or black if the theme ever holds something else there.
    fn row_background(&self) -> Color {
        self.theme
            .get(ThemeToken::Surface)
            .as_color()
            .unwrap_or(Color::new(0, 0, 0, 255))
    }

    /// Writes each button's `hovered` flag from where the pointer is.
    ///
    /// Hover is the one state that is not an event: it is a fact about where
    /// the pointer is, so it is read from the recogniser's last known position
    /// rather than carried from a motion event, and a touch — which has no
    /// position to hover with once it is gone — leaves every button unhovered.
    fn track_hover(&mut self) {
        let pointer = self.recognizer.mouse_position();
        let hovered = match pointer {
            Some(position) => {
                let nodes = self.nodes.borrow();
                self.buttons.iter().position(|button| {
                    nodes
                        .get(button.node())
                        .and_then(|node| node.layout().rect())
                        .is_some_and(|rect| {
                            position.x >= rect.origin.x
                                && position.x <= rect.origin.x + rect.size.width
                                && position.y >= rect.origin.y
                                && position.y <= rect.origin.y + rect.size.height
                        })
                })
            }
            None => None,
        };
        for (index, button) in self.buttons.iter_mut().enumerate() {
            let wanted = Some(index) == hovered;
            if button.widget.hovered.get() != wanted {
                button.widget.hovered.set(wanted);
                button.written.hovered = wanted;
            }
        }
    }

    /// Writes each button's `pressed` flag, and re-aims the buttons whose state
    /// has moved.
    ///
    /// Aiming restarts a button's transition, so it happens only when the state
    /// the demo has written has actually changed. Aiming every frame would
    /// restart the transition on every frame, and the button would creep toward
    /// its target for ever instead of arriving at it.
    fn sync_button_state(&mut self) {
        let pressed = self.pressed;
        let motion = Motion::from_theme(&self.theme);
        for (index, button) in self.buttons.iter_mut().enumerate() {
            let wanted = Some(index) == pressed;
            if button.widget.pressed.get() != wanted {
                button.widget.pressed.set(wanted);
                button.written.pressed = wanted;
            }
            let state = button.state();
            if button.aimed != state {
                button.aimed = state;
                button.widget.animate_to_state(motion);
            }
        }
    }

    /// Writes the slider's `dragging` flag, and re-aims it when that has moved.
    ///
    /// The same two records as [`Demo::sync_button_state`] and the same reason:
    /// aiming restarts the slider's transition, so aiming every frame would leave
    /// the thumb creeping toward its target for ever. The slider's *value* is not
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
    /// The same two records as [`Demo::sync_button_state`] and the same reason:
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
        let clips = self.frame_clips(&nodes);
        for (handle, clip) in self.order.iter().copied().zip(clips) {
            renderer.draw_node_clipped(handle, &mut nodes, clip);
        }
    }

    /// Returns the clip for every node in paint order, positionally matching
    /// [`Demo::order`].
    ///
    /// **This is the whole of the frame's clipping, in one place, and the frame
    /// loop uses it rather than deciding inline.** That is not tidiness: an
    /// earlier version had the loop call `clip_for` itself, and a mutation that
    /// inlined the same logic into the loop instead sailed through every test,
    /// because the tests were calling `clip_for` and the defect was in a caller
    /// of it. A test that exercises a helper cannot see a call site that stopped
    /// using the helper. One function, used by the loop and by the tests, closes
    /// that.
    ///
    /// The rects come from the `nodes` the caller already borrows, rather than
    /// from `self`: [`Demo::draw`] holds the arena mutably for the whole loop,
    /// so reaching through `self` would be a `RefCell` double borrow, which
    /// panics on the first frame instead of failing a test.
    fn frame_clips(&self, nodes: &Arena<WidgetNode>) -> Vec<Option<Rect>> {
        let list = self.list.handle();
        self.order
            .iter()
            .map(|handle| {
                let rect = nodes
                    .get(*handle)
                    .and_then(|node| node.layout().rect())
                    .map(Into::into);
                Demo::clip_for(*handle, list, rect)
            })
            .collect()
    }

    /// Returns the rect the node at `handle` must be clipped to, or `None` for
    /// the whole window. `rect` is that node's own laid-out rect.
    ///
    /// A free function over its arguments rather than a method, because
    /// [`Demo::draw`] already holds the arena mutably and a method would have to
    /// borrow it again — the same `RefCell` double borrow that
    /// [`ui_core::input::route`] was documented around. One
    /// definition, callable from both the frame loop and a test.
    ///
    /// **The list is the only clipped node here**, and it is the reason this
    /// function exists. A scrolling viewport's rows are laid out in the content's
    /// own coordinates and drawn at `viewport.y + index * item_height - offset`,
    /// so the topmost and bottommost rows are drawn **half outside the viewport by
    /// design** — that is what makes a smooth scroll rather than a jumping one.
    /// With nothing to clip them they were drawn on top of whatever is behind the
    /// list: a row's text appeared over the window background above the panel, and
    /// then vanished. It read as the list's first row jumping about.
    ///
    /// `List::paint` cannot fix this on its own, and the reason is worth writing
    /// down: a [`ui_core::paint::DrawCommand::Text`] carries an `x`, a `y` and a string and **no
    /// width**, so nothing outside the text pipeline can tell how far a run
    /// reaches, and trimming one is not something that can be done from outside
    /// it. `scroll::clip_commands` can therefore only drop a command that is
    /// *wholly* outside — which is the limitation it documents — and the
    /// half-row needs the GPU. So it gets the GPU: a scissor, carried on the
    /// batch and set at submission by [`Renderer::draw_node_clipped`].
    fn clip_for(handle: Handle, list: Handle, rect: Option<Rect>) -> Option<Rect> {
        if handle == list {
            rect
        } else {
            None
        }
    }

    /// Switches between the dark and light themes, animated over
    /// `THEME_TRANSITION` milliseconds.
    ///
    /// The buttons are aimed at the *new* theme's palette rather than the one
    /// the theme is passing through, so each button's transition and the theme's
    /// own arrive together at the end of the same window. Aiming at the
    /// theme's current value would instead leave every button chasing a target
    /// that moves for as long as the switch does.
    fn toggle_theme(&mut self) {
        self.dark = !self.dark;
        let new_theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        // Read the new theme's palette and motion before it is handed to
        // `switch_to`, which takes it by value.
        let palette = Palette::from_theme(&new_theme);
        let slider_palette = SliderPalette::from_theme(&new_theme);
        let toggle_palette = TogglePalette::from_theme(&new_theme);
        let progress_palette = ProgressPalette::from_theme(&new_theme);
        let scroll_palette = ScrollPalette::from_theme(&new_theme);
        // Read from `new_theme`, **before** `switch_to` consumes it. The switch
        // animates the theme's own tokens, so a palette read after it is the
        // palette the theme is leaving, which re-aims every widget at what it
        // already had and the transition goes nowhere.
        let text_input_palette = TextInputPalette::from_theme(&new_theme);
        let keyboard_palette = KeyboardPalette::from_theme(&new_theme);
        let motion = Motion::from_theme(&new_theme);
        self.theme.switch_to(new_theme, THEME_TRANSITION);
        for button in &mut self.buttons {
            button.widget.set_palette(palette);
            button.widget.animate_to_state(motion);
        }
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
        // The list's scrollbar themes with everything else, and now animates with
        // it: `set_palette` is the door the widget could not otherwise be given,
        // and `animate_to_state` through `scroll()` is what carries the scrollbar
        // to the new palette over the theme's own transition.
        self.list.set_palette(scroll_palette);
        self.list.scroll().animate_to_state(motion);
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
    fn press_all(&mut self) {
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

    /// Releases every pad at once, on the release spring.
    fn release_all(&mut self) {
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

    /// Returns the index of the button whose laid-out rect contains `(x, y)`.
    ///
    /// A disabled button is still under the point: it is what the tap is aimed
    /// at, and it is the button that swallows it. Deciding that here would mean
    /// the tap reached whatever is behind instead.
    fn button_at(&self, x: f32, y: f32) -> Option<usize> {
        let nodes = self.nodes.borrow();
        self.buttons.iter().position(|button| {
            nodes
                .get(button.node())
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
    /// The slider is the last thing asked about, so a point over the band or a
    /// pad never reaches it: those are the controls that are drawn on top of
    /// that part of the window.
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

    /// Returns `Some(())` when the point is over the list, and `None` when it is
    /// not.
    ///
    /// A press here starts a drag the list is offered every movement of, whether
    /// or not the pointer is still over it — see [`Demo::list_dragging`].
    fn list_at(&self, x: f32, y: f32) -> Option<()> {
        self.list_rect()
            .is_some_and(|rect| over_rect(rect, x, y))
            .then_some(())
    }

    /// Returns the list's rect in window coordinates, or `None` if it has not
    /// been laid out. See [`Demo::slider_rect`].
    fn list_rect(&self) -> Option<Rect> {
        self.node_rect(self.list.handle())
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
    /// window, the text panel is 900 by 380 of nothing, and the list's content
    /// node is as tall as its rows. Two of those "overlap" everything, so a test
    /// over all of them would assert that everything overlaps everything and
    /// prove nothing.
    ///
    /// What a reader can actually see is a set of boxes with names, and this is
    /// that set: the card the pads sit in, the seven labels of the text panel,
    /// the three buttons, the counter, the slider and its readout, and then the
    /// eight things tasks 15 to 18 added. Two tests read it —
    /// `every_placed_rect_is_inside_the_window` and
    /// `no_two_placed_rects_overlap` — and the third defect this repository has
    /// found only by looking at the screen was a control placed over the thing
    /// next to it, so this is the pair of tests that would have found the first
    /// two.
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
        for index in 0..self.buttons.len() {
            if let Some(found) = rect("button", self.buttons[index].node()) {
                rects.push(found);
            }
        }
        for (what, handle) in [
            ("click counter", self.counter.label.handle()),
            ("slider", self.slider.node()),
            ("slider readout", self.slider_readout.label.handle()),
            ("image", self.image.handle()),
            ("image fit label", self.image_fit_readout.label.handle()),
            ("toggle", self.toggle.handle()),
            ("toggle readout", self.toggle_readout.label.handle()),
            ("progress bar", self.progress.handle()),
            ("progress readout", self.progress_readout.label.handle()),
            ("list", self.list.handle()),
            ("list readout", self.list_readout.label.handle()),
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

    /// Returns the centre of the button at `index` in window coordinates, or
    /// `None` if it has not been laid out.
    ///
    /// This is what a test aims a synthetic event at, so it is the demo's
    /// statement of where a button is rather than each test working it out.
    #[cfg(test)]
    fn button_center(&self, index: usize) -> Option<(f32, f32)> {
        let button = self.buttons.get(index)?;
        let nodes = self.nodes.borrow();
        let rect = nodes.get(button.node())?.layout().rect()?;
        Some((
            rect.origin.x + rect.size.width / 2.0,
            rect.origin.y + rect.size.height / 2.0,
        ))
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

    /// Returns the text the click counter is showing, as the last frame recorded
    /// it.
    #[cfg(test)]
    fn counter_text(&self) -> Option<String> {
        let nodes = self.nodes.borrow();
        nodes
            .get(self.counter.label.handle())?
            .paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let dark = first_label_color(&laid_out());
        let mut demo = laid_out();
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

    /// Clicks the button at `index` and lays the demo out, which is the whole
    /// path a click takes: two SDL events, the recogniser's tap, and the
    /// dispatch that routes it to the button under the pointer.
    fn click_button(demo: &mut Demo, index: usize) {
        let (x, y) = demo.button_center(index).expect("a laid-out button");
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));
    }

    /// The background the button at `index` was painted with on the last frame.
    fn button_background(demo: &Demo, index: usize) -> Color {
        let button = &demo.buttons[index];
        let nodes = demo.nodes.borrow();
        nodes
            .get(button.node())
            .expect("a button node")
            .paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::RoundedRect { color, .. } => Some(*color),
                _ => None,
            })
            .expect("a button paints a background")
    }

    /// The text the button at `index` painted on the last frame.
    fn button_text(demo: &Demo, index: usize) -> Option<String> {
        let button = &demo.buttons[index];
        let nodes = demo.nodes.borrow();
        nodes
            .get(button.node())
            .expect("a button node")
            .paint()
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
    }

    /// The rect the button at `index` was laid out to, as `(x, y, width, height)`.
    fn button_rect(demo: &Demo, index: usize) -> (f32, f32, f32, f32) {
        let button = &demo.buttons[index];
        let nodes = demo.nodes.borrow();
        let rect = nodes
            .get(button.node())
            .expect("a button node")
            .layout()
            .rect()
            .expect("a laid-out button");
        (
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        )
    }

    #[test]
    fn the_demo_has_a_band_of_three_buttons() {
        let demo = laid_out();
        assert_eq!(demo.buttons.len(), 3);
        let labels: Vec<String> = demo
            .buttons
            .iter()
            .map(|button| button.widget.label.get())
            .collect();
        assert_eq!(labels, vec!["Press me", "Disabled", "Reset"]);
    }

    #[test]
    fn a_button_paints_its_label_centred_inside_its_own_background() {
        // "Renders with label centred" is two things: there is a background and
        // there is a label, and the label sits in the middle of the background
        // rather than at its corner. Both are checked against the numbers the
        // widget derives them from: a 44-tall button with 4 of vertical padding
        // leaves 36 for a 24-tall line, so the line's top is 6 below the
        // padding's, at 10.
        let demo = laid_out();
        let button = &demo.buttons[0];
        let nodes = demo.nodes.borrow();
        let node = nodes.get(button.node()).expect("a button node");
        let commands = node.paint().commands();

        let background = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::RoundedRect { rect, .. } => Some(*rect),
                _ => None,
            })
            .expect("the button paints a background");
        let (text_x, text_y) = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { x, y, .. } => Some((*x, *y)),
                _ => None,
            })
            .expect("the button paints its label");
        assert_eq!(button_text(&demo, 0).as_deref(), Some("Press me"));
        assert_eq!(
            text_y,
            background.y + 10.0,
            "the label's line box is centred vertically: 4 of padding plus half \
             of the 12 the line does not fill"
        );
        assert!(
            text_x > background.x && text_x < background.x + background.width,
            "the label starts inside the background: {} against {}",
            text_x,
            background.x
        );
    }

    #[test]
    fn every_button_is_at_least_the_minimum_touch_target() {
        // 44 by 44 is the floor the widget applies, and the band proves it on
        // real labels: "Ok" is narrower than 44, so without the floor it would
        // be a target a finger cannot hit.
        let demo = laid_out();
        for (index, _) in demo.buttons.iter().enumerate() {
            let (_, _, width, height) = button_rect(&demo, index);
            assert!(
                width >= 44.0 && height >= 44.0,
                "button {index} is {width} by {height}"
            );
        }
    }

    #[test]
    fn a_short_label_is_still_floored_at_the_minimum_touch_target() {
        // The narrowest button in the band is "Reset", five characters, and it
        // is still floored. The label is 20 pixels at half-width per character,
        // so 50 plus 16 of padding would pass the floor on width alone; the
        // height is the one under it, being one line of 24 plus 8 of padding.
        let demo = laid_out();
        let (_, _, width, height) = button_rect(&demo, 2);
        assert_eq!(button_text(&demo, 2).as_deref(), Some("Reset"));
        assert_eq!(height, 44.0, "a one-line button is floored in height");
        assert!(width > 44.0, "and is wider than the floor on its own");
    }

    #[test]
    fn clicking_a_button_counts_a_click() {
        let mut demo = laid_out();
        assert_eq!(demo.counter_text().as_deref(), Some("0 clicks"));

        click_button(&mut demo, 0);

        assert_eq!(
            demo.counter_text().as_deref(),
            Some("1 click"),
            "the button's callback wrote to the counter, and the label followed"
        );
    }

    #[test]
    fn the_reset_button_empties_the_counter() {
        let mut demo = laid_out();
        click_button(&mut demo, 0);
        click_button(&mut demo, 0);
        assert_eq!(demo.counter_text().as_deref(), Some("2 clicks"));

        click_button(&mut demo, 2);

        assert_eq!(demo.counter_text().as_deref(), Some("0 clicks"));
    }

    #[test]
    fn a_disabled_button_swallows_a_tap_and_fires_nothing() {
        let mut demo = laid_out();
        let (x, y) = demo.button_center(1).expect("a laid-out button");
        let (down, up) = click_at(x, y);

        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.counter_text().as_deref(),
            Some("0 clicks"),
            "a disabled button does nothing"
        );
    }

    #[test]
    fn one_click_on_a_button_counts_once() {
        // This is what the demo can actually establish: one click, one count.
        //
        // It is **not** a test that a consumed tap stops travelling, and it was
        // previously named and commented as if it were — the claim was that the
        // counter would go up by two if the tap fell through to the panel
        // behind, and nothing behind the band handles a `Tap` at all, so the
        // counter could not have gone up by two whatever the routing did. The
        // `break` on `event.consumed()` here is defensive, mirroring
        // `dispatch_event`.
        //
        // The consumption contract is tested where it can fail, against a real
        // node behind the button:
        // `ui_core::widgets::button::tests::a_tap_fires_the_click_and_is_consumed`
        // fails when `event.consume()` is removed, and
        // `ui_core::input::tests::an_event_bubbles_to_the_parent_until_it_is_consumed`
        // covers the bubbling.
        let mut demo = laid_out();
        click_button(&mut demo, 0);
        assert_eq!(demo.counter_text().as_deref(), Some("1 click"));

        // And a second click is a second count, so the two are not being folded
        // into one by the routing.
        click_button(&mut demo, 0);
        assert_eq!(demo.counter_text().as_deref(), Some("2 clicks"));
    }

    #[test]
    fn a_pressed_button_moves_through_its_transition_and_arrives() {
        // "State transitions are animated" has two halves that can be broken
        // separately: the state change has to start a transition rather than
        // jumping, and that transition has to finish rather than creeping.
        let mut demo = laid_out();
        let (x, y) = demo.button_center(0).expect("a laid-out button");
        demo.handle_event(Event::MouseButtonDown {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mouse_btn: MouseButton::Left,
            clicks: 1,
            x,
            y,
        });
        demo.frame(WINDOW, Duration::from_millis(10));
        assert!(demo.buttons[0].widget.pressed.get());
        let midway = demo.buttons[0].widget.scale.get();
        assert!(
            midway < 1.0 && midway > 0.95,
            "ten milliseconds in, the button is part way to 0.95, not there: {midway}"
        );

        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.buttons[0].widget.scale.get(),
            0.95,
            "and it arrives rather than creeping"
        );
        assert!(
            !demo.buttons[0].widget.is_animating(),
            "a transition that has arrived has stopped"
        );
    }

    #[test]
    fn releasing_a_button_brings_its_scale_back() {
        let mut demo = laid_out();
        let (x, y) = demo.button_center(0).expect("a laid-out button");
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(demo.buttons[0].widget.scale.get(), 0.95);

        demo.handle_event(up);
        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.buttons[0].widget.scale.get(),
            1.0,
            "the release brings it back to its resting size"
        );
    }

    #[test]
    fn a_button_under_the_pointer_is_hovered() {
        let mut demo = laid_out();
        let (x, y) = demo.button_center(2).expect("a laid-out button");
        assert!(
            demo.buttons
                .iter()
                .all(|button| !button.widget.hovered.get()),
            "nothing is hovered before the pointer has been anywhere"
        );

        demo.handle_event(Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: sdl3::mouse::MouseState::from_sdl_state(0),
            x,
            y,
            xrel: 0.0,
            yrel: 0.0,
        });
        demo.frame(WINDOW, Duration::from_millis(16));

        assert!(
            demo.buttons[2].widget.hovered.get(),
            "it is the one under it"
        );
        assert!(
            demo.buttons[0..2]
                .iter()
                .all(|button| !button.widget.hovered.get()),
            "and not the others"
        );
    }

    #[test]
    fn a_hovered_button_paints_a_lighter_background_than_a_resting_one() {
        let mut demo = laid_out();
        let resting = button_background(&demo, 0);
        let (x, y) = demo.button_center(0).expect("a laid-out button");
        demo.handle_event(Event::MouseMotion {
            timestamp: 0,
            window_id: 0,
            which: 0,
            mousestate: sdl3::mouse::MouseState::from_sdl_state(0),
            x,
            y,
            xrel: 0.0,
            yrel: 0.0,
        });
        for _ in 0..20 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }

        let hovered = button_background(&demo, 0);
        assert_ne!(hovered, resting, "the hover has arrived");
        assert!(
            hovered.r > resting.r || hovered.g > resting.g || hovered.b > resting.b,
            "and it is lighter: {resting:?} then {hovered:?}"
        );
    }

    #[test]
    fn a_focused_button_paints_a_ring_the_rest_do_not() {
        // The focus indicator is a drawing, not a flag: the ring is the extra
        // rounded rect `Button::paint` puts outside the background.
        let mut demo = laid_out();
        let rings = |demo: &Demo, index: usize| {
            let button = &demo.buttons[index];
            let nodes = demo.nodes.borrow();
            nodes
                .get(button.node())
                .expect("a button node")
                .paint()
                .commands()
                .iter()
                .filter(|command| matches!(command, DrawCommand::RoundedRect { .. }))
                .count()
        };
        assert_eq!(rings(&demo, 0), 1, "a resting button is one background");

        demo.handle_event(key(Keycode::Tab));
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(rings(&demo, 0), 2, "a focused one has a ring as well");
        assert_eq!(rings(&demo, 1), 1, "and its neighbours do not");
    }

    #[test]
    fn tab_moves_focus_to_the_first_button_and_enter_activates_it() {
        let mut demo = laid_out();
        assert!(demo.focused.is_none(), "nothing holds focus to begin with");

        demo.handle_event(key(Keycode::Tab));
        assert_eq!(
            demo.focused,
            Some(demo.buttons[0].node()),
            "Tab lands on the first focusable button"
        );
        assert!(demo.buttons[0].widget.focused.get());

        demo.handle_event(key(Keycode::Return));
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.counter_text().as_deref(),
            Some("1 click"),
            "and Enter activates the button holding focus"
        );
    }

    #[test]
    fn tab_steps_over_the_disabled_button() {
        // A control that refuses interaction has nothing to be activated by a
        // key, so it is not in the order focus walks.
        //
        // The walk has **eight** stops: the two buttons that can be activated,
        // and then the six that follow the band in the tree's paint order — the
        // slider, the image, the toggle, the bar, the list and, since task 19,
        // the text field. The claim here is still that the disabled one is never
        // visited, and it is stated over the whole walk rather than over the
        // band's first three.
        //
        // The nine presses are the eight stops and a wrap, which is what says the
        // order is a cycle rather than a run that stops.
        let mut demo = laid_out();
        let enabled: Vec<Handle> = demo
            .buttons
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != 1)
            .map(|(_, button)| button.node())
            .collect();
        let slider = demo.slider.node();
        let image = demo.image.handle();
        let toggle = demo.toggle.handle();
        let progress = demo.progress.handle();
        let list = demo.list.handle();
        let text_input = demo.text_input.handle();

        let mut visited = Vec::new();
        for _ in 0..9 {
            demo.handle_event(key(Keycode::Tab));
            visited.push(demo.focused);
        }

        assert_eq!(
            visited,
            vec![
                Some(enabled[0]),
                Some(enabled[1]),
                Some(slider),
                Some(image),
                Some(toggle),
                Some(progress),
                Some(list),
                Some(text_input),
                Some(enabled[0])
            ],
            "the walk is press, reset, the slider, the image, the toggle, the \
             progress bar, the list, the text field, and wraps back to press — \
             never the disabled one in the middle"
        );
    }

    #[test]
    fn enter_does_nothing_while_no_button_holds_focus() {
        let mut demo = laid_out();
        demo.handle_event(key(Keycode::Return));
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.counter_text().as_deref(),
            Some("0 clicks"),
            "an activation key with nothing focused belongs to nobody"
        );
    }

    #[test]
    fn a_focused_button_can_be_activated_by_the_space_bar() {
        let mut demo = laid_out();
        demo.handle_event(key(Keycode::Tab));
        demo.handle_event(key(Keycode::Space));
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(demo.counter_text().as_deref(), Some("1 click"));
    }

    #[test]
    fn the_buttons_follow_the_theme_switch() {
        // The band is themed from the Primary and OnPrimary tokens, so a
        // switch carries the new colours to it: the button is aimed at the new
        // theme's palette, and its own transition runs alongside the theme's.
        let mut demo = laid_out();
        let dark = button_background(&demo, 0);
        assert_eq!(
            dark.r,
            Theme::dark().get(ThemeToken::Primary).as_color().unwrap().r,
            "a resting button paints the dark theme's Primary token"
        );

        demo.handle_event(key(Keycode::T));
        for _ in 0..35 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }

        let light = button_background(&demo, 0);
        assert_ne!(light, dark, "the button's colour moved with the theme");
        assert_eq!(
            light.r,
            Theme::light()
                .get(ThemeToken::Primary)
                .as_color()
                .unwrap()
                .r,
            "and arrived at the light theme's Primary token"
        );
    }

    #[test]
    fn a_button_in_the_band_does_not_move_the_pads() {
        // The pads and the buttons are separate controls: a click on the band
        // must not reach the row above it.
        let mut demo = laid_out();
        click_button(&mut demo, 0);

        for pad in &demo.pads {
            assert_eq!(pad.press.get(), 0.0, "no pad was pressed");
        }
    }

    #[test]
    fn the_band_sits_clear_of_the_text_panel_and_the_pads() {
        // The band is placed by hand, so its position is a claim about the
        // window that has to be checked: a `Stack` places all of its children at
        // the origin, so an offset on the band itself rather than on the row
        // inside it would be ignored, and the band would land on the pads.
        let demo = laid_out();
        let (band_x, band_y, _, band_height) = button_rect(&demo, 0);
        let column_right = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        assert!(
            band_x > column_right,
            "the band starts at x = {band_x}, right of the text column's edge \
             at {column_right}"
        );
        assert!(
            band_y > PAD_SIZE.height,
            "the band starts at y = {band_y}, below the pads"
        );
        assert!(
            band_y + band_height + COUNTER_DROP < WINDOW.height,
            "and the counter under it is still inside the window"
        );

        // The concrete claim: no pad shares a point with the band. The pads are
        // centred in the window, so this is the check that a band at the origin
        // would fail.
        let nodes = demo.nodes.borrow();
        for pad in &demo.pads {
            let rect = nodes
                .get(pad.node)
                .and_then(|node| node.layout().rect())
                .expect("a laid-out pad");
            let overlaps = band_x < rect.origin.x + rect.size.width
                && rect.origin.x < band_x + 400.0
                && band_y < rect.origin.y + rect.size.height
                && rect.origin.y < band_y + band_height;
            assert!(!overlaps, "the band overlaps a pad at {:?}", rect);
        }
    }

    #[test]
    fn no_text_label_reaches_under_the_band() {
        // The collision a screenshot showed: the alignment rows are laid out
        // across `TEXT_COLUMN_WIDTH`, so a right-aligned one ends at the column's
        // right edge. If that edge ever moves right of the band, the text runs
        // under the buttons — and nothing else in the suite would notice, since
        // both the label and the button are laid out correctly on their own.
        let demo = laid_out();
        let (band_x, _, _, _) = button_rect(&demo, 0);
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
                right <= band_x,
                "a label ends at {right}, which is under the band at {band_x}"
            );
        }
    }

    #[test]
    fn the_row_of_buttons_does_not_overlap_the_next() {
        let demo = laid_out();
        let rects: Vec<(f32, f32, f32)> = (0..demo.buttons.len())
            .map(|index| {
                let (x, _, width, _) = button_rect(&demo, index);
                (x, x + width, width)
            })
            .collect();
        for pair in rects.windows(2) {
            assert!(
                pair[0].1 <= pair[1].0,
                "button {} ends at {} and the next starts at {}",
                pair[0].2,
                pair[0].1,
                pair[1].0
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

    #[test]
    fn every_parent_the_demo_assembles_is_a_container_widget() {
        // The demo used to assemble its own parent nodes, which meant two
        // implementations of the same composition primitive in one repository:
        // the demo's private helper and the widget. Nothing in the suite would
        // have noticed a new one appearing, so this is the check that a parent
        // the *demo* built is a `Container` and not a node the demo wired up.
        //
        // Since task 18 there are two parents in the tree that the demo did not
        // build and cannot: the node a `List` scrolls in, and the content node it
        // hangs its rows from. Both belong to the `Scroll` inside the list, the
        // demo holds no handle that is not already owned by the widget, and
        // wrapping either in a `Container` would put a node between the scroll
        // and the rows the scroll is positioning — or between the list and the
        // scroll itself. They are named here rather than papered over, because
        // the names are the whole of the exception: **the two parents that are
        // not the demo's are the two the widget owns**, and a third would fail
        // this test.
        //
        // The demo is **laid out** first, not merely built, because the content
        // node only becomes a parent once `List::sync` has put a row in it: an
        // unbuilt list holds ten rows nowhere, and a test run on it would pass
        // with one exception while the real frame has two.
        let demo = laid_out();
        let nodes = demo.nodes.borrow();
        let containers: Vec<Handle> = demo.containers.iter().map(Container::handle).collect();
        let widget_owned = [demo.list.handle(), demo.list.content()];
        let mut parents = 0;
        for &handle in &demo.order {
            let node = nodes.get(handle).expect("a node in the demo's tree");
            if node.children().is_empty() {
                continue;
            }
            parents += 1;
            assert!(
                containers.contains(&handle) || widget_owned.contains(&handle),
                "node {handle:?} has children but is neither a Container nor one \
                 of the two nodes the list's own scroll owns"
            );
        }
        assert_eq!(
            parents,
            containers.len() + 2,
            "and every one of them is one of those eight"
        );
        assert_eq!(
            containers.len(),
            6,
            "the demo still assembles six: the card, the text \
             column and its panel, the button row and its band, and the root"
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
        for _ in 0..3 {
            demo.handle_event(key(Keycode::Tab));
        }
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
        let mut demo = laid_out();
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
        // so a focused slider is driven by it and a focused button still walks.
        let mut demo = laid_out();
        for _ in 0..3 {
            demo.handle_event(key(Keycode::Tab));
        }
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
        let mut demo = laid_out();
        let rects = |demo: &Demo| {
            demo.slider_commands()
                .iter()
                .filter(|command| matches!(command, DrawCommand::RoundedRect { .. }))
                .count()
        };
        assert_eq!(rects(&demo), 2, "an unfocused slider is a track and a fill");

        for _ in 0..3 {
            demo.handle_event(key(Keycode::Tab));
        }
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(rects(&demo), 3, "and a focused one has a ring as well");
    }

    #[test]
    fn the_slider_follows_the_theme_switch() {
        // The track is the theme's `Border` token and the fill its `Primary`, so
        // a switch carries the new colours to the slider through the property
        // graph: the widget is aimed at the new palette and its own transition
        // runs alongside the theme's.
        let mut demo = laid_out();
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
        let counter_top = BUTTON_ORIGIN.1 + COUNTER_DROP;
        assert!(
            rect.y > counter_top + 24.0,
            "and at y = {}, below the click counter's own line at {counter_top}",
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
    fn toggle_center(demo: &Demo) -> Option<(f32, f32)> {
        let rect = demo.toggle_rect()?;
        Some((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))
    }

    /// Returns a point inside the list's own rect, `rows` rows down and `across`
    /// pixels from its left edge.
    ///
    /// `across` is default 20 because a tap near the list's right edge would land
    /// on its scrollbar, which is a `Scroll`'s own geometry rather than the row
    /// under the finger.
    fn list_point(demo: &Demo, rows: f32, across: f32) -> Option<(f32, f32)> {
        let rect = demo.list_rect()?;
        Some((rect.x + across, rect.y + rows))
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

    #[test]
    fn the_demo_has_the_four_widgets_the_later_tasks_added() {
        // The four are in the band, in the order they were added, and each is
        // wired to something the demo can show: the toggle to a label that names
        // its state, the progress bar to a label that names its value, the list to
        // a label that names what it is holding, and the image to a label that
        // names its fit.
        let demo = laid_out();
        for (what, handle) in [
            ("toggle", demo.toggle.handle()),
            ("image", demo.image.handle()),
            ("progress bar", demo.progress.handle()),
            ("list", demo.list.handle()),
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
        demo.handle_event(key(Keycode::Return));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(
            !demo.toggle.checked.get(),
            "an unfocused toggle ignores the key, or nothing would ever focus it"
        );

        // The fifth stop: the two enabled buttons, the slider, the image, and
        // then the toggle — the band's paint order, which is the `Tab` order.
        for _ in 0..5 {
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
    fn the_list_holds_a_hundred_rows_and_builds_only_the_ones_it_can_show() {
        // "Virtualization: only visible items are allocated", on screen and not
        // only in the widget's own tests: a hundred rows of 28 in a 280-tall
        // viewport is ten on screen, and the other ninety are not in the tree.
        let demo = laid_out();
        assert_eq!(demo.list.item_count(), LIST_ITEM_COUNT);
        assert_eq!(
            demo.list.content_height(),
            LIST_ITEM_COUNT as f32 * LIST_ITEM_HEIGHT
        );
        let live = demo.list.visible_items();
        let expected = (LIST_SIZE.height / LIST_ITEM_HEIGHT) as usize;
        assert_eq!(live.len(), expected, "ten rows in a ten-row viewport");
        assert!(
            live.len() < demo.list.item_count(),
            "and a tenth of the hundred, so the other {} were never built",
            demo.list.item_count() - live.len()
        );
        assert_eq!(
            live.iter().map(|(index, _)| *index).collect::<Vec<usize>>(),
            (0..expected).collect::<Vec<usize>>(),
            "which are the first ten, at the top of the list"
        );
    }

    #[test]
    fn the_list_says_which_rows_it_is_holding_and_what_is_on_the_free_list() {
        // "so the *virtualisation* is visible on screen, not just in a test": the
        // readout is the only place a reader can see that ninety rows are not
        // there, and the three numbers in it are all the widget's own.
        let mut demo = laid_out();
        assert_eq!(
            demo.readout_text_of(&demo.list_readout).as_deref(),
            Some("first 0, live 10, free 0, tap -"),
            "ten rows on screen out of a hundred, and no tap yet"
        );

        // A drag **down** by exactly three rows: 3 of 28 is 84, and a whole
        // number of rows is what keeps the count at ten rather than eleven — a
        // row half off the top is in the tree too, and `visible_range` is a
        // range, not a count of whole rows. Down is the direction that advances,
        // since the convention changed on 2026-09-30.
        let (x, y) = list_point(&demo, 140.0, 20.0).expect("a laid-out list");
        for event in drag_on(x, y, x, y + 84.0) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }

        let said = demo
            .readout_text_of(&demo.list_readout)
            .expect("the list's readout");
        assert_eq!(
            said, "first 3, live 10, free 0, tap -",
            "three rows down, the same ten in the tree, and the free list empty \
             because every row that left was reused"
        );
        assert!(
            demo.list
                .visible_items()
                .iter()
                .all(|(index, _)| *index >= 3),
            "with nothing below the offset left in the tree"
        );
    }

    #[test]
    fn a_tap_on_the_list_reports_the_row_it_hit() {
        let mut demo = laid_out();
        // 100 down a viewport whose rows are 28 tall is the fourth row: three
        // whole rows are above it.
        let (x, y) = list_point(&demo, 100.0, 20.0).expect("a laid-out list");
        let (down, up) = click_at(x, y);
        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));

        assert_eq!(
            demo.readout_text_of(&demo.list_readout).as_deref(),
            Some("first 0, live 10, free 0, tap 3"),
            "a tap a hundred pixels down is the fourth row of 28"
        );
    }

    #[test]
    fn the_wheel_scrolls_the_list() {
        // "Scrolling works via mouse wheel": a wheel event carries the pointer's
        // position, so it is routed as a positional event and lands on the node
        // under the cursor rather than on the focused control. A list that only
        // answered the *focused* one would not move here at all.
        let mut demo = laid_out();
        let (x, y) = list_point(&demo, 140.0, 20.0).expect("a laid-out list");
        assert!(
            demo.focused.is_none(),
            "nothing holds focus, so this is not a key"
        );

        let before = demo.list.scroll().scroll_offset.get();
        // `y = -1.0` is SDL's "the wheel rolled towards the user", and under the
        // scrollbar convention the list adopted that is the notch which advances
        // *down* the document. It used to be `+1.0`; the operator inverted it on
        // 2026-09-30 because a reader's first instinct is to roll the wheel down.
        demo.handle_event(Event::MouseWheel {
            timestamp: 0,
            window_id: 0,
            which: 0,
            x,
            y: -1.0,
            direction: sdl3::mouse::MouseWheelDirection::Normal,
            mouse_x: x,
            mouse_y: y,
            integer_x: 0,
            integer_y: -1,
        });
        demo.frame(WINDOW, Duration::from_millis(16));

        assert!(
            demo.list.scroll().scroll_offset.get() > before,
            "one notch down moved the list from {before} to {}",
            demo.list.scroll().scroll_offset.get()
        );
    }

    #[test]
    fn a_drag_past_the_end_of_the_list_still_scrollles_it() {
        // The finger that has left the viewport is the case that matters, and it
        // is the reason the demo has a `list_dragging` flag at all: a `Scroll`
        // has no grabbed state, so a drag offered only to whatever is under the
        // pointer would stop the moment the pointer left.
        let mut demo = laid_out();
        let rect = demo.list_rect().expect("a laid-out list");
        let bottom = (rect.y + rect.height - 4.0).min(rect.y + 100.0);
        let (x, y) = list_point(&demo, bottom - rect.y, 20.0).expect("a laid-out list");
        // **Downwards**, which is the direction that advances — the convention is
        // *down is later* since 2026-09-30, so the finger leaves the list below
        // rather than above it.
        for event in drag_on(x, y, x, y + 400.0) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        assert!(
            demo.list.scroll().scroll_offset.get() > 0.0,
            "a drag that ended far below the list still scrolled it to {}",
            demo.list.scroll().scroll_offset.get()
        );
    }

    /// The demo's scrollbar is drawn at the width the operator asked for, and
    /// the setter is what put it there.
    ///
    /// *"is too narrow, I have issues with pointing on it with my mouse, so doing
    /// that on tablet with a finger is impossible"* (2026-10-01). The widget's
    /// own default is still 6, so this test is the only thing standing between
    /// that report and a repeat of it: it asserts the number that reached the
    /// list's own draw commands, which is where the complaint was about.
    #[test]
    fn the_demos_scrollbar_is_twelve_wide_and_finger_sized() {
        let demo = laid_out();
        let rect = demo.list_rect().expect("a laid-out list");
        let bar = demo
            .list
            .scroll()
            .scrollbar_rect(rect)
            .expect("a hundred rows in a 280 viewport scrolls, so a bar is drawn");

        assert_eq!(
            bar.width, SCROLLBAR_THICKNESS,
            "the width the operator asked for, and the widget's default is 6"
        );
        // 2 is the widget's own `SCROLLBAR_MARGIN`, which is private to
        // `ui_core`; what matters here is that widening the bar did not push it
        // out of the list.
        assert_eq!(
            bar.x + bar.width,
            rect.x + rect.width - 2.0,
            "still inset from the list's right edge by the widget's own margin"
        );

        // And the drawn groove agrees, because a hit target wider than the bar
        // that is drawn is a target nobody can see.
        let groove = demo
            .commands_at(demo.list.handle())
            .into_iter()
            .find_map(|command| match command {
                DrawCommand::RoundedRect { rect, .. } => Some(rect),
                _ => None,
            })
            .expect("the scrollbar's groove is drawn on the list's node");
        assert_eq!(groove, bar);
    }

    /// Dragging the scrollbar's thumb moves the **thumb**, at the cursor's speed.
    ///
    /// This is the operator's second complaint end to end — *"when I click it and
    /// drag, it doesn't follow my mouse cursor exactly, it's like something was
    /// keeping it from moving faster"* — measured through the real event path
    /// (an SDL finger down, a motion, and the recogniser's own `Drag`).
    ///
    /// The arithmetic is the demo's own geometry: a 280-tall viewport over 2 800
    /// of rows gives a thumb of 28 travelling a run of 252, against a maximum
    /// offset of 2 520. So a 40-pixel drag of the thumb is **400** pixels of
    /// offset, and the content-drag path this replaced would have answered **40**.
    /// The two differ by ten, which is the size of the bug.
    #[test]
    fn a_drag_on_the_scrollbars_thumb_moves_the_thumb_and_not_the_drags_delta() {
        let mut demo = laid_out();
        let rect = demo.list_rect().expect("a laid-out list");
        // The thumb at offset zero: 280 * 280 / 2 800 is 28 of the 280.
        let thumb = Rect::new(rect.x + rect.width - 14.0, rect.y, 12.0, 28.0);
        let (x, y) = (thumb.x + thumb.width / 2.0, thumb.y + thumb.height / 2.0);

        assert_eq!(
            demo.list.scroll().scroll_offset.get(),
            0.0,
            "the list opens at the top"
        );

        for event in drag_on(x, y, x, y + 40.0) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }

        let after = demo.list.scroll().scroll_offset.get();
        assert!(
            (after - 400.0).abs() < 0.5,
            "40 pixels along a run of 252 is a fraction of 0.1587, and that \
             fraction of 2 520 is 400 — the list moved to {after}, not the 40 the \
             drag's own delta would have given"
        );
    }

    /// A drag that starts on a **row** still scrolls the content, at the
    /// finger's own speed, which is the behaviour the thumb must not have taken.
    #[test]
    fn a_drag_on_a_row_still_scrolls_the_content_by_the_drags_own_delta() {
        let mut demo = laid_out();
        let rect = demo.list_rect().expect("a laid-out list");
        // Well left of the scrollbar, and inside row 1.
        let (x, y) = (rect.x + 20.0, rect.y + 40.0);

        for event in drag_on(x, y, x, y + 40.0) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }

        assert_eq!(
            demo.list.scroll().scroll_offset.get(),
            40.0,
            "a drag on a row is a scroll of the content, one for one"
        );
    }

    /// A press on the scrollbar no longer selects the row behind it.
    ///
    /// The same complaint, second half: the strip is 12 pixels of a 270-wide
    /// list, and before this it named whichever row was under it.
    #[test]
    fn a_click_on_the_scrollbar_selects_no_row() {
        let mut demo = laid_out();
        let rect = demo.list_rect().expect("a laid-out list");
        let bar = demo
            .list
            .scroll()
            .scrollbar_rect(rect)
            .expect("a bar is drawn");

        // The scrollbar first, because the readout is a **sticky** property: it holds
        // the last tap and is never cleared, so a second click cannot be told
        // from the first by reading it. Clicking the bar first is what makes
        // "nothing happened" observable at all.
        let (down, up) = click_at(bar.x + bar.width / 2.0, rect.y + 40.0);
        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));
        let readout = demo.readout_text_of(&demo.list_readout).expect("a readout");
        assert!(
            readout.contains("tap -"),
            "a click on the scrollbar selects nothing, because a scrollbar is not \
             a row — the readout says {readout:?}"
        );

        // And a row, for the control: the same gesture a few pixels to the left
        // does select one, so the assertion above is about the bar and not about
        // clicks being off in this build.
        let (down, up) = click_at(rect.x + 20.0, rect.y + 40.0);
        demo.handle_event(down);
        demo.handle_event(up);
        demo.frame(WINDOW, Duration::from_millis(16));
        let readout = demo.readout_text_of(&demo.list_readout).expect("a readout");
        assert!(
            readout.contains("tap 1"),
            "and a click 20 pixels to the left selects the second row, so the \
             gesture and the routing both work — the readout says {readout:?}"
        );
    }

    #[test]
    fn an_arrow_key_scrolls_the_list_once_it_holds_focus() {
        let mut demo = laid_out();
        for _ in 0..7 {
            demo.handle_event(key(Keycode::Tab));
        }
        assert_eq!(
            demo.focused,
            Some(demo.list.handle()),
            "Tab reached the list"
        );

        let before = demo.list.scroll().scroll_offset.get();
        demo.handle_event(key(Keycode::Down));
        demo.frame(WINDOW, Duration::from_millis(16));
        assert!(
            demo.list.scroll().scroll_offset.get() > before,
            "one press of the down arrow scrolled it from {before} to {}",
            demo.list.scroll().scroll_offset.get()
        );
    }

    #[test]
    fn the_list_draws_its_rows_where_their_indices_say_and_not_a_second_time() {
        // The hazard `List::paint` exists to be handled: a row's commands are
        // **row-local** and are translated by the widget, so a row whose commands
        // were also recorded on its own node at the rect the layout pass gave it
        // would be drawn twice — once at the origin and once in place. The
        // check is both halves: the rows are **not** in the paint order, and the
        // text on the list's own node is at the row's own place.
        let demo = laid_out();
        for (_, handle) in demo.list.visible_items() {
            assert!(
                !demo.order.contains(handle),
                "a row at {handle:?} is in the paint order, so it is drawn twice"
            );
        }
        let rect = demo.list_rect().expect("a laid-out list");
        let text_x: Vec<f32> = demo
            .commands_at(demo.list.handle())
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        assert_eq!(
            text_x.len(),
            demo.list.visible_items().len(),
            "one text run per row on the list's node, and no more"
        );
        assert!(
            text_x
                .iter()
                .all(|x| *x > rect.x && *x < rect.x + rect.width),
            "and every one of them is inside the list's own rect: {text_x:?}"
        );
        // Row 0's own rect starts at the list's top left, so the first row's text
        // is the list's left edge plus the row's padding — the number that says
        // it was translated rather than drawn where the row node happened to be.
        assert_eq!(
            text_x[0],
            rect.x + ROW_PADDING,
            "row zero, inset by the padding"
        );
    }

    #[test]
    fn the_list_labels_each_row_with_its_own_index() {
        // The factory is given no index, because a row is recycled; the demo
        // writes the index on. So the *text* of a row is the demo's and it has to
        // be there, on every row, in ascending order.
        let demo = laid_out();
        let nodes = demo.nodes.borrow();
        let rows = demo.rows.borrow();
        let mut said = Vec::new();
        for (index, handle) in demo.list.visible_items() {
            assert!(
                rows.iter().any(|row| row.node == *handle),
                "a row at {handle:?} is one the factory built"
            );
            said.push((
                *index,
                nodes
                    .get(*handle)
                    .map(|node| node.paint().commands().len())
                    .unwrap_or_default(),
            ));
        }
        assert_eq!(
            said.iter().map(|(index, _)| *index).collect::<Vec<usize>>(),
            (0..said.len()).collect::<Vec<usize>>(),
            "the rows on screen are the first ten, in order"
        );
        for (index, commands) in said {
            assert!(
                commands >= 2,
                "row {index} recorded a background and a text run, and recorded \
                 {commands}"
            );
        }
        drop(nodes);
        // The two commands are the row's, and they are **row-local**: the
        // background is the row's own box from its own top left, and the text is
        // that box inset by the padding. `List::paint` is what puts them on the
        // screen, by translating them by the row's index — a background already
        // at a window position would be translated a second time and land
        // nowhere near its row, which no assertion about *how many* commands a
        // row holds could see.
        for &(index, handle) in demo.list.visible_items() {
            let commands = demo.commands_at(handle);
            let background = commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Rect { rect, .. } => Some(*rect),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("row {index} painted no background"));
            assert_eq!(
                (background.x, background.y),
                (0.0, 0.0),
                "row {index}'s background is its own box from its own top left"
            );
            assert_eq!(
                (background.width, background.height),
                (LIST_SIZE.width, LIST_ITEM_HEIGHT),
                "and it is the list's width by one row's height"
            );
        }
    }

    #[test]
    fn the_list_recycles_a_row_rather_than_building_a_second_one() {
        // "Items recycle when scrolling", and the number that says it is the
        // node count: a list that allocated a node per row on the way past would
        // have grown by one for every row it scrolled over.
        let mut demo = laid_out();
        let before = demo.nodes.borrow().len();
        let (x, y) = list_point(&demo, 140.0, 20.0).expect("a laid-out list");
        // Downwards, since the convention is *down is later*.
        for event in drag_on(x, y, x, y + 900.0) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        let after = demo.nodes.borrow().len();
        // At most **one** more node than the ten that fit: a row half off the
        // top and a row half off the bottom are both in the tree, so eleven can
        // be live. Thirty-odd rows were scrolled past and the arena grew by at
        // most one, which is the whole of "items recycle when scrolling" — a
        // list that allocated a node per row would have grown by thirty.
        assert!(
            after <= before + 1 && after > before,
            "scrolled past {} rows and the arena went from {before} to {after}",
            (900.0 / LIST_ITEM_HEIGHT) as usize
        );
        assert!(
            demo.list.free_len() <= demo.list.visible_items().len() + 1,
            "with a free list no longer than the rows on screen: {} free against \
             {} live",
            demo.list.free_len(),
            demo.list.visible_items().len()
        );
    }

    #[test]
    fn a_scrolled_list_clamps_at_its_last_row() {
        // "Scroll offset is clamped to [0, max_scroll]": a drag of the length of
        // the whole content must stop at the end rather than run into ninety rows
        // of nothing.
        let mut demo = laid_out();
        let (x, y) = list_point(&demo, 140.0, 20.0).expect("a laid-out list");
        // Downwards, since the convention is *down is later*.
        for event in drag_on(x, y, x, y + 20000.0) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        let rect = demo.list_rect().expect("a laid-out list");
        assert_eq!(
            demo.list.scroll().scroll_offset.get(),
            demo.list.max_scroll_for(rect),
            "the whole content is 100 rows of 28 and the viewport is 280 of them"
        );
        assert_eq!(
            demo.list.visible_items().last().map(|(index, _)| *index),
            Some(LIST_ITEM_COUNT - 1),
            "and the last row on screen is the last row there is"
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
        // The same claim, one at a time, in the words the previous three tasks
        // used: the toggle is below the slider's readout, the image is right of
        // the card of pads, and the list is right of the click counter. The pair
        // of tests above says no two boxes touch; this one says which boxes the
        // new ones are *not* allowed to be near, so a failure names the
        // neighbour rather than reporting forty pairs.
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

        let counter = at(demo.counter.label.handle());
        let list = at(demo.list.handle());
        assert!(
            list.x > counter.x + counter.width,
            "the list at {list:?} is right of the click counter at {counter:?}"
        );

        let column_right = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        for (what, handle) in [
            ("image", demo.image.handle()),
            ("list", demo.list.handle()),
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

    fn wheel_notch(dy: f32, x: f32, y: f32) -> Event {
        Event::MouseWheel {
            timestamp: 0,
            window_id: 0,
            which: 0,
            x: 0.0,
            y: dy,
            direction: sdl3::mouse::MouseWheelDirection::Normal,
            mouse_x: x,
            mouse_y: y,
            integer_x: 0,
            integer_y: if dy > 0.0 { 1 } else { -1 },
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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

    /// The wheel's two directions at the top of the list, **after the operator
    /// inverted the sign on 2026-09-30.**
    ///
    /// The list uses the **scrollbar convention**: SDL reports the wheel rolling
    /// *towards* the user as a negative `y`, and that is the notch that advances
    /// *down* the document. So the direction a reader tries first now works from
    /// the top, and the direction that does nothing is the other one — which is
    /// the whole reason the change was made.
    ///
    /// SDL already normalises that sign, and must not be inverted again: it
    /// reports `MouseWheelDirection::Flipped` to say the *device* runs the other
    /// way, and `GestureRecognizer` drops that field on purpose.
    #[test]
    fn the_wheel_scrolls_towards_the_user_down_the_list_and_no_further() {
        let mut demo = laid_out();
        let rect = demo.list_rect().expect("the list is placed");

        // One notch *towards* the user — SDL's negative y — goes down the list.
        demo.handle_event(wheel_notch(-1.0, rect.x + 40.0, rect.y + 40.0));
        assert_eq!(
            demo.list.scroll().scroll_offset.get(),
            48.0,
            "the direction a reader tries first moves the list"
        );

        // And one notch *away* brings it straight back to the top.
        demo.handle_event(wheel_notch(1.0, rect.x + 40.0, rect.y + 40.0));
        assert_eq!(demo.list.scroll().scroll_offset.get(), 0.0);
    }

    /// The other half of the above, and the case that makes the asymmetry visible
    /// rather than theoretical: from offset 0 there is nothing **above**, so the
    /// notch that would go back up the list is clamped and the list does not move.
    #[test]
    fn a_wheel_notch_away_from_the_user_at_the_top_of_the_list_does_nothing() {
        let mut demo = laid_out();
        let rect = demo.list_rect().expect("the list is placed");
        demo.handle_event(wheel_notch(1.0, rect.x + 40.0, rect.y + 40.0));
        assert_eq!(
            demo.list.scroll().scroll_offset.get(),
            0.0,
            "already at the top, so there is nothing above to reveal"
        );
        assert_eq!(
            demo.list.visible_range(rect).start,
            0,
            "and the top row is still the first"
        );
    }

    #[test]
    fn the_lists_scrollbar_animates_to_the_new_theme_rather_than_jumping() {
        let mut demo = laid_out();
        // 158 is the dark theme's `TextMuted` and 117 the light theme's, which
        // is the whole of the distance there is to travel: 41 a channel.
        assert_eq!(
            demo.list.scroll().thumb.get(),
            Color::new(158, 158, 158, 255)
        );

        demo.handle_event(toggle_theme_event());

        demo.frame(WINDOW, Duration::from_millis(10));
        assert_eq!(
            demo.list.scroll().thumb.get(),
            Color::new(158, 158, 158, 255),
            "the first frame of an EaseInOut over 150 ms rounds back to where it \
             started, because the channels have not moved yet"
        );
        assert!(
            demo.list.scroll().is_animating(),
            "but the scrollbar is mid-transition, which a direct property write \
             would never report"
        );

        for _ in 0..4 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.list.scroll().thumb.get(),
            Color::new(149, 149, 149, 255),
            "50 ms in it is part way across, not at either end"
        );

        for _ in 0..40 {
            demo.frame(WINDOW, Duration::from_millis(10));
        }
        assert_eq!(
            demo.list.scroll().thumb.get(),
            Color::new(117, 117, 117, 255),
            "and it finishes on the light theme's own colour"
        );
    }

    /// The regression for the defect the operator reported: a row scrolled half
    /// out of the list was **drawn over the window background above it**, because
    /// nothing clipped the list to its own viewport.
    ///
    /// The test cannot see the scissor — that is GPU state — so it pins the two
    /// halves of it that it can. First, that the demo hands the renderer a clip
    /// for the list and for nothing else. Second, that the geometry really does
    /// put a row outside the viewport, because that is what makes the clip
    /// load-bearing: without it this test would pass on a list that needed no
    /// clipping at all.
    #[test]
    fn the_list_is_the_only_node_clipped_and_it_does_have_rows_outside_itself() {
        let mut demo = laid_out();
        let rect = demo.list_rect().expect("the list is placed");
        // One wheel notch is 48 px, which over 28 px rows leaves the top row
        // 20 px off the top of the viewport. Negative is SDL's "towards the
        // user", and it is the direction that goes down the list.
        demo.handle_event(Event::MouseWheel {
            timestamp: 0,
            window_id: 0,
            which: 0,
            x: 0.0,
            y: -1.0,
            direction: sdl3::mouse::MouseWheelDirection::Normal,
            mouse_x: rect.x + 40.0,
            mouse_y: rect.y + 40.0,
            integer_x: 0,
            integer_y: -1,
        });
        demo.frame(WINDOW, Duration::from_millis(16));

        let drawn_above = demo
            .commands_at(demo.list.handle())
            .iter()
            .filter(|c| match c {
                DrawCommand::Text { y, .. } => *y < rect.y,
                _ => false,
            })
            .count();
        assert_eq!(
            drawn_above, 1,
            "half a row is meant to be drawn past the top edge — a smooth scroll \
             is a row leaving, not a row popping — so this is the case a clip has \
             to exist for"
        );
        // The frame's own list of clips, which is the one `Demo::draw` walks —
        // not `clip_for` called directly, because a test of a helper cannot see a
        // call site that stopped using it. That was a real survivor: a mutation
        // inlining the logic into the loop passed every test written against the
        // helper.
        let nodes = demo.nodes.borrow();
        let clips = demo.frame_clips(&nodes);
        drop(nodes);
        assert_eq!(
            clips.len(),
            demo.order.len(),
            "one clip per node in paint order, positionally matching"
        );
        let list_index = demo
            .order
            .iter()
            .position(|handle| *handle == demo.list.handle())
            .expect("the list is in the paint order");
        assert_eq!(
            clips[list_index],
            Some(rect),
            "and the list's is its own viewport, so the GPU cuts the half-row at \
             the edge instead of letting it draw over the window"
        );

        // Every other node is offered its **own real rect**, not `None`. Passing
        // `None` and comparing against `None` is an assertion that cannot fail:
        // it was written that way first, and a mutation that clipped *everything*
        // sailed through it. A mutation run is what found that out.
        let clipped_others: Vec<Handle> = demo
            .order
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != list_index && clips[*index].is_some())
            .map(|(_, handle)| *handle)
            .collect();
        assert!(
            clipped_others.is_empty(),
            "and no node but the list is clipped — but these are: {clipped_others:?}"
        );
    }

    /// The bottom edge, which the test above cannot reach: the operator's wheel
    /// moves the offset *down*, so the defect above was seen against the top edge
    /// and the bottom one is asserted here from the other direction.
    ///
    /// It asserts the **row's rect**, not a drawn text run, and the reason is
    /// worth stating because it cost a test: a [`DrawCommand::Text`] carries the
    /// top of its line box and not the row's lower edge, so a row straddling the
    /// bottom has its *text* inside the viewport and its *band* outside it.
    /// Asking about the text asks the wrong question. The band is what the scissor
    /// cuts.

    #[test]
    fn the_four_new_widgets_follow_the_theme_switch() {
        // `T` carries a new theme to every widget here, and the way it does is
        // each widget's own: the three that take a palette are aimed at the new
        // one and animated. The image is not in this list and must not be: it has
        // no palette, and a picture of a test card is the same picture in either
        // theme.
        let mut demo = laid_out();
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
        assert_eq!(
            demo.list.scroll().thumb.get(),
            ScrollPalette::from_theme(&Theme::dark()).thumb,
            "and so does the scrollbar's thumb, which `List::set_palette` + \
             `snap_to_state` put there the way the other two are put there"
        );
        let dark_toggle = demo.toggle.style().track;
        let dark_progress = demo.progress.style().fill;
        let dark_scroll = demo.list.scroll().thumb.get();

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
        assert_ne!(
            demo.list.scroll().thumb.get(),
            dark_scroll,
            "and the scrollbar, whose two colours the demo writes rather than sets"
        );
        assert_eq!(
            demo.list.scroll().thumb.get(),
            ScrollPalette::from_theme(&Theme::light()).thumb,
            "arriving at the light theme's own TextMuted"
        );
    }

    #[test]
    fn the_two_widgets_with_no_focus_state_say_where_focus_is() {
        // An `Image` and a `Progress` have no `focused` property, so a `Tab` onto
        // either of them would show nothing at all and the demo would be telling
        // the reader a control is selected with no mark on it. Their readouts say
        // so in words, which is the whole of what a widget with no focus state of
        // its own can be given.
        let mut demo = laid_out();
        for _ in 0..6 {
            demo.handle_event(key(Keycode::Tab));
        }
        assert_eq!(
            demo.focused,
            Some(demo.progress.handle()),
            "six Tabs from the top is the progress bar"
        );
        demo.frame(WINDOW, Duration::from_millis(16));
        assert_eq!(
            demo.readout_text_of(&demo.progress_readout).as_deref(),
            Some("50%, determinate, focused"),
            "and its readout says so"
        );

        // The other one is the **fourth** stop, so a fresh demo rather than a
        // walk from the progress bar: the band's order is the buttons, the
        // slider, the image, the toggle, the bar and the list.
        let mut demo = laid_out();
        for _ in 0..4 {
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
            .map(|ch| metrics.advance(ch, BUTTON_FONT))
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
        // text column's last label ends at y 501, the button band's column starts
        // at x 664 and the list's readout is at x 1000. `no_two_placed_rects_overlap`
        // says no two boxes touch; this says which boxes this one is clear of, so a
        // failure names the neighbour.
        let demo = laid_out();
        let at = |handle: Handle| demo.node_rect(handle).expect("a laid-out node");
        let fps = at(demo.fps_readout.label.handle());

        let column = TEXT_PANEL_ORIGIN.0 + TEXT_COLUMN_WIDTH;
        let band = BUTTON_ORIGIN.0;
        assert!(
            fps.x + fps.width <= band,
            "the readout at {fps:?} reaches into the button column at x {band}"
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
        let mut demo = laid_out();
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
}
