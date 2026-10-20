"""Play lattice-restoration-v1 words literally, using exact integer data only.

Run from the repository root with python3 -I -B; all output belongs outside the
repository. No restorer or checker algorithm is imported. The existing pose
table importer is the only shared loading seam.

Restored means identity sticker labels (owner decision, 15 October 2026).
Labels do not determine exact poses: retained play also transports K+ poses so
that final.digest, including at a budget stop, is a real simulator pose digest.
At a retained-first junction, the packet permits switching to the independently
built representative T only after comparing all 259,800 labels.
"""

import argparse
import functools
import gzip
import hashlib
import itertools
import json
import math
import os
from pathlib import Path, PurePosixPath
import sys
import tempfile
import time

sys.dont_write_bytecode = True
for _variable in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_variable] = '1'

import numpy as np  # noqa: E402

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
J = ROOT / 'research/jumbling'
DEFAULT_LIBRARY = J / 'restoration/09-general-method/data/library-merged.json.gz'
NS = 259800
MODEL_FILES = ('assets/model.npz', 'assets/primitives.npz',
               'assets/seed_atlas.json', 'assets/execution_trees.json.gz')


class Failed(Exception):
    """A named check, deliberately without paths or raw exception text."""


class Budget(Exception):
    def __init__(self, position):
        self.position = position


def require(condition, check):
    if not condition:
        raise Failed(check)


def read_json(path):
    raw = Path(path).read_bytes()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def labels_sha(labels):
    return hashlib.sha256(labels.astype('<i8', copy=False).tobytes()).hexdigest()


def checked_library(path, expected):
    raw = Path(path).read_bytes()
    decoded = gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw
    # The current restorer accepts plain JSON. Its compressed archival copy
    # still authenticates against that exact decoded byte stream; future
    # restorers may instead record the compressed file's raw hash.
    require(expected in (hashlib.sha256(raw).hexdigest(), hashlib.sha256(decoded).hexdigest()),
            'library_sha256')
    return json.loads(decoded)


def atomic_json(path, doc):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    name = None
    try:
        with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=path.parent,
                                         prefix='.' + path.name + '-', delete=False) as stream:
            name = stream.name
            json.dump(doc, stream, indent=1, allow_nan=False)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
    finally:
        if name is not None and os.path.exists(name):
            os.unlink(name)


def solution_factors(solution, spec):
    """The restorer's tag; committed records write the run directory as <run> in both input and key."""
    tags = [spec.replace(':', '_').replace('/', '_')[-60:]]
    if '<run>' in spec:
        tags.append(('<run>' + spec.split('<run>', 1)[1].replace(':', '_').replace('/', '_'))[-60:])
    for tag in tags:
        if tag in solution['solutions']:
            return solution['solutions'][tag]
    raise Failed('solution_tag')


def integer(value, minimum=0):
    return type(value) is int and value >= minimum


class Program:
    """Validate a topologically ordered DAG; expand it with bounded stack space.

    Only leaf words and execution-tree paths are stored. Composite words, even
    when their expanded lengths exceed 10**8, are never materialized.
    """

    def __init__(self, nodes, trees):
        require(isinstance(nodes, list), 'nodes')
        self.nodes, self.words, self.lengths = nodes, {}, []
        for i, nd in enumerate(nodes):
            require(isinstance(nd, dict), 'node')
            op = nd.get('op')
            if op in ('word', 'path'):
                if op == 'word':
                    word = nd['w']
                    require(isinstance(word, list), 'word')
                else:
                    orbit, node = nd['orbit'], nd['node']
                    require(integer(orbit) and orbit < len(trees), 'path_orbit')
                    tree = trees[orbit]
                    require(integer(node) and node < len(tree['parent']), 'path_node')
                    word, seen = [], set()
                    while tree['parent'][node] >= 0:
                        require(node not in seen, 'path_cycle')
                        seen.add(node)
                        word.append(tree['move'][node])
                        node = tree['parent'][node]
                        require(integer(node) and node < len(tree['parent']), 'path_parent')
                    word.reverse()
                require(all(type(m) is int and 1 <= abs(m) <= 1200 for m in word), 'move')
                self.words[i] = tuple(word)
                length = len(word)
            elif op == 'conj':
                x, u, sign = nd['x'], nd['u'], nd['us']
                self.factor([x, 1], i)
                self.factor([u, sign], i)
                length = self.lengths[x] + 2 * self.lengths[u]
            elif op == 'prod':
                require(isinstance(nd['f'], list), 'product')
                for factor in nd['f']:
                    self.factor(factor, i)
                length = sum(self.lengths[j] for j, _ in nd['f'])
            else:
                raise Failed('node_op')
            self.lengths.append(length)

    def factor(self, factor, limit=None):
        limit = len(self.nodes) if limit is None else limit
        require(isinstance(factor, (list, tuple)) and len(factor) == 2, 'factor')
        node, sign = factor
        require(integer(node) and node < limit, 'factor_node')
        require(type(sign) is int and sign in (-1, 1), 'factor_sign')

    def leaves(self, node, sign=1):
        """Signed leaf references in chronological order, without recursion."""
        self.factor([node, sign])
        # Iterator frames avoid both Python recursion and a stack proportional
        # to the expanded word or a large product's number of children.
        stack = [iter(((node, sign),))]
        while stack:
            item = next(stack[-1], None)
            if item is None:
                stack.pop()
                continue
            j, sg = item
            if j in self.words:
                yield j, sg
                continue
            nd = self.nodes[j]
            if nd['op'] == 'conj':
                u, us, x = nd['u'], nd['us'], nd['x']
                stack.append(iter(((u, -us), (x, sg), (u, us))))
            elif sg > 0:
                stack.append(iter(nd['f']))
            else:
                stack.append(((child, -child_sign) for child, child_sign in reversed(nd['f'])))

    def expand(self, node, sign=1):
        for leaf, sg in self.leaves(node, sign):
            word = self.words[leaf]
            if sg > 0:
                yield from word
            else:
                for move in reversed(word):
                    yield -move


