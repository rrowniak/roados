# TASK_UI_PRIM_45: Gap #5 — Per-Node Clipping Belongs to the Widget System, Not to the Demo's Frame Loop

> **2026-10-10 — this file's state-file requirements are superseded.**
> `doc/ui/IMPLEMENTATION_STATE.md` is a status board of 3 KB or less
> (`.ai/workflows/task-sequence.md` § *State*): *Current position* and *Left over*,
> no per-task record section, no task-table row, no deviations list, no history.
> Where this file asks for one, put the durable fact in the code's doc, in this
> file, or on *Left over* — and move the file to `doc/ui/done/` when it is done.

## Goal

Make `Renderer` read each node's own `LayoutState::clip`, so per-node clipping is
supplied by the widget system rather than the demo's frame loop. Delete
`Demo::frame_clips` and the eleven stale claims (nine doc comments, one code comment,
one document bullet) that assert per-node clipping does not exist. The mechanism
(`Batch::clip`, `Renderer::apply_clip`, `LayoutState::clip`) is built and tested; what
is missing is the one line that joins them.

## Context

**Decision: the renderer derives the clip from the node's own `LayoutState::clip`, and
the caller's `clip` parameter may only narrow it; `Demo::frame_clips` is deleted.**
`Renderer::draw_node_clipped` composes the caller's constraint with the node's clip
through `layout::intersect` — the pass's arithmetic, not a second copy — so a caller can
narrow but never widen past an ancestor's box. This closes row **`#5`** of
`doc/ui/DEMO_APPLICATION.md` § *Library gaps*: `Batch::clip` already rides outside
`BatchKey`, `Renderer::apply_clip` already sets the scissor at submission, and
`Layout::visit` already stores the ancestors' intersection on `LayoutState::clip`. Facts
the source cannot supply:

- The row's count is wrong: **nine doc comments in `ui/src`, one code comment on
  `TEXT_PANEL`, one document bullet**.
- Test baseline **1894** (`ui_core` 1450, `ui_demo` 224, doctests 220); this file
  projects **1903**.
- No shadow in `main.rs`; the demo's `Dialog` and `Toasts` are roots under
  `Constraints::tight(size)`, so their clips are the window.
- The demo receives no pointer event on this host (XTEST never delivers one).
- `LayoutState::hits` is task 42's; MSAA 4× (`MULTISAMPLE_SAMPLES = 4`,
  `MULTISAMPLE_BUFFERS = 1`) is default-framebuffer only.

## Requirements

Requirements 1–12 keep their source numbers and order. Requirement 13 (the per-task
record) is dropped, superseded by `.ai/workflows/task-sequence.md` § *State*.

### Sub-task 45.1 — the mechanism

1. `layout::intersect` (`ui/src/ui_core/src/layout.rs`) → `#[must_use] pub fn
   intersect(a: Option<Rect>, b: Option<Rect>) -> Option<Rect>`; doc: public as the
   arithmetic `Renderer::draw_node_clipped` composes a caller's constraint with the
   node's clip with. **Body and tests unchanged.**

2. `impl From<crate::paint::Rect> for crate::layout::Rect` (`layout.rs`, beside
   `impl From<Rect> for paint::Rect`): identity on the four coordinates, reverse,
   one-sentence doc, doctest on all four fields.

3. `Renderer::draw_node_clipped` (`render.rs`) resolves the clip; signature unchanged,
   the `is_dirty()` check first. Then `let extra = clip.map(layout::Rect::from); let own
   = node.layout().clip(); let resolved = layout::intersect(extra, own).map(
   paint::Rect::from);` and `self.batcher.add_clipped(command, resolved)` per
   `take_commands()`. **Doc:** `None` means *"no additional constraint"*, not *"no
   clip"*; the clip is `crate::layout::LayoutState::clip`; window-space (`Layout::layout`
   starts at `Offset::ZERO`).

4. `Renderer::draw_node`'s doc: `None` now means the node's ancestors' boxes. One
   sentence naming `Renderer::draw_node_clipped` and `crate::layout::LayoutState::clip`.
   **No code change.**

5. `applied_clip` becomes a private `ClipCache` — `#[derive(Clone, Copy, Debug,
   Default)] struct ClipCache { applied: Option<Rect> }` with `#[must_use] fn
   wants(&self, clip: Option<Rect>) -> bool` (`clip != self.applied`), `fn
   record(&mut self, clip: Option<Rect>)`, `fn clear(&mut self)` (`applied = None`).
   Field → `clip_cache: ClipCache`; `Renderer::new` uses `ClipCache::default()`; call
   sites `Renderer::begin_frame`, `Renderer::apply_clip`,
   `Renderer::bind_default_target`. **Extraction, not change**; its doc records what the
   requirement-12 tests prove and cannot (a GL context is needed).

6. Pre-flight gate: `tests::no_recorded_command_is_cut_by_its_own_nodes_clip`
   (requirement 12) exists and is pasted **before requirement 3 is edited**, against the
   unmodified renderer. **A cut at a container's own edge is accepted; elsewhere a stop
   condition.**

