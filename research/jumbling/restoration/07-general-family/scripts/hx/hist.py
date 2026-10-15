"""History anatomy of the W-J I_a walk: for each record, retained or jumble, pieces moved, lattice pieces sent off (L),
off pieces put on the lattice (R), off pieces inside the cap before the twist (O). Exact (simulator pose codes).

usage: hist.py <research/jumbling> <n records> <out json>
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
J, N, OUT = sys.argv[1], int(sys.argv[2]), sys.argv[3]
for p in (J + '/theory', J, J + '/fixtures'):
    sys.path.insert(0, p)
import restore  # noqa: E402
import sim  # noqa: E402
import wj  # noqa: E402

ctx = restore.ctx
fixture, M = wj.load(ctx, 'I_a')
st = sim.State(ctx, menu=M)


_codes = {}


def poscodes():
    t = np.array([_codes.setdefault(p.key, len(_codes)) for p in st._poses], np.int64)
    return t[st.pose_id]


def offmask():
    k = np.array([p.kidx for p in st._poses], np.int64)
    return k[st.pose_id] < 0


rows = []
t0 = time.time()
off0 = offmask()
for n, rec in enumerate(fixture['journal']['records'][:N]):
    pc0 = poscodes()
    out = st.apply(sim.Twist.from_record(ctx, rec))
    assert out.applied
    inside = np.flatnonzero(poscodes() != pc0)
    assert len(inside) == out.moved, (len(inside), out.moved)
    off1 = offmask()
    L = int((~off0[inside] & off1[inside]).sum())
    R = int((off0[inside] & ~off1[inside]).sum())
    O = int(off0[inside].sum())
    rows.append({'n': n + 1, 'grip': rec['grip'], 'label': rec['params']['label'], 'retained': rec['retained'],
                 'moved': int(len(inside)), 'L': L, 'R': R, 'O': O, 'off': int(off1.sum())})
    print(json.dumps(rows[-1]), file=sys.stderr, flush=True)
    off0 = off1
Path(OUT).write_text(json.dumps({'records': rows, 'digest': st.digest(), 'sec': round(time.time() - t0, 1)}, indent=1))
