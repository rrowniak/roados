# TASK_UI_PRIM_51: Gap `L10` — the `Polygon` Precondition, Made Checkable, and the Reversal Asked For

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Take row **`L10`** of `doc/ui/DEMO_APPLICATION.md`
§ *Gaps this layout exposes in `ui_core`* and do the only two things that are
honest about it: **put the renderer's convexity precondition where a caller can
ask whether it meets it**, and **put the dated decision that keeps the capability
out of the renderer to the operator in writing.**

**This task does not teach the renderer to fill a concave polygon.** It does not
reverse the operator's decision of **2026-10-02**.

## Context

**Decision: (D) — keep the row open, escalate in writing, and ship the enforcement
instrument.** The capability stays out of the renderer; the caller-side contract
becomes something a caller can check. After this task the sentence *"`Polygon` is
convex-only"* is still true — what changes is that a caller can find out whether
it is about to be wrong. **The row cannot be closed by argument, only amended**,
the instrument task 52 used on row `L3`.

Row `L10` is one title over four clauses and only the first is a convexity
question: the triangle fan is exact for a convex polygon and nothing else; there
is no stencil pass; no bezier and no fill rules is a missing curve primitive and a
missing winding rule; `DrawCommand::Path` is stroked, which answers gap `#4`.
Severity **Medium**, § *Task structure*'s **"Low / Medium"** bucket.

**Three dates, two of them about ear clipping. 2026-10-01** the operator *added*
`DrawCommand::Polygon` — not a declination, never to be cited as one. **2026-10-02**
the operator *declined* both renderer-side answers, recorded in
`widgets/chart.rs`'s module doc, `render/context.rs` beside `MULTISAMPLE_SAMPLES`
and `render.rs`'s `polygon_quad` doc. **2026-10-05** *"every library gap must be
closed"* **withdraws the 2026-10-03 deferral of gaps `#3` and `#7` by name** and
says nothing about any technical declination — so the two reconcile in exactly one
way: **`L10` must be closed by some means, and 2026-10-02 says which means are
excluded.** That reading is an interpretation and is labelled as one.

**No caller is blocked:** *"instrument arcs"* is `gauge.rs`'s shipped per-segment
band; *"any concave silhouette"* is answered offline by task 39's rasteriser plus
task 44's `Icon`; *"the proximity ramp"* is coloured radiating lines, which are
`Line` and `Path`. Tasks 37–39 mean a 3-D model needs no draw-time triangulation,
but all three Blocks entries are 2-D, so *"the mesh path makes the renderer-side
solution unnecessary"* is false here and no doc comment in this task may repeat it.

**The escalation is asked in the handoff, not built here:** one dated question —
*reverse 2026-10-02, or keep it?* — carrying the four answers that make "yes" an
instruction rather than another round: **failure mode** (O(n²); a zero-area ear is
rejected and a polygon with no ear expands to `Vec::new()`, with an O(n) convexity
pre-test running first so every existing capture stays valid by construction);
**self-intersection** (detection required, O(n²) enough for 2-D UI point counts,
Bentley–Ottmann declined); **winding** (for a *simple* polygon the clipping *is*
the nonzero rule, so no `fill_rule` field, but even-odd needs explicit hole
handling, which is clause 3); **cost** (~40 000 containment tests for a 200-point
outline, affordable on aarch64 and x86_64 alike — and the risk, not the arithmetic,
is the operator's to take, because a bug in that arm is a wrong picture on five of
six gallery pages).

Verification is `.ai/agents/developer.md` § *Phase 3* and is not restated here;
what is specific to this task is that **no page is expected to move**.

## Requirements

1. **A new public predicate in `ui/src/ui_core/src/paint.rs`, beside the doc that
   states the obligation.** `#[must_use] pub fn polygon_is_convex(points:
   &[(f32, f32)]) -> bool` — `pub` and module-level, not a method on `Painter`. No
   new type, no `Result`, no allocation. The algorithm is the one already in the
   tree twice: per consecutive triple `(a, b, c)`, the last wrapping to the first,
   form `(b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)`, and the polygon is
   convex when the positive and negative counts are not both present.

   Its doc states, as its own clauses:

   - **A cross product of exactly zero is neither,** so a collinear triple and a
     repeated point are not concavity — and **there is deliberately no tolerance**,
     because the crate's one tolerance is `render/blur.rs`'s `EPSILON: f32 = 1e-5`, a
     kernel-weight tolerance in a module with one natural scale, and a point list
     has none.
   - `points.len() < 3` answers `true`, because `command_quads` draws nothing for
     such a list.
   - A non-finite coordinate answers `false`, because without the check a `NaN`
     cross product reads as "no turns".
   - Both windings answer `true`, because window space is y-down.
   - **Necessary and not sufficient, stated by name.** A five-pointed star drawn in
     star order turns the same way five times and self-intersects, so the predicate
     reports *turns*, not *intersections*.

