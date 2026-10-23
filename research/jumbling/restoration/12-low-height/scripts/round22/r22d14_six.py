# Theory round 22, line (d), round 2 (R22D-14): at the depth 100 stop of seed 7102, the six-letter family
# a b c a^-1 c^-1 b^-1 (a a lowering letter at cap 61, b and c at free grips of the new class) has 704 cap
# combinations; 24 touch a target and all 24 fail the footprint test F(x) = F(y), so no word of the family returns
# every outside piece exactly.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d14_six.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d14_six.py
from pathlib import Path
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/r22d04_free_grips.py')
exec(p.read_text().split('for j in [15,')[0])
from exact import Q5,dot
R2=Q5(96,-32)
assert all((dot(v,v)-R2).sign()<=0 for vs in C.regions.rep_vertices for v in vs)
def meet(x,y):
 return ((C.data.NN+dot(C.data.N[x],C.data.N[y]))*R2-2*C.data.KAPPA*C.data.KAPPA).sign()>=0
near=[c for c in range(600) if meet(61,c)]
assert len(near)==81
Z=[st.classify(c,need_inside=True,stop_at_straddle=True) for c in range(600)]
assert not any(cl.uncertain for cl in Z)
lat0=st._kpose()[st.pose_id]>=0
L=cap.inside[lat0[cap.inside]]; assert len(L)==2361
F={c:np.isin(L,Z[c].inside).tobytes() for c in range(600) if Z[c].straddle is None}
total=[0,0,0]
for j in [15,18,22,33,52,53,54,55,56,57,58,59]:
 o=st.apply(sim.Twist(C,61,R.element(M,61,j),'menu',{'menu':M.name}))
 assert o.applied and not o.uncertain
 info={pid:T.pose_tree(st._pose_matrix(pid)) for pid in np.unique(st.pose_id).tolist()}
 assert all(info[int(st.pose_id[p])][0]==2 for p in Uall)
 Y=Z.copy()
 for c in near:
  Y[c]=st.classify(c,need_inside=True,stop_at_straddle=True)
  assert not Y[c].uncertain
 lat=st._kpose()[st.pose_id]>=0; row=[0,0,0]
 for q in sorted({int(info[int(st.pose_id[p])][2]) for p in Uall}):
  H=np.array([p for p in Uall if info[int(st.pose_id[p])][2]==q],np.int64)
  own=lat|np.array([info[int(pid)][0]==2 and info[int(pid)][2]==q for pid in st.pose_id])
  A=[c for c in range(600) if branches[c]==q and Z[c].straddle is None and Y[c].straddle is None and own[Y[c].inside].all()]
  touch={c:bool(np.intersect1d(H,Y[c].inside).size) for c in A}
  for x in A:
   for y in A:
    if x==y or sum([meet(61,x),meet(61,y),meet(x,y)])<2: continue
    row[0]+=1
    if not (touch[x] or touch[y]): continue
    row[1]+=1
    if F[x]==F[y]: row[2]+=1
 print(j,row,flush=True)
 total=[a+b for a,b in zip(total,row)]
 st.undo(); assert st.digest()==r['digest']
print('totals',total,'letter_words',[n*59*59 for n in total])
