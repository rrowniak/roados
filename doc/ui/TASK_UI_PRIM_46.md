# TASK_UI_PRIM_46: Gap `L5` — An Axis on `Scroll`, Momentum, and Snap Points

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Close the mechanism half of row **`L5`** in `doc/ui/DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* — **an axis on `Scroll`, momentum, and
snap points** — and deliver **none of the three products** row `L5` names in its
`Blocks` column. Those three are demo work: the card carousel, dock overflow, and
the widened wiper segmented control. This task is the mechanism they are all
waiting on, and the file says that in those words rather than claiming the row.

**This task is `ui_core` only, and that is a decision with a consequence stated
rather than discovered at review: the demo constructs no `Scroll` and no `List`
today, and this task wires none.** `grep -n 'widgets::scroll\|widgets::list'
ui/src/ui_demo/src/main.rs` returns **nothing** — the demo's `use` block names
`button`, `chart`, `container`, `dialog`, `gauge`, `image`, `keyboard`, `label`,
`progress`, `slider`, `text_input`, `toast` and `toggle`, and not `scroll` or
`list`. So the six gallery pages must come out **pixel-identical**, and the frame
rate must not move, and **the mechanism that makes both true is one fact, not a
hopeful one: no demo code path reaches the widget this task changes.** That is
also why there is no seeded capture here, no `CLIP_PROBE` equivalent, and no
temporary instrumentation to revert: there is nothing on screen this task moves.

## Context

### What is in the crate at `75a896c` plus the uncommitted diff, established and not re-derived

Citations are **by symbol and by file path**. The tree is being modified in
parallel and a line number written today is wrong tomorrow — `AGENTS.md` states
that rule for its own citations and it applies here for the same reason, and task
45's § *The eleven claims this task deletes* applies it to `ui/src` too.

| Fact | Where |
|---|---|
| **`scroll_offset` is a scalar `Property<f32>` and is the truth.** Its own doc says it *"is the distance the content is shifted **up** by. Zero is the top of the content, a positive offset reveals what is below it, and the content is drawn at `rect.y - scroll_offset`"*, and the struct doc says it *"is never animated: an interaction is immediate, and a viewport that eases towards a finger is a viewport the finger has already passed."* | `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::scroll_offset`, `Scroll` |
| **Every helper is scalar in y.** `max_scroll(viewport_height, content_height) -> f32`, `clamp_scroll(offset, viewport_height, content_height) -> f32`, `gesture_delta(delta: Offset) -> f32` (whose body is `delta.y`), `wheel_delta(delta: Offset) -> f32`, `visible_rect(viewport: Rect, content_height: f32, scroll_offset: f32) -> Rect`. | `ui/src/ui_core/src/widgets/scroll.rs`, `max_scroll`, `clamp_scroll`, `gesture_delta`, `wheel_delta`, `visible_rect` |
| **`Scroll` has 28 public methods and none of them is about an axis, a velocity or a resting place.** They are `new`, `handle`, `is_attached`, `content_height`, `set_content_height`, `content_size`, `sync_content`, `apply_offset`, `clip_rect`, `thickness`, `set_thickness`, `palette`, `set_palette`, `size`, `content_rect`, `scroll_by`, `style`, `snap_to_state`, `animate_to_state`, `tick`, `is_animating`, `scrollbar_rect`, `grab_thumb`, `release_thumb`, `is_thumb_grabbed`, `max_scroll_for`, `on_event`, `paint`. | `ui/src/ui_core/src/widgets/scroll.rs`, `impl Scroll` |
| **There is no momentum, and the widget's own doc says so, in the words this task has to remove.** `Scroll::on_event`'s doc: *"**There is no momentum.** Requirement 3 marks smooth scrolling with momentum optional and this task does not ask for it, so a released finger leaves the content exactly where the drag left it: nothing here has a velocity, and nothing keeps moving after the last event."* | `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::on_event` |
| **Horizontal is refused by name, twice, and both refusals are load-bearing.** `scroll.rs`'s module doc lists *"**Horizontal scrolling**, grid content, pull-to-refresh and sticky headers"* under *What is deliberately not here*. `Scroll::on_event` returns `false` unconsumed on a drag with `delta.y == 0.0` and on a `Scroll` with `delta.y == 0.0`, with the comment *"A drag with no vertical component is not a vertical scroll, and leaving it unconsumed lets a horizontal scroller above take it"*, and its doc says *"Left and right are not keys a vertical scroll has: they carry on up the tree"* — which is `scroll_key`'s own comment too. | `ui/src/ui_core/src/widgets/scroll.rs`, module docs, `Scroll::on_event`, `scroll_key` |
| **`Scroll::snap_to_state` is palette snapping.** Its body is `self.clock.borrow_mut().clear(); self.track.set(style.track); self.thumb.set(style.thumb);` — it sets two colours and touches nothing else. Its doc: *"Applies the appearance the scroll's focus implies at once, with no transition."* | `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::snap_to_state`, `Scroll::style` |
| **There are zero `inertia`, `momentum`, `flick`, `overscroll` or `coast` identifiers in `ui_core` outside doc comments**, and this is the absence `DEMO_APPLICATION.md` row `L5` calls *"No inertia identifier exists anywhere"*. | `grep -rn 'inertia\|momentum\|flick\|overscroll\|coast' ui/src/ui_core/src/` |
| **`InputEvent` carries no timestamp.** Its fields are the `kind`, the `position` and the `consumed` flag; `InputEvent::new(kind, position)` takes two arguments. **There is no time on an event anywhere in the crate's input path**, which is why § *Momentum* estimates velocity in `tick` and not in `on_event`. | `ui/src/ui_core/src/input.rs`, `InputEvent`, `InputEvent::new` |
| **`Callback<T>` is `Fn`, not `FnMut`, and returns `()`.** `pub struct Callback<T: 'static>(Option<Rc<dyn Fn(T)>>);` — so it cannot answer a question, only fire one. `widgets/mod.rs`'s own doc says what the two `Callback` types in the crate are for, and this one is the wrong one: *"It is deliberately *not* the `Callback<T>` of [`property`]: that one is private, it is `Fn(&T)`, and it is the notification a property fires on every write, which is a different job from an action a widget performs."* **That is what rules out a snap-point callback** (§ *Snap points*). | `ui/src/ui_core/src/widgets/mod.rs`, `Callback` and its module docs |
| **A drag has no press and no release.** `grab_thumb`'s doc records it: *"the gesture recogniser reports a [`Tap`] on the **release**, and a [`Drag`] once the pointer has already moved. A widget cannot recover 'the finger went down here' from either."* So **a release is invisible to this widget**, and the momentum cannot be armed from one. | `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::grab_thumb`, `Scroll::release_thumb` |
| **`slider::Orientation` is the crate's existing answer to "which component of a wheel does this control read", and it has exactly two variants.** `pub enum Orientation { Horizontal, Vertical }`, `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]`, with `Horizontal` as `#[default]`. Its doc names the wheel: *"which component of a [`Scroll`](InputEventKind::Scroll) does"*. **`progress.rs` deliberately imports it rather than redefining it**, and says why: *"**The `Orientation` is the slider's.** … which is [`slider::Orientation`](crate::widgets::slider::Orientation), imported rather than"*. | `ui/src/ui_core/src/widgets/slider.rs`, `Orientation`; `ui/src/ui_core/src/widgets/progress.rs`, module docs, `Progress::orientation`, `Progress::set_orientation` |
| **`List` is vertical by arithmetic, and says so.** `List::item_rect`'s doc: *"It is `rect`'s own x and width — a vertical list does not move its rows sideways — `rect.y` plus `index * item_height` less the [`offset`](List::offset)"*. `visible_range`'s parameter list is `(item_count, item_height, scroll_offset: f32, viewport_height: f32)` and `index_at(content_y, item_count, item_height)` — **one axis, `index * item_height`, in four of its free functions.** Its module doc lists *"**Momentum.** [`Scroll::on_event`] has none and this does not add any"* under *What is deliberately not here*. | `ui/src/ui_core/src/widgets/list.rs`, module docs, `List::item_rect`, `visible_range`, `index_at` |
| **A list has one offset, reached through the embedded scroll, and that is a stated principle rather than an accident.** `List::scroll(&self) -> &Scroll` with the doc: *"**one offset is the only honest number here**: a second `Property<f32>` on the list would be a second truth to write in step with the first, and a list that scrolled one while it placed rows by the other would show a gap where there is none."* The module doc extends the same principle to six things — the one offset, and then *"the clamp, the wheel step, the sign of a drag, the arrow-key step and the scrollbar all come from that one widget rather than from a second copy of any of them"* — **so a snap point is the seventh thing a list borrows, not the first thing it writes twice.** | `ui/src/ui_core/src/widgets/list.rs`, `List::scroll`, module docs |
| **A `List` that cannot be reached through `&Scroll` already has doors for the setters that need `&mut`.** `List::set_palette` and `List::set_scrollbar_thickness` are `&mut self` and delegate to the inner scroll, and `set_palette`'s doc gives the reason: *"[`List::scroll`] hands out a **shared** reference and [`Scroll::set_palette`] needs a mutable one"*. **This task's two `List` setters are the third and fourth door, and they follow the same shape.** | `ui/src/ui_core/src/widgets/list.rs`, `List::set_palette`, `List::set_scrollbar_thickness` |
| **`Painter` and `DrawCommand` cannot rotate or scale anything non-uniformly, so a horizontal scrollbar is the same construction problem as the vertical one and no new one.** Every variant of `DrawCommand` is bounded by an axis-aligned `Rect` in `command_bounds`, and row `L2`'s own statement is that *"none of its nine variants carries"* a transform. The vertical bar is two `Painter::rounded_rect` calls and a ring; the horizontal one is two more of the same with the rect's `width` and `height` transposed. | `ui/src/ui_core/src/widgets/scroll.rs`, `command_bounds`, `Scroll::paint`; `ui/src/ui_core/src/paint.rs`, `DrawCommand` |
| **`Scroll` already knows its content's width — in one place, and only in one place.** `content_size(&self, nodes) -> Option<Size>` returns the laid-out `Size`, and `sync_content` reads *only* `size.height` out of it. **The horizontal axis needs that same number's `x` half, and the change is one line.** | `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::content_size`, `Scroll::sync_content` |
| **A scroll's setters cannot clamp, and the module says so and gives the caller's answer.** `set_content_height`'s doc: *"The setter does **not** re-clamp the offset, because it cannot: clamping needs the viewport's height and the setter is not given a rect. A caller that has just changed the size of the content calls [`scroll_by`] with a delta of zero, which is exactly this widget's re-clamp."* **The snap-point setter uses this precedent rather than inventing a way around it.** | `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::set_content_height`, `Scroll::scroll_by` |
| **The crate's animation idiom is "aim once, tick per frame", and it is already time-based.** `AnimationClock::tick(delta: Duration)` advances an `elapsed` and `Animation::progress_at(now)` is *"a function of the time it is given, without a clock"*. Every `Easing` variant fixes both ends — `Easing::apply`'s doc: *"Every variant fixes both ends, `apply(0.0) == 0.0` and `apply(1.0) == 1.0`, with one apparent exception that is the point of the variant"*, that exception being `Spring`. | `ui/src/ui_core/src/animation.rs`, `AnimationClock::tick`, `Animation::progress_at`, `Easing::apply` |
| **`Interpolate` is implemented for `f32`, `Color` and `Transform`, and for nothing else.** The snap animates the offset, so `Property<Offset>::animate_to` needs `impl Interpolate for Offset`, and it does not exist. `layout::Offset` is `#[derive(Clone, Copy, Debug, Default, PartialEq)]` with `pub x`, `pub y` and `Offset::ZERO`, and **it carries no arithmetic** — no `Add`, no `Sub`, no `Neg`, no `Ord`, no `min`/`max` method. **So every vector operation this task adds is written out in `Offset::new(…)`,** and requirement 5 says why it does not add any. | `ui/src/ui_core/src/animation.rs`, `Interpolate` and its three impls; `ui/src/ui_core/src/layout.rs`, `Offset` |
| **`Scroll` runs one clock today and clears it from two directions, so a second one is a decision rather than an invention.** `clock: RefCell<AnimationClock>` is cleared by `animate_to_state` and by `snap_to_state`, and `tick` is `self.clock.borrow_mut().tick(delta)`. `AnimationClock::add`'s own doc is the collision rule: *"Starting a second animation over one already running therefore leaves both writing it, and the last to be ticked wins; a caller that means to replace an animation clears the clock with [`clear`] first."* | `ui/src/ui_core/src/widgets/scroll.rs`, `Scroll::animate_to_state`, `Scroll::snap_to_state`, `Scroll::tick`; `ui/src/ui_core/src/animation.rs`, `AnimationClock::add`, `AnimationClock::clear` |
| **Per-node clipping is task 45's, and a scroll viewport depends on it.** `LayoutState::clip` carries the per-node rect, `Batch::clip` carries it to the GPU, and `apply_clip` sets the scissor per batch. Row **#5** of `DEMO_APPLICATION.md` § *Library gaps* is the row, and `TASK_UI_PRIM_45.md` is the task that closes it. **A horizontal viewport needs it for the same reason a vertical one does, and needs it no more and no less.** | `doc/ui/DEMO_APPLICATION.md` § *Library gaps* row #5; `doc/ui/TASK_UI_PRIM_45.md`; `ui/src/ui_core/src/render.rs`, `Renderer::apply_clip` |
| **Test baseline: 1894** — `ui_core` **1450** (plus **1 ignored**), `ui_demo` **224**, doctests **220**. Measured from `ui/` in the session that wrote this file, on `75a896c` plus the uncommitted diff. | `cargo test --all-features` from `ui/` |
| **The demo cannot receive a pointer event on this host.** XTEST pointer injection has never delivered one; keyboard has delivered exactly one. **No acceptance criterion here requires a pointer-driven interaction, and none asks for a capture of one.** | `.ai/tools/README.md` § *Capturing a window* |
| **`.ai/tools/fps-check.sh` takes `seconds` then `floor` and runs the binary with no arguments**, so it **cannot name a page**; per-page is `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>`. Every run is measured — `.ai/workflows/task-sequence.md` § *Gates*. | `.ai/tools/fps-check.sh` |

