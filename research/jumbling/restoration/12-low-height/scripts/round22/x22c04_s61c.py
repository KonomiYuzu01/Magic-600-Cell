# Theory round 22, X22C-04 (line (c)): home-cell occupancy and holes at the s61c state.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/x22c04_s61c.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/x22c04_s61c.py
import sys,resource,json
from pathlib import Path
import numpy as np
resource.setrlimit(resource.RLIMIT_CPU,(600,600))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s
p='research/jumbling/restoration/08-units-and-clean-steps/certs/stop62/cm233/inputs/stop62-61-c-table.json.gz'
st,M,_=s.load('table',p)
N=len(st.pose_id);home=s.CL.data.sig_matrix(np.arange(N));members,_=s.CL.classes();typ=np.asarray(s.CL.ctx.regions.orbit_of)
omega=s.B.cell_chambers(home,members,0,[30,145]);holes=s.B.hole_set(st);inside=np.intersect1d(omega,holes)
hole_t=np.bincount(typ[holes],minlength=36);hin_t=np.bincount(typ[inside],minlength=36)
print(json.dumps({'digest':st.digest(),'off':int(st.off_lattice_count()),'cell':[30,145],'home_chambers':len(omega),'home_occupied':len(omega)-len(inside),'holes_inside':len(inside),'holes_outside':len(holes)-len(inside),'T1prime_type_deficit':int(np.maximum(0,hole_t-(hole_t-hin_t)).sum()),'holes_inside_by_type':{int(t):int(v) for t,v in enumerate(hin_t) if v}}))
