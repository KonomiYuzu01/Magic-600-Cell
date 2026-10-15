"""Check the refinement identity on instances (menu I_a).

For a word Q and a K+ letter k at grip f, with B = Bad(Q), B' = Bad(Q^-1), D = the chambers whose lattice ghost dies
under Q, S = the off-lattice contents that Q makes of lattice contents, and k~ = k on cap f and the identity outside,
the identity says, for every type t,

    |Bad([Q, k])_t| = |D_t| + |k~(B') \\ B'|_t + |S_t \\ k(S)|.

The script computes both sides on every chamber of the given types and reports mismatches.

usage: refid.py <theory> <echain> <table> <target json> <q.jsonl[.gz]> <out json> --types 7,9 [--max-q N]
"""
import gzip
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
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

table, tfile, qf, out = sys.argv[3:7]
a = sys.argv[7:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
MAXQ = int(a[a.index('--max-q') + 1]) if '--max-q' in a else 20
C = GL.Ctx(table)
M, data = C.M, GL.data
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
R = np.flatnonzero(np.isin(TYPE, TYPES))
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)


def cnt(ch):
    ch = np.asarray(ch, np.int64)
    return tuple(int((TYPE[ch] == t).sum()) for t in TYPES)


op = gzip.open if qf.endswith('.gz') else open
rows = [json.loads(x) for x in op(qf, 'rt') if x.strip()]
tmax = max(r['T_on'] for r in rows)
rows = [r for r in rows if r['T_dead'] == 0 and r['T_on'] == tmax][:MAXQ]
res = Counter()
examples = []
for r in rows:
    Q = [(int(c), int(i)) for c, i in r['Q']]
    Qi = CP.inverse_word(M, Q)
    g, m, _ = C.ghost(Q, T, R)
    L = g.q[:m].copy()
    al, dm, qq = g.alive[m:], g.dom[m:], g.q[m:]
    D = R[~al]
    offm = al & (dm != 0)
    Soff = set(zip(dm[offm].tolist(), qq[offm].tolist()))
    sd = np.array([d for d, _ in Soff], np.int64)
    sq = np.array([q for _, q in Soff], np.int64)
    gi, _, _ = C.ghost(Qi, (), R)
    Bp = R[~gi.alive | (gi.dom != 0)]
    Bps = set(Bp.tolist())
    caps = np.flatnonzero(data.sig_matrix(L).sum(axis=0) == len(L)).tolist()
    for f in caps[:3]:
        capf = GL.cap(f)
        for i in range(len(M.items)):
            k = kp.index_of_matrix(GL.restore.element(M, f, i))
            if k < 0 or k == KID:
                continue
            j = int(CP.inverse_word(M, [(f, i)])[0][1])
            kB = [int(FG.chamber_image(k, np.array([w]))[0]) if capf[w] else int(w) for w in Bp.tolist()]
            out1 = [w for w in kB if w not in Bps]
            gk = FG.Ghost(C.F, sd, sq)
            gk.apply(f, C.E(f, i), (f, i))
            kS = set(zip(gk.dom[gk.alive].tolist(), gk.q[gk.alive].tolist()))
            rest = [q for (d, q) in Soff if (d, q) not in kS]
            pred = tuple(x + y + z for x, y, z in zip(cnt(D), cnt(out1), cnt(rest)))
            W = Q + [(f, i)] + Qi + [(f, j)]
            g2, m2, _ = C.ghost(W, T, R)
            act = cnt(R[~g2.alive[m2:] | (g2.dom[m2:] != 0)])
            land = bool(g2.alive[:m2].all() and (g2.dom[:m2] == 0).all())
            res[(pred == act, land)] += 1
            if len(examples) < 12 and land:
                examples.append({'Q': r['Q'], 'f': f, 'i': i, 'D': cnt(D), 'outflow': cnt(out1),
                                 'S_minus_kS': cnt(rest), 'pred': pred, 'actual': act})
doc = {'checked': sum(res.values()), 'by_match_and_landing': [[list(k_), v] for k_, v in res.items()],
       'examples': examples}
Path(out).write_text(json.dumps(doc, indent=1) + '\n')
print(json.dumps({'checked': doc['checked'], 'by_match_and_landing': doc['by_match_and_landing']}))
