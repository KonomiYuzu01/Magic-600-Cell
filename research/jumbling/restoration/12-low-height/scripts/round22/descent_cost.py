# Theory round 22, the integrator's check after COV-51 started: the cost of the existing descent at a residual table.
# For one case (y, h) of COV-36 it builds COV-43's table (the only off piece y at R = element(I_a, 0, 45), hole h) and
# runs the existing descent of research/jumbling/theory/restore.py once (Restorer.run with at most 48 letters, as
# trial 0 of COV-51 does), under a CPU cutoff given on the command line in place of COV-51's 120 seconds. SETUP gives
# the CPU time of the setup; N0 the misalignment N of the table and the CPU time it took; STEP one line per descent
# step (its rule, its letters, N before it, the off count after it and the CPU time since the descent began); DESCENT
# the outcome ('lattice', 'stuck' or 'step-limit'), the steps, the letters, whether the table is a lattice table, the
# off count and the CPU time; CUT_SHORT that the cutoff was reached first. The integrator ran it at eight cases and
# kept the output in runs/round22/descent_cost.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/descent_cost.py <y> <h> <CPU seconds>
import sys,json,time,resource
from pathlib import Path
y,h,cap=map(int,sys.argv[1:4])
resource.setrlimit(resource.RLIMIT_CPU,(cap,cap+5))
import signal
def stop(*a):
 print('CUT_SHORT',json.dumps({'y':y,'h':h,'cpu':round(time.process_time(),1)}),flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
R=s.restore.element(M,0,45);root=s.sim.State(ctx,menu=M)
root.pose_id[y]=root._intern_matrix(R)
if h!=y:
 k=kp.compose(int(reg.transport[y]),int(kp.inv[int(reg.transport[h])]))
 root.pose_id[h]=root._intern_k(k)
root._compact()
assert s.B.off_pieces(root).tolist()==[y] and s.B.hole_set(root).tolist()==[h]
print('SETUP',json.dumps({'y':y,'h':h,'cpu':round(time.process_time(),1)}),flush=True)
t0=time.process_time()
r=s.restore.Restorer(root,M)
N0=r.misalignment()
print('N0',json.dumps({'N':int(N0),'cpu':round(time.process_time()-t0,1)}),flush=True)
def log(step):
 print('STEP',json.dumps({'rule':step['rule'],'twists':step['twists'],'N_before':int(step['N_before']),'off':int(root.off_lattice_count()),'cpu':round(time.process_time()-t0,1)}),flush=True)
out,steps=r.run(max_steps=48,log=log)
print('DESCENT',json.dumps({'y':y,'h':h,'outcome':out,'steps':len(steps),'letters':len(r.moves),'lattice':bool(root.is_lattice()),'off':int(root.off_lattice_count()),'cpu':round(time.process_time()-t0,1)}),flush=True)
