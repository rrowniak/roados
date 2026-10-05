# TASK_UI_PRIM_24: Demo Application

## Goal

Finish the demo application: group the widgets that tasks 11–22 built into
**pages behind a tab bar at the top of the window**, and make any page
reachable **directly from the command line** without clicking.

## Context

The demo application is the final task of the `PRIM` sequence. It uses the
widgets implemented in the earlier tasks to build a gallery, and it groups them
so that one page shows one kind of thing rather than all twenty-six widgets
sharing one crowded canvas.

**Amended 2026-10-03 by the operator; the supersession of 2026-09-30 is
withdrawn for the widget gallery.** `doc/ui/DEMO_APPLICATION.md` § *Relationship
to task 24* recorded this task as *"superseded … will not be started in its
current form"*, and the reasoning was sound: a widget gallery is not a demo
application. **What changed is that the gallery is what tasks 11–22 have been
building, widget by widget, and it is finished rather than replaced.** The
Tesla direction is untouched and stays in `DEMO_APPLICATION.md`; the operator's
decision of the same date is that the Tesla application will eventually be
**one more tab** rather than a replacement for this, and that question is not
settled here.

**Amended 2026-10-04 by the operator: the `overlays` page is the `Dialog`
*and* the `Toast`.** Task 23 landed as `1fed4b6`, so this file's three
sentences saying `toast` *"is not written"* — in this Context, in requirement 1
and in *Out of scope* — are **void, and are superseded here rather than left to
be found false**. The operator's reason is that the amendment above excluded the
toast **only because it did not exist**, and that reason is gone. **The other
two absences in requirement 1 stand**, and each for the same kind of reason —
the widget the screen needs is one the crate cannot build yet: *Home screen*
needs an app-launcher grid and `LayoutMode::Grid` lays out no
children, and *Navigation screen* needs a `List` the demo no longer has.

**What this amendment replaces, so the original can still be audited:**

| Original | Now |
|---|---|
| Requirement 2's seven screens | Requirement 1's six pages |
| Requirement 3's *"Bottom navigation bar (tabs for each screen)"* | Requirement 2's tab bar at the **top** |
| — | Requirement 3's `--tab=` argument, which did not exist |
| Requirement 5's *"44x44dp minimum"* | unchanged; it is why requirement 2's buttons are 44 tall |
| Requirement 6's *"60 FPS target"* | unchanged, and requirement 7 adds the per-page measurement |

**Three facts about the demo as it stands decide the shape of this task, and
each was measured rather than assumed.**

1. **The window cannot grow, and the canvas is full.** `WINDOW`, in
   `ui/src/ui_demo/src/main.rs`, is 1280 × 1020; a 1280 × 1160 request came back
   1280 × 1052, the window manager's cap on this host, and the **list was
   removed on 2026-10-02** to make room for the chart. Every position in the
   demo is an absolute constant. So a tab bar has no room of its own, and
   requirement 6 is not a stylistic choice: the crowding is an artifact of one
   canvas holding every widget at once, and it dissolves once pages exist.
2. **Hiding a page is three gates, not one.** `LayoutState::set_visible` is
   documented as hit-testing only — *"the pass places every node it reaches,
   visible or not, so hiding a node leaves its rect — and its siblings' rects —
   untouched, and only hit testing consults the flag"*
   (`LayoutState::visible` and `LayoutState::set_visible` in
   `ui/src/ui_core/src/layout.rs`), and `hit_test` does skip an invisible
   subtree (`hit_test_from`, in `ui/src/ui_core/src/input.rs`). But the demo's
   frame loop paints **every** handle in `self.order` with no test (the
   `for handle in self.order` walk in `Demo::frame`), and `Demo::draw` sends
   all of `order`. Requirement 4 names the three.
3. **A pointer cannot be injected on this host, so the argument is the
   verification route and not a convenience.** Keyboard injection has delivered
   exactly one event in this project's history — task 21's positive control,
   where `T` moved 212 pixels of gauge needle — and **pointer injection has
   never delivered anything at all**. Five of task 14's acceptance criteria and
   three of task 22's are covered by tests through the demo's own event path
   for this reason and not by a capture. A page that can only be reached by
   clicking could not be captured at all.

