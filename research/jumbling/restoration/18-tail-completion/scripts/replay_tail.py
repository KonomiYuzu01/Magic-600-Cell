"""Stage 18 (tail completion): replay a tail of literal whole-cap steps from its stored words, step by step.

The steps directory holds steps.json ({"start_digest": ..., "steps": [{"dir", "start_digest", "end_digest",
"off_before", "off_after", "unit_size", "X", "V", "letters", "x_sha256" (prepared steps only), "w_sha256", "v_sha256",
"winv_sha256"}, ...]}) and one directory per step with w.json (W = (grip, item), one letter), winv.json (W^-1 as a
core), v.json.gz (the literal SAFE-ALL word V' at the middle), x.json.gz (the SAFE-ALL preparation X; present exactly
when the record's X is positive) and unit.json (the off pieces the step lands). The start table must have the recorded
start digest, which is also the first step's. Before a step is played, its directory must be a plain name, the
record's counts must be integers in range and consistent (X >= 0, V >= 1, the unit size >= 1, letters = X + 2 + 2V,
the off count dropping by the unit size), every word file must have the recorded SHA-256 (of the JSON text of its
word, or of the whole core for W^-1) and length, and the unit must be nonempty nonnegative integers without repeats,
of the recorded size; an unreadable record or file rejects the step. Each step X.W.V'.W^-1.V'^-1 is then played twist
by twist by
slp_apply.py --unit from the previous end table, which checks the strict' contract; the replay also requires the
recorded digests and off counts, every other off piece at its pose, the off count to drop by exactly the unit's size
(so every unit piece was off and lands), and the length and hash of every segment as slp_apply reads it. It stops at
the first difference. One JSON line per step, then {"done": ...}. Exit 0 only when every step passes. Research only,
headless; exact geometry.
Usage (from the repository root): OPENBLAS_NUM_THREADS=1 python3 -I -B <this script> <start pose table> <steps dir>
<new out dir>
"""
import gzip
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

SLP = Path('research/jumbling/restoration/12-low-height/scripts/slp_apply.py')
ENV = dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1')


def read(path):
    opener = gzip.open if str(path).endswith('.gz') else open
    with opener(path, 'rt', encoding='utf-8') as stream:
        return json.load(stream)


def sha(obj):
    return hashlib.sha256(json.dumps(obj).encode()).hexdigest()


def precheck(steps_dir, rec):
    """The stored files of one step against its record, before the step is played; returns a reason or None."""
    try:
        if type(rec['dir']) is not str or rec['dir'] in ('', '.', '..') or Path(rec['dir']).name != rec['dir']:
            return 'a step directory that is not a plain name'
        d = steps_dir / rec['dir']
        X, V, size = rec['X'], rec['V'], rec['unit_size']
        if not all(type(v) is int for v in (X, V, size, rec['letters'], rec['off_before'], rec['off_after'])):
            return 'a count in the record is not an integer'
        prepared = (d / 'x.json.gz').exists()
        if X < 0 or V < 1 or size < 1 or prepared != (X > 0) or prepared != ('x_sha256' in rec):
            return 'X, V or the unit size out of range, or a preparation without its recorded X and hash'
        if rec['letters'] != X + 2 + 2 * V or rec['off_before'] - rec['off_after'] != size:
            return 'the recorded letters or off counts do not match X, V and the unit size'
        words = {'w': read(d / 'w.json')['word'], 'v': read(d / 'v.json.gz')['word']}
        if prepared:
            words['x'] = read(d / 'x.json.gz')['word']
        for key, word in words.items():
            if sha(word) != rec[f'{key}_sha256']:
                return f'the word {key} differs from the record'
        if sha(read(d / 'winv.json')) != rec['winv_sha256']:
            return 'the core W^-1 differs from the record'
        if len(words['w']) != 1 or len(words['v']) != V or (prepared and len(words['x']) != X):
            return 'a word length differs from the record'
        pieces = read(d / 'unit.json')['pieces']
        if (not pieces or not all(type(p) is int and p >= 0 for p in pieces) or len(set(pieces)) != len(pieces)
                or len(pieces) != size):
            return 'the unit is empty, names a piece that is not a nonnegative integer, repeats a piece, or differs ' \
                   'in size from the record'
    except (KeyError, TypeError, ValueError, OSError, EOFError) as error:
        return f'an unreadable record or step file ({type(error).__name__})'
    return None


def main(argv):
    if len(argv) != 3:
        sys.exit(__doc__)
    table, steps_dir, out = Path(argv[0]), Path(argv[1]), Path(argv[2])
    out.mkdir(parents=True, exist_ok=False)
    plan = json.loads((steps_dir / 'steps.json').read_text())
    if (not plan['steps'] or plan['steps'][0]['start_digest'] != plan['start_digest']
            or json.loads(table.read_text())['digest'] != plan['start_digest']):
        print(json.dumps({'done': False, 'reason': 'no steps, or the start table is not the recorded start'}))
        return 1
    for k, rec in enumerate(plan['steps']):
        reason = precheck(steps_dir, rec)
        if reason:
            print(json.dumps({'done': False, 'failed_step': k + 1, 'reason': reason}))
            return 1
        d = (steps_dir / rec['dir']).resolve()
        segs = [{'word_file': str(d / 'x.json.gz')}] if rec['X'] > 0 else []
        segs += [{'word_file': str(d / 'w.json')}, {'safe_word_file': str(d / 'v.json.gz')},
                 {'core_file': str(d / 'winv.json')}, {'safe_word_file': str(d / 'v.json.gz'), 'inverse': True}]
        want = ([rec['X']] if rec['X'] > 0 else []) + [1, rec['V'], 1, rec['V']]
        prog = out / f'{rec["dir"]}-program.json'
        prog.write_text(json.dumps({'segments': segs}) + '\n')
        end = out / f'{rec["dir"]}.json'
        p = subprocess.run([sys.executable, '-I', '-B', str(SLP), 'table', str(table), str(prog), str(end), '--unit',
                            str(d / 'unit.json')], capture_output=True, text=True, env=ENV)
        rows = [json.loads(line) for line in p.stdout.splitlines() if line.startswith('{')]
        s = rows[-1] if rows else {}
        segrows = [r for r in rows if 'segment' in r]
        lengths = [int(r['length']) for r in segrows]
        got = dict(step=k + 1, dir=rec['dir'], start_digest=s.get('start_digest'), end_digest=s.get('end_digest'),
                   off_before=s.get('off_before'), off_after=s.get('off_after'), unit_size=s.get('unit_size'),
                   letters=sum(lengths), strict_prime=s.get('strict_prime'))
        ok = (p.returncode == 0 and s.get('strict_prime') is True and lengths == want
              and segrows[-3].get('word_sha256') == rec['v_sha256']
              and s.get('unit_on_lattice') == rec['unit_size'] and s.get('collateral') == 0
              and s.get('others_exact') == s.get('others')
              and all(got[key] == rec[key] for key in ('start_digest', 'end_digest', 'off_before', 'off_after',
                                                       'unit_size', 'letters'))
              and s['off_before'] - s['off_after'] == rec['unit_size'])
        print(json.dumps(dict(got, ok=ok)), flush=True)
        if not ok:
            print(json.dumps({'done': False, 'failed_step': k + 1}))
            return 1
        table = end
    print(json.dumps({'done': True, 'steps': len(plan['steps']), 'end_digest': plan['steps'][-1]['end_digest'],
                      'off': plan['steps'][-1]['off_after']}))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
