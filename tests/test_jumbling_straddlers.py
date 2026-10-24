"""EN2-G full sets, exact certificates, harness rules and bounded cost-lead smoke.

One shared J1 context and the first 60 I_a records; no runtime repository
writes. Shared unittest invocations isolate NumPy as the J1 replay tests do.
"""

import sys

sys.dont_write_bytecode = True

from contextlib import redirect_stderr
from copy import deepcopy
from dataclasses import replace
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
JUMBLING = ROOT / 'research' / 'jumbling'
sys.path.insert(0, str(JUMBLING))

from differential import harness as H
from differential import independence as I

_IN_PROCESS = __name__ == '__main__'
_TEMP_ROOT = None
CAPS = (0, 150, 218, 599)
PREFIX_DIGEST = 'ec3acb21fe14da213680d2b2e83a04d2f74dc8f77fa6dffeac4fd495a60b6e0a'


def _audit_writes(event, args):
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
        raise AssertionError('test attempted to create a link')
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
                raise AssertionError('directory fd is outside the temporary test root')
        if not path.resolve().is_relative_to(_TEMP_ROOT):
            raise AssertionError('test attempted a write outside its temporary root')


def setUpModule():
    global _TEMP_ROOT, CTX, STATE, STEPS, np, sim, J1Engine, S, X2, COST
    temporary = tempfile.TemporaryDirectory(prefix='jumbling-straddlers-')
    _TEMP_ROOT = Path(temporary.name).resolve()
    if _TEMP_ROOT.is_relative_to(ROOT):
        raise AssertionError('temporary test data must be outside the repository')
    old_tempdir = tempfile.tempdir
    tempfile.tempdir = str(_TEMP_ROOT)

    def cleanup():
        global _TEMP_ROOT
        try:
            temporary.cleanup()
        finally:
            _TEMP_ROOT = None
            tempfile.tempdir = old_tempdir

    unittest.addModuleCleanup(cleanup)
    sys.addaudithook(_audit_writes)
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for J1 full straddler sets and cost leads') from error
    import sim
    from differential.j1_adapter import J1Engine
    from differential import straddlers as S
    from differential.leads import x2_straddlers as X2, classify_cost as COST

    CTX = sim.get_context()
    STEPS = H.load_steps(JUMBLING / 'fixtures' / 'wj-I_a.json')
    engine = J1Engine(CTX)
    engine.start(STEPS.menu)
    for step in STEPS.steps[:60]:
        if engine.apply(step).status != 'admissible':
            raise AssertionError('I_a prefix replay was not admissible')
    STATE = engine.state
    if STATE.digest() != PREFIX_DIGEST:
        raise AssertionError('I_a prefix digest differs from the recorded J1 prefix')


def _brute_force(state, cap):
    """J1's unfiltered per-piece path: every off piece, no groups or early exit."""
    from sim.kernel import Evaluator, pullback_form

    evaluators, pieces = {}, []
    for piece in np.flatnonzero(~state.lattice_flags()).tolist():
        pid = int(state.pose_id[piece])
        if pid not in evaluators:
            evaluators[pid] = Evaluator(pullback_form(state._pose_matrix(pid), state.ctx.data.N[cap]))
        result = evaluators[pid].classify(state.ctx.regions.vform(piece))
        if result[0] == 'straddle':
            pieces.append(piece)
    return pieces


def _perturb(certificate):
    certificate['point_below'][0][0] += 10**6 * certificate['point_below'][0][2]


