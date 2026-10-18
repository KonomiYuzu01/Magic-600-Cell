# Theory round 22, line (c), round 3 (R22C3-03): an independent audit of experiment D at the depth 60 stop:
# literal replay of the steps of 174, 175 and 176; for 180, H1 to H3 of T1' with and without the preparation (481, 1),
# the target T rebuilt by a ghost replay, N1 and N2, and H4 by evaluating the certificate's V' on every target orbit.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c3_audit_d.txt (about 6 CPU minutes).
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22c3_audit_d.py
import sys,json,gzip,hashlib,resource,signal,time
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(1100,1110))
def stop(*a):print('CUT_SHORT_CPU_1100_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
S=Path('research/jumbling/restoration/12-low-height');E=S/'runs/round22/line-c/exp-d'
sys.path[:0]=[str((S/'scripts').resolve()),str(Path('research/jumbling/restoration/09-general-method/scripts/echain').resolve())]
import screen as s,fastghost as FG,check_r22b10 as CK,numpy as np
from exact import transpose
from sim.kplus import to_tuple
B=s.B
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json')
def read(p):return json.loads(p.read_text())
def pose(p):return st._poses[int(st.pose_id[p])].key
def apply(word):
 for c,i in word:
  o=st.apply(s.sim.Twist(s.restore.ctx,int(c),s.restore.element(M,int(c),int(i)),'menu',{'menu':M.name}))
  assert o.applied and not o.uncertain
for u in [174,175,176]:
 U=np.array(read(E/f'units/u{u}.json')['pieces']);off=B.off_pieces(st);old={p:pose(p) for p in off};z=st.digest()
 W=read(E/f'word-{u}.json')['word'];apply(W);end=B.off_pieces(st);others=np.setdiff1d(off,U)
 assert not np.isin(U,end).any() and not len(np.setdiff1d(end,off)) and all(pose(p)==old[p] for p in others)
 r=read(E/f'step-{u}.json');assert z==r['start_digest'] and st.digest()==r['end_digest']
 print('STRICT',u,len(W),len(off),len(end),st.digest(),flush=True)
base={'poses':[B.matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],'pose_id':st.pose_id.tolist(),'digest':st.digest(),'journal':[]}
U=np.array(read(E/'units/u180.json')['pieces']);members,cls=s.CL.classes()
for case,A in [('e180-none',[]),('e180-481-1',[[481,1]])]:
 st=s.sim.State(s.CL.ctx,menu=M);s.O.set_pose_table(st,base)
 for c,i in A:assert s.CL.ctx.kplus.index_of_matrix(s.restore.element(M,c,i))>=0
 apply(A);wd=read(E/case/'cp4-word.json');W=wd['word'];assert st.digest()==wd['input_digest']
 D=s.rim.Domains(st);dom=np.asarray(D.dom);d=int(dom[U[0]]);assert d and np.all(dom[U]==d)
 h=s.CL.hub(D.reps[d],members,cls);assert len(h)==1;P=h[0][1]
 own=[dd for dd in range(1,len(D.reps)) if (hh:=s.CL.hub(D.reps[dd],members,cls)) and len(hh)==1 and hh[0][1]==P]
 for c in {c for c,i in W}:
  cl=st.classify(c,need_inside=True,stop_at_straddle=False)
  assert c in members[P] and cl.straddle is None and not cl.uncertain and np.isin(dom[np.asarray(cl.inside,np.int64)],[0]+own).all()
 candidates=np.flatnonzero(dom==d);mask=np.ones(len(candidates),bool)
 for c in members[P]:
  cl=st.classify(int(c),need_inside=True,stop_at_straddle=False)
  mask &= (np.isin(candidates,cl.inside)==bool(int(U[0]) in cl.inside))
 assert np.array_equal(np.sort(candidates[mask]),np.sort(U))
 off=B.off_pieces(st);old={p:pose(p) for p in off};apply(W);end=B.off_pieces(st);others=np.setdiff1d(off,U)
 assert not np.isin(U,end).any() and all(pose(p)==old[p] for p in others)
 assert st.digest()==wd['output_digest']
 Q=np.setdiff1d(end,off);assert sorted(Q.tolist())==sorted(read(E/case/'u-Q.json')['pieces'])
 Wi=read(E/case/'u-winv.json')['Q'];assert len(Wi)==len(W)
 for (c,i),(cc,j) in zip(reversed(W),Wi):assert c==cc and to_tuple(transpose(s.restore.element(M,c,i)))==to_tuple(s.restore.element(M,cc,j))
 gh=FG.Ghost(FG.Frames(),np.zeros(len(st.pose_id),np.int64),np.arange(len(st.pose_id)))
 for c,i in Wi:gh.apply(c,s.restore.element(M,c,i),(c,i))
 T=np.flatnonzero(~gh.alive|(gh.dom!=0));G=B.Alphabet(SimpleNamespace(st=st),M);G.orbits();co=B.condition(G,T)
 print('MIDDLE',case,len(W),'landed',len(U),'other_off_exact',len(others),'Q',len(Q),'off',len(end),'T',len(T),'occupied',int(G.occupied[T].sum()),'N1',co['fixed_vacant'],'N2',co['necessary'],st.digest(),flush=True)
 if not A:
  assert not co['fixed_vacant'] and not co['necessary'];continue
 target=read(E/'e180-cert/e180-target.json');assert T.tolist()==target['T'] and hashlib.sha256(json.dumps(T.tolist()).encode()).hexdigest()==target['T_digest']
 assert co['fixed_vacant'] and co['necessary']
 with gzip.open(E/'e180-cert/e180-cert.json.gz','rt') as f:cert=json.load(f)
 assert cert['state_digest']==target['state_digest']==st.digest() and cert['T_digest']==target['T_digest']
 lengths,children,leaves=CK.program(G,cert['nodes']);V=CK.node_id(cert['V'],cert['nodes'])
 assert lengths[V]==cert['V_length'] and lengths[V]<10**162
 fixed=T[G.orbit_size[G.orbit[T]]==1];assert not G.occupied[fixed].any()
 want={int(o) for o in G.orbit[T] if G.orbit_size[o]>1};got=set()
 for row in cert['orbits']:
  o=int(G.orbit[row['least']]);assert o in want and o not in got;got.add(o)
  DD=np.flatnonzero(G.orbit==o);assert len(DD)==row['size'] and row['k']==int(sum(G.orbit[T]==o))
  B.check_jordan(G,row['jordan_word'],o,row['p'])
  ev=CK.Evaluator(G,DD,cert['nodes'],children,leaves);p=ev.evaluate(V);occ=np.empty(len(DD),bool);occ[p]=G.occupied[DD]
  assert not occ[np.searchsorted(DD,T[G.orbit[T]==o])].any()
  assert lengths[row['seed']]==row['s']
  print('ORBIT_PASS',o,len(DD),flush=True)
 assert got==want
 bound=sum(2*r['k']*(r['s']+2*r['e']+2*r['f']) for r in cert['orbits'])
 assert bound==cert['bound'] and lengths[V]<=bound
 print('H4_PASS','nontrivial_orbits',len(got),'fixed_targets',len(fixed),'steps',len(cert['steps']),'V_length',lengths[V],flush=True)
print('PASS_CPU',round(time.process_time(),1),flush=True)
