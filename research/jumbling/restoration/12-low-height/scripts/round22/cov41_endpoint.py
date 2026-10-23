# Theory round 22, coverage round 6 (COV-41): the endpoint of the two-letter excursion [(14, 12), (35, 13)] at
# COV-17's singleton, and the same-cap counts. After COV-17's word (through cov17_single_off.py) it applies the two
# letters and compares every piece's pose before and after: ENDPOINT_EFFECTS lists the distinct left multipliers with
# their piece counts (index −1 for a matrix outside K⁺). It then counts, at cap 8, the ordered pairs of letters whose
# first letter is a jumble letter and whose product is retained, and the ordered triples whose first letter and first
# two-letter product are jumble and whose product is retained (SAME_CAP_EXCURSIONS).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov41_endpoint.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov41_endpoint.py
import runpy,contextlib,io,time
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov17_single_off.py')
s,M,st,W,np=[n[k] for k in ['s','M','st','W','np']]
from exact import matmul,transpose
from sim.kplus import to_tuple
from collections import Counter
def apply(c,i):
 o=st.apply(s.sim.Twist(s.CL.ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}));assert o.applied and not o.uncertain
for c,i in W:apply(c,i)
pid0=st.pose_id.copy();m0=[st._pose_matrix(i) for i in range(len(st._poses))]
for c,i in [[14,12],[35,13]]:apply(c,i)
effects={}
for a,b in set(zip(pid0.tolist(),st.pose_id.tolist())):
 E=to_tuple(matmul(st._pose_matrix(b),transpose(m0[a])));effects[E]=effects.get(E,0)+int(((pid0==a)&(st.pose_id==b)).sum())
print('ENDPOINT_EFFECTS',sorted((s.CL.ctx.kplus.index_of_matrix(E),count) for E,count in effects.items()),'distinct',len(effects),flush=True)
E=[s.restore.element(M,8,i) for i in range(60)];idx={e:i for i,e in enumerate(E)};mul=np.array([[idx[to_tuple(matmul(a,b))] for b in E] for a in E]);ret=np.array([s.CL.ctx.kplus.index_of_matrix(e)>=0 for e in E]);two=sum(int(ret[mul[j,i]]) for i in range(1,60) if not ret[i] for j in range(1,60));three=sum(int(ret[mul[k,mul[j,i]]]) for i in range(1,60) if not ret[i] for j in range(1,60) if not ret[mul[j,i]] for k in range(1,60))
print('SAME_CAP_EXCURSIONS',two,three,'CPU',round(time.process_time(),1),flush=True)
