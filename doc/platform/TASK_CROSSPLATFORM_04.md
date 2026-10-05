# TASK_CROSSPLATFORM_04: Head-Unit Smoke Test

## Goal

Run the demo on the actual head unit and close the acceptance criteria that this
machine cannot.

## Context

Task 24's acceptance criteria include claims no build machine can settle: "60 FPS
is maintained", "no visual glitches", "All interactions work (click, drag,
scroll, type)". Every one of them has been carried as a waiver since task 01,
because this host is x86_64 with Mesa and no panel. They should not stay waived
forever — a waiver that outlives its reason becomes a fiction.

This is the last task in the sequence. Its whole purpose is to be the place where
the deferred work is finally verified on real hardware, so it is deliberately a
smoke test rather than a development task: nothing new is built here.

## Requirements

1. **Deploy the demo to the head unit** and run it. Capture the output of
   `file target/aarch64-unknown-linux-gnu/<demo>`, which **must** report
   `AArch64`. This is the check that catches a "cross" build that quietly
   produced host objects, and nothing else in the pipeline catches it.

2. **Confirm the context is created at runtime**, not merely compiled:
   - `SDL_Init` succeeds and reports a video driver — and it should be `kmsdrm`,
     not `x11` or `dummy`.
   - The GLES context reports **3.1** or better. The host reports 3.2, which says
     nothing about the head unit's GPU. This is the one number the whole
     `PRIMITIVES_ARCHITECTURE.md` rests on.
   - No `dlopen` failure at startup — the task 03 audit should have predicted
     none, and this is where that prediction is checked.

3. **Measure frame rate** and report it honestly, including what was measured
   over what interval, at what resolution, and with which scenes. Task 24's "60
   FPS" has never been measured anywhere; if it is not met, say so and say by
   how much. A waived 60 FPS is acceptable; a claimed one that was never measured
   is not.

4. **Exercise every interaction** task 24 lists: click, drag, scroll, type, and
   the dialog and toast paths. Touch first — this is a head unit, and the input
   system was written against mouse events as well as touch.

5. **Close the waivers honestly.** For each of task 24's carried waivers, report
   it as closed with evidence, or as still open with the reason. Nothing may be
   reported closed on the strength of a build that succeeded.

6. **Feed back.** Anything an agent got wrong during this sequence belongs in
   `.ai/NEVERAGAIN.md`, and any factual question the work settled belongs in
   `doc/findings/`. This is stage 6 of `.ai/workflows/idea-to-code.md` and it is
   the step most often skipped.

## Acceptance Criteria

- [ ] `file` reports `AArch64` for the deployed binary.
- [ ] The runtime video driver is `kmsdrm`.
- [ ] The GLES context version is ≥ 3.1, quoted from the actual context.
- [ ] Frame rate is measured and reported, with method and scene.
- [ ] Every interaction in task 24 was exercised on the device.
- [ ] Each carried waiver is closed with evidence or still open with a reason.
- [ ] `NEVERAGAIN.md` and `doc/findings/` are updated, where warranted.

## Out of Scope

- Fixing what the smoke test finds. Record the defect and open a task; do not
  start rebuilding the library on the device.
- Performance work beyond reporting the measurement.
- Production styling or vehicle data — both belong to `roados_ui`, per
  `PRIMITIVES_ARCHITECTURE.md`.
