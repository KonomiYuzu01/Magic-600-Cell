"""R22D-19: a finite, state-only search for a literal height-4 strict-prime step.

Run from the repository root with Python's -I -B options::

    python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22d19_rule.py \
        (--table TABLE | --record RECORD) --out DIR [--cache DIR] [--seconds 3600] \
        [--case '{"unit":391,"cap":61,"item":57}']

Only current poses survive input loading. No certificate, journal-based search,
random ordering, or simulator pose-table index comparison is used. Point actions
use e3_class.Model's chronological operations. Unlike Model2 in cellpure5.py,
their domain includes *every* home cell, including completely vacant cells.

The elapsed-time budget includes imports, input reconstruction and enumeration.
It is cooperative: an exact simulator operation, input reconstruction, or undo
finishes before timeout is reported. A timeout before a lowering case identifies
the initialization phase instead. Outputs and the default cache live under out.
Until the enumeration ends, report.json says budget and names the case reached,
so an interrupted run never reads as stuck; stuck is written only after every
selected lowering case was enumerated. A --case that matches no lowering case is
an error, not a stuck result.

Counting units of `rejected`: split_cells, not_original_off, no_free_grip and
fixed_occupied count lowering cases (steps 1 and 2); no_landing_power counts
rejected powers e, plus one for a lowering case with no commutator to try;
no_vacator counts rejected V; replay_failed counts failed step-5 replays.
Evidence: exact synthetic geometry, headless Linux; research only.
"""

import argparse
from collections import Counter, defaultdict
from contextlib import contextmanager
import hashlib
import json
import math
import os
from pathlib import Path
import sys
import time
from types import SimpleNamespace

PROCESS_START = time.monotonic()
sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ.setdefault(_name, '1')

REPO = Path(__file__).resolve().parents[6]
SCRIPTS = REPO / 'research/jumbling/restoration/12-low-height/scripts'
sys.path[:0] = [str(SCRIPTS), str(REPO / 'research/jumbling/theory'),
                str(REPO / 'research/jumbling'), str(REPO / 'research/jumbling/fixtures')]
import numpy as np  # noqa: E402
import buffer as B  # noqa: E402
import e3_class as E3  # noqa: E402
import eb6 as E  # noqa: E402
from exact import matmul, matvec, transpose  # noqa: E402
from screen import load as screen_load  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

R, T, sim = E.R, E.T, E.sim
REASONS = ('split_cells', 'not_original_off', 'no_free_grip', 'fixed_occupied',
           'no_landing_power', 'no_vacator', 'replay_failed')
CHECK_FIELDS = ('strict_prime', 'collateral', 'others_exact', 'others_landed', 'off_before', 'off_after')


class Budget(Exception):
    pass


class Deadline:
    def __init__(self, seconds=math.inf, started=None):
        self.started = time.monotonic() if started is None else started
        self.limit = seconds
        self.reached = {'phase': 'input', 'unit': None, 'cap': None, 'item': None}

    def tick(self, **position):
        self.reached.update(position)
        if time.monotonic() - self.started >= self.limit:
            raise Budget


def configure_cache(directory):
    """Redirect both this process and any child using orient.start/seams."""
    path = Path(directory).resolve() / 'home_centroids.npy'
    os.environ['JUMBLING_HOME_CACHE'] = str(path)
    E3.seams.CACHE = path


def copy_state(st, menu):
    """Fresh mutable simulator state; pose values themselves are immutable."""
    out = sim.State(st.ctx, filtered=False, menu=menu)
    out.pose_id = st.pose_id.copy()
    out._poses = list(st._poses)
    out._lookup = dict(st._lookup)
    return out


def load_current(kind, path):
    st, menu, _ = screen_load(kind, str(path))
    st.journal = []
    assert not st.filtered
    return st, menu


def pose_info(st):
    return {pid: tuple(int(x) for x in T.pose_tree(st._pose_matrix(pid)))
            for pid in np.unique(st.pose_id[~st.lattice_flags()]).tolist()}


