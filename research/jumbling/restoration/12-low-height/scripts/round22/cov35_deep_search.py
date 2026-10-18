# Theory round 22, coverage round 5 (COV-35): the adjudicating experiment for the reachability of a valid deep
# singleton. It is COV-28's exact-prefix commutator search, adapted by the senior reviewer to eight valid deep
# representatives of COV-30 (12, 41, 64, 446, 449, 2172, 2149 and 2249; seed k takes the one at index k mod 8): the
# target's own cell, its home-cell cohort, the grips local to them and every landing letter of the cell. For one seed it
# samples words L·[[[A,B],C],D]·L⁻¹ of 36 to 48 letters (L a landing letter; A an own-class and C a foreign-class jumble
# word of two or three letters at local grips; B and D retained letters there), applies every prefix with the exact
# simulator (a blocked prefix discards the trial), and stops at FOUND if the endpoint's only off piece is the target at
# exactly element(I_a, 0, 45), checking that the inverse word returns the solved digest. It has a 1,700-second CPU
# cutoff (CUT_SHORT) and at most 1,000,000 trials (NO_HIT). FOUND decides that a valid deep class is reachable; NO_HIT
# or CUT_SHORT decides nothing, since the family is sampled, not exhausted.
# The source is the one the reviewer's generator builds from cov28_cohort_search.py by string replacement, kept as
# generated apart from this header; the integrator ran seeds 0 to 7, four at a time, and kept their outputs in
# runs/round22/cov35/seed<k>.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov35_deep_search.py <seed>
import sys,json,random,time,resource,signal
from pathlib import Path
seed=int(sys.argv[1]);trial=-1
resource.setrlimit(resource.RLIMIT_CPU,(1700,1720))
def stop(*a):
 print('CUT_SHORT',{'seed':seed,'trial':trial,'cpu':time.process_time()},flush=True)
 raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import transpose
from sim.kplus import to_tuple
M=s.restore.menu('I_a');ctx=s.CL.ctx
E=[s.restore.element(M,0,i) for i in range(60)];idx={v:i for i,v in enumerate(E)};ii=[idx[to_tuple(transpose(v))] for v in E]
R=E[45];target=[12, 41, 64, 446, 449, 2172, 2149, 2249][seed%8];ret=list(range(1,12));jum=list(range(12,60));members,cls=s.CL.classes();P=members[0]
from exact import matmul
import split_pairs as SP
home=s.CL.data.sig_matrix(np.arange(len(ctx.regions.orbit_of)))
sides=[SP.side(R,target,c) for c in range(600)]
cell=[c for c in P if sides[c]=='in']
cohort=np.flatnonzero((home[:,P]==np.isin(P,cell)[None,:]).all(axis=1)).tolist()
local=np.flatnonzero(home[target]|home[cohort].any(axis=0)).tolist()
own=[c for c in local if cls[c]==0];foreign=[c for c in local if cls[c]!=0]
LAND=[(c,i) for c in range(600) if sides[c]=='in' for i in range(12,60) if ctx.kplus.index_of_matrix(to_tuple(matmul(s.restore.element(M,c,i),R)))>=0]
assert LAND and cohort and own and foreign and s.T.dist(s.T.factors(R)[0])==2
rng=random.Random(220400+seed)
def inv(w):return [(c,ii[i]) for c,i in reversed(w)]
def apply(w):
 for c,i in w:
  o=st.apply(s.sim.Twist(ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}),journal=False)
  if not o.applied or o.uncertain:return False
  hist.append((c,i))
 return True
def grow(caps,items,length):
 w=[]
 for _ in range(length):
  choices=[c for c in caps if not w or c!=w[-1][0]]
  if not choices:return None
  q=(rng.choice(choices),rng.choice(items))
  if not apply([q]):return None
  w.append(q)
 return w
for trial in range(1000000):
 st=s.sim.State(ctx,menu=M);solved=st.digest();hist=[];J=rng.choice(LAND)
 assert apply([J])
 A=grow(own,jum,rng.choice([2,3]))
 if A is None:continue
 B=grow(local,ret,1)
 if B is None or not apply(inv(A)+inv(B)):continue
 U=A+B+inv(A)+inv(B)
 before=[st.pose(p) for p in cohort]
 C=grow(foreign,jum,rng.choice([2,3]))
 if C is None:continue
 if all(st.pose(p)==q for p,q in zip(cohort,before)):continue
 if not apply(inv(U)+inv(C)):continue
 V=U+C+inv(U)+inv(C)
 D=grow(local,ret,1)
 if D is None or not apply(inv(V)+inv(D)+inv([J])):continue
 if s.B.off_pieces(st).tolist()!=[target] or st.pose(target)!=R:continue
 H=list(hist);digest=st.digest();holes=s.B.hole_set(st).tolist();assert len(holes)==1 and int(ctx.regions.orbit_of[holes[0]])==int(ctx.regions.orbit_of[target])
 assert apply(inv(H)) and st.digest()==solved
 print('FOUND',json.dumps({'seed':seed,'trial':trial,'target':target,'word':H,'length':len(H),'digest':digest,'holes':holes,'inverse_returns_solved':True}),flush=True)
 break
else:print('NO_HIT',{'seed':seed,'trials':1000000},flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
