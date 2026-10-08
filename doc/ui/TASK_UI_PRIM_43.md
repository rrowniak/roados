# TASK_UI_PRIM_43: `Button::selected`, and a `TabBar` That Owns Its Buttons

## Goal

Close gap **#7** by giving `ui_core` the two things the gap row names and the demo
does not have: a **`selected` state on `Button`**, which is the *"active-state
indication"* half of the row, and **a `TabBar` widget** that owns a row of buttons
and knows which one is selected — the *"dedicated widget"* half.

## Context

This is **gap #7** of `DEMO_APPLICATION.md` § *Library gaps*, row **7**, and the
operator's decision of **2026-10-05** — § *Operator decisions (2026-10-05)* item
2, *"**Every library gap must be closed.** The 2026-10-03 decision to build the
page mechanism in `ui_demo` and leave gaps #3 and #7 open in `ui_core` is
**withdrawn**"* — makes it mandatory library work. Task 42 closed **#3**; this
closes **#7**, and **this task sits on `TASK_UI_PRIM_42`**, whose § *Out of Scope*
names this file: *"That is gap **#7** and `TASK_UI_PRIM_43` … This task hands 43 a
`Screens::show` to call and nothing else, and 43's six buttons will read the active
screen from `Screens::current_name()`."*

**The row is explicit that what 24.3 built is the prescription and not the
closure**, and the row says so in its own words: *"What `TASK_UI_PRIM_24.3` built
is the gallery's top tab bar from exactly `Button` + `Container`, which is the
prescription this row states and **not a closure of it**. **The "active-state
indication" half has no widget behind it either** — `Button` carries `hovered`,
`pressed`, `disabled`, `focused` and `activatable` … and **no `selected`**, so the
selected tab is a `background`/`foreground` swap the demo owns."* § *Relationship to
task 24* records the same thing from the other side: the gallery's page shell is
built **in `ui_demo`** out of `Container` + `Button`, *"which is what gap #7
prescribes for the dock"*, and *"**Withdrawn 2026-10-05** … Both gaps must now be
closed in `ui_core`, so what `24.1..3` built is the demo-level mechanism and **not
a substitute** for the library one."*

**So the two halves are named by the row itself, and this file delivers them in the
order the second depends on the first.** `Button::selected` is the state; `TabBar`
is the thing that writes it, animates it, and reports it.

### What is in the crate at `75a896c` plus the uncommitted diff, established and not re-derived

| Fact | Where |
|---|---|
| **Zero `TabBar`, `tab_bar` or `tabbar` identifiers anywhere in `ui_core`.** The bar exists only as demo-local code: `Demo::tabs: Vec<Tab>`, the private `struct Tab { page, button }`, and the constants `TAB_BAR_HEIGHT = 64.0`, `TAB_BAR_PADDING = 10.0`, `TAB_BUTTON_TALL = 44.0`, `TAB_BUTTON_GAP = 8.0`, `TAB_BUTTON_FONT = 18.0`, `TAB_BUTTON_PADDING_H = 14.0`. | `ui/src/ui_demo/src/main.rs`, `Demo::new`'s tab-bar block; `struct Tab` |
| **`Button`'s public fields, in declaration order, are sixteen:** `label`, `background`, `foreground`, `border_radius`, `on_click`, `focus_ring`, `padding_h`, `padding_v`, `font_size`, `hovered`, `pressed`, `disabled`, `focused`, `activatable` (defaults `true`), `scale`, `opacity`. **There is no `selected`**, and no second palette. | `ui/src/ui_core/src/widgets/button.rs`, `pub struct Button` |
| **`Button::may_activate()` is `!disabled.get() && activatable.get()`**, and its doc calls itself *"The one definition of 'does not act', for [`disabled`](Button::disabled) and [`activatable`](Button::activatable) together"*. **`activatable`'s own doc gives the reason it exists:** *"A second flag rather than a mode of [`Button::focused`], because the two answers come apart"* — one bit could not answer both *"does it draw the ring"* and *"may it fire"*. | `button.rs`, `Button::may_activate`, `Button::activatable` |
| **`Button::style()` computes the appearance from `self.palette` and the five flags**, applies the focus ring from `focused` independently of the rest, and lets **`disabled` alone** zero `ring_width` and force `scale = 1.0`. `Button::paint_faded` reads `self.background.get()`, `self.foreground.get()` and **`self.palette.ring`**. | `button.rs`, `Button::style`, `Button::paint_faded` |
| **`Button::animate_to_state(motion)` animates four properties** — `background`, `foreground`, `scale`, `opacity` — toward `style()`, and **`Button::tick(delta) -> bool`** drives them. **`Style::ring_width` is never animated**: the ring is the one appearance that appears and disappears with a flag rather than gliding. | `button.rs`, `Button::animate_to_state`, `Style` |
| **`ButtonState` is a `pub enum` in `button.rs` with five variants** — `Default`, `Hovered`, `Pressed`, `Disabled`, `Focused` — resolved by `Button::state()` in that precedence order, and its doc says it is *"the **primary** state: the states overlap"*. **The precedent for an interaction-state type, and the precedent for the overlap argument.** | `button.rs`, `pub enum ButtonState` |
| **`Container::new(nodes, mode)` takes a `LayoutMode`** and writes it onto the node; `set_padding` and `set_flex_config` write `LayoutState` and mark the node dirty. **`FlexConfig` carries `main_axis_alignment`, `cross_axis_alignment` and `spacing` — a main-axis gap and nothing else.** There is **no cross-axis gap** anywhere in the crate; that is gap **L7**. | `ui_core/src/widgets/container.rs`; `ui_core/src/layout.rs`, `FlexConfig`, `CrossAxisAlignment` |
| **`LayoutMode::row()` and `LayoutMode::column()` exist**; **`LayoutMode::Grid { .. } => Vec::new()`** in `arrange`'s match and `columns` is never read, and **`Flex.wrap` is discarded by the same `..`**. Gap **L3**, `TASK_UI_PRIM_52`, a separate task. **Do not depend on either.** | `layout.rs`, `LayoutMode`, `arrange` |
| **`Callback<T> = Rc<dyn Fn(T)>`** with `Callback::new(closure-with-no-args)` for `T = ()` and `Callback::from_fn` generally; **`Property::on_change` takes `Fn(&T)`**, and **`Property<T>`'s recompute closure is `Rc<dyn Fn()>`** — it is handed nothing. | `ui_core/src/widgets/mod.rs`, `Callback`; `ui_core/src/property.rs` |
| **`Dialog::add_action` is the precedent for a widget wiring a button's `on_click`**: it clones the existing handler, wraps it, and the clone shares state with the dialog. **`DialogAction` owns a `pub button: Button`** — a widget that owns its buttons is the shape here. | `ui_core/src/widgets/dialog.rs`, `Dialog::add_action`, `DialogAction` |
| **8 `pub fn on_event` impls** across 15 widget modules (`button`, `dialog`, `keyboard`, `list`, `scroll`, `slider`, `text_input`, `toggle`); arms cover only `Tap`, `Drag`, `Scroll`, `KeyDown`, `Text`. **`LongPress` and `Swipe` are consumed by nothing.** | `ui/src/ui_core/src/widgets/*.rs` |
| **`Button::on_event` consumes `Tap` unconditionally** and an activation key (`Return`, `KpEnter`, `Space`, gamepad `South`) **only while `focused`**, consulting `may_activate` on both. **`input::Focus` owns the focus order and nothing else**: `focus_next`, `focus_prev`, `handle_key` (`Tab` / `Shift+Tab`), `handle_scroll`. **It has no arrow-key movement at all.** | `button.rs`, `Button::on_event`; `ui_core/src/input.rs`, `Focus` |
| **`input::contains` is private** in `input.rs`; `list.rs` has its own private `covers(rect, position)` with the same inclusive-edge convention, and `List::item_at` / `Keyboard::key_at` are the precedent for a widget's own point-to-item test. **`input::route` sends a positionless event to `root` alone**, and its doc records why a caller routes rather than calls `dispatch_event`: **a property write from inside a handler while `dispatch_event` holds a `Ref` on the arena is a `RefCell` double borrow, which panics.** | `input.rs`, `contains`, `route`, `dispatch_event`; `widgets/list.rs`, `List::item_at`; `widgets/keyboard.rs`, `Keyboard::key_at` |
| **12 `pub enum`s in `widgets/`** — `GaugeType`, `ChartType`, `TextAlign`, `WrapMode`, `Truncation`, `Orientation`, `Severity`, `Anchor`, `ImageFit`, `ButtonState`, `KeyAction`, `Page` — **plus one private `enum Join` in `chart.rs`.** `keyboard::Page` is `{ Letters, Symbols }`, the on-screen keyboard's two-page toggle. **A name-collision risk for anything called a tab.** | `ui/src/ui_core/src/widgets/*.rs` |
| **The demo's selected appearance is a `Palette` swap, and it is two palettes**: `tab_palette(theme, true)` is `Palette::from_theme(theme)` — `Primary` / `OnPrimary` — and `tab_palette(theme, false)` is `Border` / `Text` with a `Primary` ring. **`Demo::aim_tab_buttons(&[Page])` re-points all six palettes and aims the two it was handed.** | `ui/src/ui_demo/src/main.rs`, `tab_palette`, `Demo::aim_tab_buttons` |
| **`ui/src/ui_core/src/nav.rs` does not exist yet.** `TASK_UI_PRIM_42` creates it, and its § *Out of Scope* is the sentence this file answers. **This task does not depend on it**: requirement 1–15 touch no `nav` symbol, and only sub-task 43.2 — which reads `Screens::current_name` — does. | `doc/ui/TASK_UI_PRIM_42.md` |
| **`LayoutState::visible`'s doc says *"only hit testing consults the flag"*, and task 42 amends it.** Task 42 also adds `LayoutState::hits`, defaults it to `true`, and gates `Focus` on visibility. **43.2 composes with both** and changes neither. | `layout.rs`; `doc/ui/TASK_UI_PRIM_42.md` requirements 8 and 9 |
| **`AGENTS.md`: no new dependency without the operator**; the approved **direct** dependencies are `sdl3 0.20`, `glow 0.18`, `freetype-rs 0.38`. Edition **2021**, `rust-version = "1.85"`. Tests go in a `#[cfg(test)] mod tests` beside the code; **no test needing a display, a network, a filesystem or the wall clock.** **The demo cannot receive a pointer event on this host** — XTEST pointer injection has never delivered one, keyboard delivered exactly one. **No acceptance criterion here may require a pointer-driven interaction.** | `AGENTS.md`; `doc/ui/IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture method* |
| **`.ai/tools/fps-check.sh` takes `seconds` then `floor` and runs the binary with no arguments**, so it **cannot name a page**; per page is `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>`. **Every run is measured** — `task-sequence.md` § *Gates*, *"No unmeasured run of the demo."* | `.ai/tools/fps-check.sh`; `doc/ui/IMPLEMENTATION_STATE.md` § *What the operator still has to decide* item 1 |
| **Test baseline: 1894** — `ui_core` **1450**, `ui_demo` **224**, doctests **220** — measured at `75a896c` plus the uncommitted diff, in the session that wrote this file. | `cargo test --all-features` from `ui/` |
| **The recorded release band is 61.1–63.9 fps**, floor **55**, from `demo.snap` after the frame-budget change. | `IMPLEMENTATION_STATE.md` § *The frame rate, measured* |

### The decision: `Button::selected`, a second palette, and a `TabBar` that owns its buttons

**Decision 1: `Button` gains `pub selected: Property<bool>`, and a second
`Palette` for it to select from. Decision 2: `ui_core::widgets::tab_bar::TabBar`
owns N `Button`s it built, a `Cell<Option<TabId>>` selection, one `Motion`, and the
single aim that writes every button's appearance. Decision 3: neither `TabBar` nor
`nav` depends on the other; the demo composes them by name.**

Five reasons for the shape, and the second is the one that makes the rest follow.

1. **A bar of caller-supplied `Button`s cannot own a selection at all, and cannot
   be unit-tested.** The caller would hold the `Vec<Button>`, so nothing stops it
   writing its own `selected`, and `TabBar` would have no node of its own — the
   buttons have theirs, and *who attaches them* is then unanswered. That shape is
   a `Vec` and a convention, and § *Testing* is built on the finding that a
   convention is not a mechanism: *"a sweep of a mechanism's call
   sites is not a sweep of the data it is built from"* is a finding about exactly
   that.
2. **The second palette is what makes "the six pages are pixel-identical" a
   checkable claim instead of a hope, and that is a design argument rather than a
   preference.** The demo's two appearances are **`Primary`/`OnPrimary` against
   `Border`/`Text`** — a *hue* change, and `Border`/`Text` are neither
   `Primary`/`OnPrimary` nor a darkening of it. **Any `selected` that derived its
   appearance from the one existing palette** — an alpha, a lighten, a scale —
   **would therefore draw six colours the demo does not draw today**, and the
   strongest criterion this file could have written would die on the first
   comparison. Two palettes, both written by the caller, keep every colour and put
   the choice where it already is. **`tab_palette`'s `true` branch is already
   `Palette::from_theme(theme)`, so the selected half needs no new colours at all.**
3. **A widget that owns its buttons must own their aims, or 24.3's one major
   returns.** `release_tab`'s `animate_to_state` was load-bearing **only** on the
   ordinary gesture — press and release on the button of the page already on show
   — because every fixture reached the bar through a page *switch*, which re-aims
   everything. **If the flags live in the widget and the aims live in the demo,
   the release aim is a call site again and can be dropped again.** So the aim is
   inside `TabBar`, in one function, guarded by the state it last aimed for, and
   § *Testing* names the test that drives the press and the release with **no
   selection change at all**.
4. **`Callback` is `Fn`, so a click cannot write the widget's own state, and the
   crate's answer to that is already written.** The demo's `pending_page` is a
   `Property` a handler can set and the frame drains; `route`'s doc records the
   reason a handler must not write through the arena at all. `TabBar` uses the same
   two steps: a click writes `requested`, and **`TabBar::sync` drains it into
   `TabBar::select`** — which is also what puts the move outside any live dispatch,
   the property `Demo::show_page`'s own doc says it is for.
5. **A `Button::selected` that answered *"may it fire"* would be a second bit
   answering `activatable`'s question.** `activatable` exists because one bit
   could not answer both *"does it draw the ring"* and *"may it fire"*;
   `may_activate`'s doc calls itself the one definition of *"does not act"*. **A
   third term there would make a selected button refuse to be pressed** — wrong
   for a radio, wrong for a segmented control, and wrong on a head unit where the
   one thing a driver knows is that the tab they are looking at is the tab they can
   press. § *One bit, one question* says where the idempotence lives instead.

### One bit, one question: what `selected` does to `paint`, to `may_activate`, and to the ring

**`selected` answers exactly one question — *"is this control the current member of
its group?"* — and it reaches the screen on two paths and touches neither
predicate that already has a documented owner.**

- **To `paint`: one changed line, and the animation is not it.** `selected` enters
  the animation through `style()`, which is the *only* path `animate_to_state`
  reads, and so a selection change is carried by the same four properties
  `hovered`, `pressed` and `disabled` are carried by — `background`, `foreground`,
  `scale`, `opacity`. **`paint_faded` itself changes on exactly one line: the ring's
  colour, which it reads from `self.palette.ring`, now comes from whichever
  palette is the base.** That line is required — the demo's unselected tab has a
  `Primary` ring and its selected one has `OnPrimary`, and the ring colour is the
  one appearance `paint` reads from a palette rather than from an animated
  property. **This is stated rather than glossed, because "the selected flag does
  nothing to `paint`" would be the false claim
  `DEMO_APPLICATION.md` § *Corrections to the second gap table* records three
  times.**
- **To `may_activate`: nothing, and `grep` is the criterion.** `selected` appears
  nowhere in `may_activate`, and `Button::may_activate`'s doc is unchanged except
  for one sentence saying why `selected` was deliberately left out. **Idempotence
  belongs to `TabBar`**, which owns the group and can answer *"is this already
  selected"* in one place — `TASK_UI_PRIM_42`'s
  `a_switch_to_the_screen_already_on_show_changes_nothing_at_all` is the same rule
  at the same place, and 24.3's `show_page` early return is the demo's copy of it.
- **To the focus ring: the width, nothing; the colour, the palette.** The ring's
  *width* stays a function of `focused` alone, with `disabled` the only thing that
  zeroes it. **A selection that suppressed the ring would make the ring mean
  "focused and not selected", which is a third question** — and the demo does the
  opposite today, distinguishing its two appearances by ring *colour*. The ring's
  *colour* follows the base palette, which is the one-line change above.
- **`ButtonState` gains no variant, and that is the decision rather than the
  omission.** `ButtonState`'s doc already says it is *"the **primary** state"* and
  that states overlap; `state()`'s precedence is `disabled`, `pressed`, `hovered`,
  `focused`, `Default`. **A `Selected` variant would have to sit in that chain**, and
  every position in it is wrong: above `disabled` a disabled selected tab would
  read as selected, below `hovered` a hovered selected tab would read as hovered,
  and **a selected tab's `ButtonState` would then stop describing its background,
  which is what that enum is for.** So the enum is unchanged and `selected` is
  reached the way `activatable` is: through `style()`, not through `state()`.

**And the second palette's default is the load-bearing choice, exactly as
`activatable`'s was.** `selected_palette` defaults to `Palette::default()` — the
*same* neutral grey `palette` starts at — so a `Button` nobody selected resolves
`style()` from `self.palette` and **is byte-identical to what it drew before this
field existed**, and a button a caller selected without configuring the second
palette resolves to the identical colours. `activatable`'s doc records the same
trampoline in reverse (*"A `false` default looks like the more conservative choice
and is a trap"*), and
`the_second_palette_defaults_to_the_unselected_one` is the criterion that carries
the addition, in the shape of `hits_defaults_to_true_on_a_fresh_layout_state`.

### `TabBar`'s shape, and how it composes with task 42's `Screens`

**Decision: `TabBar` owns the buttons and the selection. It owns no content, no
screen, no history and no transition. Neither it nor `nav` knows the other exists,
and the composition is by *name*.**

- **A tab is a private row — a `Button` plus the `(selected, hovered, pressed)`
  triple it was last aimed for — and the public vocabulary is `TabBar` and
  `TabId`.** There is **no `pub struct Tab`** and **no `pub enum Page`** here.
  `keyboard::Page` is a `pub enum` with two variants, thirteen `pub enum`s already
  share `widgets/`, and `DEMO_APPLICATION.md`'s own page list is a *gallery* fact
  that § *Relationship to task 24* says must stay in `ui_demo`; a navigation `Page`
  in the library would be a third spelling of one thing. `Task 42`'s § *The `Page`
  collision* settled this once — *"a module path is this repository's answer to a
  same-named type, and a rename is not"* — and this file applies that ruling rather
  than opening it again. `TabId::index` is a function of the tab's position in the
  bar's own list, and nothing else.
- **A tab's content is not part of a tab, because the content is a screen.** The
  bar draws a row of buttons; `Screens` holds a subtree per name and says which one
  is on show. **A tab that carried its own content node would be a second owner of
  the caller's tree**, which is the reason `TASK_UI_PRIM_42` requirement 4 gives for
  `Screens::add` *not* attaching: *"a registry that also built trees would be a
  second owner of the demo's shape."* `TabBar::new` builds its **buttons** — a
  widget building the widgets it owns is what `Dialog::add_action` does — and
  nothing else.
- **The dependency runs through a string, and both halves are named.** Task 42
  decided `ScreenId` plus a `&'static str` name per screen because *"a screen a
  `--tab=` flag can ask for needs a spelling"*. `TabBar::select_name(&str)` and
  `TabBar::label(&TabId) -> Option<&str>` meet that vocabulary exactly, and
  `Screens::current_name() -> Option<&'static str>` is what the demo feeds it. **So
  the demo's single composition point is one line** —
  `let _ = self.bar.select_name(self.screens.current_name().unwrap_or_default());` —
  called from the one place that calls `Screens::show`.