def projective(q):
    v = next(x for x in q if not x.is_zero())
    return tuple(x / v for x in q)


class ClassGeometry:
    """Exact cosets and all home-cell fibres of one left-tree branch."""
    def __init__(self, rule, branch):
        self.poles = [c for c in range(600) if rule.branches[c] == branch]
        assert len(self.poles) == 120
        self.kof = {c: k for k, c in enumerate(self.poles)}
        units = {projective(u) for u in T.UNITS}

        def is_unit(q):
            return projective(q) in units

        c = self.poles[0]
        reps = []
        for u in T.UNITS:
            a = T.qmul(T.qmul(E3.CL.N[c], u), E3.CL.ninv(c))
            if not any(is_unit(T.qmul(T.qconj(r), a)) for r in reps):
                reps.append(a)
        reps.sort(key=lambda r: not is_unit(r))
        assert len(reps) == 5 and is_unit(reps[0])
        self.reps, self.is_unit = reps, is_unit
        # This is the same exact proportional-unit test as E3.prop_unit, using
        # canonical projective keys rather than repeatedly scanning 120 units.
        packed = np.packbits(rule.home[:, self.poles], axis=1)
        keys, inverse = np.unique(packed, axis=0, return_inverse=True)
        self.home_keys = [int.from_bytes(row.tobytes(), 'big') for row in keys]
        order = np.argsort(inverse, kind='stable')
        cuts = np.cumsum(np.bincount(inverse))[:-1]
        self.home = dict(zip(self.home_keys, np.split(order, cuts)))
        assert sum(map(len, self.home.values())) == len(rule.Z.pose_id)
        # The full 600-cell complex, using screen.py's exact quaternion
        # adjacency construction. Some of these cells have no home chambers.
        # Seeding the full complex also closes every free letter's action.
        O = B.O
        inverse = E3.CL.ninv(self.poles[0])
        w = [O.UIDX[tuple(T.qmul(inverse, E3.CL.N[c]))] for c in self.poles]
        neighbours = {i for i, u in enumerate(O.U) if u[0] == O.HALF_PHI}
        adj = {c: {d for j, d in enumerate(self.poles)
                   if int(O.MUL[O.INV[w[i]], w[j]]) in neighbours}
               for i, c in enumerate(self.poles)}
        assert all(len(a) == 12 for a in adj.values())
        edges = {frozenset((a, b)) for a in self.poles for b in adj[a]}
        triangles = {e | {c} for e in edges for a, b in [tuple(e)] for c in adj[a] & adj[b]}
        tetrahedra = {t | {d} for t in triangles for d in set.intersection(*(adj[c] for c in t))}
        assert (len(edges), len(triangles), len(tetrahedra)) == (720, 1200, 600)
        self.cells = {0} | {1 << (119 - i) for i in range(120)}
        self.cells.update(sum(1 << (119 - self.kof[c]) for c in face)
                          for face in edges | triangles | tetrahedra)
        assert len(self.cells) == 2641 and set(self.home) <= self.cells

    def label(self, left):
        labels = [i for i, r in enumerate(self.reps)
                  if self.is_unit(T.qmul(T.qconj(r), left))]
        assert len(labels) == 1, labels
        return labels[0]


