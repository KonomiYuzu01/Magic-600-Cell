# Theory round 22, coverage shard (COV-01): an isolated triangle. Every piece is at identity except piece 1474, at
# pose element(I_a, 0, 25): one off piece of type 18 and height 2 in the class-0 triangle cell {0,2,23}, one hole.
# The script checks the cell, non-overlap (body facet 3 separates the rotated region from the lattice interiors) and,
# for every grip whose retained letters move the landed chamber 1474, that one of the type-24 chambers g0(404),
# g0(762) blocks it (an R22C2-01 intrinsic Q-shadow certificate). Whether this state is reachable is not decided here.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov01_isolated_triangle.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov01_isolated_triangle.py
import sys,json,time,resource,signal
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(1100,1120))
def stop(*a):print('CUT_SHORT_CPU_1100_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,split_pairs as SP,numpy as np
from exact import transpose,matvec,dot
from sim.kplus import to_tuple
M=s.restore.menu('I_a');reg=s.CL.ctx.regions;kp=s.CL.ctx.kplus;typ=np.asarray(reg.orbit_of);home=s.CL.data.sig_matrix(np.arange(len(typ)));members,_=s.CL.classes()
p=1474;Q=[404,762];R=s.restore.element(M,0,25);g=to_tuple(transpose(R));C=[0,2,23]
st=s.sim.State(s.CL.ctx,menu=M);st.pose_id[p]=st._intern_matrix(R);st._compact()
assert s.B.off_pieces(st).tolist()==[p] and s.B.hole_set(st).tolist()==[p]
assert s.T.dist(s.T.factors(R)[0])==2
assert [int(c) for c in members[0] if SP.side(R,p,int(c))=='in']==C
assert all(SP.side(R,p,int(c))!='straddle' for c in members[0])
assert all(int(typ[q])==24 for q in Q) and int(typ[p])==18
assert all([int(c) for c in members[0] if home[q,int(c)]]==C for q in Q)
assert all((dot(s.CL.data.N[3],matvec(R,v))-s.CL.data.NN).sign()>=0 for v in reg.vertices(p))
cert=[]
for c in np.flatnonzero(home[p]).tolist():
 ks=np.array([k for i in range(len(M.items)) if (k:=kp.index_of_matrix(s.restore.element(M,c,i)))>=0])
 if np.all(reg.piece_of[typ[p],kp.compose_vec(ks,np.full(len(ks),reg.transport[p]))]==p):continue
 blockers=[[q,SP.side(g,q,c)] for q in Q if SP.side(g,q,c)!='out']
 assert blockers;cert.append([c,blockers])
assert cert
print('ISOLATED_TRIANGLE',json.dumps({'digest':st.digest(),'piece':p,'pose':[0,25],'class':0,'cell':C,'height':2,'holes':[p],'U_type':18,'Q_chambers':Q,'Q_types':[24,24],'separating_body_facet':3,'moving_cap_certificate':cert,'reachable':'not established'}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
