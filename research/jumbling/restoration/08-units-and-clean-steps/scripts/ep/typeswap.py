"""Per-type swap condition over a unit's word pool (certificate, menu I_a).

Corollary D'' (PROOFS-U Section 3.4): if U is the only unit with pieces of type t, a step V W V^-1 that restores U
alone, with V a word of K+ letters, needs |Bad(W)_t| <= |U_t|. For every usable word of the pool the script counts
Bad(W)_t for the given types and reports the words that meet |Bad(W)_t| <= |U_t| for all of them (the only words
that can restore U alone as the last carrier of these types). For the words of the runs the landing L lies in
Bad(W) (Remark L, measured), so the bound then says Bad(W)_t = L_t; the certificate does not use this.

usage: typeswap.py <theory> <echain> <table> <target json {pieces}> <scored jsonl> <out json> --types t1,t2
       (types in the regions.orbit_of labelling)
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
need = {t: int((TYPE[T] == t).sum()) for t in types}
off_t = {t: int((TYPE[C.off] == t).sum()) for t in types}
rows = [json.loads(x) for x in Path(sfile).read_text().splitlines() if x.strip()]
rows = [r for r in rows if r.get('usable')]
excess, ok = Counter(), []
for r in rows:
    W = [(int(c), int(i)) for c, i in r['W']]
    g, m, U = C.ghost(W, T)
    bad = U[~g.alive[m:] | (g.dom[m:] != 0)]
    cnt = {t: int((TYPE[bad] == t).sum()) for t in types}
    e = sum(max(0, cnt[t] - need[t]) for t in types)
    excess[e] += 1
    if e == 0:
        ok.append({'W': r['W'], 'bad': int(len(bad)), 'by_type': cnt})
res = {'unit': int(T.min()), 'unit_size': int(len(T)), 'unit_by_type': need, 'off_by_type_at_state': off_t,
       'words': len(rows), 'excess_histogram': sorted(excess.items()), 'swap_words': len(ok), 'examples': ok[:10]}
Path(out).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k: v for k, v in res.items() if k != 'examples'}))
