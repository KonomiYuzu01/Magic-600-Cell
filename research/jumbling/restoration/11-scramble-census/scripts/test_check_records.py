# Mutation test of check_records.py: copies of the depth 10 record, each with one field changed (a move, an N after
# a step, the start counts and the twist count, the round boundary, the result, the solved flag, the start heights,
# the off poses), must each fail the replay check with exit status 1, and the record itself must pass with 0.
# Writes the copies to a temporary directory only. Run from the repository root (about two minutes):
#   python3 -I -B test_check_records.py
import json, subprocess, sys, tempfile
from pathlib import Path
S = Path('research/jumbling/restoration/11-scramble-census')
rec = json.loads((S / 'runs/round1/ia-k10-s7001.json').read_text())


def move3(x):
    c, i = x['moves'][3]
    x['moves'][3] = [c, (i + 1) % 59]


MUTANTS = {
    'move': move3,
    'N_after': lambda x: x['steps'][2].update(N_after=x['steps'][2]['N_after'] + 2),
    'start counts and twists': lambda x: x.update(N_start=-1, off_start=-1, twists=999),
    'round boundary': lambda x: x.update(continued_from_moves=999, N_start=1, off_start=1, heights_start={'2': 1}),
    'result': lambda x: x.update(result='time-limit'),
    'solved': lambda x: x.update(solved=False),
    'start heights': lambda x: x.update(heights_start={'2': x['off_start'] - 1, '4': 1}),
    'off poses': lambda x: x.update(off_poses_start=x['off_poses_start'] + 1),
}
ok = True
with tempfile.TemporaryDirectory() as tmp:
    cases = [('record itself', None)] + list(MUTANTS.items())
    for name, mutate in cases:
        x = json.loads(json.dumps(rec))
        if mutate:
            mutate(x)
        f = Path(tmp) / 'case.json'
        f.write_text(json.dumps(x))
        p = subprocess.run([sys.executable, '-I', '-B', str(S / 'scripts/check_records.py'), str(f)],
                           capture_output=True, text=True)
        want = 0 if mutate is None else 1
        line = json.loads(p.stdout.splitlines()[-1]) if p.stdout.strip() else {}
        good = p.returncode == want
        ok &= good
        print(json.dumps({'case': name, 'exit': p.returncode, 'expected': want, 'ok': good,
                          'fails': line.get('fails', [p.stderr[-300:]])[:2]}), flush=True)
print(json.dumps({'all_ok': ok}))
sys.exit(0 if ok else 1)
