# Theory round 22, coverage round 5 (COV-33, at COV-17's singleton): replays COV-17's 24-letter word (through
# cov17_single_off.py) to the state whose only off piece is 58388, then, for every unstraddled cap containing it and
# every landing item, tests the prepared vacancy criterion for a singleton (the SAFE orbit of the hole must meet the
# resource chambers of the landing chamber's reference orbit outside cap 0). It prints the eligible caps with their
# passing and failing items (SINGLETON_CAP_COUNT).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov33_singleton_caps.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov33_singleton_caps.py
import runpy,contextlib,io,time,json
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):
 n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov23_singleton_classes.py')
 q=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov17_single_off.py')
s,np,M,kp,reg,ref,home,frames,tnorm=[n[k] for k in ['s','np','M','kp','reg','G','home','frames','tnorm']]
from types import SimpleNamespace
from exact import matmul
from sim.kplus import to_tuple
st=q['st']
for c,i in q['W']:
 out=st.apply(s.sim.Twist(s.CL.ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}),journal=False);assert out.applied and not out.uncertain
assert s.B.off_pieces(st).tolist()==[58388]
p=58388;h=int(s.B.hole_set(st)[0]);t=int(reg.orbit_of[p]);pose=st.pose(p)
G0=s.B.Alphabet(SimpleNamespace(st=st),M);o0,z0=G0.orbits();ro,rz=ref.orbits();horb=set(np.flatnonzero(o0==o0[h]).tolist());rows=[]
for c in range(600):
 cl=st.classify(c,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
 if cl.straddle is not None or p not in cl.inside:continue
 yes=[];no=[]
 for i in range(12,60):
  k=kp.index_of_matrix(to_tuple(matmul(s.restore.element(M,c,i),pose)))
  if k<0:continue
  x=int(reg.piece_of[t,kp.compose(k,int(reg.transport[p]))]);j=kp.compose(frames[c],tnorm[i]);y=int(reg.piece_of[t,kp.compose(int(kp.inv[j]),int(reg.transport[x]))])
  out=np.flatnonzero((ro==ro[y])&(~home[:,0]))
  resources=reg.piece_of[t,kp.compose_vec(np.full(len(out),j),reg.transport[out])]
  (yes if horb.intersection(resources.tolist()) else no).append(i)
 if yes or no:rows.append([c,yes,no])
print('SINGLETON_CAP_COUNT',st.digest(),'hole_orbit',len(horb),'eligible',len(rows),'passing',sum(bool(r[1]) for r in rows),'rows',rows,'CPU',round(time.process_time(),1),flush=True)
