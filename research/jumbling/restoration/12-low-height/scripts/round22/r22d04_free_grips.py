# Theory round 22, R22D-04: the senior reviewer's exact computation, kept as written apart from this header;
# the integrator re-ran it and got the output in runs/round22/r22d04_free_grips.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22d04_free_grips.py
import sys,json,resource,time
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(870,870))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import numpy as np
import eb6 as E
from screen import load
from sim.kernel import Evaluator,pullback_form
R,T,sim=E.R,E.T,E.sim
r=json.loads(Path('research/jumbling/restoration/12-low-height/runs/eb6-k100.json').read_text())
st,M,_=load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k100-s7102.json')
assert st.digest()==r['digest'] and not st.filtered
st.journal=[]; C=st.ctx
branches=[T.branch(tuple(n)) for n in C.data.N]
Uall=sorted(p for u in r['units'] for p in u['piece_ids'])
info={pid:T.pose_tree(st._pose_matrix(pid)) for pid in np.unique(st.pose_id).tolist()}
assert sorted(p for p in range(len(st.pose_id)) if info[int(st.pose_id[p])][0]>=4)==Uall
cap=st.classify(61,need_inside=True,stop_at_straddle=True)
assert cap.straddle is None and not cap.uncertain and set(Uall)<=set(cap.inside.tolist())
print('digest',st.digest(),'high',len(Uall),'cap61',len(cap.inside),flush=True)
for j in [15,18,22,33,52,53,54,55,56,57,58,59]:
 o=st.apply(sim.Twist(C,61,R.element(M,61,j),'menu',{'menu':M.name}))
 assert o.applied and not o.uncertain
 info={pid:T.pose_tree(st._pose_matrix(pid)) for pid in np.unique(st.pose_id).tolist()}
 cache={}; checks={}; rows=[]
 for u in r['units']:
  cells={}; blocked=0; free_union=set()
  for p in u['piece_ids']:
   pid=int(st.pose_id[p]);h,_,b,_=info[pid];assert h==2
   if pid not in cache:
    q=st._pose_matrix(pid)
    cache[pid]=[(c,Evaluator(pullback_form(q,C.data.N[c]))) for c in range(600) if branches[c]==b]
   caps=[]; free=[]
   for c,ev in cache[pid]:
    v=ev.classify(C.regions.vform(p))[0];assert v in ('in','out')
    if v!='in':continue
    caps.append(c)
    if c not in checks:
     cl=st.classify(c,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
     checks[c]=cl.straddle is None and all(st._poses[int(st.pose_id[v])].kidx>=0 or (info[int(st.pose_id[v])][0]==2 and info[int(st.pose_id[v])][2]==b) for v in cl.inside)
    if checks[c]:free.append(c)
   cells[tuple(caps)]=cells.get(tuple(caps),0)+1
   blocked+=not free;free_union.update(free)
  rows.append([u['first'],len(cells),blocked,sorted(free_union),[[list(k),v] for k,v in sorted(cells.items())] if j in (55,57,59) else None])
 print(json.dumps({'item':j,'rows':rows}),flush=True)
 st.undo();assert st.digest()==r['digest']
print('DONE cpu',round(time.process_time(),2),flush=True)
