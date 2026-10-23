# Theory round 22, line (d), round 4 (R22D-40 and R22D-41): the blockers of cap 179 at the seed 7102 stop after
# R22D-16 (state dceb9359...) and of cap 252 at the depth 400 stop of seed 7109 (state 7f8203dd...): their rigid units,
# the source cell of each blocker, the free grips of its class and whether one of them holds the cell, the SAFE grips,
# and the home chambers of the cell that are occupied and outside every SAFE cap.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d40_blockers.txt (the same apart from the CPU times). About 8 CPU minutes.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d40_blockers.py
import sys,json,resource,time,runpy
from pathlib import Path
from collections import defaultdict
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E, buffer as B
import numpy as np
T=time.process_time()
C=E.sim.get_context(); M=E.R.menu('I_a')
branches=np.array([E.T.branch(tuple(n)) for n in C.data.N])
home=C.data.sig_matrix(np.arange(len(C.data.sigbits)))
for seed,cap,path in [(7102,179,'round2/ia-k100-s7102.json'),(7109,252,'round3/ia-k400-s7109.json')]:
 if seed==7102:
  z=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/r22d16_step.py');st=z['st']
 else:
  r=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/'+path).read_text());st=E.sim.State(C,filtered=False,menu=M)
  for c,j in r['history']+r['moves']:
   o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
 off=B.off_pieces(st); offmask=np.zeros(len(st.pose_id),bool);offmask[off]=True
 info={pid:E.T.pose_tree(st._pose_matrix(pid)) for pid in np.unique(st.pose_id).tolist()}
 S=E.straddler_set(st,cap).tolist(); holes=np.zeros(len(home),bool);holes[B.hole_set(st)]=True
 labels=np.zeros((len(off),600),np.uint8);clean={};types={}
 for c in range(600):
  cl=st.classify(c,need_inside=True,stop_at_straddle=False);assert not cl.uncertain
  labels[np.searchsorted(off,np.intersect1d(off,cl.inside)),c]=1
  if cl.straddle is None:
   clean[c]=cl.inside;types[c]={(int(info[int(st.pose_id[p])][0]),int(info[int(st.pose_id[p])][2])) for p in cl.inside if offmask[p]}
  else:labels[np.searchsorted(off,E.straddler_set(st,c)),c]=2
 safe=[c for c in clean if not types[c]];fixed=(~holes)&(~home[:,safe].any(axis=1))
 D=E.R.Restorer(st,M).domains();dom=np.array(D.dom)
 inside=np.packbits(labels==1,axis=1);outside=np.packbits(labels==0,axis=1)
 pending=set(S);units=[]
 while pending:
  start=min(pending);seen=np.zeros(len(off),bool);queue=[int(np.searchsorted(off,start))];seen[queue]=True
  while queue:
   row=queue.pop();separated=((inside[row]&outside)|(outside[row]&inside)).any(axis=1)
   hits=np.flatnonzero(~seen&~separated);seen[hits]=True;queue.extend(hits.tolist())
  block=off[seen];unit=block[dom[block]==dom[start]];hit=sorted(set(unit.tolist())&pending);pending.difference_update(hit)
  rows={}
  for p in hit:
   pid=int(st.pose_id[p]);h,_,b,_=info[pid];assert h==2
   poles=np.flatnonzero(branches==b);cell=tuple(int(c) for c in poles if labels[np.searchsorted(off,p),c]==1)
   assert not (labels[np.searchsorted(off,p),poles]==2).any()
   if cell in rows:rows[cell]['blockers'].append(p);continue
   X=np.flatnonzero((home[:,poles]==np.isin(poles,cell)[None,:]).all(axis=1));fx=X[fixed[X]]
   free=[int(c) for c in poles if c in clean and types[c]<={(2,int(b))}]
   rows[cell]={'blockers':[p],'branch':int(b),'cell':cell,'free_containing':[c for c in cell if c in free],'free_count':len(free),'home':len(X),'occupied':int((~holes[X]).sum()),'fixed_occupied':len(fx),'fixed_witness':int(fx[0]) if len(fx) else None}
  units.append({'unit':int(min(unit)),'size':len(unit),'block_first':int(min(block)),'block_size':len(block),'frames':len(set(dom[block].tolist())),'blockers':hit,'source':list(rows.values())})
 print('BLOCKER_AUDIT',json.dumps({'seed':seed,'cap':cap,'digest':st.digest(),'off':len(off),'clean':len(clean),'safe':len(safe),'units':units,'cpu':round(time.process_time()-T,1)}),flush=True)
