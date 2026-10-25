"""HD2P/2: exact height descent, each opener's CL2 before its PEEL.

Formats: height-descent-experiment/2 (reference fixtures; non-segmented CLI refuses),
height-descent-segment/4, height-descent-check/4, hd2p-cursor/2 and
hd2p-logical-units/2. HD2P/1 manifests, checker chains and qualification ledgers
are explicitly incompatible. No historical state, ceiling or witness enters the
rule. At each accepted state H, Q=177121*n6+n4 and M=601*Q+Bk are recomputed.
SINGLE and CL2 lower Q; PEEL preserves (n6,n4) and lowers global Bk in [0,600].
Thus M strictly decreases. This proves finite trajectories, not completion or
runtime. Unsupported current heights are ERROR, never STUCK or BUDGET.

Order: SINGLE(c increasing,y=12..59); then (c increasing,x=1..59), its CL2
(d increasing, minimum(Q_end,y) over ALL 48 literals), then its PEEL. The cursor
records each intermediate target's status, accumulates the global blocked delta,
and certifies cl2_done before PEEL or advancement. Interruptions commit no partial
group, word, measurement or exhaustion. Literal accepted-word replay is separate
from the search's classifications and count summaries. The checker independently
replays every prefix; terminal STUCK additionally requires its completed audit.

Exact reductions (all differentially tested):
1. Partition the exact menu at startup by E_y*E_z^-1 in its certified retained
   subgroup. Its four left cosets each have twelve jumble literals. Retained
   left multiplication fixes the tree root and hence preserves every height.
   K+ menu-frame conjugation transports this certificate to all caps. Four
   representative histograms expand to all 48 IDs before the literal tie-break.
   Opener geometry is never quotiented by these height classes.
2. The 5-regular tree has twenty depth-two prefixes. Build those exact vertices
   from the menu and retained units, certify their pairwise distances and five
   branches, and locate each pose's prefix from its FULL exact factor. The
   distance between the item's inverse-root vertex and the pose is h-2, h or
   h+2 according to two, one or zero shared edges. Cache by effective matrix,
   never mutable pose ID or old prefix. In a clean inside set count lattice L0,
   foreign F_h, own A[h,j]. Subtract F_h+A[h,j] at h, add F_h at h+2 and A[h,j]
   at h-2 (omitting zero); add L0 at 2. This includes lattice collateral and
   every histogram bin. At supported heights Q_end=Q_mid+177120*F4+F2
   -177120*A[6,j]-A[4,j]. F6 (H=6), or F4 (H=4), forbids a bounded endpoint.
   Replacing A by total own counts gives a safe lower bound compared with
   Q_start; it is NEVER applied as a descent restriction on the opener.
3. Pin all base classifications. One immutable overlay holds actual effective
   poses for each opener and is reused over its targets. The complement is
   unchanged across x; its exact blocker can reject containment without building
   the full straddler set. Classification keys include the effective state;
   summaries include state AND membership, so retained pose changes invalidate
   them. Independent endpoint measurements supply the next pinned base.
4. The opener's cap stays clean, nonmeeting caps are unchanged, and every
   excluded blocked target retains an unmoved straddler. Therefore global
   Bk_after=Bk_before+sum_targets(blocked_after-blocked_before). All eligible
   targets are classified, even when a height bound makes scoring futile.
   PEEL uses the completed cursor sum; replay checks the full 600-cap count.

Integrator commands (repository root; CASE is one of 7302..7307):
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/height_descent_experiment.py --segment --case CASE --out ../hd2p2-CASE --cpu-seconds 3300 --work-cpu-seconds 3000 --wall-seconds 3600 --max-steps 200
Resume: append --resume ../hd2p2-CASE/SEARCH/manifest.json to that search command.
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/check_height_descent_experiment.py --chain ../hd2p2-CASE/SEARCH/manifest.json --out ../hd2p2-CASE
Audit: append --audit; retry: append --resume-check ../hd2p2-CASE/CHECK/manifest.json.
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/height_descent_experiment.py --segment --diagnostics-only --case 7304 --cost-probe --out ../hd2p2-7304
The probe defaults to a 3540-second wall backstop. Resume its diagnostics manifest
with the ordinary search command: its trajectory starts at the pinned input.
The probe consumes one search reservation. Its PASS is a cost qualification,
not a search result. The fast gate never runs it on the real 7304 table.

Every case gets six search/qualification/diagnostic/probe and four checker/audit
reservations, killed attempts included. Useful work stops at 3000 absolute CPU
seconds (cold setup and verification included); publication ends by 3300 CPU
seconds with a 3600-second wall backstop. Cumulative ceilings: 19800 search CPU,
13200 checker CPU and 200 accepted steps. --not-after is immutable, defaulting
to 24 hours from campaign start. Hash-linked immutable manifests, exact boundary
tables, prefix receipts, lattice-loss bitsets and chunks below 5000000 bytes
retain the authenticated-resume induction documented in the checker. Keep every
allocation.json, including killed jobs. All outputs and caches stay under --out.
Only the six pinned complete tables are inputs; journals are discarded. No Git,
network, run-directory discovery or external service is used.
T_* timers are disjoint. The additional *_completed_cpu values are inclusive
unit-cost samples, not additive timers. Interrupted scoring has separate counts.
"""

import argparse
import copy
from contextlib import contextmanager
from functools import wraps
import io
from collections import Counter, OrderedDict
import hashlib
import os
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[_name] = '1'
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
# Shared fixed input/harness definitions only. The checker never imports us and
# computes its own histograms, global blocked counts and replay measurements.
import check_height_descent_experiment as C  # noqa: E402

P = 177120
RULE = 'HD2P/2'

TIMERS = ('T_setup', 'T_single', 'T_filter', 'T_opener', 'T_classification',
          'T_signature', 'T_scoring', 'T_replay_measure', 'T_protocol')


class Profile:
    """Nested regions charge CPU to the innermost region only."""
    def __init__(self, counts):
        self.counts, self.stack = counts, []
        self.since = time.process_time()
        for name in TIMERS:
            counts[name] = 0.

    def charge(self):
        now = time.process_time()
        if self.stack:
            self.counts[self.stack[-1]] += now - self.since
        self.since = now

    @contextmanager
    def region(self, name):
        self.charge()
        self.stack.append(name)
        try:
            yield
        finally:
            self.charge()
            self.stack.pop()


def timed(name):
    def decorate(fn):
        @wraps(fn)
        def call(self, *args, **kwargs):
            with self.profile.region(name):
                return fn(self, *args, **kwargs)
        return call
    return decorate


def top(hist):
    return C.maximum(hist)


def potential(hist):
    return 177121 * hist.get('6', 0) + hist.get('4', 0)


def counts64(hist):
    return hist.get('6', 0), hist.get('4', 0)


class Ledger:
    """Completed decisions plus the exact pending operation on interruption.

    search.jsonl is an append-only enumeration trace. Each step embeds its counts,
    frontier, exhausted families and digest of that step's trace. A pending item
    is never counted as evaluated. Target eligibility is invariant under x, so
    its exact tests are logged once per c and reused in increasing d order.
    """
    def __init__(self, step=1, sink=None, tick=lambda: None):
        self.step, self.sink, self.tick = step, sink, tick
        self.counts, self.complete = Counter(), []
        self.entries, self.frontier = 0, None
        self.hash = hashlib.sha256()

    def begin(self, family, phase, **coordinates):
        self.frontier = dict(family=family, phase=phase, **coordinates, status='pending')
        self.tick()

    def done(self, outcome, **details):
        self.tick()
        at = {k: v for k, v in self.frontier.items() if k != 'status'}
        self.entries += 1
        self.counts[at['family'] + ':' + outcome] += 1
        entry = dict(rule=RULE, step=self.step, sequence=self.entries, at=at, outcome=outcome, detail=details)
        self.hash.update((C.canonical(entry) + '\n').encode())
        if self.sink is not None:
            self.sink(entry)
        self.frontier['status'] = 'completed'

    def exhausted(self, family):
        self.begin(family, 'end')
        self.done('exhausted')
        self.complete.append(family)

    def snapshot(self):
        return dict(step=self.step, entries=self.entries, sha256=self.hash.hexdigest(),
                    counts=dict(self.counts), complete_families=list(self.complete),
                    frontier=None if self.frontier is None else dict(self.frontier))


