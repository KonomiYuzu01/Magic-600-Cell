# Theory round 22, coverage round 4 (COV-23, with the computations used in COV-24, COV-25 and COV-27). At Y =
# solved·J, J = (0, 12), it classifies the off placements R·λ_y (R = J⁻¹ = element(I_a, 0, 45)) up to K⁺ by the
# orbits of H = {h in K⁺ : R·h·R⁻¹ in K⁺} (order 360) on all 177,120 chambers, and checks with exact vertex sets
# under all 7,200 rotations that no two classes are identified (CLASSIFICATION); certifies Alt on every SAFE orbit of
# Y with rim chambers by global Jordan certificates (JORDAN); tests the hole-access criterion of COV-24 over all
# landing letters and lists the hole placements that fail it (COVERAGE); computes the rigid blocks of Y and their
# locked chambers in cap 0 (LOCKS); enumerates the last move of the off piece for X's piece 1474 at element(I_a, 0, 25)
# and for piece 2249 at R (LAST_MOVE); and lists the outer placement classes with their mover caps (OUTER).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov23_singleton_classes.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov23_singleton_classes.py
import sys,runpy,io,contextlib,json,time,resource
from collections import Counter,defaultdict
from math import lcm,gcd
from functools import reduce
sys.argv=['depth','0','48']
with contextlib.redirect_stdout(io.StringIO()):
 n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov_depth_grip0.py')
resource.setrlimit(resource.RLIMIT_CPU,(1700,1720))
s,np,M,kp,reg,G=[n[k] for k in ['s','np','M','kp','reg','G']]
from exact import matmul,transpose,matvec
from sim.kplus import to_tuple
from sim.regions import transport_rows
from sim.kernel import sign5_vec,KAPPA_FORM
E=[s.restore.element(M,0,i) for i in range(60)];L=E[12];R=to_tuple(transpose(L));C=np.asarray(G.holes);cs=set(C.tolist());home=G.home;orb,sz=G.orbits();hc=np.bincount(orb[C],minlength=len(sz));deep=set(C[sz[orb[C]]==hc[orb[C]]].tolist())
H=[k for k in range(7200) if kp.index_of_matrix(to_tuple(matmul(matmul(R,kp.matrix(k)),L)))>=0];assert len(H)==360
left=set(range(G.NP));groups=[]
while left:
 y=min(left);xs=set(reg.piece_of[G.type[y],kp.compose_vec(np.array(H),np.full(len(H),reg.transport[y]))].tolist());assert xs<=left;left-=xs;ys=sorted(xs&cs);groups.append((ys[0] if ys else min(xs),ys))
