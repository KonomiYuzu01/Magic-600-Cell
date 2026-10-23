"""P4 contracts: exact synthetic replays and independent test oracles, headless.

Run from the repository root with python3 -I -B. No runner or rule program is
executed, and all caches are temporary. The final check includes ignored files.
These are source/fixture checks, not Windows, DirectX or performance evidence.
"""

import ast
from collections import Counter
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[_name] = '1'
REPO = Path(__file__).resolve().parents[5]
HERE = Path(__file__).resolve().parent
J = REPO / 'research/jumbling'
K = J / 'restoration/12-low-height/scripts/round22'
TEMP = Path(tempfile.mkdtemp(prefix='check-contracts-'))
os.environ['JUMBLING_HOME_CACHE'] = str(TEMP / 'home_centroids.npy')
sys.path[:0] = [str(HERE), str(J / 'theory'), str(J), str(J / 'fixtures'), str(K)]

import numpy as np  # noqa: E402
import check_contracts as C  # noqa: E402
import r22d25_state as G  # noqa: E402
import eb6 as E  # noqa: E402 -- test oracle only
import restore  # noqa: E402
import sim  # noqa: E402
import trees  # noqa: E402

SOLVED = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
Z36 = '8733a0aafd8e24ea1da990ebab0347074c20b11cf61d9ed04d34d25ef6f67261'
END36 = 'd79c03679e3b065164219ebd747cc2de89bc5759c081f7b95e9554cbb8c11ea1'
A_DIGEST = '7cd868f6466953eb6ea1e0d466ed66857d34edaea7dd7c554c1e74975005541b'
B_DIGEST = '3ab25d214b6c4ad759ac7158a0fc1cabae1584aa711d3f757af7b91c43971837'
Z7102 = '4ca440085d743d680163a35d735ff6a7c35796a57cba720ab4037a49887734db'
WORD36 = [[0, 12], [74, 8], [188, 4], [74, 4], [188, 8], [188, 4], [188, 4],
          [74, 8], [188, 8], [74, 4], [188, 8], [178, 1], [188, 4], [74, 8],
          [188, 4], [74, 4], [188, 8], [188, 8], [188, 4], [74, 8], [188, 8],
          [74, 4], [178, 1], [0, 45], [14, 12], [12, 12]]
MENU = restore.menu('I_a')
ITEM_LOOKUP = {restore.to_tuple(m): i for i, (_, m) in enumerate(MENU.items)}
INVERSE = [ITEM_LOOKUP[restore.to_tuple(restore.transpose(m))] for _, m in MENU.items]
JUMBLE = restore.jumble_items(MENU)
RETAINED = next(i for i, (_, m) in enumerate(MENU.items)
                if i not in JUMBLE and restore.to_tuple(m) != restore.I4)
_TREE_HEIGHTS, _ORACLE_U = {}, {}


def clone(st):
    out = sim.State(st.ctx, filtered=False, menu=MENU)
    out.pose_id = st.pose_id.copy()
    out._poses = list(st._poses)
    out._lookup = dict(st._lookup)
    return out


def apply(st, word):
    for c, j in word:
        result = st.apply(sim.Twist(st.ctx, c, restore.element(MENU, c, j), 'menu',
                                   {'menu': MENU.name}), journal=False)
        if not result.applied or result.uncertain:
            raise AssertionError(('fixture letter not admitted', c, j, result.status))
    return st


def played(st, word):
    return apply(clone(st), word)


def inverse_word(word):
    return [[c, INVERSE[j]] for c, j in reversed(word)]


def off(st):
    return st._kpose()[st.pose_id] < 0


def heights(st):
    """Test-owned tree heights; no checker or kernel height helper is used."""
    by_pose = []
    for pid, pose in enumerate(st._poses):
        if pose.kidx >= 0:
            by_pose.append(0)
            continue
        matrix = st._pose_matrix(pid)
        if matrix not in _TREE_HEIGHTS:
            _TREE_HEIGHTS[matrix] = int(trees.dist(trees.factors(matrix)[0]))
        by_pose.append(_TREE_HEIGHTS[matrix])
    return np.asarray(by_pose, np.int64)[st.pose_id]


def record(st):
    h = heights(st)
    return {'digest': st.digest(), 'off': int(off(st).sum()),
            'heights': {str(k): int(v) for k, v in sorted(Counter(h[off(st)].tolist()).items())}}


def same_poses(before, after, pieces):
    pairs = np.column_stack((before.pose_id[pieces], after.pose_id[pieces]))
    return all(before._pose_matrix(int(a)) == after._pose_matrix(int(b))
               for a, b in np.unique(pairs, axis=0))


def oracle_u(st, tau):
    if tau is None or not off(st).any():
        return 0
    key = st.digest(), tau
    if key not in _ORACLE_U:
        _ORACLE_U[key] = len(E.straddler_set(st, tau))
    return _ORACLE_U[key]


