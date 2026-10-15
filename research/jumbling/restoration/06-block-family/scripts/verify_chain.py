"""Independent replay of block-family restorations: chains of steps from a start pose table.

For each step of a chain (a word with its recorded input and output digests, and the target pieces), starting from
the chain's pose table: check the input digest; apply every twist on the simulator (the simulator refuses a twist
whose cap a region straddles); check the output digest; and compare the end with the start of the step:
- target: every target piece off at the start and on the lattice at the end;
- collateral: lattice pieces of the step's input that are off at the end (must be 0);
- protected: every other off piece of the input keeps its exact pose (4x4 exact matrix);
- lattice kept: lattice pieces of the input on the lattice at the end, and how many of them changed K pose.
Writes one JSON line per step and a final line with the verdict. Uses only the simulator, the pose table and the
words; none of the search scripts.

usage: verify_chain.py <research/jumbling> <echain dir> <chain json> <out jsonl>
chain json: {"name": ..., "start": <pose table>, "start_digest": ...,
             "steps": [{"label": ..., "word": <json {word, input_digest, output_digest}>, "target": <json {pieces}>}]}
Paths in the chain file are relative to the chain file.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
J = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(J / 'theory'))
sys.path.insert(0, str(J))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import conjclose as CC  # noqa: E402
import ghostlib as GL  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

chain_file = Path(sys.argv[3]).resolve()
base = chain_file.parent
out = Path(sys.argv[4])
chain = json.loads(chain_file.read_text())
t0 = time.time()
R, M = GL.O.start(str(base / chain['start']))
st = R.st
codes = CC.Codes()
ok_all = True
lines = []


def emit(row):
    lines.append(row)
    print(json.dumps(row), flush=True)


d0 = st.digest()
emit({'chain': chain['name'], 'start_digest': d0, 'start_digest_ok': d0 == chain.get('start_digest', d0),
      'load_seconds': round(time.time() - t0, 1)})
ok_all &= d0 == chain.get('start_digest', d0)
for step in chain['steps']:
    t1 = time.time()
    w = json.loads((base / step['word']).read_text())
    T = np.asarray(sorted(json.loads((base / step['target']).read_text())['pieces']), np.int64)
    d_in = st.digest()
    c_s = codes.of(st)
    off_s = c_s < 0
    lat_s = ~off_s
    inT = np.zeros(len(c_s), bool)
    inT[T] = True
    prot = np.flatnonzero(off_s & ~inT)
    poses_s = {int(p): to_tuple(st.pose(int(p))) for p in prot.tolist()}
    applied, error = 0, None
    try:
        for c, i in w['word']:
            R.apply(int(c), int(i))
            applied += 1
    except Exception as e:  # noqa: BLE001 - a refused twist is a verification failure, recorded as such
        error = repr(e)[:300]
    d_out = st.digest()
    c_e = codes.of(st)
    off_e = c_e < 0
    row = {'label': step['label'], 'twists': len(w['word']), 'applied': applied, 'error': error,
           'input_digest_ok': d_in == w['input_digest'], 'output_digest_ok': d_out == w['output_digest'],
           'off_before': int(off_s.sum()), 'off_after': int(off_e.sum()), 'target': int(len(T)),
           'target_off_at_start': int(off_s[T].sum()), 'target_on_at_end': int((~off_e[T]).sum()),
           'collateral': int((lat_s & off_e).sum()),
           'protected': int(len(prot)),
           'protected_exact': sum(1 for p, m in poses_s.items() if off_e[p] and to_tuple(st.pose(p)) == m),
           'lattice_kept': int((lat_s & ~off_e).sum()), 'lattice': int(lat_s.sum()),
           'lattice_changed_K_pose': int((lat_s & ~off_e & (c_e != c_s)).sum()),
           'seconds': round(time.time() - t1, 1)}
    row['ok'] = bool(error is None and row['input_digest_ok'] and row['output_digest_ok']
                     and row['target_off_at_start'] == row['target'] and row['target_on_at_end'] == row['target']
                     and row['collateral'] == 0 and row['protected_exact'] == row['protected']
                     and row['lattice_kept'] == row['lattice'])
    ok_all &= row['ok']
    emit(row)
    if error is not None:
        break
emit({'chain': chain['name'], 'all_ok': bool(ok_all), 'steps': len(chain['steps']),
      'seconds': round(time.time() - t0, 1)})
out.write_text(''.join(json.dumps(r) + '\n' for r in lines))
