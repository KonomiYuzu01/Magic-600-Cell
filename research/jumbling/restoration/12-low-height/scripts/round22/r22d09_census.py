# Theory round 22, line (d), round 2 (R22D-09 to R22D-13, and the masks of R22D-17): the high units of the five census
# stops with height-4 pieces (depth 100 seeds 7102 and 7108, depth 200 seed 7105, depth 400 seed 7109, depth 1000
# seed 7103), each rebuilt from its record with every twist checked: per unit its clean containing caps, its own-branch
# clean caps, and the own-branch caps that would be clean if every height-2 piece were landed (geometry only).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d09_census.txt. About 15 CPU minutes.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d09_census.py
import sys,json,resource
from pathlib import Path
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import numpy as np
from sim.kernel import Evaluator,pullback_form
paths=['round2/ia-k100-s7102.json','round3/ia-k100-s7108.json','round3/ia-k200-s7105.json','round3/ia-k400-s7109.json','round2/ia-k1000-s7103.json']
C=E.sim.get_context(); M=E.R.menu('I_a')
gb=[E.T.branch(tuple(n)) for n in C.data.N]
for path in paths:
 rec=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/'+path).read_text())
 st=E.sim.State(C,filtered=False,menu=M)
 for c,j in rec['history']+rec['moves']:
  o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
  assert o.applied and not o.uncertain
 assert st.off_lattice_count()==rec['off_now']
 digest=st.digest(); lat=st._kpose()[st.pose_id]>=0; off=np.flatnonzero(~lat)
 info={pid:E.T.pose_tree(st._pose_matrix(pid)) for pid in np.unique(st.pose_id[off]).tolist()}
 high=np.array([p for p in off if info[int(st.pose_id[p])][0]>=4],np.int64)
 lab=np.zeros((len(high),600),np.uint8)
 for pid in np.unique(st.pose_id[high]).tolist():
  rs=np.flatnonzero(st.pose_id[high]==pid); q=st._pose_matrix(pid)
  for c in range(600):
   ev=Evaluator(pullback_form(q,C.data.N[c]))
   for rr in rs:
    v=ev.classify(C.regions.vform(int(high[rr])))[0]
    assert v in ('in','out','straddle')
    lab[rr,c]={'in':1,'out':0,'straddle':2}[v]
 ins=lab==1; groups={}
 for rr,p in enumerate(high.tolist()):
  groups.setdefault((int(st.pose_id[p]),ins[rr].tobytes()),[]).append(rr)
 clean={}; counts={}
 for c in np.flatnonzero(ins.any(axis=0)).tolist():
  cl=st.classify(c,need_inside=True,stop_at_straddle=False)
  assert not cl.uncertain
  clean[c]=cl.straddle is None
  counts[c]=[len(cl.inside),int(lat[cl.inside].sum()),int((~lat[cl.inside]).sum()),int(np.isin(cl.inside,high).sum())]
 rows=[]
 for (pid,_),rs in sorted(groups.items(),key=lambda kv:int(high[kv[1][0]])):
  h,hr,b,br=info[pid]; assert h==4 and hr==0
  caps=np.flatnonzero(ins[rs[0]]).tolist(); own=[c for c in caps if gb[c]==b]
  rows.append([int(high[rs[0]]),len(rs),int(b),[c for c in caps if clean[c]],[c for c in own if clean[c]],[c for c in own if not (lab[:,c]==2).any()]])
 used=sorted({c for row in rows for c in row[4]})
 print(json.dumps({'record':path,'digest':digest,'heights':dict(Counter(int(info[int(st.pose_id[p])][0]) for p in off)),'units':rows,'cap_counts_total_lattice_off_high':{c:counts[c] for c in used},'no_clean_whole':sum(not row[3] for row in rows),'no_own_clean':sum(not row[4] for row in rows),'no_own_clean_after_height2':sum(not row[5] for row in rows)}),flush=True)
 assert st.digest()==digest
