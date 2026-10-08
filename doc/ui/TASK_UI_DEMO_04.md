# TASK_UI_DEMO_04: The Indicator-Light Column — ~20 Conditions, Five Colours, Three Timing Rules, and One Latch

## Goal

Build the **indicator-light column** that `TASK_UI_DEMO_03` left as an empty box:
a **severity-ranked list of about twenty conditions**, painted as coloured discs
down the pane's left column, where **five colours** are slots rather than values,
**three conditions are distinguished by timing rather than by colour**, **one
lights that latch and clear only on an external event**, and **every light in
the column flashes briefly at power-up as a self-test and then goes out**.

This is a **state machine with an externally-triggered reset**, and the reset is
drawn in the chrome: the tyre tell-tale stays lit after you inflate until the
vehicle has been driven above **25 km/h** for a dwell, and **that dwell is the
only thing in this repository that can clear it**.

**The blink is a pure function of an elapsed `Duration`, and the latch is a
`bool` with one writer.** Nothing in the column reads a clock, a file or a
window, so every rule in it is unit-testable with literal durations — which is
the only reason the rules can be stated at all in a document.

## Context

### What this depends on, and the fact that none of it is built

**`TASK_UI_DEMO_03` is this task's hard prerequisite and its own file is absent
from the tree**, so this file states what it needs structurally rather than
citing it. Everything else is in the same position as task 03's table, and only
the rows that bear on the column are repeated here.

| task | what this task needs from it |
|---|---|
| **`TASK_UI_DEMO_03`** | **`PaneRects::indicators`, the box this column paints into, and `CarState` for the defaults.** Task 03's § *Requirements* 3 defines `pane_rects(width) -> PaneRects` with an `indicators` field, and its requirement 6 makes `Demo::set_car_state` **the one write path** with room for one added line. **This task adds exactly that line** and nothing else to it. **If `TASK_UI_DEMO_03` has not landed, this task cannot be started** — there is no box, and a column drawn into a rect that does not exist is a column no capture can photograph |
| **`TASK_UI_PRIM_44`** | `paint::untinted()` and the tint in `DrawCommand::Image`. **This task uses neither** — the column is `Circle`s and `Label`s, and § *Why no shipped widget covers this* says why the `Icon` widget is not the answer |
| **`TASK_UI_PRIM_45`** | **the per-node clip**, and the column's overflow case depends on it: § *Requirements* 7 pins a column taller than its box as **clipped**, not overlapping the card row |
| **`TASK_UI_PRIM_47`** | **nothing.** `ModeScope` is task 03's, for Track Mode; this column has no mode — it reads vehicle conditions and nothing else |
| **`TASK_UI_PRIM_49`** | `DrawCommand::Text`'s width and `Painter::text_measured`. **This task draws its names with `Label`s and not with measured runs** (requirement 6), so the width is what makes a truncated name **clippable** rather than overrunning the column — which is `L8`'s *Blocks* column, which reads *"Card content that overflows, the carousel's page edges"*, and a 132-pixel name beside a disc is card content that overflows |
| `TASK_UI_PRIM_34`, `35`, `36`, `37`, `38`, `39`, `40` | **nothing.** No mesh, no matrix, no model, no camera. The column is discs and labels |
| `TASK_UI_PRIM_41`, `42`, `43`, `46`, `48`, `50`, `51`, `52` | **nothing**, each for its own reason in task 03's table |

**One demo task and one document own the rest, and both are cited by section.**

### The evidence, read by section and row and never by line

- `DEMO_APPLICATION.md` § *The car-status pane* — *"The indicator column is a
  **severity-ranked list, not a status strip.**"* and *"The manual enumerates
  **~20 conditions** with their colours and their **timing** semantics [A]
  §Car Status"*. **This file's condition table is that enumeration, and it is
  twenty-three entries against the document's own "~20"** — the count is stated
  rather than rounded, because 23 is what the five bullets add up to and rounding
  it to 20 would hide the arithmetic.
- § *The car-status pane* — the **five slots and their contents**, verbatim:
  **Red** — *"brake fault, parking brake applied, **seat belt unfastened in an
  occupied seat**, airbag fault, door or trunk open, system failure"*;
  **Amber** — *"brake booster, ABS (brief flash at startup), parking-brake
  electrical, **tire pressure out of range**, ESC active (flashing), ESC off,
  power limited"*; **Green** — *"parking lights, low beam, ready to drive, battery
  low"*; **Blue** — *"high beam, high beam with Adaptive Headlights armed,
  snowflake (battery too cold)"*; **Grey** — *"Adaptive Headlights armed but
  dimmed, Vehicle Hold, pedestrian warning paused"*.
- § *The car-status pane* — *"Three of these are distinguished **by timing, not
  colour**: ABS flashes once at startup then faults if it stays; ESC flashes
  **while actively** correcting and goes solid if it is a fault; the tire-pressure
  tell-tale is **steady for low pressure and flashing for a sensor fault**."*
- § *The car-status pane* — *"The tire light also **latches**: it does not clear
  when you inflate, it clears only once you **"drive over 15 mph (25 km/h) for a
  short amount of time to activate the TPMS"** [A]. That is a state machine with an
  externally-triggered reset, drawn in the chrome."*
- § *Screen states of the car-status pane* — the **parked** bullet's *"Indicator
  lights **flash briefly at power-up** as a self-test and then go out [A]."*
- § *Corrections to the first sketch* — the row on the lighting icon: *"There is no
  'automatic lights armed' indicator … The glyph in the photo is one of the
  **high-beam** states — blue beams + `A` = Adaptive Headlights armed and high
  beams on; grey = armed, dimmed because light is ahead."* **This is the source of
  the three-lighting-conditions exclusivity in requirement 8**, and it is why
  those three are three entries in one severity rather than one entry with three
  appearances.
- § *Corrections to the first sketch* — the row on the seatbelt: *"it is one of
  three channels, not one. A red occupant-with-belt indicator light **in the
  cluster**, plus a bottom popup whose text and per-seat tap-to-mute are separate,
  plus the 'Fasten Seatbelt' label under the seatbelt graphic."* **The column's red
  seatbelt light is the first of those three channels and not the other two**, and
  the popup is composite row 13's and not this task's.
