# Theory round 22, R22C2-05 (line (c)): the twelve landing rotations of b06 and their fixed chambers.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c2_05_vertex.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22c2_05_vertex.py
import sys,json,resource
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(500,500))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s, split_pairs as SP
import numpy as np
from exact import matmul,transpose
from sim.kplus import to_tuple
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json')
assert st.digest()=='7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818'
bl=s.BL.Blocks(SimpleNamespace(st=st),M)
U=np.array([16314,16315,16329,16330,24016,24030,24031,24032,24033,29366])
assert np.array_equal(U,bl.blocks[6])
D=s.rim.Domains(st);reg=s.CL.ctx.regions;kp=s.CL.ctx.kplus
members,_=s.CL.classes();home=s.CL.data.sig_matrix(np.arange(len(st.pose_id)))
T=s.B.cell_chambers(home,members,3,[91]);lat=np.flatnonzero(st.lattice_flags());Q=lat[np.isin(D.cham[lat],T)]
H=s.B.hole_set(st);outside=np.setdiff1d(H,T);typ=np.asarray(reg.orbit_of)
deficit=np.maximum(0,np.bincount(typ[U],minlength=36)-np.bincount(typ[outside],minlength=36))
ret={}
def movers(c,x):
 if c not in ret:
  ret[c]=np.array([k for i in range(len(M.items)) if (k:=kp.index_of_matrix(s.restore.element(M,c,i)))>=0],np.int64)
 ks=ret[c];ys=reg.piece_of[typ[x],kp.compose_vec(ks,np.full(len(ks),reg.transport[x]))]
 return bool((ys!=x).any())
run=Path('research/jumbling/restoration/12-low-height/runs/round22/line-c')
a=json.loads((run/'exp-a.json').read_text());b=json.loads((run/'exp-b.json').read_text())
for name,c,rec in [('b15',286,a['b15']),('u130',429,b['130']),('u180',555,b['180'])]:
 xs=rec['closure']['premises']['fixed_occupied'];assert all(home[x,c] and movers(c,x) for x in xs)
 print(json.dumps({'Q_free_common_cap':c,'case':name,'all_fixed_chambers_movable_by_its_retained_group':len(xs)}),flush=True)
land=[]
for i in range(len(M.items)):
 g=s.restore.element(M,91,i);ku=np.array([kp.index_of_matrix(to_tuple(matmul(g,st.pose(int(p))))) for p in U])
 if (ku>=0).all():land.append((i,g,reg.piece_of[typ[U],kp.compose_vec(ku,reg.transport[U])]))
assert len(land)==12
_,g0,L0=land[0]
qpose={int(pid):to_tuple(matmul(g0,st._pose_matrix(int(pid)))) for pid in np.unique(st.pose_id[Q])}
qhit={}
for c in sorted({int(c) for x in L0 for c in np.flatnonzero(home[x])}):
 qhit[c]=None
 for p in Q:
  side=SP.side(qpose[int(st.pose_id[p])],int(p),c)
  assert side in ('in','out','straddle')
  if side!='out':qhit[c]=[int(p),side];break
other=~np.isin(bl.off,U);unitof={int(p):u for u,ps in enumerate(bl.blocks) for p in ps}
print(json.dumps({'b06_Q':len(Q),'cell':len(T),'outside_holes':len(outside),'type_deficits':{int(i):int(v) for i,v in enumerate(deficit) if v},'reference_landing_item':land[0][0]}),flush=True)
for i,g,L in land:
 k=kp.index_of_matrix(to_tuple(matmul(g,transpose(g0))));assert k>=0
 assert np.array_equal(L,reg.piece_of[typ[L0],kp.compose_vec(np.full(len(L0),k),reg.transport[L0])])
 fixed=[];qfixed=[];minimal=Counter();firstcert=None
 for x0,x in zip(L0,L):
  cs=[int(c) for c in np.flatnonzero(home[x0])]
  entries=[];qfree=[]
  for c0 in cs:
   c=int(kp.perms[k,c0]);rows=np.flatnonzero(other&(bl.labels[:,c]!=0));us=frozenset(unitof[int(bl.off[r])] for r in rows)
   entries.append({'grip':c,'Q':qhit[c0],'units':sorted(us),'moves':movers(c,int(x)),'off_witness':None if not len(rows) else [int(bl.off[rows[0]]),int(bl.labels[rows[0],c])]})
   if not qhit[c0] and entries[-1]['moves']:qfree.append(us)
  if all(e['Q'] or not e['moves'] for e in entries):qfixed.append(int(x))
  if all(e['Q'] or e['units'] or not e['moves'] for e in entries):
   fixed.append(int(x))
   mins={a for a in qfree if not any(b<a for b in qfree)};minimal.update(tuple(sorted(a)) for a in mins)
   if firstcert is None:firstcert={'chamber':int(x),'caps':entries}
 print(json.dumps({'item':i,'fixed':fixed,'Q_only_fixed':qfixed,'minimal_Qfree_blockers':[[list(a),n] for a,n in sorted(minimal.items())],'first_certificate':firstcert}),flush=True)
print('completed_without_cutoff',flush=True)
