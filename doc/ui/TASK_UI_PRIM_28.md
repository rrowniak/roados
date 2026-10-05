# TASK_UI_PRIM_28: Reconcile the SDL Configuration

## Goal

Make `doc/ui/PRIMITIVES_ARCHITECTURE.md` and reality agree, and settle the
subsystem policy the operator deferred.

## Context

Task 01 established four places where the architecture document and SDL 3.4.16
disagree, or where the operator chose to defer. All of them are documentation
truth problems: cheap to fix, expensive to leave, because each one will be
rediscovered by a later agent as a surprise.

The operator's 2026-09-28 decision was to build SDL3 with **default subsystems,
for now**. That is a deliberate deviation, not an oversight, and the reason it
was acceptable is that the alternative was blocked. Both facts need recording.

## Requirements

1. **Filesystem cannot be disabled.** `PRIMITIVES_ARCHITECTURE.md` lists
   filesystem among the subsystems to disable. There is no `SDL_FILESYSTEM`
   option in SDL 3.4.16 and no `sdl-filesystem` feature in `sdl3-sys`; the
   `define_sdl_subsystem()` list has twelve entries and filesystem is not one.
   On Linux SDL always compiles its Unix filesystem implementation, and the
   generated header shows `SDL_FILESYSTEM_UNIX 1` with no
   `SDL_FILESYSTEM_DISABLED` to set. **Amend the architecture document**, or
   record the accepted size and the reason it is worth keeping — it is what
   `SDL_GetPrefPath` and friends need.

2. **The subsystem configuration has no channel.** The architecture document
   describes subsystems being disabled at build time, but `sdl3` 0.20.0
   re-exports **no** subsystem features at all — its feature table has 19
   entries and none is an SDL subsystem. The switches live on `sdl3-sys` 0.7.1.
   With the operator's default-subsystems decision this is currently moot; write
   down that it is moot and what unblocks it, so the day someone wants audio off
   they know the two options and their costs:
   - a second direct `sdl3-sys` dependency, which means keeping two version
     requirements compatible by hand;
   - options forced from the toolchain file, which is target-asymmetric —
     native passes no toolchain file — and invisible to anyone reading
     `Cargo.toml`.

3. **Hardware-dependent options, now that the target is closer.** Record a
   decision or an explicit deferral for each:
   - `sdl-hidapi` — on means USB gamepads and steering wheels enumerate; off
     means only evdev. `SDL_JOYSTICK_HIDAPI` is a sub-option of `SDL_HIDAPI` in
     SDL's CMake, so this is not a free choice.
   - haptics — absent from the architecture document entirely; a steering wheel
     with force feedback needs it.
   - audio — with the current allowlist there is no audio backend at all. Is the
     head unit expected to make sound?
   - `SDL_VULKAN` is independently defaulted on and is **not** affected by the
     GPU subsystem option. Decide whether it should be.

4. **Reconcile the rest of the document against the build.** `SDL_OPENGL` is off
   by design here (EGL + GLES only, no GLX) — confirm the head unit never needs
   desktop GL, and say so explicitly. `libunwind` is checked but is test-only
   and sets no `HAVE_`, so it is invisible in the backends summary whatever it
   does; either use it or stop checking it.

5. **Fix the one spec seam between task 04 and task 05.** Task 04 places "widget
   node structure" out of scope, deferring it to task 05 — but task 05 is *Property
   System* and never lists a widget node type among its requirements. This was
   flagged on 2026-09-28 and not yet settled. **Fix it in the task files**, and
   do it before task 04 dispatches, not here — the amendment is small but it
   must land before an agent guesses.

## Acceptance Criteria

- [ ] `PRIMITIVES_ARCHITECTURE.md` and SDL 3.4.16 agree on filesystem.
- [ ] The subsystem policy is recorded as a decision with a trigger for revisiting
      it, and the two possible channels are documented with their costs.
- [ ] Every hardware-dependent option has a decision or a dated deferral with a
      named owner question.
- [ ] The default-subsystems deviation is recorded as a deviation, with the date
      and the operator's reason.
- [ ] The task 04/05 seam is resolved **in the task files**, before task 04 runs.

## Out of Scope

- Any change to SDL's build — the mechanism questions belong to task 02's
  manifest, and the driver forcing to `doc/platform/TASK_CROSSPLATFORM_02.md`.
- The sysroot, the image, and the head unit — `doc/platform/TASK_CROSSPLATFORM_01.md`,
  `TASK_CROSSPLATFORM_03.md` and `TASK_CROSSPLATFORM_04.md`.
