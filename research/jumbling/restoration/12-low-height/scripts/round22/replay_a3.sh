#!/bin/bash
# Round 22, line (a), round 3: the order of R22A2-04 replayed at the depth 40 stop, each step built at the current
# state: the unit's census word alone when it is strict' there, otherwise a fresh closure V.W.V^-1 (tclose2.py, its V
# from the peeling certificate or a 300-second greedy search); every built step is replayed by stepchk.py from the
# exported table before the next one starts. A unit with neither is skipped and the replay continues from the last
# built state. The middles W are R22A2-04's: the census words of runs/round22/line-a/census-class*.json and the cores of
# b08, b09 and b10 in runs/round22/line-a/closures/. Exact; headless; research only.
# Usage (from the repository root): bash replay_a3.sh <depth 40 stop as a pose table> <new work dir>   (about 2 hours)
set -u
Z0=$1; D=$2
S=research/jumbling/restoration/12-low-height/scripts
A=research/jumbling/restoration/12-low-height/runs/round22/line-a
G=research/jumbling/restoration/09-general-method/scripts
P="python3 -I -B"
fail() { echo "$1"; exit 1; }
if [ -e "$D" ] && [ -n "$(ls -A "$D")" ]; then fail "work directory $D is not empty"; fi
mkdir -p "$D/cores" || fail "cannot create $D"
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1
$P - "$A" "$D/cores" <<'PY' || fail "cores failed"
import json, sys
from pathlib import Path
A, out = Path(sys.argv[1]), Path(sys.argv[2])
rows = {}
for f in ('census-class3.json', 'census-class24.json'):
    rows.update({k[-3:]: v for k, v in json.loads((A / f).read_text())['units'].items()})
for n in ('b00', 'b01', 'b02', 'b07', 'b08', 'b11', 'b13', 'b19', 'b09', 'b10', 'b14', 'b16', 'b21', 'b20'):
    if n in ('b08', 'b09', 'b10', 'b19'):
        core = json.loads((A / 'closures' / f'{n}-core.json').read_text())
    else:
        w = rows[n]['zero_bad']['word']
        core = {'P': [], 'Q': w}
        (out / f'{n}-word.json').write_text(json.dumps({'word': w}) + '\n')
    (out / f'{n}-core.json').write_text(json.dumps(core) + '\n')
PY
strict() { $P -c "import json,sys; r=json.load(open(sys.argv[1])); sys.exit(0 if r.get('strict_prime') else 1)" "$1"; }
summary_none() { $P - "$@" <<'PY'
import json, sys
print(json.dumps({'unit': sys.argv[1], 'how': 'none', 'state': json.load(open(sys.argv[2]))['digest']}))
PY
}
summary_step() { $P - "$@" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
keys = ('letters', 'off_before', 'off_after', 'strict_prime', 'end_digest')
print(json.dumps(dict(unit=sys.argv[2], how=sys.argv[3], **{k: r[k] for k in keys})))
PY
}
cur=$Z0; n=0
for u in b00 b01 b02 b07 b08 b11 b13 b19 b09 b10 b14 b16 b21 b20; do
  how=none
  if [ -s $D/cores/$u-word.json ]; then
    # A census word computed at the stop may be inadmissible at a later state; that only means no plain step.
    if $P $S/round22/stepchk.py $cur $A/units/eb5-k40-k00-$u.json $D/cores/$u-word.json $D/$u-plain.json \
      $D/z-$u-plain.json > $D/$u-plain.log 2>&1 && strict $D/$u-plain.json; then how=plain; fi
  fi
  if [ $how = none ]; then
    python3 -I $G/gm/tclose2.py research/jumbling/theory $G/echain $cur $A/units/eb5-k40-k00-$u.json \
      $D/cores/$u-core.json $D/c-$u --greedy-seconds 300 > $D/c-$u.log 2>&1 || fail "$u: closure failed"
    if [ -s $D/c-$u/word-full.json ]; then
      $P $S/round22/stepchk.py $cur $A/units/eb5-k40-k00-$u.json $D/c-$u/word-full.json $D/$u-close.json \
        $D/z-$u-close.json > $D/$u-close.log 2>&1 || fail "$u: step check failed"
      if strict $D/$u-close.json; then how=close; else fail "$u: closure word not strict'"; fi
    fi
  fi
  if [ $how = none ]; then
    summary_none $u $cur >> $D/seq.out
  else
    n=$((n+1)); cur=$D/z-$u-$how.json
    summary_step $D/$u-$how.json $u $how >> $D/seq.out
  fi
done
echo "{\"done\": true, \"landed\": $n, \"last\": \"$cur\"}" >> $D/seq.out
