# Theory round 22, coverage round 5 (COV-30): singleton validity of the deep and outer placement classes. For each of
# COV-23's 131 deep class representatives and the seven outer representatives with mover caps, it rebuilds the home
# region of y from its active cuts and body facets (checking its vertices against regions.vertices), rotates it by
# R = element(I_a, 0, 45) and intersects it with the lattice body by exact double description. An empty or
# lower-dimensional intersection makes every hole of y's type valid (VALID_ALL_HOLES); otherwise an exact interior point
# names an overlapped chamber of another type (INVALID_TYPE) or two overlapped chambers (INVALID_TWO). It prints the
# valid deep representatives, the invalid ones with their certificates, the outer verdicts and the number of class–hole
# pairs left (UNIFIED_VALIDITY).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov30_validity.txt.
# It writes no file. Research only, headless Linux; exact geometry in Q(√5).
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov30_validity.py
import runpy,time,contextlib,io,json
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):
 n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov23_singleton_classes.py')
cl=n
s,np,M,reg,kp=[n[k] for k in ['s','np','M','reg','kp']]
deepreps=[y for y,ys in cl['groups'] if ys and set(ys)<=cl['deep']]
outer=[1228,1229,1338,1341,1343,1345,1437]
from exact import matvec,dot,double_description,affine_rank,Q5
R=s.restore.element(M,0,45);home=s.CL.data.sig_matrix(np.arange(len(reg.orbit_of)))
sigindex={np.packbits(home[p]).tobytes():p for p in range(len(home))}
def chamber(v):
 if any((dot(nn,v)-s.CL.data.NN).sign()>=0 for nn in s.CL.data.N):return None
 signs=[(dot(nn,v)-s.CL.data.KAPPA).sign() for nn in s.CL.data.N]
 if 0 in signs:return None
 return sigindex.get(np.packbits(np.array(signs)>0).tobytes(),-1)
def centroid(vs,weights=None):
 weights=weights or [1]*len(vs);total=sum(weights)
 return tuple(sum((Q5(w)*v[j] for v,w in zip(vs,weights)),Q5())/Q5(total) for j in range(4))
results=[]
for y in deepreps+outer:
 vs=reg.vertices(y);cons=[]
 for c,nn in enumerate(s.CL.data.N):
  if any((dot(nn,v)-s.CL.data.KAPPA).is_zero() for v in vs):
   cons.append(([-q for q in nn],-s.CL.data.KAPPA) if home[y,c] else (nn,s.CL.data.KAPPA))
  if any((dot(nn,v)-s.CL.data.NN).is_zero() for v in vs):cons.append((nn,s.CL.data.NN))
 vcheck,_=double_description(cons);assert set(map(tuple,vcheck))==set(map(tuple,vs))
 rv=[matvec(R,v) for v in vs];cons=[(matvec(R,nn),b) for nn,b in cons]
 cons.extend((nn,s.CL.data.NN) for nn in s.CL.data.N if any((dot(nn,v)-s.CL.data.NN).sign()>0 for v in rv))
 try:inter,_=double_description(cons)
 except ValueError as e:
  assert str(e) in ['region is empty','region is not full-dimensional'];r=[y,'VALID_ALL_HOLES',str(e)];results.append(r);continue
 found=None
 for k in range(1,31):
  p=centroid(inter,[1+((j+1)*k%31) for j in range(len(inter))]);q=chamber(p)
  if q is not None:found=(p,q);break
 assert found and found[1]>=0
 p,q=found
 if reg.orbit_of[q]!=reg.orbit_of[y]:r=[y,'INVALID_TYPE',q,int(reg.orbit_of[q])]
 else:
  bad=[]
  for c,nn in enumerate(s.CL.data.N):
   for v in inter:
    sg=(dot(nn,v)-s.CL.data.KAPPA).sign()
    if (home[q,c] and sg<0) or (not home[q,c] and sg>0):bad.append(v)
  if not bad:r=[y,'VALID_ONLY_HOLE',q]
  else:
   q2=None
   for v in bad:
    for k in [1,3,9,99,999]:
     z=tuple((p[j]+Q5(k)*v[j])/Q5(k+1) for j in range(4));qq=chamber(z)
     if qq is not None and qq!=q:q2=qq;break
    if q2 is not None:break
   assert q2 is not None and q2>=0;r=[y,'INVALID_TWO',q,q2]
 results.append(r)
valid=[r[0] for r in results if r[1]=='VALID_ALL_HOLES']
assert not [r for r in results if r[1]=='VALID_ONLY_HOLE']
print('UNIFIED_VALIDITY',json.dumps({'valid_deep':[y for y in valid if y in deepreps],'invalid_deep':[r for r in results if r[0] in deepreps and r[1].startswith('INVALID')],'outer':[r for r in results if r[0] in outer],'remaining_pairs':sum(int((np.asarray(reg.orbit_of)==reg.orbit_of[y]).sum()) for y in valid)}),'CPU',round(time.process_time(),1),flush=True)
