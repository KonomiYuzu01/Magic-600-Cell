"""Independent checker for the card-1 certificates (P_o on 35 orbits, F_o on 11 orbits).

usage: python3 -I check_certs.py <repo root> <certificates.json> <report.json>

Uses only numpy and the repository assets (read-only). Shares no code with the constructor.
Checks, all exact on integer sticker slots:
  R1  every retained primitive acts rigidly: each moved sticker (piece p, facet f) goes to
      (piece q, facet g(f)) where g is the primitive's pole permutation and sig(q) = g(sig(p));
  R2  the pole permutations of the 1,200 primitives lie in one group of order 7,200 (K+);
  H   for every orbit, the frame group H_o is the action of Stab_K+(C_o) on the stickers of
      C_o in the reference labels, with the order and type of the card's table;
  P   for every orbit: the complete permutation of c_o is the frame-preserving cycle
      (A B C) on the labelled fibres and the identity on every other sticker; every
      other position X has exactly one setup u_X (an execution-tree path) that fixes every
      sticker of A and B and sends the labelled fibre of C onto the stickers of X;
  F   for the 11 nontrivial frame orbits: the complete permutation of f_o fixes every
      sticker outside two pieces i, j, fixes both positions, and acts there by the exact
      frame values required by the card (C2 (s,s); C5 (r,r^-1); D5 (rs,s); A5 (a,b) with an
      explicit isomorphism H_34 -> A5 sending a to (1 2)(3 4) and b to (1 3 5)).
"""
import gzip
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np

NS = 259800
CELL = 433
T0 = time.time()


def _hook(tp, val, tb):
    # any exception is a rejection, reported as such
    import traceback
    print('FAIL exception', tp.__name__, str(val)[:200])
    traceback.print_exception(tp, val, tb, file=sys.stderr)
    sys.exit(1)


sys.excepthook = _hook
root = Path(sys.argv[1])
cert = json.loads(Path(sys.argv[2]).read_text())
out_path = sys.argv[3]
res = {'checks': {}, 'orbits': {}}
fail = []


def need(cond, msg):
    if not cond:
        fail.append(msg)
    return cond


# ---- model -------------------------------------------------------------------------------
for name, h in cert['model_files'].items():
    need(hashlib.sha256((root / name).read_bytes()).hexdigest() == h, 'digest ' + name)
z = np.load(root / 'assets/model.npz', allow_pickle=False)
sp = z['slot_piece'].astype(np.int64)
oid = z['orbit_id'].astype(np.int64)
mo = z['mask_offsets'].astype(np.int64)
mv = z['mask_values'].astype(np.int64)
rot = z['rotperms'].astype(np.int64)
pz = np.load(root / 'assets/primitives.npz', allow_pickle=False)
src, dst, off = pz['src'].astype(np.int64), pz['dst'].astype(np.int64), pz['offsets'].astype(np.int64)
with gzip.open(root / 'assets/execution_trees.json.gz', 'rt') as f:
    trees = json.load(f)
facet = np.arange(NS) // CELL
order = np.argsort(sp, kind='stable')
cnt = np.bincount(sp, minlength=len(oid))
poff = np.concatenate([[0], np.cumsum(cnt)])


def piece_slots(p):
    return order[poff[p]:poff[p + 1]]


_mv = {}


def move(m):
    r = _mv.get(m)
    if r is None:
        i = abs(m) - 1
        s, d = src[off[i]:off[i + 1]], dst[off[i]:off[i + 1]]
        if m < 0:
            s, d = d, s
        ix = np.argsort(s)
        r = (s[ix], d[ix])
        _mv[m] = r
    return r


def legal_word(w):
    return all(isinstance(m, int) and 1 <= abs(m) <= 1200 for m in w)


def follow(x, w):
    x = np.array(x, np.int64, copy=True)
    for m in w:
        s, d = move(m)
        ix = np.minimum(np.searchsorted(s, x), len(s) - 1)
        hit = s[ix] == x
        x[hit] = d[ix[hit]]
    return x


