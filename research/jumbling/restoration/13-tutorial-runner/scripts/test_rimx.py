"""Differential proof and D3 access-pattern timing for the exact domain cache.

Run with python3 -I -B. This loads mechanics, never a runner or a bulk rule.
All writable caches are redirected into a temporary directory.
Timings are headless research measurements for this fixture and source only.
"""
import hashlib
import json
import os
from concurrent.futures import ProcessPoolExecutor, as_completed
import multiprocessing
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest import mock

for variable in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[variable] = '1'
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
J = ROOT / 'research/jumbling'
FIXTURE = HERE.parent / 'runs/dev/d3-s7102-fixture'
sys.path[:0] = [str(HERE), str(J / 'theory'), str(J), str(J / 'fixtures')]

import numpy as np  # noqa: E402


def setUpModule():
    global TEMP, MECHANICS, rim, rimx, restore, seams, PROTECTED
    TEMP = tempfile.TemporaryDirectory(prefix='rimx-test-')
    cache = Path(TEMP.name)
    os.environ['JUMBLING_HOME_CACHE'] = str(cache / 'home_centroids.npy')
    # Snapshot just the packet's read-only sources, without traversing run data.
    paths = [J / 'exact.py', HERE / 'runner.py']
    paths += sorted((J / 'theory').glob('*.py'))
    paths += sorted((J / 'sim').glob('*.py'))
    paths = [p for p in paths if p.name != 'rimx.py']
    PROTECTED = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    import runner
    import rim
    import rimx
    import restore
    import seams
    MECHANICS = runner.Mechanics(cache)
    assert seams.CACHE == cache / 'home_centroids.npy'


def tearDownModule():
    try:
        rimx.clear_caches()
        for path, digest in PROTECTED.items():
            if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
                raise AssertionError('read-only source changed: ' + path.name)
    finally:
        TEMP.cleanup()


def synthetic_case(case):
    """A complete apply/apply/undo/undo path in an independent local process."""
    setUpModule()
    try:
        depth, seed = case
        check = DomainTests('test_01_synthetic_twists_undos_and_cache_independence')
        state, history = restore.scramble(MECHANICS.menu, depth, seed)
        restorer = restore.Restorer(state, MECHANICS.menu)
        original = state.digest()
        rimx.clear_caches()
        check.compare_state(restorer, f'{depth}/{seed}/cold')
        c, last_item = history[-1]
        inverse = restore.to_tuple(restore.transpose(restorer.g(c, last_item)))
        inverse_item = next(i for i in range(restorer.n_items) if restorer.g(c, i) == inverse)
        for turn in range(2):
            # The last scramble letter and its inverse give certified, different
            # configurations without adding an unrelated search to the test.
            restorer.apply(c, inverse_item if turn == 0 else last_item)
            if turn == 1:
                check.evict_shared(restorer)
            check.compare_state(restorer, f'{depth}/{seed}/apply-{turn + 1}')
        for turn in range(2):
            restorer.undo()
            if turn == 0:
                rimx.clear_caches()
            check.compare_state(restorer, f'{depth}/{seed}/undo-{turn + 1}')
        check.assertEqual(state.digest(), original)
        check.assertEqual(state.journal, [])
        check.assertEqual(restorer.moves, [])
    finally:
        tearDownModule()


