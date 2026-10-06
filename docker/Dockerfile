# A container image for this repository: opencode plus the § 2 prerequisites.
#
# Build context is `docker/`, not the repository root, and the reason is
# mechanical: the repository carries `ui/target/`, which is about 1.6 GB of
# build output, and this Dockerfile copies nothing out of the context. Passing
# the root would ship that gigabyte to the daemon on every build for nothing.
#
#   docker build -t roados-dev:dev -f docker/Dockerfile docker
#
# Run it with `docker/run.sh`, which is where the mounts are explained.
#
# **Why not `FROM ghcr.io/anomalyco/opencode`?** Two reasons, and the second is
# the one that decides it.
#
# 1. That image carries no build toolchain at all — no rustc, no cargo, no C
#    compiler, no cmake, no git. It cannot build `ui/`, which builds SDL 3.4.16,
#    SDL_image 3.4.6 and FreeType 2.13.2 from vendored C source (§1, and
#    `doc/ui/PRIMITIVES_ARCHITECTURE.md` § *Dependencies*).
# 2. It is Alpine, so it is musl, and this repository does not build musl. The
#    workspace's targets are `x86_64-unknown-linux-gnu` and
#    `aarch64-unknown-linux-gnu` (`AGENTS.md` § *Rust*), the aarch64 linker in
#    `.cargo/config.toml` is literally named `aarch64-linux-gnu-gcc` (§2.5), and
#    Debian/Ubuntu multiarch is one of the two strategies
#    `doc/platform/TASK_CROSSPLATFORM_01.md` requirement 1 is choosing between.
#    A musl image answers none of that and would build a third thing nobody
#    asked for.
#
# So this is Debian, and the opencode binary is installed from the release
# asset rather than copied out of the image above: that image's copy is linked
# against Alpine's `ld-musl-x86_64.so.1` and will not start on glibc.
FROM debian:trixie-slim

# The host's rustc as of 2026-10-05. The floor is 1.85 — `sdl3-sys` 0.7.1
# declares it, and `sdl3-sys` 0.7.1 is what `ui/Cargo.lock` pins (§2.1).
ARG RUST_VERSION=1.98.1
# The version the ghcr image carries, so the container and the host run the same
# agent. Bump both together or not at all.
ARG OPENCODE_VERSION=1.18.34

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/opencode/bin:$PATH

# §2.2 (build-essential, cmake, pkg-config, the C compiler), §2.3 (the X11
# extension `-dev` packages and the libdrm/gbm/udev trio), §2.5 (the three
# aarch64 packages, including binutils for the `gcc-ar` search that fails
# silently without it).
#
# `pkg-config` is explicit here even though §2.3's apt line leaves it out as
# implicit in build-essential: on this repository its absence is the one
# failure that is invisible — SDL's configure succeeds and drops KMSDRM and
# every other pkg-config-gated backend (§7.5).
#
# `zlib1g-dev` is not in any section above. It is here because
# `freetype-sys` builds its bundled libpng, libpng includes `zlib.h`, and
# `libz-sys` ships zlib's sources under `src/zlib` rather than its headers
# where libpng's include path expects them. Without it the build dies in
# `cc-rs` with `libpng/pngstruct.h:30:10: fatal error: zlib.h`. Observed
# 2026-10-05.
#
# The GL set (`libegl1`, `libgles2`, `libgl1`, `libglx-mesa0`,
# `libgl1-mesa-dri`) is *runtime* only, and none of it is a build requirement:
# SDL ships its own Khronos headers (§2.4) and links nothing but libc
# (§5.4). `libgl1-mesa-dri` is what gives the container a GLES 3.1+ driver,
# and it is the swrast fallback, so `ui_demo` still runs where `--device
# /dev/dri` reaches no GPU.
#
# The fonts are a hard requirement, not a nicety: `ui/src/ui_demo/src/main.rs`
# names `/usr/share/fonts/truetype/lato/Lato-Medium.ttf`, `Lato-Bold.ttf` and
# `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf` as absolute paths and
# propagates `Font::from_path`'s error with `?`, so a missing font is a failed
# run. `fonts-lato` and `fonts-dejavu-core` put them at exactly those paths.
#
# `x11-utils` (xwininfo) and `imagemagick` (`magick import`, `magick compare`)
# are the capture method `doc/ui/IMPLEMENTATION_STATE.md` § *Verifying a change
# that draws — the capture method* uses, and § *A change that alters what is on
# screen is not verified until it has been seen* in `.ai/agents/developer.md`
# makes that a gate rather than a convenience.
#
# `file` and `binutils` are what §3's `file target/debug/<binary>` and the
# `readelf -h` / `nm` checks are written against.
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake pkg-config git ca-certificates curl xz-utils \
      python3 zlib1g-dev file binutils \
      libx11-dev libxext-dev libxi-dev libxfixes-dev libxss-dev libxtst-dev \
      libxcursor-dev libxrandr-dev libxkbcommon-dev \
      libdrm-dev libgbm-dev libudev-dev \
      libegl1 libgles2 libgl1 libglvnd0 libglx-mesa0 libgl1-mesa-dri \
      fonts-lato fonts-dejavu-core \
      x11-utils imagemagick \
      gcc-aarch64-linux-gnu g++-aarch64-linux-gnu binutils-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

# `--profile minimal` because the default profile installs documentation for
# every target triple; `rustfmt` and `clippy` are named because minimal does
# not include them and both are in the verification suite
# (`.ai/agents/developer.md` § Phase 3 — Verify).
#
# The aarch64 std target is added because §4.2's cross command works today on
# this host — the platform *sequence* in `doc/platform/` is deferred pending the
# target-platform decision, but the build itself is not blocked, and a container
# that cannot run it is a container that reports a linker error instead of an
# artifact.
#
# `cargo-audit` is installed here because it is the sixth command in that same
# suite and `doc/ui/IMPLEMENTATION_STATE.md` records it as **not installed on
# the host** — a standing gap. Installing it costs about three minutes of image
# build and closes the gap for everyone who uses this image.
RUN curl -fsSL https://sh.rustup.rs \
      | sh -s -- -y --no-modify-path --profile minimal --default-toolchain "$RUST_VERSION" \
      --component rustfmt --component clippy \
 && rustup target add aarch64-unknown-linux-gnu \
 && cargo install cargo-audit --locked

RUN mkdir -p /usr/local/opencode/bin \
 && curl -fsSL "https://github.com/anomalyco/opencode/releases/download/v${OPENCODE_VERSION}/opencode-linux-x64.tar.gz" \
      | tar -xz -C /usr/local/opencode/bin \
 && chmod 755 /usr/local/opencode/bin/opencode \
 && /usr/local/opencode/bin/opencode --version

WORKDIR /workspace
ENTRYPOINT ["opencode"]