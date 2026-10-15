"""SAFE-ALL closure of a conjugate R V R^-1, read from the current state only.

Input: a current-only pose table X and a candidate word R (chronological (grip, menu_index) pairs). The script

1. computes the inverse word R^-1 (mirror order, exact menu inverses);
2. replays R at X (every twist checked admissible) and classes the pieces: T_R (off at X, on after R), C_R (on at X,
   off after R), and the off pieces of X still off after R;
3. computes Bad(R) from R alone: a ghost replay of R^-1 on the 177,120 home chambers (ghost pieces at the identity,
   each straddle recorded per chamber); a chamber is bad when its R^-1 trajectory straddles a cut or ends off K;
4. at Y = X R reads the SAFE-ALL alphabet, its orbits and the holes, and tests the vacancy condition for the target
   Bad(R): (N1) bad chambers fixed by the group are holes, (N2) every orbit has at least as many holes as bad
   chambers; it then builds a peeling certificate for the movable bad chambers (Theorem B) and the greedy word V,
   which reads only the holes of Y;
5. writes the word R V R^-1 and its prediction: if the certificate condition holds and V clears Bad(R), then R^-1
   is admissible after R V, the pieces of T_R are on the lattice at the end, every lattice piece of X is on the
   lattice, and every off piece of X outside T_R has its exact pose of X (proof: section 2 of the note).

The constructor never reads a history: only the pose table and the candidate R.

Usage: python -I conjprep.py <theory dir> <pose table> <R word json or file> <out prefix> [--maxg 2]
Evidence: exact simulator replay and exact ghost replays; no native host evidence.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import buffer as B  # noqa: E402
import sim  # noqa: E402
import conjclose as CC  # noqa: E402

CL, O, restore = B.CL, B.O, B.restore
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def inverse_word(M, word):
    out = []
    cache = {}
    for c, i in reversed(word):
        if (c, i) not in cache:
            m = restore.element(M, c, i)
            hit = None
            for j in range(len(M.items)):
                prod = restore.matmul(m, restore.element(M, c, j))
                if all(prod[r][q] == (1 if r == q else 0) for r in range(4) for q in range(4)):
                    hit = j
                    break
            if hit is None:
                raise ValueError(f'letter ({c}, {i}) has no inverse in the menu')
            cache[(c, i)] = hit
        out.append((c, cache[(c, i)]))
    return out


def bad_chambers(M, word_inv, NP):
    """Chambers whose trajectory under word_inv straddles a cut or ends off K (state independent)."""
    sol = sim.State(CL.ctx, menu=M)
    alive, first_bad, g = CC.ghost_run(M, word_inv, None, np.arange(NP), sol)
    K = g._kpose()[g.pose_id]
    good = alive & (K >= 0)
    return ~good, alive


def generic_certificate(G, T):
    """Peeling certificate (Theorem B) for an arbitrary target chamber set T at the alphabet G."""
    G.orbits()
    T = np.asarray(sorted(set(int(x) for x in T)), np.int64)
    fixed = T[G.orbit_size[G.orbit[T]] == 1]
    movable = T[G.orbit_size[G.orbit[T]] > 1]
    tr = B.Transport(G, movable)
    chosen = tr.peel()
    macros = []
    for _, w, _ in chosen:
        p = G.perm(w)
        macros.append((w, p, tr.support(p)))
    free, protected, rows = np.ones(len(G.letters), bool), [], []
    for j, (t, w, y) in enumerate(chosen):
        tree, extra = B.close_routes(tr, t, y, protected, free, macros[j:])
        metrics = B._route_metrics(G, t, protected, w, tree)
        rows.append(dict(t=t, orbit=int(G.orbit[t]), y=y, word=G.pairs(w), **metrics,
                         conjugate_edges=[dict(e, word=G.pairs(e['word'])) for e in extra]))
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    doc = {'format': 'generic-target-1', 'off_geometry_fingerprint': G.fingerprint, 'source_digest': G.state_digest,
           'menu': G.M.name, 'target_chambers': T.tolist(), 'fixed': fixed.tolist(), 'peeled': len(chosen),
           'of': len(movable), 'rows': rows, **B._totals(G, T, rows)}
    return doc


def generic_prepare(G, cert, T, holes=None):
    """The greedy procedure of buffer.prepare for an arbitrary target T (reads only the holes)."""
    T = np.asarray(cert['target_chambers'], np.int64)
    holes = B._holes_mask(G, holes)
    cond = B.condition(G, T, None, holes)
    thr = {int(o): v for o, v in cert['thresholds'].items()}
    counts = np.bincount(G.orbit[np.flatnonzero(holes)], minlength=len(G.orbit_size))
    deficits = {o: v - int(counts[o]) for o, v in thr.items() if counts[o] < v}
    if not cond['fixed_vacant'] or deficits:
        return None, cond, deficits
    tr, macros, extras = B._decoded_routes(G, cert)
    protected, free, word, steps = [], np.ones(len(G.letters), bool), [], []
    for j, row in enumerate(cert['rows']):
        t, y = row['t'], row['y']
        if not holes[t]:
            tree = B.weighted_tree(tr, y, free, macros[j:], extras[j])
            candidates = holes & (tree[0] >= 0)
            candidates[protected], candidates[t] = False, False
            points = np.flatnonzero(candidates)
            if not len(points):
                raise ValueError(f'no reached hole is available for target {t}')
            z = int(points[np.argmin(tree[0][points])])
            path = B.tree_path(tr, tree, macros[j:], z, extras[j])
            route = G.inverse(path)
            if len(path) != tree[0][z] or G.image(route, z) != y:
                raise AssertionError('reconstructed route does not match its shortest path')
            step = route + macros[j][0]
            p = G.perm(step)
            predicted = np.zeros(G.NP, bool)
            predicted[p[holes]] = True
            holes = predicted
            word.extend(step)
            steps.append({'row': j, 'target': t, 'hole': z, 'route_length': len(route), 'length': len(step)})
        if not holes[t] or not holes[protected].all():
            raise AssertionError('greedy step did not preserve the cleared target prefix')
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    if not holes[T].all():
        raise AssertionError('preparation did not clear the target')
    return {'word': G.pairs(word), 'predicted_holes': np.flatnonzero(holes).tolist(), 'length': len(word),
            'filled': len(steps), 'steps': steps}, cond, deficits


def main():
    a = sys.argv[1:]
    table, rarg, prefix = a[1], a[2], a[3]
    maxg = int(a[a.index('--maxg') + 1]) if '--maxg' in a else 2
    Rw = json.loads(Path(rarg).read_text()) if Path(rarg).exists() else json.loads(rarg)
    if isinstance(Rw, dict):
        Rw = Rw['word']
    Rw = [(int(c), int(i)) for c, i in Rw]
    Rr, M = O.start(table)
    st = Rr.st
    NP = len(st.pose_id)
    d_x = st.digest()
    Rinv = inverse_word(M, Rw)
    codes = CC.Codes()
    c_x = codes.of(st)
    holes_x = B.hole_set(st)
    res = {'input_digest': d_x, 'R': Rw, 'R_inverse': Rinv, 'off_X': int((c_x < 0).sum())}
    for j, (c, i) in enumerate(Rw):
        Rr.apply(c, i)
    c_y = codes.of(st)
    d_y = st.digest()
    T_R = (c_x < 0) & (c_y >= 0)
    C_R = (c_x >= 0) & (c_y < 0)
    still = (c_x < 0) & (c_y < 0)
    res['after_R'] = {'digest': d_y, 'off': int((c_y < 0).sum()), 'T_R': int(T_R.sum()), 'C_R': int(C_R.sum()),
                      'off_of_X_still_off': int(still.sum()),
                      'off_of_X_still_off_moved': int((still & (c_x != c_y)).sum())}
    log(after_R=res['after_R'])
    bad, alive = bad_chambers(M, Rinv, NP)
    res['bad'] = {'count': int(bad.sum()), 'unclean': int((~alive).sum())}
    G = B.Alphabet(Rr, M)
    G.orbits()
    holes_y = G.holes
    occ = np.ones(NP, bool)
    occ[holes_y] = False
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    ORB, TRN, POF = np.asarray(reg.orbit_of), np.asarray(reg.transport), reg.piece_of
    on = np.flatnonzero(c_y >= 0)
    chamber = np.full(NP, -1, np.int64)
    chamber[on] = POF[ORB[on], kp.compose_vec(c_y[on], TRN[on])]
    res['bad']['occupied_after_R'] = int((occ & bad).sum())
    res['bad']['occupied_after_R_equals_T_R_chambers'] = bool(
        np.array_equal(np.flatnonzero(occ & bad), np.sort(chamber[T_R])))
    res['bad']['holes_after_R'] = int(len(holes_y))
    target = np.flatnonzero(bad)
    cond = B.condition(G, target, None, None)
    res['condition'] = {k: cond[k] for k in ('fixed_vacant', 'necessary')}
    res['condition']['fixed_occupied'] = cond['fixed_occupied'][:50]
    res['condition']['necessary_deficits'] = cond['necessary_deficits'][:50]
    res['condition']['orbits_meeting_target'] = len(cond['by_orbit'])
    res['condition']['target_in_fixed'] = int((G.orbit_size[G.orbit[target]] == 1).sum())
    res['safe_all'] = {'grips': len(G.safe), 'letters': len(G.letters), 'orbits': int(len(G.orbit_size))}
    log(condition=res['condition'], bad=res['bad'])
    out = Path(prefix)
    if cond['necessary']:
        try:
            cert = generic_certificate(G, target)
            res['certificate'] = {k: cert[k] for k in ('peeled', 'of', 'family_bound', 'all_reached',
                                                       'thresholds_equal_targets')}
            res['certificate']['thresholds'] = cert['thresholds']
            prep, cond2, deficits = generic_prepare(G, cert, target)
            res['certificate']['deficits'] = deficits
            if prep is not None:
                V = prep['word']
                W = [list(x) for x in Rw] + V + [list(x) for x in Rinv]
                res['V'] = {'length': prep['length'], 'filled': prep['filled']}
                res['W'] = {'length': len(W), 'R': len(Rw), 'V': len(V), 'R_inverse': len(Rinv)}
                Path(str(out) + '-word.json').write_text(json.dumps({'word': W, 'input_digest': d_x}) + '\n')
                Path(str(out) + '-cert.json').write_text(json.dumps(cert) + '\n')
        except ValueError as exc:
            res['certificate_error'] = str(exc)
    res['seconds'] = round(time.time() - T0)
    Path(str(out) + '.json').write_text(json.dumps(res, indent=1) + '\n')
    log(done=str(out))


if __name__ == '__main__':
    main()
