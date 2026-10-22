"""Replay a runner-run/3 step and export a full-state tutorial picture.

    python3 -I -B export_tutorial.py RUN --step 0 --when before --out DIRECTORY

Inputs are read-only. All replay and integrity checks finish before output starts.
Evidence: exact simulator replay and float drawing data, not native rendering.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SCRIPTS = HERE.parent / 'restoration/13-tutorial-runner/scripts'
sys.path[:0] = [str(SCRIPTS), str(HERE.parent / 'fixtures')]

import numpy as np  # noqa: E402
import check_run  # noqa: E402
import wj  # noqa: E402

FORMAT = 'magic600-tutorial-shot/1'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def check_boundary(state, reference, where):
    if state.digest() != reference.get('digest'):
        raise ValueError(f'{where}: replayed digest mismatch')
    if state.off_lattice_count() != reference.get('off'):
        raise ValueError(f'{where}: off-lattice count mismatch')


def copy_state(state):
    other = check_run.sim.State(state.ctx, menu=state.menu)
    other.pose_id = state.pose_id.copy()
    other._poses, other._lookup = list(state._poses), dict(state._lookup)
    return other


def moved_ids(before, after):
    """Compare exact poses, since compaction can change otherwise unchanged pose ids."""
    keys = {p.key: i for i, p in enumerate(before._poses)}
    remap = np.array([keys.get(p.key, -1) for p in after._poses], dtype=np.int32)
    return np.flatnonzero(before.pose_id != remap[after.pose_id]).astype('<i4')


def read_word(base, row):
    failures = []
    word = check_run.load_word_v3(base, row, failures)
    if word is None or failures:
        raise ValueError(f'step {row["index"]}: invalid word ({", ".join(failures)})')
    if len(word['word']) != row.get('letters'):
        raise ValueError(f'step {row["index"]}: word length mismatch')
    for side, field in (('before', 'input_digest'), ('after', 'output_digest')):
        if word[field] != row[side]['digest']:
            raise ValueError(f'step {row["index"]}: word {field} mismatch')
    return word['word']


def play(state, row, word, base):
    if row['phase'] == 'lattice' and row['encoding'] == 'ia16':
        # The existing checker plays EVERY letter in chronological order. Its retained-
        # lattice representation rebuilds the exact simulator state at the boundary.
        failures = check_run.check_lattice.replay_lattice_step(
            state, row, check_run.inside(base, row['word']), state.menu)
        if failures:
            raise ValueError(f'step {row["index"]}: lattice replay failed ({", ".join(failures)})')
        return
    for cap, item in word:
        twist = check_run.sim.Twist(state.ctx, cap, check_run.restore.element(state.menu, cap, item),
                                    'menu', {'menu': state.menu.name})
        outcome = state.apply(twist, journal=False)
        if not outcome.applied or outcome.uncertain:
            raise ValueError(f'step {row["index"]}: inadmissible or uncertain twist ({cap}, {item})')


def checkpoint(base, record, target, menu):
    """Only use a named table at or BEFORE the target's before boundary."""
    candidates = []
    for i, row in enumerate(record['steps'][:target]):
        if row.get('table'):
            candidates.append((i + 1, row['table'], row['after']))
    bulk = record.get('phases', {}).get('bulk', {})
    end = bulk.get('end') or {}
    n = bulk.get('steps')
    if type(n) is int and 0 < n <= target and end.get('table'):
        candidates.append((n, end['table'], end))
    for n, name, reference in sorted(candidates, key=lambda c: c[0], reverse=True):
        path = check_run.inside(base, name)
        if not path.is_file():
            continue
        state = check_run.load_table(path, menu)
        check_boundary(state, reference, f'checkpoint {name}')
        check_boundary(state, record['steps'][n - 1]['after'], f'checkpoint after step {n - 1}')
        return state, n, {'file': name, 'sha256': sha(path.read_bytes()), 'after_step': n - 1}
    return None, 0, None


