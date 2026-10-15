#!/bin/bash
# two-jumble bases: export, exact cap sets, rigid-unit check for orbits 5 and 4
J=$1; CL=$2; AR=$3; CAPS=$4; shift 4
for w in "$@"; do
  n=$(echo $w | tr ':,' '-_')
  python3 -I export_word.py $J $w t-$n.json > /dev/null || { echo "$w inadmissible"; continue; }
  python3 -I $CAPS $J $CL $AR t-$n.json c-$n.json > c-$n.log 2>&1
  for o in 5 4; do
    echo "$w orbit $o $(python3 -I units5.py $J $CL $AR t-$n.json c-$n.json $o u$o-$n.json 2>&1 | head -1)"
  done
done