- § *Asset requirements* — *"the indicator column uses colour to encode severity
  and **timing** to encode fault-vs-condition. Those are **drawn from state, not
  selected from an atlas**."* **This is the rule that settles the disc**, and it
  is the sentence requirement 6 is written against.
- § *Asset requirements* — *"Tesla publishes **no design tokens at all** — no
  colours, no spacing, no radii"*, which is why every colour below is a theme
  token and every pixel figure a proposal.
- § *Could not verify*, the **Exact colours, spacing, radii** row — *"Tesla
  publishes no design tokens at all. Every measurement in the demo is a
  first-principles choice, not a transcribed value."*

### A conflict inside § *The car-status pane*, and which half wins

**The section's own diagram and the section's own bullet list disagree about one
colour.** The ASCII block draws *"🟢 high-beam A"* with a green circle, and the
bullet list six lines below reads *"**Blue** — high beam, high beam with Adaptive
Headlights armed, snowflake (battery too cold)"*.

**The bullet list decides, and the reason is what each one is.** The bullet list
is the manual's enumeration with a citation on it; the diagram is an ASCII sketch
whose glyphs are the document's author's shorthand typed into a monospace block,
and **a green circle in an ASCII diagram is not a colour specification** — §
*Could not verify*'s *Exact colours* row would say the same about any colour
anywhere, and here it says it about this one first. **The disagreement is
recorded here rather than silently resolved**, because a reader who notices it
needs to find that it was noticed, and § *Corrections to the first sketch* is the
precedent for exactly this shape of note.

**What that does not make true: that any of the five colours is from a source.**
Both halves of the conflict are about *which slot a condition sits in*, and the
five slots are the manual's. **The RGB each slot gets is this repository's**, and
requirement 4 says so in the code.

### Why no shipped widget covers this

**`Segmented`, `Toggle` and `Button` are named in this task's brief, and none of
them covers a column of twenty-three rows with five phases and a memory.** The
reasons are worth stating one by one, because the nearest widget is nearest in
only one respect at a time.

| widget | what it is | why it is not this |
|---|---|---|
| **`Button`** | a press target with a label and a hover/press/focus triple | **The cluster lights are not press targets.** § *Corrections to the first sketch* is explicit that the seatbelt light is one channel of three and that the *popup's per-seat rows* are the tappable ones; a button's whole reason is that a finger reaches it, and nothing here does. `Button` also owns an animation clock and a palette per state, and there is no state here for a button to be in |
| **`Toggle`** | two states and a sliding thumb, aimed once and ticked per frame | **A row has up to four observable phases — `Off`, `On`, `Flashing`, `Latched` — and a *severity*, and none of the four is the other.** A toggle's thumb is a position; a light's is a fraction of a period, and **a light whose "position" is `elapsed / 400 % 2` is not a position on a track** |
| **`Segmented`** | one value from a named set, with `set_choices` able to change the set while the value survives it | **`Segmented`'s shape is genuinely the right one for a *single* light** — `Off / On / Flashing` is a named set and a value in it — **and it is still wrong here for two reasons.** It is **one control with one value**, and this is twenty-three rows with twenty-three independent values; and its `set_choices` is `L6b`(c)'s enumeration story, which is about *one stored value surviving a re-enumeration*, whereas **these twenty-three values do not change their enumeration at all.** Reaching for `Segmented` twenty-three times would also give twenty-three nodes, twenty-three clocks and twenty-three focus stops for a column that is read-only |
| **`Gauge`** | an analogue with a needle and a fill | A needle is one value on a scale; **a column is a ranked list, and `Gauge`'s value is a single `f32`** |
| **`Label`** | **used**, for the name beside each disc | It draws text and owns no colour-by-state channel of its own |

**What is genuinely absent from the crate is a status light**: a small coloured
glyph whose lit-ness is a function of **time as well as state**, plus a container
that **orders its children by severity**. **The honest scope decision is
demo-local, and it is a decision rather than an omission** — three reasons, the
second of which is the one that settles it.

1. **One consumer.** `developer.md` § *Phase 2*'s *"No abstraction before the
   second use"* — a `StatusLight` in `ui_core` would have exactly one caller, and
   this repository has never shipped a widget for one caller.
2. **The rules are the demo's, not the platform's.** *"Flashes once at startup
   then faults if it stays"* and *"clears only once you drive over 15 mph"* are
   claims about **one vehicle's cluster**, carried at `[A]` from one manual. **A
   crate that encoded them would be claiming Tesla's semantics as a library
   contract**, and § *Could not verify* already records that no token, metric or
   protocol in this interface is transcribed.
3. **The counter-argument, recorded because it cuts against the decision.** The
   defroster in composite row 11 needs the same *snapshot and restore* shape, and
   `TASK_UI_PRIM_47` § *Out of Scope* and `snapshot::Snapshot` are the library's
   answer to that one. **This column's latch is not a snapshot** — a snapshot
   captures values to put back, and the latch is a memory with an external
   trigger, which is a different mechanism with a different owner. **If a second
   consumer needing "a flag that clears on an external event" appears, this is the
   moment to promote it**, and this task names the promotion rather than performing
   it.

### The blink, the latch and the self-test, as three rules over one clock

**One clock, three rules, and none of them is a timer object.**

- **`BLINK_HALF_MS` is the only duration the blink has.** One number rather than
  an on and an off, because the lit fraction is then a *consequence* — the
  period is `2 × BLINK_HALF_MS` and the duty cycle is exactly one half — and
  **two constants would permit a 400/900 blink, whose percentage is not a thing
  this column documents.**
- **`flash_lit(elapsed) = (elapsed / BLINK_HALF_MS) % 2 == 0`** is a pure function
  of an elapsed time. **No counter, no flip-flop, and therefore no state that can
  disagree with the clock** — the failure *a counter incremented
  per event, beside a doc saying it was not* describes, one level down.
- **`FLASH_ONCE_MS = 2 × BLINK_HALF_MS`** is *"flashes once"* as a duration: one
  on, one off, then solid. **It is derived from `BLINK_HALF_MS` and not typed
  beside it**, so the two cannot drift.
