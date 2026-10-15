#!/bin/bash
# Parking search over a list of Q words (one parksearch.py run per Q, every <step>-th from <start>).
# usage: runpark.sh <theory dir> <pose table> <block size> <Q list json> <start> <step> <out prefix>
here=$(cd "$(dirname "$0")" && pwd)
ulimit -v 7000000
n=$(python3 -I -c "import json,sys;print(len(json.load(open(sys.argv[1]))))" "$4")
for ((k=$5; k<n; k+=$6)); do
  Q=$(python3 -I -c "import json,sys;print(json.dumps(json.load(open(sys.argv[1]))[int(sys.argv[2])]))" "$4" "$k")
  python3 -I "$here/parksearch.py" "$1" "$2" "$3" "$Q" "$7-$k.jsonl" 2> "$7-$k.log"
done
