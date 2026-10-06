#!/usr/bin/env bash
#
# Run opencode against this repository inside the `docker/Dockerfile` image.
#
#   docker/run.sh                     # interactive opencode, cwd /workspace
#   docker/run.sh run --version       # a one-shot opencode invocation
#   ROADOS_ENTRYPOINT=sh docker/run.sh -c 'cd ui && cargo test'
#   ROADOS_REPO=~/src/roados docker/run.sh
#
# What is isolated: the filesystem — only the repository at /workspace plus
# three cache directories is visible — and the process tree. What is passed
# through, because `ui_demo` cannot draw without it, is the X11 socket, the GPU
# device nodes and the build caches. Every mount below names the document that
# makes it a requirement; nothing else is in the container.
#
#   $repo → /workspace
#       The repository, read-write. It is mounted whole rather than `ui/`
#       because `HOST_CMAKE_TOOLCHAIN_FILE = "cmake/sdl-options.cmake"` and
#       `TARGET_CMAKE_TOOLCHAIN_FILE = "cmake/aarch64-toolchain.cmake"` in
#       `.cargo/config.toml` are `relative = true` and therefore resolve against
#       the `cmake/` directory beside that file, and Cargo reads a config only
#       from the working directory and its ancestors — mount one level lower and
#       both toolchain files, and the `cmake/` they name, are gone.
#
#   $cache/cargo → /home/dev/.cargo
#       `$CARGO_HOME`: the crates.io index and compiled registry, so the next
#       container does not re-fetch and re-extract 47 dependencies. It is
#       deliberately **not** `/usr/local/cargo` — rustup's shims and
#       `cargo-audit` are installed at image-build time under
#       `/usr/local/cargo/bin`, which is on `PATH`, and a volume over
#       `/usr/local/cargo` hides the shims themselves. That failure is
#       `sh: rustc: not found` with no other symptom. Redirecting `CARGO_HOME`
#       keeps the toolchain read-only and gives the registry somewhere writable
#       for the unprivileged `--user`.
#
#   $cache/target → /workspace/ui/target
#       `CARGO_TARGET_DIR` by another name, and a volume rather than the host's
#       own `ui/target` so the container's ~1.6 GB of SDL, SDL_image, FreeType
#       and demo objects do not collide with the host's. It is also where
#       `.ai/tools/fps-check.sh` looks for `./target/release/ui_demo`. The demo
#       still finds `demo.png`: it walks *up* from the executable to
#       `src/ui_demo/assets/demo.png` and never consults the working directory
#       (`ui/src/ui_demo/src/main.rs`, `ASSET_RELATIVE`). `ROADOS_ASSET_DIR` is
#       set as well, so the asset resolves from either `/workspace` or
#       `/workspace/ui`.
#
#   $cache/home → /home/dev
#       `HOME`, so opencode's store (`~/.local/share/opencode`: sessions,
#       provider) and `git`'s user config survive between runs. It is a
#       *separate* directory from the host's `~/.local/share/opencode` on
#       purpose: that one is a 7 GB SQLite database, and two opencode processes
#       writing it concurrently is a way to lose a session.
#
#   /tmp/.X11-unix, read-only
#       The X11 socket. `ui_demo` opens a real window and has no headless path:
#       `Context::new` fails on `gl_create_context` under SDL's `dummy` driver,
#       and `.ai/tools/fps-check.sh` exits 1 with "DISPLAY is not set" before it
#       builds anything. The `--tmpfs /tmp` below is mounted *after* this one —
#       Docker orders mounts by path depth — so the socket survives under it.
#       Read-only because the demo connects to X, it does not serve it.
#
#   /run/user/$uid, read-only
#       The Xauthority file and the XDG runtime directory. On this host
#       `XAUTHORITY` is a per-session path (`/run/user/1000/xauth_*`), not
#       `~/.Xauthority`, so the directory is what stays valid across a login
#       session. Without it the failure is a bare "cannot open display", and
#       `xwininfo` is the first thing to try.
#
#   --device /dev/dri
#       GPU nodes, so the OpenGL ES 3.1 context `Context::new` insists on comes
#       from the real driver. `libgl1-mesa-dri` in the image is the fallback, so
#       this is an accelerator rather than a requirement: drop it and the demo
#       runs on llvmpipe, slower and no longer comparable against the baseline in
#       `doc/ui/IMPLEMENTATION_STATE.md` § *The frame rate, measured*.
#
#   --tmpfs /tmp:rw,exec,size=2g
#       Scratch that dies with the container, which is what `/tmp` should mean.
#       `exec` is load-bearing rather than a shortcut: the input-injection helper
#       in `doc/ui/IMPLEMENTATION_STATE.md` § *Verifying a change that draws — the
#       capture method* is a throwaway C program built against `libXtst` and run
#       from `/tmp`, and a noexec tmpfs fails it with "permission denied". The
#       size is for SDL's CMake configure, cargo's registry extraction and the
#       capture PNGs on a cold run.
#
#   --cap-drop ALL --security-opt no-new-privileges
#       The demo needs no capability. Verified with both flags set: release
#       build, X11 window, GPU context, `fps-check.sh` at 63.1 fps against the
#       55 fps floor.
#
# Not passed in, on purpose: `--privileged`, `--network host`, `--pid host`, the
# host's `$HOME`, and `--device /dev/snd`. The last is the tempting one — SDL
# configures an audio backend — and nothing in this repository makes a sound, so
# the device buys nothing and hands over a DMA-capable path to the hardware.
set -euo pipefail

