# Theory round 22, coverage round 7 (COV-46): the intrinsic rigid blocks after the first jumble letter. Through the
# setup of cov29_two_cap.py (its code before the pair loop) it takes the 3,097 pieces of cap 0, their sides at the 600
# caps after the jumble letter J = (0, 12), and their intrinsic rigid components (blocks.rigid_components).
# FIRST_JUMBLE_BLOCKS gives the number of blocks, the total, the sizes with their counts and the smallest size.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov46_first_jumble_blocks.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov46_first_jumble_blocks.py
from pathlib import Path
import time,collections
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/cov29_two_cap.py')
n={};exec(compile(p.read_text().split('pairs=')[0],str(p),'exec'),n)
s,np,M,home=n['s'],n['np'],n['M'],n['home'];C=np.flatnonzero(home[:,0]);J=s.restore.element(M,0,12);ji=n['im'](J)
ls=np.array([n['labels'](ji,int(q)) for q in C],np.uint8)
blocks=s.BL.rigid_components(C,ls)
print('FIRST_JUMBLE_BLOCKS',len(blocks),'TOTAL',sum(map(len,blocks)),'SIZES',sorted(collections.Counter(map(len,blocks)).items()),'MIN',min(map(len,blocks)),'CPU',round(time.process_time(),1),flush=True)
