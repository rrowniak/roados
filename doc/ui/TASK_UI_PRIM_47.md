# TASK_UI_PRIM_47: A Mode That Crosses a Widget Boundary — `mode::ModeScope`, `snapshot::Snapshot`, and `Segmented`

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Give `ui_core` the two of the three things row **`L6b`** of `DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* says are genuinely absent — **a mode
that reaches widgets other than the one it was set on**, and **a stored value
that survives an enumeration change and is presented through the new one** — and
**hand the third, a mode dimension in the theme, to row `L9` by name**, because
that half is the token half of `L9` and nothing in this task would consume it.

Three deliverables, none of which depends on another:

- **`ui/src/ui_core/src/mode.rs`** — **`ModeScope<K>`**: a mode resolved per node,
  where the nearest ancestor that carries one decides and every widget below it
  reads the same answer.
- **`ui/src/ui_core/src/snapshot.rs`** — **`Snapshot`**: a *set* of deferred writes,
  captured together and put back together, built on the crate's own
  `widgets::Callback<()>` and two one-line `save()` methods.
- **`ui/src/ui_core/src/widgets/segmented.rs`** — **`Segmented`**: a control whose
  value is one of a named set of points on a scale, and **whose set may change
  while the value survives it**.

And one demonstration, on the `controls` page: a climate strip in which **`Auto`
replaces the fan's `1 … 7` with `Low / Medium / High` for the same stored value**,
and a three-touch defroster whose **third touch restores the air distribution, the
heating setpoint and the fan speed it captured on the first**.

## Context

### What row `L6b` is, and what row `L6a` settled on 2026-10-05

Row **`L6a`** was split out of the original row `L6` on 2026-10-05 and is recorded
as **satisfied, per widget**. Its own evidence names the mechanism: `GaugeType` in
`ui/src/ui_core/src/widgets/gauge.rs` is documented in the crate's own words as
*"a mode and not an appearance"* and deliberately sits **behind
`Gauge::set_gauge_type` rather than a `Property`**, because *"nothing animates
which shape a gauge is in, so a property here would be one whose only writes are a
caller's and which could change on its own in the middle of a frame, leaving the
mode disagreeing with the properties actually drawn."* `Severity` in
`ui/src/ui_core/src/widgets/toast.rs` is a second such input and it changes
**layout as well as colour** — a toast with no severity draws no disc and *"its
message starts at the panel's padding rather than beside one."* **Twelve
`pub enum`s exist across `ui/src/ui_core/src/widgets/`** and this task **adds
none**, so `L6a`'s count is unchanged by it and this file does not amend that row.

**`L6a` is settled, and re-opening it is the failure this file is written
against.** Every mechanism below inherits `GaugeType`'s reasoning rather than
inventing a second position on it: a mode is a **plain value written by a setter**,
never a `Property`, so it cannot move in the middle of a frame.

Composite row **4** in `DEMO_APPLICATION.md` § *Composite widgets — the part worth
rebuilding* is the demand this row serves: *"The state that decides a control's
**presentation** must be separate from the control's **value**"* — and its `What
it demands` cell points at `L6a` for the per-widget half and at **`L6b` for the
cross-widget half**. Its own count is **nine documented instances where one control
changes meaning**.

### What `L6b`'s three claims are, and the state of each

Verbatim from row `L6b`, clause by clause:

| # | Clause of row `L6b` | This task |
|---|---|---|
| **(a)** | *"**no mode dimension in the theme** — `ThemeToken` is a flat 33-variant enum with no `Focus`, `Hover`, `Pressed`, `Shadow`, `ZOrder`, `Active` or `Selected` variant, so a mode cannot restyle a subtree"* | **deferred to row `L9`, by name** — § *The theme gains no token, and `L9` is why* below |
| **(b)** | *"**no cross-widget propagation** — all twelve enums are a field on one widget read at paint time from its own state, and the demo needs one mode to reach widgets it is not on"* | **closed**: `ModeScope<K>` |
| **(c)** | *"**no mode-dependent enumeration of a stored value** … `Slider` does let the domain change at runtime via `set_range`/`set_step`, but it is one linear value → one position with **no label or option set**, so a stored value cannot *survive* an enumeration change and be presented through the new one"* | **closed**: `Segmented` |

### What exists at `HEAD` (`75a896c` plus the uncommitted diff), established and not re-derived

- **`ThemeToken` has no mode token and `TOKEN_COUNT` is `const TOKEN_COUNT: usize
  = 33;`, private**, over a `static ALL_TOKENS: [ThemeToken; TOKEN_COUNT]`. The
  test **`dark_and_light_define_every_token`** in `theme.rs`'s own
  `#[cfg(test)] mod tests` asserts `ThemeToken::all().len() == 33` and that every
  colour token differs between the two themes — so **adding a token fails an
  existing test**, and that test is the mechanism this file relies on.
- **`Theme` is `Theme { tokens: HashMap<ThemeToken, Property<PropertyValue>>, clock:
  RefCell<AnimationClock> }`** and its public surface is exactly `new`, `dark`,
  `light`, `get`, `set`, `property`, `switch_to`, `tick`. `Theme::switch_to`
  animates **every** member of `ThemeToken::all()`. `grep` over
  `ui/src/ui_core/src/` finds **zero** occurrences of `scope`, `inherit`,
  `parent_theme`, `push_theme` or `pop_theme` as an identifier.
- **Focus rings and hover/pressed are per-widget properties**:
  `Button::focus_ring`, `Button::hovered`, `Button::pressed`, `Button::focused`,
  `Button::activatable` in `ui/src/ui_core/src/widgets/button.rs`, and `Button`
  carries **no `selected`** (row `7` of `DEMO_APPLICATION.md` § *Library gaps*
  checked `button.rs` on 2026-10-05 and says so).
- **`Property<T>` has `new`, `bind(F: Fn() -> T)`, `get`, `set`, `on_change(F:
  Fn(&T))`, `handle`, `is_bound`, `animate_to`, `animate_from_to` — and nothing
  else.** **`Property::bind`'s closure receives no arena and no node handle**, so a
  property **cannot write another widget's state**; this is the same constraint
  that makes `TASK_UI_PRIM_42` reject `Property<Screens>`.
- **`Callback<T>` is `widgets::Callback<T>`, a newtype over `Rc<dyn Fn(T)>` with
  `new` (payload-free), `from_fn`, `none`, `is_set`, `call`, `Default` and `Clone`,
  in `ui/src/ui_core/src/widgets/mod.rs`.** It is `Fn`, not `FnMut`.
- **`node::WidgetNode` has `children()`, `parent()`, `layout()`, `layout_mut()`,
  `paint()`, `paint_mut()` — and no arena.** `node.rs`'s module doc says why in its
  own words: *"A node cannot reach the arena that holds it."*
- **`Layout::visit` is a private method of `Layout`** in
  `ui/src/ui_core/src/layout.rs`. `input::Focus::collect_focusable` is a private
  recursive walk in `ui/src/ui_core/src/input.rs`; `Focus::current` is public.
- **`arrange_stack` never reads `position` and `arrange_absolute` does** — the
  recorded defect of a position API that only one parent
  mode reads.
- **`Slider::set_range` and `Slider::set_step` exist and re-project the value**, and
  `set_step`'s doc carries the sharp edge this task is built beside, verbatim:
  *"A slider from 0 to 1 with a step of 0.3 snaps to 0, 0.3, 0.6 and 0.9 and cannot
  reach 1: the snap is to the nearest step, and the nearest step to 1* is *0.9."*
  `Slider`'s only enum is `Orientation`.
- **`Segmented`'s nearest-projection rule is the crate's own rule**, borrowed from
  `Slider::set_step`: *the nearest one wins*.
- **`Container::new(nodes, LayoutMode)`, `Container::add_child`, `Button::new`,
  `Painter::{rounded_rect, text, text_bold}`, `label::layout_text` with
  `LayoutOptions { max_width, align: TextAlign::Center, wrap: WrapMode::None,
  truncation: Truncation::Ellipsis }`** all exist and are public.
- **`widgets/mod.rs` lists fifteen `pub mod` entries**, and `lib.rs` lists twelve
  modules.

### Tasks 41 and 44 are documents, not code — checked, and this task depends on neither

`ui/src/ui_core/src/widgets/mod.rs` has **no `pub mod icon`** and
`ui/src/ui_core/src/render/` has **no backdrop module**: neither `TASK_UI_PRIM_41`
nor `TASK_UI_PRIM_44` has landed. Two consequences, both recorded rather than
assumed:

1. **No dependency, and no ordering.** This task may be built before, between or
   after either. If either lands first, the two widget-module counts in this
   file's acceptance criteria move by one and the criteria are written to survive
   that; `ui/Cargo.toml` and `ui/Cargo.lock` stay untouched by all three.
2. **Consistency, not dependence.** Both tasks' decision shape is the one this file
   uses: a **narrowest library deliverable** plus the one seam that delivers it
   (41's "a public entry point", 44's "the tint rides on the command"). `Segmented`
   is the same shape — a widget plus `Palette::from_theme`, which is
   `Slider`'s and `Toggle`'s and `Gauge`'s established answer to *"a colour the
   theme has no token for"*.

### The decision: a mode map with inheritance, resolved by the owner

**Decision: (B), narrowed — `ModeScope<K>`, a map from node handle to mode whose
lookup walks the parent chain for the nearest entry, resolved by the owner before
layout and written into each widget through a setter.**

This is **(C)'s semantics without (C)'s traversal**, and it is (A) with the trait
object deleted. Four reasons, and the first settles it.

1. **The crate has no paint traversal for a scope stack to push and pop on.**
   `Layout::visit` is private to `layout.rs`, and the paint pass in this repository
   is `Demo::frame`'s own flat `for handle in self.order.iter()` with one `match`
   arm per widget — a list, not a walk. So (C) is not *"add a stack"*; it is
   ***build a paint traversal in `ui_core` and reconcile it with the demo's flat
   loop*** — new machinery in the one stage of the frame whose ordering is a
   recorded contract (task 37's *"record the map before the mesh and the chrome
   after it"*), and a flat list cannot nest without the owner computing push/pop
   boundaries itself, which is the bug of a container that covers
   the window and swallows every tap aimed at anything behind it.
2. **Inheritance is a walk up `parent()`, and the crate already walks it.**
   `node.rs`'s private `descends_from` is exactly the loop `resolve` needs:
   `nodes.get(handle).and_then(WidgetNode::parent)` until the arena answers `None`.
   One ancestor per step, no stack to maintain, no state to leak.
3. **(A) fails `developer.md` § *Code quality* on two counts.** *"Avoid
   `Box<dyn Trait>` unless dynamic dispatch is genuinely needed"* — a `Mode` trait
   answered by methods means either a method per question (which is `L6a`'s
   problem one level up) or a downcast, and neither is free. And
   `no abstraction before the second use`: this crate would have **one** caller,
   `ui_demo`, and zero in-crate callers.
4. **(A) is also the wrong *shape* for the reason `GaugeType` gives.** An ambient
   mode context is *readable at any point during a frame*, which is precisely the
   hazard the crate already documented and solved by making the mode a setter. A
   field the owner writes once, before layout, cannot be half-changed by a widget
   that is painting.

**And the mode reaches the widget as a setter write, not as a `Property`**, which
is the one thing that is not a choice: `WidgetNode` has no arena, so a widget
**cannot resolve its own mode at paint time** — `Property::bind` gets no handle
either. The owner therefore resolves and hands it over, exactly as the demo
already hands `Slider::focused` and `Button::pressed` over.

**Rejected: (D), three tasks.** Right about one thing, wrong about another. It is
right that this row's own note — *"the state that decides a control's presentation
must be separate from the control's value"* — is the deepest structural statement
in the direction document, and right that it deserves doing properly. It is wrong
as a *split of this row*, for two reasons that are both facts about this row:
**claim (a) is not this row's** (it is `L9`'s, below), and **the three claims are
not independent** — the demo's `Auto` mode is a `ModeScope` entry *and* a
`Segmented` choice-set change *and* a value that must survive, and splitting them
across three tasks makes the demonstration land three times instead of once, with
three reviews of a third each. The split that buys what (D) wants is **(b) and (c)
inside this task and (a) into `L9`**, which is what this file does.

