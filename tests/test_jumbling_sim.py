"""Headless checks of the J1 jumbling simulator (research/jumbling/sim) on a small subset.

Run from the repository root:

    python tests/test_jumbling_sim.py

The full acceptance run over all 1,200 generators is research/jumbling/sim/accept.py. These
tests read assets/model.npz and assets/primitives.npz read-only and write nothing.
"""
import json
import itertools
import math
import random
import sys
import unittest
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'research' / 'jumbling'))

import sim  # noqa: E402
import witness as W  # noqa: E402
from exact import ONE, ZERO, Q5, dot, identity, matmul, matvec, transpose  # noqa: E402
from sim.kernel import (Evaluator, classify_signs, filtered_signs, float_rows, pullback_form,  # noqa: E402
                        rows_form, sign5, sign5_vec)
from sim.kplus import generator_word_matrix, matrix_from_json, q5_from_json, to_tuple  # noqa: E402
from sim.model import CELL_SLOTS, CONTRACT_REVISION, NS  # noqa: E402
from sim.twists import nearest_parameter  # noqa: E402

CTX = None
PRIM = None
WITNESS = json.loads((ROOT / 'research' / 'jumbling' / 'witness-results.json').read_text())


def setUpModule():
    global CTX, PRIM
    CTX = sim.get_context()
    PRIM = CTX.data.primitives()


def ref_labels(word):
    """Labels after a word of signed primitive ids on the retained slot moves (core.word_net)."""
    src, dst, off = PRIM
    a = np.arange(NS, dtype=np.int64)
    for mid in word:
        k = abs(mid) - 1
        s, d = src[off[k]:off[k + 1]], dst[off[k]:off[k + 1]]
        if mid > 0:
            a[d] = a[s]
        else:
            a[s] = a[d]
    return a


def witness_twists():
    c, d = 0, 13
    g = sim.plane(CTX, c, d, degrees=10)
    td = next(a for a in W.a4(d) if sum((a[i][i] for i in range(4)), ZERO) == ONE and W.pole_perm(a)[c] != c)
    return c, d, g, sim.Twist(CTX, d, td)


def negative_control():
    axis = [dot(u, CTX.data.N[13]) / CTX.data.NN for u in sim.cap_frame(CTX.data.N[0])]
    return sim.half_turn(CTX, 0, axis)


def brute_input_map(axis, degrees, n, dmax):
    """Independent exhaustive 3x3-matrix search, including every denominator."""
    axis = np.array(axis, float)
    axis /= np.linalg.norm(axis)
    x, y, z = axis
    cross = np.array([[0, -z, y], [z, 0, -x], [-y, x, 0]])
    theta = math.radians(degrees)
    target = np.eye(3) + math.sin(theta) * cross + (1 - math.cos(theta)) * (cross @ cross)
    rows = []
    for nums in itertools.product(range(-n, n + 1), repeat=3):
        p = np.array(nums, float)
        for d in range(1, dmax + 1):
            x, y, z = p / d
            w = np.array([[0, -z, y], [z, 0, -x], [-y, x, 0]])
            r = np.eye(3) + 2 * (w + w @ w) / (1 + (p @ p) / (d * d))
            rows.append((float(np.linalg.norm(r - target)), d, nums, 'cayley'))
        canonical_angle = abs(math.remainder(degrees, 360))
        if abs(canonical_angle - 180) <= 1 and np.any(p):
            r = 2 * np.outer(p, p) / (p @ p) - np.eye(3)
            rows.append((float(np.linalg.norm(r - target)), 1, nums, 'half_turn'))
    best = min(row[0] for row in rows)
    choice = min((d, nums, family) for dist, d, nums, family in rows if dist <= best + 1e-12)
    return best, choice


