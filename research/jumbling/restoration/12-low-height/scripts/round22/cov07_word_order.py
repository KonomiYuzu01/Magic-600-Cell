# Theory round 22, coverage shard (COV-07): do the four stored T1' words of b03, b04, b12 and b18
# (runs/round22/t1prime-k40/steps/u<n>.json, 160, 942, 1158 and 160 letters, built at the depth 40 stop) form an
# order of successive strict' steps at the replay state of 737, skipping a unit that has already landed? Depth-first
# over the orders whose first unit is COV_FIRST (3, 4, 12 or 18); each word is applied literally and checked against
# the strict' contract. A CPU cutoff or an uncertain classification is inconclusive, never a negative result.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator ran it for the four
# values, at most two at a time, and got the outputs in runs/round22/cov07_word_order.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: COV_FIRST=3 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov07_word_order.py
import os,sys,json,resource,signal
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(1100,1120))
def cutoff(*args):
 print('INCONCLUSIVE_CPU_LIMIT',flush=True)
 raise SystemExit(124)
signal.signal(signal.SIGXCPU,cutoff)
S=Path('research/jumbling/restoration/12-low-height')
sys.path.insert(0,str((S/'scripts').resolve()))
import screen as s
ids=(3,4,12,18);first=int(os.environ['COV_FIRST']);assert first in ids
U={u:set(json.loads((S/f'runs/round22/line-a/units/eb5-k40-k00-b{u:02d}.json').read_text())['pieces']) for u in ids}
W={u:json.loads((S/f'runs/round22/t1prime-k40/steps/u{u}.json').read_text())['word'] for u in ids}
assert [len(W[u]) for u in ids]==[160,942,1158,160]
z,M,_=s.load('table',str(S/'runs/round22/line-a/replay/tables/737.json.gz'))
assert z.digest()=='2e3b8f990229799fc88198b2fe4f2062ab4f9d73c78347ee67b252d412b0a086'
def clone(st):
 out=s.sim.State(s.CL.ctx,menu=M)
 out.pose_id=st.pose_id.copy();out._poses=list(st._poses);out._lookup=dict(st._lookup)
 return out
def key(st,p):return st._poses[int(st.pose_id[p])].key
def step(st,u):
 out=clone(st);old=set(s.B.off_pieces(st).tolist());poses={p:key(st,p) for p in old}
 for c,i in W[u]:
  result=out.apply(s.sim.Twist(s.CL.ctx,int(c),s.restore.element(M,int(c),int(i)),'menu',{'menu':M.name}),journal=False)
  if result.uncertain:raise RuntimeError('INCONCLUSIVE_UNCERTAIN_CLASSIFICATION')
  if not result.applied:return None
 new=set(s.B.off_pieces(out).tolist())
 if U[u]&new or not new<=old:return None
 if any(key(out,p)!=poses[p] for p in new):return None
 return out
def search(st,remaining,path):
 off=set(s.B.off_pieces(st).tolist())
 remaining=tuple(u for u in remaining if U[u]&off)
 if not remaining:return {'order':path,'digest':st.digest(),'off':len(off)}
 choices=(first,) if not path else remaining
 for u in choices:
  nxt=step(st,u)
  print('CHECK',json.dumps({'prefix':path+[u],'strict_prime':nxt is not None}),flush=True)
  if nxt is None:continue
  answer=search(nxt,tuple(v for v in remaining if v!=u),path+[u])
  if answer is not None:return answer
 return None
answer=search(z,ids,[])
print('FOUND' if answer is not None else 'NO_ORDER_IN_BRANCH',json.dumps({'first':first,'result':answer}),flush=True)
