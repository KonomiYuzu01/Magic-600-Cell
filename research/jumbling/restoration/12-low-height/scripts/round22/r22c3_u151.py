# Theory round 22, line (c), round 3 (R22C3-04): at the depth 60 stop, unit 151 as the whole class-1 content
# of its edge cell; the exact endpoint of the pure-cell word that Theorem C guarantees (computed without building the
# word), N1 and N2 for its 86 home chambers there, and global Jordan certificates on every nontrivial target orbit.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c3_u151.txt (about 100 CPU seconds).
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22c3_u151.py
import sys,json,resource,signal,time,random
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(1000,1010))
def stop(*a):print('CUT_SHORT_CPU_1000_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul,matvec
from sim.kplus import to_tuple
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json')
B=s.B;D=s.rim.Domains(st);dom=np.asarray(D.dom);members,cls=s.CL.classes();P=1;cell=[443,481]
dcls={}
for d in range(1,len(D.reps)):
 h=s.CL.hub(D.reps[d],members,cls);dcls[d]=h[0][1] if len(h)==1 else None
off=B.off_pieces(st);own=off[np.array([dcls[int(dom[p])]==P for p in off])];keep=np.ones(len(own),bool)
for c in members[P]:
 c=int(c);cl=st.classify(c,need_inside=True,stop_at_straddle=False);assert not cl.uncertain
 inside=np.asarray(cl.inside,np.int64);keep &= (np.isin(own,inside)==(c in cell))
 if c in [443,481,286]:assert cl.straddle is None and all(dom[p]==0 or dcls[int(dom[p])]==P for p in inside)
U=own[keep];assert len(U)==45 and int(U.min())==102219 and len(set(dom[U]))==1
poles=list(map(int,members[P]));w={c:s.O.UIDX[tuple(s.T.qmul(s.CL.ninv(poles[0]),s.CL.N[c]))] for c in poles}
nbr={i for i,u in enumerate(s.O.U) if u[0]==s.O.HALF_PHI}
assert all(int(s.O.MUL[s.O.INV[w[c]],w[286]]) in nbr for c in cell)
home=s.CL.data.sig_matrix(np.arange(len(st.pose_id)));T=B.cell_chambers(home,members,P,cell)
Q=np.flatnonzero((dom==0)&np.isin(np.asarray(D.cham),T));assert len(T)==len(Q)==86
choices=[]
for i in range(len(M.items)):
 g=s.restore.element(M,443,i)
 if tuple(matvec(g,s.CL.N[481]))!=tuple(s.CL.N[481]):continue
 if all(s.CL.ctx.kplus.index_of_matrix(to_tuple(matmul(g,st.pose(int(p)))))>=0 for p in U):choices.append((i,g))
assert len(choices)==1;item,g=choices[0]
# Exact endpoint of the pure C5 word guaranteed by Theorem C, not a single legal twist.
for ps in [U,Q]:
 old={int(pid):to_tuple(matmul(g,st._pose_matrix(int(pid)))) for pid in np.unique(st.pose_id[ps])}
 ids=st.pose_id[ps].copy()
 for pid,matrix in old.items():st.pose_id[ps[ids==pid]]=st._intern_matrix(matrix)
st._compact();assert not np.isin(U,B.off_pieces(st)).any()
G=B.Alphabet(SimpleNamespace(st=st),M);G.orbits();cond=B.condition(G,T)
print('U151_PURE_ENDPOINT',st.digest(),'rotation_item_at_443',item,'U',len(U),'Q',len(Q),'off',int(st.off_lattice_count()),flush=True)
print('CONDITION',json.dumps(cond),flush=True)
if cond['necessary'] and cond['fixed_vacant']:
 want={int(o) for o in G.orbit[T] if G.orbit_size[o]>1};found=set()
 for stage,(length,tries) in enumerate([(120,200),(400,400),(800,400)]):
  rng=random.Random(1+stage)
  for trial in range(tries):
   if found==want:break
   word=[rng.randrange(len(G.letters)) for _ in range(length)];lab,lens=B.cycles(G.perm(word))
   for o in sorted(want-found):
    try:c=B._jordan_from_cycles(G,lab,lens,o)
    except ValueError:continue
    assert B.check_jordan(G,G.pairs(word),o,c['prime'])==c;found.add(o)
  if found==want:break
 print('JORDAN_DONE',sorted(want-found),flush=True)
print('COMPLETED','cpu',round(time.process_time(),1),flush=True)
