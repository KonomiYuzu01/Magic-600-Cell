# Theory round 22, line (b), round 6 (R22B-32): in the nine recorded target orders of E2's certificate steps no power
# of a four-letter commutator [x,y] of single SAFE menu letters is a seed. After normalizing x to grip 0 by K+, the
# 6,897 pairs with overlapping caps are tested exactly; the other 65,703 commute.
# The senior reviewer's exact computation (R22B-30 with R22B-32's block appended), kept as written apart from this
# header; the integrator re-ran it and got the output in runs/round22/r22b32_pairs.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22b32_pairs.py
bi={1:(190,10),2:(190,2),4:(219,2),11:(95,11),12:(95,4),15:(197,11),21:(116,11),22:(116,4),23:(282,11),27:(180,10),30:(104,7),31:(139,11),35:(307,1),36:(307,4),37:(140,11),38:(140,4),40:(103,4),43:(311,11),49:(42,10),50:(42,7),55:(338,1),57:(259,1),59:(37,1),64:(80,3),71:(290,11),73:(117,11),75:(407,1),76:(407,2),77:(183,1),91:(152,10),99:(136,11),101:(272,11),103:(209,1),107:(292,10),112:(358,2),113:(356,11),116:(58,7),117:(446,11),121:(273,10),128:(304,2),133:(300,11),139:(121,11),145:(384,11),147:(208,10),148:(208,7),149:(232,10),154:(175,3),181:(105,11),183:(237,10),184:(237,7),199:(453,10),255:(25,10),259:(220,1),293:(141,10),310:(244,4),380:(362,2),381:(567,1),466:(468,2)}
ii={1:1,2:5,3:9,4:8,5:2,6:7,7:6,8:4,9:3,10:10,11:11}
def wi(w):return [(g,ii[i]) for g,i in w[::-1]]
def wc(x,y):return x+y+wi(x)+wi(y)
def LL(*xs):
 def at(x):return [bi[x]] if isinstance(x,int) else x
 w=at(xs[0])
 for x in xs[1:]:w=wc(w,at(x))
 return w
