#!/bin/bash
ulimit -v 7000000
S=<scratch>
J=$S/wt-e3/research/jumbling
L="--lattice $S/lat2/run/scripts <repo> $S/lat2/run/out/library-merged.json $S/lat2/run/checker"
python3 -I $S/gm/scripts/gdrive.py $J $S/v26 <scratch>/final/pkgbf/Claude-3E-block-family-20261014/data/states/stop62-table.json stop62 $S/gm/runs/stop62 --par 2 \
  --seed $S/cm/runs/stop62 --seed $S/gen/runs/s62b --seed $S/gen/runs/s62c --seed $S/gen/runs/s62d \
  --seed $S/gen/runs/s62e --seed $S/gen/runs/s62g --seed $S/gen/runs/s62h $L > $S/gm/runs/stop62.out 2> $S/gm/runs/stop62.err
echo exit=$? > $S/gm/runs/stop62.done