class ExactEngine:
    """Search mechanics. Receives geometry/menu only, never a case or file name."""
    caps = range(600)
    items = range(1, 60)
    jumble = range(12, 60)

    def __init__(self, rt, tick=lambda: None):
        self.rt, self.tick = rt, tick
        self.factors, self.products, self.twists, self.meeting = {}, {}, {}, {}
        self.radius2 = max(rt.dot(v, v) for vs in rt.ctx.regions.rep_vertices for v in vs)
        C.require(self.radius2 == rt.Q5(96, -32), 'cap-meeting radius invariant')

    def clone(self, st):
        out = self.rt.sim.State(self.rt.ctx, filtered=False, menu=self.rt.menu)
        out.pose_id = st.pose_id.copy()
        out._poses, out._lookup = list(st._poses), dict(st._lookup)
        return out

    def info(self, st, pid):
        pose = st._poses[pid]
        if pose.kidx >= 0:
            return 0, None, None
        matrix = st._pose_matrix(pid)
        if matrix not in self.factors:
            left, right = self.rt.trees.factors(matrix)
            height = int(self.rt.trees.dist(left))
            C.require(self.rt.trees.dist(right) == 0 and height > 0 and height % 2 == 0,
                      'I_a pose invariant')
            self.factors[matrix] = height, left
        height, left = self.factors[matrix]
        return height, left, matrix

    def histogram(self, st):
        hist = Counter()
        ids, ns = self.rt.np.unique(st.pose_id, return_counts=True)
        for pid, count in zip(ids.tolist(), ns.tolist()):
            self.tick()
            h, _, _ = self.info(st, pid)
            if h:
                hist[str(h)] += count
        return dict(hist)

    def count_classification(self, canonical=False, apply=False):
        if hasattr(self, 'stats'):
            self.stats['full_classifications'] += 1
            self.stats['canonical_certificate_rebuilds'] += int(canonical)
            self.stats['apply_full_classifications'] += int(apply)
            self.stats['replay_full_classifications'] += int(getattr(self, 'classification_scope', '') == 'replay')

    def classify(self, st, c, *, canonical=False):
        self.tick()
        self.count_classification(canonical=canonical)
        cl = st.classify(c, need_inside=True, stop_at_straddle=True)
        C.require(not cl.uncertain, 'uncertain cap classification')
        return cl

    @staticmethod
    def clean(cl):
        return cl.straddle is None

    @staticmethod
    def inside(cl):
        return set(cl.inside.tolist())

    @staticmethod
    def certificate(cl):
        return None if cl.straddle is None else cl.straddle['piece']

    def meets(self, c, d):
        key = min(c, d), max(c, d)
        if key not in self.meeting:
            data, dot = self.rt.ctx.data, self.rt.dot
            self.meeting[key] = ((data.NN + dot(data.N[c], data.N[d])) * self.radius2
                                 - 2 * data.KAPPA * data.KAPPA).sign() >= 0
        return self.meeting[key]

    def straddlers(self, st, d):
        """Full S_d, only after its initial blocker is inside the opener.

        Uses the exact grouped vertex classifier of r22d81_pairs/labels; lattice
        pieces cannot straddle a lattice cut. No NEAR/centroid exclusion.
        """
        from sim.state import Classification
        from sim.kernel import Evaluator, pullback_form
        rt, found = self.rt, set()
        off = rt.np.flatnonzero(st._kpose()[st.pose_id] < 0)
        if not off.size:
            return found
        cl, reg = Classification(d), rt.ctx.regions
        for pid, members in st._groups(off):
            self.tick()
            ev = Evaluator(pullback_form(st._pose_matrix(pid), rt.ctx.data.N[d]))
            for anchor, sub in st._anchors(pid, members):
                self.tick()
                r = st._eval(ev, reg.cap_vform(anchor), lambda a=anchor: reg.cap_frows(a), cl, None)
                C.require(r[0] != 'uncertain', 'uncertain blocker set')
                if r[0] != 'straddle':
                    continue
                for p in sub.tolist():
                    self.tick()
                    r = st._eval(ev, reg.vform(p), lambda p=p: reg.frows(p), cl, None)
                    C.require(r[0] != 'uncertain', 'uncertain blocker')
                    if r[0] == 'straddle':
                        found.add(p)
        return found

    def twist(self, c, x):
        if (c, x) not in self.twists:
            rt = self.rt
            self.twists[c, x] = rt.sim.Twist(rt.ctx, c, rt.restore.element(rt.menu, c, x),
                                            'menu', {'menu': rt.menu.name})
        return self.twists[c, x]

    def predict(self, st, c, x, cl, hist):
        """Complete endpoint histogram, including every moved lattice piece.

        Local pose numbers are used only to gather this state's multiplicities.
        Persistent factor/product caches use exact matrices, never mutable IDs.
        """
        self.tick()
        tw = self.twist(c, x)
        if tw.kidx >= 0:
            return dict(hist)  # Retained left multiplication preserves heights.
        if not hasattr(cl, '_hd_distribution'):
            ids, ns = self.rt.np.unique(st.pose_id[cl.inside], return_counts=True)
            cl._hd_distribution = [(*self.info(st, p), n) for p, n in zip(ids.tolist(), ns.tolist())]
        left = self.factors.get(tw.matrix)
        if left is None:
            l, r = self.rt.trees.factors(tw.matrix)
            C.require(self.rt.trees.dist(r) == 0, 'item right factor')
            left = (int(self.rt.trees.dist(l)), l)
            self.factors[tw.matrix] = left
        out = Counter(hist)
        for old, factor, matrix, count in cl._hd_distribution:
            self.tick()
            if old == 0:
                new = left[0]  # Right multiplication by K+ fixes the root.
            else:
                key = tw.matrix, matrix
                if key not in self.products:
                    self.products[key] = int(self.rt.trees.dist(self.rt.trees.qmul(left[1], factor)))
                new = self.products[key]
                out[str(old)] -= count
            if new:
                out[str(new)] += count
        C.require(all(n >= 0 for n in out.values()), 'negative predicted count')
        return {h: n for h, n in out.items() if n}

    def move(self, st, c, x):
        self.tick()
        trial = self.clone(st)
        if hasattr(self, 'stats'):
            # Count State.apply's own full classifier at its actual invocation,
            # including interrupted calls. This is an instance-local observer;
            # the simulator code and its classification result are unchanged.
            classify = trial.classify
            def counted(*args, **kwargs):
                self.count_classification(apply=True)
                return classify(*args, **kwargs)
            trial.classify = counted
            try:
                outcome = trial.apply(self.twist(c, x), journal=False)
            finally:
                del trial.classify
        else:
            outcome = trial.apply(self.twist(c, x), journal=False)
        self.tick()
        if not outcome.applied or outcome.uncertain:
            return None
        return trial

    def blocked(self, st):
        classes = {c: self.classify(st, c) for c in self.caps}
        self.last_classes = st, classes
        return sum(not self.clean(cl) for cl in classes.values())

    def off(self, st):
        return set(self.rt.np.flatnonzero(st._kpose()[st.pose_id] < 0).tolist())

    def measure(self, st, initial_lattice, ever):
        hist, off = self.histogram(st), self.off(st)
        loss = initial_lattice & off
        updated = ever | loss
        bk = int(self.blocked(st))
        self.tick()
        n = int(self.rt.rim.Domains(st).total_misalignment())
        self.tick()
        q = potential(hist)
        return ({'digest': st.digest(), 'heights': hist, 'maximum': top(hist),
                 'n2_n4_n6': [hist.get(str(h), 0) for h in (2, 4, 6)],
                 'Q': q, 'Bk': bk, 'M': 601 * q + bk, 'N': n, 'off': len(off),
                 'lattice_loss': len(loss), 'ever_lattice_loss': len(updated)}, updated)


def eligible_targets(engine, state, c, classes, full, ledger):
    """Necessary exact conditions only; every survivor is reclassified at t."""
    inside = engine.inside(classes[c])
    targets = []
    for d in engine.caps:
        ledger.begin('CL2', 'target_filter', c=c, d=d)
        if d == c:
            ledger.done('same_cap')
        elif not engine.meets(c, d):
            ledger.done('not_meeting')
        elif not engine.clean(classes[d]) and engine.certificate(classes[d]) not in inside:
            ledger.done('blocker_certificate')
        else:
            if not engine.clean(classes[d]):
                if d not in full:
                    full[d] = engine.straddlers(state, d)
                if not full[d] <= inside:
                    ledger.done('blocker_containment')
                    continue
            targets.append(d)
            ledger.done('target_survives')
    return targets


def replay_candidate(engine, state, word, predicted, ledger, family):
    """Full State.apply replay, isolated from all trial states, before selection."""
    trial, prefixes = state, []
    for k, ((c, x), want) in enumerate(zip(word, predicted)):
        ledger.begin(family, 'replay', c=c, x=x, prefix=k + 1)
        trial = engine.move(trial, c, x)
        C.require(trial is not None, 'certified candidate failed State.apply')
        C.require(engine.histogram(trial) == want, 'exact prediction/replay mismatch')
        prefixes.append(trial)
        ledger.done('replayed', heights=want)
    return dict(result='STEP', family=family, word=word, states=prefixes, state=trial)


def choose_step(state, engine, ledger):
    """The rule API: current configuration + fixed geometry/menu, no harness data.

    Engine's tiny test implementation is an explicit transition oracle. The real
    engine fixes caps=0..599, items=1..59 and jumble=12..59; there is no CLI switch
    that reduces these sets or skips a family. Budget propagates immediately.
    """
    ledger.begin('INPUT', 'histogram')
    hist = engine.histogram(state)
    ledger.done('measured', heights=hist)
    if C.handoff(hist):
        return dict(result='HANDOFF')
    h, q = top(hist), potential(hist)
    C.require(h in (4, 6), 'unsupported height: input maximum must be 4 or 6')
    classes = {}
    for c in engine.caps:
        ledger.begin('INPUT', 'classify', c=c)
        classes[c] = engine.classify(state, c)
        ledger.done('clean' if engine.clean(classes[c]) else 'blocked',
                    blocker=engine.certificate(classes[c]))
    clean = [c for c in engine.caps if engine.clean(classes[c])]
    bk = len(classes) - len(clean)

    for c in clean:
        for x in engine.jumble:
            ledger.begin('SINGLE', 'candidate', c=c, x=x)
            end = engine.predict(state, c, x, classes[c], hist)
            if end is None:
                ledger.done('inadmissible')
            elif top(end) > h:
                ledger.done('height_prune', heights=end)
            elif potential(end) >= q:
                ledger.done('no_descent', heights=end)
            else:
                ledger.done('lowering', heights=end)
                return replay_candidate(engine, state, [[c, x]], [end], ledger, 'SINGLE')
    ledger.exhausted('SINGLE')

    full = {}
    for c in clean:
        targets = None
        for x in engine.items:
            ledger.begin('CL2', 'opener', c=c, x=x)
            middle = engine.predict(state, c, x, classes[c], hist)
            if middle is None or top(middle) > h:
                ledger.done('opener_prune', heights=middle)
                continue
            trial = engine.move(state, c, x)
            if trial is None:
                ledger.done('opener_inadmissible')
                continue
            C.require(engine.histogram(trial) == middle, 'opener prediction/replay mismatch')
            ledger.done('opener_survives', heights=middle)
            if targets is None:
                targets = eligible_targets(engine, state, c, classes, full, ledger)
            for d in targets:
                ledger.begin('CL2', 'second_cap', c=c, x=x, d=d)
                cl = engine.classify(trial, d)
                if not engine.clean(cl):
                    ledger.done('intermediate_blocked')
                    continue
                ledger.done('intermediate_clean')
                best = None
                for y in engine.jumble:
                    ledger.begin('CL2', 'second_item', c=c, x=x, d=d, y=y)
                    end = engine.predict(trial, d, y, cl, middle)
                    if end is None or top(end) > h:
                        ledger.done('endpoint_prune', heights=end)
                    else:
                        score = potential(end), y
                        ledger.done('endpoint', heights=end, Q=score[0])
                        if best is None or score < best[0]:
                            best = score, end
                # All 48 y items have completed before deciding this (c,x,d).
                if best is not None and best[0][0] < q:
                    y = best[0][1]
                    return replay_candidate(engine, state, [[c, x], [d, y]],
                                            [middle, best[1]], ledger, 'CL2')
            ledger.begin('PEEL', 'candidate', c=c, x=x)
            end = middle
            if end is None or top(end) > h or counts64(end) != counts64(hist):
                ledger.done('height_or_count_prune', heights=end)
                continue
            C.require(engine.histogram(trial) == end, 'PEEL prediction/replay mismatch')
            end_bk = engine.blocked(trial)  # Always a global count of all 600.
            ledger.done('global_blocked', before=bk, after=end_bk, heights=end)
            if end_bk < bk:
                return replay_candidate(engine, state, [[c, x]], [end], ledger, 'PEEL')
    ledger.exhausted('CL2')
    ledger.exhausted('PEEL')
    return dict(result='STUCK')


def step_record(decision, engine, before, initial_lattice, ever, index, letters, ledger, times):
    """Measure a pending step fully before advancing the accepted boundary."""
    prefixes, updated = [], set(ever)
    for k, state in enumerate(decision['states']):
        ledger.begin(decision['family'], 'measure_prefix', prefix=k + 1)
        snap, updated = engine.measure(state, initial_lattice, updated)
        prefixes.append(snap)
        ledger.done('measured_prefix', digest=snap['digest'])
    # Independently spelled progress check; the checker recomputes it on reload.
    end = prefixes[-1]
    C.require(all(top(p['heights']) <= top(before['heights']) for p in prefixes), 'accepted prefix maximum')
    if decision['family'] == 'PEEL':
        C.require(counts64(end['heights']) == counts64(before['heights']) and end['Bk'] < before['Bk'],
                  'accepted PEEL progress')
    else:
        C.require(potential(end['heights']) < potential(before['heights']), 'accepted Q progress')
    C.require(end['M'] < before['M'], 'accepted M progress')
    word = decision['word']
    row = dict(type='STEP', index=index, rule=RULE, family=decision['family'], word=word,
               letters=len(word), cumulative_letters=letters + len(word),
               input_digest=before['digest'], prefix_digests=[p['digest'] for p in prefixes],
               output_digest=end['digest'], before=before, prefixes=prefixes, after=end,
               search=ledger.snapshot(), **times())
    return row, updated


