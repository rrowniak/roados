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

   **Amended 2026-10-04: the pads card's mechanism in this requirement, and in
   the Context's paragraph above it, does not work — `set_position` on a `Stack`
   child is a no-op.** The Context says the card is *"a `Stack` child with no
   `set_position`"* and asks for an explicit `Offset::new(0.0, CONTENT_TOP)`.
   **`arrange_stack` places every child at `Offset::ZERO` and never reads
   `position`** (`ui/src/ui_core/src/layout.rs`), and `set_position`'s own doc
   says it is *"the position an `Absolute` parent uses"*. The write compiled and
   the card stayed at `y: 0.0`. **What landed instead is the root container's
   mode: `LayoutMode::Stack` → `LayoutMode::Absolute`, one token, in `ui_demo`
   and not in `ui_core`.** It is behaviour-preserving for the root's other three
   children — `arrange_stack` and `arrange_absolute` differ in exactly one
   field, the origin, and an unpositioned child sits at the parent's origin in
   both modes — and it adds no node, so `page_members` needed no row. **The
   alternative was an `Absolute` wrapper node, which would have needed a row in
   `Demo::order`, a row in `page_members`, and a move of `card()`'s handle.**

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

   **Amended 2026-10-04: the list above is incomplete, by two, and the omission
   was found by a review rather than by this file.** Brace-matching every
   `#[test]` body against the 24.1 tree gives **nine** changed; subtract the five
   named here and the two named by requirement 4, and two are named by neither.
   **They are added here**, because the reason this list exists is *"named, so
   none is missed"*, and a list missing two defeats it:

   - **`the_pads_sit_inside_the_card_their_row_draws`** — a real repair, not a
     number change: it compared a **size** against an **absolute origin** (the
     `a rect's origin and a rect's extent are different numbers` trap), and both
     sides are now measured from the card's own origin.
   - **`the_sliders_knob_fits_its_column_at_the_top_of_its_range`** — carries
     `+ CONTENT_TOP` on the right-hand side of its headroom subtraction, and is
     **one of only two assertions that kill** dropping `CONTENT_TOP` from the
     slider-readout placement site.

   **And the FOUR tests this task added**, so AC 7's *"every test this task
   touched is named"* can be checked against the file rather than a handoff:
   `the_tab_bars_strip_holds_the_background_and_nothing_else`,
   `the_highest_ink_on_any_page_is_the_card_of_pads_at_the_tab_bar`,
   `the_band_is_at_the_bottom_of_the_window_and_its_overlap_with_the_gallery_is_cross_page`,
   and `inked_box_bounds_a_shadow_by_the_blur_modules_own_reach`. **Four, not
   three** — the first version of this amendment said three and a review caught
   it, which is the same defect it was written to fix.

6. **`fps-check.sh` on all six pages**, because a per-page layout change is a
   per-page frame cost and the union is not what any one page draws.

   **Amended 2026-10-04: this criterion named an instrument that cannot produce
   the evidence, and the amendment is to the tool's reach rather than to the
   measurement.** `.ai/tools/fps-check.sh` runs `./target/release/ui_demo` with
   **no `"$@"`**, and the demo reads only `ROADOS_RUN_SECONDS` and
   `ROADOS_ASSET_DIR` from the environment — so **no argument and no environment
   variable reaches the page, and the script can only ever measure `pads`.**
   Forwarding `"$@"` is an `.ai/` change and this task may not make one.

   **So the measurement is `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo
   --tab=<page>` with the demo's own `roados-fps` stdout line parsed by field and
   compared against the same floor of 55** — the same numbers, the same report
   line, the same comparison, reached by a command the named tool does not
   wrap. **The defect this records is the general one, not the tool's**: an
   acceptance criterion that names an instrument which cannot produce the
   evidence is not satisfied by producing the evidence another way, and this file
   carried the wording from 24.1 without anyone checking it. `.ai/NEVERAGAIN.md`
   has the entry. **The tool change is still owed and is not this task's.**

## Acceptance Criteria

- [ ] No two placed rects overlap and every placed rect is inside `WINDOW`,
      **on each of the six pages**
