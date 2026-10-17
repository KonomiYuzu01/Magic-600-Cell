"""[cellpure4: copy of cellpure3.py (stage 09) with two changes, marked "cellpure4" below: the unit content is found
by its pieces, and the word is the first model-passing solution whose exact replay keeps every other off piece's exact
pose (option --max-exact N, default 12).]
[cellpure3: copy of xp-design-wordclass/cellpure2.py for a unit T at a state with several off domains:
args <theory> <table> <unit json> <out> [--search]; P = class of T's domain; T's content j_T located by domain;
goal = every current content point except j_T, plus (X, 0); model check per content; shortest passing cycle;
exact replay checks the other off pieces' exact poses and that the newly off set is the lattice pieces of X.]
Last carrier as one edge cell (two contents per cell allowed) of its class: pure single-cell twist (E3 F48 method) at an 86 state.

Light check for the word-class route (menu I_a; research only, headless; not Windows, DirectX or performance
evidence). Uses the class cell model of theory/e3_class.py (imported, not edited) with P set to the carrier's class:
1. state: the off set is one domain of class P; every class-P grip is free;
2. cell: the carrier is one content at an edge cell X with label l != 0; the holes are exactly the chambers of X
   (sign vector over the 120 caps of P), so the point (X, 0) is vacant;
3. orbit: (X, l) and (X, 0) lie in one orbit of the free twists;
4. search (optional, --search): pure two-edge-cell twists [x, y]^k near X (E3 search_f48 logic, copied and
   parametrised), one Z5 ratio per cell pair, a cycle of 3 or 4 edge cells through X whose exponents give X alone
   the twist l -> 0; model check (every content on label 0);
5. exact (with --search): replay of the word on the exact simulator from the table, every twist checked admissible,
   off count after.
usage: cellpure3.py <theory dir> <table> <unit json> <out json> [--search]
"""
import itertools
import json
import sys
import time
from collections import defaultdict
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
for p in (TH, TH.parent, TH.parent / 'fixtures'):
    sys.path.insert(0, str(p))
import classes as CL  # noqa: E402
import e3_class as E3  # noqa: E402
import orient as O  # noqa: E402

T0 = time.time()
table, ufile, out = sys.argv[2], sys.argv[3], sys.argv[4]
U = np.asarray(sorted(json.loads(Path(ufile).read_text())['pieces']), np.int64)
SEARCH = '--search' in sys.argv
rec = {'table': Path(table).name}


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), flush=True)


def save():
    Path(out).write_text(json.dumps(rec, indent=1))


