"""Small exact gadget regressions; run from the repository root with python3 -I -B."""
import gzip
import json
import sys
import unittest
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parents[2] / '03-retained-group-and-lattice-restorer' / 'scripts'))
from rlib import Retained, NS
from kit import SLP
from sched11 import Schedule11, mul, inv


class Gadgets(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.R = Retained('.')
        cls.slp = SLP(cls.R)
        realign = json.load(gzip.open(HERE.parents[1] / 'data' / 'realign.json.gz', 'rt'))
        cls.schedule = Schedule11(cls.R, cls.slp, realign)

    def test_direct_cycle_orientation(self):
        k = self.schedule.kits[0]
        self.assertEqual((k.A, k.B, k.C), (117, 175946, 6290))
        g = k.setup(k.A, k.B, 4939)
        self.assertEqual(g, [-38])
        node = k.direct_cycle((k.A, k.B, 4939), g)
        self.assertEqual(k.edge(node, k.A)[0], k.B)
        self.assertEqual(k.edge(node, k.B)[0], 4939)
        self.assertEqual(k.edge(node, 4939)[0], k.A)
        self.assertEqual(self.slp.length[node], k.s + 2)

    def test_twisted_closing_cycle(self):
        for o in (0, 6, 34):
            k = self.schedule.kits[o]
            k.prepare()
            X, Y = [p for p in k.lab if p not in (k.A, k.B, k.C)][:2]
            L = np.arange(NS, dtype=np.int64)
            u, v = k.H[-1], k.H[len(k.H) // 2]
            L[k.lab[k.A]] = k.lab[X][list(u)]
            L[k.lab[X]] = k.lab[Y][list(v)]
            node = k.twisted_cycle((k.A, X, Y), k.setup(k.A, X, Y), L, {X, Y})
            s, d = self.slp.net[node]
            L[d] = L[s]
            np.testing.assert_array_equal(L[k.lab[X]], k.lab[X])
            np.testing.assert_array_equal(L[k.lab[Y]], k.lab[Y])
            self.assertLessEqual(self.slp.length[node], k.s + 4 * k.h + 4 * len(k.swap)
                                 + 2 * (k.routes['e_max'] + max(k.R.trees[o]['depth'])))

    def test_c2_pair(self):
        k = self.schedule.kits[3]
        k.prepare()
        X, Y = [p for p in k.lab if p not in (k.A, k.B, k.C)][:2]
        v = next(x for x in k.H if x != k.ident)
        node = k.c2_pair(X, Y, v, v)
        self.assertEqual(k.edge(node, X), (X, v))
        self.assertEqual(k.edge(node, Y), (Y, v))

    def test_frame_triple(self):
        for o in (28, 34):
            k = self.schedule.kits[o]
            k.prepare()
            X, Y = [p for p in k.lab if p not in (k.A, k.B, k.C)][:2]
            u, v = k.H[-1], k.H[len(k.H) // 2]
            node = k.frame_triple(X, Y, u, v)
            self.assertEqual(k.edge(node, X), (X, u))
            self.assertEqual(k.edge(node, Y), (Y, v))

    def test_short_single(self):
        for o, extra in ((6, 16), (34, 10)):
            k = self.schedule.kits[o]
            k.prepare()
            for v in k.single_choices:
                if v == k.ident:
                    continue
                node = k.single(v)
                self.assertEqual(k.edge(node, k.C), (k.C, v))
                self.assertEqual(k.edge(node, k.A), (k.A, k.ident))
                self.assertEqual(k.edge(node, k.B), (k.B, k.ident))
                self.assertLessEqual(self.slp.length[node], 4 * k.s + extra)


if __name__ == '__main__':
    unittest.main()
