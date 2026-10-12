"""One-letter setups w v w^-1, read from the current exact state, when no SAFE-ALL word clears the target.

At a state X whose hole counts fail Proposition N for the target T (a class cell), a setup is an admissible
retained letter w that moves off blocks (its cut not straddled, off pieces inside its cap), a SAFE-ALL word v at
Y = wX, and w^-1. It returns every off piece to its exact pose and changes the holes by a chamber permutation.
For each candidate letter, in the order given:

1. flow at Y: supply |V(Y) & O| for every orbit O of G(Y), capacity |{x in D : w(x) in O}| from O to every
   orbit D of G(X) meeting T outside the fixed chambers, demand |T & D|. Below the demand no v can work
   (Proposition N at Y), and the letter is skipped;
2. a target T_Y at Y: for every unit of flow O -> D one chamber y of O with w^-1(y) in D, chambers of w(T)
   first, then current holes (a compact target keeps the peeling certificate small);
3. a peeling certificate for T_Y at Y (``buffer.Transport.peel`` and ``buffer.close_routes``) and its greedy
   preparation v, reading only the holes of Y; a letter whose certificate or preparation fails is skipped;
4. w^-1. Every orbit D of G(X) now holds at least |T & D| holes; the certificate of X (computed from the table,
   and valid because the off geometry is X's again) gives the greedy preparation that clears T.

Then everything is replayed from the table: every twist admissible (the simulator refuses others), depth one
at every prefix, the off pieces outside the moved blocks at their exact poses during the setup, all off poses
equal to the source after w^-1 and during the preparation, predicted holes equal to actual holes, T vacant.
With ``--suffix`` a word file (for example the 86-piece restoration word) is replayed after the preparation,
checking depth one at every prefix; the end reports the restored pieces, whether every source lattice piece is
on the lattice and whether the remaining off pieces have their exact source poses.

Usage (from the repository root)::

    python research/jumbling/theory/setup_flow.py --table <pose table> --cls 0 --cell 369 507 \
        [--letters <grip,menu> ...] [--suffix <word file>] --out <file>
    python research/jumbling/theory/setup_flow.py --table <pose table> --classify --out <file>

Evidence: exact simulator replay on the I_a fixture geometry, not native host or performance evidence.
"""
import argparse
import json
import sys
import time
from collections import deque
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import buffer as B  # noqa: E402

CL, O, restore = B.CL, B.O, B.restore
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def max_flow(cap, source, sink):
    """Edmonds-Karp on a dict {(a, b): capacity}; returns the value and the flow on every given edge."""
    res, adj = {}, {}
    for (a, b), v in cap.items():
        res[(a, b)] = res.get((a, b), 0) + v
        res.setdefault((b, a), 0)
        adj.setdefault(a, set()).add(b)
        adj.setdefault(b, set()).add(a)
    value = 0
    while True:
        par = {source: source}
        dq = deque([source])
        while dq and sink not in par:
            u = dq.popleft()
            for v in sorted(adj.get(u, ()), key=str):
                if v not in par and res[(u, v)] > 0:
                    par[v] = u
                    dq.append(v)
        if sink not in par:
            return value, {e: cap[e] - res[e] for e in cap}
        f, v = None, sink
        while v != source:
            f = res[(par[v], v)] if f is None else min(f, res[(par[v], v)])
            v = par[v]
        v = sink
        while v != source:
            res[(par[v], v)] -= f
            res[(v, par[v])] += f
            v = par[v]
        value += f


