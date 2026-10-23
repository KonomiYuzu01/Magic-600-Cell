# Theory round 22, line (d), round 4 (R22D-43): at the seed 7102 stop after R22D-16 (state dceb9359...), the 38-letter
# strict step a W a^-1 with a = (61, 57) and W = [(110, 3), (285, 58)]^9 lands exactly the twelve pieces of R22D-21
# (six of unit 408 and six of unit 409); checked in memory with the strict' contract of stepchk.py.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d43_twelve.txt (it prints the replay of R22D-16 first). About 1 CPU minute.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d43_twelve.py
import sys,json,runpy,resource
from pathlib import Path
from collections import Counter
resource.setrlimit(resource.RLIMIT_CPU,(1190,1200))
z=runpy.run_path('research/jumbling/restoration/12-low-height/scripts/round22/r22d16_step.py')
st,M,E,chk=z['st'],z['M'],z['E'],z['chk']
U=[789,1223,1504,1513,1514,1516,1518,1520,1523,1524,2944,2949]
word=[
 [61,57],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [110,3],[285,58],[110,9],[285,26],
 [61,57]]
docs={'unit':{'pieces':U},'word':{'word':word,'input_digest':'dceb9359b1b549c32afa36c2f88f6cd2cb6ef75feacef1b675f5c99e6b20a17b','output_digest':'f0290f93bea0295ecc00c2a42f725abfd59d3c37732834bb82f5da41c38fa793'}}
class MemoryPath:
 def __init__(self,name):self.name=name
 def read_text(self):return json.dumps(docs[self.name])
 def write_text(self,value):
  row=json.loads(value);assert row['strict_prime'] and row['output_digest_matches'] and row['others_landed']==0;return len(value)
chk.Path=MemoryPath;chk.O.start=lambda table:(E.R.Restorer(st,M),M)
assert chk.main(['state','unit','word','stdout','--require-strict'])==0
old=json.loads(Path('research/jumbling/restoration/12-low-height/runs/eb6-k100.json').read_text())['units']
split={str(u['first']):sorted(set(u['piece_ids'])&set(U)) for u in old if set(u['piece_ids'])&set(U)}
assert set(split)=={'408','409'} and all(len(ps)==6 for ps in split.values())
print('ORIGINAL_UNITS',json.dumps(split))
info={int(p):E.T.pose_tree(st._pose_matrix(int(p))) for p in set(st.pose_id.tolist())}
print('END_HEIGHTS',json.dumps(dict(Counter(int(info[int(p)][0]) for p in st.pose_id))))
