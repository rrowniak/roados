# TASK_UI_PRIM_42: `ui_core::nav::Screens` — a Table of Screens, a Back Stack, and the Gates

## Goal

Give `ui_core` the thing gap **#3** names — *"**No screen/navigation system** — no
screen stack, tab controller, or transition system in the library"* — in the
narrowest form that can be tested on this host: a **table of named screens, each
identified by a subtree root the caller owns**, a **back stack**, and **four
gates** that decide what is on show, what is hit, what `Tab` stops on, and whether
a node is a press target in its own right.

## Context

This is **gap #3** of `DEMO_APPLICATION.md` § *Library gaps*, row **3**, and the
operator's decision of **2026-10-05** — § *Operator decisions (2026-10-05)* item
2, *"**Every library gap must be closed.** The 2026-10-03 decision to build the
page mechanism in `ui_demo` and leave gaps #3 and #7 open in `ui_core` is
**withdrawn**"* — makes it mandatory library work. **This task is that work, and
it is the largest single blocker to a multi-screen application.**

**What the row itself says about what 24.1 delivered, and it is the sentence that
sets this task's shape:** *"What `TASK_UI_PRIM_24.1` built is the **demo-level**
mechanism — `enum Page`, a page-membership table, and three gates (paint, hit
test, focus), inside one binary — and **that was never this gap closed**."* So the
gates are the precedent and the deliverable; what is missing is the **library**
that carries them. **The row names three and this task delivers those three, plus
the fourth the demo's own tree turned out to need** — § *Four gates* says which
and why, and the fourth is the only one this file did not know it wanted before it
read `ui/src/ui_demo/src/main.rs`.

### What is in the crate at `75a896c`, established and not re-derived

| Fact | Where |
|---|---|
| **No navigation concept exists at all.** Zero identifiers `Screen`, `Route`, `Router`, `Navigator`, `NavStack`, `breadcrumb` anywhere in `ui_core/src`. | `ui_core`'s `lib.rs` publishes exactly `animation, arena, batch, font, input, layout, node, paint, property, render, texture, theme, widgets` |
| **`LayoutMode::Stack` is a layout mode, not a navigation stack.** Its doc reads *"Children overlap, all placed at the parent's origin"*; `arrange_stack` places each child at the parent's origin and **ignores `set_position`**, which is why task 24.2 made the gallery's root `LayoutMode::Absolute`. | `ui_core/src/layout.rs`, `LayoutMode`, `arrange_stack` |
| **A `Page` already exists and is a keyboard concept.** `pub enum Page { Letters, Symbols }` — the on-screen keyboard's two-page toggle, with `Default`, `Page::toggled`, a `label()` and its own tests. `KeyAction::PageUp` / `PageDown` are its callers' side of it. | `ui_core/src/widgets/keyboard.rs` |
| **The demo has a `Page` too, and the two already coexist.** **Amended 2026-10-09 by `TASK_UI_DEMO_01`, which added `Page::Demo`** — `enum Page { Pads, Text, Input, Controls, Data, Overlays, Demo }` with `const ALL: [Page; 7]`, `Page::name` (*"The only place the seven names are written out"*), `Page::from_name`, `Page::DEFAULT`. A fixed-size array, so **the type carries the count**. | `ui/src/ui_demo/src/main.rs` |
| **24.1's three gates are demo-local**, over one `Vec<PageMember>`: `Demo::shows`, `Demo::on_show`, `Demo::is_page_content` (paint and hit test), `Demo::focusables` (focus), `Demo::sync_page_visibility`, `Demo::empty_off_page_paint`. **A row is one node and one page**, so a subtree cannot be expressed: `text_panel` and the seven labels under it are eight rows. | `ui/src/ui_demo/src/main.rs` |
| **`hit_test` already skips an invisible node with its whole subtree**, top-down, children in reverse, `visible` read from `LayoutState`. **This is free: the hit-test gate needs one write per screen *root*, not one per member.** | `ui_core/src/input.rs`, `hit_test_from` |
| **`Focus` does *not* skip one.** `Focus::collect_focusable` walks the tree and pushes any handle in `focusable`, with **no `visible` test at all**; `Focus::current` returns whatever `self.current` holds. **So the focus gate does not exist in the crate.** | `ui_core/src/input.rs`, `collect_focusable`, `current` |
| **The paint gate has no owner either, and the renderer has no visibility test.** `Renderer::draw_node_clipped` returns early on a stale handle and on `!state.is_dirty()`, and on nothing else. `Demo::empty_off_page_paint`'s own doc records in four places that the pipeline keeps **no per-node command cache** and `begin_frame` clears the colour buffer and resets the batcher. | `ui_core/src/render.rs`, `draw_node_clipped`; the demo's `empty_off_page_paint` |
| **`LayoutState::visible`'s doc says *"only hit testing consults the flag"*** — a sentence this task makes **false**, and which therefore has to be amended. `set_visible` deliberately does **not** mark the node dirty: *"the layout inputs are unchanged, so the cached rect stays valid."* **So no rect moves when a screen is hidden, and that is a property the layout module already promised.** | `ui_core/src/layout.rs`, `visible`, `set_visible` |
| **8 `on_event` impls across 15 widget modules** (`button`, `dialog`, `keyboard`, `list`, `scroll`, `slider`, `text_input`, `toggle`); arms cover only `Tap`, `Drag`, `Scroll`, `KeyDown`, `Text`. **`LongPress` and `Swipe` are consumed by nothing.** | `ui_core/src/widgets/*.rs` |
| **`Callback<T> = Rc<dyn Fn(&T)>`**, so `Property::on_change` takes `Fn`; **`Property<T>`'s recompute closure is `Rc<dyn Fn()>`** and is handed no arena. `Transform` is `Interpolate`-able and **nothing reads it at draw time.** | `ui_core/src/property.rs`; gap **L2** |
| **`DrawCommand` has nine variants** — `Rect`, `RoundedRect`, `Shadow`, `Text`, `Image`, `Line`, `Circle`, `Path`, `Polygon` — and **only `Image` carries an `opacity: f32`**. None carries a transform or an animation. | `ui_core/src/paint.rs` |
| **`node::attach` refuses a child that already has a parent**, so re-parenting is `detach` then `attach`, and `can_attach` also refuses a cycle. | `ui_core/src/node.rs` |
| **Test baseline: 1894** — `ui_core` **1450**, `ui_demo` **224**, doctests **220**. Measured at `75a896c` plus the uncommitted diff, in the session that wrote this file. | `cargo test --all-features` from `ui/` |
| **The demo cannot receive a pointer event on this host.** XTEST pointer injection has never delivered one; keyboard delivered exactly one. **So no acceptance criterion here may require a pointer-driven interaction**, and none does. | `.ai/tools/README.md` § *Capturing a window* |
| **`.ai/tools/fps-check.sh` takes `seconds` then `floor` and runs the binary with no arguments**, so it **cannot name a page**; per-page is `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>`. Every run is measured — `task-sequence.md` § *Gates*, *"No unmeasured run of the demo."* | `.ai/tools/fps-check.sh` |

### The decision: a table of screens, each named by a subtree root the caller owns

**`ui_core::nav::Screens` — a registry of named screens, a back stack, and three
gates. A screen is a subtree, not a row.** Six reasons, and the second is the one
that makes every other decision follow.

1. **The gate is the proven precedent, and 24.1 built three of them.** What
   24.1 demonstrated is not that a page table works; it is that **a gate is a
   mechanism with no test until something asserts it**, and its review produced
   **four majors that were all that one finding**, each found by mutation rather
   than by reading: `raise_toast`'s table row (0 failed / 1811),
   `show_page`'s `sync_page_visibility` (0 failed / 1813), and `Demo::new`'s page
   table **having no completeness assertion at all** (one dropped row → 0 failed /
   1814, with the text column drawn on the wrong page). **This task's design rule
   is therefore its § *Testing*: every gate gets a named test, and every named
   test names the mutation it kills.**
2. **A screen is a subtree, and that is strictly less to get wrong than a table
   of rows.** 24.1 needs **~30 rows** to express six pages — **seven as of 2026-10-09**,
   because a row carries **one page per row** and so cannot say "these nine
   nodes are one thing". A subtree says it with **seven roots**, and then:
   - **the hit-test gate is free** — `hit_test_from` already skips an invisible
     node *with its whole subtree*, so one `set_visible` per screen root gates
     every member;
   - **the paint gate is one predicate** the caller's own walk asks;
   - **the focus gate is one fix** in `Focus`, after which the per-page
     focusable flag is **not needed at all** — which is a *deletion* in the demo,
     not a migration.
3. **It is testable with no display, no network, no filesystem and no wall
   clock**, which `AGENTS.md` permits and this host requires. `hit_test` and
   `Focus` are pure functions of an `Arena`; `Screens` holds only handles; the
   paint gate is a predicate. **The renderer was rejected as the paint gate's
   owner for exactly this reason** — see below.
4. **The arena stays the single owner of the nodes.** `Screens` stores `Handle`
   values and **never owns or borrows an `Arena`**. That is not tidiness: the
   demo's paint walk holds `borrow_mut()` on the arena for the whole walk, so a
   `Screens` that held a reference would not be *askable* from there. Holding
   only handles is what lets a caller ask `shows` in the middle of a mutating
   walk.
5. **`Property` is refused, with a specific reason rather than a preference.**
   `Screens` could be a `Property<Screens>` and be reactive like everything else
   in this crate — and it cannot be, because **`Property<T>`'s recompute closure
   is `Rc<dyn Fn()>` and receives nothing**: a screen change could not write
   `set_visible` from inside the graph, because the graph has no `Arena` to write
   it to. `Screens::sync` is the **only** writer of the flags the hit-test gate
   reads, and a flag nobody writes is not a gate. So `Screens` is a plain value
   the caller mutates and then **syncs**, and `is_dirty()` exists so a later
   transition can find out whether anything moved.
6. **The name is `Screen`, not `Page`, and nothing is renamed.** See
   § *The `Page` collision*.

**Where the gates live, and why not in the renderer.** The paint gate is
`Screens::shows`, asked by the caller's own walk. Putting it in
`Renderer::draw_node_clipped` would be one line of source and would delete the
demo's `empty_off_page_paint` outright — **and it is rejected, because a
mechanism in `render.rs` cannot be unit-tested on this host.** Every call into
`Renderer` needs a GL context, `AGENTS.md` forbids a test that needs a display,
and so a paint gate owned by the renderer would be **exactly the "a gate with no
test" shape 24.1's review produced four majors from**. The gate goes where it can
be killed by a mutation. **The missing renderer-side visibility test is recorded
as a finding for gap #5** — *"**Clipping has no owner** — … the clip is supplied by
the demo's frame loop … so there is no per-node clipping in the widget system"* —
and is named in § *Out of Scope* so it is a decision rather than an oversight.

### The `Page` collision: the library introduces no `Page`, and `keyboard::Page` is not renamed

**Decision: the navigation vocabulary is `Screen`, `Screens`, `ScreenId`. There is
no navigation `Page`, so there is nothing to collide with and nothing to rename.**

Four reasons, and the fourth settles it.

1. **Renaming a shipped `pub enum` is a public API change made for a naming
   preference.** `developer.md` § *API design* flags public API changes that break
   downstream users and requires them to be discussed. A downstream user writes
   `use ui_core::widgets::keyboard::Page;`; renaming it to `KeyboardPage` breaks
   them, and `KeyboardPage` is a *worse* name for a two-variant toggle than
   `Page`.
2. **The cost is six sites and a test migration, for zero user-visible gain.**
   `keyboard::Page` is reached through `Keyboard::page`, `Keyboard::set_page`,
   `Page::toggled`, `Page::label`, `Page: Default`, `KeyAction::PageUp` /
   `PageDown`'s doc comments, and the widget's own tests.
3. **The two senses are never in scope together**, so the ambiguity is theoretical:
   the keyboard's page is a *field of a widget* read as `keyboard.page()`, and
   the navigation value is read as `screens.current_name()`. Neither is imported
   bare.
4. **The collision already exists in this repository and does no harm.** Today
   `ui/src/ui_demo/src/main.rs` has a private `enum Page` with seven variants **and**
   `ui_core::widgets::keyboard` has a `pub enum Page` with two. Nothing is
   ambiguous, nothing is mis-resolved, and the demo drives its keyboard by key
   press through `offer_to` rather than by naming either type. **That is the
   proof: a module path is this repository's answer to a same-named type, and a
   rename is not.**

