"""Independent replay of library macros and position parities from poses and signatures only.

usage: verify_m8.py <research/jumbling> <library json> <out json> <macro id>...
For each macro: rebuild every twist from its exact record, check menu membership and admissibility
step by step from solved, check that every final pose is a rotation that permutes the 600 poles
(exact, without the K+ table), map each piece to the piece whose signature is the pole image of
its own, and report the sign of that permutation on every model orbit_id class. No coordinate
code of the restorer or of nucoords is used."""
import json
import sys
import time
from pathlib import Path

import numpy as np

J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim                                       # noqa: E402
import wj                                        # noqa: E402
from exact import matvec, Q5                     # noqa: E402
from sim.state import State                      # noqa: E402
from sim.twists import Twist                     # noqa: E402

T0 = time.time()
ctx = sim.get_context()
data = ctx.data
M = wj.make_menu(ctx, 'I_a')
assert M.identity == 'fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c', M.identity
lib = json.loads(Path(sys.argv[2]).read_text())
want = [int(x) for x in sys.argv[4:]]
mo, mv = data.mask_offsets, data.mask_values
sig_index = {tuple(mv[mo[p]:mo[p + 1]].tolist()): p for p in range(len(mo) - 1)}
oid = data.orbit_id
POLE = {tuple(n): i for i, n in enumerate(data.N)}


def det4(m):
    def det3(a):
        return (a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]))
    tot = Q5(0)
    for j in range(4):
        minor = [[m[i][k] for k in range(4) if k != j] for i in range(1, 4)]
        t = m[0][j] * det3(minor)
        tot = tot + t if j % 2 == 0 else tot - t
    return tot


def pole_perm(m):
    out = np.empty(600, np.int64)
    for c in range(600):
        img = tuple(matvec(m, data.N[c]))
        if img not in POLE:
            return None
        out[c] = POLE[img]
    return out


def parity(perm):
    seen = np.zeros(len(perm), bool)
    odd = 0
    for i in range(len(perm)):
        if seen[i]:
            continue
        j, L = i, 0
        while not seen[j]:
            seen[j] = True
            j = perm[j]
            L += 1
        odd ^= (L - 1) & 1
    return odd


report = []
for mid in want:
    mac = next(m for m in lib['macros'] if int(m['id']) == mid)
    st = State(ctx, menu=M)
    steps = []
    for i, rec in enumerate(mac['records']):
        tw = Twist.from_record(ctx, rec)
        assert M.contains(tw), f'M{mid} step {i}: not in I_a'
        out = st.apply(tw)
        assert out.applied, f'M{mid} step {i}: {out.status}'
        steps.append({'grip': tw.grip, 'retained': bool(tw.kidx >= 0), 'moved': int(out.moved),
                      'off_lattice_after': st.off_lattice_count()})
    # final poses: exact rotation permuting the poles
    pids = np.unique(st.pose_id)
    perm_of = {}
    for pid in pids.tolist():
        m = st._pose_matrix(pid)
        assert det4(m) == Q5(1), 'determinant is not 1'
        pp = pole_perm(m)
        assert pp is not None, f'M{mid}: a final pose does not permute the poles (off the lattice)'
        perm_of[pid] = pp
    lattice_sim = st.is_lattice()
    # position map: piece p -> piece with signature g(sig(p))
    NPc = len(mo) - 1
    dest = np.arange(NPc)
    moved = np.flatnonzero(st.pose_id != 0)
    for p in moved.tolist():
        pp = perm_of[int(st.pose_id[p])]
        sig = tuple(sorted(pp[mv[mo[p]:mo[p + 1]]].tolist()))
        q = sig_index.get(sig)
        assert q is not None, 'image signature is not a piece'
        dest[p] = q
    assert len(np.unique(dest)) == NPc, 'position map is not a bijection'
    assert np.all(oid[dest] == oid), 'a piece changed orbit class'
    signs = {}
    for o in range(35):
        idx = np.flatnonzero(oid == o)
        loc = {int(p): i for i, p in enumerate(idx)}
        perm = np.array([loc[int(dest[p])] for p in idx], np.int64)
        s = parity(perm)
        if s:
            signs[o] = s
    claimed = sorted(int(o) for o, v in mac['nu'].items() if v[0])
    report.append({'id': mid, 'family': mac['family'], 'length': len(mac['records']), 'steps': steps,
                   'final_distinct_poses': len(pids), 'lattice_by_pole_permutation': True,
                   'lattice_by_simulator': lattice_sim, 'moved_pieces': int(len(moved)),
                   'odd_orbits': sorted(signs), 'claimed_odd_orbits': claimed,
                   'agree': sorted(signs) == claimed,
                   'sign_O08': signs.get(8, 0), 'sign_O25': signs.get(25, 0)})
    print(json.dumps({k: v for k, v in report[-1].items() if k != 'steps'}), flush=True)
Path(sys.argv[3]).write_text(json.dumps({'menu_identity': M.identity, 'model_identity': data.identity,
                                         'macros': report, 'seconds': round(time.time() - T0, 1)}, indent=1))
