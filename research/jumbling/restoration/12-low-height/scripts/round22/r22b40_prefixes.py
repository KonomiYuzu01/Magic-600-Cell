# Theory round 22, line (b), round 6 (R22B-40 to R22B-48): for one of E2's nine certificate steps (R22B_STEP = b14,
# b16, b20, b21, b07, b13, b19, b15 or b06), the target's orbit rows (least, size, k, occupied, holes), the stored
# certificate's length arithmetic, and the literal replay of the reviewer's vacancy prefix, letter by letter, keeping
# every off pose and lattice flag (R22B_POINTS=1 also prints the holes per orbit). The prefixes are not complete steps.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it for the
# nine steps and got the output in runs/round22/r22b40_prefixes.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: R22B_STEP=b15 python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22b40_prefixes.py
import os,sys,json,hashlib,gzip
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
for k in ('OPENBLAS_NUM_THREADS','OMP_NUM_THREADS','MKL_NUM_THREADS'): os.environ[k]='1'
S=Path('research/jumbling/restoration/12-low-height/scripts'); R=S.parent/'runs/round22/line-a/replay2'
sys.path[:0]=[str(S.resolve()),str(Path('research/jumbling/theory').resolve()),str(Path('research/jumbling/restoration/09-general-method/scripts/echain').resolve()),str(Path('research/jumbling/restoration/09-general-method/scripts/gm').resolve())]
import screen,buffer as B,slp_apply as A,numpy as np
cases={'b14':('run1/z-b12.json.gz','run1/b14-target.json'),'b16':('run1/z-b14.json.gz','run1/b16-target.json'),'b20':('run1/z-b16.json.gz','run1/b20-target.json'),'b21':('run1/z-b20.json.gz','run1/b21-target.json')}
for name,p in [('b07','run2/03-b07'),('b13','run2/04-b13'),('b19','run2/05-b19'),('b15','run2/09-b15'),('b06','b06-71/t1/b06-14')]: cases[name]=(p+'-z1-table.json.gz',p+'-v-target.json')
P={'b14':[[354,4],[539,1],[354,8],[539,1]],'b16':[[65,3],[137,8],[65,9],[137,4],[118,2],[65,1],[118,5],[65,1]],'b20':[[539,1],[485,1],[556,7],[485,1],[556,6],[539,1]],'b21':[[38,3],[38,3],[65,9],[187,5],[65,9],[195,4],[227,2],[195,8],[227,5],[65,1],[32,3],[65,1],[32,9],[14,1],[65,1],[14,1],[65,1]],'b07':[[353,6],[353,10],[551,8],[353,10],[551,4],[378,6],[537,6],[378,7],[537,7],[560,2],[378,2],[560,5],[378,5],[445,7],[378,6],[445,6],[378,7]],'b13':[[353,10],[551,8],[353,10],[551,4],[378,6],[537,6],[378,7],[537,7],[527,4],[378,6],[527,8],[378,7]],'b15':[[371,3],[537,6],[371,9],[537,7],[270,9],[445,2],[270,3],[445,5],[371,3],[412,6],[371,9],[412,7],[371,5],[542,11],[371,2],[542,11],[252,6],[474,1],[551,11],[474,1],[551,11],[252,7],[252,1],[201,7],[353,5],[201,6],[353,2],[252,1],[252,5],[412,9],[515,9],[412,3],[515,3],[252,2],[252,9],[131,7],[270,1],[131,6],[270,1],[252,3],[252,8],[148,5],[350,10],[148,2],[350,10],[252,4],[252,8],[193,7],[412,2],[193,6],[412,5],[252,4],[439,8],[350,11],[462,1],[350,11],[462,1],[439,4]]}
name=os.environ.get('R22B_STEP','b15'); table,target=cases[name]
st,M,_=screen.load('table',str(R/table)); d=json.loads((R/target).read_text()); T=np.array(d['T'],np.int64)
assert d['state_digest']==st.digest()
assert hashlib.sha256(json.dumps(d['T']).encode()).hexdigest()==d['T_digest']
G=B.Alphabet(SimpleNamespace(st=st),M); G.orbits()
rows=[]
for o in sorted(set(G.orbit[T])):
 t=T[G.orbit[T]==o]; h=G.holes[G.orbit[G.holes]==o]
 if G.orbit_size[o]>1: rows.append([int(G.orbit_least[o]),int(G.orbit_size[o]),len(t),int(G.occupied[t].sum()),len(h)])
print('ROWS least,size,k,occupied,holes',rows)
print('FIXED',int((G.orbit_size[G.orbit[T]]==1).sum()),'OCCUPIED',T[G.occupied[T]].tolist())
if os.environ.get('R22B_POINTS'): print('HOLES_BY_ORBIT',{int(G.orbit_least[o]):G.holes[G.orbit[G.holes]==o].tolist() for o in sorted(set(G.orbit[G.holes]))})
c=json.load(gzip.open(R/target.replace('-target.json','-cert.json.gz'))); lengths=[]
for n in c['nodes']:
 v=1 if n['op']=='letter' else sum(lengths[x] for x in n['args']) if n['op']=='cat' else lengths[n['arg']]*(n['exp'] if n['op']=='pow' else 1)
 assert v==n['length']; lengths.append(v)
assert lengths[c['V']]==c['V_length']
cost=Counter()
for s in c['steps']: cost[s['orbit']]+=lengths[s['node']]
hot=max(cost,key=cost.get)
print('CERT V_digits,steps,hottest_orbit,percent_floor',len(str(c['V_length'])),len(c['steps']),hot,100*cost[hot]//c['V_length'])
V=P.get(name,[]); G.numbers(V); f=B.off_fingerprint(st); lat=st.lattice_flags().copy()
for c,i in V:
 A.apply_segment(st,M,{'word':[[c,i]]})
 assert B.off_fingerprint(st)==f and np.array_equal(st.lattice_flags(),lat)
left=np.setdiff1d(T,B.hole_set(st)).tolist()
print('PREFIX_ONLY',name,'length',len(V),'before',int(G.occupied[T].sum()),'after',len(left),'remaining',left,'digest',st.digest())
