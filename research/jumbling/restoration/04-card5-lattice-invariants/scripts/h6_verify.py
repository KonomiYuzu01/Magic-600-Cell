"""Exact check of the height-6 opportunity: replay the I_a W-J fixture for T records, then try every jumble
element at grip E; for each admissible one, apply it and compute the exact left height of every off-lattice
pose (T, trees.pose_tree). Report the height histogram and the pieces at the maximum height."""
import json, sys, time
from pathlib import Path
from collections import Counter
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim, wj, trees
import restore as RS
ctx = sim.get_context()
T, E = int(sys.argv[2]), int(sys.argv[3])
fixture, M = wj.load(ctx, 'I_a')
recs = fixture['journal']['records']
st = sim.State(ctx, menu=M)
for rec in recs[:T]:
    assert st.apply(sim.Twist.from_record(ctx, rec)).applied
def heights(st):
    cnt = np.bincount(st.pose_id, minlength=len(st._poses))
    h = Counter(); top = {}
    for pid in np.nonzero(cnt)[0]:
        P = st._poses[int(pid)]
        if P.kidx >= 0:
            h[0] += int(cnt[pid]); continue
        dl, dr, bl, br = trees.pose_tree(st._pose_matrix(int(pid)))
        assert dr == 0
        h[dl] += int(cnt[pid])
        top.setdefault(dl, []).append(int(pid))
    return dict(sorted(h.items())), top
h0, _ = heights(st)
print('after', T, 'records: height histogram', h0, flush=True)
JI = RS.jumble_items(M)
out = []
for i in JI:
    tw = sim.Twist(ctx, E, RS.element(M, E, i), 'menu', {'menu': M.name})
    r = st.apply(tw)
    if not r.applied:
        out.append({'item': i, 'admissible': False}); continue
    h, top = heights(st)
    mx = max(h)
    pcs = sorted(np.flatnonzero(np.isin(st.pose_id, top[mx])).tolist()) if mx > 0 else []
    out.append({'item': i, 'admissible': True, 'hist': h, 'max': mx, 'n_max': len(pcs), 'max_pieces_head': pcs[:10]})
    print(json.dumps(out[-1]), flush=True)
    assert st.apply(tw.inverse()).applied
json.dump({'T': T, 'grip': E, 'hist_before': h0, 'trials': out}, open(sys.argv[4], 'w'), indent=1)
