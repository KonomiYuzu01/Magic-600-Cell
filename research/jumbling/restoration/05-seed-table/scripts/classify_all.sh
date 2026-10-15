#!/bin/bash
# Classification of the frozen inputs (no seeds): off pieces, depth one, position classes.
# J = research/jumbling at commit 27a001c; IN = directory holding the six frozen input tables (see PROOFS-3E.md, Section 7).
# Run from scripts/.
set -e
: "${J:?set J}" "${IN:?set IN}"
for pair in "E3 $IN/E3-table.json" "E2 $IN/E2-table.json" "E1 $IN/E1-table.json" "X $IN/X-table.json" "I1 $IN/I1-table.json" "I2 $IN/I2-table.json"; do
  set -- $pair
  python3 -I e3pipe.py "$J" ../data/classes.json ../data/arrays.npz ../data/empty-seeds.json "$2" ../runs/cls-$1.json --name $1 2>&1 | tail -2
done
