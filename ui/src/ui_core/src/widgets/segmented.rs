//! The Segmented widget: one value chosen from a row of labelled columns.
//!
//! A segmented control is a node, the [`Choice`] list the caller gives it, the
//! [`value`](Segmented::value) it holds, and the two properties a caller
//! writes. It is not a matrix, a mesh, a parent of its columns, a
//! [`LayoutMode::Grid`](crate::layout::LayoutMode::Grid), a clock or a drag
//! target: the columns are painted, not placed, and there is nothing to
//! animate, so there is no `tick`, no `Style`, no `Motion` and no
//! [`AnimationClock`](crate::animation::AnimationClock).
//!
//! The value is a point on a scale, not an index into the choices — the same
//! distinction [`Slider::set_step`](crate::widgets::slider::Slider::set_step)
//! documents with its *"cannot reach 1"*: replacing the enumeration must keep
//! a stored value the new one still carries rather than resetting the control.
//! [`Segmented::set_choices`] is that replacement, and
//! [`Segmented::selected`] presents the stored value through the new
//! enumeration. There is no `Property<Choice>` for the same reason: a choice
//! is a label for a value, and the value is what survives.
//!
//! The colours follow the button's rule rather than adding theme tokens: a
//! [`Palette`] names the five colours and the properties hold none of them, so
//! a theme switch is a [`Segmented::set_palette`] away. A mode dimension in
//! the theme is row `L9`'s, so `theme.rs` is untouched here.
//!
//! Two things are the caller's, not the segmented's: which node is dirty, and
//! which node is focused. The first is a property callback per the demo's own
//! idiom — `on_change` on the animated properties, marking the node — and the
//! second is [`input::Focus`](crate::input::Focus), which knows the focus order
//! and not what a right arrow means.

use crate::arena::{Arena, Handle};
use crate::input::{InputEvent, InputEventKind, Key};
use crate::layout::{Offset, Size};
use crate::node::{self, WidgetNode};
use crate::paint::{DrawCommand, Painter, Rect};
use crate::property::{Color, Property};
use crate::theme::Theme;
use crate::widgets::label::{layout_text, LayoutOptions, TextAlign, Truncation, WrapMode};
use crate::widgets::Callback;

/// How wide one segment is, in pixels.
///
/// Seven columns span 448 pixels, which is the climate fan row's own width, so
/// a seven-choice segmented control fits the strip it was built for exactly.
/// A constant rather than a theme token for the reason the button's own
/// `MIN_TOUCH_TARGET` documents: the theme has no token for a segment's parts.
pub const SEGMENT_WIDTH: f32 = 64.0;

/// How tall the control is, in pixels.
///
/// At the 44-pixel touch floor, so every column is a target a finger can hit.
pub const SEGMENT_HEIGHT: f32 = 44.0;

/// The font size the segment labels are drawn at, in pixels.
pub const SEGMENT_FONT_SIZE: f32 = 16.0;

/// How far inside its column the selected highlight and the labels sit, in
/// pixels.
pub const SEGMENT_PADDING: f32 = 6.0;

/// The corner radius of the track and the selected column, in pixels.
pub const SEGMENT_RADIUS: f32 = 8.0;

/// One labelled point on the segmented control's scale.
///
/// A choice is a label for a value, not the value's index: replacing the list
/// keeps a stored value the new list still carries, whatever position it now
/// holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// The text drawn in the choice's column.
    pub label: String,
    /// The scale value choosing this column writes.
    pub value: f32,
}

impl Choice {
    /// Returns a choice showing `label` for `value`.
    #[must_use]
    pub fn new(label: impl Into<String>, value: f32) -> Self {
        Choice {
            label: label.into(),
            value,
        }
    }
}

/// The colours a segmented control is drawn in.
///
/// The states are not colours of their own — the theme has no token per state —
/// so the control is themed with one track, one label colour, one selected
/// pair and one ring colour, and [`Segmented::paint`] derives the selection
/// from them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The track the columns sit on.
    pub track: Color,
    /// A column's label at rest.
    pub text: Color,
    /// The selected column's fill.
    pub selected_fill: Color,
    /// The selected column's label.
    pub selected_text: Color,
    /// The focus ring's colour.
    pub ring: Color,
}

