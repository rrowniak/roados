# TASK_UI_PRIM_50: A Theme That Scopes — `FocusRing`, `scope::ThemeScope`, and What a Switch Does Inside a Scope

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Close row **`L9`** of `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* by adding **one theme token**, `ThemeToken::FocusRing`, and **a new type**, `scope::ThemeScope`, a set of per-node token overrides a subtree resolves through.
The token count goes **33 → 34**, and **six of the seven interaction-state tokens row `L9` lists are declined by argument** with the crate's recorded precedents rather than delivered.
This is the task `TASK_UI_PRIM_47` deferred to by name: its § *The theme gains no token, and row `L9` is why* hands clause **(a)** of row `L6b` here, and § *Out of Scope* there says *"Clause (a) of `L6b` is row `L9`'s, and that row is not opened here."* **This file opens it.**

## Context

**Token decision: `ThemeToken::FocusRing`, nothing else; `TOKEN_COUNT` 33 → 34; `Theme`, `PropertyValue` and `Theme::from_table` otherwise unchanged.**
The crate's test for a token (`button.rs`'s `MIN_TOUCH_TARGET` doc, repeated in `keyboard.rs`, `toggle.rs`, `slider.rs`) is *"does a theme switch change this value?"*
Six of the seven names row `L9`/`L6b`(a) list fail it, each declined with a precedent: `Hover`/`Pressed` (states are not colours of their own; `HOVER_LIGHTEN = 0.12`, `PRESS_DARKEN = 0.18`, `PRESSED_SCALE = 0.95`, `PRESS_SHADOW_ALPHA = 0.28`); `Shadow` (`Button`'s pressed-inner constants; `render.rs`'s `GL_R8` backdrop, row `L1`); `ZOrder` (not a value; `DrawCommand`'s nine variants carry no z, row `L2`); `Active` (`Button` has no selected state; `Keyboard`/`text_input` read `Primary`); `Selected` (`TabBar`, `TASK_UI_PRIM_43`, row `7`).
`FocusRing` is declared immediately after `ThemeToken::Success` and holds `Text`'s value in each theme — `(255, 255, 255, 255)` dark, `(0, 0, 0, 255)` light — so no pixel moves; `TOKEN_COUNT` stays `const` and private.

**Scoping decision: `scope::ThemeScope`, a `Vec<(Handle, ThemeToken, Property<PropertyValue>)>` holding only a node's overrides, resolved by a nearest-ancestor walk by the owner, never the widget** (`WidgetNode` has no arena; `Property::bind` gets no node handle).
`resolve_property` returns a `Property<PropertyValue>`, the same `Rc` the switch animates.
Rules: a child sees its nearest ancestor's override; nearest wins per token; a partial override is the default and no "replace the whole theme" operation exists.
An overridden token does not move during `switch_to`, nor anything bound to it, while unoverridden tokens animate — a **half-crossfade**; the one way to make an override follow the switch is a `Property::bind` on a token the switch animates.

**`TASK_UI_PRIM_47` fixes two things this task must not duplicate:** `property.rs`'s module doc and `PRIMITIVES_ARCHITECTURE.md` § *Inheritance*; this task only amends the latter. The `TOKEN_COUNT = 33` in `TASK_UI_PRIM_44.md`/`TASK_UI_PRIM_47.md` is superseded by 34, neither edited.

## Requirements

*Numbering follows the original; original 9 (an `IMPLEMENTATION_STATE.md` entry) and original 11 (the suite, capture and frame-rate roll-up) are dropped as state-file and `.ai/agents/developer.md` § *Phase 3* material, and the survivors are renumbered consecutively.*

1. **A new module `ui/src/ui_core/src/scope.rs`, declared `pub mod scope;` in `lib.rs`** in alphabetical position (**after `render`**, before `snapshot` if `TASK_UI_PRIM_47` landed, else before `texture`), holding one public type:

   ```rust
   #[derive(Clone, Default)]
   pub struct ThemeScope {
       overrides: Vec<(Handle, ThemeToken, Property<PropertyValue>)>,
   }

   impl ThemeScope {
       #[must_use] pub fn new() -> Self;
       #[must_use] pub fn len(&self) -> usize;
       #[must_use] pub fn is_empty(&self) -> bool;
       #[must_use] pub fn set_override(
           &mut self, handle: Handle, token: ThemeToken, value: Property<PropertyValue>,
       ) -> bool;
       #[must_use] pub fn clear(&mut self, handle: Handle) -> usize;
       pub fn retain(&mut self, nodes: &Arena<WidgetNode>) -> usize;
       #[must_use] pub fn resolve_property(
           &self, theme: &Theme, nodes: &Arena<WidgetNode>, handle: Handle, token: ThemeToken,
       ) -> Property<PropertyValue>;
   }
   ```

   Do not derive `Debug` (`Property` has no `Debug`). `overrides` is a `Vec`; `Clone` shares the properties. `set_override`/`clear` return counts and are `#[must_use]`, `retain` is not; `set_override` takes no arena; `resolve_property` is `O(depth)`. The module doc records what a scope is and is not, the three rules, the switch semantics including the half-crossfade, the bound-override route, the owner-resolves rule, `ModeScope` beside it, and the `L6b`(a) hand-off by name.

