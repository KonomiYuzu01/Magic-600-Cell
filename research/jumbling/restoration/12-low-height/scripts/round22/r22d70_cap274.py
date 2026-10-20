# Theory round 22, line (d), round 5 (R22D-70, R22D-71, R22D-76): at the 3000-twist stop of seed 7302 (state
# 015d839e...), the counts by left height, the caps that hold no lattice piece, the clean caps and the fixed free and
# SAFE alphabets; cap 274's height counts and straddlers, with the unit, block, class cell, free grips and home
# chambers of each straddler; and the caps that hold 108996 and 137233 (group C), with the height counts and
# straddlers of those of branch 1.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the
# output in runs/round22/r22d70_cap274.txt, which agrees with the values the reviewer reported. About 8 CPU minutes.
# Build the snapshot first (about 30 CPU minutes): python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
# It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d70_cap274.py <dir>/s7302-stop.pkl
import sys,json,resource
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/restoration/12-low-height/scripts/round22','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E, buffer as B
from r22d31_snapshots import load
C=E.sim.get_context(); M=E.R.menu('I_a'); s,_=load(sys.argv[1],E)
assert s.digest()=='015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f'
info={i:E.T.pose_tree(s._pose_matrix(i)) for i in np.unique(s.pose_id).tolist()}
h=np.array([info[int(i)][0] for i in s.pose_id]); off=np.flatnonzero(h>0)
br=np.array([E.T.branch(tuple(n)) for n in C.data.N])
home=C.data.sig_matrix(np.arange(len(s.pose_id))); holes=np.zeros(len(s.pose_id),bool); holes[B.hole_set(s)]=True
lat_counts=home[~holes].sum(axis=0)
labels=np.zeros((len(off),600),np.uint8); clean={}; types={}; counts={}; ss={}
for c in range(600):
 cl=s.classify(c,need_inside=True,stop_at_straddle=False); assert not cl.uncertain
 oi=np.intersect1d(off,cl.inside); labels[np.searchsorted(off,oi),c]=1
 counts[c]=dict(Counter(h[cl.inside].tolist())); assert counts[c].get(0,0)==int(lat_counts[c])
 if cl.straddle is None:
  clean[c]=cl.inside; types[c]={(info[int(s.pose_id[p])][0],info[int(s.pose_id[p])][2]) for p in oi}
 else:
  ss[c]=E.straddler_set(s,c); labels[np.searchsorted(off,ss[c]),c]=2
safe=[c for c in clean if not types[c]]
free={b:[c for c in clean if br[c]==b and types[c]<={(2,b)}] for b in range(5)}
print('STATE',s.digest(),dict(Counter(h.tolist())),'LATTICE_FREE',np.flatnonzero(lat_counts==0).tolist())
print('ALPHABETS',json.dumps({'clean':len(clean),'safe':safe,'free':free}))
print('CAP274',counts[274],ss[274].tolist())
D=E.R.Restorer(s,M).domains(); dom=np.asarray(D.dom)
inside=np.packbits(labels==1,axis=1); outside=np.packbits(labels==0,axis=1)
pending=set(ss[274].tolist()); rows=[]
while pending:
 p=min(pending); seen=np.zeros(len(off),bool); queue=[int(np.searchsorted(off,p))]; seen[queue]=True
 while queue:
  r=queue.pop(); separated=((inside[r]&outside)|(outside[r]&inside)).any(axis=1)
  hits=np.flatnonzero(~seen&~separated); seen[hits]=True; queue.extend(hits.tolist())
 block=off[seen]; unit=block[dom[block]==dom[p]]; hit=sorted(set(unit.tolist())&pending); pending.difference_update(hit)
 b=int(info[int(s.pose_id[p])][2]); poles=np.flatnonzero(br==b); cells={}
 for q in hit:
  cell=tuple(int(c) for c in poles if labels[np.searchsorted(off,q),c]==1)
  assert not (labels[np.searchsorted(off,q),poles]==2).any()
  X=np.flatnonzero((home[:,poles]==np.isin(poles,cell)).all(axis=1))
  cells[q]={'cell':cell,'free_containing':sorted(set(cell)&set(free[b])),'home':len(X),'occupied':int((~holes[X]).sum())}
 rows.append({'unit':int(min(unit)),'size':len(unit),'block':int(min(block)),'block_size':len(block),'frames':len(set(dom[block].tolist())),'branch':b,'blockers':hit,'cells':cells})
print('BLOCKERS',json.dumps(rows))
for p in [108996,137233]:
 cs=np.flatnonzero(labels[int(np.searchsorted(off,p))]==1).tolist()
 print('C_CAPS',p,cs)
 for c in cs:
  if br[c]==1:print('C_OWN',c,counts[c],len(ss[c]),dict(Counter(h[ss[c]].tolist())),int(ss[c][0]))
