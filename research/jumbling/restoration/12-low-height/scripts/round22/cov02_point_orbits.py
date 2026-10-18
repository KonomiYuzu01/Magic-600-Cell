# Theory round 22, coverage shard (COV-02): the geometric off-point slots of class 0 (current cell and frame, for the
# five frames of I_a at pole 0 modulo K+) up to H = K+ meet Sym+(class 0), of order 1440, closed exactly from
# generators. Seven point orbits (outer, vertex, two edge types, triangle and tetrahedron without home chambers,
# triangle with home chambers) and 5,764 off-point slots in all. A point enumeration only, not a classification of
# contents, holes or reachable states.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov02_point_orbits.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov02_point_orbits.py
import sys,json,time,resource,signal
from pathlib import Path
from collections import defaultdict,Counter
resource.setrlimit(resource.RLIMIT_CPU,(1100,1120))
def stop(*a):print('CUT_SHORT_CPU_1100_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul,matvec,transpose
from sim.kplus import to_tuple
M=s.restore.menu('I_a');kp=s.CL.ctx.kplus;members,cls=s.CL.classes();P=0;poles=list(map(int,members[P]));N=len(s.CL.ctx.regions.orbit_of)
frames=[];items=[]
for i in range(len(M.items)):
 g=s.restore.element(M,poles[0],i)
 if not any(kp.index_of_matrix(to_tuple(matmul(transpose(r),g)))>=0 for r in frames):frames.append(g);items.append(i)
assert len(frames)==5 and kp.index_of_matrix(frames[0])>=0
home=s.CL.data.sig_matrix(np.arange(N));ch=defaultdict(list)
for x in range(N):ch[tuple(poles[j] for j in np.flatnonzero(home[x,poles]))].append(x)
points={};origins={}
for l,r in enumerate(frames):
 if l==0:continue
 perm={c:s.CL.pole(matvec(r,s.CL.N[c])) for c in poles};assert set(perm.values())==set(poles)
 for C,xs in ch.items():
  key=(tuple(sorted(perm[c] for c in C)),l);assert key not in points
  points[key]=len(xs);origins[key]=C
H=set(np.flatnonzero(cls[kp.perms[:,poles[0]]]==P).tolist());assert len(H)==1440
identity=kp.index_of_matrix(s.restore.I4);generated={identity};gens=[]
for k in sorted(H):
 if k in generated:continue
 gens.append(k);todo=list(generated)
 for a in todo:
  for g in gens:
   b=kp.compose(g,a)
   if b not in generated:generated.add(b);todo.append(b)
 if generated==H:break
assert generated==H
acts=[]
for k in gens:
 perm=kp.perms[k];labels=[]
 for r in frames:
  kr=matmul(kp.matrix(k),r)
  hits=[j for j,t in enumerate(frames) if kp.index_of_matrix(to_tuple(matmul(transpose(t),kr)))>=0]
  assert len(hits)==1;labels.append(hits[0])
 assert labels[0]==0;acts.append((perm,labels))
left=set(points);rows=[]
while left:
 q=min(left);orb={q};todo=[q];left.remove(q)
 for C,l in todo:
  for perm,labels in acts:
   y=(tuple(sorted(int(perm[c]) for c in C)),labels[l]);assert y in points and points[y]==points[q]
   if y not in orb:orb.add(y);todo.append(y);left.remove(y)
 C,l=q;rows.append({'kind':s.KIND[len(C)],'representative_cell':C,'frame_item_at_0':items[l],'label':l,'home_cell':bool(C in ch),'whole_slots':points[q],'points_per_fixed_class':len(orb),'stabilizer_order':1440//len(orb),'home_source_cell':origins[q]})
print('POINT_ORBITS',json.dumps({'fixed_class':P,'frame_items':items,'H_generators':gens,'off_points':len(points),'orbits':rows}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
