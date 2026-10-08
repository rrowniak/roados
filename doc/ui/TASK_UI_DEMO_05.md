# TASK_UI_DEMO_05: The Card Carousel, the Callout Hotspots, and the Two-Axis Reshape

## Goal

Finish the car-status pane's contents in three sub-tasks, and be explicit about
which of them is built from a source and which is built from a decision:

- **05-1 — the card carousel and the per-field staleness.**
  `TASK_UI_DEMO_03` left a filled surface where a card row belongs. This builds a
  **swipeable pager with a dot pager**, three pages, snap and inertia from
  `TASK_UI_PRIM_46`, and the **Tire Pressure card's four readings at four
  positions, each with its own independent timestamp** — including the
  photograph's own pair, `42 psi / 16 minutes ago` beside `42 psi / 15 minutes
  ago`, which is the property the whole composite exists to show.
- **05-2 — the callout hotspots.** Leader-lined `Open Frunk` and `Open Trunk`
  buttons anchored to **positions on the rendered car**, a lock glyph floating
  above the roof and a charge-port glyph at the rear-left — **all four anchored in
  model space and projected through the car's own matrix**, so a hotspot stays on
  the thing it names while the car turns.
- **05-3 — the two-axis reshape.** The pane's width as a **discrete
  snap-and-tier axis** and the camera distance as a **continuous axis**, built on
  a sentence whose only source is **`[C]`** — snippet-level, corpus unreachable
  from this host. **What is implemented, what is declined, and why is the
  substance of this sub-task**, and § *The `[C]` sentence, and what survives it*
  is where the reasoning lives.

**No pointer event reaches this window on this host, so nothing here is verified
by a tap.** Every gesture has a key that drives the **same** write path, and every
gesture has a unit test over literal numbers.

## Context

### What this depends on, and the fact that none of it is built

| task | what this task needs from it |
|---|---|
| **`TASK_UI_DEMO_03`** | **`PaneRects`, in three separate ways.** `card_row` and `dots` are the two bands 05-1 fills; **`pane_rects(width) -> PaneRects` is 05-3's seam** — the reshape is one argument to a function task 03 already wrote and tested, which is why this task's file list contains no layout work; and **the car matrix, which must be one composition and not two.** Task 03's requirement 9 puts the composition *"in `Demo`'s own private helper"*. **This task requires that helper to be named `car_mvp(&self) -> Option<Mat4>`, and if task 03 named it otherwise 05 renames it in a one-line edit rather than writing a second composition** — two compositions of one camera is the second-copy rule with a different spelling, and the two would differ the first time someone changed one |
| **`TASK_UI_PRIM_44`** | **`Icon`** — `Icon::new(nodes, texture, source)`, `Icon::set_tint`, `Icon::paint(rect)`, `ICON_SIZE` — and the tint in `DrawCommand::Image`. 05-2's lock and charge-port glyphs, **which are two of the 33 baked Lucide PNGs task 39 committed**: `lock` and `plug-zap`. **This is the first consumer of those 66 files in the demo** |
| **`TASK_UI_PRIM_46`** | **`Scroll` with `Axis::Horizontal`, `set_content_width`, `set_snap_points`, `snap_to_points(rect, motion)`, `scroll_by`, `max_scroll_for`, `tick`.** 05-1's pager **is** a `Scroll` with three snap points; § *Requirements* 5 shows why it cannot be anything else and why the pages are full-width |
| **`TASK_UI_PRIM_45`** | **the per-node clip**, which 05-1 needs twice: the card at a page boundary is **half outside the pager's viewport**, and `L8`'s *Blocks* column names *"the carousel's page edges"* and `L5`'s names *"The card carousel"* |
| **`TASK_UI_PRIM_49`** | **`DrawCommand::Text`'s width and `Painter::text_measured`** — which is what makes a card's own text **clippable at the page edge** rather than running into the next card. Without it, `L8` is open and a page boundary is a place where two cards' text overlaps |
| `TASK_UI_PRIM_47` | **nothing here.** The two-level Media card and Track Mode's default page are both declined below, so `Segmented` and `ModeScope` are unused by this task |
| `TASK_UI_PRIM_34`, `35`, `36`, `37`, `38`, `39`, `40` | **`36`'s `Mat4::transform_point`, `perspective` and `Mat4::scaled`** — the projection and the zoom; and **`38`'s model bounds**, which are how 05-2's anchors are expressed. `37`'s five singleton mesh commands and `40`'s rotator are read, not changed |
| `TASK_UI_PRIM_41`, `42`, `43`, `48`, `50`, `51`, `52` | **nothing**, each for its own reason in tasks 03 and 04's tables |

### The evidence, read by section and row and never by line

- `DEMO_APPLICATION.md` § *The car-status pane* — the diagram's card row
  (`Tire Pressure`, `Navigate`, `Start FSD (greyed)`) and its *"· · · (page
  indicator)"*.
- § *Open questions* item 8 — *"Two cards are visible side by side in photo `01`
  with a 3-dot pager. Should the demo implement paging (the real behaviour) or a
  swipe-with-peek (cheaper, and what the photo's evidence supports equally
  well)?"* **This task answers it, and requirement 4 is the answer.**
- § *Composite widgets* **row 1** — *"Swipeable pages at the foot of the
  car-status pane; a dot pager; **each card is itself two-level** (the Media card
  reveals a source list on swipe-up [B]); the carousel's *default page* is
  mode-dependent — 'the G-Meter displays as the default card whenever you engage
  Track Mode' [B]; **the whole strip is dismissible and its recovery lives on a
  different affordance** [B]"*, and the demand *"Horizontal paging with snap +
  inertia; a page indicator; per-card internal scroll; **a card host that can hand
  off its own gesture**"*.
- § *Composite widgets* **row 2** — *"The Tire Pressure card carries **four
  readings positioned at the wheel they belong to, each with its own independent
  timestamp** — photo `01` shows `42 psi / 16 minutes ago` beside `42 psi / 15
  minutes ago` on the same card, plus a `Recommended Front: 42 / Rear: 42` block
  with no timestamp"*, and the demand *"A value+unit+relative-time tuple rendered
  as one unit, repeated with independent state, at four positions"*.
- § *Composite widgets* **row 3** — *"Leader lines drawn from a rendered object out
  to labelled buttons (`Open Frunk`, `Open Trunk`), plus a free-floating lock glyph
  above the roof and a charge-port glyph at the rear-left"*, and the demand *"Hit
  targets anchored to positions on a picture, with leader lines and labels,
  **revealed by a gesture on the picture**"*.
- § *Composite widgets* **row 16** — *"Horizontal drag = a **discrete snap that
  also changes the content tier** (split ↔ full-screen reveals road markings and
  objects); pinch = continuous zoom **[C]**"*, the demand *"One axis
  discrete-and-tiered, one axis continuous"*, and the row's own verdict: *"**Its
  only source is the visualisation-resize sentence, which is `[C]`** … **Snippet-
  level, not specified**: a task must re-verify it before treating the tier change
  or the pinch as a requirement"*.
- § *The car-status pane* — the resize sentence itself, tagged **`[C]`**, with the
  structural reason: Tesla's own wording, *"read in the indexed text of
  `tesla.com/ownersmanual/model3/en_us/Owners_Manual.pdf` rather than off an opened
  page, and **the corpus is unreachable from this host**"*. And the same section's
  `[A]` half: *"The **behaviour** is independently visible at `[A]` in photo `02`:
  the pane is narrower there than in photo `01`, because Controls is open."*
- § *Could not verify*, the **How many cards the carousel holds, and their order**
  row — *"The manual names 'Media, tire pressure data, trip information, and more'
  — in that order. Photo `01` shows a **3-dot pager** with Tire Pressure and
  Navigate visible, and a greyed `Start FSD` card at a different height. Pager
  count, total card count and page order are **unverified**."*
- § *What this means for the demo's shape* item 1 — *"The demo must have a map it
  can put things on top up … **gap #1** (the map widget, in the first table) is
  untestable without one."* **This is the reason 05-3 cannot implement the tier's
  content**, and § *The `[C]` sentence* says so in its own words.
- § *Asset requirements* — *"Tesla publishes no design tokens at all"*, and *"the
  asset inventory must produce them rather than transcribe them"*, which is why
  every number in 05-3's zoom bound is derived rather than chosen.

### The `[C]` sentence, and what survives it

**This is the binding constraint on 05-3 and it is handled in three steps: what
the sentence says, what survives its demotion, and what this task builds anyway.**

**The sentence**, verbatim from § *The car-status pane*:

> *"You can expand/condense the visualisation by dragging the car status area
> from side to side. Expanding the visualization displays more details about the
> roadway and its surroundings, including road markings, stop lights, objects
> (such as trash cans and poles). You can pinch to zoom in or out."* **[C]**

**What survives the demotion, and it is not nothing.**

1. **The width is a variable with at least two observed values, at `[A]`.** The
   same section says *"The **behaviour** is independently visible at `[A]` in photo
   `02`: the pane is narrower there than in photo `01`, because Controls is
   open."* **That is first-hand observational evidence of two widths, and it does
   not depend on the manual at all** — the demotion removed the manual, not the
   photographs.
2. **One of the two widths is written down at `[A]`**: *"Left ~40 % in photo
   `02`"*, which is `PANE_WIDTH`.
3. **The repository's own design demand is not a source claim.** § *Composite
   widgets* is introduced as *"the list the demo should be built to satisfy"* and
   row 16's demand is *"One axis discrete-and-tiered, one axis continuous"*.
   **A design demand is `[A]`-by-construction** — this repository wrote it — and it
   is what authorises building a two-axis reshape at all.

**What does not survive**, and is not built: the road markings, the stop lights
and the objects. **They are map layers and there is no map** — § *What this means
for the demo's shape* item 1 makes gap `#1` *"untestable without"* one, and
`TASK_UI_DEMO_03`'s § *Out of Scope* already recorded that the pane's centre is a
`ThemeToken::Background` rect and a hole. **A tier that reveals a map cannot be
built before the map, and this task says so rather than drawing three grey bars
and calling them stop lights.**

