# Theory round 22, line (d), round 5 (R22D-61 to R22D-64): at the seed 7102 stop after R22D-16 and R22D-43 (state
# f0290f93...), the clean caps that hold height-4 pieces, the blockers of caps 150 and 179, and, after each of the
# twelve lowering items of cap 61, the source cell, free grips and occupied SAFE-fixed home chambers of every height-4
# piece: the whole-unit cases of R22D-19 and the content seeds of R22D-60, each with its first failure; then the units
# that cap 150 holds whole, and the clean caps that hold a blocker of cap 179.
# The senior reviewer's exact computation, kept as written apart from this header, with the two checks of R22D-63 and
# R22D-64 appended where the reviewer said; the integrator re-ran it and got the output in
# runs/round22/r22d61_audit.txt, whose summary lines are the ones the reviewer reported. About 5 CPU minutes.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d61_audit.py
import sys,runpy,time
z=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/r22d43_twelve.py')
st,M,E,chk=z['st'],z['M'],z['E'],z['chk']
C=st.ctx
import json, numpy as np, buffer as B
from pathlib import Path
from collections import Counter,defaultdict
from sim.kernel import Evaluator,pullback_form
base=st.digest()
off0=st._kpose()[st.pose_id]<0
info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
high=np.array([p for p in range(len(st.pose_id)) if info[int(st.pose_id[p])][0]==4],np.int64)
old=json.loads(Path('research/jumbling/restoration/12-low-height/runs/eb6-k100.json').read_text())['units']
units={str(u['first']):sorted(set(u['piece_ids'])&set(high.tolist())) for u in old}
units={u:ps for u,ps in units.items() if ps}
home=C.data.sig_matrix(np.arange(len(st.pose_id)))
branches=np.array([E.T.branch(tuple(n)) for n in C.data.N])
clean={}
for c in range(600):
 cl=st.classify(c,need_inside=True,stop_at_straddle=True)
 assert not cl.uncertain
 if cl.straddle is None: clean[c]=set(cl.inside.tolist())
print('START',base,'heights',dict(Counter(int(info[int(p)][0]) for p in st.pose_id)),'clean',len(clean),'units',{u:len(ps) for u,ps in units.items()},flush=True)
print('CLEAN_HIGH_CAPS',[(c,len(set(high.tolist())&ins)) for c,ins in clean.items() if set(high.tolist())&ins],flush=True)
for u,ps in units.items():
 print('UNIT_CAPS',u,[c for c,ins in clean.items() if set(ps)<=ins],flush=True)
for c in (150,179):
 ss=E.straddler_set(st,c).tolist()
 print('BLOCKERS',c,[(p,int(info[int(st.pose_id[p])][0])) for p in ss],flush=True)
audit=[]
for j in (15,18,22,33,52,53,54,55,56,57,58,59):
 o=st.apply(E.sim.Twist(C,61,E.R.element(M,61,j),'menu',{'menu':M.name}))
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
 cells={};pcell={};ecache={}
 for p in high.tolist():
  pid=int(st.pose_id[p]);h,_,b,_=info[pid];assert h==2
  poles=np.flatnonzero(branches==b)
  free={c for c in poles if c in clean and types[c]<={(2,int(b))}}
  if pid not in ecache:ecache[pid]=[(int(c),Evaluator(pullback_form(st._pose_matrix(pid),C.data.N[c]))) for c in poles]
  cell=[]
  for c,ev in ecache[pid]:
   v=ev.classify(C.regions.vform(p))[0];assert v in ('in','out')
   if v=='in':cell.append(c)
  cell=tuple(cell);key=(int(b),cell);pcell[p]=key
  if key not in cells:
   X=np.flatnonzero((home[:,poles]==np.isin(poles,cell)[None,:]).all(axis=1))
   fx=X[fixed[X]]
   cells[key]={'branch':int(b),'cell':list(cell),'pieces':[],'free_containing':sorted(int(c) for c in free&set(cell)),'free_count':len(free),'home':len(X),'fixed_occupied':len(fx),'witness':int(fx[0]) if len(fx) else None}
  cells[key]['pieces'].append(p)
 urows=[]
 for u,ps in units.items():
  why='outside-free' if any(not cells[pcell[p]]['free_containing'] for p in ps) else ('source-fixed' if any(cells[pcell[p]]['fixed_occupied'] for p in ps) else 'survivor')
  urows.append([u,why])
 row={'item':j,'safe':len(safe),'clean':len(clean),'units':urows,'cells':list(cells.values())}
 audit.append(row)
 print('LOWERING',json.dumps(row),'CPU',round(time.process_time(),1),flush=True)
 st.undo();assert st.digest()==base
print('AUDIT_DONE',len(audit),round(time.process_time(),1),flush=True)
ordered_fail=Counter()
for row in audit:
 fs=[]
 for u,ps in units.items():
  cs=[cc for cc in row['cells'] if set(ps)&set(cc['pieces'])]
  why='split-cell' if len(cs)>1 else ('outside-free' if not cs[0]['free_containing'] else 'source-fixed' if cs[0]['fixed_occupied'] else 'survivor')
  ordered_fail[why]+=1
  fs.append([u,why])
 print('ORDERED_FAILURES',row['item'],fs,flush=True)
print('ORDERED_COUNTS',dict(ordered_fail),flush=True)
print('SEED_COUNTS',dict(Counter({why:sum(len(cc['pieces']) for row in audit for cc in row['cells'] if ('outside-free' if not cc['free_containing'] else 'source-fixed' if cc['fixed_occupied'] else 'survivor')==why) for why in ('outside-free','source-fixed','survivor')})),flush=True)
print('VERSIONS',sys.version.split()[0],np.__version__,flush=True)
cl150=st.classify(150,need_inside=True,stop_at_straddle=False)
print('CAP150_HIGH_WHOLE',[(u,len(ps)) for u,ps in units.items() if set(ps)<=set(cl150.inside.tolist())],flush=True)
info0={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
S=set(E.straddler_set(st,179).tolist())
safe0=[];caprows=[];clean0={};types0={}
for c in range(600):
 cl=st.classify(c,need_inside=True,stop_at_straddle=False);assert not cl.uncertain
 if cl.straddle is not None:continue
 clean0[c]=cl.inside
 types0[c]={(int(info0[int(st.pose_id[p])][0]),int(info0[int(st.pose_id[p])][2])) for p in cl.inside if off0[p]}
 if not types0[c]:safe0.append(c)
 hit=sorted(S&set(cl.inside.tolist()))
 if hit:
  caprows.append({'cap':c,'branch':int(branches[c]),'blockers':hit,'lattice':int((~off0[cl.inside]).sum()),'high':int(np.isin(cl.inside,high).sum()),'off_types':sorted(types0[c])})
print('CAP179_CLEAN_CONTAINING',json.dumps(caprows),flush=True)
print('SAFE_AT_START',len(safe0),flush=True)
