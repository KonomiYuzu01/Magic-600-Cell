#!/bin/bash
# usage: run_u3.sh <J> <classes> <arrays> <capsets.py> <log> word...   (export -> cap sets -> units5c, orbit 5)
J=$1; C=$2; A=$3; CAPS=$4; LOG=$5; shift 5
mkdir -p s2
for w in "$@"; do
  t=$(echo $w | tr ':,' '-_')
  python3 -I export_word.py $J $w s2/t-$t.json > s2/e-$t.log 2>&1 || python3 -I export_word.py $J $w s2/t-$t.json > s2/e-$t.log 2>&1
  python3 -I $CAPS $J $C $A s2/t-$t.json s2/c-$t.json > s2/c-$t.log 2>&1
  python3 -I units5c.py $J $C $A s2/t-$t.json s2/c-$t.json 5 s2/uc5-$t.json > s2/uc5-$t.log 2>&1
  echo "$w $(head -1 s2/uc5-$t.log)" >> $LOG
  rm -f s2/c-$t.json
done
