"""Small exact replay controls; the depth-100 acceptance check supplies the search control."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import eb6r
from exact import matmul, transpose
from sim.kplus import to_tuple


class ReplayTests(unittest.TestCase):
    def test_exact_return_and_lattice_collateral_are_distinguished(self):
        # A nontrivial commutator in A4 moves lattice collateral. Treating
        # that collateral as an exact return must fail this control.
        M = eb6r.R.menu('I_a')
        st = eb6r.sim.State(eb6r.R.ctx, menu=M)
        elements = {j: eb6r.R.element(M, 0, j) for j in range(len(M.items))}
        retained = [j for j, g in elements.items() if st.ctx.kplus.index_of_matrix(g) > 0]
        a, b = next((a, b) for a in retained for b in retained
                    if matmul(elements[a], elements[b]) != matmul(elements[b], elements[a]))
        index = {g: j for j, g in elements.items()}
        inv = {j: index[to_tuple(transpose(g))] for j, g in elements.items()}
        r = retained[0]
        U = st.classify(0, need_inside=True).inside[:1]
        pid0, digest0 = eb6r.stable_pose(st), st.digest()
        for second, strict in ((a, True), (b, False)):
            word = [[0, r], [0, a], [0, second], [0, inv[a]],
                    [0, inv[second]], [0, inv[r]]]
            got = eb6r.replay_word(st, M, U, word, pid0, digest0)
            self.assertTrue(got['admissible'])
            self.assertEqual(got['heights'], [0])
            self.assertTrue(got['lands'])
            self.assertEqual(got['strict'], strict)
            self.assertTrue(got['strict_prime'])
            self.assertEqual(st.digest(), digest0)
            self.assertEqual(st.journal, [])


if __name__ == '__main__':
    unittest.main()
