"""Search for retained loops with odd orbit parity (menu I_a): from a reachable start U (a word or a pose table),
breadth-first search over retained letters (g, a), a in A4_g, at grips that are admissible (no off piece straddles
cap g; float statuses with margin 1e-7) and whose cap holds at least one hole or off piece. Shapes (hole set and off
region set) are hashed (Zobrist). Two paths A, B from U reaching one shape give the retained loop R = A B^-1, and
by the generalized vacancy formula nu_o(U R U^-1) has sign sgn(rho_R on the orbit-o holes) * sgn(permutation of
the orbit-o off pieces). A loop with odd sign for a wanted orbit is reported for exact replay. Float search only;
every hit must be replayed exactly. usage: loopsearch.py <research/jumbling> <classes.json> <arrays.npz> <out json>
<wanted orbits comma> <depth> (--word g:i,g:i,... | --table <pose table>) [--orbit-relevant]"""
import json, sys, time
from collections import Counter
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'e3s'))
sys.path.insert(0, str(Path(__file__).resolve().parent))  # package layout: e3lib.py next to this script
from e3lib import Lib  # noqa: E402
t0 = time.time()
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
OUT = sys.argv[4]
WANT = [int(x) for x in sys.argv[5].split(',')]
DEPTH = int(sys.argv[6])
ORBREL = '--orbit-relevant' in sys.argv
d = L.d
oid = np.asarray(d.orbit_id)
NF = np.array(d.NF, float); KAP = float(d.KAPPA)
TOL = 1e-7
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in L.reg.rep_vertices]
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402
st = L.sim.State(L.ctx, menu=L.M)
if '--word' in sys.argv:
    word = [tuple(int(v) for v in w.split(':')) for w in sys.argv[sys.argv.index('--word') + 1].split(',')]
    for c, i in word:
        tw = L.sim.Twist(L.ctx, c, L.RS.element(L.M, c, i), 'menu', {'menu': L.M.name})
        assert st.apply(tw, journal=False).applied
    start = {'word': word}
else:
    doc = json.loads(Path(sys.argv[sys.argv.index('--table') + 1]).read_text())
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
    st._compact(); st.journal, st._anchor_cache = [], {}
    assert st.digest() == doc['digest']
    start = {'table_digest': doc['digest']}
K = st._kpose()[st.pose_id]
off = np.flatnonzero(K < 0)
H0 = L.vacancies(st)
offk, offs = [], []
for p in off.tolist():
    c = L.canon(p, L.fm(st.pose(p)))
    assert c is not None, 'off piece not depth one'
    offk.append(c[0]); offs.append(c[2])
offk = np.array(offk, np.int64); offs = np.array(offs, np.int64)
# statuses of gamma A_s for the positions present: 0 out, 1 in, 2 straddle
pos = sorted(set(offs.tolist()))
pidx = {s: i for i, s in enumerate(pos)}
ST = np.zeros((len(pos), 600), np.int8)
Gc = np.zeros((len(pos), 4))
for s, i in pidx.items():
    V = (repV[L.orb[s]] @ L.Kf[L.tr[s]].T) @ L.G.T
    vals = V @ NF.T - KAP
    ins = (vals > -TOL).all(0); out_ = (vals < TOL).all(0)
    ST[i] = np.where(ins & ~out_, 1, np.where(out_, 0, 2))
    ST[i][ins & out_] = 0
    Gc[i] = V.mean(0)
offsi = np.array([pidx[s] for s in offs.tolist()], np.int64)
rng = np.random.default_rng(7)
ZH = rng.integers(0, 2**63 - 1, L.NP, dtype=np.int64).astype(np.uint64)


def regkey(k, si):
    c = L.Kf[k].T @ Gc[si] if np.ndim(k) == 0 else np.einsum('nji,nj->ni', L.Kf[k], Gc[si])
    q = np.round(c * 1e4).astype(np.int64)
    return (q[..., 0] * 1000003 + q[..., 1] * 7919 + q[..., 2] * 104729 + q[..., 3] * 15485863) ^ 0x5DEECE66D


def shash(holes, ks):
    with np.errstate(over='ignore'):
        r = regkey(ks, offsi).astype(np.uint64) * np.uint64(0x9E3779B97F4A7C15)
    return int(np.bitwise_xor.reduce(ZH[holes])) ^ int(np.bitwise_xor.reduce(r))