from collections import defaultdict as _dd
class Model2(E3.Model):
    """The class-1 cell model at a state (see the module docstring)."""

    def __init__(self, R, M, members, cls):
        self.poles = poles = members[E3.P]
        self.kof = {c: k for k, c in enumerate(poles)}
        D = R.domains()
        dom = np.asarray(D.dom)
        dcls = {}
        for d in range(1, len(D.reps)):
            h = E3.CL.hub(D.reps[d], members, cls)
            dcls[d] = h[0][1] if len(h) == 1 else None
        frag = [d for d, v in dcls.items() if v == E3.P]
        self.frag = {d: len(D.members[d]) for d in frag}
        own = (dom == 0) | np.isin(dom, frag)
        sig = np.zeros((len(dom), 120), bool)
        A, blocked = [], []
        for k, c in enumerate(poles):
            cl = R.st.classify(int(c), need_inside=True, stop_at_straddle=False)
            ins = np.asarray(cl.inside, np.int64)
            sig[ins, k] = True
            if cl.straddle is not None:
                blocked.append(int(c))
            elif not (~own)[ins].any():
                A.append(int(c))
        self.A, self.blocked = A, blocked
        contents = {}
        for p in np.flatnonzero(own).tolist():
            contents.setdefault((int.from_bytes(np.packbits(sig[p]).tobytes(), 'big'), int(dom[p])), []).append(p)
        domain = {kd: kd[1] for kd in contents}
        self.size = {kd: len(ps) for kd, ps in contents.items()}
        self.cell_size = {}
        for (key, d), ps in contents.items():
            self.cell_size[key] = self.cell_size.get(key, 0) + len(ps)
        E3.log(own_contents=len(contents), free=len(A), blocked=blocked, fragments=self.frag)
        # the five left cosets of class P, label 0 = the lattice
        c0 = poles[0]
        inv0 = E3.CL.ninv(c0)
        reps = []
        for u in E3.trees.UNITS:
            a = E3.trees.qmul(E3.trees.qmul(E3.CL.N[c0], u), inv0)
            if not any(E3.prop_unit(E3.trees.qmul(E3.trees.qconj(r), a)) for r in reps):
                reps.append(a)
        assert len(reps) == 5
        reps.sort(key=lambda r: not E3.prop_unit(r))
        assert E3.prop_unit(reps[0]) and not any(E3.prop_unit(r) for r in reps[1:])

        def label_of_left(l):
            out = [j for j, r in enumerate(reps) if E3.prop_unit(E3.trees.qmul(E3.trees.qconj(r), l))]
            assert len(out) == 1, out
            return out[0]
        dl = {0: 0}
        for d in frag:
            l, r = E3.trees.factors(D.reps[d])
            assert E3.prop_unit(r)
            dl[d] = label_of_left(l)
        self.frag_label = {d: dl[d] for d in frag}
        # exact pole permutations and label actions of every free twist
        NI = len(M.items)
        perm, act = {}, {}
        for c in A:
            for i in range(NI):
                g = E3.restore.element(M, c, i)
                img = [self.kof.get(E3.CL.pole(E3.matvec(g, E3.CL.N[d])), -1) for d in poles]
                assert min(img) >= 0
                perm[(c, i)] = img
                l, _ = E3.trees.factors(g)
                act[(c, i)] = tuple(label_of_left(E3.trees.qmul(l, reps[k])) for k in range(5))
        E3.log(model='pole permutations and label actions')
        self.gens = gens = [(c, i) for c in A for i in range(NI) if act[(c, i)] != E3.ID5 or perm[(c, i)] != list(range(120))]
        cells = list(dict.fromkeys(k for k, _ in contents))
        idx = {s: n for n, s in enumerate(cells)}
        todo = list(cells)
        while todo:
            s = todo.pop()
            for c in A:
                if s >> (119 - self.kof[c]) & 1:
                    for i in range(NI):
                        t = self.move(s, perm[(c, i)])
                        if t not in idx:
                            idx[t] = len(cells)
                            cells.append(t)
                            todo.append(t)
        self.cells, self.cid = cells, idx
        self.NC = NC = len(cells)
        self.ncaps = np.array([bin(s).count('1') for s in cells])
        base = np.arange(NC * 5, dtype=np.int32)
        G = np.empty((len(gens), NC * 5), np.int32)
        for n, (c, i) in enumerate(gens):
            a, row, bit = act[(c, i)], base.copy(), 119 - self.kof[c]
            for s in cells:
                if s >> bit & 1:
                    f, t = idx[s], idx[self.move(s, perm[(c, i)])]
                    row[f * 5:f * 5 + 5] = [t * 5 + a[l] for l in range(5)]
            G[n] = row
        self.G = G
        key = {G[n].tobytes(): n for n in range(len(gens))}
        self.inv = np.empty(len(gens), np.int64)
        for n in range(len(gens)):
            r = np.empty(NC * 5, np.int32)
            r[G[n]] = base
            self.inv[n] = key[r.tobytes()]
        self.state = np.array([idx[s] * 5 + dl[d] for s, d in contents], np.int64)
        self.cdom = [d for (_, d) in contents]
        self.ckey = list(contents.keys())
        self.cpieces = [contents[k] for k in contents]
        self.byc = defaultdict(list)
        for n, (c, i) in enumerate(gens):
            self.byc[c].append(n)
        self.ar = np.arange(NC * 5)
        E3.log(cells=NC, generators=len(gens), bad_at_start=int((self.state % 5 != 0).sum()))


