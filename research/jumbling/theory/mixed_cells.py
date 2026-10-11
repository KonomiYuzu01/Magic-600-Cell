"""Mixed cells at the E2 end: where the fragments that share a cell with lattice pieces sit, and what other classes' cuts do there.

At the E2 end of I_a@40 four of the six fragments share one cell of their class with lattice pieces (card 3, D1).
This script reads, on the exact simulator:
- menus: for every grip d and every class P other than d's, the elements of d's menu that map the poles of P onto
  poles of P, and how many of them are retained (in K+);
- outline: how many pieces of each fragment lie inside the outline P~ (every facet inequality n.x <= |n|^2 holds
  on the piece); lattice pieces are chambers, so they always do;
- shared cells: for each fragment, the cells of its class that also hold lattice pieces, with the number of fragment
  pieces, lattice pieces and vacant chambers in the cell (the cell's chambers at the solved state minus the lattice
  pieces now in it; distinct lattice pieces occupy distinct chambers);
- separating cuts: the grips of other classes whose cut has every fragment piece of a shared cell on one side and
  every lattice piece of it on the other (no piece of the cell straddles), whether they are admissible, and how
  many chambers of their cap are vacant;
- retained twists of a separating grip: after each non-identity retained element (applied and undone), the shared
  cells and the off-lattice count;
- cell types (class 1, the cell model of e3_class.py): for every vertex, edge, triangle and tetrahedron cell, the size
  of the orbit of label 0 among its five labels under the free twists and how many chambers it holds (0 when it
  does not meet P~); and whether the poles of every reachable cell are pairwise nearest neighbours in the class (so
  the cells are the faces of its 600-cell).

Run from the repository root:
    python research/jumbling/theory/mixed_cells.py            (about 2 min)
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import classes as CL  # noqa: E402
import e3_class as E3  # noqa: E402
import restore  # noqa: E402
import trees  # noqa: E402
from exact import Q5, matvec  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402

OUT = HERE / 'results' / 'mixed-cells-I_a-40.json'
ALPHA = Q5(121) / Q5(125)
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def menus(M, members, cls):
    """(|Lambda_d cap Sym+(P)|, retained) over the 600 x 4 pairs (d, P) with P not d's class."""
    sets = [set(m) for m in members]
    tally = {}
    for d in range(600):
        rows = []
        for i in range(len(M.items)):
            g = restore.element(M, d, i)
            retained = all(E3.prop_unit(q) for q in trees.factors(g))
            keeps = [all(CL.pole(matvec(g, CL.N[x])) in sets[P] for x in members[P]) for P in range(5)]
            rows.append((retained, keeps))
        for P in range(5):
            if P != cls[d]:
                key = (sum(1 for _, k in rows if k[P]), sum(1 for r, k in rows if k[P] and r))
                tally[key] = tally.get(key, 0) + 1
        if d % 100 == 99:
            log(menus=d + 1)
    return [{'elements_mapping_P_onto_P': a, 'of_them_retained': b, 'pairs': n} for (a, b), n in sorted(tally.items())]


class Reader:
    def __init__(self, R, members, cls):
        self.R, self.members, self.cls = R, members, cls
        off, cp = np.asarray(CL.data.cap_offsets), np.asarray(CL.data.cap_piece)
        self.home = [set(cp[off[c]:off[c + 1]].tolist()) for c in range(600)]

    def read(self):
        R, members, cls = self.R, self.members, self.cls
        D = R.domains()
        dom = np.asarray(D.dom)
        dcls = {}
        for d in range(1, len(D.reps)):
            h = CL.hub(D.reps[d], members, cls)
            dcls[d] = h[0][1] if len(h) == 1 else None
        inside, admissible = {}, {}
        for c in range(600):
            cl = R.st.classify(int(c), need_inside=True, stop_at_straddle=False)
            if cl.uncertain:
                raise AssertionError(f'uncertain classification at grip {c}')
            inside[c] = set(np.asarray(cl.inside, np.int64).tolist())
            admissible[c] = cl.straddle is None
        lat = set(np.flatnonzero(dom == 0).tolist())
        frags = []
        for d, P in dcls.items():
            F = D.members[d].tolist()
            key = lambda p, w=inside, P=P: tuple(int(p in w[c]) for c in members[P])
            byc = {}
            for p in F:
                byc.setdefault(key(p), []).append(p)
            shared = []
            for s, Fp in byc.items():
                caps = [c for c, b in zip(members[P], s) if b]
                L, H = set(lat), set(range(len(dom)))
                for c in members[P]:
                    if c in caps:
                        L &= inside[c]
                        H &= self.home[c]
                    else:
                        L -= inside[c]
                        H -= self.home[c]
                if L:
                    shared.append({'cell': caps, 'fragment_pieces': len(Fp), 'lattice_pieces': len(L),
                                   'chambers': len(H), 'vacant_chambers': len(H) - len(L), '_F': Fp, '_L': sorted(L)})
            frags.append({'domain': d, 'pieces': len(F), 'class': P, 'cells': len(byc), 'shared': shared})
        return {'dom': dom, 'dcls': dcls, 'D': D, 'inside': inside, 'admissible': admissible, 'lat': lat,
                'fragments': frags, 'off_lattice': int((dom != 0).sum())}

    def vacant_in_cap(self, st, c):
        return len(self.home[c]) - len(st['inside'][c] & st['lat'])


def public(frags):
    return [{k: v for k, v in f.items() if k != 'shared'} | {'shared': [{k: v for k, v in s.items() if not k.startswith('_')}
                                                                         for s in f['shared']]} for f in frags]


