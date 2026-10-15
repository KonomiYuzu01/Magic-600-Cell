"""Write the best protected-exact Q of a dfam run (smallest Bad(Q)) as a {P: [], Q} word file, or the best parked W.
usage: bestq.py <dfam prefix> <out json> [--parked]"""
import json
import sys
pre, out = sys.argv[1], sys.argv[2]
d = json.load(open(pre + '.summary.json'))
if '--parked' in sys.argv and d.get('park_best'):
    r = d['park_best'][0]
    json.dump({'P': r['P'], 'Q': r['Q'], 'badW': r['badW'], 'source': pre + ' parked'}, open(out, 'w'))
    print('parked', r['badW'])
else:
    best = d['q_best'][0]
    for line in open(pre + '.q.jsonl'):
        r = json.loads(line)
        if all(r[k] == best[k] for k in ('e', 'B', 'c', 'X', 'a', 'A')):
            json.dump({'P': [], 'Q': r['Q'], 'badQ': r['badQ'], 'source': pre + ' best Q'}, open(out, 'w'))
            print('Q', r['badQ'])
            break
