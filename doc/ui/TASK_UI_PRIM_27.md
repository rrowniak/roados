# TASK_UI_PRIM_27: Target Runtime Library Audit

## Goal

Know exactly which shared libraries the built binary loads at runtime, and make
the target image ship all of them.

## Context

With `SDL_DEPS_SHARED=ON` — SDL's default, and the project's — SDL does not link
its platform libraries. It records their **sonames** and `dlopen`s them on the
target. The native build makes this concrete: `ldd` reports four dependencies
(`linux-vdso`, `libgcc_s`, `libc`, `ld-linux`) while the generated
`SDL_build_config.h` carries nine X11 sonames that are absent from the link line
entirely.

That is the right design, and it is also a deployment obligation the build
cannot discharge. A binary that links nothing can still fail to start, and it
will do so on the head unit rather than on the build machine.

## Requirements

1. **Enumerate every soname** SDL recorded, from the **target's**
   `SDL_build_config.h` — the `SDL_VIDEO_DRIVER_*_DYNAMIC*`,
   `SDL_AUDIO_DRIVER_*_DYNAMIC*` and any other `_DYNAMIC` entries. Do this on a
   real target configure, not the host's, because the two differ.

   **Count them correctly.** The first cut of this task said a naive pattern
   reports the wrong number, and it does — in the direction of over-reporting:

   ```sh
   grep -oE 'SDL_[A-Z0-9_]*DYNAMIC[A-Z0-9_]*' SDL_build_config.h | sort -u | wc -l
   # 26   <- WRONG: these are the macro NAMES in `#undef` lines
   grep -cE '^#define SDL_.*DYNAMIC' SDL_build_config.h
   # 0    <- correct for the current target build
   ```

   A `#undef` line names the macro, and the value is the soname. The `#undef`
   block is the mechanism, not a list. Two independent checks are worth running:
   count `^#define`, and `strings libSDL3.so.0 | grep -qx libdrm.so.2`. If they
   disagree, the header is the authority and the binary is the evidence.

   For the current target build the count is **0**, and that is the good case:
   every backend is either compiled in or absent, and the target `dlopen`s
   nothing. It is 0 *because* nothing was detected — KMSDRM failed, so it recorded
   no soname either. The moment task 26 makes KMSDRM work, this count becomes
   non-zero and the image obligation appears. **Do not read 0 as "nothing to
   ship" without re-running it after task 26.**

2. **Distinguish three kinds**, because they need different handling:
   - **Required at runtime** — e.g. `libdrm.so.2`, `libgbm.so.1` for KMSDRM.
     Must be in the image; absence is a boot-time failure.
   - **Host-only** — the X11 and Wayland sonames, which a target build with the
     toolchain file's forcing will not have. Confirm they are *absent from the
     header*, not merely unused. Measured on the real target build: all sixteen
     `SDL_VIDEO_DRIVER_X11_DYNAMIC*` and `SDL_VIDEO_DRIVER_WAYLAND_DYNAMIC*`
     macros are `#undef`, and none of `libX11.so.6` or
     `libwayland-client.so.0` appears in the binary.
   - **Optional / degraded** — backends whose absence only removes a feature.

3. **Cross-check against the image.** For each required soname, prove the target
   filesystem provides it. `ldconfig -p` inside the target root, or the image
   manifest.

4. **Verify the `dlopen` path actually resolves at runtime**, not just that the
   soname is recorded. A missing `libfoo.so.2` where `libfoo.so.1.2` exists is a
   real and common failure, and no build-time check catches it.

5. **Record the static-linking alternative and its cost.** `SDL_DEPS_SHARED=OFF`
   would remove this whole obligation and make a sysroot mandatory instead, per
   `CROSSBUILD.md` §4.3. Write down the trade rather than leaving it folklore.

6. **Fix the "no host isolation" warning if it still misleads.** The toolchain
   file states that `CMAKE_FIND_ROOT_PATH_MODE_*` are not a host-isolation
   guarantee, citing a measured case of a host `libX11` and a host
   `libdrm.so` being resolved during a cross configure. Once a sysroot exists,
   re-measure and say whether the warning is still needed or merely historical.

## Acceptance Criteria

- [ ] A complete list of target sonames, split by the three kinds above.
- [ ] Every required soname is proven present in the target image.
- [ ] `SDL_Init` on the target creates a window without a `dlopen` failure.
- [ ] The `SDL_DEPS_SHARED` trade-off is written down, with the sysroot
      implication.
- [ ] The toolchain file's host-isolation warning is either re-verified against
      the real sysroot or removed.

## Out of Scope

- Building the image or the sysroot — task 25.
- Making KMSDRM work — task 26.
- Booting the head unit — task 29.
