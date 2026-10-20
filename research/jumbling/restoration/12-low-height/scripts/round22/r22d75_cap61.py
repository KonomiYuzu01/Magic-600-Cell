# Theory round 22, line (d), after round 5: what the letter (61, 15) of the height-first rule does to cap 61 at the
# seed 7102 stop after R22D-43 (state f0290f93..., rebuilt by running r22d43_twelve.py, whose output comes first): the
# cap's inside pieces by left height and by lattice, own branch or other branch before the letter, then the left
# heights of the same pieces after it, with the end digest.
# The integrator's script; output in runs/round22/r22d75_cap61.txt. About a CPU minute, most of it for
# r22d43_twelve.py. It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless
# JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d75_cap61.py
import sys,json,runpy
from collections import Counter
import numpy as np
z=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/r22d43_twelve.py')
s,M,E=z['st'],z['M'],z['E']
assert s.digest()=='f0290f93bea0295ecc00c2a42f725abfd59d3c37732834bb82f5da41c38fa793'
C=E.sim.get_context()
def info(i):
    p=s._poses[i]
    if p.kidx>=0: return (0,-1)
    l,r=E.T.factors(s._pose_matrix(i)); return (E.T.dist(l),E.T.branch(l))
inf={i:info(i) for i in np.unique(s.pose_id).tolist()}
cl=s.classify(61,need_inside=True,stop_at_straddle=True); assert cl.straddle is None and not cl.uncertain
P=E.T.branch(tuple(C.data.N[61]))
cnt=Counter()
for p in cl.inside.tolist():
    h,b=inf[int(s.pose_id[p])]; cnt[(h,'lat' if h==0 else ('own' if b==P else 'foreign'))]+=1
print('CAP61_INSIDE',dict(cnt))
o=s.apply(E.sim.Twist(C,61,E.R.element(M,61,15),'menu',{'menu':M.name}),journal=False); assert o.applied and not o.uncertain
print('AFTER',s.digest())
inf2={}
def info2(i):
    if i not in inf2:
        p=s._poses[i]
        inf2[i]=(0,-1) if p.kidx>=0 else (E.T.dist(E.T.factors(s._pose_matrix(i))[0]),0)
    return inf2[i]
after=Counter(info2(int(s.pose_id[p]))[0] for p in cl.inside.tolist())
print('CAP61_INSIDE_AFTER_HEIGHTS',dict(after))
