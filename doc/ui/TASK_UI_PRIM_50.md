# TASK_UI_PRIM_50: A Theme That Scopes — `FocusRing`, `scope::ThemeScope`, and What a Switch Does Inside a Scope

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Close **row `L9`** of `DEMO_APPLICATION.md` § *Gaps this layout exposes in
`ui_core`* — *"**No theme scoping or inheritance.** One flat global token map;
no tokens for focus, hover, pressed, shadow or z-order — focus rings and
hover/pressed are per-widget properties instead"* — in two halves that are
deliberately unequal:

- **one new token**, `ThemeToken::FocusRing`, and
- **a new type**, `scope::ThemeScope`, a set of per-node token overrides that a
  subtree resolves through and the rest of the tree never sees.

**The token count goes from 33 to 34, and six of the seven interaction-state
tokens `L6b`(a) names are declined by argument rather than delivered.** That is
the decision this file exists to make, and § *The token decision* makes it with
the crate's own recorded reasons rather than with taste.

This is also the task `TASK_UI_PRIM_47` deferred to by name: its § *The theme
gains no token, and `L9` is why* hands clause **(a)** of row `L6b` here, and its
§ *Out of Scope* says *"Clause (a) of `L6b` is row `L9`'s, and that row is not
opened here."* **This file opens it.**

## Context

### What row `L9` is, and what it blocks

Row **`L9`** reads, in full: *"**No theme scoping or inheritance.** One flat
global token map; **no tokens for focus, hover, pressed, shadow or z-order** —
focus rings and hover/pressed are per-widget properties instead."* Its severity
is **Medium** and its **Blocks** column names *"Any subtree that needs to differ
from the global theme"*.

**Two clauses, and they are not the same size.** The Blocks column is the first
clause — **scoping** — and it is the one with a real consumer waiting. The second
clause is an **inventory**, and an inventory is only true if the things it lists
are genuinely absent *as a concept*. This file establishes, for each of the seven,
whether the absence is a defect or a position the crate has already taken in
writing. **Six of the seven have an argument already written in the code that would
have consumed them**, and this task records those arguments rather than reopening
them.

