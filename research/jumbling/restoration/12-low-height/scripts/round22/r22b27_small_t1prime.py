# Theory round 22, line (b), round 5 (R22B-27): the two specified T1' middles, 151 (the endpoint of Theorem C's pure
# cell action, from r22c3_u151.py) and 180 (the recorded steps 174, 175, 176, then (481,1) and the 288-letter landing
# word of experiment D, with Bad(W^-1) rebuilt by ghost replay and matched to the stored target), each with its
# alphabet rebuilt at the middle: small orbits (degrees 2 to 7) and how many target chambers they hold; N1 and N2 at 180.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22b27_small_t1prime.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22b27_small_t1prime.py
import sys,runpy,json,resource,signal,time,hashlib
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(1150,1200))
S=Path('research/jumbling/restoration/12-low-height');E=S/'runs/round22/line-c/exp-d'
ns=runpy.run_path(str(S/'scripts/round22/r22c3_u151.py'))
s=ns['s'];B=s.B;np=ns['np'];G=ns['G'];T=ns['T']
def report(name,G,T):
 small=np.flatnonzero((G.orbit_size>1)&(G.orbit_size<8))
 hits=sorted(set(map(int,G.orbit[T]))&set(map(int,small)))
 print('SMALL_TARGET_CHECK',name,'small_orbits',dict(Counter(map(int,G.orbit_size[small]))),'target_small_orbits',len(hits),'target_small_chambers',int(np.isin(G.orbit[T],small).sum()),flush=True)
report('151',G,T)
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json')
def read(p):return json.loads(p.read_text())
def apply(word):
 for c,i in word:
  out=st.apply(s.sim.Twist(s.CL.ctx,int(c),s.restore.element(M,int(c),int(i)),'menu',{'menu':M.name}))
  assert out.applied and not out.uncertain
for u in [174,175,176]:
 r=read(E/f'step-{u}.json');assert st.digest()==r['start_digest']
 apply(read(E/f'word-{u}.json')['word']);assert st.digest()==r['end_digest']
apply([[481,1]])
wd=read(E/'e180-481-1/cp4-word.json');assert st.digest()==wd['input_digest'];apply(wd['word']);assert st.digest()==wd['output_digest']
sys.path.insert(0,str(Path('research/jumbling/restoration/09-general-method/scripts/echain').resolve()))
import fastghost as FG
Wi=read(E/'e180-481-1/u-winv.json')['Q'];gh=FG.Ghost(FG.Frames(),np.zeros(len(st.pose_id),np.int64),np.arange(len(st.pose_id)))
for c,i in Wi:gh.apply(c,s.restore.element(M,c,i),(c,i))
T=np.flatnonzero(~gh.alive|(gh.dom!=0));target=read(E/'e180-cert/e180-target.json')
assert T.tolist()==target['T'] and st.digest()==target['state_digest']
assert hashlib.sha256(json.dumps(T.tolist()).encode()).hexdigest()==target['T_digest']
G=B.Alphabet(SimpleNamespace(st=st),M);G.orbits();co=B.condition(G,T)
print('MIDDLE180',st.digest(),len(G.holes),'T',len(T),'N1',co['fixed_vacant'],'N2',co['necessary'],flush=True)
report('180_after_D_and_481_1',G,T)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
