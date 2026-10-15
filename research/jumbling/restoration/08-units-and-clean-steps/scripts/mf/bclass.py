"""Block classes of a state: size, movers (unstraddled grips whose cap holds the block), frame domains.

usage: bclass.py <theory dir> <E-chain scripts dir> <pose table> <out json>
"""
import json
import sys
from collections import Counter
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402

table, out = sys.argv[3], sys.argv[4]
C = GL.Ctx(table)
rows = []
for j, b in enumerate(C.bl.blocks):
    inside = [int(c) for c in C.bl.I[j]]
    adm = [c for c in inside if C.R.inside(c) is not None]
    doms = Counter(int(C.D.dom[p]) for p in b)
    rows.append({'block': j, 'first': int(min(b)), 'size': len(b), 'movers': adm, 'inside': inside,
                 'straddled': [int(c) for c in C.bl.S[j]], 'frames': {str(k): v for k, v in doms.items()},
                 'pieces': [int(p) for p in b]})
cls = Counter()
for r in rows:
    m = 'frozen' if not r['movers'] else ('one mover' if len(r['movers']) == 1 else '2+ movers')
    f = 'one frame' if len(r['frames']) == 1 else 'several frames'
    r['class'] = m if m == 'frozen' else f'{m}, {f}'
    cls[r['class']] += 1
doc = {'table': table, 'off': int(len(C.off)), 'blocks': len(rows), 'classes': dict(cls), 'rows': rows}
Path(out).write_text(json.dumps(doc) + '\n')
print(json.dumps({'off': doc['off'], 'blocks': len(rows), 'classes': dict(cls)}))
for c in sorted(cls):
    sm = sorted((r for r in rows if r['class'] == c), key=lambda r: r['size'])[:5]
    print(c, [(r['size'], r['first'], len(r['movers']), len(r['frames'])) for r in sm])