Row **`L6b`** clause **(a)** is the same inventory one clause over — *"no mode
dimension in the theme — `ThemeToken` is a flat 33-variant enum with no `Focus`,
`Hover`, `Pressed`, `Shadow`, `ZOrder`, `Active` or `Selected` variant, so a mode
cannot restyle a subtree"* — and **`TASK_UI_PRIM_47` handed it here by name**
(its § *What `L6b`'s three claims are*, clause **(a)**). **The "so" is the half
that matters**: a mode cannot restyle a subtree **because there is no scoping**,
and scoping is the first clause. So clause (a) is answered by *scoping*, not by
seven tokens.

### What exists at `HEAD` (`75a896c` plus the uncommitted diff), established and not re-derived

- **`ThemeToken` is a flat `pub enum` of 33 variants**, and the breakdown
  established by reading the enum in `ui/src/ui_core/src/theme.rs` is **ten
  colours** (`Background`, `Surface`, `Primary`, `OnPrimary`, `Text`,
  `TextMuted`, `Border`, `Error`, `Warning`, `Success`), **five spacing**
  (`SpacingXs`…`SpacingXl`), **eight typography** (`FontFamily`, `FontSizeXs`…
  `FontSizeXl`, `FontWeightNormal`, `FontWeightBold`), **four shape**
  (`BorderRadiusSm`/`Md`/`Lg`, `BorderWidth`) and **six motion** (`DurationFast`/
  `Normal`/`Slow`, `EasingStandard`/`Decelerate`/`Accelerate`) — 10 + 5 + 8 + 4 +
  6 = **33**, which is what `const TOKEN_COUNT: usize = 33;` asserts. **The
  breakdown in this task's brief said 9 colours, 6 typography and 4 motion; that
  is wrong, and 33 is right.** The count is quoted in exactly four places in the
  crate and requirement 2 names all four.
- **`TOKEN_COUNT` is `const` and `private`.** It is reachable only through
  `ThemeToken::all() -> &'static [ThemeToken]`, whose doc records *"a fixed order
  rather than a hash iteration so a transition's frames are reproducible"*, and
  which returns a slice of a `static ALL_TOKENS: [ThemeToken; TOKEN_COUNT]`.
- **`Theme` is `Theme { tokens: HashMap<ThemeToken, Property<PropertyValue>>,
  clock: RefCell<AnimationClock> }`** and its public surface is exactly `new`,
  `dark`, `light`, `get`, `set`, `property`, `switch_to`, `tick` (plus `Default`).
  **Zero** occurrences of `scope`, `inherit`, `parent_theme`, `push_theme` or
  `pop_theme` as an identifier anywhere in `ui/src/ui_core/src/`. The `RefCell`
  is there because `switch_to` takes `&self`, and `Theme` is **not** `Clone`.
- **`Theme::from_table` is private and enforces nothing.** Its doc says *"The
  table must hold every token exactly once; a token missing from it would have no
  property, and `Theme::get` would answer it with the default"* — and **nothing
  tests it**, so the defect it describes is reachable today. `Theme::get`
  answers an absent token with `PropertyValue::default()`, which is transparent
  black, and `switch_to` reads its target from `new_theme.get(*token)`, so a
  token missing from one table **animates toward transparent black** on every
  switch. Requirement 3 closes that hole with a test, because this task is the
  one that makes adding a token to `ALL_TOKENS` without a row in both tables a
  mistake somebody is likely to make.
- **`Palette::from_theme` is how shipped widgets resolve *which* tokens they
  use**, and there are **twelve** of them across **eleven** widgets — `button`
  has **two** (`Palette` and `Motion`), and `chart`, `dialog`, `gauge`,
  `keyboard`, `progress`, `scroll`, `slider`, `text_input`, `toast` and `toggle`
  have one each. **A gauge is the case row `L6b`(a) is about**: `gauge::Palette`
  has four fields (`track`, `fill`, `tick`, `needle`) and maps them to `Border`,
  `Primary`, `TextMuted` and `Text` — four colours it can draw and **no token for
  any of the three gauge types' distinct geometry**. `Label` and `Button` each
  resolve their own, and `Label` resolves **no** theme token at all.
- **Three widget palettes read a focus ring straight off
  `ThemeToken::Text`, and say why in the test that pins it.**
  `toggle::Palette::ring`, `scroll::Palette::ring` and `slider::Palette::ring`
  are all `token_color(theme, ThemeToken::Text)`, and each of the three
  existing tests — `the_palette_is_the_themes_muted_primary_and_on_primary`,
  `the_palette_is_the_themes_border_text_muted_and_text` and
  `the_palette_is_the_themes_border_primary_and_on_primary` — asserts it, with
  the reason in the assertion's message: *"the ring is on the background, not on
  the groove"*, *"the focus ring is on the background, not on the track"*. And
  `slider::Palette::from_theme`'s own doc says the rule in the crate's words:
  *"the ring is [`Text`], the colour this repository uses for anything that has
  to be legible on the background itself."*
- **Two widget palettes read a ring from a *different* token, on purpose.**
  `button::Palette::ring` is `OnPrimary`, because a button's ring sits on
  `Primary` and `OnPrimary` is by definition the colour drawn on it. And
  `keyboard::Palette::ring` is `Border`, documented as *"the ring sits **outside**
  the panel and has to be legible on the page behind it"*. **Neither is the
  `FocusRing` case and requirement 4 changes neither.**
- **State and appearance are already separate on every shipped widget.**
  `ButtonState` is *"the **primary** state: the states overlap"*; `Button::style`
  is *"the pure resolution"* of four state flags into a `Style`; the same shape
  is on `Toggle`, `Slider`, `Scroll` and `Keyboard`. **`Button::focus_ring` is a
  `Property<f32>`** (a width in pixels) and **`Button::focused` is a
  `Property<bool>`** (a fact about the keyboard), and they answer different
  questions. `Toggle::DISABLED_OPACITY`, `Button::HOVER_LIGHTEN`,
  `Button::PRESS_DARKEN`, `Button::PRESSED_SCALE` and
  `Button::DISABLED_DESATURATE` are private constants.
- **Every token is a `Property`, so a theme switch animates through the existing
  `switch_to`/`tick` machinery** and a dependent bound by
  `Property::bind` is recomputed on every frame of the transition — proven today
  by `theme.rs`'s `a_switch_notifies_a_dependent_on_every_frame_it_moves`.
  `Property<T>` has `new`, `bind`, `get`, `set`, `on_change`, `handle`,
  `is_bound`, `animate_to`, `animate_from_to` and nothing else; `bind`'s closure
  is `Fn() -> T + 'static` and receives **no arena and no node handle**.
- **`WidgetNode` has `children()`, `parent()`, `layout()`, `layout_mut()`,
  `paint()`, `paint_mut()` — and no arena**, in its module doc's own words:
  *"A node cannot reach the arena that holds it."* `node::descends_from` is
  private to `node.rs`. `Arena<T>` has `get`, `get_mut`, `is_valid` and the rest.
- **`PRIMITIVES_ARCHITECTURE.md` § *Theme tokens*** writes the enum out as a code
  block, and `theme.rs`'s `ThemeToken` doc cites that section as the authority for
  **the order the variants are declared in**. **Adding a variant and not editing
  that block makes that citation false**, which is the defect
  `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice
  (*"Two doc comments assert the opposite of the code beside them"*).
- **`PRIMITIVES_ARCHITECTURE.md` § *Inheritance*** currently reads *"Some
  properties inherit from parent to child unless overridden: `font_family`,
  `font_size`, `color`, `opacity`, `visibility`."* **None of the five is true**,
  and `ui/src/ui_core/src/property.rs`'s module doc says *"the inheritance that
  lets a property defer to its parent"* where **`Property` has no parent**. Both
  are `TASK_UI_PRIM_47`'s to fix — see § *Task 47 has already fixed two things;
  do not duplicate either* below.
- **`ui/src/ui_demo/src/main.rs` builds one widget `Palette` by struct literal**:
  `tab_palette(theme, selected)` writes `ButtonPalette { background, foreground,
  ring }` with all three fields named. **Any change to the *fields* of a widget's
  `Palette` breaks that literal.** It is one of the reasons requirement 4 changes
  no `Palette` field.
- **The suite at this tree is 1894** — **1450** `ui_core` + **224** `ui_demo` +
  **220** doctests — measured on `75a896c` plus the uncommitted diff with
  `cargo test --all-features` from `ui/`; `ui_core` runs 1451 and reports one
  ignored. **The handoff names the tree it measured on**, because four task
  files are open at once and *on a shared tree, the suite
  you ran is not your suite* is the rule that a number without a tree is not a
  result.
- **`.ai/tools/fps-check.sh` takes `seconds` then `floor` and runs the binary
  with no arguments**, so it cannot name a page; the per-page form is
  `ROADOS_RUN_SECONDS=<n> ./target/release/ui_demo --tab=<page>`.
  `.ai/tools/README.md` § *Frame-rate baseline* records the band.
- **No pointer event has ever been observed reaching this window on this host**
  (`.ai/tools/README.md` § *Capturing a window*, measured across several sessions). **Nothing in this task is verified
  by a pointer, and no acceptance criterion here asks for one** —
  *an acceptance criterion that names an instrument which cannot
  produce the evidence is not met by producing the evidence another way* is the
  rule that decides that.

### One task, not two

**Decision: one task, `TASK_UI_PRIM_50`, with a fan-out inside it.** The two
halves are separable in code and **not separable in review**. Four reasons, and
the first is the one that settles it.

1. **The token half's acceptance criterion *is* the palette half's acceptance
   criterion.** The one new token is introduced **at the exact value its three
   consumers already read from `Text`**, and that is what makes the change
   pixel-free. So *"does the token change any colour?"* is only checkable
   against `toggle`/`scroll`/`slider`, and *"do those three palettes resolve the
   right token?"* is only checkable against `theme.rs`. **Split, each half has an
   acceptance criterion the other half has to answer**, which is the definition of
   a split that buys nothing.
2. **The token half is seven edits in one file and no new file.** One enum
   variant, one entry in `ALL_TOKENS`, one row in each of two theme tables, one
   number in `TOKEN_COUNT`, one line in a test helper, one assertion in an
   existing test and one sentence in a doc comment. Under `developer.md`
   § *Scope check* it is **not a component** — a component is a module or struct
   "that can be built and tested separately", and **this cannot be tested at all
   until the palette half exists**. **A task whose first deliverable is seven edits
   and a number has bought a second review round and a second operator-commit
   gate.**
3. **`L9` is one row, and its Blocks column is the scoping half.** Splitting
   leaves the tree in a state where the row looks addressed and the thing that
   blocks *"any subtree that needs to differ from the global theme"* still does
   not work — the exact "a gap closed on paper and open in the library" shape
   `DEMO_APPLICATION.md` § *Corrections to the second gap table* was written
   about, and task 40's § *What this task does to gap `L4`* is the precedent for
   writing the honest limit instead.
4. **The hard case is only expressible with both halves.** § *The hard case: an
   animated switch inside a scope* is a question about an **overridden token**
   during a **switch**. With no tokens beyond `Text` there is nothing to override
   in the test, and with no scope there is nothing for the switch to be inside of.

**Rejected: two tasks, `50a` and `50b`.** Right about separability, wrong about
what separates. The split that would buy something is **(b) scoping plus (a) one
token in one task** and **six declined tokens in a documentation decision**, which
is what this file already is: the declines are prose with named precedents, not
code.

### The token decision: one token, and why a token per interaction state is not right

**Decision: `ThemeToken::FocusRing`, and nothing else. `TOKEN_COUNT` goes 33 →
34. `Theme`, `PropertyValue` and `Theme::from_table`'s signature do not change.**

The question this task has to answer is *"Is a token per interaction state
actually right, or should a state resolve through a scope?"* The answer is
**neither, and the reason is a rule the crate already wrote down and applied in
four modules.**

> **The crate's own test for whether a value is a token** — stated in
> `button.rs`'s `MIN_TOUCH_TARGET` doc and repeated in `keyboard.rs`, `toggle.rs`
> and `slider.rs`: *"adding one would change [`ThemeToken::all`], both theme
> tables, the token count, and the animation every token takes part in during a
> theme switch — **all for a value a theme switch does not change**."*

So the question each candidate token has to answer is not *"would a token be
useful?"* but **"does a theme switch change this value?"** A value no switch moves
is a widget constant wearing a theme's clothes, and the crate has several such
constants already, each with that exact argument in its own doc comment. Applying
the rule to all seven names `L6b`(a) and `L9` list:

| Candidate | A switch changes it? | Outcome, and the precedent that decides it |
|---|---|---|
| **`Focus`** | **yes** — it is a *colour*, and the colour a ring must be differs between a light and a dark surface | **added** as `ThemeToken::FocusRing` |
| **`Hover`** | no | **declined.** `button.rs`'s `Palette` doc, in the crate's own words: *"The states are not colours of their own … a button is themed with one pair of colours and one ring colour, and [`Button::style`] derives the hovered, pressed and disabled appearances from them."* A hover colour would be a theme value that *freezes today's shading arithmetic* — `HOVER_LIGHTEN = 0.12` and `PRESS_DARKEN = 0.18` — into two tables, trading a **derived** appearance for a **literal** one. `keyboard.rs` says the same thing about its six palette fields: *"adding six would put six more tokens in every theme table and in every theme switch"* |
| **`Pressed`** | no | **declined**, same argument, and `PRESSED_SCALE = 0.95` and `PRESS_SHADOW_ALPHA = 0.28` beside it |
| **`Shadow`** | no | **declined**, twice over, because there are **two different shadows** in this crate and neither is a widget's. The **pressed inner shadow** is `Button`'s `PRESS_SHADOW_ALPHA`/`PRESS_SHADOW_INSET` — constants a switch does not move. The **offscreen backdrop shadow** is a constant in `render.rs`, because `ShadowTarget` is `GL_R8`, one channel of coverage, *"chosen because a shadow's colour is one constant"* (`DEMO_APPLICATION.md` row `L1`'s own evidence). Making that one tokenable means `render.rs` reads a theme, which is a **pipeline change** and out of scope by two rows |
| **`ZOrder`** | **not a value at all** | **declined, and this one is not a judgement.** Paint order in this crate is *recording* order — `DrawCommand` has nine variants and **none carries a z or a transform** (`row L2`'s evidence), 2D is painter's order, and `Demo::frame` records from a flat `order: Vec<Handle>`. **A number cannot reorder a vector.** A `ZOrder` token would be a value the frame loop does not read, and the frame loop is the one stage of the frame whose ordering task 37 made a recorded contract |
| **`Active`** | no | **declined.** `Button` carries **no `selected`** and no active state; `Keyboard`'s active key is `Primary`, read directly, and `text_input`'s focus border is `Primary`, read directly. **A token nothing reads is dead code**, which `developer.md` § *Code quality* refuses |
| **`Selected`** | yes, in principle | **declined, and named as another task's.** The one widget in this repository with a genuine selected appearance is **`TabBar` — `TASK_UI_PRIM_43`, row `7`**, which has not landed. Adding `Selected` here would put a token in two theme tables and every switch for a widget that does not exist. `TASK_UI_PRIM_47`'s `Segmented` derives its selected fill from `Primary`, which is the crate's own position |

**And the one that is added earns its place on a second ground as well.** It is
introduced **at the value its three consumers already use** — exactly
`ThemeToken::Text`'s value in each shipped theme — so **the change moves no
pixel**, and it is introduced anyway because **the name is the contract**. Today
three widgets read `Text` *as a proxy* for "the colour a focus ring takes", and
the three assertions pinning that are assertions about a **coincidence**: the day
someone darkens `Text` for reading comfort, a focus ring moves with it, and
`the_palette_is_the_themes_border_text_muted_and_text` would fail on a change
nobody intended. After this task those three assertions pin a **token identity**,
which is stronger than pinning a colour.

- **The variant is `FocusRing`, its doc names the rule in
  `slider::Palette::from_theme`'s words**, and it is declared **immediately after
  `ThemeToken::Success`**, which is the last colour — so it is grouped with the
  colours and every existing token keeps its index in `ALL_TOKENS`.
- **`ThemeToken::FocusRing` holds `Text`'s value in each shipped theme**:
  `(255, 255, 255, 255)` in dark and `(0, 0, 0, 255)` in light. So it **differs
  between the themes** — which `dark_and_light_define_every_token` requires of
  every colour token — and **its animation during a switch is the `Text`
  animation**, because it interpolates between the same two colours. **A capture
  taken half way through a theme switch therefore cannot differ either**, and
  that is a claim this file can make rather than hope for.
- **`TOKEN_COUNT` stays `const` and stays `private`.** `ThemeToken::all()`
  answers the question it answers. **A second public count is a second thing to
  disagree with the first**, which is `TASK_UI_PRIM_47` requirement 15's recorded
  position and this task's too.

### The scoping decision: a map keyed by node, resolved by the nearest ancestor

**Decision: `scope::ThemeScope`, a `Vec` of `(Handle, ThemeToken,
Property<PropertyValue>)`, holding only what a node overrides, resolved by a
nearest-ancestor walk and resolved by the **owner**, never by the widget.**

It is **option (B), a per-node override map**. The alternative the crate already
has — a **palette-level override** — is weighed and refused in reason 2 below,
because it cannot express the subtree row `L9`'s Blocks column names.

1. **The mechanism that makes a token readable is inheritance, and (B) is
   inheritance.** `TASK_UI_PRIM_47` § *The theme gains no token, and `L9` is why*
   says the reason in the crate's own words: *"A mode token with no scoping is a
   token nothing reads."* The same sentence applies to `FocusRing`: a ring colour
   nothing can restyle cannot answer `L6b`(a)'s *"so a mode cannot restyle a
   subtree"*.
2. **A `Palette`-level override cannot express a subtree, and `L9`'s Blocks
   column is a subtree.** `Palette::from_theme(theme: &Theme)` takes a `&Theme`
   and returns a struct of plain `Color`s. Making it scope-aware is one of three
   things, and all three are refused:
   - **change its parameter** — ten widget palettes plus a public signature, for a
     generic bound that would need a trait with **one** impl, which is
     `developer.md` § *Phase 2*'s *"No abstraction before the second use"* at its
     plainest, and a `Box<dyn Trait>`-free alternative does not exist;
   - **add a scoped constructor per palette** — ten copies of the same resolution
     logic, in ten files, for one mechanism;
   - **give `Palette` an override field** — a **field** addition to ten public
     structs, and `ui/src/ui_demo/src/main.rs`'s `tab_palette` builds
     `ButtonPalette` by **struct literal with all three fields named**, so this
     breaks the demo's build and therefore this task's pixel criterion.
   **What is kept is what already exists**: a caller that wants a narrower answer
   than the scope gives still writes `set_palette`, exactly as it does today. A
   scope and a palette override are two widths of the same ruler, and the scope is
   the wider one.
3. **A push/pop stack on `Theme` is refused, and the refusal is
   `TASK_UI_PRIM_47`'s own.** The crate has no paint traversal for a stack to
   push and pop on: `Layout::visit` is private to `layout.rs`, and the paint pass
   is `Demo::frame`'s own flat `for handle in self.order.iter()` with one `match`
   arm per widget — a **list, not a walk**. A stack would therefore not be *"add
   push/pop"*; it would be ***build a paint traversal in `ui_core` and reconcile
   it with the demo's flat loop***, in the one stage of the frame whose ordering is
   a recorded contract, **and a flat list cannot nest without the owner computing
   the boundaries itself** — the bug of a container that
   covers the window and swallows every tap aimed at anything behind it.
4. **A `Theme` field is refused, and this is the decisive one.** `Theme` has **no
   node** and must stay the *global* source: `switch_to` iterates
   `ThemeToken::all()` and animates **every** member, so a scope stored on `Theme`
   is either animated — wrong, because the owner chose a literal — or skipped,
   and then `Theme::get` would have to become scope-aware, which needs a handle
   it does not have and would make `get` ambiguous between *"the global value"*
   and *"the value under this node"*. **`Theme` stays one flat global map, which is
   half of `L9`'s sentence and stays true by design.**
5. **The walk is the crate's second copy of an ancestor walk and not its third
   object.** `node.rs`'s private `descends_from` answers *"is this an ancestor of
   that"*; `ThemeScope` needs *"give me each ancestor in turn"*. **No shared seam
   exists to promote**, and `developer.md` § *Phase 2`'s rule cuts against
   building one. `TASK_UI_PRIM_47`'s `ModeScope::resolve` is the same walk, and
   the crate having two of them is recorded here as a later cleanup rather than
   smuggled in.
6. **`entries` is a `Vec`, not a `HashMap`,** for `TASK_UI_PRIM_47`'s reason:
   *"a `Vec` gives a **stable iteration order** where a hash map would not — the
   same reason `ALL_TOKENS` is a `static` array."*

**And the answer arrives as a `Property<PropertyValue>` handle, not a value.**
That is the whole mechanism, and it is what makes it free:

```rust
let resolved = scope.resolve_property(&theme, &nodes, node, ThemeToken::Surface);
let surface  = Property::bind(move || resolved.get().as_color().unwrap_or(fallback));
```

`Theme::property` returns a **clone of an `Rc`**, so a resolved handle that fell
through to the theme is **the same property the switch animates**, and a widget
bound to it follows every frame of the transition **with no per-frame
re-resolution and no borrow of the arena during paint**. And a widget **cannot**
resolve its own scope — `WidgetNode` has no arena and `Property::bind` gets no
handle — so **the owner resolves and hands it over**, which is `TASK_UI_PRIM_47`'s
rule by name, applied to colours.

### Inheritance semantics, in three rules

1. **A child sees its nearest ancestor's override.** The walk starts at the node
   itself and steps `WidgetNode::parent()` until the arena answers `None`, exactly
   as `node.rs`'s `descends_from` steps it, so an override on a container reaches
   every descendant that does not override the same token itself. **Every
   ancestor is checked for the token being asked about and not for "any entry"**,
   which is what makes rule 2 true.
2. **A child overrides, per token, and the nearest wins.** Setting
   `(handle, token)` shadows a shallower scope **for that token only** and leaves
   every other token inherited. **A deeper scope never replaces a shallower one
   wholesale**, because a scope holds only what it changes.
3. **A partial override is the default shape and needs no special case.** A scope
   carrying one override for `Surface` resolves `Surface` from the override and
   **all thirty-three other tokens from the theme**. **There is no
   "replace the whole theme" operation and no copy of a theme anywhere** — a
   scope cannot become a second `Theme`, which is what keeps `L9`'s first
   sentence ("one flat global token map") true. **The common case — one button
   different inside a themed panel — is one `set_override` on the panel's node.**

**One limitation, pinned rather than left implicit: a handle resolved *before* an
override is added keeps the token it resolved.** An override applied after a
widget was bound does not reach that widget, **because the widget holds a handle,
not a lookup**. That is the crate's *"aim once, tick per frame"* discipline
applied to resolution (`sync_toggle_state`'s recorded argument: a per-frame aim
creeps toward its target and never arrives), and the remedy is the one the crate
already uses for a palette change — re-resolve and re-bind, then
`animate_to_state`. `a_handle_resolved_before_an_override_was_added_keeps_the_
theme_token` asserts it, so a reader is not left to discover it.

### The hard case: an animated switch inside a scope

**This is a question, and it has an answer rather than a deferral.** `Theme::
switch_to` clears the clock and animates **every** member of `ThemeToken::all()`
from the value it holds to `new_theme`'s. An override is **not** a member of the
theme and **not** in that list. So:

> **A token a scope overrides does not move during a theme switch, and neither
> does anything bound to it. The subtree keeps the override's colour, exactly, for
> the whole transition and after it.**

- **Why that is right, and not a bug.** An override is a **choice the owner
  made**, and the theme knows nothing about it: it is not in `ThemeToken::all()`,
  it is not in either table, and `switch_to` has no handle to it. **A theme
  switch is a change of the *global* palette; a scoped override is the
  deliberate exception to it.** If an override moved with the switch, a themed
  panel would silently revert to the global colour at the end of a transition —
  which is the "a scope that drops the override at the end of a switch" defect,
  and it is the answer a reviewer should try to break first.
- **The token the scope does *not* override animates normally**, in the same
  switch, in the same frame. So **a scoped subtree is half-crossfaded during a
  transition**: the overridden tokens are pinned and the rest are moving. **That
  is the specified behaviour and it is stated in `ThemeScope`'s module doc and in
  `resolve_property`'s**, because a half-crossfaded subtree looks like a defect
  and is not one.
- **There is exactly one supported way to want an override that *follows* the
  switch, and it needs no new machinery: bind the override.** `set_override`
  takes a `Property<PropertyValue>`, so a caller can hand it
  `Property::bind(move || other_token_property.get())` — and then the override is
  a **dependent of a token the switch animates**, so it is recomputed on every
  frame and the scoped subtree crossfades with everything else. **This is the
  crate's own mechanism with no second idea in it**, and
  `an_override_bound_to_another_token_animates_with_that_token_and_not_with_its_
  own` proves it: the override resolves to `Primary` and, half way through a
  switch, to the **interpolated** `Primary` — which is neither the dark theme's
  nor the light theme's, and is not `Surface` at any point.
- **What must not happen, and is tested:** an override must not be *replaced* by
  the theme's value at the end of a switch; an unoverridden token must not stop
  animating because something else in the same scope is overridden; and a handle
  resolved before the switch must **follow** it (test
  `resolve_property_returns_a_handle_that_follows_the_token_it_fell_through_to`,
  which is the mechanism's whole value in one assertion).

### What moves off per-widget properties — and what does not

**Nothing is removed and nothing is replaced. The decision is recorded here
because `AGENTS.md`-level out-of-scope and `developer.md` § *API design*'s
*"Respect semver. Public API changes that break downstream users require
explicit discussion"* both require it to be explicit rather than inferred.**

The distinction the crate already draws is the whole answer: **a token is an
appearance; a `Property<bool>` is a fact.** So:

| | Decision | Why |
|---|---|---|
| **`Button::focused`, `hovered`, `pressed`, `disabled`, `activatable`** | **unchanged** — every `Property<bool>` | These are **state**. `focused` is written by the owner from `input::Focus`; `hovered` and `pressed` are written from a hit test. **A token cannot be one of them**, because a token is read at paint time and these are written before it, and because `Button::focused`'s own doc records that it and `activatable` are *"two answers that come apart"* — a token has one value |
| **`Button::focus_ring`, `Scroll::focus_ring`, `Slider::focus_ring`, `Keyboard::focus_ring`** | **unchanged** — every `Property<f32>` | These are a **width in pixels**, and `FocusRing` is a **colour**. They are not the same kind of thing and no token replaces either. A width that a theme switch does not change is a widget constant, which is the `MIN_TOUCH_TARGET` rule again |
| **`Toggle::DISABLED_OPACITY`, `Button::HOVER_LIGHTEN`/`PRESS_DARKEN`/`PRESSED_SCALE`/`DISABLED_DESATURATE`/`PRESS_SHADOW_*`** | **unchanged, and they stay private constants** | § *The token decision* declines them, with the `Palette`-derivation argument, and they are not `pub` today |
| **`Toggle::Palette::ring`, `Scroll::Palette::ring`, `Slider::Palette::ring`** | **the one thing that changes: which token they read** — `Text` → `FocusRing` | Their **values do not change**, which `the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes` asserts. **No field is added, removed or retyped in any `Palette`**, which is what keeps `tab_palette`'s struct literal compiling |
| **`button::Palette::ring` (`OnPrimary`) and `keyboard::Palette::ring` (`Border`)** | **unchanged, each on its own documented reason** | A button's ring is drawn **on `Primary`**, where `OnPrimary` is by definition the legible colour; a keyboard's ring is drawn **outside its panel onto the page**, where a hairline is right. `text_input::Palette::focus` stays `Primary` for the same family of reason — it is a focus border on the widget's **own** surface |

**So the answer to "if a token replaces them, that is a public API change" is: it
does not, and the reason is that a token and a state property are different kinds
of thing.** The appearance moves onto a named token; the state stays where it is;
and `ButtonState`/`Style` keep doing the resolution between them, which is what
`button.rs`'s module doc already describes.

### The demo does not change, and why that is the design rather than the omission

**`ui/src/ui_demo/src/main.rs` is not touched. Not one line.** That is the single
strongest property this task has, and it is why the token half was cut to one.

- **Row `L9`'s Blocks column is a library capability** — *"Any subtree that needs
  to differ from the global theme"* — and no subtree in the demo needs one today.
  Every themed surface in the demo is themed from the global tokens on purpose:
  `tab_palette`'s own doc works through *why* the unselected tab is `Border` on
  `Text` and *why* the selected one is `Primary` on `OnPrimary`, in the crate's
  own register. **Inventing a panel that wanted a different surface would be a
  parameter with no source**, which is the failure
  `DEMO_APPLICATION.md` § *Asset requirements* records for the colours it could
  not verify, and § *Could not verify* for everything else.
- **A demo-local demonstration would also be the wrong place to put it.** Task 40
  § *The decision* records the inverse of this case, and it is the same rule: a
  demo-local struct cannot close a row of a table **about `ui_core`**, and the
  mechanism a row asks for belongs in the crate. `TASK_UI_PRIM_46` is the
  precedent for a `ui_core`-only task: *"This task is `ui_core` only, and that is
  a decision with a consequence stated."*
- **What the demo gains is nothing, and the pixel criterion is therefore total**:
  **AE 0 outside `y ≥ 680` on all six pages**, with the mechanism in § *Acceptance
  Criteria*.

### `TASK_UI_PRIM_47` has already fixed two things; do not duplicate either

`TASK_UI_PRIM_47` requirement 3 rewrites `property.rs`'s module doc to drop
*"the inheritance that lets a property defer to its parent"* and name
`mode::ModeScope` instead, and its requirement 15 replaces
`PRIMITIVES_ARCHITECTURE.md` § *Inheritance* — *"The section becomes a statement
of what exists: **`mode::ModeScope` is the crate's only inheritance**"* — with a
record in `IMPLEMENTATION_STATE.md` of what was searched. **Both are at `HEAD`
still unfixed**, because `mode.rs` does not exist in the tree this file was
written against.

- **This task does not touch `property.rs`.** One module doc, one sentence, and it
  is 47's. Repeating it would be *two documents each claiming
  ownership of one definition*, with a second patch on a shared file.
- **This task's own edit to § *Inheritance* is to add `scope::ThemeScope` to the
  list 47 writes** — which means **§ *Inheritance` still reading *"Some properties
  inherit from parent to child: `font_family`, `font_size`, `color`, `opacity`,
  `visibility`"* when this task starts is a dependency violation, not a variant of
  this task.** The same is true of the *word* "only": 47's corrected sentence will
  say `ModeScope` is the crate's *only* inheritance, and this task makes that
  false, so the sentence has to be reworded to name two. **47 is the prerequisite;
  every other requirement here is independent of it**, and the sequence's numeric
  order puts it first unless the operator reorders — in which case the handoff
  records that the § *Inheritance* amendment was written against a tree 47 had not
  yet corrected.
- **`TOKEN_COUNT = 33` is asserted in two other task files' acceptance criteria**
  — `TASK_UI_PRIM_44`'s *"**`TOKEN_COUNT` stays at 33**"* and
  `TASK_UI_PRIM_47`'s *"const `TOKEN_COUNT` stays **33**, stays **`const`**, stays
  **private**"*. **Those criteria were true when they were reviewed and this task
  supersedes the number**, the same way a task supersedes a count when the tree
  moves under it. Requirement 9 records that supersession, dated and attributed,
  in `IMPLEMENTATION_STATE.md` — **and neither file is edited**, because a task
  file's acceptance criteria are a historical record of what that task verified,
  not a place a later task edits.

### Scope, measured against `developer.md` § *Scope check*

**Ten files and four components — over both thresholds, so the split is not a
judgement call.** `developer.md` § *Scope check* counts the files the change
creates or modifies and the independent components in it, and splits anything
**above five files or three components** per `.ai/protocols/subagents.md`
§ *Implementation fan-out*.

| | Count | What they are |
|---|---|---|
| **Files** | **ten** | `ui/src/ui_core/src/theme.rs`, `ui/src/ui_core/src/scope.rs` (new), `ui/src/ui_core/src/lib.rs`, `ui/src/ui_core/src/widgets/toggle.rs`, `ui/src/ui_core/src/widgets/slider.rs`, `ui/src/ui_core/src/widgets/scroll.rs`, `doc/ui/DEMO_APPLICATION.md`, `doc/ui/PRIMITIVES_ARCHITECTURE.md`, `doc/ui/IMPLEMENTATION_STATE.md` — plus this file |
| **Components** | **four** | the token and its two theme tables; the scope type and its module wiring; the three palettes' ring mapping; the documentation |

**Eleven files, not ten, if `TASK_UI_PRIM_47` has landed** — its
`ui/src/ui_core/src/widgets/segmented.rs` then carries a `Palette::ring` mapped
off `ThemeToken::Text` and joins sub-task C's set, in the shape 47 requirement 5
specifies. **The implementer states which tree it found**, because a count that
is wrong is the failure `DEMO_APPLICATION.md` § *Corrections to the second gap
table* was written about.

| Sub-task | Files it owns | Its own acceptance test, passable alone | Runs |
|---|---|---|---|
| **A — the token** | `ui/src/ui_core/src/theme.rs` | `cargo test -p ui_core theme` green with `dark_and_light_define_every_token` asserting **34** and `every_token_is_answered_with_a_value_by_both_themes` among the four | **in parallel with B** |
| **B — the scope** | `ui/src/ui_core/src/scope.rs` (new), `ui/src/ui_core/src/lib.rs` (one `pub mod scope;`) | `cargo test -p ui_core scope` green with the seventeen `scope.rs` tests | **in parallel with A** |
| **C — the three palettes** | `ui/src/ui_core/src/widgets/toggle.rs`, `slider.rs`, `scroll.rs` (+ `segmented.rs` if 47 landed) | the three existing `the_palette_is_the_themes_*` tests green **naming `ThemeToken::FocusRing`**, plus `the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes` | **after A** — it does not compile until `FocusRing` exists |
| **the documents** | `DEMO_APPLICATION.md`, `PRIMITIVES_ARCHITECTURE.md`, `IMPLEMENTATION_STATE.md` | `grep` finds the `L9` note dated and attributed, § *Theme tokens* carries the variant, and § *Inheritance* names **two** inheritances | **the developer's own, after A, B and C** |

- **A and B do not depend on each other, and that is checked rather than
  assumed.** `ThemeScope` takes `&Theme`, `&Arena<WidgetNode>`, a `Handle` and a
  `ThemeToken` — **all four exist at `HEAD`** and none of them is `FocusRing` — so
  neither sub-agent is briefed against code that does not exist yet, which is
  `.ai/protocols/subagents.md` § *Parallel or sequential*'s prohibition. **A and B
  touch disjoint files.**
- **C is sequential because it reads a variant A creates**, which is
  `subagents.md` § *Splitting* property 3's *"make A a prerequisite that lands
  first"*. **It is briefed against the code as it stands after A has landed**, not
  against this document's signature list.
- **Each sub-task gets the full `subagents.md` § *Briefing contract*** — the
  sub-task, the files it owns, its acceptance test, the constraints, **what it
  must not touch**, the return format, and **read-write access stated explicitly,
  because unlike research fan-out implementation subagents write to the
  repository** — and returns that format rather than prose. **The developer
  orchestrates, integrates, runs the full verification suite once over the
  integrated result, and hands off.**
  `.ai/workflows/task-sequence.md` § Gates then reviews the **integrated** whole,
  because a sub-task reviewed in isolation is reviewed again as part of the whole.
- **If the implementer is *not* fanning out** — one agent, three sub-tasks in
  sequence — **the same three file sets apply and the same stop condition does: a
  fourth code file outside those three sets is a stop condition, not an
  expansion** (`developer.md` § *Stop conditions*). A change that cannot be split
  along file boundaries is not a splitting problem; this one splits cleanly.
- **The counts above are this file's estimate; the handoff reports the counts it
  found.** Four task files are open in this tree, so *"ten files"* is this task's
  own and not the repository's.

## Requirements

1. **A new module `ui/src/ui_core/src/scope.rs`, declared `pub mod scope;` in
   `ui/src/ui_core/src/lib.rs`** in alphabetical position (**after `render`** and
   before `snapshot` if `TASK_UI_PRIM_47` has landed, before `texture` otherwise),
   holding exactly one public type:

   ```rust
   /// A set of per-node token overrides: what a subtree resolves instead of the theme.
   #[derive(Clone, Default)]
   pub struct ThemeScope {
       overrides: Vec<(Handle, ThemeToken, Property<PropertyValue>)>,
   }

   impl ThemeScope {
       #[must_use] pub fn new() -> Self;
       #[must_use] pub fn len(&self) -> usize;
       #[must_use] pub fn is_empty(&self) -> bool;
       #[must_use] pub fn set_override(
           &mut self,
           handle: Handle,
           token: ThemeToken,
           value: Property<PropertyValue>,
       ) -> bool;
       #[must_use] pub fn clear(&mut self, handle: Handle) -> usize;
       pub fn retain(&mut self, nodes: &Arena<WidgetNode>) -> usize;
       #[must_use] pub fn resolve_property(
           &self,
           theme: &Theme,
           nodes: &Arena<WidgetNode>,
           handle: Handle,
           token: ThemeToken,
       ) -> Property<PropertyValue>;
   }
   ```

   - **`derive(Debug)` is refused, and the reason is worth a line.**
     `ThemeToken` derives `Debug`; **`Property` does not**, so a derived `Debug`
     on `ThemeScope` would not compile. **`ThemeScope` derives `Clone` and
     `Default`, and not `Debug`**, and the reason is in the module doc:
     `Property` has no `Debug` implementation, and writing one to get a derive
     to compile is a change to `property.rs`, which § *`TASK_UI_PRIM_47` has
     already fixed two things* puts out of this task's reach. `Default` is
     derived and equals `new()`, which is an empty scope.
   - **`overrides` is a `Vec` of triples, not a `HashMap` and not a
     `HashMap<Handle, HashMap<ThemeToken, …>>`.** § *The scoping decision*'s
     sixth reason: a stable iteration order, the same reason `ALL_TOKENS` is a
     `static` array. `ThemeScope` is `Clone` because a caller may want to keep a
     scope per page and hand one out; **it is cheap to clone** — three words plus a
     `Vec` of `Rc` clones — and **cloning it shares the override properties**, so a
     clone is a second view of one set of overrides and not a copy of them. The
     doc says so, because the alternative reading (a deep copy) is the expensive
     one.
   - **`set_override` returns whether the scope *gained* an entry** — `true` when
     there was no override for that `(handle, token)`, `false` when one was
     replaced. **`Property` has no `PartialEq`, so "the value changed" is not
     answerable, and the entry count is what is.** `#[must_use]`, because a caller
     that ignores it cannot tell a first override from a replacement.
   - **`clear(handle)` returns how many overrides it dropped for that node** —
     **not `bool`**, unlike `mode::ModeScope::clear`: a theme scope clears *several
     tokens at once*, and a count is the answer a caller's log or test wants.
     `#[must_use]`. `retain` returns the number it dropped and is **deliberately
     not `#[must_use]`** — `TASK_UI_PRIM_47`'s exact position: its count is
     information for a test and a log, not a contract.
   - **`set_override` takes no arena.** A stale entry cannot be resolved — no
     node's ancestor chain passes through a handle the arena cannot answer — so
     refusing one is a convenience, not a correctness rule. `node::attach`'s
     refusal precedent does not transfer to a map that is consulted, not
     traversed.
   - **`resolve_property` is `O(depth)`** — one `nodes.get` per ancestor — and its
     doc carries that arithmetic and the comparison with `node.rs`'s private
     `descends_from`, **including why the two are not the same walk**: one answers
     *"is this an ancestor of that"*, the other needs each ancestor in turn.
   - **`resolve_property` returns a `Property` and never a `PropertyValue`**,
     because the returned handle is **the same `Rc` the switch animates**, which is
     what makes § *The hard case* work with no per-frame work. **The doc says in
     its first lines that a caller binds it with `Property::bind` and gives the
     example**, because the alternative reading — read the value once — produces a
     subtree that never follows anything.
   - **The module doc records**: what a scope is and **what it is not** (not a
     second `Theme`, not a stack, not a mode, not a trait, not a paint traversal);
     the three inheritance rules; **the switch semantics in full**, including that
     a scoped subtree is **half-crossfaded** during a transition and why that is
     not a defect; the bound-override route for a scope that *should* follow the
     switch; the owner-resolves rule and why (`WidgetNode` has no arena);
     `ModeScope`'s existence beside it and why there are two walks; and the
     `L6b`(a) hand-off by name.

