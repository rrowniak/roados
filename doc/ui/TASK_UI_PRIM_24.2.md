# TASK_UI_PRIM_24.2: `CONTENT_TOP`, and the band goes page-local

## Goal

Make room at the top of the window for the tab bar, by shifting the gallery
down by the bar's height, and stop treating the whole widget set as one canvas
that has to fit at once.

## Context

Parent: `doc/ui/TASK_UI_PRIM_24.md`. Its requirement 2 wants a 64-pixel bar at
the top; this task is what makes that possible without anything overlapping.

**The window cannot grow, and that is measured.** `WINDOW` is 1280 × 1020
(`WINDOW`, `ui/src/ui_demo/src/main.rs`). A 1280 × 1160 request came back
**1280 × 1052** — the window manager's cap on this host, which has two stacked
displays — and the demo was built to 1020 to stay under it. **A window taller
than the cap is not merely awkward, it is unverifiable**: the bottom of the
keyboard would never reach the screen, so the capture meant to prove the widget
draws could not see it. `WINDOW.height` therefore **does not change in this
task**, and requirement 4 below is what pays for the bar instead.

**The arithmetic, so nobody re-derives it wrongly.** The gallery's lowest
elements are the progress bar at 668…712 and the frame-rate readout, whose own
origin is `FPS_READOUT_ORIGIN` = (60, 684). Shifting the gallery down by 64 puts
the bar at 732…776. The text-entry band starts at `BAND_TOP` = 720
(`BAND_TOP`) and its keyboard ends at `KEYBOARD_ORIGIN.1 +
KEYBOARD_HEIGHT` = 736 + 260 = **996**.

- 776 > 720, so **the shifted gallery and the unshifted band overlap.** That is
  the whole reason this task is not one constant.
- 996 + 64 = **1060 > 1020**, so shifting the band as well puts the keyboard
  forty pixels off the bottom of the window.

**Both are symptoms of one thing, and this task's real content is removing
it.** Twenty-six widgets in one window with nowhere to put any of them is the
crowding the operator resolved on 2026-10-02 by deleting the list
(*"Remove some existing widgets like list or so. (Keep fps label.)"*), and the
collision tests are what forced the slider's thumb radius to 18 instead of the
22 that a 44-pixel knob needs (`SLIDER_THUMB_RADIUS`'s doc records that ceiling as
unretested since the buttons went). **With pages, the constraint is per page
rather than global**, and the `input` page has 956 pixels of content area for a
260-pixel keyboard.

**Why the shift is applied where offsets are written and not to the constants.**
Every origin in the file is a literal like `(664.0, 396.0)` and the docs on each
one explain what it clears. Adding 64 to the constants would make every one of
those explanations a lie by 64 pixels. Adding `CONTENT_TOP` at the five places
that call `Offset::new` keeps each constant meaning what its doc says, and the
shift is then one line in one place per site.

**The pads card needs an explicit position it does not have today.** The row of
pads is a `Stack` child with no `set_position`, so it sits at the origin by
default. Every other positioned node is placed explicitly
(the placement loops in `Demo::new`). Giving the card an explicit `Offset::new(0.0,
CONTENT_TOP)` is what makes it consistent with the rest, and it is the one node
whose origin is currently implicit.

## Requirements

1. **`TAB_BAR_HEIGHT: f32 = 64.0`** and **`CONTENT_TOP: f32 = TAB_BAR_HEIGHT`**,
   with the tab buttons 44 tall at y 10 — the same numbers the parent's
   requirement 2 names, so 24.3 does not choose its own.

2. **`CONTENT_TOP` is added where the demo writes positions**, in
   `Demo::new`: the two `for (node, origin)` placement loops, the slider's own
   placement, the text panel's, the keyboard's and the pads card's new one.
   **The `*_ORIGIN` constants themselves are not edited**, so their doc comments
   stay true.