- **`SELF_TEST_MS = 4 × BLINK_HALF_MS`** is the self-test, and the multiple is
  load-bearing: **four halves is exactly two flash periods, so the self-test ends
  in its own unlit half**, and a column asked at exactly `SELF_TEST_MS` reports
  every row `Off`. That is the assertion, and it is why the constant is a
  multiple rather than a round figure like *"about a second and a half"*.
- **`TPMS_DWELL_MS = 3000`** is the only duration in this task with **no source at
  all**: the manual says *"for a short amount of time"*, and that is a phrase and
  not a duration. **It is a first-principles choice, proposed here and named as
  one**, on § *Asset requirements*'s rule. **The only thing that depends on it is
  one test's `Duration`**, which is the honest way to carry an invented constant.

**The latch's reading, stated because the source is one sentence about two
causes.** § *The car-status pane* gives the timing rule first — *"steady for low
pressure and flashing for a sensor fault"* — and then says *"The tire light **also**
latches"* **without saying which of the two causes arms the latch.** This task
implements **the union**: *the tell-tale latches whenever it lights, for either
reason*, and clears only on the dwell. **The alternative reading** — only a sensor
fault latches, and plain low pressure clears when inflated — is defensible and is
**recorded here as the alternative rather than argued against**, because the
sentence supports both and a reader who prefers the other has it written down.

## Requirements

1. **A new module `ui/src/ui_demo/src/indicators.rs`**, declared `mod
   indicators;` in `ui/src/ui_demo/src/main.rs` beside `mod fps;`, so the demo
   carries **two** modules and `fps.rs` is the precedent for the second.
   **Every public item is doc-commented**, `developer.md` § *API design*, and
   **every function that returns a value the caller could ignore is
   `#[must_use]`** — *"`showing` that is ignored draws an empty column"* is the
   case that matters.

2. **`pub enum Severity`, five variants in rank order, `Ord` derived so the rank
   is a property of the type and not a sort key beside it:**

   ```rust
   /// The five slots the indicator column ranks by.
   ///
   /// **Five, in rank order, and the declaration order *is* the order** —
   /// `#[derive(PartialOrd, Ord)]` over `Red, Amber, Green, Blue, Grey` makes
   /// `Red` the least and `Grey` the most, and a separate rank table would be a
   /// second list of the same thing.
   ///
   /// **Five and not `ui_core::widgets::toast::Severity`'s four**: the toast's
   /// `Info` is *"a message worth saying"*, and this column has no messages. It
   /// has a blue slot and a grey slot, and a rank that put grey below blue with
   /// no way to name grey would file a dimmed headlight with the faults.
   ///
   /// **`main.rs` already imports `toast::Severity` unqualified**, so every use
   /// of this one in that file is written `indicators::Severity` and is not
   /// imported.
   #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
   pub enum Severity { Red, Amber, Green, Blue, Grey }
   ```

   with `#[must_use] pub fn token(self) -> ThemeToken` and `#[must_use] pub fn
   label(self) -> &'static str`. **`token`'s mapping is this repository's and its
   doc says so**: `Red → ThemeToken::Error`, `Amber → ThemeToken::Warning`,
   `Green → ThemeToken::Success`, `Blue → ThemeToken::Primary`, `Grey →
   ThemeToken::TextMuted`. **All five are tokens that exist today** — the enum has
   33 tokens and task 50 makes it 34, and **this task adds no token**, so
   `TOKEN_COUNT` does not move and a theme switch moves all five discs at once.

3. **`pub enum Timing`, four variants, each doc-commented with the sentence from
   § *The evidence* that produced it**, and **no variant carries data**:

   ```rust
   /// How one condition's light behaves over time.
   ///
   /// **Four rules, and they are the four the source names.** `Steady` is the
   /// default and the only rule that is a plain function of the condition; the
   /// other three are the three § *The car-status pane* calls *"distinguished by
   /// timing, not colour"*, plus the latch, which is a fourth behaviour the same
   /// sentence carries.
   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   pub enum Timing {
       /// Lit exactly while the condition holds. **Twenty of the twenty-three.**
       Steady,
       /// Flashing while `Condition::active` holds, solid when it does not.
       /// **ESC** — *"flashes while actively correcting and goes solid if it is
       /// a fault"*.
       FlashWhileActive,
       /// Flashing for the first `FLASH_ONCE_MS` the condition holds, solid
       /// after. **ABS** — *"flashes once at startup then faults if it stays"*.
       FlashOnceThenSolid,
       /// Solid while the pressure is low, flashing while the sensor has faulted,
       /// and **latched** once it has been lit. **Tyre pressure**, and the only
       /// rule with memory.
       LatchedPressure,
   }
   ```

4. **`pub struct Condition`, four `pub` fields, and the table of twenty-three.**
   `name: &'static str`, `severity: Severity`, `timing: Timing`, `active: bool`,
   `faulted: bool`. **`Indicators::demo()` builds the table** in the order
   § *The evidence*'s five bullets give, so `indicators[0]` is *Brake fault* and
   `indicators[1]` is *Parking brake applied*, and **the index is the identity**:
   the demo's `I`, `J` and `L` keys write booleans into named rows by index, and
   `Indicators::index_of(name) -> Option<usize>` is how the caller names one.
   **Two names are given their exact source wording** — *"Seat belt unfastened in
   an occupied seat"* and *"Parking brake applied"* — because § *Corrections to the
   first sketch* and § *The car-status pane* both quote them and a shorter name
   would be a second spelling.

5. **`pub struct Indicators`, five fields, and the three clocks it keeps:**

   ```rust
   pub struct Indicators {
       conditions: Vec<Condition>,
       /// Whether each `FlashOnceThenSolid` row's single flash has run.
       /// **One bit per row, and it exists because "flashes once" is a rule
       /// about history and history is not in the condition.**
       flashed: Vec<bool>,
       /// Whether the tyre tell-tale is latched. **Written by `tick` and by
       /// `drive_for` and by nothing else** — that is what makes it a latch
       /// rather than a flag.
       latched: bool,
       /// How long the column has been powered up.
       power_up: Duration,
       /// How long the speed has been at or above `TPMS_CLEAR_KMH`.
       dwell: Duration,
   }
   ```

   `#[must_use] pub fn demo() -> Self` — a fresh column, `power_up` at zero and
   therefore **in its self-test**, which is the state a column is in when the
   vehicle powers up. `#[must_use] pub fn len(&self) -> usize`, and **no
   `is_empty`**, matching `nav::Screens::len`'s rule from task 42.