class KernelTest(unittest.TestCase):
    def test_sign5_matches_q5(self):
        rng = random.Random(1)
        xs = [(rng.randrange(-10 ** 6, 10 ** 6), rng.randrange(-10 ** 6, 10 ** 6)) for _ in range(3000)]
        xs += [(0, 0), (5, -2), (-5, 2), (9, -4), (-9, 4), (0, 3), (3, 0)]
        for a, b in xs:
            self.assertEqual(sign5(a, b), Q5(a, b, 1).sign())
        arr = np.array(xs[:2000])
        self.assertEqual(sign5_vec(arr[:, 0], arr[:, 1]).tolist(), [Q5(a, b, 1).sign() for a, b in xs[:2000]])

    def test_evaluator_matches_q5_offsets(self):
        st = sim.State(CTX)
        g = sim.plane(CTX, 0, 13, degrees=10)
        st.apply(g)
        reg = CTX.regions
        rng = random.Random(2)
        pieces = rng.sample(CTX.data.cap_members(0).tolist(), 40)
        for e in rng.sample(range(600), 15) + [0, 1, 13, 108]:
            ev = Evaluator(pullback_form(g.matrix, CTX.data.N[e]))
            for p in pieces:
                vf = reg.vform(p)
                kernel = [ev.sign_row(vf[0], r) for r in vf[1]]
                direct = [W.h(e, matvec(g.matrix, v)).sign() for v in reg.vertices(p)]
                self.assertEqual(kernel, direct)

    def test_e0_control_regions(self):
        ev = Evaluator(pullback_form(CTX.kplus.matrix(0), CTX.data.N[0]))
        cases = {'straddle': [(Q5(-5, 0, 10 ** 10), Q5(1, 0, 1000)), (Q5(-1, 0, 1000), Q5(5, 0, 10 ** 10))],
                 'in': [(ZERO, Q5(1, 0, 1000))], 'out': [(Q5(-1, 0, 1000), ZERO)]}
        for want, ranges in cases.items():
            for lo, hi in ranges:
                vf = rows_form(W.control_region(0, lo, hi))
                self.assertEqual(ev.classify(vf)[0], want)
                self.assertEqual(classify_signs(filtered_signs(ev, vf, float_rows(vf))[0])[0], want)

    def test_filter_never_disagrees_with_exact(self):
        reg = CTX.regions
        rng = random.Random(3)
        mats = [sim.plane(CTX, 0, 13, s=Q5(1, 0, 10 ** 9)).matrix, sim.plane(CTX, 0, 13, degrees=10).matrix,
                sim.cayley(CTX, 0, [Q5(1, 1, 7), Q5(-2, 0, 9), Q5(0, 1, 11)]).matrix]
        decided = 0
        for m in mats:
            for e in rng.sample(range(600), 25) + [0, 1, 13]:
                ev = Evaluator(pullback_form(m, CTX.data.N[e]))
                for p in rng.sample(CTX.data.cap_members(0).tolist(), 60):
                    vf = reg.vform(p)
                    signs, nd, _ = filtered_signs(ev, vf, reg.frows(p))
                    self.assertEqual(signs, [ev.sign_row(vf[0], r) for r in vf[1]])
                    decided += nd
        self.assertGreater(decided, 1000)

    def test_filter_records_exact_fallbacks(self):
        ev = Evaluator(pullback_form(CTX.kplus.matrix(0), CTX.data.N[0]))
        vf = rows_form(W.control_region(0, ZERO, Q5(1, 0, 1000)))
        records = []
        signs, _, exact = filtered_signs(ev, vf, float_rows(vf), record=records)
        self.assertGreater(exact, 0)
        self.assertEqual(signs, [r['accepted_sign'] for r in records])
        self.assertTrue(any(r['method'] == 'exact_fallback' and r['accepted_sign'] == 0 for r in records))
        records = []
        signs, filtered, exact = filtered_signs(ev, vf, None, record=records)
        self.assertEqual(filtered, 0)
        self.assertEqual(exact, len(vf[1]))
        self.assertTrue(all(r['method'] == 'exact_fallback' and r['enclosure'] is None for r in records))
        huge = Evaluator(((10 ** 400, 0, 0, 0), (0, 0, 0, 0), 1))
        records = []
        signs, filtered, exact = filtered_signs(huge, vf, float_rows(vf), record=records)
        self.assertEqual(filtered, 0)
        self.assertEqual(signs, [huge.sign_row(vf[0], row) for row in vf[1]])
        self.assertTrue(all(r['method'] == 'exact_fallback' for r in records))


