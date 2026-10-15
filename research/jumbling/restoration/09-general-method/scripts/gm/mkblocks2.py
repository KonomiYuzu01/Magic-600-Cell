"""Write one target file per restoration unit of a state (menu I_a): the rigid blocks (theory blocks.Blocks:
components of 'no cut separates'), and for a block whose pieces lie in several frame domains, one unit per domain
with the rest of the block as its free set (a partial step of the block's restoration).

usage: mkblocks2.py <theory dir> <E-chain scripts dir> <pose table> <state name> <out dir>
Prints one JSON line: state, digest, off, blocks [[j, size, movers, first, frames]], units
[[key, block first, size, target file, free file or null]].
"""
import json
import sys
from collections import defaultdict
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
rows, units = [], []
for j, b in enumerate(C.bl.blocks):
    b = sorted(int(p) for p in b)
    inside = [int(c) for c in C.bl.I[j]]
    adm = [c for c in inside if C.R.inside(c) is not None]
    by = defaultdict(list)
    for p in b:
        by[int(C.D.dom[p])].append(p)
    doc = {'state': name, 'digest': C.st.digest(), 'block': j, 'size': len(b), 'pieces': b, 'inside': inside,
           'movers': adm, 'straddled': [int(c) for c in C.bl.S[j]], 'frames': {str(d): len(v) for d, v in by.items()}}
    bf = out / f'{name}-b{j:02d}.json'
    bf.write_text(json.dumps(doc) + '\n')
    rows.append((j, len(b), len(adm), b[0], len(by)))
    if len(by) == 1:
        units.append((b[0], b[0], len(b), str(bf), None))
        continue
    for d, ps in sorted(by.items()):
        rest = [p for p in b if p not in set(ps)]
        tf = out / f'{name}-b{j:02d}-d{d}.json'
        ff = out / f'{name}-b{j:02d}-d{d}-free.json'
        tf.write_text(json.dumps({'state': name, 'block': j, 'frame': d, 'pieces': ps}) + '\n')
        ff.write_text(json.dumps({'state': name, 'block': j, 'frame': d, 'pieces': rest}) + '\n')
        units.append((ps[0], b[0], len(ps), str(tf), str(ff)))
print(json.dumps({'state': name, 'digest': C.st.digest(), 'off': int(len(C.off)), 'blocks': rows, 'units': units}))
