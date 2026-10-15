"""Symbolic seed harvest by the vacancy formula (menu I_a): nested conjugates U r U^-1 tracked without replay.

State of a level: hole chambers H with base labels, the label map P (chamber -> piece there, -1 for a hole) and the base
labels of the off pieces. Base: a short word U0 from solved, replayed once; the ghost of every hole h of U0 (a phantom
in chamber h carried through U0^-1, rotating with each cap that contains it) gives the region G(h) and its position
class; holes whose phantom straddles a cut are dead.
Transition for a retained letter r = (g, a) whose cap holds no off piece of U (clean):
  refilled R = {h in H ∩ cap g : a^-1(h) not in H},  vacated V = {h in H ∩ cap g : a(h) not in H};
  U' = U r U^-1 has its off pieces at G(lab h), h in R; holes P[a(h)], h in V, with labels lab h;
  P' = P o r^-1 o P^-1 (homes of U's off pieces map to themselves).
If |R| = 1 the word U r U^-1 is a single whose off piece lies at G(lab h): its class is known before any replay.
Every single written is replayed from solved on the exact simulator (every twist admissible, exactly one off piece,
predicted class, piece and hole confirmed).
usage: harvest.py <research/jumbling> <classes.json> <arrays.npz> <out jsonl> <rng seed> <budget s>
                  [--have seeds.json] [--per K] [--beam B] [--levels N] [--rmax R] [--base j|comm] [--want classes.json]"""
import argparse
import json
import random
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from e3lib import REGNAMES, Lib  # noqa: E402

ap = argparse.ArgumentParser()
for x in ('J', 'classes', 'arrays', 'out'):
    ap.add_argument(x)
ap.add_argument('seed', type=int), ap.add_argument('budget', type=float)
ap.add_argument('--have', default=None), ap.add_argument('--per', type=int, default=3)
ap.add_argument('--beam', type=int, default=24), ap.add_argument('--levels', type=int, default=6)
ap.add_argument('--rmax', type=int, default=400), ap.add_argument('--base', default='mixed')
ap.add_argument('--want', default=None)
a = ap.parse_args()
t0 = time.time()
rng = random.Random(a.seed)
L = Lib(a.J, a.classes, a.arrays)
sim, d, reg, kp = L.sim, L.d, L.reg, L.kp
NF = np.array(d.NF, float)
KAP = float(d.KAPPA)
TOL = 1e-7
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in reg.rep_vertices]
NP = L.NP
possible = {int(c) for c in set(L.cls_of.tolist()) if not L.core_bad[c]}
if a.want:
    possible &= {int(c) for c in json.load(open(a.want))}
have = Counter()
if a.have:
    for q in json.load(open(a.have)):
        have[q['class']] += 1
TW = {}


def tw(l):
    l = tuple(l)
    if l not in TW:
        TW[l] = sim.Twist(L.ctx, l[0], L.RS.element(L.M, l[0], l[1]), 'menu', {'menu': L.M.name})
    return TW[l]


def winv(w):
    return [L.inv_letter(tuple(l)) for l in reversed(w)]


def home_vertices(p):
    return repV[L.orb[p]] @ L.Kf[L.tr[p]].T


def status_rows(vlist, grips=None):
    """0 out, 1 in, 2 straddle/unclear, for each region (list of float vertex arrays) and each grip."""
    G = NF if grips is None else NF[grips]
    out = np.empty((len(vlist), len(G)), np.int8)
    for n, V in enumerate(vlist):
        vals = V @ G.T - KAP
        ins = (vals > -TOL).all(0)
        ou = (vals < TOL).all(0)
        out[n] = np.where(ins & ~ou, 1, np.where(ou & ~ins, 0, 2))
    return out


def canon_pose(Xf):
    A = np.einsum('ij,kjl,lm->kim', L.Gi, L.Kf, Xf)
    keys = np.round(A, 6).reshape(7200, 16)
    for k in range(7200):
        rr = L.Kidx.get(tuple(keys[k].tolist()))
        if rr is not None:
            return k, rr
    return None


RETL = {}


def retained(g):
    if g not in RETL:
        RETL[g] = [((int(g), i), L.kidx_letter(int(g), i), L.kidx_letter(*L.inv_letter((int(g), i)))) for i in L.RI]
    return RETL[g]


