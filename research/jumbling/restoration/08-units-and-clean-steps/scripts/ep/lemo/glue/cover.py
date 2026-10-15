"""Can moving the other carriers bring their glued holes onto the rare Bad set of a word (PROOFS-U Proposition GH)?

By the proof of Proposition GH, along any word V of admissible K+ letters every hole glued to a carrier B ends at
g_B(h), where g_B in K+ is B's net rotation under V. For a step V W V^-1 that restores carrier A with V(A) = A, every
chamber of Bad(W) of the rare types must be a hole of V(Z) (Lemma V), and the rare holes of V(Z) are A's own glued
holes and g_B(holes of B) for the other carriers B. So Bad(W)_rare minus A's holes must be covered by
g_B(H_B) for the other carriers, for some g_B in K+. This script tries every element of K+ (7,200; a superset of the
rotations any V can give B) for each other carrier, and reports, per word, the largest part of the needed set that
any choice covers and the choices that cover all of it.

usage: cover.py <theory> <echain> <table> <glued json> <out json> --types 7,9 --rows <A>:<rows.jsonl> ...
"""
import json
import sys
import time
from itertools import product
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()
table, gfile, out = sys.argv[3:6]
a = sys.argv[6:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
specs = a[a.index('--rows') + 1:]
C = GL.Ctx(table)
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
RARE = np.isin(TYPE, TYPES)
NK = len(FG.kp.perms)
g = json.loads(Path(gfile).read_text())
assert g['all_glued']
H = {}
for r in g['rows']:
    assert len(r['glued_to']) == 1
    H.setdefault(int(r['glued_to'][0]), []).append(int(r['hole']))
H = {u: np.array(sorted(v), np.int64) for u, v in H.items()}
holes_all = set(int(x) for v in H.values() for x in v)
assert holes_all == set(np.flatnonzero(~C.occ & RARE).tolist()), 'every rare hole must be glued'

# images of each carrier's glued holes under every element of K+
IMG = {u: [set(FG.chamber_image(k, h).tolist()) for k in range(NK)] for u, h in H.items()}

res = []
for spec in specs:
    A, rf = spec.split(':', 1)
    A = int(A)
    others = sorted(u for u in H if u != A)
    for n, line in enumerate(open(rf)):
        if not line.strip():
            continue
        row = json.loads(line)
        W = [(int(c), int(i)) for c, i in row['W']]
        grips = sorted({c for c, _ in W})
        ch = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]) & RARE)
        bad, _, _ = C.bad(W, ch)
        bad = set(bad.tolist())
        need = bad - set(H[A].tolist())
        need_occ = {x for x in need if C.occ[x]}
        # achievable covered subsets per other carrier
        subs = []
        for u in others:
            d = {}
            for k in range(NK):
                s = frozenset(need & IMG[u][k])
                if s not in d:
                    d[s] = k
            subs.append(d)
        best, full = 0, []
        for combo in product(*[list(d.items()) for d in subs]):
            cov = frozenset().union(*[s for s, _ in combo])
            best = max(best, len(cov))
            if cov == need and len(full) < 20:
                full.append({str(u): k for u, (_, k) in zip(others, combo)})
        rec = {'A': A, 'row': n, 'src': rf.split('/')[-1], 'bad_rare': len(bad),
               'bad_rare_by_type': {str(t): int(sum(TYPE[x] == t for x in bad)) for t in TYPES},
               'own_holes_in_bad': len(bad & set(H[A].tolist())), 'need': sorted(need),
               'need_by_type': {str(t): int(sum(TYPE[x] == t for x in need)) for t in TYPES},
               'need_occupied': len(need_occ), 'best_cover': best, 'covers': full,
               'subsets_per_other': {str(u): len(d) for u, d in zip(others, subs)}}
        res.append(rec)
        print(json.dumps({k: v for k, v in rec.items() if k != 'covers'} | {'n_covers': len(full),
                                                                            'sec': round(time.time() - T0)}),
              flush=True)
doc = {'table': Path(table).name, 'types': TYPES, 'glued': {str(u): v.tolist() for u, v in H.items()},
       'words': len(res), 'words_with_full_cover': sum(1 for r in res if r['covers']),
       'best_cover_histogram': sorted([[b, sum(1 for r in res if r['best_cover'] == b)]
                                       for b in {r['best_cover'] for r in res}]),
       'rows': res, 'seconds': round(time.time() - T0)}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({k: v for k, v in doc.items() if k != 'rows'}))