def hook(fn, st, *args, state_first=False):
    """Every hook must preserve the digest and all three mechanical containers."""
    digest, ids, poses, lookup = st.digest(), st.pose_id.copy(), list(st._poses), dict(st._lookup)
    result = fn(st, *args) if state_first else fn(*args, st)
    assert st.digest() == digest, (fn.__name__, 'digest changed')
    np.testing.assert_array_equal(st.pose_id, ids, err_msg=fn.__name__)
    assert st._poses == poses and st._lookup == lookup, (fn.__name__, 'pose table changed')
    return result


def run_row(row, start_state, word, observe=True):
    st = clone(start_state)
    ctx = hook(C.prepare, st, row)
    for k, letter in enumerate(word):
        apply(st, [letter])
        if observe:
            hook(C.on_letter, st, ctx, k)
    failures = hook(C.finish, st, ctx, row)
    assert isinstance(failures, list) and all(isinstance(n, str) and n for n in failures), failures
    assert len(failures) == len(set(failures)), failures
    assert C.UNCHECKED not in failures, failures
    return failures


def core_row(phase, rule, subphase, contract, gate, start, end, word):
    return {'index': 0, 'phase': phase, 'subphase': subphase, 'rule': rule,
            'word': 'words/fixture.json', 'letters': len(word), 'encoding': 'json',
            'word_sha256': '0' * 64, 'tree': {}, 'contract': contract, 'gate': gate,
            'standing': 'rule', 'before': record(start), 'after': record(end),
            'table': None, 'attempts': {}, 'seconds': 0}


def height_case(rule, start, word, tau_before=None, tau_after=None):
    end = played(start, word)
    subphase = 'r22d81' if rule.startswith('r22d88/') else rule.split('/')[0]
    row = core_row('height', rule, subphase, rule, subphase, start, end, word)
    row.update(tau_before=tau_before, tau_after=tau_after,
               u_before=None if subphase == 'r22d75' else oracle_u(start, tau_before),
               u_after=None if subphase == 'r22d75' else oracle_u(end, tau_after),
               rule_script='fixture.py')
    if len(word) == 2:
        row['u_mid'] = oracle_u(played(start, word[:1]), tau_before)
    return row, start, word


def kernel_case(family, kind, start, word, target, params=None, high_exact=False):
    end = played(start, word)
    row = core_row('high', 'r22d25/' + family, None, 'kernel_' + kind, 'r22d25', start, end, word)
    row.update(family=family, kind=kind, target=target, params={} if params is None else params,
               high_exact=high_exact, off_before=record(start)['off'], off_after=record(end)['off'],
               heights_after=record(end)['heights'], kernel={})
    return row, start, word


def clean_add(st, excluded=()):
    """Bounded deterministic search, cap then jumble item, for lattice-only motion."""
    old_off, tried = off(st), 0
    for c in range(600):
        if c in excluded:
            continue
        cl = st.classify(c, need_inside=True, stop_at_straddle=True)
        assert not cl.uncertain
        if cl.straddle is not None or old_off[cl.inside].any():
            continue
        for j in JUMBLE:
            tried += 1
            end = played(st, [[c, j]])
            new_off = off(end)
            added = new_off & ~old_off
            if (new_off[old_off].all() and same_poses(st, end, old_off)
                    and added.any() and (heights(end)[added] == 2).all()):
                return [c, j], end
    raise AssertionError(('no clean add within 600 caps and all jumble items', record(st), tried))


def find_tau(before, after, caps, relation):
    observed = []
    for c in sorted(set(int(c) for c in caps)):
        a, b = oracle_u(before, c), oracle_u(after, c)
        observed.append((c, a, b))
        if relation(a, b):
            return c
    raise AssertionError(('no tau in bounded cap set', observed))


def changed(case, **fields):
    row, st, word = case
    return dict(copy.deepcopy(row), **fields), st, word


class ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        started = time.monotonic()
        G.set_cache(TEMP / 'cache')
        cls.s0 = sim.State(restore.ctx, filtered=False, menu=MENU)
        cls.z = played(cls.s0, WORD36)
        cls.end = played(cls.z, [[12, 45]])
        cls.a = played(cls.z, [[12, 14]])
        cls.b = played(cls.a, [[35, 13]])
        source = J / 'restoration/11-scramble-census/runs/round2/ia-k100-s7102.json'
        doc = json.loads(source.read_text())
        assert (len(doc['history']), len(doc['moves'])) == (100, 272)
        cls.seed = played(cls.s0, doc['history'] + doc['moves'])
        for st, digest, count, hist in (
            (cls.s0, SOLVED, 0, {}), (cls.z, Z36, 5800, {'2': 5799, '6': 1}),
            (cls.end, END36, 3098, {'2': 3097, '4': 1}),
            (cls.a, A_DIGEST, 3098, {'2': 3097, '4': 1}), (cls.b, B_DIGEST, 5405, {'2': 5405}),
            (cls.seed, Z7102, 31479, {'2': 31355, '4': 124}),
        ):
            actual = record(st)
            expected = {'digest': digest, 'off': count, 'heights': hist}
            assert actual == expected, ('recorded fixture mismatch; computed', actual, 'expected', expected)
        assert np.flatnonzero(heights(cls.z) >= 4).tolist() == [58388]
        assert np.flatnonzero(heights(cls.a) == 4).tolist() == [58388]
        for st in (cls.z, cls.a):
            cl = st.classify(12, need_inside=True, stop_at_straddle=False)
            assert cl.straddle is None and not cl.uncertain and 58388 in cl.inside
        assert oracle_u(cls.a, 12) == 0
        # Prove the retained-pose counterexamples before relying on them.
        cls.retained = {}
        for name, st in (('Z36', cls.z), ('A', cls.a)):
            end = played(st, [[12, RETAINED]])
            np.testing.assert_array_equal(off(st), off(end))
            np.testing.assert_array_equal(heights(st), heights(end))
            assert st.pose(58388) != end.pose(58388)
            cls.retained[name] = end
        cl = cls.z.classify(12, need_inside=True)
        others = [p for p in cl.inside.tolist() if off(cls.z)[p] and p != 58388]
        assert others and any(cls.z.pose(p) != cls.retained['Z36'].pose(p) for p in others)
        assert (off(cls.b) & ~off(cls.a)).any()

        cls.cases, cls.adds, cls.taus = {}, {}, {}
        for name, st in (('Z36', cls.z), ('A', cls.a)):
            l1, y = clean_add(st)
            l2, y2 = clean_add(y, excluded=(l1[0],))
            tau = find_tau(y, st, restore.NEAR[l1[0]], lambda a, b: a > b)
            pair_tau = find_tau(y2, st, list(restore.NEAR[l1[0]]) + list(restore.NEAR[l2[0]]),
                                lambda a, b: a > b)
            cls.adds[name] = l1, y, l2, y2
            cls.taus[name] = tau, pair_tau
            print(f'{name}: clean adds {l1}, {l2}; single tau {tau} '
                  f'u {oracle_u(y, tau)} -> {oracle_u(st, tau)}; pair tau {pair_tau} '
                  f'u {oracle_u(y2, pair_tau)} -> {oracle_u(st, pair_tau)} '
                  f'mid {oracle_u(y, pair_tau)}', flush=True)
        print(f'Retained non-identity item: {RETAINED}', flush=True)
        lz, yz, lz2, yz2 = cls.adds['Z36']
        la, ya, la2, ya2 = cls.adds['A']
        tz, tpz = cls.taus['Z36']
        ta, tpa = cls.taus['A']
        h6tau = find_tau(cls.z, cls.a, restore.NEAR[12], lambda a, b: a > 0)
        l4tau = find_tau(cls.a, cls.b, restore.NEAR[35], lambda a, b: a > 0)
        cls.cases['r22d75/T0'] = height_case('r22d75/T0', cls.z, [[12, 14]])
        cls.cases['r22d75/T1'] = height_case('r22d75/T1', cls.a, [[35, 13]])
        cls.cases['r22d75/T2'] = height_case('r22d75/T2', yz, inverse_word([lz]))
        cls.cases['r22d81/H6'] = height_case('r22d81/H6', cls.z, [[12, 14]], h6tau, None)
        cls.cases['r22d81/H4'] = height_case('r22d81/H4', cls.a, [[35, 13]])
        for rule in ('r22d81/U', 'r22d88/U'):
            cls.cases[rule] = height_case(rule, yz, inverse_word([lz]), tz, tz)
        cls.cases['r22d81/H2'] = height_case('r22d81/H2', yz, inverse_word([lz]))
        cls.cases['r22d88/U2'] = height_case('r22d88/U2', yz2, inverse_word([lz, lz2]), tpz, tpz)
        cls.cases['h4/L4'] = height_case('h4/L4', cls.a, [[35, 13]], l4tau, None)
        assert oracle_u(cls.a, 35) == 0
        cls.cases['h4/L4-zero'] = height_case('h4/L4', cls.a, [[35, 13]], 35, None)
        for rule in ('h4/U', 'h4/L2'):
            cls.cases[rule] = height_case(rule, ya, inverse_word([la]), ta, ta)
        cls.cases['h4/pair'] = height_case('h4/pair', ya2, inverse_word([la, la2]), tpa, tpa)
        cls.f1 = kernel_case('F1', 'lowering', cls.z, [[12, 45]], [58388], {'height': 6})
        cls.t = int(np.flatnonzero(heights(cls.z) == 2)[0])
        cls.k2 = kernel_case('T1', 'landing', cls.z, inverse_word(WORD36), [cls.t])
        print(f'Other taus: H6 {h6tau}, L4 {l4tau}; L4 real-rule tau 35 (u 0)', flush=True)
        print(f'Fixture setup: {time.monotonic() - started:.3f}s', flush=True)

    def check_case(self, case, expected=None, contains=None):
        failures = run_row(*case)
        if expected is not None:
            self.assertEqual(failures, expected)
        if contains is not None:
            self.assertIn(contains, failures)
        return failures

    def test_01_straddlers(self):
        for cap in (0, 12, 599):
            result = hook(C.straddlers, self.s0, cap, state_first=True)
            self.assertEqual(result.dtype, np.dtype('int64'))
            self.assertEqual(result.size, 0)
        for name, st in (('Z36', self.z), ('Z7102', self.seed)):
            started, union = time.monotonic(), set()
            for cap in range(600):
                own = hook(C.straddlers, st, cap, state_first=True)
                oracle = E.straddler_set(st, cap)
                np.testing.assert_array_equal(own, oracle, err_msg=f'{name} cap {cap}')
                self.assertEqual(own.dtype, np.dtype('int64'))
                self.assertEqual(own.tolist(), sorted(set(own.tolist())))
                union.update(own.tolist())
            print(f'{name}: all 600 straddler sets agree; union {len(union)}; '
                  f'{time.monotonic() - started:.3f}s', flush=True)
        # The checker must not silently take the simulator's optional float path.
        filtered = clone(self.z)
        filtered.filtered = True
        with patch.object(filtered, '_eval', side_effect=AssertionError('filtered path used')):
            np.testing.assert_array_equal(hook(C.straddlers, filtered, 0, state_first=True),
                                          E.straddler_set(self.z, 0))

    def test_02_kernel_lowering(self):
        oracle = G.protected_lowering(clone(self.z), [58388], [[12, 45]], 6)
        self.assertTrue(oracle['ok'], oracle['reason'])
        self.assertEqual((oracle['kind'], oracle['output_digest']), ('lowering', END36))
        row = self.f1[0]
        self.assertEqual((row['off_before'], row['off_after'], row['heights_after']),
                         (5800, 3098, {'2': 3097, '4': 1}))
        self.check_case(self.f1, [])
        self.check_case(changed(self.f1, high_exact=True), [])
        self.check_case(changed(self.f1, params={'height': 6, 'high_exact_pieces': [58388]}), [])
        self.check_case(changed(self.f1, target=[58388, 58388]), [])

    def test_03_kernel_landing(self):
        row, st, word = self.k2
        self.assertEqual(len(word), 26)
        self.assertEqual(played(st, word).digest(), SOLVED)
        self.assertEqual((row['off_before'], row['off_after'], row['heights_after']), (5800, 0, {}))
        oracle = G.strict_step(clone(st), [self.t], word)
        self.assertTrue(oracle['ok'], oracle['reason'])
        self.assertEqual((oracle['kind'], oracle['output_digest']), ('landing', SOLVED))
        self.check_case(self.k2, [])

    def test_04_height_rules(self):
        for name, case in self.cases.items():
            with self.subTest(rule=name):
                self.check_case(case, [])

    def test_05_height_fields(self):
        case = self.cases['r22d81/U']
        row, st, word = case
        self.check_case(changed(case, u_before=row['u_before'] + 1), ['height_u'])
        self.check_case(changed(case, u_after=row['u_after'] + 1), contains='height_u')
        for name in ('r22d88/U2', 'h4/pair'):
            pair = self.cases[name]
            self.check_case(changed(pair, u_mid=pair[0]['u_mid'] + 1), contains='height_u')
        end = played(st, word)
        tau = next(c for c in range(600)
                   if (oracle_u(st, c), oracle_u(end, c)) != (row['u_before'], row['u_after']))
        print(f'U forged-count tau: {tau}; forged tau_after: {(row["tau_before"] + 1) % 600}', flush=True)
        self.check_case(changed(case, tau_before=tau, tau_after=tau), contains='height_u')
        self.check_case(changed(case, tau_after=(row['tau_before'] + 1) % 600), ['height_tau'])
        h6 = self.cases['r22d81/H6']
        tau = find_tau(self.a, self.s0, range(600), lambda a, b: a > 0)
        print(f'H6 forged tau_after: {tau}', flush=True)
        self.check_case(changed(h6, tau_after=tau), contains='height_u')
        for name in ('r22d81/U', 'r22d88/U', 'h4/U', 'h4/L2', 'h4/pair'):
            with self.subTest(null_tau=name):
                self.check_case(changed(self.cases[name], tau_before=None), contains='height_tau')
        for tau in (600, -1, True, '0', [], {}):
            with self.subTest(tau=tau):
                self.check_case(changed(case, tau_before=tau, tau_after=tau), contains='height_tau')
        self.check_case(changed(self.cases['r22d81/H2'], u_before=1), contains='height_u')
        self.check_case(changed(self.cases['r22d75/T0'], tau_before=12), contains='height_fields')
        pair = changed(self.cases['r22d88/U2'])
        del pair[0]['u_mid']
        self.check_case(pair, contains='height_fields')
        self.check_case(changed(case, u_mid=0), contains='height_fields')
        self.check_case(changed(self.cases['h4/U'], u_before=None), contains='height_fields')
        for value in (True, -1, '0', [], {}):
            self.check_case(changed(case, u_before=value), contains='height_fields')

    def test_06_u_progress(self):
        _, st, word = self.cases['r22d81/U']
        end = played(st, word)
        tau = find_tau(st, end, range(600), lambda a, b: a == b)
        flat = height_case('r22d81/U', st, word, tau, tau)
        self.check_case(flat, contains='height_u_progress')
        self.check_case(changed(flat, rule='r22d81/H2', contract='r22d81/H2'), [])
        print(f'u plateau tau: {tau}', flush=True)
        _, st, word = self.cases['h4/L2']
        end = played(st, word)
        for tau in sorted(int(c) for c in restore.NEAR[word[0][0]]):
            if oracle_u(end, tau) > oracle_u(st, tau):
                self.check_case(height_case('h4/L2', st, word, tau, tau), contains='height_u_progress')
                print(f'h4/L2 rising-u tau: {tau}', flush=True)
                break
        else:
            print('SKIP note (test 6): no h4/L2 rising-u tau in NEAR of the clean-add cap.', flush=True)
        for name, rule, st in (('Z36', 'r22d81/H2', self.z), ('A', 'h4/L2', self.a)):
            letter, added, _, _ = self.adds[name]
            self.assertGreater(oracle_u(added, 1), oracle_u(st, 1))
            self.check_case(height_case(rule, st, [letter], 1, 1), contains='height_u_progress')

    def test_07_top_height_poses(self):
        tau = int(restore.NEAR[12][0])
        for rule, st in (('r22d81/U', self.z), ('h4/U', self.a)):
            with self.subTest(rule=rule):
                self.check_case(height_case(rule, st, [[12, RETAINED]], tau, tau), contains='height_poses')
        print(f'Retained-pose test tau: {tau}', flush=True)

    def test_08_middle_guard(self):
        for rule, st, letter in (('r22d88/U2', self.z, [12, 45]), ('h4/pair', self.a, [35, 13])):
            tau = int(restore.NEAR[letter[0]][0])
            word = [letter] + inverse_word([letter])
            with self.subTest(rule=rule):
                self.assertEqual(played(st, word).digest(), st.digest())
                self.check_case(height_case(rule, st, word, tau, tau), contains='height_guard')
            print(f'{rule} middle-guard tau: {tau}', flush=True)
        self.assertIn('height_guard', run_row(*self.cases['r22d88/U2'], observe=False))
        # Keeping the count alone must not hide a changed middle pose.
        tau = int(restore.NEAR[12][0])
        word = [[12, RETAINED], [12, INVERSE[RETAINED]]]
        for rule, st in (('r22d88/U2', self.z), ('h4/pair', self.a)):
            self.check_case(height_case(rule, st, word, tau, tau), contains='height_guard')

    def test_09_other_height_mutations(self):
        self.check_case(height_case('h4/L4', self.z, [[12, 14]]), contains='height_max')
        self.check_case(height_case('h4/L4', self.a, [[12, INVERSE[14]]]), ['height_max'])
        for old, new in (('r22d81/U', 'r22d81/H6'), ('r22d81/H2', 'r22d81/H4'), ('h4/U', 'h4/L4')):
            self.check_case(changed(self.cases[old], rule=new, contract=new), contains='height_counts')
        self.check_case(changed(self.cases['r22d75/T0'], rule='r22d75/T1', contract='r22d75/T1'),
                        contains='height_tier')
        self.check_case(height_case('r22d75/T0', self.a, [[12, INVERSE[14]]]), contains='height_counts')
        case = self.cases['r22d81/U']
        for fields in ({'rule': 'unknown', 'contract': 'unknown'}, {'contract': 'wrong'}, {'subphase': 'h4'}):
            self.check_case(changed(case, **fields), contains='height_rule')
        self.check_case(changed(self.cases['r22d88/U2'], letters=1), contains='height_length')
        self.check_case(changed(case, letters=True), contains='height_length')
        self.check_case(changed(case, phase='tail'), ['contract_phase'])

    def test_10_kernel_mutations(self):
        lattice = int(np.flatnonzero(~off(self.z))[0])
        self.check_case(changed(self.f1, target=[lattice]), contains='kernel_target')
        self.check_case(changed(self.f1, target=[self.t]), contains='kernel_target_height')
        self.check_case(changed(self.f1, kind='landing', contract='kernel_landing'), contains='kernel_target_landed')
        for keep, failure in ((True, 'kernel_high_exact'), ([58388], 'kernel_high_exact_pieces')):
            oracle = G.strict_step(clone(self.z), [self.t], self.k2[2], high_exact=keep)
            self.assertFalse(oracle['ok'])
            self.assertEqual(oracle['reason'], 'high_exact')
            case = changed(self.k2, high_exact=True) if keep is True else changed(
                self.k2, params={'high_exact_pieces': keep})
            self.check_case(case, contains=failure)
        keys0, keys1 = E.stable_pose(self.z), E.stable_pose(self.end)
        q = int(np.flatnonzero(off(self.z) & off(self.end) & (keys0 == keys1))[0])
        q_prime = int(np.flatnonzero(off(self.z) & ~off(self.end))[0])
        self.assertEqual(self.z.pose(q), self.end.pose(q))
        self.check_case(changed(self.f1, params={'height': 6, 'high_exact_pieces': [q]}), [])
        self.check_case(changed(self.f1, params={'height': 6, 'high_exact_pieces': [q_prime]}),
                        contains='kernel_high_exact_pieces')
        retained = kernel_case('F1', 'landing', self.z, [[12, RETAINED]], [58388])
        failures = self.check_case(retained)
        for name in ('kernel_others', 'kernel_progress', 'kernel_target_landed'):
            self.assertIn(name, failures)
        failures = self.check_case(changed(retained, kind='lowering', contract='kernel_lowering'))
        for name in ('kernel_progress', 'kernel_target_height'):
            self.assertIn(name, failures)
        collateral = kernel_case('T1', 'landing', self.a, [[35, 13]], [58388])
        failures = self.check_case(collateral)
        for name in ('kernel_collateral', 'kernel_progress'):
            self.assertIn(name, failures)
        row = self.f1[0]
        for field in ('off_before', 'off_after'):
            self.check_case(changed(self.f1, **{field: row[field] + 1}), ['kernel_counts'])
        hist = dict(row['heights_after'])
        hist['2'] += 1
        self.check_case(changed(self.f1, heights_after=hist), ['kernel_counts'])
        for fields in ({'rule': 'r22d25/F2'}, {'contract': 'kernel_landing'}, {'high_exact': 1},
                       {'family': 'X'}, {'params': []}, {'params': {'high_exact_pieces': [-1]}},
                       {'params': {'high_exact_pieces': [True]}}, {'subphase': 'high'}):
            with self.subTest(fields=fields):
                self.check_case(changed(self.f1, **fields), contains='kernel_fields')
        # The strict gate accepts an ordered set, so repeats and order cannot matter.
        landing = changed(self.k2, target=[58388, self.t, 58388, self.t])
        self.check_case(landing, [])
        # Listed lattice pieces are kept by exact matrix, too.
        moved_lattice = int(np.flatnonzero(off(self.b) & ~off(self.a))[0])
        self.check_case(changed(collateral, params={'high_exact_pieces': [moved_lattice]}),
                        contains='kernel_high_exact_pieces')
        print(f'Kernel keep-list pieces: exact off {q}, landed {q_prime}, lattice {moved_lattice}', flush=True)

    def test_11_bound_and_hygiene(self):
        start, limit = record(self.z), 5591206
        self.assertEqual(6 * 1 + 964 * 5800, limit)
        self.assertEqual(C.check_high_phase(start, [{'letters': 6}, {'letters': limit - 6}]), [])
        self.assertEqual(C.check_high_phase(start, [{'letters': limit + 1}]), ['kernel_letter_bound'])
        no_six = {'off': 5800, 'heights': {'2': 5799}}
        self.assertEqual(C.check_high_phase(no_six, [{'letters': limit - 6}]), [])
        self.assertEqual(C.check_high_phase(no_six, [{'letters': limit - 5}]), ['kernel_letter_bound'])
        for malformed in (None, [], {}, {'off': True, 'heights': {}}, {'off': -1, 'heights': {}},
                          {'off': 1, 'heights': []}, {'off': 1, 'heights': {'6': True}},
                          {'off': 1, 'heights': {'6': -1}}, {'off': 1, 'heights': {6: 1}}):
            self.assertEqual(C.check_high_phase(malformed, []), ['kernel_letter_bound'])
        for rows in (None, {}, 'rows', [None], [{}], [{'letters': True}], [{'letters': -1}],
                     [{'letters': '1'}], [{'letters': 1.5}]):
            self.assertEqual(C.check_high_phase(start, rows), ['kernel_letter_bound'])
        self.assertEqual(C.check_high_phase({'off': 0, 'heights': {}}, []), [])
        search = ast.parse((K / 'r22d25_search.py').read_text())
        limits = next(ast.literal_eval(node.value) for node in search.body if isinstance(node, ast.Assign)
                      and any(isinstance(t, ast.Name) and t.id == 'DEFAULT_LIMITS' for t in node.targets))
        self.assertEqual(C.ST_LIMIT, limits['ST'])
        self.assertEqual(C.UNCHECKED, 'unchecked_contract')
        self.assertFalse(hasattr(C, 'STUB'))
        for row in (None, [], 1, 'height', {}, {'phase': 'height'}, {'phase': 'high'},
                    {'phase': [], 'rule': {}}, {'phase': 'height', 'rule': []}):
            self.assertTrue(run_row(row, self.z, []))
        for case, keys in ((self.cases['r22d81/U'], ('rule', 'contract', 'subphase', 'letters',
                                                  'tau_before', 'tau_after', 'u_before', 'u_after')),
                           (self.f1, ('rule', 'contract', 'family', 'kind', 'target', 'params',
                                      'high_exact', 'subphase', 'off_before', 'off_after', 'heights_after'))):
            for key in keys:
                mutation = changed(case)
                del mutation[0][key]
                with self.subTest(missing=key, phase=case[0]['phase']):
                    self.assertTrue(self.check_case(mutation))
        for target in ('58388', [], [True], [-1], [len(self.z.pose_id)], [[58388]], [None]):
            self.check_case(changed(self.f1, target=target), contains='kernel_target')
        for keep in (None, '58388', [1.5], [len(self.z.pose_id)], [[58388]]):
            self.check_case(changed(self.f1, params={'high_exact_pieces': keep}), contains='kernel_fields')
        # Invalid targets must still allow independent collateral/progress checks.
        collateral = kernel_case('T1', 'landing', self.a, [[35, 13]], [58388])
        failures = self.check_case(changed(collateral, target='bad'))
        for name in ('kernel_target', 'kernel_collateral', 'kernel_progress'):
            self.assertIn(name, failures)
        # Thousands of high letters must not inspect states or count heights/u.
        ctx = hook(C.prepare, self.z, self.f1[0])
        with (patch.object(C, '_Snapshot', side_effect=AssertionError('per-letter snapshot')),
              patch.object(C, 'straddlers', side_effect=AssertionError('per-letter u'))):
            for k in range(2000):
                C.on_letter(ctx, k, self.z)
        self.assertEqual(scan((HERE / 'check_contracts.py').read_text()), [])
        for source in ('import eb6', 'from restore import Restorer', 'restore.jumble_items(m)',
                       'import subprocess', 'import exact', 'import rim', 'm = rim.ctx',
                       'from . import check_run', 'f(restore)', 'eval("1")',
                       'import restore as R\nx = R.NEAR', 'import os as runner', 'open("x")',
                       'compile("1", "x", "exec")', 'from ctypes import CDLL',
                       'import sim\nsim.runner()', 'from restore import *'):
            with self.subTest(bad_source=source):
                self.assertTrue(scan(source))
        tree = ast.parse((HERE / 'check_run.py').read_text())
        calls = [node for node in ast.walk(tree) if isinstance(node, ast.Call)
                 and isinstance(node.func, ast.Attribute) and isinstance(node.func.value, ast.Name)
                 and node.func.value.id == 'check_contracts']
        for name, arity in (('prepare', 2), ('on_letter', 3), ('finish', 3)):
            found = [call for call in calls if call.func.attr == name]
            self.assertTrue(found, name)
            self.assertTrue(all(len(call.args) == arity and not call.keywords for call in found), name)
        bound_calls = [call for call in calls if call.func.attr == 'check_high_phase']
        if bound_calls:
            print('Wiring: check_run.py calls check_high_phase; start arguments: '
                  + ', '.join(ast.unparse(call.args[0]) for call in bound_calls), flush=True)
        else:
            print('Wiring: check_run.py does NOT call check_high_phase; I1 must wire the phase start.', flush=True)
        print('Hook signatures: prepare(row, st), on_letter(ctx, k, st), finish(ctx, row, st) match.', flush=True)