**`Button` has no selected state, and the tab bar is built from `Button`.**
`Button` in `ui/src/ui_core/src/widgets/button.rs` carries `hovered`, `pressed`,
`focused` and `disabled` state properties and **no `selected`**, and
requirement 2 says how the demo supplies one. A reader who assumes the widget
has it will look for it in the wrong file. **The full property list is not
reproduced here on purpose**: that file was being edited while this was written,
so an enumeration of it would be a snapshot of a moving target within minutes.
Read the struct.

**What a tab bar pays back.** The module doc of `ui/src/ui_demo/src/main.rs`
records that three buttons were removed on 2026-10-01 to make room for the
chart, and what it cost: *"the press and release transition, the
hover tint, the focus ring and the click callback are no longer on screen
anywhere, and `ui_demo` is the only place in the repository where any of them
was demonstrated."* A row of buttons puts all five back.

**Verification note, so it is not rediscovered per sub-task.** Any criterion
below that needs a tap or a click cannot be capture-verified on this host
(third fact above). It is covered by unit tests **through the demo's
own event path** — `Demo::handle_event`, the same path a real press takes —
which is what tasks 14, 18 and 22 did, and the handoff says which is which
rather than claiming a capture that was not taken.

**Citations in this file and its sub-tasks are anchored to symbols, not to line
numbers, and that is deliberate.** They were written on 2026-10-03 against a
tree that was **being edited while they were written**: `button.rs` gained a
new `activatable` property and several hundred lines between two consecutive
commands, which moved `Button::content_size` by fifty lines. **A `file:line`
citation into a file under concurrent edit decays within minutes**, and a stale
one is worse
than none — it sends a reader to the right file and the wrong place. Naming the
symbol is checkable by `grep` and survives every edit that is not the one being
described. Line numbers appear only where the number *is* the claim, and the two
that are — `WINDOW` at 1280 × 1020 and `BAND_TOP` at 720 — are constants rather
than positions.

## Requirements

1. **Six pages, named.** Each names the widgets on it, and its name is the
   value `--tab=` accepts:

   | Page | Widgets |
   |---|---|
   | `pads` | the card of pads — a `Container` with a background and padding — the press and release animation, the `Space` cascade |
   | `text` | the label column: sizes (`+`, `-`), wrapping, three alignments, truncation, the colour token (`C`) |
   | `input` | `TextInput`, the on-screen `Keyboard`, the text and submit readouts |
   | `controls` | `Slider`, `Toggle`, `Progress` and their three readouts |
   | `data` | `Gauge`, `Chart`, `Image` and their three readouts |
   | `overlays` | the `Dialog` and the `Toast` host — amended 2026-10-04, see the Context |

   **Two of the original seven screens are not in this list, and each is
   absent because it cannot be built yet rather than because it was dropped.**
   *Home screen* needs an app-launcher grid and **`LayoutMode::Grid` lays out no
   children** (the `LayoutMode::Grid` arm of `Layout::arrange` returns `Vec::new()`);
   *Navigation screen* needs a `List`, which the demo no longer has
   (removed 2026-10-02). *Overlays* **loses the toast no longer** — it was
   written for this table's sake on 2026-10-04; the 2026-10-03 text that said
   otherwise is superseded in the Context and named there. *Theme switcher* is
   not a page because the theme is global: `T` reaches every colour in the demo
   through the property graph, so a page holding only the theme switcher would be
   a page holding nothing that is not everywhere.

