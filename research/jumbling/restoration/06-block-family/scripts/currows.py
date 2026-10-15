"""Curated template rows: the words of dfam.py output (q.jsonl, park.jsonl) that put the whole target on K with no
piece of it killed, ordered by (number of other blocks disturbed, other off pieces killed or moved, occupied chambers of Bad,
size of Bad).

usage: currows.py <out jsonl> <N> <dfam prefix> ...
"""
import json
import sys
from pathlib import Path

out, N, prefixes = sys.argv[1], int(sys.argv[2]), sys.argv[3:]
rows = []
for pre in prefixes:
    T = len(json.loads(Path(pre + '.summary.json').read_text())['target'])
    for name in ('.q.jsonl', '.park.jsonl'):
        f = Path(pre + name)
        if not f.exists():
            continue
        for line in f.open():
            r = json.loads(line)
            if r.get('T_on') != T or r.get('T_dead', 0):
                continue
            bad = r.get('badW', r.get('badQ'))
            occ = r.get('badW_occupied', r.get('badQ_occupied'))
            rows.append((len(r.get('conflict_blocks', [])), r.get('prot_dead', 0) + r.get('prot_moved', 0), occ, bad,
                         {'P': r.get('P', []), 'Q': r['Q'], 'conflict_blocks': r.get('conflict_blocks', []),
                         'T_on': r['T_on'], 'T_dead': 0, 'source_disturbed': r.get('prot_dead', 0) + r.get('prot_moved', 0),
                         'source_badW': bad, 'source_badW_occupied': occ}))
rows.sort(key=lambda x: x[:4])
seen, kept = set(), []
for *_, r in rows:
    k = json.dumps([r['P'], r['Q']])
    if k not in seen:
        seen.add(k)
        kept.append(r)
    if len(kept) >= N:
        break
Path(out).write_text(''.join(json.dumps(r) + '\n' for r in kept))
print(json.dumps({'out': out, 'kept': len(kept), 'candidates': len(rows)}))