# ------------------------------------------------------------------ base
def make_base(kind):
    JI, RI = L.JI, L.RI
    if kind == 'j':
        return [(0, rng.choice(JI))]
    b = 0
    near = [c for c in range(600) if c != b and L.cls[c] == L.cls[b] and 0.8 < float(NF[b] @ NF[c] / (NF[b] @ NF[b])) < 0.999]
    a1 = rng.choice(near)
    k1 = (a1, rng.choice(RI if rng.random() < 0.5 else JI))
    x = (b, rng.choice(JI))
    C0 = [k1, x, L.inv_letter(k1), L.inv_letter(x)]
    if rng.random() < 0.5:
        a2 = rng.choice(near)
        k2 = (a2, rng.choice(RI))
        C0 = [k2] + C0 + [L.inv_letter(k2)] + winv(C0)
    return C0


def base_state(U0):
    st = sim.State(L.ctx, menu=L.M)
    for l in U0:
        if not st.apply(tw(l), journal=False).applied:
            return None
    ch = L.lattice_chambers(st)
    P = np.full(NP, -1, np.int64)
    on = np.flatnonzero(ch >= 0)
    P[ch[on]] = on
    H = np.flatnonzero(P < 0)
    off = np.flatnonzero(ch < 0)
    if not len(H):
        return None
    offV = [home_vertices(p) @ L.fm(st.pose(int(p))).T for p in off.tolist()]
    # phantoms through U0^-1
    inv = winv(U0)
    els = [L.fm(L.RS.element(L.M, c, i)) for c, i in inv]
    alive, gpose = np.ones(len(H), bool), [np.eye(4) for _ in H]
    for n, h in enumerate(H.tolist()):
        X = np.eye(4)
        hv = home_vertices(h)
        for (c, i), E in zip(inv, els):
            vals = (hv @ X.T) @ NF[c] - KAP
            if (vals > -TOL).all():
                X = E @ X
            elif not (vals < TOL).all():
                alive[n] = False
                break
        gpose[n] = X
    # classes of ghost regions
    gcls = np.full(len(H), -1, np.int64)
    gpos = np.full(len(H), -1, np.int64)
    cache = {}
    for n in np.flatnonzero(alive).tolist():
        key = L._key(gpose[n])
        if key not in cache:
            cache[key] = canon_pose(gpose[n])
        c = cache[key]
        if c is None:
            continue
        s = int(L.act(c[1], [H[n]])[0])
        gpos[n], gcls[n] = s, int(L.cls_of[s])
    gV = [home_vertices(int(h)) @ gpose[n].T for n, h in enumerate(H.tolist())]
    return {'U0': U0, 'st': st, 'P': P, 'H': H, 'off_status': status_rows(offV) if len(off) else np.zeros((0, 600), np.int8),
            'alive': alive, 'gcls': gcls, 'gpos': gpos, 'gstatus': status_rows(gV)}


# ------------------------------------------------------------------ levels
def clean_grips(B, offlabs, level):
    if level == 0:
        S = B['off_status']
    else:
        S = B['gstatus'][np.asarray(sorted(offlabs), np.int64)] if offlabs else np.zeros((0, 600), np.int8)
    return np.flatnonzero((S == 0).all(0)) if len(S) else np.arange(600)


def expand(B, node, wanted):
    """Yield (letter, R, V) for clean letters with holes in the cap."""
    H, lab = node['H'], node['lab']
    isH = np.zeros(NP, bool)
    isH[H] = True
    for g in clean_grips(B, node['off'], node['level']).tolist():
        inc = L.in_cap_chambers(g, H)
        if not inc.any():
            continue
        Hg = H[inc]
        lg = lab[inc]
        for letter, ka, kai in retained(g):
            pre = L.act(kai, Hg)
            img = L.act(ka, Hg)
            Rm = ~isH[pre]
            Vm = ~isH[img]
            if not Rm.any():
                continue
            yield letter, ka, kai, Hg[Rm], lg[Rm], Hg[Vm], lg[Vm], img[Vm]


def child(B, node, letter, ka, kai, Vh, Vl, Vimg, Rl):
    P = node['P']
    Q = np.full(NP, -1, np.int64)
    on = np.flatnonzero(P >= 0)
    Q[P[on]] = on
    g = letter[0]
    X = Q.copy()
    valid = X >= 0
    xs = X[valid]
    inc = L.in_cap_chambers(g, xs)
    xs2 = xs.copy()
    xs2[inc] = L.act(kai, xs[inc])
    Pn = np.arange(NP, dtype=np.int64)            # homes of U's off pieces map to themselves
    Pn[valid] = P[xs2]
    Hn = P[Vimg]
    assert (Hn >= 0).all()
    assert set(np.flatnonzero(Pn < 0).tolist()) == set(Hn.tolist())
    labn = np.empty(len(Hn), np.int64)
    labn[:] = Vl
    order = np.argsort(Hn)
    return {'H': Hn[order], 'lab': labn[order], 'P': Pn, 'off': set(int(x) for x in Rl), 'level': node['level'] + 1,
            'word': node['word'] + [letter] + winv(node['word'])}


