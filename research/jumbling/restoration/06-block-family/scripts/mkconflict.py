"""Block conflict digraph of one state from the template-search summaries of its blocks (Theorem CB).

For each block, the search summary lists the conflict sets of its scored words (the blocks whose pieces a word's ghost
kills or moves) with their counts; the empty set means a clean word. Per block this picks the smallest conflict set
(fewest blocks, then most words), draws block -> each member, checks that the digraph is acyclic and lists its levels
(level 0: blocks with a clean word; level k: blocks whose chosen set lies in levels below k).

usage: mkconflict.py <out json> <summary json> ...   (all summaries from the same state)
"""
import json
import sys
from pathlib import Path

out = Path(sys.argv[1])
rows, table, blocks = {}, None, None
for f in sys.argv[2:]:
    d = json.loads(Path(f).read_text())
    if table is None:
        table, blocks = d['table'], d['blocks']
    if d['table'] != table or d['blocks'] != blocks:
        raise SystemExit(f'{f}: different state')
    first = min(int(p) for p in d['target'])
    idx = next(i for i, size, p in blocks if p == first)
    sets = {}
    for key in ('q_conflict_sets', 'park_conflict_sets'):
        for s, n in d.get(key) or []:
            sets[tuple(s)] = sets.get(tuple(s), 0) + n
    best = min(sets.items(), key=lambda kv: (len(kv[0]), -kv[1]))
    rows[idx] = {'block': f'{blocks[idx][1]}@{first}', 'index': idx, 'q_scored': d.get('q_scored'),
                 'q_good': d.get('q_good'), 'park_good': d.get('park_good'),
                 'chosen_set': [f'{blocks[j][1]}@{blocks[j][2]}' for j in best[0]], 'chosen_words': best[1],
                 'sets': [[[f'{blocks[j][1]}@{blocks[j][2]}' for j in s], n]
                          for s, n in sorted(sets.items(), key=lambda kv: (len(kv[0]), -kv[1]))],
                 '_succ': list(best[0])}
missing = [f'{size}@{p}' for i, size, p in blocks if i not in rows]
level, left = {}, set(rows)
k = 0
while left:
    now = [i for i in left if all(j in level for j in rows[i]['_succ'])]
    if not now:
        break
    for i in now:
        level[i] = k
    left -= set(now)
    k += 1
res = {'state_table': Path(table).name, 'blocks': len(blocks), 'searched': len(rows), 'not_searched': missing,
       'acyclic': not left, 'levels': k,
       'order': [[rows[i]['block'] for i in sorted(level) if level[i] == lv] for lv in range(k)],
       'cyclic_part': sorted(rows[i]['block'] for i in left),
       'per_block': [{kk: v for kk, v in rows[i].items() if not kk.startswith('_')} for i in sorted(rows)]}
out.write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k_: res[k_] for k_ in ('blocks', 'searched', 'not_searched', 'acyclic', 'levels', 'order')}))
for r in res['per_block']:
    print(r['block'], r['q_scored'], r['q_good'], r['park_good'], '->', r['chosen_set'], r['chosen_words'])
