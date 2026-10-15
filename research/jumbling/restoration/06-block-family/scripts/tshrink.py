"""Type-S search for one target block T of a state (menu I_a), reading only the pose table.

Generalizes the B' construction at E2 (segment-3 shrink.py) to a target block T among other off pieces.
1. Single letters R = (g, i): g a grip whose cap holds all of T with no off piece straddling it (exact classify),
   i a menu item with E(g, i) outside K+ and E(g, i) X_p in K+ for every piece p of T (exact). R puts T on K.
2. Iterated conjugation from P_0 = R: at level k apply P_k (every twist checked admissible), check T on K and every
   other off piece still off, rebuild the SAFE-ALL alphabet at Y_k = state . P_k (caps with no off piece and no
   straddle), compute T_k = Bad(P_k^-1) (word-only chamber ghost), the bad holes BH_k (holes in T_k) and T's
   chambers L_k. A SAFE-ALL letter x whose cap holds all of L_k is
   - a finishing letter F when it moves all of L_k out of T_k and keeps every bad hole inside T_k;
   - a step letter when it moves all of L_k out of T_k with no kill violation (no piece moved into a chamber of T_k
     whose ghost straddles under P_k^-1); the step letter with the fewest entering pieces extends
     P_{k+1} = P_k x P_k^-1.
   Each level is undone and the digest checked. Stops at F, at a level without a step letter, or at the level limit.
Other off pieces in R's cap move with P_k and come back with P_k^-1, because no SAFE-ALL letter touches an off piece.
Usage: python -I tshrink.py <theory dir> <E-chain scripts dir> <pose table> <target json {pieces, movers}>
       <levels> <out jsonl> [--max-R n] [--R g,i]
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
from exact import matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def single_letters(C, T, movers):
    R, M, st = C.R, C.M, C.st
    kp = GL.BL.CL.ctx.kplus
    restore = GL.restore
    poses = [st.pose(int(p)) for p in T.tolist()]
    distinct = {to_tuple(m): m for m in poses}
    out = []
    for g in movers:
        ins = R.inside(int(g))
        if ins is None:
            continue
        ins = set(np.asarray(ins).tolist())
        if not set(T.tolist()) <= ins:
            continue
        others = len(ins - set(T.tolist()))
        for i in range(len(M.items)):
            e = restore.element(M, int(g), i)
            if kp.index_of_matrix(to_tuple(e)) >= 0:
                continue
            if all(kp.index_of_matrix(to_tuple(matmul(e, m))) >= 0 for m in distinct.values()):
                out.append({'R': [int(g), int(i)], 'cap_pieces': len(ins), 'other_pieces_in_cap': others})
    return out


def shrink(C, codes, T, prot, P, levels, fh, tag):
    R, M, st = C.R, C.M, C.st
    reg, kp = GL.BL.CL.ctx.regions, GL.BL.CL.ctx.kplus
    d0 = st.digest()
    c0 = codes.of(st)
    lat0 = c0 >= 0
    for k in range(levels + 1):
        t = time.time()
        Pinv = CP.inverse_word(M, P)
        badPinv, gb, chb = C.bad(Pinv)
        kill = np.zeros(len(C.occ), bool)
        kill[chb[~gb.alive]] = True
        applied = 0
        row = {'tag': tag, 'level': k, 'P_length': len(P), 'P': [list(x) for x in P]}
        try:
            for c, i in P:
                R.apply(c, i)
                applied += 1
        except RuntimeError as exc:
            row.update({'inadmissible_at': applied, 'stop': 'inadmissible', 'error': str(exc)[:200]})
            for _ in range(applied):
                R.undo()
            fh.write(json.dumps(row) + '\n')
            return row
        c1 = codes.of(st)
        Kp = st._kpose()[st.pose_id]
        ton = int((Kp[T] >= 0).sum())
        prot_on = int((Kp[prot] >= 0).sum()) if len(prot) else 0
        row.update({'T_on': ton, 'T_size': int(len(T)), 'prot_on_after_P': prot_on,
                    'parked': int((lat0 & (c1 < 0)).sum()), 'T': int(len(badPinv)), 'T_straddle': int(kill.sum())})
        if ton != len(T) or prot_on:
            row['stop'] = 'T not on K' if ton != len(T) else 'protected piece on K after P'
            for _ in range(applied):
                R.undo()
            fh.write(json.dumps(row) + '\n')
            return row
        L = reg.piece_of[reg.orbit_of[T], kp.compose_vec(Kp[T], reg.transport[T])]
        G = B.Alphabet(R, M)
        holes = np.asarray(G.holes)
        Tm = np.zeros(G.NP, bool)
        Tm[badPinv] = True
        isBH = np.zeros(G.NP, bool)
        isBH[holes[Tm[holes]]] = True
        isL = np.zeros(G.NP, bool)
        isL[L] = True
        cand = []
        for c, i, _, ys in G.letters:
            xs = G.capx[c]
            if not isL[xs].any():
                continue
            occ_src = G.occupied[xs]
            l_in = int(isL[xs].sum())
            l_out = int((isL[xs] & ~Tm[ys]).sum())
            bh_out = int((isBH[xs] & ~Tm[ys]).sum())
            enter = int((Tm[ys] & ~Tm[xs] & occ_src).sum())
            kv = int((kill[ys] & occ_src & ~isL[xs]).sum()) + int((kill[ys] & isL[xs]).sum())
            cand.append({'letter': [int(c), int(i)], 'L_in_cap': l_in, 'L_out': l_out, 'bh_in_cap': int(isBH[xs].sum()),
                         'bh_out': bh_out, 'enter_pieces': enter, 'kill_violations': kv,
                         'F': bool(bh_out == 0 and l_out == len(L) and l_in == len(L))})
        row.update({'L': L.tolist(), 'L_in_T': int(Tm[L].sum()), 'holes': int(len(holes)), 'BH': int(isBH.sum()),
                    'safe_grips': len(G.safe), 'letters': len(G.letters), 'candidates': len(cand),
                    'grips_holding_L': sorted({x['letter'][0] for x in cand}),
                    'F_letters': [x['letter'] for x in cand if x['F']]})
        ok = [x for x in cand if x['L_out'] == len(L) and x['L_in_cap'] == len(L) and x['kill_violations'] == 0]
        ok.sort(key=lambda x: (x['enter_pieces'], x['bh_out'], x['letter'][0], x['letter'][1]))
        row['best'] = ok[:4]
        row['level_sec'] = round(time.time() - t, 1)
        for _ in range(applied):
            R.undo()
        if st.digest() != d0:
            raise AssertionError('state not restored after undo')
        if row['F_letters']:
            row['stop'] = 'F found'
        elif not ok:
            row['stop'] = 'no step letter'
        elif k == levels:
            row['stop'] = 'level limit'
        fh.write(json.dumps(row) + '\n')
        fh.flush()
        log(tag=tag, level=k, P=len(P), parked=row['parked'], T=row['T'], BH=row['BH'], safe=row['safe_grips'],
            cands=len(cand), steps=len(ok), F=len(row['F_letters']), stop=row.get('stop'))
        if 'stop' in row:
            return row
        x = tuple(ok[0]['letter'])
        P = P + [x] + Pinv


def main():
    a = sys.argv[1:]
    table, tfile, levels, outp = a[2], a[3], int(a[4]), a[5]
    max_r = int(a[a.index('--max-R') + 1]) if '--max-R' in a else 6
    C = GL.Ctx(table)
    st = C.st
    tgt = json.loads(Path(tfile).read_text())
    T = np.asarray(sorted(tgt['pieces']), np.int64)
    off = np.asarray(C.off, np.int64)
    assert np.isin(T, off).all(), 'target pieces must be off the lattice'
    prot = np.setdiff1d(off, T)
    codes = CC.Codes()
    if '--R' in a:
        Rs = [{'R': [int(x) for x in a[a.index('--R') + 1].split(',')]}]
    else:
        Rs = single_letters(C, T, tgt['movers'])
    log(state=st.digest()[:16], off=len(off), target=len(T), single_letters=len(Rs))
    summary = {'state_digest': st.digest(), 'target': T.tolist(), 'protected': int(len(prot)), 'single_letters': Rs,
               'tried': []}
    with open(outp, 'w') as fh:
        for r in Rs[:max_r]:
            row = shrink(C, codes, T, prot, [tuple(r['R'])], levels, fh, tag=f"R={r['R'][0]},{r['R'][1]}")
            summary['tried'].append({'R': r['R'], 'stop': row.get('stop'), 'level': row['level'],
                                     'P_length': row['P_length'], 'F_letters': row.get('F_letters', [])[:8],
                                     'P': row['P'] if row.get('stop') == 'F found' else None})
            if row.get('stop') == 'F found':
                break
    summary['seconds'] = round(time.time() - T0, 1)
    Path(outp + '.summary.json').write_text(json.dumps(summary, indent=1) + '\n')
    print(json.dumps({k: v for k, v in summary.items() if k not in ('target',)})[:3000])


if __name__ == '__main__':
    main()