### Row `L5`, in the document's own words

`doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*, row
**`L5`**:

> **No horizontal scrolling, no momentum, no snap.** `Scroll` and `List` are
> vertical-only — `scroll_offset` is a scalar `Property<f32>`, and left/right are
> explicitly rejected. No inertia identifier exists anywhere.
> `Scroll::snap_to_state` is **palette** snapping, not scroll snapping.

Severity **High**. **Blocks: the card carousel, dock overflow, the widened wiper
segmented control.**

**Three claims, and this task's honest scope against each of them:**

| The row's claim | After this task | Why |
|---|---|---|
| *No horizontal scrolling* | **closed** for `Scroll`. **`List` stays vertical**, so the row's *"and `List`"* half is **still true** and is not claimed otherwise — see § *`List` stays vertical, and why that is a decision* | `List` places rows by `index * item_height` down one axis, in four free functions |
| *No momentum* | **closed** | § *Momentum* |
| *No snap* | **closed** for the offset. `Scroll::snap_to_state` is **still palette snapping** and is **not renamed**, so the row's own warning survives as a distinction rather than as a correction | § *Two names, one widget* |

**And the row's `Blocks` column keeps all three entries, none of which is
delivered.** A card carousel, dock overflow and a widened wiper are demo
compositions; § *Out of Scope* names them as `TASK_UI_DEMO_n` items, on the
recorded argument that `DEMO_APPLICATION.md` § *Corrections to the second gap
table* exists to have been caught: a row closed by a mechanism with no consumer
is the failure this repository writes paragraphs about. **This one will have a
consumer — three of them — and they are not built here.**

## The axis model

**Decision: `scroll_offset` becomes `Property<Offset>`, and the axis is a
separate private field with a getter and a setter.** Six reasons, and the fourth
is the one that settles it.

1. **The card carousel needs two axes at once, so a scalar with an axis beside
   it cannot express the thing this task exists for.** Row `L5`'s first blocked
   item is *"the card carousel"*, and a carousel is a strip that scrolls
   **sideways** while its page content scrolls **vertically** — one viewport,
   two live offsets. A `Property<f32>` plus an `Axis` field can hold one or the
   other. **This is the blocker, and it is in the document this task is measured
   against.**
2. **The alternative is not "keep the scalar"; it is "duplicate the widget."**
   Add `horizontal_offset: Property<f32>` and `horizontal_content_height: f32`
   and every one of the module's helpers needs a twin: `max_scroll`,
   `clamp_scroll`, `visible_rect`, `gesture_delta`, `wheel_delta`, `content_rect`,
   `apply_offset`, `scroll_by`, `max_scroll_for`, `thumb_rect`, `thumb_run`,
   `thumb_length`, `track_rect`, `offset_under`, `scrollbar_rect`, `paint` and
   `on_event` — **seventeen functions, a second scrollbar, and two properties a
   caller must write in step.** That is the duplication `developer.md` § *Phase 2*
   warns against, and it is worse than the duplication a reviewer would have to
   read, because the two copies would drift.
3. **A scalar whose meaning is decided by a runtime field is a reader's trap, and
   this repository has already been caught by exactly that shape.**
   **A constant named for an axis is not a deduction about a dimension**: a
   review read `X_LABEL_GUTTER` as a width subtraction
   when it came off a height, and the rule it leaves: **publish the derivation, and
   do not make a reader infer which dimension a number is about.** A `f32` whose
   axis is `self.axis` is that failure with a runtime trigger.
4. **One vector makes the geometry *shorter*, not longer.** `apply_offset` writes
   `-offset` with no axis branch. `content_rect` shifts `x` and `y` by one
   subtraction each. `thumb_length` and `thumb_run` stay scalar — they are called
   with `height`/`content_height` or `width`/`content_width` and **are the same
   function for both axes**, which is `gesture_delta`'s discipline applied to the
   thumb: one copy of the rule, one place to change it.
5. **The breaking cost is real, is confined to two files, and is mechanical.**
   **Nine public items change** — the `scroll_offset` field and eight signatures:
   `Scroll::scroll_by`, `Scroll::max_scroll_for`, `Scroll::tick`,
   `scroll::max_scroll`, `scroll::clamp_scroll`, `scroll::visible_rect`,
   `scroll::gesture_delta`, `scroll::wheel_delta`. **Fourteen are added** —
   ten on `Scroll`, three on `List`, one `Interpolate` impl. The call sites are
   **`list.rs` alone**: **2 in its production code** (`List::offset` and
   `List::sync`), **4 in its doctests**, and **1 more that the name does not appear
   in** — `List::max_scroll_for`'s body, which calls `scroll::max_scroll`. The
   other mentions of `scroll_offset` in that file are prose and intra-doc links
   that need no edit, and `list::visible_range`'s own `scroll_offset: f32`
   **parameter keeps its type**, because it is `List`'s vocabulary and `List` stays
   vertical. `image.rs`'s single mention is an intra-doc link that still resolves
   and needs no edit. **`ui_demo` has zero call sites, because it has no
   `Scroll`.** A caller today reads `scroll_offset.get()` and writes
   `scroll_offset.set(x)`; after, it reads `.get().y` and writes
   `.set(Offset::new(x, y))` — and **`List`'s own public surface stays `f32`**,
   because `List::offset` still returns the `y` half and `visible_range` still
   takes a `f32`.
6. **This crate is pre-1.0 and has no downstream consumer, and
   `developer.md` § *API design* asks for discussion rather than silence.** This
   section is the discussion; requirement 1 records it in `scroll_offset`'s own
   doc comment and requirement 24's `IMPLEMENTATION_STATE.md` entry records it in
   the state file, so it is auditable later rather than only arguable now.

**What a caller reads and writes, stated as the two lists a reviewer checks.**

| | Today | After |
|---|---|---|
| reads | `scroll.scroll_offset.get() -> f32` | `scroll.scroll_offset.get() -> Offset`; `.y` for the vertical number, `.x` for the horizontal one |
| writes | `scroll.scroll_offset.set(60.0)` | `scroll.scroll_offset.set(Offset::new(0.0, 60.0))` |
| knows its axis | nowhere; it is vertical by construction | `scroll.axis()` |
| content extent | `set_content_height(f32)`; `content_size(nodes) -> Option<Size>` already returns both | `set_content_height(f32)`, **new** `set_content_width(f32)`, and `sync_content` writes both from the one `Size` it already reads |
| maximum travel | `scroll.max_scroll_for(rect) -> f32` | `-> Offset`, one number per axis |
| re-clamp | `scroll.scroll_by(rect, 0.0) -> f32` | `scroll.scroll_by(rect, Offset::ZERO) -> Offset` |
| frame step | `scroll.tick(delta) -> bool` | **`scroll.tick(rect, delta) -> bool`** — see § *Why `tick` gains a `rect`* |

### `Axis` is a new type, and not `slider::Orientation`

**Decision: `pub enum Axis { Vertical, Horizontal, Both }` in
`scroll.rs`, `Vertical` as `#[default]`.** Three reasons, and the first is the
one that settles it.

1. **`Orientation` has two variants and the carousel needs three states.** Its
   `Horizontal` and `Vertical` are exactly the two *single*-axis cases, and
   neither can say "both". Adding `Both` to `Orientation` would put a value on a
   slider's axis type that **no slider could ever honour** — a slider's track
   cannot run along two axes at once — so the variant would be a state its own
   owner can never produce. That is the enum version of a public API that can hold
   nonsense.
2. **`progress.rs` already decided what to do when a second consumer wants the
   same shape, and it decided to import.** Its module docs say the crate has one
   `Orientation` and that `progress` *"borrows the slider's rather than declaring a
   second"*. **The direction of that rule reverses here**: `Orientation`'s own doc
   makes it the *slider's* (it names where a vertical slider's minimum sits, at
   the bottom, because up should mean more), while a scroll's `Axis` has no such
   opinion — zero is the top on both axes and there is no "bigger is higher"
   anywhere in it. **Two types with two meanings is the honest reading; one type
   with three meanings is not.**
3. **`Axis` carries a clause `Orientation` does not: `Both`.** Its doc states
   that it is a **caller's claim about which axes this viewport scrolls**, not a
   direction the widget infers — because a widget cannot infer it. A content that
   happens to be wider than its viewport is not a horizontal scroll; the caller
   says so, exactly as it says a list's `item_count`.

`Axis` is **not** a bitflag set and there is no `impl BitOr for Axis`: `Both` is
a third variant that both arms of every `match` must handle, and a bitflag would
let a caller build a state no match arm covers.

### `Axis::Vertical` keeps the two refusals, and their tests are the reason

The `delta.y == 0.0` early return in `Scroll::on_event` is **kept for
`Axis::Vertical`**, with its comment, and so is `scroll_key`'s refusal of left and
right. The existing named tests that assert them —
`a_drag_that_is_entirely_horizontal_is_left_for_a_horizontal_scroller` and
`a_horizontal_wheel_is_not_a_vertical_scroll` — **keep their names and every one
of their assertions**, and a `git diff` showing either weakened is a stop
condition.

**Why they stay, now that there is a horizontal scroll to hand the event to.**
They are not "no horizontal scroll yet"; they are *"this scroll has no horizontal
axis"*, which is a statement about `self.axis` and is **more** true with an axis
than without one. A vertical scroll that swallowed a horizontal drag would steal
it from a horizontal scroller that is now expressible — the comment's own words
(*"lets a horizontal scroller above take it"*) become true rather than
hypothetical.

**Three rules, written once each, and each pinned by a named test** — the
`gesture_delta` discipline, which exists because an earlier version restated the
sign convention in **eleven tests**:

| `Axis` | a `Drag`'s delta | a `Scroll` (wheel)'s delta | keys |
|---|---|---|---|
| `Vertical` | `y` only; a drag with `y == 0.0` is **not consumed** | `y` only; a wheel with `y == 0.0` is **not consumed** | Up / Down (+ D-pad), on `y` |
| `Horizontal` | `x` only; a drag with `x == 0.0` is **not consumed** | `x` only; a wheel with `x == 0.0` is **not consumed** | Left / Right (+ D-pad), on `x` |
| `Both` | `x` and `y`, both applied, one consumption | each non-zero component applied on its own axis; consumed if any applied | all four arrows + D-pad |

**A diagonal drag on `Axis::Vertical` applies its `y` and consumes**, which is
today's rule unchanged (`gesture_delta` returned `delta.y` and the arm consumed
whenever `delta.y != 0.0`). Stated because "a one-axis control refuses the other
axis" and "a one-axis control ignores the other component of a diagonal drag" are
different rules and a reader will assume the second was intended.

### The wheel's sign on each axis, and its zero-component change

`wheel_delta` reads **only the sign** of a component, for the reason its own doc
gives: *"The input module puts a wheel's notch of 1 and a steering wheel's axis of
32000 in the same field"*. **The horizontal component is negated for the same
reason the vertical one is**, and the reason is the same: SDL's positive `y` is
*"away from the user"*, which under *"down is later"* is **up** the document, and
SDL's positive `x` is the same convention sideways, which under *"right is later"*
is **left** along it. **One negation rule, written once, applied per component.**

**One behaviour change, and it is forced.** Today
`wheel_delta(Offset::new(0.0, 0.0))` returns `-WHEEL_STEP`, because the arm
`if delta.y < 0.0 { WHEEL_STEP } else { -WHEEL_STEP }` cannot answer "neither".
That is unreachable today **because the caller returns `false` on a zero `y`
first** — and under `Axis::Both` a wheel with `x != 0, y == 0.0` *does* reach it,
where it would move the vertical axis by a notch the user did not ask for.
**After this task a zero component contributes `0.0`**, its doctest gains the
assertion, and **no in-tree behaviour changes** because the only caller already
guarded it.

### Why `tick` gains a `rect`

`Scroll::tick(&self, delta: Duration)` has no viewport, and **a momentum without a
viewport cannot be clamped.** The alternatives were considered and rejected:
storing the last rect the widget was handed makes the clamp depend on which
method the caller happened to call this frame; a separate `tick_momentum(rect,
delta)` makes a caller responsible for remembering it, which is the failure
`DEMO_APPLICATION.md` § *Corrections to the second gap table* names when a clip is
*"supplied by the demo's frame loop"* instead of by the widget.
**One `tick`, and it takes the rect.** `List` gets `List::tick(&self, rect, delta)`
in the same shape as `List::sync(&mut self, nodes, rect)` for the same reason.

## Momentum

