# TASK_UI_PRIM_43: `Button::selected`, and a `TabBar` That Owns Its Buttons

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Close gap **#7** of `doc/ui/DEMO_APPLICATION.md` § *Library gaps* by giving `ui_core` the two things the row names: a **`selected` state on `Button`**, and a **`TabBar`** widget that owns a row of buttons and knows which one is selected.
Sub-task 43.1 adds `Button::selected`, the second palette and `TabBar`; sub-task 43.2 rewires the demo to one `TabBar` and closes the row.

## Context

**Decision.** `Button` gains `pub selected: Property<bool>` and a second `Palette`; `ui_core::widgets::tab_bar::TabBar` owns its N `Button`s, a `Cell<Option<TabId>>` selection, one `Motion`, and the single aim that writes every button's appearance; neither `TabBar` nor `nav` depends on the other and the demo composes them by name. The second palette carries the demo's two existing appearances (`Primary`/`OnPrimary` against `Border`/`Text`), so no new colours are drawn.

This closes row `#7` of `doc/ui/DEMO_APPLICATION.md` § *Library gaps*. The operator's **2026-10-05** decision (§ *Operator decisions (2026-10-05)* item 2) **withdrew the 2026-10-03 decision** that left gaps `#3` and `#7` open; task 42 closed `#3`. This task sits on `TASK_UI_PRIM_42`, whose § *Out of Scope* names this file.

Not readable off the source: no `TabBar`/`tab_bar` identifier exists in `ui_core` (the bar is demo-local: `Demo::tabs: Vec<Tab>`, `struct Tab`, and `TAB_BAR_HEIGHT`/`TAB_BAR_PADDING`/`TAB_BUTTON_TALL`/`TAB_BUTTON_GAP`/`TAB_BUTTON_FONT`/`TAB_BUTTON_PADDING_H`); baseline **1894** (**1450**/**224**/**220**); release band **61.1–63.9 fps**, floor **55**; the demo cannot receive a pointer event on this host (`.ai/tools/README.md` § *Capturing a window*). `nav.rs` does not exist, and 43.1 needs no `nav` symbol; **43.2 reads `Screens::current_name` and the seven screen hosts and is a stop condition until 42 has landed**.

> **Amended 2026-10-09 by `TASK_UI_DEMO_01`**: the demo has **seven** tabs/pages/screen hosts/page names, but *six gallery pages* still means six; the three test names carrying a count are renamed to match.

## Requirements

Source requirement 28 (the `IMPLEMENTATION_STATE.md` record entry) is dropped per `.ai/workflows/task-sequence.md` § *State*; survivors are renumbered consecutively.

### Sub-task 43.1 — `ui_core`: `Button::selected`, the second palette, and `TabBar`

