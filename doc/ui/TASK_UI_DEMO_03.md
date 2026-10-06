# TASK_UI_DEMO_03: The Car-Status Pane — the Pane, the Car, and Three Mutually Exclusive States

## Goal

Build **the car-status pane** as the seventh tab's content: a left-hand pane
**~40 %** of the window wide, holding a severity-ranked indicator column on its
left, a **3-D car standing on a floor** in the middle, a card row along its foot
and a page indicator under that — and put **three mutually exclusive states** on
it, **parked**, **driving** and **charging**, each with its own content, switched
by three keys, so the state difference is visible on screen and assertable in a
test.

Three things come with the pane and are settled here rather than left to a later
task: **the ambient rotation** of the car (which is task 40's, shared, and this
task adds no second producer), **the floor** the car stands on, and **Track
Mode's per-sub-mesh tint** — including the limit, stated in the pane's own
caption rather than only in a doc comment: **the model has one material, so
body-versus-wheels is the finest granularity the geometry gives.**

**The pane's width is a function, not a constant.** Every rect it owns is
derived by one pure function of one width, so the two-axis reshape
(`TASK_UI_DEMO_05`, § *The two-axis reshape*) is one argument and not a rewrite.
This task delivers the pane **at** its documented width and nothing moves it.

## Context

### What this depends on, and the fact that none of it is built

**Every library task below is specified and not built.** `doc/ui/IMPLEMENTATION_STATE.md`
§ *Current position* records task 31 as the last implemented task, and § *The
task table* lists **34 through 52** as *"specified 2026-10-05, not started"*.
**This task therefore cannot be started, and that is stated here rather than left
for a developer to discover at `cargo build`.** The table says what each task
gives this one; the last two rows say what this task needs from the two demo tasks
that precede it, and both of **those** files are also absent — which is recorded
rather than assumed away.

| task | what this task needs from it |
|---|---|
| `TASK_UI_PRIM_34` | the depth policy, and § *Face culling: off, and the reason recorded* — which decides that **culling is a mesh-pass property** and hands the decision to 37 by name. The floor reflection's limit below is a consequence of that decision |
| `TASK_UI_PRIM_35` | `MeshVertex`, the 32-byte stride, and the **five sub-mesh index ranges into one interleaved pair** that make one `MeshId` drawable in five commands |
| `TASK_UI_PRIM_36` | `Mat4` — `identity`, `rotated_x`, `rotated_y`, `translated`, `perspective`, `transform_point` — and **no `look_at`**, so the demo's composition is the camera's only construction |
| `TASK_UI_PRIM_37` | `DrawCommand::Mesh` and `Painter::mesh(mesh, range, mvp, tint, opacity, texture)`, `SubMeshRange`, the flat/Lambert shading, and § *The batching question, and the answer*'s **singleton pass**: the mesh is one command per sub-mesh, not batched with anything |
| `TASK_UI_PRIM_38` | the `ROADOSMF` format and its loader, and § *Task 38's file*'s **five names — `body`, `wheel-front-left`, `wheel-front-right`, `wheel-back-left`, `wheel-back-right` — and nothing finer**, with the node translations already baked into the geometry |
| `TASK_UI_PRIM_39` | the committed bytes: `ui/src/ui_demo/assets/sedan.roados` and `ui/src/ui_demo/assets/colormap.png`, and 66 baked icon PNGs under `ui/src/ui_demo/assets/icons/` |
| `TASK_UI_PRIM_40` | `Rotator` (`yaw`, `pitch`, `dragging`, `focused`), `ROTOR_SENSITIVITY`, `ROTOR_PITCH_LIMIT`, and the demo's `Demo::car`, `Demo::car_at`, `Demo::car_rect`, the ambient term in `Demo::frame` and the **camera constants** `CAR_TARGET_Y`, `CAR_DISTANCE`, `CAR_REST_YAW`, `CAR_REST_PITCH`, `AMBIENT_YAW_RATE` |
| `TASK_UI_PRIM_41` | **nothing.** `Painter::backdrop` and `BackdropMode` are the translucency of composite row 5, which is § *Out of Scope* here |
| `TASK_UI_PRIM_42` | **nothing.** `ui_core::nav::Screens` is gap `#3`'s closure and the seventh page arrives as `Page::Demo`, which is task 01's, not this task's. If 01 chose `Screens`, this task reads it and changes nothing |
| `TASK_UI_PRIM_43` | **nothing** for the pane itself. `Button::selected` and `TabBar` are the tab bar's, and the tab bar is 24.3's |
| `TASK_UI_PRIM_44` | `Painter::tinted_image` and the tint in `DrawCommand::Image`, which the **icon** channels of tasks 04 and 05 use. This task draws no icon |
| `TASK_UI_PRIM_45` | **the clip, and only the clip.** After it, the renderer reads `LayoutState::clip` per node and **`Demo::frame_clips` is gone** — so this task gives the pane's car its own node and inherits clipping, and writes nothing about `frame_clips`. That deletion is why this task's node list is a requirement rather than a convenience |
| `TASK_UI_PRIM_46` | **nothing.** `Scroll`'s axis, momentum and snap points are the card row's, and the card row is `TASK_UI_DEMO_05` |
| `TASK_UI_PRIM_47` | **`mode::ModeScope<K>`**, and this is the task that makes composite row 4's cross-widget half possible: Track Mode recolours the car body, which is a mesh the demo paints itself and no widget owns |
| `TASK_UI_PRIM_48` | **nothing.** `Margin` and `shrink` are available and unused; § *Out of Scope* says why |
| `TASK_UI_PRIM_49` | `DrawCommand::Text`'s width and `Painter::text_measured` / `text_in_measured` / `text_bold_measured`, which is what lets a readout be **clipped at the pane's edge** rather than running over it |
| `TASK_UI_PRIM_50` | **nothing.** `ThemeScope` is a theme-token mechanism; the tint here is a `Color` on a `DrawCommand::Mesh`, not a token |
| `TASK_UI_PRIM_51` | `paint::polygon_is_convex`, used by the floor's own test to state the floor is two convex quads and nothing else |
| `TASK_UI_PRIM_52` | **nothing.** `LayoutMode::Grid` is not used here; the pane is absolute placement, which is what the gallery already is |

**Two rows the table above cannot fill, and they are the honest answer rather
than a gap in the reading.** `TASK_UI_DEMO_01` — the seventh tab itself — and
`TASK_UI_DEMO_02` are **not in the tree**, so this file cannot cite what they
deliver. What this task needs from them is therefore stated as three structural
facts and one open question:

