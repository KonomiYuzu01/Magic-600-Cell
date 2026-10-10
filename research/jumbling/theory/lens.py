"""J3-R phase L: the abelian invariant v of lattice configurations, on the retained generators and
on the lens words (restoration.md, section 4).

For a lattice configuration (a K+ pose per piece), v gives, for every retained orbit of the model
(`orbit_id`; -1 is the centres), the parity of the permutation of that orbit and the sum of the
piece frames in the abelianised stabiliser S^ab of its K+ orbit. Frames: a piece p in pose k lands
on q = k(p); its frame is t_q^-1 k t_p, an element of the stabiliser of the orbit representative.
The wreath-product argument makes v a homomorphism on lattice configurations, so v(G) is spanned by
the values of the 1,200 retained generators, and v(X) outside v(G) would prove X outside G. The
converse does not hold: v(X) in v(G) proves nothing.

Lens words: W = (0, q)(d, a)(0, q^-1), q a quarter-turn of S4_0 or a fifth-turn of I_a at cap 0,
a a non-identity element of A4_d, kept when W ends on the lattice and moves neither 0 nor 3,097
pieces (the item 4 survey of the theory draft: d in {24, 42, 74, 108}).

Run from the repository root (about 10 minutes):
    python research/jumbling/theory/lens.py      writes results/lens-invariants.json
"""
import json
import math
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))

import sim  # noqa: E402
from exact import ZERO, identity  # noqa: E402
from groups import build, lift  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

ctx = sim.get_context()
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
orbit_of, transport, piece_of = reg.orbit_of, reg.transport, reg.piece_of
ROB = data.orbit_id                       # retained orbit of every piece (-1: centres)
IDK = kp.index_of_matrix(to_tuple(identity(4)))
_inv = {}


def kinv(i):
    if i not in _inv:
        row = kp.compose_vec(np.full(7200, i, np.int64), np.arange(7200, dtype=np.int64))
        _inv[i] = int(np.nonzero(row == IDK)[0][0])
    return _inv[i]


def abelianisations():
    """Per K+ orbit: map stabiliser element -> class in S^ab, and the S^ab multiplication table."""
    out = {}
    for o in range(len(reg.reps)):
        S = np.nonzero(piece_of[o] == reg.reps[o])[0].tolist()
        comm = {IDK}
        for a in S:
            for b in S:
                comm.add(kp.compose(kp.compose(a, b), kp.compose(kinv(a), kinv(b))))
        grown = True
        while grown:
            grown = False
            for a in list(comm):
                for b in list(comm):
                    c = kp.compose(a, b)
                    if c not in comm:
                        comm.add(c)
                        grown = True
        key = {s: min(kp.compose(s, c) for c in comm) for s in S}
        reps = sorted(set(key.values()))
        cls = {s: reps.index(key[s]) for s in S}
        mult = {(cls[a], cls[b]): cls[kp.compose(a, b)] for a in S for b in S}
        out[o] = {'cls': cls, 'mult': mult, 'n': len(reps), 'id': cls[IDK], 'stab': len(S)}
    return out


AB = None


def v(K):
    """{retained orbit: (parity, S^ab class)} for the lattice configuration with K+ poses K, only
    where it differs from the identity; None if a piece leaves its retained orbit."""
    K = np.asarray(K, np.int64)
    dest = piece_of[orbit_of, kp.compose_vec(K, transport)]
    if np.any(ROB[dest] != ROB):
        return None
    moved = np.nonzero(K != IDK)[0].tolist()
    odd, acc, seen = {}, {}, set()
    for s in moved:
        if s in seen:
            continue
        j, n = s, 0
        while j not in seen:
            seen.add(j)
            j = int(dest[j])
            n += 1
        r = int(ROB[s])
        odd[r] = odd.get(r, 0) ^ ((n - 1) & 1)
    for p in moved:
        o, r = int(orbit_of[p]), int(ROB[p])
        ab = AB[o]
        if ab['n'] <= 1:
            continue
        f = kp.compose(kinv(int(transport[dest[p]])), kp.compose(int(K[p]), int(transport[p])))
        acc[r] = ab['mult'][(acc.get(r, ab['id']), ab['cls'][f])]
    out = {}
    for r in set(odd) | set(acc):
        o = int(orbit_of[np.nonzero(ROB == r)[0][0]])
        a = acc.get(r, AB[o]['id'])
        if odd.get(r, 0) or a != AB[o]['id']:
            out[r] = (odd.get(r, 0), a)
    return out


def main():
    global AB
    t0 = time.time()
    AB = abelianisations()
    res = {'stabiliser_orders': sorted({a['stab'] for a in AB.values()}),
           'abelianisation_orders': sorted({a['n'] for a in AB.values()})}
    gens = {}
    for c in range(600):
        for i in (1, 4):
            st = sim.State(ctx)
            st.apply(sim.a4_element(ctx, c, i))
            val = v(st._kpose()[st.pose_id])
            gens[json.dumps(sorted(val.items()))] = gens.get(json.dumps(sorted(val.items())), 0) + 1
    res['generator_values'] = [{'value': json.loads(k), 'generators': n} for k, n in gens.items()]
    ex = build()

    def angle3(m):
        tr = float(sum((m[k][k] for k in range(3)), ZERO))
        return round(math.degrees(math.acos(max(-1, min(1, (tr - 1) / 2)))))
    lens = []
    for name, target in (('S4', 90), ('I_a', 72)):
        m = next(m for m in ex[name] if angle3(m) == target)
        q = sim.Twist(ctx, 0, lift(m))
        for d in (24, 42, 74, 108):
            for i in range(1, 12):
                st = sim.State(ctx)
                st.apply(q)
                if not st.apply(sim.a4_element(ctx, d, i)).applied:
                    continue
                if not st.apply(q.inverse()).applied or not st.is_lattice():
                    continue
                K = st._kpose()[st.pose_id]
                moved = int((K != IDK).sum())
                if moved in (0, 3097):
                    continue
                val = v(K)
                lens.append({'menu': name, 'd': d, 'a4_item': i, 'moved': moved,
                             'leaves_retained_orbit': val is None,
                             'value': None if val is None else sorted(val.items())})
    res['lens_words'] = lens
    res['seconds'] = round(time.time() - t0, 1)
    (HERE / 'results' / 'lens-invariants.json').write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: (v_ if k != 'lens_words' else len(v_)) for k, v_ in res.items()}))


if __name__ == '__main__':
    main()
