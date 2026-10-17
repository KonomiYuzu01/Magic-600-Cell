# E-B3, steps 2 and 3: from Z1 = V0 D (s61c) (eb3_prep.py), the rest of the three-phase template with catalogue cores
# and retained phases found from the state alone:
#   middle   P_S V1 W_S V1^-1   (P_S, W_S: prefix and middle of the stage 08 step s62m = P_S V_S W_S V_S^-1);
#   last     V2 W_R V2^-1       (W_R: the 288-letter pure twist inside the stage 09 step 14, Bad(W_R) = Omega_X);
# where V1 and V2 are SAFE-ALL words read from the holes at the state that vacate Bad(W): the peeling certificate
# of Theorem B first, then the two-pass greedy search of vacate2.py with a time limit (as tclose2.py). Every twist
# is replayed and checked admissible; the end must have no off piece.
# Usage (from the repository root):
#   python3 -I -B eb3_finish.py research/jumbling/theory <Z1 pose table json> <s62m step001-word.json>
#       <stage 09 step014-word.json> <out prefix> [--greedy-seconds N]
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
EC = TH.parent / 'restoration/09-general-method/scripts/echain'
GM = TH.parent / 'restoration/09-general-method/scripts/gm'
sys.path[:0] = [str(TH), str(TH.parent), str(TH.parent / 'fixtures'), str(EC), str(GM)]
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
import vacate2 as VA2  # noqa: E402

O, CL = B.O, B.CL
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def vacating_word(R, M, W, NP, greedy_seconds):
    """A SAFE-ALL word read from the holes at R's state after which Bad(W) holds no lattice piece: the peeling
    certificate of Theorem B first (conjprep.generic_certificate and generic_prepare), then the two-pass greedy of
    vacate2.py with a time limit, as tclose2.py does."""
    bad, alive = CP.bad_chambers(M, W, NP)
    G = B.Alphabet(R, M)
    G.orbits()
    target = np.flatnonzero(bad)
    cond = B.condition(G, target)
    info = {'bad': int(len(target)), 'unclean': int((~alive).sum()),
            'occupied_in_bad': int((~np.isin(target, G.holes)).sum()), 'holes': int(len(G.holes)),
            'N1': bool(cond['fixed_vacant']), 'N1_N2': bool(cond['necessary']),
            'safe_grips': len(G.safe), 'orbits': int(len(G.orbit_size))}
    log(vacancy=info)
    if not cond['necessary']:
        return None, info
    if info['occupied_in_bad'] == 0:
        info['source'] = 'empty (Bad(W) vacant)'
        return [], info
    t = time.time()
    try:
        cert = CP.generic_certificate(G, target)
        prep, cond2, deficits = CP.generic_prepare(G, cert, target)
        info['certificate'] = {'peeled': cert['peeled'], 'of': cert['of'], 'family_bound': cert['family_bound'],
                               'deficits': {str(k): v for k, v in deficits.items()},
                               'seconds': round(time.time() - t, 1)}
        log(certificate=info['certificate'])
        if prep is not None:
            info['certificate']['length'] = prep['length']
            info['source'] = 'certificate'
            return [tuple(x) for x in prep['word']], info
    except Exception as exc:  # noqa: BLE001
        info['certificate_error'] = str(exc)[:400]
        log(certificate_error=info['certificate_error'])
    mask = np.zeros(G.NP, bool)
    mask[target] = True
    va = VA2.Vacate2(G, mask, G.holes)
    deadline = time.time() + greedy_seconds
    gain0 = va.gain

    def gain(word):
        if time.time() > deadline:
            raise TimeoutError
        return gain0(word)
    va.gain = gain
    try:
        word, steps, ok = va.run(max_steps=1000)
        timed_out = False
    except TimeoutError:
        word, steps, ok, timed_out = [], [], False, True
    info['greedy'] = {'found': bool(ok), 'length': len(word), 'steps': len(steps), 'timed_out': timed_out,
                      'seconds_limit': greedy_seconds}
    if ok:
        info['source'] = 'greedy'
        return G.pairs(word), info
    return None, info


def main():
    table, sword, rword, prefix = sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
    gsec = float(sys.argv[sys.argv.index('--greedy-seconds') + 1]) if '--greedy-seconds' in sys.argv else 1200.0
    R, M = O.start(table)
    st = R.st
    NP = len(st.pose_id)
    d0 = st.digest()
    off0 = B.off_pieces(st)
    lat0 = np.setdiff1d(np.arange(NP), off0)
    S = [tuple(x) for x in json.loads(Path(sword).read_text())['word']]
    assert len(S) == 1822
    P_S, V_S, W_S, Vi_S = S[:4], S[4:890], S[890:936], S[936:]
    assert [tuple(x) for x in CP.inverse_word(M, V_S)] == Vi_S, 'S is not P V W V^-1'
    Rw = [tuple(x) for x in json.loads(Path(rword).read_text())['word']]
    assert len(Rw) == 1374
    V_R, W_R, Vi_R = Rw[:543], Rw[543:831], Rw[831:]
    assert [tuple(x) for x in CP.inverse_word(M, V_R)] == Vi_R, 'R is not V W V^-1'
    res = {'input_digest': d0, 'off_start': int(len(off0)), 'cores': {'P_S': len(P_S), 'W_S': len(W_S),
                                                                       'W_R': len(W_R)}}
    full = []

    def run(word, label):
        for c, i in word:
            R.apply(c, i)
        full.extend(word)
        off = B.off_pieces(st)
        res.setdefault('trace', []).append({'after': label, 'letters': len(full), 'off': int(len(off)),
                                            'lattice_of_start_off': int(np.isin(lat0, off).sum())})
        log(after=label, letters=len(full), off=int(len(off)))

    run(P_S, 'P_S')
    V1, info1 = vacating_word(R, M, W_S, NP, gsec)
    res['V1'] = info1
    if V1 is None:
        Path(prefix + '.json').write_text(json.dumps(res, indent=1) + '\n')
        return
    res['V1']['length'] = len(V1)
    run(V1, 'V1')
    run(W_S, 'W_S')
    run([tuple(x) for x in CP.inverse_word(M, V1)], 'V1^-1')
    V2, info2 = vacating_word(R, M, W_R, NP, gsec)
    res['V2'] = info2
    if V2 is None:
        Path(prefix + '.json').write_text(json.dumps(res, indent=1) + '\n')
        return
    res['V2']['length'] = len(V2)
    run(V2, 'V2')
    run(W_R, 'W_R')
    run([tuple(x) for x in CP.inverse_word(M, V2)], 'V2^-1')
    res['end'] = {'digest': st.digest(), 'off': int(len(B.off_pieces(st))), 'letters': len(full)}
    Path(prefix + '-word.json').write_text(json.dumps({'word': [list(x) for x in full], 'input_digest': d0,
                                                       'output_digest': st.digest()}) + '\n')
    res['seconds'] = round(time.time() - T0, 1)
    Path(prefix + '.json').write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps(res)[:4000])


if __name__ == '__main__':
    main()
