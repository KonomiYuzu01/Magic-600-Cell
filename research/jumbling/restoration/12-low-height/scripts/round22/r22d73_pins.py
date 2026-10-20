# Theory round 22, line (d), round 5 (R22D-73, R22D-76, R22D-77): at the 3000-twist stop of seed 7302 (state
# 015d839e...), the pieces of the exact poses of 152672 (group D) and 108996 (group C), grouped by their labels at the
# caps of their branch, with each group's class cell, the cuts it straddles and its home chambers; for C's own-branch
# caps 104, 290 and 311, the first straddler that lies inside no branch-1 cap; and, after each of the 24 letters that
# lower 152672 to height 4, for each cap of the new branch that holds 152672 and 152673, the first straddler that lies
# inside no cap of that branch.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d73_pins.txt, which agrees with the values the reviewer reported. Under a CPU minute.
# Build the snapshot first (about 30 CPU minutes): python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
# It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d73_pins.py <dir>/s7302-stop.pkl
import sys,json,resource
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E, buffer as B
from r22d31_snapshots import load
from sim.kernel import Evaluator,pullback_form
from sim.kplus import to_tuple
from exact import matmul
C=E.sim.get_context(); M=E.R.menu('I_a'); s,_=load(sys.argv[1],E)
br=[E.T.branch(tuple(n)) for n in C.data.N]
def clone(s):
 t=E.sim.State(C,filtered=False,menu=M); t.pose_id=s.pose_id.copy(); t._poses=list(s._poses); t._lookup=dict(s._lookup); return t
def lab(t,p,c):return Evaluator(pullback_form(t.pose(p),C.data.N[c])).classify(C.regions.vform(p))[0]
def ins(t,p,poles):return [c for c in poles if lab(t,p,c)=='in']
def pin(t,poles,d):
 ss=E.straddler_set(t,d).tolist()
 for k,p in enumerate(ss):
  if not ins(t,p,poles):return {'d':d,'straddlers':len(ss),'tested':k+1,'fixed':p,'inside_class_caps':[]}
 return {'d':d,'straddlers':len(ss),'tested':len(ss),'fixed':None}
home=C.data.sig_matrix(np.arange(len(s.pose_id))); occ=np.ones(len(s.pose_id),bool); occ[B.hole_set(s)]=False
for rep in [152672,108996]:
 ps=np.flatnonzero(s.pose_id==s.pose_id[rep]).tolist(); P=E.T.pose_tree(s.pose(rep))[2]; poles=[c for c in range(600) if br[c]==P]; groups={}
 for p in ps:
  ls=tuple(lab(s,p,c) for c in poles); groups.setdefault(ls,[]).append(p)
 rows=[]
 for ls,ps in groups.items():
  cell=[c for c,v in zip(poles,ls) if v=='in']; strad=[c for c,v in zip(poles,ls) if v=='straddle']
  X=np.flatnonzero((home[:,poles]==np.isin(poles,cell)).all(axis=1)) if not strad else np.array([],dtype=int)
  rows.append({'pieces':ps,'cell':cell,'straddles':strad,'home':len(X) if not strad else None,'occupied':int(occ[X].sum()) if not strad else None})
 print('CLASS_CELLS',json.dumps({'representative':rep,'branch':P,'rows':rows}))
print('C_PINS',json.dumps([pin(s,[c for c in range(600) if br[c]==1],d) for d in [104,290,311]]))
allp=[]
for c in [64,70]:
 js=[j for j in range(1,60) if E.T.dist(E.T.factors(to_tuple(matmul(E.R.element(M,c,j),s.pose(152672))))[0])==4]
 for j in js:
  y=clone(s); o=y.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
  Q=E.T.pose_tree(y.pose(152672))[2]; poles=[d for d in range(600) if br[d]==Q]; ds=ins(y,152672,poles); assert ds==ins(y,152673,poles)
  rows=[pin(y,poles,d) for d in ds]; allp.extend(rows); print('D_PINS',json.dumps({'a':[c,j],'Q':Q,'rows':rows}))
print('TOTAL',json.dumps({'cases':len(allp),'pinned':sum(r['fixed'] is not None for r in allp),'digest':s.digest()}))
