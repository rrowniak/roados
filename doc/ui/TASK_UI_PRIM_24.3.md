# TASK_UI_PRIM_24.3: The tab bar

## Goal

Draw the tab bar: a row of six buttons across the top of the window, the
active one visibly selected, and clicking one switches to its page.

## Context

Parent: `doc/ui/TASK_UI_PRIM_24.md`. Depends on **24.1** for the page set, the
three gates and `--tab=`, and on **24.2** for the 64 pixels this draws in.

**There is no `TabBar` widget in `ui_core`, and building one was declined.**
`ui/src/ui_core/src/widgets/mod.rs`'s `pub mod` list holds fourteen modules and
a tab bar is not among them; `DEMO_APPLICATION.md` gap #7 (*"No TabBar/Dock
widget"*) records that *"the bottom dock can be built from `Button` +
`Container`"* and rates it **Low**, while gap #3 (*"No screen/navigation
system"*) rates the missing screen controller **High**. The operator chose on
2026-10-03 to build the page mechanism **in the demo** and to leave both library
gaps open. So this is a `Container` in `LayoutMode::row()` holding six
`Button`s, which is what gap #7 prescribes.

**`Button` has no selected state, and that is the whole of requirement 4.**
Its public properties are `label`, `background`, `foreground`, `border_radius`,
`on_click`, `focus_ring`, `padding_h`, `padding_v`, `font_size`, `hovered`,
`pressed`, `disabled`, `focused`, `scale`, `opacity`
(`Button` in `ui/src/ui_core/src/widgets/button.rs`) — **no `selected`**. The
demo supplies one, and the doc has to say so, because the alternative is a
reader opening that file looking for a property that is not there.

**A click cannot write `&mut self`, so the bar uses the demo's own two-hop.**
`Callback` is `Fn`, not `FnMut`
(`Callback`, `ui/src/ui_core/src/widgets/mod.rs`), so a button's handler can only be
given something it can write. The demo already has two of these:
`Demo::pending_key` for the keyboard and `Demo::pending_theme` for the dialog's
`OK`, each read by `Demo::offer_to` or drained at the end of
`Demo::handle_event`. **The bar's `pending_page` is the third**, and it is a
`Property<Option<Page>>` rather than a `bool` because the six handlers are one
closure and a `bool` cannot say which button fired it.

**A click on the bar has to switch a page while the dialog may be up, and the
answer is that it cannot.** The dialog is modal over everything — every tap and
every key goes to it, and `Demo::route_input_event`'s modal chain is the whole
of that mechanism — so the bar sits under the scrim like everything else. **That
is the existing decision of 2026-10-03**, *"a modal which leaves the host
application's shortcuts live is not modal"*, and this task adds the visible bar
to what it covers. The bar is reachable the moment the dialog closes, and the
dialog is reachable from the `overlays` page and by `D` from any page.

**What this task puts back on screen, which is worth stating in the handoff
rather than leaving for a reader to find.** The demo's module doc
(`ui/src/ui_demo/src/main.rs`'s module doc) records that three buttons were
removed on 2026-10-01, and what it cost: *"the press and release transition, the hover tint, the focus ring
and the click callback are no longer on screen anywhere, and `ui_demo` is the
only place in the repository where any of them was demonstrated."* Six tab
buttons put all five back, which is why this task is not only chrome.

## Requirements

1. **A `Container` in `LayoutMode::row()`** holding one `Button` per page, as
   the first child of the demo's root `Stack` so it paints over the background
   and under everything else. It carries the theme's `Surface` as its
   background and a `Padding`, so it is the demo's **second** container with a
   visible background and the padding is visible — the card of pads is the
   first.

2. **Six buttons, 44 tall at y 10**, laid out left to right in `Page::ALL`
   order, each labelled with `Page::name`. Their widths come from
   `Button::content_size`, measured through the demo's `TextMetrics` rather
   than guessed, so a longer name is never cut.

3. **The selected appearance is the demo's own**: the active page's button has
   `background` and `foreground` set to the theme's active pair, and every
   other button to its rest pair.

4. **The switch animates.** Writing the pair is followed by
   `Button::animate_to_state(Motion::from_theme(&theme))` on both the button
   that was selected and the one that now is, over `THEME_TRANSITION`'s 300 ms.
   **The same argument as the gauge's needle applies in reverse**: the aim is
   written once per switch, never per frame, because a per-frame aim restarts
   the animation every frame and the button creeps toward its target for ever.

5. **`pending_page` is drained at the end of `Demo::handle_event`**, after the
   event that fired it and not inside the handler — the reason
   `Demo::pending_theme` gives. A page switch in the middle of a dispatch would
   move rects under an event still being routed.

6. **The buttons are focusable and in the `Tab` order**, first, so `Tab` from
   anywhere reaches the bar before the page's own controls, and `Space` or
   `Enter` activates the focused one. **`Demo::press_pads_if_unfocused` already
   guards `Space` on focus**, so a focused tab button does not also press the
   three pads.

7. **A theme switch reaches the bar.** The bar's background is bound to the
   theme like everything else, so `T` re-themes it through the property graph
   with nothing told.

## Acceptance Criteria

- [ ] Six buttons render, in `Page::ALL` order, each labelled with its page's
      name
- [ ] Every button is at least 44 tall — the project's own touch-target floor —
      asserted over the laid-out rects, not read off the constant
- [ ] The active page's button has the selected background and every other has
      the rest one
- [ ] A switch **animates** the two buttons that changed: asserted through
      `Button::is_animating` after the switch and not after the animation ends
- [ ] Clicking a button switches to its page, **through `Demo::handle_event`**
- [ ] `Space` and `Enter` activate the focused tab button and switch page
- [ ] `Tab` walks the six buttons then the active page's controls, in that
      order, both directions, and nothing else
- [ ] The bar is unreachable while the dialog is showing, and reachable the
      moment it closes
- [ ] The bar's background follows `T`
- [ ] A capture shows the bar over the gallery, with its 64 pixels empty of
      anything else
- [ ] A capture shows a **pressed** button mid-transition — the one thing this
      task puts back that nothing else on screen demonstrates
- [ ] `fps-check.sh` at or above the recorded baseline, on the default page and
      on one non-default page

## Deliberate breaks

1. `animate_to_state` dropped from the switch → the animation assertion fails.
2. `pending_page` drained inside the handler rather than after it → the
   routing-order test fails.
3. The bar left out of the focusable set → the `Tab`-order test fails, and the
   click test still passes — which is the asymmetry the assertion exists for.
4. `Motion::from_theme` replaced with a fixed `Motion` → the duration assertion
   fails.

## Out of Scope

- **A `ui_core` `TabBar`.** See the Context; the operator chose the demo-level
  route and gaps #3 and #7 stay open.
- **A page-switch transition** — a slide, a cross-fade, a scale. `Transform` is
  `Interpolate`-able and then never drawn: `DEMO_APPLICATION.md` gap #8 records
  no matrix and no `u_model` uniform. The *button* animates; the page does not.
- **Icons in the buttons**, and `DEMO_APPLICATION.md` gap #4 records why: there
  is no `Icon` widget, `Polygon` is convex-only and there is no bezier, so
  icons are the least supported thing in the crate.
- **Keyboard shortcuts for the pages.** The eighteen existing shortcuts activate
  their own page; nothing new is bound.
- **The bottom dock**, which the Tesla direction wants and this does not.