class Lowered:
    """Data derived solely from Y. Laziness preserves the step-1/step-2 order."""
    def __init__(self, rule, st):
        self.rule, self.st = rule, copy_state(st, rule.M)
        self.info = pose_info(self.st)
        self.off = ~self.st.lattice_flags()
        self._contents = {}
        self._alphabets = None
        self._safe_alphabet = None
        self._models = {}

    def contents(self, branch, clock):
        if branch in self._contents:
            return self._contents[branch]
        rule, st = self.rule, self.st
        geom = rule.geometry(branch)
        contents, point = defaultdict(list), {}
        for pid, (height, right, b, _) in self.info.items():
            clock.tick(phase='step1-content')
            if (height, right, b) != (2, 0, branch):
                continue
            ps = np.flatnonzero(st.pose_id == pid)
            q = st._pose_matrix(pid)
            left, rightq = T.factors(q)
            assert geom.is_unit(rightq)
            label = geom.label(left)
            assert label != 0
            # Own frames map this class's poles exactly to home poles. Thus
            # pulled-back home signatures classify entire pieces, without a
            # centroid or floating-point approximation.
            invq = transpose(q)
            back = [E3.CL.pole(matvec(invq, E3.CL.N[c])) for c in geom.poles]
            assert min(back) >= 0
            packed = np.packbits(rule.home[np.ix_(ps, back)], axis=1)
            for p, row in zip(ps.tolist(), packed):
                key = (int.from_bytes(row.tobytes(), 'big'), label)
                contents[key].append(p)
                point[p] = key
        result = ({k: np.array(sorted(ps), np.int64) for k, ps in contents.items()}, point)
        self._contents[branch] = result
        return result

    def alphabets(self, clock):
        if self._alphabets is not None:
            return self._alphabets
        # Compute both fixed cap alphabets in one exact classification pass.
        own_branch = np.full(len(self.st.pose_id), -2, np.int8)
        own_branch[~self.off] = -1
        for pid, (height, right, b, _) in self.info.items():
            if height == 2 and right == 0:
                own_branch[self.st.pose_id == pid] = b
        free, safe = defaultdict(list), []
        for c in range(600):
            clock.tick(phase='step2-alphabets', grip=c)
            cl = self.st.classify(c, need_inside=True, stop_at_straddle=True)
            assert not cl.uncertain
            if cl.straddle is not None:
                continue
            inside = own_branch[cl.inside]
            branch = self.rule.branches[c]
            if ((inside == -1) | (inside == branch)).all():
                free[branch].append(c)
            if (inside == -1).all():
                safe.append(c)
        holes = np.zeros(len(self.st.pose_id), bool)
        holes[B.hole_set(self.st)] = True
        self._alphabets = (dict(free), safe, holes)
        return self._alphabets

    def safe_alphabet(self, clock):
        if self._safe_alphabet is None:
            clock.tick(phase='step2-safe-letters')
            alphabet = B.Alphabet(SimpleNamespace(st=self.st), self.rule.M)
            free, safe, holes = self.alphabets(clock)
            assert alphabet.safe == safe
            assert np.array_equal(alphabet.holes, np.flatnonzero(holes))
            self._safe_alphabet = alphabet
        return self._safe_alphabet

    def model(self, branch, clock):
        if branch not in self._models:
            clock.tick(phase='step3-model')
            self._models[branch] = PointModel(self, branch, clock)
        self._models[branch].clock = clock
        return self._models[branch]


class _Rows(dict):
    """Materialize exact point actions as enumeration reaches their letters."""
    def __init__(self, model):
        super().__init__()
        self.model = model

    def __missing__(self, n):
        row = self.model.letter_row(n)
        self[n] = row
        return row


