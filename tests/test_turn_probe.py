"""Stdlib probe contract checks and an optional real, isolated 0.4 engine run."""
from __future__ import annotations

import base64
import contextlib
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
import uuid

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('turn_probe', ROOT / 'tools/perf/turn_probe.py')
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)
CANARIES = ('host-canary-1943', 'user-canary-1943', r'C:\private-canary-1943\data',
            '/home/private-canary-1943/data', 'auth-canary-1943-secret-0123456789',
            'launch-canary-1943-secret-0123456789')
DETAIL = ' | '.join(CANARIES)
WORDS = {'0:0:0:1': [], '0:0:1:1': [2, 1], '0:0:2:1': [2, -1],
         '0:1:0:1': [], '0:1:1:1': [2, 1, 1], '0:1:2:1': [2, 1],
         '0:2:0:0': [], '0:2:1:0': [1, 1], '0:2:2:0': [1]}


@contextlib.contextmanager
def scratch_directory(prefix):
    # On Windows, CPython 3.13+ mkdtemp's 0700 ACL excludes the restricted
    # sandbox token. Inherit the workspace ACL, without changing any ACL.
    path = ROOT / 'work' / (prefix + uuid.uuid4().hex)
    path.mkdir()
    try:
        yield path
    finally:
        path.resolve().relative_to((ROOT / 'work').resolve())
        shutil.rmtree(path)


def encoded(values, code):
    return base64.b64encode(b''.join(struct.pack(code, n) for n in values)).decode('ascii')