`Screens::add` **names** screens (`&'static str`), so the navigation value is
reachable by name for the same reason `Page::name` is: a screen a `--tab=` flag
can ask for needs a spelling, and the spelling is one string per screen.

### A back stack, and three decisions about it

**The history is the library's, not the caller's.** A caller that kept its own
`Vec<Page>` would hold the same fact twice — the hazard `GALLERY_SHORTCUTS` is
held to, and the rule is *two documents each claiming ownership of one
definition*. `Screens` is the only thing that knows what "the
screen before this one" is, because it knows the set. The demo's
`pending_page: Property<Option<Page>>` is a **pending request, not a history**,
and the two must never be confused: `pending_page` is written once per page and read
once, and `Screens` owns a `Vec<ScreenId>` that no property touches.

- **`switch` clears the history and records the outgoing screen; `push` appends.**
  One field, two operations with different meanings, and the reason is a real
  behaviour rather than a shape: **a tab bar pressed nine times must still
  switch.** If `switch` appended, the back stack would grow with every tab press
  and the depth limit would refuse the ninth tab — a defect a reviewer would find
  and a user would hit. So `switch` is *"leave this context"* and `push` is *"go
  one level deeper"*, and `pop` returns to whatever the history holds.
- **`switch` to the screen already on show is a total no-op** — not even a history
  clear. 24.3's review found the early return in `show_page` was load-bearing
  (`release_tab`'s `animate_to_state` held down by nothing when the button of the
  page already on show is pressed), so this file preserves it as a rule rather
  than re-deriving it.
- **`NAV_MAX_DEPTH = 8`**, and a `push` that would exceed it is **refused
  entirely** — `current` unchanged, history unchanged, `is_dirty` unchanged — not
  switched-without-history. A `push` that switched but did not record would leave
  a screen no `pop` can return from. The number is a first-principles choice and
  the doc comment says so: the deepest chain the manual describes is *one
  dismissal*, a panel over a card over a tray is three, and eight is comfortably
  above anything real while still bounding a leak. **A test asserts it is
  positive and that the table actually reaches it.**
- **`pop` at the root returns `false`, changes nothing, and leaves `can_pop`
  false.** `InputEventKind` has **no** `Back` — its nine variants carry no
  navigation event — so `pop`'s caller is the caller's own gesture or key, and
  this task builds neither. That is § *Out of Scope*.

**And the history is not a `LayoutMode::Stack`**, in the name or in the concept.
`grep -c LayoutMode ui/src/ui_core/src/nav.rs` returns **0** — the crate's layout
modes place children inside a box, and a back stack is a list of names.

### Transitions: none in this task, and the reason is arithmetic

**Decision: no transition. No slide, no fade, no cross-fade. The screen switch is
a cut, and the cut is already correct in this pipeline.**

1. **There is nothing to apply one with.** `DrawCommand`'s nine variants carry
   **no transform**, and gap **L2** records the whole of it: *"`Transform` is
   `Interpolate`-able so it can be animated, but `DrawCommand` has no transform
   field … and there is no matrix or `u_model` uniform"*, severity **Critical**,
   blocks *"Panel slide-ins, card paging"*. Gap **#8** is the same row in the
   first table, also **Critical**. **No task in 34–40 closes the 2D half**, and
   this task is not that task.
2. **A cross-fade via opacity alone is not the two-line change it looks like.**
   **Only `DrawCommand::Image` carries an `opacity: f32`.** The other eight would
   each need the field, a uniform, and a premultiplied-alpha answer in the
   fragment shader — and `Rect`, `RoundedRect` and `Text` are the *batched* paths,
   so it is a batching and shader change as well as a struct change. **That is a
   larger pipeline change than gap #8's own description implies**, and it would
   buy a 150 ms fade on a full-screen panel, which in an automotive interface is
   latency the driver pays for. **Not promised here, and not half-built here.**
3. **The cut needs no transition to be right, and this is the positive half.**
   `Renderer::begin_frame` clears the colour buffer **and** resets the batcher
   (`open` and `sealed`) every frame, and the renderer holds **no per-node command
   cache** — the demo's own `empty_off_page_paint` doc records this in four
   places. So **a switched screen leaves nothing of the previous one on the
   display**, and the switch is a hard cut with no residue. **A transition is
   therefore a pure addition**, and this task's API is shaped so that adding one
   changes no signature and introduces no type: `show`/`push`/`pop` set `current`
   and raise one flag; a later `Screens::tick(delta)` reads `is_dirty()`, knows
   the previous and current roots, and `sync` becomes a per-frame call rather than
   a per-switch one. **`Screens` deliberately exposes no "transition" concept and
   returns no `ScreenChange`, precisely so that nothing has to be added here for
   it to be added later.**
4. **No `AnimationClock`, no `tick`, no `animate_to`.** Each exists to drive a
   transition, and this type has none. `Screens` is a value plus an arena write,
   and a `tick` over an empty clock returns `false` for ever — the same argument
   `TASK_UI_PRIM_40` requirement 1 makes for `Rotator`'s absent `tick`.

### The demo's `enum Page` stays where it is, and becomes the vocabulary `Screens` speaks

**Decision: `Page` does not move into `ui_core`, is not renamed, and is not
replaced. `Page::ALL`, `Page::name`, `Page::from_name` and `Page::DEFAULT` are
untouched. What changes is that the demo's page *subtrees* — **seven as of
2026-10-09** — are registered with `Screens` under `Page::name()`, so
`Page::name` stays the only place the seven
names are written out.**

- **It stays in `ui_demo` because it is the CLI's vocabulary.** A seven-variant
  `Page` **— seven since 2026-10-09** — with seven lowercase spellings is what
  `--tab=`, `--help` and the
  unknown-name message resolve against — `DEMO_APPLICATION.md` § *What a seventh
  page costs* enumerates all three, and `TASK_UI_PRIM_24.md` was written so that
  adding one page touches a known list. **Moving it into `ui_core` would make the
  gallery's page count a fact of the library**, which `DEMO_APPLICATION.md`
  § *Relationship to task 24* contradicts: *"this application becomes one more
  tab"* — the gallery's page list is a **gallery** fact.
- **It is not replaced, because `Screens` is not a page list.** `Screens` knows
  names and roots; `Page` knows spellings and `--tab=`. One list, read by two
  mechanisms.