2. **`ThemeToken` gains exactly one variant, `FocusRing`**, and the token count
   goes **33 → 34**. Every edit in `ui/src/ui_core/src/theme.rs`, and there are
   **five** of them — **and the fifth is prose about the count rather than the
   count, which is the one that is easiest to miss**:

   - `ThemeToken` declares `FocusRing` **immediately after `ThemeToken::Success`**,
     so it is grouped with the colours and every existing token keeps its index in
     `ALL_TOKENS`. Its doc names the rule in `slider::Palette::from_theme`'s words
     — *"the colour this repository uses for anything that has to be legible on the
     background itself"* — **and says what it is not**: not the ring on a
     `Primary` surface (that is `OnPrimary`) and not a hairline drawn outside a
     panel (that is `Border`), both of which requirement 4 leaves alone.
   - `static ALL_TOKENS` gains the one entry, **in the same position**, so the
     array's length still *is* `TOKEN_COUNT`.
   - **`const TOKEN_COUNT: usize = 34;`** — still `const`, **still private**, still
     **not `pub`**, and the handoff says in one sentence why it stays private.
   - **`Theme::dark` and `Theme::light` each gain one row**, and **the value in
     each is exactly what `Text` holds in that theme**: `(255, 255, 255, 255)` in
     dark and `(0, 0, 0, 255)` in light, written as the same `Color::new(...)`
     calls the `Text` rows above them use. **Each row's doc says why it is the
     same number today** — so the change is free — **and why the token exists
     anyway**: the name is the contract, and three widgets were reading `Text` as
     a proxy for it.
   - **`PropertyValue`'s doc**, which currently says `Theme::get` and `Theme::set`
     have *"a single signature for all 33 tokens"*, becomes **34**. **This is the
     fifth edit and it is the one that is easiest to miss**, because it is prose
     about the count rather than the count — which is why
     `DEMO_APPLICATION.md` § *Corrections to the second gap table* is a section
     and not a habit.

   **`Theme::get`, `set`, `property`, `switch_to`, `tick`, `new`, `dark`, `light`,
   `from_table`, `PropertyValue` and its `as_*` accessors are byte-identical.**
   No signature changes. No `unsafe`, no `unwrap`, no `expect`, no `panic!`.

