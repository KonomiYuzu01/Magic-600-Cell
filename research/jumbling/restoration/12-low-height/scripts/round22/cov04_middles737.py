# Theory round 22, coverage shard (COV-04): at the replay state of 737 (digest 2e3b8f99...), the T1' middles of b05
# (class-3 cell {12,91}, landing item 17 at grip 12) and b15 (cell {217,324}, item 58 at grip 217) are built from the
# pure-cell accumulated rotations, without expanding the words. For each fixed occupied landing chamber the script
# lists which off units block every grip that moves it (minimal blocker sets), checks the fixed list against
# buffer.condition, and names the Q-free grips that move all of them. Then b06's inventory in the class-3 cell {91}:
# type by type, U plus holes inside against all holes (R22C-V).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov04_middles737.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov04_middles737.py
import sys,json,time,resource
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(1100,1120))
S=Path('research/jumbling/restoration/12-low-height')
sys.path.insert(0,str((S/'scripts').resolve()))
import screen as s,numpy as np
from exact import matmul
from sim.kplus import to_tuple
reg=s.CL.ctx.regions;kp=s.CL.ctx.kplus;typ=np.asarray(reg.orbit_of)
home=s.CL.data.sig_matrix(np.arange(len(typ)));members,_=s.CL.classes()
units={u:np.array(json.loads((S/f'runs/round22/line-a/units/eb5-k40-k00-b{u:02d}.json').read_text())['pieces']) for u in range(22)}
unitof={int(p):u for u,ps in units.items() for p in ps}
for u,cell,item in [(5,[12,91],17),(15,[217,324],58)]:
 st,M,_=s.load('table',str(S/'runs/round22/line-a/replay/tables/737.json.gz'))
 assert st.digest()=='2e3b8f990229799fc88198b2fe4f2062ab4f9d73c78347ee67b252d412b0a086'
 U=units[u];off=s.B.off_pieces(st);D=s.rim.Domains(st)
 T=s.B.cell_chambers(home,members,3,cell);Q=np.flatnonzero((np.asarray(D.dom)==0)&np.isin(D.cham,T));g=s.restore.element(M,cell[0],item)
 for ps in (U,Q):
  ids=st.pose_id[ps].copy(); mats={int(pid):to_tuple(matmul(g,st._pose_matrix(int(pid)))) for pid in np.unique(ids)}
  for pid,mat in mats.items():st.pose_id[ps[ids==pid]]=st._intern_matrix(mat)
 st._compact();now=s.B.off_pieces(st)
 assert not np.isin(U,now).any() and np.array_equal(np.sort(np.setdiff1d(now,off)),np.sort(Q))
 D=s.rim.Domains(st);landing=np.sort(np.asarray(D.cham)[U]);bl=s.BL.Blocks(SimpleNamespace(st=st),M)
 capgroups={int(c):frozenset('Q' if int(p) in set(Q.tolist()) else f'b{unitof[int(p)]:02d}' for p in bl.off[bl.labels[:,c]!=0]) for c in sorted({int(c) for x in landing for c in np.flatnonzero(home[x])})}
 fixed=[];qfixed=[];req=Counter();moving={}
 for c in capgroups:
  ks=np.array([k for i in range(len(M.items)) if (k:=kp.index_of_matrix(s.restore.element(M,c,i)))>=0])
  moving[c]={int(x) for x in landing if home[x,c] and np.any(reg.piece_of[typ[x],kp.compose_vec(ks,np.full(len(ks),reg.transport[x]))]!=x)}
 for x in landing.tolist():
  caps=[c for c in capgroups if x in moving[c]]
  if all('Q' in capgroups[c] for c in caps):qfixed.append(x)
  if all(capgroups[c] for c in caps):
   fixed.append(x);ss={capgroups[c] for c in caps if 'Q' not in capgroups[c]}
   req.update(tuple(sorted(a)) for a in ss if not any(b<a for b in ss))
 G=s.B.Alphabet(SimpleNamespace(st=st),M);co=s.B.condition(G,T)
 assert sorted(co['fixed_occupied'])==fixed
 common=[[c,sorted(v)] for c,v in capgroups.items() if 'Q' not in v and set(fixed)<=moving[c]]
 print('MIDDLE',json.dumps({'unit':u,'digest':st.digest(),'U':len(U),'Q':len(Q),'off':len(now),'fixed':len(fixed),'Q_only_fixed':len(qfixed),'minimal_blockers':[[list(a),n] for a,n in sorted(req.items())],'common_caps':common}),flush=True)
st,M,_=s.load('table',str(S/'runs/round22/line-a/replay/tables/737.json.gz'))
H=s.B.hole_set(st);C=s.B.cell_chambers(home,members,3,[91]);I=np.intersect1d(H,C);U=units[6]
h=np.bincount(typ[H],minlength=36);inside=np.bincount(typ[I],minlength=36);u=np.bincount(typ[U],minlength=36)
print('B06',json.dumps({'holes':len(H),'inside':len(I),'outside':len(H)-len(I),'deficits':{int(t):int(u[t]+inside[t]-h[t]) for t in range(36) if u[t]+inside[t]>h[t]},'type6':[int(u[6]),int(h[6]),int(inside[6])],'type9':[int(u[9]),int(h[9]),int(inside[9])]}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
