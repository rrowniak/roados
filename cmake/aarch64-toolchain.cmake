# CMake toolchain file: x86_64 Linux host -> aarch64 Linux target.
#
# Used by the vendored SDL3 build that `sdl3-sys` drives through the `cmake`
# crate. See doc/ui/CROSSBUILD.md for how it is wired into `cargo build`.
#
# Everything here is overridable, in this order:
#
#   1. `-D<variable>=<value>` on the cmake command line
#   2. the environment variable of the same name (ROADOS_* only)
#   3. the default in this file
#
# The file is read more than once per configure (once at the top level, and
# again inside each try_compile), so it must stay side-effect free apart from
# setting variables, and it must be safe to run in `cmake -P` script mode.
#
# Verified against CMake 4.2.3 with:
#   cmake -P probe.cmake   (include()s this file and prints every variable)
#   cmake -S <proj> -B <dir> -DCMAKE_TOOLCHAIN_FILE=<this file>
#   CMAKE_TOOLCHAIN_FILE=<this file> cargo build   (on a crate whose build.rs
#       uses the `cmake` crate, i.e. the path sdl3-sys takes)
# See doc/ui/CROSSBUILD.md, "Verification", for the recorded output.

# --- target identity -------------------------------------------------------

if(NOT ROADOS_TARGET_TRIPLE OR ROADOS_TARGET_TRIPLE STREQUAL "")
  if(DEFINED ENV{ROADOS_TARGET_TRIPLE} AND NOT "$ENV{ROADOS_TARGET_TRIPLE}" STREQUAL "")
    set(ROADOS_TARGET_TRIPLE "$ENV{ROADOS_TARGET_TRIPLE}")
  else()
    set(ROADOS_TARGET_TRIPLE "aarch64-linux-gnu")
  endif()
endif()
set(ROADOS_TARGET_TRIPLE "${ROADOS_TARGET_TRIPLE}"
    CACHE STRING "GNU target triple of the cross toolchain")

# Matches the Rust target triple this file is named after. The `cmake` crate
# would derive CMAKE_SYSTEM_NAME/PROCESSOR from cargo's TARGET variable when no
# toolchain file is given; setting them here makes plain `cmake` invocations
# agree with the cargo ones, which is what makes the two build paths debuggable
# against each other.
if(NOT CMAKE_SYSTEM_NAME)
  set(CMAKE_SYSTEM_NAME "Linux")
endif()
if(NOT CMAKE_SYSTEM_PROCESSOR)
  set(CMAKE_SYSTEM_PROCESSOR "aarch64")
endif()

# --- compilers -------------------------------------------------------------
#
# Deliberately NOT setting CMAKE_SYSROOT by default. On Debian/Ubuntu the
# gcc-aarch64-linux-gnu package already knows its own target layout
# (/usr/aarch64-linux-gnu/{include,lib} plus the multiarch search paths), and
# pointing CMAKE_SYSROOT at that directory breaks the build, because that
# directory is a target library root and not a sysroot -- it has no
# <sysroot>/usr/include inside it. Set ROADOS_SYSROOT only when the target
# really is a sysroot tree (Buildroot, Yocto, a hand-made rootfs).

if(NOT ROADOS_SYSROOT OR ROADOS_SYSROOT STREQUAL "")
  if(DEFINED ENV{ROADOS_SYSROOT} AND NOT "$ENV{ROADOS_SYSROOT}" STREQUAL "")
    set(ROADOS_SYSROOT "$ENV{ROADOS_SYSROOT}")
  endif()
endif()

foreach(_roados_tool cc cxx ar ranlib strip)
  if(NOT ROADOS_PREFIX_${_roados_tool} OR ROADOS_PREFIX_${_roados_tool} STREQUAL "")
    if(DEFINED ENV{ROADOS_PREFIX_${_roados_tool}} AND NOT "$ENV{ROADOS_PREFIX_${_roados_tool}}" STREQUAL "")
      set(ROADOS_PREFIX_${_roados_tool} "$ENV{ROADOS_PREFIX_${_roados_tool}}")
    else()
      set(ROADOS_PREFIX_${_roados_tool} "${ROADOS_TARGET_TRIPLE}-")
    endif()
  endif()
