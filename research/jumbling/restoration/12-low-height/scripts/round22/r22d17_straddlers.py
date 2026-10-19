# Theory round 22, line (d), round 2 (R22D-17): at the depth 100 stop of seed 7102, the straddlers of caps 150 and
# 179 with their left heights (cap 179: twenty, all at height 2; cap 150: the height-4 piece 6780 and three at height 2).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d17_straddlers.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22d17_straddlers.py
from pathlib import Path
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/r22d04_free_grips.py')
exec(p.read_text().split('for j in [15,')[0])
for c in [150,179]:
 s=E.straddler_set(st,c).tolist()
 print(c,s,[int(info[int(st.pose_id[p])][0]) for p in s])