**The rule this task applies, stated so a later task can re-use or overrule it.**

> **A `[C]` sentence is not implemented because it is written down. It is
> implemented where a repository-owned decision exists *independently* of it, and
> the decision is named as the reason.** Three decisions qualify here, and they
> are listed in the order of their strength: `[A]`-observational evidence that the
> axis exists (the photograph pair); an `[A]` written value for one end of the
> axis (*"~40 %"*); and § *Composite widgets* row 16's own demand, which this
> repository wrote. **The tier's *content* has none of the three and is declined.**

**And what 05-3 explicitly does not treat as a requirement**, per row 16's own
verdict — **not the tier change's content, and not the pinch**. The pinch is
declined for a second reason that is *this host's* and is stated in requirement 12:
a pinch needs two pointers, no pointer event reaches this window, and
`TASK_UI_PRIM_40` requirement 3 already lists `InputEventKind::Pinch` among the
seven variants `Rotator::on_event` declines on the ground that *"a pinch is a zoom
and a zoom is a projection change"* — **this task makes the projection change and
still declines the gesture**, which is the honest separation of the two.

### The carousel's page count and order are unverified, and the choice is this one

**§ *Could not verify* says the count and the order are unverified, and two
sources disagree.** The manual-as-transcribed names *"Media, tire pressure data,
trip information, and more"* — three, in that order; photo `01` shows a **3-dot
pager**; and § *The car-status pane*'s diagram draws `Tire Pressure`, `Navigate`
and a greyed `Start FSD` — **three, and a different three.**

**This task ships three pages in the manual's order: `Media`, `Tire Pressure`,
`Trip`.** Three reasons, and the second is the one that settles it.

1. **The manual's three are the only set with a written source**, and §
   *Could not verify* gives that set in that order.
2. **The photograph's 3-dot pager is independent agreement on the *count*.** The
   diagram's three carry the same count and a different membership, so the count
   has two sources and the membership has one — **and the one with a source
   wins.**
3. **The diagram's `Start FSD` is greyed, and a greyed card is a disabled
   control, not a page.** So the diagram is describing a *state* — a card the
   vehicle cannot offer — rather than the strip's membership, which is why the two
   sources do not actually contradict each other so much as answer different
   questions. **The alternative is recorded rather than smoothed**: § *Out of
   Scope* names `Navigate` and `Start FSD` as the set this task declined and says
   why.

**Three `[B]` claims in row 1 are declined, and each is declined for its tag.**

| claim | tag | decision |
|---|---|---|
| the Media card reveals a source list on swipe-up | `[B]` | **declined.** A two-level card needs gesture arbitration between a horizontal pager and a vertical list inside one card — *"a card host that can hand off its own gesture"*, which is row 1's fourth demand. **With three one-level cards there is nothing to hand off**, so the arbitration has no case to be exercised on, and building it for a card that does not exist is building a mechanism on a `[B]` claim |
| the G-Meter is the default card in Track Mode | `[B]` | **declined, and it is arithmetically impossible here**: there is no G-Meter page among the three, so honouring it needs a fourth page whose existence rests only on a `[B]` sentence |
| the strip is dismissible and its recovery is elsewhere | `[B]` | **declined.** A dismissible strip whose recovery affordance is `[B]` would leave the carousel with **no way back** on a host where no pointer arrives — a control that can be dismissed and cannot be recovered is a control that disappears, and that is a defect rather than a `[B]` fidelity gain |

## Requirements

### 05-1 — the carousel and the per-field staleness

