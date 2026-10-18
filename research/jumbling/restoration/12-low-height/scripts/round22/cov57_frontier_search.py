# Theory round 22, coverage round 8 (COV-57): a bounded exact enumeration of one three-jumble family at two residual
# cases, without the descent. For (y, h) = (12, 12) and (2149, 41) it builds COV-43's table through the head of the
# worker of cov43_residual_search.py, with the CPU cutoff raised to 900 seconds, and tries every word (0, 12)·A·E·B·F:
# A the retained element a of grip 2 (a = 0 to 11, one job each, 0 the identity), E every jumble letter at a cap with
# no straddler, B every retained element of grip 32 (for y = 12) or 23 (for y = 2149), and F a final jumble letter at
# a cap whose inside pieces are exactly the off pieces, that contains every hole and that lands every off pose. A FOUND
# word is replayed exactly from the table, with its largest left height, a lattice endpoint and the inverse word back to
# the start digest. 24 jobs, two at a time. A FOUND settles its case; EXHAUSTED_FAMILY from every job excludes only this
# family; CUT_SHORT or an error decides nothing.
# The senior reviewer's experiment design, kept as written apart from this header. The integrator ran it once and
# kept the output in runs/round22/cov57_frontier_search.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov57_frontier_search.py
import ast,os,sys,subprocess
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/cov43_residual_search.py')
doc=ast.parse(p.read_text())
a=next(n for n in doc.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='worker' for t in n.targets))
head=ast.literal_eval(a.value).split('for trial in range(1000000):')[0].replace('(120,125)','(900,905)')
body=r'''
from exact import matmul
a=int(sys.argv[3]);ca=2;cb=32 if y==12 else 23
assert np.all(np.diff(ctx.data.cap_offsets)==3097)
base=clone(root);prefix=[[0,12]]+([[ca,a]] if a else [])
for c,i in prefix:
 out=apply(base,c,i)
 assert not out.uncertain
 if not out.applied:
  print('EXHAUSTED_FAMILY',y,h,a,'inadmissible prefix',flush=True);raise SystemExit(0)
for c in range(600):
 cl=base.classify(c,need_inside=True,stop_at_straddle=True)
 assert not cl.uncertain
 if cl.straddle is not None:continue
 for i in range(12,60):
  trial+=1;mid=clone(base);out=apply(mid,c,i)
  assert out.applied and not out.uncertain
  if mid.off_lattice_count()!=3097:continue
  for b in range(12):
   z=clone(mid)
   if b:
    out=apply(z,cb,b);assert not out.uncertain
    if not out.applied:continue
   holes=s.B.hole_set(z)
   caps=np.flatnonzero(np.unpackbits(np.bitwise_and.reduce(ctx.data.sigbits[holes],axis=0))[:600])
   off=s.B.off_pieces(z);pids=np.unique(z.pose_id[off])
   for d in caps.tolist():
    cl=z.classify(d,need_inside=True,stop_at_straddle=True);assert not cl.uncertain
    if cl.straddle is not None or not np.array_equal(cl.inside,off):continue
    for j in range(12,60):
     F=s.restore.element(M,d,j)
     if any(kp.index_of_matrix(to_tuple(matmul(F,z._pose_matrix(int(pid)))))<0 for pid in pids):continue
     word=prefix+[[c,i]]+([[cb,b]] if b else [])+[[d,j]]
     check=clone(root);height=2
     for cc,ii in word:
      out=apply(check,cc,ii);assert out.applied and not out.uncertain
      height=max(height,max(s.T.dist(s.T.factors(check._pose_matrix(int(pid)))[0]) for pid in np.unique(check.pose_id)))
     assert check.is_lattice();end=check.digest()
     for cc,ii in reversed(word):
      out=apply(check,cc,inv[ii]);assert out.applied and not out.uncertain
     assert check.digest()==root_digest
     print('FOUND',json.dumps({'y':y,'h':h,'a':a,'word':word,'length':len(word),'max_height':height,'start':root_digest,'end':end,'inverse_verified':True}),flush=True)
     raise SystemExit(0)
print('EXHAUSTED_FAMILY',y,h,a,'second_letters',trial,flush=True)
'''
compile(head+body,'<frontier-worker>','exec')
jobs=[(y,h,a) for y,h in [(12,12),(2149,41)] for a in range(12)]
def job(args):
 p=subprocess.run([sys.executable,'-I','-B','-u','-c',head+body,*map(str,args)],capture_output=True,text=True,env=os.environ.copy())
 return args,p.returncode,p.stdout,p.stderr
with ThreadPoolExecutor(max_workers=2) as pool:
 for args,code,out,err in pool.map(job,jobs):
  print('JOB',args,'EXIT',code,flush=True);print(out,end='',flush=True)
  if err:print(err,end='',flush=True)
