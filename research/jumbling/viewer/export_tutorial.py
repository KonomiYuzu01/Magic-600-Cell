"""Replay a runner-run/3 step or a checked state recipe for a tutorial picture.

    python3 -I -B export_tutorial.py RUN --step 0 --when before --out DIRECTORY
    python3 -I -B export_tutorial.py --state RECIPE.json --out DIRECTORY

Inputs are read-only. All replay and integrity checks finish before output starts.
Evidence: exact simulator replay and float drawing data, not native rendering.
"""
from __future__ import annotations

import argparse
import ast
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
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
CORRECTION_LIBRARY = 'research/jumbling/restoration/09-general-method/data/library-merged.json.gz'


class ReplaySession:
    """An invocation-local cache of verified boundaries, never an input artifact."""
    def __init__(self):
        self.records = {}
        self.words = {}
        self.starts = {}


class CommittedInputs:
    """Invocation-local Git inventory, checked before opening picture inputs."""
    def __init__(self, root=None):
        self.root = Path(ROOT if root is None else root).resolve()
        self.tracked = None
        self.changed = set()

    def require(self, path):
        path = Path(path).resolve()
        if not path.is_relative_to(self.root):
            raise ValueError('picture input must be inside the repository')
        name = path.relative_to(self.root).as_posix()
        for seed in re.findall(r'(?:s|seed-?)(\d+)', name):
            refuse_seed(int(seed))
        if self.tracked is None:
            command = ['git', '--no-optional-locks', '-C', str(self.root)]
            try:
                tracked = subprocess.run([*command, 'ls-files', '-z'], check=True, capture_output=True).stdout
                status = subprocess.run([*command, 'status', '--porcelain=v1', '-z',
                                         '--untracked-files=no'], check=True, capture_output=True).stdout
            except (OSError, subprocess.CalledProcessError) as exc:
                raise ValueError('cannot verify committed picture inputs with Git') from exc
            self.tracked = {os.fsdecode(p) for p in tracked.split(b'\0') if p}
            rows = iter(status.split(b'\0'))
            for row in rows:
                if not row:
                    continue
                self.changed.add(os.fsdecode(row[3:]))
                if b'R' in row[:2] or b'C' in row[:2]:
                    self.changed.add(os.fsdecode(next(rows)))
        if name not in self.tracked:
            raise ValueError(f'{name}: picture input must be tracked and committed in Git')
        if name in self.changed:
            raise ValueError(f'{name}: picture input must be committed and unmodified')
        return path


def repository_path(name, root=None):
    root = ROOT if root is None else Path(root).resolve()
    try:
        return check_run.inside(root, name)
    except ValueError as exc:
        raise ValueError('expected a repository-relative path inside the repository') from exc


def refuse_seed(seed):
    if type(seed) is int and 9011 <= seed <= 9073:
        raise ValueError('start seeds 9011..9073 are reserved and refused')