1. **A new module `ui/src/ui_demo/src/carousel.rs`**, declared `mod carousel;` in
   `ui/src/ui_demo/src/main.rs`, so the demo carries **three** modules.

2. **`pub struct Pages`, holding the three names and the three `Scroll`s' shared
   geometry.**

   ```rust
   /// The pager's own geometry and its three pages.
   ///
   /// **One page at a time and no peek, and the reason is arithmetic rather
   /// than taste.** `Scroll::set_snap_points` drops every point outside
   /// `0.0..=max_scroll` and `max_scroll.x = content_width - viewport_width`, so
   /// with a peeking pager — `page_width = viewport_width - PEEK`, strictly less
   /// than the viewport — the last page's snap point
   /// `(n - 1) * page_width` is **greater** than `n * page_width - viewport_width`
   /// for every `PEEK > 0`, and the last page becomes **unreachable**. A peek
   /// therefore needs pages of unequal width, or a settle outside `Scroll`, which
   /// is `L5`'s mechanism re-implemented in the demo.
   ///
   /// **And the photograph's two-visible is a mid-drag frame, not a resting
   /// one.** § *Open questions* item 8 asks paging or swipe-with-peek; row 1's
   /// *demands* column says *"Horizontal paging with snap + inertia"*, and the
   /// demand wins over a photograph of an instant.
   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   pub struct Pages {
       /// The pager's viewport: `PaneRects::card_row`.
       viewport: Rect,
   }
   ```

   with `#[must_use] pub fn page_width(&self) -> f32 { self.viewport.width }`,
   `#[must_use] pub fn content_width(&self) -> f32 { self.viewport.width *
   PAGE_COUNT as f32 }`, `#[must_use] pub fn snap_points(&self) -> [f32; 3]`, and
   **`PAGE_COUNT: usize = 3`** as a `pub const` **derived from the two sources'
   agreement on the count and not from the manual's list alone.**

3. **`#[must_use] pub fn page_of(offset: f32, page_width: f32) -> usize`, and the
   tie rule is `L5`'s.** `(offset + page_width * 0.5) / page_width` clamped into
   `0..PAGE_COUNT`, **and a tie going to the earlier page** — the same rule
   `TASK_UI_PRIM_46`'s `nearest_point` states, *cited rather than reinvented*, so
   the dot pager and the settle never disagree about which page a half-drag
   between two is on.

4. **Three pages, in this order, named by these constants:**
   `PAGE_MEDIA: usize = 0`, `PAGE_TYRES: usize = 1`, `PAGE_TRIP: usize = 2`, with
   `PAGE_LABELS: [&str; 3] = ["Media", "Tire Pressure", "Trip"]`. **The order is
   the manual-as-transcribed's and the doc comment cites § *Could not verify*'s
   carousel row for both the choice and its unverified status.**

5. **The pager is a `Scroll` and nothing else.** One `Scroll::new` on the pager's
   node, `set_axis(Axis::Horizontal)`, `set_content_width(pages.content_width())`,
   `set_snap_points(&pages.snap_points())`, and `Scroll::tick(rect, delta)` in
   `Demo::frame` **beside the existing ticks and in the same position among them**.
   **`Scroll::snap_to_points(rect, SNAP_MOTION)` is the settle**, and
   **task 46 supplies `SNAP_MOTION`'s own constant as private to `scroll.rs`**,
   so this task writes **its own `Motion { duration: Duration::from_millis(200),
   easing: Easing::EaseOut }`** with the same arithmetic in its doc comment and
   **the two numbers asserted `assert_ne!`-style in one test** — the shape task
   24.3 used when `Motion::from_theme`'s 150 ms and `THEME_TRANSITION`'s 300 ms
   turned out to be different numbers.
   **Two rows in `GALLERY_SHORTCUTS`** — **`N`** (next page) and **`U`** (previous)
   — each calling `Scroll::scroll_by(rect, Offset::new(±page_width, 0.0))` and then
   `snap_to_points`. **Both keys are free today.**

6. **The dot pager is drawn, not assembled.** `pub fn dots(rect: Rect, page: usize,
   theme: &Theme) -> Vec<DrawCommand>`, **one `DrawCommand::Circle` per page**, the
   selected one at `DOT_ACTIVE_RADIUS` and the unselected at `DOT_RADIUS`, both
   **filled with `Severity::Grey` and `Severity::Blue`** — **the same two tokens
   `TASK_UI_DEMO_04`'s `Severity::token` returns**, so a theme switch moves the
   pager and the column together and **there is no second colour vocabulary in the
   demo**. **`DOT_RADIUS = 3.0` and `DOT_ACTIVE_RADIUS = 5.0`, and the doc comment
   says both are proposals** on § *Asset requirements*.

7. **`pub enum Corner` and `pub struct TyreReading` and `pub struct Tyres`, and the
   age is stored.**

   ```rust
   /// One tyre's reading: a value, its unit, and how long ago it was taken.
   ///
   /// **The age is stored rather than read from a clock, and that is the whole
   /// reason composite row 2 is demonstrable.** The photograph shows
   /// `42 psi / 16 minutes ago` beside `42 psi / 15 minutes ago` **on one card**
   /// — two equal values and two different ages — so each reading has to carry
   /// its own. A formatter that asked `Instant::now()` would need a wall clock,
   /// and `AGENTS.md` forbids a test that needs one.
   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   pub struct TyreReading {
       pub corner: Corner,
       pub psi: u32,
       pub age: Duration,
   }
   ```

   with `Tyres::demo()`, `Tyres::age(&mut self, delta: Duration) -> bool`
   (**all four ages advance together and their *differences* are what the card
   shows, so the demo seeds them with different offsets** — `16`, `15`, `16` and
   `17` minutes, **which is the photograph's own pair**), `#[must_use] fn reading
   (&self, corner: Corner) -> Option<&TyreReading>`, `#[must_use] fn line(&self,
   corner: Corner) -> Option<String>` and **`#[must_use] fn recommended(&self)
   -> String` returning `"Recommended Front: 42 / Rear: 42"`**.

8. **`pub fn relative_age(age: Duration) -> String`, `#[must_use]`, and it is a
   pure function of a duration.** `"just now"` under a minute, `"N minutes ago"`
   to 59, `"N hours ago"` to 23, `"N days ago"` beyond — **with the thresholds as
   named `pub const`s** and the seconds-never-shown decision stated: *"16 minutes
   ago"* and not *"16 min 4 s ago"*, because the photograph's own strings are
   minute-resolution and **a sub-minute rendering would make two readings
   indistinguishable at a glance, which is the property row 2 exists to show.**