class DomainTests(unittest.TestCase):
    def compare_structure(self, state, old=None, new=None):
        old = rim.Domains(state) if old is None else old
        new = rimx.Domains(state) if new is None else new
        self.assertEqual([restore.to_tuple(r) for r in new.reps],
                         [restore.to_tuple(r) for r in old.reps])
        np.testing.assert_array_equal(new.dom, old.dom)
        np.testing.assert_array_equal(new.cham, old.cham)
        self.assertEqual(len(new.members), len(old.members))
        for a, b in zip(new.members, old.members):
            np.testing.assert_array_equal(a, b)
        self.assertEqual(new.total_misalignment(), old.total_misalignment())
        for d in range(len(old.reps)):
            for c in range(600):
                self.assertEqual(new.pole(d, c), old.pole(d, c), ('pole', d, c))
        return old, new

    def compare_caps(self, restorer, old, new, caps=range(600)):
        clean, items = 0, 0
        for c in caps:
            inside = restorer.inside(c)
            if inside is None:
                continue
            clean += 1
            mask = np.zeros(rim.NP, bool)
            mask[inside] = True
            before = old.rim_sets(c, mask)
            after = new.rim_sets(c, mask)
            self.assertEqual(after, before, ('rim_sets', c))
            self.assertEqual(new.misalignment(c, mask, rs=after),
                             old.misalignment(c, mask, rs=before), ('now', c))
            for i in range(restorer.n_items):
                g = restorer.g(c, i)
                self.assertEqual(new.misalignment(c, mask, (c, i), g, after),
                                 old.misalignment(c, mask, (c, i), g, before), (c, i))
                items += 1
        return clean, items

    def compare_state(self, restorer, label):
        started = time.perf_counter()
        old, new = self.compare_structure(restorer.st)
        clean, items = self.compare_caps(restorer, old, new)
        print(json.dumps({'synthetic': label, 'domains': len(old.reps),
                          'clean_caps': clean, 'item_comparisons': items,
                          'seconds': round(time.perf_counter() - started, 3)}), flush=True)

    def evict_shared(self, restorer):
        caches = list(rimx._CACHES.values())
        limits = [cache.max_entries for cache in caches]
        try:
            for cache in caches:
                cache.clear()
                cache.max_entries = 1
            # At least two distinct exact keys for every shared cache.
            for c in (0, 1, 2):
                g = restorer.g(c, 1)
                rimx._product(g, restore.I4)
                rimx._relative_index(g, restore.I4)
                rimx._floating(g)
                matrix = rimx._matrix(g)
                rimx._CACHES['match'].put(matrix, (matrix, rim.IDK))
                rimx._chamber_cells(int(rim.kp.gen_idx[2 * c]), (0, rim.CORE))
                rimx._cut_cells(c)
                D = rimx.Domains(restorer.st)
                D.pole(0, c)
            self.assertTrue(all(cache.evictions > 0 for cache in caches))
            self.assertTrue(all(len(cache.values) <= 1 for cache in caches))
        finally:
            for cache, limit in zip(caches, limits):
                cache.max_entries = limit

    def test_01_synthetic_twists_undos_and_cache_independence(self):
        # Small scrambles keep the exhaustive (including aligned cuts) menu
        # comparisons within the wall budget; the real fixture covers 20 domains.
        # Three depths and two seeds, with two applications and two undos each.
        cases = [(1, 2), (2, 7), (3, 2)]
        with ProcessPoolExecutor(max_workers=3, mp_context=multiprocessing.get_context('spawn')) as pool:
            pending = {pool.submit(synthetic_case, case): case for case in cases}
            for future in as_completed(pending):
                with self.subTest(case=pending[future]):
                    future.result()

    def test_02_domain_indexes_and_exact_twist_keys(self):
        a, _ = restore.scramble(MECHANICS.menu, 3, 2)
        b, _ = restore.scramble(MECHANICS.menu, 3, 7)
        da, db = rimx.Domains(a), rimx.Domains(b)
        self.assertNotEqual(restore.to_tuple(da.reps[1]), restore.to_tuple(db.reps[1]))
        g = restore.to_tuple(restore.transpose(db.reps[1]))
        first = da.coset('inverse', g, 1)
        second = db.coset('inverse', g, 1)
        self.assertEqual(first, rim.Domains(a).coset('inverse', g, 1))
        self.assertEqual(second, rim.Domains(b).coset('inverse', g, 1))
        self.assertNotEqual(first, second)
        # A caller key is never a global identity for the matrix, even across
        # configurations or menus. Check both orders with intentionally reused keys.
        for matrix in (g, restore.I4, g):
            fresh = rimx.Domains(b)
            expected = rim.Domains(b)
            for d in range(len(fresh.reps)):
                self.assertEqual(fresh.coset((0, 1), matrix, d),
                                 expected.coset((0, 1), matrix, d))
        # Local caches also have eviction limits.
        with mock.patch.object(db._coset, 'max_entries', 1):
            for d in range(len(db.reps)):
                self.assertEqual(db.coset('inverse', g, d),
                                 rim.Domains(b).coset('inverse', g, d))
            self.assertLessEqual(len(db._coset.values), 1)

    def test_03_float_proposals_cannot_decide_membership(self):
        state, history = restore.scramble(MECHANICS.menu, 3, 2)
        restorer = restore.Restorer(state, MECHANICS.menu)
        for proposals in (lambda self, m: [],
                          lambda self, m: list(reversed(range(len(self.reps))))):
            rimx.clear_caches()
            with mock.patch.object(rimx.Domains, '_proposals', proposals):
                old, new = self.compare_structure(state)
                self.compare_caps(restorer, old, new, [history[-1][0]])
        # Non-finite approximate matrices must also take the exact fallback.
        with mock.patch.object(rimx, '_floating', return_value=np.full((4, 4), np.nan)):
            rimx.clear_caches()
            old, new = self.compare_structure(state)
            self.compare_caps(restorer, old, new, [history[-1][0]])
        # Exercise empty/full/arbitrary masks, absent rs, and every coset argument
        # at a cap; these are independent of a clean geometric classification.
        old, new = rim.Domains(state), rimx.Domains(state)
        rng = np.random.default_rng(123)
        for mask in (np.zeros(rim.NP, bool), np.ones(rim.NP, bool), rng.random(rim.NP) < .5):
            c = history[-1][0]
            self.assertEqual(new.rim_sets(c, mask), old.rim_sets(c, mask))
            self.assertEqual(new.misalignment(c, mask), old.misalignment(c, mask))
        for i in range(restorer.n_items):
            g = restorer.g(c, i)
            for d in range(len(old.reps)):
                self.assertEqual(new.coset((c, i), g, d), old.coset((c, i), g, d))

        # Preserve reference behavior on a deliberately non-injective pose table
        # too. This is an input-robustness case, not an admissible puzzle state.
        overlapping = MECHANICS.sim.State(restore.ctx, menu=MECHANICS.menu)
        p, q = np.flatnonzero(rim.orbit_of == rim.orbit_of[0])[:2].tolist()
        k = rim.kp.compose(int(rim.transport[p]), int(rim.kp.inv[rim.transport[q]]))
        overlapping.pose_id[q] = overlapping._intern_k(k)
        old, new = rim.Domains(overlapping), rimx.Domains(overlapping)
        self.assertEqual(old.cham[p], old.cham[q])
        mask = np.zeros(rim.NP, bool)
        mask[p] = True
        self.assertEqual(new.rim_sets(0, mask), old.rim_sets(0, mask))
        self.assertIs(new._inverse.values[0], False)

    def test_00_committed_real_state_and_conj_access_pattern(self):
        meta = json.loads((FIXTURE / 'steps-199-201.json').read_bytes())
        path = FIXTURE / 'bulk-198.json'
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), meta['table']['sha256'])
        state = MECHANICS.load(path)
        self.assertEqual(state.digest(), meta['table']['digest'])
        self.assertEqual(len(np.unique(state.pose_id)), 771)
        rimx.clear_caches()
        old, new = self.compare_structure(state)
        self.assertEqual(len(old.reps), 20)
        self.assertEqual(new.total_misalignment(), meta['steps'][0]['before']['N'])
        surveyor = seams.SeamRestorer(state, MECHANICS.menu)
        surveyor._dom = new
        survey = surveyor.survey()
        # Check all current N_c values so the accelerated survey cannot silently
        # omit or reorder one of the forty caps selected for the full comparison.
        for c in range(600):
            inside = surveyor.inside(c)
            if inside is not None:
                mask = np.zeros(rim.NP, bool)
                mask[inside] = True
                self.assertEqual(old.misalignment(c, mask), surveyor.scores(c)[0])
        mis = sorted(survey, key=lambda c: (-survey[c][0], c))
        self.assertGreaterEqual(len(mis), 40)
        self.assertEqual(self.compare_caps(surveyor, old, new, mis[:40]),
                         (40, 40 * surveyor.n_items))
        print(json.dumps({'real_comparison_caps': 40, 'survey_caps': len(mis),
                          'items_per_cap': surveyor.n_items}), flush=True)

        baseline = seams.SeamRestorer(MECHANICS.load(path), MECHANICS.menu)
        cached = seams.SeamRestorer(MECHANICS.load(path), MECHANICS.menu)
        for restorer in (baseline, cached):
            restorer._g = dict(surveyor._g)
            restorer._cls = dict(surveyor._cls)

        def trials():
            for c in mis:
                nb = sorted(({int(x) for x in restore.NEAR[c]} & set(mis)) - {c})
                if nb:
                    for i in range(surveyor.n_items):
                        if surveyor.g(c, i) != restore.I4:
                            yield c, i, nb

        def trial(restorer, factory, c, i, nb):
            started = time.perf_counter()
            restorer.apply(c, i)
            try:
                # Classification belongs to State, not Domains. Prepare exactly
                # the masks scores() requests, then time construction and the
                # complete Domains access pattern on those same masks. Also
                # retain the full apply/classify/score/undo time, separately.
                for d in nb:
                    restorer.inside(d)
                domain_started = time.perf_counter()
                restorer._dom = factory(restorer.st)
                restorer._sv.clear()
                result = [(d, restorer.scores(d)) for d in nb]
                domain_seconds = time.perf_counter() - domain_started
            finally:
                restorer.undo()
            return result, domain_seconds, time.perf_counter() - started

        old_seconds = new_seconds = 0.0
        old_trial_seconds = new_trial_seconds = 0.0
        count = 0
        for c, i, nb in trials():
            before, old_time, old_total = trial(baseline, rim.Domains, c, i, nb)
            after, new_time, new_total = trial(cached, rimx.Domains, c, i, nb)
            self.assertEqual(after, before, ('trial', count, c, i, nb))
            old_seconds += old_time
            new_seconds += new_time
            old_trial_seconds += old_total
            new_trial_seconds += new_total
            count += 1
            if count % 8 == 0:
                print(json.dumps({'trials_so_far': count, 'old_seconds': round(old_seconds, 3),
                                  'new_seconds': round(new_seconds, 3)}), flush=True)
            if old_seconds >= 120:
                break
        self.assertGreaterEqual(old_seconds, 120)
        for restorer in (baseline, cached):
            self.assertEqual(restorer.st.digest(), meta['table']['digest'])
            self.assertEqual(restorer.moves, [])
            self.assertEqual(restorer.st.journal, [])
        caches = rimx.cache_info()
        cache_bytes = sum(info['bytes'] for info in caches.values())
        ratio = old_seconds / new_seconds
        print(json.dumps({'real_state': 'd3-s7102-bulk-198', 'trial_count': count,
                          'timing_scope': 'Domains construction and scores after classification',
                          'old_seconds': round(old_seconds, 6),
                          'new_seconds': round(new_seconds, 6), 'ratio': round(ratio, 6),
                          'old_trial_seconds': round(old_trial_seconds, 6),
                          'new_trial_seconds': round(new_trial_seconds, 6),
                          'trial_ratio': round(old_trial_seconds / new_trial_seconds, 6),
                          'cache_bytes': cache_bytes, 'caches': caches,
                          'python': sys.version.split()[0], 'numpy': np.__version__}), flush=True)
        for info in caches.values():
            self.assertLessEqual(info['size'], info['max_entries'])
        self.assertLess(cache_bytes, 1_000_000_000)
        self.assertGreaterEqual(ratio, 5)


if __name__ == '__main__':
    started = time.perf_counter()
    result = unittest.main(verbosity=2, exit=False).result
    elapsed = time.perf_counter() - started
    print(f'Acceptance wall seconds: {elapsed:.3f} (limit 900)', flush=True)
    sys.exit(0 if result.wasSuccessful() and elapsed < 900 else 1)
