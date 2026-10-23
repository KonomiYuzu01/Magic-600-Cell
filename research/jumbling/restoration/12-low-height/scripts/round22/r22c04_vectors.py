# Theory round 22, R22C-04 (line (c)): the type vector of a home cell of each kind.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c04_vectors.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22c04_vectors.py
import sys,json,resource
from pathlib import Path
from collections import Counter,defaultdict
resource.setrlimit(resource.RLIMIT_CPU,(600,600))
sys.path[:0]=[str(Path('research/jumbling/theory').resolve())]
import classes as CL
import numpy as np
members,_=CL.classes(); typ=np.asarray(CL.ctx.regions.orbit_of)
home=CL.data.sig_matrix(np.arange(len(typ)))
vectors=defaultdict(set); counts=Counter(); examples={}
for P in members:
 cells=defaultdict(list)
 for p,key in enumerate(map(bytes,np.packbits(home[:,P],axis=1))): cells[key].append(p)
 for key,ps in cells.items():
  kind=int(np.unpackbits(np.frombuffer(key,np.uint8)).sum()); v=Counter(map(int,typ[ps])); vectors[kind].add(tuple(sorted(v.items())));counts[kind]+=1
  examples.setdefault(kind,ps)
for k in range(1,5):
 assert len(vectors[k])==1
 print(json.dumps({'kind':k,'home_cells_all_classes':counts[k],'size':sum(dict(next(iter(vectors[k]))).values()),'type_vector':dict(next(iter(vectors[k])))}))
print('type0_count',int((typ==0).sum()))
for p in np.flatnonzero(typ==0)[:4]: print('type0_chamber',int(p),'caps',np.flatnonzero(home[p]).tolist())
