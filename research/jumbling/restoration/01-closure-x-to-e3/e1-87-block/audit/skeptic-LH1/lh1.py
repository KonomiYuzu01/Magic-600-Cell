"""Skeptic check of finding LH-1 (lower bound |Bad(U)| >= restored pieces; Bad(U) = holes for a clean full restoration).

Independent pieces (written for this audit; the ghost of ghostlib/fastghost is used only as the thing compared):
- holes: chamber of each lattice piece located geometrically (float centroid of the posed home region, signature over
  all 600 cuts with a margin, signature -> chamber by the packed signature bits); K+ membership of each pose by a float
  pole-set test, cross-checked with the simulator's flag;
- exact tracker: exact vertices of chosen regions followed through a word, inside/outside/straddle by float screen
  with exact Q(sqrt5) fallback for every vertex near a cut; at the end K+ membership by an exact pole-set test;
- pole fix (M3): every menu element of every grip fixes n_c exactly.

Usage: python3 -I lh1.py <theory dir> <v26 dir> <mode> <out json> [args]
  modes: polefix | holes <table> | testA <table> <word json> <start> | testB <E1 table> <E2 table> <parts dir>
"""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
V26 = Path(sys.argv[2]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(V26))
import blocks as BL  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import Q5, ZERO  # noqa: E402

CTX = BL.CL.ctx
data, reg, kp = CTX.data, CTX.regions, CTX.kplus
restore = BL.restore
NP = 177120
S5 = 5 ** 0.5
NFl = np.array([[float(x) for x in row] for row in data.N])          # float poles (600, 4)
KAP = float(data.KAPPA)
NNf = float(data.NN)
T0 = time.time()
MARGIN = 1e-9


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


# ---------------------------------------------------------------- geometry helpers
_SIGD = None


def sig_dict():
    global _SIGD
    if _SIGD is None:
        _SIGD = {bytes(data.sigbits[z]): z for z in range(NP)}
        assert len(_SIGD) == NP
    return _SIGD


def locate(points, batch=20000):
    """Chamber index of each float point (or -1 when outside the body / ambiguous), the ambiguity count and the count
    of points outside the body."""
    d = sig_dict()
    out = np.full(len(points), -1, np.int64)
    namb = nout = 0
    for a in range(0, len(points), batch):
        P = points[a:a + batch]
        dots = P @ NFl.T
        s = dots - KAP
        amb = (np.abs(s) < MARGIN).any(axis=1)
        inbody = (dots <= NNf - MARGIN).all(axis=1)
        bits = np.packbits(s > 0, axis=1)
        o = np.array([d.get(bytes(b), -1) for b in bits], np.int64)
        o[amb | ~inbody] = -1
        out[a:a + batch] = o
        namb += int(amb.sum())
        nout += int((~inbody).sum())
    return out, namb, nout


_HC = None


def home_centroids():
    """Float centroid of every home chamber, from the exact representative vertices and K+ transport; checked to lie
    in the chamber with that index (signature equality)."""
    global _HC
    if _HC is not None:
        return _HC
    cache = Path(__file__).resolve().parent / 'home-centroids.npy'
    if cache.exists():
        c = np.load(cache)
        where, amb, outside = locate(c)
        assert np.array_equal(where, np.arange(NP)) and amb == 0 and outside == 0
        log(home_centroids='cache, rechecked in own chamber')
        _HC = c
        return c
    repc = np.array([np.mean([[float(x) for x in v] for v in vs], axis=0) for vs in reg.rep_vertices])
    Kf = np.zeros((7200, 4, 4))
    for i in range(7200):
        g1, g2, dd = kp.int_matrix(i)
        Kf[i] = (g1 + g2 * S5) / dd
    c = np.einsum('nij,nj->ni', Kf[reg.transport], repc[reg.orbit_of])
    where, amb, outside = locate(c)
    ok = bool(np.array_equal(where, np.arange(NP)))
    log(home_centroids_in_own_chamber=ok, ambiguous=amb, outside=outside)
    assert ok
    np.save(cache, c)
    _HC = c
    return c


def fmat(entry):
    return np.array([[(a + b * S5) / d for a, b, d in row] for row in entry])


def qmat(entry):
    return [[Q5(a, b, d) for a, b, d in row] for row in entry]


