"""Certificate-first construction for the three-block finishing family, reading only the current pose table.

Inputs: a pose table X, the fixed model (the simulator's) and the template sigma. No generation record or history.
1. certify(X, sigma) (layers3.py): K0, R0, S1-S3 per level, F1-F4, the update formulas against direct ghosts, and
   D = Bad(W), all from word-only ghosts of X's off block and of every chamber.
2. vacancy(X, D): the SAFE-ALL alphabet of X's off geometry, N1, N2, Jordan certificates for the orbits meeting D,
   the peeling certificate and the clearing word V; its predicted holes must equal D.
3. When N1 or N2 fails, the failure certificate: the deficits, the holes in singleton orbits, and for each such hole
   the lock-lemma premises (I_B within caps(x) within I_B + S_B) for X's block.
4. Writes certificate.json; on success the word U = V W V^-1 as parts V, P, F, Pinv, Vinv (the first with X's
   digest), word-full.json and claims.json (3 restored, no collateral, nothing off at the end). Applies nothing.

Usage: python -I constr3.py <theory dir> <v26 dir> <pose table> <template json> <out dir>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import layers3 as L3  # noqa: E402

B, CP, GL = L3.B, L3.CP, L3.GL
T0 = time.time()


def lock_premises(C, xs):
    H = L3.HOME()
    dom0, q0 = C.contents(np.asarray(C.off, np.int64))
    I, S = np.zeros(600, bool), np.zeros(600, bool)
    for c in range(600):
        sd = L3.sides(C, dom0, q0, c)
        S[c] = (sd < 0).any()
        I[c] = not S[c] and (sd == 1).all()
    out = {}
    for x in xs:
        caps = H[x]
        out[str(x)] = {'caps': np.flatnonzero(caps).tolist(), 'I_B_within_caps': bool(caps[I].all()),
                       'caps_within_I_B_or_S_B': bool(not caps[~(I | S)].any())}
        out[str(x)]['locked'] = out[str(x)]['I_B_within_caps'] and out[str(x)]['caps_within_I_B_or_S_B']
    return {'I_B': np.flatnonzero(I).tolist(), 'S_B': np.flatnonzero(S).tolist(), 'chambers': out}


def main():
    a = sys.argv[1:]
    table, tfile, out = a[2], a[3], Path(a[4])
    out.mkdir(parents=True, exist_ok=True)
    sigma = json.loads(Path(tfile).read_text())
    times = {}
    t = time.time()
    C = GL.Ctx(table)
    d_x = C.st.digest()
    cert = L3.certify(C, sigma)
    times['template'] = round(time.time() - t, 1)
    doc = {'input_digest': d_x, 'reads': ['pose table', 'model', 'template'], 'template_certificate': cert}
    if not cert['template_ok']:
        doc['result'] = 'failure: template conditions'
        doc['failure'] = cert['failures']
    else:
        t = time.time()
        D = cert['finish']['D_predicted']
        vac, V, G = L3.vacancy(C, D)
        times['vacancy'] = round(time.time() - t, 1)
        doc['vacancy'] = vac
        if V is None:
            doc['result'] = 'failure: vacancy'
            frozen_holes = [int(h) for h in vac['holes'] if G.orbit_size[G.orbit[h]] == 1]
            doc['failure'] = {'N1': vac['N1'], 'N2': vac['N2'], 'deficits': vac['deficits'],
                              'fixed_occupied': vac['fixed_occupied'], 'holes_in_singleton_orbits': frozen_holes,
                              'lock': lock_premises(C, frozen_holes) if frozen_holes else None,
                              'peeling_error': vac.get('peeling_error')}
        else:
            M = C.M
            W = [tuple(x) for x in cert['W']]
            m = cert['template']['m']
            plen = 2 ** (m + 1) - 1
            parts = [('V', V), ('P', W[:plen]), ('F', W[plen:plen + 1]), ('Pinv', W[plen + 1:]),
                     ('Vinv', CP.inverse_word(M, V))]
            (out / 'parts').mkdir(exist_ok=True)
            for j, (name, part) in enumerate(parts):
                body = {'word': [list(map(int, x)) for x in part]}
                if j == 0:
                    body['input_digest'] = d_x
                (out / 'parts' / f'{j}-{name}.json').write_text(json.dumps(body) + '\n')
            full = [list(map(int, x)) for _, part in parts for x in part]
            (out / 'word-full.json').write_text(json.dumps({'word': full, 'input_digest': d_x}) + '\n')
            (out / 'claims.json').write_text(json.dumps({'restored': 3, 'collateral': 0, 'off_at_end': 0,
                                                         'pose_changed': 0}) + '\n')
            doc['result'] = 'certificate issued'
            doc['U'] = {'length': len(full), 'parts': {n: len(p) for n, p in parts},
                        'length_formula': f'2*|V| + 2^(m+2) - 1 = 2*{len(V)} + {2 ** (m + 2) - 1}',
                        'predicted': {'block_landing': cert['finish']['landing_predicted'],
                                      'block_end_kplus': cert['finish']['block_end']['kplus'],
                                      'holes_before_W': vac['V']['predicted_holes'], 'collateral': 0,
                                      'off_at_end': 0}}
    doc['times'] = times
    doc['seconds'] = round(time.time() - T0, 1)
    (out / 'certificate.json').write_text(json.dumps(doc, indent=1) + '\n')
    L3.log(result=doc['result'], U=doc.get('U', {}).get('length'), times=times)


if __name__ == '__main__':
    main()
