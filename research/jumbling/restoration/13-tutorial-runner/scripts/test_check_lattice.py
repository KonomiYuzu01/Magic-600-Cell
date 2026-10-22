"""P5 acceptance: independent literal lattice replay, exact headless evidence only.

Run from the repository root with python3 -I -B. Format writers are oracles;
all reference configurations are built with State.apply, never the fast model.
All outputs and caches live in one temporary tree. Git is used only for the
required read-only worktree inventory; no simulation runs in a child process.
"""

import ast
import atexit
import copy
import gzip
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
for _variable in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_variable] = '1'
TEMP = Path(tempfile.mkdtemp(prefix='check-lattice-'))
atexit.register(shutil.rmtree, TEMP, ignore_errors=True)
os.environ['JUMBLING_HOME_CACHE'] = str(TEMP / 'home-cache')
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
J = ROOT / 'research/jumbling'
sys.path[:0] = [str(HERE), str(J / 'theory'), str(J), str(J / 'fixtures')]

# Match test_runner's private exclusions, without importing its player code.
SKIP = ('.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def inventory():
    result = {}
    for directory, dirs, files in os.walk(ROOT):
        here = Path(directory).relative_to(ROOT)
        dirs[:] = sorted(d for d in dirs if (here / d).as_posix() not in SKIP)
        for name in sorted(files):
            path = Path(directory) / name
            if path == ROOT / '.git':
                continue
            try:
                value = os.readlink(path) if path.is_symlink() else hashlib.sha256(path.read_bytes()).hexdigest()
            except FileNotFoundError:
                continue
            result[path.relative_to(ROOT).as_posix()] = value
    return result


def status():
    return subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain', '--ignored',
        '--untracked-files=all', '--', '.'] + [':(exclude)' + p for p in SKIP[1:]], cwd=ROOT)


# Include third-party imports in both the elapsed time and the no-write proof.
STARTED = time.monotonic()
BASELINE = (status(), inventory()) if __name__ == '__main__' else None

import check_lattice as C  # noqa: E402
import numpy as np  # noqa: E402
import restore  # noqa: E402
import sim  # noqa: E402
import runner_api as formats  # noqa: E402 -- file-format oracles only
import test_check_run as scan_oracle  # noqa: E402 -- do not collect its test classes

SOLVED = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
W9 = [(0, 12), (2, 12), (161, 58), (139, 26), (16, 1),
      (139, 58), (161, 26), (2, 45), (0, 45)]
OFF_COUNTS = [3097, 5404, 8501, 10808, 10808, 8501, 5404, 3097, 0]


def clone(state):
    other = sim.State(state.ctx, menu=state.menu)
    other.pose_id = state.pose_id.copy()
    other._poses, other._lookup = list(state._poses), dict(state._lookup)
    return other


def code_word(word):
    return [60 * grip + item for grip, item in word]


def write_word(path, word):
    writer = formats.Ia16Writer(path)
    writer.write_letters(word)
    return writer.close()


def write_raw_tree(path, root):
    data = (json.dumps(root, separators=(',', ':'), allow_nan=False) + '\n').encode('utf-8')
    path.write_bytes(gzip.compress(data, mtime=0))
    size, stack = 0, [root]
    while stack:
        part = stack.pop()
        size += 1
        stack.extend(part.get('parts', []))
    return {'sha256': hashlib.sha256(data).hexdigest(), 'size': size}


def snapshot(state):
    return {'digest': state.digest(), 'off': state.off_lattice_count(), 'heights': {}}


def make_row(directory, index, word, root, before, after, *, identity=None):
    word_name = f'words/{index:05d}.ia16.gz'
    tree_name = f'step-trees/{index:05d}.json.gz'
    info = write_word(directory / word_name, word)
    tree = formats.write_tree(directory / tree_name, root, len(word))
    row = {'index': index, 'phase': 'lattice', 'subphase': None,
           'rule': 'lattice/orbit-0' if index == 0 else 'lattice/correction-0-0',
           'word': word_name, 'letters': info['letters'], 'encoding': 'ia16',
           'word_sha256': info['sha256'], 'tree': {'file': tree_name, **tree},
           'contract': 'lattice_literal', 'gate': 'player', 'standing': 'rule',
           'before': before, 'after': after, 'seconds': 0.0, 'attempts': []}
    if identity is not None:
        row['labels_identity'] = identity
    return row