1. **`Button` gains `pub selected: Property<bool>` between `activatable` and `scale`**, default **`false`** in `Button::new`; doc states its question, that it is not `activatable`/`focused`, and its two screen paths (`style()` plus the ring colour).
2. **Private `selected_palette: Palette` init `Palette::default()`**; `#[must_use] pub fn selected_palette(&self) -> Palette`; `pub fn set_selected_palette(&mut self, palette: Palette)`; private `fn base_palette(&self) -> Palette`.
3. **`Button::style()` two lines:** `let base = self.base_palette();`, `background: base.background` / `foreground: base.foreground`; flag derivations unchanged.
4. **`Button::paint_faded` one line:** `with_opacity(self.base_palette().ring, opacity)`; doc paragraph.
5. **`Button::may_activate`/`ButtonState` not edited.**
6. **`button.rs` module doc paragraph.**
7. **New `ui/src/ui_core/src/widgets/tab_bar.rs`, `pub mod tab_bar;` in `widgets/mod.rs`** (sixteenth, between `slider` and `text_input`).
8. **`pub struct TabId(usize)` deriving `Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd`, `#[must_use] pub fn index(self) -> usize`**; no `Tab`/`Page`.
9. **`pub const TAB_SPACING: f32 = 8.0;`, `TAB_BAR_PADDING: f32 = 10.0;`, `TAB_HEIGHT: f32 = 44.0;`**; test finite/positive, `TAB_HEIGHT ≥ 44`; the demo's `TAB_BUTTON_TALL` named `keyboard::KEY_HEIGHT` (`52.0`) and is deleted by requirement 18.
10. **Private `struct Tab { button: Button, aimed: (bool, bool, bool) }`**; `focused` not in the triple.
11. **`#[derive(Debug)] pub struct TabBar`:** `pub bar: Container`, `pub on_select: Callback<TabId>`, private `requested: Property<Option<TabId>>`, `selected: Cell<Option<TabId>>`, `motion: Motion`, `tabs: Vec<Tab>`.
12. **`impl TabBar`, twenty-six methods:**
    ```rust
    pub fn new(nodes: &mut Arena<WidgetNode>, labels: impl IntoIterator<Item = impl Into<String>>) -> Self
    #[must_use] pub fn handle(&self) -> Handle
    #[must_use] pub fn tab_ids(&self) -> Vec<TabId>
    #[must_use] pub fn tab_id_of(&self, handle: Handle) -> Option<TabId>
    #[must_use] pub fn tab(&self, id: TabId) -> Option<&Button>
    #[must_use] pub fn label(&self, id: TabId) -> Option<&str>
    pub fn select(&self, id: TabId) -> bool
    pub fn select_name(&self, name: &str) -> bool
    #[must_use] pub fn selected(&self) -> Option<TabId>
    pub fn set_palette(&mut self, palette: Palette)
    pub fn set_selected_palette(&mut self, palette: Palette)
    pub fn set_motion(&mut self, motion: Motion)
    pub fn snap_to_state(&self)
    #[must_use] pub fn size(&self, advance: &dyn Fn(char) -> f32, line_height: f32) -> Size
    pub fn measure(&mut self, nodes: &mut Arena<WidgetNode>, advance: &dyn Fn(char) -> f32, line_height: f32) -> bool
    #[must_use] pub fn bar_rect(&self, nodes: &Arena<WidgetNode>) -> Option<Rect>
    #[must_use] pub fn tab_rect(&self, nodes: &Arena<WidgetNode>, id: TabId) -> Option<Rect>
    #[must_use] pub fn tab_at(&self, nodes: &Arena<WidgetNode>, position: Offset) -> Option<TabId>
    pub fn focus(&self, focused: Option<TabId>)
    pub fn hover(&self, over: Option<TabId>) -> bool
    pub fn press(&self, id: TabId) -> bool
    pub fn release(&self) -> bool
    pub fn sync(&self) -> bool
    #[must_use] pub fn tick(&self, delta: Duration) -> bool
    #[must_use] pub fn paint(&self, nodes: &Arena<WidgetNode>, advance: &dyn Fn(char) -> f32, line_height: f32) -> Vec<DrawCommand>
    pub fn on_event(&self, event: &mut InputEvent, focused: Option<TabId>) -> bool
    ```
    `new` builds the container, N buttons/attachments/click handlers, motion 150 ms / `Easing::EaseInOut`, selection `None`. `select` is the one write path (idempotent, writes `Button::selected` on outgoing and incoming, does not aim); `select_name` resolves `label`; `sync` is the one aim and only `animate_to_state` caller, draining `requested`. `measure` uses `Button::content_size` and `Constraints::tight(Size::new(content.width, TAB_HEIGHT))`; `size` = `sum(widths) + TAB_SPACING * (n - 1) + 2 * TAB_BAR_PADDING` by `2 * TAB_BAR_PADDING + TAB_HEIGHT`, zero-tab `Size::new(2 * TAB_BAR_PADDING, 2 * TAB_BAR_PADDING)`; `tab_at` uses private `covers`; `paint` reads cached rects, bar then tabs; `release` is one function (24.3's major); `on_event` consumes `Left`/`Right`/`Home`/`End`/`DPadLeft`/`DPadRight` only while focused.
13. **Eight tests in `button.rs`:** `a_button_nobody_selected_draws_exactly_what_it_drew_before_selected_existed`, `the_second_palette_defaults_to_the_unselected_one`, `a_selected_button_draws_its_selected_palette_and_an_unselected_one_draws_its_own`, `selecting_a_button_changes_nothing_it_paints_until_it_is_aimed`, `a_selected_button_ring_colour_comes_from_the_selected_palette`, `a_selected_button_still_activates_and_still_draws_its_ring`, `may_activate_is_selected_independent_in_all_four_cases`, `selected_is_not_a_button_state_and_the_primary_state_still_describes_the_background`.
14. **Twenty-six tests in `tab_bar.rs`**, on a `fn fixture() -> (Arena<WidgetNode>, TabBar, Vec<Handle>)` building a four-tab bar at `Rect::new(664.0, 120.0, 520.0, 64.0)`:
    `every_tab_button_is_a_child_of_the_bar_in_the_order_the_labels_came_in`, `every_label_the_bar_was_given_names_exactly_one_tab`, `a_button_that_is_not_a_tab_of_this_bar_is_never_aimed_and_never_painted_by_it`, `the_bar_places_its_tabs_left_to_right_with_the_gap_and_the_padding_it_declared`, `the_bar_is_exactly_as_tall_as_its_padding_and_its_tabs`, `every_tab_button_is_at_least_the_touch_target_floor_tall`, `the_measured_size_is_the_sum_of_the_labels_the_gaps_and_the_padding`, `a_bar_with_no_tabs_is_a_box_of_its_padding_and_says_so`, `tab_at_tab_rect_and_hit_test_read_the_same_box`, `the_bar_paints_its_background_and_then_its_tabs_in_the_bar_order`, `a_tab_with_no_cached_rect_is_painted_as_nothing_and_not_as_a_default_box`, `selecting_a_tab_moves_the_selection_and_reports_it_once`, `selecting_the_current_tab_is_a_total_no_op_including_the_aim`, `selecting_past_the_last_tab_changes_nothing_and_says_so`, `select_name_resolves_a_label_and_refuses_one_that_is_not_a_tab`, `the_selected_tab_is_aimed_on_a_press_and_on_a_release_that_changed_no_selection`, `a_click_through_the_buttons_own_event_selects_the_tab_and_reports_it_on_the_next_sync`, `an_activation_key_on_the_focused_tab_selects_it_through_the_same_path`, `an_unfocused_tab_declines_an_activation_key_and_lets_it_travel_on`, `left_and_right_and_home_and_end_walk_the_selection_and_wrap_at_both_ends`, `an_arithmetic_key_with_no_tab_focused_is_left_for_the_focused_control`, `the_bar_consumes_nothing_but_the_four_navigation_keys`, `a_selection_change_aims_exactly_two_tabs_and_touches_no_other_flag`, `an_aim_is_not_repeated_on_a_second_sync_that_changes_nothing`, `the_focus_record_is_not_part_of_the_aimed_triple`, `the_bar_follows_a_new_palette_and_a_new_selected_palette_on_the_next_sync`.
15. **`doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Module Layout* gains `tab_bar.rs — the row of tabs and the selected one`, between `slider.rs` and `text_input.rs`.**
16. **43.1's verification is `.ai/agents/developer.md` § *Phase 3*; task-specific, it launches `ui_demo` zero times and does not touch `DEMO_APPLICATION.md`** (gap #7 must not be marked closed by a mechanism with no consumer).

### Sub-task 43.2 — the demo's tabs become one `TabBar`, and gap #7 closes

17. **`Demo::tabs: Vec<Tab>` becomes `Demo::bar: TabBar`, `struct Tab` deleted.** `Demo::new` builds the bar at the container's `Position`/`Constraints` from `Page::ALL.iter().map(|page| page.name())`; calls `self.bar.set_palette(tab_palettes(&theme).0)`, `self.bar.set_selected_palette(tab_palettes(&theme).1)`, `self.bar.set_motion(tab_motion(&theme))`; sets `font_size` to `TAB_BUTTON_FONT` and `padding_h` to `TAB_BUTTON_PADDING_H` before `self.bar.measure(&mut nodes, &tab_advance(&metrics), tab_line_height(&metrics))`; `self.bar.snap_to_state()`; binds `self.bar.bar.background`/`.border_radius` (`ThemeToken::Surface`; `ThemeToken::BorderRadiusMd` via `.as_number().unwrap_or(CARD_RADIUS_FALLBACK)`).
18. **Delete `TAB_BAR_PADDING`, `TAB_BUTTON_TALL`, `TAB_BUTTON_GAP` by name.** `TAB_BAR_HEIGHT = 64.0`, `CONTENT_TOP`, `TAB_BUTTON_FONT` and `TAB_BUTTON_PADDING_H` stay; `TAB_BAR_HEIGHT`'s doc becomes *"`TAB_HEIGHT` plus `TAB_BAR_PADDING` on the top and the bottom is exactly `TAB_BAR_HEIGHT`"*; `grep -c 'TAB_BAR_PADDING\|TAB_BUTTON_TALL\|TAB_BUTTON_GAP' ui/src/ui_demo/src/main.rs` returns **0**.
19. **`tab_palette(theme, selected) -> ButtonPalette` becomes `tab_palettes(theme) -> (ButtonPalette, ButtonPalette)` = `(unselected, selected)`**; the six existing tests keep names and numeric assertions, reading `.1`/`.0`; new `the_two_tab_palettes_differ_in_all_three_colours`.
20. **`Demo::tab_bar()` returns `&TabBar`**, keeps `#[cfg(test)]`/`Demo::containers[TAB_BAR]`, adds `handle()` = `self.bar.bar.handle()`.
21. **Seven functions deleted by name, each a delegation:** `Demo::tab_focusables`→`TabBar::tab_ids`; `tab_button`→`TabBar::tab_id_of`+`tab`; `tab_at`→`TabBar::tab_at`; `aim_tab_buttons`→`TabBar::sync`; `press_tab`→`TabBar::press`; `release_tab`→`TabBar::release`; `sync_tab_hover`→`TabBar::hover`.
22. **`Demo::pending_page` writers 7→1:** `Demo::new` sets `self.bar.on_select = Callback::new(move |id: TabId| { if let Some(&page) = Page::ALL.get(id.index()) { asked.set(Some(page)); } });`, deleting each per-button `Callback::new(move || …)`.
23. **`Demo::frame`:** replace the tab tick loop with `let _ = self.bar.tick(delta); self.bar.sync();`; move the `pending_page` drain to the frame end; `set_focus` becomes `self.bar.focus(focused.and_then(|handle| self.bar.tab_id_of(handle)))`.
24. **`Demo::show_page`:** after `screens.show(page.name())`/`screens.sync(&mut nodes)`, `let _ = self.bar.select_name(page.name());`; early return kept.
25. **`Demo::offer_to`'s tab arm** `if let Some(id) = self.bar.tab_id_of(handle) { if let Some(button) = self.bar.tab(id) { return button.on_event(event); } }`; **`Demo::route_input_event`** fallback `self.bar.tab_at(&self.nodes.borrow(), Offset::new(x, y))`; **`handle_event`** press `self.bar.press(id)`/release `self.bar.release()`; **`Demo::frame`** paint `self.bar.paint(&arena, &advance, tab_line_height(&self.metrics))`.
26. **Demo tests kept by name, assertions unchanged:** `the_bar_is_exactly_as_tall_as_its_padding_and_its_buttons`, `every_tab_button_is_at_least_the_touch_target_floor_tall`, `tab_walks_the_seven_buttons_before_the_pages_own_controls`, `every_page_places_every_rect_where_the_gallery_placed_it`, `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`, `placed_handles`, `page_rects`, `always_painted_handles`, `nothing_the_demo_places_reaches_into_the_strip`, `the_container_with_a_background_are_the_card_and_the_bar`, `the_tab_bar_follows_a_theme_switch`, `the_tab_bar_is_still_first_in_the_route_chain_for_each_of_its_seven_buttons`, `a_tap_over_a_bar_button_is_not_in_the_routed_chain`.
    New: `every_tab_index_names_the_page_at_that_index_of_page_all` (walks `0 .. Page::ALL.len()`), `the_demo_bar_has_one_tab_per_page_and_no_tab_beyond_the_seven`, `the_tab_bar_selection_and_the_screens_current_name_never_disagree` (all seven pages, both switch paths), `the_bar_still_records_the_command_sequence_it_recorded_before`, `a_page_switch_aims_exactly_the_two_tabs_it_moved_between`, `a_tab_on_the_page_already_on_show_is_aimed_by_its_press_and_its_release`, `the_focus_ring_follows_the_focus_record_and_the_selection_follows_the_screen`, `the_two_tab_palettes_differ_in_all_three_colours`. `every_dialog_button_is_still_in_the_route_chain` and `a_tap_over_a_bar_button_is_not_in_the_routed_chain` are task 42.2's and 24.3's, kept green.
27. **`DEMO_APPLICATION.md` § *Library gaps* row 7 gains a dated note** in the form § *Corrections to the second gap table* prescribes (not a deletion, not a bare "closed"): `TASK_UI_PRIM_43` closes the row's `ui_core` claim, correcting its *"no `selected`"*/*"no widget behind it"* and its `TASK_UI_PRIM_24.3` sentences in place; the *"icon+label layout"* half is **NOT delivered** and names gap **#4**, `TASK_UI_PRIM_44`; one consumer exists (the gallery) and the row is not closed on the library type alone; the fourth of § *Relationship to task 24*'s four places (the demo tab-bar source comment, the same one 42's requirement 20 names) is amended or checked. `Severity` stays `Low`, `Blocks` keeps "Bottom dock implementation".
28. **43.2's verification is `.ai/agents/developer.md` § *Phase 3*; task-specific, the seven-page before/after capture (`.ai/tools/README.md` § *Capturing a window* verbatim, **AE 0 outside `y ≥ 680`** on all seven) and the frame rate on all seven pages via `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>`, each above the floor of **55**.

## Acceptance Criteria

- [ ] `Button::selected` is `pub selected: Property<bool>`, `false`, between `activatable` and `scale`; `grep -c 'pub selected'` is **1**; `may_activate`/`ButtonState` untouched; ring width `focused`-only; `paint_faded` changed one line, `style()` two, the flag derivations and `animate_to_state` untouched; the two ring tests green.
- [ ] `tab_bar.rs`: `pub mod tab_bar` is the sixteenth `pub mod`; `awk '/^pub struct TabBar/,/^}/'` shows `bar`/`on_select`/`requested`/`selected`/`motion`/`tabs`; `grep -c 'pub struct Tab'` is **0**; `grep -c '    pub fn'` is **27**; no `len`/`contains`/`iter`/`Page`; no `Rect` parameter.
- [ ] Selection is one write path, idempotent, reported once (the four `select*` tests, two-hop proved by `a_click_through_the_buttons_own_event_selects_the_tab_and_reports_it_on_the_next_sync`); the aim is inside `sync` (`grep -n 'animate_to_state'`: one line in `tab_bar.rs`, none in `main.rs`), `an_aim_is_not_repeated_on_a_second_sync_that_changes_nothing` moves counters exactly four times, and 24.3's major tests green.
- [ ] The bar consumes only the six navigation keys while focused, `LongPress`/`Swipe` unconsumed by name; `grep -c 'pub fn on_event'` 8→9; row `L4` true; the layout a `row`, `LayoutMode::Grid` still `=> Vec::new()` (`L3`, `TASK_UI_PRIM_52`), no cross-axis gap (`L7`), constants finite/positive, `TAB_HEIGHT ≥ 44.0`; the data sweep tests present.
- [ ] `cargo test --all-features` green against 1894 with the **8**/**26**/**8** new tests named, `ui_core` **1484**, `ui_demo` **232**, doctests **223** (**1939**+), no test weakened, the six `tab_palette` tests reading `.0`/`.1`; the seven pages pixel-identical outside `y ≥ 680` (**AE 0**) with the command sequence unchanged (`the_bar_still_records_the_command_sequence_it_recorded_before`) and the frame rate above **55** and inside **61.1–63.9**.
- [ ] Nothing leaked in (`git diff --stat` clean for `layout.rs`/`property.rs`/`paint.rs`/`batch.rs`/`render.rs`/`input.rs`/`animation.rs`/`theme.rs`; no new dependency; no `unsafe`/`unwrap`/`expect`/`panic!`/`todo!`/`Box<dyn Trait>`/`println!`; no doc comment contradicting the code).
- [ ] `DEMO_APPLICATION.md` row 7 amended and dated (requirement 27, gap **#4** / `TASK_UI_PRIM_44`, `Severity: Low`); the split honoured (43.1 no `nav`, 43.2 against 42); no pointer-event claim; no criterion waived (none needs a pointer event, display, network, filesystem or wall clock; `fps-check.sh` and `the_bar_still_records_the_command_sequence_it_recorded_before` cover the two things a capture cannot see; 43.1 launches `ui_demo` zero times).

## Out of Scope

- **No icons and no icon+label layout** — gap **#4**, `TASK_UI_PRIM_44`; row 7's icon half stays open (`Polygon` is convex-only, no bezier — **`L10`**; `Image` needs an atlas).
- **No scrolling, overflow handling, wrapping or ellipsis** — `Flex.wrap` is discarded and `arrange_flex` clips, so a bar that does not fit overflows.
- **No `LongPress` and no `Swipe`** — `InputEventKind` has nine variants and this widget consumes one under one condition; row `L4` stays true.
- **No re-parenting of the screen system, no change to `Screens`, `nav.rs` or `LayoutState::hits`** — `TASK_UI_PRIM_42` is a prerequisite and is not touched; `ScreenId` gains nothing.
- **No `Grid`** (gap **`L3`**, `TASK_UI_PRIM_52`) **and no cross-axis gap** (gap **`L7`**).
- **No transition or cross-fade of the selection change** — it rides `Button`'s own clock on `Motion::from_theme`'s `DurationFast` (150 ms, not 300), which this task does not change or settle; gaps **#8** / **L2** keep Critical.
- **No focus movement** — `TabBar::on_event` moves the selection, not the focus ring; wiring arrows into the demo is one arm in `Demo::offer_to` plus one line in `Demo::set_focus`, and a caller wanting them coupled writes `TabBar::focus(Some(id))` from its own `on_select`.
- **No content node, screen, history or router in `TabBar`**, no `Page` type, and `keyboard::Page` is not renamed; **no `TabBar::remove` and no `TabId` reuse**.
- **No change to any pipeline mechanism** (`DrawCommand`'s variants, `PaintState`, `Batcher`, `BatchKey`, `Segment`, `Renderer::begin_frame`, `Palette`, `Style`, `Motion`, `Easing`, `AnimationClock`, `Property`, `InputEventKind`, `InputEvent`; `DEPTH_BITS`, depth policy, `Mat4`, both projections and every shader source untouched); **no change to `LayoutMode`, `FlexConfig`, `Padding`, `Constraints` or `Layout`**; **no new dependency and no `unsafe`**; **no change to the demo's `Page`, its seven names, or `CONTENT_TOP`**. 34–41 and 52 are specified and not started.
- **No criterion waived, and none requires a pointer event, a GL readback, a display, a network, a filesystem or the wall clock.**
