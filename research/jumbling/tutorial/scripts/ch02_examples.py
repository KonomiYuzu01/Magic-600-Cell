"""Chapter 2 of the jumbling tutorial: the bulk examples it quotes (headless research).

Read-only. It imports the simulator, the theory modules and the runner's constants, plays the bulk rules on two
development seeds exactly as the runner's bulk child does, and writes one JSON record.

Run from the repository root (a few minutes on a cloud machine):
    python3 -I -B research/jumbling/tutorial/scripts/ch02_examples.py --out <new file> --cache <new dir>

Sections of the record:
  A bulk      seeds 7001 (depth 10) and 7106 (depth 20): the bulk as the runner's bulk child plays it (one
              SeamRestorer in the runner's BULK_ORDER, at each step the first rule that returns a word), with every
              step's rule, word, N, off count and digest, compared with the committed D0 records of stage 13
              (runs/dev/d0-s<seed>/run/run.json and its word files)
  B s7106i17  seed 7106 before and after its step at index 17 (the 18th step): the survey with N_c and best
              after-value, the balance (In, Out, A) and floor of each surveyed cap, the face caps, the best and
              closing items of cap 592, after_592(9), the inside pieces of caps 592 and 523 and how many are off
  C cache     after every bulk step of both seeds: every per-cap value the playing restorer keeps in its caches
              (scores, inside set, balance) against a fresh restorer at the same state
"""
import argparse
import json
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--cache', required=True, type=Path)
parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[4])
args = parser.parse_args()
ROOT = args.root.resolve()
J = ROOT / 'research/jumbling'
D0 = J / 'restoration/13-tutorial-runner/runs/dev'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures'),
                str(J / 'restoration/13-tutorial-runner/scripts')]
import numpy as np  # noqa: E402
import restore  # noqa: E402
import seams  # noqa: E402
import runner  # noqa: E402  (only BULK_ORDER and SOLVED are read)

args.cache.mkdir(parents=True, exist_ok=True)
seams.CACHE = args.cache / 'home_centroids.npy'   # never the default cache under the repository
T0 = time.time()
M = restore.menu('I_a')
ORDER = tuple(runner.BULK_ORDER)
out = {'format': 'tutorial-ch02-examples/1', 'bulk_order': list(ORDER), 'solved_digest': runner.SOLVED}


def bulk_methods(R):
    """The rule names of the runner's bulk child and the methods they call (runner.py:1536-1538)."""
    return {'seam': R.seam, 'cover': R.cover, 'close': R.level1, 'lock': R.level2, 'improve': R.improve,
            'shift': R.shift, 'conj': R.conj}


def best(after):
    return min(after.values())


def survey_rows(R):
    """The survey in cap order: N_c, the best after-value and the drop."""
    return {str(c): {'N_c': int(now), 'best': int(best(after)), 'drop': int(now - best(after))}
            for c, (now, after) in R.survey().items()}


def same_inside(a, b):
    if a is None or b is None:
        return a is None and b is None
    return np.array_equal(np.sort(np.asarray(a)), np.sort(np.asarray(b)))


def cache_check(R, st):
    """Every value R keeps in its per-cap caches against a fresh restorer at the same state."""
    F = seams.SeamRestorer(st, M, order=ORDER)
    counts = {'scores': 0, 'inside': 0, 'balance': 0}
    bad = []
    for c, v in R._cls.items():
        counts['inside'] += 1
        if not same_inside(v, F.inside(c)):
            bad.append(['inside', int(c)])
    for c, v in R._sv.items():
        counts['scores'] += 1
        if v != F.scores(c):
            bad.append(['scores', int(c)])
    for c, v in R._bal.items():
        counts['balance'] += 1
        if v != F.balance(c):
            bad.append(['balance', int(c)])
    return counts, bad


def detail(R, st, caps):
    """The facts section B quotes, read with a restorer that is not the playing one."""
    sv = R.survey()
    rows = survey_rows(R)
    for c in sv:
        In, Out, A = R.balance(c)
        rows[str(c)].update(In=int(In), Out=int(Out), A=int(A), floor=int(abs(In - Out)),
                            face_test_bound=int(2 * (In - A)))
    res = {'N': int(R.misalignment()), 'off': int(st.off_lattice_count()), 'digest': st.digest(),
           'survey': rows, 'face_caps': [int(c) for c in R.face_caps()]}
    off = np.array([p.kidx < 0 for p in st._poses], bool)[st.pose_id]
    for c in caps:
        ins = R.inside(c)
        sc = R.scores(c)
        res['cap_%d' % c] = {
            'scores_none': sc is None,
            'N_c': None if sc is None else int(sc[0]),
            'inside_pieces': None if ins is None else int(len(ins)),
            'inside_off': None if ins is None else int(off[np.asarray(ins)].sum()),
        }
    return res


