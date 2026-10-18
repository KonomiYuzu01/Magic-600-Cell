# Theory round 22, line (b), round 4 (R22B-21): from the solved state, a greedy packing of single-twist
# blocking sets isolates SAFE cap 0; the eight-twist word is replayed, the alphabet rebuilt at the reached height-2
# state, and the group induced on a twelve-chamber orbit of cap 0 closed exhaustively (order 12, so no Alt(12)).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22b21_regular12.txt (about 2 CPU minutes and 5 minutes in all).
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22b21_regular12.py
import sys,random,resource,signal,time,json,math
from pathlib import Path
from types import SimpleNamespace
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1150,1200))
def stop(*a):
 print('CUT_SHORT_CPU_1150_SECONDS',flush=True); raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen,buffer as B
M=screen.restore.menu('I_a'); ctx=screen.CL.ctx; kp,reg=ctx.kplus,ctx.regions
item=screen.restore.jumble_items(M)[0]
st=screen.sim.State(ctx,menu=M)
o=st.apply(screen.sim.Twist(ctx,0,screen.restore.element(M,0,item),'menu',{'menu':M.name})); assert o.applied and not o.uncertain
base=B.Alphabet(SimpleNamespace(st=st),M)
blocked0=sorted(set(range(600))-set(base.safe)); touch0=np.flatnonzero(base.home[base.capx.get(0,np.flatnonzero(base.home[:,0]))].any(axis=0))
frames=[kp.index_of_matrix(screen.restore.to_tuple(M.frame(c))) for c in range(600)]
assert min(frames)>=0
rot=[int(kp.compose_vec(np.array([f]),np.array([kp.inv[frames[0]]]))[0]) for f in frames]
def bits(xs):
 return sum(1<<int(x) for x in xs)
blocked=[]; touch=[]
for c,k in enumerate(rot):
 assert int(kp.perms[k,0])==c
 blocked.append(bits(kp.perms[k,blocked0])); touch.append(bits(kp.perms[k,touch0]))
allowed=[c for c in range(600) if not(touch[0]>>c&1) and not(blocked[c]&1)]
compatible={c:[d for d in allowed if not(touch[c]>>d&1) and not(blocked[c]>>d&1) and not(blocked[d]>>c&1)] for c in allowed}
target=touch[0]&~1; rng=random.Random(2221); best=(target.bit_count(),[])
print('BASE_ITEM',item,'BASE_BLOCKED',len(blocked0),'NEIGHBORS_TO_BLOCK',target.bit_count(),'CANDIDATES',len(allowed),flush=True)
for trial in range(2000):
 remaining=target; choices=allowed.copy(); word=[]
 while remaining and choices:
  ranked=sorted(choices,key=lambda c:-(blocked[c]&remaining).bit_count())
  top=(blocked[ranked[0]]&remaining).bit_count()
  if not top: break
  top_choices=[c for c in ranked if (blocked[c]&remaining).bit_count()==top]
  c=rng.choice(top_choices if trial%4 else ranked[:min(10,len(ranked))])
  word.append(c); remaining &= ~blocked[c]; choices=[d for d in choices if d in compatible[c]]
 if remaining.bit_count()<best[0]:
  best=(remaining.bit_count(),word); print('BEST',trial,best,'UNBLOCKED',[c for c in range(600) if remaining>>c&1],flush=True)
 if not remaining: break
if best[0]:
 print('NO_ISOLATED_SAFE_CAP_IN_2000_GREEDY_PACKINGS','CPU',round(time.process_time(),1),flush=True); raise SystemExit(0)
word=[[c,item] for c in best[1]]; st=screen.sim.State(ctx,menu=M)
for c,i in word:
 o=st.apply(screen.sim.Twist(ctx,c,screen.restore.element(M,c,i),'menu',{'menu':M.name})); assert o.applied and not o.uncertain
height={int(screen.T.dist(screen.T.factors(st._pose_matrix(pid))[0])) for pid in np.unique(st.pose_id[B.off_pieces(st)])}
assert height=={2}
G=B.Alphabet(SimpleNamespace(st=st),M); orbit,sizes=G.orbits()
assert 0 in G.safe and not any(c!=0 and touch[0]>>c&1 for c in G.safe)
for o in np.unique(orbit[np.flatnonzero(G.home[:,0])]):
 D=np.flatnonzero(orbit==o)
 if len(D)<8: continue
 ident=tuple(range(len(D))); gs={tuple(np.searchsorted(D,G.images([n],D)).tolist()) for n in range(len(G.letters))}-{ident}
 group={ident}; todo=[ident]
 for p in todo:
  for q in gs:
   z=tuple(q[x] for x in p)
   if z not in group: group.add(z); todo.append(z)
 assert len(group)<math.factorial(len(D))//2
 print('COUNTEREXAMPLE',json.dumps({'word':word,'state':st.digest(),'off':int(st.off_lattice_count()),'heights':sorted(height),'safe':G.safe,'members':D.tolist(),'degree':len(D),'group_order':len(group),'holes':[int(x) for x in D if not G.occupied[x]]}),flush=True)
 break
print('CPU',round(time.process_time(),1),flush=True)