- **The demo keeps one mirror, and a named test says the mirror agrees.**
  `Demo::page: Page` stays, is written **only** by `show_page`, and
  `the_demo_page_field_and_the_librarys_current_screen_never_disagree` walks all
  seven pages and both switch paths. **Two representations of one fact is a
  hazard and the mitigation is a test**, which is what this
  repository already does for `Demo::frame_clips` (*"One function, used by the
  loop and by the tests, closes that"*). The alternative — rewriting ~30 test
  sites from `Page::X` to `ScreenId(3)` — would make the demo's own tests
  unreadable and spread the CLI vocabulary into the mechanism.
- **The demo's `Page` is not a library mechanism and was never claimed to be.**
  § *Library gaps* row 3 says so in its own words, and this task must not
  re-state it as if it did.

### Four gates, and every one of them is asserted

The gates are the deliverable, so they are named here with the test that holds
each. § *Testing* says which mutation each test kills.

| Gate | Mechanism | Who owns it |
|---|---|---|
| **Hit test** | `Screens::sync` writes `set_visible(false)` on every screen root but the one on show. **`hit_test_from` already skips an invisible node with its whole subtree**, so one write per root gates every member. | **`ui_core/src/input.rs` already owns it.** This task adds the caller and the test. |
| **Focus** | `Focus::collect_focusable` and `Focus::current` skip an invisible node's whole subtree, as `hit_test_from` already does. **Both are new.** | **`ui_core/src/input.rs`.** Six lines and three tests. |
| **Paint** | `Screens::shows(nodes, handle)` — *"is this node on show?"* — asked by the caller's own paint walk and draw walk. **In no screen is on show**, which is what holds the tab bar, the background, the frame-rate readout and the toast host up. | **`ui_core/src/nav.rs`**, by predicate; the *caller* does the walking, and `demo`'s `drawn_handles` is the one function both walks share. |
| **Whether a node is a hit-test target in its own right** | **`LayoutState::hits`, defaulting to `true`.** A node with `hits == false` is never *returned* by `hit_test` and never *offered* by `Focus`, **but its children still are.** A screen host sets it. | **`ui_core/src/layout.rs`**, and the two pure functions in `input.rs` that consult it. |

**The fourth row is the one that makes the third safe, and it was found by
reading the demo's tree rather than by designing it.** A screen host is a
**full-window box**, and *"A container
that covers the window swallows every tap aimed at anything behind it"* is this
repository's measured record of the failure mode. The fix was *"a second
hit test, after `input::route` had declined — a fallback rather than a bypass"*,
and the closing rule is *"where two mechanisms read one ordering in opposite
directions, **check that they can both be satisfied before moving anything**: here
the answer was that they could not, which is a fact the operator's brief did not
contain and only the measurement produced."*

**And the demo's own tree contains the measurement this file would otherwise have
had to guess.** On page `overlays` the only visible host is the overlays host, so
with hosts attached after `tab_bar` the reverse hit-test walk reaches that host
before the bar — **and a tap on a bar button would come back with the chain
`[overlays host, root]` instead of `[button, …, bar, root]`.** The bar would look
perfect and stop responding, **on exactly one of the seven pages**, and no rect test
and no capture would see it. Ordering cannot fix it: `hit_test_from` reads the
child list backwards and `Focus::focus_order` reads it forwards, and the entry
already records that those two cannot both be satisfied by moving anything.

**So the fix is a flag, not an order.** `LayoutState::hits` answers a question
`visible` does not: *is this node a target in its own right?* A screen host is a
grouping node — it draws nothing and is nothing to press — so `hits == false`
says so, and its children are still reached. The condition goes in **after** the
child loop in both functions, so descent is unchanged and only the node's own
return is suppressed. **`hits` defaults to `true`, so no existing node changes
behaviour and no existing test can see a difference** — which is what makes it a
safe addition rather than a semantic change to the crate's most load-bearing
function.

**`LayoutMode`, and the one thing the library cannot enforce.** A screen host
must be a node whose children keep the positions they declared, which means
**`LayoutMode::Absolute`** and not `LayoutMode::Stack`: `arrange_stack` places
every child at the parent's origin and `set_position` is a no-op there — the exact
trap task 24.2 recorded (*"`set_position` on a `Stack` child is a no-op"*, which is
why the gallery's root became `Absolute`). `Screens` **creates no node, sets no
mode and sets no flag**; the caller builds the root and requirement 7 says so in
the module doc, because a registry that also built trees would be a second owner
of the caller's shape — the same reason `Screens::add` does not attach.

### The demo's tree, and the one part of the migration that is not mechanical

**Six of the seven pages regroup by moving nodes. The seventh does not, and the reason
is the demo's two-root structure.** 24.1 registers `Page::Overlays` against
**three separate roots**: `dialog.handle()` — *"The dialog's node has no parent,
because a dialog is an overlay and the widget says so in its own tests"* —
`toasts.handle()`, and the toast cards under the host. **One screen is one
subtree root, so those three have to become one.**

**Decision: the `overlays` host takes both, as children in the order
`[dialog, toasts]`, and `Demo::order` becomes a single
`paint_order(&nodes, root.handle())` walk.** Three consequences, each with a
reason:

1. **The recorded order is unchanged.** `paint_order` is depth-first in child
   order, so a host with children `[dialog, toasts]` contributes the dialog's
   subtree then the toast cards — **which is exactly what today's
   `paint_order(gallery) ++ paint_order(dialog) ++ paint_order(toasts)`
   concatenation produces.** `the_dialog_is_drawn_above_the_two_toasts` is the
   named test that says so, by the positions of the two first commands in the
   recorded stream, **so it is not an eyeball decision**.
2. **`input::route` reaches the dialog for the first time, and the demo's manual
   fallback becomes dead code.** `Demo::route_input_event` builds its chain by
   hand today — it ends with `chain.push(self.dialog.handle())`, and it has a
   `Dialog::action_rect` branch above that — *because the dialog is not in the
   tree*. Both exist for exactly that reason and **both are deleted**, which is a
   second net deletion in this task and a strict improvement: **the dialog's
   actions now get their real z-precedence from the crate's own router instead of
   from a hand-written append.** **And modal precedence is untouched**, because
   `route_input_event`'s modal branch runs **before** anything else and
   `Focus::new(&nodes, self.dialog.handle())` still walks the dialog's own
   subtree as a root.
3. **The separate layout pass over the dialog's subtree is deleted.** `Demo::frame`
   calls `layout.layout(self.dialog.handle(), Constraints::tight(size))` as well
   as the pass over the gallery root, and **one pass from the root now reaches
   it** — with the host's origin added, which is `Offset::ZERO` on both. A second
   pass over the same subtree is a wasted walk, and the cache makes it a cheap
   one, which is why this is a deletion for clarity rather than for frames.

**This is the largest risk in 42.2 and it is named as one.** `Demo::order`'s
construction is changed by 24.1's standards, the dialog moves from a root to a
child, and `placed_handles`, `page_rects` and every assertion that indexes `order`
must be checked. **The three tests that carry it are named in requirement 23**, and
the capture is the fourth check — **which is why this page's AE is the one number
a reviewer should read rather than skim.**

### The split, counted honestly

The task as one unit is **seven distinct files** and **five components**, which is
**over both thresholds** in `developer.md` § *Scope check* (>5 files, >3
components). **So it is split into two sub-tasks**, per `.ai/protocols/subagents.md`
§ *Implementation fan-out*: **file-isolated, sequential, and each verifiable
alone.** § *Sub-task 42.1* and § *Sub-task 42.2* below carry the counts, and
42.2 **cannot be split further** because its two halves both edit
`ui/src/ui_demo/src/main.rs` and § *Splitting* rule 1 is explicit that two
subagents on one file is a failed split.

## Sub-task 42.1 — `ui_core::nav::Screens`, `LayoutState::hits`, and the two gate fixes in `input.rs`

**Five files, three components — at both thresholds and neither over.** Files:
`ui/src/ui_core/src/nav.rs` (new), `ui/src/ui_core/src/lib.rs` (one `pub mod`),
`ui/src/ui_core/src/input.rs` (`hit_test_from` and `Focus`), `ui/src/ui_core/src/layout.rs`
(the `hits` flag and its two doc amendments), `doc/ui/IMPLEMENTATION_STATE.md`
(the record). Components: `Screens` and the paint predicate; the
**hit-test-target** flag; the `Focus` gate.

**Acceptable alone: it is the whole of the library work, and its acceptance test
is `an_invisible_screen_root_and_its_whole_subtree_are_not_hit` plus
`focus_next_skips_a_subtree_whose_root_is_invisible`, both of which run on a
two-screen fixture with no consumer in this repository.**

**It must not amend `DEMO_APPLICATION.md`.** Gap #3 must not be marked closed by a
mechanism with no consumer — that is the failure § *Corrections to the second gap
table* was written about, and the row's own sentence about 24.1 is what it looks
like. **42.2 closes the row.**

## Sub-task 42.2 — the demo's pages become subtrees, and gap #3 closes

> **Amended 2026-10-09 by `TASK_UI_DEMO_01`, which added a seventh page.** Every
> count of the demo's **pages**, **page hosts**, **tab buttons** and **page
> names** in this sub-task is **seven**. **The phrase *six gallery pages* still
> means six** — the gallery is unchanged and `demo` is the seventh page beside
> it. **The test names that carry a count in the name are renamed** to match,
> because a task file written against six pages and implemented against seven
> cannot write them as named.

**Three files, two components.** Files: `ui/src/ui_demo/src/main.rs`,
`doc/ui/DEMO_APPLICATION.md`, `doc/ui/IMPLEMENTATION_STATE.md`. Components: the
tree regrouping and the three call sites; the two document amendments.

**A net deletion, and that is the headline.** `PageMember` — a 30-row table with
a per-row `focusable` flag, a type 24.1's doc explains at length — **is deleted**,
and so are `Demo::shows`, `Demo::on_show`, `Demo::is_page_content`,
`Demo::focusables`, `Demo::sync_page_visibility` and `Demo::empty_off_page_paint`.
**Six functions and one table go, and what replaces them is one predicate
(`Screens::shows`), one call (`Screens::sync`) and one list (`drawn_handles`).**
`Demo` gains a `screens: Screens` field and seven screen containers.

## Requirements

### Sub-task 42.1

1. **A new module `ui/src/ui_core/src/nav.rs`, declared `pub mod nav;` in
   `ui/src/ui_core/src/lib.rs`** — **alphabetically between `layout` and `node`**,
   which is where it sits in the published list
   (`animation, arena, batch, font, input, layout, **nav**, node, paint, property,
   render, texture, theme, widgets`). **A top-level module and not a sixteenth
   widget**, for a reason the gap row states and this file repeats: *"a
   demo-local struct cannot close a row of a table about `ui_core`"*, because a
   widget is a thing that paints and this is not one — `Screens` records no
   geometry, has no `on_event`, and never touches GL.

   ```rust
   /// One screen, as [`Screens`] knows it.
   #[derive(Clone, Debug)]
   struct Entry {
       /// The screen's name, and the spelling a caller resolves.
       name: &'static str,
       /// The subtree root the caller built and attached.
       root: Handle,
   }

   /// A table of screens, the one on show, and the stack behind it.
   ///
   /// `Debug` because a failed assertion prints the table and a name is worth
   /// reading there. **`Clone` is deliberately absent**: nothing in this task
   /// copies a `Screens`, and `developer.md` § Phase 2's *"No abstraction
   /// before the second use"* applies to a `derive` as much as to a trait.
   #[derive(Debug)]
   pub struct Screens {
       entries: Vec<Entry>,
       root: Handle,
       current: Option<ScreenId>,
       history: Vec<ScreenId>,
       dirty: bool,
   }
   ```

2. **`pub struct ScreenId(usize)`** with `#[derive(Clone, Copy, Debug, Eq, Hash,
   Ord, PartialEq, PartialOrd)]` and one accessor `#[must_use] pub fn index(self)
   -> usize`. **A newtype and not a bare index**, per `developer.md` § *API design*,
   and it is a *stable* one: **entries are append-only and this task provides no
   `remove`**, so an index never shifts. **The doc comment names the consequence a
   future `remove` would have to solve** — a free list, or a generational id like
   the arena's — so the stability is recorded as a decision rather than left as an
   accident.

3. **`pub const NAV_MAX_DEPTH: usize = 8;`**, `pub`, in the module's own block,
   doc-commented with § *A back stack*'s derivation and with **what exceeding it
   costs**. **A test asserts it is greater than zero and that the table reaches
   it** — the shape `TASK_UI_PRIM_40` requirement 2 gives for `ROTOR_PITCH_LIMIT`,
   whose test admits `16`, `24`, `32` and not `0`.

4. **`impl Screens`, fourteen methods and nothing else.** `reviewer.md` flags
   *"Overly public API"* and every name below is here because a named gate, a
   named test or a named caller in this task needs it.

   - `pub fn new(root: Handle) -> Self` — an **empty table with nothing on show**.
     **`new` takes no screen**, because the tree does not exist yet when the table
     is built and a name the table has not registered cannot be resolved.
     `Option<ScreenId>` for `current` is the honest answer and it removes a panic
     path that `new(first)` would have needed. **One doctest**, on a three-screen
     fixture: register three, `show` the middle, `sync`, and assert the flags.
   - `pub fn add(&mut self, name: &'static str, root: Handle) -> ScreenId` —
     **appends and returns the new id. It does not attach anything and it does
     not sync.** The caller builds the node and attaches it; a registry that also
     built trees would be a second owner of the demo's shape. **A name already
     present is refused**: the function returns the existing `ScreenId` **and
     changes nothing**, and the doc says which of the two a caller can tell apart
     by — nothing, which is why **`show` is what a caller resolves with** and why
     the refusal is a silent no-op rather than a second row. (`add` returning
     `ScreenId` rather than `Option<ScreenId>` is what makes a duplicate harmless:
     **the duplicate and the original are the same id, so a caller that ignores
     the return value has still not created a second screen.** The test asserts
     the length is unchanged.)
   - `#[must_use] pub fn len(&self) -> usize` — and **no `is_empty`**, matching
     `Arena`, which has `len` and `is_valid` and no `is_empty`.
   - `#[must_use] pub fn current(&self) -> Option<ScreenId>`
   - `#[must_use] pub fn current_name(&self) -> Option<&'static str>`
   - `#[must_use] pub fn current_root(&self) -> Option<Handle>`
   - `pub fn show(&mut self, name: &str) -> bool` — **resolves and switches.**
     `true` when the screen changed; `false` for an unregistered name, an id
     beyond the table, or a switch to the screen already on show — and **`false`
     in the first two leaves `current`, `history` and `dirty` untouched**, which
     the tests assert rather than assume.
   - `pub fn push(&mut self, name: &str) -> bool` — as § *A back stack*.
   - `pub fn pop(&mut self) -> bool`
   - `#[must_use] pub fn can_pop(&self) -> bool`
   - `#[must_use] pub fn history(&self) -> &[ScreenId]`
   - `#[must_use] pub fn is_dirty(&self) -> bool`

   **`push` is a thin wrapper over a private `push_id(&mut self, id: ScreenId) ->
   bool`,** and so is `show` over `show_id`. **Those two private helpers and
   `pop` are the only three places `current` is written**, which makes "one write
   path" a structural fact rather than a sentence — `TASK_UI_PRIM_40`'s § *Two
   producers, one value* discipline applied to `Screens`. `grep -c '    pub fn'
   ui/src/ui_core/src/nav.rs` returns **14**.

5. **`pub fn sync(&self, nodes: &mut Arena<WidgetNode>)` — the hit-test gate, and
   the only writer of a screen root's visibility.** One walk over `entries` and
   one `layout_mut().set_visible(entry_is_current)` per row. Three facts in the
   doc comment, all of them load-bearing and all of them already recorded
   elsewhere in the crate:
   - **`set_visible` deliberately does not mark the node dirty** — *"the layout
     inputs are unchanged, so the cached rect stays valid and the pass has nothing
     to recompute"* (`layout.rs`) — **so a screen switch moves no rect, and the
     layout pass is not re-run.** This is the crate's own promise, and it is why
     no test in this task asserts a rect changed.
   - **`hit_test_from` skips an invisible node with its whole subtree**, which is
     what makes one write per root sufficient rather than one per member.
   - **With nothing on show, every registered screen is hidden.** A node in no
     screen is not on any screen's account and `shows` says so.
   **A second `sync` is a no-op**, and a test asserts it.

6. **`#[must_use] pub fn shows(&self, nodes: &Arena<WidgetNode>, handle: Handle)
   -> bool` — the paint gate, and the predicate every caller asks.** It walks
   **up** from `handle` through `WidgetNode::parent` and answers `true` when any
   ancestor-or-self is the current screen's root, or when the walk reaches a node
   with no parent **and no screen claims the walk** — which is the always-painted
   set. Three facts in the doc:
   - **A node in no screen is on show.** That is what holds the tab bar, the
     background, the frame-rate readout and the toast host up without a second
     list anywhere, and it is 24.1's `is_page_content` argument.
   - **A node attached to nothing is on show** — the demo's dialog subtree is a
     second root by design.
   - **The walk is up, not down, and it is short**: two or three `get` calls here,
     against a subtree walk per frame. `shows` takes `&Arena` rather than
     `&mut Arena` so a caller can ask it in the middle of a `borrow_mut()`
     walk — the property § *The decision* point 4 is about.
   **There is no `contains`, no `is_page_content`, and no `members`** — no gate
     needs them, and `developer.md` § Phase 2's *"No abstraction before the second
     use"* refuses a question nothing asks.

7. **`nav.rs`'s module doc**, in the form `render/target.rs` and
   `widgets/keyboard.rs` carry: what `Screens` is; **what it is not** (no router,
   no routes, no paths, no URL, no nested or parallel screens, no transition, no
   `on_event`, no GL, no paint); **why a screen is a subtree and not a row**; the
   **four gates and who owns each**; **the `Page` decision and why nothing was
   renamed**; **the transition arithmetic** — nine variants, one `opacity`, gap
   `L2`, and why a cross-fade is a pipeline change rather than a widget one;
   **`LayoutMode::Absolute` is what a screen root must be**, with
   `arrange_stack`'s origin placement named; **a screen root is a hit-test region**,
   with the full-window-container tap-swallowing failure stated; **the renderer has no
   visibility test and that is gap #5's**; and **`LayoutMode` appears nowhere in
   the file**, with the acceptance criterion named so a later edit that imports it
   fails.

8. **`layout.rs`: `LayoutState::hits`, plus two doc amendments — three edits, none
   of which changes an existing node's behaviour.**

   - **A new field `hits: bool`, defaulting to `true`,** with
     `#[must_use] pub fn hits(&self) -> bool` and
     `pub fn set_hits(&mut self, hits: bool)`. **`set_hits` marks the node
     dirty**, unlike `set_visible` — because unlike visibility it *does* change
     an answer two passes read (a hit-test chain and a `Tab` order) rather than
     nothing. The doc comment states the question it answers, in one sentence, in
     the crate's register: **"whether this node is a hit-test target in its own
     right, as opposed to a grouping node whose children are."** `false` means
     *never returned as a target; children still reached*.
   - **`visible`'s doc no longer says *"only hit testing consults the flag"*,**
     because after this task it is consulted by three things: `hit_test`, the
     focus order, and a caller's own paint walk. **The amendment keeps the
     promise that a hidden node leaves its rect and its siblings' rects
     untouched** — *"the pass places every node it reaches, visible or not"* — and
     **adds the consequence the whole of this task rests on: a hidden node's
     whole subtree leaves the hit-test chain, the focus order and the paint
     gate.** **This is a documentation change and it is required**, because
     `DEMO_APPLICATION.md` § *Corrections to the second gap table* records **three
     instances of a doc comment asserting the opposite of the code beside it** —
     `render.rs`'s `set_scissor`, `list.rs`, and `scroll.rs` / `image.rs` twice.
   - **`set_visible`'s doc gains one sentence** naming the two consumers of a
     visibility change it does not itself perform — `hit_test` reads the flag
     directly, and `hit_test_from` is what skips the subtree.

9. **`input.rs`: three edits, and they are the two gates plus the flag's first
   consumer.**

   - **`hit_test_from` gains the flag's check, after the child loop.**
     `if !node.layout().hits() { return None; }` sits **between the `for` over
     `children` and the trailing `Some(handle)`**, so descent is unchanged and
     only the node's own return is suppressed. **One condition and one
     repositioned `return`, and the existing signature and every existing caller
     are untouched.** Its doc gains the same paragraph.
   - **`Focus::collect_focusable` skips an invisible subtree and skips a node that
     is not a target.** `if !node.layout().visible() { return; }` as the first
     statement — the same early return `hit_test_from` makes, for the same
     reason — then `hits` and membership in the push condition, then the
     recursion. **The symmetry with `hit_test_from` is the point and the doc says
     so**, because a `Focus` that skipped an invisible node but kept descending
     would be a third answer to a question two functions already answer twice.
   - **`Focus::current` returns `None` when the focused node has an invisible
     ancestor or is not itself a target.** Three lines: walk up with
     `self.arena.get(handle).and_then(WidgetNode::parent)` until a node that is
     not visible or not a target is met or the chain ends, mirroring `shows`.
     **Four doc amendments in the same file, and all four are required**: `Focus`'s
     docs say focus order is *"the tree's paint order … restricted to the nodes
     marked focusable"*, which is **false** after this change — they become
     *"the visible part of the tree's paint order, restricted to the nodes that
     are targets and marked focusable"* — and `Focus::set_focusable`'s docs gain
     the sentence that membership is necessary and **not sufficient**.
     **No existing test in `input.rs`'s test module combines `set_visible` or
     `hits` with `hit_test`'s target choice or `Focus`** — the only three
     `set_visible` calls there belong to one hit-test test — **so this is not a
     behaviour any assertion depends on, and the handoff says so rather than
     leaving the suite to discover it.**

10. **The tests, named, with no display, no network, no filesystem and no wall
    clock** — the only kind `AGENTS.md` permits. § *Testing* carries the mutation
    each one kills. In `nav.rs`'s `#[cfg(test)] mod tests`, on a
    `fn fixture() -> (Arena<WidgetNode>, Handle, Vec<ScreenId>)` that builds a
    root with three full-window screen subtrees of two nodes each:

    - `a_registered_screen_is_reachable_by_name_and_holds_the_root_it_was_given`
    - `a_name_registered_twice_is_refused_and_the_table_does_not_grow`
    - `a_name_that_was_never_registered_changes_nothing_current_history_or_flag`
    - `a_pop_with_nothing_on_the_stack_changes_nothing_and_leaves_the_flags_alone`
    - `a_push_records_the_screen_it_left_and_pop_returns_to_it`
    - `a_push_deeper_than_the_depth_limit_is_refused_whole`
    - `switching_clears_the_history_and_records_where_it_came_from`
    - `a_switch_to_the_screen_already_on_show_changes_nothing_at_all`
    - `a_pop_after_a_switch_returns_to_the_screen_the_switch_came_from`
    - `sync_hides_every_screen_but_the_one_on_show`
    - `sync_touches_nothing_but_the_screen_roots_and_is_idempotent`
    - `sync_with_nothing_on_show_hides_every_registered_screen`
    - `an_invisible_screen_root_and_its_whole_subtree_are_not_hit`
    - `the_screen_on_show_is_hit_all_the_way_down_to_its_deepest_leaf`
    - `a_node_that_is_not_a_hit_target_is_never_returned_and_its_children_still_are`
    - `a_node_that_is_not_a_hit_target_is_never_offered_to_the_focus_order`
    - `a_node_that_is_not_a_hit_target_holds_focus_only_while_it_is_one`
    - `shows_is_true_only_on_the_screen_on_show_and_on_no_screen_at_all`
    - `shows_follows_a_node_up_to_its_screen_root_and_agrees_with_hit_test`
    - `shows_is_true_for_a_node_attached_to_nothing`
    - `a_show_that_changes_nothing_leaves_the_dirty_flag_down`
    - `NAV_MAX_DEPTH_is_positive_and_the_table_reaches_it`

    In `input.rs`'s test module:

    - `focus_next_skips_a_subtree_whose_root_is_invisible`
    - `focus_reports_nothing_focused_inside_a_hidden_subtree`
    - `the_focus_order_is_unaffected_when_nothing_is_hidden` — the positive
      control, **so the first two cannot pass by hiding everything**, which is the
      mutation that would otherwise satisfy both.

    In `layout.rs`'s test module:

    - `hits_defaults_to_true_on_a_fresh_layout_state` — **in `layout.rs` and not in
      `nav.rs`**, because the thing under test is `LayoutState`'s default and a
      test of it belongs beside it. **One line, and it is the one that makes the
      addition safe**: a default of `false` would silently retarget every node in
      the crate and every caller outside it.

11. **`doc/ui/IMPLEMENTATION_STATE.md` gains one entry**, carrying: the type and
    the fourteen methods; **the subtree decision and the four gates with who owns
    each, including that `LayoutState::hits` is the fourth and was added because
    a full-window host would otherwise shadow the tab bar on the one page whose
    host is visible**; **the `Page` decision and that `keyboard::Page` was not
    renamed**; the **`NAV_MAX_DEPTH` and the switch-clears-history rule**; **the
    transition arithmetic** — nine `DrawCommand` variants, one `opacity`, gap
    `L2` — and **that no slide is promised**; **`LayoutMode::Absolute` for a
    screen host and why**; **`LayoutMode` appearing nowhere in `nav.rs`**; **the
    renderer has no visibility test and that is gap #5's**; **that no
    `DEMO_APPLICATION.md` row moved**; **the `hit_test_from` and `Focus` behaviour
    changes and that no existing test depended on them**; **and the frame rate of
    the unchanged demo, measured, with the script's own line pasted.**
    `IMPLEMENTATION_STATE.md` is not a source of evidence
    (`task-sequence.md` § *State*); it points at the code.

12. **The verification suite, produced.** From `ui/`: `cargo fmt --check`,
    `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings` clean,
    `cargo test --all-features` with the per-binary counts pasted and **no test
    deleted, renamed away or weakened**, `cargo doc --no-deps` clean, and
    `cargo audit` **recorded as not installed on this host, not passed**. Then
    **the frame rate**, because this task runs the demo and
    `task-sequence.md` § *Gates* forbids an unmeasured run.

### Sub-task 42.2

13. **`Demo` gains a `screens: Screens` field, and seven screen containers.**
    `Demo::new` builds them after the seven panels exist and **before the root's
    child list is assembled**:

    ```rust
    let mut screens = Screens::new(root.handle());
    for page in Page::ALL {
        let host = Container::new(&mut nodes, LayoutMode::Absolute);
        {
            // `Container` has no `set_constraints`, so the box is written on the
            // node — `ui_demo`'s own idiom, at the `controls` layer's
            // construction, and it keeps the `?` rather than an `unwrap`.
            let layer = nodes
                .get_mut(host.handle())
                .ok_or("ui_demo: a screen host is missing")?;
            layer.layout_mut().set_constraints(Constraints::tight(WINDOW));
            // **Optional, and written anyway.** `LayoutState::position` defaults
            // to `None` and `arrange_absolute` reads `.position().unwrap_or
            // (Offset::ZERO)`, so a host with no declared position already lands
            // at the origin — which is why three of the four existing root
            // children set none. **Setting it makes this requirement's second
            // condition checkable by reading the code instead of by knowing the
            // default.**
            layer.layout_mut().set_position(Some(Offset::ZERO));
            // **The one that makes the tab bar work on every page.** A
            // full-window host is a hit-test region by default and would shadow
            // `tab_bar` on the one page whose host is visible; see § *The demo's
            // tree*.
            layer.layout_mut().set_hits(false);
        }
        screens.add(page.name(), host.handle());
    }
    ```

    - **Each host's layout mode is `LayoutMode::Absolute`, its position is
      `Offset::ZERO` and its declared box is `WINDOW`.** All three are load-bearing
      and each has a reason in the comment: `Absolute` because `arrange_stack`
      ignores `set_position` (the trap 24.2 recorded); `(0, 0)` because
      `Layout::visit` adds the parent's origin to every placement and the
      gallery root's own origin is `Offset::ZERO`; `WINDOW` because
      `arrange_absolute` sizes a child with `sized(nodes, handle, loose)` and
      `Constraints::loosen` keeps the maxima, so a `tight(WINDOW)` box resolves to
      `WINDOW` under the host exactly as it did under the root.
    - **Each host sets `hits(false)`, and this is what makes the child order
      irrelevant to hit testing.** With it, `hit_test_from` descends into a host,
      offers its children, and returns `None` for the host itself — so the tab bar
      is reached on **every** page whatever the order, and the full-window-container
      failure cannot recur. **The flag is still required rather than
      left at its default, and the doc comment says why in one sentence:** a
      grouping node that draws nothing is not something a reader can press.
    - **`Screens::add` does not attach.** `Demo::new` attaches each host to
      `self.root` **immediately after `tab_bar`** in the existing child list, and
      moves each page's top-level nodes into it with `node::detach` then
      `node::attach` — **`attach` refuses a child that already has a parent, so
      the order of those two calls is not interchangeable.**
    - **The root's child list is `background, tab_bar, <the seven hosts>, controls`**
      — `text_panel` moves into the `text` host and `controls` keeps only the
      frame-rate readout, which is requirement 8's *"Keep fps label"* and the
      operator's instruction. **`tab_bar` is second for `Focus::focus_order`,
      which walks children forward**, and
      `tab_walks_the_seven_buttons_before_the_pages_own_controls` pins that.
      **Its position relative to the hosts no longer matters for hit testing**
      because of `hits(false)`, **and the tests say both halves**: that one pins
      the focus order, and `the_tab_bar_is_still_first_in_the_route_chain_for_each
      _of_its_seven_buttons` pins the route on **all seven pages**, not on one.
    - **`text_panel`'s own child, `text_column`, stays where it is**, under
      `text_panel`. Only top-level nodes move.
    - **The `overlays` host also takes `dialog.handle()` and `toasts.handle()`**, in
      that order, and **the two separate root walks are deleted** — § *The demo's
      tree* carries all three consequences and all three reasons.
    - **A host records no commands and has no entry in `Demo::order`** other than
      through the one walk. It paints nothing, `Layout::visit` reaches it from the
      tree whatever `order` holds, and `Demo::draw` sends `drawn_handles()` — so no
      host is ever submitted and no batch changes.
    - `Demo::new` ends with **`screens.show(page.name())` and
      `screens.sync(&mut nodes)` before it returns**, preserving the property
      `Demo::new`'s own doc states: *"Three gates are closed in the constructor
      rather than on the first switch, and the hit-test gate is closed … before
      this returns, so the first frame hit-tests against the page the run asked
      for too."* **A first frame on this host is a capture**, which is the whole
      reason that sentence exists.

14. **`Demo::show_page` calls `screens.show(page.name())` and
    `screens.sync(&mut nodes)`,** replacing `sync_page_visibility`. **Its
    early-return for a page already on show stays**, because 24.3's review found
    it load-bearing (`release_tab`'s `animate_to_state` held down by nothing when
    the button of the page on show is pressed). **`Demo::page: Page` stays, is
    written only here, and `every_page_is_a_member_of_exactly_one_screen` plus
    `the_demo_page_field_and_the_librarys_current_screen_never_disagree` pin it.**

15. **`Demo::drawn_handles(&self) -> Vec<Handle>` is new, and it is the one place
    the paint gate is asked.** It is `self.order` filtered by
    `self.screens.shows(&self.nodes.borrow(), handle)` — **and it is what both
    `Demo::frame`'s paint walk and `Demo::draw` iterate**, so the two sets cannot
    disagree. *A test of a helper cannot see a call site that
    stopped using it* is the rule: **one function, used by the loop and by the
    tests.**

16. **`Demo::frame_clips` takes `&[Handle]`** — `fn frame_clips(&self, handles:
    &[Handle]) -> Vec<Option<Rect>>`, returning one entry per **drawn** handle.
    Its doc's *"This is the whole of the frame's clipping, in one place"* stays
    true and is now true of the drawn set; **its existing tests keep their names
    and pass `&demo.drawn_handles()`**, and the handoff lists every one it
    touched. **The signature changes because the thing it is positional over
    changed**, which is the same honest consequence `frame_clips`'s own doc
    records when its rects were removed.

17. **Deleted, by name: `PageMember`, `Demo::shows`, `Demo::on_show`,
    `Demo::is_page_content`, `Demo::focusables`, `Demo::sync_page_visibility`,
    `Demo::empty_off_page_paint`, and the `page_members` field and its builder
    closure;** plus, from § *The demo's tree*, **`Demo::route_input_event`'s
    trailing `chain.push(self.dialog.handle())` and its `Dialog::action_rect`
    branch**, and **the separate `layout.layout(self.dialog.handle(), …)` pass in
    `Demo::frame`.** **The last three are deleted only because
    `every_dialog_button_is_in_the_route_chain_without_the_manual_fallback` and
    `every_page_places_every_rect_where_the_gallery_placed_it` are green**, and
    the handoff pastes both — **a fallback that is still needed is a defect, and a
    fallback left in place because nothing proved it unreachable is the dead code
    `developer.md` § *Code quality* refuses.** **No dead code and no
    `#[allow(dead_code)]`.**