stab = {g: [int(k) for k in np.flatnonzero(L.perms[:, g] == g) if k != 0] for g in range(600)}
W5h = {o: np.array([h for h in H0.tolist() if oid[h] == o], np.int64) for o in WANT}
W5p = {o: np.flatnonzero(oid[off] == o) for o in WANT}


def replay(path):
    holes = H0.copy(); ks = offk.copy()
    trk = {o: W5h[o].copy() for o in WANT}
    for g, a in path:
        stat = ST[offsi, L.perms[ks, g]]
        assert not (stat == 2).any()
        inside = stat == 1
        ks[inside] = L.kp.compose_vec(ks[inside], np.full(int(inside.sum()), L.inv[a]))
        hin = d.in_sig(holes, np.full(len(holes), g))
        holes = holes.copy(); holes[hin] = L.act(a, holes[hin])
        for o in WANT:
            t = trk[o]; tin = d.in_sig(t, np.full(len(t), g))
            t[tin] = L.act(a, t[tin])
    return holes, ks, trk


def sign(src, dst):
    m = dict(zip(src.tolist(), dst.tolist()))
    assert len(m) == len(src) and set(m) == set(m.values())
    seen, sg = set(), 1
    for a in m:
        if a in seen:
            continue
        n, b = 0, a
        while b not in seen:
            seen.add(b); b = m[b]; n += 1
        if n % 2 == 0:
            sg = -sg
    return sg


nodes = [{'path': [], 'hash': shash(H0, offk)}]
seen = {nodes[0]['hash']: 0}
hits, frontier, stats = [], [0], Counter()
for depth in range(DEPTH):
    nxt = []
    for ni in frontier:
        holes, ks, trk = replay(nodes[ni]['path'])
        stat = ST[offsi[:, None], L.perms[ks[:, None], np.arange(600)[None, :]]]
        strad = (stat == 2).any(0)
        hasin = (stat == 1).any(0)
        hm = np.unpackbits(d.sigbits[holes], axis=1)[:, :600].astype(bool)
        hashole = hm.any(0)
        cand = np.flatnonzero(~strad & (hashole | hasin))
        if ORBREL:
            rel = np.zeros(600, bool)
            for o in WANT:
                rel |= np.unpackbits(d.sigbits[trk[o]], axis=1)[:, :600].astype(bool).any(0)
                po = W5p[o]
                rel |= (stat[po] == 1).any(0)
            cand = cand[rel[cand]]
        stats['expanded'] += 1
        for g in cand.tolist():
            for a in stab[g]:
                path = nodes[ni]['path'] + [(g, a)]
                h2, k2, t2 = replay(path) if False else (None, None, None)
                inside = stat[:, g] == 1
                k2 = ks.copy(); k2[inside] = L.kp.compose_vec(ks[inside], np.full(int(inside.sum()), L.inv[a]))
                hin = hm[:, g]
                h2 = holes.copy(); h2[hin] = L.act(a, holes[hin])
                hv = shash(h2, k2)
                if hv in seen:
                    stats['collisions'] += 1
                    other = nodes[seen[hv]]
                    _, ko, to = replay(other['path'])
                    t2 = {o: trk[o].copy() for o in WANT}
                    for o in WANT:
                        tin = d.in_sig(t2[o], np.full(len(t2[o]), g)); t2[o][tin] = L.act(a, t2[o][tin])
                    for o in WANT:
                        po = W5p[o]
                        sh = sign(to[o], t2[o]) if len(po) or len(t2[o]) else 1
                        sp = sign(regkey(ko[po], offsi[po]), regkey(k2[po], offsi[po])) if len(po) else 1
                        stats[f'o{o} sign {sh * sp} (holes {sh}, off {sp})'] += 1
                        if sh * sp == -1:
                            hits.append({'orbit': o, 'A': [list(x) for x in path], 'B': [list(x) for x in other['path']],
                                         'hole sign': sh, 'off sign': sp})
                            print('HIT', hits[-1], flush=True)
                    continue
                seen[hv] = len(nodes)
                nodes.append({'path': path, 'hash': hv})
                nxt.append(len(nodes) - 1)
    frontier = nxt
    print('depth', depth + 1, 'nodes', len(nodes), 'frontier', len(frontier), dict(stats), round(time.time() - t0), flush=True)
    json.dump({'start': start, 'wanted': WANT, 'depth_done': depth + 1, 'nodes': len(nodes), 'stats': dict(stats),
               'hits': hits, 'orbit_relevant_only': ORBREL}, open(OUT, 'w'), indent=1)