impl Default for Palette {
    /// Returns a neutral grey control: legible without a theme, and a visible
    /// starting point for a caller that will theme it.
    fn default() -> Self {
        Palette {
            track: Color::new(64, 64, 64, 255),
            text: Color::new(240, 240, 240, 255),
            selected_fill: Color::new(120, 120, 120, 255),
            selected_text: Color::new(255, 255, 255, 255),
            ring: Color::new(255, 255, 255, 255),
        }
    }
}

impl Palette {
    /// Returns the palette a theme describes.
    ///
    /// The track is [`Border`](crate::theme::ThemeToken::Border) — the control
    /// at rest reads as chrome — the selected column is
    /// [`Primary`](crate::theme::ThemeToken::Primary) with its label in
    /// [`OnPrimary`](crate::theme::ThemeToken::OnPrimary), and the resting
    /// labels and the ring are [`Text`](crate::theme::ThemeToken::Text). No
    /// token is added: a mode dimension in the theme is row `L9`'s.
    #[must_use]
    pub fn from_theme(theme: &Theme) -> Self {
        Palette {
            track: token_color(theme, crate::theme::ThemeToken::Border),
            text: token_color(theme, crate::theme::ThemeToken::Text),
            selected_fill: token_color(theme, crate::theme::ThemeToken::Primary),
            selected_text: token_color(theme, crate::theme::ThemeToken::OnPrimary),
            ring: token_color(theme, crate::theme::ThemeToken::Text),
        }
    }
}

/// A segmented control: a row of labelled columns choosing one scale value.
///
/// The widget holds the properties the task gives it — [`value`](Segmented::value),
/// [`focused`](Segmented::focused) and [`focus_ring`](Segmented::focus_ring) —
/// the choice list, one palette, and its node. The columns are painted, not
/// placed, so the node is a leaf and the layout never sees the choices.
pub struct Segmented {
    /// The chosen scale value. The truth; the paint pass presents it through
    /// whatever choices are currently set.
    pub value: Property<f32>,
    /// Whether the control holds focus. Written by the caller, from
    /// [`input::Focus`](crate::input::Focus).
    pub focused: Property<bool>,
    /// The focus ring's thickness, in pixels. Zero draws no ring even when the
    /// control is focused.
    pub focus_ring: Property<f32>,
    choices: Vec<Choice>,
    palette: Palette,
    node: Handle,
}

impl Segmented {
    /// Creates a segmented control showing `choices` in the arena, and returns
    /// it.
    ///
    /// The value starts at the first choice's, or `0.0` with no choices: a
    /// control with nothing to choose has no value to hold, and `0.0` is the
    /// scale's origin rather than a claim about it.
    #[must_use]
    pub fn new(nodes: &mut Arena<WidgetNode>, choices: Vec<Choice>) -> Self {
        let node = node::create(nodes, crate::layout::LayoutState::new());
        let value = choices.first().map_or(0.0, |choice| choice.value);
        Segmented {
            value: Property::new(value),
            focused: Property::new(false),
            focus_ring: Property::new(2.0),
            choices,
            palette: Palette::default(),
            node,
        }
    }

    /// Returns the control's node in the arena.
    #[must_use]
    pub fn handle(&self) -> Handle {
        self.node
    }

    /// Returns the choices the control shows, in column order.
    #[must_use]
    pub fn choices(&self) -> &[Choice] {
        &self.choices
    }

    /// Returns the colours the control is drawn in.
    #[must_use]
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// Sets the colours the control is drawn in.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Replaces the choice list, keeping the stored value wherever the new
    /// list still carries it, and reports whether the stored value moved.
    ///
    /// An empty list leaves the value alone: there is nothing to project onto,
    /// and clearing the caller's value as a side effect would be a second
    /// writer of it. A value one of the new choices carries stays bit-identical
    /// rather than being rewritten to an equal neighbour. Any other value is
    /// projected onto the nearest choice's, with a tie going to the lower
    /// value — and to the lower index where the values themselves tie.
    #[must_use]
    pub fn set_choices(&mut self, choices: Vec<Choice>) -> bool {
        self.choices = choices;
        let Some(projected) = project(&self.choices, self.value.get()) else {
            return false;
        };
        if projected == self.value.get() {
            return false;
        }
        self.value.set(projected);
        true
    }

