# Theory round 22, line (d), round 3 (R22D-24): at the depth 400 stop of seed 7109, the straddlers of cap 252 (which
# holds the ten high pieces of the three units in no clean cap), their heights, and the clean caps that could land or
# move them in one letter.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d24_cap252.txt. About 3 CPU minutes.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d24_cap252.py
import sys,json,resource
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import numpy as np
r=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round3/ia-k400-s7109.json').read_text())
C=E.sim.get_context();M=E.R.menu('I_a');st=E.sim.State(C,filtered=False,menu=M)
for c,j in r['history']+r['moves']:
 o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
 assert o.applied and not o.uncertain
info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
high=np.array([p for p in range(len(st.pose_id)) if info[int(st.pose_id[p])][0]>=4],np.int64)
off=st._kpose()[st.pose_id]<0
S=E.straddler_set(st,252)
cl=st.classify(252,need_inside=True,stop_at_straddle=False);assert not cl.uncertain
print('digest',st.digest())
print('straddlers',S.tolist(),'heights',[info[int(st.pose_id[p])][0] for p in S])
print('high_in_252',np.intersect1d(high,cl.inside).tolist())
clean={}
for c in range(600):
 cl=st.classify(c,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
 if cl.straddle is None:clean[c]=cl.inside
covers=[c for c,ins in clean.items() if np.isin(S,ins).all()]
setups=[c for c in covers if not np.isin(high,clean[c]).any()]
pure=[c for c,ins in clean.items() if len(ins) and off[ins].all() and np.isin(S,ins).any() and not np.isin(high,ins).any()]
print({'clean':len(clean),'contain_all_straddlers':covers,'also_keep_all_high':setups,'all_off_caps_touching_straddlers':pure})