18. **`Demo::focus_navigation` marks the seven tab buttons and the five controls
    focusable, and stops there.** The per-page focusable flag is gone because
    **`Focus` now skips a hidden subtree**, so a node inside another page is not
    in the order and needs no flag to keep it out. `Demo::tab_focusables` stays —
    the seven buttons are on every page and `Screens` cannot put them anywhere —
    and `tests::always_painted_handles` stays as the written-out list of the
    twelve nodes that are in no screen, which is now **asserted rather than
    described** by `the_always_painted_set_is_in_no_screen`.

19. **`Demo::focus_navigation`'s modal branch is unchanged** — `Focus::new(&nodes,
    self.dialog.handle())` when the dialog is modal, so the walk never reaches the
    gallery and the bar is absent from it rather than de-prioritised. **The
    dialog's own subtree is in no screen**, so `shows` says it is on show and the
    scrim is unaffected.

20. **The stale source comment on the tab bar is corrected.**
    `ui/src/ui_demo/src/main.rs`'s comment on the tab bar reads *"`DEMO_APPLICATION.md`
    gaps #3 and #7 stay open and this is what gap #7 prescribes"*, and
    `DEMO_APPLICATION.md` § *Relationship to task 24* lists it as the fourth of
    **four places still recording the withdrawn 2026-10-03 decision**, **known-stale
    and "left for a code task"**. **This is that code task.** The comment is
    rewritten to name § *Operator decisions (2026-10-05)* item 2 and to say that
    gap #3 is closed in `ui_core` by this task's `Screens` **while gap #7 remains
    open** — because this task does not close #7, and a comment that implies it
    did would be the same defect the other three places are.

21. **`DEMO_APPLICATION.md` § *Library gaps*, row 3, gains a dated note** in the
    form § *Corrections to the second gap table* prescribes — **not a deletion and
    not a bare "closed"**:

    - **`TASK_UI_PRIM_42` closes the row's claim about `ui_core`**:
      `ui_core::nav::Screens` is a table of named screens with a back stack and
      four gates, and the demo's seven pages are seven subtrees registered in it.
    - **What is delivered is a screen stack and a visibility gate, and the row's
      third noun — *"or transition system"* — is NOT delivered.** Gap **#8** and
      gap **L2** stay **Critical** and keep every entry in their `Blocks` column,
      and the note says why in one sentence: nine `DrawCommand` variants, one
      `opacity`, no transform, no `u_model`.
    - **One consumer exists.** The gallery is it. **The row is not closed on the
      strength of the library type alone**, and the note says which of the two
      facts the row needed.
    - **The row's own sentence about 24.1 is corrected in place rather than left
      to age**: what `TASK_UI_PRIM_24.1` built was the demo-level mechanism and
      **was never this gap closed**, which is now true as well as accurate.
    - **The four places § *Relationship to task 24* lists as still recording the
      withdrawn decision are updated**: requirement 20 closes the fourth (the
      source comment), and the other three are `TASK_UI_PRIM_24.md` § *Out of
      Scope*, `TASK_UI_PRIM_24.3.md` § *Context* and § *Out of Scope*, and
      `IMPLEMENTATION_STATE.md` — **all three amended here by this task**, and the
      table's own claim that *"none of them is amended by this pass"* is
      superseded in the same edit, because a table that says nothing is amended
      while three rows are amended is a table that is wrong.

22. **`doc/ui/IMPLEMENTATION_STATE.md` gains one entry** carrying: the
    regrouping and the seven hosts; **`Demo::page` kept as a vocabulary mirror and
    the named test that pins it**; **the seven deleted items by name**; the
    child-order constraint and the failure mode it answers; the
    `PageMember` → `Screens` completeness tests; **the seven pages' frame rates**;
    and **the honest limits** — the demo still has **no transition**, the
    renderer still has **no per-node visibility test**, and the demo's `Page` is
    **still not a library mechanism**.

23. **The demo tests, named, with no display, no network, no filesystem and no
    wall clock.** Kept, unchanged in name and in every assertion:

    - `every_page_places_every_rect_where_the_gallery_placed_it` — **the pixel-identity
      mechanism at the rect level, and it is the test the whole regrouping rests
      on.** Requirement 13's three conditions are what make it pass; if they do
      not, this test says so rather than the capture.
    - `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`,
      `placed_handles`, `page_rects`
    - `tab_walks_the_seven_buttons_before_the_pages_own_controls` — now also
      evidence that `Focus`'s gate holds in the demo.
    - `always_painted_handles` (the helper), `nothing_the_demo_places_reaches_into_the_strip`

    New, each with its mutation in § *Testing*:

    - `a_page_switch_hides_the_five_other_screens_and_shows_this_one` — **all
      four gates on one switch**, read through the crate's own mechanisms.
    - **`the_tab_bar_is_still_first_in_the_route_chain_for_each_of_its_seven_buttons`**
      — `input::route(&nodes, root, &tap)` printed for a press on each of the seven,
      **on each of the seven pages** rather than on one, asserting the button is on
      the chain. **This is the test a reviewer should break first**, and it is the
      two-line diagnosis that failure prescribes. **Running
      it on all seven pages is the point**: the failure it guards against appears on
      **one** page, the one whose host is the only visible one.
    - `every_pages_clickable_controls_are_still_in_the_route_chain` — the three
      pads, the slider, the toggle and the text field, each offered a press and
      each on the chain.
    - `every_dialog_button_is_in_the_route_chain_without_the_manual_fallback` — the
      deletion condition for requirement 17's last two items.
    - `the_dialog_is_drawn_above_the_two_toasts` — the recorded-stream half of the
      `[dialog, toasts]` order, by command position.
    - `every_page_is_a_member_of_exactly_one_screen`
    - `no_node_is_a_member_of_two_screens` — split from the above **so the
      one-row mutation is caught by a test whose name is about it.**
    - `the_always_painted_set_is_in_no_screen` — the twelve names of
      `always_painted_handles` are in no screen, so `shows` is true for every one
      on every page.
    - `the_nodes_drawn_on_a_page_are_its_own_nodes_and_the_always_painted_set`
    - `no_node_of_another_page_is_ever_drawn`
    - `the_demo_page_field_and_the_librarys_current_screen_never_disagree`
    - `no_screen_host_is_a_hit_test_target` — the `hits(false)` gate, on the
      demo's own seven hosts: each is invisible to `hit_test_from` as a target while
      its children are reached.

    **Replaced, by name, and recorded as replacements rather than deletions:**
    `every_page_lists_at_least_one_node_and_no_node_is_on_two_pages` is superseded
    by `every_page_is_a_member_of_exactly_one_screen` and
    `no_node_is_a_member_of_two_screens`; the test of `empty_off_page_paint`
    ("the recorded vector is empty") is superseded by
    `no_node_of_another_page_is_ever_drawn`. **The old second assertion was a
    proxy for the gate, and the gate is what is now tested** — a proxy is what
    24.1's review recorded as the trap, *"`PaintState::new()` would leave the
    renderer holding the batch it had already submitted"*, a claim its own comment
    later showed was **false in this pipeline**. **No other test is deleted,
    renamed away or weakened, and the handoff lists every test this requirement
    touches.**

24. **The suite, the capture and the frame rate are all produced.** From `ui/`:
    the seven commands of requirement 12. Then **the seven-page before/after
    capture**, with the commands of `.ai/tools/README.md` § *Capturing a window* verbatim: window id
    **re-read at the time of each capture** with `xwininfo -root -tree` (a root
    capture, and `ffmpeg x11grab`, return black for a GL window), `pgrep -a -x
    ui_demo` in the same call as each `magick import -window <id>`, then `magick
    compare -metric AE before.png after.png null:` per page. Then **the frame rate
    on all seven pages**.

## Testing

**The lesson this file is built around, and it is 24.1's:** its review produced
**four majors that were all one finding — a mechanism with no test** — and **every
one was found by mutation rather than by reading**. One dropped row in
`Demo::new`'s page table gave **0 failed / 1814** with the text column drawn on
the wrong page; `show_page`'s `sync_page_visibility` gave **0 failed / 1813**.
So every gate below has a test, and every test names **the mutation it kills**.

**And the second lesson, from the same review's successors:** *"A test of a
helper cannot see a call site that stopped using it."* That is why
`Demo::drawn_handles` exists as one function used by both walks, and why the
route-chain tests exercise `input::route` — the real mechanism — rather than a
predicate beside it.

| Test | Mutation it kills |
|---|---|
| `sync_hides_every_screen_but_the_one_on_show` | `sync` writing nothing; `sync` writing `true` for every root; `sync` writing only the outgoing screen and leaving the incoming one hidden. **Each fails on a different screen, which is why the fixture has three.** |
| `sync_touches_nothing_but_the_screen_roots_and_is_idempotent` | `sync` toggling, or writing a node it was not given. Kills a `sync` whose second call differs from its first — the cheapest defect to ship and the hardest to see. |
| `sync_with_nothing_on_show_hides_every_registered_screen` | `sync` treating "no current" as "show everything". |
| **`an_invisible_screen_root_and_its_whole_subtree_are_not_hit`** | **Removing the `sync` call.** This is the test that makes the gate *matter*: `sync`'s own unit test passes with `sync` never called, and only this one fails. **This is the pair a gate needs.** |
| `the_screen_on_show_is_hit_all_the_way_down_to_its_deepest_leaf` | `sync` over-hiding. **Without this positive control the test above passes for a `sync` that hides everything, which is a blank window.** |
| `shows_is_true_only_on_the_screen_on_show_and_on_no_screen_at_all` | `shows` losing the "in no screen" branch — **which would hide the tab bar, the background, the readout and the toast host**, and no rect assertion would see it. |
| `shows_follows_a_node_up_to_its_screen_root_and_agrees_with_hit_test` | `shows` checking only the node itself and not its ancestors, which is right for a member and wrong for a root's child. **It asserts agreement with `hit_test` on the same point, so the two gates cannot diverge.** |
| `shows_is_true_for_a_node_attached_to_nothing` | `shows` reaching a parentless node and answering `false` — which would take the demo's modal dialog off screen. |
| `a_node_that_is_not_a_hit_target_is_never_returned_and_its_children_still_are` | `hits` defaulting to `false`, or the check landing **before** the child loop. **The before-the-loop mutation is the interesting one: it would hide every host's entire subtree from hit testing, so nothing in a page would be reachable** — and the test's second half is what catches it, which is why the test asserts *both* that the node is never returned and that its children still are |
| `a_node_that_is_not_a_hit_target_is_never_offered_to_the_focus_order` | `hits` not consulted by `collect_focusable` — the host would become a `Tab` stop, and a `Tab` stop where nothing lights up is the defect `PageMember::focusable`'s own doc calls a *"stop a reader cannot see"* |
| `a_node_that_is_not_a_hit_target_holds_focus_only_while_it_is_one` | `Focus::current` not consulting the flag. |
| `hits_defaults_to_true_on_a_fresh_layout_state` | A default of `false`, which would **silently retarget every existing node in the crate and every caller outside it.** One line, and it is the one that makes the addition safe |
| `focus_next_skips_a_subtree_whose_root_is_invisible` | Reverting `collect_focusable`. **`Tab` would then stop on a control on a page that is not on show.** |
| `focus_reports_nothing_focused_inside_a_hidden_subtree` | Reverting `current` alone, with `collect_focusable` fixed. |
| `the_focus_order_is_unaffected_when_nothing_is_hidden` | The "hide everything" mutation that satisfies the two above. |
| `a_show_that_changes_nothing_leaves_the_dirty_flag_down` | `is_dirty` always `true`, which makes every future transition re-run a full sync per frame for nothing. |
| `a_switch_to_the_screen_already_on_show_changes_nothing_at_all` | `show` clearing the history or raising the flag on a no-op — **the mutation that would break 24.3's `release_tab` early return**, in the library this time. |
| `a_push_deeper_than_the_depth_limit_is_refused_whole` | A `push` that switches without recording, which leaves a screen no `pop` can return from. |
| `a_pop_with_nothing_on_the_stack_changes_nothing_and_leaves_the_flags_alone` | `pop` at the root wrapping the index and hiding a screen that was not on show. |
| `a_name_registered_twice_is_refused_and_the_table_does_not_grow` | `add` appending a duplicate — **the same mutation class as `Demo::new`'s dropped row, one level down.** |
| `a_name_that_was_never_registered_changes_nothing_current_history_or_flag` | `show` returning `true` for a name it did not resolve, which is 24.1's recorded *"a repeated `--tab=` silently discards an unknown name"* defect reappearing one layer down. |
| **`the_tab_bar_is_still_first_in_the_route_chain_for_each_of_its_seven_buttons`** | **Dropping one host's `set_hits(false)`**, or moving a host after `tab_bar` without the flag — reproduces the 2026-10-05 failure: the bar becomes unclickable **on the one page whose host is the only visible one**, seven handlers are wired, and nothing else in the suite sees it. **The mutation appears on `overlays` alone**, which is why the test loops all seven pages |
| `every_pages_clickable_controls_are_still_in_the_route_chain` | The same move's other half: a control inside a host that a full-window sibling shadows. |
| `no_screen_host_is_a_hit_test_target` | One host left at `hits(true)`. **Checked on the demo's seven hosts and not on a fixture**, because the defect is about *these* boxes covering *this* window. |
| `every_dialog_button_is_in_the_route_chain_without_the_manual_fallback` | The dialog left outside the tree, **or** the manual `chain.push(self.dialog.handle())` deleted while the dialog is still outside it — which would make the dialog's buttons unreachable and is the failure mode this test exists to gate the deletion with |
| `the_dialog_is_drawn_above_the_two_toasts` | Swapping the overlays host's children to `[toasts, dialog]`, which would put the two notifications **over** the modal scrim. **A z-order change with no error and no failing rect assertion** — the capture is the second check and the named test is the first |
| `every_page_is_a_member_of_exactly_one_screen` / `no_node_is_a_member_of_two_screens` | **24.1's surviving mutation**, reproduced on the table this task replaces. **Split into two tests so each name is about what it catches.** |
| `the_always_painted_set_is_in_no_screen` | One of the twelve being registered on a page — which would make it disappear on the other five, and **no rect assertion would see it** because the node still has a rect. |
| `the_nodes_drawn_on_a_page_are_its_own_nodes_and_the_always_painted_set` / `no_node_of_another_page_is_ever_drawn` | `drawn_handles` forgetting its filter, or filtering on `Page` instead of `screens.shows`. |
| `the_demo_page_field_and_the_librarys_current_screen_never_disagree` | Either half of the mirror being written without the other. **This is the whole mitigation for keeping two representations of one fact, so it is a gate and not a nicety.** |
| `every_page_places_every_rect_where_the_gallery_placed_it` | Any of requirement 13's three conditions failing. **Keeps its name and every assertion, and it is the mechanism the pixel criterion rests on.** |

## Acceptance Criteria

- [ ] **`Screens` exists, is `ui_core::nav::Screens`, and `lib.rs` publishes
      `nav` between `layout` and `node`.** `grep -n 'pub mod nav' ui/src/ui_core/src/lib.rs`
      shows the line, and the published list reads
      `animation, arena, batch, font, input, layout, nav, node, paint, property, render, texture, theme, widgets`.
      `awk '/^pub struct Screens/,/^}/' ui/src/ui_core/src/nav.rs` shows **`entries`,
      `root`, `current`, `history`, `dirty`** and nothing else, and
      `grep -c '    pub fn' ui/src/ui_core/src/nav.rs` returns **15** —
      **fourteen on `impl Screens`** and `ScreenId`'s one `index` accessor — with
      **three private helpers**, `show_id`, `push_id` and `pop`, between them the
      only writers of `current`

- [ ] **`LayoutMode` appears nowhere in `nav.rs`, and the module doc says why.**
      `grep -c LayoutMode ui/src/ui_core/src/nav.rs` returns **0** outside that
      doc paragraph — **and the acceptance criterion quotes the paragraph**, so a
      later edit that imports it has to delete the sentence that says it is
      forbidden. **`grep -n 'LayoutMode::Stack' ui/src/ui_core/src/nav.rs` returns
      the one line of prose that says the history is not one.** **The mechanism is
      named because the confusion is real and not hypothetical: `LayoutMode::Stack`
      is a layout mode whose children overlap at the parent's origin, and reusing
      the name for a back stack is the mistake this criterion makes unmakeable**

- [ ] **The `Page` collision is decided by *not* colliding.**
      `grep -c 'pub enum Page' ui/src/ui_core/src/nav.rs` returns **0** —
      **the library introduces no navigation `Page`** — and
      `git diff --stat` shows **`ui/src/ui_core/src/widgets/keyboard.rs`
      unchanged**, with `pub enum Page { Letters, Symbols }`, `Default`,
      `Page::toggled`, `Page::label` and the widget's own tests all still there.
      `nav.rs`'s module doc carries the four reasons, **the fourth being that
      `ui/src/ui_demo/src/main.rs` already has a `Page` and `ui_core::widgets::keyboard`
      already has a `Page` and nothing is ambiguous** — the coexistence as the proof

- [ ] **The four gates each have a named test that demonstrably fails if the gate
      is       removed**, and the handoff carries the mutation output for all four:
      - **hit test** — `an_invisible_screen_root_and_its_whole_subtree_are_not_hit`
        fails when the `sync` call in its fixture is deleted. **Mutation
        evidence in the handoff:** delete the `sync` call, watch it fail with the
        hidden screen's leaf still on the chain, restore it, watch it pass.
        **The test that proves the gate has an effect, which `sync`'s own unit
        test cannot**
      - **focus** — `focus_next_skips_a_subtree_whose_root_is_invisible` fails when
        `collect_focusable`'s `visible` early-return is removed. **Mutation
        evidence:** remove the two lines, watch it fail with a `Tab` stop on a
        hidden control, restore, pass
      - **paint** — `shows_is_true_only_on_the_screen_on_show_and_on_no_screen_at_all`
        fails when `shows` loses either branch. **Mutation evidence:** drop the
        "in no screen" branch, watch it fail on the always-painted node, restore,
        pass
      **and each of the first three has a positive control** —
        `the_screen_on_show_is_hit_all_the_way_down_to_its_deepest_leaf` and
        `the_focus_order_is_unaffected_when_nothing_is_hidden` — **so none of them
        can pass by hiding everything**
      - **target gate** — `a_node_that_is_not_a_hit_target_is_never_returned_and
        _its_children_still_are` fails when the `hits` check is moved before the
        child loop or when the flag's default becomes `false`. **Mutation evidence
        in the handoff:** move the check above the `for`, watch it fail with no
        page control reachable, restore, pass; then flip the default to `false` and
        watch `hits_defaults_to_true_on_a_fresh_layout_state` fail

- [ ] **`Focus` skips a hidden subtree and reports nothing focused inside one.**
      `the_focus_order_is_unaffected_when_nothing_is_hidden`,
      `focus_next_skips_a_subtree_whose_root_is_invisible` and
      `focus_reports_nothing_focused_inside_a_hidden_subtree` are present and
      green. **`Focus`'s own doc comments are amended** — *"restricted to the nodes
      marked focusable"* becomes *"the visible part of the tree's paint order …"* —
      **because the old sentence is false after this change**, and
      `git diff --stat ui/src/ui_core/src/input.rs` shows the amendment beside the
      two-line gate rather than instead of it. **The handoff states that no
      existing test in `input.rs` combined `set_visible` with `Focus`** — the only
      three `set_visible` calls in that module's test module belong to one
      hit-test test — **so no assertion depended on the old behaviour**

- [ ] **`LayoutState::hits` exists, defaults to `true`, and is consulted by both
      readers.** `hits_defaults_to_true_on_a_fresh_layout_state` is present and
      green — **it is the criterion that makes the addition safe rather than a
      silent retarget of every node in the crate.** `grep -n 'hits()' ui/src/ui_core/src/input.rs`
      returns **exactly two** call sites, `hit_test_from` and `collect_focusable`,
      **and the `hit_test_from` one is after the `for` over `children`** — a check
      before the loop would hide every host's whole subtree from hit testing and
      nothing in the window would be pressable.
      `a_node_that_is_not_a_hit_target_is_never_returned_and_its_children_still_are`
      asserts **both halves**, `never returned` and `children still are`.
      **`git diff --stat` shows no signature change** to `hit_test`, `hit_test_from`,
      `route`, `Focus::new` or any other existing item, and no existing test moved

- [ ] **`LayoutState::visible`'s doc no longer says only hit testing consults the
      flag.** `grep -n 'only hit testing' ui/src/ui_core/src/layout.rs` returns
      **nothing**, and the amendment names **three** consumers — `hit_test`, the
      focus order, and a caller's own paint walk — while keeping the promise that
      **hiding a node leaves its rect and its siblings' rects untouched** and that
      **`set_visible` does not mark the node dirty**. **This criterion is
      documentation, and it is required because `DEMO_APPLICATION.md`
      § *Corrections to the second gap table* records three doc comments asserting
      the opposite of the code beside them**

- [ ] **`Screens::sync` is the only writer of a screen root's visibility, and it
      moves no rect.** `grep -n 'set_visible' ui/src/ui_core/src/nav.rs` returns
      **exactly the one line inside `sync`**, and
      `grep -n 'set_visible' ui/src/ui_demo/src/main.rs` returns **no new line** —
      the demo's call is gone with `sync_page_visibility`.
      `sync_touches_nothing_but_the_screen_roots_and_is_idempotent` and
      `every_page_places_every_rect_where_the_gallery_placed_it` both hold.
      **The mechanism is the crate's own:** `set_visible` *"does not mark the node
      dirty: the layout inputs are unchanged, so the cached rect stays valid"*, so
      a switch re-runs no layout pass

- [ ] **Back-stack semantics are the ones this file decides, each with its test.**
      `NAV_MAX_DEPTH == 8` and `NAV_MAX_DEPTH_is_positive_and_the_table_reaches_it`
      holds. `a_push_records_the_screen_it_left_and_pop_returns_to_it`,
      `a_push_deeper_than_the_depth_limit_is_refused_whole`,
      `switching_clears_the_history_and_records_where_it_came_from`,
      `a_switch_to_the_screen_already_on_show_changes_nothing_at_all`,
      `a_pop_with_nothing_on_the_stack_changes_nothing_and_leaves_the_flags_alone`
      and `a_pop_after_a_switch_returns_to_the_screen_the_switch_came_from` are
      all present. **`switching_clears_the_history_and_records_where_it_came_from`
      is the one a reviewer should read twice: it exists because a `switch` that
      appended would make the ninth press of a tab bar fail, which is a user-facing
      defect with no error anywhere.** **The history is the library's —
      `grep -n 'history' ui/src/ui_demo/src/main.rs` returns no write and no read
      outside a test, and `pending_page` is still a `Property<Option<Page>>` that
      `Screens` does not touch**

- [ ] **No transition, no slide, and the arithmetic is on the record.**
      `git diff --stat` shows **`ui/src/ui_core/src/paint.rs`, `render.rs`,
      `batch.rs`, `property.rs`, `animation.rs` and `layout.rs`'s arrange paths
      unchanged**, and **no `DrawCommand` variant gains a field** — `grep -n
      'opacity' ui/src/ui_core/src/paint.rs` still returns exactly one field, on
      `Image`. `grep -c 'u_model\|mat4' ui/src/ui_core/src/nav.rs` returns **0**.
      **`Screens` has no `tick`, no `animate_to` and no `AnimationClock`**, and the
      module doc states the four facts: nine variants, one `opacity`, gap `L2` is
      Critical and is not this task, and **the cut needs no transition to be
      correct** because `begin_frame` clears the colour buffer and the renderer
      holds no per-node command cache. **Gap #8 and gap L2 keep Critical and every
      entry in their `Blocks` column**, and the handoff says in one sentence what
      a later transition would add: `Screens::tick` reading `is_dirty()` and
      `sync` becoming per-frame — **no signature change and no new type**

- [ ] **The demo's `Page` stays, and the seven names still have one home.**
      `git diff --stat` shows **`enum Page`, `Page::ALL`, `Page::name`,
      `Page::from_name` and `Page::DEFAULT` unchanged** — including
      `const ALL: [Page; 6]`, whose type still carries the count, because
      `DEMO_APPLICATION.md` § *What a seventh page costs* enumerates everything a
      seventh page touches and **this task must not touch any of it**.
      `Page::name`'s doc still reads *"The only place the seven names are written
      out"*, **and `grep -n 'page.name()' ui/src/ui_demo/src/main.rs` shows the seven
      `Screens::add` calls reading it** — so the registry and the CLI resolve
      through one list. **`Screens` gains no `Page`, and `ui_core` gains no string
      registry for one consumer.** `the_demo_page_field_and_the_librarys_current_screen_never_disagree`
      is present and green, **which is the whole mitigation for keeping a
      vocabulary mirror**

- [ ] **The demo's deletions happened and nothing is dead.**
      `grep -c 'PageMember\|fn sync_page_visibility\|fn empty_off_page_paint\|fn
      is_page_content\|fn on_show\|fn focusables' ui/src/ui_demo/src/main.rs`
      returns **0**, `grep -c 'allow(dead_code)' ui/src/ui_demo/src/main.rs` returns
      **0**, and `cargo clippy --all-targets --all-features -- -D warnings` is
      clean. **What replaced them is named**: `Screens::shows` at the seven
      `*_at` predicates, `Screens::sync` in `show_page`, `LayoutState::hits` on the
      seven hosts, and `Demo::drawn_handles` in both walks.
      **Ten items are deleted in total — the seven above plus
      `route_input_event`'s trailing `chain.push(self.dialog.handle())`, its
      `Dialog::action_rect` branch, and `Demo::frame`'s separate
      `layout.layout(self.dialog.handle(), …)` pass — and the handoff names the test
      that licensed each of the last three**

