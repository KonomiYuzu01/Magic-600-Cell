# Exact simulator acceptance for slp_apply.py; research/headless evidence only.
# Run from the repository root, on one core and below 4 GiB, within 20 minutes:
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/test_slp_apply.py
# Loads the actual I_a depth-40 stop once for literal controls. Closure workers
# independently load that record and run the CLI, including --unit and table
# digest round-trips. All generated certificates, programs and tables are in a
# system temporary directory. Only small control words are ever expanded.
# Prints the b00 end digest, wall time and worker peak RSS; tries b19 if budget
# remains. No source, personal session, native fixture or repository output is
# changed. Linux RSS units are KiB. Child affinity and BLAS threads are one.
import contextlib
import copy
import io
import json
import math
import os
import resource
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

T0 = time.monotonic()
sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_name] = '1'
if hasattr(os, 'sched_getaffinity'):
    os.sched_setaffinity(0, {min(os.sched_getaffinity(0))})
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import slp_apply as A
import numpy as np

ROOT = HERE.parent
RECORD = ROOT.parent / '11-scramble-census/runs/round2/ia-k40-s7101.json'
START_DIGEST = '7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818'


def clone(st):
    B, _ = A.modules()
    result = B.sim.State(st.ctx, menu=st.menu)
    result.pose_id = st.pose_id.copy()
    result._poses, result._lookup = list(st._poses), dict(st._lookup)
    return result


def replay(st, M, word):
    B, _ = A.modules()
    for j, (c, i) in enumerate(word):
        twist = B.sim.Twist(st.ctx, c, B.restore.element(M, c, i), 'menu', {'menu': M.name})
        outcome = st.apply(twist, journal=False)
        if not outcome.applied or outcome.uncertain:
            raise AssertionError(f'literal letter {j} ({c}, {i}) is not admissible: {outcome.status}')


def certificate(nodes, root=None):
    """Small test DAG with independently calculated expanded lengths."""
    nodes = copy.deepcopy(nodes)
    for ident, node in enumerate(nodes):
        op = node['op']
        if op == 'letter':
            length = 1
        elif op == 'cat':
            length = sum(nodes[n]['length'] for n in node['args'])
        else:
            length = nodes[node['arg']]['length'] * (node['exp'] if op == 'pow' else 1)
        node['length'] = length
    root = len(nodes) - 1 if root is None else root
    return {'format': 'r22b10/1', 'nodes': nodes, 'V': root, 'V_length': nodes[root]['length']}


def letter(pair):
    return {'op': 'letter', 'grip': pair[0], 'item': pair[1]}


def write_json(path, doc):
    Path(path).write_text(json.dumps(doc) + '\n')
    return str(path)


def closure_worker(name):
    """Run the real CLI, observing its state after the first segment only."""
    B, screen = A.modules()
    cert_path = ROOT / f'runs/round22/e22-1b/{name}-cert.json.gz'
    cert = A.read_json(cert_path)
    unit_path = ROOT / f'runs/round22/line-a/units/eb5-k40-k00-{name}.json'
    unit = A.read_json(unit_path)
    if name == 'b00':
        target_path = ROOT / 'runs/lc13-k40/target-b00.json'
        middle = {'word_file': str(ROOT / 'runs/lc13-k40/transport-b00-word.json')}
    else:
        target_path = ROOT / 'runs/round22/line-a/closures/b19-target.json'
        middle = {'core_file': str(ROOT / 'runs/round22/line-a/closures/b19-core.json')}
    target = A.read_json(target_path)['T']
    doc = {'segments': [
        {'slp': str(cert_path), 'node': 'V', 'state_digest': cert['state_digest']},
        middle,
        {'slp': str(cert_path), 'node': 'V', 'inverse': True},
    ]}
    rows, vacated = [], []
    run = A.run_program

    def observe(st, M, program_doc, unit=None):
        if st.digest() != START_DIGEST:
            raise AssertionError('closure worker loaded the wrong state')
        fingerprint = B.off_fingerprint(st)

        def emit(row):
            rows.append(row)
            if row.get('segment') == 0:
                holes = set(B.hole_set(st).tolist())
                if not set(target) <= holes:
                    raise AssertionError('V left an occupied target chamber')
                if B.off_fingerprint(st) != fingerprint or st.off_lattice_count() != 1193:
                    raise AssertionError('V changed the off geometry')
                vacated.append(len(target))
            A.print_row(row)

        return run(st, M, program_doc, unit, emit=emit)

    with tempfile.TemporaryDirectory(prefix='slp-closure-') as temp:
        program_path = write_json(Path(temp) / 'program.json', doc)
        end_path = str(Path(temp) / 'end.json')
        start = time.monotonic()
        with patch.object(A, 'run_program', observe):
            status = A.main(['record', str(RECORD), program_path, end_path, '--unit', str(unit_path)])
        if status != 0:
            raise AssertionError('closure CLI failed')
        summary = rows[-1]
        expected_size = len(unit['pieces'])
        if name == 'b00' and expected_size != 33:
            raise AssertionError('b00 unit size changed')
        expected = {'strict_prime': True, 'unit_size': expected_size,
                    'unit_on_lattice': expected_size, 'collateral': 0,
                    'others': 1193 - expected_size, 'others_exact': 1193 - expected_size,
                    'others_landed': 0, 'off_before': 1193, 'off_after': 1193 - expected_size}
        for key, value in expected.items():
            if summary[key] != value:
                raise AssertionError(f'{name}: {key} was {summary[key]}, expected {value}')
        if vacated != [len(target)]:
            raise AssertionError('missing after-V vacancy observation')
        if rows[0]['length'] != str(cert['V_length']) or rows[2]['length'] != str(cert['V_length']):
            raise AssertionError('SLP lengths were not exact decimal strings')
        final, _, _ = screen.load('table', end_path)  # independently checks the exported table's digest
        if final.digest() != summary['end_digest']:
            raise AssertionError('closure end table differs from the reported state')
        A.print_row({'closure_test': name, 'target_holes': len(target), **summary,
                     'seconds': round(time.monotonic() - start, 3),
                     'peak_rss_kib': resource.getrusage(resource.RUSAGE_SELF).ru_maxrss})


