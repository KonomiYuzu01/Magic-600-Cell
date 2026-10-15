#!/bin/bash
# Pipeline and exact cap sets on the deep samples (tables in data/deep/). J = research/jumbling at commit 27a001c.
# Run from scripts/. The cap-set step for fixture40 was stopped (too slow) and is not part of the results.
: "${J:?set J}"
for n in e2run24 stop62 fixture40; do
  python3 -I e3pipe.py "$J" ../data/classes.json ../data/arrays.npz ../data/seeds-lib.json ../data/deep/$n-table.json ../runs/run-$n.json --exec --name $n --kseeds ../data/kseeds.json > ../runs/run-$n.log 2>&1
  [ $n = fixture40 ] || python3 -I capsets.py "$J" ../data/classes.json ../data/arrays.npz ../data/deep/$n-table.json ../certs/capsets-$n.json > ../runs/capsets-$n.log 2>&1
done
