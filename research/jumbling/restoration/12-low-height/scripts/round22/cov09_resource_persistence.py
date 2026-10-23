# Theory round 22, coverage round 2 (COV-09): a strict' predecessor that keeps the old middle SAFE alphabet but
# spends T1' hole resources. From the replay state of 96 (run 2's table after b19), the preparation
# X = (178,7)(170,1)(75,1) of the b06 branch at 71 is checked to be SAFE there and to keep every off pose; the
# whole-cap middle of W = (91,14) is read (N1, N2); then X^-1 followed by the stored b15 program is applied as one
# program (strict', 96 to 71, ending at the recorded 71 state), and the whole-cap middle is read again.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov09_resource_persistence.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov09_resource_persistence.py
import sys,json,resource,signal,time
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(1100,1120))
def stop(*a):print('CUT_SHORT_CPU_1100_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
S=Path('research/jumbling/restoration/12-low-height');E=S/'runs/round22/line-a/replay2';sys.path.insert(0,str((S/'scripts').resolve()))
import screen as s,slp_apply as a,numpy as np
st,M,_=s.load('table',str(E/'run2/z-05-b19.json.gz'))
X=json.loads((E/'b06-71/prep2/x-word.json').read_text())['word']
G=s.B.Alphabet(SimpleNamespace(st=st),M);G.numbers(X)
fp=s.B.off_fingerprint(st);a.apply_segment(st,M,{'word':X});assert s.B.off_fingerprint(st)==fp
prepared=st.digest()
def clone(st):
 out=s.sim.State(s.CL.ctx,menu=M);out.pose_id=st.pose_id.copy();out._poses=list(st._poses);out._lookup=dict(st._lookup);return out
def middle(st,tag):
 y=clone(st);a.apply_segment(y,M,{'word':[[91,14]]});G=s.B.Alphabet(SimpleNamespace(st=y),M);G.orbits();T=np.flatnonzero(G.home[:,91]);co=s.B.condition(G,T)
 print('MIDDLE',json.dumps({'tag':tag,'source':st.digest(),'digest':y.digest(),'off':len(G.holes),'holes_in_cap_before':int(np.isin(s.B.hole_set(st),T).sum()),'N1':co['fixed_vacant'],'N2':co['necessary'],'fixed_occupied':co['fixed_occupied'],'deficits':co['necessary_deficits']}),flush=True)
 return G,co
G0,c0=middle(st,'before_predecessor')
P=json.loads((E/'run2/09-b15-program.json').read_text());U=json.loads((S/'runs/round22/line-a/units/eb5-k40-k00-b15.json').read_text())
doc={'segments':[{'word':a.inverse_word(M,X)}]+P['segments']}
summary=a.run_program(st,M,doc,U)
assert summary['strict_prime'] and st.digest()=='291ae718399791f6ee7c038c03cfd080e52aaa5a01d54cd5d01a7069b8338bdd'
G1,c1=middle(st,'after_predecessor')
print('COMPARISON',json.dumps({'start':prepared,'end':st.digest(),'old_alphabet_subset_new':set(G0.index)<=set(G1.index),'same_orbit_partition':bool(np.array_equal(G0.orbit,G1.orbit)),'N1_N2_before':[c0['fixed_vacant'],c0['necessary']],'N1_N2_after':[c1['fixed_vacant'],c1['necessary']]}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
