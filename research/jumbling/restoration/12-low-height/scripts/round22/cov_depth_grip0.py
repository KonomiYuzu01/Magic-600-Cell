# Coverage round 4 input, the integrator's table: the depth of every chamber of cap 0 after one jumble letter. For each
# jumble letter L at grip 0 (a menu item off K⁺) it builds solved·L, whose holes are the chambers of cap 0, and gives
# each of them its orbit under the SAFE group of solved·L: deep if that orbit holds no occupied chamber (a singleton
# orbit is the extreme case), rim otherwise. A word L·V·L⁻¹ with V SAFE can leave a single off piece only by bringing
# one lattice piece into a rim chamber of cap 0, so the table counts, for each L, the rim and deep chambers by piece
# type and lists the deep chambers with the sizes of their orbits (the rim ones are the rest). Every grip's menu is the
# K⁺-conjugate of grip 0's (COV-19), so grip 0 stands for every grip up to the global symmetry.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root, optionally for a shard k of n (letters in menu order, every n-th from the k-th):
# OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov_depth_grip0.py [k n]
import sys,json,time,resource,signal
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(3400,3420))
def stop(*a):print('CUT_SHORT_CPU_3400_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
k,n=(int(sys.argv[1]),int(sys.argv[2])) if len(sys.argv)==3 else (0,1)
home=s.CL.data.sig_matrix(np.arange(len(reg.orbit_of)))
cap0=np.flatnonzero(home[:,0])
jum=[i for i in range(len(M.items)) if kp.index_of_matrix(s.restore.element(M,0,i))<0]
print('LETTERS',json.dumps({'jumble_items':len(jum),'shard':[k,n],'cap0_chambers':int(len(cap0))}),flush=True)
for i in jum[k::n]:
  t0=time.process_time()
  L=s.restore.element(M,0,i)
  st=s.sim.State(ctx,menu=M)
  o=st.apply(s.sim.Twist(ctx,0,L,'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
  G=s.B.Alphabet(SimpleNamespace(st=st),M);orb,sizes=G.orbits()
  holes=np.asarray(G.holes);hc=np.bincount(orb[holes],minlength=len(sizes))
  assert set(holes.tolist())==set(cap0.tolist())
  occ=sizes-hc;oc=orb[cap0]
  rim=cap0[occ[oc]>0];deep=cap0[occ[oc]==0];single=cap0[sizes[oc]==1]
  typ=lambda a:dict(sorted(Counter(int(G.type[y]) for y in a).items()))
  rec={'L':[0,i],'digest':st.digest(),'off':len(s.B.off_pieces(st)),'safe_letters':len(G.letters),'orbits':len(sizes),
       'rim':int(len(rim)),'deep':int(len(deep)),'deep_singleton':int(len(single)),
       'rim_by_type':typ(rim),'deep_by_type':typ(deep),
       'deep_chambers':[[int(y),int(sizes[orb[y]])] for y in deep],
       'cpu':round(time.process_time()-t0,1)}
  print('DEPTH',json.dumps(rec),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