3. **`theme.rs`'s test module, in full.**

   - **`dark_and_light_define_every_token` keeps its name, its comment and its
     shape; its number becomes 34** — `assert_eq!(ThemeToken::all().len(), 34)` —
     and the comment's *"all 33 tokens"* becomes *"all 34 tokens"*.
   - **`is_color_token` gains `ThemeToken::FocusRing`**, so the test's
     "every colour token differs between the two themes" clause covers the new
     token. It must pass on its own: white in dark, black in light.
   - **`every_token_is_answered_with_a_value_by_both_themes` is new, and it closes
     a hole that is reachable at `HEAD`.** For **each** theme and **each** token in
     `ThemeToken::all()`, it asserts `theme.get(token) != PropertyValue::default()`
     with the token named in the message. **The reasoning is `from_table`'s own
     doc, which nothing tests**: a variant added to `ALL_TOKENS` without a row in
     one table answers transparent black from `get`, **and `switch_to` reads its
     target from `new_theme.get(*token)`, so that token fades to nothing on every
     switch** — a silent, plausible, green failure. The assertion goes through the
     **public** `get` and needs no access to the private `tokens` map, and the doc
     says so.
   - **`the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes` is
     new, and it is this task's pixel-identity guard.** For **both** themes it
     asserts `theme.get(FocusRing) == theme.get(Text)`. **Every reason the six
     gallery pages draw the same pixels is this one assertion**, and it is a test
     rather than a reading of the diff.

