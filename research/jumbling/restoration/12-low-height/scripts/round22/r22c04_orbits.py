# Theory round 22, R22C-04 (line (c)): K+ orbits of the home cells of each kind, and the type-0 chambers.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c04_orbits.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22c04_orbits.py
import sys,json,resource
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(300,300))
sys.path.insert(0,str(Path('research/jumbling/theory').resolve()))
import classes as CL
import numpy as np
members,cls=CL.classes(); home=CL.data.sig_matrix(np.arange(len(CL.ctx.regions.orbit_of)))
for kind in range(1,5):
 p=int(np.flatnonzero(home[:,members[0]].sum(axis=1)==kind)[0]); cell=np.asarray(members[0])[home[p,members[0]]]
 images={(int(cls[perm[members[0][0]]]),tuple(sorted(map(int,perm[cell])))) for perm in CL.ctx.kplus.perms}
 print('kind',kind,'seed',cell.tolist(),'Kplus_home_orbit',len(images))
type0=np.flatnonzero(np.asarray(CL.ctx.regions.orbit_of)==0)
assert (home[type0].sum(axis=1)==1).all()
print('all_type0_have_one_cap',len(type0))
