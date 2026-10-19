# Theory round 22, line (d), round 4 (R22D-31 to R22D-36; R22D-38 rests on the R22D-35 part): height 6 at two census
# states and at one test state. R22D-31: the state before step 74 of the depth 1000 record of seed 7103 (48ef2f68...),
# its two height-6 units, caps 328 and 241, and the prefixes of the step's word (328, 1)(241, 47)(328, 58) and of its
# middle letter alone. R22D-32: the eleven height-6 units of the 3000-twist stop of seed 7302 (015d839e...), their
# containing caps and the straddlers of each. R22D-33: the 24 single letters that lower a height-6 unit there, with
# their changes of N and end counts. R22D-34: the word (2, 1)(64, 15)(2, 1). R22D-35: after each of the 24 letters,
# the caps of the new branch that hold either unit whole, all blocked. R22D-36: a test state of 26 twists from the
# solved state with one height-6 piece, and the protected step (12, 45).
# The senior reviewer's helper and six computations, kept as written apart from this header, a heading line before
# each part, and the snapshot loader, now imported from r22d31_snapshots.py; the integrator re-ran them and got the
# output in runs/round22/r22d31_height6.txt. About 3 CPU minutes.
# Build the snapshots first (about 30 CPU minutes): python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>
# It writes no file; theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d31_height6.py <dir>
import sys,json
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0]=["research/jumbling/restoration/12-low-height/scripts","research/jumbling/theory","research/jumbling","research/jumbling/fixtures","research/jumbling/restoration/12-low-height/scripts/round22"]
import eb6 as E
import seams
from r22d31_snapshots import load
from sim.kernel import Evaluator,pullback_form
from exact import matmul
from sim.kplus import to_tuple
C=E.sim.get_context(); M=E.R.menu("I_a")
def read(name): return load(Path(sys.argv[1])/name,E)[0]
def H(s):
 return np.array([0 if p.kidx>=0 else E.T.dist(E.T.factors(s._pose_matrix(i))[0]) for i,p in enumerate(s._poses)])[s.pose_id]
def play(s,w):
 for c,j in w:
  r=s.apply(E.sim.Twist(C,c,E.R.element(M,c,j),"menu",{"menu":M.name}),journal=False)
  assert r.applied and not r.uncertain
def clone(s):
 t=E.sim.State(C,filtered=False,menu=M); t.pose_id=s.pose_id.copy(); t._poses=list(s._poses); t._lookup=dict(s._lookup); return t
def units(s):
 ps=np.flatnonzero(H(s)==6); ins=np.zeros((len(ps),600),bool)
 for pid in np.unique(s.pose_id[ps]).tolist():
  rows=np.flatnonzero(s.pose_id[ps]==pid)
  for c in range(600):
   ev=Evaluator(pullback_form(s._pose_matrix(pid),C.data.N[c]))
   for r in rows: ins[r,c]=ev.classify(C.regions.vform(int(ps[r])))[0]=="in"
 out={}
 for r,p in enumerate(ps.tolist()):
  out.setdefault((int(s.pose_id[p]),tuple(np.flatnonzero(ins[r]).tolist())),[]).append(p)
 return sorted([(qs,cs) for (pid,cs),qs in out.items()],key=lambda u:u[0][0])
def N(s): return seams.SeamRestorer(s,M).misalignment()
def report(s,w):
 h0=H(s); sid=E.stable_pose(s); print("START",s.digest(),dict(Counter(h0.tolist())),N(s))
 for a in w:
  play(s,[a]); print("PREFIX",a,s.digest(),dict(Counter(H(s).tolist())),N(s))
 h=H(s); same=E.stable_pose(s)==sid
 print("TRANSITIONS",sorted(Counter(zip(h0.tolist(),h.tolist())).items()))
 print("COLLATERAL",int(((h0==0)&(h>0)).sum()),"OTHER_OFF_CHANGED",int(((h0>0)&(h0!=6)&(h>0)&~same).sum()))
print("== R22D-31",flush=True)
s=read("s7103-before74.pkl")
print("UNITS",[(ps[0],len(ps),cs) for ps,cs in units(s)])
for c in (328,241):
 cl=s.classify(c,need_inside=True,stop_at_straddle=False)
 print("CAP",c,cl.straddle is None,dict(Counter(H(s)[cl.inside].tolist())))