9. **The card content is one card host and three card bodies, each its own node
   under the pager's node**, so a page boundary is a clip and not an overlap:
   **three `DemoLabel`s for the tyre card's four readings is not enough** — four
   labels, one per corner, at the four quadrant positions of the card — **plus one
   for the recommended block, and two per non-tyre card.** Every label carries a
   `PageMember` row and a `placed_handles` row, **`assert_placed_handles_is_complete`
   being the instrument**: the demo's page table has been caught green with a row
   dropped, twice.

10. **05-1's tests, in `carousel.rs`'s `mod tests` and `main.rs`'s, with no display,
    no network, no filesystem and no wall clock:**

    - **`the_three_pages_snap_to_three_points_and_the_last_is_reachable`** — over
      `Scroll::snap_points` at a `Pages` whose `viewport` is 512 wide, asserting
      the three points are `[0.0, 512.0, 1024.0]`, that `max_scroll_for` is
      **exactly `1024.0`**, and that **every point is inside `0.0..=max_scroll`** —
      which is the arithmetic of § *Requirements* 2's doc comment, asserted rather
      than argued.
    - **`a_peeking_pager_would_put_its_last_page_out_of_range`** — the same
      assertion over `viewport.width - PEEK` for `PEEK` in `1..=64`, asserting
      that `(n - 1) * page_width > max_scroll` for each. **The counter-case beside
      the case**, and the test that stops a later reader "improving" the pager
      into an unreachable one.
    - **`page_of_rounds_a_half_drag_to_the_earlier_page`** — `0.0 → 0`,
      `0.49 * w → 0`, **`0.5 * w → 0`**, `0.51 * w → 1`, `2 * w → 2`, and
      **`3 * w → 2`** (clamped), with `w` a literal `512.0`.
    - **`two_equal_pressures_with_different_ages_render_different_lines`** — the
      photograph's own pair: two readings with `psi: 42` and ages of 16 and 15
      minutes, asserting the two strings differ, that both contain `"42 psi"`,
      and that the difference is the age and not the value. **This is composite
      row 2's whole claim, asserted on the pair it names.**
    - **`the_recommended_block_carries_no_timestamp`** — `recommended()` contains
      neither `"/"` nor `"ago"`, **and the four readings' lines each contain
      both**. **The "with no timestamp" half of row 2**, asserted as the complement
      of the same card's other four rows.
    - **`relative_age_is_a_pure_function_of_a_duration`** — at `0`, `59 s`,
      **`60 s`**, `16 min`, `59 min`, **`60 min`**, `23 h`, `24 h`, and a
      sub-millisecond duration, asserting the exact strings.
    - **`the_n_and_u_keys_move_the_pager_one_page_and_stop_at_the_ends`** —
      `demo_on(Page::Demo)`, `handle_event` with a constructed
      `Event::KeyDown` for `N` three times and then `U` four times, asserting the
      page index sequence `0 1 2 2 1 0 0` — **the clamp at both ends is in the
      expected sequence**, because a pager that runs off its end is the failure
      and a test that only checked the middle would pass against one.
    - **`the_dot_pager_draws_three_circles_and_the_selected_one_is_larger`** — over
      every page index, counting `DrawCommand::Circle` and asserting the selected
      index's radius is `DOT_ACTIVE_RADIUS` and the other two are `DOT_RADIUS`.

### 05-2 — the callout hotspots

11. **A new module `ui/src/ui_demo/src/hotspots.rs`**, declared `mod hotspots;`.

    ```rust
    /// One hotspot's anchor, **as fractions of the model's own bounding box**.
    ///
    /// **Fractions and not metres, and the reason is that the model's facing is
    /// recorded nowhere in this repository** — `TASK_UI_PRIM_40`'s `CAR_REST_YAW`
    /// doc says so in those words and settles the rest yaw's sign from the first
    /// capture. **A fraction of the bounding box is correct whichever way the
    /// nose points; a metre offset is correct in exactly one orientation**, and
    /// the demo's own ambient rotation means the orientation changes every frame.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct Hotspot {
        pub id: HotspotId,
        /// The anchor, each component a fraction of the bounding box's own
        /// extent on that axis, `0.0` at `bounds_min` and `1.0` at `bounds_max`.
        pub at: [f32; 3],
    }
    ```

    with `pub enum HotspotId { Frunk, Trunk, Lock, ChargePort }`, the four
    `HOTSPOTS: [Hotspot; 4]`, `#[must_use] fn world_point(&self, bounds: ([f32;
    3], [f32; 3])) -> [f32; 3]` (a lerp per component), and **`#[must_use] fn
    project(mvp: &Mat4, car_area: Rect, world: [f32; 3]) -> Option<Offset>`** —
    `mvp.transform_point(world)`, `None` for a point behind the eye, and otherwise
    `car_area.x + (ndc.x + 1.0) * 0.5 * car_area.width` on x and
    `car_area.y + (1.0 - ndc.y) * 0.5 * car_area.height` on y. **The `None` is the
    honest answer and is not consumed** — `Transform_point`'s own `Option` is
    `36`'s, and a hotspot whose anchor is behind the camera is drawn nowhere.

