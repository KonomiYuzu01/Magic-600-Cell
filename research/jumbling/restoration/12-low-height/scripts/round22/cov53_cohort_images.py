# Theory round 22, coverage round 8 (COV-53): the outer placements and the images of the first jumble's cohorts under
# the nonlattice cosets. Through the head of cov29_two_cap.py it gives, for the pieces 1345 and 1437 at
# R = element(I_a, 0, 45), the chamber type, the own-class caps that contain the piece, all caps that contain it and
# its pose tree (OUTER_AUDIT); it picks one grip-0 jumble item from each left coset of K⁺ that the jumble items meet
# (LEFT_COSET_REPS, asserting there are four) and, for each, takes the 29 rigid blocks of the 3,097 off pieces after
# J = (0, 12) and splits each block into rigid components under that item (BLOCK_IMAGES: the blocks that split, the
# smallest component and the number of components), under a 900-second CPU cutoff.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov53_cohort_images.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov53_cohort_images.py
from pathlib import Path
import time,collections,resource,signal
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/cov29_two_cap.py');n={}
exec(compile(p.read_text().split('pairs=')[0],str(p),'exec'),n)
resource.setrlimit(resource.RLIMIT_CPU,(900,905))
signal.signal(signal.SIGXCPU,lambda *a:(print('CUT_SHORT_900_CPU',flush=True),exit(124)))
s,np,M,kp,reg,home=[n[k] for k in ('s','np','M','kp','reg','home')]
from exact import matmul,transpose
from sim.kplus import to_tuple
P=s.CL.classes()[0][0];R=s.restore.element(M,0,45)
for y in (1345,1437):
 sides=n['labels'](n['im'](R),y)
 print('OUTER_AUDIT',y,'type',int(reg.orbit_of[y]),'own_class_inside',[c for c in P if sides[c]==1],'inside',np.flatnonzero(sides==1).tolist(),'tree',s.T.pose_tree(R),flush=True)
C=np.flatnonzero(home[:,0]);J=s.restore.element(M,0,12)
labels=np.array([n['labels'](n['im'](J),int(q)) for q in C],np.uint8)
blocks=s.BL.rigid_components(C,labels)
reps=[]
for i in range(12,60):
 E=s.restore.element(M,0,i)
 if all(kp.index_of_matrix(to_tuple(matmul(E,transpose(Q))))<0 for _,Q in reps):reps.append((i,E))
assert len(reps)==4
print('LEFT_COSET_REPS',[i for i,E in reps],flush=True)
for i,E in reps:
 ls=np.array([n['labels'](n['im'](E),int(q)) for q in C],np.uint8)
 rows=[(int(B[0]),len(B),[len(b) for b in s.BL.rigid_components(B,ls[np.searchsorted(C,B)])]) for B in blocks]
 print('BLOCK_IMAGES',i,'split',[r for r in rows if len(r[2])!=1],'min',min(min(r[2]) for r in rows),'components',sum(len(r[2]) for r in rows),'CPU',round(time.process_time(),2),flush=True)
print('COMPLETE',round(time.process_time(),2),flush=True)