endforeach()

if(NOT CMAKE_C_COMPILER)
  set(CMAKE_C_COMPILER "${ROADOS_PREFIX_cc}gcc")
endif()
if(NOT CMAKE_CXX_COMPILER)
  set(CMAKE_CXX_COMPILER "${ROADOS_PREFIX_cxx}g++")
endif()
# CMake >= 3.4 derives AR/RANLIB/STRIP from the compiler's own prefix, but only
# when the compiler is found via find_program. Being explicit keeps the answer
# the same whether or not that inference succeeded.
#
# These three are CMAKE_AR and friends, which the plain archive rule uses
# (Modules/CMakeCInformation.cmake). They are NOT CMAKE_<LANG>_COMPILER_AR, which
# CMake searches for separately as <prefix>gcc-ar next to the compiler and uses
# for the IPO archive rules (Modules/Compiler/GNU.cmake,
# Modules/Compiler/GNU-FindBinUtils.cmake).
#
# CMAKE_<LANG>_COMPILER_AR is deliberately not set here. Setting it would work:
# find_program does not re-search when the variable already holds a value, so a
# plain set() sticks and suppresses the search. It is left to CMake's search
# because a hand-set value is a normal variable, so it never reaches
# CMakeCache.txt -- which is where anyone diagnosing a missing archiver looks
# first -- and because a hardcoded path gives up CMake's prefix- and
# version-aware search (gcc-ar-13, gcc-ar-12, ...). Install
# binutils-aarch64-linux-gnu, which provides aarch64-linux-gnu-gcc-ar. IPO
# builds fail at the archive step otherwise; plain, non-IPO builds never consult
# it. If you are stuck with a toolchain that ships no gcc-ar at all, setting
# CMAKE_C_COMPILER_AR here is a working last resort.
if(NOT CMAKE_AR)
  set(CMAKE_AR "${ROADOS_PREFIX_ar}ar")
endif()
if(NOT CMAKE_RANLIB)
  set(CMAKE_RANLIB "${ROADOS_PREFIX_ranlib}ranlib")
endif()
if(NOT CMAKE_STRIP)
  set(CMAKE_STRIP "${ROADOS_PREFIX_strip}strip")
endif()

# --- sysroot and find_* isolation -----------------------------------------
#
# These four modes are the standard cross-compilation set, and the PROGRAM
# exception is the load-bearing part: without it, CMake could not find the
# generator, git, or python3 once a sysroot is in play.
#
# The ONLY modes are NOT, on their own, a host-isolation guarantee. Measured on
# 2026-09-28: with an empty CMAKE_FIND_ROOT_PATH, SDL 3.4.16's configure still
# resolved the host's X11 headers (/usr/include/X11/XKBlib.h) and its host
# libX11. So do not read the modes below as "a host library can never be found".
# What actually keeps the wrong architecture out of a sysroot-less build is
# SDL3's own SDL_DEPS_SHARED=ON default: it records sonames such as
# "libX11.so.6" in SDL_build_config.h and dlopen()s them on the target, so the
# host path it found at configure time never reaches the link line.
#
# The moment that stops being true -- SDL_DEPS_SHARED=OFF, i.e. statically
# linked platform dependencies -- a sysroot becomes mandatory, and
# ROADOS_SYSROOT below is what supplies one.