def diagnostics(case, state, engine, initial_lattice, before, stream, times):
    # The case label and witnesses exist only in the harness, never in choose_step.
    for word, n4, end_hist, digest in C.PROBES.get(case, []):
        trial, ever, prefixes = engine.clone(state), set(), []
        for c, x in word:
            trial = engine.move(trial, c, x)
            C.require(trial is not None, 'diagnostic inadmissible')
            snap, ever = engine.measure(trial, initial_lattice, ever)
            prefixes.append(snap)
        C.require(prefixes[0]['heights'].get('4', 0) == n4 and
                  prefixes[-1]['heights'] == end_hist and prefixes[-1]['digest'].startswith(digest),
                  'diagnostic witness mismatch')
        C.require(all(p['maximum'] <= 4 for p in prefixes), 'diagnostic prefix maximum')
        if case == 7304:
            initial = {c for c in engine.caps if not engine.clean(engine.classify(state, c))}
            final = {c for c in engine.caps if not engine.clean(engine.classify(trial, c))}
            C.require(len(initial) == 559 and len(final) == 556 and
                      initial - final == {61, 112, 184} and not final - initial,
                      'diagnostic global preparation mismatch')
        C.emit(stream, dict(type='PROBE', rule=RULE, word=word, before=before, prefixes=prefixes,
                            excluded_from_trajectory=True, **times()))
        # trial is discarded. state has never changed.


def replace_final(path, row):
    raw = path.read_bytes()
    prefix = raw[:raw.rfind(b'\n', 0, len(raw) - 1) + 1]
    pending = path.with_name('events.finalizing')
    pending.write_bytes(prefix + (C.canonical(row) + '\n').encode())
    pending.replace(path)


def run(args, limits, output):
    state = rt = engine = before = initial_lattice = ledger = None
    steps = letters = 0
    ever = set()
    result, reason = 'ERROR', 'initialization incomplete'
    start_written = False
    events_path = output / 'events.jsonl'
    with events_path.open('x', encoding='utf-8') as events, \
            (output / 'search.jsonl').open('x', encoding='utf-8') as search:
        try:
            # Leave an explicit incomplete identity even if an import or initial
            # measurement consumes the allowance. No partial identity is a pass.
            manifest = dict(format=C.FORMAT, rule=RULE, case=args.case,
                            input=C.input_identity(args.case), identity=None,
                            command=['python3', '-I', '-B', str(Path(__file__).relative_to(C.ROOT)),
                                     '--case', str(args.case), '--out', '../campaign',
                                     '--cpu-seconds', str(args.cpu_seconds),
                                     '--search-cpu-seconds', str(args.search_cpu_seconds),
                                     '--max-steps', str(args.max_steps)],
                            python=sys.version, numpy=None,
                            limits=dict(cpu_seconds=args.cpu_seconds,
                                        search_cpu_seconds=args.search_cpu_seconds, max_steps=args.max_steps),
                            lattice_measurement_basis='pinned input; ever includes accepted prefixes only',
                            evidence='headless exact suffix replay; no earlier-history or seven-depth claim')
            C.write_json(output / 'manifest.json', manifest)
            rt = C.Runtime(output)
            engine = ExactEngine(rt, limits.check)
            # The loader is shared infrastructure, not a rule dependency.
            state = C.load_case(args.case, rt, engine)
            limits.check()
            manifest.update(identity=rt.identity(), numpy=rt.np.__version__)
            C.write_json(output / 'manifest.json', manifest)
            initial_lattice = set(range(P)) - engine.off(state)
            before, ever = engine.measure(state, initial_lattice, ever)
            C.emit(events, dict(type='START', format=C.FORMAT, rule=RULE,
                                input=C.input_identity(args.case), initial=before, **limits.times()))
            start_written = True
            diagnostics(args.case, state, engine, initial_lattice, before, events, limits.times)
            # Explicit restart from the pinned table after all independent probes.
            state = C.load_case(args.case, rt, engine)
            engine.tick = lambda: limits.check(search=True)
            while True:
                ledger = None
                if not (set(before['heights']) - {'2'}):
                    result, reason = 'HANDOFF', 'complete off histogram contains only height 2 or is empty'
                    break
                if steps >= args.max_steps:
                    result, reason = 'BUDGET', 'step limit'
                    break
                ledger = Ledger(steps + 1, lambda e: C.emit(search, e), engine.tick)
                decision = choose_step(state, engine, ledger)
                if decision['result'] != 'STEP':
                    result = decision['result']
                    reason = 'all three families exhausted' if result == 'STUCK' else 'height handoff'
                    break
                row, new_ever = step_record(decision, engine, before, initial_lattice, ever,
                                            steps + 1, letters, ledger, limits.times)
                limits.check(search=True)
                C.emit(events, row)
                state, before, ever = decision['state'], row['after'], new_ever
                steps, letters = row['index'], row['cumulative_letters']
        except C.Budget as exc:
            result, reason = 'BUDGET', str(exc)
        except (ValueError, AssertionError, TypeError, KeyError, OSError) as exc:
            result, reason = 'ERROR', str(exc)

        if not start_written:
            C.emit(events, dict(type='START', format=C.FORMAT, rule=RULE,
                                input=C.input_identity(args.case), initial=None,
                                initialization='incomplete', **limits.times()))
        final = dict(type='FINAL', rule=RULE, result=result, reason=reason, last=before,
                     steps=steps, letters=letters, search=None if ledger is None else ledger.snapshot(),
                     verification={'status': 'incomplete', 'passed': False}, **limits.times())
        try:
            limits.check()
            if state is not None:
                C.export_table(state, rt, output / 'final-table.json')
        except C.Budget:
            final.update(result='BUDGET', reason='total CPU allowance; final table incomplete')
        except (ValueError, OSError) as exc:
            final.update(result='ERROR', reason='final table write failed: ' + str(exc))
        C.emit(events, final)

    with (output / 'check.jsonl').open('x', encoding='utf-8') as check:
        try:
            limits.check()
            if before is None:
                if final['result'] == 'BUDGET':
                    raise C.Budget('initialization interrupted; no validated initial boundary')
                raise ValueError('no validated initial boundary')
            report = C.verify_output(output, rt, limits.check, allow_pending=True, check_stream=check)
        except C.Budget as exc:
            report = dict(status='incomplete', passed=False, reason=str(exc))
            final.update(result='BUDGET', reason='independent verification incomplete')
        except (ValueError, AssertionError, KeyError, TypeError, OSError) as exc:
            report = dict(status='error', passed=False, reason=str(exc))
            final.update(result='ERROR', reason='independent verification failed: ' + str(exc))
        final['verification'] = report
        final.update(limits.times())
        C.emit(check, dict(type='CHECK', **report, **limits.times()))
    replace_final(events_path, final)
    print(C.canonical(final), flush=True)
    return 1 if final['result'] == 'ERROR' else 0


def legacy_main():
    # The independent reference function remains available for differential
    # tests, but every executable search must use durable campaign reservations.
    print(C.canonical(dict(result='ERROR', finalized=False,
          reason='HD2P/2 search requires --segment; non-segmented execution is disabled')), flush=True)
    return 1



class BoundedCache(OrderedDict):
    """FIFO eviction changes cost only. Keys are matrices or immutable states."""
    def __init__(self, limit):
        super().__init__()
        self.limit = limit

    def __setitem__(self, key, value):
        super().__setitem__(key, value)
        while len(self) > self.limit:
            self.popitem(last=False)


class VirtualOpener:
    """Immutable mathematical overlay; never interns or compacts simulator poses.

    Its base belongs to the search engine and is never mutated. One overlay is
    shared by all targets of an opener; accepted words go through State.apply.
    """
    def __init__(self, engine, base, c, x, inside):
        self.base, self.c, self.x = base, c, x
        self.inside = engine.rt.np.array(inside, dtype=engine.rt.np.int64, copy=True)
        self.inside.flags.writeable = False
        self.mask = engine.rt.np.zeros(P, dtype=bool)
        self.mask[self.inside] = True
        self.mask.flags.writeable = False
        self.twist = engine.twist(c, x)
        self.poses = [(base._pose_matrix(i), p.kidx) for i, p in enumerate(base._poses)]
        lookup = {m: i for i, (m, _) in enumerate(self.poses)}
        self.pose_id = base.pose_id.copy()
        for matrix, k, members in engine.groups(base, self.inside):
            engine.tick()
            m, idx = engine.compose(self.twist, matrix, k)
            if m not in lookup:
                lookup[m] = len(self.poses)
                self.poses.append((m, idx))
            self.pose_id[members] = lookup[m]
        self.pose_id.flags.writeable = False