def validate_recipe(recipe, root=None, committed=None):
    """Validate the complete recipe and its paths before any mechanics or writes."""
    if (not isinstance(recipe, dict) or not {'start', 'digest'} <= set(recipe)
            or not set(recipe) <= {'start', 'play', 'emph', 'digest'}):
        raise ValueError('invalid state recipe fields')
    if not isinstance(recipe['digest'], str) or not re.fullmatch('[0-9a-f]{64}', recipe['digest']):
        raise ValueError('state digest must be a full lowercase sha256')
    start = recipe['start']
    if not isinstance(start, dict):
        raise ValueError('invalid state start')
    if set(start) == {'table'}:
        repository_path(start['table'], root)
    elif set(start) == {'record', 'step', 'when'}:
        base = repository_path(start['record'], root)
        repository_path(start['record'] + '/run.json', root)
        if type(start['step']) is not int or start['step'] < 0 or start['when'] not in ('before', 'after'):
            raise ValueError('invalid state start step or when')
        committed = CommittedInputs(root) if committed is None else committed
        run = check_run.read_json(committed.require(base / 'run.json'))
        refuse_seed(run['start'].get('seed'))
        repository_path(start['record'] + '/start-table.json', root)
        for row in run.get('steps', [])[:start['step'] + 1]:
            for name in (row.get('word'), row.get('table'), (row.get('tree') or {}).get('file')):
                if name is not None:
                    repository_path(start['record'] + '/' + name, root)
        table = (run.get('phases', {}).get('bulk', {}).get('end') or {}).get('table')
        if table is not None:
            repository_path(start['record'] + '/' + table, root)
    elif set(start) == {'scramble'}:
        pair = start['scramble']
        if (not isinstance(pair, list) or len(pair) != 2 or any(type(n) is not int for n in pair)
                or pair[0] < 0):
            raise ValueError('invalid state scramble [k, seed]')
        refuse_seed(pair[1])
    else:
        raise ValueError('invalid state start fields')
    if not isinstance(recipe.get('play', []), list):
        raise ValueError('invalid state play list')
    for item in recipe.get('play', []):
        if (not isinstance(item, dict) or 'word' not in item
                or not set(item) <= {'word', 'letters'}):
            raise ValueError('invalid state play fields')
        repository_path(item['word'], root)
        if 'letters' in item:
            span = item['letters']
            if (not isinstance(span, list) or len(span) != 2 or any(type(n) is not int for n in span)
                    or not 0 <= span[0] <= span[1]):
                raise ValueError('invalid state letter range [from, to)')
    emph = recipe.get('emph', {'moved': 'last'})
    if not isinstance(emph, dict) or len(emph) != 1:
        raise ValueError('invalid state emphasis')
    if set(emph) == {'pieces'}:
        if (not isinstance(emph['pieces'], list)
                or any(type(p) is not int or p < 0 for p in emph['pieces'])):
            raise ValueError('invalid state emphasis pieces')
    elif not (emph == {'moved': 'last'} or
              (set(emph) <= {'off', 'pose_differs_from_solved'} and next(iter(emph.values())) is True)):
        raise ValueError('invalid state emphasis')


def inverse_word(word):
    menu = check_run.restore.menu('I_a')
    R = check_run.restore
    lookup = {R.to_tuple(m): i for i, (_, m) in enumerate(menu.items)}
    inverses = [lookup[R.to_tuple(R.transpose(m))] for _, m in menu.items]
    return [[c, inverses[j]] for c, j in reversed(word)]


def correction_words(library):
    """Translate the library's exact matrix records to the same I_a letters."""
    R = check_run.restore
    menu = R.menu('I_a')
    lookup = {R.to_tuple(m): i for i, (_, m) in enumerate(menu.items)}
    memo, words = {}, []
    for macro in library['macros']:
        word = []
        for record in macro['records']:
            twist = check_run.sim.Twist.from_record(R.ctx, record)
            cap, matrix = twist.grip, R.to_tuple(twist.matrix)
            if type(cap) is not int or not 0 <= cap < 600:
                raise ValueError('invalid correction cap')
            if (cap, matrix) not in memo:
                frame = menu.frame(cap)
                base = R.to_tuple(R.matmul(R.matmul(R.transpose(frame), twist.matrix), frame))
                if base not in lookup:
                    raise ValueError('correction matrix is outside I_a')
                memo[cap, matrix] = lookup[base]
            word.append([cap, memo[cap, matrix]])
        if len(word) != macro['length']:
            raise ValueError('correction macro length mismatch')
        words.append(word)
    return words


