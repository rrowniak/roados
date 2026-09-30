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

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
#[cfg(test)]
use sdl3::keyboard::Mod;
use sdl3::mouse::MouseButton;
use std::cell::RefCell;
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
#[cfg(test)]
use ui_core::paint::DrawCommand;
use ui_core::paint::{Color, PaintState, Painter};
use ui_core::property::Property;
use ui_core::render::context::Context;
use ui_core::render::Renderer;
use ui_core::theme::{PropertyValue, Theme, ThemeToken};
use ui_core::widgets::button::{Button, Callback, Motion, Palette};
use ui_core::widgets::container::Container;
use ui_core::widgets::label::{Label, LayoutOptions, TextAlign, Truncation, WrapMode};
use ui_core::widgets::slider::{Orientation, Palette as SliderPalette, Slider};
// The button band's `Callback` is the payload-free alias of this same type, so
// the demo imports it under a second name: a slider's handler takes the value it
// moved to, and `Callback::from_fn` on the alias would be `Callback<()>`.
use ui_core::widgets::Callback as ValueCallback;

/// The window, and the box the root is laid out in.
const WINDOW: Size = Size {
    width: 1024.0,
    height: 600.0,
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

/// How far below the slider its value readout sits.
///
/// A slider asks for the same minimum touch target the button band does, 44
/// pixels tall, so this clears it and leaves the readout's own 24-pixel line
/// inside the window.
const SLIDER_READOUT_DROP: f32 = 52.0;

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
    let sdl = renderer.sdl();
    let mut events = sdl.event_pump()?;
    let mut demo = Demo::new(TextMetrics::new(font))?;

    let mut last = Instant::now();
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
    }

    Ok(())
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