for seed, k in ((7001, 10), (7106, 20)):
    rec = json.loads((D0 / ('d0-s%d' % seed) / 'run/run.json').read_text())
    d0 = [s for s in rec['steps'] if s['phase'] == 'bulk']
    d0_words = [json.loads((D0 / ('d0-s%d' % seed) / 'run' / s['word']).read_text())['word'] for s in d0]
    st, _ = restore.scramble(M, k, seed)
    R = seams.SeamRestorer(st, M, order=ORDER)
    assert list(R.order) == list(ORDER) and 'reopen' not in R.order
    methods = bulk_methods(R)
    start = {'digest': st.digest(), 'off': int(st.off_lattice_count()), 'N': int(R.misalignment())}
    steps, checks, result = [], [], None
    while True:
        n = int(R.misalignment())
        if n == 0:
            result = 'lattice'
            break
        index = len(steps)
        if seed == 7106 and index == 17:
            out['B_s7106_index17'] = {'before': detail(seams.SeamRestorer(st, M, order=ORDER), st, (523, 550, 592))}
            F = seams.SeamRestorer(st, M, order=ORDER)
            now, after = F.scores(592)
            out['B_s7106_index17']['cap_592_items'] = {
                'best_items': sorted(int(i) for i, v in after.items() if v == best(after)),
                'closing_items': sorted(int(i) for i, v in after.items() if v == 0),
                'after_item_9': int(after[9]), 'after_item_0': int(after[0])}
        first = len(R.moves)
        for rule in ORDER:
            if methods[rule]() is not None:
                break
        else:
            result = 'stuck'
            break
        word = [list(map(int, x)) for x in R.moves[first:]]
        after = {'digest': st.digest(), 'off': int(st.off_lattice_count()), 'N': int(R.misalignment())}
        steps.append({'rule': rule, 'word': word, 'N_before': n, 'N_after': after['N'], 'off_after': after['off'],
                      'digest_after': after['digest']})
        if seed == 7106 and index == 17:
            out['B_s7106_index17']['after'] = detail(seams.SeamRestorer(st, M, order=ORDER), st, (523, 550, 592))
        counts, bad = cache_check(R, st)
        checks.append({'index': index, 'cap': word[-1][0], 'compared': counts, 'mismatches': bad})
        print(seed, index, rule, word, n, '->', after['N'], 'off', after['off'], counts, len(bad),
              round(time.time() - T0), flush=True)
    same = (len(steps) == len(d0)
            and all(s['rule'] == r['rule'] and s['word'] == w and s['N_before'] == r['before']['N']
                    and s['N_after'] == r['after']['N'] and s['off_after'] == r['after']['off']
                    and s['digest_after'] == r['after']['digest'] for s, r, w in zip(steps, d0, d0_words)))
    out['A_bulk_s%d' % seed] = {
        'k': k, 'seed': seed, 'start': start, 'start_same_as_d0': start['digest'] == rec['start']['digest'],
        'result': result, 'end_digest': st.digest(), 'end_is_solved_digest': st.digest() == runner.SOLVED,
        'steps': steps, 'same_as_d0_record': bool(same),
        'd0_record': {'file': 'research/jumbling/restoration/13-tutorial-runner/runs/dev/d0-s%d/run/run.json' % seed,
                      'format': rec['format'], 'mode': rec['config']['mode'],
                      'source_commit': rec['config']['source']['commit'],
                      'bulk_result': rec['phases']['bulk']['result']},
    }
    out['C_cache_s%d' % seed] = {
        'states_checked': len(checks),
        'values_compared': {key: sum(ch['compared'][key] for ch in checks) for key in ('scores', 'inside', 'balance')},
        'mismatches': sum(len(ch['mismatches']) for ch in checks),
        'per_step': checks,
    }
out['seconds'] = round(time.time() - T0, 1)
args.out.write_text(json.dumps(out, indent=1) + '\n')
print('done', out['seconds'], flush=True)
