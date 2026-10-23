# Theory round 22, coverage round 3 (COV-18 and COV-20): singleton off contents in all four cell kinds, and the
# obstruction at COV-01's state. At Y = solved·(0, 12) it checks, for one hole h of cap 0 and two outside chambers a, b
# in the same SAFE orbit per cell kind, a global Jordan certificate of that orbit (so the three-cycle (h a b) is in the
# SAFE group and solved·J·V·J⁻¹ has one off piece in that kind of cell); then that chamber 2149, where J·R sends
# chamber 1474, is a hole in a singleton SAFE orbit (COV-20), and the inventory of type-18 holes of cap 0.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov18_singleton_kinds.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov18_singleton_kinds.py
import sys,json,time,resource,signal,hashlib
from pathlib import Path
from types import SimpleNamespace
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(1700,1720))
def stop(*a):print('CUT_SHORT_CPU_1700_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matvec,matmul
from sim.kplus import to_tuple
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions;st=s.sim.State(ctx,menu=M)
J=s.restore.element(M,0,12);Ji=s.restore.element(M,0,45)
o=st.apply(s.sim.Twist(ctx,0,J,'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
G=s.B.Alphabet(SimpleNamespace(st=st),M);orb,sizes=G.orbits();hc=np.bincount(orb[G.holes],minlength=len(sizes));P=s.CL.classes()[0][0]
perm={c:s.CL.pole(matvec(Ji,s.CL.N[c])) for c in P};keys=set(map(bytes,np.packbits(G.home[:,P],axis=1)))
def cell(h):return sorted(perm[c] for c in P if G.home[h,c])
def home(C):return np.packbits(np.isin(P,C)).tobytes() in keys
rows=[(1,2335,809,813),(2,25,561,571),(3,75,607,623),(4,116,644,909)]
cert=s.B.jordan(G,[h for _,h,_,_ in rows],seed=17);assert not cert['missing'] and not cert['uncertifiable_below_8']
for kind,h,a,b in rows:
 o=int(orb[h]);assert h in G.holes and a not in G.holes and b not in G.holes and len({h,a,b})==3
 assert orb[a]==o==orb[b] and len(cell(h))==kind
 v=cert['certificates'][str(o)];assert s.B.check_jordan(G,v['word'],o,v['prime'])['verified']
 print('SINGLETON_KIND',json.dumps({'kind':kind,'hole':h,'outside':[a,b],'orbit':o,'size':int(sizes[o]),'holes':int(hc[o]),'type':int(G.type[h]),'cell':cell(h),'home_cell':home(cell(h)),'prime':v['prime'],'word_length':v['length'],'word_sha256':hashlib.sha256(json.dumps(v['word'],separators=(',',':')).encode()).hexdigest()}),flush=True)
assert s.T.dist(s.T.factors(Ji)[0])==2
k=kp.index_of_matrix(to_tuple(matmul(J,s.restore.element(M,0,25))));assert k>=0
x=int(reg.piece_of[int(reg.orbit_of[1474]),kp.compose(k,int(reg.transport[1474]))]);assert x==2149 and sizes[orb[x]]==1 and x in G.holes
print('X_TARGET_FIXED',{'chamber':x,'orbit_size':1,'hole':True},flush=True)
cnt=Counter((len(cell(h)),home(cell(h)),bool(sizes[orb[h]]>hc[orb[h]])) for h in G.holes.tolist() if G.type[h]==18)
print('TYPE18_SINGLE_J',json.dumps([{'kind':k,'home_cell':homecell,'mixed_orbit':mixed,'count':v} for (k,homecell,mixed),v in sorted(cnt.items())]),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