@functools.cache
def simulator():
    # orient.set_pose_table is the importer used by screen.load / buffer.
    # Its dependency imports are not used to expand, solve, or check a word.
    sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
    import sim
    import orient
    import wj
    ctx = sim.get_context()
    return sim, orient.set_pose_table, ctx, wj.make_menu(ctx, 'I_a')


def state_labels(state):
    require(state.is_lattice(), 'state_lattice')
    stickers = state.lattice_stickers()
    require(bool(np.all(stickers['frame_agrees'])), 'sticker_frames')
    labels = np.asarray(stickers['labels'], np.int64)
    require(labels.shape == (NS,), 'sticker_count')
    return labels


class LatticePlay:
    """One sticker gather per primitive, with exact auxiliary pose accounting.

    Labels always play literally. Only pose accounting for an explicit leaf
    word may be cached; no cached net ever advances the played sticker labels.
    Interrupted leaves transport precisely their played prefix before hashing.
    """

    def __init__(self, state):
        self.ctx, self.menu, self.state_type = state.ctx, state.menu, type(state)
        data, kp, reg = self.ctx.data, self.ctx.kplus, self.ctx.regions
        # All labels are below NS: int32 is exact and keeps the per-twist
        # gathers small. Hashes still use the format's little-endian int64.
        self.labels = state_labels(state).astype(np.int32)
        k = state._kpose()[state.pose_id]
        destination = reg.piece_of[reg.orbit_of, kp.compose_vec(k, reg.transport)]
        require(len(np.unique(destination)) == len(k), 'piece_bijection')
        self.poses = np.empty(len(k), np.int16)
        self.poses[destination] = k
        with np.load(ROOT / 'assets/primitives.npz', allow_pickle=False) as p:
            src, dst, offsets = p['src'], p['dst'], p['offsets']
        self.moves = {}
        for i in range(1200):
            s = src[offsets[i]:offsets[i + 1]].astype(np.intp)
            d = dst[offsets[i]:offsets[i + 1]].astype(np.intp)
            ps, pd = data.move_src[i], data.move_dst[i]
            self.moves[i + 1] = s, d, ps, pd
            self.moves[-i - 1] = d, s, pd, ps
        self.composition = {}
        self.word_fields = {}
        self.defer_poses = False
        # 7200**2 int16 entries: about 104 MB at most. Fill rows lazily;
        # arbitrary pose-field compositions then need only an exact lookup.
        self.pose_products = np.empty((len(kp.perms), len(kp.perms)), dtype=np.int16)
        self.pose_rows_ready = set()

    def _pose_row(self, g):
        if g not in self.pose_rows_ready:
            kp = self.ctx.kplus
            self.pose_products[g] = kp.compose_vec(np.full(len(kp.perms), g), np.arange(len(kp.perms)))
            self.pose_rows_ready.add(g)
        return self.pose_products[g]

    def _composition(self, move):
        composition = self.composition.get(move)
        if composition is None:
            kp = self.ctx.kplus
            g = int(kp.gen_idx[abs(move) - 1])
            if move < 0:
                g = int(kp.inv[g])
            composition = self._pose_row(g)
            self.composition[move] = composition
        return composition

    def play(self, move):
        src, dst, piece_src, piece_dst = self.moves[move]
        self.labels[dst] = self.labels[src]
        if not self.defer_poses:
            self.poses[piece_dst] = self._composition(move)[self.poses[piece_src]]

    def _pose_field(self, letters):
        """Exact source positions and left rotations, including invisible ones."""
        identity = np.arange(len(self.poses), dtype=np.int32)
        origins = identity.copy()
        rotations = np.zeros(len(self.poses), dtype=np.int16)
        for move in letters:
            _, _, src, dst = self.moves[move]
            origins[dst] = origins[src]
            rotations[dst] = self._composition(move)[rotations[src]]
        dst = np.flatnonzero((origins != identity) | (rotations != 0))
        return origins[dst], dst, rotations[dst]

    def _prepare_field(self, field):
        for g in np.unique(field[2]):
            self._pose_row(int(g))
        return field

    def transport_word(self, word, sign, count):
        """Account for poses after count literal twists of this signed leaf."""
        if count == 0:
            return
        if count < len(word):
            letters = iter(word) if sign > 0 else (-m for m in reversed(word))
            field = self._prepare_field(self._pose_field(itertools.islice(letters, count)))
        else:
            field = self.word_fields.get((word, sign))
            if field is None:
                forward = self.word_fields.get((word, 1))
                if forward is None:
                    forward = self._prepare_field(self._pose_field(word))
                    self.word_fields[(word, 1)] = forward
                if sign > 0:
                    field = forward
                else:
                    src, dst, rotations = forward
                    field = self._prepare_field((dst, src, self.ctx.kplus.inv[rotations]))
                    self.word_fields[(word, sign)] = field
        src, dst, rotations = field
        if len(dst):
            self.poses[dst] = self.pose_products[rotations, self.poses[src]]

    def state(self):
        """Recover the exact played poses, including rotations invisible to labels."""
        data = self.ctx.data
        k = np.empty(len(self.poses), np.int16)
        k[data.slot_piece[self.labels]] = self.poses[data.slot_piece]
        state = self.state_type(self.ctx, menu=self.menu)
        values, inv = np.unique(k, return_inverse=True)
        ids = np.array([state._intern_k(int(v)) for v in values], np.int32)
        state.pose_id = ids[inv]
        state._compact()
        require(np.array_equal(state_labels(state), self.labels), 'pose_transport')
        return state


