"""Headless attribution of the retained 0.4 native turn route, never B4-12 evidence."""
from __future__ import annotations

import argparse
import ast
import base64
import getpass
import hashlib
import http.client
import json
import math
import os
from pathlib import Path
import platform
import re
import runpy
import socket
import statistics
import struct
import sys
import time
import traceback
from urllib.parse import urlsplit

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
EXPERIMENT = ROOT / 'work/experiments/magic600-04'
TURN_ROUTE = '/api/experiment/native-input'
HANDSHAKE_ROUTE = '/api/native/handshake'
SNAPSHOT_ROUTE = '/api/experiment/native-snapshot'
COMMAND_ROUTE = '/api/experiment/command'
SLOTS = 259800
CAPACITY = 256
REQUEST_TIMEOUT = 30
JOB_TIMEOUT_NS = 120_000_000_000
REASONS = frozenset((
    'engine-start-failed', 'handshake-failed', 'no-pair-for-length',
    'inverse-not-identity', 'turn-rejected', 'job-timeout', 'hash-mismatch',
    'engine-stop-forced', 'timing-rows-mismatch', 'timing-dropped'))
SCOPE = ("Headless attribution of the 0.4 engine's native turn path over loopback HTTP "
         "with a synthetic native enumeration. Not B4-12 evidence: no native host, "
         "input, rendering, GPU or .NET parsing.")