def sig_sorted(pieces, perm=None):
    """Concatenated sorted signatures of pieces (optionally pole-mapped) and their lengths."""
    pieces = np.asarray(pieces, np.int64)
    lens = mo[pieces + 1] - mo[pieces]
    idx = np.repeat(mo[pieces], lens) + (np.arange(lens.sum()) - np.repeat(np.cumsum(lens) - lens, lens))
    vals = mv[idx]
    if perm is not None:
        vals = perm[vals]
    seg = np.repeat(np.arange(len(pieces)), lens)
    o = np.lexsort((vals, seg))
    return vals[o], lens


# ---- R1 rigidity -------------------------------------------------------------------------
_own = np.repeat(np.arange(len(oid)), np.diff(mo))
_ord = np.argsort(mv, kind='stable')
cap_pieces = _own[_ord]
cap_off = np.searchsorted(mv[_ord], np.arange(601))
bad = 0
for i in range(1200):
    s, d = src[off[i]:off[i + 1]], dst[off[i]:off[i + 1]]
    g = rot[i]
    if not np.array_equal(facet[d], g[facet[s]]):
        bad += 1
        continue
    ps, pd = sp[s], sp[d]
    pairs = np.unique(np.stack([ps, pd], 1), axis=0)
    if len(np.unique(pairs[:, 0])) != len(pairs):
        bad += 1
        continue
    a, la = sig_sorted(pairs[:, 0], g)
    b, lb = sig_sorted(pairs[:, 1])
    if not (np.array_equal(la, lb) and np.array_equal(a, b)):
        bad += 1
    # the cap of primitive i is i // 2: its pieces move by g (unlisted stickers must be fixed
    # by g in place), every other piece is untouched
    cap = i // 2
    capP = cap_pieces[cap_off[cap]:cap_off[cap + 1]]
    if not set(pairs[:, 0].tolist()) <= set(capP.tolist()):
        bad += 1
        continue
    listed = set(s.tolist())
    for p in capP.tolist():
        sl = piece_slots(p)
        rest = [x for x in sl.tolist() if x not in listed]
        if rest:
            a1, _ = sig_sorted([p], g)
            b1, _ = sig_sorted([p])
            if not (np.array_equal(a1, b1) and all(int(g[facet[x]]) == int(facet[x]) for x in rest)):
                bad += 1
                break
need(bad == 0, 'R1 rigidity failures: %d' % bad)
res['checks']['R1_rigid_primitives'] = {'primitives': 1200, 'failures': bad}

# ---- R2 the rotation group ---------------------------------------------------------------
ident = np.arange(600, dtype=np.int64)
seen = {ident.tobytes(): 0}
elems = [ident]
fr = [ident]
gens = [rot[0], rot[1], rot[2], rot[3]]
while fr:
    nx = []
    for x in fr:
        for g in gens:
            y = g[x]
            k = y.tobytes()
            if k not in seen:
                seen[k] = len(elems)
                elems.append(y)
                nx.append(y)
    fr = nx
KP = np.array(elems)
allin = all(rot[i].tobytes() in seen for i in range(1200))
need(len(KP) == 7200 and allin, 'R2 K+ closure')
res['checks']['R2_rotation_group'] = {'order': len(KP), 'all_primitives_inside': allin}

# ---- frame-group helpers -----------------------------------------------------------------


def pmul(x, y):          # chronological: x then y
    return tuple(y[x[i]] for i in range(len(x)))


def pinv(x):
    r = [0] * len(x)
    for i, v in enumerate(x):
        r[v] = i
    return tuple(r)


def pord(x):
    e = tuple(range(len(x)))
    n, y = 1, x
    while y != e:
        y, n = pmul(y, x), n + 1
    return n


