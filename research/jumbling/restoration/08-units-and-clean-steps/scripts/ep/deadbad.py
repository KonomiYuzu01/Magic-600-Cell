"""Dead chambers in Bad(W) over a unit's word pool (diagnostic, menu I_a).

A chamber of Bad(W) is dead for the rest of a run when its type has no off piece at the state: by Fact 1' the type
then has no hole, off_t only falls along a run (every step is clean), and Lemma D (|Bad(W)_t| <= off_t) excludes W
at this state and every later one. For every usable word of the pool the script lists the dead chambers of Bad(W)
and the types involved, and reports the chambers common to all words and the words with no dead chamber.

usage: deadbad.py <theory> <echain> <table> <target json {pieces}> <scored jsonl (score4 rows)> <out json>
       [--free <json {pieces}>]
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

table, tfile, sfile, out = sys.argv[3:7]
a = sys.argv[7:]
C = GL.Ctx(table)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
FREE = np.asarray(sorted(json.loads(Path(a[a.index('--free') + 1]).read_text())['pieces']), np.int64) \
    if '--free' in a else np.zeros(0, np.int64)
OTH = np.setdiff1d(np.setdiff1d(C.off, T), FREE)
TYPE = np.asarray(GL.data.orbit_id, np.int64) + 1
NT = int(TYPE.max()) + 1
home_type = TYPE  # piece p's home chamber is chamber p
off_t = np.bincount(home_type[C.off], minlength=NT)
dead_type = off_t == 0
rows = [json.loads(x) for x in Path(sfile).read_text().splitlines() if x.strip()]
rows = [r for r in rows if r.get('usable')]
common, per, clean = None, [], []
cnt = Counter()
for r in rows:
    W = [(int(c), int(i)) for c, i in r['W']]
    g, m, U = C.ghost(W, np.concatenate([T, FREE, OTH]))
    bad = U[~g.alive[m:] | (g.dom[m:] != 0)]
    dead = bad[dead_type[TYPE[bad]]]
    s = set(dead.tolist())
    common = s if common is None else common & s
    cnt.update(s)
    per.append({'W': r['W'], 'bad': int(len(bad)), 'dead': len(s), 'dead_types': sorted(set(TYPE[dead].tolist())),
                'dead_occupied': int(C.occ[dead].sum())})
    if not s:
        clean.append(r['W'])
res = {'off': int(len(C.off)), 'off_by_type': {int(t): int(v) for t, v in enumerate(off_t) if v},
       'dead_types': np.flatnonzero(dead_type).tolist(), 'words': len(rows), 'words_without_dead': len(clean),
       'dead_common_to_all': sorted(common) if common else [],
       'dead_min': min((p['dead'] for p in per), default=None),
       'most_frequent_dead': cnt.most_common(12),
       'dead_count_histogram': sorted(Counter(p['dead'] for p in per).items())[:20],
       'clean_words': clean[:50]}
Path(out).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k: v for k, v in res.items() if k != 'clean_words'}))
