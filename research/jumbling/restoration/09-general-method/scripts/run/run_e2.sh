#!/bin/bash
ulimit -v 7000000
S=<scratch>
J=$S/wt-e3/research/jumbling
L="--lattice $S/lat2/run/scripts <repo> $S/lat2/run/out/library-merged.json $S/lat2/run/checker"
python3 -I $S/gm/scripts/gdrive.py $J $S/v26 <scratch>/final/pkgbf/Claude-3E-block-family-20261014/data/states/e2run24-table.json e2run24 $S/gm/runs/e2run24 --par 2 $L > $S/gm/runs/e2run24.out 2> $S/gm/runs/e2run24.err
echo exit=$? > $S/gm/runs/e2run24.done
