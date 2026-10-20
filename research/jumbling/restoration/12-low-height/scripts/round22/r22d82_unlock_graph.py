# Theory round 22, line (d), round 6: the senior reviewer's unlock-graph computation for R22D-82 to R22D-84, kept as
# written apart from this header (the integrator's re-run printed the same output; it is in
# runs/round22/r22d82_unlock_graph.txt). From a snapshot of the 3000-twist stop of seed 7302 (015d839e...) and the state
# after R22D-75's seven letters (1ff0061f...): the complete in/out/straddle table of every off piece at every cap
# (all_labels), the unlock closure layers, the straddlers of caps 274, 104, 290 and 311 not held whole by any cap of the
# closure, their unit rows, and the holder status [cap, branch, clean, height-6 pieces inside, foreign-branch height-6
# pieces inside] (np.int64 values in the printout).
# Research only, headless Linux; exact geometry. It writes no file. About five CPU minutes after the snapshot.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d82_unlock_graph.py <dir>/s7302-stop.pkl
import sys,json,resource
from collections import defaultdict
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
from sim.kernel import Evaluator,pullback_form
C=E.sim.get_context();M=E.R.menu('I_a');s0,_=load(sys.argv[1],E)
def clone(st):
 t=E.sim.State(C,filtered=False,menu=M);t.pose_id=st.pose_id.copy();t._poses=list(st._poses);t._lookup=dict(st._lookup);return t
s=clone(s0)
for c,j in [(64,15),(185,14),(357,12),(471,12),(263,14),(407,13),(523,12)]:
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
assert s.digest()=='1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81'
hc={}
def inf(p):
 if p.kidx>=0:return 0,-1,None
 if p.key not in hc:
  l,r=E.T.factors(p.matrix);assert E.T.dist(r)==0;hc[p.key]=(E.T.dist(l),E.T.branch(l),l)
 return hc[p.key]
def heights(st):return np.array([inf(p)[0] for p in st._poses])[st.pose_id]
def all_labels(st,pieces=None):
 n=len(st.pose_id);pick=np.arange(n) if pieces is None else np.asarray(pieces)
 out=np.zeros((len(pick),600),np.uint8);pos=np.full(n,-1,np.int32);pos[pick]=np.arange(len(pick))
 k=st._kpose()[st.pose_id];lp=pick[k[pick]>=0];op=pick[k[pick]<0]
 groups=[(pid,st._anchors(pid,mem)) for pid,mem in st._groups(op)]
 for c in range(600):
  out[pos[lp],c]=C.data.in_sig(lp,C.kplus.perms[C.kplus.inv[k[lp]],c])
  for pid,anchors in groups:
   ev=Evaluator(pullback_form(st._pose_matrix(pid),C.data.N[c]))
   for a,sub in anchors:
    lab=ev.classify(C.regions.cap_vform(a))[0];assert lab!='uncertain'
    if lab!='straddle':
     if lab=='in':out[pos[sub],c]=1
     continue
    for p in sub.tolist():
     lab=ev.classify(C.regions.vform(p))[0];assert lab!='uncertain';out[pos[p],c]={'in':1,'out':0,'straddle':2}[lab]
 return out
def unit_rows(st,lab):
 off=np.flatnonzero(heights(st)>0);dom=np.asarray(E.R.Restorer(st,M).domains().dom)
 ip=np.packbits(lab[off]==1,axis=1);op=np.packbits(lab[off]==0,axis=1)
 pats={};members=[];masks=[];pindex={}
 for n,p in enumerate(off.tolist()):
  key=(int.from_bytes(ip[n].tobytes(),'little'),int.from_bytes(op[n].tobytes(),'little'))
  if key not in pats:pats[key]=len(members);members.append([]);masks.append(key)
  i=pats[key];members[i].append(p);pindex[p]=i
 targetp=sorted({p for c in [274,104,290,311] for p in np.flatnonzero(lab[:,c]==2).tolist()})
 unresolved=set(range(len(members)));belongs={};units={}
 for p in targetp:
  if p in belongs:continue
  seed=pindex[p];found={seed};todo=[seed];unresolved.remove(seed)
  while todo:
   r=todo.pop();a,b=masks[r];hits=[t for t in unresolved if not(a&masks[t][1] or b&masks[t][0])]
   unresolved.difference_update(hits);found.update(hits);todo.extend(hits)
  block=np.array(sorted(q for t in found for q in members[t]))
  for d in np.unique(dom[block]).tolist():
   u=block[dom[block]==d];q=int(u[0]);units[q]=u.tolist()
   for x in u.tolist():belongs[x]=q
 for c in [274,104,290,311]:
  grouped=defaultdict(list)
  for p in np.flatnonzero(lab[:,c]==2).tolist():grouped[(belongs[p],tuple(np.flatnonzero(lab[p]==1).tolist()))].append(p)
  print('UNIT_ROWS',c,json.dumps([dict(unit=u,size=len(units[u]),branch=inf(st._poses[int(st.pose_id[ps[0]])])[1],pieces=ps,holders=list(holds)) for (u,holds),ps in sorted(grouped.items())]))
labels=all_labels(s)
changed=np.flatnonzero(E.stable_pose(s0)!=E.stable_pose(s));print('CHANGED_PIECES',len(changed))
labels0=labels.copy();labels0[changed]=all_labels(s0,changed)
br=np.array([E.T.branch(tuple(n)) for n in C.data.N]);six=np.flatnonzero(heights(s)==6)
for st,lab in [(s,labels),(s0,labels0)]:
 ss=[np.flatnonzero(lab[:,c]==2) for c in range(600)];L={c for c in range(600) if not len(ss[c])};layers=[sorted(L)]
 while True:
  can=(lab[:,sorted(L)]==1).any(axis=1);added=[c for c in range(600) if c not in L and can[ss[c]].all()]
  if not added:break
  layers.append(added);L.update(added)
 print('GRAPH',st.digest(),list(map(len,layers)),sorted(L))
 for c in [274,104,290,311]:
  assert np.array_equal(ss[c],E.straddler_set(st,c))
  print('TARGET',c,len(ss[c]),next((i for i,x in enumerate(layers) if c in x),None),ss[c][~can[ss[c]]].tolist())
 unit_rows(st,lab)
 holds=sorted({int(c) for d in [274,104,290,311] for p in ss[d] for c in np.flatnonzero(lab[p]==1)})
 print('HOLDER_STATUS',[[c,int(br[c]),not len(ss[c]),int((lab[six,c]==1).sum()),sum(lab[p,c]==1 and inf(st._poses[int(st.pose_id[p])])[1]!=br[c] for p in six)] for c in holds])
