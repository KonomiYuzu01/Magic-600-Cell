"""Independent replay of a heap-peeling chain: steps of single letters (or short words) from a start pose table.

For each step (a word with its recorded input and output digests and its recorded sets): check the input digest;
apply every twist on the simulator (the simulator refuses a twist whose cap a region straddles); check the output
digest; recompute from exact pose codes the restored pieces (off before, on after), the collateral (on before, off
after) and the moved off pieces (off before and after, exact pose changed), and require each to equal the recorded
set; require the off count to fall. At the end, report the off count and whether every piece is on the lattice.
Uses only the simulator, the pose table and the recorded words and sets; none of the search scripts.

usage: verify_hp.py <research/jumbling> <echain dir> <chain json> <out jsonl>
chain json: {"name", "start" (pose table), "start_digest", "steps": [{"label", "word", "sets"}]}; paths relative to it.
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

chain_file = Path(sys.argv[3]).resolve()
base = chain_file.parent
out = Path(sys.argv[4])
chain = json.loads(chain_file.read_text())
t0 = time.time()
start = Path(chain['start'])
R, M = GL.O.start(str(start if start.is_absolute() else base / start))
st = R.st
codes = CC.Codes()
lines = []


def emit(row):
    lines.append(row)
    print(json.dumps(row), flush=True)


d0 = st.digest()
ok_all = d0 == chain['start_digest']
emit({'chain': chain['name'], 'start_digest_ok': ok_all, 'load_seconds': round(time.time() - t0, 1)})
c_s = codes.of(st)
for step in chain['steps']:
    t1 = time.time()
    w = json.loads((base / step['word']).read_text())
    sets = json.loads((base / step['sets']).read_text())
    d_in = st.digest()
    applied, error = 0, None
    try:
        for c, i in w['word']:
            R.apply(int(c), int(i))
            applied += 1
    except Exception as e:  # noqa: BLE001 - a refused twist is a verification failure
        error = repr(e)[:300]
    d_out = st.digest()
    c_e = codes.of(st)
    off_s, off_e = c_s < 0, c_e < 0
    rest = np.flatnonzero(off_s & ~off_e)
    coll = np.flatnonzero(~off_s & off_e)
    mov = np.flatnonzero(off_s & off_e & (c_s != c_e))
    row = {'label': step['label'], 'twists': len(w['word']), 'applied': applied, 'error': error,
           'input_digest_ok': d_in == w['input_digest'], 'output_digest_ok': d_out == w['output_digest'],
           'off_before': int(off_s.sum()), 'off_after': int(off_e.sum()),
           'restored': int(len(rest)), 'collateral': int(len(coll)), 'moved_off': int(len(mov)),
           'restored_matches': rest.tolist() == sorted(sets['restored']),
           'collateral_matches': coll.tolist() == sorted(sets['collateral']),
           'moved_matches': mov.tolist() == sorted(sets['moved']),
           'seconds': round(time.time() - t1, 1)}
    row['ok'] = bool(error is None and row['input_digest_ok'] and row['output_digest_ok'] and row['restored_matches']
                     and row['collateral_matches'] and row['moved_matches'] and row['off_after'] < row['off_before'])
    ok_all &= row['ok']
    emit(row)
    c_s = c_e
    if error is not None:
        break
final_off = int((c_s < 0).sum())
emit({'chain': chain['name'], 'all_ok': bool(ok_all), 'steps': len(chain['steps']), 'final_off': final_off,
      'all_on_lattice': final_off == 0 and bool(st.is_lattice()), 'final_digest': st.digest(),
      'seconds': round(time.time() - t0, 1)})
out.write_text(''.join(json.dumps(r) + '\n' for r in lines))