def verify(word, pred_cls):
    st = sim.State(L.ctx, menu=L.M)
    for l in word:
        if not st.apply(tw(l), journal=False).applied:
            return None
    ch = L.lattice_chambers(st)
    off = np.flatnonzero(ch < 0)
    if len(off) != 1:
        return None
    q = int(off[0])
    c = L.canon(q, L.fm(st.pose(q)))
    if c is None:
        return None
    s = c[2]
    holes = L.vacancies(st)
    return {'off': [q], 'off_orbit': [int(L.orb[q])], 'holes': holes.tolist(), 'hole_orbit': [int(L.orb[h]) for h in holes],
            'class_replayed': int(L.cls_of[s]), 'class_predicted': int(pred_cls), 'position': int(s),
            # seed-row fields (as seeds.py writes them), so a harvest row is a seed row without another replay
            'piece': q, 'piece_orbit': int(L.orb[q]), 'k': int(c[0]), 'r': int(c[1]), 's': int(s),
            'hole': int(holes[0]) if len(holes) == 1 else None, 'class': int(L.cls_of[s]),
            'grips': sorted({int(l[0]) for l in word}), 'end_digest': st.digest()}


fout = open(a.out, 'a')
found = Counter()
nseed = 0
while time.time() - t0 < a.budget:
    U0 = make_base('j' if a.base == 'j' else ('comm' if a.base == 'comm' else rng.choice(['j', 'comm', 'comm'])))
    B = base_state(U0)
    if B is None:
        continue
    wanted = {c for c in possible if have[c] + found[c] < a.per}
    labs0 = np.arange(len(B['H']))
    base_cls = Counter(B['gcls'][B['alive'] & (B['gcls'] >= 0)].tolist())
    node0 = {'H': B['H'], 'lab': labs0, 'P': B['P'], 'off': set(), 'level': 0, 'word': list(U0)}
    beam = [node0]
    print('base', U0, 'holes', len(B['H']), 'alive', int(B['alive'].sum()), 'ghost classes', len(base_cls),
          'wanted among them', len(set(base_cls) & wanted), '%.0fs' % (time.time() - t0), flush=True)
    for level in range(a.levels):
        kids = []
        for node in beam:
            for letter, ka, kai, Rh, Rl, Vh, Vl, Vimg in expand(B, node, wanted):
                if not B['alive'][Rl].all():
                    continue
                rc = B['gcls'][Rl]
                if len(Rh) == 1 and rc[0] >= 0 and rc[0] in wanted and found[int(rc[0])] + have[int(rc[0])] < a.per:
                    word = node['word'] + [letter] + winv(node['word'])
                    v = verify(word, rc[0])
                    if v and v['class_replayed'] == v['class_predicted']:
                        found[int(rc[0])] += 1
                        nseed += 1
                        row = {'name': f'H{a.seed}-{nseed}', 'class_P': 0, 'length': len(word), 'word': [list(x) for x in word]}
                        row.update(v)
                        row['registry_orbit'] = REGNAMES[int(L.reg_code[v['position']])]
                        fout.write(json.dumps(row) + '\n')
                        fout.flush()
                        wanted = {c for c in possible if have[c] + found[c] < a.per}
                        print('SINGLE', row['name'], 'class', rc[0], row['registry_orbit'], 'len', len(word), 'level', level,
                              'classes found', len(found), '%.0fs' % (time.time() - t0), flush=True)
                    else:
                        print('mismatch or failed verify', v, flush=True)
                    continue
                if len(Rh) > a.rmax or not len(Vh):
                    continue
                vw = len({c for c in B['gcls'][Vl].tolist() if c in wanted})
                if not vw:
                    continue
                kids.append((len(Rh), -vw, rng.random(), node, letter, ka, kai, Vh, Vl, Vimg, Rl))
            if time.time() - t0 > a.budget:
                break
        if not kids or time.time() - t0 > a.budget:
            break
        kids.sort(key=lambda x: x[:3])
        seen, sel = set(), []
        for k in kids:
            key = (k[0], tuple(sorted(set(B['gcls'][k[8]].tolist()))))
            if key in seen:
                continue
            seen.add(key)
            sel.append(k)
            if len(sel) >= a.beam:
                break
        beam = [child(B, k[3], *k[4:]) for k in sel]
        print(' level', level + 1, 'children', len(kids), 'beam R', [k[0] for k in sel][:8], '%.0fs' % (time.time() - t0),
              flush=True)
print('done', 'singles', nseed, 'classes', len(found), dict(Counter(REGNAMES[int(L.reg_code[c])] for c in found)),
      '%.0fs' % (time.time() - t0))