def script_digests():
    paths = {Path(__file__).resolve()}
    for module in tuple(sys.modules.values()):
        name = getattr(module, '__file__', None)
        if name:
            path = Path(name).resolve()
            if path.is_relative_to(J) and path.suffix == '.py':
                paths.add(path)
    return {p.relative_to(ROOT).as_posix(): sha(p) for p in sorted(paths)}


def play(restore_dir, start_table, run_index=0, library=None, seconds=None):
    """Return a report even for failed checks; no output is written by this API."""
    started = time.monotonic()
    directory, table = Path(restore_dir), Path(start_table)
    report = {
        'format': 'literal-lattice/1', 'restore_dir': directory.resolve().name, 'run': run_index,
        'input': {'table': table.name, 'digest': None, 'labels_sha256': None},
        'order': 'corrections-first', 'scheduled': {'retained': 0, 'corrections': 0},
        'played': {'retained': 0, 'corrections': 0, 'total': 0},
        'junction': {'T_digest': None, 'labels_equal': None},
        'final': {'digest': None, 'labels_identity': False},
        'result': 'failed', 'failure': None, 'position': None,
        'seconds': {'retained': 0.0, 'corrections': 0.0, 'total': 0.0},
        'model_files_checked': False, 'scripts': {},
    }
    current, lattice = None, None
    # --seconds bounds play, including building T, rather than imports/validation.
    deadline = None

    def tick(position):
        if deadline is not None and time.monotonic() >= deadline:
            raise Budget({**position, 'twists_played': report['played']['total']})

    def corrections(state, sequence, *, target=False):
        t0 = time.monotonic()
        try:
            items = reversed(sequence) if target else sequence
            for index, correction in enumerate(items):
                macro = correction['macro']
                if macro not in macro_words:
                    macro_words[macro] = [sim.Twist.from_record(ctx, record) for record in macros[macro]['records']]
                word = macro_words[macro]
                inverse = correction['inverse'] != target
                word = [twist.inverse() for twist in reversed(word)] if inverse else word
                for repetition in range(correction['times']):
                    for twist_index, twist in enumerate(word):
                        tick({'phase': 'target' if target else 'corrections',
                              'correction_index': index, 'repetition': repetition,
                              'twist_within_macro': twist_index})
                        require(menu.contains(twist), 'correction_menu')
                        outcome = state.apply(twist, journal=False)
                        require(outcome.applied and not outcome.uncertain, 'correction_admissible')
                        if not target:
                            report['played']['corrections'] += 1
                            report['played']['total'] += 1
                    require(state.is_lattice(), 'correction_lattice')
        finally:
            # Target construction is verification work, never played length.
            if not target:
                report['seconds']['corrections'] += time.monotonic() - t0

    try:
        require(seconds is None or (math.isfinite(seconds) and seconds >= 0), 'seconds')
        summary = read_json(directory / 'restore-summary.json')
        require(integer(run_index) and run_index < len(summary['runs']), 'run_index')
        run = summary['runs'][run_index]
        require(run['status'] == 'restored', 'run_status')
        order = run.get('order', 'corrections-first')
        require(order in ('retained-first', 'corrections-first'), 'order')
        report['order'] = order
        solutions_path = directory / 'solutions.json'
        if not solutions_path.exists():
            solutions_path = directory / 'solutions.json.gz'
        solution = read_json(solutions_path)
        require(solution['format'] == 'lattice-restoration-v1', 'solutions_format')
        files = solution['model_files']
        require(isinstance(files, dict) and all(name in files for name in MODEL_FILES), 'model_files')
        for name, digest in files.items():
            require(isinstance(name, str) and '\\' not in name, 'model_file_path')
            path = PurePosixPath(name)
            require(not path.is_absolute() and '..' not in path.parts and path.parts[0] == 'assets',
                    'model_file_path')
            require(sha(ROOT / name) == digest, 'model_file_sha256')
        report['model_files_checked'] = True
        factors = solution_factors(solution, run['input'])
        program = Program(solution['nodes'], read_json(ROOT / 'assets/execution_trees.json.gz'))
        require(isinstance(factors, list), 'factors')
        for factor in factors:
            program.factor(factor)
        retained_length = sum(program.lengths[j] for j, _ in factors)
        require(integer(run['retained_length']) and retained_length == run['retained_length'], 'retained_length')
        require(integer(run['factors']) and len(factors) == run['factors'], 'factor_count')
        report['scheduled']['retained'] = retained_length

        # Prefer the restore-local library, then the committed compressed copy.
        if library is None:
            candidates = [directory / 'library-merged.json', directory / 'library-merged.json.gz', DEFAULT_LIBRARY]
            lib = None
            for candidate in candidates:
                if candidate.exists():
                    try:
                        lib = checked_library(candidate, summary['library']['sha256'])
                        break
                    except Failed:
                        continue
            require(lib is not None, 'library_sha256')
        else:
            lib = checked_library(library, summary['library']['sha256'])
        sim, load_table, ctx, menu = simulator()
        require(lib['menu'] == 'I_a', 'library_menu')
        macros, macro_words = lib['macros'], {}
        sequence = run['corrections']
        require(isinstance(sequence, list), 'corrections')
        for correction in sequence:
            require(isinstance(correction, dict) and integer(correction['macro']) and
                    correction['macro'] < len(macros) and integer(correction['times'], 1) and
                    type(correction['inverse']) is bool, 'corrections')
        correction_length = sum(len(macros[c['macro']]['records']) * c['times'] for c in sequence)
        report['scheduled']['corrections'] = correction_length
        if 'correction_twists' in run:
            require(correction_length == run['correction_twists'], 'correction_twists')

        doc = read_json(table)
        pose_ids = doc['pose_id']
        require(isinstance(pose_ids, list) and len(pose_ids) == len(ctx.data.orbit_id) and
                all(integer(i) and i < len(doc['poses']) for i in pose_ids), 'table_pose_ids')
        current = sim.State(ctx, menu=menu)
        try:
            load_table(current, doc)
        except (AssertionError, ValueError, IndexError, KeyError, TypeError) as exc:
            raise Failed('table_digest') from exc
        digest = current.digest()
        report['input']['digest'] = digest
        require(current.is_lattice(), 'input_lattice')
        labels = state_labels(current)
        report['input']['labels_sha256'] = labels_sha(labels)
        require(digest == run['input_digest'], 'input_digest')
        # Older corrections-first outputs hash the labels before correcting,
        # although their start-labels file contains the corrected labels.
        require(report['input']['labels_sha256'] == run['input_labels_sha256'], 'input_labels_sha256')
        deadline = None if seconds is None else time.monotonic() + seconds
        if order == 'corrections-first':
            corrections(current, sequence)
            if sequence:
                require(current.digest() == run['post_correction_digest'], 'post_correction_digest')

        lattice = LatticePlay(current)
        t0 = time.monotonic()
        try:
            for factor_index, (node, sign) in enumerate(factors):
                count = 0
                for leaf, sg in program.leaves(node, sign):
                    word = program.words[leaf]
                    letters = iter(word) if sg > 0 else (-m for m in reversed(word))
                    # Long seed words recur thousands of times. Their sticker
                    # twists are all played; their auxiliary pose fields can
                    # be reused. Tree paths keep primitive pose accounting.
                    lattice.defer_poses = program.nodes[leaf]['op'] == 'word'
                    leaf_count = 0
                    try:
                        for move in letters:
                            if deadline is not None:
                                tick({'phase': 'retained', 'factor_index': factor_index,
                                      'twist_within_factor': count})
                            lattice.play(move)
                            leaf_count += 1
                            count += 1
                            report['played']['retained'] += 1
                            report['played']['total'] += 1
                    finally:
                        if lattice.defer_poses:
                            lattice.transport_word(word, sg, leaf_count)
                        lattice.defer_poses = False
                require(count == program.lengths[node], 'factor_played_length')
        finally:
            report['seconds']['retained'] += time.monotonic() - t0
        require(report['played']['retained'] == retained_length, 'retained_played_length')
        if order == 'retained-first':
            target = sim.State(ctx, menu=menu)
            corrections(target, sequence, target=True)
            report['junction']['T_digest'] = target.digest()
            require(target.digest() == run['T_digest'], 'T_digest')
            report['junction']['labels_equal'] = bool(np.array_equal(lattice.labels, state_labels(target)))
            require(report['junction']['labels_equal'], 'junction_labels')
            # T is only the junction check. C is played on the literal state
            # Y.R, which may differ from T by rotations the labels do not see,
            # so final.digest is the digest of the state actually played.
            current, lattice = lattice.state(), None
            corrections(current, sequence)
        else:
            current = lattice.state()
            lattice = None
        report['final'] = {'digest': current.digest(),
                           'labels_identity': bool(np.array_equal(state_labels(current), np.arange(NS)))}
        require(report['final']['labels_identity'], 'final_labels_identity')
        require(report['played']['corrections'] == correction_length, 'correction_played_length')
        tick({'phase': 'complete', 'factor_index': len(factors), 'twist_within_factor': 0})
        report['result'] = 'solved'
    except Budget as exc:
        report['result'], report['position'] = 'budget', exc.position
    except Failed as exc:
        report['failure'] = str(exc)
    except (OSError, EOFError, ValueError, KeyError, TypeError, IndexError, AssertionError,
            OverflowError, AttributeError):
        # Do not leak table paths, library contents, or machine diagnostics.
        report['failure'] = 'input_format'
    finally:
        if report['result'] != 'solved' and current is not None:
            try:
                if lattice is not None:
                    current = lattice.state()
                report['final']['digest'] = current.digest()
                if current.is_lattice():
                    report['final']['labels_identity'] = bool(np.array_equal(state_labels(current), np.arange(NS)))
            except (Failed, ValueError, IndexError, AssertionError):
                report['failure'], report['result'] = 'pose_transport', 'failed'
                report['position'] = None
        report['scripts'] = script_digests()
        report['seconds']['total'] = time.monotonic() - started
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('restore_dir', type=Path)
    parser.add_argument('out_json', type=Path)
    parser.add_argument('--start-table', required=True, type=Path)
    parser.add_argument('--run', type=int, default=0)
    parser.add_argument('--library', type=Path)
    parser.add_argument('--seconds', type=float)
    args = parser.parse_args(argv)
    report = play(args.restore_dir, args.start_table, args.run, args.library, args.seconds)
    atomic_json(args.out_json, report)
    print(json.dumps({'result': report['result'], 'failure': report['failure'], 'played': report['played']}))
    return 1 if report['result'] == 'failed' else 0


if __name__ == '__main__':
    raise SystemExit(main())
