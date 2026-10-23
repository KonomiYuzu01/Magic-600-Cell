"""HD2P/1 segmented exact experiment; no solver or performance claim.

Run from the repository root. OUTPUT below is ../hd2p-7306 (outside the
repository); substitute the emitted search/checker directory names for S0/C0.
The integrator commits and pushes manifest.commit_files after each finalized
search checkpoint and before resuming. No program invokes Git or a network.

Fresh segment from the pinned input:
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/height_descent_experiment.py --segment --case 7306 --out ../hd2p-7306
Resume (source identities and the entire artifact chain must still match):
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/height_descent_experiment.py --segment --case 7306 --out ../hd2p-7306 --resume ../hd2p-7306/S0/manifest.json
Checker attempt (add --resume-check ../hd2p-7306/C0/manifest.json for a retry):
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/check_height_descent_experiment.py --chain ../hd2p-7306/S0/manifest.json --out ../hd2p-7306
Terminal audit, after or alongside completion of literal replay:
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/check_height_descent_experiment.py --chain ../hd2p-7306/S0/manifest.json --out ../hd2p-7306 --audit --resume-check ../hd2p-7306/C0/manifest.json
Qualification of the first 1000 completed logical units (not a trajectory pass):
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/height_descent_experiment.py --segment --case 7306 --out ../hd2p-7306 --qualify-ledger ../first-run-7306/search.jsonl --units 1000
Separate diagnostic job, before trajectory; resume its manifest for search:
  python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/height_descent_experiment.py --segment --case 7306 --out ../hd2p-7306 --diagnostics-only

Omitting --segment retains height-descent-experiment/1 and its old CLI. New
records use height-descent-segment/3, still rule HD2P/1. Diagnostic states and
loss history never enter search. An optional --not-after 2026-10-24T12:00:00Z
sets the immutable campaign deadline; the default is 24 hours from job start.
Each case has six search/qualification/diagnostic and four checker/audit process
reservations. Keep allocation.json for killed attempts too: deleting reservations
would discard campaign accounting. Reserve at most three processes externally.
Accounting assumes one output root per campaign, including killed attempts.
The authenticated-resume induction and receipt trust boundary are documented in
check_height_descent_experiment.py. Version 2 manifests need a fresh campaign;
the qualification ledger interface and the HD2P/1 unit/STEP formats are unchanged.

Useful work stops at 3000 CPU seconds; 300 seconds are reserved for publication,
with a 3300 CPU hard stop and 3600 wall backstop. Every manifest lists committed
chunks (<5,000,000 bytes), optional private caches and portable command arguments.
Unfinalized directories are not results. Only their allocation.json is durable.
Search results remain provisional until suffix verification; STUCK additionally
needs a complete independent terminal audit. Otherwise the observable is BUDGET.

Qualification compares coordinates, outcomes, heights and Q, grouping all 48
second items. It continues across old STEP boundaries. It fails at the first
mismatch or missing completed old unit. Events artifacts contain the unchanged
STEP words, digests and measurements for the integrator's separate events.jsonl
comparison (timing/search-work fields are intentionally different). Source and
input hashes pin the six permitted inputs; qualification/probes consume the same
per-table reservations. The fast gate does not run development trajectories or
replace the integrator's timed trajectory qualification and performance pilot.
"""

import argparse
import copy
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
RULE = 'HD2P/1'


