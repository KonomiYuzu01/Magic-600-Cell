# Theory round 22, coverage round 6 (COV-37): the vacant-lock condition VL of COV-31 at the reachable 737 table of the
# line (a) replay. Through the common setup of cov33_whole_cap_737.py (its code before the eligibility loop, which
# loads runs/round22/line-a/replay/tables/737.json.gz) it takes the intrinsic rigid components of the off pieces, with
# I_B (the caps holding every piece of the block) and S_B (the caps a piece of it straddles), and lists the chambers
# locked to each block that are occupied. VL_737 gives the table's digest and, for each violating block, its index,
# size, first piece, I_B, S_B and the occupied locked chambers.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov37_vl_737.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov37_vl_737.py
from pathlib import Path
import time,json
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/cov33_whole_cap_737.py')
n={};exec(compile(p.read_text().split('ct=Counter();eligible=[]')[0],str(p),'exec'),n)
s,np,st,off,ls=[n[k] for k in ['s','np','st','off','ls']]
home=s.CL.data.sig_matrix(np.arange(len(st.pose_id)));H=set(s.B.hole_set(st));rows=[]
for b,ps in enumerate(s.BL.rigid_components(off,ls)):
 z=ls[np.searchsorted(off,ps)];ins=(z==1).all(axis=0);out=(z==0).all(axis=0);strad=(z==2).any(axis=0);assert (ins|out|strad).all()
 locks=np.flatnonzero(home[:,ins].all(axis=1)&(~home[:,out]).all(axis=1));occupied=[int(x) for x in locks if x not in H]
 if occupied:rows.append([b,len(ps),int(ps[0]),np.flatnonzero(ins).tolist(),np.flatnonzero(strad).tolist(),occupied])
print('VL_737',st.digest(),'violations',json.dumps(rows),'CPU',round(time.process_time(),2),flush=True)
