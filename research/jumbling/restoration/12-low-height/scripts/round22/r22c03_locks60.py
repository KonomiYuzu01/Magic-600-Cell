# Theory round 22, R22C-03 (line (c)): occupied locked home chambers at the depth 60 stop.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c03_locks60.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22c03_locks60.py
import sys,json,resource
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(1100,1100))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s
import numpy as np
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json')
bl=s.BL.Blocks(SimpleNamespace(st=st),M)
d=json.loads(Path('research/jumbling/restoration/12-low-height/runs/screen-k60.json').read_text()); assert st.digest()==d['digest']
members,_=s.CL.classes(); home=s.CL.data.sig_matrix(np.arange(len(st.pose_id)))
print('digest',st.digest(),flush=True)
for u in d['units']:
 if not u.get('home_chambers'): continue
 b=u['block']; omega=set(map(int,s.B.cell_chambers(home,members,u['class'],u['cell'])))
 occ=sorted((set(bl.locked[b])-set(bl.vacant_locked[b])) & omega)
 if occ: print(json.dumps({'unit':u['unit'],'first':u['first'],'size':u['size'],'kind':u['kind'],'block_size':len(bl.blocks[b]),'occupied_locked_in_own_home':occ,'N1_fixed_occupied_recorded':u['N_fixed_occupied']}),flush=True)
