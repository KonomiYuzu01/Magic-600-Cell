"""Check Lemma M on a restoration, exactly: what a restoration keeps.

The script samples two depth-one states: the start state and the state after the given word files. It checks
Xi_end <= Xi_start (every off piece of the end is off at the start with the same exact pose), and then the
statements of Lemma M (M1) on this instance:

- every SAFE-ALL letter of the start, a (grip, menu_index) pair, is one at the end, with the same element of K and
  the same permutation of its cap;
- every orbit of G(Xi_end) is a union of orbits of G(Xi_start);
- Fix(G(Xi_end)) is contained in Fix(G(Xi_start)).

Usage (from the repository root)::

    python research/jumbling/theory/monocheck.py --table <pose table> --words <word files...> --out <file>

Without ``--table`` the start is the E2 end of I_a@40 (``orient.start``). Each word file holds {'word',
'input_digest', 'output_digest'}; every file is read once, before the start state is built. The words are replayed
on the sampled start state itself (sampling applies no twist), every twist checked admissible, and each word's
digests are checked at both ends, so the end state is the one the words reach from the sampled start. The exit
status is 0 only when every check holds. Evidence: exact simulator replay and exact computation, not native host
evidence.
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import buffer as B  # noqa: E402

T0 = time.time()


def sample(R, M):
    G = B.Alphabet(R, M)
    orbit, size = G.orbits()
    K = R.st._kpose()[R.st.pose_id]
    off = np.flatnonzero(K < 0)
    return {'digest': R.st.digest(), 'letters': {(c, i): (k, ys) for c, i, k, ys in G.letters},
            'grips': len(G.safe), 'orbit': np.asarray(orbit).copy(), 'size': np.asarray(size).copy(),
            'off': {int(q): B.matrix_json(R.st._pose_matrix(int(R.st.pose_id[q]))) for q in off.tolist()}}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--table', help='current-only pose table of the start state (default: E2 end)')
    ap.add_argument('--words', nargs='+', required=True, help='word files from the start to the end state')
    ap.add_argument('--out', required=True)
    a = ap.parse_args(argv)
    docs = [(name, json.loads(Path(name).read_text())) for name in a.words]
    R, M = B.O.start(a.table, [])
    s = sample(R, M)
    for name, doc in docs:
        if R.st.digest() != doc['input_digest']:
            raise ValueError(f'{name}: the state before the word is not its input state')
        for c, i in doc['word']:
            R.apply(int(c), int(i))
        if R.st.digest() != doc['output_digest']:
            raise ValueError(f'{name}: the word does not end at its output state')
    e = sample(R, M)
    restriction = set(e['off']) <= set(s['off']) and all(e['off'][q] == s['off'][q] for q in e['off'])
    lost = [list(x) for x in sorted(set(s['letters']) - set(e['letters']))]
    changed = [list(x) for x in sorted(set(s['letters']) & set(e['letters']))
               if s['letters'][x][0] != e['letters'][x][0]
               or not np.array_equal(s['letters'][x][1], e['letters'][x][1])]
    image = {}
    split = 0
    for xs, xe in zip(s['orbit'].tolist(), e['orbit'].tolist()):
        if image.setdefault(xs, xe) != xe:
            split += 1
    fix_s = set(np.flatnonzero(s['size'][s['orbit']] == 1).tolist())
    fix_e = set(np.flatnonzero(e['size'][e['orbit']] == 1).tolist())
    checks = {'restriction': restriction, 'letters_kept': not lost and not changed, 'orbits_coarsen': split == 0,
              'fixed_shrink': fix_e <= fix_s}
    report = {'start_digest': s['digest'], 'end_digest': e['digest'], 'words': [Path(w).name for w in a.words],
              'off': [len(s['off']), len(e['off'])], 'safe_all_grips': [s['grips'], e['grips']],
              'safe_all_letters': [len(s['letters']), len(e['letters'])], 'letters_lost': lost,
              'letters_changed': changed, 'orbits': [len(s['size']), len(e['size'])],
              'fixed_chambers': [len(fix_s), len(fix_e)], 'checks': checks, 'ok': all(checks.values()),
              'evidence': 'exact simulator replay and exact computation', 'seconds': round(time.time() - T0)}
    Path(a.out).write_text(json.dumps(report, indent=1) + '\n')
    print(json.dumps({k: report[k] for k in ('off', 'safe_all_grips', 'orbits', 'fixed_chambers', 'checks', 'ok')}))
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    sys.exit(main())
