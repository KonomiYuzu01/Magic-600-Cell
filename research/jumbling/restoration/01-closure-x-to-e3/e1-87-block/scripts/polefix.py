"""Does every menu element of every grip fix the grip's pole n_c (so it maps the cap's half-space onto itself)?"""
import json, sys
from pathlib import Path
sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH)); sys.path.insert(0, str(TH.parent))
import blocks as BL
from exact import matvec
R, M = BL.O.start(sys.argv[2])
data = BL.CL.ctx.data
bad = []
n = 0
for c in range(600):
    for i in range(len(M.items)):
        g = BL.restore.element(M, c, i)
        n += 1
        if tuple(matvec(g, data.N[c])) != tuple(data.N[c]):
            bad.append([c, i])
print(json.dumps({'checked': n, 'not_fixing_pole': len(bad), 'examples': bad[:5]}))
