"""Type-S finishing words for B' after a restoring P (reads only the state's pose table).

After P, B' is on K in chambers L, and the target Bad(P^-1) = T is BH (bad holes) plus L. The SAFE-ALL word Q' at the
state after P must end with holes exactly T. Split Q' = Q0 F:
- Q0 uses only H0 letters (SAFE-ALL grips whose caps avoid T), so it fixes T pointwise; it routes the good holes GH to
  chambers N outside T.
- F is a short SAFE-ALL word touching L with sigma_F(BH) inside T; then N = sigma_F^-1(T) minus BH must be outside T,
  and Q0 must bring the holes onto N.
This script lists, for single letters and commutators [x, y] (x touching L), the condition sigma_F(BH) in T, the set N,
whether N avoids T, and the H0 components of N against those of GH. Exact chamber permutations only.

Usage: python -I finish.py <theory dir> <pose table> <P json word> <out json> [--comm]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def components(G, grips):
    parent = np.arange(G.NP)

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    gs = set(grips)
    for c, _, _, ys in G.letters:
        if c not in gs:
            continue
        for a, b in zip(G.capx[c].tolist(), ys.tolist()):
            ra, rb = find(a), find(b)
            if ra != rb:
                parent[max(ra, rb)] = min(ra, rb)
    return np.array([find(x) for x in range(G.NP)])


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[1])
    R, M, st = C.R, C.M, C.st
    off0 = C.off.copy()
    P = [tuple(x) for x in json.loads(a[2])]
    badPinv, _, _ = C.bad(CP.inverse_word(M, P))
    for c, i in P:
        R.apply(c, i)
    G = B.Alphabet(R, M)
    G.orbits()
    holes = np.asarray(G.holes)
    T = np.zeros(G.NP, bool)
    T[badPinv] = True
    BH = holes[T[holes]]
    GH = holes[~T[holes]]
    isBH = np.zeros(G.NP, bool)
    isBH[BH] = True
    from blocks import CL
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    Kp = st._kpose()[st.pose_id]
    L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
    isL = np.zeros(G.NP, bool)
    isL[L] = True
    h0 = [c for c in G.safe if not T[G.capx[c]].any()]
    lab0 = components(G, h0)
    gh_comp = sorted(int(lab0[x]) for x in GH)
    log(T=int(T.sum()), BH=len(BH), GH=GH.tolist(), L=L.tolist(), h0=len(h0), gh_comp=gh_comp)
    lcaps = [c for c in G.safe if isL[G.capx[c]].any()]
    capinfo = [{'grip': c, 'size': int(len(G.capx[c])), 'L': int(isL[G.capx[c]].sum()),
                'BH': int(isBH[G.capx[c]].sum()), 'letters': len(G.by_grip[c])} for c in lcaps]
    log(lcaps=capinfo)

    def judge(word):
        caps = sorted({G.letters[n][0] for n in word})
        U = np.unique(np.concatenate([G.capx[c] for c in caps]))
        img = G.images(word, U)
        bh_ok = bool(T[img[isBH[U]]].all())
        into_T = U[T[img]]
        N = into_T[~isBH[into_T]]
        L_out = int((~T[img[isL[U]]]).sum())
        return {'bh_into_T': bh_ok, 'N': N.tolist(), 'N_in_L': int(isL[N].sum()), 'L_in_support': int(isL[U].sum()),
                'L_moved_out': L_out, 'N_comp': sorted(int(lab0[x]) for x in N),
                'bh_out': int((~T[img[isBH[U]]]).sum())}

    res = {'after_P_digest': st.digest(), 'P': [list(x) for x in P], 'T': int(T.sum()), 'BH': len(BH),
           'GH': GH.tolist(), 'L': L.tolist(), 'h0_grips': len(h0), 'GH_h0_comp': gh_comp,
           'L_caps': capinfo, 'singles': [], 'commutators': []}
    X = [n for c in lcaps for n in G.by_grip[c]]
    for x in X:
        j = judge([x])
        res['singles'].append(dict(letter=G.pairs([x])[0], **{k: v for k, v in j.items() if k != 'N'},
                                   N=j['N'] if len(j['N']) <= 10 else len(j['N'])))
    best = sorted(res['singles'], key=lambda r: r['bh_out'])[:5]
    log(singles=len(res['singles']), bh_into_T=sum(r['bh_into_T'] for r in res['singles']),
        best=[(r['letter'], r['bh_out'], r['L_moved_out']) for r in best])
    if '--comm' in a:
        good = 0
        for k, x in enumerate(X):
            cx = G.letters[x][0]
            ys = [n for c in G.safe if c != cx and (G.home[G.capx[cx], c]).any() for n in G.by_grip[c]]
            for y in ys:
                for w in ([x, y, int(G.inv[x]), int(G.inv[y])], [y, x, int(G.inv[y]), int(G.inv[x])]):
                    j = judge(w)
                    if j['bh_into_T'] and j['N_in_L'] == 0 and j['L_moved_out'] > 0:
                        good += 1
                        res['commutators'].append(dict(word=G.pairs(w), **j))
            log(x=k, of=len(X), ys=len(ys), good=good)
    res['seconds'] = round(time.time() - T0, 1)
    Path(a[3]).write_text(json.dumps(res) + '\n')
    log(done=True, commutators=len(res['commutators']))


if __name__ == '__main__':
    main()
