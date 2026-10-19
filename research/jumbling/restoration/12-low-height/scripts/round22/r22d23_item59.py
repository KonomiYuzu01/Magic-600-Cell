# Theory round 22, line (d), round 3 (the integrator's check for R22D-23): at the depth 100 stop of seed 7108, after
# the lowering letter (415, 59), the state digest, the heights and branches of the 111 lowered pieces, the free grips of
# their new class (clean, of that branch, holding no off piece of another type) and those among them that hold the
# whole source cell, and the number of SAFE grips (clean, holding no off piece).
# The integrator's computation; output in runs/round22/r22d23_item59.txt. About 1 CPU minute.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d23_item59.py
import sys,json
from pathlib import Path
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import numpy as np
r=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round3/ia-k100-s7108.json').read_text())
C=E.sim.get_context();M=E.R.menu('I_a');st=E.sim.State(C,filtered=False,menu=M)
for c,j in r['history']+r['moves']:
    o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
    assert o.applied and not o.uncertain
print('stop',st.digest(),flush=True)
info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
high=[p for p in range(len(st.pose_id)) if info[int(st.pose_id[p])][0]>=4]
print('high',len(high),flush=True)
o=st.apply(E.sim.Twist(C,415,E.R.element(M,415,59),'menu',{'menu':M.name}),journal=False)
assert o.applied and not o.uncertain
print('after (415, 59)',st.digest(),flush=True)
info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
hb=sorted({(int(info[int(st.pose_id[p])][0]),int(info[int(st.pose_id[p])][2])) for p in high})
print('lowered pieces: height/branch',hb,'distinct poses',len({int(st.pose_id[p]) for p in high}),flush=True)
b=hb[0][1]
off=st._kpose()[st.pose_id]<0
branches=[E.T.branch(tuple(n)) for n in C.data.N]
clean={};types={}
for c in range(600):
    cl=st.classify(c,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
    if cl.straddle is not None:continue
    clean[c]=set(int(p) for p in cl.inside)
    types[c]={(int(info[int(st.pose_id[p])][0]),int(info[int(st.pose_id[p])][2])) for p in cl.inside if off[p]}
safe=[c for c in clean if not types[c]]
free=[c for c in range(600) if branches[c]==b and c in clean and types[c]<={(2,b)}]
print('clean',len(clean),'SAFE',len(safe),'free class grips',len(free),flush=True)
print('free grips holding all 111',[c for c in free if all(p in clean[c] for p in high)],flush=True)
print('caps 268, 393, 436: clean',[c in clean for c in (268,393,436)],'free',[c in free for c in (268,393,436)],flush=True)
