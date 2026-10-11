"""The five classes of grip poles under the I_a menu: classes, frames, cells and fragments, exactly.

Card 3 (cards/card-3-hidden-puzzles.md) and open-problems.md ("Five ordinary puzzles inside I_a")
cite these checks. Quaternions: x = x0 + x1 i + x2 j + x3 k; K+ is {x -> u x conj(w) : u, w in 2I}
(trees.py --check); n_c is the exact pole of grip c, and |n_c|^2 = 12 + 4 sqrt5 for every grip.
- classes: d is in the class of c when n_c^-1 n_d lies in 2I. The numbering is that of
  deep_scramble.inscribed();
- kplus: K+ maps classes onto classes; the induced permutations of the five classes, their parity
  and the stabiliser of a class;
- menu: at every grip c the I_a menu (restore.menu) equals {x -> (n_c u n_c^-1) x u^-1 : u in 2I};
- compatible: at every grip c and every pole d outside the class of c, some menu element maps n_d to
  a vector that is not a pole; at one grip of each class, every menu element maps the class onto
  itself;
- frames: for each class the left factors n_c u n_c^-1 (u in 2I) lie in five left cosets a 2I, one
  of them 2I itself (the lattice domain); the other four of each class are distinct across classes,
  and the left tree distance of each is recorded;
- cells: per class, the pieces grouped by which of the class's caps contain their home chamber
  (exact cap membership);
- states (with --states): at the deep scramble, the I_a@40 stop and the E2 end, the class of every
  off-lattice domain (a pose gamma lies in Sym+(P) K+ exactly when gamma maps some class onto P),
  the blocked grips by class, and per class with fragments: the cells its own pieces (lattice and its
  frames) occupy, for each fragment the cells it meets and the lattice pieces in them, frame pieces
  straddling one of the class's cuts (checked piece by piece at the blocked grips of the class; an
  unblocked grip has no straddling piece at all), and the grips that are not blocked and whose caps
  hold no piece of another class's frame. At the E2 end one probe twist (507, 13) of class 0 is then
  applied and undone: which fragments it puts on the lattice and how many lattice pieces it moves off.

Run from the repository root:
    python research/jumbling/theory/classes.py [--states] [--out <file>]
Writes results/classes-I_a.json by default; progress goes to stderr.

Evidence kind: source and exact synthetic geometry (quaternions over Q(sqrt5), exact cap membership,
exact simulator states). Timings are cloud timings of research code, not performance evidence.
"""
import argparse
import json
import sys
import time
from itertools import combinations
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import deep_scramble  # noqa: E402
import restore  # noqa: E402
import seams  # noqa: E402
import trees  # noqa: E402
from exact import Q5, matvec  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.model import NP  # noqa: E402

ctx = restore.ctx
data = ctx.data
N = [tuple(r) for r in data.N]
INV = Q5(1) / data.NN
UNITS = trees.UNITS
USET = set(UNITS)
E4 = [tuple(Q5(int(i == j)) for j in range(4)) for i in range(4)]
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def ninv(c):
    return tuple(x * INV for x in trees.qconj(N[c]))


def pole(v):
    return data.POLE.get(tuple(v), -1)


def lr_matrix(a, b):
    """Matrix of x -> a x conj(b) (rows indexed by output coordinate)."""
    bc = trees.qconj(b)
    cols = [trees.qmul(trees.qmul(a, e), bc) for e in E4]
    return tuple(tuple(cols[k][r] for k in range(4)) for r in range(4))


def classes():
    left, out, cls = set(range(600)), [], np.full(600, -1)
    while left:
        c = min(left)
        inv = ninv(c)
        mem = [d for d in range(600) if trees.qmul(inv, N[d]) in USET]
        if len(mem) != 120 or not set(mem) <= left:
            raise AssertionError(f'class of {c} has {len(mem)} poles or overlaps another class')
        cls[mem] = len(out)
        out.append(mem)
        left -= set(mem)
    return out, cls


def parity(p):
    return sum(1 for i, j in combinations(range(len(p)), 2) if p[i] > p[j]) % 2


def kplus_on_classes(members, cls):
    perms = np.asarray(ctx.kplus.perms, np.int64)
    img = cls[perms]                                     # class of the image of every pole
    rows = np.stack([img[:, m] for m in members], 1)     # elements x classes x 120
    consistent = bool((rows == rows[:, :, :1]).all())
    induced = [tuple(int(x) for x in r) for r in rows[:, :, 0]]
    distinct = sorted(set(induced))
    return {'elements': len(induced), 'maps_classes_onto_classes': consistent,
            'distinct_permutations': len(distinct), 'all_even': all(parity(p) == 0 for p in distinct),
            'stabiliser_orders': [sum(1 for p in induced if p[j] == j) for j in range(len(members))]}