report(s,[(328,1),(241,47),(328,58)])
report(read("s7103-before74.pkl"),[(241,47)])
print("== R22D-32",flush=True)
s=read("s7302-stop.pkl"); h=H(s); us=units(s)
for ps,cs in us:
 print("UNIT",ps[0],len(ps),E.T.pose_tree(s.pose(ps[0])),cs)
for c in sorted({c for ps,cs in us for c in cs}):
 cl=s.classify(c,need_inside=True,stop_at_straddle=False); assert not cl.uncertain
 ss=E.straddler_set(s,c)
 print("CAP",c,"inside",dict(Counter(h[cl.inside].tolist())),"straddlers",len(ss),dict(Counter(h[ss].tolist())),"first",None if not len(ss) else int(ss[0]))
 if c==274: print("ALL_STRADDLERS_274",ss.tolist())
print("== R22D-33",flush=True)
s=read("s7302-stop.pkl"); r=seams.SeamRestorer(s,M)
for c in (64,70):
 q=s.pose(152672)
 js=[j for j in range(1,60) if E.T.dist(E.T.factors(to_tuple(matmul(E.R.element(M,c,j),q)))[0])==4]
 now,after=r.scores(c)
 print("LOWERS",c,js,"N_DELTAS",[after[j]-now for j in js])
 for j in js:
  t=clone(s); play(t,[(c,j)])
  print(c,j,t.off_lattice_count(),dict(Counter(H(t).tolist())))
report(clone(s),[(70,24)])
print("== R22D-34",flush=True)
report(read("s7302-stop.pkl"),[(2,1),(64,15),(2,1)])
print("== R22D-35",flush=True)
s=read("s7302-stop.pkl")
targets=[ps for ps,cs in units(s) if ps[0] in (152672,152673)]
gb=[E.T.branch(tuple(n)) for n in C.data.N]
items={64:[15,18,22,33,52,53,54,55,56,57,58,59],70:[12,16,20,24,25,26,27,28,29,30,31,32]}
count=0; good=0
for c,js in items.items():
 for j in js:
  y=clone(s); play(y,[(c,j)]); q=E.T.pose_tree(y.pose(152672))[2]; rows=[]
  for ps in targets:
   holds=[]
   for d in range(600):
    if gb[d]!=q: continue
    ev=Evaluator(pullback_form(y.pose(ps[0]),C.data.N[d]))
    if not all(ev.classify(C.regions.vform(p))[0]=="in" for p in ps): continue
    cl=y.classify(d,need_inside=False,stop_at_straddle=True); assert not cl.uncertain
    count+=1; good+=int(cl.straddle is None)
    holds.append((d,None if cl.straddle is None else cl.straddle["piece"]))
   rows.append((ps[0],holds))
  print("LOWERED",c,j,q,rows)
print("UNIT_CAP_CASES",count,"CLEAN",good)
print("== R22D-36",flush=True)
P=[(0,12),(74,8),(188,4),(74,4),(188,8),(188,4),(188,4),(74,8),(188,8),(74,4),(188,8),(178,1),(188,4),(74,8),(188,4),(74,4),(188,8),(188,8),(188,4),(74,8),(188,8),(74,4),(178,1),(0,45),(14,12),(12,12)]
s=E.sim.State(C,filtered=False,menu=M); play(s,P)
h0=H(s); sid=E.stable_pose(s); assert np.flatnonzero(h0==6).tolist()==[58388]
print("START",s.digest(),s.off_lattice_count(),dict(Counter(h0.tolist())),N(s))
play(s,[(12,45)]); h=H(s); same=E.stable_pose(s)==sid
other=h0>0; other[58388]=False
assert h[58388]==4
assert not ((h0==0)&(h>0)).any()
assert not (other&~same&(h>0)).any()
print("END",s.digest(),s.off_lattice_count(),dict(Counter(h.tolist())),N(s))
print("OTHER_EXACT",int((other&same).sum()),"OTHER_LANDED",int((other&(h==0)).sum()))
