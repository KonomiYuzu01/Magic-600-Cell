# Theory round 22, coverage round 4 (COV-24), the integrator's repetition of the senior reviewer's reference check.
# With every other piece at identity, piece 2335 (type 9) at R = element(I_a, 0, 45) gives the off geometry R·λ_2335
# of the exceptional class 2335 of COV-24, and the full buffer.Alphabet of that state is computed: its SAFE grips and
# the size of the SAFE orbit of the class's exceptional hole 3239 (type 9). The same for class 3805 and its exceptional
# hole 541 (type 6). The SAFE alphabet and its orbits depend only on the off geometry, so the hole of the state built
# here (the piece's own home chamber) does not change them.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cov24_reference.py
import sys,json,time
from pathlib import Path
from types import SimpleNamespace
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s,numpy as np
M=s.restore.menu('I_a');ctx=s.CL.ctx;reg=ctx.regions
R=s.restore.element(M,0,45)
for p,h in [(2335,3239),(3805,541)]:
  st=s.sim.State(ctx,menu=M);st.pose_id[p]=st._intern_matrix(R);st._compact()
  off=s.B.off_pieces(st).tolist();holes=s.B.hole_set(st).tolist();assert off==[p] and holes==[p]
  G=s.B.Alphabet(SimpleNamespace(st=st),M);orb,sz=G.orbits()
  print('REFERENCE',json.dumps({'piece':p,'pose':[0,45],'off':off,'holes':holes,'type':int(reg.orbit_of[p]),'exceptional_hole':h,
    'hole_type':int(reg.orbit_of[h]),'safe_grips':len(G.safe),'safe_letters':len(G.letters),'orbits':len(sz),
    'orbit_of_exceptional_hole':int(sz[orb[h]])}),flush=True)
print('COMPLETED_CPU',round(time.process_time(),1),flush=True)
