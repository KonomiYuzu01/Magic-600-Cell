# Theory round 22, line (d), round 4 (R22D-42): at the depth 100 stop of seed 7108 (state dff31c35...), the 50-letter
# strict step a W a^-1 with a = (415, 59), W = [(91, 22), (268, 9)]^12 and a^-1 = (415, 29) lands all 111 height-4
# pieces; checked in memory with the strict' contract of stepchk.py, and the pieces that the start tags of R22D-22
# allow inside cap 415 at the end of the middle counted.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d42_step7108.txt. About 1 CPU minute.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d42_step7108.py
import sys,json,importlib.util,resource
from pathlib import Path
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
import numpy as np
r=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round3/ia-k100-s7108.json').read_text())
C=E.sim.get_context();M=E.R.menu('I_a');st=E.sim.State(C,filtered=False,menu=M)
for c,j in r['history']+r['moves']:
 o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
start=(st.pose_id.copy(),list(st._poses),dict(st._lookup))
off0=st._kpose()[st.pose_id]<0;pose0=E.stable_pose(st)
info={p:E.T.pose_tree(st._pose_matrix(p)) for p in np.unique(st.pose_id).tolist()}
high=[p for p in range(len(st.pose_id)) if info[int(st.pose_id[p])][0]==4]
word=[
 [415,59],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [91,22],[268,9],[91,46],[268,3],
 [415,29]]
spec=importlib.util.spec_from_file_location('stepchk','research/jumbling/restoration/12-low-height/scripts/round22/stepchk.py')
chk=importlib.util.module_from_spec(spec);spec.loader.exec_module(chk)
docs={'unit':{'pieces':high},'word':{'word':word,'input_digest':'dff31c35a7e16cc83c695c1b8423c64432ad1d017359babcd5275eb833f7a8a6','output_digest':'79aaefe2f46166777eebba3105f57d89c692b3cf93457a7d736834683300564e'}}
class MemoryPath:
 def __init__(self,name):self.name=name
 def read_text(self):return json.dumps(docs[self.name])
 def write_text(self,value):
  row=json.loads(value);assert row['strict_prime'] and row['output_digest_matches'];return len(value)
chk.Path=MemoryPath;chk.O.start=lambda table:(E.R.Restorer(st,M),M)
assert chk.main(['state','unit','word','stdout','--require-strict'])==0
off1=st._kpose()[st.pose_id]<0;pose1=E.stable_pose(st)
Y=E.sim.State(C,filtered=False,menu=M);Y.pose_id=start[0].copy();Y._poses=list(start[1]);Y._lookup=dict(start[2])
c,j=word[0];o=Y.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
latY=Y._kpose()[Y.pose_id]>=0
for c,j in word[1:-1]:
 o=Y.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False);assert o.applied and not o.uncertain
latT=Y._kpose()[Y.pose_id]>=0;cl=Y.classify(415,need_inside=True,stop_at_straddle=False);assert cl.straddle is None and not cl.uncertain
witnesses=[int(p) for p in cl.inside if off0[p] and latY[p] and latT[p] and off1[p] and pose0[p]==pose1[p]]
assert all(latT[latY])
print('START_TAG_WITNESSES',json.dumps({'count':len(witnesses),'first':min(witnesses),'all_lattice_of_Y_still_lattice_at_T':bool(latT[latY].all()),'lattice_of_Y_inside_415_at_T':int(latY[cl.inside].sum())}))