### The theme gains no token, and row `L9` is why

**Claim (a) is deferred, and the reason is that it is row `L9`.** Row `L9` in
`DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`* reads *"**No theme
scoping or inheritance.** One flat global token map; **no tokens for focus, hover,
pressed, shadow or z-order**"* — **five of the seven tokens `L6b`(a) names**, in a
row that also asks for the thing that makes a token dimension usable, which is
*scoping*. Two consequences, both mechanical:

- **Adding a token here would change `ThemeToken::all()`**, and therefore both theme
  tables and `Theme::switch_to`'s animation over `ThemeToken::all()` — for a value a
  theme switch does not change, which is the argument
  `Severity`'s own doc makes about `Info`: *"Adding a token for it would change
  `ThemeToken::all()`, both theme tables and the transition every token takes part
  in during a switch, for a value a switch does not change."*
- **A mode token with no scoping is a token nothing reads.** The mechanism that
  makes it readable is inheritance, which is exactly what `L9` asks for and what
  this task does **not** build: `ModeScope<K>` is generic over `K`, so the crate
  has **no idea what a mode is**, and a `ThemeToken::Focus` would give it one.

So: **`theme.rs` is not touched by this task at all**, `const TOKEN_COUNT` stays
**33**, stays **`const`**, stays **private**, and `dark_and_light_define_every_token`
keeps its name and its `assert_eq!(ThemeToken::all().len(), 33)` unamended.
**Making `TOKEN_COUNT` public is a public-API decision this task does not take**:
`ThemeToken::all()` already answers the question it answers, and a second public
count is a second thing to disagree with it.

### `Segmented`: a value on a scale, and a set of named points on it

**Decision: a new widget, not a `Slider` extension and not a wrapper.** The reason
is `Slider`'s own documented sharp edge, quoted above. *"A slider from 0 to 1 with
a step of 0.3 … cannot reach 1"* is not a bug to be careful about — it is the
**statement that a value on a linear grid and a value in an option set are
different types**, because the grid's ends are unreachable whenever the step does
not divide the range. An enumerated control has no such failure only if its value
is **a point on a scale that the option set names**, not an index and not a
snapped float.

- **The value is `f32` and each `Choice` carries the `f32` it stands for** — not an
  index. An index would renumber on every change (`7 → 2`), so "the stored value
  survived the change" would be unstatable; a scale makes the round trip
  **Auto → Manual the identity** when Auto's levels are chosen on Manual's scale,
  which is what the demo's three choice sets do (requirement 10).
- **A `Slider` extension was rejected** because it would give one widget two hit
  tests (`Slider::value_at` is linear; an enumerated one is a nearest-column test),
  two value semantics, and two keyboard stories (`Slider::adjustment(&key,
  orientation)` adds a step that a choice set does not have) — one widget, two
  presentations, for one caller.
- **A wrapper was rejected** because a wrapper cannot present the labels:
  `Slider`'s `Palette` has no text colour and `Slider::paint` records no text
  command, so "Low / Medium / High" would need a `Label` per column beside it,
  which is a composite widget and `DEMO_APPLICATION.md` § *Composite widgets*
  reserves for the demo.
- **Equal columns, not measured ones.** `Segmented::size` is
  `n * SEGMENT_WIDTH × SEGMENT_HEIGHT` with no measurement argument, and `paint`
  divides whatever rect it is given into `n` equal columns. A label wider than its
  column is **truncated with `Truncation::Ellipsis` inside the column**, through
  `label::layout_text` — the crate's own machinery, the same one `Label` uses, and
  the only thing standing between this widget and row **`L8`**'s *"no offscreen
  text measurement on the draw command"*.
- **`Segmented` adds no `pub enum`**, so `L6a`'s count of twelve stands.

### Restore semantics: a snapshot is a set, not a property

The manual's third defroster touch — *"restore the air distribution, heating, and
fan to their previous settings"* [A], quoted in `DEMO_APPLICATION.md` § *Climate —
three surfaces* and in composite row **11** — is a **save/restore of three
heterogeneous values owned by three different widgets**. Three consequences, and
each one is a decision rather than a detail:

1. **A `Property` cannot do it.** `Property::on_change` takes `Fn(&T)`, not
   `FnMut`, and `Property::bind`'s closure gets no arena — so there is no
   reactive path from one widget's value to another's. A write from outside is a
   **plain `set`**, which is what a captured closure does.