class HeightSignatures:
    """Certified menu cosets and the twenty vertices at distance two.

    No approximate arithmetic, pose-ID cache, or truncated-factor update.
    ``signature`` locates the unique ancestor at distance two using the FULL
    current factor. In particular cancellation is never propagated from an old
    depth-two prefix. The root distances also identify the five first edges.
    """
    def __init__(self, engine, limit):
        from exact import matmul, transpose
        from sim.kplus import to_tuple
        self.engine, self.rt = engine, engine.rt
        self.cache, self.cap_roots = BoundedCache(limit), {}
        self.seen = set()
        self.profile = engine.profile
        rt, trees = self.rt, self.rt.trees
        menu = [to_tuple(m) for _, m in rt.menu.items]
        retained = {m for m in menu if rt.ctx.kplus.index_of_matrix(m) >= 0}
        C.require(len(retained) == 12 and retained == set(menu[:12]), 'retained subgroup')
        # Closure is certified, not presumed from item numbering.
        C.require(all(to_tuple(matmul(a, b)) in retained for a in retained for b in retained),
                  'retained subgroup closure')
        groups, remaining = [], set(range(12, 60))
        while remaining:
            y = min(remaining)
            inverse = transpose(menu[y])
            group = tuple(z for z in sorted(remaining)
                          if to_tuple(matmul(menu[z], inverse)) in retained)
            C.require(len(group) == 12, 'twelve literal items per left coset')
            groups.append(group)
            remaining.difference_update(group)
        C.require(len(groups) == 4, 'four jumble item classes')
        self.classes = tuple(groups)
        self.class_of = {y: j for j, group in enumerate(groups) for y in group}
        # Every transported menu is conjugated by K+, so this certificate
        # transports exactly and does not change literal item IDs.
        seed = trees.qconj(trees.factors(engine.twist(0, groups[0][0]).matrix)[0])
        roots = []
        for unit in trees.PUNITS:
            engine.tick()
            vertex = trees.qmul(unit, seed)
            C.require(trees.dist(vertex) == 2, 'depth-two root')
            if not any(trees.dist(trees.qmul(trees.qconj(r), vertex)) == 0 for r in roots):
                roots.append(vertex)
        C.require(len(roots) == 20, 'twenty exact depth-two prefixes')
        self.roots = tuple(roots)
        self.inverses = tuple(trees.qconj(r) for r in roots)
        distances = [[int(trees.dist(trees.qmul(a, b))) for b in roots] for a in self.inverses]
        self.branch = tuple(min(j for j, d in enumerate(row) if d <= 2) for row in distances)
        C.require(len(set(self.branch)) == 5 and all(self.branch.count(b) == 4 for b in self.branch),
                  'five branches of four prefixes')
        C.require(all(d == (0 if i == j else 2 if self.branch[i] == self.branch[j] else 4)
                      for i, row in enumerate(distances) for j, d in enumerate(row)), 'prefix tree metric')

    @timed('T_signature')
    def for_cap(self, c):
        if c not in self.cap_roots:
            trees, ids = self.rt.trees, []
            for group in self.classes:
                h, l, _ = self.engine.matrix_info(self.engine.twist(c, group[0]).matrix)
                C.require(h == 2, 'jumble height')
                vertex = trees.qconj(l)
                matches = [i for i, inv in enumerate(self.inverses) if trees.dist(trees.qmul(inv, vertex)) == 0]
                C.require(len(matches) == 1, 'item inverse-root vertex')
                ids.append(matches[0])
            C.require(len(set(ids)) == 4 and len({self.branch[i] for i in ids}) == 1,
                      'four own-branch item vertices')
            self.cap_roots[c] = tuple(ids)
        return self.cap_roots[c]

    @timed('T_signature')
    def signature(self, matrix, height, factor):
        if matrix not in self.cache:
            self.engine.stats['signature_misses'] += 1
            if matrix not in self.seen:
                self.seen.add(matrix)
                self.engine.stats['D'] += 1
            trees = self.rt.trees
            prefix = None
            for i, inverse in enumerate(self.inverses):
                self.engine.tick()
                if trees.dist(trees.qmul(inverse, factor)) == height - 2:
                    prefix = i
                    break
            C.require(prefix is not None, 'missing full-factor prefix')
            self.cache[matrix] = prefix
        return self.cache[matrix]


class IncrementalEngine(ExactEngine):
    """Exact subset classification with state-local membership and bounded caches.

    Blocked classifications are incomplete. If their remembered witness moves,
    classify the unmoved complement once for (state,c,d), independently of x.
    A distribution belongs to an effective state AND its classification, never
    just a classification (membership can survive a retained twist). Drop each
    old opener's summaries when advancing; all its targets share one overlay.
    States supplied here are owned immutable states; accepted moves clone them.
    """
    def __init__(self, rt, tick=lambda: None, cache_limit=20000):
        super().__init__(rt, tick)
        self.factors, self.products = BoundedCache(cache_limit), BoundedCache(cache_limit)
        self.twists, self.compositions = BoundedCache(600), BoundedCache(cache_limit)
        self.classes, self.unmoved = BoundedCache(1800), BoundedCache(160)
        self.canonical_classes = BoundedCache(1800)
        self.relations, self.anchors = BoundedCache(4), BoundedCache(2000)
        self.distribution = (None, None, ())
        self.stats = Counter()
        self.step_metrics, self.step_cpu_origin = [], time.process_time()
        self.profile = Profile(self.stats)
        self.pinned_state, self.pinned = None, {}
        self.overlay = (None, None)
        self.summaries = BoundedCache(1200)
        self.histograms = BoundedCache(4800)
        with self.profile.region('T_setup'):
            self.signatures = HeightSignatures(self, cache_limit)

    @timed('T_signature')
    def matrix_info(self, matrix, kidx=None):
        if kidx is None:
            kidx = self.rt.ctx.kplus.index_of_matrix(matrix)
        if kidx >= 0:
            return 0, None, matrix
        if matrix not in self.factors:
            l, r = self.rt.trees.factors(matrix)
            h = int(self.rt.trees.dist(l))
            C.require(self.rt.trees.dist(r) == 0 and h > 0 and h % 2 == 0, 'effective I_a pose')
            self.factors[matrix] = h, l
        h, l = self.factors[matrix]
        return h, l, matrix

    def compose(self, tw, matrix, k):
        key = tw.matrix, matrix
        if key not in self.compositions:
            from exact import matmul
            from sim.kplus import to_tuple
            kp = self.rt.ctx.kplus
            if tw.kidx >= 0 and k >= 0:
                newk = kp.compose(tw.kidx, k)
                new = kp.matrix(newk)
            else:
                new = to_tuple(matmul(tw.matrix, matrix))
                newk = kp.index_of_matrix(new)
            self.compositions[key] = new, newk
        return self.compositions[key]

    def groups(self, state, pieces):
        np = self.rt.np
        if not len(pieces):
            return []
        if not isinstance(state, VirtualOpener):
            return [(state._pose_matrix(pid), state._poses[pid].kidx, members)
                    for pid, members in state._groups(np.asarray(pieces, dtype=np.int64))]
        pieces = np.asarray(pieces, dtype=np.int64)
        ids = state.pose_id[pieces]
        order = np.argsort(ids, kind='stable')
        parts = np.split(order, np.flatnonzero(np.diff(ids[order])) + 1)
        return [(*state.poses[int(ids[sub[0]])], pieces[sub]) for sub in parts]

    def histogram(self, state):
        if not isinstance(state, VirtualOpener):
            return super().histogram(state)
        hist = Counter()
        for matrix, k, pieces in self.groups(state, self.rt.np.arange(P)):
            self.tick()
            h, _, _ = self.matrix_info(matrix, k)
            if h:
                hist[str(h)] += len(pieces)
        return dict(hist)

    def virtual(self, state, c, x, cl):
        C.require(self.clean(cl), 'blocked virtual opener')
        key = state, c, x
        if self.overlay[0] != key:
            old = self.overlay[1]
            for cache in (self.classes, self.summaries, self.histograms):
                for cached in [k for k in cache if k[0] is old]:
                    del cache[cached]
            self.overlay = (key, VirtualOpener(self, state, c, x, cl.inside))
            self.stats['virtual_openers'] += 1
        return self.overlay[1]

    def pin(self, state):
        """The 600 base classifications cannot be evicted by trial targets."""
        if state is not self.pinned_state:
            self.pinned_state = state
            self.pinned = {d: cl for (st, d), cl in self.classes.items() if st is state}
        for d in self.caps:
            if d not in self.pinned:
                self.pinned[d] = self.classify(state, d)

    def complement(self, state, c, d, mask):
        key = state, c, d
        if key not in self.unmoved:
            self.stats['unmoved_fallbacks'] += 1
            self.unmoved[key] = self.restricted(state, d, self.rt.np.flatnonzero(~mask))
        return self.unmoved[key]

    def contained(self, state, c, d, cl):
        """Equivalent to S_d subset inside(c), without constructing all S_d."""
        mask = self.rt.np.zeros(P, bool)
        mask[cl.inside] = True
        return self.clean(self.complement(state, c, d, mask))

    def anchor_groups(self, members):
        key = members.tobytes()
        if key not in self.anchors:
            bits = self.rt.ctx.data.sig_matrix(members)
            left, groups = self.rt.np.ones(len(members), bool), []
            while left.any():
                a = int(self.rt.np.argmax(bits[left].sum(axis=0)))
                sel = left & bits[:, a]
                groups.append((a, members[sel]))
                left &= ~sel
            self.anchors[key] = groups
        return self.anchors[key]

    def restricted(self, state, d, pieces):
        from sim.state import Classification
        from sim.kernel import Evaluator, pullback_form
        rt, parts, cl = self.rt, [], Classification(d)
        data, kp, reg = rt.ctx.data, rt.ctx.kplus, rt.ctx.regions
        for matrix, k, members in self.groups(state, pieces):
            self.tick()
            if k >= 0:
                e = int(kp.perms[kp.inv[k], d])
                parts.append(members[data.in_sig(members, rt.np.full(len(members), e))])
                continue
            ev = Evaluator(pullback_form(matrix, data.N[d]))
            for a, sub in self.anchor_groups(members):
                self.tick()
                r = ev.classify(reg.cap_vform(a))
                self.stats['restricted_evaluations'] += r[1]
                C.require(r[0] != 'uncertain', 'uncertain subset')
                if r[0] != 'straddle':
                    if r[0] == 'in':
                        parts.append(sub)
                    continue
                ins = []
                for p in sub.tolist():
                    self.tick()
                    r = ev.classify(reg.vform(p))
                    self.stats['restricted_evaluations'] += r[1]
                    C.require(r[0] != 'uncertain', 'uncertain piece')
                    if r[0] == 'straddle':
                        # Exact complete vertices; no approximate tangency test.
                        cl.straddle = {'piece': p}
                        return cl
                    if r[0] == 'in':
                        ins.append(p)
                parts.append(rt.np.asarray(ins, dtype=rt.np.int64))
        cl.inside = rt.np.sort(rt.np.concatenate(parts)) if parts else rt.np.zeros(0, rt.np.int64)
        cl.inside.flags.writeable = False
        return cl

    def full_classify(self, state, d, canonical=False):
        # Count at the sole fallback entry, including canonical-certificate
        # rebuilds. This counts invocations even if a budget interrupts one.
        return super().classify(state, d, canonical=canonical)

    @timed('T_classification')
    def classify(self, state, d):
        self.tick()
        key = state, d
        if state is self.pinned_state and d in self.pinned:
            return self.pinned[d]
        if key in self.classes:
            return self.classes[key]
        virtual = state if isinstance(state, VirtualOpener) else self.relations.get(state)
        if virtual is None:
            result = self.full_classify(state, d)
            self.canonical_classes[key] = True
        else:
            base = self.classify(virtual.base, d)
            if d == virtual.c or not self.meets(virtual.c, d):
                result = base
                self.stats['nonmeeting_or_opener'] += 1
            elif not self.clean(base) and not virtual.mask[self.certificate(base)]:
                result = base
                self.stats['unmoved_witness'] += 1
            else:
                moved = self.restricted(virtual, d, virtual.inside)
                if not self.clean(moved):
                    result = moved
                else:
                    if self.clean(base):
                        outside = base.inside[~virtual.mask[base.inside]]
                    else:
                        rest = self.complement(virtual.base, virtual.c, d, virtual.mask)
                        if not self.clean(rest):
                            self.classes[key] = rest
                            return rest
                        outside = rest.inside
                    from sim.state import Classification
                    result = Classification(d)
                    result.inside = self.rt.np.sort(self.rt.np.concatenate((outside, moved.inside)))
                    result.inside.flags.writeable = False
        self.classes[key] = result
        if state is self.pinned_state:
            self.pinned[d] = result
        return result

    @timed('T_classification')
    def input_classify(self, state, d):
        cl = self.classify(state, d)
        # Preserve the first-run filter outcome categories (first certificate is
        # in simulator canonical pose order). Measurements only need status.
        if not self.clean(cl) and (state, d) not in self.canonical_classes:
            cl = self.full_classify(state, d, canonical=True)
            self.classes[state, d] = cl
            self.canonical_classes[state, d] = True
            if state is self.pinned_state:
                self.pinned[d] = cl
        return cl

    @timed('T_scoring')
    def summary(self, state, c, cl):
        # Membership AND effective poses are in the key. A retained motion can
        # reuse cl but must not reuse its previous height/prefix distribution.
        key = state, c, cl
        if key not in self.summaries:
            roots = self.signatures.for_cap(c)
            branch = self.signatures.branch[roots[0]]
            lattice, foreign, own = 0, Counter(), {}
            for matrix, k, members in self.groups(state, cl.inside):
                self.tick()
                h, factor, _ = self.matrix_info(matrix, k)
                n = len(members)
                if not h:
                    lattice += n
                    continue
                prefix = self.signatures.signature(matrix, h, factor)
                if self.signatures.branch[prefix] != branch:
                    foreign[h] += n
                else:
                    if h not in own:
                        own[h] = [0, 0, 0, 0]
                    own[h][roots.index(prefix)] += n
            self.summaries[key] = lattice, dict(foreign), own
        return self.summaries[key]

    @staticmethod
    def count_histogram(hist, summary, j):
        lattice, foreign, own = summary
        out = Counter(hist)
        out['2'] += lattice
        for h in foreign.keys() | own.keys():
            rising = foreign.get(h, 0)
            falling = own.get(h, (0, 0, 0, 0))[j]
            out[str(h)] -= rising + falling
            out[str(h + 2)] += rising
            if h > 2:
                out[str(h - 2)] += falling
        C.require(all(n >= 0 for n in out.values()), 'negative count histogram')
        return {h: n for h, n in out.items() if n}

    @staticmethod
    def lower_bound(summary, middle):
        _, foreign, own = summary
        return (potential(middle) + 177120 * foreign.get(4, 0) + foreign.get(2, 0)
                - 177120 * sum(own.get(6, ())) - sum(own.get(4, ())))

    @timed('T_scoring')
    def predict(self, state, c, x, cl, hist):
        self.tick()
        if x < 12:
            C.require(self.twist(c, x).kidx >= 0, 'retained opener item')
            return dict(hist)
        summary = self.summary(state, c, cl)
        self.stats['predictions'] += 1
        j = self.signatures.class_of[x]
        key = state, c, cl, j, tuple(sorted(hist.items()))
        if key not in self.histograms:
            self.histograms[key] = self.count_histogram(hist, summary, j)
            self.stats['height_histogram_builds'] += 1
        return self.histograms[key]

    @timed('T_scoring')
    def endpoints(self, state, c, cl, hist):
        representatives = {}
        for j, group in enumerate(self.signatures.classes):
            self.tick()
            before = self.stats['height_histogram_builds']
            representatives[j] = self.predict(state, c, group[0], cl, hist)
            self.stats['representative_scorings'] += self.stats['height_histogram_builds'] - before
        return {y: representatives[j] for y, j in self.signatures.class_of.items()}

    def move(self, state, c, x):
        # Ordinary State.apply is the sole accepted-state writer.
        cl = self.classify(state, c)
        result = super().move(state, c, x)
        if result is not None:
            self.relations[result] = self.virtual(state, c, x, cl)
        return result

    def retain(self, state):
        # A measured endpoint already has all 600 classifications. Drop parent
        # states and their overlays; records contain no distribution or pose ID.
        kept = [(key, value) for key, value in self.classes.items() if key[0] is state]
        self.classes.clear()
        self.classes.update(kept)
        canonical = [key for key in self.canonical_classes if key[0] is state]
        self.canonical_classes.clear()
        self.canonical_classes.update({key: True for key in canonical})
        self.relations.clear()
        self.unmoved.clear()
        self.anchors.clear()
        self.distribution = (None, None, ())
        self.summaries.clear()
        self.histograms.clear()
        self.overlay = (None, None)
        self.pinned_state, self.pinned = None, {}
        self.pin(state)

