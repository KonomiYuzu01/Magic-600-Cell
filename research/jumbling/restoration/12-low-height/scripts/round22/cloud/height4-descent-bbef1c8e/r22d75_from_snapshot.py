# Theory round 22, line (d), round 6, cloud line height4-descent-bbef1c8e, step 2: R22D-75's height-first rule from a
# snapshot. A copy of r22d75_census.py (sha256 in RESULT.md) that loads a snapshot written by r22d31_snapshots.py
# (`from r22d31_snapshots import load`, digest and off count checked), plays a prefix of letters (each admissible and
# certain) and checks the digest after it against --expect, instead of replaying a census record. R22D-75's three
# families, its exact count prediction (checked against the simulator after every accepted letter), its cap bound and
# its end conditions are unchanged; only the CPU budget is a parameter (--cpu, default 3,000 s after the prefix).
# Each step with its letter, digests, counts at heights 6, 4 and 2, off count, N and pruning counts, then the end:
# HEIGHT2 (no piece above height 2), STUCK (every single letter exhausted) or BUDGET.
# Changes against r22d75_census.py: argument parsing (--snapshot, --prefix, --expect, --cpu), the snapshot load and
# prefix in place of the record replay, the budget value, the round22 directory on sys.path. Nothing else.
# Research only, headless Linux; exact geometry. It writes no file; theory/seams.py keeps its cache in
# work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Run from the repository root:
#   OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file> python3 -I -B -u \
#     research/jumbling/restoration/12-low-height/scripts/round22/cloud/height4-descent-bbef1c8e/r22d75_from_snapshot.py \
#     --snapshot <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[124,2],[274,13],[46,1],[46,3],[104,14]]' \
#     --expect bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299
import sys,json,resource,time,argparse
from pathlib import Path
from collections import Counter
import numpy as np
ap=argparse.ArgumentParser(); ap.add_argument('--snapshot',required=True); ap.add_argument('--prefix',default='[]'); ap.add_argument('--expect'); ap.add_argument('--cpu',type=float,default=3000.0)
A=ap.parse_args()
resource.setrlimit(resource.RLIMIT_CPU,(int(A.cpu)+1190,int(A.cpu)+1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
C=E.sim.get_context(); M=E.R.menu('I_a')
s,meta=load(A.snapshot,E)
pre=json.loads(A.prefix)
for c,j in pre:
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
 assert o.applied and not o.uncertain
if A.expect:assert s.digest()==A.expect,s.digest()
print('SNAPSHOT',json.dumps({'snapshot_digest':meta['digest'],'snapshot_off':meta['off'],'prefix':len(pre),'digest':s.digest(),'load_cpu':round(time.process_time(),1)}),flush=True)
T=time.process_time()
br=[E.T.branch(tuple(n)) for n in C.data.N]; cache={}; gcache={}
class Budget(Exception):pass
def checktime():
 if time.process_time()-T>A.cpu:raise Budget

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
 while sum(stats()[:2])>0:
  a,pred,ct=nextletter()
  if a is None:report('STUCK',steps=steps,exhausted='all single letters',checks=ct); break
  before=s.digest(); c,j=a; o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
  assert stats()==pred
  steps+=1; report('STEP',step=steps,word=[list(a)],length=1,input_digest=before,checks=ct)
 else:report('HEIGHT2',steps=steps)
except Budget:report('BUDGET',steps=steps,limit_cpu=A.cpu)