def gen_group(gs, k):
    e = tuple(range(k))
    S, fr = {e}, [e]
    while fr:
        nx = []
        for x in fr:
            for g in gs:
                y = pmul(x, g)
                if y not in S:
                    S.add(y)
                    nx.append(y)
        fr = nx
    return S


TABLE = {0: (2, 1200, 'C2'), 1: (1, 3600, '1'), 2: (1, 2400, '1'), 3: (2, 3600, 'C2'), 4: (1, 7200, '1'),
         5: (1, 7200, '1'), 6: (5, 720, 'D5'), 7: (1, 7200, '1'), 8: (2, 3600, 'C2'), 9: (2, 7200, '1'),
         10: (1, 7200, '1'), 11: (2, 3600, 'C2'), 12: (1, 7200, '1'), 13: (1, 7200, '1'), 14: (2, 7200, '1'),
         15: (1, 2400, '1'), 16: (2, 7200, '1'), 17: (5, 1440, 'C5'), 18: (1, 7200, '1'), 19: (1, 7200, '1'),
         20: (2, 7200, '1'), 21: (1, 7200, '1'), 22: (2, 3600, 'C2'), 23: (2, 7200, '1'), 24: (1, 7200, '1'),
         25: (2, 3600, 'C2'), 26: (1, 7200, '1'), 27: (1, 7200, '1'), 28: (5, 1440, 'C5'), 29: (1, 2400, '1'),
         30: (2, 7200, '1'), 31: (1, 7200, '1'), 32: (2, 3600, 'C2'), 33: (1, 2400, '1'), 34: (20, 120, 'A5')}
TYPE_ORDERS = {'1': {1: 1}, 'C2': {1: 1, 2: 1}, 'C5': {1: 1, 5: 4}, 'D5': {1: 1, 2: 5, 5: 4},
               'A5': {1: 1, 2: 15, 3: 20, 5: 24}}


def frame_group(Cpos, labels):
    """Action of Stab_K+(C) on the stickers of C, as permutations of the label indices."""
    sigC, _ = sig_sorted([Cpos])
    imgs = np.sort(KP[:, sigC], axis=1)
    stab = np.flatnonzero(np.all(imgs == sigC[None, :], axis=1))
    fac = facet[labels]
    where = {int(f): t for t, f in enumerate(fac)}
    H = set()
    for g in stab:
        H.add(tuple(where[int(KP[g][f])] for f in fac))
    return H, len(stab)


# ---- SLP evaluation ----------------------------------------------------------------------
nodes = cert['nodes']
as_u = set()
for nd in nodes:
    if nd['op'] == 'conj':
        as_u.add(nd['u'])
as_x = set()
for nd in nodes:
    if nd['op'] == 'conj':
        as_x.add(nd['x'])
    elif nd['op'] == 'prod':
        as_x.update(f[0] for f in nd['f'])
for P in cert['P']:
    as_x.add(P['c_node'])
for F in cert['F']:
    as_x.add(F['node'])
net = {}
length = []
bad_words = 0
E = np.zeros(0, np.int64)


def srt(s, d):
    ix = np.argsort(s)
    return s[ix], d[ix]


def signed(i, sg):
    s, d = net[i]
    return (s, d) if sg > 0 else srt(d, s)


def apply_sparse(x, nt):
    s, d = nt
    x = np.array(x, np.int64, copy=True)
    if len(s) == 0:
        return x
    ix = np.minimum(np.searchsorted(s, x), len(s) - 1)
    hit = s[ix] == x
    x[hit] = d[ix[hit]]
    return x


