# Theory round 22, coverage round 8 (COV-52): the outer classes among COV-36's residual cases. Through
# cov36_hole_orbits.py it lists the 16 outer placement classes of COV-23 with their own-class cells (all empty,
# OUTER_CLASSES), the totals over COV-36's rows (class–hole pairs, hole orbits, cases; TOTAL), the rows of the outer
# classes 1345 and 1437 (REMOVED_OUTER) and what remains without them (classes, class–hole pairs, cases;
# DEEP_REMAINDER).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov52_outer_classes.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov52_outer_classes.py
import runpy,contextlib,io,time
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):
 n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov36_hole_orbits.py')
rows=n['rows'];cl=n['cl'];P=n['s'].CL.classes()[0][0]
outer=[(y,cl['cell'](y)) for y,ys in cl['groups'] if not ys]
assert all(not C for y,C in outer)
print('OUTER_CLASSES',len(outer),outer,flush=True)
print('TOTAL',sum(r[2] for r in rows),sum(r[3] for r in rows),sum(r[4] for r in rows),flush=True)
print('REMOVED_OUTER',[r for r in rows if r[0] in (1345,1437)],flush=True)
deep=[r for r in rows if r[0] not in (1345,1437)]
print('DEEP_REMAINDER',len(deep),sum(r[2] for r in deep),sum(r[4] for r in deep),'CPU',round(time.process_time(),2),flush=True)
