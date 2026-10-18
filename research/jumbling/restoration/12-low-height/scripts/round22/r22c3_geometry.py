# Theory round 22, line (c), round 3 (R22C3-04, R22C3-05): Theorem C's premises for unit 142 at the depth 60
# stop with the disjointness and type-count facts of the order 142 then 151, and b17's cell, poles and pole blockers
# at the depth 40 stop.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c3_geometry.txt (a few CPU minutes).
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22c3_geometry.py
import sys,json,resource,signal
from pathlib import Path
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(700,710))
def stop(*a):print('CUT_SHORT_CPU_700_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
S=Path('research/jumbling/restoration/12-low-height')
sys.path.insert(0,str((S/'scripts').resolve()))
import screen as s,numpy as np
from exact import matmul,matvec
from sim.kplus import to_tuple
for depth,seed,uid in [(60,7104,142),(40,7101,17)]:
 st,M,_=s.load('record',f'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k{depth}-s{seed}.json')
 rec=json.loads((S/f'runs/screen-k{depth}.json').read_text());assert st.digest()==rec['digest']
 row=next(u for u in rec['units'] if u['unit']==uid);P=row['class'];cell=row['cell']
 members,cls=s.CL.classes();D=s.rim.Domains(st);dom=np.asarray(D.dom)
 dcls={}
 for d in range(1,len(D.reps)):
  h=s.CL.hub(D.reps[d],members,cls);dcls[d]=h[0][1] if len(h)==1 else None
 d=int(dom[row['first']]);assert dcls[d]==P
 cand=np.flatnonzero(np.isin(dom,[dd for dd in dcls if dcls[dd]==P]));keep=np.ones(len(cand),bool);cache={}
 for c in members[P]:
  c=int(c);cl=st.classify(c,need_inside=True,stop_at_straddle=False);assert not cl.uncertain;cache[c]=cl
  keep &= (np.isin(cand,cl.inside)==(c in cell))
 U=cand[keep];assert len(U)==row['size'] and int(U.min())==row['first'] and np.all(dom[U]==d)
 def free(c):
  cl=cache[c]
  return cl.straddle is None and all(dom[p]==0 or dcls[int(dom[p])]==P for p in cl.inside)
 home=s.CL.data.sig_matrix(np.arange(len(st.pose_id)));T=s.B.cell_chambers(home,members,P,cell)
 Q=np.flatnonzero((dom==0)&np.isin(D.cham,T));assert len(T)==86 and not len(Q)
 poles=list(map(int,members[P]));w={c:s.O.UIDX[tuple(s.T.qmul(s.CL.ninv(poles[0]),s.CL.N[c]))] for c in poles}
 nbr={i for i,u in enumerate(s.O.U) if u[0]==s.O.HALF_PHI}
 common=[c for c in poles if all(int(s.O.MUL[s.O.INV[w[a]],w[c]]) in nbr for a in cell)]
 good=[c for c in common if free(c)]
 print('GEOMETRY',depth,uid,st.digest(),'U',len(U),'vacant',len(T),'poles_free',[free(c) for c in cell],'free_common',good,flush=True)
 if depth==60:
  assert all(free(c) for c in cell) and good
  choices=[i for i in range(len(M.items)) if tuple(matvec(s.restore.element(M,cell[0],i),s.CL.N[cell[1]]))==tuple(s.CL.N[cell[1]]) and all(s.CL.ctx.kplus.index_of_matrix(to_tuple(matmul(s.restore.element(M,cell[0],i),st.pose(int(p)))))>=0 for p in U)]
  assert len(choices)==1
  T151=s.B.cell_chambers(home,members,1,[443,481])
  assert not np.intersect1d(T,T151).size and len(np.flatnonzero((dom==0)&np.isin(D.cham,T151)))==86
  maximum=max(Counter(map(int,np.asarray(s.CL.ctx.regions.orbit_of)[U])).values());assert maximum<=8
  print('PREFIX_142_151','rotation_item',choices[0],'disjoint_cells',True,'max_type_consumption',maximum,flush=True)
 else:
  assert [free(c) for c in cell]==[False,True]
  foreign={int(p) for p in cache[542].inside if dom[p]!=0 and dcls[int(dom[p])]!=P}
  ids=[];covered=set()
  for j in range(22):
   f=S/f'runs/round22/line-a/units/eb5-k40-k00-b{j:02d}.json';ps=set(json.loads(f.read_text())['pieces'])
   if ps&foreign:ids.append(j);covered |= ps&foreign
  assert ids==[8,9,10] and covered==foreign
  print('B17_BLOCKERS',ids,flush=True)
print('PASS',flush=True)
