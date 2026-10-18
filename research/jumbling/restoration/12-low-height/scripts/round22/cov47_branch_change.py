# Theory round 22, coverage round 7 (COV-47): a residual piece changes its tree branch inside U₄. It builds the table
# of the case (y, h) = (2149, 41) of COV-36 (the off piece 2149 at R = element(I_a, 0, 45), lattice piece 41 moved
# into chamber 2149 by the K⁺ transport, hole 41), applies the word [(1, 12), (41, 16)] exactly and records the
# largest left height over all pieces after each letter. REPLAY gives the heights, the tree data (trees.pose_tree) of
# piece 2149 before and after, the K⁺ index of its accumulated multiplier, the off count and both digests;
# EXACT_INVERSE checks that undoing the word returns the start digest.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov47_branch_change.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov47_branch_change.py
import sys,resource,time
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(120,125))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul,transpose
from sim.kplus import to_tuple
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
st=s.sim.State(ctx,menu=M);y,h=2149,41;R=s.restore.element(M,0,45)
st.pose_id[y]=st._intern_matrix(R)
k=kp.compose(int(reg.transport[y]),int(kp.inv[int(reg.transport[h])]))
st.pose_id[h]=st._intern_k(k);st._compact()
assert s.B.off_pieces(st).tolist()==[y] and s.B.hole_set(st).tolist()==[h]
start=st.digest();before=st.pose(y);height=[2];w=[(1,12),(41,16)]
for c,i in w:
 o=st.apply(s.sim.Twist(ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}));assert o.applied and not o.uncertain
 height.append(max(s.T.dist(s.T.factors(st._pose_matrix(int(pid)))[0]) for pid in np.unique(st.pose_id)))
K=kp.index_of_matrix(to_tuple(matmul(st.pose(y),transpose(before))))
print('REPLAY',w,'height',height,'tree',s.T.pose_tree(before),s.T.pose_tree(st.pose(y)),'K',K,'off',st.off_lattice_count(),'start',start,'end',st.digest(),flush=True)
for _ in w:st.undo()
assert st.digest()==start
print('EXACT_INVERSE',True,'CPU',round(time.process_time(),1),flush=True)