def menus_and_compatible(M, members, cls):
    reps = [m[0] for m in members]
    menu_mismatch, no_witness, closed = [], 0, {}
    for c in range(600):
        inv = ninv(c)
        formula = {lr_matrix(trees.qmul(trees.qmul(N[c], u), inv), u) for u in trees.PUNITS}
        mats = [restore.element(M, c, i) for i in range(len(M.items))]
        if len(formula) != 60 or set(mats) != formula:
            menu_mismatch.append(c)
        for d in range(600):
            if cls[d] != cls[c] and all(pole(matvec(m, N[d])) >= 0 for m in mats):
                no_witness += 1
        if c in reps:
            imgs = [pole(matvec(m, N[d])) for m in mats for d in members[cls[c]]]
            closed[c] = min(imgs) >= 0 and all(cls[i] == cls[c] for i in imgs)
        if c % 100 == 99:
            log(menu_grips=c + 1)
    return ({'grips': 600, 'elements_per_grip': len(M.items), 'grips_where_menu_differs_from_formula': menu_mismatch},
            {'pairs_checked': 600 * 480, 'pairs_without_witness': no_witness,
             'menu_maps_class_onto_itself_at': {str(c): v for c, v in closed.items()}})


def frames(members):
    lattice = frozenset(UNITS)
    rows, nonlattice = [], {}
    for j, m in enumerate(members):
        c, inv = m[0], ninv(m[0])
        cos = {}
        for u in UNITS:
            a = trees.qmul(trees.qmul(N[c], u), inv)
            cos.setdefault(frozenset(trees.qmul(a, v) for v in UNITS), a)
        others = [(k, a) for k, a in cos.items() if k != lattice]
        for k, _ in others:
            nonlattice.setdefault(k, set()).add(j)
        rows.append({'class': j, 'grip': c, 'cosets': len(cos), 'contains_lattice': lattice in cos,
                     'left_tree_distance_of_non_lattice_cosets': sorted(trees.dist(a) for _, a in others)})
    return {'per_class': rows, 'non_lattice_cosets': len(nonlattice),
            'cosets_shared_by_two_classes': sum(1 for v in nonlattice.values() if len(v) > 1)}


def cells(members):
    off, cp = np.asarray(data.cap_offsets), np.asarray(data.cap_piece)
    out = []
    for j, m in enumerate(members):
        sig = np.zeros((NP, len(m)), bool)
        for k, c in enumerate(m):
            sig[cp[off[c]:off[c + 1]], k] = True
        keys, counts = np.unique(np.packbits(sig, axis=1), axis=0, return_counts=True)
        empty = int(counts[~keys.any(1)].sum())
        rest = counts[keys.any(1)]
        out.append({'class': j, 'cells': len(counts), 'pieces_below_every_cut': empty,
                    'other_cell_sizes': {str(s): int((rest == s).sum()) for s in sorted(set(rest.tolist()))}})
    return out


def hub(rep, members, cls):
    """Pairs (Q, P) of classes with rep(Q) = P."""
    out = []
    for q, m in enumerate(members):
        img = [pole(matvec(rep, N[d])) for d in m]
        if min(img) >= 0 and len({cls[d] for d in img}) == 1:
            out.append((q, int(cls[img[0]])))
    return out