L = np.arange(NS, dtype=np.int64)
for i, nd in enumerate(nodes):
    if nd['op'] == 'word':
        w = nd['w']
        if not legal_word(w):
            bad_words += 1
        length.append(len(w))
        if i in as_x:
            L[:] = np.arange(NS)
            for m in w:
                s, d = move(m)
                L[d] = L[s]
            y = np.flatnonzero(L != np.arange(NS))
            net[i] = srt(L[y].copy(), y)
            L[:] = np.arange(NS)
    elif nd['op'] == 'conj':
        x, u, us = nd['x'], nd['u'], nd['us']
        assert x < i and u < i and us in (1, -1)
        s, d = net[x]
        if nodes[u]['op'] == 'word':
            w = nodes[u]['w'] if us > 0 else [-m for m in reversed(nodes[u]['w'])]
            net[i] = srt(follow(s, w), follow(d, w))
        else:
            un = signed(u, us)
            net[i] = srt(apply_sparse(s, un), apply_sparse(d, un))
        length.append(length[x] + 2 * length[u])
    elif nd['op'] == 'prod':
        touched = []
        for j, sg in nd['f']:
            assert j < i and sg in (1, -1)
            s, d = signed(j, sg)
            L[d] = L[s]
            touched.append(d)
        t = np.unique(np.concatenate(touched))
        lt = L[t]
        mvd = lt != t
        net[i] = srt(lt[mvd].copy(), t[mvd])
        L[t] = t
        length.append(sum(length[j] for j, _ in nd['f']))
    else:
        need(False, 'unknown node op')
need(bad_words == 0, 'illegal primitive ids in %d words' % bad_words)
res['checks']['slp'] = {'nodes': len(nodes), 'illegal_words': bad_words, 'seconds': round(time.time() - T0, 1)}

