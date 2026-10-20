# Theory round 22, line (d), round 5 (R22D-70, R22D-72): at the 3000-twist stop of seed 7302 (state 015d839e...), the
# items at cap 274 whose exact products take 22706 (group A) and 28969 (group B) to height 4; then, after each of the
# 24 letters at caps 64 and 70 that lower 152672 (group D) to height 4, the lattice pieces it takes off, the caps that
# hold all of them (with the pieces that narrow the intersection) and those of these caps that miss 152672 or 152673.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d72_one_repair.txt, which agrees with the values the reviewer reported. Under a CPU minute.
# Build the snapshot first (about 30 CPU minutes): python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
# It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d72_one_repair.py <dir>/s7302-stop.pkl
import sys,json,resource
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.kernel import Evaluator,pullback_form
from sim.kplus import to_tuple
from exact import matmul
C=E.sim.get_context(); M=E.R.menu('I_a'); s,_=load(sys.argv[1],E)
def clone(s):
 t=E.sim.State(C,filtered=False,menu=M); t.pose_id=s.pose_id.copy(); t._poses=list(s._poses); t._lookup=dict(s._lookup); return t
def ht(q):return E.T.dist(E.T.factors(q)[0])
def label(t,p,c):return Evaluator(pullback_form(t.pose(p),C.data.N[c])).classify(C.regions.vform(p))[0]
def inside(t,p,poles):return [c for c in poles if label(t,p,c)=='in']
print('CAP274_PRODUCTS',json.dumps({p:[j for j in range(1,60) if ht(to_tuple(matmul(E.R.element(M,274,j),s.pose(p))))==4] for p in [22706,28969]}))
lat0=s._kpose()[s.pose_id]>=0
for c in [64,70]:
 js=[j for j in range(1,60) if ht(to_tuple(matmul(E.R.element(M,c,j),s.pose(152672))))==4]
 for j in js:
  y=clone(s); o=y.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
  collateral=np.flatnonzero(lat0&(y._kpose()[y.pose_id]<0)).tolist(); choices=set(range(600)); cert=[]
  for p in collateral:
   cp=inside(y,p,sorted(choices)); new=choices.intersection(cp)
   if new!=choices:cert.append([p,cp])
   choices=new
   if not choices:break
  misses=[b for b in sorted(choices) if label(y,152672,b)=='out' or label(y,152673,b)=='out']
  print('ONE_REPAIR',json.dumps({'a':[c,j],'collateral':len(collateral),'common_caps':sorted(choices),'target_miss_caps':misses,'intersection_witness':cert}))
