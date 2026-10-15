"""Landing of a unit's rare-type pieces over its word pool (diagnostic, menu I_a).

For every usable word W of the pool: the chambers where W lands the unit's pieces of the given types, and whether
those chambers are holes at the state (then they need no vacating) or lie in Bad(W) occupied.

usage: landing.py <theory> <echain> <table> <target json {pieces}> <scored jsonl> <out json> --types t1,t2
       (types in the regions.orbit_of labelling, as buffer.Alphabet.type)
"""
import json
import sys
from collections import Counter
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402
import restore  # noqa: E402

table, tfile, sfile, out = sys.argv[3:7]
a = sys.argv[7:]
types = [int(x) for x in a[a.index('--types') + 1].split(',')]
C = GL.Ctx(table)
TYPE = np.asarray(restore.ctx.regions.orbit_of, np.int64)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
sel = np.isin(TYPE[T], types)
holes = ~C.occ
rows = [json.loads(x) for x in Path(sfile).read_text().splitlines() if x.strip()]
rows = [r for r in rows if r.get('usable')]
cnt, per = Counter(), []
for r in rows:
    W = [(int(c), int(i)) for c, i in r['W']]
    g, m, U = C.ghost(W, T)
    land = g.q[:len(T)][sel]
    key = tuple(sorted(int(x) for x in land))
    cnt[key] += 1
    per.append({'land': list(key), 'holes': int(holes[land].sum()), 'W': r['W'], 'n1': r['n1'], 'n2': r['n2'],
                'occ': r['occ'], 'bad': r['bad'], 'deficit': r['deficit']})
res = {'unit_pieces_of_types': T[sel].tolist(), 'their_types': TYPE[T[sel]].tolist(),
       'holes_of_types': np.flatnonzero(holes & np.isin(TYPE, types)).tolist(),
       'words': len(rows), 'distinct_landings': len(cnt), 'landing_counts': [[list(k), v] for k, v in cnt.most_common(20)],
       'words_landing_in_holes': sum(1 for p in per if p['holes'] == int(sel.sum())),
       'max_landing_holes': max((p['holes'] for p in per), default=0),
       'in_holes': sorted([p for p in per if p['holes'] == int(sel.sum())], key=lambda p: (p['n1'] + p['n2'], p['occ']))[:40]}
Path(out).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k: v for k, v in res.items() if k != 'in_holes'})[:3000])
print(json.dumps([{k: v for k, v in p.items() if k != 'W'} for p in res['in_holes'][:12]]))
