"""Audit (math): the hypotheses of Theorem B for the target Bad(W) at E2, recomputed without buffer.Transport.

- SAFE-ALL grips at E2 by direct classification of all 600 grips; letters = the non-identity K+ menu elements.
- Orbits (BFS over all letters) of the targets and of the holes; N1 (no target fixed, or fixed targets vacant) and N2.
- For each certificate row j (order of certificate.json): E_j letters are SAFE-ALL letters; pi_{E_j} fixes
  P_j = {t_1..t_(j-1)} pointwise; y_j = pi_{E_j}^-1(t_j) equals the recorded y and is outside T; the orbit of y_j
  under the letters whose caps avoid P_j (each fixes P_j pointwise), its eccentricity e_j and u_j; h_Delta.
- Sufficiency: holes per orbit >= h_Delta; bound sum (|E_j| + e_j).

Usage: python3 -I thmb.py <theory dir> <v26 dir> <E2 table> <certificate json> <pre json> <out json>
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
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import blocks as BL  # noqa: E402
import ghostlib as GL  # noqa: E402
import restore  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

CL, O = BL.CL, BL.O
reg, kp, data = CL.ctx.regions, CL.ctx.kplus, CL.ctx.data
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def bfs(NP, start, letters):
    mask = np.zeros(NP, bool)
    mask[start] = True
    dist = np.full(NP, -1, np.int64)
    dist[start] = 0
    frontier = np.array([start], np.int64)
    d = 0
    while len(frontier):
        d += 1
        fm = np.zeros(NP, bool)
        fm[frontier] = True
        new = np.zeros(NP, bool)
        for xs, ys in letters:
            sel = fm[xs]
            if sel.any():
                new[ys[sel]] = True
        new &= ~mask
        frontier = np.flatnonzero(new)
        mask |= new
        dist[frontier] = d
    return mask, dist


def main():
    a = sys.argv[1:]
    table, certf, pref, outp = a[2], a[3], a[4], Path(a[5])
    cert = json.loads(Path(certf).read_text())
    pre = json.loads(Path(pref).read_text())
    R, M = O.start(table)
    st = R.st
    NP = len(st.pose_id)
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    ch = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    occ = np.zeros(NP, bool)
    occ[ch] = True
    holes = np.flatnonzero(~occ)
    safe, blocked = [], {}
    for c in range(600):
        cl = st.classify(c, need_inside=True, stop_at_straddle=False)
        inside = np.asarray(cl.inside, np.int64)
        offin = int((K[inside] < 0).sum()) if len(inside) else 0
        if cl.uncertain:
            raise AssertionError('uncertain')
        if cl.straddle is None and not offin:
            safe.append(c)
        else:
            blocked[c] = {'straddled': cl.straddle is not None, 'off_inside': offin}
    kid = kp.index_of_matrix(to_tuple(identity(4)))
    letters, index = [], {}
    capx = {}
    for c in safe:
        xs = np.flatnonzero(GL.cap(c))
        capx[c] = xs
        for i in range(len(M.items)):
            k = kp.index_of_matrix(restore.element(M, c, i))
            if k < 0 or k == kid:
                continue
            ys = reg.piece_of[reg.orbit_of[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
            index[(c, i)] = len(letters)
            letters.append((c, i, xs, ys))
    res = {'holes': holes.tolist(), 'safe_grips': len(safe), 'letters': len(letters),
           'blocked': {str(k): v for k, v in blocked.items()},
           'safe_equal_pre': safe == pre['menu']['safe_grips'],
           'blocked_equal_pre': {str(k): v for k, v in blocked.items()} == pre['menu']['blocked']}
    log(safe=len(safe), letters=len(letters))
    allL = [(xs, ys) for _, _, xs, ys in letters]
    T = np.array(cert['target_chambers'], np.int64)
    orb = {}
    for x in sorted(set(T.tolist()) | set(holes.tolist())):
        m, dist = bfs(NP, x, allL)
        orb[x] = (m, dist)
    res['orbit_sizes'] = {str(x): int(m.sum()) for x, (m, _) in orb.items()}
    # group targets and holes by orbit
    reps = {}
    for x, (m, _) in orb.items():
        key = int(np.flatnonzero(m)[0])
        reps.setdefault(key, []).append(x)
    res['orbit_classes'] = {str(k): v for k, v in reps.items()}
    by = []
    for key, members in reps.items():
        m = orb[members[0]][0]
        by.append({'least': key, 'size': int(m.sum()), 'targets': int(m[T].sum()), 'holes': int(m[holes].sum())})
    res['by_orbit'] = by
    res['N1_no_fixed_target'] = all(int(orb[int(t)][0].sum()) > 1 for t in T)
    res['N2'] = all(b['holes'] >= b['targets'] for b in by)
    rows = []
    Tset = set(T.tolist())
    P = []
    total = 0
    hdelta = {}
    for j, row in enumerate(cert['rows']):
        t = int(row['t'])
        Ew = [tuple(x) for x in row['word']]
        Ewords_ok = all(tuple(x) in index for x in Ew)
        p = np.arange(NP)
        for c, i in Ew:
            _, _, xs, ys = letters[index[(c, i)]]
            lp = np.arange(NP)
            lp[xs] = ys
            p = lp[p]
        fixesP = bool(all(p[q] == q for q in P))
        y = int(np.flatnonzero(p == t)[0])
        avoid = [(xs, ys) for c, i, xs, ys in letters if not GL.cap(c)[P].any()] if P else allL
        m, dist = bfs(NP, y, avoid)
        Dm = orb[t][0]
        Pm = np.zeros(NP, bool)
        Pm[P] = True
        Rj = m & Dm
        uj = int((Dm & ~Pm & ~Rj).sum() - (0 if Rj[t] else 1))
        ecc = int(dist[m].max())
        key = int(np.flatnonzero(Dm)[0])
        h = int(Pm[Dm].sum()) + 1 + uj
        hdelta[key] = max(hdelta.get(key, 0), h)
        total += len(Ew) + ecc
        rows.append({'t': t, 'E': [list(x) for x in Ew], 'E_letters_safe_all': Ewords_ok, 'E_fixes_P': fixesP,
                     'y': y, 'y_recorded': int(row['y']), 'y_outside_T': y not in Tset,
                     'avoid_letters': len(avoid), 'reach': int(m.sum()), 'reach_within_orbit': bool((m <= Dm).all()),
                     'orbit_minus_P': int((Dm & ~Pm).sum()), 'u_j': uj, 'ecc': ecc, 'ecc_recorded': row['ecc']})
        P.append(t)
    res['rows'] = rows
    res['h_delta'] = {str(k): v for k, v in hdelta.items()}
    res['sufficient'] = all(next(b['holes'] for b in by if b['least'] == k) >= v for k, v in hdelta.items())
    res['bound'] = total
    res['seconds'] = round(time.time() - T0, 1)
    outp.write_text(json.dumps(res, indent=1) + '\n')
    log(done=True)


if __name__ == '__main__':
    main()
