#!/bin/bash
ulimit -v 7000000
S=<scratch>
J=$S/wt-e3/research/jumbling
L="--lattice $S/lat2/run/scripts <repo> $S/lat2/run/out/library-merged.json $S/lat2/run/checker"
for n in ia-k2-s9101 ia-k3-s9102 ia-k4-s9103 ia-k6-s9104; do
  python3 -I $S/gm/scripts/gdrive.py $J $S/v26 $S/gm/scr/$n-table.json $n $S/gm/runs/$n --par 1 $L > $S/gm/runs/$n.out 2> $S/gm/runs/$n.err
  echo "$n exit=$?" >> $S/gm/runs/scr.done
done
echo all >> $S/gm/runs/scr.done