def pole_test_float(F):
    """True when F maps the float pole set onto itself: every image has dot product |n|^2 with some pole (all poles
    have the same length, so that pole is the image). Returns (flag, max deviation)."""
    D = (NFl @ F.T) @ NFl.T
    err = np.abs(NNf - D.max(axis=1))
    return bool((err < 1e-9).all()), float(err.max())


def in_kplus_exact(H):
    """Exact: H maps every pole onto a pole (H is a product of rotations, so det H = 1)."""
    for n in data.N:
        img = tuple(sum((H[r][k] * n[k] for k in range(4)), ZERO) for r in range(4))
        if img not in data.POLE:
            return False
    return True


def state(table):
    doc = json.loads(Path(table).read_text())
    poses = doc['poses']
    pid = np.asarray(doc['pose_id'], np.int64)
    used = np.unique(pid)
    onk = {}
    worst_on, best_off = 0.0, 9.9
    for j in used.tolist():
        ok, err = pole_test_float(fmat(poses[j]))
        onk[j] = ok
        if ok:
            worst_on = max(worst_on, err)
        else:
            best_off = min(best_off, err)
    lat = np.array([onk[j] for j in pid.tolist()], bool)
    hc = home_centroids()
    F = {j: fmat(poses[j]) for j in used.tolist()}
    pos = np.zeros((NP, 4))
    for j in used.tolist():
        sel = np.flatnonzero(pid == j)
        pos[sel] = hc[sel] @ F[j].T
    ch, amb, outside = locate(pos[lat])
    assert (ch >= 0).all(), 'a lattice piece did not locate'
    assert len(np.unique(ch)) == len(ch), 'two lattice pieces in one chamber'
    occ = np.zeros(NP, bool)
    occ[ch] = True
    holes = np.flatnonzero(~occ)
    off = np.flatnonzero(~lat)
    return {'doc': doc, 'pid': pid, 'lat': lat, 'off': off, 'holes': holes, 'pose_float_err_on': worst_on,
            'pose_float_err_off_min': best_off, 'amb': amb}


# ---------------------------------------------------------------- exact tracker
class Track:
    def __init__(self, verts, start=None):
        self.v = [list(v) for v in verts]
        self.vf = np.array([[float(x) for x in v] for v in verts])
        self.h = [[Q5(1 if r == k else 0) for k in range(4)] for r in range(4)]
        self.hf = np.eye(4)
        self.start = start            # exact start pose for a real piece (None for a chamber content)
        self.alive, self.died_at, self.moves, self.exact = True, -1, 0, 0

    def side(self, c):
        s = self.vf @ (self.hf.T @ NFl[c]) - KAP
        pos, neg = s > MARGIN, s < -MARGIN
        if pos.any() and neg.any():
            return -1
        amb = np.flatnonzero(~(pos | neg))
        sg = set()
        if len(amb):
            self.exact += 1
            n = data.N[c]
            w = [sum((self.h[r][k] * n[r] for r in range(4)), ZERO) for k in range(4)]   # h^T n
            for j in amb.tolist():
                v = self.v[j]
                sg.add((sum((w[k] * v[k] for k in range(4)), ZERO) - data.KAPPA).sign())
        hp, hn = pos.any() or 1 in sg, neg.any() or -1 in sg
        if hp and hn:
            return -1
        if hp:
            return 1
        if hn:
            return 0
        raise AssertionError('full-dimensional region with all vertices on a cut')

    def move(self, E):
        self.h = [[sum((E[r][t] * self.h[t][k] for t in range(4)), ZERO) for k in range(4)] for r in range(4)]
        self.hf = np.array([[float(x) for x in row] for row in self.h])
        self.moves += 1


def run_tracks(tracks, word, M):
    els = {}
    for n, (c, i) in enumerate(word):
        if (c, i) not in els:
            els[(c, i)] = restore.element(M, c, i)
        E = els[(c, i)]
        for t in tracks:
            if not t.alive:
                continue
            sd = t.side(c)
            if sd < 0:
                t.alive, t.died_at = False, n
            elif sd == 1:
                t.move(E)


def finish(t):
    H = t.h
    if t.start is not None:
        S = t.start
        H = [[sum((t.h[r][k] * S[k][col] for k in range(4)), ZERO) for col in range(4)] for r in range(4)]
    onk = t.alive and in_kplus_exact(H)
    end = None
    if onk:
        Hf = np.array([[float(x) for x in row] for row in t.hf])
        cen = t.vf.mean(axis=0) @ Hf.T
        e, _, _ = locate(cen[None, :])
        end = int(e[0])
    return {'alive': t.alive, 'died_at': t.died_at, 'on_K': bool(onk), 'end_chamber': end, 'moves': t.moves,
            'exact_calls': t.exact}


