# Round 22, line (a): the transport census with whole cells as owners (R22A-02), for every unit of a state, in the
# class-P cell model of cellpure5.py (exact; headless; research only).
# - Owners: a lattice content owns its point (H, 0); an off content of class P owns every point of its cell (a word
#   keeps its exact pose only if it fixes all five labels of its cell, LC-03); a cell with an off piece of another
#   class, or with two off contents, is owned by nobody that can be excluded.
# - A power A^L of a single-letter commutator A = [x, y] at free class-P grips is clean for a set E of owners when L
#   is the lcm of the lengths of A's cycles that meet a point owned outside E.
# - Base powers (E empty) move free points only; unit powers (E = {u}) move the unit's cell; entry powers
#   (E = {h}, h the lattice content of a triangle or tetrahedron home cell) move h's point and free points only.
# Per unit: whether its cell holds another owner (glued); the component of its point under base and unit powers; the
# vacant home label-0 points of its kind there (zero-Bad targets) with a shortest path of powers and its exact replay;
# and the home cells of its kind that an entry power reaches from the component, by the size of their lattice content
# (the Bad of that transport), with the best such word and its exact replay (the unit lands, exactly the lattice
# pieces of that cell go off, and every other off piece keeps its exact pose). Model2 is the class of cellpure5.py,
# whose module runs its search on import, so its source lines are executed here.
#   python3 -I -B transport_census.py <table> <out json> <unit json>... [--max-depth D] [--pairs-near-units]
#       [--max-pairs N]
# Run from the repository root. --max-depth bounds the number of powers on a path (default 10).
import json
import math
import sys
import time
from collections import Counter, defaultdict, deque
from pathlib import Path

import numpy as np

REPO = Path.cwd()
TH = REPO / 'research/jumbling/theory'
sys.path[:0] = [str(TH), str(TH.parent), str(TH.parent / 'fixtures')]
import classes as CL  # noqa: E402
import e3_class as E3  # noqa: E402
import orient as O  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
argv = sys.argv[1:]
MAXD, MAXPAIRS, NEAR = 10, 0, False
if '--pairs-near-units' in argv:
    argv.remove('--pairs-near-units')
    NEAR = True
if '--max-pairs' in argv:
    k = argv.index('--max-pairs')
    MAXPAIRS = int(argv[k + 1])
    del argv[k:k + 2]
if '--max-depth' in argv:
    k = argv.index('--max-depth')
    MAXD = int(argv[k + 1])
    del argv[k:k + 2]
table, out, ufiles = argv[0], argv[1], argv[2:]
src = (REPO / 'research/jumbling/restoration/12-low-height/scripts/cellpure5.py').read_text().splitlines()
a = next(i for i, l in enumerate(src) if l.startswith('class Model2'))
b = next(i for i, l in enumerate(src) if l.startswith('members, cls = CL.classes()'))
exec('\n'.join(src[a:b]), globals())
rec = {'table': Path(table).name, 'max_depth': MAXD, 'classes': {}, 'units': {}}


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), flush=True)


def save():
    Path(out).write_text(json.dumps(rec, indent=1) + '\n')


def lcm(xs):
    v = 1
    for x in xs:
        v = v * x // math.gcd(v, x)
    return v


def roots(par):
    r = par.copy()
    while True:
        nr = r[r]
        if (nr == r).all():
            return r
        r = nr


def exact(word, U, expect_off=()):
    R2, _ = O.start(table)
    dom0 = np.asarray(R2.domains().dom).copy()
    off0 = np.flatnonzero(dom0 != 0)
    others = np.setdiff1d(off0, U)
    pose0 = {int(p): to_tuple(R2.st.pose(int(p))) for p in others.tolist()}
    for c, i in word:
        R2.apply(c, i)
    dom1 = np.asarray(R2.domains().dom)
    newly = np.setdiff1d(np.flatnonzero(dom1 != 0), off0)
    changed = [p for p in others.tolist() if to_tuple(R2.st.pose(int(p))) != pose0[p]]
    return {'letters': len(word), 'off_after': int(R2.st.off_lattice_count()), 'unit_on_after': int((dom1[U] == 0).sum()),
            'unit_size': int(len(U)), 'others_off': int(len(others)),
            'others_exact_pose_kept': int(len(others) - len(changed)), 'newly_off': int(len(newly)),
            'newly_off_as_predicted': bool(set(newly.tolist()) == set(expect_off)), 'digest_after': R2.st.digest()}