- **`Page::ALL` carries seven entries and `Page::DEFAULT` is still `Page::Pads`**
  — `DEMO_APPLICATION.md` § *What a seventh page costs* records both as
  requirements on task 01 (*"`const ALL: [Page; 6]` — a fixed-size array, so the
  *type* carries the count"*, and *"`Page::DEFAULT` stays `Pads` … The demo tab
  must not become the default"*). **This task adds no page** and must leave both
  facts as task 01 left them.
- **The seventh variant's name is task 01's**, and this file never spells it.
  Every signature below that needs it writes `Page::Demo`.
- **The `--tab=` spelling is task 01's too**, and § *Acceptance Criteria* names it
  as a placeholder to be filled from 01.
  **Filled 2026-10-05, from task 01 as written:** the variant is **`Page::Demo`**
  and the spelling is **`--tab=demo`**. `TASK_UI_DEMO_01` § *The page table*
  records both, and its own table row shows `fn name`'s seventh arm as
  `Page::Demo => "demo"` with `from_name`, `page_names` and `usage` deriving
  from `Page::ALL` — so one spelling reaches `--tab=`, `--help` and the
  unknown-name message. **If 01 is amended and either changes, this file's
  substitutions are wrong and this paragraph is the place that says so.**
- **Open, and left open:** whether task 01 put the pane's **root node** in the
  tree itself, or whether it created a container for this task to fill. **This
  task creates its own `pane` node and attaches it**, and if 01 already created
  one the implementer reuses 01's handle rather than adding a second — a second
  wrapper node over 01's container is not a defect anyone would notice on screen
  and it is one more node in the hit-test chain.

### The evidence this task cites, read by section and never by line

Every claim below is cited to `doc/ui/DEMO_APPLICATION.md` **by section name and
row**, and to `ui/src` **by symbol and path**.

- § *The car-status pane* — the pane is *"Left ~40 % in photo `02`, resizable"*;
  the **vertical structure** diagram places the indicator column at the pane's
  left, the map and the *"3-D car, floor reflection"* centre, the `Open Frunk` /
  `Open Trunk` callouts either side of the car, the 🔓 *"lock, floating above
  roof"* and the ⚡ *"charge port, rear-left"*, and the card row plus
  *"· · · (page indicator)"* at the foot.
- § *The car-status pane* — the column *"is a severity-ranked list, not a status
  strip"*, and the manual enumerates ~20 conditions with their colours and their
  *timing* semantics. **The conditions themselves are `TASK_UI_DEMO_04`'s**;
  this task builds the box they go in.
- § *Screen states of the car-status pane* — **three mutually exclusive states**,
  *"parked"*, *"driving (or ready to drive)"* and *"charging"*, and the parked
  bullet's *"Indicator lights flash briefly at power-up as a self-test and then go
  out"*.
- § *Screens* — the table's row *"Car-status pane"* reads *"persistent region of
  the map screen"* and *"never; resize drag only"* for how it is dismissed.
- § *Composite widgets* **row 4** — *"Track Mode recolours the car body by
  component temperature and tire grip [A]"*, and its demand is *"the state that
  decides a control's presentation must be separate from the control's value"*.
- § *Could not verify*, the row **"Track Mode's car-body recolouring rules"** —
  *"Named but not enumerated per component in anything read; treated here as [B]
  and **not** a spec the demo should copy until verified"*. **This task is
  bound by that sentence.**
- § *Could not verify*, the row **"Lane-guidance / junction-view rendering"** —
  *"Excluded from the demo's scope"*. Named here so the driving state's omissions
  below are not read as an oversight.
- § *Asset requirements* — *"Tesla publishes **no design tokens at all** — no
  colours, no spacing, no radii"*, and *"Every visual value in the demo is
  therefore a first-principles choice, and the asset inventory must produce them
  rather than transcribe them."* **Every colour and every rectangle below is
  derived, not transcribed, and each says which.**
- § *Design principles* — *"44dp touch targets"* is *"this project's number, not
  Tesla's"*, so no rectangle in this task is attributed to Tesla.
- § *What this means for the demo's shape*, item 1 — *"The demo must have a map
  it can put things on top of"* and *"gap #1 (the map widget …) is untestable
  without one"*. **There is no map in this repository**, which is why the pane's
  centre is an unfilled region and not a road.

### The pane's geometry, and every number in it derived

The pane's rects come from **one pure function of one width**, and the function is
what makes `TASK_UI_DEMO_05`'s reshape an argument rather than a rewrite. **The
numbers are derived in the file, cited to a photograph or to a constant, and
named as proposals where they are proposals.**

| constant | value | where it comes from |
|---|---|---|
| `PANE_WIDTH` | `WINDOW.width * 0.40` = **512.0** | § *The car-status pane*'s *"Left ~40 %"*, and `WINDOW` is 1280 wide. **Written as the product, not the product's value**, so the ratio is visible where the number is |
| `PANE_TOP` | `CONTENT_TOP` = **64.0** | task 24.2's `CONTENT_TOP`. **The pane starts under the tab bar and never over it** — `.ai/NEVERAGAIN.md` § *A container that covers the window swallows every tap aimed at anything behind it* is why the bar's band is not the pane's |
| `PANE_BOTTOM` | `FPS_READOUT_ORIGIN.1` = **684.0** | the frame-rate readout sits at `(60, 684)`, and **this page does not cover the instrument every other page shows.** A pane that ran to the window's foot would put 460 px of opaque fill over the readout |
| `PAGE_INDICATOR_HEIGHT` | **24.0** | the dots are *"· · ·"* — three marks, and a dot row a dot's diameter tall. **A proposal**, and the smallest height at which a 6 px disc with 9 px of air above and below fits |
| `CARD_ROW_HEIGHT` | **120.0** | the ASCII's card row is about a fifth of the pane's height. **A proposal**, from the photograph's proportions and nothing else |
| `INDICATOR_WIDTH` | **176.0** | **the one number here with a constraint behind it**: § *The car-status pane*'s *"severity-ranked list"* plus § *Asset requirements*'s *"the indicator column uses colour to encode severity"* means each row carries a **disc and a readable name**, so the column must fit a truncated name beside a disc. `TASK_UI_DEMO_04` inherits this width and is the reason it is not 96 |

**The car rect is the number task 40 measured against, moved.** `CAR_DISTANCE`
is metres and the visible height at that distance is `2 · 2.6 · tan(π/8) =
2.154 m` whatever the rect's pixel size — so the car's *angular* size is fixed and
its *pixel* size is the rect's height times `1.30 / 2.154 = 0.604`. **The pane's
rect is given task 40's aspect**, `336 / 238 = 1.412` against its `560 / 400 =
1.400`, **within 1 %**, so the projection is the one task 40 derived its framing
against: the car is 83.9 % of the rect's width and 60.4 % of its height, where
task 40 recorded 84 % and 60 %. **A pane at 336 px shows the car at 144 px tall
against `data`'s 242, and that is the whole difference between the two placements
— the perspective is identical and the rect is smaller.**

### Three states, and what differs between them

§ *Screen states of the car-status pane* gives each state its own content, and the
content **is** the difference. **Three rules make the difference checkable rather
than a picture.**

1. **The states are an enum, not a set of booleans**, so "mutually exclusive" is a
   type rather than a convention. `CarState` has three variants and the demo holds
   one value.
2. **Every readout belongs to exactly one state, and the state's own table lists
   it.** A readout is a `DemoLabel` with a row in exactly one state's list, and
   `every_readout_belongs_to_exactly_one_state_and_every_state_has_some` asserts
   it. **This is `.ai/NEVERAGAIN.md` § *A sweep of a mechanism's call sites is not
   a sweep of the data it is built from*, aimed at the table rather than the code** —
   the demo's page-membership table has already been caught twice by a missing row,
   and a state's readout list is the same shape of data.
3. **Range is in all three states and nothing else is.** § *Screen states* names
   range under parked *and* under driving, and names no shared readout for
   charging — so charging shows the charge state and the range, and that is the
   union this task implements rather than an invention.

| | parked | driving | charging |
|---|---|---|---|
| indicator column | 04's, on every state | 04's | 04's |
| the car, the floor, the contact shadow | shown | shown | shown |
| drive mode | shown | not shown | not shown |
| estimated range | shown | shown | shown |
| speed | not shown | shown | not shown |
| set cruising speed | not shown | shown, and a dash when unset | not shown |
| charge-port lamp | not shown | not shown | **shown, with its protocol colour** |
| charge state | not shown | not shown | shown |
| Track Mode's tint | mode-scoped, so it applies on all three | | |

**The set cruising speed's dash is a decision, and it is the one that earns the
state machine.** § *Screen states* names *"the set cruising speed"* as driving's
content and nothing says what it reads before one is set. **A dash and a number
are two renderings of one readout**, so this task stores `Option<u32>` and renders
the `None` as an em dash — which is what makes "the readout is present in this
state" and "the readout has a value" two different questions, and the second is
not reachable from a `Label` whose text is a `String` the demo rewrites.

### The floor: what the photograph shows and what the pipeline can draw

§ *The car-status pane*'s diagram says *"3-D car, floor reflection"*, and
§ *Open questions* item 5 records the reflection as part of *"a rendered vehicle
that hotspots anchor to … and that is reflected on the floor in the operator's own
photographs"*. **The mirrored image cannot be drawn by the pipeline as specified,
and the reason is one line and it is in this repository.**

**A reflection about the floor is an orientation-reversing transform, and the
front-face convention is window-space.** `TASK_UI_PRIM_34` § *Face culling: off,
and the reason recorded* decides that culling is a mesh-pass property and hands it
to 37; `TASK_UI_PRIM_37` requirement 6 brackets the mesh draws with
`gl.enable(GL_CULL_FACE)`, `gl.cull_face(GL_BACK)`, `gl.disable(GL_CULL_FACE)`
afterwards, and leaves **front face at GL's default `GL_CCW`** with no
`front_face` call. **`GL_CCW` is a statement about window coordinates**, so a
mirrored model arrives with its screen-space winding reversed, its front faces
classified as back faces, and culled: the reflection would render as the car's
interior. Folding the reflection into the view matrix instead of the model changes
**no pixel** — `projection · view · reflect` is one matrix either way — and does
not change the winding, because the winding is a property of where the triangle
lands, not of which matrix put it there.

**Three ways out, and all three are pipeline work, not demo work:** a `front_face`
call per mesh batch, a per-command winding flag on `DrawCommand::Mesh`, or a
per-command cull toggle. **None exists, and this task does not add one** — it is a
`TASK_UI_PRIM_n` item, and requirement 10 records it as **row `L12`** of
`DEMO_APPLICATION.md` § *Gaps this layout exposes in `ui_core`*, appended after
`L11` so that **no existing citation of `L1..L11` breaks**, which is that table's
own numbering rule.

**What this task draws in its place, and what it is called.** The pane's centre
carries a **floor**, and the floor carries a **contact shadow**:

- the floor is **two convex `DrawCommand::Rect`s** — a horizon band and a floor
  band — because **`Polygon` is convex-only** (`L10`, and `TASK_UI_PRIM_51`'s
  `paint::polygon_is_convex` is what asserts they are convex), and because
  **no ground geometry exists in this repository**: the only mesh 38 and 39 commit
  is the car, so a ground quad would have to be authored as a second `.roados`
  and that is an asset task.
- the contact shadow is one **`DrawCommand::Shadow`**, whose rect is derived from
  the car's own rect by three named fractions, because `DrawCommand::Shadow` is
  the crate's one primitive that blurs for free.
- **The shadow's footprint does not foreshorten with the yaw, and that is the
  honest limit.** A correct contact shadow is the projected ground quad, which is
  the same two-triangle mesh the reflection needs and therefore the same `L12`
  row. **It is drawn for the rest pose**, and the pane's caption says so.

### Track Mode's recolouring is `[B]`, and what is implemented instead

§ *Composite widgets* **row 4** records Track Mode recolouring the car body at
`[A]`, and § *Could not verify* records that the **rules are not enumerated per
component in anything read**, are `[B]`, and are *"not a spec the demo should copy
until verified"*. **This task therefore copies no colour rule.** What it
implements is the **mechanism composite row 4 demands** — *a mode that reaches a
thing it is not set on* — and the **limit the mechanism is limited to**, both of
which are checkable and neither of which requires the unverifiable rule:

- **The granularity is `body` against the four `wheel-*` ranges, and nothing
  finer.** `TASK_UI_PRIM_38` names five sub-meshes and its file's own text records
  that a node transform distinguishes *"the `body` against the four `wheel-*`
  meshes **and nothing finer**"*, because **the model carries one material and one
  texture** — `TASK_UI_PRIM_39`'s *Upstream one* records *"One material
  `colormap`, one texture, one image"*. A per-component-temperature or per-tyre-grip
  tint needs sub-meshes that do not exist.
- **The tint multiplies the colormap rather than replacing it.** `Painter::mesh`
  takes `tint` and the fragment shader is `mesh_fragment(texel, tint, shade)`, so a
  tinted car is Kenney's own colours modulated. **A tint cannot make a region of
  the car a colour the colormap does not already carry**, and a reader who expects
  it to is misreading what `tint` is.
- **The two tints are theme tokens, not constants**, and the reason is
  § *Asset requirements*: *"Tesla publishes no design tokens at all"* and every
  visual value is *"a first-principles choice"*. So the body takes the theme's
  `Warning` and the wheels its `Primary`, and a theme switch moves both — **the
  demo demonstrates that a mode crosses a widget boundary and reaches a `Color` on
  a `DrawCommand::Mesh`, which is composite row 4's demand, and asserts the two
  differ.**

**And the limit is on the screen.** The car area carries a one-line caption
naming the tint's granularity — *"one material · body and wheels"* — because §
*Could not verify* is a sentence in a document and a caption is a sentence a
capture photographs. **A limit that lives only in a doc comment is invisible in
the one piece of evidence this repository produces for a rendering change**
(`developer.md` § Phase 3, *"A change that alters what is on screen is not
verified until it has been seen"*).

### The ambient rotation is one producer of one value, and this task adds none

`TASK_UI_PRIM_40` § *Two producers, one value* puts the ambient turn in
`Demo::frame` as `if !self.rotator.dragging.get() { self.rotator.rotate_by(
AMBIENT_YAW_RATE * delta.as_secs_f32(), 0.0) }` on the `data` page, and
`Rotator::rotate_by` is *"the ONLY thing that writes either property"*.

**The pane's car is the same car.** One `Rotator`, one `MeshId`, one
`AMBIENT_YAW_RATE`, one ambient term in `Demo::frame`, and the pane's car is
**one node with a second placement**. Three reasons, and the third is the one
that settles it:

1. **Two rotators would be two values for one car.** The pane and the `data` page
   would disagree about which way the car faces, and there is no way to tell which
   is right.
2. **Two ambient terms would double the frame's rotation work for one visible
   change**, and `AMBIENT_YAW_RATE` is already a per-frame write.
3. **`NEVERAGAIN`'s "a second copy of a decision" rule applies to the constants
   too**, which is why requirement 1 renames task 40's *placement* constants to
   `DATA_CAR_ORIGIN` / `DATA_CAR_SIZE` rather than adding a second pair beside
   them. **The camera constants are shared and are not renamed** — one distance,
   one target height, one rest pose.

## Requirements

1. **Rename task 40's two placement constants and every site that names them.**
   `CAR_ORIGIN` becomes **`DATA_CAR_ORIGIN`** and `CAR_SIZE` becomes
   **`DATA_CAR_SIZE`** in `ui/src/ui_demo/src/main.rs`, and this task's own pair
   takes the plain names. `CAR_DISTANCE`, `CAR_TARGET_Y`, `CAR_REST_YAW`,
   `CAR_REST_PITCH`, `AMBIENT_YAW_RATE` and `CAR_RECT` are **unchanged and
   shared**. Before editing, run
   `rg -n 'CAR_ORIGIN|CAR_SIZE|CAR_RECT|CAR_DISTANCE|CAR_TARGET_Y|CAR_REST_YAW|CAR_REST_PITCH|AMBIENT_YAW_RATE' ui/src/ui_demo/src/main.rs`
   and carry every hit into the edit; a hit this list misses is a site the rename
   breaks and `cargo build` will not catch if the name resolved through a `use`.
   The reason is § *The ambient rotation*: one car, one pair of names, and two
   pairs of names is `.ai/NEVERAGAIN.md`'s second-copy rule with a different
   spelling.

2. **The pane's constants, in `ui/src/ui_demo/src/main.rs`, each `const`, each
   doc-commented with its derivation from the § *The pane's geometry* table and
   with the sentence that it is a proposal where it is one:**

   ```rust
   const PANE_WIDTH: f32 = WINDOW.width * 0.40;
   const PANE_TOP: f32 = CONTENT_TOP;
   const PANE_BOTTOM: f32 = FPS_READOUT_ORIGIN.1;
   const PANE_HEIGHT: f32 = PANE_BOTTOM - PANE_TOP;
   const INDICATOR_WIDTH: f32 = 176.0;
   const CARD_ROW_HEIGHT: f32 = 120.0;
   const PAGE_INDICATOR_HEIGHT: f32 = 24.0;
   const CARD_ROW_TOP: f32 = PANE_BOTTOM - CARD_ROW_HEIGHT - PAGE_INDICATOR_HEIGHT;
   const BODY_HEIGHT: f32 = CARD_ROW_TOP - PANE_TOP;
   const CAR_AREA_WIDTH: f32 = PANE_WIDTH - INDICATOR_WIDTH;
   const CAR_SIZE: Size = Size { width: CAR_AREA_WIDTH, height: 238.0 };
   const CAR_ORIGIN: (f32, f32) = (
       PANE_TOP * 0.0 + INDICATOR_WIDTH,
       PANE_TOP + (BODY_HEIGHT - CAR_SIZE.height) * 0.5,
   );
   ```

   **`CAR_ORIGIN.0`'s `PANE_TOP * 0.0 +` is not written** — it appears above only
   to make the derivation auditable; the shipped expression is
   `INDICATOR_WIDTH`. **`CAR_SIZE.height` is the literal `238.0` and not
   `CAR_AREA_WIDTH / 1.412`, and the doc comment says why**: the ratio is the
   *consequence* of matching task 40's aspect, and writing the division would make
   a reader believe the ratio is free. **`238.0` is chosen so
   `CAR_SIZE.width / CAR_SIZE.height = 1.4118`, within 1 % of task 40's `1.400`**,
   and the comment carries the `2 · 2.6 · tan(π/8) = 2.154 m` derivation and the
   83.9 % / 60.4 % figures.

3. **One pure function produces every rect the pane owns, from one width.**

   ```rust
   /// Every rect the car-status pane owns, derived from its width alone.
   ///
   /// **One width in, seven rects out, and no other input** — no theme, no
   /// state, no clock. That is what makes `TASK_UI_DEMO_05`'s reshape a single
   /// argument rather than a rewrite: the pane has no other geometry to
   /// re-derive, because there is no other geometry.
   ///
   /// **Which is also why this function cannot be a method on `Demo`.** It takes
   /// no handle and reads no property, so it is `#[must_use]`, it is free of GL
   /// and of `Arena`, and every assertion about the pane's layout is a call to it
   /// rather than a frame. `.ai/NEVERAGAIN.md`'s *A test fixture that builds what
   /// production does not define* runs the other way here: the production
   /// construction **is** the tested one.
   #[must_use]
   fn pane_rects(width: f32) -> PaneRects

   struct PaneRects {
       /// The pane's own box: `Rect::new(0.0, PANE_TOP, width, PANE_HEIGHT)`.
       pane: Rect,
       /// The indicator column, `TASK_UI_DEMO_04`'s box. **Empty in this task**
       /// and drawn as the pane's surface, so the hole is visible rather than a
       /// silent absence.
       indicators: Rect,
       /// The band above the card row: the indicator column and the car area.
       body: Rect,
       /// The car area: `PANE_TOP..CARD_ROW_TOP`, right of the column.
       car_area: Rect,
       /// The card row, `TASK_UI_DEMO_05`'s host. **A filled surface and
       /// nothing else in this task** — see § *Out of Scope*.
       card_row: Rect,
       /// The page-indicator band under the card row.
       dots: Rect,
       /// The charge-port lamp, `Size` 12 × 12 at the car's rear-left
       /// quarter. **In the charging state only**, and 05's charge-port hotspot
       /// is offset **above** it so the two never claim the same pixels.
       charge_lamp: Rect,
   }
   ```

   `width` is used **as given, unclamped**, and `pane_rects(0.0)` is a zero-width
   pane rather than an error — `Rect::new` and the arithmetic do not panic, and a
   clamped width would hide a caller that passed one. A test pins both halves.

4. **`enum CarState`, in `main.rs`, beside `enum Page` and not inside it.**

   ```rust
   /// Which of the car-status pane's three mutually exclusive states is on show.
   ///
   /// **Three, and an enum rather than three booleans** — § *Screen states of the
   /// car-status pane* names them as mutually exclusive, and a type is the only
   /// representation of "mutually exclusive" a reader does not have to take on
   /// trust.
   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   enum CarState { Parked, Driving, Charging }
   ```

   with `const ALL: [CarState; 3]`, `const DEFAULT: CarState = CarState::Parked`
   — **named, not the enum's first variant**, on `Page::DEFAULT`'s argument, which
   is that a variant inserted at the top must not change what a capture with no
   flag photographs — `#[must_use] fn key(self) -> Keycode`, `#[must_use] fn
   name(self) -> &'static str`, and `Demo::car_state: CarState` in `Demo`'s
   existing field style. **`CarState` does not derive `Default`**, because
   `DEFAULT` is a named constant and a `Default` impl would be a second place to
   change it.

5. **Four rows in `GALLERY_SHORTCUTS`**, so the array becomes
   `[GalleryShortcut; 23]` and the new rows are `Keycode::Z` (parked),
   `Keycode::X` (driving), `Keycode::V` (charging) and `Keycode::B` (Track Mode),
   each with a `Some(Page::Demo)` page and a `fn(&mut Demo)` handler.
   **All four keys are free in the table today** — `Space`, `T`, `+`, `=`, `-`,
   `C`, `Y`, `0`, `1`, `F`, `[`, `]`, `P`, `,`, `.`, `G`, `H`, `A`, `S` are taken,
   and `D` and `K` are `DIALOG_KEY` and `TOAST_KEY` outside it — and each row's
   label names the key the way the existing nineteen do (*"Z, which shows the
   pane parked"*).
   **This is `no_printable_key_acts_without_a_row_in_the_shortcut_table`'s
   obligation, not a courtesy:** a key that acts without a row passes the whole
   suite and does nothing visible on a page that is not showing.

6. **`Demo::set_car_state(&mut self, state: CarState) -> bool`, one write path,
   in that order and no other:** `if self.car_state == state { return false; }`,
   then `self.car_state = state`, then the readouts' visibility sync (requirement
   7), then `self.sync_page_visibility()`. It returns whether it moved.
   **`sync_page_visibility` is called here and nowhere else new** — the page gate
   is what makes a state change visible at all, and `.ai/NEVERAGAIN.md`'s
   2026-10-04 entry records `show_page`'s own `sync_page_visibility` call as
   *"a mechanism with no test"* found by mutation, so this call site gets a test
   in requirement 11 by name.
   **`TASK_UI_DEMO_04` adds exactly one line to this function** — the self-test
   arm — and this task must leave the function so that a one-line addition needs
   no change to it.

7. **`struct CarReadouts` and the per-state table, both in `main.rs`.**

   ```rust
   /// The pane's readouts. **`Option` where a state's own table decides
   /// whether the readout is on show at all**, and `Option<u32>` where the
   /// readout is shown and has no value yet — two different questions, and
   /// § *Three states* is why the demo keeps them apart.
   struct CarReadouts {
       drive_mode: Option<DemoLabel>,
       range_km: Option<DemoLabel>,
       speed_kmh: Option<DemoLabel>,
       set_speed_kmh: Option<DemoLabel>,
       charge_state: Option<DemoLabel>,
   }
   ```

   and

   ```rust
   /// Which readouts each state shows, by index into [`CarReadouts`]'s
   /// declaration order.
   ///
   /// **One table, read by the visibility sync and by the completeness
   /// assertion**, because two readers of one list is the arrangement
   /// `Page::ALL` already uses for the six pages and the arrangement
   /// `NEVERAGAIN`'s missing-row entry is about when there are two.
   const STATE_READOUTS: [[bool; 5]; 3] = [
       // Parked:   drive mode, range.
       [true,  true,  false, false, false],
       // Driving:  range, speed, set cruising speed.
       [false, true,  true,  true,  false],
       // Charging: range, charge state.
       [false, true,  false, false, true ],
   ];
   ```

   `Demo::readout_visible(&self, index: usize) -> bool` reads the row for
   `self.car_state` and `Demo::car_readout_rect(index)` places each readout in
   `PaneRects::body`'s right column, in declaration order, at
   `READOUT_PITCH` apart. **The em dash for an unset `set_speed_kmh` is written
   by a private `fn set_speed_text(&mut self) -> bool`**, which guards on the
   property's value so a write that changes nothing re-lays out nothing — the same
   guard `tick_fps` uses and for the reason its comment gives.

8. **The nodes, and a `PageMember` row and a `placed_handles` row for each.**
   `Demo` gains `pane: DemoPane`, and

   ```rust
   /// The car-status pane's nodes and the car it stands on.
   ///
   /// **`DemoSlider`'s shape**: a wrapper whose `node()` delegates to the inner
   /// widget's handle, so `placed_handles`, `on_show` and `focus_navigation` read
   /// it the way they already read the slider's.
   struct DemoPane {
       /// The pane's root. **Attached to `Demo::root` as its own child**, so the
       /// pane's nodes are inside the page tree rather than beside it.
       root: Handle,
       /// The indicator column's box — `TASK_UI_DEMO_04`'s, empty here.
       indicators: Handle,
       /// The car area, and the node the mesh is recorded on.
       car: Handle,
       /// The card row's host — `TASK_UI_DEMO_05`'s.
       cards: Handle,
       /// The page-indicator band — `TASK_UI_DEMO_05`'s dot pager.
       dots: Handle,
       /// The car, in `DemoCar`'s shape from `TASK_UI_PRIM_40`, so the rotator
       /// is shared rather than duplicated. `Demo::car` already exists and is
       /// **not** replaced.
       car_node: Handle,
   }
   ```

   with `impl DemoPane { #[must_use] fn node(&self) -> Handle { self.root } }`.
   **Every one of the six handles gets a `PageMember { page: <the seventh
   variant>, focusable: false, always: false }` row in `Demo::new` and a
   `("pane …", handle)` row in `Demo::placed_handles`**, and
   `expected_placed_rect_names` gains all six names. **This is the obligation
   `.ai/NEVERAGAIN.md`'s 2026-10-04 entry makes twice** — the page table's
   completeness assertion and `placed_handles` are the two instruments that catch
   a dropped row, and both were caught green before.
   **`car_node` is a second placement of the same car**, so its rect is
   `Rect::new(CAR_ORIGIN.0, CAR_ORIGIN.1, CAR_SIZE.width, CAR_SIZE.height)` and
   **`Demo::car_at` is not changed** — the `data` page's car keeps its own hit
   target and its own rect.

9. **The pane's paint, on the seventh page only, in this recording order and no
   other**, because it is the ordering contract `TASK_UI_PRIM_37` § *Where the
   mesh pass sits, and the ordering problem it solves* recorded — *"record the map
   before the mesh and the chrome after it"*:

   1. the pane's surface, one `Painter::rounded_rect` on `PaneRects::pane` in
      the theme's `Surface`;
   2. **the map region, one `Painter::rect` on `PaneRects::body` in
      `ThemeToken::Background`** — **the pane's centre is unfilled, and this
      command is the hole made visible.** § *What this means for the demo's shape*
      item 1 records that gap `#1`, the map, as *"untestable without"* one and as
      a `TASK_UI_DEMO_n` item; a map-less region drawn in the theme's own
      background is a surface the next task paints over, and a region left
      **transparent** is a hole in the page a reader has to diagnose;
   3. the floor, two `Painter::rect`s, horizon then ground;
   4. the contact shadow, one `Painter::shadow`;
   5. **the car, five `Painter::mesh` calls in `Model::ranges`' file order** —
      `body`, `wheel-front-left`, `wheel-front-right`, `wheel-back-left`,
      `wheel-back-right` — **all five carrying the same `mvp`**, because the node
      translations are baked into the geometry by task 38 and so **there is no
      per-sub-mesh transform and no wheel spin**;
   6. the charge-port lamp, **in the charging state only**, one
      `Painter::circle` on `PaneRects::charge_lamp`;
   7. the Track Mode granularity caption, one `Painter::text_in_measured`;
   8. the readouts, each on its own node;
   9. the card row's surface and the dots band's surface — **two surfaces and no
      content**, so 05's host is a box with a fill and not a gap.

   **The matrix composition is the one `TASK_UI_PRIM_40` requirement 10 wrote
   out**, in `Demo`'s own private helper, unchanged except that the projection's
   `aspect` is `CAR_SIZE.width / CAR_SIZE.height` of the pane's rect:

   ```rust
   let [eye_x, eye_y, eye_z] = orbit(yaw, pitch, CAR_DISTANCE);
   let view = Mat4::identity()
       .rotated_x(pitch)
       .rotated_y(yaw)
       .translated([-eye_x, -eye_y, -eye_z]);
   let model = Mat4::identity();
   let mvp = projection.multiply(&view).multiply(&model);
   ```

   `projection` is `Mat4::perspective(FRAC_PI_4, CAR_SIZE.width / CAR_SIZE.height,
   0.1, 20.0)`, **and the `Option` is handled and not unwrapped** — 36 returns
   `None` on every violated precondition, so the helper returns early with a
   printed reason rather than drawing with a stale matrix.
   **`Demo::frame`'s ambient term is not re-added**: the rotator is shared and
   already turns once per frame.

10. **`Row L12` is appended to `doc/ui/DEMO_APPLICATION.md` § *Gaps this layout
    exposes in `ui_core`*,** dated and attributed to this task, severity **Low**,
    Blocks *"a mirrored or foreshortened copy of any mesh — a car reflected on a
    floor, a contact shadow that tracks the yaw"*:

    > **No winding or culling control on a mesh draw.** `DrawCommand::Mesh`
    > carries `mesh`, `range`, `mvp`, `tint`, `opacity` and `texture` and **no
    > winding flag and no cull toggle**, while `draw_mesh_batch` brackets its
    > draws with `GL_CULL_FACE` on and `GL_BACK` and leaves the front face at GL's
    > default `GL_CCW`. **The front-face convention is window-space**, so an
    > orientation-reversing transform — a reflection, a mirror, a negative scale
    > — classifies the geometry's visible faces as back faces and culls them.
    > *Evidence:* `DrawCommand::Mesh` in `ui/src/ui_core/src/paint.rs`; the
    > bracket and the `GL_CCW` default in `draw_mesh_batch` in
    > `ui/src/ui_core/src/render.rs`; the policy in `TASK_UI_PRIM_34` § *Face
    > culling: off, and the reason recorded*.

    **Appended after `L11`, and no existing row is renumbered or closed** — that
    table's own rule is that the rows are cited as `L1..L11`, and a renumber
    breaks every citation. **The reversal is specified to the point of being
    built**: one `bool` on `DrawCommand::Mesh`, one `gl.front_face` bracketed
    inside `draw_mesh_batch`'s existing bracket, and the two-axis reshape's
    reflection plus this task's foreshortened shadow then become two calls.

11. **The tests, in `main.rs`'s existing `mod tests`, beside the code, with no
    display, no network, no filesystem and no wall clock** — the only kind
    `AGENTS.md` permits. **`demo_on(Page::Demo)` is the fixture**, the
    one `demo_on` already provides, and `Demo::new(mono_metrics(),
    demo_fonts(), None, page).unwrap()` needs no window — which is what makes a
    state switch assertable at all on a host where no pointer event arrives.

    - **`pane_rects_agrees_with_every_constant_it_is_built_from`** — seven rects
      at `PANE_WIDTH`, each asserted against the constant that produced it:
      `pane` is `PANE_WIDTH × PANE_HEIGHT` at `(0, PANE_TOP)`, `indicators` is
      `INDICATOR_WIDTH` wide and `BODY_HEIGHT` tall, `card_row` is
      `CARD_ROW_HEIGHT` tall at `CARD_ROW_TOP`, `dots` is `PAGE_INDICATOR_HEIGHT`
      tall and its top is `card_row`'s bottom, **`car_area` is the mirror of
      `indicators` about the pane's vertical centre line**, and `charge_lamp` is
      inside `car_area`.
    - **`pane_rects_moves_every_rect_when_its_width_moves`** — `pane_rects` at
      `PANE_WIDTH`, at `WINDOW.width` and at `0.0`; **the four left-anchored rects
      do not move and the three right-anchored ones do**, and the assertion is
      written as a *pair* because a test that only checks the moving rects passes
      against a function that moves everything and a test that only checks the
      fixed ones passes against one that moves nothing. **This is
      `TASK_UI_DEMO_05`'s precondition and it is asserted here**, because the
      function is written here.
    - **`the_pane_reaches_neither_the_tab_bar_nor_the_readout_band`** — every one
      of `pane_rects`' seven rects is inside `Rect::new(0.0, CONTENT_TOP,
      WINDOW.width, FPS_READOUT_ORIGIN.1 - CONTENT_TOP)`. **The mirror of
      `nothing_the_demo_places_reaches_into_the_strip`**, which pins the tab bar's
      premise rather than the guard.
    - **`the_car_rect_matches_task_forty_aspect_within_one_percent`** — the
      assertion is on `CAR_SIZE.width / CAR_SIZE.height` against `560.0 / 400.0`,
      and its failure message prints both ratios, **because the ratio is the claim
      and the pixel count is not.**
    - **`every_state_shows_its_own_readouts_and_shares_only_the_range`** — over
      `CarState::ALL`: each state's row is read from `STATE_READOUTS`, every index
      in a row is `true` for at least one state, **no index is `true` in two
      states' rows except the range**, and every `Option` in `CarReadouts` is
      `Some` **iff** its index is `true` in some row. **The complement is
      asserted, not the membership** — `NEVERAGAIN`'s rule, and the reason: a row
      that is missing is trivially "not a member" and every membership-phrased
      assertion passes.
    - **`switching_state_moves_the_gates_and_nothing_else`** — a `Demo` on the
      seventh page, one `set_car_state` call per state, asserting after each that
      the readouts' `LayoutState::visible` flags match the row, that
      `on_show` is false for a readout the state does not show, and that **the
      `data` page's own handles have not moved** — `Demo::car_rect()` is
      unchanged across all three, which is the one-car argument in § *The ambient
      rotation* made checkable.
    - **`the_pane_records_five_mesh_commands_and_no_more`** — after one frame on
      the seventh page, the commands recorded on `DemoPane::car` are counted and
      **exactly five are `DrawCommand::Mesh`**, and the five `SubMeshRange`s are
      the model's own in file order. **A count, beside its control**: the mirror of
      *`A test fixture that builds what production does not define*'s cheapest
      check, and the count that would catch a duplicated or dropped sub-mesh.
    - **`the_car_is_recorded_before_the_chrome_that_sits_over_it`** — the
      recorded command vector for the seventh page, asserting the first
      `DrawCommand::Mesh`'s index is **less than** the index of the Track Mode
      caption's `DrawCommand::Text` and of the card row's surface.
      **`TASK_UI_PRIM_37`'s ordering contract asserted on the demo's own recorded
      list**, which is the shape its § *The batching question* entry demands
      rather than a comment.
    - **`the_floor_is_two_convex_rects_and_the_shadow_is_one`** — the recorded
      floor commands are two `DrawCommand::Rect`s whose four corners each pass
      `paint::polygon_is_convex`, and exactly one `DrawCommand::Shadow` follows
      them. **This is `L10`'s precondition used rather than restated**, and `L12`
      pinned by a test that asserts what is *not* drawn: **no command on the car
      node carries a mirrored matrix, and the shadow's rect is the rest pose's.**
    - **`track_mode_tints_the_body_and_the_wheels_differently`** — one frame with
      `DemoMode::Standard` and one with `DemoMode::Track`, reading the `tint` off
      the `body` command and off one `wheel-*` command in each: **equal in
      `Standard`, both the premultiplied white of task 40, and different in
      `Track`**, with the body equal to the theme's `Warning` and a wheel to its
      `Primary`.
    - **`the_tint_granularity_caption_names_body_and_wheels`** — a source-string
      assertion over the caption's format string, in the shape `blur.rs` uses, on
      `NEVERAGAIN`'s rule that *"if the text being edited is a claim rather than
      code, the check that finds out is a reader, not a tool"* — except here the
      reader is a capture and the tool is a grep for the sentence the pane prints.