3. **The text-entry band becomes page-local.** `BAND_TOP` and `BAND_TOP_OF_BAND`
   stop being the literal 720 and are derived from `WINDOW.height` and the
   band's own height, so the band sits against the **bottom of the window**
   rather than against a number that happened to be right before the shift. The
   band's contents keep their side-by-side arrangement: a field over a
   300-tall keyboard needs 364 pixels of band and the budget is 300, which is
   why it is laid out that way at all.

4. **The two collision tests become per page, and this is a deliberate change of
   assertion.** `no_two_placed_rects_overlap` and
   `every_placed_rect_is_inside_the_window` currently run once over
   `Demo::placed_rects`, which is every positioned node in the demo. They run
   **over all six pages instead**: for each page, that page's rects do not
   overlap each other and every one of them is inside `WINDOW`.

   - **What this strengthens:** each page's own layout is checked where before
     only the union was.
   - **What it stops asserting:** that twenty-six widgets coexist in one window
     without touching. That was only ever true because they all had to, and it
     stops being true the moment a bar exists.
   - **It is recorded here as an acceptance-criterion change with its reason**,
     not made silently in a test.

5. **Every pinned geometry test is updated to the new numbers, not deleted.**
   Named, so none is missed: `the_gallery_above_the_band_is_where_it_was`,
   `no_text_label_reaches_under_the_controls`,
   `the_slider_is_the_last_thing_painted_in_the_band`,
   `the_readout_sits_below_the_text_column_and_left_of_the_controls`,
   `the_gauge_sits_above_the_slider_and_below_the_image`,
   `the_gauge_is_the_only_box_in_the_window_that_is_square`,
   `the_chart_sits_in_the_column_the_list_occupied_and_is_the_box_it_asks_for`,
   `the_slider_sits_clear_of_the_counter_the_buttons_and_the_text`,
   `the_image_stays_inside_the_window_and_clear_of_its_label_at_every_fit`,
   `the_new_widgets_sit_clear_of_the_things_already_in_the_window`,
   `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect`,
   and `the_plot_is_the_nodes_own_width_and_its_height_less_the_widgets_x_gutter`.
   **The last one is arithmetic and must not be edited by hand** — it reads the
     plot's edges out of the chart's own recorded paint, so it holds itself.

6. **`fps-check.sh` on all six pages**, because a per-page layout change is a
   per-page frame cost and the union is not what any one page draws.

## Acceptance Criteria

- [ ] No two placed rects overlap and every placed rect is inside `WINDOW`,
      **on each of the six pages**
- [ ] Every widget is **below** `CONTENT_TOP` except the tab bar, which is 24.3
      and absent here — so this criterion is checked as "nothing is drawn in the
      top 64 pixels", which is what makes room for it
- [ ] The frame-rate readout is inside the window and legible **on every page**
- [ ] The keyboard's own box is inside `WINDOW` on the `input` page, measured
      and not derived
- [ ] `WINDOW.height` is **unchanged at 1020**, asserted, because "the window
      cannot grow" is the constraint the whole task is built around
- [ ] `fps-check.sh` at or above the recorded baseline on all six pages
- [ ] No assertion was deleted; every test this task touched is named in the
      handoff
- [ ] A capture of each page shows the tab bar's 64 pixels empty and nothing
      drawn in them

## Deliberate breaks

1. `CONTENT_TOP` dropped from one placement loop → the gallery-overlap test
   fails on that page, and **only** on the pages that hold the moved widget.
2. The band left at the literal 720 → the keyboard-overflow assertion fails on
   the `input` page.
3. A per-page loop removed from the collision test and the union restored → it
   fails, which is what shows the change is a real assertion and not a
   relaxation.

## Out of Scope

- **The tab bar's buttons.** The bar's 64 pixels are reserved and empty; 24.3
  draws them.
- **Re-flowing a page's layout for looks.** Every origin keeps its relationship
  to every other; this task moves the set, it does not redesign it.
- **Compressing anything to make room** — a smaller keyboard, a smaller slider
  thumb, a narrower chart. The bar was paid for with the page split, not with
  the widgets.
- **`WINDOW.height`.** See the acceptance criteria.