2. **`Painter::polygon`'s doc and the `DrawCommand::Polygon` variant's doc each
   gain one intra-doc link naming `polygon_is_convex`.** No signature change and no
   behaviour change — `polygon` still records what it was handed, because
   *"the recorder's contract is to store what it was handed"*. What changes is that
   the caller's obligation has a name and a way to be discharged.

3. **`chart.rs`'s and `gauge.rs`'s private `assert_convex` helpers become two-line
   wrappers** over `paint::polygon_is_convex`, each keeping its failure message —
   `chart.rs`'s names the geometry, `gauge.rs`'s says what a failure would mean, and
   a search over 200 000 geometries that reports *which* one is worth having.
   Neither helper is deleted and neither test is renamed.

4. **Two new named tests, one per shipped decomposition.**
   `every_polygon_the_gauge_records_is_convex_by_the_library_rule` in `gauge.rs`
   sweeps all three `GaugeType` values and a swept value space over a wide and a
   square rect, and asserts the library predicate on every `Polygon` the gauge
   records. **The name states "the library rule" on purpose** — a test named only
   `..._is_convex` would pass unchanged against a reintroduced private copy.
   `the_predicate_rejects_the_counterexample_this_module_recorded` in `chart.rs`
   builds the non-convex segment quad out of that module's own recorded arithmetic.

5. **Eight tests in `paint.rs`,** no display, no network, no filesystem, no wall
   clock: `polygon_is_convex_accepts_a_triangle_and_a_convex_quadrilateral`;
   `polygon_is_convex_rejects_an_l_a_chevron_and_a_star_shaped_list`;
   `polygon_is_convex_calls_a_collinear_point_list_convex_because_it_encloses_no_area`
   (three collinear points, five collinear points, a rectangle with one edge split
   by a point on it, and `a, b, b, c` — all `true`);
   `polygon_is_convex_reads_both_windings_as_convex`;
   `polygon_is_convex_refuses_a_non_finite_point` (`NAN` and `INFINITY` in each of
   x and y); `polygon_is_convex_accepts_fewer_than_three_points`;
   `polygon_is_convex_reports_by_its_turns_and_does_not_test_for_intersections` (a
   star in star order answers `true`, asserted on purpose, because a limitation
   nothing asserts is one the next edit forgets); and
   `polygon_is_convex_agrees_with_a_half_plane_test_over_two_hundred_thousand_simple_polygons`
   — the only one whose construction needs stating: **the generator sorts points by
   angle about their own centroid**, which makes each polygon star-shaped about it
   and therefore simple; the oracle is an independent O(n²) half-plane test, not a
   restatement of the shipped O(n) rule; and the test asserts `checked >= 200_000`
   **and** that it saw both verdicts, so it cannot pass by everything being convex.
   The LCG seed is a named constant.

6. **`ui/src/ui_core/src/render.rs` is not edited — not even a comment.** The
   renderer gains nothing, `git diff --stat` for this task's commit shows it absent,
   and the six gallery pages are unaffected by construction rather than by luck.
   Degeneracy prose lives only in `paint.rs`'s doc.

7. **Row `L10` in `DEMO_APPLICATION.md` is amended, dated, attributed, and is not
   closed**, in four edits. **A dated note on the row naming this task** gives a
   per-clause status: clause 1 still true and still open but its *contract* closed
   (`polygon_is_convex` is public, `#[must_use]`, and both shipped decompositions
   assert through it); clause 2 still true, declined 2026-10-02, with the reason
   strengthened (`Context::new` requests no stencil buffer at all); clause 3
   untouched and named as not this row's question; clause 4 still true and answered
   offline by `#4`. **The sentence at the head of the row is kept verbatim, because
   it is still true.** Each of the three Blocks entries **gains the source that
   answers it, not loses it**; the row is not deleted, not marked closed, its
   severity stays Medium, its Blocks column keeps all three entries and § *Task
   structure*'s **"Low / Medium"** line is not edited. **The evidence column is
   rewritten from line numbers to symbols**, because a line number into `ui/src` is
   stale the moment the tree moves. **§ *Corrections to the second gap table* gains
   one dated paragraph** carrying the three dates distinguished, what the 2026-10-05
   instruction does and does not reach, that this task did not reverse 2026-10-02 and
   did not close the row, and that the escalation is written down rather than left
   in a session. **No other row changes**, and `L1` in particular is untouched — it
   is owned by `TASK_UI_PRIM_41.md`.

## Acceptance Criteria

- [ ] **`polygon_is_convex` exists, is public, is `#[must_use]`,** is one pass with
      no allocation and no `Result`, and both `Painter::polygon`'s doc and the
      `DrawCommand::Polygon` variant's doc link to it — `cargo doc --no-deps`
      resolving without a warning is what proves that. Its doc names both limits.
      The shipped cross product appears **0 times** in each of `chart.rs` and
      `gauge.rs` and `grep -c 'fn assert_convex'` returns **1 each**, so the crate
      has one convexity rule rather than two, and the two existing searches keep
      their names, seeds, 200 000 count and assertions.