class StraddlerSetTests(unittest.TestCase):
    def test_full_sets_against_every_off_piece_and_survey(self):
        before = STATE.snapshot()
        survey = STATE.survey(CAPS)
        for cap in CAPS:
            with self.subTest(cap=cap):
                result = S.straddler_set(STATE, cap)
                self.assertEqual(result['pieces'], _brute_force(STATE, cap))
                self.assertEqual(result['blocked'], bool(result['pieces']))
                self.assertEqual(set(result['certificates']), set(result['pieces']))
                self.assertEqual(result['status'], survey[cap]['status'])
                self.assertEqual(result['blocked'], cap in (0, 150))
                self.assertEqual(sum(result['counts'].values()), len(STATE.pose_id))
                self.assertEqual(result['uncertain_pieces'], 0)
                self.assertGreaterEqual(result['seconds'], 0)
                self.assertGreater(result['exact_evaluations'], 0)
                for piece, cert in result['certificates'].items():
                    self.assertTrue(S.check_certificate(STATE, cap, piece, cert))
                if result['blocked']:
                    self.assertIn(survey[cap]['certificate']['piece'], result['pieces'])
        self.assertEqual(STATE.snapshot(), before)

    def test_lattice_uses_only_signatures(self):
        state = sim.State(CTX)
        with patch.object(state, '_eval', side_effect=AssertionError('lattice vertices evaluated')):
            result = S.straddler_set(state, 0, budget=0)
        self.assertEqual(result['status'], 'admissible')
        self.assertFalse(result['blocked'])
        self.assertEqual(result['pieces'], [])
        self.assertEqual(result['counts']['signature'], len(state.pose_id))
        self.assertEqual(result['exact_evaluations'], 0)

    def test_budget_never_calls_unknown_pieces_clear(self):
        for budget in (0, 1):
            with self.subTest(budget=budget):
                result = S.straddler_set(STATE, 0, budget=budget)
                self.assertEqual(result['status'], 'uncertain')
                self.assertGreater(result['uncertain_pieces'], 0)
                self.assertEqual(result['pieces'], [])
                self.assertFalse(result['blocked'])
                self.assertEqual(sum(result['counts'].values()) + result['uncertain_pieces'], len(STATE.pose_id))
                self.assertLessEqual(result['exact_evaluations'], budget)

    def test_filtered_full_set_keeps_the_exact_answer(self):
        exact = S.straddler_set(STATE, 150)
        previous = STATE.filtered
        try:
            STATE.filtered = True
            filtered = S.straddler_set(STATE, 150)
        finally:
            STATE.filtered = previous
        self.assertEqual(filtered['pieces'], exact['pieces'])
        self.assertEqual(filtered['blocked'], exact['blocked'])
        self.assertGreater(filtered['filtered_evaluations'], 0)
        for piece, cert in filtered['certificates'].items():
            self.assertTrue(S.check_certificate(STATE, 150, piece, cert))

    def test_input_validation_and_import_independence(self):
        for cap in (True, -1, 600, 0.0, '0'):
            with self.subTest(cap=cap), self.assertRaises(ValueError):
                S.straddler_set(STATE, cap)
        for budget in (True, -1, 1.0):
            with self.subTest(budget=budget), self.assertRaises(ValueError):
                S.straddler_set(STATE, 0, budget=budget)
        paths = I.harness_files()
        self.assertIn(JUMBLING / 'differential' / 'straddlers.py', paths)
        self.assertEqual(I.scan_paths(paths, I.HARNESS_POLICY), [])


class CertificateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.piece, cls.cert = next(iter(S.straddler_set(STATE, 0)['certificates'].items()))

    def test_rejects_off_region_swapped_piece_and_cap(self):
        original = deepcopy(self.cert)
        bad = deepcopy(original)
        _perturb(bad)
        self.assertFalse(S.check_certificate(STATE, 0, self.piece, bad))
        bad = deepcopy(original)
        bad['point_below'], bad['point_above'] = bad['point_above'], bad['point_below']
        self.assertFalse(S.check_certificate(STATE, 0, self.piece, bad))
        bad = deepcopy(original)
        bad['piece'] += 1
        self.assertFalse(S.check_certificate(STATE, 0, self.piece, bad))
        self.assertFalse(S.check_certificate(STATE, 0, self.piece + 1, original))
        bad = deepcopy(original)
        bad['grip'] = 1
        self.assertFalse(S.check_certificate(STATE, 0, self.piece, bad))
        self.assertFalse(S.check_certificate(STATE, 1, self.piece, original))
        # Matching names alone cannot make this piece's points fit a solved pose.
        self.assertFalse(S.check_certificate(sim.State(CTX), 0, self.piece, original))
        self.assertEqual(self.cert, original)

    def test_does_not_trust_h_or_vertex_metadata(self):
        cert = deepcopy(self.cert)
        cert.update(h_below=[1, 0, 1], h_above=[-1, 0, 1], vertex_below=-999,
                    vertex_above=-999, points_checked_against_all_constraints=False)
        self.assertTrue(S.check_certificate(STATE, 0, self.piece, cert))
        # Different exact encodings of the points are also valid.
        for name in ('point_below', 'point_above'):
            cert[name] = [[2 * value for value in triple] for triple in cert[name]]
        self.assertTrue(S.check_certificate(STATE, 0, self.piece, cert))

    def test_rejects_malformed_exact_vectors(self):
        for point in (None, [], [[0, 0, 1]] * 3, [[0, 0]] * 4,
                      [[0.0, 0, 1]] * 4, [[True, 0, 1]] * 4,
                      [[0, 0, 0]] * 4, [[0, 0, -1]] * 4, [[0, 0, 1.0]] * 4):
            with self.subTest(point=point):
                bad = deepcopy(self.cert)
                bad['point_below'] = point
                self.assertFalse(S.check_certificate(STATE, 0, self.piece, bad))
        for cap, piece, cert in ((True, self.piece, self.cert), (0, True, self.cert),
                                 (600, self.piece, self.cert), (0, -1, self.cert),
                                 (0, len(STATE.pose_id), self.cert), (0, self.piece, None),
                                 (0, self.piece, {})):
            with self.subTest(cap=cap, piece=piece):
                self.assertFalse(S.check_certificate(STATE, cap, piece, cert))


class HarnessStraddlerTests(unittest.TestCase):
    def _engines(self, fault=None):
        class ObservedJ1(J1Engine):
            def start(self, menu):
                self.calls = []
                return super().start(menu)

            def apply(self, step):
                self.calls.append(len(self.calls) + 1)
                return super().apply(step)

            def straddlers(self, caps):
                result = super().straddlers(caps)
                return fault(result) if fault is not None and len(self.calls) == 60 else result

        return J1Engine(CTX, 'reference'), ObservedJ1(CTX, 'candidate')

    def test_j1_against_j1_with_reference_bound_checker(self):
        def alternate_points(result):
            from exact import Q5
            from sim.kplus import q5_json

            for cap, row in result.items():
                for cert in row['certificates'].values():
                    below = [Q5(*x) for x in cert['point_below']]
                    above = [Q5(*x) for x in cert['point_above']]
                    hb, ha = CTX.data.h(cap, below), CTX.data.h(cap, above)
                    t = -hb / (ha - hb)
                    contact = [b + t * (a - b) for b, a in zip(below, above)]
                    # New points on the same closed segment, strictly on each side.
                    cert['point_below'] = [q5_json((b + z) / 2) for b, z in zip(below, contact)]
                    cert['point_above'] = [q5_json((a + z) / 2) for a, z in zip(above, contact)]
            return result

        reference, candidate = self._engines(alternate_points)
        report = H.compare(reference, candidate, replace(STEPS, steps=STEPS.steps[:60]),
                           survey={0, 60}, grips=CAPS, straddlers={0, 60}, caps=CAPS,
                           certify=reference.certify)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual(report.straddlers_compared, 2)
        self.assertEqual(report.surveys_compared, 2)
        self.assertEqual(candidate.calls, list(range(1, 61)))
        json.dumps(report.as_dict())

    def test_faults_stop_at_first_bad_step(self):
        def drop(result):
            piece = result[0]['pieces'].pop()
            result[0]['certificates'].pop(piece)
            return result

        def flip(result):
            result[0] = {'blocked': False, 'pieces': [], 'certificates': {}}
            return result

        def perturb(result):
            _perturb(result[0]['certificates'][result[0]['pieces'][0]])
            return result

        def malformed(result):
            result[0]['certificates'].clear()
            return result

        for fault, field in ((drop, 'straddlers'), (flip, 'straddlers'),
                             (perturb, 'certificate'), (malformed, 'contract')):
            with self.subTest(fault=fault.__name__):
                reference, candidate = self._engines(fault)
                report = H.compare(reference, candidate, replace(STEPS, steps=STEPS.steps[:61]),
                                   straddlers={60, 61}, caps=[150, 0], certify=reference.certify)
                self.assertFalse(report.passed)
                self.assertEqual(report.first_mismatch.field, field)
                self.assertEqual(report.first_mismatch.step, 60)
                self.assertEqual(report.first_mismatch.step_index_in_input, 59)
                self.assertEqual(report.straddlers_compared, 1)
                self.assertEqual(candidate.calls, list(range(1, 61)))

    def test_bad_reference_set_raises(self):
        reference, candidate = self._engines(lambda result: {})
        with self.assertRaises(H.ReferenceContractError):
            H.compare(candidate, reference, replace(STEPS, steps=STEPS.steps[:60]),
                      straddlers={60}, caps=[0])


