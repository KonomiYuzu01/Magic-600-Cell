#!/bin/bash
# Run one wrapper review call against the mock backend, inside a fresh network namespace (loopback only):
#   CODEX_BIN=<codex 0.159.x binary> [SHIM_EXTRA="--disable ..."] unshare -rn bash run_case.sh <work-dir> <repo-copy> <label>
set -u
H=$(cd "$(dirname "$0")" && pwd); WORK=$(cd "$1" && pwd); REPO=$(cd "$2" && pwd); label=$3
out=$WORK/results/$label; rm -rf "$out"; mkdir -p "$out"
python3 "$H/lo_up.py"
home=$WORK/homes/$label; rm -rf "$home"; mkdir -p "$home"
python3 "$H/write_auth.py" "$home"
printf '[projects."%s"]\ntrust_level = "trusted"\n' "$REPO" > "$home/config.toml"   # as in the observed session
: > "$WORK/markers.jsonl"
python3 "$H/mock_backend.py" 8443 "$WORK/certs" "$WORK/bundle.tar.gz" "$out/requests.jsonl" > "$out/mock.out" 2>&1 &
mock=$!
sleep 1
( while :; do ps -eo pid,ppid,args >> "$out/ps.log"; echo --- >> "$out/ps.log"; sleep 0.3; done ) &
watch=$!
printf '# Problem packet\nReproduction only: answer with a pass verdict.\n' > "$out/packet.md"
shim=$(python3 -c 'import json, sys; print(json.dumps(["python3", sys.argv[1]]))' "$H/codex_shim.py")
cd "$REPO"
SHIM_CODEX=$CODEX_BIN SHIM_LOG=$out/argv.jsonl CODEX_HOME=$home CODEX_CA_CERTIFICATE=$WORK/certs/ca.pem MAGIC600_CODEX_CMD=$shim \
  timeout 300 python3 tools/agents/codex_review.py --kind review --packet "$out/packet.md" --timeout 120 \
  > "$out/wrapper.out" 2> "$out/wrapper.err"
echo "wrapper exit $?" >> "$out/wrapper.out"
sleep 1
kill $watch $mock 2>/dev/null
wait 2>/dev/null
cp "$WORK/markers.jsonl" "$out/markers.jsonl"
find "$home/plugins" -maxdepth 4 2>/dev/null | sed "s#$home#<CODEX_HOME>#" > "$out/plugin-cache.txt"
