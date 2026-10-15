#!/bin/bash
# usage: run_dfam.sh <J> <E-chain scripts> <table> <target json> <out prefix> [extra dfam args]
J=$1; SC=$2; TAB=$3; TG=$4; OUT=$5; shift 5
[ -f "$OUT.summary.json" ] && exit 0
ulimit -v 7000000
timeout 5400 python3 -I "$(dirname "$0")/dfam.py" "$J/theory" "$SC" "$TAB" "$TG" "$OUT" "$@" > "$OUT.out" 2> "$OUT.log"
echo "$(basename "$OUT") rc $? $(tail -n 1 "$OUT.log" | cut -c1-200)"
