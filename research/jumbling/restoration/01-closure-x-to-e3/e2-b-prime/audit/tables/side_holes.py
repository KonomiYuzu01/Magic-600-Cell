"""Side check (context only): where are the holes and B' after V3, and where does B' sit relative to the hole
chambers at E2 (float centroids from the read-only home-centroid cache). Writes side_holes.json.
"""
import json
import sys
from pathlib import Path

import numpy as np

SP = Path('<work>')
HERE = SP / 'v26/B4/audit/tables'
TH = SP / 'wt-e3/research/jumbling/theory'
sys.dont_write_bytecode = True
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402

CL, O = BL.CL, BL.O
BPRIME = [16245, 16249, 32088]


def holes_and_ch(st):
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    lat = np.flatnonzero(K >= 0)
    ch = np.full(len(K), -1, np.int64)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    return np.setdiff1d(np.arange(len(K)), ch[lat]), ch


def fmat(m):
    return np.array([[float(x) for x in row] for row in m])


def main():
    R, M = O.start(str(SP / 'v26/B4/E2-table.json'))
    st = R.st
    home = np.load(SP / 'wt-e3/work/jumbling-cache/home_centroids.npy', mmap_mode='r')
    out = {}
    h0, _ = holes_and_ch(st)
    out['holes_E2'] = h0.tolist()
    bc = {p: (fmat(st.pose(p)) @ np.asarray(home[p])).tolist() for p in BPRIME}
    out['Bprime_centroids_E2'] = bc
    out['hole_centroids_home'] = {int(h): np.asarray(home[h]).tolist() for h in h0}
    out['dist_B_to_holes'] = {p: {int(h): float(np.linalg.norm(np.array(bc[p]) - np.asarray(home[h]))) for h in h0}
                              for p in BPRIME}
    # nearest home centroids to each B' centroid
    H = np.asarray(home)
    out['nearest_chambers_to_B'] = {}
    for p in BPRIME:
        d = np.linalg.norm(H - np.array(bc[p]), axis=1)
        idx = np.argsort(d)[:5]
        out['nearest_chambers_to_B'][p] = [[int(i), float(d[i])] for i in idx]
    b0 = {p: st.pose(p) for p in BPRIME}
    V = json.loads((SP / 'v26/B4/c3/parts/V.json').read_text())['word']
    caps_with_holes = []
    for c, i in V:
        caps_with_holes.append({'letter': [c, i], 'hole_chambers_in_cap': [int(h) for h in
                               np.intersect1d(CL.data.cap_members(c), holes_and_ch(st)[0])]})
        R.apply(c, i)
    h8, _ = holes_and_ch(st)
    out['V_letters_caps_with_holes'] = caps_with_holes
    out['holes_after_V'] = h8.tolist()
    out['Bprime_pose_unchanged_after_V'] = all(st.pose(p) == b0[p] for p in BPRIME)
    (HERE / 'side_holes.json').write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps(out)[:3000])


if __name__ == '__main__':
    main()