12. **Two leader-lined buttons and two free-floating icons, and the difference is
    a field.** `Hotspot` gains `leader: bool`. **`Frunk` and `Trunk` have it;
    `Lock` and `ChargePort` do not**, because § *Composite widgets* row 3 reads
    *"Leader lines drawn from a rendered object out to labelled buttons … plus a
    free-floating lock glyph above the roof and a charge-port glyph at the
    rear-left"*. **The leader is one `Painter::path(&[(x0, y0), (x1, y1)],
    LEADER_WIDTH, color, false)`**, and **`DrawCommand::Path` is *stroked* with an
    explicit width — which is exactly what a leader line is and is the reason it
    is available here at all**: a filled leader would be a `DrawCommand::Polygon`,
    which is convex-only (`L10`).
    **The two buttons are `Button`s** (`Open Frunk`, `Open Trunk`), **the two
    glyphs are `Icon`s** over `lock` and `plug-zap`, and **the charge-port glyph
    is offset above `PaneRects::charge_lamp`** — task 03's lamp is a `Circle` in
    the charging state and this task's glyph is an `Icon` **above** it, **with a
    test asserting the two rects do not intersect**, because two features at one
    point is the one overlap a reviewer cannot see on screen.

13. **`Hotspots::set_revealed(bool)` is the only write path, and it is what a
    gesture on the picture would call.** `#[must_use] pub fn revealed(&self) ->
    bool`, `#[must_use] pub fn at(&self, position: Offset) -> Option<HotspotId>`,
    `#[must_use] pub fn paint(&self, car_area: Rect, mvp: &Mat4) -> Vec<DrawCommand>`.
    **`at` is `Demo::slider_at`'s shape with the page gate first** — `if !self
    .on_show(node) { return None; }` then `if !self.indicators… revealed { return
    None; }` then the rect test — **because a hidden hotspot that is still
    hit-testable is the defect of a control with no
    route to it, in the other direction.** **One row in `GALLERY_SHORTCUTS`, `R`,
    toggling the reveal**, on row 3's *"revealed by a gesture on the picture"* —
    **the key and a future tap share one write path, and the key is what makes the
    capture possible.**

14. **05-2's tests:** `world_point_lands_inside_the_bounding_box_for_every_hotspot`
    (literal bounds, all four); `project_puts_a_point_behind_the_eye_nowhere` and
    `project_maps_the_ndc_corners_onto_the_car_area`'s four corners, **so a
    flipped y is caught**; `a_leader_is_a_stroked_path_and_not_a_polygon` (**the
    recorded command's variant is asserted, with `DrawCommand::Polygon`'s own
    convex-only doc as the reason**); `the_lock_and_the_charge_port_have_no_leader
    _and_the_two_buttons_have_one`; `the_charge_port_glyph_and_the_charge_lamp_do
    _not_intersect`; `at_finds_each_hotspot_at_its_own_anchor_and_nothing_else`
    (**every hotspot's own projected position, and a position 400 px away**);
    **`at_answers_none_while_the_hotspots_are_hidden`** — the control beside the
    half above; `the_r_key_reveals_them_through_handle_event`.

### 05-3 — the two-axis reshape

15. **A new module `ui/src/ui_demo/src/reshape.rs`**, declared `mod reshape;`, with
    the two axes in one type because composite row 16's demand is about the pair.

    ```rust
    /// The two states the pane's width rests at.
    ///
    /// **Two, and the discreteness is read off the document's own verb pair**
    /// — *"expand/condense"* names two states and not a continuum — with the
    /// photograph pair behind it: § *The car-status pane* records the pane as
    /// *"narrower"* in photo `02` than in photo `01`. **A verb pair is a
    /// switch**, and § *Composite widgets* row 16's demand says the horizontal
    /// axis is *"a discrete snap"*.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Tier { Split, Full }
    ```

    with `#[must_use] fn width(self) -> f32` — **`Split` is `PANE_WIDTH`, the
    documented *"~40 %"*; `Full` is `WINDOW.width`** — and `#[must_use] fn
    label(self) -> &'static str`.

16. **`pub struct Reshape`, four fields, and one value each:**

    ```rust
    pub struct Reshape {
        /// The live width, in pixels. **A `Property<f32>`**, because the settle
        /// animates it, and `Scroll::snap_to_points`'s own shape — an
        /// `animate_to` on the value being settled — is the precedent.
        width: Property<f32>,
        /// Which tier the width is heading for, and `None` while no drag is down.
        target: Option<Tier>,
        /// Where the pointer went down, so `drag_to` measures **from the start**
        /// and does not accumulate.
        origin: f32,
        /// The continuous axis: the camera distance's multiplier.
        zoom: Property<f32>,
    }
    ```

    `#[must_use] pub fn new(nodes: &mut Arena<WidgetNode>) -> Self` at
    `Tier::Split.width()` and zoom `1.0`; `#[must_use] pub fn tier(&self) -> Tier`;
    `pub fn begin_drag(&self, at: f32) -> bool`; **`pub fn drag_to(&self, at: f32)
    -> bool`, which is the entry point a `Drag` reaches and the entry point the
    unit tests reach**; `pub fn end_drag(&self, motion: Motion) -> bool`;
    `pub fn settle_to(&self, tier: Tier, motion: Motion) -> bool`;
    `pub fn step_tier(&self, motion: Motion) -> bool`;
    `#[must_use] pub fn zoom(&self) -> f32`; `pub fn set_zoom(&self, zoom: f32)
    -> bool`; `pub fn step_zoom(&self, up: bool) -> bool`.

17. **`DRAG_GAIN: f32 = 1.0` and `ZOOM_STEPS: [f32; 5] = [0.70, 0.80, 0.90, 1.00,
    1.10]`, both derived in their doc comments and neither chosen.**

    - **`DRAG_GAIN` is 1.0 because the sentence says *drag the car status area
      from side to side*** — a 1:1 follow is what that says — **and a gain would be
      an invented constant with no source**, which § *Asset requirements* records
      as the failure for every value nobody could verify. A 768-pixel drag reaches
      `Full` from `Split`, which is 60 % of the window and a comfortable one-hand
      travel.
    - **`ZOOM_STEPS`' upper bound is arithmetic.** The visible width at orbit
      radius *d* is `2 · d · tan(fov_y/2) · aspect`; with `d = CAR_DISTANCE / zoom =
      2.6 / zoom`, `fov_y = π/4` and `aspect = 1.4118`, that is
      `3.041 / zoom` metres, and **the car is 2.55 m long, so it is inside its
      rect for every `zoom ≤ 3.041 / (2.55 · 1.05) = 1.135`.** `1.10` is inside
      that with the 5 % margin visible in the number, and a test computes the
      visible width at every step and asserts `≥ 2.55`.

18. **`end_drag` settles to the nearer tier, a tie going to the narrower one**, on
    `TASK_UI_PRIM_46`'s `nearest_point` rule cited by name, **and it calls
    `width.animate_to(target, motion.duration, motion.easing)`** — the crate's
    *"aim once, tick per frame"* idiom, **not a per-frame assignment**, which is
    `Demo::sync_toggle_state`'s argument.

19. **Two rows in `GALLERY_SHORTCUTS`** — **`E`** (`step_tier`) and **`M`**
    (`step_zoom`) — **plus one arm in `Demo::route_input_event` beside task 40's
    rotator arm**, so a `Drag` over the pane reaches `begin_drag` / `drag_to` /
    `end_drag` and **the key and the drag share those three calls.**

20. **`Demo::frame`'s per-frame block grows by two lines and no more:** the
    pane's `pane_rects(self.reshape.width.get())` call where task 03's paint path
    passes the constant, and the `distance` in the mvp composition where task 03
    passes `CAR_DISTANCE` — **`orbit(yaw, pitch, CAR_DISTANCE / self.reshape.zoom
    .get())`**. **Task 03's `CAR_DISTANCE` and `orbit`'s signature are unchanged**,
    so the ambient rotation and this task's zoom are two multiplications of one
    distance and not two distances.

21. **The tier's published value, for whichever task draws the map.**
    `#[must_use] pub fn tier(&self) -> Tier` on `Demo`, and **the map task's
    obligation is named in § *Out of Scope* rather than fulfilled here**: a tier
    that reveals road markings needs a map, and § *What this means for the demo's
    shape* item 1 is the row that says so.