if(ROADOS_SYSROOT AND NOT ROADOS_SYSROOT STREQUAL "")
  if(NOT CMAKE_SYSROOT)
    set(CMAKE_SYSROOT "${ROADOS_SYSROOT}")
  endif()
  if(NOT CMAKE_FIND_ROOT_PATH)
    set(CMAKE_FIND_ROOT_PATH "${ROADOS_SYSROOT}")
  endif()
  # Debian-style multiarch layout, plus the plain layout, so either works.
  #
  # Do not read this as load-bearing: project() derives CMAKE_LIBRARY_ARCHITECTURE
  # from the compiler it probed and overwrites whatever was set beforehand, so a
  # toolchain file cannot decide this. Measured 2026-09-28 by printing the value
  # both here and after project(): "aarch64-linux-gnu" at this point,
  # "x86_64-linux-gnu" (the stand-in compiler's own) in the project body. The real
  # value therefore comes from a real aarch64-linux-gnu-gcc, which is the correct
  # one. This line only decides the script-mode (-P) case, where project() never
  # runs. doc/ui/CROSSBUILD.md section 8, item 1 has the measurement.
  if(NOT CMAKE_LIBRARY_ARCHITECTURE)
    set(CMAKE_LIBRARY_ARCHITECTURE "${ROADOS_TARGET_TRIPLE}")
  endif()
  if(NOT PKG_CONFIG_SYSROOT_DIR)
    set(PKG_CONFIG_SYSROOT_DIR "${ROADOS_SYSROOT}")
  endif()
  if(NOT PKG_CONFIG_LIBDIR)
    set(PKG_CONFIG_LIBDIR
        "${ROADOS_SYSROOT}/usr/lib/${ROADOS_TARGET_TRIPLE}/pkgconfig"
        "${ROADOS_SYSROOT}/usr/lib/pkgconfig"
        "${ROADOS_SYSROOT}/usr/share/pkgconfig"
        "${ROADOS_SYSROOT}/usr/local/lib/pkgconfig")
  endif()
endif()

# Host tools (git, python3, ninja, the generator) must stay reachable.
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM "NEVER")
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY "ONLY")
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE "ONLY")
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE "ONLY")

# --- pkg-config ------------------------------------------------------------
#
# The reason this is here is NOT the one an earlier draft of this file gave.
# That draft said SDL "only calls pkg_check_modules for libinotify", which is
# true of SDL's CMakeLists.txt and false of the tree as a whole. Measured against
# sdl3-src 3.4.16: cmake/sdlchecks.cmake has 14 pkg_check_modules calls behind 11
# if(PKG_CONFIG_FOUND) guards, in 11 macros; CMakeLists.txt adds one more for
# BSD-only libinotify. Fifteen calls, twelve guards, in SDL's own configure.
# They gate:
#
#   KMSDRM          libdrm, gbm            <- the head unit's scanout path
#   PIPEWIRE        libpipewire-0.3        \
#   PULSEAUDIO      libpulse               |  four of the five Linux audio
#   JACK            jack                   |  backends (ALSA is the fifth and
#   SNDIO           sndio                  /  is not pkg-config gated)
#   FRIBIDI         fribidi                \  complex text shaping
#   LIBTHAI         libthai                /
#   WAYLAND         wayland-client, wayland-egl, wayland-cursor, egl,
#                   xkbcommon; then libdecor-0
#   LIBUNWIND, and the RPI(bcm_host, brcmegl)/ROCKCHIP(mali) GPU paths
#
# ALSA is absent from that list on purpose: CheckALSA uses
# find_package(ALSA MODULE), not pkg_check_modules, so it is unaffected by a
# missing pkg-config and fails for a different reason (no libasound2-dev).
# Likewise a tree-wide grep finds two further call sites -- FindLibUSB.cmake and
# FindFFmpeg.cmake -- that are not in the fifteen: both guard on
# PKG_CONFIG_FOUND and both keep find_library/find_path fallbacks. The two
# pkg_check_modules(... REQUIRED ...) calls under src/hidapi/ belong to the
# vendored hidapi's standalone build, which SDL never add_subdirectory()s.
#
# Every one of those is wrapped in `if(PKG_CONFIG_FOUND)`, and CheckKMSDRM has no
# `else()` branch at all. So a missing pkg-config does not fail the configure --
# it quietly drops the subsystem. Measured 2026-09-28 on a host with no
# pkg-config, where the native build succeeded and printed:
#
#   -- Could NOT find PkgConfig (missing: PKG_CONFIG_EXECUTABLE)
#   --   SDL_KMSDRM                  (Wanted: ON): OFF
#   --   SDL_KMSDRM_SHARED           (Wanted: ON): OFF
#   --   SDL_PIPEWIRE                (Wanted: ON): OFF
#   --   SDL_PULSEAUDIO              (Wanted: ON): OFF
#   ...
#
# Read that line correctly before citing it: "Wanted:" is the option and the
# value after the colon is HAVE_<name>, the detection result (cmake/macros.cmake
# message_tested_option). So "(Wanted: ON): OFF" means the option was on and the
# driver was not found -- not that the option was switched off. SDL_KMSDRM is
# still BOOL=ON in the cache of that same build.
#
# and SDL_build_config.h came out with /* #undef SDL_VIDEO_DRIVER_KMSDRM */.
# That is the head-unit build succeeding with no scanout driver in it.
#
# For the target this means aarch64-linux-gnu-pkg-config (plus libdrm and gbm
# for the target architecture) is a HARD requirement, not a convenience. The
# find_program below already prefers the target-prefixed name, so installing it
# is all that is needed -- but note that with ROADOS_SYSROOT unset, a host
# pkg-config would find the HOST's libdrm and record its path. That is a real
# leak, and it is why a sysroot is not optional for this target. See
# doc/ui/CROSSBUILD.md sections 2.2 and 6.7.

