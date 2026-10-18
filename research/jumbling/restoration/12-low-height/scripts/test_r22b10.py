# Synthetic exact acceptance and adversarial controls for E22-1b / R22B-10.
# No census or native fixture is run. All output files live in a temporary dir.
# Run from the repository root (well below the packet's ten-minute limit):
#   python3 -I -B research/jumbling/restoration/12-low-height/scripts/test_r22b10.py
# The checker has no dependency on either certificate builder. Replays below
# expand only SMALL synthetic words and use this alphabet's own permutations.
import ast
import copy
import contextlib
import gzip
import hashlib
import itertools
import io
import json
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest.mock import patch
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import r22b10_cert as C
import check_r22b10 as K


def cycle_perm(n, cycles):
    p = np.arange(n)
    for xs in cycles:
        p[list(xs)] = list(xs[1:]) + list(xs[:1])
    return p


class Alphabet:
    """Small independently composed SAFE permutations, with computed orbits."""
    def __init__(self, generators, holes):
        self.NP = len(generators[0])
        self.letters, self.maps, self.inv, self.capx, self.by_grip = [], [], [], {}, {}
        for g, p in enumerate(generators):
            xs = np.flatnonzero(p != np.arange(self.NP))
            self.capx[g] = xs
            self.by_grip[g] = []
            start = len(self.letters)
            for item, q in enumerate((p, np.argsort(p))):
                self.by_grip[g].append(len(self.letters))
                self.letters.append((g, item, None, q[xs]))
                self.maps.append(q)
            self.inv.extend([start + 1, start])
        self.inv = np.array(self.inv)
        self.index = {(g, item): n for n, (g, item, _, _) in enumerate(self.letters)}
        self.occupied = np.ones(self.NP, bool)
        self.occupied[holes] = False
        self.holes = np.flatnonzero(~self.occupied)
        self.state_digest = hashlib.sha256(json.dumps([p.tolist() for p in generators] + [self.holes.tolist()]).encode()).hexdigest()
        self.orbits()

    def images(self, word, points):
        xs = np.array(points, np.int64)
        for n in word:
            xs = self.maps[n][xs]
        return xs

    def perm(self, word):
        return self.images(word, np.arange(self.NP))

    def pairs(self, word):
        return [[int(self.letters[n][0]), int(self.letters[n][1])] for n in word]

    def numbers(self, pairs):
        return [self.index[tuple(pair)] for pair in pairs]

    def permutation(self, pairs):
        return self.perm(self.numbers(pairs))

    def orbits(self):
        unseen, domains = set(range(self.NP)), []
        while unseen:
            root = min(unseen)
            seen, todo = {root}, [root]
            while todo:
                x = todo.pop()
                for p in self.maps:
                    y = int(p[x])
                    if y not in seen:
                        seen.add(y)
                        todo.append(y)
            domains.append(sorted(seen))
            unseen -= seen
        self.orbit = np.empty(self.NP, np.int64)
        for j, xs in enumerate(domains):
            self.orbit[xs] = j
        self.orbit_size = np.array([len(xs) for xs in domains])
        self.orbit_least = np.array([xs[0] for xs in domains])
        return self.orbit, self.orbit_size


def coupled():
    triples = list(itertools.combinations(range(7), 3))
    generators = [cycle_perm(14, [t, tuple(x + 7 for x in t)]) for t in triples]
    generators += [cycle_perm(14, [tuple(x + 7 for x in t)]) for t in triples]
    return Alphabet(generators, [3, 4, 5, 10, 12, 13])


def nine():
    r = cycle_perm(9, [tuple(range(9))])
    t = cycle_perm(9, [(0, 1, 2)])
    return Alphabet([r, t[r]], [5, 6, 7])


def jordan_alphabet():
    # The foreign factor tests GLOBAL isolation, a nontrivial exact M, and
    # cancellation of h's collateral action in the final Jordan commutator.
    r = cycle_perm(16, [tuple(range(9))])
    t = cycle_perm(16, [(0, 1, 2), (9, 10, 11)])
    w = cycle_perm(16, [(0, 1, 2, 3, 4), tuple(range(9, 16))])
    return Alphabet([r, t, w], [5, 6, 7, 10])


def target(G, T):
    return dict(state_digest=G.state_digest, T=list(T),
                T_digest=hashlib.sha256(json.dumps(sorted(T)).encode()).hexdigest())