def outline(R, st):
    data, ctx = CL.data, CL.ctx
    ev, out = {}, []
    for f in st['fragments']:
        inside_all = 0
        for p in st['D'].members[f['domain']].tolist():
            pid = int(R.st.pose_id[p])
            ok = True
            for e in range(600):
                if (pid, e) not in ev:
                    ev[(pid, e)] = Evaluator(pullback_form(R.st.pose(p), tuple(ALPHA * x for x in data.N[e])))
                if ev[(pid, e)].classify(ctx.regions.vform(p))[0] != 'out':
                    ok = False
                    break
            inside_all += ok
        out.append({'pieces': f['pieces'], 'class': f['class'], 'pieces_inside_the_outline': inside_all})
    return out


def separating(R, st, members, cls):
    data, ctx = CL.data, CL.ctx
    out = []
    for f in st['fragments']:
        P = f['class']
        for s in f['shared']:
            rows = []
            for c in range(600):
                if cls[c] == P:
                    continue
                ins = st['inside'][c]
                fin = sum(1 for p in s['_F'] if p in ins)
                lin = sum(1 for p in s['_L'] if p in ins)
                if not ((fin == len(s['_F']) and lin == 0) or (fin == 0 and lin == len(s['_L']))):
                    continue
                strad = 0
                for p in s['_F']:
                    if p not in ins:
                        e = Evaluator(pullback_form(R.st.pose(p), data.N[c]))
                        strad += e.classify(ctx.regions.vform(p))[0] == 'straddle'
                if not strad:
                    rows.append({'grip': c, 'class': int(cls[c]), 'admissible': st['admissible'][c],
                                 'side_of_the_cap': 'fragment' if fin else 'lattice'})
            out.append({'fragment': f['pieces'], 'class': P, 'cell': s['cell'], 'separating_cuts': rows})
    return out


def cell_types(R, M, members, cls):
    m = E3.Model(R, M, members, cls)
    comp = E3.orbits(m)
    off, cp = np.asarray(CL.data.cap_offsets), np.asarray(CL.data.cap_piece)
    sig = np.zeros((CL.NP, 120), bool)
    for k, c in enumerate(m.poles):
        sig[cp[off[c]:off[c + 1]], k] = True
    home = {}
    for r in sig:
        key = int.from_bytes(np.packbits(r).tobytes(), 'big')
        home[key] = home.get(key, 0) + 1
    names = {1: 'vertex', 2: 'edge', 3: 'triangle', 4: 'tetrahedron'}
    tab = {}
    for n, s in enumerate(m.cells):
        t = int(m.ncaps[n])
        if t not in names:
            continue
        size0 = sum(1 for k in range(5) if comp[n * 5 + k] == comp[n * 5])
        key = (names[t], size0, home.get(s, 0))
        tab[key] = tab.get(key, 0) + 1
    def dot(a, b):
        return sum((CL.N[a][k] * CL.N[b][k] for k in range(4)), Q5(0))
    dots = [dot(m.poles[0], c) for c in m.poles[1:]]
    near = dots[0]
    for x in dots[1:]:
        if near < x:                                     # exact ordering in Q(sqrt 5)
            near = x
    if sum(1 for x in dots if x == near) != 12:
        raise AssertionError('a pole of the class does not have 12 nearest neighbours')
    faces = all(dot(a, b) == near for s in m.cells
                for caps in [[m.poles[k] for k in range(120) if s >> (119 - k) & 1]]
                for i, a in enumerate(caps) for b in caps[i + 1:])
    return {'reachable_cells': len(m.cells), 'poles_pairwise_nearest_neighbours': faces,
            'table': [{'cell_type': a, 'label_0_orbit_size': b, 'chambers': c, 'cells': n} for (a, b, c), n in sorted(tab.items())]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--out', default=str(OUT))
    a = ap.parse_args()
    members, cls = CL.classes()
    R, M = E3.e2_end()
    rd = Reader(R, members, cls)
    st = rd.read()
    log(read='E2 end', off=st['off_lattice'])
    out = {'state': 'E2 end of I_a@40 (seams-I_a-40-stop.json moves, then the E2 twists)',
           'off_lattice': st['off_lattice'],
           'vacant_chambers': len(st['dom']) - len(st['lat']),
           'blocked_grips_by_class': np.bincount(cls[[c for c in range(600) if not st['admissible'][c]]], minlength=5).tolist(),
           'fragments': public(st['fragments'])}
    out['outline'] = outline(R, st)
    log(outline=out['outline'])
    out['separating'] = sep = separating(R, st, members, cls)
    log(separating=sep)
    trials = []
    for row in sep:
        for g in row['separating_cuts']:
            c = g['grip']
            g['vacant_chambers_in_its_cap'] = rd.vacant_in_cap(st, c)
            if not g['admissible']:
                continue
            for i in range(1, len(M.items)):
                if not all(E3.prop_unit(q) for q in trees.factors(restore.element(M, c, i))):
                    continue
                R.apply(c, i)
                after = rd.read()
                R.undo()
                trials.append({'twist': [c, i], 'off_lattice_after': after['off_lattice'],
                               'shared_after': [[s['cell'], s['fragment_pieces'], s['lattice_pieces']]
                                                for f in after['fragments'] for s in f['shared']]})
            log(trials=len(trials))
    out['retained_twists_of_separating_grips'] = trials
    out['shared_before'] = [[s['cell'], s['fragment_pieces'], s['lattice_pieces']] for f in st['fragments'] for s in f['shared']]
    out['cell_types_class_1'] = cell_types(R, M, members, cls)
    log(cell_types=out['cell_types_class_1'])
    out['menus'] = menus(M, members, cls)
    out['seconds'] = round(time.time() - T0)
    Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
    print('wrote', a.out)


if __name__ == '__main__':
    main()