def scan(source):
    """D4 import/name scan, stricter than the existing checker-side scan."""
    allowed = {'numpy', 'sim', 'trees', 'restore'}
    restore_attributes = {'menu', 'element', 'scramble', 'ctx', 'to_tuple', 'transpose'}
    forbidden = ('runner', 'rules_', 'lattice_phase', 'literal_lattice', 'eb6', 'seams', 'orient',
                 'buffer', 'r22d25_', 'verify_late', 'check_run', 'check_r22d25', 'check_lattice')
    denied = {'subprocess', 'importlib', 'runpy', 'multiprocessing', 'ctypes'}
    dynamic = {'rim', '__import__', 'importlib', 'runpy', 'exec', 'eval'}
    problems, restore_aliases, bases = [], {'restore'}, set()
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names = [alias.name for alias in node.names]
            restore_aliases.update(alias.asname or alias.name for alias in node.names if alias.name == 'restore')
        elif isinstance(node, ast.ImportFrom):
            names = [node.module or '']
            if node.level:
                problems.append('relative import')
            if node.module == 'restore':
                problems.extend('restore.' + alias.name for alias in node.names
                                if alias.name not in restore_attributes)
        else:
            continue
        for name in names:
            top = name.split('.')[0]
            if top in denied or top.startswith(forbidden):
                problems.append('forbidden import ' + name)
            elif top not in sys.stdlib_module_names and top not in allowed:
                problems.append('unlisted import ' + name)
        for alias in node.names:
            for name in (alias.name.split('.')[0], alias.asname or ''):
                if name.startswith(forbidden) or name in dynamic:
                    problems.append('forbidden name ' + name)
    for node in ast.walk(tree):
        if isinstance(node, ast.Attribute):
            if node.attr.startswith(forbidden) or node.attr in dynamic:
                problems.append('forbidden attribute ' + node.attr)
            if isinstance(node.value, ast.Name) and node.value.id in restore_aliases:
                bases.add(id(node.value))
                if node.attr not in restore_attributes:
                    problems.append('restore.' + node.attr)
        if isinstance(node, ast.Name) and (node.id.startswith(forbidden) or node.id in dynamic):
            problems.append('forbidden name ' + node.id)
        if isinstance(node, ast.Call):
            name = node.func.id if isinstance(node.func, ast.Name) else getattr(node.func, 'attr', '')
            if name in ('open', 'compile'):
                problems.append('forbidden call ' + name)
    problems.extend('restore used as a value' for node in ast.walk(tree)
                    if isinstance(node, ast.Name) and node.id in restore_aliases and id(node) not in bases)
    return problems


