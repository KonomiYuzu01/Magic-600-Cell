# Theory round 22, line (d), round 5 (R22D-76, R22D-77): at the 3000-twist stop of seed 7302 (state 015d839e...), the
# pieces of 108996's exact pose split by their label at cap 183 (the two units of group C), the clean branch-1 caps,
# and, for each straddler of caps 104, 290 and 311, its branch and the grips of the fixed free alphabet of that branch
# (as r22d70_cap274.py prints it) that hold it.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d76_class1.txt, which agrees with the values the reviewer reported. Under a CPU minute.
# Build the snapshot first (about 30 CPU minutes): python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
# It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d76_class1.py <dir>/s7302-stop.pkl
import sys,json,resource
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.kernel import Evaluator,pullback_form
C=E.sim.get_context(); M=E.R.menu('I_a'); s,_=load(sys.argv[1],E)
def lab(p,c):return Evaluator(pullback_form(s.pose(p),C.data.N[c])).classify(C.regions.vform(p))[0]
br=[E.T.branch(tuple(n)) for n in C.data.N]
ps=np.flatnonzero(s.pose_id==s.pose_id[108996]).tolist()
print('C_UNITS',json.dumps({str(v):[p for p in ps if (lab(p,183)=='in')==v] for v in [True,False]}))
clean1=[]
for c in range(600):
 if br[c]!=1:continue
 cl=s.classify(c,need_inside=False,stop_at_straddle=True); assert not cl.uncertain
 if cl.straddle is None:clean1.append(c)
print('CLEAN_CLASS1',clean1)
free={0:[2,20,23,62,66,93,112,167,262,394],1:[],2:[263,358,364,370,407,422,523,541,552,553,590,591,593],3:[136,277,301,363],4:[39]}
for d in [104,290,311]:
 rows=[]
 for p in E.straddler_set(s,d).tolist():
  h,hr,b,brp=E.T.pose_tree(s.pose(p)); assert h==2 and hr==0
  hits=[c for c in free[b] if lab(p,c)=='in']; rows.append([p,b,hits])
 print('C_BLOCKER_FREE',json.dumps({'cap':d,'total':len(rows),'no_free':sum(not r[2] for r in rows),'first_no_free':next((r for r in rows if not r[2]),None),'movable':[r for r in rows if r[2]]}))
print('DIGEST',s.digest())
