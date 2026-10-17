# How much a person would have to try to find each step of a census record, under the rules of phase U.
# Replays a census record on the exact simulator (checking N and the off count after every step against the record)
# and prints, for every step, what the state offered before it:
# - every step: the misaligned caps that can turn, how many pass the face match, the search time of the record;
# - seam and cover steps: the chosen cap's rank among the candidates, its N_c, best drop and how many twists reach it;
# - conj steps (c, x)(d, y)(c, z): the rank of c among the misaligned caps in the rule's order (decreasing N_c), the
#   neighbours d of c in that order, the openings (c', x') that the rule's order tries before (c, x), whether d passes
#   the face match once c is opened by x, whether c passes it once y is made, whether c covers d or d covers c before
#   the step, whether z is the inverse of x, and (labels from the scramble's history only) whether x undoes c's last
#   scramble twist and whether y undoes d's last scramble twist.
# Run from the repository root:  python3 -I -B findability.py <record json> [<first step> <last step>]
import json
import sys
from pathlib import Path

import numpy as np

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore  # noqa: E402
import seams  # noqa: E402
from exact import matmul, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

M = restore.menu('I_a')
I4 = restore.I4


def passes_face(S, c):
    r = S.scores(c)
    b = S.balance(c)
    if r is None or not b or r[0] == 0:
        return False
    now, after = r
    return now - min(after.values()) >= 2 * (b[0] - b[2]) > 0


def covers(S, coverer, covered, pos):
    """Whether cap `coverer` covers cap `covered` (seams.cover_graph's test for one pair)."""
    P, A, Sd = S.face(covered)
    if not len(P):
        return False
    m = pos[P] @ seams.NF[coverer] > seams.KAPPA
    i = int((m & (Sd > 0)).sum()); o = int((m & (Sd < 0)).sum())
    ai = int((A & m & (Sd > 0)).sum()); ao = int((A & m & (Sd < 0)).sum())
    return i >= 10 and i == o == ai == ao


def last_scramble_item(hist, c):
    its = [h[1] for h in hist if h[0] == c]
    return its[-1] if its else None


def is_inverse(S, c, i, j):
    return to_tuple(matmul(S.g(c, i), S.g(c, j))) == I4


rec = json.loads(Path(sys.argv[1]).read_text())
lo, hi = (int(sys.argv[2]), int(sys.argv[3])) if len(sys.argv) > 3 else (1, len(rec['steps']))
st, hist = restore.scramble(M, rec['k'], rec['seed'])
assert [list(h) for h in hist] == rec['history'], 'the scramble differs from the record'
S = seams.SeamRestorer(st, M)
ident = next(i for i in range(S.n_items) if S.g(0, i) == I4)
pos_i, prev_sec, bad = 0, 0.0, 0
for j, step in enumerate(rec['steps']):
    word = [tuple(m) for m in rec['moves'][pos_i:pos_i + step['twists']]]
    pos_i += step['twists']
    n = j + 1
    sec = step.get('sec', 0) - prev_sec if step.get('sec', 0) >= prev_sec else step.get('sec', 0)
    prev_sec = step.get('sec', 0)
    if lo <= n <= hi:
        sv = S.survey()
        out = {'step': n, 'rule': step['rule'], 'word': word, 'N_before': step['N_before'],
               'search_sec': sec, 'misaligned_caps': len(sv), 'face_match_caps': len(S.face_caps())}
        if step['rule'] in ('seam', 'cover'):
            rows = S._rows(S.face_caps()) if step['rule'] == 'seam' else S._rows(S.cover_tops())
            c, i = word[0]
            out['candidates'] = len(rows)
            out['rank'] = next(k + 1 for k, r in enumerate(rows) if r[1] == c)
            now, after = sv[c]
            out['N_c'] = now
            out['best_drop'] = now - min(after.values())
            out['best_twists'] = sum(v == min(after.values()) for v in after.values())
        elif step['rule'] == 'conj':
            (c, x), (d, y), (c2, z) = word
            assert c2 == c
            mis = sorted(sv, key=lambda q: (-sv[q][0], q))
            nbs = {q: sorted(({int(t) for t in seams.NEAR[q]} & set(mis)) - {q}) for q in mis}
            r_c = mis.index(c)
            nb = nbs[c]
            per_cap = S.n_items - 1
            x_rank = x - (1 if ident < x else 0)
            pos = seams.positions(S.st)
            out.update({
                'c': c, 'x': x, 'd': d, 'y': y, 'z': z,
                'rank_c': r_c + 1, 'N_c': sv[c][0], 'N_d': sv[d][0],
                'neighbours_of_c': len(nb), 'index_d': nb.index(d) + 1,
                'openings_before': sum(per_cap for q in mis[:r_c] if nbs[q]) + x_rank,
                'x_rank_in_menu': x_rank + 1,
                'N_c_after_x': sv[c][1][x],
                'x_rank_by_N_c': 1 + sum(v < sv[c][1][x] for i2, v in sv[c][1].items() if i2 != ident),
                'c_covers_d': covers(S, c, d, pos), 'd_covers_c': covers(S, d, c, pos),
                'z_inverse_of_x': is_inverse(S, c, x, z),
                'x_undoes_last_scramble_twist_of_c': (last_scramble_item(hist, c) is not None
                                                      and is_inverse(S, c, x, last_scramble_item(hist, c))),
                'y_undoes_last_scramble_twist_of_d': (last_scramble_item(hist, d) is not None
                                                      and is_inverse(S, d, y, last_scramble_item(hist, d))),
            })
            S.apply(c, x)
            nd, ad = S.scores(d)
            out['N_d_after_x'] = nd
            out['d_best_after_x'] = min(ad.values())
            out['d_best_twists_after_x'] = sum(v == min(ad.values()) for v in ad.values())
            out['d_face_match_after_x'] = passes_face(S, d)
            S.apply(d, y)
            nc, ac = S.scores(c)
            out['N_c_after_xy'] = nc
            out['c_best_after_xy'] = min(ac.values())
            out['c_face_match_after_xy'] = passes_face(S, c)
            S.undo(); S.undo()
        print(json.dumps(out), flush=True)
    for c, i in word:
        S.apply(c, i)
    N, off = S.misalignment(), int(S.st.off_lattice_count())
    if (N, off) != (step['N_after'], step['off_after']):
        print(json.dumps({'step': n, 'mismatch': [N, off, step['N_after'], step['off_after']]}), flush=True)
        bad = 1
        break
print(json.dumps({'ok': not bad}))
sys.exit(bad)