def letter_map(G, c, k):
    """Chamber map of the letter (c, k): its cap moved by k, every other chamber fixed."""
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    xs = np.flatnonzero(G.home[:, c])
    p = np.arange(G.NP)
    p[xs] = reg.piece_of[reg.orbit_of[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
    return p


def setup_target(GX, GY, p, T, holes_x, choose=True):
    """Flow at Y and the target T_Y it gives, or (value, need, None) when the flow is below the demand."""
    orbX, sizeX = GX.orbits()
    orbY, _ = GY.orbits()
    movT = T[sizeX[orbX[T]] > 1]
    Ds = sorted(set(orbX[movT].tolist()))
    demand = {d: int((orbX[movT] == d).sum()) for d in Ds}
    inD = np.isin(orbX, Ds)
    yo = orbY[p]                                  # Y-orbit of w(x), for every chamber x of X
    hx = np.flatnonzero(holes_x)
    supply = {}
    for o, n in zip(*np.unique(yo[hx], return_counts=True)):
        supply[int(o)] = int(n)
    cap = {}
    pairs, counts = np.unique(np.stack([yo[inD], orbX[inD]], 1), axis=0, return_counts=True)
    for (o, d), n in zip(pairs.tolist(), counts.tolist()):
        if supply.get(o):
            cap[(('O', o), ('D', d))] = int(n)
    for o in {e[0][1] for e in cap}:
        cap[('s', ('O', o))] = supply[o]
    for d in Ds:
        cap[(('D', d), 't')] = demand[d]
    need = sum(demand.values())
    value, flow = max_flow(cap, 's', 't')
    if value < need or not choose:
        return value, need, None
    hy = np.zeros(GX.NP, bool)
    hy[p[hx]] = True                              # V(Y) = w(V(X))
    inv = np.empty_like(p)
    inv[p] = np.arange(len(p))
    pT = np.zeros(GX.NP, bool)
    pT[p[T]] = True                               # w(T): compact, so few caps meet it
    chosen = []
    for (a, b), f in sorted(flow.items(), key=lambda e: str(e[0])):
        if f <= 0 or a == 's' or b == 't':
            continue
        o, d = a[1], b[1]
        ys = np.flatnonzero((orbY == o) & (orbX[inv] == d))
        ys = ys[np.lexsort((~hy[ys], ~pT[ys]))]   # chambers of w(T) first, then holes
        ys = ys[~np.isin(ys, chosen)]
        chosen.extend(ys[:f].tolist())
    TY = np.array(sorted(chosen), np.int64)
    assert len(TY) == need and len(np.unique(TY)) == need
    return value, need, TY


def certify_any(G, T):
    """A peeling certificate for an arbitrary target T at the geometry of G (kept in memory)."""
    G.orbits()
    fixed = T[G.orbit_size[G.orbit[T]] == 1]
    movable = T[G.orbit_size[G.orbit[T]] > 1]
    tr = B.Transport(G, movable)
    touch = int((~tr.avoidT).sum())
    # Two-letter conjugators cost (touching letters)^2 in time and memory; use them only for compact targets.
    try:
        chosen = tr.peel(maxg=1)
    except ValueError:
        if touch > 150:
            raise
        chosen = tr.peel(maxg=2)
    macros = [(w, G.perm(w), tr.support(G.perm(w))) for _, w, _ in chosen]
    free, protected, rows, extras = np.ones(len(G.letters), bool), [], [], []
    for j, (t, w, y) in enumerate(chosen):
        tree, extra = B.close_routes(tr, t, y, protected, free, macros[j:])
        metrics = B._route_metrics(G, t, protected, w, tree)
        rows.append({'t': t, 'y': y, **metrics})
        extras.append(extra)
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    return {'fixed': fixed, 'tr': tr, 'macros': macros, 'rows': rows, 'extras': extras, 'touching_letters': touch,
            'bound': sum(r['bound'] for r in rows), 'all_reached': all(not r['unreached'] for r in rows)}


def prepare_any(G, cert, holes):
    """Greedy clearing word for an in-memory certificate (as ``buffer.prepare``), from the holes only."""
    holes = holes.copy()
    if not holes[cert['fixed']].all():
        raise ValueError('a fixed target chamber is occupied')
    tr, macros, extras = cert['tr'], cert['macros'], cert['extras']
    protected, free, word = [], np.ones(len(G.letters), bool), []
    for j, row in enumerate(cert['rows']):
        t, y = row['t'], row['y']
        if not holes[t]:
            tree = B.weighted_tree(tr, y, free, macros[j:], extras[j])
            cand = holes & (tree[0] >= 0)
            cand[protected], cand[t] = False, False
            pts = np.flatnonzero(cand)
            if not len(pts):
                raise ValueError(f'no reached hole for target {t}')
            z = int(pts[np.argmin(tree[0][pts])])
            path = B.tree_path(tr, tree, macros[j:], z, extras[j])
            route = G.inverse(path)
            if G.image(route, z) != y:
                raise AssertionError('route does not reach its peeling point')
            step = route + macros[j][0]
            q = G.perm(step)
            nh = np.zeros(G.NP, bool)
            nh[q[holes]] = True
            holes = nh
            word.extend(step)
        if not holes[t] or not holes[protected].all():
            raise AssertionError('greedy step lost a cleared target')
        protected.append(t)
        free &= ~G.home[t, tr.capsof]
    return word, holes


def off_now(st):
    return np.flatnonzero(st._kpose()[st.pose_id] < 0)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--table', help='current-only pose table of the source state (default: E2 end)')
    ap.add_argument('--words', nargs='*', default=[], help='word files replayed first to reach the source state')
    ap.add_argument('--classify', action='store_true',
                    help='only the flow test, for every admissible block-moving letter, recomputing G(Y) each time')
    ap.add_argument('--cls', type=int, default=0)
    ap.add_argument('--cell', type=int, nargs='*', default=[369, 507])
    ap.add_argument('--letters', nargs='*', default=None, help='candidate letters grip,menu in order (default: all)')
    ap.add_argument('--suffix', help='word file replayed after the preparation')
    ap.add_argument('--out', required=True)
    a = ap.parse_args(argv)
    kp = CL.ctx.kplus
    kid = kp.index_of_matrix(B.to_tuple(B.identity(4)))
    members, cls = CL.classes()

    # ------------------------------------------------------------ search, reading only the source table
    R, M = O.start(a.table, a.words)
    st = R.st
    src = st.digest()
    GX = B.Alphabet(R, M)
    T = B.cell_chambers(GX.home, members, a.cls, a.cell)
    certX = B.certificate(GX, a.cls, a.cell)
    cond0 = B.condition(GX, T, certX)
    log(source=src[:16], necessary=cond0['necessary'], sufficient=cond0['sufficient'],
        deficits=cond0['necessary_deficits'], fixed_vacant=cond0['fixed_vacant'])
    if not cond0['fixed_vacant']:
        raise ValueError('a fixed target chamber is occupied at the source')
    off0 = off_now(st)
    holes_x = np.zeros(GX.NP, bool)
    holes_x[GX.holes] = True
    if a.letters is None:
        letters = []
        for c in range(600):
            cl = st.classify(c, need_inside=True, stop_at_straddle=False)
            ins = np.asarray(cl.inside, np.int64)
            if cl.straddle is None and np.isin(ins, off0).any():
                letters += [(c, i) for i in range(len(M.items))]
    else:
        letters = [tuple(int(x) for x in s.split(',')) for s in a.letters]
    if a.classify:
        rows = []
        for c, i in letters:
            k = kp.index_of_matrix(restore.element(M, c, i))
            if k < 0 or k == kid:
                continue
            cl = st.classify(c, need_inside=True, stop_at_straddle=False)
            moved = sorted(set(np.asarray(cl.inside, np.int64).tolist()) & set(off0.tolist()))
            p = letter_map(GX, c, k)
            R.apply(c, i)
            GY = B.Alphabet(R, M)
            value, need, _ = setup_target(GX, GY, p, T, holes_x, choose=False)
            R.undo()
            if st.digest() != src:
                raise AssertionError('undo did not return to the source')
            rows.append({'w': [c, i], 'moved_off_pieces': len(moved), 'safe_all_grips_at_y': len(GY.safe),
                         'flow': value, 'demand': need, 'passes': value >= need})
            log(**rows[-1])
        out = {'source_digest': src, 'target': {'class': a.cls, 'cell': sorted(a.cell), 'chambers': int(len(T))},
               'source_condition': {k_: v_ for k_, v_ in cond0.items() if k_ != 'by_orbit'},
               'letters': len(rows), 'passing': [r['w'] for r in rows if r['passes']], 'rows': rows,
               'evidence': 'exact; G(Y) recomputed on the simulator for every letter', 'seconds': round(time.time() - T0)}
        Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
        print(json.dumps({'letters': len(rows), 'passing': len(out['passing'])}))
        return 0
    tried, setup = [], None
    for c, i in letters:
        k = kp.index_of_matrix(restore.element(M, c, i))
        if k < 0 or k == kid:
            continue
        cl = st.classify(c, need_inside=True, stop_at_straddle=False)
        moved = sorted(set(np.asarray(cl.inside, np.int64).tolist()) & set(off0.tolist()))
        if cl.straddle is not None or not moved:
            raise ValueError(f'({c}, {i}) is not an admissible block-moving letter')
        kinv = int(kp.inv[k])
        iinv = [j for j in range(len(M.items)) if kp.index_of_matrix(restore.element(M, c, j)) == kinv][0]
        p = letter_map(GX, c, k)
        R.apply(c, i)
        GY = B.Alphabet(R, M)
        value, need, TY = setup_target(GX, GY, p, T, holes_x)
        entry = {'w': [c, i], 'flow': value, 'demand': need, 'moved_pieces': len(moved)}
        v = None
        if TY is not None:
            log(w=[c, i], flow=value, target_y_outside_wT=int((~np.isin(TY, p[T])).sum()),
                caps_meeting_target_y=int(GY.home[np.ix_(TY, np.asarray(GY.safe, np.int64))].any(axis=0).sum()))
            try:
                certY = certify_any(GY, TY)
                hy = np.zeros(GY.NP, bool)
                hy[GY.holes] = True
                vw, hy2 = prepare_any(GY, certY, hy)
                v = GY.pairs(vw)
                entry.update(target_y=len(TY), target_y_outside_wT=int((~np.isin(TY, p[T])).sum()),
                             touching_letters=certY['touching_letters'], certificate_bound=certY['bound'],
                             v_length=len(v))
            except ValueError as e:
                entry['failure'] = str(e)[:200]
        if v is not None:
            for cc, ii in v:
                R.apply(cc, ii)
            R.apply(c, iinv)
            G2 = B.Alphabet(R, M)
            cond2 = B.condition(G2, T, certX)
            entry.update(sufficient_after=cond2['sufficient'], deficits_after=cond2['necessary_deficits'])
            if cond2['sufficient']:
                prep = B.prepare(G2, certX)
                setup = {'w': [c, i], 'w_inverse': [c, iinv], 'moved_pieces': moved, 'v': v,
                         'preparation': prep['word'], 'predicted_holes': prep['predicted_holes']}
            for _ in range(len(v) + 1):
                R.undo()
        R.undo()
        if st.digest() != src:
            raise AssertionError('undo did not return to the source')
        tried.append(entry)
        log(**entry)
        if setup is not None:
            break
    if setup is None:
        out = {'source_digest': src, 'tried': tried, 'setup': None, 'seconds': round(time.time() - T0)}
        Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
        print(json.dumps({'setup': None, 'tried': len(tried)}))
        return 2

    # ------------------------------------------------------------ replay from the table with every check
    R, M = O.start(a.table, a.words)
    st = R.st
    if st.digest() != src:
        raise AssertionError('replay source differs')
    cache = {}
    mats0 = {int(q): B.matrix_json(st._pose_matrix(int(st.pose_id[q]))) for q in off0.tolist()}
    lat0 = np.ones(GX.NP, bool)
    lat0[off0] = False

    def same(pieces):
        return all(q in mats0 and B.matrix_json(st._pose_matrix(int(st.pose_id[q]))) == mats0[q] for q in pieces)

    others = [q for q in off0.tolist() if q not in set(setup['moved_pieces'])]
    seg = [setup['w']] + setup['v'] + [setup['w_inverse']]
    checked, digests = 0, {'source': src}
    for n, (c, i) in enumerate(seg):
        R.apply(int(c), int(i))
        checked += 1
        if not np.array_equal(off_now(st), off0):
            raise AssertionError(f'lattice flags changed at setup prefix {n + 1}')
        if not same(others):
            raise AssertionError(f'an off piece outside the moved blocks changed at setup prefix {n + 1}')
        if not B._depth_one(R, members, cls, cache):
            raise AssertionError(f'not depth one at setup prefix {n + 1}')
    if not same(off0.tolist()):
        raise AssertionError('off poses differ from the source after the setup')
    digests['after_setup'] = st.digest()
    for n, (c, i) in enumerate(setup['preparation']):
        R.apply(int(c), int(i))
        checked += 1
        if not np.array_equal(off_now(st), off0):
            raise AssertionError(f'lattice flags changed at preparation prefix {n + 1}')
        if not same(off0.tolist()):
            raise AssertionError(f'an off pose changed at preparation prefix {n + 1}')
        if not B._depth_one(R, members, cls, cache):
            raise AssertionError(f'not depth one at preparation prefix {n + 1}')
    if not same(off0.tolist()):
        raise AssertionError('off poses differ from the source after the preparation')
    holes_now = B.hole_set(st)
    if holes_now.tolist() != setup['predicted_holes']:
        raise AssertionError('predicted holes differ from the replay')
    if not np.isin(T, holes_now).all():
        raise AssertionError('the target is not vacant after the preparation')
    digests['after_preparation'] = st.digest()
    suffix_len = 0
    if a.suffix:
        sw = json.loads(Path(a.suffix).read_text())['word']
        for n, (c, i) in enumerate(sw):
            R.apply(int(c), int(i))
            checked += 1
            if not B._depth_one(R, members, cls, cache):
                raise AssertionError(f'not depth one at suffix prefix {n + 1}')
        suffix_len = len(sw)
    end = st.digest()
    off1 = off_now(st)
    K1 = st._kpose()[st.pose_id]
    out = {'source_digest': src, 'source_off': int(len(off0)), 'target': {'class': a.cls, 'cell': sorted(a.cell),
           'chambers': int(len(T))}, 'source_condition': {k_: v_ for k_, v_ in cond0.items() if k_ != 'by_orbit'},
           'tried': tried, 'w': setup['w'], 'w_inverse': setup['w_inverse'],
           'moved_off_pieces': len(setup['moved_pieces']), 'v_length': len(setup['v']),
           'preparation_length': len(setup['preparation']), 'suffix_length': suffix_len,
           'word': [list(map(int, x)) for x in seg + setup['preparation']],
           'digests': dict(digests, end=end), 'prefixes_checked': checked,
           'restored': int(len(set(off0.tolist()) - set(off1.tolist()))), 'off_at_end': int(len(off1)),
           'source_lattice_all_on_lattice': bool((K1[lat0] >= 0).all()),
           'remaining_off_subset_of_source': bool(set(off1.tolist()) <= set(off0.tolist())),
           'remaining_off_poses_equal_source': bool(same(off1.tolist())), 'seconds': round(time.time() - T0)}
    Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps({k_: v_ for k_, v_ in out.items() if k_ not in ('word', 'tried', 'source_condition')}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