/// Lifts a colour toward white, for a pad's held colour: the pad's rest colour
/// is a theme token, and its held colour is that token lifted toward white, so
/// a press reads as the same hue brightened.
fn lighten(color: Color) -> Color {
    Color::interpolate(&color, &Color::new(255, 255, 255, 255), HELD_LIGHTEN)
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
    fn new(metrics: TextMetrics) -> Result<Self, &'static str> {
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
        widget.snap_to_state();
        // The size is the widget's own, rather than a number written out here: a
        // slider has no content to measure, so this is the widget saying how big
        // a slider should be until a caller says otherwise.
        {
            let size = widget.size();
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

        // The row, the counter, the slider and its readout are each placed inside
        // the band, which is what `Absolute` is for: the row at the band's own
        // offset, the counter `COUNTER_DROP` below it, and the slider and its
        // readout below that.
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
        if !button_area.add_child(&mut nodes, button_row.handle())
            || !button_area.add_child(&mut nodes, counter.label.handle())
            || !button_area.add_child(&mut nodes, slider.widget.handle())
            || !button_area.add_child(&mut nodes, slider_readout.label.handle())
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

        // The tree never changes shape, so the order is computed once.
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
            }
            // A finger is a pointer too, and a car has no mouse: the same press
            // and release the left button gets, from the touch events SDL delivers
            // for the same gesture. A canceled touch drops the slider as well as
            // the pointer, because a canceled finger is one that is gone.
            Event::FingerDown { x, y, .. } => {
                if self.slider_at(x, y).is_some() {
                    self.slider_dragging = true;
                }
            }
            Event::FingerUp { .. } | Event::FingerCanceled { .. } => {
                self.slider_dragging = false;
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
    /// A handle that belongs to neither the band nor the slider is nobody's, which
    /// is what lets one loop serve both without asking what is there.
    fn offer_to(&self, handle: Handle, event: &mut InputEvent) -> bool {
        if handle == self.slider.node() {
            return match self.slider_rect() {
                Some(rect) => self.slider.widget.on_event(event, rect),
                None => false,
            };
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
    /// slider each time, which is what makes a disabled button fall out of the
    /// order rather than sit in it.
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
            // The slider is the last stop in the tree's paint order, so `Tab`
            // reaches it after the band. It has no disabled state of its own, so
            // it is always in the order.
            focus.set_focusable(self.slider.node(), true);
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
    fn frame(&mut self, size: Size, delta: Duration) {
        let _ = self.clock.tick(delta);
        let _ = self.theme.tick(delta);
        self.track_hover();
        self.sync_button_state();
        self.sync_slider_state();
        for button in &self.buttons {
            let _ = button.widget.tick(delta);
        }
        let _ = self.slider.widget.tick(delta);

        let mut nodes = self.nodes.borrow_mut();
        Layout::new(&mut nodes).layout(self.root, Constraints::tight(size));

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
            let Some(node) = nodes.get_mut(handle) else {
                continue;
            };
            let Some(rect) = node.layout().rect() else {
                continue;
            };
            let mut options = demo_label.options;
            options.line_height = self.metrics.line_height(text_size);
            let commands = demo_label.label.paint(rect.into(), &options, &|ch: char| {
                self.metrics.advance(ch, text_size)
            });
            *node.paint_mut() = PaintState::from_commands(commands);
        }

        // The click counter is reached through the band, so it is painted the
        // same way the panel's labels are.
        {
            let Some(node) = nodes.get_mut(self.counter.label.handle()) else {
                return;
            };
            let Some(rect) = node.layout().rect() else {
                return;
            };
            let mut options = self.counter.options;
            options.line_height = self.metrics.line_height(BUTTON_FONT);
            let commands = self
                .counter
                .label
                .paint(rect.into(), &options, &|ch: char| {
                    self.metrics.advance(ch, BUTTON_FONT)
                });
            *node.paint_mut() = PaintState::from_commands(commands);
        }

        // And the slider's readout, which is reached through the band as well.
        // It is painted last of all so the text is on top of the slider it is
        // reporting.
        {
            let Some(node) = nodes.get_mut(self.slider_readout.label.handle()) else {
                return;
            };
            let Some(rect) = node.layout().rect() else {
                return;
            };
            let mut options = self.slider_readout.options;
            options.line_height = self.metrics.line_height(BUTTON_FONT);
            let commands = self
                .slider_readout
                .label
                .paint(rect.into(), &options, &|ch: char| {
                    self.metrics.advance(ch, BUTTON_FONT)
                });
            *node.paint_mut() = PaintState::from_commands(commands);
        }
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

    /// Hands the recorded commands to the renderer, in paint order.
    fn draw(&mut self, renderer: &mut Renderer) {
        let mut nodes = self.nodes.borrow_mut();
        for handle in self.order.iter().copied() {
            renderer.draw_node(handle, &mut nodes);
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
        let motion = Motion::from_theme(&new_theme);
        self.theme.switch_to(new_theme, THEME_TRANSITION);
        for button in &mut self.buttons {
            button.widget.set_palette(palette);
            button.widget.animate_to_state(motion);
        }
        self.slider.widget.set_palette(slider_palette);
        self.slider.widget.animate_to_state(motion);
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
            .is_some_and(|rect| {
                x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
            })
            .then_some(())
    }

    /// Returns the slider's rect in window coordinates, or `None` if it has not
    /// been laid out.
    ///
    /// This is the rect the slider draws inside and the one a pointer event is
    /// measured against, so it is the demo's own statement of where the slider is
    /// rather than each caller working it out — and it is why the widget's
    /// `on_event` takes a rect: a node cannot reach the arena that holds it.
    fn slider_rect(&self) -> Option<ui_core::paint::Rect> {
        let nodes = self.nodes.borrow();
        nodes
            .get(self.slider.node())?
            .layout()
            .rect()
            .map(Into::into)
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
    fn demo() -> Demo {
        Demo::new(mono_metrics()).unwrap()
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
        // The walk has four stops since task 14 added the slider: the two
        // buttons that can be activated, the slider, and back round. The claim
        // here is still that the disabled one is never visited, and it is stated
        // over the whole walk rather than over the band's first three.
        let mut demo = laid_out();
        let enabled: Vec<Handle> = demo
            .buttons
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != 1)
            .map(|(_, button)| button.node())
            .collect();
        let slider = demo.slider.node();

        let mut visited = Vec::new();
        for _ in 0..4 {
            demo.handle_event(key(Keycode::Tab));
            visited.push(demo.focused);
        }

        assert_eq!(
            visited,
            vec![
                Some(enabled[0]),
                Some(enabled[1]),
                Some(slider),
                Some(enabled[0])
            ],
            "the walk is press, reset, the slider, and wraps back to press — \
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
    fn every_parent_in_the_demo_is_a_container_widget() {
        // The demo used to assemble its own parent nodes, which meant two
        // implementations of the same composition primitive in one repository:
        // the demo's private helper and the widget. Nothing in the suite would
        // have noticed a new one appearing, so this is the check that a node
        // with children is a `Container` and not a node the demo wired up.
        let demo = demo();
        let nodes = demo.nodes.borrow();
        let containers: Vec<Handle> = demo.containers.iter().map(Container::handle).collect();
        let mut parents = 0;
        for &handle in &demo.order {
            let node = nodes.get(handle).expect("a node in the demo's tree");
            if node.children().is_empty() {
                continue;
            }
            parents += 1;
            assert!(
                containers.contains(&handle),
                "node {handle:?} has children but is not a Container"
            );
        }
        assert_eq!(parents, containers.len(), "and every one of them is");
        assert_eq!(
            containers.len(),
            6,
            "the demo has six: the card, the text \
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

        let rect = demo.slider_rect().expect("a laid-out slider");
        assert_eq!(
            painted_thumb_x(&demo),
            rect.x + rect.width - 12.0,
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

        // A finger that goes down on the track a little right of the minimum and
        // drags forty pixels further lands at 70: the point it started from is
        // where the thumb would be at half, and the drag adds 40 of the 216 the
        // thumb can travel.
        let (x, y) = demo.slider_at_fraction(0.5).expect("a laid-out slider");
        for event in drag_on(x, y, x + 40.0, y) {
            demo.handle_event(event);
            demo.frame(WINDOW, Duration::from_millis(16));
        }
        let readout = demo.readout_text().expect("a readout");
        assert!(
            readout.starts_with("70 of 100"),
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
}
