# The worked examples of chapter 2 of the manual, recomputed from the census records of stage 11.
# Rebuilds each scramble from its seed, replays the recorded moves on the exact simulator, checks N and the off
# count after every step against the record, and prints for the steps the chapter cites: the caps that can turn
# and are misaligned, how many pass the face match, the moved cap's counts (In, Out, A, N_c and the floor
# |In - Out|), N_c after each twist of the step and the best single twist, the number of twists that close the cut,
# the cover relation with the best drop of every cover top, whether the cap was blocked, and whether the step's
# twists of a cap undo the scramble's twists
# of that cap (their product is the identity). The scramble's history is read only for that last label, as the
# census does. One JSON line per cited step; exits 1 if a replayed count differs from the record.
# Run from the repository root (about 2 minutes):
#     python3 -I -B research/jumbling/manual/scripts/examples.py > research/jumbling/manual/scripts/examples.out
import json
import sys
from pathlib import Path

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore  # noqa: E402
import seams  # noqa: E402
from exact import matmul  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

RUNS = REPO / 'research/jumbling/restoration/11-scramble-census/runs/round1'
CITED = {'ia-k10-s7001.json': None, 'ia-k40-s7101.json': [16, 29, 88]}   # None: every step
M = restore.menu('I_a')
bad = 0


def undoes(S, hist, word, c):
    """Whether the step's twists of cap c, after the scramble's twists of c, give the identity on c."""
    g = restore.I4
    for h in hist:
        if h[0] == c:
            g = to_tuple(matmul(S.g(c, h[1]), g))
    for c2, i in word:
        if c2 == c:
            g = to_tuple(matmul(S.g(c, i), g))
    return g == restore.I4


for name, cited in CITED.items():
    r = json.loads((RUNS / name).read_text())
    st, hist = restore.scramble(M, r['k'], r['seed'])
    S = seams.SeamRestorer(st, M)
    pos = 0
    for j, step in enumerate(r['steps']):
        word = [tuple(m) for m in r['moves'][pos:pos + step['twists']]]
        pos += step['twists']
        if cited is None or j in cited:
            sv = S.survey()
            out = {'record': name, 'step': j + 1, 'rule': step['rule'], 'word': word,
                   'misaligned_caps_that_can_turn': len(sv), 'face_match_caps': len(S.face_caps()),
                   'N_before': S.misalignment()}
            caps = sorted({c for c, _ in word})
            out['blocked_before'] = {c: S.scores(c) is None for c in caps}
            out['counts_before'] = {}
            for c in caps:
                if S.scores(c) is not None:
                    In, Out, A = S.balance(c)
                    out['counts_before'][c] = {'In': In, 'Out': Out, 'A': A, 'N_c': In + Out - 2 * A,
                                               'floor': abs(In - Out)}
            if step['rule'] == 'cover':
                cov = S.cover_graph()
                c = word[0][0]
                out['covers'] = sorted(x for x in sv if c in cov[x])
                out['covered_by'] = sorted(cov[c])
                out['cover_top_drops'] = {t: sv[t][0] - min(sv[t][1].values()) for t in sorted(S.cover_tops())}
            trace = []
            for c, i in word:
                now, after = S.scores(c)
                row = {'twist': [c, i], 'N_c': now, 'N_c_after': after[i] if after else now,
                       'best_single_N_c_after': min(after.values()) if after else now}
                if len(word) == 1:
                    row['closing_twists'] = sum(v == 0 for v in after.values())
                trace.append(row)
                S.apply(c, i)
            out['trace'] = trace
            out['scramble_twists_on_caps'] = {c: [[t + 1, h[1]] for t, h in enumerate(hist) if h[0] == c]
                                              for c in caps}
            out['undoes_scramble_twists_of_cap'] = {c: undoes(S, hist, word, c) for c in caps}
        else:
            for c, i in word:
                S.apply(c, i)
        N, off = S.misalignment(), int(st.off_lattice_count())
        if (N, off) != (step['N_after'], step['off_after']):
            bad += 1
            print(json.dumps({'record': name, 'step': j + 1, 'mismatch': [N, off, step['N_after'],
                                                                          step['off_after']]}), flush=True)
        if cited is None or j in cited:
            out['N_after'], out['off_after'] = N, off
            print(json.dumps(out), flush=True)
        if cited is not None and j >= max(cited):
            break
print(json.dumps({'ok': bad == 0}))
sys.exit(1 if bad else 0)
