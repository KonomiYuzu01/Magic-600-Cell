# Theory round 22, coverage round 4 (COV-26): three valid singleton configurations besides COV-01's X. With every other
# piece at identity, piece 2249 (type 19, the triangle cell {0, 2, 23} with home chambers) and pieces 1345 and 1437
# (type 7, outer placements without a landing letter), each at R = element(I_a, 0, 45), have every vertex of their
# off region on or beyond one body facet (3, 7 and 7), so the off piece meets no lattice piece's interior (EXTERIOR);
# it also checks that R has left height 2. Their reachability is not claimed.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov26_exterior_singletons.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov26_exterior_singletons.py
import runpy,time
n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov01_isolated_triangle.py')
s,M=n['s'],n['M']
from exact import transpose,matvec,dot
from sim.kplus import to_tuple
R=to_tuple(transpose(s.restore.element(M,0,12)))
assert s.T.dist(s.T.factors(R)[0])==2
for p,f in [(2249,3),(1345,7),(1437,7)]:
 assert all((dot(s.CL.data.N[f],matvec(R,v))-s.CL.data.NN).sign()>=0 for v in s.CL.ctx.regions.vertices(p))
 print('EXTERIOR',{'piece':p,'type':int(s.CL.ctx.regions.orbit_of[p]),'pose':[0,45],'separating_body_facet':f},flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
