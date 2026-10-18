# Theory round 22, coverage round 2 (COV-14): the whole-cap route at COV-01's isolated triangle. It rebuilds that
# state with cov01_isolated_triangle.py, lists every unstraddled cap containing piece 1474 and the menu items off K+
# that land it, applies the first such letter and reads N1 and N2 of the whole cap at the middle. Whether the state
# is reachable is not decided here.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/cov14_isolated_wholecap.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov14_isolated_wholecap.py
import runpy,json,time
from pathlib import Path
ns=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/cov01_isolated_triangle.py')
s,st,M,R,p,np=map(ns.get,('s','st','M','R','p','np'))
from exact import matmul
from sim.kplus import to_tuple
from types import SimpleNamespace
kp=s.CL.ctx.kplus;reg=s.CL.ctx.regions
rows=[];inside=[]
for c in range(600):
 cl=st.classify(c,need_inside=True,stop_at_straddle=False)
 assert not cl.uncertain
 if p not in cl.inside or cl.straddle is not None:continue
 inside.append(c)
 items=[]
 for i in range(len(M.items)):
  g=s.restore.element(M,c,i)
  if kp.index_of_matrix(g)>=0:continue
  k=kp.index_of_matrix(to_tuple(matmul(g,R)))
  if k>=0:items.append(i)
 if items:rows.append([c,items])
print('WHOLE_CAP_LANDERS',json.dumps({'unstraddled_caps_containing_piece':inside,'letters':rows}),flush=True)
c,i=rows[0][0],rows[0][1][0]
old=s.B.hole_set(st)
out=st.apply(s.sim.Twist(s.CL.ctx,c,s.restore.element(M,c,i),'menu',{'menu':M.name}),journal=False)
assert out.applied and not out.uncertain
G=s.B.Alphabet(SimpleNamespace(st=st),M);G.orbits()
T=np.flatnonzero(G.home[:,c]);co=s.B.condition(G,T)
D=s.rim.Domains(st)
print('ISOLATED_WHOLE_CAP_MIDDLE',json.dumps({'letter':[c,i],'digest':st.digest(),'off':len(s.B.off_pieces(st)),'T':len(T),'landing_chamber':int(D.cham[p]),'N1_fixed_occupied':co['fixed_occupied'],'condition':co,'holes_before_inside_T':np.intersect1d(old,T).tolist()}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
