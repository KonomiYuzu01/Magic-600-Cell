# Coverage round 4 input, the integrator's check of the depth table's classes. It reads the two outputs of
# cov_depth_grip0.py, groups the 48 jumble letters L at grip 0 by their deep chamber sets, and checks exactly that each
# class is one left coset L·T of the twelve lattice items T of grip 0 (L⁻¹·L′ in K⁺ within a class, never across). It
# also checks the group facts that explain the classes: the menu at grip 0 is closed under products, T is the
# stabilizer of pole 0 in K⁺, and T permutes the four cosets off T transitively, each with a stabilizer of order 3.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root after cov_depth_grip0.py:
# OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov_depth_cosets.py
import sys,json
from pathlib import Path
from collections import defaultdict
P=Path('research/jumbling/restoration/12-low-height')
sys.path.insert(0,str((P/'scripts').resolve()))
import screen as s
from exact import matmul,transpose
M=s.restore.menu('I_a');kp=s.CL.ctx.kplus;tt=s.restore.to_tuple
E={i:s.restore.element(M,0,i) for i in range(len(M.items))}
idx={E[i]:i for i in E};assert len(idx)==len(E)
prod={(a,b):idx.get(tt(matmul(E[a],E[b])),-1) for a in E for b in E}
T=[i for i in E if kp.index_of_matrix(E[i])>=0];Ts=set(T)
inK=lambda m:kp.index_of_matrix(m)>=0
print('GROUP',json.dumps({'menu_items':len(E),'closed':all(v>=0 for v in prod.values()),'lattice_items':T,
  'T_subgroup':all(prod[a,b] in Ts for a in T for b in T),
  'T_is_stab_pole0':sorted(kp.index_of_matrix(E[t]) for t in T)==sorted(kp.stab0)}),flush=True)
cos=sorted({tuple(sorted(prod[a,t] for t in T)) for a in E if a not in Ts})
act={t:[cos.index(tuple(sorted(prod[t,a] for a in c))) for c in cos] for t in T}
print('COSETS',json.dumps({'cosets':[list(c) for c in cos],'transitive':sorted({act[t][0] for t in T})==list(range(len(cos))),
  'stabilizers':[[t for t in T if act[t][j]==j] for j in range(len(cos))]}),flush=True)
deep={}
for k in (0,1):
  for l in (P/f'runs/round22/cov_depth_grip0-{k}.txt').read_text().splitlines():
    if l.startswith('DEPTH'):
      r=json.loads(l.split(' ',1)[1]);deep[r['L'][1]]=frozenset(y for y,_ in r['deep_chambers'])
assert len(deep)==48
cls=defaultdict(list)
for i,d in deep.items():cls[d].append(i)
classes=sorted(sorted(v) for v in cls.values())
q=lambda a,b:inK(matmul(transpose(E[a]),E[b]))
print('CLASSES',json.dumps({'classes':classes,'sizes':[len(c) for c in classes],
  'same_coset_within':all(q(a,b) for c in classes for a in c for b in c),
  'same_coset_across':any(q(a,b) for c in classes for d in classes if c!=d for a in c for b in d),
  'classes_are_cosets':classes==[list(c) for c in cos],
  'deep_in_every_class':len(frozenset.intersection(*cls)),'deep_in_some_class':len(frozenset.union(*cls))}),flush=True)
