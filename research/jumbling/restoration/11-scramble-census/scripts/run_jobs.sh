#!/bin/bash
# Phase U census jobs in parallel, one process each. Run from the repository root:
#   run_jobs.sh <out dir> <job>...
# A job is "k:seed" (a fresh scramble of k twists) or "k:seed:<result json>" (replay that run's moves, then continue;
# keep the result json outside <out dir>, which receives a file of the same name). Each job gets a 75-minute descent
# budget, checked between steps, and a 110-minute hard limit; a job killed at the hard limit keeps the record of its
# last finished step ("result": "running") and exits with 124 in <out dir>/batch.done.
S=$(cd "$(dirname "$0")" && pwd)
O=$1; shift; mkdir -p "$O"
for j in "$@"; do
  IFS=: read -r k s st <<< "$j"
  extra=(); [ -n "$st" ] && extra=(--start "$st")
  ( ulimit -v 7000000; timeout 6600 python3 -I -B "$S/census_seams.py" "$k" "$s" 4500 "$O" "${extra[@]}" > "$O/ia-k$k-s$s.log" 2>&1
    echo "k=$k seed=$s exit=$?" >> "$O/batch.done" ) &
done
wait
for j in "$@"; do IFS=: read -r k s st <<< "$j"; tail -1 "$O/ia-k$k-s$s.log" | cut -c1-400; done
