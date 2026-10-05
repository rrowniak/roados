# SDL3 build options that are the same on every target.
#
# This file is NOT a toolchain file. It sets no compiler, no sysroot, no
# `CMAKE_SYSTEM_NAME` and no `-march`, which is exactly why it can be read by a
# native x86_64 build and by the aarch64 cross build alike. The options that
# differ per target live in `cmake/aarch64-toolchain.cmake`, which includes
# this file; see doc/ui/CROSSBUILD.md §4.1 and §5.4 for how each target reaches
# it.
#
# **Why this file exists at all.** None of the three options below is reachable
# from `Cargo.toml`. `sdl3-sys`'s `cmake_vars!` block (`build.rs:30-48`) covers
# the twelve subsystems plus `SDL_ASAN`, `SDL_CCACHE`, `SDL_LIBC`, `SDL_RPATH`
# and `SDL_UNIX_CONSOLE_BUILD` — and nothing else. `SDL_VULKAN`, `SDL_OPENGL`
# and `SDL_TEST_LIBRARY` are not in that list, so no cargo feature on either
# crate can set them, and the environment reaching SDL's configure is the only
# channel left.
#
# The three options, and why each one is off.
#
# **SDL_VULKAN.** `PRIMITIVES.md` § *Backend* rejected Vulkan: open-driver
# support on Mali/VideoCore is immature, so GLES 3.1 is the baseline. The same
# document records that SDL's GPU API does not support OpenGL ES at all -- only
# Vulkan, D3D12 and Metal -- and this project renders with `SDL_GL_CreateContext`
# and its own GLES 3.1 pipeline, so the GPU subsystem is unusable here as well.
# Left at SDL's default this compiled 716 KB of Vulkan objects into both builds
# (488 KB of it the GPU Vulkan backend) that are never called: SDL only loads
# `libvulkan.so.1` for a window created with `SDL_WINDOW_VULKAN`
# (`SDL_video.c:2690`) or from `SDL_Vulkan_CreateSurface` (`:6336`), and the
# window is built `.opengl()` only. The default is ON because `CheckVulkan`
# (`sdlchecks.cmake:928-938`) probes nothing at all -- it sets
# `SDL_VIDEO_VULKAN 1` and returns.
#
# **SDL_OPENGL.** Desktop OpenGL, reached through GLX. This project uses OpenGL
# ES 3.1 through EGL (`PRIMITIVES.md` § *Backend*, `CROSSBUILD.md` §2.4), and the
# two are separate SDL options: `CheckOpenGL` is gated on `SDL_OPENGL`
# (`sdlchecks.cmake:884`) and `CheckOpenGLES` on `SDL_OPENGLES` (`:902`), with no
# dependency between them, so turning this off cannot affect the GLES path.
# **This is not a cosmetic setting.** `dep_option(SDL_OPENGL ... ON)` is ON by
# default and it was ON in both `CMakeCache.txt` files; it came out undef only
# because `FindOpenGLHeaders()` could not compile `<GL/gl.h>` on this host, i.e.
# `libgl-dev` is not installed. Install that package and a native build silently
# grows GLX code that nothing uses and no test notices -- a successful build of a
# different artefact, which is the failure mode `CROSSBUILD.md` §6.7 records for
# a missing `pkg-config`.
#
# **SDL_TEST_LIBRARY.** SDL builds a static `SDL3_test` library by default
# (`CMakeLists.txt:399`) purely to hold its own test fixtures. Nothing in this
# workspace links it, and building it runs `CheckLibUnwind`
# (`sdlchecks.cmake:1418-1481`), a three-stage compile/link/pkg-config probe for
# a *test-only* dependency whose result is `HAVE_LIBUNWIND_H` as a PRIVATE
# definition on `SDL3_test` -- so it cannot appear in the library's
# `SDL_build_config.h` whatever it finds, and its outcome is invisible in the
# backends summary. Turning the target off removes a 228 KB archive and a probe
# whose answer nothing can read.

# Override discipline matches `aarch64-toolchain.cmake`: `-D` wins, then
# `ROADOS_SDL_*` from the environment, then the defaults here. `if(NOT DEFINED
# ...)` is what gives `-D` the win -- SDL's options are cache entries by the time
# a toolchain file is read, so a `-DSDL_VULKAN=ON` arrives already defined and
# this block leaves it alone. It is also idempotent across the repeated reads,
# because after the first read the entry is in the cache.
#
# The file is read more than once per configure and must stay side-effect free
# apart from setting variables, so it is also safe to run under `cmake -P`.

foreach(_roados_sdl_var SDL_VULKAN SDL_OPENGL SDL_TEST_LIBRARY)
  if(NOT DEFINED ${_roados_sdl_var})
    if(DEFINED ENV{ROADOS_${_roados_sdl_var}}
       AND NOT "$ENV{ROADOS_${_roados_sdl_var}}" STREQUAL "")
      set(${_roados_sdl_var} "$ENV{ROADOS_${_roados_sdl_var}}"
          CACHE BOOL "SDL ${_roados_sdl_var} (set by ${CMAKE_CURRENT_LIST_FILE})")
    else()
      set(${_roados_sdl_var} OFF
          CACHE BOOL "SDL ${_roados_sdl_var} (set by ${CMAKE_CURRENT_LIST_FILE})")
    endif()
  endif()
endforeach()
unset(_roados_sdl_var)
