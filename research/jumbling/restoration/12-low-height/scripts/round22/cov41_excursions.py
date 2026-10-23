# Theory round 22, coverage round 6 (COV-41): two-letter excursions above height 2 at COV-17's singleton. It replays
# COV-17's 24-letter word (through cov17_single_off.py) to the state A whose only off piece is 58388 and lists the
# caps of other classes that contain that piece (FOREIGN). At each such cap c it applies the item 12, checks that the
# piece reaches height 4, and lists, for every cap d of c's class that contains the piece, the items j for which
# F_d(j)·E_c(12) lies in K⁺ (RETURNERS); at a cap d ≠ c with such items it applies the first one, checks that every
# pose is back at height at most 2 and prints the endpoint (TWO_CAP_WITNESS). It ends with a three-letter excursion at
# cap 8 that returns exactly to A (K3_SAME_CAP, with the largest height after each letter).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov41_excursions.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov41_excursions.py
import runpy,contextlib,io,time
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov17_single_off.py')
s,M,st,W,np=[n[k] for k in ['s','M','st','W','np']]
from exact import matmul,transpose
from sim.kplus import to_tuple
members,cls=s.CL.classes()
def apply(c,i):
 o=st.apply(s.sim.Twist(s.CL.ctx,int(c),s.restore.element(M,int(c),int(i)),'menu',{'menu':M.name}));assert o.applied and not o.uncertain
for c,i in W:apply(c,i)
A=st.digest();p=58388;pose=st.pose(p);foreign=[]
for c in range(600):
 cl=st.classify(c,need_inside=True,stop_at_straddle=True)
 if cl.straddle is None and p in cl.inside and cls[c]!=0:foreign.append(c)
print('FOREIGN',[[c,int(cls[c])] for c in foreign],flush=True)
for c in foreign:
 E=s.restore.element(M,c,12);apply(c,12);hp=s.T.dist(s.T.factors(st.pose(p))[0]);assert hp==4
 rows=[]
 for d in members[cls[c]]:
  cl=st.classify(int(d),need_inside=True,stop_at_straddle=True);assert not cl.uncertain
  if cl.straddle is not None or p not in cl.inside:continue
  items=[]
  for j in range(12,60):
   F=s.restore.element(M,int(d),j);K=to_tuple(matmul(F,E));k=s.CL.ctx.kplus.index_of_matrix(K)
   if k>=0:items.append([j,k])
  rows.append([int(d),items])
  if d!=c and items:
   j,k=items[0];before=[st._pose_matrix(int(pid)) for pid in np.unique(st.pose_id)]
   apply(d,j);heights={int(s.T.dist(s.T.factors(st._pose_matrix(int(pid)))[0])) for pid in np.unique(st.pose_id)};assert max(heights)<=2
   print('TWO_CAP_WITNESS',[c,12,int(d),j],st.digest(),'off',st.off_lattice_count(),'heights',sorted(heights),'netK',k,flush=True)
   st.undo()
 print('RETURNERS',c,rows,flush=True);st.undo();assert st.digest()==A
c=8;P=to_tuple(matmul(s.restore.element(M,c,1),s.restore.element(M,c,12)));j=next(j for j in range(60) if s.restore.element(M,c,j)==to_tuple(transpose(P)))
h=[]
for cc,ii in [(c,12),(c,1),(c,j)]:apply(cc,ii);h.append(max(s.T.dist(s.T.factors(st._pose_matrix(int(pid)))[0]) for pid in np.unique(st.pose_id)))
assert st.digest()==A
print('K3_SAME_CAP',[[c,12],[c,1],[c,j]],'max_heights',h,'exact_identity',True,'CPU',round(time.process_time(),1),flush=True)
