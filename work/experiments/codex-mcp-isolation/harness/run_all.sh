#!/bin/bash
# Run the standard cases: the wrapper at <base-rev> without and with each flag set, and the wrapper at
# <candidate-rev> as it is. Each wrapper runs from a throwaway copy of the repository (git archive, git init);
# `git stash create` names uncommitted changes without touching any ref.
#   CODEX_BIN=<codex 0.159.x binary> bash run_all.sh <work-dir> <base-rev> <candidate-rev> > summary.json
set -eu
H=$(cd "$(dirname "$0")" && pwd); ROOT=$(git -C "$H" rev-parse --show-toplevel)
bash "$H/prepare.sh" "$1" >&2; WORK=$(cd "$1" && pwd)
"$CODEX_BIN" --version 2>/dev/null > "$WORK/codex-version.txt"
for side in base:$2 candidate:$3; do
  dir=$WORK/repo-${side%%:*}; rm -rf "$dir"; mkdir -p "$dir"
  git -C "$ROOT" archive "${side#*:}" | tar -x -C "$dir"
  git -C "$dir" init -q
  git -C "$dir" -c user.name=harness -c user.email=harness@example.invalid add -A
  git -C "$dir" -c user.name=harness -c user.email=harness@example.invalid commit -qm "copy of ${side#*:}"
done
run() { SHIM_EXTRA="$3" unshare -rn bash "$H/run_case.sh" "$WORK" "$WORK/repo-$1" "$2"; }
run base A-base ""
run base B1-base-remote_plugin-off "--disable remote_plugin"
run base B2-base-plugins-off "--disable plugins"
run base B3-base-plugins-apps-off "--disable plugins --disable apps"
run candidate C-candidate ""
python3 "$H/summarize.py" "$WORK"
