# How dense are three-twist repairs (c, x)(d, y)(c, z) at a conj step of a census record?
# For the cap c the record chose, tries every opening x (the 59 non-identity items) and every misaligned neighbour d,
# with y = d's best twist after x and z = c's best twist after x and y (the rule of seams.SeamRestorer.conj), and
# prints one row per (x, d): N_c after x, whether the repair lowers N, and by how much. The record's own step is then
# made and checked, so later steps start from the record's states.
# Run from the repository root:  python3 -I -B density.py <record json> <step> [<step> ...]
import json
import sys
from pathlib import Path

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore  # noqa: E402
import seams  # noqa: E402

M = restore.menu('I_a')
I4 = restore.I4


def repair(S, c, x, d, now_c, after_c):
    """Delta N of (c, x)(d, y)(c, z) with the rule's y and z, or None when the rule would not form it."""
    S.apply(c, x)
    try:
        r = S.scores(d)
        if r is None or r[0] == 0:
            return None, 'd_blocked_or_closed'
        n_d, a_d = r
        y = min(a_d, key=lambda k: (a_d[k], k))
        if a_d[y] >= n_d:
            return None, 'd_no_drop'
        S.apply(d, y)
        try:
            rc = S.scores(c)
            if rc is None or not rc[1]:
                return None, 'c_blocked_or_closed'
            n_c2, a_c2 = rc
            z = min(a_c2, key=lambda k: (a_c2[k], k))
            delta = (after_c[x] - now_c) + (a_d[y] - n_d) + (a_c2[z] - n_c2)
            return delta, (y, z)
        finally:
            S.undo()
    finally:
        S.undo()


rec = json.loads(Path(sys.argv[1]).read_text())
want = {int(a) for a in sys.argv[2:]}
st, hist = restore.scramble(M, rec['k'], rec['seed'])
assert [list(h) for h in hist] == rec['history'], 'the scramble differs from the record'
S = seams.SeamRestorer(st, M)
pos = 0
for j, step in enumerate(rec['steps']):
    word = [tuple(m) for m in rec['moves'][pos:pos + step['twists']]]
    pos += step['twists']
    n = j + 1
    if n in want and step['rule'] == 'conj':
        (c, x0), (d0, _), _ = word
        sv = S.survey()
        mis = set(sv)
        nb = sorted(({int(t) for t in seams.NEAR[c]} & mis) - {c})
        now_c, after_c = sv[c]
        for x in range(S.n_items):
            if S.g(c, x) == I4:
                continue
            for d in nb:
                delta, info = repair(S, c, x, d, now_c, after_c)
                print(json.dumps({'step': n, 'c': c, 'x': x, 'd': d, 'chosen': [x, d] == [x0, d0],
                                  'N_c': now_c, 'N_c_after_x': after_c[x], 'N_d': sv[d][0],
                                  'delta': delta, 'info': info}), flush=True)
    for cc, i in word:
        S.apply(cc, i)
    if (S.misalignment(), int(S.st.off_lattice_count())) != (step['N_after'], step['off_after']):
        print(json.dumps({'step': n, 'mismatch': True}), flush=True)
        sys.exit(1)
    if n >= max(want):
        break
print(json.dumps({'done': True}))
