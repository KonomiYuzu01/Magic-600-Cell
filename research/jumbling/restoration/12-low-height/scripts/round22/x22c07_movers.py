# Theory round 22, X22C-07 (line (c)): a mover for each blocking group of the depth 60 stop (Theorem FC), read from
# the committed records. The senior reviewer's check, kept as written apart from this header; the integrator re-ran it
# and got the output in runs/round22/x22c07_movers.txt. Research only, headless Linux.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/x22c07_movers.py
import json
from pathlib import Path
b=Path('research/jumbling/restoration/12-low-height/runs')
s=json.loads((b/'screen-k60.json').read_text());o=json.loads((b/'order-k60.json').read_text());u={r['unit']:r for r in s['units']}
for grp in o['blocking_groups']:
 S=set(grp['units']);w=[]
 for k in sorted(S):
  for c in u[k]['cell']:
   if str(c) not in u[k]['grip_status']:continue
   st=set(u[k]['grip_status'][str(c)]['straddled_by'])
   if not st&S:w.append([k,c,sorted(st)])
 print(json.dumps({'group':sorted(S),'FC_record_witness':w[0] if w else None}))
d=json.loads((b/'eb4-k60.json').read_text())
print('total_passing_Z',sum(r['letters_making_it_ready'] for r in d['at_Z']['units']))
for key in ('at_Z','at_Z_prime'):
 print(key,'no_r1',[r['unit'] for r in d[key]['units'] if not r['letters_making_it_ready']])
