# Theory round 22, line (b), round 5 (R22B-26): at the depth-60 stop of seed 7104, the small chamber orbits of the
# SAFE alphabet (degrees 2 to 7) and their holes; then, for every unit's home-cell target and every triangle and
# tetrahedron home-cell target of the five classes, whether it meets a small orbit, and if so the joint target-image
# search on those orbits (R22B-22). Every small orbit is full and no target meets one.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22b26_small60.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22b26_small60.py
import sys,json,resource,signal,time
from pathlib import Path
from types import SimpleNamespace
from collections import Counter,defaultdict,deque
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1150,1200))
def stop(*a): print('CUT_SHORT_CPU_1150_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s
B=s.B
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json')
print('STATE',st.digest(),int(st.off_lattice_count()),flush=True)
G=B.Alphabet(SimpleNamespace(st=st),M); orbit,sizes=G.orbits(); small=np.flatnonzero((sizes>1)&(sizes<8)); smask=np.isin(orbit,small)
for o in small:
 D=np.flatnonzero(orbit==o);print('SMALL',int(D[0]),len(D),'holes',D[~G.occupied[D]].tolist(),flush=True)
bl=s.BL.Blocks(SimpleNamespace(st=st),M);domains=s.rim.Domains(st);dom=np.asarray(domains.dom);members,cls=s.CL.classes();dcls={}
for d in range(1,len(domains.reps)):
 h=s.CL.hub(domains.reps[d],members,cls);dcls[d]=int(h[0][1]) if len(h)==1 else None
units=[]
for b,pieces in enumerate(bl.blocks):
 ps=np.asarray(pieces)
 for d in np.unique(dom[ps]):units.append((b,int(d),ps[dom[ps]==d]))
byclass={}
for P in range(5):
 cells=defaultdict(list)
 for x,k in enumerate(map(bytes,np.packbits(G.home[:,members[P]],axis=1))):cells[k].append(x)
 byclass[P]=cells
cache={}
def test(T):
 S=tuple(sorted(int(x) for x in T if smask[x]))
 if S in cache:return cache[S]
 if not S:cache[S]=('pass_empty',1,[]);return cache[S]
 os=sorted(set(orbit[list(S)]));D=np.flatnonzero(np.isin(orbit,os));H=set(np.flatnonzero(~G.occupied[D]).tolist());root=tuple(np.searchsorted(D,S).tolist())
 gens=sorted({tuple(np.searchsorted(D,G.images([n],D)).tolist()) for n in range(len(G.letters))});seen={root};queue=deque([root]);result='fail_exhausted'
 while queue:
  v=queue.popleft()
  if set(v)<=H:result='pass';break
  for g in gens:
   w=tuple(sorted(g[x] for x in v))
   if w not in seen:seen.add(w);queue.append(w)
  if len(seen)>1000000:result='incomplete_1m';break
 cache[S]=(result,len(seen),[int(G.orbit_least[o]) for o in os]);return cache[S]
rows=[]
for u,(b,d,ps) in enumerate(units):
 P=dcls[d]
 if P is None:continue
 L=bl.labels[np.searchsorted(bl.off,ps)][:,members[P]];assert (L==L[0]).all() and not (L==2).any()
 key=bytes(np.packbits(L[0]==1));T=np.asarray(byclass[P].get(key,[]),np.int64);result=test(T)
 if result[2]:
  row={'unit':u,'class':P,'cell':[int(c) for c in members[P][L[0]==1]],'home':len(T),'small_target':T[smask[T]].tolist(),'joint':result,'N':bool(B.condition(G,T)['necessary'])};rows.append(row);print('UNIT_HIT',json.dumps(row),flush=True)
print('UNIT_SUMMARY','units',len(units),'touch_small',len(rows),'joint_results',dict(Counter(r['joint'][0] for r in rows)),flush=True)
for P in range(5):
 for kind in (3,4):
  total=hit=0;out=Counter();necessary=Counter();examples=[]
  for key,xs in byclass[P].items():
   if int(np.unpackbits(np.frombuffer(key,np.uint8)).sum())!=kind:continue
   total+=1;T=np.asarray(xs,np.int64);res=test(T)
   if not res[2]:continue
   hit+=1;out[res[0]]+=1;necessary[bool(B.condition(G,T)['necessary'])]+=1
   examples.append({'cell':[int(c) for c in members[P][np.unpackbits(np.frombuffer(key,np.uint8))[:120].astype(bool)]],'small_target':T[smask[T]].tolist(),'joint':res})
  print('TRANSPORT',json.dumps({'class':P,'kind':kind,'total':total,'touch_small':hit,'joint':dict(out),'N':dict(necessary),'examples':examples}),flush=True)
print('CPU',round(time.process_time(),1),flush=True)