- [ ] **`Demo::drawn_handles` is the one place the paint gate is asked, and both
      walks use it.** `grep -n 'drawn_handles' ui/src/ui_demo/src/main.rs` shows
      **one definition and two call sites** — `Demo::frame`'s paint walk and
      `Demo::draw` — plus the tests. **The mechanism is that a test
      of a helper cannot see a call site that stopped using it, so the helper and
      the call sites are the same object.**
      `the_nodes_drawn_on_a_page_are_its_own_nodes_and_the_always_painted_set`
      and `no_node_of_another_page_is_ever_drawn` are present and green.
      **The renderer still has no visibility test**, and the handoff says so in
      one sentence and names it as gap #5's

- [ ] **The tab bar and every page's own controls are still in the route chain, on
      every page.** **`the_tab_bar_is_still_first_in_the_route_chain_for_each_of_its
      _seven_buttons`** calls `input::route(&nodes, root, &tap)` for a press on each
      of the seven **on each of the seven pages** and asserts the button is on the
      chain — the two-line diagnosis that failure
      prescribes, **and the test a reviewer should break first.**
      **The seven-page loop is not thoroughness, it is the whole test**: dropping one
      host's `set_hits(false)` shadows the bar on **`overlays` alone**, because that
      is the only page whose host is visible, and a bar that looks perfect and stops
      responding on one page of seven is invisible to every other check in this file.
      `every_pages_clickable_controls_are_still_in_the_route_chain` does the same
      for the three pads, the slider, the toggle and the text field, and
      `no_screen_host_is_a_hit_test_target` asserts the flag on **the demo's seven
      hosts rather than on a fixture**, because the defect is about *these* boxes
      covering *this* window.
      **And `tab_walks_the_seven_buttons_before_the_pages_own_controls` keeps its
      name and every assertion**, which pins the other reader: `Focus::focus_order`
      walks children **forward**, so `tab_bar` is second and the seven buttons lead
      the order