def recipe_word(name, inputs, root=None, active=(), committed=None):
    """Read literal research words, including gzip and the stored segment programs.

    A census tape is history followed by moves. A replay log's LETTER rows are
    already chronological. Neither format invokes a search or executes source code.
    """
    path = repository_path(name, root)
    if path in active:
        raise ValueError('cyclic word program')
    committed = CommittedInputs(root) if committed is None else committed
    committed.require(path)
    raw = path.read_bytes()
    inputs[path] = sha(raw)
    if path.name.endswith(('.ia16', '.ia16.gz')):
        info = check_run.check_lattice.ia16_info(path)
        word = check_run.check_lattice.read_ia16(path, info['sha256'])
    elif path.suffix == '.py':
        # These fixtures commit their literal word in a script rather than a JSON
        # file. Read a single named constant; never import or execute that script.
        try:
            assignments = [node.value for node in ast.parse(raw).body if isinstance(node, ast.Assign)
                           and any(isinstance(t, ast.Name) and t.id in ('word', 'WORD', 'WORD36')
                                   for t in node.targets)]
            if len(assignments) != 1:
                raise ValueError('expected one literal word/WORD/WORD36 assignment')
            word = ast.literal_eval(assignments[0])
        except (SyntaxError, TypeError, ValueError, RecursionError) as exc:
            raise ValueError('expected one literal word/WORD/WORD36 assignment') from exc
    elif path.suffix == '.txt':
        rows = [json.loads(line[7:], object_pairs_hook=check_run.no_duplicates)
                for line in raw.decode('utf-8').splitlines() if line.startswith('LETTER ')]
        if not rows or [r['n'] for r in rows] != list(range(1, len(rows) + 1)):
            raise ValueError('expected a chronological LETTER replay tape')
        word = [r['letter'] for r in rows]
    else:
        doc = check_run.read_json(path)
        if isinstance(doc, dict):
            refuse_seed(doc.get('seed'))
        if isinstance(doc, list):
            word = doc
        elif 'word' in doc:
            word = doc['word']
        elif {'history', 'moves'} <= set(doc):
            if doc.get('menu') != 'I_a':
                raise ValueError('census word must use menu I_a')
            word = doc['history'] + doc['moves']
        elif 'letters' in doc and isinstance(doc['letters'], list):
            word = [row['letter'] for row in doc['letters']]
        elif doc.get('format') == 'I_a-lattice-correction-library-v1' and doc.get('menu') == 'I_a':
            word = [letter for macro in correction_words(doc) for letter in macro]
        elif {'runs', 'library', 'cascade_order'} <= set(doc):
            # The committed lattice summary specifies T = solved . C^-1. Its
            # literal target-construction word is exactly the player's target=True
            # branch, without importing or executing the restoration program.
            if len(doc['runs']) != 1 or doc['runs'][0].get('order') != 'retained-first':
                raise ValueError('expected one retained-first correction target')
            path = repository_path(CORRECTION_LIBRARY, root)
            committed.require(path)
            raw_library = path.read_bytes()
            inputs[path] = sha(raw_library)
            decoded = gzip.decompress(raw_library)
            if doc['library']['sha256'] not in (sha(raw_library), sha(decoded)):
                raise ValueError('correction library sha256 mismatch')
            library = json.loads(decoded, object_pairs_hook=check_run.no_duplicates)
            macros, correction = correction_words(library), []
            for row in doc['runs'][0]['corrections']:
                if (type(row['macro']) is not int or not 0 <= row['macro'] < len(macros)
                        or type(row['times']) is not int or row['times'] < 1
                        or type(row['inverse']) is not bool):
                    raise ValueError('invalid correction target sequence')
                part = macros[row['macro']]
                correction.extend((inverse_word(part) if row['inverse'] else part) * row['times'])
            if len(correction) != doc['runs'][0]['correction_twists']:
                raise ValueError('correction target length mismatch')
            word = inverse_word(correction)
        elif str(doc.get('format', '')).startswith('tutorial-ch'):
            # Chapter records preserve short literal words alongside their evidence.
            # Concatenate only explicit word arrays, in document order; ranges pick
            # a particular recorded word without inventing or running a search.
            word = []

            def collect(value):
                if isinstance(value, dict):
                    for key, child in value.items():
                        if key == 'word' and isinstance(child, list):
                            word.extend(child)
                        else:
                            collect(child)
                elif isinstance(value, list):
                    for child in value:
                        collect(child)

            collect(doc)
            if not word:
                raise ValueError(f'{name}: no literal word')
        elif set(doc) == {'P', 'Q'}:
            # slp_apply.apply_segment's core is the conjugate P Q P^-1.
            word = doc['P'] + doc['Q'] + inverse_word(doc['P'])
        elif set(doc) == {'segments'} and isinstance(doc['segments'], list):
            word = []
            for segment in doc['segments']:
                keys = set(segment) - {'inverse'}
                if (len(keys) != 1 or not keys <= {'word_file', 'safe_word_file', 'core_file'}
                        or ('inverse' in segment and type(segment['inverse']) is not bool)):
                    raise ValueError('invalid literal word segment')
                part = recipe_word(segment[next(iter(keys))], inputs, root, (*active, path), committed)
                word.extend(inverse_word(part) if segment.get('inverse') else part)
        else:
            raise ValueError(f'{name}: no literal word')
    if (not isinstance(word, list) or any(not isinstance(letter, (list, tuple)) or len(letter) != 2
            or any(type(n) is not int for n in letter) or not 0 <= letter[0] < 600
            or not 0 <= letter[1] < 60 for letter in word)):
        raise ValueError(f'{name}: invalid I_a word')
    return word