class PointModel(E3.Model):
    """Model2's off contents, extended by all home cells, with lazy actions.

    Domain/label equivalence here is exact: right factors are projective 2I
    units, and the five left cosets are exactly the five frame domains. Every
    other class-Q off content, including another label in D's cell, is kept.
    """
    def __init__(self, lowered, branch, clock):
        self.lowered, self.clock = lowered, clock
        self.geom = geom = lowered.rule.geometry(branch)
        self.poles, self.kof = geom.poles, geom.kof
        self.A = lowered.alphabets(clock)[0].get(branch, [])
        self.contents = lowered.contents(branch, clock)[0]
        assert {key for key, _ in self.contents} <= geom.cells
        self.cells = sorted(geom.cells)
        self.cid = {s: i for i, s in enumerate(self.cells)}
        self.NC = len(self.cells)
        self.ncaps = np.array([s.bit_count() for s in self.cells])
        self.ar = np.arange(self.NC * 5, dtype=np.int32)
        self.home_points = np.array([self.cid[s] * 5 for s in geom.home], np.int32)
        self.home_size = np.array([len(geom.home[s]) for s in geom.home], np.int64)
        self.is_home = np.zeros(self.NC * 5, bool)
        self.is_home[self.home_points] = True
        self.gens = [(c, i) for c in self.A for i in range(1, 60)]
        self.index = {pair: n for n, pair in enumerate(self.gens)}
        self.byc = defaultdict(list)
        for n, (c, _) in enumerate(self.gens):
            self.byc[c].append(n)
        inverse = lowered.rule.inverse_items
        self.inv = np.array([self.index[c, inverse[i]] for c, i in self.gens], np.int64)
        self.G = _Rows(self)

    def letter_row(self, n):
        self.clock.tick(phase='step3-letter')
        c, item = self.gens[n]
        g = R.element(self.lowered.rule.M, c, item)
        img = [self.kof.get(E3.CL.pole(matvec(g, E3.CL.N[d])), -1) for d in self.poles]
        assert min(img) >= 0 and len(set(img)) == 120
        left, right = T.factors(g)
        assert self.geom.is_unit(right)
        act = [self.geom.label(T.qmul(left, r)) for r in self.geom.reps]
        assert sorted(act) == list(range(5))
        row, bit = self.ar.copy(), 119 - self.kof[c]
        for s in self.cells:
            if s >> bit & 1:
                target = self.move(s, img)
                # Never silently drop a cell missing from the original
                # current-content closure, or a vacant home cell.
                assert target in self.cid, 'point domain is not closed'
                f, t = self.cid[s], self.cid[target]
                row[f * 5:f * 5 + 5] = [t * 5 + a for a in act]
        assert np.array_equal(np.sort(row), self.ar)
        return row

    def protected(self, target):
        cs = sorted({self.cid[s] for s, label in self.contents if (s, label) != target})
        return (np.array(cs, np.int32)[:, None] * 5 + np.arange(5)).ravel()

    def bad(self, action):
        keys = [s for s, p in zip(self.geom.home, self.home_points) if action[p] % 5 != 0]
        chambers = (np.sort(np.concatenate([self.geom.home[s] for s in keys]))
                    if keys else np.empty(0, np.int64))
        return chambers, [self.caps(s) for s in keys]