class CostLeadTests(unittest.TestCase):
    def test_both_smoke_runs_and_output_fields(self):
        out = _TEMP_ROOT / 'smoke'
        before, previous = STATE.snapshot(), STATE.filtered
        rows, summary = X2.run(STATE, out, CAPS, expect=PREFIX_DIGEST, source={'fixture': 'I_a@60'})
        self.assertTrue(summary['passed'], summary)
        self.assertEqual(summary['state_digest'], PREFIX_DIGEST)
        self.assertEqual(summary['off_lattice'], STATE.off_lattice_count())
        self.assertEqual(summary['blocked_caps'], [0, 150])
        self.assertEqual(sum(summary['u_distribution'].values()), len(CAPS))
        self.assertEqual(summary['largest_u'], {'u': max(len(row['full_set']['pieces']) for row in rows),
                                                 'cap': 0})
        for row in rows:
            self.assertTrue(row['certificates_checked'])
            self.assertTrue(all(row['agreement'].values()))
            self.assertEqual(row['full_set']['u'], len(row['full_set']['pieces']))
            self.assertNotIn('certificates', row['full_set'])
        for field in ('early_exit_seconds', 'full_set_seconds'):
            self.assertGreater(summary[field]['total'], 0)
            self.assertGreaterEqual(summary[field]['largest']['seconds'], summary[field]['median'])
        self.assertEqual(set(summary['environment']), {'cpu_count', 'python', 'numpy', 'commit', 'load_average'})
        # Outside a Git checkout the commit is None, never guessed.
        self.assertRegex(summary['environment']['commit'] or '0' * 40, r'^[0-9a-f]{40}$')
        self.assertEqual(json.loads((out / 'x2-summary.json').read_text()), summary)
        self.assertEqual(len((out / 'x2-caps.jsonl').read_text().splitlines()), len(CAPS))

        rows, summary = COST.run([('I_a@60', STATE)], out, CAPS)
        self.assertTrue(summary['passed'], summary)
        self.assertEqual(summary['failures'], [])
        self.assertEqual(summary['states'][0]['state_digest'], PREFIX_DIGEST)
        for row in rows:
            self.assertTrue(all(row['agreement'].values()))
            for field in ('status', 'inside', 'blocked'):
                self.assertEqual(row['exact'][field], row['filtered'][field])
            self.assertEqual(row['exact']['filtered_evaluations'], 0)
            self.assertGreater(row['filtered']['filtered_evaluations'], 0)
            # Both modes were warmed untimed: no timed call builds an anchor or cap cache.
            for mode in ('exact', 'filtered'):
                self.assertEqual(row[mode]['cache_entries_added']['anchors'], 0)
                self.assertEqual(row[mode]['cache_entries_added']['cap_forms'], 0)
        self.assertEqual(json.loads((out / 'classify-summary.json').read_text()), summary)
        self.assertEqual(len((out / 'classify-caps.jsonl').read_text().splitlines()), len(CAPS))
        self.assertEqual(STATE.filtered, previous)
        self.assertEqual(STATE.snapshot(), before)

    def test_classification_difference_is_recorded_as_failure(self):
        original = STATE.classify

        def changed(cap, **kwargs):
            cl = original(cap, **kwargs)
            if STATE.filtered:
                cl.inside = np.array([-1], np.int64)
            return cl

        with patch.object(STATE, 'classify', side_effect=changed):
            rows, summary = COST.run([('planted', STATE)], _TEMP_ROOT / 'bad-classify', [599])
        self.assertFalse(summary['passed'])
        self.assertFalse(rows[0]['passed'])
        self.assertEqual(summary['failures'], [{'state': 'planted', 'cap': 599, 'fields': ['inside']}])

    def test_filter_restored_after_exception(self):
        previous = STATE.filtered
        with patch.object(STATE, 'classify', side_effect=RuntimeError('planted')):
            with self.assertRaises(RuntimeError):
                COST.measure(STATE, [0])
        self.assertEqual(STATE.filtered, previous)

    def test_bad_certificate_fails_x2_and_expect_is_checked(self):
        with patch.object(X2, 'check_certificate', return_value=False):
            rows, summary = X2.run(STATE, _TEMP_ROOT / 'bad-x2', [0])
        self.assertFalse(summary['passed'])
        self.assertEqual(summary['failures'], [0])
        self.assertFalse(rows[0]['certificates_checked'])
        with self.assertRaisesRegex(ValueError, 'digest'):
            X2.run(STATE, _TEMP_ROOT / 'wrong-expect', [0], expect='0' * 64)

    def test_in_repository_out_is_refused_by_functions_and_cli(self):
        out = ROOT / 'forbidden-en2-g-output'
        for call in (lambda: X2.run(STATE, out, [0]),
                     lambda: COST.run([('prefix', STATE)], out, [0])):
            with self.assertRaisesRegex(ValueError, 'outside the repository'):
                call()
        for module in (X2, COST):
            stderr = io.StringIO()
            with redirect_stderr(stderr), self.assertRaises(SystemExit) as caught:
                module.main(['--out', str(out), '--scramble', '0', '7', '--caps', '0'])
            self.assertEqual(caught.exception.code, 2)
            self.assertIn('outside the repository', stderr.getvalue())
        self.assertFalse(out.exists())

    def test_census_end_and_letters_and_scramble_inputs(self):
        # An external census-format empty walk still goes through census.walk(end=True).
        record = _TEMP_ROOT / 'record.json'
        record.write_text(json.dumps({'menu': 'I_a', 'k': 0, 'seed': 7, 'history': [], 'off_scramble': 0,
                                      'steps': [], 'moves': []}), encoding='utf-8')
        letters = _TEMP_ROOT / 'letters.json'
        letters.write_text('[[0, 1]]', encoding='utf-8')
        state, source = X2.build_state(record=record, letters=letters)
        from differential.leads import census

        expected = sim.State(CTX, menu=census.menu())
        self.assertTrue(expected.apply(census.menu().for_cap(0)[1]).applied)
        self.assertEqual(state.digest(), expected.digest())
        self.assertEqual(source['letters'], 1)
        self.assertEqual(source['kind'], 'census-end')
        state, source = X2.build_state(scramble=(0, 7))
        self.assertTrue(state.is_lattice())
        self.assertEqual(source, {'kind': 'scramble', 'k': 0, 'seed': 7})
        states = list(COST.input_states(scramble=(0, 7)))
        self.assertEqual(states[0][0], 'scramble:0:7')
        self.assertTrue(states[0][1].is_lattice())
        with self.assertRaises(ValueError):
            X2.build_state(scramble=(0, 7), letters=letters)
        letters.write_text('[[true, 1]]', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'integer pairs'):
            X2.build_state(record=record, letters=letters)


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return tests
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_straddlers', 'J1 full straddler sets in an isolated NumPy process',
                                'NumPy is required for J1 full straddler sets and cost leads')


if __name__ == '__main__':
    unittest.main()
