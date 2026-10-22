"""Chapter 4 of the jumbling tutorial: the high-phase examples it quotes (headless research).

Read-only. It imports the R22D-25 kernel (r22d25_search.py with r22d25_state.py) and one function of the kernel
plugin (rules_kernel._stop_family), rebuilds the R22D-36 test state Z36 from its 26 letters, and writes one JSON
record. Every cache file goes to --cache; nothing is written under the repository.

Run from the repository root (about three minutes on a cloud machine):
    python3 -I -B research/jumbling/tutorial/scripts/ch04_examples.py --out <new file> --cache <new dir>

Sections of the record:
  A z36       Z36: its snapshot (digest, off count, heights, high units, clean caps, cap 12); the cap-level test of
              caps 0 to 12 and the letter-level test of items 1 to 14 at cap 12 for the height-6 unit, as F1 runs
              them; one search_step of the kernel at Z36 (its candidate and its whole ledger); the protected
              lowering gate of R22D-36's letter (12, 45)
  B after_f1  the state after the F1 word, rebuilt as the kernel's run loop does: its snapshot; one search_step
              with the real D19 program under a short clock (--d19-seconds, default 60), its ledger, and the stop
              family that the kernel plugin reads from that ledger
  C tables    the inverse item of every item of I_a at one grip (the kernel's table), and the number of home
              chambers of every cap at the solved state (least and largest over the 600 caps)
"""
import argparse
import hashlib
import json
import os
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--cache', required=True, type=Path)
parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[4])
parser.add_argument('--d19-seconds', type=float, default=60.0)
args = parser.parse_args()
ROOT = args.root.resolve()
CACHE = args.cache.resolve()
CACHE.mkdir(parents=True, exist_ok=True)
# Before any import: seams.py reads this variable once, else it caches under the repository (seams.py:66).
os.environ['JUMBLING_HOME_CACHE'] = str(CACHE / 'home_centroids.npy')
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
R22 = ROOT / 'research/jumbling/restoration/12-low-height/scripts/round22'
PLUGIN = ROOT / 'research/jumbling/restoration/13-tutorial-runner/scripts'
sys.path[:0] = [str(R22), str(PLUGIN)]
import numpy as np  # noqa: E402
import r22d25_search as S  # noqa: E402  (imports r22d25_state as S.K)
import rules_kernel  # noqa: E402  (only _stop_family is called)

K = S.K
K.set_cache(CACHE)
T0 = time.time()

# The R22D-36 test state (PROOFS-B.md:2206-2211; the fixture WORD36 of test_r22d25.py:39-42).
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]
Z36 = '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261'
END36 = 'd79c03679e3b065164219ebd747cc2de89bc5759c081f7b95e9554cbb8c11ea1'


def plain(value):
    """JSON-safe copy: numpy scalars and arrays become Python values; simulator states are left out.

    A gate's 'state' is a simulator state and is dropped; a ledger cursor's 'state' is a digest string and is kept.
    """
    if isinstance(value, dict):
        return {str(k): plain(v) for k, v in value.items() if not (k == 'state' and not isinstance(v, str))}
    if isinstance(value, (list, tuple)):
        return [plain(v) for v in value]
    if isinstance(value, np.ndarray):
        return plain(value.tolist())
    if isinstance(value, np.generic):
        return value.item()
    return value


def snap(s):
    return {'digest': s.digest, 'off': s.off_count, 'heights': s.heights_census,
            'high_units': [{k: u[k] for k in ('first', 'pieces', 'height', 'branch', 'caps_whole')}
                           for u in s.high_units],
            'height2_units': len(s.height2_units), 'clean_caps': len(s.clean)}


def cap_reason(s, U, c):
    """F1's cap-level test, in its order (r22d25_search.py:200-202)."""
    return ('blocked' if c not in s.clean else
            'target_not_whole' if not (s.labels[U, c] == 1).all() else
            'lattice_inside' if s.lattice_inside[c] else None)


def gate_summary(g):
    keys = ('ok', 'reason', 'kind', 'letters', 'input_digest', 'output_digest', 'off_before', 'off_after',
            'unit_on_lattice', 'collateral', 'others_exact', 'others_landed', 'heights_after')
    return plain({k: g.get(k) for k in keys})


