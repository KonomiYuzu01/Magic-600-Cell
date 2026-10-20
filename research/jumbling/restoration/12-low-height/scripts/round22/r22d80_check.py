# Theory round 22, line (d), round 6: the senior reviewer's replay of R22D-80, kept as written apart from this header
# (the integrator's re-run printed the same output; it is in runs/round22/r22d80_check.txt). From a snapshot of the
# 3000-twist stop of seed 7302 (015d839e...): R22D-75's seven letters (to 1ff0061f...), then (124, 2) U, (274, 13) H6,
# (46, 1) U, (46, 3) U and (104, 14) H6, each admissible and certain with its family's condition checked, ending at
# bbef1c8e...
# Research only, headless Linux; exact geometry. It writes no file.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d80_check.py <dir>/s7302-stop.pkl
import sys,json,resource
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
from r22d31_snapshots import load
C=E.sim.get_context(); M=E.R.menu('I_a'); s,_=load(sys.argv[1],E)
def put(c,j):
 o=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
 assert o.applied and not o.uncertain
cache={}
def h():
 for p in s._poses:
  if p.key not in cache:
   if p.kidx>=0:cache[p.key]=0
   else:
    l,r=E.T.factors(p.matrix);assert E.T.dist(r)==0;cache[p.key]=E.T.dist(l)
 return np.array([cache[p.key] for p in s._poses])[s.pose_id]
def keys(ps):return [s._poses[int(s.pose_id[p])].key for p in ps]
for c,j in [(64,15),(185,14),(357,12),(471,12),(263,14),(407,13),(523,12)]:put(c,j)
assert s.digest()=='1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81'
hstart=h();targets=np.flatnonzero(hstart==6)
for k,(c,j,f,tau) in enumerate([(124,2,'U',274),(274,13,'H6',274),(46,1,'U',104),(46,3,'U',104),(104,14,'H6',104)],1):
 before=h();six=np.flatnonzero(before==6);oldkeys=keys(six);u0=len(E.straddler_set(s,tau))
 put(c,j);after=h();u=len(E.straddler_set(s,tau));assert after.max()<=6
 if f=='U':assert keys(six)==oldkeys and int((after==6).sum())==len(six) and u<u0
 else:assert int((after==6).sum())<len(six)
 print(json.dumps(dict(step=k,letter=[c,j],family=f,tau=tau,u=u,n2_n4_n6=[int((after==n).sum()) for n in [2,4,6]],off=int((after>0).sum()),N=E.R.Restorer(s,M).misalignment(),digest=s.digest())),flush=True)
assert s.digest()=='bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299'
print('TARGET_HEIGHTS',dict(Counter(h()[targets].tolist())),'LATTICE_COLLATERAL',int(((hstart==0)&(h()>0)).sum()),flush=True)