12. **`doc/ui/DEMO_APPLICATION.md` gains three dated, attributed notes, and none
    of them closes a row or renumbers one:**
    - § *The car-status pane* — a note recording that the pane is built at
      `PANE_WIDTH` in three states, that **the floor is drawn in 2-D and the
      mirrored image is not drawn**, with `L12` named, and that **the contact
      shadow is the rest pose's**;
    - § *Could not verify*, the **Track Mode's car-body recolouring rules** row —
      a note recording that the demo implements **the mechanism and the
      granularity limit and no colour rule**, on that row's own *"not a spec the
      demo should copy until verified"*;
    - § *Composite widgets* **row 4** — a note naming this as the first
      `ModeScope` consumer of Track Mode and stating that **the recolouring it
      performs is body-against-wheels because one material is all the geometry
      has**, which is that row's own demand being half-met and named.

13. **`doc/ui/IMPLEMENTATION_STATE.md` gains the record**: a task-table row for
    **the demo task 03**, naming the file, its review count and its
    waivers-or-none; a task section carrying **the pane's seven rects and every
    number's derivation**, **the three-state table**, **the `L12` decision and
    the culling reason in one paragraph**, **the Track Mode `[B]` decision and the
    granularity limit**, **the test count before and after**, and **the frame rate
    for all seven pages**. **`IMPLEMENTATION_STATE.md` is not a source of evidence**
    (`task-sequence.md` § *State*); it points at the code.