class SegmentedSearch:
    """One completed logical unit at a time. A failed unit changes no cursor."""
    def __init__(self, engine, interrupt=lambda _: None):
        self.engine, self.interrupt = engine, interrupt
        self.full, self.base = {}, None
        self.opened = (None, None)

    def unit(self, state, cursor, before, initial_lattice, ever, letters, times=lambda: {}):
        engine = self.engine
        if not hasattr(engine, 'profile'):
            return self._unit(state, cursor, before, initial_lattice, ever, letters, times)
        category = {'histogram': 'T_setup', 'input': 'T_setup', 'single': 'T_single',
                    'filter': 'T_filter', 'opener': 'T_opener', 'group': 'T_scoring', 'peel': 'T_opener'}
        started = time.process_time()
        previous = {k: engine.stats[k] for k in ('representative_scorings', 'predictions')}
        try:
            with engine.profile.region(category.get(cursor['phase'], 'T_protocol')):
                return self._unit(state, cursor, before, initial_lattice, ever, letters, times)
        except C.Budget:
            engine.stats['interrupted_units'] += 1
            engine.stats['interrupted_cpu_seconds'] += time.process_time() - started
            for key, value in previous.items():
                engine.stats['interrupted_' + key] += engine.stats[key] - value
                engine.stats[key] = value
            raise

    def _unit(self, state, cursor, before, initial_lattice, ever, letters, times):
        engine = self.engine
        if self.base is not state:
            if hasattr(engine, 'pin'):
                engine.pin(state)
            self.full, self.base = {}, state
        events, chosen = [], None
        unit_cpu = time.process_time()
        phase, hist = cursor['phase'], cursor['hist']
        clean = [c for c, cl in zip(engine.caps, cursor['classes']) if cl['clean']]

        def add(family, phase_name, outcome, detail=None, **coords):
            events.append(dict(at=dict(family=family, phase=phase_name, **coords),
                               outcome=outcome, detail={} if detail is None else detail))

        def predict(st, c, x, cl, h):
            started = time.process_time()
            result = engine.predict(st, c, x, cl, h)
            if hasattr(engine, 'stats'):
                category = 'endpoint' if phase == 'group' else phase
                engine.stats[category + '_prediction_cpu'] += time.process_time() - started
                engine.stats[category + '_predictions'] += 1
            return result

        def virtual(c, x):
            key = state, c, x
            if self.opened[0] != key:
                cl = engine.classify(state, c)
                self.opened = (key, engine.virtual(state, c, x, cl) if hasattr(engine, 'virtual')
                               else engine.move(state, c, x))
            return self.opened[1]

        self.interrupt('before_unit')
        if phase == 'histogram':
            measured = engine.histogram(state)
            C.same(measured, before['heights'], 'histogram unit/accepted measurements')
            add('INPUT', 'histogram', 'measured', dict(heights=measured))
        elif phase == 'input':
            c = tuple(engine.caps)[len(cursor['classes'])]
            classify = getattr(engine, 'input_classify', engine.classify)
            cl = classify(state, c)
            add('INPUT', 'classify', 'clean' if engine.clean(cl) else 'blocked',
                dict(blocker=engine.certificate(cl)), c=c)
        elif phase in ('single_end', 'cl2_end', 'peel_end'):
            add({'single_end': 'SINGLE', 'cl2_end': 'CL2', 'peel_end': 'PEEL'}[phase], 'end', 'exhausted')
        elif phase == 'single':
            c, x = clean[cursor['ci']], tuple(engine.jumble)[cursor['xi']]
            end = predict(state, c, x, engine.classify(state, c), hist)
            outcome = ('inadmissible' if end is None else 'height_prune' if top(end) > top(hist)
                       else 'no_descent' if potential(end) >= potential(hist) else 'lowering')
            add('SINGLE', 'candidate', outcome, {} if end is None else dict(heights=end), c=c, x=x)
            if outcome == 'lowering':
                chosen = 'SINGLE', [[c, x]], [end]
        elif phase == 'opener':
            c, x = clean[cursor['ci']], tuple(engine.items)[cursor['xi']]
            middle = predict(state, c, x, engine.classify(state, c), hist)
            if middle is None or top(middle) > top(hist):
                add('CL2', 'opener', 'opener_prune', dict(heights=middle), c=c, x=x)
            else:
                trial = virtual(c, x)
                if trial is None:
                    add('CL2', 'opener', 'opener_inadmissible', c=c, x=x)
                else:
                    add('CL2', 'opener', 'opener_survives', dict(heights=middle), c=c, x=x)
        elif phase == 'filter':
            c, d = clean[cursor['ci']], tuple(engine.caps)[cursor['di']]
            original = cursor['classes'][tuple(engine.caps).index(d)]
            inside = engine.inside(engine.classify(state, c))
            if d == c:
                result = 'same_cap'
            elif not engine.meets(c, d):
                result = 'not_meeting'
            elif not original['clean'] and original['blocker'] not in inside:
                result = 'blocker_certificate'
            else:
                contained = True
                if not original['clean']:
                    if hasattr(engine, 'contained'):
                        contained = engine.contained(state, c, d, engine.classify(state, c))
                    else:
                        if d not in self.full:
                            self.full[d] = engine.straddlers(state, d)
                        contained = self.full[d] <= inside
                result = 'target_survives' if contained else 'blocker_containment'
            add('CL2', 'target_filter', result, c=c, d=d)
        elif phase == 'group':
            c, x, d = clean[cursor['ci']], tuple(engine.items)[cursor['xi']], cursor['targets'][cursor['di']]
            trial, middle = virtual(c, x), cursor['middle']
            C.require(trial is not None, 'reconstructed opener')
            cl = engine.classify(trial, d)
            add('CL2', 'second_cap', 'intermediate_clean' if engine.clean(cl) else 'intermediate_blocked',
                c=c, x=x, d=d)
            best = None
            if engine.clean(cl):
                endpoints = engine.endpoints(trial, d, cl, middle) if hasattr(engine, 'endpoints') else None
                for y in engine.jumble:
                    self.interrupt('before_y_' + str(y))
                    end = endpoints[y] if endpoints is not None else predict(trial, d, y, cl, middle)
                    if end is None or top(end) > top(hist):
                        add('CL2', 'second_item', 'endpoint_prune', dict(heights=end), c=c, x=x, d=d, y=y)
                    else:
                        score = potential(end), y
                        add('CL2', 'second_item', 'endpoint', dict(heights=end, Q=score[0]), c=c, x=x, d=d, y=y)
                        if best is None or score < best[0]:
                            best = score, end
                if best is not None and best[0][0] < potential(hist):
                    chosen = 'CL2', [[c, x], [d, best[0][1]]], [middle, best[1]]
        elif phase == 'peel':
            c, x = clean[cursor['ci']], tuple(engine.items)[cursor['xi']]
            C.require(cursor['cl2_done'], 'PEEL before this opener completed CL2')
            end = cursor['middle']
            if end is None or top(end) > top(hist) or counts64(end) != counts64(hist):
                add('PEEL', 'candidate', 'height_or_count_prune', dict(heights=end), c=c, x=x)
            else:
                trial = virtual(c, x)
                C.require(trial is not None, 'reconstructed PEEL opener')
                bk = len(engine.caps) - len(clean)
                after = bk + cursor['blocked_delta'] if hasattr(engine, 'virtual') else engine.blocked(trial)
                C.require(0 <= after <= len(engine.caps), 'shared global blocked count')
                add('PEEL', 'candidate', 'global_blocked', dict(before=bk, after=after, heights=end), c=c, x=x)
                if after < bk:
                    chosen = 'PEEL', [[c, x]], [end]
        else:
            raise ValueError('terminal/invalid enumerator cursor')
        row, updated, endpoint, replay_cpu = None, ever, state, 0.
        if chosen is not None:
            family, word, predicted = chosen
            replay_started = time.process_time()
            self.interrupt('before_replay')
            ledger = Ledger(cursor['step'], tick=getattr(engine, 'tick', lambda: None))
            def replay_measure():
                # All accepted prefixes are actual simulator states, measured
                # with separate factor and classification storage.
                independent = ExactEngine(engine.rt, engine.tick) if isinstance(engine, IncrementalEngine) else engine
                if isinstance(engine, IncrementalEngine):
                    independent.stats, independent.classification_scope = engine.stats, 'replay'
                decision = replay_candidate(independent, state, word, predicted, ledger, family)
                self.interrupt('before_measurement')
                row, updated = step_record(decision, independent, before, initial_lattice, ever,
                                           cursor['step'], letters, ledger, times)
                if isinstance(engine, IncrementalEngine):
                    endpoint, classes = independent.last_classes
                    # Preserve unaffected base classifications across the
                    # accepted boundary. The independent recount above still
                    # measures all caps; summaries are invalidated by retain.
                    for d in engine.caps:
                        if d in engine.pinned and all(d != c and not engine.meets(c, d) for c, _ in word):
                            old = engine.pinned[d]
                            C.require(engine.clean(old) == engine.clean(classes[d]), 'unaffected cap status')
                            # Canonical anchor order can change even when the
                            # straddler set does not; retain only a matching
                            # canonical certificate (or complete clean set).
                            if engine.certificate(old) == engine.certificate(classes[d]):
                                classes[d] = old
                                engine.stats['unaffected_classifications_kept'] += 1
                    engine.classes.update({(endpoint, d): cl for d, cl in classes.items()})
                    engine.canonical_classes.update({(endpoint, d): True for d in classes})
                return decision, row, updated
            if hasattr(engine, 'profile'):
                with engine.profile.region('T_replay_measure'):
                    decision, row, updated = replay_measure()
            else:
                decision, row, updated = replay_measure()
            row['search'] = {'format': CHAIN_NAME, 'deciding_cursor': C.cursor_hash(cursor)}
            endpoint = decision['state']
            replay_cpu = time.process_time() - replay_started
            self.interrupt('before_step_publication')
        after = C.advance_cursor(cursor, events, row, caps=tuple(engine.caps),
                                 items=tuple(engine.items), jumble=tuple(engine.jumble))
        self.interrupt('after_unit')
        if hasattr(engine, 'stats'):
            engine.stats[phase + '_completed_cpu'] += time.process_time() - unit_cpu
            engine.stats[phase + '_completed_units'] += 1
            count = {'single': 'S', 'filter': 'F', 'opener': 'O', 'group': 'G'}.get(phase)
            if count:
                engine.stats[count] += 1
            if phase == 'opener':
                engine.stats['F'] += 1
            if phase == 'group' and events[0]['outcome'] == 'intermediate_clean':
                engine.stats['C'] += 1
                engine.stats['logical_endpoint_coverage'] += len(engine.jumble)
            if phase == 'peel' or (phase == 'opener' and after['phase'] != 'filter'
                                   and events[0]['outcome'] != 'opener_survives') or chosen is not None and chosen[0] == 'CL2':
                engine.stats['completed_openers'] += 1
            if row is not None:
                engine.stats['accepted_steps'] += 1
                engine.stats['Q_reduction'] += row['before']['Q'] - row['after']['Q']
                engine.stats['Bk_reduction'] += row['before']['Bk'] - row['after']['Bk']
                now = time.process_time()
                engine.step_metrics.append(dict(step=row['index'], cpu_seconds=now - engine.step_cpu_origin,
                    verification_cpu_seconds=replay_cpu,
                    Q_reduction=row['before']['Q'] - row['after']['Q'],
                    Bk_reduction=row['before']['Bk'] - row['after']['Bk']))
                engine.step_cpu_origin = now
        return after, events, row, endpoint, updated


