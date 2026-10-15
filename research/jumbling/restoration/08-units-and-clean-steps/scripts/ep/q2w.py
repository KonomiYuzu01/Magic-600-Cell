"""Turn the D-type and C-type rows of dfam2.py (<prefix>.q.jsonl, optionally gzipped) into a word file for typeswap.py.

Every row whose word puts the whole unit on the lattice (T_dead = 0 and T_on = the unit size, read from the summary)
is written as {"W": <word>, "usable": true}, whatever it does to the other off pieces: the type condition of
Corollary D'' is about Bad(W) only, which depends on the word alone.

usage: q2w.py <prefix>.q.jsonl[.gz] <prefix>.summary.json <out jsonl>
"""
import gzip
import json
import sys

q, summ, out = sys.argv[1:4]
n = len(json.load(open(summ))['target'])
op = gzip.open if q.endswith('.gz') else open
k = 0
with open(out, 'w') as fh:
    for line in op(q, 'rt'):
        if not line.strip():
            continue
        r = json.loads(line)
        if r['T_dead'] == 0 and r['T_on'] == n:
            fh.write(json.dumps({'W': r['Q'], 'usable': True}) + '\n')
            k += 1
print(json.dumps({'rows_written': k, 'unit_size': n}))