14. **The suite, the seven pages and the frame rate are all produced.** From
    `ui/`: `cargo fmt --check`; `cargo build --all-targets --all-features`;
    `cargo clippy --all-targets --all-features -- -D warnings`;
    `cargo test --all-features` with **the three per-binary counts pasted and
    requirement 11's eleven tests present by name**, against a baseline of
    **1894 (1450 + 224 + 220)**, **with no test deleted, renamed away or
    weakened**; `cargo doc --no-deps` clean; and `cargo audit` **recorded as not
    installed on this host rather than passed**. Then the seven-page capture and
    the frame rate on all seven pages, per § *Acceptance Criteria*.

## Acceptance Criteria

- [ ] **The pane is on screen at 40 % of the window, in three states, and the
      states differ.** `--tab=<the seventh page's name>` shows a pane
      `WINDOW.width × 0.40` wide from `CONTENT_TOP` to `FPS_READOUT_ORIGIN.1`,
      containing the indicator column's empty box, the car area with the car
      standing on a floor and a contact shadow, the card row's filled surface and
      the dots band's filled surface. `Z`, `X` and `V` move it between parked,
      driving and charging, and **each of the three captures differs from the
      other two by more than the frame-rate readout's band** — the mechanism is
      `pane_rects`'s seven-rect table and `STATE_READOUTS`, and the readout rows
      differ per state is asserted, not photographed.