def export(run, step, when, out):
    base, out = Path(run).resolve(), Path(out).resolve()
    if out.is_relative_to(base):
        raise ValueError('--out must not be inside the record directory')
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        raise ValueError('--out exists and is not empty')
    if type(step) is not int or step < 0 or when not in ('before', 'after'):
        raise ValueError('invalid step or when')
    raw = (base / 'run.json').read_bytes()
    record = json.loads(raw, object_pairs_hook=check_run.no_duplicates)
    if record.get('format') != 'runner-run/3' or record.get('menu') != 'I_a':
        raise ValueError('expected a runner-run/3 record with menu I_a')
    start = record['start']
    if start.get('kind') not in ('scramble', 'table') or ('seed' in start and type(start['seed']) is not int):
        raise ValueError('invalid start kind or seed')
    if start['kind'] == 'scramble' and (type(start.get('seed')) is not int
                                       or type(start.get('k')) is not int or start['k'] < 0):
        raise ValueError('invalid scramble start')
    if type(start.get('seed')) is int and 9011 <= start['seed'] <= 9073:
        raise ValueError('start seeds 9011..9073 are reserved and refused')
    steps = record['steps']
    if not isinstance(steps, list) or step >= len(steps):
        raise ValueError('step is outside the record')
    for i, row in enumerate(steps[:step + 1]):
        if row.get('index') != i or row.get('encoding') not in ('json', 'ia16'):
            raise ValueError(f'step {i}: invalid index or encoding')
        expected = start['digest'] if i == 0 else steps[i - 1]['after']['digest']
        if row['before']['digest'] != expected:
            raise ValueError(f'step {i}: broken digest chain')

    menu = check_run.restore.menu('I_a')
    state = check_run.load_table(base / 'start-table.json', menu)
    check_boundary(state, start, 'start')
    start_hash = sha((base / 'start-table.json').read_bytes())
    later, first, source = checkpoint(base, record, step, menu)
    if later is not None:
        state = later
    used = []
    before = after = letters = None
    for i in range(first, step + 1):
        row = steps[i]
        check_boundary(state, row['before'], f'step {i} before')
        word = read_word(base, row)
        word_path = check_run.inside(base, row['word'])
        used.append({'file': row['word'], 'sha256': sha(word_path.read_bytes())})
        if i == step:
            before, letters = copy_state(state), word
        play(state, row, word, base)
        check_boundary(state, row['after'], f'step {i} after')
    after = state
    moving = moved_ids(before, after)
    state = before if when == 'before' else after
    row = steps[step]
    arrays = wj.arrays(state)
    header = {
        'format': wj.RENDER_FORMAT, 'dimension': wj.DIMENSION, 'pieces': len(state.pose_id),
        'poses': len(state._poses), 'revision': sum(s['letters'] for s in steps[:step + (when == 'after')]),
        'digest': state.digest(), 'model_identity': state.ctx.data.identity, 'menu_identity': menu.identity,
        'contract_revision': check_run.sim.model.CONTRACT_REVISION,
        'convention': "x' = M x; M row-major float32; pose 0 is the identity",
        'files': {name: {'sha256': wj.sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str}
                  for name, a in arrays.items()},
    }
    tutorial = {
        'format': FORMAT, 'record': Path(os.path.relpath(base / 'run.json', ROOT)).as_posix(),
        'run_sha256': sha(raw), 'start_table_sha256': start_hash, 'words': used,
        'step': step, 'when': when, 'phase': row['phase'], 'subphase': row['subphase'], 'rule': row['rule'],
        'letter_count': len(letters), 'caps': list(dict.fromkeys(c for c, _ in letters)),
        'off_lattice': state.off_lattice_count(), 'digest': header['digest'],
        'moved': {'file': 'moved.i32', 'count': len(moving), 'sha256': wj.sha(moving)},
    }
    if len(letters) <= 64:
        tutorial['letters'] = letters
    else:
        tutorial['letters_sha256'] = sha(json.dumps(letters).encode('utf-8'))
    if source is not None:
        tutorial['checkpoint'] = source
    payloads = {name: np.ascontiguousarray(array).tobytes() for name, array in arrays.items()}
    payloads['moved.i32'] = moving.tobytes()
    for name, doc in (('header.json', header), ('tutorial.json', tutorial)):
        payloads[name] = (json.dumps(doc, indent=1, allow_nan=False) + '\n').encode('utf-8')
    # No output directory or file exists until the complete requested step passed,
    # including its after boundary when exporting a before picture's moved set.
    out.mkdir(parents=True, exist_ok=True)
    for name, payload in payloads.items():
        (out / name).write_bytes(payload)
    return header, tutorial


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', type=Path)
    parser.add_argument('--step', type=int, required=True)
    parser.add_argument('--when', choices=('before', 'after'), required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        header, tutorial = export(args.run, args.step, args.when, args.out)
    except (OSError, ValueError, KeyError, TypeError, AssertionError, EOFError) as exc:
        print(f'tutorial export: FAIL ({exc})', file=sys.stderr)
        return 1
    print(f'step {args.step} {args.when}: {header["digest"]}, '
          f'{tutorial["off_lattice"]} off lattice, {tutorial["moved"]["count"]} moved')
    return 0


if __name__ == '__main__':
    sys.exit(main())
