"""EN2-F: exact covers, recorded float flags, and XM1 seat smoke checks.

No generated files belong in the repository. NumPy/J1 import only during
setUpModule; a shared unittest command uses the isolated_numpy worker.
"""

import sys

sys.dont_write_bytecode = True

from contextlib import redirect_stderr
from copy import deepcopy
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
from time import perf_counter
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[1]
JUMBLING = REPO / 'research' / 'jumbling'
sys.path.insert(0, str(JUMBLING))
_IN_PROCESS = __name__ == '__main__'
_TEMP_ROOT = None
_MEASUREMENTS = {}


def _audit_temp_writes(event, args):
    if _TEMP_ROOT is None:
        return
    paths = []
    if event == 'open':
        path, mode, flags = args
        if flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC | os.O_APPEND):
            paths = [(path, None)]
    elif event in ('os.mkdir', 'os.chmod'):
        paths = [(args[0], args[2])]
    elif event in ('os.remove', 'os.rmdir'):
        paths = [(args[0], args[1])]
    elif event == 'os.truncate':
        paths = [(args[0], None)]
    elif event == 'os.rename':
        paths = [(args[0], args[2]), (args[1], args[3])]
    elif event in ('os.link', 'os.symlink'):
        raise AssertionError('lead tests must not create links')
    for path, directory_fd in paths:
        if isinstance(path, int):
            continue
        path = Path(os.fsdecode(path))
        if not path.is_absolute() and directory_fd not in (None, -1):
            status = os.fstat(directory_fd)
            for directory in [_TEMP_ROOT] + [p for p in _TEMP_ROOT.rglob('*') if p.is_dir()]:
                candidate = directory.stat()
                if (status.st_dev, status.st_ino) == (candidate.st_dev, candidate.st_ino):
                    path = directory / path
                    break
            else:
                raise AssertionError('directory fd is outside the test tempfile root')
        if not path.resolve().is_relative_to(_TEMP_ROOT):
            raise AssertionError('lead test attempted a write outside its tempfile root')


def setUpModule():
    global _TEMP_ROOT, np, sim, census, covers, xm1, x4, seams, matmul, to_tuple
    temporary = tempfile.TemporaryDirectory(prefix='jumbling-covers-test-')
    _TEMP_ROOT = Path(temporary.name).resolve()
    if _TEMP_ROOT.is_relative_to(REPO):
        _TEMP_ROOT = None
        temporary.cleanup()
        raise AssertionError('tempfile root must be outside the repository')
    previous_tempdir = tempfile.tempdir
    tempfile.tempdir = str(_TEMP_ROOT)

    def cleanup():
        global _TEMP_ROOT
        try:
            temporary.cleanup()
        finally:
            tempfile.tempdir = previous_tempdir
            _TEMP_ROOT = None

    unittest.addModuleCleanup(cleanup)
    sys.addaudithook(_audit_temp_writes)
    started = perf_counter()
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for exact covers and XM1 leads') from error
    # Census must precede every theory import so positions never uses work/.
    from differential.leads import census, covers
    from differential.leads import xm1_seats as xm1
    from differential.leads import x4_covers as x4
    import sim
    import seams
    from exact import matmul
    from sim.kplus import to_tuple
    _MEASUREMENTS['context_seconds'] = perf_counter() - started


class TimedCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._started = perf_counter()
        cls.addClassCleanup(cls._record_time)

    @classmethod
    def _record_time(cls):
        _MEASUREMENTS[cls.__name__] = perf_counter() - cls._started


class MeetingCapsTests(TimedCase):
    def test_exact_meeting_domain_and_threshold_gap(self):
        covers.check_meeting()
        for c in range(600):
            meeting = covers.meeting_caps(c)
            self.assertEqual(len(meeting), 80, c)
            self.assertEqual(meeting, tuple(sorted(set(meeting))), c)
            self.assertNotIn(c, meeting)
            self.assertTrue(all(c in covers.meeting_caps(d) for d in meeting), c)


