#!/bin/bash
# usage: run_u5b.sh <J> <classes> <arrays> <orbit> tag...   (tables t-<tag>.json, cap sets c-<tag>.json)
J=$1; C=$2; A=$3; O=$4; shift 4
for t in "$@"; do
  python3 -I units5b.py $J $C $A t-$t.json c-$t.json $O ub$O-$t.json > ub$O-$t.log 2>&1
  echo "$t orbit $O $(head -1 ub$O-$t.log)"
done