- [ ] **The `overlays` page's three former roots are one subtree, and the recorded
      order is unchanged.** `grep -n 'paint_order' ui/src/ui_demo/src/main.rs`
      shows **one call**, `paint_order(&nodes, root.handle())`, where it showed
      three concatenated walks; `grep -n 'layout.layout(self.dialog.handle()'`
      returns **nothing**; and `route_input_event` no longer ends with
      `chain.push(self.dialog.handle())` nor carries a `Dialog::action_rect`
      branch. **The three tests that carry it are green and pasted in the handoff:**
      `the_dialog_is_drawn_above_the_two_toasts` (**the `[dialog, toasts]` order, by
      command position in the recorded stream — a z-order swap puts the two
      notifications over the modal scrim and no rect assertion sees it**),
      `every_dialog_button_is_in_the_route_chain_without_the_manual_fallback`
      (**the deletion condition for those three items**), and
      `every_page_places_every_rect_where_the_gallery_placed_it`.
      **Modal precedence is unchanged and the handoff says why in one sentence:**
      `route_input_event`'s modal branch runs before anything else and
      `Focus::new(&nodes, self.dialog.handle())` still walks the dialog's subtree
      as a root

- [ ] **`cargo test --all-features` is green against the 1894 baseline**, and the
      handoff **lists every new test by name** — the 22 in `nav.rs`, the 3 in
      `input.rs`, the 1 in `layout.rs`, and the demo's 12 new plus the 2 recorded
      replacements. Per binary: `ui_core` **1450 + 26 = 1476** or more, `ui_demo`
      **224 + 12 = 236** or more, doctests **220 + 2 = 222** or more.
      **No test was deleted, renamed away or weakened**, and the **two
      replacements are recorded as replacements with their reasons** —
      `every_page_lists_at_least_one_node_and_no_node_is_on_two_pages` superseded
      by `every_page_is_a_member_of_exactly_one_screen` and
      `no_node_is_a_member_of_two_screens`, and `empty_off_page_paint`'s test
      superseded by `no_node_of_another_page_is_ever_drawn`, **because the old
      assertion was a proxy for the gate and the gate is what is now tested**.
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. **`cargo audit` is not installed on this host;
      that is recorded, not passed.** `ui/Cargo.toml` and `ui/Cargo.lock` are
      unchanged — per `AGENTS.md` the approved direct dependencies remain
      `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, **and `Screens` needs
      none**: a `Vec`, an `Option`, a `bool` and `&'static str` are the whole of it

