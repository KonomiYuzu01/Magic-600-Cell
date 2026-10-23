# Theory round 22, line (d), round 6 (R22D-95, R22D-96): at the depth 400 stop of seed 7109 after the seven letters of
# R22D-75's rule there (state b334a2dc..., rebuilt from the census record round3/ia-k400-s7109.json and those letters,
# every twist admissible and certain), the 17 height-4 pieces grouped into units by pose and by the caps that hold them,
# each unit with its branch, those caps and the items of its own-branch caps that lower it to height 2; the straddler
# count of each cap that holds a unit; the straddlers of these caps grouped by the caps they straddle and the caps that
# hold them, with the clean ones. Then, for the lowering caps in order, the letters at clean caps that hold one of the
# cap's straddlers and take at least one of them off its cut, each played until one lowers the straddler count: that
# letter (UNLOCK_STEP) with the straddlers left, the units after it, the counts, N and digest, then undone. Past 600 CPU
# seconds (the setup counted) it undoes any trial letter and reports a budget stop.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d95_unlock_graph_k400.txt, line for line the reviewer's apart from the cpu fields; the
# letter (320, 1) of R22D-96 is the first tested. About 10 CPU minutes on a loaded machine, close to the checkpoint: a
# slower run ends at the budget without that letter.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d95_unlock_graph_k400.py
import sys,json,resource,time
from pathlib import Path
from collections import defaultdict
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200)); T=time.process_time()
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
C=E.sim.get_context(); M=E.R.menu('I_a'); s=E.sim.State(C,filtered=False,menu=M)
rec=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round3/ia-k400-s7109.json').read_text())
for c,j in rec['history']+rec['moves']+[[158,13],[252,13],[406,14],[588,13],[158,14],[300,12],[300,13]]:
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
digest=s.digest(); assert digest=='b334a2dc8b7596251b00b7d3c4c293edda5254b78c045f264099f2220fce2140'
class Budget(Exception):pass
def check():
 if time.process_time()-T>600:raise Budget
cache={}
def hi(p):
 if p.kidx>=0:return 0
 if p.key not in cache:
  l,r=E.T.factors(p.matrix);assert E.T.dist(r)==0;cache[p.key]=E.T.dist(l)
 return cache[p.key]
def heights():return np.array([hi(p) for p in s._poses])[s.pose_id]
def emit(tag,**kw):print(tag,json.dumps(kw),flush=True)
h=heights(); high=np.flatnonzero(h==4);assert len(high)==17 and h.max()==4
emit('START',digest=digest,n2_n4_n6=[int((h==k).sum()) for k in (2,4,6)],off=int(s.off_lattice_count()),N=E.R.Restorer(s,M).misalignment(),cpu=round(time.process_time()-T,2))
R=E.R.Restorer(s,M); inside=[]; clean=[]; tests=0; replays=0
try:
 for c in range(600):
  check();cl=s.classify(c,need_inside=True,stop_at_straddle=False);assert not cl.uncertain
  inside.append(cl.inside);clean.append(cl.straddle is None)
 emit('CAPS',blocked=600-sum(clean),cpu=round(time.process_time()-T,2))
 ins=np.array([np.isin(high,ii) for ii in inside]).T
 groups=defaultdict(list)
 for p,row in zip(high.tolist(),ins):groups[(int(s.pose_id[p]),tuple(np.flatnonzero(row).tolist()))].append(p)
 units=[]; targets=set(); caps=set()
 for (pid,cs),ps in sorted(groups.items(),key=lambda kv:min(kv[1])):
  check();q=s._pose_matrix(pid);l=E.T.factors(q)[0];branch=E.T.branch(l);lower={}
  for c in cs:
   if E.T.branch(tuple(C.data.N[c]))!=branch:continue
   js=[j for j in range(1,60) if E.T.dist(E.T.factors(E.to_tuple(E.matmul(E.R.element(M,c,j),q)))[0])==2]
   if js:lower[c]=js;targets.add(c)
  u=dict(pieces=ps,branch=branch,caps=list(cs),lowering=lower)
  units.append(u);caps.update(cs);emit('UNIT',**u)
 strad={}
 for d in sorted(caps):
  check();strad[d]=E.straddler_set(s,d).tolist()
  emit('CAP',cap=d,clean=clean[d],straddler_count=len(strad[d]))
 blockers=sorted(set(p for ps in strad.values() for p in ps))
 contain={p:[] for p in blockers}
 for c in range(600):
  check()
  for p in np.intersect1d(blockers,inside[c],assume_unique=True).tolist():contain[p].append(c)
 cg=defaultdict(list);ss={d:set(ps) for d,ps in strad.items()}
 for p,cs in contain.items():cg[tuple(cs),tuple(d for d in sorted(ss) if p in ss[d])].append(p)
 for (cs,ds),ps in sorted(cg.items(),key=lambda kv:min(kv[1])):emit('BLOCKER_GROUP',pieces=ps,straddles=list(ds),caps=list(cs),clean_caps=[c for c in cs if clean[c]])
 found=False;lat0=s._kpose()[s.pose_id]>=0
 for d in sorted(targets):
  S=set(strad[d])
  for c in range(600):
   if not clean[c]:continue
   moved=sorted(S.intersection(inside[c].tolist()))
   if not moved:continue
   for j in range(1,60):
    check();tests+=1;g=E.R.element(M,c,j);ev={}
    def leaves(p):
     pid=int(s.pose_id[p])
     if pid not in ev:ev[pid]=E.Evaluator(E.pullback_form(E.matmul(g,s._pose_matrix(pid)),C.data.N[d]))
     r=ev[pid].classify(C.regions.vform(p))[0];assert r!='uncertain';return r!='straddle'
    if not any(leaves(p) for p in moved):continue
    R.apply(c,j);replays+=1;after=E.straddler_set(s,d).tolist()
    if len(after)<len(S):
     hh=heights();found=True;post=[]
     cl=s.classify(d,need_inside=True,stop_at_straddle=False);assert not cl.uncertain
     for u in units:
      if d not in u['lowering']:continue
      ps=u['pieces'];pids=np.unique(s.pose_id[ps]);assert len(pids)==1
      q=s._pose_matrix(int(pids[0]));whole=bool(np.isin(ps,cl.inside).all())
      its=[v for v in range(1,60) if E.T.dist(E.T.factors(E.to_tuple(E.matmul(E.R.element(M,d,v),q)))[0])==2] if whole and int(hh[ps[0]])==4 else []
      post.append(dict(pieces=ps,height=int(hh[ps[0]]),whole_in_target=whole,lowering_items=its))
     emit('UNLOCK_STEP',target_units_after=post,target_cap=d,word=[[c,j]],before_count=len(S),after_count=len(after),remaining_straddlers=after,n2_n4_n6=[int((hh==k).sum()) for k in (2,4,6)],off=int(s.off_lattice_count()),N=E.R.Restorer(s,M).misalignment(),initial_lattice_now_off=int((lat0 & (s._kpose()[s.pose_id]<0)).sum()),digest=s.digest(),tests=tests,replays=replays,cpu=round(time.process_time()-T,2))
    R.undo();assert s.digest()==digest
    if found:break
   if found:break
  if found:break
 emit('END',result='single-letter-found' if found else 'all-candidate-single-letters-exhausted',tests=tests,replays=replays,digest=s.digest(),cpu=round(time.process_time()-T,2))
except Budget:
 while R.moves:R.undo()
 assert s.digest()==digest
 emit('END',result='cpu-budget',tests=tests,replays=replays,digest=s.digest(),cpu=round(time.process_time()-T,2))
