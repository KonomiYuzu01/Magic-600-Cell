# Theory round 22, coverage round 3 (COV-22): the adjudicating experiment for the reachability of COV-01's state.
# For one seed it samples up to 20,000 words J·[[[a,b],c],d]·J⁻¹ of 24 letters (J = (0, 12); a an own-class jumble
# letter and c a foreign-class jumble letter at grips local to chambers 1474, 404 and 762; b and d retained letters
# there), applies every prefix with the exact simulator, and stops at FOUND if the endpoint's only off piece is 1474
# at exactly R = element(I_a, 0, 25). It has a 1,700-second CPU cutoff (CUT_SHORT). FOUND decides the question
# positively; NO_HIT or CUT_SHORT decides nothing, since the family is sampled, not exhausted.
# The senior reviewer's experiment, kept as written apart from this header; the integrator ran seeds 0 to 7, two at a
# time, and kept their outputs in runs/round22/cov22/seed<k>.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov22_mixed_search.py <seed>
import sys,json,random,time,resource,signal
from pathlib import Path
seed=int(sys.argv[1]);trial=-1
resource.setrlimit(resource.RLIMIT_CPU,(1700,1720))
def stop(*a):
 print('CUT_SHORT',{'seed':seed,'completed_trials':trial,'cpu':time.process_time()},flush=True)
 raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import transpose
from sim.kplus import to_tuple
M=s.restore.menu('I_a');ctx=s.CL.ctx;R=s.restore.element(M,0,25)
base=[s.restore.element(M,0,i) for i in range(60)];idx={g:i for i,g in enumerate(base)}
ii=[idx[to_tuple(transpose(g))] for g in base]
ret=[i for i,g in enumerate(base) if i!=0 and ctx.kplus.index_of_matrix(g)>=0]
jum=[i for i,g in enumerate(base) if ctx.kplus.index_of_matrix(g)<0]
assert ii[12]==45
members,cls=s.CL.classes();home=s.CL.data.sig_matrix(np.arange(len(ctx.regions.orbit_of)))
local=np.flatnonzero(home[1474]|home[404]|home[762]).tolist()
own=[c for c in local if cls[c]==0];foreign=[c for c in local if cls[c]!=0]
assert own and foreign and ret and jum
rng=random.Random(220300+seed)
def inv(w):return [(c,ii[i]) for c,i in reversed(w)]
def comm(a,b):return a+b+inv(a)+inv(b)
for trial in range(20000):
 a=[(rng.choice(own),rng.choice(jum))]
 b=[(rng.choice(local),rng.choice(ret))]
 c=[(rng.choice(foreign),rng.choice(jum))]
 d=[(rng.choice(local),rng.choice(ret))]
 w=[(0,12)]+comm(comm(comm(a,b),c),d)+[(0,45)]
 assert len(w)==24
 st=s.sim.State(ctx,menu=M);legal=True
 for grip,item in w:
  out=st.apply(s.sim.Twist(ctx,grip,s.restore.element(M,grip,item),'menu',{'menu':M.name}),journal=False)
  if not out.applied or out.uncertain:
   legal=False;break
 if legal and s.B.off_pieces(st).tolist()==[1474] and st.pose(1474)==R:
  holes=s.B.hole_set(st)
  assert len(holes)==1 and int(ctx.regions.orbit_of[int(holes[0])])==18
  print('FOUND',json.dumps({'seed':seed,'trial':trial,'word':w,'length':len(w),'digest':st.digest(),'off':[1474],'holes':holes.tolist()}),flush=True)
  break
else:
 print('NO_HIT',{'seed':seed,'trials':20000},flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