### Sub-task 45.2 — the demo stops owning the clip

7. `Demo::draw` (`main.rs`) becomes `fn draw(&mut self, renderer: &mut Renderer)`
   looping `for handle in self.order.iter().copied() { renderer.draw_node(handle, &mut
   nodes); }` over `self.nodes.borrow_mut()`. `Demo::frame_clips` is deleted with its doc
   (the `clip_for` paragraph included); `Demo::draw`'s doc gains one sentence naming each
   node's `LayoutState::clip`. **No other demo function changes.**

8. Rewrite in place the ten stale claims at: `render.rs` `Renderer::set_scissor`;
   `list.rs` module § *Clipping* and `List::clip_rect`; `scroll.rs` module docs,
   `Scroll::clip_rect`, `clip_commands`; `image.rs` module docs and `ImageFit::Cover`;
   `main.rs` `TEXT_PANEL`. Each now says the clip is the node's `LayoutState::clip`,
   applied by `Renderer::apply_clip` at submission; still-true text and doctests stay;
   `scroll.rs` drops the *"cannot clip at the GL level"* inference; `Cover` states the
   geometry argument (an overflowing quad is right only when something clips);
   `TEXT_PANEL`'s column is clipped to the panel.

9. `doc/ui/DEMO_APPLICATION.md` amended, dated, attributed. Row **`#5`** gains a dated
   note naming `TASK_UI_PRIM_45`: the clip is `LayoutState::clip` inside
   `Renderer::draw_node_clipped` and nowhere else; count corrected two→nine plus the
   `TEXT_PANEL` comment; `Blocks` names three products none delivered (map viewport =
   gap `#1`; scroll view clipping needs a scrolling widget the demo lacks; card page
   edges need card and pager). **Row not deleted.** The § *Corrections to the second gap
   table* bullet gains a dated clause: nine not two, by symbol, line numbers withdrawn.