6. **The constants, `pub`, in the module's own block, each doc-commented with its
   arithmetic and its source:**

   ```rust
   pub const BLINK_HALF_MS: u32 = 400;
   pub const FLASH_ONCE_MS: u32 = 2 * BLINK_HALF_MS;
   pub const SELF_TEST_MS: u32 = 4 * BLINK_HALF_MS;
   pub const TPMS_CLEAR_KMH: u32 = 25;
   pub const TPMS_DWELL_MS: u32 = 3000;
   pub const INDICATOR_ROW_HEIGHT: f32 = 28.0;
   pub const INDICATOR_DISC: f32 = 16.0;
   pub const INDICATOR_PAD: f32 = 10.0;
   pub const INDICATOR_GAP: f32 = 8.0;
   pub const INDICATOR_NAME_X: f32 = INDICATOR_PAD + INDICATOR_DISC + INDICATOR_GAP;
   pub const INDICATOR_FONT_SIZE: f32 = 16.0;
   ```

   `BLINK_HALF_MS`'s doc carries the duty-cycle arithmetic; `FLASH_ONCE_MS`'s and
   `SELF_TEST_MS`'s carry the multiples and **why they are multiples**;
   `TPMS_CLEAR_KMH`'s carries *"15 mph (25 km/h)"* and the reason the demo speaks
   km/h; `TPMS_DWELL_MS`'s carries **"a first-principles choice, and no source
   names a duration"**; `INDICATOR_NAME_X` is derived from the other three so a
   reader cannot move the disc without moving the name. **A test asserts all six
   duration constants are finite and positive, that `FLASH_ONCE_MS` and
   `SELF_TEST_MS` are whole multiples of `BLINK_HALF_MS`, and that
   `INDICATOR_NAME_X < 176.0`** — the width `TASK_UI_DEMO_03` gives the column,
   **which is why that number is written as a literal here and not derived from
   task 03's `INDICATOR_WIDTH`, and why the test pins the relationship rather than
   a duplicate.**

7. **`fn flash_lit(elapsed: Duration) -> bool`, `pub`, `#[must_use]`, and the
   whole of the blink:**

   ```rust
   /// Whether a blink that began `elapsed` ago is in its lit half.
   ///
   /// **A pure function of the elapsed time — no counter, no flip-flop, and
   /// nothing that can be left in a different state from the clock.** The
   /// divisor is `BLINK_HALF_MS`, which is `400` and not zero, and `Duration`'s
   /// `as_millis` cannot overflow for any duration this column is given.
   #[must_use]
   pub fn flash_lit(elapsed: Duration) -> bool
   ```

   **`fn row_rect(rect: Rect, rank: usize) -> Rect`, `pub`, `#[must_use]`, and the
   column's geometry** — one function with **two readers**, the disc painter and
   the demo's own label paint, so a row's disc and its name cannot land on
   different y values. **An overflowing row is not clamped**: `rank * 28.0` runs
   past `rect`'s bottom and the node's own clip cuts it, which is
   `TASK_UI_PRIM_45`'s contribution and `L8`'s *Blocks* column applied.

8. **`Indicators`' five mutators and four readers, with this order and no
   other:**

   ```rust
   /// Re-arms the power-up self-test by zeroing the column's clock.
   ///
   /// **The only way `power_up` returns to zero**, and it is called from
   /// `Demo::set_car_state` on every entry into `CarState::Parked` — § *Screen
   /// states* puts the self-test in the parked bullet because a power-up *is* a
   /// transition into parked, and this task implements that reading and says so.
   pub fn arm_self_test(&mut self)

   /// Writes one row's `active` and `faulted`, and returns whether either moved.
   ///
   /// **The only way a condition changes**, which is what makes
   /// `demo()`'s table data rather than state.
   pub fn set(&mut self, index: usize, active: bool, faulted: bool) -> bool

   /// The external event, and the only thing that clears the latch.
   ///
   /// Accumulates `dwell` while `speed_kmh` is at or above
   /// [`TPMS_CLEAR_KMH`], zeroes it otherwise, and clears `latched` — writing
   /// **nothing else** — on the frame `dwell` first reaches `TPMS_DWELL_MS`.
   /// Returns whether it wrote.
   pub fn drive_for(&mut self, speed_kmh: u32, delta: Duration) -> bool

   /// Advances the column's clocks and derives every row's phase.
   ///
   /// **The only function that writes a `Phase`**, and it is called once per
   /// frame from `Demo::frame` on the crate's own *"aim once, tick per frame"*
   /// idiom. It arms the latch on the frame the tell-tale lights, so the latch
   /// is history rather than a query with a side effect.
   pub fn tick(&mut self, delta: Duration) -> bool

   /// What row `index` is doing this frame. `None` for an index the column does
   /// not hold — **the honest answer, not a panic** (`developer.md` § *Panics*).
   #[must_use] pub fn phase(&self, index: usize) -> Option<Phase>
   /// The rows that are showing, **most severe first**, and `showing()` and
   /// `phase()` are the two readers of the same data, which requirement 11's
   /// `showing_and_phase_never_disagree` is about.
   #[must_use] pub fn showing(&self) -> Vec<usize>
   /// The discs only. **No text and no metrics** — the names are `Label`s the
   /// demo already paints, and a column that measured its own text would have two
   /// measurements of the same run.
   #[must_use] pub fn paint(&self, rect: Rect, theme: &Theme) -> Vec<DrawCommand>
   ```

   **`Phase` is a four-variant enum — `Off`, `On`, `Flashing`, `Latched` — and its
   `lit` mapping is one `pub fn lit(self) -> bool`**, with `Flashing` lit on the
   clock and the other two lit unconditionally. **The self-test is layered above
   the phases and not inside them**: while `power_up < SELF_TEST_MS`, `lit` is
   `flash_lit(power_up)` for **every** row and `phase` still reports the row's own
   phase, **so the self-test and a live fault are not two mechanisms fighting over
   one bit.** That sentence is the design.

