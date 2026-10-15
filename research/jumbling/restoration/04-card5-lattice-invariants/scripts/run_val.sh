#!/bin/bash
# POT5-VAL: end-to-end validation of the orbit-5 potential (K+ orbit 7) on four random walks and on deep tables.
# usage: run_val.sh <research/jumbling> <package scripts dir> <table dir with t-T200/T300/T723/T723x433-12.json> <out dir>
J=$1; SC=$2; D=$3; O=$4; mkdir -p "$O"
ulimit -v 7000000
for spec in "0 600 11" "5 600 12" "777 600 13" "1234 600 14"; do
  set -- $spec
  timeout 7200 python3 -I "$SC/potval.py" "$J" 7 $1 $2 $3 > "$O/walk-a$1-s$3.log" 2>&1
  echo "walk a=$1 seed=$3 rc=$? $(grep -v SOLUTION "$O/walk-a$1-s$3.log" | tail -1)"
done
timeout 7200 python3 -I "$SC/potval.py" "$J" 7 0 0 1 "$D/t-T200.json" "$D/t-T300.json" "$D/t-T723.json" "$D/t-T723x433-12.json" > "$O/tables.log" 2>&1
echo "tables rc=$? $(tail -2 "$O/tables.log" | tr '\n' ' ')"
echo DONE
