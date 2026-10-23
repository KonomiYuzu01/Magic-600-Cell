# Theory round 22, line (d), round 6 (R22D-92): r22d92_guarded_k200.py with a larger CPU budget. The rule of R22D-90
# from the depth 200 stop of seed 7105 (state 44b2e7a0...), with the same rebuild, steps and printed records; the end is
# height 2, stuck, 12 kept steps, or a budget stop past 2,000 CPU seconds (the setup counted), after undoing the
# unfinished word.
# A copy of r22d92_guarded_k200.py, the senior reviewer's exact computation, with two numbers changed and the rest kept
# as written apart from this header: the CPU limit is 2,390/2,400 seconds instead of 1,190/1,200, and the checkpoint
# taken from r22d75_height_first.py is 2,000 CPU seconds instead of 1,000 (one more replace on that source). The
# integrator ran it and got the output in runs/round22/r22d92_guarded_k200_2k.txt: the reviewer's 11 steps, line for
# line apart from the cpu fields, then a twelfth, and the end at the cap of 12 steps. About 23 CPU minutes.
# It writes no file; it computes the home centroids of theory/seams.py in memory instead of reading its cache.
# Research only, headless Linux; exact geometry, apart from cover's float side test (R22D-90).
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d92_guarded_k200_2k.py
import sys,json,resource,time,ast
from pathlib import Path
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(2390,2400)); T=time.process_time()
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import seams
C=E.sim.get_context(); M=E.R.menu('I_a'); s=E.sim.State(C,filtered=False,menu=M)
rec=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round3/ia-k200-s7105.json').read_text())
for c,j in rec['history']+rec['moves']:
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
assert s.off_lattice_count()==rec['off_now']
for c,j in [(237,14),(56,13),(311,13),(363,12),(486,13)]:
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
assert s.digest()=='44b2e7a06388c04f18b974dad6e4b94a09ff24a8864eefab7cb8b38e89eb6ef5'
source=Path('research/jumbling/restoration/12-low-height/scripts/round22/r22d75_height_first.py').read_text()
source=source[source.index('br=['):source.index("report('START'); steps=0")].replace('for tier in range(3):','for tier in range(2):').replace('time.process_time()-T>1000','time.process_time()-T>2000')
exec(compile(source,'<height-first-first-two-families>','exec'),globals())
def home():
 if seams._HOME is None:
  seams._HOME=np.array([[seams.wj.to_float(v) for v in seams.wj.centroid(C,p)] for p in range(seams.rim.NP)])
 return seams._HOME
seams.home_centroids=home
class Probe(seams.SeamRestorer):
 def scores(self,c):checktime(); return super().scores(c)
 def apply(self,c,j):checktime(); return super().apply(c,j)
 def undo(self):
  super().undo()
 def inside(self,c):checktime(); return super().inside(c)
def blocked():
 n=0
 for c in range(600):
  cl=s.classify(c,need_inside=False,stop_at_straddle=True); assert not cl.uncertain
  n+=cl.straddle is not None
 return n
def output(tag,**kw):
 print(tag,json.dumps(dict(kw,n2_n4_n6=stats()[::-1],off=int(s.off_lattice_count()),N=E.R.Restorer(s,M).misalignment(),blocked=blocked(),digest=s.digest(),cpu=round(time.process_time()-T,2))),flush=True)
output('START')
accepted=0; rejected_n4=0; rejected_other=0; R=None
try:
 while accepted<12:
  checktime()
  if stats()[:2]==[0,0]:output('END',reason='height2',steps=accepted,rejected_n4=rejected_n4); break
  before=s.digest(); phi=stats(); N0=E.R.Restorer(s,M).misalignment()
  a,pred,ct=nextletter()
  if a is not None:
   c,j=a; o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
   assert stats()==pred; accepted+=1
   output('STEP',step=accepted,source='height6' if pred[0]<phi[0] else 'height4',word=[list(a)],before=before,checks=ct)
   continue
  print('HEIGHT_EXHAUSTED',json.dumps(ct),flush=True)
  s.journal=[]; R=Probe(s,M)
  rules={'seam':R.seam,'cover':R.cover,'close':R.level1,'conj':R.conj,'lock':R.level2,'improve':R.improve,'shift':R.shift,'reopen':R.reopen}
  kept=False
  for rule in R.order:
   print('TRY',json.dumps({'rule':rule,'cpu':round(time.process_time()-T,2)}),flush=True)
   done=rules[rule]()
   if done is None:
    assert not R.moves and s.digest()==before
    continue
   h=heights(); q=[int((h==k).sum()) for k in (6,4,2)]; N1=R.misalignment()
   assert N1<N0
   if int(h.max())>6 or q[0]>phi[0] or q[1]>phi[1]:
    rejected_n4+=q[1]>phi[1]; rejected_other+=int(h.max())>6 or q[0]>phi[0]
    print('REJECT',json.dumps({'rule':rule,'word':done,'n2_n4_n6':q[::-1],'max_height':int(h.max()),'N':N1,'before':before,'trial_digest':s.digest()}),flush=True)
    while R.moves:R.undo()
    assert s.digest()==before
    continue
   accepted+=1; kept=True
   output('STEP',step=accepted,source=rule,word=done,before=before)
   break
  if not kept:output('END',reason='stuck',steps=accepted,rejected_n4=rejected_n4,rejected_other=rejected_other); break
  R=None
 else:output('END',reason='step-budget',steps=accepted,rejected_n4=rejected_n4,rejected_other=rejected_other)
except Budget:
 if R is not None:
  while R.moves:R.undo()
 output('END',reason='cpu-budget',steps=accepted,rejected_n4=rejected_n4,rejected_other=rejected_other)