def search_summary(result):
    out = {'status': result['status'], 'reason': result.get('reason'), 'ledger': plain(result['ledger'])}
    if result['status'] == 'candidate':
        out.update(family=result['family'], kind=result['kind'], target=plain(result['target']),
                   params=plain(result['params']), high_exact=result['high_exact'], word=plain(result['word']),
                   gate=gate_summary(result['gate']))
    return out


record = {'format': 'tutorial-ch04-examples/1',
          'scripts': {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                      for p in (R22 / 'r22d25_search.py', R22 / 'r22d25_state.py', R22 / 'r22d19_rule.py',
                                PLUGIN / 'rules_kernel.py')}}

# A. Z36, as the kernel's command line loads a record (r22d25_search.py:497-512).
start = CACHE / 'z36-record.json'
start.write_text(json.dumps({'history': WORD36, 'moves': []}))
state = S.load_start('record', start)
assert state.digest() == Z36, state.digest()
s = K.snapshot(state)
units6 = [u for u in s.high_units if u['height'] == 6]
assert len(units6) == 1
U = units6[0]['piece_ids']
inside12 = np.flatnonzero(s.labels[:, 12] == 1)
off12 = [int(p) for p in inside12 if s.off[p] and int(p) not in U]
A = {'snapshot': snap(s),
     'cap_12': {'clean': 12 in s.clean, 'holds_unit_whole': bool((s.labels[U, 12] == 1).all()),
                'lattice_inside': int(s.lattice_inside[12]), 'other_off_inside': len(off12),
                'other_off_inside_heights': plain(np.unique(s.heights[off12], return_counts=True))},
     'caps_0_to_12': {str(c): cap_reason(s, U, c) for c in range(13)},
     'cap_12_items_1_to_14': {}}
for j in range(1, 15):
    reason = K.single_letter_prune(s, U, 12, j, lowering=True)
    row = {'prune': reason}
    if reason is None:
        row['gate'] = gate_summary(K.protected_lowering(s, U, [[12, j]], 6))
    A['cap_12_items_1_to_14'][str(j)] = row
print('A letters', A['cap_12_items_1_to_14'], round(time.time() - T0), flush=True)
first = S.search_step(s, S.Limits(cache=CACHE), S.Budget(600))
A['search_step'] = search_summary(first)
assert first['status'] == 'candidate', first.get('status')
g45 = K.protected_lowering(s, U, [[12, 45]], 6)
A['gate_12_45'] = gate_summary(g45)
A['gate_12_45']['output_is_END36'] = g45.get('output_digest') == END36
record['A_z36'] = A
print('A search', A['search_step']['family'], A['search_step']['word'], round(time.time() - T0), flush=True)

# B. The next state, rebuilt as the run loop does after an accepted word (r22d25_search.py:571-575).
end = K.play(s.state, first['word']['word'])
s1 = K.refresh_after_accept(end, first['word']['output_digest'])
started = time.time()
second = S.search_step(s1, S.Limits(cache=CACHE), S.Budget(args.d19_seconds))
record['B_after_f1'] = {'snapshot': snap(s1), 'clock_seconds': args.d19_seconds,
                        'search_step': search_summary(second),
                        'plugin_stop_family': rules_kernel._stop_family(second['ledger']),
                        'elapsed_seconds': round(time.time() - started, 1)}
print('B', second['status'], record['B_after_f1']['plugin_stop_family'], round(time.time() - T0), flush=True)

# C. Fixed tables.
home = state.ctx.data.sig_matrix(np.arange(len(state.pose_id)))
per_cap = np.asarray(home, bool).sum(axis=0)
record['C_tables'] = {'inverse_items': list(S.inverse_items()),
                      'home_chambers_per_cap': {'caps': int(per_cap.size), 'least': int(per_cap.min()),
                                                'largest': int(per_cap.max()), 'chambers': int(home.shape[0])}}
record['seconds'] = round(time.time() - T0, 1)
args.out.write_text(json.dumps(record, indent=1) + '\n')
print('done', record['seconds'], flush=True)
