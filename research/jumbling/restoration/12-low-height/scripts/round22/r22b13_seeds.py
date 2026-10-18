# Theory round 22, R22B-13, R22B-14, R22B-16 and R22B-17: the senior reviewer's exact computation, kept as written apart
# from this header; the integrator re-ran it and got the output in runs/round22/r22b13_seeds.txt. At the depth 40 stop
# it checks two eight-letter commutator powers as seeds for b00's orbits 34 and 66 in the full decreasing-size order
# (the cycle criterion of R22B-12 on the protected union, then the power itself), counts the orders of all SAFE letters,
# audits every node length and both bounds of the E22-1b certificates, and prints E(7200) and F(7200).
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B
#   research/jumbling/restoration/12-low-height/scripts/round22/r22b13_seeds.py   (about 2 minutes)
import sys, math, json, gzip, resource
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1150,1200))
S=Path('research/jumbling/restoration/12-low-height/scripts').resolve()
sys.path.insert(0,str(S))
import screen, buffer as B
st,M,_=screen.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json')
G=B.Alphabet(SimpleNamespace(st=st),M)
orbit,sizes=G.orbits()
T=json.loads((S.parent/'runs/lc13-k40/target-b00.json').read_text())['T']
order=sorted(set(orbit[T].tolist()),key=lambda o:(-int(sizes[o]),int(G.orbit_least[o])))
W=[
(34,4,[[118,11],[254,11],[106,7],[160,4],[118,11],[160,8],[106,6],[254,11]]),
(66,5,[[254,4],[320,6],[340,9],[170,7],[254,8],[170,6],[340,3],[320,7]])
]
for least,m,w in W:
 o=int(orbit[least]); prefix=order[:order.index(o)+1]
 X=np.flatnonzero(np.isin(orbit,prefix))
 labels,lens=B.cycles(G.permutation(w))
 reps=[r for r in np.flatnonzero(lens) if orbit[r] in prefix]
 thirds=[r for r in reps if lens[r]%3==0]
 assert len(thirds)==1 and lens[thirds[0]]==3 and orbit[thirds[0]]==o
 spectrum=sorted({int(lens[r]) for r in reps if lens[r]>1 and r!=thirds[0]})
 assert math.lcm(*spectrum)==m
 p=G.images(G.numbers(w)*m,X)
 moved=X[p!=X]
 assert len(moved)==3 and np.all(orbit[moved]==o)
 a=int(moved.min()); b=int(p[np.searchsorted(X,a)]); c=int(p[np.searchsorted(X,b)])
 assert int(p[np.searchsorted(X,c)])==a
 print('SEED',least,'length',len(w)*m,'M',m,'other_lengths',spectrum,'anchors',[a,b,c],flush=True)
orders=Counter()
for grip,item,k,ys in G.letters:
 xs=G.capx[grip]; p=np.searchsorted(xs,ys); identity=np.arange(len(xs))
 assert not np.array_equal(p,identity)
 if np.array_equal(p[p],identity): orders[2]+=1
 else:
  assert np.array_equal(p[p[p]],identity)
  orders[3]+=1
 assert np.array_equal(G.type[xs],G.type[ys])
print('LETTER_ORDERS',dict(sorted(orders.items())),'MAX_TYPE_SIZE',int(np.bincount(G.type).max()),flush=True)
for name in ('b00','b19'):
 with gzip.open(S.parent/('runs/round22/e22-1b/'+name+'-cert.json.gz'),'rt') as f: doc=json.load(f)
 lens=[]
 for n in doc['nodes']:
  v=1 if n['op']=='letter' else sum(lens[a] for a in n['args']) if n['op']=='cat' else lens[n['arg']]*(n['exp'] if n['op']=='pow' else 1)
  assert v==n['length']; lens.append(v)
 rows=doc['orbits']; value=2*sum(r['k']*(r['s']+2*r['e']+2*r['f']) for r in rows)
 assert doc['bound']==value and doc['V_length']==lens[doc['V']]<=value
 assert doc['V_length'] < 10**(166 if name=='b00' else 141) and value<10**166
 digits=[len(str(r['s'])) for r in rows]
 constant=4*sum(r['k']*((r['size']-2)*(2*r['size']-1)+(r['size']-3)*(2*r['size']-3)*(2*r['size']-1)) for r in rows)
 print('CERT',name,'T',doc['T_size'],'orbits',len(rows),'steps',len(doc['steps']),'nodes',len(lens),'seed_digits',[min(digits),max(digits)],'max_e',max(r['e'] for r in rows),'max_f',max(r['f'] for r in rows),'V_digits',len(str(doc['V_length'])),'bound_digits',len(str(value)),'table_constant',constant,flush=True)
d=7200; e=(d-2)*(2*d-1); f=(d-3)*(2*d-3)*(2*d-1)
print('SCHREIER',e,f,[(t,2*t,4*t*(e+f)) for t in (111,198)],flush=True)
