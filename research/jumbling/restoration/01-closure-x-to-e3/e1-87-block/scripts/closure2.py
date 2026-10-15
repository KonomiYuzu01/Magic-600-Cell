"""Composable closure V W V^-1 at the current state, with a pre-construction certificate (reads only the pose table).

W = P Q P^-1 restores block T (size given). Before any twist is applied the script records, at the state S:
- the state: digest, off pieces by block, holes, the exact poses of the protected off pieces (all off pieces outside T);
- the SAFE-ALL menu rebuilt at S (buffer.Alphabet): grips, letters, blocked grips and why, orbits; optionally the
  inheritance of a reference menu (every reference SAFE-ALL grip is SAFE-ALL at S);
- the ghost premises of W: T ends on K, the protected pieces return to their exact contents, Bad(W), T's landing;
  Bad(P^-1) and T's position after P Q against Good(P^-1) (the allowed landing of the inverse part);
- the vacancy premises for the target Bad(W): N1, N2 and the per-orbit demand, then the peeling certificate
  (thresholds per orbit, all routes reached) and the sufficient condition of Theorem B (holes >= thresholds).
Then it constructs V (generic_prepare from the certificate; the greedy of vacate.py if the certificate fails), replays
V, P, Q, P^-1, V^-1 on the simulator (every twist checked admissible), writes each part with its input and output
digests, and compares the end with S: T restored, collateral, protected exact poses, and, for the lattice pieces of S,
lattice kept (on K at the end) and exact pose kept (same K pose) counted apart. Times of each phase are recorded.

Usage: python -I closure2.py <theory dir> <pose table> <block size> <W json {'P','Q'}> <out dir>
       [--ref-safe <json list of grips>] [--greedy]
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
sys.path.insert(0, str(Path(__file__).resolve().parent))
import buffer as B  # noqa: E402
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402
import vacate as VA  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def main():
    a = sys.argv[1:]
    table, size, wfile, out = a[1], int(a[2]), a[3], Path(a[4])
    out.mkdir(parents=True, exist_ok=True)
    ref_safe = json.loads(Path(a[a.index('--ref-safe') + 1]).read_text()) if '--ref-safe' in a else None
    times = {}
    t = time.time()
    C = GL.Ctx(table)
    R, M, st = C.R, C.M, C.st
    Wd = json.loads(Path(wfile).read_text())
    P = [(int(c), int(i)) for c, i in Wd['P']]
    Q = [(int(c), int(i)) for c, i in Wd['Q']]
    Pinv = CP.inverse_word(M, P)
    W = P + Q + Pinv
    T = C.block(size)
    prot = np.setdiff1d(C.off, T)
    d_s = st.digest()
    holes0 = np.flatnonzero(~C.occ)
    pre = {'state_digest': d_s, 'off': int(len(C.off)), 'blocks': [sorted(map(int, b)) for b in C.bl.blocks],
           'target_block': T.tolist(), 'protected_off': prot.tolist(),
           'protected_poses': {int(p): matrix_json(st.pose(int(p))) for p in prot.tolist()},
           'holes': holes0.tolist(), 'W': {'P': [list(x) for x in P], 'Q': [list(x) for x in Q],
                                          'P_inverse': [list(x) for x in Pinv]}}
    # ghost premises of W
    g, n, U = C.ghost(W, np.concatenate([T, prot]))
    nT = len(T)
    d0, q0 = C.contents(prot)
    lat = slice(n, None)
    badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
    gpq, _, _ = C.ghost(P + Q, T, np.zeros(0, np.int64))
    badPinv, _, _ = C.bad(Pinv)
    on_pq = gpq.alive[:nT] & (gpq.dom[:nT] == 0)
    pre['ghost'] = {'T_on_lattice_at_end': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()), 'T_size': nT,
                    'protected_exact_at_end': bool(g.alive[nT:n].all() and np.array_equal(g.dom[nT:n], d0)
                                                   and np.array_equal(g.q[nT:n], q0)),
                    'badW': badW.tolist(), 'badW_size': int(len(badW)),
                    'badW_occupied': int(C.occ[badW].sum()), 'T_landing_chambers': sorted(g.q[:nT].tolist()),
                    'T_landing_in_badW': int(np.isin(g.q[:nT], badW).sum()),
                    'badPinv_size': int(len(badPinv)),
                    'T_after_PQ_on_lattice': int(on_pq.sum()),
                    'T_after_PQ_in_badPinv': int(np.isin(gpq.q[:nT][on_pq], badPinv).sum())}
    times['ghost_premises'] = round(time.time() - t, 1)
    log(ghost={k: v for k, v in pre['ghost'].items() if not isinstance(v, list)})
    # SAFE-ALL menu rebuilt at S
    t = time.time()
    G = B.Alphabet(R, M)
    G.orbits()
    pre['menu'] = {'safe_grips': list(map(int, G.safe)), 'safe_grip_count': len(G.safe), 'letters': len(G.letters),
                   'blocked': {str(k): v for k, v in G.blocked.items()}, 'orbits': int(len(G.orbit_size)),
                   'singleton_orbits': int((G.orbit_size == 1).sum()), 'holes': int(len(G.holes))}
    if ref_safe is not None:
        ref = set(int(x) for x in ref_safe)
        pre['menu']['reference_safe_grips'] = len(ref)
        pre['menu']['reference_safe_not_safe_here'] = sorted(ref - set(G.safe))
        pre['menu']['safe_here_not_in_reference'] = sorted(set(G.safe) - ref)
    cond = B.condition(G, badW)
    pre['vacancy'] = {'N1': cond['fixed_vacant'], 'fixed_occupied': cond['fixed_occupied'],
                      'N2': cond['necessary'], 'deficits': cond['necessary_deficits'], 'by_orbit': cond['by_orbit']}
    times['menu_and_condition'] = round(time.time() - t, 1)
    log(menu={k: v for k, v in pre['menu'].items() if not isinstance(v, (list, dict))},
        N1=cond['fixed_vacant'], N2=cond['necessary'])
    # peeling certificate (Theorem B) and its sufficient condition
    V, source = None, None
    t = time.time()
    if cond['necessary'] and '--greedy' not in a:
        try:
            cert = CP.generic_certificate(G, badW)
            counts = np.bincount(G.orbit[G.holes], minlength=len(G.orbit_size))
            suff = {o: {'threshold': int(v), 'holes': int(counts[int(o)])} for o, v in cert['thresholds'].items()}
            pre['certificate'] = {'peeled': cert['peeled'], 'of': cert['of'], 'fixed': cert['fixed'],
                                  'thresholds': suff, 'thresholds_equal_targets': cert['thresholds_equal_targets'],
                                  'all_reached': cert['all_reached'], 'family_bound': cert['family_bound'],
                                  'sufficient': all(x['holes'] >= x['threshold'] for x in suff.values())}
            (out / 'certificate.json').write_text(json.dumps(cert) + '\n')
            times['certificate'] = round(time.time() - t, 1)
            log(certificate={k: v for k, v in pre['certificate'].items() if k != 'thresholds'})
            t = time.time()
            prep, _, deficits = CP.generic_prepare(G, cert, badW)
            pre['certificate']['prepare_deficits'] = {str(k): v for k, v in deficits.items()}
            if prep is not None:
                V, source = [tuple(x) for x in prep['word']], 'certificate'
            times['prepare'] = round(time.time() - t, 1)
        except Exception as exc:  # noqa: BLE001
            pre['certificate_error'] = str(exc)[:400]
            log(certificate_error=pre['certificate_error'])
    if V is None and cond['necessary']:
        t = time.time()
        mask = np.zeros(G.NP, bool)
        mask[badW] = True
        va = VA.Vacate(G, mask, G.holes)
        word, steps, ok = va.run()
        pre['greedy'] = {'ok': ok, 'steps': len(steps), 'length': len(word), 'left': int((va.occ & mask).sum())}
        times['greedy'] = round(time.time() - t, 1)
        if ok:
            V, source = [tuple(x) for x in G.pairs(word)], 'greedy'
    (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
    if V is None:
        (out / 'result.json').write_text(json.dumps({'result': 'no V', 'times': times}, indent=1) + '\n')
        log(result='no V', times=times)
        return
    log(V=len(V), source=source)
    # replay
    t = time.time()
    codes = CC.Codes()
    c_s = codes.of(st)
    poses_s = {}
    for p in np.flatnonzero(c_s < 0).tolist():
        poses_s[p] = to_tuple(st.pose(int(p)))
    parts = [('V', V), ('P', P), ('Q', Q), ('Pinv', Pinv), ('Vinv', CP.inverse_word(M, V))]
    rows = []
    (out / 'parts').mkdir(exist_ok=True)
    for name, part in parts:
        d_in = st.digest()
        for c, i in part:
            R.apply(c, i)
        d_out = st.digest()
        c_now = codes.of(st)
        if name == 'V':
            occ_v = np.ones(G.NP, bool)
            occ_v[B.hole_set(st)] = False
            after_v = {'occupied_badW': int(occ_v[badW].sum()),
                       'off_set_unchanged': bool(np.array_equal(c_now < 0, c_s < 0)),
                       'off_exact_poses_unchanged': all(to_tuple(st.pose(p)) == m for p, m in poses_s.items())}
        (out / 'parts' / f'{name}.json').write_text(json.dumps({'word': [list(x) for x in part], 'input_digest': d_in,
                                                                'output_digest': d_out}) + '\n')
        rows.append({'part': name, 'twists': len(part), 'input_digest': d_in, 'output_digest': d_out,
                     'off_end': int((c_now < 0).sum())})
        log(part=name, twists=len(part), off=rows[-1]['off_end'])
    times['replay'] = round(time.time() - t, 1)
    c_e = codes.of(st)
    inT = np.zeros(len(c_s), bool)
    inT[T] = True
    off_s, off_e = c_s < 0, c_e < 0
    lat_s = ~off_s
    end = {'digest': st.digest(), 'off': int(off_e.sum()), 'T_restored': int((inT & ~off_e).sum()),
           'collateral': int((lat_s & off_e).sum()),
           'protected_still_off': int((off_s & ~inT & off_e).sum()),
           'protected_exact_pose_changed': sum(1 for p, m in poses_s.items() if not inT[p]
                                               and (not off_e[p] or to_tuple(st.pose(int(p))) != m)),
           'lattice_of_S_on_K_at_end': int((lat_s & ~off_e).sum()), 'lattice_of_S': int(lat_s.sum()),
           'lattice_of_S_same_exact_pose': int((lat_s & (c_e == c_s)).sum())}
    full = [list(x) for _, part in parts for x in part]
    (out / 'word-full.json').write_text(json.dumps({'word': full, 'input_digest': d_s,
                                                    'output_digest': end['digest']}) + '\n')
    res = {'input_digest': d_s, 'V_source': source, 'after_V': after_v, 'parts': rows, 'end': end,
           'length': len(full), 'times': times, 'seconds': round(time.time() - T0, 1)}
    (out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
    log(end=end, length=len(full), times=times)


if __name__ == '__main__':
    main()
