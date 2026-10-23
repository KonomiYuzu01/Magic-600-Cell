# Theory round 22, coverage round 8 (COV-55): the existing descent at COV-17's reachable singleton. Through the head
# of cov17_single_off.py it reaches the singleton (piece 58388 off, hole 58388) with COV-17's 24-letter word, then
# gives the misalignment N, the grip misalignment and the grip survey with the distribution of scores after one letter
# (N, GRIP, SURVEY), runs the rules level1 (close), level2 (lock), improve and shift of
# research/jumbling/theory/restore.py in turn, each with its CPU time and whether the table is unchanged, stopping at
# the first rule that moves (RULE), and reports the end state (COMPLETE), under a 900-second CPU cutoff.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov55_descent_cov17.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov55_descent_cov17.py
from pathlib import Path
import time,resource,signal,collections
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/cov17_single_off.py')
n={};exec(compile(p.read_text().split('for c,i in reversed(W):')[0],str(p),'exec'),n)
resource.setrlimit(resource.RLIMIT_CPU,(900,905))
signal.signal(signal.SIGXCPU,lambda *a:(print('CUT_SHORT_900_CPU',flush=True),exit(124)))
s,M,st,np=[n[k] for k in ('s','M','st','np')];r=s.restore.Restorer(st,M);d=st.digest();t=time.process_time()
print('N',r.misalignment(),'GRIP',r.grip_misalignment(),'SURVEY',{c:[a,sorted(collections.Counter(b.values()).items()),[i for i,v in b.items() if v<a]] for c,(a,b) in r.survey().items()},flush=True)
for rule in ('level1','level2','improve','shift'):
 a=time.process_time();out=getattr(r,rule)();print('RULE',rule,out,'cpu',round(time.process_time()-a,2),'unchanged',st.digest()==d,flush=True)
 if out is not None:break
print('COMPLETE','lattice',st.is_lattice(),'off',st.off_lattice_count(),'moves',r.moves,'cpu',round(time.process_time()-t,2),flush=True)
