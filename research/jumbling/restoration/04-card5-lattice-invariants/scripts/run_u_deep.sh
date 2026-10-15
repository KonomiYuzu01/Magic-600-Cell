#!/bin/bash
# UNITS (orbit 5) on exported deep fixture tables.
# usage: run_u_deep.sh <research/jumbling> <package root> <table dir> <name>...   (names such as T200 T300 T723 T723x433-12)
# The tables t-<name>.json come from: python -I scripts/export_fixture.py <research/jumbling> 200,300,723 <table dir> 433:12
J=$1; PKG=$2; D=$3; shift 3
for n in "$@"; do
  python3 -I "$PKG/scripts/capsets.py" "$J" "$PKG/data/classes.json" "$PKG/data/arrays.npz" "$D/t-$n.json" "$D/c-$n.json" > "$D/c-$n.log" 2>&1
  python3 -I "$PKG/scripts/units5c.py" "$J" "$PKG/data/classes.json" "$PKG/data/arrays.npz" "$D/t-$n.json" "$D/c-$n.json" 5 "$D/uc5-$n.json" > "$D/uc5-$n.log" 2>&1
  echo "$n $(head -1 "$D/uc5-$n.log")"
done