# ---- P_o and H_o -------------------------------------------------------------------------
Hgroups = {}
LAB = {}
for P in cert['P']:
    o = P['orbit']
    r = {}
    k, N, typ = TABLE[o]
    A, B, C = P['A'], P['B'], P['C']
    fa, fb, fc = [np.array(f, np.int64) for f in P['frames']]
    pos = np.flatnonzero(oid == o)
    r['positions'] = len(pos)
    ok = need(len(pos) == N and len({A, B, C}) == 3 and all(oid[x] == o for x in (A, B, C)), 'P%d positions' % o)
    for X, f in ((A, fa), (B, fb), (C, fc)):
        ok &= need(len(f) == k and len(set(f.tolist())) == k and np.array_equal(np.sort(f), np.sort(piece_slots(X))),
                   'P%d fibre of %d' % (o, X))
    # frame group
    H, nstab = frame_group(C, fc)
    hist = {}
    for h in H:
        hist[pord(h)] = hist.get(pord(h), 0) + 1
    ok &= need(len(H) * 0 + nstab == 7200 // N and hist == TYPE_ORDERS[typ], 'H%d type' % o)
    Hgroups[o] = H
    r['frame_group'] = {'type': typ, 'order': len(H), 'stabiliser_order': nstab, 'element_orders': hist}
    # c_o
    s, d = net[P['c_node']]
    fib = np.concatenate([fa, fb, fc])
    ok &= need(np.array_equal(np.sort(s), np.sort(fib)), 'P%d c support' % o)
    img = dict(zip(s.tolist(), d.tolist()))
    for a, b in ((fa, fb), (fb, fc), (fc, fa)):
        ok &= need(all(img.get(int(x)) == int(y) for x, y in zip(a, b)), 'P%d c frame cycle' % o)
    r['c_length'] = length[P['c_node']]
    # setups
    t = trees[o]
    par, mvs, tpos, tfr = t['parent'], t['move'], t['positions'], t['frames']
    need(t['buffers'] == [A, B] and t['third'] == C and tpos[0] == C and list(tfr[0]) == fc.tolist(), 'P%d tree root' % o)
    setups = {int(X): n for X, n in P['setup_tree_nodes'].items()}
    ok &= need(set(setups) == set(int(x) for x in pos) - {A, B, C}, 'P%d setup coverage' % o)
    ABf = np.concatenate([fa, fb])
    depth_max, nbad = 0, 0
    for X, n in setups.items():
        w = []
        m = n
        while par[m] >= 0:
            w.append(mvs[m])
            m = par[m]
        w = w[::-1]
        depth_max = max(depth_max, len(w))
        if not legal_word(w) or tpos[n] != X:
            nbad += 1
            continue
        if not np.array_equal(follow(ABf, w), ABf):
            nbad += 1
            continue
        lx = follow(fc, w)
        if not np.array_equal(np.sort(lx), np.sort(piece_slots(X))) or lx.tolist() != list(tfr[n]):
            nbad += 1
        LAB[X] = lx
    ok &= need(nbad == 0, 'P%d setups: %d bad' % (o, nbad))
    LAB[A], LAB[B], LAB[C] = fa, fb, fc
    r['setups'] = len(setups)
    r['setup_depth_max'] = depth_max
    r['P_pass'] = bool(ok)
    res['orbits'][o] = r

# ---- F_o ---------------------------------------------------------------------------------
A5a = (1, 0, 3, 2, 4)   # (1 2)(3 4) on letters 1..5 as 0-based images
A5b = (2, 1, 4, 3, 0)   # (1 3 5): 1->3, 3->5, 5->1
labels_of = {}
for P in cert['P']:
    o = P['orbit']
    labels_of[o] = {P['A']: np.array(P['frames'][0]), P['B']: np.array(P['frames'][1]), P['C']: np.array(P['frames'][2])}
Fres = {}
for F in cert['F']:
    o = F['orbit']
    typ = TABLE[o][2]
    P = cert['P'][o]
    i, j = F['i'], F['j']
    ok = need(i != j and i in labels_of[o] and j in labels_of[o], 'F%d positions must be buffer positions' % o)
    li, lj = labels_of[o][i], labels_of[o][j]
    s, d = net[F['node']]
    ok &= need(np.array_equal(np.sort(s), np.sort(np.concatenate([li, lj]))) or
               set(s.tolist()) <= set(li.tolist()) | set(lj.tolist()), 'F%d support' % o)

    def val(lab):
        im = apply_sparse(lab, (s, d))
        inv = {int(x): t for t, x in enumerate(lab)}
        return tuple(inv.get(int(y), -1) for y in im)
    vi, vj = val(li), val(lj)
    H = Hgroups[o]
    ok &= need(vi in H and vj in H, 'F%d values in H' % o)
    e = tuple(range(len(vi)))
    detail = {'i': i, 'j': j, 'value_i': list(vi), 'value_j': list(vj), 'length': length[F['node']]}
    if typ == 'C2':
        ok &= need(vi != e and vi == vj, 'F%d (s,s)' % o)
    elif typ == 'C5':
        ok &= need(pord(vi) == 5 and vj == pinv(vi), 'F%d (r,r^-1)' % o)
    elif typ == 'D5':
        sR = vj
        rR = pmul(vi, sR)              # vi = r s (r first, then s)  =>  r = vi s^-1 = vi s
        ok &= need(pord(sR) == 2 and pord(rR) == 5 and pmul(rR, sR) == vi, 'F%d (rs,s)' % o)
        ok &= need(pmul(pmul(sR, rR), sR) == pinv(rR) and len(gen_group([rR, sR], 5)) == 10, 'F%d D5 relations' % o)
        detail['r'] = list(rR)
        detail['s'] = list(sR)
    elif typ == 'A5':
        ok &= need(pord(vi) == 2 and pord(vj) == 3, 'F%d orders' % o)
        # explicit isomorphism phi: <vi, vj> -> A5 on five letters, vi -> a, vj -> b
        phi = {e: tuple(range(5))}
        fr = [e]
        consistent = True
        while fr:
            nx = []
            for x in fr:
                for g, ga in ((vi, A5a), (vj, A5b)):
                    y = pmul(x, g)
                    ya = pmul(phi[x], ga)
                    if y in phi:
                        consistent &= phi[y] == ya
                    else:
                        phi[y] = ya
                        nx.append(y)
            fr = nx
        bij = len(phi) == 60 and len(set(phi.values())) == 60 and set(phi) == H
        hom = consistent and all(phi[pmul(x, y)] == pmul(phi[x], phi[y]) for x in phi for y in phi)
        ok &= need(bij and hom, 'F%d isomorphism' % o)
        detail['isomorphism_to_A5'] = {'elements': 60, 'bijective': bij, 'homomorphism_on_all_pairs': hom,
                                       'a': '(1 2)(3 4)', 'b': '(1 3 5)',
                                       'table': [[list(x), list(phi[x])] for x in sorted(phi)]}
    detail['F_pass'] = bool(ok)
    res['orbits'][o]['F'] = detail
# ---- F1 in these coordinates: nu_o(generator) = 0 for all 1,200 primitives and 35 orbits ---
slot_label = np.full(NS, -1, np.int64)
for X, lx in LAB.items():
    slot_label[lx] = np.arange(len(lx))
need(np.all((slot_label >= 0) == (oid[sp] >= 0)), 'F1 labels cover every moving sticker')
ab = {}
for o, H in Hgroups.items():
    typ = TABLE[o][2]
    if typ == 'C2':
        ab[o] = {h: (0 if h == tuple(range(len(h))) else 1) for h in H}, 2
    elif typ == 'D5':
        ab[o] = {h: (1 if pord(h) == 2 else 0) for h in H}, 2
    elif typ == 'C5':
        r0 = min(h for h in H if pord(h) == 5)
        m, x = {}, tuple(range(len(r0)))
        for kk in range(5):
            m[x] = kk
            x = pmul(x, r0)
        ab[o] = m, 5
    else:
        ab[o] = {h: 0 for h in H}, 1
nu_bad = 0
for i in range(1200):
    s, d = src[off[i]:off[i + 1]], dst[off[i]:off[i + 1]]
    ps, pd = sp[s], sp[d]
    for o in np.unique(oid[ps]).tolist():
        if o < 0:
            continue
        sel = oid[ps] == o
        mp = dict(zip(ps[sel].tolist(), pd[sel].tolist()))
        # sign of the piece permutation (fixed pieces contribute nothing)
        seen_p, parity = set(), 0
        for x in mp:
            if x in seen_p:
                continue
            n, y = 0, x
            while y not in seen_p:
                seen_p.add(y)
                y = mp.get(y, y)
                n += 1
            parity ^= (n - 1) & 1
        # frame values on every touched piece, including pieces turned in place
        img = dict(zip(s[sel].tolist(), d[sel].tolist()))
        tot = 0
        tab, mod = ab[o]
        for x, y in mp.items():
            if x not in LAB or y not in LAB:
                nu_bad += 1
                continue
            lx = LAB[x]
            v = tuple(int(slot_label[img.get(int(q), int(q))]) for q in lx)
            if not np.array_equal(np.sort(LAB[y][list(v)]), np.sort(LAB[y])) or v not in tab:
                nu_bad += 1
                continue
            tot += tab[v]
        if parity or tot % mod:
            nu_bad += 1
need(nu_bad == 0, 'F1 nu failures %d' % nu_bad)
res['checks']['F1_nu_zero_on_generators'] = {'primitives': 1200, 'orbits': 35, 'failures': nu_bad}
need(sorted(F['orbit'] for F in cert['F']) == [0, 3, 6, 8, 11, 17, 22, 25, 28, 32, 34], 'F orbit list')
need(sorted(P['orbit'] for P in cert['P']) == list(range(35)), 'P orbit list')
res['failures'] = fail
res['pass'] = not fail
res['seconds'] = round(time.time() - T0, 1)
Path(out_path).write_text(json.dumps(res, indent=1))
print('PASS' if not fail else 'FAIL', len(fail), fail[:10], res['seconds'], 's')