- [ ] **The property test searches rather than restates.** It passes, its body
      asserts both `checked >= 200_000` and that it saw a concave list, and a
      reviewer checks the centroid-sorted generator first, because without it the
      half-plane oracle is invalid. Making the predicate answer `true` for any list
      of five or more points must fail it with the list index in the message.

- [ ] **All ten new tests are named in the handoff and pass,** the suite is green
      against the recorded baseline plus the new tests with none removed, renamed
      or weakened, and `cargo audit` is reported as not installed on this host
      rather than as passed.

- [ ] **No page moves.** The six gallery pages are pixel-identical outside the fps
      band — `AE = 0` outside `y ≥ 680` on all six, every differing pixel inside the
      readout's band — and every page's frame rate is inside the band recorded in
      `.ai/tools/README.md` § *Frame-rate baseline*. **The mechanism, so a reviewer
      can check it:** the only production item added is one `pub fn` no frame path
      calls; the two widget files change `#[cfg(test)]` code and doc comments only;
      and the commit's own `git diff --stat` shows `render.rs`, `batch.rs`,
      `font.rs`, `render/target.rs`, `render/context.rs`, `render/blur.rs` and
      `ui/src/ui_demo/src/main.rs` absent — **the commit's own diff, not a diff
      against `HEAD`, because those files are already modified in the uncommitted
      diff at `75a896c` and a `HEAD` diff would attribute another task's work to
      this one.**

- [ ] **`L10` is amended, dated, attributed and open,** with the head sentence kept
      verbatim, a per-clause status, all three Blocks annotated and not deleted,
      severity Medium, § *Task structure* unedited, zero `render.rs:`-style line
      numbers left in the evidence column, the dated paragraph in § *Corrections to
      the second gap table*, and **no other row changed**.

- [ ] **No doc comment in any changed file asserts the opposite of the code beside
      it.** Three claims are named in advance because each is a sentence this task
      could have written and did not: that the renderer fills concave polygons
      (**false** — requirement 6 keeps `render.rs` untouched); that a caller's
      decomposition obligation is **enforced** (**false** — it is *askable*, and
      `Painter::polygon` still *"neither measures the points nor rejects them"*);
      and that the mesh path removed the need for this (**false** — § *Context*).

- [ ] **No reversal is claimed anywhere, and the mechanism is grep-able.**
      `grep -rn '2026-10-02' ui/src/` returns the **three pre-existing hits and no
      new one**, so the crate's own record of the declination is unchanged and only
      `paint.rs` gains prose that *links* it. Nothing claims the operator has
      answered the escalation.

- [ ] **Nothing from another task leaked in.** Five files and no more;
      `ui/Cargo.toml` and `ui/Cargo.lock` unchanged; `grep -c unsafe` over
      `paint.rs` unchanged; `grep -rn 'unwrap()\|expect(\|panic!\|todo!\|unimplemented!'`
      over the diff empty; and `render.rs` absent from the commit.

## Out of Scope

- **No ear clipping, no concave fill in the renderer, no reversal of the operator's
  2026-10-02 decision.** `command_quads`'s `DrawCommand::Polygon` arm is not edited,
  `DrawCommand` gains no variant, and no triangulation function is written.
- **No stencil pass and no stencil buffer.** `Context::new`'s attribute list is not
  touched; `set_stencil_size` and `glClear(GL_STENCIL_BUFFER_BIT)` stay absent from
  `ui/src/`.
- **No bezier, no curve, no `Path` tessellation, no stroke-to-fill conversion, no
  outline extraction, no fill rule, no `nonzero`, no `even_odd`, no winding
  argument.** These are clauses 3 and 4 of the row, and neither is a convexity
  question; `grep -rn 'bezier\|cubic\|quadratic\|curve_to\|fill_rule' ui/src/` stays
  empty. **No multi-subpath, no holes, no `DrawCommand` change** — the nine variants
  stay nine.
- **No mesh or GLB triangulation and nothing from tasks 34–40:** no
  `render/mesh.rs`, `render/matrix.rs` or `render/meshio.rs`, no `MeshVertex`, no
  `DrawCommand::Mesh`, and no change to the depth, culling, shader, loader or
  byte-format policy. **No `Icon`, no icon asset, no SVG, and no edit to task 39's
  or task 44's files** — task 44's § *Out of Scope* already names `L10` as a
  separate task and does not amend it.
- **No change to the six pages, no new page, no `ui/src/ui_demo/src/main.rs` edit
  at all** — which is why the capture criterion above is the full one. **No new
  dependency, no `unsafe`, no `as` cast:** the approved direct dependencies are
  `sdl3 0.20`, `glow 0.18` and `freetype-rs 0.38`, and the predicate is arithmetic
  on two `f32`s. **No change to any other gap row**, and none to
  `doc/ui/PRIMITIVES_ARCHITECTURE.md`.