10. Seed and number. A temporary **`CLIP_PROBE`** block in `Demo::new` (task 14's
    `LIST_OFFSET` technique) builds one `Container` (`LayoutMode::Absolute`, flat opaque
    `background`, `border_radius` zero) under a smaller `LayoutMode::Absolute` container
    (`text_panel` or the controls layer) so a known pixel band spills; tests assert the
    probe's box and spill band overlap nothing `Demo::placed_rects()` reports and the
    band is at `y < 680`. Expected **`AE = SPILL_WIDTH * SPILL_HEIGHT` exactly**
    (`MULTISAMPLE_SAMPLES`'s full-coverage statement, `border_radius` zero). Both sides
    seeded:

    ```sh
    DISPLAY=:0 xwininfo -root -tree | rg '"roados ui_demo"' | rg -o '0x[0-9a-f]+' | head -1
    DISPLAY=:0 magick import -window <id> /tmp/clip-before.png
    DISPLAY=:0 pgrep -a -x ui_demo
    DISPLAY=:0 magick import -window <id> /tmp/clip-after.png
    magick compare -metric AE -crop 1280x680+0+0 +repage /tmp/clip-before.png /tmp/clip-after.png null:
    ```

    Revert proved by `rg -c "CLIP_PROBE"` = **0**; the second edit, one row in
    `tests::undrawn_leaf_exemptions`, reverts too. Cannot show a scissored shadow or a
    `Page::Overlays` dialog/toast under a container box (both roots); Overlays is
    **AE 0**.

11. `clip_commands` and `List::paint` keep their behaviour. **No arm changes; no
    existing test renamed, weakened or deleted.**

### The tests

12. No display, network, filesystem or wall clock; each names the mutation it kills.

    - `layout.rs`: `a_node_is_not_clipped_to_its_own_box` (mutation
      `intersect(own_clip, Some(node_rect))`);
      `a_node_is_clipped_to_the_intersection_of_its_ancestors_and_not_its_parents_own_box`.
    - `batch.rs`: `a_batch_under_a_different_clip_does_not_merge_even_with_the_same_key`
      (mutation: delete `&& batch.clip == clip`;
      `commands_under_different_clips_do_not_share_a_batch` unchanged);
      `the_same_clip_after_a_different_one_is_a_third_batch`.
    - `render.rs`: `the_clip_cache_asks_for_a_write_only_when_the_clip_changed`;
      `a_cleared_clip_cache_reapplies_a_clip_it_had_already_applied`.
    - `main.rs`: `no_recorded_command_is_cut_by_its_own_nodes_clip` (all six `Page::ALL`
      pages plus `Page::Overlays` with dialog and toast; every
      `scroll::command_bounds`-boundable command inside its `LayoutState::clip`);
      `the_frame_uses_a_bounded_number_of_distinct_clips`;
      `the_clip_a_child_will_be_scissored_to_is_its_parents_box`;
      `the_chart_keeps_its_geometry_inside_its_own_rect` (renamed from
      `no_node_is_clipped_and_the_chart_keeps_its_geometry_inside_its_own_rect`, its
      `frame_clips()` half deleted); `the_probe_spill_band_touches_nothing_the_demo_places`
      (seed only).

## Acceptance Criteria

- [ ] **The clip comes from the node; the caller narrows it.** `git diff --stat`:
      `render.rs`, `layout.rs`, `batch.rs` changed, `render/target.rs`,
      `render/context.rs`, `paint.rs`, `input.rs`, `node.rs` and other `widgets/*.rs`
      unchanged, `batch.rs` test-only; `draw_node_clipped` has one
      `layout::intersect(extra, own)`, signature unchanged.
- [ ] **The eleven stale claims are gone** — this returns no output:
      `grep -rn -e 'no scissor state' -e 'no scissor is set' -e 'Real clipping needs the scissor' -e 'Real per-node clipping is not here' -e 'computed, not applied' -e 'there is no per-node clip' -e 'applies to the whole frame' -e 'per-node clipping as a deferral' -e 'until then the viewport' -e 'until that scissor is applied' -e 'not\*\* applied' ui/src/ doc/ui/IMPLEMENTATION_STATE.md`;
      at `75a896c` plus the uncommitted diff it returns sixteen lines;
      `Renderer::apply_clip`'s *"This is what makes a per-node clip possible at all"*
      still shows.
- [ ] **`add_clipped`'s merge predicate is pinned.** Delete `&& batch.clip == clip`;
      `cargo test --all-features -p ui_core batch::` from `ui/` shows the two-clip case
      go **3 batches to 2** and fail.
- [ ] **The pre-flight ran first and is pasted.** `cargo test --all-features -p ui_demo
      no_recorded_command_is_cut_by_its_own_nodes_clip` against the unmodified renderer,
      nodes cut per page; any cut not at a container's own edge is a stop condition;
      green after requirement 3.
- [ ] **Capture.** `Page::DEFAULT`, release, seed both sides; `magick compare -metric AE
      -crop 1280x680+0+0 +repage` reports exactly `SPILL_WIDTH * SPILL_HEIGHT`, every
      differing pixel in the spill band; window id re-read per capture; `rg -c
      "CLIP_PROBE"` and `rg -c "probe"` both 0. `Page::Overlays` with dialog and toast
      and `Page::pads`/`text`/`input`/`controls`/`data`: AE 0, not a clipped-shadow demo.
- [ ] **The demo's clip table is gone.** `grep -n 'frame_clips'` and `grep -n 'fn
      clip_for'` empty; `Demo::draw` the six-line loop; the chart test renamed as in
      requirement 12; `every_page_places_every_rect_where_the_gallery_placed_it`,
      `no_two_placed_rects_overlap`, `assert_placed_handles_is_complete` keep names and
      assertions.
- [ ] **The suite is green with every named test present**, listed by name in the
      handoff; **baseline 1894, project 1903** (eight new tests, one doctest, one rename,
      one in-test deletion). Verification is `.ai/agents/developer.md` § *Phase 3*;
      fmt/build/clippy/doc clean; **`cargo audit` not installed, recorded not passed**.
      `.ai/tools/fps-check.sh 10 55` and `ROADOS_RUN_SECONDS=10
      ./target/release/ui_demo --tab=<page>` per page; all above **55**, each against its
      own pre-change number.
- [ ] **Row `#5` is amended, dated and open, `Blocks` intact**, the § *Corrections to
      the second gap table* bullet dated with line numbers withdrawn;
      **`LayoutState::hits` is task 42's and untouched** — `git diff` over `layout.rs`:
      no `hits`, no `set_hits`, no change to `LayoutState::visible`/`set_visible`.
- [ ] **No new dependency, no new `unsafe`, no new page.** `ui/Cargo.toml` and
      `ui/Cargo.lock` unchanged; `Page::ALL` six names, `Page::DEFAULT` `Pads`;
      `DEPTH_BITS`, `MULTISAMPLE_SAMPLES`, `MULTISAMPLE_BUFFERS`, `SOLID_BLUR`,
      `DrawCommand`'s variants and `BatchKey::is_singleton` untouched.

## Out of Scope

- No shadow-map depth; `ShadowTarget` gains no depth attachment, `SOLID_BLUR`'s
  threshold does not move.
- No rounded or soft clip edges; `DrawCommand::Image`'s `radius` stays a hard `discard`.
- No clip animation or interpolation — gap `#8`'s row; `Transform` not read at draw time.
- No nested-clip intersection beyond requirement 3's one-line composition.
- No per-pixel scissor, no discard-based clipping, no stencil pass.
- No change to what `List::paint` and `clip_commands` do (requirement 11).
- No new widget, page or UI; the `CLIP_PROBE` seed is temporary and reverted.
- No change to the shadow FBO, `bind_for_write`, or `bind_default_target`'s behaviour;
  no on-screen proof that a scissored shadow composites inside its scissor.
- No change to `LayoutState::visible`, `LayoutState::hits`, `hit_test_from` or `Focus`.
- No change to the three layout passes in `Demo::frame`.
- No `ImageFit` change and no new fit.
