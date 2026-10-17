#!/bin/bash
# Shape-catalogue test on one census residue with the tools of stages 05 and 06: exact cap sets and Mov blocks
# (stage 05 capsets.py and blocks.py), the K+ shape census (stage 06 census.py), then the transport run (stage 06
# fxrun.py) with the stage 06 templates. Run from the repository root:
#   run_fx.sh <name> <pose table> <out dir> <templates json>
J=research/jumbling; S5=$J/restoration/05-seed-table/scripts; S=$J/restoration/06-block-family/scripts
CL=$J/restoration/05-seed-table/data/classes.json
N=$1; T=$2; O=$3; TP=$4
mkdir -p "$O"; ulimit -v 7000000; set -e
python3 -I -B $S5/capsets.py $J $CL "$O/arrays.npz" "$T" "$O/$N-capsets.json" > "$O/$N-capsets.log" 2>&1
python3 -I -B $S5/blocks.py "$O/$N-capsets.json" "$O/$N-blocks.json" > "$O/$N-blocks.log" 2>&1
python3 -I -B $S/census.py $J $CL "$O/arrays.npz" "$O/$N-census.json" "$N=$T:$O/$N-blocks.json" > "$O/$N-census.log" 2>&1
timeout 7200 python3 -I -B $S/fxrun.py $J $S/echain $S $CL "$O/arrays.npz" "$T" "$O/$N-blocks.json" "$O/$N-census.json" \
  "$N" "$TP" "$O/$N-fx" > "$O/$N-fx.log" 2>&1
