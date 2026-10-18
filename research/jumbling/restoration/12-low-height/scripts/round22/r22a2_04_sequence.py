# Theory round 22, line (a), round 2 (R22A2-01, 03, 04 and 06): at the depth 40 stop, the fourteen middles (ten census
# steps, the cores of b08, b09 and b10, b19's entry transport) with their Bad sets, ghost maps and landing chambers; the
# test of R22A2-02 for every ordered pair of the thirteen supplied steps (MATRIX) and the longest order of unchanged
# words (FIXED_WORDS_MAX); the resource search of R22A2-04 for fresh closures (RECLOSURE, RESOURCE_STEP, OFF_AFTER);
# the construction bounds (ALL_RECLOSURES_BOUND); N1 and N2 for the glued units' own cells (GLUE). The senior
# reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got the output in
# runs/round22/r22a2_04_sequence.txt (about 75 CPU seconds). Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22a2_04_sequence.py
import sys,json,runpy,io,contextlib,itertools,time,math,random
from pathlib import Path
S=Path('research/jumbling/restoration/12-low-height/scripts'); A=S.parent/'runs/round22/line-a'
sys.argv=[str(S/'round22/jordan_target.py'),str(A/'closures/b19-target.json')]
with contextlib.redirect_stdout(io.StringIO()): a=runpy.run_path(sys.argv[0])
G,B,E,np=a['G'],a['B'],a['screen'],a['np']; st0,M=a['st'],a['M']; N=G.NP; ar=np.arange(N); F=a['FG'].Frames()
assert a['cond']['necessary'] and set(a['found'])==a['want']
rows={}
for f in ('census-class3.json','census-class24.json'):
 d=json.loads((A/f).read_text()); assert d['digest']==st0.digest(); rows.update({k[-3:]:v for k,v in d['units'].items()})
names=sorted(n for n,r in rows.items() if isinstance(r.get('zero_bad'),dict) and r['zero_bad']['strict'])
names=sorted(names+['b08','b09','b10']); words={}; full={}; units={}; maps={}; land={}; need={}; consume={}; fullmaps={}; fullland={}; els={}
lat0=st0.lattice_flags(); off0=np.flatnonzero(~lat0); h0=np.bincount(G.orbit[G.holes],minlength=len(G.orbit_size))
def fresh():
 st=E.sim.State(st0.ctx,menu=M); st.pose_id=st0.pose_id.copy(); st._poses=list(st0._poses); st._lookup=dict(st0._lookup); return st
def ghost(w):
 g=a['FG'].Ghost(F,np.zeros(N,np.int64),ar)
 for c,j in w:
  if (c,j) not in els:els[c,j]=E.restore.element(M,c,j)
  g.apply(c,els[c,j],(c,j))
 return g.alive&(g.dom==0),g.q.copy()
for n in names+['b19']:
 U=np.array(json.loads((A/'units'/f'eb5-k40-k00-{n}.json').read_text())['pieces'],np.int64); units[n]=U
 if n in ('b08','b09','b10','b19'):
  w=json.loads((A/'closures'/f'{n}-core.json').read_text()); assert not w['P']; w=w['Q']
 else:w=rows[n]['zero_bad']['word']
 words[n]=w; good,q=ghost(w); bad=np.flatnonzero(~good); maps[n]=(good,q)
 assert set(G.orbit[bad])<=set(a['found'])
 crossing=int((G.orbit[q[good]]!=G.orbit[good]).sum()); assert crossing==0
 st=fresh(); R=E.restore.Restorer(st,M)
 for c,j in w:R.apply(c,j)
 assert st.lattice_flags()[U].all()
 other=np.setdiff1d(off0,U); assert all(st.pose(int(p))==st0.pose(int(p)) for p in other)
 rr=rows[n]['small_bad'] if n in ('b08','b09','b10','b19') else rows[n]['zero_bad']
 assert st.digest()==rr['exact']['digest_after']
 reg,kp=E.CL.ctx.regions,E.CL.ctx.kplus; K=st._kpose()[st.pose_id[U]]
 D=np.sort(reg.piece_of[reg.orbit_of[U],kp.compose_vec(K,reg.transport[U])]); land[n]=D
 need[n]=np.bincount(G.orbit[bad],minlength=len(h0)); consume[n]=np.bincount(G.orbit[D],minlength=len(h0))
 cond=B.condition(G,bad); assert cond['necessary']
 if n in ('b08','b09','b10'):
  fw=json.loads((A/'closures'/f'{n}-step.json').read_text())['word']; v=(len(fw)-len(w))//2; V=fw[:v]; assert fw[v:v+len(w)]==w
  assert fw[-v:]==G.pairs(G.inverse(G.numbers(V)))
  p=G.permutation(V); inv=np.argsort(p); fullmaps[n]=(good[p],inv[q[p]]); fullland[n]=np.sort(inv[D]); full[n]=fw
 else:fullmaps[n]=(good,q); fullland[n]=D; full[n]=w
 print('CORE',n,'letters',len(w),'Bad',len(bad),'occupied',int(G.occupied[bad].sum()),'good_cross_orbit',crossing,'good_moved',int((q[good]!=ar[good]).sum()),'unit',len(U),'cpu',round(time.process_time(),1),flush=True)
