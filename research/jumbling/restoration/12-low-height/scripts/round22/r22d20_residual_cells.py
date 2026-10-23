# Theory round 22, line (d), round 3 (R22D-20, R22D-21 and R22D-23): for the eight high units left at the seed 7102
# stop after R22D-16 (state dceb9359...) and the six of the seed 7108 stop, after each lowering item of their clean cap
# (61, resp. 415), per unit the number of pieces outside every free grip of the new class and the source cells with
# their counts (pieces, home chambers, occupied home chambers outside every SAFE cap).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d20_residual_cells.txt. About 8 CPU minutes.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d20_residual_cells.py
import sys,json,runpy,resource
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import buffer as B
import numpy as np
from sim.kernel import Evaluator,pullback_form
for seed in (7102,7108):
 if seed==7102:
  z=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/r22d16_step.py')
  st,M,C=z['st'],z['M'],z['C']
  old=json.loads(Path('research/jumbling/restoration/12-low-height/runs/eb6-k100.json').read_text())['units']
 else:
  r=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round3/ia-k100-s7108.json').read_text())
  C=E.sim.get_context();M=E.R.menu('I_a');st=E.sim.State(C,filtered=False,menu=M)
  for c,j in r['history']+r['moves']:
   o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
   assert o.applied and not o.uncertain
 base=st.digest()
 info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
 high={p for p in range(len(st.pose_id)) if info[int(st.pose_id[p])][0]>=4}
 groups={str(u['first']):[p for p in u['piece_ids'] if p in high] for u in old} if seed==7102 else {'all111':sorted(high)}
 groups={u:ps for u,ps in groups.items() if ps}
 print('START',seed,base,{u:len(ps) for u,ps in groups.items()},flush=True)
 home=C.data.sig_matrix(np.arange(len(C.data.sigbits)))
 branches=[E.T.branch(tuple(n)) for n in C.data.N]
 grip=61 if seed==7102 else 415
 for j in (15,18,22,33,52,53,54,55,56,57,58,59):
  o=st.apply(E.sim.Twist(C,grip,E.R.element(M,grip,j),'menu',{'menu':M.name}))
  assert o.applied and not o.uncertain
  info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
  off=st._kpose()[st.pose_id]<0
  clean={};types={};safe=[]
  for c in range(600):
   cl=st.classify(c,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
   if cl.straddle is not None:continue
   clean[c]=cl.inside
   types[c]={(int(info[int(st.pose_id[p])][0]),int(info[int(st.pose_id[p])][2])) for p in cl.inside if off[p]}
   if not types[c]:safe.append(c)
  holes=np.zeros(len(home),bool);holes[B.hole_set(st)]=True
  fixed=(~holes)&(~home[:,safe].any(axis=1))
  rows=[];cache={}
  for u,ps in groups.items():
   cells={};blocked=0
   for p in ps:
    pid=int(st.pose_id[p]);h,_,b,_=info[pid];assert h==2
    poles=[c for c in range(600) if branches[c]==b]
    free={c for c in poles if c in clean and types[c]<={(2,int(b))}}
    if pid not in cache:
     cache[pid]=[(c,Evaluator(pullback_form(st._pose_matrix(pid),C.data.N[c]))) for c in poles]
    cell=[]
    for c,ev in cache[pid]:
     v=ev.classify(C.regions.vform(p))[0];assert v in ('in','out')
     if v=='in':cell.append(c)
    cell=tuple(cell);blocked+=not any(c in free for c in cell)
    if cell not in cells:
     X=np.flatnonzero((home[:,poles]==np.isin(poles,cell)[None,:]).all(axis=1))
     cells[cell]=[0,len(X),int(fixed[X].sum())]
    cells[cell][0]+=1
   rows.append([u,len(ps),blocked,[[list(k),v] for k,v in sorted(cells.items())]])
  print(seed,j,json.dumps(rows),flush=True)
  st.undo();assert st.digest()==base
