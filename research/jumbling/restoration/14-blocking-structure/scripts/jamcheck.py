# Stage 14 (blocking structure), tool 2: the anatomy of the grips a lowering needs.
#
# For a state and its label table (blockmap.py --labels), the target grips are the own-branch caps that hold a pose
# group of height 4 or more whole (or the grips named with --caps). For each target x:
# - its straddlers in units: pieces with one pose (they move together under any twist whose cap holds them all), with
#   the unit's height, branch, size on the cut and in all, and its cell in its own class's cut arrangement (the caps
#   of the unit's branch class that hold it);
# - for each unit, the caps that hold all its straddlers of x whole, split into: clean and pure of the unit's class
#   (a twist there moves the unit and raises no height above 2), clean and impure of the unit's class, clean of
#   another class (a twist there raises the unit to height 4), and blocked;
# - the foreign load of x: the pieces of another branch at height 2 or more inside the cap (a jumble of x raises them),
#   in units with the same holder split.
# With --moves, every non-identity menu twist at every clean holder of a straddler or foreign unit is applied exactly
# (and undone), and the change of the depth Phi = sum of max(0, h - 2), the largest height, the off count, the
# straddlers and the foreign load of x, and the units that land on the lattice are recorded after it.
# Exact geometry; research only, headless Linux; no Windows, DirectX, input or performance claim.
#
# Run from the repository root:
#   python3 -I -B research/jumbling/restoration/14-blocking-structure/scripts/jamcheck.py <table.json> <labels.npz>
#       <out.json> [--caps c,c,...] [--moves] [--max-targets N]
import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blockmap as BM  # noqa: E402

C, M, KP, CLS, sim, R, T = BM.C, BM.M, BM.KP, BM.CLS, BM.sim, BM.R, BM.T


def phi(h):
    return int(np.maximum(h.astype(np.int64) - 2, 0).sum())


def exact_straddlers(st, x):
    """Every off piece whose posed region straddles the cut of grip x (the simulator's exact classifier)."""
    from sim.kernel import Evaluator, pullback_form
    from sim.state import Classification
    D, reg = st.ctx.data, st.ctx.regions
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    cl = Classification(int(x))
    out = []
    for pid, members in st._groups(off):
        ev = Evaluator(pullback_form(st._pose_matrix(pid), D.N[x]))
        for anchor, sub in st._anchors(pid, members):
            r = st._eval(ev, reg.cap_vform(anchor), lambda a=anchor: reg.cap_frows(a), cl, None)
            assert r[0] != 'uncertain'
            if r[0] != 'straddle':
                continue
            for p in sub.tolist():
                r = st._eval(ev, reg.vform(p), lambda q=p: reg.frows(q), cl, None)
                assert r[0] != 'uncertain'
                if r[0] == 'straddle':
                    out.append(p)
    return np.array(sorted(out), np.int64)


def anatomy(st, lab, h, b, x):
    S = np.flatnonzero(lab[:, x] == 2)
    Fx = np.flatnonzero((lab[:, x] == 1) & (h >= 2) & (b != CLS[x]))
    clean = ~(lab == 2).any(0)
    inside = lab == 1
    other_in = np.zeros(600, np.int64)
    for B in range(5):
        sel = (h >= 2) & (b == B)
        other_in += np.where(CLS == B, 0, inside[sel].sum(0))
    pure = clean & (other_in == 0)
    pid = st.pose_id

    def unit_rows(P, key):
        rows = []
        for g in np.unique(pid[P]).tolist():
            mem = P[pid[P] == g]
            H, Bu = int(h[mem[0]]), int(b[mem[0]])
            holders = np.flatnonzero(inside[mem].all(0))
            rows.append({'pose': int(g), 'height': H, 'branch': Bu, key: int(len(mem)),
                         'size': int((pid == g).sum()),
                         'cell': np.flatnonzero(inside[mem].all(0) & (CLS == Bu)).tolist(),
                         'holders': int(len(holders)),
                         'clean_pure_own': [int(c) for c in holders if clean[c] and pure[c] and CLS[c] == Bu],
                         'clean_impure_own': [int(c) for c in holders if clean[c] and not pure[c] and CLS[c] == Bu],
                         'clean_other': [int(c) for c in holders if clean[c] and CLS[c] != Bu],
                         'blocked_holders': int(sum(1 for c in holders if not clean[c]))})
        return rows

    units, funits = unit_rows(S, 'on_cut'), unit_rows(Fx, 'in_cap')
    load = {}
    for B in range(5):
        if B == CLS[x]:
            continue
        for H in sorted(set(h.tolist())):
            if H < 2:
                continue
            n = int((inside[:, x] & (h == H) & (b == B)).sum())
            if n:
                load[f'h{H}-b{B}'] = n
    return dict(cap=int(x), cls=int(CLS[x]), straddlers=int(len(S)), units=units,
                units_with_clean_pure_own_holder=int(sum(1 for u in units if u['clean_pure_own'])),
                units_with_clean_holder=int(sum(1 for u in units if u['clean_pure_own'] or u['clean_impure_own']
                                                or u['clean_other'])),
                foreign_load=load, foreign_units=funits, clean=bool(clean[x]), pure=bool(pure[x]))


