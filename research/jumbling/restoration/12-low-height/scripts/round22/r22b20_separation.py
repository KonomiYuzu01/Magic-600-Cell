# Theory round 22, line (b), round 4 (R22B-20): at the depth 40 stop, the cycle types of every SAFE letter on
# b00's 19 target orbits (the pairs no letter separates), one 120-letter word that separates all 40 equal-size pairs,
# local Jordan witnesses (R22B-15) on the target orbits, and the R22B-04 bound's size.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22b20_separation.txt (about 50 CPU seconds).
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22b20_separation.py
import sys,json,random,resource,signal,time,math,hashlib
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1150,1200))
def stop(*a):
 print('CUT_SHORT_CPU_1150_SECONDS',flush=True); raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen,buffer as B
st,M,_=screen.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json')
G=B.Alphabet(SimpleNamespace(st=st),M); orbit,sizes=G.orbits()
target=json.loads(Path('research/jumbling/restoration/12-low-height/runs/lc13-k40/target-b00.json').read_text()); T=target['T']
assert st.digest()==target['state_digest']
assert hashlib.sha256(json.dumps(T).encode()).hexdigest()==target['T_digest']
want=sorted({int(o) for o in orbit[T] if sizes[o]>=8})
pairs=[(a,b) for a in want for b in want if a<b and sizes[a]==sizes[b]]
undistinguished=set(pairs); orders=Counter()
for c,item,k,ys in G.letters:
 xs=G.capx[c]; q=np.searchsorted(xs,ys); ident=np.arange(len(xs))
 if np.array_equal(q[q],ident): order=2
 else:
  assert np.array_equal(q[q[q]],ident); order=3
 assert np.any(q!=ident); orders[order]+=1
 moved=np.bincount(orbit[xs[xs!=ys]],minlength=len(sizes))
 undistinguished={ab for ab in undistinguished if moved[ab[0]]==moved[ab[1]]}
print('STATE',st.digest(),len(G.holes),flush=True)
print('ALL_ORBIT_SIZES',sorted(Counter(map(int,sizes)).items()),flush=True)
print('TARGET_SMALL_2_TO_7',sum(2<=sizes[o]<=7 for o in set(orbit[T])),flush=True)
print('LETTER_ORDERS',dict(sorted(orders.items())),flush=True)
print('EQUAL_SIZE_PAIRS',len(pairs),'UNSEPARATED_BY_ANY_LETTER',[(int(G.orbit_least[a]),int(G.orbit_least[b]),int(sizes[a])) for a,b in sorted(undistinguished)],flush=True)
found={}; rng=random.Random(1); primes=B.primes_upto(int(sizes.max()))
large=set(np.flatnonzero(sizes>=8).tolist())
for trial in range(1,201):
 word=[rng.randrange(len(G.letters)) for _ in range(120)]
 lab,lens=B.cycles(G.perm(word)); reps=np.flatnonzero(lens)
 if trial==1:
  spectra={o:Counter(map(int,lens[reps[orbit[reps]==o]])) for o in want}
  left=[(int(G.orbit_least[a]),int(G.orbit_least[b])) for a,b in pairs if spectra[a]==spectra[b]]
  print('FIRST_120_WORD_SHA256',hashlib.sha256(json.dumps(G.pairs(word),separators=(',',':')).encode()).hexdigest(),'UNSEPARATED_PAIRS',left,flush=True)
 for o in sorted(large-set(found)):
  for r in reps[orbit[reps]==o]:
   p=int(lens[r]); d=int(sizes[o])
   if p in primes and d<2*p and p<=d-3:
    found[o]=(trial,p); break
 if large==set(found): break
print('LOCAL_TARGET_WITNESSES_least_d_trial_p',[[int(G.orbit_least[o]),int(sizes[o]),*found[o]] for o in want if o in found],flush=True)
print('LOCAL_ALL_LARGE',len(found),'OF',len(large),'MISSING',[(int(G.orbit_least[o]),int(sizes[o])) for o in sorted(large-set(found))],flush=True)
bound=math.prod(math.comb(int(sizes[o]),T.count(T[0]) if False else sum(int(orbit[t])==o for t in T)) for o in want)-1
print('TARGET_BOUND_DIGITS',len(str(bound)),'BELOW_7200_POW111',bound<7200**111,'CPU',round(time.process_time(),1),flush=True)
