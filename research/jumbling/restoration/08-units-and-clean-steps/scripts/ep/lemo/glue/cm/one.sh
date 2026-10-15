#!/bin/bash
# one carrier-move closure: jid A n B tgt free
ulimit -v 7000000
cd <scratch>/ep/lemo/glue/cm
read jid A n B tg fr <<< "$1"
TH=<research/jumbling>/theory; EC=<echain>; IN=<pkg>/certs/stop62/family-245/inputs
python3 -I ../apply.py $TH $EC <scratch>/ep/lemo/hole/tc/s62z-u18445/end-table.json p-$jid.json t-$jid.json a-$jid.json --types 7,9 > /dev/null 2> ap-$jid.log || { echo "$jid apply-failed" >> res.txt; exit 0; }
python3 -I <scratch>/gen/tclose2.py $TH $EC t-$jid.json $IN/s62g-k00-$tg.json w-$A-$n.json out-$jid --free $IN/s62g-k00-$fr.json --greedy-seconds 600 > c-$jid.log 2>&1
r=$(python3 -c "import json;print(json.load(open('out-$jid/result.json'))['result'] if 'result' in json.load(open('out-$jid/result.json')) else 'replayed')" 2>/dev/null)
echo "$jid $r" >> res.txt
[ "$r" = "replayed" ] || rm -f t-$jid.json