CHAIN_NAME = C.CHAIN
legacy_choose_step = choose_step


def choose_step(state, engine, ledger=None, *, cursor=None, before=None, initial_lattice=None,
                ever=None, letters=0, times=lambda: {}, interrupt=lambda _: None, machine=None):
    """Legacy Ledger API, or one transactional optimized enumeration unit.

    The optimized rule receives no case, path, witness or historical ledger.
    Harness qualification compares the returned completed units afterwards.
    """
    if ledger is not None:
        return legacy_choose_step(state, engine, ledger)
    return (machine or SegmentedSearch(engine, interrupt)).unit(state, cursor, before, initial_lattice,
                                                  ever, letters, times)


def semantic_events(events):
    # Certificates can differ under exact incremental classification. Coordinates,
    # outcomes and all reported heights/Q/global Bk must agree with the old run.
    return [dict(at=e['at'], outcome=e['outcome'],
                 detail={k: v for k, v in e['detail'].items() if k != 'blocker'}) for e in events]


def old_units(path, jumble=tuple(range(12, 60))):
    """Group complete old CL2 targets. A censored final group supplies no unit."""
    pending = None
    step, sequence = 1, 0
    with Path(path).open(encoding='utf-8') as f:
        for line in f:
            C.require(line.endswith('\n'), 'truncated qualification ledger')
            e = C.parse(line)
            C.require(e.get('rule') == RULE, 'qualification requires HD2P/2; HD2P/1 ledger rejected')
            if e['step'] != step:
                C.require(e['step'] == step + 1 and pending is None, 'old step order')
                step, sequence = e['step'], 0
            sequence += 1
            C.same(sequence, e['sequence'], 'old ledger sequence')
            e = {k: e[k] for k in ('at', 'outcome', 'detail')}
            phase = e['at']['phase']
            if phase in ('replay', 'measure_prefix'):
                C.require(pending is None, 'unfinished old y group')
                continue
            if phase == 'second_item':
                C.require(pending is not None, 'old endpoint without target')
                at = pending[0]['at']
                expected = dict(at, phase='second_item', y=jumble[len(pending) - 1])
                C.same(e['at'], expected, 'old endpoint order')
                pending.append(e)
                if len(pending) == len(jumble) + 1:
                    yield pending
                    pending = None
            else:
                C.require(pending is None, 'incomplete old y group before new operation')
                if phase == 'second_cap' and e['outcome'] == 'intermediate_clean':
                    pending = [e]
                else:
                    yield [e]


def qualification_compare(got, expected, ordinal):
    C.require(expected is not None, 'old ledger has no complete unit ' + str(ordinal))
    C.same(semantic_events(got), semantic_events(expected), 'qualification mismatch at unit ' + str(ordinal))


def prediction_totals(units):
    counts = Counter()
    groups = 0
    for events in units:
        for e in events:
            phase = e['at']['phase']
            if phase in ('candidate', 'second_item'):
                counts[e['at']['family'] + ':' + e['outcome']] += 1
        if events[0]['at']['phase'] == 'second_cap' and events[0]['outcome'] == 'intermediate_clean':
            C.require(len(events) == 49, '48 endpoint predictions per clean group')
            groups += 1
    single = sum(n for k, n in counts.items() if k.startswith('SINGLE:'))
    endpoints = sum(n for k, n in counts.items() if k.startswith('CL2:'))
    C.require(endpoints == 48 * groups, 'endpoint prediction accounting')
    return dict(single_predictions=single, endpoint_predictions=endpoints, clean_groups=groups,
                outcomes=dict(counts))


def profile_report(engine, cursor, cpu, committed_step=None):
    stats = dict(engine.stats)
    # Time outside instrumented regions is imports, orchestration, hashing and
    # publication. No nested interval is added twice.
    stats['T_protocol'] += max(0., cpu - sum(stats.get(k, 0.) for k in TIMERS))
    for name in ('S', 'F', 'O', 'G', 'C', 'D', 'completed_openers', 'representative_scorings',
                 'logical_endpoint_coverage', 'interrupted_units', 'accepted_steps',
                 'full_classifications', 'canonical_certificate_rebuilds'):
        stats.setdefault(name, 0)
    stats['pending_frontier'] = copy.deepcopy(cursor)
    stats['per_step'] = [m for m in engine.step_metrics if committed_step is None or m['step'] <= committed_step]
    stats['unpublished_steps'] = stats['accepted_steps'] - len(stats['per_step'])
    stats['accepted_steps'] = len(stats['per_step'])
    stats['Q_reduction'] = sum(m['Q_reduction'] for m in stats['per_step'])
    stats['Bk_reduction'] = sum(m['Bk_reduction'] for m in stats['per_step'])
    stats['cpu_per_completed_opener'] = cpu / stats['completed_openers'] if stats['completed_openers'] else None
    stats['cpu_per_target'] = cpu / stats['G'] if stats['G'] else None
    stats['cpu_per_accepted_step'] = cpu / stats['accepted_steps'] if stats['accepted_steps'] else None
    stats['verification_cpu'] = stats['T_replay_measure']
    return stats


def validate_search_args(args):
    """Usage/foreign-rule rejection before directories or durable allocations."""
    C.require(0 <= args.max_steps <= 200, 'cumulative max-steps')
    C.require(not (args.diagnostics_only and args.qualify_ledger), 'separate qualification and diagnostics jobs')
    if getattr(args, 'cost_probe', False):
        C.require(getattr(args, 'segment', False) and args.diagnostics_only and args.case == 7304
                  and not args.resume and not args.qualify_ledger,
                  '--cost-probe requires --segment --diagnostics-only --case 7304, without resume/qualification')
        C.require(args.cpu_seconds == 3300 and args.work_cpu_seconds == 3000 and args.wall_seconds <= 3540,
                  'cost-probe envelope: 3000 useful CPU, 3300 hard CPU, at most 3540 wall seconds')
    if args.qualify_ledger:
        with Path(args.qualify_ledger).open(encoding='utf-8') as stream:
            first = stream.readline()
        C.require(first.endswith('\n') and C.parse(first).get('rule') == RULE,
                  'qualification requires HD2P/2; HD2P/1 ledger rejected')
    if args.resume:
        doc = C.parse(Path(args.resume).read_bytes())
        C.require(doc.get('rule') == RULE and doc.get('format') == C.SEGMENT,
                  'HD2P/1 or older manifest cannot resume HD2P/2')


