"""Replay H6030 with J1, checking EH-29 on prefixes and the recorded end.

The default run checks the bound after each of the first 600 letters and at
the end; JUMBLING_FULL_REPLAY=1 checks it after every letter. Run directly,
or through unittest alongside the NumPy-free harness checks; the latter runs
these tests in one isolated worker so those checks keep their import proof.
"""

import sys

sys.dont_write_bytecode = True

import ast
from collections import Counter
from dataclasses import replace
from functools import lru_cache
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[1]
JUMBLING = REPO / 'research' / 'jumbling'
X1 = JUMBLING / 'restoration' / '10-x1-unbounded'
sys.path[:0] = [str(JUMBLING), str(JUMBLING / 'theory')]

from differential import harness as H
from differential import menu_order as M

FULL = os.environ.get('JUMBLING_FULL_REPLAY') == '1'
_IN_PROCESS = __name__ == '__main__'
# Letters with the bound checked after each one in the default run.
PREFIX_LETTERS = 600
END_DIGEST = 'cb1e89ed6b0b1581f8acea27e22b443df5784d928d4284579d69c444f60b37ad'
END_HEIGHTS = {0: 124237, 2: 52173, 4: 703, 6030: 7}
TOP_PIECES = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
CTX = None
MENU = None
WITNESS = None
STEPS = None


def setUpModule():
    global CTX, MENU, WITNESS, STEPS, np, sim, trees, J1Engine
    try:
        import numpy as np
    except ImportError as error:
        raise unittest.SkipTest('NumPy is required for the J1 X1 witness replay') from error
    import sim
    import trees
    from differential.j1_adapter import J1Engine
    CTX = sim.get_context()
    MENU = json.loads((JUMBLING / 'fixtures' / 'wj-I_a.json').read_text(encoding='utf-8'))['menu']
    # Letter indices are meaningful only in the signed I_a item order.
    M.check_menu(MENU, order=M.I_A_ORDER_DIGEST)
    WITNESS = json.loads((X1 / 'witnesses' / 'H6030.json').read_text(encoding='utf-8'))
    STEPS = H.load_steps(WITNESS, menu=MENU)
    # pose_tree repeatedly values the same large power-of-two denominators.
    # Memoize the original pure integer helper, with bounded memory, for this
    # module only. Every pose still goes through the original pose_tree.
    valuation_cache = patch.object(trees, '_v2int', lru_cache(maxsize=4096)(trees._v2int))
    valuation_cache.start()
    unittest.addModuleCleanup(valuation_cache.stop)


class X1WitnessTests(unittest.TestCase):
    def test_h6030_recorded_end_and_prefix_height_bounds(self):
        self.assertEqual(M.order_digest(MENU), M.I_A_ORDER_DIGEST)
        self.assertEqual(STEPS.menu_order_digest, M.I_A_ORDER_DIGEST)
        self.assertEqual(len(STEPS.steps), 3156)
        self.assertEqual(WITNESS['claims']['letters'], 3156)
        self.assertEqual(tuple(WITNESS['claims']['pieces']), TOP_PIECES)
        self.assertEqual(WITNESS['claims']['final_left_height_of_pieces'], 6030)
        recorded = (X1 / 'programs' / 'H6030.out').read_text(encoding='utf-8').splitlines()
        self.assertEqual(next(line[7:] for line in recorded if line.startswith('digest ')), END_DIGEST)
        histogram = next(line.removeprefix('final heights (height: pieces): ') for line in recorded
                         if line.startswith('final heights (height: pieces): '))
        self.assertEqual(ast.literal_eval(histogram), END_HEIGHTS)

        engine = J1Engine(CTX)
        engine.start(STEPS.menu)
        state = engine.state
        # programs/replay.py uses dist(factors(Q)[0]) for piece heights. pose_tree
        # returns that same left distance, including 0 for identity and K+ poses.
        identity = trees.pose_tree(state._pose_matrix(0))[:2]
        self.assertEqual(identity, (0, 0))
        memo = {state._poses[0].key: identity}

        def check_bound(played):
            for pid, pose in enumerate(state._poses[1:], 1):
                if pose.key not in memo:
                    memo[pose.key] = trees.pose_tree(state._pose_matrix(pid))[:2]
                left, right = memo[pose.key]
                self.assertEqual(right, 0, f'H6030: letter {played}, pose {pid}')
                self.assertLessEqual(left, 2 * played, f'H6030: letter {played}, pose {pid}')

        checked = len(STEPS.steps) if FULL else PREFIX_LETTERS
        for played, step in enumerate(STEPS.steps, 1):
            result = engine.apply(step)
            self.assertEqual(result.status, 'admissible', f'H6030: letter {played}')
            if played <= checked:
                check_bound(played)
        check_bound(len(STEPS.steps))

        self.assertEqual(result.digest, END_DIGEST)
        counts = np.bincount(state.pose_id, minlength=len(state._poses))
        heights = Counter()
        for pid, count in enumerate(counts):
            pose = state._poses[pid]
            left = memo[pose.key][0]
            if count:
                heights[left] += int(count)
            if pose.kidx >= 0:
                self.assertEqual(left, 0)
            # Check the convention literally against programs/replay.py's formula.
            self.assertEqual(left, trees.dist(trees.factors(state._pose_matrix(pid))[0]))
        self.assertEqual(dict(heights), END_HEIGHTS)
        top_ids = {int(state.pose_id[piece]) for piece in TOP_PIECES}
        self.assertEqual(len(top_ids), 1)
        top_id = top_ids.pop()
        self.assertEqual(memo[state._poses[top_id].key][0], 6030)
        self.assertEqual(np.flatnonzero(state.pose_id == top_id).tolist(), list(TOP_PIECES))

    def test_paired_witness_replay(self):
        self.assertEqual(M.order_digest(MENU), M.I_A_ORDER_DIGEST)
        steps = STEPS if FULL else replace(STEPS, steps=STEPS.steps[:PREFIX_LETTERS])
        reference = J1Engine(CTX, name='J1-reference')
        candidate = J1Engine(CTX, name='J1-candidate')
        report = H.compare(reference, candidate, steps)
        self.assertTrue(report.passed, report.as_dict())
        self.assertEqual(report.steps_compared, len(steps.steps) + 1)
        if FULL:
            self.assertEqual(reference.state.digest(), END_DIGEST)
            self.assertEqual(candidate.state.digest(), END_DIGEST)


def load_tests(loader, tests, pattern):
    if _IN_PROCESS:
        return tests
    from tests import isolated_numpy
    return isolated_numpy.suite('test_jumbling_x1_witness', 'J1 X1 witness replay in an isolated NumPy process',
                                'NumPy is required for the J1 X1 witness replay')


if __name__ == '__main__':
    unittest.main()
