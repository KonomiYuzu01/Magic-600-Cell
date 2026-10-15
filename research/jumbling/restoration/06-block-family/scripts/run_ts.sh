#!/bin/bash
# Type-S search (tshrink.py) on every target file of a directory.
# usage: run_ts.sh <research/jumbling> <echain dir> <pose table> <target dir> <state prefix> <out dir> <levels> <timeout s>
J=$1; SC=$2; TAB=$3; TGD=$4; PFX=$5; OUTD=$6; LV=$7; TO=$8
mkdir -p "$OUTD"
for t in "$TGD"/"$PFX"-b*.json; do
  b=$(basename "$t" .json)
  [ -f "$OUTD/$b.jsonl.summary.json" ] && continue
  echo "== $b $(date +%T)"
  timeout "$TO" python3 -I "$(dirname "$0")/tshrink.py" "$J/theory" "$SC" "$TAB" "$t" "$LV" "$OUTD/$b.jsonl" --max-R 4 > "$OUTD/$b.out" 2> "$OUTD/$b.log"
  echo "rc $? $(tail -c 300 "$OUTD/$b.out")"
done
echo DONE