class Rule:
    def __init__(self, st, menu):
        self.Z = copy_state(st, menu)
        self.M = menu
        self.digest = self.Z.digest()
        self.original_off = ~self.Z.lattice_flags()
        self.info = pose_info(self.Z)
        self.heights = dict(sorted(Counter(str(self.info[int(p)][0])
                                          for p in self.Z.pose_id[self.original_off]).items()))
        self.branches = [T.branch(tuple(n)) for n in st.ctx.data.N]
        items = [to_tuple(m) for _, m in menu.items]
        item_index = {m: i for i, m in enumerate(items)}
        self.inverse_items = [item_index[to_tuple(transpose(m))] for m in items]
        self._home, self._geometry, self._lowered = None, {}, {}
        self.units = None
        self.scratch = copy_state(self.Z, menu)

    @property
    def home(self):
        if self._home is None:
            self._home = self.Z.ctx.data.sig_matrix(np.arange(len(self.Z.pose_id)))
        return self._home

    def geometry(self, branch):
        if branch not in self._geometry:
            self._geometry[branch] = ClassGeometry(self, branch)
        return self._geometry[branch]

    def discover(self, clock):
        if self.units is not None:
            return self.units
        high = np.array([p for p in np.flatnonzero(self.original_off)
                         if self.info[int(self.Z.pose_id[p])][0] >= 4], np.int64)
        if not len(high):
            self.units = []
            return self.units
        ins = np.zeros((len(high), 600), bool)
        for pid in np.unique(self.Z.pose_id[high]).tolist():
            rows = np.flatnonzero(self.Z.pose_id[high] == pid)
            for c in range(600):
                clock.tick(phase='units', grip=c)
                ev = Evaluator(pullback_form(self.Z._pose_matrix(pid), self.Z.ctx.data.N[c]))
                for row in rows:
                    v = ev.classify(self.Z.ctx.regions.vform(int(high[row])))[0]
                    assert v in ('in', 'out', 'straddle')
                    ins[row, c] = v == 'in'
        groups = defaultdict(list)
        for row, p in enumerate(high.tolist()):
            groups[(int(self.Z.pose_id[p]), ins[row].tobytes())].append(row)
        units, clean = [], {}
        for (pid, _), rows in sorted(groups.items(), key=lambda kv: int(high[kv[1][0]])):
            height, right, branch, _ = self.info[pid]
            assert right == 0
            caps = []
            for c in np.flatnonzero(ins[rows[0]]).tolist():
                clock.tick(phase='units', unit=int(high[rows[0]]), cap=c)
                if self.branches[c] != branch:
                    continue
                if c not in clean:
                    cl = self.Z.classify(c, need_inside=True, stop_at_straddle=True)
                    assert not cl.uncertain
                    clean[c] = cl.straddle is None
                if clean[c]:
                    caps.append(c)
            lowering = {}
            for c in caps:
                lowering[c] = [i for i in range(1, 60)
                               if T.pose_tree(to_tuple(matmul(R.element(self.M, c, i),
                                                              self.Z._pose_matrix(pid))))[0] == height - 2]
                assert len(lowering[c]) == 12
            units.append({'unit': int(high[rows[0]]), 'pieces': high[rows],
                          'branch': branch, 'caps': caps, 'lowering': lowering})
        self.units = units
        return units

    @contextmanager
    def lowering(self, cap, item):
        """Every visit literally applies and undoes a, even when data is cached."""
        st = self.scratch
        assert st.digest() == self.digest and not st.journal
        o = st.apply(sim.Twist(st.ctx, cap, R.element(self.M, cap, item), 'menu', {'menu': self.M.name}))
        assert o.applied and not o.uncertain
        try:
            if (cap, item) not in self._lowered:
                self._lowered[cap, item] = Lowered(self, st)
            yield self._lowered[cap, item]
        finally:
            st.undo()
            assert st.digest() == self.digest and not st.journal, 'lowering undo changed Z'

    def filter_case(self, unit, cap, item, clock=None):
        """Steps 1 and 2 only. Returns (first rejection, context); never step 3."""
        clock = clock or Deadline()
        with self.lowering(cap, item) as y:
            clock.tick(phase='step1', unit=unit['unit'], cap=cap, item=item)
            keys = {y.info[int(y.st.pose_id[p])][:3] for p in unit['pieces']}
            if len(keys) != 1:
                return 'split_cells', None
            height, right, branch = next(iter(keys))
            if height != 2 or right != 0:
                return 'split_cells', None
            contents, point = y.contents(branch, clock)
            targets = {point[int(p)] for p in unit['pieces']}
            if len(targets) != 1:
                return 'split_cells', None
            target = next(iter(targets))
            if not self.original_off[contents[target]].all():
                return 'not_original_off', None
            free, safe, holes = y.alphabets(clock)
            geom = self.geometry(branch)
            caps = [c for c in geom.poles if target[0] >> (119 - geom.kof[c]) & 1]
            if not set(caps).intersection(free.get(branch, [])):
                return 'no_free_grip', None
            home = geom.home.get(target[0], np.empty(0, np.int64))
            fixed_occupied = (~holes[home]) & (~self.home[np.ix_(home, safe)].any(axis=1))
            if fixed_occupied.any():
                return 'fixed_occupied', None
            return None, (y, branch, target)

    def filters(self, clock=None):
        """Public filter-only census; no commutator or SAFE permutation is built."""
        clock = clock or Deadline()
        cases, rejected = [], dict.fromkeys(REASONS, 0)
        for unit in self.discover(clock):
            for cap in unit['caps']:
                for item in unit['lowering'][cap]:
                    reason, _ = self.filter_case(unit, cap, item, clock)
                    cases.append({'unit': unit['unit'], 'cap': cap, 'item': item, 'outcome': reason or 'keep'})
                    if reason:
                        rejected[reason] += 1
        return cases, rejected