**Decision: velocity is estimated from the drag stream in `tick`, decayed
exponentially with the closed-form integral, and stopped at a fixed cutoff with no
overshoot and no bounce.** This is the substantive part of the task, and every
number below is derived in its own doc comment rather than asserted.

### Why the velocity is estimated in `tick` and not in `on_event`

`InputEvent` carries **no timestamp** — `kind`, `position`, `consumed`, and
nothing else. A velocity is pixels per second, so it needs a duration, and **the
only duration the crate has without a wall clock is the frame delta `tick` is
handed.** So:

- `on_event(Drag { delta })` applies the delta through `scroll_by` **immediately**,
  exactly as today, so the content follows the finger with **zero added latency**,
  and **accumulates** the applied delta into a private `Cell<Offset>`.
- `tick(rect, delta)`: if the accumulator is non-zero and `delta` is non-zero, the
  velocity is `accumulated / delta.as_secs_f32()`, the accumulator is cleared, and
  the momentum is armed. Otherwise the armed momentum is integrated one step.

**Three consequences, all of them stated rather than discovered:**

1. **The momentum starts on the frame after the last drag**, because that is the
   first frame whose `delta` divides the drag. The first momentum step is
   `(v/k)·(1 − e^{−k·dt})` — about **1.3 %** of the whole travel at `k = 8` and a
   16 ms frame — so the one-frame handover is below the threshold where a stall
   reads as a stall. **The same argument covers the snap's own first frame**, and
   § *Snap points* leans on it a second time.
2. **A wheel notch, an arrow key, or a `scroll_by(rect, Offset::ZERO)` re-clamp in
   the same frame as a drag cancels the momentum**, because each clears the
   accumulator without arming a velocity. Requirement 23's
   `a_wheel_notch_and_a_key_press_cancel_a_momentum_and_start_none` is the test,
   and it is a **decision**: a wheel notch is a discrete request, not half of a
   flick, and a fling that a wheel cannot stop is a fling a user cannot escape.
3. **A thumb drag and a content drag share the accumulator.** A drag on a grabbed
   thumb moves the offset by the pointer's position against the groove
   (`offset_under`), and that displacement is exactly as much a velocity as a
   content drag's. Two paths, one accumulator, one measurement.

**A `Duration::ZERO` tick is guarded and asserted.** `delta.as_secs_f32()` of zero
is `0.0`, and dividing by it is `inf` — so the arm is
`if seconds > 0.0 && !accumulated.is_zero()`, and
`a_zero_tick_neither_arms_a_velocity_nor_moves_the_content` is the test. This is
the arithmetic task 36 requires of `perspective`, where a non-finite input is a
disappearing mesh, not a panic.

### The decay, and why it is exact

Three private constants, in `scroll.rs`'s own constant block, beside `WHEEL_STEP`
and `KEY_STEP_FRACTION`, **private** because every other constant in that block is
private and this crate has one scroll surface — a caller-facing setter for a
fling's feel is a parameter with no consumer, and § *Out of Scope* says so.

| Constant | Value | The arithmetic, for its doc comment |
|---|---|---|
| `MOMENTUM_DECAY_PER_SECOND` | `8.0` | The decay constant `k`, in `s⁻¹`. **Total travel from a release at `v₀` is `v₀/k`**, so `k = 8` means a release at 2 000 px/s travels **250 px** before stopping — about **0.83 of the demo's 300-tall viewport**, which is "one flick, about a screen". And `v(t) = v₀·e^{−kt}` is at `e^{−1} = 37 %` after 125 ms and `e^{−2} = 13.5 %` after 250 ms, so **a fling is over in about a third of a second** — the window in which it must have stopped or the content reads as sliding rather than as released |
| `MOMENTUM_STOP_SPEED` | `8.0` px/s | The cutoff, in px/s. The travel left in the tail from `v` down to zero is `v/k`, so stopping at 8 px/s **abandons 1.0 px** and buys an exact stop. Below about 8 px/s a move is under a tenth of a pixel per 16 ms frame, which is not where a moving pixel reads as motion |
| `MOMENTUM_MAX_SPEED` | `8 000.0` px/s | The clamp on an *estimated* velocity. A 400 px drag followed by a 1 ms `tick` is 400 000 px/s, whose `v₀/k` is **50 000 px** — a fling that teleports past the content and is caught only by the clamp. At 8 000 px/s the total is **1 000 px**, about three viewports, which is as far as a flick on a 300-px viewport goes |

**The integration, and the mutation it kills.** With `dt` seconds and the current
velocity `v`:

```rust
let factor = (-MOMENTUM_DECAY_PER_SECOND * seconds).exp();
let travel = Offset::new(
    v.x / MOMENTUM_DECAY_PER_SECOND * (1.0 - factor),
    v.y / MOMENTUM_DECAY_PER_SECOND * (1.0 - factor),
);
// clamp per component, write, then v *= factor
```

**The naive form — `travel = v * dt; v *= factor` — is not a rounding
difference; it is a different number, and the size of it is the reason this task
specifies the integral.** With the naive form the total displacement over `n`
frames is a Riemann sum, `v₀·dt·e^{−k·dt}/(1 − e^{−k·dt})`, whose value depends on
`n`. At `v₀ = 2 000`, `k = 8`: **ten-millisecond frames give ≈ 240 px and
forty-millisecond frames give ≈ 212 px — a 28 px difference, about a tenth of the
viewport, for the identical gesture.** With the exact integral the displacement
over any elapsed time `T` is `v₀/k · (1 − e^{−kT})` **for any frame sequence**, so
**640 ms of travel is `250 · (1 − e^{−5.12})` = 248.5 px, whichever way the 640 ms
was cut up** — and the two naive answers, 280.1 px and 252.1 px once the drag's own
40 px is added, sit on **either side** of it, which is the shape a reviewer should
picture: the exact form is between the two discretisations, not equal to either.

**This is the named test, and it is the one a reviewer should break first.** Its
two frame sequences are chosen to **stop before the cutoff** — at `v₀ = 2 000`,
`k = 8`, the velocity is still **11.97 px/s at 640 ms**, above the 8 px/s cutoff —
so the assertion is about the decay's shape and not about where the cutoff
happened to bite. Its tolerance is **0.01 px**, justified rather than chosen: over
64 steps the `f32` accumulation error is about 6 × 10⁻⁵ px, and a tenth of a
pixel is not a pixel.

**A second, separate test for the cutoff's own frame-rate dependence**, because it
is a different claim. Once the cutoff fires, the last step **overshoots** by up to
`MOMENTUM_STOP_SPEED / MOMENTUM_DECAY_PER_SECOND` = **1.0 px**, and by how much
depends on `dt`. So two sequences run until the momentum stops and their final
offsets agree within **1.1 px**, and **that tolerance is the derived
quantisation, written down**. A `0.01 px` tolerance on that test would be a test
that fails for a correct implementation.

### Why `Easing` is the snap's and not the momentum's

**Decision: the momentum is hand-rolled closed-form arithmetic; the snap is a
`Property::animate_to` through the widget's second `AnimationClock`.** Four
reasons for not routing the momentum through `Animation`, which is the shape a
reader will expect:

1. **A fling's destination is decided on the way, not at the start.** An
   `Animation` needs its target up front — it is `from`, `to`, `duration`, `easing`
   — and a momentum's destination is `min(start + v₀/k, max_scroll)`, which
   depends on the bounds *and* on where the velocity happened to be when the
   cutoff fired. **The widget cannot know the target when the fling starts, so it
   cannot write the `Animation` that would express it.**
2. **Every `Easing` variant fixes both ends** — `Easing::apply`'s own doc, with
   `Spring` as the stated exception — and a fling into a bound must **stop dead**
   there, not arrive at it. An `Animation` always arrives; a fling must be
   interruptible at a bound in the frame it reaches it, which the closed form
   does because the clamp is in the same step as the travel.
3. **`Scroll`'s offset is written by the gesture, by the momentum and by the
   caller.** An `Animation` writes the property from the clock, and
   `AnimationClock::add`'s doc records the collision: *"Starting a second
   animation over one already running therefore leaves both writing it, and the
   last to be ticked wins."* Putting the momentum on the clock makes a gesture
   during a fling a three-way race; keeping it out means **the gesture is the
   only writer besides the momentum and the snap, and the two of those are
   mutually exclusive by construction.**
4. **`Easing::Spring` is simulated rather than solved and its own doc says the
   answer depends on `t` alone**, so it *is* frame-rate independent as a curve —
   but a spring rings past its target and comes back, and a fling that overshoots
   the content and returns is rubber-banding, which § *Out of Scope* excludes by
   name. **A spring is the wrong curve for a release, not a worse one.**

**And the reason the *snap* does use `Easing` is the mirror image:** a snap knows
its target the moment the momentum stops, and a curve that fixes both ends is
exactly what "settle on this point" means. `Easing::EaseOut` is what
`snap_to_points`'s doc recommends, and `Motion` is the parameter, because
`animate_to_state` already takes one and this task does not introduce a second
motion vocabulary.

### When momentum ends out of bounds

**The clamp is in the same step as the travel, and it is the whole of the
out-of-bounds behaviour.** On the frame a component is pinned at `0.0` or at
`max_scroll`, **that component's velocity is set to `Offset::ZERO` and the
momentum for that axis is over.** It does not bounce, it does not overshoot and
come back, and it does not keep pushing against the bound.

**And then, if snap points are set, the nearest one inside the range is armed.**
That is the answer to *"what happens when momentum ends out of bounds"*, and it is
a **snap**, not a rebound: the fling is absorbed by the bound, and the settle is
the same settle a fling in the middle of the content gets. Requirement 12's
`a_momentum_into_the_bound_is_pinned_there_and_ends` is the no-snap-points case
and requirement 23's `a_snap_point_past_the_end_of_the_content_is_not_offered`
is the with-snap-points case.

**What the snap cannot promise, stated here rather than found at review.** A snap
target is chosen and clamped, but **an overshooting `Motion` can leave the offset
property out of range for a few frames** — `Easing::Spring` overshoots by
design. Every consumer clamps anyway (`Scroll::paint`'s thumb through `bounded`,
`visible_rect` and `clamp_scroll` explicitly, `List::offset` explicitly), so the
only unguarded consumer is `apply_offset`, which writes the node's position
unclamped — and **task 45's per-node clip is what stops that from being visible.**
Two named tests make this a measurement rather than a hope:
`an_ease_out_snap_never_leaves_the_range_and_a_spring_one_does`.

### What a caller can read about the momentum

`Scroll::momentum_velocity(&self) -> Offset` and `Scroll::is_coasting(&self) ->
bool`, both `#[must_use]`. `is_coasting` is the test-and-paint path's signal — the
same role `is_animating` plays today — and `momentum_velocity` exists because a
test needs the number and a caller that wants to draw a "coasting" affordance
needs it too. `Scroll::is_animating`'s **meaning widens and its signature does
not**: it becomes *anything about this scroll is moving*, and its doc says so,
because `tick` is the single call that has to be made for either to happen.

## Snap points

**Decision: `Scroll` declares snap points as a sorted, de-duplicated
`Vec<f32>` of offsets on its axis. `List` declares item boundaries and installs
them. Paging is a caller computing `n × page`.** Nothing is overloading
`snap_to_state`.

### What declares them, and why each of the other three cannot

| Candidate | Verdict | The reason, from the type |
|---|---|---|
| **A list of offsets** | **chosen** | It is `Scroll`'s own vocabulary, it sorts, and it answers the only question the widget asks: *which of these is nearest to where the fling stopped* |
| **A callback** | **rejected** | `Callback<T>` is `Rc<dyn Fn(T)>` and returns `()`. A snap needs the widget to **ask** a question, and `widgets/mod.rs`'s own docs draw the line between the crate's two callback types in exactly these words: *"It is deliberately *not* the `Callback<T>` of [`property`]: that one is private, it is `Fn(&T)`, and it is the notification a property fires on every write, which is a different job from an action a widget performs."* A callback here would fire an action per frame of a settle, and the answer would arrive too late to steer the settle it was answering |
| **Item indices** | **`List`'s, not `Scroll`'s** | An index means nothing without an `item_height`, and `Scroll` has never been told one — see § *`List` interaction* |
| **A page size** | **rejected** | A page count depends on the **viewport**, which a setter is not given. See § *Paging* below |

**`set_snap_points` stores, it does not clamp.** It drops a **non-finite** point,
sorts, and de-duplicates, and returns the count it kept. The **range** clamp
happens where the viewport is known, in `nearest_point(rect)`, exactly as
`set_content_height`'s doc explains it cannot do otherwise: *"clamping needs the
viewport's height and the setter is not given a rect."* Two named tests: one for
the setter's sanitising, one that a point past the end is never offered.

**One point set, both axes.** `Axis::Both` scrolls against the **same** points on
whichever axis the fling ran. A two-axis viewport that snaps `x` to pages and `y`
to rows is **two scrolls**, and this task says so rather than inventing a pair of
sets. The card carousel — row `L5`'s first blocked item — snaps one axis, so the
limitation costs it nothing.

### Paging, and the trailing spacer that is deliberately not built

**Paging is not a second API. It is a caller that computes `0, page, 2·page, …`
and hands them to `set_snap_points`.** The distinction the brief asks for is real
and it is this:

- **Snapping to a boundary** — the offsets are given and are not a regular
  lattice. A timeline, a ruler, a chapter strip. The nearest one wins, and *nearest*
  is the whole rule.