    /// Returns the selected column: the exact index when one choice carries
    /// the stored value, else the nearest, else `None` with no choices.
    ///
    /// Derived from the value on every call, so a value written directly —
    /// through [`value`](Segmented::value) rather than [`Segmented::select`] —
    /// is presented through the current choices all the same.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        if self.choices.is_empty() {
            return None;
        }
        let current = self.value.get();
        if let Some(index) = self
            .choices
            .iter()
            .position(|choice| choice.value == current)
        {
            return Some(index);
        }
        Some(nearest(&self.choices, current))
    }

    /// Writes the `index`th choice's value, and reports whether the stored
    /// value moved.
    ///
    /// `false` past the end, clamping nothing: an index no column holds is a
    /// caller error, and silently choosing a neighbour would be answering a
    /// question that was not asked.
    pub fn select(&self, index: usize) -> bool {
        let Some(choice) = self.choices.get(index) else {
            return false;
        };
        if choice.value == self.value.get() {
            return false;
        }
        self.value.set(choice.value);
        true
    }

    /// Returns the size the control asks for: one [`SEGMENT_WIDTH`] column per
    /// choice, [`SEGMENT_HEIGHT`] tall.
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(
            SEGMENT_WIDTH * count_to_f32(self.choices.len()),
            SEGMENT_HEIGHT,
        )
    }

    /// Returns the column `position` falls in, or `None` with no choices or
    /// outside `rect`.
    ///
    /// The columns tile the rect left to right, so the index is the floor of
    /// the fractional column, clamped to the last one: a position on the rect's
    /// own right edge is in the last column rather than past it.
    #[must_use]
    pub fn segment_at(&self, position: Offset, rect: Rect) -> Option<usize> {
        if self.choices.is_empty() {
            return None;
        }
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return None;
        }
        if position.x < rect.x
            || position.x > rect.x + rect.width
            || position.y < rect.y
            || position.y > rect.y + rect.height
        {
            return None;
        }
        let width = rect.width / count_to_f32(self.choices.len());
        let last = count_to_f32(self.choices.len() - 1);
        let raw = ((position.x - rect.x) / width).floor().clamp(0.0, last);
        // The one float-to-integer `as` cast in the module, for the same
        // reason and with the same guarantee as the button's own: the cast is
        // saturating (Rust 1.45 and later), so the clamp is belt and braces
        // and states the intent — a column fraction inside the row cannot wrap
        // a column round to the other end of it.
        let index = raw as usize;
        Some(index.min(self.choices.len() - 1))
    }

    /// Returns a write putting the stored value back where it is now.
    ///
    /// This captures the value, not the property: restoring after the value
    /// has moved puts back the captured one rather than re-reading the moved
    /// one, which would be a no-op wearing a snapshot's clothes.
    #[must_use]
    pub fn save(&self) -> Callback<()> {
        let target = self.value.clone();
        let saved = self.value.get();
        Callback::new(move || target.set(saved))
    }

    /// Handles `event` as this control would inside `rect`, and reports
    /// whether it consumed it.
    ///
    /// A [`Tap`](InputEventKind::Tap) selects the column under it and is
    /// consumed; a tap outside the rect belongs to whatever is behind the
    /// control. An arrow or d-pad key moves one column and is consumed, but
    /// only while the control holds focus; the move clamps at both ends and
    /// never wraps. Every other event is left alone and not consumed.
    pub fn on_event(&self, event: &mut InputEvent, rect: Rect) -> bool {
        match event.kind() {
            InputEventKind::Tap => {
                if !over(rect, event.position()) {
                    return false;
                }
                let Some(index) = self.segment_at(
                    event.position().unwrap_or(Offset::new(rect.x, rect.y)),
                    rect,
                ) else {
                    return false;
                };
                event.consume();
                let _ = self.select(index);
                true
            }
            InputEventKind::KeyDown { key, .. } => {
                if !self.focused.get() {
                    return false;
                }
                let Some(delta) = choice_key(&key) else {
                    return false;
                };
                let Some(current) = self.selected() else {
                    return false;
                };
                event.consume();
                let next = current.saturating_add_signed(delta);
                let _ = self.select(next.min(self.choices.len() - 1));
                true
            }
            _ => false,
        }
    }

    /// Returns the draw commands that paint the control within `rect`.
    ///
    /// The commands are, in order: the focus ring, if the control is focused
    /// and the ring is not zero wide; the track; the selected column, inset by
    /// [`SEGMENT_PADDING`]; and one text run per choice in column order, in the
    /// selected colour on the selected column and the resting colour
    /// elsewhere. With no choices there is nothing to choose and nothing is
    /// drawn — not even the track, which would be a bar answering to no value.
    ///
    /// `advance` supplies a character's width and `line_height` the height of
    /// one line, the same seam
    /// [`label::LayoutOptions`](crate::widgets::label::LayoutOptions) takes, so
    /// this is testable without a font.
    #[must_use]
    pub fn paint(
        &self,
        rect: Rect,
        advance: &dyn Fn(char) -> f32,
        line_height: f32,
    ) -> Vec<DrawCommand> {
        if self.choices.is_empty() {
            return Vec::new();
        }
        let mut painter = Painter::new();
        let ring = self.focus_ring.get();
        if ring > 0.0 && self.focused.get() {
            painter.rounded_rect(grow(rect, ring), SEGMENT_RADIUS + ring, self.palette.ring);
        }
        painter.rounded_rect(rect, SEGMENT_RADIUS, self.palette.track);

        let count = count_to_f32(self.choices.len());
        let column = rect.width / count;
        let selected = self.selected();
        if let Some(index) = selected {
            let inset = Rect::new(
                rect.x + count_to_f32(index) * column + SEGMENT_PADDING,
                rect.y + SEGMENT_PADDING,
                (column - SEGMENT_PADDING * 2.0).max(0.0),
                (rect.height - SEGMENT_PADDING * 2.0).max(0.0),
            );
            painter.rounded_rect(
                inset,
                (SEGMENT_RADIUS - SEGMENT_PADDING).max(0.0),
                self.palette.selected_fill,
            );
        }

        let inner = (column - SEGMENT_PADDING * 2.0).max(0.0);
        for (index, choice) in self.choices.iter().enumerate() {
            let options = LayoutOptions {
                max_width: inner,
                line_height: line_height.max(0.0),
                align: TextAlign::Center,
                wrap: WrapMode::None,
                truncation: Truncation::Ellipsis,
                ..LayoutOptions::default()
            };
            let layout = layout_text(&choice.label, &options, advance);
            let color = if Some(index) == selected {
                self.palette.selected_text
            } else {
                self.palette.text
            };
            let left = rect.x + count_to_f32(index) * column + SEGMENT_PADDING;
            // Centre the run block in the rect's own height: the rect is the
            // control's own size wherever the caller laid it out as one.
            let mut top = (rect.y + (rect.height - layout.total_height) / 2.0).max(rect.y);
            for line in &layout.lines {
                painter.text(
                    left + line.x_offset,
                    top,
                    &line.text,
                    color,
                    SEGMENT_FONT_SIZE,
                    0.0,
                );
                top += options.line_height;
            }
        }
        painter.finish()
    }
}

