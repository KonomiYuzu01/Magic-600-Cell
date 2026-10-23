"""HD2P/1: a deterministic, state-only height-descent experiment, not a solver claim.

First run test_height_descent_experiment.py. Then, in fresh output directories:
  python3 -I -B -u height_descent_experiment.py --case 7306 --out /tmp/hd-7306 \
      --cpu-seconds 3300 --search-cpu-seconds 2400 --max-steps 200
The development run order is 7306,7305,7304,7303,7307,7302, at most two independent
single-thread processes. This executable runs one case, never launches a batch.

HANDOFF is independently replayed height <=2; STUCK exhausts this ordering only;
BUDGET is inconclusive. This says nothing about the tutorial's seven-depth test
or the histories of the two unchecked checkpoints. N and lattice pieces are
unprotected. Lattice-loss measurements always refer to the pinned input, and
include intermediate prefixes. PROBE states never enter the rule trajectory.
"""

import argparse
from collections import Counter
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
                            command=[sys.executable, '-I', '-B', *sys.argv],
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


def main():
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


if __name__ == '__main__':
    sys.exit(main())
