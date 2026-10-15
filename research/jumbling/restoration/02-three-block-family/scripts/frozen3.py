"""Frozen-hole branch: why a chamber is frozen at the off geometry, and which letters move its image at each level.

For a chamber z0 that the SAFE-ALL alphabet fixes (a singleton orbit), the script records at the input state X every
grip whose cap holds z0 and why it is blocked (the block inside the cap, or cutting it). Then, at each level k of the
template (P_k from sigma), with v = pi_{P_k}(z0), T_k = Bad(P_k^-1) and L_k the block's chambers, it classifies every
K+ letter y whose cap holds v:
  blocked_L      cap(y) meets L_k (y would move the block);
  blocked_parked y moves or cuts a parked content (a content of Off(P_k) after P_k);
  pure           y(T_k) = T_k: Z = P_k y P_k^-1 is pure (Lemma Z);
  absorbing      y^-1(T_k) - T_k = {v}: at a state where z0 is a hole, Z = P_k y P_k^-1 has Bad(Z) = {z0} (no
                 collateral) and moves the hole from z0 to pi_{P_k}^-1(y(t)) for the chamber t of T_k that y moves out;
  other          y moves some other chamber outside T_k into T_k.
Word-only: nothing is applied to the simulator.

Usage: python -I frozen3.py <theory dir> <v26 dir> <pose table> <template json> <z0> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402

B, CP, FG, GL = L3.B, L3.CP, L3.FG, L3.GL
NP = L3.NP
T0 = time.time()


def main():
    a = sys.argv[1:]
    table, tfile, z0, out = a[2], a[3], int(a[4]), Path(a[5])
    sigma = json.loads(Path(tfile).read_text())
    C = GL.Ctx(table)
    st = C.st
    H = L3.HOME()
    Bp = np.asarray(C.off, np.int64)
    dom0, q0 = C.contents(Bp)
    typ = np.asarray(L3.reg.orbit_of)
    res = {'input_digest': st.digest(), 'z0': z0, 'z0_type': int(typ[z0]), 'z0_occupied': bool(C.occ[z0])}
    # why z0 is frozen at X: every grip holding z0 is blocked by the block (inside or cut)
    grips = np.flatnonzero(H[z0]).tolist()
    why = []
    for c in grips:
        sd = L3.sides(C, dom0, q0, c)
        why.append({'grip': c, 'block_inside': int((sd == 1).sum()), 'block_cut': int((sd < 0).sum())})
    res['at_X'] = {'grips_holding_z0': len(grips),
                   'blocked_by_block': sum(1 for w in why if w['block_inside'] or w['block_cut']),
                   'free': [w['grip'] for w in why if not (w['block_inside'] or w['block_cut'])],
                   'block_inside_count': sum(1 for w in why if w['block_inside']),
                   'block_cut_count': sum(1 for w in why if w['block_cut'] and not w['block_inside'])}
    L3.log(at_X=res['at_X'])
    R, xs = tuple(sigma['R']), [tuple(x) for x in sigma['x']]
    words = L3.level_words(C.M, R, xs)
    allg = sorted({R[0]} | {x[0] for x in xs} | ({sigma['F'][0]} if sigma.get('F') else set()))
    U = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in allg]))
    res['levels'] = []
    for k, Pk in enumerate(words):
        gP = L3.Chambers(C, Pk, U)
        gI = L3.Chambers(C, CP.inverse_word(C.M, Pk), U)
        T = gI.bad
        blk = L3.block_run(C, Pk, dom0, q0, [st.pose(int(p)) for p in Bp])
        Lk = blk['chambers']
        v = int(gP.pi[z0])
        row = {'k': k, 'v': v, 'z0_parked': bool(gP.off[z0]), 'T': int(T.sum()), 'L': Lk}
        if v < 0:
            res['levels'].append(row)
            L3.log(level=k, v=v, parked=row['z0_parked'])
            continue
        pd, pq = gP.parked()
        ip = gP.inverse_map()
        gv = np.flatnonzero(H[v]).tolist()
        counts = {'grips_holding_v': len(gv), 'blocked_L': 0, 'blocked_parked': 0}
        letters = []
        for c in gv:
            if H[Lk, c].any():
                counts['blocked_L'] += 1
                continue
            sd = L3.sides(C, pd, pq, c)
            if not (sd == 0).all():
                counts['blocked_parked'] += 1
                continue
            for c2, i in L3.kplus_letters(C, [c]):
                ymap, _ = L3.letter_map(C, c2, i)
                into = np.flatnonzero(~T & T[ymap])  # outside chambers that y moves into T
                outof = np.flatnonzero(T & ~T[ymap])  # chambers of T that y moves out
                kind = 'pure' if not len(into) else ('absorbing' if into.tolist() == [v] else 'other')
                rec = {'y': [c2, i], 'kind': kind, 'into_T': len(into), 'v_moves': bool(ymap[v] != v)}
                if kind == 'absorbing':
                    t = int(outof[0])
                    newhole = int(ip[ymap[t]])
                    rec.update(t=t, new_hole=newhole, new_hole_type=int(typ[newhole]),
                               z0_filled_from=int(ip[np.flatnonzero(ymap == v)[0]]) if (ymap == v).any() else None)
                if kind == 'pure' and ymap[v] != v:
                    rec['z0_image'] = int(ip[ymap[v]])
                letters.append(rec)
        row.update(counts)
        row['free_grips'] = sorted({r['y'][0] for r in letters})
        row['letters'] = letters
        row['kinds'] = {kd: sum(1 for r in letters if r['kind'] == kd) for kd in ('pure', 'absorbing', 'other')}
        res['levels'].append(row)
        L3.log(level=k, v=v, T=row['T'], **counts, kinds=row['kinds'])
    res['seconds'] = round(time.time() - T0, 1)
    out.write_text(json.dumps(res, indent=1) + '\n')
    L3.log(done=str(out))


if __name__ == '__main__':
    main()