- [ ] Every widget is **below** `CONTENT_TOP` except the tab bar, which is 24.3
      and absent here — so this criterion is checked as "nothing is drawn in the
      top 64 pixels", which is what makes room for it

  **Amended 2026-10-04: met on five pages and NOT on `overlays`, where the strip
  holds the dialog's scrim.** Measured on a capture: rows 0..63 are one colour on
  every page — `srgb(18,18,18)` on five, **`gray(9)` on `overlays`** — and
  `gray(9)` is the window background *under the modal scrim*, which the dialog
  records as a full-window command.

  **Superseded as to the strip's contents by task 24.3, and recorded here rather
  than left to be found false: those two colours are what the strip held while
  this task reserved it. Since 24.3 drew the tab bar, rows 0..63 carry the bar's
  `Surface` — `srgb(30,30,30)` on `pads`, and `srgb(15,15,15)` on `overlays`,
  which is 30 under the scrim.** 24.3's `strip_excused` grew by the bar's seven
  nodes and `the_bar_and_the_background_are_the_only_thing_in_the_strip` is the
  test that now holds the strip's contents. **What this criterion established,
  and what 24.3 spent, is the room.** **Something is drawn in the strip there, and
  it is the scrim, not a widget that has escaped the shift.** The exemption is
  recorded in code as `strip_excused`'s dialog entry, and it is deliberate: the
  parent's requirement 8 puts the bar itself under the scrim on `overlays`, so
  **a bar drawn there is correct and 24.3 must carry the fact forward** rather
  than treat it as a collision. **This criterion is met as "nothing but the
  scrim", and it was not amended in the first pass.**
- [ ] The frame-rate readout is inside the window and legible **on every page**
- [ ] The keyboard's own box is inside `WINDOW` on the `input` page, measured
      and not derived
- [ ] `WINDOW.height` is **unchanged at 1020**, asserted, because "the window
      cannot grow" is the constraint the whole task is built around
- [ ] `fps-check.sh` at or above the recorded baseline on all six pages

  **Amended 2026-10-04, with requirement 6:** `fps-check.sh` cannot select a
  page, so this criterion is met by **`ROADOS_RUN_SECONDS=<n> ./target/release/
  ui_demo --tab=<page>` with the same `roados-fps` line parsed by field and the
  same floor of 55** — the tool's own measurement, reached by a command the tool
  does not wrap. **Not waived: the numbers are real and all six clear the
  floor.** The tool's inability to name a page is a real gap and it is still
  owed.
- [ ] No assertion was deleted; every test this task touched is named in the
      handoff

  **Amended 2026-10-04: TWO assertions were retired, not one, and this
  criterion is therefore met as "none deleted silently", not as written.**

  **1. `the_gallery_above_the_band_is_where_it_was`** dropped its `else` arm —
  *"every gallery rect ends above `BAND_TOP`"* — which **requirement 4 does not
  authorise**: that authorisation is scoped by name to
  `no_two_placed_rects_overlap` and `every_placed_rect_is_inside_the_window`,
  and this test sits under requirement 5, which says *"updated to the new
  numbers, not deleted"*. **It could not be replaced either, and that is the
  finding.** The old claim was a bound between the gallery and the band, and
  **the band is now `input`'s alone**, so the per-page form is *false*: on
  `controls` the progress bar ends at 776 against a band top of 680. There is no
  per-page equivalent to substitute, **and the two per-page collision tests do
  not carry the property either** — they compare a page's rects with each other
  and with the window, and neither mentions `BAND_TOP`. **Withdrawn outright
  rather than replaced.**

  **2. `the_chart_sits_in_the_column_the_list_occupied_and_is_the_box_it_asks_for`
  lost the same bound** — `assert!(chart.y + chart.height <= BAND_TOP)`, six
  lines. **The same claim on a `data` rect against an `input` band**, and the
  same arithmetic applies verbatim: the chart's bottom is 754 against a band top
  of 680. **This one *is* replaced**, and no coverage is lost: `inside(window,
  chart)` plus the per-page neighbour loop over `Data` say the stronger thing —
  that the chart is inside the window and clear of its page's other rects.

  **So the count is two, and the first version of this amendment said one.**
  That is recorded because AC 7 is the criterion making the claim, and a count
  in the file that undercounts is worse than no count.
- [ ] A capture of each page shows the tab bar's 64 pixels empty and nothing
      drawn in them

  **Amended 2026-10-04, and again 2026-10-05 as task 24.3 drew the bar: as with
  AC 2, "empty" meant the window's own background on five pages and the
  background under the modal scrim on `overlays` — and it no longer means
  either, because the strip now holds the tab bar's `Surface`
  (`srgb(30,30,30)`, `srgb(15,15,15)` under the scrim).** The
  capture requirement stands and was met on all six — **what the six show is one
  colour in rows 0..63 each**, and the difference between `srgb(18,18,18)` and
  `gray(9)` is the scrim and nothing else. `strip_excused` is the written-out
  answer to what is excused, and 24.3's own test will want it.

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