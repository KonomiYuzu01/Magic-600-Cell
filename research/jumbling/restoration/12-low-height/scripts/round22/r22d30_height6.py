# Theory round 22, line (d), round 3 (R22D-30): replays the depth 1000 record of seed 7103 step by step and prints
# the left-height counts of the off pieces whenever the number of pieces at height 6 or more changes.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator ran it and got
# the output in runs/round22/r22d30_height6.txt. About 9 CPU minutes.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d30_height6.py
import sys,json,resource
from pathlib import Path
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import numpy as np
r=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round2/ia-k1000-s7103.json').read_text())
C=E.sim.get_context();M=E.R.menu('I_a');st=E.sim.State(C,filtered=False,menu=M)
def apply(c,j):
 o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
 assert o.applied and not o.uncertain
cache={}
def heights():
 values=[]
 for pid,p in enumerate(st._poses):
  if p.kidx>=0:values.append(0);continue
  if p.key not in cache:cache[p.key]=int(E.T.dist(E.T.factors(st._pose_matrix(pid))[0]))
  values.append(cache[p.key])
 return np.asarray(values)[st.pose_id]
for c,j in r['history']:apply(c,j)
h=heights();original6=np.flatnonzero(h==6)
print('START',st.digest(),dict(Counter(h[h>0].tolist())))
cursor=0;old=int((h>=6).sum())
for i,step in enumerate(r['steps'],1):
 for c,j in r['moves'][cursor:cursor+step['twists']]:apply(c,j)
 cursor+=step['twists'];assert st.off_lattice_count()==step['off_after']
 h=heights();n=int((h>=6).sum())
 if n!=old:
  print('CHANGE',i,step['rule'],cursor,dict(Counter(h[h>0].tolist())),st.digest(),flush=True)
  old=n
assert cursor==len(r['moves']) and st.off_lattice_count()==r['off_now']
print('END',st.digest(),dict(Counter(h[h>0].tolist())),'original6_final',dict(Counter(h[original6].tolist())))
