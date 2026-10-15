"""Independent replay of a unit chain (clean steps of Lemma U) from a start pose table.

For each step (a word with its recorded input and output digests, the unit T and the free set F): check the input
digest; apply every twist on the simulator (the simulator refuses a twist whose cap a region straddles); check the
output digest; recompute from exact pose codes the restored pieces (off before, on after), the collateral (on
before, off after) and the moved off pieces (off before and after, exact pose changed). A step passes when every
twist was admitted, both digests match, every piece of T is restored, the collateral is empty, every moved or
restored piece lies in T or F, and the off count falls. At the end, report the off count and whether every piece
is on the lattice.
Uses only the simulator, the pose table and the recorded words and sets; none of the search scripts.

usage: verify_u.py <research/jumbling> <echain dir> <chain json> <out jsonl>
chain json: {"name", "start" (pose table), "start_digest", "steps": [{"label", "word", "unit", "free"}]};
"word" is a word-full.json ({"word", "input_digest", "output_digest"}); "unit" and "free" are JSON files
{"pieces": [...]} ("free" may be null); paths are relative to the chain file.
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


def pieces(rel):
    if not rel:
        return np.zeros(0, np.int64)
    return np.asarray(sorted(json.loads((base / rel).read_text())['pieces']), np.int64)


d0 = st.digest()
ok_all = d0 == chain['start_digest']
emit({'chain': chain['name'], 'start_digest_ok': ok_all, 'load_seconds': round(time.time() - t0, 1)})
c_s = codes.of(st)
for step in chain['steps']:
    t1 = time.time()
    w = json.loads((base / step['word']).read_text())
    T, F = pieces(step['unit']), pieces(step.get('free'))
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
    allowed = np.zeros(len(c_s), bool)
    allowed[T] = True
    allowed[F] = True
    row = {'label': step['label'], 'twists': len(w['word']), 'applied': applied, 'error': error,
           'input_digest_ok': d_in == w['input_digest'], 'output_digest_ok': d_out == w['output_digest'],
           'off_before': int(off_s.sum()), 'off_after': int(off_e.sum()), 'unit': int(len(T)), 'free': int(len(F)),
           'unit_off_before': bool(off_s[T].all()), 'unit_restored': bool((~off_e[T]).all()),
           'restored': int(len(rest)), 'collateral': int(len(coll)), 'moved_off': int(len(mov)),
           'moved_outside_free': int((~allowed[mov]).sum()), 'restored_outside_unit_or_free': int((~allowed[rest]).sum()),
           'seconds': round(time.time() - t1, 1)}
    row['ok'] = bool(error is None and row['input_digest_ok'] and row['output_digest_ok'] and row['unit_off_before']
                     and row['unit_restored'] and row['collateral'] == 0 and row['moved_outside_free'] == 0
                     and row['restored_outside_unit_or_free'] == 0 and row['off_after'] < row['off_before'])
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