9. **`pub struct DemoMode` is not this task's**, and the column's one addition to
   `Demo`:

   ```rust
   /// `Demo` gains `indicators: indicators::Indicators` and
   /// `indicator_labels: Vec<DemoLabel>`, in `DemoSlider`'s style.
   ```

   **`indicator_labels` is twenty-three `DemoLabel`s built with the demo's
   existing `read_only_label`**, each with the theme's `TextMuted` colour and a
   tight constraint at `INDICATOR_NAME_X`, `INDICATOR_ROW_HEIGHT` wide minus the
   left inset. **Reusing `read_only_label` rather than writing a second
   construction** is `developer.md` § *Phase 2*'s *"Read the surrounding code
   first"* applied to a helper that exists for exactly this: a fixed string in a
   fixed box with an ellipsis. **All twenty-three nodes live under
   `DemoPane::indicators`**, and each carries a `PageMember` row and a
   `placed_handles` row — **twenty-three rows and not one**: the demo's page table
   and `placed_handles` have each been caught green with a row dropped, and
   **twenty-three labels under one box is the largest instance of that shape the
   demo has.**

10. **Three rows in `GALLERY_SHORTCUTS`**, so the array becomes
    `[GalleryShortcut; 26]`: **`I`** toggles the ABS row (*"flashes once then goes
    solid"*), **`J`** toggles the ESC row's `active` (*"flashes while actively
    correcting"*), **`L`** toggles the tyre-pressure row's `active`
    (*"does not clear when you inflate"*). **All three keys are free today** and
    each row's `Some(...)` page is `Page::Demo`.
    **Each handler writes through `Indicators::set` and nothing else** — so a key
    that acts without a row is caught by
    `no_printable_key_acts_without_a_row_in_the_shortcut_table`, which is the
    obligation that table exists for.

11. **The tests, in `indicators.rs`'s own `#[cfg(test)] mod tests` beside the
    code, plus `main.rs`'s existing `mod tests` for the demo-level half.** **No
    display, no network, no filesystem and no wall clock** — every one of the
    column's own tests is a call with a **literal `Duration`**, which is the point
    of `flash_lit` being a pure function.

    - **`flash_lit_alternates_every_blink_half_ms`** — `false` at `0`, `true` at
      one millisecond, `true` at `BLINK_HALF_MS - 1`, **`false` at exactly
      `BLINK_HALF_MS`**, `false` at `BLINK_HALF_MS + 1`, `true` at
      `2 * BLINK_HALF_MS`. **The phase boundary is asserted on both sides of it**,
      because an off-by-one at the boundary is invisible at any other sample.
    - **`the_duty_cycle_is_exactly_one_half`** — over `0..BLANK_PERIOD_MS` in
      50 ms steps, the lit fraction is within one step of one half, **beside its
      control**: the same sweep over a second half-length gives a different
      fraction, which is what stops the assertion passing for any threshold.
    - **`the_column_constants_are_consistent`** — five durations finite and
      positive, the two multiples whole, `TPMS_CLEAR_KMH` above zero,
      `INDICATOR_DISC + 2.0 * INDICATOR_PAD < INDICATOR_ROW_HEIGHT`, and
      `INDICATOR_NAME_X + INDICATOR_NAME_X < 176.0`.
    - **`the_twenty_three_conditions_are_the_five_slots_the_section_enumerates`** —
      a count of **23** and a per-slot count of **6 / 7 / 4 / 3 / 3**, asserted by
      value with the counts in the failure message. **The count and the
      distribution are both the claim**, and § *The evidence*'s "~20 conditions"
      is reconciled against them in the test's own doc comment.
    - **`the_column_runs_its_whole_self_test_and_ends_dark`** — a fresh column,
      `lit` for every row at `Duration::ZERO`, and **`None`-free at
      `SELF_TEST_MS` for every row** — the multiple's reason made checkable.
    - **`an_abs_condition_flashes_once_and_then_goes_solid`** — `set` the row,
      `tick`, and assert `Flashing` while `power_up < FLASH_ONCE_MS` and `On`
      after, **including that it does not resume flashing**: three further ticks
      and the phase is still `On`. **The "then" is the half the rule is about** and
      a test that only checked the flash would pass against a rule that blinks
      for ever.
    - **`esc_flashes_while_active_and_goes_solid_when_faulted`** — `active` alone
      gives `Flashing`; `active` cleared gives `Off`; **`faulted` with `active`
      cleared gives `On`, not `Flashing`** — the *"goes solid if it is a fault"*
      half, which is a different clause from the *"while actively"* one.
    - **`the_tyre_telltale_latches_and_nothing_else_clears_it`** — the load-bearing
      test, in three parts **with a control beside each**: (a) `active` on, the row
      is `On`; (b) **`active` off, the row is still `On`** — the latch; (c)
      `drive_for(0, ...)` for a hundred times `TPMS_DWELL_MS` and the row is
      **still `On`**, because zero is below `TPMS_CLEAR_KMH`; (d) `drive_for
      (TPMS_CLEAR_KMH, TPMS_DWELL_MS)` once and the row is **`Off`**. **Part (c)
      beside part (d) is the whole claim** — a test with only (d) passes against a
      dwell that ignores the speed.
    - **`a_sensor_fault_flashes_and_the_flash_survives_the_fault_clearing`** — the
      other cause, per § *The blink, the latch and the self-test*'s recorded
      ambiguity: `faulted` on gives `Flashing`, `faulted` off with the latch
      armed gives `On`, and `On` is not `Off`.
    - **`showing_is_ranked_and_showing_and_phase_never_disagree`** — over every
      subset of the rows a `u32` mask can name (`1 << 23` masks, cheap and
      exhaustive), `showing()` is non-increasing in `severity`, **every index in
      `showing()` has `phase != Off`, every index outside it has `phase == Off`,
      and `showing()` is empty exactly when no row is lit.** The middle clause is
      the one that catches a `showing` and a `phase` built from different data,
      and **the exhaustive mask is the same move as
      `every_grid_cell_is_inside_the_grid_and_no_two_cells_overlap` in
      `TASK_UI_PRIM_52`**: the space is small integers and a full sweep is
      strictly stronger than a sample.
    - **`the_three_lighting_conditions_are_mutually_exclusive_in_the_defaults`** —
      `Demo::seed_conditions(CarState::Driving)` sets exactly one of *High beam*,
      *High beam with Adaptive Headlights armed* and *Adaptive Headlights armed
      but dimmed*, asserted over all three states. **The source is §
      *Corrections to the first sketch*'s lighting row and the assertion is its
      exclusivity**, and it is the one place this task's data is constrained by a
      correction rather than by the enumeration.
    - **`the_column_clips_rather_than_overflowing_onto_the_card_row`** — twenty-
      three rows set active, one frame, and **the disc commands' `y` values run
      past the column's bottom** — which is the answer — **and the node's own
      `LayoutState::clip` is the intersection**. **The mirror of
      `an_overflowing_child_keeps_its_size_and_computes_a_clip_rect`**, and it is
      `TASK_UI_PRIM_45` and `L8` exercised for the first time by something on
      screen.
    - **`arming_the_self_test_is_the_only_thing_that_restarts_it`** — a column
      driven past `SELF_TEST_MS`, then `arm_self_test`, and every row is lit again
      on the next frame; **and `drive_for` does not**, with that control beside it.
    - **`every_indicator_label_has_a_page_row_and_a_placed_handles_row`** —
      `main.rs`, and **it asserts the complement**: every one of the twenty-three
      label handles is in `page_members` for the seventh variant and in
      `placed_handles`, **and `page_members.len()` grew by exactly twenty-three
      over the count `TASK_UI_DEMO_03` left**. **A count beside the membership**,
      because a membership assertion alone passes when a row is missing,
      and the demo has now been bitten by that twice.
    - **`the_i_j_and_l_keys_reach_the_column_through_handle_event`** —
      `demo_on(Page::Demo)`, one `Demo::handle_event` per key with a
      constructed `Event::KeyDown`, asserting the named row's `active` moved and
      that **no other row's did**. **This is the no-pointer route**: an `Event` is
      a value, `Demo::new` needs no window, and the gallery's existing tests
      already drive `handle_event` this way.

12. **`doc/ui/DEMO_APPLICATION.md` gains two dated, attributed notes and no new
    row:** a note in § *The car-status pane* recording that the column is built
    with **23 conditions across the five enumerated slots**, that **the diagram's
    green high-beam circle disagrees with the bullet list's blue and the bullet
    list won**, that the three timing rules are the ones above, that **the latch
    arms for either cause** and the alternative reading is recorded, and that the
    self-test runs on construction and on every entry into parked; and a note in
    § *Could not verify*, the **Exact colours, spacing, radii** row, recording
    that **all five colours are theme tokens and every pixel figure in the column
    is a proposal** — on that row's own *"a first-principles choice, not a
    transcribed value"*.

13. **`doc/ui/IMPLEMENTATION_STATE.md` gains the record**: a task-table row for
    **the demo task 04**, naming the file, its review count and its
    waivers-or-none; a task section carrying **the four timing rules and the
    column that produces them**, **the blink as one half-period and the duty cycle
    as a consequence**, **the latch's union reading and its alternative**, **the
    `TPMS_DWELL_MS` invention named as one**, **the `toast::Severity` name clash
    and the mitigation**, **the test count before and after**, and **the frame
    rate for all seven pages**.

14. **The suite, the seven pages and the frame rate are all produced**, as
    `TASK_UI_DEMO_03` requirement 14 spells out, against a baseline of **1894
    (1450 + 224 + 220)**.

## Acceptance Criteria

- [ ] **Twenty-three conditions, five slots, and the counts are asserted by
      value.** `the_twenty_three_conditions_are_the_five_slots_the_section
      _enumerates` asserts a count of **23** and a per-slot distribution of
      **6 / 7 / 4 / 3 / 3**, with both in the failure message, and
      `DEMO_APPLICATION.md` § *The car-status pane* carries a dated note
      reconciling the document's own *"~20 conditions"* against the arithmetic.
      **Two names are the source's exact wording** — *"Seat belt unfastened in an
      occupied seat"* and *"Parking brake applied"* — and no second spelling of
      either appears in the module.

- [ ] **The five colours are theme tokens, and the section's diagram conflict is
      resolved in the open.** `Severity::token` returns `Error`, `Warning`,
      `Success`, `Primary` and `TextMuted` for `Red`, `Amber`, `Green`, `Blue` and
      `Grey`; **`ThemeToken::TOKEN_COUNT` does not move**, so a theme switch moves
      all five discs and `TASK_UI_PRIM_50` is untouched. And § *The car-status pane*
      carries a note that the diagram's *"🟢 high-beam A"* disagrees with the
      bullet list's *"**Blue** — high beam"*, that **the bullet list won**, and that
      **neither half makes any colour sourced** — § *Could not verify*'s *Exact
      colours, spacing, radii* row applies to all five.

- [ ] **The blink is a pure function of an elapsed duration and the duty cycle is
      a consequence.** `flash_lit` is `(elapsed / BLINK_HALF_MS) % 2 == 0` with
      no counter and no cell to get out of step;
      `flash_lit_alternates_every_blink_half_ms` asserts the phase boundary **on
      both sides** (`BLINK_HALF_MS - 1` lit, `BLINK_HALF_MS` dark); and
      `the_duty_cycle_is_exactly_one_half` sweeps one full period **beside a
      control at a second half-length**, so the assertion cannot pass for any
      threshold. **There is no `Cell<bool>`, no `RefCell<bool>` and no flip-flop in
      the module** — `rg -n 'Cell|RefCell' ui/src/ui_demo/src/indicators.rs`
      returns nothing — which is *a counter incremented per event,
      beside a doc saying it was not* one level down.

- [ ] **All three timing rules are pinned, including their second clauses.**
      `an_abs_condition_flashes_once_and_then_goes_solid` asserts `Flashing` under
      `FLASH_ONCE_MS` and `On` after, **and that three further ticks do not resume
      the flash**; `esc_flashes_while_active_and_goes_solid_when_faulted` asserts
      `Flashing` while active, `Off` when cleared, **and `On` — not `Flashing` —
      when `faulted`**; `a_sensor_fault_flashes_and_the_flash_survives_the_fault
      _clearing` asserts `Flashing` on the fault, `On` after it, **and `On` is not
      `Off`**. **`FLASH_ONCE_MS` and `SELF_TEST_MS` are derived from
      `BLINK_HALF_MS` in the source**, so `rg -n 'FLASH_ONCE_MS\|SELF_TEST_MS'
      ui/src/ui_demo/src/indicators.rs` shows two multiplications and no literals.

- [ ] **The latch is a latch, and the three ways of clearing it are separated by
      three tests with controls.** `the_tyre_telltale_latches_and_nothing_else
      _clears_it` has four parts and the two that matter are **beside each other**:
      **`drive_for(0, …)` for a hundred dwells leaves the row `On`**, and
      **`drive_for(TPMS_CLEAR_KMH, TPMS_DWELL_MS)` once takes it to `Off`**.
      **Clearing the condition alone does not clear the light** — that is part (b)
      and it is the claim § *The car-status pane* makes in one sentence.
      `arming_the_self_test_is_the_only_thing_that_restarts_it` shows the same
      shape for the self-test's clock, **with `drive_for` as its control.**

- [ ] **The column is ranked, and the two readers of its data are proved to
      agree.** `showing_is_ranked_and_phase_never_disagree` sweeps all
      **`1 << 23` masks** of the conditions — exhaustive, with no seed, because the
      space is small integers — asserting `showing()` is non-increasing in
      `severity`, **that every index it names has `phase != Off` and every index it
      omits has `phase == Off`**, and that it is empty exactly when nothing is lit.
      **The middle clause is the one that matters and it is the complement, not the
      membership.**

- [ ] **The self-test runs, and ends dark on a whole number of flashes.**
      `the_column_runs_its_whole_self_test_and_ends_dark` asserts every row is lit
      at `Duration::ZERO` and **that every row reads `Off` at exactly
      `SELF_TEST_MS`**, which is why `SELF_TEST_MS` is `4 × BLINK_HALF_MS`. And
      `main.rs` carries **one line in `Demo::set_car_state`** —
      `self.indicators.arm_self_test()` on entry into `CarState::Parked` — with
      **no other change to that function**, which § *Requirements* 6 of task 03
      made room for.

- [ ] **The column overflows into a clip and not onto the card row.**
      `the_column_clips_rather_than_overflowing_onto_the_card_row` sets all
      twenty-three rows active, runs one frame, and asserts **the disc commands'
      `y` values run past the column's bottom** — which is the answer — **and that
      the node's `LayoutState::clip` is the intersection of its rect and its
      parent's.** **The mirror of `an_overflowing_child_keeps_its_size_and
      _computes_a_clip_rect`,** and the first thing on screen to exercise
      `TASK_UI_PRIM_45` and `L8` together.

- [ ] **Three keys reach the column, and reach only their own rows.**
      `the_i_j_and_l_keys_reach_the_column_through_handle_event` drives
      `Demo::handle_event` with a constructed `Event::KeyDown` for each of `I`,
      `J` and `L` and asserts the named row moved **and that no other row did**.
      **No acceptance criterion in this file requires a pointer event**, because
      none can be delivered on this host: `GALLERY_SHORTCUTS` is the only dispatch
      and the three keys are rows in it, which is
      `no_printable_key_acts_without_a_row_in_the_shortcut_table`'s obligation.

- [ ] **Twenty-three labels, twenty-three page rows, twenty-three placed-handles
      rows, and a count beside the membership.**
      `every_indicator_label_has_a_page_row_and_a_placed_handles_row` asserts
      every one of the twenty-three label handles is in `page_members` for the
      seventh variant and in `placed_handles`, **and that `page_members.len()`
      grew by exactly twenty-three over the count `TASK_UI_DEMO_03` left** —
      because a membership assertion passes when a row is missing, and the demo's
      page table has been caught green that way twice.
      `expected_placed_rect_names` gains all twenty-three names and **no existing
      row of any of `every_page_places_every_rect_where_the_gallery_placed_it`,
      `no_two_placed_rects_overlap`, `placed_handles` or
      `assert_placed_handles_is_complete` is loosened.**

- [ ] **The three lighting conditions are never two at once.**
      `the_three_lighting_conditions_are_mutually_exclusive_in_the_defaults`
      asserts exactly one of *High beam*, *High beam with Adaptive Headlights
      armed* and *Adaptive Headlights armed but dimmed* is active in each of the
      three states, on § *Corrections to the first sketch*'s lighting row.

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **one new file** —
      `ui/src/ui_demo/src/indicators.rs` — and changes to
      `ui/src/ui_demo/src/main.rs`, `doc/ui/DEMO_APPLICATION.md` and
      `doc/ui/IMPLEMENTATION_STATE.md`, **and no change to any file under
      `ui/src/ui_core/`**: this task is demo-local for the reason § *Why no
      shipped widget covers this* argues, and **`StatusLight` is not added to the
      crate**. `ui/Cargo.toml` and `ui/Cargo.lock` are unchanged. **No `unsafe`,
      no `unwrap`, no `expect`, no `panic!`, no `unimplemented!`, no `todo!`** —
      and in particular **`phase()` returns `Option<Phase>` rather than panicking
      on a bad index**, because an index the demo computed wrongly is not the
      caller's error to pay for in a panic. **`Page::ALL` is seven and
      `Page::DEFAULT` is still `Page::Pads`,** and **no `--tab=` name is added.**

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: `flash_lit_alternates_every_blink_half
      _ms`, `the_duty_cycle_is_exactly_one_half`,
      `the_column_constants_are_consistent`,
      `the_twenty_three_conditions_are_the_five_slots_the_section_enumerates`,
      `the_column_runs_its_whole_self_test_and_ends_dark`,
      `an_abs_condition_flashes_once_and_then_goes_solid`,
      `esc_flashes_while_active_and_goes_solid_when_faulted`,
      `the_tyre_telltale_latches_and_nothing_else_clears_it`,
      `a_sensor_fault_flashes_and_the_flash_survives_the_fault_clearing`,
      `showing_is_ranked_and_showing_and_phase_never_disagree`,
      `the_three_lighting_conditions_are_mutually_exclusive_in_the_defaults`,
      `the_column_clips_rather_than_overflowing_onto_the_card_row`,
      `arming_the_self_test_is_the_only_thing_that_restarts_it`,
      `every_indicator_label_has_a_page_row_and_a_placed_handles_row`,
      `the_i_j_and_l_keys_reach_the_column_through_handle_event` — **fifteen**,
      against a measured baseline of **1894 (1450 `ui_core` + 224 `ui_demo` +
      220 doctests)**, with the three counts pasted, **each higher than the
      baseline by the number of tests added in it** and **no test deleted, renamed
      away or weakened.** `cargo fmt --check`, `cargo build --all-targets
      --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`
      and `cargo doc --no-deps` clean. `cargo audit` is **recorded as not
      installed on this host, not passed.**

- [ ] **The seven pages are captured, and the six unchanged ones are
      pixel-identical by the same mechanism task 03 names.** The capture commands
      of `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
      method* verbatim, window id re-read at each capture, `pgrep -a -x ui_demo` in
      the same call as each `magick import`, `magick compare -metric AE` per page.
      **AE 0 outside `y ≥ 680` on the six**, and on the seventh page **the column is
      visible with at least one disc in at least two slots**, **the self-test is
      visible in a run shorter than `SELF_TEST_MS`** — `ROADOS_RUN_SECONDS` is a
      whole number of seconds and `SELF_TEST_MS` is 1600, so **a one-second run is
      inside the self-test and shows every light lit** — **and the tyre tell-tale
      is lit in the capture taken after `L` and still lit in the one taken after a
      driving dwell that has not accumulated.**

- [ ] **The frame rate is measured on all seven pages and the script's own line
      is pasted.** `.ai/tools/fps-check.sh 10 55` on the default page, which is
      **the only thing the script can do** — it takes `seconds` then `floor` and
      runs the binary with no arguments and no page — and then
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=demo` for each of
      the seven, the `roados-fps` line parsed by hand. **Every page above 55**,
      **the six unchanged pages inside the recorded release band**, and **the
      seventh page's number reported with its own arithmetic**: twenty-three
      discs and twenty-three labels per frame is a per-frame cost the other six do
      not pay. **A drop under the band is reported as a drop.**

- [ ] **What the handoff does not claim, in those words.** It states that **the
      column's lights are discs and not glyphs**, and why: § *Asset requirements*
      says the column's glyphs are *"drawn from state, not selected from an
      atlas"*, and the 33 baked Lucide PNGs do not cover 23 automotive conditions
      one for one — **a column that drew a glyph for eight of its twenty-three
      rows would read as fifteen missing assets**, so each row carries its name in
      `TextMuted` instead and the disc carries the severity and the phase. It
      states that **the latch's arming-cause reading is the union and the
      alternative is recorded**, because § *The car-status pane* supports both. It
      states that **`TPMS_DWELL_MS` has no source at all** and is a first-principles
      choice. It states that **no widget was added to `ui_core`**, and repeats the
      promotion condition — a second consumer needing "a flag that clears on an
      external event". And it states that **the demo cannot receive a pointer
      event on this host**, so nothing in this task was verified by a tap and
      every criterion above is a key, a unit test or a capture.

## Out of Scope

- **No `StatusLight` widget and no change to `ui_core` of any kind.** § *Why no
  shipped widget covers this* argues the column is demo-local, with one consumer,
  demo-specific rules, and `TASK_UI_PRIM_47`'s `Snapshot` already being the
  library's answer to the neighbouring need. **The promotion condition is named,
  not met.**
- **No glyph inside the disc and no use of the 33 baked icons.** § *Asset
  requirements* says the column's glyphs are *"drawn from state, not selected
  from an atlas"*, and the disc — filled, ringed or absent — is what carries the
  state. **Any condition-specific glyph is an asset task.**
- **No seatbelt popup, no per-seat tap-to-mute, and no "Fasten Seatbelt" label.**
  § *Corrections to the first sketch* is explicit that these are **three separate
  channels** and the cluster light is only the first; the popup is composite
  row 13's and the label is not this task's.
- **No lighting control, no switch and no `Segmented`.** The three lighting
  conditions are data this column reads; **choosing between them is
  `Controls > Lights`'s**, and it is a different demo task.
- **No charge-port lamp protocol.** Task 03 draws the lamp; **this column does
  not own it**, because § *Screen states* puts the charge-port lamp protocol in
  the **charging state's** content and not in the indicator column's list.
- **No blink on anything but these rows.** `Label`'s caret blink and the keyboard's
  key colours are the demo's existing clocks and are untouched; `Indicators` adds
  its own `power_up` and `dwell` and **shares neither with them**, because a
  shared clock would make one widget's phase another widget's business.
- **No animation, easing or spring on a light.** A light is on or off on a
  period; **there is nothing to animate towards**, and `Property::animate_to` on
  a light would draw a fade that no photograph of a cluster shows.
- **No new theme token.** `TOKEN_COUNT` does not move. All five slots are existing
  tokens, on § *Asset requirements*'s rule that no token is transcribed and on
  `TASK_UI_PRIM_50`'s not being reopened.
- **No `ThemeScope`, no `ModeScope`, no `Snapshot`, no `Margin`, no
      `LayoutMode::Grid`, no `polygon_is_convex` and no measured text runs.** Each
      is available from tasks 44, 47, 48, 50, 51, 52 and 49 and each is unused
      here, for the reason its own row gives.
- **No new dependency and no `unsafe`.** `ui/Cargo.toml` and `ui/Cargo.lock` are
  untouched; the approved direct dependencies remain `sdl3 0.20`, `glow 0.18` and
  `freetype-rs 0.38`, and a status light is arithmetic over a `u64` of
  milliseconds and a handful of `f32` rects. **Zero new `unsafe` blocks**, and no
  FFI, no GL and no pointer.
- **Found in the tree and deliberately not fixed.** **`main.rs` already imports
  `ui_core::widgets::toast::Severity` unqualified**, and this task's own `Severity`
  is a five-variant enum with the same name. **The mitigation is that `main.rs`
  writes `indicators::Severity` and never imports it**, and the clash is recorded
  here rather than avoided by renaming — because the rename would put this
  repository's word for "severity" *after* the crate's, and the demo's severity is
  a rank and the toast's is a disc. **Whether the crate's `Severity` should be
  widened to five variants is a `TASK_UI_PRIM_n` question and is not opened here.**