def check_step5(rule, pieces, word, clock=None):
    """Fresh full-state replay of stepchk.py's strict-prime endpoint contract."""
    clock = clock or Deadline()
    st = copy_state(rule.Z, rule.M)
    pieces = np.asarray(pieces, np.int64)
    assert len(pieces) and rule.original_off[pieces].all()
    before = E.stable_pose(st)
    admissible = True
    for n, (c, item) in enumerate(word):
        clock.tick(phase='step5-replay', letter=n)
        o = st.apply(sim.Twist(st.ctx, c, R.element(rule.M, c, item), 'menu',
                               {'menu': rule.M.name}), journal=False)
        if not o.applied or o.uncertain:
            admissible = False
            break
    after_off = ~st.lattice_flags()
    others = rule.original_off.copy()
    others[pieces] = False
    exact = int((others & after_off & (E.stable_pose(st) == before)).sum())
    landed = int((others & ~after_off).sum())
    collateral = int((~rule.original_off & after_off).sum())
    unit_landed = int((~after_off[pieces]).sum())
    return {'strict_prime': bool(admissible and unit_landed == len(pieces) and collateral == 0
                                 and exact + landed == int(others.sum())),
            'all_admissible': admissible, 'collateral': collateral, 'others_exact': exact,
            'others_landed': landed, 'off_before': int(rule.original_off.sum()), 'off_after': int(after_off.sum()),
            'unit_size': len(pieces), 'unit_on_lattice': unit_landed, 'others': int(others.sum()),
            'start_digest': rule.digest, 'end_digest': st.digest(), 'letters': len(word)}


def search_case(rule, unit, context, row, rejected, selection, clock):
    y, branch, target = context
    alphabet = y.safe_alphabet(clock)
    m = y.model(branch, clock)
    point = m.cid[target[0]] * 5 + target[1]
    protected = m.protected(target)
    pairs = m.adjacent(set(m.caps(target[0])))
    for c, d in pairs:
        if ('x' in selection and selection['x'][0] != c) or ('y' in selection and selection['y'][0] != d):
            continue
        row['pairs'] += 1
        for x in m.byc[c]:
            if 'x' in selection and list(m.gens[x]) != selection['x']:
                continue
            for yy in m.byc[d]:
                if 'y' in selection and list(m.gens[yy]) != selection['y']:
                    continue
                clock.tick(phase='step3', x=list(m.gens[x]), y=list(m.gens[yy]), e=None, V=None)
                action = m.array(m.comm(x, yy))
                row['commutators'] += 1
                if np.array_equal(action, m.ar):
                    continue
                power = m.ar
                for e in range(1, 61):
                    power = action[power]
                    if 'e' in selection and selection['e'] != e:
                        continue
                    clock.tick(phase='step3-power', e=e, V=None)
                    if not m.is_home[power[point]] or (power[protected] != protected).any():
                        rejected['no_landing_power'] += 1
                        continue
                    bad_count = int(m.home_size[power[m.home_points] % 5 != 0].sum())
                    if bad_count >= 3097:
                        rejected['no_landing_power'] += 1
                        continue
                    bad, _ = m.bad(power)
                    assert len(bad) == bad_count
                    row['landing_powers'] += 1
                    # Enumerate literal V, including the empty word first.
                    for v in [None] + list(range(len(alphabet.letters))):
                        V = [] if v is None else alphabet.pairs([v])
                        if 'V' in selection and selection['V'] != V:
                            continue
                        clock.tick(phase='step4-vacancy', V=V)
                        preimage = (bad if v is None else alphabet.images([int(alphabet.inv[v])], bad))
                        if alphabet.occupied[preimage].any():
                            rejected['no_vacator'] += 1
                            continue
                        row['vacated'] += 1
                        middle = [list(m.gens[g]) for g in m.comm(x, yy, e)]
                        inverse_v = [] if v is None else alphabet.pairs([int(alphabet.inv[v])])
                        a = [row['cap'], row['item']]
                        word = [a] + V + middle + inverse_v + [[a[0], rule.inverse_items[a[1]]]]
                        assert len(word) == 2 + 4 * e + 2 * len(V) <= 244
                        row['replays'] += 1
                        check = check_step5(rule, unit['pieces'], word, clock)
                        clock.tick(phase='step5-result')
                        if not check['strict_prime']:
                            rejected['replay_failed'] += 1
                            continue
                        step = {k: row[k] for k in ('unit', 'cap', 'item')}
                        step.update(x=list(m.gens[x]), y=list(m.gens[yy]), e=e, V=V, letters=len(word),
                                    bad=len(bad), word='word.json', input_digest=rule.digest,
                                    output_digest=check['end_digest'], check={k: check[k] for k in CHECK_FIELDS})
                        return step, word
    return None