members, cls = CL.classes()
R, M = O.start(table)
rec['digest'] = R.st.digest()
rec['off'] = int(R.st.off_lattice_count())
D = R.domains()
hubs = {d: CL.hub(D.reps[d], members, cls) for d in range(1, len(D.reps))}
dom_s = np.asarray(D.dom)
units = {}
for f in ufiles:
    U = np.asarray(sorted(json.loads(Path(f).read_text())['pieces']), np.int64)
    d = int(dom_s[U.min()])
    assert d != 0 and (dom_s[U] == d).all() and len(hubs[d]) == 1, f
    units[Path(f).stem] = (U, d, int(hubs[d][0][1]))
byP = defaultdict(list)
for name, (U, d, P) in units.items():
    byP[P].append(name)
log(units=len(units), classes={str(P): len(v) for P, v in byP.items()})
data = CL.ctx.data
NPC = len(data.sigbits)

for P, names in sorted(byP.items()):
    E3.P = P
    m = Model2(R, M, members, cls)
    NPT = m.NC * 5
    NCONT = len(m.state)
    SPECIAL = NCONT
    sigc = np.zeros((NPC, 120), bool)
    for k, c in enumerate(m.poles):
        sigc[:, k] = data.in_sig(np.arange(NPC), np.full(NPC, c, np.int64))
    home = Counter(m.cid[kk] for kk in (int.from_bytes(np.packbits(r).tobytes(), 'big') for r in sigc) if kk in m.cid)
    kind = np.repeat(m.ncaps, 5)
    # owners
    own = np.full(NPT, -1, np.int64)
    for j, p in enumerate(m.state.tolist()):
        if m.cdom[j] == 0:
            assert own[p] == -1
            own[p] = j
    for j, p in enumerate(m.state.tolist()):
        if m.cdom[j] != 0:
            c5 = p - p % 5
            for q in range(c5, c5 + 5):
                if own[q] == -1:
                    own[q] = j
                elif own[q] != j and m.cdom[own[q]] != 0:
                    own[q] = SPECIAL
    psig = np.zeros((len(dom_s), 120), bool)
    for k, c in enumerate(m.poles):
        cl = R.st.classify(int(c), need_inside=True, stop_at_straddle=False)
        psig[np.asarray(cl.inside, np.int64), k] = True
    foreign = np.flatnonzero((dom_s != 0) & ~np.isin(dom_s, list(m.frag)))
    fcells, untracked = set(), 0
    for p in foreign.tolist():
        kk = int.from_bytes(np.packbits(psig[p]).tobytes(), 'big')
        if kk in m.cid:
            fcells.add(m.cid[kk])
        else:
            untracked += 1
    for cc in fcells:
        own[cc * 5:cc * 5 + 5] = SPECIAL
    occ = own >= 0
    # the units of this class
    upt, uid = {}, {}
    for name in names:
        U, d, _ = units[name]
        Us = set(U.tolist())
        hit = [j for j, ps in enumerate(m.cpieces) if m.cdom[j] == d and Us & set(ps)]
        assert len(hit) == 1 and Us <= set(m.cpieces[hit[0]]), name
        j = hit[0]
        p = int(m.state[j])
        c5 = p - p % 5
        cellown = sorted({int(o) for o in own[c5:c5 + 5].tolist()} - {j})
        glued = [o for o in cellown if o >= 0]
        upt[name] = {'content': j, 'point': p, 'content_size': len(m.cpieces[j]), 'unit_size': len(U),
                     'kind': int(m.ncaps[p // 5]), 'label': p % 5, 'cell': m.caps(m.cells[p // 5]),
                     'home_cell': bool(p // 5 in home),
                     'glued_to': [('foreign or shared' if o == SPECIAL else
                                   f'lattice content of {len(m.cpieces[o])} pieces' if m.cdom[o] == 0 else
                                   f'off content of {len(m.cpieces[o])} pieces') for o in glued]}
        uid[j] = name
    kinds_needed = {u['kind'] for u in upt.values() if u['kind'] in (3, 4)}
    entry_owner = np.zeros(NCONT + 1, bool)
    for j, p in enumerate(m.state.tolist()):
        if m.cdom[j] == 0 and p // 5 in home and int(m.ncaps[p // 5]) in kinds_needed:
            entry_owner[j] = True
    vac = defaultdict(list)
    for cc in home:
        if own[cc * 5] == -1:
            vac[int(m.ncaps[cc])].append(cc * 5)
    freecells = defaultdict(int)
    for cc in range(m.NC):
        if not occ[cc * 5:cc * 5 + 5].any():
            freecells[int(m.ncaps[cc])] += 1
    rec['classes'][str(P)] = {'cells': int(m.NC), 'free_grips': len(m.A), 'home_cells': len(home),
                              'occupied_points': int(occ.sum()), 'foreign_cells': len(fcells),
                              'foreign_untracked': untracked, 'off_contents': int(sum(1 for d in m.cdom if d != 0)),
                              'free_cells_by_kind': {str(k): v for k, v in sorted(freecells.items())},
                              'vacant_home_points_by_kind': {str(k): len(v) for k, v in vac.items()}}
    log(P=int(P), **rec['classes'][str(P)])
    save()
    par = np.arange(NPT)

    def find(i, par=par):
        r = i
        while par[r] != r:
            r = par[r]
        while par[i] != r:
            par[i], i = r, par[i]
        return r

    def subcycles(cy, L):
        n = len(cy)
        g = math.gcd(n, L)
        return [[cy[(i + j * L) % n] for j in range(n // g)] for i in range(g)]

    base, upow = [], defaultdict(list)
    pairs = [(c, d) for c, d in m.adjacent(set(m.A)) if c < d]
    if NEAR:
        nc = {c for u in upt.values() for c in u['cell']}
        pairs = [(c, d) for c, d in pairs if c in nc or d in nc]
    if MAXPAIRS:
        pairs = pairs[:MAXPAIRS]
    n_comm = 0

    def cycles_of(x, y):
        A = m.array(m.comm(x, y))
        sup = np.flatnonzero(A != m.ar)
        seen, cyc = set(), []
        for s in sup.tolist():
            if s in seen:
                continue
            cy, q = [], s
            while q not in seen:
                seen.add(q)
                cy.append(q)
                q = int(A[q])
            cyc.append(cy)
        return cyc
    for c, d in pairs:
        for x in m.byc[c]:
            for y in m.byc[d]:
                n_comm += 1
                cyc = cycles_of(x, y)
                if not cyc:
                    continue
                ow = []
                for cy in cyc:
                    o = own[cy]
                    ow.append(frozenset(o[o >= 0].tolist()))
                occc = [i for i in range(len(cyc)) if ow[i]]
                L = lcm(len(cyc[i]) for i in occc)
                mv = [cy for cy in cyc if L % len(cy)]
                if mv:
                    for cy in mv:
                        for sub in subcycles(cy, L):
                            r0 = find(sub[0])
                            for q in sub[1:]:
                                rq = find(q)
                                if rq != r0:
                                    par[rq] = r0
                    base.append((x, y, L, np.asarray([q for cy in mv for q in cy], np.int32)))
                for i in occc:
                    if len(ow[i]) != 1:
                        continue
                    o = next(iter(ow[i]))
                    if o not in uid:
                        continue
                    Lo = lcm(len(cyc[k]) for k in occc if ow[k] != {o})
                    if Lo % len(cyc[i]):
                        upow[uid[o]].append((x, y, Lo, [cy for cy in cyc if Lo % len(cy)]))
        log(P=int(P), pair=[int(c), int(d)], commutators=n_comm, base=len(base),
            unit_powers=sum(len(v) for v in upow.values()))
    rec['classes'][str(P)].update(pairs=len(pairs), commutators=n_comm, base_clean_powers=len(base))
    save()
    bmoves = defaultdict(list)
    for n, (x, y, L, mvp) in enumerate(base):
        for q in mvp.tolist():
            bmoves[q].append(n)
    # components of the movable units
    comps, live = {}, []
    for name in names:
        u = upt[name]
        if u['glued_to'] or not upow[name]:
            continue
        pu = par.copy()

        def findu(i, par=pu):
            r = i
            while par[r] != r:
                r = par[r]
            while par[i] != r:
                par[i], i = r, par[i]
            return r
        for x, y, Lu, mvu in upow[name]:
            for cy in mvu:
                for sub in subcycles(cy, Lu):
                    r0 = findu(sub[0])
                    for q in sub[1:]:
                        rq = findu(q)
                        if rq != r0:
                            pu[rq] = r0
        rt = roots(pu)
        comps[name] = rt == rt[u['point']]
        live.append(name)
    # second pass: entry powers that move one lattice content h of a triangle or tetrahedron home cell, from a point
    # in a unit's component into h's point; the best (shortest power) per unit and home cell
    tl = [n for n in live if upt[n]['kind'] in (3, 4)]
    best = defaultdict(dict)
    if tl:
        CM = np.stack([comps[n] for n in tl])
        kinds_t = np.array([upt[n]['kind'] for n in tl])
        n_e = 0
        for c, d in pairs:
            for x in m.byc[c]:
                for y in m.byc[d]:
                    cyc = cycles_of(x, y)
                    if not cyc:
                        continue
                    ow = []
                    for cy in cyc:
                        o = own[cy]
                        ow.append(frozenset(o[o >= 0].tolist()))
                    occc = [i for i in range(len(cyc)) if ow[i]]
                    for i in occc:
                        if len(ow[i]) != 1:
                            continue
                        h = next(iter(ow[i]))
                        if h >= NCONT or not entry_owner[h]:
                            continue
                        hp = int(m.state[h])
                        hk = int(m.ncaps[hp // 5])
                        Lo = lcm(len(cyc[k]) for k in occc if k != i)
                        if Lo % len(cyc[i]) == 0:
                            continue
                        sub = next(sb for sb in subcycles(cyc[i], Lo) if hp in sb)
                        hit = CM[:, sub].any(axis=1) & (kinds_t == hk)
                        if not hit.any():
                            continue
                        ordA = lcm(len(cy) for cy in cyc)
                        for t in np.flatnonzero(hit).tolist():
                            bd = best[tl[t]]
                            if h not in bd or Lo < bd[h][2]:
                                bd[h] = (x, y, Lo, ordA, sub)
                                n_e += 1
            log(P=int(P), entry_pair=[int(c), int(d)], updates=n_e)
    for name in names:
        u = upt[name]
        res = dict(u)
        res['unit_powers'] = len(upow[name])
        U = units[name][0]
        if u['glued_to']:
            res['result'] = 'glued: its cell holds another owner'
            rec['units'][name] = res
            log(unit=name, **{k: res[k] for k in ('unit_size', 'kind', 'glued_to', 'result')})
            save()
            continue
        if not upow[name]:
            res['result'] = 'no clean power moves it'
            rec['units'][name] = res
            log(unit=name, **{k: res[k] for k in ('unit_size', 'kind', 'result')})
            save()
            continue
        comp = comps[name]
        ccells = sorted({int(q) // 5 for q in np.flatnonzero(comp).tolist()})
        res['component_points'] = int(comp.sum())
        res['component_cells_by_kind'] = dict(sorted(Counter(str(int(m.ncaps[cc])) for cc in ccells).items()))
        tz = [v for v in vac.get(u['kind'], []) if comp[v]]
        res['vacant_home_points_of_kind'] = len(vac.get(u['kind'], []))
        res['vacant_in_component'] = len(tz)
        bd = best.get(name, {})
        res['entry_home_cells'] = len(bd)
        res['entry_bad_histogram'] = {str(k): v for k, v in sorted(Counter(len(m.cpieces[h]) for h in bd).items())}
        umoves = defaultdict(list)
        for n, (x, y, Lu, mvu) in enumerate(upow[name]):
            for cy in mvu:
                for q in cy:
                    umoves[q].append(n)

        def bfs(targets):
            prev = {u['point']: None}
            dq = deque([(u['point'], 0)])
            while dq:
                q, dpt = dq.popleft()
                if q in targets:
                    path = []
                    while prev[q] is not None:
                        q0, kd, n = prev[q]
                        path.append((kd, n))
                        q = q0
                    return path[::-1], dpt
                if dpt >= MAXD:
                    continue
                for kd, lst, srcl in (('b', bmoves.get(q, ()), base), ('u', umoves.get(q, ()), upow[name])):
                    for n in lst:
                        x, y, L = srcl[n][0], srcl[n][1], srcl[n][2]
                        r = int(m.run(m.comm(x, y, L), np.array([q]))[0])
                        if r not in prev:
                            prev[r] = (q, kd, n)
                            dq.append((r, dpt + 1))
            return None, None

        def word_of(path):
            wg = []
            for kd, n in path:
                x, y, L = (base if kd == 'b' else upow[name])[n][:3]
                wg += m.comm(x, y, L)
            return wg
        oth = [jj for jj in range(NCONT) if jj != u['content']]
        if tz:
            path, dpt = bfs(set(tz))
            if path is None:
                res['zero_bad'] = 'no path within the depth bound'
            else:
                wg = word_of(path)
                fin = m.run(wg, m.state)
                word = [list(map(int, m.gens[g])) for g in wg]
                ex = exact(word, U)
                ok = (ex['unit_on_after'] == len(U) and ex['newly_off'] == 0
                      and ex['others_exact_pose_kept'] == ex['others_off'])
                res['zero_bad'] = {'powers': len(path), 'letters': len(word),
                                   'to': m.caps(m.cells[int(fin[u['content']]) // 5]),
                                   'others_fixed_in_model': bool((fin[oth] == m.state[oth]).all()), 'exact': ex,
                                   'strict': bool(ok), 'word': word}
                log(unit=name, zero_bad={k: v for k, v in res['zero_bad'].items() if k != 'word'})
        if bd:
            order = sorted(bd, key=lambda h: (len(m.cpieces[h]), bd[h][2]))
            for hmin in order[:3]:
                x, y, Lo, ordA, sub = bd[hmin]
                tq = {q for q in sub if comp[q]}
                path, dpt = bfs(tq)
                if path is None:
                    continue
                q = int(m.run(word_of(path), np.array([u['point']]))[0]) if path else u['point']
                hp = int(m.state[hmin])
                kk = (sub.index(hp) - sub.index(q)) % len(sub)
                ex_n = (Lo * kk) % ordA
                wg = word_of(path) + m.comm(x, y, ex_n)
                fin = m.run(wg, m.state)
                word = [list(map(int, m.gens[g])) for g in wg]
                oth2 = [jj for jj in oth if jj != hmin]
                ex = exact(word, U, expect_off=m.cpieces[hmin])
                ok = (ex['unit_on_after'] == len(U) and ex['newly_off_as_predicted']
                      and ex['others_exact_pose_kept'] == ex['others_off'])
                res['small_bad'] = {'bad': len(m.cpieces[hmin]), 'home_cell': m.caps(m.cells[hp // 5]),
                                    'powers': len(path) + 1, 'letters': len(word),
                                    'lands_in_model': bool(int(fin[u['content']]) == hp),
                                    'others_fixed_in_model': bool((fin[oth2] == m.state[oth2]).all()),
                                    'exact': ex, 'W_ok': bool(ok), 'word': word}
                log(unit=name, small_bad={k: v for k, v in res['small_bad'].items() if k != 'word'})
                if ok:
                    break
                # The exact replay rejects this entry word: record it and try the next home cell.
                res.setdefault('small_bad_rejected', []).append(res.pop('small_bad'))
        zb = res.get('zero_bad')
        if isinstance(zb, dict) and zb['strict']:
            res['result'] = 'zero-Bad strict step'
        elif isinstance(zb, dict):
            res['result'] = 'zero-Bad word fails the exact check'
        elif 'small_bad' in res:
            res['result'] = 'transport with Bad ' + str(res['small_bad']['bad'])
        elif 'small_bad_rejected' in res:
            res['result'] = 'entry words fail the exact check'
        elif not tz and not bd:
            res['result'] = 'no home cell of its kind reachable'
        else:
            res['result'] = 'no path within the depth bound'
        rec['units'][name] = res
        log(unit=name, unit_size=len(U), kind=u['kind'], component_points=res['component_points'],
            vacant_in_component=res['vacant_in_component'], entry_home_cells=res['entry_home_cells'],
            result=res['result'])
        save()
log(done=True, units=len(units))
save()
