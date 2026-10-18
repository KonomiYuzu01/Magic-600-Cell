# Theory round 22, line (b), round 4 (R22B-22): at the depth 60 stop, the left heights of the off pieces and
# the order of the group induced on every SAFE orbit of 2 to 120 chambers (exhaustive up to 100,000 elements).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22b22_small_orbits.txt (about 90 CPU seconds).
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22b22_small_orbits.py
import sys,json,resource,signal,time
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1150,1200))
def stop(*a):
 print('CUT_SHORT_CPU_1150_SECONDS',flush=True); raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen,buffer as B
st,M,_=screen.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json')
print('STATE',st.digest(),int(st.off_lattice_count()),flush=True)
height=Counter()
for pid in np.unique(st.pose_id[B.off_pieces(st)]).tolist():
 h=screen.T.dist(screen.T.factors(st._pose_matrix(pid))[0]); height[int(h)]+=int(np.count_nonzero(st.pose_id==pid))
print('HEIGHTS',dict(sorted(height.items())),flush=True)
G=B.Alphabet(SimpleNamespace(st=st),M); orbit,sizes=G.orbits()
print('ORBIT_SIZES',sorted(Counter(map(int,sizes)).items()),flush=True)
for o in np.flatnonzero((sizes>1)&(sizes<=120)).tolist():
 D=np.flatnonzero(orbit==o); ident=tuple(range(len(D)))
 gens=sorted({tuple(np.searchsorted(D,G.images([n],D)).tolist()) for n in range(len(G.letters))}-{ident})
 group={ident}; todo=[ident]; exhausted=True
 for p in todo:
  for q in gens:
   z=tuple(q[x] for x in p)
   if z not in group:
    group.add(z); todo.append(z)
  if len(group)>100000:
   exhausted=False; break
 print('SMALL_GROUP',int(D[0]),len(D),'generators',len(gens),'order',len(group) if exhausted else None,'over100000',not exhausted,flush=True)
 if exhausted and len(D)>=8:
  print('COUNTEREXAMPLE_ORBIT_MEMBERS',D.tolist(),'GENERATORS',gens,flush=True)
print('CPU',round(time.process_time(),1),flush=True)
