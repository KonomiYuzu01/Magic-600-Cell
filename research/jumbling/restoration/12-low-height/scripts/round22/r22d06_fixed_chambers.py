# Theory round 22, R22D-06: the senior reviewer's exact computation, kept as written apart from this header;
# the integrator re-ran it and got the output in runs/round22/r22d06_fixed_chambers.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22d06_fixed_chambers.py
import sys,json,resource,time
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(870,870))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import numpy as np
import eb6 as E
import buffer as B
from screen import load
R,sim=E.R,E.sim
r=json.loads(Path('research/jumbling/restoration/12-low-height/runs/eb6-k100.json').read_text())
st,M,_=load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k100-s7102.json')
assert st.digest()==r['digest'] and not st.filtered
st.journal=[]; C=st.ctx; allp=np.arange(len(st.pose_id))
U=set(p for u in r['units'][:5] for p in u['piece_ids'])
print('digest',st.digest(),flush=True)
for j,g in [(57,110),(59,21)]:
 o=st.apply(sim.Twist(C,61,R.element(M,61,j),'menu',{'menu':M.name}));assert o.applied and not o.uncertain
 off=st._kpose()[st.pose_id]<0
 cl=st.classify(g,need_inside=True,stop_at_straddle=True)
 assert cl.straddle is None and not cl.uncertain and set(cl.inside[off[cl.inside]].tolist())==U
 safe=[]
 for c in range(600):
  cl=st.classify(c,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
  if cl.straddle is None and not off[cl.inside].any():safe.append(c)
 target=allp[C.data.in_sig(allp,np.full(len(allp),g))]
 fixed=target[~C.data.sig_matrix(target)[:,safe].any(axis=1)]
 holes=B.hole_set(st)
 bad=np.setdiff1d(fixed,holes,assume_unique=True)
 print(json.dumps({'item':j,'cap':g,'off_in_cap':len(U),'SAFE_caps':len(safe),'target':len(target),'fixed':len(fixed),'fixed_occupied':len(bad),'witness_chambers':bad[:10].tolist(),'cpu':round(time.process_time(),2)}),flush=True)
 st.undo();assert st.digest()==r['digest']
print('DONE cpu',round(time.process_time(),2),flush=True)
