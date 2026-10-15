#!/bin/bash
# usage: batch_dfam.sh <J> <E-chain scripts> <table> <target dir> <state prefix> <out dir> <parallel> [extra dfam args]
J=$1; SC=$2; TAB=$3; TGD=$4; PFX=$5; OUTD=$6; PAR=$7; shift 7
mkdir -p "$OUTD"
for t in "$TGD"/"$PFX"-b*.json; do
  b=$(basename "$t" .json)
  while [ "$(jobs -rp | wc -l)" -ge "$PAR" ]; do wait -n; done
  "$(dirname "$0")/run_dfam.sh" "$J" "$SC" "$TAB" "$t" "$OUTD/$b" "$@" &
done
wait
echo BATCH-DONE