image=${ROADOS_IMAGE:-roados-dev:dev}
# `ROADOS_ENTRYPOINT` exists so the same mounts can be checked without spending
# a model call: `ROADOS_ENTRYPOINT=sh docker/run.sh -c 'cargo test'`. Everything
# below the entrypoint is unchanged, which is the point — a shell in the
# container is the same container opencode runs its own bash tool in.
entrypoint=${ROADOS_ENTRYPOINT:-opencode}
# The repository root is the parent of this script's directory, so the command
# works from anywhere. Not $PWD: a caller standing anywhere else in the tree
# would mount the wrong thing and get a confusing "no such manifest" instead.
repo=${ROADOS_REPO:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)}
cache=${ROADOS_CACHE:-$HOME/.cache/roados-dev}
uid_=$(id -u)
gid_=$(id -g)

# Numeric, not by name: debian:trixie-slim has a `video` group but no `render`
# group, and a `--group-add` naming a group the container does not have is
# ignored without a word — leaving /dev/dri/renderD128 unreadable, which surfaces
# as a missing GL context rather than as a permission error. A number needs no
# entry in the container's /etc/group to be honoured.
render_gid=$(getent group render | cut -d: -f3)
video_gid=$(getent group video | cut -d: -f3)

if ! docker image inspect "$image" >/dev/null 2>&1; then
  echo "run.sh: $image is not built; building it now (context: docker/)" >&2
  # Context is docker/, not the repository root: this Dockerfile copies nothing
  # out of the context, and the root carries ui/target's 1.6 GB with it.
  docker build -t "$image" -f "$repo/docker/Dockerfile" "$repo/docker" >&2
fi

mkdir -p "$cache/cargo" "$cache/target" "$cache/home"

# `-it` only when there is a terminal to attach to, so this script is also
# usable from a script or a tool call that has none.
tty=()
if [ -t 0 ] && [ -t 1 ]; then
  tty=(-it)
fi

exec docker run --rm ${tty[@]+"${tty[@]}"} \
  --user "$uid_:$gid_" \
  --group-add "$video_gid" \
  --group-add "$render_gid" \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  --workdir /workspace \
  --env HOME=/home/dev \
  --env CARGO_HOME=/home/dev/.cargo \
  --env DISPLAY="${DISPLAY:-:0}" \
  --env XAUTHORITY="${XAUTHORITY:-}" \
  --env XDG_RUNTIME_DIR="/run/user/$uid_" \
  --env ROADOS_ASSET_DIR=/workspace/ui/src/ui_demo/assets \
  --volume "$repo:/workspace" \
  --volume "$cache/cargo:/home/dev/.cargo" \
  --volume "$cache/target:/workspace/ui/target" \
  --volume "$cache/home:/home/dev" \
  --volume /tmp/.X11-unix:/tmp/.X11-unix:ro \
  --volume "/run/user/$uid_:/run/user/$uid_:ro" \
  --device /dev/dri \
  --tmpfs /tmp:rw,exec,size=2g \
  --shm-size 1g \
  --entrypoint "$entrypoint" \
  "$image" "$@"