- **Paging** — the offsets are `n × page`, a regular lattice, and **the page size
  is the viewport's own extent**. Same mechanism, different numbers, and **the
  caller's numbers** because only the caller knows the viewport's size at the
  moment it decides.

**The one thing paging needs that this task does not build is the trailing spacer,
and the reason is `max_scroll`'s own contract.** A content of 2 500 px in a
1 000-px viewport pages into three pages, and the third is 500 px of content in a
1 000-px page — so to *reach* that page the scroll has to travel to 1 500, past
the content's own end at 2 500 minus the viewport's 1 000. That means growing the
scrollable range **past the content**, and `max_scroll`'s doc is unambiguous about
what it is: *"the content that will not fit"*, with a doctest that says
`max_scroll(300.0, 900.0) == 600.0`. **A paged container's spacer is that
container's decision, and the container — the carousel — is a `TASK_UI_DEMO_n`
item.** So the honest consequence is stated: **a paged caller whose content is not
a whole number of pages can snap to every whole page except the last, and the last
is the carousel task's problem.**

### `List` interaction: the source is `List`'s, the mechanism is `Scroll`'s

**Decision: `List::set_snap_to_items(bool)` installs snap points on the embedded
`Scroll` from `item_height`, and `List` grows no snap logic of its own.** Three
reasons, and the second is the module's own stated principle applied a sixth time.

1. **`List::sync` is the only place in the crate that knows an `item_height` and a
   viewport extent at the same time.** It takes `(&mut self, &mut Arena<WidgetNode>,
   rect: Rect)` and runs once a frame after a layout pass. `Scroll` is told a
   `content_height` and is handed a `rect` at event and paint time, but it is
   never told what a row is. **An item height inside `Scroll` is a parameter with
   no source**, which is the same refusal task 40 made of `Swipe`'s missing
   magnitude.
2. **`list.rs`'s module doc already lists the things a list must not copy from
   `Scroll`**, and the list has six: *"the clamp, the wheel step, the sign of a
   drag, the arrow-key step and the scrollbar all come from that one widget rather
   than from a second copy of any of them."* A seventh copy — "and the snap
   points" — would contradict the sentence that justifies the design. Installing
   them **extends** the sentence, which is what the module doc asks for.
3. **The set has to be refreshed every frame anyway.** The snap points a list wants
   are `i · item_height` clamped into `0..=max_scroll`, and `max_scroll` depends
   on the viewport and on `item_count`. Both change under the list. So the write
   belongs in `sync`, and `sync` is `List`'s.

**The doors follow `List::set_palette`'s shape exactly** — `&mut self`, delegating
to the inner scroll, with a doc that says why the door exists rather than telling
the reader to reach for `scroll()`: `List::snap_to_items(&self) -> bool`,
`List::set_snap_to_items(&mut self, snap: bool) -> bool`, and **no
`List::set_snap_points`**, because a caller with its own boundary list sets them
before handing the list over and `sync` overwrites them only when item snapping is
on. That last clause is a named test, because "the flag wins" is exactly the kind
of rule that is stated in prose and reversed in code.

**`List` gains `tick`** — `List::tick(&self, rect: Rect, delta: Duration) -> bool`,
delegating to `Scroll::tick` — for the reason `List::offset` exists: without it a
list's momentum and its snap are machinery nothing calls, and the list's own
argument for having exactly one offset applies to exactly one motion as well.

### `List` stays vertical, and why that is a decision

