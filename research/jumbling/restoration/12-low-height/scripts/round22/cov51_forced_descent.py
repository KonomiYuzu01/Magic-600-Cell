# Theory round 22, coverage round 7 (COV-51): the adjudicating experiment for the singleton residual, second form. It
# runs COV-43's script (cov43_residual_search.py) with its worker replaced in memory: for each of the 118 cases, trial 0
# runs the existing Restorer (close, lock, improve, shift) from the singleton table for at most 48 descent letters, and
# each later trial first applies one to four legal jumble letters (items 12 to 59, 80% of them at grips local to y and
# h) and then the same descent; at a lattice endpoint it replays the whole word (at most 54 letters) exactly, records
# the largest height on the way, checks that the inverse word returns the start digest and prints FOUND. Each worker
# keeps COV-43's 120-second CPU cutoff (CUT_SHORT); two run at a time. ALL_PASS, a FOUND for every case, would settle
# SingletonResidual (at most 7,253 letters per residual singleton once SAFE routing of the hole is added, and COV-06
# carries a certificate to every labelled table of its case); a FOUND settles its case; CUT_SHORT, NO_HIT, MISSING, a
# descent stall or an error decides nothing.
# The senior reviewer's experiment design, kept as written apart from this header. The integrator started it once,
# stopped it after eight cases and kept the partial output in runs/round22/cov51_forced_descent.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov51_forced_descent.py
import ast
from pathlib import Path
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/cov43_residual_search.py')
doc=ast.parse(p.read_text())
a=next(n for n in doc.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='worker' for t in n.targets))
head=ast.literal_eval(a.value).split('for trial in range(1000000):')[0]
body=r'''
for trial in range(1000000):
 st=clone(root);prefix=[]
 for depth in range(0 if trial==0 else 1+(trial-1)%4):
  for attempt in range(80):
   c=rng.choice(local) if rng.random()<0.8 else rng.randrange(600)
   i=rng.randrange(12,60);out=apply(st,c,i)
   if out.applied and not out.uncertain:
    prefix.append([c,i]);break
  else:break
 restorer=s.restore.Restorer(st,M)
 restorer.run(max_steps=48)
 if not st.is_lattice():continue
 word=prefix+restorer.moves
 assert len(word)<=54
 check=clone(root);height=2
 for c,i in word:
  out=apply(check,c,i);assert out.applied and not out.uncertain
  height=max(height,max(s.T.dist(s.T.factors(check._pose_matrix(int(pid)))[0]) for pid in np.unique(check.pose_id)))
 assert check.is_lattice();end=check.digest()
 for c,i in reversed(word):
  out=apply(check,c,inv[i]);assert out.applied and not out.uncertain
 assert check.digest()==root_digest
 print('FOUND',json.dumps({'y':y,'h':h,'word':word,'length':len(word),'max_height':height,'start':root_digest,'end':end,'inverse_verified':True}),flush=True)
 raise SystemExit(0)
print('NO_HIT',json.dumps({'y':y,'h':h,'trials':1000000}),flush=True)
raise SystemExit(2)
'''
compile(head+body,'<proposed-worker>','exec')
a.value=ast.Constant(head+body);ast.fix_missing_locations(doc)
exec(compile(doc,str(p),'exec'))