def probe_enumeration(state, engine, before, lattice, limits):
    """Run a disposable first-step enumeration; no accepted campaign state write."""
    cursor, ever = C.search_cursor(before['digest']), set()
    machine, openers = SegmentedSearch(engine), []
    base_status, chosen = None, None
    while chosen is None:
        limits.check()
        old = cursor
        cursor, events, chosen, _, _ = machine.unit(state, old, before, lattice, ever, 0, limits.times)
        if len(cursor['classes']) == len(engine.caps):
            base_status = [not cl['clean'] for cl in cursor['classes']]
        phase = old['phase']
        if phase == 'opener':
            e = events[0]
            openers.append(dict(position=len(openers) + 1, c=e['at']['c'], x=e['at']['x'],
                middle=e['detail'].get('heights'), pruned=e['outcome'] != 'opener_survives',
                statuses={}, groups={}, Bk=None))
        elif phase == 'group':
            e, current = events[0], openers[-1]
            current['statuses'][e['at']['d']] = e['outcome'] == 'intermediate_blocked'
            if e['outcome'] == 'intermediate_clean':
                current['groups'][e['at']['d']] = [e['detail']['heights'] for e in events[1:]]
        elif phase == 'peel' and events[0]['outcome'] == 'global_blocked':
            openers[-1]['Bk'] = events[0]['detail']['after']
        if cursor['phase'] in ('stuck', 'handoff'):
            break
    return chosen, openers, base_status, cursor


def cost_probe_verdict(report):
    """Qualification only: the real probe's acceptance predicate, also toy-tested."""
    expected = dict(family='PEEL', word=[[23, 1]], opener_position=119,
                    Q_before=797, Q_after=797, Bk_before=559, Bk_after=558)
    if report.get('failure'):
        return 'FAIL', report['failure']
    if report.get('selection') != expected:
        return 'FAIL', 'unexpected first selection'
    if report.get('optimized_cpu_seconds', float('inf')) > 150:
        return 'FAIL', 'optimized first step exceeded 150 CPU seconds (cold setup included)'
    if not report.get('selected_opener_complete') or report.get('status_openers_complete') != 119:
        return 'FAIL', 'mandatory reference items 1 and 2 incomplete'
    return 'PASS', 'all completed comparisons match; mandatory reference items complete'


def run_cost_probe(state, engine, before, lattice, limits):
    """7304 diagnostics harness. The rule API receives neither label nor witness.

    Phase 2 uses the checker's actual-state classifier and direct quaternion
    products for ALL 48 literals, not its compressed audit groups. Its caches
    are independent of the search. Optional comparisons stop at unit boundaries
    using measured costs; any asynchronous interruption fails qualification.
    """
    report = dict(format='hd2p-cost-probe/2', selected_opener_complete=False,
                  status_openers_complete=0, spread_targets_complete=0, recounts_complete=0,
                  failure=None, excluded_from_trajectory=True)
    reference_start = None
    try:
        row, openers, base_status, cursor = probe_enumeration(state, engine, before, lattice, limits)
        report['optimized_cpu_seconds'] = time.process_time()  # Absolute, includes cold setup.
        report['optimized_profile'] = profile_report(engine, cursor, time.process_time())
        C.require(row is not None, 'probe did not find a first step')
        report['selection'] = dict(family=row['family'], word=row['word'], opener_position=len(openers),
            Q_before=before['Q'], Q_after=row['after']['Q'], Bk_before=before['Bk'], Bk_after=row['after']['Bk'])
        C.require(row['family'] == 'PEEL' and row['word'] == [[23, 1]] and len(openers) == 119,
                  'unexpected first selection')
        reference_start = time.process_time()
        independent = C.ReplayEngine(engine.rt, limits.check)
        audit = C.ExhaustionAudit(independent)
        hist = independent.histogram(state)
        C.same(hist, before['heights'], 'probe independent input histogram')
        base = {d: audit.original(state, d) for d in engine.caps}
        C.same([base[d].straddle is not None for d in engine.caps], base_status, 'probe base status')
        selected = openers[-1]

        def actual(opener):
            trial = audit.move(state, opener['c'], opener['x'])
            C.same(independent.histogram(trial), opener['middle'], 'probe opener histogram')
            return trial

        def optimized_status(opener, d):
            if opener['pruned']:
                virtual = engine.virtual(state, opener['c'], opener['x'], engine.classify(state, opener['c']))
                return not engine.clean(engine.classify(virtual, d))
            return opener['statuses'].get(d, base_status[d])

        def compare_group(opener, trial, d, cl):
            direct = [audit.endpoint(trial, d, y, cl, opener['middle']) for y in engine.jumble]
            expanded = opener['groups'].get(d)
            if expanded is None:
                virtual = engine.virtual(state, opener['c'], opener['x'], engine.classify(state, opener['c']))
                vc = engine.classify(virtual, d)
                C.require(engine.clean(vc), 'probe clean target mismatch')
                scored = engine.endpoints(virtual, d, vc, opener['middle'])
                expanded = [scored[y] for y in engine.jumble]
            C.same(expanded, direct, 'probe all 48 endpoint histograms')
            def best(hs):
                return min(((potential(h), y) for h, y in zip(hs, engine.jumble)
                            if top(h) <= top(hist)), default=None)
            C.same(best(expanded), best(direct), 'probe minimum (Q,y)')
            return best(direct)

        # Item 1: selected opener, every actual cap, every clean meeting target,
        # and independent literal replay with all STEP measurements.
        started = time.process_time()
        trial = actual(selected)
        classes = {d: audit.classification(trial, d) for d in engine.caps}
        full_bk = sum(cl.straddle is not None for cl in classes.values())
        C.same(full_bk, selected['Bk'], 'probe selected global Bk')
        selected_groups = 0
        for d in audit.meeting_caps(selected['c']):
            C.same(optimized_status(selected, d), classes[d].straddle is not None, 'probe selected status')
            if classes[d].straddle is None:
                best = compare_group(selected, trial, d, classes[d])
                C.require(best is None or best[0] >= before['Q'], 'selected PEEL preempted by CL2')
                selected_groups += 1
        C.replay_step(independent, state, row, lattice, set(), 1, 0)
        report.update(selected_opener_complete=True, selected_clean_groups=selected_groups,
                      selected_reference_cpu=time.process_time() - started)
        # Item 2: every meeting target of each actual opener, including targets
        # excluded by the search and height-pruned opener positions.
        started, targets, clean_targets = time.process_time(), 0, {}
        for opener in openers:
            limits.check()
            trial = actual(opener)
            for d in audit.meeting_caps(opener['c']):
                cl = audit.classification(trial, d)
                C.same(optimized_status(opener, d), cl.straddle is not None,
                       'probe status at opener %d target %d' % (opener['position'], d))
                targets += 1
                if cl.straddle is None and opener['position'] not in clean_targets:
                    clean_targets[opener['position']] = d
            report['status_openers_complete'] += 1
        report.update(status_targets_complete=targets, status_reference_cpu=time.process_time() - started)
        # Fixed deterministic spread. Time controls only optional extent, never
        # an unfinished search target, opener, or required comparison.
        def room(estimate):
            limits.check()
            return time.process_time() + max(10., 2 * estimate) < getattr(limits, 'useful', 3000)
        started, group_cost = time.process_time(), report['selected_reference_cpu']
        for opener in openers[::10]:
            if not room(group_cost):
                break
            if opener['position'] not in clean_targets:
                continue
            unit_start = time.process_time()
            trial = actual(opener)
            d = clean_targets[opener['position']]
            compare_group(opener, trial, d, audit.classification(trial, d))
            report['spread_targets_complete'] += 1
            group_cost = max(group_cost, time.process_time() - unit_start)
        report['spread_reference_cpu'] = time.process_time() - started
        started, recount_cost = time.process_time(), report['selected_reference_cpu']
        for opener in openers:
            if counts64(opener['middle']) != counts64(hist) or top(opener['middle']) > top(hist):
                continue
            if not room(recount_cost):
                break
            unit_start = time.process_time()
            trial = actual(opener)
            bk = sum(audit.classification(trial, d, inside=False).straddle is not None for d in engine.caps)
            C.same(bk, opener['Bk'], 'probe shared PEEL global Bk at opener ' + str(opener['position']))
            report['recounts_complete'] += 1
            recount_cost = max(recount_cost, time.process_time() - unit_start)
        report['recount_reference_cpu'] = time.process_time() - started
        # Direct 48-product cost is a conservative projection for the auditor's
        # independently certified four-representative endpoint groups.
        target_cost = report['status_reference_cpu'] / max(1, targets)
        prediction_cost = audit.stats.get('prediction_cpu', 0.) / max(1, audit.stats.get('prediction_count', 0))
        clean_mean = sum(len(o['groups']) for o in openers) / len(openers)
        report['projected_audit_cpu_per_opener'] = (report['status_reference_cpu'] / len(openers)
            + clean_mean * 48 * prediction_cost + recount_cost)
        report['reference_cpu_per_target'] = target_cost
        report['reference_cpu_per_endpoint'] = prediction_cost
        report['reference_costs'] = audit.stats
    except (C.Budget, ValueError, AssertionError) as exc:
        report['failure'] = str(exc)
    if reference_start is not None:
        report['reference_cpu_seconds'] = time.process_time() - reference_start
    report['verdict'], report['reason'] = cost_probe_verdict(report)
    return report