2. **`ThemeToken` gains one variant, `FocusRing`; 33 → 34**, in five edits to `theme.rs`: declared immediately after `ThemeToken::Success`, its doc naming the rule in `slider::Palette::from_theme`'s words (*"the colour this repository uses for anything that has to be legible on the background itself"*) and what it is not (`OnPrimary`, `Border`); the `ALL_TOKENS` entry in the same position; `const TOKEN_COUNT: usize = 34;` stays `const`, private, not `pub`; `Theme::dark` and `Theme::light` each gain one row, exactly `Text`'s value; `PropertyValue`'s doc "for all 33 tokens" becomes 34. Everything else in `theme.rs` is byte-identical; no `unsafe`, `unwrap`, `expect`, `panic!`.

3. **`theme.rs` tests:** `dark_and_light_define_every_token` keeps its name, comment and shape with number → **34** and *"all 33 tokens"* → *"all 34 tokens"*; `is_color_token` gains `FocusRing`; new `every_token_is_answered_with_a_value_by_both_themes` (`theme.get(token) != PropertyValue::default()` for each theme and token through the public `get`); new `the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes` (`theme.get(FocusRing) == theme.get(Text)` for both themes, the pixel-identity guard).

4. **Three palettes re-point their ring, one line each, changing no field:** `toggle`, `slider` and `scroll` `Palette::from_theme` map `ring: token_color(theme, ThemeToken::FocusRing)` where each maps `Text`; `TASK_UI_PRIM_47`'s `Segmented` too if 47 landed. Each assertion in `the_palette_is_the_themes_muted_primary_and_on_primary`, `the_palette_is_the_themes_border_text_muted_and_text` and `the_palette_is_the_themes_border_primary_and_on_primary` names `FocusRing` with its message rewritten; **names and all other assertions unchanged; no test deleted, renamed or weakened.** `button.rs`/`keyboard.rs`/`text_input.rs` untouched (`OnPrimary`, `Border`, `Primary`). **No `Palette` gains a field** (`ui/src/ui_demo/src/main.rs`'s `tab_palette` `ButtonPalette` struct literal).

5. **`scope.rs`'s module doc**, in the form `rotator.rs` and `render/mesh.rs` carry, carrying every Context claim, in particular the half-crossfade.

6. **Two doctests in `scope.rs`'s module doc:** an empty scope answers every token from the theme; a partial override where a child resolves the override for `Surface` and the theme's value for `Text`.

