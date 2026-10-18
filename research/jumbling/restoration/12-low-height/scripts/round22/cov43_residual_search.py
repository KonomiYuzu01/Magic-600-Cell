# Theory round 22, coverage round 6 (COV-43): the adjudicating experiment for the singleton residual. It runs COV-36's
# computation (the code of cov36_hole_orbits.py) and then, for each of the 118 cases (a class y and a representative
# hole h), a fresh worker process that builds one valid table whose only off piece is y at R = element(I_a, 0, 45),
# with hole h, and samples words of at most 48 letters, 80% of them at grips local to y and h, restarting when more
# than 12,000 pieces are off; at a lattice endpoint it replays the word exactly, records the largest height on the
# way, checks that the inverse word returns the start digest and prints FOUND. Each worker has a 120-second CPU cutoff
# (CUT_SHORT); two run at a time. ALL_PASS, a FOUND for every case, would settle SingletonResidual (COV-06 carries a
# certificate to every labelled table of its case); a FOUND settles its case; CUT_SHORT, NO_HIT or MISSING decides
# nothing.
# The senior reviewer's experiment design, kept as written apart from this header: COV-36's command with the
# experiment's lines appended, as the design says. The integrator ran it once and kept the output in
# runs/round22/cov43_residual_search.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov43_residual_search.py
import runpy,contextlib,io,time,signal,json
signal.signal(signal.SIGXCPU,lambda *a: (print('CUT_SHORT_CPU',flush=True),exit(124)))
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):
 n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov30_validity.py')
cl=n['cl'];np=n['np'];reg=n['reg'];kp=n['kp'];s=n['s'];M=n['M'];R=n['R'];home=cl['home'];ri=cl['ri']
print('VALIDITY',len(n['valid']),flush=True)
rows=[];total=0;reduced=0
for y in n['valid']:
 t=int(reg.orbit_of[y]);ts=np.flatnonzero(np.asarray(reg.orbit_of)==t)
 if t not in cl['cache']:
  es=[]
  for c in range(600):
   xs=ts[home[ts,c]];src=np.searchsorted(ts,xs);ed=[]
   for k in kp.gen_idx[2*c:2*c+2]:ed.append((src,np.searchsorted(ts,reg.piece_of[t,kp.compose_vec(np.full(len(xs),k),reg.transport[xs])])))
   es.append(ed)
  cl['cache'][t]=es
 sides=cl['labels'](ri,y);lab=np.arange(len(ts))
 while True:
  old=lab.copy()
  for c in np.flatnonzero(sides==0):
   for src,dst in cl['cache'][t][c]:
    v=np.minimum(lab[src],lab[dst]);np.minimum.at(lab,src,v);np.minimum.at(lab,dst,v)
  while not np.array_equal(lab,lab[lab]):lab=lab[lab]
  if np.array_equal(old,lab):break
 reps,sizes=np.unique(lab,return_counts=True)
 cv=cl['cent'][y];x=np.array(cv[1:5]);z=np.array(cv[5:]);arr=np.column_stack((cv[0]*cl['den'],cl['a']@x+5*(cl['b']@z),cl['b']@x+cl['a']@z));arr//=np.gcd.reduce(arr,axis=1)[:,None]
 stab=[k for k in np.flatnonzero((arr==np.array(cv)).all(axis=1)) if cl['vk'](cl['transport_rows'](cl['ki'][k],cl['V'][y]))==cl['vk'](cl['V'][y])]
 parent={int(r):int(r) for r in reps}
 def find(a):
  while parent[a]!=a:a=parent[a]
  return a
 for k in stab:
  dst=reg.piece_of[t,kp.compose_vec(np.full(len(reps),k),reg.transport[ts[reps]])]
  for r,q in zip(reps,lab[np.searchsorted(ts,dst)]):parent[find(int(r))]=find(int(q))
 ns=len({find(int(r)) for r in reps});total+=len(reps);reduced+=ns
 row=[y,t,len(ts),len(reps),ns,len(stab),[[int(ts[r]),int(sz)] for r,sz in zip(reps,sizes)]];rows.append(row)
 print('HOLE_ORBITS',json.dumps(row),flush=True)