def expand(G, nodes, root, limit=500000):
    """Separate literal replay on small certificates, not used by the checker."""
    memo, todo = {}, [(root, False)]
    while todo:
        n, done = todo.pop()
        if n in memo:
            continue
        node = nodes[n]
        if not done:
            todo.append((n, True))
            refs = node['args'] if node['op'] == 'cat' else [node['arg']] if 'arg' in node else []
            todo.extend((x, False) for x in refs)
            continue
        if node['op'] == 'letter':
            word = [G.index[(node['grip'], node['item'])]]
        elif node['op'] == 'cat':
            word = [x for child in node['args'] for x in memo[child]]
        elif node['op'] == 'inv':
            word = [int(G.inv[x]) for x in reversed(memo[node['arg']])]
        else:
            if len(memo[node['arg']]) * node['exp'] > limit:
                raise ValueError('synthetic literal replay exceeds its limit')
            word = memo[node['arg']] * node['exp']
        if len(word) > limit:
            raise ValueError('synthetic literal replay exceeds its limit')
        memo[n] = word
    return memo[root]


def append_node(doc, op, **args):
    node = dict(op=op, **args)
    if op == 'letter':
        length = 1
    elif op == 'cat':
        length = sum(doc['nodes'][n]['length'] for n in args['args'])
    else:
        length = doc['nodes'][args['arg']]['length'] * (args['exp'] if op == 'pow' else 1)
    doc['nodes'].append(dict(node, length=length))
    return len(doc['nodes']) - 1


def append_word(G, doc, word):
    ids = [append_node(doc, 'letter', grip=G.pairs([n])[0][0], item=G.pairs([n])[0][1]) for n in word]
    return append_node(doc, 'cat', args=ids)


def update_bound(doc):
    doc['bound'] = 2 * sum(row['k'] * (row['s'] + 2 * row['e'] + 2 * row['f']) for row in doc['orbits'])