class TimedResult(unittest.TextTestResult):
    def startTest(self, test):
        self.started = time.monotonic()
        super().startTest(test)

    def stopTest(self, test):
        elapsed = time.monotonic() - self.started
        super().stopTest(test)
        self.stream.writeln(f'  {test.id()}: {elapsed:.3f}s')


# Git metadata and private trees that other processes may write during this suite.
SKIP = ('.git', 'work/codex-sandbox-home', 'work/loop-memory', 'work/reviews', 'work/worktrees')


def inventory():
    """Content digests and link targets of every file, including ignored files."""
    result = {}
    for directory, dirs, files in os.walk(REPO):
        here = Path(directory).relative_to(REPO)
        kept = []
        for name in sorted(dirs):
            relative = (here / name).as_posix()
            if relative in SKIP:
                continue
            path = Path(directory) / name
            if path.is_symlink():
                result[relative] = ('link', os.readlink(path))
            else:
                kept.append(name)
        dirs[:] = kept
        for name in sorted(files):
            path = Path(directory) / name
            if path == REPO / '.git':
                continue
            try:
                value = ('link', os.readlink(path)) if path.is_symlink() else (
                    'file', hashlib.sha256(path.read_bytes()).hexdigest())
            except FileNotFoundError:
                continue
            result[path.relative_to(REPO).as_posix()] = value
    return result


