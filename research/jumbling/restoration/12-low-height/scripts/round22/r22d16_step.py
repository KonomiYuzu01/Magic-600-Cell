# Theory round 22, line (d), round 2 (R22D-16): the 28-letter strict' step a V W V^-1 a^-1 (a = (61,57),
# V = (287,3), W = [(110,54),(149,3)]^6) at the depth 100 stop of seed 7102 lands unit 391 and 41 other height-4
# pieces. The script rebuilds the stop from its record and runs stepchk.py on the word with in-memory files.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22d16_step.txt.
# It writes no file. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d16_step.py
import sys,json,importlib.util
from pathlib import Path
sys.path[:0]=['research/jumbling/restoration/12-low-height/scripts','research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import eb6 as E
rec=json.loads(Path('research/jumbling/restoration/11-scramble-census/runs/round2/ia-k100-s7102.json').read_text())
C=E.sim.get_context(); M=E.R.menu('I_a'); st=E.sim.State(C,filtered=False,menu=M)
for c,j in rec['history']+rec['moves']:
 o=st.apply(E.sim.Twist(C,c,E.R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
 assert o.applied and not o.uncertain
word=[[61,57],[287,3],[110,54],[149,3],[110,24],[149,9],[110,54],[149,3],[110,24],[149,9],[110,54],[149,3],[110,24],[149,9],[110,54],[149,3],[110,24],[149,9],[110,54],[149,3],[110,24],[149,9],[110,54],[149,3],[110,24],[149,9],[287,9],[61,57]]
spec=importlib.util.spec_from_file_location('stepchk','research/jumbling/restoration/12-low-height/scripts/round22/stepchk.py')
chk=importlib.util.module_from_spec(spec); spec.loader.exec_module(chk)
docs={'unit':{'pieces':[391]},'word':{'word':word,'input_digest':'4ca440085d743d680163a35d735ff6a7c35796a57cba720ab4037a49887734db','output_digest':'dceb9359b1b549c32afa36c2f88f6cd2cb6ef75feacef1b675f5c99e6b20a17b'}}
class MemoryPath:
 def __init__(self,name): self.name=name
 def read_text(self): return json.dumps(docs[self.name])
 def write_text(self,value):
  print(value,flush=True)
  assert json.loads(value)['strict_prime']
  return len(value)
chk.Path=MemoryPath
chk.O.start=lambda table:(E.R.Restorer(st,M),M)
sys.argv=['stepchk','state','unit','word','stdout']
chk.main()
