# E-B4, M1-18 with r = 1: for every unit that the landing order of order.py leaves at a state Z, the retained letters
# A (a non-identity K+ item at an admissible grip) after which A(U) is ready at Y = A(Z) in the sense of screen.py:
# every grip it needs is free, and no second frame domain of its class shares its cell (M1-19).
# A retained letter at grip g turns the whole content of cap g by one element E of K+, a symmetry of the 600-cell
# that permutes the 600 caps; a piece x inside cap g goes to E x, which lies inside, outside or across cap E(c)
# exactly when x lies so for c. So the cut labels at Y follow from those at Z and the cap permutation of E, without a
# new classification. A moved piece of frame domain R K+ lies in the coset E R K+ at Y, of class E(P) when R K+ has
# class P (E permutes the five classes as blocks; checked). A rigid unit lies on one side of every admissible cap,
# so it moves whole or not at all. Candidates: grips whose cap holds U, a unit that blocks one of U's needed grips,
# or (for an M1-19 unit) the other content of its cell.
# Two variants of the first stage:
#   at Z:  every off piece of Z counts (admissible grips, blockers, M1-19);
#   at Z': only the units left by the landing order count. A T1 closure keeps every other off piece exactly in
#          place, so if the landable units land as order.py's model says, the left units keep their labels at the
#          state Z' after them; the holes of Z' are not known without running the words, so this variant is
#          structural (no N1 or N2).
# Stage 2 (variant Z only): for the first passing letter of each unit the letter is applied, N1 and N2 are evaluated
# at Y with the SAFE-ALL alphabet of Y for A(U)'s home chambers (and, for a triangle or tetrahedron content, for the
# home cells of its kind and class), and the letter is undone. Lemma D is unchanged by A: A keeps every off piece
# off, and E maps home cells to home cells of the same kind and type vector. Nothing here constructs the closure C at
# Y; that is the next step.
# Usage (from the repository root):
#   python3 -I -B eb4.py <census record json> <screen json of that state> <out json> [--stage2 N]
import sys, json, time
from collections import defaultdict
from pathlib import Path
from types import SimpleNamespace
import numpy as np
REPO = Path.cwd()
TH = REPO / 'research/jumbling/theory'
SC = REPO / 'research/jumbling/restoration/12-low-height/scripts'
sys.path[:0] = [str(TH), str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures'), str(SC)]
import restore, sim, rim, buffer as B, blocks as BL, classes as CL
from exact import matmul, transpose
from sim.kplus import to_tuple
from screen import load

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def landing_order(U):
    """The units that order.py's landing model leaves (same model, same eligibility)."""
    def blk(u, c):
        g = u['grip_status'][str(c)]
        return set(g['straddled_by']) | set(g['foreign'])

    def elig(u):
        return (u.get('class') is not None and u.get('kind') not in (None, 'outer')
                and not u.get('other_domains_in_cell'))
    landed = set()
    while True:
        new = [k for k, u in U.items() if k not in landed and elig(u)
               and all(blk(u, c) <= landed for c in u['needed']['all'])
               and all(any(blk(u, c) <= landed for c in s) for s in u['needed']['one_of'])]
        if not new:
            return set(U) - landed
        landed |= set(new)


def main():
    rec, scr, outp = sys.argv[1], sys.argv[2], Path(sys.argv[3])
    a = sys.argv[4:]
    stage2 = int(a[a.index('--stage2') + 1]) if '--stage2' in a else 60
    st, M, src = load('record', rec)
    NP = len(st.pose_id)
    S = json.loads(Path(scr).read_text())
    assert S['digest'] == st.digest(), 'the screen is of another state'
    U = {u['unit']: u for u in S['units']}
    bl = BL.Blocks(SimpleNamespace(st=st), M)
    off, labels = bl.off, bl.labels
    row = {int(p): j for j, p in enumerate(off.tolist())}
    D = rim.Domains(st)
    dom = np.asarray(D.dom)
    members, cls = CL.classes()
    members = [np.asarray(m, np.int64) for m in members]
    cls = np.asarray(cls, np.int64)
    dcls = {}
    for d in range(1, len(D.reps)):
        h = CL.hub(D.reps[d], members, cls)
        dcls[d] = int(h[0][1]) if len(h) == 1 else -1
    rdom = dom[off]
    rcls = np.array([dcls[int(d)] for d in rdom], np.int64)
    units = []
    for b, pieces in enumerate(bl.blocks):
        pieces = np.asarray(pieces)
        for d in np.unique(dom[pieces]).tolist():
            units.append(np.array([row[int(p)] for p in pieces[dom[pieces] == d]], np.int64))
    assert len(units) == len(U) and all(len(units[k]) == U[k]['size'] for k in U), 'units differ from the screen'
    unit_of = np.full(len(off), -1, np.int64)
    for k, r in enumerate(units):
        unit_of[r] = k
    kp = CL.ctx.kplus
    perms, inv = np.asarray(kp.perms, np.int64), np.asarray(kp.inv, np.int64)
    kid = kp.index_of_perm(np.arange(600))
    inside = labels == 1
    straddle = labels == 2
    left = sorted(landing_order(U))
    log(off=len(off), units=len(units), left=len(left))

    ret_cache = {}

    def retained(g):
        if g not in ret_cache:
            items = []
            for i in range(len(M.items)):
                k = kp.index_of_matrix(restore.element(M, g, i))
                if k >= 0 and k != kid:
                    assert perms[k][g] == g
                    items.append((i, int(k)))
            assert len(items) == 11
            ret_cache[g] = items
        return ret_cache[g]

    cmap_cache = {}

    def cmap(e):
        """Class permutation of E (E permutes the five classes as blocks)."""
        if e not in cmap_cache:
            img = cls[perms[e][np.stack(members)]]
            assert (img == img[:, :1]).all(), 'E does not permute the classes'
            cmap_cache[e] = img[:, 0].copy()
        return cmap_cache[e]

    ykey_cache = {}

    def ykey(e, d):
        """The frame domain at Y of a moved piece of domain d: an existing domain j with E R_d K+ = R_j K+, or new."""
        if (e, d) not in ykey_cache:
            ER = matmul(kp.matrix(e), D.reps[d])
            hit = ('E', d)
            for j in range(1, len(D.reps)):
                if kp.index_of_matrix(to_tuple(matmul(transpose(D.reps[j]), ER))) >= 0:
                    hit = ('Z', j)
                    break
            ykey_cache[(e, d)] = hit
        return ykey_cache[(e, d)]

    def stage1(alive_units, tag):
        alive = np.zeros(len(off), bool)
        for m in alive_units:
            alive[units[m]] = True
        admissible = ~(straddle & alive[:, None]).any(axis=0)
        rows_out, passing = [], []
        for k in left:
            u = U[k]
            P = u['class']
            Ur = units[k]
            info = {'unit': k, 'kind': u['kind'], 'size': u['size'], 'class': P, 'cell': u['cell'],
                    'm1_19': bool(u.get('other_domains_in_cell'))}
            if P is None or u['kind'] in (None, 'outer') or u.get('own_straddle') or not u.get('one_cell'):
                info['skipped'] = 'no class, outer, own straddle or not one cell (a retained letter keeps these)'
                rows_out.append(info)
                continue

            def blockers_of(c, Pq, lab_col, ycl):
                rs = np.flatnonzero(alive & ((lab_col == 2) | ((lab_col == 1) & (ycl != Pq))))
                return {int(unit_of[r]) for r in rs}
            blockers = set()
            for c in u['needed']['all']:
                blockers |= blockers_of(c, P, labels[:, c], rcls)
            for s in u['needed']['one_of']:
                bs = [blockers_of(c, P, labels[:, c], rcls) for c in s]
                if all(bs):
                    blockers |= set().union(*bs)
            Pc = members[P]
            ins0 = labels[:, Pc] == 1                       # the cell key of screen.py: the caps a piece lies inside
            same0 = np.flatnonzero(alive & (ins0 == ins0[Ur[0]][None, :]).all(axis=1) & (rcls == P))
            others0 = {int(unit_of[r]) for r in same0 if rdom[r] != rdom[Ur[0]]}
            movers = {k} | blockers | others0
            cand = sorted(g for g in np.flatnonzero(admissible).tolist()
                          if any(inside[units[m], g].all() for m in movers))
            def test(g, i, e):
                """A(U) ready at Y and no M1-19 there, for the letter (g, i) of element e (g None: no letter)."""
                if g is None:
                    mv = np.zeros(len(off), bool)
                    e = kid
                else:
                    mv = inside[:, g] & alive
                mrows = np.flatnonzero(mv)
                moved_u = bool(mv[Ur].all())
                ei = int(inv[e])
                cm = cmap(e)
                Pn = int(cm[P]) if moved_u else P
                need_all = [int(perms[e][c]) if moved_u else c for c in u['needed']['all']]
                need_one = [[int(perms[e][c]) if moved_u else c for c in s] for s in u['needed']['one_of']]
                ycls = rcls.copy()
                okc = rcls[mrows] >= 0
                ycls[mrows[okc]] = cm[rcls[mrows[okc]]]

                def lab_y(c):
                    col = labels[:, c].copy()
                    col[mrows] = labels[mrows, int(perms[ei][c])]
                    return col

                def free(c):
                    return not blockers_of(c, Pn, lab_y(c), ycls)
                if not (all(free(c) for c in need_all) and all(any(free(c) for c in s) for s in need_one)):
                    return None
                Pcn = members[Pn]
                LY = labels[:, Pcn].copy()
                LY[mrows] = labels[np.ix_(mrows, perms[ei][Pcn])]
                IY = LY == 1
                same = np.flatnonzero(alive & (IY == IY[Ur[0]][None, :]).all(axis=1) & (ycls == Pn))
                ukey = ykey(e, int(rdom[Ur[0]])) if moved_u else ('Z', int(rdom[Ur[0]]))
                keys = {ykey(e, int(rdom[r])) if mv[r] else ('Z', int(rdom[r])) for r in same.tolist()}
                if keys - {ukey}:
                    return None
                return {'grip': None if g is None else int(g), 'item': None if g is None else int(i), 'element': e,
                        'moves_unit': moved_u, 'class_after': Pn,
                        'cell_after': sorted(int(perms[e][c]) for c in u['cell']) if moved_u else u['cell'],
                        'content_after': int(len(same))}
            info['ready_without_letter'] = test(None, None, None) is not None
            if tag == 'Z':                                  # consistency with the screen at Z itself
                assert bool(others0) == bool(u.get('other_domains_in_cell')), ('M1-19 differs', k)
                assert not info['ready_without_letter'], ('a left unit is ready at Z', k)
            tried, ok = 0, []
            for g in cand:
                for i, e in retained(g):
                    tried += 1
                    x = test(g, i, e)
                    if x is not None:
                        ok.append(x)
            info.update({'blockers': sorted(blockers), 'other_domains': sorted(others0), 'candidate_grips': len(cand),
                         'letters_tried': tried, 'letters_making_it_ready': len(ok),
                         'moving_the_unit': sum(x['moves_unit'] for x in ok), 'examples': ok[:8]})
            rows_out.append(info)
            passing += [(k, x) for x in ok]
            log(variant=tag, unit=k, kind=u['kind'], blockers=len(blockers), grips=len(cand), tried=tried,
                ready=len(ok))
        return {'admissible_grips': int(admissible.sum()),
                'units_made_ready_by_one_letter': sum(1 for r in rows_out if r.get('letters_making_it_ready')),
                'units_ready_without_letter': sum(1 for r in rows_out if r.get('ready_without_letter')),
                'skipped': sum(1 for r in rows_out if 'skipped' in r), 'units': rows_out}, passing

    resZ, passing = stage1(range(len(units)), 'Z')
    resZp, _ = stage1(left, "Z'")
    log(at_Z=resZ['units_made_ready_by_one_letter'], at_Zp=resZp['units_made_ready_by_one_letter'])
    # stage 2 (at Z): N1 and N2 at Y for one passing letter per unit (the first)
    picks, seen = [], set()
    for k, x in passing:
        if k not in seen:
            picks.append((k, x))
            seen.add(k)
    picks = picks[:stage2]
    st2 = []
    home = CL.data.sig_matrix(np.arange(NP)) if picks else None
    otype = np.asarray(CL.ctx.regions.orbit_of)
    off_t = np.bincount(otype[off], minlength=36)
    for k, x in picks:
        u = U[k]
        d0 = st.digest()
        o = st.apply(sim.Twist(restore.ctx, x['grip'], restore.element(M, x['grip'], x['item']), 'menu',
                               {'menu': M.name}))
        assert o.applied and not o.uncertain, 'letter not admissible'
        G = B.Alphabet(SimpleNamespace(st=st), M)
        G.orbits()
        Pn = x['class_after']
        Pcn = members[Pn]
        want = np.isin(Pcn, x['cell_after'])
        omega = np.flatnonzero((home[:, Pcn] == want[None, :]).all(axis=1))
        r = {'unit': k, 'kind': u['kind'], 'letter': [x['grip'], x['item']], 'moves_unit': x['moves_unit'],
             'safe_all_grips_at_Y': len(G.safe), 'home_chambers': int(len(omega))}
        if len(omega):
            cond = B.condition(G, omega)
            r.update({'N1': bool(cond['fixed_vacant']), 'N1_N2': bool(cond['necessary'])})
        if u['kind'] in ('triangle', 'tetrahedron'):
            nk = 3 if u['kind'] == 'triangle' else 4
            keys = np.packbits(home[:, Pcn], axis=1)
            cham = defaultdict(list)
            for xx, kk in enumerate(map(bytes, keys)):
                cham[kk].append(xx)
            n = nd = ndn = 0
            for kk, xs in cham.items():
                if int(np.unpackbits(np.frombuffer(kk, np.uint8)).sum()) != nk:
                    continue
                xs = np.asarray(xs, np.int64)
                n += 1
                if (np.bincount(otype[xs], minlength=36) <= off_t).all():
                    nd += 1
                    ndn += bool(B.condition(G, xs)['necessary'])
            r['transport_targets'] = [n, nd, ndn]
        st2.append(r)
        log(stage2=r)
        st.undo()
        assert st.digest() == d0
    doc = {'source': src, 'digest': st.digest(), 'left_units': len(left), 'at_Z': resZ, 'at_Z_prime': resZp,
           'stage2_at_Z': st2, 'seconds': round(time.time() - T0, 1)}
    outp.write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps({'left_units': len(left),
                      'at_Z': {k: v for k, v in resZ.items() if k != 'units'},
                      'at_Z_prime': {k: v for k, v in resZp.items() if k != 'units'},
                      'stage2_N1_N2': sum(1 for r in st2 if r.get('N1_N2')), 'stage2': len(st2),
                      'seconds': doc['seconds']}))


if __name__ == '__main__':
    main()