class Application(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.B, cls.screen = A.modules()
        cls.st, cls.M, _ = cls.screen.load('record', str(RECORD))
        if cls.st.digest() != START_DIGEST:
            raise AssertionError('depth-40 record digest changed')
        cls.G = cls.B.Alphabet(SimpleNamespace(st=cls.st), cls.M)
        cls.rotations = A.Rotations(cls.st.ctx.kplus)
        cls.temp = tempfile.TemporaryDirectory(prefix='slp-tests-')
        cls.addClassCleanup(cls.temp.cleanup)
        cls.directory = Path(cls.temp.name)
        # Choose a deterministically, requiring an OCCUPIED, chamber-fixed cap
        # point. b and c have overlapping caps and noncommuting rotations with a.
        G, mul = cls.G, cls.rotations.mul
        occupied = np.ones(G.NP, bool)
        occupied[cls.B.hole_set(cls.st)] = False
        for n, (c, i, k, ys) in enumerate(G.letters):
            xs = G.capx[c]
            fixed = xs[(xs == ys) & occupied[xs]]
            if len(fixed) and k != 0:
                cls.a_number, cls.fixed_chamber = n, int(fixed[0])
                break
        else:
            raise AssertionError('no occupied cap chamber with nontrivial stabilizer was exercised')
        chosen = [cls.a_number]
        ca, _, ka, _ = G.letters[cls.a_number]
        for n, (c, _, k, _) in enumerate(G.letters):
            if c in {G.letters[m][0] for m in chosen}:
                continue
            if G.home[G.capx[ca], c].any() and mul[ka, k] != mul[k, ka]:
                chosen.append(n)
                if len(chosen) == 3:
                    break
        if len(chosen) != 3:
            raise AssertionError('could not choose three overlapping noncommuting SAFE letters')
        cls.a, cls.b, cls.c = G.pairs(chosen)
        cls.leaves = [letter(p) for p in (cls.a, cls.b, cls.c)]
        cls.order_a = int(cls.rotations.orders[ka])
        A.print_row({'controls': [cls.a, cls.b, cls.c], 'holonomy_chamber': cls.fixed_chamber,
                     'holonomy_rotation': ka, 'rotation_order': cls.order_a})

    def evaluated(self, cert, node='V'):
        ev = A.Evaluator(self.G, self.rotations, cert, node)
        value = ev.evaluate()
        result = clone(self.st)
        A.apply_labelled(result, self.G, self.rotations, value)
        return result, value

    def agreement(self, cert, word, node='V'):
        expected = clone(self.st)
        replay(expected, self.M, word)
        result, _ = self.evaluated(cert, node)
        self.assertEqual(result.digest(), expected.digest())

    def test_01_literal_agreement(self):
        self.agreement(certificate(self.leaves + [{'op': 'cat', 'args': [0, 1, 2]}]),
                       [self.a, self.b, self.c])
        self.agreement(certificate(self.leaves + [{'op': 'cat', 'args': [0, 1]}, {'op': 'inv', 'arg': 3}]),
                       A.inverse_word(self.M, [self.a, self.b]), node=4)
        # Incremental literal replay exercises every prefix w^e once, rather
        # than replaying the same first e-1 copies for each new exponent.
        literal = clone(self.st)
        for exponent in range(41):
            if exponent:
                replay(literal, self.M, [self.a, self.b])
            cert = certificate(self.leaves + [{'op': 'cat', 'args': [0, 1]},
                                              {'op': 'pow', 'arg': 3, 'exp': exponent}])
            with self.subTest(exponent=exponent):
                result, _ = self.evaluated(cert)
                self.assertEqual(result.digest(), literal.digest())
        exponent = 3 * self.order_a + 1
        self.agreement(certificate(self.leaves + [{'op': 'pow', 'arg': 0, 'exp': exponent}]),
                       [self.a] * exponent)
        nested = certificate(self.leaves + [{'op': 'cat', 'args': [0, 1]},
                                            {'op': 'inv', 'arg': 3},
                                            {'op': 'pow', 'arg': 4, 'exp': 7},
                                            {'op': 'cat', 'args': [5, 2, 0]}])
        self.agreement(nested, A.inverse_word(self.M, [self.a, self.b]) * 7 + [self.c, self.a])
        A.print_row({'item': 1, 'literal_agreement': True, 'two_letter_exponents': '0..40'})

    def test_02_fixed_chamber_holonomy(self):
        literal = clone(self.st)
        x = self.fixed_chamber
        for exponent in range(2 * self.order_a + 2):
            if exponent:
                replay(literal, self.M, [self.a])
            cert = certificate([letter(self.a), {'op': 'pow', 'arg': 0, 'exp': exponent}])
            result, (p, r) = self.evaluated(cert)
            self.assertEqual(int(p[x]), x)
            if exponent == 1:
                self.assertNotEqual(int(r[x]), 0)
                self.assertNotEqual(result.digest(), self.st.digest())
            self.assertEqual(result.digest(), literal.digest())
        A.print_row({'item': 2, 'occupied_fixed_chamber': x, 'nontrivial_holonomy_exercised': True})

    def test_03_large_exponents(self):
        base = self.leaves + [{'op': 'cat', 'args': [0, 1]}]
        ev = A.Evaluator(self.G, self.rotations, certificate(base))
        p, r = ev.evaluate()
        # Independent cycle walk and holonomy-order calculation for the full
        # labelled permutation. In particular N includes fixed-chamber spin.
        mul = self.rotations.mul
        seen, order = np.zeros(self.G.NP, bool), 1
        for start in range(self.G.NP):
            if seen[start]:
                continue
            x, length, h = start, 0, 0
            while not seen[x]:
                seen[x] = True
                length += 1
                h = int(mul[int(r[x]), h])
                x = int(p[x])
            self.assertEqual(x, start)
            power, h_order = h, 1
            while power != 0:
                power = int(mul[h, power])
                h_order += 1
            order = math.lcm(order, length * h_order)
        exponent = 10**60 + 7
        big, big_action = self.evaluated(certificate(base + [{'op': 'pow', 'arg': 3, 'exp': exponent}]))
        reduced, reduced_action = self.evaluated(certificate(base + [{'op': 'pow', 'arg': 3, 'exp': exponent % order}]))
        self.assertEqual(big.digest(), reduced.digest())
        for left, right in zip(big_action, reduced_action):
            np.testing.assert_array_equal(left, right)
        identity, action = self.evaluated(certificate(base + [{'op': 'pow', 'arg': 3, 'exp': order}]))
        self.assertEqual(identity.digest(), self.st.digest())
        np.testing.assert_array_equal(action[0], np.arange(self.G.NP))
        self.assertFalse(action[1].any())
        A.print_row({'item': 3, 'exponent': str(exponent), 'labelled_order': str(order), 'identity_verified': True})

    def test_04_rejections_before_change(self):
        blocked = min(self.G.blocked)
        unsafe = [blocked, self.a[1]]
        bad = certificate([letter(self.a), letter(unsafe), {'op': 'cat', 'args': [0, 1]}])
        path = write_json(self.directory / 'unsafe.json', bad)
        st = clone(self.st)
        before = st.snapshot()
        with self.assertRaisesRegex(ValueError, 'segment 7:.*not a SAFE-ALL letter'):
            A.apply_segment(st, self.M, {'slp': path, 'node': 'V'}, 7, self.rotations)
        self.assertEqual(st.snapshot(), before)
        with self.assertRaisesRegex(ValueError, 'segment 8: state_digest mismatch'):
            A.apply_segment(st, self.M, {'slp': path, 'node': 'V', 'state_digest': '0' * 64}, 8, self.rotations)
        self.assertEqual(st.snapshot(), before)
        # Reachability matters: an unrelated unsafe leaf is not part of V.
        unused = certificate([letter(self.a), letter(unsafe)], root=0)
        self.agreement(unused, [self.a])
        for change, reason in (
                ({'op': 'pow', 'arg': 0, 'exp': -1}, 'nonnegative integer'),
                ({'op': 'pow', 'arg': 0, 'exp': True}, 'nonnegative integer'),
                ({'op': 'pow', 'arg': 0, 'exp': 1.0}, 'nonnegative integer'),
                ({'op': 'inv', 'arg': 1}, 'cyclic, forward'),
                ({'op': 'cat', 'args': [2]}, 'cyclic, forward'),
                ({'op': 'pow', 'arg': 0, 'exp': 3, 'length': 4}, 'false expanded length')):
            doc = {'format': 'r22b10/1', 'nodes': [letter(self.a), change], 'V': 1}
            with self.subTest(node=change), self.assertRaisesRegex(ValueError, reason):
                A.Evaluator(self.G, self.rotations, doc)
        straddled = min(c for c, why in self.G.blocked.items() if why['straddled'])
        with self.assertRaisesRegex(ValueError, 'segment 9: letter 0 .*not admissible'):
            A.apply_segment(st, self.M, {'word': [[straddled, self.a[1]]]}, 9)
        self.assertEqual(st.snapshot(), before)
        A.print_row({'item': 4, 'unsafe_leaf_refused_without_change': True, 'wrong_state_digest_refused': True})

    def test_04b_literal_cli_and_pose_table(self):
        start_path = str(self.directory / 'literal-start.json')
        self.B.export_pose_table(self.st, start_path)
        wfile = write_json(self.directory / 'word.json', {'word': [self.a]})
        core = write_json(self.directory / 'core.json', {'P': [self.a], 'Q': [self.b]})
        program_path = write_json(self.directory / 'literal-program.json', {'segments': [
            {'word': [self.c]}, {'word_file': wfile}, {'core_file': core}]})
        out_path = str(self.directory / 'literal-end.json')
        unit = str(ROOT / 'runs/round22/line-a/units/eb5-k40-k00-b00.json')
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            status = A.main(['table', start_path, program_path, out_path, '--unit', unit])
        self.assertEqual(status, 0, stdout.getvalue())
        rows = [json.loads(line) for line in stdout.getvalue().splitlines()]
        self.assertEqual([row['kind'] for row in rows[:-1]], ['word', 'word_file', 'core_file'])
        self.assertEqual([row['length'] for row in rows[:-1]], [1, 1, 3])
        self.assertIs(rows[-1]['strict_prime'], False)
        expected = clone(self.st)
        replay(expected, self.M, [self.c, self.a, self.a, self.b] + A.inverse_word(self.M, [self.a]))
        loaded, _, _ = self.screen.load('table', out_path)
        self.assertEqual(loaded.digest(), expected.digest())
        self.assertEqual(rows[-1]['end_digest'], loaded.digest())

    def closure(self, name):
        remaining = 1190 - (time.monotonic() - T0)
        result = subprocess.run([sys.executable, '-I', '-B', str(Path(__file__).resolve()), '--closure', name],
                                capture_output=True, text=True, timeout=remaining)
        self.assertEqual(result.returncode, 0, result.stdout + '\n' + result.stderr)
        rows = [json.loads(line) for line in result.stdout.splitlines()]
        evidence = rows[-1]
        self.assertEqual(evidence['closure_test'], name)
        self.assertIs(evidence['strict_prime'], True)
        self.assertLess(evidence['peak_rss_kib'], 4 * 1024**2)
        # Sum of separate peak RSS values is a conservative bound on concurrent
        # parent+worker memory. Each process uses the same single CPU affinity.
        combined = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss + evidence['peak_rss_kib']
        self.assertLess(combined, 4 * 1024**2)
        A.print_row({'item': 5 if name == 'b00' else 6, **evidence, 'combined_peak_bound_kib': combined})

    def test_05_b00_closure(self):
        self.closure('b00')

    def test_06_b19_closure_if_budget_allows(self):
        if time.monotonic() - T0 > 900:
            A.print_row({'item': 6, 'skipped': 'less than five minutes left in acceptance budget'})
            self.skipTest('optional b19 closure would risk the twenty-minute budget')
        self.closure('b19')


def main():
    if len(sys.argv) == 3 and sys.argv[1] == '--closure':
        closure_worker(sys.argv[2])
        return 0
    result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Application))
    seconds = time.monotonic() - T0
    A.print_row({'acceptance_seconds': round(seconds, 3), 'one_core': True,
                 'parent_peak_rss_kib': resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
                 'passed': result.wasSuccessful() and seconds < 1200})
    return 0 if result.wasSuccessful() and seconds < 1200 else 1


if __name__ == '__main__':
    sys.exit(main())
