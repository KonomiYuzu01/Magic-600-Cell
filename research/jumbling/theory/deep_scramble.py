"""A deep scramble sample: most pieces off the lattice, for the views and as a hard restoration input.

Random admissible twists stop at about two thirds of the pieces off the lattice (I_a: 63 to 69% from
700 to 1,400 twists), because every jumble twist blocks the grips nearest to it and leaves islands
of lattice pieces that no free grip reaches. This script uses the structure of the grip poles instead:
- the 600 poles are the vertices of a 120-cell and split into five inscribed 600-cells of 120 poles
  each (pairwise inner products of unit poles in {0, +-1/2, +-phi/2, +-1/(2 phi), +-1});
- inscribed: the five sets, each pole's neighbours at 36 degrees inside its set, and the share of
  pieces that lie inside some cap of the set (home membership);
- profile: one twist from solved at the set's first pole, for every non-identity element, and the
  status of every grip within 56 degrees, counted by angle shell;
- blocking: for every pole c of one set and every non-identity menu element, one twist from the
  solved state, then the status of c's twelve neighbours in the set (exact survey);
- scramble: rounds over that set's 120 poles in random order, one twist per pole, each pole taking
  the admissible element that leaves the most pieces off the lattice (float score of the new poses
  against K+; ties at random); a pole found blocked is retried after the others. The off-lattice
  count reported is the simulator's exact count;
- compose: an exact check of the lattice-prefix rule on a small case. G is k1 random retained
  twists, L is k2 random admissible twists recorded from the solved state, W the configuration L
  reaches. Then L is applied after G, and every piece's exact pose is compared with W[s(p)] . G[p],
  where s(p) is the piece whose home place p occupies after G (matched by float centroids).

Run from the repository root:
    python research/jumbling/theory/deep_scramble.py <menu> [--set 0] [--rounds 3] [--seed 7]
        [--blocking-poles 120] [--compose 60,60] [--out <file>]
Writes results/deep-scramble-<menu>.json by default; progress goes to stderr.

Evidence kind: source and exact synthetic geometry (the blocking survey and the off-lattice counts
are exact; the element choice uses float scores; the composition check matches places by float
centroids and compares poses exactly). Timings are cloud timings of research code, not performance
evidence. The sample is a scramble, not a restoration step: no rule of phase U reads it.
"""
import argparse
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import restore  # noqa: E402
import seams  # noqa: E402
import sim  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kernel import q5_float  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402
from sim.model import NP  # noqa: E402

ctx = restore.ctx
KP = ctx.kplus
INNER = np.array([1, (1 + 5 ** .5) / 4, .5, (5 ** .5 - 1) / 4, 0])


def log(**kw):
    print(json.dumps(kw), file=sys.stderr, flush=True)


def unit_poles():
    n = np.array([[float(x) for x in r] for r in ctx.data.N])
    return n / np.linalg.norm(n, axis=1)[:, None]


def inscribed():
    """The five inscribed 600-cells among the poles (greedy by allowed inner products)."""
    g = unit_poles() @ unit_poles().T
    ok = (np.abs(np.abs(g)[:, :, None] - INNER[None, None, :]) < 1e-9).any(-1)
    sets, left = [], set(range(600))
    while left:
        s = [min(left)]
        for c in sorted(left):
            if c != s[0] and ok[c, s].all():
                s.append(c)
        sets.append(s)
        left -= set(s)
    off, cp = np.asarray(ctx.data.cap_offsets), np.asarray(ctx.data.cap_piece)
    rows = []
    for s in sets:
        nb = (np.abs(g[np.ix_(s, s)] - INNER[1]) < 1e-9).sum(1)
        cov = np.zeros(NP, bool)
        for c in s:
            cov[cp[off[c]:off[c + 1]]] = True
        rows.append({'poles': s, 'size': len(s), 'neighbours_at_36': sorted(set(nb.tolist())),
                     'pieces_inside_some_cap': int(cov.sum()), 'share': round(float(cov.mean()), 4)})
    return rows, g


