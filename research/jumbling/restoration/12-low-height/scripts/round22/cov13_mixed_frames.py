# Theory round 22, coverage round 2 (COV-13): a reachable height-2 state without a cap whose whole off content lands
# under one letter. From the solved state, menu item 12 at every class-0 grip in increasing order (120 letters);
# every off piece has left height 2; the audit classifies all 600 grips (straddled, empty, two or more off frame
# domains, one domain with or without a landing menu element).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov13_mixed_frames.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov13_mixed_frames.py
import sys,json,resource,signal,time
from pathlib import Path
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(1100,1120))
def stop(*a):print('CUT_SHORT_CPU_1100_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul
from sim.kplus import to_tuple
M=s.restore.menu('I_a');st=s.sim.State(s.CL.ctx,menu=M);members,_=s.CL.classes()
word=[[int(c),12] for c in sorted(members[0])]
for j,(c,i) in enumerate(word):
 out=st.apply(s.sim.Twist(s.CL.ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}),journal=False)
 assert out.applied and not out.uncertain
 if (j+1)%30==0:print('APPLIED',j+1,'cpu',round(time.process_time(),1),flush=True)
off=s.B.off_pieces(st);D=s.rim.Domains(st);dom=np.asarray(D.dom)
heights={int(s.T.dist(s.T.factors(st._pose_matrix(int(pid)))[0])) for pid in np.unique(st.pose_id[off])};assert heights=={2}
print('STATE',st.digest(),'off',len(off),'heights',sorted(heights),'domains',len(D.reps)-1,flush=True)
counts=Counter();land=[];pure=[]
for c in range(600):
 cl=st.classify(c,need_inside=True,stop_at_straddle=False);assert not cl.uncertain
 ins=np.asarray(cl.inside,np.int64);u=ins[dom[ins]!=0]
 if cl.straddle is not None:counts['straddled']+=1;continue
 if not len(u):counts['empty']+=1;continue
 ds=np.unique(dom[u])
 if len(ds)>1:counts['mixed']+=1;continue
 r=st._pose_matrix(int(st.pose_id[int(u[0])]))
 items=[i for i in range(len(M.items)) if s.CL.ctx.kplus.index_of_matrix(to_tuple(matmul(s.restore.element(M,c,i),r)))>=0]
 pure.append(c)
 if items:counts['landing_caps']+=1;land.append([c,len(u),items])
 else:counts['one_domain_no_menu']+=1
print('ONE_CLASS_SCAN',json.dumps({'digest':st.digest(),'off':len(off),'heights':sorted(heights),'history':word,'counts':dict(counts),'pure_caps':pure,'landing':land}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
