#!/bin/sh
# Round 22, line (c), experiment D (R22C2-02): at the depth 60 stop, land units 174, 175 and 176 one after another by
# steps found at the current state (a zero-Bad census word, else a closure with tclose2.py) and checked strict' by
# stepchk.py, then T1' for unit 180 at the state reached, first without a preparation letter, then after (481, 1)
# (exp_b.py). Units are numbered as in exp_b.py at the depth 60 stop. Exact; headless; research only.
# Usage (from the repository root): sh exp_d.sh <depth 60 stop as a pose table> <work dir>   (about 2 hours)
set -u
K60=$1; D=$2
G=research/jumbling/restoration/09-general-method/scripts
H=research/jumbling/restoration/12-low-height/scripts/round22
P="python3 -I -B"
mkdir -p $D
$P $H/exp_d_units.py $K60 $D/units 174 175 176 180 > $D/units.out 2>&1 || { echo "units failed"; exit 1; }
cur=$K60; n=0
for u in 174 175 176; do
  $P $H/transport_census.py $cur $D/zp-$u.json $D/units/u$u.json > $D/zp-$u.out 2>&1
  $P $H/exp_d_pick.py $D/zp-$u.json u$u $D/w-$u.json $D/core-$u.json >> $D/picks.out 2>&1
  if [ ! -s $D/w-$u.json ] && [ -s $D/core-$u.json ]; then
    python3 -I $G/gm/tclose2.py research/jumbling/theory $G/echain $cur $D/units/u$u.json $D/core-$u.json \
      $D/c-$u --greedy-seconds 300 > $D/c-$u.log 2>&1
    [ -s $D/c-$u/word-full.json ] && cp $D/c-$u/word-full.json $D/w-$u.json
  fi
  if [ ! -s $D/w-$u.json ]; then echo "$u: no step at the current state"; exit 0; fi
  n=$((n+1))
  $P $H/stepchk.py $cur $D/units/u$u.json $D/w-$u.json $D/s-$u.json $D/z$n.json > $D/s-$u.out 2>&1
  $P -c "import json,sys; r=json.load(open(sys.argv[1])); assert r['strict_prime'], r" $D/s-$u.json || { echo "$u: not strict"; exit 0; }
  echo "$u: strict step of $($P -c "import json,sys; print(json.load(open(sys.argv[1]))['letters'])" $D/s-$u.json) letters"
  cur=$D/z$n.json
done
$P $H/exp_d_units.py $cur --find $D/units/u180.json
$P $H/exp_b.py $cur $D/units/u180.json - - $D/e180-none > $D/e180-none.out 2>&1
cut -c1-600 $D/e180-none.out
if ! grep -q "strict-prime step" $D/e180-none.out; then
  $P $H/exp_b.py $cur $D/units/u180.json 481 1 $D/e180-481-1 > $D/e180-481-1.out 2>&1
  cut -c1-600 $D/e180-481-1.out
fi