2. **The set must be heterogeneous**, so it is type-erased by function, not by
   name. **`String` keys are refused** (`developer.md` § *API design*: *"Newtype
   pattern over stringly-typed parameters"*), and a `Saveable` trait with one
   method per widget is refused too: the crate already has the erased closure type
   — **`widgets::Callback<()>`** — and a `Snapshot` of `Vec<Callback<()>>` is it in
   a vector. **No new callback type.**
3. **Restore runs in reverse capture order, and that is load-bearing.** The manual
   says adjusting the fan *"may change the selected setting for how air is drawn"*
   [A] — so two captured values can feed each other, and a set restored in capture
   order would let the first write be undone by the second. Reverse order makes the
   **first captured** the **last written**, which is the order a stack of overrides
   needs.

`Slider::save` and `Segmented::save` are each **one closure that reads the value
now and holds the property**, and the whole of each is four lines. They capture the
**value**, not the property's later contents — which is the mutation that kills
`restore_puts_every_captured_value_back`.

### Where the mode is resolved, and the ordering trap this design avoids

`Demo::frame` is: clocks and ticks → **`sync_slider_state` / `sync_toggle_state`** →
layout → `tick_fps` → `sync_dialog_focus` → the paint walk. **`sync_climate_mode`
goes beside `sync_toggle_state`**, and its guard is `sync_toggle_state`'s own: an
`if wanted != applied` comparison, so the demo **aims once and applies on a change
only**. Two reasons in the comment, both the crate's: the crate's idiom is *"aim
once, tick per frame"* and a per-frame aim creeps toward its target and never
arrives (`sync_toggle_state`'s recorded argument), and a mode that only changes a
label and a choice set is **not** something to animate.

**And no `mark_dirty` is required anywhere, which is a decision and not an
oversight.** Every widget this task places has a fixed rect and a mode that changes
only what is *inside* that rect, so no layout input moves. *A
cache invalidated in the wrong order is a cache that lies* is the trap this walks
around rather than into: there is no cache to invalidate because no cached rect
depends on the mode. `a_mode_change_moves_no_rect_in_the_tree` is the test that
holds that down.

### Scope, measured against `developer.md` § *Scope check*

**Eleven files, four components — over both thresholds, so the split is not a
judgement call.** `developer.md` § *Scope check* counts the files this change
creates or modifies and the independent components in it, and splits anything
**above five files or three components** per `.ai/protocols/subagents.md`
§ *Implementation fan-out*.

| | Count | What they are |
|---|---|---|
| **Files** | **eleven** | `ui/src/ui_core/src/mode.rs`, `ui/src/ui_core/src/snapshot.rs`, `ui/src/ui_core/src/lib.rs`, `ui/src/ui_core/src/property.rs`, `ui/src/ui_core/src/widgets/segmented.rs`, `ui/src/ui_core/src/widgets/mod.rs`, `ui/src/ui_core/src/widgets/slider.rs`, `ui/src/ui_demo/src/main.rs`, `doc/ui/DEMO_APPLICATION.md`, `doc/ui/PRIMITIVES_ARCHITECTURE.md`, `doc/ui/IMPLEMENTATION_STATE.md` — plus this file |
| **Components** | **four** | the two owner-side types and the module wiring; the enumerated widget and `Slider::save`; the demo's climate strip; the documentation |

Three sub-tasks, with **disjoint file sets**, which is `.ai/protocols/subagents.md`'s
first splitting property — and each with an acceptance test that passes without the
other two, which is its second:

| Sub-task | Files it owns | Its own acceptance test, passable alone | Runs |
|---|---|---|---|
| **A — the owner-side types** | `mode.rs`, `snapshot.rs`, `lib.rs` (two `pub mod` lines), `property.rs` (one sentence) | `cargo test -p ui_core` green with `two_siblings_under_one_owner_resolve_the_same_mode` and `a_snapshot_of_three_controls_is_restored_as_a_set` among the sixteen `mode.rs` / `snapshot.rs` tests | **in parallel with B** |
| **B — the enumerated widget** | `widgets/segmented.rs`, `widgets/mod.rs` (one `pub mod` line), `widgets/slider.rs` (one added method) | `cargo test -p ui_core` green with the fourteen `segmented.rs` tests and `a_saved_slider_value_is_put_back_where_it_was` | **in parallel with A** |
| **C — the demo's strip** | `ui/src/ui_demo/src/main.rs` | `cargo test` green with the ten named demo tests | **after A and B** |
| **the documents** | `DEMO_APPLICATION.md`, `PRIMITIVES_ARCHITECTURE.md`, `IMPLEMENTATION_STATE.md` | `grep` finds the `L6b` note dated and attributed, and § *Inheritance* no longer claims the five properties inherit | **the developer's own** |

- **A and B do not depend on each other, and that is checked rather than assumed.**
  `Snapshot`'s only crate dependency is `widgets::Callback<()>`, which is at `HEAD`,
  and `Segmented::save` returns that same type — **so neither sub-agent is briefed
  against code that does not exist yet**, which is `.ai/protocols/subagents.md`
  § *Parallel or sequential*'s prohibition.
- **C is sequential because it names all three types**, and it is the integration:
  the developer briefs it against the code as it stands **after A and B have
  landed**, not against this document's signatures.
- **Each sub-task gets the full `subagents.md` § *Briefing contract*** — the
  sub-task, the files it owns, its acceptance test, the constraints, **what it must
  not touch**, the return format, and read-write access — and returns that format
  rather than prose. **The developer orchestrates, integrates, runs the full
  verification suite once over the integrated result, and hands off.**
  `.ai/workflows/task-sequence.md` § Gates then reviews the **integrated** whole,
  because a sub-task reviewed in isolation is reviewed again as part of the whole.
- **If the implementer is *not* fanning out** — one agent, three sub-tasks in
  sequence — **the same three file sets apply and the same stop condition does: a
  fourth code file outside those three sets is a stop condition, not an expansion**
  (`developer.md` § *Stop conditions*). A change that cannot be split along file
  boundaries is not a splitting problem; this one splits cleanly, so it is not one.
- **The counts above are this file's estimate when it was written, and the handoff
  reports the counts it found.** Three task files are open at once in this tree, so
  *"eleven files"* is this task's own and not the repository's.

## Requirements

1. **A new module `ui/src/ui_core/src/mode.rs`, declared `pub mod mode;` in
   `ui/src/ui_core/src/lib.rs`** in alphabetical position (between `layout` and
   `node`), holding exactly one public type:

   ```rust
   /// A mode resolved per node: the nearest ancestor that carries one decides.
   pub struct ModeScope<K> {
       entries: Vec<(Handle, K)>,
   }

   impl<K> ModeScope<K> {
       #[must_use] pub fn new() -> Self;
       #[must_use] pub fn len(&self) -> usize;
       #[must_use] pub fn is_empty(&self) -> bool;
       pub fn clear(&mut self, handle: Handle) -> bool;
       pub fn retain(&mut self, nodes: &Arena<WidgetNode>) -> usize;
       #[must_use] pub fn resolve<'a>(&'a self, nodes: &'a Arena<WidgetNode>, handle: Handle) -> Option<&'a K>;
   }

   impl<K: Clone + PartialEq> ModeScope<K> {
       pub fn set(&mut self, handle: Handle, mode: K) -> bool;
       #[must_use] pub fn effective(&self, nodes: &Arena<WidgetNode>, handle: Handle, fallback: K) -> K;
   }

   impl<K> Default for ModeScope<K> { /* `new` */ }
   ```

   - **`entries` is a `Vec`, not a `HashMap`.** A scope holds a handful of entries,
     `set` and `clear` are linear over them, and `Vec` gives a **stable iteration
     order** where a hash map would not — the same reason `ALL_TOKENS` is a
     `static` array and `ThemeToken::all` is written as *"a fixed order rather than
     a hash iteration so a transition's frames are reproducible"*.
   - **`set` and `clear` return `bool`, and it means one thing: *did the scope's
     contents change*.** `set` returns `false` for a value equal to the one it
     already holds; `clear` returns `false` when there was no entry. Both are
     `#[must_use]`, because a caller that ignores the answer cannot tell a no-op
     write from a real one, and `sync_climate_mode`'s guard depends on it.
   - **`set` takes no arena.** A stale entry cannot be resolved — no node's
     ancestor chain passes through a handle the arena cannot answer — so refusing
     one is a convenience, not a correctness rule, and `node::attach`'s refusal
     precedent does not transfer to a map that is consulted, not traversed.
   - **`retain` exists, and it is this task's lesson from task 24.1.** It drops
     every entry whose handle the arena no longer holds and returns how many it
     dropped. *A sweep of a mechanism's call sites is not a sweep
     of the data it is built from* is the `page_members` leak, where *"the table
     was pruned nowhere"* and six rows named dead handles; a scope whose owner
     removes a node needs the same one call, and **`retain` is deliberately not
     `#[must_use]`** — its count is information for a test and a log, not a
     contract.
   - **`resolve` is `O(depth)`** — one `nodes.get` per ancestor — and the doc
     comment carries that arithmetic and the comparison with `node.rs`'s private
     `descends_from`, which is the same loop.
   - **The module doc records**: what a scope is; why it is a map and not a stack
     (`Layout::visit` is private, the paint pass is a flat list, and a flat list
     cannot nest); why there is no `Mode` trait and no crate-side mode vocabulary;
     why the mode is written into a widget by the owner rather than resolved by the
     widget (`WidgetNode` has no arena, and `Property::bind` gets no handle); and
     the L9a deferral by name.

2. **A new module `ui/src/ui_core/src/snapshot.rs`, declared `pub mod snapshot;` in
   `lib.rs`** in alphabetical position — between `render` and `texture`, which is
   the only place it can go — holding exactly one public type:

   ```rust
   /// A set of deferred writes, captured together and put back together.
   #[derive(Clone, Debug, Default)]
   pub struct Snapshot {
       writes: Vec<Callback<()>>,
   }

   impl Snapshot {
       #[must_use] pub fn new() -> Self;
       #[must_use] pub fn len(&self) -> usize;
       #[must_use] pub fn is_empty(&self) -> bool;
       pub fn capture(&mut self, write: Callback<()>);
       #[must_use] pub fn restore(&self) -> usize;
       pub fn clear(&mut self);
   }
   ```

   - **`Callback<()>` is `crate::widgets::Callback<()>`, not `Rc<dyn Fn()>`.** It is
     the crate's own erased action type, it is already the return type of a button's
     handler, and a second erased-closure type beside it would be
     *two documents each claiming ownership of one definition*
     with an extra struct.
   - **`capture` appends and says so**; it does **not** replace and it does **not**
     return a value (there is nothing to fail). `restore` takes `&self`, runs every
     set write **in reverse**, and returns **how many ran** — an unset
     `Callback::none()` is skipped and not counted, which is the error path and is
     tested.
   - **`restore(&self)`, not `restore(&mut self)`: a snapshot restores twice.** The
     demo's defroster uses that: the third touch restores and clears, and a fourth
     cycle captures again, so no test depends on a snapshot being single-use.
   - **`#[must_use]` on `restore`** (ignoring the count is how a caller misses that
     nothing was captured) and **not** on `clear`.
   - **The module doc records**: what a snapshot is and is not (not a map of names,
     not a single property, not an undo stack); why reverse order; why
     `Property::on_change` is `Fn` and cannot accumulate; and the demo sentence it
     exists for, quoted with its `[A]` tag.

3. **`ui/src/ui_core/src/property.rs`'s module doc loses one false claim and gains
   one true one.** Its fourth line reads *"the inheritance that lets a property
   defer to its parent"* — and `grep -rn inherit ui/src/ui_core/src/property.rs`
   returns that line and nothing else: **`Property` has no parent and no
   inheritance.** The line becomes *"and the mode scope that lets a subtree resolve
   one mode — [`crate::mode::ModeScope`], which is the crate's only inheritance."*
   **This is one sentence in one doc comment, and it is in this task because this
   task is where the truth lands**: leaving a module doc claiming an inheritance
   that has just been given a name and an owner is the defect
   `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice
   (*"Two doc comments assert the opposite of the code beside them"*).
   **Nothing else in `property.rs` changes** — no signature, no bound, no field.

4. **`Slider::save`, in `ui/src/ui_core/src/widgets/slider.rs`, immediately after
   `Slider::set_step`:**

   ```rust
   /// Returns the write that puts this slider's value back where it is now.
   ///
   /// **It captures the value, not the property's later contents.** The closure
   /// reads `self.value.get()` *now*, holds the number, and writes it back; a
   /// closure that held only the property would restore whatever the value had
   /// become, which is a no-op wearing a restore's clothes.
   #[must_use]
   pub fn save(&self) -> Callback<()> {
       let saved = self.value.get();
       let target = self.value.clone();
       Callback::new(move || target.set(saved))
   }
   ```

   **Four lines, one doc paragraph, and no change to any existing method.** A
   `Saveable` trait is **not** introduced: two widgets is one seam below the
   threshold in `developer.md` § *Phase 2* (*"No abstraction before the second
   use"*), and a trait whose every impl is these four lines buys a vtable and a
   name for no caller. **`Toggle::save` is not added** and is named in § *Out of
   Scope*.

5. **A new widget `ui/src/ui_core/src/widgets/segmented.rs`, declared
   `pub mod segmented;` in `ui/src/ui_core/src/widgets/mod.rs`** in alphabetical
   position (between `scroll` and `slider`), so the module list carries **sixteen**
   entries — **or seventeen if `TASK_UI_PRIM_40`'s `rotator` has landed, and the
   handoff says which it found**. Five `pub` constants, two small types and the
   widget:

   ```rust
   pub const SEGMENT_WIDTH: f32 = 64.0;
   pub const SEGMENT_HEIGHT: f32 = 44.0;
   pub const SEGMENT_FONT_SIZE: f32 = 16.0;
   pub const SEGMENT_PADDING: f32 = 6.0;
   pub const SEGMENT_RADIUS: f32 = 8.0;

   /// One option: the label drawn for it and the value it stands for.
   #[derive(Clone, Debug, PartialEq)]
   pub struct Choice { pub label: String, pub value: f32 }

   /// The colours a segmented control draws with, from four theme tokens.
   #[derive(Clone, Copy, Debug, PartialEq)]
   pub struct Palette {
       pub track: Color,
       pub text: Color,
       pub selected_fill: Color,
       pub selected_text: Color,
       pub ring: Color,
   }

   pub struct Segmented {
       /// The selected choice's value. **Survives `set_choices`.**
       pub value: Property<f32>,
       /// Whether this control holds focus, written by the caller from
       /// [`input::Focus`](crate::input::Focus).
       pub focused: Property<bool>,
       /// The focus ring's thickness in pixels; zero draws no ring.
       pub focus_ring: Property<f32>,
       choices: Vec<Choice>,
       palette: Palette,
       node: Handle,
   }
   ```

   `Choice` gets `#[must_use] pub fn new(label: impl Into<String>, value: f32) -> Self`.
   `Palette` gets a `Default` of neutral greys in the shape `Slider`'s and
   `Toggle`'s have, and `#[must_use] pub fn from_theme(theme: &Theme) -> Self`
   mapping **`track` ← `Border`, `selected_fill` ← `Primary`, `selected_text` ←
   `OnPrimary`, `text` ← `Text`, `ring` ← `Text`**, each with the reason
   `Palette::from_theme`'s own doc gives — including the ring, which is drawn on the
   background and so takes `Text` rather than `OnPrimary`. **`Palette` adds no
   `ThemeToken`**, for § *The theme gains no token*'s reason, and the doc says so.

6. **`Segmented`'s methods, each `#[must_use]` where ignoring the answer is a
   silent no-op:**

   - `pub fn new(nodes: &mut Arena<WidgetNode>, choices: Vec<Choice>) -> Self` —
     `node::create` with `LayoutState::new()`, `value` at the **first** choice's
     value (`0.0` and the first choice's value when there are none), `focused` and
     the palette at their defaults, `focus_ring` at `1.0`. **`new` takes no tuning
     argument**, in `Rotator`'s recorded reason (task 40 requirement 1: *"a widget
     whose limits are arguments is a widget whose limits two callers disagree
     about"*).
   - `pub fn handle(&self) -> Handle`.
   - `pub fn choices(&self) -> &[Choice]`.
   - `pub fn set_palette(&mut self, palette: Palette)` and `pub fn palette(&self) -> Palette`,
     in `Slider`'s and `Toggle`'s exact shape.
   - **`pub fn set_choices(&mut self, choices: Vec<Choice>) -> bool`** — stores the
     set, **re-projects `value` onto it, and returns whether the value moved**. The
     projection rule, which is the whole of claim (c):
     1. **an empty set leaves `value` alone** and the control draws nothing;
     2. **a set that carries the value exactly keeps it bit-identical** — and
        `Auto → Manual` in the demo is exactly this case;
     3. **otherwise the value is projected onto the choice whose value is nearest**,
        ties to the **lower**. That is `Slider::set_step`'s own rule, borrowed by
        name, and it is what makes the *unrepresentable* case a stated answer
        rather than a clamp to an end.
   - **`pub fn selected(&self) -> Option<usize>`** — the index of the choice carrying
     `value` **exactly**, else the **nearest** by value, else `None` when there are
     no choices. **Derived from `value`, never stored:** a second `selected` field
     is a second thing that can disagree with the value, which is the separation
     composite row 4 demands.
   - `pub fn select(&self, index: usize) -> bool` — writes that choice's `value`;
     `false` for an out-of-range index, or when `value` already holds it.
   - `pub fn size(&self) -> Size` — `Size::new(SEGMENT_WIDTH * choices.len() as f32,
     SEGMENT_HEIGHT)`, **with no measurement argument**, because the columns are
     equal and a label too wide for one is truncated rather than widening it.
   - `pub fn segment_at(&self, position: Offset, rect: Rect) -> Option<usize>` —
     `None` for no choices, `None` for a position outside `rect`, else
     `floor((position.x - rect.x) / (rect.width / n))` clamped to `n - 1`.
     Public in `Slider::value_at`'s shape, **because the demo routes a tap by it**.
   - `pub fn save(&self) -> Callback<()>` — requirement 4's shape, on `value`.
   - **No `tick`, no `animate_to_state`, no `Style`, no `Motion`, no
     `AnimationClock`.** Task 40's recorded reason, applied verbatim: *"a `tick`
     over an empty clock is a method that returns `false` forever"*, and no
     transition for a segmented selection has a source in this repository.
   - **No `padding_h` / `padding_v` / `border_radius` properties.** The geometry is
     five constants; a property no caller sets is dead code.

7. **`Segmented::on_event(&self, event: &mut InputEvent, rect: Rect) -> bool`** —
   `Toggle::on_event`'s exact signature and shape, with **two** arms and a
   `_ => false`:

   - **`InputEventKind::Tap`** — `if !over(rect, event.position()) { return false; }`,
     then `segment_at`; **`None` — which with equal columns can only mean the
     control has no choices at all — returns `false` unconsumed**, otherwise
     `event.consume()`, `select(index)`, `true`. **Every position inside the rect
     maps to a column**, because the columns are equal and `segment_at` floors:
     there is no dead zone between two of them, and a control with a dead zone
     would be one a finger could miss.
   - **`InputEventKind::KeyDown { key, .. }`** — `if !self.focused.get() { return
     false; }`, then a private `fn choice_key(key: &Key) -> Option<isize>` in
     `Slider::adjustment`'s shape: `Right` and `Down` are `+1`, `Left` and `Up` are
     `−1`, the d-pad likewise, `None` returns `false` unconsumed. **A hit consumes,
     and the index is clamped rather than wrapped**, in `Slider::step_by`'s shape:
     `Right` on the last column consumes and selects the last column again.
     **`Focus::focus_next`'s own doc says the focus order wraps at the ends, and
     that is the contrast**: a control that wrapped inside its own row would jump
     from `High` to `Low` on one keypress, which is a different answer from the one
     the `Tab` order gives for the same key.
   - **Seven variants reach `_ => false`** — `LongPress`, `Swipe`, `Pinch`, `Drag`,
     `KeyUp`, `Scroll`, `Text` — **each named in the arm's doc comment with why**.
     `Drag` and `Scroll` are the substantive two: **a segmented control is tapped,
     not dragged**, and there is no documented gesture that scrolls it.

8. **`Segmented::paint(&self, rect: Rect, advance: &dyn Fn(char) -> f32,
   line_height: f32) -> Vec<DrawCommand>`** — `Button::paint`'s exact signature,
   because a `Segmented` draws text and **`DrawCommand::Text` carries no width**
   (row `L8`), so the columns must be measured rather than assumed. The command
   order is the requirement, in `Slider::paint`'s own words — **the ring is under
   the track, the selected column is above the track, and every label is above
   every fill**:

   1. the **focus ring**, only if `focused` and `focus_ring.get() > 0.0`: one
      `DrawCommand::RoundedRect` of `rect` grown by the ring on every side, radius
      `SEGMENT_RADIUS + ring`;
   2. the **track**: one `DrawCommand::RoundedRect` of the whole `rect` in
      `palette.track`;
   3. the **selected column**, if there is one: a `RoundedRect` of that column
      inset by `SEGMENT_PADDING` in `palette.selected_fill`;
   4. **one `DrawCommand::Text` per choice, in order**, laid out through
      `label::layout_text` with `max_width` = the column's inner width,
      `align: TextAlign::Center`, `wrap: WrapMode::None`,
      `truncation: Truncation::Ellipsis`, coloured `palette.text` and
      `palette.selected_text` by whether that column is the selected one.

   The doc comment states **why 1 is under 2**: `DrawCommand::RoundedRect` *fills*
   its rect, so a ring drawn above the track is a white card with a track on it —
   *"the third defect in this repository found only by looking at the screen"*, in
   `Button::paint`'s and `Toggle::paint`'s own words, both of which this reuses.
   **Every colour is read at paint time**, so a mid-frame palette change shows.

9. **`segmented.rs`'s module doc**, in the form `rotator.rs` and
   `render/mesh.rs` carry: what the widget is and what it is **not** (no matrix,
   no mesh, no children, no `LayoutMode::Grid`, no clock, no drag); **why the value
   is a point on a scale and not an index**, with `Slider::set_step`'s
   *"cannot reach 1"* quoted as the reason; why `set_choices` **projects** rather
   than refuses, with the tie rule; **why there is no `Property<Choice>`** —
   `GaugeType`'s own position, *"a mode must not change in the middle of a frame"*;
   and that `Palette` adds no `ThemeToken`, with row `L9` named.

10. **The demo's climate strip, all in `ui/src/ui_demo/src/main.rs`.** One private
    mode enum, three choice-set builders, one container, five widgets, and the
    registration every one of them needs:

    - **`enum ClimateMode { Manual, Auto }`**, private, `Clone + Copy + Debug +
      PartialEq + Eq`, with a doc comment quoting
      `DEMO_APPLICATION.md` § *Climate — three surfaces*: *"Use the slider to
      adjust the fan speed. **When in Auto, the fan speed levels change to
      Low/Medium/High.**"* [A].
    - **`fn manual_fan() -> Vec<Choice>`** — seven choices, `"1"`…`"7"` at values
      `1.0`…`7.0`.
    - **`fn auto_fan() -> Vec<Choice>`** — three choices, `"Low"` at `1.0`,
      `"Medium"` at `4.0`, `"High"` at `7.0`. **The doc comment carries the
      arithmetic that makes the round trip work**: Auto's three levels are the
      manual scale's **ends and its middle**, so a value stored under `Auto` is
      carried *exactly* by `Manual`'s grid (the identity case), and a value stored
      under `Manual` is projected onto `Auto` by the nearest rule. **This is the
      design choice, stated where it is made**, and
      `the_fan_survives_the_auto_switch_and_is_presented_through_the_new_one` is its
      test.
    - **`fn airflow() -> Vec<Choice>`** — `"Face"`, `"Both"`, `"Feet"` at `1.0`,
      `2.0`, `3.0`, for the *"three-way airflow distribution"* [A] the full-screen
      climate surface documents. **The three vent names are this demo's and not the
      manual's** — § *Climate — three surfaces* names the count and not the labels —
      **and the doc comment says exactly that**, in the register
      `DEMO_APPLICATION.md` § *Could not verify* uses.
    - **`Demo` gains**: `climate: Container` (**appended** to `containers`, **never
      inserted** — `Demo::controls_layer` and `Demo::tab_bar` name containers by
      index, `CONTROLS_LAYER` is 3 and `TAB_BAR` is 5), `climate_mode_button:
      Button`, `climate_fan: Segmented`, `climate_airflow: Segmented`,
      `climate_setpoint: Slider`, `defroster: Button`,
      `climate_modes: ModeScope<ClimateMode>`, `climate_applied:
      Option<ClimateMode>`, `climate_snapshot: Snapshot`,
      `pending_climate_mode: Property<Option<ClimateMode>>` and
      `pending_defroster: Property<Option<DefrosterState>>` (requirement 12), and
      `const CLIMATE_CONTAINER: usize = 6;` beside `CONTROLS_LAYER` and `TAB_BAR`.
      **There is no `defroster_state` field**: the state is the button's label, and
      a field beside it would be a second thing that can disagree — which is what
      `Segmented::selected` being *derived* rather than stored argues against, on
      the same line.
    - **The container is `LayoutMode::Absolute`, `Constraints::tight(CLIMATE_SIZE)`,
      `set_position(Some(Offset::new(CLIMATE_ORIGIN.0, CLIMATE_ORIGIN.1)))`,
      attached to the root after the controls layer.** **`Absolute` and not
      `Stack`, by the rule *a position API that only one parent mode
      reads*: `arrange_stack` never reads `position` and `arrange_absolute` is the
      one arm that does.** Every child declares its own `set_position`, in
      coordinates relative to the container's origin, and the container gives it a
      background of **no colour at all** so
      `the_container_with_a_background_are_the_card_and_the_bar` passes unamended.
    - **The placement constants**, each doc-commented in the voice the neighbouring
      constants carry, **and each one a proposal rather than a measurement** — task
      40 requirement 7's own sentence, kept:

      | Constant | Value | Why that number |
      |---|---|---|
      | `CLIMATE_ORIGIN` | `(60.0, 500.0)` | the `controls` page's own rects are the slider, its readout, the toggle, the progress bar and its readout, **and every one of them is at `x ≥ CONTROLS_ORIGIN.0 = 664`** — so `x ∈ [60, 660)` on that page is free, and `500.0` is clear of the fps readout at `FPS_READOUT_ORIGIN.1 = 684` |
      | `CLIMATE_SIZE` | `Size { width: 580.0, height: 100.0 }` | ends at `x = 640` and `y = 600`: clear of `664` by 24 px and of the fps readout by 84 |
      | `CLIMATE_ROW_GAP` | `56.0` | the second row's top, `500.0 + 56.0 = 556.0`, leaving 44 px below it inside the 100 px box |
      | `CLIMATE_MODE_ORIGIN` / `CLIMATE_MODE_SIZE` | `(0.0, 0.0)` / `Size { width: 100.0, height: 44.0 }` | row 1, left. **The rect is fixed rather than measured**, because a row of five controls whose first column depends on a label's measured width puts a floating-point measurement between two constants; `"Manual"` and `"Auto"` are both short and `the_mode_button_label_fits_its_fixed_box` is the test that says so |
      | `CLIMATE_FAN_ORIGIN` | `(112.0, 0.0)` | row 1, right of the mode button; **`Size::new(CLIMATE_FAN_WIDTH, 44.0)` with `CLIMATE_FAN_WIDTH = 448.0 = 7 × SEGMENT_WIDTH`** — the box is **fixed at the manual width**, so `Auto`'s three columns are wider and **nothing outside the strip moves**. `112.0 + 448.0 = 560 ≤ 580` |
      | `CLIMATE_AIRFLOW_ORIGIN` | `(0.0, 56.0)` | row 2, left; `3 × SEGMENT_WIDTH = 192.0` wide, ending at `192.0` |
      | `CLIMATE_SETPOINT_ORIGIN` | `(208.0, 56.0)` | row 2, middle; **`Size { width: CLIMATE_SETPOINT_LENGTH, height: 44.0 }` with `CLIMATE_SETPOINT_LENGTH = 220.0`**, so it ends at `428.0` and leaves `436.0` for the defroster |
      | `CLIMATE_SETPOINT_MIN` / `_MAX` / `_STEP` | `16.0` / `30.0` / `Some(0.5)` | **the step divides the range** — `14.0 / 0.5 = 28` — which is precisely the fix `Slider::set_step`'s doc gives for *"cannot reach 1"*, and `the_setpoint_reaches_both_of_its_ends` is its test. **The range itself is this demo's and has no source in this repository**, and the doc comment says so |
      | `CLIMATE_DEFROSTER_ORIGIN` | `(436.0, 56.0)` | row 2, right; `Size { width: 144.0, height: 44.0 }`, ending at `580.0` — the container's own width, so `436.0 + 144.0 = 580.0` leaves nothing floating |

      **`CLIMATE_RECT`** is a `fn climate_rect(&self) -> Rect` in the demo, built
      from `CLIMATE_ORIGIN` and `CLIMATE_SIZE`, and it is **the band the capture
      criterion names** — the one below, and requirement 16's capture step.
    - **Registration, all of it, or a row is missing**: every new node gets a
      **page-membership row** (`Page::Controls`, `focusable` true for the two
      `Segmented`s, the setpoint `Slider` and both `Button`s) — `Demo::focusables`
      filters `page_members` by page and by that flag, so a node with no row is
      **invisible to `Tab` and to the hit-test gate**; every new **leaf** is named in
      **`Demo::placed_handles`** and in **`expected_placed_rect_names`**; the paint
      walk gets one `if let Some(segmented) = …` arm per `Segmented` beside the
      slider's; and `Demo::frame_clips` is **not** touched (task 45 owns clip
      ownership; this strip needs none).

11. **`Demo::sync_climate_mode`, beside `Demo::sync_toggle_state`, and where it is
    called.** It resolves the mode **once** and applies it **only on a change**:

    ```rust
    fn sync_climate_mode(&mut self) {
        let wanted = self.resolve_climate_mode();
        if self.climate_applied == Some(wanted) {
            return;
        }
        self.climate_applied = Some(wanted);
        let choices = match wanted {
            ClimateMode::Manual => manual_fan(),
            ClimateMode::Auto => auto_fan(),
        };
        self.climate_fan.set_choices(choices);
        self.climate_mode_button
            .label
            .set(self.climate_mode_name(wanted).to_string());
        self.animate_button(&mut self.climate_mode_button);
    }

    fn resolve_climate_mode(&self) -> ClimateMode {
        let nodes = self.nodes.borrow();
        self.climate_modes
            .effective(&nodes, self.climate.handle(), ClimateMode::Manual)
    }
    ```

    - **`climate_modes.set(self.climate.handle(), mode)` is the demo's only write
      to the scope**, and it happens in the **drain at the end of
      `Demo::handle_event`**, never in a frame — so the scope changes on a gesture,
      exactly as `GaugeType`'s setter discipline requires.
    - **`sync_climate_mode` is called from `Demo::frame`, immediately after
      `sync_toggle_state()` and before the layout block.** The comment carries the
      two reasons: the crate's *"aim once, tick per frame"* idiom
      (`sync_toggle_state`'s argument, cited by name), and **no `mark_dirty` is
      needed because no placed rect depends on the mode** — with
      `a_mode_change_moves_no_rect_in_the_tree` as the test that holds it down.
    - **`animate_button` is `Demo::release_tab`'s existing
      `animate_to_state(Motion::from_theme(&self.theme))` call applied to a named
      widget**, in whatever shape the implementer prefers — **not a new mechanism,
      and no new clock.**

12. **The two button handlers, and the two-hop idiom they must use.**
    `Button::on_click` is `widgets::Callback<()>`, which is **`Fn`** — so a click
    cannot write a `&mut self` it was not lent. **`Demo::pending_page`'s own doc
    states the rule this task inherits by name:** *"`Callback` is `Fn`, so a click
    cannot write a `&mut self` it was not lent, and **a property is the only thing
    it can be given that reaches the demo**."* So:

    - **`pending_climate_mode: Property<Option<ClimateMode>>`** and
      **`pending_defroster: Property<Option<DefrosterState>>`** — **two one-shot
      mailboxes, drained at the end of `Demo::handle_event` beside the existing
      `pending_page` / `pending_theme` / `pending_key` drains**, and `None` is
      written back on the way out, exactly as `pending_page`'s drain does.
    - **`enum DefrosterState { Off, Defog, Defrost }`**, private, `Clone + Copy +
      Debug + PartialEq + Eq`. **The button's handler writes the *next* state, not
      the work** — `pending_defroster.set(Some(next))` — so the button's closure is
      two lines and every transition lives in the demo where `&mut self` exists.
    - **`Demo::step_defroster(&mut self)`, called from that drain**, with the three
      transitions and nothing else:

      | From | To | What `step_defroster` does |
      |---|---|---|
      | `Off` | `Defog` | **capture**, in this order: `climate_snapshot.capture(climate_airflow.save())`, then the setpoint, then the fan. A comment records that this order is the one `Snapshot::restore` **inverts** |
      | `Defog` | `Defrost` | **nothing but the label.** The three captured values are **not** re-captured, and the label write is the only visible change |
      | `Defrost` | `Off` | **restore and clear**, the two adjacent so a restore that ran nothing shows in the next one's count: `let restored = self.climate_snapshot.restore();` then `self.climate_snapshot.clear();` |

    - **`Demo::climate_mode_name(ClimateMode) -> &'static str` is `"Manual"` and
      `"Auto"`,** and the label is written by `sync_climate_mode` rather than by the
      handler, so **one place writes it.**
    - **The drain is safe here, and the comment says why with the recorded hazard.**
      `pending_page`'s drain refuses to switch pages inside the routing walk because
      `sync_page_visibility` writes `set_visible` on the arena the walk is reading.
      **A mode change and a restore write `Property` values and no arena state**, so
      neither reaches `set_visible` and neither re-places a node — and the comment
      names the hazard it is *not* subject to rather than asserting safety.
    - **The three labels are `"Defog"`, `"Defrost"` and `"Off"`** — the label is
      the state the button is *in*, written on the transition into it — and the
      doc comment quotes the manual's third touch with its `[A]` tag, as composite
      row 11 records it.

13. **The demo's derived tables, all extended and none loosened.** Every one of
    these is a **count or a list this task changes**, and the change is an addition
    to the expected set rather than a relaxation of the assertion:

    | Table / test | Change |
    |---|---|
    | `containers.len()` in `every_parent_the_demo_assembles_is_a_container_widget` | **6 → 7**, with the seventh named in the assertion's message |
    | `page_members` rows | five new rows, `Page::Controls`, `focusable: true` for all five widgets — a node with no row is **invisible to `Tab` and to the hit-test gate**, because `Demo::focusables` filters that table by page and by that flag |
    | `Demo::placed_handles` | five new `(&'static str, Handle)` rows: `"climate mode button"`, `"climate fan"`, `"climate airflow"`, `"climate setpoint"`, `"defroster button"` — and `assert_placed_handles_is_complete`'s length follows |
    | `expected_placed_rect_names` | the same five names |
    | `page_focusables_count(Page::Controls)` | **3 → 8** |
    | `page_focusables(Page::Controls)` | the three existing rows then the five new ones, **in the container's child order**, because the `Tab` order is the tree's paint order |
    | `shows_focus` | five new arms; the two `Segmented`s and the setpoint `Slider` read `focused.get()` and the two `Button`s likewise |
    | `press_all` / `release_all` | `press_climate_mode` / `release_climate_mode` and `press_defroster` / `release_defroster` in **both**, on `a_release_is_never_gated_so_nothing_can_be_stranded_mid_press`'s argument |

    **And the hit-test chain, which is the demo's own shape and not a new one.**
    `Demo::climate_at(&self, x, y) -> Option<()>` and
    `Demo::climate_rect(&self) -> Option<Rect>` in `Demo::slider_at`'s and
    `Demo::slider_rect`'s exact shape — **`if !self.on_show(self.climate.handle())
    { return None; }` first, because without the page gate a tap aimed at the strip
    would still be reachable from a page that does not draw it** — one `else if` in
    each existing press arm **after** the four existing ones, one line in each of
    the three release arms, and a press arm **inside the existing
    `dialog_is_modal()` guard** with the four other press arms. `Demo::frame_clips`
    is **not** touched: task 45 owns clip ownership and this strip needs no clip.

14. **The tests, named, with no display, no network, no filesystem and no wall
    clock** — the only kind `AGENTS.md` permits. Each names the mutation it kills,
    because `developer.md` § Phase 3 (*"A test that has never failed is not a
    test"*) and *a survivor is a missing assertion* both require
    it.

    **In `mode.rs`:**

    - **`two_siblings_under_one_owner_resolve_the_same_mode`** — *the required one.*
      An owner node with two children; the mode set at the owner; **both children
      resolve it, and a third node outside the owner does not.** Kills: a `resolve`
      that answers from the map's first entry whatever the node is, and any
      implementation that puts the mode on a widget instead of on the tree.
    - `a_node_resolves_the_mode_set_on_itself` — kills a `resolve` that starts at
      `parent()`.
    - `the_nearest_ancestor_wins_and_a_distant_one_does_not_reach_past_it` — kills a
      `resolve` that returns the first match in insertion order rather than the
      nearest ancestor.
    - `a_deep_chain_of_thirty_two_resolves_the_mode_at_its_top` — kills a bounded
      walk, and pins the `O(depth)` cost in a test rather than in prose.
    - `an_entry_for_a_node_the_arena_no_longer_holds_cannot_be_resolved` — kills a
      `resolve` that follows a dead parent's chain instead of stopping at `None`.
    - `retain_drops_every_entry_for_a_node_the_arena_no_longer_holds` — **kills the
      whole `retain` method**, which is the `page_members` leak task 24.1 found.
    - `set_reports_whether_the_scope_changed_and_a_second_identical_set_does_not` —
      kills both an always-`true` and an always-`false` `set`.
    - `clear_takes_the_mode_off_the_subtree_below_it` — kills a `clear` that leaves
      the entry, which would leave `Auto` latched.
    - `effective_falls_back_for_a_node_with_no_entry` — kills an `effective` that
      unwraps.
    - `a_mode_does_not_cross_a_boundary_into_a_different_root` — two roots; the
      mode on one is `None` on the other. Kills a walk that ignores `parent() ==
      None`.

    **In `snapshot.rs`:**

    - **`a_snapshot_of_three_controls_is_restored_as_a_set`** — *the required
      snapshot/restore test at the library level.* Three widgets, three changed
      values, one `restore()`, all three bit-identical to what they were.
      **Kills: `Slider::save` / `Segmented::save` capturing the property and
      reading it at restore time.** That is the mutation a reviewer should run
      first, because the resulting code passes a test that only checks "something
      was written".
    - `restore_runs_the_writes_in_the_reverse_of_the_order_they_were_captured` —
      kills a forward `restore`.
    - `an_unset_write_is_skipped_and_is_not_counted` — the error path; kills a
      `restore` that counts a `Callback::none()` or unwraps it.
    - `a_snapshot_of_nothing_restores_nothing` — kills a `restore` with a phantom
      write.
    - `a_snapshot_can_be_restored_twice_and_still_holds_its_writes` — kills a
      `restore(&mut self)` that takes them.
    - `restore_returns_how_many_writes_ran` — the counter itself.

    **In `segmented.rs`:**

    - **`the_stored_value_survives_an_enumeration_change_and_is_presented_through_
      the_new_one`** — *the required enumeration test.* Seven numeric choices at
      `1.0 … 7.0`; `set_choices` to three named ones at `1.0 / 4.0 / 7.0`; **the
      value is `4.0` and `selected()` is `1`, unchanged**; then `set_choices` back to
      the seven and **the value is bit-identical to `4.0`**. Kills: a projection
      that renormalises, a `selected` that recomputes from the index, and the whole
      "value" concept replaced by an index.
    - `a_value_no_choice_carries_is_projected_onto_the_nearest_one` — `2.5` onto
      `1.0 / 4.0 / 7.0` is `4.0`. Kills a projection to the first, to the last, or
      to a default.
    - `a_projection_tie_goes_to_the_lower_choice` — `2.5` onto `1.0 / 4.0` is `1.0`.
      Kills an unspecified tie, which is a different answer on every architecture
      that is not tested.
    - `an_empty_choice_set_leaves_the_value_alone_and_draws_nothing` — kills a
      `set_choices` that clears the value and a `paint` that divides by zero.
    - `the_first_and_the_last_choice_are_both_reachable` — `select(0)` and
      `select(n - 1)`; kills an exclusive range.
    - `segment_at_names_the_column_a_position_is_in_and_nothing_outside_the_rect` —
      the left edge, the right edge, above and below; kills a `segment_at` that
      ignores the rect.
    - `a_tap_selects_the_choice_under_it_and_a_tap_elsewhere_selects_nothing` — kills
      an `on_event` that consumes a tap it did not handle.
    - `an_unfocused_segmented_declines_a_key_and_lets_it_travel_on` — in
      `Toggle::on_event`'s doc-example shape.
    - `the_arrow_keys_move_one_choice_and_stop_at_both_ends` — kills a **wrap**,
      which is the one wrong answer here that looks right.
    - `paint_puts_the_focus_ring_under_the_track_and_the_selected_column_above_it` —
      asserts the **position** of each command in the recorded order. Kills a
      reorder, which is invisible in a count and visible on screen as a white card.
    - `every_choice_records_exactly_one_text_run_and_it_is_that_choice_s_own_label` —
      kills a `paint` that draws the first label *n* times.
    - `a_label_wider_than_its_column_is_truncated_within_it` — kills the overflow
      into the neighbouring column, which is row `L8` arriving early.
    - `the_text_runs_are_centred_in_their_own_column` — kills left-aligned labels.
    - `the_five_constants_are_positive_finite_and_seven_columns_span_448_pixels` —
      the arithmetic, in task 34's `DEPTH_BITS` shape: all five positive and
      finite, `SEGMENT_HEIGHT` equal to `Button`'s private `MIN_TOUCH_TARGET`
      (`44.0`, the crate's one record of a target a finger can hit), and
      `7 * SEGMENT_WIDTH == 448.0` exactly.

    **In the demo:**

    - **`the_third_defroster_touch_restores_the_three_controls_the_first_captured`**
      — *the required snapshot/restore test, end to end.* Through
      `Demo::offer_to`: first touch, change all three values, second touch, change
      them again, third touch — **all three are back at the captured values.**
      Kills a defroster that restores one of them, and a capture that happens on the
      second touch.
    - **`the_fan_survives_the_auto_switch_and_is_presented_through_the_new_one`** —
      set the fan to `4.0`, switch to `Auto` through the mode button, assert three
      choices and `selected() == Some(1)`; switch back, assert seven and `4.0`
      **bit-identical**. Kills a mode that is resolved but never applied, and an
      applied one that resets the value.
    - `the_defroster_captures_once_on_the_first_touch_and_not_again_on_the_second` —
      kills a capture on every transition.
    - `a_mode_change_moves_no_rect_in_the_tree` — **the pixel-identical criterion as
      a test:** `placed_rects()` is identical before and after an `Auto`/`Manual`
      switch. Kills any change that grows a widget's box with its content.
    - `the_climate_strip_is_page_content_on_controls_and_of_no_other_page` — kills a
      missing or doubled page-membership row.
    - `the_climate_strip_overlaps_nothing_the_gallery_placed_on_its_page` — the
      premise requirement 10's numbers rest on, by name.
    - `tab_reaches_every_climate_control_and_wraps` — the walk on `controls`, with
      the derived count from requirement 13.
    - `the_mode_is_resolved_from_the_strip_node_and_from_nothing_else` — a mode set
      on one node is not visible from a sibling's subtree. Kills a `resolve` that
      ignores the handle.
    - `the_setpoint_reaches_both_of_its_ends` — drives the setpoint through
      `Slider::step_by` to each end and asserts `16.0` and `30.0` exactly. **This is
      `Slider::set_step`'s documented sharp edge as an assertion**, and it is here
      because requirement 10 chose a step that divides the range for exactly that
      reason.
    - `the_mode_button_label_fits_its_fixed_box` — `Button::content_size` for
      `"Manual"` and `"Auto"` is at most `CLIMATE_MODE_SIZE.width`, through the
      demo's own `metrics`. Kills a fixed column that the longest label overflows,
      which is the one number in requirement 10's table that is not an arithmetic
      fact.

    **In `slider.rs`:** `a_saved_slider_value_is_put_back_where_it_was` — the
    one-line round trip on its own, so a failure in `Snapshot` and a failure in
    `Slider::save` are two different failures.

15. **The documentation, three files, all of them dated and attributed.**

    - **`DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core***, row
      **`L6b`**, gains a dated note recording: (i) **clause (b) and clause (c) are
      closed** by `mode::ModeScope`, `snapshot::Snapshot` and
      `widgets::Segmented`, with the row's own three quoted demands against them;
      (ii) **clause (a) is handed to row `L9`**, with the reason from § *The theme
      gains no token*; (iii) **the row is not deleted and not marked closed**,
      because clause (a) is inside it; (iv) **no evidence count in the row changes**
      — the twelve `pub enum`s stand, and this task adds none. Row **`L6a`** is not
      touched at all.
    - **`DEMO_APPLICATION.md` § *Composite widgets***, composite row **4**, gains a
      dated note naming `TASK_UI_PRIM_47` for its `L6b` demand and stating which of
      its nine instances are now demonstrable in the demo (**the fan's enumeration
      in `Auto`** and **the defroster's restore**) and which are not (**Track
      Mode's recolouring**, whose rules § *Could not verify* records as `[B]` and
      *"not a spec the demo should copy until verified"*).
    - **`PRIMITIVES_ARCHITECTURE.md` § *Module Layout*** gains `mode.rs` and
      `snapshot.rs` after `input.rs`, and `widgets/segmented.rs` in the widget list.
      **The `rotator.rs` row is not this task's business**: if `TASK_UI_PRIM_40` has
      landed, the row is missing and the handoff **records that and does not fix
      it**; if it has not, there is nothing to record.
    - **`PRIMITIVES_ARCHITECTURE.md` § *Inheritance*** is **corrected in place**. It
      currently reads *"Some properties inherit from parent to child unless
      overridden: `font_family`, `font_size`, `color`, `opacity`, `visibility`"* —
      and **none of the five is true**: `grep -rn inherit ui/src/ui_core/src/`
      returns prose and no mechanism. The section becomes a statement of what
      exists: **`mode::ModeScope` is the crate's only inheritance**, it inherits
      **one thing**, a caller-chosen `K`, and it is resolved by the owner rather
      than by the widget. **This is recorded as a finding in
      `IMPLEMENTATION_STATE.md` with what was searched**, in the register of
      `DEMO_APPLICATION.md` § *Corrections to the second gap table*.
    - **`doc/ui/IMPLEMENTATION_STATE.md` gains one entry** carrying: the three type
      names and the two `save` methods; **the architecture decision and the three
      rejected alternatives with their reasons**; **that clause (a) went to `L9` and
      that `TOKEN_COUNT` is 33, private, and unchanged**; the reverse-order rule;
      the resolution point in `Demo::frame` and **that no `mark_dirty` is needed
      because no rect depends on the mode**; the `PRIMITIVES_ARCHITECTURE.md`
      § *Inheritance* finding; the six pages' frame rates and the `controls` page's
      separately; and **the honest limit** — *the mode lookup and the
      snapshot/restore are verified by test and pure; **no pointer event has ever
      been observed reaching this window** on this host, so nothing here is evidence
      that a finger changes the mode or touches the defroster.*
      `IMPLEMENTATION_STATE.md` is not a source of evidence
      (`.ai/workflows/task-sequence.md` § *State*); it points at the code.

16. **The suite, the capture and the frame rate are all produced**, by the commands
    of `.ai/tools/README.md` § *Capturing a window* and of the criterion below: `cargo fmt --check`, `cargo build
    --all-targets --all-features`, `cargo clippy --all-targets --all-features --
    -D warnings`, `cargo test --all-features` with the per-binary counts pasted,
    `cargo doc --no-deps` clean, `cargo audit` **recorded as not installed on this
    host, not passed**; then the six-page before/after capture, **window id re-read
    at the time of each capture** rather than taken from an earlier note; then **the
    frame rate on all six pages**.

## Acceptance Criteria

- [ ] **`ModeScope<K>` exists, is generic over the caller's mode, and inherits down
      a subtree.** `grep -n 'pub struct ModeScope' ui/src/ui_core/src/mode.rs` shows
      the struct and its `entries: Vec<(Handle, K)>`; `grep -n 'pub mod mode'
      ui/src/ui_core/src/lib.rs` shows the declaration **in alphabetical position**;
      `grep -c 'pub struct\|pub enum' ui/src/ui_core/src/mode.rs` returns **1** —
      the crate has **no mode vocabulary of its own**, and that is the decision.
      `two_siblings_under_one_owner_resolve_the_same_mode` passes, and the handoff
      states in one sentence what the mutation was: make `resolve` return the map's
      first entry whatever the node, and watch that test fail while the other seven
      stay green.

- [ ] **`resolve` is a nearest-ancestor walk and stops at the arena's answer.**
      `the_nearest_ancestor_wins_and_a_distant_one_does_not_reach_past_it`,
      `a_node_resolves_the_mode_set_on_itself`,
      `a_deep_chain_of_thirty_two_resolves_the_mode_at_its_top` and
      `an_entry_for_a_node_the_arena_no_longer_holds_cannot_be_resolved` all pass.
      **Mutation evidence in the handoff:** drop the loop's `parent()` step so it
      reads the map once, and watch the first and the fourth fail for that reason.
      The mode is **not** a `Property`: `grep -n 'Property' ui/src/ui_core/src/mode.rs`
      returns nothing.

- [ ] **`retain` exists and drops a dead entry.** `retain_drops_every_entry_for_a_
      node_the_arena_no_longer_holds` passes. **Mutation evidence:** delete the
      `retain` body and watch it fail; restore it and watch it pass. The rule it
      encodes is task 24.1's `page_members` leak, and the method doc says so by
      name.

- [ ] **`Snapshot` is a set of `widgets::Callback<()>` written in reverse.**
      `grep -n 'pub struct Snapshot' ui/src/ui_core/src/snapshot.rs` shows
      `writes: Vec<Callback<()>>` and `grep -n 'Rc<dyn Fn' ui/src/ui_core/src/
      snapshot.rs` returns **nothing** — the crate's erased action type is used
      rather than a second one.
      `a_snapshot_of_three_controls_is_restored_as_a_set` passes.
      **Mutation evidence, and this is the one a reviewer should break first:**
      change `Slider::save` and `Segmented::save` to capture only the property and
      read it inside the closure, and watch that test fail — **and every other test
      in the file stay green**, because a restore that restores the current value is
      indistinguishable from a restore that works by any test that does not change
      the value first.

- [ ] **Two sibling widgets under one owner observe the same mode, end to end.**
      `two_siblings_under_one_owner_resolve_the_same_mode` is named in the handoff
      with its output, and the demo's `the_mode_is_resolved_from_the_strip_node_and_
      from_nothing_else` shows the same rule one level up with real widgets.

- [ ] **The stored value survives an enumeration change and is presented through
      the new one.** `the_stored_value_survives_an_enumeration_change_and_is_
      presented_through_the_new_one` passes, and its assertions are **exact**: the
      value is `4.0` across the change and **`to_bits()`-identical** on the way
      back. `a_value_no_choice_carries_is_projected_onto_the_nearest_one` and
      `a_projection_tie_goes_to_the_lower_choice` pin the projection and its tie.
      `the_fan_survives_the_auto_switch_and_is_presented_through_the_new_one` is the
      same property through the demo's own `Auto` button.
      **Mutation evidence:** make `set_choices` renormalise the value into the new
      set's range (`(value - old_min) / (old_max - old_min) * new_span + new_min`)
      and watch the first fail on the bit-identity, not on the number.

- [ ] **The defroster's third touch restores the three controls the first
      captured.** `the_third_defroster_touch_restores_the_three_controls_the_first_
      captured` passes through `Demo::offer_to`, with **three** values asserted, and
      `the_defroster_captures_once_on_the_first_touch_and_not_again_on_the_second`
      passes. **Mutation evidence:** move the `capture` call from the first
      transition to the second and watch the first test fail on the *second* change's
      values rather than the first's — which is the only way to tell the two apart.

- [ ] **`Slider` gains `save` and nothing else.** `git diff ui/src/ui_core/src/
      widgets/slider.rs` shows **one added method and one added doc paragraph**, and
      **no changed line outside them**: every existing method, every existing test
      and every existing doc comment is byte-identical. `grep -n 'fn save'
      ui/src/ui_core/src/widgets/` returns exactly **two** lines — `slider.rs` and
      `segmented.rs` — and **no `trait`** with one method over both.

- [ ] **`Segmented` exists and is the sixteenth `pub mod`, or the seventeenth.**
      `grep -c 'pub mod' ui/src/ui_core/src/widgets/mod.rs` returns **16** if
      `rotator` is absent and **17** if `TASK_UI_PRIM_40` has landed — **the handoff
      states which it found**, because a count that is wrong is the failure
      `DEMO_APPLICATION.md` § *Corrections to the second gap table* was written
      about. `grep -n 'pub mod segmented'` shows the line, **alphabetically between
      `scroll` and `slider`**. `awk '/^pub struct Segmented/,/^}/'` shows `value`,
      `focused`, `focus_ring`, `choices`, `palette`, `node` — **and nothing else, in
      particular no clock**.
      `grep -h '^pub enum' ui/src/ui_core/src/widgets/*.rs | wc -l` returns
      **12** — **unchanged**, which is `L6a`'s evidence line and this task's promise.
      (`grep -c` with a glob prints one count per file and no total, so the `wc -l`
      is what makes the number a number.)

- [ ] **The command order is the ring under the track under the selected column
      under every label.**
      `paint_puts_the_focus_ring_under_the_track_and_the_selected_column_above_it`
      asserts the **position** of each `DrawCommand::RoundedRect` in the recorded
      vector, not a count.
      `every_choice_records_exactly_one_text_run_and_it_is_that_choice_s_own_label`
      asserts one text command per choice carrying that choice's own label.
      `a_label_wider_than_its_column_is_truncated_within_it` and
      `the_text_runs_are_centred_in_their_own_column` pin the two things row `L8`
      would otherwise make invisible.
      **Mutation evidence:** move the ring's `rounded_rect` call after the track's,
      and watch the first fail while a count-based assertion stays green — which is
      the on-screen defect `Button::paint`'s doc calls *"the third defect in this
      repository found only by looking at the screen"*.

- [ ] **The keyboard is the only interaction route this host has, and it works.**
      `an_unfocused_segmented_declines_a_key_and_lets_it_travel_on` and
      `the_arrow_keys_move_one_choice_and_stop_at_both_ends` pass, the latter
      driving `Segmented::on_event` with synthetic `KeyDown`s in
      `Slider::on_event`'s own doc-example shape. **No acceptance criterion here is
      verified by a pointer-driven capture, and none asks for one.**

- [ ] **No theme token is added, and the deferral to `L9` is checkable.**
      `git diff --stat ui/src/ui_core/src/theme.rs` shows **no change**.
      `dark_and_light_define_every_token` keeps its name and its
      `assert_eq!(ThemeToken::all().len(), 33)` and passes **unamended**.
      `const TOKEN_COUNT: usize = 33;` is still `const`, still **private**, and
      **still not `pub`** — and the handoff says in one sentence why it stays
      private. `grep -n 'ThemeToken::Focus\|ThemeToken::Hover\|ThemeToken::Pressed\|
      ThemeToken::Selected\|ThemeToken::Active\|ThemeToken::Shadow\|ThemeToken::ZOrder'
      ui/src/ui_core/src/` returns **nothing**.

- [ ] **The false inheritance claim is gone from both places it is made.**
      `grep -n 'the inheritance that lets a property defer to its parent'
      ui/src/ui_core/src/property.rs` returns **nothing**, and the module doc names
      `mode::ModeScope` instead. `PRIMITIVES_ARCHITECTURE.md` § *Inheritance* no
      longer claims `font_family`, `font_size`, `color`, `opacity` or `visibility`
      inherit from a parent, and the `IMPLEMENTATION_STATE.md` entry records what was
      searched. **And no doc comment in any changed file asserts the opposite of the
      code beside it**, which is the defect `DEMO_APPLICATION.md`
      § *Corrections to the second gap table* records twice.

- [ ] **`cargo test --all-features` is green with every named test present**, and the
      handoff **lists each by name**: in `mode.rs` — `two_siblings_under_one_owner_
      resolves_the_same_mode`, `a_node_resolves_the_mode_set_on_itself`,
      `the_nearest_ancestor_wins_and_a_distant_one_does_not_reach_past_it`,
      `a_deep_chain_of_thirty_two_resolves_the_mode_at_its_top`,
      `an_entry_for_a_node_the_arena_no_longer_holds_cannot_be_resolved`,
      `retain_drops_every_entry_for_a_node_the_arena_no_longer_holds`,
      `set_reports_whether_the_scope_changed_and_a_second_identical_set_does_not`,
      `clear_takes_the_mode_off_the_subtree_below_it`,
      `effective_falls_back_for_a_node_with_no_entry`,
      `a_mode_does_not_cross_a_boundary_into_a_different_root`; in `snapshot.rs` —
      `a_snapshot_of_three_controls_is_restored_as_a_set`,
      `restore_runs_the_writes_in_the_reverse_of_the_order_they_were_captured`,
      `an_unset_write_is_skipped_and_is_not_counted`,
      `a_snapshot_of_nothing_restores_nothing`,
      `a_snapshot_can_be_restored_twice_and_still_holds_its_writes`,
      `restore_returns_how_many_writes_ran`; in `segmented.rs` — the fourteen named
      above; in `slider.rs` — `a_saved_slider_value_is_put_back_where_it_was`; in
      the demo — the ten named above. **The suite is at least 1894** — the baseline this file
      projects from, recorded at `75a896c` in `doc/ui/done/TASK_UI_PRIM_42.md`
      § *What is in the crate at `75a896c`* (1450 `ui_core` + 224 `ui_demo` + 220
      doctests) — **plus the forty-one tests requirement 14 names**
      (16 in `mode.rs` and `snapshot.rs`, 14 in `segmented.rs`, 1 in `slider.rs`, 10
      in the demo), so **1935 or more**, and **no test was deleted, renamed away or
      weakened**: the handoff lists the before and after counts per binary. **The handoff names the tree it measured on** — this
      sequence has three task files open at once, and
      *on a shared tree, the suite you ran is not your suite*
      is the rule that a number without a tree is not a result.
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is **recorded as not installed on
      this host, not passed**.

- [ ] **The six gallery pages are pixel-identical outside two named bands, and the
      mechanism is stated rather than hoped for.** Captured **before and after** with
      the commands of `.ai/tools/README.md` § *Capturing a window* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture, and `ffmpeg x11grab`, return black for a GL window),
      `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, then `magick compare -metric AE before.png
      after.png null:` per page.

      - **On `pads`, `text`, `input`, `data` and `overlays` the criterion is exactly
        the one tasks 34 to 40 inherited: AE 0 outside `y ≥ 680`**, every differing
        pixel inside the fps readout's band. **The mechanism is one fact: the five
        new widgets are `Page::Controls` page content and the paint gate empties
        them on every other page**, which is
        `the_climate_strip_is_page_content_on_controls_and_of_no_other_page` as a
        test rather than as a hope.
      - **On `controls` the criterion is AE 0 outside `y ≥ 680` *and* outside
        `CLIMATE_RECT`** — `Demo::climate_rect()`, built from `CLIMATE_ORIGIN` and
        `CLIMATE_SIZE` — and every differing pixel inside one of those two bands.
        **This is a restatement, and the reason is stated rather than discovered at
        review:** the strip is new page content and its labels and selected column
        are *supposed* to differ inside its own rect.
      - **The mechanism is three facts and not one.** **No rect the gallery placed
        moves** — and this is now a *test*, `a_mode_change_moves_no_rect_in_the_tree`,
        so it does not rest on this file's reading of the diff. **Five of six pages
        place nothing new.** And the only thing that changes inside
        `CLIMATE_RECT` is a label and a selection.
      - **The rect-level half keeps its name and every one of its assertions, and
        the reason it does is a property of the placement arithmetic rather than of
        this file's confidence.**
        `every_page_places_every_rect_where_the_gallery_placed_it` compares
        `placed_rects()` **across all six pages**, and the five new rects are the
        same on every page because a page decides *which* rects are drawn and not
        where any of them is — so it passes **unamended**.
        `no_two_placed_rects_overlap` is **per page** and it compares **every rect
        on a page against every later rect on that same page** — so it compares the
        strip's five rects **against each other**, and it passes **only because
        requirement 10's numbers do not touch**: `0 + 100 ≤ 112` and
        `112 + 448 = 560 ≤ 580` on row 1, `0 + 192 ≤ 208`, `208 + 220 = 428 ≤ 436`
        and `436 + 144 = 580` on row 2, and the rows are `56.0` apart with a height
        of `44.0`. **That is why the arithmetic is in requirement 10 and not left to
        the implementer, and `the_climate_strip_overlaps_nothing_the_gallery_placed_
        on_its_page` is the test that says so by name.**
        `assert_the_pages_partition_the_placed_rects` **passes unamended and holds
        no list to extend** — it is a bidirectional containment check between the
        union and `placed_rects()`, so it reads like a table and is not one, and
        *a sweep of a mechanism's call sites is not a sweep of
        the data it is built from* is the rule that says so. The completeness lives
        where the lists are: `assert_placed_handles_is_complete` and
        `expected_placed_rect_names`.
        `every_placed_rect_is_inside_the_window` passes unamended:
        `CLIMATE_ORIGIN` + `CLIMATE_SIZE` ends at `(640.0, 600.0)` inside
        `WINDOW = 1280 × 1020`.

- [ ] **The frame rate is measured on every page and reported.** With the script's
      own line pasted rather than the number expected:
      `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of the
      six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `.ai/tools/README.md` § *Frame-rate baseline* records as
      the reason task 24.2's criterion 6 was amended rather than met by the script.
      Every page above the floor of **55**. **The `controls` page is expected to
      cost something and the handoff reports its number separately, with the reason
      stated:** each `Segmented` records **one focus ring (only while focused),
      one track, one selected column and one text run per choice** — **ten commands
      for the fan's seven choices and six for the airflow's three, so sixteen from
      the two `Segmented`s**, on top of whatever the two `Button`s and the setpoint
      `Slider` record — on a frame that already issues thousands. **And
      `layout::mark_dirty` is called nowhere by this task, because no placed rect
      depends on the mode**, which is the whole of the per-frame cost claim: there
      is no re-layout and no cache to invalidate.
      **The other five pages are expected to be inside the
      recorded 61.1–63.9 band in `.ai/tools/README.md` § *Frame-rate baseline*, and to have cost nothing**, because the paint
      gate empties the strip's commands before they are recorded.

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **no change** to `ui/src/ui_core/src/theme.rs`,
      `render.rs`, `render/target.rs`, `render/blur.rs`, `paint.rs`, `batch.rs`,
      `input.rs`, `layout.rs`, `arena.rs` or `animation.rs`. The only changes to
      files that already exist are **`property.rs`'s module doc (one sentence)**,
      **`slider.rs` (one added method)** and **`lib.rs` (two `pub mod` lines)**.
      `grep -c unsafe` over the three new files and over `slider.rs` is **0**, and
      `grep -n 'unwrap()\|expect(\|panic!\|unimplemented!\|todo!'` over them returns
      **nothing**.
- [ ] **No other task's file was created or edited by this one.** `git diff --stat`
      plus `git status --short` name **exactly** these paths:
      `ui/src/ui_core/src/mode.rs`, `ui/src/ui_core/src/snapshot.rs` and
      `ui/src/ui_core/src/widgets/segmented.rs` (new), and
      `ui/src/ui_core/src/property.rs`, `ui/src/ui_core/src/lib.rs`,
      `ui/src/ui_core/src/widgets/slider.rs`,
      `ui/src/ui_core/src/widgets/mod.rs`, `ui/src/ui_demo/src/main.rs`,
      `doc/ui/DEMO_APPLICATION.md` and `doc/ui/PRIMITIVES_ARCHITECTURE.md`
      (modified) — plus `doc/ui/TASK_UI_PRIM_47.md`, which is **this file** and is
      untracked until the operator adds it, which is why both commands are named.
      **No `icon.rs`, no `nav` module, no `tab_bar.rs` and no `rotator.rs` appear**,
      whether or not `TASK_UI_PRIM_40`, 42, 43 or 44 has landed — **and where one of
      those has landed, this task's diff simply does not mention it**, which is the
      form the claim can take on a tree where three task files are open at once.
      **`ui/Cargo.toml` and `ui/Cargo.lock` are unchanged** — the approved direct
      dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, and **a
      generic scope map, a vector of callbacks and a widget are three reasons not to
      have gone looking for a crate.**

- [ ] **The row is amended, dated, and not closed.** Row **`L6b`** in
      `DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core* carries a dated
      note naming `TASK_UI_PRIM_47`, stating that **clauses (b) and (c) are closed
      and clause (a) is handed to row `L9`**, and **that the row is therefore not
      deleted and not marked closed**. **No evidence count in it changes** — the
      twelve `pub enum`s stand — and row **`L6a` is not touched at all**.

- [ ] **The decisions are written down where the next agent finds them.**
      `mode.rs` and `snapshot.rs` each carry a module doc with their decision and
      its rejected alternatives; `segmented.rs`'s carries the scale-not-index
      argument with `Slider::set_step`'s *"cannot reach 1"* quoted;
      `ui/src/ui_demo/src/main.rs` carries its half — the mode enum's `[A]` quote,
      the three choice-set builders, the placement constants marked as proposals,
      the resolution point in `Demo::frame`, and the defroster's three transitions
      with the manual's third touch quoted; and `doc/ui/IMPLEMENTATION_STATE.md`
      carries the record, **including the honest limit** — *nothing here is evidence
      that a finger changes the mode or touches the defroster, because no pointer
      event has ever been observed reaching this window on this host.*

## Out of Scope

- **No re-opening row `L6a`, and no change to what already satisfies it.**
  `GaugeType`, `Gauge::set_gauge_type`, `Severity` and every one of the twelve
  `pub enum`s are **untouched**, and `Segmented` adds none. **A per-widget mode is
  still a setter and not a `Property`**, and `Segmented`'s choice set is a setter
  for exactly the reason `GaugeType` is.
- **No theme token, no `ThemeToken` variant, no `Theme` field, no scoped theme and
  no theme inheritance.** `TOKEN_COUNT` stays **33**, **`const`** and **private**.
  **Clause (a) of `L6b` is row `L9`'s**, and that row is not opened here: *"no tokens
  for focus, hover, pressed, shadow or z-order"* plus *"no theme scoping or
  inheritance"* is one piece of work and this task would have done half of it.
- **No automatic widget inference.** No widget decides its own mode from its own
  value, and no widget reads `hovered` / `pressed` / `disabled` off an ambient mode
  instead of off its own properties. **The case that would need it is named and not
  built**: composite row 4's *"the visualisation drag is a manual toggle **and** a
  persisted setting **and** an automatic FSD behaviour"* [B] is a **three-source
  precedence rule**, and `DEMO_APPLICATION.md` § *Could not verify* records that no
  source states what the precedence is. A rule invented here would be a parameter
  with no source, which is the failure § *Asset requirements* records for colours it
  could not verify.
- **No `TabBar` and no `Button::selected`** — gap `7`, `TASK_UI_PRIM_43`. The two
  climate `Button`s are ordinary buttons with ordinary labels, and the mode is
  written to a `Button::label`, not to a selected state.
- **No `Icon`, no tint uniform, no `widgets/icon.rs`** — gap `#4`,
  `TASK_UI_PRIM_44`. **Track Mode's recolouring of the car body is not built**, for
  the reason composite row 4's own note gives: its rules are `[B]` and
  *"not a spec the demo should copy until verified"*. A mode that reached a
  `DrawCommand::Mesh`'s tint would be a demo-side write on a task-40 field, and it
  is left for a task that has a verified rule to apply.
- **No screen-level mode, no page as a mode, no `nav::Screens`, no transition
  between modes** — gap `#3`, `TASK_UI_PRIM_42`. A mode here is resolved per **node**,
  and the root of a screen is a node like any other; nothing in this task knows what
  a screen is.
- **No shader-side mode.** No `u_mode` uniform, no per-fragment mode, no variant,
  no `#define` per mode. **A mode is a CPU-side value read at paint time**, and the
  only commands it reaches are the `RoundedRect` and `Text` commands
  `Segmented::paint` records.
- **No `LongPress`, `Swipe`, `Pinch`, `Drag` or `Scroll` consumption** — row `L4`,
  **whose own task is not written yet**: task 40's *Out of Scope* names
  `LongPress` and `Swipe` consumption as *"the next task's"* and declines both, and
  **the three consumers row `L4` names are all discrete threshold decisions rather
  than a control's answer.** `Segmented::on_event`'s `_ => false` names all seven
  variants and why, and the demo's strip adds **no gesture of any kind**.
- **No change to any pipeline mechanism**: no `DrawCommand` variant, no
  `ShaderKind`, no uniform, no `RenderCommand`, no batching key. **`git diff
    --stat` over `render.rs`, `paint.rs` and `batch.rs` shows nothing.**
- **No `LayoutMode::Grid`** — row `L3`, `TASK_UI_PRIM_52`. `Segmented` is **one
  node with one rect** that divides it into columns itself, in `Slider`'s shape: the
  demo places it absolutely and it has no children, no flex container and no grid.
  **No margin, no `flex-shrink`, no `flex-basis`, no cross-axis gap** — row `L7`.
- **No transform** — row `L2`, `TASK_UI_PRIM_36`.
- **No `Toggle::save` and no `Saveable` trait.** Two widgets carry `save`, which is
  one seam below `developer.md` § *Phase 2*'s threshold; a third caller is the point
  at which a trait would be earned, and the file names it rather than pre-building
  it.
- **No animated selection slide in `Segmented`, and no `tick`, no `Style`, no
  `Motion`, no `AnimationClock`.** Decided, not deferred: a clock with no transition
  is a method that returns `false` forever (task 40's recorded reason), and **no
  source in this repository states a duration for a segmented selection changing**.
  A later task adds `animate_to_state` and `tick` in `Motion::from_theme`'s shape,
  and the widget is built so that adding them changes no method's signature.
- **No drag on `Segmented` and no horizontal scrolling** — row `L5`.
- **The airflow-follows-the-fan coupling is not implemented.** The manual's
  *"Adjusting the fan speed may change the selected setting for how air is drawn
  into Model 3"* [A] **names an effect and not the rule that produces it**, so
  implementing it would be inventing one. The demo **restores** the airflow on the
  defroster's third touch, which *is* documented, and leaves the airflow alone on a
  fan adjustment, which is not documented at all. **That is why the airflow is
  captured and restored but never written by the fan** — and the capture order in
  requirement 12 is the one `Snapshot::restore` inverts, so the coupling has a place
  to land if a source ever states it.
- **No new page and no change to any of the six.** `Page::ALL` stays six,
  `Page::DEFAULT` stays `Pads`, the `--help` text is unchanged and no new `--tab=`
  name exists. The strip goes on **`controls`**, which is where a climate control
  belongs and where the free band is.
- **No `u_normal_matrix`, no wheel spin, no glass, no mesh change** — task 40's and
  task 37's *Out of Scope*, untouched.
- **No new dependency.** Per `AGENTS.md` the approved direct dependencies remain
  `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`. **A generic map, a
  `Vec<Callback<()>>` and a widget that divides a rect are three places a
  dependency would have been a licence decision against GPLv3 that nobody asked
  for**, and `AGENTS.md`'s rule is the operator's to relax, not this file's.
  **And no `unsafe` beyond what a GL call requires** — this task adds **zero** new
  `unsafe` blocks, because `ModeScope` is a `Vec`, `Snapshot` is a `Vec` of the
  crate's own `Callback<()>`, and `Segmented` contains no GL at all. **No `unwrap`, no `expect`,
  no `panic!`** in the three new files or in the new demo code; the one place a
  lookup can fail (`segment_at`'s `None`) is handled, not unwrapped, and
  `tab_handle`'s existing `panic!` in the demo's **test** code is not this task's
  and is not touched.