4. **The three widget palettes re-point their ring at the new token, in one line
   each, and change no field.** `toggle::Palette::from_theme`,
   `slider::Palette::from_theme` and `scroll::Palette::from_theme` map
   `ring: token_color(theme, ThemeToken::FocusRing)` where each maps `Text` today.
   **`TASK_UI_PRIM_47`'s `Segmented::Palette::from_theme` does the same**, if it
   has landed — and the file list above and sub-task C's set each gain that one
   file, and only then.

   - **Each of the three (four) existing assertions that pins the ring is updated
     to name `ThemeToken::FocusRing`, and its reason message is rewritten** — in
     `toggle.rs`, `scroll.rs` and `slider.rs` these are the assertions inside
     `the_palette_is_the_themes_muted_primary_and_on_primary`,
     `the_palette_is_the_themes_border_text_muted_and_text` and
     `the_palette_is_the_themes_border_primary_and_on_primary`. **The test names
     keep their existing names, every other assertion in them is byte-identical,
     and the update is recorded as an update** in the handoff, with the reason:
     the assertion changes from **a colour coincidence** to **a token identity**,
     and it is strictly stronger because
     `the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes` now
     pins the coincidence's other half explicitly. **No test is deleted, renamed
     away or weakened.**
   - **`button.rs`, `keyboard.rs` and `text_input.rs` are not touched.** Their
     rings come from `OnPrimary`, `Border` and `Primary` on the three documented
     reasons in § *What moves off per-widget properties*. **A fourth
     `Palette::from_theme` re-pointed "for consistency" is a stop condition, not
     an expansion**, and the handoff says which three were left and why.
   - **`Palette` gains no field anywhere.** `ui/src/ui_demo/src/main.rs`'s
     `tab_palette` builds `ButtonPalette` by struct literal with all three fields
     named, and **a fourth field breaks the demo's build** — which is the demo's
     pixel criterion by way of a compile error.

5. **`scope.rs`'s module doc**, in the form `rotator.rs` and `render/mesh.rs`
   carry, carrying every claim in § *The scoping decision*, § *Inheritance
   semantics* and § *The hard case* above — **in particular the half-crossfade,
   which is the one fact a reader will get wrong from the code.**

6. **Two doctests in `scope.rs`'s module doc**, each executable and each
   load-bearing:

   - **A scope with no overrides answers every token from the theme** — a
     `Theme::dark()`, an `Arena`, one node, `resolve_property` for `Surface`, and
     `assert_eq!(resolved.get(), theme.get(ThemeToken::Surface))`.
   - **A partial override**: a container node with a child, one override for
     `Surface`, and the assertion that **the child resolves the override for
     `Surface` and the theme's value for `Text`** — the common case from §
     *Inheritance semantics* rule 3, in four lines.

