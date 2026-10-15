"""Type-S closure for B' at the current state: U = P Q' P^-1 with P a restoring double commutator (dcomm2 row) and Q' a
SAFE-ALL word at the state after P (reads only the state's pose table).

For each P: apply it at the state (every twist checked admissible); record the off pieces (the parked set C and B'), the
holes, and the target Bad(P^-1) (word-only chamber ghost). Rebuild the SAFE-ALL alphabet after P (the parked pieces now
block grips) and test N1/N2 for the target with the holes after P. With --construct, for a P that passes: peeling
certificate, greedy Q' (Theorem B), then replay Q' and P^-1 and compare the end with the start: B' restored, collateral,
lattice pieces of the start at their exact pose. Each P is undone before the next.

Usage: python -I typeS.py <theory dir> <pose table> <P list json> <out jsonl> [--construct [--greedy]] [--words-dir <dir>]
With --greedy the SAFE-ALL word Q' comes from the greedy of vacate.py instead of a peeling certificate.
"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent)); sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import buffer as B
import conjclose as CC
import conjprep as CP
import ghostlib as GL
import vacate as VA
from sim.kplus import to_tuple

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[1])
    R, M, st = C.R, C.M, C.st
    Ps = json.loads(Path(a[2]).read_text())
    construct = '--construct' in a
    wdir = Path(a[a.index('--words-dir') + 1]) if '--words-dir' in a else None
    codes = CC.Codes()
    c0 = codes.of(st)
    d0 = st.digest()
    off0 = np.flatnonzero(c0 < 0)
    poses0 = {int(p): to_tuple(st.pose(int(p))) for p in off0}
    lat0 = c0 >= 0
    with open(a[3], 'w') as fh:
        for n, P in enumerate(Ps):
            P = [tuple(x) for x in P]
            Pinv = CP.inverse_word(M, P)
            row = {'n': n, 'P': [list(x) for x in P]}
            t = time.time()
            applied = 0
            try:
                for c, i in P:
                    R.apply(c, i)
                    applied += 1
            except RuntimeError as exc:
                row['inadmissible_at'] = applied
                row['error'] = str(exc)[:200]
            if 'inadmissible_at' not in row:
                c1 = codes.of(st)
                parked = np.flatnonzero(lat0 & (c1 < 0))
                bprime_on = int((c1[off0] >= 0).sum())
                badPinv, _, _ = C.bad(Pinv)
                G = B.Alphabet(R, M)
                G.orbits()
                holes = np.asarray(G.holes)
                cond = B.condition(G, badPinv)
                Kp = st._kpose()[st.pose_id]
                bl = np.flatnonzero(Kp >= 0)
                from blocks import CL
                reg, kp = CL.ctx.regions, CL.ctx.kplus
                cham = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])] if bprime_on == len(off0) else None
                row.update({'after_P_digest': st.digest(), 'off_after_P': int((c1 < 0).sum()), 'parked': int(len(parked)),
                            'bprime_on_after_P': bprime_on,
                            'bprime_chambers_after_P': cham.tolist() if cham is not None else None,
                            'bprime_chambers_in_badPinv': int(np.isin(cham, badPinv).sum()) if cham is not None else None,
                            'holes_after_P': int(len(holes)), 'badPinv': int(len(badPinv)),
                            'holes_in_badPinv': int(np.isin(holes, badPinv).sum()),
                            'safe_grips_after_P': len(G.safe), 'letters_after_P': len(G.letters),
                            'orbits_after_P': int(len(G.orbit_size)),
                            'N1': cond['fixed_vacant'], 'N2': cond['necessary'], 'fixed_occupied': cond['fixed_occupied'][:20],
                            'n_fixed_occupied': len(cond['fixed_occupied']),
                            'deficits': cond['necessary_deficits'][:20], 'deficit_total': sum(d['deficit'] for d in cond['necessary_deficits']),
                            'orbits_hit': len(cond['by_orbit']),
                            'tight': sum(1 for r in cond['by_orbit'] if r['holes'] == r['target'])})
                row['screen_sec'] = round(time.time() - t, 1)
                if construct and cond['necessary']:
                    t2 = time.time()
                    try:
                      if '--greedy' in a:
                        mask = np.zeros(G.NP, bool)
                        mask[badPinv] = True
                        va = VA.Vacate(G, mask, G.holes)
                        word, steps, ok = va.run()
                        row['greedy'] = {'ok': bool(ok), 'steps': len(steps), 'length': len(word),
                                         'left': int((va.occ & mask).sum()), 'sec': round(time.time() - t2, 1)}
                        prep = {'word': G.pairs(word)} if ok else None
                      else:
                        cert = CP.generic_certificate(G, badPinv)
                        counts = np.bincount(G.orbit[holes], minlength=len(G.orbit_size))
                        suff = all(counts[int(o)] >= v for o, v in cert['thresholds'].items())
                        row['certificate'] = {'peeled': cert['peeled'], 'of': cert['of'], 'all_reached': cert['all_reached'],
                                              'thresholds_equal_targets': cert['thresholds_equal_targets'],
                                              'sufficient': bool(suff), 'sec': round(time.time() - t2, 1)}
                        prep, _, deficits = CP.generic_prepare(G, cert, badPinv)
                        row['prepare_deficits'] = {str(k): v for k, v in deficits.items()}
                      if True:
                        if prep is not None:
                            Qp = [tuple(x) for x in prep['word']]
                            for c, i in Qp:
                                R.apply(c, i)
                            for c, i in Pinv:
                                R.apply(c, i)
                            ce = codes.of(st)
                            end = {'digest': st.digest(), 'off': int((ce < 0).sum()),
                                   'bprime_restored': int((ce[off0] >= 0).sum()),
                                   'collateral': int((lat0 & (ce < 0)).sum()),
                                   'lattice_same_exact_pose': int((lat0 & (ce == c0)).sum()), 'lattice': int(lat0.sum())}
                            row['Qprime_length'] = len(Qp)
                            row['end'] = end
                            if wdir is not None:
                                wdir.mkdir(parents=True, exist_ok=True)
                                (wdir / f'U-{n}.json').write_text(json.dumps({'P': [list(x) for x in P], 'Qprime': [list(x) for x in Qp],
                                                                             'Pinv': [list(x) for x in Pinv], 'input_digest': d0,
                                                                             'output_digest': end['digest']}) + '\n')
                            for _ in range(len(Qp) + len(Pinv)):
                                R.undo()
                    except Exception as exc:  # noqa: BLE001
                        row['construct_error'] = str(exc)[:300]
                    row['construct_sec'] = round(time.time() - t2, 1)
            for _ in range(applied):
                R.undo()
            if st.digest() != d0:
                raise AssertionError('state not restored after undo')
            fh.write(json.dumps(row) + '\n')
            fh.flush()
            log(**{k: v for k, v in row.items() if k not in ('P', 'fixed_occupied', 'deficits', 'bprime_chambers_after_P')})


if __name__ == '__main__':
    main()