- [ ] **The gallery pages are pixel-identical outside `y ≥ 680`, and the
      mechanism is stated rather than hoped for.** `Page::ALL`'s seven names —
      **six gallery pages plus `demo`, which is the seventh and is captured like
      the rest** — release
      build, captured **before and after** with the commands of
      `.ai/tools/README.md` § *Capturing a window* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab`, return black
      for a GL window), `pgrep -a -x ui_demo` in the same call as each
      `magick import -window <id>`, then `magick compare -metric AE before.png
      after.png null:` per page. **On all seven pages the criterion is AE 0 outside
      `y ≥ 680`**, every differing pixel inside the frame-rate readout's band,
      which `.ai/tools/README.md` § *Capturing a window* records as the one thing two captures of an unchanged frame differ in.

      **The mechanism is four facts, and it is the first task in this sequence
      whose criterion is not restated:**
      1. **No rect moves.** Each screen host is `LayoutMode::Absolute` at
         `Offset::ZERO` with a `tight(WINDOW)` box, and `Layout::visit` adds the
         parent's origin to every placement while the gallery root's origin is
         `Offset::ZERO` — so a child's rect under a host is arithmetically the
         rect it had under the root. **The dialog is the one that moves into a
         tree it was not in**, so its own placement is called out separately: its
         origin was `Offset::ZERO` under `layout.layout(dialog.handle(),
         Constraints::tight(size))` and is `ZERO + ZERO` under the host, and its
         declared box is unchanged either way.
      2. **No command is added, removed or reordered.** A host records nothing, and
         `Demo::draw` sends `drawn_handles()`, which on a given page is the page's
         own nodes plus the always-painted set — **the same set it sent before this
         task.** The one walk that replaces three concatenates them in the order
         `[dialog, toasts]`, which is the order the concatenation produced, and
         `the_dialog_is_drawn_above_the_two_toasts` says so by command position.
      3. **The rect-level tests keep their names and every assertion**:
         `every_page_places_every_rect_where_the_gallery_placed_it`, plus
         `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete`,
         `placed_handles` and `page_rects`. **Criterion 1's proof is a test, not
         the capture**, which is the point: a capture cannot tell a rect that moved
         by a pixel on all seven pages from one that did not, and
         `every_page_places_every_rect_where_the_gallery_placed_it` compares the
         seven pages with each other, so a change common to all seven passes it.
      4. **What did change is invisible to a capture by construction**, because it
         is entirely about which nodes are *offered input*: which are hit, which
         `Tab` stops on, and which the paint and draw walks iterate. **A screen
         switch produces no pixel change and that is the correct outcome**, so
         the criterion is the strong one rather than a restatement with a wider
         band.

- [ ] **The frame rate is measured on all seven pages and reported**, with the
      script's own line pasted rather than the number expected:
      `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of the
      seven with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `.ai/tools/README.md` § *Frame-rate baseline* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. Every page above the floor of **55** and **inside the recorded
      61.1–63.9 band** in `.ai/tools/README.md` § *Frame-rate baseline*.

      **And the handoff states what the number is expected to be and why, rather
      than reporting a number and letting it be read as luck: this task's per-frame
      cost is a second `Arena` walk the frame already does.** `drawn_handles()`
      asks `shows` once per node in `order` — about 40 nodes, each walking up two
      or three parents — **and it replaces `empty_off_page_paint`'s whole-arena
      sweep with nothing at all**, because that sweep existed only to empty paint
      states the draw walk no longer submits. **So the expected result is no
      change on any page, and a page outside the band is a finding rather than
      noise**, because the work removed should have cost less than the work it
      replaced