def native_snapshot(colors, styles=None, interactive=None, *, revision='r0', indices=None,
                    base_revision=None, head=0, state_hash='start'):
    styles = [1] * len(colors) if styles is None else styles
    interactive = [1] * len(colors) if interactive is None else interactive
    result = dict(mode='full' if indices is None else 'delta', revision=revision,
                  colors=encoded(colors, '<H'), styles=encoded(styles, '<B'),
                  interactive=encoded(interactive, '<B'),
                  state=dict(head=head, revision=head, state_hash=state_hash,
                             checkpoints=[{}] * (1 + head // 50)))
    if indices is not None:
        result.update(indices=encoded(indices, '<I'), base_revision=base_revision)
    return result


def timing_row(sequence=1):
    # Inclusive span totals deliberately exceed locked_ns; no subtraction is valid.
    spans = [dict(name=name, depth=depth, start_ns=start, ns=duration, outcome='ok')
             for name, depth, start, duration in (
                 ('command', 0, 10, 20), ('native_reply', 0, 35, 40),
                 ('reply.snapshot', 1, 36, 20), ('reply.detail', 2, 40, 5),
                 ('reply.local', 1, 58, 6), ('reply.native_snapshot', 1, 65, 8))]
    spans[1]['prediction'] = True
    spans[-1]['mode'] = 'delta'
    return dict(sequence=sequence, route=probe.TURN_ROUTE, action='turn', outcome='ok',
                queue_ns=7, lock_wait_ns=10, locked_ns=80, worker_ns=100,
                before=dict(head=sequence - 1, revision=sequence - 1),
                after=dict(head=sequence, revision=sequence), spans=spans)


class TinyModel:
    n = 5

    def __init__(self):
        self.calls = []

    def word_net(self, word):
        self.calls.append(tuple(word))
        labels = list(range(self.n))
        for move in word:
            source, destination = ([0, 1, 2], [1, 2, 0]) if abs(move) == 1 else ([3, 4], [4, 3])
            if move < 0:
                source, destination = destination, source
            previous = [labels[i] for i in source]
            for i, value in zip(destination, previous):
                labels[i] = value
        destination = [i for i, label in enumerate(labels) if i != label]
        return [labels[i] for i in destination], destination


class Clock:
    def __init__(self, oversleep=0):
        self.ns, self.oversleep, self.sleeps = 0, oversleep, []

    def now(self):
        return self.ns

    def sleep(self, seconds):
        self.sleeps.append(seconds)
        self.ns += round(seconds * 1_000_000_000) + self.oversleep


class ScriptedClient:
    def __init__(self, clock, replies):
        self.clock, self.replies, self.calls = clock, iter(replies), []

    def request(self, method, path, body=None):
        self.calls.append((method, path, body))
        self.clock.ns += 5_000_000 if method == 'POST' else 1_000_000
        response = next(self.replies)
        if isinstance(response, BaseException):
            raise response
        return response if isinstance(response, tuple) else (200, probe.encode_json(response))


@contextlib.contextmanager
def stub_run(parent, failure=None, *, words=None, mutate_timing=None, leak=False,
             warmup=1, pairs=2):
    """Exercise the real runner and reconstruction, substituting only engine/HTTP."""
    instances = []
    words = copy.deepcopy(WORDS if words is None else words)
    if failure == 'inverse-not-identity':
        words['0:0:2:1'] = [1]
    if failure == 'no-pair-for-length':
        words = {}

    class Engine:
        def __init__(self, root, data, launch_file, log_file, **kwargs):
            self.data, self.launch_file, self.log_file = data, launch_file, log_file
            self.kwargs, self.close_count, self.turns, self.rows = kwargs, 0, 0, []
            self.timing_path = Path(kwargs['engine_command'][-2])
            self.info = dict(base='http://127.0.0.1:1', token=CANARIES[4], launch_id=CANARIES[5])
            self.cleanup_result = None
            instances.append(self)

        def start(self):
            if failure == 'engine-start-failed':
                raise RuntimeError(DETAIL)
            self.data.mkdir()
            if failure != 'profile-missing':
                (self.data / 'native_profile.json').write_text(
                    json.dumps(dict(profile_sha256='profile', token_words=words)), encoding='utf-8')
            return self

        def close(self):
            self.close_count += 1
            self.cleanup_result = dict(graceful_request=failure != 'no-graceful-request',
                                       forced=failure == 'engine-stop-forced',
                                       exit_code=1 if failure == 'nonzero-exit' else 0)
            header = dict(format='magic600-backend-timing-v1', pid=12345,
                          requests=len(self.rows), dropped=int(failure == 'timing-dropped'))
            rows = copy.deepcopy(self.rows)
            if failure == 'timing-rows-mismatch':
                rows = rows[:-1]
            if mutate_timing:
                mutate_timing(header, rows)
            if failure != 'timing-file-missing':
                self.timing_path.write_text('\n'.join(json.dumps(x) for x in [header, *rows]) + '\n',
                                            encoding='utf-8')
            if failure == 'close-exception':
                raise RuntimeError(DETAIL)

    class Client:
        def __init__(self, info):
            self.engine = instances[-1]
            self.requests = []

        def request(self, method, path, body=None):
            engine = self.engine
            self.requests.append((method, path, body))
            if path == probe.HANDSHAKE_ROUTE:
                if failure == 'handshake-failed':
                    return 400, probe.encode_json(dict(error=DETAIL))
                if failure == 'handshake-timeout':
                    raise TimeoutError(DETAIL)
                return 200, probe.encode_json(dict(job='handshake'))
            if path == '/api/job/handshake':
                if failure == 'handshake-job-error':
                    return 200, probe.encode_json(dict(done=True, error=DETAIL))
                result = dict(matched_stickers=5, matched_generators=1200, profile_sha256='profile')
                if failure == 'handshake-count':
                    result['matched_generators'] = 1199
                return 200, probe.encode_json(dict(done=True, result=result))
            if path == probe.SNAPSHOT_ROUTE:
                if failure == 'snapshot-timeout':
                    raise TimeoutError(DETAIL)
                head = engine.turns
                colors = [1, 0, 2, 3, 4] if head % 2 else list(range(5))
                return 200, probe.encode_json(dict(native_snapshot=native_snapshot(
                    colors, revision=f'r{head}', head=head,
                    state_hash='turned' if head % 2 else 'start')))
            if method == 'POST' and path == probe.COMMAND_ROUTE:
                assert failure == 'protect' and body == dict(action='protect', orbits=[])
                engine.rows.append(dict(timing_row(len(engine.rows) + 1), route=probe.COMMAND_ROUTE,
                                        action='protect'))
                return 200, probe.encode_json(dict(job='protect'))
            if path == '/api/job/protect':
                return 200, probe.encode_json(dict(done=True, result=dict(protected_orbits=[])))
            if method == 'POST' and path == probe.TURN_ROUTE:
                if failure == 'job-timeout':
                    raise TimeoutError(DETAIL)
                if failure == 'turn-rejected':
                    return 409, probe.encode_json(dict(error=DETAIL))
                assert body['action'] == 'turn' and body['destination'] == 'live'
                assert body['tokens'][0] in words and body['profile_sha256'] == 'profile'
                assert body['state_hash'] == ('turned' if engine.turns % 2 else 'start')
                assert body['native_since'] == f'r{engine.turns}'
                engine.turns += 1
                row = timing_row(len(engine.rows) + 1)
                if leak:
                    row['untrusted'] = dict(hostname=CANARIES[0], username=CANARIES[1],
                                            paths=list(CANARIES[2:4]), token=CANARIES[4],
                                            launch_id=CANARIES[5], details=DETAIL)
                engine.rows.append(row)
                return 200, probe.encode_json(dict(job=f'turn-{engine.turns}'))
            if path.startswith('/api/job/turn-'):
                if failure == 'turn-job-error':
                    return 200, probe.encode_json(dict(done=True, error=DETAIL))
                head = engine.turns
                colors = [1, 0, 2, 3, 4] if head % 2 else list(range(5))
                state_hash = 'turned' if head % 2 else 'start'
                if failure == 'hash-mismatch' and head % 2 == 0:
                    state_hash = 'wrong'
                snapshot = native_snapshot(colors if head % 3 == 0 else colors[:2],
                                           indices=None if head % 3 == 0 else [0, 1],
                                           base_revision=f'r{head - 1}', revision=f'r{head}',
                                           head=head, state_hash=state_hash)
                work = dict(protected_orbits=[7]) if failure == 'protect' and head % 2 == 0 else {}
                result = dict(result=dict(committed=True), work=work, local_cell={},
                              phase_inspection=None, native_snapshot=snapshot)
                if failure == 'native-refresh-failed':
                    result = dict(result=dict(committed=True), requires_refresh=True)
                if failure == 'bad-delta':
                    snapshot['base_revision'] = 'wrong'
                return 200, probe.encode_json(dict(done=True, result=result))
            raise AssertionError('Unexpected request')

    output = Path(parent) / ('probe-' + str(len(list(Path(parent).iterdir()))))
    fixture = Mock(return_value=({'format': 'synthetic'}, None))
    load = Mock(return_value=(SimpleNamespace(__version__='stub'), TinyModel, Engine,
                             fixture, lambda value: probe.encode_json(value).decode('utf-8')))
    with (patch.object(probe, 'SLOTS', 5), patch.object(probe, 'load_engine', load),
          patch.object(probe, 'LoopbackClient', Client),
          patch.object(probe, 'private_values', return_value=set(CANARIES)),
          patch.object(probe, 'source_hashes', return_value={'core.py': 'a' * 64}),
          patch.object(probe, 'prepare_data',
                       return_value=dict(profile='basic', word=[], state_hash='start'))):
        report = probe.run_probe(output, [2], warmup, pairs)
    written = json.loads((output / 'report.json').read_text(encoding='utf-8'))
    assert written == report
    yield report, output, instances, load, fixture


class ContractTests(unittest.TestCase):
    def setUp(self):
        # Every scratch write is inside this packet's worktree and is removed.
        temporary = scratch_directory('turn-probe-test-')
        self.parent = temporary.__enter__()
        self.addCleanup(temporary.__exit__, None, None, None)

    def assert_private(self, value):
        text = json.dumps(value)
        for canary in CANARIES:
            self.assertNotIn(canary, text)
            self.assertNotIn(json.dumps(canary)[1:-1], text)
        self.assertNotIn(str(ROOT).replace('\\', '\\\\'), text)

    def test_capacity_and_usage_before_build_or_mkdir(self):
        self.assertEqual(probe.sample_plan([2, 3], 5, 15)['max_timed_requests'], 160)
        self.assertEqual(probe.sample_plan([2], 0, 64)['max_timed_requests'], 256)
        for primitives, warmup, pairs in (([2], 0, 65), ([2, 3], 30, 40), ([], 0, 1),
                                          ([0], 0, 1), ([2], -1, 1), ([2], 0, 0)):
            with self.subTest(primitives=primitives, warmup=warmup, pairs=pairs):
                output = self.parent / 'unbuilt'
                with patch.object(probe, 'load_engine') as load:
                    with self.assertRaises(ValueError):
                        probe.run_probe(output, primitives, warmup, pairs)
                    load.assert_not_called()
                    self.assertFalse(output.exists())
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            probe.main(['--output', str(self.parent / 'unbuilt'), '--pairs', '128', '--primitives', '2'])
        self.assertEqual(error.exception.code, 2)

    def test_existing_output_is_never_touched(self):
        sentinel = self.parent / 'sentinel.txt'
        sentinel.write_text('preserve', encoding='utf-8')
        with patch.object(probe, 'load_engine') as load:
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
                probe.main(['--output', str(self.parent)])
            self.assertEqual(error.exception.code, 2)
            load.assert_not_called()
        self.assertEqual(sentinel.read_text(encoding='utf-8'), 'preserve')

    def test_nearest_rank_and_inclusive_independent_summaries(self):
        self.assertEqual(probe.summarize(list(range(1, 21))),
                         dict(count=20, median=10.5, p95=19, max=20))
        self.assertEqual(probe.summarize([9]), dict(count=1, median=9, p95=9, max=9))
        self.assertEqual(probe.summarize([9, 2, 4]), dict(count=3, median=4, p95=9, max=9))
        with self.assertRaises(ValueError):
            probe.summarize([])
        row = dict(series=0, phase='measured', engine=timing_row(),
                   client=dict(post_ns=20, total_ns=200, final_get_combined_ns=80),
                   reserialize_estimate_ns=60, reply_bytes=900, reply_component_bytes={'work': 300},
                   transmitted_slots=5, array_changed_slots=2, mechanical_moved_slots=5)
        warmup = copy.deepcopy(row)
        warmup.update(phase='warmup', client=dict(post_ns=99999, total_ns=99999))
        another_series = copy.deepcopy(row)
        another_series.update(series=1, client=dict(post_ns=9999))
        metrics = probe.summaries([row, warmup, row, another_series], 2)
        first = metrics[0]['metrics']
        self.assertEqual(first['client.post_ns']['median'], 20)
        self.assertEqual(first['client.total_ns']['median'], 200)
        self.assertEqual(first['engine.worker_ns']['median'], 100)
        self.assertEqual(first['engine.locked_ns']['median'], 80)
        self.assertEqual(first['engine.spans.native_reply.depth_0.occurrence_0.ns']['median'], 40)
        self.assertEqual(first['engine.spans.reply.snapshot.depth_1.occurrence_0.ns']['median'], 20)
        self.assertEqual(first['engine.spans.reply.detail.depth_2.occurrence_0.ns']['median'], 5)
        self.assertTrue(all(metric['count'] == 2 for metric in first.values()))
        self.assertEqual(metrics[1]['metrics']['client.post_ns']['count'], 1)
        self.assertEqual(metrics[1]['metrics']['client.post_ns']['median'], 9999)
        self.assertFalse(any('combined' in key for key in first if not key.startswith('client.final_get')))

    def test_poll_schedule_and_actual_sleeps_and_all_parses(self):
        self.assertEqual([probe.poll_delay_ns(i) // 1_000_000 for i in range(1, 11)],
                         [2, 4, 8, 16, 32, 40, 40, 40, 40, 40])
        clock = Clock(oversleep=3_000_000)
        final = dict(done=True, result={'native_snapshot': {}})
        client = ScriptedClient(clock, [{'job': 'private-job'}, *[{'done': False}] * 7, final])
        original_loads = json.loads

        def parse(raw):
            clock.ns += 500_000
            return original_loads(raw)

        with patch.object(probe.json, 'loads', side_effect=parse):
            reply, envelope, raw, timing = probe.request_job(client, probe.TURN_ROUTE, {},
                                                            'turn-rejected', now=clock.now,
                                                            sleep=clock.sleep)
        requested = [n * 1_000_000 for n in (2, 4, 8, 16, 32, 40, 40)]
        self.assertEqual(timing['post_ns'], 5_000_000)
        self.assertEqual(timing['poll_count'], 8)
        self.assertEqual(timing['requested_sleeps_ns'], requested)
        self.assertEqual(timing['actual_sleeps_ns'], [n + 3_000_000 for n in requested])
        self.assertEqual(timing['non_final_gets_ns'], [1_000_000] * 7)
        self.assertEqual(timing['final_get_combined_ns'], 1_000_000)
        self.assertEqual(timing['json_loads_ns'], 9 * 500_000)
        self.assertEqual(timing['post_json_loads_ns'], 500_000)
        self.assertEqual(timing['poll_json_loads_ns'], [500_000] * 8)
        self.assertEqual(timing['final_json_loads_ns'], 500_000)
        self.assertEqual(timing['total_ns'], 5_000_000 + 8_000_000 + 9 * 500_000
                         + sum(requested) + 7 * 3_000_000)
        self.assertEqual(client.calls[1][0], 'GET')  # No sleep before the first poll.
        self.assertEqual(envelope, final)
        self.assertEqual(raw, probe.encode_json(final))
        self.assertEqual(reply, final['result'])

    def test_job_deadlines_include_late_completed_jobs_and_socket_timeouts(self):
        for replies, oversleep in (([TimeoutError(DETAIL)], 0),
                                    ([{'job': 'j'}, TimeoutError(DETAIL)], 0),
                                    ([{'job': 'j'}, {'done': False}], probe.JOB_TIMEOUT_NS)):
            clock = Clock(oversleep)
            client = ScriptedClient(clock, replies)
            with self.subTest(replies=replies), self.assertRaises(probe.ProbeFailure) as error:
                probe.request_job(client, probe.TURN_ROUTE, {}, 'turn-rejected',
                                  now=clock.now, sleep=clock.sleep)
            self.assertEqual(error.exception.reason, 'job-timeout')
        clock = Clock()
        client = ScriptedClient(clock, [{'job': 'j'}, {'done': True, 'result': {}}])
        request = client.request

        def late(method, path, body=None):
            value = request(method, path, body)
            if method == 'GET':
                clock.ns += probe.JOB_TIMEOUT_NS
            return value

        client.request = late
        with self.assertRaises(probe.ProbeFailure) as error:
            probe.request_job(client, probe.TURN_ROUTE, {}, 'turn-rejected',
                              now=clock.now, sleep=clock.sleep)
        self.assertEqual(error.exception.reason, 'job-timeout')

    def test_malformed_or_rejected_jobs(self):
        cases = [[(409, probe.encode_json({'error': DETAIL}))], [(200, b'not json')],
                 [{'result': {}}], [{'job': 'j'}, {'done': True, 'error': DETAIL}],
                 [{'job': 'j'}, {'done': True}], [{'job': 'j'}, {'done': 'true', 'result': {}}],
                 [{'job': 'j'}, (500, probe.encode_json({'error': DETAIL}))]]
        for replies in cases:
            with self.subTest(replies=replies):
                clock = Clock()
                with self.assertRaises(probe.ProbeFailure) as error:
                    probe.request_job(ScriptedClient(clock, replies), probe.TURN_ROUTE, {},
                                      'turn-rejected', now=clock.now, sleep=clock.sleep)
                self.assertEqual(error.exception.reason, 'turn-rejected')

    def test_fresh_direct_loopback_connections_and_timeout(self):
        connections = [Mock(), Mock()]
        for connection in connections:
            connection.getresponse.return_value.status = 200
            connection.getresponse.return_value.read.return_value = b'{}'
        with patch.object(probe.http.client, 'HTTPConnection', side_effect=connections) as factory:
            client = probe.LoopbackClient(dict(base='http://127.0.0.1:3210', token=CANARIES[4]))
            client.request('POST', probe.TURN_ROUTE, {'action': 'turn'})
            client.request('GET', '/api/job/private')
            self.assertEqual(factory.call_count, 2)
            factory.assert_called_with('127.0.0.1', 3210, timeout=30)
        for connection in connections:
            connection.close.assert_called_once()
            self.assertEqual(connection.request.call_args.kwargs['headers']['X-C600-Token'], CANARIES[4])
        self.assertEqual(connections[0].request.call_args.kwargs['body'], b'{"action":"turn"}')
        with patch.object(probe.http.client, 'HTTPConnection', return_value=connections[0]):
            connections[0].getresponse.side_effect = TimeoutError(DETAIL)
            with self.assertRaises(TimeoutError):
                client.request('GET', '/api/job/private')
            self.assertEqual(connections[0].close.call_count, 2)
        for base in ('https://127.0.0.1:1', 'http://example.com:1', 'http://localhost:1',
                     'http://127.0.0.1:1/a', 'http://user@127.0.0.1:1', 'http://127.0.0.1:1?x=y'):
            with self.subTest(base=base), self.assertRaises(ValueError):
                probe.LoopbackClient(dict(base=base, token='secret'))

    def test_full_delta_reconstruction_and_distinct_slot_counts(self):
        initial = native_snapshot([1, 1, 2, 3], revision='a')
        previous, transmitted, changed = probe.reconstruct(initial, slots=4)
        self.assertEqual((transmitted, changed), (4, 0))
        delta = native_snapshot([1, 9, 3], [1, 1, 0], [1, 0, 1], revision='b',
                                indices=[0, 2, 3], base_revision='a')
        rebuilt, transmitted, changed = probe.reconstruct(delta, previous, slots=4)
        self.assertEqual((transmitted, changed), (3, 2))
        self.assertEqual(rebuilt['colors'], [1, 1, 9, 3])
        self.assertEqual(rebuilt['styles'], [1, 1, 1, 0])
        self.assertEqual(rebuilt['interactive'], [1, 1, 0, 1])
        self.assertEqual(previous['colors'], [1, 1, 2, 3])
        mechanical = len(TinyModel().word_net([2, 1])[0])
        self.assertEqual((transmitted, changed, mechanical), (3, 2, 5))
        full = native_snapshot(rebuilt['colors'], rebuilt['styles'], rebuilt['interactive'], revision='c')
        again, transmitted, changed = probe.reconstruct(full, rebuilt, slots=4)
        self.assertEqual((transmitted, changed), (4, 0))
        self.assertEqual(again['colors'], rebuilt['colors'])
        empty = native_snapshot([], [], [], revision='d', indices=[], base_revision='c')
        _, transmitted, changed = probe.reconstruct(empty, again, slots=4)
        self.assertEqual((transmitted, changed), (0, 0))
        little_endian, _, _ = probe.reconstruct(native_snapshot([1, 256, 599]), slots=3)
        self.assertEqual(little_endian['colors'], [1, 256, 599])

    def test_invalid_arrays_and_delta_bases(self):
        previous, _, _ = probe.reconstruct(native_snapshot([1, 2, 3]), slots=3)
        valid = native_snapshot([9], revision='r1', indices=[1], base_revision='r0')
        corrupt = []
        for field, value in (('base_revision', 'stale'), ('indices', encoded([3], '<I')),
                             ('indices', encoded([1, 1], '<I')), ('colors', 'not-base64'),
                             ('styles', encoded([], '<B')), ('colors', encoded([1], '<B')),
                             ('mode', 'unsupported')):
            value_copy = copy.deepcopy(valid)
            value_copy[field] = value
            corrupt.append(value_copy)
        corrupt.append(native_snapshot([1, 2]))
        for snapshot in corrupt:
            with self.subTest(snapshot=snapshot), self.assertRaises((ValueError, struct.error)):
                probe.reconstruct(snapshot, previous, slots=3)
        with self.assertRaises(ValueError):
            probe.reconstruct(valid, slots=3)

    def test_inverse_keys_selection_and_model_identity(self):
        self.assertEqual(probe.inverse_key('0:0:1:1', WORDS), '0:0:2:1')
        self.assertEqual(probe.inverse_key('0:0:2:1', WORDS), '0:0:1:1')
        self.assertEqual(probe.inverse_key('0:0:0:1', WORDS), '0:0:0:1')
        order_two = {'1:2:0:5': [], '1:2:1:5': [2]}
        self.assertEqual(probe.inverse_key('1:2:1:5', order_two), '1:2:1:5')
        model = TinyModel()
        pairs = probe.select_pairs(WORDS, [2, 3, 2], model)
        self.assertEqual(pairs[0]['forward']['key'], '0:0:1:1')
        self.assertEqual(pairs[1]['forward']['word_length'], 3)
        self.assertEqual(pairs[1]['inverse']['word_length'], 2)
        self.assertEqual(pairs[0]['forward']['mechanical_moved_slots'], 5)
        # Two distinct keys share this word; repeated series reuse each key's count.
        self.assertEqual(model.calls.count((2, 1)), 2)
        self.assertIn((2, 1, 2, -1), model.calls)
        for words, length, reason in ((WORDS, 9, 'no-pair-for-length'),
                                     ({'0:0:1:1': [2, 1], '0:0:2:1': [1]}, 2, 'inverse-not-identity')):
            with self.assertRaises(probe.ProbeFailure) as error:
                probe.select_pairs(words, [length], TinyModel())
            self.assertEqual(error.exception.reason, reason)
        # From a start word, a forward word that would reach Home is skipped.
        self.assertEqual(probe.select_pairs(WORDS, [2], TinyModel(), [-1, 2])[0]['forward']['key'],
                         '0:0:2:1')
        broken = copy.deepcopy(WORDS)
        broken['0:0:2:1'] = [1]
        with self.assertRaises(probe.ProbeFailure) as error:
            probe.select_pairs(broken, [2], TinyModel())
        self.assertEqual(error.exception.reason, 'inverse-not-identity')

    def test_fixed_relations_and_nested_span_proof(self):
        expected = {
            'client': 'POST, poll sleeps, non-final GETs, the final GET and json.loads are '
                      'consecutive intervals inside total. Parsing occurs after the POST '
                      'and each GET; json_loads_ns includes all of those parses.',
            'engine': 'queue precedes worker; worker contains lock_wait and locked; locked '
                      'contains the depth-0 spans; each span at depth d + 1 lies inside '
                      'the enclosing span at depth d, using start_ns and ns.',
            'cross_clock': 'Client and engine intervals overlap in wall time and are never '
                           'added together. Span start_ns is relative to worker start.',
            'final_get': "The final GET contains the server's canonical JSON serialization, "
                         'request handling, scheduling and loopback transport together; '
                         'these costs are not separated.',
            'reserialize_estimate': 'canonical on the parsed final job envelope in the probe '
                                    'process, outside the timed turn; an estimate, never '
                                    "the server's serialization time.",
            'summaries': 'Each metric is summarized independently and inclusively. Parent '
                         'and child intervals are not added or subtracted. Only repeated '
                         'client sleeps, non-final GETs and parses have per-turn totals.',
        }
        self.assertEqual(probe.RELATIONS, expected)
        good = timing_row()
        self.assertGreater(sum(span['ns'] for span in good['spans']), good['locked_ns'])
        before = copy.deepcopy(good)
        probe.check_timing_row(good)
        self.assertEqual(good, before)
        for field, value in (('depth', 4), ('start_ns', 9), ('ns', 50), ('outcome', 'failed')):
            bad = copy.deepcopy(good)
            bad['spans'][2][field] = value
            with self.subTest(field=field), self.assertRaises(probe.ProbeFailure) as error:
                probe.check_timing_row(bad)
            self.assertEqual(error.exception.reason, 'timing-rows-mismatch')

    def test_sanitizer_nested_fields_keys_canaries_and_native_keys(self):
        nested = dict(valid=True, relations=probe.RELATIONS, native_key='0:0:1:1', word=[2, 1],
                      source_sha256={'core.py': 'a' * 64}, route=probe.TURN_ROUTE,
                      untrusted=[dict(hostname=CANARIES[0], user_name=CANARIES[1],
                                      token=CANARIES[4], launch_id=CANARIES[5]),
                                 [f'error: {canary}' for canary in CANARIES]],
                      **{canary: canary for canary in CANARIES})
        cleaned = probe.sanitize(nested, CANARIES)
        self.assert_private(cleaned)
        self.assertEqual(cleaned['native_key'], '0:0:1:1')
        self.assertEqual(cleaned['source_sha256'], {'core.py': 'a' * 64})
        self.assertEqual(cleaned['relations'], probe.RELATIONS)
        self.assertEqual(cleaned['route'], probe.TURN_ROUTE)
        paths = ['C:/not-a-canary/file', r'D:\not-a-canary\file', '/var/not-a-canary/file',
                 'path=/var/not-a-canary/file', 'path:/var/not-a-canary/file',
                 Path('/var/not-a-canary/file'), ROOT, r'\\server\share\file']
        self.assertEqual(probe.sanitize(paths), [])
        self.assertEqual(probe.sanitize({'token': 'unknown-secret', 'launch_id': 'unknown-id'}), {})
        with (patch.object(probe.socket, 'gethostname', return_value=CANARIES[0]),
              patch.object(probe.platform, 'node', return_value=CANARIES[0]),
              patch.object(probe.getpass, 'getuser', return_value=CANARIES[1])):
            self.assertTrue(set(CANARIES[:2]).issubset(probe.private_values()))

    def test_successful_stub_run_rows_sources_shutdown_and_warmups(self):
        with stub_run(self.parent, leak=True) as (report, output, engines, load, fixture):
            self.assertTrue(report['valid'])
            self.assertNotIn('reason', report)
            self.assertEqual(len(report['rows']), 4)
            self.assertEqual(len(report['warmup_rows']), 2)
            self.assertEqual(report['turn_requests'], 6)
            self.assertEqual(report['completed_pairs'], 3)
            self.assertEqual(report['backend_timing'],
                             dict(format='magic600-backend-timing-v1', requests=6, dropped=0))
            self.assertEqual(report['relations'], probe.RELATIONS)
            self.assertEqual(report['scope'], probe.SCOPE)
            self.assertIn('.NET Thread.Sleep', report['limits'][0])
            self.assertIn('15.6 ms', report['limits'][0])
            self.assertIn('checkpoint every 50 turns', report['limits'][1])
            self.assertIn('excluding object keys and separators', report['limits'][2])
            self.assertEqual(engines[0].close_count, 1)
            self.assertTrue(report['engine_stop']['graceful'])
            self.assertEqual(engines[0].kwargs['timeout'], 180)
            self.assertTrue(engines[0].kwargs['hidden_console'])
            command = engines[0].kwargs['engine_command']
            self.assertEqual(command[:4], [sys.executable, '-B', '-c', probe.CHILD_BOOTSTRAP])
            self.assertEqual(Path(command[-2]), output / 'backend-timing.jsonl')
            self.assertEqual(Path(command[-1]), probe.EXPERIMENT / 'engine.py')
            load.assert_called_once()
            fixture.assert_called_once()
            self.assert_private(report)
            self.assertEqual(report['summaries'][0]['metrics']['client.total_ns']['count'], 4)
            for row in report['rows'] + report['warmup_rows']:
                self.assertEqual(row['engine'], probe.sanitize(engines[0].rows[row['sequence'] - 1], CANARIES))
                self.assertEqual(row['mechanical_moved_slots'], 5)
                self.assertEqual(row['array_changed_slots'], 2)
                self.assertEqual(row['transmitted_slots'], 5 if row['native_mode'] == 'full' else 2)
                self.assertEqual(row['engine']['after']['head'], row['head'])
                self.assertEqual(set(row['reply_component_bytes']),
                                 {'result', 'work', 'local_cell', 'phase_inspection', 'native_snapshot'})
                self.assertEqual(row['reply_component_bytes']['work'], 2)
                self.assertEqual(row['reply_component_bytes']['phase_inspection'], 4)
            self.assertEqual([row['head'] for row in report['rows']], [3, 4, 5, 6])
            self.assertEqual([row['pair_restored'] for row in report['rows'] if row['direction'] == 'inverse'],
                             [True, True])

    def test_checkpoint_counts_record_journal_growth(self):
        with stub_run(self.parent, warmup=25, pairs=1) as (report, *_):
            self.assertTrue(report['valid'])
            rows = report['warmup_rows'] + report['rows']
            self.assertEqual(rows[48]['checkpoint_count'], 1)
            self.assertEqual(rows[49]['checkpoint_count'], 2)
            self.assertEqual(report['rows'][-1]['head'], 52)
            self.assertEqual(report['summaries'][0]['metrics']['client.total_ns']['count'], 2)

    def test_decoding_component_sizes_and_reserialization_are_outside_turn(self):
        clock = Clock()
        encode, reconstruct = probe.encode_json, probe.reconstruct

        def serialization(value):
            clock.ns += 1_000_000
            return encode(value)

        def decoding(*args, **kwargs):
            clock.ns += 500_000_000
            return reconstruct(*args, **kwargs)

        with (patch.object(probe.time, 'perf_counter_ns', side_effect=clock.now),
              patch.object(probe, 'encode_json', side_effect=serialization),
              patch.object(probe, 'reconstruct', side_effect=decoding)):
            with stub_run(self.parent) as (report, *_):
                self.assertTrue(report['valid'])
                for row in report['rows'] + report['warmup_rows']:
                    self.assertEqual(row['client']['total_ns'], 2_000_000)
                    self.assertEqual(row['reserialize_estimate_ns'], 1_000_000)

    def test_every_reason_code_runner_path_stops_and_writes_private_failure(self):
        reached = set()
        variants = {reason: reason for reason in probe.REASONS}
        variants.update({'handshake-job-error': 'handshake-failed', 'handshake-count': 'handshake-failed',
                         'profile-missing': 'handshake-failed', 'handshake-timeout': 'job-timeout',
                         'snapshot-timeout': 'job-timeout', 'turn-job-error': 'turn-rejected',
                         'native-refresh-failed': 'turn-rejected', 'bad-delta': 'turn-rejected',
                         'nonzero-exit': 'engine-stop-forced',
                         'close-exception': 'engine-stop-forced', 'timing-file-missing': 'timing-rows-mismatch'})
        for failure, expected in variants.items():
            with self.subTest(failure=failure):
                with stub_run(self.parent, failure, leak=True) as (report, output, engines, *_):
                    self.assertFalse(report['valid'])
                    self.assertEqual(report['reason'], expected)
                    self.assertNotIn('summaries', report)
                    self.assertEqual(engines[0].close_count, 1)
                    self.assert_private(report)
                    self.assertTrue((output / 'probe-error.log').is_file())
                    reached.add(report['reason'])
        self.assertEqual(reached, probe.REASONS)

    def test_auto_protection_is_released_untimed_and_rows_match_engine_sequences(self):
        with stub_run(self.parent, 'protect') as (report, output, engines, *_):
            self.assertTrue(report['valid'], report.get('reason'))
            self.assertEqual((report['turn_requests'], report['timed_jobs']), (6, 9))
            self.assertEqual(report['protection_releases'],
                             [dict(after_sequence=n, engine_sequence=n + n // 2, orbits=[7]) for n in (2, 4, 6)])
            self.assertEqual(report['backend_timing']['requests'], 9)
            rows = sorted(report['rows'] + report['warmup_rows'], key=lambda row: row['sequence'])
            self.assertEqual([row['engine_sequence'] for row in rows], [1, 2, 4, 5, 7, 8])
            for row in rows:
                self.assertEqual((row['engine']['route'], row['engine']['sequence']),
                                 (probe.TURN_ROUTE, row['engine_sequence']))
            self.assertIn('summaries', report)

        def drop_release(header, rows):
            rows[:] = [row for row in rows if row['route'] != probe.COMMAND_ROUTE]
            header['requests'] = len(rows)

        def release_as_turn(header, rows):
            rows[2]['route'], rows[2]['action'] = probe.TURN_ROUTE, 'turn'

        for mutation in (drop_release, release_as_turn):
            with self.subTest(mutation=mutation.__name__):
                with stub_run(self.parent, 'protect', mutate_timing=mutation) as (report, *_):
                    self.assertEqual(report['reason'], 'timing-rows-mismatch')
                    self.assertNotIn('summaries', report)

    def test_parent_pipe_graceful_fallback_is_valid(self):
        with stub_run(self.parent, 'no-graceful-request') as (report, *_):
            self.assertTrue(report['valid'])
            self.assertTrue(report['engine_stop']['graceful'])
            self.assertFalse(report['engine_stop']['forced'])

    def test_timing_integrity_on_current_turn_sequences(self):
        def wrong_sequence(header, rows):
            rows[-1]['sequence'] = 1

        def wrong_route(header, rows):
            rows[-1]['route'] = probe.HANDSHAKE_ROUTE

        def wrong_action(header, rows):
            rows[-1]['action'] = 'inspect'

        def wrong_header_count(header, rows):
            header['requests'] += 1

        def extra_turn(header, rows):
            rows.append(timing_row(len(rows) + 1))
            header['requests'] += 1

        def wrong_outcome(header, rows):
            rows[-1]['outcome'] = 'ValueError'

        for mutation in (wrong_sequence, wrong_route, wrong_action, wrong_header_count,
                         extra_turn, wrong_outcome):
            with self.subTest(mutation=mutation.__name__):
                with stub_run(self.parent, mutate_timing=mutation) as (report, *_):
                    self.assertEqual(report['reason'], 'timing-rows-mismatch')
                    self.assertNotIn('summaries', report)
        path = self.parent / 'malformed.jsonl'
        for text in ('', 'not-json\n', '{}\n', json.dumps(dict(format='wrong', requests=0, dropped=0))):
            path.write_text(text, encoding='utf-8')
            with self.assertRaises(probe.ProbeFailure) as error:
                probe.read_timings(path, 0)
            self.assertEqual(error.exception.reason, 'timing-rows-mismatch')

    def test_engine_import_failure_still_writes_report_and_no_summaries(self):
        with (patch.object(probe, 'load_engine', side_effect=ImportError(DETAIL)),
              patch.object(probe, 'private_values', return_value=set(CANARIES))):
            output = self.parent / 'missing-engine'
            report = probe.run_probe(output, [2], 0, 1)
        self.assertEqual(report['reason'], 'engine-start-failed')
        self.assertNotIn('summaries', report)
        self.assert_private(report)
        self.assertIn(CANARIES[4], (output / 'probe-error.log').read_text(encoding='utf-8'))
        self.assertTrue((output / 'report.json').is_file())

    def test_child_only_profile_environment_and_relative_provenance(self):
        before = os.environ.get('MAGIC600_BACKEND_PROFILE')
        with stub_run(self.parent) as (report, *_):
            self.assertTrue(report['valid'])
        self.assertEqual(os.environ.get('MAGIC600_BACKEND_PROFILE'), before)
        hashes = probe.source_hashes()
        for required in ('tools/perf/turn_probe.py', 'server.py', 'core.py', 'session.py',
                         'native_bridge.py', 'engine_process.py', 'tests/test_native_bridge.py',
                         'work/experiments/magic600-04/engine.py', 'work/experiments/magic600-04/adapter.py'):
            self.assertIn(required, hashes)
        for path, digest in hashes.items():
            self.assertFalse(Path(path).is_absolute())
            self.assertNotIn('\\', path)
            self.assertRegex(digest, r'^[0-9a-f]{64}$')
            self.assertEqual(digest, probe.hashlib.sha256((ROOT / path).read_bytes()).hexdigest())

    def test_bootstrap_sets_profile_only_in_child_and_preserves_engine_arguments(self):
        script = ("import os,runpy,sys,json; "
                  "runpy.run_path=lambda path,run_name: print(json.dumps(dict("
                  "profile=os.environ.get('MAGIC600_BACKEND_PROFILE'),"
                  "path=path,argv=sys.argv[1:]))); exec(" + repr(probe.CHILD_BOOTSTRAP) + ")")
        timing = self.parent / 'backend-timing.jsonl'
        entry = probe.EXPERIMENT / 'engine.py'
        with patch.dict(os.environ, {'MAGIC600_BACKEND_PROFILE': 'caller-value'}):
            result = subprocess.run([sys.executable, '-B', '-c', script, str(timing), str(entry),
                                     '--no-browser', '--port', '0'], cwd=ROOT,
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout),
                             dict(profile=str(timing), path=str(entry), argv=['--no-browser', '--port', '0']))
            self.assertEqual(os.environ['MAGIC600_BACKEND_PROFILE'], 'caller-value')

    def test_missing_engine_skips_normally_and_fails_when_required(self):
        for unavailable in (ModuleNotFoundError('numpy'), ImportError('core'), OSError('engine')):
            with patch.object(probe, 'load_engine', side_effect=unavailable):
                with patch.dict(os.environ, {'C600_REQUIRE_ENGINE': '0'}):
                    with self.assertRaises(unittest.SkipTest):
                        require_engine_environment(self)
                with patch.dict(os.environ, {'C600_REQUIRE_ENGINE': '1'}):
                    with self.assertRaises(AssertionError):
                        require_engine_environment(self)


def require_engine_environment(test):
    try:
        probe.load_engine()
        # Import the engine entry point without running its main guard.
        runpy_spec = importlib.util.spec_from_file_location('probe_engine_import', probe.EXPERIMENT / 'engine.py')
        runpy_spec.loader.exec_module(importlib.util.module_from_spec(runpy_spec))
    except (ImportError, OSError) as error:
        reason = 'Engine environment unavailable: ' + type(error).__name__
        if os.environ.get('C600_REQUIRE_ENGINE') == '1':
            test.fail(reason)
        test.skipTest(reason)


class EngineTests(unittest.TestCase):
    def test_real_command_short_sequence(self):
        require_engine_environment(self)
        with scratch_directory('turn-probe-engine-') as parent:
            output = Path(parent) / 'attribution'
            result = subprocess.run([sys.executable, '-B', str(ROOT / 'tools/perf/turn_probe.py'),
                                     '--output', str(output), '--primitives', '2',
                                     '--warmup', '1', '--pairs', '2'], cwd=ROOT,
                                    capture_output=True, text=True, timeout=420)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            report = json.loads((output / 'report.json').read_text(encoding='utf-8'))
            self.assertTrue(report['valid'])
            self.assertEqual(report['scope'], probe.SCOPE)
            self.assertEqual(report['relations'], probe.RELATIONS)
            self.assertEqual((len(report['warmup_rows']), len(report['rows'])), (2, 4))
            self.assertEqual(report['backend_timing']['requests'], 6 + len(report['protection_releases']))
            self.assertEqual(report['backend_timing']['dropped'], 0)
            self.assertEqual(report['fixture']['profile'], 'basic')
            self.assertEqual(report['starting_state_hash'], report['fixture']['state_hash'])
            self.assertTrue(report['engine_stop']['graceful'])
            self.assertFalse(report['engine_stop']['forced'])
            self.assertNotIn('reason', report)
            token = report['series'][0]
            self.assertEqual(token['forward']['word_length'], 2)
            profile = json.loads((output / 'data/native_profile.json').read_text(encoding='utf-8'))
            numpy, Model, *_ = probe.load_engine()
            model = Model()
            for direction in ('forward', 'inverse'):
                native = token[direction]
                self.assertEqual(profile['token_words'][native['key']], native['word'])
                self.assertEqual(native['mechanical_moved_slots'], len(model.word_net(native['word'])[0]))
            self.assertEqual(len(model.word_net(token['forward']['word'] + token['inverse']['word'])[0]), 0)
            for row in report['warmup_rows'] + report['rows']:
                self.assertEqual(row['engine']['route'], probe.TURN_ROUTE)
                self.assertEqual(row['engine']['action'], 'turn')
                probe.check_timing_row(row['engine'])
                self.assertEqual(row['client']['poll_count'], len(row['client']['non_final_gets_ns']) + 1)
                self.assertEqual(row['client']['requested_sleeps_ns'],
                                 [probe.poll_delay_ns(i) for i in range(1, row['client']['poll_count'])])
                self.assertEqual(len(row['client']['actual_sleeps_ns']), row['client']['poll_count'] - 1)
                self.assertGreaterEqual(row['client']['total_ns'], row['client']['final_get_combined_ns'])
                self.assertGreater(row['reply_bytes'], row['reply_component_bytes']['native_snapshot'])
                self.assertGreater(row['reserialize_estimate_ns'], 0)
                self.assertLessEqual(row['array_changed_slots'], row['transmitted_slots'])
                self.assertLessEqual(row['transmitted_slots'], 259800)
                if row['native_mode'] == 'full':
                    self.assertEqual(row['transmitted_slots'], 259800)
                else:
                    self.assertEqual(row['native_mode'], 'delta')
                self.assertEqual(row['mechanical_moved_slots'], token[row['direction']]['mechanical_moved_slots'])
                self.assertEqual(row['head'], row['engine']['after']['head'])
                self.assertGreaterEqual(row['checkpoint_count'], 1)
                if row['direction'] == 'inverse':
                    self.assertEqual(row['state_hash'], report['starting_state_hash'])
                    self.assertTrue(row['pair_restored'])
            self.assertEqual(report['summaries'][0]['metrics']['client.total_ns']['count'], 4)
            self.assertEqual(report['environment']['numpy'], numpy.__version__)
            lifecycle = json.loads((output / 'launch.lifecycle.json').read_text(encoding='utf-8'))
            forbidden = set(probe.private_values()) | {lifecycle['executable'], str(output), str(ROOT)}
            public_text = json.dumps(report)
            for private in forbidden:
                self.assertNotIn(json.dumps(private)[1:-1], public_text)
            self.assertNotIn('launch_id', public_text)
            self.assertNotIn('"token"', public_text)


if __name__ == '__main__':
    unittest.main(verbosity=2)
