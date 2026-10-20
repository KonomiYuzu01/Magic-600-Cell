# Theory round 22, line (d), round 6 (R22D-91, R22D-93, R22D-94): at the seed 7102 stop after R22D-16, R22D-43 and the
# height-first letter (61, 15) (state c450bb2f..., rebuilt from the census record round2/ia-k100-s7102.json, the words
# of r22d16_step.py and r22d43_twelve.py and (61, 15), every twist admissible and certain), up to three steps of phase U
# without a guard: the rules of one theory/seams.py SeamRestorer in order, the first word offered kept, with the counts
# at heights 2, 4 and 6, the largest height, the lattice pieces taken off and the largest rise after each of its
# letters. After the first step that puts a piece above height 6 or raises n6 or n4, the word is undone and the later
# rules of a fresh SeamRestorer are tried with that guard (GUARD_TRY, GUARD_REJECT, GUARDED_STEP or GUARDED_END), then
# the word is played again. Past 950 CPU seconds (the setup counted) it undoes any unfinished word and reports a budget
# stop.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d94_height2_probe.txt, line for line the reviewer's apart from the cpu fields: one step,
# cover (61, 44), close and conj tried with the guard, then the budget stop in lock. About 16 CPU minutes.
# It writes no file; it computes the home centroids of theory/seams.py in memory instead of reading its cache.
# Research only, headless Linux; exact geometry, apart from cover's float side test (R22D-90).
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d94_height2_probe.py
import sys,json,resource,time,ast
from pathlib import Path
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200)); T=time.process_time()
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import seams
C=E.sim.get_context(); M=E.R.menu('I_a'); s=E.sim.State(C,filtered=False,menu=M)
rec=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round2/ia-k100-s7102.json').read_text())
def literal_word(name):
 t=ast.parse(Path('research/jumbling/restoration/12-low-height/scripts/round22/'+name).read_text())
 return next(ast.literal_eval(n.value) for n in t.body if isinstance(n,ast.Assign) and any(isinstance(v,ast.Name) and v.id=='word' for v in n.targets))
for c,j in rec['history']+rec['moves']+literal_word('r22d16_step.py')+literal_word('r22d43_twelve.py')+[[61,15]]:
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
assert s.digest()=='c450bb2f41b5fb4cde5beefe76c48926577f3fc2f216f28976fa8a11981fdf5f'
cache={}
def heights():
 hs=[]
 for p in s._poses:
  if p.kidx>=0:hs.append(0);continue
  if p.key not in cache:
   l,r=E.T.factors(p.matrix); assert E.T.dist(r)==0
   cache[p.key]=E.T.dist(l)
  hs.append(cache[p.key])
 return np.array(hs)[s.pose_id]
def summary():
 h=heights(); blocked=0
 for c in range(600):
  cl=s.classify(c,need_inside=False,stop_at_straddle=True); assert not cl.uncertain
  blocked+=cl.straddle is not None
 return dict(n2_n4_n6=[int((h==k).sum()) for k in (2,4,6)],max_height=int(h.max()),off=int(s.off_lattice_count()),N=E.R.Restorer(s,M).misalignment(),blocked=blocked,digest=s.digest(),cpu=round(time.process_time()-T,2))
def emit(tag,**kw):print(tag,json.dumps(dict(kw,**summary())),flush=True)
def home():
 if seams._HOME is None:
  seams._HOME=np.array([[seams.wj.to_float(v) for v in seams.wj.centroid(C,p)] for p in range(seams.rim.NP)])
 return seams._HOME
seams.home_centroids=home
class Budget(Exception):pass
def check():
 if time.process_time()-T>950:raise Budget
class Probe(seams.SeamRestorer):
 def scores(self,c):check();return super().scores(c)
 def inside(self,c):check();return super().inside(c)
 def apply(self,c,j):check();return super().apply(c,j)
def methods(R):
 return {'seam':R.seam,'cover':R.cover,'close':R.level1,'conj':R.conj,'lock':R.level2,'improve':R.improve,'shift':R.shift,'reopen':R.reopen}
emit('START')
R=Probe(s,M); accepted=0; baseline=0; G=None; raw=None; guard_tested=False
try:
 for step in range(1,4):
  check(); baseline=len(R.moves); before=s.digest(); h0=heights(); N0=R.misalignment(); word=None
  for rule in R.order:
   print('TRY',json.dumps({'step':step,'rule':rule,'cpu':round(time.process_time()-T,2)}),flush=True)
   word=methods(R)[rule]()
   if word is not None:break
  if word is None:emit('END',reason='stuck',steps=accepted);break
  assert R.misalignment()<N0
  while len(R.moves)>baseline:R.undo()
  assert s.digest()==before
  prefixes=[]
  for c,j in word:
   R.apply(c,j); h=heights()
   prefixes.append(dict(letter=[int(c),int(j)],n2_n4_n6=[int((h==k).sum()) for k in (2,4,6)],max_height=int(h.max()),initial_lattice_now_off=int(((h0==0)&(h>0)).sum()),max_rise=int((h-h0).max())))
  accepted+=1; emit('STEP',step=step,source=rule,word=word,before=before,prefixes=prefixes)
  h=heights()
  if not guard_tested and ((h>6).any() or (h==6).sum()>(h0==6).sum() or (h==4).sum()>(h0==4).sum()):
   guard_tested=True; raw=list(word)
   while len(R.moves)>baseline:R.undo()
   assert s.digest()==before
   G=Probe(s,M); rejected=1; found=False
   try:
    for later in list(R.order)[list(R.order).index(rule)+1:]:
     print('GUARD_TRY',json.dumps({'rule':later,'cpu':round(time.process_time()-T,2)}),flush=True)
     w=methods(G)[later]()
     if w is None:continue
     q=heights()
     if q.max()<=6 and (q==6).sum()<=(h0==6).sum() and (q==4).sum()<=(h0==4).sum():
      found=True;emit('GUARDED_STEP',source=later,word=w,before=before,rejected=rejected)
     else:
      rejected+=1;emit('GUARD_REJECT',source=later,word=w,before=before)
     while G.moves:G.undo()
     assert s.digest()==before
     if found:break
    if not found:emit('GUARDED_END',reason='no-offered-step',rejected=rejected)
   finally:
    while G.moves:G.undo()
    assert s.digest()==before
    for c,j in raw:E.R.Restorer.apply(R,c,j)
    G=None;raw=None;baseline=len(R.moves)
  baseline=len(R.moves)
 else:emit('END',reason='three-step-budget',steps=accepted,guard_tested=guard_tested)
except Budget:
 if G is not None:
  while G.moves:G.undo()
 if raw is None:
  while len(R.moves)>baseline:R.undo()
 emit('END',reason='cpu-budget',steps=accepted,guard_tested=guard_tested)