class ModelTest(unittest.TestCase):
    def test_kplus(self):
        kp = CTX.kplus
        self.assertEqual(len(kp.perms), 7200)
        self.assertEqual(len(kp.stab0), 12)
        for i in random.Random(4).sample(range(7200), 60):
            self.assertTrue(kp.pole_images_ok(i))
        for k in range(1200):
            self.assertEqual(int(kp.perms[int(kp.gen_idx[k])][k // 2]), k // 2)

    def test_orbits_and_regions(self):
        rep = CTX.regions.report
        self.assertEqual(rep['kplus_orbits'], 36)
        self.assertTrue(rep['g_class_in_one_kplus_orbit'])
        self.assertEqual(rep['rep_full_dimensional'], '36/36')
        self.assertEqual(rep['rep_host_patches_match_retained'], '36/36')
        cert = CTX.regions.certify()
        self.assertEqual(cert['rep_vertices_violating_a_constraint'], 0)
        self.assertEqual(cert['rep_vertices_without_rank4_tight_set'], 0)
        self.assertEqual(cert['reps_touching_every_signature_cut_exactly'], '36/36')
        self.assertEqual(cert['pieces_with_transported_signature_mismatch'], 0)
        self.assertEqual(cert['pieces_with_transported_host_mismatch'], 0)


class GeneratorTest(unittest.TestCase):
    def test_sample_generators_stickers_and_frames(self):
        for k in (0, 1, 26, 27, 598, 599, 1198, 1199):
            st = sim.State(CTX)
            solved = st.snapshot()
            out = st.apply(sim.generator(CTX, k))
            self.assertTrue(out.applied)
            self.assertEqual(out.moved, 3097)
            lab = st.lattice_stickers()
            self.assertTrue(np.array_equal(lab['labels'], ref_labels([k + 1])))
            self.assertTrue(lab['frame_agrees'].all())
            self.assertTrue(np.array_equal(np.sort(lab['slot']), np.arange(NS)))
            self.assertTrue(np.array_equal(np.sort(lab['labels']), np.arange(NS)))
            self.assertTrue(((0 <= lab['orientation']) & (lab['orientation'] < 12)).all())
            centres = CTX.data.centre_slots
            self.assertEqual(len(centres), 600)
            self.assertTrue(np.array_equal(lab['slot'][centres], centres))
            centre = int(CTX.data.centre_pieces[CTX.data.centre_poles == k // 2][0])
            self.assertEqual(st.pose(centre), sim.generator(CTX, k).matrix)
            self.assertNotEqual(st.pose(centre), CTX.kplus.matrix(0))
            st.undo()
            self.assertEqual(st.snapshot(), solved)

    def test_retained_word_labels(self):
        rng = random.Random(5)
        word = [rng.choice([1, -1]) * rng.randrange(1, 1201) for _ in range(25)]
        st = sim.State(CTX)
        for mid in word:
            self.assertTrue(st.apply(sim.primitive(CTX, mid)).applied)
        lab = st.lattice_stickers()
        self.assertTrue(np.array_equal(lab['labels'], ref_labels(word)))
        self.assertTrue(lab['frame_agrees'].all())

    def test_a4_words_and_admissibility(self):
        st = sim.State(CTX)
        for c in (0, 13, 599):
            for i in range(1, 12):
                tw = sim.a4_element(CTX, c, i)
                self.assertEqual(generator_word_matrix(CTX.kplus, tw.params['word']), tw.matrix)
                out = st.apply(tw)
                self.assertTrue(out.applied)
                self.assertEqual(out.moved, 3097)
                st.undo()
        self.assertEqual(st.moved_count(), 0)

    def test_solved_projection(self):
        lab = sim.State(CTX).lattice_stickers()
        self.assertTrue(np.array_equal(lab['slot'], np.arange(NS)))
        self.assertTrue(np.array_equal(lab['labels'], np.arange(NS)))
        self.assertTrue((lab['orientation'] == 0).all())
        self.assertTrue(lab['frame_agrees'].all())


class WitnessTest(unittest.TestCase):
    def test_grouped_equals_per_piece(self):
        _, _, g, t = witness_twists()
        e2 = sim.State(CTX)
        e2.apply(g)
        e3 = sim.State.replay(e2.journal, CTX)
        e3.apply(t)
        states = [e2, e3]
        # Two independently seeded states, both with retained and jumble twists.
        for seed in (202610091, 202610092):
            rng = random.Random(seed)
            st = sim.State(CTX)
            c = rng.randrange(600)
            for tw in (sim.a4_element(CTX, c, rng.randrange(1, 12)),
                       sim.cayley(CTX, c, [Q5(rng.randrange(1, 4), 0, 17), ZERO, ZERO])):
                self.assertTrue(st.apply(tw).applied)
            states.append(st)
        for st in states:
            grips = [0, 1, 13, 108] + random.Random(20261009).sample(range(600), 4)
            for e in grips:
                a = st.classify(e)
                b = st.classify_per_piece(e)
                self.assertEqual(a.status, b.status)
                self.assertEqual(b.counts['group_superset'], 0)
                self.assertEqual(b.filtered_evaluations, 0)
                self.assertEqual(b.counts['piece_vertices'], st.off_lattice_count())
                if a.status == 'admissible':
                    self.assertTrue(np.array_equal(a.inside, b.inside))

    def test_filter_certificate_fields(self):
        _, _, g, t = witness_twists()
        st = sim.State(CTX, filtered=True)
        st.apply(g)
        for e, next_twist in ((13, t), (0, None)):
            cl = st.classify(e, record=True)
            self.assertGreater(cl.filtered_evaluations, 0)
            self.assertEqual(len(cl.decisions), cl.filtered_evaluations + cl.exact_evaluations)
            self.assertEqual(sum(r['method'] == 'filtered' for r in cl.decisions), cl.filtered_evaluations)
            for rec in cl.decisions:
                self.assertEqual(rec['state_digest'], st.digest())
                self.assertEqual(rec['model_identity'], CTX.data.identity)
                self.assertIn('arithmetic_version', rec)
                self.assertIn('error_bound_version', rec)
                key = rec['pose_key']
                pose = CTX.kplus.matrix(key[1]) if key[0] == 'K' else matrix_from_json(key[1])
                vf = CTX.regions.cap_vform(rec['anchor']) if 'anchor' in rec else CTX.regions.vform(rec['piece'])
                self.assertEqual(rec['vertex_form'], {'denominator': vf[0], 'row': list(vf[1][rec['vertex']])})
                ev = Evaluator(pullback_form(pose, CTX.data.N[e]))
                self.assertEqual(rec['accepted_sign'], ev.sign_row(vf[0], vf[1][rec['vertex']]))
                self.assertEqual(rec['coverage']['vertex_count'], len(vf[1]))
                if rec['coverage']['kind'] == 'constraint_superset':
                    self.assertTrue(rec['coverage']['all_member_signatures_contain_anchor'])
                    self.assertEqual(rec['coverage']['constraint_subset']['inside_cuts'], [rec['anchor']])
                if rec['method'] == 'filtered':
                    lo, hi = rec['enclosure']
                    self.assertTrue(0 < lo <= hi or lo <= hi < 0)
                    self.assertEqual(1 if lo > 0 else -1, rec['accepted_sign'])
                else:
                    self.assertIn('fallback_reason', rec)
            if next_twist is not None:
                st.apply(next_twist)
    def test_e2_e3_e4(self):
        c, d, g, T = witness_twists()
        gw, _, _ = W.witness_rotation(c, d)
        self.assertEqual(g.matrix, to_tuple(gw))
        self.assertFalse(g.retained)
        self.assertTrue(T.retained)
        st = sim.State(CTX)
        solved = st.snapshot()
        self.assertTrue(st.apply(g).applied)
        sv = st.survey()
        b2 = sorted(e for e, r in sv.items() if r['status'] == 'blocked')
        self.assertEqual(b2, WITNESS['E2']['admissible_after_g']['blocked'])
        before = st.snapshot()
        neg = st.apply(sim.Twist(CTX, 1, W.a4(1)[1]))
        self.assertEqual(neg.status, 'blocked')
        self.assertEqual(st.snapshot(), before)
        self.assertEqual(neg.straddle['piece'], WITNESS['E3_negative_control']['certificate']['piece'])
        self.assertTrue(st.apply(T).applied)
        sv = st.survey()
        b3 = sorted(e for e, r in sv.items() if r['status'] == 'blocked')
        self.assertEqual(b3, WITNESS['E3']['blocked_after_g_then_t'])
        cert = sv[c]['certificate']
        for key, sign in (('point_below', -1), ('point_above', 1)):
            x = [q5_from_json(v) for v in cert[key]]
            self.assertEqual(W.h(c, x).sign(), sign)
            self.assertTrue(sim.posed_point_check(CTX.data, CTX.data.signature(cert['piece']), st.pose(cert['piece']), x))
        st.undo()
        st.undo()
        self.assertEqual(st.snapshot(), solved)

    def test_filtered_survey_equals_exact(self):
        c, d, g, T = witness_twists()
        a = sim.State(CTX)
        b = sim.State(CTX, filtered=True)
        self.assertFalse(sim.State(CTX).filtered)
        for st in (a, b):
            st.apply(g)
            st.apply(T)
        grips = list(range(0, 120))
        sa, sb = a.survey(grips), b.survey(grips)
        for e in grips:
            self.assertEqual(sa[e]['status'], sb[e]['status'])
            self.assertEqual(sa[e]['certificates'], sb[e]['certificates'])


class RuleTest(unittest.TestCase):
    def test_uncertain_is_rejected_without_change(self):
        st = sim.State(CTX)
        st.apply(sim.plane(CTX, 0, 13, degrees=10))
        before = st.snapshot()
        rot = np.eye(4)
        rot[2:, 2:] = [[math.cos(1.0), -math.sin(1.0)], [math.sin(1.0), math.cos(1.0)]]
        self.assertEqual(st.apply(sim.UnrepresentableTwist(0, rot)).status, 'uncertain')
        self.assertEqual(st.snapshot(), before)
        out = st.apply(sim.a4_element(CTX, 13, 1), budget=100)
        self.assertEqual(out.status, 'uncertain')
        self.assertEqual(st.snapshot(), before)
        self.assertTrue(st.apply(sim.a4_element(CTX, 13, 1)).applied)

    def test_commit_is_all_or_nothing(self):
        st = sim.State(CTX)
        st.apply(sim.plane(CTX, 0, 13, degrees=10))
        before = st.snapshot()
        original = st._compose

        def failing(twist, pid):
            original(twist, pid)          # interns a new pose, then fails
            raise RuntimeError('injected failure')
        st._compose = failing
        with self.assertRaises(RuntimeError):
            st.apply(sim.cayley(CTX, 0, [Q5(1, 0, 7), ZERO, ZERO]))
        del st._compose
        self.assertEqual(st.snapshot(), before)
        self.assertTrue(st.apply(sim.cayley(CTX, 0, [Q5(1, 0, 7), ZERO, ZERO])).applied)

    def test_invalid_twists_are_refused(self):
        with self.assertRaises(sim.TwistError):
            sim.Twist(CTX, 5, sim.plane(CTX, 0, 13, degrees=10).matrix)
        with self.assertRaises(sim.TwistError):
            sim.Twist(CTX, 0, [[Q5(2) if i == j else ZERO for j in range(4)] for i in range(4)])
        rec = sim.plane(CTX, 0, 13, degrees=10).record()
        rec['params']['s'] = [16, 0, 961]
        with self.assertRaises(sim.TwistError):
            sim.Twist.from_record(CTX, rec)
        with self.assertRaises(sim.TwistError):
            sim.half_turn(CTX, 3, [ZERO, ZERO, ZERO])

    def test_shallow_crossing_is_blocked(self):
        st = sim.State(CTX)
        st.apply(sim.plane(CTX, 0, 13, s=Q5(1, 0, 10 ** 9)))
        sv = st.survey(range(1, 13))
        blocked = [e for e, r in sv.items() if r['status'] == 'blocked']
        self.assertTrue(blocked)
        cert = sv[blocked[0]]['certificate']
        self.assertLess(min(abs(cert['h_below_float']), abs(cert['h_above_float'])), 1e-8)
        before = st.snapshot()
        self.assertEqual(st.apply(sim.a4_element(CTX, blocked[0], 2)).status, 'blocked')
        self.assertEqual(st.snapshot(), before)

    def test_same_grip_twists_compose(self):
        g1 = sim.plane(CTX, 0, 13, degrees=10)
        g2 = sim.cayley(CTX, 0, [Q5(1, 0, 9), Q5(1, 0, 20), ZERO])
        a = sim.State(CTX)
        a.apply(g1)
        a.apply(g2)
        b = sim.State(CTX)
        b.apply(sim.Twist(CTX, 0, matmul(g2.matrix, g1.matrix)))
        self.assertEqual(a.digest(), b.digest())

    def test_random_sequence_round_trip(self):
        rng = random.Random(6)
        nf = CTX.data.NF
        near = [list(np.argsort(-(nf @ nf[c]))[:57]) for c in range(600)]
        st = sim.State(CTX)
        solved = st.snapshot()
        recent = [0]
        families = set()
        while len(st.journal) < 8 or len(families) < 3:
            c = int(rng.choice(near[rng.choice(recent)]))
            kind = ['retained', 'plane', 'cayley'][len(st.journal) % 3]
            try:
                if kind == 'retained':
                    tw = sim.a4_element(CTX, c, rng.randrange(1, 12))
                elif kind == 'plane':
                    tw = sim.plane(CTX, c, int(rng.choice(near[c][1:17])), degrees=rng.uniform(4, 30), max_den=500)
                else:
                    tw, _ = sim.cayley_axis_angle(CTX, c, [rng.gauss(0, 1) for _ in range(3)], rng.uniform(4, 40), max_den=64)
            except sim.TwistError:
                continue
            before = st.snapshot()
            out = st.apply(tw)
            if out.applied:
                recent.append(c)
                families.add(tw.family)
            else:
                self.assertIn(out.status, ('blocked', 'uncertain'))
                self.assertEqual(st.snapshot(), before)
        self.assertGreater(st.off_lattice_count(), 0)
        rep = sim.State.replay(st.journal_json(), CTX)
        self.assertEqual(rep.snapshot(), st.snapshot())
        while st.journal:
            st.undo()
        self.assertEqual(st.snapshot(), solved)


class TwistTest(unittest.TestCase):
    def test_input_map_is_exhaustive_minimum(self):
        for axis, deg, n, d in (([1, 2, 3], 67, 2, 5), ([0.2, -0.8, 0.3], 137, 3, 4),
                                ([1, 3, 2], 179.6, 2, 4), ([1, 2, -3], -179.7, 2, 3),
                                ([1, 0, 0], 180, 2, 7), ([1, 2, 3], 0, 2, 4),
                                ([1, 0, 0], 10, 0, 4)):
            expected, choice = brute_input_map(axis, deg, n, d)
            num, den, report = nearest_parameter(axis, deg, d, n)
            self.assertAlmostEqual(report['minimum_distance'], expected, places=13)
            self.assertEqual((den, tuple(num), report['family']), choice)
            self.assertLessEqual(report['distance'], expected + 1e-12)
            tw, _ = sim.cayley_axis_angle(CTX, 9, axis, deg, max_num=n, max_den=d)
            self.assertEqual(sim.Twist.from_record(CTX, tw.record()).matrix, tw.matrix)
            self.assertEqual(tw.params['requested']['N'], n)
            self.assertEqual(tw.params['requested']['D'], d)
            self.assertEqual(tw.params['requested']['distance'], report['distance'])
        for kwargs in ({'max_num': -1}, {'max_den': 0}, {'max_den': 1.5}):
            with self.assertRaises(sim.TwistError):
                sim.cayley_axis_angle(CTX, 0, [1, 0, 0], 10, **kwargs)

    def test_half_turn_and_negative_control(self):
        generic = sim.half_turn(CTX, 37, [Q5(1, 1, 7), Q5(-2, 1, 9), Q5(0, 1, 11)])
        self.assertEqual(matmul(generic.matrix, generic.matrix), identity(4))
        self.assertEqual(generic.inverse().matrix, generic.matrix)
        self.assertEqual(sim.Twist.from_record(CTX, generic.record()).matrix, generic.matrix)
        g = negative_control()
        self.assertEqual(g.family, 'half_turn')
        self.assertFalse(g.retained)
        self.assertEqual(matvec(g.matrix, CTX.data.N[13]), CTX.data.N[13])
        self.assertEqual(matmul(g.matrix, g.matrix), identity(4))
        self.assertEqual(g.inverse().matrix, g.matrix)
        product = matmul(sim.generator(CTX, 0).matrix, g.matrix)
        self.assertEqual(sum((product[i][i] for i in range(4)), ZERO), Q5(4, 0, 3))
        self.assertEqual(sim.Twist.from_record(CTX, g.record()).matrix, g.matrix)
        bad = json.loads(json.dumps(g.record()))
        bad['params']['axis'] = [[1, 0, 1], [0, 0, 1], [0, 0, 1]]
        with self.assertRaises(sim.TwistError):
            sim.Twist.from_record(CTX, bad)
        menu = sim.TwistMenu(CTX, 'NC', [('g', g.matrix)])
        st = sim.State(CTX, menu=menu)
        digests = [st.digest()]
        heights = []
        centre = int(CTX.data.centre_pieces[CTX.data.centre_poles == 0][0])
        for _ in range(12):
            for tw in (sim.generator(CTX, 0), g):
                self.assertTrue(st.apply(tw).applied)
                digests.append(st.digest())
            heights.append(max(max(abs(x.a), abs(x.b), x.d) for row in st.pose(centre) for x in row))
        self.assertEqual(len(set(digests)), 25)
        self.assertGreater(heights[-1], heights[0])
        self.assertEqual(sim.State.replay(st.journal_json(), CTX, menu=menu).digest(), st.digest())
        while st.journal:
            st.undo()
        self.assertEqual(st.moved_count(), 0)

    def test_unrepresentable_seventh_turn(self):
        st = sim.State(CTX)
        before = st.snapshot()
        angle = 2 * math.pi / 7
        r = np.eye(4)
        r[2:, 2:] = [[math.cos(angle), -math.sin(angle)], [math.sin(angle), math.cos(angle)]]
        out = st.apply(sim.UnrepresentableTwist(0, r, 'trace of a seventh turn is outside Q(sqrt5)'))
        self.assertEqual(out.status, 'uncertain')
        self.assertEqual(st.snapshot(), before)
    def test_plane_is_cayley(self):
        for c, d, s in ((0, 13, Q5(15, 0, 961)), (77, int(np.argsort(-(CTX.data.NF @ CTX.data.NF[77]))[3]), Q5(3, -1, 40))):
            om = [-s * dot(u, CTX.data.N[d]) for u in sim.cap_frame(CTX.data.N[c])]
            self.assertEqual(sim.plane(CTX, c, d, s=s).matrix, sim.cayley(CTX, c, om).matrix)

    def test_axis_angle_mapping(self):
        tw, rep = sim.cayley_axis_angle(CTX, 7, [1, 2, 2], 25, max_den=200)
        self.assertLess(rep['rotation_error_deg'], 0.5)
        self.assertAlmostEqual(rep['realised_angle_deg'], tw.angle_deg(), places=9)
        self.assertFalse(tw.retained)
        tw, _ = sim.cayley_axis_angle(CTX, 7, [1, 1, 1], 120, max_den=10)
        self.assertTrue(tw.retained)
        self.assertEqual(tw.inverse().matrix, to_tuple(transpose(tw.matrix)))

    def test_menus(self):
        menu = sim.TwistMenu.a4(CTX)
        self.assertEqual(len(menu), 12)
        self.assertTrue(menu.invariant)
        for c in (0, 13, 333):
            self.assertEqual({t.matrix for t in menu.for_cap(c)}, {sim.a4_element(CTX, c, i).matrix for i in range(12)})
        st = sim.State(CTX, menu=menu)
        before = st.snapshot()
        self.assertEqual(st.apply(sim.plane(CTX, 0, 13, degrees=10)).status, 'invalid')
        self.assertEqual(st.snapshot(), before)
        cm = sim.TwistMenu.cayley_set(CTX, 'quarter', [[Q5(1, 0, 4), ZERO, ZERO]])
        self.assertTrue(cm.invariant)
        self.assertTrue(all(cm.contains(t) for t in cm.for_cap(42)))
        self.assertFalse(cm.contains(sim.cayley(CTX, 42, [Q5(1, 0, 5), ZERO, ZERO])))

    def test_menu_closure_validation_identity_and_transport(self):
        a4 = sim.TwistMenu.a4(CTX)
        s4 = sim.TwistMenu.s4(CTX)
        nc = sim.TwistMenu(CTX, 'NC', [('g', negative_control().matrix)])
        self.assertEqual(len(s4), 24)
        for menu in (a4, s4, nc):
            rec = menu.record()
            self.assertTrue(all(rec[k] for k in ('inverse_closed', 'a4_invariant', 'contains_a4')))
            self.assertEqual(len(rec['identity']), 64)
            self.assertEqual(sim.TwistMenu.from_record(CTX, rec).identity, menu.identity)
            reordered = sim.TwistMenu(CTX, 'renamed', list(reversed(menu.items)), close=False)
            self.assertEqual(menu.identity, reordered.identity)
            for cap in (0, 13, 599):
                twists = {t.matrix for t in menu.for_cap(cap)}
                for a in CTX.kplus.stab0:
                    t = CTX.kplus.compose(int(CTX.kplus.frame_idx[cap]), a)
                    self.assertEqual(twists, {tw.matrix for tw in menu.for_cap(cap, transporter=t)})
            rec['identity'] = '0' * 64
            with self.assertRaises(sim.TwistError):
                sim.TwistMenu.from_record(CTX, rec)
        g = sim.cayley(CTX, 0, [Q5(1, 0, 7), Q5(2, 0, 7), Q5(3, 0, 7)]).matrix
        conjugates = [(str(i), to_tuple(matmul(matmul(CTX.kplus.matrix(a), g), transpose(CTX.kplus.matrix(a)))))
                      for i, a in enumerate(CTX.kplus.stab0)]
        for base in ([('identity', CTX.kplus.matrix(0))],
                     a4.items + [('g', g), ('inverse', to_tuple(transpose(g)))],
                     a4.items + conjugates):
            with self.assertRaises(sim.TwistError):
                sim.TwistMenu(CTX, 'invalid', base, close=False)
        # A nearby float request cannot be rounded into a menu element, even if the
        # bounded search's best candidate is retained.
        tw, _ = sim.cayley_axis_angle(CTX, 0, [1, 1, 1], 119, max_num=1, max_den=1)
        self.assertTrue(tw.retained)
        self.assertEqual(sim.State(CTX, menu=a4).apply(tw).status, 'invalid')
        with self.assertRaises(sim.TwistError):
            sim.cayley_axis_angle(CTX, 0, [1, 1, 1], 119, max_num=1, max_den=1, menu=a4)


class CheckpointTest(unittest.TestCase):
    def test_global_rotation_control(self):
        kp = CTX.kplus
        solved = sim.State(CTX).digest()
        moving_pole = int(np.flatnonzero(kp.perms[:, 0] != 0)[0])
        for k in (kp.stab0[1], moving_pole):
            st = sim.State.from_global_rotation(CTX, k)
            self.assertTrue(st.is_lattice())
            self.assertNotEqual(st.digest(), solved)
            self.assertTrue(all(st.pose(p) == kp.matrix(k) for p in (0, 599, 177119)))
            lab = st.lattice_stickers()
            self.assertTrue(np.array_equal(np.sort(lab['slot']), np.arange(NS)))
            self.assertTrue(lab['frame_agrees'].all())
            for witness in (None, [], [1, 2, -1]):
                self.assertFalse(st.checkpoint(witness)['checkpoint'])
                with self.assertRaises(ValueError):
                    st.export_retained(witness)

    def test_journal_identities_and_legacy_replay(self):
        menu = sim.TwistMenu.a4(CTX)
        st = sim.State(CTX, menu=menu)
        st.apply(sim.generator(CTX, 0))
        doc = json.loads(st.journal_json())
        self.assertEqual(doc['model_identity'], CTX.data.identity)
        self.assertEqual(doc['menu_identity'], menu.identity)
        self.assertEqual(doc['contract_revision'], CONTRACT_REVISION)
        self.assertEqual(sim.State.replay(doc, CTX, menu=menu).snapshot(), st.snapshot())
        for key in ('model_identity', 'menu_identity', 'contract_revision'):
            altered = dict(doc, **{key: 'wrong'})
            with self.assertRaises(ValueError):
                sim.State.replay(altered, CTX, menu=menu)
        with self.assertRaises(ValueError):
            sim.State.replay(doc, CTX)
        legacy = {'format': doc['format'], 'records': doc['records']}
        self.assertEqual(sim.State.replay(json.dumps(legacy), CTX).snapshot(), st.snapshot())
    def test_checkpoints_and_export(self):
        rng = random.Random(8)
        word = [rng.choice([1, -1]) * rng.randrange(1, 1201) for _ in range(12)]
        st = sim.State(CTX)
        for mid in word:
            st.apply(sim.primitive(CTX, mid))
        labels, cp = st.export_retained()
        self.assertTrue(cp['checkpoint'])
        self.assertTrue(np.array_equal(labels, ref_labels(word)))
        # jumble excursion merging into a retained twist of the same grip
        g = sim.plane(CTX, 0, 13, degrees=10)
        a = sim.a4_element(CTX, 0, 7)
        st.apply(g)
        self.assertFalse(st.checkpoint()['checkpoint'])
        with self.assertRaises(ValueError):
            st.export_retained()
        st.apply(sim.Twist(CTX, 0, matmul(a.matrix, transpose(g.matrix))))
        labels, cp = st.export_retained()
        self.assertTrue(np.array_equal(labels, ref_labels(cp['witness_word'])))
        # a lattice state through jumble twists that do not cancel needs a supplied witness
        far = int(np.argmin(CTX.data.NF @ CTX.data.NF[0]))
        st = sim.State(CTX)
        b = sim.a4_element(CTX, far, 2)
        for tw in (g, b, g.inverse()):
            self.assertTrue(st.apply(tw).applied)
        self.assertTrue(st.is_lattice())
        self.assertFalse(st.checkpoint()['checkpoint'])
        self.assertTrue(st.checkpoint(b.params['word'])['checkpoint'])
        self.assertFalse(st.checkpoint(b.params['word'] * 2)['checkpoint'])

    def test_off_lattice_sticker_frames(self):
        st = sim.State(CTX)
        g = sim.plane(CTX, 0, 13, degrees=10)
        st.apply(g)
        kp = CTX.kplus
        for s in (0, 17, 432):
            f = s // CELL_SLOTS
            fr = st.sticker_frame(s)
            self.assertEqual(fr, to_tuple(matmul(g.matrix, kp.matrix(int(kp.frame_idx[f])))))
            self.assertEqual(matvec(fr, CTX.data.N[0]), matvec(g.matrix, CTX.data.N[f]))


if __name__ == '__main__':
    unittest.main(verbosity=2)