- [ ] **Every one of the pane's rects comes from `pane_rects(width)` and nothing
      else.** `rg -n 'pane_rects' ui/src/ui_demo/src/main.rs` shows the definition,
      its one caller in the paint path and its tests, and **`rg -n 'PANE_WIDTH|PANE_TOP|CARD_ROW_HEIGHT|INDICATOR_WIDTH'`
      shows no arithmetic outside `pane_rects` that builds a pane rect**.
      `pane_rects_agrees_with_every_constant_it_is_built_from` asserts all seven
      against their constants, and `pane_rects_moves_every_rect_when_its_width_moves`
      asserts the moving and the fixed halves **as a pair** — at `PANE_WIDTH`,
      `WINDOW.width` and `0.0`, with a zero-width pane accepted rather than
      clamped.

- [ ] **The pane touches neither the tab bar nor the readout band.**
      `pane_reaches_neither_the_tab_bar_nor_the_readout_band` asserts every one of
      the seven rects is inside `(0, CONTENT_TOP, WINDOW.width,
      FPS_READOUT_ORIGIN.1 - CONTENT_TOP)`, and the capture shows the frame-rate
      readout **uncovered** on the seventh page. **The mirror of
      `nothing_the_demo_places_reaches_into_the_strip`.**

- [ ] **Three states, one write path, and the gates move.** `Z`/`X`/`V` each go
      through `Demo::set_car_state`, which is the only writer of
      `Demo::car_state`; `set_car_state` returns `false` and writes nothing when
      asked for the state already showing; and
      `switching_state_moves_the_gates_and_nothing_else` asserts after each switch
      that the readouts' visibility matches `STATE_READOUTS`, that `on_show` is
      false for a readout the state does not show, **and that `Demo::car_rect()`
      is unchanged across all three** — the one-car argument made checkable.
      `every_state_shows_its_own_readouts_and_shares_only_the_range` asserts the
      **complement** (no index is live that no row names) as well as the rows.