class Certificates(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.examples = []
        for name, G, T, forced in (
                ('coupled commutators', coupled(), [0, 1, 2, 7, 8, 9], False),
                ('nine-point word tables', nine(), [0, 1, 2], False),
                ('forced global Jordan', jordan_alphabet(), [0, 1, 2], True)):
            tgt = target(G, T)
            doc = C.build(G, tgt, seconds=45, force_jordan=forced)
            if not doc['complete']:
                raise AssertionError(f'{name}: {doc["missing"]}')
            cls.examples.append((name, G, tgt, doc))

    def reject(self, G, tgt, doc, reason):
        with self.assertRaisesRegex(ValueError, reason):
            K.check(G, tgt, doc)

    def test_positive_builds_independent_check_and_literal_replay(self):
        for name, G, tgt, doc in self.examples:
            with self.subTest(case=name):
                summary = K.check(G, tgt, doc)
                self.assertTrue(summary['complete'])
                literal = expand(G, doc['nodes'], doc['V'])
                self.assertEqual(len(literal), doc['V_length'])
                final = np.empty(G.NP, bool)
                final[G.perm(literal)] = G.occupied
                self.assertFalse(final[tgt['T']].any())
                self.assertLessEqual(doc['V_length'], doc['bound'])
                self.assertEqual(len(doc['missing']), 0)
        _, G, _, doc = self.examples[0]
        self.assertTrue(all(row['route'] == 'commutator' for row in doc['orbits']))
        word = expand(G, doc['nodes'], doc['orbits'][0]['seed'])
        self.assertFalse(np.array_equal(G.images(word, range(7, 14)), np.arange(7, 14)))
        _, G, _, doc = self.examples[1]
        for p in G.maps:
            self.assertTrue((p != np.arange(9)).all(), 'R22B-06 letters must all be derangements')
        self.assertTrue(all(doc['nodes'][entry[2]]['length'] > 1 for row in doc['orbits'] for key in ('P', 'Q') for entry in row[key]))
        _, G, _, doc = self.examples[2]
        self.assertEqual(doc['orbits'][0]['route'], 'jordan')
        self.assertGreaterEqual(doc['orbits'][0]['size'], 8)
        self.assertGreater(doc['orbits'][0]['M'], 1)
        row = doc['orbits'][0]
        hp = G.perm(expand(G, doc['nodes'], row['h']))
        self.assertFalse(np.array_equal(hp[9:], np.arange(9, 16)))
        seed = G.perm(expand(G, doc['nodes'], row['seed']))
        np.testing.assert_array_equal(seed[9:], np.arange(9, 16))

    def test_all_ordered_three_cycles_on_word_tables(self):
        _, G, tgt, doc = self.examples[1]
        # Rebuild tree path nodes from the public certificate; exercise the
        # builder's constructor for ALL ordered triples, including the anchor.
        P = C.Program()
        mapping = {}
        for j, node in enumerate(doc['nodes']):
            if node['op'] == 'letter':
                new = P.letter(G, G.index[(node['grip'], node['item'])])
            elif node['op'] == 'cat':
                new = P.cat([mapping[x] for x in node['args']])
            elif node['op'] == 'inv':
                new = P.inv(mapping[node['arg']])
            else:
                new = P.pow(mapping[node['arg']], node['exp'])
            mapping[j] = new
        row = dict(doc['orbits'][0])
        row['seed'] = mapping[row['seed']]
        a, b, c = row['anchors']
        pathsP, pathsQ = {b: P.empty}, {c: P.empty}
        ptable = {x: (parent, mapping[n]) for x, parent, n in row['P']}
        qtable = {child: (parent, mapping[n]) for parent, child, n in row['Q']}
        for table, paths, reverse in ((ptable, pathsP, True), (qtable, pathsQ, False)):
            for x in table:
                todo, y = [], x
                while y not in paths:
                    todo.append(y)
                    y = table[y][0]
                for y in reversed(todo):
                    parent, edge = table[y]
                    paths[y] = P.cat([paths[parent], P.inv(edge) if reverse else edge])
        B, Q = type('Paths', (), {})(), type('Paths', (), {})()
        B.paths = [pathsP.get(x) for x in range(9)]
        Q.paths = [pathsQ.get(x) for x in range(9)]
        action = C.Action(G, np.arange(9), P, time.monotonic() + 10)
        for xyz in itertools.permutations(range(9), 3):
            ident = C.three_cycle(action, row, B, Q, *xyz)
            expected = np.arange(9)
            expected[list(xyz)] = [xyz[1], xyz[2], xyz[0]]
            np.testing.assert_array_equal(action.eval(ident), expected)

    def test_control_completed_edge_moves_anchor(self):
        _, G, tgt, original = self.examples[1]
        doc = copy.deepcopy(original)
        row = doc['orbits'][0]
        row['P'][0][2] = append_word(G, doc, [0])  # every leaf moves the anchor
        self.reject(G, tgt, doc, 'completed edge word moves a fixed anchor')

    def broken_conjugation(self, reverse_q=False):
        _, G, tgt, original = self.examples[1]
        row = original['orbits'][0]
        a, b, c = row['anchors']
        P = {child: (parent, n) for child, parent, n in row['P']}
        Q = {child: (parent, n) for parent, child, n in row['Q']}
        for x, y in itertools.permutations([v for v in range(9) if v != a], 2):
            px, v = [], x
            while v != b:
                v, n = P[v]
                px += expand(G, original['nodes'], n)
            w = int(G.images(px, [y])[0])
            blocks, v = [], w
            while v != c:
                v, n = Q[v]
                blocks.append(expand(G, original['nodes'], n))
            correct_q = [n for block in reversed(blocks) for n in block]
            if reverse_q:
                # The bug: reverse the flattened letters, rather than the
                # chronological edge blocks, on the root-to-child Q path.
                wrong_q = [n for block in blocks for n in block][::-1]
                if wrong_q == correct_q:
                    continue
                uword = px + [int(G.inv[n]) for n in wrong_q[::-1]]
            else:
                uword = px + [int(G.inv[n]) for n in correct_q[::-1]]
            doc = copy.deepcopy(original)
            u = append_word(G, doc, uword)
            ui = append_node(doc, 'inv', arg=u)
            node = append_node(doc, 'cat', args=([u, row['seed'], ui] if reverse_q else [ui, row['seed'], u]))
            expected = np.arange(9)
            expected[[a, x, y]] = [x, y, a]
            actual = G.perm(expand(G, doc['nodes'], node))
            if np.array_equal(actual, expected):
                continue
            doc['steps'][0] = dict(orbit=0, cycle=[a, x, y], node=node)
            return G, tgt, doc
        self.fail('no nontrivial conjugation/Q reversal control was constructed')

    def test_control_wrong_conjugation_direction(self):
        G, tgt, doc = self.broken_conjugation()
        self.reject(G, tgt, doc, 'step is not its stated three-cycle')

    def test_control_q_reversed_letter_by_letter(self):
        G, tgt, doc = self.broken_conjugation(reverse_q=True)
        self.reject(G, tgt, doc, 'step is not its stated three-cycle')

    def test_control_non_safe_leaf(self):
        _, G, tgt, original = self.examples[1]
        doc = copy.deepcopy(original)
        append_node(doc, 'letter', grip=99999, item=0)  # even unused leaves are checked
        self.reject(G, tgt, doc, 'non-SAFE')

    def test_controls_cyclic_forward_and_bad_exponents(self):
        _, G, tgt, original = self.examples[1]
        for change, reason in ((lambda doc, n: dict(op='inv', arg=n, length=1), 'cyclic, forward'),
                               (lambda doc, n: dict(op='cat', args=[n + 1], length=1), 'cyclic, forward'),
                               (lambda doc, n: dict(op='pow', arg=0, exp=-1, length=0), 'nonnegative integer'),
                               (lambda doc, n: dict(op='pow', arg=0, exp=1.5, length=0), 'nonnegative integer'),
                               (lambda doc, n: dict(op='pow', arg=0, exp=True, length=0), 'nonnegative integer')):
            doc = copy.deepcopy(original)
            doc['nodes'].append(change(doc, len(doc['nodes'])))
            self.reject(G, tgt, doc, reason)

    def test_giant_power_exact_length_and_false_power_length_control(self):
        _, G, tgt, original = self.examples[1]
        doc = copy.deepcopy(original)
        row = doc['orbits'][0]
        exponent = 3 * 10**350 + 1  # same three-cycle, without materialising it
        row['seed'] = append_node(doc, 'pow', arg=row['seed'], exp=exponent)
        row['s'] = doc['nodes'][row['seed']]['length']
        update_bound(doc)
        # Include decimal JSON round-trip; all lengths remain Python integers.
        doc = json.loads(json.dumps(doc))
        self.assertTrue(K.check(G, tgt, doc)['complete'])
        doc['nodes'][doc['orbits'][0]['seed']]['length'] += 1
        self.reject(G, tgt, doc, 'false expanded length')

    def test_control_seed_moves_earlier_target_orbit(self):
        _, G, tgt, original = self.examples[0]
        doc = copy.deepcopy(original)
        first, second = doc['orbits']
        second['seed'], second['s'] = first['seed'], first['s']
        p = G.perm(expand(G, doc['nodes'], first['seed']))
        moved = [x for x in range(7, 14) if p[x] != x]
        self.assertEqual(len(moved), 3)
        a = min(moved)
        second['anchors'] = [a, int(p[a]), int(p[p[a]])]
        update_bound(doc)
        self.reject(G, tgt, doc, 'seed moves an earlier target orbit')

    def test_control_v_leaves_target_occupied(self):
        _, G, tgt, original = self.examples[1]
        doc = copy.deepcopy(original)
        doc['V'] = append_node(doc, 'cat', args=[])
        doc['V_length'] = 0
        self.reject(G, tgt, doc, 'V leaves a target occupied')

    def test_control_bound_below_v_length(self):
        _, G, tgt, original = self.examples[1]
        doc = copy.deepcopy(original)
        doc['bound'] = doc['V_length'] - 1
        self.reject(G, tgt, doc, 'bound')

    def test_controls_wrong_state_and_target_digests(self):
        _, G, tgt, original = self.examples[1]
        for key, reason in (('state_digest', 'state digest'), ('T_digest', 'target digest')):
            doc = copy.deepcopy(original)
            doc[key] = '0' * 64
            self.reject(G, tgt, doc, reason)
        wrong = dict(tgt, T_digest='0' * 64)
        self.reject(G, wrong, original, 'target digest')
        with self.assertRaisesRegex(ValueError, 'target digest'):
            C.build(G, wrong)
        with self.assertRaisesRegex(ValueError, 'state digest'):
            C.build(G, dict(tgt, state_digest='0' * 64))

    def test_controls_inventory_target_order_and_path_cost(self):
        _, G, tgt, original = self.examples[0]
        for T in ([0, 0], [-1], [G.NP]):
            bad = target(G, T)
            doc = dict(original, T_digest=bad['T_digest'], T_size=len(T))
            self.reject(G, bad, doc, 'duplicated or out of range')
        doc = copy.deepcopy(original)
        doc['orbits'].reverse()
        self.reject(G, tgt, doc, 'orbit order')
        doc = copy.deepcopy(original)
        doc['orbits'][0]['e'] += 1
        self.reject(G, tgt, doc, 'path cost')
        doc = copy.deepcopy(original)
        doc['orbits'][0]['P'].pop()
        self.reject(G, tgt, doc, 'cover every required')
        _, J, jt, jc = self.examples[2]
        doc = copy.deepcopy(jc)
        doc['orbits'][0]['M'] += 1
        self.reject(J, jt, doc, 'global lcm')
        bad = target(G, list(range(4)))
        self.reject(G, bad, dict(original, T_digest=bad['T_digest'], T_size=4), 'N2')

    def test_diagonal_a5_missing_second_orbit_is_not_nonexistence_claim(self):
        gens = [cycle_perm(10, [t, tuple(x + 5 for x in t)]) for t in itertools.combinations(range(5), 3)]
        G = Alphabet(gens, [1, 7])
        tgt = target(G, [1, 6])
        doc = C.build(G, tgt, seconds=2)
        self.assertFalse(doc['complete'])
        self.assertEqual([row['least'] for row in doc['orbits']], [0])
        self.assertEqual([row['least'] for row in doc['missing']], [5])
        self.assertIn('seed', doc['missing'][0]['reason'])
        self.reject(G, tgt, doc, 'incomplete certificate')

    def test_reproducibility_and_deadline_missing_reasons(self):
        _, G, tgt, original = self.examples[1]
        repeat = C.build(G, tgt, seconds=45)
        self.assertEqual({k: v for k, v in repeat.items() if k != 'seconds'},
                         {k: v for k, v in original.items() if k != 'seconds'})
        doc = C.build(G, tgt, deadline=time.monotonic() - 1)
        self.assertFalse(doc['complete'])
        self.assertEqual([x['least'] for x in doc['missing']], [0])
        self.assertIn('budget', doc['missing'][0]['reason'])

    def test_singleton_holes_and_n1_failure(self):
        G = Alphabet([cycle_perm(6, [(0, 1, 2, 3, 4)])], [5])
        tgt = target(G, [5])
        doc = C.build(G, tgt, seconds=1)
        self.assertTrue(K.check(G, tgt, doc)['complete'])
        G.occupied[5], G.holes = True, np.array([], np.int64)
        doc['state_digest'] = tgt['state_digest'] = G.state_digest
        self.reject(G, tgt, doc, 'N1')

    def test_checker_import_boundary(self):
        parsed = ast.parse((HERE / 'check_r22b10.py').read_text())
        imports = []
        for node in ast.walk(parsed):
            if isinstance(node, ast.Import):
                imports.extend(x.name.split('.')[0] for x in node.names)
            elif isinstance(node, ast.ImportFrom):
                imports.append(node.module.split('.')[0])
        allowed = {'argparse', 'gzip', 'hashlib', 'json', 'math', 'sys', 'collections', 'pathlib', 'types', 'numpy', 'buffer', 'screen'}
        self.assertLessEqual(set(imports), allowed)
        self.assertNotIn('r22b10_cert', imports)
        self.assertNotIn('r22b01_cert', imports)

    def test_duck_api_accepts_explicit_state_digest(self):
        G = nine()
        tgt = target(G, [0, 1, 2])
        actual = G.state_digest
        del G.state_digest
        doc = C.build(G, tgt, state_digest=actual, seconds=2)
        self.assertTrue(K.check(G, tgt, doc, state_digest=actual)['complete'])
        with self.assertRaisesRegex(ValueError, 'state digest'):
            K.check(G, tgt, doc, state_digest='0' * 64)

    def test_cli_modes_gzip_and_loading_deadline(self):
        # Use real CLI parsers/serialization and the real independent checker;
        # replace only expensive geometry loading with a synthetic alphabet.
        prefix = (
            'import sys, types, time; from pathlib import Path; '
            f'sys.path.insert(0, {str(HERE)!r}); '
            'import test_r22b10 as F; '
            'G=F.nine(); B=F.K.buffer_module(); B.Alphabet=lambda *a:G; '
            'st=types.SimpleNamespace(digest=lambda:G.state_digest, pose_id=list(range(G.NP))); '
            'screen=types.ModuleType("screen"); '
        )
        with tempfile.TemporaryDirectory() as directory:
            tmp = Path(directory)
            tgtfile, outfile = tmp / 'target.json', tmp / 'certificate.json.gz'
            tgtfile.write_text(json.dumps(self.examples[1][2]))
            for mode in ('record', 'table'):
                stub = f'screen.load=lambda mode,path: (st,None,{{}}) if mode=={mode!r} else 1/0; sys.modules["screen"]=screen; '
                build = prefix + stub + f'sys.exit(F.C.main([{mode!r},"synthetic",{str(tgtfile)!r},{str(outfile)!r},"--seconds","45"]))'
                run = subprocess.run([sys.executable, '-I', '-B', '-c', build], capture_output=True, text=True, timeout=60)
                self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
                with gzip.open(outfile, 'rt') as stream:
                    self.assertTrue(json.load(stream)['complete'])
                check = prefix + stub + f'sys.exit(F.K.main([{mode!r},"synthetic",{str(tgtfile)!r},{str(outfile)!r}]))'
                run = subprocess.run([sys.executable, '-I', '-B', '-c', check], capture_output=True, text=True, timeout=60)
                self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
                self.assertEqual(run.stdout.splitlines()[-1], 'PASS')
                self.assertEqual(len(run.stdout.splitlines()), 2)
            # Imports are already complete here. Time the loader itself, rather
            # than conflating an extra process's geometry imports with it.
            screen = types.ModuleType('screen')
            screen.load = lambda *args: time.sleep(2)
            start = time.monotonic()
            with patch.dict(sys.modules, {'screen': screen}), patch.object(C, 'T0', start), contextlib.redirect_stdout(io.StringIO()):
                status = C.main(['table', 'synthetic', str(tgtfile), str(outfile), '--seconds', '0.3'])
            self.assertEqual(status, 0)
            self.assertLess(time.monotonic() - start, 1.5)
            with gzip.open(outfile, 'rt') as stream:
                doc = json.load(stream)
            self.assertFalse(doc['complete'])
            self.assertIn('state loading', doc['missing'][0]['reason'])
            # Serialization shares the budget and recovers a complete JSON
            # record, with every known orbit reported missing if the DAG is lost.
            G = self.examples[1][1]
            st = types.SimpleNamespace(digest=lambda: G.state_digest, pose_id=list(range(G.NP)))
            screen.load = lambda *args: (st, None, {})
            real_write = C.write_result
            calls = []

            def slow_first_write(path, result):
                calls.append(path)
                if len(calls) == 1:
                    time.sleep(2)
                real_write(path, result)

            start = time.monotonic()
            with patch.dict(sys.modules, {'screen': screen}), patch.object(C, 'T0', start), \
                    patch.object(C.modules(), 'Alphabet', lambda *args: G), \
                    patch.object(C, 'write_result', slow_first_write), contextlib.redirect_stdout(io.StringIO()):
                status = C.main(['record', 'synthetic', str(tgtfile), str(outfile), '--seconds', '0.3'])
            self.assertEqual(status, 0)
            self.assertEqual(len(calls), 2)
            self.assertLess(time.monotonic() - start, 1.5)
            with gzip.open(outfile, 'rt') as stream:
                doc = json.load(stream)
            self.assertFalse(doc['complete'])
            self.assertEqual([row['least'] for row in doc['missing']], [0])
            self.assertIn('serialization', doc['missing'][0]['reason'])
            # A budget already consumed by startup still leaves valid JSON.
            with patch.object(C, 'T0', time.monotonic() - 1), contextlib.redirect_stdout(io.StringIO()):
                status = C.main(['record', 'unused', str(tgtfile), str(outfile), '--seconds', '0.00001'])
            self.assertEqual(status, 0)
            with gzip.open(outfile, 'rt') as stream:
                self.assertFalse(json.load(stream)['complete'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
