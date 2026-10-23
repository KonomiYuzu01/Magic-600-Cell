# Round 22, line (a), after the replay of R22A2-04: the premises of Theorem C for an edge unit of a screen record at a
# later pose table (after the depth 40 branch of r22c3_geometry.py): the unit is the whole own-class content of its
# edge cell, the cell's home chambers hold no lattice piece, both poles and a common neighbour are free (no straddler,
# no foreign off piece inside), and exactly one menu rotation at the first pole fixes the second and lands every
# piece of the unit. Prints the facts and THEOREM_C_PREMISES True or False. Exact; headless; research only.
# Usage (from the repository root): python3 -I -B thmc_premises.py <pose table> <screen record> <unit json> <unit id>
import json
import sys
from pathlib import Path

S = Path('research/jumbling/restoration/12-low-height')
sys.path.insert(0, str((S / 'scripts').resolve()))
import numpy as np  # noqa: E402
import screen as s  # noqa: E402
from exact import matmul, matvec  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

table, record, unitf, uid = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
st, M, _ = s.load('table', table)
rec = json.loads(Path(record).read_text())
row = next(u for u in rec['units'] if u['unit'] == uid)
P, cell = row['class'], row['cell']
upieces = np.array(sorted(json.loads(Path(unitf).read_text())['pieces']))
members, cls = s.CL.classes()
D = s.rim.Domains(st)
dom = np.asarray(D.dom)
dcls = {}
for d in range(1, len(D.reps)):
    h = s.CL.hub(D.reps[d], members, cls)
    dcls[d] = h[0][1] if len(h) == 1 else None
off = s.B.off_pieces(st)
print('STATE', st.digest(), 'off', len(off), 'unit', uid, 'class', P, 'cell', cell, 'unit_pieces', len(upieces),
      'unit_all_off', bool(np.isin(upieces, off).all()), flush=True)
cand = np.flatnonzero(np.isin(dom, [dd for dd in dcls if dcls[dd] == P]))
keep = np.ones(len(cand), bool)
cache = {}
for c in members[P]:
    c = int(c)
    cl = st.classify(c, need_inside=True, stop_at_straddle=False)
    assert not cl.uncertain
    cache[c] = cl
    keep &= (np.isin(cand, cl.inside) == (c in cell))
U = cand[keep]
whole = len(U) > 0 and np.array_equal(np.sort(U), upieces)
print('CELL_OWN_CONTENT', len(U), 'equals_unit', bool(whole), flush=True)


def free(c):
    cl = cache[c]
    return cl.straddle is None and all(dom[p] == 0 or dcls[int(dom[p])] == P for p in cl.inside)


def blockers(c):
    cl = cache[c]
    return sorted({int(dom[p]) for p in cl.inside if dom[p] != 0 and dcls[int(dom[p])] != P}), cl.straddle is not None


home = s.CL.data.sig_matrix(np.arange(len(st.pose_id)))
T = s.B.cell_chambers(home, members, P, cell)
Q = np.flatnonzero((dom == 0) & np.isin(D.cham, T))
poles = list(map(int, members[P]))
w = {c: s.O.UIDX[tuple(s.T.qmul(s.CL.ninv(poles[0]), s.CL.N[c]))] for c in poles}
nbr = {i for i, u in enumerate(s.O.U) if u[0] == s.O.HALF_PHI}
common = [c for c in poles if all(int(s.O.MUL[s.O.INV[w[a]], w[c]]) in nbr for a in cell)]
good = [c for c in common if free(c)]
print('GEOMETRY', 'home_chambers', len(T), 'occupied_by_lattice', len(Q), 'poles_free', [free(c) for c in cell],
      'pole_blockers', [blockers(c) for c in cell], 'common', common, 'free_common', good, flush=True)
choices = [i for i in range(len(M.items))
           if tuple(matvec(s.restore.element(M, cell[0], i), s.CL.N[cell[1]])) == tuple(s.CL.N[cell[1]])
           and all(s.CL.ctx.kplus.index_of_matrix(to_tuple(matmul(s.restore.element(M, cell[0], i),
                                                                  st.pose(int(p))))) >= 0 for p in U)]
print('LANDING_ROTATIONS', choices, flush=True)
ok = whole and len(Q) == 0 and all(free(c) for c in cell) and bool(good) and len(choices) == 1
print('THEOREM_C_PREMISES', ok, flush=True)
