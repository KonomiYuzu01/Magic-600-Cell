"""Audit (math): the step lemma of iterated conjugation at E2, recomputed level by level.

For k = 0..4, P(k) is rebuilt from the recursion P(k+1) = P(k) x_k P(k)^-1 (P0 = R), and:
- T_k = Bad(P(k)^-1) by two ghost implementations: fastghost frames (ghostlib.Ctx.bad) and the simulator-kernel
  ghost over all 177,120 chambers (conjprep.bad_chambers); their Bad sets and their dying (straddling) sets compared;
- Y_k = E2 . P(k) on the simulator (every twist checked admissible): off set, parked set, B' chambers L_k, occupied
  chambers; the retrace fact T_k cap occ(Y_k) = L_k; holes split into BH_k (in T_k) and GH_k;
- x_k: SAFE-ALL at Y_k by direct classification of its grip (no straddle, no off piece inside, element in K+);
  its chamber permutation (checked against the simulator after applying it); the chambers it moves into T_k from
  outside, split into pieces (entering) and holes (gh_in); bad holes it moves out (bh_out); L_k moved out; kill
  violations (an entering piece landing in a chamber whose ghost straddles under P(k)^-1);
- then x_k and P(k)^-1 on the simulator: the parked set of P(k+1) against the entering pieces (as piece sets), the
  pieces parked by P(k) back at their exact E2 poses, B' on K, and every non-entering piece on K;
- for every grip whose cap holds all of L_k: SAFE-ALL status and, per K+ letter, L_out, enter, bh_out, gh_in, kill
  (a letter with gh_in > 0 is a counterexample to 'enter = 3 + bh_out');
- level 4: F = (162, 1) and every SAFE-ALL K+ letter holding L_4 and no bad hole; N = sigma_F^-1(T_4) minus BH_4.

Usage: python3 -I steplemma.py <theory dir> <v26 dir> <E2 table> <P4 json> <out json> [levels...]
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
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402
import restore  # noqa: E402
from exact import identity, matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

CL = BL.CL
reg, kp, data = CL.ctx.regions, CL.ctx.kplus, CL.ctx.data
T0 = time.time()
BPRIME = np.array([16245, 16249, 32088], np.int64)
RLET = (237, 13)
XS = [(91, 6), (154, 7), (133, 1), (183, 6)]
FLET = (162, 1)


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def chambers(st):
    K = st._kpose()[st.pose_id]
    ch = np.full(len(K), -1, np.int64)
    lat = np.flatnonzero(K >= 0)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    return ch, K


def kperm(M, c, i, NP):
    """Chamber permutation of a K+ letter (identity outside its cap), and its K+ index."""
    k = kp.index_of_matrix(restore.element(M, c, i))
    if k < 0:
        return None, -1
    xs = np.flatnonzero(GL.cap(c))
    ys = reg.piece_of[reg.orbit_of[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
    p = np.arange(NP)
    p[xs] = ys
    return p, k


def safe_all(st, c, K):
    cl = st.classify(c, need_inside=True, stop_at_straddle=False)
    inside = np.asarray(cl.inside, np.int64)
    return {'uncertain': bool(cl.uncertain), 'straddled': cl.straddle is not None,
            'off_inside': int((K[inside] < 0).sum()) if len(inside) else 0,
            'safe': (not cl.uncertain) and cl.straddle is None and not (len(inside) and (K[inside] < 0).any())}


def step_counts(perm, capx, Tm, occ, BHm, GHm, L, killm, occ_piece):
    out_src = capx[~Tm[capx]]
    ent_ch = out_src[Tm[perm[out_src]]]
    ent_occ = ent_ch[occ[ent_ch]]
    return {'L_in_cap': int(np.isin(L, capx).sum()), 'L_out': int((~Tm[perm[L]]).sum()),
            'enter': int(len(ent_occ)), 'gh_in': int(GHm[ent_ch].sum()),
            'bh_out': int((BHm[capx] & ~Tm[perm[capx]]).sum()),
            'bh_in_cap': int(BHm[capx].sum()),
            'kill_violations': int(killm[perm[ent_occ]].sum()),
            'chambers_into_T_from_outside': int(len(ent_ch)),
            'entering_pieces': sorted(occ_piece[ent_occ].tolist())}


def main():
    a = sys.argv[1:]
    table, p4file, outp = a[2], a[3], Path(a[4])
    C = GL.Ctx(table)
    R, M, st = C.R, C.M, C.st
    NP = len(st.pose_id)
    d0 = st.digest()
    codes = CC.Codes()
    code0 = codes.of(st)
    ch0, K0 = chambers(st)
    off0 = np.flatnonzero(K0 < 0)
    lat0 = K0 >= 0
    res = {'E2_digest': d0, 'E2_off': off0.tolist()}
    if not np.array_equal(off0, BPRIME):
        raise AssertionError('off set at E2 is not B\'')
    # recursion
    Pl = [[RLET]]
    for k in range(4):
        Pl.append(Pl[k] + [XS[k]] + CP.inverse_word(M, Pl[k]))
    p4 = [tuple(x) for x in json.loads(Path(p4file).read_text())]
    res['P4_equals_recursion'] = Pl[4] == p4
    res['P_lengths'] = [len(p) for p in Pl]
    # exact inverse check of every letter pair used
    inv_ok = True
    for P in Pl:
        Pinv = CP.inverse_word(M, P)
        for (c, i), (c2, j) in zip(P, reversed(Pinv)):
            prod = matmul(restore.element(M, c, i), restore.element(M, c2, j))
            inv_ok &= (c == c2) and to_tuple(prod) == to_tuple(identity(4))
    res['inverse_letters_exact'] = bool(inv_ok)
    levels = []
    todo = [int(x) for x in a[5:]] or list(range(5))
    for k in todo:
        t = time.time()
        P = Pl[k]
        Pinv = CP.inverse_word(M, P)
        T1, g1, ch1 = C.bad(Pinv)
        kill1 = np.sort(ch1[~g1.alive])
        badm2, alive2 = CP.bad_chambers(M, Pinv, NP)
        T2 = np.flatnonzero(badm2)
        kill2 = np.flatnonzero(~alive2)
        row = {'level': k, 'P_length': len(P), 'T_fastghost': int(len(T1)), 'T_simghost': int(len(T2)),
               'T_equal': bool(np.array_equal(np.sort(T1), T2)), 'kill_fastghost': int(len(kill1)),
               'kill_simghost': int(len(kill2)), 'kill_equal': bool(np.array_equal(kill1, kill2))}
        Tm = np.zeros(NP, bool)
        Tm[T2] = True
        killm = np.zeros(NP, bool)
        killm[kill2] = True
        applied = 0
        for c, i in P:
            R.apply(c, i)
            applied += 1
        chk, Kk = chambers(st)
        codek = codes.of(st)
        offk = np.flatnonzero(Kk < 0)
        park = np.flatnonzero(lat0 & (Kk < 0))
        L = chk[BPRIME]
        occ = np.zeros(NP, bool)
        occ_piece = np.full(NP, -1, np.int64)
        on = np.flatnonzero(Kk >= 0)
        occ[chk[on]] = True
        occ_piece[chk[on]] = on
        holes = np.flatnonzero(~occ)
        BHm = Tm & ~occ
        GHm = ~Tm & ~occ
        row.update({'after_P_digest': st.digest(), 'off': int(len(offk)), 'parked': int(len(park)),
                    'parked_ids': park.tolist() if len(park) <= 200 else None,
                    'bprime_on_K': int((Kk[BPRIME] >= 0).sum()), 'L': L.tolist(),
                    'L_subset_T': bool(Tm[L].all()),
                    'T_cap_occupied_equals_L': bool(np.array_equal(np.flatnonzero(Tm & occ), np.sort(L))),
                    'holes': int(len(holes)), 'BH': int(BHm.sum()), 'GH': np.flatnonzero(GHm).tolist(),
                    'T_equals_L_plus_BH': bool(Tm.sum() == BHm.sum() + len(L)),
                    'parked_equals_T': bool(len(park) == Tm.sum()),
                    'Bad_P_contains_E2_hole': None})
        # scan every grip whose cap holds all of L
        holdL = [c for c in range(600) if GL.cap(c)[L].all()]
        scan = []
        for c in holdL:
            sa = safe_all(st, c, Kk)
            ent = {'grip': c, **sa, 'letters': []}
            if sa['safe']:
                capx = np.flatnonzero(GL.cap(c))
                for i in range(len(M.items)):
                    perm, kk = kperm(M, c, i, NP)
                    if perm is None or kk == kp.index_of_matrix(to_tuple(identity(4))):
                        continue
                    s = step_counts(perm, capx, Tm, occ, BHm, GHm, L, killm, occ_piece)
                    s.pop('entering_pieces')
                    ent['letters'].append({'i': i, **s})
            scan.append(ent)
        row['grips_holding_all_L'] = holdL
        row['scan'] = scan
        valid = [(e['grip'], x) for e in scan for x in e['letters']
                 if x['L_out'] == len(L) and x['kill_violations'] == 0]
        row['valid_step_letters'] = len(valid)
        row['valid_with_gh_in'] = [[g, x['i'], x['gh_in'], x['enter'], x['bh_out']] for g, x in valid if x['gh_in']]
        row['valid_enter_ne_3_plus_bh_out'] = [[g, x['i'], x['enter'], x['bh_out'], x['gh_in']] for g, x in valid
                                               if x['enter'] != 3 + x['bh_out']]
        row['finishing_letters'] = [[g, x['i']] for g, x in valid if x['bh_in_cap'] == 0]
        if k < 4:
            c, i = XS[k]
            sa = safe_all(st, c, Kk)
            perm, kk = kperm(M, c, i, NP)
            capx = np.flatnonzero(GL.cap(c))
            s = step_counts(perm, capx, Tm, occ, BHm, GHm, L, killm, occ_piece)
            row['x'] = {'letter': [c, i], 'in_Kplus': kk >= 0, **sa,
                        **{q: v for q, v in s.items() if q != 'entering_pieces'}}
            entering = np.array(s['entering_pieces'], np.int64)
            R.apply(c, i)
            applied += 1
            chx, Kx = chambers(st)
            incap = np.flatnonzero((chk >= 0) & GL.cap(c)[np.maximum(chk, 0)])
            row['x']['simulator_matches_perm'] = bool(np.array_equal(chx[incap], perm[chk[incap]]))
            row['x']['off_unchanged'] = bool(np.array_equal(codes.of(st)[offk], codek[offk]))
            for c2, i2 in Pinv:
                R.apply(c2, i2)
                applied += 1
            che, Ke = chambers(st)
            codee = codes.of(st)
            park_next = np.flatnonzero(lat0 & (Ke < 0))
            row['next'] = {'parked': int(len(park_next)),
                           'parked_equals_entering': bool(np.array_equal(np.sort(park_next), np.sort(entering))),
                           'bprime_on_K': int((Ke[BPRIME] >= 0).sum()),
                           'old_parked_back_exact_E2_pose': bool(np.array_equal(codee[park], code0[park])),
                           'off_total': int((Ke < 0).sum()),
                           'digest': st.digest()}
        else:
            c, i = FLET
            sa = safe_all(st, c, Kk)
            perm, kk = kperm(M, c, i, NP)
            capx = np.flatnonzero(GL.cap(c))
            s = step_counts(perm, capx, Tm, occ, BHm, GHm, L, killm, occ_piece)
            into_T = capx[Tm[perm[capx]]]
            N = np.sort(into_T[~BHm[into_T]])
            row['F'] = {'letter': [c, i], **sa, **{q: v for q, v in s.items() if q != 'entering_pieces'},
                        'N': N.tolist(), 'N_outside_T': bool(not Tm[N].any()),
                        'bh_fixed': bool((perm[np.flatnonzero(BHm)] == np.flatnonzero(BHm)).all())}
        for _ in range(applied):
            R.undo()
        if st.digest() != d0:
            raise AssertionError('undo did not return to E2')
        row['sec'] = round(time.time() - t, 1)
        log(**{q: v for q, v in row.items() if q not in ('scan', 'L', 'GH', 'parked_ids')})
        levels.append(row)
    res['levels'] = levels
    res['seconds'] = round(time.time() - T0, 1)
    outp.write_text(json.dumps(res) + '\n')


if __name__ == '__main__':
    main()