def validate_case(value):
    if not isinstance(value, dict) or set(value) - {'unit', 'cap', 'item', 'x', 'y', 'e', 'V'}:
        raise ValueError('case must be an object with only unit, cap, item, x, y, e, V')
    for k, lower, upper in (('unit', 0, 177119), ('cap', 0, 599), ('item', 1, 59)):
        if k not in value or type(value[k]) is not int or not lower <= value[k] <= upper:
            raise ValueError('case requires valid integer unit, cap and item')
    def letter(p):
        return (isinstance(p, list) and len(p) == 2 and all(type(i) is int for i in p)
                and 0 <= p[0] < 600 and 1 <= p[1] < 60)
    for k in ('x', 'y'):
        if k in value and not letter(value[k]):
            raise ValueError('case x and y must be nonidentity letter pairs')
    if 'e' in value and (type(value['e']) is not int or not 1 <= value['e'] <= 60):
        raise ValueError('case e must be an integer from 1 to 60')
    if 'V' in value and not (isinstance(value['V'], list) and
                             (value['V'] == [] or (len(value['V']) == 1 and letter(value['V'][0])))):
        raise ValueError('case V must be empty or one letter')
    return value


def atomic_json(path, value):
    tmp = path.with_name(path.name + '.tmp')
    tmp.write_text(json.dumps(value, indent=2) + '\n')
    tmp.replace(path)


def script_digests():
    paths = [Path(__file__).resolve(), SCRIPTS / 'screen.py', SCRIPTS / 'eb6.py',
             SCRIPTS / 'cellpure5.py', Path(__file__).with_name('stepchk.py')]
    paths += [REPO / 'research/jumbling/theory' / (s + '.py')
              for s in ('e3_class', 'buffer', 'restore', 'trees', 'classes', 'rim', 'seams', 'orient')]
    paths += [REPO / 'research/jumbling/exact.py']
    paths += sorted((REPO / 'research/jumbling/sim').glob('*.py'))
    return {p.relative_to(REPO).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}