7. **`PRIMITIVES_ARCHITECTURE.md`, three edits:** § *Theme tokens* gains `FocusRing` at the end of its colour block matching declaration order (`theme.rs`'s `ThemeToken` doc cites it), § *Animated theme transitions* untouched; § *Inheritance* — **prerequisite: `TASK_UI_PRIM_47` has corrected it** — adds `scope::ThemeScope` and rewords "only" to name **two** inheritances (a caller-chosen `K` and a `ThemeToken`), and says `Property` has none; § *Module Layout* gains `scope.rs` after `render/` and before `texture.rs`, no other row touched.

8. **`DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*, row `L9`: a dated note and the evidence column rewritten; nothing else in the section touched.** Note: (i) `ThemeScope` closes the row's first clause; (ii) `FocusRing` added, count 34, six declines recorded, superseding `TOKEN_COUNT = 33` in `TASK_UI_PRIM_44.md`/`TASK_UI_PRIM_47.md` (neither edited); (iii) row not deleted or closed (*"no tokens for … hover, pressed, shadow or z-order"* still true); (iv) no per-widget property removed or replaced; (v) 47's `L6b`(a) hand-off discharged. Line numbers → symbol and path (`ThemeToken`, `TOKEN_COUNT`, `Theme` in `theme.rs`; `Button::focused`/`hovered`/`pressed`/`focus_ring`/`disabled`/`activatable` in `widgets/button.rs`); other rows left alone; **row `L6b` not edited**.

9. **The tests, named, with no display, no network, no filesystem and no wall clock.** In `scope.rs` — seventeen: `a_scoped_subtree_resolves_the_override_and_the_rest_of_the_tree_resolves_the_theme`; `a_scope_that_overrides_one_token_leaves_every_other_token_on_the_theme`; `two_siblings_under_one_scoped_node_resolve_the_same_override`; `a_node_resolves_the_override_set_on_itself`; `the_nearest_scoped_ancestor_wins_for_the_token_it_overrides_and_the_scope_below_it_inherits_the_rest`; `an_override_does_not_cross_a_boundary_into_a_different_root`; `an_override_for_a_node_the_arena_no_longer_holds_cannot_be_resolved`; `retain_drops_every_override_for_a_node_the_arena_no_longer_holds`; `set_override_reports_whether_the_scope_gained_or_replaced_an_entry`; `clear_takes_every_override_off_the_subtree_below_it_and_reports_how_many_it_dropped`; `a_scope_with_no_overrides_resolves_every_token_from_the_theme`; `len_counts_overrides_and_not_scoped_nodes`; `resolve_property_returns_a_handle_that_follows_the_token_it_fell_through_to`; `an_animated_theme_switch_does_not_move_an_overridden_token`; `an_override_bound_to_another_token_animates_with_that_token_and_not_with_its_own`; `a_bound_property_over_a_resolved_handle_is_recomputed_on_every_frame_of_a_switch`; `a_handle_resolved_before_an_override_was_added_keeps_the_theme_token`. In `theme.rs` — three, one an update: `dark_and_light_define_every_token` (updated to 34), `every_token_is_answered_with_a_value_by_both_themes`, `the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes`. In `toggle.rs`, `scroll.rs` and `slider.rs` — zero new tests, three updated assertions. Verification is `.ai/agents/developer.md` § *Phase 3*; what is specific here is that **no page is expected to move**.

## Acceptance Criteria

- [ ] **`ThemeScope` holds only overrides and resolves by the nearest ancestor.** `grep -n 'pub struct ThemeScope' ui/src/ui_core/src/scope.rs` shows only `overrides: Vec<(Handle, ThemeToken, Property<PropertyValue>)>`; `pub mod scope` is in `lib.rs` alphabetically; `grep -c 'pub struct\|pub enum' scope.rs` is **1**; the required test passes.

- [ ] **The three rules and the switch are tests, not prose.** `a_scope_that_overrides_one_token_leaves_every_other_token_on_the_theme` asserts exactly one of **thirty-four** tokens differs; the nearest-wins and cross-root tests pin the walk's ends; `an_animated_theme_switch_does_not_move_an_overridden_token` asserts `Surface` bit-identical before, during and after while `Text` arrives at the light value; the half-crossfade is in `scope.rs`'s module doc and `resolve_property`'s; `retain_drops_every_override_for_a_node_the_arena_no_longer_holds` passes, and deleting the body fails it.

- [ ] **An override follows a switch with no per-frame work.** `resolve_property_returns_a_handle_that_follows_the_token_it_fell_through_to` holds a handle resolved before the switch and asserts it moved and at half way equals the interpolated value; `a_bound_property_over_a_resolved_handle_is_recomputed_on_every_frame_of_a_switch` asserts the frame count and arrival; returning `Property::new(theme.get(token))` must fail those two only.

- [ ] **`TOKEN_COUNT` is 34, `const` and private, and `FocusRing` is the only new token.** `grep -n 'TOKEN_COUNT' ui/src/ui_core/src/theme.rs` shows `const TOKEN_COUNT: usize = 34;` and no `pub`; `dark_and_light_define_every_token` asserts `ThemeToken::all().len() == 34`; `grep -c '^    pub fn' ui/src/ui_core/src/theme.rs` is **14**, unchanged from `HEAD`; `grep -n 'all 33 tokens'` and grep for `ThemeToken::Hover|Pressed|Shadow|ZOrder|Active|Selected` return nothing; `grep -n 'FocusRing'` shows the variant, `ALL_TOKENS` entry, one row per theme table and `is_color_token`, and in `widgets/*.rs` exactly three mappings and three assertions (plus `segmented.rs` if 47 landed); `every_token_is_answered_with_a_value_by_both_themes` passes.

- [ ] **No per-widget property removed, replaced or retyped; no ring colour changed.** `git diff --stat` empty for `button.rs`, `keyboard.rs`, `text_input.rs`; one changed line and one changed assertion each in `toggle.rs`, `scroll.rs`, `slider.rs`, no changed `pub struct Palette` or test name; `tab_palette` still compiles.

- [ ] **Suite green with every named test present** (listed by name), at least **1915** (baseline **1894** = **1450** `ui_core` + **224** `ui_demo` + **220** doctests, plus 17 + 2 + 2), none deleted, renamed or weakened, **the handoff names the tree measured on**; `cargo fmt --check`, `cargo build --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings` and `cargo doc --no-deps` clean; `cargo audit` recorded as not installed, not passed.

- [ ] **The six pages are pixel-identical and the frame rate is measured.** Before and after per `.ai/tools/README.md` § *Capturing a window*, window id re-read at each capture, **AE 0 outside `y ≥ 680`** on all six; no file under `ui/src/ui_demo/` changes; `FocusRing` holds `Text`'s value in each theme; a mid-switch capture cannot differ; `every_page_places_every_rect_where_the_gallery_placed_it`, `no_two_placed_rects_overlap`, `assert_the_pages_partition_the_placed_rects` and `every_placed_rect_is_inside_the_window` pass unamended. Frame rate via `.ai/tools/fps-check.sh 10 55` and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` on all six, every page above **55**, expected inside **61.1–63.9**; the only new per-frame work is **34** animated properties instead of 33.

- [ ] **Nothing else changed.** `git diff --stat` unchanged for `property.rs`, `render.rs`, `render/target.rs`, `render/blur.rs`, `paint.rs`, `batch.rs`, `input.rs`, `layout.rs`, `arena.rs`, `node.rs`, `animation.rs`, `widgets/button.rs`, `keyboard.rs`, `text_input.rs`, `container.rs`, `label.rs`, `list.rs`, `dialog.rs`, `toast.rs`, `chart.rs`, `gauge.rs`, `image.rs`, `progress.rs` or `ui/src/ui_demo/src/main.rs`; `grep -c unsafe` is **0** for `scope.rs`; no `unwrap()`, `expect(`, `panic!`, `unimplemented!` or `todo!` in it. Exactly these paths changed: `scope.rs` (new), `theme.rs`, `lib.rs`, `widgets/toggle.rs`, `widgets/slider.rs`, `widgets/scroll.rs`, `doc/ui/DEMO_APPLICATION.md`, `doc/ui/PRIMITIVES_ARCHITECTURE.md`, plus `widgets/segmented.rs` if and only if 47 landed, plus this file. `ui/Cargo.toml` and `ui/Cargo.lock` are unchanged (`sdl3 0.20`, `glow 0.18`, `freetype-rs 0.38`).

- [ ] **Documentation prerequisites honoured; the row is amended, not closed.** § *Theme tokens* carries `FocusRing`; § *Inheritance* names two inheritances and says `Property` has none (premise: 47 corrected it first); row `L9` carries a dated note naming `TASK_UI_PRIM_50` (scoping clause closed, token clause narrowed to one token with six declines, not deleted or closed, line numbers replaced by symbol and path, `TOKEN_COUNT` supersession recorded with `TASK_UI_PRIM_44.md`/`47.md` unedited); row `L6b` not edited. `scope.rs`'s module doc carries the map-not-stack decision, the three rules, the switch semantics including the half-crossfade, the bound-override route, the owner-resolves rule and the `L6b`(a) hand-off by name; `theme.rs`'s module doc carries the token table with each decline's precedent; `ui/src/ui_demo/src/main.rs` carries nothing; the honest limit (no pointer event has ever been observed reaching this window on this host) is in this file.

## Out of Scope

- **No demo producer for a scoped subtree.** `ui/src/ui_demo/src/main.rs` is not touched; no subtree in the demo needs one today (task 46's decision shape).
- **No mode propagation and no `L6b` clause (b) or (c), and `TASK_UI_PRIM_47`'s two fixes are not duplicated.** `mode::ModeScope`, `snapshot::Snapshot` and `Segmented` are 47's; `property.rs`'s module doc is untouched; § *Inheritance* is amended, not corrected from scratch; merging the two inheritances into one generic type is refused.
- **No `Grid`** (row `L3`, `TASK_UI_PRIM_52`), **no `TabBar`/`Button::selected`/`Selected`** (gap `7`, `TASK_UI_PRIM_43`), **no `Icon`/tint uniform/`widgets/icon.rs`** (gap `#4`, `TASK_UI_PRIM_44`).
- **No dark/light re-design and no retune of any shipped value.** `FocusRing` is introduced at `Text`'s value in both themes; every other colour is byte-identical to `HEAD`.
- **No removal, replacement or retyping of any existing per-widget property.** `Button`, `Scroll`, `Slider` and `Keyboard::focus_ring` stay `Property<f32>`; every `Property<bool>` state stays. **No `Hover`, `Pressed`, `Shadow`, `ZOrder` or `Active` token**, each declined with a precedent in Context.
- **No `Theme` change beyond the one variant, the two rows and the count.** No `Theme::push_scope`, `pop_scope`, scope-aware `get`, scoped `switch_to` or `Theme::clear`; `Theme` remains one flat global map.
- **No `TokenSource` trait, no generic parameter on any `Palette::from_theme`, no scoped palette constructor, no `Palette` field added, removed or retyped.**
- **No `node::descends_from` promotion and no shared ancestor iterator.** The crate holds two ancestor walks, recorded here as a later cleanup rather than smuggled in.
- **No pipeline change**: no `DrawCommand` variant, `ShaderKind`, uniform, `RenderCommand`, batching key, scissor, mesh or `Mat4`. **No new page and no change to any of the six** — `Page::ALL` stays six, `Page::DEFAULT` stays `Pads`, the `--help` text is unchanged.
- **No new dependency.** The approved direct dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`. **No `unsafe`, no `unwrap`, no `expect` and no `panic!`** in `scope.rs` or the changed code.
- **No clock, no `tick`, and no animation of its own on `ThemeScope`.** A scope does not animate; the properties it hands out animate.
