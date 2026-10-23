# Theory round 22, line (d), round 2 (R22D-15): at the depth 100 stop of seed 7102, after the lowering items 55, 57
# and 59 at cap 61, the class cells the high units start in, their home chambers and the occupied ones outside every
# SAFE cap (which no SAFE word can vacate).
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d15_source_cells.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d15_source_cells.py
from pathlib import Path
p=Path('research/jumbling/restoration/12-low-height/scripts/round22/r22d06_fixed_chambers.py')
exec(p.read_text().split('for j,g in ')[0])
from sim.kernel import Evaluator,pullback_form
home=C.data.sig_matrix(allp)
branches=[E.T.branch(tuple(n)) for n in C.data.N]
for j,g in [(55,215),(57,110),(59,21)]:
 o=st.apply(sim.Twist(C,61,R.element(M,61,j),'menu',{'menu':M.name}))
 assert o.applied and not o.uncertain
 off=st._kpose()[st.pose_id]<0
 q=E.T.pose_tree(st._pose_matrix(int(st.pose_id[391])))[2]
 poles=[c for c in range(600) if branches[c]==q]
 safe=[]
 for c in range(600):
  cl=st.classify(c,need_inside=True,stop_at_straddle=True)
  assert not cl.uncertain
  if cl.straddle is None and not off[cl.inside].any(): safe.append(c)
 holes=np.zeros(len(allp),bool); holes[B.hole_set(st)]=True
 fixed_occ=(~holes)&(~home[:,safe].any(axis=1))
 cells={}; units={}; evs={}
 for u in r['units'][:5]:
  uc={}
  for piece in u['piece_ids']:
   pid=int(st.pose_id[piece])
   if pid not in evs:
    evs[pid]=[(c,Evaluator(pullback_form(st._pose_matrix(pid),C.data.N[c]))) for c in poles]
   cell=[]
   for c,ev in evs[pid]:
    v=ev.classify(C.regions.vform(piece))[0]; assert v in ('in','out')
    if v=='in': cell.append(c)
   cell=tuple(cell); uc[cell]=uc.get(cell,0)+1
   if cell not in cells:
    X=allp[(home[:,poles]==np.isin(poles,cell)[None,:]).all(axis=1)]
    bad=X[fixed_occ[X]]
    cells[cell]=[len(X),len(bad),bad[:3].tolist()]
  units[u['first']]=[[list(c),n] for c,n in sorted(uc.items())]
 print(json.dumps({'item':j,'cells':[[list(c),v] for c,v in sorted(cells.items())],'unit_cells':units}),flush=True)
 st.undo(); assert st.digest()==r['digest']