def im(m):
 d=lcm(*(v.d for row in m for v in row));return np.array([[v.a*(d//v.d) for v in row] for row in m],np.int64),np.array([[v.b*(d//v.d) for v in row] for row in m],np.int64),d
ri,li=im(R),im(L)
def key(d,r):
 v=[int(d)]+list(map(int,r));q=reduce(gcd,v);return tuple(x//q for x in v)
def vk(f):return frozenset(key(f[0],r) for r in f[1])
V={y:transport_rows(ri,reg.vform(y)) for y,_ in groups};cent={y:key(d*len(v),np.array(v).sum(axis=0)) for y,(d,v) in V.items()};lookup=defaultdict(list)
for y,c in cent.items():lookup[(int(G.type[y]),c)].append(y)
ki=[kp.int_matrix(k) for k in range(7200)];a=np.array([v[0] for v in ki]);b=np.array([v[1] for v in ki]);den=np.array([v[2] for v in ki]);cross=[]
for y,_ in groups:
 v=cent[y];x=np.array(v[1:5]);z=np.array(v[5:]);arr=np.column_stack((v[0]*den,a@x+5*(b@z),b@x+a@z));arr//=np.gcd.reduce(arr,axis=1)[:,None]
 for k,row in enumerate(arr.tolist()):
  for q in lookup.get((int(G.type[y]),tuple(row)),[]):
   if q!=y and vk(transport_rows(ki[k],V[y]))==vk(V[q]):cross.append([y,q,k])
assert not cross
P=s.CL.classes()[0][0];rp={c:s.CL.pole(matvec(R,s.CL.N[c])) for c in P};hk=set(map(bytes,np.packbits(home[:,P],axis=1)))
def cell(y):return sorted(rp[c] for c in P if home[y,c])
def homecell(y):return np.packbits(np.isin(P,cell(y))).tobytes() in hk
counts=Counter((len(cell(y)),homecell(y),bool(set(ys)<=deep)) for y,ys in groups if ys)
print('CLASSIFICATION',json.dumps({'H':len(H),'all_classes':len(groups),'landing_classes':sum(bool(ys) for _,ys in groups),'counts':[[*k,v] for k,v in sorted(counts.items())],'mixed_pair_depth_classes':sum(bool(set(ys)&deep) and bool(set(ys)-deep) for _,ys in groups if ys),'no_extra_vertex_equivalences':True}),flush=True)
rim=C[sz[orb[C]]>hc[orb[C]]];target=np.array([int(rim[orb[rim]==o][0]) for o in sorted(set(orb[rim]))]);cert=s.B.jordan(G,target,seed=17);assert not cert['missing'] and not cert['uncertifiable_below_8']
for o,v in cert['certificates'].items():assert s.B.check_jordan(G,v['word'],int(o),v['prime'])['verified']
print('JORDAN',len(cert['certificates']),flush=True)
def labels(mi,y):
 e,v=transport_rows(mi,reg.vform(y));v=np.array(v,np.int64);n1,n2=s.CL.data.N1,s.CL.data.N2;aa=125*(v[:,:4]@n1.T+5*(v[:,4:]@n2.T))-2*e*KAPPA_FORM[0];bb=125*(v[:,4:]@n1.T+v[:,:4]@n2.T)-2*e*KAPPA_FORM[1];assert max(int(abs(aa).max()),int(abs(bb).max()))<2**29;ss=sign5_vec(aa,bb);return np.where((ss>=0).all(axis=0),1,np.where((ss<=0).all(axis=0),0,2))
tnorm={}
for t in range(12):
 for u in range(12):tnorm.setdefault(E.index(to_tuple(matmul(matmul(E[t],L),E[u]))),kp.index_of_matrix(E[t]))
assert len(tnorm)==48
f0=kp.index_of_matrix(M.frame(0));frames=[kp.compose(kp.index_of_matrix(M.frame(c)),int(kp.inv[f0])) for c in range(600)];cache={};exceptions=[];first_pass=0
for rep,ys in groups:
 rims=sorted(set(ys)-deep)
 if not rims:continue
 y=rims[0];t=int(G.type[y]);ts=np.flatnonzero(G.type==t)
 if t not in cache:
  edges=[]
  for c in range(600):
   xs=ts[home[ts,c]];src=np.searchsorted(ts,xs);es=[]
   for k in kp.gen_idx[2*c:2*c+2]:es.append((src,np.searchsorted(ts,reg.piece_of[t,kp.compose_vec(np.full(len(xs),k),reg.transport[xs])])))
   edges.append(es)
  cache[t]=edges
 sides=labels(ri,y);lab=np.arange(len(ts))
 while True:
  old=lab.copy()
  for c in np.flatnonzero(sides==0):
   for src,dst in cache[t][c]:
    v=np.minimum(lab[src],lab[dst]);np.minimum.at(lab,src,v);np.minimum.at(lab,dst,v)
  while not np.array_equal(lab,lab[lab]):lab=lab[lab]
  if np.array_equal(old,lab):break
 ok=np.isin(lab,lab[(orb[ts]==orb[y])&(~home[ts,0])])
 if ok.all():first_pass+=1;continue
 landers=0
 for c in np.flatnonzero(sides==1):
  for i in range(12,60):
   k=kp.index_of_matrix(to_tuple(matmul(s.restore.element(M,int(c),i),R)))
   if k<0:continue
   landers+=1;x=int(reg.piece_of[t,kp.compose(k,int(reg.transport[y]))]);j=kp.compose(frames[c],tnorm[i]);z=int(reg.piece_of[t,kp.compose(int(kp.inv[j]),int(reg.transport[x]))]);out=np.flatnonzero((orb==orb[z])&(~home[:,0]))
   if len(out):
    resources=reg.piece_of[t,kp.compose_vec(np.full(len(out),j),reg.transport[out])];assert not home[resources,c].any();ok|=np.isin(lab,lab[np.searchsorted(ts,resources)])
 exceptions.append([y,t,landers,ts[~ok].tolist()])
print('COVERAGE',json.dumps({'all_holes_at_selected_lander':first_pass,'exceptions_after_all_landers':exceptions,'exception_holes':sum(len(r[3]) for r in exceptions)}),flush=True)
ls=np.array([labels(li,int(p)) for p in C],np.uint8);blocks=s.BL.rigid_components(C,ls);locked=set();xblock=None
for bi,ps in enumerate(blocks):
 z=ls[np.searchsorted(C,ps)];ins=(z==1).all(axis=0);out=(z==0).all(axis=0);strad=(z==2).any(axis=0);assert (ins|out|strad).all();xs=C[home[C][:,ins].all(axis=1)&(~home[C][:,out]).all(axis=1)].tolist();locked.update(xs)
 if 2149 in xs:xblock=[bi,len(ps),int(ps[0]),np.flatnonzero(ins).tolist(),np.flatnonzero(strad).tolist(),xs]
dc=set().union(*(set(ys) for _,ys in groups if ys and set(ys)<=deep));assert locked==dc
print('LOCKS',json.dumps({'blocks':len(blocks),'locked_pairs':len(locked),'equals_all_deep_class_pairs':True,'x_block':xblock}),flush=True)
for p,pose in [(1474,E[25]),(2249,R)]:
 side=labels(im(pose),p);ct=Counter();zs=[]
 for c in np.flatnonzero(side==1):
  for i in range(1,60):
   k=kp.index_of_matrix(to_tuple(matmul(s.restore.element(M,int(c),i),pose)))
   if k<0:ct['retained_off_off' if i<12 else 'jumble_off_off']+=1;continue
   ct['landing']+=1;x=int(reg.piece_of[G.type[p],kp.compose(k,int(reg.transport[p]))]);j=kp.compose(frames[c],tnorm[i]);z=int(reg.piece_of[G.type[p],kp.compose(int(kp.inv[j]),int(reg.transport[x]))]);assert z in locked;zs.append(z)
 print('LAST_MOVE',json.dumps({'piece':p,'counts':dict(ct),'landing_chambers':sorted(set(zs))}),flush=True)
print('OUTER',[[y,int(G.type[y]),np.flatnonzero(labels(ri,y)==1).tolist()] for y,ys in groups if not ys],flush=True)
print('VERSIONS',sys.version.split()[0],np.__version__)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