2. **A tab bar at the top**, and **no bottom navigation bar**. Built from a
   `Container` in `LayoutMode::row()` holding one `Button` per page — which is
   what `DEMO_APPLICATION.md` gap #7 prescribes for the dock (*"the bottom dock
   can be built from `Button` + `Container`"*), and there is no `TabBar` widget
   in `ui_core` to build it from.

   - **Top, not bottom**, and the reason is the room: the text-entry band
     already occupies the bottom 300 pixels, and a second bar down there is a
     bar sharing space with a keyboard.
   - The bar is **64 tall** and its buttons are **44 tall at y 10**. 44 is this
     project's own touch-target floor — **not** a Tesla figure; nothing in
     Tesla's documentation states a touch-target size, and `DEMO_APPLICATION.md`
     § *Design principles* records that explicitly.
   - The bar carries a `Surface` background and a padding, so it is the second
     container with a background in the demo and the padding is visible.
   - **The selected tab is the demo's own, not the widget's**: the button for
     the active page has its `background` and `foreground` swapped for the
     theme's active pair, and `animate_to_state(Motion::from_theme(&theme))`
     carries the change over `THEME_TRANSITION`, the same 300 ms a theme switch
     uses. `Button` has no `selected` property (see Context).

     **Amended 2026-10-04: the call and the duration in that sentence are
     different numbers, and 24.3 uses the call — 150 ms, not 300.**
     `Motion::from_theme` reads `DurationFast`; `THEME_TRANSITION` is the demo's
     *theme-token* duration. 150 ms is what every widget in the crate animates
     over. **The consequence is that on `T` the bar and its buttons arrive 150 ms
     apart** — the bar's own `Surface` is a bound property on the animating theme
     and moves over 300 ms, the buttons reach their colours at 150 — and **nobody
     has seen that on screen**, because `T` cannot be injected on this host. See
     `TASK_UI_PRIM_24.3.md` requirement 4's amendment.

3. **`--tab=<name>` lands on a page without a click.** The one spelling is
   `--tab=`, because it selects a **page** and `--widget=` would imply one
   widget per page.

   - `--help` prints the usage and the six names, and exits **0**.
   - An unknown name **exits non-zero** and prints the six valid names. A
     silently ignored argument is a test that passes against nothing.
   - **No argument means `pads`**, named here so a capture with no flag is
     reproducible. **`pads` and not a page that shows more**, because it is the
     one page every capture taken for tasks 11–22 contains — and the honest
     consequence is that **no single page reproduces those captures**, since each
     of them shows several pages at once. `--tab=` is what replaces that: a
     capture that used to need no argument now has to name its page, and can
     reach one that no flag could reach before.
   - `ROADOS_RUN_SECONDS` and `ROADOS_ASSET_DIR` **stay environment
     variables**. Migrating them is out of scope and is a separate decision,
     but the inconsistency is real and is named here rather than left to be
     discovered.

4. **Three gates, and all three are required.** A page switch that closes only
   one of them is a page that is still on screen, still takes taps, or still
   holds `Tab` focus.

   - **Paint.** An off-page node's recorded commands become **empty**, every
     frame. `Demo::draw` sends everything in `self.order`, so leaving the old
     commands in place paints the old page over the new one.
   - **Hit test.** `set_visible(false)` on the off-page nodes, which
     `hit_test` honours for free — **plus** the three pointer helpers that
     bypass the router entirely: `Demo::pad_at`, `Demo::slider_at` and
     `Demo::keyboard_at` are called straight from the `MouseButtonDown`,
     `MouseButtonUp`, `FingerDown` and `FingerUp` arms of
     `Demo::handle_event`, and never reach `Demo::route_input_event`.
   - **Focus.** `Demo::focus_navigation` builds its focusable set from a
     hardcoded five-element array today; it becomes the active page's controls
     **plus the six tab buttons**. A switch also retires `Demo::focused`, or a
     control on the page just left keeps its focus ring.

5. **Layout is per page, and the geometry tests say so.** The `+64` shift is
   applied uniformly, so nothing moves relative to anything else and the
   existing collision checks keep their meaning *within* a page. What changes is
   their **scope**: `no_two_placed_rects_overlap` and
   `every_placed_rect_is_inside_the_window` run **over every page**, rather than
   once over the union. That is a stronger assertion for each page and it stops
   asserting something that is no longer true — that twenty-six widgets coexist
   in one window without touching, which was only ever true because they all had
   to.

6. **A shortcut activates its own page, then acts.** All eighteen rows of
   `GALLERY_SHORTCUTS` keep working from any page, and no
   keypress has an invisible effect. The page is a field on the row, so the
   table stays the single dispatch — the property the 2026-10-03 review
   installed when a `match` and a list of its keys were found to disagree.