members, cls = CL.classes()
R, M = O.start(table)
st = R.st
rec['digest'] = st.digest()
rec['off_before'] = int(st.off_lattice_count())
D = R.domains()
hubs = {d: CL.hub(D.reps[d], members, cls) for d in range(1, len(D.reps))}
rec['domains'] = {str(d): {'size': len(D.members[d]), 'hub': hubs[d]} for d in hubs}
log(**{k: rec[k] for k in ('table', 'off_before', 'domains')})
dom_s = np.asarray(D.dom)
d_T = int(dom_s[U.min()])
assert d_T != 0 and (dom_s[U] == d_T).all(), 'unit not one domain'
assert len(hubs[d_T]) == 1, hubs[d_T]
P = hubs[d_T][0][1]
rec['d_T'] = d_T
E3.P = P
rec['class'] = int(P)
m = Model2(R, M, members, cls)
rec['free_grips'] = len(m.A)
rec['blocked'] = m.blocked
rec['own_contents'] = len(m.size)
rec['cell_X_contents'] = None
rec['reachable_cells'] = int(m.NC)
rec['generators'] = len(m.gens)
# cellpure4: the unit's content is the content of its domain that holds its pieces (a frame domain may have contents
# in several cells at a deep state); cellpure3 asked the domain to have one content only
U_set = set(U.tolist())
hit = [j for j, ps in enumerate(m.cpieces) if m.cdom[j] == d_T and U_set & set(ps)]
assert len(hit) == 1 and U_set <= set(m.cpieces[hit[0]]), ('unit split over contents', len(hit))
j_T = hit[0]
rec['unit_content_size'] = len(m.cpieces[j_T])
rec['domain_contents'] = m.cdom.count(d_T)
X, ell = int(m.state[j_T]) // 5, int(m.state[j_T]) % 5
assert ell != 0
rec['fragments_in_model'] = {str(d): {'content': m.cdom.index(d), 'point': int(m.state[m.cdom.index(d)]), 'n_contents': m.cdom.count(d)} for d in m.frag}
rec['contents_off_label0'] = int((m.state % 5 != 0).sum())
capsX = m.caps(m.cells[X])
rec['carrier_cell'] = {'caps': capsX, 'n_caps': len(capsX), 'label': int(ell), 'pieces_in_cell': int(m.cell_size[m.cells[X]])}
# holes and the sign vector of X
data = CL.ctx.data
NP = len(data.sigbits)
lat = np.flatnonzero(np.asarray(D.dom) == 0)
occ = np.zeros(NP, bool)
occ[np.asarray(D.cham)[lat]] = True
holes = np.flatnonzero(~occ)
sig = np.zeros((NP, 120), bool)
for k, c in enumerate(m.poles):
    sig[:, k] = data.in_sig(np.arange(NP), np.full(NP, c, np.int64))
