# Coverage round 3, the integrator's check after COV-20, second level: words L·g·V·g⁻¹·L⁻¹ for COV-01's family, with
# L a landing letter of X's piece, g a lattice letter at an unstraddled cap of solved·L that holds off pieces (other
# than L's own cap, whose letters only give one-letter words), and V SAFE at solved·L·g. The content that g⁻¹ brings to
# x = (L·R)(1474) comes from x' = g(x) (x itself when g's cap does not contain x), so V must fill x'. It prints the SAFE
# orbit of x' at solved·L·g. If every such orbit is the one chamber x', then x is fixed by the group that the SAFE group
# of solved·L and all these conjugates g·V·g⁻¹ generate; all of them keep the off geometry of solved·L.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root, once per letter L given as grip:item:
# OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov20_two_level.py 0:12
import sys,json,time,resource,signal
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(7000,7020))
def stop(*a):print('CUT_SHORT_CPU_7000_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul
from sim.kplus import to_tuple
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
R=s.restore.element(M,0,25)
home=s.CL.data.sig_matrix(np.arange(len(reg.orbit_of)))
def image(K,x):
  ki=kp.index_of_matrix(to_tuple(K));assert ki>=0
  return int(reg.piece_of[int(reg.orbit_of[x]),kp.compose(ki,int(reg.transport[x]))])
def start(c,L):
  st=s.sim.State(ctx,menu=M);o=st.apply(s.sim.Twist(ctx,c,L,'menu',{'menu':M.name}),journal=False)
  assert o.applied and not o.uncertain
  return st
for arg in sys.argv[1:]:
  c,i=map(int,arg.split(':'));L=s.restore.element(M,c,i)
  assert kp.index_of_matrix(L)<0
  x=image(matmul(L,R),1474)
  st=start(c,L);off=set(s.B.off_pieces(st).tolist());caps=[]
  for g in range(600):
    cl=st.classify(g,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
    if cl.straddle is None and g!=c:
      n=len(off.intersection(int(p) for p in cl.inside))
      if n:caps.append((g,n))
  print('LEGAL_NONSAFE',json.dumps({'L':[c,i],'x':x,'caps':caps}),flush=True)
  fixed=True
  for g,n in caps:
    inx=bool(home[x,g])
    for j in range(1,len(M.items)):
      K=s.restore.element(M,g,j)
      if kp.index_of_matrix(K)<0:continue
      t0=time.process_time()
      st=start(c,L);o=st.apply(s.sim.Twist(ctx,g,K,'menu',{'menu':M.name}),journal=False)
      assert o.applied and not o.uncertain
      xp=image(K,x) if inx else x
      G=s.B.Alphabet(SimpleNamespace(st=st),M);orb,sizes=G.orbits()
      holes=np.asarray(G.holes);hc=np.bincount(orb[holes],minlength=len(sizes));ox=int(orb[xp])
      rec={'L':[c,i],'g':[g,j],'cap_holds_x':inx,'x':x,'xp':xp,'xp_hole':bool(xp in set(holes.tolist())),
           'off':len(s.B.off_pieces(st)),'safe_letters':len(G.letters),'orbits':len(sizes),
           'xp_orbit_size':int(sizes[ox]),'xp_orbit_holes':int(hc[ox]),'cpu':round(time.process_time()-t0,1)}
      fixed=fixed and rec['xp_orbit_size']==1
      print('CONJUGATE',json.dumps(rec),flush=True)
  print('TWO_LEVEL',json.dumps({'L':[c,i],'x':x,'conjugates':sum(1 for g,_ in caps for j in range(1,len(M.items)) if kp.index_of_matrix(s.restore.element(M,g,j))>=0),'x_fixed':fixed}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
