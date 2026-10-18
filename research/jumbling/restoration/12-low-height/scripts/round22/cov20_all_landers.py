# Coverage round 3, the integrator's check after COV-20: every one-letter SAFE sandwich L·V·L⁻¹ that could end in
# COV-01's family. The last letter L⁻¹ must create the one off piece in X's off region, so L lands X's piece and is
# one of the 36 whole-cap letters of COV-14 (L·R in K⁺). At Y' = solved·L it finds the chamber x where L·R sends
# chamber 1474, whether x is a hole, and its SAFE orbit (size, holes). A SAFE word keeps the number of occupied
# chambers in every orbit, so a wholly vacant orbit of x excludes every V; a mixed orbit gets a Jordan search.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov20_all_landers.py
import sys,json,time,resource,signal
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(3400,3420))
def stop(*a):print('CUT_SHORT_CPU_3400_SECONDS',flush=True);raise SystemExit(124)
signal.signal(signal.SIGXCPU,stop)
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
from exact import matmul
from sim.kplus import to_tuple
M=s.restore.menu('I_a');ctx=s.CL.ctx;kp=ctx.kplus;reg=ctx.regions
R=s.restore.element(M,0,25)
# COV-14's output (runs/round22/cov14_isolated_wholecap.txt, WHOLE_CAP_LANDERS): every menu item off K+ at an
# unstraddled cap containing piece 1474 at X that lands it.
LAND=[[0,[12,16,20,24,25,26,27,28,29,30,31,32]],[2,[14,21,23,43,44,45,46,47,48,49,50,51]],[23,[12,16,20,24,25,26,27,28,29,30,31,32]]]
summary=[]
for c,items in LAND:
  for i in items:
    t0=time.process_time()
    L=s.restore.element(M,c,i)
    assert kp.index_of_matrix(L)<0
    k=kp.index_of_matrix(to_tuple(matmul(L,R)));assert k>=0
    x=int(reg.piece_of[int(reg.orbit_of[1474]),kp.compose(k,int(reg.transport[1474]))])
    st=s.sim.State(ctx,menu=M)
    o=st.apply(s.sim.Twist(ctx,c,L,'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
    G=s.B.Alphabet(SimpleNamespace(st=st),M);orb,sizes=G.orbits()
    holes=np.asarray(G.holes);hs=set(holes.tolist());hc=np.bincount(orb[holes],minlength=len(sizes))
    ox=int(orb[x]);size=int(sizes[ox]);nh=int(hc[ox])
    rec={'letter':[c,i],'digest':st.digest(),'off':len(s.B.off_pieces(st)),'holes':len(hs),'safe_letters':len(G.letters),
         'orbits':len(sizes),'x':x,'x_hole':x in hs,'x_type':int(G.type[x]),'x_orbit_size':size,'x_orbit_holes':nh,
         'mixed':size>nh}
    if size>nh:
      if size>=8:
        cert=s.B.jordan(G,[x],seed=17);v=cert['certificates'].get(str(ox))
        rec['jordan']=None if v is None else {'prime':v['prime'],'length':v['length'],
            'verified':s.B.check_jordan(G,v['word'],ox,v['prime'])['verified']}
      else:
        rec['jordan']='orbit_below_8'
    rec['cpu']=round(time.process_time()-t0,1)
    summary.append(rec)
    print('SANDWICH',json.dumps(rec),flush=True)
print('ALL36',json.dumps({'letters':len(summary),'x_hole':sum(r['x_hole'] for r in summary),
      'singleton':sum(r['x_orbit_size']==1 for r in summary),'wholly_vacant':sum(not r['mixed'] for r in summary),
      'mixed':[r['letter'] for r in summary if r['mixed']]}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