def top(hist):
    return max((int(k) for k in hist), default=0)


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
        entry = dict(step=self.step, sequence=self.entries, at=at, outcome=outcome, detail=details)
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

    def classify(self, st, c):
        self.tick()
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
        outcome = trial.apply(self.twist(c, x), journal=False)
        self.tick()
        if not outcome.applied or outcome.uncertain:
            return None
        return trial

    def blocked(self, st):
        return sum(not self.clean(self.classify(st, c)) for c in self.caps)

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
    if not (set(hist) - {'2'}):
        return dict(result='HANDOFF')
    h, q = top(hist), potential(hist)
    C.require(h in (4, 6), 'input maximum must be 4 or 6')
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
    ledger.exhausted('CL2')

    for c in clean:
        for x in engine.items:
            ledger.begin('PEEL', 'candidate', c=c, x=x)
            end = engine.predict(state, c, x, classes[c], hist)
            if end is None or top(end) > h or counts64(end) != counts64(hist):
                ledger.done('height_or_count_prune', heights=end)
                continue
            trial = engine.move(state, c, x)
            if trial is None:
                ledger.done('inadmissible')
                continue
            C.require(engine.histogram(trial) == end, 'PEEL prediction/replay mismatch')
            end_bk = engine.blocked(trial)  # Always a global count of all 600.
            ledger.done('global_blocked', before=bk, after=end_bk, heights=end)
            if end_bk < bk:
                return replay_candidate(engine, state, [[c, x]], [end], ledger, 'PEEL')
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
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--case', type=int, choices=C.CASES, required=True)
    ap.add_argument('--out', required=True)
    ap.add_argument('--cpu-seconds', type=float, default=3300)
    ap.add_argument('--search-cpu-seconds', type=float, default=2400)
    ap.add_argument('--max-steps', type=int, default=200)
    args = ap.parse_args()
    C.require(0 <= args.max_steps <= 200, 'max-steps must be between 0 and 200')
    limits = C.Limits(args.cpu_seconds, args.search_cpu_seconds)  # Before NumPy/geometry imports.
    output = C.external_directory(args.out)
    output.mkdir(parents=True, exist_ok=False)
    return run(args, limits, output)



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

    Its base belongs to the search engine and is never mutated. A fresh overlay
    is constructed for each trial; accepted words still go through State.apply.
    """
    def __init__(self, engine, base, c, x, inside):
        self.base, self.c, self.x = base, c, x
        self.inside = engine.rt.np.array(inside, dtype=engine.rt.np.int64, copy=True)
        self.inside.flags.writeable = False
        self.mask = engine.rt.np.zeros(P, dtype=bool)
        self.mask[self.inside] = True
        self.mask.flags.writeable = False
        self.twist = engine.twist(c, x)


class IncrementalEngine(ExactEngine):
    """Exact subset classification with state-local membership and bounded caches.

    Blocked classifications are incomplete. If their remembered witness moves,
    classify the unmoved complement once for (state,c,d), independently of x.
    A distribution belongs to an effective state AND its classification, never
    just a classification (membership can survive a retained twist). Cache only
    the most recent distribution: all 48 endpoint items share its multiplicities.
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
        merged = {}
        for matrix, k, members in self.groups(state.base, pieces):
            moved = state.mask[members]
            for sub, changed in ((members[~moved], False), (members[moved], True)):
                if not len(sub):
                    continue
                m, idx = self.compose(state.twist, matrix, k) if changed else (matrix, k)
                if m not in merged:
                    merged[m] = idx, []
                merged[m][1].append(sub)
        return [(m, k, np.concatenate(parts)) for m, (k, parts) in merged.items()]

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
        return VirtualOpener(self, state, c, x, cl.inside)

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

    def classify(self, state, d):
        self.tick()
        key = state, d
        if key in self.classes:
            return self.classes[key]
        virtual = state if isinstance(state, VirtualOpener) else self.relations.get(state)
        if virtual is None:
            self.stats['full_classifications'] += 1
            result = super().classify(state, d)
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
                        uk = virtual.base, virtual.c, d
                        if uk not in self.unmoved:
                            self.stats['unmoved_fallbacks'] += 1
                            self.unmoved[uk] = self.restricted(virtual.base, d,
                                                             self.rt.np.flatnonzero(~virtual.mask))
                        rest = self.unmoved[uk]
                        if not self.clean(rest):
                            self.classes[key] = rest
                            return rest
                        outside = rest.inside
                    from sim.state import Classification
                    result = Classification(d)
                    result.inside = self.rt.np.sort(self.rt.np.concatenate((outside, moved.inside)))
                    result.inside.flags.writeable = False
        self.classes[key] = result
        return result

    def input_classify(self, state, d):
        cl = self.classify(state, d)
        # Preserve the first-run filter outcome categories (first certificate is
        # in simulator canonical pose order). Measurements only need status.
        if not self.clean(cl) and (state, d) not in self.canonical_classes:
            cl = super().classify(state, d)
            self.classes[state, d] = cl
            self.canonical_classes[state, d] = True
        return cl

    def predict(self, state, c, x, cl, hist):
        self.tick()
        tw = self.twist(c, x)
        if tw.kidx >= 0:
            return dict(hist)
        height, left, _ = self.matrix_info(tw.matrix, tw.kidx)
        cached_state, cached_class, distribution = self.distribution
        if cached_state is not state or cached_class is not cl:
            distribution = []
            for matrix, k, members in self.groups(state, cl.inside):
                self.tick()
                old, factor, _ = self.matrix_info(matrix, k)
                distribution.append((matrix, old, factor, len(members)))
            # Publish only a complete distribution; a budget interruption must
            # not leave a partially filled cache valid for the next item.
            distribution = tuple(distribution)
            self.distribution = (state, cl, distribution)
        out = Counter(hist)
        for matrix, old, factor, count in distribution:
            self.tick()
            if old:
                key = tw.matrix, matrix
                if key not in self.products:
                    self.products[key] = int(self.rt.trees.dist(self.rt.trees.qmul(left, factor)))
                new = self.products[key]
                out[str(old)] -= count
            else:
                new = height
            if new:
                out[str(new)] += count
        C.require(all(v >= 0 for v in out.values()), 'negative effective histogram')
        self.stats['predictions'] += 1
        return {h: n for h, n in out.items() if n}

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

