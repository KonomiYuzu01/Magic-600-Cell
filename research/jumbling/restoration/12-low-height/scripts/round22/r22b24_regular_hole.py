# Theory round 22, line (b), round 5 (R22B-24): from the solved state, A = [(26,45),(126,2),(26,12),(126,5)]^3 and
# eight more twists reach a height-2 state whose twelve-chamber orbit of chamber 8830 has an induced group of order
# exactly 12 (regular, so no Alt(12)) and one hole. The class-3 triangle cell {26,99,126} gives a 111-chamber target
# that is the complete Bad set of the transport word A^-1, meets that orbit, passes N1 and N2, and is vacated by the
# single self-inverse SAFE letter (6,1); A^-1 then lands the whole 111-piece content with no collateral.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22b24_regular_hole.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22b24_regular_hole.py
import sys,json,resource,signal,time,hashlib
from pathlib import Path
from types import SimpleNamespace
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(1150,1200))
def stop(*a):print('CUT_SHORT_CPU_1150_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s
from exact import transpose
B=s.B;M=s.restore.menu('I_a')
def apply(st,word):
 for c,i in word:
  r=st.apply(s.sim.Twist(s.CL.ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}))
  assert r.applied and not r.uncertain
A=[[26,45],[126,2],[26,12],[126,5]]*3
st=s.sim.State(s.CL.ctx,menu=M);apply(st,A);U=B.off_pieces(st);T=B.hole_set(st)
apply(st,[[c,12] for c in [15,100,105,116,84,132,238]]+[[6,1]])
assert st.digest()=='3cb419d7d60c999658a3c52b2382ed4e6bfb4e4407e3dffb27919d6bab610f35'
heights={int(s.T.dist(s.T.factors(st._pose_matrix(p))[0])) for p in np.unique(st.pose_id[B.off_pieces(st)])};assert heights=={2}
G=B.Alphabet(SimpleNamespace(st=st),M);orbit,sizes=G.orbits();D=np.flatnonzero(orbit==orbit[8830])
ident=tuple(range(len(D)));gens={tuple(np.searchsorted(D,G.images([n],D)).tolist()) for n in range(len(G.letters))}
group={ident};todo=[ident]
for p in todo:
 for g in gens:
  q=tuple(g[x] for x in p)
  if q not in group:group.add(q);todo.append(q)
assert len(D)==len(group)==12
X=G.capx[6];assert set(G.safe)&set(np.flatnonzero(G.home[X].any(axis=0)))=={6}
actions={tuple(range(len(X)))}|{tuple(np.searchsorted(X,G.images([n],X)).tolist()) for n in G.by_grip[6]}
assert len(actions)==12 and all(tuple(q[x] for x in p) in actions for p in actions for q in actions)
members,cls=s.CL.classes();assert np.array_equal(B.cell_chambers(G.home,members,3,[26,99,126]),T)
cond=B.condition(G,T);assert cond['necessary'] and cond['fixed_vacant']
passing=[G.pairs(w) for w in [[]]+[[n] for n in G.by_grip[6]] if not G.occupied[G.images(w,T)].any()]
assert passing==[[[6,1]]] and G.inv[G.index[(6,1)]]==G.index[(6,1)]
print('COUNTEREXAMPLE_TO_UNIVERSAL_ALT',st.digest(),'off',len(G.holes),'heights',sorted(heights),'safe',len(G.safe),'letters',len(G.letters),'D',D.tolist(),'order',len(group),'holes',D[~G.occupied[D]].tolist(),flush=True)
print('TARGET','class',3,'cell',[26,99,126],'size',len(T),'occupied',int(G.occupied[T].sum()),'digest',hashlib.sha256(json.dumps(T.tolist()).encode()).hexdigest(),'N1_N2',True,'joint_actions',len(actions),'passing',passing,flush=True)
Wi=[]
for c,i in reversed(A):
 inv=s.restore.to_tuple(transpose(s.restore.element(M,c,i)))
 js=[j for j in range(len(M.items)) if s.restore.to_tuple(s.restore.element(M,c,j))==inv];assert len(js)==1;Wi.append([c,js[0]])
ghost=s.sim.State(s.CL.ctx,menu=M);apply(ghost,Wi);assert np.array_equal(B.off_pieces(ghost),T)
domains=s.rim.Domains(st);dom=np.asarray(domains.dom);d=int(dom[U[0]]);assert d and np.all(dom[U]==d)
dcls={}
for dd in range(1,len(domains.reps)):
 h=s.CL.hub(domains.reps[dd],members,cls);dcls[dd]=h[0][1] if len(h)==1 else None
assert dcls[d]==3
cand=np.flatnonzero(dom==d);keep=np.ones(len(cand),bool)
for c in members[3]:
 cl=st.classify(int(c),need_inside=True,stop_at_straddle=False);assert not cl.uncertain
 isin=np.isin(U,cl.inside);assert (isin==isin[0]).all()
 keep &= (np.isin(cand,cl.inside)==isin[0])
 if c in [26,126]:assert cl.straddle is None and all(dom[p]==0 or dcls[int(dom[p])]==3 for p in cl.inside)
assert np.array_equal(cand[keep],U)
off=B.off_pieces(st);old={int(p):st._poses[int(st.pose_id[p])].key for p in off}
apply(st,Wi);end=B.off_pieces(st);others=np.setdiff1d(off,U)
assert not np.isin(U,end).any() and all(st._poses[int(st.pose_id[p])].key==old[int(p)] for p in others)
print('CATALOGUE_CORE','word',Wi,'whole_content',len(U),'landed',len(U),'other_off_exact',len(others),'collateral',len(np.setdiff1d(end,off)),'Bad_equals_target',True,'free_class',3,flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