print('TOTAL',sum(r[2] for r in rows),total,reduced,'CPU',round(time.process_time(),2),flush=True)
import os,sys,subprocess
from concurrent.futures import ThreadPoolExecutor
cases=[(row[0],pair[0]) for row in rows for pair in row[6]]
assert len(cases)==118 and len(set(cases))==118
worker=r'''
import sys,json,random,resource,signal
from pathlib import Path
y,h=map(int,sys.argv[1:3]);trial=0
resource.setrlimit(resource.RLIMIT_CPU,(120,125))
def stop(*args):
 print('CUT_SHORT',json.dumps({'y':y,'h':h,'trials':trial}),flush=True)
 raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np,split_pairs as SP
from exact import transpose
from sim.kplus import to_tuple
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
R=s.restore.element(M,0,45);root=s.sim.State(ctx,menu=M)
root.pose_id[y]=root._intern_matrix(R)
if h!=y:
 k=kp.compose(int(reg.transport[y]),int(kp.inv[int(reg.transport[h])]))
 root.pose_id[h]=root._intern_k(k)
root._compact()
assert s.B.off_pieces(root).tolist()==[y] and s.B.hole_set(root).tolist()==[h]
root_digest=root.digest();home=s.CL.data.sig_matrix(np.array([y,h]))
local=sorted(set(np.flatnonzero(home.any(axis=0)).tolist())|{c for c in range(600) if SP.side(R,y,c)=='in'})
E=[s.restore.element(M,0,i) for i in range(60)];inv=[E.index(to_tuple(transpose(g))) for g in E]
rng=random.Random(220600+177120*y+h)
def clone(a):
 z=s.sim.State(ctx,menu=M);z.pose_id=a.pose_id.copy();z._poses=list(a._poses);z._lookup=dict(a._lookup);return z
def apply(z,c,i):
 return z.apply(s.sim.Twist(ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}),journal=False)
for trial in range(1000000):
 st=clone(root);word=[]
 for depth in range(48):
  for attempt in range(80):
   c=rng.choice(local) if rng.random()<0.8 else rng.randrange(600)
   if word and c==word[-1][0]:continue
   i=rng.randrange(1,60);out=apply(st,c,i)
   if out.applied and not out.uncertain:
    word.append([c,i]);break
  else:break
  if st.is_lattice():
   check=clone(root);height=2
   for cc,ii in word:
    out=apply(check,cc,ii);assert out.applied and not out.uncertain
    height=max(height,max(s.T.dist(s.T.factors(check._pose_matrix(int(pid)))[0]) for pid in np.unique(check.pose_id)))
   assert check.is_lattice();end=check.digest()
   for cc,ii in reversed(word):
    out=apply(check,cc,inv[ii]);assert out.applied and not out.uncertain
   assert check.digest()==root_digest
   print('FOUND',json.dumps({'y':y,'h':h,'word':word,'length':len(word),'max_height':height,'start':root_digest,'end':end,'inverse_verified':True}),flush=True)
   raise SystemExit(0)
  if st.off_lattice_count()>12000:break
print('NO_HIT',json.dumps({'y':y,'h':h,'trials':1000000}),flush=True)
raise SystemExit(2)
'''
env=os.environ.copy()
for key in ('OPENBLAS_NUM_THREADS','OMP_NUM_THREADS','MKL_NUM_THREADS'):env[key]='1'
def job(case):
 p=subprocess.run([sys.executable,'-I','-B','-u','-c',worker,*map(str,case)],env=env,text=True,capture_output=True)
 return case,p.returncode,p.stdout,p.stderr
hits=set()
with ThreadPoolExecutor(max_workers=2) as pool:
 for case,code,out,err in pool.map(job,cases):
  print('CASE',case,'EXIT',code,flush=True)
  print(out,end='',flush=True)
  if err:print(err,end='',flush=True)
  for line in out.splitlines():
   if code==0 and line.startswith('FOUND '):
    result=json.loads(line[6:])
    assert (result['y'],result['h'])==case and result['inverse_verified']
    hits.add(case)
print('ALL_PASS' if hits==set(cases) else 'MISSING',sorted(set(cases)-hits),flush=True)
