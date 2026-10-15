#!/bin/bash
# Endings of the E2 table by the three-block family: generate an input (gen3.py) and construct its
# certificate-first restoring word (constr3.py).
# usage: P30=<three-block package> TH=<research/jumbling/theory> run_endings.sh <out dir> < specs
# each line of the spec file is "<tag> safe <seed> <n>" or "<tag> setup <seed> <n> <seed2> <n2>".
set -u
OUT=$1
mkdir -p "$OUT"
ulimit -v 7000000
while read -r tag rest; do
  [ -z "$tag" ] && continue
  python3 -I "$P30/scripts/gen3.py" "$TH" "$P30/helpers" "$P30/inputs/E2-table.json" "$P30/inputs/sigma-star.json" "$OUT/$tag" $rest > "$OUT/$tag-gen.log" 2>&1 || { echo "$tag gen failed"; continue; }
  python3 -I "$P30/scripts/constr3.py" "$TH" "$P30/helpers" "$OUT/$tag-table.json" "$P30/inputs/sigma-star.json" "$OUT/$tag-construct" > "$OUT/$tag-construct.log" 2>&1
  echo "$tag constr exit=$?"
done
