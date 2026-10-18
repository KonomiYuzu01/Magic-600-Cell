# Theory round 22, coverage round 3 (COV-17): a legal 24-letter word from the solved state that reaches exactly one
# off piece (58388, type 17, left height 2, class 0, triangle cell {0, 32, 45}; its only hole is 58388). The word is
# J·[[[A,B],B],D]·J⁻¹ with A = (74, 8), B = (188, 4), D = (178, 1), J = (0, 12); every middle letter is checked SAFE,
# every prefix is applied exactly, and the inverse word returns the solved digest. It does not reach COV-01's state.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov17_single_off.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov17_single_off.py
import sys,json,time,resource,signal
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(1700,1720))
def stop(*a):print('CUT_SHORT_CPU_1700_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,split_pairs as SP,numpy as np
from exact import transpose
from sim.kplus import to_tuple,matrix_json
M=s.restore.menu('I_a');st=s.sim.State(s.CL.ctx,menu=M);solved=st.digest()
W=[[0,12],[74,8],[188,4],[74,4],[188,8],[188,4],[188,4],[74,8],[188,8],[74,4],[188,8],[178,1],[188,4],[74,8],[188,4],[74,4],[188,8],[188,8],[188,4],[74,8],[188,8],[74,4],[178,1],[0,45]]
for j,(c,i) in enumerate(W):
 if 0<j<len(W)-1:
  cl=st.classify(c,need_inside=True,stop_at_straddle=False)
  assert not cl.uncertain and cl.straddle is None and st.lattice_flags()[cl.inside].all()
  assert s.CL.ctx.kplus.index_of_matrix(s.restore.element(M,c,i))>=0
 o=st.apply(s.sim.Twist(s.CL.ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
 if j==0:fp=s.B.off_fingerprint(st)
 if 0<j<len(W)-1:assert s.B.off_fingerprint(st)==fp
off=s.B.off_pieces(st);assert off.tolist()==[58388];p=int(off[0]);members,cls=s.CL.classes();h=s.CL.hub(st.pose(p),members,cls);assert len(h)==1;P=h[0][1]
cell=[c for c in members[P] if SP.side(st.pose(p),p,c)=='in'];height=int(s.T.dist(s.T.factors(st.pose(p))[0]));assert height==2
print('SINGLE_OFF_EXACT',json.dumps({'word':W,'length':len(W),'off':[p],'type':int(s.CL.ctx.regions.orbit_of[p]),'holes':s.B.hole_set(st).tolist(),'class':P,'cell':cell,'height':height,'pose':matrix_json(st.pose(p)),'digest':st.digest(),'moved_pieces':st.moved_count(),'all_middle_letters_SAFE':True}),flush=True)
for c,i in reversed(W):
 inv=next(j for j in range(60) if s.restore.element(M,c,j)==to_tuple(transpose(s.restore.element(M,c,i))))
 o=st.apply(s.sim.Twist(s.CL.ctx,c,s.restore.element(M,c,inv),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
assert st.digest()==solved
print('INVERSE_REPLAY',{'letters':len(W),'returns_exact_solved':True},flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