- [ ] **The car is five mesh commands, recorded before the chrome over it.**
      `the_pane_records_five_mesh_commands_and_no_more` counts **exactly five**
      `DrawCommand::Mesh` on the car node and reads the five `SubMeshRange`s in
      `Model::ranges`' file order; `the_car_is_recorded_before_the_chrome_that
      _sits_over_it` asserts the first mesh command precedes the caption's text and
      the card row's surface. **Mutation evidence in the handoff:** drop one
      `wheel-*` from the loop and watch the count assertion fail; move the mesh
      block after the caption and watch the ordering assertion fail. **A test that
      has never failed is a hypothesis** (`developer.md` § Phase 3).

- [ ] **The floor is two convex rects, the shadow is one, and `L12` is recorded
      with its evidence.** `the_floor_is_two_convex_rects_and_the_shadow_is_one`
      asserts the two floor commands' corners pass `paint::polygon_is_convex` and
      that exactly one `DrawCommand::Shadow` follows them, **and that no mesh
      command on the car node carries a mirrored matrix.** `DEMO_APPLICATION.md` §
      *Gaps this layout exposes in `ui_core`* carries **row `L12`** with the
      `GL_CCW`-is-window-space reason and evidence by symbol, **appended after
      `L11` with no renumber and no row closed**, and § *The car-status pane*
      carries a dated note saying the mirrored image is not drawn and why.

- [ ] **Track Mode's limit is on the screen and in the tint.**
      `track_mode_tints_the_body_and_the_wheels_differently` reads the `tint` off
      the `body` command and off a `wheel-*` command in both modes: equal in
      `Standard`, different in `Track`, body = `Warning`, wheel = `Primary`.
      `the_tint_granularity_caption_names_body_and_wheels` pins the caption's
      sentence. **`DEMO_APPLICATION.md` § *Could not verify*'s Track Mode row
      carries a dated note that the demo implements the mechanism and the
      granularity and no colour rule**, on that row's own *"not a spec the demo
      should copy until verified"*, and **composite row 4 says which half of its
      demand is met.**

- [ ] **Nothing from another task leaked in, and the dependency rule holds.**
      `git diff --stat` shows **no change** to any file under `ui/src/ui_core/` —
      this task is `ui_demo` and `doc/ui` only, and **`L12` is a row, not a
      pipeline change**. `ui/Cargo.toml` and `ui/Cargo.lock` are unchanged: the
      approved direct dependencies remain `sdl3 0.20`, `glow 0.18` and
      `freetype-rs 0.38`, and a pane is layout and paint arithmetic. **No `unsafe`
      is added, no `unwrap`, no `expect`, no `panic!`, no `unimplemented!`, no
      `todo!`** outside the existing `demo_on` fixture's `.unwrap()`, which
      predates this task and is not extended. **Edition 2021 and
      `rust-version = "1.85"` are respected** — nothing newer than the floor is
      used. **`Page::ALL` is `[Page; 7]` and `Page::DEFAULT` is still
      `Page::Pads`, both exactly as `TASK_UI_DEMO_01` left them**, and **no
      `--tab=` name is added by this task.**

- [ ] **`cargo test --all-features` is green with every named test present**, and
      the handoff **lists each by name**: `pane_rects_agrees_with_every_constant
      _it_is_built_from`, `pane_rects_moves_every_rect_when_its_width_moves`,
      `pane_reaches_neither_the_tab_bar_nor_the_readout_band`,
      `the_car_rect_matches_task_forty_aspect_within_one_percent`,
      `every_state_shows_its_own_readouts_and_shares_only_the_range`,
      `switching_state_moves_the_gates_and_nothing_else`,
      `the_pane_records_five_mesh_commands_and_no_more`,
      `the_car_is_recorded_before_the_chrome_that_sits_over_it`,
      `the_floor_is_two_convex_rects_and_the_shadow_is_one`,
      `track_mode_tints_the_body_and_the_wheels_differently`,
      `the_tint_granularity_caption_names_body_and_wheels` — **eleven**, against a
      measured baseline of **1894 (1450 `ui_core` + 224 `ui_demo` + 220
      doctests)**, with the three counts pasted, **each higher than the baseline by
      the number of tests added in it** and **no test deleted, renamed away or
      weakened.** `cargo fmt --check`, `cargo build --all-targets --all-features`,
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo doc --no-deps` clean. `cargo audit` is **recorded as not installed on
      this host, not passed.**

