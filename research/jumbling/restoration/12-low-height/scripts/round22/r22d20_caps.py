# Theory round 22, line (d), round 3 (the integrator's check for R22D-20): at the seed 7102 stop after R22D-16 (state
# dceb9359...), the clean caps that hold each remaining high unit whole (all of them, and those of the unit's own
# branch), and the straddlers of caps 150 and 179 with their left heights.
# The integrator's computation; output in runs/round22/r22d20_caps.txt. About 1 CPU minute.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d20_caps.py
import sys,json,runpy
from pathlib import Path
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import numpy as np
z=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/r22d16_step.py')
st,M,C=z['st'],z['M'],z['C']
print('digest',st.digest(),flush=True)
old=json.loads(Path('research/jumbling/restoration/12-low-height/runs/eb6-k100.json').read_text())['units']
info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
high={p for p in range(len(st.pose_id)) if info[int(st.pose_id[p])][0]>=4}
groups={str(u['first']):[p for p in u['piece_ids'] if p in high] for u in old}
groups={u:ps for u,ps in groups.items() if ps}
branches=[E.T.branch(tuple(n)) for n in C.data.N]
clean={}
for c in range(600):
    cl=st.classify(c,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
    if cl.straddle is None:clean[c]=set(int(p) for p in cl.inside)
print('clean caps',len(clean),flush=True)
for u,ps in groups.items():
    hb={(int(info[int(st.pose_id[p])][0]),int(info[int(st.pose_id[p])][2])) for p in ps}
    b=next(iter(hb))[1]
    caps=[c for c,ins in clean.items() if all(p in ins for p in ps)]
    print('unit',u,len(ps),'height/branch',sorted(hb),'clean_containing',caps,'own_branch',[c for c in caps if branches[c]==b],flush=True)
for c in (150,179):
    S=E.straddler_set(st,c)
    print('cap',c,'straddlers',S.tolist(),'heights',[int(info[int(st.pose_id[p])][0]) for p in S],flush=True)
