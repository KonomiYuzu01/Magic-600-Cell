# Theory round 22, line (d), round 6 (R22D-93, R22D-94): at the seed 7102 stop after R22D-16 and R22D-43 (state
# f0290f93..., rebuilt from the census record round2/ia-k100-s7102.json and the words of r22d16_step.py and
# r22d43_twelve.py, every twist admissible and certain), the first letter of the first two families of R22D-75 (the code
# of r22d75_height_first.py without its third family), asserted to be (61, 15), with its prediction and pruning counts;
# the state after (61, 15), improve's first word there (played, printed with the state it reaches, and undone), and the
# state after (61, 44), asserted to be f0290f93... again.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d93_cycle.txt, line for line the reviewer's apart from the cpu fields. About 2.5 CPU
# minutes. It has no checkpoint of its own: past 1,000 CPU seconds the budget check of the scan, taken from
# r22d75_height_first.py, would end it with an uncaught exception.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d93_cycle.py
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
for c,j in rec['history']+rec['moves']+literal_word('r22d16_step.py')+literal_word('r22d43_twelve.py'):
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
assert s.digest()=='f0290f93bea0295ecc00c2a42f725abfd59d3c37732834bb82f5da41c38fa793'
from collections import Counter
src=Path('research/jumbling/restoration/12-low-height/scripts/round22/r22d75_height_first.py').read_text()
exec(compile(src[src.index('br=['):src.index("report('START'); steps=0")].replace('for tier in range(3):','for tier in range(2):'),'<height-first-first-two-families>','exec'),globals())
report('START')
a,pred,ct=nextletter()
assert tuple(a)==(61,15)
print('FIRST_HEIGHT_LETTER',json.dumps(dict(letter=a,prediction_n6_n4_n2=pred,checks=ct)),flush=True)
for c,j in [a,(61,44)]:
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
 report('LETTER',letter=[c,j])
 if (c,j)==(61,15):
  assert stats()==pred
  r=E.R.Restorer(s,M); before=s.digest(); w=r.improve()
  if w is not None:
   report('IMPROVE_OFFER',word=w)
   while r.moves:r.undo()
   assert s.digest()==before
  else:print('IMPROVE_OFFER_NONE',flush=True)
assert s.digest()=='f0290f93bea0295ecc00c2a42f725abfd59d3c37732834bb82f5da41c38fa793'
print('CYCLE_VERIFIED')
