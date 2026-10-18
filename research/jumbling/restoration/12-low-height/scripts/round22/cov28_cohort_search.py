# Theory round 22, coverage round 4 (COV-28): the adjudicating experiment for the reachability of COV-01's state, a
# variant of COV-22 whose foreign segment must change the poses of X's 111-piece home-cell cohort. For one seed it
# samples words L·[[[A,B],C],D]·L⁻¹ of 36 to 48 letters (L one of X's 36 landing letters; A an own-class and C a
# foreign-class jumble word of two or three letters at grips local to chambers 1474, 404 and 762; B and D retained
# letters there), applies every prefix with the exact simulator (a blocked prefix discards the trial), and stops at
# FOUND if the endpoint's only off piece is 1474 at exactly element(I_a, 0, 25), checking that the inverse word returns
# the solved digest. It has a 1,700-second CPU cutoff (CUT_SHORT) and at most 1,000,000 trials (NO_HIT). FOUND decides
# the question positively; NO_HIT or CUT_SHORT decides nothing, since the family is sampled, not exhausted.
# The senior reviewer's experiment, kept as written apart from this header; the integrator ran seeds 0 to 7, four at
# a time, and kept their outputs in runs/round22/cov28/seed<k>.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov28_cohort_search.py <seed>
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
R=E[25];ret=list(range(1,12));jum=list(range(12,60));members,cls=s.CL.classes();P=members[0]
home=s.CL.data.sig_matrix(np.arange(len(ctx.regions.orbit_of)));local=np.flatnonzero(home[1474]|home[404]|home[762]).tolist();own=[c for c in local if cls[c]==0];foreign=[c for c in local if cls[c]!=0]
cohort=np.flatnonzero((home[:,P]==np.isin(P,[0,2,23])[None,:]).all(axis=1)).tolist();assert len(cohort)==111
LAND=[(c,i) for c,items in [(0,[12,16,20,24,25,26,27,28,29,30,31,32]),(2,[14,21,23,43,44,45,46,47,48,49,50,51]),(23,[12,16,20,24,25,26,27,28,29,30,31,32])] for i in items]
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
 if s.B.off_pieces(st).tolist()!=[1474] or st.pose(1474)!=R:continue
 H=list(hist);digest=st.digest();holes=s.B.hole_set(st).tolist();assert len(holes)==1 and int(ctx.regions.orbit_of[holes[0]])==18
 assert apply(inv(H)) and st.digest()==solved
 print('FOUND',json.dumps({'seed':seed,'trial':trial,'word':H,'length':len(H),'digest':digest,'holes':holes,'inverse_returns_solved':True}),flush=True)
 break
else:print('NO_HIT',{'seed':seed,'trials':1000000},flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
