"""Combined HD2P/1 gate: unchanged suite plus segmented tests, 550 outer wall seconds.

No trajectory or qualification runs on development tables. Real work is bounded
WORD36 and the two letters of the recorded 7306 first step. All writes are temp.
"""
import ast
import copy
from contextlib import contextmanager, ExitStack, redirect_stdout
import io
import os
from pathlib import Path
from types import SimpleNamespace
import signal
import subprocess
import sys
import time
import unittest
from unittest.mock import patch

START = time.monotonic()
sys.dont_write_bytecode = True
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[name] = '1'
HERE = Path(__file__).resolve().parent


def combined_gate():
    for script, args in ((HERE / 'test_height_descent_experiment.py', ['--worker']),
                         (Path(__file__).resolve(), ['--worker'])):
        child = subprocess.Popen([sys.executable, '-I', '-B', str(script), *args], start_new_session=True)
        try:
            code = child.wait(timeout=max(.01, 549 - (time.monotonic() - START)))
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
            print('FAILED: combined acceptance exceeded 550 wall seconds', flush=True)
            return 1
        if code:
            print(f'FAILED: {script.name}; combined wall seconds {time.monotonic() - START:.3f}', flush=True)
            return 1
    elapsed = time.monotonic() - START
    print(f'Combined acceptance passed; wall seconds: {elapsed:.3f} (ceiling 550)', flush=True)
    return 0 if elapsed < 550 else 1


if __name__ == '__main__' and len(sys.argv) == 1:
    sys.exit(combined_gate())

sys.path.insert(0, str(HERE))
# Reuse only the explicit transition oracle and literal fixture from the unchanged
# tests. Its audit hook confines this worker's writes to Legacy.OUT.
import test_height_descent_experiment as L
import height_descent_experiment as E
import check_height_descent_experiment as C
OUT = L.OUT


def oracle(kind='pair', full=False):
    o = L.Oracle()
    o.tick = lambda: None
    if full:
        o.items, o.jumble = tuple(range(1, 60)), tuple(range(12, 60))
    if kind == 'pair':
        o.node('middle', blocked={3: {11}})
        o.node('end', {'2': 12, '4': 3})
        o.edge('s', 0, 1, 'middle')
        o.edge('middle', 1, 13, 'end')
        o.edge('middle', 1, 14, 'end')  # Best-Q tie uses the smaller y.
    elif kind == 'peel':
        o.meeting.clear()
        o.node('end', blocked={3: {11}})
        o.edge('s', 0, 1, 'end')
    return o


def initial(o):
    state = L.ToyState('s')
    lattice = set(range(C.P)) - o.off(state)
    snap, ever = o.measure(state, lattice, set())
    cp = dict(cursor=C.search_cursor(state.digest()), last=snap, ever=C.bitset(ever),
              steps=0, letters=0, unit_count=0, unit_hash=C.ZERO_HASH,
              diagnostics=None, qualification=None)
    return state, cp, lattice


def one(o, state, cp, lattice, interrupt=lambda _: None):
    cursor, events, row, after, ever = E.choose_step(state, o, cursor=cp['cursor'], before=cp['last'],
        initial_lattice=lattice, ever=C.unbitset(cp['ever']), letters=cp['letters'], interrupt=interrupt)
    unit = C.unit_record(cp['unit_count'] + 1, cp['unit_hash'], cp['cursor'], cursor, events, row)
    new = dict(cp, cursor=cursor, unit_count=unit['ordinal'], unit_hash=unit['sha256'])
    if row:
        new.update(last=row['after'], ever=C.bitset(ever), steps=row['index'], letters=row['cumulative_letters'])
    return after, new, unit


def finish(o, state=None, cp=None, lattice=None):
    if state is None:
        state, cp, lattice = initial(o)
    units = []
    oldsteps = cp['steps']
    while cp['cursor']['phase'] not in ('handoff', 'stuck') and cp['steps'] == oldsteps:
        state, cp, unit = one(o, state, cp, lattice)
        units.append(unit)
    return state, cp, units


def toy_worker(path):
    doc = C.parse(Path(path).read_bytes())
    o = oracle(doc['kind'])
    state, _, lattice = initial(o)
    state.name = doc['state']
    st, cp, units = finish(o, state, doc['checkpoint'], lattice)
    print(C.canonical(dict(state=st.name, checkpoint=cp, units=units)))


def toy_checker_worker(path):
    doc = C.parse(Path(path).read_bytes())
    o = oracle()
    original, _, lattice = initial(o)
    with patch.dict(C.CASES, {7306: tuple(doc['pin'])}):
        _, cp = C.authenticate_checker(doc['attempt'], doc['identity'], [(Path(doc['target']), {'end': {'steps': 1}})], [doc['row']])
        name = next(name for name in o.nodes if L.ToyState(name).digest() == cp['last']['digest'])
        _, cp, _ = C.replay_letter(o, L.ToyState(name), cp, [doc['row']], lattice)
    print(C.canonical(cp))


class SegmentOrderingTests(unittest.TestCase):
    def test_01_all_candidate_phases_match_old_logical_units(self):
        phases = set()
        for kind in ('pair', 'peel', 'stuck'):
            o = oracle(kind)
            trace = []
            old = E.legacy_choose_step(L.ToyState('s'), o, E.Ledger(sink=trace.append))
            path = OUT / ('old-' + kind + '.jsonl')
            path.write_text(''.join(C.canonical(e) + '\n' for e in trace))
            st, cp, units = finish(oracle(kind))
            expected = list(E.old_units(path, jumble=tuple(o.jumble)))
            self.assertEqual(len(units), len(expected))
            for i, (u, e) in enumerate(zip(units, expected)):
                E.qualification_compare(u['events'], e, i + 1)
                phases.add(u['events'][0]['at']['phase'])
            actual = units[-1]['step']
            self.assertEqual(actual['word'] if actual else None, old.get('word'))
            self.assertEqual(actual['family'] if actual else cp['cursor']['phase'].upper(),
                             old.get('family', old['result']))
        self.assertEqual(phases, {'histogram', 'classify', 'candidate', 'end', 'opener', 'target_filter', 'second_cap'})

    def test_02_differential_candidate_grid_and_ties(self):
        for c in (0, 1):
            for x in L.Oracle.items:
                for d in (0, 1, 2):
                    if c == d:
                        continue
                    for y in L.Oracle.jumble:
                        o = oracle('stuck')
                        o.insides[1] = {9, 10, 11}
                        o.node('mid', blocked={3: {11}})
                        o.node('low', {'2': 20, '4': 2})
                        o.edge('s', c, x, 'mid')
                        o.edge('mid', d, y, 'low')
                        expected = L.reference(L.ToyState('s'), o)
                        _, cp, units = finish(o)
                        row = units[-1]['step']
                        self.assertEqual((row['family'], row['word']) if row else ('STUCK', None), expected)

    def test_03_interrupt_every_unit_boundary_and_selection_transaction(self):
        o = oracle(full=True)
        state, cp, lattice = initial(o)
        phases = set()
        while cp['steps'] == 0:
            old = copy.deepcopy(cp)
            for boundary in ('before_unit', 'after_unit'):
                def stop(at):
                    if at == boundary:
                        raise C.Budget(at)
                with self.assertRaises(C.Budget):
                    one(o, state, cp, lattice, stop)
                self.assertEqual(cp, old)
            if cp['cursor']['phase'] == 'group':
                for boundary in ('before_y_59', 'before_replay', 'before_measurement', 'before_step_publication'):
                    def stop(at):
                        if at == boundary:
                            raise C.Budget(at)
                    with self.assertRaises(C.Budget):
                        one(o, state, cp, lattice, stop)
                    self.assertEqual(cp, old)
                # Also interrupt inside the prefix measurements.
                with patch.object(o, 'measure', side_effect=C.Budget('measurement')):
                    with self.assertRaises(C.Budget):
                        one(o, state, cp, lattice)
                self.assertEqual(cp, old)
            phases.add(cp['cursor']['phase'])
            state, cp, unit = one(o, state, cp, lattice)
        self.assertTrue({'input', 'single', 'single_end', 'opener', 'filter', 'group'} <= phases)
        group = unit['events']
        self.assertEqual([e['at']['y'] for e in group[1:]], list(range(12, 60)))
        self.assertEqual(unit['step']['word'], [[0, 1], [1, 13]])

    def test_04_fresh_process_resume_at_every_enumerator_phase(self):
        seen = set()
        for kind in ('pair', 'peel', 'stuck'):
            o = oracle(kind)
            state, cp, lattice = initial(o)
            while cp['steps'] == 0 and cp['cursor']['phase'] != 'stuck':
                phase = cp['cursor']['phase']
                if phase not in seen:
                    seen.add(phase)
                    expected_state, expected_cp, expected_units = finish(o, state, cp, lattice)
                    packet = OUT / 'restart.json'
                    C.write_json(packet, dict(kind=kind, state=state.name, checkpoint=cp))
                    child = subprocess.run([sys.executable, '-I', '-B', str(Path(__file__).resolve()),
                                            '--toy-worker', str(packet)], capture_output=True, text=True, timeout=15)
                    self.assertEqual(child.returncode, 0, child.stderr)
                    got = C.parse(child.stdout)
                    self.assertEqual(got, dict(state=expected_state.name, checkpoint=expected_cp, units=expected_units))
                state, cp, _ = one(o, state, cp, lattice)
        self.assertEqual(seen, {'histogram', 'input', 'single', 'single_end', 'opener', 'filter', 'group',
                               'cl2_end', 'peel', 'peel_end'})

    def test_05_qualification_missing_mismatch_censored_groups_and_totals(self):
        o = oracle(full=True)
        trace = []
        E.legacy_choose_step(L.ToyState('s'), o, E.Ledger(sink=trace.append))
        path = OUT / 'qualify.jsonl'
        path.write_text(''.join(C.canonical(e) + '\n' for e in trace))
        units = list(E.old_units(path))
        last = units[-1]
        self.assertEqual(len(last), 49)
        with self.assertRaisesRegex(ValueError, 'qualification mismatch'):
            bad = copy.deepcopy(last)
            bad[-1]['detail']['Q'] += 1
            E.qualification_compare(bad, last, 10)
        with self.assertRaisesRegex(ValueError, 'no complete unit'):
            E.qualification_compare(last, None, 10)
        cutoff = next(i for i, e in enumerate(trace) if e['at'].get('y') == 59)
        path.write_text(''.join(C.canonical(e) + '\n' for e in trace[:cutoff]))
        self.assertEqual(len(list(E.old_units(path))), len(units) - 1)
        totals = E.prediction_totals(units)
        self.assertEqual(totals['endpoint_predictions'], 48)
        # DEFECT-COUNTS: every evaluated outcome, including a height prune.
        synthetic = [[dict(at=dict(family='SINGLE', phase='candidate'), outcome=o, detail={})]
                     for o in ['no_descent'] * 1200 + ['height_prune'] * 48]
        for k in range(829):
            synthetic.append([dict(at=dict(family='CL2', phase='second_cap'), outcome='intermediate_clean', detail={})]
                + [dict(at=dict(family='CL2', phase='second_item'), outcome='endpoint_prune' if k * 48 + y < 2832 else 'endpoint', detail={})
                   for y in range(48)])
        totals = E.prediction_totals(synthetic)
        self.assertEqual((totals['single_predictions'], totals['endpoint_predictions']), (1248, 39792))

    def test_06_qualification_continues_after_accepted_steps(self):
        o = oracle()
        o.node('later', {'2': 12, '4': 2})
        o.edge('end', 0, 12, 'later')
        trace, state = [], L.ToyState('s')
        for index in (1, 2):
            answer = E.legacy_choose_step(state, o, E.Ledger(index, sink=trace.append))
            state = answer['state']
        path = OUT / 'two-step-old.jsonl'
        path.write_text(''.join(C.canonical(e) + '\n' for e in trace))
        expected = list(E.old_units(path, jumble=tuple(o.jumble)))
        state, cp, lattice = initial(o)
        got = []
        while cp['steps'] < 2:
            state, cp, unit = one(o, state, cp, lattice)
            got.append(unit['events'])
        self.assertEqual(len(got), len(expected))
        for i, (a, b) in enumerate(zip(got, expected)):
            E.qualification_compare(a, b, i + 1)