if(NOT PKG_CONFIG_EXECUTABLE)
  if(DEFINED ENV{PKG_CONFIG} AND NOT "$ENV{PKG_CONFIG}" STREQUAL "")
    set(PKG_CONFIG_EXECUTABLE "$ENV{PKG_CONFIG}")
  else()
    find_program(ROADOS_CROSS_PKG_CONFIG
                 NAMES "${ROADOS_TARGET_TRIPLE}-pkg-config" pkg-config)
    if(ROADOS_CROSS_PKG_CONFIG)
      set(PKG_CONFIG_EXECUTABLE "${ROADOS_CROSS_PKG_CONFIG}")
    endif()
  endif()
endif()

# --- SDL video drivers: framebuffer-only target -----------------------------
#
# This block is deliberately ASYMMETRIC, and making it symmetric would break the
# development workflow. The two builds have genuinely different requirements:
#
#   native  keeps X11, because the dev host runs the task 24 demo in a real
#           window. There is no toolchain file on a native build, so nothing in
#           this file applies to it, and nothing needs to.
#   aarch64  has no desktop stack at all. The head unit scans out to a panel
#           through KMSDRM, and shipping X11 or Wayland to it would mean
#           carrying a compositor-shaped dependency on a device that has no
#           compositor.
#
# If you are reading this and thinking "this should be symmetric" -- it should
# not. Adding SDL_X11=ON here, or removing the two OFF lines below, does not make
# the targets agree; it makes the demo stop opening a window on the dev host.
# The asymmetry is the design. doc/ui/CROSSBUILD.md section 5.4 has the
# reasoning and the measurements.
#
# Turning X11 off for the target is also what makes the X11 development packages
# unnecessary there, and that is mechanically clean rather than merely tidy:
# CheckX11() opens with `if(SDL_X11)` and every probe inside it -- find_package(X11)
# and all nine SDL_X11_X<NAME> checks -- is nested under that one condition. With
# the option off, none of them run and none of those packages are looked for.
#
# SDL_UNIX_CONSOLE_BUILD IS A MANIFEST REQUIREMENT, not a setting of this file.
# The two are a matched pair and one without the other fails: with X11 and
# Wayland both off, SDL hits this in cmake/macros.cmake:413-414
#
#     if(NOT (HAVE_X11 OR HAVE_WAYLAND))
#       if(NOT SDL_UNIX_CONSOLE_BUILD)
#         message(FATAL_ERROR "SDL could not find X11 or Wayland ...")
#
# and the configure dies. So Cargo.toml needs, on sdl3:
#
#     sdl3 = { version = "0.20", features = [
#         "build-from-source",
#         "build-from-source-unix-console",      # <- suppresses that FATAL_ERROR
#     ] }
#
# The manifest is the right channel for it because, unlike SDL_X11, this option
# IS re-exported by sdl3 (sdl3's Cargo.toml, line 78:
# build-from-source-unix-console = ["sdl3-sys/sdl-unix-console-build"]), and it
# is harmless natively: SDL reads SDL_UNIX_CONSOLE_BUILD at exactly one place --
# the check quoted above -- and enables no driver with it. So one manifest serves
# both targets; it is not a per-target setting in practice. The X11/Wayland
# forcing below is target-asymmetric precisely because those two are NOT
# re-exported.
#
# SDL_KMSDRM is also absent from sdl3-sys's cmake_vars! list, so it too is
# unreachable from Cargo.toml, and it also defaults to ON on Unix. It is left
# alone here on purpose: it needs libdrm and gbm .pc files for the TARGET, and
# the target image is not decided yet. Forcing it on before those exist would
# add a step that cannot be verified. When the sysroot is settled, forcing
# SDL_KMSDRM=ON here is the mechanism, exactly as below.
#
# Override discipline matches the rest of the file: -D wins, then ROADOS_SDL_*
# from the environment, then these defaults. `if(NOT DEFINED ...)` is what gives
# -D the win -- SDL_X11 is not defined yet when a toolchain file is read, so a
# -DSDL_X11=ON arrives as an existing cache entry and this block leaves it alone,
# while no -D means the block supplies OFF. It is also idempotent across the
# repeated reads, because after the first read the entry is in the cache.

