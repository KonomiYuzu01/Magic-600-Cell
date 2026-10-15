"""Lemma H audit, test B: Bad(U) for the 1,210-twist word at E1, against the holes of E1 and against (H3).

U = word-full.json (V2 P2 Q2 P2^-1 V2^-1). Computed here, independently of pre.json:
- Bad(U), Bad(W2) (W2 = P2 Q2 P2^-1) and Bad(V2) by the chamber ghost over the union of each word's caps;
- the chamber permutation of V2 twice: from the ghost and from buffer.Alphabet letters at E1;
- T's and B''s own contents through U and through W2 after V2;
- the holes of E1 and E2 and the SAFE-ALL caps that contain each hole of E1.
Writes testB.json. Read-only on every input.

Usage: python3 -I testB.py <theory dir> <v26 dir> <E1 table> <E2 table> <c0 dir> <out json>
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
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import matmul  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def union_of_caps(word):
    grips = sorted({c for c, _ in word})
    return np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]))


def run_ghost(C, word, pd, pq, chambers):
    g = FG.Ghost(C.F, np.concatenate([pd, np.zeros(len(chambers), np.int64)]), np.concatenate([pq, chambers]))
    for c, i in word:
        g.apply(c, C.E(c, i), (c, i))
    return g


def is_identity(m):
    return all(m[r][q] == (1 if r == q else 0) for r in range(4) for q in range(4))


def main():
    e1, e2, c0, outp = sys.argv[3], sys.argv[4], Path(sys.argv[5]), Path(sys.argv[6])
    wf = json.loads((c0 / 'word-full.json').read_text())
    U = [(int(c), int(i)) for c, i in wf['word']]
    parts = {n: [(int(c), int(i)) for c, i in json.loads((c0 / 'parts' / f'{n}.json').read_text())['word']]
             for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')}
    pre = json.loads((c0 / 'pre.json').read_text())
    res = {'U_length': len(U), 'parts': {n: len(w) for n, w in parts.items()}}
    res['U_is_concatenation_of_parts'] = U == parts['V'] + parts['P'] + parts['Q'] + parts['Pinv'] + parts['Vinv']
    t = time.time()
    C = GL.Ctx(e1)
    st, M = C.st, C.M
    res['E1_digest'] = st.digest()
    res['word_input_digest'] = wf['input_digest']
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    T = C.block(87)
    Bp = np.setdiff1d(off, T)
    res['off_count'] = int(len(off))
    res['blocks'] = sorted(len(b) for b in C.bl.blocks)
    res['B_prime'] = Bp.tolist()
    holes = B.hole_set(st)
    res['holes_E1_count'] = int(len(holes))
    res['holes_two_ways_equal'] = bool(np.array_equal(np.flatnonzero(~C.occ), holes))
    res['load_seconds'] = round(time.time() - t, 1)
    # inverses are exact mirror inverses
    V, W2 = parts['V'], parts['P'] + parts['Q'] + parts['Pinv']
    res['Vinv_is_inverse_of_V'] = len(parts['Vinv']) == len(V) and all(
        c1 == c2 and is_identity(matmul(C.E(c1, i1), C.E(c2, i2)))
        for (c1, i1), (c2, i2) in zip(reversed(V), parts['Vinv']))
    res['Pinv_is_inverse_of_P'] = all(
        c1 == c2 and is_identity(matmul(C.E(c1, i1), C.E(c2, i2)))
        for (c1, i1), (c2, i2) in zip(reversed(parts['P']), parts['Pinv']))
    log(digest=res['E1_digest'], off=res['off_count'], holes=len(holes), concat=res['U_is_concatenation_of_parts'],
        Vinv=res['Vinv_is_inverse_of_V'], Pinv=res['Pinv_is_inverse_of_P'])
    pd, pq = C.contents(np.concatenate([T, Bp]))
    nT, n = len(T), len(T) + len(Bp)
    # Bad(U)
    t = time.time()
    chU = union_of_caps(U)
    gU = run_ghost(C, U, pd, pq, chU)
    lat = slice(n, None)
    died = ~gU.alive[lat]
    endoff = gU.alive[lat] & (gU.dom[lat] != 0)
    badU = np.sort(chU[died | endoff])
    res['ghost_U_seconds'] = round(time.time() - t, 1)
    res['chambers_in_union_of_caps_U'] = int(len(chU))
    hs = set(holes.tolist())
    bs = set(badU.tolist())
    res['badU'] = badU.tolist()
    res['badU_size'] = int(len(badU))
    res['badU_died_straddle'] = int(died.sum())
    res['badU_ended_off'] = int(endoff.sum())
    res['badU_subset_holes'] = bs <= hs
    res['badU_minus_holes'] = sorted(bs - hs)
    res['holes_minus_badU'] = sorted(hs - bs)
    res['holes_outside_union_of_caps_U'] = sorted(hs - set(chU.tolist()))
    res['T_through_U'] = {'on_K': int((gU.alive[:nT] & (gU.dom[:nT] == 0)).sum()), 'dead': int((~gU.alive[:nT]).sum())}
    res['Bp_through_U_exact_return'] = bool(gU.alive[nT:n].all() and np.array_equal(gU.dom[nT:n], pd[nT:])
                                            and np.array_equal(gU.q[nT:n], pq[nT:]))
    log(badU=res['badU_size'], subset=res['badU_subset_holes'], T=res['T_through_U'],
        Bp=res['Bp_through_U_exact_return'], sec_ghost=res['ghost_U_seconds'])
    # good map of U: images of good chambers; good holes and where they end
    good = ~(died | endoff)
    img = np.arange(GL.NP)
    img[chU[good]] = gU.q[lat][good]
    goodmask = np.ones(GL.NP, bool)
    goodmask[badU] = False
    gimg = img[goodmask]
    res['U_good_images_distinct'] = bool(len(np.unique(gimg)) == len(gimg))
    good_holes = np.array(sorted(hs - bs), np.int64)
    res['good_holes_end_chambers'] = img[good_holes].tolist() if len(good_holes) else []
    T_end = gU.q[:nT][gU.alive[:nT] & (gU.dom[:nT] == 0)]
    free = np.ones(GL.NP, bool)
    free[gimg] = False
    free[T_end] = False
    res['chambers_not_reached_by_good_contents_or_T'] = np.flatnonzero(free).tolist()
    # Bad(W2) recomputed, and V2's chamber permutation (ghost and Alphabet)
    t = time.time()
    chW = union_of_caps(W2)
    gW = run_ghost(C, W2, np.zeros(0, np.int64), np.zeros(0, np.int64), chW)
    badW = np.sort(chW[~gW.alive | (gW.dom != 0)])
    res['badW2'] = badW.tolist()
    res['badW2_size'] = int(len(badW))
    res['badW2_equals_pre_json'] = badW.tolist() == sorted(pre['ghost']['badW'])
    chV = union_of_caps(V)
    gV = run_ghost(C, V, np.zeros(0, np.int64), np.zeros(0, np.int64), chV)
    res['V2_all_contents_alive_on_K'] = bool(gV.alive.all() and (gV.dom == 0).all())
    piV = np.arange(GL.NP)
    piV[chV] = gV.q
    res['V2_ghost_perm_is_permutation'] = bool(len(np.unique(piV)) == GL.NP)
    res['ghost_W2_V2_seconds'] = round(time.time() - t, 1)
    t = time.time()
    G = B.Alphabet(C.R, M)
    G.orbits()
    piV_alpha = G.permutation([list(x) for x in V])
    res['V2_perm_ghost_equals_alphabet'] = bool(np.array_equal(piV, piV_alpha))
    inv = np.argsort(piV)
    pre_badW = np.sort(inv[badW])
    res['preimage_of_badW2_under_V2'] = pre_badW.tolist()
    res['H3_badU_equals_preimage'] = pre_badW.tolist() == badU.tolist()
    res['V2_vacates_badW2'] = bool(np.isin(badW, piV[holes]).all())
    res['holes_after_V2_minus_badW2'] = sorted(set(piV[holes].tolist()) - set(badW.tolist()))
    # T's landing under W2 (T is not moved by V2: V2 is SAFE-ALL at E1, so T's contents are unchanged)
    gT = run_ghost(C, W2, pd, pq, np.zeros(0, np.int64))
    T_land = gT.q[:nT][gT.alive[:nT] & (gT.dom[:nT] == 0)]
    res['T_landing_under_W2_size'] = int(len(T_land))
    res['T_landing_equals_badW2'] = sorted(T_land.tolist()) == badW.tolist()
    res['Bp_exact_under_W2'] = bool(gT.alive[nT:n].all() and np.array_equal(gT.dom[nT:n], pd[nT:])
                                    and np.array_equal(gT.q[nT:n], pq[nT:]))
    # holes of E1 inside SAFE-ALL caps (such a cap holds no off piece and no piece straddles its cut)
    safe = np.asarray(G.safe, np.int64)
    res['safe_all_grips_E1'] = int(len(safe))
    in_safe = np.array([G.home[z, safe].any() for z in holes.tolist()])
    res['holes_E1_in_some_safe_cap'] = int(in_safe.sum())
    res['holes_E1_not_in_safe_cap'] = holes[~in_safe].tolist()
    res['hole_E1_orbit_size_min'] = int(G.orbit_size[G.orbit[holes]].min())
    res['alphabet_seconds'] = round(time.time() - t, 1)
    log(badW=res['badW2_size'], badW_eq_pre=res['badW2_equals_pre_json'], H3=res['H3_badU_equals_preimage'],
        perm_eq=res['V2_perm_ghost_equals_alphabet'], vacates=res['V2_vacates_badW2'],
        T_land_eq=res['T_landing_equals_badW2'], holes_in_safe=res['holes_E1_in_some_safe_cap'])
    # holes of E2: good holes of E1 must end in holes of E2 (end-state argument)
    t = time.time()
    C2 = GL.Ctx(e2)
    holesE2 = B.hole_set(C2.st)
    res['E2_digest'] = C2.st.digest()
    res['E2_holes'] = holesE2.tolist()
    res['good_holes_end_in_E2_holes'] = bool(np.isin(img[good_holes], holesE2).all()) if len(good_holes) else None
    res['unreached_equals_E2_holes'] = res['chambers_not_reached_by_good_contents_or_T'] == holesE2.tolist()
    res['E2_seconds'] = round(time.time() - t, 1)
    res['seconds'] = round(time.time() - T0, 1)
    outp.write_text(json.dumps(res, indent=1) + '\n')
    log(E2_holes=res['E2_holes'], good_holes=sorted(hs - bs), good_end=res['good_holes_end_chambers'],
        good_end_in_E2_holes=res['good_holes_end_in_E2_holes'], done=str(outp))


if __name__ == '__main__':
    main()