class ArtifactTests(unittest.TestCase):
    def test_01_chunks_paths_truncation_and_atomic_finalization(self):
        directory = OUT / 'chunks'
        directory.mkdir()
        data = b'x' * (C.MAX_FILE + 77)
        a = C.pack_artifact(directory, 'big', data)
        self.assertTrue(all(c['size'] < C.MAX_FILE for c in a['chunks']))
        self.assertEqual(C.artifact_bytes(directory, a), data)
        for name in ('../outside', '/tmp/outside', 'C:\\private', 'a//b', 'a/../b'):
            with self.assertRaises(ValueError):
                C.safe_relative(name)
        p = directory / a['chunks'][-1]['path']
        p.write_bytes(p.read_bytes()[:-1])
        with self.assertRaisesRegex(ValueError, 'truncated'):
            C.artifact_bytes(directory, a)
        for where in ('before_manifest', 'before_finalize'):
            target = OUT / where
            target.mkdir()
            def stop(at):
                if at == where:
                    raise C.Budget(at)
            with self.assertRaises(C.Budget):
                C.atomic_manifest(target, {'not_a_result': True}, stop)
            self.assertFalse((target / 'manifest.json').exists())

    def test_02_cumulative_budgets_wall_deadline_and_reservations(self):
        clocks = dict(cpu=100., wall=0., utc=1000.)
        limits = C.SegmentLimits(arm=False, cpu=lambda: clocks['cpu'], monotonic=lambda: clocks['wall'],
                                  utc=lambda: clocks['utc'], not_after=5000)
        limits.boundary()
        clocks['cpu'] = 2999
        limits.check()
        self.assertEqual(limits.times()['trajectory_cpu_seconds'], 2899)
        clocks['cpu'] = 3000
        with self.assertRaises(C.Budget):
            limits.check()
        clocks['cpu'], clocks['wall'] = 100, 3300
        with self.assertRaises(C.Budget):
            limits.check()
        clocks['wall'], clocks['utc'] = 0, 4700
        with self.assertRaises(C.Budget):
            limits.check()
        for role, n in (('search', 6), ('checker', 4)):
            root = OUT / ('allocations-' + role)
            root.mkdir()
            for i in range(n):
                d = root / str(i)
                d.mkdir()
                self.assertEqual(C.reserve_attempt(root, d, 7306, role)['ordinal'], i + 1)
            with self.assertRaisesRegex(ValueError, 'exhausted'):
                C.reserve_attempt(root, root, 7306, role)
        with self.assertRaises(C.Budget):
            C.SegmentLimits(used=19700, arm=False).check()
        with self.assertRaises(ValueError):
            C.utc_deadline('2026-10-23T12:00:00')

    def test_03_bitsets_units_and_schema_rejection(self):
        self.assertEqual(C.unbitset(C.bitset({0, 99, C.P - 1})), {0, 99, C.P - 1})
        for bad in ('00', 'z' * (C.P // 4), None):
            with self.assertRaises(ValueError):
                C.unbitset(bad)
        o = oracle()
        state, cp, lattice = initial(o)
        _, after, unit = one(o, state, cp, lattice)
        domain = dict(caps=tuple(o.caps), items=tuple(o.items), jumble=tuple(o.jumble))
        self.assertEqual(C.validate_unit(unit, 1, C.ZERO_HASH, cp['cursor'], **domain), after['cursor'])
        for mutate in (lambda u: u.update(ordinal=2), lambda u: u.update(previous='1' * 64),
                       lambda u: u.update(after='1' * 64), lambda u: u.update(unknown=1),
                       lambda u: u['events'].append(u['events'][0])):
            bad = copy.deepcopy(unit)
            mutate(bad)
            bad['sha256'] = C.digest_json({k: v for k, v in bad.items() if k != 'sha256'})
            with self.assertRaises(ValueError):
                C.validate_unit(bad, 1, C.ZERO_HASH, cp['cursor'], **domain)

    def test_04_checker_import_isolation_and_portable_commands(self):
        code = 'import sys;sys.path.insert(0,' + repr(str(HERE)) + ');import check_height_descent_experiment;assert "height_descent_experiment" not in sys.modules;assert "numpy" not in sys.modules'
        result = subprocess.run([sys.executable, '-I', '-B', '-c', code], capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        for mode in ('search', 'qualification', 'diagnostics', 'checker', 'audit'):
            cmd = C.portable_command(mode, 7306)
            self.assertEqual(cmd[0], 'python3')
            self.assertTrue(all(not t.startswith('/') for t in cmd))
        with self.assertRaises(ValueError):
            C.parse('{"x":1,"x":2}')
        for kind in ('search', 'diagnostics', 'qualification', 'checker', 'audit'):
            m = dict(kind=kind, input={'case': 7306}, predecessor={'path': 'older/manifest.json'},
                     target={'path': 'search/manifest.json'}, not_after=1800000000,
                     qualification={'requested_units': 2000},
                     budget={'limits': {'work_cpu_seconds': 3000, 'cpu_seconds': 3300, 'max_steps': 200}})
            self.assertEqual(C.recorded_command(m), C.recorded_command(C.parse(C.canonical(m))))

    def test_05_manifest_chain_recovery_and_missing_artifacts(self):
        o = oracle()
        state, initial_cp, lattice = initial(o)
        pin = ('synthetic.json', C.ZERO_HASH, state.digest(), (10, 5, 0))
        root = OUT / 'chain'
        root.mkdir()
        identity = {'sources': {'test': 'fixed'}, 'ordering': C.ORDER}
        current, previous, cp, paths = state, None, initial_cp, []
        with patch.dict(C.CASES, {7306: pin}):
            for i in range(2):
                d = root / ('search-' + str(i))
                d.mkdir()
                start = C.boundary(cp)
                units = []
                for _ in range(2 if i == 0 else 1):
                    current, cp, unit = one(o, current, cp, lattice)
                    units.append(unit)
                artifacts = {
                    'checkpoint': C.pack_artifact(d, 'checkpoint', (C.canonical(cp) + '\n').encode()),
                    'table': C.pack_artifact(d, 'table', b'{}\n'),
                    'units': C.pack_artifact(d, 'units', b''.join((C.canonical(u) + '\n').encode() for u in units)),
                    'events': C.pack_artifact(d, 'events', b'')}
                path = C.publish(d, 'search', identity, C.input_identity(7306), 'campaign', previous,
                                 start, C.boundary(cp), artifacts,
                                 dict(local_cpu=1, cpu=i + 1, segments=i + 1), 9999999999, 'BUDGET')
                paths.append(path)
                previous = C.manifest_ref(path)
            chain, got = C.search_chain(paths[-1], identity)
            self.assertEqual(got, cp)
            # Disposable files are irrelevant to validated recovery.
            cache = paths[-1].parent / 'cache.disposable'
            cache.write_text('throw away')
            cache.unlink()
            self.assertEqual(C.search_chain(paths[-1], identity)[1], cp)
            with self.assertRaisesRegex(ValueError, 'identity'):
                C.search_chain(paths[-1], {'sources': {'test': 'stale'}})
            original = paths[0].read_bytes()
            paths[0].unlink()
            with self.assertRaises(OSError):
                C.search_chain(paths[-1], identity)
            paths[0].write_bytes(original)
            m = C.parse(paths[-1].read_bytes())
            cpfile = paths[-1].parent / m['artifacts']['checkpoint']['chunks'][0]['path']
            saved = cpfile.read_bytes()
            bad = C.parse(saved)
            bad['ever'] = C.bitset({17})
            cpfile.write_text(C.canonical(bad) + '\n')
            with self.assertRaises(ValueError):
                C.search_chain(paths[-1], identity)
            cpfile.write_bytes(saved)
            table = paths[-1].parent / m['artifacts']['table']['chunks'][0]['path']
            table.unlink()
            with self.assertRaises(OSError):
                C.search_chain(paths[-1], identity)


def checker_initial(o, state, lattice):
    first, ever = o.snapshot(state, lattice, set())
    return C.checker_initial(first, ever)


def segmented_row(row):
    row['search'] = dict(format=C.CHAIN, deciding_cursor=C.cursor_hash(C.search_cursor(row['input_digest'])))
    return row


class ToyAudit(C.ExhaustionAudit):
    """Explicit transition oracle adapter; no geometry shortcut is tested here."""
    def __init__(self, engine, *domain):
        super().__init__(engine, *(domain or (tuple(engine.caps), tuple(engine.items), tuple(engine.jumble))))

    def classification(self, state, c, inside=True):
        cl = self.engine.classify(state, c)
        return SimpleNamespace(straddle=None if self.engine.clean(cl) else {'piece': self.engine.certificate(cl)},
                               uncertain=False, inside=cl.inside)

    def endpoint(self, state, c, x, cl, hist):
        return self.engine.histogram(self.move(state, c, x))

    def peel_blocked(self, state, end, c, clean):
        return self.engine.blocked(end)


class TestLimits:
    def __init__(self, *args, not_after=None, **kwargs):
        self.not_after = not_after or time.time() + 86400
    def check(self):
        pass
    boundary = finalizing = check
    def times(self):
        return dict(cpu_seconds=0., wall_seconds=0., trajectory_cpu_seconds=0., setup_cpu_seconds=0.)


@contextmanager
def toy_campaign(kind='pair'):
    """Real manifests/receipts and orchestration with a declared tiny simulator."""
    o = oracle(kind)
    state, cp, lattice = initial(o)
    original = copy.deepcopy(cp)
    root = OUT / ('toy-campaign-' + C.uuid.uuid4().hex)
    root.mkdir()
    pin = ('synthetic.json', C.ZERO_HASH, state.digest(), (10, 5, 0))
    identity = {'sources': {'test': 'fixed'}, 'ordering': C.ORDER}
    rt = SimpleNamespace(np=SimpleNamespace(__version__='oracle'))
    o.snapshot_cpu = 0.
    o.rt = rt
    domain = dict(caps=tuple(o.caps), items=tuple(o.items), jumble=tuple(o.jumble))
    validate, advance_audit = C.validate_unit, C.audit_advance

    def table(st, rt):
        return (C.canonical({'state': st.name}) + '\n').encode()

    def load(directory, descriptor, rt, digest, temporary):
        st = L.ToyState(C.parse(C.artifact_bytes(directory, descriptor))['state'])
        C.same(st.digest(), digest, 'reconstructed state digest')
        return st

    with ExitStack() as stack:
        for ctx in (patch.dict(C.CASES, {7306: pin}), patch.object(C, 'Runtime', return_value=rt),
                    patch.object(C, 'segment_identity', return_value=identity), patch.object(C, 'ReplayEngine', return_value=o),
                    patch.object(C, 'load_case', return_value=state), patch.object(C, 'table_bytes', side_effect=table),
                    patch.object(C, 'load_artifact_table', side_effect=load), patch.object(C, 'SegmentLimits', TestLimits),
                    patch.object(C, 'ExhaustionAudit', ToyAudit),
                    patch.object(C, 'validate_unit', side_effect=lambda *a, **kw: validate(*a, **domain)),
                    patch.object(C, 'audit_advance', side_effect=lambda cur, proof, *args:
                                 advance_audit(cur, proof, *args) if args else advance_audit(cur, proof, **domain))):
            stack.enter_context(ctx)
        previous, paths = None, []
        for i in range(2):
            directory = root / ('search-' + str(i))
            directory.mkdir()
            start = C.boundary(cp)
            units = []
            while cp['steps'] == 0 and cp['cursor']['phase'] != 'stuck':
                state, cp, unit = one(o, state, cp, lattice)
                units.append(unit)
                if i == 0 and len(units) == 2:
                    break
            artifacts = {name: C.pack_artifact(directory, name, raw) for name, raw in (
                ('checkpoint', (C.canonical(cp) + '\n').encode()), ('table', table(state, rt)),
                ('units', b''.join((C.canonical(u) + '\n').encode() for u in units)),
                ('events', b''.join((C.canonical(u['step']) + '\n').encode() for u in units if u['step'])))}
            status = 'STUCK' if cp['cursor']['phase'] == 'stuck' else 'BUDGET'
            path = C.publish(directory, 'search', identity, C.input_identity(7306), 'toy', previous, start,
                             C.boundary(cp), artifacts, dict(local_cpu=0., cpu=0., segments=i + 1),
                             time.time() + 86000 if i == 0 else deadline, status)
            deadline = C.parse(path.read_bytes())['not_after']
            paths.append(path)
            previous = C.manifest_ref(path)
        args = SimpleNamespace(out=str(root), chain=str(paths[-1]), audit=False, resume_check=None,
                               not_after=None, cpu_seconds=3300, work_cpu_seconds=3000, wall_seconds=3600)
        def run(**changes):
            actual = SimpleNamespace(**(vars(args) | changes))
            stream = io.StringIO()
            with redirect_stdout(stream):
                code = C.run_checker(actual)
            report = C.parse(stream.getvalue().splitlines()[-1])
            return code, report, root / report['manifest']
        yield SimpleNamespace(o=o, rt=rt, root=root, paths=paths, args=args, run=run,
                              initial=original, final=cp, load=load)



class CheckerTests(unittest.TestCase):
    def test_01_linear_hundred_step_replay_and_transient_loss(self):
        o = oracle('stuck')
        o.nodes['s'] = dict(hist={'2': 10, '4': 201}, blocked={3: {11}})
        state = L.ToyState('s')
        lattice = set(range(C.P)) - o.off(state)
        before, ever = o.measure(state, lattice, set())
        rows, cursor_state = [], state
        for i in range(100):
            middle, end = f'm{i}', f'e{i}'
            o.node(middle, {'2': 500, '4': 201 - i}, blocked={3: {11}})
            o.node(end, {'2': 10, '4': 200 - i}, blocked={3: {11}})
            o.edge(cursor_state.name, 0, 1, middle)
            o.edge(middle, 1, 12, end)
            a, b = L.ToyState(middle), L.ToyState(end)
            decision = dict(family='CL2', word=[[0, 1], [1, 12]], states=[a, b], state=b)
            row, ever = E.step_record(decision, o, before, lattice, ever, i + 1, 2 * i,
                                     E.Ledger(i + 1), lambda: {})
            rows.append(segmented_row(row))
            cursor_state, before = b, row['after']
        cp = checker_initial(o, state, lattice)
        snapshots = [1]
        original = o.snapshot
        def snapshot(*args):
            snapshots[0] += 1
            return original(*args)
        o.snapshot = snapshot
        while cp['steps'] < 100:
            state, cp, _ = C.replay_letter(o, state, cp, rows, lattice)
            # Simulate serialization of unfinished steps as well as whole steps.
            cp = C.parse(C.canonical(cp))
        self.assertEqual(snapshots[0], 201)
        self.assertEqual(cp['snapshots'], 201)
        self.assertEqual(cp['last'], rows[-1]['after'])
        self.assertGreater(len(C.unbitset(cp['ever'])), cp['last']['lattice_loss'])
        self.assertEqual(cp['ever'], C.bitset(ever))

    def test_02_all_measurements_checked_and_unfinished_letter_recovery(self):
        o = oracle()
        state, _, lattice = initial(o)
        row, _, _, _ = L.record(o, state, [[0, 1], [1, 13]], 'CL2', lattice)
        segmented_row(row)
        cp = checker_initial(o, state, lattice)
        for field in ('Bk', 'M', 'N', 'off', 'Q', 'lattice_loss', 'ever_lattice_loss'):
            for prefix in (0, 1):
                bad = copy.deepcopy(row)
                bad['prefixes'][prefix][field] += 1
                s, c = state, cp
                with self.assertRaises(ValueError):
                    for _ in range(2):
                        s, c, _ = C.replay_letter(o, s, c, [bad], lattice)
        bad = copy.deepcopy(row)
        bad['before']['Bk'] += 1
        with self.assertRaisesRegex(ValueError, 'before measurements'):
            C.replay_letter(o, state, cp, [bad], lattice)
        with patch.object(o, 'snapshot', side_effect=C.Budget('interrupt snapshot')):
            with self.assertRaises(C.Budget):
                C.replay_letter(o, state, cp, [row], lattice)
        self.assertEqual(state.name, 's')
        middle, partial, _ = C.replay_letter(o, state, cp, [row], lattice)
        self.assertEqual((partial['steps'], len(partial['prefixes'])), (0, 1))
        end, final, _ = C.replay_letter(o, middle, C.parse(C.canonical(partial)), [row], lattice)
        self.assertEqual(final['steps'], 1)
        self.assertEqual(end.digest(), row['output_digest'])

    def test_03_checker_checkpoint_chain_authentication_and_retry(self):
        o = oracle()
        state, _, lattice = initial(o)
        row, _, _, _ = L.record(o, state, [[0, 1], [1, 13]], 'CL2', lattice)
        segmented_row(row)
        cp0 = checker_initial(o, state, lattice)
        mid, cp1, receipt1 = C.replay_letter(o, state, cp0, [row], lattice)
        _, cp2, receipt2 = C.replay_letter(o, mid, cp1, [row], lattice)
        root = OUT / 'checker-chain'
        root.mkdir()
        searchdir = root / 'search'
        searchdir.mkdir()
        target = searchdir / 'manifest.json'
        target.write_text('{}\n')
        identity = {'sources': {'test': 'fixed'}}
        pin = ('synthetic.json', C.ZERO_HASH, state.digest(), (10, 5, 0))
        previous = None
        with patch.dict(C.CASES, {7306: pin}):
            for i, (cp, receipts) in enumerate(((cp1, [dict(type='INITIAL', checkpoint=cp0), receipt1]),
                                               (cp2, [receipt2]))):
                d = root / ('checker-' + str(i))
                d.mkdir()
                artifacts = {name: C.pack_artifact(d, name, raw) for name, raw in (
                    ('checkpoint', (C.canonical(cp) + '\n').encode()), ('table', b'{}\n'), ('audit', b''),
                    ('verification', b''.join((C.canonical(r) + '\n').encode() for r in receipts)))}
                path = C.publish(d, 'checker', identity, C.input_identity(7306), 'c', previous,
                    None if i == 0 else C.checker_boundary(cp1), C.checker_boundary(cp), artifacts,
                    dict(local_cpu=1, cpu=i + 1, segments=i + 1), 9999999999,
                    'incomplete', extra=dict(target=C.manifest_ref(target), report={}))
                chain, authenticated = C.authenticate_checker(path, identity, [(target, {'end': {'steps': 1}})], [row])
                self.assertEqual(authenticated, cp)
                if i == 0:
                    packet = OUT / 'pending-checker.json'
                    C.write_json(packet, dict(attempt=str(path), identity=identity, target=str(target),
                                             pin=pin, row=row))
                    child = subprocess.run([sys.executable, '-I', '-B', str(Path(__file__).resolve()),
                                            '--toy-checker-worker', str(packet)], capture_output=True, text=True, timeout=15)
                    self.assertEqual(child.returncode, 0, child.stderr)
                    self.assertEqual(C.parse(child.stdout), cp2)
                previous = C.manifest_ref(path)
            m = C.parse(path.read_bytes())
            m['status'] = 'FAILED'
            C.write_json(path, m)
            with self.assertRaisesRegex(ValueError, 'failure status'):
                C.authenticate_checker(path, identity, [(target, {'end': {'steps': 1}})], [row])

    def test_04_true_false_and_forged_exhaustion(self):
        for kind in ('stuck', 'pair', 'peel'):
            o = oracle(kind)
            auditor = ToyAudit(o, tuple(o.caps), tuple(o.items), tuple(o.jumble))
            state = L.ToyState('s')
            cur = C.audit_cursor(o.histogram(state))
            proofs = []
            def run():
                nonlocal cur
                while cur['phase'] != 'complete':
                    cur, proof = auditor.unit(state, cur)
                    proofs.append(proof)
            if kind == 'stuck':
                run()
                self.assertEqual(cur['phase'], 'complete')
                # Dropping an interval cannot be hidden by a new hash chain.
                check = C.audit_cursor(o.histogram(state))
                with self.assertRaises(ValueError):
                    for proof in proofs[1:]:
                        check = C.audit_advance(check, proof, tuple(o.caps), tuple(o.items), tuple(o.jumble))
            else:
                with self.assertRaisesRegex(ValueError, 'false STUCK'):
                    run()
        o = oracle('stuck')
        cur = C.audit_cursor(o.histogram(L.ToyState('s')))
        with self.assertRaises(ValueError):
            C.audit_advance(cur, dict(at={k: cur[k] for k in ('phase', 'classified', 'ci', 'xi', 'di')},
                                     outcome='exhausted'))
        cp = checker_initial(o, L.ToyState('s'), set())
        final = dict(steps=0, cursor=dict(phase='stuck'))
        report = C.checking_report(cp, final)
        self.assertEqual(report['result'], 'BUDGET')
        self.assertEqual(report['exhaustion_verification'], 'not_performed')
        # Original defect: three labels and no input/candidate coverage.
        cursor = C.search_cursor(L.ToyState('s').digest())
        for family in ('SINGLE', 'CL2', 'PEEL'):
            with self.assertRaises(ValueError):
                C.advance_cursor(cursor, [dict(at=dict(family=family, phase='end'), outcome='exhausted', detail={})])

    def test_05_audit_incomplete_y_groups_and_resume(self):
        o = oracle('stuck', full=True)
        cur = C.audit_cursor(o.histogram(L.ToyState('s')))
        cur.update(phase='target', classified=4, clean=[0, 1], ci=0, xi=0, di=1)
        proof = dict(at={k: cur[k] for k in ('phase', 'classified', 'ci', 'xi', 'di')},
                     outcome='endpoints', ys=list(range(12, 60)), heights=[cur['hist']] * 48)
        after = C.audit_advance(cur, proof, tuple(o.caps), tuple(o.items), tuple(o.jumble))
        self.assertEqual(after['di'], 2)
        bad = copy.deepcopy(proof)
        bad['ys'].pop()
        bad['heights'].pop()
        with self.assertRaisesRegex(ValueError, 'incomplete audit y'):
            C.audit_advance(cur, bad, tuple(o.caps), tuple(o.items), tuple(o.jumble))
        self.assertEqual(C.audit_advance(C.parse(C.canonical(cur)), C.parse(C.canonical(proof)),
                                         tuple(o.caps), tuple(o.items), tuple(o.jumble)), after)


def rewrite_artifact(path, name, raw):
    """Deliberate mutation of one small test artifact, with its outer hashes fixed."""
    m = C.parse(path.read_bytes())
    old = m['artifacts'][name]
    assert len(old['chunks']) == 1
    chunk = old['chunks'][0]['path']
    (path.parent / chunk).write_bytes(raw)
    m['artifacts'][name] = dict(size=len(raw), sha256=C.sha(raw),
                              chunks=[dict(path=chunk, size=len(raw), sha256=C.sha(raw))])
    C.write_json(path, m)


class CheckerRegressionTests(unittest.TestCase):
    def test_01_post_replay_interruptions_are_incomplete_and_resume_only_missing_checks(self):
        for stopping in ('boundaries', 'final_last', 'final_ever', 'tables'):
            with self.subTest(check=stopping), toy_campaign() as t:
                post = C.checker_post_unit
                def stop(cp, *args):
                    pending = (cp['steps'] == 1 and (
                        (stopping == 'boundaries' and len(cp['checks']['boundaries']) == 1) or
                        (stopping in ('final_last', 'final_ever') and cp['checks'][stopping] is None
                         and len(cp['checks']['boundaries']) == 2) or
                        (stopping == 'tables' and len(cp['checks']['tables']) == 1)))
                    if pending:
                        raise C.Budget('post-replay ' + stopping)
                    return post(cp, *args)
                with patch.object(C, 'checker_post_unit', side_effect=stop):
                    code, report, attempt = t.run()
                self.assertEqual(code, 1)
                self.assertEqual((report['status'], report['passed'], report['literal_replay']),
                                 ('incomplete', False, 'incomplete'))
                self.assertEqual(report['steps'], 1)
                snapshots = report['snapshots']
                with patch.object(t.o, 'apply', side_effect=AssertionError('replayed completed prefix')):
                    code, report, resumed = t.run(resume_check=str(attempt))
                self.assertEqual((code, report['status'], report['passed']), (0, 'complete', True))
                self.assertEqual(report['snapshots'], snapshots)
                self.assertEqual(len(report['checks']['tables']), 2)
                chain, final = C.search_chain(t.paths[-1])
                C.authenticate_checker(resumed, chain[-1][1]['identity'], chain, C.search_rows(chain))

    def test_02_audit_usage_error_precedes_reservation_and_publication(self):
        with toy_campaign() as t, patch.object(C, 'reserve_attempt') as reserve, patch.object(C, 'new_output') as create:
            with self.assertRaisesRegex(ValueError, 'candidate STUCK'):
                t.run(audit=True)
            reserve.assert_not_called()
            create.assert_not_called()

    def test_03_false_stuck_preserves_literal_replay_and_names_candidate_on_retry(self):
        with toy_campaign('stuck') as t:
            t.o.node('low', {'2': 10, '4': 4})
            t.o.edge('s', 0, 12, 'low')
            code, report, attempt = t.run(audit=True)
            self.assertEqual((code, report['status'], report['passed']), (1, 'FAILED', False))
            self.assertEqual((report['literal_replay'], report['exhaustion_verification']), ('complete', 'failed'))
            self.assertEqual(report['invalidates'], dict(stuck_claim=True, accepted_steps=False))
            self.assertEqual(report['independent_candidate']['word'], [[0, 12]])
            code, resumed, _ = t.run(audit=True, resume_check=str(attempt))
            self.assertEqual(resumed['literal_replay'], 'complete')
            self.assertEqual(resumed['exhaustion_verification'], 'failed')
            self.assertEqual(resumed['independent_candidate'], report['independent_candidate'])

    def test_04_single_and_peel_replay_mutations_and_whole_prefix_digest_lists(self):
        for family in ('SINGLE', 'PEEL'):
            o = oracle('peel' if family == 'PEEL' else 'stuck')
            if family == 'SINGLE':
                o.node('low', {'2': 20, '4': 2})
                o.edge('s', 0, 12, 'low')
            state, _, lattice = initial(o)
            _, _, units = finish(o)
            row = units[-1]['step']
            self.assertEqual(row['family'], family)
            cp = checker_initial(o, state, lattice)
            C.replay_letter(o, state, cp, [row], lattice)
            mutations = []
            for field in row['prefixes'][0]:
                bad = copy.deepcopy(row)
                bad['prefixes'][0][field] = {} if field == 'heights' else 'forged'
                mutations.append(bad)
            for field in ('input_digest', 'output_digest'):
                bad = copy.deepcopy(row)
                bad[field] = '1' * 64
                mutations.append(bad)
            for field in ('prefixes', 'prefix_digests'):
                for delta in (-1, 1):
                    bad = copy.deepcopy(row)
                    if delta < 0:
                        bad[field].pop()
                    else:
                        bad[field].append(bad[field][-1])
                    mutations.append(bad)
            for search in ({}, {'format': C.CHAIN, 'deciding_cursor': None},
                           {'format': 'wrong', 'deciding_cursor': '1' * 64}):
                bad = copy.deepcopy(row)
                bad['search'] = search
                mutations.append(bad)
            for bad in mutations:
                with self.subTest(family=family, row=bad):
                    with self.assertRaises(ValueError):
                        C.replay_letter(o, state, cp, [bad], lattice)
            # Shape-valid but wrong deciding cursor is rejected by unit validation.
            bad = copy.deepcopy(units[-1])
            bad['step']['search']['deciding_cursor'] = '1' * 64
            bad['sha256'] = C.digest_json({k: v for k, v in bad.items() if k != 'sha256'})
            cur, prior = C.search_cursor(state.digest()), C.ZERO_HASH
            domain = dict(caps=tuple(o.caps), items=tuple(o.items), jumble=tuple(o.jumble))
            for u in units[:-1]:
                cur = C.validate_unit(u, u['ordinal'], prior, cur, **domain)
                prior = u['sha256']
            with self.assertRaisesRegex(ValueError, 'deciding cursor binding'):
                C.validate_unit(bad, bad['ordinal'], prior, cur, **domain)

    def test_05_malformed_step_and_resume_receipts_publish_failed_manifest(self):
        for field in ('prefixes', 'prefix_digests'):
            with self.subTest(field=field), toy_campaign() as t:
                path = t.paths[-1]
                m = C.parse(path.read_bytes())
                units = list(C.artifact_lines(path.parent, m['artifacts']['units']))
                units[-1]['step'][field] = []
                units[-1]['sha256'] = C.digest_json({k: v for k, v in units[-1].items() if k != 'sha256'})
                rewrite_artifact(path, 'units', b''.join((C.canonical(u) + '\n').encode() for u in units))
                code, report, attempt = t.run()
                self.assertEqual((code, report['status'], report['literal_replay']), (1, 'FAILED', 'failed'))
                self.assertIn('prefix', report['reason'])
                self.assertEqual(report['invalidates']['accepted_steps_from'], 1)
                self.assertTrue(attempt.is_file())
        with toy_campaign() as t:
            with patch.object(t.o, 'apply', side_effect=C.Budget('before first letter')):
                _, _, attempt = t.run()
            m = C.parse(attempt.read_bytes())
            receipts = list(C.artifact_lines(attempt.parent, m['artifacts']['verification']))
            receipts[-1]['checkpoint']['prefixes'] = [1, 2, 3]
            rewrite_artifact(attempt, 'verification', b''.join((C.canonical(r) + '\n').encode() for r in receipts))
            code, report, _ = t.run(resume_check=str(attempt))
            self.assertEqual((code, report['status']), (1, 'FAILED'))
            self.assertIn('prefix', report['reason'])

    def test_06_inductive_validation_new_units_only_and_chain_mutations(self):
        with toy_campaign() as t:
            head = t.paths[-1]
            m = C.parse(head.read_bytes())
            n = len(list(C.artifact_lines(head.parent, m['artifacts']['units'])))
            validator = C.validate_unit
            with patch.object(C, 'validate_unit', wraps=validator) as counted:
                chain, cp = C.search_chain(head)
                self.assertEqual(counted.call_count, n)
            code, _, attempt = t.run()
            self.assertEqual(code, 0)
            with patch.object(C, 'validate_unit', side_effect=AssertionError('revalidating certified units')):
                code, _, _ = t.run(resume_check=str(attempt))
                self.assertEqual(code, 0)
            original = head.read_bytes()
            for predecessor in (None, C.manifest_ref(head)):
                bad = C.parse(original)
                bad['predecessor'] = predecessor
                bad['command'] = C.recorded_command(bad)
                C.write_json(head, bad)
                with self.assertRaises(ValueError):
                    C.search_chain(head)
                head.write_bytes(original)
            first_original = t.paths[0].read_bytes()
            backwards_head = C.parse(original)
            backwards_head['predecessor'] = None
            backwards_head['command'] = C.recorded_command(backwards_head)
            C.write_json(head, backwards_head)
            backwards_first = C.parse(first_original)
            backwards_first['predecessor'] = C.manifest_ref(head)
            backwards_first['command'] = C.recorded_command(backwards_first)
            C.write_json(t.paths[0], backwards_first)
            with self.assertRaises(ValueError):
                C.search_chain(t.paths[0])
            t.paths[0].write_bytes(first_original)
            head.write_bytes(original)
            ancestor = t.paths[0]
            raw = ancestor.read_bytes()
            ancestor.write_bytes(raw + b' ')
            with self.assertRaises(ValueError):
                C.search_chain(head)
            ancestor.write_bytes(raw)

    def test_09_literal_failure_invalidates_only_from_the_failing_step(self):
        with toy_campaign() as t:
            t.o.nodes['end']['hist']['4'] += 1
            code, report, attempt = t.run()
            self.assertEqual((code, report['literal_replay'], report['exhaustion_verification']),
                             (1, 'failed', 'not_performed'))
            self.assertEqual(report['invalidates']['accepted_steps_from'], 1)
            chain, _ = C.search_chain(t.paths[-1])
            with self.assertRaisesRegex(C.ReplayFailure, 'invalidates steps from 1'):
                C.authenticate_checker(attempt, chain[-1][1]['identity'], chain, C.search_rows(chain))
            _, resumed, _ = t.run(resume_check=str(attempt))
            self.assertEqual(resumed['literal_replay'], 'failed')
            self.assertEqual(resumed['invalidates']['accepted_steps_from'], 1)

    def test_07_histogram_binding_and_search_status_kind(self):
        with toy_campaign('stuck') as t:
            head = t.paths[0]
            raw = head.read_bytes()
            for status in ('QUALIFIED', 'DIAGNOSTICS'):
                bad = C.parse(raw)
                bad['status'] = status
                C.write_json(head, bad)
                with self.assertRaisesRegex(ValueError, 'kind/status'):
                    C.search_chain(head)
                head.write_bytes(raw)
            m = C.parse(raw)
            units = list(C.artifact_lines(head.parent, m['artifacts']['units']))
            units[0]['events'][0]['detail']['heights']['4'] += 1
            rewrite_artifact(head, 'units', b''.join((C.canonical(u) + '\n').encode() for u in units))
            with self.assertRaisesRegex(ValueError, 'histogram unit/accepted'):
                C.search_chain(head)
        with toy_campaign('stuck') as t:
            head = t.paths[-1]
            m = C.parse(head.read_bytes())
            cp = C.parse(C.artifact_bytes(head.parent, m['artifacts']['checkpoint']))
            cp['cursor']['hist']['4'] += 1
            rewrite_artifact(head, 'checkpoint', (C.canonical(cp) + '\n').encode())
            with self.assertRaises(ValueError):
                C.search_chain(head)

    def test_08_audit_requires_finished_prefixes_and_matching_final_histogram(self):
        with toy_campaign() as t:
            apply = t.o.apply
            calls = []
            def one_letter(*args):
                if calls:
                    raise C.Budget('pending CL2')
                calls.append(True)
                return apply(*args)
            with patch.object(t.o, 'apply', side_effect=one_letter):
                _, _, attempt = t.run()
            m = C.parse(attempt.read_bytes())
            cp = C.parse(C.artifact_bytes(attempt.parent, m['artifacts']['checkpoint']))
            self.assertEqual(len(cp['prefixes']), 1)
            receipts = list(C.artifact_lines(attempt.parent, m['artifacts']['verification']))
            new = dict(cp, audit=C.audit_cursor(cp['last']['heights']))
            receipts.append(dict(type='AUDIT_START', checkpoint=new))
            rewrite_artifact(attempt, 'verification', b''.join((C.canonical(r) + '\n').encode() for r in receipts))
            chain, _ = C.search_chain(t.paths[-1])
            with self.assertRaisesRegex(ValueError, 'audit before complete'):
                C.authenticate_checker(attempt, chain[-1][1]['identity'], chain, C.search_rows(chain))
        with toy_campaign('stuck') as t:
            code, report, attempt = t.run(audit=True)
            self.assertEqual((code, report['result']), (0, 'STUCK'))
            m = C.parse(attempt.read_bytes())
            receipts = list(C.artifact_lines(attempt.parent, m['artifacts']['verification']))
            receipts[-1]['checkpoint']['audit']['hist'] = {'4': 999}
            rewrite_artifact(attempt, 'verification', b''.join((C.canonical(r) + '\n').encode() for r in receipts))
            chain, _ = C.search_chain(t.paths[-1])
            with self.assertRaisesRegex(ValueError, 'audit origin'):
                C.authenticate_checker(attempt, chain[-1][1]['identity'], chain, C.search_rows(chain))



def previous_prediction(engine, state, c, x, cl, hist):
    """fabe918 IncrementalEngine.predict, retained only as this benchmark's baseline."""
    engine.tick()
    tw = engine.twist(c, x)
    if tw.kidx >= 0:
        return dict(hist)
    height, left, _ = engine.matrix_info(tw.matrix, tw.kidx)
    out = E.Counter(hist)
    for matrix, k, members in engine.groups(state, cl.inside):
        engine.tick()
        old, factor, _ = engine.matrix_info(matrix, k)
        if old:
            key = tw.matrix, matrix
            if key not in engine.products:
                engine.products[key] = int(engine.rt.trees.dist(engine.rt.trees.qmul(left, factor)))
            new = engine.products[key]
            out[str(old)] -= len(members)
        else:
            new = height
        if new:
            out[str(new)] += len(members)
    C.require(all(v >= 0 for v in out.values()), 'negative effective histogram')
    engine.stats['predictions'] += 1
    return {h: n for h, n in out.items() if n}



class ExactSegmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        runtime_started = time.process_time()
        cls.rt = C.Runtime(OUT)
        cls.runtime_setup_cpu = time.process_time() - runtime_started
        cls.engine = E.IncrementalEngine(cls.rt)
        cls.reference = E.ExactEngine(cls.rt)
        st = cls.rt.sim.State(cls.rt.ctx, menu=cls.rt.menu, filtered=False)
        for c, x in L.WORD36:
            st = cls.reference.move(st, c, x)
        cls.z = st

    def compare_caps(self, base, c, x, caps):
        e, r = self.engine, self.reference
        cl = e.classify(base, c)
        self.assertTrue(e.clean(cl))
        virtual = e.virtual(base, c, x, cl)
        actual = e.move(base, c, x)
        self.assertIsNotNone(actual)
        self.assertEqual(e.histogram(virtual), r.histogram(actual))
        for d in caps:
            predicted, want = e.classify(virtual, d), r.classify(actual, d)
            self.assertEqual(predicted.status, want.status, (c, x, d))
            if want.straddle is None:
                self.assertEqual(set(predicted.inside.tolist()), set(want.inside.tolist()), (c, x, d))
        for p in sorted(set(cl.inside.tolist()))[::max(1, len(cl.inside) // 20)]:
            groups = e.groups(virtual, self.rt.np.array([p]))
            self.assertEqual(groups[0][0], actual.pose(p))
        return virtual, actual

    def test_01_virtual_retained_jumble_cancellation_merging_and_all_endpoints(self):
        e, r = self.engine, self.reference
        auditor = C.ExhaustionAudit(C.ReplayEngine(self.rt))
        digest = self.z.digest()
        for x in (1, 14):
            virtual, applied = self.compare_caps(self.z, 12, x, (0, 12, 35, 74, 188, 599))
            for target in (12, 35):
                vc, ac = e.classify(virtual, target), r.classify(applied, target)
                if not e.clean(vc):
                    continue
                hist, results = e.histogram(virtual), []
                for y in range(12, 60):
                    p = e.predict(virtual, target, y, vc, hist)
                    q = r.histogram(r.move(applied, target, y))
                    self.assertEqual(p, q)
                    self.assertEqual(auditor.endpoint(applied, target, y, ac, hist), q)
                    results.append((E.potential(p), y))
                self.assertEqual(min(results), min((E.potential(e.predict(applied, target, y, ac, hist)), y)
                                                  for y in range(12, 60)))
        # E(c,x)^-1 is an item of the same menu; test exact cancellation to K+.
        base = self.rt.sim.State(self.rt.ctx, menu=self.rt.menu, filtered=False)
        first = e.move(base, 0, 12)
        tw = e.twist(0, 12)
        from exact import transpose
        inverse = tuple(tuple(row) for row in transpose(tw.matrix))
        y = next(i for i in range(1, 60) if e.twist(0, i).matrix == inverse)
        virtual, actual = self.compare_caps(first, 0, y, range(600))
        self.assertEqual(e.histogram(virtual), {})
        self.assertEqual(actual.digest(), base.digest())
        self.assertEqual(self.z.digest(), digest, 'trial isolation')

    def test_02_matrix_keys_cold_warm_evicted_and_reused_ids(self):
        e, r = self.engine, self.reference
        a = r.move(self.z, 12, 14)
        for state, cap in ((self.z, 12), (a, 35)):
            cl, hist = e.classify(state, cap), e.histogram(state)
            want = e.predict(state, cap, 13, cl, hist)
            self.assertEqual(want, e.predict(state, cap, 13, cl, hist))
            e.factors.clear()
            e.products.clear()
            e.compositions.clear()
            self.assertEqual(want, e.predict(state, cap, 13, cl, hist))
            tiny = E.IncrementalEngine(self.rt, cache_limit=1)
            self.assertEqual(want, tiny.predict(state, cap, 13, cl, hist))
            self.assertLessEqual(len(tiny.products), 1)
            # Same matrices under different local IDs; no persistent ID keys.
            reordered = r.clone(state)
            n = len(reordered._poses)
            order = [0] + list(range(n - 1, 0, -1))
            mapping = self.rt.np.empty(n, dtype=self.rt.np.int32)
            mapping[order] = self.rt.np.arange(n)
            reordered._poses = [reordered._poses[i] for i in order]
            reordered.pose_id = mapping[state.pose_id]
            reordered._lookup = {p.key: i for i, p in enumerate(reordered._poses)}
            self.assertEqual(want, e.predict(reordered, cap, 13, cl, hist))
            reordered._compact()
            self.assertEqual(want, e.predict(reordered, cap, 13, cl, hist))
        # Membership retained by a retained letter, member poses changed.
        cl = e.classify(self.z, 12)
        retained = r.move(self.z, 12, 1)
        hist = r.histogram(retained)
        self.assertEqual(e.predict(retained, 12, 14, cl, hist), r.histogram(r.move(retained, 12, 14)))

    def test_03_tangency_and_selected_per_piece_oracles(self):
        from sim.kernel import Evaluator, pullback_form
        rt, e = self.rt, self.engine
        solved = rt.sim.State(rt.ctx, menu=rt.menu, filtered=False)
        inside = solved.classify(0).inside
        p = int(inside[0])
        ev = Evaluator(pullback_form(solved.pose(p), rt.ctx.data.N[0]))
        denominator, rows = rt.ctx.regions.vform(p)
        self.assertIn(0, [ev.sign_row(denominator, r) for r in rows])
        self.assertEqual(ev.classify((denominator, rows))[0], 'in')
        virtual = e.virtual(self.z, 12, 14, e.classify(self.z, 12))
        with patch.object(Evaluator, 'classify', return_value=('uncertain', 0)):
            with self.assertRaisesRegex(ValueError, 'uncertain'):
                e.restricted(self.z, 12, rt.np.array([next(iter(e.off(self.z)))]))
        for d in (0, 12, 35, 188):
            for matrix, k, pieces in e.groups(virtual, self.rt.np.arange(0, C.P, 1753)):
                for p in pieces[:2].tolist():
                    exact = Evaluator(pullback_form(matrix, rt.ctx.data.N[d])).classify(rt.ctx.regions.vform(p))[0]
                    got = e.restricted(virtual, d, rt.np.array([p]))
                    expected = 'straddle' if not e.clean(got) else 'in' if len(got.inside) else 'out'
                    self.assertEqual(expected, exact)

    def test_04_recorded_7306_both_letters_every_cap_and_global_Bk(self):
        started = time.monotonic()
        e, r = E.IncrementalEngine(self.rt), self.reference
        load_started = time.process_time()
        state = C.load_case(7306, self.rt, r)
        input_load_cpu = time.process_time() - load_started
        # Seed the input with full classifications once; both implementations use
        # this same certified base, then independently classify every new cap.
        classification_started = time.process_time()
        input_classes = {d: r.classify(state, d) for d in range(600)}
        input_classification_cpu = time.process_time() - classification_started
        auditor = C.ExhaustionAudit(C.ReplayEngine(self.rt))
        clean = [d for d, cl in input_classes.items() if r.clean(cl)]
        auditor.base_state, auditor.base_classes = state, dict(input_classes)
        eligible_pairs = sum(1 for c in clean for d in auditor.meeting_caps(c)
                             if input_classes[d].straddle is None or
                             input_classes[d].straddle['piece'] in auditor.moved(state, c))
        checker = C.ReplayEngine(self.rt)
        checker_state = state
        lattice = set(range(C.P)) - checker.off(state)
        snapshot_started = time.process_time()
        first_snapshot, seen = checker.snapshot(state, lattice, set())
        initial_snapshot_cpu = time.process_time() - snapshot_started
        self.assertEqual(first_snapshot['Bk'], 600 - len(clean))
        apply_costs, peel_costs, target_costs, prediction_costs, replay_snapshot_costs = [], [], [], [], []
        endpoint_cost = None
        e.classes.update({(state, d): cl for d, cl in input_classes.items()})
        multiple = False
        for c, x in ((208, 5), (336, 15)):
            virtual = e.virtual(state, c, x, e.classify(state, c))
            apply_started = time.process_time()
            actual = e.move(state, c, x)
            apply_costs.append(time.process_time() - apply_started)
            expected_bk = observed_bk = 0
            actual_classes, cap_costs = {}, {}
            for d in range(600):
                classify_started = time.process_time()
                want = r.classify(actual, d)
                cap_costs[d] = time.process_time() - classify_started
                actual_classes[d] = want
                got = e.classify(virtual, d)
                self.assertEqual(got.status, want.status, (c, x, d))
                if e.clean(want):
                    self.assertEqual(got.inside.tolist(), want.inside.tolist(), (c, x, d))
                expected_bk += not e.clean(want)
                observed_bk += not e.clean(got)
                e.classes[actual, d] = got
            self.assertEqual(observed_bk, expected_bk)
            checker_state = checker.clone(checker_state)
            checker.apply(checker_state, c, x)
            snapshot_started = time.process_time()
            snap, seen = checker.snapshot(checker_state, lattice, seen)
            replay_snapshot_costs.append(time.process_time() - snapshot_started)
            self.assertEqual(snap['Bk'], expected_bk)
            self.assertEqual(snap['digest'], actual.digest())
            peel_started = time.process_time()
            self.assertEqual(auditor.peel_blocked(state, actual, c,
                [d for d, cl in auditor.base_classes.items() if cl.straddle is None]), expected_bk)
            peel_costs.append(time.process_time() - peel_started)
            target_costs.extend(cap_costs[d] for d in auditor.meeting_caps(c)
                                if auditor.original(state, d).straddle is None or
                                auditor.original(state, d).straddle['piece'] in auditor.moved(state, c))
            hist = e.histogram(state)
            self.assertEqual(e.predict(state, c, x, e.classify(state, c), hist), r.histogram(actual))
            # Full set, not just the first witness; known blocked target 336.
            if c == 208:
                blockers = e.straddlers(state, 336)
                multiple = len(blockers) > 1
                self.assertTrue(blockers <= e.inside(e.classify(state, c)))
                cl = e.classify(virtual, 336)
                self.assertTrue(e.clean(cl))
                middle = e.histogram(virtual)
                ac = actual_classes[336]
                for y in range(12, 60):
                    expected = r.predict(actual, 336, y, ac, middle)
                    self.assertEqual(e.predict(virtual, 336, y, cl, middle), expected)
                    self.assertEqual(auditor.endpoint(actual, 336, y, ac, middle), expected)
                # Equal cache limits, a fresh engine for each implementation, and
                # reset the one-group distribution between 48-item groups.
                answers = []
                for implementation in (previous_prediction, E.IncrementalEngine.predict):
                    bench = E.IncrementalEngine(self.rt)
                    batches, costs = [], []
                    for repeat in range(6):
                        bench.distribution = (None, None, ())
                        begin = time.process_time()
                        batches.append([implementation(bench, virtual, 336, y, cl, middle) for y in range(12, 60)])
                        costs.append((time.process_time() - begin) / 48)
                    prediction_costs.append(sum(costs) / len(costs))
                    answers.append(batches)
                self.assertEqual(answers[0], answers[1])
                self.assertLess(prediction_costs[1], prediction_costs[0])
                endpoint_cost = auditor.stats['prediction_cpu'] / auditor.stats['prediction_count']
            e.retain(actual)
            # Endpoint classifications reused by the next step equal fresh ones.
            for d in (0, 208, 336, 487, 599):
                self.assertEqual(e.classify(actual, d).status, r.classify(actual, d).status)
            state = actual
            auditor.base_state, auditor.base_classes, auditor.insides = actual, actual_classes, {}
        self.assertTrue(multiple, 'multiple moving blockers must be exercised')
        self.assertTrue(state.digest().startswith('782c2034c13135fc'))
        self.assertGreater(e.stats['unmoved_witness'], 0)
        self.assertGreater(e.stats['unmoved_fallbacks'], 0)
        # Measure the cursor/hash/JSON/spool cost of one rejected target unit,
        # including the 26-entry clean list. This is protocol work, no trajectory.
        cur = C.audit_cursor(first_snapshot['heights'])
        cur.update(phase='target', clean=clean, classified=600, ci=0, xi=0, di=0)
        proof = dict(at={k: cur[k] for k in ('phase', 'classified', 'ci', 'xi', 'di')}, outcome='ineligible')
        cp = dict(audit=cur, audit_units=0, audit_hash=C.ZERO_HASH)
        begin = time.process_time()
        with io.BytesIO() as spool:
            for _ in range(2000):
                after = C.audit_advance(cur, proof)
                unit = C.audit_record(cp, after, proof)
                spool.write((C.canonical(unit) + '\n').encode())
        protocol_cost = (time.process_time() - begin) / 2000
        # Projection, not a timed campaign: 100 CL2 steps / 201 snapshots, four
        # cold process starts (each first replay snapshot does a full recount),
        # all 59 openers per clean cap, all certificate-surviving targets, the
        # packet's 39,792 endpoints, and all 59 PEEL items per clean cap, even
        # those the count/height guard would prune. Allow four audit-cache rebuilds.
        # Eligible-target and snapshot costs sample TWO fixed letters, not a bound
        # uniform over unseen states. Include every possible protocol interval.
        mean = lambda xs: sum(xs) / len(xs)
        replay_projection = 4 * initial_snapshot_cpu + 197 * mean(replay_snapshot_costs) + 200 * mean(apply_costs)
        audit_projection = (4 * input_classification_cpu + len(clean) * 59 * mean(apply_costs)
                            + eligible_pairs * 59 * mean(target_costs)
                            + (39792 + len(clean) * (48 + 59 + 59)) * endpoint_cost
                            + len(clean) * 59 * (mean(apply_costs) + mean(peel_costs))
                            + (600 + len(clean) * (48 + 59 + 59 * 600 + 59)) * protocol_cost)
        setup_projection = 4 * (self.runtime_setup_cpu + input_load_cpu)
        metrics = dict(prediction_before_ms=1000 * prediction_costs[0],
                       prediction_after_ms=1000 * prediction_costs[1],
                       audit_prediction_ms=1000 * endpoint_cost, apply_cpu=mean(apply_costs),
                       full_600_caps_cpu=input_classification_cpu, initial_snapshot_cpu=initial_snapshot_cpu,
                       replay_snapshot_cpu=mean(replay_snapshot_costs), audit_protocol_unit_cpu=protocol_cost,
                       eligible_target_cpu=mean(target_costs), peel_recount_cpu=mean(peel_costs),
                       clean_caps=len(clean), certificate_surviving_cap_pairs=eligible_pairs,
                       replay_100_step_projection_cpu=replay_projection,
                       audit_projection_cpu=audit_projection, four_process_setup_cpu=setup_projection,
                       combined_projection_cpu=replay_projection + audit_projection + setup_projection,
                       peak_rss_kib=C.resource.getrusage(C.resource.RUSAGE_SELF).ru_maxrss,
                       python=sys.version, numpy=self.rt.np.__version__, input=C.input_identity(7306),
                       sources={p: C.sha((C.ROOT / p).read_bytes()) for p in C.SOURCE_PATHS})
        print('7306 unit-cost projection: ' + C.canonical(metrics), flush=True)
        C.write_json(OUT / '7306-unit-costs.json', metrics)
        self.assertLess(metrics['peak_rss_kib'], 3 * 1024 * 1024)
        self.assertLess(metrics['combined_projection_cpu'], 4 * 3000, 'projected useful work exceeds four segments')
        print(f'7306 bounded two-letter all-cap differential: {time.monotonic() - started:.3f}s', flush=True)

    def test_07_audit_target_uses_one_opener_and_no_endpoint_apply_or_histogram_recount(self):
        checker = C.ReplayEngine(self.rt)
        auditor = C.ExhaustionAudit(checker)
        cur = C.audit_cursor(checker.histogram(self.z))
        cur.update(phase='target', clean=[12], classified=600, ci=0, xi=13, di=35)
        with patch.object(checker, 'apply', wraps=checker.apply) as applied, \
                patch.object(checker, 'histogram', side_effect=AssertionError('unused middle histogram')):
            with self.assertRaises(C.FalseStuck) as result:
                auditor.unit(self.z, cur)
        self.assertEqual(applied.call_count, 1)
        self.assertEqual(result.exception.proof['ys'], list(range(12, 60)))
        self.assertEqual(len(result.exception.proof['heights']), 48)

    def test_06_replay_blocked_locality_union_and_interruption_invalidation(self):
        checker = C.ReplayEngine(self.rt)
        expected = C.ReplayEngine(self.rt)
        base = checker.clone(self.z)
        checker.blocked(base)  # The first observation is always a full sweep.
        trial = checker.clone(base)
        checker.apply(trial, 12, 14)
        checker.apply(trial, 35, 13)  # No snapshot between: union of both neighbourhoods.
        classify = trial.classify
        with patch.object(trial, 'classify', wraps=classify) as counted:
            got = checker.blocked(trial)
            self.assertLessEqual(counted.call_count, 162)
            self.assertGreater(counted.call_count, 0)
        self.assertEqual(got, expected.blocked(trial))
        # Interrupt after an actual application commits but before the engine can
        # mark its new basis. It must fall back to a full sweep, never the old Bk.
        trial = checker.clone(base)
        checker.blocked(trial)
        apply = trial.apply
        def interrupted(*args, **kwargs):
            apply(*args, **kwargs)
            raise C.Budget('after simulator commit')
        with patch.object(trial, 'apply', side_effect=interrupted):
            with self.assertRaises(C.Budget):
                checker.apply(trial, 12, 14)
        with patch.object(trial, 'classify', wraps=trial.classify) as counted:
            got = checker.blocked(trial)
            self.assertEqual(counted.call_count, 600)
        self.assertEqual(got, expected.blocked(trial))

    def test_05_real_segment_diagnostics_and_fresh_checker_retry(self):
        # Only the small WORD36-derived fixture is substituted, never a real
        # development-table search. Exercise the actual CLI orchestration seams.
        from types import SimpleNamespace
        fixture = self.reference.move(self.z, 12, 14)
        table = OUT / 'segment-fixture.json'
        C.export_table(fixture, self.rt, table)
        pin = (str(table), C.sha(table.read_bytes()), fixture.digest(), (3097, 1, 0))
        class TestLimits:
            def __init__(self, *args, not_after=None, **kwargs):
                self.not_after = not_after or time.time() + 86400
                self.started = time.process_time()
                self.wall = time.monotonic()
            def check(self):
                pass
            boundary = check
            finalizing = check
            def times(self):
                return dict(cpu_seconds=time.process_time() - self.started,
                            wall_seconds=time.monotonic() - self.wall,
                            trajectory_cpu_seconds=0, setup_cpu_seconds=0)
        def search_args(root, **changes):
            values = dict(out=str(root), case=7306, diagnostics_only=False, qualify_ledger=None,
                          units=None, resume=None, not_after=None, cpu_seconds=3300,
                          work_cpu_seconds=3000, wall_seconds=3600, max_steps=1)
            values.update(changes)
            return SimpleNamespace(**values)
        def latest(root, kind):
            return next(root.glob(kind + '-*/manifest.json'))
        root, direct = OUT / 'actual-segments', OUT / 'direct-segment'
        probe = ([[35, 13]], 0, {'2': 5405}, '3ab25d214b6c4ad759ac7158a0fc1cabae1584aa711d3f757af7b91c43971837')
        with patch.object(C, 'TABLE_ROOT', ''), patch.dict(C.CASES, {7306: pin}), \
                patch.dict(C.PROBES, {7306: [probe]}), patch.object(C, 'SegmentLimits', TestLimits):
            self.assertEqual(E.run_segment(search_args(root, diagnostics_only=True)), 0)
            diagnostic = latest(root, 'diagnostics')
            _, cp = C.search_chain(diagnostic)
            self.assertEqual(cp['last']['digest'], fixture.digest())
            self.assertEqual(C.unbitset(cp['ever']), set())
            self.assertEqual(cp['unit_count'], 0)
            self.assertEqual(E.run_segment(search_args(root, resume=str(diagnostic))), 0)
            self.assertEqual(E.run_segment(search_args(direct)), 0)
            head = latest(root, 'search')
            with_diag, cp = C.search_chain(head)
            no_diag, cp_direct = C.search_chain(latest(direct, 'search'))
            self.assertEqual(cp['last'], cp_direct['last'])
            self.assertEqual(cp['ever'], cp_direct['ever'])
            for a, b in zip(C.search_rows(with_diag), C.search_rows(no_diag)):
                for key in ('word', 'family', 'input_digest', 'prefix_digests', 'output_digest',
                            'before', 'prefixes', 'after'):
                    self.assertEqual(a[key], b[key])
            args = SimpleNamespace(out=str(root), chain=str(head), audit=False, resume_check=None,
                                   not_after=None, cpu_seconds=3300, work_cpu_seconds=3000, wall_seconds=3600)
            with patch.object(C.ReplayEngine, 'apply', side_effect=C.Budget('verification interruption')):
                self.assertEqual(C.run_checker(args), 1)
            attempt = latest(root, 'checker')
            m = C.parse(attempt.read_bytes())
            self.assertEqual(m['report']['literal_replay'], 'incomplete')
            # Finish in a new process/new immutable attempt directory. The child
            # imports only the checker and receives the synthetic pin explicitly.
            payload = dict(args=vars(args), pin=pin, scripts=str(HERE))
            payload['args']['resume_check'] = str(attempt)
            code = "import sys,json;from types import SimpleNamespace;d=json.loads(sys.argv[1]);sys.path.insert(0,d['scripts']);import check_height_descent_experiment as C;C.TABLE_ROOT='';C.CASES[7306]=tuple(d['pin']);sys.exit(C.run_checker(SimpleNamespace(**d['args'])))"
            child = subprocess.run([sys.executable, '-I', '-B', '-c', code, C.canonical(payload)],
                                   capture_output=True, text=True, timeout=60)
            self.assertEqual(child.returncode, 0, child.stdout + child.stderr)
            report = C.parse(child.stdout.splitlines()[-1])
            self.assertEqual(report['result'], 'HANDOFF')
            self.assertEqual(report['snapshots'], 2)
            self.assertEqual(report['literal_replay'], 'complete')
            self.assertNotEqual(root / report['manifest'], attempt)



if __name__ == '__main__':
    try:
        if '--toy-worker' in sys.argv:
            toy_worker(sys.argv[-1])
            code = 0
        elif '--toy-checker-worker' in sys.argv:
            toy_checker_worker(sys.argv[-1])
            code = 0
        else:
            suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
            result = unittest.TextTestRunner(verbosity=2).run(suite)
            print(f'Segmented tests: {result.testsRun}; failures: {len(result.failures)}; '
                  f'errors: {len(result.errors)}; worker wall seconds: {time.monotonic() - START:.3f}', flush=True)
            code = 0 if result.wasSuccessful() else 1
    finally:
        L.TEMP.cleanup()
    sys.exit(code)