- **`TabBar` does not depend on `nav`, and `nav` does not depend on `TabBar`.**
  *`TabBar` → `nav`* is refused because a bar whose selection were derived from a
  screen table could not sit over anything else — not a segmented control, not a
  settings page, not a dialog's two actions — and gap **#7** is *"the bottom dock"*,
  which is a dock and not a screen table. *`nav` → `TabBar`* is refused because a
  registry of screens that knew about a tab widget would be a second owner of the
  demo's shape, on the library side this time. **The demo composes them, and the
  composition is two representations of one fact** — `Screens::current` and
  `TabBar`'s `Cell<Option<TabId>>` — **whose mitigation is a named test and not a
  design change**, on task 42's precedent: *"Two representations of one fact is a
  hazard and the mitigation is a test."* That test is
  `the_tab_bar_selection_and_the_screens_current_name_never_disagree`, in
  requirement 26.

### Selection semantics: idempotence, the keyboard, and what reports the change

**Re-selecting the current tab is a total no-op.** `TabBar::select` returns `false`,
writes nothing, aims nothing and **does not fire `on_select`**. This is task 42's
rule, 24.3's `show_page` early return, and the reason it is a rule rather than an
optimisation: in the composed demo a click reaches the selection **twice** — once
through the bar's own `requested`, and once through `Demo::show_page`'s
`select_name` — and **an idempotent re-selection that still aims is not
idempotent**. Only a `Property::on_change` counter on `button.background` can see
that, which is why the test uses one.

**The keyboard path, decided in two halves.**

- **Activation keys come free and are the demo's existing path.** `Button::on_event`
  already consumes `Return`, `KpEnter`, `Space` and gamepad `South` while focused,
  and fires the button's `on_click` — which `TabBar::new` wrote. **So Space and
  Enter on a focused tab select it with nothing added to `on_event`**, and
  § *Testing* has a test that drives it through `Button::on_event` rather than
  calling `select`. **That the demo's existing key path needs no rewiring is the
  reason this half is free**, and `Demo::offer_to`'s arm already reaches a tab
  button's `Button::on_event`
- **Arrows and Home/End are `TabBar`'s, and this task ships them without a demo
  consumer — recorded as the honest limit, in task 42's own words about `pop`.**
  `TabBar::on_event` consumes `Left`, `Right`, `Home`, `End` and the gamepad's
  `DPadLeft` / `DPadRight`, **and only while a tab holds focus**, wrapping at both
  ends. `Slider::adjustment` is the precedent for a widget mapping arrows, and a
  tab strip without them is broken on a head unit. **Two reasons the demo does not
  get them in 43.2, and both are decisions rather than omissions:** first,
  `Focus` is a caller value the widget cannot reach, so an arrow key would move the
  selection and **leave the focus ring where it was** — a visible product change
  that no operator asked for; second, **the demo's key path would have to be
  rewired to offer a key to the bar rather than to the focused button**, which is
  `Demo::offer_to`'s shape and therefore 43.2's risk budget for a feature nobody
  asked for. **Wiring it is one arm in `Demo::offer_to` plus one line in
  `set_focus`, and it is named in § *Out of Scope* as the next thing to do.**
- **`TabBar` does not move focus, and the module doc says so with the reason.**
  Coupling selection to focus is a product decision with a visible consequence, and
  a widget that silently made it would be a second owner of the caller's
  `Focus`. **What a caller that wants them coupled adds is named: write
  `TabBar::focus(Some(id))` from its own `on_select`.**

**What reports the change: one `Callback<TabId>`, fired only on a move.** Because
`Callback<T>` is `Fn` and `Property::on_change` is `Fn(&T)`, **a handler cannot
mutate captured state without `Rc<RefCell<…>>` or `Rc<Cell<…>>`** — the demo's
`pending_page: Property<Option<Page>>` is that answer and it is the answer here.
`TabBar::on_select` is the **only** report; `TabBar::selected()` is the **only**
poll; and **there is no `Property` over the selection**, because the value is the
widget's, `TabId` is `Copy`, and a `Property` over a value nobody observes
reactively is a notification channel with no subscriber. **`Button::selected` *is* a
`Property`, on `Button`'s own reason for the other four flags — the widget resolves
its appearance from it** — **and unlike them it is written by the *bar* rather than
by a caller**, which is the `Keyboard::page` precedent restated: the value belongs
to the thing that owns the group, and `Property::on_change` is the node-dirty
idiom a caller who wants it has anyway.

### Layout: a `row`, one main-axis gap, and what it cannot do

**`Container::new(nodes, LayoutMode::row())`, `FlexConfig::with_spacing(TAB_SPACING)`
with `CrossAxisAlignment::Center`, and `Padding::all(TAB_BAR_PADDING)`** — the
shape `Demo::new` already builds, moved into the widget.

- **One line of tabs, and no wrapping.** `Flex.wrap` is discarded by the same `..`
  that discards `LayoutMode::Grid { .. } => Vec::new()`, so a bar that does not fit
  **overflows rather than wraps**, and `arrange_flex` clips an overflowing child.
- **No cross-axis gap, and that is gap L7.** `FlexConfig` has `spacing` on the
  main axis and **nothing on the cross axis**, so **the bar's own `Padding` is the
  only vertical inset there is.** No `Grid`, no `margin`, no `flex-shrink`, no
  `flex-basis`.
- **`size()` is the sum of the labels, and the measurement is the caller's.**
  Each tab's declared width is `Button::content_size(advance, line_height)` with
  its own `padding_h` and `font_size` — `content_size` is the only thing in the
  repository that knows how wide a label is at a given font, and the demo's own
  comment says so. `TabBar::size` therefore returns
  `sum(widths) + TAB_SPACING * (n - 1) + 2 * TAB_BAR_PADDING` by
  `2 * TAB_BAR_PADDING + TAB_HEIGHT` for `n ≥ 1`, and the **zero-tab case returns
  `Size::new(2 * TAB_BAR_PADDING, 2 * TAB_BAR_PADDING)`** so the arithmetic has no
  `n - 1` underflow — a bar with no tabs is the edge a reviewer should look at, and
  a saturating subtraction would make it look fine.
- **The geometry has one owner and no `Rect` parameter anywhere on the widget.**
  `bar_rect`, `tab_rect` and `tab_at` read the nodes' **own cached rects** from the
  arena, and `paint` reads the same two. **The 2026-10-01 rule is that
  when a control's geometry is drawn from one number, *"assert that hit testing
  and drawing read the same number, because they are two consumers of one
  constant"*** — so there is no second computation and no number to keep in step.
  `List::item_rect` recomputes from `item_height` × index and `TabBar` deliberately
  does **not** copy it, because a recomputed rect is a second source of truth the
  moment the bar's padding changes; the module doc says so.
- **A tab's height is `TAB_HEIGHT`, not `Button::size`.** `Button::size` floors at
  44 and `TAB_HEIGHT` *is* 44, so the floor would be indistinguishable from the
  number, and the floor is the widget's business while the number is the bar's.
  **The floor is still asserted, over the laid-out rects**, because a constant
  compared with itself is a sentence.

### Scope, measured against `developer.md` § *Scope check*

**Seven files and four components — over both thresholds.** Files:
`ui/src/ui_core/src/widgets/tab_bar.rs` (new), `ui/src/ui_core/src/widgets/mod.rs`
(one `pub mod`), `ui/src/ui_core/src/widgets/button.rs`,
`ui/src/ui_demo/src/main.rs`, `doc/ui/PRIMITIVES_ARCHITECTURE.md` (§ *Module
Layout*), `doc/ui/DEMO_APPLICATION.md` (row 7), `doc/ui/IMPLEMENTATION_STATE.md`
(the record). Components: **`Button::selected` and the second palette**; **the
`TabBar` widget**; **the demo's rewiring**; **the document amendments**.

**So it is split into two sub-tasks**, per `.ai/protocols/subagents.md`
§ *Implementation fan-out* — **file-isolated and sequential**, because 43.2 reads a
type 43.1 creates. **The counts after the split are the ones that matter, and each
half is under both thresholds: 43.1 is four files and two components
(`tab_bar.rs`, `widgets/mod.rs`, `button.rs`, `PRIMITIVES_ARCHITECTURE.md`; the
`Button` half and the widget), 43.2 is three files and two components
(`main.rs`, `DEMO_APPLICATION.md`, `IMPLEMENTATION_STATE.md`; the rewiring and the
documents).**