- [ ] **The six gallery pages are pixel-identical, and the mechanism is the
      criterion rather than the result.** `Page::ALL`'s six original names, release
      build, captured **before and after** with the commands of
      `IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the capture
      method* verbatim: window id **re-read at the time of each capture** with
      `xwininfo -root -tree` (a root capture, and `ffmpeg x11grab`, return black
      for a GL window), `pgrep -a -x ui_demo` in the same call as each `magick
      import -window <id>`, then `magick compare -metric AE before.png after.png
      null:` per page.

      - **The criterion on all six is AE 0 outside `y ≥ 680`**, every differing
        pixel inside the frame-rate readout's band — the criterion tasks 34 to 39
        inherited.
      - **The mechanism is four facts, and saying so is the criterion:** **no
        node on those six pages is a pane node**, because every new handle carries
        `Some(Page::Demo)` and the paint gate empties the rest;
        **`Demo::frame`'s existing per-widget ticks are unchanged**, so their
        clock costs are what they were; **`the sixth page's rects are the six
        pages' rects and this task writes none of them**, the only change to
        `placed_handles` being six appended rows naming pane handles; and **the
        one ambient term is still one**, because the rotator is shared.
        **A change that cannot move a pixel is demonstrated not to, not asserted
        not to.**
      - **The rect-level half keeps its name and every one of its assertions**:
        `every_page_places_every_rect_where_the_gallery_placed_it`,
        `no_two_placed_rects_overlap`, `placed_handles`,
        `expected_placed_rect_names` and `assert_placed_handles_is_complete`. **No
        row of any of them is loosened** — six names are **added**, which
        `assert_placed_handles_is_complete` is the instrument for.