AA=LL(1,4)*4;DD=LL(1,12)*4
rw={25:LL(2,40,22),64:LL(2,40,116,2,50),67:LL(1,4,31,99),69:LL(1,12,64,148,71),16:LL(1,12,64,35,73,121),70:LL(1,12,64,148,77),17:LL(1,12,64,35,73,11,121),105:LL(AA,38,36,21,76,77,117,91,380,381),81:LL(1,12,64,148,183),41:LL(AA,38,36,255),31:LL(1,12,64,35,73,11,121,27,101),22:LL(1,12,64,35,73,11,121,27,57),116:LL(1,49,37,91,55,199,77),75:LL(1,4,31,99,107,112,154,128,139,103),82:LL(1,12,64,148,149),74:LL(1,4,184,15,91),33:LL(DD,30,77),23:LL(1,12,64,35,73,121,259,466,293,181),32:LL(1,12,64,35,73,11,121,27,149),115:LL(1,4,31,113,43),108:LL(1,4,184,15,133,50),106:LL(DD,30,113,23),83:LL(1,12,64,148,149,147),90:LL(1,4,184,15,91,71),34:LL(AA,38,30,75),66:LL(DD,30,113,59),65:LL(1,12,64,35,73,121,101,293,259,310),14:LL(AA,38,36,31,75),12:LL(LL(AA,38,1)*4,145)}
import sys,json,gzip,hashlib,math
from pathlib import Path
from types import SimpleNamespace
from collections import Counter,defaultdict
from functools import lru_cache
import numpy as np
S=Path('research/jumbling/restoration/12-low-height/scripts').resolve()
sys.path.insert(0,str(S))
import screen,buffer as B
R=S.parent/'runs/round22/line-a/replay2'
specs=[
('b14','run1/z-b12.json.gz','run1/b14-target.json','run1/b14-cert.json.gz',60),
('b16','run1/z-b14.json.gz','run1/b16-target.json','run1/b16-cert.json.gz',72),
('b20','run1/z-b16.json.gz','run1/b20-target.json','run1/b20-cert.json.gz',36),
('b21','run1/z-b20.json.gz','run1/b21-target.json','run1/b21-cert.json.gz',84),
('b07','run2/03-b07-z1-table.json.gz','run2/03-b07-v-target.json','run2/03-b07-v-cert.json.gz',96),
('b13','run2/04-b13-z1-table.json.gz','run2/04-b13-v-target.json','run2/04-b13-v-cert.json.gz',120),
('b19','run2/05-b19-z1-table.json.gz','run2/05-b19-v-target.json','run2/05-b19-v-cert.json.gz',168),
('b15','run2/09-b15-z1-table.json.gz','run2/09-b15-v-target.json','run2/09-b15-v-cert.json.gz',864),
('b06+b05','b06-71/t1/b06-14-z1-table.json.gz','b06-71/t1/b06-14-v-target.json','b06-71/t1/b06-14-v-cert.json.gz',5)]
stateinfo={};certs={};perms={};catalogue={}
for name,sp,tp,cp,constant in specs:
 st,M,_=screen.load('table',str(R/sp))
 G=B.Alphabet(SimpleNamespace(st=st),M);oo,sz=G.orbits()
 cc=json.load(gzip.open(R/cp));tt=json.loads((R/tp).read_text());T=tt['T']
 assert st.digest()==tt['state_digest']==cc['state_digest']
 assert hashlib.sha256(json.dumps(sorted(T)).encode()).hexdigest()==tt['T_digest']==cc['T_digest']
 labs=sorted(set(oo[T]),key=lambda a:(-int(sz[a]),int(G.orbit_least[a])))
 assert [(int(G.orbit_least[a]),int(sz[a]),int(sum(oo[x]==a for x in T))) for a in labs if sz[a]>1]==[(r['least'],r['size'],r['k']) for r in cc['orbits']]
 cond=B.condition(G,T);assert cond['fixed_vacant'] and cond['necessary']
 by={r['least']:r for r in cc['orbits']}
 assert set(by)<=set(rw)
 prior=np.zeros(G.NP,bool);order=[a for a in rw if a in by]
 for a in order:
  w=rw[a];G.numbers(w)
  p=perms.setdefault(a,G.permutation(w)) if a not in perms else perms[a]
  D=np.flatnonzero(oo==oo[a]);mv=D[p[D]!=D]
  assert len(mv)==3 and np.array_equal(p[prior],np.flatnonzero(prior))
  u=int(mv[0]);cy=(u,int(p[u]),int(p[p[u]]))
  assert p[cy[2]]==u and set(cy)==set(mv)
  prior[D]=True
  value=(len(w),cy)
  if a in catalogue:assert catalogue[a]==value
  catalogue[a]=value
 v=2*sum(r['k']*(len(rw[a])+8*r['e']+4*r['f']+2) for a,r in by.items())
 print('SEEDS_AND_BOUND',name,len(order),v,2*v+constant,flush=True)
 stateinfo[name]=dict(digest=st.digest(),safe=set(G.safe),orbit=oo.copy(),sizes=sz.copy(),least=G.orbit_least.copy(),target=T)
 certs[name]=cc
G0=B.Alphabet(SimpleNamespace(st=B.sim.State(B.CL.ctx,menu=M)),M)
common_safe=set.intersection(*(v['safe'] for v in stateinfo.values()))
print('COMMON_SAFE',len(common_safe))
for a in rw:print('CATALOGUE',a,*catalogue[a],[n for n,c in certs.items() if a in {r['least'] for r in c['orbits']}])
kp=B.CL.ctx.kplus;mo=B.CL.ctx.data.orbit_id
atlas=json.loads(Path('assets/seed_atlas.json').read_text())
cascade=[6,0,17,15,2,22,21,8,23,9,29,25,13,11,10,34,28,26,24,16,14,12,3,33,32,31,30,27,20,19,18,7,5,4,1]
def expand(t):
 if t[0]=='g':return [t[1]]
 if t[0]=='pow':return expand(t[1])*t[2]
 x,y=expand(t[1]),expand(t[2]);return x+y+[-v for v in x[::-1]]+[-v for v in y[::-1]]
