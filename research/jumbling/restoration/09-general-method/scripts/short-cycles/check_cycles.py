"""Independent check of the short pure elements in a short_cycles.py result. It reads only assets/primitives.npz and
assets/model.npz. For each record it composes the word C_word (signed primitive ids 1..1200, applied left to right)
and checks, on all 259,800 sticker slots:
- 'three-cycle': exactly three pieces move, all in the stated orbit; each goes to one of the others, so their positions
  are permuted cyclically; and C^3 = 1 (pure);
- 'twisted three-cycle': the same three-cycle of positions, with C^3 != 1;
- 'pair twist' and 'single twist': exactly two pieces, or one, of the stated orbit change, and each keeps its position.
It also checks that the recorded length and pieces match. Prints one line per record and a verdict; exit 0 when every
record passes. Run from the repository root: python3 -I -B check_cycles.py <result.json or .json.gz>
Research only: exact synthetic geometry; no Windows, DirectX, input or performance claim.
"""
import gzip
import json
import sys

import numpy as np

p, m = np.load('assets/primitives.npz'), np.load('assets/model.npz')
src, dst, off = p['src'], p['dst'], p['offsets']
piece = m['slot_piece'].astype(np.int64)
orbit = m['orbit_id'].astype(np.int64)
NS = len(piece)
ID = np.arange(NS, dtype=np.int64)
cache = {}


def perm(g):
    if g not in cache:
        a = abs(g)
        assert 1 <= a <= 1200, f'not a primitive: {g}'
        P = ID.copy()
        P[src[off[a - 1]:off[a]]] = dst[off[a - 1]:off[a]]
        if g < 0:
            Q = np.empty_like(P)
            Q[P] = ID
            P = Q
        cache[g] = P
    return cache[g]


def compose(word):
    P = ID
    for g in word:
        P = perm(g)[P]  # first P, then g
    return P


path = sys.argv[1]
recs = json.loads((gzip.open if path.endswith('.gz') else open)(path, 'rt').read())
ok_all = True
for key, r in sorted(recs.items(), key=lambda kv: (kv[1]['orbit'], kv[1]['kind'])):
    C = compose(r['C_word'])
    cache.clear()
    moved = np.flatnonzero(C != ID)
    pcs = np.unique(piece[moved])
    dest = {int(q): set(piece[C[moved[piece[moved] == q]]].tolist()) for q in pcs}
    ok = (len(pcs) > 0 and bool(np.all(orbit[pcs] == r['orbit'])) and len(r['C_word']) == r['C_len']
          and sorted(r['C_pieces']) == pcs.tolist() and key == f"{r['orbit']}:{r['kind']}")
    if r['kind'] in ('three-cycle', 'twisted three-cycle'):
        cyc = len(pcs) == 3 and all(len(v) == 1 and q not in v for q, v in dest.items())
        cube = bool(np.array_equal(C[C[C]], ID))
        ok = ok and cyc and cube == (r['kind'] == 'three-cycle')
    elif r['kind'] in ('pair twist', 'single twist'):
        ok = ok and len(pcs) == {'pair twist': 2, 'single twist': 1}[r['kind']] and all(
            v == {q} for q, v in dest.items())
    else:
        ok = False
    ok_all &= ok
    print(json.dumps({'orbit': r['orbit'], 'kind': r['kind'], 'length': len(r['C_word']),
                      'pieces': pcs.tolist(), 'ok': ok}))
print(json.dumps({'records': len(recs), 'all_ok': ok_all}))
sys.exit(0 if ok_all else 1)