keyX = m.cells[X]
bits = np.array([(keyX >> (119 - k)) & 1 for k in range(120)], bool)
inX = (sig == bits).all(1)
rec['holes'] = int(len(holes))
rec['chambers_of_X'] = int(inX.sum())
rec['holes_eq_chambers_of_X'] = bool(np.array_equal(np.flatnonzero(inX), holes))
occ_pts = {int(p) // 5: int(p) % 5 for p in m.state}
rec['X_label0_vacant'] = X not in occ_pts or occ_pts[X] != 0
log(carrier_cell=rec['carrier_cell'], holes=rec['holes'], chambers_of_X=rec['chambers_of_X'],
    holes_eq_chambers_of_X=rec['holes_eq_chambers_of_X'], free=rec['free_grips'], gens=rec['generators'],
    cells=rec['reachable_cells'])
save()
comp = E3.orbits(m)
rec['labels_of_X_in_carrier_orbit'] = [l for l in range(5) if comp[X * 5 + l] == comp[X * 5 + ell]]
rec['X0_in_carrier_orbit'] = bool(comp[X * 5] == comp[X * 5 + ell])
ptype = np.repeat(m.ncaps, 5)
edge_pts = np.flatnonzero(ptype == 2)
rec['edge_orbits'] = len(set(comp[edge_pts].tolist()))
log(labels_of_X_in_orbit=rec['labels_of_X_in_carrier_orbit'], edge_orbits=rec['edge_orbits'])
save()
if not SEARCH:
    sys.exit(0)

# ---- search: copy of e3_class.search_f48 with E = X, the carrier's label ell, and near = X's caps
goal = np.zeros(len(m.ar), bool)
for jj, pt in enumerate(m.state.tolist()):
    if jj != j_T:
        goal[pt] = True
goal[X * 5] = True
gnon = np.flatnonzero(goal & (ptype != 2))
rho = {}


def rpow(s, e):
    q = E3.ID5
    for _ in range(e % 5):
        q = tuple(rho[s][j] for j in q)
    return q


def expo(s, t):
    for e in range(5):
        if rpow(s, e) == tuple(t):
            return e
    raise AssertionError('twist is not a power of the cell 5-cycle')


near = set(capsX)
near = {b for a, b in m.adjacent(near)} | near
pairs = [(a, b) for a, b in m.adjacent(near) if a in near and b in near]
log(near=len(near), grip_pairs=len(pairs))
ratio, cheap, n_tools = defaultdict(set), {}, 0
for c, d in pairs:
    for x in m.byc[c]:
        for y in m.byc[d]:
            w = m.array(m.comm(x, y))
            mv = np.flatnonzero(w != m.ar)
            em = mv[ptype[mv] == 2]
            if not len(em) or (w[em] // 5 != em // 5).any():
                continue
            n = E3.lcm(E3.cycles(w, mv[ptype[mv] != 2].tolist()))
            if n % 5 == 0 or n > 100:
                continue
            v = {}
            for s in sorted(set((em // 5).tolist())):
                t = tuple(int(w[s * 5 + l]) - s * 5 for l in range(5))
                rho.setdefault(s, t)
                v[s] = expo(s, t)
            if len(v) != 2 or not all(v.values()):
                continue
            n_tools += 1
            u, z = sorted(v)
            ratio[(u, z)].add(v[z] * pow(v[u], -1, 5) % 5)
            cur = gnon.copy()
            for k in range(1, n + 1):
                cur = w[cur]
                if k % 5 and goal[cur].all():
                    lam = v[u] * k % 5
                    for sign, word in ((1, m.comm(x, y, k)), (-1, m.comm(y, x, k))):
                        key = ((u, z), sign * lam % 5)
                        if key not in cheap or len(word) < len(cheap[key]):
                            cheap[key] = word
    log(pair=[c, d], tools=n_tools, cell_pairs=len(ratio))
rec['one_ratio_per_pair'] = all(len(r) == 1 for r in ratio.values())
assert rec['one_ratio_per_pair']
nodes = sorted({c for pr in ratio for c in pr})
ix = {c: k for k, c in enumerate(nodes)}
rows = []
for (u, z), r in ratio.items():
    row = [0] * len(nodes)
    row[ix[u]], row[ix[z]] = 1, next(iter(r))
    rows.append(row)
unit = [0] * len(nodes)
if X in ix:
    unit[ix[X]] = 1
rec.update({'pair_tools': n_tools, 'cell_pairs': len(ratio), 'edge_cells': len(nodes), 'X_in_nodes': X in ix,
            'z5_rank': E3.rank5(rows) if rows else 0, 'z5_rank_with_unit_at_X': E3.rank5(rows + [unit]) if rows else 0})
log(**{k: rec[k] for k in ('pair_tools', 'cell_pairs', 'edge_cells', 'X_in_nodes', 'z5_rank', 'z5_rank_with_unit_at_X')})
save()
lab = {}
for (pr, lam), word in cheap.items():
    lab.setdefault(pr, {})[lam] = word
for pr, d in lab.items():
    base = dict(d)
    for (l1, w1), (l2, w2) in itertools.product(base.items(), base.items()):
        l = (l1 + l2) % 5
        if l and (l not in d or len(w1) + len(w2) < len(d[l])):
            d[l] = w1 + w2
if X not in ix:
    # no pair tool moves the unit's cell, so no cycle can pass through it (and rho has no 5-cycle for it)
    rec['cycles_found'] = 0
    log(cycles_found=0, unit_cell_in_pair_tools=False)
    save()
    sys.exit(0)
tgt = next(e for e in range(5) if rpow(X, e)[ell] == 0)
nb = defaultdict(set)
for (u, z) in lab:
    nb[u].add(z)
    nb[z].add(u)


def solve(cyc):
    k = len(cyc)
    es = [tuple(sorted((cyc[j], cyc[(j + 1) % k]))) for j in range(k)]
    A = np.zeros((k, k), np.int64)
    for j, (u, z) in enumerate(es):
        A[cyc.index(u), j] = 1
        A[cyc.index(z), j] = next(iter(ratio[(u, z)]))
    best = None
    for t in itertools.product(range(5), repeat=k):
        if (A @ np.array(t))[0] % 5 != tgt or any((A[r] @ np.array(t)) % 5 for r in range(1, k)):
            continue
        if any(tt and tt not in lab[es[j]] for j, tt in enumerate(t)):
            continue
        word = [g for j, tt in enumerate(t) if tt for g in lab[es[j]][tt]]
        if best is None or len(word) < len(best[0]):
            best = (word, [m.caps(m.cells[c]) for c in cyc], list(t))
    return best


sols = []
for a in sorted(nb[X]):
    for b in sorted(nb[a]):
        if b != X and X in nb[b]:
            s = solve([X, a, b])
            if s:
                sols.append(s)
        for c in sorted(nb[b]) if b != X else []:
            if c not in (X, a) and X in nb[c]:
                s = solve([X, a, b, c])
                if s:
                    sols.append(s)
sols.sort(key=lambda s: len(s[0]))
rec['cycles_found'] = len(sols)
log(cycles_found=len(sols), target_exponent=tgt)
save()
if not sols:
    sys.exit(0)
def model_check(fin):
    ok = int(fin[j_T]) == X * 5
    for jj in range(len(m.state)):
        if jj == j_T:
            continue
        if m.cdom[jj] != 0:
            ok &= int(fin[jj]) == int(m.state[jj])
        elif int(fin[jj]) % 5 != 0:
            ok &= int(m.state[jj]) // 5 == X
    return bool(ok)


# cellpure4: the model check compares content points (cell, label) only, so another off content can come back to its
# point with a different exact pose. Replay the model-passing solutions exactly, shortest first (at most MAXEXACT of
# them), and take the first that lands the unit, keeps the exact pose of every other off piece and takes off only the
# lattice pieces of X.
MAXEXACT = int(sys.argv[sys.argv.index('--max-exact') + 1]) if '--max-exact' in sys.argv else 12
from sim.kplus import to_tuple  # noqa: E402


def exact_replay(word):
    R2, _ = O.start(table)
    D0 = R2.domains()
    dom0 = np.asarray(D0.dom).copy()
    cham0 = np.asarray(D0.cham).copy()
    inXc = np.zeros(NP, bool)
    inXc[np.flatnonzero(inX)] = True
    latX = np.flatnonzero((dom0 == 0) & inXc[np.where(dom0 == 0, cham0, 0)])
    off0 = np.flatnonzero(dom0 != 0)
    others = np.setdiff1d(off0, U)
    pose0 = {int(p): to_tuple(R2.st.pose(int(p))) for p in others.tolist()}
    for c, i in word:
        R2.apply(c, i)
    D1 = R2.domains()
    dom1 = np.asarray(D1.dom)
    offa = np.flatnonzero(dom1 != 0)
    newly = np.setdiff1d(offa, off0)
    ex = {'all_admissible': True, 'off_before': int(len(off0)), 'off_after': int(R2.st.off_lattice_count()),
          'digest_after': R2.st.digest(),
          'carrier_on_after': int((dom1[U] == 0).sum()), 'carrier_size': int(len(U)),
          'others_off': int(len(others)),
          'others_exact_pose_kept': int(sum(1 for p in others.tolist() if to_tuple(R2.st.pose(int(p))) == pose0[p])),
          'lattice_pieces_in_X_before': int(len(latX)),
          'newly_off_eq_lattice_pieces_of_X': bool(np.array_equal(np.sort(newly), np.sort(latX))),
          'off_after_eq_others_plus_latX': bool(np.array_equal(np.sort(offa), np.sort(np.concatenate([others, latX]))))}
    return ex, R2.st.digest()


chosen, tried = None, []
for nsol, s_ in enumerate(sols):
    fin_ = m.run(s_[0], m.state)
    if not model_check(fin_):
        continue
    word_ = [list(map(int, m.gens[g])) for g in s_[0]]
    ex, dg = exact_replay(word_)
    tried.append({'solution': nsol, 'word_len': len(word_), 'others_exact_pose_kept': ex['others_exact_pose_kept'],
                  'carrier_on_after': ex['carrier_on_after']})
    log(exact_try=tried[-1])
    if ex['carrier_on_after'] == len(U) and ex['others_exact_pose_kept'] == ex['others_off'] \
            and ex['newly_off_eq_lattice_pieces_of_X']:
        chosen = (nsol, s_, fin_, word_, ex, dg)
        break
    if len(tried) >= MAXEXACT:
        break
rec['model_check_pass'] = bool(tried)
rec['exact_tries'] = tried
save()
if chosen is None:
    log(model_check='no cycle solution passes' if not tried else 'no exact solution among the tried')
    sys.exit(0)
word_g, cyc, coef = chosen[1]
fin = chosen[2]
word = chosen[3]
rec['model_contents_off_label_0'] = int((fin % 5 != 0).sum())
rec.update({'cycle': cyc, 'multiples': coef, 'target_exponent': int(tgt), 'word_len': len(word), 'word': word})
rec['exact'] = chosen[4]
Path(out.replace('.json', '-word.json')).write_text(json.dumps({'word': word, 'input_digest': rec['digest'],
                                                                 'output_digest': chosen[5]}))
log(model_off=rec['model_contents_off_label_0'], word_len=len(word), cycle=cyc, exact=rec['exact'])
save()