7. **The frame rate is measured on every page**, with
   `.ai/tools/fps-check.sh`, and the numbers go in the handoff whether they are
   good or bad. A per-page regression is invisible to `cargo test` and to a
   capture: a still of a 4 fps application is pixel-identical to a still of a
   60 fps one, which is how a four-fps regression survived three reviews here.

8. **The bar is unreachable while the dialog is up.** The dialog takes every
   tap and every key, so the tab bar is under the scrim with everything else.
   That is the existing decision of 2026-10-03 — *"a modal which leaves the host
   application's shortcuts live is not modal"* — and this adds the visible bar
   to what it covers.

## Acceptance Criteria

- [ ] `ui_demo` runs and every one of the six pages is reachable by clicking
      its button
- [ ] `ui_demo --tab=<name>` opens on that page, for all six names
- [ ] `ui_demo --tab=nope` exits non-zero and prints the six valid names
- [ ] `ui_demo --help` prints the usage and the six names, and exits 0
- [ ] A page shows **only** its own widgets — asserted per page, over the
      recorded draw commands
- [ ] A tab button shows a selected appearance, and it **animates** on a switch
      rather than jumping
- [ ] A press on an off-page control does nothing, for every control that
      takes a press
- [ ] `Tab` walks the six tab buttons and the active page's controls, and
      nothing else
- [ ] Every one of the eighteen gallery shortcuts works from every page
- [ ] The frame-rate readout is visible on **every** page
- [ ] The dialog still takes every tap and every key while it is showing, and
      the tab bar with them
- [ ] No two placed rects overlap and every placed rect is inside the window,
      **on every page**
- [ ] `fps-check.sh` is at or above the recorded baseline on all six pages
- [ ] The press transition, the release transition, the hover tint, the focus
      ring and the click callback are **on screen again** — the debt the
      `ui_demo` module doc records

## Out of Scope

- **A `TabBar` or page widget in `ui_core`.** This is a demo-level mechanism
  and `DEMO_APPLICATION.md` gaps #3 and #7 stay open. Closing #3 is a library
  task with its own cycle, and the operator chose the demo-level route on
  2026-10-03.
- **The Tesla application.** It is one more tab by the operator's decision, and
  `DEMO_APPLICATION.md` owns its scope. Nothing here is written to be
  Tesla-shaped, and nothing here contradicts it either.
- **Animating the page switch itself.** `DEMO_APPLICATION.md` gap #8 records
  that `Transform` is `Interpolate`-able and then never drawn — there is no
  matrix and no `u_model` uniform — so a slide or a cross-fade between pages is
  not buildable. A switch is instantaneous, and the *tab button* animates.
- **Restructuring the shortcuts.** The table stays as it is; requirement 6 adds
  a field to a row.
- **Migrating `ROADOS_RUN_SECONDS` and `ROADOS_ASSET_DIR` to arguments.**
- **Widgets the crate does not have**: a `List` back on screen, an app-launcher
  grid, drag-to-dismiss, horizontal scrolling. **`toast` is no longer on this
  list** — it landed with task 23 and the `overlays` page carries it.

## Sub-tasks

Split per `.ai/agents/developer.md` § *Scope check*, which trips at **more than
5 files** or **more than 3 independent components** — and this task trips a third
count as well. `ui_demo/src/main.rs` measured **12,820 lines and 157 `#[test]`
functions** at 2026-10-03 16:36 (`wc -l`, and
`grep -c '^    #\[test\]' ui/src/ui_demo/src/main.rs`; both figures move while
the tree is being edited, so **re-measure rather than trust this one**). **Sub-tasks are sequential, not parallel**, because each reads what
the one before wrote.

| # | Sub-task | Why it needs the one before |
|---|---|---|
| 24.1 | `Page`, `--tab=`, and the three gates. No pixel moves. | — |
| 24.2 | `CONTENT_TOP`, and the text-entry band goes page-local | 24.1's page set decides which band belongs to which page |
| 24.3 | The tab bar itself | 24.2 makes room for it; 24.1 gives it a page to switch |

**The order is chosen so the pixel-moving sub-task comes last.** 24.1 changes
what is drawn on launch and nothing else, so every capture taken for tasks
11–22 is still a capture of the same pixels until 24.2 — and 24.1 is what makes
them addressable (`--tab=<page>`) once it is not.