/// Projects `current` onto `choices': the value the store should hold after
/// the list is replaced.
///
/// `None` with no choices — the caller leaves the value alone. A value one of
/// the choices carries is returned as-is, bit-identical. Any other value
/// returns the nearest choice's, with a tie going to the lower value and then
/// the lower index.
fn project(choices: &[Choice], current: f32) -> Option<f32> {
    if choices.is_empty() {
        return None;
    }
    if choices.iter().any(|choice| choice.value == current) {
        return Some(current);
    }
    Some(choices[nearest(choices, current)].value)
}

/// Returns the index of the choice nearest `current`, with a tie going to the
/// lower value and then the lower index.
///
/// The caller guarantees `choices` is not empty.
fn nearest(choices: &[Choice], current: f32) -> usize {
    let mut best = 0;
    let mut best_distance = (choices[0].value - current).abs();
    for (index, choice) in choices.iter().enumerate().skip(1) {
        let distance = (choice.value - current).abs();
        if distance < best_distance
            || (distance == best_distance && choice.value < choices[best].value)
        {
            best = index;
            best_distance = distance;
        }
    }
    best
}

/// Returns `count` as an `f32`, saturating at `u16::MAX`.
///
/// There is no `From<usize>` for `f32` in std, so this goes through `u16` —
/// the same route the icon atlas-fit test takes — rather than an `as` cast.
/// A segmented control with more than 65,535 columns is not a control.
fn count_to_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