class SegmentedSearch:
    """One completed logical unit at a time. A failed unit changes no cursor."""
    def __init__(self, engine, interrupt=lambda _: None):
        self.engine, self.interrupt = engine, interrupt
        self.full, self.base = {}, None

    def unit(self, state, cursor, before, initial_lattice, ever, letters, times=lambda: {}):
        engine = self.engine
        if self.base is not state:
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
            cl = engine.classify(state, c)
            return (engine.virtual(state, c, x, cl) if hasattr(engine, 'virtual')
                    else engine.move(state, c, x))

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
                if not original['clean'] and d not in self.full:
                    self.full[d] = engine.straddlers(state, d)
                result = ('blocker_containment' if not original['clean'] and not self.full[d] <= inside
                          else 'target_survives')
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
                for y in engine.jumble:
                    self.interrupt('before_y_' + str(y))
                    end = predict(trial, d, y, cl, middle)
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
            end = predict(state, c, x, engine.classify(state, c), hist)
            if end is None or top(end) > top(hist) or counts64(end) != counts64(hist):
                add('PEEL', 'candidate', 'height_or_count_prune', dict(heights=end), c=c, x=x)
            else:
                trial = virtual(c, x)
                if trial is None:
                    add('PEEL', 'candidate', 'inadmissible', c=c, x=x)
                else:
                    bk, after = len(engine.caps) - len(clean), engine.blocked(trial)
                    add('PEEL', 'candidate', 'global_blocked', dict(before=bk, after=after, heights=end), c=c, x=x)
                    if after < bk:
                        chosen = 'PEEL', [[c, x]], [end]
        else:
            raise ValueError('terminal/invalid enumerator cursor')
        row, updated, endpoint = None, ever, state
        if chosen is not None:
            family, word, predicted = chosen
            self.interrupt('before_replay')
            ledger = Ledger(cursor['step'], tick=getattr(engine, 'tick', lambda: None))
            decision = replay_candidate(engine, state, word, predicted, ledger, family)
            self.interrupt('before_measurement')
            row, updated = step_record(decision, engine, before, initial_lattice, ever,
                                       cursor['step'], letters, ledger, times)
            row['search'] = {'format': CHAIN_NAME, 'deciding_cursor': C.cursor_hash(cursor)}
            endpoint = decision['state']
            self.interrupt('before_step_publication')
        after = C.advance_cursor(cursor, events, row, caps=tuple(engine.caps),
                                 items=tuple(engine.items), jumble=tuple(engine.jumble))
        if hasattr(engine, 'stats'):
            engine.stats[phase + '_completed_cpu'] += time.process_time() - unit_cpu
            engine.stats[phase + '_completed_units'] += 1
        self.interrupt('after_unit')
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