RELATIONS = {
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
LIMITS = [
    "Python time.sleep resolution differs from .NET Thread.Sleep. The default "
    "Windows timer can stretch a 2 ms sleep to about 15.6 ms; the host's actual "
    "poll delay needs the Windows harness.",
    'Journal growth and the automatic checkpoint every 50 turns affect later '
    'turns. Head and checkpoint count are recorded per row.',
    'reply_bytes counts the final job envelope. reply_component_bytes counts '
    'UTF-8 canonical JSON values of the top-level native reply components, '
    'excluding object keys and separators.',
    'The run starts from the B4-12 basic fixture (Home plus one fixed word), so '
    'no pair reaches Home. 0.4 protects an orbit that a turn completes; after '
    'such a turn the probe releases that protection with an untimed command and '
    'refreshes the native arrays, as the 0.4 latency harness does.',
]
# EngineProcess copies its environment. Set profiling inside the child interpreter
# so the calling process and any other engine launches retain their environment.
CHILD_BOOTSTRAP = ("import os,runpy,sys; "
                   "os.environ['MAGIC600_BACKEND_PROFILE']=sys.argv.pop(1); "
                   "runpy.run_path(sys.argv.pop(1),run_name='__main__')")


class ProbeFailure(Exception):
    def __init__(self, reason):
        if reason not in REASONS:
            raise ValueError('Unknown probe reason')
        self.reason = reason
        super().__init__(reason)


def sample_plan(primitives, warmup, pairs):
    if not primitives or any(type(n) is not int or n < 1 for n in primitives):
        raise ValueError('primitives must be positive word lengths')
    if type(warmup) is not int or warmup < 0 or type(pairs) is not int or pairs < 1:
        raise ValueError('warmup must be nonnegative and pairs must be positive')
    turns = 2 * len(primitives) * (warmup + pairs)
    # The handshake is not a timed job. Each turn may be followed by one timed
    # protection release, so the worst case is two timed jobs per turn.
    if 2 * turns > CAPACITY:
        raise ValueError('planned requests exceed the 256-row timer capacity')
    return dict(primitives=list(primitives), warmup_pairs=warmup,
                measured_pairs=pairs, turn_requests=turns,
                max_protection_releases=turns, max_timed_requests=2 * turns,
                timer_capacity=CAPACITY)


def encode_json(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'),
                      allow_nan=False).encode('utf-8')


def poll_delay_ns(non_final_count):
    return min(40, 2 ** min(non_final_count, 6)) * 1_000_000


class LoopbackClient:
    def __init__(self, info):
        base = urlsplit(info['base'])
        if (base.scheme != 'http' or base.hostname != '127.0.0.1'
                or base.username or base.password or base.path not in ('', '/')
                or base.query or base.fragment or not base.port):
            raise ValueError('Expected a numeric loopback engine address')
        self.port, self.secret = base.port, info['token']

    def request(self, method, path, body=None):
        connection = http.client.HTTPConnection('127.0.0.1', self.port,
                                                timeout=REQUEST_TIMEOUT)
        try:
            payload = None if body is None else encode_json(body)
            connection.request(method, path, body=payload,
                               headers={'X-C600-Token': self.secret,
                                        'Content-Type': 'application/json'})
            response = connection.getresponse()
            return response.status, response.read()
        finally:
            connection.close()


def request_job(client, path, body, rejection_reason, *, now=None, sleep=None):
    """Time only POST/poll/parse. Decoding and re-serialization belong to the caller."""
    now, sleep = now or time.perf_counter_ns, sleep or time.sleep
    start = now()
    deadline = start + JOB_TIMEOUT_NS
    requested, actual, non_final, parses = [], [], [], []

    def request(method, route, payload=None):
        before = now()
        if before >= deadline:
            raise ProbeFailure('job-timeout')
        try:
            status, raw = client.request(method, route, payload)
        except TimeoutError as error:
            raise ProbeFailure('job-timeout') from error
        elapsed = now() - before
        if now() >= deadline:
            raise ProbeFailure('job-timeout')
        before_parse = now()
        try:
            value = json.loads(raw)
        except (ValueError, UnicodeError) as error:
            raise ProbeFailure(rejection_reason) from error
        parse_ns = now() - before_parse
        parses.append(parse_ns)
        if now() >= deadline:
            raise ProbeFailure('job-timeout')
        if status != 200 or not isinstance(value, dict) or 'error' in value:
            detail = value.get('error', 'Invalid job response') if isinstance(value, dict) else 'Invalid job response'
            raise ProbeFailure(rejection_reason) from ValueError(f'HTTP {status}: {detail}')
        return value, raw, elapsed

    posted, _, post_ns = request('POST', path, body)
    if not isinstance(posted.get('job'), str) or not posted['job']:
        raise ProbeFailure(rejection_reason)
    job_path = '/api/job/' + posted['job']
    while True:
        envelope, raw, get_ns = request('GET', job_path)
        if envelope.get('done') is True:
            total_ns = now() - start
            if not isinstance(envelope.get('result'), dict):
                raise ProbeFailure(rejection_reason)
            timing = dict(post_ns=post_ns, poll_count=len(non_final) + 1,
                          requested_sleeps_ns=requested, actual_sleeps_ns=actual,
                          non_final_gets_ns=non_final, final_get_combined_ns=get_ns,
                          post_json_loads_ns=parses[0], poll_json_loads_ns=parses[1:],
                          final_json_loads_ns=parses[-1], json_loads_ns=sum(parses),
                          requested_sleep_ns=sum(requested), actual_sleep_ns=sum(actual),
                          non_final_get_ns=sum(non_final), total_ns=total_ns)
            return envelope['result'], envelope, raw, timing
        if envelope.get('done') is not False:
            raise ProbeFailure(rejection_reason)
        non_final.append(get_ns)
        delay = poll_delay_ns(len(non_final))
        requested.append(delay)
        before_sleep = now()
        sleep(delay / 1_000_000_000)
        actual.append(now() - before_sleep)


def inverse_key(key, token_words):
    axis, twist, angle, mask = map(int, key.split(':'))
    angles = {int(k.split(':')[2]) for k in token_words
              if tuple(map(int, k.split(':')))[:2] == (axis, twist)
              and int(k.split(':')[3]) == mask}
    order = len(angles)
    if angles != set(range(order)) or not order:
        raise ProbeFailure('inverse-not-identity')
    return f'{axis}:{twist}:{(order - angle) % order}:{mask}'


def select_pairs(token_words, primitives, model, start_word=()):
    selected = []
    counts = {}

    def leaves_home(key):
        # A forward turn that reached Home would start the 0.4 completion flow.
        return not start_word or len(model.word_net(list(start_word) + token_words[key])[0]) > 0

    for length in primitives:
        key = next((key for key in sorted(token_words)
                    if int(key.split(':')[2]) != 0 and int(key.split(':')[3]) != 0
                    and len(token_words[key]) == length and leaves_home(key)), None)
        if key is None:
            raise ProbeFailure('no-pair-for-length')
        inverse = inverse_key(key, token_words)
        word, reverse_word = token_words[key], token_words[inverse]
        # core.Model.word_net applies chronological source -> destination moves to
        # all labels, then returns exactly the non-identity support.
        if len(model.word_net(word + reverse_word)[0]):
            raise ProbeFailure('inverse-not-identity')
        directions = {}
        for direction, native_key in (('forward', key), ('inverse', inverse)):
            word = token_words[native_key]
            if native_key not in counts:
                counts[native_key] = len(model.word_net(word)[0])
            directions[direction] = dict(key=native_key, word=word, word_length=len(word),
                                         mechanical_moved_slots=counts[native_key])
        selected.append(dict(requested_word_length=length, **directions))
    return selected


def decode_array(value, code):
    return [item[0] for item in struct.iter_unpack(code, base64.b64decode(value, validate=True))]


def reconstruct(snapshot, predecessor=None, slots=SLOTS):
    values = {name: decode_array(snapshot[name], code) for name, code in
              (('colors', '<H'), ('styles', '<B'), ('interactive', '<B'))}
    mode = snapshot['mode']
    if mode == 'full':
        if any(len(a) != slots for a in values.values()):
            raise ValueError('Wrong full native array size')
        transmitted = slots
    elif mode == 'delta':
        if predecessor is None or snapshot['base_revision'] != predecessor['revision']:
            raise ValueError('Wrong native delta predecessor')
        indices = decode_array(snapshot['indices'], '<I')
        if (any(i < 0 or i >= slots for i in indices)
                or any(a >= b for a, b in zip(indices, indices[1:]))
                or any(len(a) != len(indices) for a in values.values())):
            raise ValueError('Invalid native delta arrays')
        transmitted = len(indices)
        for name, delta in values.items():
            full = predecessor[name].copy()
            for index, value in zip(indices, delta):
                full[index] = value
            values[name] = full
    else:
        raise ValueError('Unsupported native reply mode')
    changed = 0 if predecessor is None else sum(
        any(values[name][i] != predecessor[name][i] for name in values)
        for i in range(slots))
    return dict(revision=snapshot['revision'], **values), transmitted, changed


def summarize(values):
    ordered = sorted(values)
    if not ordered:
        raise ValueError('Cannot summarize an empty sample')
    return dict(count=len(ordered), median=statistics.median(ordered),
                p95=ordered[math.ceil(.95 * len(ordered)) - 1], max=ordered[-1])


def summaries(rows, series_count):
    result = []
    for series in range(series_count):
        metrics = {}
        for row in rows:
            if row['series'] != series or row['phase'] != 'measured':
                continue
            scalars = {f'client.{key}': value for key, value in row['client'].items()
                       if type(value) is int}
            scalars.update({f'engine.{key}': row['engine'][key] for key in
                            ('queue_ns', 'lock_wait_ns', 'locked_ns', 'worker_ns')})
            occurrences = {}
            for span in row['engine']['spans']:
                name = f"engine.spans.{span['name']}.depth_{span['depth']}"
                occurrence = occurrences.get(name, 0)
                occurrences[name] = occurrence + 1
                scalars[f'{name}.occurrence_{occurrence}.ns'] = span['ns']
            scalars.update({key: row[key] for key in
                            ('reserialize_estimate_ns', 'reply_bytes', 'transmitted_slots',
                             'array_changed_slots', 'mechanical_moved_slots')})
            scalars.update({f'reply_component_bytes.{key}': value
                            for key, value in row['reply_component_bytes'].items()})
            for metric, value in scalars.items():
                metrics.setdefault(metric, []).append(value)
        result.append(dict(series=series, metrics={key: summarize(values)
                                                  for key, values in sorted(metrics.items())}))
    return result


def check_timing_row(row):
    try:
        numbers = [row[name] for name in ('queue_ns', 'lock_wait_ns', 'locked_ns', 'worker_ns')]
        if any(type(n) is not int or n < 0 for n in numbers) or row['outcome'] != 'ok':
            raise ValueError('Invalid worker timing')
        begin, end = row['lock_wait_ns'], row['lock_wait_ns'] + row['locked_ns']
        if end > row['worker_ns'] or not row['spans']:
            raise ValueError('Invalid locked interval')
        parents = []
        for span in row['spans']:
            depth, start, duration = span['depth'], span['start_ns'], span['ns']
            if (any(type(n) is not int or n < 0 for n in (depth, start, duration))
                    or span['outcome'] != 'ok' or not isinstance(span['name'], str)):
                raise ValueError('Invalid span')
            parents = parents[:depth]
            outer_start, outer_end = parents[-1] if parents else (begin, end)
            if depth != len(parents) or not outer_start <= start <= start + duration <= outer_end:
                raise ValueError('Invalid span containment')
            parents.append((start, start + duration))
        for phase in ('before', 'after'):
            if any(type(row[phase][k]) is not int for k in ('head', 'revision')):
                raise ValueError('Invalid timing state')
    except (KeyError, TypeError, ValueError) as error:
        raise ProbeFailure('timing-rows-mismatch') from error


def read_timings(path, turn_sequences, release_sequences=()):
    """Return the turn rows. Every timed job must be a planned turn or protection
    release, at the engine sequence the probe recorded for it."""
    try:
        if isinstance(turn_sequences, int):
            turn_sequences = range(1, turn_sequences + 1)
        turns, releases = set(turn_sequences), set(release_sequences)
        with path.open(encoding='utf-8') as stream:
            records = [json.loads(line) for line in stream]
        header, rows = records[0], records[1:]
        if header['format'] != 'magic600-backend-timing-v1':
            raise ValueError('Wrong timing format')
        if header['dropped'] != 0:
            raise ProbeFailure('timing-dropped')
        if (turns & releases or header['requests'] != len(rows)
                or len(rows) != len(turns) + len(releases)):
            raise ValueError('Wrong timing row count')
        rows.sort(key=lambda row: row['sequence'])
        for sequence, row in enumerate(rows, 1):
            expected = ((TURN_ROUTE, 'turn') if sequence in turns else
                        (COMMAND_ROUTE, 'protect') if sequence in releases else None)
            if row['sequence'] != sequence or (row['route'], row['action']) != expected:
                raise ValueError('Wrong timing row identity')
            check_timing_row(row)
        rows = [row for row in rows if row['sequence'] in turns]
        return {key: header[key] for key in ('format', 'requests', 'dropped')}, rows
    except ProbeFailure:
        raise
    except (OSError, ValueError, KeyError, IndexError, TypeError) as error:
        raise ProbeFailure('timing-rows-mismatch') from error


def private_values():
    names = [socket.gethostname(), platform.node(), getpass.getuser()]
    names.extend(os.environ.get(key, '') for key in ('USERNAME', 'USER', 'COMPUTERNAME', 'HOSTNAME'))
    return {name for name in names if name}


_DROP = object()
PRIVATE_FIELDS = frozenset(('token', 'launch_id', 'x-c600-token', 'hostname', 'host_name',
                            'host', 'username', 'user_name', 'user', 'pid', 'url', 'base'))
ABSOLUTE_PATH = re.compile(r'''(?i)(?:[a-z]:[\\/]|\\\\|(?:^|[\s"'=(:,\[])/[^\s])''')


def sanitize(value, secrets=()):
    """Drop private strings/fields recursively, including keys; keep fixed API routes."""
    secrets = tuple(secret.casefold() for secret in secrets if secret)

    def clean(item):
        if isinstance(item, Path):
            return _DROP if item.is_absolute() else clean(item.as_posix())
        if isinstance(item, str):
            if any(secret in item.casefold() for secret in secrets):
                return _DROP
            if (item not in (TURN_ROUTE, HANDSHAKE_ROUTE, SNAPSHOT_ROUTE, COMMAND_ROUTE)
                    and ABSOLUTE_PATH.search(item)):
                return _DROP
            return item
        if isinstance(item, dict):
            result = {}
            for key, child in item.items():
                if not isinstance(key, str) or key.casefold() in PRIVATE_FIELDS:
                    continue
                safe_key, safe_child = clean(key), clean(child)
                if safe_key is not _DROP and safe_child is not _DROP:
                    result[safe_key] = safe_child
            return result
        if isinstance(item, (list, tuple)):
            return [safe for child in item if (safe := clean(child)) is not _DROP]
        return item

    safe = clean(value)
    return None if safe is _DROP else safe


def source_hashes():
    # The engine installs adapter via AST at runtime, so both are explicit roots.
    pending = [EXPERIMENT / 'engine.py', EXPERIMENT / 'adapter.py', ROOT / 'server.py',
               ROOT / 'engine_process.py', ROOT / 'tests/test_native_bridge.py', Path(__file__)]
    found = {}
    while pending:
        path = pending.pop().resolve()
        name = path.relative_to(ROOT).as_posix()
        if name in found:
            continue
        raw = path.read_bytes()
        found[name] = hashlib.sha256(raw).hexdigest()
        for node in ast.walk(ast.parse(raw.decode('utf-8-sig'))):
            modules = ([item.name for item in node.names] if isinstance(node, ast.Import)
                       else [node.module] if isinstance(node, ast.ImportFrom) and node.module else [])
            for module in modules:
                relative = Path(*module.split('.')).with_suffix('.py')
                for directory in (EXPERIMENT, ROOT):
                    dependency = directory / relative
                    if dependency.is_file():
                        pending.append(dependency)
                        break
    for relative in ('assets/manifest.json',
                     'work/experiments/magic600-04/evidence/orbit-invariants-20260916-generators.json'):
        found[relative] = hashlib.sha256((ROOT / relative).read_bytes()).hexdigest()
    return dict(sorted(found.items()))


def load_engine():
    sys.path.insert(0, str(ROOT))
    import numpy
    from core import Model, canonical
    from engine_process import EngineProcess
    make_fixture = runpy.run_path(str(ROOT / 'tests/test_native_bridge.py'))['make_fixture']
    return numpy, Model, EngineProcess, make_fixture, canonical


def fixture_module():
    sys.path.insert(0, str(ROOT / 'tools/perf'))
    import b412_fixture
    return b412_fixture


def prepare_data(output):
    """Start from the B4-12 basic fixture, a non-Home state, as the native harness does."""
    b412_fixture = fixture_module()
    record = b412_fixture.build_fixture('basic', output / 'fixture')
    b412_fixture.copy_fixture(output / 'fixture', output / 'data')
    return dict(profile=record['profile'], word=record['word'], state_hash=record['state_hash'])


def run_probe(output, primitives=(2, 3), warmup=5, pairs=15):
    plan = sample_plan(primitives, warmup, pairs)  # Before any mkdir, import or engine work.
    output = fixture_module().guard_path(output)  # A default data directory is refused before mkdir.
    output.mkdir()  # Exclusive creation; an existing directory is a usage error.
    (output / 'probe-error.log').touch()
    report = dict(scope=SCOPE, sample_plan=plan, relations=RELATIONS, limits=LIMITS,
                  environment=dict(python=platform.python_version(), numpy=None,
                                   system=platform.system(), machine=platform.machine()),
                  source_sha256={}, series=[], rows=[], warmup_rows=[], turn_requests=0,
                  timed_jobs=0, protection_releases=[], completed_pairs=0, valid=False)
    engine, reason, stage = None, None, 'engine-start-failed'
    secrets = private_values()
    timing_path = output / 'backend-timing.jsonl'

    def log_error():
        with (output / 'probe-error.log').open('a', encoding='utf-8') as stream:
            traceback.print_exc(file=stream)

    try:
        report['source_sha256'] = source_hashes()
        numpy, Model, EngineProcess, make_fixture, canonical = load_engine()
        report['environment']['numpy'] = numpy.__version__
        model = Model()
        export, _ = make_fixture(model)
        report['fixture'] = prepare_data(output)
        engine = EngineProcess(ROOT, output / 'data', output / 'launch.json', output / 'engine.log',
                               engine_command=[sys.executable, '-B', '-c', CHILD_BOOTSTRAP,
                                               str(timing_path), str(EXPERIMENT / 'engine.py')],
                               hidden_console=True, timeout=180)
        engine.start()
        secrets.update(engine.info[key] for key in ('token', 'launch_id'))
        client = LoopbackClient(engine.info)
        stage = 'handshake-failed'
        handshake, _, _, _ = request_job(client, HANDSHAKE_ROUTE, export, stage)
        profile = json.loads((output / 'data/native_profile.json').read_text(encoding='utf-8'))
        if (handshake.get('matched_stickers') != SLOTS or handshake.get('matched_generators') != 1200
                or handshake.get('profile_sha256') != profile['profile_sha256']):
            raise ProbeFailure('handshake-failed')
        report['series'] = select_pairs(profile['token_words'], primitives, model,
                                        report['fixture']['word'])
        stage = 'turn-rejected'

        def read_snapshot():
            try:
                status, raw = client.request('GET', SNAPSHOT_ROUTE)
            except TimeoutError as error:
                raise ProbeFailure('job-timeout') from error
            if status != 200:
                raise ProbeFailure(stage)
            snapshot = json.loads(raw)['native_snapshot']
            return snapshot, reconstruct(snapshot, slots=SLOTS)[0]

        initial, arrays = read_snapshot()
        state_hash = starting_hash = initial['state']['state_hash']
        report['starting_state_hash'] = starting_hash
        if starting_hash != report['fixture']['state_hash']:
            raise ProbeFailure('hash-mismatch')
        for series, token_pair in enumerate(report['series']):
            for pair in range(warmup + pairs):
                phase = 'warmup' if pair < warmup else 'measured'
                for direction in ('forward', 'inverse'):
                    native = token_pair[direction]
                    body = dict(action='turn', tokens=[native['key']], state_hash=state_hash,
                                profile_sha256=profile['profile_sha256'], destination='live',
                                native_since=arrays['revision'])
                    report['turn_requests'] += 1
                    report['timed_jobs'] += 1
                    reply, envelope, raw, timing = request_job(client, TURN_ROUTE, body, stage)
                    row = dict(sequence=report['turn_requests'], engine_sequence=report['timed_jobs'],
                               series=series, phase=phase,
                               pair=pair + 1 if phase == 'warmup' else pair - warmup + 1,
                               direction=direction, client=timing, reply_bytes=len(raw),
                               mechanical_moved_slots=native['mechanical_moved_slots'])
                    report['warmup_rows' if phase == 'warmup' else 'rows'].append(row)
                    # All remaining work is outside client.total_ns.
                    before = time.perf_counter_ns()
                    canonical(envelope)
                    row['reserialize_estimate_ns'] = time.perf_counter_ns() - before
                    row['reply_component_bytes'] = {key: len(canonical(value).encode('utf-8'))
                                                    for key, value in reply.items()}
                    snapshot = reply['native_snapshot']
                    arrays, transmitted, changed = reconstruct(snapshot, arrays, slots=SLOTS)
                    state = snapshot['state']
                    state_hash = state['state_hash']
                    row.update(native_mode=snapshot['mode'], transmitted_slots=transmitted,
                               array_changed_slots=changed, head=state['head'],
                               revision=state['revision'], native_revision=snapshot['revision'],
                               state_hash=state_hash, checkpoint_count=len(state['checkpoints']))
                    protected = (reply.get('work') or {}).get('protected_orbits') or []
                    if protected:
                        # Untimed setup between turns, as in the 0.4 latency harness.
                        report['timed_jobs'] += 1
                        request_job(client, COMMAND_ROUTE, dict(action='protect', orbits=[]), stage)
                        report['protection_releases'].append(dict(
                            after_sequence=row['sequence'], engine_sequence=report['timed_jobs'],
                            orbits=list(protected)))
                        refreshed, arrays = read_snapshot()
                        if refreshed['state']['state_hash'] != state_hash:
                            raise ProbeFailure('hash-mismatch')
                row['pair_restored'] = state_hash == starting_hash
                if not row['pair_restored']:
                    raise ProbeFailure('hash-mismatch')
                report['completed_pairs'] += 1
    except BaseException as error:
        reason = error.reason if isinstance(error, ProbeFailure) else stage
        log_error()
    finally:
        if engine is not None:
            try:
                engine.close()
                cleanup = engine.cleanup_result or {}
                # EngineProcess also shuts down gracefully through parent-pipe EOF
                # when the authenticated shutdown reply is unavailable.
                graceful = cleanup.get('forced') is False and cleanup.get('exit_code') == 0
                report['engine_stop'] = dict(graceful=graceful, forced=cleanup.get('forced') is True)
                if not graceful:
                    reason = reason or 'engine-stop-forced'
                    with (output / 'probe-error.log').open('a', encoding='utf-8') as stream:
                        stream.write('Engine cleanup: ' + repr(cleanup) + '\n')
            except BaseException:
                reason = reason or 'engine-stop-forced'
                log_error()
        try:
            all_rows = sorted(report['rows'] + report['warmup_rows'], key=lambda row: row['sequence'])
            header, backend = read_timings(
                timing_path, [row['engine_sequence'] for row in all_rows],
                [release['engine_sequence'] for release in report['protection_releases']])
            report['backend_timing'] = header
            if len(all_rows) != len(backend):
                raise ProbeFailure('timing-rows-mismatch')
            for row, engine_row in zip(all_rows, backend):
                if row['engine_sequence'] != engine_row['sequence']:
                    raise ProbeFailure('timing-rows-mismatch')
                row['engine'] = engine_row
        except BaseException as error:
            reason = reason or (error.reason if isinstance(error, ProbeFailure) else 'timing-rows-mismatch')
            log_error()
        if reason is None and report['turn_requests'] != plan['turn_requests']:
            reason = 'timing-rows-mismatch'
        if reason is None:
            try:
                report['summaries'] = summaries(report['rows'], len(report['series']))
            except BaseException:
                reason = 'timing-rows-mismatch'
                log_error()
        report['valid'] = reason is None
        if reason is not None:
            report['reason'] = reason
        report = sanitize(report, secrets)
        (output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n',
                                           encoding='utf-8')
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--primitives', nargs='+', type=int, default=[2, 3])
    parser.add_argument('--warmup', type=int, default=5)
    parser.add_argument('--pairs', type=int, default=15)
    args = parser.parse_args(argv)
    try:
        report = run_probe(args.output, args.primitives, args.warmup, args.pairs)
    except ValueError as error:
        parser.error(str(error))
    except OSError:
        parser.error('output must be a new directory with an existing writable parent')
    print('Valid headless attribution report.' if report['valid'] else
          'Invalid headless attribution report: ' + report['reason'])
    return 0 if report['valid'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