22. **05-3's tests:** `a_drag_to_the_right_settles_into_the_wide_tier_and_a_short
    _one_does_not` — **the two halves beside each other**, at 75 % and 25 % of the
    travel; **`a_tie_goes_to_the_narrower_tier`** — a drag landing exactly midway;
    `drag_to_writes_nothing_while_no_drag_is_down`; **`every_zoom_step_keeps_the
    _car_inside_its_rect`** — over `ZOOM_STEPS`, computing
    `2 * (CAR_DISTANCE / zoom) * tan(FRAC_PI_4 / 2) * aspect` and asserting
    `≥ 2.55`; **`set_zoom_rejects_a_value_outside_the_steps_and_step_zoom_never
    _leaves_them`**; `the_readout_stays_visible_at_both_tiers` — **`pane_rects` at
    `WINDOW.width` still ends at `FPS_READOUT_ORIGIN.1`**, because the tier changes
    the width and **not** the vertical extent, and the frame-rate readout is the
    instrument every other page shows; and `the_e_key_steps_the_tier_and_the_m_key
    _steps_the_zoom_through_handle_event`.

23. **A row added to `DEMO_APPLICATION.md` § *Could not verify*, named *"The
    car-status pane's two-axis reshape"*, with what was searched written honestly:**
    *"nothing further — `tesla.com` returns 403 and `rollout-tesla.com`'s GUID
    pages return Akamai's Access Denied, both re-tested 2026-10-05 per § *Sources*;
    **this task ran no search of its own**"* and, in the result column, the `[C]`
    demotion, the two `[A]`-observational facts that survived it, **which half this
    task built and which half it declined, and why**. **The row is added and no row
    is closed or renumbered**, and § *Composite widgets* **row 16** gains a dated
    note naming the same split — the second copy is required because that table is
    the one a reader of row 16 opens first, and a disagreement between the two is
    what `TASK_UI_PRIM_52`'s § *The gap is recorded twice* is about.

24. **`doc/ui/IMPLEMENTATION_STATE.md` gains one task section per sub-task**, each
    naming its own file, its review count and its waivers-or-none, **the
    page-count-and-order choice and the diagram it declined**, **the full-page
    arithmetic and the counter-case test**, **`DRAG_GAIN = 1.0`'s derivation and
    `ZOOM_STEPS`'**, **the `[C]` decision and the three facts that authorised
    it**, **the test count before and after each**, and **the frame rate for all
    seven pages**.

### Scope, measured against `developer.md` § *Scope check* — and the split this task needs

**As one change: eight files and twelve independent components, and both numbers
are over.** `developer.md` § *Scope check* says **more than 5 files** or **more
than 3 independent components** is too large for one agent, and
`.ai/protocols/subagents.md` § *Implementation fan-out* requires the split to have
**file isolation**, testable independence, and no hidden dependencies.

**Parallel fan-out is impossible here, and the reason is one fact: all three
sub-tasks edit `ui/src/ui_demo/src/main.rs`.** The protocol's own rule is *"A file
is the unit of isolation. If a task requires two changes to the same file, it is
one sub-task, not two"*, and § *Implementation fan-out*'s *Split along module
boundaries* says the same. So the split here is **sequential, not parallel** — each
sub-task is its own developer dispatch, its own handoff, its own review and its own
operator commit, and the next is briefed against the code the last one produced.
**That is the shape `TASK_UI_PRIM_24.1..3` already has**, and it is the honest
answer rather than a fan-out that would have two agents writing one file.

| sub-task | files | components | primitives it needs |
|---|---|---|---|
| **05-1** | `carousel.rs` (new), `main.rs`, `DEMO_APPLICATION.md`, `IMPLEMENTATION_STATE.md` — **4** | pager + snap, dot pager, staleness model, wiring — **4** | **`Scroll`** with an axis and snap points; **`DrawCommand::Text`'s width**; the per-node clip |
| **05-2** | `hotspots.rs` (new), `main.rs`, `DEMO_APPLICATION.md`, `IMPLEMENTATION_STATE.md` — **4** | projection, leader lines + buttons + glyphs, wiring — **3** | **`Mat4::transform_point`**; **`Icon`**; `Painter::path` |
| **05-3** | `reshape.rs` (new), `main.rs`, `DEMO_APPLICATION.md`, `IMPLEMENTATION_STATE.md` — **4** | tier state machine, zoom axis, wiring — **3** | **`Property::animate_to`**; `Motion`; task 03's `pane_rects` |

**05-1 is at four components and is therefore the one that needs a decision.** Its
four are not four subsystems: the dot pager is eight lines inside the pager's paint
and cannot be specified or tested apart from it, so counting it separately would be
counting a function rather than a component. **The honest count is three** — the
pager and its snap, the staleness model, the wiring and the record — and the
argument is recorded here rather than asserted silently. **If an implementer finds
the four separable, 05-1 splits again into `05-1a` (pager, dots, wiring) and
`05-1b` (the staleness model and the tyre card), which is a clean split on both
file sets and dependencies.**

**No sub-task touches `ui/src/ui_core/` at all.** Each is inside both thresholds,
and **if an implementer finds themselves editing a file under `ui/src/ui_core/`
that is a stop condition and not an expansion** (`developer.md` § *Stop
conditions*).

## Acceptance Criteria

- [ ] **Five rows reach `GALLERY_SHORTCUTS`, and none of them is a pointer.**
      `N`, `U`, `R`, `E` and `M` are rows with `Some(Page::Demo)`, so
      the array carries **28** entries, and `no_printable_key_acts_without_a_row
      _in_the_shortcut_table` and `the_gallery_shortcut_list_holds_every_key_the
      _table_has` both still hold. **Every criterion below that involves a gesture
      names a key or a unit test, and no criterion requires a pointer event**,
      because none can be delivered on this host.

- [ ] **The pager pages, snaps, and cannot run off either end.**
      `the_three_pages_snap_to_three_points_and_the_last_is_reachable` asserts the
      three snap points are `[0.0, 512.0, 1024.0]`, that `max_scroll_for` is
      **exactly** `1024.0` and that **every point is inside `0.0..=max_scroll`**.
      `the_n_and_u_keys_move_the_pager_one_page_and_stop_at_the_ends` asserts the
      sequence `0 1 2 2 1 0 0` — **the clamps are in the expected sequence**, not
      asserted separately. And `a_peeking_pager_would_put_its_last_page_out_of
      _range` sweeps `PEEK` in `1..=64` and asserts the counter-case, **so the
      full-page decision is held down by a test rather than by a doc comment.**

- [ ] **The three pages are the manual's three, and the diagram's three are named
      as the set this task declined.** `PAGE_LABELS` is `["Media", "Tire
      Pressure", "Trip"]`, `PAGE_COUNT` is `3`, and `DEMO_APPLICATION.md` §
      *Could not verify*'s **How many cards the carousel holds, and their order**
      row carries a dated note recording the choice, the **two sources' agreement
      on the count**, **the diagram's `Navigate` and `Start FSD` as the declined
      alternative**, and the fact that a greyed `Start FSD` is a disabled control
      rather than a page. **The row is annotated and not closed.**

- [ ] **Two equal pressures with different ages render two different lines.**
      `two_equal_pressures_with_different_ages_render_different_lines` asserts the
      photograph's own pair — `42 psi` at 16 minutes and `42 psi` at 15 minutes —
      produces two strings that differ, both containing `"42 psi"`, and that the
      difference is the age. **The four readings carry seeded ages of 16, 15, 16
      and 17 minutes, and the card's capture shows two adjacent rows with the same
      value and different ages.** `the_recommended_block_carries_no_timestamp`
      asserts the complement — the recommended block holds neither `"/"` nor
      `"ago"` while each of the four readings holds both.

- [ ] **A hotspot stays on the thing it names while the car turns.**
      `project_puts_a_point_behind_the_eye_nowhere` and the NDC-corner test
      together pin the projection, **including the y flip**. The capture shows the
      four hotspots on the car, and a second capture taken after the ambient
      rotation has advanced shows them **still on the car** — which is the
      projection's claim and not a fixed-rect claim. `at_finds_each_hotspot_at_its
      _own_anchor_and_nothing_else` asserts each of the four is found at its own
      projected position and that a point 400 px away is not, **and
      `at_answers_none_while_the_hotspots_are_hidden` is the control beside it.**

- [ ] **Two leader-lined buttons, two free-floating glyphs, and one polygon-free
      leader.** `a_leader_is_a_stroked_path_and_not_a_polygon` asserts the recorded
      command's variant is `DrawCommand::Path` and **not** `DrawCommand::Polygon`,
      on `L10`'s convex-only rule. `the_lock_and_the_charge_port_have_no_leader
      _and_the_two_buttons_have_one` holds the two kinds apart. And `the_charge
      _port_glyph_and_the_charge_lamp_do_not_intersect` asserts 05-2's glyph and
      `TASK_UI_DEMO_03`'s lamp occupy disjoint rects, **because two features at one
      point is the one overlap a reviewer cannot see.**

- [ ] **The hotspot reveal is one write path, and the key drives it.**
      `Hotspots::set_revealed` is the only writer; `R` is the only key that calls
      it; **and a capture taken before `R` shows no hotspot and one taken after
      shows four** — the pair, because one capture of a thing that did not change
      looks exactly like one capture of a thing that did not, and only the pair
      answers it.

- [ ] **The reshape's two axes exist, and both bounds are arithmetic.**
      `every_zoom_step_keeps_the_car_inside_its_rect` computes
      `2 · (CAR_DISTANCE / zoom) · tan(π/8) · aspect` for each of the five
      `ZOOM_STEPS` and asserts `≥ 2.55`, with the `1.13` bound in
      `ZOOM_STEPS`'s doc comment. `a_drag_to_the_right_settles_into_the_wide_tier
      _and_a_short_one_does_not` puts the 75 % and 25 % cases **beside each other**,
      `a_tie_goes_to_the_narrower_tier` pins `L5`'s tie rule, and
      `set_zoom_rejects_a_value_outside_the_steps_and_step_zoom_never_leaves_them`
      holds the clamp. **`DRAG_GAIN` is `1.0` and its doc comment says a gain would
      be an invented constant with no source.**

- [ ] **The `[C]` decision is written down in both places that hold it, in the same
      words.** `DEMO_APPLICATION.md` carries **a new row in § *Could not verify***
      — *"The car-status pane's two-axis reshape"* — whose *Searched* column says
      *"nothing further … **this task ran no search of its own**"* and whose
      *Result* column names the `[C]` demotion, **the two `[A]`-observational facts
      that survived it** (the photograph pair's two widths, and the written *"~40
      %"*), **the three repository-owned decisions that authorised the build**, and
      **what was declined and why**; **and a dated note on § *Composite widgets*
      row 16** carrying the same split. **The two cannot disagree**, which is the
      whole point of writing it twice, and **neither row is closed or renumbered.**

- [ ] **The tier's content is not built, and the handoff says so.**
      **No road markings, no stop lights and no objects appear anywhere**, because
      they are map layers and § *What this means for the demo's shape* item 1 makes
      gap `#1` untestable without a map. What the tier **does** change is the
      pane's width, the car area's width and therefore the car's rect **through
      `pane_rects`**, and `Reshape::tier()` is published for whichever task draws
      the map. **The handoff states that half of composite row 16's content claim
      is not met, and why, in those words.**

- [ ] **The pinch is not implemented, and the reason is two-fold.** No
      `InputEventKind::Pinch` reaches anything: the sentence is `[C]`, and **a
      pinch needs two pointers and this host has no pointer** — while
      `TASK_UI_PRIM_40` requirement 3 already declines `Pinch` on the ground that
      *"a pinch is a zoom and a zoom is a projection change"*, **and this task
      makes that projection change anyway.** The two are separated deliberately and
      the handoff says which is which.

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: `the_three_pages_snap_to_three_points
      _and_the_last_is_reachable`, `a_peeking_pager_would_put_its_last_page_out
      _of_range`, `page_of_rounds_a_half_drag_to_the_earlier_page`,
      `two_equal_pressures_with_different_ages_render_different_lines`,
      `the_recommended_block_carries_no_timestamp`,
      `relative_age_is_a_pure_function_of_a_duration`,
      `the_n_and_u_keys_move_the_pager_one_page_and_stop_at_the_ends`,
      `the_dot_pager_draws_three_circles_and_the_selected_one_is_larger`,
      `world_point_lands_inside_the_bounding_box_for_every_hotspot`,
      `project_puts_a_point_behind_the_eye_nowhere`, `project_maps_the_ndc_corners
      _onto_the_car_area`, `a_leader_is_a_stroked_path_and_not_a_polygon`,
      `the_lock_and_the_charge_port_have_no_leader_and_the_two_buttons_have_one`,
      `the_charge_port_glyph_and_the_charge_lamp_do_not_intersect`,
      `at_finds_each_hotspot_at_its_own_anchor_and_nothing_else`,
      `at_answers_none_while_the_hotspots_are_hidden`,
      `the_r_key_reveals_them_through_handle_event`,
      `a_drag_to_the_right_settles_into_the_wide_tier_and_a_short_one_does_not`,
      `a_tie_goes_to_the_narrower_tier`,
      `drag_to_writes_nothing_while_no_drag_is_down`,
      `every_zoom_step_keeps_the_car_inside_its_rect`,
      `set_zoom_rejects_a_value_outside_the_steps_and_step_zoom_never_leaves_them`,
      `the_readout_stays_visible_at_both_tiers`,
      `the_e_key_steps_the_tier_and_the_m_key_steps_the_zoom_through_handle_event`
      — **twenty-four**, against a measured baseline of **1894 (1450 `ui_core` +
      224 `ui_demo` + 220 doctests)**, each count pasted, **each higher than the
      baseline by the number of tests added in it**, **no test deleted, renamed
      away or weakened**, and the three sub-tasks' counts reported **separately**,
      because a single total across three sub-tasks cannot show which one lost a
      test. `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and `cargo doc
      --no-deps` clean; `cargo audit` **recorded as not installed on this host, not
      passed**.

- [ ] **The seven pages are captured, and each sub-task's capture is compared with
      its own before.** The commands of `IMPLEMENTATION_STATE.md` § *Verifying a
      change that draws — the capture method* verbatim, window id **re-read at the
      time of each capture**, `pgrep -a -x ui_demo` in the same call as each `magick
      import`, `magick compare -metric AE` per page. **AE 0 outside `y ≥ 680` on
      the six unchanged pages**, and on the seventh page: 05-1's pager sits on a
      changed card row and the other six pages are unaffected; **05-2's hotspots
      appear only after `R`** and the pair of captures is the evidence; **05-3's
      tier changes the pane's width and nothing else**, which a before/after pair
      at the same tier shows and a single capture cannot.

- [ ] **The frame rate is measured on all seven pages after each sub-task**, with
      `.ai/tools/fps-check.sh 10 55` on the default page — **the only thing the
      script can do** — and `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo
      --tab=demo` for each of the seven, the `roados-fps` line parsed by hand.
      **Every page above 55 after every sub-task**, **and each sub-task's number
      reported against the previous sub-task's** rather than against the band: a
      carousel adds per-frame work on the seventh page and nowhere else, and
      **`developer.md` § Phase 3's reason stands — a still of a 4 fps application
      is pixel-identical to a still of a 60 fps one.**

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **three new files** — `carousel.rs`, `hotspots.rs`,
      `reshape.rs` — and changes to `main.rs`, `DEMO_APPLICATION.md` and
      `IMPLEMENTATION_STATE.md`, **and no change to any file under
      `ui/src/ui_core/`**: every primitive used is one tasks 45, 46 and 49 already
      shipped. `ui/Cargo.toml` and `ui/Cargo.lock` are unchanged; the approved
      direct dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`.
      **No `unsafe`, no `unwrap`, no `expect`, no `panic!`, no `unimplemented!`,
      no `todo!`** — and **`project` returns `Option<Offset>` rather than drawing a
      hotspot whose anchor is behind the camera**. **`Page::ALL` is seven and
      `Page::DEFAULT` is still `Page::Pads`,** and **no `--tab=` name is added.**
      **`TASK_UI_DEMO_03`'s `CAR_DISTANCE`, `CAR_TARGET_Y`, `CAR_REST_YAW`,
      `CAR_REST_PITCH` and `AMBIENT_YAW_RATE` are unchanged** — one distance, one
      camera, two multipliers.

- [ ] **What the handoff does not claim, in those words.** It states that **the
      page count and order are unverified** and that the three pages are a choice,
      not a transcription. It states that **the two-level Media card, Track Mode's
      default page and the strip's dismissibility are all `[B]` and all declined**,
      and that a dismissible strip with a `[B]` recovery affordance would leave the
      carousel with no way back. It states that **half of composite row 16's
      content claim is not met**, that **the pinch is not implemented**, and that
      **the reshape's mechanism is the demo's own construction justified by three
      named facts rather than by the `[C]` sentence**. It states that **the only
      producer of the continuous axis is a key stepper**, so the axis is continuous
      in type and not in input. It states that **no pointer event can reach this
      window**, so nothing here was verified by a tap. And it states that **the
      three sub-tasks were sequential and not a parallel fan-out, because all
      three edit `main.rs`** — which is `.ai/protocols/subagents.md`
      § *Implementation fan-out*'s file-isolation rule applied to a change where it
      cannot be met.

## Out of Scope

- **No map, and therefore no road markings, no stop lights and no objects.** Gap
  **#1** stays a `TASK_UI_DEMO_n` item per § *What this means for the demo's shape*
  item 1. **05-3's tier changes the pane's width and publishes `Reshape::tier()`;
  the content the `[C]` sentence says a wider pane reveals is not drawn**, and
  saying so is the sub-task's central honesty.
- **No pinch gesture.** `[C]`, and **it needs two pointers and this host has none**.
  The continuous axis is implemented as a camera distance with a key stepper; the
  gesture is not.
- **No two-level card and no swipe-up.** The Media card's source list is `[B]`,
  and *"a card host that can hand off its own gesture"* is row 1's fourth demand
  **with nothing to hand off** once the cards are one level. The `Media` card shows
  a title and a disabled source line and nothing else.
- **No mode-dependent default page and no G-Meter.** `[B]`, and there is no
  G-Meter page among the three. **Track Mode's tint is `TASK_UI_DEMO_03`'s** and
  05 changes nothing about it.
- **No dismissible strip.** `[B]`, and a strip that can be dismissed with no
  specified way back is a defect on this host rather than a fidelity gain.
- **No `Navigate` card and no `Start FSD` card.** The diagram's alternative,
  named and declined in § *Context* and in the § *Could not verify* note.
- **No per-card internal scroll.** A card one page wide with a fixed number of
  lines does not overflow, so the scroll has no case; `L8`'s width and `L5`'s snap
  are what make the *page* edge correct, which is what 05-1 does.
- **No unequal page widths and no peek.** § *Requirements* 2's arithmetic, held
  down by `a_peeking_pager_would_put_its_last_page_out_of_range`. Revisit when a
  peeking pager is wanted — **and the fix is a `Scroll` change, not a demo
  change**, because `set_snap_points` filtering out-of-range points is what makes
  the last page unreachable.
- **No wheel steering and no per-wheel rotation.** Task 40 recorded that there is
  no wheel spin, and this task adds none.
- **No hotspot for the tyres, the doors or the windows.** Four hotspots, because
  § *Composite widgets* row 3 names four. **Anything more is an asset and a
  placement this repository has no source for.**
- **No `Segmented`, no `Snapshot`, no `ThemeScope`, no `Margin`, no
      `LayoutMode::Grid`, no `polygon_is_convex` and no `Backdrop`.** Each is
  available from tasks 41, 47, 48, 50, 51, 52 and 51 and each is unused here, for
  the reason its own row gives.
- **No new dependency and no `unsafe`.** `ui/Cargo.toml` and `ui/Cargo.lock` are
  untouched. **Zero new `unsafe` blocks** — no GL, no FFI and no pointer.
- **Found in the tree and deliberately not fixed.** `TASK_UI_DEMO_03`'s matrix
  composition is described in its own requirement 9 as *"in `Demo`'s own private
  helper"* **without naming it**, and 05 needs that composition to be *one*. **The
  resolution is a one-line rename to `car_mvp`, recorded here rather than argued
  as a disagreement**, because a task file that renames another task's private
  helper to share it is normal integration and not a defect. **`TASK_UI_DEMO_01`'s
  page variant name and `--tab=` spelling remain this task's open input**, and the
  handoff fills both from 01 rather than inventing either.