def prepare_recipe(recipe, root=None, committed=None):
    committed = CommittedInputs(root) if committed is None else committed
    validate_recipe(recipe, root, committed)
    inputs, words = {}, []
    for item in recipe.get('play', []):
        word = recipe_word(item['word'], inputs, root, committed=committed)
        first, end = item.get('letters', [0, len(word)])
        if end > len(word):
            raise ValueError('state letter range is outside the word')
        words.append((word, first, end))
    return inputs, words


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


def read_word(base, row, committed=None):
    committed = CommittedInputs() if committed is None else committed
    committed.require(check_run.inside(base, row['word']))
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


def play(state, row, word, base, committed=None):
    if row['phase'] == 'lattice' and row['encoding'] == 'ia16':
        committed = CommittedInputs() if committed is None else committed
        committed.require(check_run.inside(base, row['word']))
        committed.require(check_run.inside(base, row['tree']['file']))
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


def checkpoint(base, record, target, menu, committed=None):
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
        committed = CommittedInputs() if committed is None else committed
        committed.require(path)
        state = check_run.load_table(path, menu)
        check_boundary(state, reference, f'checkpoint {name}')
        check_boundary(state, record['steps'][n - 1]['after'], f'checkpoint after step {n - 1}')
        return state, n, {'file': name, 'sha256': sha(path.read_bytes()), 'after_step': n - 1}
    return None, 0, None