def run_rule(rule, kind, name, out, seconds=3600, selection=None, started=None):
    """Run the ordered finite rule; initialization and every case share a clock."""
    selection = {} if selection is None else validate_case(selection)
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    (out / 'word.json').unlink(missing_ok=True)
    clock = Deadline(seconds, started)
    report = {'format': 'r22d19-run/1', 'rule': 'R22D-19',
              'input': {'kind': kind, 'name': Path(name).name, 'digest': rule.digest,
                        'off': int(rule.original_off.sum()), 'heights': rule.heights},
              'result': 'budget', 'units': [], 'cases': [], 'rejected': dict.fromkeys(REASONS, 0),
              'step': None, 'reached': None, 'seconds': 0.0, 'scripts': script_digests()}
    def save():
        if report['result'] == 'budget':
            report['reached'] = dict(clock.reached)
        report['seconds'] = round(time.monotonic() - clock.started, 6)
        atomic_json(out / 'report.json', report)
    save()  # replaces any earlier report before the first possible interruption
    try:
        for unit in rule.discover(clock):
            report['units'].append({'unit': unit['unit'], 'pieces': len(unit['pieces']),
                                    'branch': unit['branch'], 'caps': unit['caps']})
        for unit in rule.units:
            if 'unit' in selection and selection['unit'] != unit['unit']:
                continue
            for cap in unit['caps']:
                if 'cap' in selection and selection['cap'] != cap:
                    continue
                for item in unit['lowering'][cap]:
                    if 'item' in selection and selection['item'] != item:
                        continue
                    begin = time.monotonic()
                    row = {'unit': unit['unit'], 'cap': cap, 'item': item, 'outcome': 'budget',
                           'pairs': 0, 'commutators': 0, 'landing_powers': 0, 'vacated': 0, 'replays': 0,
                           'seconds': 0.0}
                    report['cases'].append(row)
                    try:
                        clock.reached = dict(unit=unit['unit'], cap=cap, item=item, phase='step1')
                        reason, context = rule.filter_case(unit, cap, item, clock)
                        if reason:
                            row['outcome'] = reason
                            report['rejected'][reason] += 1
                        else:
                            answer = search_case(rule, unit, context, row, report['rejected'], selection, clock)
                            if answer:
                                step, word = answer
                                row['outcome'] = 'step'
                                report.update(result='step', step=step, reached=None)
                                atomic_json(out / 'word.json', {'word': word, 'input_digest': rule.digest,
                                                               'output_digest': step['output_digest']})
                            else:
                                row['outcome'] = ('no_landing_power' if not row['landing_powers'] else
                                                  'no_vacator' if not row['vacated'] else 'replay_failed')
                                # An empty restricted family still has a step-specific rejection.
                                if not row['commutators']:
                                    report['rejected']['no_landing_power'] += 1
                    finally:
                        row['seconds'] = round(time.monotonic() - begin, 6)
                        assert rule.scratch.digest() == rule.digest
                        save()
                    if report['result'] == 'step':
                        return report
        if selection and not report['cases']:
            raise ValueError('no lowering case matches the selection')
        report.update(result='stuck', reached=None)
    except Budget:
        report.update(result='budget', reached=dict(clock.reached))
    save()
    return report


def main(argv=None):
    started = PROCESS_START if argv is None else time.monotonic()
    ap = argparse.ArgumentParser(description=__doc__)
    source = ap.add_mutually_exclusive_group(required=True)
    source.add_argument('--table')
    source.add_argument('--record')
    ap.add_argument('--out', required=True)
    ap.add_argument('--cache')
    ap.add_argument('--seconds', type=float, default=3600)
    ap.add_argument('--case', type=json.loads)
    args = ap.parse_args(argv)
    if not math.isfinite(args.seconds) or args.seconds <= 0:
        ap.error('--seconds must be finite and positive')
    if args.case is not None:
        try:
            validate_case(args.case)
        except ValueError as error:
            ap.error(str(error))
    configure_cache(args.cache or Path(args.out) / 'cache')
    kind, path = ('table', args.table) if args.table else ('record', args.record)
    st, menu = load_current(kind, path)
    try:
        report = run_rule(Rule(st, menu), kind, path, args.out, args.seconds, args.case, started)
    except ValueError as error:
        ap.error(str(error))
    print(json.dumps({'result': report['result'], 'seconds': report['seconds'], 'reached': report['reached']}),
          flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
