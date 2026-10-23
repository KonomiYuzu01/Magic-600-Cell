#!/bin/sh
# Round 22, line (c), adjudicating experiment A (X22C-06): an order at the depth 40 stop. b12's T1' step, then b14's
# census word, then b15 by T1' at the state reached. Each step is replayed from an exported pose table and checked
# against the strict' contract (stepchk.py); b15's landing word comes from cellpure5.py and the closure of its
# inverse from tclose2.py at Z1 = W(Z). Exact; headless; research only.
# Usage (from the repository root): sh exp_a.sh <depth 40 table> <work dir>   (about 15 minutes)
set -e
K=$1
X=$2
S=research/jumbling/restoration/12-low-height
H=$S/scripts/round22
D=$S/runs/round22/line-c
G=research/jumbling/restoration/09-general-method/scripts
mkdir -p "$X/c-b15"
python3 -I -B $H/stepchk.py "$K" $D/units/eb5-k40-k00-b12.json $S/runs/round22/t1prime-k40/steps/u12.json \
    "$X/a1-b12.json" "$X/za-table.json"
python3 -I -B $H/stepchk.py "$X/za-table.json" $D/units/eb5-k40-k00-b14.json $D/w14-census.json \
    "$X/a2-b14.json" "$X/zb-table.json"
python3 -I -B $S/scripts/cellpure5.py research/jumbling/theory "$X/zb-table.json" $D/units/eb5-k40-k00-b15.json \
    "$X/t-b15.json" --search > "$X/t-b15.log" 2>&1
python3 -I -B $H/prep_t1prime.py "$X/zb-table.json" "$X/t-b15-word.json" b15 "$X" > "$X/prep-b15.out"
python3 -I $G/gm/tclose2.py research/jumbling/theory $G/echain "$X/b15-z1-table.json" "$X/b15-Q.json" \
    "$X/b15-winv.json" "$X/c-b15" --greedy-seconds 300 > "$X/c-b15.log" 2>&1
if [ ! -s "$X/c-b15/word-full.json" ]; then
    echo "b15: no closure of W^-1 at Z1 ($X/c-b15/result.json)"
    exit 0
fi
python3 -I -B - "$X" <<'PY'
import json, sys
from pathlib import Path
x = Path(sys.argv[1])
w = json.loads((x / 't-b15-word.json').read_text())
f = json.loads((x / 'c-b15/word-full.json').read_text())
(x / 'w15-t1p.json').write_text(json.dumps({'word': w['word'] + f['word'], 'input_digest': w['input_digest'],
                                            'output_digest': f['output_digest']}))
PY
python3 -I -B $H/stepchk.py "$X/zb-table.json" $D/units/eb5-k40-k00-b15.json "$X/w15-t1p.json" "$X/a3-b15.json"
