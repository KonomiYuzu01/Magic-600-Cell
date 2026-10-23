# Theory round 22, coverage round 7 (COV-45): the vacant-lock condition VL fails at a reachable singleton. Through
# cov18_singleton_kinds.py it takes COV-18's vertex example at Y = solved·(0, 12): the off piece at placement 2335 with
# the pose R = (0, 45), whose only hole after J·V·J⁻¹ is 809 (V the SAFE three-cycle (2335 809 813)). It takes the
# sides of the 600 caps for that placement (I inside, S straddling, O outside), lists the chambers locked to the piece
# (home caps containing all of I and none of O), checks again the global Jordan certificate of the SAFE orbit of 2335
# (its prime and its 400-letter word) and asserts that 809 and 813 are not locked. SINGLETON_VL_COUNTEREXAMPLE gives I,
# S, the locked chambers with their types, the orbit size and the certificate.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov45_singleton_vl.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov45_singleton_vl.py
import runpy,contextlib,io,time
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):
 n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov18_singleton_kinds.py')
s,np,G=n['s'],n['np'],n['G'];R=n['Ji'];y=2335
import split_pairs as SP
sides=np.array([SP.side(R,y,c) for c in range(600)])
I=np.flatnonzero(sides=='in');S=np.flatnonzero(sides=='straddle');O=np.flatnonzero(sides=='out')
candidates=np.flatnonzero(G.home[:,I[0]])
locks=candidates[G.home[candidates][:,I].all(axis=1)&(~G.home[candidates][:,O]).all(axis=1)]
o=int(G.orbit[y]);v=n['cert']['certificates'][str(o)]
assert s.B.check_jordan(G,v['word'],o,v['prime'])['verified']
assert 809 not in locks and 813 not in locks
print('SINGLETON_VL_COUNTEREXAMPLE','placement',y,'pose',[0,45],'hole',809,'type',int(G.type[y]),'I',I.tolist(),'S',S.tolist(),'locked',locks.tolist(),'lock_types',G.type[locks].tolist(),'orbit',int(G.orbit_size[o]),'prime',v['prime'],'certificate_letters',len(v['word']),'CPU',round(time.process_time(),1),flush=True)