class AIndependenceTests(unittest.TestCase):
    def scan(self, source):
        problems = scan_oracle.IndependenceTests().scan(source)
        for node in ast.walk(ast.parse(source)):
            if isinstance(node, ast.Import):
                names = [alias.name for alias in node.names]
            elif isinstance(node, ast.ImportFrom):
                names = [node.module or '']
            else:
                names = []
            problems.extend('process import' for name in names
                            if name.split('.')[0] in ('subprocess', 'multiprocessing'))
            if isinstance(node, (ast.Assign, ast.AnnAssign)):
                targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                if any(isinstance(n, ast.Name) and n.id == 'STUB'
                       for target in targets for n in ast.walk(target)):
                    problems.append('stub assignment')
        if 'unchecked_contract' in source:
            problems.append('unchecked contract')
        return problems

    def test_a_source_and_scan(self):
        self.assertEqual(self.scan((HERE / 'check_lattice.py').read_text()), [])
        for source in ('import literal_lattice', 'from runner_api import Ia16Writer', 'import exact',
                       'restore.jumble_items(m)', "__import__('seams')", 'import subprocess',
                       'import multiprocessing', 'STUB = False'):
            with self.subTest(source=source):
                self.assertTrue(self.scan(source))
        self.assertEqual(C._moves.cache_info().currsize, 0, 'primitive moves loaded at import')


class BReaderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = Path(tempfile.mkdtemp(dir=TEMP, prefix='readers-'))
        cls.fixtures = []
        for length in (0, 1, 1000, 200000):
            codes = ((np.arange(length, dtype=np.int64) * 7919 + 35999) % 36000).astype('<u2')
            path = cls.directory / f'{length}.ia16.gz'
            writer = formats.Ia16Writer(path)
            writer.write_codes(codes.tolist())
            cls.fixtures.append((path, codes, writer.close()))

    def test_b_decoders_and_overlapping_ranges(self):
        # 257 is odd and smaller than even the 1,000-letter file (2,000 bytes).
        for path, codes, expected in self.fixtures:
            self.assertEqual(formats.ia16_digest(path), expected)
            word = [[int(c) // 60, int(c) % 60] for c in codes]
            for chunk_bytes in (C.CHUNK_BYTES, 257):
                with self.subTest(letters=len(codes), chunk_bytes=chunk_bytes), patch.object(C, 'CHUNK_BYTES', chunk_bytes):
                    self.assertEqual(C.ia16_info(path), expected)
                    self.assertEqual(C.read_ia16(path, expected['sha256']), word)
                    with self.assertRaises(C.Sha256Mismatch):
                        C.read_ia16(path, '0' * 64)
                    n = len(codes)
                    ranges = [(n // 2, n), (0, min(n, 17)),
                              (max(0, n // 2 - 3), min(n, n // 2 + 13)), (n, n), (0, n)]
                    actual = C.ia16_ranges(path, ranges)
                    for found, (lo, hi) in zip(actual, ranges):
                        self.assertEqual(found.dtype, np.dtype('<u2'))
                        np.testing.assert_array_equal(found, codes[lo:hi])
                    if n:
                        self.assertTrue(np.shares_memory(actual[0], actual[-1]))
        # The smallest possible odd read also has to carry a code correctly.
        with patch.object(C, 'CHUNK_BYTES', 1):
            self.assertEqual(C.ia16_info(self.fixtures[1][0]), self.fixtures[1][2])

    def test_b_bad_files_and_range_past_end(self):
        good = self.fixtures[2][0].read_bytes()
        cases = {'truncated': (good[:len(good) // 2], 'word_file'),
                 'plain': (b'not gzip', 'word_file'),
                 'mtime': (gzip.compress(b'\x00\x00', mtime=1), 'word_file'),
                 'odd': (gzip.compress(b'\x00', mtime=0), 'word_file'),
                 'invalid': (gzip.compress((36000).to_bytes(2, 'little'), mtime=0), 'word_literal'),
                 'corrupt': (good[:-8] + bytes([good[-8] ^ 1]) + good[-7:], 'word_file'),
                 'missing': (None, 'word_file')}
        for name, (data, failure) in cases.items():
            path = self.directory / f'bad-{name}.gz'
            if data is not None:
                path.write_bytes(data)
            with self.subTest(case=name):
                for reader in (C.ia16_info, lambda p: C.ia16_ranges(p, [])):
                    with self.assertRaises(C.Ia16Error) as caught:
                        reader(path)
                    self.assertEqual(caught.exception.name, failure)
                    self.assertEqual(str(caught.exception), failure)
                with self.assertRaises(ValueError):
                    C.read_ia16(path)
        for ranges in ([(0, 1001)], [(1001, 1001)], [(0, 10**30)], [(-1, 0)], [(2, 1)], [(False, 1)]):
            with self.subTest(ranges=ranges), self.assertRaises(C.Ia16Error) as caught:
                C.ia16_ranges(self.fixtures[2][0], ranges)
            self.assertEqual(caught.exception.name, 'word_file')


class CLatticeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        started = time.perf_counter()
        cls.menu = restore.menu('I_a')
        cls.ctx = restore.ctx
        cls.retained = [i for i, (_, m) in enumerate(cls.menu.items)
                        if cls.ctx.kplus.index_of_matrix(m) >= 0]
        assert len(cls.menu.items) == 60 and cls.retained == list(range(12)), cls.retained
        cls.inverse = {i: next(j for j, (_, n) in enumerate(cls.menu.items)
                              if restore.to_tuple(restore.transpose(m)) == restore.to_tuple(n))
                       for i, (_, m) in enumerate(cls.menu.items)}
        cls.twists = {}
        library = json.loads(gzip.decompress((J / 'restoration/09-general-method/data/library-merged.json.gz').read_bytes()))
        macro = library['macros'][0]
        assert len(library['macros']) == 36 and macro['family'] == 'w9' and len(macro['records']) == 9
        cls.C = []
        for record in macro['records']:
            twist = sim.Twist.from_record(cls.ctx, record)
            items = [i for i in range(60) if restore.element(cls.menu, twist.grip, i) == twist.matrix]
            assert len(items) == 1, items
            letter = (twist.grip, items[0])
            cls.C.append(letter)
            cls.twists[letter] = twist
        assert cls.C == W9, cls.C
        assert [p for p in cls.C if p[1] in cls.retained] == [(16, 1)]
        rng = random.Random(5005)
        A = [(rng.randrange(600), rng.choice(cls.retained)) for _ in range(250)]
        B = [(rng.randrange(600), rng.choice(cls.retained)) for _ in range(250)]
        cls.W = A + B + cls.inverted(A) + B
        assert {i for _, i in A + B} == set(cls.retained)
        cls.solved = sim.State(cls.ctx, menu=cls.menu)
        assert cls.solved.digest() == SOLVED
        from_solved, observed = clone(cls.solved), []
        for letter in cls.C:
            cls.apply_one(from_solved, letter)
            observed.append(from_solved.off_lattice_count())
        assert observed == OFF_COUNTS, observed
        cls.T = clone(cls.solved)
        cls.apply_word(cls.T, cls.inverted(cls.C))
        assert cls.T.is_lattice()
        target_stickers = cls.T.lattice_stickers()
        assert target_stickers['frame_agrees'].all()
        assert not np.array_equal(target_stickers['labels'], np.arange(len(target_stickers['labels'])))
        cls.Y = clone(cls.T)
        cls.apply_word(cls.Y, cls.inverted(cls.W))
        assert cls.Y.is_lattice()
        reference, cls.prefixes = clone(cls.Y), {}
        for n, letter in enumerate(cls.W, 1):
            cls.apply_one(reference, letter)
            if n in (1, 333, 1000):
                cls.prefixes[n] = (reference.digest(), reference.lattice_stickers()['labels'].copy())
        assert reference.digest() == cls.T.digest(), (reference.digest(), cls.T.digest())
        observed = []
        for letter in cls.C:
            cls.apply_one(reference, letter)
            observed.append(reference.off_lattice_count())
        assert observed == OFF_COUNTS, observed
        assert reference.digest() == SOLVED, reference.digest()
        cls.S = cls.invisible_rotation(cls.T, target_stickers['labels'])
        invisible_end = clone(cls.S)
        cls.apply_word(invisible_end, cls.C)
        labels = invisible_end.lattice_stickers()
        assert labels['frame_agrees'].all() and np.array_equal(labels['labels'], np.arange(len(labels['labels'])))
        cls.invisible_digest = invisible_end.digest()
        assert cls.invisible_digest != SOLVED, cls.invisible_digest
        # Independent reference for the required mid-correction stop.
        changed = cls.C.copy()
        changed[3] = (139, 12)
        partial = clone(cls.T)
        cls.apply_word(partial, changed[:4])
        cls.partial_digest = partial.digest()
        refused = partial.apply(cls.twist(changed[4]), journal=False)
        assert not refused.applied and partial.digest() == cls.partial_digest
        cls.template = Path(tempfile.mkdtemp(dir=TEMP, prefix='fixture-'))
        ai = formats.span('A inverse', 500, 750)
        ai['inverse_of'] = [0, 0]
        bs = formats.span('B', 750, 1000)
        bs['same_as'] = [0, 1]
        cls.roots = [formats.group('orbit 0', [
            formats.group('factor 0', [formats.span('A', 0, 250), formats.span('B', 250, 500)]),
            formats.group('factor 1', [ai, bs])]),
            formats.group('correction 0', [formats.span('macro 0 w9', 0, 9)])]
        cls.rows = [make_row(cls.template, 0, cls.W, cls.roots[0], snapshot(cls.Y), snapshot(cls.T)),
                    make_row(cls.template, 1, cls.C, cls.roots[1], snapshot(cls.T), snapshot(cls.solved), identity=True)]
        print(f'Fixture (State.apply references, Y/T/S): {time.perf_counter() - started:.3f}s', flush=True)

    @classmethod
    def inverted(cls, word):
        return [(c, cls.inverse[i]) for c, i in reversed(word)]

    @classmethod
    def twist(cls, letter):
        if letter not in cls.twists:
            c, i = letter
            cls.twists[letter] = sim.Twist(cls.ctx, c, restore.element(cls.menu, c, i),
                                           'menu', {'menu': cls.menu.name})
        return cls.twists[letter]

    @classmethod
    def apply_one(cls, state, letter):
        outcome = state.apply(cls.twist(letter), journal=False)
        assert outcome.applied and not outcome.uncertain, (letter, outcome)

    @classmethod
    def apply_word(cls, state, word):
        for letter in word:
            cls.apply_one(state, letter)

    @classmethod
    def invisible_rotation(cls, target, labels):
        kp, reg = cls.ctx.kplus, cls.ctx.regions
        rotations = np.arange(len(kp.perms))
        # Search exact stabilizers of unmoved pieces; accept only unchanged labels
        # with frame agreement and a changed exact digest.
        for piece in np.flatnonzero(target.pose_id == 0)[-60:][::-1]:
            transport = kp.compose_vec(rotations, np.full(len(rotations), reg.transport[piece]))
            dest = reg.piece_of[reg.orbit_of[piece], transport]
            for h in rotations[(dest == piece) & (rotations != 0)][:4]:
                state = clone(target)
                state.pose_id[piece] = state._intern_k(int(h))
                state._compact()
                stickers = state.lattice_stickers()
                if stickers['frame_agrees'].all() and np.array_equal(stickers['labels'], labels):
                    assert state.digest() != target.digest()
                    return state
        raise AssertionError('no invisible rotation found in the recorded candidate set')

    def case(self, index=0):
        directory = Path(tempfile.mkdtemp(dir=TEMP, prefix='case-'))
        self.addCleanup(shutil.rmtree, directory)
        shutil.copytree(self.template, directory, dirs_exist_ok=True)
        row = copy.deepcopy(self.rows[index])
        return directory, row, (directory / row['word']).resolve(), clone(self.Y if index == 0 else self.T)

    def replay(self, state, row, path, expected):
        stats = {}
        result = C.replay_lattice_step(state, row, path, self.menu, stats=stats)
        self.assertEqual(len(result), len(set(result)), result)
        self.assertEqual(set(result), set(expected), (result, stats))
        self.assertTrue(all(isinstance(n, str) and n for n in result))
        self.assertEqual(stats['letters'], stats['fast'] + stats['simulated'])
        self.assertGreaterEqual(stats['seconds'], 0)
        return stats

    def update_tree(self, directory, row, root):
        row['tree'].update(write_raw_tree(directory / row['tree']['file'], root))

    def changed_word(self, word):
        return [(c, self.retained[(self.retained.index(i) + 1) % len(self.retained)]) for c, i in word]

    def test_c_model_three_prefixes(self):
        started = time.perf_counter()
        model = C.LatticeModel.from_state(clone(self.Y), self.menu)
        into = time.perf_counter() - started
        out = []
        for n, code in enumerate(code_word(self.W), 1):
            model.play(code)
            if n in self.prefixes:
                started = time.perf_counter()
                state, agrees = model.to_state()
                self.assertTrue(agrees)
                out.append(time.perf_counter() - started)
                self.assertEqual(state.digest(), self.prefixes[n][0])
                np.testing.assert_array_equal(model.labels, self.prefixes[n][1])
        print(f'Conversions: State -> model {into:.6f}s; model -> State {sum(out) / len(out):.6f}s (mean of 3).', flush=True)

    def test_c_invisible_rotation_retained_word(self):
        ends = []
        for start in (self.S, self.T):
            reference = clone(start)
            model = C.LatticeModel.from_state(clone(start), self.menu)
            for letter in self.W[:50]:
                model.play(60 * letter[0] + letter[1])
                self.apply_one(reference, letter)
            rebuilt, agrees = model.to_state()
            self.assertTrue(agrees)
            self.assertEqual(rebuilt.digest(), reference.digest())
            np.testing.assert_array_equal(model.labels, reference.lattice_stickers()['labels'])
            ends.append(rebuilt.digest())
        self.assertNotEqual(*ends)

    def test_c_guard_and_off_lattice_refusal(self):
        model = C.LatticeModel.from_state(clone(self.T), self.menu)
        labels, poses = model.labels.copy(), model.place_pose.copy()
        with self.assertRaises(C.FastPathError):
            model.play(12)
        np.testing.assert_array_equal(model.labels, labels)
        np.testing.assert_array_equal(model.place_pose, poses)
        state = clone(self.solved)
        self.apply_one(state, self.C[0])
        self.assertEqual(state.off_lattice_count(), 3097)
        with self.assertRaises(ValueError):
            C.LatticeModel.from_state(state, self.menu)

    def test_d_two_positive_steps_in_place(self):
        directory, first, path, state = self.case()
        self.assertEqual(C.check_tree_ranges(path, self.roots[0], self.menu), [])
        stats = self.replay(state, first, path, set())
        self.assertEqual((stats['letters'], stats['fast'], stats['simulated'], stats['switches']), (1000, 1000, 0, 2))
        self.assertEqual(state.digest(), self.T.digest())
        self.assertEqual(stats['digest'], self.T.digest())
        model = C.LatticeModel.from_state(clone(self.Y), self.menu)
        codes = code_word(self.W)
        self.assertTrue(all(C.fast_letter(code, True) for code in codes))
        started = time.perf_counter()
        for code in codes:
            if C.fast_letter(code, True):
                model.play(code)
            else:
                self.fail('a retained fixture letter must use the model')
        model_seconds = time.perf_counter() - started
        rebuilt, agrees = model.to_state()
        self.assertTrue(agrees)
        self.assertEqual(rebuilt.digest(), self.T.digest())
        print(f'Test D retained model loop: {1000 / model_seconds:.1f} letters/s; '
              f'whole step: {1000 / stats["seconds"]:.1f} letters/s '
              '(includes files, conversions and end checks).', flush=True)
        second = copy.deepcopy(self.rows[1])
        path = (directory / second['word']).resolve()
        self.assertEqual(C.check_tree_ranges(path, self.roots[1], self.menu), [])
        stats = self.replay(state, second, path, set())
        self.assertEqual((stats['fast'], stats['simulated'], stats['letters'], stats['switches']), (0, 9, 9, 0))
        self.assertIs(stats['labels_identity'], True)
        self.assertEqual(stats['digest'], SOLVED)
        self.assertEqual(state.digest(), SOLVED)
        self.assertEqual(state.journal, [])

    def test_e_w9_identity_labels_with_non_solved_digest(self):
        for variant, failures in (('valid', set()), ('digest', {'after_digest'}), ('labels', {'labels_identity'})):
            with self.subTest(variant=variant):
                _, row, path, _ = self.case(1)
                state = clone(self.S)
                row['before'] = snapshot(self.S)
                row['after']['digest'] = SOLVED if variant == 'digest' else self.invisible_digest
                if variant == 'labels':
                    row['labels_identity'] = False
                stats = self.replay(state, row, path, failures)
                self.assertIs(stats['labels_identity'], True)
                self.assertEqual(state.digest(), self.invisible_digest)
                self.assertEqual(stats['digest'], self.invisible_digest)
                self.assertNotEqual(state.digest(), SOLVED)

    def test_f_changed_retained_letter_and_same_as_word(self):
        for variant, expected in (('stale-sha', {'word_sha256'}),
                                  ('fresh-sha', {'tree_expansion', 'after_digest'}),
                                  ('same-as', {'tree_expansion', 'after_digest'})):
            with self.subTest(variant=variant):
                _, row, path, state = self.case()
                word = self.W.copy()
                if variant == 'same-as':
                    word[750:] = self.changed_word(word[750:])
                else:
                    word[:1] = self.changed_word(word[:1])
                info = write_word(path, word)
                if variant != 'stale-sha':
                    row['word_sha256'] = info['sha256']
                stats = self.replay(state, row, path, expected)
                if variant == 'stale-sha':
                    self.assertEqual(state.digest(), self.Y.digest())
                    self.assertEqual(stats['letters'], 0)
                    self.assertIsNone(stats['digest'])
                else:
                    self.assertEqual(stats['letters'], 1000)

    def test_f_changed_correction_and_partial_state(self):
        for index, letter, failures in ((3, (139, 12), {'admissible'}),
                                         (8, (0, 14), {'after_digest', 'labels_identity'})):
            with self.subTest(index=index):
                _, row, path, state = self.case(1)
                word = self.C.copy()
                word[index] = letter
                row['word_sha256'] = write_word(path, word)['sha256']
                stats = self.replay(state, row, path, failures)
                self.assertEqual(stats['fast'], 0)
                if index == 3:
                    self.assertEqual(state.digest(), self.partial_digest)
                    self.assertEqual(stats['simulated'], 4)
                    self.assertIsNone(stats['labels_identity'])
                    self.assertIsNone(stats['digest'])
                else:
                    reference = clone(self.T)
                    self.apply_word(reference, word)
                    self.assertTrue(reference.is_lattice())
                    self.assertEqual(state.digest(), reference.digest())
                    self.assertIs(stats['labels_identity'], False)

    def test_f_file_truncation_count_and_sha(self):
        for variant, failures in (('truncated', {'word_file'}), ('short', {'letters', 'word_sha256'}),
                                  ('sha', {'word_sha256'})):
            with self.subTest(variant=variant):
                _, row, path, state = self.case()
                if variant == 'truncated':
                    raw = path.read_bytes()
                    path.write_bytes(raw[:len(raw) // 2])
                elif variant == 'short':
                    write_word(path, self.W[:-1])
                else:
                    row['word_sha256'] = 'f' * 64
                with patch.object(C, 'check_tree_ranges', side_effect=AssertionError('must not check tree')):
                    stats = self.replay(state, row, path, failures)
                self.assertEqual(state.digest(), self.Y.digest())
                self.assertEqual(stats['letters'], 0)

    def test_f_replay_rechecks_replaced_word(self):
        for longer in (False, True):
            with self.subTest(longer=longer):
                directory, _, _, _ = self.case()
                word = [(5, 2), (5, self.inverse[2])] * 10
                replacement = word + word[:2] if longer else [(6, i) for _, i in word]
                row = make_row(directory, 2, word, formats.span('identity', 0, len(word)),
                               snapshot(self.solved), snapshot(self.solved), identity=True)
                path = directory / row['word']
                real_info = C.ia16_info

                def replaced_after_verify(path):
                    info = real_info(path)
                    write_word(path, replacement)
                    return info

                state = clone(self.solved)
                with patch.object(C, 'ia16_info', side_effect=replaced_after_verify) as verify, \
                        patch.object(C, 'check_tree_ranges', return_value=[]):
                    expected = {'word_sha256', 'letters'} if longer else {'word_sha256'}
                    stats = self.replay(state, row, path, expected)
                verify.assert_called_once_with(path)
                self.assertEqual(stats['letters'], len(replacement))
                self.assertEqual(state.digest(), SOLVED, 'the replacement has the same exact end state')

    def test_f_range_pass_rechecks_replaced_word(self):
        for longer in (False, True):
            with self.subTest(longer=longer):
                directory, _, _, _ = self.case()
                word = [(5, 2), (5, self.inverse[2])]
                replacement = word + word if longer else [(6, i) for _, i in word]
                second = formats.span('inverse', 1, 2)
                second['inverse_of'] = [0]
                root = formats.group('identity', [formats.span('forward', 0, 1), second])
                path = directory / 'race.ia16.gz'
                write_word(path, word)
                real_info = C.ia16_info

                def replaced_after_verify(path):
                    info = real_info(path)
                    write_word(path, replacement)
                    return info

                with patch.object(C, 'ia16_info', side_effect=replaced_after_verify) as verify:
                    failures = C.check_tree_ranges(path, root, self.menu)
                verify.assert_called_once_with(path)
                self.assertEqual(set(failures), {'word_sha256', 'letters'} if longer else {'word_sha256'})

    def test_f_model_label_failure_keeps_exact_end(self):
        for bad_claim in (False, True):
            with self.subTest(bad_claim=bad_claim):
                _, row, path, state = self.case()
                if bad_claim:
                    row['after'] = {'digest': '0' * 64, 'off': 1}
                    row['labels_identity'] = True
                real_stickers = sim.State.lattice_stickers
                calls = 0

                def failed_crosscheck(st):
                    nonlocal calls
                    calls += 1
                    stickers = real_stickers(st)
                    if calls == 2:
                        stickers['frame_agrees'] = stickers['frame_agrees'].copy()
                        stickers['frame_agrees'][0] = False
                    return stickers

                expected = {'model_labels', 'after_digest', 'after_off', 'labels_identity'} if bad_claim else {'model_labels'}
                with patch.object(sim.State, 'lattice_stickers', failed_crosscheck):
                    stats = self.replay(state, row, path, expected)
                self.assertGreaterEqual(calls, 3, 'step-end checks must still run after the conversion verdict')
                self.assertEqual(state.digest(), self.T.digest())
                self.assertNotEqual(state.digest(), self.Y.digest())
                self.assertEqual(stats['digest'], self.T.digest())
                self.assertIs(stats['labels_identity'], False)
                self.assertEqual((stats['letters'], stats['fast'], stats['simulated']), (1000, 1000, 0))

    def test_f_row_guards_and_end_claims(self):
        guards = [('encoding', 'json'), ('letters', -1), ('letters', True), ('letters', 1.0),
                  ('word_sha256', 'A' * 64), ('word_sha256', None)]
        for field, value in guards:
            with self.subTest(field=field, value=value):
                _, row, path, state = self.case()
                row[field] = value
                with patch.object(C, 'ia16_info', side_effect=AssertionError('must not read file')):
                    stats = self.replay(state, row, path, {field})
                self.assertEqual(state.digest(), self.Y.digest())
                self.assertEqual(stats['letters'], 0)
        for field in ('digest', 'off', 'labels_identity'):
            with self.subTest(claim=field):
                _, row, path, state = self.case()
                if field == 'labels_identity':
                    row[field] = True
                else:
                    row['after'][field] = '0' * 64 if field == 'digest' else 1
                self.replay(state, row, path, {'labels_identity' if field == 'labels_identity' else 'after_' + field})
                self.assertEqual(state.digest(), self.T.digest())

    def test_f_forced_fast_path_refuses_jumble(self):
        _, row, path, state = self.case(1)
        with patch.object(C, 'fast_letter', return_value=True):
            stats = self.replay(state, row, path, {'fast_path'})
        self.assertEqual(state.digest(), self.T.digest())
        self.assertEqual((stats['fast'], stats['simulated'], stats['letters']), (0, 0, 0))
        self.assertIsNone(stats['labels_identity'])

    def test_f_tree_reference_mutations(self):
        for variant, failure in (('wrong-inverse', 'tree_expansion'), ('forward', 'tree_malformed'),
                                  ('same-name', 'tree_malformed'), ('both', 'tree_malformed')):
            with self.subTest(variant=variant):
                directory, row, path, state = self.case()
                root = copy.deepcopy(self.roots[0])
                first, second = root['parts'][0]['parts'], root['parts'][1]['parts']
                if variant == 'wrong-inverse':
                    second[0]['inverse_of'] = [0, 1]
                elif variant == 'forward':
                    first[0]['inverse_of'] = [1, 0]
                elif variant == 'same-name':
                    second[1]['same_as'] = [0, 0]
                else:
                    second[1]['inverse_of'] = [0, 1]
                self.update_tree(directory, row, root)
                self.assertEqual(C.check_tree_ranges(path, root, self.menu), [failure])
                self.replay(state, row, path, {failure})
                self.assertEqual(state.digest(), self.T.digest(), 'tree failure must not stop verified word replay')

    def test_g_tree_structure_and_depth(self):
        _, _, path, _ = self.case()
        invalid = [None, {}, {'name': '', 'letters': 1000, 'span': [0, 1000]},
                   {'name': 'x', 'letters': 1000, 'parts': []},
                   {'name': 'x', 'letters': True, 'span': [0, 1]},
                   {'name': 'x', 'letters': 1000, 'span': [0, 1000], 'parts': []},
                   {'name': 'x', 'letters': 999, 'span': [0, 1000]},
                   {'name': 'x', 'letters': 999, 'span': [0, 999]},
                   {'name': 'x', 'letters': 1000, 'span': [1, 1001]},
                   {'name': 'x', 'letters': 1000, 'parts': [formats.span('a', 0, 500)]}]
        for root in invalid:
            with self.subTest(root=root):
                self.assertEqual(C.check_tree_ranges(path, root, self.menu), ['tree_malformed'])
        deep = formats.span('leaf', 0, 1000)
        for _ in range(63):
            deep = formats.group('level', [deep])
        self.assertEqual(C.check_tree_ranges(path, deep, self.menu), [])
        deep = formats.group('too deep', [deep])
        self.assertEqual(C.check_tree_ranges(path, deep, self.menu), ['tree_malformed'])
        # A non-empty ancestor exists in pre-order, but its end is too late.
        ancestor = copy.deepcopy(self.roots[0])
        ancestor['parts'][0]['parts'][0]['inverse_of'] = []
        self.assertEqual(C.check_tree_ranges(path, ancestor, self.menu), ['tree_malformed'])

    def test_g_tree_location_and_read_errors(self):
        variants = [('word', 'elsewhere/00000.ia16.gz', 'path'),
                    ('word', '../words/00000.ia16.gz', 'path'),
                    ('file', '../escape.json.gz', 'path'), ('file', 'C:/escape', 'path'),
                    ('file', 'missing.gz', 'tree_missing'), ('sha256', '0' * 64, 'tree_sha256'),
                    ('raw', b'not gzip', 'tree_malformed'),
                    ('raw', gzip.compress(b'{}', mtime=1), 'tree_malformed'),
                    ('json', b'{', 'tree_malformed'),
                    ('json', b'{"name":"a","name":"b"}', 'tree_malformed')]
        for field, value, failure in variants:
            with self.subTest(field=field, value=value):
                directory, row, path, state = self.case()
                if field == 'word':
                    row['word'] = value
                elif field in ('file', 'sha256'):
                    row['tree'][field] = value
                elif field == 'raw':
                    (directory / row['tree']['file']).write_bytes(value)
                else:
                    (directory / row['tree']['file']).write_bytes(gzip.compress(value, mtime=0))
                    row['tree']['sha256'] = hashlib.sha256(value).hexdigest()
                self.replay(state, row, path, {failure})
                self.assertEqual(state.digest(), self.T.digest())
        # A deeper word path still derives the same run root by its part count.
        directory, row, path, state = self.case()
        row['word'] = 'words/nested/deeper/00000.ia16.gz'
        nested = directory / row['word']
        nested.parent.mkdir(parents=True)
        shutil.copyfile(path, nested)
        self.replay(state, row, nested.resolve(), set())

    def test_g_switching_between_representations(self):
        directory, _, _, _ = self.case()
        word = [(5, 2), (0, 12), (0, self.inverse[12]), (5, self.inverse[2])]
        reference = clone(self.solved)
        self.apply_word(reference, word)
        self.assertEqual(reference.digest(), SOLVED)
        row = make_row(directory, 2, word, formats.span('mixed', 0, 4),
                       snapshot(self.solved), snapshot(reference), identity=True)
        state = clone(self.solved)
        stats = self.replay(state, row, (directory / row['word']).resolve(), set())
        self.assertEqual((stats['fast'], stats['simulated'], stats['switches']), (2, 2, 4))
        self.assertEqual(state.digest(), SOLVED)

    def test_g_failures_are_sanitized_and_end_frames_checked(self):
        _, row, path, state = self.case(1)
        with patch.object(state, 'apply', side_effect=RuntimeError('private exception text')):
            self.replay(state, row, path, {'admissible'})
        self.assertEqual(state.digest(), self.T.digest())
        _, row, path, state = self.case(1)
        with patch.object(state, 'apply', return_value=type('Outcome', (), {'applied': True, 'uncertain': [1]})()):
            self.replay(state, row, path, {'admissible'})
        self.assertEqual(state.digest(), self.T.digest())
        _, row, path, state = self.case()
        with patch.object(C, 'fast_letter', side_effect=RuntimeError('private exception text')):
            self.replay(state, row, path, {'lattice_step'})
        self.assertEqual(state.digest(), self.Y.digest())
        _, row, path, state = self.case(1)
        real_stickers = state.lattice_stickers

        def bad_frames():
            stickers = real_stickers()
            stickers['frame_agrees'][:] = False
            return stickers

        with patch.object(state, 'lattice_stickers', side_effect=bad_frames):
            self.replay(state, row, path, {'model_labels', 'labels_identity'})
        self.assertEqual(state.digest(), SOLVED)
        model = C.LatticeModel.from_state(clone(self.Y), self.menu)
        model.labels[0], model.labels[1] = model.labels[1], model.labels[0]
        with self.assertRaises(C.ModelLabelsError):
            model.to_state()


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        super().stopTest(test)
        self.stream.writeln(f'  {test.id()}: {time.monotonic() - self.started:.3f}s')


def main(names=()):
    before_status, before_files = BASELINE if BASELINE is not None else (status(), inventory())
    loader, module = unittest.defaultTestLoader, sys.modules[__name__]
    suite = loader.loadTestsFromNames(names, module) if names else loader.loadTestsFromModule(module)
    result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
    after_status, after_files = status(), inventory()
    unchanged = before_status == after_status and before_files == after_files
    if unchanged:
        print('Worktree unchanged, including ignored files.', file=sys.stderr)
    else:
        changed = sorted(p for p in before_files.keys() | after_files.keys()
                         if before_files.get(p) != after_files.get(p))
        print('FAIL: acceptance changed the worktree (including ignored files):', changed, file=sys.stderr)
    print(f'Acceptance elapsed: {time.monotonic() - STARTED:.3f}s.', file=sys.stderr)
    return 0 if result.wasSuccessful() and unchanged else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