class SeatTests(TimedCase):
    def test_exact_cosets_intersections_and_powers(self):
        cosets = xm1.seat_cosets()
        expected = (
            frozenset((12, 16, 20, *range(24, 33))),
            frozenset((13, 17, 19, *range(34, 43))),
            frozenset((14, 21, 23, *range(43, 52))),
            frozenset((15, 18, 22, 33, *range(52, 60))),
        )
        self.assertEqual(cosets.right[1:], expected)
        self.assertEqual(cosets.right[0], frozenset(range(12)))
        self.assertEqual(set.union(*(set(g) for g in cosets.left)), set(range(60)))
        for right in cosets.right[1:]:
            for left in cosets.left[1:]:
                self.assertEqual(len(right & left), 3)
        M = census.menu()
        index = {m: i for i, (_, m) in enumerate(M.items)}
        g = M.items[12][1]
        power = M.items[0][1]
        powers = []
        for _ in range(4):
            power = to_tuple(matmul(g, power))
            powers.append(index[power])
        self.assertEqual(powers, [12, 33, 39, 45])
        self.assertEqual({cosets.seat[x] for x in powers}, {1, 2, 3, 4})


class RecordedFlagsTests(TimedCase):
    def test_both_true_flags_and_planted_wrong_flag(self):
        flags = x4.recorded_flags()
        wanted = {next(n for n, r in flags.items() if r['c_covers_d']),
                  next(n for n, r in flags.items() if r['d_covers_c'])}
        self.assertEqual(wanted, {30, 33})
        observations = []
        planted = False
        for step in census.walk(7101, last=max(wanted)):
            if step.n not in wanted:
                continue
            before = step.S.st.digest()
            pairs = x4.compare_recorded_flags(step, flags[step.n])
            for row in pairs:
                self.assertEqual(row['float']['covers'], flags[step.n][row['recorded_flag']],
                                 f"step {step.n}: {row['recorded_flag']}")
                self.assertIsInstance(row['exact']['covers'], bool)
                if not row['agrees']:
                    self.assertIn(row['trace']['reason'], ('blocked_d', 'rounding'), f'step {step.n}')
                    self.assertTrue(row['trace']['differing_pieces'])
                observations.append({'step': step.n, 'flag': row['recorded_flag'],
                                     'float': row['float']['covers'], 'exact': row['exact']['covers'],
                                     'trace': row['trace']['reason'] if row['trace'] else None})
            if not planted:
                changed = deepcopy(flags[step.n])
                changed['c_covers_d'] = not changed['c_covers_d']
                with self.assertRaisesRegex(x4.RecordedFlagError, f'step {step.n}: c_covers_d: recorded'):
                    x4.compare_recorded_flags(step, changed)
                planted = True
            self.assertEqual(step.S.st.digest(), before)
        self.assertEqual(len(observations), 4)
        self.assertTrue(planted)
        _MEASUREMENTS['recorded_flags'] = observations