**The order is not 42 → 43 throughout.** `nav` does not exist yet, and **43.1 does
not need it** — requirement 1 through 15 name no `nav` symbol, which is the test
that the split is real. **43.2 cannot be briefed against a tree without `Screens`,
`Screens::current_name` and the six screen hosts**, so **43.2 is briefed against
42's landed code** and is a stop condition until 42 has landed. **43.1 may start
now.**

## Requirements

### Sub-task 43.1 — `ui_core`: `Button::selected`, the second palette, and `TabBar`

1. **`Button` gains `pub selected: Property<bool>`, declared immediately after
   `activatable` and before `scale`**, so the seventeen public fields read
   `label`, `background`, `foreground`, `border_radius`, `on_click`, `focus_ring`,
   `padding_h`, `padding_v`, `font_size`, `hovered`, `pressed`, `disabled`,
   `focused`, `activatable`, **`selected`**, `scale`, `opacity`. **Default
   `false`,** written in `Button::new` with the same comment shape as
   `activatable`'s and one more clause: *a caller that writes only the sixteen
   fields that existed must get exactly the button it got before this one was
   added.*
   - The doc comment is four paragraphs and every one is load-bearing: **the
   question it answers** (*"is this control the current member of its group?"*);
   **what it does not answer** (that it is *not* *"may this be activated"*, which is
   `activatable`, and *not* *"does this hold focus"*, which is `focused`, with
   `activatable`'s own reason quoted rather than paraphrased); **where it reaches
   the screen** (`style()`, and through it the four animated properties, plus the
   ring's colour — which is *not* animated, and says so); and **that it is not a
     `ButtonState` variant**, with `ButtonState`'s own *"the states overlap"* doc
     quoted as the reason.

2. **`Button` gains a private `selected_palette: Palette`, initialised in
   `Button::new` to `Palette::default()`** — the same neutral grey `palette` starts
   at — plus:
   - `#[must_use] pub fn selected_palette(&self) -> Palette`, and
   - `pub fn set_selected_palette(&mut self, palette: Palette)`,

   **both in `Button::set_palette`'s exact shape**, including `set_selected_palette`
   leaving the current appearance where it is and the appearance moving only when
   the caller aims. **A second `set_palette` and not a merged one**, because
   `Button` has one palette each for two roles and the symmetry with the existing
   setter is worth more than one fewer method.
   - **`fn base_palette(&self) -> Palette`, private**, returning `selected_palette`
     when `selected` is set and `palette` otherwise. **One private function rather
     than an `if` at each of its two call sites**, because two readers of *"which
     palette is the base"* is the pair of lists this repository keeps apologising
     for.

3. **`Button::style()` changes on two lines and nowhere else.** Its first two lines
   become `let base = self.base_palette();` and then `background: base.background`
   / `foreground: base.foreground`; the five flag derivations that follow are
   **byte-for-byte unchanged**, including *"`disabled` … zeroes `ring_width`"*.
   **The order matters and is not to be rearranged:** the flags are applied to the
   base's colours, so `hovered`'s `HOVER_LIGHTEN` still moves *the base's*
   background toward white — **which is what makes a hovered selected tab lighter
   than a hovered unselected one, and it falls out of the order rather than being
   written out.**

4. **`Button::paint_faded` changes on exactly one line**: the ring's colour becomes
   `with_opacity(self.base_palette().ring, opacity)` instead of
   `with_opacity(self.palette.ring, opacity)`. **Nothing else in the method
   changes** — the background still reads `self.background.get()`, the label still
   reads `self.foreground.get()`, and both are unchanged *because* `selected`
   reaches them only through `style()`. **Its doc gains one paragraph** naming the
   two paths by which `selected` reaches the screen, so that "the ring colour
   follows the palette and the rest follows the clock" is a claim in the file
   rather than something a reader has to reconstruct.

5. **`Button::may_activate` is not edited, and `selected` appears nowhere in it.**
   Its doc gains **one sentence**: *"It does not consult [`selected`](Button::selected),
   and a deliberate break of that is a test; a selected control still activates,
   because selecting it and pressing it are different questions."*
   **`ButtonState` is not edited at all** — § *One bit, one question* carries the
   decision and the test below carries it.

6. **`button.rs`'s module doc gains one paragraph** in the register the rest of the
   file uses: what `selected` is, that it is not a `ButtonState` variant and why, the
   two paths into the screen, and **that `selected` is a whole bit rather than a
   mode of `activatable` for the same reason `activatable` is a bit rather than a
   mode of `focused`** — one bit cannot answer two questions, and this one is a
   third question.

7. **A new widget module `ui/src/ui_core/src/widgets/tab_bar.rs`, declared
   `pub mod tab_bar;` in `ui/src/ui_core/src/widgets/mod.rs`** — the list carries
   **sixteen** `pub mod` entries where it carried **fifteen**, and the new name is
   **alphabetical** (`button`, `chart`, `container`, `dialog`, `gauge`, `image`,
   `keyboard`, `label`, `list`, `progress`, `scroll`, `slider`, **`tab_bar`**,
   `text_input`, `toast`, `toggle`).

8. **`pub struct TabId(usize)`**, deriving `Clone, Copy, Debug, Eq, Hash, Ord,
   PartialEq, PartialOrd`, with `#[must_use] pub fn index(self) -> usize`.
   **The `ScreenId` precedent from `TASK_UI_PRIM_42` requirement 2, verbatim in
   shape:** a newtype rather than a bare index, per `developer.md` § *API design*,
   and **a stable one** because a bar's tabs are append-only within a build and
   this task provides no `remove`. **There is no `Tab`, and there is no `Page`** —
   § *`TabBar`'s shape* carries the collision argument and cites task 42's ruling.

9. **The three constants, `pub`, in the module's own block, each doc-commented in
   the voice `Button::THEME_SPACING_SM` and `IMAGE_VERTEX_STRIDE` carry:**
   - `pub const TAB_SPACING: f32 = 8.0;` — the theme's `SpacingSm`, **copied rather
     than read**, with `Button::THEME_SPACING_SM`'s reason quoted: the widget has no
     theme to read it from and a caller themes a widget by binding properties.
   - `pub const TAB_BAR_PADDING: f32 = 10.0;` — the gap between the bar's box and
     its tabs, and **the number that makes a tab land at `y 10`**, because a `row`
     places a child at the padded edge of its parent's inner box.
   - `pub const TAB_HEIGHT: f32 = 44.0;` — one tab's height, and **the same 44 two
     other places already name**: `Button::MIN_TOUCH_TARGET`, privately and by
     flooring a button's own node at it, and `DEMO_APPLICATION.md`'s own
     design-principle item 6 (*"44dp touch targets"*). **The demo's
     `TAB_BUTTON_TALL` named a third — `keyboard::KEY_HEIGHT` — and that was
     wrong, because `KEY_HEIGHT` is `52.0`:** the constant's doc claimed *"the same
     44 three other places already name"* and listed a number that is not 44.
     **This is a familiar shape rather than a new one — a doc comment asserting
     the opposite of the code beside it, which `DEMO_APPLICATION.md`
     § *Corrections to the second gap table* records three times — and requirement 18
     deletes the constant, so the false claim goes with it.** The handoff says so
     rather than letting the deletion look like tidying.

   **A test asserts all three are finite and positive and that `TAB_HEIGHT` is at
   least 44**, because a `TAB_HEIGHT` below the touch-target floor would be a
   `TabBar` that ships a target a finger cannot hit — the same shape as task 40's
   `ROTOR_PITCH_LIMIT` test, which admits three values and not a fourth.

10. **The private row type, and it is private on purpose:**

    ```rust
    /// One tab: the button the bar owns, and the state it was last aimed for.
    struct Tab {
        /// The button. Reached through [`TabBar::tab`], not from here.
        button: Button,
        /// The `(selected, hovered, pressed)` triple this tab was last aimed
        /// for. **`focused` is not in it**, because `Style::ring_width` is the
        /// one appearance `animate_to_state` never animates — the ring appears
        /// and disappears with the flag rather than gliding, so aiming at it
        /// would cost six `animate_to_state` calls a frame and move nothing.
        aimed: (bool, bool, bool),
    }
    ```

    **`Tab` is not `pub`**, so nothing outside the module can hold one and the
    public vocabulary is `TabBar` and `TabId`. **`ButtonState` is not extended and
    neither is `Button`:** a widget owning buttons is `DialogAction`'s shape, and
    `Dialog::actions` is public only because a caller must reach
    `DialogAction::handle` — which `TabBar::tab_id_of` and `TabBar::tab` already
    provide.

11. **`pub struct TabBar`, five fields, and nothing else:**

    ```rust
    /// A row of buttons, one of which is selected.
    ///
    /// **`Debug`**, because a failed assertion prints it. **`Clone` is
    /// deliberately absent**, on `Screens`'s argument: nothing in this task copies
    /// a `TabBar`, and `developer.md` § Phase 2's *"No abstraction before the
    /// second use"* applies to a `derive` as much as to a trait.
    #[derive(Debug)]
    pub struct TabBar {
        /// The row the tabs sit in: its node, its background and its radius.
        ///
        /// **Public because a caller binds the two painted properties to theme
        /// tokens**, which is the whole of what `Container` is for here, and
        /// because `Dialog::actions` is public for the same reason.
        pub bar: Container,
        /// Fired when the selection **moves**, with the tab that is now selected.
        ///
        /// **The one report, and it is fired only on a move.** `Callback<T>` is
        /// `Fn`, so a handler cannot mutate captured state without
        /// `Rc<Cell<…>>` or `Rc<RefCell<…>>` — the demo's `pending_page` is that
        /// answer and it is the answer here.
        pub on_select: Callback<TabId>,
        /// The tab a click asked for, waiting for [`TabBar::sync`].
        ///
        /// **A `Property`, and that is the whole of the click mechanism.** It is
        /// `Clone` and `Property::set` is `&self`, so every button's click
        /// handler holds a clone and writes it; the widget drains it in `sync`.
        /// The alternative — a click reaching `select` directly — needs the
        /// handler to hold the `TabBar`, which `Callback`'s `Fn` bound forbids and
        /// which an `Rc<RefCell<TabBar>>` would answer with a cycle and a
        /// `RefCell` double borrow. `input::route`'s doc records the borrow
        /// hazard; `Demo::pending_page`'s doc records the two-hop answer.
        requested: Property<Option<TabId>>,
        /// Which tab is selected, if any.
        ///
        /// **A `Cell<Option<TabId>>` and not a `Property`**, because the value is
        /// the widget's, `TabId` is `Copy`, and nothing observes it reactively: a
        /// `Property` over a value with no subscriber is a notification channel
        /// with nobody at the other end. **`TabBar::selected` and
        /// `TabBar::on_select` are the whole of the outside world.**
        selected: Cell<Option<TabId>>,
        /// The motion every aim runs on.
        motion: Motion,
        /// The tabs, in the order their labels came in.
        tabs: Vec<Tab>,
    }
    ```

12. **`impl TabBar`, twenty-six methods, and nothing else.** `reviewer.md` flags
    *"Overly public API"* and **every name below has a named caller in requirement
    17–25 or a named test in requirement 13** — the count is stated so a later edit
    that adds one has to say which of those it is for.

    - `pub fn new(nodes: &mut Arena<WidgetNode>, labels: impl IntoIterator<Item = impl Into<String>>) -> Self`
      — **the container, the N buttons, N attachments, N click handlers, and
      `motion` at `Motion { duration: Duration::from_millis(150), easing: Easing::EaseInOut }`**, which are the *same two fallbacks* `Motion::from_theme` uses when a theme holds something else, so an unthemed bar's aim has a length and a curve rather than none. **Selection starts at `None`** — `Screens::new`'s rule, *"an empty table with nothing on show"*, and `Option` is the honest answer for a bar whose caller has not chosen yet. **It does not aim and it does not snap**; `snap_to_state` exists for the reason `Button::snap_to_state` does.
    - `#[must_use] pub fn handle(&self) -> Handle` — the bar's own node, which is what `node::attach` takes and what a caller registers with `Focus` and `input::route`.
    - `#[must_use] pub fn tab_ids(&self) -> Vec<TabId>` — **and no `len`.** `tab_ids().len()` is the count, and a second accessor for it is a second copy of one list.
    - `#[must_use] pub fn tab_id_of(&self, handle: Handle) -> Option<TabId>` — **a search over `tabs`, not an arithmetic position**, for `Demo::tab_button`'s reason: *"a search … rather than a position, because a handle is what routing and the paint walk arrive with"*.
    - `#[must_use] pub fn tab(&self, id: TabId) -> Option<&Button>` — **`Option`**, because `TabId` is constructible by a caller and a bar does not owe it a button.
    - `#[must_use] pub fn label(&self, id: TabId) -> Option<&str>`
    - `pub fn select(&self, id: TabId) -> bool` — **the one write path for the selection.** It refuses an index past the end, **writes nothing and fires nothing when the tab is already selected**, writes `Button::selected` on **both** the outgoing and the incoming button, and returns whether the selection moved. **It does not aim**: the aim is `sync`'s, on `aim_tab_buttons`'s argument.
    - `pub fn select_name(&self, name: &str) -> bool` — **resolves against `label`, which is the label `new` was given.** This is the composition point with `Screens::current_name`, and § *Selection semantics* carries why the vocabulary is a string.
    - `#[must_use] pub fn selected(&self) -> Option<TabId>`
    - `pub fn set_palette(&mut self, palette: Palette)` — **the unselected pair**, every tab, and `&mut self` because `Button::set_palette` takes it. **It does not aim**; `sync` does, when the caller says so — the asymmetry `Button::set_palette`'s own doc states.
    - `pub fn set_selected_palette(&mut self, palette: Palette)`
    - `pub fn set_motion(&mut self, motion: Motion)`
    - `pub fn snap_to_state(&self)` — **every tab, and each tab's `aimed` triple is written with the flags as they stand**, so the first `sync` after it is not an aim. `Button::new` wrote the *default* palette's colours and aiming alone leaves a bar grey until something moves it — the reason every widget in `Demo::new` has one.
    - `#[must_use] pub fn size(&self, advance: &dyn Fn(char) -> f32, line_height: f32) -> Size` — § *Layout* carries the arithmetic **including the zero-tab case**, and the doc says why `Button::size`'s 44 floor is *not* applied to a tab's width (the floor is a floor on the node, and `measure` declares the node's box outright).
    - `pub fn measure(&mut self, nodes: &mut Arena<WidgetNode>, advance: &dyn Fn(char) -> f32, line_height: f32) -> bool` — writes each tab's `Constraints::tight(Size::new(content.width, TAB_HEIGHT))` **through `Button::content_size` with that tab's own `padding_h` and `font_size` already set**, and the bar's own `Constraints::tight(self.size(..))`. **Returns whether any declared size moved**, so a caller can skip a layout pass on the frames nothing changed — `Demo::placed_handles`'s guard argument. **It is called once, from `Demo::new`, where the demo's own loop calls `Button::content_size` today**, and `measure` is the only writer of a tab's declared box.
    - `#[must_use] pub fn bar_rect(&self, nodes: &Arena<WidgetNode>) -> Option<Rect>`
    - `#[must_use] pub fn tab_rect(&self, nodes: &Arena<WidgetNode>, id: TabId) -> Option<Rect>`
    - `#[must_use] pub fn tab_at(&self, nodes: &Arena<WidgetNode>, position: Offset) -> Option<TabId>` — **a private `covers(rect, position)` free function in `List::covers`'s shape**, edges included, `input::contains`'s convention and `input::contains` being private. **`None` is the honest answer for a point outside the bar** and it is not consumed.
    - `pub fn focus(&self, focused: Option<TabId>)` — **writes `focused` on every tab, unguarded**, for `Demo::set_focus`'s recorded reason: *"a `Property::set` that writes the same value still notifies `on_change`, and a button has no such callback here; the guard would be the same two lines for no effect."*
    - `pub fn hover(&self, over: Option<TabId>) -> bool` — writes `hovered` on **the tab it is leaving and the one it is arriving at**, and nothing else. Returns whether a flag moved.
    - `pub fn press(&self, id: TabId) -> bool` — **the press is the caller's to write**, on `Demo::grab_key`'s reason: *"the gesture recogniser reports a tap on the **release**, so there is no 'the finger went down on the button' for the widget to read"*.
    - `pub fn release(&self) -> bool` — **one function and not an index**, on `Demo::release_tab`'s reason: *"the release has to be able to find out **whether** there was a press, and a condition on the current position would decline a release whose pointer had travelled off the button — which is a button left at 0.95 scale with no gesture left that could bring it back."* **That sentence is 24.3's major, and this method is why the same defect cannot return as an un-aimed call site.**
    - **`pub fn sync(&self) -> bool`** — **the one aim, and the only caller of `animate_to_state` in this module.** In order: **drain `requested`** into `select`; then **walk `tabs` in order** and, for each whose `(selected, hovered, pressed)` **differs from `aimed`**, `animate_to_state(self.motion)`, write the triple, and count it; return whether either half did anything. **The convergence is the load-bearing half**: a `sync` on a settled bar compares, finds nothing, and **does not restart a clock** — which is `sync_toggle_state`'s argument and the reason a per-frame `sync` is not a per-frame aim. **`focused` is not in the triple**, because `Style::ring_width` is the one appearance `animate_to_state` never animates; requirement 13 names the test.
    - `#[must_use] pub fn tick(&self, delta: Duration) -> bool` — every tab's clock, `Button::tick`'s contract, `OR`ed. **The per-frame cost is exactly what the demo's own `for tab in &self.tabs { tab.button.tick(delta) }` costs today**, and the handoff says so with a number.
    - `#[must_use] pub fn paint(&self, nodes: &Arena<WidgetNode>, advance: &dyn Fn(char) -> f32, line_height: f32) -> Vec<DrawCommand>` — **two lines of body and no `Painter` of its own**, because both halves already build one and the two concatenate in the tree's order:

      ```rust
      let mut commands = match self.bar_rect(nodes) {
          Some(rect) => self.bar.paint(rect),
          None => Vec::new(),
      };
      for (index, tab) in self.tabs.iter().enumerate() {
          if let Some(rect) = self.tab_rect(nodes, TabId(index)) {
              commands.extend(tab.button.paint(rect, advance, line_height));
          }
      }
      commands
      ```

      **A bar with no cached rect paints nothing at all** rather than a default box —
      the `None` arms are the honest answer and `a_tab_with_no_cached_rect_is_painted_
      as_nothing_and_not_as_a_default_box` names it. `List::paint`'s shape, and **the
      order is the tree's own order** — a parent before its children — so this is
      the recording order a draw-command assertion can pin, which is what
      `the_bar_still_records_the_command_sequence_it_recorded_before` reads
    - `pub fn on_event(&self, event: &mut InputEvent, focused: Option<TabId>) -> bool` — § *Selection semantics* carries it. **`focused.is_none()` returns `false` for every key, before the match.**

13. **The tests in `button.rs`, eight, and each names the mutation it kills.**

    - `a_button_nobody_selected_draws_exactly_what_it_drew_before_selected_existed` — **the safety criterion for the whole task**, in the shape of `a_button_written_only_with_focused_still_activates_and_still_draws_its_ring`: builds a button, records its paint, then writes `selected = true` **without configuring `selected_palette`**, records again, and asserts the two command vectors are equal. **A default of some other palette is killed by this test and by nothing else.**
    - `the_second_palette_defaults_to_the_unselected_one` — asserts `selected_palette() == palette()` on a fresh button. **One line, beside `Button::new`, not in `tab_bar.rs`**, because the thing under test is `Button`'s default.
    - `a_selected_button_draws_its_selected_palette_and_an_unselected_one_draws_its_own` — both backgrounds, both foregrounds and both ring colours, each against its own palette; **plus the control** that with `selected` false on both the two are equal, **so the test cannot pass for two palettes that always differ.**
    - `selecting_a_button_changes_nothing_it_paints_until_it_is_aimed` — the load-bearing claim that `selected` reaches the screen only through `style()`: `snap_to_state`, then `selected = true`, then the paint **is identical**, then `animate_to_state` + enough ticks, then it is not. **Kills a `selected` consulted by `paint_faded` directly**, which is the mutation a later edit is most likely to write.
    - `a_selected_button_ring_colour_comes_from_the_selected_palette` — **the one changed line in `paint_faded`**, asserted by reading the ring's alpha and colour out of the recorded commands with `focused` true and `selected` flipping. **Kills leaving `self.palette.ring` in place**, which is the single most likely mutation of requirement 4.
    - `a_selected_button_still_activates_and_still_draws_its_ring` — the `activatable` precedent by name: with `focused` true and `selected` true, `may_activate()` is `true`, `Enter` fires the click once and the command stream has **two** rounded rects with the ring at full alpha.
    - `may_activate_is_selected_independent_in_all_four_cases` — `selected` × `disabled` over `may_activate`, with a counter, **and the `activatable` pair beside it as the control** so the test cannot pass by always answering `false`.
    - `selected_is_not_a_button_state_and_the_primary_state_still_describes_the_background` — **the requirement 5 decision, as a test**: `selected` set, and `ButtonState` is `Default`; then `hovered` set as well and it is `Hovered`; then `disabled` set and it is `Disabled`. **Kills adding a `Selected` variant to the chain**, and the name says what is being held.

14. **The tests in `tab_bar.rs`, twenty-six**, on a
    `fn fixture() -> (Arena<WidgetNode>, TabBar, Vec<Handle>)` that builds a
    **four-tab** bar in an arena, lays it out **at a rect that is not the origin**
    (`Rect::new(664.0, 120.0, 520.0, 64.0)`, the rule that a
    geometry fixture at `(0, 0)` cannot see an origin read as an extent), and
    returns the tab handles.

    - `every_tab_button_is_a_child_of_the_bar_in_the_order_the_labels_came_in` — the node's child list, in order, against the labels.
    - `every_label_the_bar_was_given_names_exactly_one_tab` — **and a fifth label resolves to `None`**, which is the complement half.
    - **`a_button_that_is_not_a_tab_of_this_bar_is_never_aimed_and_never_painted_by_it`** — **the complement assertion, and the one that kills 24.1's dropped-row finding one level down.** A foreign `Button` on a node in the same arena: `tab_id_of` is `None` for it, `tab_rect` is `None` for the id it would have had, `tab_at` over its rect is `None`, and the bar's recorded stream contains none of its commands. **A membership assertion passes for a tab that was never added; this one cannot.**
    - `the_bar_places_its_tabs_left_to_right_with_the_gap_and_the_padding_it_declared` — the four rects, by their positions, against `TAB_SPACING` and `TAB_BAR_PADDING`.
    - `the_bar_is_exactly_as_tall_as_its_padding_and_its_tabs` — **the demo's `the_bar_is_exactly_as_tall_as_its_padding_and_its_buttons`, moved**, and 43.2 keeps the demo's by that name.
    - `every_tab_button_is_at_least_the_touch_target_floor_tall` — **over the laid-out rects, not over `TAB_HEIGHT` compared with itself.**
    - `the_measured_size_is_the_sum_of_the_labels_the_gaps_and_the_padding` — `size` against the four laid-out rects' extent.
    - `a_bar_with_no_tabs_is_a_box_of_its_padding_and_says_so` — **the `n - 1` edge**, and `size` is asserted rather than assumed to have clamped.
    - **`tab_at_tab_rect_and_hit_test_read_the_same_box`** — `input::hit_test(&nodes, bar.handle(), point)` for a point inside tab *k* returns tab *k*'s node, and `tab_at` returns `TabId(k)`, **for every tab, at four points per tab** (centre, and each edge's last pixel). **The 2026-10-01 rule as a test: a control that is drawn and cannot be operated is indistinguishable from one that works, to a suite that only looks at what was recorded.**
    - `the_bar_paints_its_background_and_then_its_tabs_in_the_bar_order` — **the command-position assertion**, which is what makes 43.2's pixel criterion a test rather than a capture.
    - `a_tab_with_no_cached_rect_is_painted_as_nothing_and_not_as_a_default_box` — the `None` path of `paint`.
    - `selecting_a_tab_moves_the_selection_and_reports_it_once` — `on_select`'s counter.
    - **`selecting_the_current_tab_is_a_total_no_op_including_the_aim`** — the idempotence criterion: `select` returns `false`, `selected()` is unchanged, **the `on_select` counter does not move**, and **a `Property::on_change` counter on the selected button's `background` does not move either**, and `is_animating()` is false on both ends. **The `on_change` counter is the whole test**: a `select` that short-circuits the callback but still calls `animate_to_state` is not idempotent, and no return value can see it.
    - `selecting_past_the_last_tab_changes_nothing_and_says_so` — the `Option` arm of `tab`.
    - **`select_name_resolves_a_label_and_refuses_one_that_is_not_a_tab`** — the **composition point with `Screens::current_name`**, asserted here rather than in the demo: `select_name` on a tab's own label moves the selection, on a name no tab carries changes nothing, and **the name it resolves against is `label(id)`, so the two cannot drift** — which is `Page::name` staying the only place the six names are written out, restated for the bar.
    - **`the_selected_tab_is_aimed_on_a_press_and_on_a_release_that_changed_no_selection`** — **24.3's one major, by name, and the test this file exists to write.** `press(id)`; assert the scale is moving down; `release()`; tick to arrival; **assert `scale == 1.0` exactly and that `selected()` never changed** — no `select` call anywhere in the test. **Kills moving the release aim out of `release`, into `sync`, or away entirely**, which is the mutation the 24.3 sweep missed because every fixture went through a page switch.
    - `a_click_through_the_buttons_own_event_selects_the_tab_and_reports_it_on_the_next_sync` — a synthetic `InputEventKind::Tap` driven through **`Button::on_event`**, not through `select`: it must fill `requested`, **`selected()` must still be `None`, and `on_select` must still read zero** until `sync` runs. **Kills a click that bypasses the two-hop**, and it is the mechanism requirement 4's `RefCell` paragraph rests on.
    - `an_activation_key_on_the_focused_tab_selects_it_through_the_same_path` — `KeyDown { key: Return }` with `focused` true, driven through `Button::on_event` then `sync`. **The keyboard half of the free half of the activation path.**
    - `an_unfocused_tab_declines_an_activation_key_and_lets_it_travel_on` — `Button`'s own rule, held from the bar's direction.
    - `left_and_right_and_home_and_end_walk_the_selection_and_wrap_at_both_ends` — **six presses from the middle, asserting the whole walk including the two wraps**, on a **four**-tab bar so a two-tab bar cannot make the arithmetic pass.
    - **`an_arithmetic_key_with_no_tab_focused_is_left_for_the_focused_control`** — `Left` with `focused == None` returns `false` and leaves the event unconsumed. **Kills consuming arrows unconditionally**, which would take a left/right away from every control behind the bar.
    - **`the_bar_consumes_nothing_but_the_four_navigation_keys`** — **a table over all nine `InputEventKind` variants**: only `KeyDown` carrying one of the six keys is consumed while a tab is focused, and `Swipe` and `LongPress` are **asserted not consumed by name**. **This is what makes row `L4`'s claim enforced against this widget from its own side, and a later task that consumes them here must break it deliberately.**
    - `a_selection_change_aims_exactly_two_tabs_and_touches_no_other_flag` — an `on_change` counter on **all four** of every tab's animated properties, across one `select`: exactly the outgoing and the incoming tab move, and **exactly the two properties that follow `style()` on a palette swap** (`background` and `foreground`) — `scale` and `opacity` do not, because a palette swap does not change them, and `Style::scale` is `1.0` in both branches.
    - `an_aim_is_not_repeated_on_a_second_sync_that_changes_nothing` — **one hundred `sync` calls** after one `select`, asserting the `on_change` counters moved **exactly four times in total** (two tabs × two properties) and not once more. **This is the per-frame-aim mutation, and it is the one the demo's `sync_toggle_state` argument exists for.**
    - `the_focus_record_is_not_part_of_the_aimed_triple` — **focus, hover, press, release, and one `sync`**: the `on_change` counters move for the hover and the press and the release and **nothing moves on the focus**, because `Style::ring_width` is never animated. **Kills putting `focused` in the triple**, which would cost six aims a frame for no visible change.
    - `the_bar_follows_a_new_palette_and_a_new_selected_palette_on_the_next_sync` — `set_palette` + `set_selected_palette` + `sync`, asserting each tab reaches its own pair's `background`, **and that a `set_palette` alone does not move a *selected* tab**, which is the assertion that catches the two setters being wired to the same slot.

15. **`doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Module Layout` gains one line**, `tab_bar.rs` — **`tab_bar.rs` — the row of tabs and the selected one`, alphabetically placed** in the widget list between `slider.rs` and `text_input.rs`. **Nothing else in that file is edited**: `developer.md` § Phase 2's *"do not restructure what you were not asked to touch"*, and the file's own § *Dependencies* is not this task's business.

16. **The verification suite for 43.1, produced, and it does not include a demo run.**
    From `ui/`: `cargo fmt --check`, `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings` clean,
    `cargo test --all-features` with the per-binary counts pasted and **no test
    deleted, renamed away or weakened**, `cargo doc --no-deps` clean, and `cargo
    audit` **recorded as not installed on this host, not passed**.
    **43.1 launches `ui_demo` zero times**, so `fps-check.sh` has nothing to
    measure and `task-sequence.md` § *Gates*' *"No unmeasured run of the demo"* is
    not in question — **and the handoff says that in those words rather than
    leaving a reader to wonder whether a run was missed.**
    **`DEMO_APPLICATION.md` is not touched by 43.1**, and that is task 42's rule for
    its own 42.1: **gap #7 must not be marked closed by a mechanism with no
    consumer.** 43.2 closes the row.

### Sub-task 43.2 — the demo's six tabs become one `TabBar`, and gap #7 closes

17. **`Demo::tabs: Vec<Tab>` is replaced by `Demo::bar: TabBar`, and `struct Tab` is
    deleted by name.** `Demo::new` builds the bar **where it builds the container
    today** — same position in the child list, same `Position`, same
    `Constraints` — from `Page::ALL.iter().map(|page| page.name())`, and:
    - **`self.bar.set_palette(tab_palettes(&theme).0)` and
      `self.bar.set_selected_palette(tab_palettes(&theme).1)`**, **both at
      construction, once**, from `target_theme()` — not `theme`, which is
      mid-transition for 300 ms after a switch, which is `Demo::aim_tab_buttons`'
      recorded argument.
    - **`self.bar.set_motion(tab_motion(&theme))`** at construction and on every
      theme switch.
    - **`for tab in self.bar.tab_ids()` sets `font_size` to `TAB_BUTTON_FONT` and
      `padding_h` to `TAB_BUTTON_PADDING_H`,** in that order, **before**
      `self.bar.measure(&mut nodes, &tab_advance(&metrics), tab_line_height(&metrics))`
      — because `Button::content_size` reads `padding_h`, `font_size` and the
      label, and **a width measured before they are set is a width for the widget's
      defaults**. `measure` returns whether any declared size moved and the
      **return value is discarded with `let _ =`**, because at construction nothing
      has been measured before and the answer is always `true` — a fact the comment
      says rather than a value the code branches on.
    - **`self.bar.snap_to_state()`** after the palettes, for the reason every other
      widget in `Demo::new` has one: `Button::new` wrote the *default* palette's
      colours, and aiming alone leaves the bar grey until something moves it.
    - **`self.bar.bar.background` and `.bar.border_radius`** are bound exactly as
      the container's are today — `ThemeToken::Surface` through a bound property,
      `ThemeToken::BorderRadiusMd` through `.as_number().unwrap_or(CARD_RADIUS_FALLBACK)`
      — **unchanged code on a differently-named field**, which is what
      `the_tab_bar_follows_a_theme_switch` asserts.

18. **Three of the demo's six tab constants are deleted by name** —
    `TAB_BAR_PADDING`, `TAB_BUTTON_TALL` and `TAB_BUTTON_GAP` — **because
    `TabBar` now owns them**, and `TAB_SPACING`, `TAB_BAR_PADDING` and `TAB_HEIGHT`
    are the widget's. **`TAB_BAR_HEIGHT = 64.0` and `CONTENT_TOP` stay**, and the
    reason is `TAB_BAR_HEIGHT`'s own doc: *"**`TAB_BAR_HEIGHT` plus this on the top
    and the bottom is exactly …"* becomes *"`TAB_HEIGHT` plus `TAB_BAR_PADDING` on
    the top and the bottom is exactly `TAB_BAR_HEIGHT`"*, **the same equality, with
    the two halves now named in two crates.** `TAB_BUTTON_FONT` and
    `TAB_BUTTON_PADDING_H` **stay**, for the reason their own docs give: they are
    the demo's, and every font size and horizontal padding the demo writes is its
    own constant.
    **`grep -c 'TAB_BAR_PADDING\|TAB_BUTTON_TALL\|TAB_BUTTON_GAP' ui/src/ui_demo/src/main.rs`
    returns 0**, and the three `ui_core` names appear in the demo only inside the
    equality assertion in requirement 26.

19. **`tab_palette(theme, selected) -> ButtonPalette` becomes
    `tab_palettes(theme) -> (ButtonPalette, ButtonPalette)`, returning
    `(unselected, selected)`**, with `selected` computed by `Palette::from_theme`
    and `unselected` by the existing three-token expression. **The six existing
    tests that read `tab_palette(&theme, true)` or `(&theme, false)` keep their
    names and their numeric assertions and read `.1` / `.0` instead** — which is
    the whole of the change, and `the_two_tab_palettes_differ_in_all_three_colours`
    is the new test that says the pair is a pair.

20. **`Demo::tab_bar()` returns `&TabBar` instead of `&Container`**, keeping its
    `#[cfg(test)]`, keeping `Demo::containers[TAB_BAR]` as the bar's entry, and
    **gaining a `handle()` that returns `self.bar.bar.handle()`** so every
    `demo.tab_bar().handle()` call site in the tests is unchanged.
    `the_container_with_a_background_are_the_card_and_the_bar` keeps its name and
    its assertions.

21. **Six functions are deleted by name, and the rule *a test of a helper
    cannot see a call site that stopped using it* is what makes the
    replacements one-line each:** `Demo::tab_focusables` → **`TabBar::tab_ids`**,
    `Demo::tab_button` → **`TabBar::tab_id_of`** + **`TabBar::tab`**,
    `Demo::tab_at` → **`TabBar::tab_at`**, `Demo::aim_tab_buttons` → **gone, into
    `TabBar::sync`**, `Demo::press_tab` → **`TabBar::press`**, `Demo::release_tab`
    → **`TabBar::release`**, and `Demo::sync_tab_hover` → **`TabBar::hover`**.
    **That is seven functions out and six in, and that rule is why each
    replacement is a delegation rather than a re-implementation**: a demo-local
    `tab_at` that computed its own rects would be a second geometry, and a
    demo-local `aim_tab_buttons` would be the aim 24.3's major came from.

22. **`Demo::pending_page`'s writer count goes from six to one.**
    `Demo::new` sets `self.bar.on_select = Callback::new(move |id: TabId| { if let Some(&page) = Page::ALL.get(id.index()) { asked.set(Some(page)); } })`,
    and **the `Callback::new(move || …)` written into each of the six buttons in
    `Demo::new` is deleted.** **The `get` and its `if let` are the point**, and §
    *Testing* says why in two names: an out-of-range index must not panic
    (`developer.md` § *Panics and unwrap*: no `unwrap`, no `expect`, no
    `panic!` in production code) **and a bar with more tabs than pages must be
    visible rather than silent** — which is `every_tab_index_names_the_page_at_that
    _index_of_page_all`'s other half.

23. **`Demo::frame` changes in three places, and nowhere else.**
    - **Where `for tab in &self.tabs { let _ = tab.button.tick(delta); }` stood**,
      `let _ = self.bar.tick(delta);` **followed by `self.bar.sync();`**, in that
      order, **both inside the same block the other widgets' ticks are in** — the
      tick is here and the **aim** is in `sync`, which is
      `sync_toggle_state`'s argument restated for the widget that now owns it.
    - **Immediately after that block, the `pending_page` drain that
      `handle_event` used to carry**, unchanged in body: read `pending_page`, clear
      it, `show_page(page)`. **It moves from the end of the event to the end of the
      frame, and that is a strengthening rather than a change of rule**:
      `Demo::show_page`'s doc says the drain is there so that *"a page switch never
      happens in the middle of a dispatch — a switch here would move rects and
      write `set_visible` under an event [`Demo::route_input_event`] is still
      walking up the chain"*, **and a frame boundary is strictly outside every
      dispatch.** The doc on `pending_page` is amended in the same edit, and
      **it names the new site and the test that reads the outcome.**
    - **The `for tab in &self.tabs` in `set_focus` becomes one expression, with no
      loop:** `self.bar.focus(focused.and_then(|handle| self.bar.tab_id_of(handle)))`,
      where `focused` is the `Option<Handle>` the function has just assigned. **The
      method returns `()`**, because `focus` writes six booleans whose value is
      already known to the caller — it cannot fail, so a `bool` would be a question
      with one answer

24. **`Demo::show_page` gains one line and keeps its early return.**
    After `screens.show(page.name())` and `screens.sync(&mut nodes)`,
    **`let _ = self.bar.select_name(page.name());`** — **and the early return for a
    page already on show stays**, because 24.3's review found it load-bearing
    (`release_tab`'s `animate_to_state` held down by nothing when the button of the
    page already on show is pressed) and task 42 preserved it as a rule. **The
    consequence is that a `show_page` that returns early does not call
    `select_name`, and that is correct**: the selection is already there, and
    `select` is idempotent, so the two routes reach the same place. **`select_name`'s
    return value is discarded with `let _ =`** and the comment says why — the
    selection is a mirror of `Screens`, not a second source of truth, so a caller
    that treated a `false` here as an error would be asserting the mirror outranked
    the thing it mirrors.

25. **The demo's remaining `Button`-reach paths, four, all one expression each:**
    - `Demo::offer_to`'s tab-button arm becomes

      ```rust
      if let Some(id) = self.bar.tab_id_of(handle) {
          if let Some(button) = self.bar.tab(id) {
              return button.on_event(event);
          }
      }
      ```

      **two nested `if let`s rather than a `let … else`, because the outer one is a
      guard and the inner one is the answer**, and because `offer_to` is `&self` with
      twenty-odd arms — a `?` in the middle of it would return from the whole
      function. **The arm's comment is amended in place** to say the arm now reaches
      the buttons through `TabBar` and that **`TabBar::on_event` is deliberately not
      in it** — § *Selection semantics*'s second reason, quoted.
    - `Demo::route_input_event`'s fallback asks
      **`self.bar.tab_at(&self.nodes.borrow(), Offset::new(x, y))`** in place of
      `Demo::tab_at`, and its **`chain.push` guard** — the one
      `nothing_the_demo_places_reaches_into_the_strip` pins — is unchanged.
    - `handle_event`'s press arms call **`self.bar.press(id)`** where they called
      `press_tab(index)`, and **the three release arms call
      `self.bar.release()`** where they called `release_tab()`. **The modal guards
      are unchanged** on the argument `Demo::offer_to`'s doc already records.
    - `Demo::frame`'s paint arm replaces the loop that records the six buttons at
      `rect.into()` with **`self.bar.paint(&arena, &advance, tab_line_height(&self.metrics))`**
      appended after the bar container's own command, **in tab order**.

26. **The demo tests. Kept by name, with every assertion unchanged:**
    `the_bar_is_exactly_as_tall_as_its_padding_and_its_buttons` (now reads
    `TAB_HEIGHT` for the buttons' half and the widget's own `bar_rect` for the
    bar's), `every_tab_button_is_at_least_the_touch_target_floor_tall`,
    `tab_walks_the_six_buttons_before_the_pages_own_controls` (now over
    `self.bar.tab_ids()`), `every_page_places_every_rect_where_the_gallery_placed_it`,
    `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`,
    `placed_handles`, `page_rects`, `always_painted_handles`,
    `nothing_the_demo_places_reaches_into_the_strip`,
    `the_container_with_a_background_are_the_card_and_the_bar`,
    `the_tab_bar_follows_a_theme_switch`,
    `the_tab_bar_is_still_first_in_the_route_chain_for_each_of_its_six_buttons`
    (task 42.2's, **on all six pages**, and 43.2 must not weaken it to one),
    `a_tap_over_a_bar_button_is_not_in_the_routed_chain`.

    New, each with its mutation in § *Testing`:

    - **`every_tab_index_names_the_page_at_that_index_of_page_all`** — **the complement assertion, and the one that kills 24.1's dropped row one level down.** It walks **`0 .. Page::ALL.len()`** and asserts `bar.tab_id_of`/`bar.tab_ids`/`Page::ALL` agree at every index, **and asserts `self.bar.tab_ids().len() == Page::ALL.len()`** — so a tab that was never added, and a `Page` that was never registered, are both visible. **A membership assertion passes for both; this one cannot.**
    - `the_demo_bar_has_one_tab_per_page_and_no_tab_beyond_the_six` — the other direction, split out **so each name is about what it catches.**
    - **`the_tab_bar_selection_and_the_screens_current_name_never_disagree`** — **the whole mitigation for keeping two representations of one fact, and a gate rather than a nicety.** It walks **all six pages** and **both switch paths** (the `pending_page` drain, and the `--tab=` constructor path) and asserts `self.bar.selected().map(|id| id.index())` is the index of `self.page` in `Page::ALL` **and** that `self.screens.current_name() == self.page.name()`. **Task 42's
      `the_demo_page_field_and_the_librarys_current_screen_never_disagree` is the
      precedent and the shape.**
    - `the_bar_still_records_the_command_sequence_it_recorded_before` — **the
      pixel criterion's mechanism at the command level, and it is new**: a
      `PaintState` recording of the whole frame on each of the six pages, compared
      against the same frame's recording with the selection moved to the page on
      show. **It asserts the command count, the kinds in order, the rects and the
      colours**, so a `TabBar` that records its background in the wrong place, or
      drops the last tab, or draws a tab twice, fails here. **A capture cannot see
      a command that was never recorded; this can.**
    - `a_page_switch_aims_exactly_the_two_tabs_it_moved_between` — the `on_change`
      counters on all four animated properties of all six buttons, across one
      `show_page`, asserting **exactly two buttons moved `background` and
      `foreground` and the other four moved nothing.**
    - `a_tab_on_the_page_already_on_show_is_aimed_by_its_press_and_its_release` —
      **24.3's major, re-asserted on the demo's own bar**, with no selection change
      in it.
    - `the_focus_ring_follows_the_focus_record_and_the_selection_follows_the_screen` —
      the two records are separate, on `activatable`'s argument: `bar.focus(...)`
      writes the ring and `select_name` writes the fill, **and a test that moved
      both would pass on the first frame and be wrong by the next.**
    - `the_two_tab_palettes_differ_in_all_three_colours` — § *Testing* carries why
      this is a test and not a comment.

    **Moved to the kept list, and recorded as a move rather than left ambiguous:**
    `every_dialog_button_is_still_in_the_route_chain` and
    `a_tap_over_a_bar_button_is_not_in_the_routed_chain` are **task 42.2's and
    24.3's**, not this task's. 43.2 **keeps both green and neither is rewritten**,
    because both are about `input::route` chains and neither names `Demo::tabs`.

27. **`DEMO_APPLICATION.md` § *Library gaps*, row 7, gains a dated note** in the
    form § *Corrections to the second gap table* prescribes — **not a deletion and
    not a bare "closed"**:
    - **`TASK_UI_PRIM_43` closes the row's claim about `ui_core`**: `TabBar` is a
      row of buttons with a selected one and an active-state indication, and
      `Button` carries `selected` — **so the row's *"active-state indication" half
      has no widget behind it"* is no longer true and the row's own sentence is
      corrected in place**, which is what § *Corrections to the second gap table*
      was written about.
    - **What is NOT delivered, and the row keeps saying it:** *"icon+label layout"*
      — **gap #4, `TASK_UI_PRIM_44`, and it stays open.** A bar of labelled
      buttons is what this task delivers and the row's *"a dedicated widget with
      active-state indication **and icon+label layout**"* has one of its two halves
      closed and not the other. **The note says which**, because a note that says
      "closed" where the row says two things is the defect the table exists to
      catch.
    - **The row's own sentence about 24.3 is corrected in place rather than left
      to age** — *"which is the prescription this row states and **not a closure of
      it**"* — which is now true as well as accurate.
    - **One consumer exists** (the gallery), and **the row is not closed on the
      strength of the library type alone** — the same two facts task 42's row-3 note
      names.
    - **The four places § *Relationship to task 24* lists as still recording the
      withdrawn 2026-10-03 decision are updated**: **three of them were amended by
      task 42** and requirement 20 of this task closes **the fourth**, the source
      comment on the demo's tab bar, **which is the same one 42's requirement 20
      names** — so if 42 has landed this requirement is a **check that the comment
      says gap #7 is closed by this task**, and if it has not, **this requirement
      amends it and 42's becomes the check.** The handoff says which of the two
      happened. **And the table's own claim that *"none of them is amended by this
      pass"* is superseded in the same edit**, on task 42's reasoning.
    - **`Severity` stays `Low` and `Blocks` keeps "Bottom dock implementation"** —
      the row's severity was never raised by the withdrawal and a task that
      implements a Low row does not lower it.

28. **`doc/ui/IMPLEMENTATION_STATE.md` gains one entry**, carrying: the two halves
    and which file delivered each; **`selected` on `Button` with its one-question
    rule and the three things it does not answer**; **the second palette and the
    pixel-identity reason for it**; **the `TabBar` shape and the private `Tab` row**;
    **the composition with `Screens` and the mirror's test by name**; **the seven
    functions deleted from the demo and the three constants**; **the two-hop click
    mechanism and `route`'s borrow hazard as the reason**; **the arrow-key path and
    that it has no demo consumer**; **`sync`'s convergence and the per-frame-aim
    guard**; **`LongPress` and `Swipe` still unconsumed**; **that `Grid` is still
    unimplemented and L7's missing cross-axis gap is still missing**; and **the
    six pages' frame rates, with the script's own line pasted.**

29. **The suite, the capture and the frame rate are all produced.** From `ui/`: the
    six commands of requirement 16. Then **the six-page before/after capture**,
    with the commands of `IMPLEMENTATION_STATE.md`
    § *Verifying a change that draws — the capture method* verbatim: window id
    **re-read at the time of each capture** with `xwininfo -root -tree` (a root
    capture, and `ffmpeg x11grab`, return black for a GL window), `pgrep -a -x
    ui_demo` in the same call as each `magick import -window <id>`, then `magick
    compare -metric AE before.png after.png null:` per page. Then **the frame rate
    on all six pages**.

## Testing

**Two lessons are the reason this section is shaped the way it is, and both are
this file's own history rather than a general principle.**

1. **24.3's one major was a mechanism with no test, and it hid in the ordinary
   gesture.** `release_tab`'s `animate_to_state` was load-bearing **only** on press
   and release on the button of the page **already on show**, because every fixture
   reached the bar through a page *switch*, which re-aims everything. A mutation
   sweep of the release aim passed, because the sweep's fixtures never took the
   path the aim was for. **So
   `the_selected_tab_is_aimed_on_a_press_and_on_a_release_that_changed_no_selection`
   drives press, release and arrival with no `select` call anywhere in it** — and
   43.2's `a_tab_on_the_page_already_on_show_is_aimed_by_its_press_and_its_release`
   is the same test on the demo's own bar. **And the structural answer is
   requirement 12's: the aim is inside the widget, in one function, with nothing
   left for a caller to forget.**
2. **The rule *"a sweep of a mechanism's call sites is not a sweep of the data
   it is built from"*, and its complement: a membership assertion passes for a row
   that was never added.** 24.1's round-3 reviewer deleted one line of
   `Demo::new` — the loop that adds seven text labels to the page table — and the
   whole suite stayed green while the text column drew on the wrong page. **This
   task's data is a list (the labels), a remembered state (the `aimed` triples) and
   an index mapping (`Page::ALL`), and all three are swept below — the label list
   from both sides, the triples by convergence, and the index mapping by its
   complement `0 .. Page::ALL.len()`.**

**And the third lesson is *"for every control drawn, name the gesture
that operates it"* — a tab is drawn and, on this host, cannot be pressed, so the
hit test is asserted against the crate's own `input::hit_test` and the painted rect
rather than against a pointer event that has never arrived.**

| Test | Mutation it kills |
|---|---|
| `a_button_nobody_selected_draws_exactly_what_it_drew_before_selected_existed` | **`selected_palette` defaulting to anything other than `palette`.** A different default silently repaints every button in the crate, and no pixel assertion in the demo would separate it from a theme change |
| `the_second_palette_defaults_to_the_unselected_one` | The same default, read off the widget rather than off the screen — **so the first test's failure is diagnosable rather than merely visible** |
| `a_selected_button_draws_its_selected_palette_and_an_unselected_one_draws_its_own` | `style()` consulting one palette for both, or the two accessors swapped. **The control half — with `selected` false on both the two are equal — is what stops it passing for two palettes that always differ** |
| **`selecting_a_button_changes_nothing_it_paints_until_it_is_aimed`** | **`paint_faded` consulting `selected` directly.** That is the mutation a later edit is most likely to write, it is invisible in a capture of an already-settled bar, and it would break the "aim once, tick per frame" contract by moving a colour with no clock |
| `a_selected_button_ring_colour_comes_from_the_selected_palette` | **Requirement 4's one changed line being left undone** — `self.palette.ring` still in place. **The demo's unselected ring is `Primary` and its selected one is `OnPrimary`, so on the dark theme this is a visible difference and nothing else in `ui_core` would see it** |
| `a_selected_button_still_activates_and_still_draws_its_ring` | `selected` entering `may_activate`. **A selected button that refuses to fire is a control the one thing a driver knows about stops answering** |
| `may_activate_is_selected_independent_in_all_four_cases` | The same, by table rather than by example, with the `activatable` pair as the control |
| `selected_is_not_a_button_state_and_the_primary_state_still_describes_the_background` | **Adding a `Selected` variant to `ButtonState`'s precedence chain**, in any of its five positions. Every position is wrong and § *One bit, one question* says why |
| **`a_button_that_is_not_a_tab_of_this_bar_is_never_aimed_and_never_painted_by_it`** | **24.1's dropped row, one level down.** A fifth tab built and never pushed, or pushed to the wrong bar: `tab_id_of`, `tab_rect`, `tab_at` and the recorded stream all miss it. **A membership assertion passes for a tab that was never added; this one cannot** |
| `every_label_the_bar_was_given_names_exactly_one_tab` and `every_tab_button_is_a_child_of_the_bar_in_the_order_the_labels_came_in` | **The data half of the same mutation**: a label dropped from the fixture, or an attachment refused by `node::attach` and its answer discarded, both of which leave a button in the arena that belongs to nothing |
| **`selecting_the_current_tab_is_a_total_no_op_including_the_aim`** | **`select` that short-circuits `on_select` but still calls `animate_to_state`.** The return value cannot see it, a capture cannot see it, and **in the composed demo a click reaches the selection twice** — so this is the mutation the composition is actually exposed to. The `Property::on_change` counter on `background` is the only instrument that sees it |
| **`the_selected_tab_is_aimed_on_a_press_and_on_a_release_that_changed_no_selection`** | **24.3's one major.** Press and release with **no selection change anywhere in the test**; asserts the scale returns to `1.0` exactly. Kills moving the release aim out of `release`, into `sync`, or away |
| `a_click_through_the_buttons_own_event_selects_the_tab_and_reports_it_on_the_next_sync` | **The two-hop being bypassed** — a click reaching `select` directly. It would work in the test and panic in `input::route`'s double borrow on a real dispatch, which is the reason the two hops exist |
| **`an_aim_is_not_repeated_on_a_second_sync_that_changes_nothing`** | **The per-frame aim.** One hundred `sync` calls after one `select` must move the counters exactly four times. A `sync` that re-aims unconditionally restarts every clock every frame and the colour creeps toward its target for ever and never arrives — `sync_toggle_state`'s argument, and the cheapest mutation in this file to ship |
| `a_selection_change_aims_exactly_two_tabs_and_touches_no_other_flag` | `sync` aiming all six, or aiming the incoming one and not the outgoing one, or **aiming `scale` and `opacity` for a palette swap that does not move them** |
| `the_focus_record_is_not_part_of_the_aimed_triple` | **`focused` in the triple.** It would cost six aims a frame for no visible change, because `Style::ring_width` is the one appearance `animate_to_state` never animates — and the frame-rate criterion would catch it only as a slow drift |
| `the_bar_follows_a_new_palette_and_a_new_selected_palette_on_the_next_sync` | **The two setters being wired to the same slot**, and `set_palette` moving a *selected* tab. The bar would render and no rect assertion would see it |
| `a_bar_with_no_tabs_is_a_box_of_its_padding_and_says_so` | **`n - 1` underflow in `size`** for an empty bar — which is the edge a saturating subtraction would make look fine |
| **`tab_at_tab_rect_and_hit_test_read_the_same_box`** | **The 2026-10-01 rule.** A bar that draws four tabs and hit-tests three, or tests a box `arrange_flex` never produced. **Four points per tab — centre and each edge's last pixel — against `input::hit_test`, the crate's own mechanism, rather than against a pointer event this host has never delivered** |
| `the_bar_paints_its_background_and_then_its_tabs_in_the_bar_order` | The command-position mutation, by recorded position rather than by count. **A capture sees the result; this sees the cause** |
| `left_and_right_and_home_and_end_walk_the_selection_and_wrap_at_both_ends` | The wrap arithmetic, on a **four**-tab bar so a two-tab bar cannot make it pass |
| **`an_arithmetic_key_with_no_tab_focused_is_left_for_the_focused_control`** | Arrows consumed unconditionally, which would take left/right away from every control behind the bar |
| **`the_bar_consumes_nothing_but_the_four_navigation_keys`** | **Row `L4`'s claim, enforced against this widget from its own side**: `LongPress` and `Swipe` asserted unconsumed by name, so a later task that consumes them here breaks a test deliberately rather than drifting past a claim in a document |
| **`every_tab_index_names_the_page_at_that_index_of_page_all`** | **The data this task leaves in the demo: the `Page` ↔ index mapping.** A `TabBar` and a `Page::ALL` that disagree at one index put the right label on the wrong page, and `the_demo_page_field_and_the_librarys_current_screen_never_disagree` alone cannot see it — that one compares the two mirrors, and both mirrors can be consistently wrong together |
| **`the_tab_bar_selection_and_the_screens_current_name_never_disagree`** | **Either half of the mirror being written without the other** — the whole mitigation for keeping two representations of one fact, on task 42's precedent. **Walked over all six names and both switch paths** |
| **`the_bar_still_records_the_command_sequence_it_recorded_before`** | **The pixel criterion's mechanism.** Command count, kinds in order, rects and colours, per page, against a recording with the selection moved. **A bar that records its background after its tabs, drops a tab, or draws one twice fails here and nowhere else** |
| `a_page_switch_aims_exactly_the_two_tabs_it_moved_between` | `sync` aiming all six on a switch, which is what the demo's `changed: &[Page]` list existed to prevent — **and that list is deleted, so the property now lives in the widget and needs a test here** |
| `a_tab_on_the_page_already_on_show_is_aimed_by_its_press_and_its_release` | **24.3's major, re-asserted on the demo**, with no selection change in it |
| `the_focus_ring_follows_the_focus_record_and_the_selection_follows_the_screen` | The two records being moved together. **They answer two different questions and a test that moved both would pass on the first frame and be wrong by the next** |
| `the_two_tab_palettes_differ_in_all_three_colours` | The selected/unselected pair collapsing in one of the three colours — **which is what makes "selected is legible at a glance" a fact rather than a hope, and no rect assertion can see it** |

## Acceptance Criteria

- [ ] **`Button::selected` exists, is `pub selected: Property<bool>`, defaults to
      `false`, and is declared between `activatable` and `scale`.**
      `awk '/^pub struct Button/,/^}/' ui/src/ui_core/src/widgets/button.rs` shows
      **seventeen** `pub` fields in the order requirement 1 gives, and
      `grep -c 'pub selected' ui/src/ui_core/src/widgets/button.rs` returns **1**.
      **The default is the load-bearing half** and
      `a_button_nobody_selected_draws_exactly_what_it_drew_before_selected_existed`
      plus `the_second_palette_defaults_to_the_unselected_one` are the criteria that
      carry it. **Mutation evidence in the handoff:** set `selected_palette` to a
      third, distinct palette in `Button::new` and watch the first test fail with a
      command-vector mismatch; restore it and watch it pass

- [ ] **`selected` answers one question and reaches the screen on two paths.**
      `grep -n 'selected' ui/src/ui_core/src/widgets/button.rs` shows the field, the
      `base_palette` read in `style()`, the `base_palette` read in `paint_faded`, the
      two setters, the `Button::new` initialiser, **`may_activate` nowhere**, and
      **`ButtonState` nowhere**. The doc comment carries the four paragraphs of
      requirement 1, **including the sentence that it is not a `ButtonState`
      variant and `ButtonState`'s own *"the states overlap"* text quoted beside it**
      — because a doc comment asserting the opposite of the code beside it is the
      defect `DEMO_APPLICATION.md` § *Corrections to the second gap table* records
      three times

- [ ] **The ring's *width* is still a function of `focused` alone, and the ring's
      *colour* follows the base palette.** `a_selected_button_still_activates_and_
      still_draws_its_ring` and `a_selected_button_ring_colour_comes_from_the_
      selected_palette` are present and green, **and the four-quadrant matrix of
      `IMPLEMENTATION_STATE.md`'s § *Closed: `Button::focused` no longer answers two
      questions* still holds** — the same two rows, plus `selected`'s two columns.
      **The mechanism is named: `disabled` remains the only thing that zeroes
      `ring_width`, which is unchanged code and is asserted by
      `the_disabled_button_draws_no_ring_and_its_opacity_is_the_disabled_one` in
      `button.rs` staying green**

- [ ] **`Button::paint_faded` changed on exactly one line.**
      `git diff --stat ui/src/ui_core/src/widgets/button.rs` plus the diff itself
      shows **one** changed line inside `paint_faded` (the ring's colour) and **two**
      inside `style()`, and **nothing** in the flag derivations between them —
      `HOVER_LIGHTEN`, `PRESS_DARKEN`, `PRESSED_SCALE`, `DISABLED_DESATURATE`,
      `DISABLED_OPACITY` and the `disabled` block's `scale = 1.0` are untouched.
      **`Button::animate_to_state` is untouched**, so a selection change is carried
      by the same four properties and the same clock as a hover

- [ ] **`pub mod tab_bar` exists and is the sixteenth `pub mod` in
      `widgets/mod.rs`.** `grep -c 'pub mod' ui/src/ui_core/src/widgets/mod.rs`
      returns **16** against **15** at the time of writing, and
      `grep -n 'pub mod tab_bar' ui/src/ui_core/src/widgets/mod.rs` shows the line
      in alphabetical position. **`The counts are a delta against the count measured
      when this task starts, not an absolute`** — tasks 34–40 are specified and not
      started, and if any of them landed first the absolute is 16 plus however many
      added a widget. **The handoff pastes both `grep -c` results, before and
      after**, because `task-sequence.md` § *Gates* forbids evidence by assertion.
      `awk '/^pub struct TabBar/,/^}/' ui/src/ui_core/src/widgets/tab_bar.rs` shows
      **`bar`, `on_select`, `requested`, `selected`, `motion`, `tabs`** and nothing
      else, and **`grep -c 'pub struct Tab' ui/src/ui_core/src/widgets/tab_bar.rs`
      returns 0** — the row is private on purpose

- [ ] **`TabBar`'s public surface is the twenty-six methods of requirement 12 and
      nothing else**, `grep -c '    pub fn' ui/src/ui_core/src/widgets/tab_bar.rs`
      returns **27** — **twenty-six on `impl TabBar` and `TabId`'s one `index`
      accessor** — and the handoff lists each against the caller or the test that
      needs it. **There is no `len`, no `contains`, no `iter`, no `Tab` and no
      `Page`** — `tab_ids().len()` is the count, and § *`TabBar`'s shape* carries the
      `Page` collision argument by citation to task 42's ruling

- [ ] **Nothing on the widget takes a `Rect`.**
      `grep -c 'rect: Rect' ui/src/ui_core/src/widgets/tab_bar.rs` returns **0** in
      the method signatures, and the module doc states the reason — **the bar's and
      every tab's geometry is the node's own cached rect, read by `bar_rect`,
      `tab_rect`, `tab_at` and `paint`, so hit testing and drawing cannot read two
      numbers.** `tab_at_tab_rect_and_hit_test_read_the_same_box` is the test, and
      it is **four points per tab against `input::hit_test`**, not against a pointer
      event — **which this host has never delivered, and no criterion here asks for
      one**

- [ ] **Selection changes are one write path, idempotent, and reported once.**
      `selecting_a_tab_moves_the_selection_and_reports_it_once`,
      **`selecting_the_current_tab_is_a_total_no_op_including_the_aim`**,
      `selecting_past_the_last_tab_changes_nothing_and_says_so` and
      `select_name_resolves_a_label_and_refuses_one_that_is_not_a_tab` are present
      and green. **`TabBar::select` is the only writer of `Cell::get` on the
      selection**, `on_select` fires **only on a move**, and
      `a_click_through_the_buttons_own_event_selects_the_tab_and_reports_it_on_the
      _next_sync` proves the **two-hop**: after a `Tap` through `Button::on_event`,
      `selected()` is still `None` and the counter still reads zero until `sync`
      runs. **Mutation evidence in the handoff:** make `select` skip the
      already-selected early return and watch the `on_change` counter in the
      idempotence test move; restore it and watch it pass

- [ ] **The aim is inside the widget, is guarded by what it last aimed for, and
      never repeats a settled aim.** `grep -n 'animate_to_state'
      ui/src/ui_core/src/widgets/tab_bar.rs` returns **exactly the one line inside
      `sync`**, and
      `grep -n 'animate_to_state' ui/src/ui_demo/src/main.rs` returns **no line** —
      `Demo::aim_tab_buttons` is gone with its call sites.
      `an_aim_is_not_repeated_on_a_second_sync_that_changes_nothing` runs **one
      hundred** `sync` calls and asserts the counters moved **exactly four times**;
      `a_selection_change_aims_exactly_two_tabs_and_touches_no_other_flag` and
      `the_focus_record_is_not_part_of_the_aimed_triple` are present and green.
      **The mechanism is `sync`'s convergence, and it is the recorded reason a
      per-frame `sync` is not a per-frame aim** — `sync_toggle_state`'s argument,
      restated for the widget that now owns it

- [ ] **24.3's one major cannot return, on the widget or on the demo.**
      **`the_selected_tab_is_aimed_on_a_press_and_on_a_release_that_changed_no_
      selection`** drives press, release and arrival **with no `select` call anywhere
      in it** and asserts `scale == 1.0` exactly, and
      `a_tab_on_the_page_already_on_show_is_aimed_by_its_press_and_its_release` is
      the same test on the demo's own bar. **Mutation evidence in the handoff:**
      delete the `animate_to_state` call from `TabBar::release`, watch the first fail
      with a button stuck at `0.95`, restore it and watch it pass. **The sweep that
      missed this is recorded** — 24.3's fixtures all reached the bar through a page
      *switch*, which re-aims everything

- [ ] **The bar consumes nothing but six keys, and only while a tab is focused.**
      `the_bar_consumes_nothing_but_the_four_navigation_keys` is a table over **all
      nine** `InputEventKind` variants and asserts `LongPress` and `Swipe` are
      **not consumed, by name**; `an_arithmetic_key_with_no_tab_focused_is_left_for
      _the_focused_control` and
      `left_and_right_and_home_and_end_walk_the_selection_and_wrap_at_both_ends` are
      present. **`grep -c 'pub fn on_event' ui/src/ui_core/src/` goes from 8 to 9**
      **as a delta measured before and after, not as an absolute** — tasks 34–40
      are specified and not started. **`LongPress` and `Swipe` remain unconsumed by
      anything in the crate**, so row `L4` of `DEMO_APPLICATION.md`
      § *Gaps this layout exposes in `ui_core`* stays true and **this task does not
      amend it**; a later task that consumes them here must break the table
      deliberately

- [ ] **The layout is a `row` with one main-axis gap, and what it cannot do is on
      the record.** `grep -n 'LayoutMode::row\|LayoutMode::column\|LayoutMode::Grid\|FlexDirection'
      ui/src/ui_core/src/widgets/tab_bar.rs` shows **one `LayoutMode::row()`** in
      `new` and **no `column`, no `Grid` and no `FlexDirection`** outside the prose
      that says it needs none. `LayoutMode::Grid` is **still** `=> Vec::new()` with
      `columns` never read — **gap `L3`, `TASK_UI_PRIM_52`, untouched** — and
      **`FlexConfig` still has no cross-axis gap**, which is gap `L7`; the module doc
      says so in one paragraph and names what a bar whose tabs do not fit does:
      **overflow, clipped, never wrapped.** The three constants are asserted
      positive, finite, and `TAB_HEIGHT ≥ 44.0`, **and
      `a_bar_with_no_tabs_is_a_box_of_its_padding_and_says_so` covers the empty
      bar**

- [ ] **`cargo test --all-features` is green against the 1894 baseline**, and the
      handoff **lists every new test by name**: the **8** in `button.rs`, the **26**
      in `tab_bar.rs`, and the demo's **8** new with the kept set enumerated by name
      in requirement 26. Per binary: `ui_core` **1450 + 34 = 1484** or more,
      `ui_demo` **224 + 8 = 232** or more, doctests **220 + 3 = 223** or more —
      **so 1894 + 45 = 1939 or more.** **No test was deleted, renamed away or
      weakened**, and the six tests that read `tab_palette(&theme, selected)` keep
      their names and their numeric assertions, reading `.0` / `.1` instead — which
      the handoff lists as **rewrites, not replacements**, with the reason.
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. **`cargo audit` is not installed on this host;
      that is recorded, not passed.** **`ui/Cargo.toml` and `ui/Cargo.lock` are
      unchanged** — per `AGENTS.md` the approved direct dependencies remain
      `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, **and `TabBar` needs none**:
      a `Vec`, a `Cell`, a `Property`, a `Button` and a `Container` are the whole of
      it

- [ ] **The data is swept, not only the call sites.** Four named tests exist for
      this and four mutations are in the handoff:
      - `a_button_that_is_not_a_tab_of_this_bar_is_never_aimed_and_never_painted_by_it`
        — **the complement assertion**, killed by building a fifth tab and never
        pushing it
      - `every_label_the_bar_was_given_names_exactly_one_tab` and
        `every_tab_button_is_a_child_of_the_bar_in_the_order_the_labels_came_in` —
        **the label list from both sides**
      - `every_tab_index_names_the_page_at_that_index_of_page_all` — **the
        `Page` ↔ index mapping over `0 .. Page::ALL.len()`**, killed by dropping one
        page from the bar's labels
      - `the_focus_record_is_not_part_of_the_aimed_triple` — **the remembered
        `aimed` state**, killed by putting `focused` in the triple

      **The lesson being obeyed is *"a sweep of a mechanism's call
      sites is not a sweep of the data it is built from"*, plus its complement: a
      membership assertion passes for a row that was never added.** 24.1's round-3
      reviewer deleted one line of `Demo::new` and the whole suite stayed green

- [ ] **The six gallery pages are pixel-identical outside `y ≥ 680`, and the
      mechanism is stated rather than hoped for.** `Page::ALL`'s six names, release
      build, captured **before and after** with the commands of
      `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
      method* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab`, return black for
      a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, then `magick compare -metric AE before.png
      after.png null:` per page. **On all six the criterion is AE 0 outside
      `y ≥ 680`**, every differing pixel inside the frame-rate readout's band, which
      `IMPLEMENTATION_STATE.md` § *Task 24.1 — what it decided, and what it found*
      records as the one thing two captures of an unchanged frame differ in.

      **The mechanism is four facts, and the second is the reason `Button` grew a
      second palette rather than a derivation:**
      1. **No rect moves.** The bar's own node keeps its `Constraints::tight(Size
         { width: WINDOW.width, height: TAB_BAR_HEIGHT })` and its
         `Offset::new(0.0, 0.0)`, and `TabBar::measure` writes **each tab's** box
         through `Button::content_size` with the same `padding_h` and `font_size`
         the demo set — **the same six numbers the demo wrote yesterday**. The
         `Container` mode, padding and `FlexConfig` are the same three values, so
         `arrange_flex` places the six at the same x's.
      2. **Every colour is the colour the demo drew.** The unselected tab is
         `Border`/`Text` with a `Primary` ring and the selected one is
         `Palette::from_theme` — **`Primary`/`OnPrimary`/`OnPrimary` — which is what
         `tab_palette(&theme, true)` already returned.** The second palette is a
         *carrier* for two existing answers, not a new appearance; **that is
         precisely why it was chosen over an alpha or a lighten**, either of which
         would have drawn six colours this repository does not draw today.
      3. **The recorded command sequence is the same, and that is a test rather than
         a hope.** `the_bar_still_records_the_command_sequence_it_recorded_before`
         compares count, kinds in order, rects and colours on **each of the six
         pages**, `the_bar_paints_its_background_and_then_its_tabs_in_the_bar_order`
         pins the order inside the bar, and the rect-level tests keep their names
         and every assertion: `every_page_places_every_rect_where_the_gallery_
         placed_it`, `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`,
         `placed_handles`, `page_rects`.
      4. **What did change is invisible to a capture by construction**, because it
         is entirely about which node *reaches* the caller: `TabBar::tab_at` reads
         the same cached rect the demo's `Demo::tab_at` read, so a tap aims at the
         same box, and the six buttons are reached through `TabBar::tab` rather than
         through a `Demo::tabs` search. **A press produces no pixel change and that
         is the correct outcome**, so the criterion is the strong one

- [ ] **The frame rate is measured on all six pages and reported**, with the
      script's own line pasted rather than the number expected:
      `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of the
      six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `IMPLEMENTATION_STATE.md` § *Current position* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. Every page above the floor of **55** and **inside the recorded
      61.1–63.9 band**.

      **And the handoff states what the number is expected to be and why, rather
      than reporting a number and letting it be read as luck.** Per frame, this
      task **replaces** `for tab in &self.tabs { tab.button.tick(delta) }` with
      `self.bar.tick(delta)` — the same six `AnimationClock::tick` calls — and
      **adds** one `sync`, which is **six tuple comparisons and, on a settled bar,
      six property reads that change nothing**. **A `Button::selected` write per
      changed tab is two per selection change and none per frame.** So the
      expected result is **no measurable change on any page**, and a page outside
      the band is **a finding rather than noise**. **The one honest risk is the
      `on_change` counters in the tests** — a counter per property per tab is test
      code, and test code is not in the release binary

- [ ] **What the handoff does not claim, in those words.** It states that **no
      pointer event has ever been observed reaching this window** on this host —
      `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
      method* records the drag, the two presses on task 12's button, the counter and
      the `AE = 0`, and `XQueryPointer` reporting window `0x0` — **and therefore
      that no acceptance criterion here is verified by a pointer-driven capture,
      and none asks for one.** The bar's operability is verified **by test against
      `input::hit_test` and the painted rect**, the six pages' route chains are
      verified **through `input::route`**, and the selection change is verified
      **by test through `Button::on_event` and `TabBar::sync`**. **The two together
      are not evidence that a finger switches a page**, and the
      `IMPLEMENTATION_STATE.md` entry says so

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **no change** to `ui/src/ui_core/src/layout.rs`,
      `property.rs`, `paint.rs`, `batch.rs`, `render.rs`, `input.rs`, `animation.rs`
      or `theme.rs`. **`LayoutMode::Grid` is still unimplemented** and `Flex.wrap`
      is still discarded — this task depends on neither, because a bar is a `row`
      with N declared boxes. **No `LayoutMode` import, no `paint` variant and no
      uniform** is added. **No new dependency** (`ui/Cargo.toml` and
      `ui/Cargo.lock` unchanged), **no `unsafe`** — `grep -c unsafe
      ui/src/ui_core/src/widgets/tab_bar.rs` is **0** and `button.rs` gains none —
      **no `unwrap`, no `expect`, no `panic!`, no `unimplemented!`, no `todo!`** in
      production code, **no `Box<dyn Trait>`**, nothing newer than
      `rust-version = "1.85"`'s stdlib, **no `println!`** in library code, and
      **no doc comment in any changed file asserting the opposite of the code
      beside it**, which is the defect `DEMO_APPLICATION.md`
      § *Corrections to the second gap table* records three times

- [ ] **`DEMO_APPLICATION.md` § *Library gaps* row 7 is amended, dated, and says
      exactly what is and is not closed.** It names `TASK_UI_PRIM_43`,
      `ui_core::widgets::tab_bar::TabBar` and `Button::selected`; **corrects the
      row's own *"no `selected`"* and *"no widget behind it"* sentences in place**,
      because they are now false; **corrects the row's sentence about
      `TASK_UI_PRIM_24.3` in place**, because *"the prescription this row states
      and not a closure of it"* is now true as well as accurate; **states that the
      row's *"icon+label layout"* half is NOT delivered** and names **gap #4,
      `TASK_UI_PRIM_44`**, as the work that delivers it; **keeps `Severity: Low` and
      the `Blocks` entry**; and **says that one consumer exists** rather than
      closing the row on the strength of the library type alone. **The row is not
      deleted** — it records what a library mechanism is and what a demo affordance
      is not, and those are different sentences. **And § *Relationship to task 24*'s
      four-place table is closed out**: the fourth row's source comment is amended
      or checked, whichever of 42 and 43 got there first, **and the handoff says
      which**

- [ ] **The decisions are written down where the next agent finds them.**
      `tab_bar.rs`'s module doc carries requirement 6's decision and
      § *`TabBar`'s shape*, § *One bit, one question*, § *Selection semantics* and
      § *Layout* in full — including **that no `Rect` parameter appears on the
      widget and why**, **that `Tab` is private and `Page` is not introduced, with
      task 42's ruling cited**, **that the arrows have no demo consumer and what
      adding one costs**, and **that the bar does not move focus and who does**.
      `button.rs`, `ui/src/ui_demo/src/main.rs`,
      `doc/ui/DEMO_APPLICATION.md` and `doc/ui/IMPLEMENTATION_STATE.md` each carry
      their half of requirements 15 and 28, **including the two-hop click and
      `input::route`'s borrow hazard as its reason**, **`sync`'s convergence and the
      per-frame-aim guard**, **the seven deleted demo functions and the three deleted
      constants by name**, **the mirror with `Screens` and the test that pins it**,
      and **the honest limits** — the arrows have no demo consumer, the row's icon
      half is gap #4's, `L7`'s missing cross-axis gap is still missing, and
      `LongPress` and `Swipe` are still consumed by nothing. `IMPLEMENTATION_STATE.md`
      is not a source of evidence (`task-sequence.md` § *State*); it points at the
      code

- [ ] **The split was honoured.** The handoff for 43.1 and the handoff for 43.2 are
      separate, **43.1 was briefed against a tree with no `nav`** — which is the test
      that the split is real, since requirements 1–15 name no `nav` symbol — and
      **43.2 was briefed against task 42's landed code** and is a stop condition
      until 42 has landed, because it reads `Screens::current_name` and the six
      screen hosts. **The review covers the integrated result**, because integration
      can break what the parts proved. **43.2 was not split further**, and the reason
      is stated rather than asserted: its two halves both edit
      `ui/src/ui_demo/src/main.rs`, and `.ai/protocols/subagents.md` § *Splitting*
      rule 1 is explicit that two subagents on one file is a failed split

- [ ] **No acceptance criterion is waived, and none asks for an instrument this host
      cannot produce.** No criterion here requires a pointer event, a GL readback, a
      display, a network, a filesystem or the wall clock. **The two things a capture
      cannot see — a command that was never recorded, and a frame-cost regression —
      are covered by `the_bar_still_records_the_command_sequence_it_recorded_before`
      and by `fps-check.sh` respectively**, which is the pairing
      *a still screenshot of a 4 fps application looks exactly
      like a 60 fps one* exists to demand. **And 43.1 launches `ui_demo` zero
      times**, so the one instrument this file does not use has nothing to report on
      that sub-task

## Out of Scope

- **No icons in tabs, and no icon+label layout.** That is **gap #4**,
  `TASK_UI_PRIM_44`, and **row 7's own *"icon+label layout"* half stays open** —
  requirement 27 says so in the row rather than leaving it implied. A `TabBar` whose
  tabs are `Button`s has exactly one primitive per tab, and a primitive that draws a
  glyph does not exist yet: `Polygon` is convex-only with no bezier (**L10**), and
  `Image` needs an atlas this task does not touch. **The tab is a label, and the
  label is the whole of what is delivered**

- **No scrolling, no overflow handling, no wrapping and no ellipsis in a bar.**
  `Flex.wrap` is accepted and discarded by the same `..` that discards
  `LayoutMode::Grid { .. } => Vec::new()`, and `arrange_flex` **clips** an
  overflowing child — so a bar with more tabs than fit **overflows rather than
  wrapping**, and the module doc says so in one paragraph. **No `Scroll` is
  composed in**, because a horizontally scrolling bar is a second interaction
  (fling, momentum, snap, a visible scrollbar) and every one of those is
  *a drawn control with nothing behind it* waiting to happen.
  **The six-page gallery fits**, which is a fact about six labels and 1280 px and
  not a property the widget has. **A bar that does not fit is the caller's to
  solve**, and this file names the limit rather than half-building the answer

- **No `LongPress` and no `Swipe`.** `InputEventKind` has nine variants and this
  widget consumes one of them, under one condition.
  `the_bar_consumes_nothing_but_the_four_navigation_keys` asserts both by name,
  **so a later task that consumes them here must break that test deliberately
  rather than drift past a claim in a document**, and row `L4` of
  `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* **stays true and
  is not amended by this task.** The three consumers row `L4` names — dock edit
  mode, alert dismissal, card paging — are **discrete, threshold-based decisions**,
  which is a different task with a different decision

- **No re-parenting of the screen system, and no change to `Screens`.**
  `TASK_UI_PRIM_42` is a **prerequisite of 43.2 and is not touched by it.** 43.2
  adds `let _ = self.bar.select_name(page.name());` after 42's
  `screens.show(page.name())` and `screens.sync(&mut nodes)`, and nothing else.
  **`nav.rs` is not edited, `ScreenId` gains nothing, and the hit-test, focus and
  paint gates are unchanged** — including `LayoutState::hits`, which the demo's six
  hosts keep, and which is what keeps the bar reachable on `overlays`. **The
  composition is one line, in one function, on the value side of both objects**

- **No `Grid`, and no dependence on it.** `LayoutMode::Grid { .. }` is
  `Vec::new()` and `columns` is never read — gap **L3**, `TASK_UI_PRIM_52`,
  **Critical**, a separate task. A bar is a `row` with N declared boxes, and this
  task needs no grid, no `Flex.wrap` and no second row. **Nor is gap **L7** closed**:
  `FlexConfig` still has a main-axis `spacing` and **no cross-axis gap**, so the
  bar's own `Padding` is the only vertical inset

- **No transition, and no cross-fade of the selection change.** `Button`'s own clock
  carries it on `Motion::from_theme`'s `DurationFast` — **150 ms, not 300** — which
  is `tab_motion`'s recorded decision and the operator's open item 3 in
  `IMPLEMENTATION_STATE.md` § *What the operator still has to decide*. **This task
  does not change that number and does not settle the question.** A screen *switch*
  is a cut and stays one: nine `DrawCommand` variants, one `opacity`, no transform,
  no `u_model`, and gaps **#8** / **L2** keeping Critical

- **No focus movement.** `TabBar::on_event` moves the selection and reports it;
  **it does not move the focus ring**, because `Focus` is a caller value the widget
  cannot reach and coupling the two is a product decision with a visible
  consequence. **Wiring the arrow keys into the demo is one arm in
  `Demo::offer_to` plus one line in `Demo::set_focus`, and it is named here as the
  next thing to do** — not refused, and not done. **What a caller that wants them
  coupled adds is named too**: write `TabBar::focus(Some(id))` from its own
  `on_select`

- **No content node, no screen, no history, no transition and no router in
  `TabBar`.** It draws a row of buttons and knows which one is selected. **The
  content a tab shows is a `Screens` entry**, and § *`TabBar`'s shape* carries the
  reason: a widget that built trees would be a second owner of the caller's shape,
  which is `Screens::add`'s own reason for not attaching. **No `Page` type is
  introduced and `keyboard::Page` is not renamed** — task 42 settled that and this
  file applies the ruling rather than opening it again

- **No `TabBar::remove`, and no `TabId` reuse.** Tabs are fixed at `new`, so a
  `TabId` is a stable index within the bar's life, and a future `remove` needs a
  free list or a generational id like the arena's — **a design question this task
  does not answer, and `TabId`'s doc names the consequence of this one**

- **No change to any pipeline mechanism.** `DrawCommand`'s variants, `PaintState`,
  `Batcher`, `BatchKey`, `Segment`, `Renderer::begin_frame`, `Palette`, `Style`,
  `Motion`, `Easing`, `AnimationClock`, `Property`, `InputEventKind` and
  `InputEvent` — **`Style` and `Motion` are read and written here but not changed**,
  and `InputEventKind` gains no variant. **`DEPTH_BITS`, the depth policy, `Mat4`,
  both projections and every shader source are untouched by this task and checkable
  by `git diff --stat`** — and note that **34–41 and 52 are specified and not
  started**, so this task depends on **none** of them except 42, and only 43.2
  depends on 42

- **No change to `LayoutMode`, `FlexConfig`, `Padding`, `Constraints` or `Layout`.**
  The bar writes its three layout inputs through `Container::set_mode` (via
  `new`), `set_padding` and `set_flex_config`, **and `tab_bar.rs` imports nothing
  from `layout.rs` beyond `CrossAxisAlignment`, `FlexConfig`, `Offset`, `Padding`
  and `Size`.** `Padding` is a four-sided inset, there is **no margin, no
  `flex-shrink` and no `flex-basis`** in the crate, and this task needed none of
  them

- **No new dependency, and no `unsafe`.** Per `AGENTS.md` the approved direct
  dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; a tab-bar
  crate or a selection-state crate for a `Vec<Button>` and a `Cell<Option<usize>>`
  is a licence decision against GPLv3 that nobody has asked for. `grep -c unsafe
  ui/src/ui_core/src/widgets/tab_bar.rs` is **0** and `button.rs` gains none

- **No change to the demo's `Page`, its six names, or `CONTENT_TOP`.**
  `Page::ALL`, `Page::name`, `Page::from_name`, `Page::DEFAULT`, `TAB_BAR_HEIGHT`
  and `CONTENT_TOP` are untouched — `DEMO_APPLICATION.md`
  § *What a seventh page costs* enumerates everything one costs, **and this task
  touches none of it.** `CONTENT_TOP` staying a constant is what keeps every
  `*_ORIGIN` doc that says "what it clears" true, **and `the_demo_bar_has_one_tab_
  per_page_and_no_tab_beyond_the_six` is what says the bar gained no tab of its
  own**

- **No acceptance criterion is waived, and none requires a pointer event, a GL
  readback, a display, a network, a filesystem or the wall clock.**
  *An acceptance criterion that names an instrument which
  cannot produce the evidence is not met by producing the evidence another way* is
  the rule this file obeys by leaving such a criterion out and recording the gap:
  **the tab bar's operability by finger is not one of them, and the handoff says in
  one sentence that nothing here is evidence that it is**
