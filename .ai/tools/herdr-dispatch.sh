#!/usr/bin/env bash
# Run one worker agent in its own herdr tab and print its result file.
# Usage: herdr-dispatch.sh <name> <role> <brief-file>
#   role: an .ai/agents/<role>.md file; it picks the model (see ../protocols/subagents.md § Roster)
#   exit 0 = result printed, pane closed; non-zero = pane left open for inspection
set -euo pipefail
[ "${HERDR_ENV:-}" = 1 ] || { echo "not inside herdr" >&2; exit 2; }
name=$1 role=$2 brief=$3
[ -f ".ai/agents/$role.md" ] || { echo "no such role: $role" >&2; exit 2; }
case $role in
  reviewer|architect) model=opencode-go/deepseek-v4-pro ;;
  *)                  model=opencode-go/deepseek-v4.1-flash ;;
esac
dir=/tmp/opencode/herdr; out=$dir/$name.md; mkdir -p "$dir"; rm -f "$out"
[ "$brief" -ef "$dir/$name.brief.md" ] || cp "$brief" "$dir/$name.brief.md"

pane=$(herdr tab create --cwd "$PWD" --label "$name" --no-focus | jq -r .result.root_pane.pane_id)
herdr agent start "$name" --kind opencode --pane "$pane" -- --model "$model" >/dev/null
herdr agent prompt "$name" "You are the $role. Read .ai/agents/$role.md and follow it. Your brief is $dir/$name.brief.md. Write your result block, and nothing else, to $out, then reply DONE." \
  --wait --timeout "${TIMEOUT_MS:-1800000}" >/dev/null || true

[ -s "$out" ] || { echo "no result from $name (pane $pane left open; inspect: herdr agent read $name)" >&2; exit 3; }
herdr pane close "$pane" >/dev/null
cat "$out"