/// Returns whether `position` is over `rect`, edges included.
///
/// A caller that hands [`Segmented::on_event`] an event has already decided
/// this by routing it; the check here is what stops a tap aimed at the panel
/// behind the control from selecting a column from across the window. `None`
/// answers yes: a key press has no position, and there is nothing to test it
/// against.
fn over(rect: Rect, position: Option<Offset>) -> bool {
    let Some(position) = position else {
        return true;
    };
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return true;
    }
    position.x >= rect.x
        && position.x <= rect.x + rect.width
        && position.y >= rect.y
        && position.y <= rect.y + rect.height
}

/// Returns `rect` grown by `by` on every side.
///
/// The button's and the toggle's own helper, repeated rather than imported: a
/// `pub` utility in another widget's module is not a shared place to keep one.
fn grow(rect: Rect, by: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by,
        rect.width + by * 2.0,
        rect.height + by * 2.0,
    )
}

/// Returns which way `key` moves the selection: right and down forward, left
/// and up back, the d-pad likewise.
///
/// The mapping is orientation-free — a segmented row has one axis — so unlike
/// the slider's own `adjustment` there is no orientation to consult.
fn choice_key(key: &Key) -> Option<isize> {
    use sdl3::gamepad::Button as Pad;
    match key {
        Key::Keyboard(sdl3::keyboard::Keycode::Right)
        | Key::Keyboard(sdl3::keyboard::Keycode::Down)
        | Key::Gamepad(Pad::DPadRight)
        | Key::Gamepad(Pad::DPadDown) => Some(1),
        Key::Keyboard(sdl3::keyboard::Keycode::Left)
        | Key::Keyboard(sdl3::keyboard::Keycode::Up)
        | Key::Gamepad(Pad::DPadLeft)
        | Key::Gamepad(Pad::DPadUp) => Some(-1),
        _ => None,
    }
}

