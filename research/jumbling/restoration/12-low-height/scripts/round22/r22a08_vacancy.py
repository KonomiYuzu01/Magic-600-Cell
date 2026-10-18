# Theory round 22, R22A-08: the senior reviewer's exact computation, kept as written apart from this header;
# the integrator re-ran it and got the output in runs/round22/r22a08_vacancy.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22a08_vacancy.py
import sys, resource, json
from pathlib import Path
from collections import Counter
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(840,840))
repo=Path.cwd(); S=repo/'research/jumbling/restoration/12-low-height/scripts'
sys.path.insert(0,str(S))
import screen as E
import numpy as np
st,M,_=E.load('record',str(repo/'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json'))
D=E.rim.Domains(st); dom=np.asarray(D.dom); lat=dom==0
members,cls=E.CL.classes(); hubs={d:E.CL.hub(D.reps[d],members,cls) for d in range(1,len(D.reps))}
C=E.CL.data; NP=len(st.pose_id); homes=C.sig_matrix(np.arange(NP)); holes=E.B.hole_set(st)
occupied=np.ones(NP,bool); occupied[holes]=False
print('state',st.digest(),'off',int((~lat).sum()),flush=True)
G=E.B.Alphabet(SimpleNamespace(st=st),M); G.orbits()
rows=json.loads((S.parent/'runs/screen-k40.json').read_text())['units']
for P in (2,3,4):
 poles=members[P]; own=lat|np.isin(dom,[d for d,h in hubs.items() if len(h)==1 and h[0][1]==P])
 sig=np.zeros((NP,120),bool); free=[]
 for j,c in enumerate(poles):
  cl=st.classify(int(c),need_inside=True,stop_at_straddle=False)
  assert not cl.uncertain
  ins=np.asarray(cl.inside,np.int64);sig[ins,j]=True
  if cl.straddle is None and not (~own)[ins].any():free.append(c)
 keys,inverse=np.unique(np.packbits(homes[:,poles],axis=1),axis=0,return_inverse=True)
 kinds=np.unpackbits(keys,axis=1).sum(1); counts=np.bincount(inverse); occ=np.bincount(inverse,weights=occupied).astype(int)
 physical=Counter(int(kinds[j]) for j in range(len(keys)) if occ[j]==0)
 offkeys={np.packbits(sig[p]).tobytes() for p in np.flatnonzero(~lat)}
 vacant=Counter(int(kinds[j]) for j,k in enumerate(keys) if occ[j]==0 and k.tobytes() not in offkeys)
 print('class',P,'free',len(free),'physically_vacant',dict(sorted(physical.items())),'vacant_excluding_off_signatures',dict(sorted(vacant.items())),flush=True)
 for row in rows:
  if row['class']!=P:continue
  p=row['first']; key=np.packbits(sig[p]).tobytes()
  hit=[j for j,k in enumerate(keys) if k.tobytes()==key]
  if hit:
   omega=np.flatnonzero(inverse==hit[0]); cond=E.B.condition(G,omega)
   print('home_unit',row['unit'],'kind',row['kind'],'occupied',int(occ[hit[0]]),'N1',bool(cond['fixed_vacant']),'N1_N2',bool(cond['necessary']),'fixed_occupied',len(cond['fixed_occupied']),flush=True)
print('done',flush=True)