def replay_record(run, step, when, session=None, committed=None):
    base = Path(run).resolve()
    if type(step) is not int or step < 0 or when not in ('before', 'after'):
        raise ValueError('invalid step or when')
    committed = CommittedInputs() if committed is None else committed
    raw = committed.require(base / 'run.json').read_bytes()
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

    committed.require(base / 'start-table.json')
    menu = check_run.restore.menu('I_a')
    state = check_run.load_table(base / 'start-table.json', menu)
    check_boundary(state, start, 'start')
    start_hash = sha((base / 'start-table.json').read_bytes())
    later, first, source = checkpoint(base, record, step, menu, committed)
    if later is not None:
        state = later
    used = []
    inputs = {base / 'run.json': sha(raw), base / 'start-table.json': start_hash}
    if source is not None:
        inputs[check_run.inside(base, source['file'])] = source['sha256']
    if session is not None:
        # A cached boundary is usable only while every file that established it
        # still has the same bytes. Never skip replay of the requested step itself.
        for n in range(step, first - 1, -1):
            cached = session.records.get((base, n))
            if cached and all(p.is_file() and sha(committed.require(p).read_bytes()) == h
                              for p, h in cached[3].items()):
                state, used, source, inputs = copy_state(cached[0]), list(cached[1]), cached[2], dict(cached[3])
                first = n
                break
    before = after = letters = None
    for i in range(first, step + 1):
        row = steps[i]
        check_boundary(state, row['before'], f'step {i} before')
        word = read_word(base, row, committed)
        word_path = check_run.inside(base, row['word'])
        used.append({'file': row['word'], 'sha256': sha(word_path.read_bytes())})
        inputs[word_path] = used[-1]['sha256']
        if row['encoding'] == 'ia16':
            path = check_run.inside(base, row['tree']['file'])
            committed.require(path)
            inputs[path] = sha(path.read_bytes())
        if i == step:
            before, letters = copy_state(state), word
        play(state, row, word, base, committed)
        check_boundary(state, row['after'], f'step {i} after')
        if session is not None:
            session.records[base, i + 1] = (copy_state(state), list(used), source, dict(inputs))
    after = state
    moving = moved_ids(before, after)
    state = before if when == 'before' else after
    row = steps[step]
    tutorial = {
        'format': FORMAT, 'record': Path(os.path.relpath(base / 'run.json', ROOT)).as_posix(),
        'run_sha256': sha(raw), 'start_table_sha256': start_hash, 'words': used,
        'step': step, 'when': when, 'phase': row['phase'], 'subphase': row['subphase'], 'rule': row['rule'],
        'letter_count': len(letters), 'caps': list(dict.fromkeys(c for c, _ in letters)),
        'off_lattice': state.off_lattice_count(), 'digest': state.digest(),
        'moved': {'file': 'moved.i32', 'count': len(moving), 'sha256': wj.sha(moving)},
    }
    if len(letters) <= 64:
        tutorial['letters'] = letters
    else:
        tutorial['letters_sha256'] = sha(json.dumps(letters).encode('utf-8'))
    if source is not None:
        tutorial['checkpoint'] = source
    revision = sum(s['letters'] for s in steps[:step + (when == 'after')])
    return state, moving, revision, tutorial


def write_render(state, moving, revision, tutorial, out):
    out = Path(out).resolve()
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        raise ValueError('--out exists and is not empty')
    arrays = wj.arrays(state)
    header = {
        'format': wj.RENDER_FORMAT, 'dimension': wj.DIMENSION, 'pieces': len(state.pose_id),
        'poses': len(state._poses), 'revision': revision,
        'digest': state.digest(), 'model_identity': state.ctx.data.identity, 'menu_identity': state.menu.identity,
        'contract_revision': check_run.sim.model.CONTRACT_REVISION,
        'convention': "x' = M x; M row-major float32; pose 0 is the identity",
        'files': {name: {'sha256': wj.sha(a), 'shape': list(a.shape), 'dtype': a.dtype.str}
                  for name, a in arrays.items()},
    }
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


def export(run, step, when, out):
    base, out = Path(run).resolve(), Path(out).resolve()
    if out.is_relative_to(base):
        raise ValueError('--out must not be inside the record directory')
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        raise ValueError('--out exists and is not empty')
    state, moving, revision, tutorial = replay_record(base, step, when)
    return write_render(state, moving, revision, tutorial, out)


