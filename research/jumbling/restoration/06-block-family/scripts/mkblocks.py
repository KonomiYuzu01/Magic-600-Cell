"""Write one target file per rigid block of a state (theory blocks.Blocks: components of 'no cut separates').
usage: mkblocks.py <theory dir> <E-chain scripts dir> <pose table> <state name> <out dir>"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402

table, name, out = sys.argv[3], sys.argv[4], Path(sys.argv[5])
out.mkdir(parents=True, exist_ok=True)
C = GL.Ctx(table)
rows = []
for j, b in enumerate(C.bl.blocks):
    inside = [int(c) for c in C.bl.I[j]]
    adm = [c for c in inside if C.R.inside(c) is not None]
    doc = {'state': name, 'digest': C.st.digest(), 'block': j, 'size': len(b), 'pieces': [int(p) for p in b],
           'inside': inside, 'movers': adm, 'straddled': [int(c) for c in C.bl.S[j]]}
    (out / f'{name}-b{j:02d}.json').write_text(json.dumps(doc) + '\n')
    rows.append((j, len(b), len(adm), int(b[0])))
print(json.dumps({'state': name, 'digest': C.st.digest(), 'off': int(len(C.off)), 'blocks': rows}))
