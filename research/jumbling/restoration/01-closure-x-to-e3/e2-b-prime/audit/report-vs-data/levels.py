"""report-vs-data audit: recompute one level of the iterated conjugation from E2 with my own bookkeeping.

For level k: P(k) built by the recursion P(k+1) = P(k) x_k P(k)^-1 from R = (237,13); T_k = Bad(P(k)^-1) by the
ghostlib chamber ghost (and, with --xcheck, by conjprep.bad_chambers, a different ghost engine); then P(k) applied at
E2, and at Y_k: parked set, B' on K, holes, L_k, the SAFE-ALL alphabet, and per letter whose cap meets L_k:
bad holes in cap, bad holes out, L out, entering pieces (ids), bad holes fixed pointwise.

Usage: python3 -I levels.py <theory dir> <v26 dir> <E2 table> <level> <out json> [--xcheck]
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
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402
import rim  # noqa: E402

T0 = time.time()


def I4(m):
    return all(m[r][q] == (1 if r == q else 0) for r in range(4) for q in range(4))


def main():
    a = sys.argv[1:]
    k_level = int(a[3])
    xcheck = '--xcheck' in a
    C = GL.Ctx(a[2])
    R, M, st = C.R, C.M, C.st
    restore = GL.restore
    nitems = len(M.items)
    cache = {}

    def inv_letter(c, i):
        if (c, i) not in cache:
            e = restore.element(M, c, i)
            h = [j for j in range(nitems) if I4(restore.matmul(e, restore.element(M, c, j)))]
            assert len(h) == 1
            cache[(c, i)] = (c, h[0])
        return cache[(c, i)]

    def inverse_word(w):
        return [inv_letter(c, i) for c, i in reversed(w)]

    xs_chosen = [(91, 6), (154, 7), (133, 1), (183, 6)]
    P = [(237, 13)]
    for x in xs_chosen[:k_level]:
        P = P + [x] + inverse_word(P)
    Pinv = inverse_word(P)
    d0 = st.digest()
    K0 = st._kpose()[st.pose_id]
    off0 = np.flatnonzero(K0 < 0)
    lat0 = K0 >= 0
    t = time.time()
    badPinv, g, ch = C.bad(Pinv)
    killed = ch[~g.alive]
    NP = len(st.pose_id)
    T = np.zeros(NP, bool)
    T[badPinv] = True
    out = {'level': k_level, 'P_length': len(P), 'T': int(T.sum()), 'chambers_ghosted': int(len(ch)),
           'killed_chambers': int(len(killed)), 'killed_in_T': int(T[killed].sum()), 'ghost_sec': round(time.time() - t, 1)}
    if xcheck:
        import conjprep as CP
        t = time.time()
        bad2, alive2 = CP.bad_chambers(M, Pinv, NP)
        out['xcheck_T_equal'] = bool(np.array_equal(np.flatnonzero(bad2), np.sort(badPinv)))
        out['xcheck_killed'] = int((~alive2).sum())
        out['xcheck_sec'] = round(time.time() - t, 1)
    for c, i in P:
        R.apply(c, i)
    out['after_P_digest'] = st.digest()
    K = st._kpose()[st.pose_id]
    parked = np.flatnonzero(lat0 & (K < 0))
    out['parked'] = int(len(parked))
    out['parked_ids'] = parked.tolist()
    out['bprime_on'] = int((K[off0] >= 0).sum())
    out['off_total'] = int((K < 0).sum())
    D = rim.Domains(st)
    L = np.sort(D.cham[off0])
    assert (D.dom[off0] == 0).all()
    out['L'] = D.cham[off0].tolist()
    out['L_in_T'] = int(T[L].sum())
    G = B.Alphabet(R, M)
    holes = np.asarray(G.holes, np.int64)
    out['holes'] = int(len(holes))
    isBH = np.zeros(NP, bool)
    isBH[holes[T[holes]]] = True
    out['BH'] = int(isBH.sum())
    out['GH'] = holes[~T[holes]].tolist()
    out['T_equals_BH_union_L'] = bool(np.array_equal(np.flatnonzero(T), np.union1d(np.flatnonzero(isBH), L)))
    isL = np.zeros(NP, bool)
    isL[L] = True
    lat_now = np.flatnonzero(K >= 0)
    piece_at = np.full(NP, -1, np.int64)
    piece_at[D.cham[lat_now]] = lat_now
    out['safe_grips'] = len(G.safe)
    out['letters'] = len(G.letters)
    cands = []
    for (c, i, _, ys) in G.letters:
        xs = G.capx[c]
        inL = isL[xs]
        if not inL.any():
            continue
        bh = isBH[xs]
        enter_mask = T[ys] & ~T[xs] & G.occupied[xs]
        gh_in = T[ys] & ~T[xs] & ~G.occupied[xs]
        rec = {'letter': [int(c), int(i)], 'L_in_cap': int(inL.sum()), 'bh_in_cap': int(bh.sum()),
               'bh_out': int((bh & ~T[ys]).sum()), 'L_out': int((inL & ~T[ys]).sum()),
               'enter_pieces': int(enter_mask.sum()), 'good_holes_in': int(gh_in.sum()),
               'bh_fixed': bool(np.array_equal(ys[bh], xs[bh])),
               'enter_ids': sorted(piece_at[xs[enter_mask]].tolist())}
        rec['F'] = bool(rec['bh_out'] == 0 and rec['L_out'] == len(L) and rec['L_in_cap'] == len(L))
        cands.append(rec)
    out['candidates'] = len(cands)
    out['grips_holding_L_any'] = sorted({r['letter'][0] for r in cands})
    allL = [r for r in cands if r['L_in_cap'] == len(L)]
    out['grips_holding_L_all'] = sorted({r['letter'][0] for r in allL})
    out['min_bh_in_cap_any'] = min((r['bh_in_cap'] for r in cands), default=None)
    out['min_bh_in_cap_all'] = min((r['bh_in_cap'] for r in allL), default=None)
    F = [r for r in cands if r['F']]
    out['F_count'] = len(F)
    out['F_letters'] = [r['letter'] for r in F]
    out['F_grips'] = sorted({r['letter'][0] for r in F})
    out['F_bh_in_cap'] = sorted({r['bh_in_cap'] for r in F})
    out['F_bh_fixed_all'] = all(r['bh_fixed'] for r in F)
    ok = [r for r in cands if r['L_out'] == len(L) and r['L_in_cap'] == len(L)]
    ok.sort(key=lambda r: (r['enter_pieces'], r['bh_out'], r['letter'][0], r['letter'][1]))
    out['best'] = [{x: y for x, y in r.items() if x != 'enter_ids'} for r in ok[:6]]
    if ok:
        out['chosen'] = ok[0]['letter']
        out['chosen_enter_ids'] = ok[0]['enter_ids']
        if k_level < 4:
            target = list(xs_chosen[k_level])
            tr = [r for r in cands if r['letter'] == target]
            out['report_x'] = target
            out['report_x_rec'] = {x: y for x, y in tr[0].items() if x != 'enter_ids'} if tr else None
            out['report_x_enter_ids'] = tr[0]['enter_ids'] if tr else None
    for _ in range(len(P)):
        R.undo()
    assert st.digest() == d0
    out['sec'] = round(time.time() - T0, 1)
    Path(a[4]).write_text(json.dumps(out) + '\n')
    print(json.dumps({x: y for x, y in out.items() if x not in ('parked_ids', 'chosen_enter_ids', 'report_x_enter_ids',
                                                               'F_letters', 'best')}))


if __name__ == '__main__':
    main()
