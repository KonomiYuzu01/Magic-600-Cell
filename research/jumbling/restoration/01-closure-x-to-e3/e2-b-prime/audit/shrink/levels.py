"""Audit 'shrink': recompute one iterated-conjugation level at E2 with own code (no shrink.py).

For level k: build P(k) by the recursion from P0 = [(237,13)], apply it at E2 (every twist checked admissible by the
simulator), and compute at Y_k = E2 P(k): parked set, B' on K, chambers L_k of B', holes (own computation from
poses), T_k = Bad(P(k)^-1) (ghostlib.Ctx.bad; compared later with an independent ghost), straddling T_k chambers,
bad holes, the SAFE-ALL alphabet (buffer.Alphabet), per-letter metrics for every SAFE-ALL letter whose cap holds a
chamber of L_k, the least bad holes in a SAFE-ALL cap holding a chamber of L_k, the greedy choice and the finishing
letters. For k < 4 it also applies x_k at Y_k with the simulator, reads the set of pieces now in T_k chambers,
then applies P(k)^-1 (completing P(k+1) at E2, every twist admissible) and reads the parked set of P(k+1).
Every applied twist is undone and the E2 digest is checked at the end.

Usage: python3 -I -B levels.py <k> <out json>
"""
import hashlib
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.append(str(S / 'v26'))

import numpy as np  # noqa: E402

import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import matmul, transpose  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

O, CL, restore = BL.O, BL.CL, BL.restore
kp, reg, data = CL.ctx.kplus, CL.ctx.regions, CL.ctx.data
E2 = S / 'v26/B4/E2-table.json'
E2_SHA = '649576e69ede65a6203dfbebd3b879a68e40250abaf18a0f08ad86084834d893'
E2_DIGEST = '19e404d438fd929b0c6480c35374aa5147b361831806774ff65ca0f4739bc5ff'
XS = [(91, 6), (154, 7), (133, 1), (183, 6)]
T0 = time.time()


def log(*a):
    print(round(time.time() - T0, 1), *a, file=sys.stderr, flush=True)


def inv_letter(M, c, i):
    e = restore.element(M, c, i)
    et = to_tuple(transpose(e))
    hits = [j for j in range(len(M.items)) if restore.element(M, c, j) == et]
    if len(hits) != 1:
        raise AssertionError(f'inverse of ({c},{i}) not unique: {hits}')
    j = hits[0]
    if to_tuple(matmul(e, restore.element(M, c, j))) != restore.I4:
        raise AssertionError('inverse check failed')
    return (c, j)


def inv_word(M, w):
    return [inv_letter(M, c, i) for c, i in reversed(w)]


def words(M):
    P = [[(237, 13)]]
    for k in range(4):
        P.append(P[k] + [XS[k]] + inv_word(M, P[k]))
    return P


# own random signature weights (independent of rim.py's hash)
_W = np.random.default_rng(20261012).integers(1, 2 ** 62, size=600, dtype=np.uint64)


def sig_hash_all():
    NP = len(data.sigbits)
    out = np.zeros(NP, np.uint64)
    for s0 in range(0, NP, 8192):
        bits = np.unpackbits(data.sigbits[s0:s0 + 8192], axis=1)[:, :600].astype(np.uint64)
        out[s0:s0 + 8192] = (bits * _W[None, :]).sum(axis=1, dtype=np.uint64)
    return out


def chambers_by_signature(pieces, kidx, H, order, Hs):
    """Chamber of piece p at K+ pose k, found by signature: sig(chamber) = k(sig(p)) (pole images perms[k])."""
    pieces = np.asarray(pieces, np.int64)
    out = np.empty(len(pieces), np.int64)
    perms = kp.perms
    for s0 in range(0, len(pieces), 4096):
        ps = pieces[s0:s0 + 4096]
        ks = kidx[s0:s0 + 4096]
        bits = np.unpackbits(data.sigbits[ps], axis=1)[:, :600].astype(bool)
        img = perms[ks].astype(np.int64)  # img[r, c] = k_r(c)
        h = np.zeros(len(ps), np.uint64)
        for r in range(len(ps)):
            h[r] = _W[img[r][bits[r]]].sum(dtype=np.uint64)
        pos = np.minimum(np.searchsorted(Hs, h), len(Hs) - 1)
        if not np.array_equal(Hs[pos], h):
            raise AssertionError('signature image is not a chamber signature')
        out[s0:s0 + 4096] = order[pos]
    return out


