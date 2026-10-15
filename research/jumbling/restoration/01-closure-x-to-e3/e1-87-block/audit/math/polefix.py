"""Math audit of Lemma H' (M3): every menu element of grip g is orthogonal and fixes the pole n_g (exact Q5 arithmetic).
Usage: python3 -I polefix.py <theory dir> <table json> <out json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
TABLE, OUT = sys.argv[2], Path(sys.argv[3])
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import orient as O  # noqa: E402
import restore  # noqa: E402
from exact import matvec, matmul, transpose, identity  # noqa: E402

R, M = O.start(TABLE)
data = R.st.ctx.data
I4 = identity(4)
n_items = len(M.items)
fix = orth = total = 0
bad = []
for g in range(600):
    n = data.N[g]
    for i in range(n_items):
        E = restore.element(M, g, i)
        total += 1
        f = list(matvec(E, n)) == list(n)
        o = matmul(transpose(E), E) == I4
        fix += f
        orth += o
        if not (f and o) and len(bad) < 10:
            bad.append([g, i, bool(f), bool(o)])
out = {'grips': 600, 'items_per_grip': n_items, 'total': total, 'fix_pole': fix, 'orthogonal': orth, 'failures': bad}
OUT.write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps(out), file=sys.stderr)
