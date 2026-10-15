"""Shrink the bubble around B' by iterated conjugation, reading only the current pose table.

Level 0: P_0 = R, a single letter that puts B' on K (it parks its whole cap). At level k the script applies P_k at the
state (every twist checked admissible), rebuilds the SAFE-ALL alphabet at Y_k = state . P_k, and computes
T_k = Bad(P_k^-1) (word-only chamber ghost), the holes split into bad holes BH_k (inside T_k) and good holes GH_k,
and B''s chambers L_k. For every SAFE-ALL letter x whose cap holds a chamber of L_k it records:
- F test: x keeps every bad hole inside T_k and moves every chamber of L_k out of T_k (then x is a finishing word F
  of constructS.py: U = P_k Q0 x P_k^-1);
- moves_L_out: x moves all of L_k out of T_k; bh_out: bad holes x moves out of T_k (the pieces P_{k+1} parks are
  these plus B''s 3 chambers).
- enter_pieces: lattice pieces x moves into T_k (P_{k+1} parks them); kill_violations: pieces x moves into chambers of
  T_k whose ghost straddles a cut under P_k^-1 (P_{k+1} would be inadmissible).
The letter with all of L_k moved out, no kill violation and the fewest entering pieces (ties: fewest bad holes out,
least grip, then menu index) extends the word: P_{k+1} = P_k x P_k^-1. B' stays on K through P_{k+1} because x moves it out of T_k = Bad(P_k^-1).
The run stops at an F letter, at a level with no letter that moves L_k out, or at the level limit. Each level is
undone before the next; the state digest is checked.

Usage: python -I shrink.py <theory dir> <scripts dir> <pose table> <R json pair> <levels> <out jsonl>
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
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[2])
    R, M, st = C.R, C.M, C.st
    from blocks import CL
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    codes = CC.Codes()
    c0 = codes.of(st)
    d0 = st.digest()
    off0 = np.flatnonzero(c0 < 0)
    lat0 = c0 >= 0
    P = [tuple(json.loads(a[3]))]
    levels = int(a[4])
    with open(a[5], 'w') as fh:
        for k in range(levels + 1):
            t = time.time()
            Pinv = CP.inverse_word(M, P)
            badPinv, gb, chb = C.bad(Pinv)
            kill = np.zeros(len(C.occ), bool)
            kill[chb[~gb.alive]] = True
            applied = 0
            row = {'level': k, 'P_length': len(P), 'P': [list(x) for x in P]}
            try:
                for c, i in P:
                    R.apply(c, i)
                    applied += 1
            except RuntimeError as exc:
                row['inadmissible_at'] = applied
                row['error'] = str(exc)[:200]
                fh.write(json.dumps(row) + '\n')
                log(**{x: y for x, y in row.items() if x != 'P'})
                break
            c1 = codes.of(st)
            Kp = st._kpose()[st.pose_id]
            bon = int((Kp[off0] >= 0).sum())
            parked = int((lat0 & (c1 < 0)).sum())
            row.update({'after_P_digest': st.digest(), 'bprime_on': bon, 'parked': parked, 'T': int(len(badPinv)),
                        'T_straddle': int(kill.sum())})
            if bon != len(off0):
                row['stop'] = "B' not on K"
                fh.write(json.dumps(row) + '\n')
                log(**{x: y for x, y in row.items() if x != 'P'})
                break
            L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
            G = B.Alphabet(R, M)
            holes = np.asarray(G.holes)
            T = np.zeros(G.NP, bool)
            T[badPinv] = True
            isBH = np.zeros(G.NP, bool)
            isBH[holes[T[holes]]] = True
            BH = np.flatnonzero(isBH)
            GH = holes[~T[holes]]
            isL = np.zeros(G.NP, bool)
            isL[L] = True
            cand = []
            for n, (c, i, _, ys) in enumerate(G.letters):
                xs = G.capx[c]
                if not isL[xs].any():
                    continue
                img = ys  # content at xs[j] moves to ys[j]
                bh_in_cap = int(isBH[xs].sum())
                bh_out = int((isBH[xs] & ~T[img]).sum())
                l_out = int((isL[xs] & ~T[img]).sum())
                occ_src = G.occupied[xs]
                enter = int((T[img] & ~T[xs] & occ_src).sum())
                kill_viol = int((kill[img] & occ_src & ~isL[xs]).sum()) + int((kill[img] & isL[xs]).sum())
                cand.append({'letter': [int(c), int(i)], 'cap_size': int(len(xs)), 'L_in_cap': int(isL[xs].sum()),
                             'bh_in_cap': bh_in_cap, 'bh_out': bh_out, 'L_out': l_out, 'enter_pieces': enter,
                             'kill_violations': kill_viol,
                             'F': bool(bh_out == 0 and l_out == len(L) and isL[xs].sum() == len(L))})
            row.update({'L': L.tolist(), 'L_in_T': int(T[L].sum()), 'holes': int(len(holes)), 'BH': int(len(BH)),
                        'GH': GH.tolist(), 'safe_grips': len(G.safe), 'letters': len(G.letters),
                        'grips_holding_L': sorted({x['letter'][0] for x in cand}),
                        'candidates': len(cand), 'F_letters': [x['letter'] for x in cand if x['F']],
                        'min_bh_in_cap': min((x['bh_in_cap'] for x in cand), default=None)})
            ok = [x for x in cand if x['L_out'] == len(L) and x['L_in_cap'] == len(L) and x['kill_violations'] == 0]
            ok.sort(key=lambda x: (x['enter_pieces'], x['bh_out'], x['letter'][0], x['letter'][1]))
            row['best'] = ok[:6]
            row['level_sec'] = round(time.time() - t, 1)
            for _ in range(applied):
                R.undo()
            if st.digest() != d0:
                raise AssertionError('state not restored after undo')
            fh.write(json.dumps(row) + '\n')
            fh.flush()
            log(**{x: y for x, y in row.items() if x not in ('P', 'L', 'GH', 'best')}, best=ok[:2])
            if row['F_letters'] or not ok or k == levels:
                break
            x = tuple(ok[0]['letter'])
            P = P + [x] + Pinv


if __name__ == '__main__':
    main()
