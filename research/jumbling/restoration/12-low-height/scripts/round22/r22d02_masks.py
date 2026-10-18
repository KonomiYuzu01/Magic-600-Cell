# Theory round 22, R22D-02: the senior reviewer's exact computation, kept as written apart from this header;
# the integrator re-ran it and got the output in runs/round22/r22d02_masks.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22d02_masks.py
from itertools import product
for word in ((1,2,3,-1,-3,-2),(1,2,3,-2,-1,-3)):
 out={'return':[],'move':[],'conditional':[]}
 for bits in product((0,1),repeat=6):
  st=[]
  for x,b in zip(word,bits):
   if b:
    if st and st[-1]==-x:st.pop()
    else:st.append(x)
  while len(st)>1 and st[0]==-st[-1]:st=st[1:-1]
  out['return' if not st else 'move' if len(st)==1 else 'conditional'].append(''.join(map(str,bits)))
 print(word,out)
print('connected_six_bound',2*3*600*81**2*59**3)
print('r1_meeting_bound',2574*600*80*59**2)