def twist(M, c, i):
    return sim.Twist(ctx, c, restore.element(M, c, i), 'menu', {'menu': M.name})


def blocking(M, poles, g, items):
    """Neighbours in the set blocked by one twist from solved, for every pole and element."""
    blocked, t0 = {}, time.time()
    for k, c in enumerate(poles):
        nb = [d for d in poles if abs(g[c, d] - INNER[1]) < 1e-9]
        for i in items:
            st = sim.State(ctx, menu=M)
            assert st.apply(twist(M, c, i)).applied
            sv = st.survey(grips=nb)
            n = sum(r['status'] != 'admissible' for r in sv.values())
            if n:
                blocked.setdefault(str(c), []).append([i, n])
        if (k + 1) % 10 == 0:
            log(blocking_poles=k + 1, sec=round(time.time() - t0))
    return {'poles': len(poles), 'elements_per_pole': len(items), 'twists_checked': len(poles) * len(items),
            'twists_blocking_a_neighbour': sum(len(v) for v in blocked.values()), 'detail': blocked}


def profile(M, c, g, items):
    """Blocked grips by angle shell after one twist from solved at pole c, per element."""
    ang = np.round(np.degrees(np.arccos(np.clip(g[c], -1, 1))), 1)
    near = [int(d) for d in np.flatnonzero(ang < 56) if d != c]
    shells = sorted(set(ang[near].tolist()))
    out = {}
    for i in items:
        st = sim.State(ctx, menu=M)
        assert st.apply(twist(M, c, i)).applied
        sv = st.survey(grips=near)
        b = [d for d, r in sv.items() if r['status'] != 'admissible']
        row = tuple(int((ang[b] == a).sum()) for a in shells)
        out.setdefault(row, []).append(i)
    return {'pole': c, 'shells_deg': shells, 'grips_per_shell': [int((ang[near] == a).sum()) for a in shells],
            'profiles': [{'blocked_per_shell': list(k), 'blocked': sum(k), 'elements': v} for k, v in out.items()]}


def fl(m):
    return np.array([[q5_float(x) for x in r] for r in m])


def scramble(M, poles, rounds, seed, items):
    key = lambda a: tuple(np.round(a, 6).ravel().tolist())  # noqa: E731
    kset = {key(fl(KP.matrix(i))) for i in range(len(KP.perms))}
    el, pf = {}, {}
    rng = random.Random(seed)
    st = sim.State(ctx, menu=M)
    n = len(st.pose_id)
    moves, per_round, t0 = [], [], time.time()
    for r in range(rounds):
        todo = poles[:]
        rng.shuffle(todo)
        retried = 0
        for attempt in range(3):
            left = []
            for c in todo:
                cl = st.classify(c, need_inside=True, stop_at_straddle=True)
                if cl.straddle is not None or cl.uncertain:
                    left.append(c)
                    continue
                uniq, cnt = np.unique(st.pose_id[cl.inside], return_counts=True)
                poses = []
                for u in uniq.tolist():
                    p = st._poses[u]
                    if p.key not in pf:
                        pf[p.key] = (fl(st._pose_matrix(u)), p.kidx >= 0)
                    poses.append(pf[p.key])
                on = np.array([lat for _, lat in poses])
                scores = []
                for i in items:
                    if (c, i) not in el:
                        el[(c, i)] = fl(restore.element(M, c, i))
                    on2 = np.array([key(el[(c, i)] @ m) in kset for m, _ in poses])
                    scores.append((int(((on & ~on2) * cnt).sum() - ((~on & on2) * cnt).sum()), i))
                top = max(s for s, _ in scores)
                i = rng.choice([i for s, i in scores if s == top])
                assert st.apply(twist(M, c, i)).applied
                moves.append([c, i])
            if not left:
                break
            retried += len(left)
            todo = left
        row = {'round': r + 1, 'twists': len(moves), 'retried': retried, 'blocked_after_retries': len(left),
               'off_lattice_pieces': st.off_lattice_count(), 'share_off': round(st.off_lattice_count() / n, 4),
               'poses': len(st._poses), 'sec': round(time.time() - t0)}
        per_round.append(row)
        log(**row)
    return st, moves, per_round


