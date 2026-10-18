# Theory round 22, coverage round 7 (COV-49): no replacement of at most two letters for the two different-cap
# excursions of COV-41. Through cov41_excursions.py it returns to COV-17's singleton A and, for each of the words
# [(14, 12), (35, 13)] and [(39, 12), (29, 13)], applies it and lists the distinct left multipliers of the endpoint
# with their piece counts and the number of menu letters (over all 600 caps) that have each one; it then replays every
# ordered pair of letters whose multipliers E, F give {E, F, FE} equal to the endpoint's nonidentity multipliers and
# keeps the pairs that reach the same labelled endpoint. WORD gives EFFECTS (K⁺ index, or −1 outside K⁺; pieces;
# letters), CANDIDATES (pairs replayed), EXACT_ENDPOINT_WORDS (with the largest height after each letter) and END, the
# endpoint digest. It combines the two audits that the reviewer ran separately.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov49_two_letter_audit.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov49_two_letter_audit.py
import runpy,contextlib,io,time
with contextlib.redirect_stdout(io.StringIO()),contextlib.redirect_stderr(io.StringIO()):
 n=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov41_excursions.py')
s,M,st,np=n['s'],n['M'],n['st'],n['np'];ctx=s.CL.ctx;kp=ctx.kplus
from exact import matmul,transpose
from sim.kplus import to_tuple
for _ in range(3):st.undo()
A=st.digest();start=st.pose_id.copy();m0=[st._pose_matrix(i) for i in range(len(st._poses))]
for w in [[(14,12),(35,13)],[(39,12),(29,13)]]:
 for c,i in w:n['apply'](c,i)
 effects={}
 for a,b in set(zip(start.tolist(),st.pose_id.tolist())):
  E=to_tuple(matmul(st._pose_matrix(b),transpose(m0[a])))
  effects[E]=effects.get(E,0)+int(((start==a)&(st.pose_id==b)).sum())
 end=st.digest()
 for _ in w:st.undo()
 nonid=[x for x in effects if x!=kp.matrix(0)];letters={x:[] for x in nonid}
 for c in range(600):
  for i in range(1,60):
   E=s.restore.element(M,c,i)
   if E in letters:letters[E].append((c,i))
 products=0;endpoints=[]
 for e in nonid:
  for f in nonid:
   if e==f or {e,f,to_tuple(matmul(f,e))}!=set(nonid):continue
   for c,i in letters[e]:
    for d,j in letters[f]:
     products+=1;height=[];applied=0
     for cc,ii in [(c,i),(d,j)]:
      o=st.apply(s.sim.Twist(ctx,cc,s.restore.element(M,cc,ii),'menu',{'menu':M.name}))
      assert not o.uncertain
      if not o.applied:break
      applied+=1
      height.append(max(s.T.dist(s.T.factors(st._pose_matrix(int(pid)))[0]) for pid in np.unique(st.pose_id)))
     if applied==2 and st.digest()==end:endpoints.append(([(c,i),(d,j)],height))
     for _ in range(applied):st.undo()
     assert st.digest()==A
 print('WORD',w,'EFFECTS',sorted((kp.index_of_matrix(e),effects[e],len(letters.get(e,[]))) for e in effects),'CANDIDATES',products,'EXACT_ENDPOINT_WORDS',endpoints,'END',end,flush=True)
