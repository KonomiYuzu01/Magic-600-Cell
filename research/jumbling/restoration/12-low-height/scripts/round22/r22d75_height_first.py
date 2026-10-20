# Theory round 22, line (d), round 5 (R22D-74, R22D-75): the height-first rule of R22D-75 from the 3000-twist stop of
# seed 7302 (state 015d839e...): single letters that lower n6, then those that keep n6 and lower n4, then those that
# keep both and lower n2, each in (grip, item) order, admissible and with no piece above height 6, the exact prediction
# of R22D-75 checked against the simulator after every accepted letter; each step with its letter, digests, counts at
# heights 6, 4 and 2, off count and N, then the stop with its pruning counts. Past 1,000 CPU seconds it reports a
# budget stop.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d75_height_first.txt, whose seven steps and stop are the ones the reviewer reported. About 5
# CPU minutes.
# Build the snapshot first (about 30 CPU minutes): python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
# It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d75_height_first.py <dir>/s7302-stop.pkl
import sys,json,resource,time
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200)); T=time.process_time()
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
C=E.sim.get_context(); M=E.R.menu('I_a'); s,_=load(sys.argv[1],E)
br=[E.T.branch(tuple(n)) for n in C.data.N]; cache={}; gcache={}
class Budget(Exception):pass
def checktime():
 if time.process_time()-T>1000:raise Budget

def info(p):
 if p.kidx>=0:return 0,-1,None
 if p.key not in cache:
  l,r=E.T.factors(p.matrix); assert E.T.dist(r)==0
  cache[p.key]=(E.T.dist(l),E.T.branch(l),l)
 return cache[p.key]
def heights():return np.array([info(p)[0] for p in s._poses])[s.pose_id]
def stats():
 h=heights(); assert max(h)<=6
 return [int((h==k).sum()) for k in [6,4,2]]
def report(tag,**kw):
 v=stats(); print(tag,json.dumps(dict(kw,digest=s.digest(),n6_n4_n2=v,off=sum(v),N=E.R.Restorer(s,M).misalignment(),cpu=round(time.process_time()-T,1))),flush=True)
def nextletter():
 inf=[info(p) for p in s._poses]; phi=stats(); memo={}; ct=Counter()
 def rows(c):
  if c in memo:return memo[c]
  cl=s.classify(c,need_inside=True,stop_at_straddle=True); assert not cl.uncertain; ct['classified_caps']+=1
  if cl.straddle is not None:memo[c]=[]; ct['blocked_caps']+=1; return []
  ct['clean_caps']+=1
  ids,ns=np.unique(s.pose_id[cl.inside],return_counts=True); P=br[c]
  own={2:0,4:0,6:0}; foreign={2:0,4:0,6:0}; lat=0; ops=[]
  for i,n in zip(ids.tolist(),ns.tolist()):
   h,b,l=inf[i]
   if h==0:lat+=n
   elif b==P:own[h]+=n; ops.append((h,l,n))
   else:foreign[h]+=n
  if foreign[6] or foreign[4]>own[6] or (foreign[4]==own[6] and (foreign[2]>own[4] or (foreign[2]==own[4] and lat>=own[2]))):
   memo[c]=[]; ct['cap_bound_prunes']+=1; return []
  ans=[]
  for j in range(1,60):
   checktime()
   if (c,j) not in gcache:
    g=E.R.element(M,c,j); gcache[c,j]=(g,None if C.kplus.index_of_matrix(g)>=0 else E.T.factors(g)[0])
   g,l=gcache[c,j]
   if l is None:ct['retained_prunes']+=1; continue
   r={2:0,4:0,6:0}
   for h,q,n in ops:
    new=E.T.dist(E.T.qmul(l,q)); assert new in (h,h-2)
    if new==h-2:r[h]+=n
   delta=[foreign[4]-r[6],foreign[2]+r[6]-foreign[4]-r[4],lat+r[4]-foreign[2]-r[2]]
   out=[v+d for v,d in zip(phi,delta)]; ct['item_height_tests']+=1
   if out<phi:ans.append((j,out,next(k for k in range(3) if out[k]!=phi[k])))
  memo[c]=ans; return ans
 for tier in range(3):
  for c in range(600):
   checktime()
   for j,out,k in rows(c):
    if k==tier:return (c,j),out,dict(ct)
 return None,None,dict(ct)
report('START'); steps=0
try:
 while stats()[0]>0:
  a,pred,ct=nextletter()
  if a is None:report('STUCK',steps=steps,exhausted='all single letters',checks=ct); break
  before=s.digest(); c,j=a; o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
  assert stats()==pred
  steps+=1; report('STEP',step=steps,word=[list(a)],length=1,input_digest=before,checks=ct)
 else:report('HANDOFF_R22D25',steps=steps)
except Budget:report('BUDGET',steps=steps,limit_cpu=1000)
