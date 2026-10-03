"""Headless P2 publication and power-loss cases, entirely in memory."""
from __future__ import annotations

import sys
sys.dont_write_bytecode = True

import hashlib
import json
from pathlib import Path, PurePosixPath
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools' / 'migration_probes'))
import p2_publish as P  # noqa: E402

DEST = PurePosixPath('/probe/dest')
OLD_FILES = {'state.bin': b'previous state' * 300, 'archive.bin': bytes(range(256)) * 16}
NEW_FILES = {'state.bin': b'new state' * 500, 'archive.bin': bytes(reversed(range(256))) * 16}
STARTS = ('missing', 'empty', 'previous')
BARRIERS = tuple('%s-S%d' % (side, step) for step in range(9) for side in ('before', 'after'))
FLUSHES = (
    ('S0', 'dir', DEST.parent),
    ('S0', 'dir', DEST),
    ('S0', 'dir', DEST / 'generations'),
    ('S3', 'file', DEST / 'generations/B/state.bin'),
    ('S3', 'file', DEST / 'generations/B/archive.bin'),
    ('S4', 'file', DEST / 'generations/B/MANIFEST.json'),
    ('S5', 'dir', DEST / 'generations/B'),
    ('S5', 'dir', DEST / 'generations'),
    ('S6', 'file', DEST / 'ACTIVE.tmp'),
    ('S8', 'dir', DEST),
)


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode('utf-8')