print('STATE',st0.digest(),'target_orbits',sorted(a['found']),flush=True)
fails={}; changed=[]
for i in names:
 gi,qi=fullmaps[i]; assert gi[G.occupied].all() and np.array_equal(np.sort(qi[G.occupied]),np.flatnonzero(G.occupied))
 Li=G.occupied.copy(); assert not Li[fullland[i]].any(); Li[fullland[i]]=True
 row=[]
 for j in names:
  if i==j:row.append(None); continue
  gj,qj=fullmaps[j]; c=int(Li[~gj].sum()); fails[i,j]=c; row.append(c)
  if not c and not np.array_equal(np.sort(qj[fullland[i]]),fullland[i]):changed.append((i,j))
  assert B.condition(G,np.flatnonzero(~gj),holes=np.flatnonzero(~Li))['necessary']
 print('MATRIX',i,row,flush=True)
assert not changed
m=len(names); dp={0:[]}
for mask in range(1<<m):
 if mask not in dp:continue
 for j,n in enumerate(names):
  if not(mask>>j&1) and all(fails[i,n]==0 for i in dp[mask]):dp.setdefault(mask|(1<<j),dp[mask]+[n])
best=max(dp,key=lambda z:z.bit_count())
print('FIXED_WORDS_MAX',best.bit_count(),'sequence',dp[best],'reachable_subsets',len(dp),flush=True)
for ns in (names,names+['b19']):
 m=len(ns); dp={0:[]}
 for mask in range(1<<m):
  if mask not in dp:continue
  h=h0.copy()
  for i,n in enumerate(ns):
   if mask>>i&1:h-=consume[n]
  assert (h>=0).all()
  for j,n in enumerate(ns):
   if not(mask>>j&1) and (h>=need[n]).all():dp.setdefault(mask|(1<<j),dp[mask]+[n])
 best=max(dp,key=lambda z:z.bit_count()); seq=dp[best]
 print('RECLOSURE',len(ns),'max',best.bit_count(),'sequence',seq,'reachable_subsets',len(dp),'core_letters',sum(len(words[n]) for n in seq),flush=True)
 h=h0.copy()
 for n in seq:
  nz=np.flatnonzero(need[n]); print('RESOURCE_STEP',n,'off_before',int(h.sum()),'minimum_slack',int((h[nz]-need[n][nz]).min()),'orbit35',int(h[35]),'orbit35_need',int(need[n][35]),'orbit35_consumed',int(consume[n][35]),flush=True); h-=consume[n]
 print('OFF_AFTER',int(h.sum()),flush=True)
print('RESOURCE_ORBITS',sorted(a['found']),flush=True)
for n in names+['b19']:
 os=sorted(a['found']); print('DEMAND_CONSUMPTION',n,need[n][os].tolist(),consume[n][os].tolist(),flush=True)
weights={}
for o,(stage,trial,cert) in sorted(a['found'].items()):
 length=(120,400,800,1500)[stage]; rng=random.Random(1+stage)
 for _ in range(trial):jw=[rng.randrange(len(G.letters)) for _ in range(length)]
 assert B.check_jordan(G,G.pairs(jw),o,cert['prime'])==cert
 lab,lens=B.cycles(G.perm(jw)); Mj=math.lcm(*[int(x) for x in lens if x and x!=cert['prime']]); d=int(G.orbit_size[o]); p=cert['prime']
 for c,j,_,ys in G.letters:
  xs=G.capx[c]; r=int(((G.orbit[xs]==o)&(ys!=xs)).sum())
  if 2<=r and 2*r-1<=d:break
 else:raise AssertionError(o)
 seed=4+4*(length*Mj+2*(d-p))*(math.comb(d,r)-1); e=(d-2)*(2*d-1); f=(d-3)*(2*d-3)*(2*d-1); weights[o]=seed+2*e+2*f
for ns in (names,names+['b19']):
 bound=sum(len(words[n])+4*sum(int(need[n][o])*weights[o] for o in weights) for n in ns)
 print('ALL_RECLOSURES_BOUND',len(ns),bound,'digits',len(str(bound)),flush=True)
members,_=E.CL.classes()
for n in ('b03','b04','b05','b06','b12','b15','b18'):
 T=B.cell_chambers(G.home,members,2 if n=='b18' else 3,rows[n]['cell']); c=B.condition(G,T)
 print('GLUE',n,'fixed_occupied',len(c['fixed_occupied']),'nontrivial_deficits',[(z['orbit'],int(G.orbit_size[z['orbit']]),z['have'],z['need']) for z in c['necessary_deficits'] if G.orbit_size[z['orbit']]>1],flush=True)
print('DONE_CPU',round(time.process_time(),2),flush=True)