def status():
    return subprocess.check_output(['git', '--no-optional-locks', 'status', '--porcelain', '--ignored',
        '--untracked-files=all', '--', '.'] + [':(exclude)' + p for p in SKIP[1:]], cwd=REPO)


def main(names=()):
    started = time.monotonic()
    before_status, before_files = status(), inventory()
    result = None
    try:
        loader, module = unittest.defaultTestLoader, sys.modules[__name__]
        suite = loader.loadTestsFromNames(names, module) if names else loader.loadTestsFromModule(module)
        result = unittest.TextTestRunner(verbosity=2, resultclass=TimedResult).run(suite)
    finally:
        shutil.rmtree(TEMP)
        after_status, after_files = status(), inventory()
        unchanged = before_status == after_status and before_files == after_files
        changed_paths = sorted(p for p in before_files.keys() | after_files.keys()
                               if before_files.get(p) != after_files.get(p))
        print(f'Test 12 changed paths: {changed_paths}', file=sys.stderr, flush=True)
        if unchanged:
            print('Worktree unchanged, including ignored files.', file=sys.stderr, flush=True)
        else:
            print('FAIL: acceptance changed the worktree (including ignored files).', file=sys.stderr, flush=True)
            if before_status != after_status:
                print('Git status changed.', file=sys.stderr, flush=True)
        print(f'Acceptance elapsed: {time.monotonic() - started:.3f}s.', file=sys.stderr, flush=True)
    return 0 if result is not None and result.wasSuccessful() and unchanged else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