allowed=np.zeros(600,bool);allowed[list(common_safe)]=True
grips=sorted({(abs(m)-1)//2 for a in atlas for m in expand(a['ast'])})
sigmas=np.flatnonzero(allowed[kp.perms[:,grips]].all(axis=1));sig=int(sigmas[0])
lookup={(int(g),int(k)):n for n,(g,i,k,ys) in enumerate(G0.letters)}
def rotated(t):
 w=np.asarray(expand(t),np.int64);ks=kp.gen_idx[np.abs(w)-1]
 ks=np.where(w>0,ks,kp.inv[ks])
 ks=kp.compose_vec(np.full(len(w),sig),kp.compose_vec(ks,np.full(len(w),int(kp.inv[sig]))))
 gs=kp.perms[sig,(np.abs(w)-1)//2]
 return [lookup[(int(g),int(k))] for g,k in zip(gs,ks)]
found={};prior=set();identity=np.arange(G0.NP)
for o in cascade:
 t=atlas[o]['ast'];prefix=[]
 while True:
  prefix.append(t)
  if t[0]=='g':break
  t=t[1]
 for t in prefix[::-1]:
  nums=rotated(t);p=G0.perm(nums);mv=np.flatnonzero(p!=identity)
  own=mv[mo[mv]==o]
  if len(own)==3 and not(set(mo[mv].tolist())&prior):
   a=int(own[0]);assert p[p[p[a]]]==a
   found[int(G0.type[a])]=G0.pairs(nums);break
 else:raise AssertionError(('no seed',o))
 prior.add(o)
for a,w in rw.items():assert found[int(G0.type[a])]==[list(x) for x in w]
print('FINDER',len(sigmas),sig,len(rw),'PASS',flush=True)
def sparse_cycles(p):
 xs=np.flatnonzero(p!=np.arange(len(p)))
 if not len(xs):return xs,np.array([],dtype=np.int64)
 pp=np.searchsorted(xs,p[xs]);lab,ln=B.cycles(pp);ix=np.flatnonzero(ln)
 return xs[ix],ln[ix]
assert len(set(kp.perms[:,0].tolist()))==600
for g,nums in G0.by_grip.items():
 assert {G0.letters[n][2] for n in nums}==set(np.flatnonzero(kp.perms[:,g]==g).tolist())-{0}
typeorders={name:[int(G0.type[r['least']]) for r in certs[name]['orbits']] for name in certs}
for name,info in stateinfo.items():
 for r in certs[name]['orbits']:
  o=info['orbit'][r['least']];t=int(G0.type[r['least']])
  assert np.all((info['orbit'][G0.type==t]==o)|(info['sizes'][info['orbit'][G0.type==t]]==1))
near=[g for g in G0.by_grip if G0.home[G0.capx[0],g].any()]
valid=Counter();checked=0
for x in G0.by_grip[0]:
 for g in near:
  for y in G0.by_grip[g]:
   p=G0.perm([x,y,int(G0.inv[x]),int(G0.inv[y])])
   reps,lens=sparse_cycles(p);tt=G0.type[reps];checked+=1
   for name,order in typeorders.items():
    ids=[t for t in order if np.any((tt==t)&(lens%3==0))]
    if not ids:continue
    ix=np.flatnonzero((tt==ids[0])&(lens%3==0))
    if len(ix)==1 and lens[ix[0]]==3:valid[name]+=1
print('EXHAUSTIVE_XY','near',len(near),'checked',checked,'omitted_disjoint',(600-len(near))*121,'valid',dict(valid))
