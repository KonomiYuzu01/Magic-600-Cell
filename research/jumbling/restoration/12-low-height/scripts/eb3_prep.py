# E-B3, step 1 (T2 positive control at s61c, M1-14): the creator D = W_C^-1 P_C^-1 (the inverted middle and prefix of
# the stage 08 step C = s62m3 = P_C V_C W_C V_C^-1) takes the lattice contents of its source chambers Bad(D) off. Six of
# them are holes at s61c, so D alone creates 86 + 80 pieces. From the state alone, find a SAFE-ALL word V0 after which
# no hole lies in Bad(D) (vacate.Vacate with the roles of holes and pieces exchanged: it moves the holes out of the
# target), then replay V0 D with every twist checked admissible and check what it creates.
# Usage (from the repository root):
#   python3 -I -B eb3_prep.py research/jumbling/theory <s61c pose table json> <s62m3 step001-word.json>
#       <233 pose table json> <out prefix>
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
EC = TH.parent / 'restoration/09-general-method/scripts/echain'
sys.path[:0] = [str(TH), str(TH.parent), str(TH.parent / 'fixtures'), str(EC)]
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
import vacate as VA  # noqa: E402
from exact import matvec  # noqa: E402

O, CL = B.O, B.CL
T0 = time.time()
S61C = '218f823f3b269c2c43146086601f9110274c2ccd98363276d0ea09466b30e669'
S233 = '46496bb1806f5f4078d9d15ed83986938ccb9cb776ba62fd187767f17c662514'


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def regions(st, pieces):
    """Exact regions (frozensets of vertex keys) of the given pieces: pose matrix times home-chamber vertices."""
    reg = CL.ctx.regions
    out = []
    for p in np.asarray(pieces).tolist():
        m = st._pose_matrix(int(st.pose_id[p]))
        vs = []
        for v in reg.vertices(p):
            vs.append(tuple((x.a, x.b, x.d) for x in matvec(m, v)))
        out.append(frozenset(vs))
    return out


def main():
    table, cword, t233, prefix = sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
    res = {}
    R, M = O.start(table)
    st = R.st
    NP = len(st.pose_id)
    assert st.digest() == S61C, 'not the s61c state'
    off0 = B.off_pieces(st)
    pose0 = {int(p): B.matrix_json(st._pose_matrix(int(st.pose_id[p]))) for p in off0.tolist()}
    holes0 = B.hole_set(st)
    C = [tuple(x) for x in json.loads(Path(cword).read_text())['word']]
    assert len(C) == 1749
    P, V, W, Vi = C[:5], C[5:854], C[854:900], C[900:]
    assert [tuple(x) for x in CP.inverse_word(M, V)] == Vi, 'C is not P V W V^-1'
    D = [tuple(x) for x in CP.inverse_word(M, W) + CP.inverse_word(M, P)]
    res['C'] = {'P': len(P), 'V': len(V), 'W': len(W)}
    res['D'] = {'length': len(D), 'word': [list(x) for x in D]}
    bad, alive = CP.bad_chambers(M, D, NP)
    src = np.flatnonzero(bad)
    hs = np.intersect1d(src, holes0)
    res['bad_D'] = {'chambers': int(len(src)), 'unclean': int((~alive).sum()), 'holes_in_it': hs.tolist(),
                    'types_of_its_holes': np.asarray(CL.ctx.regions.orbit_of)[hs].tolist()}
    log(bad_D=res['bad_D'])
    G = B.Alphabet(R, M)
    G.orbits()
    res['safe_all'] = {'grips': len(G.safe), 'letters': len(G.letters), 'orbits': int(len(G.orbit_size)),
                       'holes_in_fixed_orbits': int((G.orbit_size[G.orbit[hs]] == 1).sum())}
    occupied = np.setdiff1d(np.arange(NP), holes0)
    vac = VA.Vacate(G, bad, occupied)          # exchanged roles: vac.occ marks the real holes
    word, steps, ok = vac.run(max_steps=200)
    V0 = G.pairs(word)
    res['V0'] = {'found': bool(ok), 'length': len(V0), 'steps': steps, 'word': [list(x) for x in V0]}
    log(V0={'found': ok, 'length': len(V0)})
    if not ok:
        Path(prefix + '.json').write_text(json.dumps(res, indent=1) + '\n')
        return
    # replay V0 then D on a fresh copy, every twist checked admissible
    R2, _ = O.start(table)
    s2 = R2.st
    for c, i in V0:
        R2.apply(c, i)
    assert B.off_fingerprint(s2) == B.off_fingerprint(st), 'V0 moved an off piece'
    h1 = B.hole_set(s2)
    res['after_V0'] = {'holes_in_bad_D': int(np.isin(h1, src).sum()), 'digest': s2.digest()}
    for c, i in D:
        R2.apply(c, i)
    off2 = B.off_pieces(s2)
    kept = all(B.matrix_json(s2._pose_matrix(int(s2.pose_id[p]))) == pose0[p] for p in pose0)
    created = np.setdiff1d(off2, off0)
    lat0 = np.setdiff1d(np.arange(NP), off0)
    res['after_V0_D'] = {'off': int(len(off2)), 'original_off_kept_exactly': bool(kept),
                         'original_off_still_off': int(np.isin(off0, off2).sum()), 'created': int(len(created)),
                         'created_from_lattice': int(np.isin(created, lat0).sum()), 'digest': s2.digest()}
    # compare the off regions with those of the 233 state, which C^-1 reaches from s61c
    R3, _ = O.start(t233)
    s3 = R3.st
    assert s3.digest() == S233
    a = sorted(map(sorted, regions(s2, off2)))
    b = sorted(map(sorted, regions(s3, B.off_pieces(s3))))
    res['after_V0_D']['off_regions_equal_to_233'] = a == b
    log(after=res['after_V0_D'])
    B.export_pose_table(s2, prefix + '-table.json')
    Path(prefix + '-word.json').write_text(json.dumps({'word': [list(x) for x in V0 + D], 'input_digest': S61C,
                                                       'output_digest': s2.digest()}) + '\n')
    res['seconds'] = round(time.time() - T0, 1)
    Path(prefix + '.json').write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k not in ('D',)}, default=str)[:3000])


if __name__ == '__main__':
    main()