class CoversQueryTests(TimedCase):
    def test_completed_query_blocked_error_and_unchanged_state(self):
        S = census.state_at(7101, 17)
        _, record = census.load(7101)
        offset = sum(s['twists'] for s in record['steps'][:16])
        cap = record['moves'][offset][0]
        before = (S.st.digest(), S.st.pose_id.copy(), deepcopy(S.st.journal), list(S.moves))
        query, parts = covers.timed_covers_query(S, cap)
        _MEASUREMENTS['covers_query'] = {'seconds': query.seconds, 'parts': parts}
        self.assertEqual(query.status, 'completed', query.message)
        self.assertEqual(query.input_digest, before[0])
        self.assertEqual(query.domain, covers.meeting_caps(cap))
        self.assertEqual(len(query.answers), 80)
        self.assertEqual(tuple(r.cap for r in query.answers), query.domain)
        document = query.as_dict()
        self.assertEqual(set(document), {'status', 'input_digest', 'parameters', 'domain', 'answers',
                                         'seconds', 'message'})
        self.assertEqual(set(query.parameters), {'cap', 'threshold', 'menu_identity', 'menu_order_digest'})
        self.assertEqual(query.parameters['cap'], cap)
        self.assertEqual(query.parameters['threshold'], 10)
        self.assertEqual(query.parameters['menu_identity'], census.menu().identity)
        self.assertEqual(query.parameters['menu_order_digest'], census.I_A_ORDER_DIGEST)
        self.assertEqual(json.loads(json.dumps(document))['status'], 'completed')
        blocked = []
        for row in query.answers:
            self.assertEqual(set(document['answers'][query.domain.index(row.cap)]),
                             {'cap', 'd_covers_c', 'c_covers_d'})
            if isinstance(row.c_covers_d, covers.NotApplicable):
                self.assertIn(row.c_covers_d.reason, ('blocked', 'closed', 'not misaligned'))
                if row.c_covers_d.reason == 'blocked':
                    blocked.append(row.cap)
            else:
                self.assertIsInstance(row.c_covers_d.covers, bool)
            self.assertIsInstance(row.d_covers_c.covers, bool)
            classified = covers.inside_sets(S, (row.cap,))[row.cap]
            self.assertFalse(set(row.d_covers_c.straddlers) & set(classified.pieces.tolist()))
        self.assertTrue(blocked, 'smoke state must exercise a blocked neighbour')
        error = covers.covers_query(S, blocked[0])
        self.assertEqual(error.status, 'error')
        self.assertIn('blocked', error.message)
        self.assertEqual(error.answers, ())
        closed = next((r.cap for r in query.answers if isinstance(r.c_covers_d, covers.NotApplicable)
                       and r.c_covers_d.reason == 'closed'), None)
        if closed is not None:
            error = covers.covers_query(S, closed)
            self.assertEqual(error.status, 'error')
            self.assertIn('closed', error.message)
        self.assertEqual(S.st.digest(), before[0])
        np.testing.assert_array_equal(S.st.pose_id, before[1])
        self.assertEqual(S.st.journal, before[2])
        self.assertEqual(S.moves, before[3])


class XM1SmokeTests(TimedCase):
    def test_first_three_face_match_steps(self):
        observations = []
        expected_closing = ([17, 34, 38], [29], [32])
        for step in census.walk(7101, last=3):
            before = step.S.st.digest()
            row = xm1.analyze_step(step, 7101)
            self.assertEqual(row['step'], step.n)
            self.assertEqual(row['case'], 'outside_lattice')
            self.assertEqual(row['closing_set'], expected_closing[step.n - 1])
            self.assertIn(row['record_twist']['item'], row['closing_set'])
            self.assertEqual(row['seats'], [{'item': x, 'seat': xm1.seat_cosets().seat[x]}
                                             for x in row['closing_set']])
            self.assertEqual(row['record_twist']['seat'], xm1.seat_cosets().seat[step.word[0][1]])
            now, scores = step.S.scores(row['cap'])
            floor = abs(step.S.balance(row['cap'])[0] - step.S.balance(row['cap'])[1])
            self.assertEqual(row['closing_set'], [i for i in sorted(scores) if i != 0 and scores[i] == floor])
            self.assertEqual(row['closing_count'], len(row['closing_set']))
            self.assertEqual(row['N_c'], now)
            self.assertEqual(row['floor'], floor)
            sides = row['face']
            for name in ('inside', 'outside'):
                self.assertEqual(sides[name]['count'], sum(len(d['pieces']) for d in sides[name]['domains']))
            if row['case'] == 'outside_lattice':
                self.assertTrue(row['one_right_coset'])
                self.assertEqual(len({s['seat'] for s in row['seats']}), 1)
            if row['case'] == 'inside_lattice_only':
                self.assertTrue(row['one_left_coset'])
            self.assertEqual(step.S.st.digest(), before)
            observations.append({key: row[key] for key in ('step', 'case', 'closing_set', 'seat_counts')})
        self.assertEqual(len(observations), 3)
        _MEASUREMENTS['xm1_first_three'] = observations