foreach(_roados_sdl_var SDL_X11 SDL_WAYLAND)
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

# --- target tuning ---------------------------------------------------------
#
# Appended to CMAKE_C_FLAGS rather than seeded through CMAKE_C_FLAGS_INIT, and
# the reason is specific to this build: the `cmake` crate always passes
# -DCMAKE_C_FLAGS=<flags derived from cc::Build> on the command line, so
# CMAKE_C_FLAGS is already a cache entry by the time this file is read and
# CMAKE_C_FLAGS_INIT would be ignored. The flag would then be dropped silently,
# and the artifact would be a plain aarch64 build with no -march at all.
#
# The append is idempotent, because the toolchain file is read again for every
# try_compile, and each of those reads sees the previous value in the cache.
#
# armv8-a is the floor for this project. The head unit SoC may want more, which
# is what ROADOS_ARCH_FLAGS is for.

if(NOT ROADOS_ARCH_FLAGS OR ROADOS_ARCH_FLAGS STREQUAL "")
  if(DEFINED ENV{ROADOS_ARCH_FLAGS} AND NOT "$ENV{ROADOS_ARCH_FLAGS}" STREQUAL "")
    set(ROADOS_ARCH_FLAGS "$ENV{ROADOS_ARCH_FLAGS}")
  else()
    set(ROADOS_ARCH_FLAGS "-march=armv8-a")
  endif()
endif()

string(FIND "${CMAKE_C_FLAGS}" "${ROADOS_ARCH_FLAGS}" _roados_c_flags_pos)
if(_roados_c_flags_pos EQUAL -1)
  set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${ROADOS_ARCH_FLAGS}"
      CACHE STRING "Flags used by the C compiler" FORCE)
endif()
string(FIND "${CMAKE_CXX_FLAGS}" "${ROADOS_ARCH_FLAGS}" _roados_cxx_flags_pos)
if(_roados_cxx_flags_pos EQUAL -1)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${ROADOS_ARCH_FLAGS}"
      CACHE STRING "Flags used by the CXX compiler" FORCE)
endif()

# --- compiler check -------------------------------------------------------
#
# CMake's compiler check builds and links a program. Keeping it a link test is
# the honest default: it is the check that catches a broken sysroot, and on a
# stock Debian/Ubuntu cross toolchain it passes. Set
# ROADOS_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY to get a configure-only check
# when bringing up a toolchain whose link step is still incomplete -- at the
# cost of deferring the link failure to the first real build.

if(NOT ROADOS_TRY_COMPILE_TARGET_TYPE OR ROADOS_TRY_COMPILE_TARGET_TYPE STREQUAL "")
  if(DEFINED ENV{ROADOS_TRY_COMPILE_TARGET_TYPE} AND NOT "$ENV{ROADOS_TRY_COMPILE_TARGET_TYPE}" STREQUAL "")
    set(ROADOS_TRY_COMPILE_TARGET_TYPE "$ENV{ROADOS_TRY_COMPILE_TARGET_TYPE}")
  else()
    set(ROADOS_TRY_COMPILE_TARGET_TYPE "EXECUTABLE")
  endif()
endif()
if(NOT CMAKE_TRY_COMPILE_TARGET_TYPE)
  set(CMAKE_TRY_COMPILE_TARGET_TYPE "${ROADOS_TRY_COMPILE_TARGET_TYPE}")
endif()