def lattice_chambers(st):
    K = st._kpose()[st.pose_id]
    ch = np.full(len(K), -1, np.int64)
    lat = np.flatnonzero(K >= 0)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    return ch, K


def main():
    k = int(sys.argv[1])
    out = Path(sys.argv[2])
    rec = {'level': k}
    sha = hashlib.sha256(E2.read_bytes()).hexdigest()
    rec['E2_sha256'] = sha
    R, M = O.start(str(E2))
    st = R.st
    d0 = st.digest()
    rec['E2_digest'] = d0
    assert sha == E2_SHA and d0 == E2_DIGEST, 'E2 identity'
    P = words(M)
    p4 = [tuple(x) for x in json.loads((S / 'v26/B4/P4.json').read_text())]
    rec['P4_equals_P4json'] = P[4] == p4
    rec['P_lengths'] = [len(p) for p in P]
    Pk = P[k]
    Pinv = inv_word(M, Pk)
    rec['P'] = [list(x) for x in Pk]
    rec['Pinv'] = [list(x) for x in Pinv]
    NP = len(st.pose_id)
    ch0, K0 = lattice_chambers(st)
    off0 = np.flatnonzero(K0 < 0)
    lat0 = K0 >= 0
    rec['E2_off'] = off0.tolist()
    m = st.pose(int(off0[0]))
    rec['Bprime_one_pose'] = all(st.pose(int(p)) == m for p in off0)
    occ0 = np.zeros(NP, bool)
    occ0[ch0[lat0]] = True
    holes0 = np.flatnonzero(~occ0)
    rec['E2_holes'] = holes0.tolist()

    # signature-based chamber check set-up
    H = sig_hash_all()
    order = np.argsort(H)
    Hs = H[order]
    assert len(np.unique(Hs)) == NP, 'hash collision'

    # ---- apply P(k) at E2
    applied = 0
    for c, i in Pk:
        R.apply(c, i)  # raises RuntimeError if not admissible
        applied += 1
    rec['P_all_admissible'] = applied == len(Pk)
    rec['after_P_digest'] = st.digest()
    ch, K = lattice_chambers(st)
    offk = K < 0
    parked = np.flatnonzero(lat0 & offk)
    rec['parked'] = int(len(parked))
    rec['parked_ids'] = parked.tolist()
    rec['bprime_on'] = int((K[off0] >= 0).sum())
    rec['off_at_Y'] = int(offk.sum())
    latk = np.flatnonzero(~offk)
    if len(np.unique(ch[latk])) != len(latk):
        raise AssertionError('two lattice pieces share a chamber at Y_k')
    # independent chamber check by signature images
    ch_sig = chambers_by_signature(latk, K[latk], H, order, Hs)
    rec['chamber_table_matches_signature_images'] = bool(np.array_equal(ch_sig, ch[latk]))
    occ = np.zeros(NP, bool)
    occ[ch[latk]] = True
    owner = np.full(NP, -1, np.int64)
    owner[ch[latk]] = latk
    holes = np.flatnonzero(~occ)
    rec['holes'] = int(len(holes))
    L = ch[off0]
    rec['L'] = L.tolist()
    log('applied P', len(Pk), 'parked', len(parked), 'bon', rec['bprime_on'], 'holes', len(holes))

    # ---- T_k = Bad(P(k)^-1) by ghostlib (word only)
    C = GL.Ctx(str(E2))
    badPinv, gb, chb = C.bad(Pinv)
    T = np.zeros(NP, bool)
    T[badPinv] = True
    kill = np.zeros(NP, bool)
    kill[chb[~gb.alive]] = True
    rec['T'] = int(T.sum())
    rec['T_ids'] = np.flatnonzero(T).tolist()
    rec['T_straddle'] = int((kill & T).sum())
    rec['kill_total'] = int(kill.sum())
    rec['ghost_chambers'] = int(len(chb))
    rec['L_in_T'] = int(T[L].sum())
    isBH = np.zeros(NP, bool)
    isBH[holes[T[holes]]] = True
    BH = np.flatnonzero(isBH)
    GH = holes[~T[holes]]
    rec['BH'] = int(len(BH))
    rec['BH_ids'] = BH.tolist()
    rec['GH'] = GH.tolist()
    # contents of T_k chambers at Y_k
    rec['T_occupied_not_L'] = int((occ & T).sum() - T[L].sum())
    log('T', rec['T'], 'BH', rec['BH'], 'GH', rec['GH'], 'T_straddle', rec['T_straddle'])

    # ---- SAFE-ALL alphabet at Y_k
    G = B.Alphabet(R, M)
    rec['alphabet_holes_match'] = bool(np.array_equal(np.sort(np.asarray(G.holes)), holes))
    rec['safe_grips'] = len(G.safe)
    rec['letters'] = len(G.letters)
    isL = np.zeros(NP, bool)
    isL[L] = True
    grips_L, grips_all3 = [], []
    min_bh = None
    min_bh_grips = []
    for c in G.safe:
        xs = G.capx[c]
        nl = int(isL[xs].sum())
        if nl:
            grips_L.append(int(c))
            b = int(isBH[xs].sum())
            if min_bh is None or b < min_bh:
                min_bh, min_bh_grips = b, [int(c)]
            elif b == min_bh:
                min_bh_grips.append(int(c))
        if nl == len(L):
            grips_all3.append(int(c))
    rec['grips_holding_L'] = grips_L
    rec['grips_holding_all_L'] = grips_all3
    rec['min_bh_in_cap'] = min_bh
    rec['min_bh_grips'] = min_bh_grips
    cand = []
    for (c, i, kk, ys) in G.letters:
        xs = G.capx[c]
        if not isL[xs].any():
            continue
        Tin, Tout = T[xs], T[ys]
        o = occ[xs]
        row = {'letter': [int(c), int(i)], 'cap': int(len(xs)), 'L_in_cap': int(isL[xs].sum()),
               'bh_in_cap': int(isBH[xs].sum()), 'bh_out': int((isBH[xs] & ~Tout).sum()),
               'L_out': int((isL[xs] & ~Tout).sum()), 'enter': int((o & ~Tin & Tout).sum()),
               'T_to_T_occupied': int((o & Tin & Tout).sum()),
               'good_holes_in': int((~o & ~Tin & Tout).sum()),
               'kill_viol': int((o & kill[ys]).sum()),
               'gh_in_cap': int(np.isin(xs, GH).sum())}
        row['F'] = bool(row['L_in_cap'] == len(L) and row['L_out'] == len(L) and row['bh_out'] == 0)
        cand.append(row)
    rec['candidates'] = len(cand)
    rec['F_letters'] = [r['letter'] for r in cand if r['F']]
    rec['F_bh_in_cap'] = sorted({r['bh_in_cap'] for r in cand if r['F']})
    rec['F_grips'] = sorted({r['letter'][0] for r in cand if r['F']})
    ok = [r for r in cand if r['L_in_cap'] == len(L) and r['L_out'] == len(L) and r['kill_viol'] == 0]
    ok.sort(key=lambda r: (r['enter'], r['bh_out'], r['letter'][0], r['letter'][1]))
    rec['ok_count'] = len(ok)
    rec['best'] = ok[:12]
    rec['all_candidates'] = cand
    log('alphabet', len(G.safe), len(G.letters), 'cands', len(cand), 'min_bh', min_bh, 'F', len(rec['F_letters']))

    if k < 4:
        x = XS[k]
        rec['x'] = list(x)
        rec['x_is_safe_letter'] = x in G.index
        rec['x_is_first'] = bool(ok and tuple(ok[0]['letter']) == x)
        row = next((r for r in cand if tuple(r['letter']) == x), None)
        rec['x_row'] = row
        # ties with x at the first two keys
        if row is not None:
            rec['x_ties_first_keys'] = [r['letter'] for r in ok if (r['enter'], r['bh_out']) ==
                                        (row['enter'], row['bh_out'])]
        # set of pieces x moves into T_k, from the alphabet's chamber permutation
        n = G.index[x]
        c, i, kk, ys = G.letters[n]
        xs = G.capx[c]
        sel = occ[xs] & ~T[xs] & T[ys]
        enter_alpha = np.sort(owner[xs[sel]])
        rec['enter_alpha_ids'] = enter_alpha.tolist()
        # the same set read from the simulator: apply x at Y_k, pieces on K in T_k chambers
        R.apply(*x)
        ch2, K2 = lattice_chambers(st)
        lat2 = np.flatnonzero(K2 >= 0)
        inT = np.sort(lat2[T[ch2[lat2]]])
        rec['after_x_on_K_in_T_ids'] = inT.tolist()
        rec['after_x_Bprime_chambers'] = ch2[off0].tolist()
        rec['after_x_Bprime_outside_T'] = bool((K2[off0] >= 0).all() and not T[ch2[off0]].any())
        rec['after_x_parked_unchanged'] = bool(np.array_equal(np.flatnonzero(lat0 & (K2 < 0)), parked))
        # complete P(k+1) = P(k) x P(k)^-1 at E2
        n2 = 0
        for c2, i2 in Pinv:
            R.apply(c2, i2)
            n2 += 1
        rec['Pnext_all_admissible'] = n2 == len(Pinv)
        rec['Pnext_digest'] = st.digest()
        _, K3 = lattice_chambers(st)
        parked_next = np.flatnonzero(lat0 & (K3 < 0))
        rec['Pnext_parked'] = int(len(parked_next))
        rec['Pnext_parked_ids'] = parked_next.tolist()
        rec['Pnext_bprime_on'] = int((K3[off0] >= 0).sum())
        rec['step_set_equal_alpha'] = bool(np.array_equal(parked_next, enter_alpha))
        rec['step_set_equal_sim'] = bool(np.array_equal(parked_next, inT))
        rec['step_count_3_plus_bh_out'] = bool(row is not None and len(parked_next) == 3 + row['bh_out'])
        for _ in range(len(Pinv) + 1):
            R.undo()
        log('step', x, 'enter', len(enter_alpha), 'inT', len(inT), 'parked_next', len(parked_next))
    else:
        # level 4: apply F = (162,1) and P4^-1 for information (W = P4 F P4^-1 at E2)
        F = (162, 1)
        rec['F_is_safe_letter'] = F in G.index
        R.apply(*F)
        ch2, K2 = lattice_chambers(st)
        lat2 = np.flatnonzero(K2 >= 0)
        rec['afterF_on_K_in_T_ids'] = np.sort(lat2[T[ch2[lat2]]]).tolist()
        rec['afterF_Bprime_outside_T'] = bool((K2[off0] >= 0).all() and not T[ch2[off0]].any())
        rec['afterF_parked_unchanged'] = bool(np.array_equal(np.flatnonzero(lat0 & (K2 < 0)), parked))
        for c2, i2 in Pinv:
            R.apply(c2, i2)
        _, K3 = lattice_chambers(st)
        rec['W_parked_ids'] = np.flatnonzero(lat0 & (K3 < 0)).tolist()
        rec['W_bprime_on'] = int((K3[off0] >= 0).sum())
        rec['W_digest'] = st.digest()
        for _ in range(len(Pinv) + 1):
            R.undo()
    for _ in range(len(Pk)):
        R.undo()
    rec['undo_back_to_E2'] = st.digest() == d0
    rec['sec'] = round(time.time() - T0, 1)
    out.write_text(json.dumps(rec) + '\n')
    log('done', rec['sec'])


if __name__ == '__main__':
    main()