def generation_state(gen, files):
    manifest = encoded({'files': {name: {'size': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
                                  for name, data in files.items()}})
    directory = DEST / 'generations' / gen
    state = {directory / name: data for name, data in files.items()}
    state[directory / 'MANIFEST.json'] = manifest
    pointer = encoded({'generation': gen, 'manifest_sha256': hashlib.sha256(manifest).hexdigest()})
    return state, pointer


def start_state(start):
    state = {DEST.parent: None}
    if start == 'empty':
        state[DEST] = None
    elif start == 'previous':
        generation, pointer = generation_state('A', OLD_FILES)
        state.update(generation)
        state[DEST / 'ACTIVE'] = pointer
    return state


def fail_flush_at(fs, index):
    calls = []

    def fail(kind, path):
        calls.append((kind, path))
        return len(calls) - 1 == index

    fs.fail_flush = fail
    return calls


class Interrupted(RuntimeError):
    pass


class UnreadableFS(P.DoubleFS):
    unreadable = None

    def read_file(self, path):
        if path == self.unreadable:
            error = OSError('private diagnostic must not appear in records')
            error.winerror = 5
            raise error
        return super().read_file(path)


class PublicationTests(unittest.TestCase):
    def assert_active(self, fs, expected):
        result = P.recover(fs, DEST)
        self.assertEqual(result['active'], expected)
        self.assertTrue(result['valid'], result)
        if expected is not None:
            files = OLD_FILES if expected == 'A' else NEW_FILES
            for name, data in files.items():
                self.assertEqual(fs.read_file(DEST / 'generations' / expected / name), data)
        return result

    def retry(self, fs):
        fs.fail_flush = None
        fs.move_fault = None
        result = P.publish_generation(fs, DEST, 'B2', NEW_FILES)
        self.assertEqual(result['status'], 'published', result)
        self.assert_active(fs, 'B2')
        fs.power_loss()
        self.assert_active(fs, 'B2')

    def test_each_interruption_with_and_without_power_loss_then_retry(self):
        for start in STARTS:
            previous = 'A' if start == 'previous' else None
            for barrier in BARRIERS:
                expected = 'B' if barrier in ('after-S7', 'before-S8', 'after-S8') else previous
                for power_loss in (False, True):
                    with self.subTest(start=start, barrier=barrier, power_loss=power_loss):
                        fs = P.DoubleFS(start_state(start))
                        visited = []

                        def interrupt(name):
                            visited.append(name)
                            if name == barrier:
                                raise Interrupted()

                        with self.assertRaises(Interrupted):
                            P.publish_generation(fs, DEST, 'B', NEW_FILES, interrupt)
                        self.assertEqual(visited, list(BARRIERS[:BARRIERS.index(barrier) + 1]))
                        if power_loss:
                            fs.power_loss()
                        self.assert_active(fs, expected)
                        self.retry(fs)

    def test_each_precommit_flush_failure_preserves_active_then_retry(self):
        for start in STARTS:
            previous = 'A' if start == 'previous' else None
            for index, (step, kind, path) in enumerate(FLUSHES[:-1]):
                with self.subTest(start=start, flush=index, step=step):
                    fs = P.DoubleFS(start_state(start))
                    active = fs.read_file(DEST / 'ACTIVE') if previous else None
                    calls = fail_flush_at(fs, index)
                    result = P.publish_generation(fs, DEST, 'B', NEW_FILES)
                    self.assertEqual(result['status'], 'aborted', result)
                    self.assertEqual(result['step'], step)
                    self.assertEqual(result['reason'], 'OSError')
                    self.assertEqual(calls, [(kind, path) for _, kind, path in FLUSHES[:index + 1]])
                    if previous:
                        self.assertEqual(fs.read_file(DEST / 'ACTIVE'), active)
                    else:
                        self.assertFalse(fs.exists(DEST / 'ACTIVE'))
                    self.assert_active(fs, previous)
                    fs.power_loss()
                    self.assert_active(fs, previous)
                    self.retry(fs)

    def test_s8_failure_is_switched_and_survives_power_loss(self):
        for start in STARTS:
            with self.subTest(start=start):
                fs = P.DoubleFS(start_state(start))
                calls = fail_flush_at(fs, len(FLUSHES) - 1)
                result = P.publish_generation(fs, DEST, 'B', NEW_FILES)
                self.assertEqual(result['status'], 'switched-durability-unconfirmed', result)
                self.assertEqual(result['step'], 'S8')
                self.assertEqual(calls, [(kind, path) for _, kind, path in FLUSHES])
                self.assert_active(fs, 'B')
                fs.power_loss()
                self.assert_active(fs, 'B')

    def test_move_failure_before_and_after_effect(self):
        for start in STARTS:
            for fault in ('before', 'after'):
                with self.subTest(start=start, fault=fault):
                    fs = P.DoubleFS(start_state(start), move_fault=fault)
                    result = P.publish_generation(fs, DEST, 'B', NEW_FILES)
                    expected = 'A' if start == 'previous' else None
                    if fault == 'before':
                        self.assertEqual(result['status'], 'aborted', result)
                        self.assertEqual(result['step'], 'S7')
                    else:
                        self.assertEqual(result['status'], 'published', result)
                        expected = 'B'
                    self.assert_active(fs, expected)
                    fs.power_loss()
                    self.assert_active(fs, expected)
                    self.retry(fs)

    def test_move_after_effect_still_reports_s8_failure_as_switched(self):
        fs = P.DoubleFS(start_state('previous'), move_fault='after')
        fail_flush_at(fs, len(FLUSHES) - 1)
        result = P.publish_generation(fs, DEST, 'B', NEW_FILES)
        self.assertEqual(result['status'], 'switched-durability-unconfirmed', result)
        fs.power_loss()
        self.assert_active(fs, 'B')

    def test_move_read_failure_requires_recovery_before_retry(self):
        for fault, unreadable in (
            ('before', DEST / 'ACTIVE'),
            ('after', DEST / 'ACTIVE'),
            ('after', DEST / 'generations/B/MANIFEST.json'),
            ('after', DEST / 'generations/B/state.bin'),
        ):
            with self.subTest(fault=fault, unreadable=unreadable):
                fs = UnreadableFS(start_state('previous'), move_fault=fault)
                fs.unreadable = unreadable
                visited = []
                result = P.publish_generation(fs, DEST, 'B', NEW_FILES, visited.append)
                self.assertEqual(result['status'], 'publication-unknown', result)
                self.assertNotIn('before-S8', visited)
                self.assertEqual(result['recovery_error'], {'error': 'OSError', 'winerror': 5})
                self.assertNotIn('private diagnostic', json.dumps(result))
                fs.unreadable = None
                self.assert_active(fs, 'A' if fault == 'before' else 'B')
                self.retry(fs)

    def test_retry_reestablishes_ancestor_barriers_on_same_live_state(self):
        fs = P.DoubleFS(start_state('empty'))
        fs.fail_flush = lambda kind, path: kind == 'dir' and path == DEST
        result = P.publish_generation(fs, DEST, 'B', NEW_FILES)
        self.assertEqual(result['status'], 'aborted', result)
        self.assertEqual(result['step'], 'S0')
        self.assertTrue(fs.is_dir(DEST / 'generations'))
        calls = []
        fs.fail_flush = lambda kind, path: calls.append((kind, path)) or False

        def lose_power(name):
            if name == 'after-S7':
                fs.power_loss()
                raise Interrupted()

        with self.assertRaises(Interrupted):
            P.publish_generation(fs, DEST, 'B2', NEW_FILES, lose_power)
        self.assertEqual(calls[:3], [('dir', DEST.parent), ('dir', DEST), ('dir', DEST / 'generations')])
        self.assert_active(fs, 'B2')

    def test_existing_generation_is_refused_without_changing_active(self):
        fs = P.DoubleFS(start_state('previous'))
        before = fs.read_file(DEST / 'ACTIVE')
        result = P.publish_generation(fs, DEST, 'A', NEW_FILES)
        self.assertEqual(result, {'status': 'aborted', 'step': 'S1', 'reason': 'generation-exists'})
        self.assertEqual(fs.read_file(DEST / 'ACTIVE'), before)
        fs.power_loss()
        self.assert_active(fs, 'A')

    def test_invalid_file_and_generation_names_are_refused_before_writes(self):
        invalid = ('', '.', '..', 'dir/file', 'dir\\file', '/file', '\\file', 'file:stream', 'bad\x00name')
        for name in (*invalid, 'MANIFEST.json', 'manifest.json'):
            with self.subTest(file=name):
                fs = P.DoubleFS(start_state('missing'))
                result = P.publish_generation(fs, DEST, 'B', {name: b'data'})
                self.assertEqual(result['status'], 'aborted', result)
                self.assertEqual(result['reason'], 'invalid-file-name')
                self.assertFalse(fs.exists(DEST))
        for gen in invalid:
            with self.subTest(generation=gen):
                fs = P.DoubleFS(start_state('missing'))
                result = P.publish_generation(fs, DEST, gen, NEW_FILES)
                self.assertEqual(result['status'], 'aborted', result)
                self.assertFalse(fs.exists(DEST))

    def test_manifest_lists_exact_sizes_and_hashes_and_is_written_last(self):
        fs = P.DoubleFS(start_state('missing'))

        def check(name):
            directory = DEST / 'generations/B'
            if name == 'after-S3':
                self.assertEqual(fs.listdir(directory), sorted(NEW_FILES))
            if name == 'after-S4':
                manifest = json.loads(fs.read_file(directory / 'MANIFEST.json'))
                self.assertEqual(manifest, {'files': {name: {'size': len(data),
                                                            'sha256': hashlib.sha256(data).hexdigest()}
                                                     for name, data in NEW_FILES.items()}})

        self.assertEqual(P.publish_generation(fs, DEST, 'B', NEW_FILES, check)['status'], 'published')
        pointer = json.loads(fs.read_file(DEST / 'ACTIVE'))
        self.assertEqual(pointer, {'generation': 'B', 'manifest_sha256': hashlib.sha256(
            fs.read_file(DEST / 'generations/B/MANIFEST.json')).hexdigest()})


class WindowsExpectationTests(unittest.TestCase):
    def test_process_stop_switches_only_after_the_move_returned(self):
        for barrier in BARRIERS:
            with self.subTest(barrier=barrier):
                after_move = BARRIERS.index(barrier) >= BARRIERS.index('after-S7')
                self.assertEqual(P.expected_active(barrier, 'A'), 'B' if after_move else 'A')
                self.assertEqual(P.expected_active(barrier, None), 'B' if after_move else None)


class RecoveryTests(unittest.TestCase):
    def test_missing_destination_or_pointer_is_valid_unpublished_state(self):
        for start in ('missing', 'empty'):
            fs = P.DoubleFS(start_state(start))
            self.assertEqual(P.recover(fs, DEST), {'active': None, 'valid': True,
                                                  'removed_tmp': False, 'incomplete': []})

    def test_stale_tmp_removed_only_incomplete_unreferenced_generations_listed(self):
        state = start_state('previous')
        complete, _ = generation_state('C', NEW_FILES)
        state.update(complete)
        state.update({DEST / 'ACTIVE.tmp': b'stale', DEST / 'generations/B/partial.bin': b'partial'})
        fs = P.DoubleFS(state)
        result = P.recover(fs, DEST)
        self.assertEqual(result, {'active': 'A', 'valid': True, 'removed_tmp': True, 'incomplete': ['B']})
        self.assertFalse(fs.exists(DEST / 'ACTIVE.tmp'))
        self.assertTrue(fs.is_dir(DEST / 'generations/B'))
        self.assertTrue(fs.is_dir(DEST / 'generations/C'))
        self.assertFalse(P.recover(fs, DEST)['removed_tmp'])

    def test_pointer_manifest_and_listed_file_corruption_are_invalid(self):
        manifest_path = DEST / 'generations/A/MANIFEST.json'
        pointer_path = DEST / 'ACTIVE'
        file_path = DEST / 'generations/A/state.bin'
        cases = ('pointer-json', 'pointer-name', 'pointer-hash', 'manifest-missing', 'manifest-json',
                 'manifest-shape', 'manifest-name', 'file-missing', 'file-size', 'file-hash')
        for case in cases:
            with self.subTest(case=case):
                state = start_state('previous')
                if case == 'pointer-json':
                    state[pointer_path] = b'broken'
                elif case == 'pointer-name':
                    state[pointer_path] = encoded({'generation': '../A', 'manifest_sha256': '0' * 64})
                elif case == 'pointer-hash':
                    state[pointer_path] = encoded({'generation': 'A', 'manifest_sha256': '0' * 64})
                elif case == 'manifest-missing':
                    del state[manifest_path]
                elif case.startswith('manifest-'):
                    state[manifest_path] = {'manifest-json': b'broken',
                                            'manifest-shape': encoded({'files': []}),
                                            'manifest-name': encoded({'files': {'../escape': {'size': 1,
                                                                                           'sha256': '0' * 64}}})}[case]
                    state[pointer_path] = encoded({'generation': 'A', 'manifest_sha256': hashlib.sha256(
                        state[manifest_path]).hexdigest()})
                elif case == 'file-missing':
                    del state[file_path]
                elif case == 'file-size':
                    state[file_path] += b'x'
                else:
                    state[file_path] = b'x' * len(state[file_path])
                result = P.recover(P.DoubleFS(state), DEST)
                self.assertFalse(result['valid'], result)


class PersistenceDoubleTests(unittest.TestCase):
    def test_constructor_state_is_durable(self):
        fs = P.DoubleFS({'/parent/empty': None, '/parent/file': b'original'})
        fs.power_loss()
        self.assertTrue(fs.is_dir(PurePosixPath('/parent/empty')))
        self.assertEqual(fs.read_file(PurePosixPath('/parent/file')), b'original')

    def test_file_data_and_entry_need_separate_flushes(self):
        parent, file = PurePosixPath('/parent'), PurePosixPath('/parent/file')
        fs = P.DoubleFS({parent: None})
        fs.create_file(file, b'volatile')
        fs.flush_file(file)
        fs.power_loss()
        self.assertFalse(fs.exists(file))
        fs.create_file(file, b'volatile')
        fs.flush_dir(parent)
        fs.power_loss()
        self.assertEqual(fs.read_file(file), b'')

    def test_unflushed_ancestor_loses_flushed_descendants_recursively(self):
        fs = P.DoubleFS()
        parent, file = PurePosixPath('/parent'), PurePosixPath('/parent/file')
        fs.mkdir(parent)
        fs.create_file(file, b'flushed')
        fs.flush_file(file)
        fs.flush_dir(parent)
        fs.power_loss()
        self.assertFalse(fs.exists(parent))

    def test_unflushed_removal_returns_and_flushed_removal_persists(self):
        parent, file = PurePosixPath('/parent'), PurePosixPath('/parent/file')
        fs = P.DoubleFS({file: b'original'})
        fs.remove(file)
        fs.power_loss()
        self.assertEqual(fs.read_file(file), b'original')
        fs.remove(file)
        fs.flush_dir(parent)
        fs.power_loss()
        self.assertFalse(fs.exists(file))

    def test_move_persists_only_its_entries_and_keeps_only_flushed_data(self):
        source, target = PurePosixPath('/source/file'), PurePosixPath('/target/file')
        fs = P.DoubleFS({source.parent: None, target: b'old'})
        fs.create_file(source, b'unflushed')
        extra = target.parent / 'extra'
        fs.create_file(extra, b'also volatile')
        fs.move_write_through(source, target)
        fs.power_loss()
        self.assertFalse(fs.exists(source))
        self.assertFalse(fs.exists(extra))
        self.assertEqual(fs.read_file(target), b'')

    def test_creation_requires_parent_and_refuses_existing_entries(self):
        fs = P.DoubleFS()
        with self.assertRaises(FileNotFoundError):
            fs.mkdir(PurePosixPath('/missing/child'))
        with self.assertRaises(FileNotFoundError):
            fs.create_file(PurePosixPath('/missing/file'), b'data')
        fs.mkdir(PurePosixPath('/parent'))
        fs.create_file(PurePosixPath('/parent/file'), b'data')
        with self.assertRaises(FileExistsError):
            fs.mkdir(PurePosixPath('/parent'))
        with self.assertRaises(FileExistsError):
            fs.create_file(PurePosixPath('/parent/file'), b'other')


if __name__ == '__main__':
    unittest.main()
