# Theory round 22, R22C-04 (line (c)): the pieces and type vector of b06 at the depth 40 stop.
# The senior reviewer's exact computation, kept as written apart from this header; the integrator re-ran it and got
# the output in runs/round22/r22c04_b06.txt. Research only, headless Linux; exact geometry.
# Run from the repository root: python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22c04_b06.py
import sys,resource,json
from pathlib import Path
from collections import Counter
from types import SimpleNamespace
resource.setrlimit(resource.RLIMIT_CPU,(300,300))
sys.path.insert(0,str(Path('research/jumbling/restoration/12-low-height/scripts').resolve()))
import screen as s
st,M,_=s.load('record','research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json')
b=s.BL.Blocks(SimpleNamespace(st=st),M)
print('b06',json.dumps({'pieces':list(map(int,b.blocks[6])),'types':dict(sorted(Counter(map(int,s.CL.ctx.regions.orbit_of[b.blocks[6]])).items()))}))