def run_segment(args):
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
    engine.stats['checkpoint_rebuild_cpu'] = time.process_time() - rebuild_started
    start = C.boundary(cp)
    previous = C.manifest_ref(args.resume) if args.resume else None
    campaign = parent['campaign'] if parent else C.uuid.uuid4().hex
    status, report, artifacts = 'BUDGET', None, {}
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
        try:
            if args.diagnostics_only:
                C.require(cp['diagnostics'] is None and cp['steps'] == cp['unit_count'] == 0,
                          'diagnostics once, before trajectory')
                stream = io.StringIO()
                diagnostics(args.case, original, engine, initial_lattice, cp['last'], stream, limits.times)
                certificate = dict(format='hd2p-diagnostics/1', input=C.input_identity(args.case),
                                   identity=identity, probes=[C.parse(x) for x in stream.getvalue().splitlines()],
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
                    except BaseException:
                        cp, state = saved, prior_state
                        trace.seek(trace_pos)
                        trace.truncate()
                        events.seek(event_pos)
                        events.truncate()
                        raise
                    engine.stats['completed_units'] += 1
                    engine.stats['completed_unit_cpu'] += time.process_time() - before_cpu
                    if row is not None:
                        engine.retain(state)
        except C.Budget:
            status = 'BUDGET'
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
    stats = dict(engine.stats)
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
    path = C.publish(directory, kind, identity, C.input_identity(args.case), campaign, previous,
                     start, C.boundary(cp), artifacts, budget, limits.not_after, status,
                     extra={'qualification': report} if expected is not None else None)
    for name in ('units.disposable', 'events.disposable'):
        (directory / name).unlink()
    print(C.canonical(dict(result=status, manifest=str(path.relative_to(root)), verification='provisional',
                          **timing)), flush=True)
    return 0 if expected is None or report['passed'] else 1


def main():
    if '--segment' not in sys.argv:
        return legacy_main()
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--segment', action='store_true')
    ap.add_argument('--case', type=int, choices=C.CASES, required=True)
    ap.add_argument('--out', required=True)
    ap.add_argument('--resume')
    ap.add_argument('--diagnostics-only', action='store_true')
    ap.add_argument('--qualify-ledger')
    ap.add_argument('--units', type=int)
    ap.add_argument('--cpu-seconds', type=float, default=3300)
    ap.add_argument('--work-cpu-seconds', type=float, default=3000)
    ap.add_argument('--wall-seconds', type=float, default=3600)
    ap.add_argument('--max-steps', type=int, default=200)
    ap.add_argument('--not-after')
    args = ap.parse_args()
    C.require(0 <= args.max_steps <= 200, 'cumulative max-steps')
    C.require(not (args.diagnostics_only and args.qualify_ledger), 'separate qualification and diagnostics jobs')
    try:
        C.SegmentLimits(total=args.cpu_seconds, useful=args.work_cpu_seconds, wall=args.wall_seconds,
                        not_after=C.utc_deadline(args.not_after))
        return run_segment(args)
    except C.Budget as exc:
        print(C.canonical(dict(result='BUDGET', finalized=False, reason=str(exc))), flush=True)
        return 1
    except (ValueError, OSError, KeyError, TypeError, AssertionError) as exc:
        print(C.canonical(dict(result='ERROR', finalized=False, reason=str(exc))), flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