/// Returns the colour a theme holds for `token`, or black if it holds something
/// else.
///
/// A theme's own tables keep each token to its own kind, so this is a fallback
/// for a token a caller has written the wrong variant into — and black rather
/// than a panic, because a mistyped theme token is not worth taking a frame
/// down for. It is the button's own helper, repeated rather than imported: it
/// is four lines, and a shared module for one four-line helper is a module.
fn token_color(theme: &Theme, token: crate::theme::ThemeToken) -> Color {
    theme
        .get(token)
        .as_color()
        .unwrap_or(Color::new(0, 0, 0, 255))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Offset;

    /// A monospace advance of 5 pixels per character.
    fn mono(_: char) -> f32 {
        5.0
    }

    /// A line 20 pixels tall.
    const LINE: f32 = 20.0;

    /// The exterior-lights enumeration: four choices at `0.0..=3.0`.
    fn exterior() -> Vec<Choice> {
        vec![
            Choice::new("Off", 0.0),
            Choice::new("Parking", 1.0),
            Choice::new("On", 2.0),
            Choice::new("Auto", 3.0),
        ]
    }

    /// The wiper enumeration: six choices at `0.0..=5.0`.
    fn wipers() -> Vec<Choice> {
        vec![
            Choice::new("Off", 0.0),
            Choice::new("Auto", 1.0),
            Choice::new("I", 2.0),
            Choice::new("II", 3.0),
            Choice::new("III", 4.0),
            Choice::new("IIII", 5.0),
        ]
    }

    /// The rect a four-column control paints in at its own size.
    fn rect_of(control: &Segmented) -> Rect {
        let size = control.size();
        Rect::new(10.0, 20.0, size.width, size.height)
    }

    /// The text runs in `commands`, in order.
    fn texts(commands: &[DrawCommand]) -> Vec<&DrawCommand> {
        commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Text { .. }))
            .collect()
    }

    #[test]
    fn the_stored_value_survives_an_enumeration_change_and_is_presented_through_the_new_one() {
        let mut nodes = Arena::new();
        let mut control = Segmented::new(&mut nodes, exterior());
        control.value.set(3.0);
        let before = control.value.get().to_bits();

        let renamed = vec![
            Choice::new("Off", 0.0),
            Choice::new("Parking", 1.0),
            Choice::new("On", 2.0),
            Choice::new("Automatic", 3.0),
        ];
        assert!(!control.set_choices(renamed), "the value did not move");
        assert_eq!(
            control.value.get().to_bits(),
            before,
            "an exact value stays bit-identical"
        );
        assert_eq!(control.selected(), Some(3));
    }

    #[test]
    fn a_value_no_choice_carries_is_projected_onto_the_nearest_one() {
        let mut nodes = Arena::new();
        let mut control = Segmented::new(&mut nodes, exterior());
        control.value.set(2.7);

        assert!(control.set_choices(exterior()));
        assert_eq!(control.value.get(), 3.0);
        assert_eq!(control.selected(), Some(3));
    }

    #[test]
    fn a_projection_tie_goes_to_the_lower_choice() {
        let mut nodes = Arena::new();
        let mut control = Segmented::new(&mut nodes, exterior());
        control.value.set(0.5);

        assert!(control.set_choices(exterior()));
        assert_eq!(control.value.get(), 0.0);
        assert_eq!(control.selected(), Some(0));
    }

    #[test]
    fn an_empty_choice_set_leaves_the_value_alone_and_draws_nothing() {
        let mut nodes = Arena::new();
        let mut control = Segmented::new(&mut nodes, exterior());
        control.value.set(2.0);

        assert!(!control.set_choices(Vec::new()), "nothing to project onto");
        assert_eq!(control.value.get(), 2.0);
        assert_eq!(control.selected(), None);
        assert!(control
            .paint(Rect::new(0.0, 0.0, 256.0, 44.0), &mono, LINE)
            .is_empty());
    }

    #[test]
    fn the_first_and_the_last_choice_are_both_reachable() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, wipers());
        assert_eq!(control.value.get(), 0.0, "a new control starts at rest");

        assert!(control.select(5));
        assert_eq!(control.value.get(), 5.0);
        assert_eq!(control.selected(), Some(5));
        assert!(control.select(0));
        assert_eq!(control.value.get(), 0.0);
        assert_eq!(control.selected(), Some(0));
        assert!(
            !control.select(6),
            "past the end changes nothing and says so"
        );
        assert_eq!(control.value.get(), 0.0);
    }

    #[test]
    fn segment_at_names_the_column_a_position_is_in_and_nothing_outside_the_rect() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        let rect = rect_of(&control);

        assert_eq!(control.segment_at(Offset::new(10.0, 30.0), rect), Some(0));
        assert_eq!(
            control.segment_at(Offset::new(74.0, 30.0), rect),
            Some(1),
            "64-pixel columns from x = 10"
        );
        assert_eq!(
            control.segment_at(Offset::new(265.9, 30.0), rect),
            Some(3),
            "the right edge clamps to the last column"
        );
        assert_eq!(control.segment_at(Offset::new(9.9, 30.0), rect), None);
        assert_eq!(control.segment_at(Offset::new(74.0, 19.9), rect), None);
        assert_eq!(
            control.segment_at(Offset::new(74.0, 64.1), rect),
            None,
            "outside in y is outside"
        );
    }

    #[test]
    fn a_tap_selects_the_choice_under_it_and_a_tap_elsewhere_selects_nothing() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        let rect = rect_of(&control);

        let mut tap = InputEvent::new(
            InputEventKind::Tap,
            Some(Offset::new(10.0 + 64.0 * 2.0 + 5.0, 30.0)),
        );
        assert!(control.on_event(&mut tap, rect));
        assert!(tap.consumed());
        assert_eq!(control.value.get(), 2.0);

        let mut away = InputEvent::new(InputEventKind::Tap, Some(Offset::new(500.0, 500.0)));
        assert!(!control.on_event(&mut away, rect));
        assert!(!away.consumed(), "and lets it travel on");
        assert_eq!(control.value.get(), 2.0);
    }

    #[test]
    fn an_unfocused_segmented_declines_a_key_and_lets_it_travel_on() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        let rect = rect_of(&control);
        let mut right = InputEvent::new(
            InputEventKind::KeyDown {
                key: Key::Keyboard(sdl3::keyboard::Keycode::Right),
                keymod: sdl3::keyboard::Mod::empty(),
            },
            None,
        );

        assert!(!control.on_event(&mut right, rect));
        assert!(!right.consumed());
        assert_eq!(control.value.get(), 0.0);
    }

    #[test]
    fn the_arrow_keys_move_one_choice_and_stop_at_both_ends() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        let rect = rect_of(&control);
        control.focused.set(true);
        let key = |code| {
            InputEvent::new(
                InputEventKind::KeyDown {
                    key: Key::Keyboard(code),
                    keymod: sdl3::keyboard::Mod::empty(),
                },
                None,
            )
        };

        let mut right = key(sdl3::keyboard::Keycode::Right);
        assert!(control.on_event(&mut right, rect));
        assert_eq!(control.value.get(), 1.0);

        let mut left = key(sdl3::keyboard::Keycode::Left);
        assert!(control.on_event(&mut left, rect));
        assert_eq!(control.value.get(), 0.0);

        let mut clamped = key(sdl3::keyboard::Keycode::Left);
        assert!(control.on_event(&mut clamped, rect));
        assert_eq!(control.value.get(), 0.0, "never wraps past the first");

        for _ in 0..10 {
            let mut far = key(sdl3::keyboard::Keycode::Right);
            assert!(control.on_event(&mut far, rect));
        }
        assert_eq!(control.value.get(), 3.0, "and never past the last");
    }

    #[test]
    fn paint_puts_the_focus_ring_under_the_track_and_the_selected_column_above_it() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        let rect = rect_of(&control);
        control.focused.set(true);
        let commands = control.paint(rect, &mono, LINE);

        assert_eq!(commands.len(), 3 + exterior().len());
        let fills: Vec<Color> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RoundedRect { color, .. } => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(
            fills,
            vec![
                control.palette.ring,
                control.palette.track,
                control.palette.selected_fill,
            ]
        );
    }

    #[test]
    fn every_choice_records_exactly_one_text_run_and_it_is_that_choice_s_own_label() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        let commands = control.paint(rect_of(&control), &mono, LINE);
        let runs = texts(&commands);

        assert_eq!(runs.len(), exterior().len());
        for (run, choice) in runs.iter().zip(exterior().iter()) {
            let DrawCommand::Text { text, .. } = run else {
                panic!("a text run");
            };
            assert_eq!(text, &choice.label);
        }
    }

    #[test]
    fn a_label_wider_than_its_column_is_truncated_within_it() {
        let mut nodes = Arena::new();
        let choices = vec![
            Choice::new("Windshield wipers are automatic", 0.0),
            Choice::new("On", 1.0),
        ];
        let control = Segmented::new(&mut nodes, choices.clone());
        let rect = Rect::new(0.0, 0.0, 128.0, 44.0);
        let commands = control.paint(rect, &mono, LINE);
        let runs = texts(&commands);

        assert_eq!(runs.len(), 2);
        let DrawCommand::Text { text, .. } = runs[0] else {
            panic!("a text run");
        };
        assert_ne!(text, &choices[0].label, "the long label does not fit");
        assert!(!text.is_empty());
    }

    #[test]
    fn the_text_runs_are_centred_in_their_own_column() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        let rect = rect_of(&control);
        let commands = control.paint(rect, &mono, LINE);
        let runs = texts(&commands);

        // "On" is two glyphs at 5 pixels: 10 of the 52 inner pixels, so its
        // run starts 21 pixels into the third column's inner box.
        let DrawCommand::Text { x, text, .. } = runs[2] else {
            panic!("a text run");
        };
        assert_eq!(text, "On");
        let column = 10.0 + 64.0 * 2.0;
        assert_eq!(*x, column + SEGMENT_PADDING + (52.0 - 10.0) / 2.0);
    }

    #[test]
    fn the_five_constants_are_positive_finite_and_seven_columns_span_448_pixels() {
        for constant in [
            SEGMENT_WIDTH,
            SEGMENT_HEIGHT,
            SEGMENT_FONT_SIZE,
            SEGMENT_PADDING,
            SEGMENT_RADIUS,
        ] {
            assert!(constant.is_finite() && constant > 0.0);
        }
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, wipers());
        control.value.set(6.0);
        let size = Segmented::new(
            &mut nodes,
            vec![
                Choice::new("1", 1.0),
                Choice::new("2", 2.0),
                Choice::new("3", 3.0),
                Choice::new("4", 4.0),
                Choice::new("5", 5.0),
                Choice::new("6", 6.0),
                Choice::new("7", 7.0),
            ],
        )
        .size();
        assert_eq!(size, Size::new(448.0, SEGMENT_HEIGHT));
        assert_eq!(SEGMENT_HEIGHT, 44.0, "every column is a touch target");
    }

    #[test]
    fn a_saved_value_is_put_back_where_it_was() {
        let mut nodes = Arena::new();
        let control = Segmented::new(&mut nodes, exterior());
        control.value.set(2.0);
        let restore = control.save();

        control.value.set(0.0);
        restore.call(());
        assert_eq!(control.value.get(), 2.0);
    }
}