7. **`PRIMITIVES_ARCHITECTURE.md`, three edits, all dated where they are claims
   and none elsewhere.**

   - **§ *Theme tokens*** — the code block listing the enum gains
   `FocusRing` **at the end of its colour block**, matching the declaration order
   requirement 2 fixes. **This edit is not optional and not cosmetic:
   `theme.rs`'s `ThemeToken` doc cites that section as the authority for the
   order the variants are declared in, so leaving it is a doc comment asserting
   the opposite of the code beside it** — the defect
   `DEMO_APPLICATION.md` § *Corrections to the second gap table* records twice.
   § *Animated theme transitions* is **not** edited: its `switch_theme` sketch is
     already loose (`current_theme.set(token, Property::animate(...))` is not
     `Theme::switch_to`'s signature) and correcting it is not this task's
     business.
   - **§ *Inheritance*** — **prerequisite: `TASK_UI_PRIM_47` has corrected it.**
     Its corrected text names `mode::ModeScope` as the crate's only inheritance;
     this task's edit adds **`scope::ThemeScope`** and rewords **"only"** to name
     **two** inheritances — one of a caller-chosen `K` resolved by the owner, one
     of a `ThemeToken` resolved by the owner — **and states that `Property` has
     none**, which is the sentence 47 removed the false version of. **If §
     *Inheritance* still reads *"Some properties inherit from parent to child:
     `font_family`, `font_size`, `color`, `opacity`, `visibility`"* when this task
     starts, this is a dependency violation and not a variant of this task.**
   - **§ *Module Layout*** — `scope.rs` gains a row **after `render/`**'s last
     entry and before `texture.rs`, matching `lib.rs`. **No other row is touched**,
     and **if `TASK_UI_PRIM_47` has landed its `mode.rs` and `snapshot.rs` rows are
     left exactly as 47 wrote them** — a second task editing another task's rows
     in the same table is how a table stops having one owner.

8. **`DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*, row `L9`,
   gains a dated note and its evidence column is rewritten. Nothing else in that
   section is touched.**

   - The note records: (i) **`scope::ThemeScope` closes the row's first clause**
     — the Blocks column's *"Any subtree that needs to differ from the global
     theme"* — with the three inheritance rules; (ii) **`ThemeToken::FocusRing` is
     added and the count is 34**, with § *The token decision*'s table as the reason
     six of the seven names are declined; (iii) **the row is not deleted and not
     marked closed**, because *"no tokens for … hover, pressed, shadow or z-order"*
     is **still true** and is now a recorded position rather than an omission;
     (iv) **no per-widget property was removed or replaced**, with § *What moves
     off per-widget properties*'s table; (v) **`TASK_UI_PRIM_47`'s hand-off of
     `L6b`(a) is discharged** — clause (a)'s *"so a mode cannot restyle a subtree"*
     is answered by the scope, not by seven tokens.
   - **The evidence column is rewritten in place and its line numbers are
     removed**, replaced by **symbol name and file path**: `ThemeToken` and
     `TOKEN_COUNT` in `ui/src/ui_core/src/theme.rs`, `Theme` in the same file, and
     the per-widget states at `Button::focused`, `Button::hovered`,
     `Button::pressed`, `Button::focus_ring`, `Button::disabled` and
     `Button::activatable` in `ui/src/ui_core/src/widgets/button.rs`. **Every row
     in this table that keeps a `file.rs:line` citation is stale the day the tree
     moves; this row's is rewritten because this task re-verified it, and the
     other rows are left alone** — fixing them is § *Corrections to the second gap
     table*'s business, not a side effect of this task.
   - **Row `L6b` is not edited.** Its dated note from 47 already points at `L9`,
     **and that pointer now resolves**; requirement 9 records that in
     `IMPLEMENTATION_STATE.md` rather than editing a note another task owns.

9. **`doc/ui/IMPLEMENTATION_STATE.md` gains one dated entry**, carrying: the two
   type and variant names; **the one-task decision and the four reasons**; the
   token table from § *The token decision* with each decline's precedent; **the
   resolved property mechanism** (`resolve_property` returns an `Rc` clone, so a
   widget bound to it follows the switch with no per-frame work); **the three
   inheritance rules**; **the switch-in-a-scope answer in full, including the
   half-crossfade**; **which per-widget properties changed and which did not**;
   **that `TOKEN_COUNT` is 34, `const` and private, and that this supersedes the
   `TOKEN_COUNT = 33` criterion in `TASK_UI_PRIM_44.md` and
   `TASK_UI_PRIM_47.md`** — dated, attributed, **with neither file edited**, since
   a task file's acceptance criteria record what that task verified; the three
   updated assertions in `toggle.rs`/`scroll.rs`/`slider.rs` and **why the update
   strengthens them**; **the honest limit** — *the mechanism is verified by test
   and pure; no pointer event has ever been observed reaching this window on this
   host, so nothing here is evidence that a finger hovers a scoped control;* the
   six pages' frame rates; and the fact that **no demo file changed**, with the
   reason.
   `IMPLEMENTATION_STATE.md` is not a source of evidence
   (`.ai/workflows/task-sequence.md` § *State*); it points at the code.

10. **The tests, named, with no display, no network, no filesystem and no wall
    clock** — the only kind `AGENTS.md` permits. Each names the mutation it kills,
    because `developer.md` § Phase 3 (*"A test that has never failed is not a
    test"*) and *a survivor is a missing assertion* both require
    it.

    **In `scope.rs` — seventeen:**

    - **`a_scoped_subtree_resolves_the_override_and_the_rest_of_the_tree_resolves_
      the_theme`** — *the required one.* A container with two children and a
      sibling outside it; one override for `Surface` on the container; **both
      children resolve the override, the outside node does not, and for `Text` —
      which nobody overrode — all three resolve the theme.** Kills: a `resolve`
      that answers the map's first entry whatever the node is, and any
      implementation that puts the scope on a widget instead of on the tree.
    - `a_scope_that_overrides_one_token_leaves_every_other_token_on_the_theme` —
      **the partial-override case, requirement 1's rule 3**, asserted across **all
      thirty-four** tokens: exactly one differs from the theme. Kills a `resolve`
      that consults the override list for the wrong token.
    - `two_siblings_under_one_scoped_node_resolve_the_same_override`.
    - `a_node_resolves_the_override_set_on_itself` — kills a `resolve` that starts
      at `parent()`.
    - `the_nearest_scoped_ancestor_wins_for_the_token_it_overrides_and_the_scope_
      below_it_inherits_the_rest` — two nested scoped nodes, each overriding a
      different token; the deepest resolves **both**, each from the right place.
      Kills a `resolve` that returns the first match in insertion order.
    - `an_override_does_not_cross_a_boundary_into_a_different_root` — two roots;
      the override on one is `None`-equivalent on the other. Kills a walk that
      ignores `parent() == None`.
    - `an_override_for_a_node_the_arena_no_longer_holds_cannot_be_resolved`.
    - `retain_drops_every_override_for_a_node_the_arena_no_longer_holds` — kills
      the whole method, which is the `page_members` leak `TASK_UI_PRIM_47` records
      task 24.1 finding.
    - `set_override_reports_whether_the_scope_gained_or_replaced_an_entry` — kills
      both an always-`true` and an always-`false` `set_override`.
    - `clear_takes_every_override_off_the_subtree_below_it_and_reports_how_many_`
      it_dropped — kills a `clear` that leaves an entry, which would leave the
      panel's colour latched, and a `clear` that reports `1` whatever it dropped.
    - `a_scope_with_no_overrides_resolves_every_token_from_the_theme` — the
      empty-scope identity, over **all thirty-four** tokens.
    - `len_counts_overrides_and_not_scoped_nodes` — two nodes with two overrides
      each report **four**, not two.
    - **`resolve_property_returns_a_handle_that_follows_the_token_it_fell_through_
      to`** — *the mechanism's whole value in one assertion.* Resolve `Surface`
      **before** any switch, hold the handle, then `switch_to(Theme::light(), 100)`
      and `tick`; **the already-resolved handle has moved, and at half way it is
      the interpolation of the two themes' surfaces.** Kills a `resolve` that copies
      a value — which would pass every other test in this file, because every other
      one reads the value back immediately.
    - **`an_animated_theme_switch_does_not_move_an_overridden_token`** — *the
      required one.* An override for `Surface` set to a literal colour; `switch_to`
      and a full `tick`; the resolved value is **bit-identical** to the literal
      before, during and after, **while `Text` — overridden by nobody — arrives at
      the light theme's value in the same switch.** Kills a scope that re-resolves
      through the switch and a scope that drops the override at the transition's
      end. **This is the one a reviewer should break first.**
    - `an_override_bound_to_another_token_animates_with_that_token_and_not_with_
      its_own` — the override is `Property::bind` over the theme's `Primary`
      property, and the token asked for is `Surface`; **half way through the
      switch the resolved value equals the *interpolated* `Primary`**, which is
      neither theme's `Surface` and neither theme's `Primary`. Kills a bound
      override that is silently replaced by a static value.
    - `a_bound_property_over_a_resolved_handle_is_recomputed_on_every_frame_of_a_
      switch` — `Property::bind` over the resolved handle with an `on_change`
      counter, in `theme.rs`'s
      `a_switch_notifies_a_dependent_on_every_frame_it_moves` shape, asserting the
      **frame count and the final value**. This is the widget-side path end to end.
    - `a_handle_resolved_before_an_override_was_added_keeps_the_theme_token` —
      **pins the limitation in § *Inheritance semantics* rather than leaving it to
      be discovered**, with the remedy named in the test's comment.

    **In `theme.rs` — three, of which one is an update:**

    - **`dark_and_light_define_every_token`, updated to assert 34** (requirement 3),
      keeping its name and every other assertion.
    - **`every_token_is_answered_with_a_value_by_both_themes`** — kills a table
      that omits a row, which is the defect `from_table`'s doc describes and
      nothing tests.
    - **`the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes`** —
      kills any retune of the new token that would change a ring's colour, and it
      is the test the pixel criterion rests on.

    **In `toggle.rs`, `scroll.rs` and `slider.rs` — zero new tests and three
    updated assertions** (requirement 4), because the existing
    `the_palette_is_the_themes_*` tests already pin the ring exactly and the
    update is a token name. **`Segmented` gains nothing here either** if 47 landed.

11. **The suite, the capture and the frame rate are all produced**, by the
    commands of `.ai/tools/README.md` § *Capturing a window* and of the criterion below: `cargo fmt --check`,
    `cargo build --all-targets --all-features`,
    `cargo clippy --all-targets --all-features -- -D warnings`,
    `cargo test --all-features` with **the per-binary counts pasted**,
    `cargo doc --no-deps` clean, `cargo audit` **recorded as not installed on this
    host, not passed**; then the six-page before/after capture with **the window id
    re-read at the time of each capture**; then **the frame rate on all six
    pages**.

## Acceptance Criteria

- [ ] **`scope::ThemeScope` exists, holds only what a node overrides, and resolves
      by the nearest ancestor.** `grep -n 'pub struct ThemeScope'
      ui/src/ui_core/src/scope.rs` shows `overrides: Vec<(Handle, ThemeToken,
      Property<PropertyValue>)>` and nothing else;
      `grep -n 'pub mod scope' ui/src/ui_core/src/lib.rs` shows the declaration in
      alphabetical position. `grep -c 'pub struct\|pub enum' ui/src/ui_core/src/
      scope.rs` returns **1** — **the crate has no theme vocabulary of its own
      beyond `ThemeToken`, and that is the decision.**
      `a_scoped_subtree_resolves_the_override_and_the_rest_of_the_tree_resolves_
      the_theme` passes, and the handoff states in one sentence what the mutation
      was: make `resolve_property` return the first entry in the `Vec` whatever
      the node and token, and watch that test fail while the other sixteen stay
      green.

- [ ] **The three inheritance rules are the implementation, not the prose.**
      `a_scope_that_overrides_one_token_leaves_every_other_token_on_the_theme`
      asserts that across **all thirty-four** tokens exactly one differs from the
      theme; `the_nearest_scoped_ancestor_wins_for_the_token_it_overrides_and_the_
      scope_below_it_inherits_the_rest` asserts the nearest-wins half;
      `a_node_resolves_the_override_set_on_itself` and
      `an_override_does_not_cross_a_boundary_into_a_different_root` pin the walk's
      two ends. **And there is no "replace the whole theme" operation anywhere:**
      `grep -n 'fn .*theme' ui/src/ui_core/src/scope.rs` returns no method whose
      name suggests copying a theme, and `ThemeScope` holds **no `Theme` field**.

- [ ] **An override is a property, so a widget bound to it follows a switch with no
      per-frame work.**
      `resolve_property_returns_a_handle_that_follows_the_token_it_fell_through_to`
      resolves **before** the switch, holds the handle, and asserts it has moved —
      and that at half way it is the interpolation of the two themes' values.
      `a_bound_property_over_a_resolved_handle_is_recomputed_on_every_frame_of_a_
      switch` asserts the frame count and the arrival.
      **Mutation evidence in the handoff:** change `resolve_property` to return
      `Property::new(theme.get(token))` — a copy rather than a handle — and watch
      those two fail **while every other test in the file stays green**, because
      every other one reads the value back in the same breath it resolved it. That
      is the mutation a reviewer should run first.

- [ ] **An animated switch inside a scope behaves as specified, and the
      specification is a test.** `an_animated_theme_switch_does_not_move_an_
      overridden_token` passes: the overridden `Surface` is **bit-identical** to
      its literal before, during and after a full `switch_to`/`tick`, **while
      `Text` — overridden by nobody — arrives at the light theme's value in the
      same switch.**
      `an_override_bound_to_another_token_animates_with_that_token_and_not_with_
      its_own` passes for the one supported way to want the opposite.
      **The half-crossfade is stated in `scope.rs`'s module doc and in
      `resolve_property`'s**, because a subtree whose overridden tokens are pinned
      while the rest crossfade looks like a defect and is not one.
      **Mutation evidence:** make `resolve_property` ignore the override list
      whenever a transition is in flight — the "obvious" fix a well-meaning
      implementer would write — and watch this test fail on the bit-identity.

- [ ] **`retain` exists and drops a dead override.**
      `retain_drops_every_override_for_a_node_the_arena_no_longer_holds` passes.
      **Mutation evidence:** delete the `retain` body and watch it fail; restore it
      and watch it pass. The rule it encodes is task 24.1's `page_members` leak,
      and the method doc says so by name — this is `TASK_UI_PRIM_47`'s requirement,
      applied to the second map.

- [ ] **`TOKEN_COUNT` is 34, `const` and private, and the test asserts it.**
      `grep -n 'TOKEN_COUNT' ui/src/ui_core/src/theme.rs` shows
      `const TOKEN_COUNT: usize = 34;`, the `static ALL_TOKENS` declaration keyed
      to it, and **no `pub`** anywhere in the file for that name.
      `dark_and_light_define_every_token` keeps its name, its comment updated to
      *"all 34 tokens"* and its shape, and asserts
      `assert_eq!(ThemeToken::all().len(), 34)`.
      `grep -c '^    pub fn' ui/src/ui_core/src/theme.rs` returns **14** —
      `ThemeToken::all`, the five `PropertyValue::as_*` accessors, and `Theme`'s
      eight public methods (`new`, `dark`, `light`, `get`, `set`, `property`,
      `switch_to`, `tick`) — **unchanged from `HEAD`**, which is the "no public
      API change" claim made checkable. And
      `grep -n 'all 33 tokens' ui/src/ui_core/src/theme.rs` returns **nothing**,
      because that sentence in `PropertyValue`'s doc is the fifth edit requirement
      2 names and the one that is easiest to miss.

- [ ] **`FocusRing` is the only new token, and each of the six declines is
      recorded where a later agent will read it.**
      `grep -rn 'ThemeToken::Hover\|ThemeToken::Pressed\|ThemeToken::Shadow\|\
      ThemeToken::ZOrder\|ThemeToken::Active\|ThemeToken::Selected'
      ui/src/ui_core/src/` returns **nothing** — **six names, no variants, zero
      hits**, and that is the token half of the row's claim discharged by
      argument rather than by silence.
      `grep -n 'FocusRing' ui/src/ui_core/src/theme.rs` shows **the variant,
      its `ALL_TOKENS` entry, one row in each of the two theme tables, and its
      entry in the test module's `is_color_token` — six places and no sixth kind
      of place** — and `grep -n 'FocusRing' ui/src/ui_core/src/widgets/*.rs`
      shows **exactly the three `Palette::from_theme` mappings and the three
      updated assertions, and nothing else**, plus the same two in
      `segmented.rs` if `TASK_UI_PRIM_47` landed.
      `every_token_is_answered_with_a_value_by_both_themes` passes, which is the
      guard that makes adding a variant to `ALL_TOKENS` without a row in both
      tables a test failure rather than a silent fade to transparent black.
      **The token table from § *The token decision* is in `theme.rs`'s module doc
      in a form a reader can act on**, and the same six declines are in
      `IMPLEMENTATION_STATE.md` with their precedents named.

- [ ] **No per-widget property was removed, replaced or retyped — and the three
      ring re-pointings changed no colour.** `git diff --stat ui/src/ui_core/src/
      widgets/button.rs ui/src/ui_core/src/widgets/keyboard.rs ui/src/ui_core/src/
      widgets/text_input.rs` shows **nothing** for all three.
      `git diff ui/src/ui_core/src/widgets/toggle.rs ui/src/ui_core/src/
      widgets/scroll.rs ui/src/ui_core/src/widgets/slider.rs` shows **one changed
      line each** in `Palette::from_theme` plus **one changed assertion each**, and
      **no changed `pub struct Palette`, no added or removed field, and no changed
      test name.** `git diff` over the four palettes shows **no `pub` field added
      or removed and no `pub struct Palette` line touched**, which is what keeps
      `ui/src/ui_demo/src/main.rs`'s `tab_palette` struct literal compiling — and
      that literal is the mechanism, so the check is on the diff rather than on
      this file's confidence.
      **The one named decision this task records** — *a token is an appearance and
      a `Property<bool>` is a fact, so no state property is replaced by any token*
      — is in this file, in `theme.rs`'s module doc and in
      `IMPLEMENTATION_STATE.md`.

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: in `scope.rs` — the seventeen named
      above, in requirement 10's order; in `theme.rs` —
      `dark_and_light_define_every_token` (updated), `every_token_is_answered_
      with_a_value_by_both_themes`,
      `the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes`; and
      the three updated assertions in `toggle.rs`/`scroll.rs`/`slider.rs` under
      their existing names. **The suite is at least 1915** — the baseline 1894
      (1450 `ui_core` + 224 `ui_demo` + 220 doctests) **plus the seventeen
      `scope.rs` tests, the two new `theme.rs` tests and the two `scope.rs`
      doctests** — and **no test was deleted, renamed away or weakened**: the
      handoff lists the before and after counts per binary.
      **The handoff names the tree it measured on**, because four task files are
      open at once and
      *on a shared tree, the suite you ran is not your suite* is the rule that a
      number without a tree is not a result.
      `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is **recorded as not installed on
      this host, not passed**.

- [ ] **The six gallery pages are pixel-identical, and the mechanism is four facts
      rather than one comfortable one.** Captured **before and after** with the
      commands of `.ai/tools/README.md` § *Capturing a window* verbatim: window id
      **re-read at the time of each capture** with `xwininfo -root -tree` (a root
      capture and `ffmpeg x11grab` return black for a GL window), `pgrep -a -x
      ui_demo` in the same call as each `magick import -window <id>`, then
      `magick compare -metric AE before.png after.png null:` per page. **The
      criterion is the one tasks 34 to 47 inherited, unamended: AE 0 outside
      `y ≥ 680` on all six pages**, every differing pixel inside the fps readout's
      band.

      1. **No file under `ui/src/ui_demo/` changes at all.** `git diff --stat ui/
         src/ui_demo/` is **empty**. That is the strongest of the four facts and it
         is a fact about the diff rather than about this file's confidence.
      2. **`ThemeToken::FocusRing` holds exactly what `ThemeToken::Text` holds in
         each shipped theme**, asserted for both by
         `the_focus_ring_token_holds_what_the_text_token_holds_in_both_themes`.
         **Every consumer of the new token is a `Palette::ring` that read `Text`
         verbatim**, so every resolved colour is the same `Color` it was — and the
         three updated assertions prove each mapping individually.
      3. **No other palette moves a single token, and no widget property, node,
         rect, layout mode, paint order or batch key changes.** `git diff --stat`
         names only the ten files § *Scope* lists.
      4. **Even a capture taken half way through a theme switch cannot differ**,
         because `FocusRing` interpolates between the same two colours `Text`
         does — the new token's transition *is* the `Text` transition.

      **The rect-level half passes unamended, and that is a property of the
      change rather than of this file's confidence:** `every_page_places_every_
      rect_where_the_gallery_placed_it`, `no_two_placed_rects_overlap`,
      `assert_the_pages_partition_the_placed_rects` and
      `every_placed_rect_is_inside_the_window` compare rects and page membership,
      **none of which this task touches**, so all four hold without an edit.

- [ ] **The frame rate is measured on every page and reported.** With the script's
      own line pasted rather than the number expected:
      `.ai/tools/fps-check.sh 10 55` on the default page, and
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=<page>` for each of
      the six with the `roados-fps` line parsed by hand — **`fps-check.sh` takes
      `seconds` then `floor` and runs the binary with no arguments, so it cannot
      name a page**, which `.ai/tools/README.md` § *Frame-rate baseline* records
      as the reason task 24.2's criterion 6 was amended rather than met by the
      script. **Every page above the floor of 55**, and expected inside the
      recorded **61.1–63.9** band of `.ai/tools/README.md` § *Frame-rate baseline*.

      **And the per-frame cost claim is a fact about the diff, not a hope:** the
      demo's frame does exactly the work it did, because **no demo file changed and
      `ThemeScope` is never constructed in the demo**. The only new per-frame work
      anywhere is that a theme switch in flight animates **34** properties rather
      than 33 — one extra `Animation::tick` on a colour that moves exactly as
      `Text` does — and **the handoff says so in one sentence rather than
      attributing the numbers to luck.**

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **no change** to `ui/src/ui_core/src/property.rs`,
      `render.rs`, `render/target.rs`, `render/blur.rs`, `paint.rs`, `batch.rs`,
      `input.rs`, `layout.rs`, `arena.rs`, `node.rs`, `animation.rs`,
      `widgets/button.rs`, `widgets/keyboard.rs`, `widgets/text_input.rs`,
      `widgets/container.rs`, `widgets/label.rs`, `widgets/list.rs`,
      `widgets/dialog.rs`, `widgets/toast.rs`, `widgets/chart.rs`,
      `widgets/gauge.rs`, `widgets/image.rs`, `widgets/progress.rs` or
      `ui/src/ui_demo/src/main.rs`. **`property.rs` in particular is untouched**,
      because its false inheritance sentence is `TASK_UI_PRIM_47`'s requirement 3
      and duplicating a fix another task owns is
      *two documents each claiming ownership of one definition* with a patch.
      `grep -c unsafe` over `scope.rs` and over the three changed widgets is **0**,
      and `grep -n 'unwrap()\|expect(\|panic!\|unimplemented!\|todo!'` over
      `scope.rs` returns **nothing**. **`unwrap_or` is not `unwrap` and is used
      once**, in the module doc's doctest, in the shape `theme.rs` and six widget
      modules already use.

- [ ] **No other task's file was created or edited by this one.** `git diff
      --stat` plus `git status --short` name **exactly** these paths:
      `ui/src/ui_core/src/scope.rs` (new), and
      `ui/src/ui_core/src/theme.rs`, `ui/src/ui_core/src/lib.rs`,
      `ui/src/ui_core/src/widgets/toggle.rs`,
      `ui/src/ui_core/src/widgets/slider.rs`,
      `ui/src/ui_core/src/widgets/scroll.rs`, `doc/ui/DEMO_APPLICATION.md`,
      `doc/ui/PRIMITIVES_ARCHITECTURE.md`, `doc/ui/IMPLEMENTATION_STATE.md`
      (modified) — **plus
      `ui/src/ui_core/src/widgets/segmented.rs` if and only if
      `TASK_UI_PRIM_47` has already landed it**, in which case the handoff says so
      and its diff is one line plus one doc line. **Plus `doc/ui/
      TASK_UI_PRIM_50.md`**, which is **this file** and is untracked until the
      operator adds it, which is why both commands are named. **No `rotator.rs`,
      no `icon.rs`, no `tab_bar.rs` and no `mode.rs` or `snapshot.rs` appear**,
      whether or not `TASK_UI_PRIM_40`, 44, 47 or 43 has landed — **and where one
      of those has landed, this task's diff simply does not mention it**, which is
      the form the claim can take on a tree where four task files are open at once.
      **`ui/Cargo.toml` and `ui/Cargo.lock` are unchanged** — the approved direct
      dependencies remain `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, and
      **a `Vec` of triples, an ancestor walk and one enum variant are three places
      a dependency would have been a licence decision against GPLv3 that nobody
      asked for.**

- [ ] **The two documentation prerequisites were honoured, and the row is amended
      rather than closed.**

      - **`PRIMITIVES_ARCHITECTURE.md` § *Theme tokens* carries `FocusRing`** in
        its colour block, matching the declaration order, because `theme.rs`'s
        `ThemeToken` doc cites that section as the authority for that order.
      - **§ *Inheritance* names two inheritances**, `mode::ModeScope` and
        `scope::ThemeScope`, and says that `Property` has none — **and it was
        already corrected by `TASK_UI_PRIM_47` when this task started, which the
        handoff states as a checked premise rather than an assumption.** A §
        *Inheritance* still reading *"Some properties inherit from parent to
        child: `font_family`, `font_size`, `color`, `opacity`, `visibility`"* at
        the start of this task is a **dependency violation, not a variant of it.**
      - **Row `L9`** in `DEMO_APPLICATION.md`
        § *Gaps this layout exposes in `ui_core` carries a dated note naming
        `TASK_UI_PRIM_50`, stating that the **scoping** clause is closed and that
        **the token clause is narrowed to one token with six declines**, and
        **that the row is therefore not deleted and not marked closed** — because
        *"no tokens for … hover, pressed, shadow or z-order"* is still true and is
        now a recorded position. **Its evidence column's line numbers are replaced
        by symbol and file path**, because this task re-verified it and a citation
        into a file this task edits is the kind that is stale within days.
        **Row `L6b` is not edited**: its dated note already points at `L9` and that
        pointer now resolves.
      - **`TASK_UI_PRIM_44.md` and `TASK_UI_PRIM_47.md` are not edited**, and
        `IMPLEMENTATION_STATE.md` records, dated and attributed, that their
        `TOKEN_COUNT = 33` criteria were true when they were reviewed and are
        superseded by this task's 34.

- [ ] **The decisions are written down where the next agent finds them.**
      `scope.rs`'s module doc carries the map-not-stack decision with each rejected
      alternative and why, the three inheritance rules, **the switch semantics
      including the half-crossfade**, the bound-override route, the owner-resolves
      rule, and the `L6b`(a) hand-off by name; `theme.rs`'s module doc carries the
      token table with each decline's precedent in the crate's own words;
      `ui/src/ui_demo/src/main.rs` carries **nothing**, and the handoff says so
      rather than leaving a reader looking for a comment; and
      `doc/ui/IMPLEMENTATION_STATE.md` carries the record, **including the honest
      limit** — *the scope resolution and the switch semantics are verified by
      test and are pure; **no pointer event has ever been observed reaching this
      window** on this host, so nothing here is evidence that a finger hovers a
      scoped control or that a themed panel looks the way its owner intended.*

## Out of Scope

- **No demo producer for a scoped subtree.** `ui/src/ui_demo/src/main.rs` is not
  touched. **No subtree in the demo needs one today**, every themed surface is
  themed from the global tokens on purpose (`tab_palette`'s own doc works through
  why), and inventing a panel that wanted a different surface would be a
  parameter with no source — the failure `DEMO_APPLICATION.md` § *Asset
  requirements* records for the colours it could not verify. **The demo's producer
  is the task that has a surface needing it**, and by then `ThemeScope` and its
  seventeen tests are the thing it waits on. **This is task 46's decision shape
  by name**, and the honest limit — *the mechanism is library-side and verified by
  test; nothing on screen demonstrates it* — is in this file and in
  `IMPLEMENTATION_STATE.md`.
- **No mode propagation and no `L6b` clause (b) or (c).** `mode::ModeScope`,
  `snapshot::Snapshot` and `Segmented` are `TASK_UI_PRIM_47`'s, and **the fact
  that this task's `scope::ThemeScope` is a *second* inheritance beside 47's
  `ModeScope` is recorded here rather than merged.** **Merging the two into one
  generic type is refused**: `K` and `ThemeToken` are different resolutions — one
  is a caller-chosen value with no theme behind it, the other is a value the theme
  animates — and a type parameterising over both would be an abstraction with one
  caller and no second use, which is `developer.md` § *Phase 2*.
- **`TASK_UI_PRIM_47`'s two fixes are not duplicated.** `property.rs`'s module doc
  is not touched; § *Inheritance* is not corrected from scratch, only amended
  where 47 corrected it; and 47's `mode.rs`, `snapshot.rs` and `segmented.rs` rows
  in § *Module Layout* are left as 47 wrote them.
- **No `Grid`** — row `L3`, `TASK_UI_PRIM_52`. `ThemeScope` walks
  `WidgetNode::parent()` and reads nothing else about a node; it never lays
  anything out, never reads `layout()`, and has no `LayoutMode` anywhere.
- **No `TabBar`, no `Button::selected`, no `Selected` token** — gap `7`,
  `TASK_UI_PRIM_43`. The one widget with a genuine selected appearance has not
  landed, and a token nothing reads is dead code.
- **No `Icon`, no tint uniform, no `widgets/icon.rs`** — gap `#4`,
  `TASK_UI_PRIM_44`. `Icon` is ink and `ThemeToken::Text` is ink; 44's own
  requirement declines a token and says `L9` is a separate task, which this is.
- **No dark/light re-design, and no retune of any shipped value.**
  `FocusRing` is introduced **at** `Text`'s value in both themes, and the demo's
  direction — dark ships first, both required — is untouched. **Every other colour
  in both tables is byte-identical to `HEAD`.** Re-tuning a token to make the new
  one look better would move a pixel and would be a dark/light re-design by the
  back door.
- **No removal, replacement or retyping of any existing per-widget property.**
  `Button::focus_ring`, `Scroll::focus_ring`, `Slider::focus_ring` and
  `Keyboard::focus_ring` stay `Property<f32>`; `Button::focused`, `hovered`,
  `pressed`, `disabled`, `activatable`, `Toggle::disabled` and every other
  `Property<bool>` state stays exactly as it is. **The decision is recorded in §
  *What moves off per-widget properties* and in `theme.rs`'s module doc**, which is
  what `developer.md` § *API design*'s *"Respect semver"* and this file's
  out-of-scope both require of a decision not to do something.
- **No `Hover`, `Pressed`, `Shadow`, `ZOrder` or `Active` token.** Declined with
  the crate's own precedent each, in § *The token decision*'s table:
  `button.rs`'s `Palette` doc on derived state appearances,
  `MIN_TOUCH_TARGET`'s doc on *"a value a theme switch does not change"*,
  `keyboard.rs`'s `Palette` doc on six tokens for six colours, row `L1`'s
  evidence on a shadow whose colour is one constant in `render.rs`, **row `L2`'s
  evidence on a `DrawCommand` that carries no z**, and `Button`'s own absence of
  an active or selected state.
- **No `Theme` change beyond the one variant, the two rows and the count.** No
  field, no `Theme::push_scope`, no `Theme::pop_scope`, no scope-aware `Theme::get`,
  no scoped `switch_to`, no `Theme::clear`. **`Theme` remains one flat global map**,
  which is half of row `L9`'s own sentence and stays true by design.
- **No `TokenSource` trait, no generic parameter on any `Palette::from_theme`, and
  no scoped constructor on any palette.** Ten widget palettes and
  `ui/src/ui_demo/src/main.rs`'s `ButtonPalette` struct literal are the reasons,
  and `developer.md` § *Phase 2*'s *"No abstraction before the second use"* is the
  rule: one `Theme` impl would be the whole of the trait's use.
- **No `node::descends_from` promotion and no shared ancestor iterator.** The crate
  will hold **two** ancestor walks — `mode::ModeScope::resolve` and
  `ThemeScope::resolve_property` — and they answer different questions, so there is
  no seam to promote. **Recorded here as a later cleanup rather than smuggled in**,
  and adding one now would put `node.rs` in this task's file set for a third copy
  of a loop rather than for fewer copies.
- **No `Palette` field added, removed or retyped, in any widget.** `tab_palette`'s
  struct literal in the demo is the mechanism that makes this a stop condition
  rather than a preference, and requirement 4 names it.
- **No change to any pipeline mechanism**: no `DrawCommand` variant, no
  `ShaderKind`, no uniform, no `RenderCommand`, no batching key, no scissor, no
  mesh, no `Mat4`. `git diff --stat` over `render.rs`, `paint.rs` and `batch.rs`
  shows nothing.
- **No new page and no change to any of the six.** `Page::ALL` stays six,
  `Page::DEFAULT` stays `Pads`, the `--help` text is unchanged and no new
  `--tab=` name exists.
- **No new dependency.** Per `AGENTS.md` the approved direct dependencies remain
  `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`. **A `Vec` of triples, a walk
  and one enum variant are three places a dependency would have been a licence
  decision against GPLv3 that nobody asked for**, and `AGENTS.md`'s rule is the
  operator's to relax, not this file's. **No `unsafe` is added**, because
  `ThemeScope` is a `Vec` and the walk is an `Arena::get`; **and no `unwrap`, no
  `expect` and no `panic!`** in `scope.rs` or in the changed code.
- **No clock, no `tick`, and no animation of its own on `ThemeScope`.** A scope
  does not animate; **the properties it hands out animate**, through the theme's
  existing `switch_to`/`tick` and through whatever clock a caller gives an
  override it animated itself. A `tick` over an empty clock is a method that
  returns `false` forever, which is `TASK_UI_PRIM_40` requirement 1's recorded
  reason and it applies unchanged.