class CLISmokeTests(TimedCase):
    def run_cli(self, name, *args):
        env = dict(os.environ, TMPDIR=str(_TEMP_ROOT), TEMP=str(_TEMP_ROOT), TMP=str(_TEMP_ROOT),
                   PYTHONDONTWRITEBYTECODE='1')
        return subprocess.run([sys.executable, '-B', str(JUMBLING / 'differential' / 'leads' / name), *map(str, args)],
                              cwd=REPO, env=env, capture_output=True, text=True, timeout=120)

    def test_refuse_repository_output(self):
        for script in ('x4_covers.py', 'xm1_seats.py'):
            with self.subTest(script=script):
                completed = self.run_cli(script, '--out', REPO)
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn('--out must be outside the repository', completed.stderr)

    def test_refuse_output_files_resolving_into_repository(self):
        real = Path.resolve
        for module, name in ((x4, 'x4-summary.json'), (xm1, 'xm1-steps.jsonl')):
            def resolve(path, strict=False, name=name):
                return REPO / 'assets' / 'manifest.json' if path.name == name else real(path, strict)

            out = _TEMP_ROOT / f'linked-{name}'
            with self.subTest(module=module.__name__), patch.object(Path, 'resolve', resolve), \
                    patch.object(census, 'walk', side_effect=AssertionError('measured before refusing')):
                error = io.StringIO()
                with redirect_stderr(error), self.assertRaises(SystemExit) as stopped:
                    module.main(['--out', str(out)])
                self.assertEqual(stopped.exception.code, 2)
                self.assertIn(f'{name} resolves inside it', error.getvalue())
            self.assertFalse(out.exists())

    def test_xm1_seed7001(self):
        out = _TEMP_ROOT / 'xm1-smoke'
        completed = self.run_cli('xm1_seats.py', '--seeds', '7001', '--out', out)
        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        self.assertEqual(completed.stdout, '')
        self.assertEqual(completed.stderr, '')
        self.assertEqual({p.name for p in out.iterdir()}, {'xm1-steps.jsonl', 'xm1-summary.json'})
        rows = [json.loads(line) for line in (out / 'xm1-steps.jsonl').read_text(encoding='utf-8').splitlines()]
        summary = json.loads((out / 'xm1-summary.json').read_text(encoding='utf-8'))
        self.assertEqual(len(rows), 9)
        self.assertEqual(summary['steps'], 9)
        self.assertEqual(summary['numpy'], np.__version__)
        # Outside a Git checkout the commit is None, never guessed.
        self.assertRegex(summary['commit'] or '0' * 40, r'^[0-9a-f]{40}$')
        self.assertGreaterEqual(summary['cpu_count'], 1)
        self.assertEqual(summary['menu_identity'], census.menu().identity)
        self.assertEqual(summary['menu_order_digest'], census.I_A_ORDER_DIGEST)
        self.assertEqual(set(summary['cases']), set(xm1.CASES))
        self.assertEqual(sum(case['steps'] for case in summary['cases'].values()), 9)
        for row in rows:
            self.assertTrue({'seed', 'step', 'cap', 'input_digest', 'face', 'domains', 'case', 'N_c',
                             'balance', 'floor', 'closing_set', 'closing_count', 'seats', 'one_right_coset',
                             'one_left_coset', 'seat_counts', 'record_twist'} <= set(row))
            self.assertEqual(row['seed'], 7001)
            self.assertIn(row['case'], xm1.CASES)
            self.assertIn(row['record_twist']['item'], row['closing_set'])
            self.assertEqual(row['closing_count'], len(row['closing_set']))
            self.assertEqual(sum(row['seat_counts'].values()), row['closing_count'])
        for counts in summary['cases'].values():
            self.assertTrue({'steps', 'one_seat', 'one_left_coset', 'split_over_seats', 'split_sizes', 'answer'} <= set(counts))
            self.assertEqual(counts['one_seat'] + counts['split_over_seats'], counts['steps'])
        _MEASUREMENTS['xm1_cli_cases'] = summary['cases']


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return tests
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_leads_covers', 'Exact covers and XM1 in an isolated NumPy process',
                                'NumPy is required for exact covers and XM1 leads')


if __name__ == '__main__':
    unittest.main()
