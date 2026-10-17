# At every conj step of a census record: did a single twist that lowers N exist? The improve rule (U5) makes such a
# twist, but it comes after conj in the rule order, so in the census it never ran. Prints, per conj step, how many
# misaligned caps have a lowering single twist, the largest single-twist drop, the record's conj drop, and how many
# caps are above their floor |In_c - Out_c| with no lowering single twist (the floor is a bound, not a target). The
# record's steps are made on the exact simulator and N and the off count are checked after every step.
# Run from the repository root:  python3 -I -B single_twist.py <record json>
import json
import sys
from pathlib import Path

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures')]
import restore  # noqa: E402
import seams  # noqa: E402

M = restore.menu('I_a')
rec = json.loads(Path(sys.argv[1]).read_text())
st, hist = restore.scramble(M, rec['k'], rec['seed'])
assert [list(h) for h in hist] == rec['history'], 'the scramble differs from the record'
S = seams.SeamRestorer(st, M)
pos = 0
for j, step in enumerate(rec['steps']):
    word = [tuple(m) for m in rec['moves'][pos:pos + step['twists']]]
    pos += step['twists']
    if step['rule'] == 'conj':
        sv = S.survey()
        drops = sorted((now - min(after.values()) for now, after in sv.values() if min(after.values()) < now),
                       reverse=True)
        floor = {c: abs(S.balance(c)[0] - S.balance(c)[1]) for c in sv}
        stuck = [c for c, (now, after) in sv.items() if now > floor[c] and min(after.values()) >= now]
        # the cap furthest above its floor among them: [cap, N_c, floor]
        ex = min(stuck, key=lambda c: (floor[c] - sv[c][0], c), default=None)
        print(json.dumps({'step': j + 1, 'misaligned': len(sv), 'caps_with_single_drop': len(drops),
                          'best_single_drop': drops[0] if drops else 0, 'top_drops': drops[:5],
                          'conj_drop': step['N_before'] - step['N_after'],
                          'above_floor_without_drop': len(stuck),
                          'furthest_above_floor': None if ex is None else [ex, sv[ex][0], floor[ex]]}), flush=True)
    for c, i in word:
        S.apply(c, i)
    if (S.misalignment(), int(S.st.off_lattice_count())) != (step['N_after'], step['off_after']):
        print(json.dumps({'step': j + 1, 'mismatch': True}), flush=True)
        sys.exit(1)
print(json.dumps({'done': True}))
