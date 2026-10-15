"""Exhaustive fallback of the general method M* (SENIOR-REVIEW.md section 9; M1-01). From a pose table, search the words
over the I_a menu in order of length and take the first clean step for the unit U, with free set off(Z) \\ U.

Order: lengths 1, 2, ...; within a length, lexicographic over the letters (c, i), cap c = 0, ..., 599 and menu item
i = 1, ..., 59 (item 0 is the identity). Two reductions keep the search complete:
- an identity letter changes nothing, so it is left out;
- two consecutive letters at one cap act as one letter there: the I_a menu is a group of order 60 (checked exactly at
  the start), and the first letter keeps every piece on its side of the cut, so (c, g1)(c, g2) = (c, g2 g1) with the
  same admissibility. Every word has an equivalent word with neither, no longer and with the same end state.
A word is rejected at its first inadmissible letter (the simulator's exact certificate; admissibility depends on the
cap only, so a blocked cap is skipped with all its items) and accepted when the off pieces of the end state lie inside
off(Z) \\ U. The search stops: the inverse of any history from the solved state is a clean step (M1-01), so some
length has an accepted word. There is no bound on that length; the search (mstar.word_search) keeps its own stack,
so only memory limits it. Length 1 has at most 35,400 letters; at the state of test_fallback.py a letter took about
0.02 s, so length 1 takes minutes and length 2 (about 1.25 x 10^9 words) about 300 days. This is the completeness
device of M*, not a practical search.

usage: fallback.py <theory dir> <E-chain scripts dir> <pose table> <unit json {pieces}> <out dir> [--max-length N]
Writes, as tclose2.py does: word-full.json (word, input and output digests), pre.json (state digest, the off pieces,
the target, no protected piece), result.json (result, length, letters tried, end) and end-table.json. --max-length is
an experiment bound outside M*: without a word up to that length the result is 'no word up to length N' (exit 3).
Research only: headless Linux, exact synthetic geometry; no Windows, DirectX, input or performance claim.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import mstar  # noqa: E402

TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import buffer as B  # noqa: E402
import ghostlib as GL  # noqa: E402
import sim  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def off_set(st):
    return np.flatnonzero(st._kpose()[st.pose_id] < 0)


def main():
    a = sys.argv[1:]
    table, ufile, out = a[2], a[3], Path(a[4])
    maxlen = int(a[a.index('--max-length') + 1]) if '--max-length' in a else 0
    out.mkdir(parents=True, exist_ok=True)
    R, M = GL.O.start(table)
    st, restore = R.st, GL.restore
    items = [to_tuple(m) for _, m in M.items]
    n, d = len(items), len(items[0])
    assert n == 60 and all(items[0][r][c] == (1 if r == c else 0) for r in range(d) for c in range(d)), \
        'menu item 0 must be the identity of a menu of 60 items'
    group = set(items)
    assert len(group) == n and all(to_tuple(restore.matmul(x, y)) in group for x in items for y in items), \
        'the menu is not a group'
    U = sorted(int(p) for p in json.loads(Path(ufile).read_text())['pieces'])
    off0 = off_set(st)
    assert np.isin(U, off0).all(), 'the unit must be off the lattice'
    allowed = set(off0.tolist()) - set(U)
    d_s = st.digest()
    pre = {'state_digest': d_s, 'off': int(len(off0)), 'blocks': [off0.tolist()], 'target_block': U,
           'protected_off': [], 'note': 'fallback: every off piece outside the target is free'}
    (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
    st.journal = []
    el = {}

    def letter(c, i):
        if (c, i) not in el:
            el[(c, i)] = sim.Twist(restore.ctx, c, restore.element(M, c, i), 'menu', {'menu': M.name})
        return el[(c, i)]

    def apply(c, i):
        o = st.apply(letter(c, i))
        if not o.applied:
            # without a budget a menu letter is only ever blocked, and a blocked cut depends on the cap only
            assert o.status == 'blocked', f'letter {(c, i)}: status {o.status}'
        return o.applied

    word, tried = mstar.word_search(apply, st.undo, lambda: set(off_set(st).tolist()) <= allowed, 600, n, maxlen,
                                    lambda length, tried, found: log(length=length, tried=tried, found=found))
    if word is None:
        res = {'result': f'no word up to length {maxlen}', 'tried': tried, 'seconds': round(time.time() - T0)}
        (out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
        print(json.dumps(res))
        return 3
    off1 = off_set(st)
    end = {'digest': st.digest(), 'off': int(len(off1)), 'T_restored': int(np.isin(U, off1, invert=True).sum()),
           'collateral': int(len(set(off1.tolist()) - set(off0.tolist()))), 'protected_exact_pose_changed': 0}
    assert end['T_restored'] == len(U) and end['collateral'] == 0
    w = [list(x) for x in word]
    (out / 'word-full.json').write_text(json.dumps({'word': w, 'input_digest': d_s,
                                                    'output_digest': end['digest']}) + '\n')
    res = {'result': 'found', 'length': len(w), 'tried': tried, 'input_digest': d_s, 'end': end,
           'seconds': round(time.time() - T0)}
    (out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
    B.export_pose_table(st, str(out / 'end-table.json'))
    print(json.dumps({k: v for k, v in res.items() if k != 'end'} | {'off_after': end['off']}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
