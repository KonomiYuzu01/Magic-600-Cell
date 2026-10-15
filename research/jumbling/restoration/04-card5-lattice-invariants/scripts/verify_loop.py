"""Exact replay of a loop-search hit (menu I_a): the word U A B^-1 U^-1 (A, B lists of (grip, K+ index) retained
letters) from solved, every letter checked admissible by the simulator; at the end every piece must be on the
lattice; prints the position sign of every model orbit from the final poses (pieces mapped to chambers) and the
menu-item word. usage: verify_loop.py <research/jumbling> <classes.json> <arrays.npz> <loopsearch json> <hit index> <U word g:i,...> <out json>"""
import json, sys
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'e3s'))
sys.path.insert(0, str(Path(__file__).resolve().parent))  # package layout: e3lib.py next to this script
from e3lib import Lib  # noqa: E402
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
oid = np.asarray(L.d.orbit_id)
hit = json.load(open(sys.argv[4]))['hits'][int(sys.argv[5])]
U = [tuple(int(v) for v in w.split(':')) for w in sys.argv[6].split(',')]


def item_of(g, a):
    for i in range(len(L.M.items)):
        if i in L.JI:
            continue
        if L.kidx_letter(g, i) == a:
            return (g, i)
    raise KeyError((g, a))


A = [item_of(g, a) for g, a in hit['A']]
Binv = [item_of(g, int(L.inv[a])) for g, a in reversed(hit['B'])]
Uinv = [L.inv_letter(l) for l in reversed(U)]
word = U + A + Binv + Uinv
st = L.sim.State(L.ctx, menu=L.M)
for n, (c, i) in enumerate(word):
    tw = L.sim.Twist(L.ctx, c, L.RS.element(L.M, c, i), 'menu', {'menu': L.M.name})
    assert st.apply(tw, journal=False).applied, ('inadmissible', n, (c, i))
ch = L.lattice_chambers(st)
assert (ch >= 0).all(), 'not on the lattice'
signs = {}
for o in sorted(set(oid.tolist())):
    ps = np.flatnonzero(oid == o)
    m = dict(zip(ps.tolist(), ch[ps].tolist()))
    seen, sg = set(), 0
    for a in m:
        if a in seen:
            continue
        n, b = 0, a
        while b not in seen:
            seen.add(b); b = m[b]; n += 1
        sg ^= (n - 1) & 1
    signs[o] = sg
odd = [o for o, s in signs.items() if s]
out = {'word': [list(w) for w in word], 'length': len(word), 'lattice_end': True, 'odd_orbits': odd, 'digest': st.digest()}
print(json.dumps({k: v for k, v in out.items() if k != 'word'}))
json.dump(out, open(sys.argv[7], 'w'), indent=1)