- [ ] **What the handoff does not claim, in those words.** It states that
      **no pointer event has ever been observed reaching this window** on this
      host — `.ai/tools/README.md` § *Capturing a window* records the drag, the two presses on task 12's button, the
      counter and the `AE = 0`, and `XQueryPointer` reporting window `0x0` — **and
      therefore that no acceptance criterion here is verified by a pointer-driven
      capture, and none asks for one.** The gates are verified **by test through
      the crate's own mechanisms** — `input::hit_test`, `input::Focus`,
      `Screens::shows` — **and the tab bar's clickability is verified by
      `input::route` chains, which is the same claim a press would make without a
      press.** The screen switch itself is verified **by capture, by its absence of
      change**. **The two together are not evidence that a finger switches a
      screen**, and the `IMPLEMENTATION_STATE.md` entry says so

- [ ] **Nothing from another task leaked in.** `grep -c 'pub fn on_event'
      ui/src/ui_core/src/` still returns **9** and `grep -c 'pub mod'
      ui/src/ui_core/src/widgets/mod.rs` still returns **15** — this task adds a
      module and no widget. **`LongPress` and `Swipe` are still consumed by
      nothing**, so row `L4` of `DEMO_APPLICATION.md` § *Gaps this layout exposes
      in `ui_core`* remains true and this task does not touch it.
      **`LayoutMode::Grid` is still unimplemented** — `grep -n 'LayoutMode::Grid'
      ui/src/ui_core/src/layout.rs` still shows `=> Vec::new()` and `columns` still
      never read — **and this task depends on neither `Grid` nor `Flex.wrap`**,
      because every screen host is `LayoutMode::Absolute` and the demo adds no
      child inside one. **No new dependency** (`ui/Cargo.toml` and `ui/Cargo.lock`
      unchanged), **no `unsafe`** — `grep -c unsafe ui/src/ui_core/src/nav.rs` is
      **0**, and `input.rs` and `layout.rs` gain none — **no `unwrap`, no
      `expect`, no `panic!`, no `unimplemented!`** in production code, and nothing
      newer than `rust-version = "1.85"`'s stdlib

- [ ] **The demo's tab-bar comment no longer records a withdrawn decision.**
      `grep -n 'gaps #3 and #7 stay open' ui/src/ui_demo/src/main.rs` returns
      **nothing**, and the comment cites `DEMO_APPLICATION.md`
      § *Operator decisions (2026-10-05)* item 2 by name and says **gap #3 is
      closed in `ui_core` while gap #7 remains open** — **because this task does
      not close #7**, and a comment implying it did would be the defect the other
      three places in that table are

- [ ] **`DEMO_APPLICATION.md` § *Library gaps* row 3 is amended, dated, and says
      exactly what is and is not closed.** It names `TASK_UI_PRIM_42` and
      `ui_core::nav::Screens`, states that **the row's claim about `ui_core` is
      closed** and that **the row's third noun — *"or transition system"* — is
      NOT delivered**, with gap **#8** and gap **L2** keeping Critical severity and
      every entry in their `Blocks` columns. **The row's sentence about
      `TASK_UI_PRIM_24.1` is corrected in place**, because it is now true as well
      as accurate: 24.1 built the demo-level mechanism and **was never this gap
      closed**. **The row is not deleted** — it records what a library mechanism
      is and what a demo affordance is not, and those are different sentences.
      **And § *Relationship to task 24*'s four-place table is amended**: three of
      its four rows are updated here and its own claim that *"none of them is
      amended by this pass"* is **superseded in the same edit**, because a table
      that says nothing is amended while three rows are is a table that is wrong

- [ ] **The split was honoured.** The handoff for 42.1 and the handoff for 42.2
      are separate, 42.2 was briefed against the now-existing `Screens`
      (§ *Parallel or sequential*: sequential, because 42.2 reads a type 42.1
      creates), and **the review covers the integrated result** — a sub-task
      reviewed alone and then integrated is reviewed again, because integration
      can break what the parts proved. **42.2 was not split further**, and the
      reason is stated rather than asserted: its two halves both edit
      `ui/src/ui_demo/src/main.rs`, and `.ai/protocols/subagents.md` § *Splitting*
      rule 1 is explicit that two subagents on one file is a failed split

- [ ] **The decisions are written down where the next agent finds them.**
      `nav.rs`'s module doc carries requirement 7's content in full;
      `nav.rs`, `input.rs`, `layout.rs`, `ui/src/ui_demo/src/main.rs`,
      `doc/ui/DEMO_APPLICATION.md` and `doc/ui/IMPLEMENTATION_STATE.md` each carry
      their half of requirements 11 and 22, **including the `Page` decision, the
      `NAV_MAX_DEPTH` and the switch-clears-history rule, the transition
      arithmetic, `LayoutMode::Absolute` for a screen root and why, the hit-test
      region, the renderer having no visibility test, and the honest limits** —
      **and no doc comment in any changed file asserts the opposite of the code
      beside it**, which is the defect `DEMO_APPLICATION.md`
      § *Corrections to the second gap table* records three times

## Out of Scope

- **No change to the demo's modal behaviour, its scrim, its z-order against the
  toasts, or its two-root *reason*.** The dialog moves from a root to a child
  because one screen is one subtree root and the `overlays` page owns three
  separate roots — § *The demo's tree* carries the three consequences.
  **What does not change is what the dialog does**: `route_input_event`'s modal
  branch still runs before anything else, `Focus::new(&nodes,
  self.dialog.handle())` still walks its own subtree as a root, `Dialog::is_drawn`
  is still not what modality is keyed on, and `the_dialog_is_drawn_above_the_two
  _toasts` holds. **The two-root *structure* is what goes**, and it goes because a
  subtree is the unit a screen is

- **No `TabBar` widget, and no change to the demo's tab bar's appearance.** That is
  gap **#7** and `TASK_UI_PRIM_43`, and `DEMO_APPLICATION.md` § *Library gaps*
  row 7 still records that **the "active-state indication" half has no widget
  behind it either** — `Button` carries `hovered`, `pressed`, `disabled`,
  `focused` and `activatable` **and no `selected`**, so the active tab's
  `background`/`foreground` swap is the demo's. **This task hands 43 a
  `Screens::show` to call and nothing else**, and 43's seven buttons will read the
  active screen from `Screens::current_name()`. **The bar's geometry, order and
  palette are untouched, and so is `Demo::tabs`**

- **No `Grid` layout, and no dependence on it.** `LayoutMode::Grid { .. }` is
  `Vec::new()` and `columns` is never read — gap **L2**'s sibling **L3**,
  **Critical**, separate task. **Every screen host is `LayoutMode::Absolute`**, so
  this task needs no grid, adds no child inside a host, and does not depend on
  `Flex.wrap` being honoured either

- **No back gesture, and no `InputEventKind` variant consumed or added.**
  `InputEventKind` has nine variants and **none of them is a back, an escape or a
  dismiss** — `pop`'s caller is the caller's own gesture or key, and this task
  builds neither. **So `pop` and `can_pop` have no consumer in this repository**,
  and that is stated rather than left for the next agent to find out.
  **No `LongPress` and no `Swipe`**, so row `L4` stays open and unamended

- **No nested screens, no parallel screens, no routes, and no URL or deep-link
  concept.** One table, one screen on show, one stack. **A screen root that is
  inside another screen's subtree is a caller error**, and the module doc says so
  without offering to detect it — the walk in `shows` is up from the node, so a
  nested root answers for its own screen and the outer one becomes unreachable,
  which is a fact a reader can derive in one step and a test would only make
  look supported. **`Screens` is not a router, and the file says so in its first
  line**

- **No 2D transform transitions — no slide, no fade, no cross-fade, no
  `AnimationClock`, no `tick`, no `animate_to`.** Decided in § *Transitions* on
  arithmetic, not deferred: **nine `DrawCommand` variants, one `opacity`, no
  transform, no `u_model`**, and gaps **#8** / **L2** are **Critical** pipeline
  work. **The cross-fade is refused by name rather than offered as a smaller
  thing to do later**, because adding an `opacity` field to the other eight
  variants is a batching and shader change and not a widget one. **What is
  delivered is a cut, and the cut is already correct** — `begin_frame` clears the
  colour buffer and the renderer holds no per-node command cache

- **No renderer-side visibility test, and no per-node visibility ownership.**
  `Renderer::draw_node_clipped` early-returns on a stale handle and on
  `!is_dirty()` **and on nothing else**, so a caller whose paint walk does not ask
  `shows` will draw nodes nothing can reach. **This is rejected here rather than
  done, and the reason is testability:** a mechanism inside `render.rs` needs a GL
  context, `AGENTS.md` forbids a test that needs a display, and **a gate that
  cannot be killed by a mutation is the failure 24.1's review produced four
  majors from.** **It is gap #5's work — *"the clip is supplied by the demo's
  frame loop … so there is no per-node clipping in the widget system"* — and this
  file names it as such rather than leaving a reader to find the hole**

- **No change to any pipeline mechanism.** `DrawCommand`'s nine variants,
  `PaintState`, `Batcher`, `BatchKey`, `Segment`, `Renderer::begin_frame`,
  `MeshVertex` — **none touched.** `DEPTH_BITS`, the depth policy, `Mat4` and
  both projections, the mesh shader sources, the loader and the byte format are
  **all untouched by this task and checkable by `git diff --stat`** — and note
  that **34–40 are specified and not started**, so this task depends on **none** of
  them

- **No change to `LayoutMode`, `Padding`, `FlexConfig`, `Constraints`,
  `Layout` or `Layout`'s cache.** **`layout.rs`'s only edits are the `hits` flag,
  its one test, and three doc paragraphs** (requirements 8 and 9), and the promise
  that a hidden node keeps its cached rect is *used*, not changed. `Padding` is a
  four-sided inset, there is **no margin, no `flex-shrink`, no `flex-basis` and no
  cross-axis gap** in the crate (gap **L7**), and this task needed none of them.
  **No existing `LayoutState` field changes meaning and no existing signature
  changes**, which is what makes `hits_defaults_to_true_on_a_fresh_layout_state`
  the criterion that carries the addition

- **No change to `Property`, `Callback`, `Transform`, `Interpolate`,
  `AnimationClock` or `Easing`.** **`Screens` is deliberately not a
  `Property`**, on the recorded reason that a property's recompute closure is
  `Rc<dyn Fn()>` and receives no arena — **so a screen change could not write the
  flag the hit-test gate reads.** `is_dirty` is the seam a future transition uses,
  **and it is one `bool`, not a type** — which is why a transition can be added
  without changing a signature

- **No new dependency, and no `unsafe`.** Per `AGENTS.md` the approved direct
  dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`; a
  routing crate, a state-machine crate or a transition crate is a licence decision
  against GPLv3 that nobody has asked for, **and `Screens` needs none** — a `Vec`,
  an `Option`, a `bool`, a `usize` and `&'static str` are the whole of it.
  `grep -c unsafe ui/src/ui_core/src/nav.rs` is **0**, and `input.rs` and
  `layout.rs` gain none

- **No removal of a screen, and no `Screens::remove`.** Entries are append-only
  so a `ScreenId` is a stable index, and a future `remove` needs a free list or a
  generational id like the arena's — **a design question this task does not
  answer and the module doc names as the consequence of this one**

- **No acceptance criterion is waived, and none asks for an instrument this host
  cannot produce.** No criterion here requires a pointer event, a GL readback, a
  display, a network, a filesystem or the wall clock; the two things a capture
  cannot see — a rect that moved on all seven pages, and a frame-cost regression —
  are covered by `every_page_places_every_rect_where_the_gallery_placed_it` and by
  `fps-check.sh` respectively, which is the pairing
  *a still screenshot of a 4 fps application looks exactly like a 60 fps one*
  exists to demand