def chamber_track(z):
    return Track([list(v) for v in reg.vertices(int(z))])


def piece_track(p, entry):
    m = qmat(entry)
    vs = [[sum((m[r][k] * v[k] for k in range(4)), ZERO) for r in range(4)] for v in reg.vertices(int(p))]
    return Track(vs, start=None), m


# ---------------------------------------------------------------- modes
def mode_polefix(out):
    import seams
    _, M = seams._fixture_state('I_a', 40)
    bad, n = [], 0
    for c in range(600):
        nc = data.N[c]
        for i in range(len(M.items)):
            E = restore.element(M, c, i)
            img = [sum((E[r][k] * nc[k] for k in range(4)), ZERO) for r in range(4)]
            n += 1
            if any(not (img[r] - nc[r]).is_zero() for r in range(4)):
                bad.append([c, i])
        if c % 100 == 99:
            log(grips=c + 1, checked=n, bad=len(bad))
    res = {'menu_items': len(M.items), 'checked': n, 'not_fixing_pole': len(bad), 'examples': bad[:5]}
    Path(out).write_text(json.dumps(res, indent=1) + '\n')
    log(done=out, **{k: v for k, v in res.items() if k != 'examples'})


def mode_holes(out, table):
    s = state(table)
    R, M = BL.O.start(table)
    simflag = R.st._kpose()[R.st.pose_id] >= 0
    res = {'table': table, 'digest_file': s['doc']['digest'], 'digest_sim': R.st.digest(),
           'off': s['off'].tolist(), 'holes': s['holes'].tolist(),
           'lattice_flag_equals_simulator': bool(np.array_equal(simflag, s['lat'])),
           'pose_float_err_on_max': s['pose_float_err_on'], 'pose_float_err_off_min': s['pose_float_err_off_min']}
    Path(out).write_text(json.dumps(res, indent=1) + '\n')
    log(done=out, off=len(res['off']), holes=res['holes'][:10], flag_eq=res['lattice_flag_equals_simulator'])


def mode_testA(out, table, wfile, start):
    s = state(table)
    word = [(int(c), int(i)) for c, i in json.loads(Path(wfile).read_text())['word']]
    U = word[start:]
    log(off=s['off'].tolist(), holes=s['holes'].tolist(), U=len(U))
    C = GL.Ctx(table)
    simflag = C.st._kpose()[C.st.pose_id] >= 0
    t = time.time()
    bad, g, ch = C.bad(U, chambers=np.arange(NP, dtype=np.int64))
    gsec = round(time.time() - t, 1)
    good = g.alive & (g.dom == 0)
    ends = g.q[good]
    log(ghost_bad=bad.tolist(), ghost_sec=gsec)
    # exact tracker: the holes, the off pieces, and a seeded sample of chambers in the union of U's caps
    grips = sorted({c for c, _ in U})
    union = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]))
    rng = random.Random(20261012)
    sample = sorted(rng.sample(sorted(set(union.tolist()) - set(s['holes'].tolist())), 40))
    tracks, kinds = [], []
    for z in s['holes'].tolist() + sample:
        tracks.append(chamber_track(z))
        kinds.append(('chamber', z))
    for p in s['off'].tolist():
        tr, m = piece_track(p, s['doc']['poses'][int(s['pid'][p])])
        tr.start = m
        tracks.append(tr)
        kinds.append(('piece', p))
    t = time.time()
    run_tracks(tracks, U, C.M)
    tsec = round(time.time() - t, 1)
    rows = []
    agree = 0
    for (kind, z), tr in zip(kinds, tracks):
        r = dict(kind=kind, id=z, **finish(tr))
        if kind == 'chamber':
            gi = int(z)
            r['ghost'] = {'alive': bool(g.alive[gi]), 'on_K': bool(good[gi]),
                          'end_chamber': int(g.q[gi]) if good[gi] else None}
            same = (r['ghost']['alive'] == r['alive'] and r['ghost']['on_K'] == r['on_K']
                    and r['ghost']['end_chamber'] == r['end_chamber'])
            r['agree'] = same
            agree += int(same)
        rows.append(r)
    hs, bs = set(s['holes'].tolist()), set(bad.tolist())
    res = {'table': table, 'word': wfile, 'start': start, 'U_length': len(U),
           'digest_file': s['doc']['digest'], 'digest_sim': C.st.digest(),
           'off_geometric': s['off'].tolist(), 'holes_geometric': s['holes'].tolist(),
           'lattice_flag_equals_simulator': bool(np.array_equal(simflag, s['lat'])),
           'ghost_bad_all_chambers': sorted(bs), 'ghost_bad_equals_holes': bs == hs,
           'ghost_good_end_chambers_distinct': bool(len(np.unique(ends)) == len(ends)),
           'ghost_good_count': int(good.sum()),
           'ghost_seconds': gsec, 'track_seconds': tsec,
           'tracked_chambers_agree_with_ghost': f'{agree}/{len(s["holes"]) + len(sample)}',
           'tracked_off_pieces_end_on_K': [r['on_K'] for r in rows if r['kind'] == 'piece'],
           'tracked_holes': [r for r in rows if r['kind'] == 'chamber' and r['id'] in hs],
           'tracked_pieces': [r for r in rows if r['kind'] == 'piece'],
           'tracked_sample': [r for r in rows if r['kind'] == 'chamber' and r['id'] not in hs]}
    Path(out).write_text(json.dumps(res, indent=1) + '\n')
    log(done=out, bad_eq_holes=res['ghost_bad_equals_holes'], agree=res['tracked_chambers_agree_with_ghost'],
        pieces_on_K=res['tracked_off_pieces_end_on_K'])


