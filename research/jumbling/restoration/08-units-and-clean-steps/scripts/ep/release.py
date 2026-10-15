"""Where the missing holes are (diagnostic, menu I_a): for the best words of a unit, the SAFE-ALL orbits that fail N2
or N1, the holes of the same types and the orbits they sit in, and for a hole that no SAFE-ALL letter moves, the grips
whose caps hold it and the blocks that make each of those grips unsafe (an off piece inside the cap, or a region
straddling it).

usage: release.py <theory> <echain> <table> <blocks json (in the mkblocks2 output dir)> <scored jsonl> <out json> [--top N]
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402

table, bfile, sfile, out = sys.argv[3:7]
a = sys.argv[7:]
TOP = int(a[a.index('--top') + 1]) if '--top' in a else 3
C = GL.Ctx(table)
st = C.st
G = B.Alphabet(C.R, C.M)
G.orbits()
GO = np.asarray(G.orbit, np.int64)
NO = len(G.orbit_size)
TYPE = np.asarray(G.type, np.int64)
holes = np.zeros(GL.NP, bool)
holes[G.holes] = True
HO = np.bincount(GO[np.flatnonzero(holes)], minlength=NO)
# which blocks make each unsafe grip unsafe (from the block files of mkblocks2.py next to the blocks json)
why = {int(c): {'off_inside_blocks': [], 'straddling_blocks': [], 'reason': r} for c, r in G.blocked.items()}
for bf in sorted(Path(bfile).parent.glob('*-b*.json')):
    b = json.loads(bf.read_text())
    first = min(b['pieces'])
    for c in b['inside']:
        if int(c) in why:
            why[int(c)]['off_inside_blocks'].append(first)
    for c in b['straddled']:
        if int(c) in why:
            why[int(c)]['straddling_blocks'].append(first)
rows = [json.loads(x) for x in Path(sfile).read_text().splitlines() if x.strip()]
rows = [r for r in rows if r.get('usable')]
rows.sort(key=lambda r: (r['n1'] + r['n2'], r['occ'], r['bad']))
res = {'off': int(len(C.off)), 'safe_grips': len(G.safe), 'unsafe_grips': {str(k): v for k, v in sorted(why.items())},
       'words': []}
for r in rows[:TOP]:
    W = [(int(c), int(i)) for c, i in r['W']]
    g, m, U = C.ghost(W, np.zeros(0, np.int64))
    bad = U[~g.alive[m:] | (g.dom[m:] != 0)]
    need = np.bincount(GO[bad], minlength=NO)
    item = {'W': r['W'], 'bad': int(len(bad)), 'n1': r['n1'], 'n2': r['n2'], 'short': [], 'fixed_occupied': []}
    for o in np.flatnonzero(need > HO).tolist():
        t = sorted(set(TYPE[GO == o].tolist()))
        th = np.flatnonzero(holes & np.isin(TYPE, t))
        where = {}
        for x in th.tolist():
            oo = int(GO[x])
            key = 'singleton' if G.orbit_size[oo] == 1 else f'orbit {oo} (size {int(G.orbit_size[oo])})'
            where.setdefault(key, []).append(x)
        stuck = []
        for x in where.get('singleton', []):
            grips = np.flatnonzero(G.home[x, :]).tolist()
            stuck.append({'hole': x, 'grips': {str(c): why.get(c) for c in grips}})
        item['short'].append({'orbit': o, 'size': int(G.orbit_size[o]), 'types': t, 'bad': int(need[o]),
                              'holes': int(HO[o]), 'holes_of_type_by_orbit': {k: len(v) for k, v in where.items()},
                              'stuck_holes': stuck})
    fx = bad[(G.orbit_size[GO[bad]] == 1) & ~holes[bad]]
    for x in fx.tolist():
        grips = np.flatnonzero(G.home[x, :]).tolist()
        item['fixed_occupied'].append({'chamber': x, 'type': int(TYPE[x]),
                                       'grips': {str(c): why.get(c) for c in grips}})
    res['words'].append(item)
Path(out).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps(res['words'])[:4000])