def state(label, t, moves, members, cls, probes=()):
    st, M = seams._fixture_state('I_a', t)
    R = seams.SeamRestorer(st, M)
    for c, i in moves:
        R.apply(c, i)
    log(state=label, replayed=len(moves))
    D = R.domains()
    dom = np.asarray(D.dom)
    hubs = {d: hub(D.reps[d], members, cls) for d in range(1, len(D.reps))}
    dcls = {d: (h[0][1] if len(h) == 1 else None) for d, h in hubs.items()}
    blocked = [c for c in range(600) if R.inside(c) is None]
    bset = set(blocked)
    log(state=label, domains=len(D.reps), blocked=len(blocked))
    res = {'state': label, 'fixture_twists': t, 'moves': len(moves), 'off_lattice': int((dom != 0).sum()),
           'domains': [{'size': len(D.members[d]), 'class_pairs': hubs[d], 'class': dcls[d]} for d in range(1, len(D.reps))],
           'blocked_by_class': np.bincount(cls[blocked], minlength=5).tolist() if blocked else [0] * 5,
           'classes': {}}
    for P in sorted({v for v in dcls.values() if v is not None}):
        poles = members[P]
        frag_ids = [d for d, v in dcls.items() if v == P]
        own = (dom == 0) | np.isin(dom, frag_ids)
        other = ~own & (dom != 0)
        sig = np.zeros((NP, len(poles)), bool)
        avoid, straddling = 0, set()
        for k, c in enumerate(poles):
            cl = R.st.classify(int(c), need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                raise AssertionError(f'uncertain classification at grip {c}')
            ins = np.asarray(cl.inside, np.int64)
            sig[ins, k] = True
            if cl.straddle is None:
                avoid += int(not other[ins].any())
                continue
            evs = {}
            for p in np.flatnonzero(np.isin(dom, frag_ids)).tolist():
                pid = int(R.st.pose_id[p])
                if pid not in evs:
                    evs[pid] = Evaluator(pullback_form(R.st.pose(p), data.N[c]))
                if evs[pid].classify(ctx.regions.vform(p))[0] == 'straddle':
                    straddling.add(int(p))
        keyed = {}
        for p in np.flatnonzero(own).tolist():
            keyed.setdefault(sig[p].tobytes(), []).append(p)
        fr = []
        for d in frag_ids:
            F = D.members[d]
            fc = {sig[p].tobytes() for p in F.tolist()}
            fr.append({'size': len(F), 'cells': len(fc),
                       'cells_shared_with_lattice_pieces': sum(1 for b in fc if (dom[keyed[b]] == 0).any()),
                       'lattice_pieces_in_its_cells': sum(int((dom[keyed[b]] == 0).sum()) for b in fc)})
        res['classes'][str(P)] = {'cells_occupied_by_own_pieces': len(keyed), 'grips_blocked': sum(1 for c in poles if c in bset),
                                  'frame_pieces_straddling_a_cut_of_the_class': len(straddling),
                                  'grips_unblocked_and_free_of_other_classes': avoid, 'fragments': fr}
        log(state=label, cls=P, fragments=[f['size'] for f in fr])
    res['probes'] = []
    for c, i in probes:
        lat0 = R.st.lattice_flags()
        R.apply(c, i)
        lat1 = R.st.lattice_flags()
        res['probes'].append({'twist': [c, i], 'grip_class': int(cls[c]),
                              'fragments_put_on_lattice': [len(D.members[d]) for d in range(1, len(D.reps))
                                                           if lat1[D.members[d]].all()],
                              'lattice_pieces_moved_off': int((lat0 & ~lat1).sum()),
                              'off_lattice_after': int((~lat1).sum())})
        R.undo()
        if not (R.st.lattice_flags() == lat0).all():
            raise AssertionError('undo did not restore the state')
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--states', action='store_true')
    ap.add_argument('--out', default=str(HERE / 'results' / 'classes-I_a.json'))
    a = ap.parse_args()
    members, cls = classes()
    sets, _ = deep_scramble.inscribed()
    out = {'menu': 'I_a',
           'classes': {'sizes': [len(m) for m in members], 'members': members,
                       'same_as_deep_scramble_inscribed': [sorted(s['poles']) for s in sets] == [sorted(m) for m in members],
                       'class_of_grip': {str(c): int(cls[c]) for c in (0, 5, 30, 145, 154, 162, 237, 315)}}}
    log(classes=out['classes']['sizes'])
    out['kplus'] = kplus_on_classes(members, cls)
    log(kplus=out['kplus'])
    out['frames'] = frames(members)
    log(frames=out['frames']['non_lattice_cosets'])
    out['cells'] = cells(members)
    log(cells=[r['cells'] for r in out['cells']])
    out['menu_formula'], out['compatible'] = menus_and_compatible(restore.menu('I_a'), members, cls)
    log(menu=out['menu_formula']['grips_where_menu_differs_from_formula'], no_witness=out['compatible']['pairs_without_witness'])
    if a.states:
        R = HERE / 'results'
        stop = json.loads((R / 'seams-I_a-40-stop.json').read_text())['moves']
        e2 = json.loads((R / 'joint-attack-I_a-40.json').read_text())['e2_stop_no_history']['moves']
        deep = json.loads((R / 'deep-scramble-I_a.json').read_text())['scramble']['moves']
        out['states'] = [state('I_a@40 stop', 40, stop, members, cls),
                         state('E2 end (I_a@40 stop plus the E2 twists)', 40, stop + e2, members, cls, probes=[(507, 13)]),
                         state('deep scramble (seed 7)', 0, deep, members, cls)]
    out['seconds'] = round(time.time() - T0)
    Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
    print('wrote', Path(a.out).resolve().relative_to(HERE.parent.parent.parent))


if __name__ == '__main__':
    main()