def mode_testB(out, t1, t2, parts):
    s1, s2 = state(t1), state(t2)
    U = []
    for name in ('V', 'P', 'Q', 'Pinv', 'Vinv'):
        U += [(int(c), int(i)) for c, i in json.loads((Path(parts) / f'{name}.json').read_text())['word']]
    log(off1=len(s1['off']), holes1=len(s1['holes']), off2=s2['off'].tolist(), holes2=s2['holes'].tolist(), U=len(U))
    C = GL.Ctx(t1)
    t = time.time()
    bad, g, ch = C.bad(U, chambers=np.arange(NP, dtype=np.int64))
    gsec = round(time.time() - t, 1)
    good = g.alive & (g.dom == 0)
    h1, h2, bs = set(s1['holes'].tolist()), set(s2['holes'].tolist()), set(bad.tolist())
    goodholes = sorted(h1 - bs)
    ends_ghost = sorted(int(g.q[z]) for z in goodholes)
    # exact tracker for the good holes and three bad holes
    badholes = sorted(h1 & bs)[:3]
    tracks = [chamber_track(z) for z in goodholes + badholes]
    t = time.time()
    run_tracks(tracks, U, C.M)
    tsec = round(time.time() - t, 1)
    rows = [dict(id=z, **finish(tr)) for z, tr in zip(goodholes + badholes, tracks)]
    T = set(s1['off'].tolist()) - set(s2['off'].tolist())
    res = {'E1_table': t1, 'E2_table': t2, 'U_length': len(U),
           'E1_off': len(s1['off']), 'E1_holes': len(s1['holes']), 'E2_off_geometric': s2['off'].tolist(),
           'E2_holes_geometric': s2['holes'].tolist(), 'restored_count': len(T),
           'bad_size': len(bs), 'bad_subset_E1_holes': bs <= h1, 'bad_minus_holes': sorted(bs - h1)[:10],
           'good_E1_holes': goodholes, 'good_E1_holes_ghost_end': ends_ghost,
           'good_holes_end_in_E2_holes_ghost': set(ends_ghost) <= h2,
           'tracked_exact': rows, 'ghost_seconds': gsec, 'track_seconds': tsec}
    Path(out).write_text(json.dumps(res, indent=1) + '\n')
    log(done=out, bad=len(bs), subset=res['bad_subset_E1_holes'], good=goodholes, ends=ends_ghost)


def main():
    mode, out = sys.argv[3], sys.argv[4]
    a = sys.argv[5:]
    if mode == 'polefix':
        mode_polefix(out)
    elif mode == 'holes':
        mode_holes(out, a[0])
    elif mode == 'testA':
        mode_testA(out, a[0], a[1], int(a[2]))
    elif mode == 'testB':
        mode_testB(out, a[0], a[1], a[2])
    else:
        raise SystemExit('unknown mode')


if __name__ == '__main__':
    main()
