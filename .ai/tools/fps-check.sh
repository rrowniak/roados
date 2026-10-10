#!/usr/bin/env bash
#
# Measure `ui_demo`'s frame rate and, if a floor is given, judge it.
#
#   .ai/tools/fps-check.sh                 # measure a 10 s release run
#   .ai/tools/fps-check.sh 5 55            # measure 5 s and require 55 fps
#   FPS_MIN_FPS=55 .ai/tools/fps-check.sh  # the same floor, from the environment
#
# Exit status is 0 when the run happened and met the floor (or when no floor was
# given), and 1 when it did not meet one. "The run happened" is checked rather
# than assumed: a missing report line is an aborted measurement, not a fast one,
# because a demo that failed to open a window prints no `roados-fps` line at all
# and a script that treated that as a pass would certify a broken build.
#
# The demo bounds its own run through ROADOS_RUN_SECONDS and prints one
# `roados-fps key=value …` line on stdout when it stops — see `fps.rs` in
# `ui/src/ui_demo/src/` for the format and why it is that shape.
#
# The baseline this compares against is **not** written here. It is a number
# about a machine and a build, and it belongs to whoever measured it;
# `README.md` § *Frame-rate baseline* carries the one this repository has. This
# script takes the floor as an argument so that there is one copy of the number
# and not two.
set -uo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
workspace=$(cd -- "$here/../.." && pwd)/ui

seconds=${1:-10}
floor=${2:-${FPS_MIN_FPS:-}}

if [ -z "${DISPLAY:-}" ]; then
  echo "fps-check: DISPLAY is not set, and ui_demo needs a display to draw at all." >&2
  echo "fps-check: a frame rate cannot be measured without frames." >&2
  exit 1
fi

# Both arguments are checked before anything is built, and the run below carries a
# `timeout` as well: a run length of **zero** is a number the demo *refuses* — it
# treats zero as "no bound" and runs until its window closes — so a script that
# passed it through would sit here for ever, which is the one way this tool could
# hang instead of failing. A floor that is not a number would reach `awk`, become
# 0 there, and turn every run into a pass.
if ! printf '%s' "$seconds" | grep -qE '^[0-9]+(\.[0-9]+)?$'; then
  echo "fps-check: the first argument is the run length in seconds; '$seconds' is not one." >&2
  exit 1
fi
if ! awk -v s="$seconds" 'BEGIN { exit !(s + 0 > 0) }'; then
  echo "fps-check: a run of $seconds s measures no frames; give it at least one." >&2
  exit 1
fi
if [ -n "$floor" ] && ! printf '%s' "$floor" | grep -qE '^[0-9]+(\.[0-9]+)?$'; then
  echo "fps-check: the second argument is a frame rate in fps; '$floor' is not one." >&2
  exit 1
fi

echo "fps-check: building the release binary" >&2
if ! (cd -- "$workspace" && cargo build --release) >&2; then
  echo "fps-check: the build failed, so there is nothing to measure." >&2
  exit 1
fi

log=$(mktemp)
trap 'rm -f "$log"' EXIT INT TERM

# The backstop for a demo that ignored its bound: the run length plus half a
# minute, so the normal exit is the demo's own and a hang is `timeout`'s.
cap=$(awk -v s="$seconds" 'BEGIN { printf "%d", s + 30 }')

echo "fps-check: running ui_demo for ${seconds}s" >&2
if ! (cd -- "$workspace" && ROADOS_RUN_SECONDS="$seconds" timeout "$cap" \
    ./target/release/ui_demo) >"$log" 2>&1; then
  echo "fps-check: ui_demo did not exit cleanly (or was cut off at ${cap}s); its output was:" >&2
  cat "$log" >&2
  exit 1
fi

report=$(grep -m1 '^roados-fps ' "$log")
if [ -z "$report" ]; then
  echo "fps-check: no 'roados-fps' line in the run's output — NOT A RESULT." >&2
  echo "fps-check: the demo ran but printed no report, so nothing was measured." >&2
  cat "$log" >&2
  exit 1
fi

field() { # name
  local value
  value=$(printf '%s\n' "$report" | tr ' ' '\n' | grep -m1 "^$1=") || return 1
  printf '%s\n' "${value#*=}"
}

average=$(field average_fps) || { echo "fps-check: no average_fps in: $report" >&2; exit 1; }
frames=$(field frames) || { echo "fps-check: no frames in: $report" >&2; exit 1; }
duration=$(field duration_s) || { echo "fps-check: no duration_s in: $report" >&2; exit 1; }
worst=$(field worst_frame_ms) || { echo "fps-check: no worst_frame_ms in: $report" >&2; exit 1; }
long=$(field long_frames) || { echo "fps-check: no long_frames in: $report" >&2; exit 1; }

cat <<EOF
fps-check: $frames frames in ${duration}s
fps-check: average ${average} fps, worst frame ${worst} ms, ${long} frame(s) over 33 ms
EOF

if [ -z "$floor" ]; then
  echo "fps-check: no floor given, so this is a measurement and not a verdict."
  exit 0
fi

# `awk` for the comparison, not bash arithmetic: these are decimals, and
# `[ "$average" -lt "$floor" ]` is an integer comparison that would either error
# or silently truncate 49.7 to 49.
if awk -v got="$average" -v want="$floor" 'BEGIN { exit !(got + 0 < want + 0) }'; then
  echo "fps-check: FAIL — ${average} fps is below the ${floor} fps floor."
  exit 1
fi

echo "fps-check: PASS — ${average} fps is at or above the ${floor} fps floor."
if [ "$long" -gt 0 ]; then
  echo "fps-check: ${long} frame(s) took more than 33 ms, which is two of the loop's own 16 ms slots."
fi