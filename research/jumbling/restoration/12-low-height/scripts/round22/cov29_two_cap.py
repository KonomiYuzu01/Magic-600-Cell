# Theory round 22, coverage round 5 (COV-29): a four-core restoration for each of COV-24's 24 exceptional class–hole
# pairs. With D = (596, 12) at the cap antipodal to cap 0 (CREATOR_CAP), it builds each singleton (the class's piece y
# at R = element(I_a, 0, 45), hole h), applies D·J exactly and checks the holes (cap 0 without y, plus cap 596) and the
# 6,193 off pieces; it then finds a resource chamber a in cap 596 lying both in the SAFE orbit of y at the first middle
# and in the SAFE orbit of h under G(solved·D), and certifies Alt on both orbits by global Jordan certificates
# (TWO_CAP_PASS rows: y, h, a, the two orbit sizes and the two certified primes). The retained words V and V₂ that the
# certificates prove to exist are not expanded.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov29_two_cap.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov29_two_cap.py
import sys,time,json,resource,signal
from pathlib import Path
from collections import Counter
from math import lcm
resource.setrlimit(resource.RLIMIT_CPU,(1700,1720))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul
from sim.kplus import to_tuple
from sim.regions import transport_rows
from sim.kernel import sign5_vec,KAPPA_FORM
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
def im(m):
 d=lcm(*(v.d for row in m for v in row))
 return np.array([[v.a*(d//v.d) for v in row] for row in m],np.int64),np.array([[v.b*(d//v.d) for v in row] for row in m],np.int64),d
def labels(mi,y):
 e,v=transport_rows(mi,reg.vform(y));v=np.array(v,np.int64);aa=125*(v[:,:4]@s.CL.data.N1.T+5*v[:,4:]@s.CL.data.N2.T)-2*e*KAPPA_FORM[0];bb=125*(v[:,4:]@s.CL.data.N1.T+v[:,:4]@s.CL.data.N2.T)-2*e*KAPPA_FORM[1]
 assert max(int(abs(aa).max()),int(abs(bb).max()))<2**29
 ss=sign5_vec(aa,bb);return np.where((ss>=0).all(axis=0),1,np.where((ss<=0).all(axis=0),0,2))
from types import SimpleNamespace
s.B.log=lambda **kw:None
home=s.CL.data.sig_matrix(np.arange(len(reg.orbit_of)));typ=np.asarray(reg.orbit_of);NP=len(home)
allcaps=[np.flatnonzero(home[:,c]) for c in range(600)];two={}
for c in range(600):
 rows=[]
 for k in kp.gen_idx[2*c:2*c+2]:
  xs=allcaps[c];ys=reg.piece_of[typ[xs],kp.compose_vec(np.full(len(xs),k),reg.transport[xs])]
  i=next(i for i in range(1,12) if kp.index_of_matrix(s.restore.element(M,c,i))==k)
  rows.append((c,i,int(k),ys))
 two[c]=rows
def group(safe):
 g=s.B.Alphabet.__new__(s.B.Alphabet);g.NP=NP;g.home=home;g.type=typ;g.safe=list(map(int,safe));g.letters=[l for c in g.safe for l in two[c]];g.capx={c:allcaps[c] for c in g.safe};g.index={(l[0],l[1]):j for j,l in enumerate(g.letters)};g.orbits();return g
pairs={1529:[2548],2136:[3247],2328:[3238,3730,3887],2335:[3239],2648:[2352],2651:[2336],2656:[2350],2878:[2328,4675,4772],2890:[2335],2891:[2336],2905:[2329,4685,4780],2953:[2343],3730:[3904],3732:[3798],3805:[541,669,1464],3807:[543]}
J=s.restore.element(M,0,12);R=s.restore.element(M,0,45);d=int(s.CL.pole(tuple(-x for x in s.CL.N[0])));print('CREATOR_CAP',d,flush=True);D=s.restore.element(M,d,12);C=allcaps[0];E=allcaps[d]
assert not np.intersect1d(C,E).size
ql=np.array([labels(im(J),int(q)) for q in C]);dl=np.array([labels(im(D),int(q)) for q in E]);assert (dl[:,0]==0).all()
G2=group(np.flatnonzero((dl==0).all(axis=0)));o2,z2=G2.orbits();cache={};certcache={};rows=[]
def certify(g,point):
 oid=int(g.orbit[point]);key=(tuple(g.safe),oid)
 if key not in certcache:
  cc=s.B.jordan(g,np.array([point]),seed=29)
  assert not cc['missing'] and not cc['uncertifiable_below_8']
  cert=cc['certificates'][str(oid)];assert s.B.check_jordan(g,cert['word'],oid,cert['prime'])['verified'];certcache[key]=cert['prime']
 return certcache[key]
for y,hs in pairs.items():
 assert labels(im(R),y)[d]==0
 for h in hs:
  assert h in C and h not in E
  safe=np.flatnonzero((ql[C!=h]==0).all(axis=0)&(dl==0).all(axis=0));key=tuple(safe)
  if key not in cache:cache[key]=group(safe)
  G1=cache[key];o1,z1=G1.orbits();choices=E[(o1[E]==o1[y])&(o2[E]==o2[h])]
  if not len(choices):print('TWO_CAP_NO_RESOURCE',y,h,flush=True);continue
  a=int(choices[0]);p1,p2=certify(G1,y),certify(G2,h)
  st=s.sim.State(ctx,menu=M);khy=kp.compose(int(reg.transport[y]),int(kp.inv[int(reg.transport[h])]))
  st.pose_id[h]=st._intern_matrix(to_tuple(matmul(R,kp.matrix(khy))));st._compact()
  for c,i in [(d,12),(0,12)]:
   out=st.apply(s.sim.Twist(ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}),journal=False);assert out.applied and not out.uncertain
  assert np.array_equal(s.B.hole_set(st),np.sort(np.concatenate((C[C!=y],E))))
  assert len(s.B.off_pieces(st))==6193 and st.lattice_flags()[h]
  row=[y,h,a,int(z1[o1[y]]),int(z2[o2[h]]),p1,p2];rows.append(row);print('TWO_CAP_PASS',row,flush=True)
print('TWO_CAP_SUMMARY','pairs',len(rows),'middle_geometries',len(cache),'global_certificates',len(certcache),'CPU',round(time.process_time(),1),flush=True)
