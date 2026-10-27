"""Stage 18: the gate of replay_tail.py. Each mutation of a recorded tail must be rejected (exit 1): an empty unit, a
unit with a lattice piece in place of one of its pieces, a changed unit size, X (positive or negative), letters or V
in the record, a negative piece in the unit, a preparation file removed, two adjacent letters of V' swapped (the hash
catches it even when they commute: the same action, length and end), the other step form in the record, a changed end
digest, and the wrong start table. With --full, the unmutated record must also replay (exit 0). Each case runs on its
own copy of the steps directory under the new scratch directory; the record itself is only read. A piece that is in no
step's unit is on the lattice at the start, since the recorded steps land every off piece. Research only, headless;
exact geometry.
Usage (from the repository root): OPENBLAS_NUM_THREADS=1 python3 -I -B <this script> <start pose table> <steps dir>
<new scratch dir> [--full]
"""
import contextlib
import gzip
import io
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import replay_tail as R  # noqa: E402


def record(d):
    return json.loads((d / 'steps.json').read_text())


def edit_record(d, k, **changes):
    plan = record(d)
    plan['steps'][k].update(changes)
    (d / 'steps.json').write_text(json.dumps(plan) + '\n')


def edit_form(d, form):
    plan = record(d)
    plan['form'] = form
    (d / 'steps.json').write_text(json.dumps(plan) + '\n')


def edit_unit(d, k, change):
    f = d / record(d)['steps'][k]['dir'] / 'unit.json'
    f.write_text(json.dumps({'pieces': change(json.loads(f.read_text())['pieces'])}) + '\n')


def swap_v(d, k, i):
    f = d / record(d)['steps'][k]['dir'] / 'v.json.gz'
    doc = R.read(f)
    w = doc['word']
    w[i], w[i + 1] = w[i + 1], w[i]
    with gzip.open(f, 'wt', encoding='utf-8') as stream:
        json.dump(doc, stream)


def run(*argv):
    with contextlib.redirect_stdout(io.StringIO()) as buf:
        try:
            rc = R.main([str(a) for a in argv])
        except Exception as error:  # a crash is not a rejection
            rc = f'error: {type(error).__name__}'
    lines = buf.getvalue().strip().splitlines()
    return rc, (json.loads(lines[-1]) if lines else None)


def main(argv):
    full = '--full' in argv
    args = [a for a in argv if a != '--full']
    if len(args) != 3:
        sys.exit(__doc__)
    table, steps, scratch = Path(args[0]), Path(args[1]), Path(args[2])
    scratch.mkdir(parents=True, exist_ok=False)
    plan = record(steps)
    prep = next(k for k, r in enumerate(plan['steps']) if r['X'] > 0)
    landed = set()
    for r in plan['steps']:
        landed |= set(json.loads((steps / r['dir'] / 'unit.json').read_text())['pieces'])
    lattice = next(p for p in range(len(landed) + 1) if p not in landed)
    other = 'four-segment' if plan.get('form') == 'three-segment' else 'three-segment'
    cases = [
        ('empty unit', lambda d: edit_unit(d, 0, lambda ps: [])),
        ('a lattice piece in the unit', lambda d: edit_unit(d, 0, lambda ps: ps[1:] + [lattice])),
        ('unit size in the record', lambda d: edit_record(d, 0, unit_size=plan['steps'][0]['unit_size'] + 1)),
        ('X in the record', lambda d: edit_record(d, 0, X=5)),
        ('X negative in the record', lambda d: edit_record(d, 0, X=-1)),
        ('letters in the record', lambda d: edit_record(d, 0, letters=plan['steps'][0]['letters'] + 1)),
        ('a negative piece in the unit', lambda d: edit_unit(d, 0, lambda ps: ps[1:] + [-1])),
        ('V in the record', lambda d: edit_record(d, prep, V=plan['steps'][prep]['V'] + 1)),
        ('preparation file removed', lambda d: (d / plan['steps'][prep]['dir'] / 'x.json.gz').unlink()),
        ("two adjacent letters of V' swapped", lambda d: swap_v(d, 0, 2)),
        ('the other form in the record', lambda d: edit_form(d, other)),
        ('end digest in the record', lambda d: edit_record(d, 0, end_digest='0' * 64)),
    ]
    results = []
    for n, (name, mutate) in enumerate(cases):
        d = scratch / f'case-{n}'
        shutil.copytree(steps, d)
        mutate(d)
        rc, last = run(table, d, scratch / f'out-{n}')
        results.append(rc == 1)
        print(json.dumps({'case': name, 'exit': rc, 'rejected': rc == 1, 'last': last}), flush=True)
    last_case = len(cases) - 1
    wrong = scratch / f'out-{last_case}' / f'{plan["steps"][0]["dir"]}.json'  # step 1's end, played by the last case
    rc, last = run(wrong, steps, scratch / 'out-start') if wrong.exists() else ('no table', None)
    results.append(rc == 1)
    print(json.dumps({'case': 'wrong start table', 'exit': rc, 'rejected': rc == 1, 'last': last}), flush=True)
    if full:
        rc, last = run(table, steps, scratch / 'out-full')
        results.append(rc == 0)
        print(json.dumps({'case': 'unmutated record', 'exit': rc, 'accepted': rc == 0, 'last': last}), flush=True)
    print(json.dumps({'done': True, 'passed': all(results), 'cases': len(results)}))
    return 0 if all(results) else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