- [ ] **The frame rate is measured on all seven pages and the script's own line
      is pasted.** `.ai/tools/fps-check.sh 10 55` on the default page — **the only
      thing the script can do**, since it takes `seconds` then `floor` and runs
      `./target/release/ui_demo` with no arguments and no page, and forwarding
      `"$@"` is an `.ai/` change a task may not make — and then
      `ROADOS_RUN_SECONDS=10 ./target/release/ui_demo --tab=demo` for each of
      the seven, with the `roados-fps` line parsed by hand. **Every page above the
      floor of 55**, **the six unchanged pages inside the recorded release band**,
      and **the seventh page's number reported as its own figure with its own
      arithmetic** — five more `DrawCommand::Mesh` per frame plus the floor, the
      shadow and the caption is a per-frame cost the other six do not pay, and
      **`developer.md` § Phase 3's reason is that a still of a 4 fps application is
      pixel-identical to a still of a 60 fps one**, which is how a four-fps
      regression survived three reviews in this repository. **A drop under the
      band is reported as a drop, with the number, not rounded into it.**

- [ ] **What the handoff does not claim, in those words.** It states that **the
      pane's centre is not a map**: § *What this means for the demo's shape*
      item 1 makes gap `#1` a `TASK_UI_DEMO_n` item and **no map exists in this
      repository**, so the road visualisation, the other cars, the speed-limit
      sign and the power meter § *Screen states* names under driving are **not on
      screen**, and the driving state's difference is its three readouts and not a
      scene. It states that **the floor is drawn in 2-D and the mirrored image is
      not drawn**, with `L12` named. It states that **Track Mode's tint is
      body-against-wheels because one material is the finest the geometry gives,
      and that no colour rule was copied** because § *Could not verify* says not
      to. It states that **the seventh page's `Page` variant name and its `--tab=`
      spelling belong to `TASK_UI_DEMO_01` and this task never wrote either.** And
      it states that `Demo::car_rect()` on the `data` page is **unchanged**, so
      the `data` page shows the same car at the same angle as before.

## Out of Scope

- **No map, and nothing that claims to be one.** Gap **#1** stays a
  `TASK_UI_DEMO_n` item per § *What this means for the demo's shape* item 1. The
  pane's centre is a `ThemeToken::Background` rect and a hole the next task paints
  over. **The road visualisation, the detected other cars (composite row 7), the
  power meter (composite row 6), the lane markers (composite row 8) and the
  speed-limit sign are not built**, and § *Could not verify*'s lane-guidance row
  records that feature as *"Excluded from the demo's scope"* in any case. **The
  driving state therefore differs from parked by its readouts, and the handoff
  says so.**
- **No indicator lights, no glyphs and no column contents.** The **box** is
  `TASK_UI_DEMO_04`'s to fill: the ~20 conditions, the five colours, the three
  timing rules, the latch and the power-up self-test. This task draws the column
  as a surface so the hole is visible and stops there.
- **No card, no pager dot and no carousel.** The card row is a filled surface and
  the dots band is a filled surface, both `TASK_UI_DEMO_05`'s. **`N`/`U`/`R`/`E`/`M`
  are not bound here and no key owns a card.**
- **No callout, no leader line, no lock glyph and no charge-port glyph.** All four
  are composite row 3 and `TASK_UI_DEMO_05`'s. **The charge-port *lamp* this task
  draws is not the charge-port *hotspot*** — the lamp is a `Circle` at
  `PaneRects::charge_lamp` in the charging state, the hotspot is 05's icon above
  it, and the two are stated not to claim the same pixels because two features at
  one point is the one overlap a reviewer cannot see.
- **No reshape, no tier and no zoom.** `pane_rects` takes a width and this task
  passes one constant to it. `TASK_UI_DEMO_05` § *The two-axis reshape* owns the
  tier, the snap and the pinch, and the `[C]` tag that governs all three.
- **No mirrored geometry, no foreshortened shadow, and no pipeline change to make
  either possible.** § *The floor* has the reason in one line and requirement 10
  has the reversal specified to the point of being built. **No `front_face` call,
  no winding flag and no cull toggle is added to `ui_core`**, because a demo task
  that changes the renderer is a different task (`developer.md` § *Stop
  conditions*).
- **No Track Mode colour rule, no per-component temperature and no per-tyre grip
  tint.** § *Could not verify* binds this task, and one material is the geometry's
  own limit.
- **No translucent chrome and no backdrop.** Composite row 5 is `L1`'s, and
  `TASK_UI_PRIM_41`'s `Painter::backdrop` is **not** used. The pane's surface is an
  opaque `rounded_rect` in the theme's `Surface`.
- **No `Margin`, no `shrink`, no `LayoutMode::Grid`, no `ThemeScope`, no
  `Snapshot`.** `TASK_UI_PRIM_48`, `52`, `50` and `47`'s `Snapshot` are all
  available and **none is used here**, and the reason for each is its own row in
  the dependency table. `ModeScope` **is** used, for the one thing `L6b`(b) says
  is genuinely absent.
- **No `Page` change, no `--tab=` name and no eighth gate.** `Page::ALL` is seven
  and `Page::DEFAULT` is `pads`, exactly as `TASK_UI_DEMO_01` left them.
- **No new dependency and no `unsafe`.** `ui/Cargo.toml` and `ui/Cargo.lock` are
  untouched; the approved direct dependencies remain `sdl3 0.20`, `glow 0.18` and
  `freetype-rs 0.38`, and a pane is layout arithmetic over `f32` plus five draw
  commands. **Zero new `unsafe` blocks**, because there is no GL, no FFI and no
  pointer in this change.
- **Found in the tree and deliberately not fixed.** `TASK_UI_DEMO_01` and
  `TASK_UI_DEMO_02` are absent from the repository, so this file's two rows for
  them state structural facts and an open question rather than citations. **That
  is recorded here rather than resolved**, because inventing what they deliver
  would be the one error in this file that no test could catch.