def recipe_state(recipe, session=None, committed=None):
    committed = CommittedInputs() if committed is None else committed
    inputs, words = prepare_recipe(recipe, committed=committed)
    session = ReplaySession() if session is None else session
    if 'table' in recipe['start']:
        committed.require(repository_path(recipe['start']['table']))
    menu = check_run.restore.menu('I_a')
    start, revision, caps = recipe['start'], 0, []
    if 'table' in start:
        path = repository_path(start['table'])
        inputs[path] = sha(path.read_bytes())
        key = ('table', inputs[path])
        if key not in session.starts:
            session.starts[key] = check_run.load_table(path, menu)
        state = copy_state(session.starts[key])
    elif 'scramble' in start:
        key = ('scramble', *start['scramble'])
        if key not in session.starts:
            session.starts[key], _ = check_run.restore.scramble(menu, *start['scramble'])
        state = copy_state(session.starts[key])
        revision = start['scramble'][0]
    else:
        base = repository_path(start['record'])
        state, _, revision, original = replay_record(base, start['step'], start['when'], session, committed)
        caps = original['caps']
        # replay_record retains exactly the inputs used to establish this boundary,
        # including the after boundary required for a before picture.
        inputs.update(session.records[base, start['step'] + 1][3])
        if any(not p.is_relative_to(ROOT) for p in inputs):
            raise ValueError('record input is outside the repository')
    moving = np.empty(0, dtype='<i4')
    played = 0
    for word, first, end in words:
        before = copy_state(state)
        # Each input was re-read above. Identical incoming exact state and literal
        # letters establish the same boundary, including when metadata changed.
        key = (state.digest(), sha(json.dumps(word, separators=(',', ':')).encode()), first)
        boundaries = session.words.setdefault(key, {})
        skip = max((n for n in boundaries if first <= n <= end), default=first)
        if skip in boundaries:
            state = copy_state(boundaries[skip])
        play(state, {'phase': 'recipe', 'encoding': 'json', 'index': 'recipe'}, word[skip:end], ROOT)
        boundaries[end] = copy_state(state)
        moving = moved_ids(before, state)
        caps = list(dict.fromkeys(c for c, _ in word[first:end]))
        played += end - first
    if state.digest() != recipe['digest']:
        raise ValueError(f'state digest mismatch: expected {recipe["digest"]}, got {state.digest()}')
    emph = recipe.get('emph', {'moved': 'last'})
    if 'pieces' in emph:
        if any(p >= len(state.pose_id) for p in emph['pieces']):
            raise ValueError('state emphasis piece is outside the model')
        moving = np.array(sorted(set(emph['pieces'])), dtype='<i4')
    elif 'off' in emph:
        moving = np.flatnonzero(np.array([p.kidx < 0 for p in state._poses])[state.pose_id]).astype('<i4')
    elif 'pose_differs_from_solved' in emph:
        moving = np.flatnonzero(np.array([p.key != ('K', 0) for p in state._poses])[state.pose_id]).astype('<i4')
    tutorial = {
        'format': FORMAT, 'state': recipe,
        'inputs': [{'file': p.relative_to(ROOT).as_posix(), 'sha256': h} for p, h in sorted(inputs.items())],
        'letter_count': played, 'caps': caps, 'off_lattice': state.off_lattice_count(), 'digest': state.digest(),
        'moved': {'file': 'moved.i32', 'count': len(moving), 'sha256': wj.sha(moving)},
    }
    return state, moving, revision + played, tutorial


def export_recipe(recipe, out, session=None, committed=None):
    out = Path(out).resolve()
    committed = CommittedInputs() if committed is None else committed
    validate_recipe(recipe, committed=committed)
    start = recipe['start']
    if 'record' in start and out.is_relative_to(repository_path(start['record'])):
        raise ValueError('--out must not be inside the record directory')
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        raise ValueError('--out exists and is not empty')
    state, moving, revision, tutorial = recipe_state(recipe, session, committed)
    return write_render(state, moving, revision, tutorial, out)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', type=Path, nargs='?')
    parser.add_argument('--state', type=Path, help='JSON state recipe, instead of RUN/--step/--when')
    parser.add_argument('--step', type=int)
    parser.add_argument('--when', choices=('before', 'after'))
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.state is not None:
            if args.run is not None or args.step is not None or args.when is not None:
                raise ValueError('--state cannot be combined with RUN/--step/--when')
            committed = CommittedInputs()
            recipe = check_run.read_json(committed.require(args.state))
            header, tutorial = export_recipe(recipe, args.out, committed=committed)
        else:
            if args.run is None or args.step is None or args.when is None:
                raise ValueError('give --state or RUN/--step/--when')
            header, tutorial = export(args.run, args.step, args.when, args.out)
    except (OSError, ValueError, KeyError, TypeError, AssertionError, EOFError) as exc:
        print(f'tutorial export: FAIL ({exc})', file=sys.stderr)
        return 1
    print(f'{"state" if args.state else f"step {args.step} {args.when}"}: {header["digest"]}, '
          f'{tutorial["off_lattice"]} off lattice, {tutorial["moved"]["count"]} moved')
    return 0


if __name__ == '__main__':
    sys.exit(main())