def run_segment(args):
    validate_search_args(args)
    root = C.external_directory(args.out)
    kind = ('diagnostics' if args.diagnostics_only else
            'qualification' if args.qualify_ledger else 'search')
    directory = C.new_output(root, kind)
    reservation = C.reserve_attempt(root, directory, args.case, 'search')
    rebuild_started = time.process_time()
    old, parent = None, None
    if args.resume:
        chain, old = C.search_chain(args.resume)
        parent = chain[-1][1]
        C.require(Path(args.resume).resolve().parent.parent == root, 'resume must use the campaign output root')
        C.same(parent['input']['case'], args.case, 'resume case')
        C.require(parent['status'] not in ('HANDOFF', 'STUCK'), 'terminal search checkpoint')
        C.require(parent['budget']['segments'] < 6, 'six search segment limit')
    deadline = parent['not_after'] if parent else C.utc_deadline(args.not_after)
    if parent and args.not_after:
        C.same(deadline, C.utc_deadline(args.not_after), 'campaign deadline cannot reset')
    used = parent['budget']['cpu'] if parent else 0
    limits = C.SegmentLimits(used=used, total=args.cpu_seconds, useful=args.work_cpu_seconds,
                            wall=args.wall_seconds, not_after=deadline)
    rt = C.Runtime(directory)
    identity = C.segment_identity(rt)
    if parent:
        C.same(parent['identity'], identity, 'stale source/model/menu/ordering identity')
    engine = IncrementalEngine(rt, limits.check)
    original = C.load_case(args.case, rt, engine)
    initial_lattice = set(range(P)) - engine.off(original)
    if old:
        state = C.load_artifact_table(Path(args.resume).parent, parent['artifacts']['table'],
                                      rt, old['last']['digest'], directory)
        cp = copy.deepcopy(old)
        C.require(C.unbitset(cp['ever']) <= initial_lattice, 'loss set outside original lattice basis')
        C.same(engine.histogram(state), cp['last']['heights'], 'accepted checkpoint histogram')
    else:
        state = original
        first, ever = engine.measure(state, initial_lattice, set())
        cp = dict(cursor=C.search_cursor(state.digest()), last=first, ever=C.bitset(ever),
                  steps=0, letters=0, unit_count=0, unit_hash=C.ZERO_HASH,
                  diagnostics=None, qualification=None)
    if hasattr(engine, 'pin'):
        engine.pin(state)
    engine.stats['checkpoint_rebuild_cpu'] = time.process_time() - rebuild_started
    C.require(C.handoff(cp['last']['heights']) or top(cp['last']['heights']) in (4, 6),
              'unsupported height: current maximum must be 4 or 6')
    engine.stats['T_setup'] += max(0., time.process_time() - sum(engine.stats[k] for k in TIMERS))
    start = C.boundary(cp)
    previous = C.manifest_ref(args.resume) if args.resume else None
    campaign = parent['campaign'] if parent else C.uuid.uuid4().hex
    status, report, artifacts, probe = 'BUDGET', None, {}, None
    expected = None
    if args.qualify_ledger:
        C.require(type(args.units) is int and args.units > cp['unit_count'], '--units must extend completed coverage')
        old_hash = C.sha(Path(args.qualify_ledger).read_bytes())
        if cp['qualification']:
            C.same(old_hash, cp['qualification']['ledger_sha256'], 'qualification ledger identity')
        else:
            C.require(cp['unit_count'] == 0, 'qualification starts at original enumeration')
        expected = iter(old_units(args.qualify_ledger))
        for _ in range(cp['unit_count']):
            C.require(next(expected, None) is not None, 'qualification resume beyond old ledger')
    else:
        C.require(cp['qualification'] is None or kind == 'search', 'qualification mode')
    with (directory / 'units.disposable').open('w+b') as trace, \
            (directory / 'events.disposable').open('w+b') as events:
        pending_publication = False
        try:
            if args.diagnostics_only:
                C.require(cp['diagnostics'] is None and cp['steps'] == cp['unit_count'] == 0,
                          'diagnostics once, before trajectory')
                stream = io.StringIO()
                if getattr(args, 'cost_probe', False):
                    probe = run_cost_probe(original, engine, cp['last'], initial_lattice, limits)
                else:
                    diagnostics(args.case, original, engine, initial_lattice, cp['last'], stream, limits.times)
                certificate = dict(format='hd2p-diagnostics/2', input=C.input_identity(args.case),
                                   identity=identity, probes=[C.parse(x) for x in stream.getvalue().splitlines()],
                                   cost_probe=probe,
                                   **limits.times())
                artifacts['diagnostics'] = C.pack_artifact(directory, 'diagnostics',
                                                          (C.canonical(certificate) + '\n').encode())
                cp['diagnostics'] = artifacts['diagnostics']['sha256']
                status = 'DIAGNOSTICS'
            else:
                limits.boundary()
                machine = SegmentedSearch(engine)
                while True:
                    if C.handoff(cp['last']['heights']):
                        status = 'HANDOFF'
                        break
                    if cp['cursor']['phase'] == 'stuck':
                        status = 'STUCK'  # Provisional until independent audit.
                        break
                    if cp['steps'] >= args.max_steps:
                        break
                    if expected is not None and cp['unit_count'] >= args.units:
                        status = 'QUALIFIED'
                        break
                    limits.check()
                    before_cpu = time.process_time()
                    result = choose_step(state, engine, cursor=cp['cursor'], before=cp['last'],
                                         initial_lattice=initial_lattice, ever=C.unbitset(cp['ever']),
                                         letters=cp['letters'], times=limits.times, machine=machine)
                    cursor, unit_events, row, endpoint, updated = result
                    pending_publication = True
                    limits.check()
                    if expected is not None:
                        qualification_compare(unit_events, next(expected, None), cp['unit_count'] + 1)
                    unit = C.unit_record(cp['unit_count'] + 1, cp['unit_hash'], cp['cursor'], cursor, unit_events, row)
                    # No asynchronous budget timer may split publication in memory
                    # from the spool. A BUDGET rolls back to these byte offsets.
                    trace_pos, event_pos = trace.tell(), events.tell()
                    saved, prior_state = cp, state
                    try:
                        new = dict(cp, cursor=cursor, unit_count=unit['ordinal'], unit_hash=unit['sha256'])
                        if row is not None:
                            new.update(last=row['after'], ever=C.bitset(updated), steps=row['index'],
                                       letters=row['cumulative_letters'])
                            events.write((C.canonical(row) + '\n').encode())
                        trace.write((C.canonical(unit) + '\n').encode())
                        cp, state = new, endpoint
                        pending_publication = False
                    except BaseException:
                        cp, state = saved, prior_state
                        pending_publication = True
                        trace.seek(trace_pos)
                        trace.truncate()
                        events.seek(event_pos)
                        events.truncate()
                        raise
                    # This may interrupt immediately after an exhaustion cursor
                    # is durable in the spool. Its manifest then says BUDGET;
                    # checker audit requirements depend on the cursor itself.
                    limits.check()
                    engine.stats['completed_units'] += 1
                    engine.stats['completed_unit_cpu'] += time.process_time() - before_cpu
                    if row is not None:
                        engine.retain(state)
        except C.Budget:
            status = 'BUDGET'
            engine.stats['publication_interruptions'] += int(pending_publication)
        except ValueError as exc:
            # A mismatch is never a resumable qualified boundary.
            if expected is not None:
                C.write_json(directory / 'qualification-failed.json',
                             dict(passed=False, completed_units=cp['unit_count'], reason=str(exc)))
            raise
        limits.finalizing()
        engine.tick = lambda: None
        if expected is not None:
            report = dict(ledger_sha256=old_hash, completed_units=cp['unit_count'], requested_units=args.units,
                          passed=cp['unit_count'] >= args.units, scope='completed enumeration prefix; STEP replay separately')
            cp['qualification'] = report
        for name, stream in (('units', trace), ('events', events)):
            stream.flush()
            stream.seek(0)
            artifacts[name] = C.pack_artifact(directory, name, stream)
    artifacts['table'] = C.pack_artifact(directory, 'table', C.table_bytes(state, rt))
    artifacts['checkpoint'] = C.pack_artifact(directory, 'checkpoint', (C.canonical(cp) + '\n').encode())
    timing = limits.times()
    stats = profile_report(engine, cp['cursor'], timing['cpu_seconds'], committed_step=cp['steps'])
    if probe is not None and 'optimized_profile' in probe:
        stats = copy.deepcopy(probe['optimized_profile'])
        reference_cpu = probe.get('reference_cpu_seconds', 0.)
        stats['T_replay_measure'] += reference_cpu
        stats['verification_cpu'] += reference_cpu
        stats['T_protocol'] += max(0., timing['cpu_seconds'] - probe['optimized_cpu_seconds'] - reference_cpu)
        stats['profile_scope'] = 'probe optimized decisions; reference comparisons counted separately'
        stats['reference_profile'] = dict(cpu_seconds=reference_cpu, costs=probe.get('reference_costs', {}))
        stats['trajectory_accepted_steps'] = 0
    stats.update(max_rss_kib=C.resource.getrusage(C.resource.RUSAGE_SELF).ru_maxrss,
                 cache_entries={k: len(getattr(engine, k)) for k in ('factors', 'products', 'compositions', 'classes')},
                 python=sys.version, numpy=rt.np.__version__, **timing)
    artifacts['profile'] = C.pack_artifact(directory, 'profile', (C.canonical(stats) + '\n').encode())
    artifacts['allocation'] = C.pack_artifact(directory, 'allocation', (C.canonical(reservation) + '\n').encode())
    timing = limits.times()
    budget = dict(segments=1 + (parent['budget']['segments'] if parent else 0),
                  cpu=used + timing['cpu_seconds'], local_cpu=timing['cpu_seconds'],
                  local_wall=timing['wall_seconds'], reserved_segments=reservation['ordinal'],
                  reserved_cpu=reservation['ordinal'] * 3300,
                  limits=dict(cpu_seconds=args.cpu_seconds, work_cpu_seconds=args.work_cpu_seconds,
                              wall_seconds=args.wall_seconds, max_steps=args.max_steps))
    extra = {'qualification': report} if expected is not None else {}
    if probe is not None:
        extra['diagnostics'] = dict(cost_probe=True, verdict=probe['verdict'], reason=probe['reason'])
    path = C.publish(directory, kind, identity, C.input_identity(args.case), campaign, previous,
                     start, C.boundary(cp), artifacts, budget, limits.not_after, status,
                     extra=extra)
    for name in ('units.disposable', 'events.disposable'):
        (directory / name).unlink()
    print(C.canonical(dict(result=status, manifest=str(path.relative_to(root)), verification='provisional',
                          **({'cost_probe': probe['verdict'], 'reason': probe['reason']} if probe else {}),
                          **timing)), flush=True)
    return 1 if probe is not None and probe['verdict'] != 'PASS' else 0 if expected is None or report['passed'] else 1


def main():
    if '--segment' not in sys.argv:
        if '--cost-probe' in sys.argv:
            print(C.canonical(dict(result='ERROR', reason='--cost-probe requires --segment --diagnostics-only --case 7304')))
            return 1
        return legacy_main()
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--segment', action='store_true')
    ap.add_argument('--case', type=int, choices=C.CASES, required=True)
    ap.add_argument('--out', required=True)
    ap.add_argument('--resume')
    ap.add_argument('--diagnostics-only', action='store_true')
    ap.add_argument('--cost-probe', action='store_true')
    ap.add_argument('--qualify-ledger')
    ap.add_argument('--units', type=int)
    ap.add_argument('--cpu-seconds', type=float, default=3300)
    ap.add_argument('--work-cpu-seconds', type=float, default=3000)
    ap.add_argument('--wall-seconds', type=float)
    ap.add_argument('--max-steps', type=int, default=200)
    ap.add_argument('--not-after')
    args = ap.parse_args()
    if args.wall_seconds is None:
        args.wall_seconds = 3540 if args.cost_probe else 3600
    try:
        validate_search_args(args)
        C.SegmentLimits(total=args.cpu_seconds, useful=args.work_cpu_seconds, wall=args.wall_seconds,
                        not_after=C.utc_deadline(args.not_after))
        return run_segment(args)
    except C.Budget as exc:
        print(C.canonical(dict(result='BUDGET', finalized=False, reason=str(exc),
                              **({'cost_probe': 'FAIL'} if args.cost_probe else {}))), flush=True)
        return 1
    except (ValueError, OSError, KeyError, TypeError, AssertionError) as exc:
        print(C.canonical(dict(result='ERROR', finalized=False, reason=str(exc))), flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
