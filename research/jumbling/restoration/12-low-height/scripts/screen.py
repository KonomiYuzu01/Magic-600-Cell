# E-B1, the structural screen of a low-height state (option B). For every unit (a rigid block, or one frame part of
# a multi-frame block, PROOFS-U section 2): its frame class P and its class-P cell (LC-01: the sign vector over the
# 120 caps of P; LC-14: a rigid block in one frame lies in one cell), the cell kind, and whether the unit is the whole
# content of its cell (content = the off pieces with the same cell and the same domain); the other frame domains of
# class P in that cell (M1-19); for every grip that the T1 cell words need, whether it is free (LC-01: admissible,
# and its cap holds only lattice pieces and pieces of P's frames) or blocked by a straddler or by a foreign-frame
# occupant, and by which units; Lemma D's per-type counts of the cell's home chambers against the off pieces (every
# T1 word for the unit turns the whole cell, so its Bad set contains them) and N1 and N2 for vacating them with
# SAFE-ALL letters (buffer.condition), when the cell has home chambers; for a triangle or tetrahedron content, which
# lands only by a transport into a home cell of its kind, how many such cells of its class pass the same tests; and
# for every unit the units that block a grip it needs (order.py takes the
# landing order from there). Needed grips: an edge cell {a, b} needs a and b and one of their five common
# neighbours (Theorem C); a vertex cell {a} needs a (LC-12, whose bound also assumes all 120 class-P grips free);
# a triangle or tetrahedron cell needs at least one of its poles to move at all (LC-13 transport, whose bound also
# assumes all 120 free); an outer cell cannot be moved by its own class. Exact geometry; no twist is applied.
# Run from the repository root:
#   python3 -I -B screen.py record <census record json> <out json>
#   python3 -I -B screen.py table <pose table json or json.gz> <out json>
import sys, json, gzip, time
from collections import Counter, defaultdict
from pathlib import Path
from types import SimpleNamespace
import numpy as np
REPO = Path.cwd()
TH = REPO / 'research/jumbling/theory'
sys.path[:0] = [str(TH), str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures')]
import restore, sim, rim, buffer as B, blocks as BL, classes as CL, orient as O, trees as T

T0 = time.time()
KIND = {0: 'outer', 1: 'vertex', 2: 'edge', 3: 'triangle', 4: 'tetrahedron'}
HOME = {'outer': 5280, 'vertex': 385, 'edge': 86, 'triangle': 111, 'tetrahedron': 87}


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def load(mode, path):
    M = restore.menu('I_a')
    if mode == 'record':
        r = json.loads(Path(path).read_text())
        st, hist = restore.scramble(M, r['k'], r['seed'])
        assert [list(h) for h in hist] == [list(h) for h in r['history']], 'scramble differs from the record'
        for c, i in r['moves']:
            o = st.apply(sim.Twist(restore.ctx, int(c), restore.element(M, c, i), 'menu', {'menu': M.name}))
            assert o.applied and not o.uncertain, ('move not admissible', c, i)
        assert int(st.off_lattice_count()) == r['off_now'], 'off count differs from the record'
        src = {'record': Path(path).name, 'run': r['run'], 'moves': len(r['moves'])}
    else:
        raw = Path(path).read_bytes()
        doc = json.loads(gzip.decompress(raw) if path.endswith('.gz') else raw)
        st = sim.State(CL.ctx, menu=M)
        O.set_pose_table(st, doc)                  # checks the table's digest
        src = {'table': Path(path).name}
    return st, M, src


def main():
    mode, path, out_path = sys.argv[1], sys.argv[2], Path(sys.argv[3])
    st, M, src = load(mode, path)
    R = SimpleNamespace(st=st)
    NP = len(st.pose_id)
    log(loaded=src, off=int(st.off_lattice_count()))
    bl = BL.Blocks(R, M)
    off, labels = bl.off, bl.labels
    row = {int(p): j for j, p in enumerate(off.tolist())}
    D = rim.Domains(st)
    dom = np.asarray(D.dom)
    members, cls = CL.classes()
    dcls = {}
    for d in range(1, len(D.reps)):
        h = CL.hub(D.reps[d], members, cls)
        dcls[d] = int(h[0][1]) if len(h) == 1 else None
    # left heights of the off pieces
    hp = {}
    for pid in np.unique(st.pose_id[off]).tolist():
        hp[pid] = int(T.dist(T.factors(st._pose_matrix(pid))[0]))
    height = {int(p): hp[int(st.pose_id[p])] for p in off.tolist()}
    # units: blocks split by frame domain
    units = []
    for b, pieces in enumerate(bl.blocks):
        pieces = np.asarray(pieces)
        for d in np.unique(dom[pieces]).tolist():
            units.append({'block': b, 'domain': int(d), 'pieces': pieces[dom[pieces] == d]})
    unit_of = np.full(NP, -1, np.int64)
    for u, U in enumerate(units):
        unit_of[U['pieces']] = u
    movers = [[int(c) for c in bl.I[b] if c not in set(bl.straddling_grips.tolist())] for b in range(len(bl.blocks))]
    log(units=len(units), blocks=len(bl.blocks), domains=len(D.reps) - 1)
    # types and holes (Fact 1': holes of type t = off pieces of type t)
    otype = np.asarray(CL.ctx.regions.orbit_of)
    holes = B.hole_set(st)
    off_t = np.bincount(otype[off], minlength=36)
    assert np.array_equal(np.bincount(otype[holes], minlength=36), off_t), 'Fact 1 fails'
    is_hole = np.zeros(NP, bool); is_hole[holes] = True
    # the SAFE-ALL alphabet and its chamber orbits, for N1 and N2
    G = B.Alphabet(R, M)
    G.orbits()
    log(safe_all=len(G.safe), letters=len(G.letters))
    home = CL.data.sig_matrix(np.arange(NP))
    per_class = {}
    for P in range(5):
        poles = [int(c) for c in members[P]]
        N0i = CL.ninv(poles[0])
        w = [O.UIDX[tuple(T.qmul(N0i, CL.N[c]))] for c in poles]
        nbr = {i for i, u in enumerate(O.U) if u[0] == O.HALF_PHI}
        adj = {poles[k]: {poles[m] for m in range(120) if int(O.MUL[O.INV[w[k]], w[m]]) in nbr} for k in range(120)}
        assert all(len(v) == 12 for v in adj.values())
        edges = {frozenset((a, b)) for a in poles for b in adj[a]}
        tris = {frozenset((a, b, c)) for e in edges for a, b in [tuple(e)] for c in adj[a] & adj[b]}
        tets = {t | {d} for t in tris for d in set.intersection(*(adj[x] for x in t))}
        assert (len(edges), len(tris), len(tets)) == (720, 1200, 600)
        keys = np.packbits(home[:, poles], axis=1)
        cham = defaultdict(list)
        for x, k in enumerate(map(bytes, keys)):
            cham[k].append(x)
        # the class's triangle and tetrahedron home cells (cells with chambers), the landing cells of a transport:
        # Lemma D and N1/N2 for each
        targets = {3: [], 4: []}
        for k, xs in cham.items():
            nk = int(np.unpackbits(np.frombuffer(k, np.uint8)).sum())
            if nk in targets:
                xs = np.asarray(xs, np.int64)
                targets[nk].append((bool((np.bincount(otype[xs], minlength=36) <= off_t).all()),
                                    bool(B.condition(G, xs)['necessary'])))
        # grip status: straddling units, foreign-frame occupants (pieces whose domain is not a frame of class P)
        grips = {}
        for c in poles:
            sr = np.flatnonzero(labels[:, c] == 2)
            ir = np.flatnonzero(labels[:, c] == 1)
            foreign = [r for r in ir.tolist() if dcls.get(int(dom[off[r]])) != P]
            grips[c] = {'straddlers': sorted({int(unit_of[off[r]]) for r in sr.tolist()}),
                        'foreign': sorted({int(unit_of[off[r]]) for r in foreign}),
                        'n_straddling_pieces': len(sr), 'n_foreign_pieces': len(foreign)}
            grips[c]['free'] = not grips[c]['straddlers'] and not grips[c]['foreign']
        per_class[P] = {'poles': poles, 'adj': adj, 'faces': edges | tris | tets, 'cham': cham, 'grips': grips,
                        'n_free': sum(g['free'] for g in grips.values()),
                        'targets': {n: [len(v), sum(a for a, _ in v), sum(a and b for a, b in v)] for n, v in targets.items()}}
        log(cls=P, free_grips=per_class[P]['n_free'])
    # contents: (class, cell key) -> domain -> pieces
    content = defaultdict(lambda: defaultdict(list))
    for p in off.tolist():
        P = dcls.get(int(dom[p]))
        if P is None:
            continue
        lab = labels[row[p], per_class[P]['poles']]
        content[(P, bytes(np.packbits(lab == 1)))][int(dom[p])].append(p)
    rows, gedges = [], set()
    for u, U in enumerate(units):
        pcs, d = U['pieces'], U['domain']
        P = dcls.get(d)
        r = {'unit': u, 'block': U['block'], 'first': int(pcs.min()), 'size': len(pcs), 'domain': d, 'class': P,
             'heights': dict(Counter(height[int(p)] for p in pcs.tolist())), 'block_size': len(bl.blocks[U['block']]),
             'block_movers': len(movers[U['block']]), 'frames_in_block': len({int(x) for x in dom[bl.blocks[U['block']]]})}
        r['heights'] = {str(k): r['heights'][k] for k in sorted(r['heights'])}
        if P is None:
            r['cell'] = 'no frame of one class'
            rows.append(r); continue
        C = per_class[P]
        L = labels[np.searchsorted(off, pcs)][:, C['poles']]
        r['own_straddle'] = bool((L == 2).any())
        r['one_cell'] = bool((L == L[0]).all())          # LC-14
        pos = frozenset(C['poles'][k] for k in np.flatnonzero(L[0] == 1).tolist())
        kind = KIND.get(len(pos), 'more than four caps')
        r['cell'], r['kind'] = sorted(pos), kind
        r['is_face'] = len(pos) == 0 or (len(pos) == 1) or pos in C['faces']
        key = bytes(np.packbits(L[0] == 1))
        cont = content[(P, key)]
        r['content_size'] = len(cont[d])
        r['whole'] = len(cont[d]) == len(pcs)
        r['other_domains_in_cell'] = {str(e): len(v) for e, v in cont.items() if e != d}   # M1-19 when nonempty
        # needed grips
        conj, disj = [], []
        if kind == 'edge':
            a, b = sorted(pos)
            conj, disj = [a, b], [sorted(C['adj'][a] & C['adj'][b])]
        elif kind == 'vertex':
            conj = sorted(pos)
        elif kind in ('triangle', 'tetrahedron'):
            disj = [sorted(pos)]
        g = C['grips']
        r['needed'] = {'all': conj, 'one_of': disj}
        r['grip_status'] = {str(c): {'free': g[c]['free'], 'straddled_by': g[c]['straddlers'],
                                     'foreign': g[c]['foreign']} for c in sorted(set(conj) | set(sum(disj, [])))}
        r['all_class_grips_free'] = C['n_free'] == 120
        ready = kind != 'outer' and all(g[c]['free'] for c in conj) and all(any(g[c]['free'] for c in s) for s in disj)
        r['ready'] = bool(ready and r['one_cell'] and not r['own_straddle'] and not r['other_domains_in_cell'])
        for c in conj:
            for A in g[c]['straddlers'] + g[c]['foreign']:
                gedges.add((A, u))
        for s in disj:
            if not any(g[c]['free'] for c in s):
                for c in s:
                    for A in g[c]['straddlers'] + g[c]['foreign']:
                        gedges.add((A, u))
        # Lemma D and N1/N2 for the home chambers of the unit's cell, when it has any; a triangle or tetrahedron
        # content is landed by a transport into a home cell of its kind, whose chambers are then in Bad(W) too
        omega = np.asarray(C['cham'].get(key, []), np.int64)
        r['home_chambers'] = len(omega)
        r['home_cell'] = len(omega) > 0
        if len(omega):
            r['home_size_ok'] = HOME.get(kind) == len(omega)
            om_t = np.bincount(otype[omega], minlength=36)
            r['lemma_D'] = bool((om_t <= off_t).all())
            r['lemma_D_deficits'] = {str(t): [int(om_t[t]), int(off_t[t])] for t in np.flatnonzero(om_t > off_t).tolist()}
            r['home_occupied'] = int((~is_hole[omega]).sum())
            cond = B.condition(G, omega)
            r['N1'] = bool(cond['fixed_vacant'])
            r['N1_N2'] = bool(cond['necessary'])
            r['N_deficits'] = len(cond['necessary_deficits'])
            r['N_fixed_occupied'] = len(cond['fixed_occupied'])
        own = not len(omega) or (r['lemma_D'] and r['N1_N2'])
        if kind in ('triangle', 'tetrahedron'):
            r['targets'], r['targets_D'], r['targets_D_N'] = C['targets'][len(pos)]
            r['T1_necessary'] = bool(own and r['targets_D_N'] > 0)
        else:
            r['T1_necessary'] = bool(kind != 'outer' and len(omega) and own)
        rows.append(r)
    blocked_by = defaultdict(set)
    for a, b in gedges:
        blocked_by[b].add(a)
    ready = {r['unit'] for r in rows if r.get('ready')}
    n = len(rows)
    summ = {
        'units': n, 'pieces': int(sum(r['size'] for r in rows)),
        'by_class': dict(Counter(str(r['class']) for r in rows)),
        'by_kind': dict(Counter(r.get('kind', r['cell']) for r in rows)),
        'pieces_by_kind': {k: int(sum(r['size'] for r in rows if r.get('kind', r['cell']) == k))
                           for k in sorted({r.get('kind', r['cell']) for r in rows})},
        'heights': dict(Counter(h for r in rows for h, c in r['heights'].items() for _ in range(c))),
        'one_cell': sum(bool(r.get('one_cell')) for r in rows), 'own_straddle': sum(bool(r.get('own_straddle')) for r in rows),
        'not_a_face': sum(r.get('is_face') is False for r in rows),
        'whole_content': sum(bool(r.get('whole')) for r in rows),
        'm1_19_cells': sum(bool(r.get('other_domains_in_cell')) for r in rows),
        'needed_grips_free': sum(bool(r.get('ready')) for r in rows),
        'in_a_home_cell': sum(bool(r.get('home_cell')) for r in rows),
        'home_cell_lemma_D': sum(bool(r.get('lemma_D')) for r in rows),
        'home_cell_N1': sum(bool(r.get('N1')) for r in rows), 'home_cell_N1_N2': sum(bool(r.get('N1_N2')) for r in rows),
        'home_cell_vacant': sum(r.get('home_occupied') == 0 for r in rows),
        'transport_units': sum(1 for r in rows if 'targets' in r),
        'transport_units_with_a_target': sum(1 for r in rows if r.get('targets_D_N')),
        'T1_necessary': sum(bool(r.get('T1_necessary')) for r in rows),
        'ready_and_T1_necessary': sum(bool(r.get('ready') and r.get('T1_necessary')) for r in rows),
        'pieces_ready_and_T1_necessary': int(sum(r['size'] for r in rows if r.get('ready') and r.get('T1_necessary'))),
        'home_cells_by_class': {str(P): {KIND[n]: dict(zip(('cells', 'lemma_D', 'lemma_D_N1_N2'), v))
                                         for n, v in per_class[P]['targets'].items()} for P in range(5)},
        'frozen_blocks': sum(1 for m in movers if not m),
        'free_class_grips': {str(P): per_class[P]['n_free'] for P in range(5)},
        'blocking_edges': len(gedges),
        'blocked_units': sum(1 for r in rows if r['unit'] in blocked_by),
        'blocked_only_by_ready_units': sum(1 for b, s in blocked_by.items() if s <= ready),
    }
    summ['heights'] = {str(k): summ['heights'][k] for k in sorted(summ['heights'])}
    doc = {'source': src, 'digest': st.digest(), 'off': len(off), 'holes_by_type': {str(t): int(off_t[t]) for t in range(36) if off_t[t]},
           'summary': summ, 'units': [{k: v for k, v in r.items()} for r in rows],
           'seconds': round(time.time() - T0, 1)}
    out_path.write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps({'source': src, 'off': len(off), **{k: summ[k] for k in ('units', 'by_kind', 'whole_content', 'm1_19_cells', 'needed_grips_free', 'in_a_home_cell', 'home_cell_lemma_D', 'home_cell_N1_N2', 'transport_units_with_a_target', 'T1_necessary', 'ready_and_T1_necessary')}, 'seconds': doc['seconds']}))


if __name__ == '__main__':
    main()