def twist(st, c, g):
    o = st.apply(sim.Twist(C, c, g, 'menu', {'menu': M.name}), journal=False)
    assert not o.uncertain
    return o.applied


def tree_rows(st, pieces):
    """Left height and branch of the given pieces, from their current poses (right height 0 is checked)."""
    pid = st.pose_id[pieces]
    hs = np.zeros(len(pieces), np.int16)
    br = np.full(len(pieces), -1, np.int8)
    for q in np.unique(pid).tolist():
        p = st._poses[q]
        if p.kidx >= 0:
            continue
        lf, rf = T.factors(p.matrix)
        assert T.dist(rf) == 0
        sel = pid == q
        hs[sel], br[sel] = T.dist(lf), T.branch(lf)
    return hs, br


def labels_at(st, pieces, x):
    """Exact labels (0 outside, 1 inside, 2 straddles) of the given pieces at grip x."""
    k = st._kpose()[st.pose_id]
    lp, op = pieces[k[pieces] >= 0], pieces[k[pieces] < 0]
    groups = [(pid, st._anchors(pid, mem)) for pid, mem in st._groups(op)] if len(op) else []
    return BM._label_caps(st, [x], lp, k, groups)[pieces, 0]


def moves(st, lab, h, b, x, caps, units):
    """Every menu twist at each clean cap in caps, applied exactly and undone by its inverse (Proposition 2.2).

    Only the pieces inside the cap move, so heights, branches and labels at x are recomputed for them alone (exactly)
    and the rest is carried over: Phi, the largest height, the straddlers of x, the foreign load of x, the off count,
    and which straddler units of x (pose groups) land on the lattice.
    """
    from exact import transpose
    from sim.kplus import to_tuple
    out = []
    d0 = st.digest()
    S0 = lab[:, x] == 2
    F0 = (lab[:, x] == 1) & (h >= 2) & (b != CLS[x])
    for c in caps:
        moved = np.flatnonzero(lab[:, c] == 1)
        keep = np.ones(len(h), bool)
        keep[moved] = False
        for j in range(1, len(M.items)):
            g = R.element(M, c, j)
            if not twist(st, c, g):
                out.append(dict(cap=int(c), item=j, applied=False))
                continue
            h2, b2 = tree_rows(st, moved)
            l2 = labels_at(st, moved, x)
            landed = [u for u in units if np.isin(units[u], moved).all() and
                      (h2[np.searchsorted(moved, units[u])] == 0).all()]
            out.append(dict(cap=int(c), item=j, applied=True, moved=int(len(moved)),
                            dphi=phi(h2) - phi(h[moved]), max_h=int(max(h[keep].max(), h2.max())),
                            off=int(st.off_lattice_count()),
                            straddlers_x=int(S0[keep].sum() + (l2 == 2).sum()),
                            foreign_x=int(F0[keep].sum() + ((l2 == 1) & (h2 >= 2) & (b2 != CLS[x])).sum()),
                            landed_units=[int(u) for u in landed]))
            assert twist(st, c, to_tuple(transpose(g)))
    assert st.digest() == d0
    return out


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('table')
    ap.add_argument('labels')
    ap.add_argument('out')
    ap.add_argument('--caps')
    ap.add_argument('--moves', action='store_true')
    ap.add_argument('--max-targets', type=int, default=40)
    a = ap.parse_args(argv)
    st = BM.load_table(a.table)
    h, b = BM.tree_info(st)
    lab = BM.labels_for(st, a.labels, 1)
    if a.caps:
        targets = [int(c) for c in a.caps.split(',')]
    else:
        targets = []
        inside = lab == 1
        for g in np.unique(st.pose_id[h >= 4]).tolist():
            mem = np.flatnonzero(st.pose_id == g)
            own = np.flatnonzero(inside[mem].all(0) & (CLS == b[mem[0]]))
            targets.extend(int(c) for c in own if c not in targets)
    rows = [anatomy(st, lab, h, b, x) for x in targets[:a.max_targets]]
    if a.moves:
        for r in rows:
            x = r['cap']
            us = r['units'] + r['foreign_units']
            caps = sorted({c for u in us for c in u['clean_pure_own'] + u['clean_impure_own'] + u['clean_other']})
            S = np.flatnonzero((lab[:, x] == 2) | ((lab[:, x] == 1) & (h >= 2) & (b != CLS[x])))
            members = {u['pose']: S[st.pose_id[S] == u['pose']] for u in us}
            r['moves'] = moves(st, lab, h, b, x, caps, members)
    res = dict(digest=st.digest(), targets=targets, anatomy=rows, phi=phi(h))
    Path(a.out).write_text(json.dumps(res, indent=1))
    for r in rows:
        print(r['cap'], r['cls'], 'straddlers', r['straddlers'], 'units', len(r['units']),
              'clean_pure_own_units', r['units_with_clean_pure_own_holder'],
              'any_clean_holder_units', r['units_with_clean_holder'], 'load', r['foreign_load'], flush=True)


if __name__ == '__main__':
    main()