def compose_check(M, k1, k2, seed=1):
    """Exact check of the lattice-prefix rule: a twist sequence admissible from solved stays
    admissible after any lattice configuration G, and moves the piece now in a place exactly as it
    moved that place's home piece: E[p] = W[s(p)] . G[p]."""
    J = restore.jumble_items(M)
    every = list(range(len(M.items)))
    ret = [i for i in every if i not in J and restore.element(M, 0, i) != seams.I4]
    rng = random.Random(seed)
    g = sim.State(ctx, menu=M)
    while len(g.journal) < k1:
        g.apply(twist(M, rng.randrange(600), rng.choice(ret)))
    w, L = sim.State(ctx, menu=M), []
    while len(L) < k2:
        c = rng.randrange(600)
        i = rng.choice(every if rng.random() < 0.5 else J)
        if w.apply(twist(M, c, i)).applied:
            L.append((c, i))
    assert g.is_lattice()
    home = seams.home_centroids()
    look = {tuple(np.round(x, 5)): p for p, x in enumerate(home)}
    gid = g.pose_id.copy()
    gmat = {u: g._pose_matrix(u) for u in np.unique(gid).tolist()}
    gflt = {u: fl(m) for u, m in gmat.items()}
    sig = np.array([look[tuple(np.round(gflt[int(gid[p])] @ home[p], 5))] for p in range(NP)])
    assert len(np.unique(sig)) == NP, 'G does not permute the home places'
    applied = sum(bool(g.apply(twist(M, c, i)).applied) for c, i in L)
    triples = np.unique(np.stack([w.pose_id[sig], gid, g.pose_id], 1), axis=0)
    mismatched = sum(to_tuple(matmul(w._pose_matrix(int(a)), gmat[int(b)])) != to_tuple(g._pose_matrix(int(e)))
                     for a, b, e in triples.tolist())
    moved = int((w.pose_id != 0).sum())
    return {'k1_retained': k1, 'k2_sequence': k2, 'seed': seed, 'applied_after_G': applied,
            'pieces_moved_by_L_from_solved': moved, 'distinct_pose_triples': len(triples),
            'exact_mismatches': int(mismatched)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('menu')
    ap.add_argument('--set', type=int, default=0)
    ap.add_argument('--rounds', type=int, default=3)
    ap.add_argument('--seed', type=int, default=7)
    ap.add_argument('--blocking-poles', type=int, default=120)
    ap.add_argument('--compose', default='60,60')
    ap.add_argument('--out')
    a = ap.parse_args()
    M = restore.menu(a.menu)
    items = [i for i in range(len(M.items)) if restore.element(M, 0, i) != seams.I4]
    sets, g = inscribed()
    log(inscribed=[{k: v for k, v in s.items() if k != 'poles'} for s in sets])
    poles = sets[a.set]['poles']
    out = {'menu': a.menu, 'inscribed_600_cells': sets, 'set': a.set}
    out['blocking_profile'] = profile(M, poles[0], g, items)
    log(profile=out['blocking_profile'])
    out['blocking_from_solved'] = blocking(M, poles[:a.blocking_poles], g, items)
    log(blocking={k: v for k, v in out['blocking_from_solved'].items() if k != 'detail'})
    st, moves, per_round = scramble(M, poles, a.rounds, a.seed, items)
    out['scramble'] = {'seed': a.seed, 'rounds': a.rounds, 'per_round': per_round, 'moves': moves,
                       'off_lattice_pieces': st.off_lattice_count(), 'pieces': NP,
                       'moved_pieces': st.moved_count(), 'digest': st.digest()}
    k1, k2 = (int(x) for x in a.compose.split(','))
    out['compose_check'] = compose_check(M, k1, k2)
    log(compose=out['compose_check'])
    path = Path(a.out) if a.out else HERE / 'results' / f'deep-scramble-{a.menu}.json'
    path.write_text(json.dumps(out, indent=1))
    log(written=str(path))


if __name__ == '__main__':
    main()
