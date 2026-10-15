"""Extra replay-side checks with float centroid geometry: holes, chamber occupancy and chamber changes.

A lattice piece occupies the chamber (home region) whose home centroid equals its current centroid
(current centroid = exact pose (as float) applied to its home centroid). Plain replay with R.apply;
the per-twist checks were done by replay_audit.py.

Usage: python3 -I -B extra_chambers.py
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
AUD = Path('<work>/v26/B4/audit/replay')
sys.path.insert(0, str(AUD))
import numpy as np  # noqa: E402
import replay_audit as RA  # noqa: E402

SP, B4, BP = RA.SP, RA.B4, RA.BP
SCALE = 1e6


def main():
    keys = RA.Keys()
    R, M = RA.O.start(str(RA.E2_TABLE))
    st = R.st
    kp = st.ctx.kplus
    kset = {keys.of_key(RA.norm_key(RA.matrix_json(kp.matrix(k)))) for k in range(len(kp.perms))}
    C = np.load(SP / 'wt-e3/work/jumbling-cache/home_centroids.npy')
    Q = np.round(C * SCALE).astype(np.int64)
    home = {}
    dup = 0
    for idx, row in enumerate(map(tuple, Q.tolist())):
        if row in home:
            dup += 1
        home[row] = idx
    res = {'home_centroid_duplicates': dup, 'home_centroid_min_separation_note': None}
    # minimal separation proxy: nearest distinct rounded keys are distinct by construction; record min pairwise
    # distance along a sort on the first coordinate window as a sanity figure
    order = np.lexsort(C.T[::-1])
    dmin = float(np.min(np.linalg.norm(np.diff(C[order], axis=0), axis=1)))
    res['home_centroid_min_adjacent_distance_lexsorted'] = dmin
    offsets = [np.array(o) for o in np.ndindex(3, 3, 3, 3)]

    def chambers(codes):
        isK = np.zeros(len(keys.keys) + 1000, bool)
        isK[list(kset)] = True
        X = np.einsum('nij,nj->ni', keys.farr()[codes], C)
        XQ = np.round(X * SCALE).astype(np.int64)
        lat = np.flatnonzero(isK[codes])
        ch = np.full(len(codes), -1, np.int64)
        miss = 0
        for p in lat.tolist():
            c = home.get(tuple(XQ[p].tolist()))
            if c is None:
                for o in offsets:
                    c = home.get(tuple((XQ[p] + o - 1).tolist()))
                    if c is not None:
                        break
            if c is None:
                miss += 1
                continue
            ch[p] = c
        err = float(np.max(np.linalg.norm(X[lat] - C[ch[lat]], axis=1))) if miss == 0 else None
        occ = ch[ch >= 0]
        injective = len(np.unique(occ)) == len(occ)
        holes = np.setdiff1d(np.arange(len(codes)), occ)
        return ch, {'lattice': int(len(lat)), 'unmatched': miss, 'max_match_error': err, 'injective': bool(injective),
                    'holes': holes.tolist() if len(holes) <= 20 else int(len(holes)),
                    'Bprime_chambers': [int(ch[p]) for p in BP]}

    snaps = {}
    c0 = keys.codes(st)
    ch0, info = chambers(c0)
    snaps['E2'] = info
    parts = [(n, B4 / 'c3/parts' / f'{n}.json') for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')]
    for name, path in parts:
        for c, i in json.loads(path.read_text())['word']:
            R.apply(c, i)
        cc = keys.codes(st)
        ch, info = chambers(cc)
        info['digest'] = st.digest()[:16]
        target = [23354, 23362, 38667]
        info['target_chambers_occupied_by_lattice'] = [int(t) for t in target if (ch == t).any()]
        snaps[f'after_{name}'] = info
        if name == 'Vinv':
            ch3, c3 = ch, cc
    lat0 = ch0 >= 0
    moved = np.flatnonzero(lat0 & (c3 != c0))
    res['E2_lattice_changed_exact_pose'] = int(len(moved))
    res['of_them_changed_chamber'] = int((ch0[moved] != ch3[moved]).sum())
    res['E2_lattice_same_exact_pose_but_changed_chamber'] = int((lat0 & (c3 == c0) & (ch0 != ch3)).sum())
    res['states'] = snaps
    (AUD / 'extra-chambers.json').write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps(res))


if __name__ == '__main__':
    main()
