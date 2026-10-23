# Theory round 22, R22C-02 (line (c)): occupied home chambers locked to the unit's own block at the depth 40 stop.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c02_locks.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22c02_locks.py
import sys,json,resource
from pathlib import Path
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(1000,1000))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s
import numpy as np
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json')
bl=s.BL.Blocks(SimpleNamespace(st=st),M)
d=json.loads(Path('research/jumbling/restoration/12-low-height/runs/screen-k40.json').read_text())
assert st.digest()==d['digest']
home=s.CL.data.sig_matrix(np.arange(len(st.pose_id)))
members,_=s.CL.classes()
print('digest',st.digest())
for u in d['units']:
 if not u.get('home_chambers'): continue
 b=u['block']; assert len(bl.blocks[b])==u['size'] and int(bl.blocks[b].min())==u['first']
 omega=set(map(int,s.B.cell_chambers(home,members,u['class'],u['cell'])))
 occ=sorted((set(bl.locked[b])-set(bl.vacant_locked[b])) & omega)
 print(json.dumps({'unit':u['unit'],'first':u['first'],'locked':len(bl.locked[b]),'vacant_locked':len(bl.vacant_locked[b]),'occupied_locked_in_own_home':occ,'N1_fixed_occupied_recorded':u['N_fixed_occupied']}))