**`List` does not gain an axis. It keeps `Axis::Vertical` and says so.** The
reason is arithmetic: a list's four free functions — `content_height`,
`max_scroll_for`, `visible_range`, `index_at` — take `item_height` and
`viewport_height` and are built on `index * item_height` down one axis, which is
what its module doc means by *"The arithmetic in [`visible_range`] is a single
`index * item_height`, which is the whole of why a variable height is a different
function and not a parameter."* **A horizontal list is a grid**, and `LayoutMode::Grid` is row **`L3`**, whose
evidence is verbatim *"**`LayoutMode::Grid` unimplemented** — `LayoutMode::Grid { .. }
=> Vec::new()`, `columns` declared and never read"* — the arm task 52 owns.

**So row `L5`'s *"and `List`"* half is still true afterwards, and the amendment
says so in those words rather than closing the row.** That is not a failure of the
task; it is the honest shape of a row that names two widgets and a row whose
blocked items — the Controls tile grid and the app tray grid — belong to `L3`
rather than to this one.

### Two names, one widget

**`Scroll::snap_to_state` is not renamed, and `snap_to_points` is not a second
meaning of it.** The discriminator is in both docs' first lines, and it is the
preposition:

| | writes | means | first line of its doc |
|---|---|---|---|
| `snap_to_state` | `track` and `thumb`, two `Property<Color>` | **the widget's own appearance**, applied now | *"Applies the appearance the scroll's focus implies at once, with no transition."* |
| `snap_to_points` | `scroll_offset`, one `Property<Offset>` | **where the content rests**, settled over a `Motion` | *"Settles the offset on the nearest snap point, over `motion`."* |

**Why `snap_to_state` stays as it is, and it is not inertia:** `snap_to_state` is
a **crate-wide convention, and `grep -l 'pub fn snap_to_state'
ui/src/ui_core/src/widgets/` returns exactly ten files** — `button`, `chart`,
`gauge`, `image`, `keyboard`, `progress`, `scroll`, `slider`, `text_input` and
`toggle` — and `ui_demo` calls it on every one of those it holds a widget for.
**`Scroll`'s is the eleventh file's worth of a convention ten widgets already
own, and renaming it would be a change to a convention this task did not come to
change**, breaking every one of those call sites for no gain. `list.rs` refers to
it but does not define it, which is why it is ten and not eleven. The gain this
task wants is **distinguishability**, and `state` versus `points` is the whole of
it: **a name beginning `snap_to_` means "move to", and these two move to different
things.**

**And the setter is `set_snap_points`, not `set_snap_to_points`.** A name that
begins `snap_to_` must mean "move to"; a setter moves to nothing.

## Testing without a display

**No test in this task needs a display, a network, a filesystem or a wall clock,
and none needs a pointer event.** `AGENTS.md` permits only the first property and
the sequence requires the second. The mechanisms:

1. **The gesture is driven by constructing an `InputEvent` and calling
   `Scroll::on_event(&mut event, rect)` directly** — the shape `Slider::on_event`
   and `Scroll::on_event`'s own doc examples already use, and the technique tasks
   14 to 23 used for their unwinnable criteria. **An injected `InputEvent` is not
   a pointer event**: it is a value, and no window is involved.
2. **The momentum is driven by stepping `Scroll::tick(rect, delta)` with a
   `Duration` the test chose.** The velocity is therefore *exactly*
   `drag_px / delta_secs`, and `Duration::from_millis(16)` is a number, not a
   measurement. **This is what makes the frame-rate-independence test possible at
   all**: two different `delta` sequences are two different lists of `Duration`s
   handed to the same function.
3. **The snap is driven by `Property::animate_to` through
   `AnimationClock::tick`**, whose `elapsed` is a field and not a wall clock —
   `AnimationClock`'s doc is explicit: *"Animations measure their progress against
   this, which is why an animation's start time is the clock's elapsed time and not
   a wall clock."* **A snap is ticked, not slept for.**
4. **The arena is `Arena::new()`,** which is what every existing widget test
   builds. `Scroll::apply_offset` and `List::sync` read and write nodes through it
   and never need a `Renderer`.

**The four nearest instruments cannot produce this evidence, and saying so is why
the tests are shaped this way.** `magick import` photographs pixels and a momentum
between two frames is in neither; `xwininfo` reports a window; `pgrep` reports a
process; `.ai/tools/fps-check.sh` reports a rate. **A fling is a function of a
velocity, a `Duration` and a pair of bounds, and the only instrument that can
witness that is a test.**

## Scope, measured against `developer.md` § *Scope check*

**Five files and four components — over the component threshold, so this is split
into two sub-tasks** per `.ai/protocols/subagents.md` § *Implementation fan-out*.
**Sequential**, because 46.2 reads three types 46.1 creates (`Axis`,
`Scroll::set_snap_points`, `Scroll::tick`'s new signature), and a subagent briefed
against a type that does not exist **invents it** (§ *Parallel or sequential*).

| Sub-task | Files | Components | Acceptable alone because |
|---|---|---|---|
| **46.1 — the mechanism** | `ui/src/ui_core/src/widgets/scroll.rs`, `ui/src/ui_core/src/animation.rs` | 3: the axis model, the momentum, the snap points | `Scroll` and its seven free functions are testable alone; `animation.rs`'s contribution is one `impl` block with its own doctest, runnable with `cargo test --doc animation` before `scroll.rs` changes at all |
| **46.2 — the list, and the record** | `ui/src/ui_core/src/widgets/list.rs`, `doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md` | 2: the item-snap source; the record | Its acceptance test is that `List`'s item snapping and its `tick` work, which needs 46.1's `Axis` and setters — so it lands **after** 46.1, not "alone" — and the record's acceptance test is **a named grep**, which needs no build |

**File isolation holds**, and it holds in an unusual place worth stating: the
document corrections to `scroll.rs`'s **own module doc** are part of 46.1 because
they live in `scroll.rs`, and the corrections to `list.rs`'s module doc are part
of 46.2 for the same reason. **No two subagents write one file.**

**And because this tree is being modified in parallel, the rule *on a shared
tree, the suite you ran is not your suite* applies to both handoffs:** the shape
that worked there is to copy the workspace to `/tmp`,
delete the other agent's file **and its `mod` line** in the copy, `diff` this
task's files against the copy to prove they are identical, run the gate there
with its own `CARGO_TARGET_DIR`, and report both trees. **A green suite on a
shared tree is not evidence about this task until the tree is named.**

**If the implementer finds themselves editing a file outside the five above,
that is a stop condition** (`developer.md` § *Stop conditions*) — in particular
`ui/src/ui_core/src/render.rs`, `render/target.rs`, `render/context.rs`,
`paint.rs`, `input.rs`, `node.rs`, `property.rs` and every other `widgets/*.rs`
are out. **If task 45 has not landed, this task does not wait for it and does not
implement any part of it** — the horizontal viewport needs the per-node clip the
same way the vertical one does, and task 45 owns it.

## Requirements

### Sub-task 46.1 — the mechanism

1. **`pub scroll_offset: Property<Offset>`**, replacing the `Property<f32>`. Its
   doc is **rewritten, not deleted**, and keeps the whole of its present text that
   is still true: the *"down is later"* convention, zero-is-the-top, the maximum
   is `max_scroll`, *"Every write the widget makes is clamped"*, and the sentence
   about a caller writing the property directly. **The two sentences this task
   makes false are the ones that go**: *"the content is drawn at
   `rect.y - scroll_offset`"* becomes *"drawn at `rect.origin - scroll_offset`"*,
   and the struct-level claim that the offset *"is **never** animated"* is amended
   to say **what is true afterwards** — the gesture and the momentum write it
   immediately, and **only a snap animates it, and only while it is settling.**
   `Scroll::new` writes `Property::new(Offset::ZERO)`.

2. **`pub enum Axis` in `scroll.rs`,** beside `Palette` and `Style`:

   ```rust
   /// Which axes this viewport scrolls, and which gestures and keys it answers.
   ///
   /// It is a **caller's claim**, not something the widget infers: a content
   /// wider than its viewport is not a horizontal scroll, because a viewport
   /// that scrolls sideways because its content overflowed is a widget with a
   /// layout bug and a second scrollbar. Zero is the top on every axis and the
   /// maximum is [`max_scroll`]'s component for it, so nothing here carries an
   /// opinion about which end is "more".
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub enum Axis {
       /// The content moves up and down. The default, and what every scroll
       /// was before this type existed.
       #[default]
       Vertical,
       /// The content moves left and right.
       Horizontal,
       /// Both, so one viewport carries two live offsets — which is what a card
       /// carousel is, and the reason [`slider::Orientation`](crate::widgets::slider::Orientation)
       /// cannot be reused here: it has no third value, and a `Both` on a
       /// slider's axis would be a state no slider can produce.
       Both,
   }
   ```

   `Axis` carries a **doctest** that drives one horizontal wheel and one
   horizontal drag through `Scroll::on_event` and asserts both consumed the event
   and moved `x` and left `y` bit-identical.

3. **`Scroll`'s axis is a private field with a getter and a setter**, matching
   `Slider::orientation` and `Progress::orientation` exactly:

   - `pub fn axis(&self) -> Axis`, `#[must_use]`.
   - `pub fn set_axis(&mut self, axis: Axis) -> Axis`, returning what it set.
     **It takes `&mut self` and it is private behind that door, for
     `progress.rs`'s recorded reason**: *"deliberately behind a **setter rather
     than a `Property`** so it cannot change mid-frame and leave the mode
     disagreeing with the properties drawn."* An axis that changed between a
     gesture and its `apply_offset` is a content node moved on one axis and
     clamped on the other.
   - **`set_axis` writes `content_width` when the axis admits `x` and `0.0` when
     it does not**, so a caller cannot leave a stale horizontal content extent
     behind — and its doc says that in one sentence, because the silent case is
     the defect.

4. **A new private `content_width: f32` and its two doors — `pub fn
   content_width(&self) -> f32` and `pub fn set_content_width(&mut self, width:
   f32) -> f32`** in `content_height`'s and `set_content_height`'s exact
   shape, floor at zero, and **`set_content_height`'s doc gains the sentence about
   the re-clamp for its new twin.** `Scroll::sync_content` reads
   **`self.content_size(nodes)` once** and writes **both** components from that
   one `Size`, reporting whether **either** changed. `Scroll::new` writes
   `content_width: 0.0` beside `content_height: 0.0`.

5. **The five free functions become axis-aware, keeping their names:**

   - `pub fn max_scroll(viewport: Size, content: Size) -> Offset` —
     `Offset::new((content.width - viewport.width).max(0.0), (content.height -
     viewport.height).max(0.0))`. **Its doc keeps its `NaN` sentence** —
     *"[`f32::max`] returns its non-`NaN` operand"* is now true per component and
     is worth keeping — and its doctest becomes three assertions, one per axis
     plus the `NaN` case.
   - `pub fn clamp_scroll(offset: Offset, viewport: Size, content: Size) ->
     Offset` — **two comparisons per component**, `offset.x.max(0.0).min(max.x)`
     and the same for `y`, **for the reason the present doc gives**: *"`clamp`
     panics when its bounds are the wrong way round, and a widget whose caller has
     the wrong way up must not take a frame down to say so. `max` then `min` is
     the same answer for an ordered pair and cannot panic, and a `NaN` in either is
     passed over rather than propagated."* **The doc's sentence is kept, and the
     fact that makes it necessary is worth writing down: `layout::Offset` carries
     no arithmetic at all** — it derives `Clone, Copy, Debug, Default, PartialEq`
     and has `new` and `ZERO`, so there is no `Offset::min` to write and no `Ord`
     to derive from. **Adding one is explicitly not this task**: an arithmetic
     surface on a shared layout type for two call sites in one widget is
     `developer.md` § *Phase 2*'s *"no abstraction before the second use"*, and
     the two lines are also the two lines a reviewer reads to check the clamp.
   - `pub fn visible_rect(viewport: Rect, content: Size, scroll_offset: Offset) ->
     Rect` — clamps the offset **per component**, and the band's origin is that
     clamped offset's `x` and `y` **in the content's own coordinates**, with its
     extent each axis's minimum of the viewport and what is left, floored at zero.
     **There is no `min` between the two axes** — a viewport wider than its
     content still shows the content's whole width, and a viewport taller than its
     content still shows the content's whole height, which is what today's
     `visible_rect` already does for the vertical axis. Its doctest gains a
     horizontal case and keeps all four vertical ones.
   - `pub fn gesture_delta(delta: Offset, axis: Axis) -> Offset` — three arms, the
     table in § *`Axis::Vertical` keeps the two refusals* as the doc's table.
     **`gesture_delta`'s existing doctest gains the `Both` and the horizontal
     assertions** and its *"This is the direction rule, in one place, and it is the
     only copy"* sentence stays, with the count updated from two arms to three.
   - `pub fn wheel_delta(delta: Offset, axis: Axis) -> Offset` — **per component,
     and a zero component contributes `0.0`**, for the reason in § *The wheel's
     sign on each axis*. Its doctest **grows a zero-component assertion** and its
     doc records that this is a behaviour change to an unreachable case.

6. **The private geometry stays scalar and is called with the axis's numbers —
   one copy, not two.** `thumb_length(&self, viewport: f32, content: f32) -> f32`,
   `thumb_run(&self, viewport: f32, thumb_length: f32) -> f32`,
   `track_rect(&self, rect: Rect) -> Rect` (vertical, unchanged),
   **`track_rect_h(&self, rect: Rect) -> Rect`** (new: the bottom-edge strip, `Rect::new(rect.x, rect.y + rect.height - SCROLLBAR_MARGIN - self.thickness, rect.width, self.thickness)`),
   `thumb_rect` and `thumb_rect_h`, `offset_under` and `offset_under_h`.
   **`SCROLLBAR_MARGIN`'s doc gains one sentence**: it keeps the focus ring
   inside the node on the bottom edge for the same reason it does on the right.
   **`SCROLLBAR_MIN_THUMB`'s doc is extended to say the same thing holds on the
   horizontal axis** rather than being duplicated.

7. **`Scroll::scroll_by(&self, rect: Rect, delta: Offset) -> Offset`,**
   `#[must_use]`, clamping **per component** through `clamp_scroll` and returning
   what it settled on. **`Offset::ZERO` is still the re-clamp**, and its doc's
   sentence *"A delta of zero is this widget's **re-clamp**"* is kept verbatim.
   **`Scroll::max_scroll_for(&self, rect: Rect) -> Offset`,** `#[must_use]`.

8. **`Scroll::apply_offset` reads the offset once and writes its negation** —
   `let offset = self.scroll_offset.get(); let target =
   Offset::new(-offset.x, -offset.y);` — which is one negation of a vector and
   **no axis branch at all**, and the local read is a simplification the vector
   makes free rather than a change: today's body calls `.get()` once for the
   comparison and once for the target. Its doc's *"Nothing animates the offset, so
   the two cannot disagree"* is amended: the node still gets the property's own
   value every time, **and now a snap can move it between `apply_offset` calls,
   which is the point of a settle.**

9. **`Scroll::content_rect(&self, rect: Rect) -> Rect`** shifts its origin by
   **both** components — `Rect::new(rect.x - offset.x, rect.y - offset.y, …)` —
   and its extent becomes **the content's own `Size` on every axis**, which is
   what makes it correct for all three without a branch. Its doctest keeps both
   existing assertions and gains a horizontal one.

10. **The three private momentum fields**, each a `Cell` with `Scroll::grabbed`
    and the arena-borrow doc as the precedent, since `on_event` takes `&self` and
    cannot reach anything:

    ```rust
    /// The drag displacement this frame has carried but not yet been timed
    /// against, in pixels, because [`input::InputEvent`](crate::input::InputEvent)
    /// carries no timestamp and a velocity needs a duration.
    drag_pending: Cell<Offset>,
    /// The velocity the last timed drag produced, in pixels per second. Zero is
    /// not coasting.
    velocity: Cell<Offset>,
    /// Whether a momentum step pinned an axis at a bound, or fell under the
    /// stop speed, and owes a settle. Cleared by [`Scroll::snap_to_points`] on
    /// the same frame it arms one.
    snap_pending: Cell<bool>,
    ```

    **`snap_pending` exists because the fling's landing place is only known after
    the frame that stops it, and arming the animation has to happen on the same
    frame the momentum is zeroed** — otherwise the settle begins a frame after the
    content has already stopped, which is two stalls.

11. **The snap-point storage: `snap_points: Vec<f32>`,** with

    - `pub fn snap_points(&self) -> &[f32]`, `#[must_use]` — the stored,
      sorted, de-duplicated slice, in that order, and **the field is private so
      the order is a fact rather than a hope**.
    - `pub fn set_snap_points(&mut self, points: &[f32]) -> usize`, `#[must_use]` —
      **drops every non-finite point, sorts, de-duplicates, stores, returns the
      count kept.** Its doc repeats `set_content_height`'s reason for not
      clamping, and says that **the range clamp happens in
      `nearest_point(rect)`**, which is why the returned count is not the count
      used.
    - `pub fn has_snap_points(&self) -> bool`, `#[must_use]`.
    - **`fn nearest_point(&self, offset: f32, rect: Rect) -> Option<f32>`**,
      private: the nearest stored point in `0.0..=max_scroll` to the
      **already-clamped** `offset`, with **a tie going to the earlier point**, and
      `None` when the set is empty or every point is out of range. **It takes the
      component the settle is on, and `snap_to_points` chooses it**: the axis the
      scroll declares for `Vertical` and `Horizontal`, and for `Axis::Both` **the
      axis whose velocity was larger in magnitude, a tie going to `y`** — because a
      two-axis viewport settles on the way it was going hardest, and the tie is
      pinned by this file rather than left to `f32::max`'s argument order.

12. **`Scroll::snap_to_points(&self, rect: Rect, motion: Motion) -> bool`,
    `#[must_use]`,** the settle entry point. It clamps the current offset, asks
    `nearest_point` for a target, **`None` returning `false` with nothing
    written**, and otherwise **clears the offset clock, zeroes the velocity, adds
    `self.scroll_offset.animate_to(target, motion.duration, motion.easing)` to the
    offset clock**, and returns `true`. Its doc: it is *"the settle half of a
    release"*, it does **not** clear a running palette transition **because the
    offset has its own clock**, and **`Easing::EaseOut` is the recommended easing**
    with the `Spring` overshoot named as the one curve whose intermediate frames
    leave the range.

    **It clears `velocity`, `drag_pending` and `snap_pending` on the way in, and
    that is load-bearing rather than tidy**: each of the three is a way for
    something to re-arm the motion this call is about to replace, and the snap
    pending flag in particular would otherwise make `tick` re-arm the same settle
    on every subsequent frame — a settle that restarts forever, which is the same
    "a transition restarted every frame creeps toward its target and never
    arrives" defect `DEMO_APPLICATION.md` and `IMPLEMENTATION_STATE.md` record for
    `sync_toggle_state`.

    **The automatic settle's `Motion` is `SNAP_MOTION`, a private constant beside
    the momentum's three**, and its doc carries the arithmetic:
    `Motion { duration: Duration::from_millis(200), easing: Easing::EaseOut }`. On
    `EaseOut`, `apply(0.5) == 0.75`, so half the time covers three quarters of the
    distance — **a settle that commits early and arrives gently, which is what
    "settle" means**, and the difference from `Motion::from_theme`'s 150 ms is the
    distance: a theme switch moves two colours, a settle moves up to a few hundred
    pixels. **200 ms is also under the ~250 ms window `MOMENTUM_DECAY_PER_SECOND`
    gives a fling**, so a settle and the next gesture do not overlap with the
    previous settle still running. `snap_to_points` takes a `Motion` because a
    caller may want its own — `animate_to_state` takes one for the same reason and
    this task does not introduce a second motion vocabulary.

13. **A second `AnimationClock`,** `offset_clock: RefCell<AnimationClock>`,
    initialised `AnimationClock::new()` in `Scroll::new`. **Its doc is the reason
    it exists**: `animate_to_state` and a snap would otherwise share one clock,
    and `AnimationClock::add`'s own rule is that two animations over one property
    both write it and the last ticked wins — **so a theme switch would cancel a
    settle and a settle would cancel a theme switch.** **The per-frame cost is
    stated, because a new field in a per-frame path is exactly what
    `reviewer.md` § *Performance and idioms* asks for a number on**: ticking a
    clock with no animations is `animations.retain(..)` over an empty `Vec` plus
    one `Duration` addition, so the second clock costs **nothing measurable** —
    which is why the fps criterion below expects no movement and says what the
    movement would mean if there were any.

14. **`Scroll::tick(&self, rect: Rect, delta: Duration) -> bool`,** `#[must_use]`,
    in **this order and no other**, and the order is load-bearing:

    ```rust
    pub fn tick(&self, rect: Rect, delta: Duration) -> bool {
        // 1. A drag this frame becomes a velocity. This runs **first** because
        //    the drag is the newest information the widget has and it must not
        //    wait a frame behind a momentum it just replaced.
        // 2. Otherwise an armed momentum takes one exact-decay step, clamped
        //    per component, with a component pinned at a bound zeroed and
        //    `snap_pending` set.
        // 3. A `snap_pending` arms `snap_to_points(rect, SNAP_MOTION)` and
        //    clears the flag — **on the frame the momentum stopped**, not the
        //    frame after.
        // 4. The offset clock ticks, which is the settle's first movement.
        // 5. The appearance clock ticks, exactly as before.
        //    `true` if any of 1, 2, 4 or 5 wrote.
    }
    ```

    **Steps 3 and 4 are in that order because it is what makes the settle move on
    the frame it is armed, and the reason is `AnimationClock`'s own stamping rule.**
    `AnimationClock::add`'s doc: *"The start time is the clock's elapsed time"* —
    so an animation added at step 3 is stamped with the elapsed time **before**
    step 4 advances it, and step 4's write therefore lands at
    `progress = delta / duration`, which is non-zero. **Swap the two steps and the
    settle's first written value is `Easing::apply(0.0)`, which is `0.0` on every
    variant, and the content sits still for one frame between the fling stopping
    and the settle starting** — the same flat `IMPLEMENTATION_STATE.md` records for
    a transition whose clock is stamped after the tick. **There is therefore no
    dead frame, and the test that pins it is
    `a_snap_lands_exactly_on_its_point_after_its_duration`, which ticks once per
    frame and counts the frames it took.**

    **`tick`'s doc is rewritten** and states, in its first lines, that scrolling
    **does** go through here now, replacing the present *"Scrolling does **not**
    go through here. The offset is not animated, so an interaction has nothing to
    tick"*, which this task makes false.

15. **`Scroll::is_coasting(&self) -> bool` and `Scroll::momentum_velocity(&self)
    -> Offset`,** both `#[must_use]`, and **`Scroll::is_animating`'s doc is
    widened** to *"whether anything about this scroll is still moving"* — its
    signature is unchanged, because widening a meaning is a documentation change
    and renaming it would break every caller for no gain.

16. **`Scroll::on_event`'s three arms, axis-aware,** with the rules table written
    into the doc as that table, verbatim, including the two refusals and the
    three producers' cancellation rule:

    - **`Drag { delta }`** — the grabbed-thumb branch first, on its own axis
      (`offset_under` on `y` for `Vertical`, `offset_under_h` on `x` for
      `Horizontal`, **both for `Both`, in that order**); then
      `gesture_delta(delta, self.axis)`, and **`if that is `Offset::ZERO` return
      `false` unconsumed`** — **the zero-test moves from `delta.y == 0.0` to the
      gesture's own result**, which is the same refusal expressed once for all
      three axes and cannot be right on one of them and wrong on the others. The
      **`applied`** delta goes into `drag_pending`, not the raw one, so a
      `Vertical` scroll does not time a horizontal component it refused. **It
      also zeroes `velocity`**, so there is no frame in which a stale fling and a
      new gesture both write the offset: `tick`'s step 1 replaces the velocity with
      the one this frame's drag measured, and **the gesture wins over a fling in
      the same frame**, which is the only correct precedence — a finger on the
      glass outranks a fling it interrupted.
    - **`Scroll { delta }`** — `wheel_delta(delta, self.axis)`;
      `Offset::ZERO` returns `false` unconsumed; a hit consumes, **clears
      `drag_pending` and `velocity`** (the cancellation rule of § *Momentum*),
      and does **not** arm a momentum.
    - **`KeyDown { key, .. }`** — `if !self.focused.get() { return false; }`, then
      `scroll_key(&key, self.axis)` returning an `Offset` step, `None` returning
      `false` unconsumed, a hit consuming and calling `scroll_by`, **and clearing
      `drag_pending` and `velocity`** like the wheel arm.
    - **Six variants reach `_ => false`:** `Tap`, `LongPress`, `Swipe`, `Pinch`,
      `KeyUp` and `Text`. **Each is named in the arm's doc with why it is
      declined**, and `Pinch` keeps the reason task 40's `Rotator` recorded for it
      — a pinch is a zoom, and a zoom is a projection change, which is gap `L2`'s
      and `L6b`'s subject, not this task's.

17. **`fn scroll_key(key: &Key, axis: Axis) -> Option<Offset>`,** private, its
    current doc **rewritten** because *"Left and right are deliberately absent.
    This widget scrolls vertically"* becomes false: all four arrows and four D-pad
    directions, each mapped to the component of its axis, `None` on `Vertical` for
    the horizontal pair. **The sign convention stays in this one function's doc
    and in one named test**, per `gesture_delta`'s discipline.

18. **`Scroll::paint` draws up to two bars,** in this order: **the vertical bar,
    then the horizontal bar**, then the two focus rings and their thumbs. The
    order is stated because a thumb drawn under a bar is a scrollbar nobody can
    see — the same argument `List::paint` makes about rows, one level up.
    **`Scroll::paint` returns `Vec::new()` only when neither axis has anywhere
    to go**, so `scrollbar_rect`'s `None`-means-no-scrollbar contract holds for
    the pair and its doctest gains both axes. **No `DrawCommand` variant is added
    and no pipeline file is touched**, because every primitive is an axis-aligned
    quad — and the `Scope held` acceptance criterion's `git diff --stat` is what
    checks that, rather than this sentence.

19. **`impl Interpolate for Offset` in
    `ui/src/ui_core/src/animation.rs`,** beside the three existing impls, four
    lines through `f32::interpolate` on each component, with a **doctest**
    asserting the midpoint of `Offset::new(0.0, 0.0)` and
    `Offset::new(100.0, 200.0)` is `Offset::new(50.0, 100.0)`. Its doc names its
    one caller — *"the snap animates a scroll's offset, and this is the only
    `Interpolate` a scroll needs"* — and states that **`t` is not clamped here**,
    which is the trait's own contract and which is what lets `Spring` overshoot
    (requirement 12's honest limit).

20. **`scroll.rs`'s module doc is rewritten, not amended.** It gains: the axis
    model and why the offset is a vector; **the momentum model in full** — where
    the velocity comes from and why `InputEvent` cannot carry a timestamp, the
    three constants with their arithmetic, the exact integral and the 28 px the
    naive form costs; **why `Easing` is the snap's and not the momentum's**, with
    the four reasons; the snap-point model and **that a callback cannot express it
    because `Callback<T>` returns `()`**; **the boundary/paging distinction and
    the trailing spacer that is not built**; **the two refusals and why they stay**;
    and the deletion of *"**Horizontal scrolling**"* from *What is deliberately not
    here*, leaving grid content, pull-to-refresh and sticky headers. It **loses**
    *"**Momentum.** Requirement 3 marks smooth scrolling with momentum optional and
    this task does not ask for it"*.

21. **`scroll.rs`'s stale clipping claims are corrected here, by the same eleven
    by symbol task 45 names** — `Scroll::clip_rect`'s *"Applying this rect is that
    task's job"* and `clip_commands`'s *"until that scissor is applied"* among
    them. **If task 45 has landed, this requirement is already satisfied and the
    implementer says so and moves on; if it has not, this task corrects the two
    claims in `scroll.rs` and touches nothing of task 45's.** It never edits the
    same sentence twice, and `git diff` shows which of the two happened.

22. **`List` gains `tick` and two snap doors,** in `ui/src/ui_core/src/widgets/list.rs`:

    - `pub fn tick(&self, rect: Rect, delta: Duration) -> bool`, `#[must_use]`,
      delegating to `Scroll::tick` and returning what it returned. Its doc says
      **why the list needs it**: without it the momentum and the snap are
      machinery nothing calls, and `List::offset`'s own *"one offset is the only
      honest number here"* applies to one motion as well as to one position.
    - `snap_to_items: bool`, private, **false** in `List::new`.
    - `pub fn snap_to_items(&self) -> bool`, `#[must_use]`.
    - `pub fn set_snap_to_items(&mut self, snap: bool) -> bool`, `#[must_use]`,
      in `List::set_palette`'s shape and with its door explained.
    - **`List::sync` writes the snap points on every call, after its re-clamp and
      before `visible_range`**, because the points are `i · item_height` clamped
      into `0..=max_scroll` and `max_scroll` needs the rect the method was given.
      **It writes only when `snap_to_items` is true, and it clears the set when it
      is false**, so turning the flag off cannot leave stale points behind.
    - **`List::offset`, `List::max_scroll_for`, `List::visible_range`,
      `List::item_rect`, `List::item_at`, `List::clip_rect` and `List::sync`'s
      signature, and all four free functions, keep their `f32` signatures.**
      `List::offset`'s body becomes `.y` off the clamped `Offset`, and
      `List::max_scroll_for`'s becomes `scroll::max_scroll(rect.size(),
      …).y` — **the `x` half it drops is `0.0` on a list that cannot scroll
      sideways, which is the point of § *`List` stays vertical*, and the doc says
      so rather than leaving a reader to wonder whether something was lost.**
      **The public surface of `List` grows by exactly three methods and breaks
      nothing.**
    - `list.rs`'s module doc's *"Scrolling is `Scroll`'s"* section gains the snap
      points as its **seventh** borrowed thing, and *What is deliberately not
      here* loses *"**Momentum.** [`Scroll::on_event`] has none and this does not
      add any"* and gains *"**A horizontal axis**, and a variable item height: the
      arithmetic in [`visible_range`] is a single `index * item_height` down one
      axis, and a list across is the grid of row `L3`."*

23. **The tests, named, with no display, no network, no filesystem, no wall clock
    and no pointer event.** **Each names the mutation it kills**, per
    `developer.md` § Phase 3 (*"A test that has never failed is not a test"*). In
    `scroll.rs`'s `#[cfg(test)] mod tests`:

    **Axis**
    - `a_horizontal_scroll_scrolls_on_x_and_leaves_y_bit_identical` — a synthetic
      `Drag { delta: Offset::new(40.0, 7.0) }` through `on_event` on an
      `Axis::Horizontal` scroll: consumed, `x` moved by **exactly** `40.0`, `y`
      **bit-identical**. **Mutation:** reading `delta.y` on both axes.
    - `a_two_axis_scroll_moves_both_components_of_one_drag` — `Axis::Both`,
      `Offset::new(30.0, 40.0)`, both moved. **Mutation:** dominant-axis
      selection, which is the tempting one.
    - `a_horizontal_scroll_refuses_a_vertical_drag_and_leaves_it_unconsumed` —
      `Axis::Horizontal`, `Offset::new(0.0, 40.0)`: `false`, unconsumed, offset
      unchanged. **The mirror of the existing
      `a_drag_that_is_entirely_horizontal_is_left_for_a_horizontal_scroller`,
      which stays.**
    - `a_horizontal_wheel_moves_a_horizontal_scroll_the_way_a_vertical_wheel_moves_a_vertical_one`
      — `Offset::new(-1.0, 0.0)` and `Offset::new(1.0, 0.0)` through `on_event`,
      asserting the **sign**, and `wheel_delta(Offset::new(0.0, -1.0),
      Axis::Horizontal) == Offset::ZERO`. **Mutation:** the missing per-component
      negation, which is a wheel and a finger disagreeing on one control — the
      exact failure `gesture_delta`'s doc records deciding the sign the first time.
    - `a_horizontal_scroll_clamps_its_x_at_both_ends` — both halves, and a further
      drag past the end leaves it unchanged, so a clamp that leaks is caught as
      well as one that is absent.
    - `a_horizontal_scrollbar_is_drawn_along_the_bottom_edge_and_a_vertical_one_is_not`
      — on a `Horizontal` scroll `scrollbar_rect(rect)` is the bottom strip and
      `paint` records a groove there; on a `Vertical` scroll it is the right strip;
      on a scroll with nowhere to go on either axis `paint` records **nothing**.
    - `apply_offset_writes_the_negated_vector_and_a_horizontal_offset_moves_the_content_sideways`
      — the arena read, and the assertion that `x` moves the node's `x`.
    - `a_content_rect_shifts_on_both_axes` — requirement 9's doctest's twin, in
      the suite.

    **Momentum**
    - **`a_momentum_travels_the_same_distance_at_ten_milliseconds_a_frame_and_at_forty`**
      — **the required one.** One drag of 40 px, then `tick` with **64 × 10 ms**
      and with **16 × 40 ms** — both 640 ms — from identical starts. Asserts the
      two final offsets agree within **0.01 px**, and asserts **both** equal the
      closed form `40 + v₀/k · (1 − e^{−kT})` within 1 px. **Mutation:** `travel =
      v * dt`, which gives ≈ 240 px against ≈ 212 px of travel — **a 28 px
      difference, and this is the mutation the whole model exists to kill.**
    - `a_momentum_that_crosses_the_cutoff_stops_at_the_same_offset_on_both_frame_rates`
      — the same two sequences run **until `is_coasting()` is false**, asserting
      agreement within **1.1 px** — the derived `MOMENTUM_STOP_SPEED /
      MOMENTUM_DECAY_PER_SECOND` quantisation of the final step — and that
      `momentum_velocity()` is **exactly** `Offset::ZERO` at the end on both.
      **Its doc records why its tolerance is 1.1 px and not 0.01**, because that
      is the difference between a test that can fail and one that cannot.
    - `the_velocity_is_the_drag_this_frame_carried_over_the_frame` — one 40-px
      drag, one `tick` of `Duration::from_millis(16)`, and
      `momentum_velocity() == Offset::new(0.0, 2500.0)` **exactly** (40 / 0.016).
      **Mutation:** estimating in `on_event`, which has no duration to divide by.
    - `the_momentum_travels_the_closed_form_and_stops_within_a_pixel_of_it` — a
      run to the cutoff against `v₀/k · (1 − e^{−k·t_stop})`, within **1.0 px**,
      the same derivation.
    - `a_momentum_into_the_bound_is_pinned_there_and_ends` — a fling aimed past
      `max_scroll`: the offset is **exactly** `max_scroll`, `momentum_velocity()`
      is `Offset::ZERO` **on that same frame**, and a further `tick` moves nothing.
      **Mutation:** letting the velocity survive the clamp, which is a scroll
      that presses against its end forever.
    - `a_wheel_notch_and_a_key_press_cancel_a_momentum_and_start_none` — both
      halves, each with a focused scroll and a running momentum. **Mutation:**
      letting a discrete request coexist with a fling.
    - `a_drag_on_a_grabbed_thumb_arms_the_same_momentum_a_drag_on_the_content_does`
      — two identical scrolls, one dragged on the content and one on a grabbed
      thumb for the same displacement, asserting the same velocity. **Mutation:**
      a second accumulator on the thumb path.
    - `a_zero_tick_neither_arms_a_velocity_nor_moves_the_content` — and asserts
      `momentum_velocity()` is **finite**, which is the `inf` the division would
      produce.
    - `the_velocity_is_capped_at_the_maximum_and_the_cap_is_not_reachable_from_a_real_drag`
      — a 400-px drag and a 1 ms tick cannot produce more than
      `MOMENTUM_MAX_SPEED`, and `MOMENTUM_MAX_SPEED / MOMENTUM_DECAY_PER_SECOND` is
      asserted to be **1 000.0**.
    - `tick_reports_true_while_the_content_coasts_and_false_the_frame_after_it_stops`
      — and `is_coasting()` agrees with it on every frame.

    **Snap**
    - **`a_snap_settles_on_the_nearest_point_when_the_momentum_stops`** — the
      required one. Snap points `{0, 100, 200, 300, 400, 500, 600}` on a 200 × 300
      viewport over 900 of content; an 8-px drag then ticks until the momentum
      stops, landing near **70**, and the settle targets **100** — which
      **`floor`-based snapping would not choose**, since `floor(70/100)·100` is
      `0`. Asserts the offset is **exactly `100.0`** after the motion's duration
      has been ticked.
    - `a_snap_lands_exactly_on_its_point_after_its_duration` — the `Easing` fixed
      end, on the **last** tick: `Easing::apply(1.0) == 1.0` reached and held.
    - `set_snap_points_keeps_the_sorted_distinct_finite_ones_and_drops_the_rest`
      — six inputs including `f32::NAN`, `f32::INFINITY`, a duplicate and an
      out-of-order pair, asserting the returned count and the stored slice.
    - `a_scroll_with_no_snap_points_stops_where_its_momentum_stopped` — the
      negative case, and **the guard on requirement 12**: with an empty set,
      `snap_to_points` returns `false` and writes nothing.
    - `the_nearest_point_ties_go_to_the_earlier_one` — an offset exactly between
      two points picks the lower, asserted in both directions of approach.
    - `a_snap_point_past_the_end_of_the_content_is_not_offered` — a set including
      `5_000.0` in a 600-px range, and a fling into the end settles on the
      **highest in-range** point.
    - `an_ease_out_snap_never_leaves_the_range_and_a_spring_one_does` — both
      halves, **ticked frame by frame and asserting the property's value on every
      one**, which is what turns requirement 12's honest limit into a measurement.
    - `a_snap_and_a_palette_transition_can_run_at_once_because_they_have_two_clocks`
      — both clocks ticking, both writing, neither clearing the other.
    - `a_snap_to_points_can_be_asked_for_directly_with_no_momentum_at_all` — the
      caller-facing entry point.

    **Keys**
    - `left_and_right_scroll_a_horizontal_scroll_and_are_refused_by_a_vertical_one`
      — both halves, and the existing
      `a_horizontal_wheel_is_not_a_vertical_scroll` and
      `down_is_later_and_a_wheel_agrees_with_a_finger` **keep their names and
      their assertions**.
    - `a_two_axis_scroll_takes_all_four_arrows_each_on_its_own_axis` — each
      arrow's component asserted against the property's other half, **bit-identical**.
    - `an_unfocused_scroll_refuses_a_horizontal_key_as_it_does_a_vertical_one`.

    **The whole event surface, in one table**
    - `the_scroll_consumes_only_a_drag_a_wheel_and_a_focused_key` — all nine
      `InputEventKind` variants offered to `on_event` on an `Axis::Both` scroll,
      asserting that only `Drag`, `Scroll` and a **focused** `KeyDown` are
      consumed, and that `Tap`, `LongPress`, `Swipe`, `Pinch`, `KeyUp` and `Text`
      each leave the event **unconsumed** — **`Swipe` and `LongPress` asserted by
      name**, because those are the two row `L4` names and a later task that
      consumes them must break this test deliberately rather than drift past a
      claim in a document. This is task 40's
      `the_widget_consumes_nothing_but_a_drag` applied to `Scroll`, and the
      `Axis::Both` case is the one that makes it worth having.

    **The constants**
    - `the_momentum_constants_and_the_snap_motion_are_finite_and_positive` —
      `MOMENTUM_DECAY_PER_SECOND > 0.0`, `MOMENTUM_STOP_SPEED > 0.0`,
      `MOMENTUM_MAX_SPEED > MOMENTUM_STOP_SPEED`,
      `MOMENTUM_MAX_SPEED / MOMENTUM_DECAY_PER_SECOND == 1_000.0`,
      `SNAP_MOTION.duration` non-zero and `Easing::apply(1.0) == 1.0`. **The last
      ratio is the one that bites**: it is the maximum travel a capped fling can
      add, and a future edit to either constant moves it.

    **Preserved, asserted by a diff and not by a new test**
    - `a_drag_that_is_entirely_horizontal_is_left_for_a_horizontal_scroller`,
      `a_horizontal_wheel_is_not_a_vertical_scroll`,
      `a_wheels_size_is_not_used_only_its_direction`,
      `a_wheel_notch_and_a_drag_of_the_same_size_move_a_scroll_alike`,
      `a_drag_is_clamped_at_both_ends`,
      `a_drag_moves_the_offset_the_content_rect_and_the_node_together` and
      `visible_rect_is_the_band_of_content_the_viewport_shows` — **all six keep
      their names and every one of their assertions**, with `.get()` gaining `.y`
      and `set(60.0)` gaining `Offset::new(0.0, 60.0)`. **A diff weakening any of
      them is a stop condition, and the handoff lists all six by name with their
      new assertion counts.** And `the_scroll_consumes_only_a_drag_a_wheel_and_a_focused_key`
      **passes**, so row `L4`'s `LongPress` and `Swipe` claim is enforced by a test
      on this widget too, not only on `List` and `Keyboard`.

    In `ui/src/ui_core/src/widgets/list.rs`'s `#[cfg(test)] mod tests`:
    - `a_list_with_snap_to_items_settles_on_a_row_boundary` — 100 rows of 40 in a
      200 × 300 viewport; after a fling, the settle lands on a **multiple of 40**
      inside `0..=3_680`, asserted exactly.
    - `a_list_without_snap_to_items_stops_where_its_momentum_stopped` — the
      negative case.
    - `a_list_ticks_the_scroll_it_embeds_and_touches_nothing_of_its_own` — the
      delegation, plus `visible_items()` unchanged.
    - **`turning_item_snapping_off_clears_the_points_rather_than_leaving_them`**
      — `set_snap_to_items(true)`, one `sync`, `set_snap_to_items(false)`, one
      `sync`, and `has_snap_points()` is false. **Mutation:** an early return
      that leaves the previous set installed, which is invisible until the list
      stops snapping to rows it no longer has.
    - `a_list_is_vertical_and_still_refuses_a_horizontal_wheel` — the honest-limit
      test, and the thing a later grid task must break deliberately.

    **The four new doctests:** `impl Interpolate for Offset`, `Axis`,
    `Scroll::tick`, `Scroll::snap_to_points`.

    **The count, stated once so a measured number can be checked against it:
    thirty-two new unit tests in `scroll.rs`** (eight axis, ten momentum, nine
    snap, three keys, one event table, one constants) **and five in `list.rs`, so
    thirty-seven; four new doctests. 1894 + 41 = 1935. A measured number other
    than 1935 is corrected in this file rather than argued about**, per task 45's
    criterion.

24. **The two documents are amended, dated and attributed.**

    - **`doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*,
      row `L5`** gains a dated note naming `TASK_UI_PRIM_46` and stating: the
      offset is a `Property<Offset>` and the axis is `scroll::Axis`; the momentum
      is a closed-form exponential with its three constants; the snap points are
      `Vec<f32>` and **`snap_to_state` is still palette snapping and was not
      renamed**; **`List` is still vertical**, so the row's *"and `List`"* half is
      **unchanged**; and **all three entries in the `Blocks` column remain and none
      is delivered.** **The row is not deleted and not marked closed**, because
      § *Corrections to the second gap table* is a record of what happens when a
      row is closed by a mechanism with no consumer — **and this row's mechanism
      has no consumer in this tree yet, so the honest form is a dated note.**
    - **Its four evidence citations are withdrawn and replaced by symbols**, the
      same correction task 45 makes to its own: `scroll.rs`'s four line numbers
      become `Scroll::scroll_offset`, `Scroll::on_event`, `max_scroll` and
      `visible_rect`, and `list.rs`'s two become `List::item_rect`'s doc and the
      module's *What is deliberately not here*. **A line number into a file being
      edited in parallel is stale within the hour**, which is the rule
      `AGENTS.md` states for itself.
    - **`doc/ui/IMPLEMENTATION_STATE.md` gains one dated entry** carrying: the
      axis decision and its six reasons in three sentences; **the nine changed and
      fourteen added public items, by name**; `tick`'s new signature and why;
      the momentum model and **the 28 px the naive form costs**; the two clocks
      and why; the snap model, the naming rule against `snap_to_state`, and the
      trailing spacer that is **not** built; `List`'s two doors and why they are
      `List`'s; **the two refusals that stay and the four tests that pin them**;
      **and the honest limit in that section's own register** — *the demo builds no
      `Scroll` and no `List`, so nothing here has been seen on screen; every claim
      is verified by test through the crate's own event path and its own clock.*
      `IMPLEMENTATION_STATE.md` is not a source of evidence
      (`.ai/workflows/task-sequence.md` § *State*); it points at the code.
    - **`doc/ui/IMPLEMENTATION_STATE.md`'s sequence table gains row 46**, in the
      shape row 40 already has: **`specified 2026-10-06, not started`**,
      `doc/ui/TASK_UI_PRIM_46.md`, and the review column naming that this is
      **two sub-tasks** per § *Scope*.

## Acceptance Criteria

- [ ] **`scroll_offset` is a vector and the axis is a type, and the API cost is
      the nine-and-fourteen this file names.** `awk '/^pub struct Scroll/,/^}/'
      ui/src/ui_core/src/widgets/scroll.rs` shows `pub scroll_offset:
      Property<Offset>`, and `pub enum Axis { Vertical, Horizontal, Both }` is in
      `scroll.rs` with `Vertical` as `#[default]`. The handoff **lists the nine
      changed and fourteen added public items by name** and confirms that count —
      **a different count is corrected in `IMPLEMENTATION_STATE.md` rather than
      argued about here.** `ui/src/ui_demo/src/main.rs` is **unchanged**, which is
      checkable by `git diff --stat` and is the whole of the compatibility argument
      (no caller outside `list.rs`).

- [ ] **`slider::Orientation` was not reused and the reason is on the record.**
      `grep -n 'Orientation' ui/src/ui_core/src/widgets/scroll.rs` returns
      **nothing** — only `Axis` appears in `scroll.rs` — and `Axis`'s doc carries
      requirement 2's third reason. `grep -n 'Both' ui/src/ui_core/src/widgets/slider.rs`
      returns **nothing**, so no `Both` was added to a slider's axis.

- [ ] **The two refusals stay, and their four named tests keep every assertion.**
      `git diff ui/src/ui_core/src/widgets/scroll.rs` shows
      `a_drag_that_is_entirely_horizontal_is_left_for_a_horizontal_scroller`,
      `a_horizontal_wheel_is_not_a_vertical_scroll`,
      `a_wheels_size_is_not_used_only_its_direction` and
      `a_wheel_notch_and_a_drag_of_the_same_size_move_a_scroll_alike` **with their
      names unchanged and no assertion removed or loosened** — each one grows a
      `.y` or an `Offset::new` and nothing else. **The handoff names all four and
      their new assertion counts.** And
      `the_scroll_consumes_only_a_drag_a_wheel_and_a_focused_key` passes: a table
      over **all nine** `InputEventKind` variants on an `Axis::Both` scroll,
      asserting only `Drag`, `Scroll` and a **focused** `KeyDown` are consumed,
      and that `Tap`, `LongPress`, `Swipe`, `Pinch`, `KeyUp` and `Text` are not.
      **The mechanism is named because it is what makes the claim legal:** this is
      what a later task that consumes `Swipe` — which row `L4`'s `Blocks` column
      names three consumers for — must break deliberately rather than drift past a
      claim in a document.

- [ ] **A decay is frame-rate independent, and the named test can fail.**
      `a_momentum_travels_the_same_distance_at_ten_milliseconds_a_frame_and_at_forty`
      asserts **64 × 10 ms against 16 × 40 ms agree within 0.01 px** and that both
      match the closed form within 1 px. **Mutation evidence in the handoff:**
      replace the exact integral with `travel = v * dt` in `Scroll::tick`'s
      momentum step, run
      `cargo test --all-features -p ui_core
      a_momentum_travels_the_same_distance`, and watch it fail with the two
      sequences differing by **about 28 px**; restore it and watch it pass. **Its
      tolerance is not chosen to pass** — the sequences stop before the cutoff so
      the assertion is about the decay's shape and not about where the cutoff bit,
      and the doc comment carries both facts. **This is the test a reviewer should
      break first**, because a frame-rate-dependent decay produces a plausible
      picture at one frame rate and a different plausible one at another.

- [ ] **The cutoff's own frame-rate dependence is bounded by a named test and a
      derived number.** `a_momentum_that_crosses_the_cutoff_stops_at_the_same_offset_on_both_frame_rates`
      asserts agreement within **1.1 px**, and its doc **states that the number is
      `MOMENTUM_STOP_SPEED / MOMENTUM_DECAY_PER_SECOND` = 1.0 px** plus a margin —
      so a reviewer can check the tolerance rather than trust it.

- [ ] **A snap test, and the mutation it kills is named.** `a_snap_settles_on_the_nearest_point_when_the_momentum_stops`
      settles on **100** from a landing near **70**, where a `floor`-based
      implementation picks **0**. **Mutation evidence in the handoff:** replace
      `nearest_point`'s search with a `floor`-to-the-lattice pick and watch that
      one test fail; restore it and watch it pass. `a_snap_lands_exactly_on_its_point_after_its_duration`
      asserts the `Easing::apply(1.0) == 1.0` end exactly, and
      `an_ease_out_snap_never_leaves_the_range_and_a_spring_one_does` makes
      requirement 12's overshoot limit a measurement **rather than a hope** — the
      second half of that test **must** find the offset out of range at some
      frame, and if it does not, the doc's claim is wrong.

- [ ] **Nothing is overloaded onto `snap_to_state`.** `git diff` shows
      `Scroll::snap_to_state` **with its body byte-identical** and its name
      unchanged; `grep -n 'pub fn snap_to_state\|pub fn snap_to_points\|pub fn
      set_snap_points' ui/src/ui_core/src/widgets/scroll.rs` shows **three**
      distinct names, and each of the two new ones carries its first-line doc from
      § *Two names, one widget*. **`snap_to_state` keeps all ten of its
      definitions and all eleven of `ui_demo`'s call sites, unchanged** —
      checkable by `git diff --stat` over `button.rs`, `chart.rs`, `gauge.rs`,
      `image.rs`, `keyboard.rs`, `progress.rs`, `slider.rs`, `text_input.rs`,
      `toggle.rs` and `ui/src/ui_demo/src/main.rs`, **all ten of which show no
      change at all**, and `rg -c 'snap_to_state\(\);' ui/src/ui_demo/src/main.rs`
      still returns **11**.

- [ ] **`List` gained three methods and broke none.** `git diff
      ui/src/ui_core/src/widgets/list.rs` shows **`pub fn tick`, `pub fn
      snap_to_items` and `pub fn set_snap_to_items` as the only new `pub fn`**,
      and **`List::offset`, `List::max_scroll_for`, `List::visible_range`,
      `List::item_rect`, `List::item_at`, `List::clip_rect`, `List::sync`'s
      signature and all four free functions with their signatures unchanged** —
      the diff shows `.y` and `Offset::ZERO` inside them and no signature line
      touched.
      `turning_item_snapping_off_clears_the_points_rather_than_leaving_them`
      passes, and `a_list_is_vertical_and_still_refuses_a_horizontal_wheel` passes,
      **so a later grid task has to break the second deliberately.**

- [ ] **Everything is driven without a display and without a pointer.**
      `grep -n 'std::time::Instant\|SystemTime\|thread::sleep' ui/src/ui_core/src/widgets/scroll.rs
      ui/src/ui_core/src/widgets/list.rs` returns **nothing** — no wall clock and
      no sleep, which is the `AGENTS.md` prohibition and the reason every test
      above is driven by an injected `InputEvent` and by `Duration`s the test
      chose. **No acceptance criterion in this file is met by a pointer-driven
      capture, and none asks for one**, per `.ai/tools/README.md` § *Capturing a window*, which records that
      XTEST pointer injection has never delivered an event to this window. **The
      handoff states this in those words and does not claim otherwise.**

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: in `scroll.rs` — the thirty-two named in
      requirement 23 (eight axis, ten momentum, nine snap, three keys, the event
      table, the constants); in `list.rs` — the five named; the four new doctests
      (`impl Interpolate for Offset`, `Axis`, `Scroll::tick`,
      `Scroll::snap_to_points`). **Baseline 1894 (1450 + 224 + 220) and this file
      projects 1935** — thirty-seven new unit tests and four new doctests —
      **so a measured number other than 1935 is corrected here rather than argued
      about.** **No test was deleted, renamed away or weakened**, and the six
      preserved `scroll.rs` tests and the five **edited** (not added) doctests of
      `max_scroll`, `clamp_scroll`, `visible_rect`, `gesture_delta` and
      `wheel_delta` are named in the handoff as preserved or edited rather than as
      changed. `cargo fmt --check`,
      `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is not installed on this host; that
      is **recorded, not passed**.

- [ ] **The six gallery pages are pixel-identical, and the mechanism is one
      checkable fact.** `Page::ALL`'s six names, release build, captured **before
      and after** with the commands of `.ai/tools/README.md` § *Capturing a window* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree`, then
      `pgrep -a -x ui_demo` in the same call as each `magick import -window <id>`,
      then `magick compare -metric AE before.png after.png null:` per page.

      **`AE 0` over the `1280x680` crop on all six**, with every differing pixel
      inside the fps readout's band — which is the criterion task 34 established
      and `.ai/tools/README.md` § *Capturing a window* records as the one thing two captures of an unchanged frame differ in
      (**AE 0 over `y 80–680`**).

      **The mechanism, stated as a grep rather than a hope:**

      ```sh
      grep -n 'widgets::scroll\|widgets::list\|Scroll::new\|List::new' \
        ui/src/ui_demo/src/main.rs
      ```

      **returns nothing, before and after** — the demo builds no `Scroll` and no
      `List`, so **no code path on any page reaches the widget this task changed**
      and no rect the gallery placed can move. The three supporting facts: **no
      `ui/src/ui_demo/src/main.rs` line changes at all** (`git diff --stat`);
      `every_page_places_every_rect_where_the_gallery_placed_it`,
      `no_two_placed_rects_overlap` and `assert_placed_handles_is_complete`
      **keep their names and every one of their assertions**; and **no
      `DrawCommand` variant is added**, so the pipeline's recorded stream is
      unchanged by construction.

- [ ] **The frame rate is measured on all six pages and reported, and the
      expectation is that nothing moved — with the reason stated if it did.**
      `.ai/tools/fps-check.sh 10 55` on the default page with the script's own line
      pasted rather than the rate expected, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
      the six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `.ai/tools/README.md` § *Frame-rate baseline* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. Every page above the floor of **55**, and **each page is compared
      against its own pre-change number**, which is the only comparison that
      isolates this task's cost.

      **Why no movement is expected, and it is arithmetic rather than a hope:** the
      six pages construct no `Scroll` and no `List`, so no frame executes one
      extra `tick` step or one extra clock; and the second `AnimationClock` ticks
      an empty `Vec` on every `Scroll::tick` call — **and there are no such calls**,
      because nothing in the demo holds a `Scroll`. **If a page does move, that is
      a finding and the handoff reports the number with the run's page named**,
      rather than explaining it away.

- [ ] **Row `L5` is amended, dated, and its `Blocks` column is intact.**
      `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* row
      **`L5`** carries a dated note naming `TASK_UI_PRIM_46`, stating the vector
      offset and `scroll::Axis`, the momentum and its three constants, the snap
      points, **that `snap_to_state` is still palette snapping and was not
      renamed**, and **that `List` is still vertical so the row's own claim about
      it is unchanged**. **The row is not deleted and not marked closed**, its
      `Blocks` column still names **all three** — the card carousel, dock overflow,
      the widened wiper segmented control — and the note says in those words that
      **none of them is delivered by this task**, because
      `DEMO_APPLICATION.md` § *Corrections to the second gap table* is a record of
      what happens when a row is closed by a mechanism with no consumer, **and this
      row's mechanism has no consumer in this tree yet.**
      **Its four line-number citations are withdrawn and replaced by symbols**, and
      `grep -n 'scroll.rs:378' doc/ui/DEMO_APPLICATION.md` returns **nothing**.

- [ ] **The scope held, and the tree is named.** `git diff --stat` shows exactly
      the five files: `ui/src/ui_core/src/widgets/scroll.rs`,
      `ui/src/ui_core/src/animation.rs`, `ui/src/ui_core/src/widgets/list.rs`,
      `doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md`. **No
      `render.rs`, `render/target.rs`, `render/context.rs`, `render/mesh.rs`,
      `paint.rs`, `input.rs`, `node.rs`, `property.rs` or any other
      `widgets/*.rs` is touched**, which is checkable rather than asserted — **and
      in particular `LayoutState::clip`, `LayoutState::hits` and `Batch::clip` are
      task 45's and task 42's and are untouched**, whatever the state of those tasks
      in the tree when this starts. **`ui/Cargo.toml` and `ui/Cargo.lock` are
      unchanged** — the approved direct dependencies remain `sdl3 0.20`,
      `glow 0.18` and `freetype-rs 0.38`, and **per `AGENTS.md` an easing crate or
      a physics crate for one exponential is a licence decision against GPLv3 that
      nobody has asked for**; `f32::exp` is std since 1.7 and the project's
      `rust-version` is 1.85. **`grep -c unsafe` over the diff is `0`** — this task
      adds no `unsafe` block, no `unwrap`, no `expect` and no `panic!`; the only
      arithmetic added is `exp`, four multiplications and two comparisons per
      frame per axis, all on `f32`s a caller supplied.

- [ ] **Both sub-tasks were dispatched separately, sequentially, and each handoff
      names the tree its suite ran on.** The handoff records that 46.1 was
      implemented and verified **before** 46.2 was briefed, that 46.2's brief
      named `Axis`, `Scroll::set_snap_points` and `Scroll::tick`'s signature as
      existing, and that **because this tree is shared, both suites were reported
      against a `/tmp` copy with the other agent's file and its `mod` line removed**.
      **A single green number with no tree named does not meet this
      criterion.**

## Out of Scope

- **No rubber-banding, no elastic overscroll, and no bounce.** The clamp is the
  whole of the out-of-bounds behaviour: the frame a component reaches a bound it
  is pinned there, its velocity is zeroed, and the momentum is over. A fling that
  overshoots and returns is the thing the clause names, and § *Momentum* gives the
  reason `Easing::Spring` is refused for the momentum while it is permitted for a
  snap.
- **No pull-to-refresh, no sticky headers, and no overscroll affordance of any
  kind.** `scroll.rs`'s module doc keeps them in *What is deliberately not here*,
  and this task leaves them there — a pull-to-refresh needs a threshold and a
  content refresh, both of which have no source in this repository.
- **No nested scrolling and no scroll chaining.** A drag that reaches a bound stops
  and is **consumed**, exactly as it is today; nothing hands it to a parent. The
  chained case needs to know who the parent scroll is, and `Scroll` does not.
- **No grid, no `LayoutMode::Grid`, and no `Flex.wrap`.** Row **`L3`** is
  `LayoutMode::Grid`, its evidence is the `LayoutMode::Grid { .. } => Vec::new()`
  arm, and `TASK_UI_PRIM_52` owns it. **This is also why `List` does not gain a
  horizontal axis** — see § *`List` stays vertical*.
- **No card carousel, no dock overflow, and no widened wiper segmented control.**
  All three are row `L5`'s `Blocks` column and all three are `TASK_UI_DEMO_n`
  items. **The card carousel additionally owns the paged trailing spacer this
  task does not build** (§ *Paging*), because growing the scrollable range past
  the content is `max_scroll`'s contract and not this task's to change.
- **No snap to a boundary outside the content's own extent, and no trailing
  spacer.** § *Paging* states the consequence: a paged caller whose content is not
  a whole number of pages cannot snap to a full last page, and that is the
  carousel's problem rather than this task's.
- **No snap-point callback, and no `Callback`-based snap.** `Callback<T>` is
  `Rc<dyn Fn(T)>` and returns `()`; a snap needs the widget to ask a question.
  `scroll.rs`'s module doc records this rather than leaving it as a reader's
  puzzle.
- **No caller-facing momentum tuning.** `MOMENTUM_DECAY_PER_SECOND`,
  `MOMENTUM_STOP_SPEED` and `MOMENTUM_MAX_SPEED` are **private**, like
  `WHEEL_STEP`, `KEY_STEP_FRACTION` and `SCROLLBAR_THICKNESS` beside them, and
  there is **no `set_momentum`**. A widget whose limits are arguments is a widget
  whose limits two callers disagree about, and this project has one scroll
  surface. **What would reverse this** is a measured complaint about the feel, at
  which point the constants become `pub` and a door appears — task 40's own rule.
- **No vertical `Scroll` behaviour change.** `Axis::Vertical` is the default and
  every current caller gets it, so the drag's sign, the wheel's sign, the notch,
  the key step, the clamp, the thumb, the groove, the focus ring and the palette
  are **the code they are today**. The offset gains `.x` and reads as `Offset`, and
  **nothing else about a vertical scroll moves.**
- **No `List` API break.** `List::offset` still returns the `y` offset as an
  `f32`, `visible_range` still takes one, and `list.rs`'s four free functions keep
  their signatures. § *`List` interaction* names the three methods that are added
  and nothing else.
- **No `Axis::Both` snap on two different lattices.** One point set, applied to
  whichever axis the fling ran on; a viewport that needs different points on each
  is two scrolls. Stated in `nearest_point`'s doc rather than left to be found.
- **No change to `Scroll::snap_to_state`, `Palette`, `Style`, `animate_to_state`,
      `style`, `grab_thumb`, `release_thumb`, `is_thumb_grabbed`,
      `clip_rect`, `command_bounds` or `clip_commands`' behaviour.**
  `command_bounds`' seven-arm match and `clip_commands`' straddler rule are
  requirement 11 of task 45's scope, not this one; this task rewrites the two
  **docs** in that module and touches **no arm and no assertion** of either.
- **No change to `LayoutState::clip`, `LayoutState::hits`, `LayoutState::visible`,
      `Batch::clip`, `BatchKey`, `Renderer::draw_node_clipped` or any GL call.**
  Tasks 45 and 42 own all six, and `git diff --stat` is the criterion.
- **No demo change of any kind**, and therefore no new page, no `--tab=` name, no
  `--help` change and no `Page::ALL` edit. **This is the task's central scope
  decision and it is why the pixel criterion is AE 0 rather than a restatement**
  — see § *Goal*.
- **No new dependency and no `unsafe`.** Per `AGENTS.md` the approved direct
  dependencies are `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; an easing
  crate, a physics crate or a gesture crate for one exponential, four
  multiplications and a linear search over a `Vec<f32>` is a licence decision
  against GPLv3 that nobody has asked for, and `Easing`, `AnimationClock` and
  `Property` already exist in the crate for the snap half.