# Theory round 22, R22C2-02 (line (c)): the blockers of the fixed landing chambers in experiments A and B.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c2_02_blockers.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22c2_02_blockers.py
import sys,json,resource,time
from pathlib import Path
from types import SimpleNamespace
from collections import defaultdict,Counter
resource.setrlimit(resource.RLIMIT_CPU,(1100,1100))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s
import numpy as np
B=s.B; CL=s.CL
root=Path('research/jumbling/restoration/12-low-height'); run=root/'runs/round22/line-c'
home=CL.data.sig_matrix(np.arange(len(CL.ctx.regions.orbit_of)))
def apply(st,M,word):
 for c,i in word:
  o=st.apply(s.sim.Twist(s.restore.ctx,int(c),s.restore.element(M,int(c),int(i)),'menu',{'menu':M.name}))
  assert o.applied and not o.uncertain
for name,k,seed,uid,A in [('b15',40,7101,15,[]),('u130',60,7104,130,[[481,5]]),('u180',60,7104,180,[[481,1]])]:
 st,M,_=s.load('record',f'research/jumbling/restoration/11-scramble-census/runs/round2/ia-k{k}-s{seed}.json')
 bl=s.BL.Blocks(SimpleNamespace(st=st),M);dm=np.asarray(s.rim.Domains(st).dom)
 units=[]
 for pcs in bl.blocks:
  pcs=np.asarray(pcs)
  for d in np.unique(dm[pcs]).tolist():units.append(set(pcs[dm[pcs]==d].tolist()))
 U=units[uid];initial_off=set(B.off_pieces(st).tolist());unit_of={p:u for u,pcs in enumerate(units) for p in pcs}
 if name=='b15':
  apply(st,M,json.loads((root/'runs/round22/t1prime-k40/steps/u12.json').read_text())['word'])
  assert st.digest()=='4a0f42717312551e7f531ad3f57de63f0da61236e0b8d54050681d08a2ac0093'
  apply(st,M,json.loads((run/'w14-census.json').read_text())['word'])
 else:apply(st,M,A)
 off0=set(B.off_pieces(st).tolist());collateral=off0-initial_off
 W=json.loads((run/f'{name}-W.json').read_text());assert st.digest()==W['input_digest']
 apply(st,M,W['word']);assert st.digest()==W['output_digest']
 off1=set(B.off_pieces(st).tolist());Q=off1-off0
 assert not U&off1
 D=s.rim.Domains(st);landing=sorted(int(x) for x in D.cham[sorted(U)])
 record=json.loads((run/('exp-a.json' if name=='b15' else 'exp-b.json')).read_text())
 fixed=record['b15' if name=='b15' else str(uid)]['closure']['premises']['fixed_occupied']
 assert set(fixed)<=set(landing)
 final=s.BL.Blocks(SimpleNamespace(st=st),M)
 def group(p):return 'Q' if p in Q else ('collateral' if p in collateral else 'u'+str(unit_of[p]))
 caps=sorted({int(c) for x in landing for c in np.flatnonzero(home[x])})
 caprows={}
 for c in caps:
  labels=final.labels[:,c];groups=defaultdict(lambda:defaultdict(list))
  for j in np.flatnonzero(labels):groups[group(int(final.off[j]))]['inside' if labels[j]==1 else 'straddle'].append(int(final.off[j]))
  caprows[c]={g:{side:{'count':len(ps),'first':ps[0]} for side,ps in sides.items()} for g,sides in sorted(groups.items())}
 recomputed=[x for x in landing if all(caprows[int(c)] for c in np.flatnonzero(home[x]))]
 assert recomputed==sorted(fixed),(name,recomputed,fixed)
 qfixed=[x for x in landing if all('Q' in caprows[int(c)] for c in np.flatnonzero(home[x]))]
 bycaps=defaultdict(list)
 for x in fixed:bycaps[tuple(int(c) for c in np.flatnonzero(home[x]))].append(x)
 qfree=Counter()
 for x in fixed:
  ss=[frozenset(caprows[int(c)]) for c in np.flatnonzero(home[x]) if 'Q' not in caprows[int(c)]]
  mins={a for a in ss if not any(b<a for b in ss)}
  qfree.update(tuple(sorted(a)) for a in mins)
 print(json.dumps({'name':name,'digest':st.digest(),'Q':len(Q),'prior_collateral':len(collateral),'landed':len(U),'fixed':len(fixed),'Q_only_fixed':qfixed,'fixed_chambers_by_caps':[[list(c),xs] for c,xs in sorted(bycaps.items())],'caps':caprows,'minimal_Qfree_blockers':[[list(a),n] for a,n in sorted(qfree.items())]}),flush=True)
print('completed_without_cutoff',flush=True)
